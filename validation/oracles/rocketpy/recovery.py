"""RocketPy 1.13.0's parachute descent for its own example rockets, as an oracle for hpr's recovery
model (M1.7a).

RocketPy's parachute phase is a point mass: it drops the airframe's aerodynamics and the rotational
equations, and integrates

    a = D/(m_p + m_a) - g(z) z_hat - 2 omega x v,   D = -(rho/2) (C_D S) |v - w| (v - w)

with `m_p` the rocket's dry mass, `m_a = k_a rho (2/3) pi R^2 H` an added mass that carries no
weight, the wind `w` and the density `rho` at the height above sea level, `g` the Somigliana normal
gravity and `omega` Earth's rotation (rocketpy/simulation/flight.py:2710-2790, MIT). The state's
quaternion and body rates freeze at deployment. Only the last deployed parachute's `C_D S` acts: a
main replaces a drogue rather than adding to it (flight.py:1404-1431).

hpr's descent phase is the same point mass, less the added mass (hpr has no cited formula for it;
it changes no equilibrium, only the transient) and with the drogue released by the main. The
comparison therefore starts where both models agree: a declared state, with the first device
opening immediately.

Each case takes an example rocket from the committed mass fixture
`validation/fixtures/design/rocketpy-rocket-mass.json`, which `rocket_mass.py` wrote: its rocket
and motor keyword arguments, its geometry and its parachutes, all transcribed from the example
there and cited, with a bundled public-domain thrust curve in place of the example's (ADR-007). No
input is transcribed twice. Each rocket flies from a declared post-burnout state near apogee,
with:

- the example's own parachutes, `C_D S`, triggers and sampling rates, but noise set to zero (the
  noise draws from the global `np.random` and would make the run irreproducible), and the first
  device's lag set to zero, so both simulators start with it open;
- a declared environment: the example's site and elevation, RocketPy's standard atmosphere and a
  declared wind, so nothing needs the network or a licensed weather file;
- RocketPy's own solver at `rtol = atol = 1e-8`, with every case run again at 1e-6 so the fixture
  can state how much of each metric is the solver's.

What it records per case: the environment (with density, gravity and wind sampled over the
descent, so hpr's own models can be checked against RocketPy's before the descent is compared), the
dry mass, the devices, the declared start, every parachute event, the impact, and the metrics hpr
compares: descent time, impact vertical speed, drift, and the vertical speed as each later device's
charge fires.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/rocketpy/recovery.py \
        > validation/fixtures/recovery/rocketpy-descent.json
"""

import copy
import hashlib
import importlib.metadata
import json
import math
import sys
import warnings
from pathlib import Path

from rocketpy import Function, GenericMotor, Rocket, SolidMotor
from rocketpy import Environment, Flight

warnings.filterwarnings("ignore")

COMMAND = "refs/venv/bin/python validation/oracles/rocketpy/recovery.py"

# The solver settings for every case: tighter than RocketPy's defaults (rtol 1e-6, atol 1e-3 on
# position) so the reference is the model's answer and not the solver's.
SOLVER = {"rtol": 1e-8, "atol": 1e-8, "max_time": 6000}

# A looser run of every case, to show the metrics are the model's answer and not the solver's.
LOOSE_SOLVER = {"rtol": 1e-6, "atol": 1e-6, "max_time": 6000}

# When the descent starts, s after ignition: after every example's burn-out, and deliberately not a
# multiple of any parachute's sampling interval. RocketPy checks its triggers on a grid anchored at
# t = 0 (flight.py:4499-4510), and a deployment landing exactly on the start of the flight's first
# phase collides with it (`Trying to add flight phase starting together with the one preceding it`).
START_S = 60.007

# The mass oracle's fixture, which records every example's inputs, and the bundled curves it
# substitutes for the examples' thrust files.
MASS_FIXTURE = "validation/fixtures/design/rocketpy-rocket-mass.json"
CURVES = Path("crates/hpr-motor/data/thrustcurve")

# Keys of the fixture's motor block that are not RocketPy keyword arguments.
MOTOR_META = ("motor_kind", "thrust_file", "thrust_file_sha256")

# Heights above the site where the environment is sampled for the comparison, m.
ENVIRONMENT_SAMPLES_AGL_M = [0.0, 100.0, 500.0, 1000.0, 2000.0, 3000.0]

# The declared cases: the example rocket, its site, the wind, and where the descent starts.
#
# Sites and elevations come from the examples themselves (cited per case). The winds are declared
# here: the examples' own winds come from GFS forecasts or Copernicus reanalysis files, which need
# the network or carry their own terms. Constant winds are given as scalars, profiles as
# (height above sea level, component) pairs, both in RocketPy's `custom_atmosphere` form.
CASES = [
    {
        # Calisto, RocketPy's test rocket. Site and elevation:
        # tests/fixtures/environment/environment_fixtures.py:36-42 (Spaceport America). Wind: the
        # constant 5 m/s east, 2 m/s north of tests/fixtures/flight/flight_fixtures.py:206-212.
        "name": "calisto-tests-motor-at-minus-1.373",
        "site": "tests/fixtures/environment/environment_fixtures.py:36-42 (elevation, latitude, "
                "longitude); wind from tests/fixtures/flight/flight_fixtures.py:206-212",
        "latitude": 32.990254,
        "longitude": -106.974998,
        "elevation": 1400,
        "wind_u": 5,
        "wind_v": 2,
        "start_height_agl": 3000.0,
    },
    {
        # Valetudo, whose example leaves the atmospheric model at the standard atmosphere with no
        # wind (docs/examples/valetudo_flight_sim.ipynb).
        "name": "valetudo",
        "site": "docs/examples/valetudo_flight_sim.ipynb (elevation, latitude, longitude; the "
                "example's set_atmospheric_model call is commented out, leaving no wind)",
        "latitude": -23.363611,
        "longitude": -48.011389,
        "elevation": 668,
        "wind_u": 0,
        "wind_v": 0,
        "start_height_agl": 800.0,
    },
    {
        # NDRT 2020. Site: docs/examples/ndrt_2020_flight_sim.ipynb. Wind: declared here as a
        # profile that shears with height, to exercise a wind that changes during the descent.
        "name": "ndrt-2020-nose-to-tail",
        "site": "docs/examples/ndrt_2020_flight_sim.ipynb (elevation, latitude, longitude); the "
                "wind profile is declared by this script",
        "latitude": 41.775447,
        "longitude": -86.572467,
        "elevation": 206,
        "wind_u": [[206, 2.0], [706, 6.0], [1206, 11.0]],
        "wind_v": [[206, -3.0], [706, -1.0], [1206, 4.0]],
        "start_height_agl": 1000.0,
    },
    {
        # Prometheus 2022, whose parachutes keep RocketPy's defaults: 100 Hz, no lag, no noise
        # (tests/fixtures/rockets/rocket_fixtures.py:407-416).
        "name": "prometheus-2022-generic-motor",
        "site": "docs/examples/prometheus_flight_sim.ipynb (elevation, latitude, longitude); the "
                "wind is declared by this script",
        "latitude": 32.939377,
        "longitude": -106.911986,
        "elevation": 1401,
        "wind_u": -4,
        "wind_v": 7,
        "start_height_agl": 3000.0,
    },
    {
        # Juno III, drogue only (docs/examples/juno3_flight_sim.ipynb).
        "name": "juno-iii",
        "site": "docs/examples/juno3_flight_sim.ipynb (elevation, latitude, longitude); the wind "
                "is declared by this script",
        "latitude": 32.939377,
        "longitude": -106.911986,
        "elevation": 1480,
        "wind_u": 8,
        "wind_v": -3,
        "start_height_agl": 1200.0,
    },
]


def fail(message):
    sys.exit(f"recovery.py: {message}")


def finite(value, label):
    value = float(value)
    if not math.isfinite(value):
        fail(f"{label} is not finite ({value})")
    return value


def mass_fixture():
    """The committed mass fixture, which holds every example's inputs."""
    document = json.loads(Path(MASS_FIXTURE).read_text())
    if document["oracle"] != f"rocketpy {importlib.metadata.version('rocketpy')}":
        fail(f"{MASS_FIXTURE} was written by {document['oracle']}")
    return document


def mass_case(document, name):
    """The fixture's case of this name."""
    for case in document["cases"]:
        if case["name"] == name:
            return copy.deepcopy(case)
    fail(f"{name} is not a case of {MASS_FIXTURE}")


def build_motor(inputs):
    """The example's motor with the bundled substitute curve, from the fixture's own keywords."""
    kwargs = {k: v for k, v in inputs.items() if k not in MOTOR_META}
    path = CURVES / inputs["thrust_file"]
    if hashlib.sha256(path.read_bytes()).hexdigest() != inputs["thrust_file_sha256"]:
        fail(f"{path} does not match the SHA-256 the mass fixture recorded")
    kwargs["dry_inertia"] = tuple(kwargs["dry_inertia"])
    if isinstance(kwargs.get("reshape_thrust_curve"), list):
        kwargs["reshape_thrust_curve"] = tuple(kwargs["reshape_thrust_curve"])
    cls = SolidMotor if inputs["motor_kind"] == "solid" else GenericMotor
    return cls(thrust_source=str(path), **kwargs), path


def add_geometry(rocket, geometry):
    """Applies the fixture's geometry with RocketPy's own methods, as `rocket_mass.py` does."""
    if geometry["nose"]:
        rocket.add_nose(**geometry["nose"])
    for fins in geometry["fin_sets"]:
        if fins["type"] != "trapezoidal":
            fail(f"unsupported fin set {fins['type']}")
        kwargs = {k: v for k, v in fins.items() if k not in ("type", "airfoil")}
        airfoil = fins.get("airfoil")
        if airfoil and "points" in airfoil:
            kwargs["airfoil"] = (Function(airfoil["points"]), airfoil["unit"])
        rocket.add_trapezoidal_fins(**kwargs)
    for tail in geometry["tails"]:
        rocket.add_tail(**tail)
    if geometry["rail_buttons"]:
        rocket.set_rail_buttons(**geometry["rail_buttons"])
    for chute in geometry["parachutes"]:
        trigger = chute["trigger"]
        kwargs = dict(chute, trigger=trigger["height_m"] if "height_m" in trigger else "apogee")
        # A parachute the example left at RocketPy's defaults has no such key in the fixture.
        if "noise" in kwargs:
            kwargs["noise"] = tuple(kwargs["noise"])
        rocket.add_parachute(**kwargs)


def build_rocket(case):
    """The example rocket, with the parachutes the fixture recorded."""
    motor, thrust_path = build_motor(case["motor"])
    inputs = case["rocket"]
    rocket = Rocket(
        radius=inputs["radius"],
        mass=inputs["mass"],
        inertia=tuple(inputs["inertia"]),
        power_off_drag=0.5,
        power_on_drag=0.5,
        center_of_mass_without_motor=inputs["center_of_mass_without_motor"],
        coordinate_system_orientation=inputs["coordinate_system_orientation"],
    )
    rocket.add_motor(motor, position=inputs["motor_position"])
    add_geometry(rocket, case["geometry"])
    if not rocket.parachutes:
        fail(f"{case['name']} has no parachutes")
    return rocket, thrust_path


def devices_of(case):
    """The parachutes in deployment order: the apogee triggers first, then the height triggers.

    RocketPy holds one `C_D S` at a time, so a device that fires later replaces the one before it.
    hpr sums its open devices, so the fixture records which device each one releases.
    """
    chutes = case["geometry"]["parachutes"]
    order = sorted(chutes, key=lambda c: 0 if c["trigger"]["kind"] == "apogee" else 1)
    if order[0]["trigger"]["kind"] != "apogee":
        fail(f"{case['name']}: no parachute triggers at apogee")
    for later in order[1:]:
        if later["trigger"]["kind"] != "descending_below_height_agl":
            fail(f"{case['name']}: unexpected trigger {later['trigger']}")
    return order


def apply_overrides(rocket, order):
    """Zeroes every parachute's noise and the first device's lag, on the built rocket."""
    by_name = {chute.name: chute for chute in rocket.parachutes}
    if sorted(by_name) != sorted(chute["name"] for chute in order):
        fail("the rocket's parachutes differ from the case's")
    for index, chute in enumerate(order):
        parachute = by_name[chute["name"]]
        parachute.noise = (0, 0, 0)
        parachute.noise_signal = [[-1e-6, 0]]
        parachute.noise_function = lambda: 0.0
        parachute.noise_bias, parachute.noise_deviation, parachute.noise_corr = 0, 0, (0, 1)
        if index == 0:
            parachute.lag = 0.0
    return [by_name[chute["name"]] for chute in order]


def environment_of(case):
    env = Environment(
        latitude=case["latitude"],
        longitude=case["longitude"],
        elevation=case["elevation"],
    )
    wind = {}
    for key in ("wind_u", "wind_v"):
        value = case[key]
        wind[key] = [[float(h), float(v)] for h, v in value] if isinstance(value, list) else value
    env.set_atmospheric_model(type="custom_atmosphere", pressure=None, temperature=None, **wind)
    return env


def environment_samples(env, case):
    """RocketPy's density, gravity and wind over the descent, for hpr to check its own models."""
    samples = []
    for above_ground in ENVIRONMENT_SAMPLES_AGL_M:
        if above_ground > case["start_height_agl"]:
            continue
        z = case["elevation"] + above_ground
        samples.append({
            "height_above_ground_m": above_ground,
            "height_msl_m": z,
            "pressure_pa": finite(env.pressure.get_value_opt(z), "pressure"),
            "temperature_k": finite(env.temperature.get_value_opt(z), "temperature"),
            "density_kg_m3": finite(env.density.get_value_opt(z), "density"),
            "gravity_m_s2": finite(env.gravity.get_value_opt(z), "gravity"),
            "wind_east_m_s": finite(env.wind_velocity_x.get_value_opt(z), "wind east"),
            "wind_north_m_s": finite(env.wind_velocity_y.get_value_opt(z), "wind north"),
        })
    return samples


def metrics_of(flight, case, start_t, start):
    """The metrics hpr compares, from a finished flight."""
    if flight.t_final >= SOLVER["max_time"]:
        fail(f"{case['name']}: the flight did not land before the time cap")
    if flight.impact_velocity >= 0.0:
        fail(f"{case['name']}: impact velocity {flight.impact_velocity} is not a descent")
    drift_east_m = finite(flight.x_impact - start[1], "drift east")
    drift_north_m = finite(flight.y_impact - start[2], "drift north")
    return {
        "descent_time_s": finite(flight.t_final - start_t, "descent time"),
        "impact_speed_m_s": -finite(flight.impact_velocity, "impact vertical speed"),
        "drift_east_m": drift_east_m,
        "drift_north_m": drift_north_m,
        "drift_m": math.hypot(drift_east_m, drift_north_m),
        "mean_descent_rate_m_s": case["start_height_agl"] / (flight.t_final - start_t),
    }


def run(document, case):
    name = case["name"]
    inputs = mass_case(document, name)
    rocket, thrust_path = build_rocket(inputs)
    order = devices_of(inputs)
    parachutes = apply_overrides(rocket, order)
    env = environment_of(case)

    # The declared start: after burn-out, just past apogee, drifting with the wind at that height.
    start_t = START_S
    z = case["elevation"] + case["start_height_agl"]
    if start_t <= rocket.motor.burn_out_time:
        fail(f"{name}: the start time is inside the burn")
    start = [
        start_t,
        0.0,
        0.0,
        z,
        finite(env.wind_velocity_x.get_value_opt(z), "wind east"),
        finite(env.wind_velocity_y.get_value_opt(z), "wind north"),
        -0.5,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    ]
    flight = Flight(
        rocket=rocket,
        environment=env,
        rail_length=1.0,
        inclination=90,
        heading=0,
        initial_solution=list(start),
        **SOLVER,
    )

    # Every parachute event, in order: the trigger time RocketPy logs, and the deployment that
    # follows it by the parachute's lag.
    events = []
    for trigger_s, parachute in sorted(flight.parachute_events, key=lambda pair: pair[0]):
        events.append({
            "name": parachute.name,
            "trigger_s": finite(trigger_s, "trigger time"),
            "deploy_s": finite(trigger_s + parachute.lag, "deployment time"),
            "lag_s": finite(parachute.lag, "lag"),
            "vertical_speed_at_trigger_m_s": finite(
                flight.vz.get_value_opt(trigger_s), "vertical speed at the trigger"
            ),
            "height_above_ground_at_trigger_m": finite(
                flight.z.get_value_opt(trigger_s) - case["elevation"], "height at the trigger"
            ),
        })
    if len(events) != len(parachutes):
        fail(f"{name}: {len(events)} parachute events for {len(parachutes)} parachutes")
    metrics = metrics_of(flight, case, start_t, start)

    # The same case at RocketPy's looser tolerances: how much of each metric is the solver's.
    loose_rocket, _ = build_rocket(mass_case(document, name))
    apply_overrides(loose_rocket, order)
    loose_metrics = metrics_of(
        Flight(
            rocket=loose_rocket,
            environment=env,
            rail_length=1.0,
            inclination=90,
            heading=0,
            initial_solution=list(start),
            **LOOSE_SOLVER,
        ),
        case,
        start_t,
        start,
    )
    solver_change = max(
        abs(loose_metrics[key] - value) / abs(value)
        for key, value in metrics.items()
        if abs(value) > 1e-9
    )

    metrics = metrics_of(flight, case, start_t, start)
    # The same case at RocketPy's looser tolerances: how much of each metric is the solver's.
    loose = Flight(
        rocket=rocket,
        environment=env,
        rail_length=1.0,
        inclination=90,
        heading=0,
        initial_solution=list(start),
        **LOOSE_SOLVER,
    )
    loose_metrics = metrics_of(loose, case, start_t, start)
    solver_change = max(
        abs(loose_metrics[key] - value) / abs(value)
        for key, value in metrics.items()
        if abs(value) > 1e-9
    )
    return {
        "name": name,
        "source": f"{inputs['source']}; site and wind: {case['site']}",
        "design": f"validation/designs/rocketpy-{name}.json",
        "thrust_substitute": {
            "file": str(thrust_path),
            "sha256": inputs["motor"]["thrust_file_sha256"],
            "example_file": inputs["original_thrust_source"],
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
            "samples": environment_samples(env, case),
        },
        "dry_mass_kg": finite(rocket.dry_mass, "dry mass"),
        "devices": [
            {
                "name": parachute.name,
                "cd_s_m2": finite(parachute.cd_s, "cd_s"),
                "trigger": chute["trigger"],
                "sampling_rate_hz": finite(parachute.sampling_rate, "sampling rate"),
                "lag_s": finite(parachute.lag, "lag"),
                "example_lag_s": finite(chute.get("lag", 0), "the example's lag"),
                "example_noise": chute.get("noise", [0, 0, 0]),
                "added_mass_kg_at_start": finite(
                    parachute.added_mass_coefficient
                    * env.density.get_value_opt(z)
                    * (2 / 3)
                    * math.pi
                    * parachute.radius**2
                    * parachute.height,
                    "added mass",
                ),
            }
            for parachute, chute in zip(parachutes, order)
        ],
        "start": {
            "time_s": start_t,
            "height_above_ground_m": case["start_height_agl"],
            "position_msl_m": [start[1], start[2], start[3]],
            "velocity_m_s": [start[4], start[5], start[6]],
        },
        "events": events,
        "impact": {
            "time_s": finite(flight.t_final, "impact time"),
            "east_m": finite(flight.x_impact, "impact east"),
            "north_m": finite(flight.y_impact, "impact north"),
            "vertical_speed_m_s": finite(flight.impact_velocity, "impact vertical speed"),
        },
        "metrics": metrics,
        "solver": dict(
            SOLVER,
            integrator="LSODA",
            source="flight.py:489-509, :611-628",
            loose=LOOSE_SOLVER,
            loose_metrics=loose_metrics,
            worst_relative_change_from_loose=solver_change,
        ),
    }


def main():
    keep = set(sys.argv[1:])
    cases = [case for case in CASES if not keep or case["name"] in keep]
    if keep and len(cases) != len(keep):
        fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    document = mass_fixture()
    print(
        json.dumps(
            {
                "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
                "generator": "validation/oracles/rocketpy/recovery.py",
                "command": COMMAND,
                "model": "Flight.u_dot_parachute (flight.py:2710-2790): a point mass of the "
                         "rocket's dry mass under the last deployed parachute's C_D S, with an "
                         "added mass that carries no weight, the wind and density at the height "
                         "above sea level, Somigliana gravity and Earth's rotation",
                "overrides": "every parachute's noise is zero (RocketPy's noise draws from the "
                             "global np.random), and the first device's lag is zero so that the "
                             "descent starts where both models agree",
                "mass_fixture": {
                    "file": MASS_FIXTURE,
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
