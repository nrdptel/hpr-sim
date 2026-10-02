# Optimization

Sometimes you know the result you want and need the design that gives it: that is
[optimization](glossary.md#optimization). That might be a rocket
that reaches exactly 3,048 m (10,000 ft) for a competition, or the lightest fins that keep it
stable. An *optimizer* searches for that design. It tries designs, flies each one, and uses what
it learns to choose better ones, until it finds the best it can. This page shows hpr-sim's
optimizer, [CMA-ES](glossary.md#cma-es). It runs on test functions whose answers are known, then
finds the nose ballast and body length that send a rocket to 3,048 m with a chosen stability
margin. Then it chooses a motor and a catalogue nose cone as well, within a competition's limits
on stability and speed off the rail. Last, a second optimizer, [NSGA-II](glossary.md#nsga-ii),
weighs two goals against each other: how much apogee each extra [calibre](glossary.md#calibre-caliber)
of stability costs. A third, [EGO](#few-evaluations-ego), is for models so slow that only tens
of evaluations can be afforded. It needs some Rust.

> **How far to trust it.** Both optimizers are tested against answers known exactly, and against
> outside implementations. On a rocket, their answers are only as good as hpr-sim's
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
> - **Checked by re-flying:** the design each of the first two examples finds is flown again from
>   scratch, and again with the flight's numerical integration 100 times stricter
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
> - **Trade-offs** between goals (a [Pareto front](glossary.md#pareto-front)): NSGA-II is held to
>   three test problems whose fronts are known exactly, from 20 seeds each, and to pymoo, an
>   outside implementation run with the same settings. Every run's front lies within twice
>   pymoo's worst distance from the true front (the test's bound). At the median, hpr-sim's
>   fronts lie 1% to 8% closer to the true front than pymoo's, and cover it as evenly, within 3%
>   (the test allows a factor of 1.25 either way; [Trade-offs](#trade-offs-a-pareto-front)). The third
>   example's front designs are flown again and give the same apogee and margin to the bit:
>   that shows the result repeats, not that it is right. At 2.5 calibres the front is within
>   0.5% of the apogee CMA-ES finds alone (the example's check).
> - **Few evaluations** (EGO): from 20 seeds each, two standard test functions with several
>   local minima, Branin's (two variables) and Hartmann's (three), end within 1% of their known
>   minima in 50 evaluations (the test's bound; measured, within 0.12%). On Hartmann's
>   six-variable function, EGO stops at a local minimum in 6 runs of 10: not yet fixed
>   ([Few evaluations](#few-evaluations-ego)).
> - **Left out, for now:** optimizing a [Monte Carlo](glossary.md#monte-carlo) run's statistics,
>   EGO on six variables, and a rule that stops NSGA-II
>   when its front has settled (it runs the generations it is given). They are the next steps of
>   [M6.2](decisions-and-roadmap.md#m6-2), the optimization milestone. There is no command-line
>   or Python front end yet.

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
- **NSGA-II's population and generations:** the number of flights is the population times the
  generations. The defaults, 100 designs for 250 generations, are 25,000 flights, sized for 30
  variables; the third example's two variables needed 20 designs for 25 generations, 500
  flights, to pass its 0.5% check. The population is also
  how many designs the front can hold. Compare the fronts from two seeds before trusting one.

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
whole ascent, from the rail exit to apogee (§10.3.1 says "from launch"; on the rail, the rail
holds the rocket):

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
the body's outline larger than a millionth of its area ([issue #87](https://github.com/nrdptel/hpr-sim/issues/87)).
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

## Trade-offs: a Pareto front

Often two goals pull against each other. Bigger fins or more nose weight make a rocket more
stable, and both cost apogee. There is no single best design then. Instead there is a set of
designs, none of which can improve one goal without giving up some of the other: the [Pareto
front](glossary.md#pareto-front). A design *dominates* another when it is no worse in either goal
and better in at least one; the front is the designs nothing dominates. Knowing the front shows
what each extra calibre of stability costs, before you pick one design from it.

CMA-ES finds one design. For a front, hpr-sim has [NSGA-II](glossary.md#nsga-ii), a genetic
algorithm by K. Deb and co-authors (2002). It keeps a population of designs and, each
generation:

1. breeds as many children as there are parents. Each pair of parents is chosen by two
   tournaments: of two designs picked at random, the one on a better front wins, or, on the same
   front, the one further from its neighbours (its *crowding distance*: the gaps between its two
   neighbours' values of each goal, each as a share of that goal's range in the front, added
   up). Every design plays two tournaments a generation.
   A pair's children mix their parents' values
   (*crossover*), then a few values are nudged at random (*mutation*);
2. flies the children;
3. sorts parents and children together into fronts: the designs nothing dominates, then those
   only the first front dominates, and so on;
4. keeps the best half, front by front. The front that doesn't fit whole keeps its designs with
   the most room around them, so the front stays spread out instead of bunching up.

Each variable needs a low and a high bound: the first generation is drawn evenly between them,
and the variable's start and step are not used. The defaults are the paper's: 100 designs, 250
generations, crossover of 90% of pairs and mutation of one variable in `n`, the number of
variables, on average ([`Nsga2`]). NSGA-II makes every goal as small as it can; to maximize one,
give its negative. It runs the generations it is given and has no test of when the front has
settled: run it again from a second seed, or for more generations, and compare the fronts.
Limits work as they do for CMA-ES: [`Goals::constrained`] takes each limit as a number `g ≤ 0`,
and a design that keeps every limit beats one that doesn't. A design that can't be flown is
[`Goals::failed`], behind every other.

### The example

The example program
[`crates/hpr/examples/pareto_front.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/pareto_front.rs)
takes the 66 mm rocket on a J760 from [An example](#an-example), with its body fixed at 1.0 m. It
varies two things, the nose ballast (0 to 0.8 kg) and the fins' span (4 to 10 cm), for two goals:
the highest apogee, and the largest static margin at launch mass, at Mach 0.3. Every design must
keep at least 1.5 calibres. The heart of it, abridged:

```rust,ignore
let variables = vec![
    // A start and step are required by `Variable`; NSGA-II uses only the bounds.
    Variable::new("nose ballast (kg)", 0.3, 0.15)?.within(0.0, 0.8)?,
    Variable::new("fin span (m)", 0.06, 0.015)?.within(0.04, 0.10)?,
];
let optimizer = Nsga2::new(variables, 2)?.with_population(20)?.with_generations(25)?;
let front = optimizer.minimize_constrained(seed, |x| match flyer.design(x[0], x[1]) {
    // Both goals as large as they can be: their negatives as small.
    Ok((apogee, margin)) => Goals::constrained(vec![-apogee, -margin], &[1.5 - margin]),
    Err(_) => Goals::failed(), // a design that can't fly ranks last
})?;
for member in &front.members {
    // member.point: ballast and span; member.objectives: −apogee and −margin
}
```

`flyer.design` is the example's own: it builds the rocket, works out its margin and flies it. Run
it from a copy of the repository with:

```text
cargo run --example pareto_front -p hpr
```

It takes about a minute in a debug build. It prints:

<!-- quote: crates/hpr/examples/pareto_front.output.txt -->
```text
A 66 mm rocket on a J760: how much apogee each calibre of static margin costs, over
its nose ballast (0 to 0.8 kg) and fin span (4 to 10 cm), with a margin of at least
1.5 calibres at launch mass, at Mach 0.3
Not yet validated: see the Accuracy page before trusting these numbers.
NSGA-II: 20 designs a generation, 25 generations, 500 flights (seed 2026)
The front: 20 designs, each flown again to the same apogee and margin

margin (cal)   apogee on the front (m), interpolated between the two designs either side
         2.0       3100
         2.5       3050
         3.0       2990
         3.5       2940

At 2.5 calibres, CMA-ES alone (200 flights): apogee 3050 m; the front within 0.5%: yes
```

Between 2 and 3.5 calibres, each extra half calibre of margin costs this rocket 50 to 60 m of
apogee. Each apogee in the table is interpolated in a straight line between the two front designs
whose margins bracket it. Two checks back the front up:

- Every one of its 20 designs is flown again from a fresh build and gives the same apogee and
  margin to the last bit. That shows the result repeats; it doesn't show it is right.
- At 2.5 calibres, CMA-ES alone, told to find the highest apogee with at least that margin, gets
  the same 3,050 m (to the nearest 10 m) in 200 flights: the front is within 0.5% of it, the
  example's check.

How much that depends on the seed was measured once, on the development machine, and isn't
checked in CI. From each of the 26 seeds 2020 to 2045, the front came within the 0.5%, from 0.46%
below CMA-ES's apogee to 0.18% above (CMA-ES's own answer in 200 flights varies by 0.24% over
those seeds). An earlier version of the example missed the check: with 20 designs for 12
generations its front fell 0.66% short, and the generations were raised to 25. (Before that, 24
designs over wider ranges spread the front from 1.5 to 5.3 calibres, too thinly, and the ranges
were narrowed.) The 0.5% was not changed
([ADR-141](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-141-several-goals-by-nsga-ii-2026-10-02)).

### Checked against

Three test problems of E. Zitzler, K. Deb and L. Thiele (2000), with 30 variables from 0 to 1
each, whose fronts are known exactly
([`benchmark::zdt`](api/hpr_analysis/optimize/benchmark/zdt/index.html)):

| Problem | The front | What it tests |
|---|---|---|
| ZDT1 | `f₂ = 1 − √f₁`, `f₁` from 0 to 1 | a convex front |
| ZDT2 | `f₂ = 1 − f₁²` | a concave front |
| ZDT3 | `f₂ = 1 − √f₁ − f₁ sin(10π f₁)`, in five separate pieces | a broken front |

Each is run from 20 seeds with the paper's settings: 100 designs for 250 generations, 25,000
evaluations. Two numbers measure a run's front against the true one:

- the **generational distance** (GD): the mean, over the front's designs, of each one's distance
  to the true front. Small when the front lies on the true one.
- the **inverted generational distance** (IGD): the mean, over points spread along the true
  front, of each one's distance to the nearest design. Small only if the front also covers all of
  the true one, evenly. The points are at 1,000 evenly spaced `f₁`; on ZDT3, the 265 of them
  that fall on its pieces.

hpr-sim measures GD to the true front's curve itself, not to points along it. The same problems
are run with pymoo 0.6.2, an outside implementation (Apache-2.0) by J. Blank and K. Deb, set up
as the paper describes, from seeds 1 to 20
([`pymoo_runs.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/nsga2/pymoo_runs.py)).
Both sets of columns below are scored by hpr-sim's measures; the pymoo columns are pymoo's
fronts. The rules were set from pymoo's runs before hpr-sim's were measured: every hpr-sim run
within twice pymoo's worst, and hpr-sim's median at most 25% above pymoo's, the margin CMA-ES's
tests allow against pycma. Review of the first results added a second rule: the median at most
20% below pymoo's, a factor of 1.25 either way. On these problems every variable but the first
is best at its lower bound, and an optimizer that drifts toward its bounds would score better
than pymoo without being better. Measured:

| Problem | Measure | hpr-sim median | hpr-sim worst | pymoo median | pymoo worst | Bound on every run |
|---|---|---|---|---|---|---|
| ZDT1 | GD | 1.07e-3 | 1.36e-3 | 1.10e-3 | 1.46e-3 | 2.92e-3 |
| ZDT1 | IGD | 4.90e-3 | 5.66e-3 | 4.96e-3 | 5.48e-3 | 1.10e-2 |
| ZDT2 | GD | 9.78e-4 | 1.40e-3 | 9.85e-4 | 1.41e-3 | 2.82e-3 |
| ZDT2 | IGD | 5.01e-3 | 5.44e-3 | 5.08e-3 | 5.35e-3 | 1.07e-2 |
| ZDT3 | GD | 4.13e-4 | 7.02e-4 | 4.50e-4 | 6.53e-4 | 1.31e-3 |
| ZDT3 | IGD | 5.38e-3 | 3.40e-2 | 5.51e-3 | 3.39e-2 | 6.78e-2 |

At the median, hpr-sim's fronts lie 1% to 8% closer to the true front than pymoo's, and cover it
as evenly (IGD 1.3% to 2.3% smaller). Each problem's worst hpr-sim run is 1.9 to 2.1 times inside
its bound. The
worst ZDT3 run of each has an IGD six times its median: it missed part of the front, a known
hazard on a front in pieces. The per-run bound on ZDT3's IGD is loose for the same reason, set by
pymoo's one such run; the median rule is what holds ZDT3's coverage. These come from
`cargo test -p hpr-analysis --test nsga2 -- --nocapture`
([`tests/nsga2.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/nsga2.rs)).
The same test checks hpr-sim's [`generational_distance`] against pymoo's own, on pymoo's fronts
and 500 reference points, to 10⁻¹².

Deb and co-authors' own runs (their Table II) report a mean distance of 0.033, 0.072 and 0.115
on these three problems, measured to 500 points of the true front: 18 to 87 times pymoo's mean
here, to the same 500 points (0.0013 to 0.0019). Why the paper's are so much larger was not investigated.

Unit tests check the pieces by hand
([`optimize/nsga2.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/optimize/nsga2.rs)):

- fronts and crowding distances of small sets worked out on paper;
- that crossover spreads children by the distribution its authors give, far from a bound and cut
  at one, and mutation steps likewise, cut at each side's bound, each within five standard
  errors over 100,000 draws; and that crossover's two children come out in either order;
- that children never leave a variable's bounds, nor pile up on them;
- that a failed design, or one breaking a limit, ranks behind every design that keeps them.

ZDT3's five pieces were solved to 40 digits and are checked against the equations that define
their ends ([`benchmark.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/optimize/benchmark.rs)).
They agree with pymoo's to its ten digits, except where pymoo prints the second piece's start as
0.182228780, a digit short of 0.1822287280.

## Few evaluations: EGO

CMA-ES and NSGA-II spend hundreds of flights. That is fine when a flight takes a tenth of a
second, but not when one evaluation is a long Monte Carlo run, or a slow outside program. *EGO*
(efficient global optimization; D. R. Jones, M. Schonlau and W. J. Welch, 1998) is built for
that case. It spends its effort thinking between evaluations, so it needs only tens of them.

It works like this:

1. It evaluates an *initial design*: 10 points per variable, spread evenly over the box the
   variables' bounds make. The design is a *Latin hypercube*: each variable's range is cut into
   as many equal slices as there are points, and each slice gets one point.
2. It fits a *surrogate*: a smooth guess at the model, drawn through every point evaluated so
   far. The surrogate is *kriging* (a Gaussian process). Besides its guess `ŷ` at any point, it
   gives a standard error `s`: zero at the points already evaluated, and growing away from them.
3. It asks where an evaluation is most worth making. The *expected improvement* at a point is
   how far below the best value so far the model is expected to come there, counting a chance
   of no gain as zero. It is large where the guess is low, and where the guess is unsure.
4. It evaluates the point of largest expected improvement, refits, and repeats.

Each variable needs a low and a high bound, as for NSGA-II; its start and step are not used.
By default a run stops after 20 evaluations per variable. [`Ego::with_max_evaluations`] sets
the budget, [`Ego::with_target`] stops at a good enough value, and
[`Ego::with_tolerance_improvement`] stops once no point is expected to gain more than a set
amount. Jones and co-authors stop at 1% of the best value's size.

A worked example, on Branin's function of two variables: its least value is
`5/(4π) ≈ 0.397887`, at three points. In this example's runs, the initial design is 20 points.
Thirty more get within 1% of the minimum from every one of 20 seeds. This is the example on
[`Ego`]'s API page, which runs as a test:

```rust,ignore
use hpr_analysis::optimize::Variable;
use hpr_analysis::optimize::benchmark::global::{BRANIN_MINIMUM, branin};
use hpr_analysis::optimize::ego::Ego;

let variables = vec![
    Variable::new("x0", 2.5, 3.0)?.within(-5.0, 10.0)?,
    Variable::new("x1", 7.5, 3.0)?.within(0.0, 15.0)?,
];
let optimum = Ego::new(variables)?.with_max_evaluations(50)?.minimize(1, branin)?;
assert_eq!(optimum.evaluations, 50);
assert!(optimum.value < 1.01 * BRANIN_MINIMUM);
```

### Checked against

[`tests/ego.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/ego.rs)
runs EGO from seeds 1 to 20 on two test functions with several local minima each. The rule is
Jones and co-authors' measure of success: the best value within 1% of the minimum's size. It
and the budget were set before the 20 seeds were run.

| Function | Variables | Minimum | Budget | Worst of 20 runs |
|---|---|---|---|---|
| Branin | 2 | 0.397887 | 50 evaluations | 0.118% above |
| Hartmann 3 | 3 | −3.86278 | 50 evaluations | 0.124% above |

Branin's minimum is exact. Hartmann's is the value printed to six figures, and the test checks
it by polishing with CMA-ES from the printed point. Unit tests in
[`optimize/ego.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/src/optimize/ego.rs)
check that the surrogate passes through its points, its fit on two points against the formula
worked by hand, the expected improvement's limits, and that the initial design fills every
slice. No outside implementation is run against it, as for the other optimizers: the minima are
known, and the milestone asks for them.

**Not yet:** on Hartmann's six-variable function, 6 of 10 runs end at its local minimum, −3.20,
after 100 evaluations, and one run takes over half a minute. That is the next increment,
[M6.2d2](decisions-and-roadmap.md#m6-2d2) (EGO on six variables).
EGO takes no limits, whole-number variables or noisy outputs yet
([ADR-142](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-142-few-evaluations-by-ego-2026-10-02)).

## Left out

- A choice is a whole number in a list you order. There is no separate handling for choices with
  no order, beyond putting alike ones next to each other.
- Limits are inequalities only. For an equality `h = 0`, write `|h| − ε ≤ 0` with a small `ε`.
- NSGA-II takes only continuous variables with two bounds: no choices from a list yet. It
  runs the generations it is given, with no test of when its front has settled.
- EGO doesn't yet find Hartmann's six-variable minimum reliably, and takes no limits,
  whole-number variables or noisy outputs.
- No optimizing of a Monte Carlo run's statistics, such as the chance of landing within a
  distance.
- No command-line or Python front end yet.

The API reference is
[`hpr_analysis::optimize`](api/hpr_analysis/optimize/index.html).

[`Variable`]: api/hpr_analysis/optimize/struct.Variable.html
[`Variable::integer`]: api/hpr_analysis/optimize/struct.Variable.html#method.integer
[`Evaluation`]: api/hpr_analysis/optimize/struct.Evaluation.html
[`Ego`]: api/hpr_analysis/optimize/ego/struct.Ego.html
[`Ego::with_max_evaluations`]: api/hpr_analysis/optimize/ego/struct.Ego.html#method.with_max_evaluations
[`Ego::with_target`]: api/hpr_analysis/optimize/ego/struct.Ego.html#method.with_target
[`Ego::with_tolerance_improvement`]: api/hpr_analysis/optimize/ego/struct.Ego.html#method.with_tolerance_improvement
[adr-139]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-139-optimization-constraints-by-debs-feasibility-rules-2026-10-01
[`Stop`]: api/hpr_analysis/optimize/cmaes/enum.Stop.html
[`Optimum`]: api/hpr_analysis/optimize/cmaes/struct.Optimum.html
[`Run`]: api/hpr_analysis/optimize/cmaes/struct.Run.html
[`Cmaes::with_population`]: api/hpr_analysis/optimize/cmaes/struct.Cmaes.html#method.with_population
[`Nsga2`]: api/hpr_analysis/optimize/nsga2/struct.Nsga2.html
[`Goals::constrained`]: api/hpr_analysis/optimize/nsga2/struct.Goals.html#method.constrained
[`Goals::failed`]: api/hpr_analysis/optimize/nsga2/struct.Goals.html#method.failed
[`generational_distance`]: api/hpr_analysis/optimize/nsga2/fn.generational_distance.html
