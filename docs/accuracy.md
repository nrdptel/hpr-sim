# Accuracy

This page gathers every check hpr-sim has passed so far, and every known gap, in words and numbers.
Start with the bottom line: **no whole flight has been validated yet.** hpr's apogee, top speed and
landing point have not been compared with another simulator's or with a real flight's. What has been
checked is each model on its own, against exact answers, its published source and in places
RocketPy, and the descent under a parachute, against RocketPy, for five rockets.

Each number here appears, written the same way (the same digits, and the same sign and percent sign
where it writes them), in the model page, report or case file that its sentence or table row links
to, and the descent table matches the report cell by cell. The site's build checks both, so a number
that changes at its source fails it, unless the same number still appears elsewhere in that file.
The check can't tell whether a number is quoted in the right context; the model pages say what each
one means. [Checking a claim](checking-a-claim.md) shows how to follow a number back to its source
and its test.

## Four kinds of evidence

A model can be checked in four ways, from the weakest to the strongest evidence that it matches
reality. They are set out in the project's [validation plan][plan].

| kind | what is compared | what agreement shows |
|---|---|---|
| **Analytic** | The code against exact answers: closed-form solutions, conservation laws, round trips | The code computes what its equations say |
| **Published source** | The code against a source's printed tables and worked examples | The code implements the source correctly |
| **Another code** | hpr against another simulator, such as RocketPy, flying the same inputs | The two codes agree on the physics; not that either matches reality |
| **Real flights** | hpr against measured flights | The model matches reality, within the flight's own uncertainty |

The first three check the code. Only the fourth checks the physics against the world, and no
flight has been compared yet: that is [M2.3][roadmap], the real-flights milestone. The nearest
thing so far is two recovery models checked against published drop tests, which are measurements
but not flights ([Recovery](physics/recovery.md)).

## Where each model stands

A tick means the model has been checked that way; a dash means it hasn't yet. "Sampled in the
descents" means RocketPy's values were compared at the heights its parachute descents pass
through, as part of that comparison.

| model | analytic | published source | another code | real flights |
|---|---|---|---|---|
| [Frames](physics/frames.md) | ✓ | — | ✓ RocketPy | — |
| [Geodesy](physics/geodesy.md) | ✓ | ✓ | — | — |
| [Gravity](physics/gravity.md) | ✓ | ✓ | ✓ RocketPy | — |
| [Atmosphere](physics/atmosphere.md) | ✓ | ✓ | ✓ RocketPy, sampled in the descents | — |
| [Wind](physics/wind.md) | ✓ | — | ✓ RocketPy, sampled in the descents | — |
| [Turbulence](physics/turbulence.md) | ✓ | — | — | — |
| [Design tree](physics/design.md) | ✓ | — | ✓ RocketPy, mass properties only | — |
| [Shapes](physics/shapes.md) | ✓ | — | — | — |
| [Mass properties](physics/mass.md) | ✓ | — | — | — |
| [Solid motors](physics/motor.md) | ✓ | — | ✓ RocketPy, ThrustCurve.org | — |
| [Aerodynamics](physics/aero.md) | ✓ | ✓ Barrowman's examples | partial: drag only, inputs not matched | — |
| [Rigid-body flight](physics/flight.md) | ✓ | — | — | — |
| [Time integration](physics/integration.md) | ✓ | — | — | — |
| [Recovery](physics/recovery.md) | ✓ | ✓ | ✓ RocketPy | — (drop tests ✓) |
| [Interpolation](physics/interpolation.md) | ✓ | — | — | — |
| [Quadrature](physics/quadrature.md) | ✓ | — | — | — |

## Results by model

Each model page opens with *In short*, and its verification section has every test and its
tolerance. The headline results, as relative differences unless a unit is given:

| model | checked against | result |
|---|---|---|
| [Frames](physics/frames.md) | exact rotations; RocketPy's starting attitude | 8 rail setups match RocketPy to 1e-12 rad; attitude stays within 1e-9 rad of exact over 1e6 steps |
| [Geodesy](physics/geodesy.md) | the WGS 84 standard's Table 3.5; round trips | the table to its printed digits; round trips within 1e-14 rad and 2e-8 m |
| [Gravity](physics/gravity.md) | the WGS 84 standard's formulas at 40 digits; RocketPy's formula | 11 points within 2e-14; RocketPy at 8 points to under 1e-12 |
| [Atmosphere](physics/atmosphere.md) | the 1976 standard's tables; the CIPM-2007 moist-air formula; RocketPy | within 0.1% at 32 altitudes; humid density within 0.047%; RocketPy's density within 3.7e-4 ([Recovery](physics/recovery.md#against-rocketpy)) |
| [Wind](physics/wind.md) | unit tests; RocketPy's wind in its descents | RocketPy's wind to 1e-9 m/s, and drift within 0.28% in the four cases with wind ([Recovery](physics/recovery.md#against-rocketpy)) |
| [Turbulence](physics/turbulence.md) | the Dryden spectra, over 2²⁰ samples | within 4 standard errors in every octave band; unvalidated for rockets |
| [Design tree](physics/design.md) | a hand-worked rocket; eight cases of RocketPy's example rockets | 1e-12 by hand; RocketPy within 8.0e-10 at its solver's times, and between them 1.1e-5 in mass and 2.6e-5 in inertia; grain propellant mass 2.4e-9 and 4.9e-5 of its initial value |
| [Shapes](physics/shapes.md) | closed forms; independent high-precision integrals | 1e-10 and 1e-12 on 22 noses and transitions; 20 walls to 1e-10 |
| [Mass properties](physics/mass.md) | hand calculation; exact integration | 1e-11 by hand; fin sections to 1e-13; densities converted as their sources print them |
| [Solid motors](physics/motor.md) | ThrustCurve.org's statistics code; RocketPy's motor | 1.8e-15 on all 32 bundled curves; RocketPy within 7.9e-5 on three, the propellant's quantities within 1e-4 of their values at ignition |
| [Aerodynamics](physics/aero.md) | Barrowman's worked examples, at Mach 0; drag curves labelled RASAero, at Mach 0.3 | four of five examples within 1%, the Recruiter +2.87% (+3.42% on its fins); drag within 10% in four of seven cases with guessed inputs, −18.3% for Cavour under power, and −47.0% and −50.4% for Valetudo, whose table is 1.44 times its own OpenRocket export (hpr is 23.5% under that export as designed here, 1.9% with the export's own finish and lugs) |
| [Rigid-body flight](physics/flight.md) | exact motion in a vacuum | the centre of mass on the exact parabola to 1.7e-6 m over 22 s; no whole flight compared |
| [Time integration](physics/integration.md) | an independent `DOPRI5`; a flight with an exact solution | the same step counts; event times within 1.5e-8 s |
| [Recovery](physics/recovery.md) | RocketPy's descents; published drop tests | every descent metric within 3% (below); the later triggers within 0.17% and the drogue's descent rate to 0.01%; tumbling −10 to +19% off its drops; streamers +9% fast on a flat one and +58% on a pleated one |
| [Interpolation](physics/interpolation.md) | a spline's closed form; property tests | the closed form `y = 3x/2 − x³/2` matched; every table hits its points |
| [Quadrature](physics/quadrature.md) | exact integrals | polynomials to degree 22 exactly; six test integrals to 1e-11 |

## The descent under a parachute, against RocketPy

This is the one comparison the validation harness runs so far. Five of RocketPy's example rockets
start from the same state near apogee in both codes, with the first parachute opening at once, the
same drag areas, triggers and wind, and RocketPy's random noise off. To compare like with like, hpr
uses RocketPy's gravity formula and interpolates the wind the way RocketPy does, by its east and
north components, rather than its own defaults. The committed [validation report][report] says:
5 cases, 30 metrics, 30 scored, all within tolerance, the largest difference +2.865%.

Each metric must agree within 3%, with no absolute floor, as each case file argues (for example,
[NDRT's][ndrt-case]). The metrics:

- `descent_time_s`: the time from the shared start to landing.
- `impact_speed_m_s`: the vertical speed at landing; the wind adds to the speed over the ground.
- `mean_descent_rate_m_s`: the start's height over the descent time, so it repeats the descent
  time in another form.
- `drift_m`, `drift_east_m` and `drift_north_m`: how far the rocket lands from where the descent
  started, not from the pad, and that distance's east and north parts. A drift to the south or
  west is negative.

A difference is hpr's value less RocketPy's, over RocketPy's. So a positive one means hpr's value
is larger in size, in the same direction: NDRT drifts south, and hpr carries it further south.

Every result of the report, as hpr's difference from RocketPy:

| case | `descent_time_s` | `impact_speed_m_s` | `mean_descent_rate_m_s` | `drift_m` | `drift_east_m` | `drift_north_m` |
|---|---|---|---|---|---|---|
| [`descent-calisto-tests-motor-at-minus-1.373`][report] | +0.077% | −0.029% | −0.077% | +0.075% | +0.074% | +0.080% |
| [`descent-valetudo`][report] | −0.019% | +0.004% | +0.019% | −0.889% | −0.889% | −1.766% |
| [`descent-ndrt-2020-nose-to-tail`][report] | +0.705% | +0.012% | −0.700% | +0.276% | +0.214% | +2.865% |
| [`descent-prometheus-2022-generic-motor`][report] | +0.083% | −0.030% | −0.083% | +0.083% | +0.086% | +0.082% |
| [`descent-juno-iii`][report] | −0.018% | −0.008% | +0.018% | −0.018% | −0.018% | −0.018% |

Valetudo falls in still air, so its drift, 0.19 m, comes only from the Earth's rotation, and its
north part is 19 µm; a small difference there is a large fraction
([Recovery](physics/recovery.md#against-rocketpy)).

The largest gap, NDRT's north drift, most likely comes from [added mass](glossary.md#added-mass):
RocketPy counts the air a canopy drags along, 15.9 kg for NDRT's main against the rocket's 20.8 kg,
and hpr has no such term, so the two respond differently as the canopy opens
([Recovery](physics/recovery.md#against-rocketpy)). That explanation fits the size of the gap, but
no test has isolated it yet.

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

- **Aerodynamics were checked at two speeds only:** Mach 0 for the normal force and centre of
  pressure, and Mach 0.3 for drag. Nose and shoulder pressure drag is held at its low-speed value,
  so from about
  Mach 0.6 it reads low against the source's own high-subsonic correction, and the models are
  documented only to Mach 0.8 ([Aerodynamics](physics/aero.md)).
- **Drag against the RASAero curves** is within 10% in four of seven cases, with the fins and
  finish guessed, because the curves don't record them. Cavour under power is −18.3%, cause open.
  Valetudo's −47.0% and −50.4% are against a table 1.44 times its own OpenRocket export
  ([Aerodynamics](physics/aero.md#verification)).
- **Six fins.** The normal-force slope of Barrowman's six-fin Recruiter is +2.87% above his printed
  value, and +3.42% on the fins alone, mostly because hpr uses a different six-fin rule
  ([Aerodynamics](physics/aero.md#verification)).
- **Tumbling** is −10 to +19% off its source's own drop tests, and is used far outside the fit
  behind it: Valetudo tumbles at 37 m/s against a fit from 5.0 to 6.6 m/s. The default streamer
  model reads +58% fast on a pleated streamer ([Recovery](physics/recovery.md)).
- **Opening loads** are no safe bound either way: with a filling time hpr leaves out the canopy's
  overshoot, and opening at once it ignores how a light rocket slows while the canopy fills. The
  deployment speed can itself read high: a separated body falls with no drag until its device
  opens ([Recovery](physics/recovery.md#inflation)).
- **No added mass under a canopy,** the likely cause of the 2.86% drift difference above
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
