"""Motors and rockets read from files: the repository's public ones."""

import numpy as np
import pytest

import hpr

CURVES = "crates/hpr-motor/data/thrustcurve/curves"
CALISTO = "validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"


def test_a_motor_from_an_eng_file(repo):
    path = repo / CURVES / "5f4294d20002e90000000724.eng"
    motor = hpr.Motor.from_file(path, delay_s=7.0)
    same = hpr.Motor.from_eng(path.read_text(encoding="utf-8"), delay_s=7.0)
    assert motor.designation == same.designation
    assert motor.delay_s == 7.0
    times_s, thrusts_n = motor.thrust_curve()
    assert times_s.shape == thrusts_n.shape
    assert np.all(np.diff(times_s) > 0)
    # The curve's impulse, by the trapezoidal rule from the curve's first point at zero thrust.
    trapezoid = getattr(np, "trapezoid", None) or np.trapz  # NumPy 2 renamed it
    impulse_ns = trapezoid(np.concatenate(([0.0], thrusts_n)), np.concatenate(([0.0], times_s)))
    assert motor.total_impulse_ns == pytest.approx(impulse_ns, rel=1e-12)


def test_a_motor_from_an_rse_file(repo):
    path = repo / CURVES / "5f923edb1bca5800041716ab.rse"
    motor = hpr.Motor.from_file(path)
    assert motor.designation == hpr.Motor.from_rse(path.read_text(encoding="utf-8")).designation
    assert motor.delay_s is None
    assert motor.propellant_mass_kg > 0.0


def test_a_motor_file_of_another_kind_is_refused(repo):
    with pytest.raises(hpr.HprError, match="a motor file is"):
        hpr.Motor.from_file(repo / CALISTO)


def test_a_rocket_from_its_json(repo):
    rocket = hpr.Rocket.from_file(repo / CALISTO)
    assert rocket.configuration == "example"
    assert rocket.name.startswith("RocketPy example")
    environment = hpr.Environment(32.990254, -106.974998, 1400.0)
    flight = hpr.Flight(rocket, environment, 5.2, inclination_deg=85.0)
    assert flight.apogee_m > 2000.0
    assert flight.landing is not None


def test_a_design_read_back_flies_the_same(repo, tmp_path):
    rocket = hpr.Rocket.from_file(repo / CALISTO)
    copy = tmp_path / "calisto.json"
    copy.write_text(rocket.design_json(), encoding="utf-8")
    environment = hpr.Environment(32.990254, -106.974998, 1400.0)
    first = hpr.Flight(rocket, environment, 5.2)
    second = hpr.Flight(hpr.Rocket.from_file(copy, "example"), environment, 5.2)
    assert first.to_json() == second.to_json()


def test_an_hpr_document_of_an_older_version(repo):
    # A version 0.1 document, migrated as it is read, whose motor carries its own curve.
    rocket = hpr.Rocket.from_file(repo / "crates/hpr-format/fixtures/embedded-curve-0.1.hpr")
    flight = hpr.Flight(rocket, hpr.Environment(0.0, 0.0, 0.0), 1.0)
    assert flight.apogee_m > 0.0


def test_a_design_with_no_flyable_configuration_says_so(repo):
    with pytest.raises(hpr.HprError, match="no motor configuration hpr can fly"):
        hpr.Rocket.from_file(repo / "validation/fixtures/ork/rod-flights/rod-5-north.ork")


def test_a_configuration_that_isnt_there(repo):
    with pytest.raises(hpr.HprError, match="no configuration `nope`"):
        hpr.Rocket.from_file(repo / CALISTO, "nope")


def test_a_file_that_isnt_there(tmp_path):
    with pytest.raises(hpr.HprError, match="missing.hpr"):
        hpr.Rocket.from_file(tmp_path / "missing.hpr")
