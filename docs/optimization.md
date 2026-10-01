# Optimization

Sometimes you know the result you want and need the design that gives it: that is
[optimization](glossary.md#optimization). That might be a rocket
that reaches exactly 3,048 m (10,000 ft) for a competition, or the lightest fins that keep it
stable. An *optimizer* searches for that design. It tries designs, flies each one, and uses what
it learns to choose better ones, until it finds the best it can. This page shows hpr-sim's
optimizer, [CMA-ES](glossary.md#cma-es). It runs on four test functions whose answers are known, then finds the nose
ballast and body length that send a rocket to 3,048 m. It needs some Rust.

> **How far to trust it.** The optimizer is tested against answers known exactly, and against the
> reference implementation by its author. On a rocket, its answer is only as good as hpr-sim's
> flight models, which are not yet validated against real flights ([Accuracy](accuracy.md)).
>
> - **Tested:** on four standard test functions of ten variables, every run from 20 seeds reaches
>   the known minimum to within 10⁻¹⁰, except 3 of 20 on Rosenbrock's function, which end in its
>   known local minimum. The number of evaluations each run takes is within 5% of pycma's, the
>   author's own Python implementation, run from the same starting points. A run is repeated bit
>   for bit from its [seed](glossary.md#seed)
>   ([`tests/optimize.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/optimize.rs)).
> - **Checked by re-flying:** the rocket design the example finds is flown again from scratch,
>   and again with the flight's numerical integration 100 times stricter. Both land within 0.1 m
>   of the 3,048 m target.
> - **Left out, for now:** discrete choices (which motor, which catalogue part), limits other than
>   each variable's range (a minimum stability margin, say), several goals at once, and optimizing
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

The formulas and default settings are the tutorial's (Appendix A and Table 1). There is one
difference: like the tutorial's own code, hpr-sim gives the worse half of each generation no
weight. The newer *active* variant pushes the cloud away from them.
[`hpr_analysis::optimize::cmaes`](api/hpr_analysis/optimize/cmaes/index.html) lists every equation.

### Bounds

A sampled design outside a variable's bounds is thrown away and drawn again, the tutorial's
simplest way of handling bounds. It works well while the answer lies inside the range. If it
lies on a bound, the run slows down near it. If no design of a generation falls inside the range
in 1,000 tries, the run stops and says so.

### When a run stops

A run stops at the first of these ([`Stop`]):

| Stop | When |
|---|---|
| `Target` | a value at or below the target you set: a miss small enough |
| `Evaluations` | the number of evaluations you allowed is used up (10,000 by default) |
| `TolX` | the cloud has shrunk below a tolerance in every variable: the run has converged |
| `TolFun` | the values have stopped changing by more than a tolerance |
| `Condition` | the cloud is 10⁷ times longer than it is wide, too thin to keep accurate |
| `Bounds` | no design of a generation fell inside the bounds |

The result ([`Optimum`]) gives the best design found, its value, how many evaluations the run made,
and why it stopped.

## Checked against

Four test functions from CMA-ES's authors (N. Hansen, S. D. Müller and P. Koumoutsakos,
*Evolutionary Computation* 11(1), 2003, Table 1), each with ten variables
([`hpr_analysis::optimize::benchmark`](api/hpr_analysis/optimize/benchmark/index.html)):

| Function | What it tests | Minimum |
|---|---|---|
| sphere, `Σ xᵢ²` | the step size alone | 0 at `x = 0` |
| ellipsoid | stretching the cloud: one variable matters a million times more than another | 0 at `x = 0` |
| rotated ellipsoid | the same, tilted so that no single variable lines up with it | 0 at `x = 0` |
| Rosenbrock's function | following a long, curved valley | 0 at `x = 1` |

Each run starts with steps of 0.5 and goes until the value is 10⁻¹⁰ or less, the stopping value
of the 2003 paper. The tests in
[`tests/optimize.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/optimize.rs)
run each function from 20 seeds and check:

- Every sphere and ellipsoid run reaches the minimum, which puts its best point within 10⁻⁵ of
  `x = 0`.
- 17 of 20 Rosenbrock runs reach the minimum at `x = 1`. The other 3 end in the function's local
  minimum near `(−1, 1, …, 1)`. The test requires at least 17. CMA-ES's authors note that local
  minimum, and report 1 to 3 runs of 20 missing the global one at 4 to 16 variables (Kern, Hansen
  and Koumoutsakos, 2006). pycma ends in the local minimum in 1 of 20.
- The median number of evaluations is within 25% of pycma's from the same starts (the test's
  bound). Measured, they are within 5%:

| Function | hpr-sim | pycma 4.5.0 |
|---|---|---|
| sphere | 1,635 | 1,640 |
| ellipsoid | 5,920 | 5,910 |
| rotated ellipsoid | 6,010 | 5,930 |
| Rosenbrock | 6,445 | 6,195 |

- Tilting the ellipsoid changes the count by under 10%: the cloud learns the tilt.
- The default settings that pycma also takes from the tutorial's Table 1 match its values to
  rounding. pycma departs from the table in a few places by its author's choice, such as how fast
  the cloud's size adapts. That is why the counts differ a little.

Without the step that learns the cloud's shape from a whole generation (the *rank-μ update*), the
ellipsoid takes a third more evaluations (7,875) and the test fails. So the comparison catches a
real fault.

pycma is run by
[`validation/oracles/cmaes/pycma_runs.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/cmaes/pycma_runs.py),
whose results are committed beside the test.

## An example

The example program
[`crates/hpr/examples/optimization.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/optimization.rs)
runs CMA-ES on the four test functions. Then it takes a 66 mm rocket on a J760 and finds the nose
ballast and body tube length that send it to 3,048 m above the pad. The rocket starts from a 3 m
rail at 85°, into 5 m/s of wind. Run it from a copy of the repository with:

```text
cargo run --example optimization -p hpr
```

The heart of it:

```rust,ignore
let variables = vec![
    Variable::new("nose ballast (kg)", 0.3, 0.15)?.within(0.0, 1.0)?,
    Variable::new("body tube (m)", 1.0, 0.1)?.within(0.6, 1.4)?,
];
let optimizer = Cmaes::new(variables)?
    .with_target(0.1 * 0.1)?          // stop at a miss of 0.1 m or less
    .with_max_evaluations(1_000)?;
let optimum = optimizer.minimize(seed, |x| {
    match fly(x[0], x[1], FlightSettings::default()) {
        Ok(flight) => flight
            .apogee_m()
            .map_or(f64::INFINITY, |apogee| (apogee - 3048.0).powi(2)),
        Err(_) => f64::INFINITY,       // a design the simulator refuses ranks last
    }
})?;
```

`fly` builds the rocket with that ballast and body length and flies it. A design that can't fly
returns infinity, which ranks below every real flight. Then the example flies the winner twice
more: once from a fresh build, which must give the optimizer's apogee to the last bit, and once
with the integrator's tolerances 100 times tighter, which must still hit within 0.1 m.

It prints:

<!-- quote: crates/hpr/examples/optimization.output.txt -->
```text
CMA-ES on four test functions, 10 variables, run to f ≤ 1e-10 (seed 2026)
function            start   evaluations   best value   distance to the minimum
sphere                1.0          1660      9.4e-11                    9.7e-6
ellipsoid             1.0          5900      8.3e-11                    4.0e-6
rotated ellipsoid     1.0          6040      9.4e-11                    1.4e-6
Rosenbrock            0.0          6430      7.1e-11                    2.4e-6

Hit 3,048 m with a 66 mm rocket on a J760 by its nose ballast and body length
Not yet validated: see the Accuracy page before trusting these numbers.
Start: 0.300 kg of ballast, a 1.000 m body: apogee 3128.8 m
Found: 0.995 kg of ballast, a 0.806 m body, in 162 flights (27 generations, stopped: Target)
Flown again: apogee 3048.017 m (miss +0.017 m)
Flown again, tolerances 100 times tighter: apogee 3048.014 m (miss +0.014 m)
```

The rocket started 81 m too high. The optimizer found a hit in 162 flights, about 8 s in a
debug build. It didn't find *the* answer, because there is a whole curve of them. More ballast
and a shorter body cancel out, and two variables against one target leave one degree of freedom.
The run stops at the first hit it finds, which here is near the heavy end of the ballast's range.
To choose among the hits, add a second wish to the number you minimize: a small penalty on the
ballast mass, say. Or fix one variable and optimize the other alone. Limits such as a minimum
stability margin are not handled yet, so check the winner's margin yourself.

## Choosing the numbers

- **Steps:** about a quarter to a third of the range the answer is likely in. Give the variables
  units that make their steps similar in size: the tutorial warns against steps that differ by
  several orders of magnitude.
- **Target:** for a target apogee, the squared miss you accept: `0.1 * 0.1` for 0.1 m. Without a
  target, a run goes on until it converges, which can take many more flights than a hit needs.
- **Evaluations:** a cap on flights. Two variables usually need 100 to 300 flights to converge;
  ten need thousands.
- **Population:** leave it at the default unless the output has many local minima. Then a larger
  population ([`Cmaes::with_population`]) searches more widely, at more flights per generation.
- **Seed:** a different seed gives a different run. Rerun with two or three seeds if the answer
  matters: if they agree, the answer is not luck.

## Left out

- Variables are continuous. Discrete choices (a motor, a catalogue part) come next, in
  [M6.2](decisions-and-roadmap.md#m6-2).
- No limits other than a variable's range, and an answer on a bound is reached only slowly.
- One goal at a time: no trade-offs between two goals (a Pareto front).
- No optimizing of a Monte Carlo run's statistics, such as the chance of landing within a
  distance.
- The run evaluates its designs one after another; [`Run`] (start, then `tell`) lets you evaluate
  a generation's designs any way you like, on several threads, say.
- No command-line or Python front end yet.

The API reference is
[`hpr_analysis::optimize`](api/hpr_analysis/optimize/index.html).

[`Variable`]: api/hpr_analysis/optimize/struct.Variable.html
[`Stop`]: api/hpr_analysis/optimize/cmaes/enum.Stop.html
[`Optimum`]: api/hpr_analysis/optimize/cmaes/struct.Optimum.html
[`Run`]: api/hpr_analysis/optimize/cmaes/struct.Run.html
[`Cmaes::with_population`]: api/hpr_analysis/optimize/cmaes/struct.Cmaes.html#method.with_population
