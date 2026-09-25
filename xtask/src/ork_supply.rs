//! The thrust curves `cargo xtask ork` supplies from OpenRocket's own motor database (M2.2c2,
//! ADR-067), and the check that hpr integrates every curve it flies as OpenRocket does.
//!
//! `validation/oracles/openrocket/motor_database.py` writes [`RECORD`]: every motor in the database
//! OpenRocket 24.12 loads, with its digest, samples, envelope and OpenRocket's own integrals;
//! OpenRocket's reading of every curve a design embeds; and, per design, the digests of the motors
//! OpenRocket places in each configuration. From it the survey:
//!
//! - builds each solid motor as hpr builds a catalog one and supplies it for its digest
//!   ([`hpr_io::ork::SuppliedCurves`]), so a design flies the curve its digest names;
//! - holds hpr's total impulse to OpenRocket's within 0.1% on every solid database curve and every
//!   curve a design embeds, the bound M2.2c1 set for the bundled ones, and supplies no curve
//!   outside it;
//! - checks that OpenRocket places the very curves hpr is given in every configuration hpr flies
//!   with a supplied curve;
//! - follows every configuration the bundled catalog alone leaves without a curve, and says what
//!   became of it: flown, held back for another reason, or still without a curve, and why.
//!
//! The record lives under the gitignored `corpus-out/`: the database's terms are unstated and the
//! embedded curves belong to the designs' owners, so only counts leave this machine.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use hpr_io::ork::{Attachment, CaseSize, Curve, Design, NoCurve, NotFlown, SuppliedCurves};
use hpr_motor::{SolidMotor, ThrustCurve, rse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ork_motors::{no_curve, not_flown};

/// Where the oracle writes its record.
pub(crate) const RECORD: &str = "corpus-out/openrocket-motors.json";

/// The oracle.
const ORACLE: &str = "validation/oracles/openrocket/motor_database.py";

/// The scripts whose current text the record must have been written by: its name for each, and
/// where it is.
const SCRIPTS: [(&str, &str); 4] = [
    ("motor_database.py", ORACLE),
    ("motors.py", "validation/oracles/openrocket/motors.py"),
    (
        "automatic_radius.py",
        "validation/oracles/openrocket/automatic_radius.py",
    ),
    (
        "geometry.py",
        "validation/oracles/rocketserializer/geometry.py",
    ),
];

/// The OpenRocket release the record must come from, and its jar.
const OPENROCKET: &str = "24.12";
const JAR: &str = "refs/openrocket/OpenRocket-24.12.jar";

/// The bound on total impulse, relative to OpenRocket's: M2.2c's 0.1%.
const TOLERANCE: f64 = 1e-3;

/// Two repeats of a digest are the same motor when every envelope number agrees this closely.
const SAME_ENVELOPE: f64 = 1e-9;

/// What the supplied curves are called in a reason.
const SOURCE: &str = "OpenRocket 24.12's motor database";

/// hpr's total impulse against OpenRocket's, over a set of curves.
#[derive(Debug, Default)]
struct Held {
    compared: usize,
    within: usize,
    /// The largest `|hpr / OpenRocket - 1|` within the bound.
    largest: f64,
}

impl Held {
    /// Compares one curve, and returns the relative difference when it is outside the bound.
    fn compare(&mut self, hpr_ns: f64, openrocket_ns: f64) -> Option<f64> {
        self.compared += 1;
        let apart = (hpr_ns / openrocket_ns - 1.0).abs();
        // A NaN, from a missing or zero OpenRocket impulse, is outside too.
        if apart.is_nan() || apart > TOLERANCE {
            return Some(apart);
        }
        self.largest = self.largest.max(apart);
        self.within += 1;
        None
    }

    fn line(&self) -> String {
        format!(
            "{} of {} within 0.1%, the largest difference {:.3e}",
            self.within, self.compared, self.largest
        )
    }

    fn summary(&self) -> Value {
        json!({
            "compared": self.compared,
            "within_0_1_percent": self.within,
            "largest_relative": self.largest,
        })
    }
}

/// The digests OpenRocket places in each configuration of a design, by lowercase configuration id.
type Placed = BTreeMap<String, Vec<String>>;

/// A database motor, built, before it is supplied: its samples and envelope, to compare a repeat
/// of its digest with, its name, and where OpenRocket puts its centre of mass.
#[derive(Debug)]
struct Built {
    samples: (Vec<f64>, Vec<f64>),
    envelope: [f64; 4],
    named: String,
    cg_off_middle_m: f64,
    case: CaseSize,
    motor: SolidMotor,
}

/// The supplied curves, the checks on them, and what became of the configurations they were for.
#[derive(Debug, Default)]
pub(crate) struct Supply {
    present: bool,
    curves: SuppliedCurves,
    database_motors: usize,
    hybrid_motors: usize,
    hybrids: BTreeSet<String>,
    /// Digests two database motors hold with different samples or envelopes, with their names:
    /// never supplied.
    conflicting: BTreeMap<String, String>,
    /// Digests more than one database motor holds, alike.
    repeated: usize,
    /// Database motors hpr could not build or that fail the impulse check, by name, and why; their
    /// digests are not supplied.
    refused: BTreeMap<String, String>,
    refused_digests: BTreeSet<String>,
    /// Manufacturer and designation pairs the database holds under more than one digest.
    names_with_several_curves: usize,
    /// How far OpenRocket puts each supplied motor's centre of mass from its case's middle, m.
    cg_off_middle_m: BTreeMap<String, f64>,
    database_held: Held,
    /// OpenRocket's reading of each embedded curve, by the entry's SHA-256: the total impulse of
    /// each motor in it, or why it refused the file.
    embedded_record: BTreeMap<String, Result<Vec<f64>, String>>,
    embedded_seen: BTreeSet<String>,
    embedded_held: Held,
    /// What OpenRocket places in each configuration, by the design's SHA-256 and the lowercase
    /// configuration id: `Ok(None)` for a design it does not open, `Err` for an entry the oracle
    /// could not complete.
    placed: BTreeMap<String, Result<Option<Placed>, String>>,
    /// Configurations flown with a supplied curve: those whose supplied curves OpenRocket places
    /// too, the motors in them, and those in designs OpenRocket does not open.
    confirmed: [usize; 2],
    unopened: usize,
    /// The digests of the supplied curves in the configurations the designs fly.
    used: BTreeSet<String>,
    /// Each configuration the bundled catalog alone leaves without a curve, by what became of it.
    fates: BTreeMap<String, usize>,
    problems: Vec<String>,
}

impl Supply {
    /// Reads the record, when the survey covers the library and the record is there.
    pub(crate) fn load(root: &Path, library: bool) -> Result<Self, String> {
        let path = root.join(RECORD);
        if !library || !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{RECORD}: {error}"))?;
        let record: Value =
            serde_json::from_str(&text).map_err(|error| format!("{RECORD}: {error}"))?;
        let mut supply = Self::of(&record);
        let mut current = BTreeMap::new();
        for (script, at) in SCRIPTS {
            let bytes = std::fs::read(root.join(at)).map_err(|error| format!("{at}: {error}"))?;
            current.insert(script, sha256(&bytes));
        }
        let jar = root.join(JAR);
        let jar = jar
            .is_file()
            .then(|| std::fs::read(&jar).map(|bytes| sha256(&bytes)))
            .transpose()
            .map_err(|error| format!("{JAR}: {error}"))?;
        supply
            .problems
            .extend(stale(&record, &current, jar.as_deref()));
        Ok(supply)
    }

    fn of(record: &Value) -> Self {
        let mut supply = Self {
            present: true,
            curves: SuppliedCurves::new(SOURCE),
            ..Self::default()
        };
        for key in ["database", "embedded", "flown"] {
            if !record[key].is_array() {
                supply.problems.push(format!(
                    "{RECORD} has no `{key}` list; run the oracle again"
                ));
            }
        }
        let mut built: BTreeMap<String, Built> = BTreeMap::new();
        let mut names: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for motor in record["database"].as_array().into_iter().flatten() {
            supply.database_motors += 1;
            let digest = motor["digest"].as_str().unwrap_or_default().to_owned();
            if motor["motor_type"] == "Hybrid" {
                supply.hybrid_motors += 1;
                supply.hybrids.insert(digest);
                continue;
            }
            let text = |key: &str| motor[key].as_str().unwrap_or("?").to_owned();
            let numbers = |key: &str| -> Vec<f64> {
                motor[key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|v| v.as_f64().unwrap_or(f64::NAN))
                    .collect()
            };
            let number = |key: &str| motor[key].as_f64().unwrap_or(f64::NAN);
            names
                .entry((text("manufacturer"), text("designation")))
                .or_default()
                .insert(digest.clone());
            let named = format!(
                "{} {} ({})",
                text("manufacturer"),
                text("designation"),
                digest.get(..8).unwrap_or(&digest)
            );
            let samples = (numbers("time_s"), numbers("thrust_n"));
            let curve = match ThrustCurve::new(samples.0.clone(), samples.1.clone()) {
                Ok(curve) => curve,
                Err(error) => {
                    supply.refuse(&digest, named, format!("its curve does not build: {error}"));
                    continue;
                }
            };
            if let Some(apart) = supply
                .database_held
                .compare(curve.total_impulse_ns(), number("total_impulse_ns"))
            {
                let why = format!("hpr's total impulse is {apart:.3e} from OpenRocket's");
                supply.problems.push(format!("{named}: {why}"));
                supply.refuse(&digest, named, why);
                continue;
            }
            let case = CaseSize {
                diameter_m: number("diameter_m"),
                length_m: number("length_m"),
            };
            let envelope = [
                case.diameter_m,
                case.length_m,
                number("propellant_mass_kg"),
                number("launch_mass_kg"),
            ];
            let solid = match SolidMotor::from_envelope(
                curve,
                envelope[0],
                envelope[1],
                envelope[2],
                envelope[3],
            ) {
                Ok(solid) => solid,
                Err(error) => {
                    supply.refuse(&digest, named, format!("its motor does not build: {error}"));
                    continue;
                }
            };
            let cg_off_middle_m = number("launch_cg_x_m") - 0.5 * case.length_m;
            match built.get(&digest) {
                None => {
                    built.insert(
                        digest,
                        Built {
                            samples,
                            envelope,
                            named,
                            cg_off_middle_m,
                            case,
                            motor: solid,
                        },
                    );
                }
                Some(first) if first.samples == samples && alike(&first.envelope, &envelope) => {
                    supply.repeated += 1;
                }
                Some(first) => {
                    let pair = format!("{} and {named}", first.named);
                    supply.conflicting.insert(digest, pair);
                }
            }
        }
        supply.names_with_several_curves = names.values().filter(|d| d.len() > 1).count();
        for (digest, first) in built {
            if supply.conflicting.contains_key(&digest) || supply.refused_digests.contains(&digest)
            {
                continue;
            }
            match supply
                .curves
                .insert(digest.clone(), first.case, first.motor)
            {
                Ok(_) => {
                    supply.cg_off_middle_m.insert(digest, first.cg_off_middle_m);
                }
                Err(error) => {
                    supply.refuse(&digest, first.named, format!("its case: {error}"));
                }
            }
        }
        if supply.database_held.compared == 0 {
            supply.problems.push(format!(
                "{RECORD} holds no solid motor to check; run the oracle again"
            ));
        }
        for curve in record["embedded"].as_array().into_iter().flatten() {
            let sha = curve["sha256"].as_str().unwrap_or_default().to_owned();
            let reading = match curve["driver_error"].as_str() {
                Some(error) => Err(error.to_owned()),
                None => Ok(curve["motors"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|m| m["total_impulse_ns"].as_f64().unwrap_or(f64::NAN))
                    .collect()),
            };
            supply.embedded_record.insert(sha, reading);
        }
        for design in record["flown"].as_array().into_iter().flatten() {
            let sha = design["sha256"].as_str().unwrap_or_default().to_owned();
            let entry = match (
                design["opens"].as_bool(),
                design["configurations"].as_object(),
            ) {
                (Some(false), _) => Ok(None),
                (Some(true), Some(configurations)) => Ok(Some(
                    configurations
                        .iter()
                        .map(|(id, digests)| {
                            let digests = digests
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|d| d.as_str().map(str::to_owned))
                                .collect();
                            (id.to_ascii_lowercase(), digests)
                        })
                        .collect(),
                )),
                _ => Err(design["driver_error"]
                    .as_str()
                    .unwrap_or("an entry with no configurations")
                    .to_owned()),
            };
            supply.placed.insert(sha, entry);
        }
        supply
    }

    fn refuse(&mut self, digest: &str, named: String, why: String) {
        self.refused.insert(named, why);
        self.refused_digests.insert(digest.to_owned());
    }

    /// The digests the record finds OpenRocket placing in configuration `id` of the design whose
    /// SHA-256 is `sha`: nothing when OpenRocket has no configuration of that id. A design the
    /// record does not read back, or that the oracle failed on, is an error: the record is to be
    /// written again, not the configuration left out.
    pub(crate) fn placed(&self, sha: &str, id: &str) -> Result<Option<&[String]>, String> {
        let short = sha.get(..8).unwrap_or(sha);
        match self.placed.get(sha) {
            Some(Ok(Some(configurations))) => Ok(configurations
                .get(&id.to_ascii_lowercase())
                .map(Vec::as_slice)),
            Some(Ok(None)) => Err(format!(
                "OpenRocket did not open design {short} in {RECORD}, though it flew it"
            )),
            Some(Err(error)) => Err(format!(
                "the oracle failed on design {short} in {RECORD}: {error}"
            )),
            None => Err(format!(
                "design {short} is not read back in {RECORD}; run {ORACLE} again, with --jar"
            )),
        }
    }

    /// Whether the survey has the record.
    pub(crate) fn is_present(&self) -> bool {
        self.present
    }

    /// The curves to read designs with: none when the record is absent.
    pub(crate) fn curves(&self) -> &SuppliedCurves {
        &self.curves
    }

    /// Holds every curve a design embeds to OpenRocket's reading of the same bytes.
    pub(crate) fn hold_embedded(&mut self, attachments: &[Attachment]) {
        if !self.present {
            return;
        }
        for attachment in attachments {
            let name = &attachment.name;
            if !(name.starts_with("thrustcurves/") && name.to_ascii_lowercase().ends_with(".rse")) {
                continue;
            }
            let sha = sha256(&attachment.bytes);
            if !self.embedded_seen.insert(sha.clone()) {
                continue;
            }
            let short = sha.get(..12).unwrap_or(&sha).to_owned();
            let openrocket = match self.embedded_record.get(&sha) {
                None => {
                    self.problems.push(format!(
                        "the embedded curve {short} is not in {RECORD}; run {ORACLE} again"
                    ));
                    continue;
                }
                Some(Err(error)) => {
                    self.problems.push(format!(
                        "OpenRocket refused the embedded curve {short}: {error}"
                    ));
                    continue;
                }
                Some(Ok(impulses)) => impulses.clone(),
            };
            let engines = std::str::from_utf8(&attachment.bytes)
                .map_err(|error| error.to_string())
                .and_then(|text| rse::parse(text).map_err(|error| error.to_string()))
                .map(|parsed| parsed.value.engines);
            let engines = match engines {
                Ok(engines) if engines.len() == openrocket.len() => engines,
                Ok(engines) => {
                    self.problems.push(format!(
                        "the embedded curve {short}: hpr reads {} engine(s), OpenRocket {}",
                        engines.len(),
                        openrocket.len()
                    ));
                    continue;
                }
                Err(error) => {
                    self.problems
                        .push(format!("the embedded curve {short} does not read: {error}"));
                    continue;
                }
            };
            // Both read a file's engines in the order it lists them.
            for (engine, openrocket_ns) in engines.iter().zip(openrocket) {
                match engine.thrust_curve() {
                    Ok(curve) => {
                        if let Some(apart) = self
                            .embedded_held
                            .compare(curve.total_impulse_ns(), openrocket_ns)
                        {
                            self.problems.push(format!(
                                "the embedded curve {short}: hpr's total impulse is {apart:.3e} \
                                 from OpenRocket's"
                            ));
                        }
                    }
                    Err(error) => self.problems.push(format!(
                        "the embedded curve {short} does not build: {error}"
                    )),
                }
            }
        }
    }

    /// Follows each configuration `bare` (read with the bundled catalog alone) leaves without a
    /// curve into `supplied` (read with the supply), and counts what became of it; and checks
    /// that OpenRocket places the supplied curves of each configuration flown in the design whose
    /// file is `bytes`.
    pub(crate) fn follow(&mut self, bytes: &[u8], bare: &Design, supplied: &Design) {
        if !self.present {
            return;
        }
        let sha = sha256(bytes);
        let short = sha.get(..8).unwrap_or(&sha).to_owned();
        for configuration in &supplied.motors.configurations {
            if configuration.left_out.is_some() {
                continue;
            }
            let digests: Vec<&str> = configuration
                .motors
                .iter()
                .filter_map(|motor| match &motor.curve {
                    Curve::Supplied { digest, .. } => Some(digest.as_str()),
                    _ => None,
                })
                .collect();
            if digests.is_empty() {
                continue;
            }
            self.used.extend(digests.iter().map(|d| (*d).to_owned()));
            let id = &configuration.id;
            let placed = match self.placed.get(&sha) {
                None => {
                    self.problems.push(format!(
                        "design {short} flies supplied curves and is not read back in {RECORD}; \
                         run {ORACLE} again, with --jar"
                    ));
                    continue;
                }
                Some(Err(error)) => {
                    self.problems.push(format!(
                        "design {short} flies supplied curves, and the oracle failed on it: {error}"
                    ));
                    continue;
                }
                Some(Ok(None)) => {
                    self.unopened += 1;
                    continue;
                }
                Some(Ok(Some(configurations))) => {
                    // OpenRocket keys a configuration by a lowercase UUID, and re-keys an id that
                    // is not one.
                    match configurations.get(&id.to_ascii_lowercase()) {
                        Some(placed) => placed,
                        None => {
                            self.problems.push(format!(
                                "design {short}: OpenRocket has no configuration `{id}`, so its \
                                 supplied curves cannot be checked"
                            ));
                            continue;
                        }
                    }
                }
            };
            let mut left: Vec<&str> = placed.iter().map(String::as_str).collect();
            let missing = digests
                .iter()
                .filter(|digest| match left.iter().position(|d| d == *digest) {
                    Some(at) => {
                        left.swap_remove(at);
                        false
                    }
                    None => true,
                })
                .count();
            if missing == 0 {
                self.confirmed[0] += 1;
                self.confirmed[1] += digests.len();
            } else {
                self.problems.push(format!(
                    "design {short}, configuration {id}: OpenRocket does not place {missing} of \
                     the {} curve(s) hpr is supplied",
                    digests.len()
                ));
            }
        }
        for configuration in &bare.motors.configurations {
            if configuration.left_out.as_ref().map(|out| out.why) != Some(NotFlown::NoCurve) {
                continue;
            }
            let now = supplied
                .motors
                .configurations
                .iter()
                .find(|c| c.id == configuration.id);
            let fate = match now.map(|c| (c, c.left_out.as_ref().map(|out| out.why))) {
                None => "not read again".to_owned(),
                Some((_, None)) => "flies".to_owned(),
                Some((now, Some(NotFlown::NoCurve))) => {
                    let why = now
                        .motors
                        .iter()
                        .find_map(|motor| match &motor.curve {
                            Curve::Unresolved { why, .. } => {
                                Some(self.why(motor.digest.as_deref(), *why))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| "no curve".to_owned());
                    format!("still no curve: {why}")
                }
                Some((_, Some(why))) => {
                    format!("held back for another reason: {}", not_flown(why))
                }
            };
            *self.fates.entry(fate).or_default() += 1;
        }
    }

    /// Why a motor still has no curve, naming what the database says of its digest.
    fn why(&self, digest: Option<&str>, why: NoCurve) -> String {
        if why != NoCurve::NotFound {
            return no_curve(why).to_owned();
        }
        let Some(digest) = digest else {
            return "it records no digest, and the bundled catalog has no such motor".to_owned();
        };
        if self.hybrids.contains(digest) {
            "its digest is a hybrid's in the database".to_owned()
        } else if self.conflicting.contains_key(digest) {
            "two different database motors share its digest".to_owned()
        } else if self.refused_digests.contains(digest) {
            "the database's motor for its digest was refused".to_owned()
        } else {
            "its digest is not in the database, and the bundled catalog has no such motor"
                .to_owned()
        }
    }

    /// How many motors OpenRocket gives a centre of mass more than a millimetre from mid-case,
    /// and the largest offset (m): over every supplied motor, and over those the designs fly.
    fn cg_counts(&self) -> [(usize, f64); 2] {
        let count = |offsets: Vec<f64>| {
            let off: Vec<f64> = offsets.into_iter().filter(|d| *d > 1e-3).collect();
            (off.len(), off.iter().copied().fold(0.0, f64::max))
        };
        [
            count(self.cg_off_middle_m.values().map(|d| d.abs()).collect()),
            count(
                self.used
                    .iter()
                    .filter_map(|d| self.cg_off_middle_m.get(d))
                    .map(|d| d.abs())
                    .collect(),
            ),
        ]
    }

    pub(crate) fn summary(&self) -> Value {
        if !self.present {
            return Value::Null;
        }
        let [all, used] = self.cg_counts();
        json!({
            "database_motors": self.database_motors,
            "hybrid_motors": self.hybrid_motors,
            "hybrid_digests": self.hybrids.len(),
            "digests_supplied": self.curves.len(),
            "digests_repeated_alike": self.repeated,
            "digests_held_by_different_motors": self.conflicting,
            "motors_refused": self.refused,
            "names_with_several_curves": self.names_with_several_curves,
            "database_total_impulse": self.database_held.summary(),
            "embedded_total_impulse": self.embedded_held.summary(),
            "openrocket_places_the_supplied_curves": {
                "configurations": self.confirmed[0],
                "motors": self.confirmed[1],
                "configurations_in_designs_openrocket_does_not_open": self.unopened,
            },
            "centre_of_mass_off_mid_case_over_1_mm": {
                "supplied": all.0,
                "supplied_largest_m": all.1,
                "flown": used.0,
                "flown_largest_m": used.1,
                "supplied_curves_flown": self.used.len(),
            },
            "configurations_without_a_curve_from_the_bundled_catalog_alone":
                self.fates.values().sum::<usize>(),
            "what_became_of_them": self.fates,
        })
    }

    pub(crate) fn print(&self) {
        if !self.present {
            println!(
                "  curves from OpenRocket's motor database: not run; {ORACLE} writes {RECORD}"
            );
            return;
        }
        let listed = |map: &BTreeMap<String, String>| {
            if map.is_empty() {
                return String::new();
            }
            let items: Vec<String> = map.iter().map(|(a, b)| format!("{a}: {b}")).collect();
            format!(" ({})", items.join("; "))
        };
        println!(
            "  curves from {SOURCE} ({RECORD}): {} motors, {} digests supplied; not supplied: {} \
             hybrid motor(s) under {} digest(s), {} digest(s) held by different motors{}, {} \
             motor(s) refused{}; {} digest(s) repeated alike; {} name(s) held under more than one \
             digest",
            self.database_motors,
            self.curves.len(),
            self.hybrid_motors,
            self.hybrids.len(),
            self.conflicting.len(),
            if self.conflicting.is_empty() {
                String::new()
            } else {
                let pairs: Vec<&str> = self.conflicting.values().map(String::as_str).collect();
                format!(" ({})", pairs.join("; "))
            },
            self.refused.len(),
            listed(&self.refused),
            self.repeated,
            self.names_with_several_curves
        );
        println!(
            "    total impulse against OpenRocket's, every solid database curve: {}",
            self.database_held.line()
        );
        println!(
            "    total impulse against OpenRocket's, every distinct embedded curve: {}",
            self.embedded_held.line()
        );
        println!(
            "    OpenRocket places the supplied curves in each configuration hpr flies with one: {} \
             configuration(s), {} motor(s); {} configuration(s) in designs OpenRocket does not open",
            self.confirmed[0], self.confirmed[1], self.unopened
        );
        let [all, used] = self.cg_counts();
        println!(
            "    centre of mass more than 1 mm from mid-case in OpenRocket (hpr puts it there): {} \
             of {} supplied motors, up to {:.1} mm; {} of the {} the designs fly, up to {:.1} mm",
            all.0,
            self.curves.len(),
            all.1 * 1e3,
            used.0,
            self.used.len(),
            used.1 * 1e3
        );
        println!(
            "    configurations without a curve from the bundled catalog alone: {}; with the \
             database: {}",
            self.fates.values().sum::<usize>(),
            self.fates
                .iter()
                .map(|(fate, n)| format!("{fate} {n}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    /// Why the survey should fail: a stale or incomplete record, or a curve outside 0.1%.
    pub(crate) fn failure(&self) -> Option<String> {
        (!self.problems.is_empty()).then(|| self.problems.join("; "))
    }
}

/// What makes `record` stale: a script it was written by that has changed since (`current`, by
/// name), another OpenRocket release, or another jar than the pinned one (`jar`, when present).
fn stale(record: &Value, current: &BTreeMap<&str, String>, jar: Option<&str>) -> Vec<String> {
    let mut problems = Vec::new();
    for (script, sha) in current {
        if record["inputs_sha256"][*script].as_str() != Some(sha.as_str()) {
            problems.push(format!(
                "{RECORD} was written by another version of {script}; run {ORACLE} again"
            ));
        }
    }
    if record["openrocket"].as_str() != Some(OPENROCKET) {
        problems.push(format!(
            "{RECORD} is from OpenRocket {}, not {OPENROCKET}",
            record["openrocket"]
        ));
    }
    if let Some(jar) = jar
        && record["jar_sha256"].as_str() != Some(jar)
    {
        problems.push(format!("{RECORD} was written with another jar than {JAR}"));
    }
    problems
}

/// Whether two envelopes (diameter, length, propellant and loaded mass) are the same motor's.
fn alike(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a.iter()
        .zip(b)
        .all(|(x, y)| (x - y).abs() <= SAME_ENVELOPE * x.abs().max(y.abs()))
}

pub(crate) fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn motor(digest: &str, kind: &str, thrust: f64, impulse: f64, length_m: f64) -> Value {
        json!({
            "digest": digest, "manufacturer": "Nobody", "designation": "G100",
            "motor_type": kind, "diameter_m": 0.029, "length_m": length_m,
            "propellant_mass_kg": 0.06, "launch_mass_kg": 0.15, "launch_cg_x_m": 0.5 * length_m,
            "time_s": [0.0, 0.1, 1.0, 1.1], "thrust_n": [0.0, thrust, thrust, 0.0],
            "total_impulse_ns": impulse,
        })
    }

    /// Two motors sharing a digest with different samples, two with different cases, one repeated
    /// exactly, one hybrid, one whose impulse OpenRocket reports 1% higher, and one that builds.
    #[test]
    fn a_record_is_screened_before_its_curves_are_supplied() {
        let record = json!({
            "database": [
                motor("aa", "Reloadable", 100.0, 100.0, 0.124),
                motor("bb", "Single-use", 100.0, 100.0, 0.124),
                motor("bb", "Single-use", 100.0, 100.0, 0.124),
                motor("cc", "Single-use", 100.0, 100.0, 0.124),
                motor("cc", "Single-use", 90.0, 90.0, 0.124),
                motor("dd", "Hybrid", 100.0, 100.0, 0.124),
                motor("ee", "Single-use", 100.0, 101.0, 0.124),
                motor("ff", "Single-use", 100.0, 100.0, 0.124),
                motor("ff", "Single-use", 100.0, 100.0, 0.150),
            ],
            "embedded": [],
            "flown": [],
        });
        let supply = Supply::of(&record);
        assert_eq!(supply.database_motors, 9);
        assert!(supply.hybrids.contains("dd"));
        assert_eq!(
            supply.conflicting.keys().collect::<Vec<_>>(),
            ["cc", "ff"],
            "different samples, and a different case"
        );
        assert_eq!(supply.repeated, 1);
        let supplied: Vec<&str> = ["aa", "bb", "cc", "dd", "ee", "ff"]
            .into_iter()
            .filter(|d| supply.curves().get(d).is_some())
            .collect();
        assert_eq!(supplied, ["aa", "bb"], "the 1% curve is not supplied");
        assert_eq!(
            (supply.database_held.compared, supply.database_held.within),
            (8, 7)
        );
        let failure = supply.failure().expect("the 1% curve fails the survey");
        assert!(failure.contains("Nobody G100 (ee)"), "{failure}");
        assert_eq!(
            supply.why(Some("cc"), NoCurve::NotFound),
            "two different database motors share its digest"
        );
        assert_eq!(
            supply.why(Some("dd"), NoCurve::NotFound),
            "its digest is a hybrid's in the database"
        );
        assert_eq!(
            supply.why(Some("ee"), NoCurve::NotFound),
            "the database's motor for its digest was refused"
        );
        assert_eq!(supply.why(None, NoCurve::Hybrid), "a hybrid");
    }

    /// An empty or incomplete record, or one written by other scripts or another OpenRocket,
    /// fails the survey rather than passing with nothing checked.
    #[test]
    fn a_stale_or_empty_record_fails() {
        let empty = Supply::of(&json!({ "database": [] }));
        let failure = empty.failure().expect("nothing checked");
        assert!(failure.contains("no solid motor"), "{failure}");
        assert!(failure.contains("no `embedded` list"), "{failure}");
        assert!(failure.contains("no `flown` list"), "{failure}");

        let record = json!({
            "openrocket": "24.12",
            "jar_sha256": "j",
            "inputs_sha256": {
                "motor_database.py": "a", "motors.py": "b", "automatic_radius.py": "c",
                "geometry.py": "d",
            },
        });
        let current = BTreeMap::from([
            ("motor_database.py", "a".to_owned()),
            ("motors.py", "b".to_owned()),
            ("automatic_radius.py", "c".to_owned()),
            ("geometry.py", "d".to_owned()),
        ]);
        assert!(stale(&record, &current, Some("j")).is_empty());
        assert!(stale(&record, &current, None).is_empty());
        let mut edited = current.clone();
        edited.insert("motors.py", "b2".to_owned());
        let problems = stale(&record, &edited, Some("j"));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("motors.py"), "{problems:?}");
        assert_eq!(stale(&record, &current, Some("j2")).len(), 1);
        let mut newer = record.clone();
        newer["openrocket"] = json!("25.03");
        assert_eq!(stale(&newer, &current, Some("j")).len(), 1);
    }

    /// Embedded curves are held to OpenRocket's reading of the same bytes; one it refused, or one
    /// missing from the record, fails the survey with that said.
    #[test]
    fn embedded_curves_are_held_to_openrocket() {
        let rse = r#"<engine-database><engine-list>
          <engine mfg="Nobody" code="G100T" Type="single-use" dia="29." len="124." initWt="150."
            propWt="60." delays="6" Itot="102.5" burn-time="1.1"><data>
            <eng-data t="0." f="0." m="60."/><eng-data t="0.05" f="100." m="58."/>
            <eng-data t="1." f="100." m="3."/><eng-data t="1.1" f="0." m="0."/>
          </data></engine></engine-list></engine-database>"#;
        let attachment = |text: &str| Attachment {
            name: "thrustcurves/d1935f00.rse".to_owned(),
            bytes: text.as_bytes().to_vec(),
        };
        let good = attachment(rse);
        let refused = attachment(&rse.replace("G100T", "G100X"));
        let missing = attachment(&rse.replace("G100T", "G100Y"));
        let record = json!({
            "database": [motor("aa", "Reloadable", 100.0, 100.0, 0.124)],
            "embedded": [
                { "sha256": sha256(&good.bytes), "motors": [{ "total_impulse_ns": 102.5 }] },
                { "sha256": sha256(&refused.bytes), "driver_error": "a reason" },
            ],
            "flown": [],
        });
        let mut supply = Supply::of(&record);
        supply.hold_embedded(&[good.clone(), good]);
        assert_eq!(
            (supply.embedded_held.compared, supply.embedded_held.within),
            (1, 1),
            "each distinct curve once"
        );
        assert!(supply.failure().is_none(), "{:?}", supply.failure());
        supply.hold_embedded(&[refused, missing]);
        let failure = supply.failure().expect("two problems");
        assert!(
            failure.contains("OpenRocket refused the embedded curve"),
            "{failure}"
        );
        assert!(failure.contains("is not in"), "{failure}");
    }

    /// A configuration flown with supplied curves passes only when OpenRocket places the same
    /// curves in the configuration of the same id; a design OpenRocket does not open is counted
    /// apart, and one missing from the record fails.
    #[test]
    fn openrocket_must_place_the_supplied_curves() {
        let xml = br#"<?xml version='1.0' encoding='utf-8'?>
<openrocket version="1.9" creator="test">
  <rocket><subcomponents><stage><name>Sustainer</name><subcomponents>
    <bodytube><id>body</id><length>0.4</length><thickness>0.001</thickness>
      <radius>0.0145</radius>
      <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
        <overhang>0.0</overhang>
        <motor configid="c1"><type>single</type><manufacturer>Nobody</manufacturer>
          <digest>aa</digest><designation>G100</designation>
          <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>
      </motormount></bodytube>
  </subcomponents></stage></subcomponents></rocket>
</openrocket>"#;
        let file = hpr_io::ork::read(xml).expect("a readable design").value;
        let sha = sha256(xml);
        let checked = |placed: Value| {
            let mut supply = Supply::of(&json!({
                "database": [motor("aa", "Reloadable", 100.0, 100.0, 0.124)],
                "embedded": [],
                "flown": [placed],
            }));
            let supplied = hpr_io::ork::design_with(&file, supply.curves()).value;
            let bare = hpr_io::ork::design(&file).value;
            supply.follow(xml, &bare, &supplied);
            supply
        };
        let opened = |configurations: Value| json!({ "sha256": sha, "opens": true, "configurations": configurations });
        let good = checked(opened(json!({ "C1": ["aa"] })));
        assert_eq!((good.confirmed, good.unopened), ([1, 1], 0));
        assert_eq!(good.fates.get("flies"), Some(&1));
        assert!(good.failure().is_none(), "{:?}", good.failure());

        let other = checked(opened(json!({ "c1": ["bb"] })));
        let failure = other.failure().expect("another curve placed");
        assert!(failure.contains("does not place 1 of the 1"), "{failure}");

        let rekeyed = checked(opened(json!({ "c2": ["aa"] })));
        let failure = rekeyed.failure().expect("no such configuration");
        assert!(failure.contains("has no configuration `c1`"), "{failure}");

        let crashed = checked(json!({ "sha256": sha, "driver_error": "a crash" }));
        let failure = crashed.failure().expect("the oracle failed");
        assert!(
            failure.contains("the oracle failed on it: a crash"),
            "{failure}"
        );

        let refused = checked(json!({ "sha256": sha, "opens": false }));
        assert_eq!((refused.confirmed, refused.unopened), ([0, 0], 1));
        assert!(refused.failure().is_none());

        let absent = checked(json!({ "sha256": "elsewhere", "opens": true, "configurations": {} }));
        let failure = absent.failure().expect("not read back");
        assert!(failure.contains("is not read back"), "{failure}");
    }
}
