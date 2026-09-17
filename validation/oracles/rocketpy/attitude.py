"""RocketPy 1.13.0's initial attitude quaternion for a set of launch rails, as an oracle for hpr-core's
`LaunchAngles` (docs/physics/frames.md).

Each case builds a real `Flight` (the getting-started Calisto rocket and Cesaroni M1670 motor, with
rail buttons) and reads the Euler parameters (e0, e1, e2, e3), scalar first, from the first row of
the flight solution. This exercises RocketPy's own mapping from inclination, heading and rail-button
angle to its 3-1-3 Euler angles (rocketpy/simulation/flight.py, `Flight.__init_flight_state`), for
both rocket coordinate systems: with `tail_to_nose` the spin angle is the rail-button angular
position; with `nose_to_tail` it is 2*pi minus that position.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/rocketpy/attitude.py \
        > validation/fixtures/earth/rocketpy-attitude.json
"""

import importlib.metadata
import json
import warnings
from pathlib import Path

from rocketpy import Environment, Flight, Rocket, SolidMotor

warnings.filterwarnings("ignore")

DATA = Path("refs/rocketpy/data")

# (heading deg, inclination deg, rail-button angular position deg, rocket coordinate system):
# rail setups from vertical to nearly horizontal, every quadrant of heading, both orientations.
CASES = [
    (0.0, 90.0, 0.0, "tail_to_nose"),
    (0.0, 85.0, 45.0, "tail_to_nose"),
    (90.0, 80.0, 0.0, "tail_to_nose"),
    (135.0, 84.0, 120.0, "tail_to_nose"),
    (200.0, 60.0, 300.0, "tail_to_nose"),
    (271.5, 88.5, 10.0, "nose_to_tail"),
    (359.0, 45.0, 45.0, "nose_to_tail"),
    (17.0, 30.0, 200.0, "nose_to_tail"),
]


def calisto(orientation, button_angle_deg):
    csys = 1 if orientation == "tail_to_nose" else -1
    motor = SolidMotor(
        thrust_source=str(DATA / "motors/cesaroni/Cesaroni_M1670.eng"),
        burn_time=3.9,
        dry_mass=1.815,
        dry_inertia=(0.125, 0.125, 0.002),
        center_of_dry_mass_position=0.317,
        nozzle_position=0,
        grain_number=5,
        grain_density=1815,
        nozzle_radius=33 / 1000,
        throat_radius=11 / 1000,
        grain_separation=5 / 1000,
        grain_outer_radius=33 / 1000,
        grain_initial_height=120 / 1000,
        grains_center_of_mass_position=0.397,
        grain_initial_inner_radius=15 / 1000,
        interpolation_method="linear",
        coordinate_system_orientation="nozzle_to_combustion_chamber",
    )
    rocket = Rocket(
        radius=0.0635,
        mass=14.426,
        inertia=(6.321, 6.321, 0.034),
        power_off_drag=str(DATA / "rockets/calisto/powerOffDragCurve.csv"),
        power_on_drag=str(DATA / "rockets/calisto/powerOnDragCurve.csv"),
        center_of_mass_without_motor=0,
        coordinate_system_orientation=orientation,
    )
    rocket.add_motor(motor, position=-1.255 * csys)
    rocket.set_rail_buttons(
        upper_button_position=0.082 * csys,
        lower_button_position=-0.618 * csys,
        angular_position=button_angle_deg,
    )
    return rocket


env = Environment(latitude=32.990254, longitude=-106.974998, elevation=1400)
cases = []
for heading, inclination, button_angle, orientation in CASES:
    flight = Flight(
        rocket=calisto(orientation, button_angle),
        environment=env,
        rail_length=5.2,
        inclination=inclination,
        heading=heading,
        max_time=0.01,
    )
    e0, e1, e2, e3 = (float(v) for v in flight.solution[0][7:11])
    cases.append({
        "heading_deg": heading,
        "inclination_deg": inclination,
        "rail_button_angle_deg": button_angle,
        "coordinate_system_orientation": orientation,
        "e0": e0,
        "e1": e1,
        "e2": e2,
        "e3": e3,
    })

print(json.dumps({
    "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
    "generator": "validation/oracles/rocketpy/attitude.py",
    "cases": cases,
}, indent=2))
