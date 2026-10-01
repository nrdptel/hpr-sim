//! Geodesics on an ellipsoid: the distance and bearing between two places, and the place a given
//! distance away along a bearing.
//!
//! A geodesic is the shortest path between two points on the ellipsoid's surface. The two
//! problems are Karney's:
//!
//! - **inverse** ([`Ellipsoid::geodesic_inverse`]): given two points, the length `s₁₂` of the
//!   geodesic between them and its azimuths `α₁` (leaving point 1) and `α₂` (arriving at point 2);
//! - **direct** ([`Ellipsoid::geodesic_direct`]): given a point, an azimuth `α₁` and a distance
//!   `s₁₂`, the end point and `α₂`.
//!
//! The algorithms are C. F. F. Karney, *Algorithms for geodesics*, J. Geodesy 87 (2013) 43–55,
//! doi:10.1007/s00190-012-0578-z (arXiv:1109.4448v2). The ellipsoid is mapped onto an auxiliary
//! sphere, where a geodesic is a great circle, and distance and longitude are corrected by series
//! in the third flattening `n = f/(2 − f)` to sixth order in the flattening `f` (§2, the direct
//! problem); the inverse finds `α₁` by Newton's method (§4), from a starting guess (§5). The code
//! is GeographicLib's, as georust's `geographiclib-rs` ports it (MIT). Karney states that round-off
//! in both problems stays under 15 nanometres on WGS 84 (§7, page 10), and that for `f` up to
//! 1/150 the series' truncation is smaller than round-off (page 9): past that the series lose
//! accuracy, so a flatter ellipsoid is refused ([`GEODESIC_MAX_FLATTENING`]). Karney's published
//! test set of 500,000 WGS 84 geodesics (doi:10.5281/zenodo.32156) is the check:
//! `tests/geodtest.rs`, with the measured errors in `docs/physics/geodesy.md`.
//!
//! Azimuths are clockwise from north, in radians, in `[−π, π]`; `.rem_euclid(TAU)` gives the
//! 0 to 2π of a compass. `α₂` is the direction of travel at point 2, so the bearing back to point 1
//! from there is `α₂ ± π`. Heights play no part: the path lies on the ellipsoid's surface, not at
//! the points' heights.

use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use geographiclib_rs::{DirectGeodesic, Geodesic as Solver, InverseGeodesic};

use crate::error::CoreError;
use crate::geodesy::{Ellipsoid, Geodetic};

/// The largest flattening the geodesics accept, 1/150: Karney 2013 (page 9) shows the sixth-order
/// series' truncation below `f64` round-off up to there. GeographicLib's own error table for the
/// series grows to 10 µm at `f` = 0.05 and 0.3 m at 0.2. Every planet-like body hpr flies on is
/// inside: WGS 84's `f` is 1/298.257.
pub const GEODESIC_MAX_FLATTENING: f64 = 1.0 / 150.0;

/// The geodesic between two points: Karney's inverse problem.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GeodesicInverse {
    /// The geodesic's length `s₁₂` on the ellipsoid's surface, m.
    pub distance_m: f64,
    /// Azimuth `α₁` leaving point 1, rad, clockwise from north, in `[−π, π]`: the bearing from
    /// point 1 to point 2.
    pub initial_azimuth_rad: f64,
    /// Azimuth `α₂` arriving at point 2, rad, clockwise from north, in `[−π, π]`: the direction of
    /// travel there, not the bearing back to point 1 (that is `α₂ ± π`).
    pub final_azimuth_rad: f64,
}

/// The end of a geodesic from a start, an azimuth and a distance: Karney's direct problem.
///
/// The end has no height: the geodesic lies on the ellipsoid's surface. [`GeodesicDirect::end`]
/// makes a position of it at a height the caller chooses.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct GeodesicDirect {
    /// The end's geodetic latitude, rad, in `[−π/2, π/2]`.
    pub latitude_rad: f64,
    /// The end's longitude, rad, in `[−π, π]`.
    pub longitude_rad: f64,
    /// Azimuth `α₂` arriving at the end, rad, clockwise from north, in `[−π, π]`.
    pub final_azimuth_rad: f64,
}

impl GeodesicDirect {
    /// The end as a position at ellipsoidal height `height_m`.
    ///
    /// # Errors
    ///
    /// As [`Geodetic::new`]: [`CoreError::Domain`] if the height is not finite.
    pub fn end(&self, height_m: f64) -> Result<Geodetic, CoreError> {
        Geodetic::new(self.latitude_rad, self.longitude_rad, height_m)
    }
}

/// An angle in degrees for the solver. One outside `[−π, π]` is first reduced modulo 2π, so that a
/// longitude of many turns, which [`Geodetic::new`] accepts, can't overflow to infinity.
fn degrees(angle_rad: f64) -> f64 {
    if angle_rad.abs() <= PI {
        angle_rad.to_degrees()
    } else {
        (angle_rad % TAU).to_degrees()
    }
}

impl Ellipsoid {
    fn geodesic_solver(&self) -> Result<Solver, CoreError> {
        if self.flattening() > GEODESIC_MAX_FLATTENING {
            return Err(CoreError::Domain {
                what: "flattening for geodesics (at most 1/150)",
                value: self.flattening(),
            });
        }
        Ok(Solver::new(self.semi_major_axis_m(), self.flattening()))
    }

    /// The geodesic from `from` to `to` (Karney 2013 §4, §5): its length and the azimuths at both
    /// ends. Heights are ignored.
    ///
    /// Two coincident points give a zero distance. Where the points are nearly antipodal, or one
    /// is near a pole, the azimuths are ill-conditioned: a tiny move of a point turns them through
    /// large angles while the distance hardly changes. And the shortest path is not always
    /// unique: when `φ₂ = −φ₁` exactly, two geodesics of the same length join the points (unless
    /// `α₁ = α₂`), the second with `α₁` and `α₂` swapped (GeographicLib's `GeodSolve` manual,
    /// *Multiple solutions*); either may be returned.
    ///
    /// ```
    /// use hpr_core::geodesy::{Ellipsoid, Geodetic};
    ///
    /// // Two points a degree of longitude apart on the equator.
    /// let a = Geodetic::from_degrees(0.0, 0.0, 0.0)?;
    /// let b = Geodetic::from_degrees(0.0, 1.0, 0.0)?;
    /// let g = Ellipsoid::WGS84.geodesic_inverse(a, b)?;
    /// // An arc of the equator: a·(π/180).
    /// assert!((g.distance_m - 111_319.490_793_273_57).abs() < 1e-8);
    /// assert!((g.initial_azimuth_rad.to_degrees() - 90.0).abs() < 1e-12);
    /// # Ok::<(), hpr_core::CoreError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the ellipsoid's flattening is past [`GEODESIC_MAX_FLATTENING`],
    /// or either point fails [`Geodetic::validated`]'s checks.
    pub fn geodesic_inverse(
        &self,
        from: Geodetic,
        to: Geodetic,
    ) -> Result<GeodesicInverse, CoreError> {
        let solver = self.geodesic_solver()?;
        let (from, to) = (from.validated()?, to.validated()?);
        let (distance_m, azi1_deg, azi2_deg, _arc_deg) = solver.inverse(
            from.latitude_rad.to_degrees(),
            degrees(from.longitude_rad),
            to.latitude_rad.to_degrees(),
            degrees(to.longitude_rad),
        );
        Ok(GeodesicInverse {
            distance_m,
            initial_azimuth_rad: azi1_deg.to_radians(),
            final_azimuth_rad: azi2_deg.to_radians(),
        })
    }

    /// The point `distance_m` along the geodesic leaving `from` at azimuth `azimuth_rad`
    /// (clockwise from north), with the azimuth there (Karney 2013 §2). `from`'s height is
    /// ignored. A negative distance runs backwards along the same geodesic. Karney's accuracy is
    /// shown up to half a meridian (20,004 km); a distance of many circuits also carries its own
    /// rounding, one ulp of `distance_m` (15 nm at 67,000 km).
    ///
    /// ```
    /// use hpr_core::geodesy::{Ellipsoid, Geodetic};
    ///
    /// // A degree of longitude east along the equator, as above.
    /// let a = Geodetic::from_degrees(0.0, 0.0, 0.0)?;
    /// let d = Ellipsoid::WGS84.geodesic_direct(a, 90f64.to_radians(), 111_319.490_793_273_57)?;
    /// assert!((d.longitude_rad.to_degrees() - 1.0).abs() < 1e-12);
    /// assert!(d.latitude_rad.abs() < 1e-15);
    /// # Ok::<(), hpr_core::CoreError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the ellipsoid's flattening is past [`GEODESIC_MAX_FLATTENING`],
    /// `from` fails [`Geodetic::validated`]'s checks, or the azimuth or the distance is not
    /// finite.
    pub fn geodesic_direct(
        &self,
        from: Geodetic,
        azimuth_rad: f64,
        distance_m: f64,
    ) -> Result<GeodesicDirect, CoreError> {
        let solver = self.geodesic_solver()?;
        let from = from.validated()?;
        if !azimuth_rad.is_finite() {
            return Err(CoreError::Domain {
                what: "geodesic azimuth (rad)",
                value: azimuth_rad,
            });
        }
        if !distance_m.is_finite() {
            return Err(CoreError::Domain {
                what: "geodesic distance (m)",
                value: distance_m,
            });
        }
        let (lat2_deg, lon2_deg, azi2_deg) = solver.direct(
            from.latitude_rad.to_degrees(),
            degrees(from.longitude_rad),
            degrees(azimuth_rad),
            distance_m,
        );
        Ok(GeodesicDirect {
            latitude_rad: lat2_deg.to_radians(),
            longitude_rad: lon2_deg.to_radians(),
            final_azimuth_rad: azi2_deg.to_radians(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use proptest::prelude::*;

    use super::*;

    const WGS84: Ellipsoid = Ellipsoid::WGS84;

    fn point(latitude_deg: f64, longitude_deg: f64) -> Geodetic {
        Geodetic::from_degrees(latitude_deg, longitude_deg, 0.0).expect("a valid point")
    }

    #[test]
    fn on_a_sphere_the_distance_is_the_great_circle_arc() {
        // The central angle by the haversine formula, and the initial bearing by the spherical
        // azimuth formula `tan α₁ = sin Δλ cos φ₂ / (cos φ₁ sin φ₂ − sin φ₁ cos φ₂ cos Δλ)`.
        let sphere = Ellipsoid::new(6_371_000.0, f64::INFINITY).expect("a sphere");
        for (from, to) in [
            (point(10.0, 20.0), point(-35.0, 140.0)),
            (point(60.0, -5.0), point(61.0, 2.5)),
            (point(-80.0, 0.0), point(0.0, -170.0)),
        ] {
            let (p1, p2, dl) = (
                from.latitude_rad,
                to.latitude_rad,
                to.longitude_rad - from.longitude_rad,
            );
            let h =
                ((p2 - p1) / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
            let arc = 2.0 * h.sqrt().asin();
            let azimuth =
                (dl.sin() * p2.cos()).atan2(p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos());
            let g = sphere.geodesic_inverse(from, to).expect("a geodesic");
            assert!((g.distance_m - 6_371_000.0 * arc).abs() < 1e-8, "{g:?}");
            assert!((g.initial_azimuth_rad - azimuth).abs() < 1e-14, "{g:?}");
        }
    }

    /// The worked example in `docs/physics/geodesy.md`: a landing 1.2 km from the pad.
    #[test]
    fn the_guides_worked_example() {
        let pad = point(32.990_4, -106.975_0);
        let landing = point(33.000_0, -106.968_0);
        let g = WGS84.geodesic_inverse(pad, landing).expect("a geodesic");
        // The page's digits: 1,249.614 m, 31.567° out, 31.571° on arrival.
        assert!((g.distance_m - 1_249.614).abs() < 5e-4, "{g:?}");
        assert!(
            (g.initial_azimuth_rad.to_degrees() - 31.567).abs() < 5e-4,
            "{g:?}"
        );
        assert!(
            (g.final_azimuth_rad.to_degrees() - 31.571).abs() < 5e-4,
            "{g:?}"
        );
        // A flat map with the ellipsoid's radii of curvature at the mean latitude, the meridian's
        // `M = a(1 − e²)/w³` and the prime vertical's `N = a/w` (`w = √(1 − e² sin²φ)`), gives
        // the same distance to under a millimetre at this range.
        let mean = 0.5 * (pad.latitude_rad + landing.latitude_rad);
        let (a, e2) = (WGS84.semi_major_axis_m(), WGS84.eccentricity_squared());
        let w = (1.0 - e2 * mean.sin().powi(2)).sqrt();
        let north = a * (1.0 - e2) / w.powi(3) * (landing.latitude_rad - pad.latitude_rad);
        let east = a / w * mean.cos() * (landing.longitude_rad - pad.longitude_rad);
        assert!((north.hypot(east) - g.distance_m).abs() < 1e-3);
        // 2 km at 60°: 32.999415° N, 106.956466° W, heading 60.010°.
        let d = WGS84
            .geodesic_direct(pad, 60f64.to_radians(), 2_000.0)
            .expect("finite");
        assert!(
            (d.latitude_rad.to_degrees() - 32.999_415).abs() < 5e-7,
            "{d:?}"
        );
        assert!(
            (d.longitude_rad.to_degrees() + 106.956_466).abs() < 5e-7,
            "{d:?}"
        );
        assert!(
            (d.final_azimuth_rad.to_degrees() - 60.010).abs() < 5e-4,
            "{d:?}"
        );
    }

    #[test]
    fn coincident_points_are_zero_apart() {
        let p = point(32.99, -106.97);
        assert_eq!(
            WGS84.geodesic_inverse(p, p).expect("a geodesic").distance_m,
            0.0
        );
    }

    #[test]
    fn a_meridian_runs_due_north_at_both_ends() {
        let g = WGS84
            .geodesic_inverse(point(0.0, 10.0), point(45.0, 10.0))
            .expect("a geodesic");
        assert_eq!(g.initial_azimuth_rad, 0.0);
        assert_eq!(g.final_azimuth_rad, 0.0);
        let d = WGS84
            .geodesic_direct(point(0.0, 10.0), 0.0, g.distance_m)
            .expect("finite");
        assert!((d.latitude_rad.to_degrees() - 45.0).abs() < 1e-13);
    }

    #[test]
    fn heights_are_ignored() {
        let low = point(40.0, -100.0);
        let high = Geodetic::from_degrees(40.0, -100.0, 3_000.0).expect("a valid point");
        let to = point(41.0, -99.0);
        assert_eq!(
            WGS84.geodesic_inverse(low, to).expect("a geodesic"),
            WGS84.geodesic_inverse(high, to).expect("a geodesic")
        );
        let d = WGS84.geodesic_direct(high, 1.0, 5_000.0).expect("finite");
        assert_eq!(d.end(0.0).expect("a position").height_m, 0.0);
        assert_eq!(d, WGS84.geodesic_direct(low, 1.0, 5_000.0).expect("finite"));
    }

    #[test]
    fn a_negative_distance_runs_backwards() {
        let from = point(20.0, 30.0);
        let ahead = WGS84
            .geodesic_direct(from, 0.7, -250_000.0)
            .expect("finite");
        let behind = WGS84
            .geodesic_direct(from, 0.7 - PI, 250_000.0)
            .expect("finite");
        assert!((ahead.latitude_rad - behind.latitude_rad).abs() < 1e-14);
        assert!((ahead.longitude_rad - behind.longitude_rad).abs() < 1e-14);
    }

    #[test]
    fn refuses_flattenings_past_one_in_150() {
        let a = point(10.0, 20.0);
        let b = point(-35.0, 140.0);
        let edge = Ellipsoid::from_flattening(6.4e6, GEODESIC_MAX_FLATTENING).expect("valid");
        assert!(edge.geodesic_inverse(a, b).is_ok());
        let past = Ellipsoid::from_flattening(6.4e6, 0.2).expect("a valid ellipsoid");
        for result in [
            past.geodesic_inverse(a, b).map(|_| ()),
            past.geodesic_direct(a, 1.0, 1e6).map(|_| ()),
        ] {
            match result {
                Err(CoreError::Domain { what, value }) => {
                    assert_eq!(what, "flattening for geodesics (at most 1/150)");
                    assert_eq!(value, 0.2);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn refuses_points_built_out_of_range() {
        let bad = Geodetic {
            latitude_rad: 2.0,
            longitude_rad: 0.0,
            height_m: 0.0,
        };
        let good = point(0.0, 0.0);
        for result in [
            WGS84.geodesic_inverse(bad, good).map(|_| ()),
            WGS84.geodesic_inverse(good, bad).map(|_| ()),
            WGS84.geodesic_direct(bad, 1.0, 1.0).map(|_| ()),
        ] {
            match result {
                Err(CoreError::Domain { what, value }) => {
                    assert_eq!(what, "geodetic latitude (rad)");
                    assert_eq!(value, 2.0);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn a_longitude_of_many_turns_is_reduced() {
        // 1e307 rad is a valid `Geodetic` longitude whose degrees overflow `f64`.
        let far = Geodetic::new(0.5, 1e307, 0.0).expect("a valid point");
        let g = WGS84
            .geodesic_inverse(far, point(0.0, 0.0))
            .expect("a geodesic");
        assert!(g.distance_m.is_finite() && g.initial_azimuth_rad.is_finite());
        let d = WGS84.geodesic_direct(far, 1e300, 1_000.0).expect("finite");
        assert!(d.longitude_rad.is_finite() && d.final_azimuth_rad.is_finite());
        // Within a turn the reduction is not applied: 3π/2 east is π/2 west.
        let east = Geodetic::new(0.0, 1.5 * PI, 0.0).expect("a valid point");
        let west = Geodetic::new(0.0, -0.5 * PI, 0.0).expect("a valid point");
        let to = point(10.0, 0.0);
        let (ge, gw) = (
            WGS84.geodesic_inverse(east, to).expect("a geodesic"),
            WGS84.geodesic_inverse(west, to).expect("a geodesic"),
        );
        assert!((ge.distance_m - gw.distance_m).abs() < 1e-8);
    }

    #[test]
    fn results_round_trip_through_serde() {
        let g = WGS84
            .geodesic_inverse(point(1.0, 2.0), point(3.0, 4.0))
            .expect("a geodesic");
        let text = serde_json::to_string(&g).expect("serializes");
        assert_eq!(
            serde_json::from_str::<GeodesicInverse>(&text).expect("reads"),
            g
        );
        let d = WGS84
            .geodesic_direct(point(1.0, 2.0), 0.3, 5e5)
            .expect("finite");
        let text = serde_json::to_string(&d).expect("serializes");
        assert_eq!(
            serde_json::from_str::<GeodesicDirect>(&text).expect("reads"),
            d
        );
    }

    #[test]
    fn direct_refuses_non_finite_inputs() {
        let from = point(0.0, 0.0);
        for (azimuth, distance, what) in [
            (f64::NAN, 1.0, "geodesic azimuth (rad)"),
            (f64::INFINITY, 1.0, "geodesic azimuth (rad)"),
            (0.0, f64::NAN, "geodesic distance (m)"),
            (0.0, f64::NEG_INFINITY, "geodesic distance (m)"),
        ] {
            match WGS84.geodesic_direct(from, azimuth, distance) {
                Err(CoreError::Domain { what: w, .. }) => assert_eq!(w, what),
                other => panic!("{other:?}"),
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// The inverse's bearing and distance lead back to the second point, and the azimuths and
        /// the end's longitude stay in `[−π, π]`.
        #[test]
        fn inverse_then_direct_lands_on_the_second_point(
            lat1 in -FRAC_PI_2..=FRAC_PI_2,
            lon1 in -10.0..10.0f64,
            lat2 in -FRAC_PI_2..=FRAC_PI_2,
            lon2 in -10.0..10.0f64,
        ) {
            let from = Geodetic::new(lat1, lon1, 0.0).expect("a valid point");
            let to = Geodetic::new(lat2, lon2, 0.0).expect("a valid point");
            let g = WGS84.geodesic_inverse(from, to).expect("a geodesic");
            // No geodesic is longer than half a meridian, pole to pole: 20,003,931.4586 m.
            prop_assert!(g.distance_m >= 0.0 && g.distance_m <= 20_003_931.46);
            prop_assert!(g.initial_azimuth_rad.abs() <= PI && g.final_azimuth_rad.abs() <= PI);
            let d = WGS84
                .geodesic_direct(from, g.initial_azimuth_rad, g.distance_m)
                .expect("finite");
            prop_assert!(d.longitude_rad.abs() <= PI && d.final_azimuth_rad.abs() <= PI);
            let miss = WGS84.ecef_from_geodetic(d.end(0.0).expect("a position")) - WGS84.ecef_from_geodetic(to);
            prop_assert!(miss.length() < 1e-7, "missed by {} m", miss.length());
        }
    }
}
