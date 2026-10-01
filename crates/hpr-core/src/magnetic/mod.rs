//! The Earth's main magnetic field from the World Magnetic Model, WMM2025: declination (the angle
//! from true north to magnetic north), inclination, intensity and their yearly change.
//!
//! A compass points along the horizontal part of the field, not at true north. Declination `D`
//! is the angle between the two, positive when magnetic north lies east of true north, so a
//! magnetic heading converts to a true one by adding `D`.
//!
//! Source: A. Chulliat, W. Brown, M. Nair et al., *The US/UK World Magnetic Model for 2025–2030:
//! Technical Report*, NCEI, NOAA, 2025, <https://doi.org/10.25923/prbc-s316>, section 1.2,
//! equations 3 to 20. The coefficients are NCEI's `WMM2025.COF`, in the public domain. Details
//! and tests are in the guide's [magnetic-field page][guide].
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/physics/magnetic.html
//!
//! The model is valid from 2025.0 to 2030.0 and from 1 km below the WGS 84 ellipsoid to 850 km
//! above it (report, sections 1.3 and 3); [`MagneticModel::field`] refuses anything outside.
//!
//! Two departures from the report's printed text, both checked by NOAA's test values:
//!
//! - Equation 15 prints `ġ cos mλ − ḣ sin mλ` for the rate of `Z′`. The potential (equation 4)
//!   and equation 12 give `+`, and NOAA's 100 test values of `Ż` need `+`.
//! - Equation 16, the derivative of `P̆ₙᵐ`, and equation 11 divide by `cos φ′`, which is zero at a
//!   pole. Here every Legendre function is written `P̆ₙᵐ = cₙₘ cosᵐφ′ qₙᵐ(sin φ′)`, with
//!   `qₙᵐ = dᵐPₙ/dμᵐ` a polynomial, so both divisions cancel exactly and the field is finite at
//!   the poles (report, section 1.4).

mod coefficients;
#[cfg(test)]
mod tests;

use glam::DVec3;
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::geodesy::{Ellipsoid, Geodetic};

/// The geomagnetic reference radius `a`, m (report, equation 4).
pub const REFERENCE_RADIUS_M: f64 = 6_371_200.0;

/// The highest degree of the WMM's expansion, `N = 12`.
const DEGREE: usize = 12;

/// A spherical-harmonic model of the main field: Gauss coefficients at an epoch and their linear
/// secular variation, to degree 12.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MagneticModel {
    name: &'static str,
    epoch_year: f64,
    valid_until_year: f64,
    coefficients: &'static [[f64; 4]; 90],
}

/// WMM2025, the World Magnetic Model for 2025.0 to 2030.0 (NOAA NCEI and the British Geological
/// Survey, released 2024-12-17).
pub const WMM2025: MagneticModel = MagneticModel {
    name: "WMM-2025",
    epoch_year: 2025.0,
    valid_until_year: 2030.0,
    coefficients: &coefficients::WMM2025,
};

/// The lowest height the WMM is specified for: 1 km below the WGS 84 ellipsoid (report, section
/// 3, after MIL-PRF-89500B).
pub const MIN_HEIGHT_M: f64 = -1_000.0;

/// The highest height the WMM is specified for: 850 km above the WGS 84 ellipsoid (report,
/// section 3, after MIL-PRF-89500B).
pub const MAX_HEIGHT_M: f64 = 850_000.0;

/// Below this horizontal intensity, 6,000 nT, the report's caution zone around a magnetic pole
/// begins (report, section 1.8).
pub const CAUTION_HORIZONTAL_NT: f64 = 6_000.0;

/// Below this horizontal intensity, 2,000 nT, the report's blackout zone begins: declination can
/// be wrong by up to 180° (report, section 1.8).
pub const BLACKOUT_HORIZONTAL_NT: f64 = 2_000.0;

/// How far a compass, and so the declination, can be trusted at a place (report, section 1.8).
///
/// The report draws the zones on the ellipsoid's surface; at height the horizontal intensity
/// weakens (to about 0.7 of the surface's at 850 km), so the zones there are wider than the
/// report's. At a rocket's heights the difference is negligible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CompassZone {
    /// Horizontal intensity of at least 6,000 nT.
    Reliable,
    /// Horizontal intensity from 2,000 to 6,000 nT: approaching a magnetic pole, where
    /// declination errors exceed 1°.
    Caution,
    /// Horizontal intensity below 2,000 nT: near a magnetic pole, where declination errors of up
    /// to 180° occur.
    Blackout,
}

/// The magnetic elements at one place and time, and their rates of change.
///
/// Components are in the local geodetic north-east-down frame of the WGS 84 ellipsoid, as the
/// report gives them (hpr's launch frame is east-north-up: see [`MagneticField::enu_nt`]); angles
/// are in radians; rates are per year.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MagneticField {
    /// `X`, the northward component, nT.
    pub north_nt: f64,
    /// `Y`, the eastward component, nT.
    pub east_nt: f64,
    /// `Z`, the downward component, nT.
    pub down_nt: f64,
    /// `H = √(X² + Y²)`, the horizontal intensity, nT.
    pub horizontal_nt: f64,
    /// `F = √(H² + Z²)`, the total intensity, nT.
    pub total_nt: f64,
    /// `I = atan2(Z, H)`, the inclination or dip, rad, positive down.
    pub inclination_rad: f64,
    /// `D = atan2(Y, X)`, the declination, rad, positive east of true north.
    pub declination_rad: f64,
    /// The grid variation poleward of 55° (report, equation 1): `D − λ` north of 55° N, `D + λ`
    /// south of 55° S, in `(−π, π]`; `None` elsewhere.
    pub grid_variation_rad: Option<f64>,
    /// `Ẋ`, nT per year.
    pub north_rate_nt_per_year: f64,
    /// `Ẏ`, nT per year.
    pub east_rate_nt_per_year: f64,
    /// `Ż`, nT per year.
    pub down_rate_nt_per_year: f64,
    /// `Ḣ = (X Ẋ + Y Ẏ) / H`, nT per year.
    pub horizontal_rate_nt_per_year: f64,
    /// `Ḟ = (X Ẋ + Y Ẏ + Z Ż) / F`, nT per year.
    pub total_rate_nt_per_year: f64,
    /// `İ = (H Ż − Z Ḣ) / F²`, rad per year.
    pub inclination_rate_rad_per_year: f64,
    /// `Ḋ = (X Ẏ − Y Ẋ) / H²`, rad per year. The grid variation changes at the same rate.
    pub declination_rate_rad_per_year: f64,
}

impl MagneticField {
    /// The field as east, north and up components, nT: `(Y, X, −Z)`, the axes of hpr's launch
    /// frame (`docs/physics/frames.md`).
    #[must_use]
    pub fn enu_nt(&self) -> DVec3 {
        DVec3::new(self.east_nt, self.north_nt, -self.down_nt)
    }

    /// The compass zone at this place, from the horizontal intensity (report, section 1.8). A
    /// horizontal intensity that is not a number counts as the blackout zone.
    #[must_use]
    pub fn compass_zone(&self) -> CompassZone {
        // Written so that NaN fails closed, into the blackout zone.
        if !(self.horizontal_nt >= BLACKOUT_HORIZONTAL_NT) {
            CompassZone::Blackout
        } else if self.horizontal_nt < CAUTION_HORIZONTAL_NT {
            CompassZone::Caution
        } else {
            CompassZone::Reliable
        }
    }

    /// The model's own estimate of its declination error, one standard deviation, rad (report,
    /// section 3.4, equation 43):
    ///
    /// ```text
    /// δD = √(0.26² + (5417 / H)²)   degrees, with H in nT
    /// ```
    ///
    /// About 0.29° where the field is strongest and growing without bound toward a magnetic pole.
    /// It covers the coefficients' and the forecast's errors, the crust's local fields the model
    /// leaves out, and magnetic storms; a steel rail or car beside a compass adds its own. The
    /// report fits it on the ellipsoid's surface; at height it uses that height's `H`, which is
    /// weaker, so the estimate grows a little (negligibly at a rocket's heights).
    #[must_use]
    pub fn declination_uncertainty_rad(&self) -> f64 {
        0.26_f64.hypot(5_417.0 / self.horizontal_nt).to_radians()
    }

    /// A true bearing from a magnetic one: `true = magnetic + D`, rad, in `[0, 2π)`.
    #[must_use]
    pub fn true_from_magnetic_rad(&self, magnetic_bearing_rad: f64) -> f64 {
        (magnetic_bearing_rad + self.declination_rad).rem_euclid(std::f64::consts::TAU)
    }
}

/// The geocentric field components `(X′, Y′, Z′)` of one set of coefficients, nT or nT per year
/// (report, equations 10 to 15).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geocentric {
    north: f64,
    east: f64,
    down: f64,
}

/// What the spherical harmonic sums need at a point, shared by the field and its rate.
#[derive(Debug)]
struct Harmonics {
    /// `P̆ₙᵐ(sin φ′)`, by `[n][m]`.
    p: [[f64; DEGREE + 1]; DEGREE + 1],
    /// `dP̆ₙᵐ(sin φ′)/dφ′`, by `[n][m]`.
    dp: [[f64; DEGREE + 1]; DEGREE + 1],
    /// `m P̆ₙᵐ(sin φ′) / cos φ′`, by `[n][m]`, finite at the poles.
    p_over_cos: [[f64; DEGREE + 1]; DEGREE + 1],
    /// `(a/r)^(n+2)`, by `n`.
    radial: [f64; DEGREE + 1],
    /// `cos mλ` and `sin mλ`, by `m`.
    cos_m: [f64; DEGREE + 1],
    sin_m: [f64; DEGREE + 1],
}

impl Harmonics {
    #[expect(
        clippy::needless_range_loop,
        reason = "the recurrences index rows by degree and order, as the equations do"
    )]
    fn new(geocentric_latitude_rad: f64, radius_m: f64, longitude_rad: f64) -> Self {
        let (mu, cos_lat) = geocentric_latitude_rad.sin_cos();
        // cos φ′ ≥ 0 on [−π/2, π/2]; the clamp only removes a rounding below zero.
        let s = cos_lat.max(0.0);

        // q[n][m] = dᵐPₙ/dμᵐ, zero for m > n. In n at fixed m it obeys the recurrence of the
        // associated Legendre functions (DLMF 14.10.3), (n − m) qₙᵐ = (2n − 1) μ qₙ₋₁ᵐ −
        // (n + m − 1) qₙ₋₂ᵐ, divided through by (1 − μ²)^(m/2); it starts from qₘᵐ = (2m)!/(2ᵐ m!)
        // = (2m − 1)!!, the m-th derivative of Pₘ's leading term (Rodrigues' formula, DLMF 18.5.5).
        let mut q = [[0.0_f64; DEGREE + 2]; DEGREE + 1];
        let mut double_factorial = 1.0;
        for m in 0..=DEGREE {
            if m > 0 {
                double_factorial *= (2 * m - 1) as f64;
            }
            q[m][m] = double_factorial;
            for n in (m + 1)..=DEGREE {
                let previous = q[n - 1][m];
                let before = if n >= m + 2 { q[n - 2][m] } else { 0.0 };
                q[n][m] = ((2 * n - 1) as f64 * mu * previous - (n + m - 1) as f64 * before)
                    / (n - m) as f64;
            }
        }

        let mut p = [[0.0; DEGREE + 1]; DEGREE + 1];
        let mut dp = [[0.0; DEGREE + 1]; DEGREE + 1];
        let mut p_over_cos = [[0.0; DEGREE + 1]; DEGREE + 1];
        for n in 1..=DEGREE {
            for m in 0..=n {
                // Schmidt semi-normalization (report, equation 5): √(2 (n − m)! / (n + m)!) for
                // m > 0, and 1 for m = 0.
                let norm = if m == 0 {
                    1.0
                } else {
                    let ratio: f64 = ((n - m + 1)..=(n + m)).map(|k| k as f64).product();
                    (2.0 / ratio).sqrt()
                };
                let s_m = s.powi(m as i32);
                p[n][m] = norm * s_m * q[n][m];
                // d/dφ′ [cosᵐφ′ qₙᵐ(sin φ′)] = cosᵐ⁺¹φ′ qₙᵐ⁺¹ − m sin φ′ cosᵐ⁻¹φ′ qₙᵐ.
                let s_m_minus_1 = if m == 0 { 0.0 } else { s.powi(m as i32 - 1) };
                dp[n][m] = norm * (s_m * s * q[n][m + 1] - m as f64 * mu * s_m_minus_1 * q[n][m]);
                p_over_cos[n][m] = norm * m as f64 * s_m_minus_1 * q[n][m];
            }
        }

        let ratio = REFERENCE_RADIUS_M / radius_m;
        let mut radial = [0.0; DEGREE + 1];
        let mut power = ratio * ratio;
        for value in &mut radial[1..] {
            power *= ratio;
            *value = power;
        }

        let mut cos_m = [0.0; DEGREE + 1];
        let mut sin_m = [0.0; DEGREE + 1];
        for m in 0..=DEGREE {
            let (sin, cos) = (m as f64 * longitude_rad).sin_cos();
            cos_m[m] = cos;
            sin_m[m] = sin;
        }

        Self {
            p,
            dp,
            p_over_cos,
            radial,
            cos_m,
            sin_m,
        }
    }

    /// Equations 10 to 12 (and 13 to 15 with the rates as coefficients), for coefficients given
    /// as `(g, h)` by `[n][m]`.
    fn sum(
        &self,
        g: &[[f64; DEGREE + 1]; DEGREE + 1],
        h: &[[f64; DEGREE + 1]; DEGREE + 1],
    ) -> Geocentric {
        let (mut north, mut east, mut down) = (0.0, 0.0, 0.0);
        for n in 1..=DEGREE {
            let (mut sum_north, mut sum_east, mut sum_down) = (0.0, 0.0, 0.0);
            for m in 0..=n {
                let cos_term = g[n][m] * self.cos_m[m] + h[n][m] * self.sin_m[m];
                let sin_term = g[n][m] * self.sin_m[m] - h[n][m] * self.cos_m[m];
                sum_north += cos_term * self.dp[n][m];
                sum_east += sin_term * self.p_over_cos[n][m];
                sum_down += cos_term * self.p[n][m];
            }
            north -= self.radial[n] * sum_north;
            east += self.radial[n] * sum_east;
            down -= (n + 1) as f64 * self.radial[n] * sum_down;
        }
        Geocentric { north, east, down }
    }
}

/// The geocentric latitude `φ′` and radius `r` of a geodetic point on WGS 84 (report, equations
/// 7 and 8).
fn geocentric(point: Geodetic) -> (f64, f64) {
    let ellipsoid = Ellipsoid::WGS84;
    let e2 = ellipsoid.eccentricity_squared();
    let rc = ellipsoid.prime_vertical_radius_m(point.latitude_rad);
    let (sin_lat, cos_lat) = point.latitude_rad.sin_cos();
    let p = (rc + point.height_m) * cos_lat;
    let z = (rc * (1.0 - e2) + point.height_m) * sin_lat;
    let r = p.hypot(z);
    ((z / r).asin(), r)
}

/// Wraps an angle into `(−π, π]`. `rem_euclid` is exact for any finite angle.
fn wrap_pi(angle_rad: f64) -> f64 {
    let wrapped = angle_rad.rem_euclid(std::f64::consts::TAU);
    if wrapped > std::f64::consts::PI {
        wrapped - std::f64::consts::TAU
    } else {
        wrapped
    }
}

impl MagneticModel {
    /// The model's name, as its coefficient file's header gives it.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The epoch `t₀` of the main-field coefficients, decimal year.
    #[must_use]
    pub fn epoch_year(&self) -> f64 {
        self.epoch_year
    }

    /// The end of the model's validity, decimal year.
    #[must_use]
    pub fn valid_until_year(&self) -> f64 {
        self.valid_until_year
    }

    /// The Gauss coefficients and their rates at decimal year `t` (report, equation 9):
    /// `g(t) = g(t₀) + (t − t₀) ġ`, and the same for `h`.
    #[expect(
        clippy::type_complexity,
        reason = "four coefficient tables by [n][m], private to this module"
    )]
    fn coefficients_at(
        &self,
        decimal_year: f64,
    ) -> (
        [[f64; DEGREE + 1]; DEGREE + 1],
        [[f64; DEGREE + 1]; DEGREE + 1],
        [[f64; DEGREE + 1]; DEGREE + 1],
        [[f64; DEGREE + 1]; DEGREE + 1],
    ) {
        let dt = decimal_year - self.epoch_year;
        let mut g = [[0.0; DEGREE + 1]; DEGREE + 1];
        let mut h = [[0.0; DEGREE + 1]; DEGREE + 1];
        let mut g_dot = [[0.0; DEGREE + 1]; DEGREE + 1];
        let mut h_dot = [[0.0; DEGREE + 1]; DEGREE + 1];
        for n in 1..=DEGREE {
            for m in 0..=n {
                // Row n(n + 1)/2 − 1 + m holds (n, m): the 90 rows of n = 1 to 12, m = 0 to n, in
                // order, so the index is below 90 by construction.
                let [g0, h0, gd, hd] = self.coefficients[n * (n + 1) / 2 - 1 + m];
                g[n][m] = g0 + dt * gd;
                h[n][m] = h0 + dt * hd;
                g_dot[n][m] = gd;
                h_dot[n][m] = hd;
            }
        }
        (g, h, g_dot, h_dot)
    }

    /// The magnetic elements at `point` (geodetic, on WGS 84, height above the ellipsoid) at time
    /// `decimal_year` (report, section 1.2, equations 7 to 20).
    ///
    /// The geocentric components are
    ///
    /// ```text
    /// X′ = −Σₙ (a/r)ⁿ⁺² Σₘ (gₙᵐ cos mλ + hₙᵐ sin mλ) dP̆ₙᵐ(sin φ′)/dφ′
    /// Y′ = (1/cos φ′) Σₙ (a/r)ⁿ⁺² Σₘ m (gₙᵐ sin mλ − hₙᵐ cos mλ) P̆ₙᵐ(sin φ′)
    /// Z′ = −Σₙ (n + 1)(a/r)ⁿ⁺² Σₘ (gₙᵐ cos mλ + hₙᵐ sin mλ) P̆ₙᵐ(sin φ′)
    /// ```
    ///
    /// turned by `φ′ − φ` into the geodetic frame: `X = X′ cos(φ′ − φ) − Z′ sin(φ′ − φ)`,
    /// `Y = Y′`, `Z = X′ sin(φ′ − φ) + Z′ cos(φ′ − φ)`. The rates are the same sums over `ġ` and
    /// `ḣ`.
    ///
    /// At a pole the north and east directions follow the given longitude's meridian, as the
    /// report's section 1.4 sets them.
    ///
    /// # Errors
    ///
    /// [`CoreError::Domain`] if `point` fails [`Geodetic::validated`], if `decimal_year` is outside
    /// the model's validity (`[2025.0, 2030.0]` for WMM2025) or not finite, or if the height is
    /// outside [`MIN_HEIGHT_M`] to [`MAX_HEIGHT_M`].
    pub fn field(&self, point: Geodetic, decimal_year: f64) -> Result<MagneticField, CoreError> {
        let point = point.validated()?;
        if !(self.epoch_year..=self.valid_until_year).contains(&decimal_year) {
            return Err(CoreError::Domain {
                what: "decimal year for the magnetic model",
                value: decimal_year,
            });
        }
        if !(MIN_HEIGHT_M..=MAX_HEIGHT_M).contains(&point.height_m) {
            return Err(CoreError::Domain {
                what: "ellipsoidal height for the magnetic model (m)",
                value: point.height_m,
            });
        }

        // Any finite longitude is accepted; reduce it exactly first, so that `m λ` stays small.
        let longitude = point.longitude_rad.rem_euclid(std::f64::consts::TAU);
        let (latitude_prime, radius) = geocentric(point);
        let harmonics = Harmonics::new(latitude_prime, radius, longitude);
        let (g, h, g_dot, h_dot) = self.coefficients_at(decimal_year);
        let field = harmonics.sum(&g, &h);
        let rate = harmonics.sum(&g_dot, &h_dot);

        let (sin_turn, cos_turn) = (latitude_prime - point.latitude_rad).sin_cos();
        let rotate = |v: Geocentric| {
            (
                v.north * cos_turn - v.down * sin_turn,
                v.east,
                v.north * sin_turn + v.down * cos_turn,
            )
        };
        let (x, y, z) = rotate(field);
        let (x_dot, y_dot, z_dot) = rotate(rate);

        let horizontal = x.hypot(y);
        let total = horizontal.hypot(z);
        let declination = y.atan2(x);
        let horizontal_rate = (x * x_dot + y * y_dot) / horizontal;
        let latitude_deg = point.latitude_rad.to_degrees();
        let grid_variation = if latitude_deg > 55.0 {
            Some(wrap_pi(declination - longitude))
        } else if latitude_deg < -55.0 {
            Some(wrap_pi(declination + longitude))
        } else {
            None
        };

        Ok(MagneticField {
            north_nt: x,
            east_nt: y,
            down_nt: z,
            horizontal_nt: horizontal,
            total_nt: total,
            inclination_rad: z.atan2(horizontal),
            declination_rad: declination,
            grid_variation_rad: grid_variation,
            north_rate_nt_per_year: x_dot,
            east_rate_nt_per_year: y_dot,
            down_rate_nt_per_year: z_dot,
            horizontal_rate_nt_per_year: horizontal_rate,
            total_rate_nt_per_year: (x * x_dot + y * y_dot + z * z_dot) / total,
            inclination_rate_rad_per_year: (horizontal * z_dot - z * horizontal_rate)
                / (total * total),
            declination_rate_rad_per_year: (x * y_dot - y * x_dot) / (horizontal * horizontal),
        })
    }
}

/// A calendar date as a decimal year: `year + (d − 1) / L`, where `d` is the day of the year
/// (1 for January 1) and `L` is 365 or 366. This is the start of the day. Outside the blackout
/// zones around the magnetic poles ([`CompassZone`]), the declination changes within a day by a
/// few thousandths of a degree at most, far below the model's own error.
///
/// # Errors
///
/// [`CoreError::Domain`] if `month` is not 1 to 12 or `day` is not a day of that month
/// (Gregorian calendar).
pub fn decimal_year(year: i32, month: u32, day: u32) -> Result<f64, CoreError> {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let Some(&length) = month
        .checked_sub(1)
        .and_then(|index| lengths.get(index as usize))
    else {
        return Err(CoreError::Domain {
            what: "month",
            value: f64::from(month),
        });
    };
    if day == 0 || day > length {
        return Err(CoreError::Domain {
            what: "day of the month",
            value: f64::from(day),
        });
    }
    let before: u32 = lengths[..(month - 1) as usize].iter().sum();
    let days_in_year = if leap { 366.0 } else { 365.0 };
    Ok(f64::from(year) + f64::from(before + day - 1) / days_in_year)
}
