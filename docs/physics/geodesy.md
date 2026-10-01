# Geodesy: the ellipsoid, coordinates, distance and bearing

## In short

- **What it models:** the Earth's shape, as the WGS 84 ellipsoid (a sphere slightly flattened at
  the poles); conversions between latitude, longitude and height and Earth-centred x, y, z; the
  local east, north and up directions; and the distance and bearing between two places.
- **Sources:** the NGA's WGS 84 standard, NGA.STND.0036 (2014); C. F. F. Karney, *Geodesics on
  an ellipsoid of revolution* (2011), appendix B; C. F. F. Karney, *Algorithms for geodesics*
  (2013), through the `geographiclib-rs` crate.
- **How well it is validated:** the derived ellipsoid values reproduce the standard's Table 3.5
  to its printed digits. In unit tests, random round trips return latitude within 1e-14 rad and
  height within 2e-8 m, from −10 km to +1000 km. Distances and bearings match all 500,000 lines
  of Karney's published test set within 15 nanometres
  ([below](#distance-and-bearing-geodesics)). The conversions are not compared with another
  library, a simulator or a real flight.
- **What it leaves out:** height above sea level, which needs the geoid, up to about 100 m from
  the ellipsoid ([Frames](frames.md#earth-centred-earth-fixed-ecef)). hpr has no geoid model; a
  flight takes that difference at the site as an input.

## Sources

Code: `hpr_core::geodesy` and `hpr_core::geodesic`. Conventions: [Frames](frames.md).

- **[NGA]** NGA.STND.0036_1.0.0_WGS84, *Department of Defense World Geodetic System 1984, Its
  Definition and Relationships with Local Geodetic Systems*, 2014-07-08. Pinned as
  `wgs84-nga-stnd-0036`. US government work.
- **[Karney]** C. F. F. Karney, *Geodesics on an ellipsoid of revolution*, arXiv:1102.1215v1,
  2011, appendix B. Pinned as `karney-2011-geodesics`.
- **[Karney2013]** C. F. F. Karney, *Algorithms for geodesics*, J. Geodesy 87 (2013) 43–55,
  arXiv:1109.4448v2. Pinned as `karney-2013-algorithms-for-geodesics`.
- **[GeodTest]** C. F. F. Karney, *Test set for geodesics*, doi:10.5281/zenodo.32156, CC0.
  Pinned as `karney-geodtest`.

## WGS 84 ellipsoid

- **Defining parameters ([NGA] Table 3.1):** `a = 6378137.0 m` and `1/f = 298.257223563`.
- **Derived values:** `b = a(1 − f)`, `e² = f(2 − f)` and `E = a e`.
- **Check:** these reproduce [NGA] Table 3.5 to its printed digits
  (`wgs84_derived_geometry_matches_table_3_5`):

  | quantity | value |
  |---|---|
  | `b` | 6356752.3142 m |
  | `e²` | 6.694379990141e-3 |
  | `E` | 5.2185400842339e5 m |

## Geodetic to ECEF ([NGA] eqs. 4-14, 4-15)

```text
N = a / √(1 − e² sin²φ)
X = (N + h) cos φ cos λ,   Y = (N + h) cos φ sin λ,   Z = ((b²/a²) N + h) sin φ
```

The code writes `b²/a²` as `1 − e²`.

## ECEF to geodetic ([Karney] appendix B)

This is Vermeille's closed form, which Karney extended to cover points near the Earth's centre.

**Setup.** Let `R = √(X² + Y²)`, `x = R/a` and `y = √(1 − e²) Z/a`. The geodetic latitude
follows from the largest real root `κ` of

```text
κ⁴ + 2e²κ³ − (x² + y² − e⁴)κ² − 2e²y²κ − e⁴y² = 0                          (B1)
```

**Solving for `u`.**

```text
r = (x² + y² − e⁴)/6,   S = e⁴x²y²/4,   d = S(S + 2r³)
d ≥ 0:  T = (S + r³ ± √d)^(1/3), sign of √d = sign of S + r³, real cube root
        u = r + T + r²/T   (u = 0 if T = 0)
d < 0:  ψ = ph(−S − r³ + i√(−d)),   u = r(1 + 2 cos(ψ/3))
```

**Solving for `κ`.**

```text
v = √(u² + e⁴y²)
v + u = e⁴y²/(v − u) when u < 0   (avoids cancellation)
w = ((v + u) − y²) e²/(2v)
κ = (v + u) / (√((v + u) + w²) + w)                                          (B5)
```

**Latitude, height and longitude.**

```text
φ = ph(R/(κ + e²) + iZ/κ)                                                    (B2)
h = (1 − (1 − e²)/κ) √(D² + Z²),   D = κR/(κ + e²)                            (B3)
λ = ph(X + iY)
```

`ph` is the argument, computed with `atan2`; `λ = 0` on the axis.

The closed form fails only in the equatorial plane within `a e²` (42.7 km) of the centre. There
the paper needs its limiting forms (B6)–(B7); the code returns `CoreError::Domain` instead.

**Measured accuracy** (property tests, 256 cases each per run):

- geodetic → ECEF → geodetic: latitude within 1e-14 rad and height within 2e-8 m, from
  −10 km to +1000 km;
- ECEF → geodetic → ECEF: within 1e-7 m per 6400 km of radius, out to 46,000 km.

## Local ENU axes

`ecef_from_enu_rotation(φ, λ)` has columns `ê`, `n̂` and `û` ([Frames](frames.md)). A test checks that `û`
equals the normalized gradient of `x²/a² + y²/a² + z²/b²` at the foot point, i.e. the ellipsoid
normal.

## Distance and bearing: geodesics

How far is the landing from the pad, and in which direction? On a flat map that is Pythagoras;
on the Earth it is a **[geodesic](../glossary.md#geodesic)**, the shortest path over the
ellipsoid's surface. Its length is the distance, and its direction where it leaves is the
[bearing](../glossary.md#bearing). `hpr_core::geodesic` solves the two classic problems on any
`Ellipsoid`:

- **Inverse** (`Ellipsoid::geodesic_inverse`): from two places, the distance `s₁₂` and the
  azimuths `α₁` (leaving the first) and `α₂` (arriving at the second).
- **Direct** (`Ellipsoid::geodesic_direct`): from a place, a bearing `α₁` and a distance, the
  place you reach and the azimuth `α₂` there.

Azimuths are clockwise from true north, in radians from −π to π. `α₂` is the direction of travel
on arrival, so the bearing back to the start is `α₂ ± π`; over a long path it differs from
`α₁ ± π` because meridians converge. Heights are ignored: the path runs on the ellipsoid's
surface, and the direct problem's end has height 0.

**Method.** The code is Karney's GeographicLib, as georust's `geographiclib-rs` 0.2.7 (MIT) ports
it ([ADR-127 decision record][adr-127]). [Karney2013] maps the ellipsoid onto an auxiliary sphere,
where the geodesic is a great circle, and corrects distance and longitude with series in the
flattening to sixth order (§3 to §5). The inverse finds `α₁` by Newton's method on that sphere.
Karney states that round-off stays under 15 nm (nanometres) in both problems on WGS 84, and that
up to `f` = 1/150 the series' truncation is smaller still (§7, page 10).

**Worked example.** A pad at 32.9904° N, 106.9750° W and a landing at 33.0000° N, 106.9680° W:

| quantity | value |
|---|---|
| distance `s₁₂` | 1,249.614 m |
| bearing from the pad `α₁` | 31.567° |
| azimuth on arrival `α₂` | 31.571° |

The other way round, 2 km from the same pad on a bearing of 60° reaches 32.999415° N,
106.956466° W, heading 60.010°. A flat-Earth estimate of the first (1,065 m north, 654 m east)
gives 1,250 m at 31.6°; the geodesic matters over long paths and where precision does. A unit
test, `the_guides_worked_example`, holds these numbers.

**Validation.** [GeodTest] is Karney's set of 500,000 WGS 84 geodesics, worked in high precision
with each end known to 1e-18°, in nine kinds: random, nearly antipodal, short, one end near a
pole, both ends near opposite poles, nearly meridional, nearly equatorial, between vertices (where
a geodesic runs due east or west) and ending close to vertices. `crates/hpr-core/tests/geodtest.rs`
solves every line both ways and measures five errors, all held to Karney's 15 nm:

- the inverse's distance error;
- the inverse's *landing* miss: solve the direct problem from point 1 with the inverse's own
  `α₁` and `s₁₂`, and measure how far it lands from point 2;
- the inverse's azimuth errors times the reduced length `m₁₂`, the sideways miss an azimuth error
  stands for at the other end;
- the direct's end-point miss;
- the direct's heading error at the end, times `m₁₂`, compared as a direction in space (near a
  pole an azimuth swings through large angles as its point moves by nanometres).

The full table is in the
[geodesics report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/geodesics.md).
The largest errors over all 500,000 lines:

| error | largest | where |
|---|---|---|
| inverse distance | 11.18 nm | nearly antipodal, nearly equatorial, ending close to vertices |
| inverse landing | 11.26 nm | ending close to vertices |
| direct end point | 14.02 nm | nearly equatorial |
| direct heading × `m₁₂` | 10.35 nm | random |

One measure needs a caveat. On 2 nearly antipodal lines that end close to a vertex, the inverse's
azimuths are off by 3.15e-4 rad, 75.33 nm times `m₁₂`. These azimuths are ill-conditioned: the
set's ends are rounded to the nearest `f64` on reading, and moving the far end by one step of
that rounding (1.7 nm) turns `α₁` by 3.2e-4 rad, more than the error. The test checks that line
by line. The distance and the landing on those lines are within the 15 nm.

Every 500th line (1,000 of them) is committed and checked in CI; the whole set is checked where
`cargo xtask refs fetch` has downloaded it, against the committed report.

**What it leaves out.** Heights: two places at 3,000 m are as far apart as the same places at
sea level. Only WGS 84 is measured; another `Ellipsoid` rests on Karney's method. Nothing in a
flight uses geodesics yet, and `hpr` has no command for them.

[Karney2013]: https://arxiv.org/abs/1109.4448
[GeodTest]: https://doi.org/10.5281/zenodo.32156
[adr-127]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-127-m53c1-geodesics-through-geographiclib-held-to-karneys-test-set-2026-09-30
