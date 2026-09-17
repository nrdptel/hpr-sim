//! Running cases: fly what the case says, measure it, and compare against the reference.

use std::path::Path;

use hpr_atmos::{LayeredWind, WindInterpolation, WindLevel};
use hpr_core::DVec3;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    Device, DeviceDrag, Environment, EventKind, FlightSettings, Rail, Simulation, State,
    Termination, Trigger,
};
use sha2::{Digest, Sha256};

use crate::case::{Case, CaseLock, Flight, cases_dir, committed_cases};
use crate::metrics::{Measured, Reference};
use crate::report::{Comparison, Report, Source};

/// What can go wrong running a case.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidateError {
    /// A file could not be read or written.
    #[error("{what} {path}: {source}")]
    Io {
        /// What was being done.
        what: &'static str,
        /// The path.
        path: String,
        /// The cause.
        source: std::io::Error,
    },
    /// A case or lock file could not be parsed.
    #[error("{path} is not a valid case file: {source}")]
    Toml {
        /// The path.
        path: String,
        /// The cause.
        source: Box<toml::de::Error>,
    },
    /// A reference or design file could not be parsed.
    #[error("{path} is not valid JSON: {source}")]
    Json {
        /// The path.
        path: String,
        /// The cause.
        source: Box<serde_json::Error>,
    },
    /// The case, the lock and the references disagree.
    #[error("{0}")]
    Case(String),
    /// The flight itself failed, or was refused before it started.
    #[error("case {case}: {what}")]
    Flight {
        /// The case's id.
        case: String,
        /// What went wrong, in words.
        what: String,
        /// The cause, where the simulator gave one.
        source: Option<Box<hpr_sim::SimError>>,
    },
}

/// How close hpr's mass has to be to the mass the oracle recorded flying, as a fraction.
///
/// They are two builds of one rocket, not two measurements of it: anything above rounding means
/// the case is comparing different vehicles, which must not read as a difference in the physics
/// (Loft lesson L75).
const MASS_AGREEMENT: f64 = 1e-9;

/// Runs every case the lock names, in its order, and reports the comparisons.
///
/// # Errors
///
/// [`ValidateError`] if a file is missing or malformed, if the lock names a case that is not
/// there or a committed case is not locked (Loft lesson L78), if a reference has no provenance
/// (L77), if a metric is neither gated nor declared (L79), or if a flight fails.
pub fn run_lock(root: &Path, fast: bool) -> Result<Report, ValidateError> {
    let lock_path = cases_dir(root).join("lock.toml");
    let lock: CaseLock =
        toml::from_str(&read(&lock_path, "reading the case lock")?).map_err(|source| {
            ValidateError::Toml {
                path: lock_path.display().to_string(),
                source: Box::new(source),
            }
        })?;
    let unknown = lock.unknown_slow();
    if !unknown.is_empty() {
        return Err(ValidateError::Case(format!(
            "the case lock names {unknown:?} as slow, which are not cases"
        )));
    }
    // Loft lesson L78's other half: a case nobody locked would never run, and a suite that does
    // not notice is one edit away from reporting green over half of itself.
    let committed = committed_cases(root).map_err(ValidateError::Case)?;
    let unlocked: Vec<&String> = committed
        .iter()
        .filter(|id| !lock.cases.contains(id))
        .collect();
    if !unlocked.is_empty() {
        return Err(ValidateError::Case(format!(
            "{unlocked:?} are committed under validation/cases/ but the lock does not name them, \
             so they would never run"
        )));
    }

    let wanted = lock.wanted(fast);
    let mut cases = Vec::new();
    let mut comparisons = Vec::new();
    let mut sources = Vec::new();
    for id in &wanted {
        let path = cases_dir(root).join(format!("{id}.toml"));
        if !path.is_file() {
            // Loft lesson L78: a case that isn't there is a failure, not a quiet skip.
            return Err(ValidateError::Case(format!(
                "the case lock names {id}, but {} is not there",
                path.display()
            )));
        }
        let case: Case = toml::from_str(&read(&path, "reading a case")?).map_err(|source| {
            ValidateError::Toml {
                path: path.display().to_string(),
                source: Box::new(source),
            }
        })?;
        if case.id != *id {
            return Err(ValidateError::Case(format!(
                "{} calls itself {}, not {id}",
                path.display(),
                case.id
            )));
        }
        let (case_comparisons, source) = run_case(root, &case)?;
        // The report records the cases that were compared, not the ones a lock hoped for.
        cases.push(case.id.clone());
        comparisons.extend(case_comparisons);
        sources.push(source);
    }
    Ok(Report {
        harness_version: env!("CARGO_PKG_VERSION").to_owned(),
        fast,
        cases,
        skipped: lock.skipped(fast),
        comparisons,
        sources,
    })
}

/// Runs one case: flies it, measures it, and compares every metric it names.
///
/// # Errors
///
/// As [`run_lock`].
pub fn run_case(root: &Path, case: &Case) -> Result<(Vec<Comparison>, Source), ValidateError> {
    // A name the flight cannot measure is a typo, and it costs nothing to say so before flying.
    let known: &[&str] = match &case.flight {
        Flight::RecoveryDescent { .. } => &DESCENT_METRICS,
    };
    let unknown: Vec<&String> = case
        .metrics
        .keys()
        .filter(|name| !known.contains(&name.as_str()))
        .collect();
    if !unknown.is_empty() {
        return Err(ValidateError::Case(format!(
            "case {}: {unknown:?} are not metrics this flight measures; it measures {known:?}",
            case.id
        )));
    }
    for (name, metric) in &case.metrics {
        metric.check(&case.id, name).map_err(ValidateError::Case)?;
    }

    let (reference, setup) = load_reference(root, case)?;
    let missing = reference.without_provenance();
    if !missing.is_empty() {
        // Loft lesson L77: a value with no source is not a reference.
        return Err(ValidateError::Case(format!(
            "case {}: the reference's {missing:?} carry no source",
            case.id
        )));
    }
    // Loft lesson L79 from the reference's side: a number the oracle published that the case says
    // nothing about is a metric nobody is watching. Gate it, or write down why it is not scored.
    let ignored: Vec<&String> = reference
        .values
        .keys()
        .filter(|name| !case.metrics.contains_key(*name))
        .collect();
    if !ignored.is_empty() {
        return Err(ValidateError::Case(format!(
            "case {}: the reference publishes {ignored:?}, which the case neither gates nor \
             declares not scored",
            case.id
        )));
    }

    let measured = measure(root, case, &setup)?;
    let mut comparisons = Vec::new();
    for (name, metric) in &case.metrics {
        let value = reference.values.get(name).ok_or_else(|| {
            ValidateError::Case(format!(
                "case {}: the reference has no {name}; it has {:?}",
                case.id,
                reference.values.keys().collect::<Vec<_>>()
            ))
        })?;
        let got = measured.values.get(name).ok_or_else(|| {
            ValidateError::Case(format!(
                "case {}: hpr measured no {name}; it measured {:?}",
                case.id,
                measured.values.keys().collect::<Vec<_>>()
            ))
        })?;
        comparisons.push(match metric.reason() {
            Some(reason) => {
                Comparison::not_scored(&case.id, name, *got, value.value, &value.source, reason)
            }
            None => Comparison::new(
                &case.id,
                name,
                *got,
                value.value,
                &value.source,
                metric.tolerance(),
            ),
        });
    }
    // Loft lesson L79: every metric hpr measured for the case is accounted for, so nothing is
    // reported without a tolerance and nothing silently drifts.
    let ungated: Vec<&String> = measured
        .values
        .keys()
        .filter(|name| !case.metrics.contains_key(*name))
        .collect();
    if !ungated.is_empty() {
        return Err(ValidateError::Case(format!(
            "case {}: {ungated:?} are measured but have no tolerance",
            case.id
        )));
    }
    Ok((
        comparisons,
        Source {
            case: case.id.clone(),
            oracle: reference.oracle,
            generator: reference.generator,
            command: reference.command,
            file: reference.file,
            sha256: reference.sha256,
            model: reference.model,
            overrides: reference.overrides,
        },
    ))
}

/// The reference a case names, in the harness's shape, with the inputs the oracle flew.
fn load_reference(root: &Path, case: &Case) -> Result<(Reference, DescentSetup), ValidateError> {
    let path = root.join(&case.reference);
    let text = read(&path, "reading a reference")?;
    let document: serde_json::Value =
        serde_json::from_str(&text).map_err(|source| ValidateError::Json {
            path: path.display().to_string(),
            source: Box::new(source),
        })?;
    let sha256: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let wanted = case.reference_case.as_deref().unwrap_or(&case.id);
    let file = case.reference.display().to_string().replace('\\', "/");
    crate::rocketpy::descent_case(&document, wanted, &file, &sha256).ok_or_else(|| {
        ValidateError::Case(format!(
            "case {}: {} has no case {wanted} that names the run which produced it",
            case.id,
            path.display()
        ))
    })
}

/// Flies the case and measures what it reports.
fn measure(root: &Path, case: &Case, setup: &DescentSetup) -> Result<Measured, ValidateError> {
    match &case.flight {
        Flight::RecoveryDescent {
            design,
            configuration,
        } => fly_descent(root, case, setup, design, configuration),
    }
}

/// A descent from the state the reference declares, under the devices it declares: the flight
/// `hpr_sim::recovery::tests::descent_matches_rocketpy_examples` makes, through the harness.
fn fly_descent(
    root: &Path,
    case: &Case,
    setup: &DescentSetup,
    design: &str,
    configuration: &str,
) -> Result<Measured, ValidateError> {
    let refuse = |what: String| ValidateError::Flight {
        case: case.id.clone(),
        what,
        source: None,
    };
    // L75: the reference records which rocket the oracle flew. A case that names another one is
    // comparing two vehicles, and that must not read as a difference in the physics.
    let named = format!("validation/designs/{design}.json");
    if named != setup.design {
        return Err(refuse(format!(
            "the case flies {named}, but the reference was flown with {}",
            setup.design
        )));
    }
    // The devices are in the order they open, and the state the reference hands over is the one
    // at the first deployment: device 0 has already opened, so only later devices may carry a
    // trigger of their own.
    match setup.devices.split_first() {
        None => return Err(refuse("the reference declares no devices".to_owned())),
        Some((first, rest)) => {
            if first.height_above_ground_m.is_some() || first.lag_s != 0.0 {
                return Err(refuse(format!(
                    "the reference's first device ({}) has to be the one that opens at the \
                     handover, with no lag left to run",
                    first.name
                )));
            }
            if let Some(device) = rest
                .iter()
                .find(|device| device.height_above_ground_m.is_none())
            {
                return Err(refuse(format!(
                    "device {} opens at apogee, which is behind the state the reference hands \
                     over; there is no apogee left for the harness to trigger on",
                    device.name
                )));
            }
        }
    }

    let path = root.join(&named);
    let rocket: Rocket =
        serde_json::from_str(&read(&path, "reading a design")?).map_err(|source| {
            ValidateError::Json {
                path: path.display().to_string(),
                source: Box::new(source),
            }
        })?;
    let flight = || -> Result<Result<Measured, String>, hpr_sim::SimError> {
        let site =
            Geodetic::from_degrees(setup.latitude_deg, setup.longitude_deg, setup.elevation_m)?;
        let levels: Vec<WindLevel> = setup
            .wind
            .iter()
            .map(|(height_msl_m, east, north)| WindLevel {
                height_msl_m: *height_msl_m,
                speed_m_s: east.hypot(*north),
                direction_from_rad: (-east).atan2(-north).rem_euclid(std::f64::consts::TAU),
            })
            .collect();
        let environment = Environment {
            wind: std::sync::Arc::new(LayeredWind::new(levels, WindInterpolation::Components)?),
            ..Environment::standard(site)?
        };
        let devices = setup
            .devices
            .iter()
            .enumerate()
            .map(|(index, device)| {
                let drag = DeviceDrag::DragArea {
                    cd_s_m2: device.cd_s_m2,
                };
                let trigger = match device.height_above_ground_m {
                    Some(height_above_ground_m) => Trigger::Altitude {
                        height_above_ground_m,
                    },
                    None => Trigger::Time {
                        time_s: setup.start_time_s,
                    },
                };
                let mut built = Device::new(&device.name, drag, trigger).with_lag_s(device.lag_s);
                if index + 1 < setup.devices.len() {
                    built = built.with_release_by(index + 1);
                }
                built
            })
            .collect();
        let simulation = Simulation::new(
            &rocket,
            configuration,
            environment,
            Rail::vertical(6.0),
            FlightSettings {
                max_time_s: 6000.0,
                ..FlightSettings::default()
            },
        )?
        .with_recovery(devices)?;
        let properties = simulation.assembly().mass_properties(setup.start_time_s);
        // L75 again: the same rocket has to weigh the same in both codes before any difference in
        // where it lands can be read as physics.
        let mass_gap = (properties.mass_kg - setup.dry_mass_kg).abs() / setup.dry_mass_kg;
        if !mass_gap.is_finite() || mass_gap > MASS_AGREEMENT {
            return Ok(Err(format!(
                "hpr flies {:.6} kg where the reference recorded {:.6} kg, a relative {mass_gap:.3e}",
                properties.mass_kg, setup.dry_mass_kg
            )));
        }
        let attitude = Rail::vertical(6.0).attitude();
        let cg_enu_m = DVec3::new(
            setup.start_position_m.0,
            setup.start_position_m.1,
            setup.start_position_m.2 - setup.elevation_m,
        );
        let state = State {
            position_enu_m: cg_enu_m - attitude.mul_vec3(properties.cg_m),
            velocity_enu_m_s: DVec3::new(
                setup.start_velocity_m_s.0,
                setup.start_velocity_m_s.1,
                setup.start_velocity_m_s.2,
            ),
            attitude,
            body_rate_rad_s: DVec3::ZERO,
        };
        let result = simulation.run_free(setup.start_time_s, state, &mut ())?;
        if result.termination != Termination::GroundHit {
            return Ok(Err(format!(
                "the descent ended as {:?} after {} steps, not at the ground",
                result.termination, result.stats.accepted_steps
            )));
        }
        let landing = result
            .event(EventKind::GroundHit)
            .map_or(result.final_sample, |event| event.sample);
        let drift = landing.cg_enu_m - cg_enu_m;
        let descent_time_s = landing.time_s - setup.start_time_s;
        let mut measured = Measured::default();
        measured.insert("descent_time_s", descent_time_s);
        measured.insert("impact_speed_m_s", -landing.vertical_speed_m_s);
        measured.insert("drift_m", drift.truncate().length());
        measured.insert("drift_east_m", drift.x);
        measured.insert("drift_north_m", drift.y);
        // As the generator defines it: the height given up, over the time it took.
        measured.insert(
            "mean_descent_rate_m_s",
            (cg_enu_m.z - landing.cg_enu_m.z) / descent_time_s,
        );
        Ok(Ok(measured))
    };
    match flight() {
        Ok(Ok(measured)) => Ok(measured),
        Ok(Err(what)) => Err(refuse(what)),
        Err(source) => Err(ValidateError::Flight {
            case: case.id.clone(),
            what: source.to_string(),
            source: Some(Box::new(source)),
        }),
    }
}

/// Reads a file, naming what was being done.
fn read(path: &Path, what: &'static str) -> Result<String, ValidateError> {
    std::fs::read_to_string(path).map_err(|source| ValidateError::Io {
        what,
        path: path.display().to_string(),
        source,
    })
}

/// The metric names a descent case can report.
pub const DESCENT_METRICS: [&str; 6] = [
    "descent_time_s",
    "impact_speed_m_s",
    "drift_m",
    "drift_east_m",
    "drift_north_m",
    "mean_descent_rate_m_s",
];

/// The parts of a reference a descent case needs, read from the generator's own JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct DescentSetup {
    /// The design the oracle flew, from the repository root.
    pub design: String,
    /// What it weighed as it descended, kg.
    pub dry_mass_kg: f64,
    /// The site's latitude, degrees.
    pub latitude_deg: f64,
    /// Its longitude, degrees.
    pub longitude_deg: f64,
    /// Its elevation above sea level, m.
    pub elevation_m: f64,
    /// The wind as `(height above sea level, east, north)` levels, m and m/s.
    pub wind: Vec<(f64, f64, f64)>,
    /// When the descent starts, s after ignition.
    pub start_time_s: f64,
    /// Where its centre of mass starts, m (east, north, height above sea level).
    pub start_position_m: (f64, f64, f64),
    /// That point's velocity, m/s.
    pub start_velocity_m_s: (f64, f64, f64),
    /// The devices, in the order they open.
    pub devices: Vec<DescentDevice>,
}

/// One device as the reference declares it.
#[derive(Debug, Clone, PartialEq)]
pub struct DescentDevice {
    /// Its name, for reports.
    pub name: String,
    /// Its drag area `C_D S`, m2.
    pub cd_s_m2: f64,
    /// Its lag from the trigger to line stretch, s.
    pub lag_s: f64,
    /// The height above the site it opens at, m; `None` opens it at the start.
    pub height_above_ground_m: Option<f64>,
}
