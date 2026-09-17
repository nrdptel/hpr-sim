//! Seeded Dryden turbulence: the spectra, MIL-F-8785C parameters, a generator that is exact for
//! any step length, and a precomputed gust field.
//!
//! Source: MIL-F-8785C, *Military Specification: Flying Qualities of Piloted Airplanes*
//! (5 November 1980), section 3.7, pinned as `mil-f-8785c` (see `docs/physics/turbulence.md`).
//!
//! **Spectra** (§3.7.1.2 and the definitions in §6.2.7). Turbulence is a frozen random field that
//! the vehicle flies through, so the spectra are functions of spatial frequency `Ω` (rad/m). They
//! are one-sided, with `∫₀^∞ Φ(Ω) dΩ = σ²`:
//!
//! ```text
//! Φ_u(Ω) = σ_u² (2 L_u / π) / (1 + (L_u Ω)²)
//! Φ_v(Ω) = σ_v² (L_v / π) (1 + 3 (L_v Ω)²) / (1 + (L_v Ω)²)²
//! Φ_w(Ω) = σ_w² (L_w / π) (1 + 3 (L_w Ω)²) / (1 + (L_w Ω)²)²
//! ```
//!
//! These are MIL-F-8785C's scale lengths. MIL-HDBK-1797 writes the transverse spectra with
//! `2 L_v` and `2 L_w` and halves those lengths, which gives the same spectra; mixing one
//! document's lengths with the other's formula is off by a factor of two.
//!
//! Their autocorrelations along the path, at separation `ξ`, are
//!
//! ```text
//! R_u(ξ) = σ_u² e^(−|ξ|/L_u)
//! R_v(ξ) = σ_v² e^(−|ξ|/L_v) (1 − |ξ| / (2 L_v))
//! ```
//!
//! **Generator.** The longitudinal component is a first-order Gauss–Markov process. Each
//! transverse component is the output `y = (σ/√2)(√3 x₁ + (1 − √3) x₂)` of two states driven by
//! white noise through `dx₁/ds = (−x₁ + η)/L`, `dx₂/ds = (x₁ − x₂)/L`. Its stationary state
//! covariance is `P = [[1, ½], [½, ½]]` for every `L`, and the output's autocorrelation is exactly
//! `R_v` above. A step of length `Δs` (with `r = Δs/L`, `x = 2r`) maps the state through
//!
//! ```text
//! Φ(r) = e^(−r) [[1, 0], [r, 1]]
//! Q(r) = [[P(1, x), ½ P(2, x)], [½ P(2, x), ½ P(3, x)]],   P(n, x) = γ(n, x)/Γ(n)
//! ```
//!
//! and adds Gaussian noise of covariance `Q = P − Φ P Φᵀ`, written with regularized incomplete
//! gamma functions so it has no cancellation for tiny steps. The longitudinal state uses
//! `ρ = e^(−r)` and noise variance `P(1, x) = 1 − ρ²`. Because this is the exact transition of the
//! continuous process, any sequence of step lengths samples the same field statistics, and
//! parameters that change between steps keep the state stationary.

use hpr_core::DVec3;
use hpr_core::interp::Side;
use hpr_core::random::SeededRng;
use serde::{Deserialize, Serialize};

use crate::error::{AtmosError, finite, positive};

/// Largest `r = Δs/L` stepped exactly; beyond it the state is redrawn from the stationary
/// distribution, which differs from the exact transition by less than `e^(−800)`.
const DECORRELATED: f64 = 800.0;

/// The most samples a [`GustField`] holds (240 MB of `f64`s).
pub const MAX_GUST_FIELD_SAMPLES: usize = 10_000_000;

/// The international foot, m.
const FOOT_M: f64 = 0.3048;

/// The international knot, m/s (1852 m per hour).
const KNOT_M_S: f64 = 1852.0 / 3600.0;

/// Lower end of the low-altitude formulas, ft.
const LOW_ALTITUDE_MIN_FT: f64 = 10.0;

/// Upper end of the low-altitude formulas, ft.
const LOW_ALTITUDE_MAX_FT: f64 = 1000.0;

/// Dryden scale length above about 2000 ft, ft (MIL-F-8785C §3.7.2).
const MEDIUM_HIGH_ALTITUDE_SCALE_LENGTH_FT: f64 = 1750.0;

/// Turbulence severity, which sets the low-altitude reference wind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurbulenceSeverity {
    /// 15 kt at 20 ft.
    Light,
    /// 30 kt at 20 ft.
    Moderate,
    /// 45 kt at 20 ft.
    Severe,
}

impl TurbulenceSeverity {
    /// The wind speed at 20 ft that MIL-F-8785C Figure 9 marks for this severity (15, 30 and
    /// 45 kt), m/s.
    pub fn wind_speed_20_ft_m_s(self) -> f64 {
        let knots = match self {
            TurbulenceSeverity::Light => 15.0,
            TurbulenceSeverity::Moderate => 30.0,
            TurbulenceSeverity::Severe => 45.0,
        };
        knots * KNOT_M_S
    }
}

/// Intensities and scale lengths of Dryden turbulence.
///
/// Components are `u` (longitudinal), `v` (lateral) and `w` (vertical); see [`GustField`] for the
/// axes they apply along.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DrydenParametersData", into = "DrydenParametersData")]
pub struct DrydenParameters {
    intensity_m_s: DVec3,
    scale_length_m: DVec3,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DrydenParametersData {
    intensity_m_s: DVec3,
    scale_length_m: DVec3,
}

impl TryFrom<DrydenParametersData> for DrydenParameters {
    type Error = AtmosError;

    fn try_from(data: DrydenParametersData) -> Result<Self, AtmosError> {
        DrydenParameters::new(data.intensity_m_s, data.scale_length_m)
    }
}

impl From<DrydenParameters> for DrydenParametersData {
    fn from(p: DrydenParameters) -> Self {
        DrydenParametersData {
            intensity_m_s: p.intensity_m_s,
            scale_length_m: p.scale_length_m,
        }
    }
}

impl DrydenParameters {
    /// Turbulence with RMS intensities `(σ_u, σ_v, σ_w)` in m/s and scale lengths
    /// `(L_u, L_v, L_w)` in m.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if an intensity is negative or a scale length is not positive, or
    /// any value is not finite.
    pub fn new(intensity_m_s: DVec3, scale_length_m: DVec3) -> Result<Self, AtmosError> {
        for sigma in intensity_m_s.to_array() {
            if finite("turbulence intensity (m/s)", sigma)? < 0.0 {
                return Err(AtmosError::Domain {
                    what: "turbulence intensity (m/s)",
                    value: sigma,
                });
            }
        }
        for length in scale_length_m.to_array() {
            positive("turbulence scale length (m)", length)?;
        }
        Ok(DrydenParameters {
            intensity_m_s,
            scale_length_m,
        })
    }

    /// MIL-F-8785C low-altitude turbulence (§3.7.3.4, Figures 10 and 11) at `height_agl_m`
    /// above the terrain, for a mean wind of `wind_speed_20_ft_m_s` at 20 ft (6.096 m). With `h`
    /// in feet:
    ///
    /// ```text
    /// L_u = L_v = h / (0.177 + 0.000823 h)^1.2,   L_w = h                      (ft)
    /// σ_w = 0.1 u₂₀,   σ_u = σ_v = σ_w / (0.177 + 0.000823 h)^0.4
    /// ```
    ///
    /// The formulas hold from 10 to 1000 ft. Below 10 ft this uses the 10 ft values; above
    /// 1000 ft, the specification's figures' values there (`L = 1000 ft`, `σ_u = σ_v = σ_w`).
    /// The specification applies the low-altitude model up to about 2000 ft and gives no blend
    /// into the medium/high-altitude model ([`DrydenParameters::mil_f_8785c_medium_high_altitude`]).
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite or the wind speed is negative or not
    /// finite.
    pub fn mil_f_8785c_low_altitude(
        height_agl_m: f64,
        wind_speed_20_ft_m_s: f64,
    ) -> Result<Self, AtmosError> {
        let height_ft = (finite("height above terrain (m)", height_agl_m)? / FOOT_M)
            .clamp(LOW_ALTITUDE_MIN_FT, LOW_ALTITUDE_MAX_FT);
        let u20 = finite("wind speed at 20 ft (m/s)", wind_speed_20_ft_m_s)?;
        if u20 < 0.0 {
            return Err(AtmosError::Domain {
                what: "wind speed at 20 ft (m/s)",
                value: u20,
            });
        }
        let factor = 0.177 + 0.000_823 * height_ft;
        let horizontal_length_m = height_ft / factor.powf(1.2) * FOOT_M;
        let sigma_w = 0.1 * u20;
        let sigma_horizontal = sigma_w / factor.powf(0.4);
        DrydenParameters::new(
            DVec3::new(sigma_horizontal, sigma_horizontal, sigma_w),
            DVec3::new(horizontal_length_m, horizontal_length_m, height_ft * FOOT_M),
        )
    }

    /// MIL-F-8785C medium/high-altitude turbulence (§3.7.2, above about 2000 ft): isotropic, with
    /// `σ_u = σ_v = σ_w = intensity_m_s` and `L_u = L_v = L_w = 1750 ft` (533.4 m).
    ///
    /// The specification gives the intensity against altitude and probability of exceedance only
    /// as a graph (Figure 7), so the caller supplies it.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the intensity is negative or not finite.
    pub fn mil_f_8785c_medium_high_altitude(intensity_m_s: f64) -> Result<Self, AtmosError> {
        DrydenParameters::new(
            DVec3::splat(intensity_m_s),
            DVec3::splat(MEDIUM_HIGH_ALTITUDE_SCALE_LENGTH_FT * FOOT_M),
        )
    }

    /// RMS intensities `(σ_u, σ_v, σ_w)`, m/s.
    pub fn intensity_m_s(&self) -> DVec3 {
        self.intensity_m_s
    }

    /// Scale lengths `(L_u, L_v, L_w)`, m.
    pub fn scale_length_m(&self) -> DVec3 {
        self.scale_length_m
    }

    /// The one-sided spectra `(Φ_u, Φ_v, Φ_w)` at spatial frequency `omega_rad_m` (rad/m), in
    /// (m/s)² per rad/m.
    pub fn spectra(&self, omega_rad_m: f64) -> DVec3 {
        let s = self.intensity_m_s;
        let l = self.scale_length_m;
        let longitudinal = {
            let a = l.x * omega_rad_m;
            s.x * s.x * (2.0 * l.x / std::f64::consts::PI) / (1.0 + a * a)
        };
        let transverse = |sigma: f64, length: f64| {
            let a2 = (length * omega_rad_m).powi(2);
            sigma * sigma * (length / std::f64::consts::PI) * (1.0 + 3.0 * a2) / (1.0 + a2).powi(2)
        };
        DVec3::new(longitudinal, transverse(s.y, l.y), transverse(s.z, l.z))
    }

    /// The autocorrelations `(R_u, R_v, R_w)` at path separation `separation_m`, (m/s)².
    pub fn autocorrelation(&self, separation_m: f64) -> DVec3 {
        let s = self.intensity_m_s;
        let l = self.scale_length_m;
        let xi = separation_m.abs();
        let transverse = |sigma: f64, length: f64| {
            sigma * sigma * (-xi / length).exp() * (1.0 - xi / (2.0 * length))
        };
        DVec3::new(
            s.x * s.x * (-xi / l.x).exp(),
            transverse(s.y, l.y),
            transverse(s.z, l.z),
        )
    }
}

/// The regularized lower incomplete gamma function `P(n, x) = γ(n, x)/Γ(n)` for `n = 1, 2, 3`
/// and `x ≥ 0`: `1 − e^(−x) Σ_{k<n} x^k/k!`, summed as `e^(−x) Σ_{k≥n} x^k/k!` for `x < 1` so tiny
/// arguments keep every digit.
fn regularized_gamma(n: u32, x: f64) -> f64 {
    if x >= 1.0 {
        if x > DECORRELATED {
            return 1.0;
        }
        let mut partial = 0.0;
        let mut term = 1.0;
        for k in 0..n {
            partial += term;
            term *= x / f64::from(k + 1);
        }
        return 1.0 - (-x).exp() * partial;
    }
    // term = x^k / k!, starting at k = n.
    let mut term = 1.0;
    for k in 1..=n {
        term *= x / f64::from(k);
    }
    let mut sum = term;
    let mut k = n;
    // For x < 1 each term is less than the last divided by k + 1, so this stops within about 20
    // terms. At x = 0 every term is zero and it stops at once.
    loop {
        k += 1;
        term *= x / f64::from(k);
        if term <= f64::EPSILON * 0.125 * sum {
            break;
        }
        sum += term;
    }
    (-x).exp() * sum
}

/// The exact transition of a normalized transverse state over `r = Δs/L`: the decay `e^(−r)` of
/// `Φ(r)` and the noise covariance entries `(Q₁₁, Q₁₂, Q₂₂)`.
fn transverse_transition(r: f64) -> (f64, [f64; 3]) {
    let x = 2.0 * r;
    (
        (-r).exp(),
        [
            regularized_gamma(1, x),
            0.5 * regularized_gamma(2, x),
            0.5 * regularized_gamma(3, x),
        ],
    )
}

/// The normalized state of a transverse component, with stationary covariance
/// `[[1, ½], [½, ½]]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransverseState {
    x1: f64,
    x2: f64,
}

impl TransverseState {
    fn stationary(rng: &mut SeededRng) -> Self {
        let n1 = rng.standard_normal();
        let n2 = rng.standard_normal();
        TransverseState {
            x1: n1,
            x2: 0.5 * n1 + 0.5 * n2,
        }
    }

    /// Advances by `r = Δs/L ≥ 0`.
    fn advance(&mut self, r: f64, rng: &mut SeededRng) {
        if r > DECORRELATED {
            *self = TransverseState::stationary(rng);
            return;
        }
        let (decay, [q11, q12, q22]) = transverse_transition(r);
        let (x1, x2) = (self.x1, self.x2);
        // Cholesky factor of Q. `q11 > 0` for r > 0; at r = 0, Q = 0 and the state is unchanged.
        let n1 = rng.standard_normal();
        let n2 = rng.standard_normal();
        let (w1, w2) = if q11 > 0.0 {
            let l11 = q11.sqrt();
            let l21 = q12 / l11;
            let l22 = (q22 - l21 * l21).max(0.0).sqrt();
            (l11 * n1, l21 * n1 + l22 * n2)
        } else {
            (0.0, 0.0)
        };
        self.x1 = decay * x1 + w1;
        self.x2 = decay * (r * x1 + x2) + w2;
    }

    /// The output for intensity `sigma`: `(σ/√2)(√3 x₁ + (1 − √3) x₂)`.
    fn output(&self, sigma: f64) -> f64 {
        let sqrt3 = 3.0_f64.sqrt();
        sigma * std::f64::consts::FRAC_1_SQRT_2 * (sqrt3 * self.x1 + (1.0 - sqrt3) * self.x2)
    }
}

/// A seeded Dryden turbulence generator that advances along the flight path.
///
/// The state is normalized, so the intensities and scale lengths may change from one step to the
/// next (for example with altitude); within a step they are held constant. Draws come from a
/// [`SeededRng`] in a fixed order (`u`, then two for `v`, then two for `w`), so the same seed
/// and the same steps give bit-identical gusts on one platform. (Across platforms the math
/// library's `exp` and `ln` may differ in the last bit.)
///
/// It serializes as its generator and state, so a run can be checkpointed and resumed; a
/// non-finite state is rejected when deserialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DrydenGeneratorData", into = "DrydenGeneratorData")]
pub struct DrydenGenerator {
    rng: SeededRng,
    u: f64,
    v: TransverseState,
    w: TransverseState,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DrydenGeneratorData {
    rng: SeededRng,
    u: f64,
    v: TransverseState,
    w: TransverseState,
}

impl TryFrom<DrydenGeneratorData> for DrydenGenerator {
    type Error = AtmosError;

    fn try_from(data: DrydenGeneratorData) -> Result<Self, AtmosError> {
        for value in [data.u, data.v.x1, data.v.x2, data.w.x1, data.w.x2] {
            finite("turbulence generator state", value)?;
        }
        Ok(DrydenGenerator {
            rng: data.rng,
            u: data.u,
            v: data.v,
            w: data.w,
        })
    }
}

impl From<DrydenGenerator> for DrydenGeneratorData {
    fn from(generator: DrydenGenerator) -> Self {
        DrydenGeneratorData {
            rng: generator.rng,
            u: generator.u,
            v: generator.v,
            w: generator.w,
        }
    }
}

impl DrydenGenerator {
    /// A generator whose initial state is drawn from the stationary distribution using `seed`.
    pub fn new(seed: u64) -> Self {
        let mut rng = SeededRng::seed_from_u64(seed);
        let u = rng.standard_normal();
        let v = TransverseState::stationary(&mut rng);
        let w = TransverseState::stationary(&mut rng);
        DrydenGenerator { rng, u, v, w }
    }

    /// The current gust `(u, v, w)` for `parameters`, m/s.
    pub fn gust_m_s(&self, parameters: &DrydenParameters) -> DVec3 {
        let s = parameters.intensity_m_s;
        DVec3::new(s.x * self.u, self.v.output(s.y), self.w.output(s.z))
    }

    /// Moves `distance_m` along the path through turbulence described by `parameters`, and
    /// returns the gust there.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the distance is negative or not finite.
    pub fn advance(
        &mut self,
        distance_m: f64,
        parameters: &DrydenParameters,
    ) -> Result<DVec3, AtmosError> {
        let ds = finite("turbulence step (m)", distance_m)?;
        if ds < 0.0 {
            return Err(AtmosError::Domain {
                what: "turbulence step (m)",
                value: ds,
            });
        }
        let l = parameters.scale_length_m;
        let r = ds / l.x;
        let n = self.rng.standard_normal();
        self.u = if r > DECORRELATED {
            n
        } else {
            (-r).exp() * self.u + regularized_gamma(1, 2.0 * r).sqrt() * n
        };
        self.v.advance(ds / l.y, &mut self.rng);
        self.w.advance(ds / l.z, &mut self.rng);
        Ok(self.gust_m_s(parameters))
    }
}

/// A gust and whether it was looked up beyond the end of its field.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GustSample {
    /// Gust velocity `(u, v, w)`, m/s.
    pub gust_m_s: DVec3,
    /// `Some` when the distance was outside `[0, length]` and the end sample was held.
    pub extrapolated: Option<Side>,
}

/// A precomputed Dryden turbulence realization along a path coordinate, sampled every `spacing`
/// metres and interpolated linearly, so it is a pure function an adaptive integrator can call
/// repeatedly.
///
/// **Axes.** `u` is along the mean wind's horizontal direction of travel, `w` is up, and `v`
/// completes a right-handed set (90° to the left of `u`, seen from above). The field is
/// isotropic in sign, so these choices do not change its statistics.
///
/// **Path coordinate.** The field does not decide what `s` is: distance flown through the air is
/// Taylor's hypothesis, but a caller may key it on altitude or on time at a reference speed.
/// Linear interpolation removes variance at wavelengths near the spacing, so keep the spacing
/// well below the smallest scale length (a tenth or less).
///
/// It serializes as its spacing and samples, and re-checks them when deserialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "GustFieldData")]
pub struct GustField {
    spacing_m: f64,
    samples: Vec<DVec3>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GustFieldData {
    spacing_m: f64,
    samples: Vec<DVec3>,
}

impl TryFrom<GustFieldData> for GustField {
    type Error = AtmosError;

    fn try_from(data: GustFieldData) -> Result<Self, AtmosError> {
        let spacing = positive("gust field spacing (m)", data.spacing_m)?;
        if data.samples.is_empty() {
            return Err(AtmosError::EmptyGustField);
        }
        if data.samples.len() > MAX_GUST_FIELD_SAMPLES {
            // Cast: only reported in the error.
            let count = data.samples.len() as f64;
            return Err(AtmosError::Domain {
                what: "gust field samples",
                value: count,
            });
        }
        for sample in &data.samples {
            for value in sample.to_array() {
                finite("gust sample (m/s)", value)?;
            }
        }
        Ok(GustField {
            spacing_m: spacing,
            samples: data.samples,
        })
    }
}

impl GustField {
    /// A field covering at least `length_m` (rounded up to a whole number of spacings), with
    /// samples every `spacing_m` and constant `parameters`.
    ///
    /// # Errors
    ///
    /// See [`GustField::generate_with`].
    pub fn generate(
        seed: u64,
        parameters: &DrydenParameters,
        length_m: f64,
        spacing_m: f64,
    ) -> Result<Self, AtmosError> {
        GustField::generate_with(seed, length_m, spacing_m, |_| Ok(*parameters))
    }

    /// A field whose parameters vary along the path: `parameters(s)` is evaluated at each sample
    /// point `s = k·spacing` and held over the step that ends there (the first sample uses
    /// `parameters(0)`).
    ///
    /// # Errors
    ///
    /// - [`AtmosError::Domain`] if the length is negative or not finite, the spacing is not
    ///   finite and positive, or the field would need more than [`MAX_GUST_FIELD_SAMPLES`].
    /// - Any error `parameters` returns.
    pub fn generate_with(
        seed: u64,
        length_m: f64,
        spacing_m: f64,
        mut parameters: impl FnMut(f64) -> Result<DrydenParameters, AtmosError>,
    ) -> Result<Self, AtmosError> {
        let length = finite("gust field length (m)", length_m)?;
        if length < 0.0 {
            return Err(AtmosError::Domain {
                what: "gust field length (m)",
                value: length,
            });
        }
        let spacing = positive("gust field spacing (m)", spacing_m)?;
        let intervals = (length / spacing).ceil();
        // Cast: the sample limit is far below 2⁵³.
        let limit = MAX_GUST_FIELD_SAMPLES as f64;
        if intervals + 1.0 > limit {
            return Err(AtmosError::Domain {
                what: "gust field samples",
                value: intervals + 1.0,
            });
        }
        // Cast: intervals is a non-negative integer below the sample limit.
        let intervals = intervals as usize;
        let mut generator = DrydenGenerator::new(seed);
        let mut samples = Vec::with_capacity(intervals + 1);
        samples.push(generator.gust_m_s(&parameters(0.0)?));
        for k in 1..=intervals {
            // Cast: k is below the sample limit, far below 2⁵³.
            let s = k as f64 * spacing;
            samples.push(generator.advance(spacing, &parameters(s)?)?);
        }
        Ok(GustField {
            spacing_m: spacing,
            samples,
        })
    }

    /// The spacing between samples, m.
    pub fn spacing_m(&self) -> f64 {
        self.spacing_m
    }

    /// The samples, at `s = 0, spacing, 2·spacing, …`.
    pub fn samples(&self) -> &[DVec3] {
        &self.samples
    }

    /// The path length the field covers, m.
    pub fn length_m(&self) -> f64 {
        // Cast: the sample count is below the limit, far below 2⁵³.
        let intervals = self.samples.len().saturating_sub(1) as f64;
        intervals * self.spacing_m
    }

    /// The gust at path coordinate `distance_m`, linearly interpolated; beyond either end the end
    /// sample is held and flagged.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the distance is not finite.
    pub fn gust(&self, distance_m: f64) -> Result<GustSample, AtmosError> {
        let s = finite("gust field distance (m)", distance_m)?;
        let (Some(&first), Some(&last)) = (self.samples.first(), self.samples.last()) else {
            return Err(AtmosError::EmptyGustField);
        };
        if s < 0.0 {
            return Ok(GustSample {
                gust_m_s: first,
                extrapolated: Some(Side::Below),
            });
        }
        if s > self.length_m() {
            return Ok(GustSample {
                gust_m_s: last,
                extrapolated: Some(Side::Above),
            });
        }
        let position = s / self.spacing_m;
        // Cast: 0 ≤ position ≤ the sample count, which fits a usize.
        let i = (position.floor() as usize).min(self.samples.len() - 1);
        let Some(&b) = self.samples.get(i + 1) else {
            return Ok(GustSample {
                gust_m_s: last,
                extrapolated: None,
            });
        };
        let a = self.samples[i];
        let t = (position - position.floor()).clamp(0.0, 1.0);
        Ok(GustSample {
            gust_m_s: a + t * (b - a),
            extrapolated: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    use super::*;

    fn parameters() -> DrydenParameters {
        DrydenParameters::new(DVec3::new(1.5, 1.2, 0.9), DVec3::new(40.0, 40.0, 20.0)).unwrap()
    }

    /// Composite Simpson's rule on `[a, b]` with `n` (even) intervals.
    fn simpson(f: impl Fn(f64) -> f64, a: f64, b: f64, n: usize) -> f64 {
        let h = (b - a) / n as f64;
        let mut sum = f(a) + f(b);
        for i in 1..n {
            let weight = if i % 2 == 1 { 4.0 } else { 2.0 };
            sum += weight * f(a + i as f64 * h);
        }
        sum * h / 3.0
    }

    #[test]
    fn spectra_integrate_to_the_variance() {
        let p = parameters();
        let sigma2 = p.intensity_m_s() * p.intensity_m_s();
        for (component, length) in p.scale_length_m().to_array().into_iter().enumerate() {
            // Ω = tan(θ)/L maps [0, ∞) onto [0, π/2), where the integrand stays bounded.
            let integrand = |theta: f64| {
                let omega = theta.tan() / length;
                let jacobian = 1.0 / (length * theta.cos().powi(2));
                p.spectra(omega)[component] * jacobian
            };
            let integral = simpson(integrand, 0.0, FRAC_PI_2 - 1e-9, 20_000);
            let expected = sigma2[component];
            assert!(
                (integral / expected - 1.0).abs() < 1e-8,
                "component {component}: {integral} vs {expected}"
            );
        }
    }

    /// The one-sided spectrum is `(2/π) ∫₀^∞ R(ξ) cos(Ωξ) dξ`: the stated spectra and
    /// autocorrelations are a Fourier pair.
    #[test]
    fn spectra_are_the_cosine_transforms_of_the_autocorrelations() {
        let p = parameters();
        for omega in [0.0, 0.005, 0.02, 0.05, 0.1] {
            for component in 0..3 {
                let length = p.scale_length_m()[component];
                let transform = simpson(
                    |xi| p.autocorrelation(xi)[component] * (omega * xi).cos(),
                    0.0,
                    60.0 * length,
                    200_000,
                ) * 2.0
                    / PI;
                let spectrum = p.spectra(omega)[component];
                assert!(
                    (transform - spectrum).abs() < 1e-7 * spectrum.abs().max(1e-3),
                    "component {component} at Ω = {omega}: {transform} vs {spectrum}"
                );
            }
        }
    }

    /// `P(n, x)` against `∫₀^x t^(n−1) e^(−t) dt / (n−1)!` by Simpson's rule, and for tiny `x`
    /// against its series `e^(−x) x^n/n! (1 + x/(n+1) + x²/((n+1)(n+2)))`, whose truncation error
    /// is below 10⁻¹⁵ there.
    #[test]
    fn regularized_gamma_matches_independent_references() {
        let factorial = [1.0, 1.0, 2.0, 6.0];
        for x in [
            0.0_f64, 1e-300, 1e-12, 1e-6, 0.01, 0.3, 0.999, 1.0, 2.5, 30.0, 900.0,
        ] {
            for n in 1..=3_u32 {
                let nf = f64::from(n);
                let ours = regularized_gamma(n, x);
                let (reference, tolerance) = if x < 1e-5 {
                    let series = (-x).exp() * x.powi(n as i32) / factorial[n as usize]
                        * (1.0 + x / (nf + 1.0) + x * x / ((nf + 1.0) * (nf + 2.0)));
                    (series, 1e-14)
                } else if x > 60.0 {
                    (1.0, 1e-15)
                } else {
                    let integral = simpson(|t| t.powi(n as i32 - 1) * (-t).exp(), 0.0, x, 20_000)
                        / factorial[n as usize - 1];
                    (integral, 1e-12)
                };
                assert!(
                    (ours - reference).abs() <= tolerance * reference,
                    "P({n}, {x}) = {ours}, reference {reference}"
                );
            }
        }
    }

    type Mat2 = [[f64; 2]; 2];

    fn mul(a: Mat2, b: Mat2) -> Mat2 {
        [
            [
                a[0][0] * b[0][0] + a[0][1] * b[1][0],
                a[0][0] * b[0][1] + a[0][1] * b[1][1],
            ],
            [
                a[1][0] * b[0][0] + a[1][1] * b[1][0],
                a[1][0] * b[0][1] + a[1][1] * b[1][1],
            ],
        ]
    }

    fn transpose(a: Mat2) -> Mat2 {
        [[a[0][0], a[1][0]], [a[0][1], a[1][1]]]
    }

    fn add(a: Mat2, b: Mat2) -> Mat2 {
        [
            [a[0][0] + b[0][0], a[0][1] + b[0][1]],
            [a[1][0] + b[1][0], a[1][1] + b[1][1]],
        ]
    }

    fn phi(r: f64) -> Mat2 {
        let (decay, _) = transverse_transition(r);
        [[decay, 0.0], [decay * r, decay]]
    }

    fn q(r: f64) -> Mat2 {
        let (_, [q11, q12, q22]) = transverse_transition(r);
        [[q11, q12], [q12, q22]]
    }

    const STATIONARY: Mat2 = [[1.0, 0.5], [0.5, 0.5]];

    fn assert_mat_close(a: Mat2, b: Mat2, tolerance: f64) {
        for i in 0..2 {
            for j in 0..2 {
                assert!((a[i][j] - b[i][j]).abs() <= tolerance, "{a:?} vs {b:?}");
            }
        }
    }

    /// `Φ P Φᵀ + Q = P` for steps from 10⁻¹² to 10³ scale lengths: every step keeps the state
    /// stationary.
    #[test]
    fn transition_preserves_the_stationary_covariance() {
        for exponent in -12..=3 {
            for mantissa in [1.0, 3.0] {
                let r = mantissa * 10f64.powi(exponent);
                let propagated = add(mul(mul(phi(r), STATIONARY), transpose(phi(r))), q(r));
                assert_mat_close(propagated, STATIONARY, 4.0 * f64::EPSILON);
            }
        }
    }

    /// Two steps `a` then `b` equal one step `a + b` in distribution: `Φ(a+b) = Φ(b)Φ(a)` and
    /// `Q(a+b) = Φ(b) Q(a) Φ(b)ᵀ + Q(b)`. Small steps keep their noise covariance to relative
    /// precision, which the cancelling closed form would not.
    #[test]
    fn two_steps_compose_into_one() {
        for (a, b) in [
            (0.3, 0.7),
            (1e-6, 2e-6),
            (1e-9, 1e-9),
            (5.0, 0.01),
            (2.0, 3.0),
        ] {
            assert_mat_close(phi(a + b), mul(phi(b), phi(a)), 1e-15);
            let composed = add(mul(mul(phi(b), q(a)), transpose(phi(b))), q(b));
            let direct = q(a + b);
            for i in 0..2 {
                for j in 0..2 {
                    let scale = direct[i][j].abs();
                    assert!(
                        (composed[i][j] - direct[i][j]).abs() <= 1e-12 * scale,
                        "({a}, {b}) [{i}][{j}]: {composed:?} vs {direct:?}"
                    );
                }
            }
        }
    }

    /// The output `(σ/√2)(√3 x₁ + (1 − √3) x₂)` of the stationary state has autocorrelation
    /// `σ² e^(−r)(1 − r/2)`, which is `R_v` of MIL-F-8785C's Dryden spectrum.
    #[test]
    fn transverse_output_has_the_dryden_autocorrelation() {
        let sqrt3 = 3.0_f64.sqrt();
        let c = [sqrt3, 1.0 - sqrt3];
        let p = parameters();
        for r in [0.0, 0.1, 0.5, 1.0, 2.0, 4.0, 10.0] {
            let cov = mul(phi(r), STATIONARY);
            let value = 0.5
                * (c[0] * (cov[0][0] * c[0] + cov[0][1] * c[1])
                    + c[1] * (cov[1][0] * c[0] + cov[1][1] * c[1]));
            let length = p.scale_length_m().z;
            let expected = p.autocorrelation(r * length).z / p.intensity_m_s().z.powi(2);
            assert!(
                (value - expected).abs() < 1e-15,
                "r = {r}: {value} vs {expected}"
            );
        }
    }

    /// In-place iterative radix-2 FFT, `X_k = Σ x_n e^(−2πikn/N)`.
    fn fft(re: &mut [f64], im: &mut [f64]) {
        let n = re.len();
        assert!(n.is_power_of_two());
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j |= bit;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut len = 2;
        while len <= n {
            let angle = -TAU / len as f64;
            for start in (0..n).step_by(len) {
                for k in 0..len / 2 {
                    let (c, s) = ((angle * k as f64).cos(), (angle * k as f64).sin());
                    let (a, b) = (start + k, start + k + len / 2);
                    let tr = re[b] * c - im[b] * s;
                    let ti = re[b] * s + im[b] * c;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
            }
            len <<= 1;
        }
    }

    #[test]
    fn test_fft_matches_the_direct_transform() {
        let mut rng = SeededRng::seed_from_u64(5);
        let x: Vec<f64> = (0..64).map(|_| rng.standard_normal()).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; 64]);
        fft(&mut re, &mut im);
        for k in 0..64 {
            let (mut dr, mut di) = (0.0, 0.0);
            for (n, &value) in x.iter().enumerate() {
                let angle = -TAU * (k * n) as f64 / 64.0;
                dr += value * angle.cos();
                di += value * angle.sin();
            }
            assert!((re[k] - dr).abs() < 1e-12 && (im[k] - di).abs() < 1e-12);
        }
    }

    /// Two-sided spectral density (per cycle/m) of the process sampled every `ds` at frequency
    /// `f`: `ds Σ_k R(k ds) e^(−2πifk ds)`, summed in closed form. With `ρ = e^(−ds/L)`,
    /// `ω = 2πf ds` and `z = ρ e^(−iω)`, `Σ ρ^|k| e^(−iωk) = (1 − ρ²)/(1 − 2ρ cos ω + ρ²)` and
    /// `Σ |k| ρ^|k| e^(−iωk) = 2 Re[z/(1 − z)²]`.
    fn sampled_spectrum(sigma: f64, length: f64, transverse: bool, ds: f64, f: f64) -> f64 {
        let rho = (-ds / length).exp();
        let omega = TAU * f * ds;
        let a = (1.0 - rho * rho) / (1.0 - 2.0 * rho * omega.cos() + rho * rho);
        if !transverse {
            return ds * sigma * sigma * a;
        }
        let (zr, zi) = (rho * omega.cos(), -rho * omega.sin());
        // (1 − z)² = (1 − zr)² − zi² − 2i(1 − zr)zi... as (dr + i di).
        let (wr, wi) = (1.0 - zr, -zi);
        let (dr, di) = (wr * wr - wi * wi, 2.0 * wr * wi);
        let denominator = dr * dr + di * di;
        let real = (zr * dr + zi * di) / denominator;
        let r = ds / length;
        ds * sigma * sigma * (a - r * real)
    }

    #[test]
    fn sampled_spectrum_closed_form_matches_the_direct_sum() {
        for (length, transverse) in [(40.0, false), (40.0, true), (20.0, true)] {
            for f in [0.0, 0.001, 0.01, 0.1, 0.37, 0.5] {
                let ds = 1.0;
                let mut sum = 1.0;
                for k in 1..4000 {
                    let xi = k as f64 * ds;
                    let rho = (-xi / length).exp();
                    let r = if transverse {
                        rho * (1.0 - xi / (2.0 * length))
                    } else {
                        rho
                    };
                    sum += 2.0 * r * (TAU * f * xi).cos();
                }
                let direct = ds * sum;
                let closed = sampled_spectrum(1.0, length, transverse, ds, f);
                assert!(
                    (closed - direct).abs() < 1e-9 * direct.abs().max(1e-6),
                    "{f}"
                );
            }
        }
    }

    /// The M1.2 *done when*: a Dryden spectrum test.
    ///
    /// A seeded field of 2²⁰ samples one metre apart is cut into 256 segments of 4096. Each is
    /// Hann-windowed and transformed, and the periodograms are averaged (Bartlett's method).
    /// Over octave bands of frequency bins, the mean ratio of the estimate to theory must fall
    /// within 4 standard errors. The standard error of a band of `n` bins averaged over `K`
    /// segments is `√(1.94/(nK))`, where 1.94 accounts for the Hann window's correlation between
    /// neighbouring bins (`1 + 2·(2/3)² + 2·(1/6)²`).
    ///
    /// Theory is the spectrum of the continuous process sampled every metre (its aliased
    /// spectrum), which the exact discretization produces. Below a tenth of the Nyquist
    /// frequency it is also checked against MIL-F-8785C's continuous formula itself, to 1%.
    #[test]
    fn dryden_spectrum_matches_theory() {
        let p = parameters();
        let (segment, segments, ds) = (4096_usize, 256_usize, 1.0);
        let field = GustField::generate(8785, &p, (segment * segments) as f64, ds).unwrap();
        let window: Vec<f64> = (0..segment)
            .map(|n| 0.5 - 0.5 * (TAU * n as f64 / segment as f64).cos())
            .collect();
        let window_power: f64 = window.iter().map(|w| w * w).sum();
        let mut estimate = vec![[0.0_f64; 3]; segment / 2 + 1];
        for j in 0..segments {
            let chunk = &field.samples()[j * segment..(j + 1) * segment];
            for component in 0..3 {
                let mut re: Vec<f64> = chunk
                    .iter()
                    .zip(&window)
                    .map(|(g, w)| g[component] * w)
                    .collect();
                let mut im = vec![0.0; segment];
                fft(&mut re, &mut im);
                for (k, bin) in estimate.iter_mut().enumerate() {
                    // Two-sided density per cycle/m.
                    bin[component] +=
                        ds * (re[k] * re[k] + im[k] * im[k]) / window_power / segments as f64;
                }
            }
        }

        let sigma = p.intensity_m_s();
        let length = p.scale_length_m();
        let frequency = |k: usize| k as f64 / (segment as f64 * ds);
        for component in 0..3 {
            let theory = |k: usize| {
                sampled_spectrum(
                    sigma[component],
                    length[component],
                    component > 0,
                    ds,
                    frequency(k),
                )
            };
            let mut band_start = 8;
            while band_start < segment / 2 {
                let band_end = (2 * band_start).min(segment / 2);
                let n = band_end - band_start;
                let ratio = (band_start..band_end)
                    .map(|k| estimate[k][component] / theory(k))
                    .sum::<f64>()
                    / n as f64;
                let standard_error = (1.94 / (n * segments) as f64).sqrt();
                assert!(
                    (ratio - 1.0).abs() < 4.0 * standard_error,
                    "component {component}, bins {band_start}..{band_end}: ratio {ratio}, \
                     standard error {standard_error}"
                );
                band_start = band_end;
            }
            // The sampled spectrum is MIL-F-8785C's: two-sided per cycle/m is π Φ(2πf).
            for k in 8..=segment / 20 {
                let continuous = PI * p.spectra(TAU * frequency(k))[component];
                assert!((theory(k) / continuous - 1.0).abs() < 0.01, "bin {k}");
            }
            // And the variance of the whole record is σ², within 5 standard errors (about 1%).
            let variance = field
                .samples()
                .iter()
                .map(|g| g[component].powi(2))
                .sum::<f64>()
                / field.samples().len() as f64;
            assert!(
                (variance / sigma[component].powi(2) - 1.0).abs() < 0.05,
                "component {component}: variance {variance}"
            );
        }
    }

    /// Stepping 0.25 m at a time samples the same process as stepping 1 m: the lag-1 m
    /// correlation of the fine record matches `R(1 m)/σ²`.
    #[test]
    fn step_length_does_not_change_the_statistics() {
        let p = parameters();
        let fine = GustField::generate(11, &p, 400_000.0, 0.25).unwrap();
        let samples = fine.samples();
        for component in 0..3 {
            let (mut lag0, mut lag1) = (0.0, 0.0);
            for i in 0..samples.len() - 4 {
                lag0 += samples[i][component].powi(2);
                lag1 += samples[i][component] * samples[i + 4][component];
            }
            let correlation = lag1 / lag0;
            let expected = p.autocorrelation(1.0)[component] / p.intensity_m_s()[component].powi(2);
            assert!(
                (correlation - expected).abs() < 0.01,
                "component {component}: {correlation} vs {expected}"
            );
        }
    }

    #[test]
    fn same_seed_gives_bit_identical_fields() {
        let p = parameters();
        let a = GustField::generate(42, &p, 500.0, 0.5).unwrap();
        let b = GustField::generate(42, &p, 500.0, 0.5).unwrap();
        let c = GustField::generate(43, &p, 500.0, 0.5).unwrap();
        assert_eq!(a.samples().len(), 1001);
        assert!(
            a.samples()
                .iter()
                .zip(b.samples())
                .all(|(x, y)| x.to_array().map(f64::to_bits) == y.to_array().map(f64::to_bits))
        );
        assert_ne!(a.samples(), c.samples());
        // A checkpointed generator resumes the same stream.
        let mut generator = DrydenGenerator::new(9);
        generator.advance(3.0, &p).unwrap();
        let json = serde_json::to_string(&generator).unwrap();
        let mut resumed: DrydenGenerator = serde_json::from_str(&json).unwrap();
        for _ in 0..10 {
            assert_eq!(
                generator.advance(0.7, &p).unwrap(),
                resumed.advance(0.7, &p).unwrap()
            );
        }
    }

    #[test]
    fn gust_field_interpolates_and_flags_its_ends() {
        let p = parameters();
        let field = GustField::generate(1, &p, 10.0, 2.0).unwrap();
        assert_eq!(field.samples().len(), 6);
        assert_eq!(field.length_m(), 10.0);
        let s = field.samples();
        let mid = field.gust(3.0).unwrap();
        assert_eq!(mid.extrapolated, None);
        assert!((mid.gust_m_s - 0.5 * (s[1] + s[2])).length() < 1e-15);
        assert_eq!(field.gust(4.0).unwrap().gust_m_s, s[2]);
        assert_eq!(field.gust(10.0).unwrap().gust_m_s, s[5]);
        let below = field.gust(-1.0).unwrap();
        assert_eq!(
            (below.gust_m_s, below.extrapolated),
            (s[0], Some(Side::Below))
        );
        let above = field.gust(10.5).unwrap();
        assert_eq!(
            (above.gust_m_s, above.extrapolated),
            (s[5], Some(Side::Above))
        );
        assert!(field.gust(f64::NAN).is_err());
        // A zero-length field is one sample.
        let point = GustField::generate(1, &p, 0.0, 1.0).unwrap();
        assert_eq!(point.samples().len(), 1);
        assert_eq!(point.gust(0.0).unwrap().extrapolated, None);
    }

    #[test]
    fn zero_intensity_is_calm_and_bad_inputs_are_rejected() {
        let calm = DrydenParameters::new(DVec3::ZERO, DVec3::splat(100.0)).unwrap();
        let field = GustField::generate(3, &calm, 100.0, 1.0).unwrap();
        assert!(field.samples().iter().all(|g| *g == DVec3::ZERO));
        assert!(DrydenParameters::new(DVec3::new(-1.0, 1.0, 1.0), DVec3::ONE).is_err());
        assert!(DrydenParameters::new(DVec3::ONE, DVec3::new(1.0, 0.0, 1.0)).is_err());
        assert!(DrydenParameters::new(DVec3::ONE, DVec3::new(1.0, f64::INFINITY, 1.0)).is_err());
        let p = parameters();
        assert!(GustField::generate(1, &p, -1.0, 1.0).is_err());
        assert!(GustField::generate(1, &p, 1.0, 0.0).is_err());
        assert!(GustField::generate(1, &p, 1e9, 1e-3).is_err());
        let mut generator = DrydenGenerator::new(1);
        assert!(generator.advance(-0.1, &p).is_err());
        // A step of zero changes nothing; a huge step still gives finite gusts.
        let before = generator.gust_m_s(&p);
        assert_eq!(generator.advance(0.0, &p).unwrap(), before);
        assert!(generator.advance(1e300, &p).unwrap().is_finite());
        let json = r#"{"intensity_m_s":[1,1,1],"scale_length_m":[1,-1,1]}"#;
        assert!(serde_json::from_str::<DrydenParameters>(json).is_err());
        // Fields and generators re-check what they deserialize.
        let field = GustField::generate(2, &p, 5.0, 1.0).unwrap();
        let json = serde_json::to_string(&field).unwrap();
        assert_eq!(serde_json::from_str::<GustField>(&json).unwrap(), field);
        let bad_json = r#"{"spacing_m":0.0,"samples":[[0,0,0]]}"#;
        assert!(serde_json::from_str::<GustField>(bad_json).is_err());
        let data = |spacing_m, samples| GustFieldData { spacing_m, samples };
        assert!(matches!(
            GustField::try_from(data(0.0, vec![DVec3::ZERO])),
            Err(AtmosError::Domain { .. })
        ));
        assert_eq!(
            GustField::try_from(data(1.0, vec![])),
            Err(AtmosError::EmptyGustField)
        );
        assert!(matches!(
            GustField::try_from(data(1.0, vec![DVec3::new(0.0, f64::INFINITY, 0.0)])),
            Err(AtmosError::Domain { .. })
        ));
        let generator = DrydenGenerator::new(4);
        let state = |v_x1| DrydenGeneratorData {
            rng: generator.rng.clone(),
            u: generator.u,
            v: TransverseState {
                x1: v_x1,
                x2: generator.v.x2,
            },
            w: generator.w,
        };
        assert_eq!(
            DrydenGenerator::try_from(state(generator.v.x1)).unwrap(),
            generator
        );
        assert!(matches!(
            DrydenGenerator::try_from(state(f64::NAN)),
            Err(AtmosError::Domain { .. })
        ));
    }

    /// MIL-F-8785C Figures 10 and 11 at h = 100 ft for a moderate 30 kt wind at 20 ft, evaluated
    /// separately: 0.177 + 0.0823 = 0.2593; L_u = 100/0.2593^1.2 = 505.169 ft; σ_w = 0.1·15.433
    /// m/s; σ_u = σ_w/0.2593^0.4 = 1.715849 σ_w.
    #[test]
    fn low_altitude_parameters_follow_the_specification() {
        let u20 = TurbulenceSeverity::Moderate.wind_speed_20_ft_m_s();
        assert!((u20 - 30.0 * 1852.0 / 3600.0).abs() < 1e-12);
        let p = DrydenParameters::mil_f_8785c_low_altitude(100.0 * FOOT_M, u20).unwrap();
        let factor: f64 = 0.2593;
        let l = p.scale_length_m();
        let s = p.intensity_m_s();
        assert!((l.x / FOOT_M - 100.0 / factor.powf(1.2)).abs() < 1e-9);
        assert!((l.x / FOOT_M - 505.169).abs() < 5e-4);
        assert_eq!(l.x, l.y);
        assert!((l.z - 30.48).abs() < 1e-12);
        assert!((s.z - 0.1 * u20).abs() < 1e-15);
        assert!((s.x / s.z - 1.715_849).abs() < 5e-7);
        assert_eq!(s.x, s.y);
        // Continuous into the constant values above 1000 ft; held below 10 ft.
        let top = DrydenParameters::mil_f_8785c_low_altitude(1000.0 * FOOT_M, u20).unwrap();
        let above = DrydenParameters::mil_f_8785c_low_altitude(1500.0 * FOOT_M, u20).unwrap();
        assert!((top.scale_length_m() - DVec3::splat(304.8)).length() < 1e-9);
        assert!((top.intensity_m_s() - DVec3::splat(0.1 * u20)).length() < 1e-12);
        assert_eq!(top, above);
        let ground = DrydenParameters::mil_f_8785c_low_altitude(0.0, u20).unwrap();
        let ten_feet = DrydenParameters::mil_f_8785c_low_altitude(10.0 * FOOT_M, u20).unwrap();
        assert_eq!(ground, ten_feet);
        let high = DrydenParameters::mil_f_8785c_medium_high_altitude(2.0).unwrap();
        assert_eq!(high.scale_length_m(), DVec3::splat(533.4));
        assert!(DrydenParameters::mil_f_8785c_low_altitude(10.0, -1.0).is_err());
    }
}
