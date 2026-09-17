# Nose cones, transitions and solids of revolution

Code: `hpr_design::shapes` (profiles) and `hpr_design::solids` (volume, centroid, moments, areas).

Sources:

- **[CR]** G. A. Crowell Sr., *The Descriptive Geometry of Nose Cones* (1996), pp. 1–6 and 12–14.
  Cited, not pinned: the only copy found is on a plain-http university mirror, and ADR-002 pins
  over https. [TD] gives the same curves.
- **[TD]** S. Niskanen, *OpenRocket technical documentation* v13.05 (2013), appendix A, pp. 102–106,
  pinned as `openrocket-techdoc-13.05`. This is the published document, not the program's source.

## Profiles

A profile gives the outer radius `r(x)` at distance `x` aft of its forward end, `0 ≤ x ≤ L`. Each
shape is a normalized curve `g(ξ)` with `g(0) = 0` at the tip and `g(1) = 1` at the base:

| shape | `g(ξ)` or `y(x)` | parameter | source |
|---|---|---|---|
| conical | `ξ` | — | [CR] p. 1; [TD] A.1 |
| ogive | `y = √(ρ² − (x − ρ cos α)²) + ρ sin α`, `α = atan(R/L) − acos(√(L² + R²)/2ρ)` | `ρ/ρ_t ≥ R/L` | [CR] p. 4 |
| elliptical | `√(1 − (1 − ξ)²)` | — | [TD] A.6 |
| power series | `ξⁿ` | `0 < n ≤ 1` | [CR] p. 2; [TD] A.8 |
| parabolic series | `(2ξ − K′ξ²)/(2 − K′)` | `0 ≤ K′ ≤ 1` | [CR] p. 5; [TD] A.7 |
| Haack series | `√((θ − sin 2θ/2 + C sin³θ)/π)`, `θ = acos(1 − 2ξ)` | `0 ≤ C ≤ 2/3` | [CR] p. 6; [TD] A.9–A.10 |

- **Ogive.** The shape is Crowell's secant ogive: an arc of radius `ρ` through the tip and the base
  rim, with `ρ` given as a multiple of the tangent radius `ρ_t = (R² + L²)/2R`.
  - `1` is the tangent ogive, whose slope is zero at the base.
  - Values above 1 meet the base at an angle.
  - Values below 1 bulge past `R` before the base.
  - The arc reaches the tip only while its centre is aft of it, which needs `ρ ≥ (L² + R²)/2L`.
- **OpenRocket's ogive parameter is not adopted.** [TD] contradicts itself about it:
  - A.3 defines it as `κ = ρ_t/ρ`.
  - A.4–A.5 describe the first `L` of a tangent ogive of length `L/κ`, which at `L = 4`, `R = 1`,
    `κ = ½` has `ρ = 24.8`, not `ρ_t/κ = 17.0`.
  - The `.ork` importer (M3.1) must settle the mapping by running the OpenRocket jar.
- **Haack.** Monotone for `C ≤ 2/3`, because `d(g²)/dθ = sin²θ (2 + 3C cos θ)/π`. `C = 0` is the
  von Kármán (LD-Haack) ogive and `C = 1/3` the LV-Haack. [TD] limits `C` to `1/3` in the program.
- **Blunt tips.** The elliptical, power-series (`n < 1`) and Haack slopes are infinite at the tip.
  `n = 0` (a flat cylinder) is rejected; model it as a tube and a bulkhead.
- **Errors in [CR].** Its prose says "greater than twice the length" for the bulged secant ogive,
  against its own formula. Its ogive and ellipsoid areas are wrong, and the author marked them so.
  Its elliptical formula measures `x` from the base, and its ellipse CP ratio should read `2L/3`.
  hpr uses none of those.

### Transitions

A transition runs from fore radius `R_f` to aft radius `R_a` over `L`.

- **Orientation.** The shape's tip lies at the **smaller** end. Growing aft,
  `r = R_f + (R_a − R_f) g(x/L)`. A boattail is the mirror image,
  `r = R_a + (R_f − R_a) g(1 − x/L)`.
  [TD] doesn't say how a shrinking transition is oriented; this choice keeps both ends' radii
  exact and the profile monotone (Loft lesson L49).
- **Clipped** ([TD] §A.7): cut a whole nose cone of base radius `max(R_f, R_a)` where its radius
  is `min(R_f, R_a)`, with the nose length chosen so the piece is `L` long.
  - Shapes other than the ogive invert `g` by bisection.
  - The ogive's curve depends on its fineness, so the nose length is found by bisection too.
  - Conical and tangent-ogive transitions are the same clipped or not; the test checks this to
    1e-12.

## Solids of revolution

Per unit density, with inner radius `r_i` (zero when filled):

```text
V = π ∫ (y² − r_i²) dx          x̄ = π ∫ x (y² − r_i²) dx / V
J_a = (π/2) ∫ (y⁴ − r_i⁴) dx    J_t = π ∫ [(y⁴ − r_i⁴)/4 + x² (y² − r_i²)] dx − V x̄²
S = 2π ∫ y √(1 + y′²) dx        A_p = 2 ∫ y dx,  x_p = ∫ x y dx / ∫ y dx
```

Each slice is an annulus: `(π/2)(y⁴ − r_i⁴) dx` about the axis and `(π/4)(y⁴ − r_i⁴) dx` about its
diameter, moved to the reference plane by the parallel-axis theorem. `S` excludes the end faces.

- **Numerics.** Each half of the profile is integrated from its own end in `u = s²`, where `u` is
  the normalized distance from that end. The substitution removes the `u^(−1/2)` singularity of a
  blunt tip's surface integrand, and measuring from the end keeps the tip exact. The integrals use
  `hpr_core::quadrature` (`quadrature.md`) at a relative tolerance of 1e-12.
- **Walls** (ADR-006).
  - A wall of thickness `t` is the part of the solid within `t` of the outer surface, so `t` is
    measured normal to the surface, which is how molded and laid-up shells are made.
  - Its inner radius is the lower envelope of circles of radius `t` on the profile:
    `r_i(x) = max(0, min_{|s−x|≤t} [y(s) − √(t² − (x − s)²)])`. The profile is extended past cut
    ends along their tangents, so the wall is cut square.
  - The minimum comes from a 32-point scan and a golden-section search.
  - The envelope is exact for profiles that don't fall faster than 45° where they fall.
  - The hollow is integrated separately and split where `r_i` reaches zero.
  - [TD] doesn't say how OpenRocket measures thickness, and [CR] measures it radially. The two
    differ by a factor `√(1 + y′²)` in wall volume, 1.4% for a cone three calibres long. M2.2
    will measure OpenRocket's choice.

## Verification

- **Closed forms** (`shapes::tests::nose_volumes_match_closed_forms`, 1e-10 relative):
  - Every shape's filled volume and centroid.
  - Cone and ogive: `V`, `x̄`, `S` and `A_p` from arc integrals.
  - Half spheroid: `x̄ = 5L/8`, and `S` for prolate and oblate cases.
  - Power series: `V = πR²L/(2n+1)`, `x̄ = L(2n+1)/(2n+2)`, and the paraboloid's `S`.
  - Power series and parabolic series: `A_p` and `x_p` too; parabolic series by polynomial
    integrals.
  - Haack: `V = πR²L(½ + 3C/16)` and `x̄ = L(11 + 3C)/(2(8 + 3C))`, integrated in `θ`.
  - Loft's tangent-ogive value, `R = 0.04 m`, `L = 0.25 m` gives `6.7509e-4 m³` (lesson L91).
- **mpmath references** (`solids::tests::filled_solids_match_the_mpmath_references`):
  - 22 noses and transitions of every family, in both directions, clipped and not.
  - All seven quantities, to 1e-11 relative (1e-10 for `J_t`).
  - Reference: `validation/fixtures/design/shape-integrals.json`, from
    `validation/oracles/design/shapes.py` (40-digit tanh-sinh on the defining formulas, Haack in
    `θ`, with no code shared with Rust).
  - These references alone check the wetted areas of the power series (`n ≠ ½`) and parabolic
    series, both Haack areas, and the moments of inertia of the filled shapes.
- **Walls:**
  - A conical wall is the cone minus the same cone moved aft by `t/sin β`: volume, centroid and
    both moments by hand, to 1e-9.
  - A tangent-ogive wall is bounded by the concentric arc of radius `ρ − t`: volume by hand, to
    1e-10.
  - A tube matches the hollow-cylinder formulas.
  - A wall thicker than the body fills it, and a thin wall's volume tends to `S t`.
- **Profiles:** each ends at `0` and `R`, slopes match central differences, and parameters out of
  range are errors (lesson L48). Transitions hit both radii and are monotone both ways, clipped or
  not (lesson L49).
