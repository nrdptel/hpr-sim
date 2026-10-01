//! OpenRocket's `.orc` parts catalogues: nose cones, tubes, rings, transitions, parachutes and
//! streamers as their makers sell them, by maker and part number.
//!
//! A `.orc` file is XML: an `<OpenRocketComponent>` root holding a `<Version>`, a `<Materials>`
//! list (each a name, a density, the density's units and its kind: `BULK`, `SURFACE` or `LINE`)
//! and a `<Components>` list of parts. A part names its maker, part number and description, its
//! dimensions each with its own `Unit` attribute, and its material by name and kind; the material
//! must be defined in the same file. The format has no published schema; its field list and units
//! are the ones the `openrocket-database` project documents (`docs/TechnicalInfo.md` at the pinned
//! commit), and what each field means is what OpenRocket 24.12's preset loader makes of it,
//! measured by running that loader on the same files as an oracle
//! (`validation/oracles/openrocket/orc_presets.py`).
//!
//! [`read`] reads one file into a [`Catalog`]. [`bundled`] is the catalogue OpenRocket 24.12
//! ships: the 16 files of `openrocket-database` (Apache-2.0) at the commit
//! `validation/refs.lock.toml` pins, compiled into this crate unchanged, 3,449 parts.
//! [`Catalog::find`] looks a part up by maker and part number.
//!
//! **What is read, and how far it agrees with OpenRocket.** Every value is converted to SI as the
//! file states it: lengths in metres, masses in kilograms, densities in kg/m³, kg/m² or kg/m. On
//! the bundled files, every part OpenRocket reads is read here, in the same order, and every
//! value equals OpenRocket's bit for bit (17,911 of 18,306 numbers), with four exceptions that the
//! test `crates/hpr-io/tests/orc_openrocket.rs` counts:
//!
//! - **Ounces.** OpenRocket's ounce is 0.0283495231 kg; the avoirdupois ounce is exactly
//!   0.028349523125 kg (NIST Handbook 44, Appendix C), and that is the one used here. The
//!   difference is 9e-10 of the value, on the 185 masses stated in ounces. OpenRocket rounds a
//!   few other imperial factors the same way (`lb/ft³`, `oz/in²`, `oz/ft²`, `lb/ft²`, `oz/ft`);
//!   no bundled file uses them.
//! - **Makers' names.** OpenRocket shows two makers under names of its own: "LOC Precision" as
//!   "LOC/Precision" and "Public Missiles" as "Public Missiles, Ltd." The name here is the file's.
//! - **A stated mass.** When a solid part states its mass, OpenRocket replaces its material's
//!   density with the one that gives the part that mass. Here the material keeps the file's
//!   density and the part keeps its mass ([`Part::mass_kg`]), which a design built from the part
//!   by hand can set as its mass. This is 207 parts; with the masses taken out of the files,
//!   OpenRocket's density is the one read here on every one of them. A parachute's or streamer's
//!   stated mass leaves its fabric's density alone, here and in OpenRocket.
//! - **Undefined materials.** A material the file names but doesn't define has no density here
//!   ([`MaterialRef::density`] is `None`); OpenRocket gives it zero. This is 3 parts.
//!
//! **What the file leaves unsaid.** No `.orc` field gives a nose or transition shape's parameter
//! (an ogive's radius, a Haack series' `C`), a shoulder's wall, or a parachute's drag coefficient;
//! OpenRocket uses its own defaults for these when a part is put in a design. A part's material
//! is only as good as the file's density, and the database's own README warns to weigh real parts.
//!
//! **Warnings.** A part or material that can't be read is left out with a [`Warning`] saying why,
//! rather than failing the file; so are fields ignored or repeated, and values read as written but
//! implausible (a wall thicker than its tube, a fabric under 1 g/m²). The bundled files give 52.

mod bundled;
#[cfg(test)]
mod tests;

pub use bundled::{BUNDLED_FILES, bundled};

use serde::{Deserialize, Serialize};

/// The most characters of a file's text a warning or an error quotes.
const QUOTE_LIMIT: usize = 64;

/// A `.orc` file that can't be read at all.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OrcError {
    /// The elements nest deeper than [`MAX_DEPTH`]. Checked before the XML is parsed, as the
    /// parser descends the tree and a file nested deep enough would exhaust the stack.
    #[error("the elements nest {depth} deep, past the {limit} this reads")]
    TooDeep {
        /// [`MAX_DEPTH`].
        limit: usize,
        /// The nesting found, or more.
        depth: usize,
    },
    /// The text is not well-formed XML.
    #[error("not well-formed XML: {reason}")]
    Xml {
        /// The XML parser's message.
        reason: String,
    },
    /// The root element is not `<OpenRocketComponent>`.
    #[error("the root element is <{root}>, not <OpenRocketComponent>: not a parts catalogue")]
    NotACatalog {
        /// The root element's name, shortened.
        root: String,
    },
}

/// The deepest nesting read. A `.orc` nests four deep (the root, the parts list, a part, its
/// field); the parser below has been seen to exhaust a 2 MiB stack at 130 levels (the `.ork`
/// reader's measurement, [`crate::ork::MAX_DEPTH`]).
pub const MAX_DEPTH: usize = 16;

/// The most warnings one file gives; past it, one last warning says how many more there were.
pub const MAX_WARNINGS: usize = 1000;

/// What a [`Warning`] is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WarningKind {
    /// A part was left out: a size missing or unreadable, a unit or shape the reader doesn't
    /// take, a material of the wrong kind, or an element that isn't a kind of part.
    PartLeftOut,
    /// A material was left out: an unknown unit or kind, or no density.
    MaterialLeftOut,
    /// A part names a material its file doesn't define; it has no density.
    MaterialUndefined,
    /// A file defines two materials of the same name and kind with different densities; parts
    /// take the first.
    MaterialRepeated,
    /// Fields a part's kind doesn't have, or elements beside the catalogue's lists, were ignored.
    Ignored,
    /// A field was stated more than once; the last was read.
    Repeated,
    /// A value read as written that can't be right: a wall of no thickness or less, or a fabric
    /// lighter than any made.
    Implausible,
    /// [`MAX_WARNINGS`] were given; the rest are counted, not listed.
    TooMany,
}

/// Something read differently from what the file says, or left out. Reading goes on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Warning {
    /// Where: the part's maker and number, `Materials: ` and a material's name, or the file.
    pub at: String,
    /// What kind of warning.
    pub kind: WarningKind,
    /// What happened, in a sentence.
    pub message: String,
}

/// A catalogue file read, with what was left out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CatalogFile {
    /// The parts read, in the file's order.
    pub catalog: Catalog,
    /// The file's `<Version>`, as written; `None` if it has none.
    pub version: Option<String>,
    /// The materials the file defines, in its order.
    pub materials: Vec<CatalogMaterial>,
    /// Parts left out and values read differently, one entry each, at most [`MAX_WARNINGS`] and
    /// one more.
    pub warnings: Vec<Warning>,
}

/// Parts, from one catalogue file or several.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    /// Every part, in file order.
    pub parts: Vec<Part>,
}

/// One part as its catalogue lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Part {
    /// The catalogue file it came from, such as `estes_classic.orc`.
    pub file: String,
    /// The maker, as the file writes it.
    pub manufacturer: String,
    /// The maker's part number, as the file writes it.
    pub part_number: String,
    /// The file's description; empty if it has none. A part that states two keeps the last, as
    /// OpenRocket does.
    pub description: String,
    /// The part's mass as the file states it, kg; `None` for most parts, whose mass follows from
    /// their size and material.
    pub mass_kg: Option<f64>,
    /// What kind of part, with its dimensions.
    pub kind: PartKind,
}

/// A part's kind and dimensions.
///
/// The five tube-like kinds share [`Tube`]'s fields, as the format gives them: an inner and
/// outer diameter and a length (a centering ring's length is its thickness).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum PartKind {
    /// An airframe tube (`<BodyTube>`).
    BodyTube(Tube),
    /// A coupler that joins two tubes (`<TubeCoupler>`); an inner diameter of zero makes it a
    /// solid nose block.
    TubeCoupler(Tube),
    /// A ring that stops the motor moving forward (`<EngineBlock>`).
    EngineBlock(Tube),
    /// A ring that holds an inner tube in an outer one (`<CenteringRing>`).
    CenteringRing(Tube),
    /// A launch lug (`<LaunchLug>`).
    LaunchLug(Tube),
    /// A solid disc (`<BulkHead>`).
    Bulkhead(Bulkhead),
    /// A nose cone (`<NoseCone>`).
    NoseCone(NoseCone),
    /// A transition between two diameters (`<Transition>`).
    Transition(Transition),
    /// A parachute (`<Parachute>`).
    Parachute(Parachute),
    /// A streamer (`<Streamer>`).
    Streamer(Streamer),
}

/// A tube-like part: a body tube, coupler, engine block, centering ring or launch lug.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tube {
    /// Inner diameter, m.
    pub inner_diameter_m: f64,
    /// Outer diameter, m.
    pub outer_diameter_m: f64,
    /// Length, m.
    pub length_m: f64,
    /// Material (bulk).
    pub material: MaterialRef,
}

impl Tube {
    /// The wall's thickness, m: `(outer diameter − inner diameter)/2`, as OpenRocket works it out.
    #[must_use]
    pub fn thickness_m(&self) -> f64 {
        (self.outer_diameter_m - self.inner_diameter_m) / 2.0
    }
}

/// A bulkhead: a solid disc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Bulkhead {
    /// Outer diameter, m.
    pub outer_diameter_m: f64,
    /// Length (thickness), m.
    pub length_m: f64,
    /// `<Filled>` as written; `None` if absent. A bulkhead is solid either way.
    pub filled: Option<bool>,
    /// Material (bulk).
    pub material: MaterialRef,
}

/// A nose cone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NoseCone {
    /// Profile shape. The file gives no shape parameter.
    pub shape: Shape,
    /// Length from tip to base, not counting the shoulder, m.
    pub length_m: f64,
    /// Base diameter, m.
    pub outer_diameter_m: f64,
    /// Shoulder diameter, m; zero for no shoulder.
    pub shoulder_diameter_m: f64,
    /// Shoulder length, m; zero for no shoulder.
    pub shoulder_length_m: f64,
    /// `<Filled>` as written; `None` if absent.
    pub filled: Option<bool>,
    /// Wall thickness, m; `None` if absent.
    pub thickness_m: Option<f64>,
    /// Material (bulk).
    pub material: MaterialRef,
}

/// A transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Transition {
    /// Profile shape. The file gives no shape parameter.
    pub shape: Shape,
    /// Length, not counting shoulders, m.
    pub length_m: f64,
    /// Forward outer diameter, m.
    pub fore_outer_diameter_m: f64,
    /// Forward shoulder diameter, m; zero for none.
    pub fore_shoulder_diameter_m: f64,
    /// Forward shoulder length, m; zero for none.
    pub fore_shoulder_length_m: f64,
    /// Aft outer diameter, m.
    pub aft_outer_diameter_m: f64,
    /// Aft shoulder diameter, m; zero for none.
    pub aft_shoulder_diameter_m: f64,
    /// Aft shoulder length, m; zero for none.
    pub aft_shoulder_length_m: f64,
    /// `<Filled>` as written; `None` if absent.
    pub filled: Option<bool>,
    /// Wall thickness, m; `None` if absent.
    pub thickness_m: Option<f64>,
    /// Material (bulk).
    pub material: MaterialRef,
}

/// A parachute.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Parachute {
    /// Canopy diameter, m.
    pub diameter_m: f64,
    /// The canopy's number of sides.
    pub sides: u32,
    /// Number of shroud lines.
    pub line_count: u32,
    /// Length of each shroud line, m.
    pub line_length_m: f64,
    /// Canopy fabric (surface).
    pub material: MaterialRef,
    /// Shroud-line cord (line); `None` if the file names none.
    pub line_material: Option<MaterialRef>,
}

/// A streamer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Streamer {
    /// Length, m.
    pub length_m: f64,
    /// Width, m.
    pub width_m: f64,
    /// Thickness, m. The database notes OpenRocket may make no use of it.
    pub thickness_m: f64,
    /// Material (surface).
    pub material: MaterialRef,
}

/// A nose cone's or transition's profile, as the file names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Shape {
    /// `CONICAL`.
    Conical,
    /// `OGIVE`.
    Ogive,
    /// `ELLIPSOID`.
    Ellipsoid,
    /// `PARABOLIC`: a parabolic series.
    Parabolic,
    /// `HAACK`: a Haack series.
    Haack,
    /// `POWER`: a power series.
    Power,
}

impl Shape {
    /// The shape named by `word`, as the format spells it.
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "CONICAL" => Self::Conical,
            "OGIVE" => Self::Ogive,
            "ELLIPSOID" => Self::Ellipsoid,
            "PARABOLIC" => Self::Parabolic,
            "HAACK" => Self::Haack,
            "POWER" => Self::Power,
            _ => return None,
        })
    }
}

/// The kind of density a material has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MaterialKind {
    /// Mass per volume, kg/m³: anything solid.
    Bulk,
    /// Mass per area, kg/m²: canopy and streamer fabric.
    Surface,
    /// Mass per length, kg/m: shroud lines.
    Line,
}

impl MaterialKind {
    /// The kind named by `word`, as the format spells it.
    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "BULK" => Self::Bulk,
            "SURFACE" => Self::Surface,
            "LINE" => Self::Line,
            _ => return None,
        })
    }
}

/// A material a catalogue file defines.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CatalogMaterial {
    /// Its name.
    pub name: String,
    /// Its kind.
    pub kind: MaterialKind,
    /// Its density in SI units of its kind: kg/m³, kg/m² or kg/m.
    pub density: f64,
}

/// The material a part names, with the density its file defines for it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MaterialRef {
    /// The name the part gives.
    pub name: String,
    /// The kind the part gives.
    pub kind: MaterialKind,
    /// The density in SI units of its kind, from the first material of that name and kind the
    /// file defines; `None` if it defines none.
    pub density: Option<f64>,
}

impl MaterialRef {
    /// The material as `hpr_design` holds it, or `None` if the file doesn't define it.
    #[must_use]
    pub fn material(&self) -> Option<hpr_design::Material> {
        let density = self.density?;
        Some(match self.kind {
            MaterialKind::Bulk => hpr_design::Material::bulk(self.name.clone(), density),
            MaterialKind::Surface => hpr_design::Material::surface(self.name.clone(), density),
            MaterialKind::Line => hpr_design::Material::line(self.name.clone(), density),
        })
    }
}

impl Catalog {
    /// Every part whose maker is `manufacturer` and whose part number is `part_number`, in
    /// catalogue order. Both are compared with surrounding spaces trimmed, the maker in any case
    /// and the part number exactly. The maker is the file's name for it, not OpenRocket's display
    /// name ("LOC Precision", not "LOC/Precision"). Most numbers name one part; in the bundled
    /// catalogue, 21 numbers name two parts of the same kind, and 3 of those pairs are identical.
    #[must_use]
    pub fn find(&self, manufacturer: &str, part_number: &str) -> Vec<&Part> {
        let manufacturer = manufacturer.trim().to_lowercase();
        let part_number = part_number.trim();
        self.parts
            .iter()
            .filter(|part| {
                part.manufacturer.trim().to_lowercase() == manufacturer
                    && part.part_number.trim() == part_number
            })
            .collect()
    }

    /// Every part whose part number or description holds `text`, in any case, from the maker
    /// `manufacturer` (in any case) or from any maker if `None`, in catalogue order; empty
    /// `text` matches every part. Part numbers are often several numbers in one, such as Estes'
    /// `BT-20, 30316`, which `find` matches only whole; a search for `30316` finds it.
    #[must_use]
    pub fn search(&self, manufacturer: Option<&str>, text: &str) -> Vec<&Part> {
        let text = text.trim().to_lowercase();
        let manufacturer = manufacturer.map(|maker| maker.trim().to_lowercase());
        self.parts
            .iter()
            .filter(|part| {
                manufacturer
                    .as_ref()
                    .is_none_or(|maker| part.manufacturer.trim().to_lowercase() == *maker)
                    && (part.part_number.to_lowercase().contains(&text)
                        || part.description.to_lowercase().contains(&text))
            })
            .collect()
    }

    /// The makers, each once, in the order they first appear.
    #[must_use]
    pub fn manufacturers(&self) -> Vec<&str> {
        let mut seen = std::collections::BTreeSet::new();
        self.parts
            .iter()
            .map(|part| part.manufacturer.as_str())
            .filter(|name| seen.insert(*name))
            .collect()
    }

    /// Adds another catalogue's parts after this one's.
    pub fn extend(&mut self, other: Catalog) {
        self.parts.extend(other.parts);
    }
}

/// Reads one `.orc` file's text; `file` names it in each [`Part::file`].
///
/// Only text nested past [`MAX_DEPTH`], text that isn't XML, or a root that isn't
/// `<OpenRocketComponent>` is refused. A part this can't read (a missing or unreadable size, a
/// unit or shape it doesn't take, a material of the wrong kind, an element that isn't a kind of
/// part) is left out with a warning, where OpenRocket 24.12 refuses the whole file for most of
/// these. Fields a part's kind doesn't have are ignored with a warning, as are a second
/// `<Materials>` or `<Components>` list and anything else beside them.
///
/// # Errors
///
/// [`OrcError::TooDeep`], [`OrcError::Xml`] and [`OrcError::NotACatalog`].
pub fn read(text: &str, file: &str) -> Result<CatalogFile, OrcError> {
    let depth = crate::ork::document::deepest_nesting(text);
    if depth > MAX_DEPTH {
        return Err(OrcError::TooDeep {
            limit: MAX_DEPTH,
            depth,
        });
    }
    let document = roxmltree::Document::parse(text).map_err(|error| OrcError::Xml {
        reason: error.to_string(),
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "OpenRocketComponent" {
        return Err(OrcError::NotACatalog {
            root: quote(root.tag_name().name()),
        });
    }
    let mut warnings = Warnings::default();
    let mut ignored: Vec<String> = Vec::new();
    let mut lists = (0, 0, 0);
    for element in root.children().filter(roxmltree::Node::is_element) {
        let name = element.tag_name().name();
        let count = match name {
            "Version" => &mut lists.0,
            "Materials" => &mut lists.1,
            "Components" => &mut lists.2,
            _ => {
                ignored.push(format!("<{}>", quote(name)));
                continue;
            }
        };
        *count += 1;
        if *count == 2 {
            ignored.push(format!("a second <{name}>"));
        }
    }
    if !ignored.is_empty() {
        warnings.push(
            file,
            WarningKind::Ignored,
            format!(
                "{} beside the catalogue's lists {} ignored",
                listed(&ignored),
                if ignored.len() == 1 { "was" } else { "were" }
            ),
        );
    }
    let version = child(root, "Version").map(text_of);
    let materials = child(root, "Materials")
        .map(|list| read_materials(list, &mut warnings))
        .unwrap_or_default();
    let mut parts = Vec::new();
    if let Some(list) = child(root, "Components") {
        for element in list.children().filter(roxmltree::Node::is_element) {
            if let Some(part) = read_part(element, file, &materials, &mut warnings) {
                parts.push(part);
            }
        }
    }
    Ok(CatalogFile {
        catalog: Catalog { parts },
        version,
        materials,
        warnings: warnings.finish(file),
    })
}

/// A file's warnings, up to [`MAX_WARNINGS`], and a count of the rest.
#[derive(Default)]
struct Warnings {
    list: Vec<Warning>,
    more: usize,
}

impl Warnings {
    fn push(&mut self, at: &str, kind: WarningKind, message: String) {
        if self.list.len() < MAX_WARNINGS {
            self.list.push(Warning {
                at: at.to_owned(),
                kind,
                message,
            });
        } else {
            self.more += 1;
        }
    }

    fn finish(mut self, file: &str) -> Vec<Warning> {
        if self.more > 0 {
            self.list.push(Warning {
                at: file.to_owned(),
                kind: WarningKind::TooMany,
                message: format!("{} more warnings were not listed", self.more),
            });
        }
        self.list
    }
}

/// Up to the first five of `items`, joined, and how many more.
fn listed(items: &[String]) -> String {
    let shown = items.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    match items.len() {
        0..=5 => shown,
        n => format!("{shown} and {} more", n - 5),
    }
}

/// The first child element named `name`.
fn child<'a, 'input>(
    node: roxmltree::Node<'a, 'input>,
    name: &str,
) -> Option<roxmltree::Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

/// An element's text, every text node joined, as written.
fn text_of(node: roxmltree::Node<'_, '_>) -> String {
    node.children()
        .filter(roxmltree::Node::is_text)
        .filter_map(|text| text.text())
        .collect()
}

/// A value's text, or why it has none: an element inside a value is not part of the format.
fn value_text(node: roxmltree::Node<'_, '_>, name: &str) -> Result<String, String> {
    if node.children().any(|child| child.is_element()) {
        Err(format!("<{name}> holds an element, not a value"))
    } else {
        Ok(text_of(node))
    }
}

/// `text` cut to [`QUOTE_LIMIT`] characters, marked when cut.
fn quote(text: &str) -> String {
    match text.char_indices().nth(QUOTE_LIMIT) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_owned(),
    }
}

/// A number as the format writes one: a finite decimal, spaces around it allowed.
fn number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

/// The SI factor of a length unit. These are the length units OpenRocket 24.12 reads in a
/// `.orc` (`docs/TechnicalInfo.md`'s list less `in/64`, which OpenRocket reads as inches); each
/// factor is its exact definition (NIST Handbook 44, Appendix C: 1 in = 0.0254 m,
/// 1 ft = 0.3048 m).
fn length_factor(unit: &str) -> Option<f64> {
    Some(match unit {
        "m" => 1.0,
        "cm" => 0.01,
        "mm" => 0.001,
        "in" => INCH_M,
        "ft" => FOOT_M,
        _ => return None,
    })
}

/// The avoirdupois pound, kg (NIST Handbook 44, Appendix C).
const POUND_KG: f64 = 0.453_592_37;
/// The avoirdupois ounce, a sixteenth of a pound, kg.
const OUNCE_KG: f64 = POUND_KG / 16.0;
/// The inch, m.
const INCH_M: f64 = 0.0254;
/// The foot, m.
const FOOT_M: f64 = 0.3048;

/// The SI factor of a mass unit (exact definitions, as [`length_factor`]).
fn mass_factor(unit: &str) -> Option<f64> {
    Some(match unit {
        "kg" => 1.0,
        "g" => 0.001,
        "oz" => OUNCE_KG,
        "lb" => POUND_KG,
        _ => return None,
    })
}

/// The SI factor of a density unit of `kind`: the units `docs/TechnicalInfo.md` lists for each
/// kind that OpenRocket 24.12 reads (it refuses a file with `oz/in`), from the exact definitions.
fn density_factor(kind: MaterialKind, unit: &str) -> Option<f64> {
    Some(match (kind, unit) {
        (MaterialKind::Bulk, "kg/m3") => 1.0,
        (MaterialKind::Bulk, "g/cm3" | "kg/dm3") => 1000.0,
        (MaterialKind::Bulk, "lb/ft3") => POUND_KG / (FOOT_M * FOOT_M * FOOT_M),
        (MaterialKind::Surface, "kg/m2") => 1.0,
        (MaterialKind::Surface, "g/m2") => 0.001,
        (MaterialKind::Surface, "g/cm2") => 10.0,
        (MaterialKind::Surface, "oz/in2") => OUNCE_KG / (INCH_M * INCH_M),
        (MaterialKind::Surface, "oz/ft2") => OUNCE_KG / (FOOT_M * FOOT_M),
        (MaterialKind::Surface, "lb/ft2") => POUND_KG / (FOOT_M * FOOT_M),
        (MaterialKind::Line, "kg/m") => 1.0,
        (MaterialKind::Line, "g/m") => 0.001,
        (MaterialKind::Line, "g/cm") => 0.1,
        (MaterialKind::Line, "oz/ft") => OUNCE_KG / FOOT_M,
        _ => return None,
    })
}

/// The lightest fabric this reads without a warning, kg/m²: 1 g/m². The lightest in the bundled
/// files is 0.3 mil polyethylene film at 7.05 g/m²; a canopy under 1 g/m² is a unit written
/// wrong, as the six `g/m2` ripstop nylons of the bundled files are (0.067 g/m² for a fabric of
/// about 67 g/m²).
const LIGHTEST_FABRIC_KG_M2: f64 = 0.001;

impl MaterialKind {
    /// The kind's name, for messages.
    fn name(self) -> &'static str {
        match self {
            Self::Bulk => "bulk",
            Self::Surface => "surface",
            Self::Line => "line",
        }
    }
}

/// The `<Materials>` list. A material that can't be read is left out with a warning, so parts
/// naming it read with no density.
fn read_materials(list: roxmltree::Node<'_, '_>, warnings: &mut Warnings) -> Vec<CatalogMaterial> {
    let mut materials: Vec<CatalogMaterial> = Vec::new();
    for element in list.children().filter(roxmltree::Node::is_element) {
        let name = child(element, "Name").map(text_of).unwrap_or_default();
        let at = format!("Materials: {}", quote(&name));
        let left_out = |warnings: &mut Warnings, why: String| {
            warnings.push(
                &at,
                WarningKind::MaterialLeftOut,
                format!("{why}; it was left out"),
            );
        };
        if element.tag_name().name() != "Material" {
            let why = format!("<{}> is not a material", quote(element.tag_name().name()));
            left_out(warnings, why);
            continue;
        }
        let Some(kind) = child(element, "Type")
            .map(text_of)
            .and_then(|word| MaterialKind::parse(word.trim()))
        else {
            left_out(
                warnings,
                "its <Type> is not BULK, SURFACE or LINE".to_owned(),
            );
            continue;
        };
        let Some(value) = child(element, "Density")
            .and_then(|node| value_text(node, "Density").ok())
            .and_then(|text| number(&text))
        else {
            left_out(
                warnings,
                "its <Density> is missing or not a number".to_owned(),
            );
            continue;
        };
        // A density with no units is read in SI, as OpenRocket 24.12 reads it (the oracle's
        // `no units` probe).
        let unit = element.attribute("UnitsOfMeasure").unwrap_or(match kind {
            MaterialKind::Bulk => "kg/m3",
            MaterialKind::Surface => "kg/m2",
            MaterialKind::Line => "kg/m",
        });
        let Some(factor) = density_factor(kind, unit) else {
            let why = format!(
                "`{}` is not a unit of {} density this reads",
                quote(unit),
                kind.name()
            );
            left_out(warnings, why);
            continue;
        };
        let density = value * factor;
        if let Some(first) = materials
            .iter()
            .find(|material| material.kind == kind && material.name == name)
            && first.density != density
        {
            warnings.push(
                &at,
                WarningKind::MaterialRepeated,
                format!(
                    "it is defined again with another density, {density} against {}; parts \
                     take the first",
                    first.density
                ),
            );
        }
        if kind == MaterialKind::Surface && density < LIGHTEST_FABRIC_KG_M2 {
            warnings.push(
                &at,
                WarningKind::Implausible,
                format!(
                    "{value} {} is {density} kg/m², lighter than any fabric: the unit is likely \
                     wrong; it was read as written, as OpenRocket reads it",
                    quote(unit)
                ),
            );
        }
        materials.push(CatalogMaterial {
            name,
            kind,
            density,
        });
    }
    materials
}

/// Why a part was left out.
struct Skip(String);

/// The fields every kind of part has.
const COMMON_FIELDS: &[&str] = &[
    "Manufacturer",
    "PartNumber",
    "Description",
    "Material",
    "Mass",
];

/// One part, or `None` (with a warning) if it can't be read.
fn read_part(
    element: roxmltree::Node<'_, '_>,
    file: &str,
    materials: &[CatalogMaterial],
    warnings: &mut Warnings,
) -> Option<Part> {
    let tag = element.tag_name().name();
    // Each field by name, the last of a repeated one kept, as OpenRocket keeps it.
    let mut fields: std::collections::BTreeMap<&str, (roxmltree::Node<'_, '_>, usize)> =
        std::collections::BTreeMap::new();
    for node in element.children().filter(roxmltree::Node::is_element) {
        let entry = fields.entry(node.tag_name().name()).or_insert((node, 0));
        *entry = (node, entry.1 + 1);
    }
    let text = |name: &str| fields.get(name).map(|(node, _)| text_of(*node));
    let manufacturer = text("Manufacturer").unwrap_or_default();
    let part_number = text("PartNumber").unwrap_or_default();
    let at = format!("{} {}", quote(&manufacturer), quote(&part_number));
    let Some(known) = known_fields(tag) else {
        warnings.push(
            &at,
            WarningKind::PartLeftOut,
            format!(
                "<{}> is not a kind of part this reads; it was left out",
                quote(tag)
            ),
        );
        return None;
    };
    let unknown: Vec<String> = fields
        .keys()
        .filter(|name| !known.contains(name) && !COMMON_FIELDS.contains(name))
        .map(|name| format!("<{}>", quote(name)))
        .collect();
    if !unknown.is_empty() {
        warnings.push(
            &at,
            WarningKind::Ignored,
            format!(
                "{} {} not a field of a <{tag}> OpenRocket reads; ignored",
                listed(&unknown),
                if unknown.len() == 1 { "is" } else { "are" }
            ),
        );
    }
    for (name, (_, count)) in &fields {
        if *count > 1 && (known.contains(name) || COMMON_FIELDS.contains(name)) {
            warnings.push(
                &at,
                WarningKind::Repeated,
                format!(
                    "<{name}> is stated {count} times; the last was read, as OpenRocket reads it"
                ),
            );
        }
    }
    let get = |name: &str| fields.get(name).map(|(node, _)| *node);
    match part(tag, &get, materials) {
        Ok((kind, mass_kg)) => {
            for material in kind_materials(&kind) {
                if material.density.is_none() {
                    warnings.push(
                        &at,
                        WarningKind::MaterialUndefined,
                        format!(
                            "its material `{}` is not defined in this file, so it has no density",
                            quote(&material.name)
                        ),
                    );
                }
            }
            if let Some(tube) = kind.tube()
                && tube.inner_diameter_m >= tube.outer_diameter_m
            {
                warnings.push(
                    &at,
                    WarningKind::Implausible,
                    format!(
                        "its inside diameter, {} m, is not less than its outside diameter, {} \
                         m, so its wall is {} m; read as written, as OpenRocket reads it",
                        tube.inner_diameter_m,
                        tube.outer_diameter_m,
                        tube.thickness_m()
                    ),
                );
            }
            Some(Part {
                file: file.to_owned(),
                manufacturer,
                part_number,
                description: text("Description").unwrap_or_default(),
                mass_kg,
                kind,
            })
        }
        Err(Skip(reason)) => {
            warnings.push(
                &at,
                WarningKind::PartLeftOut,
                format!("{reason}; the part was left out"),
            );
            None
        }
    }
}

/// The fields OpenRocket 24.12 reads for a kind of part, besides [`COMMON_FIELDS`]; `None` for an
/// element that isn't a kind of part.
fn known_fields(tag: &str) -> Option<&'static [&'static str]> {
    Some(match tag {
        "BodyTube" | "TubeCoupler" | "EngineBlock" | "CenteringRing" | "LaunchLug" => {
            &["InsideDiameter", "OutsideDiameter", "Length"]
        }
        "BulkHead" => &["OutsideDiameter", "Length", "Filled"],
        "NoseCone" => &[
            "Shape",
            "OutsideDiameter",
            "ShoulderDiameter",
            "ShoulderLength",
            "Length",
            "Filled",
            "Thickness",
        ],
        "Transition" => &[
            "Shape",
            "ForeOutsideDiameter",
            "ForeShoulderDiameter",
            "ForeShoulderLength",
            "AftOutsideDiameter",
            "AftShoulderDiameter",
            "AftShoulderLength",
            "Length",
            "Filled",
            "Thickness",
        ],
        "Parachute" => &[
            "Diameter",
            "Sides",
            "LineCount",
            "LineLength",
            "LineMaterial",
        ],
        "Streamer" => &["Width", "Length", "Thickness"],
        _ => return None,
    })
}

impl PartKind {
    /// The tube-like part's sizes, for the five tube-like kinds.
    #[must_use]
    pub fn tube(&self) -> Option<&Tube> {
        match self {
            Self::BodyTube(tube)
            | Self::TubeCoupler(tube)
            | Self::EngineBlock(tube)
            | Self::CenteringRing(tube)
            | Self::LaunchLug(tube) => Some(tube),
            _ => None,
        }
    }

    /// The materials the part names: one, or a parachute's canopy and its lines.
    #[must_use]
    pub fn materials(&self) -> Vec<&MaterialRef> {
        match self {
            Self::BodyTube(tube)
            | Self::TubeCoupler(tube)
            | Self::EngineBlock(tube)
            | Self::CenteringRing(tube)
            | Self::LaunchLug(tube) => vec![&tube.material],
            Self::Bulkhead(bulkhead) => vec![&bulkhead.material],
            Self::NoseCone(nose) => vec![&nose.material],
            Self::Transition(transition) => vec![&transition.material],
            Self::Parachute(parachute) => {
                let mut list = vec![&parachute.material];
                list.extend(parachute.line_material.as_ref());
                list
            }
            Self::Streamer(streamer) => vec![&streamer.material],
        }
    }
}

/// The materials a part names.
fn kind_materials(kind: &PartKind) -> Vec<&MaterialRef> {
    kind.materials()
}

/// A part's kind and stated mass; `get` finds a field by name.
fn part<'a, 'input: 'a>(
    tag: &str,
    get: &dyn Fn(&str) -> Option<roxmltree::Node<'a, 'input>>,
    materials: &[CatalogMaterial],
) -> Result<(PartKind, Option<f64>), Skip> {
    if get("Manufacturer").is_none() {
        return Err(Skip("it names no <Manufacturer>".to_owned()));
    }
    if get("PartNumber").is_none() {
        return Err(Skip("it has no <PartNumber>".to_owned()));
    }
    let mass_kg = get("Mass")
        .map(|node| measure(node, "Mass", mass_factor))
        .transpose()?;
    let length = |name: &str| -> Result<f64, Skip> {
        let node = get(name).ok_or_else(|| Skip(format!("it has no <{name}>")))?;
        measure(node, name, length_factor)
    };
    let optional_length = |name: &str| -> Result<Option<f64>, Skip> {
        get(name)
            .map(|node| measure(node, name, length_factor))
            .transpose()
    };
    let material = |name: &str, want: MaterialKind| -> Result<MaterialRef, Skip> {
        let node = get(name).ok_or_else(|| Skip(format!("it names no <{name}>")))?;
        material_ref(node, name, want, materials)
    };
    let word = |name: &str| -> Result<Option<String>, Skip> {
        get(name)
            .map(|node| value_text(node, name).map_err(Skip))
            .transpose()
    };
    let filled = || -> Result<Option<bool>, Skip> {
        word("Filled")?
            .map(|text| match text.trim() {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                other => Err(Skip(format!(
                    "<Filled> is `{}`, not true or false",
                    quote(other)
                ))),
            })
            .transpose()
    };
    let shape = || -> Result<Shape, Skip> {
        let text = word("Shape")?.ok_or_else(|| Skip("it has no <Shape>".to_owned()))?;
        Shape::parse(text.trim()).ok_or_else(|| {
            Skip(format!(
                "<Shape> is `{}`, not one of CONICAL, OGIVE, ELLIPSOID, PARABOLIC, HAACK or POWER",
                quote(&text)
            ))
        })
    };
    let count = |name: &str| -> Result<u32, Skip> {
        let text = word(name)?.ok_or_else(|| Skip(format!("it has no <{name}>")))?;
        text.trim().parse::<u32>().map_err(|_| {
            Skip(format!(
                "<{name}> is `{}`, not a whole number",
                quote(&text)
            ))
        })
    };
    let tube = || -> Result<Tube, Skip> {
        Ok(Tube {
            inner_diameter_m: length("InsideDiameter")?,
            outer_diameter_m: length("OutsideDiameter")?,
            length_m: length("Length")?,
            material: material("Material", MaterialKind::Bulk)?,
        })
    };
    let kind = match tag {
        "BodyTube" => PartKind::BodyTube(tube()?),
        "TubeCoupler" => PartKind::TubeCoupler(tube()?),
        "EngineBlock" => PartKind::EngineBlock(tube()?),
        "CenteringRing" => PartKind::CenteringRing(tube()?),
        "LaunchLug" => PartKind::LaunchLug(tube()?),
        "BulkHead" => PartKind::Bulkhead(Bulkhead {
            outer_diameter_m: length("OutsideDiameter")?,
            length_m: length("Length")?,
            filled: filled()?,
            material: material("Material", MaterialKind::Bulk)?,
        }),
        "NoseCone" => PartKind::NoseCone(NoseCone {
            shape: shape()?,
            length_m: length("Length")?,
            outer_diameter_m: length("OutsideDiameter")?,
            shoulder_diameter_m: length("ShoulderDiameter")?,
            shoulder_length_m: length("ShoulderLength")?,
            filled: filled()?,
            thickness_m: optional_length("Thickness")?,
            material: material("Material", MaterialKind::Bulk)?,
        }),
        "Transition" => PartKind::Transition(Transition {
            shape: shape()?,
            length_m: length("Length")?,
            fore_outer_diameter_m: length("ForeOutsideDiameter")?,
            fore_shoulder_diameter_m: length("ForeShoulderDiameter")?,
            fore_shoulder_length_m: length("ForeShoulderLength")?,
            aft_outer_diameter_m: length("AftOutsideDiameter")?,
            aft_shoulder_diameter_m: length("AftShoulderDiameter")?,
            aft_shoulder_length_m: length("AftShoulderLength")?,
            filled: filled()?,
            thickness_m: optional_length("Thickness")?,
            material: material("Material", MaterialKind::Bulk)?,
        }),
        "Parachute" => PartKind::Parachute(Parachute {
            diameter_m: length("Diameter")?,
            sides: count("Sides")?,
            line_count: count("LineCount")?,
            line_length_m: length("LineLength")?,
            material: material("Material", MaterialKind::Surface)?,
            line_material: get("LineMaterial")
                .map(|node| material_ref(node, "LineMaterial", MaterialKind::Line, materials))
                .transpose()?,
        }),
        "Streamer" => PartKind::Streamer(Streamer {
            length_m: length("Length")?,
            width_m: length("Width")?,
            thickness_m: length("Thickness")?,
            material: material("Material", MaterialKind::Surface)?,
        }),
        // `known_fields` let through only the kinds above.
        _ => return Err(Skip(format!("<{}> is not a kind of part", quote(tag)))),
    };
    Ok((kind, mass_kg))
}

/// A size in SI: its number times its `Unit`'s factor. A size with no unit is in SI, as
/// OpenRocket 24.12 reads it (the oracle's `no units` probe).
fn measure(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    factor: fn(&str) -> Option<f64>,
) -> Result<f64, Skip> {
    let text = value_text(node, name).map_err(Skip)?;
    let value = number(&text)
        .ok_or_else(|| Skip(format!("<{name}> is `{}`, not a number", quote(&text))))?;
    let scale = match node.attribute("Unit") {
        None => 1.0,
        Some(unit) => factor(unit).ok_or_else(|| {
            Skip(format!(
                "<{name}>'s unit `{}` is not one this reads",
                quote(unit)
            ))
        })?,
    };
    Ok(value * scale)
}

/// The material a part names: its name and kind from the part, its density from the first
/// material of that name and kind in the file. A part that names a material as another kind than
/// it needs is left out (OpenRocket reads it with a density of zero; no bundled part does it).
fn material_ref(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    want: MaterialKind,
    materials: &[CatalogMaterial],
) -> Result<MaterialRef, Skip> {
    let kind = node
        .attribute("Type")
        .and_then(MaterialKind::parse)
        .ok_or_else(|| Skip(format!("<{name}>'s Type is not BULK, SURFACE or LINE")))?;
    if kind != want {
        return Err(Skip(format!(
            "<{name}> names a material of the wrong kind for the part"
        )));
    }
    let material_name = value_text(node, name).map_err(Skip)?;
    let density = materials
        .iter()
        .find(|material| material.kind == kind && material.name == material_name)
        .map(|material| material.density);
    Ok(MaterialRef {
        name: material_name,
        kind,
        density,
    })
}
