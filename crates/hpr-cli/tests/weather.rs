//! `hpr weather` run as a user runs it, offline from the recorded answers in
//! `crates/hpr-net/tests/fixtures/replay/` and the ERA5 files in `validation/fixtures/weather/`:
//! each source writes its profile, from a saved answer (`--from`) and from the cache (`--offline`),
//! and the profile is the one the library builds from the same bytes.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read fixtures and write profiles; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::path::{Path, PathBuf};
use std::process::Output;

use hpr::hpr_atmos::{SoundingProfile, WindInterpolation};
use hpr::hpr_io::era5::{Era5Profile, Era5Request, UtcTime};
use hpr::hpr_io::netcdf::NetCdf;
use hpr::hpr_net::nomads::{self, NomadsModel, NomadsProfile, NomadsRequest};
use hpr::hpr_net::open_meteo::{self, OpenMeteoApi, OpenMeteoProfile, OpenMeteoRequest};
use hpr::hpr_net::wyoming::{self, WyomingRequest, WyomingSounding, WyomingVersion};
use hpr::hpr_net::{Cache, Client, Mode, Replay};
use serde_json::Value;

/// The recordings' site, Spaceport America.
const LATITUDE: &str = "32.99";
const LONGITUDE: &str = "-106.97";
/// The historical Open-Meteo recording's hours are 15 and 16 UTC on 2025-06-21.
const OPEN_METEO_TIME: &str = "2025-06-21T15:30Z";
const OPEN_METEO_S: i64 = 1_750_519_800;
/// The forecast recording's hours are 18 and 19 UTC on 2026-10-02.
const FORECAST_TIME: &str = "2026-10-02T18:00Z";
const FORECAST_S: i64 = 1_790_964_000;
/// A launch after El Paso's 12 UTC sounding of 2025-06-21.
const LAUNCH_TIME: &str = "2025-06-21T15:30Z";
const LAUNCH_S: i64 = 1_750_519_800;
/// The GFS run at 00 UTC and the RAP run at 12 UTC on 2026-09-30.
const GFS_CYCLE: &str = "2026-09-30T00Z";
const GFS_CYCLE_S: i64 = 1_790_726_400;
const RAP_CYCLE: &str = "2026-09-30T12Z";
const RAP_CYCLE_S: i64 = 1_790_769_600;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn replay_dir() -> PathBuf {
    root().join("crates/hpr-net/tests/fixtures/replay")
}

fn recording(name: &str) -> String {
    replay_dir().join(name).to_string_lossy().into_owned()
}

fn era5_file(name: &str) -> String {
    root()
        .join("validation/fixtures/weather/era5")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// Runs `hpr` with `args` and `HPR_CACHE_DIR` set to `cache`.
fn hpr(args: &[&str], cache: &Path) -> Output {
    assert_cmd::cargo::cargo_bin_cmd!("hpr")
        .args(args)
        .env("HPR_CACHE_DIR", cache)
        .output()
        .unwrap()
}

/// Runs `hpr` with `--json`, expects `code` and nothing on standard error, checks the document
/// against the committed schema `schema`, and returns it.
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

/// One source: its arguments to fetch (before `--offline`), its arguments with `--from`, the
/// recording, and what the library reads from the recording's bytes.
struct Case {
    name: &'static str,
    args: Vec<&'static str>,
    from_args: Vec<&'static str>,
    recording: &'static str,
    source: &'static str,
    time: &'static str,
    run: Option<&'static str>,
    library: Library,
}

/// What the library reads from a recording.
struct Library {
    profile: SoundingProfile,
    /// Where the profile is, degrees north and east.
    position: (f64, f64),
    /// The levels left out: the reason as the output names it, the pressure (Pa) or the line.
    dropped: Vec<(String, Option<f64>, Option<usize>)>,
}

fn bytes(name: &str) -> Vec<u8> {
    std::fs::read(recording(name)).unwrap()
}

/// A library reason's name as the output writes it: `BelowGround` as `below_ground`. Open-Meteo's
/// `Humidity`, a humidity outside 0 to 100%, is the output's `out_of_range`.
fn reason_name(reason: &impl std::fmt::Debug) -> String {
    let debug = format!("{reason:?}");
    if debug == "Humidity" {
        return "out_of_range".to_owned();
    }
    let mut name = String::new();
    for (i, c) in debug.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            name.push('_');
        }
        name.push(c.to_ascii_lowercase());
    }
    name
}

fn cases() -> Vec<Case> {
    let open_meteo = |name, time_s| {
        let read = OpenMeteoProfile::parse(&bytes(name), time_s).unwrap();
        Library {
            profile: read.sounding(WindInterpolation::SpeedDirection).unwrap(),
            position: (read.latitude_deg, read.longitude_deg),
            dropped: read
                .dropped
                .iter()
                .map(|level| (reason_name(&level.reason), Some(level.pressure_pa), None))
                .collect(),
        }
    };
    let wyoming = |name| {
        let read = WyomingSounding::parse(&bytes(name)).unwrap();
        Library {
            profile: read.sounding(WindInterpolation::SpeedDirection).unwrap(),
            position: (read.latitude_deg, read.longitude_deg),
            dropped: read
                .dropped
                .iter()
                .map(|row| (reason_name(&row.reason), None, Some(row.line)))
                .collect(),
        }
    };
    let nomads = |name| {
        let read = NomadsProfile::parse(&bytes(name), 32.99, -106.97).unwrap();
        Library {
            profile: read.sounding(WindInterpolation::SpeedDirection).unwrap(),
            position: (read.latitude_deg, read.longitude_deg),
            dropped: read
                .dropped
                .iter()
                .map(|level| (reason_name(&level.reason), Some(level.pressure_pa), None))
                .collect(),
        }
    };
    let site = ["--latitude", LATITUDE, "--longitude", LONGITUDE];
    let with = |command: &[&'static str], rest: &[&'static str]| {
        let mut args = vec!["weather"];
        args.extend_from_slice(command);
        args.extend_from_slice(&site);
        args.extend_from_slice(rest);
        args
    };
    vec![
        Case {
            name: "Open-Meteo historical forecast",
            args: with(
                &["open-meteo"],
                &["--time", OPEN_METEO_TIME, "--historical"],
            ),
            from_args: vec!["weather", "open-meteo", "--time", OPEN_METEO_TIME],
            recording: "open-meteo-historical.json",
            source: "open_meteo",
            time: "2025-06-21T15:30:00Z",
            run: None,
            library: open_meteo("open-meteo-historical.json", OPEN_METEO_S),
        },
        Case {
            name: "Open-Meteo forecast",
            args: with(&["open-meteo"], &["--time", FORECAST_TIME]),
            from_args: vec!["weather", "open-meteo", "--time", FORECAST_TIME],
            recording: "open-meteo-forecast.json",
            source: "open_meteo",
            time: "2026-10-02T18:00:00Z",
            run: None,
            library: open_meteo("open-meteo-forecast.json", FORECAST_S),
        },
        Case {
            name: "Wyoming FM 35",
            args: vec![
                "weather",
                "wyoming",
                "--station",
                "72364",
                "--time",
                LAUNCH_TIME,
            ],
            from_args: vec!["weather", "wyoming"],
            recording: "wyoming-72364-fm35.csv",
            source: "wyoming",
            time: "2025-06-21T11:02:00Z",
            run: None,
            library: wyoming("wyoming-72364-fm35.csv"),
        },
        Case {
            name: "Wyoming BUFR",
            args: vec![
                "weather",
                "wyoming",
                "--station",
                "72364",
                "--time",
                LAUNCH_TIME,
                "--bufr",
            ],
            from_args: vec!["weather", "wyoming"],
            recording: "wyoming-72364-bufr.csv",
            source: "wyoming",
            time: "2025-06-21T11:02:18Z",
            run: None,
            library: wyoming("wyoming-72364-bufr.csv"),
        },
        Case {
            name: "GFS",
            args: with(&["gfs"], &["--cycle", GFS_CYCLE, "--hour", "18"]),
            from_args: with(&["gfs"], &[]),
            recording: "nomads-gfs.grib2",
            source: "gfs",
            time: "2026-09-30T18:00:00Z",
            run: Some("2026-09-30T00:00:00Z"),
            library: nomads("nomads-gfs.grib2"),
        },
        Case {
            name: "RAP",
            args: with(&["rap"], &["--cycle", RAP_CYCLE, "--hour", "6"]),
            from_args: with(&["rap"], &[]),
            recording: "nomads-rap.grib2",
            source: "rap",
            time: "2026-09-30T18:00:00Z",
            run: Some("2026-09-30T12:00:00Z"),
            library: nomads("nomads-rap.grib2"),
        },
    ]
}

/// Fills `dir` with every recording, fetched as `hpr weather` asks for it, through a client over
/// the replay: the cache a user has after fetching them online.
fn fill_cache(dir: &Path, now_s: u64) {
    let replay = Replay::open(replay_dir()).unwrap();
    let client = Client::new(&replay, Cache::new(dir), Mode::Online);
    for (time_s, api) in [
        (OPEN_METEO_S, OpenMeteoApi::HistoricalForecast),
        (FORECAST_S, OpenMeteoApi::Forecast),
    ] {
        let request = OpenMeteoRequest::new(32.99, -106.97, time_s, api);
        open_meteo::fetch(&client, &request, now_s).unwrap();
    }
    for version in [WyomingVersion::Fm35, WyomingVersion::Bufr] {
        let mut request = WyomingRequest::latest_before("72364", LAUNCH_S);
        request.version = version;
        wyoming::fetch(&client, &request, now_s).unwrap();
    }
    for (model, cycle_s, hour) in [
        (NomadsModel::Gfs, GFS_CYCLE_S, 18),
        (NomadsModel::Rap, RAP_CYCLE_S, 6),
    ] {
        let request = NomadsRequest::new(32.99, -106.97, model, cycle_s, hour);
        nomads::fetch(&client, &request, now_s).unwrap();
    }
    assert_eq!(replay.calls(), 6);
}

fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Checks a run's document and the profile file it wrote against the library's profile.
fn check_written(case: &Case, document: &Value, written: &Path) {
    let name = case.name;
    assert_eq!(document["source"]["name"], case.source, "{name}");
    assert_eq!(document["time"], case.time, "{name}");
    assert_eq!(document["run"].as_str(), case.run, "{name}");
    let library = &case.library;
    let position = &document["position"];
    assert_eq!(
        (
            position["latitude_deg"].as_f64(),
            position["longitude_deg"].as_f64()
        ),
        (Some(library.position.0), Some(library.position.1)),
        "{name}"
    );
    let dropped: Vec<(String, Option<f64>, Option<usize>)> = document["dropped"]
        .as_array()
        .unwrap()
        .iter()
        .map(|level| {
            (
                level["reason"].as_str().unwrap().to_owned(),
                level["pressure_pa"].as_f64(),
                level["line"]
                    .as_u64()
                    .map(|line| usize::try_from(line).unwrap()),
            )
        })
        .collect();
    assert_eq!(dropped, library.dropped, "{name}: the levels left out");
    let profile: SoundingProfile =
        serde_json::from_str(&std::fs::read_to_string(written).unwrap()).unwrap();
    assert_eq!(profile, library.profile, "{name}: the written profile");
    let levels = document["levels"].as_array().unwrap();
    assert_eq!(levels.len(), library.profile.levels().len(), "{name}");
    for (row, level) in levels.iter().zip(library.profile.levels()) {
        assert_eq!(
            row["height_msl_m"].as_f64(),
            Some(level.height_msl_m),
            "{name}"
        );
        assert_eq!(row["pressure_pa"].as_f64(), level.pressure_pa, "{name}");
        assert_eq!(
            row["temperature_k"].as_f64(),
            Some(level.temperature_k),
            "{name}"
        );
        assert_eq!(
            row["relative_humidity"].as_f64(),
            level.relative_humidity,
            "{name}"
        );
        assert_eq!(
            row["wind_speed_m_s"].as_f64(),
            level.wind_speed_m_s,
            "{name}"
        );
        assert_eq!(
            row["wind_from_deg"].as_f64(),
            level.wind_direction_from_rad.map(f64::to_degrees),
            "{name}"
        );
    }
}

/// Every online source writes a site's profile from its answer saved to a file, with no network
/// and no cache: the profile the library builds from the same bytes.
#[test]
fn every_source_writes_its_profile_from_a_saved_answer() {
    let scratch = tempfile::tempdir().unwrap();
    let cache = scratch.path().join("cache");
    for case in cases() {
        let from = recording(case.recording);
        let written = scratch.path().join("profile.json");
        let mut args = case.from_args.clone();
        args.extend(["--from", &from, "--output", written.to_str().unwrap()]);
        let document = json(&args, &cache, 0, "weather.schema.json");
        assert_eq!(document["read_from"]["kind"], "file", "{}", case.name);
        assert_eq!(
            document["read_from"]["path"],
            from.as_str(),
            "{}",
            case.name
        );
        check_written(&case, &document, &written);
    }
    assert!(!cache.exists(), "a saved answer mustn't touch the cache");
}

/// Offline, every online source writes its profile from the cache a fetch filled, and the
/// profile is the one its saved answer gives.
#[test]
fn every_source_writes_its_profile_offline_from_the_cache() {
    let scratch = tempfile::tempdir().unwrap();
    let cache = scratch.path().join("cache");
    fill_cache(&cache, now_s());
    for case in cases() {
        let written = scratch.path().join("profile.json");
        let mut args = case.args.clone();
        args.extend(["--offline", "--output", written.to_str().unwrap()]);
        let document = json(&args, &cache, 0, "weather.schema.json");
        let kind = document["read_from"]["kind"].as_str().unwrap();
        // Fresh unless the run crossed a forecast's hour of freshness since the fill.
        assert!(
            kind == "cache" || kind == "stale_cache",
            "{}: {kind}",
            case.name
        );
        check_written(&case, &document, &written);
    }
}

/// Offline with nothing cached, the fetch is refused and names what it would have fetched.
#[test]
fn offline_without_a_copy_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    for case in cases() {
        let mut args = case.args.clone();
        args.push("--offline");
        let document = json(&args, scratch.path(), 1, "error.schema.json");
        let message = document["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("is not in the cache"),
            "{}: {message}",
            case.name
        );
    }
}

/// An ERA5 file writes the profile `Era5Profile` reads from it, at both sites the fixtures cover.
#[test]
fn an_era5_file_writes_its_profile() {
    let scratch = tempfile::tempdir().unwrap();
    for (file, latitude, longitude, time, written_time, (year, month, day, hour)) in [
        (
            "bella-lui.nc",
            47.213_476,
            9.003_336,
            "2020-02-22T13:00Z",
            "2020-02-22T13:00:00Z",
            (2020, 2, 22, 13),
        ),
        (
            "ndrt-2020.nc",
            41.775_447,
            -86.572_467,
            "2020-02-23T16:00Z",
            "2020-02-23T16:00:00Z",
            (2020, 2, 23, 16),
        ),
    ] {
        let path = era5_file(file);
        let written = scratch.path().join("profile.json");
        let (lat, lon) = (latitude.to_string(), longitude.to_string());
        let document = json(
            &[
                "weather",
                "era5",
                &path,
                "--latitude",
                &lat,
                "--longitude",
                &lon,
                "--time",
                time,
                "-o",
                written.to_str().unwrap(),
            ],
            scratch.path(),
            0,
            "weather.schema.json",
        );
        let request = Era5Request {
            latitude_deg: latitude,
            longitude_deg: longitude,
            time: UtcTime::from_civil(year, month, day, hour, 0, 0.0).unwrap(),
        };
        let netcdf = NetCdf::parse(&std::fs::read(&path).unwrap()).unwrap();
        let profile = Era5Profile::read(&netcdf, request)
            .unwrap()
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        let case = Case {
            name: file,
            args: Vec::new(),
            from_args: Vec::new(),
            recording: file,
            source: "era5",
            time: written_time,
            run: None,
            library: Library {
                profile,
                position: (latitude, longitude),
                dropped: Vec::new(),
            },
        };
        assert_eq!(document["read_from"]["kind"], "file");
        assert_eq!(document["read_from"]["path"], path.as_str());
        check_written(&case, &document, &written);
    }
}

/// A saved answer is held to the checks a fetch makes: a RAP cut is not GFS, and a cut of
/// another run or hour than the one named is refused.
#[test]
fn a_saved_cut_is_checked_like_a_fetched_one() {
    let scratch = tempfile::tempdir().unwrap();
    let site = ["--latitude", LATITUDE, "--longitude", LONGITUDE];
    let rap = recording("nomads-rap.grib2");
    let mut args = vec!["weather", "gfs"];
    args.extend(site);
    args.extend(["--from", &rap]);
    let document = json(&args, scratch.path(), 1, "error.schema.json");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("its grid is not GFS's"), "{message}");

    let gfs = recording("nomads-gfs.grib2");
    let mut args = vec!["weather", "gfs"];
    args.extend(site);
    args.extend(["--from", &gfs, "--cycle", GFS_CYCLE, "--hour", "12"]);
    let document = json(&args, scratch.path(), 1, "error.schema.json");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(
        message.contains(
            "it is the run of 2026-09-30T00:00:00Z for 2026-09-30T18:00:00Z, not the run of \
             2026-09-30T00:00:00Z for 2026-09-30T12:00:00Z"
        ),
        "{message}"
    );
}

/// The text output names the source's credit, where it read from, and the file written.
#[test]
fn the_text_names_the_credit_and_the_file() {
    let scratch = tempfile::tempdir().unwrap();
    let from = recording("nomads-gfs.grib2");
    let written = scratch.path().join("gfs.json");
    let output = hpr(
        &[
            "weather",
            "gfs",
            "--latitude",
            LATITUDE,
            "--longitude",
            LONGITUDE,
            "--from",
            &from,
            "-o",
            written.to_str().unwrap(),
        ],
        scratch.path(),
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "GFS at 32.9900° N, 106.9700° W, for 2026-09-30T18:00:00Z, run of 2026-09-30T00:00:00Z",
        "Read from nomads-gfs.grib2",
        nomads::ATTRIBUTION,
        "below the ground, 6 levels: 1000 hPa, 975 hPa, 950 hPa, 925 hPa, 900 hPa, 850 hPa",
        &format!("Profile written to {}", written.display()),
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
}

/// Times are UTC with a `Z`; anything else is refused before any fetch.
#[test]
fn a_time_without_its_zone_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let document = json(
        &[
            "weather",
            "open-meteo",
            "--latitude",
            LATITUDE,
            "--longitude",
            LONGITUDE,
            "--time",
            "2025-06-21T15:30",
            "--offline",
        ],
        scratch.path(),
        1,
        "error.schema.json",
    );
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("write a time in UTC"), "{message}");
}

/// With `--from`, the options that choose which answer to fetch are refused rather than ignored:
/// the saved answer says where and when it is for, and nothing could check it against them.
#[test]
fn options_a_saved_answer_cant_honour_are_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let om = recording("open-meteo-historical.json");
    let wy = recording("wyoming-72364-fm35.csv");
    let open_meteo = [
        "weather",
        "open-meteo",
        "--time",
        OPEN_METEO_TIME,
        "--from",
        &om,
    ];
    let wyoming = ["weather", "wyoming", "--from", &wy];
    for (base, extra) in [
        (&open_meteo[..], &["--latitude", LATITUDE][..]),
        (&open_meteo[..], &["--longitude", LONGITUDE][..]),
        (&open_meteo[..], &["--historical"][..]),
        (&open_meteo[..], &["--model", "gfs_seamless"][..]),
        (&wyoming[..], &["--station", "72364"][..]),
        (&wyoming[..], &["--time", LAUNCH_TIME][..]),
        (&wyoming[..], &["--bufr"][..]),
    ] {
        let mut args = base.to_vec();
        args.extend_from_slice(extra);
        let document = json(&args, scratch.path(), 2, "error.schema.json");
        let message = document["error"]["message"].as_str().unwrap();
        // Clap names the two in either order.
        assert!(
            message.contains("cannot be used with")
                && message.contains("'--from <FILE>'")
                && message.contains(&format!("'{}", extra[0])),
            "{extra:?}: {message}"
        );
    }
}

/// A time outside the saved answer's hours is refused, written in UTC as it was asked.
#[test]
fn a_time_outside_the_answer_is_named_in_utc() {
    let scratch = tempfile::tempdir().unwrap();
    let om = recording("open-meteo-historical.json");
    let document = json(
        &[
            "weather",
            "open-meteo",
            "--time",
            "2025-06-22T15:30Z",
            "--from",
            &om,
        ],
        scratch.path(),
        1,
        "error.schema.json",
    );
    let message = document["error"]["message"].as_str().unwrap();
    assert!(
        message.contains(
            "2025-06-22T15:30:00Z is outside the answer's hours, 2025-06-21T15:00:00Z to \
             2025-06-21T16:00:00Z"
        ),
        "{message}"
    );
}
