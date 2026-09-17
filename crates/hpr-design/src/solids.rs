//! Solids of revolution: volume, centroid, moments of inertia and surface areas of a profile swept
//! about its axis, either filled or as a wall of constant thickness normal to the outer surface.
//!
//! **Integrals.** With outer radius `y(x)`, inner radius `r_i(x)` (zero when filled) and `x`
//! measured aft of the forward end over `[0, L]`, per unit density:
//!
//! ```text
//! V    = π ∫ (y² − r_i²) dx                     volume
//! x̄    = π ∫ x (y² − r_i²) dx / V               centroid, aft of the forward end
//! J_a  = (π/2) ∫ (y⁴ − r_i⁴) dx                 moment about the axis
//! J_t  = π ∫ [(y⁴ − r_i⁴)/4 + x² (y² − r_i²)] dx − V x̄²    moment about a transverse axis
//!                                                          through the centroid
//! S    = 2π ∫ y √(1 + y′²) dx                   outer (wetted) area, ends excluded
//! A_p  = 2 ∫ y dx,   x_p = ∫ x y dx / ∫ y dx    planform (side-view) area and its centroid
//! ```
//!
//! Each slice is a thin annulus of radii `r_i < y` and thickness `dx`, with `dI_a = (π/2)(y⁴ −
//! r_i⁴) dx` about the axis and `dI_t = (π/4)(y⁴ − r_i⁴) dx` about its own diameter (Meriam and
//! Kraige, appendix B), moved to the reference plane by the parallel-axis theorem.
//!
//! **Walls.** A wall of thickness `t` is the set of points of the solid within `t` of the outer
//! surface, so the thickness is measured normal to the surface, as a molded or laid-up shell is
//! made. Its inner radius at station `x` is the lower envelope of circles of radius `t` centred on
//! the profile:
//!
//! ```text
//! r_i(x) = max(0, min_{|s − x| ≤ t} [ y(s) − √(t² − (x − s)²) ])
//! ```
//!
//! A point below the profile is at least `t` from every surface point exactly when it lies below
//! all those circles: if it lay above the lower half of the circle about some surface point, the
//! continuous profile would cross the point's height closer than `t`, so the point would be in the
//! wall anyway. The minimum is taken over the profile extended past each cut end along its end
//! tangent, so the wall is cut square by the end planes; past an end whose tangent is vertical
//! there is no extension, and the end point itself is a candidate. ADR-006 records this choice
//! against the radial-thickness alternative. The integration is split where `r_i` reaches zero and
//! at `t` from each end, where the end points' circles enter the envelope.
//!
//! The integrals use [`hpr_core::quadrature::integrate`] on integrands scaled to order one.
//! See `docs/physics/mass.md`.

use std::f64::consts::PI;

use hpr_core::quadrature::{Tolerance, integrate};
use serde::{Deserialize, Serialize};

use crate::error::DesignError;
use crate::shapes::{Profile, check_dimension};

/// Whether a solid of revolution is filled or a wall.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Wall {
    /// Solid all the way to the axis.
    Filled {},
    /// A wall of constant thickness measured normal to the outer surface.
    Shell {
        /// Wall thickness, m.
        thickness_m: f64,
    },
}

/// Volume, centroid, moments and areas of a solid of revolution per unit density; multiply the
/// volume and moments by a density to get mass and inertia.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RevolvedGeometry {
    /// Volume, m³.
    pub volume_m3: f64,
    /// Centroid of the volume, m aft of the forward end.
    pub centroid_m: f64,
    /// Moment of inertia about the axis per unit density, `∫ r² dV`, m⁵.
    pub axial_m5: f64,
    /// Moment of inertia about a transverse axis through the centroid per unit density, m⁵.
    pub transverse_m5: f64,
    /// Outer surface area, ends excluded, m².
    pub wetted_area_m2: f64,
    /// Side-view (planform) area, m².
    pub planform_area_m2: f64,
    /// Centroid of the planform area, m aft of the forward end.
    pub planform_centroid_m: f64,
}

/// Quadrature settings for smooth profile integrands.
const SMOOTH: Tolerance = Tolerance {
    relative: 1e-12,
    absolute: 1e-14,
    max_intervals: 4000,
};

/// Quadrature settings for wall integrands, whose inner radius comes from a numerical minimum.
const WALL: Tolerance = Tolerance {
    relative: 1e-11,
    absolute: 1e-13,
    max_intervals: 4000,
};

/// Computes the volume, moments and areas of `profile` swept about its axis with `wall`.
///
/// # Errors
///
/// - [`DesignError::Domain`] for a non-positive or non-finite wall thickness.
/// - [`DesignError::Numerics`] if an integral doesn't converge.
pub fn revolve(profile: &Profile, wall: Wall) -> Result<RevolvedGeometry, DesignError> {
    let length = profile.length_m();
    let scale = profile.max_radius_m();
    let thickness = match wall {
        Wall::Filled {} => None,
        Wall::Shell { thickness_m } => {
            check_dimension("wall thickness", thickness_m, false)?;
            Some(thickness_m)
        }
    };
    // Each half of the profile is integrated from its own end, in `u = s²` with `u` the
    // normalized distance from that end: the substitution smooths the `u^(−1/2)` singularity of a
    // blunt tip's surface integrand, and measuring from the end keeps the tip exact.
    let halves = |f: &dyn Fn(f64, f64, f64) -> [f64; 5]| -> Result<[f64; 5], DesignError> {
        let mut total = [0.0; 5];
        for from_aft in [false, true] {
            let piece = integrate(
                |s| {
                    let u = s * s;
                    let (r, slope) = profile.at_distance(u * length, from_aft);
                    let x = if from_aft { 1.0 - u } else { u };
                    f(x, r / scale, slope).map(|v| 2.0 * s * v)
                },
                0.0,
                std::f64::consts::FRAC_1_SQRT_2,
                SMOOTH,
            )?;
            for (sum, value) in total.iter_mut().zip(piece.value) {
                *sum += value;
            }
        }
        Ok(total)
    };

    // Surfaces, and the filled solid's moments: [∫Y², ∫XY², ∫Y⁴, ∫X²Y²] over X.
    let [wetted, planform, planform_moment, _, _] = halves(&|x, y, slope| {
        // `hypot` keeps a very blunt tip's `y y′` from overflowing when squared.
        let wetted = if y == 0.0 { 0.0 } else { y.hypot(y * slope) };
        [wetted, y, x * y, 0.0, 0.0]
    })?;
    let [a, b, c, d, _] = halves(&|x, y, _| {
        let y2 = y * y;
        [y2, x * y2, y2 * y2, x * x * y2, 0.0]
    })?;
    let filled = [a, b, c, d];

    // A wall subtracts the hollow's moments: [∫Yi², ∫XYi², ∫Yi⁴, ∫X²Yi²].
    let moments = match thickness {
        None => filled,
        Some(t) => {
            let inner = |x: f64| inner_radius(profile, x * length, t) / scale;
            let integrand = |x: f64| {
                let yi2 = inner(x).powi(2);
                [yi2, x * yi2, yi2 * yi2, x * x * yi2]
            };
            let mut hollow = [0.0; 4];
            // Split where the inner radius fills in, and where the circles about the end points
            // enter the envelope (`t` from each end), which is a kink.
            let mut breaks = filling_points(&inner);
            let edge = t / length;
            breaks.extend(
                [edge, 1.0 - edge]
                    .into_iter()
                    .filter(|b| *b > 0.0 && *b < 1.0),
            );
            breaks.sort_by(f64::total_cmp);
            breaks.dedup();
            for pair in breaks.windows(2) {
                // Every piece is integrated, even one that looks filled in at its middle, so a
                // thin hollow the crossing grid misses still counts.
                let piece = integrate(integrand, pair[0], pair[1], WALL)?;
                for (sum, value) in hollow.iter_mut().zip(piece.value) {
                    *sum += value;
                }
            }
            std::array::from_fn(|k| filled[k] - hollow[k])
        }
    };
    let [area, first, fourth, second] = moments;
    let r2l = scale * scale * length;
    let volume = PI * r2l * area;
    let centroid = if area > 0.0 {
        length * first / area
    } else {
        0.5 * length
    };
    let axial = 0.5 * PI * r2l * scale * scale * fourth;
    let about_fore = PI * (0.25 * r2l * scale * scale * fourth + r2l * length * length * second);
    let transverse = about_fore - volume * centroid * centroid;
    // The subtraction may round a tiny moment below zero; anything larger is a numerical fault.
    if transverse < -1e-9 * about_fore {
        return Err(DesignError::Geometry(format!(
            "the transverse moment came out negative ({transverse:e} m⁵)"
        )));
    }
    Ok(RevolvedGeometry {
        volume_m3: volume,
        centroid_m: centroid,
        axial_m5: axial,
        transverse_m5: transverse.max(0.0),
        wetted_area_m2: 2.0 * PI * scale * length * wetted,
        planform_area_m2: 2.0 * scale * length * planform,
        planform_centroid_m: if planform > 0.0 {
            length * planform_moment / planform
        } else {
            0.5 * length
        },
    })
}

/// The inner radius of a wall of thickness `t` at station `x` (both m): the lower envelope of
/// circles of radius `t` centred on the profile, extended along its end tangents.
fn inner_radius(profile: &Profile, x: f64, t: f64) -> f64 {
    let length = profile.length_m();
    let (fore_r, fore_slope) = profile.radius_and_slope(0.0);
    let (aft_r, aft_slope) = profile.radius_and_slope(length);
    // The profile height at s, or None past an end whose tangent is vertical.
    let height = |s: f64| -> Option<f64> {
        if s < 0.0 {
            fore_slope.is_finite().then_some(fore_r + fore_slope * s)
        } else if s > length {
            aft_slope
                .is_finite()
                .then_some(aft_r + aft_slope * (s - length))
        } else {
            Some(profile.radius_m(s))
        }
    };
    let bound = |s: f64| -> f64 {
        let dx = x - s;
        match height(s) {
            Some(y) => y - (t * t - dx * dx).max(0.0).sqrt(),
            None => f64::INFINITY,
        }
    };
    // A coarse scan finds the basin, and golden-section search refines it.
    const SAMPLES: usize = 32;
    let step = 2.0 * t / SAMPLES as f64;
    let sample = |k: usize| x - t + step * (k as f64 + 0.5);
    let best = (0..SAMPLES)
        .min_by(|&i, &j| bound(sample(i)).total_cmp(&bound(sample(j))))
        .unwrap_or(0);
    let mut lo = (sample(best) - step).max(x - t);
    let mut hi = (sample(best) + step).min(x + t);
    let ratio = 0.5 * (5f64.sqrt() - 1.0);
    let mut a = hi - ratio * (hi - lo);
    let mut b = lo + ratio * (hi - lo);
    let (mut fa, mut fb) = (bound(a), bound(b));
    for _ in 0..80 {
        if fa <= fb {
            hi = b;
            b = a;
            fb = fa;
            a = hi - ratio * (hi - lo);
            fa = bound(a);
        } else {
            lo = a;
            a = b;
            fa = fb;
            b = lo + ratio * (hi - lo);
            fb = bound(b);
        }
        // Near the minimum the bound is quadratic in s, with curvature of order 1/t, so locating s
        // to 1e-9 t fixes the value to about 1e-18 t.
        if hi - lo <= 1e-9 * t {
            break;
        }
    }
    let mut minimum = fa.min(fb).min(bound(sample(best)));
    // Past an end with a vertical tangent the profile stops, so the minimum can sit exactly on the
    // end, where the search above only approaches it from inside.
    for end in [0.0, length] {
        if (x - end).abs() <= t {
            minimum = minimum.min(bound(end));
        }
    }
    minimum.max(0.0)
}

/// Normalized stations bounding the pieces between which the wall's inner radius is either zero
/// or positive: 0, every crossing (to near machine precision), and 1.
fn filling_points(inner: &impl Fn(f64) -> f64) -> Vec<f64> {
    const GRID: usize = 64;
    let mut points = vec![0.0];
    let hollow = |x: f64| inner(x) > 0.0;
    let mut previous = hollow(0.0);
    for k in 1..=GRID {
        let x = k as f64 / GRID as f64;
        let now = hollow(x);
        if now != previous {
            let (mut lo, mut hi) = ((k - 1) as f64 / GRID as f64, x);
            for _ in 0..60 {
                let mid = 0.5 * (lo + hi);
                if hollow(mid) == previous {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            points.push(0.5 * (lo + hi));
        }
        previous = now;
    }
    points.push(1.0);
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shapes::NoseShape;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(err <= rel, "{what}: {got} vs {want} (relative {err:e})");
    }

    #[derive(serde::Deserialize)]
    struct Fixture {
        cases: Vec<Case>,
    }

    #[derive(serde::Deserialize)]
    struct Case {
        input: serde_json::Value,
        expected: RevolvedGeometry,
    }

    /// Every shape family, as noses and as transitions both ways and clipped, against mpmath's
    /// 40-digit integrals of the defining formulas (`validation/oracles/design/shapes.py`).
    #[test]
    fn filled_solids_match_the_mpmath_references() {
        let text = include_str!("../../../validation/fixtures/design/shape-integrals.json");
        let fixture: Fixture = serde_json::from_str(text).unwrap();
        assert_eq!(fixture.cases.len(), 22);
        for case in fixture.cases {
            let profile = fixture_profile(&case.input);
            let got = revolve(&profile, Wall::Filled {}).unwrap();
            let want = case.expected;
            let label = case.input.to_string();
            close(
                got.volume_m3,
                want.volume_m3,
                1e-12,
                &format!("volume {label}"),
            );
            close(
                got.centroid_m,
                want.centroid_m,
                1e-12,
                &format!("centroid {label}"),
            );
            close(
                got.axial_m5,
                want.axial_m5,
                1e-12,
                &format!("axial {label}"),
            );
            close(
                got.transverse_m5,
                want.transverse_m5,
                1e-12,
                &format!("transverse {label}"),
            );
            close(
                got.wetted_area_m2,
                want.wetted_area_m2,
                1e-12,
                &format!("wetted {label}"),
            );
            close(
                got.planform_area_m2,
                want.planform_area_m2,
                1e-12,
                &format!("planform {label}"),
            );
            close(
                got.planform_centroid_m,
                want.planform_centroid_m,
                1e-12,
                &format!("planform centroid {label}"),
            );
        }
    }

    #[derive(serde::Deserialize)]
    struct WallFixture {
        cases: Vec<WallCase>,
    }

    #[derive(serde::Deserialize)]
    struct WallCase {
        input: serde_json::Value,
        wall_thickness_m: f64,
        expected: WallExpected,
    }

    #[derive(serde::Deserialize)]
    struct WallExpected {
        volume_m3: f64,
        centroid_m: f64,
        axial_m5: f64,
        transverse_m5: f64,
    }

    /// A fixture case's input as a profile (the fixture names the shape parameter `parameter`).
    fn fixture_profile(input: &serde_json::Value) -> Profile {
        let mut input = input.clone();
        let object = input.as_object_mut().unwrap();
        let kind = object.remove("shape").unwrap();
        let mut shape = serde_json::json!({ "kind": kind });
        if let Some(parameter) = object.remove("parameter") {
            let key = match kind.as_str().unwrap() {
                "ogive" => "radius_ratio",
                "power_series" => "exponent",
                _ => "parameter",
            };
            shape[key] = parameter;
        }
        object.insert("shape".to_owned(), shape);
        serde_json::from_value(input).unwrap()
    }

    /// Walls of every shape family, as noses and as transitions both ways, clipped and not (blunt
    /// unclipped ends included), against `validation/oracles/design/walls.py`, which finds the
    /// envelope from the roots of its derivative at 25 digits.
    #[test]
    fn walls_match_the_mpmath_references() {
        let text = include_str!("../../../validation/fixtures/design/wall-integrals.json");
        let fixture: WallFixture = serde_json::from_str(text).unwrap();
        assert_eq!(fixture.cases.len(), 20);
        for case in fixture.cases {
            let profile = fixture_profile(&case.input);
            let wall = Wall::Shell {
                thickness_m: case.wall_thickness_m,
            };
            let got = revolve(&profile, wall).unwrap();
            let want = case.expected;
            let label = format!("{} t = {}", case.input, case.wall_thickness_m);
            close(
                got.volume_m3,
                want.volume_m3,
                1e-9,
                &format!("volume {label}"),
            );
            close(
                got.centroid_m,
                want.centroid_m,
                1e-9,
                &format!("centroid {label}"),
            );
            close(got.axial_m5, want.axial_m5, 1e-9, &format!("axial {label}"));
            close(
                got.transverse_m5,
                want.transverse_m5,
                1e-9,
                &format!("transverse {label}"),
            );
        }
    }

    #[test]
    fn extreme_parameters_stay_accurate_or_fail_loudly() {
        // A huge ogive radius is a cone.
        let cone = revolve(
            &Profile::nose(NoseShape::Conical {}, 0.3, 0.05).unwrap(),
            Wall::Filled {},
        )
        .unwrap();
        for ratio in [1e6, 1e12] {
            let profile = Profile::nose(
                NoseShape::Ogive {
                    radius_ratio: ratio,
                },
                0.3,
                0.05,
            )
            .unwrap();
            assert!(
                (profile.radius_m(0.001) - 0.05 / 300.0).abs() < 1e-9,
                "{ratio}"
            );
            let g = revolve(&profile, Wall::Filled {}).unwrap();
            close(g.volume_m3, cone.volume_m3, 1e-5, &format!("ogive {ratio}"));
        }
        // A very blunt power series still integrates.
        let n = 0.02;
        let g = revolve(
            &Profile::nose(NoseShape::PowerSeries { exponent: n }, 0.2, 0.05).unwrap(),
            Wall::Filled {},
        )
        .unwrap();
        close(
            g.volume_m3,
            PI * 0.05 * 0.05 * 0.2 / (2.0 * n + 1.0),
            1e-10,
            "blunt power series",
        );
        assert!(g.wetted_area_m2.is_finite());
    }

    #[test]
    fn a_tube_matches_the_hollow_cylinder_formulas() {
        let (l, r, t) = (0.5, 0.05, 0.002);
        let profile = Profile::transition(NoseShape::Conical {}, l, r, r, false).unwrap();
        let g = revolve(&profile, Wall::Shell { thickness_m: t }).unwrap();
        let ri = r - t;
        let v = PI * (r * r - ri * ri) * l;
        close(g.volume_m3, v, 1e-12, "volume");
        close(g.centroid_m, l / 2.0, 1e-12, "centroid");
        close(g.axial_m5, 0.5 * v * (r * r + ri * ri), 1e-11, "axial");
        close(
            g.transverse_m5,
            v * ((r * r + ri * ri) / 4.0 + l * l / 12.0),
            1e-11,
            "transverse",
        );
        close(g.wetted_area_m2, 2.0 * PI * r * l, 1e-12, "wetted");
        close(g.planform_area_m2, 2.0 * r * l, 1e-12, "planform");
    }

    #[test]
    fn a_conical_wall_is_the_cone_minus_its_offset_cone() {
        // The inner surface of a cone with normal wall t is the same cone moved aft by t / sin β,
        // with β the half-angle: r_i = k (x − x0), k = R/L, x0 = t √(1 + k²) / k.
        let (l, r, t): (f64, f64, f64) = (0.3, 0.05, 0.003);
        let k = r / l;
        let x0 = t * (1.0 + k * k).sqrt() / k;
        let profile = Profile::nose(NoseShape::Conical {}, l, r).unwrap();
        let g = revolve(&profile, Wall::Shell { thickness_m: t }).unwrap();
        // Integrals of the outer cone and the hollow cone over [x0, L], by hand.
        let u = l - x0;
        let v_out = PI * r * r * l / 3.0;
        let v_in = PI * k * k * u.powi(3) / 3.0;
        let m_out = PI * k * k * l.powi(4) / 4.0;
        let m_in = PI * k * k * (u.powi(4) / 4.0 + x0 * u.powi(3) / 3.0);
        let volume = v_out - v_in;
        close(g.volume_m3, volume, 1e-10, "volume");
        close(g.centroid_m, (m_out - m_in) / volume, 1e-10, "centroid");
        // ∫ y⁴ dx over the cones.
        let a_out = 0.5 * PI * k.powi(4) * l.powi(5) / 5.0;
        let a_in = 0.5 * PI * k.powi(4) * u.powi(5) / 5.0;
        close(g.axial_m5, a_out - a_in, 1e-10, "axial");
        // ∫ x² y² dx about the tip plane.
        let s_out = PI * k * k * l.powi(5) / 5.0;
        let s_in =
            PI * k * k * (u.powi(5) / 5.0 + 2.0 * x0 * u.powi(4) / 4.0 + x0 * x0 * u.powi(3) / 3.0);
        let transverse =
            0.5 * (a_out - a_in) + (s_out - s_in) - volume * ((m_out - m_in) / volume).powi(2);
        close(g.transverse_m5, transverse, 1e-9, "transverse");
        close(
            g.wetted_area_m2,
            PI * r * (r * r + l * l).sqrt(),
            1e-12,
            "wetted",
        );
    }

    #[test]
    fn a_tangent_ogive_wall_is_bounded_by_the_concentric_arc() {
        // The inner surface is the arc of radius ρ − t about the same centre (L, R − ρ).
        let (l, r, t): (f64, f64, f64) = (0.25, 0.04, 0.002);
        let rho = (r * r + l * l) / (2.0 * r);
        let (a, c) = (rho - t, r - rho);
        let u0 = (a * a - c * c).sqrt();
        // ∫_0^{u0} (√(a² − u²) + c)² du, with u = L − x.
        let hollow = PI
            * ((a * a + c * c) * u0 - u0.powi(3) / 3.0
                + c * (u0 * (a * a - u0 * u0).sqrt() + a * a * (u0 / a).asin()));
        let outer =
            PI * (l * rho * rho - l.powi(3) / 3.0 - (rho - r) * rho * rho * (l / rho).asin());
        let profile = Profile::nose(NoseShape::TANGENT_OGIVE, l, r).unwrap();
        let wall = revolve(&profile, Wall::Shell { thickness_m: t }).unwrap();
        let filled = revolve(&profile, Wall::Filled {}).unwrap();
        close(filled.volume_m3, outer, 1e-12, "filled volume");
        close(wall.volume_m3, outer - hollow, 1e-10, "wall volume");
    }

    #[test]
    fn a_thick_wall_fills_the_solid_and_a_thin_one_is_area_times_thickness() {
        let profile = Profile::nose(NoseShape::VON_KARMAN, 0.3, 0.05).unwrap();
        let filled = revolve(&profile, Wall::Filled {}).unwrap();
        let thick = revolve(&profile, Wall::Shell { thickness_m: 0.2 }).unwrap();
        close(thick.volume_m3, filled.volume_m3, 1e-12, "thick wall");
        close(
            thick.transverse_m5,
            filled.transverse_m5,
            1e-12,
            "thick wall inertia",
        );
        // A thin wall's volume approaches S t (1 − t κ̄/2...) with an error of order t².
        let t = 1e-5;
        let thin = revolve(&profile, Wall::Shell { thickness_m: t }).unwrap();
        close(thin.volume_m3, filled.wetted_area_m2 * t, 2e-3, "thin wall");
        assert!(revolve(&profile, Wall::Shell { thickness_m: -1.0 }).is_err());
    }
}
