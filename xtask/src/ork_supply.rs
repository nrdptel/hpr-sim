//! The thrust curves `cargo xtask ork` supplies from OpenRocket's own motor database (M2.2c2,
//! ADR-067), and the check that hpr integrates every curve it flies as OpenRocket does.
//!
//! `validation/oracles/openrocket/motor_database.py` writes [`RECORD`]: every motor in the database
//! OpenRocket 24.12 loads, with its digest, samples, envelope and OpenRocket's own integrals, and
//! OpenRocket's reading of every curve a design embeds. From it the survey:
//!
//! - builds each solid motor as hpr builds a catalog one and supplies it for its digest
//!   ([`hpr_io::ork::SuppliedCurves`]), so a design flies the curve its digest names;
//! - holds hpr's total impulse to OpenRocket's within 0.1% on every database curve it builds and
//!   every curve a design embeds, the bound M2.2c1 set for the bundled ones;
//! - follows every configuration the bundled catalog alone leaves without a curve, and says what
//!   became of it: flown, held back for another reason, or still without a curve, and why.
//!
//! The record lives under the gitignored `corpus-out/`: the database's terms are unstated and the
//! embedded curves belong to the designs' owners, so only counts leave this machine.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use hpr_io::ork::{Attachment, Curve, Design, NoCurve, NotFlown, SuppliedCurves};
use hpr_motor::{SolidMotor, ThrustCurve, rse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ork_motors::{no_curve, not_flown};

/// Where the oracle writes its record.
pub(crate) const RECORD: &str = "corpus-out/openrocket-motors.json";

/// The oracle, whose current text the record must have been written by.
const ORACLE: &str = "validation/oracles/openrocket/motor_database.py";

/// The bound on total impulse, relative to OpenRocket's: M2.2c's 0.1%.
const TOLERANCE: f64 = 1e-3;

/// What the supplied curves are called in a reason.
const SOURCE: &str = "OpenRocket 24.12's motor database";

/// A database motor's case diameter and length, m, and the motor hpr builds from it.
type Case = (f64, f64, SolidMotor);

/// hpr's total impulse against OpenRocket's, over a set of curves.
#[derive(Debug, Default)]
struct Held {
    compared: usize,
    within: usize,
    /// The largest `|hpr / OpenRocket - 1|`.
    largest: f64,
}

impl Held {
    /// Compares one curve, and returns the relative difference when it is outside the bound.
    fn compare(&mut self, hpr_ns: f64, openrocket_ns: f64) -> Option<f64> {
        self.compared += 1;
        let apart = (hpr_ns / openrocket_ns - 1.0).abs();
        self.largest = self.largest.max(apart);
        if apart <= TOLERANCE {
            self.within += 1;
            None
        } else {
            Some(apart)
        }
    }

    fn line(&self) -> String {
        format!(
            "{} of {} within 0.1%, the largest difference {:.3e}",
            self.within, self.compared, self.largest
        )
    }

    fn summary(&self) -> Value {
        json!({ "compared": self.compared, "within_0_1_percent": self.within, "largest_relative": self.largest })
    }
}

/// The supplied curves, the checks on them, and what became of the configurations they were for.
#[derive(Debug, Default)]
pub(crate) struct Supply {
    present: bool,
    curves: SuppliedCurves,
    database_motors: usize,
    hybrids: BTreeSet<String>,
    /// Digests two database motors hold with different samples: never supplied.
    conflicting: BTreeSet<String>,
    /// Digests more than one database motor holds, with the same samples.
    repeated: usize,
    /// Database curves hpr could not build, by why, and their digests.
    unbuilt: BTreeMap<String, usize>,
    unbuilt_digests: BTreeSet<String>,
    database_held: Held,
    /// OpenRocket's total impulse of each embedded curve's motors, by the entry's SHA-256.
    embedded_record: BTreeMap<String, Vec<f64>>,
    embedded_seen: BTreeSet<String>,
    embedded_held: Held,
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
        let oracle =
            std::fs::read(root.join(ORACLE)).map_err(|error| format!("{ORACLE}: {error}"))?;
        let mut supply = Self::of(&record);
        if record["inputs_sha256"]["motor_database.py"].as_str() != Some(sha256(&oracle).as_str()) {
            supply.problems.push(format!(
                "{RECORD} was written by another version of {ORACLE}; run it again"
            ));
        }
        Ok(supply)
    }

    fn of(record: &Value) -> Self {
        let mut supply = Self {
            present: true,
            curves: SuppliedCurves::new(SOURCE),
            ..Self::default()
        };
        // Each digest's first motor: its samples, to compare a repeat with, and its case.
        let mut built: BTreeMap<String, (Vec<f64>, Vec<f64>, Case)> = BTreeMap::new();
        for motor in record["database"].as_array().into_iter().flatten() {
            supply.database_motors += 1;
            let digest = motor["digest"].as_str().unwrap_or_default().to_owned();
            if motor["motor_type"] == "Hybrid" {
                supply.hybrids.insert(digest);
                continue;
            }
            let numbers = |key: &str| -> Vec<f64> {
                motor[key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|v| v.as_f64().unwrap_or(f64::NAN))
                    .collect()
            };
            let number = |key: &str| motor[key].as_f64().unwrap_or(f64::NAN);
            let (times, thrusts) = (numbers("time_s"), numbers("thrust_n"));
            let named = format!(
                "{} {} ({})",
                motor["manufacturer"].as_str().unwrap_or("?"),
                motor["designation"].as_str().unwrap_or("?"),
                &digest[..digest.len().min(8)]
            );
            let curve = match ThrustCurve::new(times.clone(), thrusts.clone()) {
                Ok(curve) => curve,
                Err(error) => {
                    supply.unbuilt(&digest, &format!("the curve: {}", variant(&error)));
                    continue;
                }
            };
            if let Some(apart) = supply
                .database_held
                .compare(curve.total_impulse_ns(), number("total_impulse_ns"))
            {
                supply.problems.push(format!(
                    "{named}: hpr's total impulse is {apart:.3e} from OpenRocket's"
                ));
            }
            let (diameter_m, length_m) = (number("diameter_m"), number("length_m"));
            let solid = match SolidMotor::from_envelope(
                curve,
                diameter_m,
                length_m,
                number("propellant_mass_kg"),
                number("launch_mass_kg"),
            ) {
                Ok(solid) => solid,
                Err(error) => {
                    supply.unbuilt(&digest, &format!("the motor: {}", variant(&error)));
                    continue;
                }
            };
            match built.get(&digest) {
                None => {
                    built.insert(digest, (times, thrusts, (diameter_m, length_m, solid)));
                }
                Some((t, f, ..)) if *t == times && *f == thrusts => supply.repeated += 1,
                Some(_) => {
                    supply.conflicting.insert(digest);
                }
            }
        }
        for (digest, (_, _, (diameter_m, length_m, solid))) in built {
            if !supply.conflicting.contains(&digest) {
                supply.curves.insert(digest, diameter_m, length_m, solid);
            }
        }
        for curve in record["embedded"].as_array().into_iter().flatten() {
            let sha = curve["sha256"].as_str().unwrap_or_default().to_owned();
            let impulses = curve["motors"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|m| m["total_impulse_ns"].as_f64().unwrap_or(f64::NAN))
                .collect();
            supply.embedded_record.insert(sha, impulses);
        }
        supply
    }

    fn unbuilt(&mut self, digest: &str, why: &str) {
        *self.unbuilt.entry(why.to_owned()).or_default() += 1;
        self.unbuilt_digests.insert(digest.to_owned());
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
            let short = &sha[..12];
            let Some(openrocket) = self.embedded_record.get(&sha).cloned() else {
                self.problems.push(format!(
                    "the embedded curve {short} is not in {RECORD}; run {ORACLE} again"
                ));
                continue;
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
    /// curve into `supplied` (read with the supply), and counts what became of it.
    pub(crate) fn follow(&mut self, bare: &Design, supplied: &Design) {
        if !self.present {
            return;
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
            let fate = match now.and_then(|c| c.left_out.as_ref().map(|out| (c, out.why))) {
                None if now.is_some() => "flies".to_owned(),
                None => "not read again".to_owned(),
                Some((now, NotFlown::NoCurve)) => {
                    let why = now
                        .motors
                        .iter()
                        .find_map(|motor| match &motor.curve {
                            Curve::Unresolved { why, .. } => Some((motor, *why)),
                            _ => None,
                        })
                        .map_or_else(
                            || "no curve".to_owned(),
                            |(motor, why)| self.why(motor.digest.as_deref(), why),
                        );
                    format!("still no curve: {why}")
                }
                Some((_, why)) => format!("held back for another reason: {}", not_flown(why)),
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
        } else if self.conflicting.contains(digest) {
            "two database curves with different samples share its digest".to_owned()
        } else if self.unbuilt_digests.contains(digest) {
            "the database's curve for its digest does not build in hpr".to_owned()
        } else {
            "its digest is not in the database, and the bundled catalog has no such motor"
                .to_owned()
        }
    }

    pub(crate) fn summary(&self) -> Value {
        if !self.present {
            return Value::Null;
        }
        json!({
            "database_motors": self.database_motors,
            "digests_supplied": self.curves.len(),
            "hybrid_digests": self.hybrids.len(),
            "digests_repeated_with_the_same_curve": self.repeated,
            "digests_with_conflicting_curves": self.conflicting.len(),
            "database_curves_not_built": self.unbuilt,
            "database_total_impulse": self.database_held.summary(),
            "embedded_total_impulse": self.embedded_held.summary(),
            "configurations_without_a_curve_from_the_bundled_catalog_alone": self.fates.values().sum::<usize>(),
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
        let unbuilt = self
            .unbuilt
            .iter()
            .map(|(why, n)| format!("{why} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "  curves from {SOURCE} ({RECORD}): {} motors, {} digests supplied; not supplied: {} \
             hybrid digest(s), {} digest(s) with conflicting curves, {} curve(s) hpr cannot build{}; \
             {} digest(s) repeated with the same curve",
            self.database_motors,
            self.curves.len(),
            self.hybrids.len(),
            self.conflicting.len(),
            self.unbuilt.values().sum::<usize>(),
            if unbuilt.is_empty() {
                String::new()
            } else {
                format!(" ({unbuilt})")
            },
            self.repeated
        );
        println!(
            "    total impulse against OpenRocket's, every database curve built: {}",
            self.database_held.line()
        );
        println!(
            "    total impulse against OpenRocket's, every distinct embedded curve: {}",
            self.embedded_held.line()
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

/// A motor error's kind, without the numbers that make each message different.
fn variant(error: &hpr_motor::MotorError) -> String {
    let debug = format!("{error:?}");
    debug
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_owned()
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record of two database motors sharing a digest with different samples, one repeated
    /// exactly, one hybrid, one whose impulse OpenRocket reports 1% higher, and one that builds.
    #[test]
    fn a_record_is_screened_before_its_curves_are_supplied() {
        let motor = |digest: &str, kind: &str, thrust: f64, impulse: f64| {
            json!({
                "digest": digest, "manufacturer": "Nobody", "designation": "G100",
                "motor_type": kind, "diameter_m": 0.029, "length_m": 0.124,
                "propellant_mass_kg": 0.06, "launch_mass_kg": 0.15,
                "time_s": [0.0, 0.1, 1.0, 1.1], "thrust_n": [0.0, thrust, thrust, 0.0],
                "total_impulse_ns": impulse,
            })
        };
        let record = json!({
            "database": [
                motor("aa", "Reloadable", 100.0, 100.0),
                motor("bb", "Single-use", 100.0, 100.0),
                motor("bb", "Single-use", 100.0, 100.0),
                motor("cc", "Single-use", 100.0, 100.0),
                motor("cc", "Single-use", 90.0, 90.0),
                motor("dd", "Hybrid", 100.0, 100.0),
                motor("ee", "Single-use", 100.0, 101.0),
            ],
            "embedded": [],
        });
        let supply = Supply::of(&record);
        assert_eq!(supply.database_motors, 7);
        assert!(supply.hybrids.contains("dd"));
        assert!(supply.conflicting.contains("cc"));
        assert_eq!(supply.repeated, 1);
        let supplied: Vec<&str> = ["aa", "bb", "cc", "dd", "ee"]
            .into_iter()
            .filter(|d| supply.curves().get(d).is_some())
            .collect();
        assert_eq!(supplied, ["aa", "bb", "ee"]);
        assert_eq!(
            (supply.database_held.compared, supply.database_held.within),
            (6, 5)
        );
        let failure = supply.failure().expect("the 1% curve fails the survey");
        assert!(failure.contains("Nobody G100 (ee)"), "{failure}");
        assert_eq!(
            supply.why(Some("cc"), NoCurve::NotFound),
            "two database curves with different samples share its digest"
        );
        assert_eq!(
            supply.why(Some("dd"), NoCurve::NotFound),
            "its digest is a hybrid's in the database"
        );
        assert_eq!(supply.why(None, NoCurve::Hybrid), "a hybrid");
    }
}
