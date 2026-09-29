"""RocketPy's Calisto, flown from Python by `examples/calisto.py`: within M2.1's 3% of RocketPy on
every metric the validation suite scores as a share of RocketPy's value (its two trajectory rows
have absolute bounds, and are the suite's alone), measured as the suite measures them, and
printing what the guide's Python page shows."""

import contextlib
import io
import json
import re
import runpy

CASE = "flight-calisto-tests-motor-at-minus-1.373"


def run_example(repo, monkeypatch):
    monkeypatch.chdir(repo)
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        namespace = runpy.run_path(str(repo / "crates/hpr-py/examples/calisto.py"))
    return namespace, out.getvalue()


def test_calisto_flies_within_three_percent_of_rocketpy(repo, monkeypatch):
    namespace, _ = run_example(repo, monkeypatch)
    report = json.loads((repo / "validation/reports/latest.json").read_text(encoding="utf-8"))
    scored = {
        row["metric"]: row
        for row in report["comparisons"]
        if row["case"] == CASE and "relative" in row["tolerance"]
    }
    metrics = namespace["metrics"]
    # Every metric the suite holds to a share of RocketPy's value, and nothing else.
    assert set(metrics) == set(scored)
    for name, row in scored.items():
        assert row["tolerance"]["relative"] == 0.03
        difference = abs(metrics[name] - row["reference"]) / abs(row["reference"])
        assert difference <= 0.03, name
        # The same flight as the suite's, measured the same way: its committed hpr numbers. The
        # example reads the recording at the integrator's steps, the suite inside them too; on
        # 2026-09-29 the largest gap was the top speed's, 3.2e-6, and a wrong gravity model or a
        # main opening at 800 m rather than 800 m + h0 moved the landing drift by 1.8e-4 or 5.9e-4.
        assert abs(metrics[name] - row["measured"]) <= 1e-5 * abs(row["measured"]), name
    assert namespace["worst"] <= 0.03


def test_the_python_page_shows_what_the_example_prints(repo, monkeypatch):
    _, printed = run_example(repo, monkeypatch)
    page = (repo / "docs/python.md").read_text(encoding="utf-8")
    (shown,) = re.findall(r"<!-- calisto\.py prints -->\n```text\n(.*?)```", page, re.DOTALL)
    assert printed == shown
