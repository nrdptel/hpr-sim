//! The Earth as the flight engine sees it: a launch frame, a gravity model and the rotation
//! terms, all resolved in the launch frame `L`.
//!
//! `L` is fixed to the rotating Earth, so a body in it feels normal gravity (gravitation plus
//! centrifugal, [`crate::gravity`]) and, when it moves, the Coriolis acceleration `−2 Ω × v`.
//! The centrifugal term is already inside normal gravity and is never added separately.
//! Equations and choices are in `docs/physics/gravity.md` and ADR-003.

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::frames::LaunchFrame;
use crate::geodesy::{Geodetic, first_non_finite};
use crate::gravity::NormalGravity;

/// How gravity is evaluated along the trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum GravityModel {
    /// A uniform field `(0, 0, −g)` in the launch frame.
    Constant {
        /// Gravity magnitude, m/s², not negative: the field points down whatever the sign
        /// convention of the source, and [`Earth::new`] rejects a negative value.
        g_mps2: f64,
    },
    /// `(0, 0, −γ)` with `γ` from the Taylor series (eq. 4-3) at the launch latitude and height
    /// `h₀ + z`: RocketPy's gravity formula, for like-for-like comparisons. A RocketPy flight
    /// differs from the formula in three ways a case must reproduce itself: it evaluates it at
    /// height above sea level rather than above the ellipsoid; it holds the value constant above
    /// `max_expected_height` (80 km by default; at 100 km that is 0.6% high); and it does not fly
    /// the formula at all but a 100-point cubic spline through it
    /// (`Environment.set_gravity_model` → `Function.set_discrete`), which over the 0 to 4.4 km of
    /// M1.7a's descents differs from the formula by at most 4.6e-8 m/s².
    VerticalTaylor,
    /// `(0, 0, −|γ|)` with the exact magnitude (eq. 4-4) at the launch latitude and longitude and
    /// height `h₀ + z`: altitude-dependent, but always along the launch site's vertical.
    Vertical,
    /// The full normal gravity vector at the body's actual position, resolved in the launch
    /// frame. It includes the turn of the vertical over long ranges (about 0.9 mrad per 5.7 km)
    /// and the small deflection above the ellipsoid.
    #[default]
    Ellipsoidal,
}

/// Which Earth-rotation terms the launch-frame equations of motion include.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EarthRotation {
    /// No rotation terms: `L` is treated as inertial apart from the centrifugal part already in
    /// normal gravity.
    Ignore,
    /// The Coriolis acceleration `−2 Ω × v`.
    #[default]
    Coriolis,
}

/// The Earth model for one launch site.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EarthData", into = "EarthData")]
pub struct Earth {
    field: NormalGravity,
    frame: LaunchFrame,
    gravity: GravityModel,
    rotation: EarthRotation,
    /// `Ω` resolved in the launch frame, rad/s.
    omega_enu_rad_s: DVec3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EarthData {
    field: NormalGravity,
    site: Geodetic,
    gravity: GravityModel,
    rotation: EarthRotation,
}

impl TryFrom<EarthData> for Earth {
    type Error = CoreError;

    fn try_from(data: EarthData) -> Result<Self, CoreError> {
        Earth::new(data.field, data.site, data.gravity, data.rotation)
    }
}

impl From<Earth> for EarthData {
    fn from(earth: Earth) -> Self {
        EarthData {
            field: earth.field,
            site: earth.frame.origin(),
            gravity: earth.gravity,
            rotation: earth.rotation,
        }
    }
}

impl Earth {
    /// The Earth model for a launch site at `site` (on the field's ellipsoid).
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the site is not a valid geodetic position, or a constant gravity
    /// magnitude is negative or not finite.
    pub fn new(
        field: NormalGravity,
        site: Geodetic,
        gravity: GravityModel,
        rotation: EarthRotation,
    ) -> Result<Self, CoreError> {
        if let GravityModel::Constant { g_mps2 } = gravity
            && !(g_mps2.is_finite() && g_mps2 >= 0.0)
        {
            return Err(CoreError::Domain {
                what: "constant gravity magnitude (m/s²), which must be finite and not negative",
                value: g_mps2,
            });
        }
        let frame = LaunchFrame::new(field.ellipsoid(), site)?;
        Ok(Self {
            field,
            frame,
            gravity,
            rotation,
            omega_enu_rad_s: frame.earth_rotation_enu_rad_s(field.angular_velocity_rad_s()),
        })
    }

    /// The WGS 84 Earth at `site`, with the default models: ellipsoidal gravity and Coriolis.
    ///
    /// # Errors
    ///
    /// As [`Earth::new`].
    pub fn wgs84(site: Geodetic) -> Result<Self, CoreError> {
        Self::new(
            NormalGravity::wgs84(),
            site,
            GravityModel::default(),
            EarthRotation::default(),
        )
    }

    /// The normal gravity field.
    #[must_use]
    pub fn field(&self) -> &NormalGravity {
        &self.field
    }

    /// The launch frame.
    #[must_use]
    pub fn frame(&self) -> &LaunchFrame {
        &self.frame
    }

    /// The gravity model.
    #[must_use]
    pub fn gravity_model(&self) -> GravityModel {
        self.gravity
    }

    /// The rotation terms included.
    #[must_use]
    pub fn rotation(&self) -> EarthRotation {
        self.rotation
    }

    /// The Earth's angular velocity `Ω = ω (0, cos φ₀, sin φ₀)` resolved in the launch frame,
    /// rad/s, whichever terms are enabled.
    #[must_use]
    pub fn angular_velocity_enu_rad_s(&self) -> DVec3 {
        self.omega_enu_rad_s
    }

    /// Gravity at a launch-frame position, resolved in the launch frame, m/s².
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] for a non-finite position, or (for the ellipsoidal model) a position
    /// hundreds of kilometres inside the Earth.
    pub fn gravity_enu_mps2(&self, position_enu_m: DVec3) -> Result<DVec3, CoreError> {
        if let Some(value) = first_non_finite(position_enu_m) {
            return Err(CoreError::Domain {
                what: "launch-frame position component (m)",
                value,
            });
        }
        let site = self.frame.origin();
        let height_m = site.height_m + position_enu_m.z;
        match self.gravity {
            GravityModel::Constant { g_mps2 } => Ok(DVec3::new(0.0, 0.0, -g_mps2)),
            GravityModel::VerticalTaylor => Ok(DVec3::new(
                0.0,
                0.0,
                -self.field.taylor_mps2(site.latitude_rad, height_m)?,
            )),
            GravityModel::Vertical => {
                let above = Geodetic { height_m, ..site };
                Ok(DVec3::new(
                    0.0,
                    0.0,
                    -self.field.enu_at_mps2(above)?.length(),
                ))
            }
            GravityModel::Ellipsoidal => {
                let gamma = self
                    .field
                    .ecef_mps2(self.frame.ecef_from_enu(position_enu_m))?;
                Ok(self.frame.ecef_from_enu_rotation().transpose() * gamma)
            }
        }
    }

    /// The Earth-rotation acceleration on a body moving at `velocity_enu_m_s` relative to the
    /// launch frame: `−2 Ω × v` with Coriolis enabled, zero otherwise, m/s².
    #[must_use]
    pub fn rotation_acceleration_enu_mps2(&self, velocity_enu_m_s: DVec3) -> DVec3 {
        match self.rotation {
            EarthRotation::Ignore => DVec3::ZERO,
            EarthRotation::Coriolis => -2.0 * self.omega_enu_rad_s.cross(velocity_enu_m_s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gravity::STANDARD_GRAVITY_MPS2;

    fn site() -> Geodetic {
        Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap()
    }

    fn earth(gravity: GravityModel) -> Earth {
        Earth::new(
            NormalGravity::wgs84(),
            site(),
            gravity,
            EarthRotation::Coriolis,
        )
        .unwrap()
    }

    #[test]
    fn gravity_models_agree_at_the_pad_and_differ_as_documented_aloft() {
        let field = NormalGravity::wgs84();
        let pad = DVec3::ZERO;
        let exact = field.enu_at_mps2(site()).unwrap();

        let constant = earth(GravityModel::Constant {
            g_mps2: STANDARD_GRAVITY_MPS2,
        });
        assert_eq!(
            constant
                .gravity_enu_mps2(DVec3::new(1e4, -3e3, 2e4))
                .unwrap(),
            DVec3::new(0.0, 0.0, -9.806_65)
        );

        // At the pad the vertical models all give the exact magnitude (Taylor to its 1e-8).
        let vertical = earth(GravityModel::Vertical).gravity_enu_mps2(pad).unwrap();
        assert_eq!(vertical, DVec3::new(0.0, 0.0, -exact.length()));
        let taylor = earth(GravityModel::VerticalTaylor)
            .gravity_enu_mps2(pad)
            .unwrap();
        assert!((taylor.z - vertical.z).abs() < 1e-7);
        let full = earth(GravityModel::Ellipsoidal)
            .gravity_enu_mps2(pad)
            .unwrap();
        assert!((full - exact).length() < 1e-12);

        // 30 km straight up: the models share the height dependence.
        let up = DVec3::new(0.0, 0.0, 3.0e4);
        let above = Geodetic {
            height_m: site().height_m + 3.0e4,
            ..site()
        };
        let exact_up = field.enu_at_mps2(above).unwrap();
        let full_up = earth(GravityModel::Ellipsoidal)
            .gravity_enu_mps2(up)
            .unwrap();
        assert!((full_up - exact_up).length() < 1e-9);
        let vertical_up = earth(GravityModel::Vertical).gravity_enu_mps2(up).unwrap();
        assert!((vertical_up.z + exact_up.length()).abs() < 1e-12);
    }

    /// Far downrange the ellipsoidal model tilts gravity back toward the pad by the angle the
    /// vertical turns through: about d/R, with R the local radius of curvature.
    #[test]
    fn ellipsoidal_gravity_turns_with_the_vertical_downrange() {
        let e = earth(GravityModel::Ellipsoidal);
        let d = 20_000.0;
        let ellipsoid = e.frame().ellipsoid();
        let lat = site().latitude_rad;
        let h0 = site().height_m;
        // East-west the vertical turns with the prime-vertical radius N; north-south with the
        // meridian radius M = a(1 − e²)/(1 − e² sin²φ)^(3/2).
        let n = ellipsoid.prime_vertical_radius_m(lat);
        let e2 = ellipsoid.eccentricity_squared();
        let m =
            ellipsoid.semi_major_axis_m() * (1.0 - e2) / (1.0 - e2 * lat.sin().powi(2)).powf(1.5);

        let east = e.gravity_enu_mps2(DVec3::new(d, 0.0, 0.0)).unwrap();
        let east_tilt = (-east.x).atan2(-east.z);
        let east_expected = d / (n + h0);
        assert!(east.x < 0.0, "gravity leans back toward the pad");
        assert!(
            (east_tilt - east_expected).abs() < 1e-4 * east_expected,
            "east: {east_tilt} vs {east_expected}"
        );

        // Northward the ellipsoid's curvature varies along the path and the normal gravity
        // deflection above the ellipsoid adds a little, so the agreement is looser.
        let north = e.gravity_enu_mps2(DVec3::new(0.0, d, 0.0)).unwrap();
        let north_tilt = (-north.y).atan2(-north.z);
        let north_expected = d / (m + h0);
        assert!(north.y < 0.0, "gravity leans back toward the pad");
        assert!(
            (north_tilt - north_expected).abs() < 1e-3 * north_expected,
            "north: {north_tilt} vs {north_expected}"
        );
        // And not with N, which differs from M by about 0.5% at this latitude.
        assert!((north_tilt - d / (n + h0)).abs() > 3e-3 * north_expected);
    }

    #[test]
    fn coriolis_is_minus_two_omega_cross_v() {
        let e = earth(GravityModel::Ellipsoidal);
        let lat = site().latitude_rad;
        let omega = crate::gravity::WGS84_ANGULAR_VELOCITY_RAD_S;
        assert_eq!(
            e.angular_velocity_enu_rad_s(),
            DVec3::new(0.0, omega * lat.cos(), omega * lat.sin())
        );
        // For v northward, Ω × ŷ = (−Ω_z, 0, 0), so −2Ω×v = (2Ω_z v, 0, 0): eastward, the
        // northern-hemisphere deflection to the right.
        let v_north = DVec3::new(0.0, 100.0, 0.0);
        let a = e.rotation_acceleration_enu_mps2(v_north);
        assert!((a - DVec3::new(2.0 * omega * lat.sin() * 100.0, 0.0, 0.0)).length() < 1e-15);
        // A vertical launch drifts west.
        let a_up = e.rotation_acceleration_enu_mps2(DVec3::new(0.0, 0.0, 300.0));
        assert!(a_up.x < 0.0 && a_up.y == 0.0 && a_up.z == 0.0);

        let still = Earth::new(
            NormalGravity::wgs84(),
            site(),
            GravityModel::Ellipsoidal,
            EarthRotation::Ignore,
        )
        .unwrap();
        assert_eq!(still.rotation_acceleration_enu_mps2(v_north), DVec3::ZERO);
    }

    #[test]
    fn rejects_bad_inputs_and_round_trips_through_serde() {
        for g_mps2 in [f64::NAN, f64::INFINITY, -9.81] {
            let bad_g = Earth::new(
                NormalGravity::wgs84(),
                site(),
                GravityModel::Constant { g_mps2 },
                EarthRotation::Ignore,
            );
            assert!(bad_g.is_err(), "g = {g_mps2}");
        }
        let bad_site = Geodetic {
            latitude_rad: 2.0,
            ..site()
        };
        assert!(Earth::wgs84(bad_site).is_err());
        let e = earth(GravityModel::Ellipsoidal);
        assert!(e.gravity_enu_mps2(DVec3::new(f64::NAN, 0.0, 0.0)).is_err());

        let json = serde_json::to_string(&e).unwrap();
        let back: Earth = serde_json::from_str(&json).unwrap();
        assert_eq!(back, e);
        let constant = earth(GravityModel::Constant { g_mps2: 9.81 });
        let json = serde_json::to_string(&constant).unwrap();
        assert!(
            json.contains(r#""gravity":{"kind":"constant","g_mps2":9.81}"#),
            "{json}"
        );
    }
}
