//! Whole-flight tests: the analytic cases of M1.6 and Loft lessons L20, L24 and L25.

use std::f64::consts::PI;

use hpr_aero::Flow;
use hpr_atmos::ConstantWind;
use hpr_core::{DQuat, DVec3};

use crate::environment::Environment;
use crate::error::SimError;
use crate::events::{Direction, find_root};
use crate::flight::{EventKind, FlightSettings, Simulation, Termination, UserEvent};
use crate::rail::Rail;
use crate::recorder::{Channel, Recorder};
use crate::state::State;
use crate::testing::{
    UniformAir, analytic_environment, constant_drag, design, site, windy_environment,
};

const G: f64 = 9.806_65;

/// Valetudo (K400C), whose motor burns out at 3.26 s: the analytic cases fly it from t = 10 s.
fn valetudo(environment: Environment, settings: FlightSettings) -> Simulation {
    Simulation::new(
        &design("rocketpy-valetudo"),
        "example",
        environment,
        Rail::vertical(3.0),
        settings,
    )
    .unwrap()
}

/// Valetudo with its thrust curve taken as measured at standard sea-level pressure, so that a
/// flight in a vacuum carries the pressure term `p_ref A_e`. The design itself, like RocketPy's
/// example, gives no reference pressure and so no correction.
fn valetudo_measured_at_sea_level(
    environment: Environment,
    settings: FlightSettings,
) -> Simulation {
    let mut rocket = serde_json::to_value(design("rocketpy-valetudo")).unwrap();
    rocket["configurations"][0]["motors"][0]["motor"]["nozzle"]["reference_pressure_pa"] =
        serde_json::json!(hpr_motor::motor::STANDARD_SEA_LEVEL_PRESSURE_PA);
    Simulation::new(
        &serde_json::from_value(rocket).unwrap(),
        "example",
        environment,
        Rail::vertical(3.0),
        settings,
    )
    .unwrap()
}

fn capped(max_time_s: f64) -> FlightSettings {
    FlightSettings {
        max_time_s,
        ..FlightSettings::default()
    }
}

/// Column `name` of every row.
fn column(recorder: &Recorder, name: &str) -> Vec<f64> {
    let index = recorder
        .columns()
        .iter()
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("no column {name}"));
    recorder.rows().iter().map(|row| row[index]).collect()
}

#[test]
fn vacuum_ballistic_flight_follows_the_parabola() {
    // In a vacuum under uniform gravity the centre of mass follows a parabola whatever the body
    // does, and the angular momentum about it is constant. The rocket starts tumbling and spinning,
    // with the nose tip (the state's reference point) well away from the centre of mass.
    let t0 = 10.0;
    let sim = valetudo(
        analytic_environment(UniformAir::vacuum(), G),
        FlightSettings::default(),
    );
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 500.0),
        velocity_enu_m_s: DVec3::new(30.0, -20.0, 80.0),
        attitude: DQuat::from_rotation_z(0.3)
            * DQuat::from_rotation_y(0.4)
            * DQuat::from_rotation_x(0.2),
        body_rate_rad_s: DVec3::new(0.4, -0.3, 3.0),
    };
    let mass = sim.assembly().mass_properties(t0);
    let q = state.unit_attitude();
    let cg0 = state.point_enu_m(mass.cg_m);
    let v_cg0 = state.velocity_enu_m_s + q.mul_vec3(state.body_rate_rad_s.cross(mass.cg_m));
    let h0 = q.mul_vec3(mass.inertia_kg_m2 * state.body_rate_rad_s);
    let parabola = |t: f64| {
        let tau = t - t0;
        cg0 + v_cg0 * tau - DVec3::Z * (0.5 * G * tau * tau)
    };

    let mut recorder = Recorder::new(
        vec![
            Channel::Time,
            Channel::CgPosition,
            Channel::Attitude,
            Channel::BodyRates,
        ],
        Some(0.5),
    )
    .unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::GroundHit);

    let mut worst_position: f64 = 0.0;
    let mut worst_momentum: f64 = 0.0;
    for row in recorder.rows() {
        let cg = DVec3::new(row[1], row[2], row[3]);
        worst_position = worst_position.max((cg - parabola(row[0])).length());
        let q = DQuat::from_xyzw(row[5], row[6], row[7], row[4]);
        let rates = DVec3::new(row[8], row[9], row[10]);
        let h = q.mul_vec3(mass.inertia_kg_m2 * rates);
        worst_momentum = worst_momentum.max((h - h0).length() / h0.length());
    }

    // Apogee and ground contact on the parabola, from the ellipsoidal height of the closed form.
    let frame = sim.environment().earth.frame();
    let ground_m = frame.origin().height_m;
    let height = |t: f64| frame.geodetic_from_enu(parabola(t)).unwrap().height_m - ground_m;
    let rate = |t: f64| (height(t + 1e-4) - height(t - 1e-4)) / 2e-4;
    let apogee_guess = t0 + v_cg0.z / G;
    let apogee_s = find_root(
        rate,
        apogee_guess - 1.0,
        apogee_guess + 1.0,
        rate(apogee_guess - 1.0),
        rate(apogee_guess + 1.0),
        1e-12,
    )
    .unwrap();
    let ground_s = find_root(
        height,
        apogee_s,
        apogee_s + 60.0,
        height(apogee_s),
        height(apogee_s + 60.0),
        1e-12,
    )
    .unwrap();
    let apogee = result.event(EventKind::Apogee).unwrap().sample;
    let ground = result.event(EventKind::GroundHit).unwrap().sample;
    // Measured at the default tolerances: 1.7e-6 m over 22 s and 700 m, momentum 7.8e-7 after
    // 60 rad of spin, apogee −4.7e-7 s and ground contact 1.1e-8 s from the closed form.
    assert!(worst_position < 5e-6, "{worst_position}");
    assert!(worst_momentum < 2.5e-6, "{worst_momentum}");
    assert!((apogee.time_s - apogee_s).abs() < 1e-6);
    assert!((ground.time_s - ground_s).abs() < 1e-6);
    assert!((ground.cg_enu_m - parabola(ground_s)).length() < 5e-6);
}

#[test]
fn powered_vertical_climb_in_vacuum_integrates_the_axial_equation() {
    // Mid-burn, vertical, in a vacuum under uniform gravity, with no rotation: the nose tip's
    // velocity is v0 + ∫ ((T − m r″ − 2ṁ r′ + m̈(n − r))/m − g) dt along the axis. The integrand
    // comes here from the motor and the assembly directly, and the quadrature splits at the
    // thrust curve's knots.
    let (t0, t1) = (0.5, 3.0);
    let sim =
        valetudo_measured_at_sea_level(analytic_environment(UniformAir::vacuum(), G), capped(t1));
    let assembly = sim.assembly();
    let placed = &assembly.motors[0];
    let motor = &placed.mounted.motor;
    let h = 1e-4;
    let props = |t: f64| assembly.mass_properties(t);
    // Rates by central differences centred inside each knot interval [a, b] and extended
    // linearly to t (exact for the quadratic mass of a linear thrust segment).
    let integrand = |t: f64, a: f64, b: f64| {
        let c = t.clamp(a + h, b - h);
        let (m, r) = (props(t).mass_kg, props(t).cg_m.z);
        let r_mid = props(c).cg_m.z;
        let r2 = (props(c + h).cg_m.z - 2.0 * r_mid + props(c - h).cg_m.z) / (h * h);
        let r1 = (props(c + h).cg_m.z - props(c - h).cg_m.z) / (2.0 * h) + (t - c) * r2;
        let mdot = -motor.state(t).mass_flow_kg_s;
        let mddot =
            -(motor.state(c + h).mass_flow_kg_s - motor.state(c - h).mass_flow_kg_s) / (2.0 * h);
        let thrust = motor.thrust_at_pressure_n(t, 0.0);
        (thrust - m * r2 - 2.0 * mdot * r1 + mddot * (placed.nozzle_m.z - r)) / m - G
    };
    let mut knots: Vec<f64> = motor
        .curve()
        .times_s()
        .iter()
        .copied()
        .filter(|t| *t > t0 && *t < t1)
        .collect();
    knots.insert(0, t0);
    knots.push(t1);
    let tolerance = hpr_core::quadrature::Tolerance {
        relative: 1e-10,
        absolute: 1e-10,
        max_intervals: 4000,
    };
    let delta_v: f64 = knots
        .windows(2)
        .map(|w| {
            hpr_core::quadrature::integrate_scalar(
                |t| integrand(t, w[0], w[1]),
                w[0],
                w[1],
                tolerance,
            )
            .unwrap()
        })
        .sum();
    let v0 = 50.0;
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 300.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, v0),
        attitude: DQuat::IDENTITY,
        body_rate_rad_s: DVec3::ZERO,
    };
    let result = sim.run_free(t0, state, &mut ()).unwrap();
    assert_eq!(result.termination, Termination::TimeCap);
    let end = result.final_sample.state;
    assert!(
        end.body_rate_rad_s.length() < 1e-9,
        "{:?}",
        end.body_rate_rad_s
    );
    assert!(end.velocity_enu_m_s.x.abs() + end.velocity_enu_m_s.y.abs() < 1e-9);
    let error = end.velocity_enu_m_s.z - (v0 + delta_v);
    // Measured: 4.3e-8 m/s of 185 m/s. Dropping the m̈(n − r) term alone moves it by about
    // 0.05 m/s.
    assert!(error.abs() < 1.5e-7, "{error} of {}", v0 + delta_v);
}

#[test]
fn nose_down_fall_reaches_the_closed_form_terminal_velocity() {
    // A body falling nose down through uniform air with a constant drag coefficient:
    // `v = v_t tanh(g t / v_t)`, `v_t = √(2 m g / (ρ A C_D))`.
    let t0 = 10.0;
    let cd = 0.5;
    let air = UniformAir::sea_level();
    let sim = valetudo(analytic_environment(air, G), capped(t0 + 150.0))
        .with_drag_table(constant_drag(cd));
    let mass = sim.assembly().mass_properties(t0).mass_kg;
    let area = sim.aero().reference_area_m2();
    let terminal = (2.0 * mass * G / (air.0.density_kg_m3 * area * cd)).sqrt();
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 40_000.0),
        velocity_enu_m_s: DVec3::ZERO,
        attitude: DQuat::from_rotation_x(PI),
        body_rate_rad_s: DVec3::ZERO,
    };
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::Velocity], Some(5.0)).unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::TimeCap);
    let times = column(&recorder, "time_s");
    let fall = column(&recorder, "velocity_up_m_s");
    let mut worst: f64 = 0.0;
    for (t, v) in times.iter().zip(&fall) {
        let expected = -terminal * (G * (t - t0) / terminal).tanh();
        worst = worst.max((v - expected).abs() / terminal);
    }
    let last = fall[fall.len() - 1];
    // Measured: 5.5e-9 of v_t; after 150 s the speed is v_t to 1e-5.
    assert!(worst < 2.5e-8, "{worst}");
    assert!(
        (-last / terminal - 1.0).abs() < 2e-5,
        "{last} vs {terminal}"
    );
}

#[test]
fn torque_free_precession_matches_eulers_solution() {
    // An axisymmetric body with no torque: the spin `ω_z` is constant and the transverse rate
    // turns in body axes at `Ω = (I_a − I_t) ω_z / I_t`; the angular momentum is fixed in the
    // launch frame.
    let t0 = 10.0;
    let sim = valetudo(
        analytic_environment(UniformAir::vacuum(), 0.0),
        capped(t0 + 3.0),
    );
    let mass = sim.assembly().mass_properties(t0);
    let inertia = mass.inertia_kg_m2;
    let (i_t, i_a) = (inertia.x_axis.x, inertia.z_axis.z);
    assert!((inertia.y_axis.y - i_t).abs() <= 1e-12 * i_t);
    assert!(inertia.x_axis.y.abs() + inertia.x_axis.z.abs() + inertia.y_axis.z.abs() <= 1e-15);
    let omega0 = DVec3::new(0.3, 0.1, 25.0);
    let spin_rate = (i_a - i_t) / i_t * omega0.z;
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, 10.0),
        attitude: DQuat::IDENTITY,
        body_rate_rad_s: omega0,
    };
    let mut recorder = Recorder::new(
        vec![Channel::Time, Channel::Attitude, Channel::BodyRates],
        Some(0.01),
    )
    .unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::TimeCap);
    let transverse = omega0.x.hypot(omega0.y);
    let phase0 = omega0.y.atan2(omega0.x);
    let h0 = inertia * omega0;
    let (mut worst_rate, mut worst_momentum): (f64, f64) = (0.0, 0.0);
    for row in recorder.rows() {
        let tau = row[0] - t0;
        let expected = DVec3::new(
            transverse * (spin_rate * tau + phase0).cos(),
            transverse * (spin_rate * tau + phase0).sin(),
            omega0.z,
        );
        let rates = DVec3::new(row[5], row[6], row[7]);
        worst_rate = worst_rate.max((rates - expected).length());
        let q = DQuat::from_xyzw(row[2], row[3], row[4], row[1]);
        let h = q.mul_vec3(inertia * rates);
        worst_momentum = worst_momentum.max((h - h0).length() / h0.length());
    }
    // Measured at the default tolerances: 4.7e-7 rad/s (2e-8 of the spin) and a momentum drift
    // of 1.5e-6 after 75 rad of spin, about the tolerance per radian.
    assert!(worst_rate < 1.5e-6, "{worst_rate}");
    assert!(worst_momentum < 5e-6, "{worst_momentum}");
}

#[test]
fn pitch_oscillation_matches_linear_theory() {
    // At constant speed, with no drag and no gravity, a small angle of attack oscillates. With
    // `ℓᵢ` each component's lever arm aft of the centre of mass and `q̄A` the dynamic pressure on
    // the reference area, `Z = q̄A Σ C_Nαᵢ`, `K₁ = q̄A Σ C_Nαᵢ ℓᵢ` and `K₂ = q̄A Σ C_Nαᵢ ℓᵢ²`.
    // The angle of attack `α` and pitch rate `θ̇` obey
    //   α̇ = −Z/(mV) α + (1 − K₁/(mV²)) θ̇,   I θ̈ = −K₁ α − K₂/V θ̇
    // (the normal force turning the path, the restoring moment, and the damping from the airspeed
    // the rotation adds at each component), whose eigenvalues give the damped frequency and decay.
    let t0 = 10.0;
    let speed = 100.0;
    let air = UniformAir::sea_level();
    let sim = valetudo(analytic_environment(air, 0.0), capped(t0 + 10.0))
        .with_drag_table(constant_drag(0.0))
        .with_event(UserEvent {
            name: "pitch rate crosses zero".to_owned(),
            direction: Direction::Either,
            function: Box::new(|sample| sample.state.body_rate_rad_s.y),
        });
    let mass = sim.assembly().mass_properties(t0);
    let (m, x_cg, inertia) = (mass.mass_kg, -mass.cg_m.z, mass.inertia_kg_m2.y_axis.y);
    let aero = sim.aero();
    let mach = speed / air.0.speed_of_sound_m_s;
    let q_area = 0.5 * air.0.density_kg_m3 * speed * speed * aero.reference_area_m2();
    let (mut z, mut k1, mut k2) = (0.0, 0.0, 0.0);
    for index in 0..aero.component_count() {
        let slope = aero
            .component_normal_force(index, &Flow::new(mach, 1e-7, 0.0))
            .unwrap()
            .slope_per_rad;
        let lever = aero.component_station_m(index, mach).unwrap() - x_cg;
        z += q_area * slope;
        k1 += q_area * slope * lever;
        k2 += q_area * slope * lever * lever;
    }
    let (a11, a12) = (-z / (m * speed), 1.0 - k1 / (m * speed * speed));
    let (a21, a22) = (-k1 / inertia, -k2 / (inertia * speed));
    let trace = a11 + a22;
    let determinant = a11 * a22 - a12 * a21;
    let damped = (determinant - 0.25 * trace * trace).sqrt();
    let period = 2.0 * PI / damped;

    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, speed),
        attitude: DQuat::from_rotation_y(0.005),
        body_rate_rad_s: DVec3::ZERO,
    };
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::BodyRates], Some(0.001)).unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    let crossings: Vec<f64> = result
        .events
        .iter()
        .filter(|event| event.kind == EventKind::User(0))
        .map(|event| event.sample.time_s)
        .collect();
    assert!(crossings.len() >= 6, "{crossings:?}");
    let measured =
        2.0 * (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f64;

    // Decay: the largest |θ̇| in each half period falls by exp(trace/2 · period/2) per half.
    let times = column(&recorder, "time_s");
    let rates = column(&recorder, "body_rate_y_rad_s");
    let peaks: Vec<f64> = crossings
        .windows(2)
        .map(|w| {
            times
                .iter()
                .zip(&rates)
                .filter(|(t, _)| **t > w[0] && **t < w[1])
                .fold(0.0_f64, |peak, (_, r)| peak.max(r.abs()))
        })
        .collect();
    let decay = (peaks[peaks.len() - 1] / peaks[0]).ln() / (peaks.len() - 1) as f64;
    let expected_decay = 0.25 * trace * period;
    // Measured: the period 1.44965 s against 1.44954 s (8e-5), the decay 0.2% from the theory's.
    assert!((measured / period - 1.0).abs() < 3e-4);
    assert!((decay / expected_decay - 1.0).abs() < 0.01);
}

#[test]
fn stable_rocket_weathercocks_into_crosswind() {
    // Loft lesson L20: a 3-DOF boost drifted downwind. A stable 6-DOF rocket turns into the wind
    // off the rail and flies upwind. Wind 5 m/s from the west (toward +x, east).
    let fly = |environment| {
        valetudo(environment, FlightSettings::default())
            .run(&mut ())
            .unwrap()
    };
    let calm = fly(Environment::standard(site()).unwrap());
    let windy = fly(windy_environment(ConstantWind::new(5.0, 1.5 * PI).unwrap()));
    let at = |result: &crate::flight::FlightResult, kind| result.event(kind).unwrap().sample;
    let calm_apogee = at(&calm, EventKind::Apogee);
    let windy_apogee = at(&windy, EventKind::Apogee);
    let burnout = at(&windy, EventKind::Burnout);
    let axis_east = burnout.state.unit_attitude().mul_vec3(DVec3::Z).x;
    // Measured: calm −0.86 m (Earth rotation), windy −86 m, axis 0.12 west of vertical at burnout.
    assert!(calm_apogee.cg_enu_m.x.abs() < 5.0);
    assert!(axis_east < -0.05, "{axis_east}");
    assert!(windy_apogee.cg_enu_m.x - calm_apogee.cg_enu_m.x < -50.0);
}

#[test]
fn repeated_runs_are_bit_identical() {
    // Loft lesson L24: `simulate()` mutated its inputs, so repeated runs differed. The same
    // simulation run twice, and a second one built the same way, give the same bits.
    let build = || {
        valetudo(
            windy_environment(ConstantWind::new(4.0, 1.0).unwrap()),
            capped(40.0),
        )
    };
    let record = |sim: &Simulation| {
        let mut recorder = Recorder::new(Channel::ALL.to_vec(), None).unwrap();
        let result = sim.run(&mut recorder).unwrap();
        let bits: Vec<Vec<u64>> = recorder
            .rows()
            .iter()
            .map(|row| row.iter().map(|v| v.to_bits()).collect())
            .collect();
        (bits, result)
    };
    let sim = build();
    let first = record(&sim);
    let second = record(&sim);
    let third = record(&build());
    assert!(first.0.len() > 100);
    assert_eq!(first.0, second.0);
    assert_eq!(first.0, third.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.1, third.1);
}

#[test]
fn termination_reason_distinguishes_no_liftoff_time_cap_and_step_limit() {
    // Loft lesson L25: every stop before the time cap was called "step budget", even a rocket that
    // never lifted off.
    let rocket = design("rocketpy-valetudo");
    let fly = |rail: Rail, settings: FlightSettings| {
        Simulation::new(
            &rocket,
            "example",
            Environment::standard(site()).unwrap(),
            rail,
            settings,
        )
        .unwrap()
        .run(&mut ())
        .unwrap()
    };
    let rail = Rail::vertical(3.0);
    // Friction needs a normal force: on a rail 30° from vertical, the weight's share across the rail.
    let stuck = Rail {
        elevation_rad: PI / 3.0,
        friction_coefficient: 20.0,
        ..rail
    };
    let normal = fly(rail, FlightSettings::default());
    assert_eq!(normal.termination, Termination::GroundHit);
    let no_liftoff = fly(stuck, FlightSettings::default());
    assert_eq!(no_liftoff.termination, Termination::NoLiftoff);
    assert!(no_liftoff.event(EventKind::Liftoff).is_none());
    assert_eq!(no_liftoff.final_sample.state.velocity_enu_m_s, DVec3::ZERO);
    let capped_flight = fly(rail, capped(5.0));
    assert_eq!(capped_flight.termination, Termination::TimeCap);
    assert_eq!(capped_flight.final_sample.time_s, 5.0);
    let limited = fly(
        rail,
        FlightSettings {
            step_limit: 40,
            ..FlightSettings::default()
        },
    );
    assert_eq!(limited.termination, Termination::StepLimit);
    assert!(limited.final_sample.time_s < normal.final_sample.time_s);
    // A rail far longer than the burn: the rocket coasts to a stop on it and settles back.
    let stalled = fly(Rail::vertical(2000.0), FlightSettings::default());
    assert_eq!(stalled.termination, Termination::StalledOnRail);
    let kinds: Vec<EventKind> = stalled.events.iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [EventKind::Liftoff, EventKind::Burnout]);
    assert_eq!(stalled.final_sample.state.velocity_enu_m_s, DVec3::ZERO);
}

#[test]
fn a_calm_vertical_flight_falls_tail_first_and_lands() {
    // With no wind and no Earth rotation the rocket stops at apogee pointing up and falls tail
    // first. The fins' normal force must vanish for that axial flow; a force linear in α would
    // stay large at α = π and flip direction with rounding noise, collapsing the step size.
    use hpr_core::earth::{Earth, EarthRotation, GravityModel};
    use hpr_core::gravity::NormalGravity;
    let earth = Earth::new(
        NormalGravity::wgs84(),
        site(),
        GravityModel::Ellipsoidal,
        EarthRotation::Ignore,
    )
    .unwrap();
    let environment = Environment::new(
        earth,
        hpr_atmos::AtmosphereModel::default(),
        ConstantWind::calm(),
    );
    let result = valetudo(environment, FlightSettings::default())
        .run(&mut ())
        .unwrap();
    assert_eq!(result.termination, Termination::GroundHit);
    assert!(result.stats.evaluations < 20_000, "{:?}", result.stats);
}

#[test]
fn burnout_is_recorded_when_an_event_ends_the_step_on_it() {
    // A user event whose root is exactly the burnout stop time ends the step there as an event.
    let sim = valetudo(Environment::standard(site()).unwrap(), capped(20.0));
    let burnout_s = sim.assembly().motors[0].mounted.motor.burnout_time_s();
    let sim = sim.with_event(UserEvent {
        name: "burnout clock".to_owned(),
        direction: Direction::Rising,
        function: Box::new(move |sample| sample.time_s - burnout_s),
    });
    let result = sim.run(&mut ()).unwrap();
    let burnouts: Vec<f64> = result
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Burnout)
        .map(|e| e.sample.time_s)
        .collect();
    let user: Vec<f64> = result
        .events
        .iter()
        .filter(|e| e.kind == EventKind::User(0))
        .map(|e| e.sample.time_s)
        .collect();
    assert_eq!(burnouts.len(), 1, "{:?}", result.events);
    assert_eq!(user.len(), 1);
    assert!(
        (user[0] - burnouts[0]).abs() < 1e-9,
        "{user:?} {burnouts:?}"
    );
}

#[test]
fn observer_errors_stop_the_flight_and_bad_starts_are_refused() {
    struct Quitter;
    impl crate::recorder::Observer for Quitter {
        fn step(&mut self, step: &dyn crate::recorder::FlightStep) -> Result<(), SimError> {
            if step.end_s() > 2.0 {
                Err(SimError::Domain {
                    what: "observer stop",
                    value: step.end_s(),
                })
            } else {
                Ok(())
            }
        }
    }
    let sim = valetudo(
        Environment::standard(site()).unwrap(),
        FlightSettings::default(),
    );
    let error = sim.run(&mut Quitter).unwrap_err();
    assert!(
        matches!(error, SimError::Domain { what: "observer stop", value } if value > 2.0),
        "{error:?}"
    );
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 100.0),
        velocity_enu_m_s: DVec3::ZERO,
        attitude: DQuat::IDENTITY,
        body_rate_rad_s: DVec3::ZERO,
    };
    assert!(sim.run_free(-1.0, state, &mut ()).is_err());
    let underground = State {
        position_enu_m: DVec3::new(0.0, 0.0, -5.0),
        ..state
    };
    assert!(sim.run_free(10.0, underground, &mut ()).is_err());
}

#[test]
fn rk4_and_dormand_prince_fly_the_same_trajectory() {
    let fly = |method| {
        let settings = FlightSettings {
            method,
            ..FlightSettings::default()
        };
        valetudo(
            windy_environment(ConstantWind::new(3.0, 0.5).unwrap()),
            settings,
        )
        .run(&mut ())
        .unwrap()
    };
    let adaptive = fly(crate::integrator::Method::default());
    let fixed = fly(crate::integrator::Method::Rk4 { step_s: 0.002 });
    assert_eq!(fixed.termination, Termination::GroundHit);
    let apogee = |r: &crate::flight::FlightResult| r.event(EventKind::Apogee).unwrap().sample;
    let (a, b) = (apogee(&adaptive), apogee(&fixed));
    // Measured: 1.2e-5 m and 2.1e-7 s, Dormand–Prince's own error at its tolerance; RK4 gives the
    // same apogee at 2, 1 and 0.5 ms. Before motors were evaluated inside their burn at the
    // interval ends, RK4 converged at first order (5.1 mm at 2 ms).
    assert!((a.height_above_ground_m - b.height_above_ground_m).abs() < 4e-5);
    assert!((a.time_s - b.time_s).abs() < 1e-6);
    assert!((a.cg_enu_m - b.cg_enu_m).length() < 4e-5);
}

#[test]
fn a_recorder_is_cleared_between_flights_and_simulations_are_shareable() {
    fn shareable<T: Send + Sync>() {}
    shareable::<Simulation>();
    shareable::<Recorder>();
    shareable::<Environment>();
    assert!(Recorder::new(vec![Channel::Time], Some(0.0)).is_err());
    let sim = valetudo(Environment::standard(site()).unwrap(), capped(8.0));
    let mut recorder = Recorder::new(vec![Channel::Time], Some(0.1)).unwrap();
    sim.run(&mut recorder).unwrap();
    let first = recorder.rows().to_vec();
    recorder.clear();
    sim.run(&mut recorder).unwrap();
    assert_eq!(recorder.rows(), first);
    let times = column(&recorder, "time_s");
    assert!(times.windows(2).all(|w| w[1] > w[0]), "no repeated times");
}

#[test]
fn flight_events_come_in_order_and_the_recorder_keeps_its_interval() {
    let sim = valetudo(
        Environment::standard(site()).unwrap(),
        FlightSettings::default(),
    );
    let mut recorder =
        Recorder::new(vec![Channel::Time, Channel::HeightAboveGround], Some(0.25)).unwrap();
    let result = sim.run(&mut recorder).unwrap();
    let kinds: Vec<EventKind> = result.events.iter().map(|event| event.kind).collect();
    assert_eq!(
        kinds,
        [
            EventKind::Liftoff,
            EventKind::RailExit,
            EventKind::Burnout,
            EventKind::Apogee,
            EventKind::GroundHit
        ]
    );
    let times = column(&recorder, "time_s");
    assert!(times.windows(2).all(|w| w[1] >= w[0]));
    let event_times: Vec<f64> = result.events.iter().map(|e| e.sample.time_s).collect();
    for t in &times {
        let on_grid = ((t / 0.25).round() * 0.25 - t).abs() < 1e-12;
        assert!(on_grid || event_times.contains(t), "{t}");
    }
    let apogee = result.event(EventKind::Apogee).unwrap().sample;
    let heights = column(&recorder, "height_above_ground_m");
    assert!(
        heights
            .iter()
            .all(|h| *h <= apogee.height_above_ground_m + 1e-9)
    );
    assert!(apogee.vertical_speed_m_s.abs() < 1e-6);
    let ground = result.event(EventKind::GroundHit).unwrap().sample;
    assert!(ground.height_above_ground_m.abs() < 1e-6);
}

#[test]
fn a_flight_on_a_drag_table_flies_through_mach_1() {
    // The normal force covers Mach 0 to 5 since M1.8a, so with a drag table in place of the
    // buildup the same rocket passes Mach 1 and lands.
    struct Fastest(f64);
    impl crate::recorder::Observer for Fastest {
        fn step(&mut self, step: &dyn crate::recorder::FlightStep) -> Result<(), SimError> {
            self.0 = self.0.max(step.sample(step.end_s())?.mach);
            Ok(())
        }
    }
    let sim = Simulation::new(
        &design("synthetic-54mm-three-fin"),
        "i175",
        Environment::standard(site()).unwrap(),
        Rail::vertical(2.0),
        FlightSettings::default(),
    )
    .unwrap()
    .with_drag_table(constant_drag(0.5));
    let mut fastest = Fastest(0.0);
    let result = sim.run(&mut fastest).unwrap();
    assert_eq!(result.termination, Termination::GroundHit);
    assert!((1.05..1.6).contains(&fastest.0), "{}", fastest.0);
}

#[test]
fn a_supersonic_flight_flies_on_the_drag_buildup() {
    // The drag buildup covers Mach 0 to 5 since M1.8b1 (ADR-028), so the same rocket on its own
    // drag passes Mach 1 and lands, slower than on a constant 0.5, since its own drag rises
    // through Mach 1.
    struct Fastest(f64);
    impl crate::recorder::Observer for Fastest {
        fn step(&mut self, step: &dyn crate::recorder::FlightStep) -> Result<(), SimError> {
            self.0 = self.0.max(step.sample(step.end_s())?.mach);
            Ok(())
        }
    }
    let sim = Simulation::new(
        &design("synthetic-54mm-three-fin"),
        "i175",
        Environment::standard(site()).unwrap(),
        Rail::vertical(2.0),
        FlightSettings::default(),
    )
    .unwrap();
    let mut fastest = Fastest(0.0);
    let result = sim.run(&mut fastest).unwrap();
    assert_eq!(result.termination, Termination::GroundHit);
    let on_table = {
        let mut table = Fastest(0.0);
        Simulation::new(
            &design("synthetic-54mm-three-fin"),
            "i175",
            Environment::standard(site()).unwrap(),
            Rail::vertical(2.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_drag_table(constant_drag(0.5))
        .run(&mut table)
        .unwrap();
        table.0
    };
    // Measured: Mach 1.093 on the buildup against 1.192 on the table.
    assert!((fastest.0 - 1.093).abs() < 0.005, "{}", fastest.0);
    assert!(
        fastest.0 < on_table - 0.05,
        "{} against {on_table}",
        fastest.0
    );
}

/// M1.8's roll bullet: canted fins spin the rocket to the roll rate where their forcing and their
/// damping balance. At constant speed, with no drag and no gravity and the axis along the flight,
/// the roll obeys `I ṗ = q A d (C_l0 + C_lp p d/2V)`, so `p` rises as `p_eq (1 − e^(−kt))`. For a
/// trapezoid in subsonic flow the fin's slope cancels between the two (Barrowman 1967 eq. 3-35
/// and 3-48, Niskanen 2009 eq. 3.73 with the fin's own slope in both):
/// `p_eq = −δ V A_fin (r_t + y_MAC) k_T(B) / (k_R(B) Σ)`, with `Σ = ∫ξ² c dξ` (Niskanen eq. 3.70),
/// and `k = q N a Σ k_R(B)/(V I)`, `a` the fin's slope per unit of its area (Barrowman eq. 3-6).
/// Positive cant turns fin 0's leading edge toward `−y_B`, so the rocket rolls toward `−z_B`.
#[test]
fn canted_fins_spin_to_the_analytic_balance() {
    let (t0, speed, cant) = (10.0, 100.0, 1f64.to_radians());
    let air = UniformAir::sea_level();
    let mut rocket = serde_json::to_value(design("rocketpy-valetudo")).unwrap();
    rocket["stages"][0]["components"][1]["children"][1]["part"]["fin_set"]["cant_rad"] =
        serde_json::json!(cant);
    let sim = Simulation::new(
        &serde_json::from_value(rocket).unwrap(),
        "example",
        analytic_environment(air, 0.0),
        Rail::vertical(3.0),
        capped(t0 + 12.0),
    )
    .unwrap()
    .with_drag_table(constant_drag(0.0));

    // The closed forms for Valetudo's three trapezoidal fins.
    let (c_r, c_t, s, x_t, n) = (0.058, 0.018, 0.077, 0.04, 3.0);
    let aero = sim.aero();
    let set = &aero.fin_sets()[0];
    assert_eq!(set.cant_rad, cant);
    let r = set.body_radius_m;
    let area = 0.5 * s * (c_r + c_t);
    let y_mac = s * (c_r + 2.0 * c_t) / (3.0 * (c_r + c_t));
    let sigma = 0.5 * (c_r + c_t) * r * r * s
        + (c_r + 2.0 * c_t) / 3.0 * r * s * s
        + (c_r + 3.0 * c_t) / 12.0 * s * s * s;
    let k_t = hpr_aero::roll_forcing_interference(s, r).unwrap();
    let k_r = hpr_aero::roll_damping_interference(s, r, c_t / c_r).unwrap();
    let p_eq = -cant * speed * area * (r + y_mac) * k_t / (k_r * sigma);
    let mach = speed / air.0.speed_of_sound_m_s;
    let beta = (1.0 - mach * mach).sqrt();
    let midchord = ((x_t + 0.5 * c_t - 0.5 * c_r) / s).atan();
    let f = beta * s * s / (area * midchord.cos());
    let per_area = 2.0 * PI * s * s / area / (1.0 + (1.0 + f * f).sqrt());
    let q = 0.5 * air.0.density_kg_m3 * speed * speed;
    let inertia = sim.assembly().mass_properties(t0).inertia_kg_m2.z_axis.z;
    let k = q * n * per_area * sigma * k_r / (speed * inertia);
    let steady = aero.steady_roll_rate_rad_s(mach, speed).unwrap();
    assert!(
        ((steady - p_eq) / p_eq).abs() < 1e-12,
        "{steady} against {p_eq}"
    );
    // The guide's worked example (docs/physics/aero.md, Roll: forcing and damping).
    assert!(
        (k_t - 0.935).abs() < 5e-4 && (k_r - 1.228).abs() < 5e-4,
        "{k_t} {k_r}"
    );
    assert!(
        (p_eq + 16.948).abs() < 5e-4 && (1.0 / k - 0.476).abs() < 5e-4,
        "{p_eq} {k}"
    );

    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, speed),
        attitude: DQuat::IDENTITY,
        body_rate_rad_s: DVec3::ZERO,
    };
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::BodyRates], Some(0.001)).unwrap();
    sim.run_free(t0, state, &mut recorder).unwrap();
    let times = column(&recorder, "time_s");
    let rates = column(&recorder, "body_rate_z_rad_s");
    // Settled: over 20 time constants on, within 1e-6 of the balance. Measured: −16.948242838
    // rad/s against −16.948242838 (1e-11), and 2e-10 one time constant in.
    assert!(12.0 * k > 20.0, "k = {k}");
    let last = *rates.last().unwrap();
    assert!(
        p_eq < 0.0 && ((last - p_eq) / p_eq).abs() < 1e-6,
        "{last} against {p_eq}"
    );
    // On the way: one time constant in, `1 − 1/e` of the way, sampled at the nearest millisecond.
    let at = t0 + 1.0 / k;
    let i = times
        .iter()
        .position(|&t| t >= at)
        .unwrap_or(times.len() - 1);
    let want = p_eq * (1.0 - (-k * (times[i] - t0)).exp());
    assert!(
        ((rates[i] - want) / p_eq).abs() < 1e-5,
        "{} against {want}",
        rates[i]
    );
    // No pitch or yaw appears.
    for axis in ["body_rate_x_rad_s", "body_rate_y_rad_s"] {
        assert!(
            column(&recorder, axis).iter().all(|w| w.abs() < 1e-9),
            "{axis}"
        );
    }
}
