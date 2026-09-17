"""Reference integrals for nose cones and transitions with a wall measured normal to the surface:
volume, centroid and both moments of inertia, per unit density.

The wall is the part of the solid within t of the outer surface (ADR-006, docs/physics/shapes.md).
The surface is the profile over its own length, ends included, with no extension past a cut end.
Its inner radius at station x is the lower envelope of circles of radius t centred on it:

    r_i(x) = max(0, min over s in [0, L], |s - x| <= t, of [ y(s) - sqrt(t^2 - (x - s)^2) ])

hpr-design finds that minimum by a coarse scan and a golden-section search. This script shares no
code with it and finds the minimum differently: every interior minimum is a root of the derivative
y'(s) - (x - s) / sqrt(t^2 - (x - s)^2), bracketed on a grid (from the window's edges too, where the
derivative is infinite) and solved by bisection, and the profile's two ends are candidates. The
hollow is integrated by tanh-sinh at 25 digits, split wherever the hollow opens or closes or the
nearest surface point moves between the lateral surface and a rim, and subtracted from the filled
solid; every hollow integral's error estimate must be below 1e-18 relative. The profiles come from
shapes.py in this directory.

Run from the repository root (the oracle environment has mpmath):

    refs/venv/bin/python validation/oracles/design/walls.py \\
        > validation/fixtures/design/wall-integrals.json
"""

import hashlib
import importlib.util
import json
from pathlib import Path

import mpmath
from mpmath import inf, mp, mpf, pi, quad, sqrt

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("shapes", HERE / "shapes.py")
shapes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(shapes)

mp.dps = 25

GENERATED = "2026-09-17"
COMMAND = (
    "refs/venv/bin/python validation/oracles/design/walls.py "
    "> validation/fixtures/design/wall-integrals.json"
)


def wall_integrals(case, t):
    at, length = shapes.profile(case)
    t = mpf(t)

    cache = {}

    def inner(x):
        """(r_i(x), source): source is "surface", "fore" or "aft", the point setting the minimum."""
        if x in cache:
            return cache[x]

        def bound(s):
            return at(s)[0] - sqrt(max(t * t - (x - s) ** 2, 0))

        def slope(s):
            y_slope = at(s)[1]
            if abs(y_slope) == inf:
                return None
            gap = t * t - (x - s) ** 2
            if gap <= 0:
                return -inf if s < x else inf
            return y_slope - (x - s) / sqrt(gap)

        # The window of surface points within t, on the profile. At a window edge inside the
        # profile the derivative is -inf (left) or +inf (right); at a profile end it is whatever the
        # profile's slope gives.
        lo, hi = max(x - t, mpf(0)), min(x + t, length)
        lo_slope = -inf if x - t > 0 else slope(lo)
        hi_slope = inf if x + t < length else slope(hi)
        n = 48
        points = [lo] + [lo + (hi - lo) * (k + mpf(1) / 2) / n for k in range(n)] + [hi]
        values = [lo_slope] + [slope(s) for s in points[1:-1]] + [hi_slope]
        candidates = []
        for k in range(len(points) - 1):
            a, b = values[k], values[k + 1]
            if a is None or b is None or not (a <= 0 <= b):
                continue
            # Bisection on the sign: the derivative is steep near the window's edges, which
            # defeats a residual test.
            left, right = points[k], points[k + 1]
            for _ in range(100):
                mid = (left + right) / 2
                value = slope(mid)
                if value is not None and value <= 0:
                    left = mid
                else:
                    right = mid
            candidates.append((bound((left + right) / 2), "surface"))
        for s in points[1:-1]:
            candidates.append((bound(s), "surface"))
        # The profile's ends are surface points too; there is no extension past them.
        for end, name in ((mpf(0), "fore"), (length, "aft")):
            if abs(x - end) <= t:
                candidates.append((bound(end), name))
        value, source = min(candidates, key=lambda c: c[0])
        result = (max(value, mpf(0)), source)
        cache[x] = result
        return result

    def state(x):
        r, source = inner(x)
        return (r > 0, source if r > 0 else None)

    # Kinks: where the hollow opens or closes, and where the nearest surface point moves between
    # the lateral surface and a rim. Found on a grid and located by bisection.
    grid = [length * k / 256 for k in range(257)]
    states = [state(x) for x in grid]
    breaks = {mpf(0), length}
    for k in range(256):
        if states[k] != states[k + 1]:
            lo, hi = grid[k], grid[k + 1]
            for _ in range(90):
                mid = (lo + hi) / 2
                if state(mid) == states[k]:
                    lo = mid
                else:
                    hi = mid
            breaks.add((lo + hi) / 2)
    points = sorted(breaks)

    errors = []

    def integral(f):
        value, error = quad(lambda x: f(x, inner(x)[0]), points, maxdegree=10, error=True)
        errors.append(error / abs(value) if value else error)
        return value

    filled = shapes.integrals(case)
    v_in = pi * integral(lambda x, r: r**2)
    m_in = pi * integral(lambda x, r: x * r**2)
    a_in = pi / 2 * integral(lambda x, r: r**4)
    f_in = pi * integral(lambda x, r: r**4 / 4 + x**2 * r**2)
    worst = max(errors)
    if worst > mpf("1e-18"):
        raise RuntimeError(f"hollow integral error estimate {mpmath.nstr(worst, 3)} for {case}")
    volume = filled["volume_m3"] - v_in
    first = filled["volume_m3"] * filled["centroid_m"] - m_in
    fore_plane = (
        filled["transverse_m5"] + filled["volume_m3"] * filled["centroid_m"] ** 2 - f_in
    )
    centroid = first / volume
    return {
        "volume_m3": volume,
        "centroid_m": centroid,
        "axial_m5": filled["axial_m5"] - a_in,
        "transverse_m5": fore_plane - volume * centroid**2,
    }


CASES = [
    (shapes.nose("conical", None, 0.3, 0.05), 0.002),
    (shapes.nose("ogive", 1.0, 0.25, 0.04), 0.002),
    (shapes.nose("ogive", 2.5, 0.4, 0.051), 0.003),
    (shapes.nose("ogive", 0.6, 0.2, 0.03), 0.0015),
    (shapes.nose("elliptical", None, 0.12, 0.05), 0.002),
    (shapes.nose("power_series", 0.5, 0.3, 0.05), 0.002),
    (shapes.nose("power_series", 0.3, 0.2, 0.0275), 0.0015),
    (shapes.nose("parabolic_series", 0.5, 0.3, 0.05), 0.002),
    (shapes.nose("haack", 0.0, 0.5, 0.0785), 0.003),
    (shapes.nose("haack", 1.0 / 3.0, 0.3, 0.05), 0.002),
    (shapes.nose("power_series", 0.3, 0.2, 0.0275), 0.006),
    (shapes.transition("conical", None, 0.1, 0.0508, 0.0381, False), 0.002),
    (shapes.transition("ogive", 1.0, 0.1, 0.0381, 0.0508, False), 0.002),
    (shapes.transition("elliptical", None, 0.1, 0.0381, 0.0508, False), 0.002),
    (shapes.transition("elliptical", None, 0.1, 0.0508, 0.0381, False), 0.002),
    (shapes.transition("power_series", 0.5, 0.1, 0.0381, 0.0508, False), 0.002),
    (shapes.transition("power_series", 0.5, 0.1, 0.0381, 0.0508, True), 0.002),
    (shapes.transition("parabolic_series", 0.75, 0.1, 0.0508, 0.0381, True), 0.002),
    (shapes.transition("haack", 0.0, 0.1, 0.0381, 0.0508, False), 0.002),
    (shapes.transition("haack", 1.0 / 3.0, 0.1, 0.0508, 0.0381, True), 0.002),
]


def main():
    out = {
        "source": "ADR-006 wall definition (docs/physics/shapes.md); profiles from shapes.py",
        "generator": "validation/oracles/design/walls.py (mpmath tanh-sinh, 25 digits)",
        "tool": f"mpmath {mpmath.__version__}",
        "generated": GENERATED,
        "command": COMMAND,
        "inputs_sha256": {
            "script": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "shapes.py": hashlib.sha256((HERE / "shapes.py").read_bytes()).hexdigest(),
        },
        "units": "SI; centroids are metres aft of the forward end; moments are per unit density",
        "cases": [],
    }
    for case, t in CASES:
        values = wall_integrals(case, t)
        out["cases"].append(
            {
                "input": case,
                "wall_thickness_m": t,
                "expected": {k: float(mp.nstr(v, 18)) for k, v in values.items()},
            }
        )
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
