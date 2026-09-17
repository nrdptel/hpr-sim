"""Reference integrals for nose cone and transition profiles: volume, centroid, moments of inertia
and areas of the filled solids of revolution, to 30 significant digits.

Sources for the profiles:
- G. A. Crowell Sr., The Descriptive Geometry of Nose Cones, 1996, pp. 1-6 (cited, not pinned; see
  docs/VALIDATION.md): cone, tangent and secant ogive, elliptical, power series, parabolic series,
  Haack series.
- S. Niskanen, OpenRocket technical documentation v13.05, 2013, appendix A, pp. 102-106
  (`openrocket-techdoc-13.05`): the same curves, and clipped transitions (section A.7).

Every integral is evaluated by mpmath's tanh-sinh quadrature at 40 working digits from the
defining formula, with the Haack series integrated in its angle variable theta (dx = L/2 sin theta
dtheta), where the integrands are smooth. hpr-design integrates in x with an adaptive Gauss-Kronrod
rule; the two share no code, and this script uses no clipping search from the Rust side (it solves
for the clipped nose length with mpmath's root finder).

Run from the repository root (the oracle environment has mpmath):

    refs/venv/bin/python validation/oracles/design/shapes.py \
        > validation/fixtures/design/shape-integrals.json
"""

import hashlib
import json
from pathlib import Path

import mpmath
from mpmath import acos, atan, cos, findroot, inf, mp, mpf, pi, quad, sin, sqrt

mp.dps = 40

# Provenance written into the fixture (docs/VALIDATION.md). Update the date when regenerating.
GENERATED = "2026-09-17"
COMMAND = (
    "refs/venv/bin/python validation/oracles/design/shapes.py "
    "> validation/fixtures/design/shape-integrals.json"
)


def curve(kind, param, fineness):
    """The normalized curve as (g(xi), dg/dxi), tip at xi = 0 and base at xi = 1."""
    if kind == "conical":
        return lambda xi: (xi, mpf(1))
    if kind == "ogive":
        # Crowell p. 4: alpha = atan(R/L) - acos(sqrt(L^2 + R^2) / (2 rho)), in units of R, for an
        # arc through (0, 0) and (L, R) with centre (rho cos alpha, rho sin alpha).
        lam = mpf(fineness)
        rho = mpf(param) * (lam**2 + 1) / 2
        alpha = atan(1 / lam) - acos(sqrt(lam**2 + 1) / (2 * rho))
        xc, yc = rho * cos(alpha), rho * sin(alpha)

        def g(xi):
            root = sqrt(rho**2 - (lam * xi - xc) ** 2)
            return root + yc, -lam * (lam * xi - xc) / root

        return g
    if kind == "elliptical":
        def g(xi):
            value = sqrt(xi * (2 - xi))
            return value, ((1 - xi) / value if value else inf)

        return g
    if kind == "power_series":
        n = mpf(param)
        return lambda xi: (xi**n, n * xi ** (n - 1) if xi else inf)
    if kind == "parabolic_series":
        k = mpf(param)
        return lambda xi: ((2 * xi - k * xi**2) / (2 - k), (2 - 2 * k * xi) / (2 - k))
    if kind == "haack":
        c = mpf(param)

        def core(theta):
            """theta - sin(2 theta)/2, by its Taylor series near 0 where the difference cancels."""
            if theta > mpf("0.1"):
                return theta - sin(2 * theta) / 2
            total, term = mpf(0), theta
            # Terms fall by at least 4 theta^2 / 6 < 1/100 each; 2 dps terms leave 2 dps digits.
            for k in range(1, 2 * mp.dps):
                term *= 4 * theta**2 / ((2 * k) * (2 * k + 1))
                total += term if k % 2 else -term
            return total

        def g(xi):
            theta = 2 * mpmath.asin(sqrt(xi))
            value = sqrt((core(theta) + c * sin(theta) ** 3) / pi)
            # d(g^2)/dtheta = sin^2 theta (2 + 3C cos theta) / pi and dtheta/dxi = 2 / sin theta.
            # At the tip, theta - sin(2 theta)/2 cancels to zero at the working precision, where
            # the slope is infinite.
            if value == 0:
                return value, inf
            return value, sin(theta) * (2 + 3 * c * cos(theta)) / (pi * value)

        return g
    raise ValueError(kind)


def profile(case):
    """(r(x), dr/dx(x)) for a case, physical units, x aft of the forward end."""
    kind, param = case["shape"], case.get("parameter")
    length = mpf(case["length_m"])
    fore, aft = mpf(case["fore_radius_m"]), mpf(case["aft_radius_m"])
    big, small = max(fore, aft), min(fore, aft)
    tip_aft = aft < fore
    sign = -1 if tip_aft else 1
    if case.get("clipped") and small > 0:
        g = curve(kind, param, 1)
        xi0 = findroot(lambda xi: g(xi)[0] - small / big, (mpf("1e-30"), 1), solver="anderson")
        nose = length / (1 - xi0)
        start, span, scale = nose - length, big, nose
        offset = mpf(0)
    else:
        g = curve(kind, param, length / (big - small))
        start, span, scale = mpf(0), big - small, length
        offset = small

    def at(x):
        u = (length - x) if tip_aft else x
        value, slope = g((start + u) / scale)
        return offset + span * value, sign * span * slope / scale

    return at, length


def integrals(case):
    at, length = profile(case)
    if case["shape"] == "haack" and not case.get("clipped"):
        # Substitute u (from the tip) = L sin^2(theta/2) = L/2 (1 - cos theta), with
        # dx = L/2 sin theta dtheta: the integrands are smooth in theta, and the sin^2 form keeps u
        # exact near the tip.
        tip_aft = mpf(case["aft_radius_m"]) < mpf(case["fore_radius_m"])

        def integrate(f):
            def integrand(theta):
                u = length * sin(theta / 2) ** 2
                x = length - u if tip_aft else u
                y, slope = at(x)
                return f(x, y, slope) * length / 2 * sin(theta)

            return quad(integrand, [0, pi / 2, pi])

    else:

        def integrate(f):
            return quad(lambda x: f(x, *at(x)), [0, length / 2, length])

    vol = pi * integrate(lambda x, y, s: y**2)
    first = pi * integrate(lambda x, y, s: x * y**2)
    axial = pi / 2 * integrate(lambda x, y, s: y**4)
    fore_plane = pi * integrate(lambda x, y, s: y**4 / 4 + x**2 * y**2)
    centroid = first / vol
    # At a blunt tip y = 0 and the slope is infinite; the integrand's limit there is finite.
    wetted = 2 * pi * integrate(lambda x, y, s: sqrt(y**2 + (y * s) ** 2) if y else 0)
    planform = 2 * integrate(lambda x, y, s: y)
    planform_first = 2 * integrate(lambda x, y, s: x * y)
    return {
        "volume_m3": vol,
        "centroid_m": centroid,
        "axial_m5": axial,
        "transverse_m5": fore_plane - vol * centroid**2,
        "wetted_area_m2": wetted,
        "planform_area_m2": planform,
        "planform_centroid_m": planform_first / planform,
    }


def nose(shape, parameter, length, radius):
    case = {"shape": shape, "length_m": length, "fore_radius_m": 0.0, "aft_radius_m": radius}
    if parameter is not None:
        case["parameter"] = parameter
    return case


def transition(shape, parameter, length, fore, aft, clipped):
    case = {
        "shape": shape,
        "length_m": length,
        "fore_radius_m": fore,
        "aft_radius_m": aft,
        "clipped": clipped,
    }
    if parameter is not None:
        case["parameter"] = parameter
    return case


CASES = [
    nose("conical", None, 0.3, 0.05),
    nose("ogive", 1.0, 0.25, 0.04),
    nose("ogive", 2.5, 0.4, 0.051),
    nose("ogive", 0.6, 0.2, 0.03),
    nose("elliptical", None, 0.12, 0.05),
    nose("power_series", 0.5, 0.3, 0.05),
    nose("power_series", 0.3, 0.2, 0.0275),
    nose("power_series", 0.75, 0.5, 0.0785),
    nose("parabolic_series", 0.0, 0.3, 0.05),
    nose("parabolic_series", 0.5, 0.3, 0.05),
    nose("parabolic_series", 1.0, 0.35, 0.0635),
    nose("haack", 0.0, 0.5, 0.0785),
    nose("haack", 1.0 / 3.0, 0.3, 0.05),
    nose("haack", 2.0 / 3.0, 0.2, 0.02),
    transition("conical", None, 0.1, 0.03, 0.05, False),
    transition("ogive", 1.0, 0.08, 0.0635, 0.0385, False),
    transition("elliptical", None, 0.06, 0.05, 0.02, False),
    transition("power_series", 0.5, 0.1, 0.03, 0.05, False),
    transition("power_series", 0.5, 0.1, 0.03, 0.05, True),
    transition("parabolic_series", 0.75, 0.12, 0.05, 0.03, True),
    transition("haack", 1.0 / 3.0, 0.15, 0.02, 0.05, False),
    transition("haack", 0.0, 0.15, 0.05, 0.02, True),
]


def main():
    script = Path(__file__).read_bytes()
    out = {
        "source": "G. A. Crowell Sr., The Descriptive Geometry of Nose Cones (1996), pp. 1-6; "
        "S. Niskanen, OpenRocket technical documentation v13.05 (2013), appendix A",
        "generator": "validation/oracles/design/shapes.py (mpmath tanh-sinh, 40 digits)",
        "tool": f"mpmath {mpmath.__version__}",
        "generated": GENERATED,
        "command": COMMAND,
        "inputs_sha256": {"script": hashlib.sha256(script).hexdigest()},
        "units": "SI; centroids are metres aft of the forward end; moments are per unit density",
        "cases": [],
    }
    for case in CASES:
        values = integrals(case)
        out["cases"].append(
            {"input": case, "expected": {k: float(mp.nstr(v, 20)) for k, v in values.items()}}
        )
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
