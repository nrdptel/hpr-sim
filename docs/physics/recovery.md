# Recovery

## In short

- **What it models:** how a rocket comes down under a parachute, a
  [streamer](../glossary.md#streamer) or tumbling: when each device fires, how a canopy fills, the
  descent in the wind, and a stack that splits into parts
  ([separation](../glossary.md#separation)) that each land.
- **Sources:** Knacke's *Parachute Recovery Systems Design Manual* (1991) for parachutes;
  Carruthers and Filippone's streamer tests (2005) and the OpenRocket technical documentation for
  streamers and tumbling.
- **How well it is validated:** against another simulator and against drop tests; no descent has
  been compared with a real flight yet.
  - The descent under a parachute matches [RocketPy](../glossary.md#rocketpy)'s for five example
    rockets, started from the same state near apogee: all 30 metrics within 3%, the largest
    +2.865% ([validation report][report]). [Drift](../glossary.md#drift) is measured from that
    shared start, not from the pad.
  - That comparison flies RocketPy's gravity formula and RocketPy's way of interpolating the wind,
    and hpr's defaults differ from both (the full gravity vector, and a wind table interpolated by
    speed and direction). Under hpr's own gravity, Calisto's drift read a little closer, once, before
    the comparison switched (Against RocketPy, below); no test pins that. What hpr's default wind
    interpolation does to drift has not been measured.
  - Against measured drop tests, tumbling is −10 to +19% off, and the default streamer model
    predicts a descent +9% faster than Kidwell's one flat streamer.
- **What it leaves out:**
  - Two effects on the [opening load](../glossary.md#opening-load): the drag overshoot as a canopy
    fills, and, when the canopy opens at once (the default), the way a light rocket slows while it
    fills. So hpr's peak opening load is no safe bound either way. Don't size recovery hardware
    from it: use a dedicated opening-load method and the hardware's own ratings.
  - Airframe drag: a separated body falls with no drag until its device opens, so its
    [deployment](../glossary.md#deployment) speed can read high, and the airframe's own drag under
    a canopy is left out too.
  - [Added mass](../glossary.md#added-mass) (air carried along), the swing (the attitude freezes at
    deployment), and streamer pleats (+58% fast on a pleated one).
  - [Tumbling](../glossary.md#tumble-recovery) is used far outside its fit: 37 m/s for Valetudo,
    against 5.0 to 6.6 m/s.

## Code and sources

Code: [`hpr_sim::recovery`](../api/hpr_sim/recovery/index.html) in the API reference
(`crates/hpr-sim/src/recovery.rs`), and the descent branch of `crates/hpr-sim/src/dynamics.rs`.
Decisions: [ADR-012][adr-012] (parachutes and the descent), [ADR-013][adr-013] (streamers and
tumble), [ADR-014][adr-014] (separation).

Sources:

- T. W. Knacke, *Parachute Recovery Systems Design Manual*, NWC TP 6575 (1991), for canopy drag
  coefficients, filling times, drag-area growth and the equilibrium descent speed. Its title page
  limits distribution, so it is cited, never redistributed (`docs/VALIDATION.md`).
- RocketPy 1.13.0 (MIT), `rocketpy/simulation/flight.py:2710-2790` and
  `rocketpy/rocket/parachute.py`, for the point-mass descent that the parachute milestone
  ([M1.7a](../decisions-and-roadmap.md#m1-7a)) is compared against.
- J. Carruthers and A. Filippone, "Aerodynamic Drag of Streamers and Flags", *Journal of Aircraft*
  42(4), 2005, and the OpenRocket technical documentation v13.05 (CC BY-SA), Appendix C, for
  streamers; the same documentation's §3.5 for tumbling bodies; and C. Kidwell's drop tests
  (2001), a research report for NARAM-43, the National Association of Rocketry's annual meet, as
  the measurement both streamer models are checked against.

## Drag area

A device's drag is set by its [drag area](../glossary.md#drag-area) `C_D S`, in m²: a drag
coefficient times the area that coefficient is measured on. hpr takes it in one of two ways:

- **Given directly** (`DeviceDrag::DragArea`), which is RocketPy's `cd_s`.
- **From a canopy** (`DeviceDrag::Canopy`): `C_D S = C_D0 · π D₀²/4`, with `D₀` the nominal
  diameter and `C_D0` the drag coefficient on the **nominal** area `S₀ = π D₀²/4`, Knacke's
  convention (printed page 5-2; `D₀ = √(4 S₀/π)`, so `S₀` includes the vent and every opening).

`CanopyType` carries Knacke's printed data for thirteen canopy types:

- **The `C_D0` range** (Table 5-1 for solid textile canopies, Table 5-2 for slotted). Knacke prints
  a **range** for every type and no single value; hpr's default is the middle of the range (flat
  circular: 0.75 to 0.80, so 0.775).
- **The fill constant `n`** (Table 5-6), which sets how long the canopy takes to fill: the filling
  time is `n` nominal diameters divided by the speed at line stretch ([Inflation](#inflation)).
  hpr takes it from the table's *unreefed* column, for a canopy that opens freely. (Reefing is a
  line round a canopy's edge that holds it partly closed at first, to cut the opening load.)
- **The drag-area growth exponent of Pflanz's method** (Figure 5-51). Pflanz's method, in
  Knacke's manual, works out the opening force with a drag area that grows as a power of the time
  since line stretch.
- **The infinite-mass opening-force coefficient `C_x`**: the peak force as the canopy opens, over
  its steady drag at the same speed, for a load so heavy that it doesn't slow while the canopy
  fills. hpr reports it; its equations don't use it ([Inflation](#inflation)).

Where Knacke prints "insufficient data" hpr has `None`, and the user has to supply the number.

RocketPy's default parachute `C_D` of 1.4 is not a `C_D0` in this sense: it is a hemispherical
canopy's coefficient on the projected area, which it uses only to turn `cd_s` into a radius for its
added mass. Knacke's hemispherical range on `S₀` is 0.62 to 0.77.

## Streamers

A [streamer](../glossary.md#streamer) is a strip of fabric of length `l` and width `w`, so a
planform (one-side) area `S = l w` and an aspect ratio `AR = l/w`. `StreamerModel` picks the
correlation, the curve fitted to measurements that gives its drag coefficient:

- **`Filippone`** (the default), on `S`, from wind-tunnel tests of cotton streamers **clamped at
  the leading edge** at `AR` 3.3 to 30 and 6 to 18.9 m/s. The paper fits one power curve per
  planform area, and prints all three:

  | planform area | curve | where |
  |---|---|---|
  | 0.025 m² | `C_D = 0.561 AR^−0.480` | eq. 2 (Figure 2's trend reads `0.561 AR^−0.4795`) |
  | 0.05 m² | `C_D = 0.6514 AR^−0.6075` | the trend line on Figure 3; the text does not repeat it |
  | 0.075 m² | `C_D = 0.405 AR^−0.494` | eq. 1 (Figure 4 reads `0.4046 AR^−0.494`) |

  **hpr** interpolates between *neighbouring* curves, linearly in the logarithm of the area
  (`ln S`), and holds the end curve outside the fitted areas; that is hpr's choice, not the
  paper's. All three curves are needed, because `C_D` is far from linear in `ln S`. At
  `AR = 3.3`, the middle area's `C_D` (0.3154) sits 0.3% *below* the smallest area's (0.3163),
  not partway down to the largest's (0.2245); a straight line in `ln S` would put it 63% of the
  way there. Blending only the two end curves would read 18% low there.

  Three of the paper's own findings bear on how to read it, and none is in its curves:

  - a **free** leading edge gives more drag than the clamped mounting these curves come from;
  - lighter, smoother fabric gives less (polyester at 64 g/m² against cotton at 177);
  - at the largest area it finds a [drag crisis](../glossary.md#drag-crisis), a sudden change in
    the drag coefficient over a narrow range of [Reynolds number](../glossary.md#reynolds-number),
    near `Re = 7.2e5`.

  A rocket's streamer is free and light, so the first two pull in opposite directions.
- **`OpenRocket`**: `C_Dm = 0.034 ((ρ_m + 25 g/m²)/(105 g/m²)) ((l + 1 m)/l)` on `S`, with `ρ_m`
  the fabric's mass per square metre (Appendix C, eq. C.6, printed page 117). It was fitted to
  model-rocket streamers (`w` 0.01 to 0.09 m, `l` 0.2 to 1.0 m, 10 to 80 g/m², 6 to 12 m/s), with a
  stated 12 to 27% error on an independent set. It is the only one of the two that uses the
  material.

### Which model is right?

The two disagree by a factor that depends on the fabric: at `AR = 10` and `l = 1.016 m` the ratio
of their drag areas is 5.8 at 10 g/m², 3.5 at 32 and 1.9 at 80.

C. Kidwell's NARAM-43 drop tests (2001) settle it as far as one dataset can: sixteen 4 in × 40 in
streamers, each with about 5 g at one corner, dropped 20.1 m.

Two details of his method decide how to compare, and both were got wrong here before review:

- His rates are **distance over time**, so they are averages over the drop rather than terminal
  speeds. A 20.1 m drop averages 0.97 of terminal at 2.8 m/s but 0.89 at 5.9 m/s.
- They are **normalised** to a notional 5.000 g weight: scaled to what that standard weight would
  give.

`streamer_models_against_kidwells_drop_tests` uses the notional mass and compares the same
average, from the [closed-form](../glossary.md#closed-form) fall:

| case | measured | `C_D` it implies, on `S` | `Filippone` | `OpenRocket` |
|---|---|---|---|---|
| crêpe paper, 32 g/m², **unpleated** | 2.80 m/s | 0.155 | 3.05 m/s (+9%) | 5.28 m/s (+88%) |
| Micafilm, 42 g/m², pleated | 2.04 m/s | 0.338 | 3.21 m/s (+58%) | 5.19 m/s (+154%) |

What the table shows:

- **The flat streamer.** Kidwell's crêpe streamer is the one he left flat, and a flat correlation
  predicts it to 9%. Appendix C reads 88% fast.
- **The pleated ones.** His other materials were folded in ¾ in pleats, which more than doubles the
  drag again. **hpr models no pleats**, so it predicts a faster descent for a pleated streamer,
  which is the safe direction for a landing.
- **Both are a little outside both fits:** 0.1032 m² of planform against Filippone's largest
  0.075 m², and 0.1016 m wide by 1.016 m long against appendix C's `w ≤ 0.09`, `l ≤ 1.0`.

### Streamers larger than the fits

Above 0.075 m², hpr's **clamp** holds the largest area's curve. Kidwell's streamers are above it, so
his drops test the clamp, and the evidence pulls two ways:

- **The paper's trend says the clamp reads high.** Its `C_D` falls as the area grows: the two end
  curves at `AR = 10` imply `C_D ∝ S^−0.326`, which extrapolated to Kidwell's 0.1032 m² would give
  `C_D = 0.117` where the clamp gives 0.130. The steeper inner pair, `S^−0.53`, would give 0.110.
- **The drop says it reads low.** The drop itself implies 0.155, higher than any of them.
- **What hpr does.** It holds the end curve rather than extrapolating, because that is the closer of
  the two to the one free-drop measurement in hand. It says so here rather than claiming the trend.
- **Two caveats.** That rests on a single point, at one aspect ratio, with a fabric nothing like
  the paper's cotton. And it holds partly because the correlation, measured with the leading edge
  clamped, is itself biased low, so two errors cancel.
- **Nothing is measured** anywhere near 0.225 m², the 1.5 m by 0.15 m streamer one of hpr's tests
  flies on the clamp.

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
- The documentation notes 0.56 is half a circular cylinder's 1.12 in crossflow (air flowing across
  it, side-on), as expected of a cylinder falling at a random angle, and that 1.42 is "similar to
  that of a flat plate 1.17 or an open hemispherical cup 1.42". Those come from Hoerner's
  *Fluid-Dynamic Drag* (1965), which is copyrighted with no legal free copy; NASA TN D-540 and TR
  R-474 carry the same numbers and are free (`docs/research/streamer-and-tumble-drag.md`).
- The body's profile uses each component's **end** diameters, so a curved nose is under-counted:
  for Valetudo's tangent ogive the true side area (the diameter integrated along the length,
  `∫d dx`) is 0.0148 m² against the 0.0111 m² hpr takes, 25% low on the nose and 2.2% low on the
  whole body (1.1% high in terminal speed).

**How well it does.** The documentation says its fit predicts its own five drop-test models within
3 to 14%. hpr does not reproduce that. Replaying Table 3.3 (printed page 54: five models, 22 m,
ρ = 1.31 kg/m³, the descent rate `v₀` read from video to ±0.3 m/s) through hpr's reading of the
model (`the_tumble_model_against_its_own_drop_tests`):

| model | fins | measured | hpr |
|---|---|---|---|
| #1 | 3 | 5.6 m/s | 5.27 (−5.8%) |
| #2 | 4 | 6.3 m/s | 5.96 (−5.4%) |
| #3 | 3 | 6.6 m/s | 6.13 (−7.2%) |
| #4 | none | 5.4 m/s | 6.43 (**+19.0%**) |
| #5 | fins only | 5.0 m/s | 4.50 (−10.0%) |

So the spread is −10 to +19%, and the finless tube is the outlier: it wants a body coefficient
near 0.79 where the model prints 0.56.

The text pins neither the body-profile nor the fin-area convention (exactly which areas to
measure). So either hpr reads the areas differently from whoever fitted the constants, or the claim
is not reproducible. The table above is what hpr can demonstrate, so it is what hpr states.

**Where it stops being true.** The fit covers 44 to 103 mm bodies of 6.8 to 160 g descending at
5.0 to 6.6 m/s. A high-power booster is far outside it: Valetudo tumbling comes out at 37 m/s.

- A cylinder's crossflow drag falls by roughly half above a
  [Reynolds number](../glossary.md#reynolds-number) near 2e5, its
  [drag crisis](../glossary.md#drag-crisis). A 100 mm body reaches that at about 30 m/s, and
  Valetudo's 80 mm at 37 m/s is right at it.
- So a real body that size would descend faster than hpr says.
- hpr does not model that fall, and nothing in the pinned sources covers it.

A tumbling body is a device like any other: give it a trigger, and it starts at that moment.
hpr does not decide by itself when a rocket tumbles — nothing citable says when a stage becomes
unstable enough (the same documentation declines to model the analogous streamer regime).

## Triggers, lag and release

A device's charge fires at its `Trigger`:

- `Apogee`: when the centre of mass is descending. hpr's apogee [event](../glossary.md#event)
  fires it the instant the height rate crosses zero.
  - A flight that *starts* in mid-air, already past its apogee, has no crossing to find, so the
    charge fires at its first step. RocketPy's own apogee trigger does the same: it fires whenever
    the vertical velocity is negative (`parachute.py:368-376`).
  - Such a flight starts from
    [`Simulation::run_free`](../api/hpr_sim/flight/struct.Simulation.html#method.run_free), as the
    comparison with RocketPy [below](#against-rocketpy) does, and as staging and flight-data replay
    will.
  - The charge fires once, at whichever of the two comes first.
- `Altitude { height_above_ground_m }`: the first time the centre of mass is **descending** and at
  or below that height above the launch site. This is an altimeter's main setting. A rocket whose
  apogee is already below the setting fires at apogee, because no crossing follows. That is
  RocketPy's numeric trigger: vertical velocity negative and height below the setting
  (`parachute.py:354-364`).
- `Time { time_s }`: a time after the first ignition.
- `MotorDelay { motor }`: that motor's [ejection delay](../glossary.md#ejection-delay) after its
  own burnout. The motor must have a delay in seconds; a plugged motor or one with no delay set is
  refused.

Charges are only checked in free flight and during the descent, so a `Time` or `MotorDelay`
trigger whose time passes while the rocket is still on the pad or the rail fires at
[rail exit](../glossary.md#rail-exit-and-rail-exit-velocity).

`lag_s` seconds after the trigger the device **deploys** (line stretch, where the lines pull taut)
and starts to fill. The first deployment of a flight switches it to the descent phase.

A device can name another whose opening **releases** it (`released_by`), which is how a drogue is
cut away under a main:

- **The release waits until the releasing device is fully open**, not its line stretch. Cutting the
  drogue at line stretch would leave the rocket under an empty canopy: the drag area would collapse
  and the descent speed up (found in review; measured at 0.45 m² → 0.02 m² and 18.3 → 22.0 m/s
  before the fix).
- **A released device contributes nothing** from its release.
- **One released before its own charge fires never deploys at all:** its `Trigger` is recorded and
  no `Deployment` follows.

Events, in the order a two-device flight records them: `Apogee`, `Trigger(drogue)`,
`Deployment(drogue)`, `Trigger(main)`, `Deployment(main)`, `Release(drogue)` (at the end of the
main's filling), `GroundHit`.

Trigger times that are known before the flight (a time, or a motor delay) and every deployment and
end of filling are [stop times](../glossary.md#stop-time), so no integration step straddles a change
in the drag area.

## Inflation

A canopy doesn't reach its full drag area at line stretch: it grows over a filling time. Each
device that has deployed and is not released contributes

```text
(C_D S)(t) = (C_D S)₀ · min(1, (t − t_d)/t_f)^j
```

with `(C_D S)₀` its full drag area, `t_d` its deployment, `t_f` its filling time and `j` its growth
exponent. The open devices' drag area at time `t` is the sum of these.

The exponent sets the shape of the growth:

- `j = 1` is linear growth (Knacke's ribbon and ringslot canopies);
- `j = 2` is the growth of solid cloth (Pflanz, Figure 5-51), `(t/t_f)²`: slow at first, fast at
  the end.

`Inflation` chooses `t_f`:

- `Instant` (the default): `t_f = 0`, the full drag area at line stretch. This is RocketPy's model.
  For a deployment well above the canopy's terminal speed it gives hpr's highest opening load; near
  terminal speed, as at apogee, a filling time can give a higher one, because the rocket speeds up
  while the canopy fills.
- `FillingTime { time_s, exponent }`: a filling time fixed in advance.
- `FillConstant { constant, exponent }`: Knacke's `t_f = n D₀/v` (printed page 5-43), with `v` the
  airspeed at line stretch and `n` the canopy fill constant, from Table 5-6's **unreefed** column
  (printed page 5-44): 8 for a flat circular canopy. A deployment at rest has no filling time in
  this law (`n D₀/v` diverges), so the canopy is taken as open at once and fills as the rocket
  picks up speed.

### The fill constant at hobby speeds

Knacke states the linear form only "in the medium-velocity range of about 150 to 500 ft/s"
(45.7 to 152.4 m/s; `Inflation::FILL_CONSTANT_RANGE_M_S`). A hobby main is slower:

- A main opening at 20 to 30 m/s is **below** that range.
- There, his alternative for solid flat circular canopies is `t_f = n D₀/v^0.85` with `n = 4.0`.
  It is a dimensional form (feet and ft/s) that cannot be used in SI as printed, so hpr does not.
- So outside the range the filling time, and with it the peak load hpr reports, is an
  extrapolation. At 25 m/s the linear form gives a 2.5 m main `t_f = 0.8 s`, from a correlation
  fitted at three to six times that speed.

### The opening load

hpr's drag area rises to the steady value and stays there. Knacke writes the opening force as
`F = (C_D S) q C_x X1` (printed page 5-50), with `q` the
[dynamic pressure](../glossary.md#dynamic-pressure) at line stretch. Its two factors fare
differently in hpr:

- **The overshoot, `C_x`, is left out.** Knacke's measured drag area **overshoots** the steady
  value by 10 to 80% near the end of filling (Figure 5-40, printed page 5-47). His infinite-mass
  opening-force coefficient is `C_x = 1.7` for a flat circular canopy; hpr's is 1 in every mode.
- **The slowing, `X1`, depends on the mode.** `X1` allows for the rocket slowing while the canopy
  fills. It is 1 at infinite mass (a load too heavy to slow), and as low as 0.02 for a
  final-descent parachute with a low canopy loading (little weight for the canopy's size).
  - **With a filling time** (`FillingTime` or `FillConstant`), hpr already includes the slowing:
    it integrates the rocket's deceleration while the drag area grows, which is Pflanz's method
    done step by step, without the overshoot. Don't apply `X1` on top of hpr's peak, or the
    slowing is counted twice and the load reads low.
  - **Opening at once** (`Instant`, the default), there is no slowing: the peak is Knacke's
    infinite-mass case with `C_x = 1`.

So the peak load hpr reports is **no safe bound** on the real one, in either direction:

- **With a filling time,** the missing overshoot alone can only raise the real peak, but the growth
  law and the filling time, extrapolated at hobby speeds, can move it either way.
- **Opening at once,** hpr applies the full drag area at line stretch: the infinite-mass case. A
  big main on a light rocket can therefore see far less than hpr's instant peak.
- **For scale,** the 1.5 m flat circular canopy deployed at 60 m/s in
  [Verification](#verification) peaks at 1.6 kN filling and 3.0 kN opening instantly. Knacke's
  infinite-mass `C_x = 1.7` on the same dynamic pressure would be 5.1 kN.

**Don't size recovery hardware from hpr's opening load.** Use a dedicated opening-load method and
the hardware's own ratings. Ludtke's law and Pflanz's `X1` reduction factor are candidates for a
later [milestone](../glossary.md#milestone).

## The descent

Once a device is open the rocket is a point mass: all of its mass at the centre of mass, with no
rotation. In the [launch frame](../glossary.md#launch-frame-enu), with `m` the mass, `v_cg` and
`a_cg` the centre of mass's velocity and acceleration, `w` the wind, `ρ` the density at its height,
`g` [normal gravity](../glossary.md#normal-gravity), `a_Coriolis` the
[Coriolis acceleration](../glossary.md#coriolis-acceleration) and `T` the thrust:

```text
m a_cg = −½ ρ (C_D S)(t) |v_cg − w| (v_cg − w) + m (g + a_Coriolis) + T
```

- **In code it is the free-flight equation.** hpr uses the translational equation of
  [Rigid-body flight](flight.md#equations-of-motion) with the body's rotation rate `ω = 0` and the
  canopy drag in place of the airframe's aerodynamics. Two things carry over:
  - The terms for a motor that is still burning: the centre of mass moving inside the body as
    propellant burns (`−m r″ − 2ṁ r′`) and the exhaust jet's terms. That page sums them, with the
    forces, into `T20`. After [burnout](../glossary.md#burnout) every one of them is zero.
  - The point the integrator follows is still the nose tip `O`, not the centre of mass. With no
    rotation and nothing burning the two move together, so the nose tip's acceleration `a_O`
    equals `a_cg` and the equation is the one above.
- **No moment.** The drag acts at the centre of mass along the air's relative velocity, so it exerts
  no turning moment.
- **The attitude freezes at deployment.** The body rates are set to zero, and the nose tip keeps its
  rigid offset from the centre of mass, `r_cg` (the centre of mass's position from the nose tip, in
  body axes). As the rates go, the nose tip's velocity is shifted by `ω × r_cg` (turned from body
  axes into the launch frame), so that the centre of mass keeps the velocity it had. Whatever stops the rotation acts through the lines and the canopy, inside the
  system, so it can't change the centre of mass's momentum.
- **The airframe's own drag is left out**, as RocketPy leaves it out. A rocket's attitude under a
  canopy, and so the area it presents, is not modelled. For a drogue whose drag area is close to
  the airframe's broadside area this is a real omission. It is the same omission RocketPy, the
  [oracle](../glossary.md#oracle) this is compared with, makes. The tumble model
  ([M1.7b](../decisions-and-roadmap.md#m1-7b), streamers and tumble) is where a body's own drag
  belongs.
- **The thrust `T` is kept**, along the frozen axis, so a device that opens while a motor still
  burns (an off-nominal case) is not silently thrust-free. Its direction is wrong the moment the
  rocket would have swung under the canopy.
- **The rest is shared.** Gravity, the Coriolis force, the atmosphere and the wind are the same
  models the rest of the flight uses ([Rigid-body flight](flight.md)).
- **[Added mass](../glossary.md#added-mass) is not modelled.**
  - Knacke gives no closed-form apparent mass (printed page 5-40 says only that it is the enclosed
    volume times density times a form factor), and RocketPy's `m_a = k_a ρ (2/3) π R² H` has no
    citation in its code.
  - It carries no weight in RocketPy either, so it changes no equilibrium descent rate, only the
    transient right after an opening.
  - It is not small: the fixture records 5.6 kg for Calisto's main against a 16.2 kg rocket and
    15.9 kg for NDRT's against a 20.8 kg one (both at the start height; both grow with density as
    the rocket descends). The comparison below shows what leaving it out costs.

The equilibrium descent speed is Knacke's (printed page 5-128), and `recovery::terminal_speed_m_s`
computes it:

```text
v_e = √(2 m g / (ρ C_D S))
```

## Separation

A `Separation` is a trigger and a stage boundary. At the trigger the stack comes apart into two
bodies:

- **body 0** keeps the nose: the stages from stage 0, at the nose, through the one `after_stage`
  names;
- **body 1** is the stages aft of the split.

Each body flies on as a point mass under the devices that name it (`Device::on_body`). The ascent
ends there: its `FlightResult` has `Termination::Separated`, a `Separation` event, and one
`BodyFlight` per body in `bodies`. The exception is a powered separation, where body 0 still has a
motor to burn: it flies on as a sustainer, and only body 1 descends here
([Staging](staging.md#powered-separation)).

This section describes the unpowered case, and the booster's descent after a powered one.

**What happens at a separation:**

- **Each body is its own stages and their motors.** `body_mass_properties` sums the stages'
  `MassProperties` and the motors mounted in them, so the bodies' masses add to the whole rocket's
  at that instant — which is a test.
- **Nothing pushes the bodies apart.** Each body starts at its **own** centre of mass, with the
  velocity that point already had: the nose tip's velocity plus the rotation's share,
  `v_O + ω × r_cg`, in the launch frame. So the bodies' **linear** momenta add to the stack's,
  which is a test.
- **Their spin is dropped.** Each body's centre keeps moving round the stack's as it was, but each
  body's spin about its own centre is lost, so angular momentum is not conserved. In the test,
  spinning at 0.6 rad/s, the lost spin is 31% of the angular momentum, and 0.02 J of energy.
- **Only body 0's devices act before the separation.** A device meant for another body has a drag
  area computed for that body — a booster's tumbling area, say — which is not a model of the whole
  stack, so it waits for its body. With no separation every device is body 0's.
- **Each body finds its own apogee**, whatever its devices are triggered by. The ascent ends at
  the separation, so this is the only place a staged flight can record a peak, and without it a
  body separated while climbing would never fire an apogee charge (found in review, now a test).
- **The bodies descend independently**, each with the same point-mass equations as the descent
  phase, less the thrust: `m a = −½ ρ (C_D S)(t) |v − w| (v − w) + m (g + a_Coriolis)`. They share
  the flight's devices and their progress, so a canopy that opened before the separation stays open
  on whichever body carries it.
- Bodies are not watched by the `Observer`: their events and samples are in their `BodyFlight`.

**Limits:**

- **No ejection charge, spring or [tip-off](../glossary.md#tip-off).** An ejection charge's
  impulse, and the tumbling that follows, are not modelled.
- **A body must start above the ground**, as a free flight must: the ground event is a falling
  crossing, so a body that started below the site would go on integrating underground until the
  flight's time limit.
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
- **The aft body's motors must have burned out**, because a body's mass is held constant through
  its descent. A trigger that fires while one burns is an error, before the flight when its time is
  known and in flight otherwise, not a silent approximation. A release across the separation is
  refused too: a line cuts a device on its own body. A forward body with a motor still to burn
  flies on as a sustainer ([Staging](staging.md)).

## Verification

`crates/hpr-sim/src/recovery.rs`'s tests, all analytic (checked against exact answers) unless they
name the [oracle](../glossary.md#oracle):

| What | Result |
|---|---|
| Knacke's `v_e` against a case from Loft, the project before hpr-sim (1.1 kg, 1 m flat canopy, `C_D` 0.8, ρ 1.225) | 5.294 m/s, as Loft printed |
| A descent from rest against the closed-form fall under quadratic drag (2 km, uniform air, constant gravity) | 2.1e-8 of the terminal speed `v_t` over the whole descent; the landing time within 1e-5 s of the closed form's 204 s |
| Drift in a steady wind, entered drifting with the air, no Earth rotation | exactly the wind times the time of flight (1e-8); the fall itself within 1e-4 of the closed form |
| The Coriolis drift of a 3 km descent, Earth rotation on | 0.3666 m east against the steady prediction `2Ω cos φ · v_t²/g · T` = 0.3685 m, 0.52% apart, and 29 µm north |
| Knacke's filling law, `t_f = n D₀/v` and `(t/t_f)^j` | the recorded drag area to 1e-9 of `(C_D S)₀` |
| Inflation against instant opening (deployed at 60 m/s under a 1.5 m flat circular canopy) | peak load 1,615 N against 3,020 N instant (0.53 of it), between the closed-form 1,527 N without gravity and 1,670 N with it |
| An oversized canopy (5 m) opening at 100 m/s, 10 km of descent at 2.95 m/s | lands in 3,392 s in 6,914 accepted steps and 2 rejected (a mean step of 0.49 s, where Loft's explicit [RK4](../glossary.md#dormandprince-and-rk4) needed a 2e-4 s floor) |
| A deployment with a 0.7 rad/s body rate, and another inside the burn in a crosswind | the centre of mass keeps its velocity across the handover to 1e-12, and the nose tip's moves by exactly `ω × r_cg`, turned into the launch frame |
| A whole flight: drogue at apogee with a lag, main at 300 m, drogue released | events in order; each stage settles within 2% of its own `v_e` |
| Two devices triggered at the same instant | both open in the same pass, and the descent settles at the `v_e` of the **sum** of their drag areas |
| A device released before its own charge fires | it is recorded as triggered and never deploys; the descent stays at the open device's `v_e` |
| A drogue released by a main that fills over 2 s | the release waits for the end of filling, the drag area never falls below the drogue's, and the descent never speeds up |
| An apogee charge on a flight that starts descending | it fires at the first step (there is no apogee event to find), and a climbing start still waits for the apogee |
| Two user events and an altitude device on one flight | the user events keep their numbers and fire during the descent, in height order |
| The same recovered flight flown twice | bit-identical rows, events, final sample and step counts ([Loft lesson L24](../decisions-and-roadmap.md#l24): a run does not mutate the simulation) |
| A separation at apogee of the two-stage test design, canopy on the sustainer and tumble on the booster | both bodies land: the 0.550 kg sustainer at 729.0 s and 2.11 m/s under its 1.8 m canopy, the 1.125 kg booster at 107.5 s and 16.74 m/s tumbling; the masses add to the 1.675 kg stack to 1e-12 and each lands within 0.1% of its own `v_e` |
| The linear momenta of the bodies at a separation with a 0.6 rad/s body rate | add to the stack's to 1e-9, and each body starts at its own centre of mass to 1e-12 (0.817 m apart on this design) |
| A separation while the booster's motor burns | refused in flight, with the booster's burnout time in the error |
| A separation while still climbing at 100 m/s | both bodies find their own apogee above 1,400 m, fire there, and land within 1% of their own `v_e` |
| A timed separation, and a height separation | fire at their own time to 1e-9 s and at their own height to 1e-6 m, rather than at the next boundary that happens to exist (found in review: one fired 186 s late, another never) |
| A body that runs out of time | says `TimeCap` in its own `BodyFlight`; `FlightResult::bodies_landed` is false and `landings()` is short |
| A body whose device never fires (a timer set after it lands) | refused in flight, naming the body, rather than landed at 170 m/s |
| A body whose canopy opened on the stack just before the separation (an apogee separation with an apogee parachute) | lands: the open canopy counts, though its deployment is in the flight's events, not the body's (found with [M1.9a](../decisions-and-roadmap.md#m1-9a)) |
| A timed separation known to precede the booster's burnout | refused when the separation is given; a height one that a climbing rocket passes early is refused in flight |

### Against RocketPy

hpr's descent is compared with RocketPy's for five of RocketPy's example rockets, a
[code-to-code comparison](../glossary.md#code-to-code-comparison). **Every compared number agrees
within 3%, and most within 0.1%** (22 of the 30 in the [validation report][report]). The largest
gaps, in order:

- **NDRT 2020's north drift: +2.86%.** hpr carries it 50.8 m south, 2.86% further than RocketPy
  does. NDRT's main has a drag area of 16 m², and RocketPy's
  [added mass](../glossary.md#added-mass), which hpr leaves out, is the likely cause; no test has
  isolated it yet.
- **Valetudo's north drift: −1.77%.** That drift is 19 µm, from the Earth's rotation alone, so a
  tiny difference is a large fraction ([its own section](#valetudos-north-drift) below).
- **Valetudo's whole drift: −0.89%**, of 0.19 m, also from the Earth's rotation alone.
- **NDRT 2020's descent time: +0.71%** longer in hpr, likely the same added mass.

**How the comparison is run:**

- [`validation/oracles/rocketpy/recovery.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/rocketpy/recovery.py)
  flies RocketPy's own parachute phase for the five rockets and writes what it computes to
  [`validation/fixtures/recovery/rocketpy-descent.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/recovery/rocketpy-descent.json).
  The test `descent_matches_rocketpy_examples` replays each case in hpr.
- Both codes start from the same declared state after burnout, near apogee. The first device opens
  at once: its lag is overridden to zero, so no ballistic stretch, flown under each code's own
  rocket aerodynamics, comes between them.
- Both get the same `C_D S`, the same deployment settings and the same wind. RocketPy's random
  noise on each parachute is set to zero, and hpr flies RocketPy's formula for gravity (see
  *Gravity* below).
- The test checks that the two environments agree first, then compares the descents.
- The oracle runs at `rtol = atol = 1e-8`: its relative and absolute
  [tolerances](../glossary.md#tolerance), how much error each step may make, both 1e-8
  ([scientific notation](../glossary.md#scientific-notation) for 0.00000001). Run again at 1e-6,
  it moves every compared metric by at most 3.5e-6 relative (the fixture's
  `solver.relative_change_from_loose`), far below the gaps. The one larger entry, 2.1e-3, is
  Valetudo's north drift of 19 µm, which has [its own section](#valetudos-north-drift) below.

**The results.** Measured (hpr against RocketPy, 2026-09-17):

| case | descent time | descent rate under the drogue | impact descent rate | drift | worst drift component |
|---|---|---|---|---|---|
| Calisto (drogue 1.0 m², main 10 m² at 800 m, wind 5 E / 2 N) | +0.08% (257.27 s) | −0.01% (17.967 m/s) | −0.03% (5.454 m/s) | +0.08% (1,386.0 m) | +0.08% |
| Valetudo (drogue 0.4537 m², no wind) | −0.02% (45.76 s) | — | +0.00% (17.627 m/s) | −0.89% (0.19 m, Coriolis only) | −1.77% (north, 19 µm; +2704% under hpr's own gravity, [issue #27](https://github.com/nrdptel/hpr-sim/issues/27)) |
| NDRT 2020 (drogue 0.438 m², main 16.05 m² at 167.6 m, sheared wind) | +0.71% (61.60 s) | +0.01% (28.156 m/s) | +0.01% (4.604 m/s) | +0.28% (327.9 m) | +2.86% (north, −50.8 m) |
| Prometheus 2022 (drogue 0.467 m², main 5.78 m² at 457.2 m) | +0.08% (153.50 s) | −0.01% (26.400 m/s) | −0.03% (7.323 m/s) | +0.08% (1,237.1 m) | +0.09% |
| Juno III (drogue 0.885 m²) | −0.02% (53.56 s) | — | −0.01% (22.431 m/s) | −0.02% (457.9 m) | −0.02% |

- **Every metric is inside the 3%** that the parachute milestone
  ([M1.7a](../decisions-and-roadmap.md#m1-7a)) set. The descent rate under the drogue, where a
  case has a main, agrees to 0.01%.
- **The later devices' trigger heights** agree to −0.01%, −0.17% and −0.01%. RocketPy's trigger
  sampling (below) accounts for them.
- **Both simulators land within 1% of Knacke's `v_e`** for the device that is open, computed from
  hpr's own air and gravity at the site.
- **These numbers use RocketPy's gravity and wind interpolation, not hpr's defaults.**
  - Gravity: the comparison has flown RocketPy's gravity model since
    [issue #27](https://github.com/nrdptel/hpr-sim/issues/27). Under hpr's own gravity the drifting
    cases read a little closer (Calisto +0.06% rather than +0.08%, measured once when the
    comparison switched and not pinned by a test), because the vertical's turn
    downrange pushes the rocket back toward the pad and cancels part of a real difference. The
    like-for-like number is the honest one.
  - Wind: here both codes interpolate the wind by its east and north components, as RocketPy does.
    For a table of wind levels, hpr's default is to interpolate speed and direction instead
    ([Wind](wind.md)). Only NDRT 2020's wind changes with height; the other four cases have one
    wind at every height, where the two ways agree. What hpr's default would do to NDRT's drift
    has not been measured.

**What still differs between the two codes:**

- **Added mass.** hpr has none. RocketPy's carries no weight, so it changes no steady descent rate,
  only the response just after an opening. It is most likely the largest difference.
  - RocketPy's added mass for NDRT's main is 15.9 kg, against the rocket's 20.8 kg, so its response
    to the opening is slower.
  - That most likely lengthens the descent (+0.71%) and, in a wind that shears with height, moves
    the smaller drift component by 2.86%.
  - A cited apparent-mass model would show whether it closes that gap.
- **When a trigger fires.** RocketPy checks its triggers on a grid of `1/sampling_rate` (100 or
  105 Hz), anchored at `t = 0`, and only over the span after its first accepted step. hpr has no
  sampling rate: its [event](../glossary.md#event) finder locates the crossing.
  - So RocketPy's first deployment is 2.5 ms late in the four 105 Hz cases, and 13 ms late in
    Prometheus's.
  - Its test, height below the setting (`h < setting`), can fire only at or **below** the setting,
    by at most one sample of fall, the descent speed over the sampling rate (`v_z/rate`): 0.17 m
    for Calisto and 0.27 m for NDRT (about 0.01 s of descent).
  - The heights the fixture records at those triggers (800.07 m, 167.93 m, 457.26 m) come from
    the spline RocketPy fits through its stored samples for *reporting*, not from the continuous
    solution between steps that its trigger read, so they sit just above the setting instead.
  - The table compares hpr's trigger heights with those reported values, the closest the fixture
    can come. The difference is the same size either way.
- **Release against replacement.** hpr sums its open devices, and releases the drogue when the main
  is full; RocketPy holds one `C_D S` and replaces it. For these cases, whose canopies open
  instantly, the two are the same.
- **Wind: no difference.** Both codes interpolate the declared wind by its east and north
  components, and the test holds hpr's to RocketPy's samples within 1e-9 m/s in each, NDRT's
  sheared profile included.
- **Atmosphere.** hpr evaluates the 1976 [standard atmosphere](../glossary.md#standard-atmosphere);
  RocketPy interpolates a 100-point pressure table over 0 to 80 km. Over the fixture's 23 samples
  they differ by at most 3.7e-4 in density, which the test gates at 5e-4.
- **Gravity: the same size, a different direction.** RocketPy's "Somigliana" formula is
  [WGS 84](../glossary.md#wgs-84) [normal gravity](../glossary.md#normal-gravity), and hpr's agrees
  with the fixture's samples to 1e-8 (the worst of 23 is 4.7e-9 relative). The two point it
  differently, and for a long time this page compared only the size.
  - RocketPy applies gravity to the vertical axis alone (`Flight.u_dot_parachute`, `flight.py:2777`,
    where only `az` carries a gravity term).
  - hpr's default, `GravityModel::Ellipsoidal`, uses the full normal-gravity **vector**. Above the
    ellipsoid it tilts slightly toward the equator ([Gravity](gravity.md)), in proportion to height
    above the ellipsoid: 4.0e-6 m/s² sideways at Valetudo's site at ground level, and 8.7e-6 m/s²
    at 1,468 m, where Valetudo's descent starts.
  - Over the fixture's 23 gravity samples the tilt runs from +6.9e-6 m/s² (north) at Valetudo's
    top sample, 1,168 m, to −3.3e-5 m/s² (south) at Calisto's 4,400 m.
  - hpr's vector also turns with the local vertical downrange, by `g·d/R`, with `d` the distance
    drifted and `R` the Earth's radius: 2.1e-3 m/s² at Calisto's 1.4 km of drift. Wherever a rocket
    drifts at all, that is much the larger of the two.
  - The parachute milestone's test ([M1.7a](../decisions-and-roadmap.md#m1-7a)) used to compare
    gravity by its size alone, so it could see neither. This comparison and the validation suite
    now both fly `GravityModel::VerticalTaylor`, which hpr ships as RocketPy's own formula for
    like-for-like comparisons, and both check the gravity **vector**, not its length.
- **Geometry.** hpr flies over the curved ellipsoid and measures heights along its perpendicular
  ([ellipsoidal height](../glossary.md#ellipsoidal-height)); RocketPy's height `z` is measured in a
  flat frame. Over Calisto's 1.4 km of drift the curvature is 0.15 m of height, 0.03 s of descent.

#### Valetudo's north drift

This tiny number is worth its own section, because it is the one that found the gravity difference
above.

- **What to expect.** In still air, Valetudo's north drift comes from the
  [Coriolis acceleration](../glossary.md#coriolis-acceleration) alone. The falling rocket picks up a
  small eastward velocity `v_east` from it, and the same acceleration acting on that eastward
  motion pushes it slightly north (Valetudo's site is in the southern hemisphere). The horizontal
  velocity relaxes to a drag balance in about `v_t/g` ≈ 1.8 s, with `v_t` the terminal speed, so
  `v_north ≈ −2 ω_z v_east · v_t/g`, with `ω_z` the vertical part of the Earth's rotation. That
  integrates to 2.0e-5 m over the descent. RocketPy gives 1.9653e-5 m.
- **What hpr gave at first.** The parachute milestone ([M1.7a](../decisions-and-roadmap.md#m1-7a))
  didn't compare this component; the validation harness
  ([M2.1a](../decisions-and-roadmap.md#m2-1a)) does. When it first did, hpr read 28 times
  RocketPy's: 5.51e-4 m (0.55 mm), under hpr's default gravity. The tilt of that gravity,
  integrated down the 800 m of descent, `(1/g)∫₀^800 g_north dz` = 5.2e-4 m, accounts for the
  difference to within a few percent ([issue #27](https://github.com/nrdptel/hpr-sim/issues/27)).
- **What it gives now.** Flown against RocketPy's own gravity formula, as the validation suite
  does, hpr gives 1.93e-5 m, −1.8% ([validation report][report]). Both codes carry the same
  Coriolis term (hpr in `dynamics.rs`; RocketPy in `flight.py:2779-2783`), and on this evidence
  neither is wrong: they were being asked different questions.

[adr-012]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-012-recovery-drag-areas-triggers-inflation-and-the-descent-phase-2026-09-17
[adr-013]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-013-streamer-and-tumble-drag-2026-09-17
[adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
