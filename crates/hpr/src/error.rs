//! The facade's error type.

use hpr_aero::AeroError;
use hpr_atmos::AtmosError;
use hpr_core::CoreError;
use hpr_design::checks::Severity;
use hpr_design::{DesignError, Finding};
use hpr_motor::MotorError;
use hpr_sim::SimError;
use thiserror::Error;

/// Anything the builder API can refuse: its own checks, and every error of the crates it calls.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A number outside its domain, such as a negative length or a NaN, found as it was given.
    #[error("{what} is {value}, outside its domain")]
    Domain {
        /// What the number is.
        what: &'static str,
        /// The number.
        value: f64,
    },
    /// Parts added in an order the rocket can't be built in, such as fins before any body tube.
    #[error(transparent)]
    Order(Order),
    /// A material id that isn't built in; [`hpr_design::materials`] lists them.
    #[error("no built-in material `{0}` (`hpr_design::materials` lists them)")]
    UnknownMaterial(String),
    /// No motor with a thrust curve in the bundled catalog matches the name.
    #[error("no motor with a bundled thrust curve matches `{0}`")]
    NoSuchMotor(String),
    /// Several different motors match the name. Each is listed by its designation, which finds
    /// it alone, then its manufacturer: `I175WS (AeroTech)`.
    #[error("`{name}` matches several motors: {}", candidates.join(", "))]
    AmbiguousMotor {
        /// The name asked for.
        name: String,
        /// The motors it matches.
        candidates: Vec<String>,
    },
    /// A motor with an empty designation, which the design would take as a duplicate id.
    #[error("a motor needs a designation")]
    EmptyDesignation,
    /// A motor file holds no motor, or more than one where one was expected.
    #[error("the motor file holds {0} motors, not one")]
    MotorCount(usize),
    /// A flight of a rocket with no motor: give it one with [`crate::Rocket::set_motor`].
    #[error("the rocket has no motor; give it one with `Rocket::set_motor`")]
    NoMotor,
    /// A configuration id the design doesn't hold.
    #[error("the design has no configuration `{0}`")]
    NoSuchConfiguration(String),
    /// The design's checks ([`hpr_design::checks`]) found errors: every finding, errors and
    /// warnings, in the checks' order.
    #[error(
        "the design's checks found {} finding(s), errors among them; the first error is {}",
        .0.len(),
        first_error(.0)
    )]
    DesignChecks(Vec<Finding>),
    /// A catalogue part the builder can't make into the part asked for.
    #[error("catalogue part {part}: {problem}")]
    Catalog {
        /// The part: its maker and part number, and its file.
        part: String,
        /// What is wrong with it.
        problem: CatalogProblem,
    },
    /// From the design: its tree, parts and mass properties.
    #[error(transparent)]
    Design(#[from] DesignError),
    /// From the motor models and motor files.
    #[error(transparent)]
    Motor(#[from] MotorError),
    /// From the flight simulation.
    #[error(transparent)]
    Sim(#[from] SimError),
    /// From the aerodynamic model.
    #[error(transparent)]
    Aero(#[from] AeroError),
    /// From the atmosphere and wind models.
    #[error(transparent)]
    Atmos(#[from] AtmosError),
    /// From the geodesy: a launch site that isn't on the Earth.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// What is out of order when a part can't go where it was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum Order {
    /// A nose added after another body part: the nose goes first.
    #[error("the nose goes first, before any other body part")]
    NoseNotFirst,
    /// A transition with no body part before it to start from.
    #[error("a transition needs a body part before it")]
    NothingBeforeTransition,
    /// Fins, a motor tube or a mass with no body tube to go on.
    #[error("fins, a motor tube and masses go on a body tube: add one first")]
    NoTube,
    /// A second motor tube: a built rocket has one.
    #[error("the rocket has a motor tube already")]
    SecondMotorTube,
    /// A motor with no motor tube to go in.
    #[error("a motor needs a motor tube to go in")]
    NoMotorTube,
    /// A fitting holding a part that isn't one (one read from a file): a fitting is a coupler,
    /// a centering ring, a bulkhead, a launch lug, a parachute or a streamer.
    #[error("a fitting is a coupler, centering ring, bulkhead, launch lug, parachute or streamer")]
    NotAFitting,
    /// A part added to a rocket read from a design, which the builder doesn't change.
    #[error("parts can't be added to a rocket read from a design")]
    ReadFromDesign,
}

/// Why a catalogue part ([`hpr_io::orc::Part`]) can't be made into a builder part.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CatalogProblem {
    /// A part of a kind the builder's part isn't, such as a body tube given to
    /// `Nose::from_catalog`.
    #[error("it is a {found}, which `{builder}` doesn't make")]
    Kind {
        /// The part's kind.
        found: &'static str,
        /// The function it was given to.
        builder: &'static str,
    },
    /// A material its file names but doesn't define, so it has no density.
    #[error("its file doesn't define its material `{0}`")]
    UndefinedMaterial(String),
    /// A nose cone or transition neither filled nor given a wall thickness.
    #[error("it is neither filled nor given a wall thickness")]
    NoWall,
    /// A nose cone's or transition's shape the builder doesn't know.
    #[error("its shape is one the builder doesn't know")]
    Shape,
    /// A part that states its mass but whose sizes give it no volume to hold it, so no density
    /// gives it that mass.
    #[error("it states a mass, but its sizes give it no volume to hold it")]
    NoVolume,
}

/// `value` if it is finite and positive, [`Error::Domain`] otherwise.
pub(crate) fn positive(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}

/// `value` if it is finite and not negative, [`Error::Domain`] otherwise.
pub(crate) fn non_negative(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}

/// `value` if it is finite, [`Error::Domain`] otherwise.
pub(crate) fn finite(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}

/// The first finding that is an error, for [`Error::DesignChecks`]'s message.
fn first_error(findings: &[Finding]) -> String {
    findings
        .iter()
        .find(|finding| finding.severity() == Severity::Error)
        .map_or_else(|| "missing".to_owned(), |finding| format!("{finding:?}"))
}
