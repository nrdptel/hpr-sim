# Aerodynamics

## In short

- **What it models:** the air's forces on a rocket: the
  [normal force](../glossary.md#normal-force) (the sideways push when flying at an angle to the
  airflow) and the [centre of pressure](../glossary.md#centre-of-pressure-cp) (where it acts) from
  [Mach](../glossary.md#mach-number) 0 to 5, drag over the same range, and the rolling moment
  from canted fins and the roll rate. A flight can also take another program's drag, or its
  normal force and centre of pressure, in place of hpr's own
  ([The normal force from RASAero II](#the-normal-force-from-rasaero-ii)).
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
    from Mach 0.6 to 4.63 against the same wind tunnel: from Mach 1.5 the slope within +9.4% to
    −3.3% and the centre of pressure within 0.53 [calibres](../glossary.md#calibre-caliber) (the
    long model misses the half-calibre target at Mach 1.8 and 2.3, where its body reads high), and
    between Mach 0.8 and 1.2 both miss
    ([Normal force through Mach 1](#normal-force-through-mach-1)).
  - *The body faster than sound*, by the method a flight blends in over 0.3 in Mach from Mach 1.2
    at the earliest, on the bodies it covers: against its report's wind-tunnel measurements of 120
    cone- and ogive-cylinders from Mach 3 to 6.28, 117 slopes within ±0.2 per radian and 109
    centres of pressure within 0.2 calibres. Against the Arcas Robin wind tunnel's body alone
    (its two lengths, the short model and the long), with a pointed nose fitted to its shape and
    the lip behind its boattail left off
    ([Checking the shock-expansion method](#checking-the-shock-expansion-method)):
    - *Like for like*, fitted at the tunnel's angles with the [body lift](../glossary.md#body-lift)
      a flight adds, the body with its boattail reads +3.4% to +41.0%, within 15% from Mach 3.96
      (before [M1.8e6](../decisions-and-roadmap.md#m1-8e6) sized body lift and the boattail's
      share, 14.9% to 73.2% high). At its 62 angles from 5.5° to 21.7°, 48 are within 15%.
    - *The method alone at `α → 0`* against the tunnel's line, which includes crossflow: the nose
      and cylinder of the short model within 5% from Mach 1.8 to 2.96, and −15.0% and −18.7% past
      Mach 3; the long model −13.7% to −26.4% throughout.

    hpr's committed Arcas Robin designs fly that method to their base since
    [M1.8e8](../decisions-and-roadmap.md#m1-8e8); like for like, the short model's body reads 3.02
    to 3.95 per radian from Mach 1.5, where the tunnel reads 2.19 to 4.15
    ([Normal force through Mach 1](#normal-force-through-mach-1)).
  - *In whole flights* in wind, body lift, which RocketPy leaves out, is the largest reason a slow
    rocket's drift differs from RocketPy's ([ADR-026][adr-026]). Nothing against a real flight.
  - *Roll*: the spin that [canted](../glossary.md#cant) fins give, against NASA's measured roll
    effectiveness (the rolling moment per degree of cant), is within 5.3% at all 8 readings from
    Mach 2.3 to 4.63, and 14.3% to 47.8% high at Mach 1.5 and 1.8; the roll damping reads 5.9% to 16.2% low against the one measured
    set, that of the Basic Finner, a standard finned test body, from Mach 1.5 to 3
    ([Roll against the Arcas Robin and the Basic Finner](#roll-against-the-arcas-robin-and-the-basic-finner)).
  - *Another program's normal force*, read from RASAero II's export: every row of Calisto's
    export comes back from hpr's table, and a flight on a table swings in pitch as the equations
    predict; no real export has flown faster than Mach 0.75
    ([The normal force from RASAero II](#the-normal-force-from-rasaero-ii)).
- **What it leaves out:** large angles and [stall](../glossary.md#stall), though a flight uses
  these models at every angle. Faster than sound
  ([transonic and supersonic](../glossary.md#transonic-and-supersonic)), a steep boattail's drag
  in a thick boundary layer reads high, and nothing corrects for it; the fins' drag takes a
  blunt edge's formula, which reads far high for thin, sharp fins, and nothing models a thin fin's
  own wave drag or the drag where fins meet the body. Faster than sound a flight takes a pointed
  nose and its cylinder from the method that adds the cylinder's lift, and a boattail behind them
  from a measured correlation of boattails of 4° to 9.5°, an extrapolation for steeper ones,
  which hpr stops reading past the angle where the flow separates
  ([The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight)). A nose
  with a vertical tip (power-series, Haack, elliptical) takes a Newtonian cap ahead of the method,
  checked on a sphere-cone only ([Blunt tips](#blunt-tips)), and a lip inside a boattail's wake
  carries nothing ([A lip in a boattail's wake](#a-lip-in-a-boattails-wake)). A **conical**
  [flare](../glossary.md#flare) flush with the tube ahead of it flies the method too, and ends the
  run, while its corner's shock stays attached — checked against **one** measured flare, where it
  reads −1.9% and +7.0% at Mach 1.9 and 2.3, +13.4% at 2.96, then +51.5% and +50.4% at 3.95 and
  4.63 ([What a marched flare is worth](#what-a-marched-flare-is-worth)); a flare shallow enough
  to turn the flow almost not at all is read by an older, rougher method instead, which nothing
  measures ([A near-flat flare](#a-near-flat-flare)). A rocket with any other
  widening shape, or any step, behind the nose (a motor retainer behind a step down counts: the
  step ends the run) keeps slender-body theory for its whole body at every speed, which reads low
  past Mach 3 ([A step in radius](#a-step-in-radius): −8.65% and 1.03 calibres at the threshold). Body lift leaves out
  the fall in crossflow drag past the critical crossflow Reynolds number
  ([Body lift](#body-lift)), and it reads too large at the few degrees a slope is fitted over: the
  body alone misses the 15% target the milestone set on six of eleven wind-tunnel rows, by +37.7%
  at worst and within 5% at Mach 3.96 and 4.63
  ([The body alone, against the 15% target](#the-body-alone-against-the-15-target)). There are no damping coefficients for pitch and
  yaw: a flight takes that damping from each part's own local flow. The roll forcing near Mach
  1.5 reads high, and nothing measured checks roll below it
  ([Roll: forcing and damping](#roll-forcing-and-damping)).

## Code and sources

Code: [`hpr_aero::body`](../api/hpr_aero/body/index.html) (bodies of revolution),
[`hpr_aero::crossflow`](../api/hpr_aero/crossflow/index.html) (body lift),
[`hpr_aero::fins`](../api/hpr_aero/fins/index.html) (fin sets),
[`hpr_aero::nose_drag`](../api/hpr_aero/nose_drag/index.html) (noses' drag through Mach 1),
[`hpr_aero::afterbody`](../api/hpr_aero/afterbody/index.html) (boattails faster than sound),
[`hpr_aero::shock_expansion`](../api/hpr_aero/shock_expansion/index.html) (the body faster than
sound), [`hpr_aero::blunt_tip`](../api/hpr_aero/blunt_tip/index.html) (a blunt tip's cap and its
handover),
[`hpr_aero::table`](../api/hpr_aero/table/index.html) (tables from other programs) and
[`hpr_aero::model`](../api/hpr_aero/model/index.html) (a whole rocket's terms, built from its
[`Layout`](../api/hpr_design/tree/struct.Layout.html)). Decisions: [ADR-008][adr-008] (normal force
and centre of pressure) and [ADR-009][adr-009] (drag). The milestone [M1.5a](../decisions-and-roadmap.md#m1-5a) covers the
subsonic normal force and centre of pressure, [M1.5b](../decisions-and-roadmap.md#m1-5b) the subsonic drag and override
tables; [M1.8a](../decisions-and-roadmap.md#m1-8a) the normal force through Mach 1
([ADR-027][adr-027]); [M1.8b1](../decisions-and-roadmap.md#m1-8b1) the drag through Mach 1
([ADR-028][adr-028]); [M1.8b3](../decisions-and-roadmap.md#m1-8b3) boattails faster than sound
([ADR-030][adr-030]); [M1.8c](../decisions-and-roadmap.md#m1-8c) roll ([ADR-031][adr-031]);
[M1.8d](../decisions-and-roadmap.md#m1-8d) the normal force from RASAero II
([ADR-032][adr-032]); [M1.8e1](../decisions-and-roadmap.md#m1-8e1) the body faster than sound
([`hpr_aero::shock_expansion`](../api/hpr_aero/shock_expansion/index.html), [ADR-033][adr-033]),
flown from [M1.8e2](../decisions-and-roadmap.md#m1-8e2),
[M1.8e4](../decisions-and-roadmap.md#m1-8e4) the boattail's share of it, and
[M1.8e6](../decisions-and-roadmap.md#m1-8e6) body lift's size at every speed and the boattail's
measured share ([`hpr_aero::supersonic_boattail`](../api/hpr_aero/supersonic_boattail/index.html),
[ADR-037][adr-037]). The rest of transonic and supersonic flow arrives with the rest of
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
- **[SD56]** C. A. Syvertson and D. H. Dennis, *A Second-Order Shock-Expansion Method Applicable
  to Bodies of Revolution Near Zero Lift*, NACA TN 3527, 1956 (also NACA Report 1328;
  `naca-tn-3527-syvertson-dennis-1956`).
- **[S64]** J. L. Sims, *Tables for Supersonic Flow Around Right Circular Cones at Small Angle of
  Attack*, NASA SP-3007, 1964 (`nasa-sp-3007-sims-1964-cones-small-alpha`).
- **[J77]** L. H. Jorgensen, *Prediction of Static Aerodynamic Characteristics for Slender Bodies
  Alone and With Lifting Surfaces to Very High Angles of Attack*, NASA TR R-474, 1977
  (`nasa-tr-r-474-jorgensen-1977`).
- **[WP68]** W. D. Washington and W. Pettis Jr., *Boattail Effects on Static Stability at Small
  Angles of Attack*, U.S. Army Missile Command report RD-TM-68-5, 1968
  (`washington-pettis-1968-rd-tm-68-5`).
- **[J68]** C. M. Jackson Jr., W. C. Sawyer and R. S. Smith, *A Method for Determining Surface
  Pressures on Blunt Bodies of Revolution at Small Angles of Attack in Supersonic Flow*, NASA TN
  D-4865, 1968 (`nasa-tn-d-4865-jackson-1968`).
- **[S62]** A. Seiff, *Secondary Flow Fields Embedded in Hypersonic Shock Layers*, NASA TN D-1304,
  1962 (`nasa-tn-d-1304-seiff-1962`).
- **[R22]** C. E. Rogers, *RASAero II Comparisons with ARCAS Center of Pressure (CP) and Drag
  Coefficient (CD) Wind Tunnel Data*, Rogers Aeroscience, 2022 (slides).
- **[RAS]** C. E. Rogers and D. Cooper, *Rogers Aeroscience RASAero II Aerodynamic Analysis and
  Flight Simulation Program Users Manual*, version 1.0.2.0, 2019.

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

Body lift is the extra push of the air crossing the body at larger angles of attack. It grows with
`sin² α`, so it is zero at `α = 0` and adds nothing to the slope there. Its size is in
[Body lift](#body-lift), below.

| term | formula | source |
|---|---|---|
| slope | `(C_Nα)_B = (2/A_ref)[A(l) − A(0)] · sin α/α` | [B66] eq. 10, [B67] eq. 3-65, [N09] eq. 3.19 |
| CP, aft of the fore end | `X_B = [l A(l) − V] / [A(l) − A(0)]` | [B66] eq. 28, [B67] eq. 3-89, [N09] eq. 3.28 |
| moment slope | `(2/A_ref)[l A(l) − V] · sin α/α` | [N09] eq. 3.25 |
| body lift | `C_N = η C_dn (A_plan/A_ref) sin² α`, at the planform centroid ([Body lift](#body-lift)) | [J77] eq. 2.12 |

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
- Slender-body theory's slope has no Mach term: [B67] p. 18 leaves body compressibility out as a
  conservative choice, and [N09] p. 22 takes the body's normal force as the same at all speeds.
  Faster than Mach 1.2 a flight can use another method
  ([The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight)).
- Body lift is zero at `α = 0`, so Barrowman's worked examples don't test it.

### Body lift

*Changed in [M1.8e6](../decisions-and-roadmap.md#m1-8e6)* ([ADR-037][adr-037]): until then hpr
used Galejs's constant, `C_N = K (A_plan/A_ref) sin² α` with `K` = 1.1 at every speed.

What this covers: the size of body lift, the sideways push of the air crossing a body at an angle
of attack. How far to trust it: it is Jorgensen's method ([J77]) with hpr's own way of combining
two of his figures (below). Against NASA's Arcas Robin body alone at the 62 angles the wind tunnel
plotted from 5.5° to 21.7°, from Mach 1.5 to 4.63, hpr's normal force with it is within 15% at 48
(34 with Galejs's constant); where the air crosses the body faster than sound, it reads 1% to 16%
high. Below Mach 1 the only check is that body's slope fitted from −4° to +4°, where body lift
adds a little: at Mach 0.6 hpr's body reads 25% high (41% with Galejs's constant), against
readings the tunnel determines poorly. No real flight checks it, so whether it is better than
Galejs's constant for a slow rocket leaving the rail in wind, where it matters most, is open until
[M2.3](../decisions-and-roadmap.md#m2-3).

At an angle of attack `α` the air meets the body partly from the side, at `V sin α`. Behind a
long cylinder in a cross-wind the air separates, and the drag of that separated flow pushes the
body sideways. Jorgensen sizes body lift from that drag: `C_dn`, the drag coefficient of an
infinitely long circular cylinder in the crossflow, and `η`, the ratio of a finite cylinder's to
an infinite one's. Both depend on the *crossflow Mach number* `M_n = M sin α`, the Mach number of
the air crossing the body; `η` also on the body's [fineness](../glossary.md#fineness-ratio) `f`,
its length over its largest diameter.

| term | formula or value | source |
|---|---|---|
| body lift | `C_N = η C_dn (A_plan/A_ref) sin² α`, at each part's planform centroid | [J77] eq. 2.12, p. 10 |
| crossflow Mach number | `M_n = M sin α` | [J77] eq. 2.3, p. 8 |
| `C_dn` | 1.20 up to `M_n` 0.2, rising to 1.334 at 0.5 and 1.985 at 1.0, then falling to 1.266 by 4.8 | [J77] Fig. 1, p. 75 |
| `η` at low `M_n`, `η₄(f)` | 0.577 at `f` = 2, 0.685 at 10, 0.753 at 20, 0.815 at 40 | [J77] Fig. 4, p. 77 |
| `η` with `M_n`, `η₆` | 0.69 at 0; 0.717, 0.804, 0.815, 0.845, 0.994, 0.979, 0.769, 0.910 and 0.937 at 0.4 to 1.2 in steps of 0.1; 0.985 at 1.4; 0.984 at 1.6 | [J77] Fig. 6, p. 78 |
| `η` for fineness `f` | `η = η₆ [η₄(f) + (1 − η₄(f)) r] / [0.69 + 0.31 r]`, `r` the most `s = (η₆ − 0.69)/0.31` has reached up to that `M_n` | hpr's, a judgement |

- **`C_dn`** is Jorgensen's value below the critical crossflow Reynolds number, where the air
  separates from a smooth cylinder early: "C_dn = 1.2" at low speed (p. 15). From `M_n` 0.6 to 1.2
  hpr takes the points Jorgensen marks as extrapolated from NASA Ames wind-tunnel data, and from
  1.4 his curve through the supersonic experiments.
- **`η`.** (Jorgensen's; the shock-expansion method below uses `η` for something else.) Fig. 4
  gives `η` against length for cylinders measured only at low speed. Fig. 6 gives
  how `η` grows toward 1 as the crossflow speeds up, but only for the two bodies (fineness 10 and
  12) it was computed from: Jorgensen divided the `η C_dn` those bodies' measured normal force
  gives (his Fig. 5) by Fig. 1's `C_dn`. For any other fineness hpr scales Fig. 6's `η` by how
  much Fig. 4 changes it for the body's length, and lets that scaling fade by the share `s` Fig. 6
  has risen toward 1. The share it uses, `r`, never falls back: Fig. 6 dips at `M_n` = 1 only
  because Jorgensen divided by Fig. 1's peak there, not because length counts again. Below `M_n`
  0.8, where Fig. 6 only rises, this is `η = η₄ + (1 − η₄) s`; past 0.8 every fineness takes
  Fig. 5's `η C_dn` to within 0.3%. That rule is hpr's; it gives Fig. 6 back for a body of
  fineness about 10.6 and stays below 1.
- **Sampled, not smoothed.** Near `M_n` = 1 Fig. 1's `C_dn` peaks and Fig. 6's `η` dips, each
  steeply. hpr reads both at the eleven crossflow Mach numbers Jorgensen computed Fig. 6 at and
  interpolates between them, so their product is his own `η C_dn` there: within 3% of his Fig. 5
  at all ten of its points from 0.5 to 1.6 (test `the_product_follows_figure_5`).

**A worked example.** NASA's short Arcas Robin model without fins, fineness 18.18, has a planform
of 20.79 times its cross-section. At Mach 2.3 and `α` = 12.56°, `M_n` = 2.3 × sin 12.56° = 0.50.
Fig. 4 gives `η₄` = 0.742; Fig. 6 gives `η₆` = 0.804, so `s` = (0.804 − 0.69)/0.31 = 0.368 and
`η` = 0.742 + 0.258 × 0.368 = 0.837. With `C_dn` = 1.334, `η C_dn` = 1.117, and body lift is
1.117 × 20.79 × sin² 12.56° = 1.10. Adding the attached-flow part (the method's slope at small
angles, without crossflow), hpr's body gives `C_N` 1.632 there; the tunnel measured 1.630. This
point happens to agree closely; across the 62 points the spread is −22.3% to +31.7% (table below).

**What changes in a flight.** At the low crossflow speeds of most flights `η C_dn` is `1.2 η₄(f)`:
0.82 at fineness 10, 0.90 at 20, 0.95 at 30, against Galejs's 1.1. A slow rocket leaving the rail
in wind feels it most ([Validity and open questions](#validity-and-open-questions)). The tunnel
points where hpr reads high, the air crossing the body faster than sound, are at 12° to 21° from
Mach 2.3 up; a flight meets such angles that fast only if it is unstable or hit by a strong gust.

**How it was checked.** The Arcas Robin wind tunnel measured the body alone from −5° to 21° at six
Mach numbers ([TN D-4014](#code-and-sources); the points above +4° were read for this milestone
into
[`arcas-robin-high-alpha.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-high-alpha.json)).
[`arcas-robin-crossflow.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-crossflow.json)
compares hpr's body at each point, with the pointed nose fitted to the tunnel's and the lip left
off ([Checking the shock-expansion method](#checking-the-shock-expansion-method)):

| crossflow Mach number `M_n` | points | hpr's `C_N` against the tunnel's | with Galejs's `K` = 1.1 |
|---|---|---|---|
| under 0.45 | 21 | −6.9% to +31.7% | +1.9% to +61.4% |
| 0.45 to 0.95 | 25 | −22.3% to +13.0% | −21.1% to +17.0% |
| 0.95 and over | 16 | +0.8% to +16.5% | −21.7% to −7.1% |

At the lowest crossflow speeds the readings can't say how much of what is left is body lift and
how much the slope at `α → 0` ([Checking the shock-expansion method](#checking-the-shock-expansion-method)).
Where the air crosses faster than sound, hpr's `η C_dn` (1.45 to 1.61) is above what the tunnel's
points need (1.26 to 1.54). Jorgensen's `η C_dn` was worked out from measured normal force less
his own attached-flow term, `sin 2α cos(α/2)`; hpr pairs it with its own, `sin α` times its slope,
which is 8% larger at 20° and, faster than sound, carries the method's slope for the nose and
cylinder, 2.55 to 3.40 against slender-body theory's 2. Two cautions from Jorgensen: his `η` comes from cylinders measured "only at
very low subsonic Mach numbers" (p. 17), and the tunnel tripped its boundary layer, which can
move the flow past the critical crossflow Reynolds number, where `C_dn` falls to "between about
0.15 and 0.30" (p. 15).

**What it leaves out.** That fall past the critical crossflow Reynolds number (about 2 × 10⁵,
[J77] Fig. 2), which Jorgensen computes only for illustration; roughness and fins' effect on the
body's crossflow; and Jorgensen's own attached-flow term, `sin 2α cos(α/2)` in place of hpr's
`sin α`.
Galejs's constant stays available for comparison
([`BodyLift::Galejs`](../api/hpr_aero/crossflow/enum.BodyLift.html), with
[`AeroModel::with_body_model`](../api/hpr_aero/model/struct.AeroModel.html#method.with_body_model)):
[G] cites Hoerner's 1.1 to 1.5 and fitted 1.0 to his own data.

### Bodies faster than sound

*Flown faster than sound for a pointed nose and its cylinder* (see
[The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight), below). This is
[M1.8e1](../decisions-and-roadmap.md#m1-8e1)'s method, the second-order shock-expansion method as a
tested library model. Against its report's wind-tunnel data, 117 of
120 slopes are within 0.2 per radian. On the Arcas Robin's body it reads from 16% high at Mach 1.5
to 27% low past Mach 3 at `α → 0`, a comparison that leaves out the
[body lift](../glossary.md#body-lift) a flight adds; compared the way the tunnel measures, the
body reads high
([Checking the shock-expansion method](#checking-the-shock-expansion-method), under
Verification). It needs a pointed tip; a blunt or vertical one (power-series, elliptical and
Haack noses) takes a Newtonian cap ahead of it ([Blunt tips](#blunt-tips), below).

Slender-body theory, above, gives a pointed nose a slope of 2 and a cylinder none, at any speed.
Faster than sound that is too little. The air speeds up around the shoulder where the nose meets
the cylinder, and its pressure then recovers along the cylinder toward the free stream's. At an
angle of attack it recovers unevenly around the body, so the cylinder carries lift too, more the
longer it is. NASA measured the Arcas Robin's body alone at 3.9 per radian at Mach 3.96, where
slender-body theory gives 2 for its nose.

hpr computes that lift by Syvertson and Dennis's *second-order shock-expansion method* ([SD56]),
for a body with a pointed tip and supersonic flow everywhere on it, as the slope at small angles
of attack (`α → 0`). The older *generalized* shock-expansion method holds the pressure constant
along each straight piece of the profile; the second-order method also carries the pressure's
rate of change across each corner, so the pressure can recover along a piece:

1. **The tangent body.** The profile becomes straight elements, each tangent to it: the first at
   the tip, the rest at equal steps along a curved nose (ten per curved piece, the report's own
   choice), one per cone or cylinder. Where two elements meet is a corner.
2. **The tip** is a cone, so its flow is exactly a cone's, found by integrating the
   Taylor–Maccoll equation (the exact equation of supersonic flow over a cone) from the shock to
   the surface ([R1135] eq. 177). The shock must be *attached*, touching the tip, which holds up
   to a half-angle that grows with the Mach number. For cones under 0.03°, where that equation
   can't be integrated, hpr takes slender-cone linear theory, blended in up to 0.06°.
3. **Around each corner** the flow turns by a
   [Prandtl–Meyer expansion](../glossary.md#prandtlmeyer-expansion).
4. **Along each element** the pressure relaxes from its value behind the corner toward the
   pressure on the element's *tangent cone*: the cone, pointed into the oncoming flow, whose
   surface has the body's local slope there. The lift per unit length relaxes the same way,
   toward that cone's.
5. **The slope and CP** follow by adding the lift over the body.

| step | formula | source |
|---|---|---|
| pressure along an element | `p = p_c − (p_c − p₂) e^(−η)`, `η = (∂p/∂s)₂ (x − x₂) / ((p_c − p₂) cos δ₂)` | [SD56] eqs. 8, 9 |
| gradient behind a corner | `(∂p/∂s)₂ = (B₂/r)(Ω₁/Ω₂ sin δ₁ − sin δ₂) + (B₂Ω₁/B₁Ω₂)(∂p/∂s)₁`, `B = γpM²/(2(M² − 1))` | [SD56] eqs. 4, 6 |
| gradient at an element's end | `(∂p/∂s)₃ = (p_c − p₃)/(p_c − p₂) · (∂p/∂s)₂` | [SD56] eq. 10 |
| lift per unit length | `Λ = (1 − e^(−η)) tan δ · (dC_N/dα)_tc + (λ₂/λ₁) e^(−η) Λ₁`, `λ = 2γp / sin 2μ` | [SD56] eqs. 5, 19 |
| slope and CP | `C_Nα = (2π/A_ref) ∫ Λ r dx`, `x_cp = ∫ Λ r x dx / ∫ Λ r dx` | [SD56] eqs. 14, 21 |

Here `δ` is an element's angle to the axis, `p` the pressure over the free stream's (the
undisturbed air ahead of the rocket), `M` the Mach number at the surface, `r` the radius at the
corner, `Ω` how much a thin tube of flowing air widens as its Mach number rises (its area over
its area at Mach 1, [SD56] eq. 7), `μ` the Mach angle `asin(1/M)`, and `(dC_N/dα)_tc` the slope
of the tangent cone, digitised by hand from the report's Fig. 2 into a table and continued past
that chart's 24° by Sims's own tables of the same theory, to 30° ([ADR-042][adr-042],
[`cone_normal_force_slope`](../api/hpr_aero/shock_expansion/fn.cone_normal_force_slope.html)).
For a worked example with numbers, see
[Checking the shock-expansion method](#checking-the-shock-expansion-method).

- **A cylinder's tangent cone** is the free stream, so its lift decays to nothing along it.
- **A boattail has no tangent cone.** The report's footnote 8 takes the free stream's pressure
  and a slope of 2, "reasonable results for bodies having moderate amounts of boattail". The
  method does the same, but a measurement says it takes too little lift off: since
  [M1.8e6](../decisions-and-roadmap.md#m1-8e6) a flight gives a boattail a measured share instead
  ([The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight)).
- **Where the method stops.** The relaxation holds only where the gradient behind a corner points
  toward the tangent cone's pressure (`η ≥ 0`, [SD56] p. 13). The report states that as a
  condition and doesn't say how it went on where it fails, near a sharp tip at high Mach number.
  hpr's own reading is to reduce such an element to the older *generalized* method, which the
  report says the equations become at `η = 0`: the pressure stays as it is along the element and
  no gradient passes to the next corner. On the report's fineness-3 ogive at Mach 5.05 that
  departs from its values ([issue #81: the method's limit near a sharp
  tip](https://github.com/nrdptel/hpr-sim/issues/81)). hpr reads an element that way wherever it
  has a tangent cone of its own, behind the nose as well as on it
  ([A near-flat flare](#a-near-flat-flare)). A cylinder's tangent cone is the free stream and a
  boattail's is footnote 8's, so neither is a solution of that element's own flow: one of those
  that would need reducing is refused instead, and the whole body keeps slender-body theory
  ([issue #123: a cylinder's or a boattail's reduced
  element](https://github.com/nrdptel/hpr-sim/issues/123)).
- **Mach number over nose fineness.** The report states the method for 0.4 to 2; hpr doesn't
  enforce it (the report's own Mach 6.28 rows are at 2.09, and the Arcas Robin at Mach 1.5 is at
  0.36).
- **Its range.** The report states the method for Mach number over nose fineness from 0.4 to 2,
  within ±0.2 per radian and ±0.2 calibres of its measurements. Fig. 2 covers Mach 3 to 10; below
  Mach 3 hpr holds the Mach 3 curve, an assumption. The tip's shock must be attached, and the
  profile continuous. Tangent cones run to **30°**: to 24° from Fig. 2, and from there to 30° from
  NASA SP-3007's tables of the same theory, which agree with the chart to 0.0021 per radian where
  both cover the same angle ([ADR-042][adr-042]). A cone steeper than 30° is refused, and the
  whole body then keeps slender-body theory.
- **What it leaves out:** the crossflow lift that grows with `sin² α`
  ([Body lift](#body-lift), above), and anything viscous. It is the slope at small angles only.

### The body faster than sound in a flight

What this covers: how a flight uses the method above, from Mach 1.2, and a boattail's measured
share. How far to trust it: as far as the method's own checks above, for a pointed nose and
cylinder. A boattail behind them takes Washington and Pettis's measured increment ([WP68]), which
their data give within about 15% for conical boattails of 4° to 9.5°; a steeper, shorter, longer
or narrower one, like the Arcas Robin's 15°, is an extrapolation, as is a transition that isn't
conical (it takes the same correlation from its length and radii). A long boattail reads the
curve near zero argument, which comes from the report's lowest supersonic runs. Past 16°, where the flow
separates, hpr stops reading the correlation any steeper and holds it there
([A steep boattail reads the correlation no steeper than 16°](#the-body-faster-than-sound-in-a-flight),
[issue #90: how steep a boattail the correlation should cover](https://github.com/nrdptel/hpr-sim/issues/90));
nothing measures what such a boattail really carries, and the choice is worth 0.67 to 1.35
calibres of centre of pressure at 30°, most at the lowest speeds. A tube behind the boattail takes the method's decay of its expansion, which no
measurement here checks. A blunt or vertical nose tip takes a Newtonian cap ahead of the method
([Blunt tips](#blunt-tips)), an extrapolation from spherical caps, and a lip inside a boattail's
wake rides along carrying nothing ([A lip in a boattail's wake](#a-lip-in-a-boattails-wake)). A
conical flare flush with the part ahead of it flies the method while its corner's shock is
attached, and is read drawn out where it is not; **one** measured flare has been put beside it
([What a marched flare is worth](#what-a-marched-flare-is-worth)), and the drawn-out half still
has none. A rocket with any
other widening shape, or any step, behind the nose gets nothing from the method and keeps
slender-body theory, which on the Arcas Robin's body reads 15% to 50% below the tunnel faster than
sound. The join between the two models is a
judgement, not a measurement, and no validation flight goes past Mach 1.06 (Prometheus, the
fastest; see its `max_mach` rows in the
[validation report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md)),
so no flight checks it yet.

**Which model your rocket gets.** Every body takes [body lift](#body-lift) at every speed. Past
Mach 1.2, a rocket whose first body is a nose (pointed, or with a blunt or vertical tip that the
cap covers), followed only by tubes of its radius and boattails (and tubes behind those), takes
the method below for those parts, and each boattail its measured share. A lip wholly inside a
boattail's wake rides along, carrying nothing; one only partly in the wake gets the method in the
wake's own proportion, and slender-body theory for the rest
([A lip in a boattail's wake](#a-lip-in-a-boattails-wake)). A **conical flare** flush with the
part ahead of it joins the run as well, and ends it
([A flare through the method](#a-flare-through-the-method)); a widening part behind a boattail is
a lip, not a flare, and keeps the lip's rule. Anything else (a step, a widening shape that is not
a cone, a motor retainer behind a step down — the step itself ends the run — or a nose steeper
than the cap's handover all the way to its base) keeps slender-body theory for its whole body at
every speed.

**Where that choice still jumps.** It is one choice for the whole body, so wherever it turns on a
threshold, a rocket either side of that threshold gets two different models — and the difference is
the whole body's, not the part that changed. On the tests' rocket at Mach 3 and 4°, each remaining
threshold is worth this much, and the last column says who owns it — *open* means an issue with no
milestone behind it, *queued* a milestone on the roadmap, and *no longer a switch* a threshold
since removed, kept here because its size is the measured cost of the model it replaced
(`issue_87s_switches_are_this_big` pins the first four,
`a_lip_in_a_boattails_wake_carries_nothing` the lip's two, and
`a_near_flat_flare_marches_every_row_and_the_fallback_is_still_measured` the last):

| drawing this | normal force | centre of pressure | who owns it |
|---|---|---|---|
| a step in radius, past a billionth of the local radius | −8.7% | 1.03 calibres | open: [issue #87: a step has no model of its own](https://github.com/nrdptel/hpr-sim/issues/87) |
| a flare behind a boattail and too long for its wake, however small | −27.5% | 0.29 calibres | open: [issue #120: a lip longer than its wake](https://github.com/nrdptel/hpr-sim/issues/120) |
| a pointed tip steeper than the cone tables' 30° | −7.7% | 0.81 calibres | open: [issue #121: a tip past the cone tables](https://github.com/nrdptel/hpr-sim/issues/121) |
| a vertical tip steeper than the cap's handover to its base | −7.0% | 0.64 calibres | queued: [M1.8e16: the handover past 24°](../decisions-and-roadmap.md#m1-8e16) |
| a lip leaving its boattail's wake by rising or by sitting back | −29 to −34% | 0.93 to 1.97 calibres | no longer a switch: the rise is weighed, [ADR-041][adr-041] |
| a lip longer than its boattail's drop in diameter, however little it rises | −33.0% | 1.77 calibres, forward | open: [issue #120: a lip longer than its wake](https://github.com/nrdptel/hpr-sim/issues/120) |
| a near-flat flare, 0.03816° to 0.05882° at the table's top rows, which the march used to refuse | −8.3% | 1.16 calibres | no longer a switch: the element is read by the generalized method, [ADR-050][adr-050] |

In the first four rows the centre of pressure moves **aft** when the method is lost, so a rocket
that trips one reads *more* stable than one that doesn't. The two lip rows and the near-flat-flare
row go the other way: on a body whose tail takes lift off — a boattail's wake, or a flare the
method reads lower than slender-body theory does — slender-body theory puts the centre of pressure
**forward** of the method's, so losing the method there reads *less* stable. Which way it goes
depends on the body; what is reliable is the size.

The lip row about length is a switch the wake's grading does not cover: a lip is only sheltered if it is no
longer than the boattail's drop in diameter, which is the wake's own scale, and that length is a
threshold, not a ramp. It is the one this page's measurements use to take a rocket off the method
without changing a radius or an angle.

The first lip row is no longer a switch: it is spread over the band the wake grades, as
[above](#a-lip-in-a-boattails-wake). The second still is one. Nor is the last: since
[M1.8e19](../decisions-and-roadmap.md#m1-8e19) the march reads that flare's element rather than
refusing it, and what is left where it used to switch is a step of at most +0.129% and 0.0051
calibres, in [A near-flat flare](#a-near-flat-flare). The two tips moved rather than
went: a pointed tip's edge is the cone tables' 30° since
[M1.8e11](../decisions-and-roadmap.md#m1-8e11), and a vertical tip's is the handover's cap, which
stands at 24° for the reason in [What the cap is worth](#what-the-cap-is-worth).

**The flare went** ([M1.8e17](../decisions-and-roadmap.md#m1-8e17), [ADR-047][adr-047]): a conical
flare in the free stream now flies the method, and the boundary where its shock detaches has
nothing jumping across it — see [A flare through the method](#a-flare-through-the-method) below.
The second row is what is left of it, a flare *behind a boattail*, which is a lip in the
boattail's wake rather than a flare in the free stream and keeps [its own
rule](#a-lip-in-a-boattails-wake). The last row is what that milestone turned up on the way, and
[M1.8e19](../decisions-and-roadmap.md#m1-8e19) closed it: a band of near-flat flares, a third of a
millimetre tall, whose one element the march reduces to the generalized method.

**The step stayed**, and [M1.8e15](../decisions-and-roadmap.md#m1-8e15) says why: the march needs a
profile without a jump in it, so a step needs a model of its own rather than a decision about an
existing one, and the obvious fix turned out to cost more than it saved. What that milestone did
measure is in [A step in radius](#a-step-in-radius) below. Its threshold is finer than it sounds —
a billionth of the radius, a few hundredths of a nanometre on a 54 mm body — so any step a person
would draw is past it, and a rocket whose shape sits near one of these thresholds is worth checking
on both sides.

**What a flight takes.** The method covers the nose, when it is the first body (a blunt or vertical
tip behind its [Newtonian cap](#blunt-tips)), the body tubes straight behind it at the same radius,
and [boattails](../glossary.md#boattail)
(transitions that narrow toward the tail) and tubes behind those, with no step between them
([M1.8e4](../decisions-and-roadmap.md#m1-8e4)), and a conical flare, which ends the run
([M1.8e17](../decisions-and-roadmap.md#m1-8e17)). It flies only if nothing else behind them
changes the radius: no step, and no widening shape but that flare. Mixing the method's nose and cylinder with slender-body theory's
boattail would put the centre of pressure further off than slender-body theory alone, so the
boattail takes a supersonic share too, measured (below). Each covered part gets its own share,
so the flight's pitch damping still comes from each part's own local flow. That flow is taken at
one [station](../glossary.md#station) per part; the part's force and its moment about the nose tip
are the method's either way. A nose or cylinder takes that station at its share's own centre of
pressure. A boattail's share is usually negative, and so is a tube's behind it: the method carries the
boattail's expansion down the tube, where it fades out over several calibers, so a long tube can
lose as much as the short boattail. On the finned rocket of the tests (measured by hand, not
pinned), a 5.7° boattail 0.05 m long takes 0.284 per radian off at Mach 2 (footnote 8 took 0.117)
and the 0.3 m tube behind it 0.210. Such a share could cross zero as the Mach number changes, and its centre of
pressure would then run off to infinity. So these parts take their local flow where slender-body
theory does, on the part: a boattail at its slender-body centre of pressure, a tube at its
body-lift station. Only the damping feels this: on the test rocket at Mach 2 the tube's share acts
at 1.040 m but its flow is taken at about 1.15 m, so with the centre of mass near 0.7 m that
part's (negative) damping reads about 30% large. Body lift, the `sin² α` term, is unchanged.

**The boattail, measured.** Washington and Pettis ([WP68]) mounted the aft section of a
wind-tunnel model on its own balance and measured it with a conical boattail and as a plain
cylinder of the same length, Mach 1.75 to 4.5, and a whole model with and without one from Mach
0.8 to 1.5. The boattail's increment in slope collapses onto one curve:

`ΔC_Nα / [1 − (D_B/D)²] = F(√(M² − 1) / (L_B/D))`, per degree on the cylinder's area,

with `D` the diameter ahead of the boattail, `D_B` its base diameter and `L_B` its length
([WP68] Fig. 5, p. 8, read by hand into
[`WP_SLOPE_PER_DEG`](../api/hpr_aero/supersonic_boattail/constant.WP_SLOPE_PER_DEG.html) to about
±0.02 per radian). The increment acts about halfway along the boattail, from 43% of its length at
Mach 2 to 64% at 4.5 ([WP68] Fig. 6, p. 9). A flight gives a boattail the share the method gives
a cylinder of the boattail's length and fore diameter in its place, plus that increment at that
centre of pressure (test `the_boattail_takes_washington_and_pettis_increment`). Slender-body theory
gives the same boattail `2[(D_B/D)² − 1]` at every speed, which is Fig. 5's own subsonic line;
faster than sound the measured increment is less than half of it at the Arcas Robin's speeds
(0.24 to 0.47), and footnote 8's less again — though the curve is not always below that line:
across Fig. 5 it runs from 0.23 to 1.58 times it, passing it at an argument of 0.635, near Mach 1.

**A steep boattail reads the correlation no steeper than 16°.** Washington and Pettis measured
boattails of 4° to 9.5°, where the flow follows the surface. Past about 16° it doesn't: the drag
buildup already treats a boattail as separating from there
([Boattails faster than sound](#boattails-faster-than-sound), Cubbage's steepest attached
boattail). Nothing measures what a separated boattail's normal force then does, so hpr reads the
correlation at the steepest angle where the flow is still attached: a boattail past 16° takes the
increment of one of the **same radii** drawn out to 16°, and its centre of pressure stays on the
real boattail ([ADR-040][adr-040],
[issue #90: how steep a boattail the correlation should cover](https://github.com/nrdptel/hpr-sim/issues/90)).
The Arcas Robin's 15° boattail is untouched; Calisto's 18.4° reads its correlation as a 16° one.
The increment is continuous in the angle, so a rocket doesn't jump as its boattail is drawn
steeper (test `a_separating_boattail_reads_the_correlation_at_its_steepest_measured_angle`).

**Why hold it rather than let it fade.** The two honest limits for a separated boattail are the
correlation held at 16°, and nothing at all — the body behaving as though the boattail were a
cylinder, since a separated surface no longer turns the flow. hpr takes the first. The increment
is negative, so it takes lift off the tail: holding it keeps the centre of pressure forward, and
letting it fade to zero would move the centre of pressure **aft** and make a steep boattail look
more stable than anything measured. On the tests' rocket — an ogive nose, a tube, and a 30°
boattail — the body's centre of pressure sits this much further aft if the increment fades away
than if it is held:

| Mach | 1.5 | 2 | 3 | 4.63 |
|---|---|---|---|---|
| calibres between the two rules | 1.35 | 0.91 | 0.75 | 0.67 |

That is the size of the doubt, and it is largest where a hobby rocket spends its supersonic flight:
a boattail steeper than 16° is worth two thirds of a calibre at Mach 4.63 and a third of a calibre
more than one at Mach 1.5. hpr takes the forward end of that range (test
`a_separating_boattail_reads_the_correlation_at_its_steepest_measured_angle` pins both ends).

There is one more bound, on the holding rather than on the measurement. Reading a longer boattail
walks the correlation's argument `√(M² − 1)/(L_B/D)` toward zero, where Fig. 5's curve comes from
the report's lowest supersonic runs and rises past Munk's slender-body line — which the report
plots there for comparison *at subsonic speeds* (p. 3). hpr does not invent a length and then read
that branch, so the extra the holding takes off stops at potential flow's
`2 (A_aft − A_fore)/A_fore` (`holding_the_correlation_stops_at_potential_flow`).

Be clear about what this does and does not do. A boattail's read **at its own length** is never
clipped, wherever it sits — that is the correlation as published, and a genuinely long boattail
reads the same near-Mach-1 branch with no bound at all. A 4° boattail to 0.6 of the radius reads
1.29 times Munk's line at Mach 1.5, and hpr flies it. Only the length the 16° hold invents is
capped. The bound bites when the aft radius is under about `1 − √(M² − 1)/1.11` of the fore
radius — two fifths at Mach 1.2, a quarter at 1.3, a twentieth at 1.45, nothing much above Mach
1.49 — and only where the method's table has started, which on such shapes it barely has.

How much that is worth is measured rather than argued, by sweeping boattails of 16° to 53.6°
narrowing to between a thousandth and three tenths of the fore radius, and reading their shares
back out of the table (`what_the_potential_flow_bound_reaches`). Two things come out.

- **At the table's rows a boattail never takes off more lift than potential flow** — except in
  the sliver described below, where its own read already passes it and the bound never clips that.
- **The bound moves a printed coefficient by about 0.060 per radian at most**, at 53.5° narrowing
  to a thousandth of the radius. That is the printed number, after the join's weight; the holdback
  on the boattail's own cross-section is up to about six times larger near the join, where the
  join is barely open.
  53.5° is the steepest boattail the sweep found the method willing to table at all — it refuses
  53.6°, and refuses shallower angles than that where the boattail narrows less.

No committed design comes near: the steepest is Calisto's 18.4°, whose aft radius is 0.685 of its
fore radius, where the bound would need under 0.40 even at Mach 1.2.

Two caveats, both small and both real. The bound applies where the shares are computed, at the
table's rows; between rows the table interpolates, so a printed value beside a row of the next
kind can sit past potential flow, by up to about 0.003 per radian on the shapes swept. And in a
sliver just above the hold's own angle — the sweep finds it at 16°, 16.5°, 17° and 17.25° — a
boattail is deep enough that **its own** read already passes potential flow, and since the bound
never clips a boattail's own length, the hold does nothing there and the rocket flies an
extrapolation nothing measured checks. That window runs a few hundredths of a Mach from where the
table starts, so the join is barely open across it, and it closes as the angle or the speed rises
rather than at a fixed angle.

**How far to trust the 16°.** It is Cubbage's, measured at Mach 0.6 to 1.28 and on *drag*, and it
is used here on the normal force from Mach 1.2 up. A shoulder turns the flow through a
Prandtl–Meyer expansion faster than sound, where separation is less likely than transonically, so
16° is if anything early. No measurement of a steep boattail's supersonic normal force exists to
check it. Note too that the two models take opposite consequences from the same angle: separation
*lowers* a boattail's pressure drag toward the base value, and here it *holds* the lift the
boattail takes off instead of letting it shrink. The argument for that is the stability one above,
not a flow one. The angle is also read on each narrowing part's own chord angle, while the drag merges
adjacent narrowing parts into one cone before grading, so a boattail drawn in several parts can be
graded differently by the two.

*A worked example.* The Arcas Robin's boattail narrows from 2.25 in to 1.308 in over 1.757 in, so
`L_B/D` = 0.781 and `1 − (D_B/D)²` = 0.662. At Mach 2.3, `√(M² − 1)` = 2.071 and Fig. 5's
argument is 2.071/0.781 = 2.652, where the curve reads −0.01263 per degree, −0.7237 per radian;
times 0.662 that is −0.479 per radian on the body's cross-section. Slender-body theory gives
−1.324, footnote 8 −0.103. Across the tunnel's Mach numbers:

| Mach | Washington and Pettis | footnote 8 | slender-body theory |
|---|---|---|---|
| 1.5 | −0.622 | −0.177 | −1.324 |
| 1.8 | −0.554 | −0.144 | −1.324 |
| 2.3 | −0.479 | −0.103 | −1.324 |
| 2.96 | −0.409 | −0.068 | −1.324 |
| 3.96 | −0.340 | −0.038 | −1.324 |
| 4.63 | −0.314 | −0.026 | −1.324 |

Their models were conical boattails of 4° to 9.5°, 0.82 to 1.18 diameters long, narrowing to 0.72
to 0.86 of the diameter; the Arcas Robin's is steeper (15°), shorter (0.78) and narrower (0.58),
so its row is an extrapolation. Their points scatter about the curve by up to about 15%. Past the
curve's end (an argument of 6.1: a short boattail at a high Mach number) hpr holds its last value.
[`BodyModel`](../api/hpr_aero/model/struct.BodyModel.html) keeps footnote 8 for comparison.

**A table.** One run of the method takes a few milliseconds, too slow for every step of a
flight. So the first time a flow faster than Mach 1.2 needs it, hpr runs the method every 0.05 in
Mach from Mach 5 down, to the lowest Mach at which it holds, and keeps the results. Between those
Mach numbers it interpolates in a straight line. That took about 0.3 s once per rocket in a debug
build on the development Mac (measured by hand, for a body the method takes from Mach 1.2); a
body whose join starts higher adds about 48 runs for the bisection below. A rocket that never
passes Mach 1.2 never pays it.

**The join.** Write SB for slender-body theory, SE for the shock-expansion method, and `M_j` for
where the join starts: Mach 1.2, or the lowest Mach at which the method holds if that is higher.
hpr narrows that Mach down between two rows of the table by halving the gap (bisection) until no
smaller step exists in the computer's numbers. It then runs the method at that Mach and adds the
result as an extra row. The start must be that exact: there the method's shares climb from zero
like the square root of the distance in Mach, so a start off by `δ` puts `√δ`-sized shares in
that row.
So the start moves smoothly with the nose's shape instead of in 0.05 steps. A cone with a 20°
half-angle (the angle between its side and its axis) joins from Mach 1.341910; each 0.1° steeper,
up to 20.5°, moves the start about 0.0027 later, to 1.355500. From `M_j` to
`M_j + 0.3`, each covered part's slope, moment and station move in a straight line from SB's to
SE's:

`C_Nα = C_Nα,SB + w (C_Nα,SE − C_Nα,SB)`, `w = (M − M_j)/0.3`, clamped to 0 to 1.

Every piece is a straight line in Mach, so nothing jumps. The test
`the_supersonic_join_has_no_jump` looks at ±1e-9 in Mach on each side of the join's ends, of
table rows, between rows and at Mach 4.999, and `a_blunter_cone_joins_where_the_method_starts_to_hold`
does the same for the 20° cone, whose join starts higher.
`the_joins_start_moves_with_the_nose_not_in_steps` pins that cone's start and the 20.5° cone's to
1e-6, checks the start is off the grid, that the cylinder's share at the start is under 1e-5 per
radian (about 1e-7), and that the start moves by less than 1e-7 when the cone steepens by a
millionth of a degree (2.7e-8). `a_boattailed_body_flies_the_method_without_a_jump` does the same
±1e-9 probe for the finned rocket with its boattail and tail tube. Mach 1.2 to 1.5 is a judgement: below Mach 1.2 the flow over the nose is
transonic, which the method doesn't cover, and Mach 1.5 is the
lowest Mach at which NASA measured the Arcas Robin
([ADR-034](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-034-the-bodys-supersonic-normal-force-in-flight-tabulated-shock-expansion-shares-joined-linearly-from-mach-12-2026-09-19),
the decision behind it). Small changes in shape can still switch a body between the two models,
for example a nose so steep that the method never holds at any Mach up to 5, so the rocket
keeps slender-body theory throughout
([issue #87](https://github.com/nrdptel/hpr-sim/issues/87)).

**A worked example: the Arcas Robin's body.** NASA measured the Arcas Robin's body
without fins in a wind tunnel ([TN D-4014](#code-and-sources)). Flown through a flight's own code,
with the secant-ogive nose (a [tangent ogive](../glossary.md#tangent-ogive)'s cousin, its arc larger) fitted to the report's coordinates and the
boattail left off, the nose and cylinder give the method's own values: equal on the table's rows
(Mach 1.5, 1.8, 2.3), within 1e-4 between them.

With the boattail on (the lip behind it off, a flare the method doesn't take), the boattail takes
its measured share, 0.31 to 0.62 per radian off, most at Mach 1.5 (footnote 8 took 0.03 to 0.18).
At `α → 0` the short model then reads 12.0% to 26.2% below the measured line and the long model
29.7% to 33.3% below it. That line is fitted over the plotted angles, so it carries crossflow lift
and these columns leave body lift out: fitted the same way, with the body lift a flight adds, the
same body reads 3.4% to 41.0% *high*
([Checking the shock-expansion method](#checking-the-shock-expansion-method)).

hpr's committed Arcas Robin design flies the method to its base since
[Blunt tips](#blunt-tips) and [A lip in a boattail's wake](#a-lip-in-a-boattails-wake): the last
two columns are the method's own values for its power-series nose, cylinder and boattail, with the
lip carrying nothing. Slopes are per radian on the body's cross-section, at `α → 0`; the measured
slope is fitted over the plotted angles with the boattail and lip on, so it also carries some
crossflow lift and their share, which is why every column reads below it here.

| model | Mach | measured | method | flight, nose and cylinder | flight vs measured | flight, with boattail | with boattail vs measured | design as committed | committed vs measured |
|---|---|---|---|---|---|---|---|---|---|
| short | 1.5 | 2.192 | 2.552 | 2.552 | +16.4% | 1.930 | −12.0% | 1.852 | −15.5% |
| short | 1.8 | 2.613 | 2.724 | 2.724 | +4.3% | 2.171 | −16.9% | 2.143 | −18.0% |
| short | 2.3 | 3.078 | 2.931 | 2.931 | −4.8% | 2.452 | −20.3% | 2.394 | −22.2% |
| short | 2.96 | 3.284 | 3.124 | 3.124 | −4.9% | 2.716 | −17.3% | 2.612 | −20.5% |
| short | 3.96 | 3.884 | 3.300 | 3.300 | −15.0% | 2.963 | −23.7% | 2.735 | −29.6% |
| short | 4.63 | 4.149 | 3.371 | 3.371 | −18.7% | 3.063 | −26.2% | 2.718 | −34.5% |
| long | 1.8 | 3.159 | 2.724 | 2.724 | −13.7% | 2.171 | −31.3% | 2.143 | −32.1% |
| long | 2.3 | 3.525 | 2.932 | 2.932 | −16.8% | 2.453 | −30.4% | 2.394 | −32.1% |
| long | 2.96 | 3.868 | 3.127 | 3.127 | −19.2% | 2.718 | −29.7% | 2.614 | −32.4% |
| long | 3.96 | 4.455 | 3.313 | 3.313 | −25.6% | 2.974 | −33.3% | 2.740 | −38.5% |
| long | 4.63 | 4.615 | 3.395 | 3.395 | −26.4% | 3.082 | −33.2% | 2.724 | −41.0% |

The rows are in
[`validation/fixtures/aero/shock-expansion.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/shock-expansion.json)
(`arcas_robin`: `nose_and_cylinder`, `in_flight`, `in_flight_with_boattail` and `as_designed`), written by
`cargo xtask aero`; no test re-reads the flight columns, so they are regenerated by hand.

**What it leaves out.** Steps, and widening shapes that are not cones, as above; a conical flare
has its own section ([A flare through the method](#a-flare-through-the-method)); a blunt or
vertical tip takes the cap of
[Blunt tips](#blunt-tips), below. The method itself has no crossflow lift; a flight adds
[body lift](#body-lift) on top.

#### A lip in a boattail's wake

What this covers: a short flare at the very base, behind a boattail, like the reflex lip of NASA's
Arcas Robin models. How far to trust it: hpr gives such a lip no normal force faster than sound,
which is what the measured pitching moment supports, but the moment bounds the lip rather than
measuring it.

**The rule.** A lip that sits wholly in a boattail's wake carries no potential-flow slope from the
Mach number where the method takes over; below the join it keeps slender-body theory's
`2 ΔA/A_ref`, and the join blends the two, so nothing jumps. hpr takes the shelter's share from its
drag model ([ADR-030][adr-030], which takes the same lip's drag away): wholly in the wake up
to a rise of a quarter of the boattail's drop in diameter, not at all from half of it, and the wake
fades over any tube between them. On top of that fraction the normal force asks one thing the drag
model doesn't: the lip must be no longer than the boattail's drop in diameter, the wake's own
scale, or it grows out of the wake however little it rises. The decision record on the lip,
[ADR-039][adr-039], sets out the readings behind the share itself.

**A lip part way out of the wake.** Where the wake covers the lip only partly, the drag model has
always graded it so. Since [M1.8e10](../decisions-and-roadmap.md#m1-8e10) the normal force reads
the same number as a **weight**: the method's share counts for the wake's share of the lip,
slender-body theory for the rest, exactly as they blend across the Mach join
([ADR-041][adr-041]). It is the whole wake fraction, not the rise alone — the shelter fades with
the lip's rise, with any tube between it and the boattail, and with anything else in the way.

Be clear about what that buys. The **jump** is gone: a lip drawn a hair taller no longer switches
the whole body between the two models, which it used to do by a third of the normal force and 1.77
calibres of centre of pressure on the tests' rocket at Mach 3. The **sensitivity** is not gone; it
is spread over the band. On that rocket, at Mach 3 and 4°:

| drawn from | to | normal force | centre of pressure |
|---|---|---|---|
| a lip rising a quarter of the boattail's drop | rising a half (1.25 mm of radius) | −29.1% | 0.93 calibres |
| a lip flush behind the boattail | one a boattail's drop in diameter behind it (10 mm) | −33.8% | 1.97 calibres |

That table is a **design** sensitivity: how the printed answer moves as you draw the lip
differently. It is not the same as how far apart the two models are, which is what a lip in the
band is actually uncertain by. At one fixed shape — a lip rising a quarter of the drop — the
method and slender-body theory differ by **33.0% of the normal force and 1.77 calibres**, and
nothing measured says which is right for a part-sheltered lip. That number has not changed; what
changed is that a rocket can no longer cross it in a ten-thousandth of its geometry.

**Why nothing.** Three readings point the same way.

- The tunnel itself. TN D-4014 ([D4014] p. 6) traces an odd chamber axial force at Mach 1.50 and
  1.80, fins off, to the reflex lip, and says the effect is "masked" once separation runs over the
  boattail at higher Mach numbers or the fins thicken the boundary layer — and that the longer
  model shows none of it, "probably because of the thicker boundary layer at the model base". So
  the lip does something at those two speeds, and those are the very rows whose moment implies a
  *negative* share below; what it does there isn't a normal force this model can carry.
- Seiff's own limits. His embedded Newtonian flare method holds for "thin shock layers when the
  flow is not extensively separated" ([S62] p. 13), and he notes that "a 90° ramp will invariably
  separate the flow" (p. 4). The Arcas lip's face stands about 57° to the axis, behind a 15°
  expansion.
- The size, at most. Taking Seiff's method anyway as an upper bound (eq. 9, p. 12, which for a
  conical flare at one dynamic pressure is `2 (q₁/q∞) cos²θ ΔA/A_ref`, with `q₁` the flow that has
  expanded through the boattail's turn, and `θ` taken to the axis, 56.8°, where Seiff measures it
  from the local stream — the looser of the two) gives 0.044 per radian at Mach 1.5 falling to
  0.014 at 4.63, against slender-body theory's 0.178 at every speed.

**What the moment says, and what it can't.** For each fins-off row, the share at the lip's station
that would put hpr's centre of pressure on the measured one runs from −0.256 ± 0.068 per radian
(short model, Mach 1.5) to +0.229 ± 0.084 (long, Mach 3.96), changing sign with Mach number and
with the model's length. Fitting one share:

| rows | share, per radian | χ² per degree of freedom | from zero | from slender-body theory's 0.178 |
|---|---|---|---|---|
| all eleven | +0.021 ± 0.019 | 4.5 | 1.1 σ | 8.4 σ |
| the short model's six | −0.016 ± 0.022 | 6.4 | 0.7 σ | 8.7 σ |
| the long model's five | +0.108 ± 0.034 | 0.9 | 3.2 σ | 2.1 σ |

A χ² per degree of freedom of 1 means rows agreeing within their own error bars. The short model's
6.4 means its rows disagree among themselves; the long model's 0.9 means its five agree — on a
share of +0.108, five times Seiff's bound at that speed and three standard errors above zero, yet
still two below slender-body theory's.

So the moment does not settle the lip, and the model doesn't rest on it. The reason is in how the
number is made: it blames the lip for *every* miss in the centre of pressure, and hpr's body alone
reads 15% to 19% high on the long model at Mach 1.8 and 2.3, which shifts its centre of pressure by
far more than any lip. The short model's own fit comes out negative, which no flare can produce.
What the moment does say is that slender-body theory's 0.178 at the base is too much: eight
standard errors out on the short model, two on the long.

**How it was checked.** The committed designs, fins off, through a flight's path, fitted at the
tunnel's plotted angles as [Checking the shock-expansion method](#checking-the-shock-expansion-method)
fits them. Slopes are per radian on the body's cross-section; centres of pressure are calibres aft
of the nose tip. The last column is where hpr's whole-body centre of pressure would sit if the lip
carried slender-body theory's share instead of nothing. hpr's slopes here are fitted over the
tunnel's angles, so they carry body lift; the same bodies' slopes at `α → 0`, in
[Checking the shock-expansion method](#checking-the-shock-expansion-method)'s table, are lower.

| model | Mach | measured | hpr, as committed | vs measured | measured CP | hpr's | hpr's CP with the lip at slender-body theory's share |
|---|---|---|---|---|---|---|---|
| short | 1.5 | 2.192 | 3.017 | +37.7% | 1.00 | 2.46 | 3.33 |
| short | 1.8 | 2.613 | 3.290 | +25.9% | 2.36 | 3.06 | 3.84 |
| short | 2.3 | 3.078 | 3.598 | +16.9% | 3.56 | 3.69 | 4.37 |
| short | 2.96 | 3.284 | 3.838 | +16.9% | 3.21 | 3.75 | 4.43 |
| short | 3.96 | 3.884 | 3.946 | +1.6% | 4.88 | 4.57 | 5.16 |
| short | 4.63 | 4.149 | 3.950 | −4.8% | 5.05 | 4.72 | 5.30 |
| long | 1.8 | 3.159 | 3.770 | +19.4% | 4.61 | 4.31 | 5.19 |
| long | 2.3 | 3.525 | 4.071 | +15.5% | 5.04 | 4.89 | 5.68 |
| long | 2.96 | 3.868 | 4.400 | +13.7% | 5.33 | 4.64 | 5.47 |
| long | 3.96 | 4.455 | 4.428 | −0.6% | 6.19 | 5.20 | 5.98 |
| long | 4.63 | 4.615 | 4.425 | −4.1% | 6.40 | 5.96 | 6.65 |

The rows are in
[`validation/fixtures/aero/arcas-robin-lip.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-lip.json),
written by `cargo xtask aero`, and a test holds this table to it cell by cell. With the lip left
off entirely the same bodies read within 0.05 percentage points of these rows at ten of the eleven,
and 0.5 at Mach 1.5 on the short model, so the lip changes little but which parts of the body the
method may cover.

**What it means for a rocket.** Most rockets have no lip, and nothing changes for them. For one
that does, the rule decides whether the *whole body* flies the method or slender-body theory, so it
is worth more than the lip itself: on the Arcas Robin's short model at Mach 2.96 the body's slope
goes from 2.08 per radian to 3.84, and the whole rocket's from −16.3% against the tunnel to +3.7%
([Normal force through Mach 1](#normal-force-through-mach-1)). The extra lift sits on the body,
ahead of the fins, so the centre of pressure moves *forward* by 0.22 calibres there: a little less
stability margin, and a good deal more restoring force.

**What it leaves out.** The lip still has drag, and its own wake rule there ([ADR-030][adr-030]).
Nothing here measures a lip's lift directly: the tunnel gives forces for the whole body, and the
moment bounds the share rather than measuring it. The shelter used to be a switch in shape, of the
family [issue #87](https://github.com/nrdptel/hpr-sim/issues/87) tracks, and a large one, because
it decides whether the *whole body* flies the method. Since
[M1.8e10](../decisions-and-roadmap.md#m1-8e10) the rise no longer switches it: on the test rocket
at Mach 3 and 4°, lips rising 0.2499 and 0.2501 of the boattail's drop now agree to a
ten-thousandth. What is left is how far apart the models are at one shape — 33.0% of the normal
force, and the centre of pressure 1.77 calibres **forward** on slender-body theory — spread over
the band the wake grades ([above](#a-lip-in-a-boattails-wake)). A narrowing part behind the
run is a boattail the method hasn't covered, not a lip, and keeps slender-body theory's share. A
widening part behind a boattail that is *too long* for the wake stays a lip as well, and still
takes the whole body off the method: the flow reaching its corner is the wake's, which the march
does not compute, so [A flare through the method](#a-flare-through-the-method) does not apply to
it.


#### Where a flare's march stops

**In short:** a [flare](../glossary.md#flare) is a transition that widens toward the tail, and the
method will march one — but only up to a limit, and that limit is *not* where the flare's shock
detaches. It is where the corner's turn would take the flow to Mach 1, which is a property of hpr's
method rather than of the air, and it depends on the whole body ahead of the flare: on the body
measured below it falls short of a wedge's detachment angle at Mach 1.5 and runs past it at Mach 2,
and taking the tube away moves it past the wedge at both. From Mach 2.13 to Mach 5, the highest
checked, the limit is neither — it is the 30° where the cone tables end. The flare's own detachment
angle is not known here: the wedge's is a conservative stand-in for it. This section is the
measurement, and it is why the attachment test a flight uses had to be **chosen** rather than read
off the march's refusal ([ADR-045][adr-045]). What a flight does with a flare is the next section,
[A flare through the method](#a-flare-through-the-method), and how close that comes to a measured
flare is the one after it, [What a marched flare is
worth](#what-a-marched-flare-is-worth). Neither this section's edge nor the drawn-out reading past
it is itself compared with a measurement.

Second-order shock-expansion turns every corner
isentropically — no entropy rise, so no shock — with the
[Prandtl–Meyer](../glossary.md#prandtlmeyer-expansion) angle ν ([SD56]) eq. 3. A flare
is a compression corner, so the turn spends ν: the march goes on only while the flow reaching the
flare has enough of it left to turn through the flare's angle without dropping to Mach 1. Whether a
shock instead stands attached to that corner is a separate question, and it is the one that decides
whether the march is modelling the real flow at all.

The table below sweeps the flare's angle on a fixed body — a pointed 2.75° cone, five calibres of
tube, and a conical flare — and bisects, until the two angles are adjacent double-precision
numbers, the steepest flare the march accepts. The two angles are NASA TN D-4865 model 2's; **the
layout is not**. That model is blunt-nosed and has no tube at all, and the edge depends on what is
ahead of the flare, because that is what sets the flow reaching it. Angles are quoted to seven
decimals so the differences add up, and the detachment column is taken at the *free-stream* Mach
number (the flow reaching the flare is a little faster, which would move the wedge's angle by about
0.02°). Both tests are in
[`shock_expansion.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-aero/src/shock_expansion.rs):
`a_flare_marches_to_the_isentropic_turn_not_to_detachment` and
`past_mach_2_13_the_flare_stops_where_the_cone_tables_do`.

| free stream | the march accepts a flare to | a wedge's shock detaches at | so the march is |
|---|---|---|---|
| Mach 1.5 | 11.9312175° | 12.1126689° | 0.1814514° short of it |
| Mach 1.547787962528 | 13.346819° | 13.346819° | exactly on it |
| Mach 2 | 26.4714031° | 22.9735318° | 3.4978713° past it |
| Mach 2.5 | 30° (the tables) | 29.7974° | 0.20° past it |
| Mach 3 | 30° (the tables) | 34.0734° | 4.07° short of it |

**How much of that is the tube.** A great deal, and it is the point rather than a caveat: what the
march has left to spend is ν of the flow arriving at the corner, and the body ahead sets that flow.
Keeping the same cone and flare and changing only the tube's length:

| tube | the march accepts a flare to, Mach 1.5 | at Mach 2 |
|---|---|---|
| none (the report's own layout) | 14.194333° | 28.509856° |
| 1 calibre | 12.821811° | 27.500078° |
| 2.5 calibres | 12.144405° | 26.815015° |
| 5 calibres (the table above) | 11.9312175° | 26.4714031° |

So the first row of the first table — the march stopping *short* of a wedge's detachment — is a
property of that five-calibre body, not of the method: on the report's own tube-less layout the
Mach 1.5 edge is 14.19°, two degrees *past* the wedge's limit. What does not depend on the body is
the conclusion: the march's edge is set by the corner's isentropic turn, and it lands on both sides
of a wedge's detachment angle depending on nothing more than how long the tube is.

The detachment angles are a *wedge's* largest deflection ([R1135], through
[`wedge_detachment_angle_rad`](../api/hpr_aero/blunt_tip/fn.wedge_detachment_angle_rad.html), which
lives with the blunt-tip cap because that cap uses the same relation). A cone's shock holds to
steeper angles than a wedge's, and a conical flare on a cylinder sits between the two, so the
wedge's column is a conservative stand-in, not the flare's own boundary. Only the rows where the
march stops *below* the wedge's angle prove anything about attachment; where the march runs past
it, the flare's own limit may still be higher. Which boundary a flight uses is
[the next section](#a-flare-through-the-method)'s subject. What the table shows is that the march's edge
lands on both sides of any such boundary: you cannot tell, from hpr returning an answer, that the
flow it modelled is the flow that would be there.

The last two rows of the first table are a different limit altogether. Each element's tangent cone
is looked up in NASA SP-3007 Table 2, whose slopes stop at 30° — the milestone that took them there
from 24° is [M1.8e11](../decisions-and-roadmap.md#m1-8e11) ([ADR-042][adr-042]) — so from Mach
2.129702032593 to Mach 5, the highest checked, every Mach number gives the same edge. The reference
data runs out before the flow does, and the last column then says nothing about attachment.

**An 18.5° flare, the report's angle, on the 2.75°-cone body above** (not the flared rocket of
[the next section](#a-flare-through-the-method), whose numbers are close but not these). The march
accepts it from Mach 1.721760;
a wedge's shock reaches 18.5° only at Mach 1.767575. Between the two, hpr returns a
number for a flare whose shock is, on that reckoning, detached — a bow shock standing ahead of the
juncture with a pocket of subsonic flow behind it, which an isentropic corner turn does not
describe. TN D-4865's lowest run, Mach 1.50, is below both, and there the march refuses outright,
as the report itself says it should. On a shorter body those two Mach numbers move, as the table
above shows.

**What it leaves out.** These digits pin what this program does, not what air does: every one of
them comes from bisecting hpr's own refusal, and the 30° rows come from where a lookup table ends.
A band of very shallow flares is not marched by the second-order law at all: the pressure behind
such a corner moves away from its tangent cone's rather than toward it, so its one element is
*reduced* to the older generalized method, which since
[M1.8e19](../decisions-and-roadmap.md#m1-8e19) is read rather than refused
([A near-flat flare](#a-near-flat-flare)). **Where that band sits is a property of the body ahead
of the corner, and it moves by orders of magnitude**: 0.773° to 0.823° at Mach 3 on this
2.5-calibre body, against 0.0066° to 0.0081° at the same Mach number on the flared rocket of
[A near-flat flare](#a-near-flat-flare). The whole edge is inviscid, too. From Mach 2.96 up TN D-4865 records the boundary layer separating
ahead of the flare and reattaching on it, which moves the pressure rise downstream of where a
tangent body puts it; nothing here models that. And below Mach 1.5 nothing here was measured,
although a flight uses the method from Mach 1.2.

#### A flare through the method

**In short:** since [M1.8e17](../decisions-and-roadmap.md#m1-8e17) a rocket with a
[flare](../glossary.md#flare) flies the [shock-expansion method](#bodies-faster-than-sound) rather
than dropping to [slender-body theory](../glossary.md#slender-body-theory) the moment one is drawn.
The method marches through the flare's corner while the shock there stays attached; a steeper flare
is read as one of the same radii **drawn out** to the steepest attached angle, which is what keeps
the answer from jumping as a shape or a speed crosses that boundary. How far to trust it: the
attachment test is a standard relation, applied at a corner it was not derived for; the reading
either side of the boundary it draws agrees to four parts in 1e11, though its *slope* kinks there;
and what it is worth against the one measured flare in the
sources is [the section after this one](#what-a-marched-flare-is-worth): −1.9% and +7.0% at Mach
1.9 and 2.3, +13.4% at 2.96, and +51.5% and +50.4% at 3.95 and 4.63. Read the numbers below as
what this program does, and that section as how close it lands. What qualifies is narrow: a *conical* flare, flush with the part ahead of
it, not in a boattail's wake, with nothing behind it that carries lift of its own. A boattail then
a flare is a [lip in a wake](#a-lip-in-a-boattails-wake) and keeps that rule; any other widening
shape still ends the run. The decision record is [ADR-047][adr-047].

**Why a test had to be picked.** The march will return a number for a flare whose shock has long
since detached — [the section above](#where-a-flares-march-stops) measures exactly that — so "the
method answered" is not evidence the flow it modelled is the flow that would be there. Something
independent has to say where the corner's shock detaches.

**The test.** A flare's shock springs from a **circular corner**, not from a point apex. Where the
shock forms, the flow is two-dimensional: the body's radius is the scale over which the
axisymmetric relief acts, and at the corner itself none of it has happened yet. So hpr uses
NACA Report 1135's ([R1135]) largest deflection behind an attached plane oblique shock (eq. 168 into
eq. 138) — the same relation, in the same function
([`wedge_detachment_angle_rad`](../api/hpr_aero/blunt_tip/fn.wedge_detachment_angle_rad.html)),
that TN D-4865 ([J68]) p. 5 uses to hand a [blunt tip's cap](#blunt-tips) over to this
method. One attachment rule, at both corners the program has.

Two details matter.

- **It is read at the flow reaching the corner**, not at the free stream. The body ahead has
  already changed the air, and which way depends on what that body is: the ogive nose and tube of
  the tests' flared rocket leave it a little *slower* than the free stream, while the 2.75° cone
  and tube of [the section above](#where-a-flares-march-stops) leave it a little *faster*. The
  march knows by how much
  ([`aft_flow`](../api/hpr_aero/shock_expansion/struct.ShockExpansionBody.html#method.aft_flow)).
  On the tests' flared rocket a Mach 2.0 free stream reaches the corner at Mach 1.9998, so the
  limit is 22.9698° and not the free stream's 22.9735°. The march is downstream-only, so the flare
  cannot change the flow arriving at its own corner — which is what lets the limit be worked out
  before the flare is drawn.
- **The cone tables cap it at 30°**, because past that the march has no tangent cone to relax
  toward ([tangent cone](../glossary.md#tangent-cone), [ADR-042][adr-042]) — but that bounds the
  flare's **surface angle**, while the shock
  bounds the **turn** at its corner, so each caps its own quantity. On a flare behind a cylinder,
  where the surface ahead is at 0°, the two are the same number and the cap binds from Mach
  2.5192034260 **of the flow reaching the corner** up; a faster flow buys nothing above that.

Because the shock's bound is on the turn and not on the flare's angle, a flare put straight onto
an ogive nose — no tube between them — is read differently from the same flare behind a tube: the
corner there turns the flow by the flare's angle *less* the nose's base slope, so the limit bites
at a steeper flare. On the tests' flared rocket (an ogive nose 0.25 m long on a 27 mm radius, then
a 0.7 m tube of that radius) the surface ahead is at 0° and the turn and the angle are one number:

| free stream | the flow reaching the corner | the corner is read to |
|---|---|---|
| Mach 1.5 | Mach 1.4999688 | 12.1118502° |
| Mach 2.0 | Mach 1.9997809 | 22.9697612° |
| Mach 2.5 | Mach 2.4989548 | 29.7863106° |
| Mach 3.0 | Mach 2.9965267 | 30° (the tables) |
| Mach 4.95 | Mach 4.8922987 | 30° (the tables) |

A cone's shock holds to steeper angles than a wedge's, so **if** a conical flare on a cylinder
does sit between the two, this errs one way only: it stops reading some flares whose shock is in
fact still attached, and reads none whose shock is not. Nothing here measures where a flare's own
boundary actually is, so that "if" is an argument, not a result — and erring low is not the same
as erring safely. A flare just past the limit is not left unread; it is read as a *different*
flare, and how different nothing here measures either.

**Past the limit, the flare is drawn out.** A flare steeper than the limit is read as a flare of
**the same radii** stretched to the limiting angle — longer and shallower, turning the same air
through a corner the shock can hold — with its centre of pressure put back on the real flare, at
the same fraction along it. Because the radii are kept and the angle is not, **every flare steeper
than the limit reads the same force**: held at the limit, in other words. There is no bound on how
far that goes, and it under-reads badly at the extreme — at Mach 2 a 75° flare, an annular face a
detached bow shock would stand in front of, reads 1.638 per radian against slender-body theory's
2.010, where the truth is above both. It reads a steep flare as *less* stabilizing than it is,
which is the safe direction for a stability margin and the wrong one for a load. This is the boattail rule turned around: a boattail past 16° reads the
correlation of one of the same radii drawn out to 16° ([ADR-040][adr-040],
[A boattail faster than sound](#the-body-faster-than-sound-in-a-flight)).
The radii are what set how much air the flare turns, and they are never changed.

That is also what makes the answer continuous, **by construction rather than by tuning**: at the
limit the drawn-out flare *is* the real flare, so the two readings are the same body. The test
`nothing_jumps_where_the_flares_shock_detaches` probes either side of the boundary. At Mach 2.0 —
a row of the table, so the reading is that row's and not an interpolation — the boundary is a flare
of 22.969761173077°:

| probe, in the flare's angle | the normal-force slope moves by, as a fraction of itself |
|---|---|
| ±1e-9° | 4.527e-11 |
| ±1e-7° | 4.527e-9 |
| ±1e-5° | 4.527e-7 |

A hundredfold smaller probe moves the answer a hundredfold less, so what the probe finds is a
slope and not a step: the reading is continuous across the boundary. **Its slope is not**, and
nothing here claims otherwise — a cap makes a kink, because below the boundary the flare's angle
moves the body the march sees and above it only the radii do. Measured at the same place,
`dC_Nα/dδ` changes by −31.4% across the boundary on the whole rocket and by −141.6% on the flare's
own share, where it changes sign. The marched branch is not smooth in the angle either: the same
probe at 20°, away from any boundary, finds +3.5% and +41.1%
(`the_cap_makes_a_kink_in_the_slope_even_though_the_reading_holds`). Probing the
Mach number instead, at the 18.5° of TN D-4865's model 2 — whose shock holds on **the tests'
flared rocket** from Mach 1.767666917849 — gives 3.622e-10, 3.622e-8 and 3.622e-6 for the same
three probes. That half is taken on the table's own rows rather than on a reading between them:
the crossing falls inside the Mach 1.75 to 1.80 interval, where the reading is a straight line
between rows and would look continuous whatever the two branches did.

**What it is worth.** The tests' flared rocket is an ogive nose 0.25 m long on a 27 mm radius, a
0.7 m tube of that radius, a **10° conical flare** 0.3 m long opening to a 79.9 mm radius, a 0.2 m
tail tube and four fins. Its reference is the largest diameter, 0.1598 m, and the numbers are at a
small angle of attack. The method reads *less* normal force than slender-body theory and puts the
centre of pressure forward of it, so this rocket now reads **less** stable rather than more:

| | the method | slender-body theory | the method's centre of pressure |
|---|---|---|---|
| Mach 2.0 | 3.5371 per rad, at 7.365 calibres | 3.7153 per rad | 0.089 calibres forward |
| Mach 3.0 | 2.8899 per rad, at 7.015 calibres | 3.0815 per rad | 0.169 calibres forward |
| Mach 4.95 | 2.4880 per rad, at 6.681 calibres | 2.6431 per rad | 0.238 calibres forward |

So for a flare of this size the stability margin drops by a tenth to a quarter of a calibre,
growing with Mach number. `a_flared_body_flies_the_method` pins every figure in the table.

The old behaviour is still selectable, for comparing:
`BodyModel::with_supersonic_flare(SupersonicFlare::SlenderBody)` in Rust, or
`{"supersonic_flare": "slender_body"}` in the body model's JSON, reproduces every number from
before this milestone. **Which to fly is not settled here**: neither column has been compared with
a measured flare, and the method is the default because it is the model the rest of the body
already uses faster than sound, not because it is known to be closer.

**What it leaves out.**

- **The one measured flare is not this flare.** [What a marched flare is
  worth](#what-a-marched-flare-is-worth) compares an 18.5° flare on a 2.75° cone, at six speeds,
  three of them with its boundary layer separated. The 10° flare in the table above is not that
  body, so the size of the difference it shows is still a change of model rather than a measured
  correction.
- **The flare's own detachment angle is still the wedge's.** A conical flare on a cylinder sits
  between a wedge and a cone, and nothing here measures where it actually is.
- **Below about Mach 1.5552 the method has no reading for an 18.5° flare on the tests' rocket at
  all.** The corner's isentropic turn runs out before its shock detaches ([the section
  above](#where-a-flares-march-stops)), so the body's table of the method's shares starts there
  and the [join](#bodies-faster-than-sound) carries the reading up from slender-body theory's over
  0.3 Mach. That is continuous — a ±1e-9 probe at the join's start moves the slope 8.5e-10 of
  itself — but it means a flare's march is not used at all at the low end of what a flight uses.
- **A near-flat flare is read by the older, rougher method.** Between about **0.0004° and 0.059°**
  on the tests' rocket, depending on the Mach number, a flare's single element is *reduced*: the
  pressure behind its corner sits just past its tangent cone's while the gradient the tube
  delivers still pushes it away, so hpr reads that element by the generalized shock-expansion
  method instead ([issue #81: the method's limit near a sharp
  tip](https://github.com/nrdptel/hpr-sim/issues/81) is the same reading on a nose). Until
  [M1.8e19: the near-flat flare](../decisions-and-roadmap.md#m1-8e19) the march refused such an
  element behind the nose and the whole body fell back to slender-body theory at every Mach
  number, which was a switch worth −8.3% and 1.16 calibres.
  [A near-flat flare](#a-near-flat-flare) below solves for where the region is at each Mach
  number, says what the change was worth, and gives the one step that is left, which is at most
  +0.129% and 0.0051 calibres.
- **The march ends at the flare, and nothing behind it may carry lift.** Anything behind the flare
  takes slender-body theory's share, which for a tube is nothing — but a part that carries a share
  of its own, such as a small tail cone behind the flare, takes the *whole rocket* off the method
  again, at every Mach number, exactly as a second flare would. A flare followed by a plain tube
  and fins is the layout that flies. Where it does fly, the relaxation the method would give the
  tube behind the flare is left out, which reads a little less stable, not more.
- **Everything here is inviscid.** From Mach 2.96 up TN D-4865 records the boundary layer
  separating ahead of the juncture and reattaching on the flare, which moves the pressure rise
  downstream of where a tangent body puts it. Nothing here models that.


#### What a marched flare is worth

**In short:** [A flare through the method](#a-flare-through-the-method), above, says what hpr
*does* with a flare — it marches the [shock-expansion method](#bodies-faster-than-sound) through
the flare's corner while the shock there stays attached. This section says how close that comes to
a measured flare, which before [M1.8e18](../decisions-and-roadmap.md#m1-8e18), the milestone that
did this work, nothing had.

There is one flared body in the sources whose normal force and pitching moment are printed: NASA
TN D-4865's model 2 ([J68]), a blunt 2.75° cone with an 18.5° flare, in the Langley Unitary Plan
tunnel from Mach 1.50 to 4.63. Against it hpr reads the normal-force slope **−1.9% at Mach 1.90,
+7.0% at 2.30, +13.4% at 2.96, then +51.5% and +50.4% at 3.95 and 4.63**, with the centre of
pressure within 0.05 [calibres](../glossary.md#calibre-caliber) through Mach 2.96 and 0.088 at
3.95. The report's own shadowgraphs show that flare's boundary layer separated ahead of the
juncture from Mach 2.96 up, but the cost only shows in the two fastest rows; why is not settled
here. Below **Mach 1.5289** there is no reading at all, and a flared body falls back to
[slender-body theory](../glossary.md#slender-body-theory) instead.

One body, one flare angle, six speeds, three of them separated: that is the whole of the evidence,
and it is not enough to call the model right — only enough to say where it is not obviously wrong.
The decision record is [ADR-048][adr-048].

**The body.** Two things about it bear on the comparison: its nose is blunter and more compound
than model 1's, so hpr had to learn to hand a blunt tip's cap over on a later piece of a nose; and
the report's own drawing does not quite close, so which of its printed numbers to keep had to be
chosen. Neither is worth much — the closure is worth 0.64 points at most, measured three ways —
but both are choices, so here they are.

Fig. 3(b) (printed p. 91) draws model 2 in base diameters, `d` = 0.583 ft (0.178 m). Its nose is
not a sphere-cone: a 0.257 sphere from the tip, then a **second arc of 0.429** whose
centre sits 0.135 *below* the axis, then the 2.75° cone, then the 18.5° flare. Those three printed
radii fix everything else. The two arcs are internally tangent, so their centres are 0.429 − 0.257
= 0.172 apart, which with the 0.135 offset puts the second centre 0.3635786 aft of the tip; its
tangent to the 2.75° cone then falls at **0.3429960** aft of the tip, matching the drawing's
printed 0.343 to four figures, at diameter **0.5870**, which misses its printed 0.586 by 0.001 of
a diameter. A fourth printed dimension checks the same derivation from the other end: 0.722 runs
from the arc's centre to the flare juncture, and 0.3635786 + 0.722 = 1.08558 against 0.343 + 0.743
= 1.08600, a residual of 0.0004. hpr draws that blend arc as a circular arc — the [tangent
ogive](../glossary.md#tangent-ogive)'s shape with its radius ratio solved for a 0.429 arc — and
the profile it builds misses the drawn circle by 8.3e−17 of a diameter.

The printed dimensions do not quite close, and the gap is the **flare's**. Nose, cone and flare
come to 0.3429960 + 0.743 + 0.523 = 1.6090, the printed length exactly. The cone closes on its own
numbers too: 0.586 + 2 × 0.743 tan 2.75° = 0.65737, against the printed 0.657. The flare does not:
0.657 + 2 × 0.523 tan 18.5° = **1.0074**, against a base that is 1.000 by definition — and
**1.0084** with the nose taken from its three radii rather than from its printed 0.586. So either
the flare is shorter than 0.523, or it is shallower than 18.5°.

hpr keeps both half-angles, because they are the report's **text** and not only its drawing —
*"a blunted cone with a 2.75° half-angle and a flare afterbody having an 18.500° half-angle"*
(printed p. 8), stated to three decimals — along with the nose, the length 1.609 and the base
1.000, which are the numbers the measured coefficients are divided by. What gives is the split of
the length between the cone and the flare: a **0.7576199** cone and a **0.5083841** flare, which
puts the juncture 0.0146 diameters aft of the printed one, at diameter 0.6598 against 0.657.

Two other closures are computed and published beside it, so the choice can be checked rather than
trusted. Keeping the printed lengths and scaling the whole body to a 1.000 base moves the error by
−0.12 to −0.17 points (points here and below are percentage points of error). Keeping the printed
lengths **and** the base and giving up the flare's stated angle instead — 18.5° becomes
18.0864° — moves it by +0.06 to +0.64 points. Over all three the spread is **0.64 points or less**
in the slope and **0.0043 calibres or less** in the centre of pressure, and Mach 1.50 is refused in
every one of them, so nothing below turns on which closure is flown.

**What it is compared with.** Fig. 8(b) (printed p. 102) plots normal force `C_N`, pitching moment
`C_m` and axial force `C_A` — all three as coefficients, `C_m` about the nose tip on the body's
own length — against `α` at 0°, 4°, 8° and 12°, for each of six Mach numbers. The circles are the
report's experiment: the surface pressures of its tables VII to XII integrated over the
**forebody** (printed p. 12), so no base pressure is in them. Every circle was read off the
page's 300-ppi
scan by pixel analysis, the method written for [model 1](#blunt-tips) in
[M1.8e7](../decisions-and-roadmap.md#m1-8e7); the `α` = 0 circles come out at −0.0039 to +0.0043
where they should read 0, which is what the plotting itself is worth. `C_A` is not read, because
hpr's supersonic drag is a separate model this comparison does not touch. hpr's slope and centre
of pressure are fitted the way that section fits model 1 — a straight line through hpr's own `C_N`
at those same four angles, [body lift](#body-lift) included — so the two sides are the same
quantity.

| Mach | measured `C_Nα`, per radian | hpr | hpr's error | measured CP, calibres aft of the tip | hpr | hpr − measured, calibres |
|---|---|---|---|---|---|---|
| 1.5 | 1.650 | none | none | 0.810 | none | none |
| 1.9 | 1.800 | 1.766 | −1.9% | 0.888 | 0.934 | 0.046 |
| 2.3 | 1.667 | 1.784 | +7.0% | 0.921 | 0.926 | 0.005 |
| 2.96 | 1.594 | 1.807 | +13.4% | 0.948 | 0.935 | −0.013 |
| 3.95 | 1.270 | 1.923 | +51.5% | 1.046 | 0.958 | −0.088 |
| 4.63 | 1.303 | 1.960 | +50.4% | 0.998 | 0.975 | −0.023 |

A positive number in the last column means hpr puts the centre of pressure **further aft** than the
tunnel did, which reads as more stable than the rocket is; a negative one reads as less. The
half-calibre the rest of this page uses as a target is the scale to hold them against.

**The flare is most of what is being compared.** Its own share of the body's slope is 52.1% at Mach
1.90 and 60.8% at 4.63, and never below 51.5% (at Mach 2.30) in between. That share acts 1.382 to
1.390 calibres aft of the tip — on the flare itself, which runs from 1.101 to 1.609. So this is a
test of the flare and not of a body that happens to have one.

**How much of the miss is the flare's.** The same report, the same tunnel, the same figure and the
same reading and fit also give **model 1** — a sphere-cone with *no* flare, the body
[Blunt tips](#blunt-tips) already checks. Putting the two side by side separates what the flare
costs from what the rest of the body costs:

| Mach | hpr's error, model 1 (no flare) | hpr's error, model 2 (flared) | the flare adds | the report's own method, model 1 | the report's own method, model 2 |
|---|---|---|---|---|---|
| 1.5 | −1.2% | none | none | −3.3% | +28.6% |
| 1.9 | +0.0% | −1.9% | −1.9 points | +2.0% | +24.8% |
| 2.3 | +7.5% | +7.0% | −0.5 points | +8.5% | +31.6% |
| 2.96 | +12.5% | +13.4% | +0.9 points | +5.2% | +12.1% |
| 3.95 | +29.7% | +51.5% | +21.7 points | +11.3% | +28.8% |
| 4.63 | +32.1% | +50.4% | +18.3 points | +13.6% | +20.3% |

They are not one body with and without a flare — model 1 is an 11.5° cone on a 0.175-diameter nose
radius, 1.755 diameters long, and model 2 a 2.75° cone on a 0.257-diameter one, 1.609 long — so
**the flare adds** column bounds what the flare costs rather than measuring it. Read it as a
signed difference and nothing more: it is *not* a verdict on which body hpr reads better. At Mach
1.90 the unflared model 1 is almost exact (+0.011%) and the flared one is 1.9% low, so in size of
error the flare is the worse row, not the better one.

Taken that way the column still says something clear. **Through Mach 2.96 the flare moves the
error by −1.9 to +0.9 points** — that is, by less than the rest of the body already misses by. At
Mach 3.95 and 4.63 it moves it by 21.7 and 18.3 points, an order of magnitude more, and that is
where the measured `C_Nα` itself falls away — from 1.594 at Mach 2.96 to 1.270 at 3.95 — while
both attached-flow methods on the figure, the report's own and hpr's, stay between 1.57 and 1.96.

The obvious explanation is the flow: from Mach 2.96 the report's shadowgraphs show the laminar
boundary layer separating *ahead* of the juncture and reattaching behind it (printed p. 10), and
nothing in an attached-flow method describes that. How far the report backs it up is worth being
exact about. It blames the separated flow for its **own method's** disagreement with the measured
*pressures* at the high Mach numbers (printed p. 10) and for its over-prediction of *axial* force
(printed p. 12). It says nothing at all about what separation does to the normal force or the
pitching moment. Nor does its method carry a separation signature on this body: it reads +24.8% at
Mach 1.90 and +31.6% at 2.30, both attached rows, against +28.8% and +20.3% at the separated ones —
uniformly high on model 2 and near the tunnel on model 1, which is a large offset this milestone
has not explained. So read the separation as **consistent with** the two fast rows rather than
measured by them. What nothing here settles is why it costs 0.9 points at Mach 2.96, where the
report says it has already begun, and twenty times that at 3.95.

**Where the method has no reading.** At Mach 1.50 hpr refuses model 2 outright, and the refusal is
worth following, because it is not the rule the section above describes. Two limits decide what the
march does with a flare's corner: the steepest **surface angle** whose shock stays attached there,
which is the rule that section sets, and the steepest the march itself can turn the flow through.
Both are in the table below as surface angles of the flare, so they can be read against its 18.5°
and against each other:

| Mach | the flow reaching the corner | the steepest angle its shock holds | the steepest the march takes | the flare is read |
|---|---|---|---|---|
| 1.5 | Mach 1.5241 | 15.4885° | 15.3647° | drawn out, then refused |
| 1.9 | Mach 1.9354 | 24.5773° | 27.3345° | as drawn |
| 2.3 | Mach 2.3072 | 30.0000° | ≥ 30° (the tables) | as drawn |
| 2.96 | Mach 2.8966 | 30.0000° | ≥ 30° (the tables) | as drawn |
| 3.95 | Mach 3.6913 | 30.0000° | ≥ 30° (the tables) | as drawn |
| 4.63 | Mach 4.1636 | 30.0000° | ≥ 30° (the tables) | as drawn |

The turn the corner actually makes is each of those angles less the 2.75° of the cone ahead. From
Mach 2.3 up the third column is clipped at the cone tables' 30° — past that an element has no
[tangent cone](../glossary.md#tangent-cone) to relax toward — and the search behind the fourth
stops at the same place, so neither is a measured edge there and the fourth says so rather than
printing the cap as though it were one. At Mach 1.50 the flare is steeper than the 15.4885° its
corner's shock holds, so [ADR-047][adr-047]'s rule draws it out to 15.4885° — and the march then
refuses that body too, because the steepest flare it can march there is 15.3647°, a tenth of a
degree shallower. The two limits are different things: one is where the corner's shock detaches,
the other where the corner's **isentropic** turn runs out ([Where a flare's march
stops](#where-a-flares-march-stops), [ADR-045][adr-045]), and which is the tighter one changes
with speed. Below where they cross, drawing a flare out to the shock's limit lands past what the
march can do. Swept every 0.005 Mach from 1.05 to 4.63 the reading turns on exactly once, and
bisecting that one crossing to `f64` resolution puts hpr's first reading of model 2 at **Mach
1.5288696**; just below it the two limits agree to five parts in 1e14, both 16.2844275°, so the
reading begins exactly where they cross. (The fixture keeps the whole `f64`; seven figures is what
the three operating systems CI runs agree on, since each regenerates the bisection to within a
part in 1e12 of the others.) That is a different number from the Mach 1.5552
[the section above](#a-flare-through-the-method) quotes for the same 18.5° flare, and it should be
— the body ahead of the corner is different, so the flow it delivers there is different, and the
crossing moves with that flow. Below that a flared body takes [slender-body
theory](../glossary.md#slender-body-theory) instead, carried up over 0.3 Mach by the
[join](#bodies-faster-than-sound) — which is continuous, but means the method is not used at the
low end of what a flight flies. The report says the same thing about its own method at that speed:
*"at M∞ = 1.50, the shock wave produced by the flare is not theoretically attached"*
(printed p. 10), and its method reads +28.6% there against +2.0% on model 1.

The rows above are in
[`validation/fixtures/aero/marched-flare.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/marched-flare.json),
written by `cargo xtask aero`; the readings in
[`tn-d-4865-flared-cone.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/tn-d-4865-flared-cone.json),
with how each circle was read. A test holds every table here to the fixture, cell by cell. The
reading is hpr's own rule and not a second copy of it: `hpr-design` has no spherical-cap nose, so
model 2 cannot be flown through a `Rocket`, and `the_flare_is_read_as_the_model_reads_it` pins the
fixture's reading against the model's own shares — share by share, to a part in 1e12, on a flared
body the design route *can* express, at angles the corner's shock holds and at angles it does not.
That pins the arithmetic, not the physics: an error in the rule itself would be in both and pass.

**What it leaves out.**

- **Three of the six rows are a separated flare.** From Mach 2.96 the report's shadowgraphs show
  the boundary layer separating ahead of the juncture; nothing in hpr models that, and the +51.5%
  and +50.4% at Mach 3.95 and 4.63 are what those rows cost on this body — which the report's own
  words make *consistent with* separation rather than caused by it, since it never says what
  separation does to the normal force. No source here says how a
  separated flare scales with the flare's angle, its length or the boundary layer's thickness, so
  those two numbers do not transfer to another flare.
- **One body and one flare angle.** 18.5°, on a 2.75° cone, at six speeds. Nothing here measures a
  shallow flare, a steep one, a flare behind a cylinder rather than a cone, or a flare on a
  pointed nose. A reader with a different flare has no measured error to apply: what this section
  supports is that hpr's flare is *not obviously wrong* where the flow stays attached, not that it
  is good to 7% on some other body.
- **The drawn-out reading past the limit is measured against nothing at all.** The one row where
  [it](#a-flare-through-the-method) would have applied is the row the march then refused, so the
  rule above the corner's limit is still a construction chosen for continuity.
- **The measurement is the forebody only.** Its `C_N` and `C_m` are integrated surface pressures
  with no base term, which is what hpr's body model computes too — but it also means the tunnel's
  own balance never weighed this body, and a reading error of about 0.004 in `C_N` sits under
  every circle.
- **`C_A` is not compared.** The same figure plots axial force, and the report notes its own
  method reads it high where the flare separates. hpr's supersonic drag is a different model with
  its own [checks](#drag-verification); this milestone did not touch it.
- **The drawing is 1% inconsistent and one closure had to be chosen.** The spread over the three
  closures is small (0.64 points, 0.0043 calibres) and the flare's own angle is inside it, but it
  is the drawing's disagreement with itself, not an error bar on the measurement.
- **No target was set, and none is met or missed here.**
  [M1.8e18](../decisions-and-roadmap.md#m1-8e18) asked what a marched flare is worth, not that it
  reach a number. The tables above are the answer.


#### A near-flat flare

**In short:** a flare that opens by very little turns the flow so little that the [second-order
shock-expansion method](#bodies-faster-than-sound)'s own pressure curve has nothing left to
describe, and the march falls back on the older *generalized* method for that one element. Until
[M1.8e19: the near-flat flare](../decisions-and-roadmap.md#m1-8e19) hpr refused to read such an
element at all behind the nose, which took a rocket with a flare of about a third of a
millimetre's rise off the method entirely. It now reads it. This section says which flares those
are — the two angles that bound them are solved from two equations about the corner's own flow,
rather than found by bisecting the model's refusal — what the change was worth, and the one small
step that is left.

**How far to trust it.** No wind tunnel has measured a flare this shallow, and the report does not
say what it would have done here, so what follows is hpr's own reading of the report's own limit,
chosen because it is continuous in the flare's angle and smooth through the region — not because
it is known to be nearer the air. What changed is which model runs, not how well either matches a
measurement. And **where the region sits depends entirely on the body ahead of the corner**: a few
thousandths of a degree on the rocket measured here, nearly a degree on the body of
[Where a flare's march stops](#where-a-flares-march-stops) — a pointed 2.75° cone and five
calibres of tube, whose radius is nearly four times as large.

**What a flare that small does to the method.** *The march* is the walk along the
[tangent body](#bodies-faster-than-sound)'s straight elements, from the nose tip aft, that the
method makes. It fixes the pressure just behind each
corner from the [Prandtl–Meyer](../glossary.md#prandtlmeyer-expansion) turn there, and then lets it
relax along the element toward the pressure on that element's [tangent
cone](#bodies-faster-than-sound), as

*p* = *p*_c − (*p*_c − *p*₂) e^(−η),  η = (∂*p*/∂*s*)₂ (*x* − *x*₂) ⁄ ((*p*_c − *p*₂) cos δ₂)

(NACA TN 3527 eqs. 8 and 9), where *x* − *x*₂ is the distance back from the corner. The same
exponent written as a rate per metre is the `k` of [What a crossing is, and what it
costs](#what-a-crossing-is-and-what-it-costs). The symbols are the method's own, listed under
[Bodies faster than sound](#bodies-faster-than-sound): δ an element's angle to the axis, *p* the
pressure over the free stream's, *s* distance along the surface, *r* the radius at the corner, Ω a
stream tube's widening and *B* = γ*pM*²/(2(*M*² − 1)); θ below is the flare's own turn through its
corner, and subscript 1 is the state the body ahead delivers, 2 the state just behind the corner.

That is a curve that starts at *p*₂ and walks one way, toward *p*_c. It can only do that if the
gradient just behind the corner points at *p*_c — the report keeps the form only for η ≥ 0 (p. 13)
and says that at η = 0 "all equations reduce to those given by the generalized shock-expansion
method", whose pressure is simply constant along the element.

On a near-flat flare the two disagree, for a reason you can picture. The flow arrives at the flare
along a long tube, where the pressure is still climbing back toward the free stream's after the
nose let it down: it is *below* the free stream and *rising*. The flare's corner compresses it a
little. Turn far enough and the pressure lands well above the tiny cone's, and the gradient behind
the corner turns downward with it — everything agrees and the method runs. Turn less and the
pressure stays below the cone's, still rising toward it — again everything agrees. In between there
is a band where the compression has already carried the pressure just past its cone's value while
the tube's own climb still pushes it further away. It has to rise, overshoot and come back, and one
exponential cannot rise and fall. So η is negative there, and the element is *reduced* to the
generalized method ([issue #81: the method's limit near a sharp
tip](https://github.com/nrdptel/hpr-sim/issues/81) is the same reading on a nose).

**The region's two edges are solved from the corner, not searched for.** Each of the two
quantities whose signs must agree is a smooth function of the flare's turn, and on every corner
state checked here each has a single zero — so the signs disagree on the open interval between
those two zeros and nowhere else:

| the turn | what is zero there | what it means |
|---|---|---|
| the **crossing** | *p*_c − *p*₂ | the compression lands the pressure exactly on its tangent cone's, and η has a pole |
| the **balance** | (∂*p*/∂*s*)₂ | the corner's own compression exactly cancels the climb the tube delivers, and η is zero |

*Single zero* is an observation over the corner states checked, not a proof: a corner state that
broke it would put a reduced element outside the interval. Which of the two is the shallower is
not fixed either — on this rocket the crossing is below the balance from Mach 1.5 up, and below
that the order swaps.

Both are properties of the flow the body hands to the corner — its Mach number, its pressure, the
gradient it carries, the radius there, the angle ahead and the free stream it was read in — and
`flare_reduction_turns_rad` takes exactly that and nothing else. The balance is TN 3527's eq. 4
set to zero and rearranged, sin(δ₁ + θ) = (Ω₁/Ω₂(θ)) (sin δ₁ + *r* (∂*p*/∂*s*)₁ ⁄ *B*₁), which
iterates on itself. The crossing is *p*₂(θ) = *p*_c(δ₁ + θ): an isentropic turn on one side, a
cone solution on the other. It is bracketed over the turns a widening corner can make at all —
from a surface lying along the axis up to the isentropic turn running out or the cone tables' 30°,
whichever comes first — and found by false position from the turn that would bring the pressure
back to the free stream's.

On the tests' flared rocket — an ogive nose 0.25 m long on a 27 mm radius, a 0.7 m tube and a
0.3 m conical flare — they are these:

| Mach | crossing | balance | a flare between them rises, over 0.3 m, by |
|---|---|---|---|
| 2.00 | 0.000403337° | 0.000454464° | 2.1 to 2.4 µm |
| 2.20 | 0.000901825° | 0.001041403° | 4.7 to 5.5 µm |
| 3.00 | 0.006619249° | 0.008121929° | 35 to 43 µm |
| 4.00 | 0.023088893° | 0.029499725° | 0.12 to 0.15 mm |
| 4.70 | 0.038161270° | 0.049811274° | 0.20 to 0.26 mm |
| 5.00 | 0.044649637° | 0.058820517° | 0.23 to 0.31 mm |

`the_turns_a_reduced_element_lies_between_come_from_the_corners_own_state`, in
[`shock_expansion.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-aero/src/shock_expansion.rs),
checks at eight Mach numbers that an angle a millionth either side of each edge falls on the right
side of the march's own reduction, and that the midpoint between them is reduced. That is a spot
check at each edge, not an exhaustive sweep of the angles in between.

Those are the same numbers [issue #117: the near-flat band the march used to
refuse](https://github.com/nrdptel/hpr-sim/issues/117) reported after bisecting the model's own
refusal — the band it quoted, 0.03816127° to 0.05882052°, is the crossing at Mach 4.70 and the
balance at Mach 5, and the 0.00090182° it quoted is the crossing at Mach 2.20 — but they are now
read off two equations rather than found by trying the whole model against a sign test. Note what
the table shows and that band hides: the region is **not one interval in the angle**. It moves
with the Mach number, so a 0.04° flare is reduced at Mach 4.70 and marched at Mach 2.

**How far to trust those digits.** Not the search's accuracy any more, but the tangent cone's. Up
to a half-thousandth of a radian (0.029°) hpr's [cone flow](#bodies-faster-than-sound) is
slender-cone theory's closed form, and the crossing closes to the last bits of an `f64` — the
residual it leaves in the pressure is under 2e-14 at Mach 2.00, 2.20 and 3.00. Above that angle
the cone flow is a Taylor–Maccoll integration, blended with the closed form up to 0.0573°, so the
two edges past Mach 4 are read off the blend; there the residual is that integration's own, about
1e-10 of the free stream's pressure. Divided by how fast the gap closes with the turn, that is
about 2e-10° — close to the 2.6e-10° the three operating systems CI runs were seen to spread the
band's lower edge over, so that spread was the cone's and not the bisection's.

**Neither residual is promised to be zero**, and on a body whose cone flow is harder it is
larger — 4e-9 of the free stream's pressure has been seen on a fatter body at Mach 2.6. So
`flare_reduction_turns_rad` returns both of them beside the turns, and a caller who needs the
digits should read them.

**What hpr does now, and what changed.** A reduced element is read by the generalized method
wherever it has a tangent cone of its own: constant pressure and constant loading along it, which
is what the report says the equations become. Before
[M1.8e19](../decisions-and-roadmap.md#m1-8e19) that reading was allowed only on the nose, and
anywhere behind it the march refused. Because hpr builds a body's table of the method's shares
downward from Mach 5 and needs the whole 0.3 Mach of the [join](#bodies-faster-than-sound) inside
it, one refused row near the top took the table away and dropped the **whole body** to
[slender-body theory](../glossary.md#slender-body-theory) at every speed. Two switches came of
that, and both are gone:

| drawing this | was worth | is worth |
|---|---|---|
| a flare of 0.05882052°, the band's steep edge, at Mach 3 and 4° | −8.30% of the normal force and 1.16 [calibres](../glossary.md#calibre-caliber) | nothing |
| a flare of 0.00090182°, which lifted the table's start from Mach 1.2 to Mach 2.2, at Mach 2 and 4° | −4.62% and 0.75 calibres | nothing |

The sizes in the middle column are still measured, because they are the size of the fallback the
model used to drop to: `a_near_flat_flare_marches_every_row_and_the_fallback_is_still_measured`,
in [`model.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-aero/src/model.rs), reads
the same rocket on `SupersonicFlare::SlenderBody` and finds them again. The table's start is now
Mach 1.2 at fifteen flare angles from 0° to 1°, including 0.00024°, 0.00025° and 0.0003° — three
angles a hair apart that used to give three different answers.

**The one step that is left, and how big it is.** At the crossing itself η has a pole, and that
does leave a step — not in the pressure, which rides through because the gap it multiplies is zero
there, but in the *loading*. On the side the method still owns, η runs to +∞ as the turn
approaches the crossing, so the element sheds its corner's loading onto its tangent cone's within
its own length; on the reduced side it holds the corner's. The two differ, so the reading steps. On the whole rocket at 4°, measured
either side of that Mach number's own crossing with a ±1e-9° probe by
`a_near_flat_flare_reads_through_and_leaves_only_the_corners_crossing`, in
[`model.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-aero/src/model.rs)
(Mach 4.95 rather than 5 because the body's normal force stops at Mach 5):

| Mach | at a flare of | normal force | centre of pressure |
|---|---|---|---|
| 2.00 | 0.000403337° | +0.00032% | −0.0000016 calibres |
| 3.00 | 0.006619249° | +0.011% | +0.00018 calibres |
| 4.00 | 0.023088893° | +0.055% | +0.0017 calibres |
| 4.95 | 0.043584193° | +0.129% | +0.0051 calibres |

**A step, not a slope — except on the first row.** Widening the probe a hundredfold, to ±1e-7°,
leaves the figure where it is at Mach 3, 4 and 4.95, which is what says it is a step rather than
the reading's ordinary movement. At Mach 2 it does not: +0.00032% is already about what the
reading itself moves over a ±1e-7° probe there, so that row is an upper bound on the step, not a
measurement of one.

The worst of those is a 64th of the switch it replaced in the force and a 227th of it in the
centre of pressure — the step at Mach 4.95 against the switch measured at Mach 3, so that is a
comparison of sizes, not of the same flight condition. It is also not a new question: it is the
loading through a tangent-cone crossing, which is [what a crossing
costs](#what-a-crossing-is-and-what-it-costs) inside a segment, and which is open as [issue #108:
the loading through a crossing](https://github.com/nrdptel/hpr-sim/issues/108). The region's other
edge, the balance, has no step at all — η is zero there, so the exponential form and the
generalized method are the same reading, and the two branches meet.

**What it leaves out.**

- **A cylinder's and a boattail's reduced elements are still refused**, and still take the whole
  body off the method. Neither has a tangent cone of its own — a cylinder relaxes toward the free
  stream and a boattail toward footnote 8's constant — so there is nothing for the reduction to
  relax toward, and a reduced element holds its corner's loading over any length. Nothing here
  measures what that would be worth, so the refusal stands: [issue #123: a cylinder's or a
  boattail's reduced element](https://github.com/nrdptel/hpr-sim/issues/123). How near is it? An
  ogive nose on a tube — 0.25 m on a 27 mm radius, with tubes of 0.7 m, 3 m and 6 m — marches
  every row from Mach 1.2 to Mach 5 with nothing reduced, so it is not something a plain rocket
  walks into. What does hit it is TN D-4865's own Newtonian start on the Arcas Robin's nose from
  Mach 3.96, which is a reason hpr does not use that start
  ([The two starts](#the-two-starts)).
- **The generalized method is the older, rougher one.** Reading an element with it is a real
  choice, not a formality, and TN 3527 does not say it is what it would have done. What is checked
  here is that the choice joins the second-order reading continuously at the balance and that it
  makes the reading smooth in the flare's angle — not that it is closer to a measured flare. No
  measured flare in this band exists.
- **Below about a millionth of a radian the flare is not drawn at all.** Corners turning by less
  than that are merged into the element ahead of them, because two tangents that nearly coincide
  meet at an ill-conditioned point. That covers every flare shallower than about 0.00006°, which
  at the bottom of the table's range swallows both turns: at Mach 1.2 the crossing is 0.00000023°
  and the balance 0.00000018°, so there is no corner there to reduce and nothing the reading could
  switch on.
- **Only the flare's own element was measured.** The two turns are solved for a corner behind a
  body; where a body has several corners that could be reduced at once, nothing here says how their
  readings combine. Nor does anything here say whether the generalized reading or the slender-body
  fallback it replaced was the nearer of the two to the air: what is claimed is that one of them
  moves smoothly with the shape and the other jumps.
- **There is no rule of thumb for "is my flare one of these?".** The region belongs to the corner,
  so the only way to ask is to run the model:
  [`ShockExpansionBody::aft_flow`](https://nrdptel.github.io/hpr-sim/api/hpr_aero/shock_expansion/struct.ShockExpansionBody.html#method.aft_flow)
  on the body ahead of the flare, at the Mach number you care about, then
  [`flare_reduction_turns_rad`](https://nrdptel.github.io/hpr-sim/api/hpr_aero/shock_expansion/fn.flare_reduction_turns_rad.html)
  on what it returns.


#### A step in radius

**In short:** a **step** is a joint where one part's radius does not match the next one's, so the
rocket's outline jumps rather than bending — a 54 mm tube butted straight onto a 75 mm one, or a
coupler left standing proud of the airframe. That stops hpr's [second-order shock-expansion
method](#bodies-faster-than-sound), *the march*: it walks a chain of straight elements, the
[tangent body](#bodies-faster-than-sound), from the nose tip aft, and it needs an outline without a
jump in it. What hpr does about that is drop the **whole body** to [slender-body
theory](../glossary.md#slender-body-theory), at every speed — a rocket with a step reads as though
the method did not exist. [M1.8e15](../decisions-and-roadmap.md#m1-8e15) measured what that costs
and tried the obvious fix; the fix was worse, so the behaviour is unchanged and the cost is
published instead. The decision record is [ADR-049][adr-049]; the work left over is [issue #87: a
step takes the whole body off the method](https://github.com/nrdptel/hpr-sim/issues/87), open and
not on the roadmap. **Nothing measures a stepped body faster than sound**, so neither the present
reading nor any replacement has a reference.

**What it costs.** On the tests' straight rocket — a tangent-ogive nose 0.25 m and three tubes of
0.7, 0.05 and 0.3 m, all 27 mm in radius — at Mach 3 and 4°, with the reference diameter pinned at
54 mm so that every row is divided by the same area. With no step it reads `C_N` = 0.899592, its
centre of pressure 16.9492 [calibres](../glossary.md#calibre-caliber) aft of the nose tip. Each row
is the change from that. **Down** means the body narrows from that joint aft, **up** that it widens:

| the step, and which joint it is at | normal force | centre of pressure |
|---|---|---|
| down 2.8e−11 m, the first size measured past the threshold, at any of the three joints | −8.65% | +1.0285 calibres |
| down 1 mm, at the nose's joint | −10.62% | +1.1938 calibres |
| down 1 mm, at the last joint | −10.29% | +0.9949 calibres |
| down 2 mm, at the nose's joint | −12.55% | +1.3593 calibres |
| down 2 mm, at the last joint | −11.89% | +0.9597 calibres |
| up 2.8e−11 m, at any of the three joints | −8.65% | +1.0285 calibres |
| up 1 mm, at the nose's joint | −6.72% | +0.8674 calibres |
| up 1 mm, at the last joint | −7.05% | +1.0644 calibres |
| up 2 mm, at the nose's joint | −4.75% | +0.7064 calibres |
| up 2 mm, at the last joint | −5.41% | +1.0987 calibres |

At the threshold the shape is flush to a part in a billion either way, so the whole difference there
is the method itself — the same wherever the step sits and whichever way it goes. Past that the
shape itself starts to matter, and the two directions part: a step **down** takes area off the body
and costs more, a step **up** adds area that carries slender-body normal force of its own and costs
less. The centre of pressure moves **aft** in every row, so a rocket that trips this reads *more*
stable than the same rocket drawn flush. That extra margin is more likely optimistic than real: the
model it falls back to reads 15% to 50% below the wind tunnel on the Arcas Robin's body faster than
sound ([The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight)). If you
can draw the joint as a short transition instead of a butt joint, the body keeps the method; hpr's
own `radius_step` warning ([the design model's checks](design.md#checks)) is what tells you a design
has tripped this.

`a_step_takes_the_whole_body_off_the_method` pins every figure in this section and the flush
rocket's own readings with it; [Checking a claim](../checking-a-claim.md) says how to run a named
test.

**Where the threshold is.** It is a **pair**, not one number, and which of the two binds depends on
the joint:

| the joint | what binds | on the tests' bodies |
|---|---|---|
| radius changes, slope does not (tube to tube), either direction | the tangent body merges two elements whose radii agree to a billionth of the radius | 1e−9 × 27 mm = **2.7e−11 m** |
| slope changes too (a step **up** at a boattail's fore end) | the elements' corners have to stay in order along the body | 1e−12 × 1.3 m × 0.1 = **1.3e−13 m** |

Both are bisected. The second is 208× finer, and it depends on the body's **length** and the change
of slope rather than on its radius — so it is not a property of the step at all. It bites on the
commonest high-power shape there is: on the tests' finned rocket, a boattail whose fore radius is
27.0000000000002 mm rather than 27 mm loses the method for the whole body, worth **−11.34% and
1.0951 calibres** — larger than the tube-to-tube switch above.

Neither number is a judgement about steps. They are the widths of the rounding the tangent body can
absorb, and **every step anyone could build or draw is far past both**, so in practice a step always
takes the body off the method. (A third, much looser test — the run's own *coverage gate*, which
asks how much of the body the method can cover, at a millionth of the fore area or 13.5 nm of radius
here — is what actually refuses every step bigger than that. It gives the same reading, and the test
pins which of the two owns which range.)

**What a fix has to handle.** The obvious fix is to stop the march *at* the step and let the body
ahead of it keep the method, the way the run already ends at a flare. That was built and measured,
and it failed three ways. The numbers in this list were taken on that prototype, which was **not
kept**: unlike the tables above, no committed test reproduces them, and [ADR-049][adr-049] records
how each was measured and where the prototype lives.

- **What is behind the step.** [ADR-034][adr-034], the decision that the method covers a body or
  nothing, rejected mixing the two models on a measured case. A step's remainder is supposed to be
  a plain tube, but "the run stopped at a step" does not make it one: with a boattail behind the
  step, the mixture's centre of pressure lands at 16.7209 calibres — **forward of both** pure
  models, the method's 16.7286 and slender-body theory's 17.8237. A reading outside the envelope of
  both models it is made of is the pathology [ADR-034][adr-034] measured.
- **It does not close the band it was meant to close.** The prototype removes the step's switch at
  a tube-to-tube joint, but at a joint whose slope changes it is the corner ordering that refuses
  the body, at 1.3e−13 m — so a boattailed rocket still loses the method, worth −11.34% and 1.0951
  calibres. The commonest shape it was supposed to help is the one it does not.
- **Which shape stopped the run, not whether the joint was flush.** The prototype keyed off the
  joint: any reason the run closed — a *non-conical* flare, a lip out of its wake — kept the
  forebody marched as soon as its fore radius was a picometre off, a **new** jump of +7.2% and 0.69
  calibres where today the reading is continuous. That one is a property of how the prototype was
  built rather than of the idea, and a fix keyed off the shape would not have it; it is listed here
  because it is what a fix has to get right, not as evidence the idea cannot work.

**What it leaves out.**

- **The step's own force is slender-body theory's, at any speed.** `(2/A_ref)ΔA` at the joint, the
  limit of a transition whose length goes to zero — [B67] p. 18 assumes no discontinuities, so this
  goes beyond its source, and there is no compressibility term. A forward-facing step at supersonic
  speed stands a detached shock with a separated pocket ahead of it; none of that is modelled.
- **No source gives a stepped body's normal force faster than sound.** MIL-HDBK-762 treats a
  rearward-facing step only as base drag, TN 3527 ([SD56]) needs a continuous profile, and nothing
  else pinned here covers one. So "take the whole body off the method" is not known to be right
  either; it is the reading that does not mix two models, which is the only argument for it.
- **One body, one placement sweep.** Three joints on one rocket at one Mach number and one angle,
  and the large steps up carry a moving reference diameter with them.


### Blunt tips

What this covers: the body faster than sound when the nose's tip is blunt or vertical, as on
power-series noses with `n` below 1, Haack series (the von Kármán and L-V Haack) and elliptical
noses, whose profile leaves the tip at 90°. How far to trust it: the cap comes from a NASA method
checked only on spherical caps. On that report's own sphere-cone, compared as its tunnel measured
it (at the plotted angles, body lift included), hpr reads −1.2% to +32.1%: close through Mach 2.3,
high from Mach 2.96, where the report's own method reads +5.2% to +13.6%. On the Arcas Robin's
power-series nose the cap is an extrapolation. There, like for like, the body reads +37.2% at Mach
1.5 and +13.7% to +25.9% from Mach 1.8 to 2.96, and within 5% past Mach 3. Against the smooth
secant ogive fitted to the same nose it reads lower at every Mach number: closer to the tunnel at
nine of the eleven rows, by 0.7 to 5.8 points, and past Mach 4 it crosses into under-prediction and
lands 0.6 to 1.5 points further out. A nose that is
nearly a cone but for a vanishing tip carries a bias nothing here measures
([issue #101](https://github.com/nrdptel/hpr-sim/issues/101), below). No validation flight reaches
the speeds where any of this applies.

**Why a cap.** The [shock-expansion method](#bodies-faster-than-sound) replaces the nose by
straight *elements*, short cones and frustums each tangent to the profile, and starts at a
pointed tip, where the air flows as it does over a cone. A vertical tip has no such cone: the
shock stands off the nose, and the air just behind it is slower than sound. Jackson, Sawyer and
Smith ([J68]) handled blunt noses by giving the tip's *cap* [Newtonian](../glossary.md#newtonian-theory)
pressures and handing over to the method where the flow behind the cap is fast again, the
*handover*. hpr does the same.

**The cap.** Newtonian theory takes the pressure from the angle `δ` between the surface and the
wind: `C_p = C_p,max sin²δ` ([J68] eq. 1, p. 5). `C_p,max` is the pressure coefficient at the
*stagnation point*, the tip, where the air comes to rest behind a normal shock: it follows from the
*pitot pressure* a probe would read there, the Rayleigh pitot formula ([R1135] eq. 100). At a small angle of
attack the windward side meets the wind a little more steeply, so the cap carries
`C_p,max sin δ cos δ` of loading in the method's terms (a hemisphere then carries its Newtonian
drag turned into the body's axes, `C_p,max/2`, as it must; test
`a_hemisphere_carries_its_drag_turned`).

**The handover.** The method takes over where the surface's slope falls to the largest angle a
wedge can turn the flow through with its shock attached: 12.1° at Mach 1.5, 22.97° at Mach 2
([R1135] eqs. 138 and 168). The report chose this point "simply because it gave the best
agreement with the available data in the low supersonic-speed range" ([J68] p. 5). hpr caps the
handover at 24°, which the wedge's angle passes at Mach 2.06. The cap is there because the method
needs the normal-force slope of a cone tangent to the body, and TN 3527's chart stopped at 24°
([SD56] Fig. 2). Those slopes now reach 30° ([ADR-042][adr-042]) and the cap has not followed,
because the method's march does not carry it that far.
[What the cap is worth](#what-the-cap-is-worth) measures what moving it would buy and what it
would cost.

**How much of the nose the cap covers** depends strongly on speed. Where it ends, as a share of
the nose's length and of its base radius:

| nose | Mach 1.25 | Mach 1.5 | Mach 2 | Mach 3 |
|---|---|---|---|---|
| arcas robin, the committed nose | 59.1% / 0.72 | 5.8% / 0.16 | 0.9% / 0.05 | 0.8% / 0.05 |
| von Karman, five calibres | 47.8% / 0.69 | 4.1% / 0.12 | 0.3% / 0.02 | 0.2% / 0.01 |
| power series n = 0.5, five calibres | 29.2% / 0.54 | 5.4% / 0.23 | 1.4% / 0.12 | 1.3% / 0.11 |
| elliptical, two calibres | 65.3% / 0.94 | 34.9% / 0.76 | 13.9% / 0.51 | 12.8% / 0.49 |
| TN D-4865's sphere-cone (model 1) | past the sphere: the method doesn't hold | 7.8% / 0.34 | 6.0% / 0.32 | 5.9% / 0.32 |

Near the join's start a slender nose leans on Newtonian pressures over far more of itself than
anything the report checked; by Mach 2 the cap is a percent or so of the nose, less than the
report's own. The join's weight rises from 0 at its start to 1 a third of a Mach number later,
which damps that, but read a vertical tip's numbers between the join's ends as the blend they are.

A power-series nose meets its base at a slope of `n/(2f)`, with `f` its length over its diameter,
and the cap can't end on the nose while that is steeper than the handover's angle. So such a nose
takes the method no earlier than the Mach number where the handover's angle passes its base's,
and its join to slender-body theory starts there rather than at Mach 1.2: for `n` = 0.5, Mach 1.23 at
3 diameters long, 1.49 at 1.2 diameters; the Arcas Robin's from 1.22. One shorter than `n`/0.89
diameters (0.56 for `n` = 0.5) never takes it, since the handover stops at 24°, and keeps
slender-body theory: hpr doesn't warn, and
[`AeroModel::supersonic_body`](../api/hpr_aero/model/struct.AeroModel.html#method.supersonic_body)
returns `None`. Haack and elliptical noses end level, so the cap always ends on them.

**A blunt nose can be more than one shape, and the cap may end on any of them.** Since
[M1.8e18](../decisions-and-roadmap.md#m1-8e18), a nose that *starts with a sphere* carries on
through the curved, widening shapes behind that sphere, and hpr looks for the handover along all
of them rather than in the sphere alone. TN D-4865's own model 2 needs it: its nose is a sphere
blended into a 2.75° cone by a second arc, and the sphere is still at 38.3° where the arc takes
over, steeper than the 24° cap at any speed, so the handover always falls on the arc
([What a marched flare is worth](#what-a-marched-flare-is-worth)).

Everything else reads exactly as it did before, on purpose. A *pointed* nose is one shape however
many curved shapes follow it, so a curved transition behind one is still the afterbody. And the
search stops at the first shape that is straight or narrows: a cap that reached a cylinder or a
boattail would hand the flow over at no angle at all, with none of the total pressure the tip took
out of it, so a nose steeper than the handover all the way to one is still refused — which is the
case in the paragraph above, and its numbers are unchanged.

**Behind the handover.** hpr starts the method there as it starts at a pointed tip, with the flow
on the *tangent cone*, the cone that touches the body at the handover. The report starts it from
the Newtonian pressure instead. What that choice is worth is set out in
[The two starts](#the-two-starts), after the checks below.

*A worked example.* The report's sphere-cone at Mach 1.5: a nose radius of 0.175 base diameters
on an 11.5° cone. The pitot pressure is 3.413 times the free stream's, so `C_p,max` = 2.413/(γM²/2) =
2.413/(0.7 × 1.5²) = 1.532. The wedge's largest angle is 12.11°, reached on the sphere
0.175 (1 − sin 12.11°) = 0.138 diameters behind the tip. On a sphere, with `θ` the angle from the
tip, the loading `C_p,max sin δ cos δ` integrates over the cap to `C_p,max sin⁴θ/2` on the
sphere's own cross-section; to `θ` = 90° − 12.11° that is 0.700, and 0.086 on the base (times
0.35²). The cone behind it, marched from the flow on a 12.11° cone, carries the other 1.595 of
hpr's 1.681.

**How it was checked.** Against the report's own model 1, measured at Mach 1.50 to 4.63 ([J68]
Fig. 8(a), p. 101, read from the scan by pixel analysis to about ±0.003, the plotting itself
good to about ±0.01), compared as [ADR-036][adr-036] compares the Arcas Robin: hpr's `C_N` at the
plotted 0° to 12°, the method's slope with [body lift](#body-lift) (Jorgensen's, for a body of
fineness 1.75, shorter than his Fig. 4 covers), fitted with a straight line just as the measured
`C_N` and `C_m` are, and the report's own method fitted the same way; per radian on the base, the
centre of pressure in base diameters from the tip:

| Mach | `C_Nα` measured, per rad | the report's method | vs measured | hpr | vs measured | CP measured, diameters | hpr |
|---|---|---|---|---|---|---|---|
| 1.5 | 1.908 | 1.844 | −3.3% | 1.885 | −1.2% | 1.00 | 1.03 |
| 1.9 | 1.926 | 1.964 | +2.0% | 1.926 | +0.0% | 1.03 | 1.02 |
| 2.3 | 1.833 | 1.989 | +8.5% | 1.971 | +7.5% | 1.03 | 1.02 |
| 2.96 | 1.786 | 1.878 | +5.2% | 2.009 | +12.5% | 1.06 | 1.02 |
| 3.95 | 1.618 | 1.800 | +11.3% | 2.099 | +29.7% | 1.05 | 1.03 |
| 4.63 | 1.550 | 1.762 | +13.6% | 2.048 | +32.1% | 1.09 | 1.03 |

Past Mach 2.3 hpr reads high twice over. At Mach 3.95 and 4.63 its slope at `α → 0` is 13% to 21%
above the measured one, and its body lift lifts its fitted slope 25% to 27% above that, where the
measured curve rises only 9% to 16% above its own. The slopes at `α → 0`, beside it and not
judged, with the measured one fitted with a curve two ways, as the decision record on comparing
with a wind tunnel, [ADR-036][adr-036], asks:

| Mach | measured, `α\|α\|` fit | measured, `α³` fit | hpr | hpr from the report's start |
|---|---|---|---|---|
| 1.5 | 1.741 | 1.810 | 1.681 | 3.392 |
| 1.9 | 1.979 | 1.952 | 1.711 | 1.960 |
| 2.3 | 1.657 | 1.717 | 1.713 | 1.792 |
| 2.96 | 1.646 | 1.684 | 1.702 | 1.700 |
| 3.95 | 1.433 | 1.484 | 1.678 | 1.625 |
| 4.63 | 1.339 | 1.419 | 1.613 | 1.484 |

And the Arcas Robin's committed design, its power-series nose, cylinder and boattail with the lip
left off, to show the cap's own effect (the lip itself carries nothing:
[A lip in a boattail's wake](#a-lip-in-a-boattails-wake)), through a flight's path, fitted
at the tunnel's plotted angles as
[Checking the shock-expansion method](#checking-the-shock-expansion-method) fits them, beside the
secant ogive fitted to the same nose:

| model | Mach | measured | committed nose | vs measured | fitted ogive | vs measured | cap to `r/R` |
|---|---|---|---|---|---|---|---|
| short | 1.5 | 2.192 | 3.007 | +37.2% | 3.090 | +41.0% | 0.163 |
| short | 1.8 | 2.613 | 3.289 | +25.9% | 3.312 | +26.8% | 0.070 |
| short | 2.3 | 3.078 | 3.597 | +16.9% | 3.651 | +18.6% | 0.045 |
| short | 2.96 | 3.284 | 3.837 | +16.8% | 3.936 | +19.8% | 0.045 |
| short | 3.96 | 3.884 | 3.944 | +1.5% | 4.168 | +7.3% | 0.045 |
| short | 4.63 | 4.149 | 3.948 | −4.8% | 4.288 | +3.4% | 0.045 |
| long | 1.8 | 3.159 | 3.769 | +19.3% | 3.792 | +20.0% | 0.070 |
| long | 2.3 | 3.525 | 4.070 | +15.4% | 4.124 | +17.0% | 0.045 |
| long | 2.96 | 3.868 | 4.398 | +13.7% | 4.497 | +16.3% | 0.045 |
| long | 3.96 | 4.455 | 4.426 | −0.7% | 4.655 | +4.5% | 0.045 |
| long | 4.63 | 4.615 | 4.424 | −4.1% | 4.777 | +3.5% | 0.045 |

The rows are in
[`validation/fixtures/aero/blunt-tips.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/blunt-tips.json),
written by `cargo xtask aero`; the readings in
[`tn-d-4865-sphere-cone.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/tn-d-4865-sphere-cone.json).
A test holds these tables to the fixture, cell by cell. The committed nose reads within 3.8
points of the fitted ogive up to Mach 2.96 and 5.1 to 8.2 points below it past Mach 3. Below it is
not always closer: past Mach 4 its error changes sign, so at Mach 4.63 it reads −4.8% where the
ogive reads +3.4%, 1.5 points further from the tunnel. Over the eleven rows it is nearer at nine. Below Mach 3 both read high, most at Mach 1.5, for
the reasons in [Checking the shock-expansion method](#checking-the-shock-expansion-method).

#### What the cap is worth

A nose with a blunt or vertical tip flies Newtonian pressures over the tip and hands the rest of
the body to the shock-expansion method where the surface's slope falls far enough, the *handover*
([Blunt tips](#blunt-tips) above). That handover is never steeper than a *cap*, 24°. This section
measures what moving the cap to 30° would buy and what it would cost. Nothing here changes what a
rocket flies: the cap is where it was, and no committed number moved.

The cap could move because the cone slopes the method reads now reach 30° ([ADR-042][adr-042]),
where they once stopped at 24°. Moving it would follow the report's own rule further: the wedge's
largest deflection passes 24° at Mach 2.06 and 30° at Mach 2.52, so a 30° cap keeps TN D-4865's
([J68]'s) rule over that whole band, where 24° cuts it short from Mach 2.06 up. Where a cap binds at
all, going all the way to 30° reads nearer the report's own sphere-cone at every row, though not at
every step of the way. And on the Arcas Robin's committed nose it breaks *the march* — the method
stepping element by element down the body from the handover — above Mach 4. hpr keeps 24° until that
is settled ([ADR-043][adr-043]), and the cap is a parameter of the method rather than a constant to
argue over
([`with_handover_cap_rad`](../api/hpr_aero/shock_expansion/struct.ShockExpansionBody.html#method.with_handover_cap_rad)).

Each cap starts to bind at its own speed — Mach 2.06, 2.19, 2.34 and 2.52 — and below that it
costs nothing at all. hpr's error against TN D-4865's sphere-cone, fitted as
[How it was checked](#blunt-tips) fits it (hpr's `C_N` at the tunnel's plotted 0° to 12°, body
lift included, fitted with a straight line), under four caps:

| Mach | error at 24°, as flown | at 26° | at 28° | at 30° |
|---|---|---|---|---|
| 1.5 | −1.2% | −1.2% | −1.2% | −1.2% |
| 1.9 | +0.0% | +0.0% | +0.0% | +0.0% |
| 2.3 | +7.5% | +7.3% | +7.1% | +7.1% |
| 2.96 | +12.5% | +12.2% | +11.9% | +11.5% |
| 3.95 | +29.7% | +28.9% | +28.3% | +28.0% |
| 4.63 | +32.1% | +31.2% | +30.9% | +31.3% |

Below Mach 2.06 no cap binds, which is why the first two rows are one reading four times; at Mach
2.3 only 24° and 26° bind, so the last two columns agree. Read the first two binding rows with
care for another reason: the cone slopes are tabulated from Mach 3 up and held at that row below
it ([Bodies faster than sound](#bodies-faster-than-sound)), so at Mach 2.3 and 2.96 a steeper
cap's gain is read off a slope that is not itself a function of Mach there. Where a cap does bind, the
whole step from 24° to 30° reads nearer the tunnel by 0.4 to 1.7 points, most at Mach 3.95. The
one place a steeper cap reads further out is the last step at Mach 4.63, where 28° reads +30.9%
and 30° +31.3%.

Now the cost, which two counts tell you about. A *reduced* element is one where the method's
exponential law would run the wrong way — the pressure behind the corner heading away from the
tangent cone's instead of toward it, `η < 0` in [the method's own terms](#bodies-faster-than-sound)
([SD56] p. 13) — so hpr holds the pressure along it instead,
[issue #81](https://github.com/nrdptel/hpr-sim/issues/81)'s open question. A *crossing* is the
rarer and worse thing: the marched surface pressure passing through its own tangent cone's, either
way, within one part of the body — where a nose meets a cylinder, a boattail or a flare the cone's
own pressure steps, which is not the same thing and is not counted. Both counts are per march, not
per element.
[What a crossing is, and what it costs](#what-a-crossing-is-and-what-it-costs) says how they
differ and how far either one can be trusted
([`tangent_cone_crossings`](../api/hpr_aero/shock_expansion/struct.ShockExpansionBody.html#method.tangent_cone_crossings)).
Here are two of the four caps on the Arcas Robin's committed power-series nose and the short
model's cylinder, nothing aft, `C_Nα` per radian on its cross-section at `α → 0`, read with the
flown 10 elements per curve and with 160:

| Mach | 24°, 10 elements | 24°, 160 | 30°, 10 elements | 30°, 160 |
|---|---|---|---|---|
| 1.5 | 2.531 | 2.532 | 2.531 | 2.532 |
| 1.8 | 2.697 | 2.702 | 2.697 | 2.702 |
| 2.3 | 2.873 | 2.881 | 2.851 | 2.862 |
| 2.96 | 3.021 | 3.029 | 2.943 | 2.955 |
| 3.5 | 3.071 | 3.079 | 2.953 | 2.964 |
| 3.96 | 3.073 | 3.080 (1 of 160 reduced) | 2.919 | 2.927 (1 of 160 reduced) |
| 4.63 | 3.030 | 3.034 (2 of 160 reduced) | 3.047 (5 of 10 reduced, 2 crossings) | 3.260 (109 of 160 reduced, 2 crossings) |
| 5 | 2.980 | 2.984 (2 of 160 reduced) | 3.400 (9 of 10 reduced, 1 crossing) | 3.454 (145 of 160 reduced, 1 crossing) |

Through Mach 3.96 cutting the nose into sixteen times as many elements moves the answer by under
0.01 per radian under the flown cap and under 0.013 under the 30° one: the answer is the model's,
not the mesh's. Above it the 30° cap's march crosses its tangent cone twice, and reduces most of
the nose along with it — 5 of 10 elements at Mach 4.63 and 9 of 10 at Mach 5, which is what hpr
would fly, and 109 and 145 of 160 — and the answer follows the element count instead, and not even
in order: 3.047 at 10 elements, 2.928 at the 40 the fixture also holds, and 3.260 at 160 — a spread
of 0.33 per radian, 11% — where the flown cap moves by 0.1%. The crossing is the cause and the
reductions travel with it, which the next section takes apart.

##### What a crossing is, and what it costs

Throughout this section, `p₂` is the surface pressure just behind a corner and `p_c` its tangent
cone's, `Λ` the lift per unit length (the *loading*) and `Λ_c` the tangent cone's, all as
[the method's equations](#bodies-faster-than-sound) define them.

The steeper cap starts the march from a steeper cone at a higher pressure, and from there the
tangent cone's own pressure falls away faster than the marched pressure does as the nose flattens.
So the surface pressure catches its tangent cone's and passes through it — at Mach 4.63 under the
30° cap, about a tenth of the way back — and stays above it until the nose flattens enough for the
cone to catch up again. That is two crossings: one out, one back.

Why that hurts has nothing to do with `η < 0`. Write the exponent in `e^(−η)` as `η = k (x − x₂)`,
so that `k = (∂p/∂s)₂ / ((p_c − p₂) cos δ₂)` is a relaxation rate per metre — the gradient just
behind the corner divided by how far the pressure has to go. A crossing closes that gap while the
gradient carries on, so `k` has a **pole**: it runs to infinity. The pressure itself doesn't mind,
because `k (p_c − p) cos δ₂` is only the gradient again, and that stays finite. The *loading* does
mind, because it relaxes toward `Λ_c` at the same `k` ([SD56] eq. 19) while its own gap is set by
something else entirely.

Follow that gap through the Mach 4.63 march under the 30° cap. Between the two crossings nearly
every element is reduced, so its loading is held where it was and never relaxes: by the second
crossing `Λ` stands about a quarter *above* `Λ_c`. The element at that second crossing is back
inside the method — and how much of that quarter it sheds in its own length is whatever
`1 − e^(−η)` happens to be for the step the mesh gave it. On the 40-element march it sheds 98% of
the gap in one step; on the 160-element march, 12%. That is the answer moving with the mesh, in one
number (test `a_crossing_is_a_pole_in_the_rate_the_march_relaxes_at`).

The counts say the same thing over the whole sweep. Of its thirty-two readings — four caps at eight
Mach numbers on this one nose — twenty-seven never cross and five do:

| | readings | most the answer moves over 10 → 160 elements |
|---|---|---|
| no crossing | 27 | 0.012 per radian |
| a crossing | 5 | at least 0.035, up to 0.69 |

No overlap, and nearly three times (2.9×) between the two groups. The five are 30° and 28° at Mach
4.63 and 5, and 26° at Mach 5 (fixture
[`blunt-tips.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/blunt-tips.json),
test `over_the_sweeps_meshes_a_crossing_separates_the_readings_that_move`).

**A crossing is a flag, not a verdict, and it has to be read carefully.** Three limits, all
measured:

- *A crossing does not prove an answer never settles.* 28° at Mach 5 crosses at every mesh, and
  its 0.69 spread is all in the coarse end: from 60 elements to 640 it holds to 0.005 per radian,
  tighter than the worst reading in the sweep that never crosses. Compare 30° at Mach 4.63, which
  moves by more than 0.2 per radian over that same range. What the sweep shows is that across its
  three meshes — the flown ten elements per curve and refinements to forty and a hundred and sixty
  — the crossings and only the crossings mark the readings that move (test
  `a_crossing_says_the_answer_moved_not_that_it_never_settles`).
- *A count of zero does not prove one settled.* Whether a crossing is seen depends on the mesh:
  26° at Mach 5 and 28° at Mach 4.63 show none at the flown ten elements per curve and two at
  forty and a hundred and sixty, and both move. Read zero as "not proven".
- *hpr does not count crossings while it flies,* and everything here is one nose. The count is a
  tool for studying a body, not a guard, and another blunt nose above Mach 4 could cross under the
  flown 24° cap without saying so.

Reduced elements, on their own, do not move an answer. TN 3527's own fineness-3 ogive reduces 27 of
160 elements at Mach 5.05 and 50 of 160 at Mach 6.28 — and its answer settles to 0.002 per radian
from 10 elements to 160. There `η < 0` comes from the *gradient* changing sign, with the surface
pressure below its tangent cone's the whole way down; the gap never closes, so there is no pole.
That ogive never crosses at either Mach number we can check it against the report at, which is why
the report could state its condition ([SD56] p. 13) and stop: it never had to say what a crossing
does (test `a_reduced_element_settles_where_tn3527s_own_bodies_never_cross`).

The two questions are tangled, though, and that is the state of play. The quarter-wide loading gap
the second crossing sheds was opened by the reduced stretch behind it — which is hpr's `η = 0`
reading, not the report's rule. So a different reading of `η < 0` would change the size of the step
as well, and neither question can be judged without the other. What has changed is that the step
itself is taken by an element the method still owns, so a rule for `η < 0` alone is not obviously
enough.

The disorder under the steeper cap is not rounding: nudge the Mach number by eight units in its
last place — about a part in 10^15 — and the same elements reduce, for an answer that follows to a
part in a billion (test `a_steeper_handover_moves_the_march_out_of_its_range`).

The break is not at 30°, and it is not orderly. It sits between the flown cap and the next step,
and 28° is the worst of the four — at Mach 5 its answer moves 0.69 per radian over the element
count, twelve times the 30° cap's 0.055. No cap above the flown one holds its answer to Mach 5:

| cap | Mach 4.63, 10 elements | 160 elements | Mach 5, 10 elements | 160 elements |
|---|---|---|---|---|
| 24°, as flown | 3.030 | 3.034 (2 of 160 reduced) | 2.980 | 2.984 (2 of 160 reduced) |
| 26° | 2.961 | 2.965 (2 of 160 reduced) | 2.900 | 2.946 (40 of 160 reduced, 2 crossings) |
| 28° | 2.892 | 2.926 (33 of 160 reduced, 2 crossings) | 2.923 (4 of 10 reduced, 2 crossings) | 3.612 (140 of 160 reduced, 1 crossing) |
| 30° | 3.047 (5 of 10 reduced, 2 crossings) | 3.260 (109 of 160 reduced, 2 crossings) | 3.400 (9 of 10 reduced, 1 crossing) | 3.454 (145 of 160 reduced, 1 crossing) |

Below Mach 4 the four agree to 0.013 per radian, so nothing here says a cap between the two ends
is a middle ground. It says the flown cap is the last one whose answer is the model's all the way
to Mach 5.

Settled is not the same as right. Under the flown cap hpr still reads +29.7% and +32.1% against
the sphere-cone at Mach 3.95 and 4.63, as the first table says. The cap chooses between an answer
that is high and one that is high *and* moves with the mesh.

So the cap hpr flies is set by the march's range rather than by a chart's edge. It moves when the
method has a *rule* for what the loading does where the surface pressure crosses its tangent
cone's — a rule whose answer stops changing as the nose is cut finer, judged together with the
reading of `η < 0` that sets the gap it sheds. TN 3527 does not state either, because its own
bodies never cross, so this is a modelling decision rather than a measurement to look up ([issue #108: a steeper handover crosses the tangent cone above Mach
4](https://github.com/nrdptel/hpr-sim/issues/108)). The milestone that would then move the
cap, [M1.8e16, the blunt tip's handover past 24°](../decisions-and-roadmap.md#m1-8e16), waits on
that ([ADR-044][adr-044], which records the measurement behind this section).
All three tables are held to
[`blunt-tips.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/blunt-tips.json)
by a test, cell by cell; the second shows the two ends of the sweep, and the fixture holds 26° and
28° and a 40-element reading too. Its numbers are stored to six decimals, three more than the tables
quote: where most of the nose is reduced the march is not reproducible past about 1e-11 from one
machine's maths library to another's, so pinning more would only break the build
([ADR-043][adr-043]).

#### The two starts

hpr starts the march from the tangent cone at the handover, the report from the Newtonian pressure
there. Read at a small angle, the report's start fails on
the Arcas Robin's nose from Mach 3.96, where the march *reduces* the element at the nose's end —
holding its pressure where the method's exponential law would run the wrong way — on the
**cylinder** behind it, whose tangent cone is the free stream rather than a cone of its own, and
which hpr therefore refuses ([issue #123: a cylinder's or a boattail's reduced
element](https://github.com/nrdptel/hpr-sim/issues/123)); from Mach 2.96 its answer drifts as the
nose is cut into more elements, for the same reason
([issue #81](https://github.com/nrdptel/hpr-sim/issues/81), the method's open question there). A flight's table is built from Mach 5
down, so that failure would leave such a rocket no method at all. The tangent cone's start holds
to Mach 5 and settles: the Arcas nose moves under 0.01 per radian from 10 elements to 40, a
five-calibre elliptical or von Kármán nose under 0.02 from 10 to 160. On the report's sphere-cone, against the
measured slope at `α → 0`, the report's start reads closer than hpr's at Mach 1.9, 3.95 and
4.63, about the same at 2.96 and further at 2.3, and 95% high at Mach 1.5. That last is hpr's
reading of the report's start at `α → 0`, not the report's method, which reads 1.844 there at
its own angles: the handover sits 0.6° above the cone, so its linear range is that small. hpr
takes the start that holds and settles everywhere over one that fits one body better where it
holds. Both are kept:
[`HandoverStart`](../api/hpr_aero/shock_expansion/enum.HandoverStart.html) selects the report's
for comparison ([ADR-038][adr-038]). Slopes per radian on the body's cross-section, at `α → 0`, for
the committed nose and the short model's cylinder, nothing aft:

| Mach | the tangent cone's start, 10 elements | 40 elements | the report's start, 10 elements | 40 elements |
|---|---|---|---|---|
| 1.5 | 2.531 | 2.532 | 2.866 | 2.770 |
| 1.8 | 2.697 | 2.701 | 2.676 | 2.635 |
| 2.3 | 2.873 | 2.880 | 2.641 | 2.635 |
| 2.96 | 3.021 | 3.028 | 2.544 | 2.583 (6 of 40 reduced) |
| 3.5 | 3.071 | 3.078 | 3.361 (9 of 10 reduced) | 3.418 (39 of 40 reduced) |
| 3.96 | 3.073 | 3.080 | fails | fails |
| 4.63 | 3.030 | 3.031 (1 of 40 reduced) | fails | fails |
| 5 | 2.980 | 2.982 (1 of 40 reduced) | fails | fails |

**What it means for a rocket.** Mostly more force, barely any change of balance. On Calisto,
whose von Kármán nose now takes the method past Mach 1.2, the whole rocket's normal-force slope
rises 17% to 31% from Mach 1.5 to 2
([Normal force through Mach 1](#normal-force-through-mach-1)), while its centre of pressure moves
by under 0.15 calibres (forward at Mach 1.5, aft at Mach 2). So the stability margin moves by under
a sixth of a calibre, and the force that holds the rocket into the wind grows by about a quarter.

**What it leaves out.**

- **No measurement checks a tip that isn't spherical.** The report tested spherical caps only.
  Newtonian theory on the cap and the tangent cone's start are both approximations, and the
  method's reduced elements ([issue #81](https://github.com/nrdptel/hpr-sim/issues/81)) still
  apply behind them.
- **A cap that shrinks to nothing doesn't reach the cone it sits on**
  ([issue #101](https://github.com/nrdptel/hpr-sim/issues/101)). The march carries its start
  cone's total pressure the whole way, as the method does from any vertex, and nothing makes that
  fade as the cap shrinks. A power-series nose of `n` = 0.99 is a 7.1° cone but for a tip 1e-55
  calibres across, yet at Mach 4 its cylinder carries 1.21 per radian where the cone's carries
  1.37, 12% less, because the march runs on the 24° cone's total pressure rather than the 7.1°
  cone's. The shapes a rocket really uses have caps that are small but not vanishing — at Mach 1.5
  the table above puts their ends at 0.12 to 0.76 of the base radius, where that nose's is 1e-55 —
  and how much of this bias they carry is unknown.
- **At `α → 0` the handover is held where it sits on the body**, as TN 3527 holds every other
  point. The report's equivalent bodies turn the body about the sphere's centre, which slides the
  handover along the surface instead; hpr leaves that term out. How much it is worth is not
  measured here. The two starts in the tables above differ by more than it alone, since their
  pressure and total pressure differ too: 2.53 against 2.87 per radian on the Arcas nose at
  Mach 1.5, and 1.70 against 1.70 on the sphere-cone at Mach 2.96.
- **Two switches in shape**, of the family [issue #87](https://github.com/nrdptel/hpr-sim/issues/87)
  tracks: a vertical-tip nose steeper than the cap's handover all the way to its base gets no
  method at all, and a pointed tip steeper than the cone tables' 30° is refused where a vertical
  one flies. The pointed tip's edge was Fig. 2's 24° until
  [M1.8e11](../decisions-and-roadmap.md#m1-8e11). The vertical tip's edge is the handover's cap,
  which stands at 24° for the reason [above](#what-the-cap-is-worth), so that switch waits on
  [issue #108: a steeper handover puts the march into `η < 0` above Mach
  4](https://github.com/nrdptel/hpr-sim/issues/108) too.
- **Elements that merge, merge with Mach.** Behind the cap, a tangency point whose tangent turns by
  under a microradian is folded into the element before it, because its corner can't be placed in
  floating point. Which points merge changes with the handover, so the method's answer takes a step
  of about a millionth of a per-radian slope as it does: far below anything measured here, but
  there.
- **Drag is unchanged:** the nose's wave drag already covers blunt shapes
  ([Drag through Mach 1](#drag-through-mach-1)). 
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
  - The roll moment of a single fin's normal force at an angle of attack: its force acts at
    `r_t + y_MAC` along the fin's normal. Two or more even fins cancel it; one fin doesn't. Roll
    from cant and against the roll rate is modelled
    ([Roll: forcing and damping](#roll-forcing-and-damping)).
  - Interference between fin sets at the same station.
  - Damping coefficients for pitch and yaw. They would replace the local-flow damping below, not
    add to it, or it would be counted twice; hpr keeps the local flow:
    - In a flight, pitch and yaw damping come only from evaluating each component in its own
      local flow, which includes the speed the rocket's rotation adds there
      ([Rigid-body flight](flight.md#aerodynamics-in-flight)).
    - Only components with a slope give it: nose cones, transitions and fin sets. A boattail's
      slope is negative, so it takes some away.
    - Body tubes give none at small angles. Their own slope is 0, and their body lift grows with
      `sin² α`, so it adds nothing there.
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

### Roll: forcing and damping

Fins set at a small angle to the rocket's axis, [cant](../glossary.md#cant), each push sideways as
a wing at that angle would. Each push acts off the axis, so together they twist the rocket and spin
it up: the [roll forcing](../glossary.md#roll-damping-and-roll-forcing). Once the rocket rolls,
each fin also moves sideways through the air, meets it at an angle of its own, and pushes back
against the spin: the roll damping. The two balance at a steady roll rate that grows with the
airspeed. hpr takes both from Barrowman's thesis ([B67] §3.13–3.14, appendix A), by strip theory:
each narrow strip of a fin, running with the flow, lifts in proportion to the angle it meets the
air at.

How far to trust it
([Roll against the Arcas Robin and the Basic Finner](#roll-against-the-arcas-robin-and-the-basic-finner)):

- The forcing is within 5.3% of NASA's measured roll effectiveness (the rolling moment per degree
  of cant) from Mach 2.3 to 4.63, and reads 14% to 48% high at Mach 1.5 and 1.8.
- The damping reads 6% to 16% low against the one measured set, from Mach 1.5 to 3.
- Below Mach 1.5, where most hobby flights stay, nothing measured checks either: only the flight's
  agreement with the closed-form balance, and Barrowman's own computed damping at Mach 0.07.
  Strips that each lift at the fin's average slope ignore how the flow at one strip changes the
  next, which for short fins likely overstates the damping, so a subsonic spin may read low.
- The steady spin rate carries both errors, and both push it high: forcing that reads high and
  damping that reads low each raise it.

Other symbols are as in [Fins](#fins): `r_t` the body's radius at the fins, `s` the span, `c_r`
and `c_t` the root and tip chords, `A_fin` one fin's area, `y_MAC` the mean aerodynamic chord's
distance from the root.

| Symbol | Meaning | Unit |
|---|---|---|
| `δ` | cant: the angle each fin is turned about its own span, positive turning fin 0's (the fin along `+x_B`) leading edge toward `−y_B` ([Mass properties](mass.md)) | rad |
| `p` | roll rate about `+z_B` ([Frames](frames.md)) | rad/s |
| `ξ` | a strip's distance from the rocket's axis, `r_t + y` | m |
| `C_l` | rolling moment about `+z_B` over `q A_ref d`, with `d` the reference diameter | — |
| `C_lδ` | one fin's rolling moment per radian of cant, in the sense its lift turns the rocket | per rad |
| `C_lp` | one fin's rolling moment per unit of `p d/(2V)`: the damping, negative | — |
| `C_l0` | the whole rocket's rolling moment from cant at no roll rate | — |
| `k_T(B)`, `k_R(B)` | the body's effect on the forcing and on the damping | — |

**The moment.** A fin set of `N` fins adds `C_l = −N C_lδ k_T(B) δ + N C_lp k_R(B) (p d/2V)`. The
minus sign is the cant's direction: a positive cant turns fin 0's leading edge toward `−y_B`, so
its lift pushes toward `−y_B` and turns the rocket about `−z_B` (a negative `C_l`), clockwise
seen from ahead of the nose, looking aft. Its first term, summed over the sets, is `C_l0`. In a
flight the cant's forcing is scaled by `cos α`: the cant meets the air as it runs along the axis,
so there is none broadside and it reverses tail first, as the fins' normal force follows `sin α`
([Rigid-body flight](flight.md#aerodynamics-in-flight)).
Bodies of revolution add nothing, and fin–fin interference is left out, as in [N09] eq. 3.66.

**Below Mach 0.8.** The cant is an angle of attack for each fin, so its lift is the fin's own
normal force, acting at its mean aerodynamic chord: `C_lδ = (C_Nα)₁ (r_t + y_MAC)/d` ([B67] eq.
3-35, [N09] eq. 3.66). For the damping, a strip at `ξ` meets the air at `−pξ/V`, and lifts by the
fin's slope per unit of its area, `a = (C_Nα)₁ A_ref/A_fin`: `C_lp = −2a ∫ξ² dA/(A_ref d²)` ([B67]
eq. 3-40–3-49, [N09] eq. 3.67–3.70). For a trapezoid `∫ξ² dA = (c_r + c_t) r_t² s/2 +
(c_r + 2c_t) r_t s²/3 + (c_r + 3c_t) s³/12`; for any outline hpr takes it from the polygon.

**From `M_s`,** where [supersonic linear theory](../glossary.md#supersonic-linear-theory) starts
([Fins through Mach 1](#fins-through-mach-1)), each strip carries the load `4α/β`, halved inside
the tip's [Mach cone](../glossary.md#mach-cone):
`C_lδ = (4/β)(∫ξ dA − ½∫_cone ξ dA)/(A_ref d)` and
`C_lp = −(8/β)(∫ξ² dA − ½∫_cone ξ² dA)/(A_ref d²)`, with `β = √(M² − 1)` ([B67] appendix A, first
order). Between Mach 0.8 and `M_s` each is a straight line in `M`, as the fin's slope is.

**The body.** The body reshapes the flow the fins meet. Barrowman's factors from slender-body
theory ([B67] eq. 3-95 with 3-105, and 3-123 with 3-122), with `τ = (s + r_t)/r_t`, scale the
forcing by `k_T(B)`, 0.940 at `τ = 2`, and the damping by `k_R(B)`, 1.33 at `τ = 2` for a
rectangular fin; both are 1 without a body. [N09] leaves both out. `k_R(B)` is for a chord that
falls linearly from root to tip; another outline takes it at its tip-to-root chord ratio, so an
elliptical fin is taken as a triangle, about 5.5% too much damping.

**The steady roll rate** is where the two cancel: `p = −(C_l0/C_lp)(2V/d)`. Below Mach 0.8 the
fin's slope cancels between them, and for one fin set
`p = −δ V A_fin (r_t + y_MAC) k_T(B) / (k_R(B) ∫ξ² dA)`: it grows with the airspeed and the cant,
and not with the air's density or the number of fins.

**A worked example: Valetudo with its fins canted 1°.** Valetudo, one of RocketPy's example
rockets, has three fins 58 mm long at the root, 18 mm at the tip and 77 mm in span on a body of
radius 40.45 mm. One fin has `A_fin = 2926 mm²`, `y_MAC = 31.75 mm`, `∫ξ² dA = 1.656 × 10⁻⁵ m⁴`,
and `τ = 2.904`, so `k_T(B) = 0.935` and `k_R(B) = 1.228`. At 100 m/s,
`p = −0.01745 × 100 × 0.002926 × 0.0722 × 0.935/(1.228 × 1.656 × 10⁻⁵) = −16.95 rad/s`, 2.7
turns a second. With no drag and no gravity hpr's flight settles on it within 1e-6 (1e-11
measured), spinning up with a time constant of 0.48 s
(`canted_fins_spin_to_the_analytic_balance`, which also checks this example's rate and time
constant).

Why these choices:

- *The fin's own slope in the damping.* Barrowman's text writes the airfoil's slope `C_Nα0` there
  (eq. 3-40, `2π/β`), as does [N09] eq. 3.69. His own computed curve for the Basic Finner reads
  −34.21 at Mach 0.07 (Fig. 5-7), which is the fin's slope spread over its strips, −33.53
  (`the_basic_finner_damps_as_barrowman_computed`); the airfoil's gives about −81, 2.4 times as
  hard. His curve's rise toward Mach 1, about 20% read from the figure, follows the fin's slope
  too, where the airfoil's would grow without bound. Stubby fins lift far less than an airfoil.
- *The body factors.* Barrowman has them and [N09] doesn't. For the Arcas Robin's fins they lower
  the forcing 6.5% and raise the damping 20%. His `k_R(B)` is a ratio of forces (eq. 3-116,
  3-120) applied to a moment (eq. 3-123); weighted by the moment it would be 3.4% to 4.2% smaller
  for the fins here. hpr keeps his, which his computed curve seems to use too. `k_T(B)` is
  reference 23's factor for fins turned together; for cant, whose load turns the other way on the
  opposite fin, it isn't derived.
- *Moments about the body's axis.* Faster than sound Barrowman's appendix A takes each strip's
  moment about the fin's root; hpr takes it about the axis, `ξ = r_t + y`, as his subsonic eq.
  3-27 and 3-35 do. About the root the Arcas Robin's forcing would be about half: its load sits
  25 mm from the root and 53 mm from the axis.
- *Limits on the input.* hpr refuses a cant beyond 15°, where a fin stalls and the linear model
  means nothing, and a cant on a single fin, whose sideways push it doesn't carry.
- *Pitch and yaw keep the local-flow damping.* A flight's pitch and yaw damping come from each
  part's own local flow ([ADR-011][adr-011]); coefficients would count it twice.
- *No target was set for the comparisons.* The roadmap asked for the forcing to be compared with
  the measured roll effectiveness, and set no bar; the numbers are reported as they are
  ([ADR-031][adr-031]).

What it leaves out:

- The roll forcing near Mach 1.5 reads high: linear theory's load rises as `1/β` toward Mach 1,
  and the Arcas Robin's measured forcing doesn't. Barrowman found the same for the Tomahawk
  sounding rocket ("the theoretical value at M = 1.5 is no good", [B67] p. 66).
- The damping reads low for the Basic Finner's thick wedge fins, 8% of the diameter thick:
  first-order theory has no term for thickness, the likely cause.
- Nothing measured checks roll below Mach 1.5.
- A fast spin at low airspeed meets the fins at angles past stall, where the linear damping no
  longer holds: Valetudo spinning at 17 rad/s at 5 m/s meets the air 23° off at its fin tips.
- The angle of attack: the measured roll effectiveness changes by up to 13% between 0° and ±4°
  (TN D-4014 Fig. 14); hpr's is the same at every angle. A single fin's roll from its normal
  force, the body's own roll, and fins' airfoil sections are not modelled.

## The normal force from RASAero II

A flight can use another program's normal force and centre of pressure in place of hpr's own.
Today that program is [RASAero II](../glossary.md#rasaero-ii), read from the table it exports.
Use it to fly two programs on the same aerodynamics, so that a difference between them comes from
something else. Or use it to fly RASAero II's numbers faster than sound, where hpr's own normal
force is less tested ([Fins through Mach 1](#fins-through-mach-1)).

How far to trust it:

- **The reading matches the file.** On the export for
  [Calisto](../glossary.md#example-rockets), every one of its 4,999 rows at 2° and 4° comes back
  from hpr's table to 2e-16. The 0° column, which hpr works out, agrees at 15 Mach numbers with
  the reading made for [M1.8a](../decisions-and-roadmap.md#m1-8a).
- **The flight uses the table as the equations say it should.** A rocket flying on a table swings
  in pitch and yaw as the small-angle equations of motion predict for the table's slope and
  centre of pressure.
- **Only up to Mach 0.75 on real data.** Only one real export has been flown, Calisto's.
  Faster than that, the table is checked by unit tests alone.
- **Past the export's last angle, and at 0° faster than Mach 1.3, hpr assumes.** The assumptions
  fit RASAero II's viscous part through Mach 1.3. Faster, that part grows much more slowly with
  the angle, so past 4° the table probably gives too much force at Mach 3 and above.

Three parts are hpr's choices, not RASAero II's:

- the damping, which stays hpr's own;
- the normal force past the export's largest angle of attack (4° in the one export tested);
- the slope at 0°, where the export's normal force is zero.

In code, two calls take a file to a flight:

1. [`NormalForceTable::from_rasaero_csv`](../api/hpr_aero/table/struct.NormalForceTable.html#method.from_rasaero_csv)
   reads the text.
2. [`Simulation::with_normal_force_table`](../api/hpr_sim/flight/struct.Simulation.html#method.with_normal_force_table)
   flies it. Its documentation has a worked program.

The decisions are in the record on normal-force overrides, [ADR-032][adr-032]. The milestone is
[M1.8d](../decisions-and-roadmap.md#m1-8d).

### What the export holds

RASAero II's Aero Plots screen exports a table to CSV (File, Export, To CSV File; [RAS] p. 76).
There is one row for each Mach number and [angle of attack](../glossary.md#angle-of-attack)
(`Alpha`, in degrees). The Calisto export has rows at 0°, 2° and 4°. hpr reads five of the
columns:

| column | what it is |
|---|---|
| `Mach`, `Alpha` | the Mach number, and the angle of attack in degrees |
| `CN` | the normal-force coefficient at that angle |
| `CN Potential` | the part of `CN` from potential flow (the air treated as smooth and without friction), which grows in step with the angle |
| `CP` | the centre of pressure, in inches ([RAS] p. 13) measured from the nose tip (p. 114) |

`CN` also holds a viscous part, `CN Viscous`. It is the extra push, from the air's friction, of
the air flowing sideways across the body. RASAero II takes it from Jorgensen's method ([RAS]
p. 55), adds it from Mach 0.91 in Calisto's export, and moves the centre of pressure forward with
the angle. hpr's own model has neither. From Mach 0.91 through Mach 1.3 the viscous part grows
exactly as `sin² α`: at 4° it is (sin 4°/sin 2°)² = 3.995 times its value at 2°. Faster, it grows
more slowly: 3.90 times at Mach 1.5, 3.16 at Mach 2, 1.73 at Mach 3 and 1.05 at Mach 4. The export's `CNalpha (0 to 4 deg)` and `CP (0 to 4 deg)` columns
repeat its 4° values on every row; hpr doesn't read them.

### How hpr reads it

hpr builds one column for each angle in the export. Each column holds `C_N/α` (the normal force
over the angle, per radian) and the centre of pressure, both against Mach number.

- **At a positive angle**, `C_N/α` is `CN` over the angle in radians.
- **At 0°**, `CN` is zero, so it can't be divided. hpr takes `CN Potential` at the smallest
  positive angle, over that angle. The potential part grows in step with the angle: in Calisto's
  export its `C_N/α` is the same at 2° and 4° to 2e-15. Through Mach 1.3 the viscous part grows as
  `sin² α`, so it adds no slope at 0°. Faster, the export doesn't show how it starts from 0°, and
  leaving it out of the slope is an assumption.
- **The centre of pressure** is converted from inches to metres at 0.0254 m to the inch. It
  stays measured from the nose tip, as hpr's stations are
  ([station](../glossary.md#station)). So the design must start at the same nose tip as the
  RASAero II file. A table whose centre of pressure, at one of its Mach numbers up to 5 (where a
  flight stops), lies outside the rocket, ahead of its nose or behind its tail, is refused: it is
  the sign of a length in the wrong unit.
- **Reference area.** RASAero II's coefficients are on the body's largest cross-section ([RAS]
  p. 72). hpr records that and rescales them to the rocket's own
  [reference area](../glossary.md#reference-area), when that is something else. For a rocket of
  several stages, export the whole stack ("Sustainer plus Booster" or "All Stages"): hpr flies
  the whole stack.

A flight looks up the table at its Mach number and angle of attack:

- Within one column, the values are linear in Mach number. Outside a column's range the end
  values hold, and the lookup says so.
- Between two columns, `C_N/α` and the centre of pressure are linear in the angle. Then
  `C_N = (C_N/α)·α` comes back exactly at each column's angle. Between them it is a part in step
  with the angle plus one in its square: RASAero II's own shape through Mach 1.3 (`α²` is within
  0.2% of `sin² α` to 4°), and an assumption faster than that.
- **Past the largest angle** `α_n`, the normal force splits in two. The linear share is the
  slope at 0° times `α_n`, at the 0° centre of pressure; it grows as `sin α`, as hpr's own fins do
  ([Aerodynamics in flight](flight.md#aerodynamics-in-flight)). The rest of the force, with the
  rest of the moment, grows as `sin² α`, the form of the air crossing the body that hpr's
  [body lift](#bodies-of-revolution) also takes ([G] p. 1; [N09] eq. 3.26). The force and centre
  of pressure are continuous at `α_n`, and the force is zero when the air comes from the tail.
  Two limits keep this sensible for any table. The rest's centre of pressure is held within the
  rocket. And a table whose `C_N/α` falls with the angle has no rest: its whole force grows as
  `sin α`, so the force never turns round. Neither limit makes a jump as the Mach number changes.
  Either way this is an assumption, and the lookup reports it.

**A worked example.** Take an export with invented numbers. At Mach 1, `CN Potential` is 10 per
radian times the angle, `CN Viscous` is 0.03 at 2° and 0.12 at 4°, and the centre of pressure is
50, 49 and 48 inches at 0°, 2° and 4°. These are the numbers in the CSV reader's unit test,
`reads_a_rasaero_export_by_angle_of_attack`.

| angle | `CN` | `C_N/α`, per rad | centre of pressure |
|---|---|---|---|
| 0° | 0 | 10.000 (`CN Potential` at 2° over 2°) | 1.2700 m |
| 2° | 0.37907 | 10.8594 | 1.2446 m |
| 3° (looked up) | 0.59110 | 11.2892, halfway between 2° and 4° | 1.2319 m |
| 4° | 0.81813 | 11.7189 | 1.2192 m |
| 10° (past the table) | 2.4815 | 14.2180 | 1.1662 m |

At 10°, `sin 10°/sin 4°` is 2.4893. The linear share, 10 × 0.069813 = 0.69813 at 1.2700 m,
grows to 1.7379. The rest, 0.12 at 0.9237 m (the station that gives the 4° moment), grows by
2.4893² to 0.7436. Together they make 2.4815 at 1.1662 m: the centre of pressure moves forward
with the angle, as RASAero II's does. The first draft scaled the whole 0.81813 by 2.4893 at
1.2192 m instead: 2.0366, 18% less force, with the centre of pressure 5.3 cm further aft. That
reads the rocket as more stable than the viscous part makes it.

### In a flight

The flight takes the table's normal force at the centre of mass's airflow and applies it at the
table's centre of pressure.

The export has no damping, so hpr keeps its own ([Rigid-body flight](flight.md)). The table gives
the force as if the rocket weren't turning. When it turns, each part of the rocket meets the air
at a slightly different angle, and hpr adds that difference: it is the damping. When the rocket
isn't turning, the difference is exactly zero.

The flight still stops at Mach 5, where hpr's own parts, which give the damping, end. The table
gives no side force: RASAero II's rockets are symmetric.

### How it was checked

| check | result | where the numbers are |
|---|---|---|
| Calisto's export, every row at 2° and 4° read again apart from the library | 4,999 rows; `CN` within 2.2e-16 relative, `CP` exact; columns at 0°, 2° and 4° of 2,500, 2,500 and 2,499 Mach numbers, from Mach 0.01 to 25 (24.99 at 4°) | [`normal-force-override.json`][override-fixture] |
| The 0° column at 15 Mach numbers against the reading of the export made for [M1.8a](../decisions-and-roadmap.md#m1-8a), the normal force through Mach 1, which [`normal-force-vs-mach.json`][mach-fixture] holds | the same to 1e-12 relative. That reading applies the same 0° rule, so this checks the reading, not the rule | both files |
| [Valetudo](../glossary.md#example-rockets) at 100 m/s on a table of 1.5 times hpr's slope with the centre of pressure 5 cm further aft, against the small-angle equations of motion, in pitch and in yaw | period 1.104077 s against 1.104073 s, within the test's 3e-5 (1.44965 s on hpr's own); the decay within 0.03%, the test's bound 1% | `hpr_sim::tests::pitch_oscillation_follows_a_normal_force_table` |
| Tables of hpr's own normal force flown in a crosswind: every 0.5° and every Mach 0.01, and at 0°, 2° and 4° only, where the flight uses the continuation past 4° | apogee within 7.8 mm and 5.5 cm of hpr's own flight, the test's bounds 5 cm and 10 cm | `hpr_sim::tests::a_table_of_hpr_s_own_normal_force_flies_as_hpr_does` |
| The continuation past the last angle: a table shaped as RASAero II's (a part linear in the angle, one as `sin² α`), and random tables | the `sin² α` part continues to 1e-12; the force never turns round, the centre of pressure stays within the rocket, and nothing jumps as the table's values change with Mach number | `hpr_aero::table::tests` |
| Calisto from a 5.2 m rail at 85° in a 5 m/s crosswind, up to Mach 0.746: on the export, on hpr's own normal force, and on hpr's own as a table at the export's angles | the export: apogee 2,793.09 m against 2,794.21 m, 12.4 m further into the wind. hpr's own as a table moves it 0.02 m: the table's method, apart from its numbers. Each flight spends 2.1 to 2.2 s past 4° before apogee | [`normal-force-override.json`][override-fixture] |

The Calisto flights show how much the change matters. They are not a check of accuracy: nothing
measured flew. `cargo xtask aero` writes the fixture from the export, which isn't committed, and
checks it again wherever the export is present. CI has no copy of the export: there, a test checks
that the fixture's 0° values agree with the ones committed for
[M1.8a](../decisions-and-roadmap.md#m1-8a), and flies the 15-point table again.

[override-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/normal-force-override.json
[mach-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/normal-force-vs-mach.json

What it leaves out:

- **No real export has been flown through Mach 1.** Calisto peaks at Mach 0.75. The reader's unit
  tests cover the transonic columns.
- **Past the export's largest angle**, the split continuation is hpr's assumption. In a 5 m/s
  crosswind Calisto flies past 4° for about 0.3 s just after leaving the rail (up to 7.9°) and for
  the last 1.9 s before apogee, as it slows below 30 m/s. Its export has no viscous part below
  Mach 0.91, so Calisto's flights use only the linear share; the tables of hpr's own normal force
  are the flights that grow a rest as `sin² α`. From Mach 3, where RASAero II's viscous part
  hardly grows between 2° and 4°, the `sin² α` share probably gives too much force.
- **The nose tip** can't be checked from the export beyond the refusal above. A design that
  starts somewhere else gets a shifted centre of pressure, with no warning.
- **Only RASAero II's layout is read.** A table from anywhere else can be built in code with
  [`NormalForceTable::new`](../api/hpr_aero/table/struct.NormalForceTable.html#method.new).

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
  Mach number. `AeroModel::buildup_components` always reports the buildup, table or not. The
  normal force has a table of its own
  ([The normal force from RASAero II](#the-normal-force-from-rasaero-ii)).

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

- **A flared body's supersonic normal force rests on one measured flare.** Since
  [M1.8e17](../decisions-and-roadmap.md#m1-8e17) a conical flare flies the shock-expansion method
  while its corner's shock is attached, which moves a flared rocket's centre of pressure forward
  by a tenth to a quarter of a calibre against the model it had before. The one measurement beside
  it is TN D-4865's model 2, an 18.5° flare on a 2.75° cone: −1.9% and +7.0% at Mach 1.9 and 2.3,
  +13.4% at 2.96, and +51.5% and +50.4% at 3.95 and 4.63, where that flare's boundary layer is
  separated ([What a marched flare is worth](#what-a-marched-flare-is-worth)). No other flare
  angle, and no other body, has been checked. Two more things about that model are open: the
  attachment test is a wedge's limit, not a flare's own
  ([A flare through the method](#a-flare-through-the-method)), and a near-flat flare has its one
  element read by the older generalized method, which leaves a step at the corner's crossing of at
  most +0.129% of the normal force and 0.0051 calibres on the tests' rocket
  ([A near-flat flare](#a-near-flat-flare)).
- **A step in radius is unmodelled, and nothing measures one faster than sound.** Any mismatch of
  radius at a joint — past 2.7e−11 m between two tubes, or past 1.3e−13 m stepping up at a
  boattail's fore end, both far below any tolerance anyone builds to — takes the **whole** body off
  the shock-expansion method at every speed. On the tests' straight rocket that is worth −8.65% of
  the normal force and 1.03 calibres of centre of pressure at the threshold, and −12.55% and 1.36
  calibres at a 2 mm step down; on the boattailed one, −11.34% and 1.10 calibres
  ([A step in radius](#a-step-in-radius)). No published source gives a stepped body's normal force
  faster than sound, so the present behaviour is not known to be right either; stopping the march at
  the step was built and rejected ([ADR-049][adr-049],
  [issue #87: a step has no model of its own](https://github.com/nrdptel/hpr-sim/issues/87)).
- **The body alone misses the 15% target on six of eleven wind-tunnel rows**, the one
  [M1.8e set for the body faster than sound](../decisions-and-roadmap.md#m1-8e), by +37.7% at
  worst (the short Arcas Robin at Mach 1.5) and within 5% at Mach 3.96 and 4.63. The likeliest
  cause is Jorgensen's crossflow term reading too large at the few degrees a slope is fitted over,
  but the measurement cannot split its own slope from its curvature cleanly, and one row points at
  the method instead. Closing it needs a cited rule for how the crossflow term grows from zero
  over the first few degrees, or measurements at finer angles than the reports plot; neither is in
  hand, so the gap is left visible
  ([The body alone, against the 15% target](#the-body-alone-against-the-15-target)).
- **A centre of pressure means little where the body's normal force is near zero.** A deep,
  steep transition can remove almost all the lift the nose and tube carry, and hpr still divides
  the moment by what is left: one test shape reports its body's centre of pressure 160 calibres
  ahead of its own nose tip at Mach 1.2
  ([issue #104: a near-zero normal force gives a meaningless centre of
  pressure](https://github.com/nrdptel/hpr-sim/issues/104)).
- **A blunt tip's handover is capped where the method's march still settles, not where its
  theory runs out.** The cap is 24° and the cone slopes reach 30°; at 30° the committed Arcas
  Robin nose reads nearer the report's own sphere-cone but its answer starts to follow the element
  count above Mach 4, so the cap stays. What that is worth is measured on both sides in
  [What the cap is worth](#what-the-cap-is-worth), and the way out is
  [issue #108: a steeper handover puts the march into `η < 0` above Mach
  4](https://github.com/nrdptel/hpr-sim/issues/108).
- **A boattail steeper than 16° is worth 0.67 to 1.35 calibres of doubt**, the most at the lowest
  supersonic speeds. Nothing measures a separated boattail's supersonic normal force; hpr holds
  the measured correlation at 16° rather than letting it fade, which is the conservative end
  ([A steep boattail reads the correlation no steeper than 16°](#the-body-faster-than-sound-in-a-flight)).
- These are small-angle models. `α` is accepted over `[0, π]`, but fin slopes stay linear in `α`
  and nothing models stall. The flight engine uses them at every angle all the same
  ([Rigid-body flight](flight.md)), so its results are least trustworthy where large angles occur:
  off the rail in a strong crosswind, and near apogee.
- **Body lift in wind** (measured by flying both codes, not against a real flight;
  [ADR-026][adr-026], [ADR-037][adr-037]). A rocket that leaves the rail slowly in
  a crosswind meets the air at a steep angle. Juno III, one of RocketPy's example rockets, leaves
  at 18 m/s in an 8.5 m/s wind, 26° off the airflow, and there body lift is nearly half its normal
  force. Much of it acts ahead of the rocket's centre of mass, the nose's above all, so it moves
  the centre of pressure forward and weakens the moment that
  [turns the rocket into the wind](../glossary.md#weathercocking); hpr turns into it less than
  RocketPy, whose normal force has no body term. Its sideways push alone is about a sixth of
  body lift's effect on the drift. Juno III's apogee ends 245.3 m from the pad in hpr and 396.6 m in
  RocketPy; body lift is about half of that difference, and hpr's rail release and fin slope most
  of the rest. Body lift's size matters there. Flown in RocketPy with hpr's model, Juno III's
  apogee drift is 248.3 m with Jorgensen's crossflow ([Body lift](#body-lift), `η C_dn` about 0.91
  at that speed), and with a constant `K` across [G]'s range from 194.1 m at `K = 1.5` to
  240.2 m at 1.0 (231.1 m at 1.1, hpr's before [M1.8e6](../decisions-and-roadmap.md#m1-8e6)); it
  would be 328.0 m with no body lift. Calisto, off the rail at 28 m/s and 11°, changes its drift by
  under 0.5% across that range. Which is nearer a real flight is open until
  [M2.3](../decisions-and-roadmap.md#m2-3).
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
calibres (a calibre is one reference diameter). 13 of the 37 rows miss, each for a measured
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
the same way. The design's lip sits in the boattail's wake and carries nothing faster than sound
([A lip in a boattail's wake](#a-lip-in-a-boattails-wake)), and its vertical tip flies behind a
Newtonian cap ([Blunt tips](#blunt-tips)), so the body flies the method to its base from Mach 1.2:
fins off it reads 3.02 to 3.95 per radian from Mach 1.5, where it read 1.90 to 2.09 on slender-body
theory. Below the join its [body lift](#body-lift) grows as `sin² α` and with the crossflow Mach number, so
their fitted slope moves a little with Mach and with the angles each plot happens to cover (1.90
to 2.09). From Mach 0.6 to 1.2 the fins-off readings, on a coarse grid (±0.02 per point), scatter
from 1.41 to 2.88 with no trend, so they don't settle whether hpr's 1.91 is high there.

| Mach | `C_Nα` measured, per rad | hpr | difference | CP measured, m | hpr | difference, calibres | body alone, measured / hpr |
|---|---|---|---|---|---|---|---|
| 0.6 | 11.05 | 10.54 | −4.6% | 0.7770 | 0.7837 | +0.12 | 1.53 / 1.91 |
| **0.8** | 9.92 | 10.85 | +9.5% | 0.7375 | 0.7896 | +0.91 | 1.41 / 1.90 |
| **0.9** | 10.22 | 12.77 | +25.0% | 0.7447 | 0.8215 | +1.34 | 2.44 / 1.91 |
| **0.95** | 11.88 | 13.74 | +15.6% | 0.7952 | 0.8342 | +0.68 | 2.88 / 1.92 |
| 1 | 15.79 | 14.69 | −6.9% | 0.8690 | 0.8455 | −0.41 | 1.58 / 1.92 |
| **1.2** | 15.42 | 18.54 | +20.2% | 0.8862 | 0.8798 | −0.11 | 2.43 / 1.92 |
| 1.5 | 13.43 | 14.61 | +8.8% | 0.8137 | 0.8282 | +0.25 | 2.19 / 3.02 |
| 1.8 | 11.99 | 12.46 | +3.9% | 0.7908 | 0.7883 | −0.04 | 2.61 / 3.29 |
| 2.3 | 9.89 | 10.50 | +6.2% | 0.7505 | 0.7369 | −0.24 | 3.08 / 3.60 |
| 2.96 | 8.77 | 9.10 | +3.7% | 0.6967 | 0.6858 | −0.19 | 3.28 / 3.84 |
| 3.96 | 7.73 | 7.85 | +1.6% | 0.6312 | 0.6330 | +0.03 | 3.88 / 3.95 |
| 4.63 | 7.55 | 7.30 | −3.3% | 0.5876 | 0.6073 | +0.34 | 4.15 / 3.95 |

| reference, Mach | `C_Nα` difference | CP difference, calibres | rows within both targets |
|---|---|---|---|
| Arcas, long, 0.6 and 0.8 | +0.7%, +8.9% | −0.36, +0.07 | 2 of 2 |
| Arcas, long, 0.9 to 1.2 | −8.8% to +27.1% | +0.67 to +2.36 | 0 of 3 |
| Arcas, long, 1.8 to 2.96 | +8.4% to +9.4% | −0.53 to −0.43 | 1 of 3 |
| Arcas, long, 3.96 and 4.63 | +2.8%, −2.3% | −0.13, +0.21 | 2 of 2 |
| Calisto against RASAero II, 0.1 to 0.7 | +0.1% to +10.1% | −0.08 to +0.43 | 4 of 4 |
| Calisto against RASAero II, 0.8 to 2.0 | −6.3% to +21.9% | −0.44 to +0.95 | 7 of 11 |

What the misses come from:

- **Faster than sound, every slope now passes, and the long model's CP at Mach 1.8 and 2.3 does
  not.** Until [M1.8e7](../decisions-and-roadmap.md#m1-8e7) and
  [M1.8e8](../decisions-and-roadmap.md#m1-8e8) the committed designs' vertical tip and lip kept the
  shock-expansion method off, so their bodies flew slender-body theory past Mach 1 and lifted 2.0
  to 2.6 per rad where the tunnel's body alone lifts 2.2 to 4.6: the short model read −16.3% at
  Mach 2.96 and −28.0% at 4.63. With the cap ([Blunt tips](#blunt-tips)) and the lip carrying
  nothing in the boattail's wake ([A lip in a boattail's wake](#a-lip-in-a-boattails-wake)), the
  body grows with Mach, as the measurement does though not as steeply (3.02 to 3.95 per rad fins
  off on the short model, against the tunnel's 2.19 to 4.15). The whole rocket's rows from Mach
  1.5 read +8.8% to −3.3% (short) and +9.4% to −2.3% (long). What is left is where the body reads
  *high*: fins off it
  is 15% to 19% above the tunnel at Mach 1.8 and 2.3 on the long model, which pulls the whole
  rocket's CP 0.53 and 0.52 calibres forward of the measured one, just outside the half-calibre
  target. [M1.8e6](../decisions-and-roadmap.md#m1-8e6) sized that excess and left it
  ([ADR-037][adr-037]); the body alone is judged against the 15% target in
  [The body alone, against the 15% target](#the-body-alone-against-the-15-target), where it is
  outside on six of eleven rows. The fins' share (the fins-on reading less the fins-off one) agrees with hpr's
  fins within −1.4% to +7.0% at Mach 3.96 and 4.63, with about 5% of doubt of its own: over the
  boattail the models' fin roots follow its 15° surface below the cylinder, and the design leaves
  that strip out, about 0.32 in² of each fin's 5.8 in² (5.5%).
- **Mach 0.6, within the targets by errors that cancel.** Both models pass there, but hpr's body
  is 25% and 19% above the fins-off readings, which are poorly determined at these speeds, and its
  fins' share 9.3% and 3.9% below the measured one.
- **Transonic, Mach 0.8 to 1.2.** The fins' measured share lifts less at Mach 0.8 and 0.9 than at
  0.6, then jumps at Mach 1. hpr's fins lift more, by Prandtl–Glauert and then along
  the join to linear theory's peak at `M_s` (1.2 for these fins). The long model's CP jumps
  forward at Mach 1, 2.29 calibres from hpr's. No closed-form method covers this region, and the
  join is not fitted to it.
- **RASAero II** keeps its slope and CP constant through subsonic flow, where hpr's rise with
  Prandtl–Glauert, so they part from Mach 0.8. Past Mach 1.2 Calisto's von Kármán nose flies the
  shock-expansion method behind a [Newtonian cap](#blunt-tips), so its cylinder carries lift:
  Mach 1.5 reads +12.9% and Mach 2 +8.3% (−3.1% and −16.8% on slender-body theory, before
  [M1.8e7](../decisions-and-roadmap.md#m1-8e7)). The wind tunnel sides with neither there. Below
  that the agreement is partly by construction: the Calisto design has the 2018 fins because
  they reproduce this export at low speed ([ADR-009][adr-009]). Past Mach 1 the result rests on
  the choice of RASAero's columns: against its secant slope and CP to 4°, which include its
  crossflow lift, 3 of the 11 rows from Mach 0.8 are within the targets (the tightest, Mach 1 by
  0.00002 calibres), not 7, and Mach 2 is −9.6%. That comparison is a summary in the fixture's
  `secant_comparison`, not a second set of rows: the export stays in `refs/`, and the fixture
  commits its values once ([ADR-009][adr-009], [ADR-027][adr-027]). Calisto has no fins-off data, so its body and fins
  can't be split as the wind tunnel's can.

So, for fins like these, whose linear theory starts at `M_s` = 1.2: from Mach 1.5 up, trust hpr's
slope to about 10% (the rows run +9.4% to −3.3%) and its CP to about half a calibre, which the long
model misses by 0.03 at Mach 1.8 and 0.02 at 2.3; between Mach 0.8 and `M_s`, in the join, neither. A fin set's own `M_s` is
[`FinSetAero::fin`](../api/hpr_aero/model/struct.FinSetAero.html#structfield.fin)`.supersonic_mach`,
from [`AeroModel::fin_sets`](../api/hpr_aero/model/struct.AeroModel.html#method.fin_sets). Fins
swept further back start later: a leading edge swept 48° starts at Mach 1.5, and until then it
is in the join. Nothing past Mach 4.63 has been checked, though the model runs to 5.

#### The body alone, against the 15% target

What this covers: how far hpr's body alone is from NASA's measurement of the same body, and where
what is left of the gap sits. How far to trust it: at Mach 3.96 and 4.63 the two agree within 5%;
below that hpr reads up to 38% high. On five of the six rows outside the target most of that
excess is [body lift](#body-lift); on the sixth it is the [method](#bodies-faster-than-sound)
itself.

The milestone [M1.8e](../decisions-and-roadmap.md#m1-8e) set a target before any of this was
built: the Arcas Robin's body alone within 15% at every Mach number from 1.5, and both
configurations' whole-rocket slope within 15% at Mach 3.96 and 4.63. **The second half is met**
(+2.8% to −3.3%). **The first is not**, on six of eleven rows, and this is where they stand
([ADR-040][adr-040]). Reading the table:

- **Rows outside the target are in bold.** Slopes are per radian on the body's cross-section.
- **`M/f_n`** is the Mach number over the nose's
  [fineness](../glossary.md#fineness-ratio), the argument TN 3527 ([SD56]) states its method for
  from 0.4 to 2. One row, the short model at Mach 1.5, is below that at 0.36.
- **measured** and **hpr** are the straight-line slopes fitted at the tunnel's plotted angles, as
  [ADR-036, which fixes how these comparisons are fitted][adr-036] judges them.
- **at `α → 0`** is the slope at zero angle. hpr's is the method alone, since body lift vanishes
  there; the measurement's comes from fitting its points with `C_N = a α + b α |α|`, the form the
  tunnel's own curves follow, and `a` is quoted with its
  [standard error](../glossary.md#standard-error).
- **curvature** is the rest: the fitted slope less the slope at `α → 0`. For hpr it is body lift.

| model | Mach | `M/f_n` | measured | hpr | difference | measured at `α → 0` | hpr at `α → 0` | at `α → 0`, hpr ÷ measured | curvature, hpr ÷ measured |
|---|---|---|---|---|---|---|---|---|---|
| short | **1.5** | 0.36 | 2.192 | 3.017 | +37.7% | 1.779 ± 0.32 | 1.852 | 1.04 | 2.82 |
| short | **1.8** | 0.43 | 2.613 | 3.290 | +25.9% | 2.519 ± 0.33 | 2.143 | 0.85 | 12.20 |
| short | **2.3** | 0.55 | 3.078 | 3.598 | +16.9% | 2.196 ± 0.32 | 2.394 | 1.09 | 1.37 |
| short | **2.96** | 0.71 | 3.284 | 3.838 | +16.9% | 2.184 ± 0.30 | 2.612 | 1.20 | 1.11 |
| short | 3.96 | 0.95 | 3.884 | 3.946 | +1.6% | 2.694 ± 0.32 | 2.735 | 1.02 | 1.02 |
| short | 4.63 | 1.11 | 4.149 | 3.950 | −4.8% | 2.758 ± 0.32 | 2.718 | 0.99 | 0.89 |
| long | **1.8** | 0.43 | 3.159 | 3.770 | +19.4% | 1.920 ± 0.42 | 2.143 | 1.12 | 1.31 |
| long | **2.3** | 0.55 | 3.525 | 4.071 | +15.5% | 2.245 ± 0.41 | 2.394 | 1.07 | 1.31 |
| long | 2.96 | 0.71 | 3.868 | 4.400 | +13.7% | 2.521 ± 0.36 | 2.614 | 1.04 | 1.33 |
| long | 3.96 | 0.95 | 4.455 | 4.428 | −0.6% | 3.129 ± 0.41 | 2.740 | 0.88 | 1.27 |
| long | 4.63 | 1.11 | 4.615 | 4.425 | −4.1% | 2.877 ± 0.41 | 2.724 | 0.95 | 0.98 |

**What the rows say, and what they can't.** What is solid is the first three number columns: on six
rows hpr's fitted slope is 15% to 38% above the tunnel's, and at Mach 3.96 and 4.63 it is within
5%. The split into a slope at `α → 0` and a curvature is softer, and it is worth saying why before
leaning on it. The tunnel plots seven points over about ±4.5°, and in a fit of
`C_N = a α + b α |α|` over so short a span the two terms trade off almost exactly: their
correlation is −0.96. A fit that reads `a` low must read `b` high. So the measurement's own split
carries the standard errors in the table — ±0.30 to ±0.42 per radian, the widest of them on a
slope of 1.920 — and the curvature, being the same slope subtracted from another, carries at least
as much.

**Where the gap most likely is.** With that said: at `α → 0` hpr is within 1.5 standard errors of
the measurement on every row outside the target (0.2 to 1.5 of one), so the readings cannot
convict the shock-expansion method, the Newtonian cap or the boattail's measured share. On five of
those six rows most of the fitted gap sits in the curvature instead — what the rest of the plotted
angles add, which for hpr is body lift. The sixth is the short model at Mach 2.96, where 77% of the
gap is hpr's slope at `α → 0`, 1.2 times the measured one: there the method itself, not body lift,
carries most of the miss. Hpr's curvature is 1.3 to 2.8 times the measured one below Mach 2.96,
1.1 to 1.3 times it at Mach 2.96, and 0.89 to 1.27 times it at Mach 3.96 and 4.63. The ×12.20 on
the short model at Mach 1.8 is not a measurement of anything: the tunnel's own curve barely bends
there (0.094 per radian, against an uncertainty three times its size), so the ratio's denominator
is consistent with zero.

The likeliest single cause is Jorgensen's crossflow term, which [ADR-037][adr-037] chose because
the tunnel's high-angle points support its size: at the few degrees these slopes are fitted over it
reads too large. Two other explanations are open and the readings do not close them — the short
model at Mach 1.5, the worst row at +37.7%, sits at `M/f_n` 0.36, below the 0.4 that TN 3527 states
its method for, and the same model at Mach 2.96 points at the method rather than at body lift.

**What would close it.** A cited rule for how the crossflow term grows from zero over the first few
degrees, or measurements of this body at finer angles than the reports plot. Neither is in hand,
so the gap is left visible here rather than tuned away. The rows are in
[`validation/fixtures/aero/arcas-robin-body-gap.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-body-gap.json),
written by `cargo xtask aero`, and a test pins which rows are outside.

### Checking the shock-expansion method

The second-order shock-expansion method of [Bodies faster than sound](#bodies-faster-than-sound),
which a flight uses from Mach 1.2 on the bodies it covers, against two references in the fixture
[`validation/fixtures/aero/shock-expansion.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/shock-expansion.json).
`cargo xtask aero` writes it, and `shock_expansion::tests::against_tn3527_and_the_arcas_robin`
recomputes every value and pins every miss ([ADR-033][adr-033]).

To check a share by hand from outside the crate,
[`ShockExpansionBody::element_flows`](../api/hpr_aero/shock_expansion/struct.ShockExpansionBody.html#method.element_flows)
reports each element's flow — the state behind its corner, the tangent cone it relaxes toward, how
fast it does so, and the radius eq. 19 needs — which is what the library's own hand integral of a
boattail and the tube behind it uses (`footnote_eights_boattail_share_by_hand`).

**The tip cone's flow** (`cone_flow_agrees_with_naca_1135_charts`), against [R1135]'s cone
charts 5 to 7 at Mach 1.5 to 3 and cones of 10° and 20°: the shock angle within 0.3°, the
surface pressure coefficient within 0.004 and the surface Mach number within 0.015, twice the
charts' reading error, since they are drawn for `γ = 1.405` and hpr uses 1.4. At Mach 2 on a 10°
cone hpr gives a shock at 31.21° against the chart's 31.25°.

**The report's own tables** ([SD56] Tables I and II, transcribed from the page images into
[`tn3527-bodies.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/tn3527-bodies.json)).
They cover 144 bodies: cones and tangent ogives of fineness 3, 5 and 7, on cylinders of 0 to 10
calibres, at Mach 3, 4.24, 5.05 and 6.28. For each they give the method's slope and CP, as its
authors computed them by hand in 1956, and, for all but the 8-calibre cylinders, NASA's
wind-tunnel measurements. The targets were set before measuring: within 0.05 per radian and
0.1 calibres of the report's values, and within the ±0.2 per radian and ±0.2 calibres the report
claims against its measurements. Each cell counts the rows within, with the range of hpr's value
less the reference's:

| nose | slope within 0.05 per radian of the report's | CP within 0.1 calibre of the report's | slope within 0.2 per radian of the measured | CP within 0.2 calibre of the measured |
|---|---|---|---|---|
| cone, fineness 3 | 24 of 24 (−0.006 to +0.024) | 24 of 24 (−0.003 to +0.040) | 20 of 20 (−0.128 to +0.145) | 19 of 20 (−0.194 to +0.250) |
| cone, fineness 5 | 23 of 24 (−0.003 to +0.066) | 24 of 24 (−0.003 to +0.072) | 20 of 20 (−0.075 to +0.176) | 19 of 20 (−0.165 to +0.272) |
| cone, fineness 7 | 11 of 24 (−0.001 to +0.146) | 16 of 24 (−0.003 to +0.257) | 18 of 20 (−0.061 to +0.251) | 16 of 20 (−0.153 to +0.328) |
| tangent ogive, fineness 3 | 12 of 24 (−0.115 to +0.014) | 19 of 24 (−0.670 to +0.062) | 19 of 20 (−0.278 to +0.008) | 17 of 20 (−0.540 to +0.104) |
| tangent ogive, fineness 5 | 15 of 24 (−0.134 to +0.009) | 20 of 24 (−0.181 to +0.090) | 20 of 20 (−0.111 to +0.138) | 19 of 20 (−0.143 to +0.217) |
| tangent ogive, fineness 7 | 17 of 24 (−0.072 to +0.013) | 22 of 24 (−0.107 to +0.128) | 20 of 20 (−0.107 to +0.142) | 19 of 20 (−0.206 to +0.147) |

The 12 cones with no cylinder only read Fig. 2 back, so against the report's values they check
the hand reading, not the method.

A worked example: a cone of fineness 5 on a cylinder 4 calibres long, at Mach 4.24. Slender-body
theory gives 2 per radian at any length. The tip cone alone gives 1.868 (Fig. 2). With the
cylinder hpr gives 2.922, the report 2.91, and the wind tunnel 2.84.

In all, against the measurements, 117 of 120 slopes and 109 of 120 centres of pressure are within
the report's ±0.2; against the report's own values, 102 of 144 slopes and 125 of 144 centres of
pressure are within 0.05 and 0.1. That is 75 of the 528 comparisons outside, so the targets are
not met ([ADR-033][adr-033] records it):

- **Against the report's own values** (61 misses), in two kinds.
  - *Where the march stays inside the method's limit* (49 misses), hpr follows the printed
    equations and the printed values depart from them. The largest are the fineness-7 cone on
    long cylinders (hpr high, up to +0.146 at Mach 6.28 over 10 calibres) and the ogives (hpr
    low, most at fineness 5 and Mach 3, down to −0.134). The evidence is a second implementation
    of the same equations, written from the paper during this work with its own cone solver.
    For the cone-cylinders it used the report's closed form (its Appendix C), for the ogives its
    ten-element march. With hpr's hand-read Fig. 2, it agrees with hpr within 0.0001 per radian
    on all 72 cone-cylinders and within 0.0006 per radian and 0.0003 calibres on the 60
    ogive-cylinders that stay inside the limit. It is an uncommitted scratch script by the same
    author, so it can't be rerun from the repository, and it can't catch a misreading both
    share. Why the printed values differ is not known. The report took its cone pressures from
    charts (its Fig. 1), and a thin cone's small pressure differences are sensitive to them; that
    is a guess, not a finding.
  - *Where the march reaches the method's limit* (12 misses), on the fineness-3 ogive at Mach
    5.05 and 6.28, near the tip. There hpr's CP at Mach 5.05 sits 0.19 to 0.67 calibres ahead of
    the report's, and the report's measurements agree with the report, so this is hpr's gap, not
    the report's. The report doesn't say how it continued past its limit. Carrying the pressure
    gradient on through the reduced elements comes closer at Mach 5.05 but further at Mach 6.28,
    and it doesn't settle as elements are added. This is open
    ([issue #81](https://github.com/nrdptel/hpr-sim/issues/81)).
- **Against the measurements** (14 misses), in four groups:
  - *The fineness-7 cone on long cylinders* (6). The report is already 0.07 to 0.15 high there,
    and hpr, following the closed form, adds 0.06 to 0.19 more.
  - *Rows where the report is itself 0.20 to 0.22 off* (3): the fineness-5 and fineness-3 cones'
    CP at Mach 6.28 over 10 calibres, and the fineness-5 ogive's at Mach 5.05 over 10.
  - *The fineness-3 ogive at Mach 5.05* (4): the limit above; the worst misses, −0.278 per
    radian and −0.540 calibres.
  - *The fineness-7 ogive's CP at Mach 5.05 over 4 calibres* (1): −0.206, just outside, where
    the report reads −0.13.

**The Arcas Robin** ([D4014], the wind tunnel of
[Normal force through Mach 1](#normal-force-through-mach-1)). The method needed a pointed tip when
this comparison was made (the committed nose, behind the cap it now takes, is compared in
[Blunt tips](#blunt-tips)), so here the nose is the secant ogive (a circular arc meeting the body at an angle) through the tip
and base that best fits the report's coordinate table, 4.17 calibres long, so the report's
Mach-over-fineness range of 0.4 to 2 covers Mach 1.67 to 8.3: its arc radius is 1.744 times a
tangent ogive's, it misses the table by 0.003 in
rms, and its tip half-angle is 10.76°. hpr's committed design keeps its power-series nose, whose
tip is blunt. The measured slope is the fins-off reading fitted over the plotted angles, as
above. It includes the boattail, the lip behind it, and crossflow lift at those angles. The
method has neither the lip nor crossflow at `α → 0`, and takes the boattail only by the report's
footnote 8, so the target is not applied here; it is applied to the body a flight flies, in
[The body alone, against the 15% target](#the-body-alone-against-the-15-target). This table is the method's own; the body a flight flies since
[M1.8e6](../decisions-and-roadmap.md#m1-8e6), with the boattail's measured share, is compared
below.

| model | Mach | measured | nose and cylinder | error | with boattail (footnote 8) | error |
|---|---|---|---|---|---|---|
| short | 1.5 | 2.192 | 2.552 | +16.4% | 2.375 | +8.3% |
| short | 1.8 | 2.613 | 2.724 | +4.3% | 2.580 | −1.2% |
| short | 2.3 | 3.078 | 2.931 | −4.8% | 2.828 | −8.1% |
| short | 2.96 | 3.284 | 3.124 | −4.9% | 3.056 | −6.9% |
| short | 3.96 | 3.884 | 3.300 | −15.0% | 3.262 | −16.0% |
| short | 4.63 | 4.149 | 3.371 | −18.7% | 3.345 | −19.4% |
| long | 1.8 | 3.159 | 2.724 | −13.7% | 2.581 | −18.3% |
| long | 2.3 | 3.525 | 2.932 | −16.8% | 2.829 | −19.8% |
| long | 2.96 | 3.868 | 3.127 | −19.2% | 3.059 | −20.9% |
| long | 3.96 | 4.455 | 3.313 | −25.6% | 3.275 | −26.5% |
| long | 4.63 | 4.615 | 3.395 | −26.4% | 3.369 | −27.0% |

The method's slope grows with Mach number, as the measurement does: 2.55 to 3.37 on the short
model, where slender-body theory keeps its nose at 2. By the end of either cylinder the lift has
died away, so the long model gets almost nothing more (3.313 against 3.300 at Mach 3.96), while
its measurement is 0.57 higher. That difference goes with the longer body's larger side area,
the mark of crossflow lift. The boattail column here is the report's footnote 8, the method's own
rule, which a flight used from [M1.8e4](../decisions-and-roadmap.md#m1-8e4); since
[M1.8e6](../decisions-and-roadmap.md#m1-8e6) a flight takes Washington and Pettis's measured share
instead (the worked example under
[The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight)).

The table above is the method alone, at `α → 0`. A flight also adds body lift, which grows as
`sin² α`; the table leaves it out, and the measured line includes it.

**Most of that gap was the comparison, not missing lift**
([M1.8e5](../decisions-and-roadmap.md#m1-8e5) sized each cause of it on hpr's body model before
[M1.8e6](../decisions-and-roadmap.md#m1-8e6), Galejs's body lift and footnote 8's boattail;
[the research note][gap-note] has the tables). The measurement is a straight line through points
from about −5° to +4°, and crossflow lift, which grows as `α |α|`, steepens it. Fitted the same way
at the same angles, with the body lift a flight added, hpr's body with its boattail read 14.9% to
73.2% high at every Mach number (fixture [`arcas-robin-gap.json`][gap-fixture], which
`cargo xtask aero` writes and `aero_gap::tests::committed_fixture_is_current` keeps current).

- Crossflow lift steepens that fitted line. In hpr's body lift it adds 1.40 to 2.09 per radian;
  in the tunnel's own points (fitted with an `α |α|` term, the curvature) it adds 0.09 to 1.74.
  From Mach 2.3 the tunnel's curvature matches body lift with a factor `K`
  ([Bodies of revolution](#bodies-of-revolution)) from 0.66 to 1.05, each ±0.18 to ±0.23 (one
  standard error), where hpr used 1.1. Jorgensen's crossflow method ([J77] eq. 2.12, Fig. 4 and
  p. 15) gives about 0.9 for these bodies at small angles.
- The fit's slope at `α → 0` and its curvature move together (their errors correlate at −0.95
  to −0.96), so the readings can't split hpr's excess between body lift and its slope at
  `α → 0`. With body lift at Jorgensen's size alone, hpr still reads 8.2% to 60.7% high.
- The one sized cause that size is the boattail's share: TN 3527's footnote 8, which a flight used,
  gives −0.177 to −0.026, slender-body theory −1.324. The lip, which the method can't take, adds
  +0.178 by slender-body theory; a blunt tip like the tunnel's, scaled from a blunter one
  measured, loses 0.015 to 0.07 past Mach 3. Below Mach 3, where the tangent cones' slopes are
  held at TN 3527's Fig. 2 Mach 3 curve, Sims's tables ([S64] Table 2, p. 20) move the nose and
  cylinder's share by −0.031 to +0.056 at most. hpr's reading of
  [issue #81](https://github.com/nrdptel/hpr-sim/issues/81) (how it continues the method where
  TN 3527's relaxation condition fails) changes nothing on this body.

From then on the Arcas Robin is judged like for like, at the tunnel's angles
([ADR-036][adr-036]).

**Crossflow's size and the boattail's share, together.** Since [M1.8e6](../decisions-and-roadmap.md#m1-8e6) body lift takes
Jorgensen's crossflow ([Body lift](#body-lift)) and a boattail Washington and Pettis's measured
share ([The body faster than sound in a flight](#the-body-faster-than-sound-in-a-flight))
([ADR-037][adr-037]). Fitted like for like, as the tunnel's line is, the same body reads +3.4% to
+41.0%. Each change takes about half of the old excess off; from Mach 3.96 both models are within
15%, and from Mach 1.5 to 2.96 hpr still reads 16% to 41% high. The columns are hpr's body model
before [M1.8e6](../decisions-and-roadmap.md#m1-8e6), each change alone, and both; the last column is the current model's slope at
`α → 0`, which leaves body lift out. Slopes per radian on the body's cross-section; the measured
line's standard error takes each reading's accuracy as independent.

| model | Mach | measured, fitted | before | Jorgensen's crossflow alone | measured boattail alone | both (current) | current at `α → 0` |
|---|---|---|---|---|---|---|---|
| short | 1.5 | 2.19 ± 0.09 | 3.80 (+73.2%) | 3.53 (+61.2%) | 3.35 (+52.9%) | 3.09 (+41.0%) | 1.93 |
| short | 1.8 | 2.61 ± 0.09 | 3.98 (+52.2%) | 3.72 (+42.4%) | 3.57 (+36.6%) | 3.31 (+26.8%) | 2.17 |
| short | 2.3 | 3.08 ± 0.08 | 4.29 (+39.4%) | 4.03 (+30.8%) | 3.92 (+27.2%) | 3.65 (+18.6%) | 2.45 |
| short | 2.96 | 3.28 ± 0.08 | 4.54 (+38.1%) | 4.28 (+30.2%) | 4.20 (+27.8%) | 3.94 (+19.8%) | 2.72 |
| short | 3.96 | 3.88 ± 0.09 | 4.71 (+21.3%) | 4.47 (+15.0%) | 4.41 (+13.6%) | 4.17 (+7.3%) | 2.96 |
| short | 4.63 | 4.15 ± 0.09 | 4.79 (+15.5%) | 4.57 (+10.2%) | 4.51 (+8.7%) | 4.29 (+3.4%) | 3.06 |
| long | 1.8 | 3.16 ± 0.11 | 4.50 (+42.4%) | 4.20 (+33.0%) | 4.09 (+29.4%) | 3.79 (+20.0%) | 2.17 |
| long | 2.3 | 3.53 ± 0.11 | 4.80 (+36.1%) | 4.50 (+27.6%) | 4.42 (+25.5%) | 4.12 (+17.0%) | 2.45 |
| long | 2.96 | 3.87 ± 0.11 | 5.14 (+32.9%) | 4.84 (+25.1%) | 4.80 (+24.1%) | 4.50 (+16.3%) | 2.72 |
| long | 3.96 | 4.46 ± 0.11 | 5.23 (+17.3%) | 4.96 (+11.2%) | 4.92 (+10.5%) | 4.66 (+4.5%) | 2.97 |
| long | 4.63 | 4.62 ± 0.11 | 5.30 (+14.9%) | 5.06 (+9.7%) | 5.02 (+8.7%) | 4.78 (+3.5%) | 3.08 |

The rows are in
[`arcas-robin-crossflow.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-crossflow.json),
which `cargo xtask aero` writes and `aero_crossflow::tests::committed_fixture_is_current` keeps
current; `aero_crossflow::tests::the_guide_quotes_the_fixture` checks this table against it cell
by cell. What is left at Mach 1.5 to 2.96 can't be split between body lift and the slope at
`α → 0` from these readings, as [M1.8e5](../decisions-and-roadmap.md#m1-8e5) found. On the short
model at Mach 1.5 and 1.8 the tunnel's points from −5° to +4° barely curve (a factor of 0.32 ± 0.24
and 0.07 ± 0.25 on body lift, where hpr uses about 0.9), while hpr's slope at `α → 0` (1.93 and
2.17) lies within about one standard error of the tunnel's (1.78 ± 0.32, and 2.52 ± 0.33, which
hpr is 1.1 below): there most of the excess is body lift, which at 6° is already about half of
hpr's normal force, and the points at 6° need a factor of 0.47 and 0.58 on it. From Mach 2.3 the
curvature gives factors of 0.66 to 1.05, each within about one standard error (±0.18 to ±0.23) of
Jorgensen's, five of the eight below it. The fixture also holds each of the 62 points above +4° and
the boattail's share at `α → 0` under each rule. One caution on the long model: its points from
−5° to +4° are [M1.8a](../decisions-and-roadmap.md#m1-8a)'s reading, which may carry a skew of the
page that puts its slope 3% to 5% high
([issue #97](https://github.com/nrdptel/hpr-sim/issues/97)); its rows here depend on how that is
settled.

**Where the body's lift acts.** A body model can match the slope with its lift in the wrong place,
so the body's centre of pressure is checked too, from the tunnel's fins-off pitching moment (read
for this milestone into
[`arcas-robin-fins-off-moment.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-fins-off-moment.json),
±0.02 to ±0.025 in `C_m`). Both are taken the way the tunnel's are: straight lines through the
pitching moment and the normal force at the plotted angles up to ±4.5°, calibres from the nose
tip; the measured ones carry about ±0.5 calibres from the readings. The measured boattail share,
which takes lift off at the tail, moves hpr's centre of pressure forward, toward the tunnel's:
before [M1.8e6](../decisions-and-roadmap.md#m1-8e6) hpr put it 0.90 to 3.85 calibres aft of the tunnel's on the short model and 0.59 to
1.94 on the long; now −0.19 to +1.59 and −0.88 to −0.18. Two rows still miss by more than the
readings' half a calibre: the short model at Mach 1.5 and 1.8, where the slope misses most too.

| model | Mach | measured, calibres from the tip | before | current |
|---|---|---|---|---|
| short | 1.5 | 1.00 | 4.85 (+3.85) | 2.59 (+1.59) |
| short | 1.8 | 2.36 | 4.98 (+2.62) | 3.03 (+0.68) |
| short | 2.3 | 3.56 | 5.27 (+1.71) | 3.67 (+0.10) |
| short | 2.96 | 3.21 | 5.20 (+1.99) | 3.77 (+0.56) |
| short | 3.96 | 4.88 | 5.79 (+0.91) | 4.69 (−0.19) |
| short | 4.63 | 5.05 | 5.94 (+0.90) | 4.95 (−0.09) |
| long | 1.8 | 4.61 | 6.55 (+1.94) | 4.27 (−0.33) |
| long | 2.3 | 5.04 | 6.79 (+1.74) | 4.86 (−0.18) |
| long | 2.96 | 5.33 | 6.43 (+1.11) | 4.65 (−0.68) |
| long | 3.96 | 6.19 | 6.78 (+0.59) | 5.31 (−0.88) |
| long | 4.63 | 6.40 | 7.35 (+0.96) | 6.12 (−0.28) |

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

### Roll against the Arcas Robin and the Basic Finner

What is checked: hpr's roll forcing against NASA's measured roll effectiveness of the two Arcas
Robin models from Mach 1.5 to 4.63 (TN D-4014 Fig. 14, [D4014]), and its roll damping against the
Basic Finner's measured from Mach 1.5 to 3.0 and Barrowman's own computed value at Mach 0.07
([B67] Figs. 5-6 and 5-7). The readings are in [the wind-tunnel fixture][wind-tunnel] and
[the Basic Finner's][finner-fixture]; `cargo xtask aero` writes the comparison to
[the roll fixture][roll-fixture], and `hpr_aero::tests::roll_against_mach` recomputes every row.
The roadmap set no target.

**The forcing.** `C_lδ` per degree of cant, on the body's cross-section and diameter, at an angle
of attack of 0 (the reports' symbol is per degree; the models' fins were canted 2°):

| Mach | model | measured `C_lδ` (the report's) | hpr's `N C_lδ k_T(B)` | error |
|---|---|---|---|---|
| 1.5 | short | 0.1684 | 0.2489 | +47.8% |
| 1.8 | short | 0.1722 | 0.1968 | +14.3% |
| 1.8 | long | 0.1670 | 0.1968 | +17.8% |
| 2.3 | short | 0.1449 | 0.1495 | +3.2% |
| 2.3 | long | 0.1460 | 0.1495 | +2.4% |
| 2.96 | short | 0.1152 | 0.1151 | −0.1% |
| 2.96 | long | 0.1140 | 0.1151 | +1.0% |
| 3.96 | short | 0.0874 | 0.0861 | −1.4% |
| 3.96 | long | 0.0870 | 0.0861 | −1.0% |
| 4.63 | short | 0.0721 | 0.0739 | +2.5% |
| 4.63 | long | 0.0780 | 0.0739 | −5.3% |

The two models differ only in the body's length ahead of the fins, which hpr's forcing doesn't
see; the measured values differ by up to 0.006 per degree (8%, at Mach 4.63), and the report
calls the effectiveness "about the same for either vehicle" ([D4014] p. 6). The short model's
readings were corrected when roll was added: the first reading had put each of its panels' zeros 0.005 to
0.007 above the grid line it lies on ([ADR-031][adr-031]). From Mach 2.3 all 8 are within 5.3%.
At Mach 1.5 and 1.8 hpr reads high, as
Barrowman found for another sounding rocket: linear theory's load climbs toward Mach 1 faster than
the fins' does. Without the body factor `k_T(B)` (0.935 here) every value would be 7% higher.

**The damping.** `C_lp` of the Basic Finner, four square fins one diameter in chord and span on a
body one diameter across, per unit of `p d/(2V)`:

| Mach | reference | reference's `C_lp` | hpr's `N C_lp k_R(B)` | error |
|---|---|---|---|---|
| 0.07 | Barrowman's computed curve (chose the method; not a validation) | −34.21 | −33.53 | −2.0% |
| 1.51 | wind tunnel | −33.60 | −31.62 | −5.9% |
| 1.82 | wind tunnel | −27.45 | −25.31 | −7.8% |
| 2.27 | wind tunnel | −23.48 | −20.12 | −14.3% |
| 2.60 | wind tunnel | −20.92 | −17.61 | −15.8% |
| 3.00 | wind tunnel | −18.32 | −15.36 | −16.2% |

hpr reads low faster than sound, more so as the Mach number grows. Barrowman's own curve, from
Busemann's third-order expansion ([B67] eq. 3-7, a higher-order theory that counts the fins'
thickness), is 5.68% from the same points on average
([B67] p. 66); first-order theory, hpr's, reads low partly for want of a term for the fins' 8%
thickness. At Mach 0.07 hpr gives his computed value within 2.0%, which is how hpr's reading of his
damping method, the fin's own slope over the strips, was checked (above).

**The flight** (`hpr_sim::tests::canted_fins_spin_to_the_analytic_balance`): Valetudo with 1° of
cant at 100 m/s, with no drag and no gravity, settles on the closed-form steady roll rate of the
worked example, −16.948 rad/s, within 1e-6 (the test's bound; 1e-11 measured), and one time
constant in is within 1e-5 of the exponential approach (2e-10 measured); no pitch or yaw
appears.

**The pieces** (`hpr_aero::fins::tests`): the polygon's span moments against the trapezoid's and the
ellipse's integrals ([N09] eq. 3.70–3.71); the supersonic forcing and damping against a
20,000-strip sum of the same load on the Arcas Robin's swept fin, within 1e-7, from Mach 1.5 to
4.63; the subsonic ones against Barrowman's closed forms, and both continuous at Mach 0.8 and
`M_s`; `k_R(B)` against its integral (eq. 3-121) by Simpson's rule within 1e-10.

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
[roll-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/roll-vs-mach.json
[finner-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/basic-finner-roll-damping.json
[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[adr-031]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-031-roll-from-canted-fins-and-roll-damping-by-barrowmans-strip-theory-2026-09-19
[adr-032]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-032-normal-force-overrides-from-rasaero-ii-the-static-force-replaced-hprs-damping-kept-2026-09-19
[adr-033]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-033-the-body-faster-than-sound-syvertson-and-denniss-second-order-shock-expansion-method-2026-09-19
[gap-note]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/body-supersonic-gap.md
[adr-036]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-036-the-arcas-robins-supersonic-body-gap-judged-as-the-tunnel-measures-m18e6-takes-crossflows-size-and-the-boattail-2026-09-19
[adr-037]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-037-body-lift-by-jorgensens-crossflow-at-every-speed-and-a-boattails-measured-share-faster-than-sound-2026-09-19
[adr-038]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-038-blunt-and-vertical-nose-tips-faster-than-sound-by-a-newtonian-cap-the-method-started-from-the-tangent-cone-2026-09-19
[adr-039]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-039-a-lip-in-a-boattails-wake-carries-nothing-faster-than-sound-2026-09-19
[adr-040]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-040-a-steep-boattail-reads-its-measured-correlation-no-steeper-than-16-and-m18es-15-target-judged-2026-09-19
[adr-041]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-041-a-lips-shelter-is-weighed-as-the-drag-buildup-weighs-it-not-switched-at-a-threshold-2026-09-20
[adr-042]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-042-cone-slopes-from-24-to-30-come-from-simss-tables-where-tn-3527s-chart-stops-2026-09-20
[adr-043]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-043-the-blunt-tips-handover-cap-what-it-is-worth-and-what-stops-it-moving-2026-09-20
[adr-044]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-044-what-the-answer-follows-when-it-follows-the-mesh-is-a-crossing-of-the-tangent-cone-not-a-reduced-element-2026-09-20
[adr-045]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-045-where-a-flares-march-stops-is-the-corners-isentropic-turn-not-the-shock-detaching-2026-09-20
[adr-047]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-047-a-flare-flies-the-method-where-its-corners-shock-is-attached-and-is-read-drawn-out-where-it-is-not-2026-09-20
[adr-048]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-048-what-a-marched-flare-is-worth-measured-against-tn-d-4865s-model-2-2026-09-20
[adr-049]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-049-what-a-step-in-radius-costs-and-why-the-obvious-fix-is-not-taken-yet-2026-09-20
[adr-050]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-050-a-reduced-element-is-read-by-the-generalized-method-wherever-it-has-a-tangent-cone-of-its-own-2026-09-20
[adr-034]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-034-the-bodys-supersonic-normal-force-in-flight-tabulated-shock-expansion-shares-joined-linearly-from-mach-12-2026-09-19
[gap-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-gap.json
