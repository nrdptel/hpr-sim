# Optimization

Sometimes you know the result you want and need the design that gives it: that is
[optimization](glossary.md#optimization). That might be a rocket
that reaches exactly 3,048 m (10,000 ft) for a competition, or the lightest fins that keep it
stable. An *optimizer* searches for that design. It tries designs, flies each one, and uses what
it learns to choose better ones, until it finds the best it can. This page shows hpr-sim's
optimizer, [CMA-ES](glossary.md#cma-es). It runs on test functions whose answers are known, then
finds the nose ballast and body length that send a rocket to 3,048 m with a chosen stability
margin. Last, it chooses a motor and a catalogue nose cone as well, within a competition's limits
on stability and speed off the rail. It needs some Rust.

> **How far to trust it.** The optimizer is tested against answers known exactly, and against the
> reference implementation by its author. On a rocket, its answer is only as good as hpr-sim's
> flight models, which are not yet validated against real flights ([Accuracy](accuracy.md)).
>
> - **Tested:** four standard test functions of ten variables are run from 20 seeds each. Every
>   run reaches a value of 10⁻¹⁰ or less, which puts its best point within 10⁻⁵ of the known
>   minimum (10⁻⁴ for Rosenbrock's function). The exception is 3 of the 20 Rosenbrock runs, which
>   end in that function's known local minimum instead. An *evaluation* is one call of your model:
>   here, one flight. Over the 20 seeds, the median number of evaluations is within 25% of
>   pycma's, the method's author's own Python implementation run from the same starts. That 25% is
>   the test's bound; measured, they are within 5%. Twelve generations of a three-variable run are
>   also recomputed by a separate implementation of the published formulas, and agree to rounding
>   ([`tests/optimize.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/optimize.rs)).
>   A run is repeated bit for bit from its [seed](glossary.md#seed) (`a_seed_fixes_the_run` in
>   [`optimize/cmaes.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/optimize/cmaes.rs)).
> - **Checked by re-flying:** the rocket design each example finds is flown again from scratch,
>   and again with the flight's numerical integration 100 times stricter
>   ([tolerances](glossary.md#tolerance)). Both reach apogee within 0.1 m of 3,048 m (the
>   first example's within 2 mm, measured).
> - **Limits** (a minimum stability margin, say): held to three test problems whose answers on
>   their limits are known exactly, from 20 seeds each, to 10⁻¹⁰
>   ([`tests/constrained.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/constrained.rs)).
>   The second example's winner is flown again and keeps its margin limits over the whole
>   ascent and its rail-exit limit.
> - **Choices** (which motor, which catalogue part): held to three test functions mixing
>   continuous and whole-number variables, at two sizes, from 20 seeds each. Every run reaches
>   the known minimum, with every whole number exactly right, and the median evaluations are
>   within 25% of an outside implementation's (the test's bound; measured, within 5%;
>   [`tests/mixed.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/mixed.rs)).
>   The second example chooses a motor and a nose cone that hit 3,048 m, checked by flying them
>   again ([Choices](#choices-a-motor-a-catalogue-part)). It finds *a* design that does; it
>   doesn't promise the best of several that would.
> - **Left out, for now:** trade-offs between goals (several goals can only be folded into one
>   number, as the first example does), and optimizing a [Monte Carlo](glossary.md#monte-carlo)
>   run's statistics. They are the next steps of [M6.2](decisions-and-roadmap.md#m6-2), the
>   optimization milestone. There is no command-line or Python front end yet.

## What the optimizer does

The optimizer changes *variables*: numbers in the design, such as a ballast mass or a body length
([`Variable`]). For each variable you give three things:

- a **start**, the value to begin from;
- a **step**, how far to try from the start at first, about a quarter to a third of the range you
  expect the answer in;
- optional **bounds**, the range the variable must stay in.

It *minimizes* one number your model returns. To maximize something, minimize its negative. To hit
a target, minimize the squared miss: `(apogee − 3,048 m)²` is smallest, zero, on the target.

## How CMA-ES searches

CMA-ES is the *covariance matrix adaptation evolution strategy*, N. Hansen's method ("The CMA
Evolution Strategy: A Tutorial", [arXiv:1604.00772](https://arxiv.org/abs/1604.00772), 2023). It
keeps a cloud of likely designs, shaped like a stretched, tilted ball (a
[normal distribution](glossary.md#normal-distribution)), and repeats four steps each *generation*:

1. **Sample.** Draw a handful of designs from the cloud: `λ = 4 + ⌊3 ln n⌋` of them for `n`
   variables, so 6 for two variables and 10 for ten.
2. **Rank.** Fly each one and sort them by the number to minimize. Only the order matters, not the
   values.
3. **Move.** Move the cloud's centre to a weighted average of the better half, the best weighted
   most.
4. **Learn.** Stretch and tilt the cloud toward the directions the good designs lay in. Widen it
   when successive moves go the same way, which means the steps are too short, and shrink it when
   they cancel out, which means the steps overshoot.

So the cloud learns the problem's shape. If the apogee depends far more on one variable than on
another, or on a combination of them, the cloud stretches to match. No derivatives are needed,
which suits flights: their outputs wobble slightly in the last digits as the integrator's steps
change, which would spoil a slope.

The formulas and default settings are the tutorial's (Appendix A and Table 1). Table 1 also gives
the worse half of each generation negative weights, which push the cloud away from them: the
*active* variant. hpr-sim gives them zero weight, as the tutorial's own code does.

The cloud is shaped in each variable divided by its step, so variables in metres and in
kilograms start on an equal footing, as the tutorial advises.
[`hpr_analysis::optimize::cmaes`](api/hpr_analysis/optimize/cmaes/index.html) lists every equation.

### Bounds

A sampled design outside a variable's bounds is thrown away and drawn again, the tutorial's
simplest way of handling bounds. It works well while the answer lies inside the range. If it
lies on a bound, the run slows down near it. If any one design of a generation is still outside
after 1,000 tries, the run stops and says so. With many variables near their bounds this comes
soon: each such variable halves the chance that a draw falls inside.

### When a run stops

A run stops at the first of these ([`Stop`]):

| Stop | When |
|---|---|
| `Target` | a value at or below the target you set: a miss small enough |
| `Evaluations` | the number of evaluations you allowed is used up (10,000 by default) |
| `TolX` | the cloud has shrunk below 10⁻¹² of each variable's step: the run has converged |
| `TolFun` | the values have stopped changing by more than 10⁻¹², in your model's units |
| `Condition` | the cloud is 10⁷ times longer than it is wide, too thin to keep accurate, or its numbers have overflowed |
| `Bounds` | a design of a generation couldn't be drawn inside the bounds (in the first generation, `start` returns an error instead) |

Both tolerances can be changed or turned off (`with_tolerance_x`, `with_tolerance_value`).

The result ([`Optimum`]) gives the best design found, its value, how many evaluations the run made,
and why it stopped. If every design so far failed (your model returned infinity for each), the
value is infinite: check it before using the design.

## Checked against

Four test functions from CMA-ES's authors (N. Hansen, S. D. Müller and P. Koumoutsakos,
*Evolutionary Computation* 11(1), 2003, Table 1), each with ten variables
([`hpr_analysis::optimize::benchmark`](api/hpr_analysis/optimize/benchmark/index.html)):

| Function | What it tests | Minimum |
|---|---|---|
| sphere, `Σ xᵢ²` | the step size alone | 0 at `x = 0` |
| ellipsoid | stretching the cloud: coefficients from 1 to 10⁶, so the cloud must grow 1,000 times longer one way than the other | 0 at `x = 0` |
| rotated ellipsoid | the same, tilted so that no single variable lines up with it | 0 at `x = 0` |
| Rosenbrock's function | following a long, curved valley | 0 at `x = 1` |

Each run starts with every variable at 1 (at 0 for Rosenbrock's function) and steps of 0.5, and
goes until the value is 10⁻¹⁰ or less, the stopping value of the 2003 paper. The tests in
[`tests/optimize.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/optimize.rs)
run each function from 20 seeds and check:

- Every sphere and ellipsoid run reaches the minimum, which puts its best point within 10⁻⁵ of
  `x = 0`.
- 17 of 20 Rosenbrock runs reach the minimum at `x = 1`. The other 3 end in the function's local
  minimum near `(−1, 1, …, 1)`. The test requires at least 17. CMA-ES's authors note that local
  minimum, and report 1 to 3 runs of 20 missing the global one at 4 to 16 variables (Kern, Hansen
  and Koumoutsakos, 2006). pycma ends in the local minimum in 1 of 20. Measured once over seeds
  1 to 300, hpr-sim reaches the global minimum in 290 (97%), so 17 of 20 is at the low end of what
  chance gives.
- The median number of evaluations is within 25% of pycma's from the same starts (the test's
  bound). Measured, they are within 5%:

| Function | hpr-sim | pycma 4.5.0 |
|---|---|---|
| sphere | 1,635 | 1,640 |
| ellipsoid | 5,920 | 5,910 |
| rotated ellipsoid | 6,010 | 5,930 |
| Rosenbrock | 6,445 | 6,195 |

- Tilting the ellipsoid changes the count by under 10%: the cloud learns the tilt.
- Every generation's mean, cloud size and shape, over twelve generations with steps that differ
  200-fold, match a second, separate implementation of the tutorial's formulas to rounding. It
  takes the cloud's inverse square root by a different method (the Denman–Beavers iteration), and
  passes through both branches of the step that pauses the shape's learning when the cloud
  grows fast.
- The default settings that pycma also takes from the tutorial's Table 1 match its values to
  rounding. pycma departs from the table in a few places by its author's choice, such as how fast
  the cloud's size adapts. That is why the counts differ a little.

The tests catch real faults. Without the step that learns the cloud's shape from a generation's
better half (the *rank-μ update*, μ being the size of that half), the ellipsoid takes a third more
evaluations (7,875) and the pycma comparison fails. A reviewer found three faults the pycma
comparison misses: the shape's learning never paused, the pause's test off by one generation, and
the mean's step not scaled back to the variable's units. Each fails the second implementation's
check. These were one-off checks, recorded in
[ADR-138](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-138-optimization-cma-es-first-held-to-test-functions-and-to-pycma-2026-10-01)
(the decision record for this work), not run in CI.

pycma is run by
[`validation/oracles/cmaes/pycma_runs.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/cmaes/pycma_runs.py),
whose results are committed beside the test.

## An example

The example program
[`crates/hpr/examples/optimization.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/optimization.rs)
runs CMA-ES on three of the test functions. Then it takes a 66 mm rocket on a J760, starting from
a 3 m rail at 85° into 5 m/s of wind. It finds the nose ballast and body tube length that give it
two things at once: an apogee of 3,048 m above the pad, and a static margin of 2.20
[calibres](glossary.md#calibre-caliber) at launch mass (at Mach 0.3). Run it from a copy of the
repository with:

```text
cargo run --example optimization -p hpr
```

Why two goals? With one goal, the apogee, and two variables, there is a whole curve of answers:
more ballast and a shorter body cancel out, and the run would stop at whichever hit it reached
first. A second goal, the margin, leaves one design that meets both, so the answer is the
rocket's, not the run's.

The heart of it, abridged from the example:

```rust,ignore
let variables = vec![
    Variable::new("nose ballast (kg)", 0.3, 0.15)?.within(0.0, 1.0)?,
    Variable::new("body tube (m)", 1.0, 0.1)?.within(0.6, 1.4)?,
];
let optimizer = Cmaes::new(variables)?
    .with_target(1e-4)?            // stop within a centimetre and a ten-thousandth of a calibre
    .with_max_evaluations(2_000)?;
let optimum = optimizer.minimize(seed, |x| {
    // (apogee miss in m)² + (margin miss in hundredths of a calibre)²;
    // a design the simulator refuses ranks last
    miss(x[0], x[1], FlightSettings::default()).unwrap_or(f64::INFINITY)
})?;
```

`miss` builds the rocket with that ballast and body length, flies it, and adds the two squared
misses, each in a unit that makes a miss of one about equally bad. A design that can't fly returns
infinity, which ranks below every real flight. Then the example flies the winner twice more: once
from a fresh build, which must give the optimizer's result to the last bit, and once with the
integrator's tolerances 100 times tighter, which must still reach apogee within 0.1 m of
3,048 m.

The example prints whether each test function reached its minimum, not how many evaluations it
took. Operating systems round the last bit of `ln` and `exp` differently, and an optimizer's path
amplifies that, so the counts move by a few per cent between macOS, Linux and Windows. The
medians [above](#checked-against) are the test's, on the development machine. It prints:

<!-- quote: crates/hpr/examples/optimization.output.txt -->
```text
CMA-ES on three test functions, 10 variables from 1.0, run to f ≤ 1e-10 (seed 2026)
function             reached f ≤ 1e-10   best point within 1e-5 of x = 0
sphere                             yes                               yes
ellipsoid                          yes                               yes
rotated ellipsoid                  yes                               yes

A 66 mm rocket on a J760: find the nose ballast and body length for a 3,048 m apogee
and a static margin of 2.20 calibres at launch mass, at Mach 0.3
Not yet validated: see the Accuracy page before trusting these numbers.
Start: 0.30 kg of ballast, a 1.00 m body: apogee 3128.8 m, margin 1.79 calibres
Found: 0.34 kg of ballast, a 1.11 m body (stopped: Target)
Flown again: apogee 3048.0 m, margin 2.20 calibres
Flown again, tolerances 100 times tighter: apogee 3048.0 m
```

The rocket started 81 m too high, with a margin of 1.79 calibres. The optimizer found the design
in 300 flights on the development machine, about 50 generations. Flown again with tighter
tolerances, its apogee moves by 1.4 mm. Limits on other things, such as the rail-exit speed, are
the subject of [Limits on a design](#limits-on-a-design), and the second example uses them.

## Evaluating designs your own way

`minimize` flies one design after another. To fly a generation's designs on several threads or
machines, use the run's steps yourself ([`Run`]): `Cmaes::start` draws the first generation,
`candidates` lists its designs, and `tell` takes their values, in the same order, and draws the
next. It returns the result once the run stops.

## Choosing the numbers

- **Steps:** about a quarter to a third of the range the answer is likely in, in the variable's
  own units. Steps of very different sizes are fine: the run works in each variable divided by its
  step.
- **Target:** for a target apogee, the squared miss you accept: `0.1 * 0.1` for 0.1 m. Without a
  target, a run goes on until it converges, which can take many more flights than a hit needs.
- **Evaluations:** a cap on flights, 10,000 by default. The first example's two variables needed
  300 flights, the second example's four variables 360; the ten-variable test functions take 1,600 to 6,500
  evaluations to converge.
- **Population:** leave it at the default unless the output has many local minima. Then a larger
  population ([`Cmaes::with_population`]) searches more widely, at more flights per generation.
- **Seed:** a different seed gives a different run. Rerun with two or three seeds if the answer
  matters: if they agree, the answer is not luck.

## Limits on a design

A design usually has limits as well as a goal: a stability margin of at least 1.5 calibres, say,
or at least 15 m/s off the rail. The optimizer takes them as constraints, each written as a
number `g` that must not be above zero. A margin of at least 1.5 calibres is `g = 1.5 − margin`.
Your model returns an [`Evaluation`]: the value, and the *violation*, the sum of every `g` that
is above zero. `Evaluation::constrained(value, &[g1, g2])` adds them up for you.

Candidates are ranked by K. Deb's feasibility rules ([ADR-139][adr-139], from Deb's 2000 paper):

| Comparing | The better one is |
|---|---|
| one that keeps every limit, one that doesn't | the one that keeps them |
| two that keep every limit | the smaller value |
| two that break some | the smaller violation |

No penalty weight is needed, because a value is never weighed against a violation. Do scale the
limits so that they count alike: a margin 0.1 calibre short and a rail speed 0.1 m/s short are
not equally bad, so divide each `g` by a size you care about (the 1.5 calibres, the 15 m/s).

Candidates that break a limit are kept and ranked, not redrawn, so a run can close in on an
answer that sits right on a limit, as most good designs do. Only a point that keeps every limit
counts as reaching a target; the [`Optimum`] reports its `violation`, zero when it keeps them all.
For a bound that the answer will sit on, leave the variable unbounded on that side and write the
bound as a limit; clamp the value you fly (no negative ballast), and work the limit out from the
unclamped one. A design that can't be flown at all returns `Evaluation::failed()`, which ranks
behind every other, and a run that never keeps every limit says so: its `violation` is above
zero.

The tests hold this to three problems whose answers on their limits are known exactly:

| Problem | Limit | Answer |
|---|---|---|
| Σ xᵢ², 10 variables | x₀ ≥ 1 | 1, at x = (1, 0, …, 0) |
| Σ xᵢ², 10 variables (the "tangent" problem) | Σ xᵢ ≥ 10 | 10, at every xᵢ = 1 |
| g06 of the CEC 2006 benchmark, 2 variables | inside one circle, outside another | −6961.81388, where the circles cross |

From each of 20 seeds the run reaches the answer to 10⁻¹⁰ of its size and to within 10⁻⁴ of the
point, also when it starts where the limit is broken. The next section's example puts limits on a
rocket: its stability margin and its speed off the rail.

## Choices: a motor, a catalogue part

Some variables are choices from a list rather than amounts: which motor, which nose cone from a
maker's catalogue. Make each one an *integer variable* ([`Variable::integer`]). It takes only the
whole numbers from its low bound to its high one, and your model uses the number as a place in
your list: 0 for the first motor, 1 for the second, and so on.

The optimizer still draws a real number for the variable, and gives your model the nearest whole
number. Left at that, the cloud would shrink in that variable until every draw rounded to the same
number, and the choice would freeze, even with a better one next to it. *CMA-ES with margin*
(R. Hamano, S. Saito, M. Nomura and S. Shirakawa, GECCO 2022,
[arXiv:2205.13482](https://arxiv.org/abs/2205.13482)) prevents that. After each generation it
keeps at least a small chance, the *margin* (nothing to do with a stability margin), that a draw
lands on another value. It does so by
moving the cloud's centre towards the edge between two values, or by widening the cloud in that
variable. The margin is `1/(n λ)`, for `n` variables and `λ` designs a generation: 1 in 32 for the
example below. So a choice is never final: its neighbours keep being tried.

Put the list in an order where neighbours are alike, such as motors by total impulse. The
optimizer steps between neighbouring numbers, so in that order a step up means a little more
motor. A list in no order still works, but the search has less to go on.

### The example

The example program
[`crates/hpr/examples/motor_and_nose.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/motor_and_nose.rs)
builds a 2.6 in rocket from Madcow Rocketry's parts in the built-in
[catalogue](the-builder.md#parts-from-a-catalogue): a 1.0 m fiberglass body tube, an 18 in motor tube and three
fiberglass fins. It flies it from a 3 m rail at 85° into 5 m/s of wind. The optimizer chooses four
things:

| Variable | Kind | Range |
|---|---|---|
| the motor | integer | the five 54 mm motors of the built-in catalogue that fit, by total impulse |
| the nose cone | integer | four Madcow nose cones for 2.6 in, shortest first |
| the nose ballast | continuous | 0 to 1.5 kg |
| the fins' span | continuous | 3 to 15 cm |

The goal is the squared miss from 3,048 m. The limits come from the International Rocket
Engineering Competition's rules (its *Design, Test & Evaluation Guide*, 2025), and hold over the
whole ascent, from the rail exit to apogee:

- a [stability margin](glossary.md#stability-margin) of at least 1.5
  [calibres](glossary.md#calibre-caliber) in flight (§10.3.1 asks for a "dynamic" margin; the
  example takes the *flight margin*, the margin at the flight's own Mach number, as the
  [flight metrics](physics/metrics.md) define it);
- a *static* margin, the one at Mach 0, of at most 4 calibres, and a flight margin of at most 6,
  so the rocket isn't over-stable (§10.4.1);
- at least 30 m/s off the rail (§10.2.1).

The margins change through the flight: the centre of mass moves forward as the motor burns, so
the margin grows, and the flight margin changes with speed as well. For the lower limit the
example uses the least flight margin of the ascent, which the flight finds inside its steps, not
only at their ends. For the upper limits it uses the largest static and flight margins seen at
the end of each integration step. The static margin only grows during the burn and stays put
after it, so its largest is exact. A peak of the flight margin between two step ends could be
missed; the winner's largest, 4.83 calibres, is well under its limit of 6.

The heart of it, abridged from the example. A choice is an integer variable: set its bounds with
`within`, then mark it `integer`, and use the whole number your model is given as a place in your
list. With limits, use the run's steps yourself ([`Run`]) and give `tell_constrained` an
[`Evaluation`] per design:

```rust,ignore
let variables = vec![
    Variable::new("motor", 2.0, 1.0)?.within(0.0, 4.0)?.integer()?, // a place in the motor list
    Variable::new("nose", 1.0, 1.0)?.within(0.0, 3.0)?.integer()?,  // a place in the nose list
    Variable::new("nose ballast (kg)", 0.4, 0.15)?.within(0.0, 1.5)?,
    Variable::new("fin span (m)", 0.07, 0.015)?.within(0.03, 0.15)?,
];
let mut run = Cmaes::new(variables)?.with_target(1e-4)?.with_max_evaluations(3_000)?.start(seed)?;
let optimum = loop {
    let mut evaluations = Vec::new();
    for x in run.candidates() {
        // x[0] and x[1] are whole numbers within their bounds
        let (motor, nose) = (x[0] as usize, x[1] as usize);
        let design = Design { nose, ballast_kg: x[2], fin_span_m: x[3] };
        evaluations.push(match flyer.fly(&motors[motor], &design) {
            Ok(flown) => {
                let miss = flown.apogee_m - 3048.0;
                Evaluation::constrained(miss * miss, &flown.limits()) // each limit g ≤ 0
            }
            Err(_) => Evaluation::failed(), // a design that can't fly ranks last
        });
    }
    if let Some(optimum) = run.tell_constrained(&evaluations)? {
        break optimum;
    }
};
```

`Design`, `Flyer::fly` and `limits` are the example's own: they build the rocket, fly it, and
write each limit as a number `g` that must not be above zero, as [Limits on a
design](#limits-on-a-design) describes. A draw for an integer variable outside its bounds is
clamped to the nearest end, not drawn again. Run it from a copy of the repository with:

```text
cargo run --example motor_and_nose -p hpr
```

It takes about half a minute in a debug build. It prints:

<!-- quote: crates/hpr/examples/motor_and_nose.output.txt -->
```text
A 2.6 in rocket of Madcow Rocketry's parts: choose the motor, the nose cone, the nose
ballast and the fin span for a 3,048 m apogee, with margins over the ascent of at
least 1.5 calibres (flight), at most 4 (static) and 6 (flight), and at least 30 m/s off a 3 m rail
Not yet validated: see the Accuracy page before trusting these numbers.
motor    total impulse (N·s)   designs flown   nearest apogee within the limits
J450DM                1061.6               3   2648 m
J300LR                1212.8               6   none
J760                  1267.3              25   3057 m
K400C                 1307.3             307   3048 m
K940                  1636.1              19   3150 m
Found: the K400C with the PNC26K-W nose (3:1 ogive, plastic), 0.35 kg of ballast, fins 8.9 cm in span
(stopped: Target, after 360 flights)
Flown again: apogee 3048.0 m, top speed Mach 1.23, rail exit 33.8 m/s, liftoff mass 2.60 kg;
margins over the ascent: flight 3.04 to 4.83 calibres, static at most 3.88
Flown again, tolerances 100 times tighter: apogee 3048.0 m, rail exit 33.8 m/s
Both flights within 0.1 m of 3,048 m and within every limit: yes
```

The table counts the designs the run flew with each motor; the total impulse is from the motor's
thrust curve. The last column is the apogee nearest 3,048 m among the designs that kept every
limit. The run tried every motor. Its few J450DM designs that kept the limits fell well short.
None of its J300LR designs kept them all. It settled on the K400C and stopped once it was within a
centimetre of the target. Flown again, with the integrator's tolerances as set and then 100 times
tighter, the winner is within 0.1 m of 3,048 m, and keeps every limit over the whole ascent.

Read the table as what this one run saw, not as what each motor can do: a few designs, or a few
dozen, say little about a motor. The answer is not unique. A scan of every motor and nose over the ballast
and the fin span, run once on the development machine and not kept, found that the J760, the
K400C and the K940 can each hit 3,048 m within the limits, with any of the four noses. A hit with
any of them scores the same, so another seed may well settle on another. To prefer one, say so in
the goal. A small cost for liftoff mass is one way.

Unlike the first example's counts, this run's come out the same on all three operating systems:
CI checks the output on macOS, Linux and Windows, whose last bits of `ln` and `exp` differ. On
the development machine the run was also tried with every flight perturbed (the integrator's
tolerance changed by up to 1%) and with the step size moved by a few bits each generation, and
neither changed a line of the output; those trials are not kept. Your own runs repeat bit for bit
on one machine from the same seed, but on another machine a run can take another path.

A goal that changes only when the choice changes, such as "the smallest motor that can do it",
gives the search nothing to follow between choices. For that question, run the optimizer once
for each motor, the motor fixed, and compare. The continuous search then only has to find a
design that keeps the limits.

The body tube's length is fixed on purpose. Past Mach 1.2 a flight needs a table of the body's
supersonic pressures, which takes a fifth of a second or more to build. Designs with the same
outside shape can share one table, so with the length fixed the example builds at most three,
one per fiberglass nose cone (the next paragraph says why the plastic one has none). With the length a variable, every design would build its own, and the run
would take minutes. The example shows how to share the table, in its `Flyer::fly`.

The winning plastic nose cone gets no table at all. The catalogue lists it 0.05 mm narrower than
the tube (2.638 in against 2.640 in), and the method behind the table doesn't yet take a step in
the body's outline, however small ([issue #87](https://github.com/nrdptel/hpr-sim/issues/87)).
So for the short stretch of its flight past Mach 1.2 (its top speed is Mach 1.23) hpr-sim falls
back on slender-body theory for the body's normal force, its lift at an angle ([Bodies faster than
sound](physics/aero.md#bodies-faster-than-sound)). How much that moves the apogee hasn't been
measured; the flight spends only a moment above Mach 1.2.

### Checked against

Three test functions of Hamano and co-authors (§5.1 of their paper), each with half its variables
continuous and half whole numbers
([`benchmark::mixed`](api/hpr_analysis/optimize/benchmark/mixed/index.html)):

| Function | Whole-number variables | Minimum |
|---|---|---|
| SphereInt, `Σ xᵢ²` | −10 to 10 | 0 at `x = 0` |
| EllipsoidInt, the ellipsoid above | −10 to 10, with the largest coefficients | 0 at `x = 0` |
| SphereOneMax, `Σ xᵢ²` + the number of zeros | 0 or 1 | 0 at continuous 0, every whole number 1 |

Each runs with 10 and with 20 variables, from 20 seeds, to a value of 10⁻¹⁰. The starts and
settings are those of
[`validation/oracles/cmawm/cmawm_runs.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/cmawm/cmawm_runs.py),
which runs the same functions with `cmaes` 0.13.1, an outside implementation (MIT) adapted from
the method's authors' code, with its active weights off as hpr-sim's are. Every run of every case
reaches the minimum, with each whole number exactly right. The median evaluations are within the
test's 25% of the outside implementation's, and measured within 5%:

| Function | Variables | hpr-sim | `cmaes` 0.13.1 |
|---|---|---|---|
| SphereInt | 10 | 1,855 | 1,850 |
| SphereInt | 20 | 3,870 | 3,798 |
| EllipsoidInt | 10 | 3,460 | 3,625 |
| EllipsoidInt | 20 | 9,246 | 9,372 |
| SphereOneMax | 10 | 1,925 | 1,955 |
| SphereOneMax | 20 | 3,828 | 3,726 |

A unit test checks the margin itself after every generation of a mixed run, with each kind of
whole-number variable (`integer_draws_keep_the_margin` in
[`optimize/cmaes.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/optimize/cmaes.rs)).
Another sets up a run's state by hand and checks one correction of each kind against the paper's
equations worked out to 40 digits (`margin_correction_matches_the_equations`).
With the margin taken out, three of the six cases fail: some runs stall with a whole number
stuck on a wrong value. That was a one-off check, recorded in
[ADR-140](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-140-discrete-choices-by-cma-es-with-margin-2026-10-02),
not run in CI.

## Left out

- A choice is a whole number in a list you order. There is no separate handling for choices with
  no order, beyond putting alike ones next to each other.
- Limits are inequalities only. For an equality `h = 0`, write `|h| − ε ≤ 0` with a small `ε`.
- No trade-offs between goals (a Pareto front): several goals can only be folded into one
  number, as the example does.
- No optimizing of a Monte Carlo run's statistics, such as the chance of landing within a
  distance.
- No command-line or Python front end yet.

The API reference is
[`hpr_analysis::optimize`](api/hpr_analysis/optimize/index.html).

[`Variable`]: api/hpr_analysis/optimize/struct.Variable.html
[`Variable::integer`]: api/hpr_analysis/optimize/struct.Variable.html#method.integer
[`Evaluation`]: api/hpr_analysis/optimize/struct.Evaluation.html
[adr-139]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-139-optimization-constraints-by-debs-feasibility-rules-2026-10-01
[`Stop`]: api/hpr_analysis/optimize/cmaes/enum.Stop.html
[`Optimum`]: api/hpr_analysis/optimize/cmaes/struct.Optimum.html
[`Run`]: api/hpr_analysis/optimize/cmaes/struct.Run.html
[`Cmaes::with_population`]: api/hpr_analysis/optimize/cmaes/struct.Cmaes.html#method.with_population
