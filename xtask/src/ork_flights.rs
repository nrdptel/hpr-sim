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
//! summarised apart. The configurations the record holds that hpr does not fly are listed with
//! the importer's reason.
//!
//! The motor curves come from OpenRocket's own database by digest (ADR-067), whose record lives
//! under the gitignored `corpus-out/`, and the examples from the pinned jar: so the flights run
//! only where both are fetched. What they write, [`REPORT_JSON`] and [`REPORT_MD`], is committed,
//! and a test holds its reference values to the record and its outcomes to [`compare`] in CI.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use hpr_aero::Flow;
use hpr_core::geodesy::Geodetic;
use hpr_io::ork;
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
    for entry in designs {
        let file = entry["file"].as_str().ok_or("a design without a file")?;
        let Some(recorded) = entry["flights"].as_array() else {
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
        let flown = fly_design(file, &name, &bytes, recorded, &supply, label, false)?;
        flights.extend(flown.flights);
        not_flown.extend(flown.not_flown);
    }
    let summary = summarise(&flights, &not_flown);
    Ok(json!({
        "generated_by": "cargo xtask ork-flights",
        "record": RECORD,
        "reference": { "tool": "OpenRocket", "version": OPENROCKET_MEASURED },
        "summary": summary,
        "flights": flights,
        "not_flown": not_flown,
    }))
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
/// `list_failures`, when the configuration is listed as [`FLIGHT_FAILED`] and the detail, which can
/// name a part, is printed on this machine only.
pub(crate) fn fly_design(
    file: &str,
    name: &str,
    bytes: &[u8],
    recorded: &[Value],
    supply: &crate::ork_supply::Supply,
    label: impl Fn(usize, &Value) -> String,
    list_failures: bool,
) -> Result<Flown, String> {
    let read = ork::read(bytes).map_err(|error| format!("{name}: {error}"))?;
    let design = ork::design_with(&read.value, supply.curves()).value;
    let sha = crate::ork_supply::sha256(bytes);
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
        let flew = |rocket: &hpr_design::Rocket| {
            fly(name, rocket, &configuration.id, &motors, flight).map_err(|error| {
                if list_failures {
                    eprintln!("{motors}: {FLIGHT_FAILED}: {error}");
                }
                error
            })
        };
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
            causes_removed(flight, &entry, overrides.len()).map_err(|e| format!("{motors}: {e}"))?
        {
            entry["apogee_with_the_causes_removed"] = sized;
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

/// Why a configuration is not flown: a tilted launch rod.
pub(crate) const ROD_NOT_VERTICAL: &str = "a launch rod not vertical";

/// Why a configuration is not flown: wind.
pub(crate) const WIND: &str = "wind";

/// Why a configuration is not flown: an atmosphere other than the standard one.
pub(crate) const NOT_STANDARD_AIR: &str = "an atmosphere other than the standard one";

/// Why a configuration is not flown: a curve from hpr's bundled catalog, found by name.
pub(crate) const CURVE_BY_NAME: &str = "a curve found by name, which OpenRocket may not fly";

/// Why a configuration is not flown: the motor record does not find OpenRocket placing its curves.
pub(crate) const NOT_PLACED: &str = "a curve OpenRocket is not shown to place";

/// Why the recorded launch `conditions` are not ones this report flies: it flies a vertical rod
/// in calm standard air only, as [`fly`] requires. A condition the record does not state is an
/// error, not a reason.
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
    Ok(if rod != 0.0 {
        Some(ROD_NOT_VERTICAL)
    } else if wind != 0.0 || turbulence != 0.0 || model != "AVERAGE" {
        // `flights.py` calms the average wind model only.
        Some(WIND)
    } else if !standard {
        Some(NOT_STANDARD_AIR)
    } else {
        None
    })
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

/// The centre of mass's height at the start, and its largest speed up to apogee, from the dense
/// output. hpr flies no recovery from a `.ork`, so its fall is unbraked and is left out: the
/// reference's peak speed is on the way up, and a free fall could outrun it.
#[derive(Default)]
struct Peaks {
    start_height_m: Option<f64>,
    max_speed_m_s: Option<f64>,
    climbed: bool,
    past_apogee: bool,
}

impl Observer for Peaks {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let (start, end) = (step.start_s(), step.end_s());
        if self.start_height_m.is_none() {
            self.start_height_m = Some(step.sample(start)?.height_above_ground_m);
        }
        for k in 1..=4 {
            if self.past_apogee {
                break;
            }
            let t = if k == 4 {
                end
            } else {
                start + (end - start) * f64::from(k) / 4.0
            };
            let sample = step.sample(t)?;
            if sample.vertical_speed_m_s > 0.0 {
                self.climbed = true;
            } else if self.climbed {
                self.past_apogee = true;
            }
            let speed = sample.cg_velocity_enu_m_s.length();
            if self.max_speed_m_s.is_none_or(|max| speed > max) {
                self.max_speed_m_s = Some(speed);
            }
        }
        Ok(())
    }
}

/// Flies one configuration in the recorded conditions and compares it with the record.
fn fly(
    design: &str,
    rocket: &hpr_design::Rocket,
    configuration: &str,
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
    if number(&conditions["rod_angle_rad"], "rod angle")? != 0.0
        || number(&conditions["wind_average_m_s"], "wind")? != 0.0
        || conditions["isa_atmosphere"] != true
    {
        return Err(format!(
            "{at}: only a vertical rod in calm standard air is flown here"
        ));
    }
    let rod_length_m = number(&conditions["rod_length_m"], "rod length")?;
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
    let simulation = Simulation::new(
        rocket,
        configuration,
        environment,
        Rail::vertical(rod_length_m),
        settings,
    )
    .map_err(|error| format!("{at}: {error}"))?;
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

    // The margin at the recorded rod-clearance step: hpr's mass at its time, and its centre of
    // pressure at its Mach number with the air along the axis.
    let assembly = simulation.assembly();
    let clearance_time_s = number(&clearance["time_s"], "rod-clearance time")?;
    let clearance_mach = number(&clearance["mach"], "rod-clearance Mach number")?;
    let mass = assembly.mass_properties(clearance_time_s);
    let cg_m = -mass.cg_m.z;
    let cp_m = simulation
        .aero()
        .normal_force(&Flow::axial(clearance_mach))
        .map_err(|error| format!("{at}: {error}"))?
        .cp_station_m;
    let reference_m = assembly.layout.reference_diameter_m;
    let margin_cal = cp_m.map(|cp| (cp - cg_m) / reference_m);

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
    Ok(json!({
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
            },
            "hpr": {
                "mass_kg": mass.mass_kg,
                "cg_from_nose_m": cg_m,
                "cp_from_nose_m": cp_m,
                "reference_length_m": reference_m,
            },
        },
        "launch_mass_kg": {
            "openrocket": recorded["series"]["launch_mass_kg"],
            "hpr": assembly.mass_properties(0.0).mass_kg,
        },
    }))
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
/// against it (M2.2e4, decision ADR-073), or `None` for an aborted flight or one with neither
/// cause. `overrides` is how many parts state a drag coefficient hpr reads and does not apply.
///
/// - `parachutes_held`: the record's `undeployed` flight, the same one with nothing deployed. A
///   parachute that opens before apogee lowers the apogee, and hpr flies none from a `.ork`.
/// - `drag_overrides_cleared_too`, where a part states a drag coefficient: its
///   `undeployed_without_drag_overrides` flight, with nothing deployed and every stated coefficient
///   cleared, which is OpenRocket flying what hpr reads, since hpr applies none (#165).
///
/// `remaining_percent` is hpr's apogee against the last of them, in per cent of OpenRocket's:
/// what the named causes leave unexplained.
///
/// - `parts_removed_from_both`, where hpr has flown its probe without the parts set to no drag
///   (`without_the_overridden_parts`): that probe's apogee against OpenRocket's
///   `undeployed_without_parts_set_to_no_drag` flight, the same rocket with the same parts gone
///   and nothing deployed. It asks whether what remains is the parts' own drag.
fn causes_removed(
    recorded: &Value,
    entry: &Value,
    overrides: usize,
) -> Result<Option<Value>, String> {
    let early = !entry["deployed_before_apogee_s"].is_null();
    if entry["aborted"] == true || (!early && overrides == 0) {
        return Ok(None);
    }
    let hpr = entry["metrics"]["apogee_m"]["hpr"]
        .as_f64()
        .ok_or("hpr's flight has no apogee")?;
    let step = |key: &str| -> Result<Value, String> {
        let held = &recorded[key];
        if held["aborted"] != false {
            return Err(format!("OpenRocket's {key} flight is missing or aborted"));
        }
        let openrocket = held["max_altitude_m"]
            .as_f64()
            .filter(|m| m.is_finite() && *m > 0.0)
            .ok_or_else(|| format!("the record's {key} flight has no apogee"))?;
        Ok(json!({
            "openrocket_m": openrocket,
            "relative_percent": 100.0 * (hpr - openrocket) / openrocket,
        }))
    };
    let held = step("undeployed")?;
    let mut remaining = held["relative_percent"].clone();
    let mut sized = json!({ "parachutes_held": held });
    if overrides > 0 {
        let cleared =
            recorded["undeployed_without_drag_overrides"]["drag_overrides_cleared"].as_u64();
        if cleared != Some(overrides as u64) {
            return Err(format!(
                "OpenRocket cleared {cleared:?} stated drag coefficients, hpr reads {overrides}"
            ));
        }
        let too = step("undeployed_without_drag_overrides")?;
        remaining = too["relative_percent"].clone();
        sized["drag_overrides_cleared_too"] = too;
    }
    if let Some(probe) = entry["without_the_overridden_parts"]["apogee_m"]["hpr"].as_f64() {
        let key = "undeployed_without_parts_set_to_no_drag";
        if recorded[key]["parts_removed"].as_u64() != Some(overrides as u64) {
            return Err(format!(
                "OpenRocket removed {:?} parts set to no drag, hpr removed {overrides}",
                recorded[key]["parts_removed"].as_u64()
            ));
        }
        let mut both = step(key)?;
        let openrocket = both["openrocket_m"].as_f64().unwrap_or(f64::NAN);
        both["hpr_m"] = json!(probe);
        both["relative_percent"] = json!(100.0 * (probe - openrocket) / openrocket);
        sized["parts_removed_from_both"] = both;
    }
    sized["remaining_percent"] = remaining;
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

/// The spread of what a named cause leaves of hpr's apogee difference, over the flights that have
/// one (M2.2e4), and how many of those more than 5% from OpenRocket's are still more than 5% from
/// its flight with the causes removed.
fn with_the_causes_removed(flights: &[Value]) -> Value {
    let over = |percent: Option<f64>| percent.is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT);
    let (mut remaining, mut still_over) = (Vec::new(), 0);
    for flight in flights {
        let Some(left) = flight["apogee_with_the_causes_removed"]["remaining_percent"].as_f64()
        else {
            continue;
        };
        remaining.push(left);
        if over(flight["metrics"]["apogee_m"]["relative_percent"].as_f64()) && over(Some(left)) {
            still_over += 1;
        }
    }
    json!({
        "remaining_percent": spread(&remaining),
        "over_5_percent_still_over": still_over,
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

/// Neither.
pub(crate) const NO_NAMED_CAUSE: &str = "no named cause";

/// The named cause a flight's difference in `metric` is summarised under. A drag override moves
/// every metric but the margin; an early parachute only the apogee. A flight with both is put
/// under the drag override.
pub(crate) fn cause(flight: &Value, metric: FlightMetric) -> &'static str {
    if metric == FlightMetric::RodClearanceStability {
        NO_NAMED_CAUSE
    } else if !flight["drag_overrides_not_applied"].is_null() {
        DRAG_OVERRIDE
    } else if metric == FlightMetric::Apogee && !flight["deployed_before_apogee_s"].is_null() {
        EARLY_CHUTE
    } else {
        NO_NAMED_CAUSE
    }
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
         flew each flight with a named cause again without it: first with nothing deployed, then, \
         where a part states its own drag coefficient, with every such statement cleared as well, \
         which is what hpr reads. Δ is hpr's apogee less that flight's, in per cent of it.\n\n\
         | design | motors | Δ apogee | chute early (s) | OR, nothing deployed (m) | Δ \
         | OR, drag coefficients cleared too (m) | Δ | OR, the parts removed (m) | hpr (m) | Δ |\n\
         |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for flight in sized {
        let removed = &flight["apogee_with_the_causes_removed"];
        let (held, too) = (
            &removed["parachutes_held"],
            &removed["drag_overrides_cleared_too"],
        );
        let both = &removed["parts_removed_from_both"];
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            flight["design"].as_str().unwrap_or_default(),
            flight["motors"].as_str().unwrap_or_default(),
            percent(&flight["metrics"]["apogee_m"]),
            fixed(&flight["deployed_before_apogee_s"], 2),
            fixed(&held["openrocket_m"], 1),
            signed(&held["relative_percent"], 2),
            fixed(&too["openrocket_m"], 1),
            signed(&too["relative_percent"], 2),
            fixed(&both["openrocket_m"], 1),
            fixed(&both["hpr_m"], 1),
            signed(&both["relative_percent"], 2),
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
            "apogee against OpenRocket's own flight with the named causes removed",
            &sized["remaining_percent"],
            "%",
            2,
        ));
        out.push_str(&format!(
            "- apogees more than 5% off that are still more than 5% off with the causes removed: \
             {}\n",
            sized["over_5_percent_still_over"],
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

#[cfg(test)]
mod tests {
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
        // M2.2d2's scope: the 21 configurations, in five of the jar's examples, that hpr flies.
        assert_eq!(report["flights"].as_array().unwrap().len(), 21);
    }

    #[test]
    fn the_reference_values_are_the_records_and_the_outcomes_are_compares() {
        let (record, report) = committed();
        let tool = openrocket();
        for flight in report["flights"].as_array().unwrap() {
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

    #[test]
    fn every_apogee_more_than_5_percent_off_has_a_named_cause() {
        // M2.2's parent asks a written cause for each. This report has two: a reference parachute
        // open before apogee, which hpr does not fly from a `.ork`, and a part whose drag
        // OpenRocket is told is zero, which hpr cannot yet be told.
        let (_, report) = committed();
        for flight in report["flights"].as_array().unwrap() {
            let percent = flight["metrics"]["apogee_m"]["relative_percent"].as_f64();
            if percent.is_some_and(|p| p.abs() > APOGEE_CAUSE_PERCENT) {
                assert!(
                    !flight["deployed_before_apogee_s"].is_null()
                        || !flight["drag_overrides_not_applied"].is_null(),
                    "{} {} is {percent:?}% off with no cause written",
                    flight["design"],
                    flight["motors"]
                );
            }
        }
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
    fn only_a_vertical_rod_in_calm_standard_air_is_flown() {
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
        assert_eq!(
            with("rod_angle_rad", json!(0.05)),
            Ok(Some(ROD_NOT_VERTICAL))
        );
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
