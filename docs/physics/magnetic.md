# The Earth's magnetic field and declination

## In short

- **What it models:** the Earth's main magnetic field at any place and time from 2025.0 to
  2030.0, from the World Magnetic Model (WMM2025). Its most useful output is the
  **declination**: the angle between true north and the north a compass shows. Use it to turn a
  compass bearing (a launch rail's heading, say) into a true one, or back.
- **Sources:** A. Chulliat, W. Brown, M. Nair and others, *The US/UK World Magnetic Model for
  2025–2030: Technical Report*, NOAA NCEI (2025), with NCEI's coefficient file `WMM2025.COF`,
  which NCEI places in the public domain.
- **How well it is validated:** hpr reproduces all 12 of the report's test points (its Table 6)
  to their last printed digit, declination to 0.005°. Of NCEI's 100 high-precision points, the
  declination, the inclination and the east component match to their last printed digit; the
  north component matches to 7.2e-4 nT, 3 parts in 100 million of the field (see
  [The test values](#the-test-values)). That shows the model is computed correctly. The model
  itself is only as good as the Earth allows: its own error estimate for declination is 0.29° at
  best and 0.35° to 0.56° at the example's sites ([A worked example](#a-worked-example)), more
  near the magnetic poles. Each result carries that estimate.
- **What it leaves out:** local magnetic rocks, magnetic storms and the steel near a compass.
  Dates outside 2025.0 to 2030.0 are refused, and so are heights below −1 km or above 850 km
  ([Limits](#limits)).

## Sources

Code: `hpr_core::magnetic`. Conventions: [Frames](frames.md) and [Geodesy](geodesy.md).

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

The field also dips into the ground. The **inclination** `I` is that dip, positive downward: about
60° across the southern United States, and 90° at a magnetic pole, where a compass has nothing
horizontal to follow.

## The model

The WMM writes the field's potential as a sum of spherical harmonics, 90 of them up to degree 12,
each with a coefficient in nanoteslas (nT; the Earth's field is 20,000 to 70,000 nT). Each
coefficient changes linearly in time from its 2025.0 value ([WMM] eq. 9):

```text
g(t) = g(2025.0) + (t − 2025.0) ġ
```

where `t` is the decimal year. hpr computes the field the way [WMM] section 1.2 sets out:

1. **To geocentric coordinates** (eqs. 7, 8): the site's latitude `φ`, longitude `λ` and height
   `h` above the WGS 84 ellipsoid become a radius `r` and a geocentric latitude `φ′`, the angle seen
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

The same sums over the coefficients' rates give how fast each part changes per year (eqs. 13 to
15, 18 and 20).

Two details differ from the report's printed text, and NOAA's test values settle both:

- **The poles.** Equation 11 and the derivative in equation 16 divide by `cos φ′`, which is zero
  at a pole. hpr writes each function as `cosᵐφ′` times a polynomial in `sin φ′`, so the division
  cancels exactly and the field at a pole is the limit along the meridian given ([WMM] section
  1.4). A test checks the report's printed field over each pole.
- **A sign in equation 15.** The rate of `Z′` is printed with `ġ cos mλ − ḣ sin mλ`. The potential
  gives `+`, as in equation 12, and only `+` reproduces NOAA's rates.

### Grid variation

Near the geographic poles, declination swings with every step east or west, so polar navigators
use **grid variation** instead ([WMM] eq. 1): `D − λ` north of 55° N, and `D + λ` south of 55° S.
Elsewhere hpr gives none (`None`), as the test values print `NaN`.

### How far a compass can be trusted

The report marks a **blackout zone** around each magnetic pole, where the horizontal intensity `H`
is under 2,000 nT and declination can be wrong by up to 180°, and a **caution zone** around it,
under 6,000 nT ([WMM] section 1.8). `MagneticField::compass_zone` says which applies.

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
| the same file | 100 | `X`, `H`, `F` | 7.19e-4 nT | 7.2e-4 nT |
| the same file | 100 | `Z`; the rates of `X`, `H`, `F` | 2.2e-6 nT; 1.5e-6 nT/yr | as measured |

The north component `X` in NCEI's high-precision file differs from hpr's by up to 7.19e-4 nT
(at 2026.5, 12 km, 33° N, 145° W). The difference lies entirely in the geocentric north part `X′`:
one residue there explains the file's `X` and `Z` together, and once it is taken out `Z` agrees to
the file's last digit. The report's own ten-decimal worked example (Table 3b) agrees with hpr's
`X′` to 5e-11 nT. A second, independent implementation, pygeomag 1.1.0 (MIT), run once by hand,
gives hpr's `X` to 7e-7 nT and the same residue. So the file is held to its measured residue rather
than to its printing. The residue's cause in NCEI's program is not known. It is 3 parts in 100
million of the field, and 140 times smaller than the 0.1 nT the report allows for single
precision. [ADR-125][adr-125] records the decision.

A property test also checks, at random places and times, that `H`, `F`, `I` and `D` agree with
`X`, `Y`, `Z` by their definitions, and that `F` lies between 20,000 and 70,000 nT. The report's
Table 1 rounds the surface range to 23,000 to 67,000 nT, but the model itself falls to about
21,900 nT over South America by 2030.

## Limits

- **Five years only.** WMM2025 covers 2025.0 to 2030.0. A flight log from 2024 needs WMM2020,
  which hpr does not bundle; such a date is refused, not extrapolated.
- **Heights from −1 km to 850 km.** The model is specified from 1 km below the WGS 84 ellipsoid
  to 850 km above it ([WMM] section 3); outside that, a height is refused.
- **Height above the ellipsoid.** Heights are above WGS 84, as `Geodetic` holds them. A site's
  height above sea level differs by the geoid, up to about 100 m; the report puts that effect at
  about 1 nT, far below the model's error.
- **The main field only.** The WMM leaves out the crust's local fields, which can move a compass
  by degrees near iron ore or volcanic rock, and the fields of magnetic storms. Both are inside
  the error estimate on average, not at any one place.
- **Nothing in a flight uses it yet.** Converting a compass heading is the caller's step, with
  `MagneticField::true_from_magnetic_rad`.

[adr-125]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-125-site-data-split-and-wmm2025-in-hpr-core-2026-09-30
