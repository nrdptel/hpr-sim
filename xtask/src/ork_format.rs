//! The hpr-format counts `cargo xtask ork` prints (M3.3a): each design written as an hpr design
//! document, checked against the committed schema, read back, written as a `.ork`, read again,
//! and flown from each end.

use std::collections::BTreeMap;

use hpr_core::geodesy::Geodetic;
use hpr_format::{Content, DesignFile, Provenance, Source, SourceFormat};
use hpr_io::ork::{self, Attachment, Design, SuppliedCurves, export};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};
use serde_json::{Value, json};

/// How far apart two apogees of one design may be, relative, when it went `.ork` → hpr design
/// format → `.ork` between them: M3.3's first *done when*.
pub(crate) const APOGEE_RELATIVE: f64 = 1e-9;

/// The committed schema, which every document is checked against.
const SCHEMA: &str = "schema/format/hpr-design-0.2.schema.json";

/// The counts, summed over the designs.
pub(crate) struct FormatTally {
    validator: jsonschema::Validator,
    designs: usize,
    /// Documents the committed schema takes.
    valid: usize,
    /// Documents that read back as the document written.
    read_back: usize,
    /// Designs whose `.ork` written from the document is the `.ork` hpr writes from the file
    /// itself, with the archive's other entries (M3.2a).
    same_ork: usize,
    /// The archive's other entries the documents carry, as text and as base64.
    attachments: BTreeMap<String, usize>,
    /// Designs that read back from that `.ork` as first read.
    same_design: usize,
    /// Documents that, rewritten as version 0.1 (`source_files` named `attachments`, and no
    /// `airframe_not_as_written`, which 0.1 didn't record), migrate back to the same document but
    /// for that reason.
    migrated: usize,
    /// Documents whose `.ork` airframe was not read exactly as written, and of them, those whose
    /// migration from 0.1 recovers the reason, and those it marks unknown.
    not_as_written: usize,
    recovered: usize,
    unknown: usize,
    /// Documents read as written that the migration marks unknown, which refuses another motor in
    /// them though it could fly.
    unknown_as_written: usize,
    /// Documents whose migration gives a wrong reason, or none where there is one: a failure.
    wrong_reason: Vec<String>,
    /// Motor configurations the designs leave out of their rockets, as the `.ork` reader does:
    /// no curve, a size that disagrees, a part read simpler (ADR-055). None has a flight to compare.
    left_out: usize,
    /// Configurations flown three ways, and the largest relative apogee difference.
    flown: usize,
    largest_relative: f64,
    /// Configurations that could not be flown, by why, the same all three ways.
    not_flown: BTreeMap<String, usize>,
    /// Configurations whose three flights disagree: flown one way and not another, or apart by more
    /// than [`APOGEE_RELATIVE`].
    apart: Vec<String>,
    /// Schema errors, by message.
    schema_errors: BTreeMap<String, usize>,
    failed: Vec<String>,
}

impl FormatTally {
    /// A tally checking documents against the schema committed under `root`.
    pub(crate) fn new(root: &std::path::Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(root.join(SCHEMA)).map_err(|e| format!("{SCHEMA}: {e}"))?;
        let schema: Value = serde_json::from_str(&text).map_err(|e| format!("{SCHEMA}: {e}"))?;
        let validator = jsonschema::validator_for(&schema).map_err(|e| format!("{SCHEMA}: {e}"))?;
        Ok(Self {
            validator,
            designs: 0,
            valid: 0,
            read_back: 0,
            same_ork: 0,
            attachments: BTreeMap::new(),
            same_design: 0,
            migrated: 0,
            not_as_written: 0,
            recovered: 0,
            unknown: 0,
            unknown_as_written: 0,
            wrong_reason: Vec::new(),
            left_out: 0,
            flown: 0,
            largest_relative: 0.0,
            not_flown: BTreeMap::new(),
            apart: Vec::new(),
            schema_errors: BTreeMap::new(),
            failed: Vec::new(),
        })
    }

    /// Takes `design`, read with `curves` from `bytes`, whose archive's other entries are
    /// `attachments`, through the format and back; returns the per-file detail.
    pub(crate) fn add(
        &mut self,
        name: &str,
        (bytes, attachments): (&[u8], &[Attachment]),
        design: &Design,
        curves: &SuppliedCurves,
    ) -> Value {
        self.designs += 1;
        let mut source = Source::of(SourceFormat::Ork, bytes);
        source.airframe_not_as_written = ork::read(bytes)
            .ok()
            .and_then(|file| ork::airframe_not_as_written(&file.value));
        let airframe = source.airframe_not_as_written.clone();
        let document = DesignFile::new(design.clone(), Provenance::hpr(Some(source)), attachments);
        for file in &document.source_files {
            let key = match file.content {
                Content::Text(_) => "text",
                _ => "base64",
            };
            *self.attachments.entry(key.to_owned()).or_default() += 1;
        }
        let text = match hpr_format::to_json(&document) {
            Ok(text) => text,
            Err(error) => {
                self.failed.push(name.to_owned());
                return json!({ "error": error.to_string() });
            }
        };
        let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        let errors: Vec<String> = self
            .validator
            .iter_errors(&value)
            .map(|error| format!("{}: {error}", error.instance_path()))
            .collect();
        if errors.is_empty() {
            self.valid += 1;
        }
        for error in &errors {
            // By the schema's complaint, without the value, which can be a design's.
            let message = error.split_once(": ").map_or(error.as_str(), |(_, m)| m);
            let kind = message.rsplit(" is ").next().unwrap_or(message);
            *self.schema_errors.entry(kind.to_owned()).or_default() += 1;
        }
        let back = match hpr_format::from_json(&text) {
            Ok(back) => back,
            Err(error) => {
                self.failed.push(name.to_owned());
                return json!({ "error": error.to_string() });
            }
        };
        let read_back = back == document;
        if read_back {
            self.read_back += 1;
        }
        let migrated = as_version_0_1(&value)
            .and_then(|old| hpr_format::read_json(&old).ok())
            .filter(|opened| opened.written_as.to_string() == "0.1")
            .map(|opened| opened.value);
        let recovered = migrated.as_ref().and_then(|migrated| {
            let source = migrated.provenance.source.as_ref()?;
            Some(source.airframe_not_as_written.clone())
        });
        let migrated = migrated.is_some_and(|mut migrated| {
            if let Some(source) = migrated.provenance.source.as_mut() {
                source.airframe_not_as_written.clone_from(&airframe);
            }
            migrated == document
        });
        if migrated {
            self.migrated += 1;
        }
        let unknown = Some(hpr_format::migrate::UNKNOWN);
        match (
            airframe.as_deref(),
            recovered.as_ref().map(Option::as_deref),
        ) {
            (Some(truth), Some(Some(reason))) if reason == truth => {
                self.not_as_written += 1;
                self.recovered += 1;
            }
            (Some(_), Some(reason)) if reason == unknown => {
                self.not_as_written += 1;
                self.unknown += 1;
            }
            (None, Some(reason)) if reason == unknown => self.unknown_as_written += 1,
            (None, Some(None)) => {}
            _ => {
                self.not_as_written += usize::from(airframe.is_some());
                self.wrong_reason.push(name.to_owned());
            }
        }
        // The `.ork` M3.2a writes from the file itself, its other entries and all.
        let (Ok(through), Ok(direct)) = (back.to_ork(), export::write(design, attachments)) else {
            self.failed.push(name.to_owned());
            return json!({ "error": "a .ork was not written" });
        };
        let same_ork = through.value == direct.value;
        if same_ork {
            self.same_ork += 1;
        }
        let Ok(file) = ork::read(&through.value) else {
            self.failed.push(name.to_owned());
            return json!({ "error": "the .ork written from the document does not read" });
        };
        let again = ork::design_with(&file.value, curves).value;
        let same_design = again == *design;
        if same_design {
            self.same_design += 1;
        }
        self.left_out += design
            .motors
            .configurations
            .iter()
            .filter(|configuration| configuration.left_out.is_some())
            .count();
        let mut flights = Vec::new();
        for configuration in &design.rocket.configurations {
            let id = &configuration.id;
            let before = apogee(design, id);
            let from_document = apogee(&back.design(), id);
            let after = apogee(&again, id);
            let detail = match (&before, &from_document, &after) {
                (Ok(before), Ok(from_document), Ok(after)) => {
                    let relatives = [from_document, after].map(|apogee| (apogee - before) / before);
                    // A NaN is apart, and is not lost to `max`.
                    let within = relatives
                        .iter()
                        .all(|relative| relative.abs() <= APOGEE_RELATIVE);
                    let relative = relatives.iter().map(|r| r.abs()).fold(0.0_f64, f64::max);
                    self.flown += 1;
                    self.largest_relative = self.largest_relative.max(relative);
                    if !within {
                        self.apart.push(format!("{name} {id}"));
                    }
                    json!({ "configuration": id, "apogee_m": before, "relative": relative })
                }
                (Err(why), Err(why_document), Err(why_after))
                    if why == why_document && why == why_after =>
                {
                    *self.not_flown.entry(kind_of(why)).or_default() += 1;
                    json!({ "configuration": id, "not_flown": why })
                }
                _ => {
                    self.apart.push(format!("{name} {id}"));
                    json!({ "configuration": id, "apart": [
                        format!("{before:?}"), format!("{from_document:?}"), format!("{after:?}")
                    ] })
                }
            };
            flights.push(detail);
        }
        json!({
            "schema_errors": errors,
            "read_back_the_same": read_back,
            "ork_same_as_the_designs_own": same_ork,
            "design_read_back_the_same": same_design,
            "migrated_from_0_1_the_same": migrated,
            "flights": flights,
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "designs": self.designs,
            "valid": self.valid,
            "read_back_the_same": self.read_back,
            "ork_same_as_the_designs_own": self.same_ork,
            "attachments": self.attachments,
            "design_read_back_the_same": self.same_design,
            "migrated_from_0_1_the_same": self.migrated,
            "airframe_not_as_written": self.not_as_written,
            "airframe_recovered_by_the_migration": self.recovered,
            "airframe_marked_unknown_by_the_migration": self.unknown,
            "as_written_marked_unknown_by_the_migration": self.unknown_as_written,
            "airframe_wrong_after_the_migration": self.wrong_reason.len(),
            "configurations_left_out": self.left_out,
            "configurations_flown": self.flown,
            "largest_relative_apogee_difference": self.largest_relative,
            "not_flown": self.not_flown,
            "apart": self.apart.len(),
            "schema_errors": self.schema_errors,
            "failed": self.failed.len(),
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  through the hpr design format: {} design(s); {} valid against {SCHEMA}, {} read \
             back the same, {} write the .ork hpr writes from the file, {} read back from it the same, \
             {} migrate from 0.1 the same; {} failed",
            self.designs,
            self.valid,
            self.read_back,
            self.same_ork,
            self.same_design,
            self.migrated,
            self.failed.len()
        );
        println!(
            "    airframe not read exactly as written: {} design(s); the migration from 0.1 \
             recovers the reason for {} and marks {} unknown; of the others, it marks {} \
             unknown; {} wrong",
            self.not_as_written,
            self.recovered,
            self.unknown,
            self.unknown_as_written,
            self.wrong_reason.len()
        );
        println!(
            "    flown three ways (as read, from the document, from its .ork): {} configuration(s), apogees at most {:e} apart (relative; \
             {APOGEE_RELATIVE:e} allowed); {} apart; {} left out of the rockets, as read",
            self.flown,
            self.largest_relative,
            self.apart.len(),
            self.left_out
        );
        crate::ork::print_counts("archive entries carried", &self.attachments);
        crate::ork::print_counts(
            "configurations not flown, the same all three ways",
            &self.not_flown,
        );
        crate::ork::print_counts("schema errors", &self.schema_errors);
    }

    /// Why the survey should fail: a design that does not come through the format as it went in.
    pub(crate) fn failure(&self) -> Option<String> {
        let counts = [
            self.valid,
            self.read_back,
            self.same_ork,
            self.same_design,
            self.migrated,
        ];
        let short = counts.iter().any(|&count| count != self.designs);
        let wrong = !self.wrong_reason.is_empty();
        (short || wrong || !self.apart.is_empty() || !self.failed.is_empty()).then(|| {
            format!(
                "through the hpr design format, of {} design(s): {} valid, {} read back the same, \
                 {} write the same .ork, {} read back from it the same, {} migrate from 0.1 the \
                 same, {} with a wrong airframe reason; {} configuration(s) apart; {} failed",
                self.designs,
                self.valid,
                self.read_back,
                self.same_ork,
                self.same_design,
                self.migrated,
                self.wrong_reason.len(),
                self.apart.len(),
                self.failed.len()
            )
        })
    }
}

/// The document `value` of the current version as version 0.1 would have held it, the reverse of
/// the one migration there is: `source_files` named `attachments`, in its place, and no
/// `airframe_not_as_written` in the source.
fn as_version_0_1(value: &Value) -> Option<String> {
    let mut old: serde_json::Map<String, Value> = value
        .as_object()?
        .iter()
        .map(|(key, value)| match key.as_str() {
            "version" => ("version".to_owned(), json!("0.1")),
            "source_files" => ("attachments".to_owned(), value.clone()),
            _ => (key.clone(), value.clone()),
        })
        .collect();
    if let Some(source) = old
        .get_mut("provenance")
        .and_then(|provenance| provenance.get_mut("source"))
        .and_then(Value::as_object_mut)
    {
        source.remove("airframe_not_as_written");
    }
    serde_json::to_string_pretty(&Value::Object(old)).ok()
}

/// Why a configuration was not flown, without the part or design names it can hold.
fn kind_of(why: &str) -> String {
    why.split_once(':').map_or(why, |(kind, _)| kind).to_owned()
}

/// Flies configuration `id` of `design`, calm and standard at sea level off a 1.5 m vertical rail,
/// with its stages' separation if it has one, past any design errors; returns the apogee's height
/// above the ground, m, or why there is none.
fn apogee(design: &Design, id: &str) -> Result<f64, String> {
    let site = Geodetic::from_degrees(0.0, 0.0, 0.0).map_err(|e| format!("site: {e}"))?;
    let environment = Environment::standard(site).map_err(|e| format!("environment: {e}"))?;
    let settings = FlightSettings {
        accept_design_errors: true,
        ..FlightSettings::default()
    };
    let simulation = Simulation::new(
        &design.rocket,
        id,
        environment,
        Rail::vertical(1.5),
        settings,
    )
    .map_err(|e| format!("set up: {e}"))?;
    let staging = design
        .motors
        .configurations
        .iter()
        .find(|configuration| configuration.id == id)
        .and_then(|configuration| configuration.staging.as_ref());
    let simulation = match staging {
        Some(staging) => {
            crate::ork_flights::staged(simulation, staging).map_err(|e| format!("staging: {e}"))?
        }
        None => simulation,
    };
    let result = simulation
        .run(&mut ())
        .map_err(|e| format!("flight: {e}"))?;
    result
        .event(EventKind::Apogee)
        .map(|event| event.sample.height_above_ground_m)
        .ok_or_else(|| "flight: no apogee".to_owned())
}
