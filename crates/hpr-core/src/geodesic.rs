//! Geodesics on an ellipsoid: the distance and bearing between two places, and the place a given
//! distance away along a bearing.
//!
//! A geodesic is the shortest path between two points on the ellipsoid's surface. The two
//! problems are Karney's:
//!
//! - **inverse:** given two points, the length `s₁₂` of the geodesic between them and its azimuths
//!   `α₁` (leaving point 1) and `α₂` (arriving at point 2);
//! - **direct:** given a point, an azimuth `α₁` and a distance `s₁₂`, the end point and `α₂`.
//!
//! The algorithms are C. F. F. Karney, *Algorithms for geodesics*, J. Geodesy 87 (2013) 43–55,
//! doi:10.1007/s00190-012-0578-z (arXiv:1109.4448v2): series in the third flattening `n` to
//! `O(f⁶)` (§3 to §5), with Newton's method on the auxiliary sphere for the inverse (§5). The code
//! is GeographicLib's, as georust's `geographiclib-rs` ports it (MIT). Karney states (§7, page 10)
//! that round-off in both problems stays under 15 nm on WGS 84, and that for `f` up to 1/150 the
//! series' truncation is below round-off. Karney's published test set of 500,000 WGS 84
//! geodesics (doi:10.5281/zenodo.32156) is the check: `tests/geodtest.rs`, and the measured errors
//! in `docs/physics/geodesy.md`.
//!
//! Azimuths are clockwise from north, in radians, in `[−π, π]`. `α₂` is the direction of travel at
//! point 2, so the bearing back to point 1 from there is `α₂ ± π`. Heights play no part: the path
//! lies on the ellipsoid's surface, not at the points' heights.

use serde::{Deserialize, Serialize};

use geographiclib_rs::{DirectGeodesic, Geodesic as Solver, InverseGeodesic};

use crate::error::CoreError;
use crate::geodesy::{Ellipsoid, Geodetic};

/// The geodesic between two points: Karney's inverse problem.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeodesicDirect {
    /// The end point. Its height is zero: the geodesic lies on the ellipsoid's surface. Its
    /// longitude is in `[−π, π]`.
    pub end: Geodetic,
    /// Azimuth `α₂` arriving at the end, rad, clockwise from north, in `[−π, π]`.
    pub final_azimuth_rad: f64,
}

impl Ellipsoid {
    fn geodesic_solver(&self) -> Solver {
        Solver::new(self.semi_major_axis_m(), self.flattening())
    }

    /// The geodesic from `from` to `to` (Karney 2013 §5): its length and the azimuths at both
    /// ends. Heights are ignored.
    ///
    /// Two coincident points give a zero distance. When the points are nearly antipodal, or one is
    /// at a pole, the shortest path's azimuths are ill-conditioned or not unique: a tiny move of
    /// a point turns them through large angles while the distance hardly changes.
    ///
    /// ```
    /// use hpr_core::geodesy::{Ellipsoid, Geodetic};
    ///
    /// // Two points a degree of longitude apart on the equator.
    /// let a = Geodetic::from_degrees(0.0, 0.0, 0.0)?;
    /// let b = Geodetic::from_degrees(0.0, 1.0, 0.0)?;
    /// let g = Ellipsoid::WGS84.geodesic_inverse(a, b);
    /// // An arc of the equator: a·(π/180).
    /// assert!((g.distance_m - 111_319.490_793_273_57).abs() < 1e-8);
    /// assert!((g.initial_azimuth_rad.to_degrees() - 90.0).abs() < 1e-12);
    /// # Ok::<(), hpr_core::CoreError>(())
    /// ```
    #[must_use]
    pub fn geodesic_inverse(&self, from: Geodetic, to: Geodetic) -> GeodesicInverse {
        let (distance_m, azi1_deg, azi2_deg, _arc_deg) = self.geodesic_solver().inverse(
            from.latitude_rad.to_degrees(),
            from.longitude_rad.to_degrees(),
            to.latitude_rad.to_degrees(),
            to.longitude_rad.to_degrees(),
        );
        GeodesicInverse {
            distance_m,
            initial_azimuth_rad: azi1_deg.to_radians(),
            final_azimuth_rad: azi2_deg.to_radians(),
        }
    }

    /// The point `distance_m` along the geodesic leaving `from` at azimuth `azimuth_rad`
    /// (clockwise from north), with the azimuth there (Karney 2013 §4). `from`'s height is
    /// ignored. A negative distance runs backwards along the same geodesic.
    ///
    /// ```
    /// use hpr_core::geodesy::{Ellipsoid, Geodetic};
    ///
    /// // A degree of longitude east along the equator, as above.
    /// let a = Geodetic::from_degrees(0.0, 0.0, 0.0)?;
    /// let d = Ellipsoid::WGS84.geodesic_direct(a, 90f64.to_radians(), 111_319.490_793_273_57)?;
    /// assert!((d.end.longitude_rad.to_degrees() - 1.0).abs() < 1e-12);
    /// assert!(d.end.latitude_rad.abs() < 1e-15);
    /// # Ok::<(), hpr_core::CoreError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the azimuth or the distance is not finite.
    pub fn geodesic_direct(
        &self,
        from: Geodetic,
        azimuth_rad: f64,
        distance_m: f64,
    ) -> Result<GeodesicDirect, CoreError> {
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
        let (lat2_deg, lon2_deg, azi2_deg) = self.geodesic_solver().direct(
            from.latitude_rad.to_degrees(),
            from.longitude_rad.to_degrees(),
            azimuth_rad.to_degrees(),
            distance_m,
        );
        let end = Geodetic::new(lat2_deg.to_radians(), lon2_deg.to_radians(), 0.0)?;
        Ok(GeodesicDirect {
            end,
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
            let g = sphere.geodesic_inverse(from, to);
            assert!((g.distance_m - 6_371_000.0 * arc).abs() < 1e-8, "{g:?}");
            assert!((g.initial_azimuth_rad - azimuth).abs() < 1e-14, "{g:?}");
        }
    }

    /// The worked example in `docs/physics/geodesy.md`: a landing 1.2 km from the pad.
    #[test]
    fn the_guides_worked_example() {
        let pad = point(32.990_4, -106.975_0);
        let landing = point(33.000_0, -106.968_0);
        let g = WGS84.geodesic_inverse(pad, landing);
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
        // 2 km at 60°: 32.999415° N, 106.956466° W, heading 60.010°.
        let d = WGS84
            .geodesic_direct(pad, 60f64.to_radians(), 2_000.0)
            .expect("finite");
        assert!(
            (d.end.latitude_rad.to_degrees() - 32.999_415).abs() < 5e-7,
            "{d:?}"
        );
        assert!(
            (d.end.longitude_rad.to_degrees() + 106.956_466).abs() < 5e-7,
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
        assert_eq!(WGS84.geodesic_inverse(p, p).distance_m, 0.0);
    }

    #[test]
    fn a_meridian_runs_due_north_at_both_ends() {
        let g = WGS84.geodesic_inverse(point(0.0, 10.0), point(45.0, 10.0));
        assert_eq!(g.initial_azimuth_rad, 0.0);
        assert_eq!(g.final_azimuth_rad, 0.0);
        let d = WGS84
            .geodesic_direct(point(0.0, 10.0), 0.0, g.distance_m)
            .expect("finite");
        assert!((d.end.latitude_rad.to_degrees() - 45.0).abs() < 1e-13);
    }

    #[test]
    fn the_end_is_on_the_ellipsoid_and_heights_are_ignored() {
        let low = point(40.0, -100.0);
        let high = Geodetic::from_degrees(40.0, -100.0, 3_000.0).expect("a valid point");
        let to = point(41.0, -99.0);
        assert_eq!(
            WGS84.geodesic_inverse(low, to),
            WGS84.geodesic_inverse(high, to)
        );
        let d = WGS84.geodesic_direct(high, 1.0, 5_000.0).expect("finite");
        assert_eq!(d.end.height_m, 0.0);
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
        assert!((ahead.end.latitude_rad - behind.end.latitude_rad).abs() < 1e-14);
        assert!((ahead.end.longitude_rad - behind.end.longitude_rad).abs() < 1e-14);
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
            let g = WGS84.geodesic_inverse(from, to);
            // No geodesic is longer than half a meridian, pole to pole: 20,003,931.4586 m.
            prop_assert!(g.distance_m >= 0.0 && g.distance_m <= 20_003_931.46);
            prop_assert!(g.initial_azimuth_rad.abs() <= PI && g.final_azimuth_rad.abs() <= PI);
            let d = WGS84
                .geodesic_direct(from, g.initial_azimuth_rad, g.distance_m)
                .expect("finite");
            prop_assert!(d.end.longitude_rad.abs() <= PI && d.final_azimuth_rad.abs() <= PI);
            let miss = WGS84.ecef_from_geodetic(d.end) - WGS84.ecef_from_geodetic(to);
            prop_assert!(miss.length() < 1e-7, "missed by {} m", miss.length());
        }
    }
}
