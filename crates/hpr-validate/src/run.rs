//! Running cases: fly what the case says, measure it, and compare against the reference.

use std::path::Path;

use hpr_aero::{AeroError, DragTable};
use hpr_atmos::AtmosphereModel;
use hpr_atmos::{LayeredWind, WindInterpolation, WindLevel};
use hpr_core::DVec3;
use hpr_core::earth::{Earth, EarthRotation, GravityModel};
use hpr_core::geodesy::Geodetic;
use hpr_core::gravity::NormalGravity;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_design::{MassProperties, Rocket};
use hpr_sim::{
    Adaptive, Device, DeviceDrag, Direction, Environment, EventKind, FlightSettings, FlightStep,
    Method, Observer, Phase, Rail, Sample, SimError, Simulation, State, Termination, Trigger,
    UserEvent,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::case::{Case, CaseLock, DragMode, Flight, cases_dir, committed_cases};
use crate::metrics::{Measured, Reference};
use crate::report::{Comparison, Gap, Report, Source};

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

/// How close hpr's reference area has to be to the one the oracle flew its drag table on, as a
/// fraction. The drag force is `½ρV² A C_D`, so "the same drag" means the same `A` as well as the
/// same `C_D`; as with the mass, anything above rounding is two different vehicles.
const AREA_AGREEMENT: f64 = 1e-9;

/// Runs every case the lock names, in its order, and reports the comparisons.
///
/// # Errors
///
/// [`ValidateError`] if a file is missing or malformed, if the lock names a case that is not
/// there or a committed case is not locked ([Loft lesson L78][l78]), if a reference has no
/// provenance ([L77][l77]), if a metric is neither gated nor declared ([L79][l79]), or if a flight
/// fails.
///
/// [l77]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l77
/// [l78]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l78
/// [l79]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l79
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
    let mut gaps = Vec::new();
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
        let run = run_case(root, &case)?;
        // The report records the cases that were compared, not the ones a lock hoped for.
        cases.push(case.id.clone());
        comparisons.extend(run.comparisons);
        sources.push(run.source);
        gaps.extend(run.gap);
    }
    if comparisons.is_empty() {
        // L78 once more: "ok" over nothing is the report Loft's suites produced when their
        // fixtures were missing. A run that scored no metric has not validated anything.
        return Err(ValidateError::Case(format!(
            "the run covered {} case(s) and compared nothing; a suite that checks nothing is not \
             a suite that passes",
            cases.len()
        )));
    }
    Ok(Report {
        harness_version: env!("CARGO_PKG_VERSION").to_owned(),
        fast,
        cases,
        skipped: lock.skipped(fast),
        comparisons,
        gaps,
        sources,
    })
}

/// What running one case gives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CaseRun {
    /// One comparison per metric, or none for a known gap.
    pub comparisons: Vec<Comparison>,
    /// Where the reference came from.
    pub source: Source,
    /// The gap, when the case declares one and hpr refused the flight as it said.
    pub gap: Option<Gap>,
}

/// Runs one case: flies it, measures it, and compares every metric it names.
///
/// # Errors
///
/// As [`run_lock`].
pub fn run_case(root: &Path, case: &Case) -> Result<CaseRun, ValidateError> {
    // A name the flight cannot measure is a typo, and it costs nothing to say so before flying.
    let known: &[&str] = match &case.flight {
        Flight::RecoveryDescent { .. } => &DESCENT_METRICS,
        Flight::WholeFlight { .. } => &WHOLE_FLIGHT_METRICS,
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

    let source = Source {
        case: case.id.clone(),
        oracle: reference.oracle.clone(),
        generator: reference.generator.clone(),
        command: reference.command.clone(),
        file: reference.file.clone(),
        sha256: reference.sha256.clone(),
        model: reference.model.clone(),
        overrides: reference.overrides.clone(),
    };
    let gap = match &case.known_gap {
        None => None,
        Some(reason) if reason.trim().is_empty() => {
            // As with a metric that is not scored: an excuse nobody wrote down is not one.
            return Err(ValidateError::Case(format!(
                "case {}: its known gap gives no reason",
                case.id
            )));
        }
        Some(reason) => {
            // The only gap the harness accepts is hpr's refusal of M >= 1 (M1.8), and it checks
            // the reference agrees that the flight gets there before it lets the case off.
            let peak = reference.values.get("max_mach").map(|value| value.value);
            if !peak.is_some_and(|mach| mach >= 1.0) {
                return Err(ValidateError::Case(format!(
                    "case {}: it declares the M >= 1 gap, but the reference peaks at Mach {peak:?}",
                    case.id
                )));
            }
            Some(reason.trim().to_owned())
        }
    };
    let measured = match (measure(root, case, &setup)?, gap) {
        (Flown::Measured(measured), None) => measured,
        (Flown::Measured(_), Some(_)) => {
            // Loft lesson L85: a gap that has closed and still reads as excused is a gate that has
            // stopped watching. Score the case instead.
            return Err(ValidateError::Case(format!(
                "case {}: it declares a known gap, but hpr flew it to the ground; remove the gap \
                 and score it",
                case.id
            )));
        }
        (Flown::RefusedAtMach { mach }, Some(reason)) => {
            return Ok(CaseRun {
                comparisons: Vec::new(),
                source,
                gap: Some(Gap {
                    case: case.id.clone(),
                    reason,
                    // Rounded: the report is pinned across platforms. The integrator narrows its
                    // step onto the boundary, so this is Mach 1.000 wherever it runs.
                    refusal: format!(
                        "refused the flight at Mach {mach:.3}, outside its subsonic models' range \
                         of [0, 1)"
                    ),
                    mach,
                    metric_count: case.metrics.len(),
                }),
            });
        }
        (Flown::RefusedAtMach { mach }, None) => {
            // The error hpr raised, which carries nothing but this Mach number; the words are
            // rounded as the gap's are, since the Mach number's last bits differ by platform.
            return Err(ValidateError::Flight {
                case: case.id.clone(),
                what: format!(
                    "refused the flight at Mach {mach:.3}, outside its subsonic models' range of \
                     [0, 1), and the case declares no known gap"
                ),
                source: Some(Box::new(SimError::Aero(AeroError::Mach { mach }))),
            });
        }
    };
    let predicted = matches!(
        case.flight,
        Flight::WholeFlight {
            mode: DragMode::Predicted,
            ..
        }
    );
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
        comparisons.push(match (metric.reason(), predicted) {
            (Some(reason), _) => {
                Comparison::not_scored(&case.id, name, *got, value.value, &value.source, reason)
            }
            (None, false) => Comparison::new(
                &case.id,
                name,
                *got,
                value.value,
                &value.source,
                metric.tolerance(),
            ),
            // Predicted mode: the tolerance is a target, reported and never gated (ADR-023).
            (None, true) => Comparison::targeted(
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
    Ok(CaseRun {
        comparisons,
        source,
        gap: None,
    })
}

/// The inputs a reference records for its kind of flight.
#[derive(Debug, Clone, PartialEq)]
enum Setup {
    /// A descent's.
    Descent(DescentSetup),
    /// A whole flight's.
    WholeFlight(WholeFlightSetup),
}

/// How a flight came out.
#[derive(Debug, Clone, PartialEq)]
enum Flown {
    /// It reached the ground, and these are its metrics.
    Measured(Measured),
    /// hpr refused it at this Mach number, at or past 1, which its aerodynamics do not cover
    /// until M1.8.
    RefusedAtMach {
        /// The Mach number it refused.
        mach: f64,
    },
}

/// The reference a case names, in the harness's shape, with the inputs the oracle flew.
fn load_reference(root: &Path, case: &Case) -> Result<(Reference, Setup), ValidateError> {
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
    match &case.flight {
        Flight::RecoveryDescent { .. } => {
            crate::rocketpy::descent_case(&document, wanted, &file, &sha256)
                .map(|(reference, setup)| (reference, Setup::Descent(setup)))
        }
        Flight::WholeFlight { .. } => {
            crate::rocketpy::whole_flight_case(&document, wanted, &file, &sha256)
                .map(|(reference, setup)| (reference, Setup::WholeFlight(setup)))
        }
    }
    .ok_or_else(|| {
        ValidateError::Case(format!(
            "case {}: {} has no case {wanted} that names the run which produced it",
            case.id,
            path.display()
        ))
    })
}

/// Flies the case and measures what it reports.
fn measure(root: &Path, case: &Case, setup: &Setup) -> Result<Flown, ValidateError> {
    match (&case.flight, setup) {
        (
            Flight::RecoveryDescent {
                design,
                configuration,
            },
            Setup::Descent(setup),
        ) => fly_descent(root, case, setup, design, configuration).map(Flown::Measured),
        (
            Flight::WholeFlight {
                design,
                configuration,
                mode,
            },
            Setup::WholeFlight(setup),
        ) => fly_whole_flight(root, case, setup, design, configuration, *mode),
        _ => Err(ValidateError::Case(format!(
            "case {}: its reference was read for another kind of flight",
            case.id
        ))),
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
        let environment = rocketpy_environment(
            setup.latitude_deg,
            setup.longitude_deg,
            setup.elevation_m,
            &setup.wind,
        )?;
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
        let mass_gap = (properties.mass_kg - setup.dry_mass_kg).abs() / setup.dry_mass_kg.abs();
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
        // As the generator defines it: the height above the site it started from, over the time
        // it took. Measuring the ENU `z` drop instead would carry the curvature of the ground
        // (`drift²/2R`, 15 cm over Calisto's 1.4 km drift), which is not what the oracle divided.
        measured.insert(
            "mean_descent_rate_m_s",
            (setup.start_height_above_ground_m - landing.height_above_ground_m) / descent_time_s,
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

/// The environment a RocketPy case declares, flown with RocketPy's own models where hpr has them
/// (ADR-015): its standard atmosphere, the declared wind interpolated by component as RocketPy's
/// `custom_atmosphere` does, Coriolis, and RocketPy's gravity.
fn rocketpy_environment(
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_m: f64,
    wind: &[(f64, f64, f64)],
) -> Result<Environment, SimError> {
    let site = Geodetic::from_degrees(latitude_deg, longitude_deg, elevation_m)?;
    let levels: Vec<WindLevel> = wind
        .iter()
        .map(|(height_msl_m, east, north)| WindLevel {
            height_msl_m: *height_msl_m,
            speed_m_s: east.hypot(*north),
            direction_from_rad: (-east).atan2(-north).rem_euclid(std::f64::consts::TAU),
        })
        .collect();
    // L75, on a difference that took two reviews to find: hpr's default gravity is the full
    // normal-gravity *vector*, which above the ellipsoid leans very slightly north, while RocketPy
    // applies gravity to the vertical axis alone (`Flight.u_dot_parachute`, `flight.py:2777`, and
    // `u_dot_generalized` likewise). Over an 800 m descent that difference is 5.2e-4 m of
    // northward drift — 26 times the Coriolis drift it hides among. hpr ships RocketPy's formula
    // for exactly this reason, so a RocketPy comparison uses it.
    let earth = Earth::new(
        NormalGravity::wgs84(),
        site,
        GravityModel::VerticalTaylor,
        EarthRotation::Coriolis,
    )?;
    Ok(Environment::new(
        earth,
        AtmosphereModel::default(),
        LayeredWind::new(levels, WindInterpolation::Components)?,
    ))
}

/// A flight from the pad to the ground under the rail, site, wind and devices the reference
/// declares, and in same-drag mode its drag table (in predicted mode, the design's own drag):
/// `flight.py`'s flight, through hpr.
///
/// The metrics are measured as RocketPy defines them, which is not always as hpr's own events
/// would ([Loft lesson L80][l80]: the same word can name a different quantity):
///
/// - RocketPy's state follows the **centre of dry mass** (`flight.py`'s `vx`, `ax`), so speeds
///   and accelerations are that body point's, not the nose tip's or the burning rocket's centre
///   of mass: `v_P = v_O + ω × p` and `a_P = a_O + ω̇ × p + ω × (ω × p)`, with `p` the dry centre of
///   mass from the nose tip.
/// - RocketPy's rail exit is the **forward** button reaching the top of the rail
///   (`effective_1rl`, `flight.py:1716-1730`), where hpr's own rail exit is the aft-most guide's
///   ([Loft lesson L26][l26]). hpr is still on its rail then, so the harness finds the instant the
///   forward guide's travel is reached and reads the speed there.
/// - The maxima are over the solver's steps, both ends of each, as RocketPy's are over its
///   solution array, which starts each phase at its first instant. Unlike RocketPy's, they are
///   also over the peaks found inside a step on its dense output (`Peaks::peaks_within`), so that
///   they do not move with the step sequence (ADR-023). The power-on maximum is over those up to
///   burnout (`max_acceleration_power_on`).
///
/// [l26]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l26
/// [l80]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
fn fly_whole_flight(
    root: &Path,
    case: &Case,
    setup: &WholeFlightSetup,
    design: &str,
    configuration: &str,
    mode: DragMode,
) -> Result<Flown, ValidateError> {
    let refuse = |what: String| ValidateError::Flight {
        case: case.id.clone(),
        what,
        source: None,
    };
    // L75: the reference records which rocket the oracle flew, and with which drag.
    let named = format!("validation/designs/{design}.json");
    if named != setup.design {
        return Err(refuse(format!(
            "the case flies {named}, but the reference was flown with {}",
            setup.design
        )));
    }
    // Each mode needs the reference that means something against it: same-drag the declared
    // table, flown by both codes; predicted RocketPy flying the example's own drag, since hpr's
    // own drag scored against the declared constant would measure it against an arbitrary number.
    let declared = match (
        mode,
        &setup.cd0_vs_mach,
        &setup.declared_cd0_vs_mach,
        &setup.own_drag_source,
    ) {
        (DragMode::SameDrag, Some(flown), Some(declared), None) => {
            if flown != declared {
                return Err(refuse(format!(
                    "the reference's case flew C_D0(M) = {flown:?}, not the {declared:?} its \
                     generator declares"
                )));
            }
            Some(flown.clone())
        }
        (DragMode::Predicted, None, None, Some(_)) => None,
        (DragMode::SameDrag, ..) => {
            return Err(refuse(
                "a same-drag case needs a reference that flew its generator's declared C_D0(M), \
                 and this one did not"
                    .to_owned(),
            ));
        }
        (DragMode::Predicted, ..) => {
            return Err(refuse(
                "a predicted case needs a reference that flew the example's own drag; against a \
                 declared table it would score hpr's drag against an arbitrary constant"
                    .to_owned(),
            ));
        }
    };
    if let Some((first, rest)) = setup.devices.split_first() {
        if first.height_above_ground_m.is_some() {
            return Err(refuse(format!(
                "the reference's first device ({}) has to open at apogee, as RocketPy's first \
                 parachute does",
                first.name
            )));
        }
        if let Some(device) = rest.iter().find(|d| d.height_above_ground_m.is_none()) {
            return Err(refuse(format!(
                "device {} opens at apogee too; RocketPy's parachutes replace one another in \
                 order, which hpr flies as each releasing the one before",
                device.name
            )));
        }
    } else {
        return Err(refuse("the reference declares no devices".to_owned()));
    }

    let path = root.join(&named);
    let rocket: Rocket =
        serde_json::from_str(&read(&path, "reading a design")?).map_err(|source| {
            ValidateError::Json {
                path: path.display().to_string(),
                source: Box::new(source),
            }
        })?;
    let flight = || -> Result<Result<Flown, String>, SimError> {
        let environment = rocketpy_environment(
            setup.latitude_deg,
            setup.longitude_deg,
            setup.elevation_m,
            &setup.wind,
        )?;
        let rail = Rail {
            length_m: setup.rail_length_m,
            azimuth_rad: setup.heading_deg.to_radians(),
            elevation_rad: setup.inclination_deg.to_radians(),
            roll_rad: 0.0,
            // RocketPy's rail has no friction (ADR-011).
            friction_coefficient: 0.0,
        };
        let simulation = Simulation::new(
            &rocket,
            configuration,
            environment,
            rail,
            FlightSettings {
                max_time_s: 6000.0,
                method: match mode {
                    DragMode::Predicted => Method::DormandPrince54(Adaptive {
                        relative_tolerance: PREDICTED_TOLERANCE,
                        absolute_tolerance: PREDICTED_TOLERANCE,
                        ..Adaptive::default()
                    }),
                    _ => FlightSettings::default().method,
                },
                ..FlightSettings::default()
            },
        )?;
        // Predicted mode flies the design's own aerodynamics: no table.
        let simulation = match &declared {
            Some(rows) => {
                let (machs, coefficients): (Vec<f64>, Vec<f64>) = rows.iter().copied().unzip();
                let table = Table1D::new(
                    machs,
                    coefficients,
                    Interpolation::Linear,
                    // Outside the declared Mach range there is no declared drag, so none is
                    // invented.
                    Extrapolation::Error,
                )?;
                simulation.with_drag_table(
                    DragTable::new(table.clone(), Some(table))
                        .with_reference_diameter_m(2.0 * setup.reference_radius_m),
                )
            }
            None => simulation,
        };

        // L75: the same rocket has to weigh the same, fly its drag on the same area and burn the
        // same motor in both codes before any difference in how it flies can be read as physics.
        let assembly = simulation.assembly();
        let dry = assembly
            .motors
            .iter()
            .fold(assembly.layout.structure, |sum, motor| {
                MassProperties::combine([&sum, &motor.dry_mass_properties()])
            });
        let mass_gap = (dry.mass_kg - setup.dry_mass_kg).abs() / setup.dry_mass_kg;
        if !mass_gap.is_finite() || mass_gap > MASS_AGREEMENT {
            return Ok(Err(format!(
                "hpr's dry mass is {:.6} kg where the reference recorded {:.6} kg, a relative \
                 {mass_gap:.3e}",
                dry.mass_kg, setup.dry_mass_kg
            )));
        }
        let radius_m = 0.5 * assembly.layout.reference_diameter_m;
        let area_m2 = std::f64::consts::PI * radius_m * radius_m;
        let area_gap = (area_m2 - setup.reference_area_m2).abs() / setup.reference_area_m2;
        let radius_gap = (radius_m - setup.reference_radius_m).abs() / setup.reference_radius_m;
        if !(area_gap.is_finite() && radius_gap.is_finite())
            || area_gap > AREA_AGREEMENT
            || radius_gap > AREA_AGREEMENT
        {
            return Ok(Err(format!(
                "hpr's reference area is {area_m2:.9} m2 (radius {radius_m:.6} m) where the \
                 reference flew its drag on {:.9} m2 (radius {:.6} m)",
                setup.reference_area_m2, setup.reference_radius_m
            )));
        }
        let [placed] = assembly.motors.as_slice() else {
            return Ok(Err(format!(
                "the design flies {} motors, where RocketPy's examples fly one",
                assembly.motors.len()
            )));
        };
        let motor = &placed.mounted.motor;
        for (what, hpr, rocketpy) in [
            (
                "total impulse, N s",
                motor.curve().total_impulse_ns(),
                setup.motor.total_impulse_ns,
            ),
            (
                "burn-out time, s",
                motor.burnout_time_s(),
                setup.motor.burn_out_time_s,
            ),
            (
                "initial propellant mass, kg",
                motor.propellant_initial_mass_kg(),
                setup.motor.propellant_initial_mass_kg,
            ),
        ] {
            let gap = (hpr - rocketpy).abs() / rocketpy.abs();
            if !gap.is_finite() || gap > MASS_AGREEMENT {
                return Ok(Err(format!(
                    "hpr's motor has a {what} of {hpr} where the reference flew {rocketpy}, a \
                     relative {gap:.3e}"
                )));
            }
        }
        // The input this milestone found transcribed wrong: a correction for ambient pressure
        // that RocketPy's examples never apply.
        let reference_pa = motor
            .nozzle()
            .and_then(|nozzle| nozzle.reference_pressure_pa);
        if reference_pa != setup.motor.reference_pressure_pa {
            return Ok(Err(format!(
                "hpr's motor corrects its thrust for a reference pressure of {reference_pa:?} Pa, \
                 where the reference flew {:?}",
                setup.motor.reference_pressure_pa
            )));
        }

        // RocketPy's tracked point, the centre of dry mass, starts at the ground
        // (`z_init = elevation`, flight.py:1553); hpr's starts `h0` above it, with the rocket's
        // aft end at the rail's foot. So heights are measured from `h0`, the main opens `h0`
        // higher, and the flight lands when it is back at `h0`, where RocketPy's does.
        let start = simulation.initial_state();
        let origin = start.position_enu_m + start.attitude.mul_vec3(dry.cg_m);
        let h0 = origin.z;
        let devices = setup
            .devices
            .iter()
            .enumerate()
            .map(|(index, device)| {
                let trigger = match device.height_above_ground_m {
                    Some(height_above_ground_m) => Trigger::Altitude {
                        height_above_ground_m: height_above_ground_m + h0,
                    },
                    None => Trigger::Apogee,
                };
                let mut built = Device::new(
                    &device.name,
                    DeviceDrag::DragArea {
                        cd_s_m2: device.cd_s_m2,
                    },
                    trigger,
                )
                .with_lag_s(device.lag_s);
                // RocketPy flies one parachute at a time, the last deployed (flight.py:1404-1431);
                // hpr sums its open devices, so each one releases the one before as it opens.
                if index + 1 < setup.devices.len() {
                    built = built.with_release_by(index + 1);
                }
                built
            })
            .collect();
        let simulation = simulation.with_recovery(devices)?.with_event(UserEvent {
            name: "back at the starting height".to_owned(),
            direction: Direction::Falling,
            function: Box::new(move |sample: &Sample| sample.height_above_ground_m - h0),
        });

        let mut peaks = Peaks {
            dry_cg_m: dry.cg_m,
            rail_axis_enu: rail.direction_enu(),
            start_enu_m: start.position_enu_m,
            forward_guide_travel_m: setup.effective_1rl_m,
            forward_guide_exit: None,
            rows: Vec::new(),
            start_height_m: h0,
            grid_s: setup.series.iter().map(|&(t_s, _, _)| t_s).collect(),
            on_grid: Vec::new(),
        };
        let result = match simulation.run(&mut peaks) {
            Ok(result) => result,
            // Only a real Mach number at or past 1: the aerodynamics raise the same error for a
            // NaN, and that is a failure, not the known gap.
            Err(SimError::Aero(AeroError::Mach { mach })) if mach.is_finite() && mach >= 1.0 => {
                return Ok(Ok(Flown::RefusedAtMach { mach }));
            }
            Err(error) => return Err(error),
        };
        if result.termination != Termination::GroundHit {
            return Ok(Err(format!(
                "the flight ended as {:?} after {} steps, not at the ground",
                result.termination, result.stats.accepted_steps
            )));
        }
        let event = |kind: EventKind| {
            result
                .event(kind)
                .map(|event| event.sample)
                .ok_or_else(|| format!("the flight has no {kind:?} event"))
        };
        let (apogee, burnout, landing) = match (
            event(EventKind::Apogee),
            event(EventKind::Burnout),
            event(EventKind::User(0)),
        ) {
            (Ok(apogee), Ok(burnout), Ok(landing)) => (apogee, burnout, landing),
            (Err(what), _, _) | (_, Err(what), _) | (_, _, Err(what)) => return Ok(Err(what)),
        };
        let Some((rail_exit_time_s, rail_exit_speed_m_s)) = peaks.forward_guide_exit else {
            return Ok(Err(format!(
                "the rocket never travelled RocketPy's effective_1rl, {} m, along the rail",
                setup.effective_1rl_m
            )));
        };
        // The flight is RocketPy's until it lands, which is where its record ends.
        let flown: Vec<&Row> = peaks
            .rows
            .iter()
            .filter(|row| row.time_s <= landing.time_s)
            .collect();
        // `f64::max` passes over a NaN, so a sample that is not a number would vanish from the
        // maxima below rather than fail them. Refuse the flight instead.
        if let Some(row) = flown.iter().find(|row| !row.is_finite()) {
            return Ok(Err(format!(
                "the flight has a sample that is not a number at {} s: {row:?}",
                row.time_s
            )));
        }
        let fastest = flown.iter().map(|row| row.speed_m_s).fold(0.0, f64::max);
        let max_mach = flown.iter().map(|row| row.mach).fold(0.0, f64::max);
        let hardest = |rows: &mut dyn Iterator<Item = &&Row>| {
            rows.fold((0.0, 0.0), |(peak, at), row| {
                if row.acceleration_m_s2 > peak {
                    (row.acceleration_m_s2, row.time_s)
                } else {
                    (peak, at)
                }
            })
        };
        let (max_acceleration, max_acceleration_time_s) = hardest(&mut flown.iter());
        let (max_acceleration_power_on, _) =
            hardest(&mut flown.iter().filter(|row| row.time_s <= burnout.time_s));
        // Where it went, from where the dry centre of mass started, as RocketPy's x and y are.
        let drift = |state: &State| {
            (state.position_enu_m + state.attitude.mul_vec3(dry.cg_m) - origin)
                .truncate()
                .length()
        };

        let mut measured = Measured::default();
        measured.insert("apogee_agl_m", apogee.height_above_ground_m - h0);
        measured.insert("apogee_time_s", apogee.time_s);
        measured.insert("max_speed_m_s", fastest);
        measured.insert("max_mach", max_mach);
        measured.insert("max_acceleration_m_s2", max_acceleration);
        measured.insert("max_acceleration_time_s", max_acceleration_time_s);
        measured.insert("max_acceleration_power_on_m_s2", max_acceleration_power_on);
        measured.insert("rail_exit_speed_m_s", rail_exit_speed_m_s);
        measured.insert("rail_exit_time_s", rail_exit_time_s);
        // At burnout the propellant is gone, so the centre of mass is the dry one.
        measured.insert("burnout_altitude_agl_m", burnout.height_above_ground_m - h0);
        measured.insert(
            "burnout_speed_m_s",
            point_velocity(&burnout.state, dry.cg_m).length(),
        );
        measured.insert("flight_time_s", landing.time_s);
        measured.insert("apogee_drift_m", drift(&apogee.state));
        measured.insert("landing_drift_m", drift(&landing.state));
        measured.insert("impact_speed_m_s", -landing.vertical_speed_m_s);
        // The two trajectories at the reference's times: both clocks start at ignition with the
        // rocket on the rail, so the times align as they are, and no shift is fitted, which would
        // hide a difference in the burn. The comparison runs while both fly: until hpr lands, as
        // the reference's series ends where it does.
        let both_fly: Vec<(&SeriesRow, &SeriesRow)> = setup
            .series
            .iter()
            .zip(&peaks.on_grid)
            .filter(|(_, ours)| ours.0 <= landing.time_s)
            .collect();
        if let Some((_, ours)) = both_fly
            .iter()
            .find(|(_, ours)| !(ours.1.is_finite() && ours.2.is_finite()))
        {
            return Ok(Err(format!(
                "the flight's trajectory is not a number at {} s: {ours:?}",
                ours.0
            )));
        }
        if both_fly.is_empty() {
            return Ok(Err(
                "the flight reached none of the reference's series times".into(),
            ));
        }
        let rms = |difference: fn(&SeriesRow, &SeriesRow) -> f64| {
            let sum: f64 = both_fly
                .iter()
                .map(|(theirs, ours)| difference(theirs, ours).powi(2))
                .sum();
            #[allow(
                clippy::cast_precision_loss,
                reason = "a count of at most the series' 120 rows is exact in an f64"
            )]
            let count = both_fly.len() as f64;
            (sum / count).sqrt()
        };
        measured.insert("series_height_rms_m", rms(|theirs, ours| ours.1 - theirs.1));
        measured.insert(
            "series_speed_rms_m_s",
            rms(|theirs, ours| ours.2 - theirs.2),
        );
        Ok(Ok(Flown::Measured(measured)))
    };
    match flight() {
        Ok(Ok(flown)) => Ok(flown),
        Ok(Err(what)) => Err(refuse(what)),
        Err(source) => Err(ValidateError::Flight {
            case: case.id.clone(),
            what: source.to_string(),
            source: Some(Box::new(source)),
        }),
    }
}

/// The velocity of the body point `p` (body axes, from the nose tip), m/s: `v_O + R (ω × p)`.
fn point_velocity(state: &State, p: DVec3) -> DVec3 {
    state.velocity_enu_m_s + state.attitude.mul_vec3(state.body_rate_rad_s.cross(p))
}

/// One instant of the flight, as the whole-flight metrics need it.
#[derive(Debug, Clone, Copy)]
struct Row {
    time_s: f64,
    speed_m_s: f64,
    mach: f64,
    acceleration_m_s2: f64,
}

impl Row {
    /// Whether every quantity in the row is a number.
    fn is_finite(&self) -> bool {
        self.speed_m_s.is_finite() && self.mach.is_finite() && self.acceleration_m_s2.is_finite()
    }

    /// The quantities whose peaks the metrics report, each as a function of a row.
    const PEAKED: [fn(&Row) -> f64; 3] = [
        |row| row.speed_m_s,
        |row| row.mach,
        |row| row.acceleration_m_s2,
    ];
}

/// Golden-section iterations that narrow onto a peak inside one step: each keeps 0.618 of the
/// bracket, so 50 leave 3.5e-11 of the step's length.
const PEAK_ITERATIONS: usize = 50;

/// Where `value` peaks between `low` and `high`, by golden-section search (Kiefer 1953, "Sequential
/// minimax search for a maximum", Proc. AMS 4(3), 502-506): each iteration keeps 0.618 of the
/// bracket, for [`PEAK_ITERATIONS`] iterations. For a `value` that rises to one peak and then
/// falls, the answer is within 3.5e-11 of the bracket's length of a kinked peak. A smooth peak's
/// top is flat to the value's rounding over a wider span, so the answer is somewhere on it and
/// the value there is the peak's to rounding.
///
/// # Errors
///
/// The first error `value` returns.
pub(crate) fn peak_between<E>(
    low: f64,
    high: f64,
    mut value: impl FnMut(f64) -> Result<f64, E>,
) -> Result<f64, E> {
    let keep = 0.5 * (5.0_f64.sqrt() - 1.0);
    let (mut low, mut high) = (low, high);
    let (mut left, mut right) = (high - keep * (high - low), low + keep * (high - low));
    let (mut at_left, mut at_right) = (value(left)?, value(right)?);
    for _ in 0..PEAK_ITERATIONS {
        if at_left > at_right {
            (high, right, at_right) = (right, left, at_left);
            left = high - keep * (high - low);
            at_left = value(left)?;
        } else {
            (low, left, at_left) = (left, right, at_right);
            right = low + keep * (high - low);
            at_right = value(right)?;
        }
    }
    Ok(if at_left > at_right { left } else { right })
}

/// Watches a whole flight for what its metrics need: speed, Mach and acceleration at the dry centre
/// of mass at every step's ends and at each of their peaks inside a step, and the instant the
/// forward guide leaves the rail.
struct Peaks {
    /// The dry centre of mass, body axes from the nose tip, m.
    dry_cg_m: DVec3,
    /// The rail's axis, up the rail.
    rail_axis_enu: DVec3,
    /// The nose tip's position at ignition, m.
    start_enu_m: DVec3,
    /// How far the rocket travels along the rail before RocketPy's forward button leaves its top,
    /// m (`effective_1rl`).
    forward_guide_travel_m: f64,
    /// When that happened and the speed then, once it has.
    forward_guide_exit: Option<(f64, f64)>,
    /// A row at every step's start and end, and at every peak inside a step.
    rows: Vec<Row>,
    /// The height the dry centre of mass started at, m: RocketPy's zero.
    start_height_m: f64,
    /// The reference's series times, s since ignition.
    grid_s: Vec<f64>,
    /// `(time, height from the start, speed)` of the dry centre of mass at each of those times the
    /// flight has reached, from the dense output of the step that holds it, s, m and m/s.
    on_grid: Vec<SeriesRow>,
}

impl Peaks {
    /// The row at `t_s` within `step`.
    fn row(&self, step: &dyn FlightStep, t_s: f64) -> Result<Option<Row>, SimError> {
        let sample: Sample = step.sample(t_s)?;
        let state = sample.state;
        let omega = state.body_rate_rad_s;
        let p = self.dry_cg_m;
        // ω̇ from the step's own interpolant, whose derivative is the integrator's to the order
        // of its dense output: a backward difference over a microsecond, or over the step's first
        // quarter if it is shorter. Off the free phase nothing rotates.
        let omega_dot = if step.phase() == Phase::Free {
            let back = (0.25 * (t_s - step.start_s())).min(1e-6);
            let ahead = (0.25 * (step.end_s() - t_s)).min(1e-6);
            if back > 0.0 {
                (omega - step.state_at(t_s - back).body_rate_rad_s) / back
            } else if ahead > 0.0 {
                (step.state_at(t_s + ahead).body_rate_rad_s - omega) / ahead
            } else {
                // A step of no length has no interpolant to differentiate, and its instant is
                // the end of the step before it, which is already a row.
                return Ok(None);
            }
        } else {
            DVec3::ZERO
        };
        let acceleration = sample.acceleration_enu_m_s2
            + state
                .attitude
                .mul_vec3(omega_dot.cross(p) + omega.cross(omega.cross(p)));
        Ok(Some(Row {
            time_s: t_s,
            speed_m_s: point_velocity(&state, p).length(),
            mach: sample.mach,
            acceleration_m_s2: acceleration.length(),
        }))
    }

    /// The rows at the peaks of speed, Mach and acceleration that fall inside `step`.
    ///
    /// A peak read only where steps end is off by wherever the step control put them, by O(h²) of
    /// the step length h at a smooth peak. (A thrust curve's points are stop times, so a peak there
    /// is a step's end and read exactly.) The step control is not the same on every platform
    /// (predicted mode's drag calls `ln` and `powf`), so neither is such a peak: moving predicted
    /// mode's solver tolerance by 1e-7 of itself moved NDRT 2020's max speed by 2.4e-6 of itself,
    /// where the event-located apogee moved by 1.3e-9. So where a quantity rises out of the step's start and falls into its end,
    /// as read one microsecond (or a quarter of the step) inside each, a golden-section search on
    /// the step's dense output narrows onto the peak between them. What it finds is the
    /// interpolant's peak, which the tolerance controls, wherever the steps fall.
    ///
    /// Its limits, measured by sampling every step at 400 points: a step whose quantity turns more
    /// than once, or jumps (the skin friction at the critical Reynolds number), is not searched;
    /// none of those is a flight's maximum today. And the acceleration an evaluation of the
    /// equations of motion gives is smooth only to about 1e-7 m/s², so a smooth acceleration peak
    /// is found to about 1e-8 of itself and its time only to about 1e-4 s (issue #53).
    ///
    /// `first` and `last` are the rows at the step's start and end. A sample inside the step that
    /// is not a number comes back as a row of its own, so that the flight is refused for it rather
    /// than the comparisons passing over it.
    fn peaks_within(
        &self,
        step: &dyn FlightStep,
        first: &Row,
        last: &Row,
    ) -> Result<Vec<Row>, SimError> {
        let (start_s, end_s) = (step.start_s(), step.end_s());
        let inside = (0.25 * (end_s - start_s)).min(1e-6);
        // A step of no length has no inside: in free flight `row` gives none there, and elsewhere
        // every sample of it is one instant, so no quantity rises out of its start.
        let (Some(second), Some(penultimate)) = (
            self.row(step, start_s + inside)?,
            self.row(step, end_s - inside)?,
        ) else {
            return Ok(Vec::new());
        };
        let mut not_a_number = [second, penultimate]
            .into_iter()
            .find(|row| !row.is_finite());
        let mut peaks = Vec::new();
        for quantity in Row::PEAKED {
            if !(quantity(&second) > quantity(first) && quantity(&penultimate) > quantity(last)) {
                continue;
            }
            let peak_s = peak_between(start_s, end_s, |t_s| -> Result<f64, SimError> {
                Ok(match self.row(step, t_s)? {
                    Some(row) if row.is_finite() => quantity(&row),
                    Some(row) => {
                        not_a_number.get_or_insert(row);
                        f64::NAN
                    }
                    // Only a step of no length, returned from above.
                    None => f64::NEG_INFINITY,
                })
            })?;
            peaks.extend(self.row(step, peak_s)?);
        }
        peaks.extend(not_a_number);
        Ok(peaks)
    }

    /// How far past the forward guide's exit the rocket has travelled at `t_s`, m.
    fn past_forward_guide(&self, step: &dyn FlightStep, t_s: f64) -> f64 {
        (step.state_at(t_s).position_enu_m - self.start_enu_m).dot(self.rail_axis_enu)
            - self.forward_guide_travel_m
    }
}

impl Observer for Peaks {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let (start_s, end_s) = (step.start_s(), step.end_s());
        // Each step's start as well as its end: after an event the start is the new phase's first
        // instant, such as a canopy fully open, where the deceleration peaks. Taking only the ends
        // would read that peak one step late, wherever the step control happens to put it, which
        // differs across platforms in the sixth figure where the peak falls steeply.
        let (first, last) = (self.row(step, start_s)?, self.row(step, end_s)?);
        let peaks = match (&first, &last) {
            (Some(first), Some(last)) => self.peaks_within(step, first, last)?,
            _ => Vec::new(),
        };
        self.rows.extend(first);
        self.rows.extend(last);
        self.rows.extend(peaks);
        // Each series time in the first step that reaches it: the one before ended short of it,
        // so it lies inside this step.
        while let Some(&t_s) = self.grid_s.get(self.on_grid.len()) {
            if t_s > end_s {
                break;
            }
            let state = step.state_at(t_s);
            let height_m = (state.position_enu_m + state.attitude.mul_vec3(self.dry_cg_m)).z
                - self.start_height_m;
            let speed_m_s = point_velocity(&state, self.dry_cg_m).length();
            self.on_grid.push((t_s, height_m, speed_m_s));
        }
        if self.forward_guide_exit.is_none()
            && step.phase() == Phase::Rail
            && self.past_forward_guide(step, end_s) >= 0.0
        {
            // Bisect the step's interpolant for the crossing. On the rail nothing rotates, so
            // every point of the rocket moves at the speed the state carries.
            let (mut before, mut after) = (start_s, end_s);
            for _ in 0..80 {
                let middle = 0.5 * (before + after);
                if self.past_forward_guide(step, middle) >= 0.0 {
                    after = middle;
                } else {
                    before = middle;
                }
            }
            let speed = step.state_at(after).velocity_enu_m_s.length();
            self.forward_guide_exit = Some((after, speed));
        }
        Ok(())
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

/// The metric names a whole-flight case can report: `flight.py`'s, as RocketPy defines them, and
/// the root mean square of hpr's height and speed less the reference's `series`, whose
/// reference is 0, exact agreement.
pub const WHOLE_FLIGHT_METRICS: [&str; 17] = [
    "apogee_agl_m",
    "apogee_time_s",
    "max_speed_m_s",
    "max_mach",
    "max_acceleration_m_s2",
    "max_acceleration_time_s",
    "max_acceleration_power_on_m_s2",
    "rail_exit_speed_m_s",
    "rail_exit_time_s",
    "burnout_altitude_agl_m",
    "burnout_speed_m_s",
    "flight_time_s",
    "apogee_drift_m",
    "landing_drift_m",
    "impact_speed_m_s",
    "series_height_rms_m",
    "series_speed_rms_m_s",
];

/// The solver tolerance, `rtol` and `atol` alike, predicted mode flies at (ADR-023).
///
/// Its aerodynamics call `ln` and `powf` (the skin friction), whose last bits differ between the
/// platforms' maths libraries, so the adaptive step sequence can differ between them, and the answer
/// then differs by the solver's global error. At the default 1e-8, NDRT 2020's predicted apogee was
/// 1404.058522 m on macOS and 1404.058761 m on Linux, 1.7e-7 apart, past the committed report's
/// 1e-7 reproduction bound (ADR-022). Measured on macOS, that apogee is 1404.058522, .057883,
/// .058122 and .058145 m at 1e-8, 1e-9, 1e-10 and 1e-11: converged at 1e-11 to about 1e-5 m, far
/// inside the bound, for 0.5 s more over the whole suite. Same-drag mode's table interpolation
/// reproduces at the default and keeps it.
const PREDICTED_TOLERANCE: f64 = 1e-11;

/// The parts of a reference a whole-flight case needs, read from the generator's own JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WholeFlightSetup {
    /// The design the oracle flew, from the repository root.
    pub design: String,
    /// What it weighed with its propellant gone, kg.
    pub dry_mass_kg: f64,
    /// The site's latitude, degrees.
    pub latitude_deg: f64,
    /// Its longitude, degrees.
    pub longitude_deg: f64,
    /// Its elevation above sea level, m.
    pub elevation_m: f64,
    /// The wind as `(height above sea level, east, north)` levels, m and m/s.
    pub wind: Vec<(f64, f64, f64)>,
    /// The rail's length, m.
    pub rail_length_m: f64,
    /// The rail's angle above the horizon, degrees (RocketPy's `inclination`).
    pub inclination_deg: f64,
    /// Its heading, clockwise from north, degrees.
    pub heading_deg: f64,
    /// How far RocketPy's rocket travels along the rail before its forward button leaves the top,
    /// m (`effective_1rl`): where the rail-exit metrics are taken.
    pub effective_1rl_m: f64,
    /// The motor as RocketPy flew it, which hpr's must match.
    pub motor: FlightMotor,
    /// The `(Mach, C_D0)` rows the case flew, power on and off alike, in a same-drag reference;
    /// `None` in a reference that flew the example's own drag.
    pub cd0_vs_mach: Option<Vec<(f64, f64)>>,
    /// The rows the generator declares for every case, which the case's must equal; `None` in a
    /// reference that flew the example's own drag.
    pub declared_cd0_vs_mach: Option<Vec<(f64, f64)>>,
    /// Where the example's own drag came from, in a reference that flew it: recorded by its
    /// source and hash, never its values, which carry their own terms
    /// ([ADR-009](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17),
    /// the drag decisions).
    pub own_drag_source: Option<String>,
    /// The radius the drag coefficients are on, m (RocketPy's `Rocket(radius)`).
    pub reference_radius_m: f64,
    /// The area they are on, m².
    pub reference_area_m2: f64,
    /// The recovery devices, in the order they open.
    pub devices: Vec<FlightDevice>,
    /// The reference's trajectory as `(time since ignition s, height above the ground m, speed
    /// m/s)` rows, of the centre of dry mass, from ignition to its impact: what the RMS metrics
    /// compare hpr's against.
    pub series: Vec<SeriesRow>,
}

/// One instant of a trajectory: time since ignition, height of the centre of dry mass above where
/// it started, and its speed, s, m and m/s.
pub type SeriesRow = (f64, f64, f64);

/// The motor of a whole flight, as the reference records RocketPy flying it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlightMotor {
    /// The thrust curve's total impulse, N s.
    pub total_impulse_ns: f64,
    /// When the curve ends, s after ignition.
    pub burn_out_time_s: f64,
    /// The propellant's mass at ignition, kg.
    pub propellant_initial_mass_kg: f64,
    /// The pressure the curve is corrected from, Pa; `None` for no correction.
    pub reference_pressure_pa: Option<f64>,
}

/// One recovery device of a whole flight, as the reference declares it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightDevice {
    /// Its name, for reports.
    pub name: String,
    /// Its drag area `C_D S`, m².
    pub cd_s_m2: f64,
    /// Its lag from the trigger to line stretch, s.
    pub lag_s: f64,
    /// The height above the site it opens at on the way down, m; `None` opens it at apogee.
    pub height_above_ground_m: Option<f64>,
}

/// The parts of a reference a descent case needs, read from the generator's own JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DescentSetup {
    /// The design the oracle flew, from the repository root.
    pub design: String,
    /// What it weighed as it descended, kg.
    pub dry_mass_kg: f64,
    /// How far above the site the descent starts, m: the generator's own numerator for the mean
    /// descent rate.
    pub start_height_above_ground_m: f64,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[cfg(test)]
mod peak_tests {
    use hpr_core::DQuat;

    use super::*;

    /// A free-flight step whose rocket climbs straight up at `speed(t)`, at a steady 9 m/s².
    struct Climb {
        start_s: f64,
        end_s: f64,
        speed: fn(f64) -> f64,
    }

    impl FlightStep for Climb {
        fn phase(&self) -> Phase {
            Phase::Free
        }
        fn start_s(&self) -> f64 {
            self.start_s
        }
        fn end_s(&self) -> f64 {
            self.end_s
        }
        fn state_at(&self, t_s: f64) -> State {
            State {
                position_enu_m: DVec3::ZERO,
                velocity_enu_m_s: DVec3::new(0.0, 0.0, (self.speed)(t_s)),
                attitude: DQuat::IDENTITY,
                body_rate_rad_s: DVec3::ZERO,
            }
        }
        fn sample(&self, t_s: f64) -> Result<Sample, SimError> {
            let state = self.state_at(t_s);
            let speed = state.velocity_enu_m_s.z;
            Ok(Sample {
                time_s: t_s,
                phase: Phase::Free,
                state,
                cg_enu_m: DVec3::ZERO,
                cg_velocity_enu_m_s: state.velocity_enu_m_s,
                height_above_ground_m: 0.0,
                vertical_speed_m_s: speed,
                acceleration_enu_m_s2: DVec3::new(0.0, 0.0, 9.0),
                airspeed_m_s: speed,
                mach: speed / 340.0,
                angle_of_attack_rad: 0.0,
                dynamic_pressure_pa: 0.0,
                axial_coefficient: 0.0,
                thrust_n: 0.0,
                mass_kg: 1.0,
                recovery_drag_area_m2: 0.0,
            })
        }
    }

    /// The rows the observer keeps for one step.
    fn rows(start_s: f64, end_s: f64, speed: fn(f64) -> f64) -> Vec<Row> {
        let mut peaks = Peaks {
            dry_cg_m: DVec3::ZERO,
            rail_axis_enu: DVec3::Z,
            start_enu_m: DVec3::ZERO,
            forward_guide_travel_m: 1.0,
            forward_guide_exit: None,
            rows: Vec::new(),
            start_height_m: 0.0,
            grid_s: Vec::new(),
            on_grid: Vec::new(),
        };
        let step = Climb {
            start_s,
            end_s,
            speed,
        };
        peaks.step(&step).expect("the stub's samples never fail");
        peaks.rows
    }

    #[test]
    fn a_peak_inside_a_step_gets_a_row_and_a_rise_does_not() {
        // Speed, and so Mach, peak at 0.013 s inside the step; the acceleration is steady. The
        // ends and the two peaks give four rows, the peaks at the top to rounding.
        let peaked = rows(0.0, 0.05, |t| 100.0 - 1e4 * (t - 0.013).powi(2));
        assert_eq!(peaked.len(), 4, "{peaked:?}");
        for row in &peaked[2..] {
            assert!((row.time_s - 0.013).abs() < 1e-8, "{row:?}");
            assert!((row.speed_m_s - 100.0).abs() < 1e-12, "{row:?}");
        }
        // A step that only rises has its peak at its end, which is a row already.
        assert_eq!(rows(0.0, 0.05, |t| 100.0 + t).len(), 2);
        // A step of no length has no row at all in free flight: its instant is the end of the
        // step before it.
        assert!(rows(0.03, 0.03, |t| 100.0 + t).is_empty());
    }

    #[test]
    fn a_sample_inside_a_step_that_is_not_a_number_is_kept_to_be_refused() {
        // Finite at the ends and a microsecond inside them, not a number around the peak.
        let rows = rows(0.0, 0.05, |t| {
            if (t - 0.013).abs() < 1e-3 {
                f64::NAN
            } else {
                100.0 - 1e4 * (t - 0.013).powi(2)
            }
        });
        assert!(rows.iter().any(|row| !row.is_finite()), "{rows:?}");
    }
}
