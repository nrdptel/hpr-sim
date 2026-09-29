"""Every option of the builder's parts, with valid values: each lands in the design the Rust
builder writes (`Rocket.design_json`), and each parachute trigger fires in flight."""

import json
import math

import pytest

import hpr


def components(rocket: hpr.Rocket) -> dict:
    """The design's components by id, children included."""
    found = {}

    def walk(parts):
        for part in parts:
            found[part["id"]] = part
            walk(part.get("children", []))

    walk(json.loads(rocket.design_json())["stages"][0]["components"])
    return found


def test_every_part_option_reaches_the_design():
    rocket = hpr.Rocket("R", 0.05)
    rocket.add_nose(
        "Power-Series", 0.2, "abs", parameter=0.5, shoulder_length_m=0.03, shoulder_wall_m=0.001
    )
    rocket.add_tube(0.3, 0.001, "abs")
    rocket.add_transition(0.05, 0.04, 0.001, "abs", shape="ogive", parameter=2.0)
    rocket.add_tube(0.4, 0.001, "abs", diameter_m=0.04, name="aft tube")
    rocket.add_fins(
        4,
        root_chord_m=0.08,
        tip_chord_m=0.03,
        span_m=0.04,
        sweep_m=0.03,
        thickness_m=0.003,
        material="abs",
        cross_section="airfoil",
        position="Bottom",
        offset_m=-0.01,
        cant_deg=1.0,
    )
    rocket.add_motor_tube(0.15, 0.029, 0.001, "abs", position="middle", offset_m=0.02)
    rocket.add_mass(0.1, position="after")
    parts = components(rocket)

    nose = parts["nose"]["part"]["nose_cone"]
    assert nose["shape"] == {"kind": "power_series", "exponent": 0.5}
    assert nose["wall"] == {"kind": "filled"}
    assert nose["shoulder"]["length_m"] == 0.03 and nose["shoulder"]["capped"] is False
    transition = parts["transition"]["part"]["transition"]
    assert transition["shape"] == {"kind": "ogive", "radius_ratio": 2.0}
    assert transition["aft_radius_m"] == 0.02
    assert transition["wall"] == {"kind": "shell", "thickness_m": 0.001}
    assert parts["tube-2"]["name"] == "aft tube"
    assert parts["tube-2"]["part"]["body_tube"]["outer_radius_m"] == 0.02
    fins = parts["fins"]
    assert fins["part"]["fin_set"]["cross_section"] == "airfoil"
    assert fins["part"]["fin_set"]["cant_rad"] == math.radians(1.0)
    assert fins["position"] == {"from": "bottom", "aft_offset_m": -0.01}
    assert parts["motor-tube"]["position"] == {"from": "middle", "aft_offset_m": 0.02}
    assert parts["mass"]["position"] == {"from": "after", "aft_offset_m": 0.0}


@pytest.mark.parametrize(
    ("shape", "parameter", "written"),
    [
        ("conical", None, {"kind": "conical"}),
        ("elliptical", None, {"kind": "elliptical"}),
        ("ogive", None, {"kind": "ogive", "radius_ratio": 1.0}),
        ("haack", None, {"kind": "haack", "parameter": 0.0}),
        ("haack", 1 / 3, {"kind": "haack", "parameter": 1 / 3}),
        ("parabolic_series", 0.75, {"kind": "parabolic_series", "parameter": 0.75}),
    ],
)
def test_nose_shapes(shape, parameter, written):
    rocket = hpr.Rocket("R", 0.05)
    rocket.add_nose(shape, 0.2, "abs", wall_m=0.002, parameter=parameter)
    nose = components(rocket)["nose"]["part"]["nose_cone"]
    assert nose["shape"] == written
    assert nose["wall"] == {"kind": "shell", "thickness_m": 0.002}


def test_a_solid_transition():
    rocket = hpr.Rocket("R", 0.05)
    rocket.add_nose("conical", 0.2, "abs").add_tube(0.3, 0.001, "abs")
    rocket.add_transition(0.05, 0.03, 0.001, "abs", solid=True)
    transition = components(rocket)["transition"]["part"]["transition"]
    assert transition["shape"] == {"kind": "conical"}
    assert transition["wall"] == {"kind": "filled"}


def small_rocket() -> hpr.Rocket:
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
    rocket.set_motor(hpr.Motor.from_catalog("H54", delay_s=10.0))
    return rocket


@pytest.mark.parametrize(
    ("options", "fires_at"),
    [
        ({"trigger": "Apogee"}, "apogee"),
        ({"trigger": "altitude", "altitude_m": 300.0}, 300.0),
        ({"trigger": "time", "time_s": 8.0}, 8.0),
        ({"trigger": "motor-delay", "motor": 0}, 13.5),
    ],
)
def test_each_trigger_fires(options, fires_at):
    rocket = small_rocket()
    rocket.add_parachute("chute", cd_s_m2=0.5, lag_s=1.0, **options)
    flight = hpr.Flight(rocket, hpr.Environment(32.99, -106.97, 1400.0), 1.8)
    events = {event["kind"]: event for event in flight.events}
    trigger, deployment = events["trigger"], events["deployment"]
    assert deployment["time_s"] == pytest.approx(trigger["time_s"] + 1.0, abs=1e-9)
    if fires_at == "apogee":
        assert trigger["time_s"] == pytest.approx(events["apogee"]["time_s"], abs=1e-9)
    elif options["trigger"] == "altitude":
        assert trigger["sample"]["height_above_ground_m"] == pytest.approx(fires_at, abs=1e-6)
        assert trigger["time_s"] > events["apogee"]["time_s"]
    else:
        # The H54 burns out at 3.5 s, so its 10 s delay ends at 13.5 s.
        assert trigger["time_s"] == pytest.approx(fires_at, abs=1e-9)


def test_a_canopy_takes_its_type_or_a_coefficient():
    plain = small_rocket().add_parachute("chute", diameter_m=0.9, canopy="Hemispherical")
    mine = small_rocket().add_parachute("chute", diameter_m=0.9, drag_coefficient=1.5)
    environment = hpr.Environment(32.99, -106.97, 1400.0)
    rate = [hpr.Flight(r, environment, 1.8).landing["descent_rate_m_s"] for r in (plain, mine)]
    assert rate[0] != rate[1]


def test_the_margin_moves_as_the_python_page_says():
    # The guide's Python page explains its rocket's 2.12 calibres against The builder's 1.92:
    # packing the recovery bay takes about 0.4 off, and the capped nose shoulder adds about 0.2.
    def margin(shoulder: bool, packed: bool) -> float:
        rocket = hpr.Rocket("R", 0.0563)
        cap = {"shoulder_length_m": 0.06, "shoulder_wall_m": 0.0015, "capped_shoulder": True}
        rocket.add_nose("ogive", 0.22, "abs", wall_m=0.0015, **(cap if shoulder else {}))
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
        pack = {"packed_length_m": 0.15, "packed_diameter_m": 0.05}
        rocket.add_mass(0.2, position="top", offset_m=0.07, **(pack if packed else {}))
        rocket.set_motor(hpr.Motor.from_catalog("H54", delay_s=10.0))
        return rocket.static_margin_cal(0.0, 0.3)

    plain, both = margin(False, False), margin(True, True)
    assert round(plain, 2) == 2.12 and round(both, 2) == 1.92
    for packing in (margin(False, True) - plain, both - margin(True, False)):
        assert round(packing, 1) == -0.4
    for shoulder in (margin(True, False) - plain, both - margin(False, True)):
        assert round(shoulder, 1) == 0.2


def test_mass_properties_are_a_tensor_of_rows():
    properties = small_rocket().mass_properties(0.0)
    inertia = properties["inertia_kg_m2"]
    assert len(inertia) == 3 and all(len(row) == 3 for row in inertia)
    # A rocket with three equal fins is symmetric about its axis: equal pitch and yaw inertia,
    # far above the roll inertia, and a symmetric tensor.
    assert inertia[0][0] == pytest.approx(inertia[1][1], rel=1e-12)
    assert inertia[2][2] < inertia[0][0] / 10
    assert all(inertia[i][j] == inertia[j][i] for i in range(3) for j in range(3))
    assert properties["cg_m"][2] < 0.0 < properties["mass_kg"]


def test_a_flight_reads_as_a_dictionary_of_arrays():
    flight = hpr.Flight(small_rocket(), hpr.Environment(0.0, 0.0, 0.0), 1.8, interval_s=0.1)
    assert "mach" in flight and "altitude" not in flight
    assert flight.keys() == flight.columns == list(flight)
    assert len(flight) == len(flight.columns)
    assert dict(zip(flight.keys(), (flight[k] for k in flight.keys()))).keys() == flight.series.keys()


def test_notes_from_reading_a_design(repo):
    built = small_rocket()
    assert built.notes == []
    json_design = hpr.Rocket.from_file(
        repo / "validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
    )
    assert json_design.notes == []
    older = hpr.Rocket.from_file(repo / "crates/hpr-format/fixtures/embedded-curve-0.1.hpr")
    assert any("version 0.1 of the hpr design format" in note for note in older.notes)
    staged = hpr.Rocket.from_file(
        repo / "validation/designs/synthetic-two-stage-75mm-54mm.json", "j760-i175"
    )
    assert any("the stages fly as one stack" in note for note in staged.notes)

