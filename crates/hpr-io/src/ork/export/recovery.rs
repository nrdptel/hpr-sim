//! When recovery devices open and stages separate, written as [`super::super::recovery`] reads
//! them.

use hpr_design::tree::{Component, Stage};

use super::super::Design;
use super::super::document::Element;

/// The tags a parachute or streamer carries for when it opens, and its drag coefficient.
pub(super) fn device(design: &Design, component: &Component) -> Vec<Element> {
    let _ = (design, component);
    Vec::new()
}

/// The tags stage `index` carries for when it separates.
pub(super) fn stage(design: &Design, index: usize, stage: &Stage) -> Vec<Element> {
    let _ = (design, index, stage);
    Vec::new()
}
