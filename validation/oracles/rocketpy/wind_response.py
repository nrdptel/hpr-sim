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
5. **+ thin-plate fins** (Juno III only): without the airfoil lift curve Juno III's example gives
   its fins, which makes RocketPy's fin slope 7.6% steeper than hpr's thin-plate one.

After step 5 RocketPy flies hpr's choices on every point the two codes' normal force and rail
differ on but two small ones (hpr's `sin alpha` in place of `alpha`, and its drag factor at an
angle of attack), so its drifts should land on hpr's.

It also prints, for each case, the check that found the first correction: at the released
flight's rail exit, where the rotation rate is zero and so no damping acts, RocketPy's angular
acceleration against the moment about its own centre of mass (`Rocket.center_of_mass`) and about
the point mirrored across the centre of dry mass, each over the inertia there.

Run from the repo root:

    refs/venv/bin/python validation/oracles/rocketpy/wind_response.py [case ...]

with no names for every case, or with names (`juno-iii`, `juno-iii-calm`, ...) to pick some.
"""

import json
import math
import sys
import warnings

import numpy as np
from rocketpy import Flight, Function
from rocketpy.mathutils.vector_matrix import Vector
from rocketpy.rocket.aero_surface.aero_surface import AeroSurface

import corrections
import flight
import recovery

warnings.filterwarnings("ignore")

# Galejs's body-lift constant as hpr uses it (crates/hpr-aero/src/body.rs, `BODY_LIFT_K`).
BODY_LIFT_K = 1.1

# The whole-flight cases hpr flies; Prometheus 2022 is the checked M >= 1 gap and flies nothing.
SKIP = {"prometheus-2022-generic-motor"}


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
            continue
        planforms.append((component["id"], area, station + centroid))
        station += length
    return planforms


def add_body_lift(rocket, design):
    """Adds `BodyLift` for each body component, placed at its planform centroid."""
    nose, nose_position = next(
        (surface, position)
        for surface, position in rocket.aerodynamic_surfaces
        if type(surface).__name__ == "NoseCone"
    )
    surfaces, positions = [], []
    for name, area, station in body_planforms(design, nose):
        factor = BODY_LIFT_K * area / rocket.area
        surfaces.append(BodyLift(f"body lift, {name}", factor, rocket.radius))
        positions.append(nose_position.z - station * rocket._csys)
    rocket.add_surfaces(surfaces, positions)
    return [(surface.name, surface.lift_factor) for surface in surfaces]


def rail_exit_check(flown):
    """RocketPy's angular acceleration at the rail exit, against the moment about its own centre
    of mass and about the point mirrored across the centre of dry mass, over the inertia there."""
    t_exit = flown.out_of_rail_time
    row = next(row for row in flown.solution if row[0] >= t_exit)
    t, state = row[0], list(row[1:])
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
    return {
        "t": t,
        "flown": derivative[10],
        "about_centre_of_mass": about_true.x / inertia,
        "about_the_mirror_point": about_mirror.x / inertia,
    }


def fly(case, inputs, env, variant):
    """The case's metrics and apogee position under one variant."""
    inputs = json.loads(json.dumps(inputs))
    base = inputs["name"]
    if variant["thin_fins"]:
        for fins in inputs["geometry"]["fin_sets"]:
            fins.pop("airfoil", None)
    rocket, _ = flight.build_rocket(inputs, False)
    lifts = None
    if variant["body_lift"]:
        with open(f"validation/designs/rocketpy-{base}.json") as file:
            lifts = add_body_lift(rocket, json.load(file))
    rail = dict(flight.RAILS[base])
    if variant["release"]:
        buttons = inputs["geometry"]["rail_buttons"]
        rail["rail_length_m"] += abs(
            buttons["upper_button_position"] - buttons["lower_button_position"]
        )
    if variant["corrected"]:
        flown = flight.fly(rocket, env, rail, flight.SOLVER)
    else:
        # RocketPy as released: its own tensor and equations.
        rocket.evaluate_nozzle_gyration_tensor()
        flown = Flight(
            rocket=rocket,
            environment=env,
            rail_length=rail["rail_length_m"],
            inclination=rail["inclination_deg"],
            heading=rail["heading_deg"],
            **flight.SOLVER,
        )
    metrics = flight.metrics_of(flown, case, case["name"], flight.SOLVER)
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
        flight.fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    for case in cases:
        base = case.get("base", case["name"])
        inputs = recovery.mass_case(document, base)
        inputs["name"] = base
        env = recovery.environment_of(case)
        has_airfoil = any("airfoil" in fins for fins in inputs["geometry"]["fin_sets"])
        print(f"{case['name']}:")
        for variant in VARIANTS:
            if variant["thin_fins"] and not has_airfoil:
                continue
            flown, metrics, (east, north), lifts = fly(case, inputs, env, variant)
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
                print(
                    f"  {'':30s} rail exit {check['t']:.4f} s, angular acceleration about x: "
                    f"flown {check['flown']:+.4f} rad/s^2, moment about the centre of mass "
                    f"over I {check['about_centre_of_mass']:+.4f}, about the mirrored point "
                    f"{check['about_the_mirror_point']:+.4f}"
                )


if __name__ == "__main__":
    main()
