"""RocketPy 1.13.0's rocket mass, centre of mass and inertia over time (the rocket plus its placed
motor) for RocketPy's example rockets, as an oracle for hpr's mass composition (M1.4b; model summary
in docs/research/rocketpy-rocket-mass.md).

Each case builds an example rocket from the example's own inputs: the rocket's radius, mass without
motor, inertia, centre of mass without motor, orientation and motor position, and every motor input
(dry mass, dry inertia and centre, nozzle, grains or propellant chamber, orientation). Only the
thrust source is replaced (see "Thrust curves" below). The example's nose, fins, tails, rail buttons
and parachutes are recorded as data, with RocketPy's keyword names, for later design files. The
script applies them to the rocket, so a mistyped keyword fails here; none of them enters a mass
property. Keywords an example leaves out keep RocketPy's defaults: fin cant 0, a right trapezoid
(sweep = root chord - tip chord), fin and tail reference radius = rocket radius, no airfoil, nose
bluffness 0, nose base radius = rocket radius, rail-button angle 45 degrees, parachute sampling
rate 100 Hz, lag 0 s and noise (0, 0, 0). Airfoil lift curves read from RocketPy's data files are
recorded by file name and left off the rocket.

What it pins, per case:

- Scalars: the rocket's dry mass (rocket without motor plus motor dry mass), centre of dry mass, dry
  I_11 and I_33 about that centre, the motor's centre of dry mass and nozzle exit in rocket
  coordinates, and the motor's initial propellant mass, total impulse and burn-out time.
- Time series on a fixed grid (100 points from t = 0 to burn-out, then 3 points after): the motor's
  propellant mass, and the rocket's total mass, centre of mass, I_11 and I_33 about the centre of
  dry mass, and I_11 about the instantaneous centre of mass.
- For a `SolidMotor`, the same at up to 60 of the LSODA steps of its grain geometry, evenly spread
  by index (`knot_series`; solid_motor.py:603-630). There RocketPy's functions hold computed values
  rather than interpolants, so a comparison is not limited by RocketPy's resampling. A
  `GenericMotor` has no such grid: its mass is exact on the even grid and its inertias at the thrust
  knots, so it has no `knot_series`.

Frames and reference points (rocketpy/rocket/rocket.py):

- Positions are metres along the rocket axis in the rocket's own coordinates: the origin is wherever
  the example puts it, and +z points toward the nose for "tail_to_nose" and toward the tail for
  "nose_to_tail" (`:312-316`). A nose's position is its tip; a tail's and a fin set's is their
  forward end (`add_nose`, `add_tail`, `add_trapezoidal_fins`).
- `add_motor` puts the motor's coordinate origin, not its nozzle, at `motor_position`. A motor point
  z_m lands at motor_position + s z_m, where s is the product of the rocket's sign and the motor's
  (+1 for "nozzle_to_combustion_chamber", rocketpy/motors/motor.py:269-273) (`:1113-1125`).
- `I_11` and `I_33` are about the rocket's centre of dry mass, a fixed point (`:853-893`,
  `:928-957`), not the instantaneous centre of mass.
- `I_11_about_cg = I_11 - total_mass (center_of_mass - center_of_dry_mass_position)^2`: the shift
  `Flight` applies before it solves for the angular acceleration
  (rocketpy/simulation/flight.py:2503-2525, `I_CM = inertia_tensor - H`, with the offset from
  `com_to_cdm_function`, rocket.py:1013-1021). The script checks that offset's square against
  (center_of_mass - center_of_dry_mass_position)^2.
- `I_22` equals `I_11` and the products of inertia are zero for these inputs. The script checks both
  instead of storing them, and fails on a nonzero product.

Every Function is read with `get_value_opt`, as `Flight` does. Rocket Functions are Function
arithmetic on the motor's, so between knots they are linear interpolants of the products at the
knots (docs/research/rocketpy-solid-motor.md). A `GenericMotor`'s propellant mass is the mass-flow
integral sampled at 100 evenly spaced times from the first to the last thrust knot, which are this
grid's burn times (`Motor.propellant_mass`, motor.py:470-481; `Function.integral_function`,
rocketpy/mathutils/function.py:3243-3295); its propellant inertias are sampled at the thrust knots
(motor.py:1599-1603, :1647-1649).

`GenericMotor` (Prometheus) keeps its propellant as a solid cylinder of the chamber's radius and
height, centred at `chamber_position` with inertia in proportion to the remaining mass
(motor.py:1566-1649), burned by impulse fraction (motor.py:483-524). With
`center_of_dry_mass_position` left as None, RocketPy uses `chamber_position` (motor.py:1518-1523).

Thrust curves. RocketPy's data files carry their own terms (THIRD-PARTY-NOTICES.md), so each case
reads the bundled public-domain .eng curve (crates/hpr-motor/data/thrustcurve/catalog.json) closest
in total impulse to the example's motor, with `burn_time=None` (the curve's own knot range), no
reshaping, and linear interpolation. The script checks the catalog's license and SHA-256. The output
records the curve's path and SHA-256, the example's original thrust file name, and the burn options
the example gave it. The propellant mass comes from the grain geometry (SolidMotor) or the stated
initial mass (GenericMotor), so the substitute changes when the propellant burns, not how much there
is. An .eng file must not contain a `0 0` point (see solid_motor.py); no bundled curve does.

Which examples. Eight cases from seven rockets (Calisto at two motor positions), plus Prometheus's
`GenericMotor`. Cavour's motor has no dry mass or inertia, so its mass tests little that Juno III
doesn't; it is here because its drag curve is labelled RASAero II, which M1.5b compares (ADR-009).
Left out: Valkyrie, whose inputs exist only in RocketPy's data file
`data/rockets/valkyrie/VLK.json` (data files carry their own terms), and Andromeda, Astra, Camoes,
Erebus 11, Genesis and Lince, whose motors have no dry mass or inertia and so test nothing Bella Lui,
Valetudo and Juno III don't.

`--example-curves` reads each example's own thrust file from refs/rocketpy/data/motors with the
example's burn options instead, as a local cross-check of docs/research/rocketpy-rocket-mass.md.
Its output stays under refs/ and is never committed.

Run from the repository root with the oracle environment (optionally naming cases to keep):

    refs/venv/bin/python validation/oracles/rocketpy/rocket_mass.py \
        > validation/fixtures/design/rocketpy-rocket-mass.json

    refs/venv/bin/python validation/oracles/rocketpy/rocket_mass.py --example-curves \
        calisto-getting-started-motor-at-minus-1.255 calisto-tests-motor-at-minus-1.373 \
        > refs/scratch/rocketpy-rocket-mass-example-curves.json
"""

import copy
import hashlib
import importlib.metadata
import json
import math
import re
import sys
import warnings
from pathlib import Path

import numpy as np
from rocketpy import Function, GenericMotor, Rocket, SolidMotor

warnings.filterwarnings("ignore")


CURVES = Path("crates/hpr-motor/data/thrustcurve")
EXAMPLE_MOTORS = Path("refs/rocketpy/data/motors")

# Rocket keyword arguments every case records (drag never enters a mass property, so the script
# passes a constant drag coefficient instead of the example's).
ROCKET_KEYS = [
    "radius", "mass", "inertia", "center_of_mass_without_motor", "coordinate_system_orientation",
    "motor_position",
]

# Motor keyword arguments every case records, including the ones an example leaves at RocketPy's
# default, so the fixture needs no defaults to read.
SOLID_KEYS = [
    "dry_mass", "dry_inertia", "center_of_dry_mass_position", "nozzle_position", "nozzle_radius",
    "throat_radius", "grain_number", "grain_density", "grain_outer_radius",
    "grain_initial_inner_radius", "grain_initial_height", "grain_separation",
    "grains_center_of_mass_position", "coordinate_system_orientation", "burn_time",
    "reshape_thrust_curve", "interpolation_method", "only_radial_burn",
]
GENERIC_KEYS = [
    "dry_mass", "dry_inertia", "center_of_dry_mass_position", "nozzle_position", "nozzle_radius",
    "propellant_initial_mass", "chamber_radius", "chamber_height", "chamber_position",
    "coordinate_system_orientation", "burn_time", "reshape_thrust_curve", "interpolation_method",
]

APOGEE = {"kind": "apogee"}
NOISE = [0, 8.3, 0.5]


def below(height_m):
    """RocketPy's numeric trigger: falling, below `height_m` above ground (parachute.py:354-361)."""
    return {"kind": "descending_below_height_agl", "height_m": height_m}


def calisto_motor():
    # getting_started.ipynb:298-315; tests/fixtures/motor/solid_motor_fixtures.py:20-38 is the same.
    return {
        "dry_mass": 1.815,
        "dry_inertia": [0.125, 0.125, 0.002],
        "center_of_dry_mass_position": 0.317,
        "nozzle_position": 0,
        "nozzle_radius": 33 / 1000,
        "throat_radius": 11 / 1000,
        "grain_number": 5,
        "grain_density": 1815,
        "grain_outer_radius": 33 / 1000,
        "grain_initial_inner_radius": 15 / 1000,
        "grain_initial_height": 120 / 1000,
        "grain_separation": 5 / 1000,
        "grains_center_of_mass_position": 0.397,
        "coordinate_system_orientation": "nozzle_to_combustion_chamber",
        "burn_time": None,
        "reshape_thrust_curve": False,
        "interpolation_method": "linear",
        "only_radial_burn": False,
    }


# Impulses in the comments are RocketPy's trapezoid integrals: a substitute's over its whole curve,
# an example motor's over its own burn_time or reshaped curve.
CASES = [
    {
        # docs/notebooks/getting_started.ipynb: motor :298-315 (Cesaroni_M1670.eng, burn_time 3.9),
        # rocket :427-435, rail buttons :437-441, add_motor(position=-1.255) :458, nose, fins and
        # tail :500-515, parachutes :732-748. Substitute: Loki M1378LR (5362 N s against the M1670's
        # 6026 N s), shared with the next case so the two differ only in motor position.
        "name": "calisto-getting-started-motor-at-minus-1.255",
        "source": "docs/notebooks/getting_started.ipynb:298-315 (motor), :427-441 (rocket, rail "
                  "buttons), :458 (add_motor), :500-515 (nose, fins, tail), :732-748 (parachutes)",
        "rocket": {
            "radius": 127 / 2000,
            "mass": 14.426,
            "inertia": [6.321, 6.321, 0.034],
            "center_of_mass_without_motor": 0,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": -1.255,
        },
        "motor_kind": "solid",
        "motor": calisto_motor(),
        "thrust_file": "curves/5f4294d20002e90000000867.eng",
        "example_thrust": {"file": "cesaroni/Cesaroni_M1670.eng", "burn_time": 3.9},
        "geometry": {
            "nose": {"length": 0.55829, "kind": "vonKarman", "position": 1.278},
            "fin_sets": [{
                "type": "trapezoidal", "n": 4, "root_chord": 0.120, "tip_chord": 0.060,
                "span": 0.110, "position": -1.04956, "cant_angle": 0,
                "airfoil": {"file": "NACA0012-radians.txt", "unit": "radians"},
            }],
            "tails": [{"top_radius": 0.0635, "bottom_radius": 0.0435, "length": 0.060,
                       "position": -1.194656}],
            "rail_buttons": {"upper_button_position": 0.0818, "lower_button_position": -0.618,
                             "angular_position": 45},
            "parachutes": [
                {"name": "Main", "cd_s": 10.0, "trigger": below(800), "sampling_rate": 105,
                 "lag": 1.5, "noise": NOISE},
                {"name": "Drogue", "cd_s": 1.0, "trigger": APOGEE, "sampling_rate": 105,
                 "lag": 1.5, "noise": NOISE},
            ],
        },
    },
    {
        # RocketPy's test rocket: tests/fixtures/rockets/rocket_fixtures.py rocket :18-26,
        # add_motor(position=-1.373) :50; motor tests/fixtures/motor/solid_motor_fixtures.py:20-38
        # (Cesaroni_M1670.eng, burn_time 3.9). Geometry from the `calisto_robust` fixture
        # (rocket_fixtures.py:171-183): nose, tail and fins
        # tests/fixtures/surfaces/surface_fixtures.py:25-31, :43-49, :61-72, whose `rocket_radius`
        # is add_tail's and add_trapezoidal_fins' `radius`; parachutes
        # tests/fixtures/parachutes/parachute_fixtures.py:55-62, :80-87, whose trigger functions
        # (:16-18, :33-35) are RocketPy's "apogee" and 800 m triggers.
        "name": "calisto-tests-motor-at-minus-1.373",
        "source": "tests/fixtures/rockets/rocket_fixtures.py:18-26 (rocket), :50 (add_motor), "
                  ":171-183 (surfaces, rail buttons, parachutes); "
                  "tests/fixtures/motor/solid_motor_fixtures.py:20-38 (motor); "
                  "tests/fixtures/surfaces/surface_fixtures.py:25-72; "
                  "tests/fixtures/parachutes/parachute_fixtures.py:16-87",
        "rocket": {
            "radius": 0.0635,
            "mass": 14.426,
            "inertia": [6.321, 6.321, 0.034],
            "center_of_mass_without_motor": 0,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": -1.373,
        },
        "motor_kind": "solid",
        "motor": calisto_motor(),
        "thrust_file": "curves/5f4294d20002e90000000867.eng",
        "example_thrust": {"file": "cesaroni/Cesaroni_M1670.eng", "burn_time": 3.9},
        "geometry": {
            "nose": {"length": 0.55829, "kind": "vonkarman", "position": 1.160,
                     "base_radius": 0.0635},
            "fin_sets": [{
                "type": "trapezoidal", "n": 4, "root_chord": 0.120, "tip_chord": 0.040,
                "span": 0.100, "position": -1.168, "cant_angle": 0, "radius": 0.0635,
            }],
            "tails": [{"top_radius": 0.0635, "bottom_radius": 0.0435, "length": 0.060,
                       "position": -1.313, "radius": 0.0635}],
            "rail_buttons": {"upper_button_position": 0.082, "lower_button_position": -0.618,
                             "angular_position": 0},
            "parachutes": [
                {"name": "calisto_main_chute", "cd_s": 10.0, "trigger": below(800),
                 "sampling_rate": 105, "lag": 1.5, "noise": NOISE},
                {"name": "calisto_drogue_chute", "cd_s": 1.0, "trigger": APOGEE,
                 "sampling_rate": 105, "lag": 1.5, "noise": NOISE},
            ],
        },
    },
    {
        # docs/examples/bella_lui_flight_sim.ipynb: parameters :75-115, motor :257-275
        # (AeroTech_K828FJ.eng, burn_time 2.43), rocket :366-377 (default "tail_to_nose"), rail
        # buttons :378, add_motor :379, nose, fins and tail :395-413, parachute :429-436.
        # Substitute: AMW 2245K1075-P (2255 N s against the K828FJ's 2071 N s).
        "name": "bella-lui",
        "source": "docs/examples/bella_lui_flight_sim.ipynb:75-115 (parameters), :257-275 (motor), "
                  ":366-379 (rocket, rail buttons, add_motor), :395-413 (nose, fins, tail), "
                  ":429-436 (parachute)",
        "rocket": {
            "radius": 156 / 2000,
            "mass": 18.227 - 0.001,
            "inertia": [0.78267, 0.78267, 0.064244],
            "center_of_mass_without_motor": 0,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": -1.1356,
        },
        "motor_kind": "solid",
        "motor": {
            "dry_mass": 0.001,
            "dry_inertia": [0, 0, 0],
            "center_of_dry_mass_position": 0.3,
            "nozzle_position": 0,
            "nozzle_radius": 44.45 / 1000,
            "throat_radius": 21.4376 / 1000,
            "grain_number": 3,
            "grain_density": 782.4,
            "grain_outer_radius": 85.598 / 2000,
            "grain_initial_inner_radius": 33.147 / 1000,
            "grain_initial_height": 152.4 / 1000,
            "grain_separation": 3 / 1000,
            "grains_center_of_mass_position": 0.3,
            "coordinate_system_orientation": "nozzle_to_combustion_chamber",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
            "only_radial_burn": False,
        },
        "thrust_file": "curves/5f4294d20002e9000000073d.eng",
        "example_thrust": {"file": "aerotech/AeroTech_K828FJ.eng", "burn_time": 2.43},
        "geometry": {
            "nose": {"length": 0.242, "kind": "tangent", "position": 1.3 + 0.242},
            "fin_sets": [{"type": "trapezoidal", "n": 3, "root_chord": 0.280, "tip_chord": 0.125,
                          "span": 0.200, "position": -0.75}],
            "tails": [{"top_radius": 156 / 2000, "bottom_radius": 135 / 2000, "length": 0.050,
                       "position": -1.0856}],
            "rail_buttons": {"upper_button_position": 0.1, "lower_button_position": -0.5},
            "parachutes": [
                {"name": "Drogue", "cd_s": math.pi / 4, "trigger": APOGEE, "sampling_rate": 105,
                 "lag": 1, "noise": NOISE},
            ],
        },
    },
    {
        # docs/examples/ndrt_2020_flight_sim.ipynb: parameters :73-122, motor :199-217
        # (Cesaroni_4895L1395-P.eng, burn_time 3.433, "combustion_chamber_to_nozzle"), rocket
        # :308-320 ("nose_to_tail"), rail buttons :321, add_motor :323, nose, fins and transition
        # :339-358, parachutes :374-389. Substitute: AeroTech L2500ST (4672 N s against the L1395's
        # 4895 N s).
        "name": "ndrt-2020-nose-to-tail",
        "source": "docs/examples/ndrt_2020_flight_sim.ipynb:73-122 (parameters), :199-217 (motor), "
                  ":308-323 (rocket, rail buttons, add_motor), :339-358 (nose, fins, transition), "
                  ":374-389 (parachutes)",
        "rocket": {
            "radius": 0.1015,
            "mass": 18.998,
            "inertia": [73.316, 73.316, 0.15982],
            "center_of_mass_without_motor": 1.3,
            "coordinate_system_orientation": "nose_to_tail",
            "motor_position": 3.391,
        },
        "motor_kind": "solid",
        "motor": {
            "dry_mass": 1.848,
            "dry_inertia": [0, 0, 0],
            "center_of_dry_mass_position": -0.35,
            "nozzle_position": 0,
            "nozzle_radius": 0.02475,
            "throat_radius": 0.01075,
            "grain_number": 5,
            "grain_density": 1519.708,
            "grain_outer_radius": 0.033,
            "grain_initial_inner_radius": 0.015,
            "grain_initial_height": 0.12,
            "grain_separation": 0.003,
            "grains_center_of_mass_position": -0.35,
            "coordinate_system_orientation": "combustion_chamber_to_nozzle",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
            "only_radial_burn": False,
        },
        "thrust_file": "curves/5f4294d20002e9000000085f.eng",
        "example_thrust": {"file": "cesaroni/Cesaroni_4895L1395-P.eng", "burn_time": 3.433},
        "geometry": {
            "nose": {"length": 0.610, "kind": "tangent", "position": 0},
            "fin_sets": [{"type": "trapezoidal", "n": 4, "root_chord": 0.152, "tip_chord": 0.0762,
                          "span": 0.165, "position": 3.050, "sweep_angle": 13, "radius": 0.0775}],
            "tails": [{"top_radius": 0.1015, "bottom_radius": 0.0775, "length": 0.127,
                       "position": 1.2}],
            "rail_buttons": {"upper_button_position": 1.5, "lower_button_position": 2,
                             "angular_position": 45},
            "parachutes": [
                {"name": "Drogue",
                 "cd_s": 1.5 * math.pi * (24 * 25.4 / 1000) * (24 * 25.4 / 1000) / 4,
                 "trigger": APOGEE, "sampling_rate": 105, "lag": 1, "noise": NOISE},
                {"name": "Main",
                 "cd_s": 2.2 * math.pi * (120 * 25.4 / 1000) * (120 * 25.4 / 1000) / 4,
                 "trigger": below(167.64), "sampling_rate": 105, "lag": 1, "noise": NOISE},
            ],
        },
    },
    {
        # docs/examples/valetudo_flight_sim.ipynb: parameters :72-110, motor :245-263
        # (keron_thrust_curve.csv, burn_time 5.274), rocket :334-345 (default "tail_to_nose"), rail
        # buttons :346, add_motor :347, nose and fins :363-374 (no tail), parachute :390-397.
        # Substitute: AeroTech K400C (1307 N s against the Keron's 1415 N s).
        "name": "valetudo",
        "source": "docs/examples/valetudo_flight_sim.ipynb:72-110 (parameters), :245-263 (motor), "
                  ":334-347 (rocket, rail buttons, add_motor), :363-374 (nose, fins), "
                  ":390-397 (parachute)",
        "rocket": {
            "radius": 40.45 / 1000,
            "mass": 8.257,
            "inertia": [3.675, 3.675, 0.007],
            "center_of_mass_without_motor": 0,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": -1.024,
        },
        "motor_kind": "solid",
        "motor": {
            "dry_mass": 0.001,
            "dry_inertia": [0, 0, 0],
            "center_of_dry_mass_position": 0.42,
            "nozzle_position": 0,
            "nozzle_radius": 21.642 / 1000,
            "throat_radius": 8 / 1000,
            "grain_number": 6,
            "grain_density": 1707,
            "grain_outer_radius": 21.4 / 1000,
            "grain_initial_inner_radius": 9.65 / 1000,
            "grain_initial_height": 120 / 1000,
            "grain_separation": 6 / 1000,
            "grains_center_of_mass_position": 0.42,
            "coordinate_system_orientation": "nozzle_to_combustion_chamber",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
            "only_radial_burn": False,
        },
        "thrust_file": "curves/5f4294d20002e9000000086c.eng",
        "example_thrust": {"file": "projeto-jupiter/keron_thrust_curve.csv", "burn_time": 5.274},
        "geometry": {
            "nose": {"length": 0.274, "kind": "tangent", "position": 1.134},
            "fin_sets": [{"type": "trapezoidal", "n": 3, "root_chord": 0.058, "tip_chord": 0.018,
                          "span": 0.077, "position": -0.906}],
            "tails": [],
            "rail_buttons": {"upper_button_position": 0.224, "lower_button_position": -0.93,
                             "angular_position": 30},
            "parachutes": [
                {"name": "Drogue", "cd_s": 0.349 * 1.3, "trigger": APOGEE, "sampling_rate": 105,
                 "lag": 1, "noise": NOISE},
            ],
        },
    },
    {
        # docs/examples/juno3_flight_sim.ipynb: motor :168-185 (mandioca_thrust_curve.csv reshaped
        # to 5.8 s and 8800 N s; default "nozzle_to_combustion_chamber"), rocket :246-254, add_motor
        # :255, nose :275-279, fins :299-307, tail :316-318, rail buttons :327-331, parachute
        # :340-347. With a substitute curve the script doesn't reshape. Substitute: Cesaroni
        # 8187M1545-P (8182 N s against the reshaped 8800 N s).
        "name": "juno-iii",
        "source": "docs/examples/juno3_flight_sim.ipynb:168-185 (motor), :246-255 (rocket, "
                  "add_motor), :275-331 (nose, fins, tail, rail buttons), :340-347 (parachute)",
        "rocket": {
            "radius": 0.0655,
            "mass": 24.05,
            "inertia": [15.07, 15.07, 0.067],
            "center_of_mass_without_motor": 0,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": 0,
        },
        "motor_kind": "solid",
        "motor": {
            "dry_mass": 0.00000000001,
            "dry_inertia": [0.0000000000001, 0.0000000000001, 0.0000000000001],
            "center_of_dry_mass_position": -0.683,
            "nozzle_position": -1.294,
            "nozzle_radius": 0.0335,
            "throat_radius": 0.0114,
            "grain_number": 5,
            "grain_density": 1748.9,
            "grain_outer_radius": 0.0465,
            "grain_initial_inner_radius": 0.016,
            "grain_initial_height": 0.156,
            "grain_separation": 0.006,
            "grains_center_of_mass_position": -0.683,
            "coordinate_system_orientation": "nozzle_to_combustion_chamber",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
            "only_radial_burn": False,
        },
        "thrust_file": "curves/5f4294d20002e90000000741.eng",
        "example_thrust": {"file": "projeto-jupiter/mandioca_thrust_curve.csv",
                           "reshape_thrust_curve": [5.8, 8800]},
        "geometry": {
            "nose": {"length": 0.565, "kind": "vonKarman", "position": 1.477},
            "fin_sets": [{
                "type": "trapezoidal", "n": 4, "root_chord": 0.20, "tip_chord": 0.12,
                "span": 0.130, "position": -0.928, "cant_angle": 0,
                "airfoil": {"points": [[0, 0.0002], [2, 0.3320], [4, 0.6335], [6, 0.6877]],
                            "unit": "degrees"},
            }],
            "tails": [{"top_radius": 0.0655, "bottom_radius": 0.0535, "length": 0.068,
                       "position": -1.226}],
            "rail_buttons": {"upper_button_position": 0.24, "lower_button_position": -1.17,
                             "angular_position": 45},
            "parachutes": [
                {"name": "Drogue", "cd_s": 0.885, "trigger": APOGEE, "sampling_rate": 105,
                 "noise": NOISE, "lag": 0.5},
            ],
        },
    },
    {
        # docs/examples/cavour_flight_sim.ipynb: motor :157-172 (Cesaroni_3618L995-P.eng, burn_time
        # 3.8, zero dry mass and inertia, default "nozzle_to_combustion_chamber"), rocket :245-254
        # (drag curves "from RASAero II", :244), rail buttons :255, add_motor :257, nose and fins
        # :266-278 (no tail, no parachute). Substitute: Loki L1040LR (3707 N s against the L995's
        # 3618 N s).
        "name": "cavour",
        "source": "docs/examples/cavour_flight_sim.ipynb:157-172 (motor), :245-257 (rocket, rail "
                  "buttons, add_motor), :266-278 (nose, fins)",
        "rocket": {
            "radius": 0.052,
            "mass": 8.219,
            "inertia": [4.449, 4.449, 0.014634],
            "center_of_mass_without_motor": 1.1994,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": 0,
        },
        "motor_kind": "solid",
        "motor": {
            "dry_mass": 0,
            "dry_inertia": [0, 0, 0],
            "center_of_dry_mass_position": 1.1994,
            "nozzle_position": 0,
            "nozzle_radius": 0.0335,
            "throat_radius": 0.0114,
            "grain_number": 3,
            "grain_density": 1653.53,
            "grain_outer_radius": 0.0325,
            "grain_initial_inner_radius": 0.011375,
            "grain_initial_height": 0.13244,
            "grain_separation": 0.001,
            "grains_center_of_mass_position": 0.19966000000000006,
            "coordinate_system_orientation": "nozzle_to_combustion_chamber",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
            "only_radial_burn": False,
        },
        "thrust_file": "curves/5f4294d20002e90000000839.eng",
        "example_thrust": {"file": "cesaroni/Cesaroni_3618L995-P.eng", "burn_time": 3.8},
        "geometry": {
            "nose": {"length": 0.52, "kind": "vonKarman", "position": 2.7224},
            "fin_sets": [{
                "type": "trapezoidal", "n": 4, "span": 0.1, "root_chord": 0.2, "tip_chord": 0.07,
                "position": 0.2104,
            }],
            "tails": [],
            "rail_buttons": {"upper_button_position": 1.0954,
                             "lower_button_position": 0.005400000000000071},
            "parachutes": [],
        },
    },
    {
        # docs/examples/prometheus_2022_flight_sim.ipynb: GenericMotor :194-205
        # (Cesaroni_7579M1520-P.eng, burn_time 4.897; nozzle position, dry inertia, dry centre and
        # orientation left at their defaults), rocket :252-264, rail buttons :266, add_motor :268,
        # nose :269, fins :270-277 (no tail), parachutes :278-287 (sampling rate, lag and noise left
        # at their defaults). Substitute: Cesaroni 8187M1545-P (8182 N s against the M1520's
        # 7579 N s), shared with Juno III: no unused bundled curve is within 30%.
        "name": "prometheus-2022-generic-motor",
        "source": "docs/examples/prometheus_2022_flight_sim.ipynb:194-205 (motor), :252-268 "
                  "(rocket, rail buttons, add_motor), :269-277 (nose, fins), :278-287 (parachutes)",
        "rocket": {
            "radius": 0.06985,
            "mass": 13.93,
            "inertia": [4.87, 4.87, 0.05],
            "center_of_mass_without_motor": 0.9549,
            "coordinate_system_orientation": "tail_to_nose",
            "motor_position": 0,
        },
        "motor_kind": "generic",
        "motor": {
            "dry_mass": 2.981,
            "dry_inertia": [0, 0, 0],
            "center_of_dry_mass_position": None,
            "nozzle_position": 0,
            "nozzle_radius": 0.027,
            "propellant_initial_mass": 3.737,
            "chamber_radius": 0.064,
            "chamber_height": 0.548,
            "chamber_position": 0.274,
            "coordinate_system_orientation": "nozzle_to_combustion_chamber",
            "burn_time": None,
            "reshape_thrust_curve": False,
            "interpolation_method": "linear",
        },
        "thrust_file": "curves/5f4294d20002e90000000741.eng",
        "example_thrust": {"file": "cesaroni/Cesaroni_7579M1520-P.eng", "burn_time": 4.897},
        "geometry": {
            "nose": {"length": 0.742, "kind": "Von Karman", "position": 2.229},
            "fin_sets": [{"type": "trapezoidal", "n": 3, "root_chord": 0.268, "tip_chord": 0.136,
                          "span": 0.13, "position": 0.273, "sweep_length": 0.066}],
            "tails": [],
            "rail_buttons": {"upper_button_position": 0.69, "lower_button_position": 0.21,
                             "angular_position": 60},
            "parachutes": [
                {"name": "Drogue", "cd_s": 1.6 * math.pi * 0.3048**2, "trigger": APOGEE},
                {"name": "Main", "cd_s": 2.2 * math.pi * 0.9144**2, "trigger": below(457.2)},
            ],
        },
    },
]

GRID_POINTS = 100
AFTER_BURNOUT_S = [0.05, 0.5, 2.0]
KNOT_SAMPLES = 60

# Largest allowed gap, m^2, between Flight's squared CG offset and (center_of_mass - z_cdm)^2.
OFFSET_TOLERANCE_M2 = 1e-12


def fail(message):
    sys.exit(f"rocket_mass.py: {message}")


def finite(value, label):
    value = float(value)
    if not math.isfinite(value):
        fail(f"{label} is not finite ({value})")
    return value


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_substitute(case, catalog):
    """The bundled curve is an .eng file that the catalog marks public domain, byte for byte."""
    name, file = case["name"], case["thrust_file"]
    if not file.endswith(".eng"):
        fail(f"{name}: {file} is not an .eng file")
    entry = catalog.get(file)
    if entry is None:
        fail(f"{name}: {file} is not in the bundled catalog")
    if entry["license"] != "PD":
        fail(f"{name}: {file} is not public domain ({entry['license']})")
    if entry["sha256"] != sha256(CURVES / file):
        fail(f"{name}: {file} does not match the catalog's SHA-256")


def build_motor(case, example_curves):
    kwargs = copy.deepcopy(case["motor"])
    keys = SOLID_KEYS if case["motor_kind"] == "solid" else GENERIC_KEYS
    if list(kwargs) != keys:
        fail(f"{case['name']}: motor keys differ from the {case['motor_kind']} list")
    if example_curves:
        path = EXAMPLE_MOTORS / case["example_thrust"]["file"]
        kwargs.update({k: v for k, v in case["example_thrust"].items() if k != "file"})
    else:
        path = CURVES / case["thrust_file"]
    call = dict(kwargs, dry_inertia=tuple(kwargs["dry_inertia"]))
    if isinstance(call["reshape_thrust_curve"], list):
        call["reshape_thrust_curve"] = tuple(call["reshape_thrust_curve"])
    cls = SolidMotor if case["motor_kind"] == "solid" else GenericMotor
    return cls(thrust_source=str(path), **call), kwargs, path


def add_geometry(rocket, geometry):
    """Applies the recorded geometry with RocketPy's own methods, so a wrong keyword raises."""
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
        if "noise" in kwargs:
            kwargs["noise"] = tuple(kwargs["noise"])
        rocket.add_parachute(**kwargs)


def sample(name, rocket, motor, z_cdm, times):
    """The compared quantities at `times`, with the checks on I_22 and Flight's CG offset."""
    series = {"time_s": times}
    for key, f in [
        ("propellant_mass", motor.propellant_mass),
        ("total_mass", rocket.total_mass),
        ("center_of_mass", rocket.center_of_mass),
        ("I_11", rocket.I_11),
        ("I_33", rocket.I_33),
    ]:
        series[key] = [finite(f.get_value_opt(t), f"{name} {key}({t})") for t in times]

    about_cg = []
    rows = zip(times, series["total_mass"], series["center_of_mass"], series["I_11"])
    for t, m, z_cm, i_11 in rows:
        offset = finite(rocket.com_to_cdm_function.get_value_opt(t), f"{name} com_to_cdm({t})")
        if abs(offset * offset - (z_cm - z_cdm) ** 2) > OFFSET_TOLERANCE_M2:
            fail(f"{name} com_to_cdm_function({t}) differs from center_of_mass - z_cdm")
        about_cg.append(i_11 - m * (z_cm - z_cdm) ** 2)
    series["I_11_about_cg"] = about_cg

    if [float(rocket.I_22.get_value_opt(t)) for t in times] != series["I_11"]:
        fail(f"{name} I_22 differs from I_11")
    return series


def run(case, example_curves):
    name = case["name"]
    motor, motor_kwargs, thrust_path = build_motor(case, example_curves)

    inputs = case["rocket"]
    if list(inputs) != ROCKET_KEYS:
        fail(f"{name}: rocket keys differ from the list")
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

    t_out = float(motor.burn_out_time)
    times = [float(t) for t in np.linspace(0.0, t_out, GRID_POINTS)]
    times += [t_out + dt for dt in AFTER_BURNOUT_S]
    z_cdm = finite(rocket.center_of_dry_mass_position, f"{name} center_of_dry_mass_position")
    series = sample(name, rocket, motor, z_cdm, times)
    knot_series = None
    if case["motor_kind"] == "solid":
        knots = motor.grain_inner_radius.source[:, 0]
        picks = np.unique(np.linspace(0, len(knots) - 1, min(KNOT_SAMPLES, len(knots))).round())
        knot_series = sample(name, rocket, motor, z_cdm, [float(knots[int(i)]) for i in picks])
    if rocket.dry_I_22 != rocket.dry_I_11:
        fail(f"{name} dry_I_22 differs from dry_I_11")

    scalars = {
        "dry_mass_kg": rocket.dry_mass,
        "center_of_dry_mass_position_m": z_cdm,
        "dry_I_11_kg_m2": rocket.dry_I_11,
        "dry_I_33_kg_m2": rocket.dry_I_33,
        "motor_center_of_dry_mass_position_m": rocket.motor_center_of_dry_mass_position,
        "nozzle_position_m": rocket.nozzle_position,
        "propellant_initial_mass_kg": motor.propellant_initial_mass,
        "total_impulse_ns": motor.total_impulse,
        "burn_out_time_s": t_out,
    }
    for product in ["I_12", "I_13", "I_23"]:
        f = getattr(rocket, product)
        values = [finite(f.get_value_opt(t), f"{name} {product}({t})") for t in times]
        dry = finite(getattr(rocket, f"dry_{product}"), f"{name} dry_{product}")
        if any(values) or dry:
            fail(f"{name}: {product} is not zero; hpr's comparison assumes it is")
    scalars = {key: finite(value, f"{name} {key}") for key, value in scalars.items()}

    motor_record = {"motor_kind": case["motor_kind"], **motor_kwargs}
    motor_record["thrust_file"] = (
        str(thrust_path) if example_curves else case["thrust_file"]
    )
    motor_record["thrust_file_sha256"] = sha256(thrust_path)

    return {
        "name": name,
        "source": case["source"],
        "original_thrust_source": Path(case["example_thrust"]["file"]).name,
        "original_thrust_options": {k: v for k, v in case["example_thrust"].items() if k != "file"},
        "rocket": inputs,
        "motor": motor_record,
        "geometry": case["geometry"],
        "scalars": scalars,
        "series": series,
        "knot_series": knot_series,
    }


args = sys.argv[1:]
example_curves = "--example-curves" in args
wanted = {arg for arg in args if arg != "--example-curves"}
unknown = wanted - {case["name"] for case in CASES}
if unknown:
    fail(f"unknown cases {sorted(unknown)}")

catalog = {
    curve["file"]: curve
    for motor in json.loads((CURVES / "catalog.json").read_text())["motors"]
    for curve in motor["curves"]
}
for case in CASES:
    check_substitute(case, catalog)

document = json.dumps({
    "oracle": f"rocketpy {importlib.metadata.version('rocketpy')}",
    "generator": "validation/oracles/rocketpy/rocket_mass.py",
    "command": "refs/venv/bin/python validation/oracles/rocketpy/rocket_mass.py"
               + "".join(f" {arg}" for arg in args)
               + " > validation/fixtures/design/rocketpy-rocket-mass.json",
    "evaluation": "Function.get_value_opt",
    "thrust_curves": (
        "the examples' own (refs/rocketpy/data/motors; local cross-check, never committed)"
        if example_curves
        else "bundled public-domain substitutes (crates/hpr-motor/data/thrustcurve)"
    ),
    "cases": [run(case, example_curves) for case in CASES if not wanted or case["name"] in wanted],
}, indent=2)
# One line per numeric array: `repr` floats round-trip exactly, so this only removes whitespace.
print(re.sub(r"\[\s*([-0-9.eE+,\s]+?)\s*\]", lambda m: "[" + re.sub(r"\s+", " ", m.group(1)) + "]", document))
