//! Builders shared by the tests.

use hpr_motor::{SolidMotor, ThrustCurve};

use crate::{
    BodyTube, CenteringRing, Component, Configuration, FinCrossSection, FinPlanform, FinSet,
    InnerTube, LaunchLug, MassComponent, Material, MotorMount, MountedMotor, NoseCone, NoseShape,
    Overrides, Packing, Parachute, Part, Position, ReferenceDiameter, Rocket, Stage, Wall,
};

pub(crate) fn cardboard() -> Material {
    Material::bulk("cardboard", 790.0)
}

/// A body component.
pub(crate) fn body(id: &str, part: Part) -> Component {
    Component {
        id: id.to_owned(),
        name: String::new(),
        part,
        position: None,
        auto: Vec::new(),
        motor_mount: None,
        finish: None,
        overrides: Overrides::default(),
        overrides_include_children: false,
        children: Vec::new(),
    }
}

/// An attached part.
pub(crate) fn attached(id: &str, part: Part, position: Position) -> Component {
    Component {
        position: Some(position),
        ..body(id, part)
    }
}

pub(crate) fn top(aft_offset_m: f64) -> Position {
    Position::Top { aft_offset_m }
}

pub(crate) fn bottom(aft_offset_m: f64) -> Position {
    Position::Bottom { aft_offset_m }
}

/// A conical nose cone with a 2 mm PLA wall.
pub(crate) fn nose(length_m: f64, base_radius_m: f64) -> Part {
    Part::NoseCone(NoseCone {
        shape: NoseShape::Conical {},
        length_m,
        base_radius_m,
        wall: Wall::Shell { thickness_m: 0.002 },
        shoulder: None,
        material: Material::bulk("PLA", 1240.0),
    })
}

pub(crate) fn tube(length_m: f64, outer_radius_m: f64, thickness_m: f64) -> Part {
    Part::BodyTube(BodyTube {
        length_m,
        outer_radius_m,
        thickness_m,
        material: cardboard(),
    })
}

pub(crate) fn inner_tube(length_m: f64, outer_radius_m: f64, thickness_m: f64) -> Part {
    Part::InnerTube(InnerTube {
        length_m,
        outer_radius_m,
        thickness_m,
        radial_offset_m: 0.0,
        angle_rad: 0.0,
        material: cardboard(),
    })
}

pub(crate) fn ring(length_m: f64, outer_radius_m: f64, inner_radius_m: f64) -> Part {
    Part::CenteringRing(CenteringRing {
        length_m,
        outer_radius_m,
        inner_radius_m,
        material: Material::bulk("plywood", 630.0),
    })
}

/// Three trapezoidal plywood fins.
pub(crate) fn fins(root_chord_m: f64, span_m: f64) -> Part {
    Part::FinSet(FinSet {
        count: 3,
        planform: FinPlanform::Trapezoidal {
            root_chord_m,
            tip_chord_m: 0.5 * root_chord_m,
            span_m,
            sweep_m: 0.5 * root_chord_m,
        },
        thickness_m: 0.003,
        cross_section: FinCrossSection::Square,
        tab: None,
        cant_rad: 0.0,
        base_angle_rad: 0.0,
        material: Material::bulk("plywood", 630.0),
    })
}

pub(crate) fn mass_component(mass_kg: f64, length_m: f64, radius_m: f64) -> Part {
    Part::MassComponent(MassComponent {
        mass_kg,
        packing: Packing {
            length_m,
            radius_m,
            radial_offset_m: 0.0,
            angle_rad: 0.0,
        },
    })
}

pub(crate) fn stage(id: &str, components: Vec<Component>) -> Stage {
    Stage {
        id: id.to_owned(),
        name: String::new(),
        components,
        overrides: Overrides::default(),
    }
}

pub(crate) fn rocket(stages: Vec<Stage>) -> Rocket {
    Rocket {
        name: "test".to_owned(),
        stages,
        reference_diameter: ReferenceDiameter::default(),
        configurations: Vec::new(),
    }
}

/// A motor of `diameter_m` by `length_m` built from its envelope: 1 kg loaded with 0.5 kg of
/// propellant, burning 200 N for 1 s.
pub(crate) fn motor(mount: &str, diameter_m: f64, length_m: f64) -> MountedMotor {
    let curve = ThrustCurve::new(vec![0.01, 0.99, 1.0], vec![200.0, 200.0, 0.0]).unwrap();
    MountedMotor {
        mount: mount.to_owned(),
        designation: "test".to_owned(),
        diameter_m,
        length_m,
        motor: SolidMotor::from_envelope(curve, diameter_m, length_m, 0.5, 1.0).unwrap(),
        delay: None,
    }
}

/// A 54 mm, single-stage, three-fin rocket with a 38 mm motor mount, two automatic centering
/// rings, a parachute packed to the tube's bore, a launch lug and one configuration, `"main"`.
pub(crate) fn three_fin_rocket() -> Rocket {
    let mut airframe = body("airframe", tube(0.8, 0.027, 0.0015));
    let mut mount = attached("mmt", inner_tube(0.3, 0.020, 0.001), bottom(0.0));
    mount.motor_mount = Some(MotorMount { overhang_m: 0.01 });
    let mut fore_ring = attached("ring-fore", ring(0.006, 0.0, 0.0), bottom(-0.25));
    fore_ring.auto = vec![
        crate::AutoDimension::OuterRadius,
        crate::AutoDimension::InnerRadius,
    ];
    let mut aft_ring = attached("ring-aft", ring(0.006, 0.0, 0.0), bottom(-0.02));
    aft_ring.auto = fore_ring.auto.clone();
    let mut chute = attached(
        "chute",
        Part::Parachute(Parachute {
            diameter_m: 0.6,
            canopy_material: Material::surface("ripstop nylon", 0.067),
            line_count: 6,
            line_length_m: 0.6,
            line_material: Material::line("nylon line", 0.0015),
            packing: Packing {
                length_m: 0.08,
                radius_m: 0.0,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
        }),
        top(0.05),
    );
    chute.auto = vec![crate::AutoDimension::PackedRadius];
    airframe.children = vec![
        mount,
        fore_ring,
        aft_ring,
        attached("fins", fins(0.1, 0.06), bottom(0.0)),
        chute,
        attached(
            "lug",
            Part::LaunchLug(LaunchLug {
                length_m: 0.05,
                outer_radius_m: 0.003,
                thickness_m: 0.0005,
                angle_rad: 0.0,
                count: 1,
                spacing_m: 0.0,
                material: cardboard(),
            }),
            Position::Middle { aft_offset_m: 0.0 },
        ),
    ];
    let mut nose_cone = body("nose", nose(0.2, 0.0));
    nose_cone.auto = vec![crate::AutoDimension::BaseRadius];
    let mut design = rocket(vec![stage("sustainer", vec![nose_cone, airframe])]);
    design.configurations = vec![Configuration {
        id: "main".to_owned(),
        name: String::new(),
        motors: vec![motor("mmt", 0.038, 0.2)],
    }];
    design
}
