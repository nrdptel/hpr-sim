# Geodesy: the ellipsoid, coordinates, distance and bearing

## In short

- **What it models:** the Earth's shape, as the WGS 84 ellipsoid (a sphere slightly flattened at
  the poles); conversions between latitude, longitude and height and Earth-centred x, y, z; the
  local east, north and up directions; and the distance and bearing between two places.
- **Sources:** the NGA's WGS 84 standard, NGA.STND.0036 (2014); C. F. F. Karney, *Geodesics on
  an ellipsoid of revolution* (2011), appendix B; C. F. F. Karney, *Algorithms for geodesics*
  (2013), through the `geographiclib-rs` crate.
- **How well it is validated:** for the conversions, the derived ellipsoid values reproduce the
  standard's Table 3.5 to its printed digits, and in unit tests random round trips return
  latitude within 1e-14 rad and height within 2e-8 m, from −10 km to +1000 km; they are not
  compared with another library, a simulator or a real flight. For distance and bearing, every
  one of the 500,000 lines of Karney's published test set is matched within 15 nanometres (nm,
  billionths of a metre): distance within 11.18 nm, the far point within 14.02 nm, and the
  bearings within 15 nm of sideways miss ([below](#distance-and-bearing-geodesics)). CI checks
  every 500th line and the 21 mirror lines; the whole set is checked where it has been
  downloaded, and has been measured on macOS.
- **What it leaves out:** height above sea level, which needs the geoid, up to about 100 m from
  the ellipsoid ([Frames](frames.md#earth-centred-earth-fixed-ecef)). hpr has no geoid model; a
  flight takes that difference at the site as an input. Nothing in a flight uses distance and
  bearing yet: the landing distance `hpr` prints is measured on a flat map from the pad's east and
  north offsets, and there is no command for geodesics.

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
[bearing](../glossary.md#bearing). Geodesists call a bearing an *azimuth*; on this page the two
words mean the same. `hpr_core::geodesic` solves the two classic problems
([API reference](https://nrdptel.github.io/hpr-sim/api/hpr_core/geodesic/index.html)):

- **Inverse** (`Ellipsoid::geodesic_inverse`): from two places, the distance `s₁₂` and the
  azimuths `α₁` (leaving the first) and `α₂` (arriving at the second).
- **Direct** (`Ellipsoid::geodesic_direct`): from a place, a bearing `α₁` and a distance, the
  latitude and longitude you reach and the azimuth `α₂` there.

Azimuths are clockwise from true north, in radians from −π to π, so due west is −π/2. For the 0°
to 360° a compass uses, take `.to_degrees().rem_euclid(360.0)`. `α₂` is the direction of travel
on arrival, so the bearing back to the start is `α₂ ± π`; over a long path it differs from
`α₁ ± π` because meridians converge. Heights are ignored: the path runs on the ellipsoid's
surface, and the direct problem's end has no height until you give it one.

**Method.** The code is Karney's GeographicLib, as georust's `geographiclib-rs` 0.2.7 (MIT) ports
it. hpr uses the crate rather than a port of its own, so the code is Karney's line for line
([ADR-127 decision record][adr-127]). [Karney2013] maps the ellipsoid onto an auxiliary sphere,
where a geodesic is a great circle (the sphere's shortest path), and corrects distance and
longitude with series in the flattening to sixth order (§2). The inverse finds `α₁` by Newton's
method (§4), from a starting guess (§5). Karney states that round-off stays under 15 nm in both
problems on WGS 84 (§7, page 10), and that up to a flattening of 1/150 the series' truncation is
smaller still (page 9). Past 1/150 the series lose accuracy, so hpr refuses such an ellipsoid;
WGS 84's flattening is 1/298. The familiar haversine formula treats the Earth as a sphere, which
the Earth is not; Vincenty's ellipsoidal method is less accurate than Karney's, and its inverse
sometimes fails to converge (§7).

**Worked example.** A pad at 32.9904° N, 106.9750° W and a landing at 33.0000° N, 106.9680° W:

| quantity | value |
|---|---|
| distance `s₁₂` | 1,249.614 m |
| bearing from the pad `α₁` | 31.567° |
| azimuth on arrival `α₂` | 31.571° |

The other way round, 2 km from the same pad on a bearing of 60° reaches 32.999415° N,
106.956466° W, heading 60.010°. At this range a flat map gives the same distance, if it uses
the ellipsoid's curvature at the middle latitude: 1,249.614 m again, to under a millimetre. The
geodesic matters over long paths, where a flat map's error grows. A unit test,
`the_guides_worked_example`, holds these numbers.

**Validation.** [GeodTest] is Karney's set of 500,000 WGS 84 geodesics. They were worked
separately from the code, with the series carried to thirtieth order and high-precision
arithmetic, each end known to 1e-18°, so they check this code's method and its rounding. They come
in nine kinds of 50,000 or 100,000 lines each, listed in the
[geodesics report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/geodesics.md):
random pairs, nearly antipodal pairs (almost opposite sides of the Earth), short paths, paths
near the poles, nearly along a meridian or the equator, and paths near a *vertex*, the point
where a geodesic runs due east or west. `crates/hpr-core/tests/geodtest.rs` solves every line both
ways and measures five errors, each held to Karney's 15 nm on every line:

| error | what it is | largest |
|---|---|---|
| inverse distance | the computed `s₁₂` against the set's | 11.18 nm |
| inverse landing | solve the direct problem with the inverse's own `α₁` and `s₁₂`, and measure how far it lands from the second place | 11.26 nm |
| inverse azimuths × `m₁₂` | each azimuth's error times the *reduced length* `m₁₂`, how far the far end moves sideways per radian the start's bearing turns: the sideways miss the error stands for | 8.49 nm |
| direct end point | the computed end against the set's, in Earth-centred coordinates | 14.02 nm |
| direct heading × `a` | the angle between the computed and the set's direction of travel at the end, as directions in space, times the Earth's radius `a` | 13.99 nm |

The inverse's azimuths can't be checked on the 50,000 "between vertices" lines: there `m₁₂` is
at most 1e-13 m, so any azimuth error reads as no miss. Their distance and landing are checked.

**Mirror lines: two paths of the same length.** When the second place's latitude is exactly the
first's negated (`φ₂ = −φ₁`) and the two azimuths differ (`α₁ ≠ α₂`), two geodesics of the same
length join the places, one the mirror of the other, and the second has `α₁` and `α₂` swapped
(GeographicLib's `GeodSolve` manual, *Multiple solutions*). Either answer is right. Where
`α₁ = α₂`, as on the between-vertices lines, the geodesic is unique. The set has 21 mirror lines
once its numbers are read as `f64` (the 64-bit floating-point numbers hpr computes in). They are
all nearly antipodal, with `m₁₂` under a centimetre, so their azimuths are nearly undetermined.
On these lines the test scores hpr's answer against whichever pair it is nearer to, and that
error is within 15 nm on all 21; on 4 the nearer pair is the swapped one.

CI checks every 500th line (1,000 of them) and the 21 mirror lines, which are committed. The
whole set is checked where `cargo xtask refs fetch` has downloaded it, against the committed
report. The whole set has been measured only on macOS, by the debug build that wrote the report's
table; a release build there moves three cells by up to 1.83 nm, and on any other build the test
holds the 15 nm bound without comparing the table.

**What it leaves out.** Heights: two places at 3,000 m are as far apart as the same places at
sea level. Only WGS 84 is measured; on any other ellipsoid up to a flattening of 1/150, the
accuracy is Karney's claim, not something hpr has measured. A distance of many trips round the
Earth carries its own rounding, one step of `f64` in the distance (at least 15 nm past 67,109 km). Nothing
in a flight uses geodesics yet, and `hpr` has no command for them.

[Karney2013]: https://arxiv.org/abs/1109.4448
[GeodTest]: https://doi.org/10.5281/zenodo.32156
[adr-127]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-127-m53c1-geodesics-through-geographiclib-held-to-karneys-test-set-2026-09-30
