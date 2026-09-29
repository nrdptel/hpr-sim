//! What a `.ork` design holds that hpr's design does not model, kept whole for an export to put
//! back: the `x-openrocket` extension.
//!
//! **What is kept.** Four kinds of thing, each with the path it was found at:
//!
//! - **Parts** hpr does not read: every child of a `<subcomponents>` that the walk left out — a pod
//!   set or a parallel stage ([Loft lesson L66][l66]: Loft dropped them), a part hpr cannot give a
//!   shape, or a tag it has never seen. A design with any is *reduced*: its rocket is not the whole
//!   of what the file describes ([`super::Design::is_reduced`]).
//! - **Sections** of the document hpr does not read: every child of `<openrocket>` besides
//!   `<rocket>` and `<simulations>` (such as `<photostudio>` or `<docprefs>`), and every child of a
//!   stored `<simulation>` besides its name, simulator, calculator, conditions and flight data (such
//!   as a simulation `<extension>`).
//!
//! - **Tags** no reader asks for in an element hpr does read — the rocket, a stage, a part, a
//!   stored simulation, and any tag inside those a reader did ask for — such as a part's
//!   `<appearance>`.
//! - **Attributes** no reader asks for on an element hpr does read, such as a material's `group`.
//!
//! The readers record every tag and attribute they ask for while [`super::design`] reads, so one is
//! kept when nothing asked for it. One a reader asked for and then dropped or simplified — a rail
//! button's screw height, a drag override, a ring's count above one, a word with no reading — is
//! kept too, since the design does not hold what it says: an export writes it back in place of
//! what the design would, and reading that export warns of it again.
//!
//! **What is not.** The text of a second copy of a tag a reader takes once by name; its attributes
//! and unread children are kept. Everything is still in the document itself, which
//! [`super::OrkFile`] keeps whole ([ADR-051][adr-051]); [`super::export`] writes a design back out
//! from the design and these.
//!
//! **The path.** `openrocket/rocket/stage[0]/bodytube[1]/podset[0]` counts each part's step among
//! all its parent's `<subcomponents>` children, the way a warning's path does, since the order of
//! parts is where they stack. A section's step, and a tag's, marked `@` at any depth, counts only
//! among the parent's child elements of its own name: `openrocket/rocket/stage[0]/nosecone[0]/
//! @appearance[0]` is the nose cone's first `<appearance>`, wherever it stood among the other
//! tags. OpenRocket does not read meaning into the order of tags, and counting this way lets an
//! export put each one back without knowing that order ([ADR-109][adr-109]). An attribute keeps
//! the path of the element it was on. [`element_at`] follows a path back.
//!
//! [l66]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l66
//! [adr-051]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20
//! [adr-109]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-109-m32-split-and-a-ork-written-from-the-design-2026-09-29

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::component::subcomponents;
use super::document::{Document, Element};
use super::reads::{self, Reads};

/// The extensions a `.ork` design carries, by namespace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Extensions {
    /// What the design holds that hpr does not model.
    #[serde(rename = "x-openrocket", default)]
    pub x_openrocket: OpenRocketExtension,
}

/// The `x-openrocket` extension: the parts and sections of a `.ork` that hpr does not read, each
/// kept whole where it was.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OpenRocketExtension {
    /// The parts hpr does not read, in file order.
    #[serde(default)]
    pub parts: Vec<Kept>,
    /// The sections of the document hpr does not read, in file order.
    #[serde(default)]
    pub sections: Vec<Kept>,
    /// The tags hpr does not read in an element it does read — a part, a stage, the rocket, a
    /// stored simulation, or a tag inside any of those that a reader asked for — such as a part's
    /// `<appearance>`; and those a reader asked for and dropped or simplified, such as a rail
    /// button's `<screwheight>`, which the design does not hold.
    #[serde(default)]
    pub tags: Vec<Kept>,
    /// The attributes hpr does not read on an element it does read, such as a material's
    /// `group`, and those whose value a reader dropped, such as a material's declared `type` where
    /// the part needs another.
    #[serde(default)]
    pub attributes: Vec<KeptAttribute>,
}

/// An attribute kept, and the element it was on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KeptAttribute {
    /// The path of the element it was on; see [`element_at`].
    pub at: String,
    /// Its name.
    pub name: String,
    /// Its value, as written.
    pub value: String,
}

/// An element kept whole, and where it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Kept {
    /// Its path in the document; see [`element_at`].
    pub at: String,
    /// The element, with everything inside it.
    pub element: Element,
}

/// Everything in `document` that hpr does not read, given the paths of the stages and components
/// the walk read and every tag and attribute the readers asked for.
pub(super) fn read(
    document: &Document,
    read: &BTreeSet<String>,
    asked: &Reads,
) -> OpenRocketExtension {
    let mut kept = OpenRocketExtension::default();
    let root = &document.root;
    // hpr reads the first `<rocket>` and the first `<simulations>`; a second is kept whole.
    let (mut rocket_seen, mut simulations_seen) = (false, false);
    let mut sections = Counter::default();
    for child in root.elements() {
        let index = sections.next(&child.name);
        match child.name.as_str() {
            "rocket" if !std::mem::replace(&mut rocket_seen, true) => {
                unasked(child, "openrocket/rocket", asked, &mut kept);
                for (index, part) in subcomponents(child).enumerate() {
                    let at = format!("openrocket/rocket/{}[{index}]", part.name);
                    parts(part, &at, read, asked, &mut kept);
                }
            }
            "simulations" if !std::mem::replace(&mut simulations_seen, true) => {
                // Anything beside the `<simulation>`s is kept, counted among its own name.
                let mut seen = Counter::default();
                for other in child.elements().filter(|e| e.name != "simulation") {
                    let index = seen.next(&other.name);
                    kept.sections.push(Kept {
                        at: format!("openrocket/simulations/{}[{index}]", other.name),
                        element: other.clone(),
                    });
                }
                for (index, simulation) in child.children_named("simulation").enumerate() {
                    let at = format!("openrocket/simulations/simulation[{index}]");
                    keep_attributes(simulation, &at, asked, &mut kept);
                    let mut seen = Counter::default();
                    for inside in simulation.elements() {
                        let index = seen.next(&inside.name);
                        let path = format!("{at}/{}[{index}]", inside.name);
                        if reads::asked(asked, simulation, &inside.name) {
                            unasked(inside, &path, asked, &mut kept);
                        } else {
                            kept.sections.push(Kept {
                                at: path,
                                element: inside.clone(),
                            });
                        }
                    }
                }
            }
            _ => kept.sections.push(Kept {
                at: format!("openrocket/{}[{index}]", child.name),
                element: child.clone(),
            }),
        }
    }
    kept
}

/// Keeps the attributes of `element` no reader asked for, and each child tag no reader asked for,
/// looking inside the ones a reader did. The parts inside it are [`parts`]'s.
fn unasked(element: &Element, at: &str, asked: &Reads, kept: &mut OpenRocketExtension) {
    keep_attributes(element, at, asked, kept);
    let mut seen = Counter::default();
    for tag in element.elements() {
        let index = seen.next(&tag.name);
        if tag.name == "subcomponents" {
            continue;
        }
        let path = format!("{at}/@{}[{index}]", tag.name);
        if reads::asked(asked, element, &tag.name) {
            unasked(tag, &path, asked, kept);
        } else {
            kept.tags.push(Kept {
                at: path,
                element: tag.clone(),
            });
        }
    }
}

/// Keeps the attributes of `element` no reader asked for.
fn keep_attributes(element: &Element, at: &str, asked: &Reads, kept: &mut OpenRocketExtension) {
    for (name, value) in &element.attributes {
        if !reads::asked_attribute(asked, element, name) {
            kept.attributes.push(KeptAttribute {
                at: at.to_owned(),
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
}

/// Keeps `element` whole if the walk did not read it, or looks inside it if it did: at what no
/// reader asked for in it, and at the parts inside it.
fn parts(
    element: &Element,
    at: &str,
    read: &BTreeSet<String>,
    asked: &Reads,
    kept: &mut OpenRocketExtension,
) {
    if !read.contains(at) {
        kept.parts.push(Kept {
            at: at.to_owned(),
            element: element.clone(),
        });
        return;
    }
    unasked(element, at, asked, kept);
    for (index, child) in subcomponents(element).enumerate() {
        parts(
            child,
            &format!("{at}/{}[{index}]", child.name),
            read,
            asked,
            kept,
        );
    }
}

/// The element of `document` at `at`, a path as [`Kept::at`] writes it, or `None` when the path
/// does not lead to an element of the name it gives.
pub fn element_at<'a>(document: &'a Document, at: &str) -> Option<&'a Element> {
    let mut steps = at.split('/');
    if steps.next() != Some("openrocket") {
        return None;
    }
    let root = &document.root;
    let step = |text: &str| -> Option<(String, usize)> {
        let (name, rest) = text.split_once('[')?;
        let index = rest.strip_suffix(']')?.parse().ok()?;
        Some((name.to_owned(), index))
    };
    // A tag step, `@name[k]`: the k-th child element called `name`.
    let tag = |here: &'a Element, text: &str| -> Option<&'a Element> {
        let (name, index) = step(text.strip_prefix('@')?)?;
        here.children_named(&name).nth(index)
    };
    match steps.next()? {
        "rocket" => {
            let mut here = root.child("rocket")?;
            let mut in_tags = false;
            for text in steps {
                // Only tags follow a tag.
                if text.starts_with('@') {
                    in_tags = true;
                    here = tag(here, text)?;
                } else if in_tags {
                    return None;
                } else {
                    let (name, index) = step(text)?;
                    here = subcomponents(here)
                        .nth(index)
                        .filter(|child| child.name == name)?;
                }
            }
            Some(here)
        }
        "simulations" => {
            let (name, index) = step(steps.next()?)?;
            let simulation = root
                .child("simulations")?
                .children_named(&name)
                .nth(index)?;
            let Some(text) = steps.next() else {
                return Some(simulation);
            };
            let (name, index) = step(text)?;
            let mut here = simulation.children_named(&name).nth(index)?;
            for text in steps {
                here = tag(here, text)?;
            }
            Some(here)
        }
        text => {
            let (name, index) = step(text)?;
            let section = root.children_named(&name).nth(index)?;
            steps.next().is_none().then_some(section)
        }
    }
}

/// Counts the child elements met so far by name, for a step that counts among its own name.
#[derive(Default)]
pub(super) struct Counter(std::collections::BTreeMap<String, usize>);

impl Counter {
    /// How many elements called `name` came before this one; counts this one.
    pub(super) fn next(&mut self, name: &str) -> usize {
        let count = self.0.entry(name.to_owned()).or_default();
        let index = *count;
        *count += 1;
        index
    }
}
