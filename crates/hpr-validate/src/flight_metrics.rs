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
//! definition below to it on every complete flight ([ADR-068][adr-068]).
//!
//! A metric taken at an event that did not happen, such as the deployment speed of a flight whose
//! parachute never opened, has no value. OpenRocket writes `NaN` for it. Loft scored it as 0
//! ([Loft lesson L81][l81]). [`compare`] withholds it when neither flight had the event, and fails
//! it when only one did; it is never scored.
//!
//! ```
//! use hpr_validate::flight_metrics::{
//!     Event, Failure, FlightMetric, MetricOutcome, ReferenceReading, Side, Tool, Withheld, compare,
//! };
//!
//! let openrocket = Tool::OpenRocket { version: "24.12".to_owned() };
//! // OpenRocket's Chute release example with a G40W-7: its last parachute opens at 14.2306 m/s.
//! let reading = ReferenceReading::Complete(Some(14.2306));
//! let outcome = compare(&openrocket, FlightMetric::DeploymentSpeed, reading, Some(14.0));
//! assert!((outcome.difference().unwrap() + 0.2306).abs() < 1e-9);
//!
//! // No parachute opened in OpenRocket's flight (it wrote `NaN`), nor in hpr's: withheld.
//! let none = ReferenceReading::Complete(Some(f64::NAN));
//! assert_eq!(
//!     compare(&openrocket, FlightMetric::DeploymentSpeed, none, None),
//!     MetricOutcome::Withheld(Withheld::NoEventInEither { event: Event::Deployment })
//! );
//! // Only hpr's opened: a disagreement, which fails rather than being scored against 0.
//! assert_eq!(
//!     compare(&openrocket, FlightMetric::DeploymentSpeed, none, Some(9.0)),
//!     MetricOutcome::Failed(Failure::EventOnlyIn { event: Event::Deployment, side: Side::Measured })
//! );
//! ```
//!
//! [m2-2d1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d1
//! [l80]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l80
//! [l81]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l81
//! [adr-068]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-068-openrockets-flights-of-the-public-designs-and-what-its-metric-words-mean-2026-09-25

use serde::{Deserialize, Serialize};

/// A tool that produced a reference flight, with its version as the tool writes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "tool")]
#[non_exhaustive]
pub enum Tool {
    /// OpenRocket, for example version `24.12`.
    #[serde(rename = "openrocket")]
    OpenRocket {
        /// The version, as in the `.ork`'s `creator` attribute after `OpenRocket `.
        version: String,
    },
    /// RocketPy, for example version `1.13.0`.
    #[serde(rename = "rocketpy")]
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
    /// The largest acceleration, m/s².
    MaxAcceleration,
    /// The largest Mach number.
    MaxMach,
    /// The time from launch to apogee, s.
    TimeToApogee,
    /// The time from launch to the end of the flight, s.
    FlightTime,
    /// The speed as the rocket leaves the launch rod or rail, m/s.
    RodClearanceSpeed,
    /// The static stability margin as the rocket leaves the launch rod or rail, calibres.
    RodClearanceStability,
    /// The speed at a recovery device's deployment, m/s.
    DeploymentSpeed,
    /// The speed at ground hit, m/s.
    GroundHitSpeed,
    /// The motor delay that would fire the ejection charge at apogee, s. OpenRocket 24.12's
    /// `optimumdelay` is not always apogee less burnout; see [`definition`].
    OptimumDelay,
}

impl FlightMetric {
    /// Every metric, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::Apogee,
        Self::MaxSpeed,
        Self::MaxAcceleration,
        Self::MaxMach,
        Self::TimeToApogee,
        Self::FlightTime,
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
            Self::MaxAcceleration => Some("maxacceleration"),
            Self::MaxMach => Some("maxmach"),
            Self::TimeToApogee => Some("timetoapogee"),
            Self::FlightTime => Some("flighttime"),
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
    /// is described only as "the position and velocity of a rocket", kept in world coordinates
    /// (Niskanen 2009, §4.2.1, p. 61); no probe here tells it from the centre of mass.
    Unstated,
}

/// An event in a flight that a metric is taken at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Event {
    /// The first integration step at which the rocket has travelled past the launch rod's
    /// length: OpenRocket 24.12's rod clearance. The rod's end falls between that step and the
    /// one before, so the speed here is above the speed at the rod's end (by 0.06% to 7.50% on
    /// the record's flights).
    RodClearance,
    /// The moment the rocket's travel equals the effective rail length, found between solver
    /// steps: RocketPy's rail exit ([ADR-021][adr-021]).
    ///
    /// [adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
    RailExit,
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
    /// The time of the stored step at which the height of `point` is largest. This is not
    /// always the apogee event's time: the event can fall between steps.
    TimeOfPeakHeight {
        /// Whose height.
        point: Point,
    },
    /// The largest speed of `point` over the flight.
    PeakSpeed {
        /// Whose speed.
        point: Point,
    },
    /// The largest Mach number over the flight.
    PeakMach,
    /// The largest total acceleration up to the first occurrence of `event`, or over the whole
    /// flight if it never happens: nothing after the event counts.
    PeakAccelerationBefore {
        /// The event.
        event: Event,
    },
    /// The speed of `point` at `which` occurrence of `event`, interpolated linearly in time
    /// between the two stored steps either side of it (exact when the event is on a step).
    SpeedAtEvent {
        /// Whose speed.
        point: Point,
        /// The event.
        event: Event,
        /// Which occurrence.
        which: Occurrence,
    },
    /// The time of the first occurrence of `event`.
    TimeOfEvent {
        /// The event.
        event: Event,
    },
    /// The static stability margin at the first occurrence of `event`: the distance from the
    /// centre of mass aft to the centre of pressure, divided by the tool's reference length, in
    /// calibres (Niskanen 2009, p. 12). OpenRocket's reference length is by default the largest
    /// body diameter, and was that on every flight of the record.
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
            Self::PeakHeight { .. }
            | Self::TimeOfPeakHeight { .. }
            | Self::PeakSpeed { .. }
            | Self::PeakMach
            | Self::PeakAccelerationBefore { .. } => None,
            Self::SpeedAtEvent { event, .. }
            | Self::TimeOfEvent { event }
            | Self::StabilityAtEvent { event } => Some(event),
        }
    }
}

/// The OpenRocket version whose summary words `flights.py` measured.
pub const OPENROCKET_MEASURED: &str = "24.12";

/// The RocketPy version whose metrics hpr's whole-flight cases compare against. Its definitions
/// come from [ADR-021][adr-021], which read what RocketPy computes, not from a probe like
/// `flights.py`.
///
/// [adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
pub const ROCKETPY_MEASURED: &str = "1.13.0";

/// What `metric` measures in a reference from `tool`, or `None` where that has not been measured
/// for the tool's version.
///
/// OpenRocket 24.12, measured by `flights.py` on the 56 complete flights of the public designs:
///
/// | metric | definition |
/// |---|---|
/// | apogee, largest speed, largest Mach | the peak of its altitude, total velocity or Mach column |
/// | largest acceleration | the peak of total acceleration before the first deployment |
/// | time to apogee | the time of the highest stored step, not the apogee event (13 flights differ) |
/// | flight time | the time of ground hit, its last step |
/// | rod clearance, ground-hit speed | total velocity at the event, which is on a step |
/// | deployment speed | total velocity interpolated in time between the steps either side |
/// | which deployment | the **last** (no flight has more than two) |
/// | stability margin | its stability column at rod clearance |
/// | optimum delay | none: apogee less burnout misses on 15 flights, and so does the same flight's with nothing deployed |
///
/// RocketPy 1.13.0: the apogee, largest speed and rail-exit speed of hpr's whole-flight cases,
/// taken at the centre of dry mass ([ADR-021][adr-021]). Any other version: `None`.
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
                FlightMetric::MaxMach => Some(Definition::PeakMach),
                FlightMetric::TimeToApogee => Some(Definition::TimeOfPeakHeight { point }),
                FlightMetric::FlightTime => Some(Definition::TimeOfEvent {
                    event: Event::GroundHit,
                }),
                FlightMetric::RodClearanceSpeed => Some(at(Event::RodClearance, Occurrence::First)),
                FlightMetric::RodClearanceStability => Some(Definition::StabilityAtEvent {
                    event: Event::RodClearance,
                }),
                FlightMetric::DeploymentSpeed => Some(at(Event::Deployment, Occurrence::Last)),
                FlightMetric::GroundHitSpeed => Some(at(Event::GroundHit, Occurrence::First)),
                FlightMetric::MaxAcceleration => Some(Definition::PeakAccelerationBefore {
                    event: Event::Deployment,
                }),
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
                    event: Event::RailExit,
                    which: Occurrence::First,
                }),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A reference flight's value for one metric.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "flight", content = "value", rename_all = "snake_case")]
pub enum ReferenceReading {
    /// The flight ran to its end. `None`, or OpenRocket's `NaN`, means it has no value: for a
    /// metric taken at an event, that the event never happened, which OpenRocket 24.12 writes as
    /// `NaN` exactly then.
    Complete(Option<f64>),
    /// The tool stopped the flight early (OpenRocket's `SIM_ABORT`), so even its peaks are where
    /// it stopped, not a flight's.
    Aborted,
}

/// One of the two flights compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// The reference's flight.
    Reference,
    /// hpr's flight.
    Measured,
}

/// Why a metric was not compared, when that is no fault of either flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Withheld {
    /// What the metric means in the reference's tool and version has not been measured.
    DefinitionUnmeasured,
    /// The reference's flight was aborted.
    ReferenceAborted,
    /// The event the metric is taken at happened in neither flight.
    NoEventInEither {
        /// The event.
        event: Event,
    },
}

/// Why a metric fails without being scored: the two flights disagree about what happened, or a
/// value is broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Failure {
    /// The event happened in one flight only.
    EventOnlyIn {
        /// The event.
        event: Event,
        /// The flight that had it.
        side: Side,
    },
    /// A value that is not a number: any `NaN` or infinity from hpr, or an infinity from the
    /// reference (whose `NaN` means its event did not happen).
    NotANumber {
        /// The side with the value.
        side: Side,
    },
    /// A metric that every complete flight has, such as the apogee, has no value on one side.
    NoValue {
        /// The side without one.
        side: Side,
    },
}

/// A metric compared, withheld, or failed. Never a zero standing in for a missing value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
#[non_exhaustive]
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
    /// Not scored, and a failure of the comparison.
    Failed(Failure),
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
            Self::Withheld(_) | Self::Failed(_) => None,
        }
    }
}

/// Compares hpr's `measured` value of `metric` with a `reference` reading from `tool`.
///
/// - A metric whose meaning in `tool`'s version is not measured is withheld ([Loft lesson
///   L80][l80]), and so is every metric of an aborted reference flight.
/// - A reference `None` or `NaN` means its flight has no value; hpr's `None` means the same, but
///   a `NaN` or infinity from hpr, or an infinity from the reference, is a failure.
/// - For a metric taken at an event, no value on both sides withholds it; on one side only, it
///   fails, since the flights disagree about what happened ([Loft lesson L81][l81]: never 0).
/// - For a metric every complete flight has, such as the apogee, a missing value fails.
///
/// `measured` must be taken by the [`Definition`] that [`definition`] gives for `tool`: the last
/// deployment, say, not the first.
///
/// [l80]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l80
/// [l81]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l81
#[must_use]
pub fn compare(
    tool: &Tool,
    metric: FlightMetric,
    reference: ReferenceReading,
    measured: Option<f64>,
) -> MetricOutcome {
    let Some(definition) = definition(tool, metric) else {
        return MetricOutcome::Withheld(Withheld::DefinitionUnmeasured);
    };
    let ReferenceReading::Complete(reference) = reference else {
        return MetricOutcome::Withheld(Withheld::ReferenceAborted);
    };
    if measured.is_some_and(|value| !value.is_finite()) {
        return MetricOutcome::Failed(Failure::NotANumber {
            side: Side::Measured,
        });
    }
    if reference.is_some_and(f64::is_infinite) {
        return MetricOutcome::Failed(Failure::NotANumber {
            side: Side::Reference,
        });
    }
    let reference = reference.filter(|value| value.is_finite());
    match (reference, measured, definition.event()) {
        (Some(reference), Some(measured), _) => MetricOutcome::Scored {
            definition,
            reference,
            measured,
        },
        (None, None, Some(event)) => MetricOutcome::Withheld(Withheld::NoEventInEither { event }),
        (None, Some(_), Some(event)) => MetricOutcome::Failed(Failure::EventOnlyIn {
            event,
            side: Side::Measured,
        }),
        (Some(_), None, Some(event)) => MetricOutcome::Failed(Failure::EventOnlyIn {
            event,
            side: Side::Reference,
        }),
        (None, _, None) => MetricOutcome::Failed(Failure::NoValue {
            side: Side::Reference,
        }),
        (Some(_), None, None) => MetricOutcome::Failed(Failure::NoValue {
            side: Side::Measured,
        }),
    }
}
