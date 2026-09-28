//! `cargo xtask ork-flights [--check]`: flies hpr on every configuration of OpenRocket's flight
//! record that hpr flies, in the recorded conditions, and reports the apogee, the largest speed and
//! the stability margin at rod clearance against OpenRocket's (M2.2d2, ADR-069).
//!
//! The record is `validation/fixtures/ork/openrocket-flights.json` (M2.2d1, ADR-068): OpenRocket
//! 24.12's calm-air flight of every motor configuration of its 17 examples and the seven Loft
//! demos. Each metric is taken by the [`definition`] hpr holds for OpenRocket 24.12 and compared
//! with [`compare`], so an aborted reference is withheld and a missing value is never scored:
//!
//! - **apogee**: the largest height of hpr's centre of mass above the site, which is where hpr's
//!   apogee event puts it;
//! - **largest speed**: the largest speed of the centre of mass, sampled at every step's end and
//!   at three points inside it from the dense output;
//! - **margin at rod clearance**: at the recorded rod-clearance step's time and Mach number, hpr's
//!   centre of pressure at zero angle of attack less its centre of mass at that time, over its
//!   reference diameter.
//!
//! hpr flies no recovery device from a `.ork` yet (ADR-057 reads them, nothing flies them), so a
//! reference whose parachute opened before its apogee is marked, with how long before, and is
//! summarised apart. A configuration with a powered separation ([`ork::Staging`], M1.9c, ADR-076)
//! flies it: the sustainer's apogee and largest speed are compared with OpenRocket's, whose
//! record holds the flight of the branch that keeps the nose. hpr's descent of a separated body
//! needs a device on each (ADR-074), so the sustainer tumbles from its apogee and the booster from
//! the separation; neither changes the climb that is compared. The configurations the record holds that hpr does not fly are listed with
//! the importer's reason.
//!
//! The motor curves come from OpenRocket's own database by digest (ADR-067), whose record lives
//! under the gitignored `corpus-out/`, and the examples from the pinned jar: so the flights run
//! only where both are fetched. What they write, [`REPORT_JSON`] and [`REPORT_MD`], is committed,
//! and a test holds its reference values to the record and its outcomes to [`compare`] in CI.

use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::FRAC_PI_2;
use std::fs;
use std::path::Path;

use hpr_aero::{DragTable, Flow};
use hpr_core::DVec3;
use hpr_core::geodesy::Geodetic;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_io::ork;
use hpr_sim::recovery::{Device, DeviceDrag, Trigger};
use hpr_sim::{
    Environment, EventKind, FlightSettings, FlightStep, Observer, Rail, SimError, Simulation,
};
use hpr_validate::flight_metrics::{
    FlightMetric, MetricOutcome, OPENROCKET_MEASURED, ReferenceReading, Tool, compare, definition,
};
use serde_json::{Value, json};

pub const USAGE: &str = "\
  ork-flights [--check | --corpus [RECORD] | --library [--check]]
                           Fly hpr on each configuration of OpenRocket's flight record it
                           flies, and write validation/reports/openrocket-flights.{md,json}.
                           Needs the pinned jar and corpus-out/openrocket-motors.json. --check
                           compares with the committed report instead of writing it.
                           --corpus prints only counts of OpenRocket's flights of the
                           private library, from corpus-out/openrocket-flights.json or
                           another record flights.py wrote. --library flies the
                           private library's configurations from that record and
                           writes validation/reports/openrocket-library-flights.{md,json}
                           under anonymised ids (with --check, compares instead).";

/// OpenRocket's flights (M2.2d1).
pub(crate) const RECORD: &str = "validation/fixtures/ork/openrocket-flights.json";

/// The report, as data.
pub(crate) const REPORT_JSON: &str = "validation/reports/openrocket-flights.json";

/// The report, as a page.
pub(crate) const REPORT_MD: &str = "validation/reports/openrocket-flights.md";

/// The jar the examples are read from.
pub(crate) const JAR: &str = "refs/openrocket/OpenRocket-24.12.jar";

/// The pod probes (M1.13c2), which `validation/oracles/openrocket/pod_probes.py` writes.
pub(crate) const POD_PROBES: &str = "validation/fixtures/ork/pod-flights/";

/// The tilted-rod probes (M2.2e5), which `validation/oracles/openrocket/rod_probes.py` writes.
pub(crate) const ROD_PROBES: &str = "validation/fixtures/ork/rod-flights/";

/// The metrics compared, with their names in the report.
pub(crate) const METRICS: [(FlightMetric, &str); 3] = [
    (FlightMetric::Apogee, "apogee_m"),
    (FlightMetric::MaxSpeed, "max_speed_m_s"),
    (
        FlightMetric::RodClearanceStability,
        "rod_clearance_margin_cal",
    ),
];

/// An apogee difference above this, in per cent of OpenRocket's, needs a written cause (M2.2's
/// parent *done when*).
const APOGEE_CAUSE_PERCENT: f64 = 5.0;

/// How far `--check` lets a regenerated number move, relative.
pub(crate) const CHECK_RELATIVE: f64 = 1e-9;

pub fn run(args: &[String]) -> Result<(), String> {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        [flag] if flag == "--corpus" => return crate::ork_corpus_flights::run(None),
        [flag] if flag == "--library" => return crate::ork_library_flights::run(false),
        [flag, check] if flag == "--library" && check == "--check" => {
            return crate::ork_library_flights::run(true);
        }
        [flag, record] if flag == "--corpus" => {
            return crate::ork_corpus_flights::run(Some(record));
        }
        _ => return Err(format!("usage:\n{USAGE}")),
    };
    let root = crate::ork::root()?;
    let record = read_json(&root.join(RECORD))?;
    let report = fly_all(&root, &record)?;
    let page = page(&report);
    if check {
        let committed = read_json(&root.join(REPORT_JSON))?;
        let mut apart = Vec::new();
        same(&committed, &report, "", &mut apart);
        let committed_page = fs::read_to_string(root.join(REPORT_MD))
            .map_err(|error| format!("{REPORT_MD}: {error}"))?;
        if committed_page.replace("\r\n", "\n") != page {
            apart.push(format!("{REPORT_MD} differs from what the flights write"));
        }
        if !apart.is_empty() {
            return Err(format!(
                "the committed report is stale; run `cargo xtask ork-flights`:\n  {}",
                apart.join("\n  ")
            ));
        }
        println!("ork-flights: the committed report matches the flights");
    } else {
        let text = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())? + "\n";
        fs::write(root.join(REPORT_JSON), text)
            .map_err(|error| format!("{REPORT_JSON}: {error}"))?;
        fs::write(root.join(REPORT_MD), &page).map_err(|error| format!("{REPORT_MD}: {error}"))?;
        println!("ork-flights: wrote {REPORT_MD} and {REPORT_JSON}");
    }
    print!("{}", summary_lines(&report));
    Ok(())
}

pub(crate) fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))
}

/// Flies every configuration of the record that hpr flies, and builds the report.
fn fly_all(root: &Path, record: &Value) -> Result<Value, String> {
    let jar = root.join(JAR);
    if !jar.is_file() {
        return Err(format!(
            "{JAR} is missing: fetch it with `cargo xtask refs fetch`"
        ));
    }
    let examples = crate::ork::examples_in_jar(&jar)?;
    let supply = crate::ork_supply::Supply::load(root, true)?;
    if !supply.is_present() {
        return Err(format!(
            "{} is missing: run validation/oracles/openrocket/motor_database.py (ADR-067)",
            crate::ork_supply::RECORD
        ));
    }
    if let Some(failure) = supply.failure() {
        return Err(format!("the supplied curves fail their checks: {failure}"));
    }
    let designs = record["designs"]
        .as_array()
        .ok_or_else(|| format!("{RECORD} has no `designs` list"))?;
    let mut flights = Vec::new();
    let mut not_flown = Vec::new();
    let mut probes = Vec::new();
    for entry in designs {
        let file = entry["file"].as_str().ok_or("a design without a file")?;
        let probe = file.starts_with(POD_PROBES) || file.starts_with(ROD_PROBES);
        let Some(recorded) = entry["flights"].as_array() else {
            if probe {
                return Err(format!("OpenRocket flew nothing of the probe {file}"));
            }
            continue;
        };
        let bytes = match file.split_once('!') {
            Some((_, inside)) => examples
                .iter()
                .find(|(name, _)| name.ends_with(&format!("!{inside}")))
                .map(|(_, bytes)| bytes.clone())
                .ok_or_else(|| format!("{inside} is not in {JAR}"))?,
            None => fs::read(root.join(file)).map_err(|error| format!("{file}: {error}"))?,
        };
        let digest = crate::ork_supply::sha256(&bytes);
        if Some(digest.as_str()) != entry["sha256"].as_str() {
            return Err(format!(
                "{file} is not the file the record flew (SHA-256 differs)"
            ));
        }
        let name = design_name(file);
        let label =
            |_: usize, flight: &Value| flight["name"].as_str().unwrap_or_default().to_owned();
        let flown = fly_design(file, &name, &bytes, recorded, &supply, label, &Mode::Public)?;
        if probe {
            // A probe is not a design: it is listed apart, out of the designs' statistics, and
            // it exists to be flown.
            if let Some(refused) = flown.not_flown.first() {
                return Err(format!("the probe {name} is not flown: {}", refused["why"]));
            }
            if flown.flights.is_empty() {
                return Err(format!("the probe {name} has no flight"));
            }
            probes.extend(flown.flights);
            continue;
        }
        flights.extend(flown.flights);
        not_flown.extend(flown.not_flown);
    }
    if !probes.is_empty() && !probes.iter().any(|probe| probe["design"] == WITHOUT_PODS) {
        return Err(format!(
            "the probes have no {WITHOUT_PODS} to measure the pods and the rods against"
        ));
    }
    let summary = summarise(&flights, &not_flown);
    Ok(json!({
        "generated_by": "cargo xtask ork-flights",
        "record": RECORD,
        "reference": { "tool": "OpenRocket", "version": OPENROCKET_MEASURED },
        "summary": summary,
        "flights": flights,
        "not_flown": not_flown,
        "probes": probes,
    }))
}

/// The probe of the pods' airframe alone, which the others' pods are measured against.
pub(crate) const WITHOUT_PODS: &str = "pods-none";

/// What a probe's pods or rod change, against [`WITHOUT_PODS`]: the apogee in per cent and the
/// margin at rod clearance in calibres, each as `[openrocket, hpr]`.
pub(crate) fn probe_change(probe: &Value, without: &Value) -> Option<[[f64; 2]; 2]> {
    let number =
        |flight: &Value, metric: &str, code: &str| flight["metrics"][metric][code].as_f64();
    let mut change = [[0.0; 2]; 2];
    for (at, code) in ["openrocket", "hpr"].into_iter().enumerate() {
        let apogee = number(probe, "apogee_m", code)? / number(without, "apogee_m", code)?;
        change[0][at] = 100.0 * (apogee - 1.0);
        change[1][at] = number(probe, "rod_clearance_margin_cal", code)?
            - number(without, "rod_clearance_margin_cal", code)?;
    }
    Some(change)
}

/// What hpr made of one design's recorded flights.
pub(crate) struct Flown {
    /// The flights hpr flew, each compared with OpenRocket's.
    pub flights: Vec<Value>,
    /// The configurations OpenRocket flew that hpr did not, with why.
    pub not_flown: Vec<Value>,
}

/// Flies every powered configuration of `recorded`, OpenRocket's flights of the design `file`
/// whose bytes are `bytes`, that hpr flies, under the name `name`. `label` names a configuration
/// in the report from its place among the design's recorded configurations (from 1) and its
/// record.
///
/// A configuration is flown only when OpenRocket is shown to fly the curves hpr is given: each
/// motor's curve is the design's own embedded one or one supplied for its digest, and the motor
/// record ([`crate::ork_supply::RECORD`]) finds OpenRocket placing those digests in that
/// configuration. A curve from hpr's bundled catalog is found by name, which can name another
/// curve than the digest OpenRocket loads, and OpenRocket's loader takes another curve without
/// saying so in the flight record when a digest is not in its database.
///
/// A record that lacks what a comparison needs is an error. So is a flight hpr fails, unless
/// flying the library ([`Mode::Library`]), when the configuration is listed as [`FLIGHT_FAILED`]
/// and the detail, which can name a part, is printed on this machine only.
pub(crate) fn fly_design(
    file: &str,
    name: &str,
    bytes: &[u8],
    recorded: &[Value],
    supply: &crate::ork_supply::Supply,
    label: impl Fn(usize, &Value) -> String,
    mode: &Mode,
) -> Result<Flown, String> {
    let list_failures = matches!(mode, Mode::Library { .. });
    let read = ork::read(bytes).map_err(|error| format!("{name}: {error}"))?;
    let design = ork::design_with(&read.value, supply.curves()).value;
    let sha = crate::ork_supply::sha256(bytes);
    let curves = match mode {
        Mode::Library { drag_curves } => drag_curves["designs"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|entry| entry["sha256"].as_str() == Some(sha.as_str())),
        Mode::Public => None,
    };
    let overrides = drag_overrides(&read.value.document.root);
    let powered: Vec<(usize, &Value)> = recorded
        .iter()
        .enumerate()
        .filter(|(_, f)| f["has_motors"] == true)
        .map(|(index, f)| (index + 1, f))
        .collect();
    let mut out = Flown {
        flights: Vec::new(),
        not_flown: Vec::new(),
    };
    for &(place, flight) in &powered {
        let id = flight["configuration"]
            .as_str()
            .ok_or("a flight without a configuration")?;
        let motors = label(place, flight);
        let not_flown = |why: &str| {
            json!({
                "file": file,
                "design": name,
                "configuration": id,
                "motors": motors,
                "aborted": flight["aborted"],
                "why": why,
            })
        };
        if flight["refused"].is_string() {
            out.not_flown.push(not_flown(REFUSED));
            continue;
        }
        // OpenRocket gives a configuration whose id is not a UUID a new one, so a design's
        // only configuration is matched to the record's only powered flight.
        let configurations = &design.motors.configurations;
        let (matched, renamed) = match configurations
            .iter()
            .find(|c| c.id.eq_ignore_ascii_case(id))
        {
            Some(configuration) => (Some(configuration), false),
            None if configurations.len() == 1 && powered.len() == 1 => {
                (configurations.first(), true)
            }
            None => (None, false),
        };
        let flown = matched.and_then(|matched| {
            design
                .rocket
                .configurations
                .iter()
                .find(|c| c.id == matched.id)
        });
        let unconfirmed = match matched.filter(|_| flown.is_some()) {
            Some(matched) => match unflown_conditions(&flight["conditions"])
                .map_err(|error| format!("{motors}: {error}"))?
            {
                Some(why) => Some(why),
                None => unconfirmed_curve(
                    &curve_sources(matched),
                    supply
                        .placed(&sha, id)
                        .map_err(|error| format!("{name}: {error}"))?,
                ),
            },
            None => None,
        };
        let Some(configuration) = flown.filter(|_| unconfirmed.is_none()) else {
            let why = match (flown, unconfirmed) {
                (Some(_), Some(why)) => why.to_owned(),
                _ => matched.and_then(|c| c.left_out.as_ref()).map_or_else(
                    || NO_SUCH_CONFIGURATION.to_owned(),
                    |out| {
                        let prefix = if renamed { RENAMED } else { "" };
                        format!("{prefix}{}", crate::ork_motors::not_flown(out.why))
                    },
                ),
            };
            out.not_flown.push(not_flown(&why));
            continue;
        };
        recorded_enough(flight).map_err(|error| format!("{motors}: {error}"))?;
        let staging = matched.and_then(|matched| matched.staging.as_ref());
        let flew_as = |rocket: &hpr_design::Rocket, drag: Drag| {
            let flown = (name, rocket, &configuration.id[..], staging);
            fly(flown, drag, &motors, flight).map_err(|error| {
                if list_failures {
                    eprintln!("{motors}: {FLIGHT_FAILED}: {error}");
                }
                error
            })
        };
        let flew = |rocket: &hpr_design::Rocket| flew_as(rocket, Drag::Own);
        let mut entry = match flew(&design.rocket) {
            Ok(entry) => entry,
            Err(_) if list_failures => {
                out.not_flown.push(not_flown(FLIGHT_FAILED));
                continue;
            }
            Err(error) => return Err(error),
        };
        entry["file"] = json!(file);
        if !overrides.is_empty() {
            entry["drag_overrides_not_applied"] =
                json!(overrides.iter().map(|o| &o.name).collect::<Vec<_>>());
        }
        if !overrides.is_empty() && overrides.iter().all(|o| o.zero && o.removable) {
            // hpr has no drag override yet (#165), so it charges these parts the drag OpenRocket
            // is told is zero. The same flight with them removed (their mass and lift go too) is
            // a probe of what that costs, not the override itself. A part stating another value
            // has no such probe: removing it would not stand in for its override. Nor has the
            // rocket or a stage stating one.
            let mut without = design.rocket.clone();
            let mut removed = 0;
            for stage in &mut without.stages {
                removed += remove(&mut stage.components, &overrides);
            }
            if removed != overrides.len() {
                return Err(format!(
                    "{name}: removed {removed} of the {} parts set to no drag",
                    overrides.len()
                ));
            }
            let probe = match flew(&without) {
                Ok(probe) => probe,
                Err(_) if list_failures => {
                    out.not_flown.push(not_flown(FLIGHT_FAILED));
                    continue;
                }
                Err(error) => return Err(error),
            };
            entry["without_the_overridden_parts"] = json!({
                "apogee_m": probe["metrics"]["apogee_m"],
                "max_speed_m_s": probe["metrics"]["max_speed_m_s"],
            });
        }
        if let Some(sized) =
            causes_removed(flight, &entry, &overrides).map_err(|e| format!("{motors}: {e}"))?
        {
            entry["apogee_with_the_causes_removed"] = sized;
        }
        let off = entry["metrics"]["apogee_m"]["relative_percent"]
            .as_f64()
            .is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT);
        if off && entry["aborted"] != true && cause(&entry, FlightMetric::Apogee) == NO_NAMED_CAUSE
        {
            // An apogee off by more than the bar with neither cause above: hpr flies it again on
            // OpenRocket's own drag, where the library's record has it (ADR-097). Within the bar,
            // the drag is its named cause. Flown also with only OpenRocket's base drag under
            // power, which says how much of it that one rule is.
            let table = curves
                .map(|curves| openrocket_drag(curves, id))
                .transpose()
                .map_err(|error| format!("{motors}: {error}"))?
                .flatten()
                .filter(|_| staging.is_none());
            let probes = [
                (WHOLE_BASE_PROBE, Some(Drag::WholeBase)),
                (OPENROCKET_DRAG_PROBE, table.map(Drag::OpenRocket)),
            ];
            let mut failed = false;
            for (key, drag) in probes {
                let Some(drag) = drag else { continue };
                match flew_as(&design.rocket, drag) {
                    Ok(probe) => {
                        entry[key] = json!({
                            "apogee_m": probe["metrics"]["apogee_m"],
                            "max_speed_m_s": probe["metrics"]["max_speed_m_s"],
                        });
                    }
                    Err(_) if list_failures => failed = true,
                    Err(error) => return Err(error),
                }
            }
            if failed {
                out.not_flown.push(not_flown(FLIGHT_FAILED));
                continue;
            }
        }
        out.flights.push(entry);
    }
    Ok(out)
}

/// Checks the record holds everything [`fly`] reads from it, so that an error from [`fly`] is
/// hpr's, not the record's.
fn recorded_enough(flight: &Value) -> Result<(), String> {
    let number = |value: &Value, what: &str| {
        value
            .as_f64()
            .ok_or_else(|| format!("the record has no {what}"))
    };
    let conditions = &flight["conditions"];
    Geodetic::from_degrees(
        number(&conditions["launch_latitude_deg"], "latitude")?,
        number(&conditions["launch_longitude_deg"], "longitude")?,
        number(&conditions["launch_altitude_m"], "launch altitude")?,
    )
    .map_err(|error| error.to_string())?;
    number(&conditions["rod_length_m"], "rod length")?;
    number(&conditions["rod_angle_rad"], "rod angle")?;
    number(&conditions["rod_direction_rad"], "rod direction")?;
    number(&flight["rod_clearance"]["time_s"], "rod-clearance time")?;
    number(
        &flight["rod_clearance"]["mach"],
        "rod-clearance Mach number",
    )?;
    early_chute(flight)?;
    Ok(())
}

/// Why a configuration is not flown: OpenRocket refused to fly it.
pub(crate) const REFUSED: &str = "OpenRocket refused to fly it";

/// Why a configuration is not flown: hpr's flight of it failed.
pub(crate) const FLIGHT_FAILED: &str = "hpr's flight of it failed";

/// Why a configuration is not flown: the importer builds none of the recorded id.
pub(crate) const NO_SUCH_CONFIGURATION: &str = "the importer builds no configuration of that id";

/// Put before the importer's reason when OpenRocket gave the design's only configuration a new id.
pub(crate) const RENAMED: &str =
    "the design's only configuration, which OpenRocket gave a new id: ";

/// Why a configuration is not flown: a launch rod hpr's rail does not take.
pub(crate) const ROD_NOT_TAKEN: &str =
    "a launch rod tilted outside 0 up to 90 degrees from the vertical";

/// Why a configuration is not flown: wind.
pub(crate) const WIND: &str = "wind";

/// Why a configuration is not flown: an atmosphere other than the standard one.
pub(crate) const NOT_STANDARD_AIR: &str = "an atmosphere other than the standard one";

/// Why a configuration is not flown: a curve from hpr's bundled catalog, found by name.
pub(crate) const CURVE_BY_NAME: &str = "a curve found by name, which OpenRocket may not fly";

/// Why a configuration is not flown: the motor record does not find OpenRocket placing its curves.
pub(crate) const NOT_PLACED: &str = "a curve OpenRocket is not shown to place";

/// Why the recorded launch `conditions` are not ones this report flies: it flies a rod tilted
/// from 0 up to 90 degrees from the vertical ([`rod_rail`]) in calm standard air only, as [`fly`]
/// requires. A condition the record does not state is an error, not a reason.
pub(crate) fn unflown_conditions(conditions: &Value) -> Result<Option<&'static str>, String> {
    let number = |key: &str| {
        conditions[key]
            .as_f64()
            .ok_or_else(|| format!("the record states no `{key}`"))
    };
    let (rod, wind, turbulence) = (
        number("rod_angle_rad")?,
        number("wind_average_m_s")?,
        number("wind_turbulence")?,
    );
    let (model, standard) = (
        conditions["wind_model"]
            .as_str()
            .ok_or("the record states no `wind_model`")?,
        conditions["isa_atmosphere"]
            .as_bool()
            .ok_or("the record states no `isa_atmosphere`")?,
    );
    Ok(if !(0.0..FRAC_PI_2).contains(&rod) {
        Some(ROD_NOT_TAKEN)
    } else if wind != 0.0 || turbulence != 0.0 || model != "AVERAGE" {
        // `flights.py` calms the average wind model only.
        Some(WIND)
    } else if !standard {
        Some(NOT_STANDARD_AIR)
    } else {
        None
    })
}

/// The rail that OpenRocket's recorded launch rod stands for (M2.2e5, issue #173).
///
/// OpenRocket's `rod_angle_rad` is the rod's angle from the vertical and `rod_direction_rad` the
/// compass bearing it leans toward: `conditions.py` measured that a rocket from a rod tilted
/// toward 0 lands north of the pad and one tilted toward π/2 lands east
/// (`validation/fixtures/ork/openrocket-conditions.json`). hpr's rail takes the same bearing,
/// clockwise from true north, and its angle above the horizon, `π/2` less OpenRocket's. The rail
/// is frictionless and the rocket unrolled on it, as the vertical rod was flown before.
///
/// # Errors
///
/// When the record states no rod length, angle or direction, or the rail refuses them.
pub(crate) fn rod_rail(conditions: &Value) -> Result<Rail, String> {
    let number = |key: &str| {
        conditions[key]
            .as_f64()
            .ok_or_else(|| format!("the record states no `{key}`"))
    };
    let rail = Rail {
        azimuth_rad: number("rod_direction_rad")?,
        elevation_rad: FRAC_PI_2 - number("rod_angle_rad")?,
        ..Rail::vertical(number("rod_length_m")?)
    };
    rail.validate().map_err(|error| error.to_string())?;
    Ok(rail)
}

/// Where the curve of one motor hpr flies came from, as [`unconfirmed_curve`] needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurveSource<'a> {
    /// The design's own embedded curve, or one supplied for the motor's digest: the digest.
    Digest(&'a str),
    /// hpr's bundled catalog, found by name.
    Catalog,
    /// Anything else, such as a curve with no digest.
    Other,
}

/// The curve sources of the motors of `configuration`.
fn curve_sources(configuration: &ork::MotorConfiguration) -> Vec<CurveSource<'_>> {
    configuration
        .motors
        .iter()
        .map(|motor| match (&motor.curve, &motor.digest) {
            (ork::Curve::Embedded { .. } | ork::Curve::Supplied { .. }, Some(digest)) => {
                CurveSource::Digest(digest)
            }
            (ork::Curve::Catalog { .. }, _) => CurveSource::Catalog,
            _ => CurveSource::Other,
        })
        .collect()
}

/// Why OpenRocket is not shown to fly the curves hpr flies, given each motor's [`CurveSource`] and
/// the digests the motor record finds OpenRocket placing in that configuration (`None` when it
/// finds no such configuration): nothing when every curve has a digest and OpenRocket places
/// exactly those digests, as many times each.
pub(crate) fn unconfirmed_curve(
    curves: &[CurveSource<'_>],
    placed: Option<&[String]>,
) -> Option<&'static str> {
    if curves.contains(&CurveSource::Catalog) {
        return Some(CURVE_BY_NAME);
    }
    let Some(placed) = placed else {
        return Some(NOT_PLACED);
    };
    let mut left: Vec<&str> = placed.iter().map(String::as_str).collect();
    for curve in curves {
        let CurveSource::Digest(digest) = curve else {
            return Some(NOT_PLACED);
        };
        match left.iter().position(|d| d == digest) {
            Some(at) => {
                left.swap_remove(at);
            }
            None => return Some(NOT_PLACED),
        }
    }
    if left.is_empty() {
        None
    } else {
        Some(NOT_PLACED)
    }
}

/// A design's name in the report: an example's file name, or a demo's.
fn design_name(file: &str) -> String {
    let base = file.rsplit(['/', '!']).next().unwrap_or(file);
    base.strip_suffix(".ork").unwrap_or(base).to_owned()
}

/// A part of the design (not of its stored simulations) that states its own drag coefficient.
#[derive(Clone)]
pub(crate) struct DragOverride {
    /// The part's id.
    pub id: String,
    /// The part's name.
    pub name: String,
    /// Whether the coefficient stated is zero.
    pub zero: bool,
    /// Whether it is a part, which can be removed, rather than the rocket or a stage.
    pub removable: bool,
}

/// The parts of the design that state a drag coefficient (`<overridecd>`), which hpr reads but
/// does not apply (#165).
pub(crate) fn drag_overrides(root: &ork::Element) -> Vec<DragOverride> {
    fn walk(element: &ork::Element, found: &mut Vec<DragOverride>) {
        let text = |tag: &str| {
            element
                .child(tag)
                .map(|e| e.text().trim().to_owned())
                .unwrap_or_default()
        };
        if element.child("overridecd").is_some() {
            found.push(DragOverride {
                id: text("id"),
                name: text("name"),
                zero: text("overridecd").parse::<f64>() == Ok(0.0),
                removable: !matches!(element.name.as_str(), "rocket" | "stage"),
            });
        }
        element.elements().for_each(|child| walk(child, found));
    }
    let mut found = Vec::new();
    for rocket in root.children_named("rocket") {
        walk(rocket, &mut found);
    }
    found
}

/// Removes the components whose ids are in `parts`, wherever they are, and counts them.
fn remove(components: &mut Vec<hpr_design::tree::Component>, parts: &[DragOverride]) -> usize {
    let before = components.len();
    components.retain(|component| !parts.iter().any(|part| part.id == component.id));
    let mut removed = before - components.len();
    for component in components {
        removed += remove(&mut component.children, parts);
    }
    removed
}

/// The centre of mass's height and place at the start, and its largest speed up to apogee, from
/// the dense output. hpr flies no recovery from a `.ork`, so its fall is unbraked and is left out:
/// the reference's peak speed is on the way up, and a free fall could outrun it.
///
/// Apogee is the highest sample, and the largest speed is taken over every sample up to it. An
/// earlier rule stopped at the first sample whose vertical speed was not positive after one that
/// was, and a rocket held on the pad can show a vertical speed of a few µm/s either way (the
/// held state in Earth's frame): on one private flight it stopped before liftoff, and the
/// largest speed read 100% low.
#[derive(Default)]
struct Peaks {
    start_height_m: Option<f64>,
    start_enu_m: Option<DVec3>,
    /// The largest speed up to the highest sample so far, m/s.
    max_speed_m_s: Option<f64>,
    /// The largest speed so far, m/s.
    running_speed_m_s: f64,
    highest_m: Option<f64>,
}

impl Observer for Peaks {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let (start, end) = (step.start_s(), step.end_s());
        if self.start_height_m.is_none() {
            let sample = step.sample(start)?;
            self.start_height_m = Some(sample.height_above_ground_m);
            self.start_enu_m = Some(sample.cg_enu_m);
        }
        for k in 1..=4 {
            let t = if k == 4 {
                end
            } else {
                start + (end - start) * f64::from(k) / 4.0
            };
            let sample = step.sample(t)?;
            self.running_speed_m_s = self
                .running_speed_m_s
                .max(sample.cg_velocity_enu_m_s.length());
            if self
                .highest_m
                .is_none_or(|highest| sample.height_above_ground_m > highest)
            {
                self.highest_m = Some(sample.height_above_ground_m);
                self.max_speed_m_s = Some(self.running_speed_m_s);
            }
        }
        Ok(())
    }
}

/// `simulation` with `staging`'s separation, the booster tumbling from it and the sustainer from
/// its apogee: hpr's descent of a separated body needs a device on each (ADR-074).
fn staged(simulation: Simulation, staging: &ork::Staging) -> Result<Simulation, SimError> {
    let assembly = simulation.assembly();
    let separation = hpr::ork::separation(staging, assembly)?;
    let last = assembly.layout.stages.len().saturating_sub(1);
    let sustainer = DeviceDrag::tumbling_stages(assembly, (0, staging.after_stage))?;
    let booster = DeviceDrag::tumbling_stages(assembly, (staging.after_stage + 1, last))?;
    simulation
        .with_recovery(vec![
            Device::new("the sustainer, tumbling", sustainer, Trigger::Apogee),
            Device::new(
                "the booster, tumbling",
                booster,
                Trigger::Time { time_s: 0.0 },
            )
            .on_body(1),
        ])?
        .with_separation(separation)
}

/// How [`fly_design`] flies: the public designs, where a flight hpr fails stops the report, or the
/// private library, where it is listed, with OpenRocket's drag curves ([`DRAG_CURVES`]) to size a
/// cause in the drag.
pub(crate) enum Mode<'a> {
    /// The public designs.
    Public,
    /// The private library, with the record `drag_curves.py` wrote of it.
    Library {
        /// That record.
        drag_curves: &'a Value,
    },
}

/// Where `drag_curves.py` writes OpenRocket's drag along its flights of the private library.
pub(crate) const DRAG_CURVES: &str = "corpus-out/openrocket-drag-curves.json";

/// The drag hpr flies a configuration on.
enum Drag {
    /// Its own buildup.
    Own,
    /// Its own buildup with the base's whole drag kept while a motor burns, as OpenRocket's
    /// ([`Simulation::with_full_base_drag_under_power`]).
    WholeBase,
    /// OpenRocket's drag coefficient along its own flight ([`openrocket_drag`]).
    OpenRocket(DragTable),
}

/// OpenRocket's drag along its flight of `configuration`, from the design's entry in
/// [`DRAG_CURVES`], as a table hpr can fly: power on while a motor burns, power off after, each
/// linear in Mach number and held at its ends, on OpenRocket's reference diameter. `None` when the
/// record has no such flight, or OpenRocket refused or aborted it, or it has more than one branch
/// (a separation: the curves would join two shapes), or a curve has fewer than two points.
fn openrocket_drag(design: &Value, configuration: &str) -> Result<Option<DragTable>, String> {
    let Some(flight) = design["flights"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|flight| {
            flight["configuration"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(configuration))
        })
    else {
        return Ok(None);
    };
    if !flight["refused"].is_null() || flight["aborted"] != false || flight["branches"] != 1 {
        return Ok(None);
    }
    let curve = |key: &str| -> Result<Option<Table1D>, String> {
        let points = flight[key]
            .as_array()
            .ok_or_else(|| format!("{DRAG_CURVES} has no {key} curve"))?;
        if points.len() < 2 {
            return Ok(None);
        }
        let mut xs = Vec::with_capacity(points.len());
        let mut ys = Vec::with_capacity(points.len());
        for point in points {
            let (Some(mach), Some(cd)) = (point[0].as_f64(), point[1].as_f64()) else {
                return Err(format!(
                    "{DRAG_CURVES} has a {key} point that is not two numbers"
                ));
            };
            xs.push(mach);
            ys.push(cd);
        }
        Table1D::new(xs, ys, Interpolation::Linear, Extrapolation::Clamp)
            .map(Some)
            .map_err(|error| format!("{DRAG_CURVES}'s {key} curve: {error}"))
    };
    let (Some(off), Some(on)) = (curve("power_off")?, curve("power_on")?) else {
        return Ok(None);
    };
    let reference = flight["reference_length_m"]
        .as_f64()
        .ok_or_else(|| format!("{DRAG_CURVES} has no reference length"))?;
    Ok(Some(
        DragTable::new(off, Some(on)).with_reference_diameter_m(reference),
    ))
}

/// Flies one configuration (the design's name, its rocket, the configuration's id and its
/// staging) in the recorded conditions, with its powered separation if it has one, on `drag`, and
/// compares it with the record.
fn fly(
    (design, rocket, configuration, staging): (
        &str,
        &hpr_design::Rocket,
        &str,
        Option<&ork::Staging>,
    ),
    drag: Drag,
    motors: &str,
    recorded: &Value,
) -> Result<Value, String> {
    let at = format!("{design} {motors}");
    let number = |value: &Value, what: &str| {
        value
            .as_f64()
            .ok_or_else(|| format!("{at}: the record has no {what}"))
    };
    let conditions = &recorded["conditions"];
    let clearance = &recorded["rod_clearance"];
    let aborted = recorded["aborted"] == true;
    let site = Geodetic::from_degrees(
        number(&conditions["launch_latitude_deg"], "latitude")?,
        number(&conditions["launch_longitude_deg"], "longitude")?,
        number(&conditions["launch_altitude_m"], "launch altitude")?,
    )
    .map_err(|error| format!("{at}: {error}"))?;
    if number(&conditions["wind_average_m_s"], "wind")? != 0.0
        || conditions["isa_atmosphere"] != true
    {
        return Err(format!("{at}: only calm standard air is flown here"));
    }
    let rail = rod_rail(conditions).map_err(|error| format!("{at}: {error}"))?;
    let rod_length_m = rail.length_m;
    let environment = Environment::standard(site).map_err(|error| format!("{at}: {error}"))?;
    // OpenRocket flies a design whatever hpr's checks find in it, so hpr does too, and the report
    // lists what they found.
    let findings = hpr_design::checks::check(rocket).map_err(|error| format!("{at}: {error}"))?;
    let design_errors: Vec<_> = findings
        .iter()
        .filter(|finding| finding.severity() == hpr_design::checks::Severity::Error)
        .collect();
    let settings = FlightSettings {
        accept_design_errors: true,
        ..FlightSettings::default()
    };
    let simulation = Simulation::new(rocket, configuration, environment, rail, settings)
        .map_err(|error| format!("{at}: {error}"))?;
    let simulation = match staging {
        Some(staging) => staged(simulation, staging).map_err(|error| format!("{at}: {error}"))?,
        None => simulation,
    };
    let simulation = match drag {
        Drag::Own => simulation,
        Drag::WholeBase => simulation.with_full_base_drag_under_power(),
        Drag::OpenRocket(table) => simulation.with_drag_table(table),
    };
    let mut peaks = Peaks::default();
    let result = simulation
        .run(&mut peaks)
        .map_err(|error| format!("{at}: {error}"))?;
    // OpenRocket's altitude is 0 at launch; hpr's centre of mass starts above the ground (the
    // rocket stands on the rail's foot), so hpr's apogee is counted from where it starts.
    let apogee_m = result
        .event(EventKind::Apogee)
        .zip(peaks.start_height_m)
        .map(|(event, start)| event.sample.height_above_ground_m - start);
    // Where the rocket is at apogee, east and north of where it started, which OpenRocket's
    // position at its highest row is (its position is 0 at launch): what says a tilted rod sent it
    // the same way in both codes.
    let apogee_moved_m = result
        .event(EventKind::Apogee)
        .zip(peaks.start_enu_m)
        .map(|(event, start)| event.sample.cg_enu_m - start);

    // The margin at the recorded rod-clearance step: hpr's mass at its time, and its centre of
    // pressure at its Mach number with the air along the axis.
    let assembly = simulation.assembly();
    let clearance_time_s = number(&clearance["time_s"], "rod-clearance time")?;
    let clearance_mach = number(&clearance["mach"], "rod-clearance Mach number")?;
    // Before any separation, so a motor waiting on one counts as unlit.
    let lit = assembly.ignition_times_s(|_| None);
    let apogee_s = result
        .event(EventKind::Apogee)
        .map(|apogee| apogee.sample.time_s)
        .ok_or_else(|| format!("{at}: hpr's flight has no apogee"))?;
    let burns: Vec<_> = assembly
        .motors
        .iter()
        .map(|motor| (motor.mounted.motor.burnout_time_s(), motor.fails))
        .collect();
    spent_by_apogee(&lit, &burns, apogee_s).map_err(|error| format!("{at}: {error}"))?;
    let mass = assembly.mass_properties_lit(clearance_time_s, &lit);
    let cg_m = -mass.cg_m.z;
    let cp_m = simulation
        .aero()
        .normal_force(&Flow::axial(clearance_mach))
        .map_err(|error| format!("{at}: {error}"))?
        .cp_station_m;
    let reference_m = assembly.layout.reference_diameter_m;
    let margin_cal = cp_m.map(|cp| (cp - cg_m) / reference_m);
    // OpenRocket's rocket can reach the rod-clearance row at an angle of attack, more the more
    // its rod is tilted (M2.2e5), where hpr's margin is taken at none: hpr's centre of pressure
    // at OpenRocket's angle sizes what that difference moves.
    let clearance_alpha_rad = clearance["angle_of_attack_rad"]
        .as_f64()
        .filter(|alpha| alpha.is_finite());
    // A diagnostic, so a flow the model refuses leaves it out rather than failing the flight.
    let cp_at_openrocket_alpha_m = clearance_alpha_rad.and_then(|alpha| {
        simulation
            .aero()
            .normal_force(&Flow::new(clearance_mach, alpha, 0.0))
            .ok()
            .and_then(|force| force.cp_station_m)
    });

    let summary = &recorded["summary"];
    let reference_value = |key: &str| {
        if aborted {
            ReferenceReading::Aborted
        } else {
            // OpenRocket's `NaN` is written as JSON null.
            ReferenceReading::Complete(summary[key].as_f64().or(Some(f64::NAN)))
        }
    };
    let references = [
        reference_value("max_altitude_m"),
        reference_value("max_velocity_m_s"),
        if aborted {
            ReferenceReading::Aborted
        } else {
            ReferenceReading::Complete(clearance["stability_cal"].as_f64().or(Some(f64::NAN)))
        },
    ];
    let measured = [apogee_m, peaks.max_speed_m_s, margin_cal];
    let tool = openrocket();
    let mut metrics = serde_json::Map::new();
    for (((metric, key), reference), measured) in METRICS.iter().zip(references).zip(measured) {
        let outcome = compare(&tool, *metric, reference, measured);
        metrics.insert(
            (*key).to_owned(),
            metric_entry(reference, measured, &outcome),
        );
    }

    let deployed_before_apogee_s =
        early_chute(recorded).map_err(|error| format!("{at}: {error}"))?;
    let events = recorded["events"]
        .as_array()
        .ok_or_else(|| format!("{at}: the record has no events"))?;
    let separated_s = |kind: &str| {
        events
            .iter()
            .find(|event| event["type"] == kind)
            .and_then(|event| event["time_s"].as_f64())
    };
    // A staged or clustered flight says so, for M1.9c's tolerance (ADR-076).
    let separated = staging.map(|staging| {
        json!({
            "after_stage": staging.after_stage,
            "separation_s": {
                "openrocket": separated_s("STAGE_SEPARATION"),
                "hpr": result.event(EventKind::Separation).map(|event| event.sample.time_s),
            },
        })
    });
    // The motors in a mount that places more than one, a motor in each tube of a cluster.
    let clusters: BTreeSet<&str> = assembly
        .motors
        .iter()
        .filter(|motor| motor.tube > 0)
        .map(|motor| motor.mounted.mount.as_str())
        .collect();
    let clustered_motors = assembly
        .motors
        .iter()
        .filter(|motor| clusters.contains(motor.mounted.mount.as_str()))
        .count();
    let mut flown = json!({
        "design": design,
        "configuration": recorded["configuration"],
        "motors": motors,
        "aborted": aborted,
        "rod_length_m": rod_length_m,
        "termination": result.termination,
        "design_errors_accepted": design_errors,
        "deployed_before_apogee_s": deployed_before_apogee_s,
        "max_mach_openrocket": recorded["summary"]["max_mach"],
        "metrics": metrics,
        "at_rod_clearance": {
            "time_s": clearance_time_s,
            "mach": clearance_mach,
            "openrocket": {
                "mass_kg": clearance["mass_kg"],
                "cg_from_nose_m": clearance["cg_from_nose_m"],
                "cp_from_nose_m": clearance["cp_from_nose_m"],
                "reference_length_m": clearance["reference_length_m"],
                "angle_of_attack_rad": clearance["angle_of_attack_rad"],
            },
            "hpr": {
                "mass_kg": mass.mass_kg,
                "cg_from_nose_m": cg_m,
                "cp_from_nose_m": cp_m,
                "reference_length_m": reference_m,
                "cp_from_nose_m_at_openrocket_angle_of_attack": cp_at_openrocket_alpha_m,
            },
        },
        "launch_mass_kg": {
            "openrocket": recorded["series"]["launch_mass_kg"],
            "hpr": assembly.mass_properties_lit(0.0, &lit).mass_kg,
        },
        "rod": {
            "angle_from_vertical_rad": conditions["rod_angle_rad"],
            "direction_rad": conditions["rod_direction_rad"],
        },
        "apogee_position_m": {
            "openrocket": {
                "east": recorded["series"]["east_at_max_altitude_m"],
                "north": recorded["series"]["north_at_max_altitude_m"],
            },
            "hpr": {
                "east": apogee_moved_m.map(|moved| moved.x),
                "north": apogee_moved_m.map(|moved| moved.y),
            },
        },
    });
    if let Some(separated) = separated {
        flown["staging"] = separated;
    }
    if clustered_motors > 0 {
        flown["clustered_motors"] = json!(clustered_motors);
    }
    Ok(flown)
}

/// How long before the apogee of the same flight with nothing deployed OpenRocket's first
/// parachute opened, when it opened before the flight's own apogee: the early parachute moves that
/// apogee, not the undeployed one.
fn early_chute(recorded: &Value) -> Result<Option<f64>, String> {
    let empty = Vec::new();
    let events = recorded["events"].as_array().unwrap_or(&empty);
    let first = |kind: &str| {
        events
            .iter()
            .find(|event| event["type"] == kind)
            .and_then(|event| event["time_s"].as_f64())
    };
    match (first("RECOVERY_DEVICE_DEPLOYMENT"), first("APOGEE")) {
        (Some(deployed), Some(apogee)) if deployed < apogee => {
            recorded["undeployed"]["apogee_time_s"]
                .as_f64()
                .map(|undeployed| Some(undeployed - deployed))
                .ok_or_else(|| "the record has no apogee without deployment".to_owned())
        }
        _ => Ok(None),
    }
}

/// OpenRocket's apogee with a flight's named causes taken out of its own flight, and hpr's apogee
/// against it (M2.2e4, decision ADR-073), or `None` for an aborted flight, a flight of hpr's with
/// no apogee (its outcome is scored already), or one with neither cause. `overrides` are the
/// parts that state a drag coefficient hpr reads and does not apply.
///
/// - `parachutes_held`: the record's `undeployed` flight, the same one with nothing deployed. A
///   parachute that opens before apogee lowers the apogee, and hpr flies none from a `.ork`.
/// - `drag_overrides_cleared_too`, where a part states a drag coefficient: its
///   `undeployed_without_drag_overrides` flight, with nothing deployed and every stated coefficient
///   cleared, which is OpenRocket flying what hpr flies, since hpr applies none (#165).
/// - `parts_removed_from_both`, where hpr has flown its probe without the parts set to no drag
///   (`without_the_overridden_parts`): that probe's apogee against OpenRocket's
///   `undeployed_without_parts_set_to_no_drag` flight, the same rocket with the same parts gone
///   and nothing deployed.
///
/// Each is OpenRocket's apogee and hpr's difference in per cent of it, or why there is none (the
/// oracle failed, or OpenRocket refused or aborted the flight). The flights with all the causes
/// taken out are the second where there is one, else the first, and the third where there is one.
/// `remaining_percent` is the largest of hpr's differences from them in size, what the named
/// causes leave, since two differences can cancel in one of them; `within_5_percent` holds when
/// each of them has a difference and every one is within 5%. The record must name the same parts
/// hpr reads as the ones OpenRocket cleared or removed (a part with no id by count only), or the
/// report stops.
fn causes_removed(
    recorded: &Value,
    entry: &Value,
    overrides: &[DragOverride],
) -> Result<Option<Value>, String> {
    let early = !entry["deployed_before_apogee_s"].is_null();
    if entry["aborted"] == true || (!early && overrides.is_empty()) {
        return Ok(None);
    }
    let Some(hpr) = entry["metrics"]["apogee_m"]["hpr"].as_f64() else {
        return Ok(None);
    };
    // A part with no id in the file gets a new one in OpenRocket, so it is matched by count only.
    let ids: BTreeSet<String> = overrides
        .iter()
        .filter(|o| !o.id.is_empty())
        .map(|o| o.id.to_lowercase())
        .collect();
    let step = |key: &str, parts: Option<&str>, measured: f64| -> Result<Value, String> {
        let held = &recorded[key];
        if held.is_null() {
            return Err(format!("the record has no {key} flight"));
        }
        if let Some(error) = held["driver_error"].as_str() {
            return Ok(json!({ "why": format!("the oracle failed: {error}") }));
        }
        if let Some(verb) = parts {
            let changed: BTreeSet<String> = held[verb]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_lowercase)
                .collect();
            if changed.len() != overrides.len() || !ids.is_subset(&changed) {
                return Err(format!(
                    "OpenRocket's {key} flight {verb} {} parts, hpr reads {}, and {} of hpr's \
                     ids are not among them",
                    changed.len(),
                    overrides.len(),
                    ids.difference(&changed).count()
                ));
            }
        }
        if !held["refused"].is_null() {
            return Ok(json!({ "why": "OpenRocket refused it" }));
        }
        if held["aborted"] != false {
            return Ok(json!({ "why": "OpenRocket aborted it" }));
        }
        let openrocket = held["max_altitude_m"]
            .as_f64()
            .filter(|m| m.is_finite() && *m > 0.0)
            .ok_or_else(|| format!("the record's {key} flight has no apogee"))?;
        Ok(json!({
            "openrocket_m": openrocket,
            "relative_percent": 100.0 * (measured - openrocket) / openrocket,
        }))
    };
    let held = step("undeployed", None, hpr)?;
    let mut last = held["relative_percent"].as_f64();
    let mut sized = json!({ "parachutes_held": held });
    if !overrides.is_empty() {
        let too = step("undeployed_without_drag_overrides", Some("cleared"), hpr)?;
        last = too["relative_percent"].as_f64();
        sized["drag_overrides_cleared_too"] = too;
    }
    let mut left = vec![last];
    if let Some(probe) = entry["without_the_overridden_parts"]["apogee_m"]["hpr"].as_f64() {
        let key = "undeployed_without_parts_set_to_no_drag";
        let mut both = step(key, Some("removed"), probe)?;
        both["hpr_m"] = json!(probe);
        left.push(both["relative_percent"].as_f64());
        sized["parts_removed_from_both"] = both;
    }
    let remaining = left
        .iter()
        .flatten()
        .copied()
        .max_by(|a, b| a.abs().total_cmp(&b.abs()));
    let agrees = left
        .iter()
        .all(|p| p.is_some_and(|p| p.abs() <= APOGEE_CAUSE_PERCENT));
    sized["remaining_percent"] = json!(remaining);
    sized["within_5_percent"] = json!(agrees);
    Ok(Some(sized))
}

/// The reference tool.
pub(crate) fn openrocket() -> Tool {
    Tool::OpenRocket {
        version: OPENROCKET_MEASURED.to_owned(),
    }
}

/// One metric's entry: both values, the outcome, and the difference.
pub(crate) fn metric_entry(
    reference: ReferenceReading,
    measured: Option<f64>,
    outcome: &MetricOutcome,
) -> Value {
    let reference = match reference {
        ReferenceReading::Complete(value) => value.filter(|v| v.is_finite()),
        _ => None,
    };
    let measured = measured.filter(|v| v.is_finite());
    let difference = outcome.difference();
    let relative_percent = match (difference, reference) {
        (Some(difference), Some(reference)) if reference != 0.0 => {
            Some(100.0 * difference / reference)
        }
        _ => None,
    };
    json!({
        "openrocket": reference,
        "hpr": measured,
        "outcome": outcome,
        "difference": difference,
        "relative_percent": relative_percent,
    })
}

/// The counts and the spread of each metric's differences.
pub(crate) fn summarise(flights: &[Value], not_flown: &[Value]) -> Value {
    let mut metrics = serde_json::Map::new();
    for (metric, key) in METRICS {
        let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
        let mut groups: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for flight in flights {
            let entry = &flight["metrics"][key];
            let outcome = entry["outcome"]["outcome"].as_str().unwrap_or("missing");
            *outcomes.entry(outcome.to_owned()).or_default() += 1;
            // The margin's difference is in calibres; the others' in per cent of OpenRocket's.
            let value = if metric == FlightMetric::RodClearanceStability {
                entry["difference"].as_f64()
            } else {
                entry["relative_percent"].as_f64()
            };
            if let Some(value) = value {
                groups.entry(cause(flight, metric)).or_default().push(value);
            }
        }
        let unit = if metric == FlightMetric::RodClearanceStability {
            "calibres"
        } else {
            "percent of OpenRocket's"
        };
        let groups: serde_json::Map<String, Value> = groups
            .iter()
            .map(|(cause, values)| ((*cause).to_owned(), spread(values)))
            .collect();
        metrics.insert(
            key.to_owned(),
            json!({
                "definition": definition(&openrocket(), metric),
                "outcomes": outcomes,
                "difference_unit": unit,
                "scored_by_cause": groups,
            }),
        );
    }
    let over = flights
        .iter()
        .filter(|f| {
            f["metrics"]["apogee_m"]["relative_percent"]
                .as_f64()
                .is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT)
        })
        .count();
    json!({
        "flown": flights.len(),
        "not_flown": not_flown.len(),
        "apogee_over_5_percent": over,
        "metrics": metrics,
        "apogee_with_the_causes_removed": with_the_causes_removed(flights),
        "mass_and_cg": mass_and_cg(flights),
    })
}

/// The spread of what the named causes leave of hpr's apogee difference, over the flights that
/// have one (M2.2e4); and, of the flights more than 5% from OpenRocket's, how many have their
/// causes sized (a difference from a flight without them) and how many come within 5% of every
/// flight of OpenRocket's without them.
fn with_the_causes_removed(flights: &[Value]) -> Value {
    let (mut remaining, mut over, mut sized, mut within) = (Vec::new(), 0, 0, 0);
    for flight in flights {
        let removed = &flight["apogee_with_the_causes_removed"];
        remaining.extend(removed["remaining_percent"].as_f64());
        if flight["metrics"]["apogee_m"]["relative_percent"]
            .as_f64()
            .is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT)
        {
            over += 1;
            sized += usize::from(removed["remaining_percent"].is_f64());
            within += usize::from(removed["within_5_percent"] == true);
        }
    }
    json!({
        "remaining_percent": spread(&remaining),
        "over_5_percent": over,
        "over_5_percent_sized": sized,
        "over_5_percent_within_5_percent_after": within,
    })
}

/// The spread of hpr's mass and centre of mass against OpenRocket's (M2.2e1), over the flights
/// that were not aborted: the mass at launch and at the rod-clearance step in per cent of
/// OpenRocket's, and the centre of mass at that step, hpr's less OpenRocket's distance from the
/// nose, in OpenRocket's calibres (so it moves the margin by as much, the other way).
fn mass_and_cg(flights: &[Value]) -> Value {
    let (mut launch, mut clearance, mut cg) = (Vec::new(), Vec::new(), Vec::new());
    for flight in flights.iter().filter(|f| f["aborted"] != true) {
        let [l, c, g] = mass_and_cg_differences(flight);
        launch.extend(l);
        clearance.extend(c);
        cg.extend(g);
    }
    json!({
        "launch_mass_percent": spread(&launch),
        "rod_clearance_mass_percent": spread(&clearance),
        "rod_clearance_cg_cal": spread(&cg),
    })
}

/// One flight's differences [`mass_and_cg`] spreads: its mass at launch and at the rod-clearance
/// step in per cent of OpenRocket's, and its centre of mass at that step in OpenRocket's calibres.
pub(crate) fn mass_and_cg_differences(flight: &Value) -> [Option<f64>; 3] {
    let relative = |hpr: &Value, openrocket: &Value| {
        hpr.as_f64()
            .zip(openrocket.as_f64())
            .map(|(h, o)| 100.0 * (h - o) / o)
            .filter(|p| p.is_finite())
    };
    let at = &flight["at_rod_clearance"];
    let (hpr, openrocket) = (&at["hpr"], &at["openrocket"]);
    [
        relative(
            &flight["launch_mass_kg"]["hpr"],
            &flight["launch_mass_kg"]["openrocket"],
        ),
        relative(&hpr["mass_kg"], &openrocket["mass_kg"]),
        hpr["cg_from_nose_m"]
            .as_f64()
            .zip(openrocket["cg_from_nose_m"].as_f64())
            .zip(openrocket["reference_length_m"].as_f64())
            .map(|((h, o), reference)| (h - o) / reference)
            .filter(|cal| cal.is_finite()),
    ]
}

/// A drag override hpr does not apply.
pub(crate) const DRAG_OVERRIDE: &str = "a part's drag override not applied";

/// A reference parachute open before its apogee.
pub(crate) const EARLY_CHUTE: &str = "reference parachute open before apogee";

/// hpr's drag coefficient, not OpenRocket's: flown on OpenRocket's, hpr's apogee comes within 5%
/// of OpenRocket's (ADR-097).
pub(crate) const OWN_DRAG: &str = "hpr's own drag coefficient";

/// None of these.
pub(crate) const NO_NAMED_CAUSE: &str = "no named cause";

/// Where a flight holds hpr's flight again with only OpenRocket's base drag under power: its
/// apogee and largest speed against OpenRocket's.
pub(crate) const WHOLE_BASE_PROBE: &str = "with_the_whole_base_under_power";

/// Where a flight holds hpr's flight again on OpenRocket's drag coefficient along OpenRocket's
/// flight ([`DRAG_CURVES`]): its apogee and largest speed against OpenRocket's.
pub(crate) const OPENROCKET_DRAG_PROBE: &str = "on_openrocket_s_drag";

/// The named cause a flight's difference in `metric` is summarised under. A drag override moves
/// every metric but the margin; an early parachute only the apogee. A flight with both is put
/// under the drag override. A flight with neither whose apogee hpr brings within 5% of
/// OpenRocket's by flying OpenRocket's drag has hpr's own drag as its cause, for the apogee and the
/// largest speed.
pub(crate) fn cause(flight: &Value, metric: FlightMetric) -> &'static str {
    if metric == FlightMetric::RodClearanceStability {
        NO_NAMED_CAUSE
    } else if !flight["drag_overrides_not_applied"].is_null() {
        DRAG_OVERRIDE
    } else if metric == FlightMetric::Apogee && !flight["deployed_before_apogee_s"].is_null() {
        EARLY_CHUTE
    } else if drag_sizes(flight[OPENROCKET_DRAG_PROBE]["apogee_m"]["relative_percent"].as_f64()) {
        OWN_DRAG
    } else {
        NO_NAMED_CAUSE
    }
}

/// Whether hpr's apogee on OpenRocket's drag, `percent` from OpenRocket's, is within the bar that
/// sizes a cause.
pub(crate) fn drag_sizes(percent: Option<f64>) -> bool {
    percent.is_some_and(|p| p.abs() <= APOGEE_CAUSE_PERCENT)
}

/// How many, the median, the mean, and the smallest and largest of `values`.
pub(crate) fn spread(values: &[f64]) -> Value {
    if values.is_empty() {
        return json!({ "count": 0 });
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    };
    let mean_abs = sorted.iter().map(|v| v.abs()).sum::<f64>() / n as f64;
    json!({
        "count": n,
        "median": median,
        "mean_absolute": mean_abs,
        "min": sorted[0],
        "max": sorted[n - 1],
    })
}

/// The report as a page.
pub(crate) fn page(report: &Value) -> String {
    let mut out = String::new();
    out.push_str("# hpr against OpenRocket 24.12's flights of the public designs\n\n");
    out.push_str(
        "Written by `cargo xtask ork-flights` ([M2.2d2][m2-2d2], hpr's flights against \
         OpenRocket's; decision [ADR-069][adr-069]) from OpenRocket 24.12's calm-air flights in \
         `validation/fixtures/ork/openrocket-flights.json` ([M2.2d1][m2-2d1]). OR is \
         OpenRocket. Each metric is taken by the definition hpr holds for OpenRocket 24.12: \
         the apogee is the highest point above the launch position; the largest speed is the \
         peak speed (hpr's up to its apogee, since it flies no parachute); the margin, in \
         calibres, is OpenRocket's stability column at its rod-clearance step, which hpr takes \
         at that step's time and Mach number with the air along the axis. Differences (Δ) are \
         hpr less OpenRocket. Motors are OpenRocket's configuration names, one bracketed group \
         per stage. The explanation is on the [documentation site][site].\n\n\
         [m2-2d2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d2\n\
         [m2-2d1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d1\n\
         [m2-2e1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e1\n\
         [adr-069]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-069-hprs-flights-of-the-public-designs-against-openrockets-2026-09-25\n\
         [adr-070]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-070-m22e-split-mass-and-centre-of-mass-first-then-the-corpus-2026-09-25\n\
         [site]: https://nrdptel.github.io/hpr-sim/format/ork.html#hprs-flights-against-openrockets\n\n",
    );
    out.push_str(&summary_lines(report));
    out.push('\n');
    out.push_str(
        "| design | motors | apogee OR (m) | hpr (m) | Δ | max speed OR (m/s) | hpr (m/s) | Δ \
         | max Mach OR | margin OR (cal) | hpr (cal) | Δ (cal) |\n",
    );
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    let empty = Vec::new();
    for flight in report["flights"].as_array().unwrap_or(&empty) {
        let m = &flight["metrics"];
        let mut early = flight["deployed_before_apogee_s"]
            .as_f64()
            .map_or(String::new(), |s| format!(" (chute {s:.2} s early)"));
        let probe = &flight["without_the_overridden_parts"];
        if let Some(without) = probe["apogee_m"]["relative_percent"].as_f64() {
            early.push_str(&format!(
                " ({without:+.2}% without the part set to no drag)"
            ));
        }
        let speed_probe = probe["max_speed_m_s"]["relative_percent"]
            .as_f64()
            .map_or(String::new(), |p| {
                format!(" ({p:+.2}% without the part set to no drag)")
            });
        out.push_str(&format!(
            "| {} | {} | {} | {} | {}{} | {} | {} | {}{} | {} | {} | {} | {} |\n",
            flight["design"].as_str().unwrap_or_default(),
            flight["motors"].as_str().unwrap_or_default(),
            fixed(&m["apogee_m"]["openrocket"], 1),
            fixed(&m["apogee_m"]["hpr"], 1),
            percent(&m["apogee_m"]),
            early,
            fixed(&m["max_speed_m_s"]["openrocket"], 2),
            fixed(&m["max_speed_m_s"]["hpr"], 2),
            percent(&m["max_speed_m_s"]),
            speed_probe,
            fixed(&flight["max_mach_openrocket"], 3),
            fixed(&m["rod_clearance_margin_cal"]["openrocket"], 3),
            fixed(&m["rod_clearance_margin_cal"]["hpr"], 3),
            signed(&m["rod_clearance_margin_cal"]["difference"], 4),
        ));
    }
    out.push_str(
        "\n*Chute s early*: OpenRocket's parachute opened that long before the apogee of the \
         same flight with nothing deployed, which the record also holds; hpr flies no parachute \
         from a `.ork` yet. *Without the part set to no drag*: the same flight by hpr with the \
         parts OpenRocket is told have no drag removed, which takes their mass, lift and shape \
         away too, so it is a probe, not the override ([#165][i165]).\n\n\
         [i165]: https://github.com/nrdptel/hpr-sim/issues/165\n",
    );
    out.push_str(&causes_table(report));
    out.push_str(
        "\nAt the rod-clearance step, the parts of the margin (m from the nose tip, and kg):\n\n",
    );
    out.push_str(
        "| design | motors | CG OR | CG hpr | CP OR | CP hpr | reference OR | reference hpr \
         | mass OR | mass hpr |\n",
    );
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for flight in report["flights"].as_array().unwrap_or(&empty) {
        let at = &flight["at_rod_clearance"];
        let (or, hpr) = (&at["openrocket"], &at["hpr"]);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            flight["design"].as_str().unwrap_or_default(),
            flight["motors"].as_str().unwrap_or_default(),
            fixed(&or["cg_from_nose_m"], 4),
            fixed(&hpr["cg_from_nose_m"], 4),
            fixed(&or["cp_from_nose_m"], 4),
            fixed(&hpr["cp_from_nose_m"], 4),
            fixed(&or["reference_length_m"], 4),
            fixed(&hpr["reference_length_m"], 4),
            fixed(&or["mass_kg"], 4),
            fixed(&hpr["mass_kg"], 4),
        ));
    }
    let checked: Vec<&Value> = report["flights"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|f| {
            f["design_errors_accepted"]
                .as_array()
                .is_some_and(|e| !e.is_empty())
        })
        .collect();
    if !checked.is_empty() {
        out.push_str(
            "\nWhat hpr's design checks object to, in flights flown anyway as OpenRocket flies \
             them:\n\n| design | motors | findings |\n|---|---|---|\n",
        );
        for flight in checked {
            out.push_str(&format!(
                "| {} | {} | `{}` |\n",
                flight["design"].as_str().unwrap_or_default(),
                flight["motors"].as_str().unwrap_or_default(),
                flight["design_errors_accepted"],
            ));
        }
    }
    let not_flown = report["not_flown"].as_array().unwrap_or(&empty);
    if !not_flown.is_empty() {
        out.push_str("\nConfigurations OpenRocket flew that hpr does not fly yet:\n\n");
        out.push_str("| design | motors | why |\n|---|---|---|\n");
        for entry in not_flown {
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                entry["design"].as_str().unwrap_or_default(),
                entry["motors"].as_str().unwrap_or_default(),
                entry["why"].as_str().unwrap_or_default(),
            ));
        }
    }
    out.push_str(&probes_table(report));
    out.push_str(&rod_probes_table(report));
    out
}

/// The pod probes' table (M1.13c2): each probe against OpenRocket, and what its pods change against
/// the airframe alone in each code.
fn probes_table(report: &Value) -> String {
    let probes: Vec<&Value> = report["probes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|probe| {
            probe["file"]
                .as_str()
                .is_some_and(|file| file.starts_with(POD_PROBES))
        })
        .collect();
    if probes.is_empty() {
        return String::new();
    }
    let without = probes
        .iter()
        .find(|probe| probe["design"] == WITHOUT_PODS)
        .copied()
        .unwrap_or(&Value::Null);
    let mut out = String::from(
        "\n## Pod probes\n\nSmall designs written to test pods ([M1.13c2][m1-13c2]), not counted \
         among the designs above: one airframe on an AeroTech H128W, carrying pods of bodies, \
         fins, a tail cone, winglets or motors, and once with none (`pods-none`, first). *Pods' \
         change* is the apogee's and the margin's change from `pods-none` in each code. \
         `pods-motors-2` flies an H128W in each of its two pods and none in the airframe, with \
         0.35 kg of nose ballast, not 0.15, so its change is not its pods' alone.\n\n\
         [m1-13c2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-13c2\n\n\
         | probe | apogee OR (m) | hpr (m) | Δ | pods' change OR | hpr | max speed OR (m/s) \
         | hpr (m/s) | Δ | max Mach OR | margin OR (cal) | hpr (cal) | pods' change OR (cal) \
         | hpr (cal) |\n|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    let first = probes
        .iter()
        .filter(|probe| probe["design"] == WITHOUT_PODS);
    let others = probes
        .iter()
        .filter(|probe| probe["design"] != WITHOUT_PODS);
    for probe in first.chain(others) {
        let m = &probe["metrics"];
        let change = probe_change(probe, without).map_or_else(
            || {
                [
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                    "-".to_owned(),
                ]
            },
            |[apogee, margin]| {
                [
                    format!("{:+.2}%", apogee[0]),
                    format!("{:+.2}%", apogee[1]),
                    format!("{:+.4}", margin[0]),
                    format!("{:+.4}", margin[1]),
                ]
            },
        );
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            probe["design"].as_str().unwrap_or_default(),
            fixed(&m["apogee_m"]["openrocket"], 1),
            fixed(&m["apogee_m"]["hpr"], 1),
            percent(&m["apogee_m"]),
            change[0],
            change[1],
            fixed(&m["max_speed_m_s"]["openrocket"], 2),
            fixed(&m["max_speed_m_s"]["hpr"], 2),
            percent(&m["max_speed_m_s"]),
            fixed(&probe["max_mach_openrocket"], 3),
            fixed(&m["rod_clearance_margin_cal"]["openrocket"], 3),
            fixed(&m["rod_clearance_margin_cal"]["hpr"], 3),
            change[2],
            change[3],
        ));
    }
    out
}

/// hpr's margin at rod clearance with its centre of pressure at OpenRocket's angle of attack
/// there, in calibres of hpr's reference diameter: `None` when either is missing.
pub(crate) fn margin_at_openrocket_alpha_cal(flight: &Value) -> Option<f64> {
    let hpr = &flight["at_rod_clearance"]["hpr"];
    let cp = hpr["cp_from_nose_m_at_openrocket_angle_of_attack"].as_f64()?;
    Some((cp - hpr["cg_from_nose_m"].as_f64()?) / hpr["reference_length_m"].as_f64()?)
}

/// `value` to one decimal, with no sign on a value that rounds to zero.
fn tenths(value: f64) -> String {
    // Adding zero turns the `-0.0` a small negative value rounds to into `0.0`.
    format!("{:.1}", (value * 10.0).round() / 10.0 + 0.0)
}

/// The compass bearing of a place `east` and `north` of the pad, in degrees from 0 up to 360.
pub(crate) fn bearing_deg(east: f64, north: f64) -> f64 {
    let bearing = east.atan2(north).to_degrees().rem_euclid(360.0);
    // `rem_euclid` rounds a bearing a hair below 0 up to 360 itself.
    if bearing >= 360.0 { 0.0 } else { bearing }
}

/// The tilted-rod probes' table (M2.2e5): the pod probes' airframe from a vertical rod
/// ([`WITHOUT_PODS`]) and from each tilted rod, against OpenRocket, with what the rod changes and
/// where each code's rocket is at apogee.
fn rod_probes_table(report: &Value) -> String {
    let all = report["probes"].as_array().into_iter().flatten();
    let without = all
        .clone()
        .find(|probe| probe["design"] == WITHOUT_PODS)
        .unwrap_or(&Value::Null);
    let rods: Vec<&Value> = all
        .filter(|probe| {
            probe["file"]
                .as_str()
                .is_some_and(|file| file.starts_with(ROD_PROBES))
        })
        .collect();
    // `fly_all` refuses probes without the control, so neither is ever missing here.
    if rods.is_empty() || without.is_null() {
        return String::new();
    }
    let mut out = String::from(
        "\n## Tilted-rod probes\n\nThe pod probes' airframe with no pods, launched from a 1 m \
         rod tilted from the vertical toward a compass bearing ([M2.2e5][m2-2e5]), in calm air, \
         and not counted among the designs above:\n\n\
         - `pods-none`, first, is the same airframe from OpenRocket's default vertical rod: the \
         control. Its bearing means nothing: from a vertical rod both codes' rockets drift about \
         0.4 m west, from Earth's rotation.\n\
         - *Rod's change* is the apogee's change from `pods-none` in each code.\n\
         - *At apogee* is where the rocket is then, metres east and north of where it started, \
         and *bearing* the direction of that place from the pad, clockwise from north: a rod \
         read the wrong way round would send hpr's rocket another way than OpenRocket's.\n\
         - *α OR* is the angle of attack OpenRocket's rocket has at its rod-clearance row, which \
         grows with the tilt, where hpr's margin is taken at none; *at OR's α* is hpr's margin \
         at OpenRocket's angle, which says how much of the margins' difference that accounts \
         for.\n\n\
         [m2-2e5]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e5\n\n\
         | probe | rod | apogee OR (m) | hpr (m) | Δ | rod's change OR | hpr \
         | max speed OR (m/s) | hpr (m/s) | Δ | α OR (°) | margin OR (cal) | hpr (cal) \
         | at OR's α (cal) | at apogee OR (E, N m) | hpr (E, N m) | bearing OR | hpr |\n\
         |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:\
         |---:|\n",
    );
    for probe in std::iter::once(without).chain(rods) {
        let m = &probe["metrics"];
        let rod = &probe["rod"];
        let angle = rod["angle_from_vertical_rad"].as_f64().unwrap_or(f64::NAN);
        let direction = rod["direction_rad"].as_f64().unwrap_or(f64::NAN);
        let rod = if angle == 0.0 {
            "vertical".to_owned()
        } else {
            let bearing = direction.to_degrees();
            let names = [
                "north",
                "north-east",
                "east",
                "south-east",
                "south",
                "south-west",
                "west",
                "north-west",
            ];
            // A whole eighth of the compass is named; 8 eighths is north again.
            let eighth = (bearing / 45.0).rem_euclid(8.0);
            let name = (0_u8..)
                .zip(names)
                .find(|(at, _)| {
                    let off = (eighth - f64::from(*at)).abs();
                    off < 1e-9 || (*at == 0 && (8.0 - eighth).abs() < 1e-9)
                })
                .map_or_else(String::new, |(_, name)| format!(" ({name})"));
            format!("{:.0}° toward {bearing:.0}°{name}", angle.to_degrees())
        };
        let change = |at: usize| {
            probe_change(probe, without).map_or_else(
                || "-".to_owned(),
                |[apogee, _]| format!("{:+.2}%", apogee[at]),
            )
        };
        let place = |code: &str| {
            let at = &probe["apogee_position_m"][code];
            match (at["east"].as_f64(), at["north"].as_f64()) {
                (Some(east), Some(north)) => (
                    format!("{}, {}", tenths(east), tenths(north)),
                    format!("{:.1}°", bearing_deg(east, north)),
                ),
                _ => ("-".to_owned(), "-".to_owned()),
            }
        };
        let (at_or, bearing_or) = place("openrocket");
        let (at_hpr, bearing_hpr) = place("hpr");
        let clearance = &probe["at_rod_clearance"];
        let alpha_deg = clearance["openrocket"]["angle_of_attack_rad"]
            .as_f64()
            .map(f64::to_degrees);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} \
             | {} |\n",
            probe["design"].as_str().unwrap_or_default(),
            rod,
            fixed(&m["apogee_m"]["openrocket"], 1),
            fixed(&m["apogee_m"]["hpr"], 1),
            percent(&m["apogee_m"]),
            change(0),
            change(1),
            fixed(&m["max_speed_m_s"]["openrocket"], 2),
            fixed(&m["max_speed_m_s"]["hpr"], 2),
            percent(&m["max_speed_m_s"]),
            fixed(&json!(alpha_deg), 3),
            fixed(&m["rod_clearance_margin_cal"]["openrocket"], 3),
            fixed(&m["rod_clearance_margin_cal"]["hpr"], 3),
            fixed(&json!(margin_at_openrocket_alpha_cal(probe)), 3),
            at_or,
            at_hpr,
            bearing_or,
            bearing_hpr,
        ));
    }
    out
}

/// The table of each named cause's size: every flight with one, and OpenRocket's own flight flown
/// again without it (M2.2e4, decision ADR-073).
fn causes_table(report: &Value) -> String {
    let empty = Vec::new();
    let sized: Vec<&Value> = report["flights"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|f| f["apogee_with_the_causes_removed"].is_object())
        .collect();
    if sized.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "\nThe named causes, sized ([M2.2e4][m2-2e4], decision [ADR-073][adr-073]). OpenRocket \
         flew each flight with a named cause again without it. *Nothing deployed*: the same \
         flight with no parachute opening. *Drag settings cleared too*: also with every part's \
         stated drag coefficient cleared, so each part has the drag of its shape, as in hpr, \
         which reads the setting but can't apply it yet. *The parts removed*: nothing deployed \
         and the parts set to no drag taken off, in OpenRocket and in hpr alike, which also takes \
         away their lift. Each Δ is hpr's apogee less that flight's, in per cent of it. *Within \
         5% after*, for an apogee more than 5% off: whether every one of these flights with all \
         its causes taken out is within 5% of hpr's.\n\n\
         | design | motors | Δ apogee | chute early (s) | OR, nothing deployed (m) | Δ \
         | OR, drag settings cleared too (m) | Δ | OR, the parts removed (m) \
         | hpr, the parts removed (m) | Δ | within 5% after |\n\
         |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n",
    );
    // A flight OpenRocket could not fly shows why instead of its apogee.
    let apogee = |step: &Value| {
        step["why"]
            .as_str()
            .map_or_else(|| fixed(&step["openrocket_m"], 1), str::to_owned)
    };
    let relative = |step: &Value| {
        step["relative_percent"]
            .as_f64()
            .map_or_else(|| "—".to_owned(), |p| format!("{p:+.2}%"))
    };
    for flight in sized {
        let removed = &flight["apogee_with_the_causes_removed"];
        let (held, too) = (
            &removed["parachutes_held"],
            &removed["drag_overrides_cleared_too"],
        );
        let both = &removed["parts_removed_from_both"];
        let over = flight["metrics"]["apogee_m"]["relative_percent"]
            .as_f64()
            .is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT);
        let within = match (over, removed["within_5_percent"].as_bool()) {
            (true, Some(true)) => "yes",
            (true, _) => "no",
            (false, _) => "—",
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {within} |\n",
            flight["design"].as_str().unwrap_or_default(),
            flight["motors"].as_str().unwrap_or_default(),
            percent(&flight["metrics"]["apogee_m"]),
            fixed(&flight["deployed_before_apogee_s"], 2),
            apogee(held),
            relative(held),
            if too.is_null() {
                "—".to_owned()
            } else {
                apogee(too)
            },
            relative(too),
            if both.is_null() {
                "—".to_owned()
            } else {
                apogee(both)
            },
            fixed(&both["hpr_m"], 1),
            relative(both),
        ));
    }
    out.push_str(
        "\n[m2-2e4]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e4\n\
         [adr-073]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-073-each-named-cause-sized-by-openrockets-own-flight-without-it-2026-09-25\n",
    );
    out
}

/// The summary, as lines for the terminal and the page.
pub(crate) fn summary_lines(report: &Value) -> String {
    let summary = &report["summary"];
    let metrics = &summary["metrics"];
    let line = |label: &str, spread: &Value, unit: &str, digits: usize| {
        format!(
            "- {label}: {} scored, median {}{unit}, mean absolute {}{unit}, from {}{unit} to {}{unit}\n",
            spread["count"],
            signed(&spread["median"], digits),
            fixed(&spread["mean_absolute"], digits),
            signed(&spread["min"], digits),
            signed(&spread["max"], digits),
        )
    };
    let mut out = format!(
        "- configurations flown: {} ({} the record holds are not flown by hpr); apogee more than \
         5% from OpenRocket's: {}\n",
        summary["flown"], summary["not_flown"], summary["apogee_over_5_percent"],
    );
    let empty = Vec::new();
    let fastest = report["flights"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .filter(|f| f["max_mach_openrocket"].is_f64())
        .max_by(|a, b| {
            let mach = |f: &Value| f["max_mach_openrocket"].as_f64().unwrap_or(0.0);
            mach(a).total_cmp(&mach(b))
        });
    if let Some(fastest) = fastest {
        out.push_str(&format!(
            "- fastest: {} {}, OpenRocket's largest Mach number {}\n",
            fastest["design"].as_str().unwrap_or_default(),
            fastest["motors"].as_str().unwrap_or_default(),
            fixed(&fastest["max_mach_openrocket"], 3),
        ));
    }
    for (key, label, unit, digits) in [
        ("apogee_m", "apogee", "%", 2),
        ("max_speed_m_s", "largest speed", "%", 2),
        (
            "rod_clearance_margin_cal",
            "margin at rod clearance",
            " cal",
            4,
        ),
    ] {
        let empty = serde_json::Map::new();
        let groups = metrics[key]["scored_by_cause"]
            .as_object()
            .unwrap_or(&empty);
        for (cause, spread) in groups {
            out.push_str(&line(&format!("{label}, {cause}"), spread, unit, digits));
        }
    }
    let sized = &summary["apogee_with_the_causes_removed"];
    if sized.is_object() {
        out.push_str(&line(
            "apogee against OpenRocket's own flights with the named causes removed, over the \
             flights with a named cause, each counting its largest difference from them",
            &sized["remaining_percent"],
            "%",
            2,
        ));
        out.push_str(&format!(
            "- apogees more than 5% off: {}, {} with their named causes sized; within 5% of every \
             flight of OpenRocket's without the causes: {} of {}\n",
            sized["over_5_percent"],
            sized["over_5_percent_sized"],
            sized["over_5_percent_within_5_percent_after"],
            sized["over_5_percent"],
        ));
    }
    let mass_and_cg = &summary["mass_and_cg"];
    out.push_str(
        "\nhpr's mass and centre of mass less OpenRocket's ([M2.2e1][m2-2e1], decision \
         [ADR-070][adr-070]), over the flights not aborted: the masses in per cent of \
         OpenRocket's, the centre of mass in OpenRocket's calibres, positive when hpr's is further \
         aft, which shortens the margin by as much.\n\n",
    );
    for (key, label, unit, digits) in [
        ("launch_mass_percent", "mass at launch", "%", 3),
        (
            "rod_clearance_mass_percent",
            "mass at rod clearance",
            "%",
            3,
        ),
        (
            "rod_clearance_cg_cal",
            "centre of mass at rod clearance",
            " cal",
            4,
        ),
    ] {
        out.push_str(&line(label, &mass_and_cg[key], unit, digits).replacen(
            " scored,",
            " compared,",
            1,
        ));
    }
    out
}

pub(crate) fn fixed(value: &Value, digits: usize) -> String {
    value
        .as_f64()
        .map_or_else(|| "—".to_owned(), |v| format!("{v:.digits$}"))
}

pub(crate) fn signed(value: &Value, digits: usize) -> String {
    value
        .as_f64()
        .map_or_else(|| "—".to_owned(), |v| format!("{v:+.digits$}"))
}

fn percent(entry: &Value) -> String {
    match entry["relative_percent"].as_f64() {
        Some(p) => format!("{p:+.2}%"),
        None => entry["outcome"]["outcome"]
            .as_str()
            .unwrap_or("—")
            .to_owned(),
    }
}

/// Collects where `now` departs from `committed`: numbers beyond [`CHECK_RELATIVE`], anything
/// else that differs.
pub(crate) fn same(committed: &Value, now: &Value, at: &str, apart: &mut Vec<String>) {
    match (committed, now) {
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (
                a.as_f64().unwrap_or(f64::NAN),
                b.as_f64().unwrap_or(f64::NAN),
            );
            if (a - b).abs() > CHECK_RELATIVE * a.abs().max(b.abs()) {
                apart.push(format!("{at}: {a} then, {b} now"));
            }
        }
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
                same(
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    &format!("{at}/{key}"),
                    apart,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (index, (a, b)) in a.iter().zip(b).enumerate() {
                same(a, b, &format!("{at}/{index}"), apart);
            }
        }
        (a, b) if a != b => apart.push(format!("{at}: {a} then, {b} now")),
        _ => {}
    }
}

/// Checks that every motor that lights is spent by hpr's apogee at `apogee_s`, from each motor's
/// ignition time (`lit`) and its burn time and whether it fails (`burns`).
///
/// hpr-io leaves a separation at apogee or on the way down to the descent and flies the
/// configuration whole (ADR-076 §4), which holds only if every motor is spent by then. The check
/// is made of every flight, staged or not, since a motor still burning at apogee would also make
/// the compared climb something other than a climb. A motor that lights must have a time: a
/// `.ork` configuration never lights one at a separation.
fn spent_by_apogee(
    lit: &[Option<f64>],
    burns: &[(f64, bool)],
    apogee_s: f64,
) -> Result<(), String> {
    for (index, (lit_s, (burn_s, fails))) in lit.iter().zip(burns).enumerate() {
        match lit_s {
            Some(lit_s) if lit_s + burn_s > apogee_s => {
                return Err(format!(
                    "motor {index} burns until {} s, after hpr's apogee at {apogee_s} s",
                    lit_s + burn_s
                ));
            }
            Some(_) => {}
            None if *fails => {}
            None => {
                return Err(format!(
                    "motor {index} lights at no time known before the flight"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_flight_is_reported_only_with_every_lit_motor_spent_by_apogee() {
        let burns = [(3.0, false), (2.0, false), (1.0, true)];
        // Lit at launch and at 4 s: spent at 3 s and 6 s, before an apogee at 10 s; the failing
        // tube never lights.
        assert_eq!(
            super::spent_by_apogee(&[Some(0.0), Some(4.0), None], &burns, 10.0),
            Ok(())
        );
        // The second lit at 9 s burns to 11 s, past it.
        let late = super::spent_by_apogee(&[Some(0.0), Some(9.0), None], &burns, 10.0);
        assert!(late.is_err_and(|e| e.contains("motor 1 burns until 11 s")));
        // A motor that lights with no known time.
        let unknown = super::spent_by_apogee(&[Some(0.0), None, None], &burns, 10.0);
        assert!(unknown.is_err_and(|e| e.contains("motor 1 lights at no time known")));
    }

    use super::*;

    fn committed() -> (Value, Value) {
        let root = crate::ork::root().unwrap();
        (
            read_json(&root.join(RECORD)).unwrap(),
            read_json(&root.join(REPORT_JSON)).unwrap(),
        )
    }

    /// The record's flight of `file`'s `configuration`.
    fn recorded<'a>(record: &'a Value, file: &str, configuration: &str) -> &'a Value {
        record["designs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["file"] == file)
            .flat_map(|entry| entry["flights"].as_array().unwrap())
            .find(|flight| flight["configuration"] == configuration)
            .unwrap_or_else(|| panic!("{file} {configuration} is not in the record"))
    }

    #[test]
    fn every_motor_configuration_of_the_record_is_flown_or_named() {
        let (record, report) = committed();
        let mut listed: Vec<(String, String)> = report["flights"]
            .as_array()
            .unwrap()
            .iter()
            .chain(report["not_flown"].as_array().unwrap())
            .chain(report["probes"].as_array().unwrap())
            .map(|f| {
                (
                    f["file"].as_str().unwrap().to_owned(),
                    f["configuration"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let mut expected: Vec<(String, String)> = record["designs"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|entry| {
                let name = entry["file"].as_str().unwrap().to_owned();
                entry["flights"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|f| f["has_motors"] == true)
                    .map(move |f| {
                        (
                            name.clone(),
                            f["configuration"].as_str().unwrap().to_owned(),
                        )
                    })
            })
            .collect();
        listed.sort();
        expected.sort();
        assert_eq!(listed, expected);
        // M2.2d2's scope, the 21 configurations in five of the jar's examples that hpr flies, and
        // M1.9c's 12: five clusters, five air starts beside a cluster and two two-stage flights.
        // M1.13c2's six pod probes and M2.2e5's four tilted-rod probes are listed apart.
        assert_eq!(report["flights"].as_array().unwrap().len(), 33);
        assert_eq!(report["probes"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn the_reference_values_are_the_records_and_the_outcomes_are_compares() {
        let (record, report) = committed();
        let tool = openrocket();
        let probes = report["probes"].as_array().unwrap();
        for flight in report["flights"].as_array().unwrap().iter().chain(probes) {
            let design = flight["file"].as_str().unwrap();
            let configuration = flight["configuration"].as_str().unwrap();
            let source = recorded(&record, design, configuration);
            assert_eq!(flight["design"], design_name(design).as_str());
            assert_eq!(flight["aborted"], source["aborted"]);
            assert_eq!(flight["motors"], source["name"]);
            assert_eq!(flight["rod_length_m"], source["conditions"]["rod_length_m"]);
            assert_eq!(flight["max_mach_openrocket"], source["summary"]["max_mach"]);
            assert_eq!(
                flight["launch_mass_kg"]["openrocket"],
                source["series"]["launch_mass_kg"]
            );
            assert_eq!(
                flight["deployed_before_apogee_s"],
                json!(early_chute(source).unwrap()),
                "{design} {configuration}"
            );
            for key in [
                "mass_kg",
                "cg_from_nose_m",
                "cp_from_nose_m",
                "reference_length_m",
            ] {
                assert_eq!(
                    flight["at_rod_clearance"]["openrocket"][key], source["rod_clearance"][key],
                    "{design} {configuration} {key}"
                );
            }
            let keys = [
                &source["summary"]["max_altitude_m"],
                &source["summary"]["max_velocity_m_s"],
                &source["rod_clearance"]["stability_cal"],
            ];
            for ((metric, key), reference) in METRICS.iter().zip(keys) {
                let entry = &flight["metrics"][key];
                assert_eq!(
                    &entry["openrocket"], reference,
                    "{design} {configuration} {key}"
                );
                let reading = if source["aborted"] == true {
                    ReferenceReading::Aborted
                } else {
                    ReferenceReading::Complete(reference.as_f64().or(Some(f64::NAN)))
                };
                let outcome = compare(&tool, *metric, reading, entry["hpr"].as_f64());
                assert_eq!(
                    metric_entry(reading, entry["hpr"].as_f64(), &outcome),
                    *entry,
                    "{design} {configuration} {key}"
                );
            }
            for key in ["time_s", "mach"] {
                assert_eq!(
                    flight["at_rod_clearance"][key], source["rod_clearance"][key],
                    "{design} {configuration} {key}"
                );
            }
        }
    }

    #[test]
    fn the_summary_and_the_page_are_the_flights() {
        let root = crate::ork::root().unwrap();
        let (_, report) = committed();
        let summary = summarise(
            report["flights"].as_array().unwrap(),
            report["not_flown"].as_array().unwrap(),
        );
        assert_eq!(summary, report["summary"]);
        let page_now = fs::read_to_string(root.join(REPORT_MD)).unwrap();
        assert_eq!(page_now.replace("\r\n", "\n"), page(&report));
    }

    /// OpenRocket's drag curves become a table on its reference diameter, power on while burning;
    /// a flight that can't stand for one gives none, and a malformed record is an error.
    #[test]
    fn openrocket_s_drag_is_a_table_of_its_two_curves() {
        let flight = json!({
            "configuration": "ABC",
            "branches": 1,
            "aborted": false,
            "reference_length_m": 0.1,
            "power_on": [[0.1, 0.5], [0.5, 0.4], [1.5, 0.6]],
            "power_off": [[0.2, 0.7], [1.0, 0.9]],
        });
        let design = |flight: &Value| json!({ "flights": [flight] });
        let table = openrocket_drag(&design(&flight), "abc").unwrap().unwrap();
        assert_eq!(table.reference_diameter_m, Some(0.1));
        let at = |mach: f64, thrusting: bool| table.lookup(mach, thrusting).unwrap().value;
        assert!((at(1.0, true) - 0.5).abs() < 1e-15);
        assert!((at(0.6, false) - 0.8).abs() < 1e-15);
        // Held at the ends.
        assert_eq!(at(3.0, true), 0.6);
        assert_eq!(at(0.0, false), 0.7);
        assert!(
            openrocket_drag(&design(&flight), "other")
                .unwrap()
                .is_none()
        );
        for (key, value) in [
            ("branches", json!(2)),
            ("aborted", json!(true)),
            ("refused", json!("no")),
            ("power_off", json!([[0.2, 0.7]])),
        ] {
            let mut changed = flight.clone();
            changed[key] = value;
            assert!(
                openrocket_drag(&design(&changed), "ABC").unwrap().is_none(),
                "{key}"
            );
        }
        for (key, value) in [
            ("power_on", Value::Null),
            ("power_on", json!([[0.1, "x"], [0.5, 0.4]])),
            ("power_on", json!([[0.5, 0.5], [0.1, 0.4]])),
            ("reference_length_m", Value::Null),
        ] {
            let mut changed = flight.clone();
            changed[key] = value;
            assert!(openrocket_drag(&design(&changed), "ABC").is_err(), "{key}");
        }
    }

    #[test]
    fn every_apogee_more_than_5_percent_off_has_a_named_cause() {
        // M2.2's parent asks a written cause for each. This report has two: a reference parachute
        // open before apogee, which hpr does not fly from a `.ork`, and a part whose drag
        // OpenRocket is told is zero, which hpr cannot yet be told. The third, hpr's own drag,
        // needs OpenRocket's drag curves, which are recorded only for the private library.
        let (_, report) = committed();
        for flight in report["flights"].as_array().unwrap() {
            let percent = flight["metrics"]["apogee_m"]["relative_percent"].as_f64();
            if percent.is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT) {
                assert!(
                    cause(flight, FlightMetric::Apogee) != NO_NAMED_CAUSE,
                    "{} {} is {percent:?}% off with no cause written",
                    flight["design"],
                    flight["motors"]
                );
            }
        }
    }

    /// M1.9c's bar (ADR-076 §1, set before these flights were measured): a two-stage design and a
    /// cluster design, each with every configuration hpr flies within 5% of OpenRocket's apogee
    /// (of its flight with nothing deployed, where its parachute opened before apogee) and of its
    /// largest speed. A staged flight also separates when OpenRocket's does.
    #[test]
    fn a_two_stage_and_a_cluster_design_are_within_5_percent_of_openrocket() {
        let (_, report) = committed();
        let flights = report["flights"].as_array().unwrap();
        let designs = |mark: &str| -> BTreeSet<&str> {
            flights
                .iter()
                .filter(|flight| !flight[mark].is_null())
                .map(|flight| flight["design"].as_str().unwrap())
                .collect()
        };
        let staged = designs("staging");
        let clustered = designs("clustered_motors");
        assert!(!staged.is_empty(), "no staged design is flown");
        assert!(!clustered.is_empty(), "no cluster design is flown");
        for flight in flights {
            let design = flight["design"].as_str().unwrap();
            if !staged.contains(design) && !clustered.contains(design) {
                continue;
            }
            let at = format!("{design} {}", flight["motors"]);
            let apogee_percent = if flight["deployed_before_apogee_s"].is_null() {
                &flight["metrics"]["apogee_m"]["relative_percent"]
            } else {
                &flight["apogee_with_the_causes_removed"]["parachutes_held"]["relative_percent"]
            };
            let speed_percent = &flight["metrics"]["max_speed_m_s"]["relative_percent"];
            for (what, percent) in [("apogee", apogee_percent), ("largest speed", speed_percent)] {
                let percent = percent.as_f64().unwrap_or(f64::NAN);
                assert!(percent.abs() <= 5.0, "{at}: {what} {percent}% off");
            }
            if let Some(separation) = flight.get("staging") {
                let times = &separation["separation_s"];
                let (hpr, openrocket) = (
                    times["hpr"].as_f64().unwrap(),
                    times["openrocket"].as_f64().unwrap(),
                );
                assert!(
                    (hpr - openrocket).abs() < 1e-6,
                    "{at}: {hpr} s, {openrocket} s"
                );
            }
        }
    }

    /// M1.13's second bar (M1.13c2), at ADR-076's per-case 5%: every pod probe, one airframe
    /// carrying pods of bodies, fins, a tail cone, winglets or motors, within 5% of OpenRocket's
    /// apogee and largest speed. So the pods are seen to count, not only the airframe, what each
    /// probe's pods change (its apogee and margin against the same airframe without pods) is held
    /// too: within 1 point of apogee and 0.005 calibres of OpenRocket's change, and each margin
    /// within 0.005 calibres of OpenRocket's. The bounds were set after the measurement (0.32
    /// points, 0.0004 and 0.0014 calibres at most), to catch a pod rule that breaks: a pod fin's
    /// `K_T(B)` taken on the airframe, by a hand estimate, moves the margin by some 0.04 calibres.
    /// The census leaves the probes out (ADR-093), so these bounds are what holds them in CI.
    #[test]
    fn pod_designs_are_within_5_percent_of_openrocket() {
        let (_, report) = committed();
        let probe = |entry: &&Value| {
            entry["file"]
                .as_str()
                .is_some_and(|file| file.starts_with(POD_PROBES))
        };
        for list in ["flights", "not_flown"] {
            let listed: Vec<_> = report[list]
                .as_array()
                .unwrap()
                .iter()
                .filter(probe)
                .collect();
            assert!(listed.is_empty(), "a probe among the designs: {listed:?}");
        }
        let probes: BTreeMap<&str, &Value> = report["probes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(probe)
            .map(|flight| (flight["design"].as_str().unwrap(), flight))
            .collect();
        let names: Vec<_> = probes.keys().copied().collect();
        assert_eq!(
            names,
            [
                "pods-bodies-3",
                "pods-fins-2",
                "pods-fins-tail-4",
                "pods-motors-2",
                "pods-none",
                "pods-winglets-2"
            ]
        );
        let without = probes[WITHOUT_PODS];
        for (design, flight) in &probes {
            assert!(flight["deployed_before_apogee_s"].is_null(), "{design}");
            for metric in ["apogee_m", "max_speed_m_s"] {
                let percent = flight["metrics"][metric]["relative_percent"]
                    .as_f64()
                    .unwrap_or(f64::NAN);
                assert!(percent.abs() <= 5.0, "{design}: {metric} {percent}% off");
            }
            let [[apogee_or, apogee_hpr], [margin_or, margin_hpr]] =
                probe_change(flight, without).unwrap();
            assert!(
                (apogee_hpr - apogee_or).abs() <= 1.0,
                "{design}: the pods change the apogee {apogee_hpr:+.2}% in hpr, {apogee_or:+.2}% \
                 in OpenRocket"
            );
            let margin = flight["metrics"]["rod_clearance_margin_cal"]["difference"]
                .as_f64()
                .unwrap_or(f64::NAN);
            assert!(
                margin.abs() <= 0.005,
                "{design}: margin {margin:+.4} cal off"
            );
            assert!(
                (margin_hpr - margin_or).abs() <= 0.005,
                "{design}: the pods change the margin {margin_hpr:+.4} cal in hpr, \
                 {margin_or:+.4} in OpenRocket"
            );
        }
    }

    /// The smaller turn between two compass bearings, in degrees.
    fn turn_deg(a: f64, b: f64) -> f64 {
        let turn = (a - b).rem_euclid(360.0);
        turn.min(360.0 - turn)
    }

    /// OpenRocket's rod leans the way `conditions.py` found its rocket lands (a compass bearing,
    /// the angle from the vertical), and [`rod_rail`] points hpr's rail the same way: its bearing
    /// is where OpenRocket's rocket landed from the same rod, and its angle above the horizon is
    /// 90 degrees less the rod's.
    #[test]
    fn the_rail_leans_where_openrockets_rocket_lands() {
        let root = crate::ork::root().unwrap();
        let measured =
            read_json(&root.join("validation/fixtures/ork/openrocket-conditions.json")).unwrap();
        let landings = measured["direction"].as_array().unwrap();
        assert_eq!(landings.len(), 2);
        for landing in landings {
            let direction_deg = landing["rod_direction_deg"].as_f64().unwrap();
            let rail = rod_rail(&json!({
                "rod_length_m": 1.0,
                "rod_angle_rad": 10f64.to_radians(),
                "rod_direction_rad": direction_deg.to_radians(),
            }))
            .unwrap();
            let up = rail.direction_enu();
            assert!((up.z - 10f64.to_radians().cos()).abs() < 1e-12, "{up}");
            let landed = bearing_deg(
                landing["landed_east_m"].as_f64().unwrap(),
                landing["landed_north_m"].as_f64().unwrap(),
            );
            let leans = bearing_deg(up.x, up.y);
            assert!(
                turn_deg(leans, landed) < 1e-9,
                "a rod toward {direction_deg}° leans to {leans}°, OpenRocket landed at {landed}°"
            );
        }
        // No length, angle or direction is an error; a rod the rail refuses too.
        let rod = json!({"rod_length_m": 1.0, "rod_angle_rad": 0.1, "rod_direction_rad": 0.0});
        for key in ["rod_length_m", "rod_angle_rad", "rod_direction_rad"] {
            let mut missing = rod.clone();
            missing[key] = Value::Null;
            assert!(rod_rail(&missing).is_err(), "{key}");
        }
        let mut flat = rod.clone();
        flat["rod_angle_rad"] = json!(FRAC_PI_2);
        assert!(rod_rail(&flat).is_err());
    }

    /// M2.2e5 (issue #173), on the tilted-rod probes: the pod probes' airframe from rods tilted 5,
    /// 10 and 20 degrees toward three bearings, which OpenRocket flew from each file's stored
    /// conditions. Each is within ADR-076's per-case 5% of OpenRocket's apogee and largest speed,
    /// and three things a rod read the wrong way would break are held: where the rocket is at
    /// apogee, within 0.1 degrees of OpenRocket's bearing from the pad and 1% of its distance;
    /// what the rod takes off the apogee against the vertical `pods-none`, within 0.5 points of
    /// OpenRocket's; and the margin at rod clearance at OpenRocket's angle of attack there,
    /// within 0.006 calibres of OpenRocket's. The bounds were set after the measurement (0.03
    /// degrees, 0.64%, 0.27 points and 0.0048 calibres at most); a bearing read anticlockwise
    /// turns the east probes 180 degrees, and an angle read from the horizon makes the vertical
    /// rod a horizontal rail, which hpr refuses. The bearing's 0.03 degrees is motion across the
    /// tilt (0.18 m across 370 m at 20 degrees): hpr's fits Earth's rotation, OpenRocket's goes
    /// the other way, cause unmeasured. The distance's 0.64% is partly that OpenRocket's position
    /// is its highest 0.05 s row and hpr's its apogee event: a longer flight would take more of
    /// both bounds.
    /// The census leaves the probes out (ADR-093), so these bounds are what holds them in CI.
    #[test]
    fn tilted_rods_fly_as_openrocket_flies_them() {
        let (_, report) = committed();
        let probes: BTreeMap<&str, &Value> = report["probes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|flight| (flight["design"].as_str().unwrap(), flight))
            .collect();
        let rods: Vec<(&str, &Value)> = probes
            .iter()
            .filter(|(_, flight)| {
                flight["file"]
                    .as_str()
                    .is_some_and(|file| file.starts_with(ROD_PROBES))
            })
            .map(|(design, flight)| (*design, *flight))
            .collect();
        let names: Vec<_> = rods.iter().map(|(design, _)| *design).collect();
        assert_eq!(
            names,
            [
                "rod-10-east",
                "rod-10-southwest",
                "rod-20-east",
                "rod-5-north"
            ]
        );
        let without = probes[WITHOUT_PODS];
        assert_eq!(without["rod"]["angle_from_vertical_rad"], json!(0.0));
        // From the vertical rod OpenRocket clears at next to no angle of attack (1.6e-7 rad), a
        // thousandth of the smallest tilt's.
        let vertical_alpha = without["at_rod_clearance"]["openrocket"]["angle_of_attack_rad"]
            .as_f64()
            .unwrap();
        assert!(vertical_alpha.abs() < 1e-6, "{vertical_alpha}");
        for (design, flight) in rods {
            let (angle, direction) = match design {
                "rod-5-north" => (5.0, 0.0),
                "rod-10-east" => (10.0, 90.0),
                "rod-10-southwest" => (10.0, 225.0),
                _ => (20.0, 90.0),
            };
            let rod = &flight["rod"];
            let read = |key: &str| rod[key].as_f64().unwrap().to_degrees();
            assert!(
                (read("angle_from_vertical_rad") - angle).abs() < 1e-9,
                "{design}"
            );
            assert!((read("direction_rad") - direction).abs() < 1e-9, "{design}");
            for metric in ["apogee_m", "max_speed_m_s"] {
                let percent = flight["metrics"][metric]["relative_percent"]
                    .as_f64()
                    .unwrap_or(f64::NAN);
                assert!(percent.abs() <= 5.0, "{design}: {metric} {percent}% off");
            }
            let place = |code: &str| {
                let at = &flight["apogee_position_m"][code];
                (at["east"].as_f64().unwrap(), at["north"].as_f64().unwrap())
            };
            let ((east_or, north_or), (east_hpr, north_hpr)) = (place("openrocket"), place("hpr"));
            let (bearing_or, bearing_hpr) = (
                bearing_deg(east_or, north_or),
                bearing_deg(east_hpr, north_hpr),
            );
            assert!(
                turn_deg(bearing_or, direction) < 1.0,
                "{design}: OpenRocket's rocket is at {bearing_or}° at apogee"
            );
            assert!(
                turn_deg(bearing_hpr, bearing_or) <= 0.1,
                "{design}: at apogee hpr's rocket is at {bearing_hpr}°, OpenRocket's at \
                 {bearing_or}°"
            );
            let distance = east_hpr.hypot(north_hpr) / east_or.hypot(north_or) - 1.0;
            assert!(
                distance.abs() <= 0.01,
                "{design}: hpr's rocket is {:+.2}% as far from the pad at apogee",
                100.0 * distance
            );
            let [[apogee_or, apogee_hpr], _] = probe_change(flight, without).unwrap();
            assert!(
                apogee_or < 0.0,
                "{design}: the rod adds {apogee_or}% in OpenRocket"
            );
            assert!(
                (apogee_hpr - apogee_or).abs() <= 0.5,
                "{design}: the rod changes the apogee {apogee_hpr:+.2}% in hpr, \
                 {apogee_or:+.2}% in OpenRocket"
            );
            let alpha = flight["at_rod_clearance"]["openrocket"]["angle_of_attack_rad"]
                .as_f64()
                .unwrap();
            assert!(
                alpha > 0.0,
                "{design}: OpenRocket clears the rod at α {alpha}"
            );
            let margin_or = flight["metrics"]["rod_clearance_margin_cal"]["openrocket"]
                .as_f64()
                .unwrap();
            let at_alpha = margin_at_openrocket_alpha_cal(flight).unwrap();
            assert!(
                (at_alpha - margin_or).abs() <= 0.006,
                "{design}: margin at OpenRocket's α {at_alpha:.4} cal in hpr, {margin_or:.4} in \
                 OpenRocket"
            );
        }
    }

    #[test]
    fn every_named_cause_is_sized_by_openrockets_own_flight_without_it() {
        // M2.2e4: each named cause is sized by OpenRocket's flight of the same configuration
        // without it, which the record holds, and nothing else.
        let (record, report) = committed();
        let (mut sized, mut over, mut within) = (0, 0, 0);
        for flight in report["flights"].as_array().unwrap() {
            let overridden = !flight["drag_overrides_not_applied"].is_null();
            let named = !flight["deployed_before_apogee_s"].is_null() || overridden;
            let removed = &flight["apogee_with_the_causes_removed"];
            let at = format!("{} {}", flight["design"], flight["motors"]);
            if !named || flight["aborted"] == true {
                assert!(removed.is_null(), "{at} is sized with no cause named");
                continue;
            }
            sized += 1;
            let file = flight["file"].as_str().unwrap();
            let source = recorded(&record, file, flight["configuration"].as_str().unwrap());
            let hpr = flight["metrics"]["apogee_m"]["hpr"].as_f64().unwrap();
            let check = |step: &Value, key: &str, measured: f64| {
                let openrocket = source[key]["max_altitude_m"].as_f64().unwrap();
                assert_eq!(
                    step["openrocket_m"].as_f64(),
                    Some(openrocket),
                    "{at} {key}"
                );
                let percent = 100.0 * (measured - openrocket) / openrocket;
                let written = step["relative_percent"].as_f64().unwrap();
                assert!((written - percent).abs() < 1e-9, "{at} {key}");
                written
            };
            let mut last = check(&removed["parachutes_held"], "undeployed", hpr);
            let too = &removed["drag_overrides_cleared_too"];
            if overridden {
                last = check(too, "undeployed_without_drag_overrides", hpr);
            } else {
                assert!(too.is_null(), "{at}");
            }
            let mut left = vec![last];
            let probe = &flight["without_the_overridden_parts"]["apogee_m"]["hpr"];
            let both = &removed["parts_removed_from_both"];
            if let Some(probe) = probe.as_f64() {
                assert_eq!(both["hpr_m"].as_f64(), Some(probe), "{at}");
                let key = "undeployed_without_parts_set_to_no_drag";
                left.push(check(both, key, probe));
            } else {
                assert!(both.is_null(), "{at}");
            }
            let agrees = left.iter().all(|p| p.abs() <= APOGEE_CAUSE_PERCENT);
            let largest = left
                .iter()
                .copied()
                .max_by(|a, b| a.abs().total_cmp(&b.abs()));
            assert_eq!(removed["remaining_percent"].as_f64(), largest, "{at}");
            assert_eq!(removed["within_5_percent"].as_bool(), Some(agrees), "{at}");
            let percent = flight["metrics"]["apogee_m"]["relative_percent"].as_f64();
            if percent.is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT) {
                over += 1;
                within += usize::from(agrees);
            }
        }
        // M1.9c added the cluster's three flights with an early parachute, one of them over 5%.
        assert_eq!((sized, over, within), (12, 6, 5));
        let summary = &report["summary"]["apogee_with_the_causes_removed"];
        assert_eq!(summary["over_5_percent"], over);
        assert_eq!(summary["over_5_percent_sized"], over);
        assert_eq!(summary["over_5_percent_within_5_percent_after"], within);
    }

    #[test]
    fn a_parachute_moves_openrockets_apogee_only_when_it_opens_before_it() {
        // The early-chute cause, both ways, over every flight of the record. Where the first
        // deployment is not before the apogee event, the flight with nothing deployed has the same
        // apogee to the bit: nothing else differs between the two runs. Where it is before, the
        // flight with nothing deployed climbs higher, or is within what the deployment event alone
        // can move the peak of rows 0.05 s apart (g dt²/8, 3 mm).
        let (record, _) = committed();
        let sampling_m = 9.81 * 0.05 * 0.05 / 8.0;
        let (mut lowered, mut within_sampling, mut late) = (0, 0, 0);
        for design in record["designs"].as_array().unwrap() {
            for flight in design["flights"].as_array().into_iter().flatten() {
                if flight["has_motors"] != true
                    || flight["aborted"] != false
                    || flight["undeployed"]["aborted"] != false
                {
                    continue;
                }
                let at = format!("{} {}", design["file"], flight["name"]);
                // The flight's apogee is its summary's, which is its altitude column's peak.
                let deployed = flight["summary"]["max_altitude_m"].as_f64().unwrap();
                assert_eq!(flight["series"]["max_altitude_m"].as_f64(), Some(deployed));
                let held = flight["undeployed"]["max_altitude_m"].as_f64().unwrap();
                if early_chute(flight).unwrap().is_some() {
                    if held - deployed > sampling_m {
                        lowered += 1;
                    } else {
                        assert!((held - deployed).abs() <= sampling_m, "{at}");
                        within_sampling += 1;
                    }
                } else {
                    late += 1;
                    assert_eq!(deployed, held, "{at}: a late parachute moved the apogee");
                }
            }
        }
        // The six pod probes (M1.13c2) and four rod probes (M2.2e5) carry no parachute, so they
        // are among the late.
        assert_eq!((lowered, within_sampling, late), (14, 1, 51));
    }

    #[test]
    fn a_cause_is_sized_against_the_flight_without_it() {
        let part = "a1b2";
        let recorded = json!({
            "undeployed": { "aborted": false, "max_altitude_m": 200.0 },
            "undeployed_without_drag_overrides": {
                "aborted": false, "refused": null, "cleared": [part], "max_altitude_m": 160.0,
            },
            "undeployed_without_parts_set_to_no_drag": {
                "aborted": false, "refused": null, "removed": [part], "max_altitude_m": 250.0,
            },
        });
        let overrides = [DragOverride {
            id: part.to_owned(),
            name: "a transition".to_owned(),
            zero: true,
            removable: true,
        }];
        let entry = |early: Value, overridden: bool, probe: Option<f64>| {
            let mut entry = json!({
                "aborted": false,
                "deployed_before_apogee_s": early,
                "metrics": { "apogee_m": { "hpr": 168.0 } },
            });
            if overridden {
                entry["drag_overrides_not_applied"] = json!(["a transition"]);
            }
            if let Some(probe) = probe {
                entry["without_the_overridden_parts"] = json!({"apogee_m": {"hpr": probe}});
            }
            entry
        };
        let close = |value: &Value, expected: f64| {
            assert!(
                (value.as_f64().unwrap() - expected).abs() < 1e-12,
                "{value}"
            );
        };
        // An early parachute alone: held, 168 against 200, more than 5% off after.
        let sized = causes_removed(&recorded, &entry(json!(1.5), false, None), &[])
            .unwrap()
            .unwrap();
        close(&sized["parachutes_held"]["relative_percent"], -16.0);
        close(&sized["remaining_percent"], -16.0);
        assert_eq!(sized["within_5_percent"], false);
        assert!(sized["drag_overrides_cleared_too"].is_null());
        assert!(sized["parts_removed_from_both"].is_null());
        // A drag override with no early parachute: 168 against 160, within 5%.
        let sized = causes_removed(&recorded, &entry(Value::Null, true, None), &overrides)
            .unwrap()
            .unwrap();
        close(
            &sized["drag_overrides_cleared_too"]["relative_percent"],
            5.0,
        );
        close(&sized["remaining_percent"], 5.0);
        assert_eq!(sized["within_5_percent"], true);
        // With the probe, 240 against 250 is within 5% too; 270 against 250 is not.
        let sized = causes_removed(&recorded, &entry(json!(1.5), true, Some(240.0)), &overrides)
            .unwrap()
            .unwrap();
        close(&sized["parts_removed_from_both"]["relative_percent"], -4.0);
        close(&sized["remaining_percent"], 5.0);
        assert_eq!(sized["within_5_percent"], true);
        let sized = causes_removed(&recorded, &entry(json!(1.5), true, Some(270.0)), &overrides)
            .unwrap()
            .unwrap();
        close(&sized["parts_removed_from_both"]["relative_percent"], 8.0);
        close(&sized["remaining_percent"], 8.0);
        assert_eq!(sized["within_5_percent"], false);
        // A removed-part flight OpenRocket aborted leaves the cleared one, but not within.
        let mut failed = recorded.clone();
        failed["undeployed_without_parts_set_to_no_drag"]["aborted"] = json!(true);
        let probed = entry(json!(1.5), true, Some(240.0));
        let sized = causes_removed(&failed, &probed, &overrides)
            .unwrap()
            .unwrap();
        assert_eq!(
            sized["parts_removed_from_both"]["why"],
            "OpenRocket aborted it"
        );
        close(&sized["remaining_percent"], 5.0);
        assert_eq!(sized["within_5_percent"], false);
        // A part with no id is matched by count, as OpenRocket gives it a new one.
        let mut unnamed = overrides.to_vec();
        unnamed[0].id = String::new();
        assert!(causes_removed(&recorded, &entry(Value::Null, true, None), &unnamed).is_ok());
        unnamed.push(unnamed[0].clone());
        assert!(causes_removed(&recorded, &entry(Value::Null, true, None), &unnamed).is_err());
        let mut upper = overrides.clone();
        upper[0].id = part.to_uppercase();
        assert!(causes_removed(&recorded, &entry(Value::Null, true, None), &upper).is_ok());
        // Neither cause, an aborted flight or hpr's flight with no apogee is not sized.
        let none = |entry: &Value| causes_removed(&recorded, entry, &[]).unwrap().is_none();
        assert!(none(&entry(Value::Null, false, None)));
        let mut aborted = entry(json!(1.5), false, None);
        aborted["aborted"] = json!(true);
        assert!(none(&aborted));
        let mut no_apogee = entry(json!(1.5), false, None);
        no_apogee["metrics"]["apogee_m"]["hpr"] = Value::Null;
        assert!(none(&no_apogee));
        // A flight OpenRocket aborted, refused or failed has a reason and no difference.
        for (key, value, why) in [
            ("aborted", json!(true), "OpenRocket aborted it"),
            ("refused", json!("no motor"), "OpenRocket refused it"),
            ("driver_error", json!("boom"), "the oracle failed: boom"),
        ] {
            let mut failed = recorded.clone();
            failed["undeployed_without_drag_overrides"][key] = value;
            let sized = causes_removed(&failed, &entry(Value::Null, true, None), &overrides)
                .unwrap()
                .unwrap();
            assert_eq!(sized["drag_overrides_cleared_too"]["why"], why);
            assert!(sized["remaining_percent"].is_null());
            assert_eq!(sized["within_5_percent"], false);
        }
        // OpenRocket must have changed exactly the parts hpr reads, and a flight must be there.
        let mut other = recorded.clone();
        other["undeployed_without_drag_overrides"]["cleared"] = json!(["c3d4"]);
        assert!(causes_removed(&other, &entry(Value::Null, true, None), &overrides).is_err());
        let mut other = recorded.clone();
        other["undeployed_without_parts_set_to_no_drag"]["removed"] = json!([part, "c3d4"]);
        let probed = entry(Value::Null, true, Some(240.0));
        assert!(causes_removed(&other, &probed, &overrides).is_err());
        let mut missing = recorded.clone();
        missing["undeployed"]["max_altitude_m"] = Value::Null;
        assert!(causes_removed(&missing, &entry(json!(1.5), false, None), &[]).is_err());
        let mut missing = recorded.clone();
        missing["undeployed_without_parts_set_to_no_drag"] = Value::Null;
        assert!(causes_removed(&missing, &probed, &overrides).is_err());
    }

    #[test]
    fn the_causes_summary_counts_only_flights_with_a_difference() {
        let flight = |percent: f64, removed: Value| {
            json!({
                "metrics": { "apogee_m": { "relative_percent": percent } },
                "apogee_with_the_causes_removed": removed,
            })
        };
        let summary = with_the_causes_removed(&[
            // Over 5%, sized and brought within.
            flight(
                -16.0,
                json!({"remaining_percent": 1.0, "within_5_percent": true}),
            ),
            // Over 5%, sized and not.
            flight(
                12.0,
                json!({"remaining_percent": 7.0, "within_5_percent": false}),
            ),
            // Over 5%, but OpenRocket's flights without the cause have only reasons.
            flight(
                9.0,
                json!({"remaining_percent": null, "within_5_percent": false}),
            ),
            // Over 5% with no named cause.
            flight(-6.0, Value::Null),
            // Within 5%, sized.
            flight(
                -1.0,
                json!({"remaining_percent": -2.0, "within_5_percent": true}),
            ),
        ]);
        assert_eq!(summary["over_5_percent"], 4);
        assert_eq!(summary["over_5_percent_sized"], 2);
        assert_eq!(summary["over_5_percent_within_5_percent_after"], 1);
        assert_eq!(summary["remaining_percent"]["count"], 3);
        assert_eq!(summary["remaining_percent"]["max"], 7.0);
    }

    #[test]
    fn mass_and_cg_are_hprs_less_openrockets_and_skip_an_aborted_flight() {
        let flight = |aborted: bool| {
            json!({
                "aborted": aborted,
                "launch_mass_kg": { "openrocket": 2.0, "hpr": 2.02 },
                "at_rod_clearance": {
                    "openrocket": { "mass_kg": 1.6, "cg_from_nose_m": 1.0, "reference_length_m": 0.1 },
                    "hpr": { "mass_kg": 1.64, "cg_from_nose_m": 1.03, "reference_length_m": 0.2 },
                },
            })
        };
        let spreads = mass_and_cg(&[flight(false), flight(true)]);
        let only = |key: &str| {
            let s = &spreads[key];
            assert_eq!(s["count"], 1, "{key}");
            s["median"].as_f64().unwrap()
        };
        assert!((only("launch_mass_percent") - 1.0).abs() < 1e-12);
        assert!((only("rod_clearance_mass_percent") - 2.5).abs() < 1e-12);
        assert!((only("rod_clearance_cg_cal") - 0.3).abs() < 1e-12);
    }

    #[test]
    fn a_spread_is_the_count_median_mean_size_and_ends() {
        assert_eq!(
            spread(&[1.0, -2.0, 3.0, -4.0]),
            json!({ "count": 4, "median": -0.5, "mean_absolute": 2.5, "min": -4.0, "max": 3.0 })
        );
        assert_eq!(spread(&[]), json!({ "count": 0 }));
    }

    #[test]
    fn the_check_finds_a_moved_number_a_missing_key_and_a_changed_list() {
        let committed = json!({ "a": 1.0, "b": [1, 2], "c": "x" });
        let mut apart = Vec::new();
        same(
            &committed,
            &json!({ "a": 1.0 + 1e-12, "b": [1, 2], "c": "x" }),
            "",
            &mut apart,
        );
        assert!(apart.is_empty(), "{apart:?}");
        same(
            &committed,
            &json!({ "a": 1.001, "b": [1], "d": "x" }),
            "",
            &mut apart,
        );
        assert_eq!(apart.len(), 4, "{apart:?}");
    }

    #[test]
    fn a_designs_name_is_its_file_name() {
        assert_eq!(
            design_name(
                "refs/openrocket/OpenRocket-24.12.jar!datafiles/examples/Chute release.ork"
            ),
            "Chute release"
        );
        assert_eq!(
            design_name("validation/fixtures/ork/loft-demo/demo-stable.ork"),
            "demo-stable"
        );
    }

    #[test]
    fn a_curve_is_confirmed_only_when_openrocket_places_exactly_it() {
        use CurveSource::{Catalog, Digest, Other};
        let placed = |digests: &[&str]| digests.iter().map(|d| (*d).to_owned()).collect::<Vec<_>>();
        let two = placed(&["a", "b"]);
        assert_eq!(
            unconfirmed_curve(&[Digest("a"), Digest("b")], Some(&two)),
            None
        );
        assert_eq!(
            unconfirmed_curve(&[Digest("b"), Digest("a")], Some(&two)),
            None
        );
        // One too few, one too many, or one other, either way round.
        assert_eq!(
            unconfirmed_curve(&[Digest("a")], Some(&two)),
            Some(NOT_PLACED)
        );
        let repeated = placed(&["a", "a"]);
        assert_eq!(
            unconfirmed_curve(&[Digest("a")], Some(&repeated)),
            Some(NOT_PLACED)
        );
        assert_eq!(
            unconfirmed_curve(&[Digest("a"), Digest("a")], Some(&placed(&["a"]))),
            Some(NOT_PLACED)
        );
        assert_eq!(
            unconfirmed_curve(&[Digest("a"), Digest("c")], Some(&two)),
            Some(NOT_PLACED)
        );
        // No such configuration in the motor record, or a curve without a digest.
        assert_eq!(unconfirmed_curve(&[Digest("a")], None), Some(NOT_PLACED));
        assert_eq!(
            unconfirmed_curve(&[Other], Some(&placed(&[]))),
            Some(NOT_PLACED)
        );
        // A catalog curve is found by name, whatever OpenRocket places.
        assert_eq!(
            unconfirmed_curve(&[Digest("a"), Catalog], Some(&two)),
            Some(CURVE_BY_NAME)
        );
        assert_eq!(unconfirmed_curve(&[Catalog], None), Some(CURVE_BY_NAME));
    }

    #[test]
    fn only_a_rod_the_rail_takes_in_calm_standard_air_is_flown() {
        let calm = json!({
            "rod_angle_rad": 0.0, "wind_average_m_s": 0.0, "wind_turbulence": 0.0,
            "wind_model": "AVERAGE", "isa_atmosphere": true,
        });
        assert_eq!(unflown_conditions(&calm), Ok(None));
        let with = |key: &str, value: Value| {
            let mut conditions = calm.clone();
            conditions[key] = value;
            unflown_conditions(&conditions)
        };
        // A tilted rod is flown (M2.2e5), up to but not at the horizontal; one tilted back past
        // the vertical is not.
        assert_eq!(with("rod_angle_rad", json!(0.05)), Ok(None));
        assert_eq!(with("rod_angle_rad", json!(1.5)), Ok(None));
        for refused in [-0.05, FRAC_PI_2, 2.0] {
            assert_eq!(
                with("rod_angle_rad", json!(refused)),
                Ok(Some(ROD_NOT_TAKEN)),
                "{refused}"
            );
        }
        assert_eq!(with("wind_average_m_s", json!(2.0)), Ok(Some(WIND)));
        assert_eq!(with("wind_turbulence", json!(0.1)), Ok(Some(WIND)));
        assert_eq!(with("wind_model", json!("MULTI_LEVEL")), Ok(Some(WIND)));
        assert_eq!(
            with("isa_atmosphere", json!(false)),
            Ok(Some(NOT_STANDARD_AIR))
        );
        // A condition the record does not state is an error, not a reason.
        assert!(with("rod_angle_rad", Value::Null).is_err());
        assert!(with("wind_model", Value::Null).is_err());
        assert!(with("isa_atmosphere", Value::Null).is_err());
    }

    #[test]
    fn drag_overrides_are_read_with_their_value_and_place() {
        let text = |t: &str| json!({"kind": "text", "text": t});
        let element = |name: &str, children: Vec<Value>| json!({"kind": "element", "name": name, "attributes": [], "children": children});
        let part = |tag: &str, id: &str, cd: &str| {
            element(
                tag,
                vec![
                    element("id", vec![text(id)]),
                    element("name", vec![text(&format!("{tag} {id}"))]),
                    element("overridecd", vec![text(cd)]),
                ],
            )
        };
        let mut rocket = element(
            "rocket",
            vec![element(
                "subcomponents",
                vec![element(
                    "stage",
                    vec![
                        element("overridecd", vec![text("0.0")]),
                        element(
                            "subcomponents",
                            vec![
                                part("nosecone", "n", "0.0"),
                                part("transition", "t", "0.35"),
                            ],
                        ),
                    ],
                )],
            )],
        );
        rocket.as_object_mut().unwrap().remove("kind");
        let root = json!({"name": "openrocket", "attributes": [], "children": [
            {"kind": "element", "name": "rocket", "attributes": [],
             "children": rocket["children"]},
        ]});
        let root: ork::Element = serde_json::from_value(root).unwrap();
        let found = drag_overrides(&root);
        let summary: Vec<(&str, bool, bool)> = found
            .iter()
            .map(|o| (o.id.as_str(), o.zero, o.removable))
            .collect();
        assert_eq!(
            summary,
            vec![("", true, false), ("n", true, true), ("t", false, true)]
        );
    }
}
