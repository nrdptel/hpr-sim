//! Tests of reading RocketPy's generator output.

use serde_json::{Value, json};

use super::{series, whole_flight_case};
use crate::case::Flight;
use crate::run::run_lock;
use crate::tests::{cases, root, scratch_case};

/// The committed whole-flight reference.
const WHOLE_FLIGHT: &str = "validation/fixtures/flight/rocketpy-whole-flight.json";

fn fixture(relative: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(root().join(relative)).expect("the fixture is committed"),
    )
    .expect("the fixture is JSON")
}

/// The case of the document named `name`.
fn case_of<'a>(document: &'a Value, name: &str) -> &'a Value {
    document["cases"]
        .as_array()
        .expect("the fixture has cases")
        .iter()
        .find(|case| case["name"] == name)
        .expect("the case is in the fixture")
}

#[test]
fn oracle_inputs_come_from_the_case_file_not_hpr_outputs() {
    // Loft lesson L75. Loft's RocketPy check was not like for like: it flew RocketPy in a standard
    // atmosphere against stored conditions, left the latitude and gravity unstated, and fed RocketPy
    // Loft's own drag and mass, so the "oracle" partly repeated Loft back to itself. Here every
    // input of a whole flight is read from the reference's record of what RocketPy flew, stated
    // there, and checked against hpr's design rather than filled in from it.
    let document = fixture(WHOLE_FLIGHT);
    let masses = fixture("validation/fixtures/design/rocketpy-rocket-mass.json");
    let declared = &document["declared_drag"]["cd0_vs_mach"];
    let mut whole_flights = 0;
    for case in cases() {
        let Flight::WholeFlight { design, .. } = &case.flight else {
            continue;
        };
        whole_flights += 1;
        let name = case.reference_case.as_deref().unwrap_or(&case.id);
        let found = case_of(&document, name);
        let (_, setup) = whole_flight_case(&document, name, WHOLE_FLIGHT, "")
            .unwrap_or_else(|| panic!("{name} reads as a whole flight"));

        // 1. Each input is the reference's own, field for field.
        let environment = &found["environment"];
        let rail = &found["rail"];
        let drag = &found["drag"];
        assert_eq!(setup.design, found["design"], "{name}");
        assert_eq!(setup.design, format!("validation/designs/{design}.json"));
        assert_eq!(setup.latitude_deg, environment["latitude_deg"], "{name}");
        assert_eq!(setup.longitude_deg, environment["longitude_deg"], "{name}");
        assert_eq!(setup.elevation_m, environment["elevation_m"], "{name}");
        assert_eq!(setup.rail_length_m, rail["rail_length_m"], "{name}");
        assert_eq!(setup.inclination_deg, rail["inclination_deg"], "{name}");
        assert_eq!(setup.heading_deg, rail["heading_deg"], "{name}");
        assert_eq!(
            setup.reference_radius_m, drag["reference_radius_m"],
            "{name}"
        );
        assert_eq!(setup.reference_area_m2, drag["reference_area_m2"], "{name}");
        assert_eq!(setup.dry_mass_kg, found["dry_mass_kg"], "{name}");
        assert_eq!(setup.effective_1rl_m, rail["effective_1rl_m"], "{name}");
        let motor = &found["motor"];
        assert_eq!(
            setup.motor.total_impulse_ns, motor["total_impulse_ns"],
            "{name}"
        );
        assert_eq!(
            setup.motor.burn_out_time_s, motor["burn_out_time_s"],
            "{name}"
        );
        assert_eq!(
            setup.motor.propellant_initial_mass_kg, motor["propellant_initial_mass_kg"],
            "{name}"
        );
        assert_eq!(
            setup.motor.reference_pressure_pa,
            motor["reference_pressure_pa"].as_f64(),
            "{name}"
        );
        let devices = found["devices"].as_array().expect("devices");
        assert_eq!(setup.devices.len(), devices.len(), "{name}");
        for (read, written) in setup.devices.iter().zip(devices) {
            assert_eq!(read.name, written["name"], "{name}");
            assert_eq!(read.cd_s_m2, written["cd_s_m2"], "{name}");
            assert_eq!(read.lag_s, written["lag_s"], "{name}");
            assert_eq!(
                read.height_above_ground_m,
                written["trigger"]["height_m"].as_f64(),
                "{name}"
            );
        }

        // 2. What Loft left unstated is stated: the site's latitude is the example's, and the
        //    gravity and atmosphere are named as RocketPy's own, which the harness flies (ADR-015).
        assert!(setup.latitude_deg.abs() > 1.0, "{name} has a real latitude");
        let gravity = environment["gravity"]
            .as_str()
            .expect("the gravity is named");
        assert!(gravity.contains("Somigliana"), "{name}: {gravity}");
        let atmosphere = environment["atmosphere"].as_str().expect("named");
        assert!(atmosphere.contains("standard atmosphere"), "{name}");

        // 3. The drag RocketPy flew is the generator's declaration, the same table for every case,
        //    not a drag of hpr's: hpr's own buildup gives every rocket a different C_D0.
        assert_eq!(&drag["cd0_vs_mach"], declared, "{name}");
        assert_eq!(setup.cd0_vs_mach, setup.declared_cd0_vs_mach, "{name}");

        // 4. The mass RocketPy flew is its own: the example's rocket and motor dry masses as
        //    RocketPy's inputs (`rocket_mass.py` recorded them), not a mass hpr computed.
        let inputs = case_of(&masses, name);
        let rocketpy_dry = inputs["rocket"]["mass"].as_f64().expect("rocket mass")
            + inputs["motor"]["dry_mass"]
                .as_f64()
                .expect("motor dry mass");
        assert!(
            (setup.dry_mass_kg - rocketpy_dry).abs() <= 1e-9 * rocketpy_dry,
            "{name}: the reference flew {} kg; RocketPy's inputs add to {rocketpy_dry}",
            setup.dry_mass_kg
        );
    }
    assert!(
        whole_flights >= 5,
        "the lock has {whole_flights} whole flights"
    );

    // 5. The inputs follow the reference and nothing else: change one in the document and what hpr
    //    would fly changes with it, exactly.
    let name = "valetudo";
    let mut moved = document.clone();
    for case in moved["cases"].as_array_mut().expect("cases") {
        if case["name"] == name {
            case["rail"]["inclination_deg"] = json!(70.0);
            case["environment"]["wind_u"] = json!(4.0);
            case["devices"][0]["cd_s_m2"] = json!(0.9);
        }
    }
    let (_, before) = whole_flight_case(&document, name, WHOLE_FLIGHT, "").expect("reads");
    let (_, after) = whole_flight_case(&moved, name, WHOLE_FLIGHT, "").expect("reads");
    assert_eq!(after.inclination_deg, 70.0);
    assert_eq!(after.wind, vec![(668.0, 4.0, 0.0)]);
    assert_eq!(after.devices[0].cd_s_m2, 0.9);
    assert_eq!(after.dry_mass_kg, before.dry_mass_kg, "nothing else moved");
    assert_eq!(after.cd0_vs_mach, before.cd0_vs_mach, "nothing else moved");

    // 6. hpr's side cannot leak into the comparison: where hpr's design and the reference disagree
    //    about the vehicle, the case is refused, never flown on hpr's numbers.
    let refused = |edit: fn(&mut Value), expected: &str| {
        let scratch = scratch_case("flight-valetudo", |document| {
            for case in document["cases"].as_array_mut().expect("cases") {
                edit(case);
            }
        });
        let error = run_lock(scratch.path(), false).expect_err(expected);
        assert!(error.to_string().contains(expected), "{error}");
    };
    // A mass that is not the one RocketPy flew.
    refused(
        |case| {
            let mass = case["dry_mass_kg"].as_f64().expect("a mass");
            case["dry_mass_kg"] = json!(mass * 1.001);
        },
        "dry mass",
    );
    // A drag table on another area: "the same C_D0" on a different area is a different drag.
    refused(
        |case| {
            let radius = case["drag"]["reference_radius_m"]
                .as_f64()
                .expect("a radius");
            case["drag"]["reference_radius_m"] = json!(radius * 1.001);
        },
        "reference area",
    );
    // A motor corrected for a reference pressure RocketPy never applied: the input this
    // milestone found transcribed wrong (ADR-021).
    refused(
        |case| case["motor"]["reference_pressure_pa"] = json!(101_325.0),
        "reference pressure",
    );
    // A motor with another impulse.
    refused(
        |case| {
            let impulse = case["motor"]["total_impulse_ns"]
                .as_f64()
                .expect("an impulse");
            case["motor"]["total_impulse_ns"] = json!(impulse * 1.001);
        },
        "total impulse",
    );
    // A case whose drag is not the generator's declaration.
    refused(
        |case| case["drag"]["cd0_vs_mach"] = json!([[0.0, 0.45], [3.0, 0.45]]),
        "its generator declares",
    );
}

#[test]
fn the_harness_flies_the_references_rail_and_wind() {
    // L75 again, at the level of the flight rather than the parser: a rail angle or a wind that
    // the harness took from anywhere but the reference would leave hpr's answer where it was when
    // the reference moved. The drift is the metric that moves most with both.
    let drift = |edit: fn(&mut Value)| {
        let scratch = scratch_case("flight-valetudo", |document| {
            for case in document["cases"].as_array_mut().expect("cases") {
                edit(case);
            }
        });
        let report = run_lock(scratch.path(), false).expect("the case runs");
        report
            .comparisons
            .iter()
            .find(|comparison| comparison.metric == "apogee_drift_m")
            .map(|comparison| comparison.measured)
            .expect("the drift is measured")
    };
    let as_flown = drift(|_| {});
    let steeper = drift(|case| case["rail"]["inclination_deg"] = json!(89.0));
    let windy = drift(|case| case["environment"]["wind_u"] = json!(-6.0));
    // Valetudo's 84.7 degree rail carries its apogee 164 m downrange in still air; nearly
    // vertical, it goes a fraction of that, and a crosswind moves it again.
    assert!(steeper < 0.5 * as_flown, "{steeper} against {as_flown}");
    assert!(
        (windy - as_flown).abs() > 20.0,
        "{windy} against {as_flown}"
    );
}

#[test]
fn the_own_drag_reference_differs_from_the_same_drag_one_only_in_its_drag() {
    // Predicted mode's case files say that everything but the drag is the same-drag case's
    // (ADR-023): the rocket, the site and wind, the rail, the motor and the parachutes. So a
    // predicted difference is the aerodynamics, not an input that drifted between the fixtures.
    let same = fixture(WHOLE_FLIGHT);
    let own = fixture("validation/fixtures/flight/rocketpy-whole-flight-own-drag.json");
    let names = |document: &Value| -> Vec<Value> {
        document["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .map(|case| case["name"].clone())
            .collect()
    };
    assert_eq!(names(&own), names(&same));
    for name in names(&same) {
        let name = name.as_str().expect("a name");
        let (a, b) = (case_of(&same, name), case_of(&own, name));
        for input in [
            "design",
            "source",
            "thrust_substitute",
            "motor",
            "environment",
            "dry_mass_kg",
            "peak_thrust_to_weight",
            "devices",
        ] {
            assert_eq!(a[input], b[input], "{name}'s {input}");
        }
        // The rail is the same but for where RocketPy's rail phase ends, which it measures.
        for key in ["rail_length_m", "inclination_deg", "heading_deg", "source"] {
            assert_eq!(a["rail"][key], b["rail"][key], "{name}'s rail {key}");
        }
        for key in ["reference_radius_m", "reference_area_m2"] {
            assert_eq!(a["drag"][key], b["drag"][key], "{name}'s drag {key}");
        }
        assert!(a["drag"]["own"].is_null() && b["drag"]["cd0_vs_mach"].is_null());
    }
    assert_eq!(own["mass_fixture"], same["mass_fixture"]);
    assert_eq!(own["oracle"], same["oracle"]);
}

#[test]
fn a_series_is_read_only_as_time_height_speed_rising_from_ignition() {
    let good = json!({
        "columns": ["time_s", "height_above_ground_m", "speed_m_s"],
        "rows": [[0.0, 0.0, 0.0], [1.5, 20.0, 30.0], [3.0, 70.0, 35.0]],
    });
    assert_eq!(
        series(Some(&good)),
        Some(vec![(0.0, 0.0, 0.0), (1.5, 20.0, 30.0), (3.0, 70.0, 35.0)])
    );
    // Every way a series could be misread is refused rather than compared.
    let with = |key: &str, value: Value| {
        let mut changed = good.clone();
        changed[key] = value;
        series(Some(&changed))
    };
    let swapped = json!(["time_s", "speed_m_s", "height_above_ground_m"]);
    assert_eq!(with("columns", swapped), None, "columns in another order");
    let late = json!([[0.5, 0.0, 0.0], [1.5, 20.0, 30.0]]);
    assert_eq!(with("rows", late), None, "a clock that is not at ignition");
    let back = json!([[0.0, 0.0, 0.0], [1.5, 20.0, 30.0], [1.5, 21.0, 30.0]]);
    assert_eq!(with("rows", back), None, "times that do not rise");
    let short = json!([[0.0, 0.0, 0.0], [1.5, 20.0]]);
    assert_eq!(with("rows", short), None, "a row missing a column");
    assert_eq!(with("rows", json!([])), None, "no rows");
    assert_eq!(series(None), None, "no series");
    // And each committed whole flight carries its 120 rows, into both RMS references at 0.
    let document = fixture(WHOLE_FLIGHT);
    let (reference, setup) = whole_flight_case(
        &document,
        "calisto-tests-motor-at-minus-1.373",
        WHOLE_FLIGHT,
        "0",
    )
    .expect("the Calisto case reads");
    assert_eq!(setup.series.len(), 120);
    for name in ["series_height_rms_m", "series_speed_rms_m_s"] {
        assert_eq!(reference.values[name].value, 0.0, "{name}");
    }
}
