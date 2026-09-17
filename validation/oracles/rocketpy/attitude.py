"""RocketPy 1.13.0's initial attitude quaternion for a set of launch angles, as an oracle for
hpr-core's `LaunchAngles` (docs/physics/frames.md).

RocketPy builds the initial attitude from 3-1-3 Euler angles: precession psi = -heading, nutation
theta = inclination - 90 degrees, spin phi = the rail-button angular position
(rocketpy/simulation/flight.py, `Flight.__init_flight_state`, and rocketpy/tools.py
`euler313_to_quaternions`). Its Euler parameters (e0, e1, e2, e3) are scalar-first.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/rocketpy/attitude.py \
        > validation/fixtures/earth/rocketpy-attitude.json
"""

import importlib.metadata
import json
import math

from rocketpy.tools import euler313_to_quaternions

# (heading deg, inclination deg, spin rad): rail setups from vertical to nearly horizontal, every
# quadrant of heading, and a spread of rail-button angles.
CASES = [
    (0.0, 90.0, 0.0),
    (0.0, 85.0, 0.0),
    (90.0, 80.0, 0.0),
    (135.0, 84.0, 0.7853981633974483),
    (200.0, 60.0, 3.0),
    (271.5, 88.5, -1.2),
    (359.0, 45.0, 5.5),
    (17.0, 5.0, 2.2),
]

cases = []
for heading, inclination, spin in CASES:
    e0, e1, e2, e3 = euler313_to_quaternions(spin, math.radians(inclination - 90.0), math.radians(-heading))
    cases.append({
        "heading_deg": heading,
        "inclination_deg": inclination,
        "rail_button_angle_rad": spin,
        "e0": float(e0),
        "e1": float(e1),
        "e2": float(e2),
        "e3": float(e3),
    })

print(json.dumps({
    "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
    "generator": "validation/oracles/rocketpy/attitude.py",
    "cases": cases,
}, indent=2))
