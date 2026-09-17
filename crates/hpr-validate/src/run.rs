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

use crate::case::{Case, CaseLock, Flight, cases_dir};
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
    /// The flight itself failed.
    #[error("case {case} could not be flown")]
    Flight {
        /// The case's id.
        case: String,
        /// The cause.
        source: Box<hpr_sim::SimError>,
    },
}

/// Runs every case the lock names, in its order, and reports the comparisons.
///
/// # Errors
///
/// [`ValidateError`] if a file is missing or malformed, if the lock names a case that is not
/// there (Loft lesson L78), if a reference value has no provenance (L77), if a metric has no
/// tolerance (L79), or if a flight fails.
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
    let wanted = lock.wanted(fast);
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
        comparisons.extend(case_comparisons);
        sources.push(source);
    }
    Ok(Report {
        hpr_version: env!("CARGO_PKG_VERSION").to_owned(),
        fast,
        cases: wanted,
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
    let (reference, setup) = load_reference(root, case)?;
    let missing = reference.without_provenance();
    if !missing.is_empty() {
        // Loft lesson L77: a value with no source is not a reference.
        return Err(ValidateError::Case(format!(
            "case {}: the reference's {missing:?} carry no source",
            case.id
        )));
    }
    let measured = measure(root, case, &setup)?;
    let mut comparisons = Vec::new();
    for (name, metric) in &case.metrics {
        if !metric.tolerance.is_set() {
            // Loft lesson L79 again: a tolerance that bounds nothing gates nothing.
            return Err(ValidateError::Case(format!(
                "case {}: {name} has a tolerance that bounds nothing",
                case.id
            )));
        }
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
        comparisons.push(Comparison::new(
            &case.id,
            name,
            *got,
            value.value,
            &value.source,
            metric.tolerance,
        ));
    }
    // Loft lesson L79: every metric hpr measured for the case is gated, so nothing is reported
    // without a tolerance and nothing silently drifts.
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
    let wanted = case.reference_case.as_deref().unwrap_or(&case.id);
    crate::rocketpy::descent_case(&document, wanted).ok_or_else(|| {
        ValidateError::Case(format!(
            "case {}: {} has no case {wanted} in the shape the harness reads",
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
    let path = root.join(format!("validation/designs/{design}.json"));
    let rocket: Rocket =
        serde_json::from_str(&read(&path, "reading a design")?).map_err(|source| {
            ValidateError::Json {
                path: path.display().to_string(),
                source: Box::new(source),
            }
        })?;
    let flight = || -> Result<Measured, hpr_sim::SimError> {
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
        let cg_m = simulation
            .assembly()
            .mass_properties(setup.start_time_s)
            .cg_m;
        let attitude = Rail::vertical(6.0).attitude();
        let cg_enu_m = DVec3::new(
            setup.start_position_m.0,
            setup.start_position_m.1,
            setup.start_position_m.2 - setup.elevation_m,
        );
        let state = State {
            position_enu_m: cg_enu_m - attitude.mul_vec3(cg_m),
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
            return Err(hpr_sim::SimError::Domain {
                what: "termination of a validation descent (it has to reach the ground)",
                value: result.stats.accepted_steps as f64,
            });
        }
        let landing = result
            .event(EventKind::GroundHit)
            .map_or(result.final_sample, |event| event.sample);
        let drift = landing.cg_enu_m - cg_enu_m;
        let mut measured = Measured::default();
        measured.insert("descent_time_s", landing.time_s - setup.start_time_s);
        measured.insert("impact_speed_m_s", -landing.vertical_speed_m_s);
        measured.insert("drift_m", drift.truncate().length());
        measured.insert("drift_east_m", drift.x);
        measured.insert("drift_north_m", drift.y);
        Ok(measured)
    };
    flight().map_err(|source| ValidateError::Flight {
        case: case.id.clone(),
        source: Box::new(source),
    })
}

/// Reads a file, naming what was being done.
fn read(path: &Path, what: &'static str) -> Result<String, ValidateError> {
    std::fs::read_to_string(path).map_err(|source| ValidateError::Io {
        what,
        path: path.display().to_string(),
        source,
    })
}

/// Writes `text` to `path`, creating the directory.
///
/// # Errors
///
/// [`ValidateError::Io`].
pub fn write(path: &Path, text: &str) -> Result<(), ValidateError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| ValidateError::Io {
            what: "creating a report directory",
            path: parent.display().to_string(),
            source,
        })?;
    }
    std::fs::write(path, text).map_err(|source| ValidateError::Io {
        what: "writing a report",
        path: path.display().to_string(),
        source,
    })
}

/// The metric names a descent case can report.
pub const DESCENT_METRICS: [&str; 5] = [
    "descent_time_s",
    "impact_speed_m_s",
    "drift_m",
    "drift_east_m",
    "drift_north_m",
];

/// The parts of a reference a descent case needs, read from the generator's own JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct DescentSetup {
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
    /// Its drag area `C_D S`, m².
    pub cd_s_m2: f64,
    /// Its lag from the trigger to line stretch, s.
    pub lag_s: f64,
    /// The height above the site it opens at, m; `None` opens it at the start.
    pub height_above_ground_m: Option<f64>,
}
