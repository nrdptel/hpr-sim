"""What the bindings refuse, and how: `hpr.HprError`, a `ValueError`, with the library's words."""

import pytest

import hpr


def rocket_with_nose() -> hpr.Rocket:
    rocket = hpr.Rocket("R", 0.0563)
    rocket.add_nose("ogive", 0.22, "abs", wall_m=0.0015)
    return rocket


def test_the_error_is_a_value_error():
    assert issubclass(hpr.HprError, ValueError)
    assert hpr.HprError.__module__ == "hpr"


@pytest.mark.parametrize(
    ("call", "message"),
    [
        (lambda: hpr.Rocket("R", -1.0), "rocket diameter, m is -1"),
        (lambda: hpr.Environment(91.0, 0.0, 0.0), "latitude"),
        (lambda: hpr.Motor.from_catalog("Z9000"), "no motor with a bundled thrust curve"),
        (lambda: hpr.Motor.from_catalog("H54", delay_s=-1.0), "delay"),
        (lambda: rocket_with_nose().add_tube(0.9, 0.001, "unobtainium"), "no built-in material"),
        (lambda: rocket_with_nose().add_nose("ogive", 0.2, "abs"), "nose"),
        (lambda: hpr.Rocket("R", 0.05).add_nose("pointy", 0.2, "abs"), "no nose shape `pointy`"),
        (lambda: hpr.Rocket("R", 0.05).add_nose("conical", 0.2, "abs", parameter=0.5), "takes no parameter"),
        (lambda: hpr.Rocket("R", 0.05).add_nose("power_series", 0.2, "abs"), "needs its parameter"),
        (
            lambda: hpr.Rocket("R", 0.05).add_nose("ogive", 0.2, "abs", shoulder_length_m=0.05),
            "both shoulder_length_m and shoulder_wall_m",
        ),
        (
            lambda: rocket_with_nose()
            .add_tube(0.9, 0.001, "abs")
            .add_fins(
                3,
                root_chord_m=0.1,
                tip_chord_m=0.04,
                span_m=0.045,
                sweep_m=0.05,
                thickness_m=0.003,
                material="abs",
                cross_section="wedge",
            ),
            "no fin cross-section `wedge`",
        ),
        (
            lambda: rocket_with_nose().add_tube(0.9, 0.001, "abs").add_mass(0.1, position="side"),
            "no position `side`",
        ),
        (
            lambda: rocket_with_nose().add_tube(0.9, 0.001, "abs").add_mass(0.1, packed_length_m=0.1),
            "both packed_length_m and packed_diameter_m",
        ),
        (lambda: rocket_with_nose().add_parachute("p"), "takes diameter_m"),
        (lambda: rocket_with_nose().add_parachute("p", diameter_m=1.0, cd_s_m2=1.0), "takes diameter_m"),
        (lambda: rocket_with_nose().add_parachute("p", cd_s_m2=1.0, trigger="altitude"), "trigger `altitude`"),
        (lambda: rocket_with_nose().add_parachute("p", cd_s_m2=1.0, canopy="kite", diameter_m=1.0), "takes diameter_m"),
        (lambda: rocket_with_nose().add_parachute("p", diameter_m=1.0, canopy="kite"), "no canopy type `kite`"),
        (lambda: rocket_with_nose().add_parachute("p", cd_s_m2=1.0, canopy="conical"), "cd_s_m2 alone"),
        (lambda: rocket_with_nose().add_parachute("p", cd_s_m2=1.0, motor=1), "trigger `apogee`"),
        (
            lambda: rocket_with_nose().add_parachute("p", cd_s_m2=1.0, trigger="time", time_s=5.0, motor=0),
            "trigger `time`",
        ),
    ],
)
def test_refusals(call, message):
    with pytest.raises(hpr.HprError, match=message):
        call()


def test_fins_take_their_lengths_by_name():
    rocket = rocket_with_nose().add_tube(0.9, 0.001, "abs")
    with pytest.raises(TypeError):
        rocket.add_fins(3, 0.1, 0.04, 0.045, 0.05, 0.003, "abs")


def test_a_rocket_with_no_motor_does_not_fly():
    rocket = rocket_with_nose().add_tube(0.9, 0.001, "abs")
    with pytest.raises(hpr.HprError, match="no motor"):
        hpr.Flight(rocket, hpr.Environment(0.0, 0.0, 0.0), 1.0)


@pytest.mark.parametrize("interval_s", [0.0, 1e-300, 0.000999, float("nan"), -1.0])
def test_a_recording_interval_under_a_millisecond_is_refused(interval_s):
    rocket = rocket_with_nose().add_tube(0.9, 0.001, "abs")
    with pytest.raises(hpr.HprError, match="at least 0.001 s"):
        hpr.Flight(rocket, hpr.Environment(0.0, 0.0, 0.0), 1.0, interval_s=interval_s)


def test_a_missing_column(repo):
    rocket = hpr.Rocket.from_file(
        repo / "validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
    )
    flight = hpr.Flight(rocket, hpr.Environment(0.0, 0.0, 0.0), 5.2, interval_s=1.0)
    with pytest.raises(KeyError, match="no column `altitude`"):
        flight["altitude"]


def test_the_materials_are_listed():
    names = hpr.materials()
    assert {"abs", "kraft_phenolic", "birch_plywood"} <= set(names)
    assert len(names) == len(set(names))
