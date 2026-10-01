# The magnetic field and declination

## In short

- **What it models:** the Earth's main magnetic field at any place and time from 2025.0 to
  2030.0, from the World Magnetic Model (WMM2025). Its most useful output is the
  **declination**: the angle between true north and the north a compass shows. Nothing in hpr
  applies it for you: a rail's heading (`hpr sim --heading`, the builder's `heading_deg`) is a
  true bearing, so add the declination to a compass reading first ([A worked
  example](#a-worked-example) shows how).
- **Sources:** A. Chulliat, W. Brown, M. Nair and others, *The US/UK World Magnetic Model for
  2025–2030: Technical Report*, NOAA NCEI (2025), with NCEI's coefficient file `WMM2025.COF`,
  which NCEI places in the public domain.
- **How well it is validated:** hpr reproduces all 12 of the report's test points (its Table 6)
  to their last printed digit, declination to 0.005°. NCEI's file of 100 points, printed to 1e-6
  nT and angles to 0.01°, matches to its last digit in declination, inclination, the east
  component and four rates. Its north component differs at 97 points, by up to 7.18e-4 nT (2.11e-8
  of the total field), and the horizontal and total intensities with it; an independent check
  in the tests places that difference in NCEI's file, not in hpr ([The test values](#the-test-values)).
  That shows the model is computed correctly. The model itself is only as good as the Earth
  allows: its own error estimate for declination is 0.29° at best and 0.35° to 0.56° at the
  example's sites, more near the magnetic poles. Each result carries that estimate.
- **What it leaves out:** local magnetic rocks, magnetic storms and the steel near a compass.
  Dates outside 2025.0 to 2030.0 are refused, and so are heights below −1 km or above 850 km
  ([Limits](#limits)).

## Sources

Code: [`hpr_core::magnetic`](../api/hpr_core/magnetic/index.html). Conventions:
[Frames](frames.md) and [Geodesy](geodesy.md). Each source below is pinned by its checksum in the
reference library ([`validation/refs.lock.toml`](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml)),
under the name given.

- **[WMM]** A. Chulliat, W. Brown, M. Nair, N. Gomez Perez, L.-Y. Young, C. Watson, N. Boneh,
  C. Beggan, B. Meyer and M. Paniccia, *The US/UK World Magnetic Model for 2025–2030: Technical
  Report*, National Centers for Environmental Information, NOAA, 2025,
  <https://doi.org/10.25923/prbc-s316>. Section 1.2 gives the equations, 1.4 the poles, 1.8 the
  blackout zones, 3.4 the error model. Pinned as `wmm2025-report`.
- **[COF]** NCEI, `WMM2025COF.zip` (2024-12-17): the coefficient file `WMM2025.COF` and the test
  values `WMM2025_TestValues.txt`, committed unchanged in
  [`crates/hpr-core/data/wmm2025/`](https://github.com/nrdptel/hpr-sim/tree/main/crates/hpr-core/data/wmm2025),
  with the report's Table 6 as `WMM2025_TEST_VALUES.txt`. Pinned as `wmm2025-coefficients`.
- **[DLMF]** NIST Digital Library of Mathematical Functions, equation
  [14.10.3](https://dlmf.nist.gov/14.10.E3): the recurrence used for the Legendre functions.

## What declination is

A compass needle lines up with the horizontal part of the Earth's magnetic field. That points to
magnetic north, which is not true north (the direction of the North Pole along the ground). The
angle from true north to magnetic north is the **declination** `D`. It is positive when magnetic
north is east of true north.

So a bearing read off a compass becomes a true bearing by adding the declination:

```text
true bearing = magnetic bearing + D
```

At Spaceport America in mid-2026, `D` is +7.75°: a rail aimed at a compass's north points 7.75°
east of true north. hpr's flight engine takes headings as true bearings (clockwise from true north,
see [Frames](frames.md)), so a heading measured with a compass needs this correction first.

The field also dips into the ground. The field's **inclination** `I` (not the rail's inclination)
is that dip, positive downward: about 60° across the southern United States, and 90° at a
magnetic pole, where a compass has nothing horizontal to follow.

Field strengths are in nanoteslas (nT). The Earth's field is 20,000 to 70,000 nT at the surface.

## The model

The WMM writes the field as a sum of **spherical harmonics**: patterns over the globe that get
finer as their **degree** `n` rises, from 1 (one north and one south pole, like a bar magnet) to
12, each split into **orders** `m` from 0 to `n`. That makes 90 terms. Each has two **Gauss
coefficients**, `g` and `h` in nT (`h` is zero where `m = 0`), which set how strong the pattern
is. Each coefficient changes linearly in time from its 2025.0 value, at the rate `ġ` or `ḣ`
([WMM] eq. 9):

```text
g(t) = g(2025.0) + (t − 2025.0) ġ
```

where `t` is the decimal year (2026.5 is the middle of 2026). hpr computes the field the way [WMM] section 1.2 sets out:

1. **To geocentric coordinates** (eqs. 7, 8): the site's latitude `φ`, longitude `λ` and height
   above the WGS 84 ellipsoid become a radius `r` and a geocentric latitude `φ′`, the angle seen
   from the Earth's centre.
2. **The field's three parts there** (eqs. 10 to 12), north `X′`, east `Y′` and down `Z′`, as sums
   over degree `n` and order `m`, with `a = 6,371,200 m`:

   ```text
   X′ = −Σ (a/r)^(n+2) Σ (g cos mλ + h sin mλ) dP̆(sin φ′)/dφ′
   Y′ =  Σ (a/r)^(n+2) Σ m (g sin mλ − h cos mλ) P̆(sin φ′) / cos φ′
   Z′ = −Σ (n+1)(a/r)^(n+2) Σ (g cos mλ + h sin mλ) P̆(sin φ′)
   ```

   `P̆` are the Schmidt semi-normalized associated Legendre functions ([WMM] eq. 5).
3. **Turned into the local frame** (eq. 17) by the angle `φ′ − φ` between the two latitudes, which
   gives `X`, `Y`, `Z` in the site's north, east and down directions.
4. **The elements** (eq. 19): horizontal intensity `H = √(X² + Y²)`, total intensity
   `F = √(H² + Z²)`, inclination `I = atan2(Z, H)` and declination `D = atan2(Y, X)`.

The same sums over the coefficients' rates `ġ` and `ḣ` give how fast each part changes per year
(eqs. 13 to 15, 18 and 20), written with a dot: `Ẋ`, `Ḋ` and so on (`dD` in the example).

Two details differ from the report's printed text, and NOAA's test values settle both:

- **The poles.** Equation 11 and the derivative in equation 16 divide by `cos φ′`, which is zero
  at a pole. hpr writes each function as `cosᵐφ′` times a polynomial in `sin φ′`, so the division
  cancels exactly and the field at a pole is the limit along the meridian given ([WMM] section
  1.4). A test checks the report's printed field over each pole.
- **A sign in equation 15.** The rate of `Z′` is printed with `ġ cos mλ − ḣ sin mλ`. The potential
  gives `+`, as in equation 12, and only `+` reproduces NOAA's rates.

### Grid variation

Near the geographic poles, declination swings with every step east or west, so polar navigators
use **grid variation** instead ([WMM] eq. 1): the angle from a map grid's north, that of the polar
stereographic grid (Table 6's note), to magnetic north. It is `D − λ` north of 55° N, and `D + λ`
south of 55° S. Elsewhere hpr gives none (`None`), as the test values print `NaN`.

### How far a compass can be trusted

The report marks a **blackout zone** around each magnetic pole, where the horizontal intensity `H`
is under 2,000 nT and declination can be wrong by up to 180°, and a **caution zone** around it,
under 6,000 nT, where declination errors exceed 1° ([WMM] section 1.8).
`MagneticField::compass_zone` says which applies: `Reliable`, `Caution` or `Blackout`. The report
draws the zones on the ground; hpr uses the field at the height asked, which is a little weaker
higher up, so its zones there are a little wider.

The report's error model ([WMM] eq. 43) gives the declination's expected error, one standard
deviation, in degrees:

```text
δD = √(0.26² + (5417 / H)²)      H in nT
```

That is 0.29° where the horizontal field is strongest and grows without bound toward a magnetic
pole. `MagneticField::declination_uncertainty_rad` returns it. It covers the coefficients'
errors, the drift of the forecast to 2030, local rocks the model leaves out, and magnetic storms;
it does not cover a steel rail, a car or a motor case beside the compass.

## A worked example

The example program
[`crates/hpr/examples/declination.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/declination.rs)
asks for the field at four launch sites on 20 June 2026. It turns the date into a decimal year
with `decimal_year` (the start of that day, 2026 + 170/365), then calls `WMM2025.field` with each
site's latitude, longitude and height above the ellipsoid. Run it from a copy of the repository
with `cargo run --example declination -p hpr`. It prints:

<!-- quote: crates/hpr/examples/declination.output.txt -->
```text
WMM-2025 on 2026-06-20 (decimal year 2026.4658)

site                             D (°)  ± (°)  dD (°/yr)   I (°)  F (nT)  GV (°)  zone
Spaceport America, New Mexico     7.75   0.35     -0.076   59.89   46981       -  Reliable
Black Rock Desert, Nevada        12.85   0.36     -0.096   64.24   49646       -  Reliable
Lucerne Dry Lake, California     11.16   0.35     -0.079   59.38   46407       -  Reliable
Andøya, Norway                    9.41   0.56      0.268   78.16   53674   -6.61  Reliable

At Spaceport America, New Mexico, a rail aimed at magnetic north points 7.75° east of true north.
```

`±` is the model's own error estimate, `dD` the declination's drift per year, `GV` the grid
variation (only at Andøya, north of 55°). The drift is small: Spaceport America's declination
falls by about 0.08° a year, so it moves well under a degree over the model's five years.

## The test values

| check | points | what is compared | worst difference | held to |
|---|---|---|---|---|
| the coefficient table | 90 rows | every coefficient against `WMM2025.COF` | none | equal |
| the report's Table 6 | 12 | `X`, `Y`, `Z`, `H`, `F`, their rates; `I`, `D`, grid variation, their rates | half the last digit | 0.05 nT, 0.005° |
| the report's Table 3b | 1 | `φ′`, `r`, the coefficients, `X′`, `Y′`, `Z′`, `X` … `Ḋ`, printed to 10 decimals | 5.0e-11 nT on `X′` | half the last digit, plus 8 units in the last place |
| the report's poles (section 1.4) | 2 | `X`, `Y`, `Z` over each pole at `r = a` | inside 0.05 nT | 0.05 nT |
| NCEI's high-precision file | 100 | `Y`, `D`, `I`, and the rates of `Y`, `Z`, `D`, `I` | half the last digit | 5e-7 nT, 0.005° |
| the same file | 100 | `X`, `H`, `F` | 7.18e-4 nT | 7.2e-4 nT, the measured worst |
| the same file | 100 | `Z`; the rates of `X`, `H`, `F` | 2.2e-6 nT; 1.5e-6 nT/yr | the measured worst |
| the potential, by differences | 100 | hpr's `X′` and `Ẋ′` | 1.3e-7 nT | 1e-6 nT |

Rows are counted from 0. The north component `X` in NCEI's file differs from hpr's at 97 of its
100 points, by up to 7.18e-4 nT (row 35: 2026.5, 12 km, 33° N, 145° W). That is at most 2.11e-8
of the total field, and 140 times smaller than the 0.1 nT the report allows for single precision
(the note under its Table 6).

The tests show where the difference comes from:

- **It lies in one quantity, the geocentric `X′`.** If the file's `X′` is off by an amount `e`,
  its `X` moves by `e cos(φ′ − φ)` and its `Z` by `e sin(φ′ − φ)`. Taking `e` from each point's `X`
  and applying it to `Z` brings the file's `Z` to within 4.9e-7 nT of hpr's, inside its printing.
- **hpr's `X′` is right.** The tests take the potential's derivative in latitude by differences,
  independently of hpr's formula for it: hpr's `X′` matches to 1.3e-7 nT at all 100 points. The
  report's own ten-decimal `X′` (Table 3b) matches to 5e-11 nT.
- **A second program agrees with hpr.** pygeomag 1.1.0 (MIT), a port of NOAA's own `geomag`
  program, run once by hand on two of the points (rows 3 and 35, not in CI), gives hpr's `X` to
  7e-7 nT, so it differs from the file by the same amount.

The rate of `X` differs too, by up to 9.5e-7 nT a year at 24 points: a second, smaller residue
whose cause is not known. hpr's `Ẋ′` passes the same derivative check. The rates of `H` and `F`
follow from `X` and its rate.

So the file's `X`, `H`, `F` and `Z`, and the rates of `X`, `H` and `F`, are held to the measured
differences, not to the file's printing. [ADR-125][adr-125] records the decision.

A property test also checks, at random places and times, that `H`, `F`, `I` and `D` agree with
`X`, `Y`, `Z` by their definitions, and that `F` lies between 20,000 and 70,000 nT. The report's
Table 1 rounds the surface range to 23,000 to 67,000 nT, but the model itself falls to about
21,900 nT over South America by 2030, which a test pins at 26° S, 61° W.

## Limits

- **Five years only.** WMM2025 covers 2025.0 to 2030.0. A flight log from 2024 needs WMM2020,
  which hpr does not bundle; such a date is refused, not extrapolated.
- **Heights from −1 km to 850 km.** The model is specified from 1 km below the WGS 84 ellipsoid
  to 850 km above it ([WMM] section 3); outside that, a height is refused.
- **Height above the ellipsoid.** Heights are above WGS 84, as `Geodetic` holds them. A site's
  height above sea level (from a map, a GPS or an altimeter) differs by the geoid, up to about
  100 m; the report puts that effect at about 1 nT or less, far below the model's error, so a
  height above sea level can be used as it is.
- **The main field only.** The WMM leaves out the crust's local fields, which can move a compass
  by degrees near iron ore or volcanic rock, and the fields of magnetic storms. The error
  estimate allows for them as a worldwide average; at any one site they can be larger.
- **Nothing in a flight uses it yet.** Converting a compass heading is the caller's step, with
  `MagneticField::true_from_magnetic_rad`.

[adr-125]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-125-site-data-split-and-wmm2025-in-hpr-core-2026-09-30
