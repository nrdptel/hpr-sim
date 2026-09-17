//! Solid rocket motor model (thrust, propellant mass, CG and inertia over time), `.eng`/`.rse`
//! reading and writing, and catalog types.
//!
//! - [`curve`]: thrust curves, total impulse, NFPA 1125 burn time and average thrust.
//! - [`class`]: impulse classes (`1/8A` to `O` and beyond).
//! - [`motor`]: the solid motor: consumption, mass properties and ambient-pressure thrust, with
//!   [`grains`] (BATES grains) and [`mass`] (axisymmetric mass elements).
//! - [`eng`] and [`rse`]: RASP and RockSim motor files, with [`delay`] strings.
//! - [`catalog`]: the offline catalog and its bundled ThrustCurve.org curves.
//!
//! Physics: `docs/physics/motor.md`. Formats: `docs/format/eng.md`, `docs/format/rse.md`.

mod bundled;
pub mod catalog;
pub mod class;
pub mod curve;
pub mod delay;
pub mod eng;
pub mod error;
pub mod grains;
pub mod mass;
pub mod motor;
pub mod rse;
pub mod text;

pub use catalog::{Catalog, CatalogCurve, CatalogMotor};
pub use class::ImpulseClass;
pub use curve::ThrustCurve;
pub use delay::{Delay, DelayList, DelayWarning};
pub use error::MotorError;
pub use grains::{BatesGrains, GrainShape};
pub use mass::MassElement;
pub use motor::{
    EXHAUST_VELOCITY_RANGE_M_S, MotorState, Nozzle, Propellant, PropellantColumn, SolidMotor,
};
pub use text::{ParseWarning, Parsed, WarningKind};

#[cfg(test)]
mod tests {
    use super::*;

    /// Loft lesson L38: Loft's class letter was off by one at band tops (2.5 N·s gave `B`) and
    /// had no `1/8A`.
    #[test]
    fn impulse_class_upper_bound_inclusive() {
        let class = |ns: f64| ImpulseClass::from_total_impulse(ns).unwrap().label();
        assert_eq!(class(2.5), "A");
        assert_eq!(class(2.500_000_000_000_001), "B");
        assert_eq!(class(5.0), "B");
        assert_eq!(class(10.0), "C");
        assert_eq!(class(160.0), "G");
        assert_eq!(class(160.000_1), "H");
        assert_eq!(class(40960.0), "O");
        assert_eq!(class(0.3125), "1/8A");
        assert_eq!(class(0.1), "1/8A");
        assert_eq!(class(0.3126), "1/4A");
        assert_eq!(class(1.25), "1/2A");
        assert_eq!(class(1.2501), "A");
    }

    /// Loft lesson L40: Loft fixed the motor CG at the casing midpoint with no inertia of its own,
    /// and its impulse-fraction model was uncited.
    #[test]
    fn cg_and_inertia_move_from_loaded_to_burnout() {
        let curve =
            ThrustCurve::new(vec![0.05, 0.2, 1.8, 2.0], vec![900.0, 800.0, 700.0, 0.0]).unwrap();
        // A 38 mm reload: a heavy nozzle and aft closure put the dry centre at 0.10 m, and three
        // grains are centred further forward, at 0.20 m.
        let grains = BatesGrains {
            count: 3,
            density_kg_m3: 1815.0,
            outer_radius_m: 0.0165,
            initial_inner_radius_m: 0.006,
            initial_height_m: 0.09,
            separation_m: 0.005,
            center_m: 0.2,
            inhibited_ends: false,
        };
        let dry = MassElement {
            mass_kg: 0.45,
            cg_m: 0.10,
            axial_inertia_kg_m2: 1.6e-4,
            transverse_inertia_kg_m2: 7.6e-3,
        };
        let motor = SolidMotor::new(curve, Propellant::Grains(grains), dry, None).unwrap();
        let loaded = motor.state(0.0);
        let burnout = motor.state(motor.burnout_time_s());

        // Loaded: the parallel-axis combination of the dry mass and the full grains.
        let m_p = grains.initial_mass_kg();
        assert_eq!(loaded.propellant.mass_kg, m_p);
        let cg = (0.45 * 0.10 + m_p * 0.2) / (0.45 + m_p);
        assert!((loaded.total.cg_m - cg).abs() < 1e-15);
        let full = grains.mass_element(m_p);
        let transverse = dry.transverse_inertia_kg_m2
            + 0.45 * (0.10 - cg).powi(2)
            + full.transverse_inertia_kg_m2
            + m_p * (0.2 - cg).powi(2);
        assert!((loaded.total.transverse_inertia_kg_m2 - transverse).abs() < 1e-15);

        // Burnout: only the dry mass is left, with its own inertia.
        assert_eq!(burnout.propellant.mass_kg, 0.0);
        assert_eq!(burnout.total.mass_kg, 0.45);
        assert!((burnout.total.cg_m - 0.10).abs() < 1e-15);
        assert!((burnout.total.axial_inertia_kg_m2 - 1.6e-4).abs() < 1e-18);
        assert!((burnout.total.transverse_inertia_kg_m2 - 7.6e-3).abs() < 1e-15);

        // In between, the centre moves aft and both inertias fall, monotonically.
        let states: Vec<MotorState> = (0..=200)
            .map(|i| motor.state(f64::from(i) * 0.01))
            .collect();
        for pair in states.windows(2) {
            assert!(pair[1].total.cg_m <= pair[0].total.cg_m + 1e-15);
            assert!(pair[1].total.mass_kg <= pair[0].total.mass_kg);
            assert!(pair[1].total.axial_inertia_kg_m2 <= pair[0].total.axial_inertia_kg_m2 + 1e-18);
        }
        assert!(loaded.total.cg_m - burnout.total.cg_m > 0.03);
        assert!(
            loaded.total.transverse_inertia_kg_m2 > 1.3 * burnout.total.transverse_inertia_kg_m2
        );
    }

    /// Loft lesson L39: Loft took the last sample as the burn time. ThrustCurve's glossary uses
    /// NFPA 1125: from 5% of peak thrust on the way up to 5% of peak on the way down.
    #[test]
    fn burn_time_uses_the_nfpa_1125_definition() {
        // A 1 s ramp to a 200 N peak, a 1 s plateau at 100 N, and a 1 s tail-off to zero; then a
        // long trickle at 2 N (1% of peak) that NFPA 1125 leaves out.
        let curve = ThrustCurve::new(
            vec![0.0, 1.0, 1.001, 2.0, 3.0, 3.001, 6.0, 6.001],
            vec![0.0, 200.0, 100.0, 100.0, 0.0, 2.0, 2.0, 0.0],
        )
        .unwrap();
        assert_eq!(curve.end_time_s(), 6.001);
        // Up through 10 N at 0.05 s; down through 10 N at 2.9 s.
        let (start, end) = curve.burn_window_s();
        assert!((start - 0.05).abs() < 1e-12, "{start}");
        assert!((end - 2.9).abs() < 1e-12, "{end}");
        assert!((curve.burn_time_s() - 2.85).abs() < 1e-12);
        assert!(
            (curve.average_thrust_n() - curve.total_impulse_ns() / 2.85).abs() < 1e-9,
            "average thrust is the total impulse over the NFPA burn time"
        );
    }
}
