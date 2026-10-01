//! `hpr motors search` run as a user runs it, offline from motor.fusionspace.co's recorded
//! answers in `crates/hpr-net/tests/fixtures/replay/` (ADR-131): from a saved list (`--from`) and
//! from the cache a fetch filled (`--offline`), each listing the recording's own values, chosen
//! and ordered as the filters say, with both credits on every list.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read fixtures and write edited copies; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::path::{Path, PathBuf};
use std::process::Output;

use hpr::hpr_net::motor_finder::{self, Endpoint};
use hpr::hpr_net::{Cache, Client, Mode, Replay, thrustcurve};
use serde_json::Value;

/// The example the milestone names.
const EXAMPLE: [&str; 5] = ["--in-stock", "--class", "L", "--max-price", "150"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn recording(name: &str) -> PathBuf {
    root()
        .join("crates/hpr-net/tests/fixtures/replay")
        .join(name)
}

fn in_stock_file() -> String {
    recording("motor-finder-in-stock.json")
        .to_string_lossy()
        .into_owned()
}

fn motors_file() -> String {
    recording("motor-finder-motors.json")
        .to_string_lossy()
        .into_owned()
}

/// A recorded list's JSON.
fn raw(path: &str) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// Runs `hpr motors search` with `args` and `HPR_CACHE_DIR` set to `cache`.
fn hpr(args: &[&str], cache: &Path) -> Output {
    assert_cmd::cargo::cargo_bin_cmd!("hpr")
        .args(["motors", "search"])
        .args(args)
        .env("HPR_CACHE_DIR", cache)
        .output()
        .unwrap()
}

/// The text `hpr motors search` prints with `args`, which must exit 0 with nothing on standard
/// error.
fn text(args: &[&str], cache: &Path) -> String {
    let output = hpr(args, cache);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// Runs with `--json`, expects `code` and nothing on standard error, checks the document against
/// the committed schema `schema`, and returns it.
fn json(args: &[&str], cache: &Path, code: i32, schema: &str) -> Value {
    let mut args = args.to_vec();
    args.push("--json");
    let output = hpr(&args, cache);
    assert_eq!(output.status.code(), Some(code), "{args:?}: {output:?}");
    assert!(output.stderr.is_empty(), "{args:?}: {output:?}");
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    let path = root().join("schema/cli").join(schema);
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(&document)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{errors:?}\n{document:#}");
    document
}

/// A search's document, which must exit 0.
fn search(args: &[&str], cache: &Path) -> Value {
    json(args, cache, 0, "motors-search.schema.json")
}

/// The filters a test applies to the recording's JSON by itself, apart from the command's code.
#[derive(Default)]
struct Expect {
    in_stock: bool,
    class: Option<&'static str>,
    diameter_mm: Option<f64>,
    manufacturer: Option<&'static str>,
    max_price_cents: Option<u64>,
}

/// One motor's price at the cheapest vendor with it in stock, U.S. cents, from the raw JSON.
fn raw_price(motor: &Value) -> Option<u64> {
    let offer = &motor["cheapest_in_stock"];
    (offer["currency"] == "USD")
        .then(|| offer["unit_price_cents"].as_u64())
        .flatten()
}

impl Expect {
    /// The recording's motors that pass, as `(manufacturer, designation)`, cheapest first, then
    /// by maker and designation, unpriced last.
    fn names(&self, list: &Value) -> Vec<(String, String)> {
        let mut motors: Vec<&Value> = list["motors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| !self.in_stock || m["in_stock"] == true)
            .filter(|m| self.class.is_none_or(|class| m["impulse_class"] == class))
            .filter(|m| {
                self.diameter_mm
                    .is_none_or(|mm| (m["diameter_mm"].as_f64().unwrap() - mm).abs() <= 0.5)
            })
            .filter(|m| {
                self.manufacturer
                    .is_none_or(|name| m["manufacturer"] == name)
            })
            .filter(|m| {
                self.max_price_cents
                    .is_none_or(|most| raw_price(m).is_some_and(|cents| cents <= most))
            })
            .collect();
        let key = |m: &Value| {
            (
                raw_price(m).is_none(),
                raw_price(m).unwrap_or(0),
                m["manufacturer"].as_str().unwrap().to_owned(),
                m["designation"].as_str().unwrap().to_owned(),
            )
        };
        motors.sort_by_key(|m| key(m));
        motors.iter().map(|m| name(m)).collect()
    }
}

fn name(motor: &Value) -> (String, String) {
    (
        motor["manufacturer"].as_str().unwrap().to_owned(),
        motor["designation"].as_str().unwrap().to_owned(),
    )
}

/// Checks that every motor `document` lists is the recording's, value for value.
fn values_are_the_recordings(document: &Value, list: &Value) {
    let recorded = list["motors"].as_array().unwrap();
    for listed in document["motors"].as_array().unwrap() {
        let motor = recorded.iter().find(|m| name(m) == name(listed));
        assert!(motor.is_some(), "{listed} is not in the recording");
        let motor = motor.unwrap();
        for (out, raw) in [
            ("manufacturer", "manufacturer"),
            ("designation", "designation"),
            ("common_name", "common_name"),
            ("impulse_class", "impulse_class"),
            ("diameter_mm", "diameter_mm"),
            ("total_impulse_ns", "total_impulse_ns"),
            ("average_thrust_n", "avg_thrust_n"),
            ("burn_time_s", "burn_time_s"),
            ("propellant", "propellant"),
            ("delays", "delays"),
            ("in_stock", "in_stock"),
            ("in_stock_vendor_count", "in_stock_vendor_count"),
        ] {
            // Numbers compare as numbers: the recording writes `75`, the output `75.0`.
            match (listed[out].as_f64(), motor[raw].as_f64()) {
                (Some(a), Some(b)) => assert_eq!(a.to_bits(), b.to_bits(), "{out}: {listed}"),
                _ => assert_eq!(listed[out], motor[raw], "{out}: {listed}"),
            }
        }
        let kind = match motor["motor_type"].as_str() {
            Some("SU") => Value::from("single_use"),
            Some("reload") => Value::from("reload"),
            Some("hybrid") => Value::from("hybrid"),
            _ => Value::Null,
        };
        assert_eq!(listed["motor_type"], kind, "{listed}");
        let offer = &motor["cheapest_in_stock"];
        if offer.is_null() {
            assert!(listed["cheapest_in_stock"].is_null(), "{listed}");
        } else {
            for field in [
                "vendor",
                "url",
                "unit_price_cents",
                "price_cents",
                "pack_size",
                "currency",
            ] {
                assert_eq!(listed["cheapest_in_stock"][field], offer[field], "{listed}");
            }
        }
    }
}

/// Both credits, in the order shown.
fn credits() -> [&'static str; 2] {
    [motor_finder::ATTRIBUTION, thrustcurve::ATTRIBUTION]
}

/// Checks the credits a document and its text carry.
fn credited(document: &Value, text: &str) {
    assert_eq!(document["attribution"], serde_json::json!(credits()));
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[2..4], credits(), "{text}");
}

/// The example lists from the recorded snapshot: nothing, as no L motor in stock sold for $150
/// or less on the recording, and the text says what the cheapest cost. With one L motor's price
/// edited to $149.99 in a copy, the same command lists exactly it, and at $149.98 nothing.
#[test]
fn the_example_lists_from_a_recorded_snapshot() {
    let scratch = tempfile::tempdir().unwrap();
    let file = in_stock_file();
    let list = raw(&file);
    let mut args = EXAMPLE.to_vec();
    args.extend(["--from", &file]);
    let document = search(&args, scratch.path());
    assert_eq!(document["motors"], serde_json::json!([]));
    assert_eq!(document["read_from"]["kind"], "file");
    assert_eq!(document["generated_at"], list["generated_at"]);
    let printed = text(&args, scratch.path());
    credited(&document, &printed);
    assert_eq!(
        printed.lines().next().unwrap(),
        "0 motors (in stock, class L, at most $150.00 each) in motor.fusionspace.co's list built \
         2026-10-01T07:07:29Z."
    );
    // The recording's cheapest L in stock, found here from its JSON.
    let cheapest = Expect {
        in_stock: true,
        class: Some("L"),
        ..Expect::default()
    }
    .names(&list)[0]
        .clone();
    assert_eq!(cheapest, ("AeroTech".to_owned(), "L1520T".to_owned()));
    assert!(
        printed.ends_with(
            "None costs $150.00 or less: of the 20 motors the other filters pass, the cheapest \
             is AeroTech L1520T at $260.99.\n"
        ),
        "{printed}"
    );

    // L1520T's cheapest offer and its listing at $149.99 a motor, in a copy.
    let mut edited = list.clone();
    let motor = edited["motors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|m| m["designation"] == "L1520T")
        .unwrap();
    let url = motor["cheapest_in_stock"]["url"].clone();
    motor["cheapest_in_stock"]["price_cents"] = 14_999.into();
    motor["cheapest_in_stock"]["unit_price_cents"] = 14_999.into();
    let listing = motor["listings"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["url"] == url && l["status"] == "in_stock")
        .unwrap();
    listing["price_cents"] = 14_999.into();
    listing["unit_price_cents"] = 14_999.into();
    let copy = scratch.path().join("in-stock.json");
    std::fs::write(&copy, serde_json::to_vec(&edited).unwrap()).unwrap();
    let copy = copy.to_string_lossy().into_owned();
    let mut args = EXAMPLE.to_vec();
    args.extend(["--from", &copy]);
    let document = search(&args, scratch.path());
    let motors = document["motors"].as_array().unwrap();
    assert_eq!(motors.len(), 1, "{document:#}");
    assert_eq!(motors[0]["designation"], "L1520T");
    assert_eq!(motors[0]["cheapest_in_stock"]["unit_price_cents"], 14_999);
    values_are_the_recordings(&document, &edited);
    let printed = text(&args, scratch.path());
    assert!(printed.contains("\nL1520T "), "{printed}");
    assert!(printed.contains(" 149.99 "), "{printed}");
    args[4] = "149.98";
    assert_eq!(
        search(&args, scratch.path())["motors"],
        serde_json::json!([])
    );
}

/// Each filter keeps what the recording's JSON says it should, in the order it says, with the
/// recording's values: on the in-stock list and on the whole one.
#[test]
fn listed_motors_are_the_recordings() {
    let scratch = tempfile::tempdir().unwrap();
    for (file, args, expect, count) in [
        (
            in_stock_file(),
            vec!["--in-stock", "--class", "L", "--max-price", "300"],
            Expect {
                in_stock: true,
                class: Some("L"),
                max_price_cents: Some(30_000),
                ..Expect::default()
            },
            4,
        ),
        (in_stock_file(), vec![], Expect::default(), 282),
        (motors_file(), vec![], Expect::default(), 598),
        (
            motors_file(),
            vec!["--class", "l"],
            Expect {
                class: Some("L"),
                ..Expect::default()
            },
            55,
        ),
        (
            motors_file(),
            vec!["--in-stock", "--class", "L"],
            Expect {
                in_stock: true,
                class: Some("L"),
                ..Expect::default()
            },
            20,
        ),
        (
            motors_file(),
            vec![
                "--diameter",
                "38",
                "--manufacturer",
                "loki",
                "--max-price",
                "87",
            ],
            Expect {
                diameter_mm: Some(38.0),
                manufacturer: Some("Loki Research"),
                max_price_cents: Some(8_700),
                ..Expect::default()
            },
            0,
        ),
        (
            motors_file(),
            vec!["--manufacturer", "Cesaroni Technology", "--diameter", "38"],
            Expect {
                manufacturer: Some("Cesaroni Technology"),
                diameter_mm: Some(38.0),
                ..Expect::default()
            },
            0,
        ),
    ] {
        let list = raw(&file);
        let mut all = args.clone();
        all.extend(["--from", &file]);
        let document = search(&all, scratch.path());
        let listed: Vec<(String, String)> = document["motors"]
            .as_array()
            .unwrap()
            .iter()
            .map(name)
            .collect();
        let expected = expect.names(&list);
        assert_eq!(listed, expected, "{args:?}");
        // The counts the recording holds (ADR-129), where a test names one; 0 marks a count left
        // to the recording's JSON.
        if count > 0 {
            assert_eq!(listed.len(), count, "{args:?}");
        } else {
            assert!(
                !listed.is_empty(),
                "{args:?} lists nothing: a filter test needs motors"
            );
        }
        values_are_the_recordings(&document, &list);
        credited(&document, &text(&all, scratch.path()));
    }
}

/// Fills `dir` with both lists, fetched as `hpr motors search` asks for them, through a client
/// over the replay: the cache a user has after searching online.
fn fill_cache(dir: &Path, now_s: u64) {
    let replay = Replay::open(root().join("crates/hpr-net/tests/fixtures/replay")).unwrap();
    let client = Client::new(&replay, Cache::new(dir), Mode::Online);
    motor_finder::fetch_in_stock(&client, now_s).unwrap();
    motor_finder::fetch_motors(&client, now_s).unwrap();
    assert_eq!(replay.calls(), 2);
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Offline, a search answers from the cache a fetch filled, with the same motors as from the
/// saved list and both credits; `--in-stock` reads `in-stock.json`, and without it `motors.json`.
#[test]
fn offline_from_the_cache() {
    let cache = tempfile::tempdir().unwrap();
    let filled_s = now_s();
    fill_cache(cache.path(), filled_s);
    for (args, file) in [
        (
            vec!["--in-stock", "--class", "L", "--max-price", "300"],
            in_stock_file(),
        ),
        (EXAMPLE.to_vec(), in_stock_file()),
        (vec!["--class", "O"], motors_file()),
    ] {
        let mut offline = args.clone();
        offline.push("--offline");
        let document = search(&offline, cache.path());
        let mut from = args.clone();
        from.extend(["--from", &file]);
        let saved = search(&from, cache.path());
        assert_eq!(document["motors"], saved["motors"], "{args:?}");
        assert_eq!(document["generated_at"], saved["generated_at"]);
        // Fresh unless the run crossed the hour a copy stays fresh since the fill.
        let read_from = &document["read_from"];
        assert!(
            read_from["kind"] == "cache" || read_from["kind"] == "stale_cache",
            "{read_from}"
        );
        assert_eq!(read_from["fetched_at_unix_s"], filled_s);
        let printed = text(&offline, cache.path());
        credited(&document, &printed);
        assert!(
            printed
                .lines()
                .nth(1)
                .unwrap()
                .starts_with("From the cache"),
            "{printed}"
        );
    }
}

/// Offline with nothing cached, the search is refused and names what it would have fetched.
#[test]
fn offline_without_a_copy_is_refused() {
    let empty = tempfile::tempdir().unwrap();
    for (args, endpoint) in [
        (vec!["--in-stock", "--offline"], Endpoint::InStock),
        (vec!["--offline"], Endpoint::Motors),
    ] {
        let document = json(&args, empty.path(), 1, "error.schema.json");
        let message = document["error"]["message"].as_str().unwrap();
        assert!(message.starts_with("motor.fusionspace.co: "), "{message}");
        assert!(message.contains("is not in the cache"), "{message}");
        assert!(message.contains(&endpoint.url()), "{message}");
    }
}

/// A filter that can't match anything real is refused before anything is read, rather than
/// listing nothing as if no motor matched; so is a saved file that isn't a list.
#[test]
fn bad_filters_and_files_are_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let file = in_stock_file();
    for (args, refusal) in [
        (vec!["--class", "LL"], "--class LL is not an impulse class"),
        (
            vec!["--diameter", "-54"],
            "--diameter must be a positive number",
        ),
        (
            vec!["--diameter", "NaN"],
            "--diameter must be a positive number",
        ),
        (
            vec!["--manufacturer", "Estes"],
            "--manufacturer Estes is no maker",
        ),
        (
            vec!["--max-price", "150.001"],
            "--max-price 150.001 is not a price",
        ),
        (vec!["--max-price", "-5"], "--max-price -5 is not a price"),
    ] {
        let mut all = args.clone();
        // Refused before the file is read: the file doesn't exist.
        all.extend(["--from", "no-such-file.json"]);
        let document = json(&all, scratch.path(), 1, "error.schema.json");
        let message = document["error"]["message"].as_str().unwrap();
        assert!(message.starts_with(refusal), "{args:?}: {message}");
        let mut all = args.clone();
        all.extend(["--from", &file]);
        json(&all, scratch.path(), 1, "error.schema.json");
    }
    // A saved file that isn't a motor list: the finder's `meta.json`.
    let meta = recording("motor-finder-meta.json")
        .to_string_lossy()
        .into_owned();
    let document = json(&["--from", &meta], scratch.path(), 1, "error.schema.json");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.starts_with(&format!("{meta}: ")), "{message}");
    // `--from` and `--offline` together are a usage error.
    let output = hpr(&["--from", &file, "--offline"], scratch.path());
    assert_eq!(output.status.code(), Some(2), "{output:?}");
}
