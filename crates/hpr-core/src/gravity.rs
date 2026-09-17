//! Normal gravity of a level ellipsoid (WGS 84 by default), and the gravity models the flight
//! engine chooses from.
//!
//! Normal gravity is the gravitational attraction of the reference ellipsoid plus the centrifugal
//! acceleration of the Earth's rotation, so it is the acceleration a body at rest relative to the
//! Earth feels. A simulation in an Earth-fixed frame adds only the Coriolis term
//! ([`crate::earth`]); adding a centrifugal term as well would count it twice.
//!
//! Source: NGA.STND.0036_1.0.0_WGS84, *Department of Defense World Geodetic System 1984*
//! (2014-07-08), chapter 4 and appendix B. Details and tests are in `docs/physics/gravity.md`.

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::geodesy::{Ellipsoid, Geodetic, ecef_from_enu_rotation};

/// Standard acceleration of free fall, `g₀ = 9.80665 m/s²`: the conventional value adopted by
/// the 3rd CGPM (1901) and used by the U.S. Standard Atmosphere, 1976. It is not the gravity at
/// any particular place.
pub const STANDARD_GRAVITY_MPS2: f64 = 9.806_65;

/// WGS 84 geocentric gravitational constant `GM`, m³/s² (NGA.STND.0036, Table 3.1).
pub const WGS84_GM_M3_S2: f64 = 3.986_004_418e14;

/// WGS 84 nominal mean angular velocity of the Earth `ω`, rad/s (NGA.STND.0036, Table 3.1).
pub const WGS84_ANGULAR_VELOCITY_RAD_S: f64 = 7.292_115e-5;

/// `q` and `q′` of eqs. 4-11 and 4-13 as functions of `ε = E/u`:
///
/// ```text
/// q  = ½ [(1 + 3/ε²) atan ε − 3/ε]
/// q′ = 3 (1 + 1/ε²) (1 − atan(ε)/ε) − 1
/// ```
///
/// For the Earth `ε ≤ e′ ≈ 0.082`, where both closed forms cancel about five digits. Below
/// `ε = 0.5` the Taylor series of `atan ε` gives them without cancellation (substitute
/// `atan ε = Σ (−1)ⁿ ε^(2n+1)/(2n+1)` and collect powers):
///
/// ```text
/// q  = Σ_{n≥1} (−1)^(n+1) 2n ε^(2n+1) / ((2n+1)(2n+3))
/// q′ = Σ_{n≥1} (−1)^(n+1) 6 ε^(2n)    / ((2n+1)(2n+3))
/// ```
fn q_and_q_prime(eps: f64) -> (f64, f64) {
    if eps >= 0.5 {
        let atan_eps = eps.atan();
        let q = 0.5 * ((1.0 + 3.0 / (eps * eps)) * atan_eps - 3.0 / eps);
        let q_prime = 3.0 * (1.0 + 1.0 / (eps * eps)) * (1.0 - atan_eps / eps) - 1.0;
        return (q, q_prime);
    }
    let t = eps * eps;
    let (mut q, mut q_prime) = (0.0, 0.0);
    // (−1)^(n+1) ε^(2n); at ε < 0.5 each term is under a quarter of the last, so 40 terms reach
    // far below rounding.
    let mut power = t;
    for n in 1..=40 {
        let n = f64::from(n);
        let denominator = (2.0 * n + 1.0) * (2.0 * n + 3.0);
        q += 2.0 * n * power * eps / denominator;
        q_prime += 6.0 * power / denominator;
        power *= -t;
    }
    (q, q_prime)
}

/// The normal gravity field of a level ellipsoid, fixed by four defining parameters: `a`, `1/f`,
/// `GM` and `ω`. Everything else is derived from them (NGA.STND.0036 appendix B).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "NormalGravityData", into = "NormalGravityData")]
pub struct NormalGravity {
    ellipsoid: Ellipsoid,
    gm_m3_s2: f64,
    omega_rad_s: f64,
    /// `q₀` (eq. B-18).
    q0: f64,
    /// `m = ω²a²b/GM` (eq. B-20).
    m: f64,
    /// Normal gravity at the equator `γ_e`, m/s² (eq. B-24).
    gamma_e: f64,
    /// Normal gravity at the poles `γ_p`, m/s² (eq. B-25).
    gamma_p: f64,
    /// Somigliana's constant `k` (eq. B-26).
    k: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalGravityData {
    ellipsoid: Ellipsoid,
    gm_m3_s2: f64,
    angular_velocity_rad_s: f64,
}

impl TryFrom<NormalGravityData> for NormalGravity {
    type Error = CoreError;

    fn try_from(data: NormalGravityData) -> Result<Self, CoreError> {
        NormalGravity::new(data.ellipsoid, data.gm_m3_s2, data.angular_velocity_rad_s)
    }
}

impl From<NormalGravity> for NormalGravityData {
    fn from(field: NormalGravity) -> Self {
        NormalGravityData {
            ellipsoid: field.ellipsoid,
            gm_m3_s2: field.gm_m3_s2,
            angular_velocity_rad_s: field.omega_rad_s,
        }
    }
}

impl NormalGravity {
    /// The field of `ellipsoid` with geocentric gravitational constant `gm_m3_s2` and angular
    /// velocity `omega_rad_s`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] unless the ellipsoid is oblate (the ellipsoidal-harmonic formulas
    /// divide by its linear eccentricity), `GM` is finite and positive, and `ω` is finite and not
    /// negative.
    pub fn new(ellipsoid: Ellipsoid, gm_m3_s2: f64, omega_rad_s: f64) -> Result<Self, CoreError> {
        if ellipsoid.flattening() <= 0.0 {
            return Err(CoreError::Domain {
                what: "flattening of a normal gravity ellipsoid",
                value: ellipsoid.flattening(),
            });
        }
        if !(gm_m3_s2.is_finite() && gm_m3_s2 > 0.0) {
            return Err(CoreError::Domain {
                what: "GM (m³/s²)",
                value: gm_m3_s2,
            });
        }
        if !(omega_rad_s.is_finite() && omega_rad_s >= 0.0) {
            return Err(CoreError::Domain {
                what: "angular velocity (rad/s)",
                value: omega_rad_s,
            });
        }
        Ok(Self::derive(ellipsoid, gm_m3_s2, omega_rad_s))
    }

    /// The WGS 84 normal gravity field.
    #[must_use]
    pub fn wgs84() -> Self {
        Self::derive(
            Ellipsoid::WGS84,
            WGS84_GM_M3_S2,
            WGS84_ANGULAR_VELOCITY_RAD_S,
        )
    }

    /// The derived constants of appendix B, from validated defining parameters.
    fn derive(ellipsoid: Ellipsoid, gm: f64, omega: f64) -> Self {
        let a = ellipsoid.semi_major_axis_m();
        let b = ellipsoid.semi_minor_axis_m();
        // Second eccentricity e′ = E/b; (B-18) and (B-19) are q and q′ at u = b.
        let ep = ellipsoid.linear_eccentricity_m() / b;
        let (q0, q0_prime) = q_and_q_prime(ep);
        let m = omega * omega * a * a * b / gm; // (B-20)
        let ratio = m * ep * q0_prime / q0;
        let gamma_e = gm / (a * b) * (1.0 - m - ratio / 6.0); // (B-24)
        let gamma_p = gm / (a * a) * (1.0 + ratio / 3.0); // (B-25)
        let k = (b * gamma_p - a * gamma_e) / (a * gamma_e); // (B-26)
        Self {
            ellipsoid,
            gm_m3_s2: gm,
            omega_rad_s: omega,
            q0,
            m,
            gamma_e,
            gamma_p,
            k,
        }
    }

    /// The reference ellipsoid.
    #[must_use]
    pub fn ellipsoid(&self) -> Ellipsoid {
        self.ellipsoid
    }

    /// Geocentric gravitational constant `GM`, m³/s².
    #[must_use]
    pub fn gm_m3_s2(&self) -> f64 {
        self.gm_m3_s2
    }

    /// The Earth's angular velocity `ω`, rad/s.
    #[must_use]
    pub fn angular_velocity_rad_s(&self) -> f64 {
        self.omega_rad_s
    }

    /// Normal gravity at the equator on the ellipsoid, `γ_e`, m/s² (eq. B-24).
    #[must_use]
    pub fn equatorial_gravity_mps2(&self) -> f64 {
        self.gamma_e
    }

    /// Normal gravity at the poles on the ellipsoid, `γ_p`, m/s² (eq. B-25).
    #[must_use]
    pub fn polar_gravity_mps2(&self) -> f64 {
        self.gamma_p
    }

    /// Somigliana's constant `k = bγ_p/(aγ_e) − 1` (eq. B-26).
    #[must_use]
    pub fn somigliana_constant(&self) -> f64 {
        self.k
    }

    /// `m = ω²a²b/GM` (eq. B-20).
    #[must_use]
    pub fn m(&self) -> f64 {
        self.m
    }

    /// Normal gravity on the ellipsoid at geodetic latitude `φ`, by Somigliana's closed formula
    /// (eq. 4-1), m/s²:
    ///
    /// ```text
    /// γ = γ_e (1 + k sin²φ) / √(1 − e² sin²φ)
    /// ```
    #[must_use]
    pub fn surface_mps2(&self, latitude_rad: f64) -> f64 {
        let s2 = latitude_rad.sin().powi(2);
        self.gamma_e * (1.0 + self.k * s2)
            / (1.0 - self.ellipsoid.eccentricity_squared() * s2).sqrt()
    }

    /// Magnitude of normal gravity at geodetic latitude `φ` and ellipsoidal height `h` by the
    /// truncated Taylor series (eq. 4-3), m/s²:
    ///
    /// ```text
    /// γ_h = γ [1 − (2/a)(1 + f + m − 2f sin²φ) h + (3/a²) h²]
    /// ```
    ///
    /// RocketPy uses this form. It drifts from the exact field with height: about 3e-7 relative
    /// at 30 km and 1.4e-5 at 100 km (`docs/physics/gravity.md`). Prefer
    /// [`NormalGravity::enu_at_mps2`] unless matching an oracle that uses it.
    #[must_use]
    pub fn taylor_mps2(&self, latitude_rad: f64, height_m: f64) -> f64 {
        let a = self.ellipsoid.semi_major_axis_m();
        let f = self.ellipsoid.flattening();
        let s2 = latitude_rad.sin().powi(2);
        self.surface_mps2(latitude_rad)
            * (1.0 - 2.0 / a * (1.0 + f + self.m - 2.0 * f * s2) * height_m
                + 3.0 / (a * a) * height_m * height_m)
    }

    /// The normal gravity vector at an ECEF position, resolved in ECEF, m/s². Exact closed form
    /// in ellipsoidal-harmonic coordinates `(u, β)` (eqs. 4-5 to 4-13), rotated to Cartesian
    /// components by `R₁` (eq. 4-18):
    ///
    /// ```text
    /// u² = ½ [s + √(s² + 4E²z²)],   s = x² + y² + z² − E²                     (4-8)
    /// β  = atan2(z √(u² + E²), u √(x² + y²))                                   (4-9)
    /// w  = √((u² + E² sin²β)/(u² + E²))                                         (4-10)
    /// q  = ½ [(1 + 3u²/E²) atan(E/u) − 3u/E]                                    (4-11)
    /// q′ = 3 (1 + u²/E²) [1 − (u/E) atan(E/u)] − 1                              (4-13)
    /// γ_u = −(1/w) [GM/(u² + E²) + ω²a²E/(u² + E²) (q′/q₀)(½ sin²β − 1/6)] + (1/w) ω² u cos²β
    /// γ_β = (1/w) ω²a²/√(u² + E²) (q/q₀) sin β cos β − (1/w) ω² √(u² + E²) sin β cos β
    /// ```
    ///
    /// Equation 4-8 is written in the algebraically equivalent form above, which has no
    /// division by `s`.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if the position is not finite or lies on the focal disc (`u = 0`:
    /// the equatorial plane within `E`, about 522 km, of the centre).
    pub fn ecef_mps2(&self, position_ecef_m: DVec3) -> Result<DVec3, CoreError> {
        let a = self.ellipsoid.semi_major_axis_m();
        let big_e = self.ellipsoid.linear_eccentricity_m();
        let e2_lin = big_e * big_e;
        let DVec3 { x, y, z } = position_ecef_m;
        let p = x.hypot(y);
        let s = x * x + y * y + z * z - e2_lin;
        let u2 = 0.5 * (s + (s * s + 4.0 * e2_lin * z * z).sqrt());
        let u = u2.sqrt();
        if !(u > 0.0 && u.is_finite() && p.is_finite()) {
            return Err(CoreError::Domain {
                what: "distance from the Earth's centre for normal gravity (m)",
                value: position_ecef_m.length(),
            });
        }
        let big_u2 = u2 + e2_lin;
        let big_u = big_u2.sqrt();
        let beta = (z * big_u).atan2(u * p);
        let (sin_b, cos_b) = beta.sin_cos();
        let w = ((u2 + e2_lin * sin_b * sin_b) / big_u2).sqrt();
        let (q, q_prime) = q_and_q_prime(big_e / u);
        let omega2 = self.omega_rad_s * self.omega_rad_s;

        let gamma_u = -(self.gm_m3_s2 / big_u2
            + omega2 * a * a * big_e / big_u2
                * (q_prime / self.q0)
                * (0.5 * sin_b * sin_b - 1.0 / 6.0))
            / w
            + omega2 * u * cos_b * cos_b / w;
        let gamma_beta =
            (omega2 * a * a / big_u * (q / self.q0) - omega2 * big_u) * sin_b * cos_b / w;

        // Longitude direction; arbitrary on the axis, where γ_β vanishes.
        let (cos_l, sin_l) = if p > 0.0 { (x / p, y / p) } else { (1.0, 0.0) };
        let c = u / (w * big_u);
        let gamma = DVec3::new(
            c * cos_b * cos_l * gamma_u - sin_b * cos_l / w * gamma_beta,
            c * cos_b * sin_l * gamma_u - sin_b * sin_l / w * gamma_beta,
            sin_b / w * gamma_u + c * cos_b * gamma_beta,
        );
        Ok(gamma)
    }

    /// The normal gravity vector at a geodetic position, resolved in that point's local
    /// East-North-Up axes, m/s². `−z` is the exact normal component `γ_h` (eq. 4-16), `y` is
    /// `γ_φ` (eq. 4-23, positive north) and `x` is zero up to rounding; the length is
    /// `|γ_total|` (eq. 4-4).
    ///
    /// # Errors
    ///
    /// As [`NormalGravity::ecef_mps2`], which cannot fail for heights above −6000 km.
    pub fn enu_at_mps2(&self, point: Geodetic) -> Result<DVec3, CoreError> {
        let gamma = self.ecef_mps2(self.ellipsoid.ecef_from_geodetic(point))?;
        Ok(ecef_from_enu_rotation(point).transpose() * gamma)
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct Fixture {
        constants: Constants,
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Constants {
        gamma_e_mps2: f64,
        gamma_p_mps2: f64,
        k: f64,
        m: f64,
    }

    #[derive(Deserialize)]
    struct Case {
        latitude_deg: f64,
        longitude_deg: f64,
        height_m: f64,
        surface_mps2: f64,
        taylor_mps2: f64,
        magnitude_mps2: f64,
        down_mps2: f64,
        north_mps2: f64,
        ecef_mps2: [f64; 3],
    }

    /// Values of the published formulas at 40 digits, from
    /// `validation/oracles/wgs84/normal_gravity.py`.
    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../validation/fixtures/earth/wgs84-normal-gravity.json"
        ))
        .unwrap()
    }

    fn relative_error(value: f64, reference: f64) -> f64 {
        ((value - reference) / reference).abs()
    }

    fn point(case: &Case) -> Geodetic {
        Geodetic::from_degrees(case.latitude_deg, case.longitude_deg, case.height_m).unwrap()
    }

    /// Loft lesson L1 and the M1.1 *done when*: WGS 84 Somigliana gravity with altitude matches
    /// the published formula values at 11 latitude/height points to 1e-6 relative. The
    /// implementation actually agrees to about 1e-13; the tighter bounds below pin that.
    ///
    /// References: `γ_e` and `γ_p` as printed in NGA.STND.0036 Table 3.6, and the published
    /// formulas (eqs. 4-1, 4-3, 4-4, 4-16, 4-23) evaluated at 40 digits by an independent script
    /// that also checks the vector against the gradient of the normal potential.
    #[test]
    fn somigliana_matches_published_values() {
        let field = NormalGravity::wgs84();
        let fixture = fixture();

        // Table 3.6, to its printed digits.
        assert!((field.equatorial_gravity_mps2() - 9.780_325_335_9).abs() < 5e-11);
        assert!((field.polar_gravity_mps2() - 9.832_184_937_9).abs() < 5e-11);
        assert!((field.somigliana_constant() - 1.931_852_652_458e-3).abs() < 5e-16);
        assert!((field.m() - 3.449_786_506_841e-3).abs() < 5e-16);
        // The 40-digit derivation.
        let c = &fixture.constants;
        assert!(relative_error(field.equatorial_gravity_mps2(), c.gamma_e_mps2) < 1e-14);
        assert!(relative_error(field.polar_gravity_mps2(), c.gamma_p_mps2) < 1e-14);
        assert!(relative_error(field.somigliana_constant(), c.k) < 1e-12);
        assert!(relative_error(field.m(), c.m) < 1e-14);

        assert!(fixture.cases.len() >= 6);
        for case in &fixture.cases {
            let at = format!(
                "({}°, {}°, {} m)",
                case.latitude_deg, case.longitude_deg, case.height_m
            );
            let g = point(case);
            let surface = field.surface_mps2(g.latitude_rad);
            let taylor = field.taylor_mps2(g.latitude_rad, g.height_m);
            let enu = field.enu_at_mps2(g).unwrap();
            let ecef = field
                .ecef_mps2(field.ellipsoid().ecef_from_geodetic(g))
                .unwrap();

            // The done-when bound first, then the tighter implementation bound.
            for (name, value, reference) in [
                ("surface (4-1)", surface, case.surface_mps2),
                ("Taylor (4-3)", taylor, case.taylor_mps2),
                ("|γ| (4-4)", enu.length(), case.magnitude_mps2),
                ("γ_h (4-16)", -enu.z, case.down_mps2),
            ] {
                let error = relative_error(value, reference);
                assert!(error <= 1e-6, "{name} at {at}: {value} vs {reference}");
                assert!(error <= 2e-14, "{name} at {at}: relative error {error:e}");
            }
            // The horizontal component is tiny, so compare it absolutely.
            assert!(
                (enu.y - case.north_mps2).abs() < 1e-12,
                "γ_φ at {at}: {}",
                enu.y
            );
            assert!(enu.x.abs() < 1e-12, "east component at {at}: {}", enu.x);
            for (value, reference) in ecef.to_array().into_iter().zip(case.ecef_mps2) {
                assert!(
                    (value - reference).abs() < 2e-13,
                    "ECEF at {at}: {value} vs {reference}"
                );
            }
        }
    }

    /// The Taylor series agrees with RocketPy 1.13.0's formula, which uses the same equation with
    /// Table 3.6 constants rounded to 13 digits (`validation/oracles/rocketpy/gravity.py`).
    #[test]
    fn taylor_series_matches_the_rocketpy_oracle() {
        #[derive(Deserialize)]
        struct Oracle {
            cases: Vec<OracleCase>,
        }
        #[derive(Deserialize)]
        struct OracleCase {
            latitude_deg: f64,
            height_m: f64,
            formula_mps2: f64,
        }
        let oracle: Oracle = serde_json::from_str(include_str!(
            "../../../validation/fixtures/earth/rocketpy-gravity.json"
        ))
        .unwrap();
        let field = NormalGravity::wgs84();
        assert!(oracle.cases.len() >= 6);
        for case in &oracle.cases {
            let value = field.taylor_mps2(case.latitude_deg.to_radians(), case.height_m);
            let error = relative_error(value, case.formula_mps2);
            assert!(
                error < 1e-12,
                "({}°, {} m): {value} vs {}",
                case.latitude_deg,
                case.height_m,
                case.formula_mps2
            );
        }
    }

    /// On the ellipsoid the exact field reduces to Somigliana's formula and points straight down.
    #[test]
    fn exact_field_reduces_to_somigliana_on_the_ellipsoid() {
        let field = NormalGravity::wgs84();
        for k in -18..=18 {
            let lat = f64::from(k) * 5.0;
            let g = Geodetic::from_degrees(lat, 17.0 * f64::from(k), 0.0).unwrap();
            let enu = field.enu_at_mps2(g).unwrap();
            let surface = field.surface_mps2(g.latitude_rad);
            assert!(relative_error(-enu.z, surface) < 1e-14, "lat {lat}");
            assert!(enu.truncate().length() < 1e-12, "lat {lat}: {enu}");
        }
    }

    /// The series and closed forms of `q` and `q′` agree where they meet, and the series
    /// reproduces the printed values of `q₀` and `q₀′` (NGA.STND.0036 eqs. B-18 and B-19).
    #[test]
    fn q_functions_match_appendix_b() {
        let ellipsoid = Ellipsoid::WGS84;
        let ep = ellipsoid.linear_eccentricity_m() / ellipsoid.semi_minor_axis_m();
        let (q0, q0_prime) = q_and_q_prime(ep);
        assert!((q0 - 7.334_625_787_083e-5).abs() < 5e-18, "q0 = {q0:e}");
        assert!(
            (q0_prime - 2.688_041_300_461e-3).abs() < 5e-16,
            "q0' = {q0_prime:e}"
        );
        let (below, below_prime) = q_and_q_prime(0.5 - 1e-15);
        let (above, above_prime) = q_and_q_prime(0.5);
        assert!(relative_error(below, above) < 1e-13);
        assert!(relative_error(below_prime, above_prime) < 1e-13);
    }

    #[test]
    fn rejects_invalid_fields_and_positions() {
        let sphere = Ellipsoid::new(6.371e6, f64::INFINITY).unwrap();
        assert!(NormalGravity::new(sphere, WGS84_GM_M3_S2, 0.0).is_err());
        assert!(NormalGravity::new(Ellipsoid::WGS84, -1.0, 0.0).is_err());
        assert!(NormalGravity::new(Ellipsoid::WGS84, WGS84_GM_M3_S2, f64::NAN).is_err());
        assert_eq!(
            NormalGravity::new(
                Ellipsoid::WGS84,
                WGS84_GM_M3_S2,
                WGS84_ANGULAR_VELOCITY_RAD_S
            ),
            Ok(NormalGravity::wgs84())
        );
        let field = NormalGravity::wgs84();
        assert!(field.ecef_mps2(DVec3::ZERO).is_err());
        assert!(field.ecef_mps2(DVec3::new(1.0e5, 0.0, 0.0)).is_err());
        assert!(
            field
                .ecef_mps2(DVec3::new(f64::INFINITY, 0.0, 0.0))
                .is_err()
        );
    }

    #[test]
    fn serde_round_trip_keeps_the_defining_parameters() {
        let field = NormalGravity::wgs84();
        let json = serde_json::to_string(&field).unwrap();
        let back: NormalGravity = serde_json::from_str(&json).unwrap();
        assert_eq!(back, field);
    }
}
