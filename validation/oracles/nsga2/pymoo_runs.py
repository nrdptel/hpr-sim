"""Reference runs of pymoo's NSGA-II on ZDT1, ZDT2 and ZDT3, set up as Deb et al. (2002) ran it.

Checks: `hpr_analysis::optimize::nsga2`, hpr's own NSGA-II. This script shares no code with it.

pymoo (Apache-2.0) is J. Blank and K. Deb's multi-objective optimization library. It is run here
as an outside oracle, with the settings of K. Deb, A. Pratap, S. Agarwal and T. Meyarivan, "A fast
and elitist multiobjective genetic algorithm: NSGA-II", IEEE TEC 6(2), 182-197 (2002), p. 187:

- a population of 100, run for 250 generations (25,000 evaluations), the first one random;
- SBX crossover with probability 0.9 per pair, each variable crossed with probability 0.5,
  distribution index 20 (`SBX(prob=0.9, prob_var=0.5, eta=20)`);
- polynomial mutation of each variable with probability 1/n, distribution index 20
  (`PM(prob=1.0, eta=20)`: pymoo's per-variable default is min(0.5, 1/n) = 1/30 here);
- binary tournaments by the paper's crowded comparison, rank then crowding distance
  (`tournament_type = "comp_by_rank_and_crowding"`; pymoo's default compares by dominance);
- no removal of duplicate designs (`eliminate_duplicates=False`), as the paper has none.

Each problem has 30 variables in [0, 1] (pymoo's `get_problem("zdt1")` and so on), and each run
uses one of seeds 1 to 20. For each run it records the final population's first front (`res.F`),
every value written as Python's shortest round-trip repr, and pymoo's own generational distance
(`pymoo.indicators.gd.GD`, the mean distance to the nearest of the reference points, p = 1) of that
front from `problem.pareto_front(500)`, Deb's H = 500 points, which it records too. Its IGD is not
used: it goes through moocore (LGPL), so hpr computes IGD itself.

Run from the repository root (the oracle environment has pymoo); it writes the fixture itself:

    refs/venv/bin/python validation/oracles/nsga2/pymoo_runs.py

Output: crates/hpr-analysis/tests/fixtures/nsga2/pymoo.json
"""

import json
import pathlib

import pymoo
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.indicators.gd import GD
from pymoo.operators.crossover.sbx import SBX
from pymoo.operators.mutation.pm import PM
from pymoo.optimize import minimize
from pymoo.problems import get_problem

PROBLEMS = ["zdt1", "zdt2", "zdt3"]
POPULATION = 100
GENERATIONS = 250
REFERENCE_POINTS = 500
SEEDS = range(1, 21)

OUTPUT = (
    pathlib.Path(__file__).resolve().parents[3]
    / "crates/hpr-analysis/tests/fixtures/nsga2/pymoo.json"
)


def run(problem, seed):
    algorithm = NSGA2(
        pop_size=POPULATION,
        crossover=SBX(prob=0.9, prob_var=0.5, eta=20),
        mutation=PM(prob=1.0, eta=20),
        eliminate_duplicates=False,
    )
    algorithm.tournament_type = "comp_by_rank_and_crowding"
    result = minimize(problem, algorithm, ("n_gen", GENERATIONS), seed=seed, verbose=False)
    return result.F


def main():
    fixture = {
        "pymoo": pymoo.__version__,
        "population": POPULATION,
        "generations": GENERATIONS,
        "seeds": list(SEEDS),
        "problems": {},
    }
    for name in PROBLEMS:
        problem = get_problem(name)
        assert problem.n_var == 30
        reference = problem.pareto_front(REFERENCE_POINTS)
        gd = GD(reference)
        runs = []
        for seed in SEEDS:
            front = run(problem, seed)
            runs.append(
                {
                    "seed": seed,
                    "front": [[float(v) for v in f] for f in front],
                    "gd": float(gd(front)),
                }
            )
            print(f"{name} seed {seed}: {len(front)} on the front, GD {runs[-1]['gd']:.3e}")
        fixture["problems"][name] = {
            "reference": [[float(v) for v in f] for f in reference],
            "runs": runs,
        }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(fixture, indent=1) + "\n")


if __name__ == "__main__":
    main()
