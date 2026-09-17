//! Reference ellipsoids, geodetic and Earth-centred Earth-fixed (ECEF) coordinates.
//!
//! Formulas and sources are in `docs/physics/geodesy.md`; frame conventions in
//! `docs/physics/frames.md`.
//!
//! - Geodetic to ECEF: NGA.STND.0036_1.0.0_WGS84 (2014), eqs. 4-14 and 4-15.
//! - ECEF to geodetic: C. F. F. Karney, *Geodesics on an ellipsoid of revolution*,
//!   arXiv:1102.1215v1 (2011), appendix B, eqs. B1 to B5: Vermeille's closed form, extended by
//!   Karney to be valid everywhere except in the equatorial plane within `a e²` (about 43 km) of
//!   the Earth's centre.

use glam::{DMat3, DVec3};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// A reference ellipsoid of revolution, oblate or spherical.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EllipsoidData", into = "EllipsoidData")]
pub struct Ellipsoid {
    semi_major_axis_m: f64,
    flattening: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EllipsoidData {
    semi_major_axis_m: f64,
    /// The flattening itself rather than its inverse, which is infinite for a sphere and has no
    /// JSON representation.
    flattening: f64,
}

impl TryFrom<EllipsoidData> for Ellipsoid {
    type Error = CoreError;

    fn try_from(data: EllipsoidData) -> Result<Self, CoreError> {
        Ellipsoid::from_flattening(data.semi_major_axis_m, data.flattening)
    }
}

impl From<Ellipsoid> for EllipsoidData {
    fn from(ellipsoid: Ellipsoid) -> Self {
        EllipsoidData {
            semi_major_axis_m: ellipsoid.semi_major_axis_m,
            flattening: ellipsoid.flattening,
        }
    }
}

impl Ellipsoid {
    /// The WGS 84 ellipsoid: `a = 6378137.0 m`, `1/f = 298.257223563` (NGA.STND.0036_1.0.0_WGS84,
    /// Table 3.1).
    pub const WGS84: Self = Self {
        semi_major_axis_m: 6_378_137.0,
        flattening: 1.0 / 298.257_223_563,
    };

    /// An ellipsoid from its semi-major axis `a` and inverse flattening `1/f`. An infinite `1/f`
    /// is a sphere.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] unless `a` is finite and positive and `1/f` is greater than one
    /// (or `+∞`).
    pub fn new(semi_major_axis_m: f64, inverse_flattening: f64) -> Result<Self, CoreError> {
        if inverse_flattening.is_nan() || inverse_flattening <= 1.0 {
            return Err(CoreError::Domain {
                what: "inverse flattening",
                value: inverse_flattening,
            });
        }
        Self::from_flattening(semi_major_axis_m, 1.0 / inverse_flattening)
    }

    /// An ellipsoid from its semi-major axis `a` and flattening `f` (zero for a sphere).
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] unless `a` is finite and positive and `0 ≤ f < 1`.
    pub fn from_flattening(semi_major_axis_m: f64, flattening: f64) -> Result<Self, CoreError> {
        if !(semi_major_axis_m.is_finite() && semi_major_axis_m > 0.0) {
            return Err(CoreError::Domain {
                what: "semi-major axis (m)",
                value: semi_major_axis_m,
            });
        }
        if !(0.0..1.0).contains(&flattening) {
            return Err(CoreError::Domain {
                what: "flattening",
                value: flattening,
            });
        }
        Ok(Self {
            semi_major_axis_m,
            flattening,
        })
    }

    /// Semi-major (equatorial) axis `a`, m.
    #[must_use]
    pub fn semi_major_axis_m(&self) -> f64 {
        self.semi_major_axis_m
    }

    /// Flattening `f = (a − b)/a`.
    #[must_use]
    pub fn flattening(&self) -> f64 {
        self.flattening
    }

    /// Inverse flattening `1/f` (`+∞` for a sphere).
    #[must_use]
    pub fn inverse_flattening(&self) -> f64 {
        1.0 / self.flattening
    }

    /// Semi-minor (polar) axis `b = a(1 − f)`, m.
    #[must_use]
    pub fn semi_minor_axis_m(&self) -> f64 {
        self.semi_major_axis_m * (1.0 - self.flattening)
    }

    /// First eccentricity squared `e² = f(2 − f) = (a² − b²)/a²`.
    #[must_use]
    pub fn eccentricity_squared(&self) -> f64 {
        self.flattening * (2.0 - self.flattening)
    }

    /// Linear eccentricity `E = √(a² − b²) = a e`, m.
    #[must_use]
    pub fn linear_eccentricity_m(&self) -> f64 {
        self.semi_major_axis_m * self.eccentricity_squared().sqrt()
    }

    /// Radius of curvature in the prime vertical, `N(φ) = a / √(1 − e² sin²φ)`, m (eq. 4-15).
    #[must_use]
    pub fn prime_vertical_radius_m(&self, latitude_rad: f64) -> f64 {
        let s = latitude_rad.sin();
        self.semi_major_axis_m / (1.0 - self.eccentricity_squared() * s * s).sqrt()
    }

    /// ECEF position of a geodetic point (NGA.STND.0036 eq. 4-14):
    ///
    /// ```text
    /// X = (N + h) cos φ cos λ,   Y = (N + h) cos φ sin λ,   Z = ((b²/a²) N + h) sin φ
    /// ```
    #[must_use]
    pub fn ecef_from_geodetic(&self, point: Geodetic) -> DVec3 {
        let (sin_lat, cos_lat) = point.latitude_rad.sin_cos();
        let (sin_lon, cos_lon) = point.longitude_rad.sin_cos();
        let n = self.prime_vertical_radius_m(point.latitude_rad);
        let horizontal = (n + point.height_m) * cos_lat;
        DVec3::new(
            horizontal * cos_lon,
            horizontal * sin_lon,
            (n * (1.0 - self.eccentricity_squared()) + point.height_m) * sin_lat,
        )
    }

    /// Geodetic position of an ECEF point (Karney 2011, appendix B). With `R = √(X² + Y²)`,
    /// `x = R/a` and `y = √(1 − e²) Z/a`, the largest real root `κ` of
    ///
    /// ```text
    /// κ⁴ + 2e²κ³ − (x² + y² − e⁴)κ² − 2e²y²κ − e⁴y² = 0                        (B1)
    /// ```
    ///
    /// gives `φ = ph(R/(κ + e²) + iZ/κ)` (B2) and `h = (1 − (1 − e²)/κ) √(D² + Z²)` with
    /// `D = κR/(κ + e²)` (B3). `κ` comes from the resolvent cubic and the factorization (B4)–(B5)
    /// in the round-off-avoiding form the paper gives. `λ = ph(X + iY)`, which is 0 on the axis.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the position is not finite, or lies in the degenerate set where
    /// the closed form needs limiting forms: the equatorial plane within `a e²` of the centre
    /// (about 42.7 km for WGS 84), deep inside the Earth.
    pub fn geodetic_from_ecef(&self, position_ecef_m: DVec3) -> Result<Geodetic, CoreError> {
        if let Some(value) = first_non_finite(position_ecef_m) {
            return Err(CoreError::Domain {
                what: "ECEF position component (m)",
                value,
            });
        }
        let a = self.semi_major_axis_m;
        let e2 = self.eccentricity_squared();
        let e4 = e2 * e2;
        let big_r = position_ecef_m.x.hypot(position_ecef_m.y);
        let big_z = position_ecef_m.z;
        let longitude_rad = position_ecef_m.y.atan2(position_ecef_m.x);

        let x = big_r / a;
        let y = (1.0 - e2).sqrt() * big_z / a;
        let (x2, y2) = (x * x, y * y);
        let r = (x2 + y2 - e4) / 6.0;
        let s = e4 * x2 * y2 / 4.0;
        let r3 = r * r * r;
        let d = s * (s + 2.0 * r3);
        let u = if d >= 0.0 {
            // The square root takes the sign of S + r³ to avoid cancellation; the cube root is
            // real.
            let t3 = s + r3;
            let t = (t3 + d.sqrt().copysign(t3)).cbrt();
            if t == 0.0 { 0.0 } else { r + t + r * r / t }
        } else {
            let psi = (-d).sqrt().atan2(-s - r3);
            r * (1.0 + 2.0 * (psi / 3.0).cos())
        };
        let v = (u * u + e4 * y2).sqrt();
        // v + u, computed without cancellation when u < 0.
        let v_plus_u = if u < 0.0 { e4 * y2 / (v - u) } else { u + v };
        let w = (v_plus_u - y2) * e2 / (2.0 * v);
        let kappa = v_plus_u / ((v_plus_u + w * w).sqrt() + w);

        let latitude_rad = (big_z / kappa).atan2(big_r / (kappa + e2));
        let big_d = kappa * big_r / (kappa + e2);
        let height_m = (1.0 - (1.0 - e2) / kappa) * big_d.hypot(big_z);
        if !(kappa > 0.0 && latitude_rad.is_finite() && height_m.is_finite()) {
            return Err(CoreError::Domain {
                what: "distance from the Earth's centre (m)",
                value: position_ecef_m.length(),
            });
        }
        Geodetic::new(latitude_rad, longitude_rad, height_m)
    }
}

/// The first component of `v` that is NaN or infinite, for error reports.
pub(crate) fn first_non_finite(v: DVec3) -> Option<f64> {
    v.to_array().into_iter().find(|c| !c.is_finite())
}

/// A geodetic position on an ellipsoid.
///
/// The fields are public for convenience; [`Geodetic::new`], deserialization and every
/// constructor that takes a site ([`crate::frames::LaunchFrame::new`], [`crate::earth::Earth::new`])
/// check them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GeodeticData", into = "GeodeticData")]
pub struct Geodetic {
    /// Geodetic latitude `φ`, rad, in `[−π/2, π/2]`, positive north.
    pub latitude_rad: f64,
    /// Longitude `λ`, rad, positive east of the prime meridian.
    pub longitude_rad: f64,
    /// Height `h` above the ellipsoid along its normal, m. This is not height above mean sea
    /// level: the two differ by the geoid undulation, up to about ±100 m.
    pub height_m: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GeodeticData {
    latitude_rad: f64,
    longitude_rad: f64,
    height_m: f64,
}

impl TryFrom<GeodeticData> for Geodetic {
    type Error = CoreError;

    fn try_from(data: GeodeticData) -> Result<Self, CoreError> {
        Geodetic::new(data.latitude_rad, data.longitude_rad, data.height_m)
    }
}

impl From<Geodetic> for GeodeticData {
    fn from(point: Geodetic) -> Self {
        GeodeticData {
            latitude_rad: point.latitude_rad,
            longitude_rad: point.longitude_rad,
            height_m: point.height_m,
        }
    }
}

impl Geodetic {
    /// Checks a position built from the public fields: the same checks as [`Geodetic::new`].
    ///
    /// # Errors
    ///
    /// As [`Geodetic::new`].
    pub fn validated(self) -> Result<Self, CoreError> {
        Self::new(self.latitude_rad, self.longitude_rad, self.height_m)
    }

    /// A position from latitude and longitude in radians and ellipsoidal height in metres.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the latitude is outside `[−π/2, π/2]` or any value is not finite.
    pub fn new(latitude_rad: f64, longitude_rad: f64, height_m: f64) -> Result<Self, CoreError> {
        if latitude_rad.is_nan() || latitude_rad.abs() > std::f64::consts::FRAC_PI_2 {
            return Err(CoreError::Domain {
                what: "geodetic latitude (rad)",
                value: latitude_rad,
            });
        }
        if !longitude_rad.is_finite() {
            return Err(CoreError::Domain {
                what: "longitude (rad)",
                value: longitude_rad,
            });
        }
        if !height_m.is_finite() {
            return Err(CoreError::Domain {
                what: "ellipsoidal height (m)",
                value: height_m,
            });
        }
        Ok(Self {
            latitude_rad,
            longitude_rad,
            height_m,
        })
    }

    /// A position from latitude and longitude in degrees and ellipsoidal height in metres.
    ///
    /// # Errors
    ///
    /// As [`Geodetic::new`].
    pub fn from_degrees(
        latitude_deg: f64,
        longitude_deg: f64,
        height_m: f64,
    ) -> Result<Self, CoreError> {
        Self::new(
            latitude_deg.to_radians(),
            longitude_deg.to_radians(),
            height_m,
        )
    }
}

/// The rotation whose columns are the East, North and Up unit vectors at `point`, resolved in
/// ECEF. It maps ENU components to ECEF components; its transpose maps back.
///
/// ```text
/// ê = (−sin λ, cos λ, 0)
/// n̂ = (−sin φ cos λ, −sin φ sin λ, cos φ)
/// û = (cos φ cos λ, cos φ sin λ, sin φ)
/// ```
#[must_use]
pub fn ecef_from_enu_rotation(point: Geodetic) -> DMat3 {
    let (sin_lat, cos_lat) = point.latitude_rad.sin_cos();
    let (sin_lon, cos_lon) = point.longitude_rad.sin_cos();
    DMat3::from_cols(
        DVec3::new(-sin_lon, cos_lon, 0.0),
        DVec3::new(-sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat),
        DVec3::new(cos_lat * cos_lon, cos_lat * sin_lon, sin_lat),
    )
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use proptest::prelude::*;

    use super::*;

    const WGS84: Ellipsoid = Ellipsoid::WGS84;

    #[test]
    fn wgs84_derived_geometry_matches_table_3_5() {
        // NGA.STND.0036_1.0.0_WGS84, Table 3.5, to the printed digits.
        assert!((WGS84.semi_minor_axis_m() - 6_356_752.314_2).abs() < 5e-5);
        assert!((WGS84.eccentricity_squared() - 6.694_379_990_141e-3).abs() < 5e-16);
        assert!((WGS84.linear_eccentricity_m() - 5.218_540_084_233_9e5).abs() < 5e-9);
        assert!((WGS84.flattening() - 3.352_810_664_747_5e-3).abs() < 5e-17);
    }

    #[test]
    fn rejects_invalid_ellipsoids_and_points() {
        assert!(Ellipsoid::new(0.0, 298.0).is_err());
        assert!(Ellipsoid::new(6.4e6, 1.0).is_err());
        assert!(Ellipsoid::new(6.4e6, f64::NAN).is_err());
        assert!(Ellipsoid::new(6.4e6, f64::INFINITY).is_ok());
        assert!(Geodetic::new(FRAC_PI_2 + 1e-9, 0.0, 0.0).is_err());
        assert!(Geodetic::new(0.0, f64::INFINITY, 0.0).is_err());
        assert!(Geodetic::from_degrees(0.0, 0.0, f64::NAN).is_err());
        assert!(
            WGS84
                .geodetic_from_ecef(DVec3::new(f64::NAN, 0.0, 0.0))
                .is_err()
        );
        // The degenerate set: the equatorial plane within a e² of the centre.
        assert!(
            WGS84
                .geodetic_from_ecef(DVec3::new(1.0e4, 0.0, 0.0))
                .is_err()
        );
        assert!(WGS84.geodetic_from_ecef(DVec3::ZERO).is_err());
    }

    #[test]
    fn closed_form_points_on_the_axes() {
        let a = WGS84.semi_major_axis_m();
        let b = WGS84.semi_minor_axis_m();
        let equator = WGS84.ecef_from_geodetic(Geodetic::from_degrees(0.0, 90.0, 250.0).unwrap());
        assert!((equator - DVec3::new(0.0, a + 250.0, 0.0)).length() < 1e-9);
        let pole = WGS84.ecef_from_geodetic(Geodetic::from_degrees(-90.0, 0.0, 1000.0).unwrap());
        assert!((pole - DVec3::new(0.0, 0.0, -(b + 1000.0))).length() < 1e-9);

        let g = WGS84
            .geodetic_from_ecef(DVec3::new(0.0, 0.0, b + 3.0e5))
            .unwrap();
        assert_eq!(g.latitude_rad, FRAC_PI_2);
        assert!((g.height_m - 3.0e5).abs() < 1e-8);
        let g = WGS84
            .geodetic_from_ecef(DVec3::new(-(a - 1.0e3), 0.0, 0.0))
            .unwrap();
        assert_eq!(g.latitude_rad, 0.0);
        assert!((g.longitude_rad.abs() - std::f64::consts::PI).abs() < 1e-15);
        assert!((g.height_m + 1.0e3).abs() < 1e-8);
    }

    #[test]
    fn a_sphere_inverts_to_spherical_coordinates() {
        let sphere = Ellipsoid::new(6.371e6, f64::INFINITY).unwrap();
        let p = DVec3::new(3.0e6, 4.0e6, 5.0e6);
        let g = sphere.geodetic_from_ecef(p).unwrap();
        assert!((g.height_m - (p.length() - 6.371e6)).abs() < 1e-8);
        assert!((g.latitude_rad - (5.0e6f64).atan2(5.0e6)).abs() < 1e-15);
    }

    #[test]
    fn serde_round_trips_and_rechecks() {
        for ellipsoid in [WGS84, Ellipsoid::new(6.371e6, f64::INFINITY).unwrap()] {
            let json = serde_json::to_string(&ellipsoid).unwrap();
            assert_eq!(
                serde_json::from_str::<Ellipsoid>(&json).unwrap(),
                ellipsoid,
                "{json}"
            );
        }
        assert!(
            serde_json::from_str::<Ellipsoid>(r#"{"semi_major_axis_m": 6.4e6, "flattening": 1.0}"#)
                .is_err()
        );
        let point = Geodetic::from_degrees(-33.9, 18.6, 300.0).unwrap();
        let json = serde_json::to_string(&point).unwrap();
        assert_eq!(serde_json::from_str::<Geodetic>(&json).unwrap(), point);
        let degrees_in_radian_fields =
            r#"{"latitude_rad": 32.99, "longitude_rad": -106.97, "height_m": 1400.0}"#;
        assert!(serde_json::from_str::<Geodetic>(degrees_in_radian_fields).is_err());
        let unchecked = Geodetic {
            latitude_rad: f64::NAN,
            ..point
        };
        assert!(unchecked.validated().is_err());
        assert_eq!(point.validated(), Ok(point));
    }

    #[test]
    fn errors_report_the_offending_component() {
        let err = WGS84
            .geodetic_from_ecef(DVec3::new(f64::NAN, 1.0, 2.0))
            .unwrap_err();
        assert!(matches!(err, CoreError::Domain { value, .. } if value.is_nan()));
    }

    #[test]
    fn enu_rotation_is_proper_and_up_is_the_ellipsoid_normal() {
        let point = Geodetic::from_degrees(37.2, -115.8, 1300.0).unwrap();
        let r = ecef_from_enu_rotation(point);
        assert!((r.transpose() * r).abs_diff_eq(DMat3::IDENTITY, 1e-15));
        assert!((r.determinant() - 1.0).abs() < 1e-15);
        // The outward normal of x²/a² + y²/a² + z²/b² = 1 at the foot of the point.
        let foot = WGS84.ecef_from_geodetic(Geodetic {
            height_m: 0.0,
            ..point
        });
        let a2 = WGS84.semi_major_axis_m().powi(2);
        let b2 = WGS84.semi_minor_axis_m().powi(2);
        let normal = DVec3::new(foot.x / a2, foot.y / a2, foot.z / b2).normalize();
        assert!((r.z_axis - normal).length() < 1e-15);
    }

    proptest! {
        /// Geodetic → ECEF → geodetic, from 10 km below the ellipsoid to 1000 km above it.
        #[test]
        fn geodetic_round_trips_through_ecef(
            lat in -FRAC_PI_2..=FRAC_PI_2,
            lon in -std::f64::consts::PI..std::f64::consts::PI,
            h in -1.0e4..1.0e6f64,
        ) {
            let point = Geodetic::new(lat, lon, h).unwrap();
            let ecef = WGS84.ecef_from_geodetic(point);
            let back = WGS84.geodetic_from_ecef(ecef).unwrap();
            prop_assert!((back.latitude_rad - lat).abs() < 1e-14, "lat {} vs {}", back.latitude_rad, lat);
            prop_assert!((back.height_m - h).abs() < 2e-8, "h {} vs {}", back.height_m, h);
            // Longitude is undefined on the axis; compare positions instead.
            prop_assert!((WGS84.ecef_from_geodetic(back) - ecef).length() < 2e-8);
            if lat.abs() < FRAC_PI_2 - 1e-9 {
                let dlon = (back.longitude_rad - lon + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
                prop_assert!(dlon.abs() < 1e-14 / lat.cos().max(1e-6), "lon {} vs {}", back.longitude_rad, lon);
            }
        }

        /// ECEF → geodetic → ECEF anywhere from 100 km below the surface to 40,000 km out.
        #[test]
        fn ecef_round_trips_through_geodetic(
            direction in prop::array::uniform3(-1.0..1.0f64),
            radius in 6.25e6..4.6e7f64,
        ) {
            let d = DVec3::from_array(direction);
            prop_assume!(d.length() > 1e-3);
            let p = d.normalize() * radius;
            let g = WGS84.geodetic_from_ecef(p).unwrap();
            let back = WGS84.ecef_from_geodetic(g);
            prop_assert!((back - p).length() < 1e-7 * (radius / 6.4e6), "{back} vs {p}");
        }
    }
}
