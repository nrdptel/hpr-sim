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
//! value equals OpenRocket's bit for bit, with three exceptions that the test
//! `crates/hpr-io/tests/orc_openrocket.rs` counts:
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
//!   density and the part keeps its mass ([`Part::mass_kg`]); a design built from the part can
//!   override its mass with it. This is 207 parts. A parachute's or streamer's stated mass leaves
//!   its fabric's density alone, here and in OpenRocket.
//!
//! **What the file leaves unsaid.** No `.orc` field gives a nose or transition shape's parameter
//! (an ogive's radius, a Haack series' `C`), a shoulder's wall, or a parachute's drag coefficient;
//! OpenRocket uses its own defaults for these when a part is put in a design. A part's material
//! is only as good as the file's density, and the database's own README warns to weigh real parts.
//! [`MaterialRef::density`] is `None` for a material the file names but doesn't define (3 parts in
//! the bundled files; OpenRocket gives each a density of zero).

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

/// Something read differently from what the file says, or left out. Reading goes on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Warning {
    /// Where: the part's maker and number, or `Materials`.
    pub at: String,
    /// What happened, in a sentence.
    pub message: String,
}

/// A catalogue file read, with what was left out of it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Read {
    /// The parts read, in the file's order.
    pub catalog: Catalog,
    /// The file's `<Version>`, as written; `None` if it has none.
    pub version: Option<String>,
    /// The materials the file defines, in its order.
    pub materials: Vec<CatalogMaterial>,
    /// Parts left out and values read differently, one entry each.
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
    /// and the part number exactly. Most numbers name one part; in the bundled catalogue, 21
    /// numbers name two parts of the same kind, and 4 of those pairs are identical.
    #[must_use]
    pub fn find(&self, manufacturer: &str, part_number: &str) -> Vec<&Part> {
        let manufacturer = manufacturer.trim();
        let part_number = part_number.trim();
        self.parts
            .iter()
            .filter(|part| {
                part.manufacturer.trim().eq_ignore_ascii_case(manufacturer)
                    && part.part_number.trim() == part_number
            })
            .collect()
    }

    /// Every part whose part number or description holds `text`, in any case, from the maker
    /// `manufacturer` (in any case) or from any maker if `None`, in catalogue order. Part numbers
    /// are often several numbers in one, such as Estes' `BT-20, 30316`, which `find` matches
    /// only whole; a search for `BT-20` finds it, and `BT-20P` and every other number holding it.
    #[must_use]
    pub fn search(&self, manufacturer: Option<&str>, text: &str) -> Vec<&Part> {
        let text = text.trim().to_lowercase();
        let manufacturer = manufacturer.map(str::trim);
        self.parts
            .iter()
            .filter(|part| {
                manufacturer
                    .is_none_or(|maker| part.manufacturer.trim().eq_ignore_ascii_case(maker))
                    && (part.part_number.to_lowercase().contains(&text)
                        || part.description.to_lowercase().contains(&text))
            })
            .collect()
    }

    /// The makers, each once, in the order they first appear.
    #[must_use]
    pub fn manufacturers(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for part in &self.parts {
            if !names.contains(&part.manufacturer.as_str()) {
                names.push(&part.manufacturer);
            }
        }
        names
    }

    /// Adds another catalogue's parts after this one's.
    pub fn extend(&mut self, other: Catalog) {
        self.parts.extend(other.parts);
    }
}

/// Reads one `.orc` file's text; `file` names it in each [`Part::file`].
///
/// Only text that isn't XML, or whose root isn't `<OpenRocketComponent>`, is refused. A part
/// this can't read (a missing or unreadable dimension, a unit or shape the format doesn't have, a
/// kind of part it doesn't know) is left out with a warning, where OpenRocket 24.12 refuses the
/// whole file for most of these; unknown elements inside a part are ignored with a warning.
///
/// # Errors
///
/// [`OrcError::Xml`] and [`OrcError::NotACatalog`].
pub fn read(text: &str, file: &str) -> Result<Read, OrcError> {
    let document = roxmltree::Document::parse(text).map_err(|error| OrcError::Xml {
        reason: error.to_string(),
    })?;
    let root = document.root_element();
    if root.tag_name().name() != "OpenRocketComponent" {
        return Err(OrcError::NotACatalog {
            root: quote(root.tag_name().name()),
        });
    }
    let mut warnings = Vec::new();
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
    Ok(Read {
        catalog: Catalog { parts },
        version,
        materials,
        warnings,
    })
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
        "in" => 0.0254,
        "ft" => 0.3048,
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

/// The `<Materials>` list. A material that can't be read is left out with a warning, so parts
/// naming it read with no density.
fn read_materials(
    list: roxmltree::Node<'_, '_>,
    warnings: &mut Vec<Warning>,
) -> Vec<CatalogMaterial> {
    let mut materials = Vec::new();
    for element in list.children().filter(roxmltree::Node::is_element) {
        let name = child(element, "Name").map(text_of).unwrap_or_default();
        let mut warn = |message: String| {
            warnings.push(Warning {
                at: format!("Materials: {}", quote(&name)),
                message,
            });
        };
        if element.tag_name().name() != "Material" {
            warn(format!(
                "<{}> is not a material; it was left out",
                quote(element.tag_name().name())
            ));
            continue;
        }
        let Some(kind) = child(element, "Type")
            .map(text_of)
            .and_then(|word| MaterialKind::parse(word.trim()))
        else {
            warn("its <Type> is not BULK, SURFACE or LINE; it was left out".to_owned());
            continue;
        };
        let Some(value) = child(element, "Density")
            .map(text_of)
            .and_then(|text| number(&text))
        else {
            warn("its <Density> is missing or not a number; it was left out".to_owned());
            continue;
        };
        // A density with no units is read in SI, as OpenRocket 24.12 reads it (measured by probe).
        let unit = element.attribute("UnitsOfMeasure").unwrap_or(match kind {
            MaterialKind::Bulk => "kg/m3",
            MaterialKind::Surface => "kg/m2",
            MaterialKind::Line => "kg/m",
        });
        let Some(factor) = density_factor(kind, unit) else {
            warn(format!(
                "`{}` is not a unit of {} density the format has; it was left out",
                quote(unit),
                match kind {
                    MaterialKind::Bulk => "bulk",
                    MaterialKind::Surface => "surface",
                    MaterialKind::Line => "line",
                }
            ));
            continue;
        };
        materials.push(CatalogMaterial {
            name,
            kind,
            density: value * factor,
        });
    }
    materials
}

/// The fields of one part element, by name; a field stated twice keeps the last, as OpenRocket
/// does.
struct Fields<'a, 'input> {
    nodes: Vec<(&'a str, roxmltree::Node<'a, 'input>)>,
}

impl<'a, 'input> Fields<'a, 'input> {
    fn get(&self, name: &str) -> Option<roxmltree::Node<'a, 'input>> {
        self.nodes
            .iter()
            .rev()
            .find(|(field, _)| *field == name)
            .map(|(_, node)| *node)
    }
}

/// Why a part was left out.
struct Skip(String);

/// One part, or `None` (with a warning) if it can't be read.
fn read_part(
    element: roxmltree::Node<'_, '_>,
    file: &str,
    materials: &[CatalogMaterial],
    warnings: &mut Vec<Warning>,
) -> Option<Part> {
    let tag = element.tag_name().name();
    let fields = Fields {
        nodes: element
            .children()
            .filter(roxmltree::Node::is_element)
            .map(|node| (node.tag_name().name(), node))
            .collect(),
    };
    let manufacturer = fields.get("Manufacturer").map(text_of).unwrap_or_default();
    let part_number = fields.get("PartNumber").map(text_of).unwrap_or_default();
    let at = format!("{} {}", quote(&manufacturer), quote(&part_number));
    let known = known_fields(tag);
    let Some(known) = known else {
        warnings.push(Warning {
            at,
            message: format!(
                "<{}> is not a kind of part this reads; it was left out",
                quote(tag)
            ),
        });
        return None;
    };
    let mut seen: Vec<&str> = Vec::new();
    for (name, _) in &fields.nodes {
        if seen.contains(name) {
            continue;
        }
        let count = fields
            .nodes
            .iter()
            .filter(|(other, _)| other == name)
            .count();
        if !known.contains(name) && !COMMON_FIELDS.contains(name) {
            warnings.push(Warning {
                at: at.clone(),
                message: format!(
                    "<{}> is not a field of a <{tag}> OpenRocket reads; it was ignored",
                    quote(name)
                ),
            });
        } else if count > 1 {
            warnings.push(Warning {
                at: at.clone(),
                message: format!(
                    "<{name}> is stated {count} times; the last was read, as OpenRocket reads it"
                ),
            });
        }
        seen.push(name);
    }
    match part(tag, &fields, materials) {
        Ok((kind, mass_kg)) => {
            let unresolved = kind_materials(&kind)
                .into_iter()
                .filter(|material| material.density.is_none())
                .map(|material| quote(&material.name))
                .collect::<Vec<_>>();
            for name in unresolved {
                warnings.push(Warning {
                    at: at.clone(),
                    message: format!(
                        "its material `{name}` is not defined in this file, so it has no density"
                    ),
                });
            }
            Some(Part {
                file: file.to_owned(),
                manufacturer,
                part_number,
                description: fields.get("Description").map(text_of).unwrap_or_default(),
                mass_kg,
                kind,
            })
        }
        Err(Skip(reason)) => {
            warnings.push(Warning {
                at,
                message: format!("{reason}; the part was left out"),
            });
            None
        }
    }
}

/// The fields every kind of part has.
const COMMON_FIELDS: &[&str] = &[
    "Manufacturer",
    "PartNumber",
    "Description",
    "Material",
    "Mass",
];

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

/// The materials a part names.
fn kind_materials(kind: &PartKind) -> Vec<&MaterialRef> {
    match kind {
        PartKind::BodyTube(tube)
        | PartKind::TubeCoupler(tube)
        | PartKind::EngineBlock(tube)
        | PartKind::CenteringRing(tube)
        | PartKind::LaunchLug(tube) => vec![&tube.material],
        PartKind::Bulkhead(bulkhead) => vec![&bulkhead.material],
        PartKind::NoseCone(nose) => vec![&nose.material],
        PartKind::Transition(transition) => vec![&transition.material],
        PartKind::Parachute(parachute) => {
            let mut list = vec![&parachute.material];
            list.extend(parachute.line_material.as_ref());
            list
        }
        PartKind::Streamer(streamer) => vec![&streamer.material],
    }
}

/// A part's kind and stated mass.
fn part(
    tag: &str,
    fields: &Fields<'_, '_>,
    materials: &[CatalogMaterial],
) -> Result<(PartKind, Option<f64>), Skip> {
    if fields.get("Manufacturer").is_none() {
        return Err(Skip("it names no <Manufacturer>".to_owned()));
    }
    if fields.get("PartNumber").is_none() {
        return Err(Skip("it has no <PartNumber>".to_owned()));
    }
    let mass_kg = fields
        .get("Mass")
        .map(|node| measure(node, "Mass", mass_factor))
        .transpose()?;
    let length = |name: &str| -> Result<f64, Skip> {
        let node = fields
            .get(name)
            .ok_or_else(|| Skip(format!("it has no <{name}>")))?;
        measure(node, name, length_factor)
    };
    let optional_length = |name: &str| -> Result<Option<f64>, Skip> {
        fields
            .get(name)
            .map(|node| measure(node, name, length_factor))
            .transpose()
    };
    let material = |name: &str, want: MaterialKind| -> Result<MaterialRef, Skip> {
        let node = fields
            .get(name)
            .ok_or_else(|| Skip(format!("it names no <{name}>")))?;
        material_ref(node, name, want, materials)
    };
    let filled = || -> Result<Option<bool>, Skip> {
        fields
            .get("Filled")
            .map(|node| match text_of(node).trim() {
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
        let word = fields
            .get("Shape")
            .map(text_of)
            .ok_or_else(|| Skip("it has no <Shape>".to_owned()))?;
        Shape::parse(word.trim()).ok_or_else(|| {
            Skip(format!(
                "<Shape> is `{}`, not one of CONICAL, OGIVE, ELLIPSOID, PARABOLIC, HAACK or POWER",
                quote(&word)
            ))
        })
    };
    let count = |name: &str| -> Result<u32, Skip> {
        let text = fields
            .get(name)
            .map(text_of)
            .ok_or_else(|| Skip(format!("it has no <{name}>")))?;
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
            line_material: fields
                .get("LineMaterial")
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

/// A dimension in SI: its number times its `Unit`'s factor. A dimension with no unit is in SI,
/// as OpenRocket 24.12 reads it (measured by probe).
fn measure(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    factor: fn(&str) -> Option<f64>,
) -> Result<f64, Skip> {
    let text = text_of(node);
    let value = number(&text)
        .ok_or_else(|| Skip(format!("<{name}> is `{}`, not a number", quote(&text))))?;
    let scale = match node.attribute("Unit") {
        None => 1.0,
        Some(unit) => factor(unit).ok_or_else(|| {
            Skip(format!(
                "<{name}>'s unit `{}` is not one the format has",
                quote(unit)
            ))
        })?,
    };
    Ok(value * scale)
}

/// The material a part names: its name and kind from the part, its density from the first
/// material of that name and kind in the file. A part that names a material as another kind than
/// it needs is left out: no bundled part does.
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
    let material_name = text_of(node);
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
