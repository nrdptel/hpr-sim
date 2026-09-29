"""The builder's example rocket, built and flown from Python.

`crates/hpr/examples/build_and_fly.rs` builds this rocket with the Rust builder, and CI keeps
what it prints in `build_and_fly.output.txt`. These tests build the same rocket from Python and
print the same lines with the same formats: the bindings add no physics, so every digit agrees.
"""

import numpy as np
import pytest

import hpr


def example_rocket() -> hpr.Rocket:
    """The Rust example's rocket, part by part, with the same numbers."""
    motor = hpr.Motor.from_catalog("H54", delay_s=10.0)
    rocket = hpr.Rocket("My 54 mm rocket", 0.0563)
    rocket.add_nose(
        "ogive",
        0.22,
        "abs",
        wall_m=0.0015,
        shoulder_length_m=0.06,
        shoulder_wall_m=0.0015,
        capped_shoulder=True,
    )
    rocket.add_tube(0.9, 0.00115, "kraft_phenolic")
    rocket.add_motor_tube(0.2, 0.029, 0.001, "kraft_phenolic", overhang_m=0.005)
    rocket.add_fins(
        3,
        root_chord_m=0.1,
        tip_chord_m=0.04,
        span_m=0.045,
        sweep_m=0.05,
        thickness_m=0.003175,
        material="birch_plywood",
        cross_section="rounded",
    )
    rocket.add_mass(
        0.2,
        position="top",
        offset_m=0.07,
        packed_length_m=0.15,
        packed_diameter_m=0.05,
        name="recovery bay",
    )
    rocket.set_motor(motor)
    rocket.add_parachute("parachute", diameter_m=0.9, trigger="motor_delay", motor=0)
    return rocket


def example_flight(rocket: hpr.Rocket) -> hpr.Flight:
    environment = hpr.Environment(
        32.99, -106.97, 1400.0, wind_speed_m_s=5.0, wind_from_deg=270.0
    )
    return hpr.Flight(rocket, environment, 1.8, inclination_deg=85.0, heading_deg=270.0)


def printed(rocket: hpr.Rocket, flight: hpr.Flight) -> str:
    """The Rust example's output, line for line, from the Python objects."""
    motor = hpr.Motor.from_catalog("H54")
    end_s = motor.thrust_curve()[0][-1]
    liftoff, spent = rocket.margin(0.0, 0.3), rocket.margin(end_s, 0.3)
    full, empty = rocket.mass_properties(0.0), rocket.mass_properties(end_s)
    apogee = next(e for e in flight.events if e["kind"] == "apogee")
    landing = flight.landing
    lines = [
        f"{rocket.name} on a {rocket.configuration}",
        "Not yet validated: see the Accuracy page before trusting these numbers.",
        "",
        "                                 liftoff     spent",
        f"mass (kg)                        {full['mass_kg']:>7.3f} {empty['mass_kg']:>9.3f}",
        "centre of gravity (m from nose)  "
        f"{-full['cg_m'][2]:>7.3f} {-empty['cg_m'][2]:>9.3f}",
        "centre of pressure (m from nose) "
        f"{liftoff['cp_station_m']:>7.3f} {spent['cp_station_m']:>9.3f}",
        "stability margin (calibres)      "
        f"{liftoff['margin_cal']:>7.2f} {spent['margin_cal']:>9.2f}",
        "",
        "From a 1.8 m rail leaning 5° west, in 5 m/s of wind from the west:",
        f"Rail exit:  {flight.rail_exit_speed_m_s:.1f} m/s",
        f"Apogee:     {flight.apogee_m:.1f} m above the pad, at {flight.apogee_time_s:.2f} s, "
        f"{-apogee['sample']['cg_enu_m'][0]:.0f} m west of it",
        f"Top speed:  {flight.max_speed_m_s:.0f} m/s (Mach {flight.max_mach:.2f})",
        f"Landing:    {landing['east_m']:.0f} m east of the pad, at "
        f"{landing['descent_rate_m_s']:.1f} m/s, at {landing['time_s']:.1f} s",
    ]
    return "\n".join(lines) + "\n"


def test_prints_what_the_rust_example_prints(repo):
    rocket = example_rocket()
    expected = (repo / "crates/hpr/examples/build_and_fly.output.txt").read_text(
        encoding="utf-8"
    )
    assert printed(rocket, example_flight(rocket)) == expected


def test_a_flight_is_deterministic():
    rocket = example_rocket()
    first, second = example_flight(rocket), example_flight(rocket)
    assert first.to_json() == second.to_json()
    for name in first.columns:
        assert np.array_equal(first[name], second[name])


def test_the_recording_is_numpy_arrays():
    flight = example_flight(example_rocket())
    series = flight.series
    assert list(series) == flight.columns
    time_s = series["time_s"]
    assert isinstance(time_s, np.ndarray) and time_s.dtype == np.float64
    assert all(array.shape == time_s.shape for array in series.values())
    assert np.all(np.diff(time_s) > 0)
    # Its peak height is the apogee: the step ends bracket it, so the recorded peak is at most
    # the apogee, and within a metre of it.
    height_m = flight["height_above_ground_m"]
    assert flight.apogee_m - 1.0 < height_m.max() <= flight.apogee_m
    # The arrays are read-only copies of the flight's record.
    with pytest.raises(ValueError):
        height_m[0] = 0.0


def test_a_recording_at_an_interval():
    rocket = example_rocket()
    environment = hpr.Environment(32.99, -106.97, 1400.0)
    flight = hpr.Flight(rocket, environment, 1.8, interval_s=0.5)
    time_s = flight["time_s"]
    # A row at every multiple of 0.5 s within the flight, and one at every event.
    grid = np.arange(0.0, time_s[-1], 0.5)
    assert np.all(np.isin(grid, time_s))
    event_times = [event["time_s"] for event in flight.events]
    assert set(time_s) == set(grid) | set(event_times)


def test_events_come_in_order():
    flight = example_flight(example_rocket())
    kinds = [event["kind"] for event in flight.events]
    for first, then in [("liftoff", "rail_exit"), ("rail_exit", "burnout"), ("burnout", "apogee")]:
        assert kinds.index(first) < kinds.index(then)
    assert kinds[-1] == "ground_hit"
    times = [event["time_s"] for event in flight.events]
    assert times == sorted(times)
    deployment = next(e for e in flight.events if e["kind"] == "deployment")
    assert deployment["index"] == 0
