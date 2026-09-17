//! Nose cone and transition profiles: radius and slope along the axis for every shape.
//!
//! A profile gives the outer radius `r(x)` at distance `x` aft of its forward end, over
//! `0 ≤ x ≤ L`. Every shape is defined by a normalized curve `g(ξ)` with `g(0) = 0` at the tip and
//! `g(1) = 1` at the base, where `ξ` runs from tip to base. The formulas, with `R` the base radius
//! and `L` the length, are from G. A. Crowell Sr., *The Descriptive Geometry of Nose Cones*, 1996
//! (pp. 1–6), and S. Niskanen, *OpenRocket technical documentation* v13.05, 2013, appendix A
//! (pp. 102–105):
//!
//! ```text
//! conical            g = ξ
//! ogive              y = √(ρ² − (Lξ − ρ cos α)²) + ρ sin α,   α = atan(R/L) − acos(√(L² + R²) / 2ρ)
//! elliptical         g = √(1 − (1 − ξ)²)
//! power series       g = ξⁿ,                            0.05 ≤ n ≤ 1
//! parabolic series   g = (2ξ − K′ξ²) / (2 − K′),        0 ≤ K′ ≤ 1
//! Haack series       g = √((θ − sin 2θ / 2 + C sin³θ) / π),   θ = acos(1 − 2ξ),   0 ≤ C ≤ 2/3
//! ```
//!
//! The ogive is Crowell's secant ogive: a circular arc of radius `ρ` through the tip and the base
//! rim. `ρ` is given as a multiple of the tangent-ogive radius `ρ_t = (R² + L²) / 2R`
//! ([`NoseShape::Ogive::radius_ratio`]): 1 is the tangent ogive, larger values are secant ogives
//! that meet the base at an angle, and values below 1 bulge beyond `R` before the base. The arc
//! passes through the tip only while its centre is not above the axis, which needs
//! `ρ ≥ (L² + R²) / 2L`, that is `radius_ratio ≥ R/L`. A cone is the limit of infinite `ρ`.
//! The Haack series is monotone for `C ≤ 2/3` (`d(g²)/dθ ∝ sin²θ (2 + 3C cos θ)`); `C = 0` is the
//! LD-Haack (von Kármán) ogive and `C = 1/3` the LV-Haack.
//!
//! **Transitions** join a fore radius `R_f` to an aft radius `R_a` over length `L`. The shape's tip
//! lies at the smaller end, so a transition that grows aft has `r = R_f + (R_a − R_f) g(x/L)` and
//! one that shrinks aft (a boattail) is its mirror image, `r = R_a + (R_f − R_a) g(1 − x/L)`.
//! A **clipped** transition instead takes a whole nose cone of base radius `max(R_f, R_a)` and
//! length `L_n ≥ L`, and cuts it where its radius is `min(R_f, R_a)`; `L_n` is chosen so that the
//! piece left is `L` long (OpenRocket technical documentation, §A.7, p. 105). A conical or
//! tangent-ogive transition is the same clipped or not.
//!
//! See `docs/physics/shapes.md`.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::error::DesignError;

/// The shape of a nose cone, or of a transition's profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum NoseShape {
    /// A straight cone.
    Conical {},
    /// A circular-arc ogive whose arc radius is `radius_ratio` times the tangent-ogive radius.
    Ogive {
        /// The arc radius over the tangent-ogive radius: 1 for a tangent ogive, above 1 for a
        /// secant ogive, below 1 (down to `R/L`) for a bulged secant ogive.
        radius_ratio: f64,
    },
    /// Half an ellipse: a blunt, rounded tip.
    Elliptical {},
    /// `g = ξⁿ`: `n = 1` is a cone and `n = ½` a paraboloid.
    PowerSeries {
        /// The exponent `n`, in `[0.05, 1]` ([`MIN_POWER_EXPONENT`]).
        exponent: f64,
    },
    /// Parabolic series: `K′ = 0` is a cone and `K′ = 1` a full parabola, tangent at the base.
    ParabolicSeries {
        /// The parameter `K′`, in `[0, 1]`.
        parameter: f64,
    },
    /// Haack series: `C = 0` is the von Kármán (LD-Haack) ogive and `C = 1/3` the LV-Haack.
    Haack {
        /// The parameter `C`, in `[0, 2/3]`.
        parameter: f64,
    },
}

/// The smallest power-series exponent accepted. Blunter profiles approach a flat face whose area the
/// integrals can't resolve: at `n = 1e-9` a nose's wetted area misses the face's `πR²`, a nose fails
/// to converge between `1e-8` and `0.01`, and an unclipped transition up to about `0.038`, where
/// the surface integrand `∝ ξ^(2n−1)` drives bisection to subnormal stations. `0.05` is the bluntest
/// checked, as a nose and as transitions both ways, against closed-form volumes. Model a flat face
/// as a tube and a bulkhead.
pub const MIN_POWER_EXPONENT: f64 = 0.05;

impl NoseShape {
    /// The tangent ogive.
    pub const TANGENT_OGIVE: Self = Self::Ogive { radius_ratio: 1.0 };
    /// The von Kármán (LD-Haack) ogive.
    pub const VON_KARMAN: Self = Self::Haack { parameter: 0.0 };
    /// The LV-Haack shape.
    pub const LV_HAACK: Self = Self::Haack {
        parameter: 1.0 / 3.0,
    };

    /// Checks the shape parameter's range. `fineness` is the length over the base radius of the
    /// curve the shape is applied to (the ogive's lower bound on `radius_ratio` depends on it).
    fn validate(&self, fineness: f64) -> Result<(), DesignError> {
        let (what, value, ok) = match *self {
            Self::Conical {} | Self::Elliptical {} => return Ok(()),
            Self::Ogive { radius_ratio } => (
                "ogive radius ratio",
                radius_ratio,
                // A small slack keeps the boundary shape (the arc centre at the tip) valid.
                radius_ratio.is_finite() && radius_ratio * fineness >= 1.0 - 1e-12,
            ),
            Self::PowerSeries { exponent } => (
                "power series exponent",
                exponent,
                (MIN_POWER_EXPONENT..=1.0).contains(&exponent),
            ),
            Self::ParabolicSeries { parameter } => (
                "parabolic series parameter",
                parameter,
                (0.0..=1.0).contains(&parameter),
            ),
            Self::Haack { parameter } => (
                "Haack series parameter",
                parameter,
                (0.0..=2.0 / 3.0).contains(&parameter),
            ),
        };
        if ok {
            Ok(())
        } else {
            Err(DesignError::Domain { what, value })
        }
    }
}

/// `θ − sin 2θ / 2`, by its Taylor series below `θ = 0.1`, where the difference cancels:
/// `Σ_{k≥1} (−1)^(k+1) 2^(2k) θ^(2k+1) / (2k+1)!`. Five terms leave an error below `1e-16`
/// relative there.
fn haack_core(theta: f64) -> f64 {
    if theta >= 0.1 {
        return theta - (2.0 * theta).sin() / 2.0;
    }
    let t2 = theta * theta;
    let mut term = theta;
    let mut sum = 0.0;
    for k in 1..=5 {
        // term = 2^(2k) θ^(2k+1) / (2k+1)!, built up from the previous one.
        let k2 = f64::from(2 * k);
        term *= 4.0 * t2 / (k2 * (k2 + 1.0));
        sum += if k % 2 == 1 { term } else { -term };
    }
    sum
}

/// A circular arc through `(0, 0)` and `(L, R)` in units of `R`: centre `(xc, yc)`, radius `rho`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Arc {
    xc: f64,
    yc: f64,
    rho: f64,
}

impl Arc {
    /// Crowell's secant ogive for fineness `lambda = L/R` and `rho = ratio · ρ_t` (units of `R`).
    /// The centre lies on the chord's perpendicular bisector, on the side away from the profile,
    /// at `d = √(ρ² − c²/4)` from the chord's midpoint (chord length `c = √(λ² + 1)`), which is the
    /// centre `(ρ cos α, ρ sin α)` of Crowell's formula computed without rounding `α` near `−π/2`.
    fn new(lambda: f64, ratio: f64) -> Self {
        let chord = (lambda * lambda + 1.0).sqrt();
        let rho = ratio * 0.5 * (lambda * lambda + 1.0);
        let half = 0.5 * chord;
        let d = ((rho - half).max(0.0) * (rho + half)).sqrt();
        Self {
            xc: 0.5 * lambda + d / chord,
            // Rounding can leave the boundary shape's centre a hair above the axis.
            yc: (0.5 - d * lambda / chord).min(0.0),
            rho,
        }
    }

    /// Height and slope at `x` (units of `R`). The arc passes through the origin, so
    /// `ρ² = x_c² + y_c²` and `ρ² − (x − x_c)² = y_c² + x (2x_c − x)`, a sum with no cancellation
    /// even when `ρ` is huge; and `y (y − 2y_c) = x (2x_c − x)` gives
    /// `y = x (2x_c − x) / (root − y_c)` without the cancellation in `root + y_c` near the tip.
    fn eval(&self, x: f64) -> (f64, f64) {
        let chord_term = x * (2.0 * self.xc - x);
        let root = (self.yc * self.yc + chord_term).max(0.0).sqrt();
        let denominator = root - self.yc;
        let y = if denominator > 0.0 {
            chord_term / denominator
        } else {
            0.0
        };
        (y, (self.xc - x) / root)
    }
}

/// The normalized curve of a shape at a given fineness.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Curve {
    Conical,
    Arc { arc: Arc, lambda: f64 },
    Elliptical,
    Power(f64),
    Parabolic(f64),
    Haack(f64),
}

impl Curve {
    fn new(shape: NoseShape, fineness: f64) -> Self {
        match shape {
            NoseShape::Conical {} => Self::Conical,
            NoseShape::Ogive { radius_ratio } => Self::Arc {
                arc: Arc::new(fineness, radius_ratio),
                lambda: fineness,
            },
            NoseShape::Elliptical {} => Self::Elliptical,
            NoseShape::PowerSeries { exponent } => Self::Power(exponent),
            NoseShape::ParabolicSeries { parameter } => Self::Parabolic(parameter),
            NoseShape::Haack { parameter } => Self::Haack(parameter),
        }
    }

    /// `(g(ξ), dg/dξ)` for `ξ` in `[0, 1]`. The slope is infinite at a blunt tip.
    fn eval(&self, xi: f64) -> (f64, f64) {
        let xi = xi.clamp(0.0, 1.0);
        match *self {
            Self::Conical => (xi, 1.0),
            Self::Arc { arc, lambda } => {
                let (y, slope) = arc.eval(lambda * xi);
                (y.max(0.0), slope * lambda)
            }
            Self::Elliptical => {
                let g = (xi * (2.0 - xi)).sqrt();
                (g, (1.0 - xi) / g)
            }
            Self::Power(n) => (xi.powf(n), n * xi.powf(n - 1.0)),
            Self::Parabolic(k) => (
                (2.0 * xi - k * xi * xi) / (2.0 - k),
                (2.0 - 2.0 * k * xi) / (2.0 - k),
            ),
            Self::Haack(c) => {
                // θ = acos(1 − 2ξ) = 2 asin(√ξ); the second form keeps θ exact near the tip.
                let theta = 2.0 * xi.sqrt().asin();
                let (sin, cos) = (theta.sin(), theta.cos());
                let g2 = (haack_core(theta) + c * sin * sin * sin) / PI;
                let g = g2.max(0.0).sqrt();
                if g == 0.0 {
                    return (0.0, f64::INFINITY);
                }
                // dg/dξ = sin θ (2 + 3C cos θ) / (π g), from d(g²)/dθ and dθ/dξ = 2 / sin θ.
                (g, sin * (2.0 + 3.0 * c * cos) / (PI * g))
            }
        }
    }
}

/// An axisymmetric profile: a nose cone (fore radius zero) or a transition, as radius against
/// distance aft of its forward end.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ProfileData", into = "ProfileData")]
pub struct Profile {
    shape: NoseShape,
    length_m: f64,
    fore_radius_m: f64,
    aft_radius_m: f64,
    clipped: bool,
    /// The curve and where it sits: `r = offset + scale · g(ξ)`.
    curve: Curve,
    /// Radius at the shape's tip end, m.
    offset_m: f64,
    /// Radius span of the curve, m.
    scale_m: f64,
    /// Whether the shape's tip is at the aft end (the profile shrinks aft).
    tip_aft: bool,
    /// For a clipped profile, the fraction of the whole nose cut away at the tip: `ξ` runs over
    /// `[xi0, 1]` along the piece.
    xi0: f64,
}

/// The serialized form of a [`Profile`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileData {
    shape: NoseShape,
    length_m: f64,
    fore_radius_m: f64,
    aft_radius_m: f64,
    #[serde(default)]
    clipped: bool,
}

impl TryFrom<ProfileData> for Profile {
    type Error = DesignError;

    fn try_from(data: ProfileData) -> Result<Self, DesignError> {
        Profile::transition(
            data.shape,
            data.length_m,
            data.fore_radius_m,
            data.aft_radius_m,
            data.clipped,
        )
    }
}

impl From<Profile> for ProfileData {
    fn from(profile: Profile) -> Self {
        Self {
            shape: profile.shape,
            length_m: profile.length_m,
            fore_radius_m: profile.fore_radius_m,
            aft_radius_m: profile.aft_radius_m,
            clipped: profile.clipped,
        }
    }
}

/// Checks that a dimension is finite and positive (or non-negative when `allow_zero`).
pub(crate) fn check_dimension(
    what: &'static str,
    value: f64,
    allow_zero: bool,
) -> Result<(), DesignError> {
    let ok = value.is_finite() && (value > 0.0 || (allow_zero && value == 0.0));
    if ok {
        Ok(())
    } else {
        Err(DesignError::Domain { what, value })
    }
}

impl Profile {
    /// A nose cone of length `length_m` and base radius `base_radius_m`, tip forward.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a non-positive or non-finite dimension or a shape parameter out
    /// of range.
    pub fn nose(shape: NoseShape, length_m: f64, base_radius_m: f64) -> Result<Self, DesignError> {
        check_dimension("nose cone base radius", base_radius_m, false)?;
        Self::transition(shape, length_m, 0.0, base_radius_m, false)
    }

    /// A transition from `fore_radius_m` to `aft_radius_m` over `length_m`; see the module docs
    /// for how the shape is oriented and what `clipped` means. Equal radii give a cylinder.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a non-positive length, a negative or non-finite radius, both
    /// radii zero, or a shape parameter out of range.
    pub fn transition(
        shape: NoseShape,
        length_m: f64,
        fore_radius_m: f64,
        aft_radius_m: f64,
        clipped: bool,
    ) -> Result<Self, DesignError> {
        check_dimension("profile length", length_m, false)?;
        check_dimension("profile fore radius", fore_radius_m, true)?;
        check_dimension("profile aft radius", aft_radius_m, true)?;
        let big = fore_radius_m.max(aft_radius_m);
        let small = fore_radius_m.min(aft_radius_m);
        check_dimension("profile largest radius", big, false)?;
        let tip_aft = aft_radius_m < fore_radius_m;
        let mut profile = Self {
            shape,
            length_m,
            fore_radius_m,
            aft_radius_m,
            clipped,
            curve: Curve::Conical,
            offset_m: small,
            scale_m: big - small,
            tip_aft,
            xi0: 0.0,
        };
        if big == small {
            // A cylinder: the curve's scale is zero, so its shape doesn't matter.
            return Ok(profile);
        }
        if clipped && small > 0.0 {
            let (curve, nose_length) = clip(shape, length_m, small, big)?;
            profile.curve = curve;
            profile.offset_m = 0.0;
            profile.scale_m = big;
            profile.xi0 = 1.0 - length_m / nose_length;
        } else {
            let fineness = length_m / (big - small);
            shape.validate(fineness)?;
            profile.curve = Curve::new(shape, fineness);
        }
        Ok(profile)
    }

    /// The shape.
    pub fn shape(&self) -> NoseShape {
        self.shape
    }

    /// Length, m.
    pub fn length_m(&self) -> f64 {
        self.length_m
    }

    /// Radius at the forward end, m.
    pub fn fore_radius_m(&self) -> f64 {
        self.fore_radius_m
    }

    /// Radius at the aft end, m.
    pub fn aft_radius_m(&self) -> f64 {
        self.aft_radius_m
    }

    /// Whether the profile is cut from a longer nose cone.
    pub fn clipped(&self) -> bool {
        self.clipped
    }

    /// The largest radius anywhere on the profile, m (above both end radii for a bulged ogive).
    pub fn max_radius_m(&self) -> f64 {
        let ends = self.fore_radius_m.max(self.aft_radius_m);
        match self.curve {
            Curve::Arc { arc, lambda } if self.scale_m > 0.0 => {
                // The arc peaks at its centre's station when that lies inside the curve.
                let xi_peak = arc.xc / lambda;
                let lo = self.xi0;
                if xi_peak > lo && xi_peak < 1.0 {
                    ends.max(self.offset_m + self.scale_m * (arc.rho + arc.yc))
                } else {
                    ends
                }
            }
            _ => ends,
        }
    }

    /// `ξ` along the curve at `distance_m` from the forward end (or the aft end when `from_aft`),
    /// and `dξ/dx`. Measuring from the end nearer the tip keeps `ξ` exact where a blunt tip's
    /// slope blows up.
    fn xi(&self, distance_m: f64, from_aft: bool) -> (f64, f64) {
        let fraction = (distance_m / self.length_m).clamp(0.0, 1.0);
        let span = 1.0 - self.xi0;
        let toward_tip = if from_aft == self.tip_aft {
            fraction
        } else {
            1.0 - fraction
        };
        let sign = if self.tip_aft { -1.0 } else { 1.0 };
        (self.xi0 + span * toward_tip, sign * span / self.length_m)
    }

    /// Radius at `x_m` aft of the forward end, m; `x` is clamped to `[0, L]`.
    pub fn radius_m(&self, x_m: f64) -> f64 {
        self.radius_and_slope(x_m).0
    }

    /// Radius and slope `dr/dx` at `x_m`; the slope is infinite at a blunt tip.
    pub fn radius_and_slope(&self, x_m: f64) -> (f64, f64) {
        self.at_distance(x_m, false)
    }

    /// Radius and slope `dr/dx` at `distance_m` from the forward end, or from the aft end when
    /// `from_aft`, computed without rounding the distance through `L − x`.
    pub(crate) fn at_distance(&self, distance_m: f64, from_aft: bool) -> (f64, f64) {
        if self.scale_m == 0.0 {
            return (self.offset_m, 0.0);
        }
        let (xi, dxi) = self.xi(distance_m, from_aft);
        let (g, dg) = self.curve.eval(xi);
        (self.offset_m + self.scale_m * g, self.scale_m * dg * dxi)
    }
}

/// For a clipped transition, the whole nose cone's curve and length: the nose of base radius
/// `big` whose radius falls to `small` at `length` from its base.
fn clip(shape: NoseShape, length: f64, small: f64, big: f64) -> Result<(Curve, f64), DesignError> {
    let target = small / big;
    // Shapes other than the ogive have a curve independent of fineness: invert g directly.
    let invert = |curve: Curve| -> f64 {
        let (mut lo, mut hi) = (0.0, 1.0);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if curve.eval(mid).0 < target {
                lo = mid;
            } else {
                hi = mid;
            }
            if hi - lo <= f64::EPSILON {
                break;
            }
        }
        0.5 * (lo + hi)
    };
    match shape {
        NoseShape::Ogive { radius_ratio } if radius_ratio < 1.0 => {
            Err(DesignError::Geometry(format!(
                "a clipped ogive transition needs a monotone profile, so its radius ratio must be at \
             least 1, not {radius_ratio}"
            )))
        }
        NoseShape::Ogive { radius_ratio } => {
            // The whole nose's fineness sets its arc; find the nose length whose cut piece is
            // `length` long. The cut fraction grows with the nose length, so bisect on it.
            let piece = |nose_length: f64| -> Result<f64, DesignError> {
                let fineness = nose_length / big;
                shape.validate(fineness)?;
                let xi0 = invert(Curve::new(shape, fineness));
                Ok(nose_length * (1.0 - xi0))
            };
            let mut lo = length;
            let mut hi = length;
            // Grow the bracket until the piece is at least `length` long.
            let mut grown = false;
            for _ in 0..200 {
                hi *= 2.0;
                if piece(hi).is_ok_and(|p| p >= length) {
                    grown = true;
                    break;
                }
            }
            if !grown {
                return Err(DesignError::Domain {
                    what: "ogive radius ratio",
                    value: radius_ratio,
                });
            }
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                match piece(mid) {
                    Ok(p) if p >= length => hi = mid,
                    _ => lo = mid,
                }
                if hi - lo <= 4.0 * f64::EPSILON * hi {
                    break;
                }
            }
            let fineness = hi / big;
            shape.validate(fineness)?;
            Ok((Curve::new(shape, fineness), hi))
        }
        _ => {
            shape.validate(1.0)?;
            let curve = Curve::new(shape, 1.0);
            let xi0 = invert(curve);
            Ok((curve, length / (1.0 - xi0)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [NoseShape; 9] = [
        NoseShape::Conical {},
        NoseShape::TANGENT_OGIVE,
        NoseShape::Ogive { radius_ratio: 2.5 },
        NoseShape::Ogive { radius_ratio: 0.6 },
        NoseShape::Elliptical {},
        NoseShape::PowerSeries { exponent: 0.5 },
        NoseShape::ParabolicSeries { parameter: 0.75 },
        NoseShape::VON_KARMAN,
        NoseShape::LV_HAACK,
    ];

    #[test]
    fn every_nose_runs_from_the_tip_to_the_base_radius() {
        for shape in ALL {
            let nose = Profile::nose(shape, 0.3, 0.05).unwrap();
            assert!(nose.radius_m(0.0).abs() < 1e-15, "{shape:?}");
            assert!((nose.radius_m(0.3) - 0.05).abs() < 1e-15, "{shape:?}");
            // The slope matches a central difference inside the profile.
            for x in [0.03, 0.11, 0.2, 0.29] {
                let h = 1e-6;
                let numeric = (nose.radius_m(x + h) - nose.radius_m(x - h)) / (2.0 * h);
                let (_, slope) = nose.radius_and_slope(x);
                assert!(
                    (numeric - slope).abs() < 1e-7 * slope.abs().max(1.0),
                    "{shape:?} at {x}: {numeric} vs {slope}"
                );
            }
        }
    }

    /// Loft lesson L48: Loft had only the tangent ogive, a silent power-series default, and Haack
    /// names swapped in a comment.
    #[test]
    fn secant_ogive_and_haack_parameters_change_profile() {
        let (length, radius) = (0.4, 0.05);
        let at =
            |shape: NoseShape, x: f64| Profile::nose(shape, length, radius).unwrap().radius_m(x);
        // The tangent ogive meets the base with zero slope; a secant ogive doesn't.
        let tangent = Profile::nose(NoseShape::TANGENT_OGIVE, length, radius).unwrap();
        assert!(tangent.radius_and_slope(length).1.abs() < 1e-12);
        let secant = Profile::nose(NoseShape::Ogive { radius_ratio: 2.0 }, length, radius).unwrap();
        assert!(secant.radius_and_slope(length).1 > 0.01);
        // A secant ogive lies between the cone and the tangent ogive; a bulged one exceeds R.
        let x = 0.2;
        assert!(at(NoseShape::Conical {}, x) < at(NoseShape::Ogive { radius_ratio: 2.0 }, x));
        assert!(at(NoseShape::Ogive { radius_ratio: 2.0 }, x) < at(NoseShape::TANGENT_OGIVE, x));
        let bulged = Profile::nose(NoseShape::Ogive { radius_ratio: 0.5 }, length, radius).unwrap();
        assert!(bulged.max_radius_m() > radius);
        assert!(bulged.radius_m(0.35) > radius);
        // An enormous ogive radius approaches the cone.
        let flat = at(NoseShape::Ogive { radius_ratio: 1e8 }, x);
        assert!((flat - at(NoseShape::Conical {}, x)).abs() < 1e-8);
        // Haack C = 1/3 (LV) is fuller than C = 0 (von Kármán) everywhere inside, and the tips
        // are the published closed forms: at ξ = ½, θ = π/2 and g² = (π/2 + C)/π.
        let mid = length / 2.0;
        let vk = at(NoseShape::VON_KARMAN, mid);
        let lv = at(NoseShape::LV_HAACK, mid);
        assert!((vk - radius * 0.5f64.sqrt()).abs() < 1e-15);
        assert!((lv - radius * ((PI / 2.0 + 1.0 / 3.0) / PI).sqrt()).abs() < 1e-15);
        assert!(lv > vk);
        // Out-of-range parameters are errors, not silent defaults.
        for shape in [
            NoseShape::PowerSeries { exponent: 0.0 },
            NoseShape::PowerSeries { exponent: 1.5 },
            NoseShape::PowerSeries { exponent: 0.049 },
            NoseShape::ParabolicSeries { parameter: -0.1 },
            NoseShape::Haack { parameter: 0.7 },
            NoseShape::Ogive { radius_ratio: 0.1 },
            NoseShape::Ogive {
                radius_ratio: f64::NAN,
            },
        ] {
            assert!(Profile::nose(shape, length, radius).is_err(), "{shape:?}");
        }
    }

    /// Loft lesson L49: Loft's transitions used a "clipped nose" profile with a kink and never
    /// checked it against what `.ork` files mean.
    #[test]
    fn ogive_transition_hits_both_radii_and_is_monotone() {
        for shape in ALL {
            if matches!(shape, NoseShape::Ogive { radius_ratio } if radius_ratio < 1.0) {
                continue; // a bulged ogive is deliberately not monotone
            }
            for clipped in [false, true] {
                for (fore, aft) in [(0.03, 0.05), (0.05, 0.03)] {
                    let t = Profile::transition(shape, 0.1, fore, aft, clipped).unwrap();
                    assert!(
                        (t.radius_m(0.0) - fore).abs() < 1e-12,
                        "{shape:?} {clipped}"
                    );
                    assert!((t.radius_m(0.1) - aft).abs() < 1e-12, "{shape:?} {clipped}");
                    let mut last = t.radius_m(0.0);
                    for i in 1..=1000 {
                        let r = t.radius_m(0.1 * f64::from(i) / 1000.0);
                        if fore < aft {
                            assert!(r >= last - 1e-15, "{shape:?} {clipped} grows");
                        } else {
                            assert!(r <= last + 1e-15, "{shape:?} {clipped} shrinks");
                        }
                        last = r;
                    }
                }
            }
        }
        // A boattail is the mirror image of the growing transition.
        let grow = Profile::transition(NoseShape::TANGENT_OGIVE, 0.1, 0.03, 0.05, false).unwrap();
        let shrink = Profile::transition(NoseShape::TANGENT_OGIVE, 0.1, 0.05, 0.03, false).unwrap();
        for x in [0.0, 0.013, 0.05, 0.08] {
            assert!((grow.radius_m(x) - shrink.radius_m(0.1 - x)).abs() < 1e-15);
        }
        // Clipped and unclipped agree for the cone and the tangent ogive (techdoc §A.7) and differ
        // for a power series.
        for shape in [NoseShape::Conical {}, NoseShape::TANGENT_OGIVE] {
            let a = Profile::transition(shape, 0.1, 0.03, 0.05, false).unwrap();
            let b = Profile::transition(shape, 0.1, 0.03, 0.05, true).unwrap();
            for x in [0.01, 0.04, 0.07] {
                assert!(
                    (a.radius_m(x) - b.radius_m(x)).abs() < 1e-12,
                    "{shape:?} at {x}"
                );
            }
        }
        let shape = NoseShape::PowerSeries { exponent: 0.5 };
        let a = Profile::transition(shape, 0.1, 0.03, 0.05, false).unwrap();
        let b = Profile::transition(shape, 0.1, 0.03, 0.05, true).unwrap();
        assert!((a.radius_m(0.02) - b.radius_m(0.02)).abs() > 1e-3);
        // A clipped power-series transition is a piece of the whole nose: r = R (x_n / L_n)^n.
        let n_len = 0.1 / (1.0 - (0.03f64 / 0.05).powi(2));
        let x0 = n_len - 0.1;
        assert!((b.radius_m(0.02) - 0.05 * ((x0 + 0.02) / n_len).sqrt()).abs() < 1e-12);
    }

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(err <= rel, "{what}: {got} vs {want} (relative {err:e})");
    }

    /// Loft lesson L91: Loft's nose-volume tests (cone πR²L/3; tangent ogive R 0.04 m, L 0.25 m
    /// gives 6.7509e-4 m³; Haack πR²L(1/2 + 3C/16)) are worth keeping. Here every shape's filled
    /// volume and centroid, and every closed-form wetted and planform area, is checked against its
    /// closed form (Crowell 1996, pp. 12-14, with the corrections in `docs/physics/shapes.md`).
    #[test]
    fn nose_volumes_match_closed_forms() {
        use crate::solids::{Wall, revolve};
        let tol = 1e-10;
        let (r, l): (f64, f64) = (0.05, 0.3);
        let solid = |shape| revolve(&Profile::nose(shape, l, r).unwrap(), Wall::Filled {}).unwrap();

        // Cone.
        let g = solid(NoseShape::Conical {});
        close(g.volume_m3, PI * r * r * l / 3.0, tol, "cone volume");
        close(g.centroid_m, 0.75 * l, tol, "cone centroid");
        close(
            g.wetted_area_m2,
            PI * r * (r * r + l * l).sqrt(),
            tol,
            "cone area",
        );
        close(g.planform_area_m2, r * l, tol, "cone planform");
        close(
            g.planform_centroid_m,
            2.0 * l / 3.0,
            tol,
            "cone planform centroid",
        );

        // Loft's number for the tangent ogive.
        let loft = revolve(
            &Profile::nose(NoseShape::TANGENT_OGIVE, 0.25, 0.04).unwrap(),
            Wall::Filled {},
        )
        .unwrap();
        assert!((loft.volume_m3 - 6.7509e-4).abs() < 5e-9);

        // Ogives, as arcs of radius ρ centred at (xc, yc): y = √(ρ² − (x − xc)²) + yc.
        for ratio in [1.0, 2.5, 0.6] {
            let lam = l / r;
            let rho = ratio * (r * r + l * l) / (2.0 * r);
            let alpha = (r / l).atan() - ((l * l + r * r).sqrt() / (2.0 * rho)).acos();
            let (xc, yc) = (rho * alpha.cos(), rho * alpha.sin());
            assert!(ratio * lam >= 1.0);
            // F(u) = ∫ √(ρ² − u²) du and G(u) = ∫ u √(ρ² − u²) du.
            let f = |u: f64| 0.5 * (u * (rho * rho - u * u).sqrt() + rho * rho * (u / rho).asin());
            let g_ = |u: f64| -(rho * rho - u * u).powf(1.5) / 3.0;
            let (u0, u1) = (-xc, l - xc);
            let volume = PI
                * ((rho * rho + yc * yc) * l - (u1.powi(3) - u0.powi(3)) / 3.0
                    + 2.0 * yc * (f(u1) - f(u0)));
            // ∫ x y² dx with x = u + xc.
            let moment = PI
                * ((rho * rho + yc * yc) * l * l / 2.0
                    - ((u1.powi(4) - u0.powi(4)) / 4.0 + xc * (u1.powi(3) - u0.powi(3)) / 3.0)
                    + 2.0 * yc * (g_(u1) - g_(u0) + xc * (f(u1) - f(u0))));
            let area = 2.0 * PI * rho * (l + yc * ((u1 / rho).asin() - (u0 / rho).asin()));
            let planform = 2.0 * (yc * l + f(u1) - f(u0));
            let s = solid(NoseShape::Ogive {
                radius_ratio: ratio,
            });
            close(s.volume_m3, volume, tol, "ogive volume");
            close(s.centroid_m, moment / volume, tol, "ogive centroid");
            close(s.wetted_area_m2, area, tol, "ogive area");
            close(s.planform_area_m2, planform, tol, "ogive planform");
        }

        // Half a prolate spheroid, tip forward: centroid 3L/8 from the base; Crowell's area with
        // e = √(1 − R²/L²); planform a half ellipse with centroid 4L/3π from the base.
        let g = solid(NoseShape::Elliptical {});
        let e = (1.0 - r * r / (l * l)).sqrt();
        close(
            g.volume_m3,
            2.0 * PI * r * r * l / 3.0,
            tol,
            "ellipsoid volume",
        );
        close(g.centroid_m, 5.0 * l / 8.0, tol, "ellipsoid centroid");
        close(
            g.wetted_area_m2,
            PI * r * r + PI * r * l * e.asin() / e,
            tol,
            "ellipsoid area",
        );
        close(
            g.planform_area_m2,
            PI * r * l / 2.0,
            tol,
            "ellipse planform",
        );
        close(
            g.planform_centroid_m,
            l - 4.0 * l / (3.0 * PI),
            tol,
            "ellipse planform centroid",
        );
        // An oblate half spheroid (L < R): area πR² + (πL²/2e) ln((1 + e)/(1 − e)), e = √(1 − L²/R²).
        let oblate = revolve(
            &Profile::nose(NoseShape::Elliptical {}, 0.03, r).unwrap(),
            Wall::Filled {},
        )
        .unwrap();
        let e = (1.0 - 0.03f64.powi(2) / (r * r)).sqrt();
        let area = PI * r * r + PI * 0.03f64.powi(2) / (2.0 * e) * ((1.0 + e) / (1.0 - e)).ln();
        close(oblate.wetted_area_m2, area, tol, "oblate area");

        // Power series: V = πR²L/(2n+1), x̄ = L(2n+1)/(2n+2), planform 2RL/(n+1) at L(n+1)/(n+2);
        // the paraboloid (n = ½) has area πR/(6L²) ((R² + 4L²)^{3/2} − R³).
        for n in [0.3, 0.5, 0.75, 1.0] {
            let g = solid(NoseShape::PowerSeries { exponent: n });
            close(
                g.volume_m3,
                PI * r * r * l / (2.0 * n + 1.0),
                tol,
                "power volume",
            );
            close(
                g.centroid_m,
                l * (2.0 * n + 1.0) / (2.0 * n + 2.0),
                tol,
                "power centroid",
            );
            close(
                g.planform_area_m2,
                2.0 * r * l / (n + 1.0),
                tol,
                "power planform",
            );
            close(
                g.planform_centroid_m,
                l * (n + 1.0) / (n + 2.0),
                tol,
                "power planform centroid",
            );
        }
        let g = solid(NoseShape::PowerSeries { exponent: 0.5 });
        let area = PI * r / (6.0 * l * l) * ((r * r + 4.0 * l * l).powf(1.5) - r.powi(3));
        close(g.wetted_area_m2, area, tol, "paraboloid area");

        // Parabolic series: y = R(2ξ − Kξ²)/(2 − K).
        for k in [0.0, 0.5, 0.75, 1.0] {
            let g = solid(NoseShape::ParabolicSeries { parameter: k });
            let v = 4.0 / 3.0 - k + k * k / 5.0;
            let m = 1.0 - 0.8 * k + k * k / 6.0;
            close(
                g.volume_m3,
                PI * r * r * l * v / (2.0 - k).powi(2),
                tol,
                "parabolic volume",
            );
            close(g.centroid_m, l * m / v, tol, "parabolic centroid");
            close(
                g.planform_area_m2,
                2.0 * r * l * (1.0 - k / 3.0) / (2.0 - k),
                tol,
                "parabolic planform",
            );
            close(
                g.planform_centroid_m,
                l * (2.0 / 3.0 - k / 4.0) / (1.0 - k / 3.0),
                tol,
                "parabolic planform centroid",
            );
        }

        // Haack series: V = πR²L(1/2 + 3C/16) and, integrating in θ, x̄ = L(11 + 3C)/(2(8 + 3C)).
        for c in [0.0, 1.0 / 3.0, 2.0 / 3.0] {
            let g = solid(NoseShape::Haack { parameter: c });
            close(
                g.volume_m3,
                PI * r * r * l * (0.5 + 3.0 * c / 16.0),
                tol,
                "Haack volume",
            );
            close(
                g.centroid_m,
                l * (11.0 + 3.0 * c) / (2.0 * (8.0 + 3.0 * c)),
                tol,
                "Haack centroid",
            );
        }
    }

    #[test]
    fn tips_stay_exact_where_the_formulas_cancel() {
        // θ − sin 2θ/2 by series agrees with the direct form where both are accurate, and with
        // (2/3)θ³ − (2/15)θ⁵ at small θ.
        for theta in [0.1f64, 0.1 - 1e-12, 0.3] {
            let direct = theta - (2.0 * theta).sin() / 2.0;
            assert!(
                (haack_core(theta) - direct).abs() <= 1e-13 * direct,
                "{theta}"
            );
        }
        let theta: f64 = 1e-3;
        let series =
            2.0 / 3.0 * theta.powi(3) - 2.0 / 15.0 * theta.powi(5) + 4.0 / 315.0 * theta.powi(7);
        assert!((haack_core(theta) - series).abs() <= 1e-15 * series);
        // A Haack transition's slope at its small end is infinite, never NaN.
        let t = Profile::transition(NoseShape::VON_KARMAN, 0.02, 0.0508, 0.0785, false).unwrap();
        assert_eq!(t.radius_and_slope(0.0), (0.0508, f64::INFINITY));
        let nose = Profile::nose(NoseShape::VON_KARMAN, 0.3, 0.05).unwrap();
        assert_eq!(nose.radius_and_slope(0.0), (0.0, f64::INFINITY));
        for xi in [1e-18, 1e-12, 1e-6] {
            let (r, slope) = nose.radius_and_slope(0.3 * xi);
            assert!(r > 0.0 && slope.is_finite() && slope > 0.0, "{xi}");
        }
        // A tangent-ogive transition 500 times as long as its radius step still reaches both
        // radii, and its volume lies between the two cylinders'.
        let slender =
            Profile::transition(NoseShape::TANGENT_OGIVE, 0.05, 0.025, 0.0251, false).unwrap();
        assert!((slender.radius_m(0.0) - 0.025).abs() < 1e-15);
        assert!((slender.radius_m(0.05) - 0.0251).abs() < 1e-15);
        let g = crate::solids::revolve(&slender, crate::solids::Wall::Filled {}).unwrap();
        let (lo, hi) = (PI * 0.025f64.powi(2) * 0.05, PI * 0.0251f64.powi(2) * 0.05);
        assert!(g.volume_m3 > lo && g.volume_m3 < hi);
        // The boundary ogive, whose arc centre sits on the axis, is valid and ends at R.
        let edge = Profile::nose(NoseShape::Ogive { radius_ratio: 0.2 }, 0.25, 0.05).unwrap();
        assert!((edge.radius_m(0.25) - 0.05).abs() < 1e-15);
        assert!(edge.radius_m(0.0).abs() < 1e-15);
        // Clipped bulged ogives are rejected.
        assert!(matches!(
            Profile::transition(
                NoseShape::Ogive { radius_ratio: 0.7 },
                0.05,
                0.02,
                0.025,
                true
            ),
            Err(DesignError::Geometry(_))
        ));
    }

    #[test]
    fn unknown_shape_fields_are_rejected() {
        for bad in [
            r#"{"kind":"conical","radius_ratio":2.0}"#,
            r#"{"kind":"elliptical","parameter":0.5}"#,
            r#"{"kind":"haack","parameter":0.0,"clipped":true}"#,
        ] {
            assert!(serde_json::from_str::<NoseShape>(bad).is_err(), "{bad}");
        }
        assert_eq!(
            serde_json::from_str::<NoseShape>(r#"{"kind":"conical"}"#).unwrap(),
            NoseShape::Conical {}
        );
        assert!(
            serde_json::from_str::<crate::solids::Wall>(r#"{"kind":"filled","thickness_m":0.002}"#)
                .is_err()
        );
    }

    #[test]
    fn profiles_round_trip_through_serde_and_reject_bad_data() {
        let t = Profile::transition(NoseShape::LV_HAACK, 0.12, 0.04, 0.02, true).unwrap();
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(
            json,
            r#"{"shape":{"kind":"haack","parameter":0.3333333333333333},"length_m":0.12,"fore_radius_m":0.04,"aft_radius_m":0.02,"clipped":true}"#
        );
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, t);
        let bad =
            r#"{"shape":{"kind":"conical"},"length_m":-1,"fore_radius_m":0,"aft_radius_m":0.02}"#;
        assert!(serde_json::from_str::<Profile>(bad).is_err());
        assert!(Profile::transition(NoseShape::Conical {}, 0.1, 0.0, 0.0, false).is_err());
        let cylinder =
            Profile::transition(NoseShape::Elliptical {}, 0.1, 0.02, 0.02, true).unwrap();
        assert_eq!(cylinder.radius_and_slope(0.05), (0.02, 0.0));
    }
}
