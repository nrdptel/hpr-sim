//! The simulations OpenRocket last ran, with their conditions and results, written as
//! [`super::super::simulations`] reads them.

use super::super::Design;
use super::super::document::Element;
use super::super::warning::Warning;

/// The `<simulations>` element, or `None` when the design has none.
pub(super) fn simulations(design: &Design, warnings: &mut Vec<Warning>) -> Option<Element> {
    let _ = (design, warnings);
    None
}
