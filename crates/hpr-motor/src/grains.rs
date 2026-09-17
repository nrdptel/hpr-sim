//! BATES grains: a stack of identical hollow cylindrical grains that burn on their bores and,
//! unless inhibited, on both ends.
//!
//! The geometry follows RocketPy's `SolidMotor` (MIT; `rocketpy/motors/solid_motor.py`, v1.13.0),
//! which integrates the regression as an ODE in time. Here it is solved exactly in the burned
//! mass instead. Every burning surface recedes by the same web depth `x`, so one grain's volume is
//!
//! ```text
//! V(x) = π (R² − (r₀ + x)²) (h₀ − 2x)    ends burning,  0 ≤ x ≤ min(R − r₀, h₀/2)
//! V(x) = π (R² − (r₀ + x)²) h₀           ends inhibited, 0 ≤ x ≤ R − r₀
//! ```
//!
//! and `dV/dx = −A_b`, the burning area `2π (r h + R² − r²)` (or `2π r h`), which is RocketPy's
//! `ṙ = −V̇/A_b`, `ḣ = −2ṙ` written without time. Given the propellant mass left, `V(x) = m/(N ρ)`
//! is solved for `x` by safeguarded Newton iteration; `V` falls monotonically, so the root is
//! unique.
//!
//! Every grain needs a bore (`r₀ > 0`): a solid end burner shortens without widening, which this
//! regression doesn't describe (nor does RocketPy's). Facing ends burn even with no gap between
//! grains, as in RocketPy.
//!
//! The grains' centres stay put (both ends burn equally), spaced `h₀ + s` apart. About the
//! stack's centre, with `m_g = m/N` per grain and the current `r` and `h`:
//!
//! ```text
//! I_a = ½ m (R² + r²)
//! I_t = N m_g ((R² + r²)/4 + h²/12) + m_g (h₀ + s)² N (N² − 1)/12
//! ```
//!
//! The last term is `Σ m_g d_k²` over grain offsets `d_k = (k − (N−1)/2)(h₀ + s)`
//! (`solid_motor.py:724-740, 784-789`). See `docs/physics/motor.md`.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::error::MotorError;
use crate::mass::MassElement;

/// A stack of identical BATES grains.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BatesGrains {
    /// Number of grains `N`, at least 1.
    pub count: u32,
    /// Propellant density `ρ`, kg/m³.
    pub density_kg_m3: f64,
    /// Grain outer radius `R`, m.
    pub outer_radius_m: f64,
    /// Initial bore radius `r₀`, m, positive.
    pub initial_inner_radius_m: f64,
    /// Initial grain height (length) `h₀`, m.
    pub initial_height_m: f64,
    /// Gap between adjacent grains `s`, m.
    pub separation_m: f64,
    /// Centre of the grain stack along the motor axis, m from the nozzle exit.
    pub center_m: f64,
    /// Whether the grain ends are inhibited, so only the bores burn (RocketPy's
    /// `only_radial_burn`).
    pub inhibited_ends: bool,
}

/// The shape of one grain at some point in the burn.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GrainShape {
    /// Web burned so far `x`, m.
    pub web_burned_m: f64,
    /// Bore radius `r₀ + x`, m.
    pub inner_radius_m: f64,
    /// Grain height, m.
    pub height_m: f64,
}

impl BatesGrains {
    /// Checks the geometry.
    ///
    /// # Errors
    ///
    /// [`MotorError::Domain`] for a value that is not finite or out of range (including no bore),
    /// and [`MotorError::Inconsistent`] for no grains, a bore at least as wide as the grain, or a
    /// propellant mass that isn't a positive finite number (geometry so small or dense that it
    /// underflows or overflows).
    pub fn validate(&self) -> Result<(), MotorError> {
        if self.count == 0 {
            return Err(MotorError::Inconsistent(
                "a grain stack needs at least one grain".into(),
            ));
        }
        let positive = [
            (self.density_kg_m3, "grain density (kg/m³)"),
            (self.outer_radius_m, "grain outer radius (m)"),
            (self.initial_inner_radius_m, "grain bore radius (m)"),
            (self.initial_height_m, "grain height (m)"),
        ];
        for (value, what) in positive {
            if !(value.is_finite() && value > 0.0) {
                return Err(MotorError::Domain { what, value });
            }
        }
        if !(self.separation_m.is_finite() && self.separation_m >= 0.0) {
            return Err(MotorError::Domain {
                what: "grain separation (m)",
                value: self.separation_m,
            });
        }
        if !self.center_m.is_finite() {
            return Err(MotorError::Domain {
                what: "grain stack centre (m)",
                value: self.center_m,
            });
        }
        if self.initial_inner_radius_m >= self.outer_radius_m {
            return Err(MotorError::Inconsistent(format!(
                "grain bore radius {} m is not inside the outer radius {} m",
                self.initial_inner_radius_m, self.outer_radius_m
            )));
        }
        let mass = self.initial_mass_kg();
        if !(mass.is_finite() && mass > 0.0) {
            return Err(MotorError::Inconsistent(format!(
                "the grains' propellant mass {mass} kg is not a positive finite number"
            )));
        }
        Ok(())
    }

    /// The total propellant mass before ignition, `N ρ π (R² − r₀²) h₀`, kg.
    pub fn initial_mass_kg(&self) -> f64 {
        f64::from(self.count) * self.density_kg_m3 * self.volume_m3(0.0)
    }

    /// The largest web the grains can burn, m: where the bore reaches the outer radius or, with
    /// burning ends, the height reaches zero.
    pub fn web_m(&self) -> f64 {
        let radial = self.outer_radius_m - self.initial_inner_radius_m;
        if self.inhibited_ends {
            radial
        } else {
            radial.min(0.5 * self.initial_height_m)
        }
    }

    /// One grain's shape when `mass_kg` of propellant is left in the stack. Masses at or above the
    /// initial mass give the initial shape, and masses at or below zero the burned-out shape.
    pub fn shape(&self, mass_kg: f64) -> GrainShape {
        let target = mass_kg / (f64::from(self.count) * self.density_kg_m3);
        let web = self.web_m();
        if target.is_nan() || !(web.is_finite() && web > 0.0) {
            return GrainShape {
                web_burned_m: f64::NAN,
                inner_radius_m: f64::NAN,
                height_m: f64::NAN,
            };
        }
        let x = if target >= self.volume_m3(0.0) {
            0.0
        } else if target <= 0.0 {
            web
        } else {
            self.solve_web(target, web)
        };
        self.shape_at_web(x)
    }

    /// The grains, with `mass_kg` of propellant left, as one mass element about the stack centre.
    pub fn mass_element(&self, mass_kg: f64) -> MassElement {
        let mass = if mass_kg.is_nan() {
            f64::NAN
        } else {
            mass_kg.max(0.0)
        };
        let shape = self.shape(mass);
        let n = f64::from(self.count);
        let radii = self.outer_radius_m.powi(2) + shape.inner_radius_m.powi(2);
        let grain = mass / n;
        let pitch = self.initial_height_m + self.separation_m;
        MassElement {
            mass_kg: mass,
            cg_m: self.center_m,
            axial_inertia_kg_m2: 0.5 * mass * radii,
            transverse_inertia_kg_m2: n * grain * (0.25 * radii + shape.height_m.powi(2) / 12.0)
                + grain * pitch * pitch * n * (n * n - 1.0) / 12.0,
        }
    }

    /// One grain's volume after burning a web `x`, m³.
    fn volume_m3(&self, x: f64) -> f64 {
        let shape = self.shape_at_web(x);
        PI * (self.outer_radius_m.powi(2) - shape.inner_radius_m.powi(2)) * shape.height_m
    }

    /// The burning area `A_b = −dV/dx` of one grain at web `x`, m².
    fn burning_area_m2(&self, x: f64) -> f64 {
        let shape = self.shape_at_web(x);
        let bore = 2.0 * PI * shape.inner_radius_m * shape.height_m;
        if self.inhibited_ends {
            bore
        } else {
            bore + 2.0 * PI * (self.outer_radius_m.powi(2) - shape.inner_radius_m.powi(2))
        }
    }

    fn shape_at_web(&self, x: f64) -> GrainShape {
        let height = if self.inhibited_ends {
            self.initial_height_m
        } else {
            (self.initial_height_m - 2.0 * x).max(0.0)
        };
        GrainShape {
            web_burned_m: x,
            inner_radius_m: (self.initial_inner_radius_m + x).min(self.outer_radius_m),
            height_m: height,
        }
    }

    /// Solves `V(x) = target` on `[0, web]`, where `V(0) > target > 0 = V(web)`.
    fn solve_web(&self, target: f64, web: f64) -> f64 {
        // Newton's method from the first step at x = 0, kept inside a bracket [lo, hi] with
        // V(lo) > target > V(hi): a step that leaves the bracket bisects it instead. Newton
        // converges in a handful of steps, and stops once a step is within rounding of zero.
        let tolerance = 4.0 * f64::EPSILON * web;
        let (mut lo, mut hi) = (0.0, web);
        let mut x = ((self.volume_m3(0.0) - target) / self.burning_area_m2(0.0)).clamp(0.0, web);
        for _ in 0..100 {
            let residual = self.volume_m3(x) - target;
            if residual == 0.0 {
                return x;
            }
            if residual > 0.0 {
                lo = x;
            } else {
                hi = x;
            }
            let area = self.burning_area_m2(x);
            let newton = x + residual / area;
            if area > 0.0 && (newton - x).abs() <= tolerance {
                return newton.clamp(lo, hi);
            }
            x = if area > 0.0 && newton > lo && newton < hi {
                newton
            } else {
                0.5 * (lo + hi)
            };
            if hi - lo <= tolerance {
                return x;
            }
        }
        x
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn grains(inhibited_ends: bool) -> BatesGrains {
        BatesGrains {
            count: 3,
            density_kg_m3: 1815.0,
            outer_radius_m: 0.0165,
            initial_inner_radius_m: 0.006,
            initial_height_m: 0.09,
            separation_m: 0.005,
            center_m: 0.2,
            inhibited_ends,
        }
    }

    #[test]
    fn initial_mass_and_inertia_match_the_closed_forms() {
        let g = grains(false);
        let volume = PI * (0.0165f64.powi(2) - 0.006f64.powi(2)) * 0.09;
        assert!((g.initial_mass_kg() - 3.0 * 1815.0 * volume).abs() < 1e-15);
        let m = g.initial_mass_kg();
        let e = g.mass_element(m);
        let radii = 0.0165f64.powi(2) + 0.006f64.powi(2);
        assert!((e.axial_inertia_kg_m2 - 0.5 * m * radii).abs() < 1e-18);
        // Three grains 0.095 m apart: offsets −0.095, 0, 0.095.
        let each = m / 3.0 * (radii / 4.0 + 0.09f64.powi(2) / 12.0);
        let spread = m / 3.0 * 2.0 * 0.095f64.powi(2);
        assert!((e.transverse_inertia_kg_m2 - (3.0 * each + spread)).abs() < 1e-15);
        assert_eq!(e.cg_m, 0.2);
    }

    #[test]
    fn shape_inverts_the_volume() {
        for inhibited in [false, true] {
            let g = grains(inhibited);
            for x in [0.0, 1e-4, 0.003, 0.007, 0.01, g.web_m()] {
                let mass = 3.0 * 1815.0 * g.volume_m3(x);
                let shape = g.shape(mass);
                assert!(
                    (shape.web_burned_m - x).abs() < 1e-12,
                    "{inhibited} {x} {shape:?}"
                );
            }
            assert_eq!(g.shape(g.initial_mass_kg() * 2.0).web_burned_m, 0.0);
            assert_eq!(g.shape(-1.0).web_burned_m, g.web_m());
        }
        // Short grains burn out axially: the height reaches zero before the bore reaches R.
        let short = BatesGrains {
            initial_height_m: 0.01,
            ..grains(false)
        };
        assert_eq!(short.web_m(), 0.005);
        assert_eq!(short.shape(0.0).height_m, 0.0);
    }

    #[test]
    fn rejects_bad_geometry() {
        let good = grains(false);
        let bad = [
            BatesGrains { count: 0, ..good },
            BatesGrains {
                density_kg_m3: 0.0,
                ..good
            },
            BatesGrains {
                outer_radius_m: f64::NAN,
                ..good
            },
            BatesGrains {
                initial_inner_radius_m: 0.0165,
                ..good
            },
            BatesGrains {
                initial_inner_radius_m: -0.001,
                ..good
            },
            BatesGrains {
                separation_m: -0.001,
                ..good
            },
            BatesGrains {
                center_m: f64::INFINITY,
                ..good
            },
            BatesGrains {
                initial_inner_radius_m: 0.0,
                inhibited_ends: true,
                ..good
            },
            // An end burner: no bore, ends burning.
            BatesGrains {
                initial_inner_radius_m: 0.0,
                ..good
            },
            // Geometry whose mass underflows or overflows.
            BatesGrains {
                outer_radius_m: 1e-200,
                initial_inner_radius_m: 1e-201,
                ..good
            },
            BatesGrains {
                density_kg_m3: 1e308,
                ..good
            },
        ];
        for g in bad {
            assert!(g.validate().is_err(), "{g:?}");
        }
        assert!(good.validate().is_ok());
        // Unvalidated geometry gives NaN rather than panicking.
        for g in [
            BatesGrains {
                outer_radius_m: f64::NAN,
                inhibited_ends: true,
                ..good
            },
            BatesGrains {
                outer_radius_m: f64::NEG_INFINITY,
                ..good
            },
            BatesGrains {
                initial_inner_radius_m: 0.02,
                ..good
            },
        ] {
            assert!(g.shape(0.1).web_burned_m.is_nan(), "{g:?}");
            let _ = g.mass_element(0.1);
        }
        assert!(good.shape(f64::NAN).height_m.is_nan());
        assert!(good.mass_element(f64::NAN).mass_kg.is_nan());
    }

    proptest! {
        #[test]
        fn burning_area_is_minus_the_volume_derivative(
            fraction in 0.01..0.99f64, inhibited in any::<bool>()
        ) {
            let g = grains(inhibited);
            let x = fraction * g.web_m();
            let h = 1e-7;
            let derivative = (g.volume_m3(x + h) - g.volume_m3(x - h)) / (2.0 * h);
            prop_assert!((derivative + g.burning_area_m2(x)).abs() < 1e-6 * g.burning_area_m2(x));
        }

        #[test]
        fn mass_element_mass_round_trips(fraction in 0.0..1.0f64, inhibited in any::<bool>()) {
            let g = grains(inhibited);
            let mass = fraction * g.initial_mass_kg();
            let shape = g.shape(mass);
            let back = 3.0 * 1815.0 * g.volume_m3(shape.web_burned_m);
            prop_assert!((back - mass).abs() <= 1e-12 * g.initial_mass_kg());
        }
    }
}
