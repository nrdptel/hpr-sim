//! Design builders shared by the tests.

use hpr_design::{
    BodyTube, Component, FinCrossSection, FinPlanform, FinSet, Material, NoseCone, NoseShape,
    Overrides, Part, Position, ReferenceDiameter, Rocket, Stage, Transition, Wall,
};

pub(crate) fn material() -> Material {
    Material::bulk("test", 1000.0)
}

pub(crate) fn component(id: &str, part: Part, position: Option<Position>) -> Component {
    Component {
        id: id.to_owned(),
        name: String::new(),
        part,
        position,
        auto: Vec::new(),
        motor_mount: None,
        finish: None,
        overrides: Overrides::default(),
        overrides_include_children: false,
        children: Vec::new(),
    }
}

pub(crate) fn nose(shape: NoseShape, length_m: f64, radius_m: f64) -> Part {
    Part::NoseCone(NoseCone {
        shape,
        length_m,
        base_radius_m: radius_m,
        wall: Wall::Filled {},
        shoulder: None,
        material: material(),
    })
}

/// A body tube, or a conical transition when the radii differ.
pub(crate) fn body_part(length_m: f64, fore_radius_m: f64, aft_radius_m: f64) -> Part {
    if fore_radius_m == aft_radius_m {
        Part::BodyTube(BodyTube {
            length_m,
            outer_radius_m: fore_radius_m,
            thickness_m: 0.1 * fore_radius_m,
            material: material(),
        })
    } else {
        Part::Transition(Transition {
            shape: NoseShape::Conical {},
            clipped: false,
            length_m,
            fore_radius_m,
            aft_radius_m,
            wall: Wall::Filled {},
            fore_shoulder: None,
            aft_shoulder: None,
            material: material(),
        })
    }
}

pub(crate) fn fin_set(count: u32, planform: FinPlanform) -> Part {
    Part::FinSet(FinSet {
        count,
        planform,
        thickness_m: 0.001,
        cross_section: FinCrossSection::Square,
        tab: None,
        cant_rad: 0.0,
        base_angle_rad: 0.0,
        material: material(),
    })
}

pub(crate) fn one_stage(components: Vec<Component>, reference: ReferenceDiameter) -> Rocket {
    Rocket {
        name: String::new(),
        stages: vec![Stage {
            id: "stage".to_owned(),
            name: String::new(),
            components,
            overrides: Overrides::default(),
        }],
        reference_diameter: reference,
        configurations: Vec::new(),
    }
}

/// A 54 mm rocket: a tangent-ogive nose, a body tube, a conical boattail and a tail tube carrying
/// `count` trapezoidal fins, with the reference diameter the widest body's.
pub(crate) fn finned_rocket(count: u32) -> Rocket {
    let mut tail = component("tail", body_part(0.3, 0.022, 0.022), None);
    tail.children = vec![component(
        "fins",
        fin_set(
            count,
            FinPlanform::Trapezoidal {
                root_chord_m: 0.12,
                tip_chord_m: 0.05,
                span_m: 0.06,
                sweep_m: 0.07,
            },
        ),
        Some(Position::Bottom { aft_offset_m: 0.0 }),
    )];
    one_stage(
        vec![
            component(
                "nose",
                nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027),
                None,
            ),
            component("body", body_part(0.7, 0.027, 0.027), None),
            component("boattail", body_part(0.05, 0.027, 0.022), None),
            tail,
        ],
        ReferenceDiameter::Maximum {},
    )
}

/// A design committed under `validation/designs/`, by file name.
pub(crate) fn committed_design(name: &str) -> Rocket {
    let text = match name {
        "rocketpy-calisto-tests-motor-at-minus-1.373.json" => include_str!(
            "../../../validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
        ),
        "rocketpy-calisto-getting-started-motor-at-minus-1.255.json" => include_str!(
            "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
        ),
        "rocketpy-juno-iii.json" => {
            include_str!("../../../validation/designs/rocketpy-juno-iii.json")
        }
        "rocketpy-valetudo.json" => {
            include_str!("../../../validation/designs/rocketpy-valetudo.json")
        }
        "rocketpy-cavour.json" => {
            include_str!("../../../validation/designs/rocketpy-cavour.json")
        }
        "wind-tunnel-arcas-robin-short.json" => {
            include_str!("../../../validation/designs/wind-tunnel-arcas-robin-short.json")
        }
        "wind-tunnel-arcas-robin-long.json" => {
            include_str!("../../../validation/designs/wind-tunnel-arcas-robin-long.json")
        }
        "mil-hdbk-762-sample-rocket.json" => {
            include_str!("../../../validation/designs/mil-hdbk-762-sample-rocket.json")
        }
        other => panic!("no committed design {other}"),
    };
    serde_json::from_str(text).unwrap()
}
