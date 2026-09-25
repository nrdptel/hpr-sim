//! What a flight metric measures, tool by tool and version by version, and what becomes of a
//! metric whose event never happened ([M2.2d1][m2-2d1]).
//!
//! A reference names a quantity with a word: `maxvelocity`, `deploymentvelocity`,
//! `groundhitvelocity`, `optimumdelay`. The same word can mean a different quantity in another
//! tool, or in another version of the same tool ([Loft lesson L80][l80]). So a metric is compared
//! only through its [`Definition`], looked up by [`definition`] for the [`Tool`] and version that
//! produced the reference. A version whose meaning has not been measured has no definition, and
//! its value is withheld rather than compared as if it meant what the newest version means.
//!
//! OpenRocket 24.12's definitions are measured, not read from its source (which is GPL):
//! `validation/oracles/openrocket/flights.py` flies every configuration of the public designs and
//! records each summary word beside the quantities of OpenRocket's own time series it could mean.
//! The record is `validation/fixtures/ork/openrocket-flights.json`, and a test holds each
//! definition below to it on every flight ([ADR-068][adr-068]).
//!
//! A metric taken at an event that did not happen, such as the deployment speed of a flight whose
//! parachute never opened, has no value. OpenRocket writes `NaN` for it. Loft scored it as 0
//! ([Loft lesson L81][l81]). [`compare`] withholds it instead, with the event named.
//!
//! [m2-2d1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d1
//! [l80]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l80
//! [l81]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l81
//! [adr-068]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-068-openrockets-flights-of-the-public-designs-and-what-its-metric-words-mean-2026-09-25

use serde::{Deserialize, Serialize};

/// A tool that produced a reference flight, with its version as the tool writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Tool {
    /// OpenRocket, for example version `24.12`.
    OpenRocket {
        /// The version, as in the `.ork`'s `creator` attribute after `OpenRocket `.
        version: String,
    },
    /// RocketPy, for example version `1.13.0`.
    RocketPy {
        /// The version, as `rocketpy.__version__` gives it.
        version: String,
    },
}

impl Tool {
    /// The tool named by a `.ork`'s root `creator` attribute, such as `OpenRocket 24.12`. `None`
    /// for any other writer.
    #[must_use]
    pub fn from_ork_creator(creator: &str) -> Option<Self> {
        let version = creator.trim().strip_prefix("OpenRocket ")?.trim();
        (!version.is_empty()).then(|| Self::OpenRocket {
            version: version.to_owned(),
        })
    }
}

/// A summary quantity of one flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FlightMetric {
    /// The highest point above the launch site, m.
    Apogee,
    /// The largest speed, m/s.
    MaxSpeed,
    /// The speed at launch rod (or rail) clearance, m/s.
    RodClearanceSpeed,
    /// The static stability margin at launch rod clearance, calibres.
    RodClearanceStability,
    /// The speed at a recovery device's deployment, m/s.
    DeploymentSpeed,
    /// The speed at ground hit, m/s.
    GroundHitSpeed,
    /// The motor delay that would fire the ejection charge at apogee, s.
    OptimumDelay,
}

impl FlightMetric {
    /// Every metric, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Apogee,
        Self::MaxSpeed,
        Self::RodClearanceSpeed,
        Self::RodClearanceStability,
        Self::DeploymentSpeed,
        Self::GroundHitSpeed,
        Self::OptimumDelay,
    ];

    /// The attribute of a `.ork`'s stored `<flightdata>` that carries this metric, or `None` for
    /// the stability margin, which OpenRocket keeps only in its time series.
    #[must_use]
    pub const fn ork_summary_word(self) -> Option<&'static str> {
        match self {
            Self::Apogee => Some("maxaltitude"),
            Self::MaxSpeed => Some("maxvelocity"),
            Self::RodClearanceSpeed => Some("launchrodvelocity"),
            Self::RodClearanceStability => None,
            Self::DeploymentSpeed => Some("deploymentvelocity"),
            Self::GroundHitSpeed => Some("groundhitvelocity"),
            Self::OptimumDelay => Some("optimumdelay"),
        }
    }
}

/// The point on the rocket whose height or speed a metric takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Point {
    /// The centre of dry mass, the point RocketPy's state follows ([ADR-021][adr-021]).
    ///
    /// [adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
    CentreOfDryMass,
    /// The point the tool's own state tracks, which its documentation does not name. OpenRocket's
    /// is described only as "the position and velocity of a rocket", kept in world coordinates (Niskanen 2009,
    /// §4.2.1, p. 61); no probe here tells it from the centre of mass.
    Unstated,
}

/// An event in a flight that a metric is taken at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The rocket leaves the launch rod or rail.
    RodClearance,
    /// A recovery device deploys.
    Deployment,
    /// The rocket reaches the ground.
    GroundHit,
}

/// Which occurrence of an event that can happen more than once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Occurrence {
    /// The first time it happens.
    First,
    /// The last time it happens.
    Last,
}

/// What a metric measures, precisely enough to take it from a flight's time series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Definition {
    /// The largest height of `point` above the launch site over the flight.
    PeakHeight {
        /// Whose height.
        point: Point,
    },
    /// The largest speed of `point` over the flight.
    PeakSpeed {
        /// Whose speed.
        point: Point,
    },
    /// The speed of `point` at `which` occurrence of `event`, interpolated linearly in time
    /// between the two stored rows either side of it.
    SpeedAtEvent {
        /// Whose speed.
        point: Point,
        /// The event.
        event: Event,
        /// Which occurrence.
        which: Occurrence,
    },
    /// The static stability margin at the first occurrence of `event`: the distance from the
    /// centre of mass aft to the centre of pressure, divided by the reference length (the largest
    /// body diameter), in calibres (Niskanen 2009, p. 12).
    StabilityAtEvent {
        /// The event.
        event: Event,
    },
}

impl Definition {
    /// The event this definition takes its value at, if any.
    #[must_use]
    pub const fn event(self) -> Option<Event> {
        match self {
            Self::PeakHeight { .. } | Self::PeakSpeed { .. } => None,
            Self::SpeedAtEvent { event, .. } | Self::StabilityAtEvent { event } => Some(event),
        }
    }
}

/// The OpenRocket version whose summary words `flights.py` measured.
pub const OPENROCKET_MEASURED: &str = "24.12";

/// The RocketPy version whose metrics hpr's whole-flight cases compare against.
pub const ROCKETPY_MEASURED: &str = "1.13.0";

/// What `metric` measures in a reference from `tool`, or `None` where that has not been measured
/// for the tool's version.
///
/// - OpenRocket 24.12, measured by `flights.py` on all 57 flights of the public designs: the
///   apogee and largest speed are the peaks of its altitude and total-velocity columns; the rod
///   clearance, deployment and ground-hit speeds are its total velocity interpolated at that
///   event, and at the **last** deployment where there are two (17 flights tell them apart); the
///   stability margin is its stability column at rod clearance. Its optimum delay is not apogee
///   less burnout on 16 of the 57 flights, and what it is instead is not measured, so it has no
///   definition.
/// - RocketPy 1.13.0: the apogee, largest speed and rail-exit speed of hpr's whole-flight cases,
///   taken at the centre of dry mass ([ADR-021][adr-021]).
/// - Any other version: `None`.
///
/// [adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
#[must_use]
pub fn definition(tool: &Tool, metric: FlightMetric) -> Option<Definition> {
    match tool {
        Tool::OpenRocket { version } if version == OPENROCKET_MEASURED => {
            let point = Point::Unstated;
            let at = |event, which| Definition::SpeedAtEvent {
                point,
                event,
                which,
            };
            match metric {
                FlightMetric::Apogee => Some(Definition::PeakHeight { point }),
                FlightMetric::MaxSpeed => Some(Definition::PeakSpeed { point }),
                FlightMetric::RodClearanceSpeed => Some(at(Event::RodClearance, Occurrence::First)),
                FlightMetric::RodClearanceStability => Some(Definition::StabilityAtEvent {
                    event: Event::RodClearance,
                }),
                FlightMetric::DeploymentSpeed => Some(at(Event::Deployment, Occurrence::Last)),
                FlightMetric::GroundHitSpeed => Some(at(Event::GroundHit, Occurrence::First)),
                FlightMetric::OptimumDelay => None,
            }
        }
        Tool::RocketPy { version } if version == ROCKETPY_MEASURED => {
            let point = Point::CentreOfDryMass;
            match metric {
                FlightMetric::Apogee => Some(Definition::PeakHeight { point }),
                FlightMetric::MaxSpeed => Some(Definition::PeakSpeed { point }),
                FlightMetric::RodClearanceSpeed => Some(Definition::SpeedAtEvent {
                    point,
                    event: Event::RodClearance,
                    which: Occurrence::First,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Why a metric was not scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Withheld {
    /// What the metric means in the reference's tool and version has not been measured.
    DefinitionUnmeasured,
    /// The event the metric is taken at never happened in the reference's flight.
    NoEventInReference {
        /// The event.
        event: Event,
    },
    /// The event never happened in hpr's flight.
    NoEventInMeasured {
        /// The event.
        event: Event,
    },
    /// A peak with no value on one side, or a value that is not a finite number.
    NoValue,
}

/// A metric compared, or withheld with its reason. Never a zero standing in for a missing value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum MetricOutcome {
    /// Both sides have a value for the same defined quantity.
    Scored {
        /// The quantity both values are.
        definition: Definition,
        /// The reference's value.
        reference: f64,
        /// hpr's value.
        measured: f64,
    },
    /// Not compared, and why.
    Withheld(Withheld),
}

impl MetricOutcome {
    /// `measured − reference`, for a scored metric.
    #[must_use]
    pub fn difference(&self) -> Option<f64> {
        match *self {
            Self::Scored {
                reference,
                measured,
                ..
            } => Some(measured - reference),
            Self::Withheld(_) => None,
        }
    }
}

/// Compares hpr's `measured` value of `metric` with a `reference` value from `tool`.
///
/// A side's `None` or non-finite value means its flight has no value for the metric: for a metric
/// taken at an event, that the event never happened ([Loft lesson L81][l81]). Such a metric is
/// withheld with the event named, never scored against zero. A metric whose meaning in `tool`'s
/// version has not been measured is withheld too ([Loft lesson L80][l80]).
///
/// [l80]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l80
/// [l81]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l81
#[must_use]
pub fn compare(
    tool: &Tool,
    metric: FlightMetric,
    reference: Option<f64>,
    measured: Option<f64>,
) -> MetricOutcome {
    let Some(definition) = definition(tool, metric) else {
        return MetricOutcome::Withheld(Withheld::DefinitionUnmeasured);
    };
    let present = |value: Option<f64>| value.filter(|v| v.is_finite());
    match (present(reference), present(measured), definition.event()) {
        (Some(reference), Some(measured), _) => MetricOutcome::Scored {
            definition,
            reference,
            measured,
        },
        (None, _, Some(event)) => MetricOutcome::Withheld(Withheld::NoEventInReference { event }),
        (Some(_), None, Some(event)) => {
            MetricOutcome::Withheld(Withheld::NoEventInMeasured { event })
        }
        (_, _, None) => MetricOutcome::Withheld(Withheld::NoValue),
    }
}
