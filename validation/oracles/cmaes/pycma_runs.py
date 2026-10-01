"""Reference runs of pycma, the reference CMA-ES, on four benchmark functions in 10 dimensions.

Checks: `hpr_analysis::optimize::cmaes`, hpr's own CMA-ES. This script shares no code with it.

pycma (the `cma` package, BSD-3-Clause) is Nikolaus Hansen's implementation of the covariance
matrix adaptation evolution strategy. It is run here as an outside oracle, with its default
population (lambda = 4 + floor(3 ln n) = 10) and non-active weights (`CMA_active: False`), on:

- sphere:            f(x) = sum x_i^2, from x = (1, ..., 1);
- ellipsoid:         f(x) = sum 10^(6 i / (n - 1)) x_i^2, from x = (1, ..., 1);
- rotated_ellipsoid: the ellipsoid of H x, where H = I - 2 v v^T / (v^T v) is the Householder
                     reflection with v_i = i + 1, from x = (1, ..., 1);
- rosenbrock:        f(x) = sum_{i=0}^{n-2} [100 (x_i^2 - x_{i+1})^2 + (1 - x_i)^2], from x = 0.

Indices are 0-based. Every run starts with sigma0 = 0.5 and uses one of seeds 1 to 20. Every
termination criterion pycma lets a caller turn off is off, so a run stops only when its best value
reaches the target f <= 1e-10 or after 100,000 evaluations. The ask-tell loop below tests those
two itself rather than `es.stop()`: pycma's `maxfevals` test stops one iteration late (it tests
`countevals - 1 >= maxfevals`), and its numerical `noeffectcoord`/`noeffectaxis` checks cannot be
turned off. The options still carry the target and cap, as a caller of `cma.fmin` would set them.

For each run it records the evaluations used, the best value, whether the target was reached, and
the best point's distance to the known optimum (the origin, or all ones for rosenbrock). It also
records pycma's strategy parameters for these settings, read from where pycma keeps the values its
update uses:

- weights (positive recombination weights): `es.sp.weights.positive_weights`
- mueff:   `es.sp.weights.mueff`
- c1, cmu: `es.sm.parameters(mueff=..., lam=...)` (what `tell` uses; `es.sp.c1`/`es.sp.cmu` agree)
- cc:      `es.sp.cc`
- cs, damps (c_sigma, d_sigma): `es.adapt_sigma.cs`, `es.adapt_sigma.damps`
- chi_n:   `cma.utilities.math.Mh.chiN(n)`, the E||N(0, I)|| the step-size update divides by

Run from the repository root (the oracle environment has cma); it writes the fixture itself:

    refs/venv/bin/python validation/oracles/cmaes/pycma_runs.py

Output: crates/hpr-analysis/tests/fixtures/cmaes/pycma.json
"""

import json
import math
import pathlib

import cma
import numpy as np
from cma.utilities.math import Mh

DIMENSION = 10
SIGMA0 = 0.5
FTARGET = 1e-10
MAX_EVALUATIONS = 100_000
SEEDS = range(1, 21)

OUTPUT = (
    pathlib.Path(__file__).resolve().parents[3]
    / "crates/hpr-analysis/tests/fixtures/cmaes/pycma.json"
)

ELLIPSOID_SCALES = np.array([10.0 ** (6.0 * i / (DIMENSION - 1)) for i in range(DIMENSION)])
_V = np.arange(1, DIMENSION + 1, dtype=float)
HOUSEHOLDER = np.eye(DIMENSION) - 2.0 * np.outer(_V, _V) / (_V @ _V)


def sphere(x):
    return float(np.sum(x * x))


def ellipsoid(x):
    return float(np.sum(ELLIPSOID_SCALES * x * x))


def rotated_ellipsoid(x):
    return ellipsoid(HOUSEHOLDER @ x)


def rosenbrock(x):
    return float(np.sum(100.0 * (x[:-1] ** 2 - x[1:]) ** 2 + (1.0 - x[:-1]) ** 2))


# name: (function, start mean, optimum)
FUNCTIONS = {
    "sphere": (sphere, np.ones(DIMENSION), np.zeros(DIMENSION)),
    "ellipsoid": (ellipsoid, np.ones(DIMENSION), np.zeros(DIMENSION)),
    "rotated_ellipsoid": (rotated_ellipsoid, np.ones(DIMENSION), np.zeros(DIMENSION)),
    "rosenbrock": (rosenbrock, np.zeros(DIMENSION), np.ones(DIMENSION)),
}


def options(seed):
    """Default CMA-ES, non-active, with every switchable stopping rule but the two kept off."""
    return {
        "seed": seed,
        "CMA_active": False,
        "ftarget": FTARGET,
        "maxfevals": MAX_EVALUATIONS,
        "verbose": -9,
        "verb_log": 0,
        "verb_disp": 0,
        "signals_filename": "",
        # Termination criteria, off.
        "maxiter": math.inf,
        "timeout": math.inf,
        "tolfun": 0,
        "tolfunhist": 0,
        "tolfunrel": 0,
        "tolx": 0,
        "tolstagnation": 0,
        "tolxstagnation": False,
        "tolflatfitness": math.inf,
        "tolfacupx": math.inf,
        "tolupsigma": math.inf,
        "tolconditioncov": math.inf,
        # Not a stopping rule: pycma would reset C into a coordinate transformation once its
        # condition passes 1e8 / 1e12. The runs here stay far below; off so it cannot act.
        "conditioncov_alleviate": False,
    }


def run(function, x0, optimum, seed):
    es = cma.CMAEvolutionStrategy(x0.tolist(), SIGMA0, options(seed))
    while es.countevals < MAX_EVALUATIONS and es.best.f > FTARGET:
        candidates = es.ask()
        es.tell(candidates, [function(np.asarray(x)) for x in candidates])
    best = float(es.best.f)
    return {
        "seed": seed,
        "evaluations": int(es.countevals),
        "best": best,
        "reached": best <= FTARGET,
        "distance": float(np.linalg.norm(np.asarray(es.best.x) - optimum)),
    }


def parameters():
    es = cma.CMAEvolutionStrategy(np.ones(DIMENSION).tolist(), SIGMA0, options(1))
    weights = es.sp.weights
    used = es.sm.parameters(mueff=weights.mueff, lam=weights.lambda_)
    assert es.sp.popsize == 10 and weights.mu == 5, (es.sp.popsize, weights.mu)
    assert es.sp.lam_mirr == 0 and not es.opts["CMA_diagonal"], "mirrors or diagonal phase on"
    assert all(w == 0 for w in list(weights)[weights.mu :]), "negative weights not zeroed"
    assert (used["c1"], used["cmu"]) == (es.sp.c1, es.sp.cmu), (used, es.sp.c1, es.sp.cmu)
    es.adapt_sigma.initialize(es)
    return {
        "weights": [float(w) for w in weights.positive_weights],
        "mueff": float(weights.mueff),
        "c1": float(used["c1"]),
        "cmu": float(used["cmu"]),
        "cc": float(es.sp.cc),
        "cs": float(es.adapt_sigma.cs),
        "damps": float(es.adapt_sigma.damps),
        "chi_n": float(Mh.chiN(DIMENSION)),
    }


def main():
    fixture = {
        "pycma_version": cma.__version__,
        "dimension": DIMENSION,
        "sigma0": SIGMA0,
        "ftarget": FTARGET,
        "max_evaluations": MAX_EVALUATIONS,
        "parameters": parameters(),
        "runs": {
            name: [run(function, x0, optimum, seed) for seed in SEEDS]
            for name, (function, x0, optimum) in FUNCTIONS.items()
        },
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(fixture, indent=2) + "\n")


if __name__ == "__main__":
    main()
