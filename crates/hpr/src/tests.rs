//! The builder against the design tree it makes, and its refusals.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests state their expectations by unwrapping and panicking"
)]

use hpr_design::{
    AutoDimension, BodyTube, Component, Configuration, DesignError, FinCrossSection, FinPlanform,
    FinSet, Ignition, InnerTube, MassComponent, MotorMount, MountedMotor, NoseCone, Overrides,
    Packing, Part, ReferenceDiameter, Shoulder, Stage, Wall,
};
use hpr_motor::{Delay, MotorError};
use hpr_sim::{EventKind, FlightSettings, Rail, Simulation};

use crate::rocket::{Fins, Mass, MotorTube, Nose, Transition, Tube, material};
use crate::{
    CanopyType, Device, DeviceDrag, Environment, Error, Flight, Motor, NoseShape, Order, Position,
    Rocket, Trigger,
};

/// The rocket of the example `own_rocket` in `hpr-sim`, built part by part with the builder.
fn built() -> Rocket {
    let mut rocket = Rocket::new("My 54 mm rocket", 0.0563).unwrap();
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.22,
                0.0015,
                material("abs").unwrap(),
            )
            .with_capped_shoulder(0.06, 0.0015),
        )
        .unwrap()
        .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic").unwrap()))
        .unwrap()
        .add_motor_tube(
            MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic").unwrap())
                .with_overhang_m(0.005),
        )
        .unwrap()
        .add_fins(
            Fins::new(
                3,
                trapezoid([0.1, 0.04, 0.045, 0.05]),
                0.003175,
                material("birch_plywood").unwrap(),
            )
            .with_cross_section(FinCrossSection::Rounded),
        )
        .unwrap()
        .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }).packed(0.15, 0.05))
        .unwrap()
        .set_motor(
            Motor::from_catalog("168H54-10A")
                .unwrap()
                .with_delay_s(10.0)
                .unwrap(),
        )
        .unwrap()
        .add_parachute(parachute());
    rocket
}

/// A trapezoidal planform from its root chord, tip chord, span and sweep, m.
fn trapezoid([root_chord_m, tip_chord_m, span_m, sweep_m]: [f64; 4]) -> FinPlanform {
    FinPlanform::Trapezoidal {
        root_chord_m,
        tip_chord_m,
        span_m,
        sweep_m,
    }
}

/// The example's parachute, opened by the motor's ejection charge.
fn parachute() -> Device {
    Device::new(
        "parachute",
        DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
        Trigger::MotorDelay { motor: 0 },
    )
}

/// A node of the design tree, as the example makes one.
fn component(id: &str, part: Part, position: Option<Position>) -> Component {
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

/// The same rocket as the example `own_rocket` builds it, as struct literals, with its ids.
fn by_hand() -> hpr_design::Rocket {
    let mut nose = component(
        "nose",
        Part::NoseCone(NoseCone {
            shape: NoseShape::Ogive { radius_ratio: 1.0 },
            length_m: 0.22,
            base_radius_m: 0.0,
            wall: Wall::Shell {
                thickness_m: 0.0015,
            },
            shoulder: Some(Shoulder {
                length_m: 0.06,
                outer_radius_m: 0.0,
                thickness_m: 0.0015,
                capped: true,
            }),
            material: material("abs").unwrap(),
        }),
        None,
    );
    nose.auto = vec![AutoDimension::BaseRadius, AutoDimension::ShoulderRadius];
    let mut airframe = component(
        "airframe",
        Part::BodyTube(BodyTube {
            length_m: 0.9,
            outer_radius_m: 0.02815,
            thickness_m: 0.00115,
            material: material("kraft_phenolic").unwrap(),
        }),
        None,
    );
    let mut mount = component(
        "motor-mount",
        Part::InnerTube(InnerTube {
            length_m: 0.2,
            outer_radius_m: 0.0155,
            thickness_m: 0.001,
            radial_offset_m: 0.0,
            angle_rad: 0.0,
            material: material("kraft_phenolic").unwrap(),
            cluster_m: Vec::new(),
        }),
        Some(Position::Bottom { aft_offset_m: 0.0 }),
    );
    mount.motor_mount = Some(MotorMount { overhang_m: 0.005 });
    let fins = component(
        "fins",
        Part::FinSet(FinSet {
            count: 3,
            planform: FinPlanform::Trapezoidal {
                root_chord_m: 0.1,
                tip_chord_m: 0.04,
                span_m: 0.045,
                sweep_m: 0.05,
            },
            thickness_m: 0.003175,
            cross_section: FinCrossSection::Rounded,
            tab: None,
            fillet: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: material("birch_plywood").unwrap(),
        }),
        Some(Position::Bottom { aft_offset_m: 0.0 }),
    );
    let bay = component(
        "recovery-bay",
        Part::MassComponent(MassComponent {
            mass_kg: 0.2,
            packing: Packing {
                length_m: 0.15,
                radius_m: 0.025,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
        }),
        Some(Position::Top { aft_offset_m: 0.07 }),
    );
    airframe.children = vec![mount, fins, bay];
    let catalog = hpr_motor::Catalog::bundled().unwrap();
    let entry = catalog.find("168H54-10A").next().unwrap();
    hpr_design::Rocket {
        name: "My 54 mm rocket".to_owned(),
        stages: vec![Stage {
            id: "sustainer".to_owned(),
            name: String::new(),
            components: vec![nose, airframe],
            overrides: Overrides::default(),
        }],
        reference_diameter: ReferenceDiameter::Maximum {},
        configurations: vec![Configuration {
            id: "h54".to_owned(),
            name: String::new(),
            motors: vec![MountedMotor {
                mount: "motor-mount".to_owned(),
                designation: entry.designation.clone(),
                diameter_m: entry.diameter_mm / 1000.0,
                length_m: entry.length_mm / 1000.0,
                motor: entry.bundled_motor().unwrap(),
                delay: Some(Delay::Seconds(10.0)),
                ignition: Ignition::Launch,
                failed_tubes: Vec::new(),
            }],
        }],
    }
}

/// Spaceport America's site, 1,400 m up, in calm air.
fn environment() -> Environment {
    Environment::new(32.99, -106.97, 1400.0).unwrap()
}

/// The builder's rocket is the example's, placed and flown: the same mass properties through the
/// burn, the same centre of pressure, and the same flight, bit for bit. Only the ids differ, and
/// the nose's base radius, which the example leaves automatic and the builder gives.
#[test]
fn the_builder_makes_the_rocket_own_rocket_builds_by_hand() {
    let rocket = built();
    let hand = by_hand();
    let (a, b) = (rocket.assemble().unwrap(), hand.assemble("h54").unwrap());
    for t in [0.0, 0.5, 1.0, 2.0, 3.0, 10.0] {
        assert_eq!(a.mass_properties(t), b.mass_properties(t), "t = {t} s");
    }
    assert_eq!(a.dry_mass_properties(), b.dry_mass_properties());
    assert_eq!(a.layout.length_m, b.layout.length_m);
    assert_eq!(a.layout.reference_diameter_m, b.layout.reference_diameter_m);

    // The example prints this margin: 1.92 calibres at liftoff, CP 0.779 m from the nose.
    let margin = rocket.margin(0.0, 0.3).unwrap();
    assert_eq!(format!("{:.2}", margin.margin_cal.unwrap()), "1.92");
    assert_eq!(format!("{:.3}", margin.cp_station_m.unwrap()), "0.779");

    let flight = Flight::builder(&rocket, &environment(), 1.8).fly().unwrap();
    let by_hand = Simulation::new(
        &hand,
        "h54",
        environment().sim().clone(),
        Rail::vertical(1.8),
        FlightSettings::default(),
    )
    .unwrap()
    .with_recovery(vec![parachute()])
    .unwrap()
    .run(&mut ())
    .unwrap();
    assert_eq!(flight.result(), &by_hand);
    // And the example's printed apogee, 1144.5 m at 13.92 s.
    assert_eq!(format!("{:.1}", flight.apogee_m().unwrap()), "1144.5");
    assert_eq!(format!("{:.2}", flight.apogee_time_s().unwrap()), "13.92");
    let apogee = by_hand.event(EventKind::Apogee).unwrap().sample;
    assert_eq!(flight.apogee_m(), Some(apogee.height_above_ground_m));
}

/// The design the builder makes: its ids, the automatic shoulder radius, the motor in its tube.
#[test]
fn the_builder_names_its_parts_and_configuration() {
    let rocket = built();
    let design = rocket.design();
    let stage = &design.stages[0];
    let ids =
        |components: &[Component]| components.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&stage.components), ["nose", "tube"]);
    assert_eq!(
        ids(&stage.components[1].children),
        ["motor-tube", "fins", "mass"]
    );
    assert_eq!(stage.components[0].auto, [AutoDimension::ShoulderRadius]);
    assert_eq!(rocket.configuration_id(), Some("168H54-10A"));
    let motors = &design.configurations[0].motors;
    assert_eq!(motors.len(), 1);
    assert_eq!(motors[0].mount, "motor-tube");
    assert_eq!(motors[0].delay, Some(Delay::Seconds(10.0)));

    // A second part of a kind gets a numbered id; a transition takes the diameter before it.
    let mut stepped = Rocket::new("Stepped", 0.1).unwrap();
    let paper = material("kraft_phenolic").unwrap();
    stepped
        .add_tube(Tube::new(0.5, 0.002, paper.clone()))
        .unwrap()
        .add_transition(Transition::conical(0.1, 0.05, 0.002, paper.clone()))
        .unwrap()
        .add_tube(Tube::new(0.5, 0.002, paper))
        .unwrap();
    let components = &stepped.design().stages[0].components;
    assert_eq!(ids(components), ["tube", "transition", "tube-2"]);
    let Part::Transition(transition) = &components[1].part else {
        panic!("not a transition: {:?}", components[1].part);
    };
    assert_eq!(
        (transition.fore_radius_m, transition.aft_radius_m),
        (0.05, 0.025)
    );
    // The tube behind it takes its aft diameter.
    let Part::BodyTube(tube) = &components[2].part else {
        panic!("not a tube: {:?}", components[2].part);
    };
    assert_eq!(tube.outer_radius_m, 0.025);
}

/// Parts in an order the tree can't take, and a rocket read from a design, are refused.
#[test]
fn parts_out_of_order_are_refused() {
    let paper = material("kraft_phenolic").unwrap();
    let fins = Fins::new(3, trapezoid([0.1, 0.05, 0.05, 0.03]), 0.003, paper.clone());
    let mut rocket = Rocket::new("Out of order", 0.05).unwrap();
    assert!(matches!(
        rocket.add_fins(fins.clone()),
        Err(Error::Order(Order::NoTube))
    ));
    assert!(matches!(
        rocket.add_transition(Transition::conical(0.1, 0.03, 0.002, paper.clone())),
        Err(Error::Order(Order::NothingBeforeTransition))
    ));
    let motor = Motor::from_catalog("F52C").unwrap();
    assert!(matches!(
        rocket.set_motor(motor.clone()),
        Err(Error::Order(Order::NoMotorTube))
    ));
    rocket
        .add_tube(Tube::new(0.5, 0.001, paper.clone()))
        .unwrap();
    let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
    assert!(matches!(
        rocket.add_nose(Nose::solid(ogive, 0.1, paper.clone())),
        Err(Error::Order(Order::NoseNotFirst))
    ));
    let tube = MotorTube::new(0.1, 0.029, 0.001, paper.clone());
    rocket.add_motor_tube(tube.clone()).unwrap();
    assert!(matches!(
        rocket.add_motor_tube(tube),
        Err(Error::Order(Order::SecondMotorTube))
    ));
    // No motor yet: nothing to fly or to weigh.
    assert!(matches!(rocket.assemble(), Err(Error::NoMotor)));
    assert!(matches!(
        Flight::builder(&rocket, &environment(), 1.0).fly(),
        Err(Error::NoMotor)
    ));

    let mut read = Rocket::from_design(by_hand(), "h54").unwrap();
    assert!(matches!(
        read.add_tube(Tube::new(0.5, 0.001, paper)),
        Err(Error::Order(Order::ReadFromDesign))
    ));
    assert!(matches!(
        Rocket::from_design(by_hand(), "j350"),
        Err(Error::NoSuchConfiguration(id)) if id == "j350"
    ));
    // A read design flies as it is, with the recovery devices added to it.
    read.add_parachute(parachute());
    let flight = Flight::builder(&read, &environment(), 1.8).fly().unwrap();
    assert_eq!(format!("{:.1}", flight.apogee_m().unwrap()), "1144.5");
}

/// Motors from the catalog and from a `.eng` file, and the names that find none or several.
#[test]
fn motors_come_from_the_catalog_or_a_file() {
    let catalog = Motor::from_catalog("h54").unwrap();
    assert_eq!(catalog.designation(), "168H54-10A");
    assert_eq!((catalog.diameter_m(), catalog.length_m()), (0.029, 0.187));
    assert_eq!(catalog.delay(), None);
    assert!(matches!(
        Motor::from_catalog("Z9000"),
        Err(Error::NoSuchMotor(name)) if name == "Z9000"
    ));
    assert!(matches!(
        catalog.clone().with_delay_s(-1.0),
        Err(Error::Domain { what: "motor delay, s", value }) if value == -1.0
    ));
    assert_eq!(
        catalog.clone().with_delay(Delay::Plugged).unwrap().delay(),
        Some(Delay::Plugged)
    );
    assert!(matches!(
        catalog.clone().with_delay(Delay::Seconds(f64::NAN)),
        Err(Error::Domain {
            what: "motor delay, s",
            ..
        })
    ));
    // A common name two motors share is refused, both listed, not guessed.
    match Motor::from_catalog("I175") {
        Err(Error::AmbiguousMotor { name, candidates }) => {
            assert_eq!(name, "I175");
            assert_eq!(candidates.len(), 2, "{candidates:?}");
            assert_eq!(candidates, ["I175WS (AeroTech)", "411I175-14A (Cesaroni)"]);
            // Each is listed by a designation that finds it alone.
            for candidate in &candidates {
                let designation = candidate.split(' ').next().unwrap();
                assert_eq!(
                    Motor::from_catalog(designation).unwrap().designation(),
                    designation
                );
            }
        }
        other => panic!("expected an ambiguous name, got {other:?}"),
    }
    // The F15's curve is a RockSim `.rse` file.
    let f15 = Motor::from_catalog("F15").unwrap();
    assert_eq!((f15.diameter_m(), f15.length_m()), (0.029, 0.114));

    let text = include_str!("../../hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000863.eng");
    // Its header: `I377CT 38 292 8-18 0.25 0.56 Loki`, millimetres read as metres.
    let file = Motor::from_eng(text).unwrap();
    assert_eq!(file.designation(), "I377CT");
    assert_eq!((file.diameter_m(), file.length_m()), (0.038, 0.292));
    assert_eq!(file.delay(), None);
    let two = format!("{text}\n{text}");
    assert!(matches!(Motor::from_eng(&two), Err(Error::MotorCount(2))));

    let text = include_str!("../../hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000719.rse");
    // Its engine: `code="H170M" dia="38." len="191." initWt="330." propWt="182.5"`, sizes in
    // millimetres and masses in grams, converted to metres and kilograms.
    let file = Motor::from_rse(text).unwrap();
    assert_eq!(file.designation(), "H170M");
    assert_eq!((file.diameter_m(), file.length_m()), (0.038, 0.191));
    assert_eq!(file.delay(), None);
    let solid = file.solid_motor();
    let propellant = solid.propellant_initial_mass_kg();
    assert!((propellant - 0.1825).abs() < 1e-12, "{propellant}");
    let loaded = propellant + solid.dry().mass_kg;
    assert!((loaded - 0.330).abs() < 1e-12, "{loaded}");
    let hybrid = text.replacen("Type=\"reloadable\"", "Type=\" Hybrid\"", 1);
    assert_ne!(hybrid, text);
    assert!(matches!(
        Motor::from_rse(&hybrid),
        Err(Error::Motor(MotorError::Inconsistent(message))) if message.contains("hybrid")
    ));
    let engine = text.find("<engine ").unwrap();
    let end = text.find("</engine>").unwrap() + "</engine>".len();
    let two = format!("{}{}{}", &text[..end], &text[engine..end], &text[end..]);
    assert!(matches!(Motor::from_rse(&two), Err(Error::MotorCount(2))));
    assert!(matches!(
        Motor::new(" ", catalog.solid_motor().clone(), 0.029, 0.1),
        Err(Error::EmptyDesignation)
    ));
    assert!(matches!(
        Motor::new("x", catalog.solid_motor().clone(), 0.0, 0.1),
        Err(Error::Domain { what: "motor diameter, m", value }) if value == 0.0
    ));
}

/// The environment's site and wind, and what it refuses.
#[test]
fn the_environment_takes_a_site_and_a_wind() {
    let calm = environment();
    let site = calm.sim().site();
    assert_eq!(site.height_m, 1400.0);
    assert_eq!(site.latitude_rad, 32.99_f64.to_radians());
    let windy = calm.clone().with_constant_wind(5.0, 270.0).unwrap();
    // A west wind blows toward the east: +x in the launch frame.
    let wind = windy.sim().wind.wind(1500.0).unwrap().velocity_enu_m_s;
    assert!(
        (wind.x - 5.0).abs() < 1e-12 && wind.y.abs() < 1e-12,
        "{wind:?}"
    );
    assert!(matches!(
        Environment::new(91.0, 0.0, 0.0),
        Err(Error::Core(_))
    ));
    assert!(matches!(
        calm.clone().with_constant_wind(5.0, f64::NAN),
        Err(Error::Domain {
            what: "wind direction, degrees",
            ..
        })
    ));
    assert!(matches!(
        calm.with_constant_wind(-1.0, 0.0),
        Err(Error::Atmos(_))
    ));
}

/// A leaning rail: the rocket drifts the way it leans, and a flat rail is refused.
#[test]
fn the_rail_leans_where_it_is_headed() {
    let rocket = built();
    let environment = environment();
    let east = Flight::builder(&rocket, &environment, 1.8)
        .inclination_deg(80.0)
        .heading_deg(90.0)
        .fly()
        .unwrap();
    // East, and a little south: the Earth's rotation turns a flight to its right in the northern
    // hemisphere (0.16 m in 337 m here; the plumb line's curve alone gives 0.001 m).
    let apogee = east.result().event(EventKind::Apogee).unwrap().sample;
    let (east_m, north_m) = (apogee.cg_enu_m.x, apogee.cg_enu_m.y);
    assert!(
        east_m > 100.0 && north_m < -0.1 && north_m > -1e-3 * east_m,
        "{:?}",
        apogee.cg_enu_m
    );
    // Refused in the degrees they were given: flat, and past the vertical.
    for inclination_deg in [0.0, 95.0, f64::NAN] {
        let refused = Flight::builder(&rocket, &environment, 1.8)
            .inclination_deg(inclination_deg)
            .fly();
        assert!(
            matches!(
                refused,
                Err(Error::Domain { what: "rail inclination, degrees above the horizon", value })
                    if value.to_bits() == inclination_deg.to_bits()
            ),
            "{inclination_deg}: {refused:?}"
        );
    }
    // A whole rail keeps its own angles, exactly, unless the degrees are set too.
    let rail = Rail {
        azimuth_rad: 0.3,
        elevation_rad: 1.4,
        ..Rail::vertical(1.8)
    };
    let launch = Flight::builder(&rocket, &environment, 1.0).rail(rail);
    assert_eq!(launch.simulation().unwrap().rail(), rail);
    let steeper = launch.inclination_deg(89.0).simulation().unwrap();
    assert_eq!(steeper.rail().elevation_rad, 89.0_f64.to_radians());
    assert_eq!(steeper.rail().azimuth_rad, 0.3);
}

/// A packed mass's position places its packing's end, so packing it moves its centre by half its
/// length (the guide says so); placed by its middle, it doesn't move.
#[test]
fn packing_a_mass_moves_its_centre_unless_placed_by_its_middle() {
    // A tube, its motor, and the bay: the rocket's mass and the `z` of its centre of gravity.
    let with_bay = |bay: Mass| {
        let paper = material("kraft_phenolic").unwrap();
        let mut rocket = Rocket::new("Bay", 0.0563).unwrap();
        rocket
            .add_tube(Tube::new(0.9, 0.00115, paper.clone()))
            .unwrap()
            .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, paper))
            .unwrap()
            .add_mass(bay)
            .unwrap()
            .set_motor(Motor::from_catalog("H54").unwrap())
            .unwrap();
        let properties = rocket.mass_properties(0.0).unwrap();
        (properties.mass_kg, properties.cg_m.z)
    };
    let top = Position::Top { aft_offset_m: 0.07 };
    let (mass_kg, point_z) = with_bay(Mass::new(0.2, top));
    let (_, packed_z) = with_bay(Mass::new(0.2, top).packed(0.15, 0.05));
    // The bay's centre moves 0.075 m aft, and the rocket's by 0.2 × 0.075 / its mass.
    let shift_m = 0.2 * 0.075 / mass_kg;
    assert!(
        ((point_z - packed_z) - shift_m).abs() < 1e-12,
        "{} against {shift_m}",
        point_z - packed_z
    );
    let middle = Position::Middle { aft_offset_m: 0.0 };
    let (_, point_z) = with_bay(Mass::new(0.2, middle));
    let (_, packed_z) = with_bay(Mass::new(0.2, middle).packed(0.15, 0.05));
    assert!((point_z - packed_z).abs() < 1e-15, "{point_z} {packed_z}");
}

/// Weighing a rocket runs the checks a flight runs: a motor wider than its tube is refused by
/// both, not weighed by one and refused by the other.
#[test]
fn weighing_refuses_what_flying_refuses() {
    let mut rocket = built();
    rocket
        .set_motor(Motor::from_catalog("K400C").unwrap())
        .unwrap();
    let wider = |findings: &[hpr_design::Finding]| {
        findings
            .iter()
            .any(|finding| matches!(finding, hpr_design::Finding::MotorWiderThanMount { .. }))
    };
    match rocket.mass_properties(0.0) {
        Err(Error::DesignChecks(findings)) => assert!(wider(&findings), "{findings:?}"),
        other => panic!("expected the design checks, got {other:?}"),
    }
    assert!(matches!(
        rocket.margin(0.0, 0.3),
        Err(Error::DesignChecks(_))
    ));
    match Flight::builder(&rocket, &environment(), 1.8).fly() {
        Err(Error::Sim(hpr_sim::SimError::DesignChecks(findings))) => {
            assert!(wider(&findings), "{findings:?}");
        }
        other => panic!("expected the design checks, got {other:?}"),
    }
    assert!(matches!(
        built().mass_properties(-1.0),
        Err(Error::Domain {
            what: "time, s",
            ..
        })
    ));
}

/// `fly_with` shows the observer every step: a recorder's last row is the flight's end.
#[test]
fn an_observer_sees_the_flight() {
    use hpr_sim::{Channel, Recorder};
    let rocket = built();
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::Mass], None).unwrap();
    let flight = Flight::builder(&rocket, &environment(), 1.8)
        .fly_with(&mut recorder)
        .unwrap();
    let last = recorder.rows().last().unwrap();
    let end = flight.result().final_sample;
    assert_eq!((last[0], last[1]), (end.time_s, end.mass_kg));
    assert_eq!(
        flight,
        Flight::builder(&rocket, &environment(), 1.8).fly().unwrap()
    );
    // A flight's record reads back as the same flight.
    let text = serde_json::to_string(&flight).unwrap();
    assert_eq!(serde_json::from_str::<Flight>(&text).unwrap(), flight);
}

/// hpr's own drag buildup, handed back through the drag-model trait unchanged.
#[derive(Debug)]
struct HprsOwn;

impl hpr_aero::DragModel for HprsOwn {
    fn zero_lift_drag(&self, query: &hpr_aero::DragQuery<'_>) -> Result<f64, hpr_aero::AeroError> {
        Ok(query.buildup()?.zero_lift_coefficient)
    }
}

/// The same drag coefficient at every flow.
#[derive(Debug)]
struct ConstantDrag(f64);

impl hpr_aero::DragModel for ConstantDrag {
    fn zero_lift_drag(&self, _query: &hpr_aero::DragQuery<'_>) -> Result<f64, hpr_aero::AeroError> {
        Ok(self.0)
    }
}

/// A drag model is flown in hpr's place: one handing back hpr's own drag flies the same flight,
/// bit for bit, and a constant one flies as the same constant table.
#[test]
fn a_drag_model_is_flown_in_place_of_hprs_drag() {
    let rocket = built();
    let environment = environment();
    let launch = Flight::builder(&rocket, &environment, 1.8);
    let own = launch.fly().unwrap();
    assert_eq!(launch.clone().drag_model(HprsOwn).fly().unwrap(), own);

    let table = hpr_aero::DragTable::from_csv("0,0.5\n1,0.5\n", None).unwrap();
    let by_table = launch
        .simulation()
        .unwrap()
        .with_drag_table(table)
        .run(&mut ())
        .unwrap();
    let by_model = launch.clone().drag_model(ConstantDrag(0.5)).fly().unwrap();
    assert_eq!(by_model.result(), &by_table);
    assert_ne!(by_model.apogee_m(), own.apogee_m());

    // More drag, a lower apogee; the last model set is the one flown.
    let draggier = launch.clone().drag_model(ConstantDrag(0.9)).fly().unwrap();
    assert!(draggier.apogee_m().unwrap() < by_model.apogee_m().unwrap());
    let last = launch
        .clone()
        .drag_model(ConstantDrag(0.9))
        .drag_model(HprsOwn)
        .fly()
        .unwrap();
    assert_eq!(last, own);

    // One shared model flies two builders' flights the same.
    let shared: std::sync::Arc<dyn hpr_aero::DragModel> = std::sync::Arc::new(ConstantDrag(0.5));
    let other = Flight::builder(&rocket, &environment, 1.8);
    assert_eq!(
        launch
            .clone()
            .shared_drag_model(shared.clone())
            .fly()
            .unwrap(),
        other.shared_drag_model(shared).fly().unwrap()
    );
    assert_eq!(
        launch
            .clone()
            .shared_drag_model(std::sync::Arc::new(ConstantDrag(0.5)))
            .fly()
            .unwrap(),
        by_model
    );

    // A model that answers nonsense stops the flight, named.
    let error = launch.drag_model(ConstantDrag(-1.0)).fly().unwrap_err();
    assert!(
        matches!(
            &error,
            Error::Sim(hpr_sim::SimError::Aero(hpr_aero::AeroError::Domain { what, value }))
                if *what == "zero-lift drag coefficient from a drag model" && *value == -1.0
        ),
        "{error:?}"
    );
}

/// Loft lesson L95: a degenerate design must be refused or fly to finite numbers, never to a NaN
/// or a hang. The builder refuses each one as it is given, naming it. Put straight into a design
/// the builder can't check, each is refused before the flight or flies finite.
#[test]
fn degenerate_designs_error_or_stay_finite() {
    let abs = || material("abs").unwrap();
    let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
    let domain = |result: Result<&mut Rocket, Error>| match result {
        Err(Error::Domain { what, value }) => (what, value),
        other => panic!("expected a domain error, got {other:?}"),
    };

    // Zero radius.
    assert!(matches!(
        Rocket::new("Zero", 0.0),
        Err(Error::Domain { what: "rocket diameter, m", value }) if value == 0.0
    ));
    let mut rocket = Rocket::new("Degenerate", 0.05).unwrap();
    let zero_tube = Tube::new(0.5, 0.001, abs()).with_diameter_m(0.0);
    assert_eq!(domain(rocket.add_tube(zero_tube)).0, "tube diameter, m");

    // NaN tokens, wherever a number goes.
    let (what, value) = domain(rocket.add_nose(Nose::solid(ogive, f64::NAN, abs())));
    assert!(what == "nose length, m" && value.is_nan());
    // A shape parameter out of its range: an ogive's radius ratio, its arc's radius over a tangent
    // ogive's, is at least the nose's radius over its length, 0.025 / 0.2 here.
    let short_arc = NoseShape::Ogive { radius_ratio: 0.1 };
    let design_domain = |result: Result<&mut Rocket, Error>| match result {
        Err(Error::Design(DesignError::Domain { what, value })) => (what, value),
        other => panic!("expected the design's domain error, got {other:?}"),
    };
    let (what, value) = design_domain(rocket.add_nose(Nose::solid(short_arc, 0.2, abs())));
    assert!(what.contains("ogive") && value == 0.1, "{what} {value}");
    let nan_wall = Nose::hollow(ogive, 0.2, f64::NAN, abs());
    assert_eq!(domain(rocket.add_nose(nan_wall)).0, "nose wall, m");
    let nan_tube = Tube::new(f64::INFINITY, 0.001, abs());
    assert_eq!(domain(rocket.add_tube(nan_tube)).0, "tube length, m");
    rocket.add_tube(Tube::new(0.5, 0.001, abs())).unwrap();
    let nan_mass = Mass::new(f64::NAN, Position::Top { aft_offset_m: 0.0 });
    assert_eq!(domain(rocket.add_mass(nan_mass)).0, "mass, kg");
    let nan_place = Mass::new(
        0.1,
        Position::Top {
            aft_offset_m: f64::NAN,
        },
    );
    assert_eq!(domain(rocket.add_mass(nan_place)).0, "position, m");
    assert!(matches!(
        rocket.add_fins(Fins::new(
            3,
            trapezoid([0.1, f64::NAN, 0.05, 0.0]),
            0.003,
            abs()
        )),
        Err(Error::Design(_))
    ));
    assert!(matches!(
        Rocket::new("NaN", f64::NAN),
        Err(Error::Domain { .. })
    ));
    assert!(matches!(
        Environment::new(f64::NAN, 0.0, 0.0),
        Err(Error::Core(_))
    ));

    // Zero fins.
    assert!(matches!(
        rocket.add_fins(Fins::new(
            0,
            trapezoid([0.1, 0.05, 0.05, 0.0]),
            0.003,
            abs()
        )),
        Err(Error::Design(_))
    ));

    // Negative mass.
    let negative = Mass::new(-0.1, Position::Top { aft_offset_m: 0.0 });
    let (what, value) = domain(rocket.add_mass(negative));
    assert!(what == "mass, kg" && value == -0.1);

    // The same, put straight into the tree of a rocket that flies, where only the design's own
    // checks see them: each is refused before the flight, by the part it is in, or flies to
    // finite numbers. Without any fin set the rocket is unstable, and tumbles.
    let refused = |id: &'static str, what: &'static str| -> Refusal { Some((id, what)) };
    let cases: [(&str, Spoil, Refusal); 8] = [
        (
            "zero tube radius",
            |design| {
                if let Part::BodyTube(tube) = &mut design.stages[0].components[1].part {
                    tube.outer_radius_m = 0.0;
                }
            },
            // The nose's shoulder takes the tube's inner radius, and finds it negative first.
            refused("nose", "outer radius"),
        ),
        (
            "zero tube radius, no shoulder before it",
            |design| {
                if let Part::NoseCone(nose) = &mut design.stages[0].components[0].part {
                    nose.shoulder = None;
                }
                design.stages[0].components[0].auto.clear();
                if let Part::BodyTube(tube) = &mut design.stages[0].components[1].part {
                    tube.outer_radius_m = 0.0;
                }
            },
            refused("tube", "outer radius"),
        ),
        (
            "NaN nose length",
            |design| {
                if let Part::NoseCone(nose) = &mut design.stages[0].components[0].part {
                    nose.length_m = f64::NAN;
                }
            },
            refused("nose", "body component length"),
        ),
        (
            "NaN fin span",
            |design| {
                if let Part::FinSet(FinSet {
                    planform: FinPlanform::Trapezoidal { span_m, .. },
                    ..
                }) = &mut design.stages[0].components[1].children[1].part
                {
                    *span_m = f64::NAN;
                }
            },
            refused("fins", "fin span"),
        ),
        (
            "zero fins",
            |design| {
                if let Part::FinSet(fins) = &mut design.stages[0].components[1].children[1].part {
                    fins.count = 0;
                }
            },
            refused("fins", "fin count"),
        ),
        (
            "no fin set",
            |design| {
                design.stages[0].components[1].children.remove(1);
            },
            None,
        ),
        (
            "negative mass",
            |design| {
                if let Part::MassComponent(mass) =
                    &mut design.stages[0].components[1].children[2].part
                {
                    mass.mass_kg = -0.1;
                }
            },
            refused("mass", "mass"),
        ),
        (
            "negative mass override",
            |design| {
                design.stages[0].overrides.mass_kg = Some(-1.0);
            },
            refused("sustainer", "mass override (kg)"),
        ),
    ];
    for (name, spoil, expected) in cases {
        let mut design = built().design().clone();
        let before = design.clone();
        spoil(&mut design);
        assert_ne!(design, before, "{name}: the case changed nothing");
        let outcome = Rocket::from_design(design, "168H54-10A").and_then(|mut rocket| {
            rocket.add_parachute(parachute());
            Flight::builder(&rocket, &environment(), 1.8).fly()
        });
        match (outcome, expected) {
            (Ok(flight), None) => assert_flies_finite(name, &flight),
            (
                Err(Error::Sim(hpr_sim::SimError::Design(DesignError::InComponent { id, source }))),
                Some((part, quantity)),
            ) if id == part
                && matches!(*source, DesignError::Domain { what, .. } if what == quantity) => {}
            (other, expected) => panic!("{name}: expected {expected:?}, got {other:?}"),
        }
    }
}

/// A change that spoils a design.
type Spoil = fn(&mut hpr_design::Rocket);

/// The part and the quantity that refuse a spoiled design, or `None` if it flies.
type Refusal = Option<(&'static str, &'static str)>;

/// Every number a flight reports is finite.
fn assert_flies_finite(name: &str, flight: &Flight) {
    let summary = flight.summary();
    let peaks = [
        summary.rail_exit_speed_m_s,
        summary.max_speed_m_s,
        summary.max_mach,
        summary.max_dynamic_pressure_pa,
        summary.max_acceleration_m_s2,
    ];
    for peak in peaks.into_iter().flatten() {
        assert!(
            peak.value.is_finite() && peak.time_s.is_finite(),
            "{name}: {peak:?}"
        );
    }
    for event in &flight.result().events {
        let sample = &event.sample;
        let numbers = [
            sample.time_s,
            sample.height_above_ground_m,
            sample.cg_velocity_enu_m_s.length(),
            sample.mass_kg,
            sample.mach,
        ];
        assert!(
            numbers.iter().all(|x| x.is_finite()),
            "{name}: {:?} at {:?}",
            numbers,
            event.kind
        );
    }
}

/// A wind of your own that isn't a finite velocity stops the flight: the air's speed then isn't
/// finite, and the drag's Reynolds number, checked at every step, refuses it.
#[test]
fn a_wind_that_is_not_finite_stops_the_flight() {
    /// A west wind of `speed_m_s` above 300 m over the site, and calm below.
    #[derive(Debug)]
    struct Aloft(f64);
    impl hpr_atmos::Wind for Aloft {
        fn wind(&self, height_msl_m: f64) -> Result<hpr_atmos::WindSample, hpr_atmos::AtmosError> {
            let east_m_s = if height_msl_m > 1700.0 { self.0 } else { 0.0 };
            Ok(hpr_atmos::WindSample {
                velocity_enu_m_s: hpr_core::DVec3::new(east_m_s, 0.0, 0.0),
                extrapolated: None,
            })
        }
    }
    let rocket = built();
    for bad in [f64::NAN, f64::INFINITY] {
        let environment = environment().with_wind(Aloft(bad));
        let error = Flight::builder(&rocket, &environment, 1.8)
            .fly()
            .unwrap_err();
        assert!(
            matches!(
                &error,
                Error::Sim(hpr_sim::SimError::Aero(hpr_aero::AeroError::Domain { what, .. }))
                    if *what == "Reynolds number per metre"
            ),
            "{error:?}"
        );
    }
    // The same wind, finite, flies.
    let environment = environment().with_wind(Aloft(5.0));
    assert!(Flight::builder(&rocket, &environment, 1.8).fly().is_ok());
}
