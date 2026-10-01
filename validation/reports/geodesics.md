# Geodesics against Karney's test set

`hpr_core::geodesic` ([M5.3c1](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m5-3c1), distance and bearing
on WGS 84; [ADR-127](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-127-m53c1-geodesics-through-geographiclib-held-to-karneys-test-set-2026-09-30))
against C. F. F. Karney's *Test set for geodesics* (`GeodTest.dat`, doi:10.5281/zenodo.32156,
CC0, sha256 `c1cabdddbcd7d5cfc6e6111db4608fa55be292b15ba2c5bcd6372a178848c692`), solved through
`geographiclib-rs` 0.2.7. Its 500,000 WGS 84 geodesics are each solved as an inverse problem (two
places to distance and azimuths) and a direct one (a place, an azimuth and a distance to the far
place). Each cell is the largest error of its kind, in nanometres (nm), and every one is held to
Karney's 15 nm (*Algorithms for geodesics*, 2013, §7):

- **Inverse `s₁₂`:** the distance error.
- **Inverse landing:** how far the direct problem from point 1, with the inverse's own `α₁` and
  `s₁₂`, lands from point 2.
- **Inverse azimuth × `m₁₂`:** the larger azimuth error times the reduced length `m₁₂`, the
  sideways miss it stands for at the other end. Not measured where every `m₁₂` is about zero.
- **Mirror lines (swapped):** lines with `φ₂ = −φ₁` exactly once read as `f64` and `α₁ ≠ α₂`,
  where two geodesics of the same length have `α₁` and `α₂` swapped; the azimuths are scored
  against whichever pair is nearer, and the bracket counts the lines nearer the swapped pair.
- **Direct position:** the far place's miss, in Earth-centred coordinates.
- **Direct heading × `a`:** the angle between the computed and the set's direction of travel at
  the far place, as directions in space, times the equatorial radius `a`.

The kinds start at lines 1, 100,001, 150,001 and every 50,000 after. The table is written by a
debug build on macOS aarch64 (`crates/hpr-core/tests/geodtest.rs` with `HPR_WRITE_GEODESICS=1`,
where `refs/sources/geodtest/GeodTest.dat` is fetched) and checked by the same build; other builds
move some cells by a few nanometres and are held to the 15 nm bound only. CI, without `refs/`,
checks every 500th line and the 21 mirror lines.

<!-- table: written by crates/hpr-core/tests/geodtest.rs -->
| Kind | Lines | Inverse `s₁₂` | Inverse landing | Inverse azimuth × `m₁₂` | Mirror lines (swapped) | Direct position | Direct heading × `a` |
|---|---:|---:|---:|---:|---:|---:|---:|
| random | 100000 | 7.45 | 10.27 | 8.41 | 0 (0) | 11.34 | 11.34 |
| nearly antipodal | 50000 | 11.18 | 10.45 | 4.65 | 0 (0) | 11.97 | 11.85 |
| short distances | 50000 | 3.42 | 3.95 | 3.24 | 0 (0) | 4.75 | 9.43 |
| one end near a pole | 50000 | 7.45 | 10.21 | 5.66 | 0 (0) | 8.65 | 8.95 |
| both ends near opposite poles | 50000 | 7.45 | 8.53 | 2.99 | 0 (0) | 10.00 | 12.22 |
| nearly meridional | 50000 | 7.45 | 8.39 | 8.49 | 0 (0) | 9.74 | 10.01 |
| nearly equatorial | 50000 | 11.18 | 11.22 | 2.29 | 0 (0) | 14.02 | 13.99 |
| between vertices | 50000 | 7.45 | 8.75 | not measured (`m₁₂` ≤ 1e-13 m) | 0 (0) | 11.00 | 11.08 |
| ending close to vertices | 50000 | 11.18 | 11.26 | 3.24 | 21 (4) | 13.19 | 13.36 |
<!-- end of table -->
