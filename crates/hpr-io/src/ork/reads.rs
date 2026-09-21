//! Which child tags the readers consult, so that [`super::extensions`] can keep the rest.
//!
//! [`super::design`] records while it reads the rocket: every lookup of a child tag by name, on
//! the element it was looked up in, whether the tag was there or not. A tag of a part the walk read
//! that no reader ever asked for is one hpr does not model, and is kept whole.
//!
//! Recording is scoped to one call and keyed by the element's address, which is only compared
//! within that call, while the document it points into is borrowed.

use std::cell::RefCell;
use std::collections::BTreeSet;

use super::document::Element;

/// The lookups recorded: an element's address and the tag name asked for.
pub(super) type Reads = BTreeSet<(usize, String)>;

thread_local! {
    static READS: RefCell<Option<Reads>> = const { RefCell::new(None) };
}

/// Records that a reader asked `element` for its child tag `name`, when a recording is on.
pub(super) fn note(element: &Element, name: &str) {
    READS.with(|reads| {
        if let Some(reads) = reads.borrow_mut().as_mut() {
            reads.insert((address(element), name.to_owned()));
        }
    });
}

/// Records that a reader asked `element` for its attribute `name`, when a recording is on.
pub(super) fn note_attribute(element: &Element, name: &str) {
    note(element, &attribute_key(name));
}

/// Whether a reader asked `element` for its child tag `name` in `reads`.
pub(super) fn asked(reads: &Reads, element: &Element, name: &str) -> bool {
    reads.contains(&(address(element), name.to_owned()))
}

/// Whether a reader asked `element` for its attribute `name` in `reads`.
pub(super) fn asked_attribute(reads: &Reads, element: &Element, name: &str) -> bool {
    asked(reads, element, &attribute_key(name))
}

/// An attribute's key in the recording: `@` and its name, which no tag name can be.
fn attribute_key(name: &str) -> String {
    format!("@{name}")
}

/// Puts back the recording that was on before, even if the read panics.
struct Restore(Option<Option<Reads>>);

impl Drop for Restore {
    fn drop(&mut self) {
        if let Some(before) = self.0.take() {
            READS.with(|reads| *reads.borrow_mut() = before);
        }
    }
}

/// Runs `read`, recording every lookup it makes, and gives both back.
pub(super) fn recording<T>(read: impl FnOnce() -> T) -> (T, Reads) {
    let before = READS.with(|reads| reads.borrow_mut().replace(Reads::new()));
    let restore = Restore(Some(before));
    let value = read();
    let recorded = READS.with(|reads| reads.borrow_mut().take().unwrap_or_default());
    drop(restore);
    (value, recorded)
}

fn address(element: &Element) -> usize {
    std::ptr::from_ref(element).addr()
}
