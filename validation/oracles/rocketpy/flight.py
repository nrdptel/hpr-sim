"""RocketPy 1.13.0's whole flight, pad to landing, for its own example rockets, as the same-drag
oracle for hpr's flight engine (M2.1b1).

Same-drag mode exists to isolate dynamics, environment and motor: both codes fly the *same*
`C_D0(M)`, so a difference in apogee is a difference in the equations of motion and not in the
aerodynamics. That leaves the question of where the shared table comes from, and it is not
RocketPy's own drag exports: those carry their own terms and are never committed (ADR-009,
`THIRD-PARTY-NOTICES.md`), `refs/` is gitignored, and M2.1c's CI job has no checkout of RocketPy to
read them from. So **the table is declared here**, and this script hands it to RocketPy's
`power_off_drag` and `power_on_drag`, exactly as `recovery.py` declares the wind for the examples
whose own winds come from Copernicus reanalysis files.

The declared table is a constant (`DECLARED_CD0`), power-on and power-off alike. That is a
deliberate refusal to invent: a Mach-dependent curve written here would be an uncited drag model
sitting inside the reference the suite is scored against, which is Loft lesson L18 (its wave-drag
curve was invented and never measured) rebuilt in the validation layer. The *level* of the drag
does not matter to what same-drag mode measures, only that both codes fly the identical table; the
examples' own aerodynamics are hpr's to predict in M2.1c's predicted mode. It is written as two
rows so the case exercises a real interpolated table rather than a scalar.

Everything else about each rocket comes from `validation/fixtures/design/rocketpy-rocket-mass.json`
by way of `recovery.py`, which already reads the example's rocket, motor and geometry from it and
substitutes a bundled public-domain thrust curve for the example's (ADR-007). No input is
transcribed twice. This script adds only what a flight from the pad needs: the rail length,
inclination and heading, cited per case, and the site and wind, declared as `recovery.py` declares
them.

What it records per case: the declared drag, the environment, the rail, every flight event, a time
series for the RMS comparison, the metrics M2.1 names (apogee and time to it, maximum velocity,
Mach and acceleration, rail-exit velocity, burnout altitude and velocity), and the same case again
at looser solver tolerances, so the fixture can state how much of each metric is the solver's.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/rocketpy/flight.py \
        > validation/fixtures/flight/rocketpy-whole-flight.json

**`--own-drag`** flies the same cases with each example's *own* drag instead, as RocketPy 1.13.0
flies the example (`OWN_DRAG`): the reference for M2.1c2's predicted mode, in which hpr flies its own
aerodynamics. The curves live in the RocketPy checkout under the gitignored `refs/` and carry their
own terms (ADR-009), so this mode needs `cargo xtask refs fetch rocketpy`, and its output records
each curve's path and SHA-256, never the curve:

    refs/venv/bin/python validation/oracles/rocketpy/flight.py --own-drag \
        > validation/fixtures/flight/rocketpy-whole-flight-own-drag.json
"""

import hashlib

import importlib.metadata
import json
import math
import sys
import warnings
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import recovery
import numpy as np
from rocketpy import Environment, Flight, Rocket

warnings.filterwarnings("ignore")

COMMAND = "refs/venv/bin/python validation/oracles/rocketpy/flight.py"
OWN_DRAG_FLAG = "--own-drag"

# The drag both codes fly, as (Mach, C_D0) rows. Declared here, not read from RocketPy's exports:
# see the module docstring. Constant, so that nothing about the curve's shape is invented.
DECLARED_CD0 = [[0.0, 0.5], [3.0, 0.5]]

# Prometheus 2022's own drag, ported from RocketPy's MIT-licensed test fixtures
# (tests/fixtures/rockets/rocket_fixtures.py:354-380 at v1.13.0, `prometheus_cd_at_ma`, the same
# function as docs/examples/prometheus_2022_flight_sim.ipynb's cell 9): piecewise linear through
# these (Mach, C_D) points and constant outside them, which `np.interp` reproduces.
PROMETHEUS_CD_POINTS = [
    (0.15, 0.422), (0.45, 0.38), (0.77, 0.32), (0.82, 0.3), (0.88, 0.3), (0.94, 0.32),
    (0.99, 0.37), (1.04, 0.44), (1.24, 0.43), (1.33, 0.42), (1.49, 0.39),
]


def prometheus_cd_at_ma(mach):
    machs, cds = zip(*PROMETHEUS_CD_POINTS)
    return float(np.interp(mach, machs, cds))


# The drag each example flies in RocketPy 1.13.0, for `--own-drag`, power-off and power-on. A string
# is a curve under refs/rocketpy/ (Mach, C_D rows, which RocketPy reads itself), a number a constant,
# and a callable RocketPy's own function. "As RocketPy flies it" matters twice: `Rocket.__init__`
# fixes the drag the flight uses (`power_off_drag_7d`, rocketpy/rocket/rocket.py:399-424, read by
# `Flight` at flight.py:2032-2042), so an example that rescales `rocket.power_off_drag` after
# building the rocket only rebinds a public attribute the flight never reads. Juno III's scaling to
# C_D(0.6) = 0.38 and Bella Lui's replacement curve are two such; the flights use what the
# constructor was given, and so does this reference. Checked against RocketPy's source.
OWN_DRAG = {
    "calisto-tests-motor-at-minus-1.373": {
        "power_off": "refs/rocketpy/data/rockets/calisto/powerOffDragCurve.csv",
        "power_on": "refs/rocketpy/data/rockets/calisto/powerOnDragCurve.csv",
        "source": "tests/fixtures/rockets/rocket_fixtures.py:22-23",
    },
    "valetudo": {
        "power_off": "refs/rocketpy/data/rockets/valetudo/Cd_PowerOff_RASAero.csv",
        "power_on": "refs/rocketpy/data/rockets/valetudo/Cd_PowerOn_RASAero.csv",
        "source": "docs/examples/valetudo_flight_sim.ipynb, cell 18 (the `parameters` dict's "
                  "drag factors are for its Monte Carlo and are not applied)",
    },
    "ndrt-2020-nose-to-tail": {
        "power_off": 0.44,
        "power_on": 0.44,
        "source": "docs/examples/ndrt_2020_flight_sim.ipynb, cell 19: "
                  "parameters.get('drag_coefficient')[0], which is 0.44 (cell 6)",
    },
    "prometheus-2022-generic-motor": {
        "power_off": prometheus_cd_at_ma,
        "power_on": lambda mach: prometheus_cd_at_ma(mach) * 1.02,
        "source": "tests/fixtures/rockets/rocket_fixtures.py:354-390 (`prometheus_cd_at_ma`, "
                  "and 1.02 times it power-on), ported in PROMETHEUS_CD_POINTS",
    },
    "juno-iii": {
        "power_off": "refs/rocketpy/data/rockets/juno3/drag_curve.csv",
        "power_on": "refs/rocketpy/data/rockets/juno3/drag_curve.csv",
        "source": "docs/examples/juno3_flight_sim.ipynb, cell 8 (cell 14's rescaling of "
                  "power_off_drag and power_on_drag never reaches the flight)",
    },
    "bella-lui": {
        "power_off": 0.43,
        "power_on": 0.43,
        "source": "docs/examples/bella_lui_flight_sim.ipynb, cell 19 (cell 25's replacement "
                  "Function never reaches the flight)",
    },
}

# RocketPy's `.eng` reader puts a (0, 0) point before the file's first one (RocketPy's
# rocketpy/motors/motor.py:1133, not this file), so thrust ramps linearly from exactly 0 at t = 0.
# On the rail `Flight.udot_rail1` then clamps the acceleration to zero (RocketPy's
# rocketpy/simulation/flight.py:1862-1871), so the derivative at the initial state is the zero
# vector. With no step bound (`max_time_step=inf`, RocketPy's default) the rail phase's bound is
# `max_time`, and this script's 6000 s horizon, ten times RocketPy's default 600 s, lets LSODA's
# first step go straight over the whole burn: the rocket never leaves the rail.
#
# Bounding `max_time_step` is the fix. Measured at rtol 1e-6 with nothing else changed, all five
# cases: RocketPy's own defaults (`max_time` 600, no bound) fly (valetudo 778.812 m AGL); a
# `max_time` of 6000 with no bound leaves every one on the rail; 6000 with a 0.05 s bound flies
# every one again (valetudo 778.881 m, rail exit 0.4557 s).
MAX_TIME_STEP_S = 0.05

# Tighter than RocketPy's defaults (rtol 1e-6, atol 1e-3 on position), so the reference is the
# model's answer and not the solver's.
SOLVER = {"rtol": 1e-8, "atol": 1e-8, "max_time": 6000, "max_time_step": MAX_TIME_STEP_S}

# The same case at RocketPy's looser tolerances, as `recovery.py` does for a descent: how much of
# each metric is the solver's rather than the model's.
LOOSE_SOLVER = {"rtol": 1e-6, "atol": 1e-6, "max_time": 6000, "max_time_step": MAX_TIME_STEP_S}

# The time series the RMS comparison aligns on: this many samples from launch to landing.
SERIES_SAMPLES = 120

# The rail each example flies from, and the site and wind `recovery.py` declares for it. The rail
# is the one input a descent did not need, so it is cited here per case; everything else is read
# from the case of the same name in `recovery.py`.
RAILS = {
    "calisto-tests-motor-at-minus-1.373": {
        "rail_length_m": 5.2,
        "inclination_deg": 85,
        "heading_deg": 0,
        "source": "tests/fixtures/flight/flight_fixtures.py:34-36",
    },
    "valetudo": {
        "rail_length_m": 5.7,
        "inclination_deg": 84.7,
        "heading_deg": 53,
        "source": "docs/examples/valetudo_flight_sim.ipynb, the `parameters` dict "
                  "(rail_length, inclination, heading; the means, not the dispersions)",
    },
    "ndrt-2020-nose-to-tail": {
        "rail_length_m": 3.353,
        "inclination_deg": 90,
        "heading_deg": 181,
        "source": "docs/examples/ndrt_2020_flight_sim.ipynb, the `parameters` dict "
                  "(rail_length, inclination, heading; the means, not the dispersions)",
    },
    "prometheus-2022-generic-motor": {
        "rail_length_m": 5.18,
        "inclination_deg": 80,
        "heading_deg": 75,
        "source": "docs/examples/prometheus_2022_flight_sim.ipynb:404-406",
    },
    "juno-iii": {
        "rail_length_m": 5.2,
        "inclination_deg": 85,
        "heading_deg": 105,
        "source": "docs/examples/juno3_flight_sim.ipynb, the `Flight` call",
    },
    "bella-lui": {
        "rail_length_m": 4.2,
        "inclination_deg": 89,
        "heading_deg": 45,
        "source": "docs/examples/bella_lui_flight_sim.ipynb:75-115, the `parameters` dict "
                  "(rail_length, inclination, heading; the means, not the dispersions)",
    },
}

# Examples flown pad to landing but not in `recovery.py`'s descent cases, declared in its form.
#
# Bella Lui is the sixth rocket because Prometheus 2022 peaks at Mach 1.014, which hpr refuses
# until M1.8, so five examples alone could never give M2.1 its five same-drag passes; it is on
# M2.1's own list of example rockets. Its example takes its weather from an ERA5 reanalysis file
# (Copernicus), so the wind is declared here, as `recovery.py` declares the winds it cannot use.
WHOLE_FLIGHT_ONLY_CASES = [
    {
        "name": "bella-lui",
        "site": "docs/examples/bella_lui_flight_sim.ipynb:139-153 (elevation, latitude, "
                "longitude; the example's wind comes from an ERA5 reanalysis file, and its "
                "gravity=9.81 is replaced by RocketPy's Somigliana gravity, as for every case); "
                "the wind is declared by this script",
        "latitude": 47.213476,
        "longitude": 9.003336,
        "elevation": 407,
        "wind_u": 3,
        "wind_v": -2,
    },
]


# Issue #50 (M2.1d2): three of the windy cases flown again in calm air, each on its base case's
# rocket, rail and site with both wind components zero, so that the wind's effect on each code's
# path can be measured against a flight without it.
CALM_AIR_BASES = ["juno-iii", "calisto-tests-motor-at-minus-1.373", "bella-lui"]


def calm_air_cases(cases):
    """The calm-air copy of each case named in `CALM_AIR_BASES`, named `<base>-calm`."""
    by_name = {case["name"]: case for case in cases}
    return [
        {
            **by_name[base],
            "name": f"{base}-calm",
            "base": base,
            "site": f"{by_name[base]['site']}; flown in calm air (wind_u = wind_v = 0, issue #50)",
            "wind_u": 0,
            "wind_v": 0,
        }
        for base in CALM_AIR_BASES
    ]


def fail(message):
    sys.exit(f"flight.py: {message}")


def finite(value, label):
    value = float(value)
    if not math.isfinite(value):
        fail(f"{label} is not finite ({value})")
    return value


def drag_of(name, own_drag):
    """The drag a case flies: the declared table, or with `--own-drag` the example's own."""
    if not own_drag:
        return [list(row) for row in DECLARED_CD0], [list(row) for row in DECLARED_CD0]
    own = OWN_DRAG.get(name) or fail(f"{name} has no own drag declared")
    for key in ("power_off", "power_on"):
        if isinstance(own[key], str) and not Path(own[key]).is_file():
            fail(f"{own[key]} is missing; run `cargo xtask refs fetch rocketpy`")
    return own["power_off"], own["power_on"]


def own_drag_record(name):
    """What the reference records of an example's own drag: where it came from, never the curve."""
    own = OWN_DRAG[name]

    def one(value):
        if isinstance(value, str):
            return {
                "kind": "file",
                "file": value,
                "sha256": hashlib.sha256(Path(value).read_bytes()).hexdigest(),
            }
        if callable(value):
            return {"kind": "function", "at_mach_0_3": finite(value(0.3), "drag at Mach 0.3")}
        return {"kind": "constant", "value": finite(value, "constant drag")}

    return {"power_off": one(own["power_off"]), "power_on": one(own["power_on"])}


def build_rocket(inputs, own_drag):
    """The example rocket with the declared drag, or its own, built as `recovery.py` builds it."""
    motor, thrust_path = recovery.build_motor(inputs["motor"])
    rocket_inputs = inputs["rocket"]
    power_off_drag, power_on_drag = drag_of(inputs["name"], own_drag)
    rocket = Rocket(
        radius=rocket_inputs["radius"],
        mass=rocket_inputs["mass"],
        inertia=tuple(rocket_inputs["inertia"]),
        power_off_drag=power_off_drag,
        power_on_drag=power_on_drag,
        center_of_mass_without_motor=rocket_inputs["center_of_mass_without_motor"],
        coordinate_system_orientation=rocket_inputs["coordinate_system_orientation"],
    )
    rocket.add_motor(motor, position=rocket_inputs["motor_position"])
    recovery.add_geometry(rocket, inputs["geometry"])
    if not rocket.parachutes:
        fail(f"{inputs['name']} has no parachutes, so it would not come down under one")
    zero_noise(rocket)
    return rocket, thrust_path


def devices_of(inputs, rocket):
    """The parachutes the flight carried, in the order they open, as the example declared them.

    A reference names everything the comparison flies (L75), so the devices are recorded here,
    not left for the harness to find in the mass fixture. RocketPy checks a trigger at the
    parachute's sampling rate, so its charge fires up to one sample after the condition is met.
    """
    by_name = {parachute.name: parachute for parachute in rocket.parachutes}
    return [
        {
            "name": chute["name"],
            "cd_s_m2": finite(by_name[chute["name"]].cd_s, "cd_s"),
            "trigger": chute["trigger"],
            "lag_s": finite(by_name[chute["name"]].lag, "lag"),
            "sampling_rate_hz": finite(by_name[chute["name"]].sampling_rate, "sampling rate"),
        }
        for chute in recovery.devices_of(inputs)
    ]


def zero_noise(rocket):
    """Zeroes every parachute's noise, which draws from the global `np.random`.

    Unlike `recovery.py`, the lags are the example's own: a whole flight has no reason to start
    where both models already agree, so every device fires as its example says it does.
    """
    for parachute in rocket.parachutes:
        parachute.noise = (0, 0, 0)
        parachute.noise_signal = [[-1e-6, 0]]
        parachute.noise_function = lambda: 0.0
        parachute.noise_bias, parachute.noise_deviation, parachute.noise_corr = 0, 0, (0, 1)


def peak_thrust_to_weight(rocket, env, case):
    """Peak thrust over the weight on the pad, sampled on a grid over the burn.

    Context, not a liftoff criterion: what decides whether RocketPy's rocket leaves the rail is
    the *instantaneous* thrust at t = 0, which is zero for every bundled curve (see
    `MAX_TIME_STEP_S`). The thrust here is the substitute curve's (ADR-007), not the example
    motor's.
    """
    peak = max(
        rocket.motor.thrust.get_value_opt(t)
        for t in [i * rocket.motor.burn_out_time / 200 for i in range(201)]
    )
    weight = rocket.total_mass.get_value_opt(0.0) * env.gravity.get_value_opt(case["elevation"])
    return finite(peak / weight, "thrust to weight")


def fly(rocket, env, rail, solver):
    return Flight(
        rocket=rocket,
        environment=env,
        rail_length=rail["rail_length_m"],
        inclination=rail["inclination_deg"],
        heading=rail["heading_deg"],
        **solver,
    )


def metrics_of(flight, case, name, solver):
    """The metrics M2.1 names, from a finished flight."""
    elevation = case["elevation"]
    if flight.t_final >= solver["max_time"]:
        fail(f"{name}: the flight did not land before the time cap")
    if flight.apogee <= elevation:
        fail(f"{name}: apogee {flight.apogee} is not above the site")
    burnout_s = finite(flight.rocket.motor.burn_out_time, "burn-out time")
    return {
        "apogee_agl_m": finite(flight.apogee - elevation, "apogee"),
        "apogee_time_s": finite(flight.apogee_time, "time to apogee"),
        "max_speed_m_s": finite(flight.max_speed, "maximum speed"),
        "max_mach": finite(flight.max_mach_number, "maximum Mach"),
        # RocketPy's `max_acceleration` is the largest |a| over the *whole* flight, which for
        # ndrt-2020 and prometheus-2022 is the parachute inflating, not the airframe's flight
        # load, and the two models deliberately differ there (recovery.py:15-18: hpr carries no
        # added mass and releases the drogue rather than replacing it). Both are recorded, with
        # the instant of each, so a case can gate the power-on load and say in writing what it
        # does with the other.
        "max_acceleration_m_s2": finite(flight.max_acceleration, "maximum acceleration"),
        "max_acceleration_time_s": finite(
            flight.max_acceleration_time, "time of maximum acceleration"
        ),
        "max_acceleration_power_on_m_s2": finite(
            flight.max_acceleration_power_on, "maximum power-on acceleration"
        ),
        "rail_exit_speed_m_s": finite(flight.out_of_rail_velocity, "rail-exit speed"),
        "rail_exit_time_s": finite(flight.out_of_rail_time, "rail-exit time"),
        "burnout_altitude_agl_m": finite(
            flight.z.get_value_opt(burnout_s) - elevation, "burn-out altitude"
        ),
        "burnout_speed_m_s": finite(flight.speed.get_value_opt(burnout_s), "burn-out speed"),
        "flight_time_s": finite(flight.t_final, "flight time"),
        # Where the flight went, not only how high: the horizontal distance from the pad of the
        # apogee and of the landing point (M2.1's "landing offset"), and the vertical speed at
        # landing (its "descent rates"). RocketPy's x and y are the centre of dry mass's, from
        # where it started (`apogee_x`, `x_impact`, flight.py:1156-1158, :1223-1226).
        "apogee_drift_m": finite(math.hypot(flight.apogee_x, flight.apogee_y), "apogee drift"),
        "landing_drift_m": finite(math.hypot(flight.x_impact, flight.y_impact), "landing drift"),
        "impact_speed_m_s": finite(-flight.impact_velocity, "impact vertical speed"),
    }


def series_of(flight, case):
    """The trajectory on a uniform grid from launch to landing, for the RMS comparison."""
    elevation = case["elevation"]
    step = flight.t_final / (SERIES_SAMPLES - 1)
    rows = []
    for index in range(SERIES_SAMPLES):
        t = min(index * step, flight.t_final)
        rows.append([
            finite(t, "series time"),
            finite(flight.z.get_value_opt(t) - elevation, "series height"),
            finite(flight.speed.get_value_opt(t), "series speed"),
        ])
    return rows


def run(document, case, own_drag):
    name = case["name"]
    # A calm-air copy flies its base case's rocket, rail and drag.
    base = case.get("base", name)
    rail = RAILS.get(base) or fail(f"{base} has no rail declared")
    inputs = recovery.mass_case(document, base)
    inputs["name"] = base
    rocket, thrust_path = build_rocket(inputs, own_drag)
    env = recovery.environment_of(case)
    flight = fly(rocket, env, rail, SOLVER)

    events = [
        {
            "name": parachute.name,
            "trigger_s": finite(trigger_s, "trigger time"),
            "deploy_s": finite(trigger_s + parachute.lag, "deployment time"),
            "lag_s": finite(parachute.lag, "lag"),
            "height_above_ground_at_trigger_m": finite(
                flight.z.get_value_opt(trigger_s) - case["elevation"], "height at the trigger"
            ),
        }
        for trigger_s, parachute in sorted(flight.parachute_events, key=lambda pair: pair[0])
    ]
    if not events:
        fail(f"{name}: no parachute deployed, so the descent is ballistic")
    devices = devices_of(inputs, rocket)
    metrics = metrics_of(flight, case, name, SOLVER)

    loose_inputs = recovery.mass_case(document, base)
    loose_inputs["name"] = base
    loose_rocket, _ = build_rocket(loose_inputs, own_drag)
    other_metrics = metrics_of(fly(loose_rocket, env, rail, LOOSE_SOLVER), case, name, LOOSE_SOLVER)
    solver_change_by_metric = {
        key: abs(other_metrics[key] - value) / abs(value)
        for key, value in metrics.items()
        if abs(value) > 1e-9
    }

    return {
        "name": name,
        "source": f"{inputs['source']}; site and wind: {case['site']}; rail: {rail['source']}",
        "design": f"validation/designs/rocketpy-{base}.json",
        "thrust_substitute": {
            "file": str(thrust_path),
            "sha256": inputs["motor"]["thrust_file_sha256"],
            "example_file": inputs["original_thrust_source"],
        },
        "drag": ({
            "own": own_drag_record(base),
            "source": OWN_DRAG[name]["source"],
        } if own_drag else {
            "cd0_vs_mach": [list(row) for row in DECLARED_CD0],
            "source": "declared by validation/oracles/rocketpy/flight.py; both codes fly it",
            "applies_to": "power_off_drag and power_on_drag alike",
        }) | {
            # The drag *force* is 0.5 rho V^2 A C_D, so "same drag" is only pinned if A is too.
            # RocketPy takes it from Rocket(radius); hpr takes its reference diameter from the
            # design file, a different source that agrees today. Recorded so a case can assert it.
            "reference_radius_m": finite(rocket.radius, "reference radius"),
            "reference_area_m2": finite(rocket.area, "reference area"),
        },
        # RocketPy's rail phase ends when its tracked point has moved `effective_1rl` along the
        # rail: the forward button, at its recorded position, reaching the top
        # (flight.py:1716-1730). The rail-exit metrics are defined by this distance.
        "rail": dict(rail, effective_1rl_m=finite(flight.effective_1rl, "effective_1rl")),
        # The motor as RocketPy flew it, so the harness can check hpr's against it rather than
        # trust the design (L75): the impulse and burn of the curve, the propellant, and the
        # reference pressure, which RocketPy leaves at None (no ambient-pressure correction).
        "motor": {
            "total_impulse_ns": finite(rocket.motor.total_impulse, "total impulse"),
            "burn_out_time_s": finite(rocket.motor.burn_out_time, "burn-out time"),
            "propellant_initial_mass_kg": finite(
                rocket.motor.propellant_initial_mass, "propellant mass"
            ),
            "reference_pressure_pa": rocket.motor.reference_pressure,
        },
        "environment": {
            "latitude_deg": case["latitude"],
            "longitude_deg": case["longitude"],
            "elevation_m": case["elevation"],
            "atmosphere": "rocketpy standard atmosphere (custom_atmosphere with no pressure or "
                          "temperature given)",
            "gravity": "rocketpy Somigliana normal gravity (environment.py:938-985)",
            "wind_u": case["wind_u"],
            "wind_v": case["wind_v"],
        },
        "dry_mass_kg": finite(rocket.dry_mass, "dry mass"),
        "peak_thrust_to_weight": peak_thrust_to_weight(rocket, env, case),
        "devices": devices,
        "events": events,
        "impact": {
            "time_s": finite(flight.t_final, "impact time"),
            "east_m": finite(flight.x_impact, "impact east"),
            "north_m": finite(flight.y_impact, "impact north"),
            "vertical_speed_m_s": finite(flight.impact_velocity, "impact vertical speed"),
        },
        "series": {
            "columns": ["time_s", "height_above_ground_m", "speed_m_s"],
            "samples": SERIES_SAMPLES,
            "rows": series_of(flight, case),
        },
        "metrics": metrics,
        "solver": dict(
            SOLVER,
            integrator="LSODA",
            source="flight.py:489-509, :611-628",
            loose=LOOSE_SOLVER,
            loose_metrics=other_metrics,
            relative_change_from_loose=solver_change_by_metric,
            worst_relative_change_from_loose=max(solver_change_by_metric.values()),
        ),
    }


def main():
    arguments = sys.argv[1:]
    own_drag = OWN_DRAG_FLAG in arguments
    keep = {argument for argument in arguments if argument != OWN_DRAG_FLAG}
    every = recovery.CASES + WHOLE_FLIGHT_ONLY_CASES
    if not own_drag:
        # Calm air is a same-drag comparison only: it measures the response to wind (issue #50).
        every = every + calm_air_cases(every)
    cases = [case for case in every if not keep or case["name"] in keep]
    if keep and len(cases) != len(keep):
        fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    document = recovery.mass_fixture()
    if own_drag:
        drag_fields = {
            "model": "Flight.u_dot_generalized (flight.py:2471-2709, the default "
                     "equations_of_motion='standard'): RocketPy's 6-DOF variable-mass rigid "
                     "body from the rail to apogee, then its point-mass parachute phase "
                     "(u_dot_parachute, flight.py:2710-2790), under each example's own drag "
                     "and RocketPy's Barrowman lift",
            "overrides": "none to the drag: each example flies its own, as RocketPy 1.13.0 "
                         "flies the example (the curves stay under refs/, ADR-009; each case "
                         "records its file and SHA-256); every parachute's noise is zero (it "
                         "draws from the global np.random), and the lags are the examples' own",
            "own_drag": {
                "why": "predicted mode compares hpr's own aerodynamics with the drag each "
                       "example ships, which RocketPy's authors took from RASAero, OpenRocket "
                       "or a team's own estimate; neither code's drag is the truth",
            },
        }
    else:
        drag_fields = {
            "model": "Flight.u_dot_generalized (flight.py:2471-2709, the default "
                     "equations_of_motion='standard'): RocketPy's 6-DOF variable-mass rigid "
                     "body from the rail to apogee, then its point-mass parachute phase "
                     "(u_dot_parachute, flight.py:2710-2790), under the drag table this "
                     "script declares",
            "overrides": "the drag is the constant C_D0 this script declares, handed to "
                         "power_off_drag and power_on_drag, because RocketPy's own exports "
                         "carry their own terms and are never committed (ADR-009); every "
                         "parachute's noise is zero (it draws from the global np.random), and "
                         "the lags are the examples' own",
            "declared_drag": {
                "cd0_vs_mach": [list(row) for row in DECLARED_CD0],
                "why": "same-drag mode scores the equations of motion, not the aerodynamics, "
                       "so both codes fly one declared table; a Mach-dependent curve written "
                       "here would be an uncited drag model inside the reference (L18)",
            },
        }
    print(
        json.dumps(
            {
                "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
                "generator": "validation/oracles/rocketpy/flight.py",
                "command": f"{COMMAND} {OWN_DRAG_FLAG}" if own_drag else COMMAND,
                **drag_fields,
                "mass_fixture": {
                    "file": recovery.MASS_FIXTURE,
                    "generator": document["generator"],
                    "thrust_curves": document["thrust_curves"],
                },
                "cases": [run(document, case, own_drag) for case in cases],
            },
            indent=1,
            sort_keys=False,
        )
    )


if __name__ == "__main__":
    main()
