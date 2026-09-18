//! The solid motor model: thrust, propellant consumption, and the motor's mass, centre of mass and
//! inertia at any time in the burn.
//!
//! **Consumption by impulse fraction.** With a constant effective exhaust velocity `c = F/ṁ`
//! (NASA SP-8039, glossary p. 95), the propellant burned by time `t` is in proportion to the
//! impulse delivered:
//!
//! ```text
//! c = I_total / m_p0,   ṁ(t) = F(t) / c,   m_p(t) = m_p0 (1 − I(t)/I_total)
//! ```
//!
//! This is RocketPy's `SolidMotor` consumption (`rocketpy/motors/solid_motor.py:401-418`,
//! `motor.py:483-524`), and the mass column of ThrustCurve.org's `.rse` files follows it
//! (`docs/format/rse.md`). It is an approximation: the real `c` drifts with chamber pressure and
//! nozzle erosion (SP-8039 p. 14). All propellant is gone when the thrust curve ends.
//!
//! **Where the propellant is.** A [`Propellant::Column`] keeps its shape and loses density as it
//! burns, so its centre stays put and its inertia scales with its mass (the model of RocketPy's
//! `GenericMotor`). [`Propellant::Grains`] regress as BATES grains ([`crate::grains`]).
//!
//! **The whole motor** combines the propellant with the dry mass (case, closures, nozzle, liner)
//! about the instantaneous centre of mass ([`MassElement::combine`]).
//!
//! **Ambient pressure.** A curve measured at reference pressure `p_ref` gives, at ambient `p_a`,
//! `F = F_ref + (p_ref − p_a) A_e` (from SP-8039 eq. 2, `F = ṁ u_e + (p_e − p_a) A_e`, with the
//! flow unchanged; RocketPy's `Motor.pressure_thrust`, `motor.py:1173-1191`). It holds while the
//! nozzle flows full. See `docs/physics/motor.md`.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::curve::ThrustCurve;
use crate::error::MotorError;
use crate::grains::BatesGrains;
use crate::mass::MassElement;

/// Standard sea-level pressure (US Standard Atmosphere 1976), Pa.
///
/// A thrust curve's reference pressure is the ambient pressure where the motor was static-tested,
/// which motor files and catalogs don't record; this value is only a stand-in when the test site's
/// pressure is unknown.
pub const STANDARD_SEA_LEVEL_PRESSURE_PA: f64 = 101_325.0;

/// A propellant charge of fixed shape whose density falls as it burns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PropellantColumn {
    /// Initial propellant mass, kg.
    pub mass_kg: f64,
    /// Centre of the column along the motor axis, m from the nozzle exit.
    pub center_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Inner (bore) radius, m; zero for a solid column.
    pub inner_radius_m: f64,
    /// Length, m.
    pub length_m: f64,
}

/// How the propellant is laid out and how its shape evolves. Serialized with a `model` tag
/// (`"column"`, `"grains"`).
///
/// Not `Copy`, so that a later model can hold tabulated data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Propellant {
    /// A fixed-shape column: the default when only a motor's envelope is known.
    Column(PropellantColumn),
    /// BATES grains that regress on their bores and ends.
    Grains(BatesGrains),
}

/// A nozzle, for the ambient-pressure correction of thrust.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Nozzle {
    /// Exit radius, m.
    pub exit_radius_m: f64,
    /// Throat radius, m, when known (informational: the thrust curve already carries its effect).
    pub throat_radius_m: Option<f64>,
    /// The ambient pressure the thrust curve was measured at, Pa: the static test site's. Motor
    /// files don't record it; [`STANDARD_SEA_LEVEL_PRESSURE_PA`] is a stand-in when it's unknown.
    pub reference_pressure_pa: f64,
}

impl Nozzle {
    /// The exit area `A_e = π r_e²`, m².
    pub fn exit_area_m2(&self) -> f64 {
        PI * self.exit_radius_m * self.exit_radius_m
    }
}

/// A solid rocket motor.
///
/// Nothing here can tell a hybrid's thrust curve from a solid's, so the checks are at the edges:
/// [`crate::catalog::CatalogMotor::motor`] refuses hybrids and the `.rse` reader warns about them
/// (`.eng` files don't say).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "MotorData", into = "MotorData")]
pub struct SolidMotor {
    curve: ThrustCurve,
    propellant: Propellant,
    dry: MassElement,
    nozzle: Option<Nozzle>,
    /// `m_p0`, kg.
    propellant_mass_kg: f64,
}

/// The serialized form of a [`SolidMotor`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct MotorData {
    curve: ThrustCurve,
    propellant: Propellant,
    dry: MassElement,
    nozzle: Option<Nozzle>,
}

impl TryFrom<MotorData> for SolidMotor {
    type Error = MotorError;

    fn try_from(data: MotorData) -> Result<Self, Self::Error> {
        Self::new(data.curve, data.propellant, data.dry, data.nozzle)
    }
}

impl From<SolidMotor> for MotorData {
    fn from(motor: SolidMotor) -> Self {
        Self {
            curve: motor.curve,
            propellant: motor.propellant,
            dry: motor.dry,
            nozzle: motor.nozzle,
        }
    }
}

/// A motor's thrust and mass properties at one time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotorState {
    /// Time since ignition, s.
    pub time_s: f64,
    /// Thrust from the curve (at its reference pressure), N.
    pub thrust_n: f64,
    /// Propellant consumption rate `ṁ = F/c`, kg/s (non-negative; the motor's mass falls at this
    /// rate).
    pub mass_flow_kg_s: f64,
    /// The propellant left, about its own centre of mass.
    pub propellant: MassElement,
    /// The whole motor (dry mass plus propellant), about the motor's centre of mass.
    pub total: MassElement,
}

/// The effective exhaust velocity `c = I/m_p` a solid motor's curve and propellant mass have to
/// imply, m/s.
///
/// Measured over the 1,708 ThrustCurve.org simulator files with both a parsed impulse and a
/// catalog propellant mass — the mass [`crate::CatalogMotor::motor`] uses, which prefers the
/// metadata over the curve file's header — `c` runs 236 to 3,031 m/s, with a median of 1,867 and
/// 90% of them between 928 and 2,210 (the full table is in
/// `docs/research/exhaust-velocity-guard.md`). The bulk is APCP; the tail below
/// about 900 m/s is black powder, read low because Estes and Quest count the delay grain and the
/// ejection charge as propellant.
///
/// **The bound rejects none of those 1,708.** It is not a filter on propellant: it is there to
/// catch a units slip, which moves `c` by a factor of 1,000 — the worked example in
/// [`SolidMotor::from_envelope`]'s test lands at 1.8 m/s. The headroom is real but not enormous at
/// the low end (the lowest catalog entry is 1.2x above the floor, the highest 1.65x below the
/// ceiling), and a 1/8A whose recorded propellant mass is mostly delay grain could fall through
/// the floor; issue #11 records the fallback, which is to apply the bound only above a couple of
/// grams.
pub const EXHAUST_VELOCITY_RANGE_M_S: std::ops::RangeInclusive<f64> = 200.0..=5000.0;

impl SolidMotor {
    /// Builds a motor from its thrust curve, propellant, dry mass (about its own centre) and
    /// optional nozzle.
    ///
    /// # Errors
    ///
    /// - [`MotorError::Domain`] for a non-positive or non-finite dry mass (the motor at burnout,
    ///   whose centre of mass needs some mass), a negative or non-finite dry inertia, a non-finite
    ///   position, a column with non-positive mass, radius or length, a nozzle with a non-positive
    ///   exit radius, a throat radius outside `(0, exit radius]`, or a negative or non-finite
    ///   reference pressure.
    /// - [`MotorError::Inconsistent`] for a column bore at least as wide as the column, bad grain
    ///   geometry ([`BatesGrains::validate`]), or a curve and propellant mass whose effective
    ///   exhaust velocity `I/m_p` is outside [`EXHAUST_VELOCITY_RANGE_M_S`], which is what a
    ///   units slip looks like.
    pub fn new(
        curve: ThrustCurve,
        propellant: Propellant,
        dry: MassElement,
        nozzle: Option<Nozzle>,
    ) -> Result<Self, MotorError> {
        dry.validate([
            "dry mass (kg)",
            "dry centre of mass (m)",
            "dry axial inertia (kg·m²)",
            "dry transverse inertia (kg·m²)",
        ])?;
        if dry.mass_kg <= 0.0 {
            return Err(MotorError::Domain {
                what: "dry mass (kg), which must be positive",
                value: dry.mass_kg,
            });
        }
        let propellant_mass_kg = match &propellant {
            Propellant::Column(column) => {
                for (value, what) in [
                    (column.mass_kg, "propellant mass (kg)"),
                    (column.outer_radius_m, "propellant outer radius (m)"),
                    (column.length_m, "propellant length (m)"),
                ] {
                    if !(value.is_finite() && value > 0.0) {
                        return Err(MotorError::Domain { what, value });
                    }
                }
                if !column.center_m.is_finite() {
                    return Err(MotorError::Domain {
                        what: "propellant centre (m)",
                        value: column.center_m,
                    });
                }
                if !(column.inner_radius_m.is_finite() && column.inner_radius_m >= 0.0) {
                    return Err(MotorError::Domain {
                        what: "propellant bore radius (m)",
                        value: column.inner_radius_m,
                    });
                }
                if column.inner_radius_m >= column.outer_radius_m {
                    return Err(MotorError::Inconsistent(format!(
                        "propellant bore radius {} m is not inside the outer radius {} m",
                        column.inner_radius_m, column.outer_radius_m
                    )));
                }
                column.mass_kg
            }
            Propellant::Grains(grains) => {
                grains.validate()?;
                grains.initial_mass_kg()
            }
        };
        if let Some(nozzle) = &nozzle {
            if !(nozzle.exit_radius_m.is_finite() && nozzle.exit_radius_m > 0.0) {
                return Err(MotorError::Domain {
                    what: "nozzle exit radius (m)",
                    value: nozzle.exit_radius_m,
                });
            }
            if let Some(throat) = nozzle.throat_radius_m
                && !(throat.is_finite() && throat > 0.0 && throat <= nozzle.exit_radius_m)
            {
                return Err(MotorError::Domain {
                    what: "nozzle throat radius (m), which must be in (0, exit radius]",
                    value: throat,
                });
            }
            if !(nozzle.reference_pressure_pa.is_finite() && nozzle.reference_pressure_pa >= 0.0) {
                return Err(MotorError::Domain {
                    what: "thrust-curve reference pressure (Pa)",
                    value: nozzle.reference_pressure_pa,
                });
            }
        }
        // A units slip is the failure this catches: the 411I175 built from millimetres and grams
        // read as metres and kilograms is accepted by every check above, and flies with an
        // effective exhaust velocity of 1.8 m/s.
        let exhaust_velocity_m_s = curve.total_impulse_ns() / propellant_mass_kg;
        if !EXHAUST_VELOCITY_RANGE_M_S.contains(&exhaust_velocity_m_s) {
            return Err(MotorError::Inconsistent(format!(
                "a total impulse of {} N·s from {propellant_mass_kg} kg of propellant is an \
                 effective exhaust velocity of {exhaust_velocity_m_s} m/s, outside the {} to {} \
                 m/s a solid motor can have; check the units of the masses and the curve",
                curve.total_impulse_ns(),
                EXHAUST_VELOCITY_RANGE_M_S.start(),
                EXHAUST_VELOCITY_RANGE_M_S.end()
            )));
        }
        Ok(Self {
            curve,
            propellant,
            dry,
            nozzle,
            propellant_mass_kg,
        })
    }

    /// A motor known only by its envelope, as in a motor file or catalog: diameter `D`, length `L`,
    /// propellant mass and loaded mass.
    ///
    /// The assumptions are crude, and a motor with measured data should be built with
    /// [`SolidMotor::new`] instead:
    ///
    /// - the propellant is a solid column of radius `D/2` filling the length, centred at `L/2`;
    /// - the dry mass (loaded minus propellant) is a thin tube of radius `D/2` and length `L`,
    ///   centred at `L/2`;
    /// - no nozzle is known, so thrust gets no ambient-pressure correction.
    ///
    /// # Errors
    ///
    /// [`MotorError::Domain`] for a non-positive or non-finite dimension or propellant mass, and
    /// [`MotorError::Inconsistent`] when the propellant mass is not below the loaded mass (the
    /// motor would weigh nothing at burnout), or when the curve and the propellant mass imply an
    /// effective exhaust velocity outside [`EXHAUST_VELOCITY_RANGE_M_S`] — which is what this
    /// constructor's arguments look like in millimetres and grams.
    pub fn from_envelope(
        curve: ThrustCurve,
        diameter_m: f64,
        length_m: f64,
        propellant_mass_kg: f64,
        loaded_mass_kg: f64,
    ) -> Result<Self, MotorError> {
        for (value, what) in [
            (propellant_mass_kg, "propellant mass (kg)"),
            (loaded_mass_kg, "loaded motor mass (kg)"),
            (diameter_m, "motor diameter (m)"),
            (length_m, "motor length (m)"),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(MotorError::Domain { what, value });
            }
        }
        if propellant_mass_kg >= loaded_mass_kg {
            return Err(MotorError::Inconsistent(format!(
                "propellant mass {propellant_mass_kg} kg is not below the loaded mass \
                 {loaded_mass_kg} kg"
            )));
        }
        let radius = 0.5 * diameter_m;
        let center = 0.5 * length_m;
        let column = PropellantColumn {
            mass_kg: propellant_mass_kg,
            center_m: center,
            outer_radius_m: radius,
            inner_radius_m: 0.0,
            length_m,
        };
        let dry = MassElement::thin_tube(
            loaded_mass_kg - propellant_mass_kg,
            center,
            radius,
            length_m,
        );
        Self::new(curve, Propellant::Column(column), dry, None)
    }

    /// The same motor with more dry mass, such as a reload's case and closures (when the loaded
    /// mass excludes them) or a motor retainer, combined about the new dry centre of mass.
    ///
    /// # Errors
    ///
    /// [`MotorError::Domain`] for a negative or non-finite mass or inertia, or a non-finite
    /// position.
    pub fn with_added_dry_mass(mut self, hardware: MassElement) -> Result<Self, MotorError> {
        hardware.validate([
            "added dry mass (kg)",
            "added dry mass centre (m)",
            "added dry axial inertia (kg·m²)",
            "added dry transverse inertia (kg·m²)",
        ])?;
        self.dry = MassElement::combine([&self.dry, &hardware]);
        Ok(self)
    }

    /// The thrust curve.
    pub fn curve(&self) -> &ThrustCurve {
        &self.curve
    }

    /// The propellant model.
    pub fn propellant(&self) -> &Propellant {
        &self.propellant
    }

    /// The dry mass, about its own centre of mass.
    pub fn dry(&self) -> MassElement {
        self.dry
    }

    /// The nozzle, when known.
    pub fn nozzle(&self) -> Option<Nozzle> {
        self.nozzle
    }

    /// The propellant mass at ignition `m_p0`, kg.
    pub fn propellant_initial_mass_kg(&self) -> f64 {
        self.propellant_mass_kg
    }

    /// The effective exhaust velocity `c = I_total / m_p0`, m/s.
    pub fn exhaust_velocity_m_s(&self) -> f64 {
        self.curve.total_impulse_ns() / self.propellant_mass_kg
    }

    /// The time the thrust curve ends and the propellant is gone, s.
    pub fn burnout_time_s(&self) -> f64 {
        self.curve.end_time_s()
    }

    /// The propellant left at time `t`, `m_p0 (1 − I(t)/I_total)`, kg. A NaN time gives NaN.
    pub fn propellant_mass_kg(&self, t: f64) -> f64 {
        if t.is_nan() {
            return f64::NAN;
        }
        let fraction = self.curve.impulse_ns(t) / self.curve.total_impulse_ns();
        (self.propellant_mass_kg * (1.0 - fraction)).max(0.0)
    }

    /// The thrust and mass properties at `t` seconds after ignition. Before ignition the motor is
    /// loaded and after burnout it is empty.
    pub fn state(&self, t: f64) -> MotorState {
        let thrust_n = self.curve.thrust_n(t);
        let mass = self.propellant_mass_kg(t);
        let propellant = match &self.propellant {
            Propellant::Column(column) => MassElement::hollow_cylinder(
                mass,
                column.center_m,
                column.outer_radius_m,
                column.inner_radius_m,
                column.length_m,
            ),
            Propellant::Grains(grains) => grains.mass_element(mass),
        };
        MotorState {
            time_s: t,
            thrust_n,
            mass_flow_kg_s: thrust_n / self.exhaust_velocity_m_s(),
            propellant,
            total: MassElement::combine([&self.dry, &propellant]),
        }
    }

    /// The thrust at `t` with ambient pressure `ambient_pa`, N: `F + (p_ref − p_a) A_e`, with the
    /// nozzle's [`Nozzle::reference_pressure_pa`], strictly inside the burn (`0 < t < t_end`, as
    /// RocketPy's flight applies it) where the curve's thrust is positive, never below zero; the
    /// curve's thrust elsewhere, and without a nozzle. A NaN time gives NaN, and so does a NaN
    /// ambient pressure where the term applies.
    ///
    /// The term is the full-flow value throughout, so the thrust steps by it just after ignition
    /// and again at `t_end` (both events for an integrator), and in the ignition transient and the
    /// tail-off, where a real nozzle's exit pressure is far from its full-flow value, it misstates
    /// the thrust (`docs/physics/motor.md`).
    pub fn thrust_at_pressure_n(&self, t: f64, ambient_pa: f64) -> f64 {
        let thrust = self.curve.thrust_n(t);
        match self.nozzle {
            Some(nozzle)
                if t > 0.0 && t < self.curve.end_time_s() && (thrust > 0.0 || thrust.is_nan()) =>
            {
                let corrected =
                    thrust + (nozzle.reference_pressure_pa - ambient_pa) * nozzle.exit_area_m2();
                if corrected.is_nan() {
                    corrected
                } else {
                    corrected.max(0.0)
                }
            }
            _ => thrust,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> ThrustCurve {
        ThrustCurve::new(vec![0.0, 0.1, 1.0, 1.2], vec![0.0, 500.0, 400.0, 0.0]).unwrap()
    }

    #[test]
    fn column_mass_follows_the_impulse_fraction() {
        let motor = SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.3, 0.6).unwrap();
        let total = motor.curve().total_impulse_ns();
        assert!((total - (25.0 + 405.0 + 40.0)).abs() < 1e-12);
        assert!((motor.exhaust_velocity_m_s() - total / 0.3).abs() < 1e-9);
        for t in [-1.0, 0.0, 0.05, 0.5, 1.1, 1.2, 5.0] {
            let state = motor.state(t);
            let expected = 0.3 * (1.0 - motor.curve().impulse_ns(t) / total);
            assert!((state.propellant.mass_kg - expected).abs() < 1e-15, "{t}");
            assert!((state.total.mass_kg - (0.3 + expected)).abs() < 1e-15);
            assert!(
                (state.mass_flow_kg_s * motor.exhaust_velocity_m_s() - state.thrust_n).abs() < 1e-9
            );
        }
        assert_eq!(motor.state(0.0).propellant.mass_kg, 0.3);
        assert_eq!(motor.state(1.2).propellant.mass_kg, 0.0);
        // The ṁ integral is the propellant burned: trapezoids on a fine grid.
        let n = 12_000;
        let dt = 1.2 / f64::from(n);
        let burned: f64 = (0..n)
            .map(|i| {
                let (a, b) = (f64::from(i) * dt, f64::from(i + 1) * dt);
                0.5 * (motor.state(a).mass_flow_kg_s + motor.state(b).mass_flow_kg_s) * dt
            })
            .sum();
        assert!((burned - 0.3).abs() < 1e-6, "{burned}");
    }

    #[test]
    fn a_units_slip_is_refused_by_its_exhaust_velocity() {
        // The worked example from the issue: the 411I175's envelope in millimetres and grams,
        // read as metres and kilograms. Every dimension is positive and finite, the propellant is
        // below the loaded mass, and the motor is nonsense: 437.5 kg of motor, and 411 N·s from
        // 228.9 kg of propellant is an exhaust velocity of 1.8 m/s.
        let i175 = ThrustCurve::new(vec![0.0, 0.1, 2.3, 2.4], vec![0.0, 220.0, 150.0, 0.0])
            .expect("a plausible I-class curve");
        // 425.5 N·s: an I by the NFPA classes, which is what the designation says.
        let impulse = i175.total_impulse_ns();
        assert!((impulse - 425.5).abs() < 0.1, "{impulse}");
        let slipped = SolidMotor::from_envelope(i175.clone(), 38.0, 245.0, 228.9, 437.5)
            .expect_err("millimetres and grams read as metres and kilograms");
        assert!(
            matches!(&slipped, MotorError::Inconsistent(message)
                if message.contains("exhaust velocity") && message.contains("check the units")),
            "{slipped}"
        );
        // The same motor in SI is accepted: 38 mm by 245 mm, 228.9 g of propellant, 437.5 g
        // loaded, c = 1,859 m/s, which is within 0.5% of the median of the 1,710 surveyed files.
        let motor = SolidMotor::from_envelope(i175.clone(), 0.038, 0.245, 0.2289, 0.4375)
            .expect("the same motor in metres and kilograms");
        let c = motor.curve().total_impulse_ns() / motor.propellant_mass_kg(0.0);
        assert!((1700.0..1900.0).contains(&c), "{c}");

        // The bound is on the physics, not on which constructor was used.
        let dry = MassElement::thin_tube(0.2086, 0.1225, 0.019, 0.245);
        let column = PropellantColumn {
            mass_kg: 2.289,
            center_m: 0.1225,
            outer_radius_m: 0.017,
            inner_radius_m: 0.005,
            length_m: 0.2,
        };
        let ten_times = SolidMotor::new(i175, Propellant::Column(column), dry, None)
            .expect_err("ten times the propellant for the same impulse");
        assert!(
            // 425.5 N·s over 2.289 kg is 185.9 m/s, just under the floor.
            matches!(&ten_times, MotorError::Inconsistent(message) if message.contains("185.8")),
            "{ten_times}"
        );

        assert!(!EXHAUST_VELOCITY_RANGE_M_S.contains(&1.8));
    }

    #[test]
    fn every_bundled_motor_is_inside_the_range() {
        // The check's other half: a bound real motors fall outside is a bug, not a check. This
        // builds all 32 rather than asserting a remembered pair of numbers — the doc's figures
        // came out of the catalog's stored impulse once, which is not what the check divides.
        let catalog = crate::Catalog::bundled().expect("the bundled catalog parses");
        let mut lowest = f64::INFINITY;
        let mut highest: f64 = 0.0;
        let (mut low_name, mut high_name) = (String::new(), String::new());
        for entry in &catalog.motors {
            let motor = entry
                .bundled_motor()
                .unwrap_or_else(|error| panic!("{}: {error}", entry.designation));
            let c = motor.curve().total_impulse_ns() / motor.propellant_mass_kg(0.0);
            assert!(
                EXHAUST_VELOCITY_RANGE_M_S.contains(&c),
                "{}: c = {c} m/s",
                entry.designation
            );
            if c < lowest {
                lowest = c;
                low_name = entry.common_name.clone();
            }
            if c > highest {
                highest = c;
                high_name = entry.common_name.clone();
            }
        }
        // `docs/physics/motor.md` quotes these; they are measured here so the doc cannot drift.
        assert!((lowest - 689.78).abs() < 0.01, "{low_name} at {lowest}");
        assert!((highest - 2651.64).abs() < 0.01, "{high_name} at {highest}");
    }

    #[test]
    fn a_curve_ending_above_zero_is_empty_and_silent_at_its_end() {
        let cut = ThrustCurve::new(vec![0.0, 1.0], vec![20.0, 20.0]).unwrap();
        // 20 N·s from 10 g is c = 2,000 m/s, which is what APCP does; the 200 g this used to burn
        // was 100 m/s, and `EXHAUST_VELOCITY_RANGE_M_S` now refuses it.
        let motor = SolidMotor::from_envelope(cut, 0.038, 0.25, 0.01, 0.5).unwrap();
        let before = motor.state(1.0 - 1e-9);
        assert_eq!(before.thrust_n, 20.0);
        assert!(before.mass_flow_kg_s > 0.0 && before.propellant.mass_kg > 0.0);
        let end = motor.state(motor.burnout_time_s());
        assert_eq!(
            (end.thrust_n, end.mass_flow_kg_s, end.propellant.mass_kg),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(end.total.mass_kg, 0.49);
    }

    #[test]
    fn envelope_defaults_are_centred_tubes_and_columns() {
        let motor = SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.3, 0.6).unwrap();
        let loaded = motor.state(0.0);
        assert_eq!(loaded.total.cg_m, 0.125);
        let r2 = 0.019f64 * 0.019;
        let dry_axial = 0.3 * r2;
        let prop_axial = 0.5 * 0.3 * r2;
        assert!((loaded.total.axial_inertia_kg_m2 - (dry_axial + prop_axial)).abs() < 1e-15);
        let dry_t = 0.3 * (r2 / 2.0 + 0.25f64.powi(2) / 12.0);
        let prop_t = 0.3 * (r2 / 4.0 + 0.25f64.powi(2) / 12.0);
        assert!((loaded.total.transverse_inertia_kg_m2 - (dry_t + prop_t)).abs() < 1e-15);
        assert!(motor.nozzle().is_none());
        assert_eq!(
            motor.thrust_at_pressure_n(0.5, 0.0),
            motor.curve().thrust_n(0.5)
        );
    }

    #[test]
    fn pressure_correction_uses_the_exit_area() {
        let dry = MassElement::thin_tube(0.3, 0.125, 0.019, 0.25);
        let column = PropellantColumn {
            mass_kg: 0.3,
            center_m: 0.15,
            outer_radius_m: 0.017,
            inner_radius_m: 0.005,
            length_m: 0.2,
        };
        let nozzle = Nozzle {
            exit_radius_m: 0.01,
            throat_radius_m: Some(0.004),
            reference_pressure_pa: STANDARD_SEA_LEVEL_PRESSURE_PA,
        };
        let motor =
            SolidMotor::new(curve(), Propellant::Column(column), dry, Some(nozzle)).unwrap();
        let area = PI * 1e-4;
        let f = motor.curve().thrust_n(0.5);
        let vacuum = motor.thrust_at_pressure_n(0.5, 0.0);
        assert!((vacuum - (f + 101_325.0 * area)).abs() < 1e-9);
        assert_eq!(motor.thrust_at_pressure_n(0.5, 101_325.0), f);
        assert_eq!(motor.thrust_at_pressure_n(2.0, 0.0), 0.0);
        assert_eq!(motor.thrust_at_pressure_n(0.001, 1e9), 0.0);
        // The curve's own reference pressure is used: tested at altitude, it gains less in vacuum.
        let high = SolidMotor::new(
            curve(),
            Propellant::Column(column),
            dry,
            Some(Nozzle {
                reference_pressure_pa: 80_000.0,
                ..nozzle
            }),
        )
        .unwrap();
        assert!((high.thrust_at_pressure_n(0.5, 0.0) - (f + 80_000.0 * area)).abs() < 1e-9);
        assert!((high.thrust_at_pressure_n(0.5, 101_325.0) - (f - 21_325.0 * area)).abs() < 1e-9);
        // Only strictly inside the burn: nothing is added at ignition or at the last sample.
        assert_eq!(motor.thrust_at_pressure_n(0.0, 0.0), 0.0);
        assert_eq!(motor.thrust_at_pressure_n(1.2, 0.0), 0.0);
        assert!(motor.thrust_at_pressure_n(0.5, f64::NAN).is_nan());
        assert!(motor.thrust_at_pressure_n(f64::NAN, 0.0).is_nan());
        // A NaN time gives NaN mass properties, for columns and grains alike.
        let state = motor.state(f64::NAN);
        assert!(state.total.mass_kg.is_nan() && state.propellant.mass_kg.is_nan());
        assert!(state.total.cg_m.is_nan());
        // No flow, no pressure term: a zero-thrust gap inside the burn gets none. The thrusts are
        // 400 N rather than the 10 N this used to use, so that 0.3 kg of propellant implies
        // c = 1,990 m/s instead of 50 m/s, which `EXHAUST_VELOCITY_RANGE_M_S` refuses.
        let gap = ThrustCurve::new(
            vec![0.0, 1.0, 1.0, 2.0, 2.0, 3.0],
            vec![400.0, 400.0, 0.0, 0.0, 400.0, 0.0],
        )
        .unwrap();
        let gapped = SolidMotor::new(gap, Propellant::Column(column), dry, Some(nozzle)).unwrap();
        assert_eq!(gapped.thrust_at_pressure_n(1.5, 0.0), 0.0);
        assert!(gapped.thrust_at_pressure_n(2.5, 0.0) > gapped.curve().thrust_n(2.5));
    }

    #[test]
    fn rejects_bad_inputs() {
        let dry = MassElement::thin_tube(0.3, 0.125, 0.019, 0.25);
        let column = PropellantColumn {
            mass_kg: 0.3,
            center_m: 0.15,
            outer_radius_m: 0.017,
            inner_radius_m: 0.005,
            length_m: 0.2,
        };
        let build = |column: PropellantColumn, dry: MassElement, nozzle: Option<Nozzle>| {
            SolidMotor::new(curve(), Propellant::Column(column), dry, nozzle)
        };
        assert!(build(column, dry, None).is_ok());
        assert!(
            build(
                PropellantColumn {
                    mass_kg: 0.0,
                    ..column
                },
                dry,
                None
            )
            .is_err()
        );
        assert!(
            build(
                PropellantColumn {
                    inner_radius_m: 0.017,
                    ..column
                },
                dry,
                None
            )
            .is_err()
        );
        assert!(
            build(
                PropellantColumn {
                    center_m: f64::NAN,
                    ..column
                },
                dry,
                None
            )
            .is_err()
        );
        assert!(
            build(
                column,
                MassElement {
                    mass_kg: -0.1,
                    ..dry
                },
                None
            )
            .is_err()
        );
        // No dry mass: nothing would be left at burnout to have a centre of mass.
        assert!(
            build(
                column,
                MassElement {
                    mass_kg: 0.0,
                    ..dry
                },
                None
            )
            .is_err()
        );
        let nozzle = |exit, throat, reference| {
            Some(Nozzle {
                exit_radius_m: exit,
                throat_radius_m: throat,
                reference_pressure_pa: reference,
            })
        };
        assert!(build(column, dry, nozzle(0.01, Some(0.004), 101_325.0)).is_ok());
        assert!(build(column, dry, nozzle(0.0, None, 101_325.0)).is_err());
        assert!(build(column, dry, nozzle(0.01, Some(0.02), 101_325.0)).is_err());
        assert!(build(column, dry, nozzle(0.01, None, -1.0)).is_err());
        assert!(build(column, dry, nozzle(0.01, None, f64::NAN)).is_err());
        assert!(SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.7, 0.6).is_err());
        assert!(SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.6, 0.6).is_err());
        assert!(matches!(
            SolidMotor::from_envelope(curve(), 0.038, 0.25, f64::NAN, 0.6),
            Err(MotorError::Domain {
                what: "propellant mass (kg)",
                ..
            })
        ));
        assert!(SolidMotor::from_envelope(curve(), 0.0, 0.25, 0.3, 0.6).is_err());
    }

    #[test]
    fn added_hardware_joins_the_dry_mass() {
        let motor = SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.3, 0.6).unwrap();
        let retainer = MassElement::thin_tube(0.05, -0.01, 0.022, 0.02);
        let with = motor.clone().with_added_dry_mass(retainer).unwrap();
        let burnout = with.state(with.burnout_time_s()).total;
        let expected = MassElement::combine([&motor.dry(), &retainer]);
        assert_eq!(
            burnout,
            MassElement::combine([
                &expected,
                &MassElement {
                    cg_m: 0.125,
                    ..MassElement::ZERO
                }
            ])
        );
        assert!((burnout.mass_kg - 0.35).abs() < 1e-15);
        assert!(burnout.cg_m < 0.125);
        assert!(
            motor
                .with_added_dry_mass(MassElement {
                    mass_kg: f64::NAN,
                    ..retainer
                })
                .is_err()
        );
    }

    #[derive(Debug, serde::Deserialize)]
    struct Oracle {
        oracle: String,
        cases: Vec<OracleCase>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleCase {
        name: String,
        inputs: OracleInputs,
        scalars: OracleScalars,
        series: OracleSeries,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleInputs {
        thrust_file: String,
        thrust_file_sha256: String,
        dry_mass: f64,
        dry_inertia: [f64; 3],
        center_of_dry_mass_position: f64,
        nozzle_position: f64,
        nozzle_radius: f64,
        throat_radius: f64,
        grain_number: u32,
        grain_density: f64,
        grain_outer_radius: f64,
        grain_initial_inner_radius: f64,
        grain_initial_height: f64,
        grain_separation: f64,
        grains_center_of_mass_position: f64,
        coordinate_system_orientation: String,
        only_radial_burn: bool,
        interpolation_method: String,
        burn_time: Option<f64>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleScalars {
        total_impulse_ns: f64,
        burn_out_time_s: f64,
        exhaust_velocity_mps: f64,
        propellant_initial_mass_kg: f64,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleSeries {
        time_s: Vec<f64>,
        thrust: Vec<f64>,
        mass_flow_rate: Vec<f64>,
        propellant_mass: Vec<f64>,
        total_mass: Vec<f64>,
        center_of_propellant_mass: Vec<f64>,
        center_of_mass: Vec<f64>,
        grain_inner_radius: Vec<f64>,
        grain_height: Vec<f64>,
        #[serde(rename = "propellant_I_11")]
        propellant_i_11: Vec<f64>,
        #[serde(rename = "propellant_I_33")]
        propellant_i_33: Vec<f64>,
        #[serde(rename = "I_11")]
        i_11: Vec<f64>,
        #[serde(rename = "I_33")]
        i_33: Vec<f64>,
    }

    /// M1.3's done-when: mass and inertia evolution matches RocketPy's `SolidMotor` for three
    /// motors within 1%. The fixture comes from `validation/oracles/rocketpy/solid_motor.py`,
    /// which runs RocketPy 1.13.0 on three bundled curves with BATES grain loads.
    ///
    /// Each quantity's error is relative to RocketPy's value, except where the value itself goes
    /// to zero (thrust, flow, and the propellant's mass and inertias): there it is relative to the
    /// peak or ignition value, and a grain height that burns away is relative to its initial
    /// height. Positions are compared in hpr's motor frame (metres forward of the nozzle exit),
    /// relative to the motor's length.
    ///
    /// Measured: every error is below 1e-4. At RocketPy's own ODE knots hpr's exact web matches
    /// RocketPy's bore and height to about 1e-9, so the residual is RocketPy's linear interpolation
    /// between knots. The test holds 0.1%, ten times tighter than the done-when, so a regression
    /// shows long before 1%. Because inputs and outputs share the frame mapping, loaded and
    /// burned-out centres are also checked against values worked by hand from the cases' layouts.
    #[test]
    fn matches_rocketpy_solid_motor_for_three_bundled_motors() {
        let oracle: Oracle = serde_json::from_str(include_str!(
            "../../../validation/fixtures/motor/rocketpy-solid-motor.json"
        ))
        .unwrap();
        assert_eq!(oracle.oracle, "rocketpy 1.13.0");
        assert_eq!(oracle.cases.len(), 3);
        let catalog = crate::catalog::Catalog::bundled().unwrap();
        for case in &oracle.cases {
            let inputs = &case.inputs;
            let (entry, curve) = catalog
                .motors
                .iter()
                .find_map(|m| {
                    m.curves
                        .iter()
                        .find(|c| c.file == inputs.thrust_file)
                        .map(|c| (m, c))
                })
                .unwrap();
            // The oracle read the same bytes the crate bundles.
            assert_eq!(curve.sha256, inputs.thrust_file_sha256, "{}", case.name);
            let text = crate::catalog::bundled_curve_text(&curve.file).unwrap();
            let thrust = entry.thrust_curve(curve, text).unwrap();

            // RocketPy positions are along its own axis from its own origin; hpr's run forward
            // from the nozzle exit.
            let sign = match inputs.coordinate_system_orientation.as_str() {
                "nozzle_to_combustion_chamber" => 1.0,
                "combustion_chamber_to_nozzle" => -1.0,
                other => panic!("unknown orientation {other}"),
            };
            let to_hpr = |z: f64| sign * (z - inputs.nozzle_position);
            let grains = BatesGrains {
                count: inputs.grain_number,
                density_kg_m3: inputs.grain_density,
                outer_radius_m: inputs.grain_outer_radius,
                initial_inner_radius_m: inputs.grain_initial_inner_radius,
                initial_height_m: inputs.grain_initial_height,
                separation_m: inputs.grain_separation,
                center_m: to_hpr(inputs.grains_center_of_mass_position),
                inhibited_ends: inputs.only_radial_burn,
            };
            let dry = MassElement {
                mass_kg: inputs.dry_mass,
                cg_m: to_hpr(inputs.center_of_dry_mass_position),
                axial_inertia_kg_m2: inputs.dry_inertia[2],
                transverse_inertia_kg_m2: inputs.dry_inertia[0],
            };
            let nozzle = Nozzle {
                exit_radius_m: inputs.nozzle_radius,
                throat_radius_m: Some(inputs.throat_radius),
                reference_pressure_pa: STANDARD_SEA_LEVEL_PRESSURE_PA,
            };
            let motor =
                SolidMotor::new(thrust, Propellant::Grains(grains), dry, Some(nozzle)).unwrap();
            // The fixture must describe the model compared: linear thrust, the whole curve.
            assert_eq!(inputs.interpolation_method, "linear");
            assert_eq!(inputs.burn_time, None);
            // Centres in hpr's frame, worked by hand from each case's layout (metres forward of
            // the nozzle exit): the dry mass and grains at loaded, the dry mass at burnout.
            let hand = match case.name.as_str() {
                // Nozzle exit at 0: dry 0.2086 kg at 0.11, grains 0.2289391 kg at 0.13.
                "cti-411i175-38mm-radial-burnout" => (0.2086, 0.11, 0.13),
                // Axis from the forward closure to the nozzle exit at 0.404: dry at 0.25 and grains
                // at 0.19 from the closure are 0.154 and 0.214 forward of the exit.
                "cti-1633k940-54mm-axial-burnout-chamber-to-nozzle" => (0.5985, 0.154, 0.214),
                // Nozzle exit at 0: dry 1.731 kg at 0.50, grains at 0.56.
                "loki-m1378lr-54mm-inhibited-ends" => (1.731, 0.50, 0.56),
                other => panic!("no hand values for {other}"),
            };
            let (dry_kg, dry_z, grains_z) = hand;
            let m_p = case.scalars.propellant_initial_mass_kg;
            let loaded_cg = (dry_kg * dry_z + m_p * grains_z) / (dry_kg + m_p);
            assert!(
                (motor.state(0.0).total.cg_m - loaded_cg).abs() < 1e-12,
                "{}",
                case.name
            );
            assert!(
                (motor.state(motor.burnout_time_s()).total.cg_m - dry_z).abs() < 1e-12,
                "{}",
                case.name
            );

            let scalars = &case.scalars;
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs();
            assert!(
                close(motor.curve().total_impulse_ns(), scalars.total_impulse_ns),
                "{}",
                case.name
            );
            assert!(
                close(motor.burnout_time_s(), scalars.burn_out_time_s),
                "{}",
                case.name
            );
            assert!(close(
                motor.propellant_initial_mass_kg(),
                scalars.propellant_initial_mass_kg
            ));
            assert!(close(
                motor.exhaust_velocity_m_s(),
                scalars.exhaust_velocity_mps
            ));

            let series = &case.series;
            let length = entry.length_mm * 1e-3;
            let mut worst: Vec<(&str, f64)> = Vec::new();
            let mut record = |what: &'static str, error: f64| match worst
                .iter_mut()
                .find(|(name, _)| *name == what)
            {
                Some((_, max)) => *max = max.max(error),
                None => worst.push((what, error)),
            };
            let loaded = motor.state(0.0);
            let peak_flow = motor.curve().peak_thrust_n() / motor.exhaust_velocity_m_s();
            for (i, &t) in series.time_s.iter().enumerate() {
                let state = motor.state(t);
                let shape = grains.shape(state.propellant.mass_kg);
                let relative = |ours: f64, theirs: f64| (ours - theirs).abs() / theirs.abs();
                let scaled = |ours: f64, theirs: f64, scale: f64| (ours - theirs).abs() / scale;
                record(
                    "thrust",
                    scaled(
                        state.thrust_n,
                        series.thrust[i],
                        motor.curve().peak_thrust_n(),
                    ),
                );
                record(
                    "mass flow",
                    scaled(state.mass_flow_kg_s, -series.mass_flow_rate[i], peak_flow),
                );
                record(
                    "propellant mass",
                    scaled(
                        state.propellant.mass_kg,
                        series.propellant_mass[i],
                        scalars.propellant_initial_mass_kg,
                    ),
                );
                record(
                    "total mass",
                    relative(state.total.mass_kg, series.total_mass[i]),
                );
                record(
                    "propellant centre",
                    scaled(
                        state.propellant.cg_m,
                        to_hpr(series.center_of_propellant_mass[i]),
                        length,
                    ),
                );
                record(
                    "centre of mass",
                    scaled(state.total.cg_m, to_hpr(series.center_of_mass[i]), length),
                );
                record(
                    "grain bore radius",
                    relative(shape.inner_radius_m, series.grain_inner_radius[i]),
                );
                record(
                    "grain height",
                    scaled(
                        shape.height_m,
                        series.grain_height[i],
                        inputs.grain_initial_height,
                    ),
                );
                record(
                    "propellant I_11",
                    scaled(
                        state.propellant.transverse_inertia_kg_m2,
                        series.propellant_i_11[i],
                        loaded.propellant.transverse_inertia_kg_m2,
                    ),
                );
                record(
                    "propellant I_33",
                    scaled(
                        state.propellant.axial_inertia_kg_m2,
                        series.propellant_i_33[i],
                        loaded.propellant.axial_inertia_kg_m2,
                    ),
                );
                record(
                    "motor I_11",
                    relative(state.total.transverse_inertia_kg_m2, series.i_11[i]),
                );
                record(
                    "motor I_33",
                    relative(state.total.axial_inertia_kg_m2, series.i_33[i]),
                );
            }
            eprintln!("{}: largest errors against RocketPy", case.name);
            for (what, error) in &worst {
                eprintln!("  {what:<18} {:.3e}", error);
                assert!(
                    *error <= 1e-3,
                    "{}: {what} differs from RocketPy by {:.4}%",
                    case.name,
                    100.0 * error
                );
            }
        }
    }

    #[test]
    fn serde_round_trips_and_rechecks() {
        let motor = SolidMotor::from_envelope(curve(), 0.038, 0.25, 0.3, 0.6).unwrap();
        let json = serde_json::to_string(&motor).unwrap();
        assert_eq!(serde_json::from_str::<SolidMotor>(&json).unwrap(), motor);
        let broken = json.replace(
            "\"mass_kg\":0.3,\"center_m\"",
            "\"mass_kg\":-0.3,\"center_m\"",
        );
        assert_ne!(broken, json);
        assert!(serde_json::from_str::<SolidMotor>(&broken).is_err());
    }
}
