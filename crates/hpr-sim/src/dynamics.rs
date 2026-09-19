//! The equations of motion: a rigid body of varying mass referred to a point fixed in the body.
//!
//! Source: the RocketPy technical documentation, "Equations of Motion" v0 (Kane's method with the
//! Reynolds transport theorem) and v1 (the form solved), RocketPy 1.13.0, MIT; G. H. Ceotto,
//! R. N. Schmitt, G. F. Alves, L. A. Pezente and B. S. Carmo, "RocketPy: Six Degree-of-Freedom
//! Rocket Trajectory Simulator", *J. Aerosp. Eng.* 34(6), 2021. The nozzle gyration tensor is
//! integrated here from the boxed rotational equation of v0 (a uniform jet over the exit disc).
//! The derivation, the assumptions and every term are in `docs/physics/flight.md`.
//!
//! In body axes, with `O` the nose tip, `r` the centre of mass from `O`, `m` the mass, `I` the
//! inertia about `O` and `I_c` about the centre of mass, primes body-frame time derivatives and
//! `ṁ_k ≤ 0` each motor's mass rate with its nozzle exit at `n_k`:
//!
//! ```text
//! T20 = −ω×(ω×m r) + ω×(2 Σ ṁ_k (n_k − r) − 2 m r′) + T − m r″ − 2 ṁ r′ + Σ m̈_k (n_k − r) + W + A
//! T21 = −ω×(I ω) + (Σ ṁ_k S_k − I′) ω + r×W + M_A + M_T
//! ω̇   = I_c⁻¹ (T21 − r × T20)
//! a_O = T20/m − ω̇ × r
//! ```
//!
//! `T` is the thrust, `W` the weight with the Coriolis force (both acting at the centre of mass),
//! `A` and `M_A` the aerodynamic force and its moment about `O`, `M_T` the thrust's moment about
//! `O`, and `S_k = (r_e²/4) diag(1, 1, 2) + |n_k|² 1 − n_k n_kᵀ` the gyration tensor of motor
//! `k`'s exit disc of radius `r_e` about `O`. `a_O` is `O`'s acceleration relative to the launch
//! frame, whose rotation enters only through the Coriolis force (normal gravity already holds the
//! centrifugal term) and not through the rotational equations (at most 7.3e-5 rad/s; the decision
//! record on rigid-body flight, [ADR-011][adr-011]).
//!
//! [adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17

use hpr_aero::{AeroModel, DragConditions, Flow};
use hpr_core::attitude::quaternion_derivative;
use hpr_core::{DMat3, DQuat, DVec3};
use hpr_design::Assembly;

use crate::environment::Environment;
use crate::error::SimError;
use crate::state::{STATE_LEN, State};

/// The half-width of the central differences for the mass properties' time derivatives, s.
const MASS_DERIVATIVE_STEP_S: f64 = 1e-4;

/// Airspeeds below this (m/s) produce no aerodynamic force: the angles are undefined at rest.
const MIN_AIRSPEED_M_S: f64 = 1e-9;

/// Integration intervals shorter than this (s) take no mass-property rates: central differences
/// over them would be rounding noise, and the motors can't change measurably within them.
const MIN_DERIVATIVE_INTERVAL_S: f64 = 2e-5;

/// Where the rocket is in its flight, which decides what it is free to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Phase {
    /// Held on the pad by gravity and friction: the state doesn't change.
    Pad,
    /// Guided along the rail: one degree of freedom, no rotation.
    Rail,
    /// Free flight: six degrees of freedom.
    Free,
    /// Descent under recovery devices: a point mass, with the attitude frozen where it deployed
    /// (`docs/physics/recovery.md`).
    Descent,
}

/// What an evaluation needs besides the time and the state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Conditions {
    /// Which phase the flight is in, which decides the degrees of freedom and the forces.
    pub(crate) phase: Phase,
    /// The integration interval: which motors burn through it, and the bounds the mass-property
    /// difference stencils stay inside.
    pub(crate) window: (f64, f64),
    /// The open recovery devices' drag area, m² (descent phase only).
    pub(crate) drag_area_m2: f64,
}

/// A motor's fixed data for the equations.
#[derive(Debug, Clone, Copy)]
struct MotorTerms {
    /// The nozzle exit in body axes, m.
    nozzle_m: DVec3,
    /// The nozzle exit radius, m (zero when the motor doesn't give one).
    exit_radius_m: f64,
    /// The motor's cross-section, for power-on base drag, m².
    area_m2: f64,
    /// The end of its thrust curve, s.
    burnout_s: f64,
}

/// The mass properties at a time and their time derivatives.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MassState {
    pub(crate) mass_kg: f64,
    pub(crate) mass_rate_kg_s: f64,
    pub(crate) cg_m: DVec3,
    pub(crate) cg_rate_m_s: DVec3,
    pub(crate) cg_accel_m_s2: DVec3,
    pub(crate) inertia_cg: DMat3,
    pub(crate) inertia_o: DMat3,
    pub(crate) inertia_o_rate: DMat3,
}

/// Everything the equations need from a flight at one instant, besides the derivative.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Evaluation {
    pub(crate) derivative: [f64; STATE_LEN],
    pub(crate) mass: MassState,
    pub(crate) cg_enu_m: DVec3,
    pub(crate) cg_velocity_enu_m_s: DVec3,
    pub(crate) height_above_ground_m: f64,
    pub(crate) vertical_speed_m_s: f64,
    pub(crate) airspeed_m_s: f64,
    pub(crate) mach: f64,
    pub(crate) angle_of_attack_rad: f64,
    pub(crate) dynamic_pressure_pa: f64,
    pub(crate) axial_coefficient: f64,
    pub(crate) thrust_n: f64,
    /// Force along the rail less friction, N (rail and pad phases; the liftoff condition).
    pub(crate) rail_force_n: f64,
    /// The body origin's acceleration relative to `L`, in `L`, m/s².
    pub(crate) acceleration_enu_m_s2: DVec3,
    /// The drag area of the open recovery devices, m².
    pub(crate) recovery_drag_area_m2: f64,
}

/// The rocket's models, fixed for a flight.
#[derive(Debug, Clone)]
pub(crate) struct Vehicle {
    pub(crate) assembly: Assembly,
    pub(crate) aero: AeroModel,
    /// The body components' stations, which don't change with Mach; fin sets' are taken at each
    /// evaluation's Mach number.
    body_stations_m: Vec<f64>,
    /// The first fin set's index among the aerodynamic components.
    first_fin_index: usize,
    motors: Vec<MotorTerms>,
    reference_area_m2: f64,
}

impl Vehicle {
    pub(crate) fn new(assembly: Assembly, aero: AeroModel) -> Result<Self, SimError> {
        let body_stations_m = (0..aero.bodies().len())
            .map(|index| aero.component_station_m(index, 0.0))
            .collect::<Result<Vec<f64>, _>>()?;
        let motors = assembly
            .motors
            .iter()
            .map(|placed| MotorTerms {
                nozzle_m: placed.nozzle_m,
                exit_radius_m: placed
                    .mounted
                    .motor
                    .nozzle()
                    .map_or(0.0, |nozzle| nozzle.exit_radius_m),
                area_m2: std::f64::consts::PI * (0.5 * placed.mounted.diameter_m).powi(2),
                burnout_s: placed.mounted.motor.burnout_time_s(),
            })
            .collect();
        let reference_area_m2 = aero.reference_area_m2();
        let first_fin_index = aero.bodies().len();
        Ok(Self {
            assembly,
            aero,
            body_stations_m,
            first_fin_index,
            motors,
            reference_area_m2,
        })
    }

    /// The time the last motor burns out, s.
    pub(crate) fn burnout_s(&self) -> f64 {
        self.motors.iter().map(|m| m.burnout_s).fold(0.0, f64::max)
    }

    /// The times at which the thrust curves have knots or end, sorted, after `0`.
    pub(crate) fn thrust_knots_s(&self) -> Vec<f64> {
        let mut times: Vec<f64> = self
            .assembly
            .motors
            .iter()
            .flat_map(|placed| placed.mounted.motor.curve().times_s().iter().copied())
            .filter(|t| *t > 0.0)
            .collect();
        times.sort_by(f64::total_cmp);
        times.dedup();
        times
    }

    /// The mass properties at `t` and their derivatives, from central differences of
    /// `Assembly::mass_properties` kept inside `window` (the current integration interval), so
    /// that no difference straddles a thrust-curve knot or a burnout.
    pub(crate) fn mass_state(&self, t: f64, window: (f64, f64)) -> MassState {
        let props = |t: f64| {
            let mp = self.assembly.mass_properties(t);
            (
                mp.mass_kg,
                mp.cg_m,
                mp.inertia_kg_m2,
                mp.inertia_about(DVec3::ZERO),
            )
        };
        let (mass_kg, cg_m, inertia_cg, inertia_o) = props(t);
        let (a, b) = window;
        let burning = self.motors.iter().any(|m| a < m.burnout_s);
        let mut state = MassState {
            mass_kg,
            mass_rate_kg_s: 0.0,
            cg_m,
            cg_rate_m_s: DVec3::ZERO,
            cg_accel_m_s2: DVec3::ZERO,
            inertia_cg,
            inertia_o,
            inertia_o_rate: DMat3::ZERO,
        };
        if !burning || b - a < MIN_DERIVATIVE_INTERVAL_S {
            return state;
        }
        let h = MASS_DERIVATIVE_STEP_S.min(0.5 * (b - a));
        let c = t.max(a + h).min(b - h);
        let (m_minus, r_minus, _, i_minus) = props(c - h);
        let (m_plus, r_plus, _, i_plus) = props(c + h);
        let (m_mid, r_mid) = if c == t {
            (mass_kg, cg_m)
        } else {
            let (m, r, _, _) = props(c);
            (m, r)
        };
        let offset = t - c;
        let m_second = (m_plus - 2.0 * m_mid + m_minus) / (h * h);
        state.mass_rate_kg_s = (m_plus - m_minus) / (2.0 * h) + offset * m_second;
        let r_second = (r_plus - 2.0 * r_mid + r_minus) / (h * h);
        state.cg_rate_m_s = (r_plus - r_minus) / (2.0 * h) + offset * r_second;
        state.cg_accel_m_s2 = r_second;
        state.inertia_o_rate = (i_plus - i_minus) * (0.5 / h);
        state
    }

    /// Evaluates the equations of motion at `(t, y)` under `conditions`: the phase, the motors
    /// burning as its integration interval decides, and the recovery devices open.
    pub(crate) fn evaluate(
        &self,
        environment: &Environment,
        friction_coefficient: f64,
        conditions: Conditions,
        t: f64,
        y: &[f64; STATE_LEN],
    ) -> Result<Evaluation, SimError> {
        let Conditions {
            phase,
            window,
            drag_area_m2: recovery_drag_area_m2,
        } = conditions;
        let state = State::from_array(y);
        let norm = state.attitude.length();
        if !(norm.is_finite() && norm > 0.0) {
            return Err(SimError::Domain {
                what: "attitude quaternion norm",
                value: norm,
            });
        }
        let q: DQuat = state.attitude / norm;
        let to_body = q.conjugate();
        let mass = self.mass_state(t, window);
        let m = mass.mass_kg;
        let r = mass.cg_m;
        let omega = if phase == Phase::Free {
            state.body_rate_rad_s
        } else {
            DVec3::ZERO
        };

        // Where the centre of mass is, and the air there.
        let v_o = state.velocity_enu_m_s;
        let cg_enu_m = state.position_enu_m + q.mul_vec3(r);
        let cg_velocity_enu_m_s = v_o + q.mul_vec3(omega.cross(r) + mass.cg_rate_m_s);
        let frame = environment.earth.frame();
        let geodetic = frame.geodetic_from_enu(cg_enu_m)?;
        let height_above_ground_m = geodetic.height_m - frame.origin().height_m;
        let up_ecef = DVec3::new(
            geodetic.latitude_rad.cos() * geodetic.longitude_rad.cos(),
            geodetic.latitude_rad.cos() * geodetic.longitude_rad.sin(),
            geodetic.latitude_rad.sin(),
        );
        let up_enu = frame.ecef_from_enu_rotation().transpose() * up_ecef;
        let vertical_speed_m_s = up_enu.dot(cg_velocity_enu_m_s);
        let height_msl_m = geodetic.height_m - environment.geoid_undulation_m;
        let air = environment.atmosphere.air(height_msl_m)?.air;
        let wind_enu = environment.wind.wind(height_msl_m)?.velocity_enu_m_s;

        // Weight and the Coriolis force, at the centre of mass.
        let gravity_enu = environment.earth.gravity_enu_mps2(cg_enu_m)?;
        let coriolis_enu = environment
            .earth
            .rotation_acceleration_enu_mps2(cg_velocity_enu_m_s);
        let weight = to_body.mul_vec3((gravity_enu + coriolis_enu) * m);

        // Thrust and the motors' mass terms. A motor burns during the interval if the interval
        // ends by its burnout.
        let (a, b) = window;
        let pressure_pa = air.pressure_pa;
        let mut thrust = DVec3::ZERO;
        let mut thrust_moment = DVec3::ZERO;
        let mut t03_jet = DVec3::ZERO;
        let mut t04_jet = DVec3::ZERO;
        let mut jet_gyration = DMat3::ZERO;
        let mut burning_area_m2 = 0.0;
        for (terms, placed) in self.motors.iter().zip(&self.assembly.motors) {
            if a >= terms.burnout_s {
                continue;
            }
            let motor = &placed.mounted.motor;
            // The motor burns throughout this interval, so a stage evaluated on its ends (ignition
            // or burnout, where the pressure correction switches) takes the one-sided limit
            // inside the burn.
            let t = t.max(0.0_f64.next_up()).min(terms.burnout_s.next_down());
            let force = DVec3::Z * motor.thrust_at_pressure_n(t, pressure_pa);
            thrust += force;
            thrust_moment += terms.nozzle_m.cross(force);
            burning_area_m2 += terms.area_m2;
            let mdot = -motor.state(t).mass_flow_kg_s;
            let mddot = if b - a < MIN_DERIVATIVE_INTERVAL_S {
                0.0
            } else {
                let h = MASS_DERIVATIVE_STEP_S.min(0.5 * (b - a));
                let c = t.max(a + h).min(b - h);
                -(motor.state(c + h).mass_flow_kg_s - motor.state(c - h).mass_flow_kg_s) / (2.0 * h)
            };
            let lever = terms.nozzle_m - r;
            t03_jet += lever * (2.0 * mdot);
            t04_jet += lever * mddot;
            let n = terms.nozzle_m;
            let disc = 0.25 * terms.exit_radius_m * terms.exit_radius_m;
            let gyration = DMat3::from_diagonal(DVec3::new(disc, disc, 2.0 * disc))
                + DMat3::from_diagonal(DVec3::splat(n.length_squared()))
                - outer(n, n);
            jet_gyration += gyration * mdot;
        }

        // Aerodynamics: the airframe in flight, the open canopies during the descent.
        let aero = if phase == Phase::Descent {
            canopy_drag(
                &air,
                to_body.mul_vec3(cg_velocity_enu_m_s - wind_enu),
                recovery_drag_area_m2,
            )
        } else {
            self.aerodynamics(
                &air,
                to_body.mul_vec3(v_o - wind_enu),
                omega,
                r,
                burning_area_m2,
            )?
        };

        // The equations.
        let forces = aero.force + weight;
        let t03 = t03_jet - mass.cg_rate_m_s * (2.0 * m);
        let t04 = thrust - mass.cg_accel_m_s2 * m - mass.cg_rate_m_s * (2.0 * mass.mass_rate_kg_s)
            + t04_jet;
        let mut derivative = [0.0; STATE_LEN];
        let mut acceleration_enu_m_s2 = DVec3::ZERO;
        let mut rail_force_n = 0.0;
        match phase {
            Phase::Free => {
                let t20 = -omega.cross(omega.cross(r * m)) + omega.cross(t03) + t04 + forces;
                let t21 = -omega.cross(mass.inertia_o * omega)
                    + (jet_gyration - mass.inertia_o_rate) * omega
                    + r.cross(weight)
                    + aero.moment
                    + thrust_moment;
                let determinant = mass.inertia_cg.determinant();
                if !(determinant.is_finite() && determinant > 0.0) {
                    return Err(SimError::Domain {
                        what: "determinant of the inertia about the centre of mass",
                        value: determinant,
                    });
                }
                let omega_dot = mass.inertia_cg.inverse() * (t21 - r.cross(t20));
                let a_body = t20 / m - omega_dot.cross(r);
                acceleration_enu_m_s2 = q.mul_vec3(a_body);
                let q_dot = quaternion_derivative(state.attitude, omega);
                derivative = [
                    v_o.x,
                    v_o.y,
                    v_o.z,
                    acceleration_enu_m_s2.x,
                    acceleration_enu_m_s2.y,
                    acceleration_enu_m_s2.z,
                    q_dot.w,
                    q_dot.x,
                    q_dot.y,
                    q_dot.z,
                    omega_dot.x,
                    omega_dot.y,
                    omega_dot.z,
                ];
            }
            Phase::Descent => {
                // A point mass: the canopies' drag and the weight, with no rotation. The mass
                // terms of `T04` stay, so a device that opens while a motor burns still feels the
                // thrust along the axis it froze at.
                let t20 = t04 + forces;
                acceleration_enu_m_s2 = q.mul_vec3(t20 / m);
                derivative[..3].copy_from_slice(&v_o.to_array());
                derivative[3..6].copy_from_slice(&acceleration_enu_m_s2.to_array());
            }
            Phase::Rail | Phase::Pad => {
                // No rotation: the rail supplies the moments and the force across the axis.
                let t20 = t04 + forces;
                let across = (t20.x * t20.x + t20.y * t20.y).sqrt();
                rail_force_n = t20.z - friction_coefficient * across;
                if phase == Phase::Rail {
                    let along = q.mul_vec3(DVec3::Z);
                    acceleration_enu_m_s2 = along * (rail_force_n / m);
                    derivative[..3].copy_from_slice(&v_o.to_array());
                    derivative[3..6].copy_from_slice(&acceleration_enu_m_s2.to_array());
                }
            }
        }

        Ok(Evaluation {
            derivative,
            mass,
            cg_enu_m,
            cg_velocity_enu_m_s,
            height_above_ground_m,
            vertical_speed_m_s,
            airspeed_m_s: aero.airspeed_m_s,
            mach: aero.mach,
            angle_of_attack_rad: aero.angle_of_attack_rad,
            dynamic_pressure_pa: aero.dynamic_pressure_pa,
            axial_coefficient: aero.axial_coefficient,
            thrust_n: thrust.z,
            rail_force_n,
            acceleration_enu_m_s2,
            recovery_drag_area_m2: if phase == Phase::Descent {
                recovery_drag_area_m2
            } else {
                0.0
            },
        })
    }

    /// The aerodynamic force (body axes) and its moment about the nose tip.
    ///
    /// - The axial force is `−q A C_A z_B` from the whole rocket's drag at the centre of mass's
    ///   airspeed; it acts along the axis, so it has no moment about the nose tip.
    /// - Each component's normal and side force comes from its own local flow: the nose tip's air
    ///   velocity plus `ω × p` at the component's small-angle centre of pressure `p` (at the
    ///   centre of mass's Mach number), which gives the aerodynamic damping in pitch and yaw.
    ///   `C_N` acts along the crossing air `ŵ`, `C_Y` along `z_B × ŵ`, at the stations their
    ///   moments give (`docs/physics/frames.md`).
    /// - Fin sets use `sin α` in place of their model's `α`, so their force vanishes when the air
    ///   comes from the tail as well as from the nose.
    fn aerodynamics(
        &self,
        air: &hpr_atmos::AirState,
        air_velocity_o_body: DVec3,
        omega: DVec3,
        cg_m: DVec3,
        burning_area_m2: f64,
    ) -> Result<Aerodynamics, SimError> {
        let rho = air.density_kg_m3;
        let sound = air.speed_of_sound_m_s;
        let area = self.reference_area_m2;
        let v_cg = air_velocity_o_body + omega.cross(cg_m);
        let speed = v_cg.length();
        let mut out = Aerodynamics {
            airspeed_m_s: speed,
            mach: speed / sound,
            ..Aerodynamics::default()
        };
        if rho <= 0.0 || speed < MIN_AIRSPEED_M_S {
            return Ok(out);
        }
        let (alpha, roll) = flow_angles(v_cg, speed);
        out.angle_of_attack_rad = alpha;
        let q = 0.5 * rho * speed * speed;
        out.dynamic_pressure_pa = q;
        let reynolds_per_m = speed / air.kinematic_viscosity_m2_s();
        let conditions = if burning_area_m2 > 0.0 {
            DragConditions::thrusting(reynolds_per_m, burning_area_m2)
        } else {
            DragConditions::coasting(reynolds_per_m)
        };
        let flow = Flow::new(out.mach, alpha, roll);
        let drag = self.aero.drag(&flow, &conditions)?;
        out.axial_coefficient = drag.axial_coefficient;
        out.force = DVec3::new(0.0, 0.0, -q * area * drag.axial_coefficient);

        // The normal force's range, checked once here, before the bodies' cached stations.
        flow.validate()?;
        for index in 0..self.aero.component_count() {
            // A fin set's station moves with Mach.
            let station = match self.body_stations_m.get(index) {
                Some(&station) => station,
                None => self.aero.component_station_m(index, out.mach)?,
            };
            let p = DVec3::new(0.0, 0.0, -station);
            let local = air_velocity_o_body + omega.cross(p);
            let local_speed = local.length();
            if local_speed < MIN_AIRSPEED_M_S {
                continue;
            }
            let (alpha_i, roll_i) = flow_angles(local, local_speed);
            let normal = self
                .aero
                .component_normal_force(index, &Flow::new(local_speed / sound, alpha_i, roll_i))?;
            // Fin normal force follows the crossflow `V sin α`, as the body terms do: the
            // small-angle slope times `sin α` rather than `α`, so it vanishes for axial flow either
            // way (ADR-011).
            let fin_scale = if index >= self.first_fin_index && alpha_i > 0.0 {
                alpha_i.sin() / alpha_i
            } else {
                1.0
            };
            let q_i = 0.5 * rho * local_speed * local_speed * area * fin_scale;
            let across = DVec3::new(roll_i.cos(), roll_i.sin(), 0.0);
            let side = DVec3::Z.cross(across);
            out.force += (across * normal.coefficient + side * normal.side_coefficient) * q_i;
            out.moment += (side * -normal.moment_m + across * normal.side_moment_m) * q_i;
        }
        Ok(out)
    }
}

/// The aerodynamic results of one evaluation.
#[derive(Debug, Clone, Copy, Default)]
struct Aerodynamics {
    force: DVec3,
    moment: DVec3,
    airspeed_m_s: f64,
    mach: f64,
    angle_of_attack_rad: f64,
    dynamic_pressure_pa: f64,
    axial_coefficient: f64,
}

/// The drag of the open recovery devices: `D = −½ ρ (C_D S) |v| v` on the centre of mass's air
/// velocity `air_velocity_cg_body` (body axes), with no moment about it.
///
/// Source: Knacke's steady drag on the drag area `C_D S` (`docs/physics/recovery.md`), the same
/// form RocketPy's parachute phase uses (`flight.py:2770-2774`, MIT). The airframe's own drag is
/// left out, as RocketPy leaves it out: the rocket's attitude under a canopy is not modelled.
fn canopy_drag(
    air: &hpr_atmos::AirState,
    air_velocity_cg_body: DVec3,
    drag_area_m2: f64,
) -> Aerodynamics {
    let speed = air_velocity_cg_body.length();
    let rho = air.density_kg_m3;
    let mut out = Aerodynamics {
        airspeed_m_s: speed,
        mach: speed / air.speed_of_sound_m_s,
        ..Aerodynamics::default()
    };
    if rho <= 0.0 || speed < MIN_AIRSPEED_M_S || drag_area_m2 <= 0.0 {
        return out;
    }
    let (alpha, _) = flow_angles(air_velocity_cg_body, speed);
    out.angle_of_attack_rad = alpha;
    out.dynamic_pressure_pa = 0.5 * rho * speed * speed;
    out.force = air_velocity_cg_body * (-0.5 * rho * drag_area_m2 * speed);
    out
}

/// The total angle of attack and the flow roll of a body moving at `v` (body axes) through still
/// air: `α` between `z_B` and `v`, and `φ` the direction the air crosses the body, from `x_B`
/// toward `y_B`, which is opposite the lateral velocity (`docs/physics/frames.md`).
fn flow_angles(v: DVec3, _speed: f64) -> (f64, f64) {
    // `atan2` keeps full precision near 0 and π, where `acos` loses half the digits.
    let alpha = v.x.hypot(v.y).atan2(v.z);
    let roll = if v.x == 0.0 && v.y == 0.0 {
        0.0
    } else {
        (-v.y).atan2(-v.x)
    };
    (alpha, roll)
}

/// `u vᵀ`.
fn outer(u: DVec3, v: DVec3) -> DMat3 {
    DMat3::from_cols(u * v.x, u * v.y, u * v.z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{UniformAir, analytic_environment, design};

    fn valetudo() -> Vehicle {
        let assembly = design("rocketpy-valetudo").assemble("example").unwrap();
        let aero = AeroModel::new(&assembly.layout).unwrap();
        Vehicle::new(assembly, aero).unwrap()
    }

    #[test]
    fn mass_rates_match_the_motor_and_stay_inside_the_interval() {
        // The mass rate from the differences is the motor's `−F/c`; a window ending just after `t`
        // keeps the stencil inside it without losing accuracy.
        let vehicle = valetudo();
        let motor = &vehicle.assembly.motors[0].mounted.motor;
        for (t, window) in [
            (1.5, (1.4, 1.6)),
            (1.5, (1.0, 1.50001)),
            (0.00002, (0.0, 0.1)),
        ] {
            let state = vehicle.mass_state(t, window);
            let expected = -motor.state(t).mass_flow_kg_s;
            assert!(
                (state.mass_rate_kg_s - expected).abs() <= 1e-6 * expected.abs(),
                "t {t}: {} vs {expected}",
                state.mass_rate_kg_s
            );
        }
        let coasting = vehicle.mass_state(10.0, (9.0, 11.0));
        assert_eq!(coasting.mass_rate_kg_s, 0.0);
        assert_eq!(coasting.inertia_o_rate, DMat3::ZERO);
    }

    #[test]
    fn rail_friction_is_coulomb_on_the_force_across_the_rail() {
        // In a vacuum on a rail at 60°, friction removes exactly μ g cos E from the acceleration
        // along the rail: the weight's share across the rail is the only normal force, and the
        // mass terms act along the axis.
        let vehicle = valetudo();
        let environment = analytic_environment(UniformAir::vacuum(), 9.806_65);
        let elevation = std::f64::consts::FRAC_PI_3;
        let state = State {
            position_enu_m: DVec3::new(0.0, 0.0, 10.0),
            velocity_enu_m_s: DVec3::ZERO,
            attitude: crate::rail::Rail {
                elevation_rad: elevation,
                ..crate::rail::Rail::vertical(3.0)
            }
            .attitude(),
            body_rate_rad_s: DVec3::ZERO,
        };
        let along = |mu: f64| {
            vehicle
                .evaluate(
                    &environment,
                    mu,
                    Conditions {
                        phase: Phase::Rail,
                        window: (1.0, 2.0),
                        drag_area_m2: 0.0,
                    },
                    1.5,
                    &state.to_array(),
                )
                .unwrap()
        };
        let (free, rubbing) = (along(0.0), along(0.3));
        let mass = vehicle.mass_state(1.5, (1.0, 2.0)).mass_kg;
        let expected = 0.3 * mass * 9.806_65 * elevation.cos();
        assert!((free.rail_force_n - rubbing.rail_force_n - expected).abs() < 1e-9 * expected);
        let direction = state.unit_attitude().mul_vec3(DVec3::Z);
        let difference = free.acceleration_enu_m_s2 - rubbing.acceleration_enu_m_s2;
        assert!((difference - direction * (expected / mass)).length() < 1e-12);
    }

    #[test]
    fn jet_damping_matches_the_classical_form() {
        // For an axisymmetric rocket turning slowly about a transverse axis in a vacuum, the
        // equations about the nose tip reduce to the classical jet-damping result about the centre
        // of mass: I_c ω̇ = [ṁ (r_e²/4 + l²) − İ_c] ω, with l from the centre of mass to the nozzle
        // exit and ṁ < 0. Every term on the right is computed here independently of the equations.
        let vehicle = valetudo();
        let environment = analytic_environment(UniformAir::vacuum(), 0.0);
        let placed = &vehicle.assembly.motors[0];
        let motor = &placed.mounted.motor;
        let exit_radius = motor.nozzle().map_or(0.0, |n| n.exit_radius_m);
        assert!(exit_radius > 0.0);
        let t = 1.5;
        let window = (1.0, 2.0);
        let rate = 0.2;
        let state = State {
            position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
            velocity_enu_m_s: DVec3::new(0.0, 0.0, 50.0),
            attitude: DQuat::IDENTITY,
            body_rate_rad_s: DVec3::new(0.0, rate, 0.0),
        };
        let evaluation = vehicle
            .evaluate(
                &environment,
                0.0,
                Conditions {
                    phase: Phase::Free,
                    window,
                    drag_area_m2: 0.0,
                },
                t,
                &state.to_array(),
            )
            .unwrap();
        let omega_dot = DVec3::from_slice(&evaluation.derivative[10..13]);

        let props = vehicle.assembly.mass_properties(t);
        let h = 1e-4;
        let inertia_rate = (vehicle
            .assembly
            .mass_properties(t + h)
            .inertia_kg_m2
            .y_axis
            .y
            - vehicle
                .assembly
                .mass_properties(t - h)
                .inertia_kg_m2
                .y_axis
                .y)
            / (2.0 * h);
        let mdot = -motor.state(t).mass_flow_kg_s;
        let lever = placed.nozzle_m.z - props.cg_m.z;
        let inertia = props.inertia_kg_m2.y_axis.y;
        let expected = (mdot * (0.25 * exit_radius * exit_radius + lever * lever) - inertia_rate)
            * rate
            / inertia;
        assert!(expected < 0.0, "jet damping must damp: {expected}");
        assert!(
            (omega_dot.y - expected).abs() <= 1e-6 * expected.abs(),
            "{} vs {expected}",
            omega_dot.y
        );
        assert!(omega_dot.x.abs() + omega_dot.z.abs() < 1e-12);
    }

    #[test]
    fn flow_angles_follow_the_frames_conventions() {
        // Moving along +z_B: no angle of attack.
        let (alpha, _) = flow_angles(DVec3::Z, 1.0);
        assert_eq!(alpha, 0.0);
        // Moving along +z_B and +x_B: the air crosses the body toward −x_B (φ = π).
        let v = DVec3::new(1.0, 0.0, 1.0);
        let (alpha, roll) = flow_angles(v, v.length());
        assert!((alpha - std::f64::consts::FRAC_PI_4).abs() < 1e-15);
        assert!((roll.abs() - std::f64::consts::PI).abs() < 1e-15);
        // Moving along −y_B only: the air crosses toward +y_B (φ = π/2), α = 90°.
        let (alpha, roll) = flow_angles(-DVec3::Y, 1.0);
        assert!((alpha - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
        assert!((roll - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
        // Tail first.
        let (alpha, _) = flow_angles(-DVec3::Z, 1.0);
        assert!((alpha - std::f64::consts::PI).abs() < 1e-15);
    }

    #[test]
    fn outer_product_is_u_v_transpose() {
        let m = outer(DVec3::new(1.0, 2.0, 3.0), DVec3::new(4.0, 5.0, 6.0));
        assert_eq!(m.row(1), DVec3::new(8.0, 10.0, 12.0));
        assert_eq!(m.col(2), DVec3::new(6.0, 12.0, 18.0));
    }
}
