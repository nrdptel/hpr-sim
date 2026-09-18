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
"""

import importlib.metadata
import json
import math
import sys
import warnings
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import recovery
from rocketpy import Environment, Flight, Rocket

warnings.filterwarnings("ignore")

COMMAND = "refs/venv/bin/python validation/oracles/rocketpy/flight.py"

# The drag both codes fly, as (Mach, C_D0) rows. Declared here, not read from RocketPy's exports:
# see the module docstring. Constant, so that nothing about the curve's shape is invented.
DECLARED_CD0 = [[0.0, 0.5], [3.0, 0.5]]

# RocketPy's thrust `Function` extrapolates to zero, and every bundled substitute curve starts at
# t = 0.008 s, so thrust(0) is exactly 0. On the rail `Flight.udot_rail1` then clamps the
# acceleration to zero (RocketPy's rocketpy/simulation/flight.py:1862-1871, not this file), so the derivative at the initial state is the zero
# vector. With RocketPy's defaults (`time_overshoot=True`, `max_time_step=inf`) the rail phase's
# bound is `max_time`, and LSODA, handed a zero derivative and a 6000 s horizon, takes one step
# straight over the whole burn: the rocket never leaves the rail and the run reports apogee 0.
#
# Bounding `max_time_step` is the fix, and it is what RocketPy's own examples do. Measured on
# valetudo with nothing else changed: at RocketPy's default tolerances the flight goes from apogee
# 0 to 778.881 m AGL, leaving the rail at 0.4557 s. So the reference is no longer sitting one
# decade from total failure, and the loose run below is a real one again.
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
}


def fail(message):
    sys.exit(f"flight.py: {message}")


def finite(value, label):
    value = float(value)
    if not math.isfinite(value):
        fail(f"{label} is not finite ({value})")
    return value


def build_rocket(inputs):
    """The example rocket with the declared drag, built as `recovery.py` builds it."""
    motor, thrust_path = recovery.build_motor(inputs["motor"])
    rocket_inputs = inputs["rocket"]
    rocket = Rocket(
        radius=rocket_inputs["radius"],
        mass=rocket_inputs["mass"],
        inertia=tuple(rocket_inputs["inertia"]),
        power_off_drag=[list(row) for row in DECLARED_CD0],
        power_on_drag=[list(row) for row in DECLARED_CD0],
        center_of_mass_without_motor=rocket_inputs["center_of_mass_without_motor"],
        coordinate_system_orientation=rocket_inputs["coordinate_system_orientation"],
    )
    rocket.add_motor(motor, position=rocket_inputs["motor_position"])
    recovery.add_geometry(rocket, inputs["geometry"])
    if not rocket.parachutes:
        fail(f"{inputs['name']} has no parachutes, so it would not come down under one")
    zero_noise(rocket)
    return rocket, thrust_path


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


def run(document, case):
    name = case["name"]
    rail = RAILS.get(name) or fail(f"{name} has no rail declared")
    inputs = recovery.mass_case(document, name)
    inputs["name"] = name
    rocket, thrust_path = build_rocket(inputs)
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
    metrics = metrics_of(flight, case, name, SOLVER)

    loose_inputs = recovery.mass_case(document, name)
    loose_inputs["name"] = name
    loose_rocket, _ = build_rocket(loose_inputs)
    other_metrics = metrics_of(fly(loose_rocket, env, rail, LOOSE_SOLVER), case, name, LOOSE_SOLVER)
    solver_change_by_metric = {
        key: abs(other_metrics[key] - value) / abs(value)
        for key, value in metrics.items()
        if abs(value) > 1e-9
    }

    return {
        "name": name,
        "source": f"{inputs['source']}; site and wind: {case['site']}; rail: {rail['source']}",
        "design": f"validation/designs/rocketpy-{name}.json",
        "thrust_substitute": {
            "file": str(thrust_path),
            "sha256": inputs["motor"]["thrust_file_sha256"],
            "example_file": inputs["original_thrust_source"],
        },
        "drag": {
            "cd0_vs_mach": [list(row) for row in DECLARED_CD0],
            "source": "declared by validation/oracles/rocketpy/flight.py; both codes fly it",
            "applies_to": "power_off_drag and power_on_drag alike",
            # The drag *force* is 0.5 rho V^2 A C_D, so "same drag" is only pinned if A is too.
            # RocketPy takes it from Rocket(radius); hpr takes its reference diameter from the
            # design file, a different source that agrees today. Recorded so a case can assert it.
            "reference_radius_m": finite(rocket.radius, "reference radius"),
            "reference_area_m2": finite(rocket.area, "reference area"),
        },
        "rail": dict(rail),
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
    keep = set(sys.argv[1:])
    cases = [case for case in recovery.CASES if not keep or case["name"] in keep]
    if keep and len(cases) != len(keep):
        fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    document = recovery.mass_fixture()
    print(
        json.dumps(
            {
                "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
                "generator": "validation/oracles/rocketpy/flight.py",
                "command": COMMAND,
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
                "mass_fixture": {
                    "file": recovery.MASS_FIXTURE,
                    "generator": document["generator"],
                    "thrust_curves": document["thrust_curves"],
                },
                "cases": [run(document, case) for case in cases],
            },
            indent=1,
            sort_keys=False,
        )
    )


if __name__ == "__main__":
    main()
