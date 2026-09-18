# Aerodynamics

## In short

- **What it models:** the air's forces on a rocket below [Mach](../glossary.md#mach-number) 1: the
  [normal force](../glossary.md#normal-force) (the sideways push when flying at an angle to the
  airflow), the [centre of pressure](../glossary.md#centre-of-pressure-cp) (where it acts) and
  drag.
- **Sources:** Barrowman's 1966 report, 1967 thesis and Centuri TIR-33 (1970), the basis of
  [Barrowman's method](../glossary.md#barrowmans-method); for drag, mainly Niskanen's 2009
  OpenRocket thesis.
- **How well it is validated:** the normal force and centre of pressure only at Mach 0, against
  Barrowman's worked examples (rockets he calculated by hand): every centre of pressure agrees
  within 1%, and so does every [normal-force slope](../glossary.md#normal-force-slope) but his
  six-fin Recruiter's: +2.87% high for the rocket and +3.42% for its fins, mostly from a different
  six-fin rule.
  Drag only at Mach 0.3, against curves labelled [RASAero](../glossary.md#rasaero-ii) in
  [RocketPy](../glossary.md#rocketpy)'s [example rockets](../glossary.md#example-rockets), which
  don't record their fins or surface finish, so hpr's follow a declared rule: within 10% in four
  of seven cases, and −18.3% for Cavour [under power](../glossary.md#power-on-and-power-off-drag)
  (motor burning), cause open. Valetudo's −47.0% and −50.4% are against a table 1.44 times its own
  [OpenRocket](../glossary.md#openrocket) export; hpr is 23.5% under that export as designed here,
  and 1.9% under it with the export's own finish and launch lugs. In whole flights in wind, body
  lift, which RocketPy leaves out, is the largest reason a slow rocket's drift differs from
  RocketPy's ([ADR-026][adr-026]). Nothing against a real flight.
- **What it leaves out:** large angles and [stall](../glossary.md#stall), though a flight uses
  these models at every angle. Nose and shoulder pressure drag is held at its low-speed value, so
  from about Mach 0.6 it reads low against the source's own high-subsonic correction; the models
  are documented to Mach 0.8 and refuse Mach 1 until [M1.8](../decisions-and-roadmap.md#m1-8)
  ([transonic and supersonic](../glossary.md#transonic-and-supersonic) aerodynamics), which also
  brings damping coefficients for pitch, yaw and roll, and roll forcing (the torque from fins set
  at an angle that spins a rocket up).

## Code and sources

Code: [`hpr_aero::body`](../api/hpr_aero/body/index.html) (bodies of revolution),
[`hpr_aero::fins`](../api/hpr_aero/fins/index.html) (fin sets) and
[`hpr_aero::model`](../api/hpr_aero/model/index.html) (a whole rocket's terms, built from its
[`Layout`](../api/hpr_design/tree/struct.Layout.html)). Decisions: [ADR-008][adr-008] (normal force
and centre of pressure) and [ADR-009][adr-009] (drag). The milestone [M1.5a](../decisions-and-roadmap.md#m1-5a) covers the
subsonic normal force and centre of pressure, [M1.5b](../decisions-and-roadmap.md#m1-5b) the subsonic drag and override
tables; transonic and supersonic flow arrive with [M1.8](../decisions-and-roadmap.md#m1-8), the supersonic aerodynamics
milestone.

A [Loft lesson](../glossary.md#loft-lesson) is something learned from Loft, the project that came
before hpr-sim: usually a mistake it made, sometimes a check worth keeping. This page names the
ones that concern aerodynamics, and the test here that covers each.

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

An aerodynamic coefficient is a force divided by the
[dynamic pressure](../glossary.md#dynamic-pressure) `q` (the pressure of the oncoming air,
`½ ρ V²`, with `ρ` the air's density and `V` the airspeed) and by the reference area. It has no
units, so the same number describes a small rocket and a large one of the same shape. These are
the symbols the whole page uses; each section defines its own as well.

| symbol | meaning | unit |
|---|---|---|
| `d_ref`, `A_ref` | the reference diameter, and the [reference area](../glossary.md#reference-area) `A_ref = π d_ref²/4` that every coefficient here is divided by | m, m² |
| station, `X` | a [station](../glossary.md#station): a position along the rocket, in metres aft of the nose tip ([Frames](frames.md), [Design tree](design.md)) | m |
| `x_B`, `y_B`, `z_B` | the axes of the [body frame](../glossary.md#body-frame), fixed to the rocket: `z_B` along its axis toward the nose, `x_B` the direction around the body that fins are placed from, and `y_B` square to both | none |
| `M` | the [Mach number](../glossary.md#mach-number): airspeed divided by the speed of sound | none |
| `α` | the total [angle of attack](../glossary.md#angle-of-attack): the angle between the nose direction `+z_B` and the rocket's velocity relative to the air, from 0 to π ([Frames](frames.md#aerodynamic-angles)). The oncoming air flows the opposite way, so at `α = 0` it meets the nose head-on | rad |
| `φ` | the flow roll: which way around the body the air crosses it, measured from `x_B` toward `y_B` ([Frames](frames.md#aerodynamic-angles)) | rad |
| `C_N` | the coefficient of the [normal force](../glossary.md#normal-force): the sideways push, square to the axis, in the plane that holds the axis and the airflow | none |
| `C_Nα` | the [normal-force slope](../glossary.md#normal-force-slope): how fast `C_N` grows with `α`. It is `C_N/α` for `α > 0`, and the derivative `∂C_N/∂α` at `α = 0` ([N09] eq. 3.8) | per rad |
| `C_Y` | the side-force coefficient: a sideways push across that plane, along `z_B` × the direction the air crosses in. Only sets of one or two fins produce it | none |
| CP | the [centre of pressure](../glossary.md#centre-of-pressure-cp): the station where the normal force acts | m |

The rocket's CP is the average of its components' CPs `X_i`, each weighted by that component's
slope: `X = Σ C_Nα,i X_i / Σ C_Nα,i` ([B66] p. 38; [N09] eq. 3.29). The components `i` are the
nose cones, transitions, body tubes and fin sets; a body tube's own slope is 0 (*Bodies of
revolution*, below). A rocket whose slopes add up to zero has no CP.

In code, these are the names the page uses:

| name | what it is |
|---|---|
| [`Layout`](../api/hpr_design/tree/struct.Layout.html) | a design resolved into placed parts, from [`Rocket::layout`](../api/hpr_design/tree/struct.Rocket.html#method.layout); it holds the reference diameter ([`reference_diameter_m`](../api/hpr_design/tree/struct.Layout.html#structfield.reference_diameter_m)) |
| [`AeroModel`](../api/hpr_aero/model/struct.AeroModel.html) | a rocket's aerodynamic terms, built from its `Layout` |
| [`Flow`](../api/hpr_aero/model/struct.Flow.html) | the conditions a model is evaluated at: `M`, `α` and `φ` |
| [`NormalForce`](../api/hpr_aero/model/struct.NormalForce.html) | the result, for the whole rocket or one component: `C_N` (`coefficient`), `C_Nα` (`slope_per_rad`), the CP (`cp_station_m`, `None` when there is none) and `C_Y` (`side_coefficient`) |
| [`NormalForce::moment_m`](../api/hpr_aero/model/struct.NormalForce.html#structfield.moment_m) | the normal force's turning effect about the nose tip, divided by `q` and `A_ref`: `Σ C_N,i X_i`, in metres. At an angle of attack the CP is `moment_m / C_N`; unlike the CP, `moment_m` is defined even when the forces cancel |

## Your rocket's centre of pressure

hpr works out the CP from the rocket's shape alone, the way
[Barrowman's method](../glossary.md#barrowmans-method) does by hand. Each nose, transition and fin
set gets its own slope and CP (the next two sections), and the rocket's CP is their weighted
average, as above. [Your own rocket](../your-own-rocket.md) prints the CP, the centre of gravity
and the stability margin of an example rocket.

To get it in code:

1. Take the rocket's `AeroModel` from a simulation with
   [`Simulation::aero`](../api/hpr_sim/flight/struct.Simulation.html#method.aero), or build one
   from a design with `Rocket::layout` and [`AeroModel::new`](../api/hpr_aero/model/struct.AeroModel.html#method.new).
2. Call [`AeroModel::normal_force`](../api/hpr_aero/model/struct.AeroModel.html#method.normal_force)
   with [`Flow::axial`](../api/hpr_aero/model/struct.Flow.html#method.axial)`(mach)`: the air
   straight along the axis, at a Mach number below 1. The result's
   [`cp_station_m`](../api/hpr_aero/model/struct.NormalForce.html#structfield.cp_station_m) is the
   CP, in metres aft of the nose tip, and its `slope_per_rad` is the rocket's `C_Nα`.
3. [`AeroModel::components`](../api/hpr_aero/model/struct.AeroModel.html#method.components), at
   the same flow, lists each component's share, which shows what moves the CP.

What changes it:

- **Speed.** Only the fins' slope changes with Mach number. It grows toward Mach 1 through the
  Prandtl–Glauert factor, the classic correction for the air's compressibility, whose effect grows
  as the speed nears that of sound (*Prandtl–Glauert*, under Fins). How much a fin set gains
  depends on its span, area and sweep. So as the rocket speeds up, the CP moves toward its fins:
  - With fins only at the tail, it moves aft.
  - With canards (a second fin set near the nose) as well, both sets gain, and the CP can move
    either way, depending on each set's shape and place.
  - hpr keeps each fin set's own CP a quarter of the way along its
    [mean aerodynamic chord](#fins) (MAC, a weighted average of its chords) at every speed below
    Mach 1. Niskanen's thesis moves it further aft above about Mach 0.5, which hpr leaves out
    until [M1.8](../decisions-and-roadmap.md#m1-8), the transonic and supersonic aerodynamics
    milestone (*Validity and open questions*, below, gives its size).

  `Flow::axial(0.0)` gives the low-speed CP that Barrowman's method gives by hand.
- **Angle.** `Flow::axial` gives the small-angle CP. At an angle of attack, body lift adds a force
  at each body's side-view centroid, the centre of its outline seen from the side
  (*Bodies of revolution*, below), and the CP moves with it.
- **Stability.** A rocket is statically stable when its CP is aft of its
  [centre of gravity](../glossary.md#centre-of-gravity-cg) (CG).
  - [`Assembly::mass_properties`](../api/hpr_design/config/struct.Assembly.html#method.mass_properties)
    gives the rocket's mass, CG and inertia `t` seconds after ignition. A simulation's assembly
    comes from [`Simulation::assembly`](../api/hpr_sim/flight/struct.Simulation.html#method.assembly).
  - Its [`cg_m`](../api/hpr_design/mass/struct.MassProperties.html#structfield.cg_m) is the CG in
    body axes, so the CG's station is `−cg_m.z`.
  - The CP's station less the CG's, divided by `d_ref`, is the
    [stability margin](../glossary.md#stability-margin) in [calibres](../glossary.md#calibre-caliber).
  - hpr doesn't report the margin yet: a margin over the flight comes with [M1.10](../decisions-and-roadmap.md#m1-10), the
    outputs milestone.

## Bodies of revolution

Nose cones, transitions and body tubes, from the outer profile. A shoulder (the sleeve of a nose or
transition that slides into the next tube) is inside the body and adds nothing. For one component:

| symbol | meaning | unit |
|---|---|---|
| `l` | its length | m |
| `A(x)` | its cross-section area `x` aft of its fore end; `A(0)` at the fore end, `A(l)` at the aft end | m² |
| `V` | its volume | m³ |
| `X_B` | its CP, aft of its own fore end (not the body axis `x_B`) | m |
| `A_plan` | its planform area: the area of its outline seen from the side. Body lift acts at its centroid, the centre of that area | m² |
| `K` | the body-lift constant, 1.1 ([`BODY_LIFT_K`](../api/hpr_aero/body/constant.BODY_LIFT_K.html)) | none |

Body lift is the extra push of the air crossing the body at larger angles of attack. It grows with
`sin² α`, so it is zero at `α = 0` and adds nothing to the slope there.

| term | formula | source |
|---|---|---|
| slope | `(C_Nα)_B = (2/A_ref)[A(l) − A(0)] · sin α/α` | [B66] eq. 10, [B67] eq. 3-65, [N09] eq. 3.19 |
| CP, aft of the fore end | `X_B = [l A(l) − V] / [A(l) − A(0)]` | [B66] eq. 28, [B67] eq. 3-89, [N09] eq. 3.28 |
| moment slope | `(2/A_ref)[l A(l) − V] · sin α/α` | [N09] eq. 3.25 |
| body lift | `C_N = K (A_plan/A_ref) sin² α`, `K = 1.1`, at the planform centroid | [G] p. 1, [N09] eq. 3.26–3.27 |

- A nose with a sharp tip has slope 2. A cylinder has 0 and no CP. A boattail has a negative
  slope, and the CP formula for a frustum (a cone with its tip cut off), [B66] eq. 44, still holds
  ([B66] p. 21).
- **Radius steps (an extrapolation).** A step where one body component meets the next adds
  `(2/A_ref)ΔA` at the joint, the limit of a transition whose length goes to zero, so the body's
  total slope is [B66] eq. 10 over the whole body. [B67] p. 18 assumes no discontinuities, so this
  goes beyond the source; leaving the step out would silently drop its slope (a 27 mm nose base on
  a 29 mm tube loses 13%). It is reported with the aft component (`BodyAero::step_area_m2`). A
  blunt front face gets no term, as eq. 10 gives. The design checks warn about steps
  (`radius_step`); the real flow separates there, and the drag buildup counts it as a zero-length
  shoulder or boattail (*Steps in radius*, under Drag).
- `V` and the planform come from integrating the real profile (`hpr_design::revolve`), so ogive,
  power, parabolic and Haack transitions ([Shapes](shapes.md#profiles)) get their own CP
  ([Loft lesson L9](../decisions-and-roadmap.md#l9): Loft used the conical transition's CP formula for every shape).
  [B66] puts a tangent ogive nose's CP at 0.466 L instead, with `L` the nose's length: 0.2–0.9%
  different at [fineness](../glossary.md#fineness-ratio) 2.8–5.
- The body's slope has no Mach term: [B67] p. 18 leaves body compressibility out as a
  conservative choice, and [N09] p. 22 takes the body's normal force as the same at all speeds.
- `K` is uncertain: [G] cites Hoerner's 1.1 to 1.5, fitted 1.0 to his own data, and says 1.2
  suits large angles better. Body lift is zero at `α = 0`, so the worked examples don't test it.

## Fins

A fin set is `N` identical fins spaced evenly around a body tube. For one fin of the set:

| symbol | meaning | unit |
|---|---|---|
| `s` | span: the fin's height from the body surface to its tip | m |
| `y` | height above the root, from 0 to `s` | m |
| `c`, `c_r`, `c_t` | chord: the fin's length along the airflow at height `y`; at the root and at the tip | m |
| `x_LE`, `x_t` | how far the leading edge sits aft of the root's leading edge, at height `y`; at the tip (the sweep length) | m |
| `A_fin` | one fin's area, one side (written `A` inside the integrals) | m² |
| `Γ_c` | the mid-chord sweep: the angle by which the line joining the chords' midpoints leans aft from square to the body | rad |
| `β` | the Prandtl–Glauert factor `√(1 − M²)`, the classic correction for the air's compressibility below Mach 1: 1 at rest, falling to 0 at Mach 1 | none |
| `c̄`, `y_MAC`, `x_MAC,LE` | the mean aerodynamic chord (MAC), an average chord that weights long chords more; its height; and its leading edge, aft of the root's | m |
| `X_f` | the fin set's CP, aft of the root leading edge | m |
| `Λ_k` | the angle between fin `k` and the direction the air crosses in (set by the flow roll `φ`) | rad |
| `f_N` | the fin-count factor, for five to eight fins | none |
| `r_t` | the body tube's radius at the fins | m |
| `K_T(B)` | the interference factor: how much the body raises the fins' normal force | none |

| term | formula | source |
|---|---|---|
| one fin | `(C_Nα)₁ = 2π (s²/A_ref) / (1 + √(1 + (β s²/(A_fin cos Γ_c))²))`, `β = √(1 − M²)` | [B67] eq. 3-4–3-6, [N09] eq. 3.38–3.40 |
| mean aerodynamic chord (MAC) | `c̄ = (1/A)∫c² dy`, `y_MAC = (1/A)∫y c dy`, `x_MAC,LE = (1/A)∫x_LE c dy` | [N09] eq. 3.30–3.32 |
| CP, aft of the root leading edge | `X_f = x_MAC,LE + c̄/4` | [B66] eq. 76a, [N09] eq. 3.34 |
| N fins | `(C_Nα)₁ Σ sin² Λ_k · f_N` | [N09] eq. 3.51–3.53, [TD] eq. 3.54 |
| interference | `K_T(B) = 1 + r_t/(s + r_t)` | [B66] eq. 77, [N09] eq. 3.56 |

- **Trapezoids.** `tan Γ_c = (x_t + c_t/2 − c_r/2)/s`, and the closed forms give [B66] eq. 57 and
  76a exactly.
- **Ellipses** on the root chord. `Γ_c = 0`, `c̄ = 8c_r/(3π)`, `y_MAC = 4s/(3π)` and
  `X_f = (½ − 2/(3π)) c_r = 0.28779 c_r`. Loft replaced the ellipse with an equal-area trapezoid,
  whose sweep made the slope 1.3% low ([Loft lesson L10](../decisions-and-roadmap.md#l10)).
- **Freeform outlines.** `c(y)` runs from the leading edge to the trailing edge, so a jagged edge's
  gap counts toward the CP but not toward `A_fin` ([N09] pp. 27–28). `Γ_c` is the span average of
  the mid-chord angle ([N09] p. 29), which gives the natural angle for trapezoids and ellipses. The
  integrals are exact: between vertex heights the edges are straight, and a three-point Gauss rule
  ([quadrature](quadrature.md), a weighted sum of samples) per band is exact. Bands thinner than
  1e-12 of the span (vertex heights a few rounding steps apart, as when a tip is converted from
  inches) are skipped.
- **Prandtl–Glauert** enters through `β` in the fin slope only. As `M → 1` the slope tends to
  `π s²/A_ref`. The CP stays at the quarter chord, a quarter of the way along the MAC, for all
  subsonic Mach ([B67] p. 6). Niskanen's aft shift above Mach 0.5 ([N09] eq. 3.35–3.36) moves to
  the planned transonic and supersonic milestone ([M1.8](../decisions-and-roadmap.md#m1-8)),
  together with the supersonic fit it interpolates to.
- **Fin count.** A fin at angle `Λ_k` to the lateral airflow adds `(C_Nα)₁ sin² Λ_k` in the plane of
  the flow. The sum is `N/2` for three or more evenly spaced fins, at any roll. `f_N` is 1 up to four
  fins, then 0.948, 0.913, 0.854 and 0.810 for five to eight ([TD] eq. 3.54). Those factors make
  six and eight fins 1.37 and 1.62 times four ([762] p. 5-24), and interpolate five and seven
  ([Loft lesson L8](../decisions-and-roadmap.md#l8)). More than eight fins are refused: [TD]'s 0.750 has no data behind
  it. [N09]'s roll-dependent 15% and 6% reductions for three and four fins were dropped in [TD].
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
    more even fins cancel it; one fin doesn't (roll arrives with [M1.8](../decisions-and-roadmap.md#m1-8), a planned
    aerodynamics milestone).
  - Interference between fin sets at the same station.
  - Cant (fins set at an angle to spin the rocket), which matters for roll ([M1.8](../decisions-and-roadmap.md#m1-8)).
  - Damping coefficients, for pitch, yaw and roll, and roll forcing from cant: all planned for
    [M1.8](../decisions-and-roadmap.md#m1-8). Pitch and yaw coefficients will have to replace the
    local-flow damping below, not add to it, or it would be counted twice. Until then:
    - In a flight, pitch and yaw damping come only from evaluating each component in its own
      local flow, which includes the speed the rocket's rotation adds there
      ([Rigid-body flight](flight.md#aerodynamics-in-flight)).
    - Only components with a slope give it: nose cones, transitions and fin sets. A boattail's
      slope is negative, so it takes some away.
    - Body tubes give none at small angles. Their own slope is 0, and their body lift grows with
      `sin² α`, so it adds nothing there.
    - Nothing aerodynamic damps or drives roll.
  - Tube fins, which are refused until a cited method exists
    ([issue #15](https://github.com/nrdptel/hpr-sim/issues/15)). Any part kind the model doesn't
    know is refused too.
  - Launch lugs and rail buttons add drag only.

## Drag

Code: [`hpr_aero::drag`](../api/hpr_aero/drag/index.html) (the terms),
[`AeroModel::drag`](../api/hpr_aero/model/struct.AeroModel.html#method.drag) and
[`AeroModel::buildup_components`](../api/hpr_aero/model/struct.AeroModel.html#method.buildup_components)
(their sum over a rocket), [`hpr_aero::table`](../api/hpr_aero/table/index.html) (override tables),
[`hpr_design::Finish`](../api/hpr_design/finish/enum.Finish.html) (roughness).
Decisions: [ADR-009][adr-009] (drag buildup, surface finishes and override tables). Extra sources:

- **[B67] ch. 4** (pp. 43–62): the friction, roughness and leading-edge formulas Niskanen adopts,
  and Table 4-1 of roughness heights (p. 46, after Hoerner p. 5-3).
- **[N09] §3.4** (pp. 41–53) and appendix B (pp. 106–110). [TD] reprints the same drag equations
  and tables unchanged.

On this page `C_D0` is the zero-lift [drag coefficient](../glossary.md#drag-coefficient): the
drag with the air straight along the axis (no angle of attack), divided by `q A_ref`.
[Recovery](recovery.md#drag-area) uses the same symbol for something else: a parachute canopy's
drag coefficient on its [nominal area](../glossary.md#nominal-area).

Drag is built up term by term. Skin friction acts over the whole surface. Pressure drag acts on
noses and shoulders (here, a transition that widens toward the tail) and on boattails (one that
narrows). Base drag acts on the flat aft end, fin pressure drag on the fins' edges, and parasitic
drag on launch lugs and rail buttons:
`C_D0 = C_D,friction + Σ_T (A_T/A_ref)(C_D•)_T` ([N09] eq. 3.75, 3.97), each pressure, base and
parasitic term on its own area. The axial coefficient is `C_A = C_D0 f(α)`.

| symbol | meaning | unit |
|---|---|---|
| `C_D0` | the zero-lift [drag coefficient](../glossary.md#drag-coefficient): the drag with the air straight along the axis, divided by `q A_ref` | none |
| `C_D,friction` | its skin-friction part | none |
| `A_T`, `(C_D•)_T` | the area one pressure, base or parasitic term `T` acts on (a nose's base, the fins' front edges), and its coefficient on that area | m², none |
| `C_A`, `f(α)` | the axial coefficient (the drag along the axis at an angle of attack), and the factor that turns `C_D0` into it | none |
| `R`, `V`, `L`, `ν` | the [Reynolds number](../glossary.md#reynolds-number), the airspeed, the rocket's length and the air's kinematic viscosity | none, m/s, m, m²/s |
| `R_s`, `R_crit` | the surface's roughness height, set by its finish; the Reynolds number above which roughness, not `R`, sets the friction | m, none |
| `C_f`, `C_fc` | the skin-friction coefficient, before and after its correction for Mach number | none |
| `f_B` | the body's [fineness ratio](../glossary.md#fineness-ratio): its length over its largest diameter | none |
| `A_body`, `A_fins` | the areas friction acts on (see *Friction on the axial projection*) | m² |
| `t` | a fin's thickness | m |
| `φ` (nose and shoulder row) | the joint angle between the surface and the axis where a nose or shoulder meets the next component: 0 for a smooth joint, `π/2` for a step. Not the flow roll | rad |
| `γ`, `l`, `d₁`, `d₂` | a boattail's length `l` over its drop in diameter, from `d₁` at its fore end to `d₂` at its aft end | none, m |
| `(C_D•)_base` | the base drag coefficient (the *base* row) | none |
| `q_stag/q` | the pressure rise where the air comes to rest on a blunt face, over `q`; 1 at low speed | none |
| `Γ_L` | a fin's leading-edge sweep | rad |
| `N t s` | the fins' frontal area: fin count × thickness × span | m² |
| `r_ext`, `r_int`, `l/d` | a launch lug's outer and inner radii, and its length over its outer diameter | m, none |
| square, rounded, airfoil | a fin's cross-section: constant thickness with square edges; semicircular leading and trailing edges; or a NACA four-digit symmetric airfoil | |

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
| angle of attack | `f = 1 + 0.3(3t² − 2t³)`, `t = α/17°`; `1.3(1 − 3u² + 2u³)`, `u = (α − 17°)/73°`; `−f(180° − α)` past 90° | | [N09] §3.4.7 (conditions only) |

- **Roughness.** `hpr_design::Finish` names the fifteen rows of [B67] Table 4-1 (0 to 1000 µm;
  [N09] Table 3.2 reprints ten) or takes a custom height. The default is "paint in aircraft mass
  production", 20 µm. Each component has its own finish; the Reynolds number and `R_s/L` use the
  whole rocket's length, as [N09] does ([Loft lesson L12](../decisions-and-roadmap.md#l12)). Loft cited none of its
  values: its 60 µm is OpenRocket's "regular paint" ([N09] p. 83), its 2 µm isn't in either table,
  and its `1 + 60/f³ + 0.0025f` is Raymer's aircraft fuselage form factor, not [N09]'s.
- **Fully turbulent.** The boundary layer (the thin layer of air the skin drags along) is taken as
  turbulent everywhere, never laminar (smooth and layered). [N09] p. 43 found laminar runs changed
  apogee by under 5% and dropped them.
  Eq. 3.81's `R < 1e4` branch applies first, even on surfaces rough enough that `R_crit < 1e4`.
- **Friction jumps where [N09] does.** Eq. 3.79 is not where eq. 3.78 and 3.80 cross, so eq. 3.81
  jumps at `R_crit`: +9% for 60 µm on a 1 m rocket (0.00419 to 0.00458). The subsonic and
  supersonic corrections also differ at Mach 1 (0.900 against 0.922 turbulent). hpr keeps the
  published forms, and the tests pin both jumps ([Loft lesson L90](../decisions-and-roadmap.md#l90)).
- **Friction on the axial projection (a departure).** Wall shear (the air's drag on the skin,
  `τ` per unit area) acts along the surface, so on an element of area `dA` at an angle `θ` to the
  axis its axial share is `τ cos θ dA`, and the body's friction area is `2π ∫ r dx = π A_plan`
  rather than the slant surface in [N09] eq. 3.85. On slender noses the difference is small: a tangent ogive
  loses 1.1% of its own friction area at fineness 3 and 2.4% at fineness 2. On a short, steep
  shoulder it removes friction on what is nearly a flat face, so a shoulder's drag tends to a bare
  step's as its length goes to zero ([Loft lesson L15](../decisions-and-roadmap.md#l15)); with the slant surface it would
  stay about `C_fc ΔA/A_ref` above it. The OpenRocket comparison ([M2.2](../decisions-and-roadmap.md#m2-2)) will measure the
  difference.
- **Steps in radius.** Where one body component meets the next with a different radius, a step up
  is a zero-length shoulder, `0.8 ΔA`, and a step down a zero-length boattail, the base drag of the
  uncovered area. A body with no nose cone gets `0.8 A` on its front face. Each is the limit of the
  transition it replaces ([Loft lesson L15](../decisions-and-roadmap.md#l15)), and it is reported with the aft component.
- **Boattails.** [N09] eq. 3.88 writes `A_base/A_boattail` without defining the areas, and p. 48
  says a zero-length boattail drags like "the total base drag". Taking `A_base` as the aft base
  would count that base twice and leave out the uncovered ring (annulus), so hpr reads both as the
  boattail's decrease in area (Calisto's boattail: 0.052, against 0.046 the other way). The joint
  angle is `atan(dr/dx)` at the aft end, `±π/2` where a curved transition ends in a blunt tip.
- **Base drag under power** subtracts the thrusting motors' cross-section from the aft base, down
  to zero ([N09] p. 50: "if the base is the same size as the motor itself, no base drag";
  [Loft lesson L13](../decisions-and-roadmap.md#l13)).
  `DragConditions::thrusting(reynolds_per_m, motor_area_m2)` takes the cross-section of the
  burning motors from the flight engine (zero when unknown: no relief). The base belongs to the
  last body component.
- **Fins.** Each fin set is its own term with its own thickness, chord and cross-section, so their
  order doesn't matter ([Loft lesson L11](../decisions-and-roadmap.md#l11)). `c̄` is the mean aerodynamic chord and `Γ_L`
  the leading-edge sweep: `atan(x_t/s)` for a trapezoid, the span average for freeform outlines
  ([N09] p. 50), and for an ellipse `π/2 − acos(k)/√(1 − k²)`, `k = c_r/(2s)` (the closed-form
  average, with its `acosh` form for `k > 1`). The drag goes as `cos² Γ`, whose span average is 6%
  lower than `cos²` of the average angle for an ellipse of `k = 1`. Fin–body interference drag and
  tip vortices are neglected ([N09] p. 41).
- **Launch lugs.** `d` in eq. 3.95–3.96 is taken as the outer diameter: [N09] p. 52 treats a solid
  rail pin as a lug "with a length equal to its diameter", which only reads that way
  ([Loft lesson L14](../decisions-and-roadmap.md#l14)). A row of `count` lugs is `count` lugs. **Rail buttons** follow
  [N09]'s rail-pin rule on their side profile (base and flange at the outer diameter, waist at the
  inner).
- **Angle of attack (derived coefficients).** [N09] §3.4.7 describes, without an equation, a
  two-part polynomial from 1 at 0° to 1.3 at 17° and 0 at 90°, with zero slope at each. hpr uses
  the unique cubic on each part that meets those four conditions. `C_A` is positive toward the
  tail. Past 90° the flow meets the tail, and hpr mirrors with the sign reversed, `−f(180° − α)`,
  an assumption that keeps drag opposing the motion. The planned OpenRocket comparison
  ([M2.2](../decisions-and-roadmap.md#m2-2)) will check it against OpenRocket, whose polynomial may differ.
- **Refusals, not clamps ([Loft lesson L16](../decisions-and-roadmap.md#l16)).** Geometry the terms can't use (a lug wall
  thicker than its radius, a button's base and flange taller than the button, a negative roughness,
  which `Rocket::layout` already refuses) is an error naming the component; a coasting condition
  (no motor burning) with a motor area and a non-finite result are errors; large coefficients are
  returned as they are.
- **Override tables** ([`DragTable`](../api/hpr_aero/table/struct.DragTable.html)) replace
  hpr's own `C_D0` with curves of `C_D0` against Mach number from another tool, power-off and
  power-on, read from CSV text: two columns, optionally under a header (RocketPy's curves; `\r\n`, a
  byte-order mark and `01.05` accepted), or a header naming the column, with rows at non-zero
  `Alpha` skipped (RASAero II's export: `Mach, Alpha, CD, CD Power-Off, CD Power-On, …`). An
  identical repeated row is skipped; a Mach number repeated with another value, or out of order, is
  refused with its line, not sorted. Tables interpolate linearly and
  hold their end values; `Drag::table` reports any extrapolation. `DragConditions::thrusting`
  selects the power-on curve. A table's `reference_diameter_m`, when set, rescales it to the
  rocket's reference area. The angle-of-attack factor still applies, and an override accepts any
  Mach number. `AeroModel::buildup_components` always reports the buildup, table or not.

### Drag limits

- The buildup refuses `M ≥ 1` until [M1.8](../decisions-and-roadmap.md#m1-8) (transonic and supersonic aerodynamics), like
  the normal force. The term functions are defined to any Mach number and stay finite to Mach 5
  (tested), for [M1.8](../decisions-and-roadmap.md#m1-8) to build on.
- **High subsonic drag is low; above Mach 0.8 it is flagged.** [N09] eq. 3.87 interpolates nose and
  shoulder pressure drag from its Mach 0 value (eq. 3.86) to appendix B's value and slope at Mach 1:
  closed forms for cones and ogives, Stoney's data (NASA TR-R-100) for other shapes. That arrives
  with [M1.8](../decisions-and-roadmap.md#m1-8); until then nose and shoulder pressure drag is held at its low-subsonic
  value, so it reads low from about Mach 0.6. How far `C_D0` falls short of eq. 3.87, by nose
  shape (3:1 is a [fineness](../glossary.md#fineness-ratio) of 3, three times as long as it is
  wide):

  | nose | Mach | `C_D0` reads low by |
  |---|---|---|
  | 3:1 tangent ogive | 0.7 | 0.006 |
  | 3:1 tangent ogive | 0.8 | 0.021, which is 4–5% of `C_D0` |
  | 2:1 cone | 0.8 | 0.037 |
  | 3:1 cone | 0.9 | about 0.05 |

  - hpr also holds the drag of a flat nose face or a step at its low-speed value, 0.80 on its own
    area; the source has it rising toward 1.04 as the speed rises.
  - No test computes these shortfalls, because eq. 3.87 isn't in the code yet. The drag decision
    ([ADR-009][adr-009]) records the ogive's and the 2:1 cone's.
  - `Drag::beyond_subsonic_methods` marks the top of [N09]'s subsonic region, Mach 0.8
    (Table 3.1), not the start of the error.
- Nothing models laminar flow, fin-tip vortices, interference drag, fin tabs, fillets, canted fins
  or the flow a boattail guides into the base ([N09] p. 51).

## Validity and open questions

- These are small-angle models. `α` is accepted over `[0, π]`, but fin slopes stay linear in `α`
  and nothing models stall. The flight engine uses them at every angle all the same
  ([Rigid-body flight](flight.md)), so its results are least trustworthy where large angles occur:
  off the rail in a strong crosswind, and near apogee.
- **Body lift in wind** (measured by flying both codes, not against a real flight;
  [ADR-026][adr-026]). A rocket that leaves the rail slowly into
  a crosswind meets the air at a steep angle. Juno III, one of RocketPy's example rockets, leaves
  at 18 m/s into an 8.5 m/s wind, 26° off the airflow, and there body lift is about half its normal
  force. Acting near the middle of the body, it pushes the rocket downwind with little turning,
  so hpr [turns into the wind](../glossary.md#weathercocking) less than RocketPy, whose normal
  force has no body term. Juno III's apogee ends 228.0 m from the pad in hpr and 396.6 m in
  RocketPy; body lift is about half of that difference, and hpr's rail release and fin slope most
  of the rest. `K` matters there: across [G]'s range, hpr's apogee drift runs from 191 m at
  `K = 1.5` to 237 m at 1.0, and would be 326 m with no body lift. Calisto, off the rail at 28 m/s
  and 11°, changes its drift by under 0.5% across that range. Which is nearer a real flight is
  open until [M2.3](../decisions-and-roadmap.md#m2-3).
- **No airfoils.** Fins use the flat-plate lift slope (2π per radian in two dimensions). An airfoil
  lift curve, such as the one Juno III's example gives its fins, is not modelled; RocketPy uses
  it, and its fin slope there is 7.6% steeper ([ADR-026][adr-026]).
- In one measured case, fins at `α = π/2` give `C_N` 17.4 against a flat-plate estimate near 5, and
  at `α = π` the fins still give 34.7 while every body term vanishes. That case is a 54 mm
  four-fin rocket at Mach 0.3.
- `M ≥ 1` is an error until [M1.8](../decisions-and-roadmap.md#m1-8) (transonic and supersonic aerodynamics), but the
  models are only documented to Mach 0.8.
  - [N09]'s subsonic range is 0–0.8, and [B67] p. 18 notes that `C_Nα` rises near Mach 1.
  - [N09] eq. 3.35–3.36 would move a fin set's CP aft, from 0.25 of the way along its mean
    aerodynamic chord (MAC, defined under *Fins*) to about 0.30 at Mach 0.8 and about 0.33 at 0.9,
    for fins of aspect ratio 1.6 (a measure of how long the span is against the chord). hpr keeps
    0.25.
  - Between 0.8 and 1, results are unvalidated extrapolations; [M1.8](../decisions-and-roadmap.md#m1-8) will replace them.

## Verification

- **Barrowman's worked examples** (`hpr_aero::tests::barrowman_worked_examples`). Inputs and
  printed results, with page numbers, are in the [fixture](../glossary.md#reference-value-and-fixture)
  [`validation/fixtures/aero/barrowman-worked-examples.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/barrowman-worked-examples.json).
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

  - **With hpr's own model, every CP agrees within 1%, and every slope but the Recruiter's.**
    Its six-fin slopes are +3.42% (fins) and +2.87% (total). Those are the only 2 of the 38
    printed values (19 slopes, 19 CPs) outside 1%, and the test pins that list.
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
- **[Loft lessons](../glossary.md#loft-lesson)**, each with what it concerns:
  - [L8](../decisions-and-roadmap.md#l8), no correction for five to eight fins:
    `fins::tests::six_fin_cna_applies_fin_count_factor`
  - [L9](../decisions-and-roadmap.md#l9), the conical transition's CP formula used for every shape:
    `body::tests::ogive_transition_cp_uses_volume_form`
  - [L10](../decisions-and-roadmap.md#l10), elliptical fins given a trapezoid's sweep:
    `fins::tests::elliptical_fin_cna_uses_zero_midchord_sweep`
  - [L89](../decisions-and-roadmap.md#l89), Barrowman's values worked by hand, a check kept from Loft's tests:
    `tests::barrowman_hand_values`. A cone's slope is 2 with its CP at 2L/3 (`L` its length);
    a conical transition from 20 to 40 mm radius over 0.1 m is 1.5 at 0.05556 m aft of its fore
    end; an elliptical fin's CP is 0.28779 `c_r` aft of its root leading edge.
- **Limits and invariants** (`body::tests`, `fins::tests`, `model::tests`):
  - Cylinders and thin transitions; body lift at 0 and 90°.
  - Eq. 57 and 76a closed forms against the same trapezoid as a polygon (1e-13).
  - A 2000-gon ellipse, and a jagged fin.
  - Prandtl–Glauert against [B67] eq. 3-6 written with the aspect ratio, and its `M → 1` limit.
  - Roll sums against direct sums; a two-fin set along and across the flow.
  - Mach changes only the fins.
  - A proptest (a rule checked on many random inputs): scaling every length leaves slopes
    unchanged and scales the CP; the reference diameter scales slopes only.
  - Refusals: tube fins, nine fins, Mach 1, angles out of range.

### Drag verification

- **RocketPy's drag curves at Mach 0.3** (`tests::rocketpy_drag_curves_at_mach_0_3`, fixture
  [`validation/fixtures/aero/rocketpy-drag-curves.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/rocketpy-drag-curves.json),
  written by `cargo xtask aero` from `refs/rocketpy`). Every RocketPy example whose curve is labelled RASAero, at sea level in the
  1976 [standard atmosphere](../glossary.md#standard-atmosphere) (USSA76; RASAero II computes its
  exports' Reynolds numbers there); [tolerance](../glossary.md#tolerance) 10%. The fixture holds only
  derived numbers: each curve's value at Mach 0.3, hpr's `C_D0` and the error. The test recomputes
  hpr's `C_D0` and the errors from the committed designs, and `cargo test -p xtask` reruns the
  comparison when `refs/rocketpy` is present.

  | case | curve | hpr `C_D0` | error | range over inputs |
  |---|---|---|---|---|
  | Calisto, 2018 fins | RASAero II export, power-off | 0.3982 | +4.4% | −14.0% to +12.8% |
  | Calisto, getting-started fins (variant) | the same | 0.3537 | −7.3% | −12.9% to +19.3% |
  | Juno III | labelled RASAero II, 3-decimal table | 0.3525 | −6.0% | −10.5% to +24.1% |
  | Cavour, power-off | labelled RASAero II, 3-decimal table | 0.5034 | −8.3% | −22.3% to −0.4% |
  | **Cavour, power-on (outside 10%)** | the same, power-on | 0.4487 | **−18.3%** | −32.2% to −10.3% |
  | **Valetudo, power-off (outside 10%)** | labelled RASAero, 3-decimal table | 0.5566 | **−47.0%** | −59.4% to −42.5% |
  | **Valetudo, power-on (outside 10%)** | the same, power-on | 0.5189 | **−50.4%** | −62.8% to −45.9% |

  - **Inputs ([ADR-009][adr-009], the drag decision).** The exports record none, so the designs
    follow one declared rule:
    - RASAero II's default smooth finish.
    - A NACA 00xx airfoil file in the example gives an airfoil section that thick at the mean
      aerodynamic chord (Calisto's getting-started fins).
    - A published section is used: Juno III's team placed second for a technical award, cited for
      "análise de aletas com perfil de aerofólio truncado" (an analysis of truncated-airfoil fins);
      taken as rounded at the placeholder thickness, since the citation gives no thickness.
    - Otherwise the placeholder, square 3 mm (Calisto's 2018 fins, Cavour, Valetudo).
    - Rail buttons are as RocketPy defines them (without them, Calisto is +1.8%).
  - **Sensitivity.** The range is over square, rounded and airfoil fins (3 mm, or 12% for the
    airfoil), 0 or 20 µm, and with or without rail buttons. Before the published-section rule, square
    fins gave Juno III +14.6% and the getting-started Calisto +10.7%. The check places hpr near
    RASAero's subsonic drag under a declared rule; without the inputs it can't show agreement to 10%.
  - **Power-on.** Separate power-on curves are compared (Cavour's and Valetudo's). Subtracting the
    motor's area ([N09] p. 50) removes 42% of Cavour's base drag and 29% of Valetudo's at Mach 0.3.
    Cavour's power-on table is within its 0.001 rounding of power-off from Mach 0.16 up (0.0001 at
    0.3) and 0.001 to 0.013 lower below; Valetudo's is 0.004 lower, about a ninth of hpr's relief.
    The designs' motor diameter is the larger of the grain and nozzle exit diameters, since RocketPy
    gives no case, and Cavour's result depends on it: −8.3% with no relief, −14.8% at 54 mm,
    −18.3% at the design's 67 mm nozzle exit, −20.8% with the example's 75 mm motor. The cause of
    that miss stays open: Niskanen's relief, a RASAero run with little or no nozzle exit diameter,
    or tables sampled along a flight (their uneven Mach spacing suggests it; unconfirmed).
  - **Valetudo.** Its table (1.05) is 1.44 times the OpenRocket export for the same rocket (0.728).
    With that file's own inputs (60 µm, two 14 mm × 30 mm lugs, 3 mm square fins), hpr gives
    0.714, 1.9% under the OpenRocket export and 32% under the table. As designed for this
    comparison, its 0.5566 is 23.5% under the export.
  - **Not compared.**
    - Calisto's power-on curve, which equals its power-off curve (no nozzle exit diameter in
      RASAero).
    - Juno III's power-on drag, which RocketPy takes from the same file.
    - Calisto's power-on result would be −5.0% (its power-on file is its power-off file).
    - The other examples, whose drag is a constant, CFD or of unknown origin.
- **[Loft lessons](../glossary.md#loft-lesson)**, each with what it concerns:
  - [L11](../decisions-and-roadmap.md#l11), drag that changed with the order of the fin sets:
    `drag::tests::drag_invariant_to_fin_set_order`
  - [L12](../decisions-and-roadmap.md#l12), uncited form-factor, friction and roughness constants:
    `drag::tests::form_factor_and_roughness_match_cited_values`
  - [L13](../decisions-and-roadmap.md#l13), no relief of base drag while a motor burns:
    `drag::tests::power_on_base_drag_subtracts_thrusting_motor_area`
  - [L14](../decisions-and-roadmap.md#l14), uncited launch-lug drag:
    `drag::tests::launch_lug_drag_matches_cited_hollow_tube_formula`
  - [L15](../decisions-and-roadmap.md#l15), shoulder drag that jumped as its length went to zero:
    `drag::tests::shoulder_drag_continuous_as_transition_length_tends_to_zero`
  - [L16](../decisions-and-roadmap.md#l16), a silent cap on the drag coefficient that hid bad geometry:
    `drag::tests::malformed_geometry_is_an_error_not_a_clamped_cd`
  - [L90](../decisions-and-roadmap.md#l90), drag invariants, a check kept from Loft's tests (skin friction's published
    jumps, split fin sets, base drag at Mach 1):
    `drag::tests::skin_friction_follows_eq_3_81_and_drag_invariants_hold`
- **Limits of every term** (`drag::tests`): friction below `1e4`, at `R_crit` and to Mach 5;
  stagnation pressure against the isentropic series and its limits either side of Mach 1; base drag
  at rest, at Mach 1 and far above; the joint term from smooth to a step; the boattail factor's
  three pieces and their joins; fin pressure drag by cross-section with the leading edge's joins at
  Mach 0.9 and 1 and the sweep; the angle-of-attack factor's stated values and zero slopes,
  monotonicity and its sign-reversed mirror (and `C_A < 0` at 135°); joint angles of cones and
  power-series, Haack and ogive noses; curved boattails ending in blunt tips; a tail closing to a
  point; leading-edge sweeps of trapezoids, kinked outlines and ellipses (against a quadrature to
  1e-8); a whole rocket's buildup written out by hand to 1e-12, its component sum, the Mach 0.8
  flag, and override tables (rescaled to another reference diameter).
- **Tables** (`table::tests`): RocketPy's quirks (`\r\n`, `01.05`, a repeated row), a byte-order
  mark, quoted fields and trailing commas, RASAero II's header with rows at 2° and 4° skipped, and
  malformed text (a bad first row, repeated or unsorted Mach numbers, `nan`) by line.

[adr-008]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-008-subsonic-normal-force-and-centre-of-pressure-2026-09-17
[adr-026]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18
[adr-009]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17
