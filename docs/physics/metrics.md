# Flight metrics

## In short

- **What it models:** the numbers a flight is judged by. These are the apogee, the top speed and
  Mach number, the peak [dynamic pressure](../glossary.md#dynamic-pressure) ("max q"), the boost's
  peak acceleration and, kept apart, the opening shock. It also gives the
  [stability margin](../glossary.md#stability-margin) from the rail exit to apogee or the first
  [deployment](../glossary.md#deployment), the
  [ejection delay](../glossary.md#ejection-delay) that would fire the charge at apogee, and each
  landing's latitude and longitude. Anything that didn't happen is `None`, never a zero.
- **Sources:** the margin is [Barrowman's](../glossary.md#barrowmans-method) centre of pressure
  (to Mach 0.8; faster, as [Aerodynamics](aero.md#your-rockets-centre-of-pressure) describes)
  against the [centre of mass](../glossary.md#centre-of-gravity-cg) (the CG), defined as [RocketPy](../glossary.md#rocketpy) defines its static
  margin and stability margin. The peak search is Kiefer's golden-section search
  ([References](#references)). Latitude and longitude come from hpr's WGS 84 conversions
  ([Geodesy](geodesy.md)).
- **How well it is validated:** each number is only as good as the flight it comes from. The
  flight is checked against RocketPy and OpenRocket ([Accuracy](../accuracy.md)), and the metrics
  add no physics of their own. Tests check each against a hand calculation or against hpr's own
  models evaluated directly ([Tests](#tests)). None is validated against a real flight.
- **What it leaves out:**
  - Fin flutter, which has its own page: [Fin flutter](flutter.md). Its margin comes from the
    max q found here.
  - File exports: a later part of the same milestone,
    [M1.10c](../decisions-and-roadmap.md#m1-10c) (CSV, JSON, KML and GeoJSON files).
  - The descent of a separated body, such as a dropped [booster](../glossary.md#booster). It gets
    a landing, but no peaks.
  - A damping ratio: how fast a wobble dies out. Both margins here are static quantities.
  - The margin at the flight's [angle of attack](../glossary.md#angle-of-attack). Both margins
    take the air along the rocket's axis. In hpr's model a rocket meeting the air at an angle, as
    it does leaving a rail in wind, can have a smaller or a larger margin than these, depending on
    where its body's lift acts ([Stability margins](#stability-margins) says why they leave it
    out).

## Why these rules

Loft, this project's predecessor, got four things wrong that these metrics are built to avoid:

- It read peaks off a table of recorded rows and missed them, and it counted the opening shock as
  the boost's peak acceleration ([L34](../decisions-and-roadmap.md#l34)).
- It printed zeros for things that never happened, and never said which height an apogee was
  counted from ([L35](../decisions-and-roadmap.md#l35)).
- It published margins of ±12 to 15 calibres for rockets where a margin has no meaning
  ([L33](../decisions-and-roadmap.md#l33)).
- Its optimum ejection delay depended on the delay flown ([L94](../decisions-and-roadmap.md#l94)).

Each is a numbered lesson with a test ([Tests](#tests)).

## Getting the numbers

Fly a `Simulation` with a
[`FlightMetrics`](../api/hpr_sim/metrics/struct.FlightMetrics.html) watching it. Then ask the
watcher for a [`FlightSummary`](../api/hpr_sim/metrics/struct.FlightSummary.html):

```rust,ignore
let mut metrics = FlightMetrics::new();
let flight = simulation.run(&mut metrics)?;
let summary = metrics.summary(&flight, simulation.environment())?;
let best = hpr_sim::metrics::optimum_delays(&simulation)?;
```

A watcher keeps one flight. Call `metrics.clear()` before it watches another; `summary` refuses a
flight unless the watcher saw each of its steps once.

The example program
[`flight_metrics.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/flight_metrics.rs)
does this for Valetudo, the rocket from [Getting started](../getting-started.md). It flies with a
5 m/s wind from the west and one 1.5 m parachute. The charge fires 6 s after
[burnout](../glossary.md#burnout) (the end of the thrust curve), and the canopy is open 0.5 s
later. Run it with `cargo run --example flight_metrics -p hpr-sim`. It prints:

```text
Valetudo on a K400C, a 1.5 m parachute fired 6 s after burnout, open 0.5 s later
Not yet validated: see the Accuracy page before trusting these numbers.

Heights are the centre of mass's above the launch site; it starts 0.94 m up.
Apogee:                714.0 m above the site (713.0 m of climb) at 11.16 s
Top speed:             112.3 m/s at 3.00 s, 188 m up
Top Mach number:       0.337 at 3.02 s, 190 m up
Max q:                  6667 Pa at 2.99 s, 186 m up
Boost acceleration:     47.2 m/s² at 0.03 s, 1 m up
Opening shock:         153.4 m/s² at 9.76 s, 698 m up

Rail exit:            16.2 m/s at 0.37 s, 17.2° off the oncoming air
At rail exit (0.37 s): static margin 3.09 cal, flight margin 3.09 cal at Mach 0.051
Least static margin:    3.09 cal at 0.37 s, 4 m up
Least flight margin:    3.09 cal at 0.37 s, 4 m up

Optimum delay:        10.6 s after burnout at 3.26 s (flown: 6.0 s), for an apogee of 778.7 m at 13.83 s
Landing:              32.990000° N, 106.967084° W: 272.6 m east and 0.0 m north of the site, at 11.6 m/s, at 78.67 s
```

The 6 s delay is too short:

- The charge fires at 9.26 s, and the canopy opens at 9.76 s: about 4 s (13.83 − 9.76) before
  the apogee it would have reached.
- The rocket still rises 1.4 s more, to an apogee of 714.0 m.
- With the charge held, it would have coasted to 778.7 m at 13.83 s: the apogee of the
  [Getting started](../getting-started.md) flight, whose drogue fires at apogee.
- So the optimum delay is 10.6 s. The opening shock, 153.4 m/s², is three times the boost's
  47.2 m/s², and it is not counted as the boost's peak.
- Don't size a shock cord from the 153.4 m/s²; it is no bound either way. The canopy reaches
  full drag at line stretch, 0.5 s after the charge, with no filling time (hpr's default), so the
  rocket doesn't slow while it fills: that reads high. hpr also leaves out a canopy's drag
  overshoot near the end of filling: that reads low
  ([The opening load](recovery.md#the-opening-load)).

Top speed comes at 3.00 s, before the 3.26 s burnout, because in the thrust curve's last moments
the motor pushes less than drag and gravity pull back.

## Heights

Every height is of the centre of mass: its
[ellipsoidal height](../glossary.md#ellipsoidal-height) above the launch site's, the same as
a flight's `height_above_ground_m`. The centre of mass starts above the site, because the rocket
stands on the rail: 0.94 m for Valetudo. So the summary gives both:

- `apogee.height_above_ground_m`: 714.0 m, above the site.
- `apogee.gain_m`: 713.0 m, the climb from where the centre of mass stood at launch. OpenRocket's
  altitude counts this way. A flight started in the air (`Simulation::run_free`) has no launch
  height, and no climb: `None`.

A flight ends when its centre of mass comes back down to the site's height, so a landing is at
height 0 by this measure.

## Peaks

Each peak comes with its time and its height. The watcher looks at every step the integrator takes
([Time integration and events](integration.md)), not at a table of recorded rows:

1. It evaluates the equations of motion at each step's start, middle and end.
2. It fits a parabola through the three values. When the parabola bends down with its top inside
   the step, the peak may lie inside. A golden-section search then looks for it on the step's
   dense output, the integrator's smooth curve through the step, to a billionth of the time
   since launch (or of 1 s early in the flight).
3. It keeps the largest of what the search finds and the three samples.

Thrust-curve knots, the times where a motor's tabulated thrust changes slope, end steps. So a spike
in the thrust curve is a step's end, and is never averaged away. Loft took its peak acceleration
from a finite difference of the recorded speed, and read it low. Valetudo in a vacuum shows how
much: the watcher's peak matches the hand value from the motor and the masses within the test's
1e-6, and a finite difference of the speed recorded at 100 Hz reads it more than 1% low.

| Peak | What it is |
|---|---|
| Top speed | The centre of mass's speed relative to the ground, after liftoff |
| Top Mach number | The airspeed over the local speed of sound |
| Max q | The largest dynamic pressure, ½ρv² on the airspeed |
| Boost acceleration | The largest acceleration of the nose tip, from liftoff until a recovery device opens |
| Opening shock | The largest acceleration while a device is open, the [opening load](../glossary.md#opening-load) over the mass. It follows hpr's inflation model ([Recovery](recovery.md)) |

The accelerations are of the nose tip, which is the body's origin in hpr's
[frames](frames.md). It differs from the centre of mass's only by the rocket's turning, which is
small in a straight boost, and by the centre of mass's slow drift forward as propellant burns. The accelerations are relative to the
[launch frame](../glossary.md#launch-frame-enu) and straight from the equations of motion, so they
include gravity's pull, as a trajectory's acceleration does. An accelerometer reads something
else: it does not feel gravity.

Nothing is kept while the rocket sits on the pad. A rocket that never lifts off has no peaks at
all: `None`, not zero.

## Stability margins

The margin is how far the [centre of pressure](../glossary.md#centre-of-pressure-cp) lies behind
the centre of mass, in [calibres](../glossary.md#calibre-caliber) of the reference diameter `d`:

```text
margin = (x_cp − x_cg) / d,    x_cp = Σ C_Nα,i x_i / Σ C_Nα,i
```

Here `x` is a station measured aft of the nose tip, and `C_Nα,i` is component `i`'s
[normal-force slope](../glossary.md#normal-force-slope). hpr gives two margins at each instant:

- **Static margin:** the centre of pressure at zero angle of attack and Mach 0, against the centre
  of mass of that instant. This is RocketPy's `static_margin`. It changes only as propellant burns.
- **Flight margin:** the centre of pressure at the flight's own Mach number, still with the air
  along the axis, against the centre of mass of that instant. It is defined as RocketPy's
  `stability_margin` is. RocketPy's `min_stability_margin` takes the least over its whole flight,
  on the rail and in the descent too, at its solver's steps, so it can differ from hpr's least
  below. No page compares hpr's margins with RocketPy's yet. Against OpenRocket, in calm air at
  rod clearance, where a rocket is still slow, hpr's margin at the flight's Mach number is within
  0.016 calibres on the 33 flights of OpenRocket's own examples, and up to 0.11 calibres higher on
  18 flights of private designs, cause not yet traced ([Accuracy](../accuracy.md)). Near Mach 1
  hpr puts the Arcas Robin's centre of pressure up to 2.36 calibres behind the wind tunnel's
  ([Normal force through Mach 1](aero.md#normal-force-through-mach-1)), so there its flight margin
  reads high.

  The Mach number moves the centre of pressure. On a rocket with fins only at the tail it
  generally moves aft up to where supersonic theory starts for the fins, Mach 1.2 or later: their
  slope grows, and from Mach 0.8 their own centre moves aft too. Past that their slope falls and
  the body's lift can grow, and the centre of pressure mostly moves forward again
  ([Your rocket's centre of pressure](aero.md#your-rockets-centre-of-pressure)). Valetudo is slow:
  at its rail exit, at Mach 0.051, both margins read 3.09 calibres.

Both margins leave out the [angle of attack](../glossary.md#angle-of-attack). In hpr,
[body lift](aero.md#body-lift) grows with the angle and acts at each body's side-view centroid, and
the centre of pressure moves toward it. Where that centroid lies ahead of the zero-angle centre of
pressure, the margin at an angle shrinks; on a long body with small fins it can lie behind, and the
margin grows. No test pins either direction. But the angle is not a steady property of
the rocket. Valetudo leaves its rail 17.2° off the oncoming air in the example's 5 m/s crosswind,
and near apogee the angle swings toward 90° as the rocket slows and tips over. If the least margin
followed the angle, it would land wherever hpr chose to stop counting large angles, and hpr models
no fin [stall](../glossary.md#stall) that could say where that is. For the margin at a given
angle, call [`margin`](../api/hpr_sim/metrics/fn.margin.html) with
[`Flow::new(mach, angle_rad, roll_rad)`](../api/hpr_aero/model/struct.Flow.html#method.new).

The watcher keeps both margins in `FlightMetrics::stability()`. The series starts at the rail exit
and ends at apogee or when a recovery device opens, whichever comes first. Before the rail exit the
rail holds the rocket, so its margin says nothing about how it flies.

The summary's least margins cover the same span. Each is looked for inside every step, as a peak
is ([Peaks](#peaks)): wherever the margin is defined at a step's start, middle and end and the
parabola through them bends up with its bottom inside the step, a golden-section search finds the
bottom. A later least replaces an earlier one only when lower by more than rounding, so a flat
least keeps its first time. A margin with a sharp corner
near a step's end can still hide from the parabola.

- On four test flights, and in the example, both leasts come at the rail exit, where the rocket is
  heaviest and its centre of mass furthest aft. Flown in steps of at most 1 ms, those flights give
  the same leasts to a millionth of a calibre.
- On a two-stage test flight the least flight margin comes inside a step, after the split. There
  it matches a scan of every step at 201 points to 1e-7 calibres, and is never above it.

After a powered separation the margins are the sustainer's, in its own diameter. The split has two
entries in the series: the stack's, then the sustainer's.

### When there is no margin

As the net slope `Σ C_Nα,i` goes to zero, the air's loads become a pure couple: a turning moment
with no line of action. The quotient `x_cp` then runs away. That is how Loft came to publish
margins of ±12 to 15 calibres.

hpr judges the net slope against the sum of its terms' sizes:

```text
κ = Σ |C_Nα,i| / Σ C_Nα,i
```

Each part the model adds up acts at a station `x_i` on the rocket, within its length `L`. The
centre of pressure then lies within `κ L` of every station. So a fractional error `ε` in one
part's slope moves the centre of pressure by at most about `ε κ² L` (to first order). A part that
is a pure couple on its own (below) has no station, and `κ` doesn't count it.

- A rocket whose parts all push the same way has `κ = 1`, and a 1% error moves its centre of
  pressure by at most 1% of its length. All 13 designs in
  [`validation/designs/`](https://github.com/nrdptel/hpr-sim/tree/main/validation/designs) stay
  below 1.5 at Mach 0, 0.3, 0.8, 1.2 and 2 and at angles of attack of 0°, 5°, 10° and 20°.
- At `κ = √10 = 3.16`, a 1% error in one slope can move the centre of pressure by a tenth of the
  rocket. Past that, or when the net slope is not positive, hpr gives no margin and no centre of
  pressure: `None`. The limit is a chosen bound on that sensitivity, not a measurement.

The pitch-moment slope about the centre of mass is always given, because it stays finite:

```text
C_mα = −(Σ C_Nα,i x_i − x_cg Σ C_Nα,i) / d
```

A negative `C_mα` turns the nose back into the wind. With a margin defined, `C_mα = −C_Nα · margin`.

A worked case, with no fins, pinned by a test:

- The rocket is a conical nose 0.3 m long of radius `R` = 0.05 m, a 0.5 m tube, and a conical
  boattail 0.4 m long, narrowing to a radius `r`. So `d` = 0.1 m.
- Barrowman gives the nose a slope of 2 at 0.2 m. The boattail gets `2((r/R)² − 1)`, at
  `0.8 + (0.4/3)(1 + 1/(1 + R/r))` m.
- So `κ = 2/ρ² − 1`, with `ρ = r/R`. The margin is given for `ρ ≥ 0.6932`, an aft radius of
  34.66 mm or more.
- The centre of mass is at 0.5 m.

| Boattail's aft radius | Net slope | κ | Margin | `C_mα` |
|---|---|---|---|---|
| 40 mm | 1.28 | 2.13 | −7.46 cal | +9.55 |
| 35 mm | 0.98 | 3.08 | −11.2 cal | +10.98 |
| 34.5 mm | 0.952 | 3.20 | none (the quotient: −11.7 cal) | +11.11 |
| 21.5 mm | 0.370 | 9.82 | none (the quotient: −37.1 cal) | +13.72 |
| 5 mm | 0.020 | 199 | none (the quotient: −741 cal) | +14.82 |

With no fins, every one of these rockets is unstable: its centre of pressure lies ahead of its
centre of mass, and `C_mα` is positive. The cut is not about how large the margin is. The 35 mm and
34.5 mm rows have margins near −11 calibres; what differs is whether the quotient can be trusted.

A component can be a pure couple on its own. A step down in radius followed by a flare back up
cancels its own slope and still turns the rocket. `C_mα` keeps its moment, which a test also
checks.

## Optimum ejection delay

The optimum delay is the time from a motor's burnout to apogee: the delay that fires its charge at
the top. [`optimum_delays`](../api/hpr_sim/metrics/fn.optimum_delays.html) flies the rocket again
with every recovery charge held, so that it coasts to the apogee it would reach untouched. So the
answer belongs to the rocket, its motors and its air, not to the delay flown. A charge that fires
too early cuts the coast short. Loft's optimum then came out too short as well, which advised an
even shorter delay. hpr gives the same optimum for delays of 1 s and 20 s.

- A [separation](../glossary.md#separation) with nothing ahead of it left to burn is part of the
  recovery, so it is held too. A two-stage rocket that separates some seconds after its last burnout gets
  the same optimum for a 3 s delay as for a 30 s one.
- A powered separation still happens. The motors in the booster it drops have no optimum: their
  charges fire in the booster, which never reaches the [sustainer's](../glossary.md#sustainer)
  apogee.
- A motor that burns out after the apogee, or never lights, has none either. A flight that never
  reaches an apogee (it never lifts off, or hits its time cap) gives `None`.

## Landing points

A landing is where the centre of mass came back down to the site's height. It is given as a WGS 84
latitude and longitude, as east and north metres from the site in its local frame, and as the speed
at the ground hit. The flight's own landing is the stack's (the whole rocket before any
separation), or after a powered separation the sustainer's. Each separated body that lands has its
own landing, with its body number: 0 keeps the nose, and 1 is the stages aft of the split.

A flight that did not land has no landing and no ground-hit speed: `None`. A test checks this for a
flight stopped by its time cap, and the JSON it writes, where the missing values are `null`.

## Tests

In `crates/hpr-sim/src/metrics.rs`, unless named otherwise:

| Test | What it pins |
|---|---|
| `static_margin_undefined_when_cn_alpha_near_zero` | The boattail table above, against Barrowman by hand, to 1e-9 relative, on both sides of the limit ([L33](../decisions-and-roadmap.md#l33)) |
| `ordinary_rockets_keep_their_margin` | All 13 validation designs keep their margin at Mach 0, 0.3, 0.8, 1.2 and 2 and at 0°, 5°, 10° and 20°, with `κ` below 1.5 |
| `a_pure_couple_keeps_its_moment` | The step and flare above: `C_mα` against the hand value, to 1e-9 |
| `a_normal_force_table_gives_its_own_margin` | A table's margin and `C_mα` against its own numbers |
| `peak_acceleration_is_analytic_and_excludes_opening_shock` | The boost's peak in a vacuum against the hand value, to 1e-6; a 100 Hz finite difference reads it more than 1% low; an opening shock over three times the boost's is kept apart ([L34](../decisions-and-roadmap.md#l34)) |
| `unlanded_flight_has_no_ground_hit_speed_and_outputs_name_datum` | `None` and `null` for an unlanded flight; the launch height against the rail's geometry, to 1e-9 relative ([L35](../decisions-and-roadmap.md#l35)) |
| `optimum_delay_independent_of_flown_delay` | Delays of 1 s and 20 s give the same optimum, equal to a flight with no recovery ([L94](../decisions-and-roadmap.md#l94)) |
| `peaks_are_refined_inside_steps` | On three rockets, max q and top Mach are above every row of a 1 ms record, and the record's best row is within 0.01% of them; on Valetudo max q comes before top speed, and top speed before top Mach |
| `landings_are_placed_on_the_ellipsoid` | A landing more than 100 m downwind, against the radii of curvature at the site, to second order |
| `stability_is_kept_from_rail_exit_to_apogee` | The series' ends; the static margin at the rail exit and, in a crosswind, the flight margin at the flight's Mach number, against the model and the masses directly, to 1e-12; the least flight margin is at or below every entry |
| `least_margins_do_not_depend_on_where_steps_end` | Valetudo in calm air off a vertical and an 84° rail and in a crosswind, and Prometheus: both leasts are at the rail exit, and agree with steps of at most 1 ms to 1e-6 calibres |
| `a_least_margin_between_step_ends_is_found` | A margin of `2 + (t − 0.37)²` across a step: the search finds 2 at 0.37 s; a margin falling across the step, or undefined in its middle, is not searched |
| `a_summary_needs_the_flight_it_watched` | A watcher refuses a flight when it missed some of its steps or also watched another flight, naming the steps it saw, and sums up one that ended without a step; a flight started in the air has no launch height, and one started on its way down keeps no stability |
| `staging::tests::metrics_follow_a_powered_separation` | Both landings; the sustainer's diameter after the split, with its own entry there; both leasts against a scan of every step at 201 points, to 1e-7 calibres, the flight margin's inside a step; no optimum for the booster's motor |
| `staging::tests::a_held_flight_holds_a_separation_after_the_last_burnout` | A split after the last burnout is held: the same optimum for delays of 3 s and 30 s |

## References

- J. Kiefer, "Sequential minimax search for a maximum", *Proceedings of the American Mathematical
  Society* 4(3), 502–506, 1953. The golden-section search.
- J. S. Barrowman and J. A. Barrowman, "The theoretical prediction of the center of pressure",
  NARAM-8 research and development report, 1966. The slopes and stations, as
  [Aerodynamics](aero.md) uses them.

The decision behind these choices is [ADR-077][adr-077], from the
[M1.10a milestone](../decisions-and-roadmap.md#m1-10a).

[adr-077]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-077-flight-metrics-peaks-on-the-dense-output-margins-only-where-they-mean-something-and-none-for-what-didnt-happen-2026-09-26
