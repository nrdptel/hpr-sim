# %% [markdown]
# # RocketPy's Calisto, flown from Python
#
# RocketPy's example rocket, Calisto, flown by hpr from Python the way the validation suite flies
# it, then held to RocketPy's own numbers for the same flight. The suite ran RocketPy 1.13.0, with
# two fixes from RocketPy's own repository applied
# (https://nrdptel.github.io/hpr-sim/accuracy.html), and committed what it printed, so no
# RocketPy is needed here.
#
# Both codes fly the same drag: a zero-lift drag coefficient of 0.5 at every Mach number, power on
# and off. The rest (the rocket, its motor, the rail, the site, the wind and the two parachutes)
# comes from the repository's files. What differs is each code's physics: its equations of motion,
# atmosphere, rail and parachutes. So this is a code-to-code comparison, not a check against a
# real flight.
#
# Run it from the repository's root: `python crates/hpr-py/examples/calisto.py`. It needs the `hpr`
# package and NumPy. Each `# %%` line starts a cell, for an editor that runs cells as a notebook.

# %%
import json
import math

import numpy as np

import hpr

# The suite's reference: RocketPy's flight of Calisto in a wind, with its inputs and its metrics.
CASE = "calisto-tests-motor-at-minus-1.373"
with open("validation/fixtures/flight/rocketpy-whole-flight.json", encoding="utf-8") as file:
    reference = next(case for case in json.load(file)["cases"] if case["name"] == CASE)

# %% [markdown]
# ## The flight
#
# The design file holds the rocket and its motor, with RocketPy's masses, inertias and thrust
# curve. The drag table is the one both codes fly, on RocketPy's reference radius. RocketPy flies
# one parachute at a time, the last to open, so the drogue is cut away when the main is fully open.
# RocketPy's wind is given as the way it blows (east `u`, north `v`); hpr's as where it blows
# from. Gravity is RocketPy's formula.

# %%
calisto = hpr.Rocket.from_file(reference["design"], configuration="example")

environment = reference["environment"]
u, v = environment["wind_u"], environment["wind_v"]
site = hpr.Environment(
    environment["latitude_deg"],
    environment["longitude_deg"],
    environment["elevation_m"],
    wind_speed_m_s=math.hypot(u, v),
    wind_from_deg=math.degrees(math.atan2(-u, -v)) % 360.0,
    gravity="vertical_taylor",
)

drag = reference["drag"]
table = hpr.DragTable(
    drag["cd0_vs_mach"],
    drag["cd0_vs_mach"],
    reference_diameter_m=2.0 * drag["reference_radius_m"],
)

# %% [markdown]
# RocketPy follows the rocket's **dry** centre of mass, the one without propellant, and starts it
# at the ground. hpr stands the rocket on its rail with its aft end at the rail's foot, so that
# point starts `h0` above the ground: 1.25 m here, the first line the example prints. Heights are measured from `h0`, and the main
# opens `h0` higher than RocketPy's 800 m, so that both open at the same place. A first flight,
# before the parachutes are added, shows where the rocket stands on the rail.

# %%
DRY_CG_M = np.array(calisto.mass_properties(1.0e6)["cg_m"])  # long after burnout


def body_to_launch(attitude, vector):
    """Turns a body-frame vector into the launch frame, by each row's attitude quaternion."""
    w = attitude[:, :1]
    axis = attitude[:, 1:]
    twice = 2.0 * np.cross(axis, vector)
    return vector + w * twice + np.cross(axis, twice)


rail = reference["rail"]
standing = hpr.Flight(
    calisto,
    site,
    rail["rail_length_m"],
    inclination_deg=rail["inclination_deg"],
    heading_deg=rail["heading_deg"],
    drag_table=table,
)


def columns(flight, *names):
    return np.column_stack([flight[name] for name in names])


def dry_cg(flight):
    """The dry centre of mass's position, m, on every row of the flight's recording."""
    nose = columns(flight, "position_east_m", "position_north_m", "position_up_m")
    attitude = columns(flight, "attitude_w", "attitude_x", "attitude_y", "attitude_z")
    return nose + body_to_launch(attitude, DRY_CG_M)


h0 = dry_cg(standing)[0, 2]
drogue, main = reference["devices"]
calisto.add_parachute(
    "drogue", cd_s_m2=drogue["cd_s_m2"], lag_s=drogue["lag_s"], released_by=1
)
calisto.add_parachute(
    "main",
    cd_s_m2=main["cd_s_m2"],
    trigger="altitude",
    altitude_m=main["trigger"]["height_m"] + h0,
    lag_s=main["lag_s"],
)
flight = hpr.Flight(
    calisto,
    site,
    rail["rail_length_m"],
    inclination_deg=rail["inclination_deg"],
    heading_deg=rail["heading_deg"],
    drag_table=table,
)
print(f"The dry centre of mass starts {h0:.3f} m above the ground.")

# %% [markdown]
# ## RocketPy's metrics, measured on hpr's flight
#
# The recording's `position_*` and `velocity_*` columns are the nose tip's, in the launch frame
# (east, north, up), and `attitude_*` is the quaternion that turns the rocket's own axes into it
# (https://nrdptel.github.io/hpr-sim/physics/frames.html). Each metric is measured as RocketPy
# defines it:
#
# - **Rail exit** is when RocketPy's forward rail button leaves the rail. The rocket has then
#   travelled `effective_1rl`, RocketPy's name for that distance: 3.745 m here, from the suite's
#   reference file. hpr's own rail exit comes later, when its aft-most rail guide clears the top of
#   the 5.2 m rail, so RocketPy's is read off the recording.
# - **Heights** are the dry centre of mass's, above `h0`. At burnout and after, the rocket's
#   centre of mass is the dry one, so hpr's own heights are that point's.
# - **Speeds** are the dry centre of mass's: the nose tip's velocity plus the turn about it.
# - **Accelerations** are hpr's own, which are the nose tip's. The largest comes on the rail,
#   where the rocket doesn't turn and every point of it accelerates alike; the example checks that.
# - **Landing** is when the dry centre of mass is back at `h0`, where RocketPy's flight ends.
#   **Drift** is the horizontal distance from where that point started.

# %%
time = flight["time_s"]
point = dry_cg(flight)
attitude = columns(flight, "attitude_w", "attitude_x", "attitude_y", "attitude_z")
rates = columns(flight, "body_rate_x_rad_s", "body_rate_y_rad_s", "body_rate_z_rad_s")
nose_velocity = columns(flight, "velocity_east_m_s", "velocity_north_m_s", "velocity_up_m_s")
speed = np.linalg.norm(
    nose_velocity + body_to_launch(attitude, np.cross(rates, DRY_CG_M)), axis=1
)
acceleration = columns(
    flight, "acceleration_east_m_s2", "acceleration_north_m_s2", "acceleration_up_m_s2"
)
events = {event["kind"]: event for event in flight.events}


def crossing(values, what, last=False):
    """The index of the row before `values` goes from above zero to zero or below: the first such
    row, or the last."""
    found = np.flatnonzero((values[:-1] > 0.0) & (values[1:] <= 0.0))
    if len(found) == 0:
        raise RuntimeError(f"the flight never reached {what}")
    return found[-1] if last else found[0]


# Rail exit: the travel along the rail reaches effective_1rl. The step of the integrator that
# crosses it is filled in with a cubic for the speed, matching the speed and the acceleration
# along the rail at both ends, and the travel is that cubic's integral.
effective_1rl_m = rail["effective_1rl_m"]
travel = np.linalg.norm(point - point[0], axis=1)
step = crossing(effective_1rl_m - travel, "effective_1rl")
along = (point[step + 1] - point[0]) / travel[step + 1]
t0, width = time[step], time[step + 1] - time[step]
v0, v1 = speed[step], speed[step + 1]
a0, a1 = acceleration[step] @ along * width, acceleration[step + 1] @ along * width


def speed_at(x):
    return (
        (2 * x**3 - 3 * x**2 + 1) * v0
        + (x**3 - 2 * x**2 + x) * a0
        + (-2 * x**3 + 3 * x**2) * v1
        + (x**3 - x**2) * a1
    )


def travel_at(x):
    return travel[step] + width * (
        (x**4 / 2 - x**3 + x) * v0
        + (x**4 / 4 - 2 * x**3 / 3 + x**2 / 2) * a0
        + (-(x**4) / 2 + x**3) * v1
        + (x**4 / 4 - x**3 / 3) * a1
    )


low, high = 0.0, 1.0
for _ in range(60):
    middle = 0.5 * (low + high)
    low, high = (middle, high) if travel_at(middle) < effective_1rl_m else (low, middle)
rail_exit_time_s = t0 + low * width
rail_exit_speed_m_s = speed_at(low)

# Landing: the last time the dry centre of mass comes down through h0, by its height above the
# ground (hpr's `height_above_ground_m`, which after burnout is that point's).
last = crossing(flight["height_above_ground_m"] - h0, "h0 on the way down", last=True)
height = flight["height_above_ground_m"][last : last + 2] - h0
share = height[0] / (height[0] - height[1])


def at_landing(values):
    return values[last] + share * (values[last + 1] - values[last])


def drift(where):
    return float(np.linalg.norm(where[:2] - point[0, :2]))


apogee = events["apogee"]["sample"]
burnout = events["burnout"]["sample"]
summary = flight.summary
boost = summary["max_acceleration_m_s2"]  # liftoff until the drogue opens
# The largest acceleration is the boost's, on the rail, so it is also the power-on one.
if not (
    boost["time_s"] <= events["rail_exit"]["time_s"] <= burnout["time_s"]
    and boost["value"] >= summary["max_descent_acceleration_m_s2"]["value"]
):
    raise RuntimeError("the largest acceleration isn't on the rail: measure it at the dry CG")
landing_time_s = at_landing(time)
metrics = {
    "apogee_agl_m": apogee["height_above_ground_m"] - h0,
    "apogee_time_s": apogee["time_s"],
    "max_speed_m_s": speed[time <= landing_time_s].max(),
    "max_mach": summary["max_mach"]["value"],
    "max_acceleration_m_s2": boost["value"],
    "max_acceleration_power_on_m_s2": boost["value"],
    "rail_exit_speed_m_s": rail_exit_speed_m_s,
    "rail_exit_time_s": rail_exit_time_s,
    "burnout_altitude_agl_m": burnout["height_above_ground_m"] - h0,
    "burnout_speed_m_s": float(np.linalg.norm(burnout["cg_velocity_enu_m_s"])),
    "flight_time_s": landing_time_s,
    "apogee_drift_m": drift(np.array(apogee["cg_enu_m"])),
    "landing_drift_m": drift(at_landing(point)),
    "impact_speed_m_s": -at_landing(flight["vertical_speed_m_s"]),
}

# %% [markdown]
# ## Against RocketPy
#
# The validation suite holds each of these to 3% of RocketPy's value, the bound it set for
# comparisons with RocketPy when it was built (milestone M2.1,
# https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1).

# %%
TOLERANCE = 0.03
rocketpy = reference["metrics"]
print(f"{'metric':<32}{'hpr':>11}{'RocketPy':>11}{'difference':>12}")
worst = 0.0
for name, ours in metrics.items():
    theirs = rocketpy[name]
    difference = (ours - theirs) / theirs
    # A number that isn't one fails, rather than slipping past `max`.
    worst = max(worst, abs(difference) if math.isfinite(difference) else math.inf)
    digits = 2 if abs(theirs) >= 10.0 else 4
    print(f"{name:<32}{ours:>11.{digits}f}{theirs:>11.{digits}f}{difference:>+12.2%}")
print(f"largest difference {worst:.2%}, within {TOLERANCE:.0%}: {worst <= TOLERANCE}")
assert worst <= TOLERANCE
