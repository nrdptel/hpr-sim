# Aerodynamics

Code: `hpr_aero::body` (bodies of revolution), `hpr_aero::fins` (fin sets) and `hpr_aero::model`
(a rocket's terms over a `Layout`). Decisions: ADR-008 and ADR-009. M1.5a covers the subsonic normal
force and centre of pressure, M1.5b the subsonic drag and override tables; transonic and supersonic
flow arrive in M1.8.

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
- `C_N` lies in the plane of the flow. The side coefficient `C_Y` lies across it, along `z_B` × the
  lateral-flow direction; only one- and two-fin sets produce it. The slope is `C_Nα = C_N/α` for `α > 0` and `∂C_N/∂α` at
  `α = 0` ([N09] eq. 3.8). The centre of pressure is the moment sum
  `X = Σ C_Nα,i X_i / Σ C_Nα,i` ([B66] p. 38; [N09] eq. 3.29). A rocket with no net slope has no CP
  (`None`), but `NormalForce::moment_m = Σ C_N,i X_i` (the moment about the nose tip per unit
  dynamic pressure and reference area) is always defined.

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
- **Radius steps (an extrapolation).** A step where one body component meets the next adds
  `(2/A_ref)ΔA` at the joint, the limit of a transition whose length goes to zero, so the body's
  total slope is [B66] eq. 10 over the whole body. [B67] p. 18 assumes no discontinuities, so this
  goes beyond the source; leaving the step out would silently drop its slope (a 27 mm nose base on
  a 29 mm tube loses 13%). It is reported with the aft component (`BodyAero::step_area_m2`). A
  blunt front face gets no term, as eq. 10 gives. The design checks warn about steps
  (`radius_step`); the real flow separates there, which M1.5b's drag must count.
- `V` and the planform come from integrating the real profile (`hpr_design::revolve`), so ogive,
  power, parabolic and Haack transitions get their own CP (Loft lesson L9). [B66] fits tangent
  ogives with 0.466 L instead: 0.2–0.9% different at fineness 2.8–5.
- The body's slope has no Mach term: [B67] p. 18 leaves body compressibility out as a
  conservative choice, and [N09] p. 22 takes the body's normal force as the same at all speeds.
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
  per band is exact. Bands thinner than 1e-12 of the span (vertex heights a few rounding steps
  apart, as when a tip is converted from inches) are skipped.
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
- **Side force of one- and two-fin sets.** Each fin sees `α sin Λ_k` ([N09] eq. 3.50) and pushes
  along its own normal. Eq. 3.51 keeps the in-plane share `sin² Λ_k`; the share across the plane is
  `sin Λ_k cos Λ_k`, which cancels for three or more fins but not for one or two. [N09] pp. 31–32
  drops it, arguing that it cancels for two or more fins; for two fins the pushes add. hpr reports
  it as `C_Y` at the fins' CP (derived here from eq. 3.50, not taken from a source).
- **Interference** `K_T(B)` is Barrowman's straight-line fit to NACA TR-1307, justified for
  `r_t/(s + r_t) < 0.4` ([B66] p. 36).
- **Not modelled.**
  - The body lift the fins induce, `K_B(T)` ([B66] p. 36 neglects it; [B67] eq. 3-98 has it).
  - The roll moment of a single fin: its force acts at `r_t + y_MAC` along the fin's normal. Two or
    more even fins cancel it; one fin doesn't (roll arrives in M1.8).
  - Interference between fin sets at the same station.
  - Cant, which matters for roll (M1.8).
  - Tube fins, which are refused until a cited method exists (issue #15). Any part kind the model
    doesn't know is refused too.
  - Launch lugs and rail buttons add drag only.

## Drag

Code: `hpr_aero::drag` (the terms), `AeroModel::drag` and `AeroModel::drag_components` (their sum
over a rocket), `hpr_aero::table` (override tables), `hpr_design::Finish` (roughness). Decisions:
ADR-009. Extra sources:

- **[B67] ch. 4** (pp. 43–62): the friction, roughness and leading-edge formulas Niskanen adopts,
  and Table 4-1 of roughness heights (p. 46, after Hoerner p. 5-3).
- **[N09] §3.4** (pp. 41–53) and appendix B (pp. 106–110). [TD] reprints the same drag equations
  and tables unchanged.

`C_D0 = C_D,friction + Σ_T (A_T/A_ref)(C_D•)_T` ([N09] eq. 3.75, 3.97), each pressure, base and
parasitic term on its own area. The axial coefficient is `C_A = C_D0 f(α)`.

| term | formula | area | source |
|---|---|---|---|
| Reynolds number | `R = V L/ν`, `L` nose tip to aft end of the last body component | | [N09] eq. 3.12, p. 42 |
| skin friction | `1.48e-2` for `R < 1e4`; `1/(1.50 ln R − 5.6)²` to `R_crit`; `0.032 (R_s/L)^0.2` from it | | [N09] eq. 3.78–3.81, [B67] eq. 4-4–4-8 |
| roughness limit | `R_crit = 51 (R_s/L)^−1.039` | | [N09] eq. 3.79, [B67] eq. 4-7 |
| compressibility | `C_f (1 − 0.1 M²)` for `M < 1`; `C_f/(1 + 0.15 M²)^0.58` turbulent and `C_f/(1 + 0.18 M²)` rough (not below turbulent) above | | [N09] eq. 3.82–3.84 |
| friction drag | `C_fc [(1 + 1/(2 f_B)) A_body + (1 + 2t/c̄) A_fins]/A_ref` | body: `π A_plan`; fins: both sides | [N09] eq. 3.85 |
| nose, shoulder | `0.8 sin² φ`, `φ` the joint angle at the aft end | base area; increase in area | [N09] eq. 3.86 |
| boattail | `(C_D•)_base` × 1 (`γ ≤ 1`), `(3 − γ)/2`, 0 (`γ ≥ 3`); `γ = l/(d₁ − d₂)` | decrease in area | [N09] eq. 3.88 |
| base | `0.12 + 0.13 M²` below Mach 1, `0.25/M` above | aft base less thrusting motors | [N09] eq. 3.94, p. 50 |
| fin leading edge | square: `0.85 q_stag/q`; rounded, airfoil: `(1 − M²)^−0.417 − 1` (to 0.9), `1 − 1.785(M − 0.9)` (to 1), `1.214 − 0.502/M² + 0.1095/M⁴`; times `cos² Γ_L` | `N t s` | [N09] eq. 3.89–3.91, B.2 |
| fin trailing edge | square: base; rounded: half base; airfoil: 0 | `N t s` | [N09] eq. 3.92–3.93 |
| stagnation pressure | `q_stag/q = 1 + M²/4 + M⁴/40` below Mach 1, `1.84 − 0.76/M² + 0.166/M⁴ + 0.035/M⁶` above | | [N09] eq. B.1 |
| launch lug | `max{1.3 − 0.3 l/d, 1} · 0.85 q_stag/q` | `π r_ext² − π r_int² max{1 − l/d, 0}` | [N09] eq. 3.95–3.96 |
| rail button | `0.85 q_stag/q` (a rail pin) | side profile | [N09] p. 52 |
| angle of attack | `f = 1 + 0.3(3t² − 2t³)`, `t = α/17°`; `1.3(1 − 3u² + 2u³)`, `u = (α − 17°)/73°` | | [N09] §3.4.7 (conditions only) |

- **Roughness.** `hpr_design::Finish` names the fifteen rows of [B67] Table 4-1 (0 to 1000 µm;
  [N09] Table 3.2 reprints ten) or takes a custom height. The default is "paint in aircraft mass
  production", 20 µm. Each component has its own finish; the Reynolds number and `R_s/L` use the
  whole rocket's length, as [N09] does (L12). Loft's 60 µm and 2 µm and its `1 + 60/f³ + 0.0025f`
  form factor had no source.
- **Fully turbulent.** [N09] p. 43 found laminar runs changed apogee by under 5% and dropped them.
- **Friction jumps where [N09] does.** Eq. 3.79 is not where eq. 3.78 and 3.80 cross, so eq. 3.81
  jumps at `R_crit`: +9% for 60 µm on a 1 m rocket (0.00419 to 0.00458). The subsonic and
  supersonic corrections also differ at Mach 1 (0.900 against 0.922 turbulent). hpr keeps the
  published forms, and the tests pin both jumps (L90).
- **Friction on the axial projection (a departure).** Wall shear acts along the surface, so its
  axial share is `τ cos θ dA`, and the body's friction area is `2π ∫ r dx = π A_plan` rather than
  the slant surface in [N09] eq. 3.85. On slender noses the difference is small: a tangent ogive
  loses 1.1% of its own friction area at fineness 3 and 2.4% at fineness 2. On a short, steep
  shoulder it removes friction on what is nearly a flat face, so a shoulder's drag tends to a bare
  step's as its length goes to zero (L15).
- **Steps in radius.** Where one body component meets the next with a different radius, a step up
  is a zero-length shoulder, `0.8 ΔA`, and a step down a zero-length boattail, the base drag of the
  uncovered area. A body with no nose cone gets `0.8 A` on its front face. Each is the limit of the
  transition it replaces (L15), and it is reported with the aft component.
- **Boattails.** [N09] eq. 3.88 writes `A_base/A_boattail` without defining the areas. hpr reads
  both as the boattail's decrease in area: only that reading makes a zero-length boattail drag like
  the base it uncovers, as [N09] p. 49 says, instead of counting the base twice.
- **Base drag under power** subtracts the thrusting motors' cross-section from the aft base, down
  to zero ([N09] p. 50: "if the base is the same size as the motor itself, no base drag"; L13).
  `DragConditions::thrusting_motor_area_m2` is the case area of the burning motors; the flight
  engine supplies it. The base belongs to the last body component.
- **Fins.** Each fin set is its own term with its own thickness, chord and cross-section, so their
  order doesn't matter (L11). `c̄` is the mean aerodynamic chord and `Γ_L` the leading-edge sweep:
  `atan(x_t/s)` for a trapezoid, the span average for freeform outlines ([N09] p. 50), and for an
  ellipse `π/2 − acos(k)/√(1 − k²)`, `k = c_r/(2s)` (the closed-form average, with its `acosh` form
  for `k > 1`). Fin–body interference drag and tip vortices are neglected ([N09] p. 41).
- **Launch lugs.** `d` in eq. 3.95–3.96 is taken as the outer diameter: [N09] p. 52 treats a solid
  rail pin as a lug "with a length equal to its diameter", which only reads that way (L14). A row
  of `count` lugs is `count` lugs. **Rail buttons** follow [N09]'s rail-pin rule on their side
  profile (base and flange at the outer diameter, waist at the inner).
- **Angle of attack (derived coefficients).** [N09] §3.4.7 describes, without an equation, a
  two-part polynomial from 1 at 0° to 1.3 at 17° and 0 at 90°, with zero slope at each. hpr uses
  the unique cubic on each part that meets those four conditions. Past 90° it mirrors,
  `f(180° − α)`, an assumption. M2.2 compares against OpenRocket, whose polynomial may differ.
- **Refusals, not clamps (L16).** Geometry the terms can't use (a lug wall thicker than its
  radius, a button's base and flange taller than the button, a negative roughness) is an error
  naming the component; a non-finite result is an error; large coefficients are returned as they
  are.
- **Override tables** (`DragTable`) replace `C_D0` with `C_D0(M)` curves, power-off and power-on,
  read from CSV text: two headerless columns (RocketPy's curves, `\r\n` and `01.05` accepted) or a
  header naming the column, with rows at non-zero `Alpha` skipped (RASAero II's export: `Mach,
  Alpha, CD, CD Power-Off, CD Power-On, …`). Tables interpolate linearly and hold their end values;
  `Drag::table` reports any extrapolation. A positive thrusting-motor area selects the power-on
  curve. The angle-of-attack factor still applies, and an override accepts any Mach number.

### Drag limits

- The buildup refuses `M ≥ 1` until M1.8, like the normal force. The term functions are defined to
  any Mach number and stay finite to Mach 5 (tested), for M1.8 to build on.
- **Nose pressure drag is held at its low-subsonic value.** [N09] eq. 3.87 raises it toward the
  transonic method's value at Mach 1, which needs appendix B's shape data (Stoney, NASA TR-R-100)
  and arrives with M1.8. Until then pressure drag of pointed noses and shoulders is low above about
  Mach 0.6 (a 10° cone's grows from 0.024 toward 0.17 by Mach 1).
- Nothing models laminar flow, fin-tip vortices, interference drag, fin tabs, fillets, canted fins
  or the flow a boattail guides into the base ([N09] p. 51).

## Validity and open questions

- These are small-angle models. `α` is accepted over `[0, π]`, but fin slopes stay linear in `α`
  and nothing models stall. The flight engine (M1.6) must decide how to treat large angles near
  rail exit and apogee.
- In one measured case, fins at `α = π/2` give `C_N` 17.4 against a flat-plate estimate near 5, and
  at `α = π` the fins still give 34.7 while every body term vanishes. That case is a 54 mm
  four-fin rocket at Mach 0.3.
- `M ≥ 1` is an error until M1.8, but the models are only documented to Mach 0.8.
  - [N09]'s subsonic range is 0–0.8, and [B67] p. 18 notes that `C_Nα` rises near Mach 1.
  - [N09] eq. 3.35–3.36 would move the fin CP from 0.25 to about 0.30 of the MAC at Mach 0.8 and
    about 0.33 at 0.9 (aspect ratio 1.6); hpr keeps 0.25.
  - Between 0.8 and 1, results are unvalidated extrapolations; M1.8 replaces them.

## Verification

- **Barrowman's worked examples** (`hpr_aero::tests::barrowman_worked_examples`). Inputs and
  printed results, with page numbers, are in `validation/fixtures/aero/barrowman-worked-examples.json`.
  Every printed component and total must agree within 1%. Measured:

  | example | C_Nα: hpr / printed | CP: hpr / printed (in) |
  |---|---|---|
  | Testbed II [B66] pp. 41–45 | 21.397 / 21.44 (−0.20%) | 16.703 / 16.7 (+0.02%) |
  | Aerobee 350 [B66] pp. 47–50 | 21.449 / 21.5 (−0.24%) | 390.48 / 391 (−0.13%) |
  | Javelin [TIR] pp. 21–22 | 35.927 / 35.9 (+0.07%) | 11.286 / 11.3 (−0.13%) |
  | **Recruiter [TIR] pp. 23–25, hpr's model: outside 1%** | **36.416 / 35.4 (+2.87%)** | 15.665 / 15.6 (+0.42%) |
  | Recruiter with TIR-33's six-fin rule substituted | 35.415 / 35.4 (+0.04%) | 15.627 / 15.6 (+0.17%) |
  | Arcon-Hi, two stages [TIR] pp. 27–29 | 96.163 / 96.2 (−0.04%) | 20.803 / 20.8 (+0.02%) |
  | Arcon-Hi, sustainer alone | 32.257 / 32.2 (+0.18%) | 17.845 / 17.9 (−0.31%) |

  - **With hpr's own model, four examples agree within 1% and the Recruiter does not.** Its six-fin
    slopes are +3.42% (fins) and +2.87% (total). Those are the only 2 of the 38 printed values (19
    slopes, 19 CPs) outside 1%, and the test pins that list.
  - With TIR-33's six-fin rule substituted for the Recruiter, the worst is the Testbed II nose CP,
    −0.77%: [B66]'s 0.466 L fit against the integrated tangent ogive.
  - CPs are compared as stations from the nose tip. Measured from each part's own front, two
    printed values miss 1%: the Testbed II boattail (0.655 in against 0.72 in, −9%, Barrowman's
    diameter ratio slip) and the Javelin fins (0.653 in against 0.66 in, −1.1%, rounding).
  - **Recruiter's six fins.** TIR-33 scales six fins by `N/2` with `K = 1 + 0.5 R/(S + R)` and no
    fin-count factor. With hpr's own rule (0.913 and the full `K`), the fins are +3.42% and the
    total +2.87% from the print. The difference between the two rules accounts for +3.22% and
    +2.83% of that. The test checks the TIR-33 rule within 1% (slopes and CP weighting), reports
    hpr's own values, and checks that the rules differ by more than 2%.
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

### Drag verification

- **RocketPy's drag curves at Mach 0.3** (`tests::rocketpy_drag_curves_at_mach_0_3`, fixture
  `validation/fixtures/aero/rocketpy-drag-curves.json`, written by `cargo xtask aero` from
  `refs/rocketpy`). Sea level (USSA76), where RASAero II computes its exports; tolerance 10%. The
  fixture holds only derived numbers; the test recomputes hpr's `C_D0` from the committed designs.

  | case | curve | hpr `C_D0` | error |
  |---|---|---|---|
  | Calisto, 2018 fins | RASAero II export, power-off | 0.3982 | +4.4% |
  | Calisto, getting-started fins (NACA 0012) | the same | 0.3537 | −7.3% |
  | Juno III | labelled RASAero II, hand-edited | 0.3449 | −8.0% |
  | **Valetudo, outside 10%** | labelled RASAero, hand-edited, power-off | 0.5566 | **−47.0%** |
  | **Valetudo, outside 10%** | the same, power-on | 0.5189 | **−50.4%** |

  - **Inputs.** The exports record none. The designs take RASAero II's default smooth finish and
    each RocketPy example's own fin data: NACA 0012 at 12% of the MAC for Calisto's getting-started
    fins, an airfoil section of the 3 mm placeholder for Juno III (a lift-curve airfoil), square
    3 mm for the rest (ADR-009).
  - **Sensitivity.** Over square, rounded or airfoil fins (3 mm, or 12% for the airfoil), 0 or
    20 µm, with or without rail buttons, the three passing cases range from −14% to +24%. With the
    20 µm default they are +12.8%, +1.4% and +1.5%. The check places hpr near RASAero; it can't show
    10% agreement without the inputs.
  - **Valetudo.** Its table is 1.44 times the OpenRocket export for the same rocket (1.05 against
    0.73 at Mach 0.3) and has no input file. hpr is 42% to 63% under it for every input above. The
    test pins exactly these two cases outside the tolerance.
  - Not compared: Calisto's power-on curve, which equals its power-off curve (RASAero II without a
    nozzle exit diameter); Juno III's power-on drag, which RocketPy takes from the same file; and
    the other examples, whose drag is a constant, CFD or of unknown origin.
- **Loft lessons.**
  - L11: `drag::tests::drag_invariant_to_fin_set_order`
  - L12: `drag::tests::form_factor_and_roughness_match_cited_values`
  - L13: `drag::tests::power_on_base_drag_subtracts_thrusting_motor_area`
  - L14: `drag::tests::launch_lug_drag_matches_cited_hollow_tube_formula`
  - L15: `drag::tests::shoulder_drag_continuous_as_transition_length_tends_to_zero`
  - L16: `drag::tests::malformed_geometry_is_an_error_not_a_clamped_cd`
  - L90: `drag::tests::skin_friction_follows_eq_3_81_and_drag_invariants_hold`
- **Limits of every term** (`drag::tests`): friction below `1e4`, at `R_crit` and to Mach 5;
  stagnation pressure against the isentropic series and its limits either side of Mach 1; base drag
  at rest, at Mach 1 and far above; the joint term from smooth to a step; the boattail factor's
  three pieces and their joins; fin pressure drag by cross-section with the leading edge's joins at
  Mach 0.9 and 1 and the sweep; the angle-of-attack factor's stated values and zero slopes,
  monotonicity and mirror; leading-edge sweeps of trapezoids, kinked outlines and ellipses (against
  a quadrature to 1e-8); a whole rocket's buildup written out by hand to 1e-12, its component sum,
  and an override table.
- **Tables** (`table::tests`): RocketPy's quirks (`\r\n`, `01.05`), RASAero II's header with rows
  at 2° and 4° skipped, malformed text by line.
