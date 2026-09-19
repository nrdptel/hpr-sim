"""Where issue #50's gap in wind came from, measured in RocketPy 1.13.0 (M2.1d3, ADR-026). A
measurement, not a fixture or a check.

In wind hpr's whole flights turned into the wind less than RocketPy's: Juno III's apogee drift was
60.8% short. This script flies each whole-flight case that hpr flies, cumulatively:

1. **released**: RocketPy 1.13.0 as released, which the references flew until M2.1d3.
2. **corrected**: with the two upstream corrections to its equations of motion
   (`corrections.py`), as the references now fly.
3. **+ release at the last button**: on a rail longer by the rail buttons' spacing, so RocketPy's
   first button leaves where hpr's last one does (hpr keeps the rocket guided until then;
   `rail_release.py`, ADR-025).
4. **+ body lift**: with hpr's body lift added to each body component as a RocketPy surface,
   `C_N = K (A_plan / A_ref) sin^2 alpha` at the planform centroid, `K = 1.1` (Galejs, as
   `hpr_aero::BODY_LIFT_K`; `docs/physics/aero.md`). RocketPy's own normal force is linear in
   alpha and has no body term. The planforms come from hpr's design (`validation/designs/`), with
   each nose's profile from RocketPy's own `NoseCone.y_nosecone`.
5. **+ thin-plate fins**, for a case whose fins carry an airfoil lift curve (Juno III's, in wind
   and calm): without it, as hpr flies them. The airfoil makes RocketPy's fin slope 7.6% steeper
   than hpr's thin-plate one.

After step 5 RocketPy flies hpr's choices on every point the two codes' normal force and rail
differ on but two small ones (hpr's `sin alpha` in place of `alpha`, and its drag factor at an
angle of attack), so its drifts should land on hpr's.

Three more measurements follow each case: RocketPy's fin slope at Mach 0.1 with and without the
airfoil, for a case whose fins carry one; the drifts of step 5 with body lift's `K` at 0 (none),
1.0 and 1.5, the ends of the range Galejs cites, to show how much a drift in wind rests on that
constant; and the drifts of step 5 with the nose's body lift alone, and with all of it placed at
the centre of mass at the rail exit, where it could only push the rocket sideways, not turn it.
The two show how body lift acts: mostly by turning the rocket, through the lift ahead of its
centre of mass.

hpr takes each body component's airflow at its small-angle centre of pressure and applies body
lift at its planform centroid; `BodyLift` takes the airflow at the centroid too. That, and the two
small differences above, are why step 5 lands within 1.4% of hpr rather than on it.

It also prints, for each case, the check that found the first correction: at the released
flight's rail exit, where the rotation rate is zero and so no damping acts, RocketPy's angular
acceleration against the moment about its own centre of mass (`Rocket.center_of_mass`) and about
the point mirrored across the centre of dry mass, each over the inertia there.

Run from the repo root:

    refs/venv/bin/python validation/oracles/rocketpy/wind_response.py [case ...]

with no names for every case, or with names (`juno-iii`, `juno-iii-calm`, ...) to pick some.
"""

import copy
import math
import json
import sys
import warnings

import numpy as np
from rocketpy import Flight, Function
from rocketpy.mathutils.vector_matrix import Vector
from rocketpy.rocket.aero_surface.aero_surface import AeroSurface

import flight
import recovery

warnings.filterwarnings("ignore")

# Galejs's body-lift constant as hpr uses it (crates/hpr-aero/src/body.rs, `BODY_LIFT_K`).
BODY_LIFT_K = 1.1

# Whole-flight cases hpr doesn't fly. Prometheus 2022 joined when hpr's normal force passed Mach 1
# (M1.8a).
SKIP = set()


class BodyLift(AeroSurface):
    """hpr's body lift on one body component: `C_N = f sin^2 alpha` on the rocket's reference
    area, acting at the position the surface is added at, with no slope at alpha = 0."""

    def __init__(self, name, lift_factor, rocket_radius):
        super().__init__(name, math.pi * rocket_radius**2, 2 * rocket_radius)
        # On the rocket's own radius, so `Rocket.evaluate_center_of_pressure` scales it by 1.
        self.rocket_radius = rocket_radius
        self.lift_factor = lift_factor
        self.evaluate_lift_coefficient()
        self.evaluate_center_of_pressure()

    def evaluate_lift_coefficient(self):
        self.clalpha = Function(lambda mach: 0.0, "Mach", "body lift slope at alpha = 0")
        self.cl = Function(
            lambda alpha, mach: self.lift_factor * math.sin(alpha) ** 2,
            ["Alpha (rad)", "Mach"],
            "Cl",
        )

    def evaluate_center_of_pressure(self):
        self.cpx, self.cpy, self.cpz = 0, 0, 0
        self.cp = (0, 0, 0)

    def evaluate_geometrical_parameters(self):
        pass

    def info(self):
        pass

    def all_info(self):
        pass


def body_planforms(design, nose):
    """(name, planform area m^2, centroid m aft of the nose tip) of each body component of hpr's
    design, in order: the nose from RocketPy's own profile, tubes and transitions exactly."""
    components = design["stages"][0]["components"]
    station = 0.0
    planforms = []
    for component in components:
        part = component["part"]
        if "nose_cone" in part:
            length = part["nose_cone"]["length_m"]
            x = np.linspace(0.0, length, 20001)
            radius = np.array([nose.y_nosecone(value) for value in x])
            area = 2.0 * np.trapezoid(radius, x)
            centroid = 2.0 * np.trapezoid(radius * x, x) / area
        elif "body_tube" in part:
            length = part["body_tube"]["length_m"]
            area = 2.0 * part["body_tube"]["outer_radius_m"] * length
            centroid = 0.5 * length
        elif "transition" in part:
            tail = part["transition"]
            length = tail["length_m"]
            fore, aft = tail["fore_radius_m"], tail["aft_radius_m"]
            area = (fore + aft) * length
            centroid = length * (fore + 2.0 * aft) / (3.0 * (fore + aft))
        else:
            fail(f"{component['id']} is a part this script has no planform for: {sorted(part)}")
        planforms.append((component["id"], area, station + centroid))
        station += length
    return planforms


def add_body_lift(rocket, design, k, only=None, at_station_m=None):
    """Adds `BodyLift` with constant `k` for each body component, at its planform centroid, or
    for component `only` alone, or with every component's placed at `at_station_m` from the nose
    tip instead."""
    nose, nose_position = next(
        (surface, position)
        for surface, position in rocket.aerodynamic_surfaces
        if type(surface).__name__ == "NoseCone"
    )
    surfaces, positions = [], []
    for name, area, station in body_planforms(design, nose):
        if only is not None and name != only:
            continue
        if at_station_m is not None:
            station = at_station_m
        factor = k * area / rocket.area
        surfaces.append(BodyLift(f"body lift, {name}", factor, rocket.radius))
        positions.append(nose_position.z - station * rocket._csys)
    rocket.add_surfaces(surfaces, positions)
    return [(surface.name, surface.lift_factor) for surface in surfaces]


def fail(message):
    sys.exit(f"wind_response.py: {message}")


def rail_exit_check(flown):
    """RocketPy's angular acceleration at the rail exit, about body x and y, against the moment
    about its own centre of mass and about the point mirrored across the centre of dry mass, over
    the inertia there. The rotation rate must be zero, so no damping term acts."""
    t_exit = flown.out_of_rail_time
    row = next(row for row in flown.solution if row[0] >= t_exit)
    t, state = row[0], list(row[1:])
    if any(state[10:13]):
        fail(f"the rotation rate at the rail exit is {state[10:13]}, not zero")
    variables = flown._Flight__post_processed_variables
    variables.clear()
    derivative = flown.u_dot_generalized(t, state, post_processing=True)
    r1, r2, r3, m1, m2, m3 = variables[-1][7:13]
    rocket = flown.rocket
    # `com_to_cdm_function` runs from the centre of mass to the dry one, so the centre of mass
    # is at minus it from the centre of dry mass (`rocket.py:996-1025`).
    true_r = -rocket.com_to_cdm_function.get_value_opt(t)
    mass = rocket.total_mass.get_value_opt(t)
    inertia = rocket.get_inertia_tensor_at_time(t)[0][0] - mass * true_r**2
    force = Vector([r1, r2, r3])
    moment = Vector([m1, m2, m3])
    about_true = moment - (Vector([0, 0, true_r]) ^ force)
    about_mirror = moment + (Vector([0, 0, true_r]) ^ force)
    # The rocket is axisymmetric, so the inertia about body x and y is the same.
    return {
        "t": t,
        "flown": (derivative[10], derivative[11]),
        "about_centre_of_mass": (about_true.x / inertia, about_true.y / inertia),
        "about_the_mirror_point": (about_mirror.x / inertia, about_mirror.y / inertia),
    }


def fly(case, inputs, env, variant):
    """The case's metrics and apogee position under one variant."""
    inputs = copy.deepcopy(inputs)
    base = inputs["name"]
    if variant["thin_fins"]:
        for fins in inputs["geometry"]["fin_sets"]:
            fins.pop("airfoil", None)
    rocket, _ = flight.build_rocket(inputs, False)
    lifts = None
    if variant["body_lift"]:
        with open(f"validation/designs/rocketpy-{base}.json") as file:
            lifts = add_body_lift(
                rocket,
                json.load(file),
                variant.get("k", BODY_LIFT_K),
                variant.get("only"),
                variant.get("at_station_m"),
            )
    rail = dict(flight.RAILS[base])
    if variant["release"]:
        buttons = inputs["geometry"].get("rail_buttons") or fail(f"{base} has no rail buttons")
        rail["rail_length_m"] += abs(
            buttons["upper_button_position"] - buttons["lower_button_position"]
        )
    if variant["corrected"]:
        flown = flight.fly(rocket, env, rail, flight.SOLVER)
    else:
        # RocketPy as released: its own tensor and equations.
        rocket.evaluate_nozzle_gyration_tensor()
        flown = flight.fly(rocket, env, rail, flight.SOLVER, flight_class=Flight)
    metrics = flight.metrics_of(flown, case, case["name"], flight.SOLVER)
    metrics["fin_slope_at_mach_0_1"] = sum(
        surface.clalpha(0.1)
        for surface, _ in rocket.aerodynamic_surfaces
        if type(surface).__name__ == "TrapezoidalFins"
    )
    east = flown.apogee_x - flown.x(0)
    north = flown.apogee_y - flown.y(0)
    return flown, metrics, (east, north), lifts


VARIANTS = [
    {"label": "released", "corrected": False, "release": False, "body_lift": False,
     "thin_fins": False},
    {"label": "corrected", "corrected": True, "release": False, "body_lift": False,
     "thin_fins": False},
    {"label": "+ release at the last button", "corrected": True, "release": True,
     "body_lift": False, "thin_fins": False},
    {"label": "+ body lift", "corrected": True, "release": True, "body_lift": True,
     "thin_fins": False},
    {"label": "+ thin-plate fins", "corrected": True, "release": True, "body_lift": True,
     "thin_fins": True},
]


def main():
    keep = set(sys.argv[1:])
    document = recovery.mass_fixture()
    every = [
        case for case in recovery.CASES + flight.WHOLE_FLIGHT_ONLY_CASES
        if case["name"] not in SKIP
    ]
    every = every + flight.calm_air_cases(every)
    cases = [case for case in every if not keep or case["name"] in keep]
    if keep and len(cases) != len(keep):
        fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    for case in cases:
        base = case.get("base", case["name"])
        inputs = recovery.mass_case(document, base)
        inputs["name"] = base
        env = recovery.environment_of(case)
        has_airfoil = any("airfoil" in fins for fins in inputs["geometry"]["fin_sets"])
        print(f"{case['name']}:")
        slopes = {}
        for variant in VARIANTS:
            if variant["thin_fins"] and not has_airfoil:
                continue
            flown, metrics, (east, north), lifts = fly(case, inputs, env, variant)
            slopes[variant["thin_fins"]] = metrics["fin_slope_at_mach_0_1"]
            print(
                f"  {variant['label']:30s} apogee_agl_m {metrics['apogee_agl_m']:7.1f}  "
                f"apogee_drift_m {metrics['apogee_drift_m']:7.1f}  "
                f"landing_drift_m {metrics['landing_drift_m']:7.1f}  "
                f"apogee at ({east:7.1f} E, {north:7.1f} N) m"
            )
            if lifts:
                described = ", ".join(f"{name} {factor:.3f}" for name, factor in lifts)
                print(f"  {'':30s} body-lift factors K A_plan/A_ref: {described}")
            if variant["label"] == "released":
                check = rail_exit_check(flown)
                pair = lambda values: "({:+.4f}, {:+.4f})".format(*values)
                print(
                    f"  {'':30s} rail exit {check['t']:.4f} s, angular acceleration about body "
                    f"(x, y): flown {pair(check['flown'])} rad/s^2, moment about the centre of "
                    f"mass over I {pair(check['about_centre_of_mass'])}, about the mirrored point "
                    f"{pair(check['about_the_mirror_point'])}"
                )
        if has_airfoil:
            print(
                f"  fin slope at Mach 0.1, per radian: {slopes[False]:.4f} with the airfoil, "
                f"{slopes[True]:.4f} thin-plate, a ratio of {slopes[False] / slopes[True]:.4f}"
            )
        last = dict(VARIANTS[-1] if has_airfoil else VARIANTS[-2])
        flown, _, _, _ = fly(case, inputs, env, last)
        rocket = flown.rocket
        nose_z = next(
            position.z
            for surface, position in rocket.aerodynamic_surfaces
            if type(surface).__name__ == "NoseCone"
        )
        t_exit = flown.out_of_rail_time
        centre_m = (nose_z - rocket.center_of_mass.get_value_opt(t_exit)) * rocket._csys
        _, nose_only, _, _ = fly(case, inputs, env, dict(last, only="nose"))
        _, at_centre, _, _ = fly(case, inputs, env, dict(last, at_station_m=centre_m))
        print(
            f"  body lift placed, apogee / landing drift in m: the nose's alone "
            f"{nose_only['apogee_drift_m']:.1f} / {nose_only['landing_drift_m']:.1f}; all at the "
            f"centre of mass at the rail exit, {centre_m:.3f} m from the nose tip, "
            f"{at_centre['apogee_drift_m']:.1f} / {at_centre['landing_drift_m']:.1f}"
        )
        swept = []
        for k in (0.0, 1.0, 1.5):
            _, metrics, _, _ = fly(case, inputs, env, dict(last, k=k))
            swept.append(
                f"K {k:.1f}: {metrics['apogee_drift_m']:.1f} / {metrics['landing_drift_m']:.1f}"
            )
        print(f"  body-lift K swept, apogee / landing drift in m: {'; '.join(swept)}")


if __name__ == "__main__":
    main()
