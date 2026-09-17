//! Rigid-body mass properties: mass, centre of mass and the full inertia tensor, and how bodies
//! combine, move and turn.
//!
//! **Frame.** Positions and tensors are in body axes (`docs/physics/frames.md`): `z` along the
//! axis of symmetry, positive toward the nose; `x` the design's zero radial direction;
//! `y = z × x`. A component's own frame has these axes with its origin on the axis at the
//! component's forward reference plane (its forward end, or a nose cone's tip), so points of the
//! component have `z ≤ 0`. The design places a component by translating and rolling it.
//!
//! **Inertia tensor** about a point `p` (positive products-of-inertia convention):
//!
//! ```text
//! I_p = ∫ (|r|² E − r rᵀ) dm,     r = position − p
//! ```
//!
//! so `I_xx = ∫(y² + z²) dm` and `I_xy = −∫ x y dm`. [`MassProperties`] stores it about the centre
//! of mass. The standard results used here (parallel-axis theorem, rotation of a tensor) are in any
//! dynamics text, for example J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*,
//! appendix B:
//!
//! ```text
//! parallel axis:  I_p = I_cg + m (|d|² E − d dᵀ),   d = cg − p
//! rotation R:     cg' = R cg,  I' = R I Rᵀ
//! combination:    m = Σ m_k,  cg = Σ m_k cg_k / m,  I = Σ [I_k + m_k (|d_k|² E − d_k d_kᵀ)],  d_k = cg_k − cg
//! ```
//!
//! See `docs/physics/mass.md`.

use hpr_core::{DMat3, DQuat, DVec3};
use hpr_motor::MassElement;
use serde::{Deserialize, Serialize};

use crate::error::DesignError;

/// Mass, centre of mass and inertia tensor about the centre of mass, in body axes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MassProperties {
    /// Mass, kg.
    pub mass_kg: f64,
    /// Centre of mass in body axes, m from the frame origin.
    pub cg_m: DVec3,
    /// Inertia tensor about the centre of mass, in body axes, kg·m².
    pub inertia_kg_m2: DMat3,
}

impl Default for MassProperties {
    fn default() -> Self {
        Self::ZERO
    }
}

/// `|d|² E − d dᵀ`: the parallel-axis term for unit mass at offset `d`.
fn offset_tensor(d: DVec3) -> DMat3 {
    DMat3::from_diagonal(DVec3::splat(d.length_squared())) - outer(d, d)
}

/// The outer product `a bᵀ`.
fn outer(a: DVec3, b: DVec3) -> DMat3 {
    DMat3::from_cols(a * b.x, a * b.y, a * b.z)
}

impl MassProperties {
    /// No mass, at the origin.
    pub const ZERO: Self = Self {
        mass_kg: 0.0,
        cg_m: DVec3::ZERO,
        inertia_kg_m2: DMat3::ZERO,
    };

    /// A point mass at `position_m`.
    pub fn point(mass_kg: f64, position_m: DVec3) -> Self {
        Self {
            mass_kg,
            cg_m: position_m,
            inertia_kg_m2: DMat3::ZERO,
        }
    }

    /// A body symmetric about a line parallel to `z` through `cg_m`, with moment of inertia
    /// `axial` about that line and `transverse` about any perpendicular line through the centre:
    /// the tensor `diag(I_t, I_t, I_a)`.
    pub fn axisymmetric(
        mass_kg: f64,
        cg_m: DVec3,
        axial_kg_m2: f64,
        transverse_kg_m2: f64,
    ) -> Self {
        Self {
            mass_kg,
            cg_m,
            inertia_kg_m2: DMat3::from_diagonal(DVec3::new(
                transverse_kg_m2,
                transverse_kg_m2,
                axial_kg_m2,
            )),
        }
    }

    /// A motor-crate mass element on the body axis. The element's axial coordinate runs toward the
    /// nose like `z`, so a nozzle exit at body `z = nozzle_z_m` puts the element's centre at
    /// `nozzle_z_m + element.cg_m`.
    pub fn from_motor_element(element: &MassElement, nozzle_z_m: f64) -> Self {
        Self::axisymmetric(
            element.mass_kg,
            DVec3::new(0.0, 0.0, nozzle_z_m + element.cg_m),
            element.axial_inertia_kg_m2,
            element.transverse_inertia_kg_m2,
        )
    }

    /// The inertia tensor about `point_m` instead of the centre of mass (parallel-axis theorem).
    pub fn inertia_about(&self, point_m: DVec3) -> DMat3 {
        self.inertia_kg_m2 + offset_tensor(self.cg_m - point_m) * self.mass_kg
    }

    /// The same body moved by `offset_m`.
    #[must_use]
    pub fn translated(&self, offset_m: DVec3) -> Self {
        Self {
            cg_m: self.cg_m + offset_m,
            ..*self
        }
    }

    /// The same body turned by `rotation` about the frame origin: `cg' = R cg`, `I' = R I Rᵀ`.
    #[must_use]
    pub fn rotated(&self, rotation: DQuat) -> Self {
        let r = DMat3::from_quat(rotation);
        Self {
            mass_kg: self.mass_kg,
            cg_m: r * self.cg_m,
            inertia_kg_m2: r * self.inertia_kg_m2 * r.transpose(),
        }
    }

    /// The same body rolled by `angle_rad` about the body `z` axis (right-handed: `x` toward `y`).
    #[must_use]
    pub fn rolled(&self, angle_rad: f64) -> Self {
        self.rotated(DQuat::from_rotation_z(angle_rad))
    }

    /// The same body with its mass scaled by `factor` and its shape unchanged: the inertia scales
    /// with the mass.
    #[must_use]
    pub fn scaled(&self, factor: f64) -> Self {
        Self {
            mass_kg: self.mass_kg * factor,
            cg_m: self.cg_m,
            inertia_kg_m2: self.inertia_kg_m2 * factor,
        }
    }

    /// The bodies combined into one rigid body.
    ///
    /// With zero total mass the centre is the plain average of the parts' centres (the origin with
    /// no parts) and the tensors are summed, so massless placeholders stay finite.
    pub fn combine<'a>(parts: impl IntoIterator<Item = &'a MassProperties> + Clone) -> Self {
        let mut mass = 0.0;
        let mut moment = DVec3::ZERO;
        let mut position_sum = DVec3::ZERO;
        let mut count = 0.0;
        for part in parts.clone() {
            mass += part.mass_kg;
            moment += part.cg_m * part.mass_kg;
            position_sum += part.cg_m;
            count += 1.0;
        }
        let cg = if mass.is_nan() {
            DVec3::NAN
        } else if mass > 0.0 {
            moment / mass
        } else if count > 0.0 {
            position_sum / count
        } else {
            DVec3::ZERO
        };
        let inertia = parts
            .into_iter()
            .fold(DMat3::ZERO, |sum, part| sum + part.inertia_about(cg));
        Self {
            mass_kg: mass,
            cg_m: cg,
            inertia_kg_m2: inertia,
        }
    }

    /// The principal moments of inertia about the centre of mass, ascending: the eigenvalues of
    /// the tensor.
    pub fn principal_moments_kg_m2(&self) -> [f64; 3] {
        symmetric_eigenvalues(self.inertia_kg_m2)
    }

    /// Checks that this could be a real body: finite, non-negative mass; finite centre; a
    /// symmetric tensor (to 1e-9 of its largest entry) whose principal moments are non-negative
    /// and obey the triangle inequality `I_1 + I_2 ≥ I_3` (to the same tolerance), and that is zero
    /// when the mass is.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a bad mass or centre, [`DesignError::UnphysicalInertia`] for a
    /// bad tensor.
    pub fn validate(&self) -> Result<(), DesignError> {
        if !self.mass_kg.is_finite() || self.mass_kg < 0.0 {
            return Err(DesignError::Domain {
                what: "mass",
                value: self.mass_kg,
            });
        }
        if !self.cg_m.is_finite() {
            return Err(DesignError::Domain {
                what: "centre of mass",
                value: self.cg_m.max_element().max(-self.cg_m.min_element()),
            });
        }
        let i = self.inertia_kg_m2;
        let entries = i.to_cols_array();
        if entries.iter().any(|v| !v.is_finite()) {
            return Err(DesignError::UnphysicalInertia(
                "the tensor has a non-finite entry".to_owned(),
            ));
        }
        let scale = entries.iter().fold(0.0f64, |m, v| m.max(v.abs()));
        if self.mass_kg == 0.0 && scale > 0.0 {
            return Err(DesignError::UnphysicalInertia(format!(
                "a body with no mass has no inertia, but this one has {scale:e} kg·m²"
            )));
        }
        let tolerance = 1e-9 * scale;
        let asymmetry = (i - i.transpose())
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        if asymmetry > tolerance {
            return Err(DesignError::UnphysicalInertia(format!(
                "the tensor is not symmetric (largest difference {asymmetry:e} kg·m²)"
            )));
        }
        let [a, b, c] = self.principal_moments_kg_m2();
        if a < -tolerance {
            return Err(DesignError::UnphysicalInertia(format!(
                "a principal moment is negative ({a:e} kg·m²)"
            )));
        }
        if a + b < c - tolerance {
            return Err(DesignError::UnphysicalInertia(format!(
                "the principal moments {a:e}, {b:e}, {c:e} kg·m² break I1 + I2 ≥ I3"
            )));
        }
        Ok(())
    }
}

/// Eigenvalues of a symmetric 3×3 matrix, ascending, by the cyclic Jacobi method (G. H. Golub and
/// C. F. Van Loan, *Matrix Computations*, 4th ed., 2013, §8.5.2–8.5.3), which is accurate to about
/// machine precision relative to the matrix norm even for repeated eigenvalues.
fn symmetric_eigenvalues(m: DMat3) -> [f64; 3] {
    let mut a = [
        [m.x_axis.x, m.y_axis.x, m.z_axis.x],
        [m.x_axis.y, m.y_axis.y, m.z_axis.y],
        [m.x_axis.z, m.y_axis.z, m.z_axis.z],
    ];
    // Symmetrize, so a slightly asymmetric input gives the eigenvalues of its symmetric part.
    for (p, q) in [(0, 1), (0, 2), (1, 2)] {
        let mean = 0.5 * (a[p][q] + a[q][p]);
        a[p][q] = mean;
        a[q][p] = mean;
    }
    // Quadratic convergence: a handful of sweeps reaches machine precision.
    for _ in 0..32 {
        let off = a[0][1].abs() + a[0][2].abs() + a[1][2].abs();
        if off == 0.0 {
            break;
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            if a[p][q] == 0.0 {
                continue;
            }
            // The symmetric Schur decomposition of the (p, q) block (Golub and Van Loan,
            // algorithm 8.5.1).
            let tau = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
            let t = tau.signum() / (tau.abs() + (1.0 + tau * tau).sqrt());
            let t = if tau == 0.0 { 1.0 } else { t };
            let c = 1.0 / (1.0 + t * t).sqrt();
            let s = t * c;
            for row in &mut a {
                let (kp, kq) = (row[p], row[q]);
                row[p] = c * kp - s * kq;
                row[q] = s * kp + c * kq;
            }
            #[expect(
                clippy::needless_range_loop,
                reason = "rows p and q of the same matrix change together"
            )]
            for k in 0..3 {
                let (pk, qk) = (a[p][k], a[q][k]);
                a[p][k] = c * pk - s * qk;
                a[q][k] = s * pk + c * qk;
            }
        }
    }
    let mut eig = [a[0][0], a[1][1], a[2][2]];
    eig.sort_by(f64::total_cmp);
    eig
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    fn assert_mat_close(a: DMat3, b: DMat3, tol: f64) {
        let diff = (a - b)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(diff <= tol, "{a:?}\nvs\n{b:?}\n(difference {diff:e})");
    }

    /// A solid box with sides `sx, sy, sz` centred at `centre`: `I = m/12 diag(sy²+sz², ...)`.
    fn box_body(mass: f64, sides: DVec3, centre: DVec3) -> MassProperties {
        let s2 = sides * sides;
        MassProperties {
            mass_kg: mass,
            cg_m: centre,
            inertia_kg_m2: DMat3::from_diagonal(
                DVec3::new(s2.y + s2.z, s2.x + s2.z, s2.x + s2.y) * (mass / 12.0),
            ),
        }
    }

    #[test]
    fn two_boxes_side_by_side_are_one_box() {
        // Two 1×1×1 boxes of 3 kg at x = ±0.5 make a 2×1×1 box of 6 kg at the origin.
        let left = box_body(3.0, DVec3::ONE, DVec3::new(-0.5, 0.0, 0.0));
        let right = box_body(3.0, DVec3::ONE, DVec3::new(0.5, 0.0, 0.0));
        let joined = MassProperties::combine([&left, &right]);
        let whole = box_body(6.0, DVec3::new(2.0, 1.0, 1.0), DVec3::ZERO);
        assert_eq!(joined.mass_kg, 6.0);
        assert!(joined.cg_m.length() < 1e-15);
        assert_mat_close(joined.inertia_kg_m2, whole.inertia_kg_m2, 1e-15);
    }

    #[test]
    fn point_masses_give_products_of_inertia_by_hand() {
        // 2 kg at (1, 2, 0) and 2 kg at (−1, −2, 0): centre at the origin, and by hand
        // I_xx = Σ m(y²+z²) = 16, I_yy = Σ m(x²+z²) = 4, I_zz = Σ m(x²+y²) = 20,
        // I_xy = −Σ m x y = −8, I_xz = I_yz = 0.
        let a = MassProperties::point(2.0, DVec3::new(1.0, 2.0, 0.0));
        let b = MassProperties::point(2.0, DVec3::new(-1.0, -2.0, 0.0));
        let c = MassProperties::combine([&a, &b]);
        let expected = DMat3::from_cols(
            DVec3::new(16.0, -8.0, 0.0),
            DVec3::new(-8.0, 4.0, 0.0),
            DVec3::new(0.0, 0.0, 20.0),
        );
        assert_mat_close(c.inertia_kg_m2, expected, 1e-15);
        let principal = c.principal_moments_kg_m2();
        for (got, want) in principal.iter().zip([0.0, 20.0, 20.0]) {
            assert!((got - want).abs() < 1e-14, "{principal:?}");
        }
        c.validate().unwrap();
        // About a point off the centre: add m (|d|² E − d dᵀ) with d = (0, 0, 1) and m = 4.
        let about = c.inertia_about(DVec3::new(0.0, 0.0, -1.0));
        assert_mat_close(
            about - c.inertia_kg_m2,
            DMat3::from_diagonal(DVec3::new(4.0, 4.0, 0.0)),
            1e-15,
        );
    }

    #[test]
    fn rotation_and_roll_move_the_tensor_with_the_body() {
        let body = box_body(1.0, DVec3::new(0.2, 0.4, 1.0), DVec3::new(0.1, 0.0, -0.5));
        // A quarter roll swaps x and y: the centre moves to (0, 0.1, −0.5) and I_xx ↔ I_yy.
        let rolled = body.rolled(std::f64::consts::FRAC_PI_2);
        assert!((rolled.cg_m - DVec3::new(0.0, 0.1, -0.5)).length() < 1e-16);
        let d = body.inertia_kg_m2;
        assert_mat_close(
            rolled.inertia_kg_m2,
            DMat3::from_diagonal(DVec3::new(d.y_axis.y, d.x_axis.x, d.z_axis.z)),
            1e-16,
        );
        // A 30° roll of a body with I_xx ≠ I_yy gives I_xy = −(I_yy − I_xx)... by the rotation
        // formula: I'_xy = (I_xx − I_yy) sin θ cos θ.
        let theta = 30f64.to_radians();
        let turned = body.rolled(theta);
        let expected_xy = (d.x_axis.x - d.y_axis.y) * theta.sin() * theta.cos();
        assert!((turned.inertia_kg_m2.y_axis.x - expected_xy).abs() < 1e-16);
        // Rotation keeps the principal moments.
        let a = body.principal_moments_kg_m2();
        let q = DQuat::from_euler(glam::EulerRot::XYZ, 0.3, -1.1, 2.0);
        let b = body.rotated(q).principal_moments_kg_m2();
        for k in 0..3 {
            assert!((a[k] - b[k]).abs() < 1e-15, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn motor_elements_land_on_the_axis_and_scaling_keeps_the_shape() {
        let element = MassElement::hollow_cylinder(1.2, 0.3, 0.027, 0.01, 0.4);
        let placed = MassProperties::from_motor_element(&element, -1.5);
        assert_eq!(placed.cg_m, DVec3::new(0.0, 0.0, -1.2));
        assert_eq!(placed.inertia_kg_m2.z_axis.z, element.axial_inertia_kg_m2);
        assert_eq!(
            placed.inertia_kg_m2.x_axis.x,
            element.transverse_inertia_kg_m2
        );
        let half = placed.scaled(0.5);
        assert_eq!(half.mass_kg, 0.6);
        assert_eq!(
            half.inertia_kg_m2.z_axis.z,
            0.5 * element.axial_inertia_kg_m2
        );
    }

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(err <= rel, "{what}: {got} vs {want} (relative {err:e})");
    }

    /// Loft lesson L44: Loft's inertia was pitch only, used the rod formula `mL²/12` without the
    /// radial term, ignored fin span, and gave rings and masses zero.
    #[test]
    fn thin_tube_inertia_includes_radial_term() {
        use crate::fins::{FinCrossSection, FinPlanform, FinSet};
        use crate::material::Material;
        use crate::parts::{BodyTube, CenteringRing, MassComponent, Packing};
        let tube = BodyTube {
            length_m: 0.6,
            outer_radius_m: 0.0508,
            thickness_m: 0.0015,
            material: Material::bulk("fiberglass", 1990.0),
        };
        let g = tube.mass_properties().unwrap();
        let (big, small) = (0.0508f64, 0.0493f64);
        let radii = big * big + small * small;
        let rod = g.mass_kg * 0.36 / 12.0;
        close(
            g.inertia_kg_m2.x_axis.x,
            g.mass_kg * (radii / 4.0 + 0.36 / 12.0),
            1e-14,
            "pitch",
        );
        close(
            g.inertia_kg_m2.y_axis.y,
            g.inertia_kg_m2.x_axis.x,
            1e-15,
            "yaw",
        );
        close(
            g.inertia_kg_m2.z_axis.z,
            g.mass_kg * radii / 2.0,
            1e-14,
            "roll",
        );
        // For this 4" tube the radial term is 2% of the rod value; it grows as tubes get stubbier.
        assert!(g.inertia_kg_m2.x_axis.x > 1.02 * rod);

        // Fins: the span sets the roll inertia and part of pitch.
        let fins = FinSet {
            count: 3,
            planform: FinPlanform::Trapezoidal {
                root_chord_m: 0.2,
                tip_chord_m: 0.1,
                span_m: 0.12,
                sweep_m: 0.1,
            },
            thickness_m: 0.003,
            cross_section: FinCrossSection::Square,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: Material::bulk("G10", 1800.0),
        };
        let f = fins.mass_properties(0.0508).unwrap();
        // Every fin element is at least R_b from the axis.
        assert!(f.inertia_kg_m2.z_axis.z > f.mass_kg * 0.0508 * 0.0508);
        assert!(f.inertia_kg_m2.x_axis.x > 0.5 * f.inertia_kg_m2.z_axis.z);

        // Rings and packed masses have inertia of their own.
        let ring = CenteringRing {
            length_m: 0.006,
            outer_radius_m: 0.0493,
            inner_radius_m: 0.0275,
            material: Material::bulk("plywood", 630.0),
        };
        let r = ring.mass_properties().unwrap();
        close(
            r.inertia_kg_m2.z_axis.z,
            r.mass_kg * (0.0493f64.powi(2) + 0.0275f64.powi(2)) / 2.0,
            1e-14,
            "ring roll",
        );
        let bay = MassComponent {
            mass_kg: 0.4,
            packing: Packing {
                length_m: 0.15,
                radius_m: 0.045,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
        };
        let b = bay.mass_properties().unwrap();
        close(
            b.inertia_kg_m2.x_axis.x,
            0.4 * (3.0 * 0.045f64.powi(2) + 0.0225) / 12.0,
            1e-14,
            "bay",
        );
    }

    /// Loft lesson L45: Loft put a hollow transition's CG at the solid centroid and hard-coded a
    /// freeform fin's CG at 0.42 of the root chord.
    #[test]
    fn hollow_transition_and_freeform_fin_cg_are_exact_centroids() {
        use crate::fins::{FinCrossSection, FinPlanform, FinSet};
        use crate::material::Material;
        use crate::parts::Transition;
        use crate::shapes::NoseShape;
        use crate::solids::Wall;
        // A conical shoulderless transition from R1 to R2 with a wall t normal to the surface. Cut
        // square, the inner surface would be the outer line moved in by t √(1 + k²),
        // k = (R2 − R1)/L, giving a wall of area π t w (2R1 − t w + 2kx) at x, w = √(1 + k²).
        let (r1, r2, l, t) = (0.0381f64, 0.0508f64, 0.1f64, 0.002f64);
        let k = (r2 - r1) / l;
        let w = (1.0 + k * k).sqrt();
        let square_volume = PI * t * w * ((2.0 * r1 - t * w) * l + k * l * l);
        let square_moment =
            PI * t * w * ((2.0 * r1 - t * w) * l * l / 2.0 + 2.0 * k * l.powi(3) / 3.0);
        // At the fore end the surface meets the end plane at an obtuse angle inside the wall, so
        // the wall is the square cut less the sliver outside the fore rim's circle. In polar
        // coordinates (ρ, ψ) about the rim, with φ = atan k, the sliver is 0 ≤ ψ ≤ φ,
        // t < ρ ≤ t / cos(φ − ψ), at x = ρ sin ψ and r = R1 − ρ cos ψ; its volume and first moment
        // are 2π ∬ r ρ dρ dψ and 2π ∬ x r ρ dρ dψ, done in ρ by hand and in ψ by quadrature.
        let phi = k.atan();
        let tol = hpr_core::quadrature::Tolerance::default();
        let (sliver_volume, sliver_moment) = {
            let span = |psi: f64| (t, t / (phi - psi).cos());
            let volume = hpr_core::quadrature::integrate_scalar(
                |psi| {
                    let (a, b) = span(psi);
                    r1 * (b * b - a * a) / 2.0 - psi.cos() * (b.powi(3) - a.powi(3)) / 3.0
                },
                0.0,
                phi,
                tol,
            )
            .unwrap();
            let moment = hpr_core::quadrature::integrate_scalar(
                |psi| {
                    let (a, b) = span(psi);
                    r1 * psi.sin() * (b.powi(3) - a.powi(3)) / 3.0
                        - psi.sin() * psi.cos() * (b.powi(4) - a.powi(4)) / 4.0
                },
                0.0,
                phi,
                tol,
            )
            .unwrap();
            (2.0 * PI * volume, 2.0 * PI * moment)
        };
        let volume = square_volume - sliver_volume;
        let moment = square_moment - sliver_moment;
        // The sliver's section is ∬ ρ dρ dψ = (t²/2) ∫ (sec²(φ − ψ) − 1) dψ = t² (tan φ − φ)/2.
        let sliver_area = hpr_core::quadrature::integrate_scalar(
            |psi| 0.5 * t * t * ((phi - psi).cos().powi(-2) - 1.0),
            0.0,
            phi,
            tol,
        )
        .unwrap();
        close(
            sliver_area,
            t * t * (phi.tan() - phi) / 2.0,
            1e-10,
            "sliver section",
        );
        let transition = Transition {
            shape: NoseShape::Conical {},
            clipped: false,
            length_m: l,
            fore_radius_m: r1,
            aft_radius_m: r2,
            wall: Wall::Shell { thickness_m: t },
            fore_shoulder: None,
            aft_shoulder: None,
            material: Material::bulk("PLA", 1240.0),
        };
        let g = transition.mass_properties().unwrap();
        close(g.mass_kg, 1240.0 * volume, 1e-10, "transition mass");
        close(-g.cg_m.z, moment / volume, 1e-10, "transition centre");
        // The solid frustum's centroid is further aft; the shell's is not the same point.
        let solid =
            l * (r1 * r1 + 2.0 * r1 * r2 + 3.0 * r2 * r2) / (4.0 * (r1 * r1 + r1 * r2 + r2 * r2));
        assert!((moment / volume - solid).abs() > 1e-3);

        // A non-convex (M-shaped) freeform fin against the polygon centroid (shoelace formulas).
        let points = vec![
            [0.0, 0.0],
            [0.04, 0.1],
            [0.07, 0.05],
            [0.1, 0.1],
            [0.14, 0.0],
        ];
        let (mut a2, mut cx, mut cy) = (0.0, 0.0, 0.0);
        for i in 0..points.len() {
            let [x0, y0] = points[i];
            let [x1, y1] = points[(i + 1) % points.len()];
            let cross = x0 * y1 - x1 * y0;
            a2 += cross;
            cx += (x0 + x1) * cross;
            cy += (y0 + y1) * cross;
        }
        let (cx, cy) = (cx / (3.0 * a2), cy / (3.0 * a2));
        let fins = FinSet {
            count: 1,
            planform: FinPlanform::Freeform { points_m: points },
            thickness_m: 0.003,
            cross_section: FinCrossSection::Square,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: Material::bulk("plywood", 630.0),
        };
        let fin = fins.single_fin(0.04).unwrap();
        close(
            fin.mass_kg,
            630.0 * 0.003 * 0.5 * a2.abs(),
            1e-12,
            "fin mass",
        );
        close(-fin.cg_m.z, cx, 1e-12, "fin centroid along the chord");
        close(fin.cg_m.x, 0.04 + cy, 1e-12, "fin centroid across the span");
        assert!((cx - 0.42 * 0.14).abs() > 1e-2);
    }

    /// Loft lesson L46: Loft never read fin tabs (100 to 120 g lost on two designs) and weighed
    /// rail buttons at 0 kg.
    #[test]
    fn fin_tab_and_rail_button_mass_counted() {
        use crate::fins::{FinCrossSection, FinPlanform, FinSet, FinTab};
        use crate::material::Material;
        use crate::parts::RailButton;
        let (h, len, t, rho, rb) = (0.02f64, 0.15f64, 0.003f64, 1800.0f64, 0.0508f64);
        let mut fins = FinSet {
            count: 4,
            planform: FinPlanform::Trapezoidal {
                root_chord_m: 0.2,
                tip_chord_m: 0.08,
                span_m: 0.1,
                sweep_m: 0.12,
            },
            thickness_m: t,
            cross_section: FinCrossSection::Square,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: Material::bulk("G10", rho),
        };
        let bare = fins.single_fin(rb).unwrap();
        fins.tab = Some(FinTab {
            height_m: h,
            length_m: len,
            offset_m: 0.025,
        });
        let tabbed = fins.single_fin(rb).unwrap();
        let tab_mass = rho * h * len * t;
        close(tabbed.mass_kg - bare.mass_kg, tab_mass, 1e-12, "tab mass");
        // The tab's centre is at radius R_b − h/2 and 0.025 + len/2 aft of the root leading edge.
        let expected_r = (bare.mass_kg * bare.cg_m.x + tab_mass * (rb - h / 2.0)) / tabbed.mass_kg;
        close(
            tabbed.cg_m.x,
            expected_r,
            1e-12,
            "tab moves the centre inward",
        );
        let set = fins.mass_properties(rb).unwrap();
        close(set.mass_kg, 4.0 * tabbed.mass_kg, 1e-14, "four tabbed fins");

        // A 1010-size button: 11.1 mm flange and base, 6.3 mm waist, 8.6 mm tall.
        let button = RailButton {
            outer_diameter_m: 0.0111,
            inner_diameter_m: 0.0063,
            height_m: 0.0086,
            base_height_m: 0.0025,
            flange_height_m: 0.0022,
            angle_rad: 0.0,
            count: 2,
            spacing_m: 0.5,
            material: Material::bulk("acetal", 1420.0),
        };
        let g = button.mass_properties(rb).unwrap();
        let one = 1420.0
            * PI
            * (0.0111f64.powi(2) / 4.0 * (0.0025 + 0.0022) + 0.0063f64.powi(2) / 4.0 * 0.0039);
        close(g.mass_kg, 2.0 * one, 1e-12, "two rail buttons");
        assert!(g.mass_kg > 1e-3);
    }

    /// A random physical body: a box of random sides and mass, turned and moved at random.
    fn arbitrary_body() -> impl proptest::strategy::Strategy<Value = MassProperties> {
        use proptest::prelude::*;
        (
            0.01..10.0f64,
            prop::array::uniform3(0.01..2.0f64),
            prop::array::uniform3(-3.0..3.0f64),
            prop::array::uniform3(-3.2..3.2f64),
        )
            .prop_map(|(mass, sides, centre, angles)| {
                box_body(mass, DVec3::from(sides), DVec3::ZERO)
                    .rotated(DQuat::from_euler(
                        glam::EulerRot::ZYX,
                        angles[0],
                        angles[1],
                        angles[2],
                    ))
                    .translated(DVec3::from(centre))
            })
    }

    proptest::proptest! {
        #[test]
        fn combining_is_associative_and_order_free(
            a in arbitrary_body(),
            b in arbitrary_body(),
            c in arbitrary_body(),
        ) {
            let all = MassProperties::combine([&a, &b, &c]);
            let nested = MassProperties::combine([&MassProperties::combine([&c, &a]), &b]);
            let scale = all.inertia_kg_m2.to_cols_array().iter().fold(0.0f64, |m, v| m.max(v.abs()));
            proptest::prop_assert!((all.mass_kg - nested.mass_kg).abs() <= 1e-13 * all.mass_kg);
            proptest::prop_assert!((all.cg_m - nested.cg_m).length() <= 1e-12);
            let diff = (all.inertia_kg_m2 - nested.inertia_kg_m2)
                .to_cols_array()
                .iter()
                .fold(0.0f64, |m, v| m.max(v.abs()));
            proptest::prop_assert!(diff <= 1e-11 * scale);
            proptest::prop_assert!(all.validate().is_ok());
        }

        #[test]
        fn turning_a_body_keeps_its_principal_moments(
            body in arbitrary_body(),
            angles in proptest::array::uniform3(-3.2..3.2f64),
        ) {
            let q = DQuat::from_euler(glam::EulerRot::XYZ, angles[0], angles[1], angles[2]);
            let turned = body.rotated(q);
            let (a, b) = (body.principal_moments_kg_m2(), turned.principal_moments_kg_m2());
            for k in 0..3 {
                proptest::prop_assert!((a[k] - b[k]).abs() <= 1e-12 * a[2]);
            }
            proptest::prop_assert!(turned.validate().is_ok());
            // The inertia about any point is at least the inertia about the centre of mass.
            let about = body.inertia_about(DVec3::new(0.3, -0.2, 1.0));
            let delta = MassProperties { inertia_kg_m2: about - body.inertia_kg_m2, ..body };
            let extra = delta.principal_moments_kg_m2();
            // `delta` is m(|d|² 1 − d dᵀ), with an exact zero eigenvalue along d: its rounding is
            // relative to the tensor's own size, which can dwarf the body's moments (#20).
            let size = delta
                .inertia_kg_m2
                .to_cols_array()
                .iter()
                .fold(a[2], |m, v| m.max(v.abs()));
            proptest::prop_assert!(extra[0] >= -1e-12 * size);
        }
    }

    #[test]
    fn massless_parts_and_unphysical_tensors() {
        let none: [&MassProperties; 0] = [];
        assert_eq!(MassProperties::combine(none), MassProperties::ZERO);
        let a = MassProperties::point(0.0, DVec3::new(0.0, 0.0, -1.0));
        let b = MassProperties::point(0.0, DVec3::new(0.0, 0.0, -3.0));
        assert_eq!(
            MassProperties::combine([&a, &b]).cg_m,
            DVec3::new(0.0, 0.0, -2.0)
        );
        let bad = MassProperties::axisymmetric(1.0, DVec3::ZERO, 1.0, 0.1);
        assert!(matches!(
            bad.validate(),
            Err(DesignError::UnphysicalInertia(_))
        ));
        let negative = MassProperties::axisymmetric(1.0, DVec3::ZERO, -1.0, 1.0);
        assert!(negative.validate().is_err());
        assert!(MassProperties::point(-1.0, DVec3::ZERO).validate().is_err());
        // A thin rod along z has I_zz = 0 and I_xx = I_yy: on the triangle-inequality boundary.
        MassProperties::axisymmetric(1.0, DVec3::ZERO, 0.0, 0.5)
            .validate()
            .unwrap();
        // No mass, no inertia; massless placeholders stay valid.
        assert!(matches!(
            MassProperties::axisymmetric(0.0, DVec3::ZERO, 1.0, 5.0).validate(),
            Err(DesignError::UnphysicalInertia(_))
        ));
        MassProperties::combine([&a, &b]).validate().unwrap();
    }
}
