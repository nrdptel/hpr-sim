//! The canonical rocket design model: component tree, shapes, materials, mass properties, stages
//! and configurations, and design checks.
//!
//! - [`mass`]: mass, centre of mass and full inertia tensor, and how bodies combine.
//! - [`shapes`]: nose cone and transition profiles.
//! - [`solids`]: solids of revolution, filled or with a wall.
//! - [`fins`]: fin sets (trapezoidal, elliptical, freeform) and tube fins.
//! - [`parts`]: every other component, from body tubes to shock cords.
//! - [`material`]: materials and their densities, and [`materials`]: built-in values with sources.
//!
//! Physics: `docs/physics/shapes.md` and `docs/physics/mass.md`.

pub mod error;
pub mod fins;
pub mod mass;
pub mod material;
pub mod materials;
pub mod parts;
pub mod shapes;
pub mod solids;

pub use error::DesignError;
pub use fins::{FinCrossSection, FinPlanform, FinSet, FinTab, TubeFinSet};
pub use mass::MassProperties;
pub use material::{Density, Material};
pub use parts::{
    BodyTube, CenteringRing, InnerTube, LaunchLug, MassComponent, NoseCone, Packing, Parachute,
    RailButton, ShockCord, Shoulder, Streamer, Transition,
};
pub use shapes::{NoseShape, Profile};
pub use solids::{RevolvedGeometry, Wall, revolve};

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
            shape: NoseShape::Conical,
            length_m: 0.3,
            base_radius_m: r,
            wall: Wall::Filled,
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
}
