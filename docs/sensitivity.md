# Sensitivity analysis

A [Monte Carlo](glossary.md#monte-carlo) run shows how far a rocket's apogee spreads. It doesn't
say which input causes the spread. [Sensitivity analysis](glossary.md#sensitivity-analysis) does:
it ranks the uncertain inputs by how much each moves a result, so you know which to measure more
carefully and which you can stop worrying about. This page shows hpr-sim's two methods. They are
Morris's screening, which is cheap, and Sobol' indices, which cost more and say more. It runs
both on two test functions whose answers are known, then screens a rocket's apogee and landing.
It needs some Rust, and follows on from [Monte Carlo dispersion](monte-carlo.md).

> **How far to trust it.** The two methods are tested against answers known in closed form. On a
> rocket, the ranking they give is only as good as the ranges you give and hpr-sim's flight
> models, which are not yet validated against real flights ([Accuracy](accuracy.md)).
>
> - **Tested:** both methods match the known answers of two standard test functions within four
>   of their own [standard errors](glossary.md#standard-error). Over 500 to 1,000 runs with
>   different seeds, the error a run reports for a Sobol' index, or for a Morris `μ*` of the g
>   function, is the scatter the runs really show. The same
>   [seed](glossary.md#seed) gives the same numbers, bit for bit
>   ([`tests/sensitivity.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/sensitivity.rs)).
> - **Not checked:** whether a rocket's ranking matches what real flights would show.
> - **Left out:** inputs that depend on each other, inputs that aren't spread evenly over a
>   range, and the [Sobol' indices](glossary.md#sobol-index) of pairs of inputs. A Sobol'
>   analysis costs thousands of runs per input, too many flights for this page's rocket example,
>   which uses only the cheaper Morris screening.

## Inputs as factors

Each uncertain input is a *factor*: a name and a range, with every value in the range equally
likely ([`Factor`]). The drag might be anywhere from 10% below its estimate to 10% above, say, or
the wind anywhere from calm to 8 m/s. Factors are independent: a heavier rocket isn't also
draggier.

Both methods work the same way. They lay out the points to try (a *design*), you run your model
at each point in order, and they analyse what came back. The model can be a whole flight, with the
factors set into its inputs, or any function. So you can run the points however you like, on many
threads or many machines. Each method also has a shortcut that takes a closure and runs the points
itself.

## Morris screening

Morris's method nudges one factor at a time and watches the output move. It works on a grid: each
factor's range is cut into `p` evenly spaced *levels* (4 is usual). A step moves one factor `p/2`
levels (two of four), which is a fraction `Δ = p / (2(p − 1))` of its range: 2/3 for four
levels.

The change in the output over one step, divided by `Δ`, is an
[elementary effect](glossary.md#elementary-effect). It is the change the factor would cause across
its whole range, at the slope measured over the step. A *path* starts at a random grid point and
steps each factor once, in a random order, so `k` factors take `k + 1` runs per path. Each step
gives one effect. After `r` paths, each factor has `r` effects, and the screening reports three
numbers for each factor ([`ElementaryEffects`]):

| Number | What it is | What it says |
|---|---|---|
| `μ*` (`mean_absolute`) | the mean of the effects' sizes | how much the factor matters: the ranking |
| `σ` (`standard_deviation`) | how much the effects vary | a large `σ` means the factor's effect bends, or depends on other factors |
| `μ` (`mean`) | the mean of the effects, with their signs | effects of opposite sign cancel here, so use `μ*` to rank |

A factor with `μ*` near zero can be left at its planned value. The screening also gives `μ*`'s
standard error, which shrinks as `1/√r`: ten paths are usually enough to rank, and the cost is
`r (k + 1)` runs.

For example, Ishigami's test function (below) has `a sin² x₂` as a term, with `a` = 7. On a
four-level grid from −π to π, a step of `x₂` moves `sin²` by exactly 3/4, up or down, so every
effect of `x₂` is `± (3/4)(7)/(2/3) = ± 7.875`. The example's screening finds `μ* = 7.875` with no error, and
`μ` near zero.

M. D. Morris, "Factorial sampling plans for preliminary computational experiments",
*Technometrics* 33(2), 161–174, 1991, defines the effects, the grid and the paths. F. Campolongo,
J. Cariboni and A. Saltelli, "An effective screening design for sensitivity analysis of large
models", *Environmental Modelling & Software* 22, 1509–1518, 2007, add `μ*`. They found, by
experiment rather than proof, that it ranks factors in the same order as the total Sobol' index.

That is usual, not certain. On Ishigami's function, Morris puts `x₂` first (`μ*` 7.875 against
7.704 for `x₁`), while the total index puts `x₁` first (0.558 against 0.442). The two measure
different things, and a four-level grid sees `sin² x₂` at only two values. So treat factors whose
`μ*`s are close as tied, and use Sobol' indices to settle their order if it matters.
[`hpr_analysis::sensitivity::morris`](api/hpr_analysis/sensitivity/morris/index.html) gives the
equations and page numbers.

## Sobol' indices

Sobol's method splits the output's [variance](glossary.md#variance) (the square of its
[standard deviation](glossary.md#standard-deviation)) among the factors. It reports two shares for
each factor ([`SobolIndex`]):

- The **first-order index** `Sᵢ` is the share of the variance the factor causes alone. If you
  could pin the factor to its true value, the variance would drop by this share, on average.
- The **total index** `S_Tᵢ` is the share it has any part in, alone or together with others. If
  you pinned every *other* factor, this share of the variance would remain, on average.

`Sᵢ ≤ S_Tᵢ`. The gap is how much the factor acts through others. A factor with a total index near
zero can be left at its planned value.

The analysis draws two independent tables, `A` and `B`, each of `N` rows with every factor spread
over its range. Then, for each factor `i`, it builds a third table: `A` with factor `i`'s column
taken from `B`. Comparing the outputs of the three tables row by row gives both indices. A.
Saltelli and others ("Variance based sensitivity analysis of model output", *Computer Physics
Communications* 181, 259–270, 2010) give the estimates used here, the best of those they compared.
The cost is `N (k + 2)` runs, and the error shrinks as `1/√N`: with `N` = 8,192, an index comes
out to about ±0.01.

Each index comes with its standard error, found by the
[delta method](https://en.wikipedia.org/wiki/Delta_method) from how much the rows scatter.
[`hpr_analysis::sensitivity::sobol`](api/hpr_analysis/sensitivity/sobol/index.html) gives the
formulas.

## Checked against

Two test functions have Sobol' indices known in closed form
([`hpr_analysis::sensitivity::benchmark`](api/hpr_analysis/sensitivity/benchmark/index.html)):

- **Ishigami and Homma's function**, `y = sin x₁ + 7 sin² x₂ + 0.1 x₃⁴ sin x₁`, each `x` from −π
  to π. `x₃` does nothing alone (`S₃ = 0`), yet it is part of a quarter of the variance, through
  `x₁`. The closed forms are I. M. Sobol' and Y. L. Levitan's (*Computer Physics Communications*
  117, 1999, p. 57).
- **Sobol's g function**, a product of one term per factor, each with a weight `aᵢ`: the factor
  matters most at `aᵢ = 0` and hardly at all at 99. The closed forms are Saltelli and others' (2010,
  p. 268). The tests use `aᵢ` = 0, 1, 4.5, 9 and four 99s, spanning the four classes Marrel and
  others (2008) name, as the
  [SFU library of test functions](https://www.sfu.ca/~ssurjano/gfunc.html) quotes them: very
  important at 0, relatively important at 1, non-important at 9 and non-significant at 99.

For Morris there is also an exact answer. A screening estimates the moments of a finite set of
effects: every step on the grid. [`Morris::population`] runs the model at every grid point and
computes them exactly: the example's "whole grid" column. For both functions they also follow in closed form, which the tests check.

The tests in
[`tests/sensitivity.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-analysis/tests/sensitivity.rs)
check four things:

- Every Sobol' index of both functions, from 32,768 and 16,384 rows, lies within four standard
  errors of its closed form.
- Every Morris `μ*`, from 1,000 paths, lies within four standard errors of the exact one.
- The standard errors are honest. Take each estimate's distance from the known answer, divided by
  its standard error. Over `n` seeds, these have a mean within `4/√n` of 0 and a standard
  deviation within `4/√(2n)` of 1. This holds for every Sobol' index of Ishigami's function
  (1,000 seeds) and of the g function's first four factors (500 seeds), and for Morris's `μ*` of
  those four (1,000 seeds). The same test checks that standard errors 15% too small would fail.
  A separate unit test computes each Sobol' standard error a second way, from the raw row means.
- On the g function, twenty Morris paths rank the four factors that matter in the order of their
  total indices.

## An example

The example program
[`crates/hpr/examples/sensitivity.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/sensitivity.rs)
runs both methods on the two test functions and prints them beside the known answers. Then it
screens the rocket of [Monte Carlo dispersion](monte-carlo.md) over six inputs. Run it from a copy
of the repository with:

```text
cargo run --example sensitivity -p hpr
```

Excerpts from it follow. The test functions take a closure:

```rust,ignore
let ishigami = Ishigami::STANDARD;
let sobol = Sobol::new(Ishigami::factors()?, 8192)?;     // N = 8,192 rows: 40,960 runs
let indices = sobol.indices(seed, |x| ishigami.evaluate(x))?;
```

For the rocket, the screening lays out its points and the program flies each one. A Monte Carlo
set-up with no dispersion turns a *draw* (each input's factor or offset) into the flight it flies
([`MonteCarlo::inputs`]):

```rust,ignore
let monte_carlo = MonteCarlo::new(launch.inputs()?, Dispersion::default())?;
let nominal = monte_carlo.draw(seed, 0);                 // every input as planned
let factors = vec![
    Factor::new("dry mass factor", 0.95, 1.05)?,
    Factor::new("drag factor", 0.9, 1.1)?,
    Factor::new("impulse factor", 0.94, 1.06)?,
    Factor::new("wind speed (m/s)", 0.0, 8.0)?,
    Factor::new("wind turn (°)", -30.0, 30.0)?,
    Factor::new("rail angle (°)", 80.0, 90.0)?,
];
let morris = Morris::new(factors, 4, 10)?;               // 4 levels, 10 paths: 70 flights
let design = morris.design(seed);
for x in design.points() {
    let mut draw = nominal.clone();
    draw.drag_scale = x[1];                              // a factor on the drag
    draw.wind_speed_scale = x[3] / 4.0;                  // a factor on the 4 m/s forecast
    draw.rail_elevation_offset_rad = (x[5] - 85.0).to_radians(); // an offset from 85°
    // ... and the dry mass, impulse and wind turn the same way
    let flight = monte_carlo.inputs(&draw)?.fly()?;
    apogees.push(flight.apogee.as_ref().ok_or("no apogee")?.height_above_ground_m);
}
let apogee = design.analyse(&apogees)?;                  // the same flights, any output
```

The six inputs, with ranges made up for the example:

| Input | Range | Planned value | What it is |
|---|---|---|---|
| dry mass factor | 0.95 to 1.05 | 1 | the rocket's mass without its motor, ±5% |
| drag factor | 0.9 to 1.1 | 1 | the zero-lift drag, ±10% |
| impulse factor | 0.94 to 1.06 | 1 | the motor's total impulse, ±6%; NFPA 1125 caps a motor type's standard deviation at 6.7% |
| wind speed | 0 to 8 m/s | 4 m/s | the wind at every height |
| wind turn | −30° to 30° | 0° | the wind's direction turned clockwise from the forecast's west |
| rail angle | 80° to 90° | 85° | the rail's angle above the horizon; 90° is vertical |

A [`Draw`] can also move a stage's centre of mass, the motor's burn time and ejection delay, the
rail's heading and a recovery device's delay ([What each dispersion does](monte-carlo.md#what-each-dispersion-does)).

It prints:

<!-- quote: crates/hpr/examples/sensitivity.output.txt -->
```text
Sobol' indices of Ishigami's function (a = 7, b = 0.1), 8192 rows, 40960 runs
        first order: known  estimate ± error      total: known  estimate ± error
x1                 0.3139    0.3101 ± 0.0108             0.5576    0.5453 ± 0.0154
x2                 0.4424    0.4549 ± 0.0098             0.4424    0.4494 ± 0.0074
x3                 0.0000   -0.0071 ± 0.0099             0.2437    0.2404 ± 0.0050

Sobol' indices of the g function, a = 0, 1, 4.5, 9, 99 (four times), 8192 rows, 81920 runs
        first order: known  estimate ± error      total: known  estimate ± error
x1                 0.7162    0.7162 ± 0.0124             0.7871    0.7844 ± 0.0127
x2                 0.1790    0.1683 ± 0.0077             0.2422    0.2444 ± 0.0050
x3                 0.0237    0.0246 ± 0.0029             0.0343    0.0353 ± 0.0008
x4                 0.0072    0.0059 ± 0.0017             0.0105    0.0109 ± 0.0003
x5                 0.0001    0.0001 ± 0.0002             0.0001    0.0001 ± 0.0000
(x6 to x8 are as x5)

Morris screening of Ishigami's function, 4 levels, 100 paths, 400 runs
            μ*: whole grid  estimate ± error       σ: whole grid  estimate
x1                     7.704     7.079 ± 0.625                6.249     6.249
x2                     7.875     7.875 ± 0.000                7.875     7.915
x3                     6.249     6.124 ± 0.628                8.837     8.784

Morris screening of the Monte Carlo page's rocket, 4 levels, 10 paths, 70 flights
Not yet validated: see the Accuracy page before trusting these numbers.
Each effect is the change across the input's whole range, m.
                     apogee: μ*  ± error      σ     landing distance: μ*  ± error      σ
dry mass factor             36.0 ±   1.8    5.6                       70.0 ±  22.5   72.0
drag factor                112.9 ±   3.2   10.3                       80.5 ±  18.0   57.0
impulse factor             118.3 ±   2.3    7.1                      107.5 ±  23.7   75.1
wind speed (m/s)            50.3 ±   4.1   13.1                     1417.5 ± 101.4  320.7
wind turn (°)                4.4 ±   1.0    5.4                       53.4 ±  10.5   64.5
rail angle (°)              68.4 ±   3.1    9.9                      350.2 ±  17.6  231.0
```

How to read it:

- **The test functions** land within 1.4 standard errors of the known answers, from the printed
  digits. `S₃` comes out slightly below zero: the estimate of a share that is really zero
  scatters around zero. The g function's `x5` shows "± 0.0000" because its error is below
  0.00005.
- **Ishigami's Morris screening** at 400 runs puts `x2`, `x1`, `x3` in the whole grid's order, but
  the gaps are only about one standard error, so at 100 paths they aren't really ranked. Its `σ`s
  are as large as its `μ*`s, the sign of a function that bends or whose factors act together.
- **The rocket's apogee** moves most with the motor's impulse (118 m across ±6%) and the drag
  (113 m across ±10%), then the rail's angle (68 m across 80° to 90°), the wind's speed (50 m)
  and the dry mass (36 m across ±5%). The wind's direction hardly matters (4 m). The `σ`s are
  small beside the `μ*`s: each input acts nearly in a straight line and alone. The exception is
  the wind's direction, whose small effect changes sign. The impulse and the drag are 1.4 errors
  apart: ten paths don't settle which comes first.
- **The landing** is the wind's: 1,418 m across calm to 8 m/s, then the rail's angle (350 m).
  Here the `σ`s are large, so these effects bend or depend on the other inputs.

The ranges are made up for the example. With your own rocket, use the ranges your measurements
support; a factor's effect grows with its range.

## Choosing the numbers

- **Levels:** four, as Morris and Campolongo use. More levels lets a path reach more of each
  range, at the same cost per path. The number must be even.
- **Paths:** ten to twenty to rank. Check `μ*`'s standard error: two factors whose `μ*`s differ
  by less than a couple of errors aren't ranked yet.
- **Rows:** a Sobol' analysis needs about 8,000 rows, each `k + 2` runs, to pin an index to
  ±0.01. Use Morris first to find the few factors that matter, then Sobol' on those if you need
  the shares or the order of two close ones.
- **Normal inputs:** give each one the same multiple of its standard deviation, such as ±2, so
  they are compared alike. A range spreads the input evenly, which weighs its ends more than a
  normal spread does.

## Left out

- Factors are uniform over their ranges and independent. A normal input can be given as a range
  about its mean, such as ±2 standard deviations, which spreads it more evenly than it really is.
- Sobol' rows are plain pseudo-random draws, not quasi-random ones (evenly spread sequences, such
  as Sobol's own), which converge faster.
- No second-order Sobol' indices (the share of each pair alone).
- No choice of the most spread-out Morris paths among many (Campolongo's improvement).
- No command-line or Python front end yet.

The API reference is
[`hpr_analysis::sensitivity`](api/hpr_analysis/sensitivity/index.html).

[`Factor`]: api/hpr_analysis/sensitivity/struct.Factor.html
[`ElementaryEffects`]: api/hpr_analysis/sensitivity/morris/struct.ElementaryEffects.html
[`SobolIndex`]: api/hpr_analysis/sensitivity/sobol/struct.SobolIndex.html
[`Morris::population`]: api/hpr_analysis/sensitivity/morris/struct.Morris.html#method.population
[`MonteCarlo::inputs`]: api/hpr_analysis/montecarlo/struct.MonteCarlo.html#method.inputs
[`Draw`]: api/hpr_analysis/montecarlo/struct.Draw.html
