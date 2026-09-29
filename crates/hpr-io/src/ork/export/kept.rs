//! Putting what hpr keeps but does not model back where it was: the `x-openrocket` extension's
//! parts, sections, tags and attributes.
//!
//! A kept part goes back at its place among its parent's parts, since the order of parts is where
//! they stack. A section or a tag goes back as the next of its name in its parent — of its name
//! and configuration, for a tag that belongs to one — which is where its path's count says it was
//! ([`super::super::extensions`]); it is written before the parent's `<subcomponents>`, where
//! OpenRocket writes its tags. An attribute goes back on its element.
//!
//! **What is kept wins over what is written.** A tag or attribute is kept when no reader asked for
//! it, or when a reader asked and then dropped or simplified what it says, so that the design
//! does not hold it: a rail button's screw height, a ring's count above one, a word the reader has
//! no meaning for. The writer may still write a tag of that name from what the design holds
//! instead — a count of one, a square fin section — and that one is taken out, every copy of it,
//! before the kept ones go back, so the file says what it said before and says it once. A kept
//! attribute likewise replaces one the writer wrote.
//!
//! Something whose path no longer leads anywhere — its parent is not in the written document —
//! is left out with a warning rather than put somewhere it was not.

use super::super::document::{Document, Element, Node};
use super::super::extensions::{OpenRocketExtension, tag_key};
use super::super::warning::{Warning, WarningKind};
use super::xml::{Build as _, element};

/// Puts everything in `kept` back into `document`, and says what could not be.
pub(super) fn splice(document: &mut Document, kept: &OpenRocketExtension) -> Vec<Warning> {
    let mut warnings = Vec::new();
    let mut lost = |at: &str, what: &str| {
        warnings.push(Warning::new(
            at,
            WarningKind::Dropped,
            format!(
                "{what} kept from the file has no place in the written design; it was left out"
            ),
        ));
    };
    // The parts first: they are the steps of every other path.
    for part in &kept.parts {
        if !insert_part(document, &part.at, &part.element) {
            lost(&part.at, "a part");
        }
    }
    for kept in kept.sections.iter().chain(&kept.tags) {
        if !insert_tag(document, &kept.at, &kept.element) {
            lost(&kept.at, "a tag");
        }
    }
    for attribute in &kept.attributes {
        match element_at_mut(document, &attribute.at) {
            Some(on) => {
                match on
                    .attributes
                    .iter_mut()
                    .find(|(name, _)| *name == attribute.name)
                {
                    Some((_, value)) => value.clone_from(&attribute.value),
                    None => {
                        on.with_attribute(&attribute.name, attribute.value.clone());
                    }
                }
            }
            None => lost(&attribute.at, "an attribute"),
        }
    }
    warnings
}

/// A path's last step, and the path before it.
fn split(at: &str) -> Option<(&str, &str)> {
    at.rsplit_once('/')
}

/// A step's name and count: `bodytube[1]` gives `("bodytube", 1)`.
fn step(text: &str) -> Option<(&str, usize)> {
    let (name, rest) = text.split_once('[')?;
    let index = rest.strip_suffix(']')?.parse().ok()?;
    Some((name, index))
}

/// Puts a part back at its place among the parts of the element at its parent's path.
fn insert_part(document: &mut Document, at: &str, part: &Element) -> bool {
    let Some((parent, last)) = split(at) else {
        return false;
    };
    let Some((name, index)) = step(last) else {
        return false;
    };
    if name != part.name {
        return false;
    }
    let Some(parent) = element_at_mut(document, parent) else {
        return false;
    };
    let holder = match parent
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(e) if e.name == "subcomponents"))
    {
        Some(position) => position,
        None => {
            parent.push(element("subcomponents"));
            parent.children.len() - 1
        }
    };
    let Node::Element(holder) = &mut parent.children[holder] else {
        return false;
    };
    // The place among the element children, whatever text sits between them.
    let place = holder
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| matches!(child, Node::Element(_)))
        .nth(index)
        .map_or(holder.children.len(), |(place, _)| place);
    let before = holder.elements().count();
    if index > before {
        return false;
    }
    holder.children.insert(place, Node::Element(part.clone()));
    true
}

/// Puts a section or a tag back as the next of its name in the element at its parent's path: the
/// next of its name and configuration, for a tag that belongs to one.
fn insert_tag(document: &mut Document, at: &str, tag: &Element) -> bool {
    let Some((parent, last)) = split(at) else {
        return false;
    };
    let (key, index) = match last.strip_prefix('@') {
        Some(last) => match step(last) {
            Some((key, index)) if key == tag_key(tag) => (key.to_owned(), index),
            _ => return false,
        },
        // A section's step counts among its name alone.
        None => match step(last) {
            Some((name, index)) if name == tag.name => (name.to_owned(), index),
            _ => return false,
        },
    };
    let tagged = last.starts_with('@');
    let same = |element: &Element| {
        if tagged {
            tag_key(element) == key
        } else {
            element.name == key
        }
    };
    let Some(parent) = element_at_mut(document, parent) else {
        return false;
    };
    // Every copy of a name is kept or none is, so the first kept copy takes the place of what the
    // writer wrote of that name (and configuration).
    if index == 0 {
        parent
            .children
            .retain(|child| !matches!(child, Node::Element(e) if same(e)));
    }
    // Its count says how many of its key came before it, and they have all been put back by now:
    // the extension lists them in document order.
    if parent.elements().filter(|e| same(e)).count() != index {
        return false;
    }
    let place = parent
        .children
        .iter()
        .position(|child| matches!(child, Node::Element(e) if e.name == "subcomponents"))
        .unwrap_or(parent.children.len());
    parent.children.insert(place, Node::Element(tag.clone()));
    true
}

/// The element of `document` at `at`, as [`super::super::element_at`] finds it, to change.
fn element_at_mut<'a>(document: &'a mut Document, at: &str) -> Option<&'a mut Element> {
    let mut steps = at.split('/');
    if steps.next() != Some("openrocket") {
        return None;
    }
    let mut here = &mut document.root;
    let Some(first) = steps.next() else {
        return Some(here);
    };
    // `rocket` and `simulations` are the first of their name, and their step says no count;
    // a section's step does.
    let (name, index) = step(first).unwrap_or((first, 0));
    here = nth_named(here, name, index)?;
    let rocket = first == "rocket";
    let mut in_tags = false;
    for text in steps {
        if let Some(text) = text.strip_prefix('@') {
            in_tags = true;
            let (key, index) = step(text)?;
            here = here
                .children
                .iter_mut()
                .filter_map(|child| match child {
                    Node::Element(element) if tag_key(element) == key => Some(element),
                    _ => None,
                })
                .nth(index)?;
        } else if rocket && !in_tags {
            let (name, index) = step(text)?;
            let holder = nth_named(here, "subcomponents", 0)?;
            here = holder
                .children
                .iter_mut()
                .filter_map(|child| match child {
                    Node::Element(element) => Some(element),
                    Node::Text { .. } => None,
                })
                .nth(index)
                .filter(|child| child.name == name)?;
        } else if rocket {
            // Only tags follow a tag.
            return None;
        } else {
            // Inside `<simulations>`: a simulation, then a section of it, each counted by name.
            let (name, index) = step(text)?;
            here = nth_named(here, name, index)?;
        }
    }
    Some(here)
}

/// The `index`-th child element of `element` called `name`.
fn nth_named<'a>(element: &'a mut Element, name: &str, index: usize) -> Option<&'a mut Element> {
    element
        .children
        .iter_mut()
        .filter_map(|child| match child {
            Node::Element(element) if element.name == name => Some(element),
            _ => None,
        })
        .nth(index)
}
