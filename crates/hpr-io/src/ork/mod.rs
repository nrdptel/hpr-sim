//! Reading OpenRocket `.ork` design files.
//!
//! **Guide:** [OpenRocket `.ork` design files][guide] says what this reads and what it leaves out.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/format/ork.html
//!
//! A `.ork` is a design: a tree of components, the materials they are made of, the motors flown in
//! them, and the simulations OpenRocket last ran. This module is the first step of reading one —
//! getting the design document out of whichever container it arrived in, and into a tree that
//! keeps everything the file said. [`rocket`] turns that tree into [`hpr_design`] types: the spine
//! — the stages and the body components stacked in them ([`component`]) — and the parts on and
//! inside each of those ([`attached`]).
//!
//! Nothing here reads a file: `hpr-io` does no I/O and builds for `wasm32-unknown-unknown`, so the
//! caller supplies the bytes.
//!
//! ```
//! # fn main() -> Result<(), hpr_io::ork::OrkError> {
//! let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
//! <openrocket version="1.10" creator="OpenRocket 24.12">
//!   <rocket><name>Sounder</name></rocket>
//! </openrocket>"#;
//!
//! let read = hpr_io::ork::read(xml)?;
//! assert_eq!(read.value.container, hpr_io::ork::Container::Xml);
//! assert_eq!(read.value.document.version.to_string(), "1.10");
//! let rocket = read.value.document.root.child("rocket").expect("a rocket");
//! assert_eq!(rocket.child("name").expect("a name").text(), "Sounder");
//! assert!(read.warnings.is_empty());
//! # Ok(())
//! # }
//! ```

pub mod attached;
pub mod component;
pub mod container;
pub mod document;
mod error;
pub mod extensions;
pub mod motors;
mod reads;
pub mod recovery;
pub mod simulations;
pub mod value;
mod warning;

pub use attached::ATTACHED_TAGS;
pub use component::{OPENROCKET_DEFAULT_RADIUS_M, rocket};
pub use container::{Attachment, Container, MAX_UNPACKED_BYTES, Unpacked};
pub use document::{Document, Element, MAX_DEPTH, MAX_KNOWN_MINOR, Node, SchemaVersion};
pub use error::OrkError;
pub use extensions::{Extensions, Kept, KeptAttribute, OpenRocketExtension, element_at};
pub use motors::{
    Curve, Ignition, IgnitionEvent, LeftOut, MotorConfiguration, Motors, NoCurve, NotFlown,
    OrkMotor, SuppliedCurves, UnreadMotor,
};
pub use recovery::{
    DeployEvent, Deployment, DeviceKind, EventSetting, Recovery, RecoveryDevice, Separation,
    SeparationEvent, StageSeparation, UnreadDevice,
};
pub use simulations::{
    Atmosphere, LaunchConditions, OPENROCKET_CALCULATOR, OPENROCKET_SIMULATOR, StoredBranch,
    StoredEvent, StoredReferenceExclusion, StoredResults, StoredSimulation, WindLevel,
};
pub use value::{AXIAL_OFFSET, Dimension, INSTANCE_COUNT, Overrides, Values};
pub use warning::{Imported, Warning, WarningKind};

/// A `.ork` file, read: the container it came in, its design document, and everything else it
/// carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrkFile {
    /// Which of the three containers the bytes were packed in.
    pub container: Container,
    /// The archive entry the design came from, when it came from an archive.
    pub design_entry: Option<String>,
    /// The design document.
    pub document: Document,
    /// The archive's other entries, in archive order: thrust curves, preview images, lookup
    /// tables. Held as stored, for the milestones that read them and for an export to put back.
    pub attachments: Vec<Attachment>,
}

impl OrkFile {
    /// The attachment called `name`, if the archive had one.
    pub fn attachment(&self, name: &str) -> Option<&Attachment> {
        self.attachments
            .iter()
            .find(|attachment| attachment.name == name)
    }
}

/// A `.ork` design read whole: its rocket, carrying every motor configuration hpr can fly as
/// written, and everything the file says about its motors.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Design {
    /// The rocket, as [`rocket`] reads it, with [`Rocket::configurations`] holding the
    /// configurations in [`Design::motors`] that were not left out.
    ///
    /// [`Rocket::configurations`]: hpr_design::Rocket::configurations
    pub rocket: hpr_design::Rocket,
    /// Every motor configuration in the file, flown or not.
    pub motors: Motors,
    /// When each parachute and streamer opens and each stage separates.
    pub recovery: Recovery,
    /// The simulations OpenRocket last ran on the design, with their conditions and results.
    pub simulations: Vec<StoredSimulation>,
    /// What the file holds that hpr does not model, kept whole for an export to put back.
    #[serde(default)]
    pub extensions: Extensions,
}

impl Design {
    /// Whether the rocket is reduced: the file describes parts of it — a pod, a parallel stage, a
    /// part hpr cannot shape — that are kept in [`Design::extensions`] rather than read into it.
    ///
    /// The flag is on the `Design`, not on [`Design::rocket`]: check it before using the rocket on
    /// its own. A part read as something simpler, such as a cluster of tubes read as one, does not
    /// make a design reduced; its warning keeps every configuration of it from flying
    /// ([ADR-055][adr-055], the rule for which configurations fly).
    ///
    /// [adr-055]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-055-m31c-split-and-the-motors-a-ork-flies-its-own-curve-first-and-only-what-lights-at-launch-2026-09-21
    pub fn is_reduced(&self) -> bool {
        !self.extensions.x_openrocket.parts.is_empty()
    }

    /// Returns why `simulation` cannot be used as a reference for this design.
    ///
    /// This combines the stored-result screen with hpr's design-reproduction screen. Use
    /// [`StoredSimulation::reference_exclusion`] and [`Self::reproduction_exclusion`] separately
    /// when those two questions need to be reported independently.
    #[must_use]
    pub fn reference_exclusion(
        &self,
        simulation: &StoredSimulation,
    ) -> Option<StoredReferenceExclusion> {
        simulation
            .reference_exclusion()
            .or_else(|| self.reproduction_exclusion(simulation))
    }

    /// Returns why hpr cannot reproduce the design named by a stored launch configuration.
    ///
    /// This is deliberately separate from [`StoredSimulation::reference_exclusion`]: a complete
    /// OpenRocket result can be a useful stored reference even when hpr does not yet have its
    /// motor curve or full airframe model. A caller that needs a result hpr can fly should apply
    /// both classifiers.
    #[must_use]
    pub fn reproduction_exclusion(
        &self,
        simulation: &StoredSimulation,
    ) -> Option<StoredReferenceExclusion> {
        if self.is_reduced() {
            return Some(StoredReferenceExclusion::ReducedDesign);
        }
        let Some(conditions) = simulation.conditions.as_ref() else {
            return Some(StoredReferenceExclusion::MissingConfiguration);
        };
        let Some(configuration_id) = conditions.configuration.as_deref() else {
            return Some(StoredReferenceExclusion::MissingConfiguration);
        };
        let Some(configuration) = self
            .motors
            .configurations
            .iter()
            .find(|configuration| configuration.id == configuration_id)
        else {
            return Some(StoredReferenceExclusion::UnknownConfiguration);
        };
        if configuration.left_out.is_some()
            || self.rocket.configuration(configuration_id).is_none()
            || self.rocket.assemble(configuration_id).is_err()
        {
            return Some(StoredReferenceExclusion::UnflyableConfiguration);
        }
        None
    }
}

/// Reads the design in a `.ork` file: the rocket ([`rocket`]) and its motors ([`motors`]), with
/// thrust curves from the archive's `thrustcurves/*.rse` entries or the bundled catalog.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
/// <openrocket version="1.10" creator="OpenRocket 24.12">
///   <rocket><name>Sounder</name>
///     <motorconfiguration configid="c1" default="true"><name>F15</name></motorconfiguration>
///     <subcomponents><stage><name>Sustainer</name><subcomponents>
///       <nosecone><name>Nose</name><id>nose</id>
///         <material type="bulk" density="1000.0">Plastic</material>
///         <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
///         <aftradius>0.0165</aftradius></nosecone>
///       <bodytube><name>Body</name><id>body</id>
///         <material type="bulk" density="680.0">Cardboard</material>
///         <length>0.6</length><thickness>0.001</thickness><radius>0.0165</radius>
///         <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
///           <overhang>0.005</overhang>
///           <motor configid="c1"><type>single</type><manufacturer>Estes</manufacturer>
///             <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
///             <delay>4.0</delay></motor>
///         </motormount></bodytube>
///     </subcomponents></stage></subcomponents></rocket>
/// </openrocket>"#;
///
/// let read = hpr_io::ork::read(xml)?;
/// let design = hpr_io::ork::design(&read.value).value;
///
/// // The F15 has no curve in this file, so it comes from the bundled catalog...
/// let motor = &design.motors.configurations[0].motors[0];
/// assert!(matches!(motor.curve, hpr_io::ork::Curve::Catalog { .. }));
/// assert_eq!(motor.delay, Some(hpr_motor::Delay::Seconds(4.0)));
/// let impulse_ns = motor.curve.motor().expect("a curve").curve().total_impulse_ns();
/// assert!((impulse_ns - 49.61).abs() < 0.01, "{impulse_ns}");
///
/// // ...and it ignites at launch, so the configuration is one the rocket flies.
/// let assembly = design.rocket.assemble("c1")?;
/// assert_eq!(assembly.motors[0].mount, "body");
/// # Ok(())
/// # }
/// ```
pub fn design(file: &OrkFile) -> Imported<Design> {
    design_with(file, &SuppliedCurves::default())
}

/// Reads the design in `file` as [`design`] does, with `supplied` curves for the motors whose
/// digest they name and whose archive embeds no usable curve: an embedded curve first, then a
/// supplied one, then the bundled catalog.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use hpr_io::ork::{Curve, SuppliedCurves};
/// use hpr_motor::{SolidMotor, ThrustCurve};
///
/// let xml = br#"<?xml version='1.0' encoding='utf-8'?>
/// <openrocket version="1.9" creator="example">
///   <rocket><subcomponents><stage><name>Sustainer</name><subcomponents>
///     <bodytube><id>body</id><length>0.4</length><thickness>0.001</thickness>
///       <radius>0.0125</radius>
///       <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
///         <overhang>0.0</overhang>
///         <motor configid="c1"><type>single</type><manufacturer>Example</manufacturer>
///           <digest>0123abcd</digest><designation>E9</designation>
///           <diameter>0.024</diameter><length>0.07</length><delay>4.0</delay></motor>
///       </motormount></bodytube>
///   </subcomponents></stage></subcomponents></rocket>
/// </openrocket>"#;
/// let read = hpr_io::ork::read(xml)?;
///
/// // A made-up 20 N·s motor: 10 N for two seconds.
/// let thrust = ThrustCurve::new(vec![0.0, 0.1, 1.9, 2.0], vec![0.0, 10.0, 10.0, 0.0])?;
/// let motor = SolidMotor::from_envelope(thrust, 0.024, 0.07, 0.01, 0.03)?;
/// let mut supplied = SuppliedCurves::new("a motor database");
/// supplied.insert("0123abcd", 0.024, 0.07, motor);
///
/// let design = hpr_io::ork::design_with(&read.value, &supplied).value;
/// let flown = &design.motors.configurations[0].motors[0];
/// assert!(matches!(flown.curve, Curve::Supplied { .. }));
/// assert!(design.rocket.assemble("c1").is_ok());
/// # Ok(())
/// # }
/// ```
pub fn design_with(file: &OrkFile, supplied: &SuppliedCurves) -> Imported<Design> {
    // Every tag the readers ask for is recorded, so that the extension can keep the rest.
    let ((mut design, warnings, read), reads) = reads::recording(|| {
        let (mut design, mut warnings, read) = read_design(file, supplied);
        // Stored simulations stand apart from the airframe, so their warnings come after the check
        // in `read_design`; a document can hold them without a design, as Debrief's results-only
        // file does.
        design.simulations = simulations::read(&file.document, &mut warnings);
        (design, warnings, read)
    });
    design.extensions = Extensions {
        x_openrocket: extensions::read(&file.document, &read, &reads),
    };
    Imported {
        value: design,
        warnings,
    }
}

/// The rocket, its motors and its recovery, the warnings reading them raised, and the paths of
/// the stages and components read.
fn read_design(
    file: &OrkFile,
    supplied: &SuppliedCurves,
) -> (Design, Vec<Warning>, std::collections::BTreeSet<String>) {
    let (rocket, walked) = component::walk(&file.document);
    let Imported {
        value: rocket,
        mut warnings,
    } = rocket;
    // Any warning the walk raised about the rocket or a motor mount means it was not read exactly
    // as written: a part left out, a value dropped or simplified, or something assumed. No
    // configuration of such a rocket is flown (ADR-055). A recovery device's or a stage
    // separation's own warnings are about when things happen, which is not flown (ADR-056), and
    // their paths say so.
    let skipped: Vec<&str> = warnings
        .iter()
        .filter(|w| {
            ![recovery::DEPLOYMENT, recovery::DRAG, recovery::SEPARATION]
                .iter()
                .any(|segment| w.at.contains(segment))
        })
        .map(|w| w.message.as_str())
        .collect();
    let incomplete = match skipped.as_slice() {
        [] => None,
        [one] => Some((*one).to_owned()),
        [first, rest @ ..] => Some(format!("{first}, and {} more", rest.len())),
    };
    let mut design = Design {
        rocket,
        motors: Motors::default(),
        recovery: Recovery::default(),
        simulations: Vec::new(),
        extensions: Extensions::default(),
    };
    if let Some(element) = file.document.root.child("rocket") {
        design.motors = motors::read(
            element,
            &mut design.rocket,
            incomplete.as_deref(),
            &walked.mounts,
            &file.attachments,
            supplied,
            &mut warnings,
        );
        design.recovery =
            recovery::read(element, &design.rocket, walked.devices, walked.separations);
    }
    (design, warnings, walked.read)
}

/// Reads a `.ork` file from its bytes: sniffs the container, unpacks it, and parses the design.
///
/// # Errors
///
/// Returns [`OrkError`] when the bytes are not a `.ork` at all, when the container is damaged,
/// when no entry in it can be the design document, or when that document is not well-formed XML
/// rooted at `<openrocket>` with a readable schema version. Everything a reader can go on from is
/// a [`Warning`] instead.
pub fn read(bytes: &[u8]) -> Result<Imported<OrkFile>, OrkError> {
    let unpacked = container::unpack(bytes)?;
    let mut warnings = unpacked.warnings;
    let Unpacked {
        container,
        design_entry,
        design,
        attachments,
    } = unpacked.value;
    let document = Document::parse(&design)?;
    warnings.extend(document.warnings);
    let document = document.value;
    Ok(Imported {
        value: OrkFile {
            container,
            design_entry,
            document,
            attachments,
        },
        warnings,
    })
}

#[cfg(test)]
mod tests;
