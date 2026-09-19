# Aerodynamics

## In short

- **What it models:** the air's forces on a rocket: the
  [normal force](../glossary.md#normal-force) (the sideways push when flying at an angle to the
  airflow) and the [centre of pressure](../glossary.md#centre-of-pressure-cp) (where it acts) from
  [Mach](../glossary.md#mach-number) 0 to 5, and drag over the same range.
- **Sources:** Barrowman's 1966 report, 1967 thesis and Centuri TIR-33 (1970), the basis of
  [Barrowman's method](../glossary.md#barrowmans-method); supersonic linear theory for fins past
  Mach 1; for drag, mainly Niskanen's 2009 OpenRocket thesis, with Stoney's 1961 NASA measurements
  of noses through Mach 1, and MIL-HDBK-762 (1990) for boattails faster than sound.
- **How well it is validated:**
  - *Faster than sound, drag is only partly validated, and it misses both ways.* It reads high
    against a wind tunnel, most of all for thin, sharp fins; the body alone reads a little low
    against a handbook's worked example; and a rocket with a short, steep boattail reads 5% to
    15% low against RASAero II. Treat a supersonic flight's drag, and its apogee, as rough. The
    bullets below give the numbers.
  - *A boattail's own drag faster than sound* against 58 readings of 20 measured boattails of 3°
    to 10°, Mach 1.2 to 3.12: −21.9% to +28.3%, within 0.0123. Through Mach 1 it reads low, and
    under Niskanen's subsonic rule long boattails get almost nothing. Steeper ones in a thick
    [boundary layer](../glossary.md#boundary-layer) read high: +26.4% to +54.2% for 16°. The drag
    of the base behind a boattail is within 0.0102 of 12 measured bases, which behind a small base
    can be 40% of its own drag ([Boattails faster than sound](#boattails-faster-than-sound)).
  - *Drag reads high against a wind tunnel.* Against NASA's wind-tunnel tests of the Arcas Robin
    sounding rocket, Mach 0.6 to 4.63, on the [forebody](../glossary.md#forebody) only (the models'
    bases sat on a [sting](../glossary.md#sting)), every reading is high and 2 of 44 are within
    10%, both at Mach 1.0 with the fins on. With the fins on, from Mach 1.5 up, hpr reads +39.4% to
    +154.0% high: the fins take a blunt edge's formula. With the fins off it reads +13.5% to
    +24.1% from Mach 1.5 and +12.0% to +54.1% below, most of it the models' 15° boattail, which
    hpr over-predicts in a [boundary layer](../glossary.md#boundary-layer) thicker than the
    boattail is deep. hpr's base drag
    behind a plain cylinder has been checked against no measurement faster than Mach 0.3
    ([Drag against the Arcas Robin wind tunnel](#drag-against-the-arcas-robin-wind-tunnel)).
  - *Drag reads low against [RASAero II](../glossary.md#rasaero-ii) past Mach 1.6*, missing
    [M1.8](../decisions-and-roadmap.md#m1-8)'s 10% there. The curves labelled RASAero in
    [RocketPy](../glossary.md#rocketpy)'s [example rockets](../glossary.md#example-rockets) don't
    record their fins or surface finish, so hpr uses stated guesses for them. Against Calisto's,
    the one real RASAero II export, hpr is within 10% at every Mach number up to 0.8, at 3 of the 7
    between, and at 8 of 17 from 1.2 to 2.0, where it reads −14.9% to −5.1%, lowest at Mach 2.
    Other plausible fins bring 14 to 17 of the 17 within 10%, though none puts every row of every
    band within it; before hpr modelled the boattail's wave drag it read
    −29.8% to −24.4% there
    ([Drag against RASAero II through Mach 2](#drag-against-rasaero-ii-through-mach-2)).
  - *Drag at Mach 0.3* against the same curves: four of seven cases within 10%. Cavour
    [under power](../glossary.md#power-on-and-power-off-drag) (motor burning) is −18.3%, cause
    open, and Valetudo's table is 1.44 times its own [OpenRocket](../glossary.md#openrocket)
    export.
  - *Against [MIL-HDBK-762](../glossary.md#mil-hdbk-762)'s worked example*, a rocket whose drag
    the U.S. Army's handbook calculates term by term with every input known, the fins left out:
    6 of 12 Mach numbers within 10%; hpr reads +12.3% to +31.9% from Mach 0.9 to 1.2 (the nose and
    the base) and −6.0% to −9.6% from Mach 1.6 (friction and the base)
    ([Drag against MIL-HDBK-762's sample calculation](#drag-against-mil-hdbk-762s-sample-calculation)).
  - *The normal force and centre of pressure* at Mach 0 against Barrowman's worked examples
    (rockets he calculated by hand), where every centre of pressure agrees within 1%, and so does
    every [normal-force slope](../glossary.md#normal-force-slope) but his six-fin Recruiter's:
    +2.87% high for the rocket and +3.42% for its fins, mostly from a different six-fin rule; and
    from Mach 0.6 to 4.63 against the same wind tunnel: from Mach 1.5 to 2.96 the slope within
    −13.4% to +3.3% and the centre of pressure within 0.42
    [calibres](../glossary.md#calibre-caliber); past Mach 3 the slope reads −17.2% to −25.0% (the
    body), and between Mach 0.8 and 1.2 both miss
    ([Normal force through Mach 1](#normal-force-through-mach-1)).
  - *In whole flights* in wind, body lift, which RocketPy leaves out, is the largest reason a slow
    rocket's drift differs from RocketPy's ([ADR-026][adr-026]). Nothing against a real flight.
- **What it leaves out:** large angles and [stall](../glossary.md#stall), though a flight uses
  these models at every angle. Faster than sound
  ([transonic and supersonic](../glossary.md#transonic-and-supersonic)), a steep boattail's drag
  in a thick boundary layer reads high, and nothing corrects for it; the fins' drag takes a
  blunt edge's formula, which reads far high for thin, sharp fins, and nothing models a thin fin's
  own wave drag or the drag where fins meet the body. The body's normal force faster than
  sound is slender-body theory's, which the wind tunnel shows low past Mach 3
  ([M1.8e](../decisions-and-roadmap.md#m1-8e)). Damping coefficients for pitch, yaw and roll, and
  roll forcing (the torque from fins set at an angle that spins a rocket up) come with
  [M1.8c](../decisions-and-roadmap.md#m1-8c).

## Code and sources

Code: [`hpr_aero::body`](../api/hpr_aero/body/index.html) (bodies of revolution),
[`hpr_aero::fins`](../api/hpr_aero/fins/index.html) (fin sets),
[`hpr_aero::nose_drag`](../api/hpr_aero/nose_drag/index.html) (noses' drag through Mach 1),
[`hpr_aero::afterbody`](../api/hpr_aero/afterbody/index.html) (boattails faster than sound) and
[`hpr_aero::model`](../api/hpr_aero/model/index.html) (a whole rocket's terms, built from its
[`Layout`](../api/hpr_design/tree/struct.Layout.html)). Decisions: [ADR-008][adr-008] (normal force
and centre of pressure) and [ADR-009][adr-009] (drag). The milestone [M1.5a](../decisions-and-roadmap.md#m1-5a) covers the
subsonic normal force and centre of pressure, [M1.5b](../decisions-and-roadmap.md#m1-5b) the subsonic drag and override
tables; [M1.8a](../decisions-and-roadmap.md#m1-8a) the normal force through Mach 1
([ADR-027][adr-027]); [M1.8b1](../decisions-and-roadmap.md#m1-8b1) the drag through Mach 1
([ADR-028][adr-028]); [M1.8b3](../decisions-and-roadmap.md#m1-8b3) boattails faster than sound
([ADR-030][adr-030]). The rest of transonic and supersonic flow arrives with the rest of
[M1.8](../decisions-and-roadmap.md#m1-8), the supersonic aerodynamics milestone.

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
- **[762]** MIL-HDBK-762(MI), *Design of Aerodynamically Stabilized Free Rockets*, 1990.
- **[TN2114]** S. M. Harmon and I. Jeffreys, *Theoretical Lift and Damping in Roll of Thin Wings
  with Arbitrary Sweep and Taper at Supersonic Speeds: Supersonic Leading and Trailing Edges*,
  NACA TN 2114, 1950.
- **[D4013]** J. C. Ferris, *Static Stability Investigation of a Single-Stage Sounding Rocket at
  Mach Numbers from 0.60 to 1.20*, NASA TN D-4013, 1967.
- **[D4014]** C. D. Babb and D. E. Fuller, *Static Stability Investigation of a Sounding-Rocket
  Vehicle at Mach Numbers from 1.50 to 4.63*, NASA TN D-4014, 1967.
- **[S61]** W. E. Stoney, *Collection of Zero-Lift Drag Data on Bodies of Revolution from
  Free-Flight Investigations*, NASA TR R-100, 1961 (`nasa-tr-r-100-stoney-1961`).
- **[J53]** J. R. Jack, *Theoretical Pressure Distributions and Wave Drags for Conical Boattails*,
  NACA TN 2972, 1953.
- **[CS51]** E. M. Cortright Jr. and A. H. Schroeder, *Investigation at Mach Number 1.91 of Side
  and Base Pressure Distributions over Conical Boattails without and with Jet Flow Issuing from
  Base*, NACA RM E51F26, 1951.
- **[DN54]** C. A. de Moraes and A. M. Nowitzky, *Experimental Effects of Propulsive Jets and
  Afterbody Configurations on the Zero-Lift Drag of Bodies of Revolution at a Mach Number of
  1.59*, NACA RM L54C16, 1954.
- **[MJ54]** B. Moskowitz and J. R. Jack, *Aerodynamics of Slender Bodies at Mach Number of 3.12
  … V: Aerodynamic Load Distributions for a Series of Four Boattailed Bodies*, NACA RM E54B11, 1954.
- **[C57]** J. M. Cubbage Jr., *Jet Effects on the Drag of Conical Afterbodies for Mach Numbers of
  0.6 to 1.28*, NACA RM L57B21, 1957.
- **[Love57]** E. S. Love, *Base Pressure at Supersonic Speeds on Two-Dimensional Airfoils and on
  Bodies of Revolution with and without Fins Having Turbulent Boundary Layers*, NACA TN 3819, 1957.
- **[C72]** W. B. Compton III, *Jet Effects on the Drag of Conical Afterbodies at Supersonic
  Speeds*, NASA TN D-6789, 1972.
- **[R1135]** Ames Research Staff, *Equations, Tables, and Charts for Compressible Flow*, NACA
  Report 1135, 1953.
- **[R22]** C. E. Rogers, *RASAero II Comparisons with ARCAS Center of Pressure (CP) and Drag
  Coefficient (CD) Wind Tunnel Data*, Rogers Aeroscience, 2022 (slides).

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
   straight along the axis, at a Mach number below 5. The result's
   [`cp_station_m`](../api/hpr_aero/model/struct.NormalForce.html#structfield.cp_station_m) is the
   CP, in metres aft of the nose tip, and its `slope_per_rad` is the rocket's `C_Nα`.
3. [`AeroModel::components`](../api/hpr_aero/model/struct.AeroModel.html#method.components), at
   the same flow, lists each component's share, which shows what moves the CP.

What changes it:

- **Speed.** Only the fins' terms change with Mach number; the bodies' don't. Up to Mach 0.8 the
  fins' slope grows through the Prandtl–Glauert factor, the classic correction for the air's
  compressibility, whose effect grows as the speed nears that of sound (*Prandtl–Glauert*, under
  Fins). How much a fin set gains depends on its span, area and sweep. So as the rocket speeds
  up, the CP moves toward its fins:
  - With fins only at the tail, it moves aft.
  - With canards (a second fin set near the nose) as well, both sets gain, and the CP can move
    either way, depending on each set's shape and place.
  - Up to Mach 0.8 hpr keeps each fin set's own CP a quarter of the way along its
    [mean aerodynamic chord](#fins) (MAC, a weighted average of its chords). From there it moves
    aft, and past Mach 1 the fins' slope falls again
    ([Fins through Mach 1](#fins-through-mach-1)), so a fast rocket's CP moves forward.

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
- **Prandtl–Glauert** enters through `β` in the fin slope only, up to Mach 0.8. The CP stays at
  the quarter chord, a quarter of the way along the MAC, through that range ([B67] p. 6). Past
  Mach 0.8 the fins follow [Fins through Mach 1](#fins-through-mach-1), below.
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

### Fins through Mach 1

Faster than sound, air can't flow around a fin's edges ahead of it. The fin's lift comes from the
pressure behind the shock and expansion waves at its surfaces, and a different theory applies.
hpr uses Barrowman's subsonic method to Mach 0.8, supersonic
[linear theory](../glossary.md#supersonic-linear-theory) from where that theory holds, and a
straight-line join between them. The body terms don't change with Mach:
[slender-body theory](../glossary.md#slender-body-theory)'s slope and CP hold at any speed
([B67] p. 18). How well this agrees with a wind tunnel and with RASAero II is under
[Verification](#normal-force-through-mach-1).

**Supersonic linear theory.** Past Mach 1, `β` is redefined as `√(M² − 1)`, which grows from 0 as
the speed passes that of sound. A thin flat plate at a small angle `α` to a supersonic flow has
the pressure coefficient `+2α/β` on the side facing the flow and `−2α/β` on the other: the first
term of the pressure series Barrowman uses ([B67] appendix A, p. 82). So every part of the fin
carries the same load, `4α/β` per unit area, and each strip (a narrow slice of the fin along the
airflow) carries it at its middle. MIL-HDBK-762 finds linearized theory accurate for the
supersonic stability of thin fins ([762] p. 5-15). Two things change the load near the tip:

- **The tip's [Mach cone](../glossary.md#mach-cone).** The tip disturbs the flow only inside the
  cone that spreads inboard from its leading edge at the Mach angle, `atan(1/β)`. Barrowman halves the load inside it
  ([B67] appendix A, p. 84). For a rectangular tip that is exactly linear theory's loss.
- **The body as a mirror.** At the root the body stands in for the fin's mirror image. A cone
  that reaches the root continues into the mirror image, and the part of it there counts too, as
  the mirror fin's cone crossing onto this one.

| term | formula | source |
|---|---|---|
| one fin | `(C_Nα)₁ = (4/β)(A_fin − A_cone/2)/A_ref`, `β = √(M² − 1)` | [B67] appendix A |
| CP, aft of the root leading edge | the centroid of that load: the fin's area centroid, less half the cone's | [B67] appendix A |
| rectangle, `AR = 2s/c` | `(4/β)(1 − 1/(2β·AR))` and `X_f/c = (β·AR − 2/3)/(2β·AR − 1)`, exact linear theory | [TN2114], [N09] eq. 3.35 |

`A_cone` is the part of the fin (and of its mirror image) inside the tip's Mach cone. The
fin-count factor, the sum over fins and `K_T(B)` apply as above.

**Where it starts.** Linear theory's strips need four things, so it starts at
`M_s = max(1.2, 1/cos Γ_L, 1/cos Γ_T, √(1 + 1/AR²), √(1 + (c_t/2s)²))`, where `Γ_L` and `Γ_T`
are the leading- and trailing-edge sweeps, `AR = 2s²/A_fin` is the aspect ratio of the fin and
its mirror image, and `c_t` the tip chord:

- Mach 1.2, the bottom of the supersonic region ([N09] Table 3.1, p. 19);
- supersonic edges, each with its Mach number square to the edge, `M cos Γ`, past 1, the case
  [TN2114] covers;
- `β·AR ≥ 1`, where linear theory's tip loss holds; a rectangle's slope peaks there, at `2·AR`;
- `β ≥ c_t/(2s)`, so the mirror fin's tip cone stays off this fin's tip, which matters for a tip
  chord longer than the fin's average.

Calisto's and the Arcas Robin's fins start at their leading edge's 1.2806 and at 1.2.

**The transonic join.** From Mach 0.8 to `M_s`, the slope and the CP are each a straight line in
`M` between their values at the two ends. No source gives this region in closed form. MIL-HDBK-762
reads it from charts of transonic similarity (the way thickness and Mach number combine near
Mach 1, [762] pp. 5-104–5-105). The join keeps both continuous. For most fins the slope peaks at
`M_s` (a leading edge swept forward can make it fall across the join instead), and the
CP moves aft from the quarter chord toward the middle of the chord.

**Worked example.** Calisto's 2018 fins: root chord 0.12 m, tip chord 0.04 m, span 0.10 m and
sweep length 0.08 m, on `A_ref = 0.012668` m² (`d_ref` = 0.127 m). The leading edge is swept
38.66°, so `M_s = 1/cos 38.66° = 1.2806`. At Mach 2, `β = 1.732`. The tip cone is a triangle
0.04 m along the tip and `0.04/β = 0.0231` m down the unswept trailing edge: 0.000462 m², 5.8% of
the fin's 0.008 m². So `(C_Nα)₁ = (4/1.732)(0.008 − 0.000231)/0.012668 = 1.416` per rad. hpr
gives, per fin:

| Mach | 0 | 0.8 | 1.0 | 1.2806 | 1.5 | 2.0 | 3.0 |
|---|---|---|---|---|---|---|---|
| `(C_Nα)₁`, per rad | 1.853 | 2.170 | 2.499 | 2.960 | 2.158 | 1.416 | 0.877 |
| CP aft of the root leading edge, m | 0.0550 | 0.0550 | 0.0632 | 0.0747 | 0.0753 | 0.0758 | 0.0761 |

**What it leaves out.**

- Thickness. Linear theory is for thin plates; a thick fin or a blunt leading edge detaches the
  bow shock near Mach 1.
- Exact linear theory for a tapered fin. The strip method's constant load outside the tip cone
  runs above it. Against [TN2114] eq. A7 (printed p. 18), a fin with a taper ratio of 0.5, an
  unswept trailing edge and `βA = 3` gets 4.5% more slope.
- Subsonic leading and trailing edges, which the join covers without a method of its own. A
  curved edge counts by its span-averaged sweep, so an elliptical fin, whose edge is swept 90° at
  the tip, or a freeform fin with a raked outboard edge, keeps a subsonic stretch past `M_s`.
- Leading edges swept forward. The tip then sits ahead of the root and its Mach cone covers much
  of the fin, where the half-load overstates the loss, so the slope rises past `M_s` instead of
  falling: by up to +7.7% for fins swept 40° to 50° forward, peaking up to 0.37 Mach later
  ([issue #64](https://github.com/nrdptel/hpr-sim/issues/64)). Such fins are rare on rockets. A
  property test holds every trapezoid whose leading edge is straight or swept aft (to 65°),
  tapered either way, to a slope that falls with Mach from `M_s`.
- The fins' lift carried onto the body behind them, `K_B(T)`, as below Mach 1.

**Other choices, and why not.**

- *Niskanen's supersonic slope* ([N09] eq. 3.48–3.49) multiplies the fin's area by one strip's
  pressure coefficient, `K₁α + …` with `K₁ = 2/β`: the pressure on one face. A plate is pushed by
  the difference between its faces, twice that. His thesis finds its simulated `C_Nα` for the
  Arcas Robin "notably lower than the experimental values", with the cause unknown (p. 91, a
  comparison that runs to Mach 4). hpr counts both faces.
- *RocketPy 1.13.0* flies Diederich's subsonic slope at every Mach number, with `β` held at 0.6
  from Mach 0.8 to 1.1; past Mach 1 that tends to `2π cos Γ_c/β`, about π/2 times linear theory's
  `4/β`. Its fin CP doesn't move with Mach.
- *Tuning the join to the wind tunnel* below would shrink its misses by fitting the model to its
  own check. The join's ends come from the sources' speed regions, set before measuring.

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
| nose, shoulder | `0.8 sin² φ` at rest, `φ` the joint angle at the aft end; through Mach 1 as under *Drag through Mach 1* | base area; increase in area | [N09] eq. 3.86–3.87, appendix B |
| boattail | to Mach 0.8, `(C_D•)_base` × 1 (`γ ≤ 1`), `(3 − γ)/2`, 0 (`γ ≥ 3`); `γ = l/(d₁ − d₂)`; faster, as under *Boattails faster than sound* | decrease in area | [N09] eq. 3.88; [762] Fig. 5-122 |
| base | `0.12 + 0.13 M²` below Mach 1, `0.25/M` above; behind a boattail, from Mach 0.8, times its base-pressure ratio (*Boattails faster than sound*) | aft base less thrusting motors | [N09] eq. 3.94, p. 50; [762] Fig. 5-141 |
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
  is a zero-length shoulder, a flat face (`0.8 ΔA` at rest, rising with Mach as under *Drag
  through Mach 1*), and a step down a zero-length boattail, the base drag of the uncovered area. A
  body with no nose cone gets the same flat face on its front. Each is the limit of the
  transition it replaces ([Loft lesson L15](../decisions-and-roadmap.md#l15)), and it is reported with the aft component.
  A step up just behind a step down, such as a motor retainer behind the step to the motor tube,
  is sheltered by the step's corner at every speed, by how far it rises and how much motor tube
  shows ahead of it, and not at all once that is as long as the step's drop in diameter. This is
  unmeasured; for a 98 mm airframe with 12 mm of a 54 mm motor tube showing and a 62 mm retainer
  it lowers the rocket's `C_D0` 12% to 23% from Mach 0.3 to 2.5
  ([Boattails faster than sound](#boattails-faster-than-sound)).
- **Boattails.** [N09] eq. 3.88 writes `A_base/A_boattail` without defining the areas, and p. 48
  says a zero-length boattail drags like "the total base drag". Taking `A_base` as the aft base
  would count that base twice and leave out the uncovered ring (annulus), so hpr reads both as the
  boattail's decrease in area (Calisto's boattail: 0.052, against 0.046 the other way). The joint
  angle is `atan(dr/dx)` at the aft end, `±π/2` where a curved transition ends in a blunt tip.
  A lip (a short step up or flare) just behind a boattail or a step down is in its wake at every
  speed, and from Mach 0.8 a boattail's drag rises to its supersonic wave drag
  ([Boattails faster than sound](#boattails-faster-than-sound)).
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

### Drag through Mach 1

Near the speed of sound a nose starts to push shock waves ahead of it, and the pressure on its
surface climbs: the transonic drag rise. Past Mach 1 this pressure drag, called
[wave drag](../glossary.md#wave-drag), settles
to a value set mostly by the nose's shape and how slender it is. hpr follows Niskanen's method
([N09] §3.4.3 and appendix B, pp. 47–48 and 106–110) for noses, shoulders and steps. The other
terms already had their faster-than-sound forms in the table above: friction's Mach correction,
base drag's `0.25/M`, the fins' leading and trailing edges, and the stagnation pressure on blunt
faces. The code is [`hpr_aero::nose_drag`](../api/hpr_aero/nose_drag/index.html); the decision
record is [ADR-028][adr-028]. How far it holds against a wind tunnel is under
[Verification](#drag-against-the-arcas-robin-wind-tunnel): it reads high except near Mach 1, most
of all with fins on past Mach 1.2.

A nose's or shoulder's pressure-drag coefficient, on the area it adds, has three parts:

- **At rest**, eq. 3.86's `0.8 sin² φ`, with `φ` the joint angle at the aft end (the table above).
  A step, or a bare front face, has no length, and takes the flat face's `0.85 q_stag/q` at every
  Mach number instead (below).
- **From `M_L`**, the Mach number where the transonic formula takes over (0.8 or 1 by shape, in
  the table below), a transonic and supersonic value `C_T(M)` that depends on the shape
  and the [fineness ratio](../glossary.md#fineness-ratio) `f = l/(d_aft − d_fore)`: a nose's
  length over its base diameter, and for
  a shoulder its length over its rise in diameter, so that a conical shoulder drags like the cone
  with the same surface angle.
- **Between rest and `M_L`**, eq. 3.87: `a Mᵇ + 0.8 sin² φ`, with `a` and `b` chosen so the curve
  meets `C_T` and its slope at `M_L`: `b = C_T′(M_L) M_L/Δ` and `a = Δ/M_Lᵇ`, where
  `Δ = C_T(M_L) − 0.8 sin² φ`. Niskanen asks for a curve that doesn't fall and is flat at rest,
  which needs `Δ > 0` and `b > 1`; otherwise hpr uses a quadratic (below). With `b` near 10, as
  for slender cones, the curve stays close to its value at rest until about Mach 0.8.

| shape | `C_T(M)` | `M_L` | source |
|---|---|---|---|
| step up in radius, or a bare front face | a flat face: `0.85 q_stag/q`, at every Mach number | — | [N09] eq. B.1–B.2 |
| cone | `sin ε` at Mach 1 with slope `4/(γ + 1)(1 − sin ε/2)`; `2.1 sin² ε + 0.5 sin ε/√(M² − 1)` from Mach 1.3; a cubic between, meeting both ends' values and slopes; `tan ε = 1/(2f)` | 1 | [N09] eq. B.3–B.6 |
| ogive | the cone of the same length and diameter, times `0.72 (κ − ½)² + 0.82`, `κ` the tangent ogive's arc radius over this one's (0 for a cone, 1 for a tangent ogive) | 1 | [N09] eq. B.8 |
| elliptical, power series, parabolic series, Haack series | Stoney's measured curve at fineness 3, `C₃(M)`, scaled to the nose's fineness by `C₀ (C₃/C₀)^log₄(f + 1)`, with `C₀` the flat face's `0.85 q_stag/q` | 0.8 | [N09] eq. B.7, B.9; [S61] Fig. 12 |

Here `γ = 1.4` is the ratio of specific heats of air, `q_stag/q` the stagnation-pressure ratio of
the table above, and the fineness scaling is the curve `a/(f + 1)ᵇ` through a flat face at
fineness 0 and the measured nose at fineness 3 (eq. B.7). Stoney's report suggested that fineness
and Mach number act separately ([N09] p. 108).

**Stoney's curves.** Stoney's 1961 NASA report collected the drag of about 200 bodies flown on
rockets at NASA Langley ([S61]). Its Figure 12 plots the pressure drag of noses of fineness 3
against Mach number: panel (a) from flight models, Mach 0.8 to 2.0, and panel (b) from a wind
tunnel (his ref. 30), to Mach 3.6. No table prints them, so hpr carries them as points read off a
600-dpi scan of the figure, each panel's grid calibrated where the curve runs, to about ±0.0015.
hpr takes panel (a) for the seven shapes it has, and panel (b) for the x^¼ and the ellipsoid,
which only it has. Where (a) and (b) overlap, (b)'s von Kármán reads 0.004 to 0.011 higher from
Mach 1.2. Past a curve's last point hpr holds its last value. Panel (b) checks two of those holds:
its von Kármán reads 0.079 to 0.086 from Mach 2.4 to 3.59, against panel (a)'s held 0.079, and its
x^¾ falls to 0.073 by Mach 3.2, 8% under the held 0.079; panel (a)'s x^½ is still rising at its
end. The x^¼ and the ellipsoid, which panel (b) starts at Mach 1.2, are joined by a straight line
to 0 at Mach 0.8, where every smooth 3:1 nose of panel (a) reads 0; so every measured shape starts
at Mach 0.8. The points and where each was read are in the code
([`StoneyNose`](../api/hpr_aero/nose_drag/enum.StoneyNose.html)). A sample, on the nose's base
area (panel (a)'s values at Mach 3.0 are its held end values):

| shape | Fig. 12 panel, Stoney's model number | Mach 0.9 | Mach 1.0 | Mach 1.2 | Mach 1.5 | Mach 2.0 | Mach 3.0 |
|---|---|---|---|---|---|---|---|
| von Kármán | (a), 58 | 0.000 | 0.025 | 0.076 | 0.089 | 0.079 | 0.079 |
| L-V Haack | (a), 60 | 0.000 | 0.025 | 0.100 | 0.116 | 0.112 | 0.112 |
| parabola | (a), 59 | 0.000 | 0.037 | 0.116 | 0.108 | 0.107 | 0.107 |
| ¾ parabola | (a), 62 | 0.000 | 0.069 | 0.104 | 0.082 | 0.081 | 0.081 |
| ½ parabola | (a), 57 | 0.015 | 0.094 | 0.114 | 0.088 | 0.086 | 0.086 |
| x^¾ | (a), 61 | 0.013 | 0.085 | 0.109 | 0.093 | 0.079 | 0.079 |
| x^½ | (a), 63 | 0.000 | 0.046 | 0.080 | 0.086 | 0.090 | 0.090 |
| x^¼ | (b) | — | — | 0.141 | 0.181 | 0.216 | 0.246 |
| ellipsoid | (b) | — | — | 0.111 | 0.151 | 0.158 | 0.160 |

A shape between two measured ones interpolates between their curves in its parameter (the
exponent `n`, `K′` or `C` of [Shapes](shapes.md#profiles)), before the fineness scaling ([N09]
p. 108): a power series `xⁿ` runs through a flat face (`n = 0`), x^¼, x^½,
x^¾ and the 3:1 cone (`n = 1`); a parabolic series through the 3:1 cone (`K′ = 0`) and the ½, ¾
and full parabolas; a Haack series between von Kármán (`C = 0`) and L-V Haack (`C = ⅓`).

**Worked example.** A 5:1 von Kármán nose at Mach 1.5. Stoney's 3:1 von Kármán gives 0.0893. The
flat face gives `0.85 q_stag/q = 0.85 × 1.5381 = 1.3074`, with
`q_stag/q = 1.84 − 0.76/1.5² + 0.166/1.5⁴ + 0.035/1.5⁶`. The exponent is `log₄ 6 = 1.2925`, so
the nose drags
`1.3074 × (0.0893/1.3074)^1.2925 = 0.0407` on its base area. A 5:1 cone drags 0.0653 there, from
eq. B.4, so the von Kármán's wave drag is 38% lower. The test
`nose_drag::tests::the_guides_worked_example` pins these numbers.

**Where hpr departs from, or adds to, the source** ([ADR-028][adr-028]):

- **Short cones and ogives.** Below fineness 1 the cone formula runs past a flat face's drag: as
  the cone flattens, eq. B.4 tends to 2.39 at Mach 2 against the flat face's 1.41. So below
  fineness 1 hpr scales, at every Mach number, between a flat face at fineness 0 and the whole
  curve of the cone at fineness 1, as eq. B.9 scales the measured shapes. A shoulder then tends to
  a bare step as it shortens ([Loft lesson L15](../decisions-and-roadmap.md#l15)), and above
  fineness 1 Niskanen's cone is unchanged.
- **Steps.** A step up in radius, or a body with no nose cone, is a flat face: the blunt
  cylinder's `0.85 q_stag/q` at every Mach number, 0.85 at rest, 0.9947 at Mach 0.8, 1.0888 at
  Mach 1 and 1.4118 at Mach 2. Eq. 3.86 "does not take into account the effect of extremely blunt
  nose cones (length less than half of the diameter)" ([N09] p. 47), and a step has no length.
  Before [M1.8b1](../decisions-and-roadmap.md#m1-8b1) it was eq. 3.86's 0.8 at every speed.
- **Where eq. 3.87 has no solution.** An x^½ nose meets its tube at a small angle, so it has some
  drag at rest, but Stoney's measured x^½ curve is still at 0 at Mach 0.8: no `a Mᵇ` can rise
  from the first to the second. Eq. 3.87 needs the transonic value above the value at rest and
  `b > 1`; where either fails, hpr goes from the value at rest to `C_T(M_L)` along
  `0.8 sin² φ + Δ (M/M_L)²` instead: continuous, flat at rest, with a kink at `M_L`. Falling, as
  here, it follows Stoney's measurement rather than Niskanen's assumption that the curve doesn't
  fall; the coefficients are below 0.01. Rising, it serves near-flat noses: a power series x^0.05
  has almost nothing at rest by eq. 3.86, which leaves bluntness out, and rises along it to 0.80
  at Mach 0.8.
- **Refused shapes.** A bulged secant ogive (its arc radius below the tangent ogive's) is outside
  eq. B.8, and a Haack series past `C = ⅓` outside Stoney's data (Niskanen limits it the same way,
  p. 103). The drag buildup refuses both, naming the component, when asked for drag; the model
  still builds, so the normal force, the centre of pressure and a drag table still work.

**Cross-check against a measured cone.** Stoney's Figure 12(a) also has a 3:1 cone. Niskanen's
closed form reads high through the whole rise: +87% at Mach 0.8 and +105% at 0.85, where eq. 3.87
carries its Mach 1 value down; +49% at Mach 1 and +48% at 1.1, where the cubic join is near its
peak, 0.234 against the measured 0.158; then +15% at Mach 1.5 and +4% at Mach 1.94, the curve's end
(`nose_drag::tests::niskanens_cone_against_stoneys_measured_cone`). Ogives inherit this. So a
stubby cone or ogive gains the most drag at high subsonic speeds: Bella Lui's 1.55:1 tangent ogive
takes its whole rocket's `C_D0` at Mach 0.9 38% above the model before
[M1.8b1](../decisions-and-roadmap.md#m1-8b1), which held the nose at its value at rest, where the
von Kármán noses of Calisto and Prometheus move it under 1% ([ADR-028][adr-028]).

### Boattails faster than sound

A [boattail](../glossary.md#boattail) narrows the body toward the tail, usually to shrink the flat
base behind it. Below Mach 0.8 hpr keeps Niskanen's boattail rule from the table above, a share of
the base drag. Faster than sound two more things happen. The air turns inward around the
boattail's shoulder and expands, as in a [Prandtl–Meyer expansion](../glossary.md#prandtlmeyer-expansion):
it speeds up, its pressure falls below the free stream's, and it pulls back on the boattail. That
is a [wave drag](../glossary.md#wave-drag), and on a short, steep boattail it can be the largest
drag term on the rocket. Behind the boattail, the base's pressure is higher than behind a plain
cylinder, which lowers the base drag. Code: [`hpr_aero::afterbody`](../api/hpr_aero/afterbody/index.html).
Decision: [ADR-030][adr-030].

**How far to trust it.** Against 58 readings of 20 measured boattails of 3° to 10° from Mach 1.2
to 3.12 it reads −21.9% to +28.3%, and within 0.0123 in drag coefficient: the largest percentages
are the smallest drags. Theory that leaves out the air's viscosity (inviscid theory) reads such
boattails up to about 20% high ([CS51] p. 17). Through Mach 1 it reads low, and below Mach 0.8
the rule gives long, gentle boattails almost nothing. Steeper boattails in a thick
[boundary layer](../glossary.md#boundary-layer) read 26% to 54% high, and the one full rocket
measured with one, the Arcas Robin, reads high too. So [M1.8b3](../decisions-and-roadmap.md#m1-8b3)'s
targets were not met: none of the Arcas Robin's 11 fins-off readings from Mach 1.5 is within 10%,
and 8 of Calisto's 17 supersonic rows against RASAero II are. No whole flight in the validation
suite uses this model yet: none of its boattailed rockets passes Mach 0.8. The rules for a
boattail drawn in parts, a lip in its wake and the gaps between (the last three rows of the table)
apply at every speed, and are judgements that no measurement checks but the Arcas Robin's lip.

In the table, a boattail runs from diameter `d₁` to `d₂` over its length `l`; `a = (d₂/d₁)²` is its
area ratio, `θ = atan((d₁ − d₂)/(2l))` its half-angle (the cone through the same ends; curved
boattails are taken as that cone), and coefficients are on its fore area `π d₁²/4`, the
cross-section where it starts. `C_p,PM` is the pressure coefficient after the Prandtl–Meyer turn,
`(C_D•)_base` the base drag coefficient of the table above, `a_b` the base's area over `π d₁²/4`,
and `p_cyl/p_bt` a cylinder's base pressure over a boattail's. "Jet off" means the measurements
were made with no motor exhaust.

| piece | what hpr does | source |
|---|---|---|
| wave drag, attached flow | MIL-HDBK-762's chart for conical boattails, `4 C_D (l/d₁)²` against `x = √(M² − 1)/(2 l/d₁)` for `a` from 0.25 to 0.80, read into the code (±(0.005 + 2%)) | [762] Fig. 5-122, p. 5-187 |
| its upper limit | never more than the pressure after a two-dimensional Prandtl–Meyer turn through `θ` over the whole annulus, `−C_p,PM(M, θ)(1 − a)`; past the chart's end at `x = 1.4` the drag approaches that limit, the gap shrinking as `1/x` | [R1135] eq. 44, 171c |
| separation | between 16° and 30°, a straight-line blend in `θ` from the attached value to the base drag on the annulus, `(C_D•)_base(1 − a)`, where [flow separates](../glossary.md#flow-separation) | [C57] pp. 6, 8 |
| through Mach 1 | the rule to Mach 0.8, where the buildup's other transonic terms start; a straight line to Mach 1; from there the attached drag held at its Mach 1.2 value to Mach 1.2 | [N09] p. 47, [762] p. 5-47 |
| base behind a boattail | from Mach 2.5, `p_cyl/p_bt = 0.442 + 0.558 a_b`, with the cylinder's pressure from Love's correlation of measured bases, turned into the ratio of the two base-pressure coefficients, `k = (1 − p_bt/p)/(1 − p_cyl/p)`, which multiplies hpr's own base drag; below Mach 2.5, `k` at Mach 2.5; back to 1 between Mach 1 and 0.8; none for a separated boattail | [762] Figs. 5-139, 5-141, pp. 5-208, 5-210 |
| a lip in its wake | a lip behind a boattail or a step down (a boattail of no length), drawn as a shoulder, a step up or both, in one part or several, loses its pressure drag while its top rises up to a quarter of the boattail's drop in diameter above the boattail's end, keeps all of it from half, and a straight-line share between; a lip in parts takes at each part the smallest share any top so far leaves; the base behind it takes the same share of the relief, less the lip's length's fade. A lip's rise is its top diameter less the boattail's aft diameter. So a motor retainer behind the step down to its motor tube loses much of its step's drag: behind a 98 mm airframe stepping down to a 54 mm motor tube, a 62 mm retainer rises 8 mm, 0.18 of the 44 mm drop, so its rise alone would shelter it wholly, and the 12 mm of motor tube ahead of it fades that by 12/44: its step keeps 27% of its drag (`a_retainer_behind_a_step_down_is_in_its_wake`), and the rocket's `C_D0` reads 12% to 23% lower from Mach 0.3 to 2.5 than with the retainer's step in full (the physics review's measurement; unmeasured in any tunnel); with the exposed motor tube as long as the 44 mm drop, none | [D4014] p. 6, [R22] slide 2; the quarter and half are a judgement |
| a boattail in parts | a narrowing part after another drags, as its share of the boattail it continues, as the cone from that boattail's start through its aft end less the cone through its fore end (below 0 where extending the boattail lowers its drag); so parts of one straight cone add up to one cone. Its drag is that share for a turn of up to 3° between the parts, its own drag as a boattail from 10° (a corner), and a straight-line blend of the two between (the *turn* is the difference of the two parts' half-angles); when either part is shallower than 1°, the merge is scaled by the smaller angle over the larger (the larger taken as at most 1°), so a part narrowing by almost nothing acts as a tube and a straight cone of any angle drawn in parts merges wholly. This holds at every speed, so below Mach 0.8 a curved boattail in parts drags as the cones through its ends, not part by part as eq. 3.88 would | a judgement |
| gaps and steps | whatever lies between a boattail and what follows weakens its effect in a straight line, gone once the gaps add up to one of the boattail's drops in diameter; gaps add: the lengths of tubes, lips and parts, and the drops in diameter of steps down and narrowing parts. With several boattails ahead, the flow is shared among them: a narrowing part takes over the share it merges with, and takes what no boattail holds as its own; a step down counts as a boattail of no length. The base and each lip add the boattails' shares. So a part narrowing by nothing drags as a tube, a part of no length as a step, and a small change in any radius or length changes the drag a little | a judgement |

Why each piece is there:

- **The chart is second-order theory.** MIL-HDBK-762 cites no source for it. Jack computed conical
  boattails by Van Dyke's second-order theory ([J53]), an inviscid method that keeps the next
  term past linear theory's small-disturbance approximation, from Mach 1.5 to 4.5, and the chart
  agrees with his 83 points inside it within −10.4% to +8.0% for `a` up to 0.6. First-order
  (linear) theory reads far higher at these angles, so the chart is not linear theory.
- **The 2D limit** matters for short, steep boattails. The chart plots its drag against one
  combined variable, `x`, which holds only for small angles, and near Mach 1 it can ask for more
  suction than a flat (two-dimensional) turn gives, which a round boattail, whose pressure
  recovers aft of the shoulder, can't exceed. Past the chart the same limit gives the curve its
  shape: against Jack's 28 points beyond it, −2.7% to +8.0%.
- **Near Mach 1** no method exists for boattails; [762] p. 5-47 says so and advises holding the
  supersonic value to a peak between Mach 1.0 and 1.2. The straight line starts at Mach 0.8, where
  hpr's other transonic terms start, and is half-way up at 0.9. The measured rise is later and
  steeper: half-way by about 0.89 for Compton's 10° boattail ([C72]) and 0.92 to 0.96 for
  Cubbage's ([C57]), with a peak at Mach 1.0 to 1.1 that holding the Mach 1.2 value doesn't
  reach. An earlier draft started the line at Mach 0.9, a choice made after seeing the Arcas
  Robin, one of the targets; the validation audit caught it, and it was put back to 0.8.
- **The base's relief** is the handbook's correlation, measured at Mach 2.5 to 3.5. Used as a
  ratio of pressures below Mach 2.5 it over-predicts the relief of the bases measured at Mach
  1.59 and 1.91 ([DN54], [CS51]); held as a ratio of coefficients, it matches them on average.
  It depends on the base's area only, where the measured relief also grows with the boattail's
  angle: behind Cortright and Schroeder's small bases (`a_b = 0.256`) hpr keeps 0.352 of the
  cylinder's base pressure coefficient at every angle, where they measured 0.60 at 5.6° and 0.26
  at 9.3°.
- **The lip.** NASA's Arcas Robin models end in a lip 1.3 mm long that flares from the boattail's
  end to the base. NASA found it lowering the force on the balance chamber inside the base at
  Mach 1.5 and 1.8 with the fins off, and its effect "masked" when the flow over the boattail
  separates or the boundary layer thickens ([D4014] p. 6); RASAero II's own comparison with the
  tunnel left it out as "buried in the boattail boundary layer" ([R22]). hpr used to take it as a
  stubby cone in undisturbed air, 0.085 of drag. The quarter and half that bound the wake were
  chosen knowing this lip rises 0.17 of its boattail's drop, and every one of the 44 Arcas Robin
  rows depends on that choice: with the lip counted as a shoulder in undisturbed air, each would
  read 0.065 to 0.086 higher.

**A worked example: Calisto at Mach 1.5.** Calisto's boattail is 60 mm long from 127 mm to 87 mm:
`a = 0.469`, `l/d₁ = 0.472`, `θ = 18.4°`. The chart's `x` is `√1.25/(2 × 0.472) = 1.18`, where it
gives `C_D = 0.219`. A Prandtl–Meyer turn of 18.4° from Mach 1.5 gives `C_p = −0.398`, so the
limit is `0.398 × 0.531 = 0.211`, and the attached boattail drags 0.211. At 18.4° it is 17% of
the way from 16° to 30°, so the blend takes it 17% toward the base drag on the annulus,
`0.167 × 0.531 = 0.088`: 0.190, where the rule gave 0.066. For the base, `a_b = 0.469`: at Mach
2.5 Love's cylinder gives `−C_p = 0.1195`, so `p_cyl/p = 1 − 0.1195 × 0.7 × 2.5² = 0.477`, and
`p_bt/p = 0.477/(0.442 + 0.558 × 0.469) = 0.678`, so `k = 0.322/0.523 = 0.616`; after the blend
0.683, so the base's drag falls from 0.078 to 0.053. Calisto's `C_D0` at Mach 1.5 rises from
0.443 to 0.542. The module's documentation test runs this example
([`hpr_aero::afterbody`](../api/hpr_aero/afterbody/index.html)).

**Against measurements** ([`drag-vs-mach.json`][drag-fixture], sections `boattails`,
`base_pressures` and `second_order_theory`, from the readings in
[`measured-boattails.json`][boattail-fixture]; `tests::boattails_against_measurements`). Every
source is a wind tunnel with a turbulent boundary layer and the jet off; each value was read from
the report's figure, with its reading uncertainty in the file. The last column says whether the
rows helped build the model: those check it on its own data.

| boattails | Mach | rows | hpr against measured | helped build it |
|---|---|---|---|---|
| attached, 3° to 10°: [CS51], [DN54], [C72], [MJ54], [C57] | 1.2 to 3.12 | 58 | −21.9% to +28.3%, within 0.0123 | no |
| attached, 5.6° and 8° ([C57]) | 1.0 and 1.1 | 4 | −18.2% to −5.4% | partly: Cubbage's peak was weighed in holding the Mach 1.2 value |
| attached, 3° to 10°, points near Mach 1 that Compton calls questionable (strut interference, reflected bow shock; [C72] p. 9) | 0.95 to 1.1 | 27 | −46.2% to +60.0% | no |
| attached, 3° to 10°, in the rise ([C72], [C57]) | 0.85 to 0.95 | 28 | −77.5% to +7.6% | no |
| attached, 3° to 10°, under the rule ([C72], [C57]) | 0.3 to 0.8 | 58 | −100% to −83.5% | no |
| 16°, attached, boundary layer 0.20 `d₁` thick ([C57]) | 1.0 to 1.28 | 9 | +26.4% to +54.2% | no |
| the same, below Mach 1 | 0.6 to 0.9 | 6 | −30.2% to +60.4% | no |
| 30° and 45°, separated ([C57]) | 1.2 | 3 | −2.8% to +6.6% | yes: the separation angles |
| the base behind 5° to 10° boattails ([CS51], [DN54]) | 1.59, 1.91 | 8 | base drag within 0.0102 of the measured on the cylinder's area; behind small bases up to about 40% of the base's own drag | yes: the ratio held below Mach 2.5 |
| the base behind 2.5° to 15° boattails ([Love57]) | 3.24 | 4 | within 0.003 | no |

The 0.0102 and 0.0123 are pins on these readings, not tolerances. Below Mach 0.8 Niskanen's rule,
which gives nothing to a boattail longer than three times its drop in diameter, gives Compton's
and Cubbage's long, gentle boattails 0 to 0.009 where they measure 0.011 to 0.075 (issue
[#73](https://github.com/nrdptel/hpr-sim/issues/73)).

**What it leaves out.**

- **Steep boattails in a thick boundary layer read high.** hpr reads Cubbage's 16° boattails, in a
  boundary layer a fifth of the diameter thick, 26% to 54% above the measurements, though the
  flow is still attached. The Arcas Robin's 15° boattail sits in one thicker than its own drop in
  radius, and its forebody reads +13.5% to +24.1% from Mach 1.5 (below). No source here gives a
  correction, so none is applied (issue [#72](https://github.com/nrdptel/hpr-sim/issues/72));
  MIL-HDBK-762 advises boattails under 8° to avoid separation ([762] p. 5-12).
- **Through Mach 1 it reads low** for gentle boattails: the straight line from Mach 0.8 misses the
  measured rise's later, steeper climb and its peak.
- Separation's 16° and 30° come from one report at Mach 0.6 to 1.28. Between 10° and 30° no
  attached boattail was measured faster than Mach 1.28, so a boattail of 12° to 20°, like
  Calisto's 18.4°, rests on the least-validated part of the model, and likely reads high.
- For one length and area ratio, Jack found the cone's wave drag the smallest of three shapes
  ([J53] p. 1), so a curved boattail likely drags more than hpr gives. The chart's 0.70 and 0.80
  curves read up to 32% above Jack past `x ≈ 1`, 0.0084 at most.
- A tube behind a boattail loses the base's relief over one drop in diameter, and the 3°, 10°,
  quarter and half that shape the merge and the wake are judgements, with no measurement behind
  them but the Arcas Robin's lip. Its own 1.3 mm length is a gap too: the base keeps 0.944 of its
  relief.
- A straight cone drawn in parts drags as one cone. A part's share can be below 0, where the chart
  makes the longer cone drag less than the shorter: extending the boattail lowers its drag. The
  one exception: behind a boattail it only partly merges with (a turn of 3° to 10°), a cone's
  parts may drag a little less than the whole: an 8° cone behind a 14° part reads the same in 2
  or 4 parts, up to 0.45% lower in 8, and up to 2.2% lower drawn in hundreds, worst near Mach 1.
- The 1° below which a part merges only in part, and the fade of a step's corner over one drop in
  diameter (flow behind a backward-facing step reattaches farther downstream, so this likely
  understates a lip's shelter), are judgements too.
- Nothing models the jet. Fig. 5-141 is measured with the motor off, as is the base drag it
  scales, and under power hpr applies both to what the motors leave of the base.

### Drag limits

- The buildup covers Mach 0 to 5 and refuses Mach 5 and faster, like the normal force
  (`drag::BUILDUP_MACH_LIMIT`); an override table takes any Mach number.
- **It reads high against the one wind tunnel it has been measured against**, at every reading:
  with the fins off, 12% to 54%, most of it the model's steep boattail; with the fins on, past
  Mach 1.2, far more, from the fins
  ([Verification](#drag-against-the-arcas-robin-wind-tunnel)).
  The fins' leading edge takes [N09]'s rounded-edge formula, a blunt edge's, for the airfoil and
  rounded sections alike. Nothing models the wave drag of a thin, sharp fin, which is far smaller:
  the Arcas Robin's four double-wedge fins measure 0.046 at Mach 4.63, against hpr's 0.30.
- **Against MIL-HDBK-762's worked example**, fins left out, the body reads high from Mach 0.9 to
  1.2 (the nose and the base) and 6% to 10% low from Mach 1.6 (friction and the base)
  ([Drag against MIL-HDBK-762's sample calculation](#drag-against-mil-hdbk-762s-sample-calculation)).
- **It reads low against RASAero II's Calisto past Mach 1.6**, −14.9% at Mach 2
  ([Drag against RASAero II through Mach 2](#drag-against-rasaero-ii-through-mach-2)). Part of
  that is the body's lean above; plausible fin inputs bring most rows within 10%.
- **Through the transonic rise,** from Mach 0.8 to 1.2, the measured shapes follow Stoney's
  curves, and cones and ogives Niskanen's closed form, which reads 45% to 105% above Stoney's
  measured 3:1 cone there (the cross-check above).
- **Shoulders and boattails past Mach 1.** A shoulder takes the nose method, which [N09] calls
  "somewhat dubious at supersonic velocities" (p. 48), except a lip in a boattail's wake, which
  loses a share of it. A boattail keeps eq. 3.88 to Mach 0.8, a rule "based primarily on subsonic data"
  (p. 49), which over-predicts the Arcas Robin's 15° boattail and gives long boattails nothing
  (issue [#73](https://github.com/nrdptel/hpr-sim/issues/73)). Faster than sound its wave drag
  reads high for steep boattails in a thick boundary layer
  ([Boattails faster than sound](#boattails-faster-than-sound)).
- **Stoney's curves end** at Mach 1.94 to 1.99 (panel (a)) and 3.59 (panel (b)); past that hpr
  holds their last value, which panel (b) puts within 8% for two shapes (above).
- **Stubby noses.** Below fineness 1 a cone or ogive blends toward the flat face at every Mach
  number, so at rest it reads above eq. 3.86: a cone of fineness 0.5 gives 0.547 against eq.
  3.86's 0.400. And two routes to one shape disagree: a tangent ogive and an ellipse of fineness
  0.5 are both hemispheres, but the ogive rises from rest by that blend while the ellipse, on
  Stoney's measured curve scaled by eq. B.9, stays at 0 until Mach 0.8 and then climbs steeply.
- Before [M1.8b1](../decisions-and-roadmap.md#m1-8b1) the buildup held nose and shoulder pressure
  drag at its value at rest and refused Mach 1.
- Nothing models laminar flow, fin-tip vortices, interference drag, fin tabs, fillets, canted fins
  or the flow a boattail guides into the base ([N09] p. 51).

## Validity and open questions

- These are small-angle models. `α` is accepted over `[0, π]`, but fin slopes stay linear in `α`
  and nothing models stall. The flight engine uses them at every angle all the same
  ([Rigid-body flight](flight.md)), so its results are least trustworthy where large angles occur:
  off the rail in a strong crosswind, and near apogee.
- **Body lift in wind** (measured by flying both codes, not against a real flight;
  [ADR-026][adr-026]). A rocket that leaves the rail slowly in
  a crosswind meets the air at a steep angle. Juno III, one of RocketPy's example rockets, leaves
  at 18 m/s in an 8.5 m/s wind, 26° off the airflow, and there body lift is about half its normal
  force. Much of it acts ahead of the rocket's centre of mass, the nose's above all, so it moves
  the centre of pressure forward and weakens the moment that
  [turns the rocket into the wind](../glossary.md#weathercocking); hpr turns into it less than
  RocketPy, whose normal force has no body term. Its sideways push alone is about a sixth of the
  effect. Juno III's apogee ends 228.0 m from the pad in hpr and 396.6 m in
  RocketPy; body lift is about half of that difference, and hpr's rail release and fin slope most
  of the rest. `K` matters there: across [G]'s range, Juno III's apogee drift runs from 194.1 m at
  `K = 1.5` to 240.2 m at 1.0, and would be 328.0 m with no body lift (flown in RocketPy with
  hpr's model; hpr gives within 4 m of each). Calisto, off the rail at 28 m/s and 11°, changes
  its drift by under 0.5% across that range. Which is nearer a real flight is
  open until [M2.3](../decisions-and-roadmap.md#m2-3).
- **No airfoils.** Fins use the flat-plate lift slope (2π per radian in two dimensions). An airfoil
  lift curve, such as the one Juno III's example gives its fins, is not modelled; RocketPy uses
  it, and its fin slope there is 7.6% steeper ([ADR-026][adr-026]).
- In one measured case, fins at `α = π/2` give `C_N` 17.4 against a flat-plate estimate near 5, and
  at `α = π` the fins still give 34.7 while every body term vanishes. That case is a 54 mm
  four-fin rocket at Mach 0.3.
- **Speed.** The normal force and the drag buildup cover `0 ≤ M < 5`; a drag table covers any
  Mach number.
  - The drag buildup's transonic and supersonic terms are [N09]'s semi-empirical ones. [N09]
    expects them "to be reasonably accurate to at least Mach 1.5" (p. 94); against the one wind
    tunnel they read high from about Mach 1.2 ([Drag limits](#drag-limits)).
  - The normal force between Mach 0.8 and linear theory's start `M_s` is the straight-line join
    of [Fins through Mach 1](#fins-through-mach-1), which the wind tunnel shows missing by up to
    +29.3% in slope and 2.29 calibres in CP. Past Mach 3 its body terms read low
    ([Normal force through Mach 1](#normal-force-through-mach-1)).
  - [N09] eq. 3.35–3.36 would start moving a fin set's CP aft at Mach 0.5, to about 0.30 of the
    way along its mean aerodynamic chord (MAC, defined under *Fins*) at Mach 0.8 for fins of
    aspect ratio 1.6 (a measure of how long the span is against the chord). hpr keeps 0.25 to
    Mach 0.8: the Arcas Robin's measured CP moves forward, not aft, from Mach 0.6 to 0.8.

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
  - [L7](../decisions-and-roadmap.md#l7), a fin slope and CP that never changed with Mach:
    `fins::tests::fin_cna_compressibility_reduces_to_barrowman_at_m0`
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
  - Supersonic linear theory on a rectangle, whose slope and CP have closed forms, including a
    tip cone that crosses the root; outlines of each planform; where linear theory starts.
  - A proptest (a rule checked on many random inputs): scaling every length leaves slopes
    unchanged and scales the CP; the reference diameter scales slopes only.
  - Refusals: tube fins, nine fins, Mach 5 for the normal force and Mach 1 for the drag buildup,
    angles out of range.

### Normal force through Mach 1

Two references, in the fixture
[`validation/fixtures/aero/normal-force-vs-mach.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/normal-force-vs-mach.json),
which `cargo xtask aero` writes and `tests::normal_force_against_mach` recomputes and pins
([ADR-027][adr-027]). The targets, set before measuring: `C_Nα` within 15% and the CP within 0.5
calibres (a calibre is one reference diameter). 16 of the 37 rows miss, each for a measured
reason below.

- **A wind tunnel.** NASA tested half-scale models of the Arcas Robin sounding rocket from Mach 0.6
  to 4.63 ([D4013], [D4014]): a nose 4.2 calibres long, a cylinder, a 15° boattail and four trapezoidal
  fins swept 30°, 18.2 calibres long in all, and a longer version of 23.8. The reports print only
  plots, so their points were read off the pages into
  [`validation/fixtures/aero/arcas-robin-wind-tunnel.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-wind-tunnel.json),
  with every figure and page, each `C_N` to about ±0.01 to ±0.02. The designs model the reports'
  nose, a table of coordinates rather than a named shape, as a power-series nose with the same
  volume, which sets its slender-body CP, and a planform within 2.4%. The slope is the straight line
  fitted through the plotted `C_N` from about −4° to +4°, so hpr's `C_N` is fitted the same way at
  the same angles. Its CP is taken over −2° to 2°, the reports' low angles.
- **RASAero II,** another code, for Calisto from Mach 0.1 to 2.0: its potential-flow slope (the
  attached-flow part, without the crossflow lift its export adds from Mach 0.95) and CP from the
  export RocketPy's first commit shipped, against hpr's small-angle values.

The short model (the Arcas Robin itself, 18.2 calibres long), rows outside the targets in bold.
The last column is the body alone: the fins-off wind-tunnel reading, and hpr's body terms fitted
the same way. hpr's body terms don't change with Mach, but its body lift grows as `sin² α`, so
their fitted slope moves a little with the angles each plot happens to cover (2.15 to 2.34).
From Mach 0.6 to 1.2 the fins-off readings, on a coarse grid (±0.02 per point), scatter from 1.41
to 2.88 with no trend, so they don't settle whether hpr's 2.15 is high there.

| Mach | `C_Nα` measured, per rad | hpr | difference | CP measured, m | hpr | difference, calibres | body alone, measured / hpr |
|---|---|---|---|---|---|---|---|
| 0.6 | 11.05 | 10.78 | −2.4% | 0.7770 | 0.7807 | +0.06 | 1.53 / 2.15 |
| **0.8** | 9.92 | 11.10 | +11.9% | 0.7375 | 0.7866 | +0.86 | 1.41 / 2.15 |
| **0.9** | 10.22 | 13.02 | +27.4% | 0.7447 | 0.8186 | +1.29 | 2.44 / 2.15 |
| **0.95** | 11.88 | 13.98 | +17.6% | 0.7952 | 0.8314 | +0.63 | 2.88 / 2.16 |
| 1 | 15.79 | 14.94 | −5.4% | 0.8690 | 0.8427 | −0.46 | 1.58 / 2.16 |
| **1.2** | 15.42 | 18.78 | +21.8% | 0.8862 | 0.8774 | −0.15 | 2.43 / 2.16 |
| 1.5 | 13.43 | 13.87 | +3.3% | 0.8137 | 0.8379 | +0.42 | 2.19 / 2.28 |
| 1.8 | 11.99 | 11.43 | −4.7% | 0.7908 | 0.8032 | +0.22 | 2.61 / 2.26 |
| 2.3 | 9.89 | 9.23 | −6.7% | 0.7505 | 0.7529 | +0.04 | 3.08 / 2.32 |
| 2.96 | 8.77 | 7.60 | −13.4% | 0.6967 | 0.6955 | −0.02 | 3.28 / 2.34 |
| **3.96** | 7.73 | 6.22 | −19.6% | 0.6312 | 0.6215 | −0.17 | 3.88 / 2.31 |
| **4.63** | 7.55 | 5.66 | −25.0% | 0.5876 | 0.5783 | −0.16 | 4.15 / 2.31 |

| reference, Mach | `C_Nα` difference | CP difference, calibres | rows within both targets |
|---|---|---|---|
| Arcas, long, 0.6 and 0.8 | +3.6%, +12.0% | −0.44, −0.01 | 2 of 2 |
| Arcas, long, 0.9 to 1.2 | −6.9% to +29.3% | +0.60 to +2.29 | 0 of 3 |
| Arcas, long, 1.8 to 2.96 | −8.2% to +0.5% | −0.12 to −0.04 | 3 of 3 |
| Arcas, long, 3.96 and 4.63 | −17.2%, −22.8% | −0.16, −0.19 | 0 of 2 |
| Calisto against RASAero II, 0.1 to 0.7 | +0.1% to +10.1% | −0.08 to +0.43 | 4 of 4 |
| Calisto against RASAero II, 0.8 to 2.0 | −16.8% to +21.9% | −0.56 to +0.95 | 6 of 11 |

What the misses come from:

- **Past Mach 3, the body.** The fins' share (the fins-on reading less the fins-off one) agrees with hpr's fins within
  −1.4% to +7.0% at Mach 3.96 and 4.63. The body alone lifts 3.9 to 4.6 per rad there, where hpr
  gives 2.3 to 2.8: slender-body theory's nose and boattail don't change with Mach, and the real
  body lifts more as it flies faster. The CP stays within 0.19 calibres, so the stability margin
  holds, but the slope is low. The planned increment
  [M1.8e](../decisions-and-roadmap.md#m1-8e) takes this on. The fins' agreement carries about 5%
  of doubt of its own: over the boattail the models' fin roots follow its 15° surface below the
  cylinder, and the design leaves that strip out, about 0.32 in² of each fin's 5.8 in² (5.5%).
- **Mach 0.6, within the targets by errors that cancel.** Both models pass there, but hpr's body
  is 40% and 34% above the fins-off readings, which are poorly determined at these speeds, and its
  fins' share 9.3% and 3.9% below the measured one.
- **Transonic, Mach 0.8 to 1.2.** The fins' measured share lifts less at Mach 0.8 and 0.9 than at
  0.6, then jumps at Mach 1. hpr's fins lift more, by Prandtl–Glauert and then along
  the join to linear theory's peak at `M_s` (1.2 for these fins). The long model's CP jumps
  forward at Mach 1, 2.29 calibres from hpr's. No closed-form method covers this region, and the
  join is not fitted to it.
- **RASAero II** keeps its slope and CP constant through subsonic flow, where hpr's rise with
  Prandtl–Glauert, so they part from Mach 0.8. The wind tunnel sides with neither there. Below
  that the agreement is partly by construction: the Calisto design has the 2018 fins because
  they reproduce this export at low speed ([ADR-009][adr-009]). Past Mach 1 the result rests on
  the choice of RASAero's columns: against its secant slope and CP to 4°, which include its
  crossflow lift, 1 of the 11 rows from Mach 0.8 is within the targets, not 6, and Mach 2 is
  −30.6%. Calisto has no fins-off data, so its Mach 2 miss can't be split as the wind tunnel's
  can.

So, for fins like these, whose linear theory starts at `M_s` = 1.2: from Mach 1.5 to about 3,
trust hpr's slope to about 15% and its CP to about half a calibre; past Mach 3 the CP still, but
the slope reads low; between Mach 0.8 and `M_s`, in the join, neither. A fin set's own `M_s` is
[`FinSetAero::fin`](../api/hpr_aero/model/struct.FinSetAero.html#structfield.fin)`.supersonic_mach`,
from [`AeroModel::fin_sets`](../api/hpr_aero/model/struct.AeroModel.html#method.fin_sets). Fins
swept further back start later: a leading edge swept 48° starts at Mach 1.5, and until then it
is in the join. Nothing past Mach 4.63 has been checked, though the model runs to 5.

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
  1e-8); a whole rocket's buildup written out by hand to 1e-12, its component sum, and override
  tables (rescaled to another reference diameter).
- **Drag through Mach 1** (`nose_drag::tests`, `drag::tests`):
  [Loft lesson L17](../decisions-and-roadmap.md#l17), Loft's fin leading-edge drag frozen at its
  Mach 1 value and nose drag with no Mach term, is
  `drag::tests::leading_edge_and_cone_pressure_drag_have_supersonic_branches`. A 3:1 cone by hand
  from rest through eq. B.4, with its joins smooth to 1e-5; the ogive factor at its ends and middle;
  eq. B.9 through both its anchors; eq. 3.87 meeting its lower bound and its fallback; a step from
  0.8 to the flat face; short cones tending to the step and meeting the closed form at fineness 1
  ([Loft lesson L15](../decisions-and-roadmap.md#l15)); Stoney's curves reproduced at fineness 3,
  held past their ends and interpolated between shapes; the guide's worked example; the 3:1 cone
  against Stoney's measured one; refused shapes; and a property test that every shape at any
  fineness, joint angle and Mach number to 5 gives a finite, non-negative coefficient with no jump
  at `M_L`.
- **Boattails faster than sound** (`afterbody::tests`): the Prandtl–Meyer function against NACA
  Report 1135's table and its limit of 130.45°, and its inverse; an expansion's pressure by hand
  and at the vacuum limit; the chart giving back its readings, falling with `x` and the area
  ratio, and reaching 0 at `a = 1`; Calisto's boattail by hand (the guide's worked example) with
  the joins at Mach 0.9, 1 and 1.2; the drag past the chart continuous at its end and closing on
  the 2D limit; separation from 16° to 30°; the base-pressure ratio on Fig. 5-141's line; and
  every boattail from 1° to 89° finite and non-negative to Mach 5. The comparison with measured
  boattails and Jack's theory is `tests::boattails_against_measurements`.
- **Boattails in parts, wakes and gaps** (`drag::tests`): a boattail split in two drags as one
  (`a_boattail_split_in_two_drags_as_one`), and so does a straight cone of 0.5° to 7° in 2, 4 or 8
  parts, where a part's share is below 0 too (`a_straight_cone_in_parts_is_one_cone`); a corner keeps two parts apart and the
  merge is continuous in the turn (`a_sharp_corner_keeps_its_boattails_apart`); a pair drags
  between its two limits (`soft_merges_stay_between_their_limits`); a partial merge shares the
  flow (`a_partial_merge_shares_the_flow`); a change of `ε` in any radius or length behind 2° to
  14° boattails moves the drag in proportion to `ε`
  (`a_part_narrowing_by_nothing_is_a_tube_and_one_of_no_length_a_step`); a lip's wake by its rise
  and its gaps (`a_lip_in_a_boattails_wake_fades_with_its_rise`, `a_lip_drawn_as_a_step_up_is_a_lip`,
  `a_hairline_step_before_a_lip_changes_nothing`); a retainer behind a step down
  (`a_retainer_behind_a_step_down_is_in_its_wake`); and a 40-part zigzag keeps its tails few
  (`a_zigzag_boattail_keeps_its_tails_few`).
- **Tables** (`table::tests`): RocketPy's quirks (`\r\n`, `01.05`, a repeated row), a byte-order
  mark, quoted fields and trailing commas, RASAero II's header with rows at 2° and 4° skipped, and
  malformed text (a bad first row, repeated or unsorted Mach numbers, `nan`) by line.

### Drag against the Arcas Robin wind tunnel

NASA measured the axial force on its half-scale Arcas Robin models, the same ones as the normal
force above, from Mach 0.6 to 4.63 ([D4013], [D4014]), fins at 0° and with the fins off. The
models sat on a sting, so their base pressure isn't a free flight's, and both reports take the base
apart: [D4013] plots the axial force "corrected for base axial force" (`C_A,corr`, Figs. 11–12),
and [D4014] the axial force and, separately, the force on the balance chamber inside the base
(`C_A,c`, Figs. 4–6). What compares, then, is the forebody: hpr's `C_D0` less its base drag
(friction, pressure and parasitic drag) against the measured axial force with the base at the free
stream's pressure. For [D4014] that is `C_A − 1.383 C_A,c`, which takes the chamber's pressure over
the whole base as [D4013]'s correction does: 1.383 is (1.470/1.250)², the base's diameter in inches
over the 1.250-inch cavity drawn in [D4014] Fig. 1(a), squared. The report states no chamber
area, and taking it over the chamber alone moves the measured values by 0.002 to 0.015, which
changes no row's verdict. The readings are in
[`arcas-robin-wind-tunnel.json`][wind-tunnel], read off the reports' plots with their figure,
page and reading uncertainty (±0.002 in `C_A` for most, against the reports' own ±0.004).

hpr flies the committed designs ([Normal force through Mach 1](#normal-force-through-mach-1)) at
both tunnels' Reynolds number, 3.0 million per foot, with two inputs set for drag before measuring:
the double-wedge fins take hpr's airfoil section, as Niskanen modelled them ([N09] p. 90), and the
machined steel models a polished finish, 0.5 µm, since the reports state none. The target, set
before measuring, was [M1.8](../decisions-and-roadmap.md#m1-8)'s 10% for drag. `cargo xtask aero` writes
[`drag-vs-mach.json`][drag-fixture], each row with hpr's drag by part, and
`tests::drag_against_mach` recomputes it from the designs and pins the 2 rows of 44 within target.
The two input choices matter, and moved hpr toward the tunnel: with square edges and the default
20 µm finish no row is within target, with the airfoil section alone none, with the polished
finish alone none, and with both 2 (`tests::drag_against_mach_depends_on_the_fins_and_finish`).
The airfoil section follows the drawings and Niskanen; the finish is a guess. Allowing each
reading its uncertainty and the reports' ±0.004, neither of the 2 could fall the other side of
10%. Before
[M1.8b3](../decisions-and-roadmap.md#m1-8b3) modelled the boattail faster than sound and the lip
in its wake, 8 rows were within target, 6 of them because the lip's 0.085 made up for the missing
wave drag. Forebody drag on the reference area, measured and hpr's, and hpr's error:

| Mach | fins | short: measured | hpr | error | long: measured | hpr | error |
|---|---|---|---|---|---|---|---|
| 0.6 | on | 0.2987 | 0.3371 | +12.9% | 0.3455 | 0.3874 | +12.1% |
| 0.6 | off | 0.2217 | 0.2523 | +13.8% | 0.2477 | 0.3041 | +22.8% |
| 0.8 | on | 0.3299 | 0.4200 | +27.3% | 0.3706 | 0.4688 | +26.5% |
| 0.8 | off | 0.2308 | 0.2585 | +12.0% | 0.2517 | 0.3088 | +22.7% |
| 0.9 | on | 0.4202 | 0.6346 | +51.0% | 0.4510 | 0.6826 | +51.3% |
| 0.9 | off | 0.2610 | 0.3623 | +38.8% | 0.2671 | 0.4116 | +54.1% |
| 0.95 | on | 0.5680 | 0.6722 | +18.3% | — | — | — |
| 0.95 | off | 0.2916 | 0.4214 | +44.5% | — | — | — |
| 1.0 | on | 0.6858 | 0.7344 | +7.1% | 0.7247 | 0.7825 | +8.0% |
| 1.0 | off | 0.4194 | 0.5044 | +20.3% | 0.3674 | 0.5540 | +50.8% |
| 1.2 | on | 0.5935 | 0.7706 | +29.8% | 0.6245 | 0.8172 | +30.9% |
| 1.2 | off | 0.4311 | 0.5187 | +20.3% | 0.4220 | 0.5667 | +34.3% |
| 1.5 | on | 0.4932 | 0.6873 | +39.4% | — | — | — |
| 1.5 | off | 0.3401 | 0.4148 | +22.0% | — | — | — |
| 1.8 | on | 0.4260 | 0.6412 | +50.5% | 0.4543 | 0.6827 | +50.3% |
| 1.8 | off | 0.3142 | 0.3570 | +13.6% | 0.3267 | 0.3997 | +22.3% |
| 2.3 | on | 0.3328 | 0.5879 | +76.7% | 0.3730 | 0.6251 | +67.6% |
| 2.3 | off | 0.2474 | 0.2940 | +18.8% | 0.2927 | 0.3323 | +13.5% |
| 2.96 | on | 0.2639 | 0.5409 | +105.0% | 0.3010 | 0.5729 | +90.3% |
| 2.96 | off | 0.2030 | 0.2423 | +19.4% | 0.2362 | 0.2753 | +16.6% |
| 3.96 | on | 0.2056 | 0.4928 | +139.7% | 0.2342 | 0.5186 | +121.4% |
| 3.96 | off | 0.1554 | 0.1929 | +24.1% | 0.1894 | 0.2195 | +15.9% |
| 4.63 | on | 0.1850 | 0.4700 | +154.0% | 0.2113 | 0.4926 | +133.1% |
| 4.63 | off | 0.1390 | 0.1704 | +22.6% | 0.1648 | 0.1937 | +17.5% |

Why it misses, from the drag by part in the fixture:

- **The fins past Mach 1.2.** hpr's fins add about 0.30 from Mach 1.5 up, where the measured
  fins-on less fins-off falls from 0.153 at Mach 1.5 to 0.046 at 4.63: +78% at Mach 1.5 and +551%
  at 4.63 on the short model. The leading edge takes [N09]'s rounded-edge formula, whose value
  grows toward 1.2 on the fins' frontal area, where a thin, sharp fin's wave drag falls with Mach.
  Niskanen's own comparison with this wind tunnel shows the same, his simulation about 80% high by
  Mach 3.96 ([N09] Fig. 6.6, p. 90). At Mach 0.6 hpr's fins are +10% and −15% of the measured
  increment; from 0.8 to 0.9 they rise sooner than the measured fins do.
- **The boattail faster than sound.** hpr's 15° boattail drags 0.285 from Mach 1.0 to 1.2, 0.196
  at 1.5 and 0.037 at 4.63 ([Boattails faster than sound](#boattails-faster-than-sound)). If the
  rest of hpr's forebody were right, the tunnel's boattail would drag about 0.12 at Mach 1.5 and
  0.08 to 0.11 at 1.8, 40% to 94% under hpr, and next to nothing from Mach 3.96. Cubbage's 16°
  boattails, in a boundary layer a fifth of the diameter thick, read high the same way, 26% to 54%;
  NASA reports the flow separating over this boattail at the higher Mach numbers ([D4014] p. 6).
  With the fins off hpr reads +13.5% to +24.1% from Mach 1.5 (issue
  [#72](https://github.com/nrdptel/hpr-sim/issues/72)).
- **The lip.** The models end in a lip 1.3 mm long that flares from the boattail's 33.2 mm to the
  base's 37.3 mm. It sits in the boattail's wake, and hpr gives it no pressure drag. Before
  [M1.8b3](../decisions-and-roadmap.md#m1-8b3) hpr took it as a shoulder in the free stream, a
  stubby cone worth 0.065 at Mach 0.6 and 0.084 to 0.086 from Mach 1.2, which made up for the
  missing wave drag and put six rows within 10%. The short model also keeps the fins' raised root
  fairings with its fins off, which hpr leaves out and TN D-4013 blames for its higher drag from
  Mach 0.975 to 1.2 (pp. 4–5).
- **The boattail below Mach 1.** [N09]'s boattail rule (eq. 3.88) gives the 15° boattail a
  pressure drag of 0.063 at Mach 0.6 (its 0.070 less its friction). The tunnel's forebody holds
  that same pressure on the boattail's surface, and on the short model the whole forebody with its
  fins off measures 0.22 there, against hpr's friction alone of 0.19: little is left for the
  boattail's pressure. The rule over-predicts this boattail, as Niskanen found against the same
  tunnel ([N09] p. 90). At Mach 0.6 and 0.8 hpr's forebody with its fins off is +12.0% to +22.8%
  high, most of it the boattail rule.
- **Through Mach 1**, where drag rises steeply, the measured forebody with fins off jumps from
  0.29 to 0.42 between Mach 0.95 and 1.0 on the short model. hpr's boattail rises from 0.076 at
  Mach 0.8 to 0.285 at 1.0, sooner than the tunnel's, and the forebody with its fins off reads
  +38.8% to +54.1% from Mach 0.9 to 0.95 and +20.3% to +50.8% from Mach 1.0 to 1.2.

What this shows: hpr's drag reads high for this rocket at every Mach number, from about Mach 1.2
most of all by its thin, sharp fins, which hpr takes as blunt, and at every speed by its steep
boattail. The body alone reads +12.0% to +54.1%; before hpr modelled the boattail faster than
sound, with the lip in it and no wave drag, it read −9.2% to +71.1%. hpr's base drag, which the tunnel can't measure, is compared with
a calculation
([Drag against MIL-HDBK-762's sample calculation](#drag-against-mil-hdbk-762s-sample-calculation))
and, behind boattails, with measured bases
([Boattails faster than sound](#boattails-faster-than-sound)).

### Drag against RASAero II through Mach 2

[M1.8](../decisions-and-roadmap.md#m1-8) asks for drag within 10% of the curves labelled
[RASAero](../glossary.md#rasaero-ii) in [RocketPy](../glossary.md#rocketpy)'s example rockets
from Mach 0.1 to 2.0, with the errors by band. hpr doesn't meet that. This section gives the
errors, what the boattail's wave drag changed, and how far the curves' unrecorded inputs reach
([ADR-029][adr-029], the decision on this comparison, and [ADR-030][adr-030]).

**How it is compared.** `cargo xtask aero` compares hpr's zero-lift
[drag coefficient](../glossary.md#drag-coefficient), `C_D0`, with each curve every 0.05 from
Mach 0.1 to 2.0, wherever the curve reaches. Each point is at sea level in the 1976 standard
atmosphere, at the [Reynolds number](../glossary.md#reynolds-number) for its Mach number, as
RASAero II computes its exports. The designs and their inputs are those of the Mach 0.3 check
above. Bands are Niskanen's (Table 3.1): subsonic to Mach 0.8, transonic below 1.2, supersonic from
1.2. The fixture, [`rocketpy-drag-curves.json`][curves-fixture], holds hpr's value and the error
at every Mach number and each band's summary. It doesn't hold the curves, but the two numbers
give a curve's value back at each Mach number.
`drag::tests::supersonic_cd_against_rasaero_tables` recomputes every row and pins the counts
([Loft lesson L18](../decisions-and-roadmap.md#l18): Loft, the earlier simulator this project
learns from, used an invented transonic drag curve).

The curves don't all reach Mach 2. Calisto's, the one real RASAero II export, does. Juno III's is
hand-edited past Mach 0.92: it climbs a constant step per row to Mach 1.0 and then drops to 0.001,
so the comparison stops at 0.92. Cavour's stop below Mach 0.93, and Valetudo's at 1.53.

Rows within 10%, and the range of the errors, by band:

| case | to Mach | subsonic, to 0.8 | transonic | supersonic, from 1.2 |
|---|---|---|---|---|
| Calisto, 2018 fins | 2 | 15 of 15: +3.9% to +8.9% | 3 of 7: −10.1% to +16.4% | 8 of 17: −14.9% to −5.1% |
| Calisto, getting-started fins (variant) | 2 | 12 of 15: −8.2% to +30.7% | 0 of 7: +13.5% to +65.4% | 0 of 17: +22.6% to +31.7% |
| Juno III | 0.9 | 15 of 15: −6.2% to +9.1% | 0 of 2: +19.3% to +32.0% | — |
| Cavour, power-off | 0.85 | 6 of 15: −12.6% to −2.2% | 0 of 1: −12.6% | — |
| Cavour, power-on | 0.9 | 1 of 15: −26.2% to −9.2% | 0 of 2: −27.0% to −26.7% | — |
| Valetudo, power-off | 1.5 | 0 of 15: −51.4% to −43.5% | 0 of 7: −53.3% to −50.9% | 0 of 7: −54.6% to −52.9% |
| Valetudo, power-on | 1.5 | 0 of 15: −55.6% to −46.6% | 0 of 7: −57.6% to −54.7% | 0 of 7: −57.8% to −56.2% |

The getting-started fins are a variant: RocketPy's getting-started example gives Calisto larger
fins with a thick NACA 0012 airfoil, but the export was made for the 2018 fins, whose normal
force it matches. So the variant's rows show how much the fins move drag, not a second
agreement: its thick fins now read 23% to 32% high faster than sound. Calisto on its 2018 fins is
within 10% up to Mach 0.8, at 0.95 and 1.0 and from 1.2 to 1.55; it reads +13.0% and +16.4% at
Mach 0.85 and 0.9, where its boattail's rise starts sooner than RASAero II's, −10.1% at Mach 1.05,
and falls below the curve from Mach 1.6, to −14.9% at 2.0. Valetudo's curve is
1.44 times its own OpenRocket export, as the Mach 0.3 check found, and Cavour's power-on miss is
the same open question. Two misses are unexplained: Cavour's power-off curve rises faster than
hpr's through subsonic flow, from −8.3% at Mach 0.3 to −12.6% at 0.85; and Juno III's stays flat
up to Mach 0.91, where hpr's has begun its rise, its nose's and its boattail's, +19.3% at Mach
0.85 and +32.0% at 0.9.

**What the boattail's wave drag changed.** Calisto ends in a short, steep conical
[boattail](../glossary.md#boattail): 0.47 calibres long, narrowing to 69% of the diameter, a
slope of 18.4°. Until [M1.8b3](../decisions-and-roadmap.md#m1-8b3) hpr gave it only a share of
the base drag, 0.083 at Mach 1.2 and 0.050 at 2.0, and read −29.8% to −24.4% from Mach 1.2. Its
supersonic wave drag ([Boattails faster than sound](#boattails-faster-than-sound)) is 0.283 at
Mach 1.2, 0.190 at 1.5 and 0.120 at 2.0, and its base drag falls by a third; together they close
the gap from 0.204 to 0.035 at Mach 1.2 and from 0.128 to 0.077 at 2.0. RASAero II lists such drag as its own
term, "other body wave" drag. Calisto's 18.4° boattail is steeper than any attached boattail
measured here, and 16° boattails read 26% to 54% high, so this agreement is not support for the
model at that angle. What is left grows with Mach number, and part of it is hpr's body,
which reads 6% to 10% low faster than sound against a worked example with every input known
([below](#drag-against-mil-hdbk-762s-sample-calculation)).

**The unrecorded inputs now span most of the rest.** Calisto's fins could be square, rounded or an
airfoil, 2 to 6.35 mm thick, smooth or painted (`tests::calistos_rows_by_fin_and_finish`). The
committed inputs, square, 3 mm and smooth by the rule of the Mach 0.3 check, have 15, 3 and 8
rows within 10% by band. Rounded fins 4.76 mm thick, smooth, have 15, 4 and 14; airfoil fins
6.35 mm thick, smooth, have 11, 4 and 17. No combination has every row within 10%. Before the
wave drag no combination had rows within 10% both below Mach 0.8 and from Mach 1.2. So most of
what is left is within what the unrecorded inputs span; hpr keeps the stated rule rather than
picking the inputs that fit.

### Drag against MIL-HDBK-762's sample calculation

RASAero II's curves can't show which way hpr leans, because their inputs are guessed. A reference
with every input known can. MIL-HDBK-762, the U.S. Army's handbook for designing unguided rockets,
works one rocket's drag through by its own methods, term by term, from Mach 0.5 to 3.2 ([762]
Table 5-4, pp. 5-58 to 5-66). The rocket is 3.84 m long and 0.16 m across. It has a 3-calibre
[tangent ogive](../glossary.md#tangent-ogive) nose, a plain cylinder with no boattail, and four
fins 0.32 m long, 51 mm tall and 6.4 mm thick, flush with the base (Fig. 5-155).

This is a calculation, not a measurement: it checks hpr's methods against another set of
methods, whose base drag comes from measured bases. The table is transcribed with its pages in
[`mil-hdbk-762-sample-drag.json`][handbook-fixture], and every row sums to its printed total.
The rocket is `validation/designs/mil-hdbk-762-sample-rocket.json`, with a smooth finish, as the
handbook's friction is. hpr flies it at the table's Reynolds numbers.
`tests::drag_against_mil_hdbk_762_sample` recomputes the comparison in
[`drag-vs-mach.json`][drag-fixture] and pins the rows within 10%. The 10% target comes from
[M1.8](../decisions-and-roadmap.md#m1-8). hpr's numbers for this rocket were seen before it was
chosen as a reference, so this is not a blind test.

**The fins are left out.** The handbook draws each fin as a single wedge, sharp at the leading
edge and blunt at the trailing edge, and gives the fins a thin wedge's wave drag and the base drag
of their trailing edges. hpr has no such section. The design gives them square edges, whose
leading edges hpr charges the pressure of air brought to a stop against them (0.100 at Mach 2,
against the handbook's 0.016 for the whole fin). So each side's fin pressure drag is shown but
left out of the totals compared; the fins' friction stays in.

Each term, the handbook's first and then hpr's, on the reference area:

| Mach | handbook | hpr | error | nose (handbook, hpr) | base | friction | fins, left out (handbook, hpr) |
|---|---|---|---|---|---|---|---|
| 0.5 | 0.423 | 0.379 | −10.3% | 0.000, 0.000 | 0.170, 0.152 | 0.253, 0.227 | 0.023, 0.069 |
| 0.7 | 0.405 | 0.401 | −1.1% | 0.000, 0.006 | 0.163, 0.184 | 0.242, 0.211 | 0.023, 0.075 |
| 0.9 | 0.393 | 0.484 | +23.1% | 0.007, 0.062 | 0.156, 0.225 | 0.230, 0.197 | 0.023, 0.082 |
| 0.95 | 0.404 | 0.533 | +31.9% | 0.011, 0.102 | 0.163, 0.237 | 0.230, 0.194 | 0.026, 0.085 |
| 1.0 | 0.465 | 0.609 | +31.0% | 0.052, 0.164 | 0.183, 0.250 | 0.230, 0.195 | 0.043, 0.087 |
| 1.1 | 0.542 | 0.651 | +20.1% | 0.109, 0.234 | 0.215, 0.227 | 0.218, 0.189 | 0.043, 0.089 |
| 1.2 | 0.528 | 0.593 | +12.3% | 0.117, 0.200 | 0.194, 0.208 | 0.217, 0.184 | 0.036, 0.091 |
| 1.6 | 0.472 | 0.444 | −6.0% | 0.109, 0.123 | 0.168, 0.156 | 0.195, 0.165 | 0.022, 0.097 |
| 2.0 | 0.415 | 0.377 | −9.2% | 0.095, 0.104 | 0.147, 0.125 | 0.173, 0.147 | 0.016, 0.100 |
| 2.4 | 0.363 | 0.331 | −8.9% | 0.089, 0.094 | 0.124, 0.104 | 0.150, 0.132 | 0.013, 0.102 |
| 2.8 | 0.328 | 0.296 | −9.6% | 0.085, 0.088 | 0.106, 0.089 | 0.137, 0.119 | 0.011, 0.103 |
| 3.2 | 0.298 | 0.269 | −9.6% | 0.083, 0.084 | 0.089, 0.078 | 0.126, 0.107 | 0.009, 0.103 |

Six of twelve rows are within 10%. hpr reads +12.3% to +31.9% high from Mach 0.9 to 1.2, and
−6.0% to −9.6% low from Mach 1.6:

- **The nose through Mach 1.** Niskanen's ogive gives two to three times the handbook's: 0.164
  against 0.052 at Mach 1.0, and 0.234 against 0.109 at 1.1. Stoney's measured 3:1 cone also sits
  under Niskanen's closed form through the rise ([Drag through Mach 1](#drag-through-mach-1)).
  From Mach 2 the two agree within 10%.
- **The base.** Niskanen's base drag (eq. 3.94, after Fleeman's missile design textbook) gives
  0.250 at Mach 1.0 where the handbook reads 0.183 from measured bases. Faster than sound hpr's is
  the lower: 0.125 against 0.147 at Mach 2.
- **Friction** reads 10.4% to 15.9% lower in hpr. The handbook takes a smooth flat plate's
  friction and adds 15% on the body; hpr's body factor ([N09] eq. 3.85) adds 2% for this slender
  body, which accounts for about 11 points. The rest is unexplained; the two methods correct
  friction for Mach number differently.

So hpr's body reads high through Mach 1, from the nose and the base, and 6% to 10% low faster than
sound, from friction and the base. That is the same sign as Calisto's gap to RASAero II, a third
of its size. This rocket has no boattail, so it says nothing about a boattail's own drag.

[adr-008]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-008-subsonic-normal-force-and-centre-of-pressure-2026-09-17
[adr-026]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18
[adr-027]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-027-the-normal-force-through-mach-1-supersonic-linear-theory-a-transonic-join-and-the-measured-references-2026-09-18
[adr-009]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17
[adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18
[wind-tunnel]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-wind-tunnel.json
[drag-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/drag-vs-mach.json
[curves-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/rocketpy-drag-curves.json
[handbook-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/mil-hdbk-762-sample-drag.json
[adr-029]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-029-drag-against-rasaero-ii-through-mach-2-the-gap-by-band-mil-hdbk-762s-sample-calculation-and-the-boattails-wave-drag-2026-09-18
[adr-030]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-030-the-afterbody-faster-than-sound-a-boattails-wave-drag-the-base-behind-it-and-a-lip-in-its-wake-2026-09-18
[boattail-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/measured-boattails.json
