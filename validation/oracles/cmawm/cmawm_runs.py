"""Reference runs of CMA-ES with margin (CMAwM) on three mixed-integer test functions.

Checks: hpr's mixed-integer CMA-ES (`hpr_analysis::optimize`). This script shares no code with it.

CMA-ES with margin is Hamano, Saito, Nomura and Shirakawa's handling of integer and binary
variables in CMA-ES ("CMA-ES with Margin: Lower-Bounding Marginal Probability for Mixed-Integer
Black-Box Optimization", GECCO 2022, arXiv:2205.13482). It keeps each discrete variable's marginal
probability of leaving the integer the mean rounds to at least a margin alpha, by moving the mean
and by scaling a diagonal matrix A that stretches the discrete axes before they are rounded. The
outside implementation run here is `cmaes.CMAwM` from the `cmaes` package (MIT), which its
docstring says is adapted from the authors' code (EvoConJP/CMA-ES_with_Margin, MIT).

The test functions are the paper's (section 5). The first N/2 variables are continuous, the rest
discrete; xbar is the candidate after rounding, the vector the function is evaluated at:

- sphere_int:      f = sum_j xbar_j^2, integers in {-10, ..., 10};
- ellipsoid_int:   f = sum_j (1000^((j - 1)/(N - 1)) xbar_j)^2, j = 1..N, integers in
                   {-10, ..., 10} (the integers carry the largest coefficients);
- sphere_one_max:  f = sum_continuous xbar_j^2 + N_bi - sum_binary xbar_j, binaries in {0, 1}.

Each has its minimum 0, at the origin (all ones for the binaries). Each runs with N = 10 and 20.

Settings, the same for every run:

- seeds k = 0..19. The initial mean's continuous and integer entries are drawn uniformly from
  [1, 3] by `numpy.random.RandomState(1000 + k).uniform(1, 3, N)`; binary entries are then set to
  0.5. The paper's section 3 starts its binaries at 0; 0.5 sits on the rounding threshold instead.
  Every run's initial mean is written out in full, so a reader need not redraw it.
- `CMAwM(mean, sigma=1, bounds, steps, seed=k)`: C0 = I, cmaes's default population
  lambda = 4 + floor(3 ln N) (10 for N = 10, 12 for N = 20), and its default margin
  alpha = 1/(N lambda). Continuous bounds are (-inf, inf), so no candidate is ever resampled.
- variant "active": cmaes as shipped, with its negative recombination weights (active CMA).
- variant "positive": right after construction, the weights after the mu-th, which cmaes keeps in
  `CMA._weights` with the positive ones, are set to 0. That array is read in exactly three places
  in `cmaes/_cma.py` (0.13.1): the mean's recombination (line 336, which uses only the first mu),
  the rank-mu term's weights `w_io` (lines 364-368, where a zero weight gives a zero term), and the
  covariance's decay factor `1 + c1 delta - c1 - cmu sum(w)` (line 381). With the negative weights
  zero, sum(w) is the positive weights' sum, 1 to rounding, and no negative-weight term remains: the
  update is the non-active one of Hansen's tutorial (arXiv:1604.00772, eq. 47 with w_i = 0 for
  i > mu). The script asserts both after patching.

Each generation samples lambda candidates, evaluates them all, then:

1. target: if the best value so far is <= 1e-10, the run succeeded and stops (no tell);
2. otherwise the generation is told to the optimizer;
3. min_eigenvalue: stop if the smallest eigenvalue of sigma^2 C is below 1e-30;
4. condition: stop if C's condition number exceeds 1e14 (or C is no longer positive definite);
5. evaluations: stop if N * 10^4 evaluations or more have been used.

Evaluations are counted per whole generation, lambda at a time, so a run that ends at the cap can
pass it by less than lambda (200,004 for N = 20). cmaes's own `should_stop` is not consulted. These
stops are the paper's (sections 3 and 5.2) plus the evaluation cap.

For each run it records the seed, the initial mean, whether the target was reached, the evaluations
used, the best value, the stop reason and the generations evaluated. Each case and variant has a
summary over the 20 seeds: the successes, and the median, quartiles (`numpy.percentile`, linear
interpolation), least and most evaluations of the successful runs (null with no success), the
paper's way of reporting evaluation counts. The fixture also records the strategy parameters
cmaes used for each N (read from the `CMA` object), Table 1's c_mu beside cmaes's for comparison,
and the versions of cmaes, numpy, scipy and Python, and which normal CDF and chi-squared quantile
cmaes's margin correction used: scipy's when scipy imports, else cmaes's own `cmaes._stats`.

Run from the repository root (the oracle environment has cmaes); it writes the fixture itself:

    refs/venv/bin/python validation/oracles/cmawm/cmawm_runs.py

Output: crates/hpr-analysis/tests/fixtures/cmawm/cmawm.json
"""

import json
import math
import pathlib
import platform

import cmaes
import cmaes._cmawm
import numpy as np
from cmaes import CMAwM

SIGMA0 = 1.0
FTARGET = 1e-10
MIN_EIGENVALUE = 1e-30
MAX_CONDITION = 1e14
EVALUATIONS_PER_DIMENSION = 10_000
SEEDS = range(20)
MEAN_SEED_OFFSET = 1000
MEAN_RANGE = (1.0, 3.0)
INTEGER_RANGE = (-10, 10)
BINARY_MEAN = 0.5

OUTPUT = (
    pathlib.Path(__file__).resolve().parents[3]
    / "crates/hpr-analysis/tests/fixtures/cmawm/cmawm.json"
)


def sphere_int(x, n):
    return float(np.sum(x * x))


def ellipsoid_int(x, n):
    scales = np.array([1000.0 ** (j / (n - 1)) for j in range(n)])
    return float(np.sum((scales * x) ** 2))


def sphere_one_max(x, n):
    half = n // 2
    return float(np.sum(x[:half] ** 2) + (n - half) - np.sum(x[half:]))


# name: (function, kind of the discrete half)
FUNCTIONS = {
    "sphere_int": (sphere_int, "integer"),
    "ellipsoid_int": (ellipsoid_int, "integer"),
    "sphere_one_max": (sphere_one_max, "binary"),
}
CASES = [
    ("sphere_int", 10),
    ("ellipsoid_int", 10),
    ("sphere_one_max", 10),
    ("sphere_int", 20),
    ("ellipsoid_int", 20),
    ("sphere_one_max", 20),
]
VARIANTS = ("active", "positive")


def stats_implementation():
    """Which normal CDF and chi-squared quantile `cmaes._cmawm` imported (its lines 13-20)."""
    if hasattr(cmaes._cmawm, "stats"):
        import scipy.stats

        assert cmaes._cmawm.norm_cdf == scipy.stats.norm.cdf, "scipy imported, CDF not scipy's"
        return "scipy"
    from cmaes import _stats

    assert cmaes._cmawm.norm_cdf is _stats.norm_cdf, "neither scipy's CDF nor cmaes's"
    return "cmaes._stats"


def problem(kind, n):
    """Bounds and steps: the first n/2 continuous and unbounded, the rest `kind`."""
    half = n // 2
    low, high = INTEGER_RANGE if kind == "integer" else (0, 1)
    bounds = np.array([[-math.inf, math.inf]] * half + [[low, high]] * (n - half), dtype=float)
    steps = np.array([0.0] * half + [1.0] * (n - half))
    return bounds, steps


def initial_mean(kind, n, seed):
    mean = np.random.RandomState(MEAN_SEED_OFFSET + seed).uniform(*MEAN_RANGE, n)
    if kind == "binary":
        mean[n // 2 :] = BINARY_MEAN
    return mean


def make(kind, n, seed, variant):
    bounds, steps = problem(kind, n)
    mean = initial_mean(kind, n, seed)
    optimizer = CMAwM(mean=mean.copy(), sigma=SIGMA0, bounds=bounds, steps=steps, seed=seed)
    cma = optimizer._cma
    if variant == "positive":
        cma._weights[cma._mu :] = 0.0
        assert np.all(cma._weights[: cma._mu] > 0) and np.all(cma._weights[cma._mu :] == 0)
        assert abs(np.sum(cma._weights) - 1.0) < 1e-14, np.sum(cma._weights)
    else:
        assert np.all(cma._weights[cma._mu :] < 0), "active weights expected"
    return optimizer, mean


def run(name, n, seed, variant):
    function, kind = FUNCTIONS[name]
    optimizer, mean = make(kind, n, seed, variant)
    cma = optimizer._cma
    cap = n * EVALUATIONS_PER_DIMENSION
    evaluations = 0
    generations = 0
    best = math.inf
    while True:
        solutions = []
        for _ in range(optimizer.population_size):
            encoded, raw = optimizer.ask()
            value = function(encoded, n)
            best = min(best, value)
            solutions.append((raw, value))
        evaluations += optimizer.population_size
        generations += 1
        if best <= FTARGET:
            stop = "target"
            break
        optimizer.tell(solutions)
        c = cma._C
        if not (np.all(np.isfinite(c)) and math.isfinite(cma._sigma)):
            stop = "condition"
            break
        eigenvalues = np.linalg.eigvalsh((c + c.T) / 2)
        if cma._sigma**2 * eigenvalues[0] < MIN_EIGENVALUE:
            stop = "min_eigenvalue"
            break
        if not (eigenvalues[0] > 0 and eigenvalues[-1] / eigenvalues[0] <= MAX_CONDITION):
            stop = "condition"
            break
        if evaluations >= cap:
            stop = "evaluations"
            break
    return {
        "seed": seed,
        "initial_mean": [float(m) for m in mean],
        "success": best <= FTARGET,
        "evaluations": evaluations,
        "best": best,
        "stop": stop,
        "generations": generations,
    }


def summary(runs):
    used = [r["evaluations"] for r in runs if r["success"]]
    if not used:
        stats = dict.fromkeys(("median", "q1", "q3", "min", "max"))
    else:
        q1, median, q3 = (float(q) for q in np.percentile(used, [25, 50, 75]))
        stats = {"median": median, "q1": q1, "q3": q3, "min": min(used), "max": max(used)}
    return {"successes": len(used), "runs": len(runs), **stats}


def parameters(n):
    """The strategy parameters cmaes uses at dimension n, read from its `CMA` object."""
    optimizer, _ = make("integer", n, 0, "active")
    cma = optimizer._cma
    mu_eff = cma._mu_eff
    table_1_cmu = min(
        1 - cma._c1, 2 * (0.25 + mu_eff + 1 / mu_eff - 2) / ((n + 2) ** 2 + 2 * mu_eff / 2)
    )
    return {
        "lambda": int(cma._popsize),
        "mu": int(cma._mu),
        "weights": [float(w) for w in cma._weights],
        "mu_eff": float(mu_eff),
        "c1": float(cma._c1),
        "cmu": float(cma._cmu),
        "cmu_table_1": float(table_1_cmu),
        "c_sigma": float(cma._c_sigma),
        "d_sigma": float(cma._d_sigma),
        "cc": float(cma._cc),
        "chi_n": float(cma._chi_n),
        "margin": float(optimizer.margin),
    }


def main():
    import scipy

    cases = {}
    for name, n in CASES:
        variants = {}
        for variant in VARIANTS:
            runs = [run(name, n, seed, variant) for seed in SEEDS]
            variants[variant] = {"runs": runs, "summary": summary(runs)}
        cases[f"{name}_{n}"] = {
            "function": name,
            "dimension": n,
            "continuous": n // 2,
            "discrete": FUNCTIONS[name][1],
            "variants": variants,
        }
    fixture = {
        "pins": {
            "cmaes": cmaes.__version__,
            "numpy": np.__version__,
            "scipy": scipy.__version__,
            "python": platform.python_version(),
            "stats": stats_implementation(),
        },
        "settings": {
            "sigma0": SIGMA0,
            "ftarget": FTARGET,
            "min_eigenvalue": MIN_EIGENVALUE,
            "max_condition": MAX_CONDITION,
            "evaluations_per_dimension": EVALUATIONS_PER_DIMENSION,
            "seeds": list(SEEDS),
            "mean_seed_offset": MEAN_SEED_OFFSET,
            "mean_range": list(MEAN_RANGE),
            "integer_range": list(INTEGER_RANGE),
            "binary_mean": BINARY_MEAN,
        },
        "parameters": {str(n): parameters(n) for n in sorted({n for _, n in CASES})},
        "cases": cases,
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(fixture, sort_keys=True, indent=1) + "\n")


if __name__ == "__main__":
    main()
