# NASA SP-8039: thrust, impulse, burn-time and mass definitions

NASA SP-8039, *Solid Rocket Motor Performance Analysis and Prediction*, May 1971
(https://ntrs.nasa.gov/api/citations/19720011135/downloads/19720011135.pdf; cached in gitignored
`refs/papers/`). Read from page images. PDF page = printed + 8 up to p. 93; p. 94 is absent from
the scan, so glossary pp. 95-101 = PDF 102-108. US units: g_c = 32.17 lbm·ft/(lbf·s²).

## 1. Thrust and the pressure term

- Eq. (2), p. 3 (PDF 11), "stationary motor with one-dimensional steady flow":
  `F = ṁ_e u_e/g_c + (P_e − P_a) A_e` (P_e exit static, P_a ambient, A_e exit flow area).
- Eq. (3), p. 4 (PDF 12): `F = P_c A_t C_F = (ṁ_t I_spd)_motor = (ṁ_t I_spd)_propellant +
  (ṁ_t I_spd)_inerts + (ṁ_t I_spd)_igniter`. `C_F = F/(P_c A_t)`: p. 14 (PDF 22), p. 95 (PDF 102).
- Eq. (22), p. 29 (PDF 37): `C_F,ideal = √{ [2γ²/(γ−1)] [2/(γ+1)]^((γ+1)/(γ−1))
  [1 − (P_e/P_c)^((γ−1)/γ)] } + ((P_e − P_a)/P_c)(A_e/A_t)`. "The maximum coefficient value
  corresponding to any fixed pressure ratio is achieved when P_e = P_a; the largest possible value
  occurs under vacuum conditions where P_a = 0."
- Eq. (23): `λ = (1 + cos α)/2`. Eq. (28), p. 31 (PDF 39): `F_vac = λ ṁ_e′ u_e′/g_c + P_e′ A_e`;
  eq. (29): `F_vac = λ(ṁ_e′ u_e′/g_c + P_e′ A_e′)`.
- Eq. (65), p. 69 (PDF 77): `C_F,delivered = η_F { √{…as (22)…}[1 − (P_e′/P_c)^((γ−1)/γ)] +
  ((P_e′ − P_a)/P_c) ε_e′ }`; η_F = C_F/C_F,ideal (p. 100, PDF 107), 0.92-0.98 (p. 32, PDF 40).
- Eq. (34), p. 37 (PDF 45): `I°_sp = u_e/g_c + ((P_e − P_a)/ṁ) A_e`; eq. (35):
  `I°_sp,vac = u_e/g_c + P_e A_e/ṁ`.
- No printed sea-level-to-altitude conversion. Derived here from eq. (2), with ṁ_e, u_e, P_e fixed:
  `F(P_a2) = F(P_a1) + (P_a1 − P_a2) A_e`, so `F_vac = F_SL + P_a,SL A_e`.
- Caveat, pp. 32-34 (PDF 40-42): sea-level tests of altitude nozzles run over-expanded. Flow stays
  attached when P_a is "less than or only slightly greater than" P_e. Eq. (32):
  `P_i/P_a = (2/3)(P_a/P_c)^(1/5)`. The test prints "when P_i/P_a < P_e/P_a"; physically P_e < P_i,
  so the sign looks reversed. Eqs. (66)-(67), p. 70 (PDF 78), give separated-flow C_F.

## 2. Impulse, specific impulse, propellant consumed

- p. 96 (PDF 103): `I = ∫_{t1}^{t2} F dt` (lbf·s). I_sp values must state P_c, P_a, ε and α.
  p. 97 (PDF 104): I_spd (measured) also needs "(5) time interval used for impulse determination
  (6) propellant mass assumption". I_sps: P_c = 1000 psia, P_a = 14.7 psia, ε optimum, α = 0°.
- p. 35 (PDF 43): "I_spd is defined as the ratio of the sum of the forces acting on the rocket
  thrust chamber to the flowrate of the mass being discharged." Eq. (33): `I_sp = c* C_F/g_c`.
  Eqs. (68)-(69), p. 72 (PDF 80): `I_spd = η_μ I°_spd`, `I_spd = η_θ c* η_F C_F/g_c`.
- p. 95 (PDF 102): c = effective exhaust velocity, "in instantaneous form, c = F/ṁ_p";
  c* "in instantaneous form, c* = (P_c A_t)/ṁ_p".
- Eq. (4), p. 5 (PDF 13): `ṁ_p = A_b r_b ρ_p`. Eq. (46), p. 57 (PDF 65):
  `P_c = c* ṁ_t/(A_t g_c) = (c*/(A_t g_c)) [ρ_p Σ_{i=1}^{N} (Δvol)_i/Δt]`.
- Fig. 25, p. 74 (PDF 82): BEM `I_spd = ∫F dt / W_p` (operator faint, reads "="); full scale
  `F = ṁ I_spd`. Same page: "Specific impulse should be based on propellant charge weight.
  Consumed weight should be determined by pre- and post-firing motor weight measurements."
- I_sp is not constant over a burn: erosion gave "a loss of 6 to 9 seconds in delivered specific
  impulse ... by burnout of a 55-second-duration motor" (p. 14, PDF 22); I_spd efficiency varies
  with P_c, ṁ and size (Figs. 16, 18, 19).
- Inerts: (I_sp)_inert 120-200 lbf·s/lbm, or half the propellant I_sp for a same-binder liner
  (p. 43, PDF 51). Eq. (71), p. 76 (PDF 84): `(I_spd)_g = (I_spd)_p (W_p/(W_p + W_L))^(−1/2)`.
- Not stated: mass consumed ∝ impulse, or ∝ ∫P_c dt. Both follow only if c (or c* and A_t) is held
  constant, via c = F/ṁ_p and eq. (46).

## 3. Burn time, action time, web burn time

- p. 77 (PDF 85): "Burning rate is determined by dividing the propellant web by the burning time."
  "The start of web burning should correspond to that point on the trace where an inflection point
  occurs in the ignition transient phase of the motor pressure-time trace. ... Web burnout
  corresponds to the point of maximum rate of change of curvature in the tailoff region of the
  pressure-time trace" (ref. 117). p. 78 (PDF 86): if unclear, "the web burnout point may be
  determined (as suggested in ref. 118) by the widely used tangent-bisector method."
- p. 45 (PDF 53): main cause of BEM/full-scale rate disagreement is "improper definition of the
  burning time of the small motors". Web-burnout points there came from the tangent-bisector method.
- Not in SP-8039: the tangent-bisector construction, and definitions of action time, average
  thrust or average pressure. No %-of-max threshold (such as 10%) appears.
- Used without definition: p. 74 (PDF 82), BEM neutral trace "(within ±10%)" and "burning time ≥
  0.87 action time, and tailoff pressure integral <5% action time pressure integral". Table III,
  p. 53 (PDF 61), coefficients of variation: action time 2.0%, total impulse 0.3%, avg thrust 2.1%.
- Defers to ref. 118 (CPIA Publ. 80, *Solid Propellant Nomenclature Guide*, 1965, AD 465058),
  ref. 117 (Jessup & VanWie, CPIA Publ. 24, 1963, AD 345567), ref. 27 (ICRPG, CPIA Publ. 174, 1968).

## 4. Mass, CG and inertia during burn (qualitative, no equations)

- pp. 34-35 (PDF 42-43): some programs output propellant mass, CG and principal MOI histories
  (ref. 3); inert discharge is predicted separately.
- §3.2.1, p. 71 (PDF 79): account for "the discharge of both propellant and inert products":
  propellant from eq. (4), inerts from heat-transfer analyses, "combined with hardware mass".
- §3.2.2, p. 71 (PDF 79): CG and MOI "shall be based on the motor station distribution of the
  discharged propellant and inert material". "Use standard mathematical techniques in calculating
  centers of gravity and in calculating and shifting principal moments of inertia."
