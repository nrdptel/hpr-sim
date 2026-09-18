# Gravity and Earth rotation

Code: `hpr_core::gravity` (the normal gravity field) and `hpr_core::earth` (what the flight engine
uses). Source: **[NGA]** NGA.STND.0036_1.0.0_WGS84 (2014), chapter 4 and appendix B, pinned as
`wgs84-nga-stnd-0036`.

## What normal gravity is

Normal gravity `γ` is the gravity of the level ellipsoid: the attraction of the WGS 84 ellipsoid
plus the centrifugal acceleration of the Earth's rotation. It is what a body at rest on the
rotating Earth feels.

- In the Earth-fixed launch frame, the only extra inertial term is Coriolis, `−2Ω × v`. Adding a
  centrifugal term as well would count it twice.
- Normal gravity ignores the real Earth's gravity anomalies, typically within ±1e-4 relative
  (±100 mGal). A geopotential model such as EGM2008 is a later option.

## Defining parameters and derived constants

The four defining parameters ([NGA] Table 3.1) are:

- `a = 6378137.0 m`
- `1/f = 298.257223563`
- `GM = 3.986004418e14 m³/s²`
- `ω = 7.292115e-5 rad/s`

Everything else is derived from them ([NGA] appendix B):

```text
e′ = E/b                                   (second eccentricity)
q₀ = ½[(1 + 3/e′²) atan e′ − 3/e′]          (B-18)
q₀′ = 3(1 + 1/e′²)(1 − atan(e′)/e′) − 1     (B-19)
m = ω²a²b/GM                                (B-20)
γ_e = GM/(ab) (1 − m − m e′ q₀′/(6 q₀))     (B-24)
γ_p = GM/a² (1 + m e′ q₀′/(3 q₀))           (B-25)
k = (b γ_p − a γ_e)/(a γ_e)                 (B-26)
```

**Cancellation.** The closed forms for `q` and `q′` cancel about five digits at `ε = E/u ≤ 0.082`.
A first version computed `k` with a relative error of 2.6e-11, so it no longer rounded to Table
3.6. Below `ε = 0.5` the code therefore uses the series from `atan ε = Σ (−1)ⁿ ε^(2n+1)/(2n+1)`:

```text
q  = Σ_{n≥1} (−1)^(n+1) 2n ε^(2n+1) / ((2n+1)(2n+3))
q′ = Σ_{n≥1} (−1)^(n+1) 6 ε^(2n)    / ((2n+1)(2n+3))
```

**Check.** These reproduce the printed values to their last digit:

| quantity | value | printed in |
|---|---|---|
| `q₀` | 7.334625787083e-5 | eq. B-18 |
| `q₀′` | 2.688041300461e-3 | eq. B-19 |
| `γ_e` | 9.7803253359 m/s² | Table 3.6 |
| `γ_p` | 9.8321849379 m/s² | Table 3.6 |
| `k` | 1.931852652458e-3 | Table 3.6 |
| `m` | 3.449786506841e-3 | Table 3.6 |

## Formulas

- **On the ellipsoid (Somigliana, eq. 4-1):** `γ = γ_e (1 + k sin²φ)/√(1 − e² sin²φ)`.
- **Taylor series in height (eq. 4-3):**
  `γ_h = γ [1 − (2/a)(1 + f + m − 2f sin²φ) h + (3/a²) h²]`. RocketPy uses this form. Its error
  against the exact field is 1e-8 relative at 1.4 km, 3e-7 at 30 km, 1.4e-5 at 100 km and 1.1e-4
  at 200 km (fixture values below).
- **Exact field (eqs. 4-5 to 4-13):** ellipsoidal-harmonic coordinates `(u, β)` give the components
  `γ_u` and `γ_β`. The code rotates them into ECEF with `R₁` (eq. 4-18).
  - Eq. 4-8 is used in the equivalent form `u² = ½[s + √(s² + 4E²z²)]`, with
    `s = x² + y² + z² − E²`, which never divides by `s`.
  - `β` comes from `atan2`, so every quadrant works.
  - The field is undefined on the focal disc (`u = 0`, the equatorial plane within 522 km of the
    centre); the code returns an error there.
- **Local components:** in the ENU axes at the point, `−γ·û` is the exact normal component `γ_h`
  (eq. 4-16), `γ·n̂` is `γ_φ` (eq. 4-23, positive north), and the length is `|γ_total|` (eq. 4-4).
  - Above the ellipsoid, the vector tilts slightly toward the equator: `γ_φ < 0` in the northern
    hemisphere, `−8.1e-4 m/s²` at 45° N and 100 km.
  - That sign is confirmed independently. The reference script differentiates the normal
    potential numerically, and its value on the ellipsoid matches Table 3.6's `U₀`.

## Gravity models for the flight engine (`earth::GravityModel`)

| model | vector in `L` | use |
|---|---|---|
| `constant { g_mps2 }` | `(0, 0, −g)` | analytic tests; comparisons with tools that use 9.80665 |
| `vertical_taylor` | `(0, 0, −γ_h)` from eq. 4-3 at `(φ₀, h₀ + z)` | like-for-like with RocketPy's formula (see its flight quirks below) |
| `vertical` | `(0, 0, −|γ|)` exact at `(φ₀, λ₀, h₀ + z)` | along the launch vertical |
| `ellipsoidal` (default) | the full vector at the body's position, rotated into `L` | everything else |

The ellipsoidal model follows the vertical as it turns downrange: by about `d/(N + h)` east-west
and `d/(M + h)` north-south, with `M = a(1 − e²)/(1 − e² sin²φ)^(3/2)` the meridian radius. A
test checks both at 20 km, to 1e-4 east and 1e-3 north (the curvature changes along a meridian). Earth rotation (`earth::EarthRotation`) is `coriolis` by default, `−2Ω × v` with
`Ω = ω(0, cos φ₀, sin φ₀)`, or `ignore`.

**`STANDARD_GRAVITY_MPS2 = 9.80665`** is the conventional `g₀` (3rd CGPM, 1901; also used by the
U.S. Standard Atmosphere 1976). It is a unit convention, not a model of local gravity. Loft used it
as gravity, which put Loft 0.3% off RocketPy at the equator ([Loft lesson L1][lessons]).

## RocketPy 1.13.0, for like-for-like cases

Recorded by `validation/oracles/rocketpy/gravity.py` into
`validation/fixtures/earth/rocketpy-gravity.json`, for the RocketPy comparison of the validation
milestone ([M2.1][roadmap]).

- **Formula:** Somigliana (4-1) times the Taylor factor (4-3), with Table 3.6 constants
  (`rocketpy/environment/environment.py`, `somigliana_gravity`). It matches
  `NormalGravity::taylor_mps2` to under 1e-12 relative.
- **Sampled, then held above 80 km:** a flight samples the formula at 100 points between 0 and
  `max_expected_height` (80 km by default) and holds the last value above that. At 45° and 100 km
  a flight uses 9.563982 m/s², where the formula gives 9.504874.
- **Height datum:** it is fed height above sea level, not above the ellipsoid.
- **Missing latitude:** the default latitude of 0 gives equatorial gravity.
- **Coriolis:** included as `−2Ω×v` with `Ω = 2π/86164.1 s`, in the 6-DOF and parachute
  equations but not in the rail phase.

## Tests that pin this

- **`gravity::tests::somigliana_matches_published_values`** ([Loft lesson L1][lessons], and the
  *done when* of [M1.1][roadmap], the core math, frames and Earth milestone):
  - Table 3.6 constants to their printed digits.
  - At 11 latitude/longitude/height points, including the equator, both poles, launch sites and
    heights up to 200 km:
    - surface (4-1), Taylor (4-3), `|γ|` (4-4) and `γ_h` (4-16) within 1e-6 relative, and in
      fact within 2e-14 relative;
    - `γ_φ` and the ECEF vector within 1e-12 m/s².
  - The reference values come from `validation/oracles/wgs84/normal_gravity.py`, which evaluates
    the published formulas in 40-digit arithmetic, checks the constants against the tables, and
    checks the vector against the gradient of the normal potential.
- **`taylor_series_matches_the_rocketpy_oracle`:** 8 points against RocketPy's formula.
- **`exact_field_reduces_to_somigliana_on_the_ellipsoid`:** 37 latitudes, zero horizontal
  component.
- **`q_functions_match_appendix_b`:** the printed `q₀` and `q₀′`, and continuity where the series
  hands over to the closed form.
- **`earth::tests`:** the models agree at the pad; the ellipsoidal model follows the vertical
  downrange; the Coriolis direction and magnitude.

[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
