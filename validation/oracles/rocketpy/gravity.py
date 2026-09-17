"""RocketPy 1.13.0's gravity model at a few latitude and height points, as an oracle for hpr-core.

RocketPy evaluates Somigliana's formula times the Taylor series in height (NGA.STND.0036 eqs. 4-1
and 4-3; rocketpy/environment/environment.py, `somigliana_gravity`). A flight does not call that
formula directly: `Environment.set_gravity_model` samples it at 100 points between 0 and
`max_expected_height` (80 km by default) and holds the last value above that. It is also fed height
above sea level, not above the ellipsoid. Both values are recorded so hpr's like-for-like cases
(M2.1) can reproduce either.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/rocketpy/gravity.py \
        > validation/fixtures/earth/rocketpy-gravity.json
"""

import importlib.metadata
import json
import warnings

from rocketpy import Environment

warnings.filterwarnings("ignore")  # RocketPy warns about UTM zones beyond 84 degrees of latitude.

# The undecorated method: `set_gravity_model` replaces the cached property with a sampled Function.
formula = Environment.__dict__["somigliana_gravity"].func

POINTS = [(0.0, 0.0), (45.0, 0.0), (90.0, 0.0), (32.99, 1400.0), (-33.9, 300.0),
          (60.0, 10000.0), (28.5, 30000.0), (45.0, 100000.0)]

cases = []
for latitude, height in POINTS:
    env = Environment(latitude=latitude)
    cases.append({
        "latitude_deg": latitude,
        "height_m": height,
        "formula_mps2": float(formula(env, height)),
        "flight_mps2": float(env.gravity.get_value_opt(height)),
    })

print(json.dumps({
    "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
    "generator": "validation/oracles/rocketpy/gravity.py",
    "cases": cases,
}, indent=2))
