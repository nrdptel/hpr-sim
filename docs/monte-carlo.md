# Monte Carlo dispersion

No two flights of one rocket are the same. The motor burns a little hotter or cooler than its
label, the rocket weighs a few grams more than the plan, the wind is not the forecast's. A
[Monte Carlo](glossary.md#monte-carlo) run flies the rocket hundreds or thousands of times. Each
time it draws the uncertain inputs afresh around their planned (nominal) values, and the run shows
how far the apogee and the landing spread, and draws the ellipse the landings fall in. This page
runs one, says what each
[dispersion](glossary.md#dispersion) does to a flight, and how to choose the numbers. It needs some
Rust and follows on from [The builder](the-builder.md).

> **How far to trust it.** The sampling is tested; the spread it gives is only as good as the
> uncertainties you give it and hpr-sim's flight models, which are not yet validated against real
> flights ([Accuracy](accuracy.md)).
>
> - **Tested:** the same [seed](glossary.md#seed) gives the same run, bit for bit, however many
>   flights it has and however many threads fly them; a run with no dispersion flies the nominal
>   flight in every sample; each dispersion moves its input as the table below says; a dispersed
>   motor keeps its [specific impulse](glossary.md#specific-impulse); failed flights are counted
>   ([`montecarlo.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/montecarlo.rs)'s
>   tests). The [landing ellipses](#landing-ellipses) are tested against normal spreads with known
>   answers.
> - **Not checked:** whether the spread matches the spread of real flights. No measured set of
>   repeated flights has been compared yet.
> - **Left out:** correlations between inputs, distributions other than the
>   [normal](glossary.md#normal-distribution), and the inputs listed under
>   [What is not dispersed](#what-is-not-dispersed). A run flies what `Flight::builder` sets up, so
>   staged flights, separations and events of your own can't be part of one yet.

## A run of 200 flights

The example program
[`crates/hpr/examples/monte_carlo.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/monte_carlo.rs)
takes the 54 mm rocket of [The builder](the-builder.md) on a Cesaroni H54
([motor designation](glossary.md#motor-designation) 168H54-10A), at Spaceport America in a forecast
wind of 4 m/s from the west, off a rail leaned 5° into the wind. It disperses the rocket's mass, its
centre of mass, its drag, the motor's impulse and burn time, the wind and the rail, and flies 200
flights. Run it from a copy of the repository with:

```text
cargo run --example monte_carlo -p hpr
```

The heart of it:

```rust,ignore
let launch = Flight::builder(&rocket, &environment, 1.8)
    .inclination_deg(85.0)
    .heading_deg(270.0);
let dispersion = Dispersion {
    dry_mass_sd_fraction: 0.02,              // 2% of the mass without the motor
    cg_sd_m: 0.005,                          // 5 mm
    drag_sd_fraction: 0.05,                  // 5% of the drag coefficient
    impulse_sd_fraction: 0.03,               // 3% of the motor's total impulse
    burn_time_sd_fraction: 0.02,             // 2% of its burn time
    wind_speed_sd_fraction: 0.25,            // 25% of the wind's speed
    wind_heading_sd_rad: 15_f64.to_radians(),
    rail_elevation_sd_rad: 1_f64.to_radians(),
    rail_azimuth_sd_rad: 2_f64.to_radians(),
    ..Dispersion::default()
};
let monte_carlo = MonteCarlo::new(launch.inputs()?, dispersion)?;
let run = monte_carlo.run(2026, 200);          // seed 2026, 200 flights
let apogee = run.apogee()?;                    // the apogees' spread
let landing = run.landing()?;                  // where they landed
let ellipse = landing
    .prediction_ellipse(0.95)?                 // where the next flight lands, 95 times in 100
    .ok_or("too few landings")?;
```

It prints:

<!-- quote: crates/hpr/examples/monte_carlo.output.txt -->
```text
My 54 mm rocket on a 168H54-10A, from a 1.8 m rail at 85°, heading west into a 4 m/s west wind
200 flights, seed 2026: 0 failed
Not yet validated: see the Accuracy page before trusting these numbers.

                       nominal     mean  std dev       5%   median      95%
apogee (m)              1113.3   1119.8     47.4   1040.5   1121.3   1198.8
landing distance (m)     662.4    660.3    208.9    329.3    670.5    996.5
landing east (m)         662.4    626.1    209.6    274.9    635.5    957.7

Landing ellipses, centred 626 m east and 23 m south of the pad:
                       semi-major  semi-minor  heading  landings inside
50%                         253 m       239 m     131°            48.0%
95%                         525 m       497 m     131°            94.5%
95%, the next flight        532 m       503 m     131°            95.5%

Reached 1,100 m: 65.5% of the flights
Apogee with 5% less drag: +29 m; with 5% more: -27 m
```

How to read it:

- **Nominal** is the one flight with every input at its planned value.
- **Landing distance** is how far from the pad the rocket lands; **landing east** is the eastward,
  downwind, part of it. The nominal flight lands due east, so the two are equal. In the run the
  wind's heading has a standard deviation of 15°, which pushes some flights north or south, so the mean landing east
  is smaller than the mean distance.
- **Std dev** is the [standard deviation](glossary.md#standard-deviation). If the spread is normal,
  about two values in three lie within one standard deviation of the mean. **5%** and **95%** are
  [percentiles](glossary.md#percentile): one flight in twenty went lower than the 5% value, one in
  twenty higher than the 95% value. So nine flights in ten reached between about 1,040 and 1,200 m.
- **The mean is not the nominal flight.** The mean apogee is 6.5 m above the nominal one. With 200
  flights the mean itself is uncertain by about 47.4 / √200 = 3.4 m (its
  [standard error](glossary.md#standard-error)), so 6.5 m is 1.9 standard errors: chance can
  account for most of it. Part is that the apogee doesn't respond evenly: 5% less drag gains 29 m
  and 5% more loses 27 m, so an even spread of drag lifts the mean apogee by about 1 m.
- **The landing spreads far more than the apogee.** The landing distance's standard deviation is
  about a third of its mean (209 of 660 m); the apogee's is 4% (47 of 1,120 m). Under the parachute
  the drift is the wind's speed times the time in the air, and the wind's speed is the most
  uncertain input here.

## Landing ellipses

A [landing ellipse](glossary.md#landing-ellipse) is the outline a range safety officer or a
competition asks for: an area on the ground that the rocket lands inside, say, 95 times in 100.
Comparing it with the field's boundary says whether the field is big enough for this rocket in this
wind. For that check, use the *next-flight* ellipse below ([`Scatter::prediction_ellipse`]).

The ellipse assumes the landings follow a [normal
distribution](glossary.md#normal-distribution), and it is only as good as the run's landings,
which carry the flight models' errors and those of the dispersions you chose; neither has been
compared with real flights yet. The section ends with how to tell when the landings aren't normal.

hpr-sim draws the ellipse from the run's landing points ([`Run::landing`], then
[`Scatter::ellipse`]) in three steps. Its *semi-major* and *semi-minor* axes are its half-lengths
along its long and its short direction.

1. **The centre** is the landings' mean: here 626 m east and 23 m south of the pad.
2. **The axes.** The landings' [covariance](glossary.md#covariance) gives the direction they spread
   most, the *major axis*, and the direction across it, the *minor axis*, with a standard deviation
   along each. Here those are about 214.6 m and 203.0 m. The spread is nearly round, because the wind's
   uncertain heading (15°) spreads the landings sideways about as much as its uncertain speed (25%)
   spreads them downwind.
3. **The size.** If the landings follow a two-dimensional [normal
   distribution](glossary.md#normal-distribution), the ellipse reaching `k` standard deviations
   along each axis holds the share `p = 1 − e^(−k²/2)` of them. So the ellipse of level `p` has
   `k = √(−2 ln(1 − p))`:

   | Level `p` | 50% | 90% | 95% | 99% |
   |---|---|---|---|---|
   | Scale `k` | 1.177 | 2.146 | 2.448 | 3.035 |

   The 95% ellipse's semi-axes are 2.448 × 214.6 m = 525 m and 2.448 × 203.0 m = 497 m.

In two dimensions it takes more standard deviations to hold 95% than in one (2.448 against 1.960),
because a landing can stray in two directions at once.

**Heading** is the major axis's direction, clockwise from north, between 0° and 180°: here 131°,
running from north-west to south-east. With axes this close to equal the heading means little: a
few landings more or less could turn it a long way. A circle has no heading at all; hpr-sim then
reports 90°, east.

**The next flight.** The mean and covariance of 200 flights are only estimates, so an ellipse
drawn from them holds a little less than its level of the flights still to come. For normal
landings [`Scatter::prediction_ellipse`] allows for that exactly. Its `k` comes from Hotelling's
`T²` distribution, the one that accounts for the mean and the spread both being estimated from the
same flights: `k² = ((n² − 1)/n)((1 − p)^(−2/(n − 2)) − 1)` for `n` landings. With
200 landings its axes are 1.3% longer (532 m against 525 m); with 10 they would be 36% longer. Use
it to answer "will my next flight land in the field?".

**Landings inside** counts the run's landings each ellipse really holds
([`Scatter::share_inside`]). Here they are 48.0%, 94.5% and 95.5%. Those are within the
[standard error](glossary.md#standard-error) of a share from a run of 200 (about 3.5 percentage
points at 50%, 1.5 at 95%), so the landings are consistent with a normal spread, though that
doesn't prove it. With few landings the shares run high, because the ellipse is fitted to the
same points: with three landings, even the 50% ellipse holds all three. If the share is far from
the level in a run of hundreds of flights, the landings aren't normal: an uncertain wind
heading in a strong wind spreads them along an arc, and an ellipse is then the wrong shape. Look at
the points themselves ([`Scatter::points`]). A failed flight has no landing; it counts as outside
for the lower bound and inside for the upper ([Failed flights are counted](#failed-flights-are-counted)).

> **How far to trust it.** The ellipse math is tested against normal spreads whose answers are
> known exactly
> ([`ellipse.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/ellipse.rs)'s
> tests):
>
> - The scale matches the NIST/SEMATECH handbook's chi-square table and its closed form.
> - The axes and heading of turned, stretched covariances come back to 1e-14.
> - Integrating a normal density over its ellipse gives the level to 1e-12.
> - 100,000 points drawn from a known normal spread give back its covariance, and land inside each
>   ellipse at its level, within five standard errors.
> - A new point lands inside the next-flight ellipse of 3, 5 or 20 others at its level, within
>   five standard errors, and inside the plain ellipse visibly less often.
>
> The checks can fail: a long, narrow 95% ellipse turned 6° off its axes holds 90.6%.

## What each dispersion does

Each dispersion is a standard deviation: zero, the default, leaves its input at the nominal value.
For each flight hpr-sim draws a standard normal number `z` (mean 0, standard deviation 1) for each
input and moves the input by `σ z`, with `σ` the standard deviation you gave. A rocket built of
more than one stage, flown together and never separated, gets a draw for each stage; each motor
and each parachute gets its own draw too.

| Field | What each flight flies |
|---|---|
| `dry_mass_sd_fraction` | Each stage's mass without motors times `1 + σ z`. Its moments of inertia scale with it. |
| `cg_sd_m` | Each stage's [centre of mass](glossary.md#centre-of-gravity-cg) moved `σ z` metres towards the tail (towards the nose when negative). |
| `drag_sd_fraction` | The rocket's zero-lift [drag coefficient](glossary.md#drag-coefficient) times `1 + σ z`, whether hpr-sim's own, a drag table's or a [drag model's](custom-models.md). |
| `impulse_sd_fraction` | Each motor's thrust and propellant mass, both times `1 + σ z`. Its [total impulse](glossary.md#total-impulse) changes and its specific impulse doesn't, as for a motor that holds a little more or less of the same propellant. |
| `burn_time_sd_fraction` | Each motor's thrust curve stretched in time by `1 + σ z` and its thrust divided by the same: a longer, softer burn of the same impulse. |
| `ejection_delay_sd_s` | Each motor's [ejection delay](glossary.md#ejection-delay) plus `σ z` seconds, never below zero. |
| `wind_speed_sd_fraction` | The wind at every height times `1 + σ z`, never below calm. A calm forecast stays calm. |
| `wind_heading_sd_rad` | The wind at every height turned `σ z` clockwise: the forecast's direction, give or take. |
| `rail_elevation_sd_rad` | The rail's angle above the horizon plus `σ z`. Past vertical, it leans the other way. |
| `rail_azimuth_sd_rad` | The rail's heading plus `σ z`, clockwise. |
| `deployment_lag_sd_s` | Each recovery device's lag after its trigger plus `σ z` seconds, never below zero. |

Three cases need a word:

- **A draw that makes an input impossible fails that flight.** With a 60% mass spread, a draw of
  `z` below −1.67 gives a negative mass; a rail drawn below the horizon is another. That flight is
  kept in the run as failed, with its reason, and counted (next section). The two delays and the
  wind's speed are the exceptions: a charge can't fire before its event and a wind can't blow at
  less than calm, so a draw below zero is flown as zero.
- **A [cluster](glossary.md#cluster) is one draw.** hpr-sim holds a cluster as one motor in a mount
  with several tubes, so all its motors get the same impulse and burn time.
- **A vertical rail leans along one line.** The elevation is dispersed in the plane of the rail's
  heading, so on a vertical rail with only its elevation dispersed every flight leans towards or
  away from that heading, never sideways. RocketPy disperses its rail the same way, an inclination
  and a heading (`rocketpy/stochastic/stochastic_flight.py:21-24`, version 1.13.0). Disperse the
  heading too for leans in every direction.

## Failed flights are counted

A run never drops a flight. [`Run::failed`] lists those that failed, saying whether the draw made
an impossible input or the flight refused it. A spread like [`Run::apogee`] counts every flight
tried: `attempted()` is the run's size, `count()` the flights that gave a value, and `missing()`
the rest. Its mean and percentiles are over the flights that gave a value, so many failures can
bias them: check `missing()` first.

A share such as "reached 1,100 m" is given as two bounds, `share_at_least(1100.0)`:

- `low` counts a failed flight as not reaching it;
- `high` counts it as reaching it.

With no failures the two are equal. With some, the truth lies between them, and a wide gap says the
run can't answer the question.

## The same seed, the same run

Every number a flight draws comes from its own stream of random numbers, picked out by the run's
seed, the flight's number in the run and the input it is for. So:

- flight 37 of a run is the same flight in a run of 100 or of 10,000;
- a run flown on one thread or on twelve is the same, bit for bit (with the `parallel` feature,
  [`run_parallel`]);
- turning a dispersion on or off doesn't change what the other inputs draw.

To fly on several threads, turn on the `parallel` feature where your program depends on hpr, as
[Using it from your own program](api.md#using-it-from-your-own-program) describes, and call
`run_parallel` instead of `run`:

```toml
[dependencies]
hpr = { git = "https://github.com/nrdptel/hpr-sim", rev = "<commit>", features = ["parallel"] }
```

On another platform (operating system and processor) a draw can differ in its last binary digit,
because the normal numbers use the platform's logarithm; a run then agrees to many digits, not to
the bit. The decision record is [ADR-134](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-134-monte-carlo-dispersion-independent-normals-one-stream-per-sample-and-input-2026-10-01).

**Run time.** A subsonic Level 2 flight takes about 1 ms in a release build in hpr-sim's
benchmarks ([Performance](https://github.com/nrdptel/hpr-sim/blob/main/docs/perf.md)). But each
flight of a run builds its own aerodynamic model, and a rocket that passes Mach 1.2 builds
supersonic tables that take 0.3 to 0.7 s. So 1,000 flights of a supersonic rocket can take several
minutes on one thread, until [M6.1d](decisions-and-roadmap.md#m6-1d) shares that work between
flights.

## Choosing the numbers

The spread a run gives is the spread you put in. Some places to start:

- **Motor impulse.** NFPA 1125, the code commercial motors are certified to, requires that the
  "standard deviation of the total impulse data shall be no greater than 6.7 percent of the mean"
  over a motor type's certification firings (§8.1.7 and §8.2.7). Four certification reports of the
  National Association of Rocketry (NAR) measured 1.3% to 3.1%: an Estes C6 (2.0%), D12 (3.1%),
  E12 (1.28%) and an AeroTech G80 (2.3%), hosted on
  [ThrustCurve.org](https://www.thrustcurve.org/motors/cert/60c63bfcb5bc370004713e8f/G80-20071207.pdf).
  So 2% to 3% is a fair guess, and 6.7% the most the code allows.
- **Ejection delay.** The same code allows a measured delay to differ from the labelled one by
  "1.5 seconds or 20 percent (whichever is greater, but not to exceed 3 seconds)". The E12 and G80
  reports' firings of one delay scatter by 0.2 to 0.9 s, and a delay's average can sit more than a
  second from its label: the G80's 7 s delay averaged 5.88 s. A dispersion is about the nominal
  value, so give the delay you expect, not only the one printed on the motor.
- **Burn time.** The code sets no limit of its own. The four reports disagree: the E12's and G80's
  burn times scatter by 1.5% and 1.8%, the older C6's and D12's by 17% and 18%. Why isn't recorded;
  the older sheets may time the burn differently. Start from a few percent for a composite motor,
  and try a larger value to see whether it matters to your flight.
- **Mass, centre of mass and drag** depend on how well you know your rocket. A rocket weighed
  ready to fly needs a smaller mass dispersion than one weighed on paper. Drag is usually the least
  certain of the three: [Accuracy](accuracy.md) shows how far hpr-sim's drag sits from other
  programs' and from wind-tunnel data.
- **Wind** depends on the forecast, its age and the hour. A [sounding](glossary.md#sounding) of the
  day, or a forecast's spread between models, is a better guide than a guess.

The NFPA wording is the 2019 edition's, as quoted in the public first-draft documents of its next
revision
([public inputs 4 and 5](https://docinfofiles.nfpa.org/files/AboutTheCodes/1125/1125_A2021_PYR_AAA_FD_PIResponses.pdf)),
whose first revisions FR-7 and FR-8 keep both sentences
([`1125_A2021_PYR_AAA_FD_FRStatements.pdf`](https://docinfofiles.nfpa.org/files/AboutTheCodes/1125/1125_A2021_PYR_AAA_FD_FRStatements.pdf)).
The edition in force today hasn't been checked.

## What is not dispersed

- Inputs are independent: a heavier rocket isn't also draggier.
- Every dispersion is normal; there are no uniform or skewed ones.
- Moving a stage's centre of mass keeps its inertia about the centre.
- The drag dispersion scales the zero-lift drag only, not the [normal force](glossary.md#normal-force)
  or the moments; a recovery device's drag isn't dispersed.
- A stretched thrust curve keeps its shape.
- The atmosphere's temperature and pressure, a motor's ignition time, and anything of a flight's
  [staging](glossary.md#stage) and separations other than a recovery device's lag.

## What comes next

The run gives each flight's whole [`FlightSummary`](api/hpr_sim/metrics/struct.FlightSummary.html),
so any number a flight reports can be spread with `run.distribution(...)`, as the example does for
the landing. Still to come in [M6.1](decisions-and-roadmap.md#m6-1):
sensitivity analysis, which input moves the apogee most ([M6.1c](decisions-and-roadmap.md#m6-1c));
and 10,000 flights in seconds ([M6.1d](decisions-and-roadmap.md#m6-1d)).

The API reference is [`hpr_analysis::montecarlo`](api/hpr_analysis/montecarlo/index.html),
[`hpr_analysis::statistics`](api/hpr_analysis/statistics/index.html) and
[`hpr_analysis::ellipse`](api/hpr_analysis/ellipse/index.html).

[`Run::failed`]: api/hpr_analysis/montecarlo/struct.Run.html#method.failed
[`Run::apogee`]: api/hpr_analysis/montecarlo/struct.Run.html#method.apogee
[`run_parallel`]: api/hpr_analysis/montecarlo/struct.MonteCarlo.html#method.run_parallel
[`Run::landing`]: api/hpr_analysis/montecarlo/struct.Run.html#method.landing
[`Scatter::ellipse`]: api/hpr_analysis/ellipse/struct.Scatter.html#method.ellipse
[`Scatter::prediction_ellipse`]: api/hpr_analysis/ellipse/struct.Scatter.html#method.prediction_ellipse
[`Scatter::share_inside`]: api/hpr_analysis/ellipse/struct.Scatter.html#method.share_inside
[`Scatter::points`]: api/hpr_analysis/ellipse/struct.Scatter.html#method.points
