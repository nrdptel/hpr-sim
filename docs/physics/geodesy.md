# Geodesy: the ellipsoid and coordinate conversions

## In short

- **What it models:** the Earth's shape, as the WGS 84 ellipsoid (a sphere slightly flattened at
  the poles); conversions between latitude, longitude and height and Earth-centred x, y, z; and
  the local east, north and up directions.
- **Sources:** the NGA's WGS 84 standard, NGA.STND.0036 (2014); C. F. F. Karney, *Geodesics on
  an ellipsoid of revolution* (2011), appendix B.
- **How well it is validated:** the derived ellipsoid values reproduce the standard's Table 3.5
  to its printed digits. In unit tests, random round trips return latitude within 1e-14 rad and
  height within 2e-8 m, from −10 km to +1000 km. Not compared with another library, a simulator
  or a real flight.
- **What it leaves out:** height above sea level, which needs the geoid, up to about 100 m from
  the ellipsoid. hpr has no geoid model; a flight takes that difference at the site as an input.

## Sources

Code: `hpr_core::geodesy`. Conventions: [Frames](frames.md).

- **[NGA]** NGA.STND.0036_1.0.0_WGS84, *Department of Defense World Geodetic System 1984, Its
  Definition and Relationships with Local Geodetic Systems*, 2014-07-08. Pinned as
  `wgs84-nga-stnd-0036`. US government work.
- **[Karney]** C. F. F. Karney, *Geodesics on an ellipsoid of revolution*, arXiv:1102.1215v1,
  2011, appendix B. Pinned as `karney-2011-geodesics`.

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
