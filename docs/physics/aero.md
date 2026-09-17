# Aerodynamics

Code: `hpr_aero::body` (bodies of revolution), `hpr_aero::fins` (fin sets) and `hpr_aero::model`
(a rocket's terms over a `Layout`). Decisions: ADR-008. M1.5a covers the subsonic normal force and
centre of pressure; drag and override tables arrive in M1.5b, and transonic and supersonic flow in
M1.8.

Sources:

- **[B66]** J. S. and J. A. Barrowman, *The Theoretical Prediction of the Center of Pressure*,
  NARAM-8, 1966 (`barrowman-1966-naram8-nakka`; the Apogee copy lacks pp. 39–50).
- **[B67]** J. S. Barrowman, *The Practical Calculation of the Aerodynamic Characteristics of
  Slender Finned Vehicles*, MS thesis, 1967 (NASA/TM-2001-209983).
- **[TIR]** J. S. Barrowman, *Calculating the Center of Pressure of a Model Rocket*, Centuri TIR-33,
  1970.
- **[N09]** S. Niskanen, *Development of an Open Source model rocket simulation software*, MSc
  thesis, 2009, chapter 3.
- **[TD]** *OpenRocket technical documentation* 13.05 (the thesis revised; a document, not code).
- **[G]** R. Galejs, *Wind Instability: What Barrowman Left Out*, Sentinel 39.
- **[762]** MIL-HDBK-762(MI), *Design of Aerodynamically Stabilized Free Rockets*, 1990, p. 5-24.

## Conventions

- Coefficients use the reference area `A_ref = π d_ref²/4` from `Layout::reference_diameter_m`.
  Stations are metres aft of the nose tip (`frames.md`, `design.md`).
- `Flow` holds the Mach number, the total angle of attack `α ∈ [0, π]` between `+z_B` and the
  air-relative velocity, and the roll `φ` of the lateral airflow, measured from `x_B` toward `y_B`.
- `C_N` lies in the plane of the flow. The slope is `C_Nα = C_N/α` for `α > 0` and `∂C_N/∂α` at
  `α = 0` ([N09] eq. 3.8). The centre of pressure is the moment sum
  `X = Σ C_Nα,i X_i / Σ C_Nα,i` ([B66] p. 38; [N09] eq. 3.29). A rocket with no net slope has no CP
  (`None`).

## Bodies of revolution

Nose cones, transitions and body tubes, from the outer profile (shoulders are inside the body).

| term | formula | source |
|---|---|---|
| slope | `(C_Nα)_B = (2/A_ref)[A(l) − A(0)] · sin α/α` | [B66] eq. 10, [B67] eq. 3-65, [N09] eq. 3.19 |
| CP, aft of the fore end | `X_B = [l A(l) − V] / [A(l) − A(0)]` | [B66] eq. 28, [B67] eq. 3-89, [N09] eq. 3.28 |
| moment slope | `(2/A_ref)[l A(l) − V] · sin α/α` | [N09] eq. 3.25 |
| body lift | `C_N = K (A_plan/A_ref) sin² α`, `K = 1.1`, at the planform centroid | [G] p. 1, [N09] eq. 3.26–3.27 |

- A nose with a sharp tip has slope 2. A cylinder has 0 and no CP. A boattail has a negative
  slope, and the frustum CP formula [B66] eq. 44 still holds ([B66] p. 21).
- `V` and the planform come from integrating the real profile (`hpr_design::revolve`), so ogive,
  power, parabolic and Haack transitions get their own CP (Loft lesson L9). [B66] fits tangent
  ogives with 0.466 L instead: 0.2–0.9% different at fineness 2.8–5.
- The body's slope has no Mach term in slender-body theory ([B67] p. 3; [N09] p. 22).
- `K` is uncertain: [G] cites Hoerner's 1.1 to 1.5, fitted 1.0 to his own data, and says 1.2
  suits large angles better. Body lift is zero at `α = 0`, so the worked examples don't test it.

## Fins

| term | formula | source |
|---|---|---|
| one fin | `(C_Nα)₁ = 2π (s²/A_ref) / (1 + √(1 + (β s²/(A_fin cos Γ_c))²))`, `β = √(1 − M²)` | [B67] eq. 3-4–3-6, [N09] eq. 3.38–3.40 |
| mean aerodynamic chord | `c̄ = (1/A)∫c² dy`, `y_MAC = (1/A)∫y c dy`, `x_MAC,LE = (1/A)∫x_LE c dy` | [N09] eq. 3.30–3.32 |
| CP, aft of the root leading edge | `X_f = x_MAC,LE + c̄/4` | [B66] eq. 76a, [N09] eq. 3.34 |
| N fins | `(C_Nα)₁ Σ sin² Λ_k · f_N` | [N09] eq. 3.51–3.53, [TD] eq. 3.54 |
| interference | `K_T(B) = 1 + r_t/(s + r_t)` | [B66] eq. 77, [N09] eq. 3.56 |

- `s` is the span from the body surface, `A_fin` one fin's area, `Γ_c` the mid-chord sweep, and
  `r_t` the body tube's radius at the fins.
- **Trapezoids.** `tan Γ_c = (x_t + c_t/2 − c_r/2)/s`, and the closed forms give [B66] eq. 57 and
  76a exactly.
- **Ellipses** on the root chord. `Γ_c = 0`, `c̄ = 8c_r/(3π)`, `y_MAC = 4s/(3π)` and
  `X_f = (½ − 2/(3π)) c_r = 0.28779 c_r`. Loft replaced the ellipse with an equal-area trapezoid,
  whose sweep made the slope 1.3% low (L10).
- **Freeform outlines.** `c(y)` runs from the leading edge to the trailing edge, so a jagged edge's
  gap counts toward the CP but not toward `A_fin` ([N09] pp. 27–28). `Γ_c` is the span average of
  the mid-chord angle ([N09] p. 29), which gives the natural angle for trapezoids and ellipses. The
  integrals are exact: between vertex heights the edges are straight, and a three-point Gauss rule
  per band is exact.
- **Prandtl–Glauert** enters through `β` in the fin slope only. As `M → 1` the slope tends to
  `π s²/A_ref`. The CP stays at the quarter chord for all subsonic Mach ([B67] p. 6). Niskanen's aft
  shift above Mach 0.5 ([N09] eq. 3.35–3.36) moves to M1.8, together with the supersonic fit it
  interpolates to.
- **Fin count.** A fin at angle `Λ_k` to the lateral airflow adds `(C_Nα)₁ sin² Λ_k` in the plane of
  the flow. The sum is `N/2` for three or more evenly spaced fins, at any roll. `f_N` is 1 up to four
  fins, then 0.948, 0.913, 0.854 and 0.810 for five to eight ([TD] eq. 3.54). Those factors make
  six and eight fins 1.37 and 1.62 times four ([762] p. 5-24), and interpolate five and seven (L8).
  More than eight fins are refused: [TD]'s 0.750 has no data behind it. [N09]'s roll-dependent
  15% and 6% reductions for three and four fins were dropped in [TD].
- **Not modelled.**
  - The side force of one- and two-fin sets: [N09] keeps only the in-plane component.
  - Interference between fin sets at the same station.
  - Cant, which matters for roll (M1.8).
  - Tube fins, which are refused until a cited method exists.
  - Launch lugs and rail buttons add drag only (M1.5b).

## Validity and open questions

- These are small-angle models. `α` is accepted over `[0, π]`, but fin slopes stay linear in `α`
  and nothing models stall. The flight engine (M1.6) must decide how to treat large angles near
  rail exit and apogee.
- `M ≥ 1` is an error until M1.8. Transonic effects above about Mach 0.8 are not modelled.

## Verification

- **Barrowman's worked examples** (`hpr_aero::tests::barrowman_worked_examples`). Inputs and
  printed results, with page numbers, are in `validation/fixtures/aero/barrowman-worked-examples.json`.
  Every printed component and total must agree within 1%. Measured:

  | example | C_Nα: hpr / printed | CP: hpr / printed (in) |
  |---|---|---|
  | Testbed II [B66] pp. 41–45 | 21.397 / 21.44 (−0.20%) | 16.703 / 16.7 (+0.02%) |
  | Aerobee 350 [B66] pp. 47–50 | 21.449 / 21.5 (−0.24%) | 390.48 / 391 (−0.13%) |
  | Javelin [TIR] pp. 21–22 | 35.927 / 35.9 (+0.07%) | 11.286 / 11.3 (−0.13%) |
  | Recruiter [TIR] pp. 23–25, TIR-33's six-fin rule | 35.415 / 35.4 (+0.04%) | 15.665 / 15.6 (+0.42%) |
  | Arcon-Hi, two stages [TIR] pp. 27–29 | 96.163 / 96.2 (−0.04%) | 20.803 / 20.8 (+0.02%) |
  | Arcon-Hi, sustainer alone | 32.257 / 32.2 (+0.18%) | 17.845 / 17.9 (−0.31%) |

  - The worst of the 38 printed values (19 slopes, 19 CPs) is the Testbed II nose CP, −0.77%. It is [B66]'s 0.466 L
    fit against the integrated tangent ogive.
  - **Recruiter's six fins.** TIR-33 scales six fins by `N/2` with `K = 1 + 0.5 R/(S + R)` and no
    fin-count factor. With hpr's own rule (0.913 and the full `K`), the fins are +3.42% and the
    total +2.87% from the print. The difference between the two rules accounts for +3.22% and
    +2.83% of that. The test checks the TIR-33 rule within 1%, and checks that hpr's gap is that
    difference to within 1%.
  - The printed mid-chord lengths were measured or rounded. hpr computes them from the geometry
    (Aerobee: 39.7 in printed, 40.50 in geometric). The fixture's notes list each slip in the
    printed arithmetic.
- **Loft lessons.**
  - L8: `fins::tests::six_fin_cna_applies_fin_count_factor`
  - L9: `body::tests::ogive_transition_cp_uses_volume_form`
  - L10: `fins::tests::elliptical_fin_cna_uses_zero_midchord_sweep`
  - L89: `tests::barrowman_hand_values` (cone 2 at 2L/3; a 20→40 mm conical transition over 0.1 m
    is 1.5 at 0.05556 m; an elliptical fin's CP is 0.28779 `c_r`)
- **Limits and invariants** (`body::tests`, `fins::tests`, `model::tests`):
  - Cylinders and thin transitions; body lift at 0 and 90°.
  - Eq. 57 and 76a closed forms against the same trapezoid as a polygon (1e-13).
  - A 2000-gon ellipse, and a jagged fin.
  - Prandtl–Glauert against [B67] eq. 3-6 written with the aspect ratio, and its `M → 1` limit.
  - Roll sums against direct sums; a two-fin set along and across the flow.
  - Mach changes only the fins.
  - A proptest: scaling every length leaves slopes unchanged and scales the CP; the reference
    diameter scales slopes only.
  - Refusals: tube fins, nine fins, Mach 1, angles out of range.
