# Mass properties of components

## In short

- **What it models:** the mass, centre of mass and inertia of each part (tubes, rings, shoulders,
  fins, rail buttons, lugs, mass components, recovery gear), how they add up, and 49 built-in
  material densities.
- **Sources:** Meriam and Kraige's *Engineering Mechanics: Dynamics*, the *OpenRocket technical
  documentation* v13.05, Abbott and von Doenhoff's *Theory of Wing Sections*, Golub and Van
  Loan's *Matrix Computations*, and data sheets, specifications and handbooks for densities.
- **How well it is validated:** by analytic tests, the first of four [kinds of evidence][levels]:
  a cone, a tube, four fins and an off-axis payload agree with hand calculation to 1e-11, and fin
  cross-sections with exact numerical integration to 1e-13. Density unit conversions reproduce
  their sources, such as the *Wood Handbook*'s white ash at 678 kg/m³. Against
  [OpenRocket](../glossary.md#openrocket) 24.12, on the structure (the rocket without motors) of
  74 design files: the mass is within 1% on 61 and the centre of mass within 1% of the rocket's
  length on 62. Every file outside either shows a difference hpr names in a warning. The roll
  inertia is not explained yet (median 2.1% apart). Not compared with weighed parts or a real
  flight ([checked against OpenRocket](#checked-against-openrocket)).
- **What it leaves out:** fin fillets, the sliver between a flat fin root and the round tube, and
  the step ring at a nose shoulder. Parachutes weigh as flat circular canopies. Where a `.ork`
  file leaves something unsaid (a wall of no thickness, no material), hpr reads it as OpenRocket
  does, and two rules for overrides stay hpr's own, each measured
  ([below](#what-a-ork-leaves-unsaid-and-overrides)). Clusters, fillets, airfoil fins and roll
  inertia are the next roadmap step, [M2.2b2](../decisions-and-roadmap.md#m2-2b2); designs with
  parts hpr does not read (pods, parallel stages) differ too.

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
inside OpenRocket's program file (its Java *jar*). That is 74 files. Some hold the same design found
in two places (several private files are copies of OpenRocket's examples), so there are 54
different files by content. Mass and centre of mass agree closely on most, and every file outside
1% has a cause hpr already warns about. The roll inertia does not agree, and why is not known yet.
This was [M2.2a](../decisions-and-roadmap.md#m2-2a); [ADR-060][adr-060] records how it was decided.
The numbers below are from rerunning it after
[M2.2b1](../decisions-and-roadmap.md#m2-2b1), which settled two of its causes
([next section](#what-a-ork-leaves-unsaid-and-overrides)).

**What you can check yourself.** The private files are not public, so only counts come from them,
and a fresh clone cannot reproduce the 74-file table. It can check the probe tube and Loft's public
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
| mass | 54 of 74 | 61 of 74 | 0.002% |
| centre of mass (share of length) | 54 of 74 | 62 of 74 | 0.012% |
| pitch inertia | 34 of 74 | 53 of 74 | 0.11% |
| roll inertia | 11 of 74 | 29 of 74 | 2.1% |

Counting each file's content once, 46 of 54 are within 1% in mass and 47 of 54 in centre of mass.
Before [M2.2b1](../decisions-and-roadmap.md#m2-2b1) (reading what a `.ork` leaves unsaid), 57 of
74 files were within 1% in mass and 58 in centre of mass (median mass 0.020%).

**The 13 files outside a threshold** are 8 different files by content, each a different design.
Each has one or two of three causes, and
hpr already warns of every one when it reads the file. `cargo xtask ork` works the causes out from
those warnings, counts them by content as below, and fails if a file outside has none. One file has
two causes, so the last column adds to 9:

| cause | what hpr does | what OpenRocket does | files by content |
|---|---|---|---|
| a [cluster](../glossary.md#cluster) of motor tubes in the file | reads it as one motor tube, with a warning | counts every tube | 2 |
| fin fillets (the rounded glue joint along a fin's root) | leaves them out, with a warning | counts them | 2 |
| pods, parallel stages, tube fins and parts left out | keeps them unread (the design is [reduced](../format/ork.md#what-hpr-keeps-for-writing-the-file-back)) | counts them | 5 |

`cargo xtask ork` prints each of the 13 with the parts that differ most, by id, or by name in an
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
  lighter, with no warning.

**Roll and pitch inertia.** On the six Loft demo designs OpenRocket opens, the roll inertia is 1.2%
to 3.8% apart, though their mass, centre of mass and pitch inertia agree within 0.1% and every part
of each is within 0.3 g of OpenRocket's. So it is none of the causes above. Across all 74 files
the median is 2.1%, and only 29 are within 1%. A tube's roll inertia matches (the probe), and so do
a nose cone's, a shoulder's and a transition's (the probes of the next section, within 0.001%), so
fins and the parts inside are the first place to look;
[M2.2b2](../decisions-and-roadmap.md#m2-2b2) takes it up. The pitch inertia is within 1% on 53 of 74. No bound is known for either yet.

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
`validation/oracles/openrocket/conventions.py` writes 32 small *probe designs*, each a rocket of
one or two parts built to ask one question, runs OpenRocket 24.12 on them and records its answers.
The test module `hpr_validate::openrocket::tests` reads the same designs with hpr and holds hpr to
them. Where hpr keeps a rule of its own, the test pins how far apart the two are. This was
[M2.2b1](../decisions-and-roadmap.md#m2-2b1); [ADR-061][adr-061] records the decisions.

How far to trust it: each probe asks about one kind of part at one size, so each reading is
measured, not proven for every case. OpenRocket's defaults were read with its preferences as a
fresh install sets them; an OpenRocket whose preferences were changed may give others.

**Read as OpenRocket reads it.** On every one of these probes hpr's mass is OpenRocket's within
0.001%, part by part as well as whole (the worst, a transition, is 0.0003% apart), and its centre
of mass within 0.001 mm. Where no fin, rail button or recovery part is in the probe, the inertias
agree within 0.001% too. Two gaps are pinned rather than hidden, and described below: a rail
button sits 5 mm further aft, and an elliptical fin set weighs 0.18% more. None of these readings
raises a warning, since nothing is assumed:

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

**Two gaps the probes found.** Both are for [M2.2b2](../decisions-and-roadmap.md#m2-2b2), and a
test pins each.

- OpenRocket puts a rail button's centre at the position the file gives. hpr places a button by
  its forward edge, so a button read from a `.ork` sits one radius further aft: 5 mm for a 10 mm
  button (issue [#151](https://github.com/nrdptel/hpr-sim/issues/151)).
- hpr's elliptical fin is the exact ellipse, with an area of `π c h / 4` for a root chord `c` and
  a span `h`. OpenRocket's weighs 0.18% less on the probe, as a many-sided polygon drawn inside the
  ellipse would.

**What it leaves out.** An inner tube, coupler or lug that writes no thickness at all is read as
no wall, with a warning. OpenRocket gives it a wall of its own: on the probe, 0.5 mm for a 20 mm
inner tube, 1 mm for a 5 mm lug, and none for a coupler. One size each does not say whether that
wall follows the radius, and no file in the reference library has one, so hpr does not follow it
yet; the test pins the difference.

**Run it yourself.** With Java 17 and the OpenRocket jar (`cargo xtask refs fetch`), from the
repository root:

```sh
refs/venv/bin/python validation/oracles/openrocket/conventions.py \
    validation/fixtures/ork/openrocket-conventions.json
cargo test -p hpr-validate openrocket
```

[adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21

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
