# Atmosphere

Code: `hpr_atmos::ussa76` (the standard and its offsets), `hpr_atmos::moist` (humid air),
`hpr_atmos::profile` (soundings and forecasts), and the `Atmosphere` trait in `hpr_atmos::air`.

Sources:

- **[USSA]** *U.S. Standard Atmosphere, 1976*, NOAA-S/T 76-1562, part 1, pinned as
  `us-std-atmosphere-1976`. Equation, table and page numbers below are its own.
- **[WMO]** WMO-No. 8, *Guide to Instruments and Methods of Observation*, Vol. I (2023),
  `wmo-no8-vol1-2023`.
- **[CIPM]** A. Picard et al., "Revised formula for the density of moist air (CIPM-2007)",
  *Metrologia* 45 (2008) 149–155, `picard-2008-cipm-2007`.

## Height datum

Every atmosphere is queried with **geometric height above mean sea level** (`height_msl_m`). The
core's heights are ellipsoidal (`frames.md`), so the flight engine subtracts the geoid undulation
first: `H_msl = h − N`. Samples carry an `extrapolated` flag, set whenever a model answers outside
the range it is defined or tabulated over.

## The 1976 standard, −5 km to 86 km

Seven layers in geopotential altitude `H`, each with a constant gradient of the molecular-scale
temperature `T_M` ([USSA] Table 4):

| base `H_b` (km′) | 0 | 11 | 20 | 32 | 47 | 51 | 71 | 84.852 (top) |
|---|---|---|---|---|---|---|---|---|
| `L_M,b` (K/km′) | −6.5 | 0 | +1.0 | +2.8 | 0 | −2.8 | −2.0 | |

```text
H   = r₀ Z / (r₀ + Z)                                  (18)
T_M = T_M,b + L_M,b (H − H_b)                          (23)
P   = P_b [T_M,b / T_M]^(g₀′ M₀ / (R* L_M,b))           (33a)   L_M,b ≠ 0
P   = P_b exp[−g₀′ M₀ (H − H_b) / (R* T_M,b)]           (33b)   L_M,b = 0
ρ   = P M₀ / (R* T_M)                                  (42)
T   = T_M M / M₀                                       (22)
a   = (γ R* T_M / M₀)^½                                (50)
μ   = β T^(3/2) / (T + S)                              (51)
ν   = μ / ρ                                            (52)
```

- **Constants:**
  - `R* = 8314.32 J/(kmol·K)` (p. 3), not CODATA's 8314.46. Table 2 misprints the exponent's sign.
  - `M₀ = 28.9644 kg/kmol`, `g₀ = g₀′ = 9.80665`, `r₀ = 6 356 766 m`.
  - `P₀ = 101325 Pa`, `T₀ = 288.15 K`, `γ = 1.40`, `β = 1.458e-6`.
  - `S = 110.4 K` (p. 19). Table 2 and p. 4 print 110 K, but the tables use 110.4 K: sea-level μ
    is 1.7894e-5 Pa·s with it and 1.7912e-5 with 110.
- **80 to 86 km:** `M/M₀` comes from Table 8, interpolated linearly in `Z`. The printed tables
  leave it out below 86 km and print `T = T_M` (p. 9). This model follows the equations, so its
  kinetic temperature and viscosity there are up to 0.031% below the print.
- **Outside the range:** below −5 km the first layer continues, and above 86 km the atmosphere
  is isothermal at 186.87 K. Both are flagged. The real standard warms above 86 km and changes
  composition, so values there are rough, but the densities are tiny.
- **Loft got this wrong** (lessons L2–L4):
  - It fed geometric altitude to geopotential formulas: at 11 km it gave 216.65 K and 22 632 Pa,
    against 216.774 K and 22 699.96 Pa.
  - It had only four layers, so at 70 km it gave 335 K against 219.6 K.
  - Its Sutherland constants gave a sea-level viscosity 1.3% high.

## Offsets and launch-site conditions

`Ussa76::with_offset(ΔT, P₀)` adds `ΔT` to `T_M` at every geopotential height and integrates eqs.
33a/33b from `P₀`, so the atmosphere stays hydrostatic. `Ussa76::anchored(Z, T, P)` picks `ΔT` and
`P₀` so the profile passes through a measured temperature and pressure, such as field conditions.

This is **not** the aviation convention (ESDU 77022), which offsets temperature at equal pressure
altitude. Both are hydrostatic, but they differ aloft. At `H = 3000 m′` with `ΔT = +20 K`, the
aviation convention gives 289.95 K where this gives 288.65 K, and densities 0.38% apart
(`validation/oracles/atmosphere/conventions.py`). This choice keeps the lapse rate attached to
height, like a sounding; see ADR-004.

## Moist air

An ideal mixture of dry air (`M₀`, `γ = 1.4`) and water vapour (`M_v = 18.01528 kg/kmol`,
[CIPM] §2.1):

```text
e_w(t) = 6.112 exp(17.62 t / (243.12 + t)) hPa        [WMO] Annex 4.B, eq. 4.B.1, t in °C
e      = U e_w(T),   x_v = e / p
ρ      = p [(1 − x_v) M₀ + x_v M_v] / (R* T)  =  p / (R_d T_v)      [WMO] eq. 12.18
C_p    = (1 − x_v)(7/2) R* + x_v · 4 R*,   γ = C_p / (C_p − R*),   a = (γ R* T / M)^½
```

- **Relative humidity** is taken with respect to liquid water at every temperature, as
  radiosondes report it ([WMO] §12.1.2). No enhancement factor is applied: it would change `e` by
  about 0.5% near the surface, which is under 0.01% of density.
- **Accuracy:**
  - Density agrees with CIPM-2007 (a real-gas equation) to 0.047% over its range: 15–27 °C,
    600–1100 hPa, dry to saturated. Ignoring humidity entirely is 0.4% off at 20 °C and 50% RH.
  - Taking water vapour's `C_p` as `4R*` instead of its real value near 300 K (about 1% higher)
    moves `a` by 0.009% at 30 °C and saturation.
  - Viscosity stays dry air's Sutherland value. By Wilke's rule with IAPWS R12-08 vapour
    viscosity, saturation at 30 °C lowers it 2.1%, which moves turbulent skin friction about
    0.4%.
  - All these numbers come from `validation/oracles/atmosphere/moist_air.py`.

## Sounding and forecast profiles

`SoundingProfile` takes levels of geometric height, temperature, optional pressure, optional
relative humidity, and optional wind speed and direction. Humidity and wind are given on every
level or on none.

- **Between levels,** interpolation runs in the standard's geopotential height `H`:
  - `T` and `U` are linear in `H`.
  - Pressure uses `ln P = ln P_i + ln(P_{i+1}/P_i) · ln(T/T_i)/ln(T_{i+1}/T_i)`, which is
    exact for a dry hydrostatic layer with `T` linear in `H` and passes through both levels'
    pressures.
  - Levels sampled from the standard reproduce it between them to 1e-12.
- **Missing pressures** above the lowest level are filled hydrostatically, with virtual
  temperature linear in `H` ([WMO] eqs. 12.17–12.18). A dry fill agrees with direct integration
  to 1e-11. A humid fill agrees to 1.1e-5, because vapour pressure is exponential in `T` and so
  not quite linear across the layer.
- **Beyond the levels** the profile continues as the standard atmosphere anchored at the end
  level, and flags the sample:
  - Below, it holds relative humidity.
  - Above, it holds the vapour mole fraction, capped at saturation, so a humid top doesn't
    carry water into the cold stratosphere.
- **Geopotential heights.** Soundings (Wyoming's `HGHT`) and forecasts (Open-Meteo's
  `geopotential_height`) report geopotential metres above sea level. Convert them with
  `geometric_from_wmo_geopotential_m`, which uses [WMO] eqs. 12.15–12.16 with latitude-dependent
  normal gravity and radius. At 30 km that is 29.7785 km of geopotential at the equator and
  29.932 km at 80° N. The standard's latitude-free `r₀` formula is only for the standard itself.
- **Loft lesson L5:** "today's conditions" kept the standard lapse from the field up, ignored
  humidity and never used sounding temperatures.

## RocketPy 1.13.0, for M2.1

These are findings from reading `refs/rocketpy`, not yet pinned by fixtures:

- **Standard atmosphere:** ISO 2533 layers from −2 to 80 km, with `R = 287.05287` (the same as
  `R*/M₀`). Temperature is linear in geometric height between converted layer boundaries.
  Pressure is sampled at 100 points and splined.
- **Custom profiles:**
  - Every quantity, pressure included, is linear in height above sea level, with constant
    extrapolation.
  - Linear pressure is off hydrostatic by up to 0.06% between 1000 and 925 hPa, 1.15% between
    700 and 500 hPa, and 3.4% between 50 and 30 hPa (`conventions.py`).
  - M2.1 comparisons need a RocketPy-compatible option or levels dense enough that this
    doesn't matter.
- **Humidity** is not used anywhere.
- **Wyoming heights** are converted from geopotential with a radius only (no latitude).
  - The helper's default radius, 63 781 370 m in `rocketpy/tools.py:972`, is ten times the
    Earth's.
  - Check which callers rely on that default before M2.1.

## Tests that pin this

- **`ussa76::tests::matches_the_1976_tables_at_32_altitudes`** (M1.2 *done when*):
  - Covers `T`, `T_M`, `H`, `P`, `ρ`, `a`, `μ` and `ν` at 32 altitudes from −2 to 86 km.
  - Every value is within 0.1%, and within one count of its last printed digit.
  - The exceptions are those above (80–85.5 km, where the printed `T` equals `T_M`) and the
    84 km density. The latter prints 9.6940E-6 where the equations give 9.69387e-6; the row's
    own `ρ/ρ₀` agrees with the equations.
  - The fixture was transcribed from the page images and cross-checked by
    `validation/oracles/ussa76/tables.py` against mpmath and `ambiance`.
- **Loft lessons:**
  - `geometric_11_km_matches_the_1976_tables` (L2)
  - `fifty_km_is_270_65_k_and_79_779_pa` (L3)
  - `sea_level_viscosity_is_1_7894e_5` (L4)
  - `profile::tests::sounding_temperature_overrides_standard_lapse` (L5)
- **Constants and structure:**
  - `constants_match_the_transcription` checks the constants and Tables 4 and 8.
  - Hydrostatic balance `dP/dZ = −ρg` is checked in every layer, and as a property test over
    offsets.
  - Anchors reproduce their conditions, and layers join continuously.
- **`moist::tests`:**
  - WMO 4.B.1 values.
  - Dry air equals the standard.
  - CIPM-2007 density to 0.05% at 36 points.
- **`profile::tests`:**
  - Standard levels are reproduced.
  - Dry and humid fills match direct integration.
  - The continuation beyond the levels.
  - WMO geopotential values.
  - Errors and serde.
