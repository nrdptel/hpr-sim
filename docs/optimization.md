# Optimization

Sometimes you know the result you want and need the design that gives it: that is
[optimization](glossary.md#optimization). That might be a rocket
that reaches exactly 3,048 m (10,000 ft) for a competition, or the lightest fins that keep it
stable. An *optimizer* searches for that design. It tries designs, flies each one, and uses what
it learns to choose better ones, until it finds the best it can. This page shows hpr-sim's
optimizer, [CMA-ES](glossary.md#cma-es). It runs on test functions whose answers are known, then
finds the nose ballast and body length that send a rocket to 3,048 m with a chosen stability
margin. It needs some Rust.

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
> - **Checked by re-flying:** the rocket design the example finds is flown again from scratch,
>   and again with the flight's numerical integration 100 times stricter
>   ([tolerances](glossary.md#tolerance)). Both reach apogee within 0.1 m of 3,048 m (within
>   2 mm, measured).
> - **Limits** (a minimum stability margin, say): held to three test problems whose answers on
>   their limits are known exactly, from 20 seeds each, to 10⁻¹⁰
>   ([`tests/constrained.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/constrained.rs)).
>   No rocket limit is checked yet ([Limits on a design](#limits-on-a-design)).
> - **Left out, for now:** discrete choices (which motor, which catalogue part), trade-offs between goals (several
>   goals can only be folded into one number, as the example does), and optimizing
>   a [Monte Carlo](glossary.md#monte-carlo) run's statistics. They are the next steps of
>   [M6.2](decisions-and-roadmap.md#m6-2), the optimization milestone. There is no command-line or
>   Python front end yet.

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
not handled yet: check those yourself, or add them to the number you minimize.

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
- **Evaluations:** a cap on flights, 10,000 by default. The example's two variables needed 300
  flights; the ten-variable test functions take 1,600 to 6,500 evaluations to converge.
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
point, also when it starts where the limit is broken. These are test problems; no rocket limit
is checked yet. That comes with [M6.2b2](decisions-and-roadmap.md#m6-2b2).

## Left out

- Variables are continuous. Discrete choices (a motor, a catalogue part) come next, in
  [M6.2b2](decisions-and-roadmap.md#m6-2b2). Until then, run the optimizer once for each motor you
  are considering and compare the results.
- Limits are inequalities only. For an equality `h = 0`, write `|h| − ε ≤ 0` with a small `ε`.
- No trade-offs between goals (a Pareto front): several goals can only be folded into one
  number, as the example does.
- No optimizing of a Monte Carlo run's statistics, such as the chance of landing within a
  distance.
- No command-line or Python front end yet.

The API reference is
[`hpr_analysis::optimize`](api/hpr_analysis/optimize/index.html).

[`Variable`]: api/hpr_analysis/optimize/struct.Variable.html
[`Evaluation`]: api/hpr_analysis/optimize/struct.Evaluation.html
[adr-139]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-139-optimization-constraints-by-debs-feasibility-rules-2026-10-01
[`Stop`]: api/hpr_analysis/optimize/cmaes/enum.Stop.html
[`Optimum`]: api/hpr_analysis/optimize/cmaes/struct.Optimum.html
[`Run`]: api/hpr_analysis/optimize/cmaes/struct.Run.html
[`Cmaes::with_population`]: api/hpr_analysis/optimize/cmaes/struct.Cmaes.html#method.with_population
