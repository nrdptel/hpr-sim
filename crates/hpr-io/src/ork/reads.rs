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

/// Whether a reader asked `element` for `name` in `reads`.
pub(super) fn asked(reads: &Reads, element: &Element, name: &str) -> bool {
    reads.contains(&(address(element), name.to_owned()))
}

/// Runs `read`, recording every lookup it makes, and gives both back.
pub(super) fn recording<T>(read: impl FnOnce() -> T) -> (T, Reads) {
    let before = READS.with(|reads| reads.borrow_mut().replace(Reads::new()));
    let value = read();
    let recorded = READS.with(|reads| {
        let mut reads = reads.borrow_mut();
        let recorded = reads.take().unwrap_or_default();
        *reads = before;
        recorded
    });
    (value, recorded)
}

fn address(element: &Element) -> usize {
    std::ptr::from_ref(element).addr()
}
