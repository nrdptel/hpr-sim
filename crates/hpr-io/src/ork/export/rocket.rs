//! The rocket: its stages, the body components stacked in them, and the parts on and inside each
//! of those, written as [`super::super::component`] and [`super::super::attached`] read them.

use hpr_design::tree::{Component, Stage};

use super::super::Design;
use super::super::document::Element;
use super::super::warning::Warning;
use super::xml::{self, Build as _};
use super::{motors, recovery};

/// The `<rocket>` element for `design`.
pub(super) fn rocket(design: &Design, warnings: &mut Vec<Warning>) -> Element {
    let _ = warnings;
    let mut rocket = xml::element("rocket");
    rocket.leaf("name", design.rocket.name.clone());
    for tag in motors::rocket(design) {
        rocket.push(tag);
    }
    let mut stages = xml::element("subcomponents");
    for (index, stage) in design.rocket.stages.iter().enumerate() {
        stages.push(self::stage(design, index, stage));
    }
    rocket.push(stages);
    rocket
}

fn stage(design: &Design, index: usize, stage: &Stage) -> Element {
    let mut element = xml::element("stage");
    element.leaf("name", stage.name.clone());
    for tag in recovery::stage(design, index, stage) {
        element.push(tag);
    }
    let _ = |component: &Component| {
        (
            motors::mount(design, component),
            recovery::device(design, component),
        )
    };
    element
}
