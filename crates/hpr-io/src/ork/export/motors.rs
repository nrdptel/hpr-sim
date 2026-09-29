//! The motors: each configuration, and each mount's motors, ignition and overhang, written as
//! [`super::super::motors`] reads them.

use hpr_design::tree::Component;

use super::super::Design;
use super::super::document::Element;

/// The tags the rocket itself carries for its motors: one `<motorconfiguration>` per
/// configuration.
pub(super) fn rocket(design: &Design) -> Vec<Element> {
    let _ = design;
    Vec::new()
}

/// The `<motormount>` of `component`, if it is a motor mount.
pub(super) fn mount(design: &Design, component: &Component) -> Option<Element> {
    let _ = (design, component);
    None
}
