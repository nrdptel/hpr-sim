# The Arcas Robin's supersonic body gap, source by source (M1.8e5)

What this covers: after [M1.8e4](../decisions-and-roadmap.md#m1-8e4), hpr's body alone faster
than sound read from 8.3% high to 27.0% low against NASA's Arcas Robin wind tunnel (TN D-4014,
fins off, Mach 1.5 to 4.63). This note sizes each candidate cause against that gap, row by row,
and ranks them, before any model is built. What it is for:
[M1.8e6](../decisions-and-roadmap.md#m1-8e6) and [M1.8e7](../decisions-and-roadmap.md#m1-8e7)
start from its ranking. How far to trust it: every fit and every hpr number below is in
[`validation/fixtures/aero/arcas-robin-gap.json`](../../validation/fixtures/aero/arcas-robin-gap.json),
which `cargo xtask aero` writes from committed files and a test keeps current. The outside sources
are pinned in `validation/refs.lock.toml`. The tunnel's points were read off plots to ±0.01 in
`C_N`, so its slope at `α → 0` is known only to ±0.2 to ±0.4 per radian. BLUNT-TIP-TRUST

Slopes are per radian, on the body's cross-section (2.25 in across). `α` is the angle of attack.

## In short

1. **Most of the gap is the comparison, not missing lift.** The measured slope is a straight line
   fitted through `C_N` plotted from about −5° to +4° (M1.8a's fit). hpr's is its slope at
   `α → 0`. Crossflow lift grows as `α |α|`, so it steepens the fitted line. Fitted the same way,
   at the same angles, with the body lift a flight adds, hpr's body reads **15% to 73% high** at
   every Mach number, not low.
2. **Rank 1: the size of crossflow lift.** hpr's body lift puts 1.40 to 2.09 per radian into the
   fitted line. The tunnel's own curvature puts in 0.09 to 1.74. From Mach 2.3 that curvature
   matches hpr's form with a factor `K` of 0.66 to 1.05 (±0.2), where hpr uses 1.1 and
   Jorgensen's method gives about 0.9.
3. **Rank 2: the slope at `α → 0`.** With crossflow fitted out, hpr's slope is between 0.37 below
   and 0.87 above the tunnel's. The readings can't settle it more finely.
4. **Rank 3: the lip** behind the boattail, +0.18 by slender-body theory, measured but left out.
5. **Rank 4: TN 3527's Fig. 2 below Mach 3,** at most 0.06 either way, none from Mach 3.
6. **Rank 5: the blunt tip.** BLUNT-TIP-SHORT
7. **Rank 6: [issue #81](https://github.com/nrdptel/hpr-sim/issues/81),** zero: no element of
   this body is reduced at any of the tunnel's Mach numbers.

## The gap

The measured slope is the least-squares straight line, with an intercept, through the tunnel's
fins-off points. Its standard error treats each point's reading error as independent, with the
standard deviation the reading states: ±0.01 in `C_N` on the short model (TN D-4014 Fig. 5),
±0.013 on the long one (Fig. 6). hpr's body is the one M1.8e4 compared: the nose as the secant
ogive that best fits the report's coordinates, the cylinder, and the 15° boattail, through a
flight's own path (`AeroModel::components`), without the lip. The gap is the measured slope less
hpr's at `α → 0`. The last two columns fit hpr's bodies' `C_N` at the tunnel's own angles, the
same way as the measurement.

| model | Mach | measured, fitted | hpr at `α → 0` | gap | hpr, fitted the same way | difference |
|---|---|---|---|---|---|---|
| short | 1.5 | 2.19 ± 0.09 | 2.37 | −0.18 | 3.80 | +73.2% |
| short | 1.8 | 2.61 ± 0.09 | 2.58 | +0.03 | 3.98 | +52.2% |
| short | 2.3 | 3.08 ± 0.08 | 2.83 | +0.25 | 4.29 | +39.4% |
| short | 2.96 | 3.28 ± 0.08 | 3.06 | +0.23 | 4.54 | +38.1% |
| short | 3.96 | 3.88 ± 0.09 | 3.26 | +0.62 | 4.71 | +21.3% |
| short | 4.63 | 4.15 ± 0.09 | 3.35 | +0.80 | 4.79 | +15.5% |
| long | 1.8 | 3.16 ± 0.11 | 2.58 | +0.58 | 4.50 | +42.4% |
| long | 2.3 | 3.53 ± 0.11 | 2.83 | +0.70 | 4.80 | +36.1% |
| long | 2.96 | 3.87 ± 0.11 | 3.06 | +0.81 | 5.14 | +32.9% |
| long | 3.96 | 4.46 ± 0.11 | 3.27 | +1.18 | 5.23 | +17.3% |
| long | 4.63 | 4.62 ± 0.11 | 3.37 | +1.25 | 5.30 | +14.9% |

The gap runs from −0.18 to +1.25. hpr at the tunnel's angles reads 14.9% to 73.2% above the
measurement. That second comparison is the one a flight sees, because a flight adds body lift at
every angle.

## 1. Crossflow at the tunnel's angles

**What it is.** At an angle of attack, air crosses the body sideways. Behind the body it separates,
as it does behind a cylinder in a cross-wind, and pushes the body with a force that grows as
`sin² α`. hpr calls this [body lift](../glossary.md#body-lift):

```text
C_N,body lift = K (A_plan / A_ref) sin² α,     K = 1.1
```

Here `A_plan` is the body's planform area (its outline seen from the side) and `A_ref` the
reference area. Galejs gives `K` from 1.0 to 1.5 (*Wind Instability*, p. 1). Jorgensen writes the
same term as `η C_dn (A_p / A_r) sin² α` (NASA TR R-474, eq. 2.12, printed p. 10). `C_dn` is a
long cylinder's crossflow drag coefficient, and `η` the ratio of a finite cylinder's to an
infinite one's. hpr's `K (A_plan/A_ref)` is 22.87 per radian² on the short model with its
boattail, and 30.69 on the long.

**How it is sized.** Fit the tunnel's points with a curvature term,
`C_N = a + b α + c α |α|`. `b` is the slope at `α → 0` and `c` the curvature. Then `fitted − b`
is the curvature's part of the straight line, uncertain by as much as `b` is, and
`c / (A_plan/A_ref)` is the `K` that would give hpr's body lift that curvature. The straight line
through the middle five points alone (`|α| < 3°`) is a second reading of the slope at `α → 0`,
with less crossflow in it.

| model | Mach | tunnel at `α → 0` (`b`) | middle points | hpr at `α → 0` | tunnel's crossflow in its line | hpr's (`K` 1.0 to 1.5) | `K` the tunnel implies |
|---|---|---|---|---|---|---|---|
| short | 1.5 | 1.78 ± 0.32 | 2.04 ± 0.18 | 2.37 | 0.41 | 1.42 (1.29–1.94) | 0.32 ± 0.24 |
| short | 1.8 | 2.52 ± 0.33 | 2.55 ± 0.18 | 2.58 | 0.09 | 1.40 (1.27–1.91) | 0.07 ± 0.25 |
| short | 2.3 | 2.20 ± 0.32 | 2.61 ± 0.18 | 2.83 | 0.88 | 1.46 (1.33–2.00) | 0.66 ± 0.23 |
| short | 2.96 | 2.18 ± 0.30 | 2.69 ± 0.18 | 3.06 | 1.10 | 1.48 (1.35–2.02) | 0.81 ± 0.21 |
| short | 3.96 | 2.69 ± 0.32 | 3.30 ± 0.18 | 3.26 | 1.19 | 1.45 (1.32–1.98) | 0.90 ± 0.23 |
| short | 4.63 | 2.76 ± 0.32 | 3.38 ± 0.18 | 3.35 | 1.39 | 1.45 (1.32–1.97) | 1.05 ± 0.23 |
| long | 1.8 | 1.92 ± 0.42 | 2.58 ± 0.23 | 2.58 | 1.24 | 1.92 (1.74–2.62) | 0.71 ± 0.23 |
| long | 2.3 | 2.24 ± 0.41 | 2.88 ± 0.23 | 2.83 | 1.28 | 1.97 (1.79–2.69) | 0.71 ± 0.22 |
| long | 2.96 | 2.52 ± 0.36 | 3.03 ± 0.23 | 3.06 | 1.35 | 2.09 (1.90–2.84) | 0.71 ± 0.18 |
| long | 3.96 | 3.13 ± 0.41 | 3.56 ± 0.23 | 3.27 | 1.33 | 1.95 (1.78–2.66) | 0.75 ± 0.22 |
| long | 4.63 | 2.88 ± 0.41 | 3.74 ± 0.23 | 3.37 | 1.74 | 1.94 (1.76–2.64) | 0.98 ± 0.23 |

**Jorgensen's value for these bodies.** His Fig. 4 (printed p. 77) gives `η` from a cylinder's
length over its diameter: about 0.74 for the short model's 18.2 and 0.77 for the long model's 23.8.
In the tunnel the crossflow Reynolds number (on the diameter) is 5.6 × 10⁵ sin α, below 5 × 10⁴ up
to 5°. That is subcritical, where "C_dn = 1.2" (printed p. 15; his Fig. 2, printed p. 76, gives
1.20 to 1.21 from 2 × 10⁴ to 5 × 10⁴). The crossflow Mach number `M sin α` stays below 0.4, where
his Fig. 1 keeps `C_dn` within 1.20 to 1.27. So `η C_dn` is about 0.89 and 0.92. His `η` comes
from cylinders measured "only at very low subsonic Mach numbers" (printed p. 17). His Fig. 6
shows it rising by about 0.04 up to a crossflow Mach number of 0.4, but only for bodies of
fineness 10 and 12.

**What the table says.**

- Crossflow at the tunnel's angles is larger than the gap at every row, by either reading: 0.09
  to 1.74 from the tunnel's own curvature, 1.40 to 2.09 in hpr. It ranks first.
- hpr's body lift is too large here. From Mach 2.3, the curvature implies `K` from 0.66 to 1.05,
  each ±0.18 to ±0.23. Jorgensen's 0.89 and 0.92 lie within 1.3 standard errors of all eight of
  those rows. hpr's 1.1 lies above all eight, by up to 2.2 standard errors.
- The short model at Mach 1.5 and 1.8 shows almost no curvature (`K` 0.32 and 0.07), while the
  long model at Mach 1.8 shows 0.71. Nothing here explains that difference.

## 2. The slope at `α → 0`

With crossflow fitted out, what remains is the slope at `α → 0`. The two readings of it disagree
by more than their standard errors suggest. The `α |α|` form bends from the smallest angle; if the
real curve bends later, `b` reads low. The middle points' line still carries some crossflow, so it
reads high. hpr's slope lies 0.06 to 0.87 above `b`, and between 0.37 below and 0.36 above the
middle points' line. TN 3527 states its method within ±0.2 per radian of its own measurements
(Summary, p. 1), and at Mach 1.5 the Arcas Robin sits at 0.36 in Mach number over nose fineness,
below the method's stated 0.4. So this ranks second: it could be as large as 0.9, or nothing.

## 3. The lip

Behind the 15° boattail the model flares out again in a short lip, from 1.308 to 1.470 in across
over 0.053 in (TN D-4014 Fig. 1(a)), a 57° flare. The tunnel measures it. The method can't take it
(its tangent cone would be past Fig. 2's 24°), so M1.8e4's comparison leaves it out. Slender-body
theory gives a flare `2 ΔA / A_ref`: +0.178 at every Mach number. The lip sits in the boattail's
wake, so its real share is probably smaller. It would raise hpr's slope, not close the gap at
`α → 0`.

## 4. TN 3527's Fig. 2 below Mach 3

The method needs each tangent cone's normal-force slope. TN 3527 plots it in Fig. 2 (p. 40) from
Mach 3 to 10, and below Mach 3 hpr holds the Mach 3 curve. Sims (NASA SP-3007, 1964, Table 2,
printed p. 20) tabulates the same theory from Mach 1.5. His expressions are "identical to those
found by Kopal" (p. 7), whose tables Fig. 2 plots. At Mach 3 his table agrees with hpr's reading
of Fig. 2 within 0.003 from 5° to 12.5°, and within 0.01 at 2.5°.

In the method each element's lift carries its tangent cone's slope with a positive weight
(TN 3527 eq. 19). So if every held slope were replaced by Sims's, the nose and cylinder's share
would change by a ratio between the least and greatest ratio of his slopes to his Mach 3 slopes.
Taken over his angles to 12.5° (the nose's cones run up to 10.76°) and his Mach numbers at or
around each row, that range is −1.2% to +2.2% of the share at Mach 1.5, and −0.7% to +0.6% at
Mach 2.96. In slope that is −0.033 to +0.057, and nothing from Mach 3.

## 5. The blunt tip

BLUNT-TIP-SECTION

## 6. Issue #81

Where the pressure gradient behind a corner points away from its tangent cone's pressure
(`η < 0`), hpr reduces the element to the generalized method and carries no gradient on; issue
#81 records where that departs from TN 3527. `ShockExpansionBody::reduced_elements` counts such
elements. On the Arcas Robin's body, both lengths, with its boattail, it is zero at all six Mach
numbers. So #81 changes nothing here. It still matters for blunter noses at higher Mach numbers,
such as TN 3527's fineness-3 ogive at Mach 5.05.

## Not sized

- **The boundary layer.** Its displacement thickens along the body and acts like a slight flare.
  The reports give no thickness to size it with.
- **The nose's shape.** The secant ogive misses the report's coordinates by 0.003 in rms
  (`shock-expansion.json`), too little to matter.

## The ranking

| rank | source | size across the 11 rows, per radian | evidence |
|---|---|---|---|
| 1 | crossflow at the tunnel's angles | 0.09 to 1.74 (tunnel); 1.40 to 2.09 (hpr) | fits of committed readings; Jorgensen |
| 2 | the slope at `α → 0` | hpr 0.37 below to 0.87 above | within the readings' ±0.2 to ±0.4 |
| 3 | the lip | +0.18 | slender-body theory, likely an upper bound |
| 4 | Fig. 2 below Mach 3 | −0.033 to +0.057 | Sims's tables, a bound |
| 5 | the blunt tip | BLUNT-TIP-SIZE | BLUNT-TIP-EVIDENCE |
| 6 | issue #81 | 0 | counted by the method |

## What this means for M1.8e6 and M1.8e7

- **Size crossflow, don't add it.** hpr already flies body lift at every angle, and at the
  tunnel's angles it reads high. Jorgensen's `η C_dn`, with `η` from the body's fineness (Fig. 4)
  and `C_dn` from the crossflow Mach and Reynolds numbers (Figs. 1 and 2), fits the tunnel better
  than `K` = 1.1 from Mach 2.3. Body lift also drives a slow rocket's drift in wind
  ([ADR-026](../DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18), body lift in wind), so a change below Mach 1 moves the validation report's whole
  flights. M1.8e6 should decide whether the change applies faster than sound only or at every
  speed, and regenerate the report.
- **Compare as the tunnel measures.** Judge the body fitted at the plotted angles
  (`hpr.fitted_c_n_alpha` in the fixture), not at `α → 0`.
- **Blunt tips are a coverage question.** BLUNT-TIP-COVERAGE The committed design's power-series
  nose has a vertical tip, which the method refuses. So the design as committed keeps slender-body
  theory at every Mach number: 0.854 at `α → 0`, where the tunnel reads 2.19 to 4.62 fitted.
- **M1.8e7's 15% bullet** is judged the way M1.8a fits it, at the plotted angles. There the body
  with the fitted nose now reads 15% to 73% high. The committed short model's body, on
  slender-body theory, reads from 4% high to 44% low
  (the [guide's table](../physics/aero.md#normal-force-through-mach-1)).
