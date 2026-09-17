# Adaptive quadrature

Code: `hpr_core::quadrature`.

Source: **[QP]** R. Piessens, E. de Doncker-Kapenga, C. Überhuber and D. Kahaner, *QUADPACK: A
Subroutine Package for Automatic Integration*, Springer, 1983. QUADPACK is public domain. Its
routine `qk15` has the rule's nodes and weights to 33 digits.

## Method

- **Rule.** On `[a, b]`, the 15-point Gauss–Kronrod rule `K` embeds the 7-point Gauss rule `G`.
  `K` is exact for polynomials of degree ≤ 22 and `G` for degree ≤ 13.
- **Error estimate** of a subinterval: `|K − G|` for each component. This overestimates the error
  of `K` when the integrand is smooth.
- **Global adaptivity** ([QP] `QAG`, §2.2 and §3.3):
  - Keep every subinterval with its estimates.
  - While the summed error of some component exceeds `max(absolute, relative · |Σ K|)`, bisect the
    subinterval that contributes most to the worst component.
  - Stop with `QuadratureDidNotConverge` at the subinterval budget, or when a bisection point can
    no longer be represented.
  - QUADPACK's `QAGS` also extrapolates with the ε-algorithm; hpr does not, and relies on
    bisection plus variable substitutions at known singularities (`shapes.md`).
- **Vector integrands** share one set of subintervals, so the moments of a solid come from the same
  integrand evaluations. Integrands should be scaled to order one so that one absolute tolerance
  suits every component.
- **Floors.** The relative tolerance never goes below `50 ε`. A NaN or infinite sample is an error
  (`QuadratureNotFinite`), never a silent zero.

## Verification (`quadrature::tests`)

- **The rule is the published one.**
  - The Gauss nodes are roots of `P₇` to 1e-15.
  - Both weight sets sum to 2.
  - One panel integrates `x^k` exactly for `k ≤ 22` (Kronrod) and `k ≤ 13` (Gauss). At the next
    even degree it is not exact.
  - A digit typo in any node or weight fails these checks.
- **Convergence:**
  - `eˣ`, `sin x`, `√x`, `x^(−1/2)`, `x^0.3` and `|x − 0.3|` all reach 1e-11 relative.
  - Four components converge together.
  - Reversed limits change the sign.
- **Failures:**
  - `1/x` on `(0, 1]` runs out of subintervals.
  - A NaN integrand is reported.
