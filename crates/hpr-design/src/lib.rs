//! The canonical rocket design model: component tree, shapes, materials, mass properties, stages
//! and configurations, and design checks.
//!
//! **Guide:** [The design tree][guide-design], [Shapes][guide-shapes] and [Mass
//! properties][guide-mass]: the models, their sources, how well they are validated and what they
//! leave out.
//!
//! [guide-design]: https://nrdptel.github.io/hpr-sim/physics/design.html
//! [guide-shapes]: https://nrdptel.github.io/hpr-sim/physics/shapes.html
//! [guide-mass]: https://nrdptel.github.io/hpr-sim/physics/mass.html
//!
//! - [`mass`]: mass, centre of mass and full inertia tensor, and how bodies combine.
//! - [`shapes`]: nose cone and transition profiles.
//! - [`solids`]: solids of revolution, filled or with a wall.
//! - [`finish`]: surface finishes and their roughness heights.
//! - [`fins`]: fin sets (trapezoidal, elliptical, freeform) and tube fins.
//! - [`parts`]: every other component, from body tubes to shock cords.
//! - [`material`]: materials and their densities, and [`materials`]: built-in values with sources.
//! - [`tree`]: the design tree of stages and components, placement, automatic radii, overrides and
//!   the reference diameter.
//! - [`config`]: motor mounts, configurations, and the rocket's mass properties through the burn.
//! - [`checks`]: structural checks with typed findings.

pub mod checks;
pub mod config;
pub mod error;
pub mod finish;
pub mod fins;
pub mod mass;
pub mod material;
pub mod materials;
pub mod parts;
pub mod shapes;
pub mod solids;
pub mod tree;

#[cfg(test)]
mod testing;

pub use checks::{Finding, Severity};
pub use config::{Assembly, Configuration, Ignition, MotorMount, MountedMotor, PlacedMotor};
pub use error::DesignError;
pub use finish::Finish;
pub use fins::{FinCrossSection, FinPlanform, FinSet, FinTab, TubeFinSet};
pub use mass::MassProperties;
pub use material::{Density, Material};
pub use parts::{
    BodyTube, CenteringRing, InnerTube, LaunchLug, MassComponent, NoseCone, Packing, Parachute,
    RailButton, ShockCord, Shoulder, Streamer, Transition,
};
pub use shapes::{NoseShape, Profile};
pub use solids::{RevolvedGeometry, Wall, revolve};
pub use tree::{
    AutoDimension, Component, InertiaOverride, Layout, Overrides, Part, PlacedComponent,
    PlacedStage, Position, ReferenceDiameter, Rocket, Stage, UnresolvableRadius,
};

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use hpr_core::{DMat3, DVec3};

    use super::*;

    /// A rocket-like composite: a filled conical nose, a body tube, four rectangular fins and an
    /// off-axis mass, placed along the body and combined. The expected tensor is written out term
    /// by term from each part's textbook moments and the parallel-axis theorem, including the
    /// products of inertia the off-axis mass creates.
    #[test]
    fn composite_rocket_inertia_matches_hand_calculation() {
        let r = 0.05;
        let nose = NoseCone {
            shape: NoseShape::Conical {},
            length_m: 0.3,
            base_radius_m: r,
            wall: Wall::Filled {},
            shoulder: None,
            material: Material::bulk("PLA", 1240.0),
        };
        let tube = BodyTube {
            length_m: 0.8,
            outer_radius_m: r,
            thickness_m: 0.002,
            material: Material::bulk("cardboard", 790.0),
        };
        let fins = FinSet {
            count: 4,
            planform: FinPlanform::Trapezoidal {
                root_chord_m: 0.15,
                tip_chord_m: 0.15,
                span_m: 0.1,
                sweep_m: 0.0,
            },
            thickness_m: 0.004,
            cross_section: FinCrossSection::Square,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: Material::bulk("plywood", 630.0),
        };
        let angle = 30f64.to_radians();
        let payload = MassComponent {
            mass_kg: 0.3,
            packing: Packing {
                length_m: 0.1,
                radius_m: 0.02,
                radial_offset_m: 0.03,
                angle_rad: angle,
            },
        };
        let placed = [
            nose.mass_properties().unwrap(),
            tube.mass_properties()
                .unwrap()
                .translated(DVec3::new(0.0, 0.0, -0.3)),
            fins.mass_properties(r)
                .unwrap()
                .translated(DVec3::new(0.0, 0.0, -0.95)),
            payload
                .mass_properties()
                .unwrap()
                .translated(DVec3::new(0.0, 0.0, -0.5)),
        ];
        let rocket = MassProperties::combine(&placed);

        // Each part as (mass, centre, own diagonal tensor [I_xx, I_yy, I_zz]).
        let mut parts: Vec<(f64, DVec3, [f64; 3])> = Vec::new();
        // Cone: I_axis = 3/10 m R², I_across = 3/80 m (4R² + L²), centre 3L/4 from the tip.
        let m = 1240.0 * PI * r * r * 0.3 / 3.0;
        let across = 3.0 / 80.0 * m * (4.0 * r * r + 0.09);
        parts.push((
            m,
            DVec3::new(0.0, 0.0, -0.225),
            [across, across, 0.3 * m * r * r],
        ));
        // Tube: radii 0.05 and 0.048, centre 0.4 m below its forward end at station 0.3.
        let radii = r * r + 0.048 * 0.048;
        let m = 790.0 * PI * (r * r - 0.048 * 0.048) * 0.8;
        let across = m * (radii / 4.0 + 0.64 / 12.0);
        parts.push((
            m,
            DVec3::new(0.0, 0.0, -0.7),
            [across, across, m * radii / 2.0],
        ));
        // Fins: boxes 0.1 (span) × 0.004 × 0.15, centres at radius R + s/2 = 0.1, station 1.025.
        let m = 630.0 * 0.15 * 0.1 * 0.004;
        let (s2, t2, c2) = (0.01, 0.004f64.powi(2), 0.0225);
        for (x, y) in [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)] {
            let span_along_x = x != 0.0;
            let (ixx, iyy) = if span_along_x {
                (m * (t2 + c2) / 12.0, m * (s2 + c2) / 12.0)
            } else {
                (m * (s2 + c2) / 12.0, m * (t2 + c2) / 12.0)
            };
            parts.push((
                m,
                DVec3::new(0.1 * x, 0.1 * y, -1.025),
                [ixx, iyy, m * (s2 + t2) / 12.0],
            ));
        }
        // Payload: solid cylinder a = 0.02, h = 0.1, at 0.03 m and 30°, centre at station 0.55.
        let across = 0.3 * (3.0 * 0.0004 + 0.01) / 12.0;
        parts.push((
            0.3,
            DVec3::new(0.03 * angle.cos(), 0.03 * angle.sin(), -0.55),
            [across, across, 0.3 * 0.0004 / 2.0],
        ));

        let total: f64 = parts.iter().map(|p| p.0).sum();
        let centre = parts.iter().fold(DVec3::ZERO, |sum, p| sum + p.1 * p.0) / total;
        let (mut xx, mut yy, mut zz, mut xy, mut xz, mut yz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for (m, c, own) in &parts {
            let d = *c - centre;
            xx += own[0] + m * (d.y * d.y + d.z * d.z);
            yy += own[1] + m * (d.x * d.x + d.z * d.z);
            zz += own[2] + m * (d.x * d.x + d.y * d.y);
            xy -= m * d.x * d.y;
            xz -= m * d.x * d.z;
            yz -= m * d.y * d.z;
        }
        let expected = DMat3::from_cols(
            DVec3::new(xx, xy, xz),
            DVec3::new(xy, yy, yz),
            DVec3::new(xz, yz, zz),
        );

        assert!((rocket.mass_kg - total).abs() < 1e-12 * total);
        assert!((rocket.cg_m - centre).length() < 1e-12);
        let scale = xx.max(yy).max(zz);
        let diff = (rocket.inertia_kg_m2 - expected)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |acc, v| acc.max(v.abs()));
        assert!(
            diff < 1e-11 * scale,
            "{:?}\nvs\n{expected:?}",
            rocket.inertia_kg_m2
        );
        // The off-axis payload makes the products of inertia non-zero.
        assert!(xy.abs() > 1e-6 && xz.abs() > 1e-6 && yz.abs() > 1e-6);
        rocket.validate().unwrap();
    }
    /// Loft lesson L47: the reference diameter was the widest component, which could be an
    /// internal one. Here the default is the widest body component; an oversized ring, a mass
    /// wider than the airframe, fins, tube fins and a shoulder don't count, and the nose-base and
    /// custom choices do what they say.
    #[test]
    fn reference_diameter_ignores_internal_components() {
        use crate::testing::{
            attached, body, fins, mass_component, nose, ring, rocket, stage, top, tube,
        };

        let mut upper = body("upper", tube(0.5, 0.03, 0.001));
        upper.children = vec![
            attached("oversized-ring", ring(0.005, 0.07, 0.0), top(0.1)),
            attached("wide-mass", mass_component(0.2, 0.05, 0.09), top(0.2)),
            attached("fins", fins(0.1, 0.2), top(0.3)),
            attached(
                "tube-fins",
                Part::TubeFinSet(TubeFinSet {
                    count: 6,
                    length_m: 0.1,
                    outer_radius_m: 0.03,
                    thickness_m: 0.001,
                    base_angle_rad: 0.0,
                    material: Material::bulk("cardboard", 790.0),
                }),
                top(0.35),
            ),
        ];
        let mut nose_cone = body("nose", nose(0.2, 0.03));
        if let Part::NoseCone(n) = &mut nose_cone.part {
            n.shoulder = Some(Shoulder {
                length_m: 0.05,
                outer_radius_m: 0.08,
                thickness_m: 0.002,
                capped: false,
            });
        }
        let lower = body(
            "flare",
            Part::Transition(Transition {
                shape: NoseShape::Conical {},
                clipped: false,
                length_m: 0.1,
                fore_radius_m: 0.03,
                aft_radius_m: 0.04,
                wall: Wall::Shell { thickness_m: 0.002 },
                fore_shoulder: None,
                aft_shoulder: None,
                material: Material::bulk("cardboard", 790.0),
            }),
        );
        let mut design = rocket(vec![stage(
            "s",
            vec![
                nose_cone,
                upper,
                lower,
                body("booster", tube(0.4, 0.04, 0.001)),
            ],
        )]);
        let layout = design.layout().unwrap();
        assert_eq!(layout.reference_diameter_m, 0.08);
        assert!((layout.reference_area_m2() - PI * 0.0016).abs() < 1e-15);

        // Without the wider booster section, the internal parts still don't count.
        let mut short = design.clone();
        short.stages[0].components.truncate(2);
        assert_eq!(short.layout().unwrap().reference_diameter_m, 0.06);

        design.reference_diameter = ReferenceDiameter::NoseBase {};
        assert_eq!(design.layout().unwrap().reference_diameter_m, 0.06);
        design.reference_diameter = ReferenceDiameter::Custom { diameter_m: 0.1 };
        assert_eq!(design.layout().unwrap().reference_diameter_m, 0.1);
        design.reference_diameter = ReferenceDiameter::Custom { diameter_m: 0.0 };
        assert!(matches!(design.layout(), Err(DesignError::Domain { .. })));

        // A bulged secant ogive is wider than its base, and that width counts.
        let mut bulged = short;
        if let Part::NoseCone(n) = &mut bulged.stages[0].components[0].part {
            n.shape = NoseShape::Ogive { radius_ratio: 0.5 };
        }
        let d = bulged.layout().unwrap().reference_diameter_m;
        assert!(d > 0.06 + 1e-4, "{d}");
    }
    /// Every public test design in `validation/designs/` (written by `cargo xtask designs`)
    /// resolves, has no findings, and assembles into a real body at ignition and burnout in every
    /// configuration.
    #[test]
    fn validation_designs_resolve_and_pass_checks() {
        const DESIGNS: [(&str, &str); 9] = [
            (
                "rocketpy-calisto-getting-started-motor-at-minus-1.255",
                include_str!(
                    "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
                ),
            ),
            (
                "rocketpy-calisto-tests-motor-at-minus-1.373",
                include_str!(
                    "../../../validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
                ),
            ),
            (
                "rocketpy-bella-lui",
                include_str!("../../../validation/designs/rocketpy-bella-lui.json"),
            ),
            (
                "rocketpy-ndrt-2020-nose-to-tail",
                include_str!("../../../validation/designs/rocketpy-ndrt-2020-nose-to-tail.json"),
            ),
            (
                "rocketpy-valetudo",
                include_str!("../../../validation/designs/rocketpy-valetudo.json"),
            ),
            (
                "rocketpy-juno-iii",
                include_str!("../../../validation/designs/rocketpy-juno-iii.json"),
            ),
            (
                "rocketpy-prometheus-2022-generic-motor",
                include_str!(
                    "../../../validation/designs/rocketpy-prometheus-2022-generic-motor.json"
                ),
            ),
            (
                "synthetic-54mm-three-fin",
                include_str!("../../../validation/designs/synthetic-54mm-three-fin.json"),
            ),
            (
                "synthetic-two-stage-75mm-54mm",
                include_str!("../../../validation/designs/synthetic-two-stage-75mm-54mm.json"),
            ),
        ];
        for (name, text) in DESIGNS {
            let design: Rocket = serde_json::from_str(text).unwrap();
            let findings = checks::check(&design).unwrap();
            println!("{name}: {findings:?}");
            assert!(findings.is_empty(), "{name}: {findings:?}");
            let layout = design.layout().unwrap();
            println!(
                "  structure {:.4} kg, centre at station {:.4} m, length {:.4} m, reference {:.4} m",
                layout.structure.mass_kg,
                -layout.structure.cg_m.z,
                layout.length_m,
                layout.reference_diameter_m
            );
            assert!(layout.reference_diameter_m > 0.0);
            assert!(!design.configurations.is_empty(), "{name}");
            for configuration in &design.configurations {
                let assembly = design.assemble(&configuration.id).unwrap();
                let burnout = assembly
                    .motors
                    .iter()
                    .map(|m| m.mounted.motor.burnout_time_s())
                    .fold(0.0, f64::max);
                for t in [0.0, 0.5 * burnout, burnout] {
                    let whole = assembly.mass_properties(t);
                    whole.validate().unwrap();
                    assert!(whole.mass_kg > layout.structure.mass_kg, "{name}");
                }
            }
        }
    }
}
