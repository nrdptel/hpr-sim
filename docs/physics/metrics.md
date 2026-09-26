# Flight metrics

## In short

- **What it models:** the numbers a flight is judged by, for reading a flight without writing your
  own peak-finding code. These are the apogee, the top speed and Mach number, the peak
  [dynamic pressure](../glossary.md#dynamic-pressure) ("max q"), the boost's peak acceleration
  and, kept apart, the opening shock. It also gives the
  [stability margin](../glossary.md#stability-margin) from the rail exit to apogee, the
  [ejection delay](../glossary.md#ejection-delay) that would fire the charge at apogee, and each
  landing's latitude and longitude. Loft, this project's predecessor, read peaks off a table and
  missed them, counted the opening shock as the boost's peak, printed zeros for things that never
  happened, and published margins of ±12 to 15 calibres where a margin has no meaning.
- **Sources:** the margin is Barrowman's centre of pressure against the centre of mass, defined as
  RocketPy defines its static margin and stability margin. The peak search is Kiefer's
  golden-section search ([References](#references)). Latitude and longitude come from hpr's WGS 84
  conversions ([Geodesy](geodesy.md)).
- **How well it is validated:** each number is only as good as the flight it comes from. The
  flight is checked against RocketPy and OpenRocket ([Accuracy](../accuracy.md)), and the metrics
  add no physics of their own. Tests pin each one against a hand calculation, listed under
  [Tests](#tests). None is validated against a real flight.
- **What it leaves out:**
  - Fin flutter and file exports. Both are later parts of the same milestone:
    [M1.10b](../decisions-and-roadmap.md#m1-10b) (flutter) and
    [M1.10c](../decisions-and-roadmap.md#m1-10c) (CSV, JSON, KML and GeoJSON files).
  - The descent of a separated body (a dropped booster, say). It gets a landing, but no peaks.
  - A damping ratio, the other sense of "dynamic stability".

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

The example program
[`flight_metrics.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/flight_metrics.rs)
does this for Valetudo, the rocket from [Getting started](../getting-started.md). It flies with a
5 m/s wind from the west and one 1.5 m parachute, fired 6 s after the motor burns out. Run it
with `cargo run --example flight_metrics -p hpr-sim`. It prints:

```text
Valetudo on a K400C, a 1.5 m parachute fired 6 s after burnout
Not yet validated: see the Accuracy page before trusting these numbers.

Heights are the centre of mass's above the launch site; it starts 0.94 m up.
Apogee:                714.0 m above the site (713.0 m of climb) at 11.16 s
Top speed:             112.3 m/s at 3.00 s, 187 m up
Top Mach number:       0.337 at 3.02 s, 190 m up
Max q:                  6667 Pa at 2.99 s, 186 m up
Boost acceleration:     47.2 m/s² at 0.03 s, 1 m up
Opening shock:         153.4 m/s² at 9.76 s, 698 m up

At rail exit (0.37 s): static margin 3.09 cal, flight margin 0.69 cal at Mach 0.051
Least static margin:    3.09 cal at 0.37 s, 4 m up
Least flight margin:    0.69 cal at 0.37 s, 4 m up

Optimum delay:        10.6 s after burnout at 3.26 s (flown: 6.0 s)
Landing:              32.990000° N, 106.967084° W: 272.6 m east and 0.0 m north of the site, at 11.6 m/s, at 78.67 s
```

The 6 s delay is too short. The parachute opens at 9.76 s, 1.4 s before the apogee it cuts short,
at 714 m. The optimum delay says the charge should have fired 10.6 s after burnout. The opening
shock, 153 m/s², is three times the boost's 47 m/s², and it is not counted as the boost's peak.

## Heights

Every height is the centre of mass's ellipsoidal height above the launch site's, the same as a
flight's `height_above_ground_m`. The centre of mass starts above the site, because the rocket
stands on the rail: 0.94 m for Valetudo. So the summary gives both:

- `apogee.height_above_ground_m`: 714.0 m, above the site.
- `apogee.gain_m`: 713.0 m, the climb from where the centre of mass started. OpenRocket's
  altitude counts this way.

A flight ends when its centre of mass comes back down to the site's height, so a landing is at
height 0 by this measure.

## Peaks

Each peak comes with its time and its height. The watcher looks at every step the integrator takes
(see [Time integration and events](integration.md)), not at a table of recorded rows. It
evaluates the equations of motion at each step's start, middle and end. When the middle is higher
than both ends, the peak lies inside the step. A golden-section search on the step's dense output
then finds it, to a billionth of the flight's clock.

Thrust-curve knots end steps, so a spike in the thrust curve is a step's end and is never averaged
away. Loft took its peak acceleration from a finite difference of the recorded speed, and read it
low. Valetudo in a vacuum shows how much. The watcher's peak matches the hand value from the
motor and the masses to 1.6e-7. A finite difference of the speed recorded at 100 Hz reads it
1.3% low.

| Peak | What it is |
|---|---|
| Top speed | The centre of mass's speed relative to the ground, after liftoff |
| Top Mach number | The airspeed over the local speed of sound |
| Max q | The largest dynamic pressure, ½ρv² on the airspeed |
| Boost acceleration | The largest acceleration of the nose tip, from liftoff until a recovery device opens |
| Opening shock | The largest acceleration while a device is open. It follows hpr's inflation model ([Recovery](recovery.md)) |

The accelerations are of the nose tip (the body's origin), relative to the launch frame, and
straight from the equations of motion. They include gravity's pull, as a trajectory's acceleration
does. An accelerometer reads something else: it does not feel gravity.

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

- **Static margin:** the centre of pressure with the air straight along the axis at Mach 0,
  against the centre of mass of that instant. This is RocketPy's `static_margin`. It changes only
  as propellant burns.
- **Flight margin:** the centre of pressure in the flight's own air, at its Mach number and
  [angle of attack](../glossary.md#angle-of-attack). RocketPy's `stability_margin` is this at zero
  angle of attack. hpr's body lift grows with the angle and acts ahead of the fins, so in a
  crosswind the flight margin is lower. Valetudo leaves its rail at 16 m/s into a 5 m/s crosswind,
  about 17°: its flight margin there is 0.69 calibres against a static 3.09.

The watcher keeps both from the rail exit to apogee. On the rail the rail holds the rocket. There,
the slow climb through the wind gives angles of attack near 90°, where a margin says nothing about
stability.

### When there is no margin

As the net slope `Σ C_Nα,i` goes to zero, the air's loads become a pure couple: a turning moment
with no line of action. The quotient `x_cp` then runs away. That is how Loft came to publish
margins of ±12 to 15 calibres.

hpr judges the net slope against the sum of its terms' sizes:

```text
κ = Σ |C_Nα,i| / Σ C_Nα,i
```

An error `ε` in one component's slope moves the centre of pressure by up to `ε κ L`, for a rocket
of length `L`.

- A rocket whose parts all push the same way has `κ = 1`. A boattail's negative slope raises it a
  little.
- At `κ = 10`, a 1% error in one slope can move the centre of pressure by a tenth of the rocket.
  Past that, or when the net slope is not positive, hpr gives no margin and no centre of pressure:
  `None`.

The pitch-moment slope about the centre of mass is always given, because it stays finite:

```text
C_mα = −Σ C_Nα,i (x_i − x_cg) / d
```

A negative `C_mα` turns the nose back into the wind. With a margin defined, `C_mα = −C_Nα · margin`.

A worked case, with no fins, pinned by a test:

- The rocket is a conical nose 0.3 m long of radius `R` = 0.05 m, a 0.5 m tube, and a conical
  boattail 0.4 m long, narrowing to a radius `r`.
- Barrowman gives the nose a slope of 2 at 0.2 m, and the boattail `2((r/R)² − 1)`.
- So `κ = 2/ρ² − 1`, with `ρ = r/R`. The margin is given for `ρ ≥ √(2/11) = 0.426`.

| Boattail's aft radius | Net slope | κ | Margin | `C_mα` (CG at 0.5 m) |
|---|---|---|---|---|
| 40 mm | 1.28 | 2.13 | −7.46 cal | +9.55 |
| 21.5 mm | 0.370 | 9.82 | −37.1 cal | +13.72 |
| 21.0 mm | 0.353 | 10.34 | none (the quotient: −39.1 cal) | +13.79 |
| 5 mm | 0.020 | 199 | none (the quotient: −741 cal) | +14.82 |

With no fins, every one of these rockets is unstable: its centre of pressure lies ahead of its
centre of mass, and `C_mα` is positive. The cut is not about how large the margin is. Both
21.5 mm and 21.0 mm give margins near −38 calibres; what differs is whether the quotient can be
trusted.

## Optimum ejection delay

The optimum delay is the time from a motor's burnout to apogee: the delay that fires its charge at
the top. [`optimum_delays`](../api/hpr_sim/metrics/fn.optimum_delays.html) flies the rocket again
with every recovery charge held, so that it coasts to its true apogee. So the answer belongs to
the rocket, its motors and its air, not to the delay flown. A charge that fires too early cuts the
coast short. Loft's optimum then came out too short as well, which advised an even shorter delay.
hpr gives the same optimum for delays of 1 s and 20 s.

Each motor that burns out before that apogee gets its own optimum. A separation after the last
burnout ends the stack's flight. If it comes before apogee, there is no apogee, and so no optimum
(`None`).

## Landing points

A landing is where the centre of mass came back down to the site's height. It is given as a WGS 84
latitude and longitude, as east and north metres from the site in its local frame, and as the speed
at the ground hit. The flight's own landing is the stack's, or after a powered separation the
sustainer's. Each separated body that lands has its own landing, with its body number.

A flight that did not land has no landing and no ground-hit speed: `None`. That covers a flight
stopped by its time cap, and a stack that came apart. A test checks both the time-capped case and
the JSON it writes, where the missing values are `null`.

## Tests

In `crates/hpr-sim/src/metrics.rs`:

| Test | What it pins |
|---|---|
| `static_margin_undefined_when_cn_alpha_near_zero` | The boattail table above, against Barrowman by hand, to 1e-9 ([Loft lesson L33](../decisions-and-roadmap.md#l33)) |
| `peak_acceleration_is_analytic_and_excludes_opening_shock` | The boost's peak in a vacuum against the hand value to 1e-6; a 100 Hz finite difference reads it 1.3% low; an opening shock over three times the boost's is kept apart ([L34](../decisions-and-roadmap.md#l34)) |
| `unlanded_flight_has_no_ground_hit_speed_and_outputs_name_datum` | `None` for an unlanded flight; the starting height against the rail's geometry to 1e-9 m ([L35](../decisions-and-roadmap.md#l35)) |
| `optimum_delay_independent_of_flown_delay` | Delays of 1 s and 20 s give the same optimum, equal to a flight with no recovery ([L94](../decisions-and-roadmap.md#l94)) |
| `peaks_are_refined_inside_steps` | Each peak is above every row of a 1 ms record and within 1e-4 of the best row; max q comes before top speed, and top speed before top Mach |
| `landings_are_placed_on_the_ellipsoid` | A landing 270 m downwind against the radii of curvature at the site, to second order |
| `stability_is_kept_from_rail_exit_to_apogee` | The series' ends, and the static margin at the rail exit against the model and the masses directly |

## References

- J. Kiefer, "Sequential minimax search for a maximum", *Proceedings of the American Mathematical
  Society* 4(3), 502–506, 1953. The golden-section search.
- J. S. Barrowman and J. A. Barrowman, "The theoretical prediction of the center of pressure",
  NARAM-8 research and development report, 1966. The slopes and stations, as
  [Aerodynamics](aero.md) uses them.

The decision behind these choices is [ADR-077][adr-077], from the
[M1.10a milestone](../decisions-and-roadmap.md#m1-10a).

[adr-077]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-077-flight-metrics-peaks-on-the-dense-output-margins-only-where-they-mean-something-and-none-for-what-didnt-happen-2026-09-26
