//! The launch frame, the body frame and launch attitude angles.
//!
//! `docs/physics/frames.md` is the specification; this module implements it.
//!
//! - **Launch frame `L`:** East-North-Up at the launch site. Origin at the pad on the ground;
//!   `x_L` east, `y_L` north, `z_L` up along the ellipsoid normal. See [`LaunchFrame`].
//! - **Body frame `B`:** `z_B` along the rocket's axis of symmetry, positive toward the nose;
//!   `x_B` along the design's reference radial direction; `y_B = z_B × x_B`.
//! - **Attitude:** the unit quaternion `q` with `v_L = q ⊗ v_B ⊗ q*` ([`crate::attitude`]).
//! - **Launch angles:** azimuth `A` (clockwise from true north), elevation `E` (above the
//!   horizon) and roll `φ` (about `z_B`), with `q = R_z(−A) R_x(E − π/2) R_z(φ)`. See
//!   [`LaunchAngles`].

use std::f64::consts::{FRAC_PI_2, TAU};

use glam::{DMat3, DQuat, DVec3};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::geodesy::{Ellipsoid, Geodetic, ecef_from_enu_rotation};

/// Below this horizontal length of the body axis (in the launch frame, for a unit axis) the
/// azimuth is undefined: the axis is within about 1e-12 rad of vertical.
const VERTICAL_TOLERANCE: f64 = 1e-12;

/// An East-North-Up frame tied to a point on an ellipsoid: the launch frame `L`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LaunchFrameData", into = "LaunchFrameData")]
pub struct LaunchFrame {
    ellipsoid: Ellipsoid,
    origin: Geodetic,
    origin_ecef_m: DVec3,
    ecef_from_enu: DMat3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchFrameData {
    ellipsoid: Ellipsoid,
    origin: Geodetic,
}

impl TryFrom<LaunchFrameData> for LaunchFrame {
    type Error = CoreError;

    fn try_from(data: LaunchFrameData) -> Result<Self, CoreError> {
        LaunchFrame::new(data.ellipsoid, data.origin)
    }
}

impl From<LaunchFrame> for LaunchFrameData {
    fn from(frame: LaunchFrame) -> Self {
        LaunchFrameData {
            ellipsoid: frame.ellipsoid,
            origin: frame.origin,
        }
    }
}

impl LaunchFrame {
    /// The ENU frame with its origin at `origin` on `ellipsoid`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if `origin` fails [`Geodetic::new`]'s checks (for example degrees
    /// written into the radian fields).
    pub fn new(ellipsoid: Ellipsoid, origin: Geodetic) -> Result<Self, CoreError> {
        let origin = origin.validated()?;
        Ok(Self {
            ellipsoid,
            origin,
            origin_ecef_m: ellipsoid.ecef_from_geodetic(origin),
            ecef_from_enu: ecef_from_enu_rotation(origin),
        })
    }

    /// The ENU frame at `origin` on the WGS 84 ellipsoid.
    ///
    /// # Errors
    ///
    /// As [`LaunchFrame::new`].
    pub fn wgs84(origin: Geodetic) -> Result<Self, CoreError> {
        Self::new(Ellipsoid::WGS84, origin)
    }

    /// The ellipsoid the frame is tied to.
    #[must_use]
    pub fn ellipsoid(&self) -> Ellipsoid {
        self.ellipsoid
    }

    /// The origin's geodetic position.
    #[must_use]
    pub fn origin(&self) -> Geodetic {
        self.origin
    }

    /// The origin's ECEF position, m.
    #[must_use]
    pub fn origin_ecef_m(&self) -> DVec3 {
        self.origin_ecef_m
    }

    /// The rotation mapping launch-frame components to ECEF components; its columns are `ê`,
    /// `n̂` and `û` at the origin.
    #[must_use]
    pub fn ecef_from_enu_rotation(&self) -> DMat3 {
        self.ecef_from_enu
    }

    /// ECEF position of a point given in the launch frame: `r_ECEF = r_0 + R r_L`.
    #[must_use]
    pub fn ecef_from_enu(&self, position_enu_m: DVec3) -> DVec3 {
        self.origin_ecef_m + self.ecef_from_enu * position_enu_m
    }

    /// Launch-frame position of a point given in ECEF: `r_L = Rᵀ (r_ECEF − r_0)`.
    #[must_use]
    pub fn enu_from_ecef(&self, position_ecef_m: DVec3) -> DVec3 {
        self.ecef_from_enu.transpose() * (position_ecef_m - self.origin_ecef_m)
    }

    /// Geodetic position of a point given in the launch frame.
    ///
    /// # Errors
    ///
    /// As [`Ellipsoid::geodetic_from_ecef`]: only for non-finite input or points deep inside the
    /// Earth.
    pub fn geodetic_from_enu(&self, position_enu_m: DVec3) -> Result<Geodetic, CoreError> {
        self.ellipsoid
            .geodetic_from_ecef(self.ecef_from_enu(position_enu_m))
    }

    /// Launch-frame position of a geodetic point.
    #[must_use]
    pub fn enu_from_geodetic(&self, point: Geodetic) -> DVec3 {
        self.enu_from_ecef(self.ellipsoid.ecef_from_geodetic(point))
    }

    /// The Earth's rotation vector `Ω = ω (0, cos φ₀, sin φ₀)` resolved in the launch frame,
    /// rad/s, for a rotation rate `omega_rad_s` (WGS 84: 7.292115e-5 rad/s).
    #[must_use]
    pub fn earth_rotation_enu_rad_s(&self, omega_rad_s: f64) -> DVec3 {
        let (sin_lat, cos_lat) = self.origin.latitude_rad.sin_cos();
        DVec3::new(0.0, omega_rad_s * cos_lat, omega_rad_s * sin_lat)
    }
}

/// The attitude of the body axis relative to the launch frame, as launch-rail style angles.
///
/// `q = R_z(−A) ⊗ R_x(E − π/2) ⊗ R_z(φ)`, so the body axis `z_B` points along
/// `(sin A cos E, cos A cos E, sin E)` in the launch frame. At `E = ±90°` azimuth and roll turn
/// about the same axis; [`LaunchAngles::from_quaternion`] then reports `A = 0` and puts the whole
/// turn into `φ`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LaunchAngles {
    /// Azimuth `A` of the body axis, rad, clockwise from true north (`π/2` points east).
    pub azimuth_rad: f64,
    /// Elevation `E` of the body axis above the horizon, rad (`π/2` is vertical).
    pub elevation_rad: f64,
    /// Roll `φ` about the body axis, rad, right-handed about `z_B`.
    pub roll_rad: f64,
}

impl LaunchAngles {
    /// The attitude quaternion `q = R_z(−A) ⊗ R_x(E − π/2) ⊗ R_z(φ)`.
    #[must_use]
    pub fn to_quaternion(self) -> DQuat {
        DQuat::from_rotation_z(-self.azimuth_rad)
            * DQuat::from_rotation_x(self.elevation_rad - FRAC_PI_2)
            * DQuat::from_rotation_z(self.roll_rad)
    }

    /// The launch angles of a unit attitude quaternion, with `A ∈ [0, 2π)`, `E ∈ [−π/2, π/2]`
    /// and `φ ∈ (−π, π]`.
    #[must_use]
    pub fn from_quaternion(q: DQuat) -> Self {
        let axis = q.mul_vec3(DVec3::Z);
        let horizontal = axis.x.hypot(axis.y);
        let elevation_rad = axis.z.atan2(horizontal);
        let azimuth_rad = if horizontal < VERTICAL_TOLERANCE {
            0.0
        } else {
            // `rem_euclid` can round a tiny negative angle up to exactly 2π.
            let a = axis.x.atan2(axis.y).rem_euclid(TAU);
            if a < TAU { a } else { 0.0 }
        };
        // The zero-roll radial axes for this azimuth and elevation, and the body's actual x axis.
        let reference = DQuat::from_rotation_z(-azimuth_rad)
            * DQuat::from_rotation_x(elevation_rad - FRAC_PI_2);
        let x0 = reference.mul_vec3(DVec3::X);
        let y0 = reference.mul_vec3(DVec3::Y);
        let x_body = q.mul_vec3(DVec3::X);
        // atan2 returns −π for (−0, negative); the documented range is (−π, π].
        let roll = x_body.dot(y0).atan2(x_body.dot(x0));
        let roll_rad = if roll <= -std::f64::consts::PI {
            std::f64::consts::PI
        } else {
            roll
        };
        Self {
            azimuth_rad,
            elevation_rad,
            roll_rad,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use proptest::prelude::*;

    use super::*;

    /// The rotation angle between two attitudes, rad (`q` and `−q` are the same attitude).
    fn angle_between(a: DQuat, b: DQuat) -> f64 {
        let d = a.inverse() * b;
        2.0 * d.xyz().length().atan2(d.w.abs())
    }

    fn unit_quaternion() -> impl Strategy<Value = DQuat> {
        prop::array::uniform4(-1.0..1.0f64)
            .prop_filter("non-degenerate", |c| {
                c.iter().map(|x| x * x).sum::<f64>() > 1e-6
            })
            .prop_map(|c| DQuat::from_array(c).normalize())
    }

    #[test]
    fn named_attitudes() {
        let vertical = LaunchAngles {
            azimuth_rad: 0.0,
            elevation_rad: FRAC_PI_2,
            roll_rad: 0.0,
        };
        assert!(angle_between(vertical.to_quaternion(), DQuat::IDENTITY) < 1e-15);

        // Horizontal, pointing east: the nose axis is +x_L; zero roll keeps x_B horizontal and to
        // the right of the heading (south), so y_B = z_B × x_B points down.
        let east = LaunchAngles {
            azimuth_rad: FRAC_PI_2,
            elevation_rad: 0.0,
            roll_rad: 0.0,
        }
        .to_quaternion();
        assert!((east.mul_vec3(DVec3::Z) - DVec3::X).length() < 1e-15);
        assert!((east.mul_vec3(DVec3::X) - DVec3::NEG_Y).length() < 1e-15);
        assert!((east.mul_vec3(DVec3::Y) - DVec3::NEG_Z).length() < 1e-15);

        // A rail tilted 5° off vertical toward the north-east.
        let rail = LaunchAngles {
            azimuth_rad: 45f64.to_radians(),
            elevation_rad: 85f64.to_radians(),
            roll_rad: 0.3,
        }
        .to_quaternion();
        let axis = rail.mul_vec3(DVec3::Z);
        let (s5, c5) = 5f64.to_radians().sin_cos();
        let expected = DVec3::new(s5 * 0.5f64.sqrt(), s5 * 0.5f64.sqrt(), c5);
        assert!((axis - expected).length() < 1e-14);
    }

    #[test]
    fn vertical_attitudes_report_zero_azimuth_and_the_whole_turn_as_roll() {
        let q = DQuat::from_rotation_z(-1.1);
        let angles = LaunchAngles::from_quaternion(q);
        assert_eq!(angles.azimuth_rad, 0.0);
        assert!((angles.elevation_rad - FRAC_PI_2).abs() < 1e-15);
        assert!((angles.roll_rad + 1.1).abs() < 1e-15);

        // Nose straight down by a half turn about x_L: exactly the zero-roll reference.
        let down = LaunchAngles::from_quaternion(DQuat::from_xyzw(1.0, 0.0, 0.0, 0.0));
        assert_eq!(down.azimuth_rad, 0.0);
        assert!((down.elevation_rad + FRAC_PI_2).abs() < 1e-15);
        assert_eq!(down.roll_rad, 0.0);
        // By a half turn about y_L, x_B ends up at −x_L and atan2 meets (−0, −1): roll lands on
        // +π, not −π.
        for sign in [1.0, -1.0] {
            let down = LaunchAngles::from_quaternion(DQuat::from_xyzw(0.0, sign, 0.0, 0.0));
            assert!((down.elevation_rad + FRAC_PI_2).abs() < 1e-15);
            assert_eq!(down.roll_rad, PI);
        }
    }

    /// ADR-003 adopts RocketPy's launch attitude convention: heading = azimuth, inclination =
    /// elevation, and roll = the rail-button angular position for a `tail_to_nose` rocket or
    /// 2π minus it for `nose_to_tail`. The oracle builds real RocketPy flights and reads the
    /// initial Euler parameters (`validation/oracles/rocketpy/attitude.py`).
    #[test]
    fn launch_angles_match_the_rocketpy_oracle() {
        #[derive(serde::Deserialize)]
        struct Oracle {
            cases: Vec<Case>,
        }
        #[derive(serde::Deserialize)]
        struct Case {
            heading_deg: f64,
            inclination_deg: f64,
            rail_button_angle_deg: f64,
            coordinate_system_orientation: String,
            e0: f64,
            e1: f64,
            e2: f64,
            e3: f64,
        }
        let oracle: Oracle = serde_json::from_str(include_str!(
            "../../../validation/fixtures/earth/rocketpy-attitude.json"
        ))
        .unwrap();
        assert!(oracle.cases.len() >= 6);
        let mut orientations = std::collections::BTreeSet::new();
        for case in &oracle.cases {
            let button = case.rail_button_angle_deg.to_radians();
            let roll_rad = match case.coordinate_system_orientation.as_str() {
                "tail_to_nose" => button,
                "nose_to_tail" => 2.0 * PI - button,
                other => panic!("unknown orientation {other}"),
            };
            orientations.insert(case.coordinate_system_orientation.as_str());
            let q = LaunchAngles {
                azimuth_rad: case.heading_deg.to_radians(),
                elevation_rad: case.inclination_deg.to_radians(),
                roll_rad,
            }
            .to_quaternion();
            let rocketpy = DQuat::from_xyzw(case.e1, case.e2, case.e3, case.e0);
            assert!(
                angle_between(q, rocketpy) < 1e-12,
                "heading {} inclination {} buttons {}: {q} vs {rocketpy}",
                case.heading_deg,
                case.inclination_deg,
                case.rail_button_angle_deg
            );
        }
        assert_eq!(
            orientations.len(),
            2,
            "both rocket orientations are covered"
        );
    }

    #[test]
    fn launch_frame_rejects_unchecked_sites() {
        let degrees = Geodetic {
            latitude_rad: 32.99,
            longitude_rad: -106.97,
            height_m: 1400.0,
        };
        assert!(LaunchFrame::wgs84(degrees).is_err());
        let json = r#"{"ellipsoid": {"semi_major_axis_m": 6378137.0, "flattening": 0.0033528106647474805},
            "origin": {"latitude_rad": 1.0, "longitude_rad": 0.0, "height_m": "NaN"}}"#;
        assert!(serde_json::from_str::<LaunchFrame>(json).is_err());
    }

    #[test]
    fn launch_frame_axes_and_origin() {
        let site = Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap();
        let frame = LaunchFrame::wgs84(site).unwrap();
        assert_eq!(frame.enu_from_ecef(frame.origin_ecef_m()), DVec3::ZERO);
        let back = frame.geodetic_from_enu(DVec3::ZERO).unwrap();
        assert!((back.latitude_rad - site.latitude_rad).abs() < 1e-15);
        assert!((back.height_m - site.height_m).abs() < 1e-8);
        // 1 km straight up is 1 km higher on the ellipsoid, same latitude and longitude.
        let up = frame
            .geodetic_from_enu(DVec3::new(0.0, 0.0, 1000.0))
            .unwrap();
        assert!((up.height_m - 2400.0).abs() < 1e-8);
        assert!((up.latitude_rad - site.latitude_rad).abs() < 1e-15);
        assert!((up.longitude_rad - site.longitude_rad).abs() < 1e-15);
        // Moving north raises latitude; moving east raises longitude.
        let north = frame
            .geodetic_from_enu(DVec3::new(0.0, 1000.0, 0.0))
            .unwrap();
        assert!(north.latitude_rad > site.latitude_rad);
        let east = frame
            .geodetic_from_enu(DVec3::new(1000.0, 0.0, 0.0))
            .unwrap();
        assert!(east.longitude_rad > site.longitude_rad);
        // A tangent-plane point 10 km away sits above the curving ellipsoid by about d²/(2N).
        let n = frame.ellipsoid().prime_vertical_radius_m(site.latitude_rad);
        let far = frame
            .geodetic_from_enu(DVec3::new(1.0e4, 0.0, 0.0))
            .unwrap();
        let drop = far.height_m - site.height_m;
        assert!((drop - 1.0e8 / (2.0 * (n + 1400.0))).abs() < 1e-3, "{drop}");

        let json = serde_json::to_string(&frame).unwrap();
        assert_eq!(serde_json::from_str::<LaunchFrame>(&json).unwrap(), frame);
    }

    proptest! {
        /// Angles → quaternion → angles, away from the vertical singularity.
        #[test]
        fn launch_angles_round_trip(
            azimuth in 0.0..2.0 * PI,
            elevation in -FRAC_PI_2 + 1e-3..FRAC_PI_2 - 1e-3,
            roll in -PI + 1e-9..PI,
        ) {
            let angles = LaunchAngles { azimuth_rad: azimuth, elevation_rad: elevation, roll_rad: roll };
            let back = LaunchAngles::from_quaternion(angles.to_quaternion());
            let wrap = |x: f64| (x + PI).rem_euclid(2.0 * PI) - PI;
            prop_assert!(wrap(back.azimuth_rad - azimuth).abs() < 1e-11, "azimuth {} vs {}", back.azimuth_rad, azimuth);
            prop_assert!((back.elevation_rad - elevation).abs() < 1e-11, "elevation {} vs {}", back.elevation_rad, elevation);
            prop_assert!(wrap(back.roll_rad - roll).abs() < 1e-11, "roll {} vs {}", back.roll_rad, roll);
            prop_assert!((0.0..2.0 * PI).contains(&back.azimuth_rad));
        }

        /// Quaternion → angles → quaternion is the same rotation everywhere, the vertical
        /// singularity included, and the angles land in their documented ranges.
        #[test]
        fn quaternion_round_trips_through_launch_angles(q in unit_quaternion(), case in 0u8..3) {
            // A third of the cases point exactly up and a third exactly down.
            let turn = DQuat::from_rotation_z(2.0 * q.w.atan2(q.z));
            let q = match case {
                0 => q,
                1 => turn,
                _ => turn * DQuat::from_rotation_x(PI),
            };
            let angles = LaunchAngles::from_quaternion(q);
            prop_assert!(angle_between(angles.to_quaternion(), q) < 1e-12);
            prop_assert!((0.0..2.0 * PI).contains(&angles.azimuth_rad));
            prop_assert!(angles.elevation_rad.abs() <= FRAC_PI_2);
            prop_assert!(angles.roll_rad > -PI && angles.roll_rad <= PI);
            // The body axis agrees with the azimuth and elevation formula.
            let (sa, ca) = angles.azimuth_rad.sin_cos();
            let (se, ce) = angles.elevation_rad.sin_cos();
            prop_assert!((q.mul_vec3(DVec3::Z) - DVec3::new(sa * ce, ca * ce, se)).length() < 1e-12);
        }

        /// ENU ↔ ECEF and ENU ↔ geodetic round trips for points within 2000 km of the site and up
        /// to 1000 km high, at any site.
        #[test]
        fn launch_frame_round_trips(
            lat in -FRAC_PI_2..=FRAC_PI_2,
            lon in -PI..PI,
            h0 in -500.0..5000.0f64,
            offset in prop::array::uniform3(-2.0e6..2.0e6f64),
            up in 0.0..1.0e6f64,
        ) {
            let frame = LaunchFrame::wgs84(Geodetic::new(lat, lon, h0).unwrap()).unwrap();
            let p = DVec3::new(offset[0], offset[1], up);
            let ecef = frame.ecef_from_enu(p);
            prop_assert!((frame.enu_from_ecef(ecef) - p).length() < 1e-7);
            let g = frame.geodetic_from_enu(p).unwrap();
            prop_assert!((frame.enu_from_geodetic(g) - p).length() < 1e-7);
            // The frame rotation is orthonormal and right-handed.
            let r = frame.ecef_from_enu_rotation();
            prop_assert!((r.transpose() * r).abs_diff_eq(DMat3::IDENTITY, 1e-15));
            prop_assert!((r.determinant() - 1.0).abs() < 1e-15);
        }
    }
}
