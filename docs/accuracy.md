# Accuracy

This page gathers every check hpr-sim has passed so far, and every known gap, in words and numbers.
Start with the bottom line: **no whole flight has been validated yet.** hpr's apogee, top speed and
landing point have not been compared with another simulator's or with a real flight's. What has been
checked is each model on its own, against its published source and in places against RocketPy, and
the descent under a parachute, against RocketPy, for five rockets.

Every number here links to the model page or the report it comes from, and the site's build fails if
a number here no longer matches its source. [Checking a claim](checking-a-claim.md) shows how to
follow one back to its source and its test.

## Four kinds of evidence

A model can be checked in four ways, from the weakest to the strongest evidence that it matches
reality. They are set out in the project's [validation plan][plan].

| kind | what is compared | what agreement shows |
|---|---|---|
| **Analytic** | The code against exact answers: closed-form solutions, conservation laws, round trips | The code computes what its equations say |
| **Published source** | The code against a source's printed tables and worked examples | The code implements the source correctly |
| **Another code** | hpr against another simulator, such as RocketPy, flying the same inputs | The two codes agree on the physics; not that either matches reality |
| **Real flights** | hpr against measured flights | The model matches reality, within the flight's own uncertainty |

The first three check the code. Only the fourth checks the physics against the world, and none has
been done yet: it is planned for [M2.3][roadmap], the real-flights milestone.

## Where each model stands

A tick means the model has been checked that way; a dash means it hasn't yet. "Indirect" means the
model was exercised by another comparison without being compared on its own.

| model | analytic | published source | another code | real flights |
|---|---|---|---|---|
| [Frames](physics/frames.md) | ✓ | — | ✓ RocketPy | — |
| [Geodesy](physics/geodesy.md) | ✓ | ✓ | — | — |
| [Gravity](physics/gravity.md) | ✓ | ✓ | ✓ RocketPy | — |
| [Atmosphere](physics/atmosphere.md) | ✓ | ✓ | indirect | — |
| [Wind](physics/wind.md) | ✓ | — | indirect | — |
| [Turbulence](physics/turbulence.md) | ✓ | — | — | — |
| [Design tree](physics/design.md) | ✓ | — | ✓ RocketPy | — |
| [Shapes](physics/shapes.md) | ✓ | — | — | — |
| [Mass properties](physics/mass.md) | ✓ | — | — | — |
| [Solid motors](physics/motor.md) | ✓ | — | ✓ RocketPy, ThrustCurve.org | — |
| [Aerodynamics](physics/aero.md) | ✓ | ✓ | ✓ RASAero curves | — |
| [Rigid-body flight](physics/flight.md) | ✓ | — | — | — |
| [Time integration](physics/integration.md) | ✓ | — | — | — |
| [Recovery](physics/recovery.md) | ✓ | ✓ | ✓ RocketPy | — |
| [Interpolation](physics/interpolation.md) | ✓ | — | — | — |
| [Quadrature](physics/quadrature.md) | ✓ | — | — | — |

## Results by model

Each model page opens with *In short*, and its verification section has every test and its
tolerance. The headline results:

| model | checked against | result |
|---|---|---|
| [Frames](physics/frames.md) | exact rotations; RocketPy's starting attitude | 8 rail setups match RocketPy to 1e-12 rad; attitude stays within 1e-9 rad of exact over 1e6 steps |
| [Geodesy](physics/geodesy.md) | the WGS 84 standard's Table 3.5; round trips | the table to its printed digits; round trips within 1e-14 rad and 2e-8 m |
| [Gravity](physics/gravity.md) | the WGS 84 standard's formulas at 40 digits; RocketPy's formula | 11 points within 2e-14 relative; RocketPy at 8 points to under 1e-12 |
| [Atmosphere](physics/atmosphere.md) | the 1976 standard's tables; the CIPM-2007 moist-air formula | within 0.1% at 32 altitudes; humid density within 0.047% |
| [Wind](physics/wind.md) | unit tests; RocketPy descents in the same winds | drift magnitude within 0.28% |
| [Turbulence](physics/turbulence.md) | the Dryden spectra, over 2²⁰ samples | within 4 standard errors in every octave band; unvalidated for rockets |
| [Design tree](physics/design.md) | a hand-worked rocket; eight of RocketPy's example rockets | 1e-12 by hand; RocketPy within 8.0e-10 at its solver steps, 1.1e-5 in mass and 2.6e-5 in inertia between them |
| [Shapes](physics/shapes.md) | closed forms; independent high-precision integrals | 1e-10 and 1e-12 on 22 noses and transitions; 20 walls to 1e-10 |
| [Mass properties](physics/mass.md) | hand calculation; exact integration | 1e-11 and 1e-13 |
| [Solid motors](physics/motor.md) | ThrustCurve.org's statistics code; RocketPy's motor | 1.8e-15 on all 32 bundled curves; RocketPy within 7.9e-5 on three |
| [Aerodynamics](physics/aero.md) | Barrowman's worked examples; RASAero drag curves at Mach 0.3 | four of five examples within 1%, the Recruiter +2.87%; drag within 10% in four of seven cases, −18.3% and −47.0% to −50.4% in the rest |
| [Rigid-body flight](physics/flight.md) | exact motion in a vacuum | the centre of mass on the exact parabola to 1.7e-6 m over 22 s; no whole flight compared |
| [Time integration](physics/integration.md) | an independent `DOPRI5`; a flight with an exact solution | the same step counts; event times within 1.5e-8 s |
| [Recovery](physics/recovery.md) | RocketPy's descent for five rockets; published drop tests | all 30 metrics within 3%; tumbling −10% to +19% off its source's drops; streamers +9% flat, +58% pleated |
| [Interpolation](physics/interpolation.md) | a spline's closed form; property tests | every point hit; the closed form `y = 3x/2 − x³/2` matched |
| [Quadrature](physics/quadrature.md) | exact integrals | polynomials to degree 22 exactly; six test integrals to 1e-11 |

## The descent under a parachute, against RocketPy

This is the one comparison run by the validation harness so far. Five of RocketPy's example
rockets start from the same state near apogee in both codes: the first parachute opens at once,
with the same drag areas, triggers and wind, RocketPy's random noise off, and RocketPy's gravity
formula. The committed [validation report][report] says: 5 cases, 30 metrics, 30 scored, all within
tolerance, the largest difference +2.865%.

Each metric must agree within 3%, with no absolute floor, as each case file argues (for example,
[NDRT's][ndrt-case]). The differences, hpr against RocketPy:

| case | descent time | landing speed | drift | largest drift component | from |
|---|---|---|---|---|---|
| `descent-calisto-tests-motor-at-minus-1.373` | +0.077% | −0.029% | +0.075% | north, +0.080% | [report][report] |
| `descent-valetudo` | −0.019% | +0.004% | −0.889% | north, −1.766%, of a 0.000019 m drift | [report][report] |
| `descent-ndrt-2020-nose-to-tail` | +0.705% | +0.012% | +0.276% | north, +2.865% | [report][report] |
| `descent-prometheus-2022-generic-motor` | +0.083% | −0.030% | +0.083% | east, +0.086% | [report][report] |
| `descent-juno-iii` | −0.018% | −0.008% | −0.018% | east and north, −0.018% | [report][report] |

The largest gap, NDRT's north drift, comes from *added mass*: RocketPy counts the air a canopy
drags along, 15.9 kg for NDRT's main against the rocket's 20.8 kg, and hpr has no such term, so the
two respond differently as the canopy opens ([Recovery](physics/recovery.md#against-rocketpy)).
Valetudo falls in still air, so its drift comes only from the Earth's rotation, and its
largest relative difference is on a drift too small to matter.

What this shows: the two codes agree on the physics of a descent. It says nothing about whether
either matches a real parachute on a real day.

## Whole flights

Not validated yet. RocketPy's five example rockets have been flown from the pad to landing, with a
drag coefficient declared the same for both codes, and the result is committed as a reference
([validation plan][plan-refs]). hpr will be scored against it in [M2.1b2][roadmap], the
whole-flight comparison. One case will show a gap from the start: RocketPy's Prometheus peaks at
Mach 1.014, and hpr stops any flight that reaches Mach 1 until [M1.8][roadmap] adds transonic and
supersonic aerodynamics. The comparisons with OpenRocket ([M2.2][roadmap]) and with real flights
([M2.3][roadmap]) come after.

## Known gaps

These are the largest known differences and missing pieces. Each model page's *In short* lists the
rest.

- **Drag reads low from about Mach 0.6,** and above Mach 0.8 the aerodynamics are unvalidated
  ([Aerodynamics](physics/aero.md)).
- **Drag against RASAero's curves** at Mach 0.3 is within 10% in four of seven cases, but −18.3%
  for Cavour under power, and −47.0% and −50.4% for Valetudo
  ([Aerodynamics](physics/aero.md#verification)).
- **Six fins.** The normal-force slope of Barrowman's six-fin Recruiter is +2.87% above his
  printed value, mostly because hpr uses a different six-fin rule
  ([Aerodynamics](physics/aero.md#verification)).
- **Tumbling** drag is −10% to +19% off its source's own drop tests, and the default streamer model
  reads +58% fast on a pleated streamer ([Recovery](physics/recovery.md)).
- **No added mass under a canopy,** which is behind the +2.865% drift difference above
  ([Recovery](physics/recovery.md#against-rocketpy)).
- **Turbulence** is an aircraft model, unvalidated for rockets, and no flight uses it yet
  ([Turbulence](physics/turbulence.md)).
- **Wall and fin mass** may follow different conventions from OpenRocket's, which its documentation
  doesn't state: measuring a wall radially changes its volume by 1.4% on one cone
  ([Shapes](physics/shapes.md)).

[ndrt-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-ndrt-2020-nose-to-tail.toml
[plan]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md#principles
[plan-refs]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
