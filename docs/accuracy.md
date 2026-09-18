# Accuracy

This page gathers every check hpr-sim has passed so far, and every known gap, in words and numbers.
Start with the bottom line: **when both codes fly the same drag
([same-drag](glossary.md#same-drag-and-predicted-mode)), hpr's whole flights match RocketPy's in
height, speed and time, but not in where they go in wind.** With each code's own drag
([predicted](glossary.md#same-drag-and-predicted-mode)), hpr's heights differ from RocketPy's by
−0.527% to +10.232% ([report][report]), the larger gaps where its drag is well below the example's.
No flight has been compared with a real one.

What has been checked so far:

- each model on its own, against exact answers, its published source and, in places,
  [RocketPy](glossary.md#rocketpy), an open-source flight simulator;
- the descent under a parachute, against RocketPy, for five rockets;
- whole flights from the pad to the ground, against RocketPy, for five rockets flown with one
  declared drag coefficient, with a sixth reported as a gap: heights, speeds and times agree, the
  path in wind does not;
- the same flights with each code's own drag, reported against a target rather than gated.

Every number here links to the page or file it comes from. [Checking a claim](checking-a-claim.md)
shows how to follow one back to its source and its test, and
[how the site keeps the two in step](checking-a-claim.md#rules-that-keep-the-trail-honest).

## How to read the numbers

- **Powers of ten.** Very small and very large numbers are written the way programs print them.
  The number after the `e` says how many places the decimal point moves, to the left when it is
  negative. So 1e-12 is a millionth of a millionth, a 1 in the twelfth decimal place, and 1e6 is a
  million. Both appear in the two [Frames](physics/frames.md) rows of the results table below.
- **Absolute differences** carry a unit: they say how far apart two values are, in that unit.
  [Geodesy](physics/geodesy.md)'s round trips return heights within 2e-8 m, twenty billionths of a
  metre.
- **Relative differences** are marked *relative*: the difference as a fraction of the value it is
  compared with. [Gravity](physics/gravity.md) within 2e-14 relative means within two parts in a
  hundred million million. A percentage is a relative difference counted in hundredths.
- **Signs.** A signed difference is hpr's value less the one it is compared with, over that one.
  A plus means hpr's value is the larger in size, a minus the smaller.

## Four kinds of evidence

A model can be checked in four ways, from the weakest to the strongest evidence that it matches
reality. They are set out in the project's [validation plan][plan].

| kind | what is compared | what agreement shows |
|---|---|---|
| **Analytic** | The code against exact answers: formulas solved by hand (closed forms), conservation laws, and round trips (converting a value and converting it back) | The code computes what its equations say |
| **Published source** | The code against a source's printed tables and worked examples | The code implements the source correctly |
| **Another code** | hpr against another simulator, such as RocketPy, flying the same inputs | The two codes agree on the physics; not that either matches reality |
| **Real flights** | hpr against measured flights | The model matches reality, within the flight's own uncertainty |

The first three check the code. Only the fourth checks the physics against the world, and no
flight has been compared yet: that is [M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone. The nearest
thing so far is two recovery models checked against published drop tests, which are measurements
but not flights ([Recovery](physics/recovery.md)).

## Where each model stands

A tick means the model has been checked that way; a dash means it hasn't yet. *In the descents
only* means RocketPy's air density and wind were compared with hpr's at just the 23 heights its
parachute descents sample, as part of that comparison, and nowhere else
([Recovery](physics/recovery.md#against-rocketpy)).

| model | analytic | published source | another code | real flights |
|---|---|---|---|---|
| [Frames](physics/frames.md) | ✓ | — | ✓ RocketPy | — |
| [Geodesy](physics/geodesy.md) | ✓ | ✓ | — | — |
| [Gravity](physics/gravity.md) | ✓ | ✓ | ✓ RocketPy | — |
| [Atmosphere](physics/atmosphere.md) | ✓ | ✓ | ✓ RocketPy, in the descents only | — |
| [Wind](physics/wind.md) | ✓ | — | ✓ RocketPy, in the descents only | — |
| [Turbulence](physics/turbulence.md) | ✓ | — | — | — |
| [Design tree](physics/design.md) | ✓ | — | ✓ RocketPy, mass properties only | — |
| [Shapes](physics/shapes.md) | ✓ | — | — | — |
| [Mass properties](physics/mass.md) | ✓ | — | — | — |
| [Solid motors](physics/motor.md) | ✓ | — | ✓ RocketPy, ThrustCurve.org | — |
| [Aerodynamics](physics/aero.md) | ✓ | ✓ Barrowman's examples | partial: drag only, with the fins and finish guessed; and in whole flights, against a target | — |
| [Rigid-body flight](physics/flight.md) | ✓ | — | ✓ RocketPy, with the drag given; and on each code's own drag, against a target | — |
| [Time integration](physics/integration.md) | ✓ | — | — | — |
| [Recovery](physics/recovery.md) | ✓ | ✓ | ✓ RocketPy | — (drop tests ✓) |
| [Interpolation](physics/interpolation.md) | ✓ | — | — | — |
| [Quadrature](physics/quadrature.md) | ✓ | — | — | — |

## Results by model

The headline results, one check to a row. Each model page opens with *In short*, and its
verification section lists every test and its [tolerance](glossary.md#tolerance), how far a result
may be from its reference and still pass.

| model | compared with | how close |
|---|---|---|
| [Frames](physics/frames.md) | RocketPy's starting attitude, worked out from the launch rail's angles, for 8 rail setups | within 1e-12 rad |
| [Frames](physics/frames.md) | an exactly solvable spin whose axis sweeps round a cone (coning), over 1e6 integration steps | attitude within 1e-9 rad |
| [Geodesy](physics/geodesy.md) | the ellipsoid values printed in Table 3.5 of the [WGS 84](glossary.md#wgs-84) standard | to their printed digits |
| [Geodesy](physics/geodesy.md) | round trips at random points from −10 km to +1000 km: latitude, longitude and height to Earth-centred x, y, z, and back | latitude within 1e-14 rad, height within 2e-8 m |
| [Gravity](physics/gravity.md) | the WGS 84 formulas, worked to 40 digits by a separate script, at 11 points from the equator to both poles and up to 200 km high | gravity's strength within 2e-14 relative |
| [Gravity](physics/gravity.md) | RocketPy's gravity formula, at 8 points | under 1e-12 relative |
| [Atmosphere](physics/atmosphere.md) | the tables of the 1976 [standard atmosphere](glossary.md#standard-atmosphere), at 32 heights from −2 to 86 km | every value within 0.1% |
| [Atmosphere](physics/atmosphere.md) | CIPM-2007 (Picard et al., 2008), a published reference formula for the density of humid air that treats air as a real gas, from 15 to 27 °C | humid-air density within 0.047% |
| [Atmosphere](physics/atmosphere.md) | RocketPy's air density, at the 23 heights its descents sample ([Recovery](physics/recovery.md#against-rocketpy)) | within 3.7e-4 relative |
| [Wind](physics/wind.md) | RocketPy's wind, at the same heights ([Recovery](physics/recovery.md#against-rocketpy)) | each component within 1e-9 m/s |
| [Wind](physics/wind.md) | the drift of RocketPy's four descents with wind ([Recovery](physics/recovery.md#against-rocketpy)) | the distance drifted within 0.28% |
| [Turbulence](physics/turbulence.md) | the [Dryden](glossary.md#turbulence-dryden) gust spectra, published formulas for how gust strength spreads over wavelength, over 2²⁰ random samples (about a million) | within 4 standard errors (the scatter expected by chance) in every octave band (a range of wavelengths spanning a factor of two): ±1–3% in the wide bands. Unvalidated for rockets |
| [Design tree](physics/design.md) | a rocket worked by hand, loaded, burning and burnt out | mass within 1e-12 kg, centre of mass within 1e-12 m, inertia within 1e-12 relative |
| [Design tree](physics/design.md) | RocketPy, for eight cases of its example rockets, at the times its equation solver computed the burning grains (up to 60 per case) | mass, centre of mass and inertia within 8.0e-10 relative (the centre as a fraction of the rocket's length) |
| [Design tree](physics/design.md) | the same, at 103 even times through the burn and after it, where RocketPy interpolates between its solver's times | mass within 1.1e-5 relative, inertia within 2.6e-5 relative |
| [Design tree](physics/design.md) | the propellant mass left in the grains, at both sets of times | within 2.4e-9 of the initial propellant mass at the solver's times, and 4.9e-5 between them |
| [Shapes](physics/shapes.md) | exact formulas (closed forms) for filled noses and transitions | within 1e-10 relative |
| [Shapes](physics/shapes.md) | separately computed high-precision integrals, for 22 noses and transitions | within 1e-12 relative |
| [Shapes](physics/shapes.md) | the same kind of integrals, for 20 hollow shells of a given wall thickness | within 1e-10 relative |
| [Mass properties](physics/mass.md) | a cone, a tube, four fins and an off-axis payload, added up by hand | within 1e-11 relative |
| [Mass properties](physics/mass.md) | fin cross-sections, against exact numerical integration | within 1e-13 relative |
| [Mass properties](physics/mass.md) | material densities, converted from the units their sources print | the sources' values, such as white ash at 678 kg/m³ |
| [Solid motors](physics/motor.md) | [ThrustCurve.org](glossary.md#thrustcurveorg)'s own statistics code (total impulse, burn time, average and peak thrust), on all 32 bundled curves | within 1.8e-15 relative |
| [Solid motors](physics/motor.md) | RocketPy's solid-motor model, on three bundled motors, at 203 times each | total mass and inertias within 7.9e-5 relative; the propellant's own mass and inertias within 1e-4 of their values at ignition |
| [Aerodynamics](physics/aero.md) | [Barrowman's](glossary.md#barrowmans-method) five worked examples, at Mach 0 (low speed): each rocket's [normal-force slope](glossary.md#normal-force-slope) and [centre of pressure](glossary.md#centre-of-pressure-cp) | every centre of pressure within 1%. Every slope within 1% too, except the six-fin Recruiter's: +2.87% (+3.42% on its fins alone) |
| [Aerodynamics](physics/aero.md) | drag curves labelled [RASAero](glossary.md#rasaero-ii) in RocketPy's examples, at [Mach](glossary.md#mach-number) 0.3, with the fins and surface finish guessed because the curves don't record them | within 10% in four of seven cases; −18.3% for Cavour [power-on](glossary.md#power-on-and-power-off-drag) (motor burning), cause open |
| [Aerodynamics](physics/aero.md) | Valetudo's drag table, which is 1.44 times the drag in the [OpenRocket](glossary.md#openrocket) export for the same rocket | −47.0% power-off and −50.4% power-on. Against the OpenRocket export, hpr is 23.5% under as designed here, and 1.9% under with the export's own surface finish and launch lugs |
| [Rigid-body flight](physics/flight.md) | the exact motion of a tumbling, spinning rocket in a vacuum, over 22 s | the centre of mass within 1.7e-6 m of the exact parabola |
| [Rigid-body flight](physics/flight.md) | RocketPy's whole flights from the pad to the ground, for five rockets, both codes flying one declared drag coefficient | heights, speeds, times and accelerations within 3% ([below](#whole-flights-against-rocketpy)), the largest +1.783% in the [report][report]; in wind the horizontal path is far off, reported but not scored ([issue #50][issue-50]) |
| [Time integration](physics/integration.md) | a separate line-by-line transcription of `DOPRI5`, the published Fortran integrator by Hairer and Wanner that hpr's [Dormand–Prince](glossary.md#dormandprince-and-rk4) stepper follows, on the problem Hairer's own example program for `DOPRI5` solves: the Arenstorf orbit, the closed, looping path of a small body pulled by two large ones that circle each other | the same step counts |
| [Time integration](physics/integration.md) | a vertical flight with drag that has an exact solution | apogee, deployment and landing times within 1.5e-8 s |
| [Recovery](physics/recovery.md) | RocketPy's descents under a parachute, for five rockets | every descent metric within 3% ([below](#the-descent-under-a-parachute-against-rocketpy)) |
| [Recovery](physics/recovery.md) | the same descents: the heights where the later parachutes fire, and the descent rate under the drogue | within 0.17% and 0.01% |
| [Recovery](physics/recovery.md) | published drop tests of five small models falling with nothing deployed ([tumbling](glossary.md#tumble-recovery)) | descent rate −10 to +19% off |
| [Recovery](physics/recovery.md) | Kidwell's [streamer](glossary.md#streamer) drop tests (2001) | descent rate +9% fast for his one flat streamer, and +58% fast for one folded into pleats, which hpr doesn't model |
| [Interpolation](physics/interpolation.md) | the exact formula of a smooth curve (a spline) through three points, `y = 3x/2 − x³/2` | matched |
| [Interpolation](physics/interpolation.md) | property tests, which check a rule on many randomly generated tables | every table passes exactly through its own points |
| [Quadrature](physics/quadrature.md) | polynomials, whose integrals are known exactly | exact up to degree 22 |
| [Quadrature](physics/quadrature.md) | six test integrals with known answers, one of them infinite at an end | within 1e-11 relative |

## The descent under a parachute, against RocketPy

This is one of the two comparisons the validation harness runs; the other is
[whole flights](#whole-flights-against-rocketpy). The harness is the program behind
`cargo xtask validate`, which flies every [validation case](glossary.md#validation-case) and
writes the committed report.

Five of RocketPy's [example rockets](glossary.md#example-rockets) are flown down in both codes,
set up the same way:

- the same starting state near apogee, with the first parachute opening at once;
- the same [drag areas](glossary.md#drag-area), deployment triggers and wind;
- the random noise RocketPy can add to each parachute switched off, so its runs repeat exactly;
- RocketPy's gravity formula, and its way of interpolating the wind (by its east and north
  components), in place of hpr's own defaults, to compare like with like.

The committed [validation report][report] gives the six numbers below for each descent. All of
them were scored and all are within tolerance; the largest difference is +2.865%.

Each metric must agree within 3% of RocketPy's value, with no absolute floor (a fixed allowance,
in metres or seconds, that would pass any smaller difference). Each case file argues why, for
example [NDRT's][ndrt-case]. The metrics:

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

Each case is named after the RocketPy example it flies. Three names carry more:

- `descent-calisto-tests-motor-at-minus-1.373` is Calisto as RocketPy's own tests build it, with
  the motor at −1.373 m in RocketPy's coordinates, where its getting-started notebook puts it at
  −1.255 m ([notes on RocketPy's example rockets][rocket-notes]).
- `descent-ndrt-2020-nose-to-tail` is the NDRT 2020 rocket, which RocketPy's example measures from
  the nose toward the tail ([notes on RocketPy's example rockets][rocket-notes]).
- `descent-prometheus-2022-generic-motor` is Prometheus 2022, whose motor RocketPy describes with
  its generic-motor model, which treats the propellant as a solid cylinder
  ([notes on RocketPy's example rockets][rocket-notes],
  [Recovery](physics/recovery.md#against-rocketpy)).

The Valetudo case flies RocketPy's own Valetudo example, not the flight on
[Getting started](getting-started.md). From the shared start, 800 m above the ground, it falls in
still air under the example's one drogue, with RocketPy's drag area of 0.4537 m², and lands at
17.627 m/s. Getting started flies the same airframe from the pad, with its own drogue, a main
parachute and a 5 m/s wind ([case file][valetudo-case],
[Recovery](physics/recovery.md#against-rocketpy)).

In still air, Valetudo's drift, 0.19 m, comes only from the Earth's rotation (the
[Coriolis acceleration](glossary.md#coriolis-acceleration)), and its north part is 19 µm. So a
small difference there is a large fraction ([Recovery](physics/recovery.md#against-rocketpy)).

The largest gap, NDRT's north drift, most likely comes from [added mass](glossary.md#added-mass):
RocketPy counts the air a canopy drags along, 15.9 kg for NDRT's main against the rocket's 20.8 kg,
and hpr has no such term, so the two respond differently as the canopy opens
([Recovery](physics/recovery.md#against-rocketpy)). That explanation fits the size of the gap, but
no test has isolated it yet.

What this shows: the two codes agree on the physics of a descent. It says nothing about whether
either matches a real parachute on a real day.

## Whole flights against RocketPy

Six of RocketPy's example rockets are flown from the pad to the ground in both codes: Calisto,
Valetudo, NDRT 2020, Juno III, Bella Lui and Prometheus 2022. They are set up the same way
([ADR-021][adr-021], the whole-flight comparison):

- one declared [drag coefficient](glossary.md#drag-coefficient), a constant 0.5
  ([case file][juno-case]), on the same reference area;
- the example's launch rail, site, parachutes and motor, with the thrust curve flown as measured
  and no correction for the thinner air at the site, as RocketPy's examples fly it;
- RocketPy's gravity formula, standard atmosphere and frictionless rail, and a declared wind;
- the random noise RocketPy can add to each parachute switched off.

This is [same-drag](glossary.md#same-drag-and-predicted-mode) mode. It checks the equations of
motion, the motor and the air, not the drag. hpr's own drag is compared
[below](#whole-flights-with-each-codes-own-drag).

**In short: how high, how fast and how long agree; where the rocket goes in wind does not.** The
heights, speeds, times and accelerations of five flights agree within the 3% of each case's gate
([case file][juno-case]); the largest difference is +1.783% ([report][report]). The horizontal path
does not: in wind hpr turns into the wind less than RocketPy does, and its landing points are far
off. So the landing offset that [M2.1](decisions-and-roadmap.md#m2-1) asks for is not met ([issue #50][issue-50]).

Each of fifteen numbers per flight must agree within 3% of RocketPy's, with no absolute floor, or
say in its case file why it is not scored. Each case file argues why, for example
[Juno III's][juno-case]. Two more numbers compare the whole trace; they are explained after the
tables.

The numbers are measured as RocketPy defines them, with one exception. Each code's solver advances
the flight in [time steps](glossary.md#adaptive-time-step) and keeps the state at each step's end.
RocketPy takes each maximum (top speed, top Mach, top acceleration) only at those step ends. hpr
also searches between its own step ends for the true peak, so that its number does not depend on
where its solver happened to step. That search can only raise a maximum. Against hpr's old
step-end readings it raised them by at most 6.3e-5 of themselves (NDRT 2020's top speed). How much
RocketPy's own step ends miss was not measured. Either way it is far inside the 3% gate
([ADR-023][adr-023], the decision that also sets how peaks are found in both modes).

The definitions:

- `apogee_agl_m` and `apogee_time_s`: the highest point, and when.
- `flight_time_s`: the time from ignition to landing.
- `max_speed_m_s` and `max_mach`: the top speed over the ground, and the top
  [Mach number](glossary.md#mach-number).
- `rail_exit_speed_m_s` and `rail_exit_time_s`: when the forward rail button reaches the top of
  the rail, and the speed then. hpr's own rail-exit event waits for the last button, so the
  comparison finds RocketPy's instant instead.
- `burnout_altitude_agl_m` and `burnout_speed_m_s`: at the end of the thrust curve.
- `impact_speed_m_s`: the vertical speed at landing.
- `max_acceleration_power_on_m_s2`: the largest acceleration while the motor burns.
- `max_acceleration_m_s2` and `max_acceleration_time_s`: the largest over the whole flight, and
  when. For NDRT 2020 that is its main parachute opening, not a flight load
  ([case file][ndrt-flight-case]).
- `apogee_drift_m` and `landing_drift_m`: how far from the pad, along the ground, the apogee and
  the landing point are.

Speeds and accelerations are those of the rocket's centre of mass without propellant, the point
RocketPy's flight follows; hpr's own output follows the centre of mass of the loaded rocket.
Heights are measured from where that point starts, as RocketPy's are. A difference is hpr's value
less RocketPy's, over RocketPy's.

The [validation report][report] scores all but eleven of the numbers of the five flights, and all
of the scored ones are within tolerance. The eleven are measured and reported but not scored, each for
a reason written in its case file (below). Prometheus 2022, the sixth rocket, is a known gap
(below). Every result of the report, as hpr's difference from RocketPy:

| case | `apogee_agl_m` | `apogee_time_s` | `flight_time_s` | `max_speed_m_s` | `max_mach` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +0.136% | +0.167% | +0.165% | −0.009% | −0.178% |
| [`flight-valetudo`][report] | +0.003% | +0.217% | +0.160% | −0.049% | −0.110% |
| [`flight-ndrt-2020-nose-to-tail`][report] | −0.010% | +0.150% | +0.630% | −0.049% | −0.089% |
| [`flight-juno-iii`][report] | +1.710% | +0.956% | +1.293% | +0.059% | −0.392% |
| [`flight-bella-lui`][report] | +0.476% | +0.275% | +0.319% | −0.018% | −0.134% |
| [`flight-juno-iii-calm`][report] | +0.060% | +0.069% | +0.042% | −0.034% | −0.172% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | −0.004% | +0.096% | +0.091% | −0.025% | −0.152% |
| [`flight-bella-lui-calm`][report] | −0.029% | −0.013% | −0.065% | −0.018% | −0.056% |

| case | `rail_exit_speed_m_s` | `rail_exit_time_s` | `burnout_altitude_agl_m` | `burnout_speed_m_s` | `impact_speed_m_s` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | −0.010% | −0.072% | +0.065% | −0.016% | −0.020% |
| [`flight-valetudo`][report] | −0.002% | −0.100% | +0.033% | −0.066% | +0.009% |
| [`flight-ndrt-2020-nose-to-tail`][report] | −0.009% | −0.085% | +0.032% | −0.066% | +0.022% |
| [`flight-juno-iii`][report] | −0.005% | −0.142% | +0.782% | +0.040% | −0.003% |
| [`flight-bella-lui`][report] | −0.013% | −0.029% | +0.228% | −0.029% | +0.021% |
| [`flight-juno-iii-calm`][report] | −0.002% | −0.137% | +0.062% | −0.049% | +0.001% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | −0.001% | −0.074% | +0.005% | −0.032% | −0.003% |
| [`flight-bella-lui-calm`][report] | −0.000% | −0.032% | +0.007% | −0.025% | +0.021% |

| case | `max_acceleration_power_on_m_s2` | `max_acceleration_m_s2` | `max_acceleration_time_s` | `apogee_drift_m` | `landing_drift_m` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +0.074% | +0.074% | −96.812% | −5.573% | +7.034% |
| [`flight-valetudo`][report] | +0.248% | +0.248% | +0.006% | −2.426% | −3.410% |
| [`flight-ndrt-2020-nose-to-tail`][report] | −0.001% | +83.060% | +0.147% | −18.646% | +3.887% |
| [`flight-juno-iii`][report] | −0.235% | −0.235% | −0.002% | −60.848% | +151.334% |
| [`flight-bella-lui`][report] | +1.783% | +1.783% | +0.001% | −15.589% | −31.320% |
| [`flight-juno-iii-calm`][report] | −0.020% | −0.020% | −0.036% | −3.670% | −3.695% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | +0.070% | +0.070% | −96.811% | −1.258% | −1.438% |
| [`flight-bella-lui-calm`][report] | +1.778% | +1.778% | +0.001% | −1.593% | −2.583% |

The last two numbers compare the whole trace, not one point of it. The series height RMS
(`series_height_rms_m`) is the root mean square of hpr's height less RocketPy's: square each
difference, average the squares, and take the square root. The series speed RMS
(`series_speed_rms_m_s`) is the same for speed. Both follow the centre of mass without propellant,
at RocketPy's 120 series times, from ignition until hpr lands. Both codes' clocks start at ignition
on the rail, so no time shift is fitted: a fitted shift would hide a real difference in the burn
or on the rail ([case file][juno-case]). The centre of mass without propellant is the point
RocketPy's series records. Every time counts the same, so the long descent weighs most; a
difference during the burn shows in the burnout and top-speed numbers instead.

Exact agreement would give 0, so these two are given in metres and metres per second, not as a
percentage. Each is held to 3% of RocketPy's apogee (for height) or top speed (for speed). That is
[M2.1](decisions-and-roadmap.md#m2-1)'s 3% for one number, applied to the whole trace
([case file][juno-case]).

All five flights pass, each well inside its bound. The largest height RMS is Juno III's,
39.200769 m against its 77.6 m bound, about half of it; its apogee is also the furthest off. The
other four are under a sixth of theirs. The speed RMS runs from 0.132323 to 2.058916 m/s
([report][report]).

| case | `series_height_rms_m` | height bound, m | `series_speed_rms_m_s` | speed bound, m/s |
|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +3.166705 | 78.3 | +0.160127 | 7.3 |
| [`flight-valetudo`][report] | +1.441957 | 23.3 | +0.228240 | 3.3 |
| [`flight-ndrt-2020-nose-to-tail`][report] | +2.037097 | 36.5 | +0.132323 | 5.4 |
| [`flight-juno-iii`][report] | +39.200769 | 77.6 | +2.058916 | 6.7 |
| [`flight-bella-lui`][report] | +2.302356 | 15.8 | +0.444735 | 2.9 |
| [`flight-juno-iii-calm`][report] | +1.296871 | 78.7 | +0.156012 | 6.8 |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | +1.141118 | 78.4 | +0.094629 | 7.3 |
| [`flight-bella-lui-calm`][report] | +0.389902 | 16.2 | +0.024575 | 2.9 |

What the two codes still do differently, and what it moves:

- **In wind, the path.** hpr turns into the wind less than RocketPy. In Juno III's 8.5 m/s wind,
  RocketPy's apogee moves 1,147 m upwind of where it is in calm air, and hpr's 769 m, so hpr's
  apogee ends 228 m from the pad and RocketPy's 582 m. Flown with no wind, as three cases whose ids
  end in `-calm`, the same rockets agree on the apogee to within 0.060%. Calisto's and Bella Lui's
  drifts agree to within −1.258% to −2.583%, and pass. Juno III's miss by −3.670% and −3.695%,
  and are reported but not scored ([case file][juno-calm-case], [ADR-025][adr-025]): RocketPy
  frees the rocket from the rail when its first rail button leaves, and hpr when its last one does.
  Flown so both free it at the same point, every calm drift agrees to within 2.2%. So most of the
  gap in wind is in the response to wind, which same-drag mode doesn't share: each code's own normal force and damping, and how hpr's drag grows
  at an angle. The drifts of the four windy cases, and Valetudo's still-air landing drift,
  −3.410%, are open misses: reported, not scored and not passed, until [issue #50][issue-50] finds
  the cause. Turning less, hpr also flies higher, the likely source of Juno III's +1.710% apogee
  ([report][report], [case file][juno-case]).
- **On the rail,** hpr keeps the terms for the centre of mass moving inside the body as the
  propellant burns, and RocketPy's rail equation leaves them out. At a sharp ignition spike, with
  thrust and mass the same to five digits, hpr's acceleration is 1.2 to 1.3 m/s² higher. That is
  Bella Lui's +1.783%, whose peak is 7 ms after ignition ([report][report],
  [case file][bella-case]).
- **Calisto's two peaks.** Calisto's acceleration peaks twice, 0.9% apart: on the rail at 0.05 s
  and at 1.568 s. The same rail terms make hpr's first peak the higher, so `max_acceleration_time_s`
  moves from one peak to the other (−96.812%); it is reported but not scored. The peak's size is
  scored, but its +0.074% compares two instants; at 0.05 s hpr is 1.05% higher
  ([report][report], [case file][calisto-case]).
- **The main opening.** RocketPy adds the air a canopy drags along
  ([added mass](glossary.md#added-mass)), and hpr has none. So NDRT's peak deceleration as its
  main opens is +83.060% in hpr, reported but not scored. Its time is scored, since both codes put
  it where the main opens ([report][report], [case file][ndrt-flight-case]).

**Prometheus 2022 is a known gap.** RocketPy's flight peaks at Mach 1.014, and hpr stops any
flight at Mach 1 until [M1.8](decisions-and-roadmap.md#m1-8) adds transonic and supersonic
aerodynamics. The case still runs: the harness checks that hpr stops it for that reason and no
other, lists it under the report's *Known gaps*, and fails once hpr flies it, so the gap can't
outlive its cause ([case file][prometheus-case]). Bella Lui was added so that five rockets can
still be scored while Prometheus can't.

What this shows: with the drag given, the two codes agree on how high, how fast and how long a
rocket flies. They don't agree well enough on where it goes: within a few per cent in calm air, and
far apart in wind, which is open. [M2.1](decisions-and-roadmap.md#m2-1)'s landing offset is
reported but not met ([issue #50][issue-50]). Nothing here says anything about hpr's own drag, which the next section compares, or about a real flight. The
comparisons with OpenRocket ([M2.2](decisions-and-roadmap.md#m2-2), the OpenRocket comparison)
and with real flights ([M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone) come
after.

## Whole flights with each code's own drag

The same five flights again, in [predicted](glossary.md#same-drag-and-predicted-mode) mode: hpr
flies its own drag, from each design's shape, where same-drag mode gives it the declared one; both
modes use hpr's own [normal force](physics/aero.md), so only the drag differs. RocketPy flies the drag each of its examples ships, as RocketPy 1.13.0 flies the example:
a drag curve for Calisto, Valetudo and Juno III, a constant for NDRT 2020 (0.44,
[case file][ndrt-predicted-case]) and Bella Lui (0.43, [case file][bella-predicted-case]).
Everything else is set up as in the same-drag flights above ([ADR-023][adr-023], the
predicted-mode comparison).

**In short: hpr's heights are within 3% of RocketPy's for Calisto (−0.527%) and Bella Lui
(+1.118%), just outside for Juno III (+3.181%), and well above for Valetudo (+10.007%) and NDRT
2020 (+10.232%), where its drag is well below the example's** ([report][report]). These are
results, not a pass or fail. Neither code's drag is the truth: each example's drag came from
RASAero, OpenRocket or its team's own estimate. So each number is compared with the same 3% as the
same-drag flights ([M2.1](decisions-and-roadmap.md#m2-1), the validation milestone), but only as a
[target](glossary.md#gate-and-target). A miss is reported and explained in its case file
([Valetudo's][valetudo-predicted-case] and [NDRT 2020's][ndrt-predicted-case], for example), and
does not fail the test suite. The report is committed, so any number that moves shows up in
review.

Every predicted result of the report, as hpr's difference from RocketPy:

| case | `apogee_agl_m` | `apogee_time_s` | `flight_time_s` | `max_speed_m_s` | `max_mach` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | −0.527% | −0.505% | −0.244% | +0.337% | +0.172% |
| [`predicted-valetudo`][report] | +10.007% | +6.165% | +8.847% | +2.210% | +2.155% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +10.232% | +6.532% | +6.834% | +1.298% | +1.258% |
| [`predicted-juno-iii`][report] | +3.181% | +1.620% | +2.510% | +1.007% | +0.560% |
| [`predicted-bella-lui`][report] | +1.118% | +0.616% | +0.843% | +0.209% | +0.092% |

| case | `rail_exit_speed_m_s` | `rail_exit_time_s` | `burnout_altitude_agl_m` | `burnout_speed_m_s` | `impact_speed_m_s` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | −0.009% | −0.071% | +0.285% | +0.490% | −0.020% |
| [`predicted-valetudo`][report] | +0.029% | −0.109% | +1.310% | +2.771% | +0.009% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +0.000% | −0.087% | +0.759% | +1.425% | +0.022% |
| [`predicted-juno-iii`][report] | −0.001% | −0.143% | +1.285% | +1.108% | −0.003% |
| [`predicted-bella-lui`][report] | −0.011% | −0.029% | +0.357% | +0.252% | +0.021% |

| case | `max_acceleration_power_on_m_s2` | `max_acceleration_m_s2` | `max_acceleration_time_s` | `apogee_drift_m` | `landing_drift_m` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | +0.090% | +0.090% | −0.005% | −7.002% | +6.918% |
| [`predicted-valetudo`][report] | +0.250% | +0.250% | −0.030% | +10.547% | +8.686% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +0.674% | +83.058% | +9.606% | −4.440% | +14.981% |
| [`predicted-juno-iii`][report] | +0.072% | +0.072% | −0.045% | −59.133% | +171.819% |
| [`predicted-bella-lui`][report] | +1.797% | +1.797% | −0.029% | −14.798% | −30.341% |

The whole-trace numbers, in metres and metres per second, defined as for the same-drag flights
above. [Valetudo's][valetudo-predicted-case] and [NDRT 2020's][ndrt-predicted-case] height RMS
are outside the target, and so is NDRT 2020's speed RMS, for the same reason as their apogees:
hpr's own drag is lower than those examples' drag. Each bound is 3% of that case's own RocketPy
apogee or top speed, so it differs from the same-drag bound: Juno III's 80.919560 m is inside its
83.1 m here ([report][report]).

| case | `series_height_rms_m` | height bound, m | `series_speed_rms_m_s` | speed bound, m/s |
|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | +10.937445 | 84.5 | +0.398035 | 7.4 |
| [`predicted-valetudo`][report] | +74.842332 | 21 | +2.742051 | 3.3 |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +115.696314 | 38.2 | +6.741165 | 5.5 |
| [`predicted-juno-iii`][report] | +80.919560 | 83.1 | +1.940801 | 6.8 |
| [`predicted-bella-lui`][report] | +5.953813 | 16.1 | +0.422442 | 2.9 |

Why the misses, largest first:

- **Valetudo and NDRT 2020 fly high: the drag.** hpr's drag coefficient at Mach 0.3 is −47.0%
  from Valetudo's table, a hand-edited table 1.44 times the drag of the OpenRocket export for the
  same rocket ([Aerodynamics](physics/aero.md#verification)). For NDRT 2020 it is 0.318 against
  the example's constant 0.44 ([case file][ndrt-predicted-case]). These drags are compared at Mach 0.3 only. Flown on the same drag, the
  apogees agree with RocketPy's to +0.003% and −0.010% ([report][report]). Less drag also means a
  later apogee, a longer descent and further to drift, which moves their times and drifts too.
- **Juno III is +3.181% high.** hpr's drag is 6.0% below the example's curve at Mach 0.3, and the
  same-drag flight is already +1.710% ([report][report], [case file][juno-predicted-case]).
- **hpr's drag here is for the design as transcribed.** Where RocketPy's examples say nothing, the
  designs' fin thickness and edges and their surface finish are placeholders, so these results
  compare hpr's drag for those designs, not for the rockets as built. For Valetudo, the rocket's own
  OpenRocket finish and launch lugs take hpr's drag coefficient from 0.5566 to 0.714
  ([Aerodynamics](physics/aero.md#drag-verification)). So a miss here is not a gap for hpr
  to close toward the example's drag.
- **The drifts in wind,** in every windy case: the open question of [issue #50][issue-50], as in
  same-drag mode.
- **NDRT 2020's peak deceleration** at its main opening, +83.058%, is the added-mass difference
  explained above. Its time moves +9.606% with the later apogee ([report][report],
  [case file][ndrt-predicted-case]).

**Prometheus 2022 is a known gap here too.** On its own drag RocketPy's flight peaks at Mach 1.049,
and hpr stops at Mach 1 until [M1.8](decisions-and-roadmap.md#m1-8)
([case file][prometheus-predicted-case]).

What this shows: with its own drag, hpr's heights differ from RocketPy's by −0.527% to +10.232%
([report][report]), and the larger gaps are the two drags differing, not the flight. It does not
say which drag is right; only real flights can ([M2.3](decisions-and-roadmap.md#m2-3), the
real-flights milestone).

## Known gaps

These are the largest known differences and missing pieces. Each model page's *In short* lists the
rest.

- **hpr's own drag in a whole flight.** Its heights are +10.007% and +10.232% above RocketPy's for
  Valetudo and NDRT 2020, where its drag is well below the examples' ([report][report]). Which
  drag is right is open until real flights ([M2.3](decisions-and-roadmap.md#m2-3), the
  real-flights milestone).
- **Aerodynamics were checked at two speeds only:** Mach 0 for the normal force and centre of
  pressure, and Mach 0.3 for drag. The drag from air pressure on the nose, and on any shoulder (a
  transition that widens toward the tail), is held at its low-speed value. So from about Mach 0.6
  it reads low: the source's own correction for high subsonic speeds gives more. The models are
  documented only to Mach 0.8 ([Aerodynamics](physics/aero.md)).
- **Drag against the RASAero curves** is within 10% in four of seven cases, with the fins and
  surface finish guessed, because the curves don't record them. Cavour power-on is −18.3%, cause
  open. Valetudo's −47.0% and −50.4% are against a table 1.44 times the drag in the OpenRocket
  export for the same rocket ([Aerodynamics](physics/aero.md#verification)).
- **Six fins.** The [normal-force slope](glossary.md#normal-force-slope) of Barrowman's six-fin
  Recruiter is +2.87% above his printed value, and +3.42% on the fins alone, mostly because hpr
  uses a different six-fin rule ([Aerodynamics](physics/aero.md#verification)).
- **Tumbling** is −10 to +19% off its source's own drop tests, and is used far outside the fit
  behind it. That fit comes from small models falling at 5.0 to 6.6 m/s; if Valetudo came down
  tumbling, with nothing deployed, hpr would bring it down at 37 m/s. The default streamer model
  reads +58% fast on a pleated streamer ([Recovery](physics/recovery.md)).
- **Opening loads,** the force on the rocket as a canopy opens, are no safe bound either way. With
  a [filling time](glossary.md#inflation-and-filling-time), hpr leaves out the brief rise of drag
  above its steady value while the canopy fills. Opening at once, it ignores how a light rocket
  slows while the canopy fills. The deployment speed can itself read high: a
  [separated](glossary.md#separation) body falls with no drag until its device opens
  ([Recovery](physics/recovery.md#inflation)).
- **No added mass under a canopy,** the likely cause of the 2.86% drift difference above
  ([Recovery](physics/recovery.md#against-rocketpy)), and the cause of NDRT's +83.060% peak as
  its main opens in the whole flight ([report][report], [case file][ndrt-flight-case]).
- **The path in wind.** In a whole flight in wind, hpr turns into the wind less than RocketPy:
  Juno III's apogee is 228 m from the pad in hpr and 582 m in RocketPy. In calm air they agree
  within a few per cent. The cause is open, and [M2.1](decisions-and-roadmap.md#m2-1)'s landing offset is not met
  ([issue #50][issue-50], [case file][juno-case]).
- **Mach 1.** hpr stops any flight that reaches Mach 1, so Prometheus 2022 can't be compared
  until [M1.8](decisions-and-roadmap.md#m1-8) ([case file][prometheus-case]).
- **Turbulence** is an aircraft model, unvalidated for rockets, and no flight uses it yet
  ([Turbulence](physics/turbulence.md)).
- **Wall and fin mass** may follow different conventions from OpenRocket's, which its documentation
  doesn't state. Measuring a nose cone's wall thickness straight out from the axis, rather than
  square to its surface, changes the wall's volume by 1.4% on a cone three
  [calibres](glossary.md#calibre-caliber) long ([Shapes](physics/shapes.md)).

[ndrt-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-ndrt-2020-nose-to-tail.toml
[plan]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md#principles
[plan-refs]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[rocket-notes]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/rocketpy-rocket-mass.md
[valetudo-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-valetudo.toml
[adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
[adr-023]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-023-predicted-mode-each-codes-own-drag-reported-against-a-target-2026-09-18
[bella-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-bella-lui.toml
[calisto-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-calisto-tests-motor-at-minus-1.373.toml
[issue-50]: https://github.com/nrdptel/hpr-sim/issues/50
[bella-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-bella-lui.toml
[juno-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-juno-iii.toml
[ndrt-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-ndrt-2020-nose-to-tail.toml
[prometheus-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-prometheus-2022-generic-motor.toml
[valetudo-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-valetudo.toml
[juno-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-juno-iii.toml
[adr-025]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-025-the-calm-air-cases-and-juno-iiis-drifts-left-to-the-rail-release-2026-09-18
[juno-calm-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-juno-iii-calm.toml
[ndrt-flight-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-ndrt-2020-nose-to-tail.toml
[prometheus-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-prometheus-2022-generic-motor.toml
