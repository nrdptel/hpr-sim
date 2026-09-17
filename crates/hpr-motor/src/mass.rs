//! Axisymmetric mass elements: a mass on the motor axis with its own axial and transverse moments
//! of inertia, and how elements combine.
//!
//! **Motor axis coordinate.** Positions are metres along the motor's axis, measured **from the
//! nozzle exit plane toward the forward closure**, so `+z` points toward the rocket's nose like the
//! body frame's `+z` (`docs/physics/frames.md`). Every element is symmetric about that axis, so its
//! inertia tensor about its own centre of mass is `diag(I_t, I_t, I_a)`.
//!
//! **Combining** uses the parallel-axis theorem (any dynamics text, e.g. Meriam and Kraige,
//! *Engineering Mechanics: Dynamics*, the appendix on mass moments of inertia). For elements with
//! masses `m_k`, centres `z_k`, and inertias `I_a,k`, `I_t,k` about their own centres,
//!
//! ```text
//! m = Σ m_k,   z = Σ m_k z_k / m,   I_a = Σ I_a,k,   I_t = Σ (I_t,k + m_k (z_k − z)²)
//! ```
//!
//! See `docs/physics/motor.md`.

use serde::{Deserialize, Serialize};

use crate::error::MotorError;

/// A mass on the motor axis, with its moments of inertia about its own centre of mass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MassElement {
    /// Mass, kg.
    pub mass_kg: f64,
    /// Centre of mass along the motor axis, m from the nozzle exit toward the forward end.
    pub cg_m: f64,
    /// Moment of inertia about the motor axis, through the element's centre of mass, kg·m².
    pub axial_inertia_kg_m2: f64,
    /// Moment of inertia about a transverse axis through the element's centre of mass, kg·m².
    pub transverse_inertia_kg_m2: f64,
}

impl MassElement {
    /// A massless element at the origin.
    pub const ZERO: Self = Self {
        mass_kg: 0.0,
        cg_m: 0.0,
        axial_inertia_kg_m2: 0.0,
        transverse_inertia_kg_m2: 0.0,
    };

    /// Checks that the mass and inertias are finite and non-negative and the position finite;
    /// `what` names the mass, position, axial and transverse inertia in an error.
    pub(crate) fn validate(&self, what: [&'static str; 4]) -> Result<(), MotorError> {
        let checks = [
            (self.mass_kg, what[0], true),
            (self.cg_m, what[1], false),
            (self.axial_inertia_kg_m2, what[2], true),
            (self.transverse_inertia_kg_m2, what[3], true),
        ];
        for (value, name, non_negative) in checks {
            if !value.is_finite() || (non_negative && value < 0.0) {
                return Err(MotorError::Domain { what: name, value });
            }
        }
        Ok(())
    }

    /// A thin-walled tube of radius `radius_m` and length `length_m` centred at `cg_m`:
    /// `I_a = m r²`, `I_t = m (r²/2 + L²/12)`.
    pub fn thin_tube(mass_kg: f64, cg_m: f64, radius_m: f64, length_m: f64) -> Self {
        let r2 = radius_m * radius_m;
        Self {
            mass_kg,
            cg_m,
            axial_inertia_kg_m2: mass_kg * r2,
            transverse_inertia_kg_m2: mass_kg * (0.5 * r2 + length_m * length_m / 12.0),
        }
    }

    /// A hollow cylinder of outer radius `outer_radius_m`, inner radius `inner_radius_m` and
    /// length `length_m`, centred at `cg_m`: `I_a = ½ m (R² + r²)`,
    /// `I_t = m ((R² + r²)/4 + L²/12)`. An inner radius of zero gives a solid cylinder.
    pub fn hollow_cylinder(
        mass_kg: f64,
        cg_m: f64,
        outer_radius_m: f64,
        inner_radius_m: f64,
        length_m: f64,
    ) -> Self {
        let radii = outer_radius_m * outer_radius_m + inner_radius_m * inner_radius_m;
        Self {
            mass_kg,
            cg_m,
            axial_inertia_kg_m2: 0.5 * mass_kg * radii,
            transverse_inertia_kg_m2: mass_kg * (0.25 * radii + length_m * length_m / 12.0),
        }
    }

    /// The elements combined into one: total mass, mass-weighted centre, and inertias moved to
    /// that centre with the parallel-axis theorem. With zero total mass the centre is the plain
    /// average of the element positions (or 0 with no elements) and the inertias are summed; a NaN
    /// mass gives a NaN centre.
    pub fn combine<'a>(elements: impl IntoIterator<Item = &'a MassElement> + Clone) -> Self {
        let mut mass = 0.0;
        let mut moment = 0.0;
        let mut axial = 0.0;
        let mut count = 0.0;
        let mut position_sum = 0.0;
        for element in elements.clone() {
            mass += element.mass_kg;
            moment += element.mass_kg * element.cg_m;
            axial += element.axial_inertia_kg_m2;
            count += 1.0;
            position_sum += element.cg_m;
        }
        let cg = if mass.is_nan() {
            f64::NAN
        } else if mass > 0.0 {
            moment / mass
        } else if count > 0.0 {
            position_sum / count
        } else {
            0.0
        };
        let transverse = elements
            .into_iter()
            .map(|element| {
                let arm = element.cg_m - cg;
                element.transverse_inertia_kg_m2 + element.mass_kg * arm * arm
            })
            .sum();
        Self {
            mass_kg: mass,
            cg_m: cg,
            axial_inertia_kg_m2: axial,
            transverse_inertia_kg_m2: transverse,
        }
    }

    /// The transverse moment of inertia about an axis through `z_m` instead of the centre of mass,
    /// kg·m² (the parallel-axis theorem).
    pub fn transverse_inertia_about(&self, z_m: f64) -> f64 {
        let arm = self.cg_m - z_m;
        self.transverse_inertia_kg_m2 + self.mass_kg * arm * arm
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combining_matches_the_parallel_axis_theorem() {
        let a = MassElement::hollow_cylinder(2.0, 0.1, 0.02, 0.01, 0.2);
        let b = MassElement::thin_tube(1.0, 0.4, 0.03, 0.5);
        let c = MassElement::combine([&a, &b]);
        assert_eq!(c.mass_kg, 3.0);
        assert!((c.cg_m - 0.2).abs() < 1e-15);
        assert_eq!(
            c.axial_inertia_kg_m2,
            a.axial_inertia_kg_m2 + b.axial_inertia_kg_m2
        );
        let expected = a.transverse_inertia_kg_m2
            + 2.0 * 0.1 * 0.1
            + b.transverse_inertia_kg_m2
            + 1.0 * 0.2 * 0.2;
        assert!((c.transverse_inertia_kg_m2 - expected).abs() < 1e-15);
        // Splitting a cylinder into two halves and recombining gives the whole cylinder back.
        let whole = MassElement::hollow_cylinder(4.0, 0.5, 0.05, 0.02, 1.0);
        let lower = MassElement::hollow_cylinder(2.0, 0.25, 0.05, 0.02, 0.5);
        let upper = MassElement::hollow_cylinder(2.0, 0.75, 0.05, 0.02, 0.5);
        let joined = MassElement::combine([&lower, &upper]);
        assert!((joined.cg_m - whole.cg_m).abs() < 1e-15);
        assert!((joined.axial_inertia_kg_m2 - whole.axial_inertia_kg_m2).abs() < 1e-15);
        assert!((joined.transverse_inertia_kg_m2 - whole.transverse_inertia_kg_m2).abs() < 1e-15);
        assert_eq!(
            joined.transverse_inertia_about(0.0),
            joined.transverse_inertia_kg_m2 + 4.0 * 0.25
        );
    }

    #[test]
    fn a_thin_tube_is_the_limit_of_a_hollow_cylinder() {
        let tube = MassElement::thin_tube(1.0, 0.0, 0.05, 0.3);
        let shell = MassElement::hollow_cylinder(1.0, 0.0, 0.05, 0.05, 0.3);
        assert!((tube.axial_inertia_kg_m2 - shell.axial_inertia_kg_m2).abs() < 1e-15);
        assert!((tube.transverse_inertia_kg_m2 - shell.transverse_inertia_kg_m2).abs() < 1e-15);
    }

    #[test]
    fn massless_combinations_stay_finite() {
        let none: [&MassElement; 0] = [];
        assert_eq!(MassElement::combine(none), MassElement::ZERO);
        let empty = MassElement {
            cg_m: 0.3,
            ..MassElement::ZERO
        };
        let other = MassElement {
            cg_m: 0.5,
            ..MassElement::ZERO
        };
        let c = MassElement::combine([&empty, &other]);
        assert!((c.cg_m - 0.4).abs() < 1e-15);
        assert_eq!(c.transverse_inertia_kg_m2, 0.0);
    }
}
