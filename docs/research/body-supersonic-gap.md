# The Arcas Robin's supersonic body gap, source by source (M1.8e5)

**What this covers.** After [M1.8e4](../decisions-and-roadmap.md#m1-8e4), hpr's body alone
faster than sound read from 8.3% high to 27.0% low against NASA's Arcas Robin wind tunnel
(TN D-4014, fins off, Mach 1.5 to 4.63). hpr computes that body's normal force by the
[second-order shock-expansion method](../physics/aero.md#bodies-faster-than-sound) (NACA TN 3527),
plus [body lift](../glossary.md#body-lift) at an angle. This note sizes each candidate cause of
the gap, row by row, and ranks them, before any model is built. The smaller causes are sized in
[its companion](body-supersonic-gap-sources.md). Slopes are per radian, on the body's
cross-section (2.25 in across); `α` is the angle of attack.

**What it is for.** [M1.8e6](../decisions-and-roadmap.md#m1-8e6) and
[M1.8e7](../decisions-and-roadmap.md#m1-8e7) start from its ranking;
[ADR-036](../DECISIONS.md#adr-036-the-arcas-robins-supersonic-body-gap-judged-as-the-tunnel-measures-m18e6-takes-crossflows-size-and-the-boattail-2026-09-19)
records what follows from it.

**How far to trust it.** Every fit and every hpr number below is in
[`validation/fixtures/aero/arcas-robin-gap.json`](../../validation/fixtures/aero/arcas-robin-gap.json),
which `cargo xtask aero` writes from committed files and a test keeps current; the outside sources
are pinned in `validation/refs.lock.toml`. The tunnel's points were read off plots to ±0.01 in
`C_N`, and the fits' scatter matches that: χ² 43.6 on 44 degrees of freedom over the 11 rows. The
report states ±0.03 in `C_N` and ±0.1° in `α` (TN D-4014 p. 5); this note takes the rest of that
as offsets and scale errors common to a run, which it doesn't carry. The slope at `α → 0` also
depends on the form fitted (section 2).

## In short

1. **Most of the gap is the comparison, not missing lift.** The measured slope is a straight line
   through `C_N` from about −5° to +4° ([M1.8a](../decisions-and-roadmap.md#m1-8a)'s fit), hpr's
   its slope at `α → 0`. Crossflow lift grows as `α |α|` and steepens the line; the report says
   its slope "increases considerably with increasing angle of attack. This trend is
   characteristic of slender bodies" (p. 6). Fitted the same way, with the body lift a flight
   adds, hpr's body reads **15% to 73% high**, not low.
2. **What's left has more than one cause.** The fit's slope at `α → 0` and its curvature move
   together, so the readings can't split that excess between body lift and hpr's slope at
   `α → 0`. With body lift at Jorgensen's size, hpr still reads 8% to 61% high: resizing body
   lift alone won't close it.
3. **The ranking,** largest first:

| rank | source | size across the 11 rows, per radian | evidence |
|---|---|---|---|
| 1 | crossflow's size at the tunnel's angles | 0.09 to 1.74 (tunnel); 1.40 to 2.09 (hpr) | fits of committed readings; Jorgensen |
| 2 | the boattail's share | −0.18 to −0.03 (hpr); −1.32 (slender-body theory) | two theories, nothing measured alone |
| 3 | the lip | +0.178 | slender-body theory, unmeasured |
| 4 | the blunt tip | 0 to −0.07 (about −0.01 at most below Mach 3) | a tip four times larger, scaled; swaps with 5 if `n` = 2 |
| 5 | TN 3527's Fig. 2 below Mach 3 | −0.031 to +0.056 | Sims's tables, a bound |
| 6 | [issue #81](https://github.com/nrdptel/hpr-sim/issues/81) | 0 | counted by the method |

## The gap

The measured slope is the least-squares line, with an intercept, through the fins-off points; its
standard error takes each point's reading error as an independent standard deviation: ±0.01 in
`C_N` on the short model (TN D-4014 Fig. 5), ±0.013 on the long (Fig. 6). hpr's body is M1.8e4's:
the nose as the secant ogive (a circular arc meeting the body at an angle) that best fits the
report's coordinates, the cylinder and the 15° boattail, without the lip, through a flight's own
path (`AeroModel::components`). The last two columns fit hpr's bodies' `C_N` at the tunnel's own
angles, the same way; that is the comparison a flight sees.

| model | Mach | measured, fitted | hpr at `α → 0` | gap: measured − hpr | hpr, fitted the same way | hpr fitted vs measured |
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

## 1. Crossflow at the tunnel's angles

**What it is.** At an angle of attack, air crosses the body sideways, separates behind it as
behind a cylinder in a cross-wind, and pushes it with a force that grows as `sin² α`. hpr's
[body lift](../glossary.md#body-lift) is `K (A_plan/A_ref) sin² α`, `A_plan` the side outline's
area and `K` = 1.1 (Galejs: 1.0 to 1.5); `K (A_plan/A_ref)` is 22.87 per radian² on the short
model, 30.69 on the long. Jorgensen writes it `η C_dn (A_p/A_r) sin² α` (NASA TR R-474, eq. 2.12,
p. 10): `C_dn` a long cylinder's crossflow drag coefficient, `η` a finite one's over an infinite
one's (not TN 3527's `η`).

**How it is sized.** Fit the tunnel's points with a curvature term, `C_N = a + b α + c α |α|`.
`b` is the slope at `α → 0` and `c` the curvature. Then `fitted − b` is the curvature's part of
the straight line, uncertain by as much as `b` is, and `c / (A_plan/A_ref)` is the `K` that would
give hpr's body lift that curvature. The next column fits `α³` instead, a curve that bends later.

| model | Mach | tunnel at `α → 0` (`b`) | `b` with `α³` | hpr at `α → 0` | tunnel's crossflow in its line | hpr's (`K` 1.0 to 1.5) | `K` the tunnel implies |
|---|---|---|---|---|---|---|---|
| short | 1.5 | 1.78 ± 0.32 | 1.94 | 2.37 | 0.41 | 1.42 (1.29–1.94) | 0.32 ± 0.24 |
| short | 1.8 | 2.52 ± 0.33 | 2.53 | 2.58 | 0.09 | 1.40 (1.27–1.91) | 0.07 ± 0.25 |
| short | 2.3 | 2.20 ± 0.32 | 2.49 | 2.83 | 0.88 | 1.46 (1.33–2.00) | 0.66 ± 0.23 |
| short | 2.96 | 2.18 ± 0.30 | 2.63 | 3.06 | 1.10 | 1.48 (1.35–2.02) | 0.81 ± 0.21 |
| short | 3.96 | 2.69 ± 0.32 | 3.10 | 3.26 | 1.19 | 1.45 (1.32–1.98) | 0.90 ± 0.23 |
| short | 4.63 | 2.76 ± 0.32 | 3.23 | 3.35 | 1.39 | 1.45 (1.32–1.97) | 1.05 ± 0.23 |
| long | 1.8 | 1.92 ± 0.42 | 2.35 | 2.58 | 1.24 | 1.92 (1.74–2.62) | 0.71 ± 0.23 |
| long | 2.3 | 2.24 ± 0.41 | 2.66 | 2.83 | 1.28 | 1.97 (1.79–2.69) | 0.71 ± 0.22 |
| long | 2.96 | 2.52 ± 0.36 | 3.10 | 3.06 | 1.35 | 2.09 (1.90–2.84) | 0.71 ± 0.18 |
| long | 3.96 | 3.13 ± 0.41 | 3.63 | 3.27 | 1.33 | 1.95 (1.78–2.66) | 0.75 ± 0.22 |
| long | 4.63 | 2.88 ± 0.41 | 3.49 | 3.37 | 1.74 | 1.94 (1.76–2.64) | 0.98 ± 0.23 |

**Jorgensen's value.** His Fig. 4 (p. 77) gives `η` about 0.74 at the short model's length over
diameter, 18.2, and 0.77 at the long's 23.8. The tunnel's crossflow Reynolds number,
5.6 × 10⁵ sin α, stays below 5 × 10⁴ to 5° and its crossflow Mach number `M sin α` below 0.4:
there "C_dn = 1.2" (p. 15), and `η C_dn` is about 0.89 and 0.92. Over the tunnel's angles his
Figs. 1 and 2 give `C_dn` from 1.12 at 1° to 1.27, and his Fig. 6 raises `η` by about 0.04: 0.83
to 1.03 in all. Two cautions: his `η` comes from cylinders measured "only at very low subsonic
Mach numbers" (p. 17), and his 1.2 is for laminar separation, while the tunnel tripped its
boundary layer "to obtain turbulent flow" (TN D-4014 p. 4); past the critical Reynolds number his
`C_dn` falls to "between about 0.15 and 0.30" (p. 15). So the match below may be chance.

**What the table says.**

- Crossflow at the tunnel's angles is larger than the gap at every row, by either reading: 0.09
  to 1.74 from the tunnel's own curvature, 1.40 to 2.09 in hpr. It ranks first.
- From Mach 2.3 the curvature implies `K` from 0.66 to 1.05, each ±0.18 to ±0.23. Jorgensen's
  0.89 and 0.92 lie within 1.3 standard errors of all eight of those rows; hpr's 1.1 lies above
  all eight, by up to 2.2 standard errors.
- The short model at Mach 1.5 and 1.8 shows almost no curvature (`K` 0.32 and 0.07), while the
  long model at Mach 1.8 shows 0.71. A lead: at those two Mach numbers the report finds the short
  model's chamber axial force low, fins off, "believed to be because of the reflex lip" (p. 6).

## 2. What remains at `α → 0`

`b` and `c` come from the same seven points, and their errors correlate at −0.95 to −0.96 in
every row: a fit that bends more has a smaller slope at `α → 0`. So the readings can't say how
much of hpr's excess is body lift and how much its slope at `α → 0`:

- With `α |α|`, hpr's slope is 0.06 to 0.87 above the tunnel's `b`; with `α³`, whose `b` is 0.01
  to 0.61 higher, between 0.35 below and 0.43 above.
- With the curvature held at Jorgensen's `K`, the tunnel's slope at `α → 0` is 1.04 to 2.98,
  0.37 to 1.33 below hpr's at every row, and hpr fitted the same way reads 8.2% to 60.7% high.
- No single `K`, whatever `C_dn` really is, puts all 11 rows within 15%: the short model at Mach
  1.5 needs `K` at most 0.11, the long at 4.63 at least 0.32.

TN 3527 states its method within ±0.2 per radian (Summary, p. 1), for Mach number over nose
fineness from 0.4: the Arcas Robin is at 0.36 at Mach 1.5. The causes that could make hpr's
slope too high follow, the boattail first.

## 3. The boattail

M1.8e4 flies the 15° boattail by TN 3527's footnote 8, which takes its tangent cone (the cone
tangent to the profile) as the free stream with a slope of 2, for "reasonable results for bodies
having moderate amounts of boattail". Nothing measures it on its own (issue #90); the report
says flow separates over it "at the higher Mach numbers" (TN D-4014 p. 6). As hpr flies
it, the boattail's share is −0.177 at Mach 1.5 to −0.026 at 4.63. Slender-body theory, which the
committed design uses at every Mach number, gives it `2 (A_aft − A)/A_ref` = −1.324. The spread,
1.15 to 1.30, is as large as what remains at Jorgensen's `K` (0.37 to 1.33), so the boattail
ranks second: the only sized cause that size (the fitted form and the method's own ±0.2 aren't
causes). Nothing here says where between the two theories the real share lies.

## What this means for M1.8e6 and M1.8e7

- **Crossflow and the boattail together,** judged fitted at the plotted angles
  (`hpr.fitted_c_n_alpha`), not at `α → 0`. Jorgensen's `η C_dn` fits the curvature better than
  `K` = 1.1 from Mach 2.3, but at his `K` hpr still reads 8% to 61% high, and only the boattail is
  that size. The report plots `C_N` to 16° to 21° (Figs. 5(a), 6(a)), where `M sin α` passes 1:
  those points would show how `C_dn` grows with it, not the small-angle `K`. Body lift drives a
  slow rocket's drift in wind
  ([ADR-026](../DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18)),
  so M1.8e6 should decide whether a change applies below Mach 1, and regenerate the report.
- **Blunt tips are a coverage question.** The committed design's power-series nose has a vertical
  tip, which the method refuses. A lead: NASA TN D-4865 (1968) puts a Newtonian cap ahead of
  TN 3527's method and compares it from Mach 1.50 to 4.63.
- **M1.8e7's target** (the body within 15% of the tunnel) is judged as M1.8a fits it (ADR-036).
  There the fitted nose's body reads 15% to 73% high; the committed short model's, on
  slender-body theory, 2.26 to 2.34 where the tunnel reads 2.19 to 4.15
  ([table](../physics/aero.md#normal-force-through-mach-1)).
