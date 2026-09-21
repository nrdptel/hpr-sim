//! What a `.ork` design holds that hpr's design does not model, kept whole for an export to put
//! back: the `x-openrocket` extension.
//!
//! **What is kept.** Three kinds of element, each with the path it was found at:
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
//! - **Tags** hpr does not read in an element it does read: a part's, a stage's or the rocket's own
//!   tags that no reader asks for (a part's `<appearance>`, say), and the same in a stored
//!   simulation's conditions and results. The readers record every tag they ask for while
//!   [`super::design`] reads, so a tag is kept exactly when nothing asked for it.
//!
//! **What is not, yet.** Anything deeper: an attribute, or a tag inside a tag hpr reads (inside
//! `<motormount>`, say). It stays in the document itself, which [`super::OrkFile`] keeps whole
//! ([ADR-051][adr-051]). Writing a `.ork` back out is [M3.2][m3-2]'s work, and it starts from both.
//!
//! **The path.** `openrocket/rocket/stage[0]/bodytube[1]/podset[0]` counts each step among its
//! parent's `<subcomponents>` children, the way a warning's path does; a section's step counts
//! among its parent's child elements, and so does a tag's, marked `@`:
//! `openrocket/rocket/stage[0]/nosecone[0]/@appearance[3]`. [`element_at`] follows one back.
//!
//! [l66]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l66
//! [adr-051]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20
//! [m3-2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-2

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
    /// stored simulation's conditions or results — such as a part's `<appearance>`.
    #[serde(default)]
    pub tags: Vec<Kept>,
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
/// the walk read and every tag the readers asked for.
pub(super) fn read(
    document: &Document,
    read: &BTreeSet<String>,
    asked: &Reads,
) -> OpenRocketExtension {
    let mut kept = OpenRocketExtension::default();
    // The tags of every element read whole, apart from the parts inside it.
    let mut read_elements: Vec<(String, &Element)> = Vec::new();
    if let Some(rocket) = document.root.child("rocket") {
        read_elements.push(("openrocket/rocket".to_owned(), rocket));
    }
    for at in read {
        if let Some(element) = element_at(document, at) {
            read_elements.push((at.clone(), element));
        }
    }
    if let Some(simulations) = document.root.child("simulations") {
        for (index, simulation) in simulations.children_named("simulation").enumerate() {
            for (inner, inside) in simulation.elements().enumerate() {
                if matches!(inside.name.as_str(), "conditions" | "flightdata") {
                    read_elements.push((
                        format!(
                            "openrocket/simulations/simulation[{index}]/{}[{inner}]",
                            inside.name
                        ),
                        inside,
                    ));
                }
            }
        }
    }
    for (at, element) in read_elements {
        for (index, tag) in element.elements().enumerate() {
            if tag.name != "subcomponents" && !reads::asked(asked, element, &tag.name) {
                kept.tags.push(Kept {
                    at: format!("{at}/@{}[{index}]", tag.name),
                    element: tag.clone(),
                });
            }
        }
    }
    let root = &document.root;
    // hpr reads the first `<rocket>` and the first `<simulations>`; a second is kept whole.
    let (mut rocket_seen, mut simulations_seen) = (false, false);
    for (index, child) in root.elements().enumerate() {
        let first = match child.name.as_str() {
            "rocket" => !std::mem::replace(&mut rocket_seen, true),
            "simulations" => !std::mem::replace(&mut simulations_seen, true),
            _ => false,
        };
        match child.name.as_str() {
            "rocket" if first => {
                for (index, part) in subcomponents(child).enumerate() {
                    let at = format!("openrocket/rocket/{}[{index}]", part.name);
                    parts(part, &at, read, &mut kept.parts);
                }
            }
            "simulations" if first => {
                for (index, simulation) in child.children_named("simulation").enumerate() {
                    let at = format!("openrocket/simulations/simulation[{index}]");
                    for (index, inside) in simulation.elements().enumerate() {
                        if !reads::asked(asked, simulation, &inside.name) {
                            kept.sections.push(Kept {
                                at: format!("{at}/{}[{index}]", inside.name),
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

/// Keeps `element` whole if the walk did not read it, or looks inside it if it did.
fn parts(element: &Element, at: &str, read: &BTreeSet<String>, kept: &mut Vec<Kept>) {
    if !read.contains(at) {
        kept.push(Kept {
            at: at.to_owned(),
            element: element.clone(),
        });
        return;
    }
    for (index, child) in subcomponents(element).enumerate() {
        parts(child, &format!("{at}/{}[{index}]", child.name), read, kept);
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
    match steps.next()? {
        "rocket" => {
            let mut here = root.child("rocket")?;
            while let Some(text) = steps.next() {
                if let Some(text) = text.strip_prefix('@') {
                    // A tag of the element reached so far, which ends the path.
                    let (name, index) = step(text)?;
                    let tag = here
                        .elements()
                        .nth(index)
                        .filter(|child| child.name == name)?;
                    return steps.next().is_none().then_some(tag);
                }
                let (name, index) = step(text)?;
                here = subcomponents(here)
                    .nth(index)
                    .filter(|child| child.name == name)?;
            }
            Some(here)
        }
        "simulations" => {
            let (name, index) = step(steps.next()?)?;
            let simulation = root
                .child("simulations")?
                .children_named(&name)
                .nth(index)?;
            let (name, index) = step(steps.next()?)?;
            let inside = simulation
                .elements()
                .nth(index)
                .filter(|child| child.name == name)?;
            match steps.next() {
                None => Some(inside),
                Some(text) => {
                    let (name, index) = step(text.strip_prefix('@')?)?;
                    let tag = inside
                        .elements()
                        .nth(index)
                        .filter(|child| child.name == name)?;
                    steps.next().is_none().then_some(tag)
                }
            }
        }
        text => {
            let (name, index) = step(text)?;
            let section = root
                .elements()
                .nth(index)
                .filter(|child| child.name == name)?;
            steps.next().is_none().then_some(section)
        }
    }
}
