"""RocketPy 1.13.0's SolidMotor mass, centre of mass and inertia over time, as an oracle for
hpr-motor (M1.3; model summary in docs/research/rocketpy-solid-motor.md).

What it pins, per motor case:

- Scalars: total impulse (trapezoid over the clipped thrust knots), burn start and burn-out time,
  average thrust (impulse / burn duration), the constant exhaust velocity (impulse / initial
  propellant mass), and the initial propellant mass. In SolidMotor that mass comes from the BATES
  grain geometry (number x density x volume), not from the .eng header.
- Time series on a fixed grid (200 points from t = 0 to burn-out, then 3 points after): thrust,
  mass flow rate (dm/dt, negative), propellant and total mass, the centres of mass, grain inner
  radius and height, and the propellant and whole-motor inertias.

Frames and reference points (rocketpy/motors/motor.py and solid_motor.py):

- Positions are metres along the motor axis in the motor's own coordinate system: the origin is
  wherever the case puts it, and the axis points from the nozzle to the combustion chamber, or the
  reverse, as `coordinate_system_orientation` says. RocketPy never applies that sign inside the
  motor; `Rocket.add_motor` does.
- `propellant_I_11` and `propellant_I_33` are about the propellant centre of mass. `I_11` and `I_33`
  are about the instantaneous motor centre of mass. `dry_inertia` is given about the dry centre of
  mass. `I_22` equals `I_11` by construction; the script checks that instead of storing it.

Every Function is read with `get_value_opt`, as `Flight` does (rocketpy/simulation/flight.py,
`u_dot` and `u_dot_generalized`). For these array-backed Functions `__call__` gives the same
numbers.

Each case reads one of hpr-motor's bundled public-domain curves with RocketPy's own .eng reader;
the output records the file's path and SHA-256. An .eng file must not contain a `0 0` point:
RocketPy prepends one, and the duplicate knot makes the total impulse NaN. No bundled curve does.

Run from the repository root with the oracle environment (optionally naming cases to keep):

    refs/venv/bin/python validation/oracles/rocketpy/solid_motor.py \
        > validation/fixtures/motor/rocketpy-solid-motor.json
"""

import hashlib
import importlib.metadata
import json
import math
import re
import sys
import warnings
from pathlib import Path

import numpy as np
from rocketpy import SolidMotor

warnings.filterwarnings("ignore")


# SolidMotor keyword arguments that every case sets; the rest keep RocketPy's defaults.
MOTOR_KEYS = [
    "dry_mass", "dry_inertia", "center_of_dry_mass_position", "nozzle_position", "nozzle_radius",
    "throat_radius", "grain_number", "grain_density", "grain_outer_radius",
    "grain_initial_inner_radius", "grain_initial_height", "grain_separation",
    "grains_center_of_mass_position", "coordinate_system_orientation", "burn_time",
    "interpolation_method", "only_radial_burn",
]

CURVES = Path("crates/hpr-motor/data/thrustcurve")

# Three bundled public-domain curves (crates/hpr-motor/data/thrustcurve/catalog.json), each with a
# hypothetical but plausible BATES load sized so the grain mass is close to ThrustCurve's
# propellant mass. Dry inertias are thin tubes of the motor's radius and length. Every bundled curve
# starts after t = 0, so RocketPy's prepended (0, 0) knot is not duplicated. Together the cases
# cover grains that burn out radially and axially, inhibited ends, and both axis orientations.
CASES = [
    {
        # Cesaroni 411I175-14A (Pro38-3G): three grains burning on both ends; the web (9.15 mm) is
        # less than half the grain height, so they burn out radially.
        "name": "cti-411i175-38mm-radial-burnout",
        "thrust_file": "curves/5f4294d20002e9000000075a.eng",
        "dry_mass": 0.2086,
        "dry_inertia": [0.0010811, 0.0010811, 7.53e-5],
        "center_of_dry_mass_position": 0.11,
        "nozzle_position": 0.0,
        "nozzle_radius": 0.0095,
        "throat_radius": 0.0045,
        "grain_number": 3,
        "grain_density": 1800.0,
        "grain_outer_radius": 0.0155,
        "grain_initial_inner_radius": 0.00635,
        "grain_initial_height": 0.0675,
        "grain_separation": 0.003,
        "grains_center_of_mass_position": 0.13,
        "coordinate_system_orientation": "nozzle_to_combustion_chamber",
        "burn_time": None,
        "interpolation_method": "linear",
        "only_radial_burn": False,
    },
    {
        # Cesaroni 1633K940-18A (54 mm): eight short grains, so the height reaches zero (web 19.5 mm
        # against a half height of 15.85 mm) and they burn out axially. The axis runs from the
        # forward closure (origin) to the nozzle exit at 0.404 m.
        "name": "cti-1633k940-54mm-axial-burnout-chamber-to-nozzle",
        "thrust_file": "curves/5f4294d20002e90000000749.eng",
        "dry_mass": 0.5985,
        "dry_inertia": [0.0083584, 0.0083584, 4.363e-4],
        "center_of_dry_mass_position": 0.25,
        "nozzle_position": 0.404,
        "nozzle_radius": 0.0135,
        "throat_radius": 0.0065,
        "grain_number": 8,
        "grain_density": 1800.0,
        "grain_outer_radius": 0.0235,
        "grain_initial_inner_radius": 0.004,
        "grain_initial_height": 0.0317,
        "grain_separation": 0.002,
        "grains_center_of_mass_position": 0.19,
        "coordinate_system_orientation": "combustion_chamber_to_nozzle",
        "burn_time": None,
        "interpolation_method": "linear",
        "only_radial_burn": False,
    },
    {
        # Loki M1378LR (54/4000): four long grains with inhibited ends, burning only on their bores.
        "name": "loki-m1378lr-54mm-inhibited-ends",
        "thrust_file": "curves/5f4294d20002e90000000867.eng",
        "dry_mass": 1.731,
        "dry_inertia": [0.17772, 0.17772, 1.2619e-3],
        "center_of_dry_mass_position": 0.50,
        "nozzle_position": 0.0,
        "nozzle_radius": 0.015,
        "throat_radius": 0.0075,
        "grain_number": 4,
        "grain_density": 1750.0,
        "grain_outer_radius": 0.0235,
        "grain_initial_inner_radius": 0.0095,
        "grain_initial_height": 0.2559,
        "grain_separation": 0.004,
        "grains_center_of_mass_position": 0.56,
        "coordinate_system_orientation": "nozzle_to_combustion_chamber",
        "burn_time": None,
        "interpolation_method": "linear",
        "only_radial_burn": True,
    },
]

GRID_POINTS = 200
AFTER_BURNOUT_S = [0.05, 0.5, 2.0]

SERIES = [
    "thrust", "mass_flow_rate", "propellant_mass", "total_mass", "center_of_propellant_mass",
    "center_of_mass", "grain_inner_radius", "grain_height", "propellant_I_11", "propellant_I_33",
    "I_11", "I_33",
]


def finite(value, label):
    value = float(value)
    if not math.isfinite(value):
        sys.exit(f"solid_motor.py: {label} is not finite ({value})")
    return value


def build(case):
    kwargs = {key: case[key] for key in MOTOR_KEYS}
    kwargs["dry_inertia"] = tuple(case["dry_inertia"])
    return SolidMotor(thrust_source=str(CURVES / case["thrust_file"]), **kwargs)


def run(case):
    motor = build(case)
    name = case["name"]
    t_out = float(motor.burn_out_time)
    times = [float(t) for t in np.linspace(0.0, t_out, GRID_POINTS)]
    times += [t_out + dt for dt in AFTER_BURNOUT_S]

    series = {"time_s": times}
    for key in SERIES:
        f = getattr(motor, key)
        series[key] = [finite(f.get_value_opt(t), f"{name} {key}({t})") for t in times]

    i_22 = [float(motor.I_22.get_value_opt(t)) for t in times]
    if i_22 != series["I_11"]:
        sys.exit(f"solid_motor.py: {name} I_22 differs from I_11")

    # hpr's default model on the same grid: m0 (1 - I(t) / I_total), from RocketPy's own thrust.
    m0 = float(motor.propellant_initial_mass)
    impulse = float(motor.total_impulse)
    t0 = float(motor.burn_start_time)
    fraction_gap = max(
        abs(m - m0 * (1.0 - motor.thrust.integral(t0, min(max(t, t0), t_out)) / impulse))
        for t, m in zip(times, series["propellant_mass"])
    )

    inputs = {key: case[key] for key in MOTOR_KEYS}
    inputs["thrust_file"] = case["thrust_file"]
    inputs["thrust_file_sha256"] = hashlib.sha256((CURVES / case["thrust_file"]).read_bytes()).hexdigest()

    scalars = {
        "total_impulse_ns": impulse,
        "burn_start_time_s": t0,
        "burn_out_time_s": t_out,
        "burn_duration_s": float(motor.burn_duration),
        "average_thrust_n": float(motor.average_thrust),
        "max_thrust_n": float(motor.max_thrust),
        "exhaust_velocity_mps": float(motor.exhaust_velocity.get_value_opt(t_out / 2)),
        "propellant_initial_mass_kg": m0,
        "grain_initial_mass_kg": float(motor.grain_initial_mass),
        "nozzle_area_m2": float(motor.nozzle_area),
        "throat_area_m2": float(motor.throat_area),
        "geometry_ode_end_time_s": float(motor.grain_burn_out),
        "geometry_ode_knots": len(motor.grain_inner_radius.x_array),
        "max_abs_propellant_mass_minus_impulse_fraction_kg": float(fraction_gap),
    }
    for key, value in scalars.items():
        finite(value, f"{name} {key}")

    return {"name": name, "inputs": inputs, "scalars": scalars, "series": series}


wanted = set(sys.argv[1:])
unknown = wanted - {case["name"] for case in CASES}
if unknown:
    sys.exit(f"solid_motor.py: unknown cases {sorted(unknown)}")

document = json.dumps({
    "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
    "generator": "validation/oracles/rocketpy/solid_motor.py",
    "evaluation": "Function.get_value_opt",
    "cases": [run(case) for case in CASES if not wanted or case["name"] in wanted],
}, indent=2)
# One line per series: `repr` floats round-trip exactly, so this only removes whitespace.
print(re.sub(r"\[\s*([-0-9.eE+,\s]+?)\s*\]", lambda m: "[" + re.sub(r"\s+", " ", m.group(1)) + "]", document))
