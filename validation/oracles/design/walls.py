"""Reference integrals for nose cones and transitions with a wall measured normal to the surface:
volume, centroid and both moments of inertia, per unit density.

The wall is the part of the solid within t of the outer surface (ADR-006, docs/physics/shapes.md).
Its inner radius at station x is the lower envelope of circles of radius t centred on the profile,
extended past a cut end along the end tangent (and not at all past an end whose tangent is vertical):

    r_i(x) = max(0, min over |s - x| <= t of [ y(s) - sqrt(t^2 - (x - s)^2) ])

hpr-design finds that minimum by a coarse scan and a golden-section search. This script shares no
code with it and finds the minimum differently: every interior minimum is a root of the derivative
y'(s) - (x - s) / sqrt(t^2 - (x - s)^2), bracketed on a grid and solved by bisection,
and the ends of the profile are added as candidates. The hollow is integrated by tanh-sinh at 25
digits, split where r_i reaches zero and at t from each end, and subtracted from the filled solid.
The profiles come from shapes.py in this directory.

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
    fore_r, fore_slope = at(mpf(0))
    aft_r, aft_slope = at(length)

    def height(s):
        """(y, y') on the profile or its tangent extensions; None where there is no surface."""
        if s < 0:
            return None if abs(fore_slope) == inf else (fore_r + fore_slope * s, fore_slope)
        if s > length:
            return None if abs(aft_slope) == inf else (aft_r + aft_slope * (s - length), aft_slope)
        return at(s)

    cache = {}

    def inner(x):
        if x in cache:
            return cache[x]
        candidates = []

        def bound(s):
            h = height(s)
            return None if h is None else h[0] - sqrt(max(t * t - (x - s) ** 2, 0))

        def slope(s):
            h = height(s)
            if h is None or abs(h[1]) == inf:
                return None
            gap = t * t - (x - s) ** 2
            if gap <= 0:
                return -inf if s < x else inf
            return h[1] - (x - s) / sqrt(gap)

        # The window's own edges have infinite slopes (-inf on the left, +inf on the right), so a
        # minimum can lie between an edge and the first grid point; the profile's ends, where it
        # stops, bound the window too.
        lo = x - t
        hi = x + t
        lo_slope, hi_slope = -inf, inf
        if lo < 0 and height(lo) is None:
            lo, lo_slope = mpf(0), slope(mpf(0))
        if hi > length and height(hi) is None:
            hi, hi_slope = length, slope(length)
        n = 48
        points = [lo] + [lo + (hi - lo) * (k + mpf(1) / 2) / n for k in range(n)] + [hi]
        values = [lo_slope] + [slope(s) for s in points[1:-1]] + [hi_slope]
        for k in range(len(points) - 1):
            a, b = values[k], values[k + 1]
            if a is None or b is None:
                continue
            if a <= 0 <= b:
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
                candidates.append(bound((left + right) / 2))
        for s in points:
            value = bound(s)
            if value is not None:
                candidates.append(value)
        for end in (mpf(0), length):
            if abs(x - end) <= t:
                candidates.append(bound(end))
        value = max(min(c for c in candidates if c is not None), mpf(0))
        cache[x] = value
        return value

    # Where the hollow starts or ends, found by bisection on a grid.
    grid = [length * k / 128 for k in range(129)]
    breaks = {mpf(0), length, t, length - t}
    hollow = [inner(x) > 0 for x in grid]
    for k in range(128):
        if hollow[k] != hollow[k + 1]:
            lo, hi = grid[k], grid[k + 1]
            for _ in range(80):
                mid = (lo + hi) / 2
                if (inner(mid) > 0) == hollow[k]:
                    lo = mid
                else:
                    hi = mid
            breaks.add((lo + hi) / 2)
    points = sorted(b for b in breaks if 0 <= b <= length)

    def integral(f):
        # The default degree stops early near a blunt end's envelope; degree 10 converges.
        return quad(lambda x: f(x, inner(x)), points, maxdegree=10)

    filled = shapes.integrals(case)
    v_in = pi * integral(lambda x, r: r**2)
    m_in = pi * integral(lambda x, r: x * r**2)
    a_in = pi / 2 * integral(lambda x, r: r**4)
    f_in = pi * integral(lambda x, r: r**4 / 4 + x**2 * r**2)
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
