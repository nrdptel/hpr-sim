# Mass properties of components

## In short

- **What it models:** the mass, centre of mass and inertia of each part (tubes, rings, shoulders,
  fins, rail buttons, lugs, mass components, recovery gear), every tube of a cluster, how they add
  up, and 49 built-in
  material densities.
- **Sources:** Meriam and Kraige's *Engineering Mechanics: Dynamics*, the *OpenRocket technical
  documentation* v13.05, Abbott and von Doenhoff's *Theory of Wing Sections*, Golub and Van
  Loan's *Matrix Computations*, and data sheets, specifications and handbooks for densities.
- **How well it is validated:** by analytic tests, the first of four [kinds of evidence][levels]:
  a cone, a tube, four fins and an off-axis payload agree with hand calculation to 1e-11, and fin
  cross-sections with exact numerical integration to 1e-13. Density unit conversions reproduce
  their sources, such as the *Wood Handbook*'s white ash at 678 kg/m³. Against
  [OpenRocket](../glossary.md#openrocket) 24.12, on the structure (the rocket without motors) of
  71 compared designs: the mass is within 1% on 62 and the centre of mass within 1% of the rocket's
  length on 63. Every file outside either shows a difference hpr names in a warning. The roll
  inertia is a median 1.686% apart, and that is explained: OpenRocket takes a shortcut for fins that
  hpr does not, and hpr's figure is the exact one for the fin as drawn
  ([below](#fins-rail-buttons-and-roll-inertia)); on the four designs with a cluster, OpenRocket
  also stacks the tubes on the cluster's axis ([below](#clusters-and-fillets)). The pitch inertia
  is within 1% on 54, and the rest have no named cause yet. Not compared with weighed parts or a real flight
  ([checked against OpenRocket](#checked-against-openrocket)).
- **What it leaves out:** fin fillets, the sliver between a flat fin root and the round tube, and
  the step ring at a nose shoulder. Parachutes weigh as flat circular canopies. Where a `.ork`
  file leaves something unsaid (a wall of no thickness, no material), hpr reads it as OpenRocket
  does, and two rules for overrides stay hpr's own, each measured
  ([below](#what-a-ork-leaves-unsaid-and-overrides)). Fin sections are hpr's own too: an airfoil
  fin weighs 0.6851 of a square slab of its outline, where OpenRocket's weighs 0.85, so hpr's
  airfoil fins are 19.4% lighter, with no warning ([below](#fins-rail-buttons-and-roll-inertia)). Packed
  recovery gear and mass components are OpenRocket's too, down to the size one takes when its file
  writes none ([below](#packed-parts)). A cluster's inertia and fin fillets remain measured,
  visible departures ([below](#clusters-and-fillets)); designs with parts hpr does not read (pods, parallel stages and
  five skipped parts across three kinds) are retained as reduced designs ([the format guide](../format/ork.md#what-hpr-keeps-for-writing-the-file-back)).

## Code and sources

Code: `hpr_design::mass` (`MassProperties`), `hpr_design::parts`, `hpr_design::fins`,
`hpr_design::material`, `hpr_design::materials`. The nose and transition solids are in
[Shapes](shapes.md).

Sources:

- **[MK]** J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*, appendix B (moments
  of inertia of standard solids, the parallel-axis theorem and the inertia tensor).
- **[GVL]** G. H. Golub and C. F. Van Loan, *Matrix Computations*, 4th ed. (2013), §8.5 (the
  Jacobi eigenvalue method).
- **[TD]** S. Niskanen, *OpenRocket technical documentation* v13.05 (2013), §3.2.2, pp. 25–29 (fin
  planforms), §3.4.4, pp. 49–50 (cross-sections), §4.2.3, p. 66 and Table 5.1, p. 75 (component
  masses), pinned as `openrocket-techdoc-13.05`.
- **[AvD]** I. H. Abbott and A. E. von Doenhoff, *Theory of Wing Sections*, Dover (1959), eq. 6.2
  (NACA four-digit thickness distribution).

## Frames and conventions

These conventions were set in [ADR-006][adr-006], the decision on component geometry and mass
properties.

- **Body axes** follow [Frames](frames.md): `z` along the axis toward the nose, `x` the zero radial
  direction, `y = z × x`. Roll angles run from `x` toward `y`.
- **A component's frame** has body axes and its origin on the axis at the component's forward end,
  or at a nose cone's tip, so the component lies at `z ≤ 0`. The design tree places it by
  translating it to its station ([Design tree](design.md)).
- **`MassProperties`** holds the mass, the centre of mass in body axes, and the **full** inertia
  tensor about the centre of mass. The tensor is taken with the positive products-of-inertia
  convention: `I = ∫ (|r|² E − r rᵀ) dm`, so `I_xy = −∫ x y dm`.
- **Operations** ([MK]):
  - Parallel axis: `I_p = I_cg + m (|d|² E − d dᵀ)`, with `d = cg − p`.
  - Rotation: `cg′ = R cg`, `I′ = R I Rᵀ`.
  - Combination: sum the masses, mass-weight the centres, and sum each tensor moved to the common
    centre.
  - Zero total mass gives the plain average of the centres, so placeholders stay finite.
- **Validity.** `validate` requires a finite, non-negative mass, and a tensor that is symmetric (to
  1e-9 of its largest entry) with non-negative principal moments obeying `I₁ + I₂ ≥ I₃`. That
  condition is the same as `J = tr(I)/2 E − I = ∫ r rᵀ dm` being positive semidefinite. Principal
  moments come from cyclic Jacobi ([GVL] algorithm 8.5.1), accurate for repeated eigenvalues where
  the closed-form trigonometric method loses `√ε`.

## Standard solids ([MK])

- **Hollow cylinder**, radii `R ≥ r`, length `L`: `I_axis = m(R² + r²)/2` and
  `I_across = m((R² + r²)/4 + L²/12)`. This covers body tubes, inner tubes and couplers, centering
  rings, bulkheads (`r = 0`), launch lugs, tube fins, and shoulders.
  - `r = R` is allowed, and is a tube of **no wall**: mass `π(R² − r²)L ρ` is exactly zero, and so
    is the tensor. That is a real thing for a design to say — an imported `.ork` says it of twelve
    parts ([`.ork` design files](../format/ork.md#what-is-left-out-and-why)) — and refusing it
    would force a reader to invent a wall instead. A **centering ring** is the exception: a bore
    that reaches the rim leaves no ring at all, so `CenteringRing` refuses `r ≥ R` rather than
    weighing nothing in silence. `Wall::Shell` also still refuses a zero thickness, because a
    solid of revolution says "filled" with `Wall::Filled` and a zero there is a mistake, not a
    statement.
  - Loft used `mL²/12` with no radial term, and no roll inertia at all ([Loft lesson L44](../decisions-and-roadmap.md#l44)).
- **Solid cylinder**, radius `a`, height `h`: `I_axis = m a²/2` and `I_across = m(3a² + h²)/12`.
  This covers mass components, packed parachutes, streamers and shock cords ([TD] Table 5.1
  treats recovery parts as cylinders too), and each disc of a rail button.
- **Rail button:**
  - Three coaxial discs stacked outward on a radial line: base (outer diameter), waist (inner
    diameter), flange (outer diameter).
  - The waist height is the total height less the base and flange.
  - Buttons and lugs may repeat along the axis (`count`, `spacing_m`).
- **Shoulder:** a hollow cylinder beyond the profile's end. A capped shoulder adds a disc of its
  inner radius and wall thickness, flush with its far end. The step ring between a nose's base
  radius and its shoulder is not modeled.
- **Parachute:** `m = ρ_s π D²/4 + n ℓ ρ_l`, the nominal area of a flat circular canopy plus its
  shroud lines. A conical or hemispherical canopy has more cloth than `πD²/4`; give its mass
  through an override ([Design tree](design.md)) or a matching nominal diameter.
- **Streamer:** `ρ_s × length × width`. **Shock cord:** `ρ_l × length`.

## Fins

- **Planforms** ([TD] §3.2.2):
  - Trapezoidal: root chord `c_r`, tip chord `c_t` parallel to the body, span `s`, and sweep `x_t`
    from the root leading edge to the tip leading edge.
  - Elliptical: `c(h) = c_r √(1 − (h/s)²)`, centred on the root chord (implied by [TD] eq. 3.71).
  - Freeform: a simple polygon from the root leading edge `[0, 0]` to the root trailing edge
    `[c_r, 0]`, closed along the root. Crossing edges, points below the root, and outlines that
    don't run from the origin aft along the root are errors.
- **Cross-sections.** Each chord from `a` to `b` has a thickness distribution `t(x)`. [TD] uses
  the cross-section for drag only; hpr also counts the volume it removes.
  - **Square:** `t(x) = t`.
  - **Rounded:** semicircular edges of radius `a_r = min(t, c)/2`, so a chord shorter than `t`
    near a pointed tip is a disc of diameter `c`. Its moments are closed forms in `a_r`:
    `D₀ = a_r²(2 − π/2)`, `D₁ = a_r D₀ − a_r³/3`, `D₂ = a_r² D₀ − π a_r⁴/8` for the removed edge
    material, and `E₀ = 8a_r⁴ − 3π a_r⁴/2` for `∫t³`. A wide rounded chord loses
    `(1 − π/4) t²` of section area.
  - **Airfoil:** `t(x) = 10 t P(ξ)` with the NACA four-digit polynomial
    `P = 0.2969√ξ − 0.1260ξ − 0.3516ξ² + 0.2843ξ³ − 0.1015ξ⁴` ([AvD]).
    - Its maximum is `1.0003 t` at `ξ = 0.2998`.
    - Its moments `10∫ξᵏP = 0.685083, 0.288033, 0.158919` hold term by term, and
      `1000∫P³ = 0.4728895` comes from mpmath.
    - An airfoiled fin weighs 68.5% of the square slab, with its centroid at 42% chord.
    - A hand-sanded "airfoil" is between the two; [TD] doesn't define the section.
- **Integrals.** Per fin, over the span `h` with `r = R_b + h`, and chordwise moments
  `M_k = ∫ x^k t dx` and `T = ∫ t³/12 dx`, taken per unit density:
  `V = ∫M₀`, `∫r = ∫rM₀`, `∫r² = ∫r²M₀`, `∫x = ∫M₁`, `∫x² = ∫M₂`, `∫rx = ∫rM₁` and `∫τ² = ∫T`.
  The span integration is split at every vertex height and runs adaptively.
  - With the fin at roll 0 (points at `(r, τ, −x)`):
    `I_xx = ∫(τ² + x²)`, `I_yy = ∫(r² + x²)`, `I_zz = ∫(r² + τ²)` and `I_xz = ∫ r x`, all `dm`.
  - Loft ignored the span and fixed a freeform fin's CG at `0.42 c_r`
    (Loft lessons [L44](../decisions-and-roadmap.md#l44) and [L45](../decisions-and-roadmap.md#l45)).
- **Tabs** are square slabs below the root, `−h_tab ≤ h ≤ 0`, with closed-form integrals. A tab
  must lie along the root chord and reach no deeper than the body radius. Loft never
  read them ([Loft lesson L46](../decisions-and-roadmap.md#l46)).
- **Root.** The flat root is placed at radius `R_b`; the sliver between it and the curved tube,
  `t²/8R_b` deep, is ignored. Fillets are not modeled yet.
- **Cant** `δ` turns each fin and its tab about the fin's outward span axis through the root
  mid-chord, right-handed, so a positive cant turns fin 0's leading edge toward `−y_B`
  (`positive_cant_turns_the_leading_edge_toward_negative_y`). [TD] doesn't state the pivot. Mass and trace are unchanged; the
  products of inertia in the fin's own frame grow as `sin 2δ`.
- **Sets** roll the fin to `φ_k = φ₀ + 2πk/N` and combine. Three or more fins are isotropic across
  the axis; one or two are not, and the full tensor keeps the difference.

## Materials

`Density` is `bulk` (kg/m³), `surface` (kg/m²) or `line` (kg/m). A part asking for the wrong kind
is an error. A design stores the values, not a library key. The built-in values and their sources
are in `hpr_design::materials` and summarized below.

`hpr_design::materials::BUILTIN` holds 49 materials. Each carries its source (with table or page),
the URL it was read from, and a basis:

- **published:** the source states the value.
- **derived:** computed from the source's numbers.
- **maximum:** a specification's upper weight limit.
- **vendor:** a seller's figure, used where no specification exists.

| group | values | sources | basis |
|---|---|---|---|
| hobby tubes | cardboard 790, kraft phenolic 950, Blue Tube 1250, Quantum 1090 kg/m³ | LOC Precision and Public Missiles weight tables (mass over wall volume); Always Ready Rocketry's own material file | derived; published |
| composites | G10/FR-4 1800, filament-wound E-glass 1990, carbon/epoxy 1580 kg/m³ | Norplex-Micarta NP130, Comptec, Hexcel HexPly 8552 data sheets | published |
| metals | Al 6061 2700, Al 7075 2800, steel 7850, Ti-6Al-4V 4430, brass 8500 kg/m³ | Kaiser Aluminum, MIL-HDBK-5J, TIMET, Copper Development Association | published |
| woods | balsa 180; basswood, yellow birch, Sitka spruce, eastern white pine, sugar maple, northern red oak from `G₁₂`; birch plywood 680 kg/m³ | Wood Handbook FPL-GTR-190 (pinned as `fpl-gtr-190-wood-handbook`), p. 2-21 and Table 5-3a; Riga Wood Plywood Handbook | published; derived |
| plastics | PLA 1240, ABS 1040, PETG 1270, nylon 6/6 1140, PC 1200, PMMA 1190, acetal 1420, PS 1040, PVC 1400, HDPE 955, epoxy 1180, Depron 40, paper 755 kg/m³ | manufacturers' data sheets (NatureWorks, SABIC, Eastman, Celanese, Covestro, Röhm, AmSty, Charlotte Pipe, Chevron Phillips, West System, Depron, HP) | published (paper derived) |
| fabrics | ripstop 1.1 and 1.6 oz/yd², Mylar and LDPE film at 1 mil, paper 80 g/m², Nomex cloth, silnylon | MIL-C-7020H, DuPont Teijin, Dow, HP, MIL-C-83429B; a seller for silnylon | maximum, derived, published, vendor |
| cords | nylon cord types I and III, tubular nylon ½", 9/16", 1", ⅛" and ¼" Kevlar, ¼" bungee, Tex 80 Kevlar thread | MIL-C-5040H, MIL-W-5625K, MIL-C-5651D, A-A-55220; Giant Leap Rocketry's measurements for Kevlar | maximum, vendor, published |

- **Wood** at 12% moisture: `ρ = 1000 G₁₂ (1.12)` (Wood Handbook eq. 4-12). The handbook's own
  example, white ash at `G₁₂ = 0.605`, gives 678 kg/m³.
- **Specification maxima** overstate typical cloth and webbing: Giant Leap's measured 9/16"
  tubular nylon is 12% under MIL-W-5625K's limit.
- **openrocket-database** (Apache-2.0) was used only as a cross-check. Two problems turned up in it:
  - Its ripstop weights use 31 g/m² per oz/yd² (the factor is 33.906), so they are 8.6% low.
  - Its "Plywood, aircraft" at 337–361 kg/m³ is lite-ply, not birch.

## Checked against OpenRocket

**In short.** hpr adds a design's parts into its *structure*: every stage together, with no
motor. OpenRocket 24.12 computes the same thing. On 2026-09-21 the two were compared on every file
OpenRocket opens among hpr's `.ork` test files: the *reference library* (designs gathered under
`refs/`, many of them private files other people shared) and the 17 example designs that ship
inside OpenRocket's program file (its Java *jar*). The current default survey compares 71 designs.
Some hold the same design found in two places (several private files are copies of OpenRocket's
examples), so there are 51 different files by content. Mass and centre of mass agree closely on most,
and every file outside 1% has a cause hpr already warns about. The roll inertia is a median 2.351%
apart, and that gap is
OpenRocket's shortcut for fins ([below](#fins-rail-buttons-and-roll-inertia)).
This was [M2.2a](../decisions-and-roadmap.md#m2-2a); [ADR-060][adr-060] records how it was decided.
The numbers below are from the current scratch-excluding rerun after
[M2.2b4](../decisions-and-roadmap.md#m2-2b4), which settled two more of its causes
([next section](#what-a-ork-leaves-unsaid-and-overrides)).

**What you can check yourself.** The private files are not public, so only counts come from them,
and a fresh clone cannot reproduce the 71-file table. It can check the probe tube and Loft's public
demo designs (`cargo test -p xtask ork_mass`), and it can run the script on `.ork` files of its own
to see OpenRocket's numbers (*Run it yourself*, at the end of this section); comparing hpr's with
them is not automated yet.

**How.** `validation/oracles/openrocket/mass.py` runs OpenRocket and asks it for each design's
structure, after saving the design once so that every automatic dimension is the one OpenRocket
settles on. `cargo xtask ork` compares hpr's with it:

- the mass, relative to OpenRocket's;
- the centre of mass's [station](../glossary.md#station), as a share of the rocket's length;
- the roll inertia (about the rocket's axis) and the pitch inertia (about an axis across it), each
  about the program's own centre of mass and relative to OpenRocket's. Pitch is taken as the mean
  of the two inertias across the axis, which does not depend on how either program turns its axes
  about the rocket's length.

Which of OpenRocket's numbers is roll was measured, not assumed. The script first reads a probe: one
tube, 1 m long, 50 mm in outer radius with a 2 mm wall, of a material at 1,000 kg/m³. Worked by
hand, it weighs 0.61575 kg, with a roll inertia of `m (r_o² + r_i²)/2` = 0.0014790 kg·m² and a
pitch inertia about its middle of `m ((r_o² + r_i²)/4 + L²/12)` = 0.052052 kg·m². OpenRocket's
numbers match to 15 digits, and so does hpr's layout of the same file (the test
`the_probe_tube_is_the_one_worked_by_hand`).

The thresholds, 1% of the mass and 1% of the length, were set before any design was measured. A
design outside either needs a written reason, not a pass.

**The results.**

| | within 0.1% | within 1% | median |
|---|---|---|---|
| mass | 55 of 71 | 62 of 71 | 0.001% |
| centre of mass (share of length) | 58 of 71 | 63 of 71 | 0.000% |
| pitch inertia | 39 of 71 | 54 of 71 | 0.065% |
| roll inertia | 10 of 71 | 29 of 71 | 1.686% |
| roll inertia, OpenRocket's fin shortcut in hpr's place | 52 of 71 | 52 of 71 | 0.001% |

Counting each file's content once, 45 of 51 are within 1% in mass and 46 of 51 in centre of mass.
Before [M1.9b](../decisions-and-roadmap.md#m1-9b) read every tube of a
[cluster](../glossary.md#cluster), 58 and 59 of 71 were.
Before [M2.2b1](../decisions-and-roadmap.md#m2-2b1) (reading what a `.ork` leaves unsaid), 57 of
74 files were within 1% in mass and 58 in centre of mass (median mass 0.020%). Those are the earlier
74-file measurement; the current default survey is the 71-file table above.

**The 9 files outside a threshold** are 6 different files by content, each a different design.
Each has one or two of two causes, and
hpr already warns of every one when it reads the file. `cargo xtask ork` works the causes out from
those warnings, counts them by content as below, and fails if a file outside has none. One file has
two causes, so the last column adds to 7:

| cause | what hpr does | what OpenRocket does | files by content |
|---|---|---|---|
| fin fillets (the rounded glue joint along a fin's root) | leaves them out, with a warning | counts them | 2 |
| pods, parallel stages, tube fins and parts left out | keeps them unread (the design is [reduced](../format/ork.md#what-hpr-keeps-for-writing-the-file-back)) | counts them | 5 |

A third cause, a cluster read as one tube, went when
[M1.9b](../decisions-and-roadmap.md#m1-9b) read every tube of a cluster: its two files are now within both
thresholds. `cargo xtask ork` prints each of the 9 with the parts that differ most, by id, or by name in an
older file that writes no ids. A private design is named only by the start of its file's hash.

**A worked example, now settled.** In the first comparison
([M2.2a](../decisions-and-roadmap.md#m2-2a)) the OpenRocket jar's *Two stage high power rocket* was
18.74% heavier in hpr: 1.956 kg in OpenRocket, 0.3666 kg more in hpr. All of it was the nose cone.
Its shoulder is written with a radius of 49.28 mm, a length of 50.8 mm and a wall thickness of 0.
hpr read that as solid: a cylinder of `π × 0.04928² × 0.0508` = 3.875e-4 m³ of the file's own
material, polypropylene at 946 kg/m³, weighs 0.3666 kg. OpenRocket gives the same shoulder no mass,
and since [M2.2b1](../decisions-and-roadmap.md#m2-2b1) so does hpr, so the file is within 1% in
both mass and centre of mass. Two causes the first comparison counted, this shoulder of no wall (6
files by content) and a part written with no material (1), are gone the same way.

**Two more conventions, which move no file outside a threshold.**

- **Inertia under a mass override.** A departure kept on purpose
  ([below](#what-a-ork-leaves-unsaid-and-overrides)). Loft's public `stage-weighed.ork` overrides
  its stage to 1.234 kg on 0.614 kg of parts, a ratio of 2.009; hpr scales the stage's inertia by it
  and OpenRocket does not, so hpr's pitch inertia is +100.9% apart and its roll +108.6%: the largest
  inertia differences measured.
- **An airfoil fin section.** hpr's airfoil fin weighs less than OpenRocket's: the CONTROL fins of
  the jar's *Simulation scripting* example are 0.0378 kg in hpr and 0.0469 kg in OpenRocket, 19.4%
  lighter, with no warning. It is a departure kept on purpose
  ([below](#fins-rail-buttons-and-roll-inertia)).

**Roll and pitch inertia.** On the six Loft demo designs OpenRocket opens, the roll inertia is 1.2%
to 3.8% apart, though their mass, centre of mass and pitch inertia agree within 0.1% and every part
of each is within 0.3 g of OpenRocket's. Across all 71 compared designs the median is 1.686%. It is the fins:
OpenRocket takes a shortcut for a fin set's roll inertia, and hpr integrates the fin exactly
([below](#fins-rail-buttons-and-roll-inertia)). With OpenRocket's shortcut in hpr's place, the
median is 0.001% and 52 files are within 1%. The shortcut takes OpenRocket's own mass for each fin
set, paired by id, or in an older file by name, so the way OpenRocket weighs a section (below) is
set aside too. Five of the six Loft demos come within 0.0005%. The sixth, whose fins are
elliptical, is 0.093% apart in that row, and within 0.0002% once OpenRocket's ellipse is drawn as
OpenRocket draws it, a 30-sided polygon (a test). For each of the 19 files still outside 1% (13 by
content), `cargo xtask ork` names a cause, and it fails if it can't. Each has exactly one:

| cause | files by content |
|---|---|
| a mass override covering the parts inside (a departure, [below](#what-a-ork-leaves-unsaid-and-overrides)) | 5 |
| parts hpr keeps unread (a reduced design) | 6 |
| a cluster's tubes, which OpenRocket weighs stacked on the cluster's axis ([below](#clusters-and-fillets)) | 2 |

The cluster cause is sized, not only present: the survey names it only when hpr's roll inertia,
less the spread of the clusters' own tubes, is within 1% of OpenRocket's. The two are +1.01% and
+2.08% apart, and +0.04% and +0.00% without the spread.

The pitch inertia is within 1% on 54 of 71. The 17 outside have no named cause yet, and no bound
is known; on the fin probes below, pitch differs by up to 0.41% where the fins weigh the same.

**What it leaves out.** Motors: this is the structure alone, and a motor's mass is
[M2.2c](../decisions-and-roadmap.md#m2-2c)'s. Only the design's selected
[configuration](../glossary.md#configuration) (the one OpenRocket opens it with) is weighed. The 4
files OpenRocket 24.12 does not open are not compared.

**Run it yourself.** CI does not run OpenRocket. It holds hpr to OpenRocket's saved answers for
Loft's seven public demo designs, `validation/fixtures/ork/openrocket-mass-loft-demo.json`, with
`cargo test -p xtask ork_mass`: OpenRocket opens six of the seven, and hpr is within 0.1% of it on
those six in mass, centre of mass and pitch inertia, and every part of each within 0.3 g. With
Java 17 and the OpenRocket jar
(`cargo xtask refs fetch`), from the repository root:

```sh
refs/venv/bin/python validation/oracles/openrocket/mass.py \
    validation/fixtures/ork/openrocket-mass-loft-demo.json validation/fixtures/ork/loft-demo
refs/venv/bin/python validation/oracles/openrocket/mass.py corpus-out/openrocket-mass.json refs --jar
cargo xtask ork
```

The script takes any directory of `.ork` files in place of `refs`; `cargo xtask ork` compares hpr
with the record of the reference library only.

[adr-060]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-060-m22-split-and-the-structures-mass-held-to-openrockets-2026-09-21

## What a `.ork` leaves unsaid, and overrides

**In short.** A design file does not say everything. What does a nose cone's shoulder written with
a wall thickness of 0 weigh? What is a part that names no material made of? When a part and the
parts inside it both have an [override](../glossary.md#override) (a mass or centre of mass the
designer typed in), which wins? OpenRocket, which writes these files, has an answer to each, and
its answer is what the file means to the person who wrote it. So hpr asks it:
`validation/oracles/openrocket/conventions.py` writes 32 small *probe designs*, each a rocket of a
few parts built to ask one question, runs OpenRocket 24.12 on them and records its answers.
The test module `hpr_validate::openrocket::tests` reads the same designs with hpr and holds hpr to
them. Where hpr keeps a rule of its own, the test pins how far apart the two are. This was
[M2.2b1](../decisions-and-roadmap.md#m2-2b1); [ADR-061][adr-061] records the decisions.

How far to trust it: each probe asks about one kind of part at one size, so each reading is
measured, not proven for every case. OpenRocket's defaults were read with its preferences as a
fresh install sets them; an OpenRocket whose preferences were changed may give others.

**Read as OpenRocket reads it.** On every probe of the readings in this table hpr's mass is
OpenRocket's within
0.001%, part by part as well as whole (the worst, a transition, is 0.0003% apart), and its centre
of mass within 0.001 mm. Where no fin, rail button or recovery part is in the probe, the inertias
agree within 0.001% too. One gap is pinned rather than hidden, and described below: an elliptical
fin set weighs 0.18% more. None of these readings raises a warning, since nothing is assumed:

| the file says | what it weighs (OpenRocket 24.12, and now hpr) |
|---|---|
| a nose cone, transition or body tube with a wall of 0 | nothing: the part keeps its shape (hpr's drag uses the shape, not the wall) but has no wall; a part meant to be solid is written `filled` |
| an inner tube, coupler or launch lug with a wall of 0 | nothing |
| a shoulder with a wall of 0, or none written | nothing, whether or not the file closes its end with a cap, on a hollow nose or a filled one |
| a filled nose cone with a walled shoulder | the solid cone plus the shoulder's own wall |
| a nose cone, transition or body tube with no thickness written | a 2 mm wall, whatever its radius (measured on a nose cone and a tube at 50 mm and at 30 mm, and on a transition) |
| a part weighed by its volume (a nose, transition, tube, coupler, engine block, fin set, ring or lug) with no material | cardboard, 680 kg/m³ |
| a canopy or streamer with no material | ripstop nylon, 0.067 kg/m² |
| shroud lines or a shock cord with no material | a 2 mm elastic cord, 0.0018 kg/m |
| a rail button with no material | Delrin, 1,420 kg/m³ |

A worked example with the probe's numbers: a conical nose cone 0.3 m long on a 50 mm base, with a
2 mm wall of a material at 1,000 kg/m³, weighs 0.091726 kg. With a shoulder 100 mm long, 48 mm in
radius and a 2 mm wall, it weighs 0.150787 kg. With the same shoulder written with a wall of 0 it
weighs 0.091726 kg again, in both programs (the probe *a nose whose shoulder has no wall*). Before
this step hpr read that shoulder as solid, and would have added `π × 0.048² × 0.1 × 1000` =
0.724 kg.

**Which override wins.** An override is a number the designer typed in place of what the parts add
up to, usually after weighing the real thing. The file can say that an override *covers the parts
inside*: that the number is for the part together with everything attached to it. The probes find
hpr and OpenRocket agree on which override wins, and on where a centre is measured from:

- An override on a part that covers the parts inside it wins over any of theirs, and a stage's wins
  over everything in the stage.
- A centre-of-gravity override is measured from the part's front, not from its shoulder's, and the
  shoulder moves with the part.
- A centre-of-gravity override alone, written to cover the parts inside, sets the whole assembly's
  centre. (The two place the parts inside differently, which shows only in the inertia: below.)

This settles [Loft lesson L51](../decisions-and-roadmap.md#l51), whose rule for this came from
OpenRocket's source and was unsettled by up to 133 mm. The test is
`override_precedence_matches_oracle`.

**Where hpr keeps its own rule.** Each of these is a *departure*: hpr knowingly differs from
OpenRocket, and a test pins by how much.

| when | hpr | OpenRocket | apart on the probe |
|---|---|---|---|
| a mass override covers the parts inside and states no centre | keeps the centre the parts lay out | puts it at the overriding part's own, ignoring where the parts inside sit | hpr's centre 3.7 mm forward of OpenRocket's, or 19.7 mm if the part inside has an override of its own |
| a mass override covers more than one part | scales the inertia of everything it covers by the override's ratio | scales only the overriding part's own inertia, and keeps the parts inside at theirs; a stage, having none of its own, scales nothing | roll inertia 6.9% to 37% lower in hpr under a tube's; 2.5 to 5.0 times OpenRocket's under a stage's |
| a centre override covers the parts inside | moves the whole assembly, so the inertia about the new centre is the assembly's own | moves the overriding part alone, and adds the parts inside where they were | the centre agrees; pitch inertia 2.65% lower in hpr |

Under a tube's covering override hpr's roll inertia is the *lower* one, though hpr scales more of
the parts. OpenRocket keeps the inertia of the parts inside while leaving their mass out of the
total, so its assembly carries inertia for mass it does not count.

Why keep them: a builder who weighs a tube with its fins and motor mount inside has not moved their
centre, so the centre the parts lay out is the better estimate. And scaling the inertia with the
mass keeps it consistent with the mass: the extra weight sits where the parts' weight does.
Neither rule is right for every rocket (a heavy avionics bay at the centre of mass adds little
inertia). On a single part, with nothing inside, the two programs agree.

One more difference cannot be said in hpr's design format: a part that overrides both its mass and
its centre, with one covering the parts inside and the other not. hpr scopes a part's overrides
once, takes the mass's, and warns. On the probes the centre is 4.7 mm apart when the centre's
override is the covering one, and agrees when the mass's is (with pitch inertia 8.2% lower in hpr).

**Two gaps the probes found**, both settled in [M2.2b2](../decisions-and-roadmap.md#m2-2b2)
([next section](#fins-rail-buttons-and-roll-inertia)): a rail button sat 5 mm further aft in hpr
than in OpenRocket, and is now where OpenRocket puts it; and OpenRocket's elliptical fin weighs
0.18% less than hpr's exact ellipse, which matches, to 13 digits, a 30-sided polygon drawn inside
the ellipse at equal angles. hpr keeps the ellipse.

**What it leaves out.** An inner tube, coupler or lug that writes no thickness at all is read as
no wall, with a warning. OpenRocket gives it a wall of its own: on the probe, 0.5 mm for a 20 mm
inner tube, 1 mm for a 5 mm lug, and none for a coupler. One size each does not say whether that
wall follows the radius, and no file in the reference library has one, so hpr does not follow it
yet; the test pins the difference, −2.4% in the mass of that probe's structure.

**Run it yourself.** With Java 17 and the OpenRocket jar (`cargo xtask refs fetch`), from the
repository root:

```sh
refs/venv/bin/python validation/oracles/openrocket/conventions.py \
    validation/fixtures/ork/openrocket-conventions.json
cargo test -p hpr-validate openrocket
```

[adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21

## Fins, rail buttons and roll inertia

**In short.** A rocket's roll inertia is its resistance to spinning about its long axis. hpr's was
a median 2.351% from OpenRocket's on the 71 compared designs, and nothing explained it. It is the fins.
OpenRocket works out a fin set's roll inertia with a shortcut; hpr integrates over the fin exactly.
[M2.2b2](../decisions-and-roadmap.md#m2-2b2) measured the shortcut on 33 more probe designs, each a
tube and one part. hpr keeps its own: a *departure*, a rule hpr keeps on purpose, measured and
pinned by a test. The same probes settle how each fin section is weighed and where a rail button
sits. [ADR-062][adr-062] records the decisions.

How far to trust it: the shortcut is inferred from OpenRocket's output; its source is GPL, so the
project does not read it. It matches every fin probe but two to 1e-12, and those two are
explained below. Every tapered probe has a span half its root chord; Loft's demos, whose spans are
0.39 to 0.50 of the root, hold to 0.0005% as well.

**The other parts agree, bar a rail button.** A bulkhead, centering ring, inner tube, mass
component, parachute, shock cord and streamer each have OpenRocket's mass, centre of mass and both
inertias on their probes, to 1e-15. A rail button's inertias are apart by up to 0.05% of its
probe's, and a launch lug's pitch inertia by 0.03% (below).

**OpenRocket's shortcut.** For a set of two or more fins, OpenRocket spreads the set's mass `m`
evenly along a thin rod that runs straight out from the body, at radius `R`, to `R + hₑ`, and
takes that rod's roll inertia. `hₑ` is an *effective span*:

```text
I_roll = m (R² + R hₑ + hₑ²/3),   hₑ² = A h / c_r
```

Here `A` is one fin's area, `h` its span and `c_r` its root chord. For a rectangular fin, `hₑ` is
the span, and the shortcut is exact except that it leaves out the fin's thickness. `hₑ` is shorter
than the span when the fin narrows outward, and longer when it widens. The rule is inferred from
OpenRocket's output, and every tapered probe narrows outward, so a fin that widens is the rule
carried past what was measured. A tab's mass goes where the
fin's does, and neither the section nor the thickness enters. A single fin gets the same rod about
its own middle, `m hₑ²/12`.

**A worked example: the probe's trapezoid.** Three fins with a 100 mm root chord, a 50 mm tip
chord, a 50 mm span and 50 mm of sweep, 3 mm thick, of 1,000 kg/m³, on a tube 50 mm in radius.
Each fin has an area of 0.00375 m², so the set weighs 33.75 g. Then `hₑ² = 0.00375 × 0.05 / 0.1 =
0.001875 m²`, so `hₑ` = 43.3 mm, and `I_roll = 0.03375 × (0.0025 + 0.05 × 0.0433 + 0.000625) =
1.7854e-4 kg·m²`, OpenRocket's figure. hpr's exact integral is 1.8284e-4 kg·m², 2.4% more: the
fin's outer part weighs more than the rod puts there.

The rod spreads the mass evenly, but a real fin's mass follows its chord, so the sign depends on
the outline: a triangle's mass sits nearer the body than the rod's. A tab lies inside the body
tube, 40 to 50 mm from the axis on the probe, but the rod puts its mass out with the fin's, so
OpenRocket's figure is the larger there.

| the probe's fin set | hpr's roll inertia (kg·m²) | OpenRocket's | hpr against OpenRocket |
|---|---|---|---|
| rectangular, 100 mm by 50 mm | 2.6253e-4 | 2.6250e-4 | +0.013% (the thickness) |
| the trapezoid above | 1.8284e-4 | 1.7854e-4 | +2.41% |
| triangular, 100 mm root, 50 mm span | 1.0314e-4 | 1.0540e-4 | −2.14% |
| the trapezoid with a tab 50 mm by 10 mm | 1.9199e-4 | 2.0234e-4 | −5.12% |

The two fin probes the shortcut does not match to 1e-12 are an elliptical fin set (its polygon,
below) and a canted one (by 2.66e-5 of the probe's roll inertia, not traced).
`hpr_validate::openrocket::openrocket_fin_set_roll_kg_m2` states the shortcut, and the 74-file
comparison uses it for the second roll row of the table [above](#checked-against-openrocket). The
test `each_part_alone_is_openrocket_s_or_pinned` holds every probe of this section to OpenRocket's,
or pins how far apart they are.

**How each fin section is weighed.** OpenRocket weighs a fin set as its outline times its thickness
times a factor for its section: 1 for square, 0.99 for rounded and 0.85 for an airfoil, whatever
the thickness (checked at 3 mm and 6 mm). hpr works the section out: a rounded edge is a
semicircle, 0.9914 of the square section on the probe, and an airfoil is NACA's four-digit
section, 0.6851 (Abbott and von Doenhoff). A file that says `airfoil` does not say which airfoil,
and a builder who weighed the fins can give their mass. So hpr keeps its sections. The elliptical
fin stays the exact ellipse too; OpenRocket's 30-sided polygon weighs 0.18% less.

**Where a rail button sits.** OpenRocket gives a rail button no length. It puts the button's centre
where a part of no length would sit, whichever end of the tube the file measures from, and a row's
first button there, the rest following aft. hpr now reads a `.ork` button so (issue
[#151](https://github.com/nrdptel/hpr-sim/issues/151)). Before, hpr put the row's forward edge, middle
or aft edge on the position. The move depends on which end the file measures from:

| measured from | how the row moves (r the button's outer radius, s the spacing centre to centre, n buttons) | two buttons of 10 mm outer diameter, 100 mm centre to centre |
|---|---|---|
| the top, after a part, or absolute | forward r | 5 mm forward |
| the middle | aft (n − 1)s/2; one button does not move | 50 mm aft |
| the bottom | aft r + (n − 1)s | 105 mm aft |

A flight leaves the rail when its aft-most guide does. So a row placed from the top, after a part
or absolutely now leaves it a radius earlier, and one placed from the middle or the bottom leaves it
later: more rail to travel, so a little faster off the rail. The probes check the top, the middle and the bottom, with one button
and with a row of two.

**What is left, each pinned by a test.**

- Fin fillets: 0.81% of the probe's mass at a 5 mm radius, left out, with a warning.
- Small and not traced: a canted fin set's mass (−0.004%), fins' pitch inertia (up to 0.41% where
  the fins weigh the same, on a single fin; up to 0.11% on the other probes), a launch lug's pitch
  inertia (0.03%) and a rail button's inertias (up to 0.05%).

The fillet omission is a measured departure, settled with the 5 mm and 10 mm probes in
[ADR-064][adr-064]. A cluster's departure is measured below. Two more gaps these probes found, both
in packed parts, are settled [below](#packed-parts).

**Run it yourself.** As for [the probes above](#what-a-ork-leaves-unsaid-and-overrides): the same
script writes these, and `cargo test -p hpr-validate openrocket` checks them.

[adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21

## Clusters and fillets

A [cluster](../glossary.md#cluster) is a motor mount with more than one motor tube. Since
[M1.9b](../decisions-and-roadmap.md#m1-9b) hpr weighs every tube where it sits, each with its own
[parallel-axis](../glossary.md#parallel-axis-theorem) term, and repeats what a tube holds in every
tube ([Clusters](design.md#clusters)). OpenRocket 24.12, asked on 24 probes
([ADR-075][adr-075]), agrees on the mass and the centre of mass to 1e-12, but not on the inertia.
It weighs a cluster's tubes as if stacked on the cluster's axis: a 3-ring at scale 1 and at scale
1.5 have the same inertias in OpenRocket. It does weigh an engine block inside each tube where it
sits. hpr does not copy the stacking, since the tubes are not on the axis.

The fixed 3-ring probe is a tube and a 200 mm inner tube with a 20 mm outer radius and 1 mm wall,
three of them 23.09 mm from the axis. OpenRocket's saved structure is 0.3813893481458014 kg, with
centre 0.24036243822075784 m, roll 0.0007674901427941916 kg m² and pitch 0.007191233036548777
kg m². hpr's mass and centre are the same, and its roll inertia is +5.11% and its pitch +0.273%
apart: the tubes' `3 m d²` (3.92e-5 kg m²) and half of it. On every cluster probe on the body's
axis the difference is exactly that, to 1e-12, whatever the pattern, scale or contents. Off the
axis, three differences are left and pinned as measured, with the spread taken out: a lone tube
10 mm off the axis (+0.303% roll, +0.016% pitch), and the pitch of two clusters off the axis
(+0.015%, +0.021%). Before then, reading one tube left hpr 12.85% light, 5.95 mm forward, 2.43% low
in roll and 3.68% low in pitch. `each_part_alone_is_openrocket_s_or_pinned` and
`a_cluster_weighs_as_openrocket_s_but_for_its_tubes_spread` check these.

A fin fillet is the rounded joint along a fin root. hpr reads a positive `filletradius`, warns, and
does not add a fillet solid or use `filletmaterial`. The 5 mm probe is 0.808% light, 0.737 mm
forward in centre and 0.439% low in pitch; the 10 mm probe is 2.79% light, 2.54 mm forward and
1.52% low in pitch. Their roll rows are zero because the comparison substitutes OpenRocket's fin
shortcut, as it does for every fin probe. The omissions remain deliberate until a fillet shape is
modelled and measured; [ADR-064][adr-064] records the decision.

[adr-064]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-064-clusters-fillets-and-unread-parts-remain-visible-departures-2026-09-22
[adr-075]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-075-a-cluster-is-one-tube-repeated-and-a-motor-in-it-one-motor-per-tube-2026-09-25

## Packed parts

A parachute, streamer, shock cord or mass component (an altimeter, a battery, ballast) is weighed
as a solid cylinder: its packed length and packed radius, where it sits in the tube
([above](#standard-solids-mk)). Two cases need a rule of their own, and hpr now takes OpenRocket's
for both, measured on probe designs OpenRocket 24.12 reads ([M2.2b3](../decisions-and-roadmap.md#m2-2b3),
[ADR-063][adr-063]). Like every OpenRocket rule on this page, they are inferred from what
OpenRocket prints, not read from its source.

**A file that writes no packed size.** OpenRocket packs the part 25 mm long and 12.5 mm in radius.
The radius is a fixed number, not the tube's bore: it is the same in bores 48 and 98 mm in radius,
and OpenRocket does not shrink it to fit a bore of 8 mm. Neither does hpr. Each
number stands alone, so a file that writes only a length gets the 12.5 mm radius, and one that
writes only a radius gets the 25 mm length. hpr reads a `.ork` the same way, with no warning.
Before, it read a length of zero with no warning, and a radius of zero with one.

**A mass override on a part that weighs nothing.** A parachute with no canopy, or a mass component
of 0 g, given a 30 g override: OpenRocket spreads the 30 g over the packing. So does hpr now. In a
packing 50 mm long and 20 mm in radius that is:

| | formula | 30 g in the probes' packing |
|---|---|---|
| roll inertia | `m r²/2` | 6.0 × 10⁻⁶ kg·m² |
| pitch inertia, about its own centre | `m (3r² + l²)/12` | 9.25 × 10⁻⁶ kg·m² |
| centre | halfway along the packing | 25 mm aft of its forward end |

Before, hpr put the 30 g at a point, which left the probe's roll inertia 0.805% low. Any other part
that weighs nothing still becomes a point mass under an override; both programs do that
([above](#what-a-ork-leaves-unsaid-and-overrides)).

**How well.** Eleven probes ask these questions, a tube and one packed part each. hpr's structure is
OpenRocket's to 1e-12 on every one, in mass, centre of mass, roll and pitch. The earlier 74-file
before-and-after measurement took the roll inertia, with OpenRocket's fin shortcut in hpr's place,
from 49 to 53 files within 0.1%, and from 55 to 57 within 1%. The current scratch-excluding survey
has 52 of 71 within 0.1% and 52 of 71 within 1% with that shortcut (56 before [M1.9b](../decisions-and-roadmap.md#m1-9b) weighed the
clusters' tubes where they sit):

| (earlier 74-file measurement) | before | after |
|---|---|---|
| centre of mass within 0.1% of length | 54 of 74 | 55 of 74 |
| pitch inertia within 0.1% (median) | 34 of 74 (0.110%) | 37 of 74 (0.086%) |
| roll inertia, fin shortcut in hpr's place, within 1% | 55 of 74 | 57 of 74 |
| roll inertia, hpr's own fins, within 1% (median) | 29 of 74 (2.112%) | 28 of 74 (2.354%) |

The last row moves the wrong way. hpr's own fin roll departs from OpenRocket's
([above](#fins-rail-buttons-and-roll-inertia)). In one file the point mass had left hpr's roll
inertia low, which happened to cancel part of that departure; the packing now adds it back.

**What it leaves out.** A packed size the file writes but hpr cannot read as a number is read as
zero, with a warning, as any unreadable number is. Every probe places its part from the tube's top,
so how the 25 mm length moves a part placed from the middle, the bottom or after another part is
worked out, not measured. The override rule was probed on a parachute, a mass component and a
shock cord; a streamer takes it too, by the same packing, without a probe of its own. Mass is
unchanged: the rules move mass, they add none.

**Run it yourself.** `refs/venv/bin/python validation/oracles/openrocket/conventions.py
validation/fixtures/ork/openrocket-conventions.json` writes the probes (it needs OpenRocket 24.12
and Java 17), and `cargo test -p hpr-validate openrocket` checks them.

[adr-063]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-063-packed-parts-read-and-weighed-as-openrocket-packs-them-2026-09-21

## Verification

- **By hand:**
  - `mass::tests`: two boxes make one box; point masses give the products of inertia; rolling
    swaps and mixes axes as `I′_xy = (I_xx − I_yy) sin θ cos θ`; rotation keeps the principal
    moments; motor elements land on the axis.
  - `tests::composite_rocket_inertia_matches_hand_calculation` (crate root) combines a filled cone, a tube,
    four fins and an off-axis payload. Each part's moments come from its own formula and the six
    tensor terms are written out; the result agrees to 1e-11.
  - `parts::tests::a_nose_cone_with_a_capped_shoulder_adds_up_by_hand` checks the cone, tube and
    cap to 1e-11.
  - `fins::tests`:
    - A rectangular fin is a box, and four fins are the sum of rotated boxes.
    - A swept fin's `I_xz` matches quadrature of the planform.
    - A fin canted 90° is the box turned.
    - Tube fins match the parallel-axis theorem.
- **Closed forms against quadrature:**
  - Each cross-section's `M₀, M₁, M₂, T` matches exact quadrature of `t(x)` to 1e-13, on a wide
    chord and one narrower than `t`.
  - The airfoil constants match to 1e-14.
  - Trapezoidal, elliptical and freeform areas and centroids match to 1e-12.
- **Properties** (`mass::tests`, proptest): combining is associative and order-free, turning a
  body keeps its principal moments, and the inertia about any point exceeds that about the centre.
- **Materials:** ids are unique, sources present, and the unit conversions reproduce the sources
  (1.1 oz/yd² = 37.3 g/m², 225 ft/lb = 6.61 g/m, white ash 678 kg/m³).
- **Loft lessons:**
  - [L44](../decisions-and-roadmap.md#l44) `thin_tube_inertia_includes_radial_term`.
  - [L45](../decisions-and-roadmap.md#l45) `hollow_transition_and_freeform_fin_cg_are_exact_centroids`: a conical wall's
    exact centroid, and an M-shaped fin against the shoelace centroid.
  - [L46](../decisions-and-roadmap.md#l46) `fin_tab_and_rail_button_mass_counted`.

[adr-006]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-006-component-geometry-and-mass-properties-frames-shapes-walls-fins-and-materials-2026-09-17
[levels]: ../accuracy.md#four-kinds-of-evidence
