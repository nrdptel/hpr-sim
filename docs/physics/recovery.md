# Recovery

How hpr flies a rocket under a parachute, a streamer or tumbling, and how a separated stack flies
every body to its own landing: the drag area of a device, when it opens, how it fills, and the
equations of the descent. Code: `crates/hpr-sim/src/recovery.rs` and the descent branch of
`crates/hpr-sim/src/dynamics.rs`. Decisions: [ADR-012][adr-012] (parachutes and the descent),
[ADR-013][adr-013] (streamers and tumble), [ADR-014][adr-014] (separation).

Sources:

- T. W. Knacke, *Parachute Recovery Systems Design Manual*, NWC TP 6575 (1991), for canopy drag
  coefficients, filling times, drag-area growth and the equilibrium descent speed. Its title page
  limits distribution, so it is cited, never redistributed (`docs/VALIDATION.md`).
- RocketPy 1.13.0 (MIT), `rocketpy/simulation/flight.py:2710-2790` and
  `rocketpy/rocket/parachute.py`, for the point-mass descent that the parachute milestone
  ([M1.7a][roadmap]) is compared against.
- J. Carruthers and A. Filippone, "Aerodynamic Drag of Streamers and Flags", *Journal of Aircraft*
  42(4), 2005, and the OpenRocket technical documentation v13.05 (CC BY-SA), Appendix C, for
  streamers; the same documentation's §3.5 for tumbling bodies; and C. Kidwell's NARAM-43 drop
  tests (2001) as the measurement both streamer models are checked against.

## Drag area

A device's drag is set by its drag area `C_D S`, m²:

- **Given directly** (`DeviceDrag::DragArea`), which is RocketPy's `cd_s`.
- **From a canopy** (`DeviceDrag::Canopy`): `C_D S = C_D0 · π D₀²/4`, with `D₀` the nominal
  diameter and `C_D0` the drag coefficient on the **nominal** area `S₀ = π D₀²/4`, Knacke's
  convention (printed page 5-2; `D₀ = √(4 S₀/π)`, so `S₀` includes the vent and every opening).

`CanopyType` carries Knacke's printed data for thirteen canopy types: the `C_D0` range (Table 5-1
for solid textile canopies, Table 5-2 for slotted), the fill constant `n` (Table 5-6, unreefed),
the drag-area growth exponent of Pflanz's method (Figure 5-51) and the infinite-mass opening-force
coefficient `C_x`. Knacke prints a **range** of `C_D0` for every type and no single value; hpr's
default is the middle of the range (flat circular: 0.75 to 0.80, so 0.775). Where Knacke prints
"insufficient data" hpr has `None` and the user has to supply the number.

RocketPy's default parachute `C_D` of 1.4 is not a `C_D0` in this sense: it is a hemispherical
canopy's coefficient on the projected area, which it uses only to turn `cd_s` into a radius for its
added mass. Knacke's hemispherical range on `S₀` is 0.62 to 0.77.

## Streamers

A streamer is a strip of fabric of length `l` and width `w`, so a planform (one-side) area
`S = l w` and an aspect ratio `AR = l/w`. `StreamerModel` picks the correlation:

- **`Filippone`** (the default), on `S`, from wind-tunnel tests of cotton streamers **clamped at
  the leading edge** at `AR` 3.3 to 30 and 6 to 18.9 m/s. The paper fits one power curve per
  planform area, and prints all three:

  | planform area | curve | where |
  |---|---|---|
  | 0.025 m² | `C_D = 0.561 AR^−0.480` | eq. 2 (Figure 2's trend reads `0.561 AR^−0.4795`) |
  | 0.05 m² | `C_D = 0.6514 AR^−0.6075` | the trend line on Figure 3; the text does not repeat it |
  | 0.075 m² | `C_D = 0.405 AR^−0.494` | eq. 1 (Figure 4 reads `0.4046 AR^−0.494`) |

  **hpr** interpolates between *neighbouring* curves linearly in `ln S`, and holds the end curve
  outside the fitted areas; that is hpr's choice, not the paper's. All three are needed because
  `C_D` is far from linear in `ln S`: at `AR = 3.3` the middle curve sits 0.3% *below* the
  smallest area's (0.3154 against 0.3163, with 0.2245 at the largest) rather than 63% of the way
  between them, so blending only the extremes reads 18% low there.

  Two of the paper's own measurements bear on how hpr should read it, and neither is in the
  correlations: a **free** leading edge gives more drag than the clamped mounting these curves
  come from, and lighter, smoother fabric gives less (polyester at 64 g/m² against cotton at 177).
  A rocket's streamer is free and light. It also finds a drag crisis near `Re = 7.2e5` at the
  largest area.
- **`OpenRocket`**: `C_Dm = 0.034 ((ρ_m + 25 g/m²)/(105 g/m²)) ((l + 1 m)/l)` on `S` (Appendix C,
  eq. C.6, printed page 117), fitted to model-rocket streamers (`w` 0.01 to 0.09 m, `l` 0.2 to
  1.0 m, 10 to 80 g/m², 6 to 12 m/s) with a stated 12 to 27% error on an independent set. It is
  the only one of the two that uses the material.

**Which is right?** The two disagree by a factor that depends on the fabric: at `AR = 10` and
`l = 1.016 m` the ratio of their drag areas is 5.8 at 10 g/m², 3.5 at 32 and 1.9 at 80. C.
Kidwell's NARAM-43 drop tests (2001) settle it as far as one dataset can: sixteen 4 in × 40 in
streamers, each with about 5 g at one corner, dropped 20.1 m.

Two details of his method decide how to compare, and both were got wrong here before review: his
rates are **distance over time**, so they are averages over the drop rather than terminal speeds
(a 20.1 m drop averages 0.97 of terminal at 2.8 m/s but 0.89 at 5.9 m/s), and they are
**normalised** to a notional 5.000 g weight. `streamer_models_against_kidwells_drop_tests` uses
the notional mass and compares the same average, from the closed-form fall:

| case | measured | `C_D` it implies, on `S` | `Filippone` | `OpenRocket` |
|---|---|---|---|---|
| crêpe paper, 32 g/m², **unpleated** | 2.80 m/s | 0.155 | 3.05 m/s (+9%) | 5.28 m/s (+88%) |
| Micafilm, 42 g/m², pleated | 2.04 m/s | 0.338 | 3.21 m/s (+58%) | 5.19 m/s (+154%) |

Kidwell's crêpe streamer is the one he left flat, and a flat correlation predicts it to 9%;
appendix C reads 88% fast. His other materials were folded in ¾ in pleats, which more than doubles
the drag again — **hpr models no pleats**, so it predicts a faster descent for a pleated streamer,
which is the safe direction for a landing. Both his cases are a little outside both fits: 0.1032 m²
of planform against Filippone's largest 0.075 m², and 0.1016 m wide by 1.016 m long against
appendix C's `w ≤ 0.09`, `l ≤ 1.0`.

That last point is where hpr's **clamp** above 0.075 m² matters, and the evidence pulls two ways.
The paper's own trend is that `C_D` falls as the area grows: its two end curves at `AR = 10` imply
`S^−0.326`, which extrapolated to Kidwell's 0.1032 m² would give `C_D = 0.117` where the clamp
gives 0.130 (the steeper inner pair, `S^−0.53`, would give 0.110). So the clamp reads high against
the trend. But the drop itself implies 0.155, higher than any of them. hpr holds the end curve
rather than extrapolating because that is the closer of the two to the one free-drop measurement
in hand, and it says so here rather than claiming the trend. Two caveats: that rests on a single
point, at one aspect ratio, with a fabric nothing like the paper's cotton; and it holds partly
because the clamped-luff correlation is itself biased low, so two errors cancel. Nothing is
measured anywhere near the 0.225 m² end of the clamp.

hpr therefore defaults to `Filippone` and keeps `OpenRocket` for comparing with OpenRocket
([ADR-013][adr-013], streamer and tumble drag).

## Tumble

A body with nothing deployed descends broadside, tumbling. `DeviceDrag::tumbling(&assembly)`
computes its drag area from the airframe by the OpenRocket technical documentation's §3.5
(printed pages 53 to 55, eqs. 3.98 and 3.99):

```text
C_D S = 1.42 A_f + 0.56 A_bt
```

- `A_bt` is the body's side profile area. hpr integrates the outer diameter along the axis, taking
  each body component's mean diameter times its length: exact for tubes and cones, approximate for
  a curved nose.
- `A_f` is, for each fin set, **one** fin's planform area times an efficiency factor by fin count:
  0.50, 1.00, 1.50, 1.41, 1.81, 1.73, 1.90, 1.85 for 1 to 8 fins (Table 3.4). It is a fit, not a
  model: four fins are 1.41 of one fin, not 2, and it is not monotonic. More than eight fins is
  refused.
- The documentation notes 0.56 is half a circular cylinder's 1.12 in crossflow, as expected of a
  cylinder falling at a random angle, and that 1.42 is "similar to that of a flat plate 1.17 or an
  open hemispherical cup 1.42". Those come from Hoerner's *Fluid-Dynamic Drag* (1965), which is
  copyrighted with no legal free copy; NASA TN D-540 and TR R-474 carry the same numbers and are
  free (`docs/research/streamer-and-tumble-drag.md`).
- The body's profile uses each component's **end** diameters, so a curved nose is under-counted:
  for Valetudo's tangent ogive the true `∫d dx` is 0.0148 m² against the 0.0111 m² hpr takes, 25%
  low on the nose and 2.2% low on the whole body (1.1% high in terminal speed).

**How well it does.** The documentation says its fit predicts its own five drop-test models within
3 to 14%. hpr does not reproduce that. Replaying Table 3.3 (printed page 54: five models, 22 m,
ρ = 1.31 kg/m³, `v₀` read from video to ±0.3 m/s) through hpr's reading of the model
(`the_tumble_model_against_its_own_drop_tests`):

| model | fins | measured | hpr |
|---|---|---|---|
| #1 | 3 | 5.6 m/s | 5.27 (−5.8%) |
| #2 | 4 | 6.3 m/s | 5.96 (−5.4%) |
| #3 | 3 | 6.6 m/s | 6.13 (−7.2%) |
| #4 | none | 5.4 m/s | 6.43 (**+19.0%**) |
| #5 | fins only | 5.0 m/s | 4.50 (−10.0%) |

So the spread is −10 to +19%, and the finless tube is the outlier: it wants a body coefficient
near 0.79 where the model prints 0.56. The text pins neither the body-profile nor the fin-area
convention, so either hpr reads the areas differently from whoever fitted the constants, or the
claim is not reproducible. The table above is what hpr can demonstrate, so it is what hpr states.

**Where it stops being true.** The fit covers 44 to 103 mm bodies of 6.8 to 160 g descending at
5.0 to 6.6 m/s. A high-power booster is far outside it: Valetudo tumbling comes out at 37 m/s.
A cylinder's crossflow drag falls by roughly half above a Reynolds number near 2e5 (a 100 mm body
reaches that at about 30 m/s, and Valetudo's 80 mm at 37 m/s is right at it), so a real body that
size would descend faster than hpr says. hpr does not model that fall, and nothing in the pinned
sources covers it.

A tumbling body is a device like any other: give it a trigger, and it starts at that moment.
hpr does not decide by itself when a rocket tumbles — nothing citable says when a stage becomes
unstable enough (the same documentation declines to model the analogous streamer regime).

## Triggers, lag and release

A device's charge fires at its `Trigger`:

- `Apogee`: when the centre of mass is descending. hpr's apogee event fires it the instant the
  height rate crosses zero; a flight that *starts* past its apogee (`run_free`, which staging and
  flight-data replay use) fires it at its first step, as RocketPy's own apogee trigger does
  (`y[5] < 0`, `parachute.py:368-376`). Whichever comes first wins; it fires once.
- `Altitude { height_above_ground_m }`: the first time the centre of mass is **descending** and at
  or below that height above the launch site. This is an altimeter's main setting. A rocket whose
  apogee is already below the setting fires at apogee, because no crossing follows; that is
  RocketPy's numeric trigger (`y[5] < 0 and h < trigger`, `parachute.py:354-364`).
- `Time { time_s }`: a time after the first ignition.
- `MotorDelay { motor }`: that motor's ejection delay after its own burnout. The motor must have a
  delay in seconds; a plugged motor or one with no delay set is refused.

Charges are only checked in free flight and during the descent, so a `Time` or `MotorDelay`
trigger whose time passes while the rocket is still on the pad or the rail fires at rail exit.

`lag_s` seconds after the trigger the device **deploys** (line stretch) and starts to fill. The
first deployment of a flight switches it to the descent phase.

A device can name another whose opening **releases** it (`released_by`), which is how a drogue is
cut away under a main. The release happens when the releasing device is **fully open**, not at its
line stretch: cutting the drogue at line stretch would leave the rocket under an empty canopy, and
the drag area would collapse and the descent speed up (found in review; measured at 0.45 m² → 0.02
m² and 18.3 → 22.0 m/s before the fix). A released device contributes nothing from its release, and
one released before its own charge fires never deploys at all — its `Trigger` is recorded and no
`Deployment` follows.

Events, in the order a two-device flight records them: `Apogee`, `Trigger(drogue)`,
`Deployment(drogue)`, `Trigger(main)`, `Deployment(main)`, `Release(drogue)` (at the end of the
main's filling), `GroundHit`.

Trigger times that are known before the flight (a time, or a motor delay) and every deployment and
end of filling are stop times, so no integration step straddles a change in the drag area.

## Inflation

The open devices' drag area at time `t` is the sum over the devices that have deployed and are not
released, each contributing

```text
(C_D S)(t) = (C_D S)₀ · min(1, (t − t_d)/t_f)^j
```

with `t_d` its deployment, `t_f` its filling time and `j` its growth exponent. `Inflation` chooses
`t_f`:

- `Instant`: `t_f = 0`, the full drag area at line stretch. This is RocketPy's model, and the
  upper bound on hpr's opening load.
- `FillingTime { time_s, exponent }`: a filling time fixed in advance.
- `FillConstant { constant, exponent }`: Knacke's `t_f = n D₀/v` (printed page 5-43), with `v` the
  airspeed at line stretch and `n` the canopy fill constant, from Table 5-6's **unreefed** column
  (printed page 5-44). A deployment at rest has no filling time in this law (`n D₀/v` diverges), so
  the canopy is taken as open at once and fills as the rocket picks up speed.

  Knacke states the linear form only "in the medium-velocity range of about 150 to 500 ft/s"
  (45.7 to 152.4 m/s; `Inflation::FILL_CONSTANT_RANGE_M_S`). A hobby main opening at 20 to 30 m/s
  is **below** that range, where his alternative for solid flat circular canopies is
  `t_f = n D₀/v^0.85` with `n = 4.0` — a dimensional form (feet and ft/s) that cannot be used in
  SI as printed, so hpr does not. Outside the range the filling time, and with it the peak load
  hpr reports, is an extrapolation: at 25 m/s the linear form gives a 2.5 m main `t_f = 0.8 s`
  from a correlation fitted at three to six times that speed.

`j = 1` is linear growth (Knacke's ribbon and ringslot canopies) and `j = 2` the concave growth of
solid cloth (Pflanz, Figure 5-51). Knacke's measured drag area **overshoots** the steady value by
10 to 80% near the end of filling (Figure 5-40, printed page 5-47), and his infinite-mass opening
force is `C_x = 1.7` for a flat circular canopy. hpr models neither: its drag area rises to the
steady value and stays. The peak load hpr reports is therefore a lower bound on the real opening
shock, and instant inflation is hpr's own upper bound. For scale, the 1.5 m flat circular canopy
of the test above peaks at 1.6 kN filling and 3.0 kN opening instantly, where Knacke's
infinite-mass `C_x = 1.7` on the same dynamic pressure would be 5.1 kN: size hardware from the
source, not from hpr. Ludtke's law and Pflanz's `X1` reduction factor are candidates for a later
milestone.

## The descent

Once a device is open the rocket is a point mass. In the launch frame, with `m` the mass, `r_cg`
the centre of mass, `v_cg` its velocity, `w` the wind, `ρ` the density at its height and `g` normal
gravity:

```text
m a_cg = −½ ρ (C_D S)(t) |v_cg − w| (v_cg − w) + m (g + a_Coriolis) + T
```

- In code this is the free-flight translational equation of `docs/physics/flight.md` with `ω = 0`
  and the canopy drag in place of the airframe's aerodynamics, so the mass terms of `T20` (the
  centre of mass's motion inside the body, `−m r″ − 2ṁ r′`, and the jet terms) are still there and
  the integrated point is still the nose tip. After burnout every one of them is zero and the
  equation is the one above, with `a_cg = a_O`.
- The drag acts at the centre of mass along the air's relative velocity, so it exerts no moment.
- The attitude and the body rates **freeze** at deployment (the body rates are set to zero), and
  the state's reference point, the nose tip, keeps its rigid offset from the centre of mass. The
  state's nose-tip velocity is shifted by `q(ω × r_cg)` as the rates go, so that the centre of
  mass keeps the velocity it had: snubbing the rotation is internal to the lines and the canopy
  and cannot move the centre of mass's momentum.
- The airframe's own drag is **left out**, as RocketPy leaves it out. A rocket's attitude under a
  canopy, and so the area it presents, is not modelled. For a drogue whose drag area is close to
  the airframe's broadside area this is a real omission; it is the same omission the oracle makes,
  and the tumble model ([M1.7b][roadmap], streamers and tumble) is where a body's own drag belongs.
- The thrust `T` is kept, along the frozen axis, so a device that opens while a motor still burns
  (an off-nominal case) is not silently thrust-free. Its direction is wrong the moment the rocket
  would have swung under the canopy.
- Gravity, the Coriolis force, the atmosphere and the wind are the same models the rest of the
  flight uses (`docs/physics/flight.md`).
- **Added mass is not modelled.** Knacke gives no closed-form apparent mass (printed page 5-40
  says only that it is the enclosed volume times density times a form factor), and RocketPy's
  `m_a = k_a ρ (2/3) π R² H` has no citation in its code. It carries no weight in RocketPy either,
  so it changes no equilibrium descent rate, only the transient right after an opening. It is not
  small: the fixture records 5.6 kg for Calisto's main against a 16.2 kg rocket and 15.9 kg for
  NDRT's against a 20.8 kg one (both at the start height; both grow with density as the rocket
  descends). The comparison below shows what leaving it out costs.

The equilibrium descent speed is Knacke's (printed page 5-128), and `recovery::terminal_speed_m_s`
computes it:

```text
v_e = √(2 m g / (ρ C_D S))
```

## Separation

A `Separation` is a trigger and a stage boundary. At the trigger the stack comes apart: stages
`0..=after_stage` keep the nose and are **body 0**, the stages aft of the split are **body 1**, and
each body flies on as a point mass under the devices that name it (`Device::on_body`). The ascent
ends there: its `FlightResult` has `Termination::Separated`, a `Separation` event, and one
`BodyFlight` per body in `bodies`.

- **Each body is its own stages and their motors.** `body_mass_properties` sums the stages'
  `MassProperties` and the motors mounted in them, so the bodies' masses add to the whole rocket's
  at that instant — which is a test.
- **The separation adds no impulse.** Each body starts at its **own** centre of mass, with the
  velocity that point already had: `v_O + ω × r_cg` in the launch frame. The bodies' **linear**
  momenta therefore add to the stack's, which is a test. Their rotation is dropped, so the angular
  momentum is not conserved: the orbital part survives, each body's spin about its own centre does
  not (31% of it at the 0.6 rad/s of the test, 0.02 J). No spring, no gas pressure, no tip-off: an
  ejection charge's impulse and the tumbling that follows are not modelled.
- **Only body 0's devices act before the separation.** A device meant for another body has a drag
  area computed for that body — a booster's tumbling area, say — which is not a model of the whole
  stack, so it waits for its body. With no separation every device is body 0's.
- **Each body finds its own apogee**, whatever its devices are triggered by. The ascent ends at
  the separation, so this is the only place a staged flight can record a peak, and without it a
  body separated while climbing would never fire an apogee charge (found in review, now a test).
- **A body must start above the ground**, as a free flight must: the ground event is a falling
  crossing, so a body that started below the site would integrate underground to the time cap.
- **Every body must carry a device, and it must open.** The descent has no airframe drag (the
  descent-phase decision, [ADR-012][adr-012]), so a body with nothing open would fall as if in a
  vacuum. A flight whose bodies are not all covered is refused when it is set up, and a body that
  reaches the ground without a single deployment — an altimeter set above that body's own apogee,
  say — is a flight-time error rather than a landing at 170 m/s (both found in review).
- **A body coasts with no drag at all until its first device opens**, which is the same omission
  as the descent phase's and hurts more here: a 0.55 kg sustainer that separates at 2 km and waits
  for a 300 m main arrives at **168 m/s** where an airframe would have held it near 60 to 70, so
  its deployment speed, and any opening load taken from it, read high. Give a body a device that
  opens at once (`DeviceDrag::tumbling_stages` over its own stages is the cited way) if the coast
  matters.
  A spent booster's device is usually `DeviceDrag::tumbling_stages(&assembly, its stages)`, which
  is §3.5's model over that body's own components rather than the whole stack's.
- **The bodies descend independently**, each with the same point-mass equations as the descent
  phase, less the thrust: `m a = −½ ρ (C_D S)(t) |v − w| (v − w) + m (g + a_Coriolis)`. They share
  the flight's devices and their progress, so a canopy that opened before the separation stays open
  on whichever body carries it.
- **A separation must follow the last burnout**, because a body's mass is held constant through its
  descent. A trigger that fires earlier is a flight-time error, not a silent approximation, since
  whether it does depends on the flight. A release across the separation is refused too: a line
  cuts a device on its own body. Powered staging, where a sustainer lights and keeps flying, is
  planned for the staging milestone ([M1.9][roadmap]).
- Bodies are not watched by the `Observer`: their events and samples are in their `BodyFlight`.

## Verification

`crates/hpr-sim/src/recovery.rs`'s tests, all analytic unless they name the oracle:

| What | Result |
|---|---|
| Knacke's `v_e` against Loft's case (1.1 kg, 1 m flat canopy, `C_D` 0.8, ρ 1.225) | 5.294 m/s, as Loft printed |
| A descent from rest against the closed-form fall under quadratic drag (2 km, uniform air, constant gravity) | 2.1e-8 of `v_t` over the whole descent; the landing time within 1e-5 s of the closed form's 204 s |
| Drift in a steady wind, entered drifting with the air, no Earth rotation | exactly the wind times the time of flight (1e-8); the fall itself within 1e-4 of the closed form |
| The Coriolis drift of a 3 km descent, Earth rotation on | 0.3666 m east against the steady prediction `2Ω cos φ · v_t²/g · T` = 0.3685 m, 0.52% apart, and 29 µm north |
| Knacke's filling law, `t_f = n D₀/v` and `(t/t_f)^j` | the recorded drag area to 1e-9 of `(C_D S)₀` |
| Inflation against instant opening (deployed at 60 m/s under a 1.5 m flat circular canopy) | peak load 1,615 N against 3,020 N instant (0.53 of it), between the closed-form 1,527 N without gravity and 1,670 N with it |
| An oversized canopy (5 m) opening at 100 m/s, 10 km of descent at 2.95 m/s | lands in 3,392 s in 6,914 accepted steps and 2 rejected (a mean step of 0.49 s, where Loft's explicit RK4 needed a 2e-4 s floor) |
| A deployment with a 0.7 rad/s body rate, and another inside the burn in a crosswind | the centre of mass keeps its velocity across the handover to 1e-12, and the nose tip's moves by exactly `q(ω × r_cg)` |
| A whole flight: drogue at apogee with a lag, main at 300 m, drogue released | events in order; each stage settles within 2% of its own `v_e` |
| Two devices triggered at the same instant | both open in the same pass, and the descent settles at the `v_e` of the **sum** of their drag areas |
| A device released before its own charge fires | it is recorded as triggered and never deploys; the descent stays at the open device's `v_e` |
| A drogue released by a main that fills over 2 s | the release waits for the end of filling, the drag area never falls below the drogue's, and the descent never speeds up |
| An apogee charge on a flight that starts descending | it fires at the first step (there is no apogee event to find), and a climbing start still waits for the apogee |
| Two user events and an altitude device on one flight | the user events keep their numbers and fire during the descent, in height order |
| The same recovered flight flown twice | bit-identical rows, events, final sample and step counts ([Loft lesson L24][lessons]: a run does not mutate the simulation) |
| A separation at apogee of the two-stage test design, canopy on the sustainer and tumble on the booster | both bodies land: the 0.550 kg sustainer at 729.0 s and 2.11 m/s under its 1.8 m canopy, the 1.125 kg booster at 107.5 s and 16.74 m/s tumbling; the masses add to the 1.675 kg stack to 1e-12 and each lands within 0.1% of its own `v_e` |
| The linear momenta of the bodies at a separation with a 0.6 rad/s body rate | add to the stack's to 1e-9, and each body starts at its own centre of mass to 1e-12 (0.817 m apart on this design) |
| A separation before the last burnout | refused in flight, with the burnout time in the error |
| A separation while still climbing at 100 m/s | both bodies find their own apogee above 1,400 m, fire there, and land within 1% of their own `v_e` |
| A timed separation, and a height separation | fire at their own time to 1e-9 s and at their own height to 1e-6 m, rather than at the next boundary that happens to exist (found in review: one fired 186 s late, another never) |
| A body that runs out of time | says `TimeCap` in its own `BodyFlight`; `FlightResult::bodies_landed` is false and `landings()` is short |
| A body whose device never fires (an altimeter above its apogee) | refused in flight, naming the body, rather than landed at 170 m/s |
| A timed separation known to precede the burnout | refused when the separation is given; a height one that a climbing rocket passes early is refused in flight |

### Against RocketPy

`validation/oracles/rocketpy/recovery.py` flies RocketPy's own parachute phase for five of its
example rockets and writes `validation/fixtures/recovery/rocketpy-descent.json`;
`descent_matches_rocketpy_examples` replays each case in hpr. Both start from the same declared
state after burnout, near apogee, with the first device opening at once (its lag is overridden to
zero, so no ballistic segment under either model's aerodynamics separates them), the same `C_D S`,
the same deployment settings and the same wind, and RocketPy's noise set to zero. The oracle runs
at `rtol = atol = 1e-8`; run again at 1e-6 it moves every compared metric by at most 3.5e-6
(the fixture's `solver.relative_change_from_loose`). Its one larger entry, 2.1e-3, is on
Valetudo's 20 µm *north* drift component, which the parachute milestone ([M1.7a][roadmap]) did not
compare. The validation harness ([M2.1a][roadmap]) does. When it first did, hpr read 28x above
RocketPy, at 0.55 mm, and that is what found the gravity-model difference below
([issue #27](https://github.com/nrdptel/hpr-sim/issues/27)). With that fixed, the two agree within
1.8% ([validation report][report]).

What still differs, and by how much:

- **Added mass.** hpr has none; RocketPy's carries no weight, so it changes no equilibrium, only
  the transient after an opening. This is the largest difference (see NDRT below).
- **Trigger sampling.** RocketPy checks its triggers on a grid of `1/sampling_rate` (100 or
  105 Hz) anchored at `t = 0`, and only over the span after its first accepted step; hpr has no
  sampling rate and locates the crossing with its event finder. So RocketPy's first deployment is
  2.5 ms late in the four 105 Hz cases and 13 ms in Prometheus's, and its `h < setting` predicate
  can only fire at or **below** the setting, by at most one sample of fall: `v_z/rate` is 0.17 m
  for Calisto and 0.27 m for NDRT (about 0.01 s of descent). The heights the fixture records at
  those triggers (800.07 m, 167.93 m, 457.26 m) come from RocketPy's *reporting* spline over its
  stored samples, not from the dense output its trigger read, so they sit just above the setting
  instead. The table below compares hpr's trigger height against those reported values, which is
  the closest the fixture can come; it is a difference of the same size either way.
- **Release against replacement.** hpr sums its open devices and releases the drogue when the main
  is full; RocketPy holds one `C_D S` and replaces it. For these cases, whose canopies open
  instantly, the two are the same.
- **Atmosphere.** hpr evaluates the 1976 standard atmosphere; RocketPy interpolates a 100-point
  pressure table over 0 to 80 km. Measured over the fixture's 23 samples: at most 3.7e-4 in
  density, which the test gates at 5e-4.
- **Gravity.** The same *magnitude*, and for a long time that was all this said. RocketPy's
  "Somigliana" formula is WGS 84 normal gravity and hpr's agrees with the fixture's samples to
  1e-6 — but RocketPy applies it to the vertical axis alone (`Flight.u_dot_parachute`,
  `flight.py:2777`, where only `az` carries a gravity term), while hpr's default
  `GravityModel::Ellipsoidal` uses the full normal-gravity **vector**, which above the ellipsoid
  leans a few parts in 10⁶ toward the pole: 4.0e-6 m/s² at Valetudo's site at ground level and
  8.7e-6 m/s² at 1,468 m, growing in proportion to height above the ellipsoid and pointing toward
  the equator (`docs/physics/gravity.md`): over these five sites it runs from +6.9e-6 m/s² at
  Valetudo's topmost gravity sample to −3.3e-5 m/s² at Calisto's 4,400 m. hpr's vector also turns
  with the local vertical downrange, `g·d/R`, which is 2.1e-3 m/s² at Calisto's 1.4 km of drift and
  is much the larger of the two wherever a rocket drifts at all. The parachute milestone's test
  ([M1.7a][roadmap]) used to compare gravity by magnitude alone, so it could see neither. Both this
  comparison and the validation suite now fly `GravityModel::VerticalTaylor`, which hpr ships as
  RocketPy's own formula for like-for-like comparisons, and both assert the gravity **vector**
  rather than its length.
- **Geometry.** hpr flies over the ellipsoid and takes heights along its normal; RocketPy's `z` is
  flat. Over Calisto's 1.4 km of drift the curvature is 0.15 m of height, 0.03 s of descent.

The test checks the environments agree first, then the descent.

Measured (hpr against RocketPy, 2026-09-17):

| case | descent time | descent rate under the drogue | impact descent rate | drift | worst drift component |
|---|---|---|---|---|---|
| Calisto (drogue 1.0 m², main 10 m² at 800 m, wind 5 E / 2 N) | +0.08% (257.27 s) | −0.01% (17.967 m/s) | −0.03% (5.454 m/s) | +0.08% (1,386.0 m) | +0.08% |
| Valetudo (drogue 0.4537 m², no wind) | −0.02% (45.76 s) | — | +0.00% (17.627 m/s) | −0.89% (0.19 m, Coriolis only) | −1.77% (north, 19 µm; +2704% under hpr's own gravity, issue #27) |
| NDRT 2020 (drogue 0.438 m², main 16.05 m² at 167.6 m, sheared wind) | +0.71% (61.60 s) | +0.01% (28.156 m/s) | +0.01% (4.604 m/s) | +0.28% (327.9 m) | +2.86% (north, −50.8 m) |
| Prometheus 2022 (drogue 0.467 m², main 5.78 m² at 457.2 m) | +0.08% (153.50 s) | −0.01% (26.400 m/s) | −0.03% (7.323 m/s) | +0.08% (1,237.1 m) | +0.09% |
| Juno III (drogue 0.885 m²) | −0.02% (53.56 s) | — | −0.01% (22.431 m/s) | −0.02% (457.9 m) | −0.02% |

These numbers are hpr flown under RocketPy's gravity model, as the comparison has been since
issue #27. Under hpr's own the drifting cases read a little closer — Calisto +0.06% rather than
+0.08% — because the vertical's turn downrange pushes the rocket back toward the pad and cancels
part of a real difference. The like-for-like number is the honest one.

Valetudo's north drift is worth its own paragraph, because it is the number that found the gravity
difference above. In still air it is Coriolis alone: the horizontal velocity relaxes to a drag
balance in about `v_t/g` ≈ 1.8 s, so `v_north ≈ −2 ω_z v_east · v_t/g`, which integrates to 2.0e-5 m
over the descent. RocketPy gives 1.9653e-5 m. Under hpr's default gravity hpr gave 5.51e-4 m, 28x
high, and `(1/g)∫₀^800 g_north dz` = 5.2e-4 m accounts for the difference to within a few percent.
Flown against RocketPy's own gravity formula, as the validation suite does, hpr gives 1.93e-5 m,
−1.8%. Both codes carry the same Coriolis term (hpr in `dynamics.rs`; RocketPy in
`flight.py:2779-2783`), and on this evidence neither is wrong: they were being asked different
questions.

The later devices' trigger heights agree to −0.01%, −0.17% and −0.01% (RocketPy's trigger
sampling, above), and in every case both simulators land within 1% of Knacke's `v_e` for the
device that is open, computed from hpr's own air and gravity at the site.

Every metric is inside the milestone's 3%. The descent rate under the drogue, where a case has a
main, agrees to 0.01%. The two largest gaps are both NDRT's, whose main has a drag area of 16 m²:
RocketPy's added mass for it is 15.9 kg against the rocket's 20.8 kg, so its response to the
opening is slower, which lengthens the descent (+0.71%) and, in a wind that shears with height,
moves the smaller drift component by 2.86%. Adding a cited apparent-mass model would close that
gap.

[adr-012]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-012-recovery-drag-areas-triggers-inflation-and-the-descent-phase-2026-09-17
[adr-013]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-013-streamer-and-tumble-drag-2026-09-17
[adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
