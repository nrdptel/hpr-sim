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
pub mod motors;
pub mod value;
mod warning;

pub use attached::ATTACHED_TAGS;
pub use component::{OPENROCKET_DEFAULT_RADIUS_M, rocket};
pub use container::{Attachment, Container, MAX_UNPACKED_BYTES, Unpacked};
pub use document::{Document, Element, MAX_DEPTH, MAX_KNOWN_MINOR, Node, SchemaVersion};
pub use error::OrkError;
pub use motors::{
    Curve, Ignition, IgnitionEvent, LeftOut, MotorConfiguration, Motors, NoCurve, NotFlown,
    OrkMotor, UnreadMotor,
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
pub struct Design {
    /// The rocket, as [`rocket`] reads it, with [`Rocket::configurations`] holding the
    /// configurations in [`Design::motors`] that were not left out.
    ///
    /// [`Rocket::configurations`]: hpr_design::Rocket::configurations
    pub rocket: hpr_design::Rocket,
    /// Every motor configuration in the file, flown or not.
    pub motors: Motors,
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
///
/// // ...and it ignites at launch, so the configuration is one the rocket flies.
/// let assembly = design.rocket.assemble("c1")?;
/// assert_eq!(assembly.motors[0].mount, "body");
/// # Ok(())
/// # }
/// ```
pub fn design(file: &OrkFile) -> Imported<Design> {
    let (rocket, mounts) = component::walk(&file.document);
    let Imported {
        value: mut rocket,
        mut warnings,
    } = rocket;
    let motors = match file.document.root.child("rocket") {
        Some(element) => motors::read(
            element,
            &mut rocket,
            &mounts,
            &file.attachments,
            &mut warnings,
        ),
        None => Motors::default(),
    };
    Imported {
        value: Design { rocket, motors },
        warnings,
    }
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
