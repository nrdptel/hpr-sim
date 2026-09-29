"""A drag table and a gravity model flown from Python, and parachutes released by another."""

import numpy as np
import pytest

import hpr


def rocket() -> hpr.Rocket:
    rocket = hpr.Rocket("R", 0.0563)
    rocket.add_nose("ogive", 0.22, "abs", wall_m=0.0015)
    rocket.add_tube(0.9, 0.00115, "kraft_phenolic")
    rocket.add_motor_tube(0.2, 0.029, 0.001, "kraft_phenolic")
    rocket.add_fins(
        3,
        root_chord_m=0.1,
        tip_chord_m=0.04,
        span_m=0.045,
        sweep_m=0.05,
        thickness_m=0.003175,
        material="birch_plywood",
    )
    rocket.add_mass(0.2, position="top", offset_m=0.07)
    rocket.set_motor(hpr.Motor.from_catalog("H54"))
    return rocket


SITE = hpr.Environment(32.99, -106.97, 1400.0)


def apogee(**options) -> float:
    return hpr.Flight(rocket(), options.pop("site", SITE), 1.8, **options).apogee_m


def test_a_table_reads_its_curves():
    table = hpr.DragTable([(0.0, 0.5), (1.0, 0.7)], np.array([[0.0, 0.4], [1.0, 0.6]]))
    assert table.has_power_on
    assert table.reference_diameter_m is None
    assert table.cd0(0.5) == pytest.approx(0.6)
    assert table.cd0(0.5, thrusting=True) == pytest.approx(0.5)
    # Held at the ends.
    assert table.cd0(3.0) == 0.7
    assert not hpr.DragTable([(0.0, 0.5), (1.0, 0.5)]).has_power_on
    assert "2 power-off rows, 2 power-on rows" in repr(table)


def test_a_table_reads_csv_files(tmp_path):
    off = tmp_path / "off.csv"
    off.write_text("mach,cd\n0,0.45\n2,0.65\n", encoding="utf-8")
    table = hpr.DragTable.from_csv(off, reference_diameter_m=0.1)
    assert table.cd0(1.0) == pytest.approx(0.55)
    assert table.reference_diameter_m == 0.1
    assert not table.has_power_on
    assert repr(table) == "DragTable(2 power-off rows, reference_diameter_m=0.1)"
    on = tmp_path / "on.csv"
    on.write_text("0,0.40\n2,0.60\n", encoding="utf-8")
    both = hpr.DragTable.from_csv(off, on)
    assert both.has_power_on
    assert both.cd0(1.0, thrusting=True) == pytest.approx(0.5)
    # An error names the file it is in.
    bad = tmp_path / "bad.csv"
    bad.write_text("0,0.4\n0,0.5\n", encoding="utf-8")
    with pytest.raises(hpr.HprError, match="bad.csv: "):
        hpr.DragTable.from_csv(off, bad)
    pushing = tmp_path / "pushing.csv"
    pushing.write_text("0,0.4\n1,-0.1\n", encoding="utf-8")
    with pytest.raises(hpr.HprError, match="pushing.csv's row 1 has a drag coefficient of -0.1"):
        hpr.DragTable.from_csv(pushing)


def test_the_table_is_flown():
    own = apogee()
    half = [(0.0, 0.5), (1.0, 0.5)]
    tabled = apogee(drag_table=hpr.DragTable(half))
    assert tabled != own
    # Less drag while the motor burns: higher than power off's curve alone.
    assert apogee(drag_table=hpr.DragTable(half, [(0.0, 0.2), (1.0, 0.2)])) > tabled
    # A quarter of the coefficient on twice the diameter is the same drag.
    wide = hpr.DragTable([(0.0, 0.125), (1.0, 0.125)], reference_diameter_m=2 * 0.0563)
    assert apogee(drag_table=wide) == pytest.approx(tabled, rel=1e-9)


def test_a_gravity_model_is_flown():
    own = apogee()
    # Each model a different flight, by about 1e-8 near the ground.
    flown = {}
    for name in ("vertical_taylor", "Vertical Taylor", "vertical"):
        site = hpr.Environment(32.99, -106.97, 1400.0, gravity=name)
        flown[name] = apogee(site=site)
        assert flown[name] != own
        assert flown[name] == pytest.approx(own, rel=1e-6)
    assert flown["vertical_taylor"] == flown["Vertical Taylor"]
    assert flown["vertical"] != flown["vertical_taylor"]
    assert apogee(site=hpr.Environment(32.99, -106.97, 1400.0, gravity="ellipsoidal")) == own


def test_a_parachute_is_released_by_another():
    def landing(released_by):
        chuted = rocket()
        chuted.add_parachute("drogue", cd_s_m2=0.2, released_by=released_by)
        chuted.add_parachute("main", cd_s_m2=1.0, trigger="altitude", altitude_m=150.0)
        flight = hpr.Flight(chuted, SITE, 1.8)
        kinds = [event["kind"] for event in flight.events]
        return flight.landing["descent_rate_m_s"], kinds

    both, kinds = landing(None)
    alone, released = landing(1)
    assert "release" not in kinds
    assert "release" in released
    # Under the main alone it falls faster than under both.
    assert alone > both


@pytest.mark.parametrize(
    ("call", "message"),
    [
        (lambda: hpr.DragTable([(0.0, 0.5, 1.0), (1.0, 0.5)]), "power_off's row 0 has 3 values"),
        (lambda: hpr.DragTable([(0.0, 0.5)]), "power_off"),
        (lambda: hpr.DragTable([(1.0, 0.5), (0.0, 0.5)]), "power_off"),
        (lambda: hpr.DragTable([(0.0, 0.5), (1.0, 0.5)], [(0.0, 0.5)]), "power_on"),
        (
            lambda: hpr.DragTable([(0.0, 0.5), (1.0, -0.01)]),
            "power_off's row 1 has a drag coefficient of -0.01, and it can't be negative",
        ),
        (
            lambda: hpr.DragTable([(0.0, 0.5), (1.0, 0.5)], [(0.0, -1.0), (1.0, 0.5)]),
            "power_on's row 0",
        ),
        (
            lambda: hpr.DragTable([(0.0, 0.5), (1.0, 0.5)], reference_diameter_m=0.0),
            "reference diameter is 0 m",
        ),
        (lambda: hpr.DragTable.from_csv("no/such/file.csv"), "no/such/file.csv"),
        (lambda: hpr.DragTable([(0.0, 0.5), (1.0, 0.5)]).cd0(-1.0), "Mach"),
        (lambda: hpr.Environment(0.0, 0.0, 0.0, gravity="constant"), "no gravity `constant`"),
        (lambda: hpr.Environment(0.0, 0.0, 0.0, gravity="flat"), "no gravity `flat`"),
    ],
)
def test_refusals(call, message):
    with pytest.raises(hpr.HprError, match=message):
        call()


@pytest.mark.parametrize(
    ("released_by", "message"),
    [
        (5, "index of the device that releases this one is outside its domain: 5"),
        (0, "releases run in a cycle"),
    ],
)
def test_a_release_by_no_other_parachute_is_refused(released_by, message):
    chuted = rocket()
    chuted.add_parachute("drogue", cd_s_m2=0.2, released_by=released_by)
    with pytest.raises(hpr.HprError, match=message):
        hpr.Flight(chuted, SITE, 1.8)
