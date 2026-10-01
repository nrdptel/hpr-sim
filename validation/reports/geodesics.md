# Geodesics against Karney's test set

`hpr_core::geodesic` (M5.3c1, ADR-127) against C. F. F. Karney's *Test set for geodesics*
(`GeodTest.dat`, doi:10.5281/zenodo.32156, CC0): 500,000 WGS 84 geodesics, each solved as an
inverse problem (two places to distance and azimuths) and a direct one (a place, an azimuth and a
distance to the far place). Each cell is the largest error of its kind, in nanometres:

- **Inverse `s₁₂`:** the distance error.
- **Inverse landing:** how far the direct problem from point 1, with the inverse's own `α₁` and
  `s₁₂`, lands from point 2.
- **Inverse azimuth × `m₁₂`:** the larger azimuth error times the reduced length `m₁₂`, the
  sideways miss it stands for at the other end.
- **Over 15 nm, ill-conditioned:** lines whose inverse azimuth passes Karney's 15 nm, each
  checked to be smaller than the turn a one-ulp move of an input gives.
- **Direct position:** the far place's miss, in Earth-centred coordinates.
- **Direct azimuth × `m₁₂`:** the angle between the computed and the set's heading at the far
  place, as directions in space, times `m₁₂`.

Every value but the ill-conditioned azimuths is held to Karney's 15 nm (*Algorithms for
geodesics*, 2013, §7). `crates/hpr-core/tests/geodtest.rs` writes the table with
`HPR_WRITE_GEODESICS=1` where `refs/sources/geodtest/GeodTest.dat` is fetched, and otherwise
checks it; CI, without `refs/`, checks every 500th line.

<!-- table: written by crates/hpr-core/tests/geodtest.rs -->
| Kind | Lines | Inverse `s₁₂` | Inverse landing | Inverse azimuth × `m₁₂` | Over 15 nm, ill-conditioned | Direct position | Direct azimuth × `m₁₂` |
|---|---:|---:|---:|---:|---:|---:|---:|
| random | 100000 | 7.45 | 10.27 | 8.41 | 0 | 11.34 | 10.35 |
| nearly antipodal | 50000 | 11.18 | 10.45 | 4.65 | 0 | 11.97 | 0.22 |
| short distances | 50000 | 3.42 | 3.95 | 3.24 | 0 | 4.75 | 0.00 |
| one end near a pole | 50000 | 7.45 | 10.21 | 5.66 | 0 | 8.65 | 6.75 |
| both ends near opposite poles | 50000 | 7.45 | 8.53 | 2.99 | 0 | 10.00 | 0.00 |
| nearly meridional | 50000 | 7.45 | 8.39 | 8.49 | 0 | 9.74 | 7.52 |
| nearly equatorial | 50000 | 11.18 | 11.22 | 2.29 | 0 | 14.02 | 8.89 |
| between vertices | 50000 | 7.45 | 8.75 | 0.00 | 0 | 11.00 | 0.00 |
| ending close to vertices | 50000 | 11.18 | 11.26 | 75.33 | 2 | 13.19 | 0.00 |
<!-- end of table -->
