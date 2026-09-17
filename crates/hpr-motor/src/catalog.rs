//! An offline motor catalog: motor metadata, thrust curves, and each curve's provenance and
//! license.
//!
//! [`Catalog::bundled`] is the catalog compiled into the crate: public-domain curves from
//! ThrustCurve.org whose total impulse, NFPA 1125 burn time and average thrust each match
//! ThrustCurve's stored values within 1% (ADR-005; the selection and the curves left out are in
//! `docs/research/thrustcurve-data.md`). `validation/oracles/thrustcurve/bundle.py` regenerates it.
//! Other curves are fetched and cached later, by `hpr-net` (M5).
//!
//! The index keeps ThrustCurve's own units and values verbatim (mm, g, N·s, N, s), in fields named
//! for them. [`CatalogMotor::motor`] converts to SI and takes the envelope (diameter, length and
//! masses) from the metadata rather than the curve file's header, which can be wrong (Loft lesson
//! L43).

use serde::{Deserialize, Serialize};

use crate::bundled;
use crate::class::ImpulseClass;
use crate::curve::ThrustCurve;
use crate::delay::DelayList;
use crate::error::MotorError;
use crate::motor::SolidMotor;
use crate::{eng, rse};

/// A motor catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// Where the metadata comes from, such as `ThrustCurve.org API v1`.
    pub source: String,
    /// The API capture the metadata was copied from.
    pub snapshot: Snapshot,
    /// The rule that chose the curves.
    pub selection: String,
    /// The motors.
    pub motors: Vec<CatalogMotor>,
}

/// A dated capture of a catalog API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The URL captured.
    pub motors_url: String,
    /// The capture date, `YYYY-MM-DD`.
    pub captured: String,
    /// SHA-256 of the captured bytes, hex.
    pub sha256: String,
}

/// Whether a motor is used once or reloaded into a reusable case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MotorType {
    /// A single-use motor (ThrustCurve `SU`).
    #[serde(rename = "SU")]
    SingleUse,
    /// A reload for a reusable case (ThrustCurve `reload`).
    #[serde(rename = "reload")]
    Reload,
    /// A hybrid motor (ThrustCurve `hybrid`). ThrustCurve lists them; hpr doesn't model them (COTS
    /// solids only), and the bundled catalog has none.
    #[serde(rename = "hybrid")]
    Hybrid,
}

/// One motor's catalog entry, in ThrustCurve's units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogMotor {
    /// ThrustCurve's motor id.
    pub motor_id: String,
    /// Manufacturer name.
    pub manufacturer: String,
    /// Manufacturer abbreviation.
    pub manufacturer_abbrev: String,
    /// Full designation, such as `1266J760-19A`.
    pub designation: String,
    /// Common name, such as `J760`.
    pub common_name: String,
    /// Impulse class letter.
    pub impulse_class: ImpulseClass,
    /// Single-use or reload.
    pub motor_type: MotorType,
    /// ThrustCurve's availability, such as `regular` or `OOP` (out of production).
    pub availability: String,
    /// The certifying organization.
    pub cert_org: Option<String>,
    /// The case a reload fits, such as `Pro54-3G`.
    pub case_info: Option<String>,
    /// The propellant.
    pub prop_info: Option<String>,
    /// Available delays as ThrustCurve writes them (`6,10,14`, `P`); see
    /// [`CatalogMotor::delays`].
    pub delays: Option<String>,
    /// Whether the delay can be shortened (drilled or adjusted).
    pub delay_adjustable: Option<bool>,
    /// Diameter, mm.
    pub diameter_mm: f64,
    /// Length, mm.
    pub length_mm: f64,
    /// Total impulse, N·s.
    pub total_impulse_ns: f64,
    /// Average thrust, N.
    pub average_thrust_n: f64,
    /// Peak thrust, N.
    pub max_thrust_n: Option<f64>,
    /// Burn time, s.
    pub burn_time_s: f64,
    /// Propellant mass, g.
    pub propellant_mass_g: Option<f64>,
    /// Loaded mass, g.
    pub total_mass_g: Option<f64>,
    /// When ThrustCurve last updated the entry, `YYYY-MM-DD`.
    pub updated_on: Option<String>,
    /// The motor's curves.
    pub curves: Vec<CatalogCurve>,
}

/// Where a thrust curve came from and under what terms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogCurve {
    /// ThrustCurve's simfile id.
    pub simfile_id: String,
    /// The file format.
    pub format: CurveFormat,
    /// Who produced the data.
    pub source: CurveSource,
    /// The data license.
    #[serde(default)]
    pub license: CurveLicense,
    /// The file's path relative to the catalog index.
    pub file: String,
    /// SHA-256 of the file's bytes, hex.
    pub sha256: String,
    /// Where the file was downloaded from.
    pub url: String,
    /// ThrustCurve's page for the file.
    pub info_url: Option<String>,
    /// The download date, `YYYY-MM-DD`.
    pub retrieved: String,
}

/// A thrust-curve file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CurveFormat {
    /// RASP `.eng`.
    #[serde(rename = "RASP")]
    Rasp,
    /// RockSim `.rse`.
    #[serde(rename = "RockSim")]
    RockSim,
}

/// Who produced a curve (ThrustCurve's `source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CurveSource {
    /// A certification body's test data.
    #[serde(rename = "cert")]
    Certification,
    /// The manufacturer.
    #[serde(rename = "mfr")]
    Manufacturer,
    /// A ThrustCurve user.
    #[serde(rename = "user")]
    User,
}

/// A curve's data license, as ThrustCurve records it. ThrustCurve's API leaves the key out when no
/// license is recorded, which reads as [`CurveLicense::Unknown`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CurveLicense {
    /// Public domain.
    #[serde(rename = "PD")]
    PublicDomain,
    /// "Free": by ThrustCurve's definition this includes GPL and Creative Commons terms.
    #[serde(rename = "free")]
    Free,
    /// Other, restricted terms.
    #[serde(rename = "other")]
    Other,
    /// No license recorded.
    #[default]
    #[serde(rename = "unknown", alias = "")]
    Unknown,
}

impl CurveLicense {
    /// Whether a curve under this license may be bundled in this `MIT OR Apache-2.0` crate: only
    /// public domain. "Free" can mean GPL, and the others are restricted or unknown.
    pub fn is_bundleable(self) -> bool {
        self == Self::PublicDomain
    }
}

impl Catalog {
    /// The catalog compiled into this crate.
    ///
    /// # Errors
    ///
    /// [`MotorError::Catalog`] if the bundled index doesn't deserialize, which a test rules out.
    pub fn bundled() -> Result<Self, MotorError> {
        Self::from_json(bundled::CATALOG_JSON)
    }

    /// Reads a catalog index.
    ///
    /// # Errors
    ///
    /// [`MotorError::Catalog`] if the text isn't a catalog index.
    pub fn from_json(text: &str) -> Result<Self, MotorError> {
        serde_json::from_str(text).map_err(|error| MotorError::Catalog(error.to_string()))
    }

    /// The motors whose designation or common name matches `name`, ignoring ASCII case, spaces
    /// and hyphens (`j760`, `1266J760-19A`, `K 940`).
    pub fn find<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a CatalogMotor> + 'a {
        let key = normalize(name);
        self.motors.iter().filter(move |motor| {
            normalize(&motor.designation) == key || normalize(&motor.common_name) == key
        })
    }
}

/// Lowercase, without spaces or hyphens.
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// A curve file's thrust curve and its header's propellant and loaded masses, kg.
fn read_curve_file(
    curve: &CatalogCurve,
    text: &str,
) -> Result<(ThrustCurve, f64, f64), MotorError> {
    let one = |count: usize| {
        if count == 1 {
            Ok(())
        } else {
            Err(MotorError::Inconsistent(format!(
                "the curve file {} holds {count} motors, not one",
                curve.file
            )))
        }
    };
    match curve.format {
        CurveFormat::Rasp => {
            let file = eng::parse(text)?.value;
            one(file.entries.len())?;
            let entry = &file.entries[0];
            Ok((
                entry.thrust_curve()?,
                entry.propellant_mass_kg,
                entry.total_mass_kg,
            ))
        }
        CurveFormat::RockSim => {
            let file = rse::parse(text)?.value;
            one(file.engines.len())?;
            let engine = &file.engines[0];
            Ok((
                engine.thrust_curve()?,
                engine.propellant_mass_g * 1e-3,
                engine.initial_mass_g * 1e-3,
            ))
        }
    }
}

/// The text of a bundled curve file, by its path in the bundled index.
pub fn bundled_curve_text(file: &str) -> Option<&'static str> {
    bundled::CURVE_FILES
        .iter()
        .find(|(path, _)| *path == file)
        .map(|(_, text)| *text)
}

impl CatalogMotor {
    /// The delay settings read from [`CatalogMotor::delays`]; empty when none are recorded.
    pub fn delays(&self) -> DelayList {
        DelayList::parse(self.delays.as_deref().unwrap_or(""))
    }

    /// Reads a curve of this motor from its file's text.
    ///
    /// # Errors
    ///
    /// - The format's parse error ([`eng::parse`], [`rse::parse`]).
    /// - [`MotorError::Inconsistent`] if the file doesn't hold exactly one motor.
    /// - [`ThrustCurve::new`]'s errors.
    pub fn thrust_curve(
        &self,
        curve: &CatalogCurve,
        text: &str,
    ) -> Result<ThrustCurve, MotorError> {
        read_curve_file(curve, text).map(|(thrust, _, _)| thrust)
    }

    /// The motor with the curve read from `text`, built with [`SolidMotor::from_envelope`] from
    /// this entry's diameter, length and masses. The curve file's header is used only for a mass
    /// the metadata lacks.
    ///
    /// # Errors
    ///
    /// As [`CatalogMotor::thrust_curve`] and [`SolidMotor::from_envelope`], and
    /// [`MotorError::Inconsistent`] for a hybrid (hpr models solids only) or when neither the
    /// metadata nor the file gives the masses.
    pub fn motor(&self, curve: &CatalogCurve, text: &str) -> Result<SolidMotor, MotorError> {
        if self.motor_type == MotorType::Hybrid {
            return Err(MotorError::Inconsistent(format!(
                "{} is a hybrid; hpr models solid motors only",
                self.designation
            )));
        }
        let (thrust, header_propellant_kg, header_total_kg) = read_curve_file(curve, text)?;
        let propellant_kg = self
            .propellant_mass_g
            .map_or(header_propellant_kg, |g| g * 1e-3);
        let total_kg = self.total_mass_g.map_or(header_total_kg, |g| g * 1e-3);
        if !(propellant_kg > 0.0 && total_kg > 0.0) {
            return Err(MotorError::Inconsistent(format!(
                "{} has no propellant and loaded masses",
                self.designation
            )));
        }
        SolidMotor::from_envelope(
            thrust,
            self.diameter_mm * 1e-3,
            self.length_mm * 1e-3,
            propellant_kg,
            total_kg,
        )
    }

    /// The motor with its first bundled curve.
    ///
    /// # Errors
    ///
    /// [`MotorError::Inconsistent`] if the entry has no bundled curve file, and
    /// [`CatalogMotor::motor`]'s errors.
    pub fn bundled_motor(&self) -> Result<SolidMotor, MotorError> {
        let (curve, text) = self
            .curves
            .iter()
            .find_map(|curve| bundled_curve_text(&curve.file).map(|text| (curve, text)))
            .ok_or_else(|| {
                MotorError::Inconsistent(format!("{} has no bundled curve", self.designation))
            })?;
        self.motor(curve, text)
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::delay::Delay;

    fn bundled() -> Catalog {
        Catalog::bundled().unwrap()
    }

    /// Loft lesson L41: Loft's bundled curves were 45 public domain, 38 with no license, 22
    /// unknown and 3 "free" (which can include GPL).
    #[test]
    fn bundled_curves_have_permissive_license() {
        let catalog = bundled();
        assert!(!catalog.motors.is_empty());
        let mut files = 0;
        for motor in &catalog.motors {
            for curve in &motor.curves {
                assert!(
                    curve.license.is_bundleable(),
                    "{} {}: {:?}",
                    motor.designation,
                    curve.simfile_id,
                    curve.license
                );
                assert!(bundled_curve_text(&curve.file).is_some(), "{}", curve.file);
                files += 1;
            }
        }
        // Every compiled-in file is indexed, so nothing unlicensed rides along.
        assert_eq!(files, bundled::CURVE_FILES.len());
        // Each file is byte for byte what was downloaded from its recorded URL.
        for motor in &catalog.motors {
            for curve in &motor.curves {
                let text = bundled_curve_text(&curve.file).unwrap();
                let digest: String = Sha256::digest(text.as_bytes())
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                assert_eq!(digest, curve.sha256, "{}", curve.file);
                assert!(curve.url.contains(&curve.simfile_id));
            }
        }
        assert!(!CurveLicense::Free.is_bundleable());
        assert!(!CurveLicense::Unknown.is_bundleable());
    }

    /// Loft lesson L42: Loft's impulse checks allowed −8% to +8%, and a mis-sourced curve flew
    /// about 26% high until caught. M1.3's done-when: every bundled curve's total impulse, average
    /// thrust and burn time match ThrustCurve's metadata within 1%.
    #[test]
    fn every_bundled_curve_impulse_within_1pct_of_thrustcurve() {
        let catalog = bundled();
        let mut checked = 0;
        for motor in &catalog.motors {
            for curve in &motor.curves {
                let text = bundled_curve_text(&curve.file).unwrap();
                let thrust = motor.thrust_curve(curve, text).unwrap();
                let within = |computed: f64, stored: f64, what: &str| {
                    let error = (computed - stored) / stored;
                    assert!(
                        error.abs() <= 0.01,
                        "{} ({}): {what} {computed} against ThrustCurve's {stored} ({:+.2}%)",
                        motor.designation,
                        curve.simfile_id,
                        100.0 * error
                    );
                };
                within(
                    thrust.total_impulse_ns(),
                    motor.total_impulse_ns,
                    "total impulse",
                );
                within(
                    thrust.average_thrust_n(),
                    motor.average_thrust_n,
                    "average thrust",
                );
                within(thrust.burn_time_s(), motor.burn_time_s, "burn time");
                assert_eq!(
                    ImpulseClass::from_total_impulse(motor.total_impulse_ns).unwrap(),
                    motor.impulse_class,
                    "{}",
                    motor.designation
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 32);
    }

    /// Loft lesson L43: ThrustCurve's metadata overrides the curve file's envelope; one header
    /// said 75 mm for a 54 mm motor.
    #[test]
    fn metadata_overrides_header_envelope() {
        let catalog = bundled();
        // The bundled N3300R file's header gives a 1060 mm length; ThrustCurve says 1046 mm.
        let motor = catalog.find("N3300R").next().unwrap();
        let curve = &motor.curves[0];
        let text = bundled_curve_text(&curve.file).unwrap();
        let header = &eng::parse(text).unwrap().value.entries[0];
        assert_eq!(header.length_mm, 1060.0);
        assert_eq!(motor.length_mm, 1046.0);
        let built = motor.bundled_motor().unwrap();
        // The envelope default centres the dry mass on the metadata's length.
        assert!((built.dry().cg_m - 0.523).abs() < 1e-12);
        let loaded = built.state(0.0).total;
        assert!((loaded.mass_kg - motor.total_mass_g.unwrap() * 1e-3).abs() < 1e-9);
        // With no metadata masses, the header's are used.
        let bare = CatalogMotor {
            propellant_mass_g: None,
            total_mass_g: None,
            ..motor.clone()
        };
        let fallback = bare.motor(curve, text).unwrap();
        assert!((fallback.state(0.0).total.mass_kg - header.total_mass_kg).abs() < 1e-12);
        assert!((fallback.dry().cg_m - 0.523).abs() < 1e-12);
    }

    /// M1.3's done-when: parse-write-parse round trips are identical, on every bundled file.
    #[test]
    fn every_bundled_file_round_trips_bit_for_bit() {
        let catalog = bundled();
        for motor in &catalog.motors {
            for curve in &motor.curves {
                let text = bundled_curve_text(&curve.file).unwrap();
                match curve.format {
                    CurveFormat::Rasp => {
                        let first = eng::parse(text).unwrap().value;
                        let written = eng::write(&first).unwrap();
                        let second = eng::parse(&written).unwrap().value;
                        assert_eq!(second, first, "{}", curve.file);
                        assert_eq!(eng::tests::bits(&second), eng::tests::bits(&first));
                        assert_eq!(eng::write(&second).unwrap(), written);
                    }
                    CurveFormat::RockSim => {
                        let first = rse::parse(text).unwrap().value;
                        let written = rse::write(&first).unwrap();
                        let second = rse::parse(&written).unwrap().value;
                        assert_eq!(second, first, "{}", curve.file);
                        assert_eq!(rse::tests::bits(&second), rse::tests::bits(&first));
                        assert_eq!(rse::write(&second).unwrap(), written);
                    }
                }
            }
        }
    }

    #[test]
    fn finds_motors_and_reads_delays() {
        let catalog = bundled();
        let j760: Vec<_> = catalog.find("j760").collect();
        assert_eq!(j760.len(), 1);
        assert_eq!(j760[0].designation, "1266J760-19A");
        assert_eq!(catalog.find("1266J760-19A").count(), 1);
        let plugged = catalog.find("L3200").next().unwrap();
        assert_eq!(plugged.delays().delays, [Delay::Plugged]);
        assert!(catalog.find("no such motor").next().is_none());
        // Every bundled motor builds.
        for motor in &catalog.motors {
            let built = motor.bundled_motor().unwrap();
            assert!(
                built.propellant_initial_mass_kg() > 0.0,
                "{}",
                motor.designation
            );
        }
    }

    #[test]
    fn catalog_json_reads_thrustcurve_license_values_and_rejects_others() {
        assert!(matches!(
            Catalog::from_json("{}"),
            Err(MotorError::Catalog(_))
        ));
        let with = |license: &str| {
            Catalog::from_json(&bundled::CATALOG_JSON.replacen("\"license\": \"PD\",", license, 1))
        };
        let first_license = |catalog: Catalog| catalog.motors[0].curves[0].license;
        assert!(with("\"license\": \"GPL\",").is_err());
        assert_eq!(
            first_license(with("\"license\": \"free\",").unwrap()),
            CurveLicense::Free
        );
        // ThrustCurve's API leaves the key out, or blank, when no license is recorded.
        assert_eq!(first_license(with("").unwrap()), CurveLicense::Unknown);
        assert_eq!(
            first_license(with("\"license\": \"\",").unwrap()),
            CurveLicense::Unknown
        );
        let hybrid = bundled::CATALOG_JSON.replacen(
            "\"motor_type\": \"SU\"",
            "\"motor_type\": \"hybrid\"",
            1,
        );
        let hybrid = Catalog::from_json(&hybrid).unwrap();
        assert_eq!(hybrid.motors[0].motor_type, MotorType::Hybrid);
        assert!(hybrid.motors[0].bundled_motor().is_err());
    }

    #[derive(Debug, serde::Deserialize)]
    struct AnalyzeOracle {
        oracle: String,
        curves: Vec<AnalyzeCurve>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct AnalyzeCurve {
        file: String,
        total_impulse_ns: f64,
        burn_time_s: f64,
        average_thrust_n: f64,
        max_thrust_n: f64,
        burn_start_s: f64,
        burn_end_s: f64,
    }

    /// hpr's statistics against ThrustCurve.org's own code (`simulate/analyze/analyze.js`, run
    /// unchanged by `validation/oracles/thrustcurve/analyze_stats.js`) on every bundled curve.
    /// Unlike the 1% check against the stored values, which chose the bundle, this can fail: it
    /// checks that hpr computes what ThrustCurve computes, to rounding.
    #[test]
    fn every_bundled_curve_matches_thrustcurve_statistics_code() {
        let oracle: AnalyzeOracle = serde_json::from_str(include_str!(
            "../../../validation/fixtures/motor/thrustcurve-analyze-stats.json"
        ))
        .unwrap();
        assert!(oracle.oracle.contains("analyze.js at commit 577afa6"));
        let catalog = bundled();
        assert_eq!(oracle.curves.len(), bundled::CURVE_FILES.len());
        let mut worst = 0.0f64;
        for expected in &oracle.curves {
            let (motor, curve) = catalog
                .motors
                .iter()
                .find_map(|m| {
                    m.curves
                        .iter()
                        .find(|c| c.file == expected.file)
                        .map(|c| (m, c))
                })
                .unwrap();
            let thrust = motor
                .thrust_curve(curve, bundled_curve_text(&curve.file).unwrap())
                .unwrap();
            let (start, end) = thrust.burn_window_s();
            for (ours, theirs, what) in [
                (
                    thrust.total_impulse_ns(),
                    expected.total_impulse_ns,
                    "total impulse",
                ),
                (thrust.burn_time_s(), expected.burn_time_s, "burn time"),
                (
                    thrust.average_thrust_n(),
                    expected.average_thrust_n,
                    "average thrust",
                ),
                (thrust.peak_thrust_n(), expected.max_thrust_n, "peak thrust"),
                (start, expected.burn_start_s, "burn start"),
                (end, expected.burn_end_s, "burn end"),
            ] {
                let error = (ours - theirs).abs() / theirs.abs().max(1e-3);
                worst = worst.max(error);
                assert!(
                    error <= 1e-12,
                    "{}: {what} {ours} against {theirs}",
                    expected.file
                );
            }
        }
        eprintln!("largest relative difference from ThrustCurve's code: {worst:.2e}");
    }
}
