# Turbulence

Code: `hpr_atmos::dryden`, using the seeded generator in `hpr_core::random`.

Sources:

- **[8785C]** MIL-F-8785C, *Military Specification: Flying Qualities of Piloted Airplanes*
  (5 November 1980), §3.7 and the definitions in §6.2.7, pinned as `mil-f-8785c`.
- **[1797]** MIL-HDBK-1797 (1997), Appendix A §4.9, for its differences only. It is not pinned:
  no stable public copy was found.
- **[BV]** D. Blackman and S. Vigna, "Scrambled linear pseudorandom number generators", *ACM
  TOMS* 47(4) (2021): xoshiro256++ with SplitMix64 seeding.
- **[MB]** G. Marsaglia and T. A. Bray, "A convenient method for generating normal variables",
  *SIAM Review* 6(3) (1964): the polar method.

## Dryden spectra

Turbulence is a frozen random field that the vehicle moves through, so its spectra are functions
of spatial frequency `Ω` (rad/m, [8785C] §6.2.7). They are one-sided, with
`∫₀^∞ Φ dΩ = σ²` ([8785C] §3.7.1.2):

```text
Φ_u(Ω) = σ_u² (2L_u/π) / (1 + (L_u Ω)²)
Φ_v(Ω) = σ_v² (L_v/π) (1 + 3(L_v Ω)²) / (1 + (L_v Ω)²)²       (Φ_w likewise)
R_u(ξ) = σ_u² e^(−|ξ|/L_u)
R_v(ξ) = σ_v² e^(−|ξ|/L_v) (1 − |ξ|/(2L_v))
```

`Φ = (2/π) ∫₀^∞ R(ξ) cos(Ωξ) dξ`, so each spectrum and autocorrelation is a Fourier pair. A test
checks this numerically.

**The factor-of-two trap.** [1797] writes the transverse spectra with `2L_v` and `12(L_vΩ)²` over
`(1 + 4(L_vΩ)²)²`, and halves the lengths (`L_u = 2L_v`). The spectra are identical, but mixing
one document's lengths with the other's formula puts the transverse scales off by two. This code
uses [8785C]'s form and lengths throughout.

## Parameters

- **Low altitude** ([8785C] §3.7.3.4, Figs. 10–11). With `h` the height above terrain in ft and
  `u₂₀` the mean wind at 20 ft:

  ```text
  L_u = L_v = h / (0.177 + 0.000823 h)^1.2,   L_w = h           (ft)
  σ_w = 0.1 u₂₀,   σ_u = σ_v = σ_w / (0.177 + 0.000823 h)^0.4
  ```

  - The formulas hold from 10 to 1000 ft. Below 10 ft this code uses the 10 ft values. From 1000
    to 2000 ft the figures give `L = 1000 ft` and equal intensities.
  - The model covers only up to about 2000 ft (§3.8.1), so above that the constructor returns an
    error instead of quietly holding values.
  - Fig. 9 marks `u₂₀` = 15, 30 and 45 kt for light, moderate and severe turbulence.
  - For aircraft at low altitude, `u` lies along the horizontal *relative* mean wind and `w` is
    vertical ([8785C] p. 60).
- **Medium/high altitude** ([8785C] §3.7.2, above about 2000 ft): isotropic, with
  `L = 1750 ft`.
  - The intensity against altitude and exceedance probability is only a graph (Fig. 7), so the
    caller supplies it.
  - Neither military document says how to blend 1000–2000 ft. MATLAB's documentation
    interpolates linearly, but that is its own choice.
- **Axes of the gust field:** `u` is the longitudinal component and `v`, `w` the transverse ones.
  - The longitudinal spectrum belongs to the component along the path through the frozen
    field. For a climbing rocket that is nearly vertical, so M1.6 must align `u` with the path.
  - Getting that wrong changes the statistics: a horizontal gust given the longitudinal spectrum
    has twice the transverse power at low frequency and 2/3 of it at high frequency.
  - The axes' signs don't matter.

## Generator

Exact discretization, so step length never changes the statistics.

- **`u`:** a first-order Gauss–Markov state. Over a step `Δs`, with `ρ = e^(−Δs/L)`:
  `x ← ρx + √(1 − ρ²) n`.
- **`v` and `w`:** two normalized states, driven as `dx₁/ds = (−x₁ + η)/L` and
  `dx₂/ds = (x₁ − x₂)/L`, with output `y = (σ/√2)(√3 x₁ + (1 − √3) x₂)`.
  - The stationary covariance is `P = [[1, ½], [½, ½]]` for every `L`.
  - The output's autocorrelation is exactly `R_v`. Algebra: `c Φ(r) P cᵀ = 2e^(−r)(1 − r/2)`
    with `c = (√3, 1 − √3)`.
  - Over `r = Δs/L`, the transition is `Φ(r) = e^(−r) [[1, 0], [r, 1]]`, plus Gaussian noise of
    covariance `Q = P − ΦPΦᵀ`:

    ```text
    Q = [[P(1,2r), ½P(2,2r)], [½P(2,2r), ½P(3,2r)]],   P(n,x) = γ(n,x)/Γ(n)
    ```

  - Written with the regularized incomplete gamma function, `Q` keeps full relative precision
    for tiny steps, where the closed form `½ − e^(−2r)(r² + r + ½)` cancels to nothing. The
    noise is drawn through `Q`'s Cholesky factor.
- **State is normalized,** so intensities and lengths may change from one step to the next
  (with altitude, say) without breaking stationarity.
- **Determinism:** draws come in a fixed order (`u`, two for `v`, two for `w`) from a seeded
  xoshiro256++, so the same seed and steps give bit-identical gusts on one platform.
  - The integer stream is identical everywhere.
  - Normals and gusts go through the math library's `ln` and `exp`, which may differ in the last
    bit between platforms.
- **Checkpointing:** the generator serializes, so a run can be checkpointed and resumed.

`GustField` precomputes a realization at a fixed spacing and interpolates it linearly. That makes
the gust a pure function of the path coordinate, which an adaptive integrator can evaluate
repeatedly and on rejected steps.

- **Spacing:** linear interpolation smooths wavelengths near the spacing, so keep the spacing at
  or below a tenth of the smallest scale length.
- **Size limit:** a field holds at most 10⁷ samples.

## Limits for rockets

- **Scaled for aircraft.** Dryden's lengths and intensities describe aircraft flying roughly
  level. A rocket climbs through the low-altitude model's height dependence in seconds. The
  frozen-field assumption holds when airspeed is well above the gust velocities. That is false on
  the rail and near apogee, where the gust field barely moves past the vehicle.
- **Path coordinate:** M1.6 decides what to key the field on: distance flown through the air, or
  altitude. It also decides how to fade gusts in on the rail. This module does neither.

## Tests that pin this

- **`dryden::tests::dryden_spectrum_matches_theory`** (M1.2 *done when*):
  - Setup: 2²⁰ samples at 1 m, with `σ = (1.5, 1.2, 0.9)` m/s and `L = (40, 40, 20)` m.
  - Estimate: 256 Hann-windowed segments of 4096 samples, averaged (Bartlett's method).
  - In every octave band from bin 1 to Nyquist, each component's mean ratio to theory is within
    4 standard errors.
  - The variance of a band's mean ratio is `[1 + 2ρ₁²(n−1)/n + 2ρ₂²(n−2)/n]/(nK)` for `n` bins
    and `K` segments, with the Hann window's neighbouring-bin correlations `ρ₁ = 2/3` and
    `ρ₂ = 1/6`. The bracket tends to 1.94.
  - The one- and two-bin bands at the bottom are loose (±25%, ±21%); the wide bands (±1–3%) pin
    the spectrum.
  - Theory is the continuous spectrum sampled at 1 m, in closed form. Below a tenth of Nyquist it
    is checked against [8785C]'s formula to 1%.
  - The variance of the record is within 5% of `σ²`.
- **Exactness:**
  - `transition_preserves_the_stationary_covariance`: `ΦPΦᵀ + Q = P` to 4ε for steps from 10⁻¹²
    to 10³ scale lengths.
  - `two_steps_compose_into_one`: `Q(a+b) = Φ(b)Q(a)Φ(b)ᵀ + Q(b)` to 1e-12 relative, down to
    `r = 1e-9`.
  - `transverse_output_has_the_dryden_autocorrelation`.
  - `regularized_gamma_matches_independent_references`.
- **Stepping loop:** `quarter_metre_steps_give_the_one_metre_correlation`, to 2e-3.
  - The sampling error is about 4e-4, so this catches a scale length 10% off.
  - It can't tell an exact step from an Euler step (8e-5 apart), which is why the exactness
    tests above exist.
- **Spectra:**
  - `spectra_integrate_to_the_variance`
  - `spectra_are_the_cosine_transforms_of_the_autocorrelations`
- **Parameters:** `low_altitude_parameters_follow_the_specification`, at 100 ft moderate:
  `L_u = 505.169 ft` and `σ_u/σ_w = 1.715849`. It also checks the 10 ft floor, the 1000–2000 ft
  hold, and the refusal above 2000 ft.
- **Determinism:** the same seed gives bit-identical fields, and a serialized generator resumes
  the stream.
- **`hpr_core::random::tests`:**
  - Bit-identical to `rand_xoshiro` over 10⁴ draws for 5 seeds.
  - The normal sampler's moments and CDF at ±2σ, within 5 standard errors.
