//! A rocket built part by part, from the nose back.
//!
//! [`Rocket::new`] starts an empty rocket of one body diameter. Body parts ([`Nose`], [`Tube`],
//! [`Transition`]) stack from the nose tip in the order they are added. Attached parts ([`Fins`],
//! [`MotorTube`], [`Mass`]) go on the last body tube added, where their [`Position`] puts them.
//! [`Rocket::set_motor`] puts a [`Motor`] in the motor tube, and [`Rocket::add_parachute`] adds a
//! recovery device. What the builder makes is an ordinary [`hpr_design::Rocket`], the same tree a
//! design file holds ([`Rocket::design`]); a design you already have flies through
//! [`Rocket::from_design`].
//!
//! Every part names its material. [`material`] finds a built-in one by id; the list, with each
//! density's source, is [`hpr_design::materials`]. The builder has no default materials or wall
//! thicknesses, because each one would be a guess at your rocket's mass.

use hpr_aero::{AeroModel, Flow};
use hpr_design::{
    Assembly, AutoDimension, BodyTube, Component, Configuration, FinCrossSection, FinPlanform,
    FinSet, Ignition, InnerTube, MassComponent, MassProperties, Material, MotorMount, MountedMotor,
    NoseCone, NoseShape, Overrides, Packing, Part, Position, ReferenceDiameter, Shoulder, Stage,
    Wall, materials,
};
use hpr_sim::Device;
use hpr_sim::metrics::{self, Margin};

use crate::error::{Error, finite, non_negative, positive};
use crate::motor::Motor;

/// The built-in material with id `id`, such as `"abs"`, `"kraft_phenolic"` or
/// `"birch_plywood"`. [`hpr_design::materials`] lists them all, each with its density's source.
///
/// # Errors
///
/// [`Error::UnknownMaterial`] if no built-in material has that id.
pub fn material(id: &str) -> Result<Material, Error> {
    materials::find(id)
        .map(|builtin| builtin.material())
        .ok_or_else(|| Error::UnknownMaterial(id.to_owned()))
}

/// A nose cone. Its base takes the rocket's diameter.
#[derive(Debug, Clone, PartialEq)]
pub struct Nose {
    shape: NoseShape,
    length_m: f64,
    wall: Wall,
    shoulder: Option<Shoulder>,
    material: Material,
    name: String,
}

impl Nose {
    /// A hollow nose cone of `shape` and `length_m`, with a wall `wall_m` thick.
    #[must_use]
    pub fn hollow(shape: NoseShape, length_m: f64, wall_m: f64, material: Material) -> Self {
        Self {
            shape,
            length_m,
            wall: Wall::Shell {
                thickness_m: wall_m,
            },
            shoulder: None,
            material,
            name: String::new(),
        }
    }

    /// A solid nose cone of `shape` and `length_m`.
    #[must_use]
    pub fn solid(shape: NoseShape, length_m: f64, material: Material) -> Self {
        Self {
            wall: Wall::Filled {},
            ..Self::hollow(shape, length_m, 0.0, material)
        }
    }

    /// The same nose with a shoulder `length_m` long and `wall_m` thick behind its base, to fit
    /// inside the tube behind it: its outer radius is that tube's inner radius. A `capped`
    /// shoulder has a disc of the same thickness closing its aft end.
    #[must_use]
    pub fn with_shoulder(mut self, length_m: f64, wall_m: f64, capped: bool) -> Self {
        self.shoulder = Some(Shoulder {
            length_m,
            outer_radius_m: 0.0,
            thickness_m: wall_m,
            capped,
        });
        self
    }

    /// The same nose with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A body tube: a length of the airframe.
#[derive(Debug, Clone, PartialEq)]
pub struct Tube {
    length_m: f64,
    wall_m: f64,
    diameter_m: Option<f64>,
    material: Material,
    name: String,
}

impl Tube {
    /// A tube `length_m` long with a wall `wall_m` thick, of the rocket's diameter.
    #[must_use]
    pub fn new(length_m: f64, wall_m: f64, material: Material) -> Self {
        Self {
            length_m,
            wall_m,
            diameter_m: None,
            material,
            name: String::new(),
        }
    }

    /// The same tube with outer diameter `diameter_m` instead of the rocket's, for a rocket whose
    /// airframe steps up or down at a [`Transition`].
    #[must_use]
    pub fn with_diameter_m(mut self, diameter_m: f64) -> Self {
        self.diameter_m = Some(diameter_m);
        self
    }

    /// The same tube with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A transition: a shoulder or boattail between two diameters. Its fore end takes the diameter
/// of the body part before it.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    shape: NoseShape,
    length_m: f64,
    aft_diameter_m: f64,
    wall: Wall,
    material: Material,
    name: String,
}

impl Transition {
    /// A hollow conical transition `length_m` long, to `aft_diameter_m` at its aft end, with a
    /// wall `wall_m` thick.
    #[must_use]
    pub fn conical(length_m: f64, aft_diameter_m: f64, wall_m: f64, material: Material) -> Self {
        Self {
            shape: NoseShape::Conical {},
            length_m,
            aft_diameter_m,
            wall: Wall::Shell {
                thickness_m: wall_m,
            },
            material,
            name: String::new(),
        }
    }

    /// The same transition with the profile `shape` instead of a cone.
    #[must_use]
    pub fn with_shape(mut self, shape: NoseShape) -> Self {
        self.shape = shape;
        self
    }

    /// The same transition, solid.
    #[must_use]
    pub fn solid(mut self) -> Self {
        self.wall = Wall::Filled {};
        self
    }

    /// The same transition with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A set of identical fins spaced evenly around the last body tube, flush with its aft end
/// unless placed elsewhere with [`Fins::at`]. Their edges are square unless
/// [`Fins::with_cross_section`] says otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct Fins {
    set: FinSet,
    position: Position,
    name: String,
}

impl Fins {
    /// `count` trapezoidal fins of root chord `root_chord_m`, tip chord `tip_chord_m`, span
    /// `span_m` and thickness `thickness_m`, the tip's leading edge `sweep_m` aft of the root's.
    #[must_use]
    pub fn trapezoidal(
        count: u32,
        [root_chord_m, tip_chord_m, span_m, sweep_m]: [f64; 4],
        thickness_m: f64,
        material: Material,
    ) -> Self {
        Self::of(
            count,
            FinPlanform::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                sweep_m,
            },
            thickness_m,
            material,
        )
    }

    /// `count` elliptical fins of root chord `root_chord_m`, span `span_m` and thickness
    /// `thickness_m`.
    #[must_use]
    pub fn elliptical(
        count: u32,
        root_chord_m: f64,
        span_m: f64,
        thickness_m: f64,
        material: Material,
    ) -> Self {
        Self::of(
            count,
            FinPlanform::Elliptical {
                root_chord_m,
                span_m,
            },
            thickness_m,
            material,
        )
    }

    fn of(count: u32, planform: FinPlanform, thickness_m: f64, material: Material) -> Self {
        Self {
            set: FinSet {
                count,
                planform,
                thickness_m,
                cross_section: FinCrossSection::Square,
                tab: None,
                fillet: None,
                cant_rad: 0.0,
                base_angle_rad: 0.0,
                material,
            },
            position: Position::Bottom { aft_offset_m: 0.0 },
            name: String::new(),
        }
    }

    /// The same fins with edges shaped `cross_section`: square, rounded or an airfoil. It changes
    /// their drag.
    #[must_use]
    pub fn with_cross_section(mut self, cross_section: FinCrossSection) -> Self {
        self.set.cross_section = cross_section;
        self
    }

    /// The same fins canted by `cant_deg`, which spins the rocket (the sign convention is
    /// [`FinSet::cant_rad`]'s).
    #[must_use]
    pub fn with_cant_deg(mut self, cant_deg: f64) -> Self {
        self.set.cant_rad = cant_deg.to_radians();
        self
    }

    /// The same fins at `position` along their tube.
    #[must_use]
    pub fn at(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// The same fins with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// The tube inside the airframe that holds the motor, flush with the last body tube's aft end
/// unless placed elsewhere with [`MotorTube::at`].
#[derive(Debug, Clone, PartialEq)]
pub struct MotorTube {
    length_m: f64,
    inner_diameter_m: f64,
    wall_m: f64,
    overhang_m: f64,
    position: Position,
    material: Material,
    name: String,
}

impl MotorTube {
    /// A motor tube `length_m` long, with a bore of `inner_diameter_m` and a wall `wall_m` thick.
    #[must_use]
    pub fn new(length_m: f64, inner_diameter_m: f64, wall_m: f64, material: Material) -> Self {
        Self {
            length_m,
            inner_diameter_m,
            wall_m,
            overhang_m: 0.0,
            position: Position::Bottom { aft_offset_m: 0.0 },
            material,
            name: String::new(),
        }
    }

    /// The same tube with the motor's aft end `overhang_m` past the tube's.
    #[must_use]
    pub fn with_overhang_m(mut self, overhang_m: f64) -> Self {
        self.overhang_m = overhang_m;
        self
    }

    /// The same tube at `position` along the body tube it is in.
    #[must_use]
    pub fn at(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// The same tube with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A mass inside the last body tube: a parachute and its cord, an altimeter bay, ballast.
///
/// It is a solid cylinder along the axis, as long and as wide as its packing, which is a point
/// unless given with [`Mass::packed`]. The packing changes only its moments of inertia.
#[derive(Debug, Clone, PartialEq)]
pub struct Mass {
    mass_kg: f64,
    position: Position,
    packing: Packing,
    name: String,
}

impl Mass {
    /// `mass_kg` at `position` along the body tube it is in.
    #[must_use]
    pub fn new(mass_kg: f64, position: Position) -> Self {
        Self {
            mass_kg,
            position,
            packing: Packing {
                length_m: 0.0,
                radius_m: 0.0,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
            name: String::new(),
        }
    }

    /// The same mass packed in a cylinder `length_m` long and `diameter_m` across.
    #[must_use]
    pub fn packed(mut self, length_m: f64, diameter_m: f64) -> Self {
        self.packing.length_m = length_m;
        self.packing.radius_m = 0.5 * diameter_m;
        self
    }

    /// The same mass with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A rocket: its design, its motor and its recovery devices.
///
/// ```
/// use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
/// use hpr::{Motor, NoseShape, Position, Rocket};
///
/// let mut rocket = Rocket::new("Small", 0.0563)?;
/// let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
/// rocket
///     .add_nose(Nose::hollow(ogive, 0.22, 0.0015, material("abs")?))?
///     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
///     .add_fins(Fins::trapezoidal(
///         3,
///         [0.1, 0.04, 0.045, 0.05],
///         0.003175,
///         material("birch_plywood")?,
///     ))?
///     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
///     .set_motor(Motor::from_catalog("H54")?)?;
///
/// // Unstable as it stands: the motor's weight puts the centre of gravity behind the centre of
/// // pressure, and the margin, in calibres, is negative.
/// assert!(rocket.static_margin_cal(0.0, 0.3)? < 0.0);
/// // 200 g near the top of the tube, a recovery bay, brings it forward of it.
/// rocket.add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?;
/// assert!(rocket.static_margin_cal(0.0, 0.3)? > 1.0);
/// # Ok::<(), hpr::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Rocket {
    design: hpr_design::Rocket,
    configuration_id: Option<String>,
    recovery: Vec<Device>,
    /// Where the builder has got to; `None` for a rocket read from a design.
    build: Option<Build>,
}

/// The builder's place in the tree.
#[derive(Debug, Clone, PartialEq)]
struct Build {
    /// The rocket's body diameter, m.
    diameter_m: f64,
    /// The last body part's aft radius, m; `None` before the first.
    aft_radius_m: Option<f64>,
    /// The index of the last body tube among the stage's components.
    tube: Option<usize>,
    /// The motor tube's component id, once there is one.
    motor_tube: Option<String>,
}

/// The id of the stage the builder makes.
const STAGE_ID: &str = "sustainer";

impl Rocket {
    /// An empty rocket called `name`, of body diameter `diameter_m`. Its reference diameter, the
    /// one the stability margin is counted in, is its largest body diameter.
    ///
    /// # Errors
    ///
    /// [`Error::Domain`] for a diameter that isn't finite and positive.
    pub fn new(name: &str, diameter_m: f64) -> Result<Self, Error> {
        Ok(Self {
            design: hpr_design::Rocket {
                name: name.to_owned(),
                stages: vec![Stage {
                    id: STAGE_ID.to_owned(),
                    name: String::new(),
                    components: Vec::new(),
                    overrides: Overrides::default(),
                }],
                reference_diameter: ReferenceDiameter::Maximum {},
                configurations: Vec::new(),
            },
            configuration_id: None,
            recovery: Vec::new(),
            build: Some(Build {
                diameter_m: positive("rocket diameter, m", diameter_m)?,
                aft_radius_m: None,
                tube: None,
                motor_tube: None,
            }),
        })
    }

    /// A rocket from a design you already have, such as a design file read with `serde_json` or
    /// an OpenRocket file read with [`hpr_io::ork`], flown in its configuration
    /// `configuration_id`. Parts can't be added to it; recovery devices can.
    ///
    /// # Errors
    ///
    /// [`Error::NoSuchConfiguration`] if the design has no configuration of that id.
    pub fn from_design(design: hpr_design::Rocket, configuration_id: &str) -> Result<Self, Error> {
        if design.configuration(configuration_id).is_none() {
            return Err(Error::NoSuchConfiguration(configuration_id.to_owned()));
        }
        Ok(Self {
            design,
            configuration_id: Some(configuration_id.to_owned()),
            recovery: Vec::new(),
            build: None,
        })
    }

    /// Adds the nose cone, which goes first.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if a body part is already in place, or the rocket was read from a
    ///   design.
    /// - [`Error::Domain`] for a length, wall or shoulder dimension that isn't finite and
    ///   positive.
    pub fn add_nose(&mut self, nose: Nose) -> Result<&mut Self, Error> {
        let build = self.build()?;
        if build.aft_radius_m.is_some() {
            return Err(Error::Order(
                "the nose goes first, before any other body part",
            ));
        }
        let radius_m = 0.5 * build.diameter_m;
        positive("nose length, m", nose.length_m)?;
        check_wall("nose wall, m", nose.wall)?;
        let mut auto = Vec::new();
        if let Some(shoulder) = &nose.shoulder {
            positive("nose shoulder length, m", shoulder.length_m)?;
            positive("nose shoulder wall, m", shoulder.thickness_m)?;
            auto.push(AutoDimension::ShoulderRadius);
        }
        let part = Part::NoseCone(NoseCone {
            shape: nose.shape,
            length_m: nose.length_m,
            base_radius_m: radius_m,
            wall: nose.wall,
            shoulder: nose.shoulder,
            material: nose.material,
        });
        let mut component = self.component("nose", &nose.name, part, None);
        component.auto = auto;
        self.push_body(component, radius_m, false);
        Ok(self)
    }

    /// Adds a body tube behind the last body part.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if the rocket was read from a design.
    /// - [`Error::Domain`] for a length, wall or diameter that isn't finite and positive.
    pub fn add_tube(&mut self, tube: Tube) -> Result<&mut Self, Error> {
        let build = self.build()?;
        let diameter_m = positive(
            "tube diameter, m",
            tube.diameter_m.unwrap_or(build.diameter_m),
        )?;
        let part = Part::BodyTube(BodyTube {
            length_m: positive("tube length, m", tube.length_m)?,
            outer_radius_m: 0.5 * diameter_m,
            thickness_m: positive("tube wall, m", tube.wall_m)?,
            material: tube.material,
        });
        let component = self.component("tube", &tube.name, part, None);
        self.push_body(component, 0.5 * diameter_m, true);
        Ok(self)
    }

    /// Adds a transition behind the last body part, starting at its diameter.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body part before it, or the rocket was read from a
    ///   design.
    /// - [`Error::Domain`] for a length or wall that isn't finite and positive, or an aft
    ///   diameter that is negative or not finite.
    pub fn add_transition(&mut self, transition: Transition) -> Result<&mut Self, Error> {
        let fore_radius_m = self
            .build()?
            .aft_radius_m
            .ok_or(Error::Order("a transition needs a body part before it"))?;
        positive("transition length, m", transition.length_m)?;
        check_wall("transition wall, m", transition.wall)?;
        let aft_radius_m =
            0.5 * non_negative("transition aft diameter, m", transition.aft_diameter_m)?;
        let part = Part::Transition(hpr_design::Transition {
            shape: transition.shape,
            clipped: false,
            length_m: transition.length_m,
            fore_radius_m,
            aft_radius_m,
            wall: transition.wall,
            fore_shoulder: None,
            aft_shoulder: None,
            material: transition.material,
        });
        let component = self.component("transition", &transition.name, part, None);
        self.push_body(component, aft_radius_m, false);
        Ok(self)
    }

    /// Adds a fin set to the last body tube.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body tube yet, or the rocket was read from a design.
    /// - [`Error::Design`] for fins the design refuses ([`FinSet::validate`]): no fins, a chord,
    ///   span or thickness that isn't finite and positive, and the like.
    /// - [`Error::Domain`] for a position that isn't finite.
    pub fn add_fins(&mut self, fins: Fins) -> Result<&mut Self, Error> {
        fins.set.validate()?;
        check_position(fins.position)?;
        let component = self.component(
            "fins",
            &fins.name,
            Part::FinSet(fins.set),
            Some(fins.position),
        );
        self.attach(component)?;
        Ok(self)
    }

    /// Adds the motor tube to the last body tube. A rocket has one.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body tube yet, the rocket has a motor tube already, or
    ///   it was read from a design.
    /// - [`Error::Domain`] for a length, bore or wall that isn't finite and positive, an overhang
    ///   that is negative or not finite, or a position that isn't finite.
    pub fn add_motor_tube(&mut self, tube: MotorTube) -> Result<&mut Self, Error> {
        if self.build()?.motor_tube.is_some() {
            return Err(Error::Order("the rocket has a motor tube already"));
        }
        let wall_m = positive("motor tube wall, m", tube.wall_m)?;
        let part = Part::InnerTube(InnerTube {
            length_m: positive("motor tube length, m", tube.length_m)?,
            outer_radius_m: 0.5 * positive("motor tube bore, m", tube.inner_diameter_m)? + wall_m,
            thickness_m: wall_m,
            radial_offset_m: 0.0,
            angle_rad: 0.0,
            material: tube.material,
            cluster_m: Vec::new(),
        });
        check_position(tube.position)?;
        let mut component = self.component("motor-tube", &tube.name, part, Some(tube.position));
        component.motor_mount = Some(MotorMount {
            overhang_m: non_negative("motor overhang, m", tube.overhang_m)?,
        });
        let id = component.id.clone();
        self.attach(component)?;
        if let Some(build) = &mut self.build {
            build.motor_tube = Some(id);
        }
        Ok(self)
    }

    /// Adds a mass inside the last body tube.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body tube yet, or the rocket was read from a design.
    /// - [`Error::Domain`] for a mass or packing that is negative or not finite, or a position
    ///   that isn't finite.
    pub fn add_mass(&mut self, mass: Mass) -> Result<&mut Self, Error> {
        non_negative("mass, kg", mass.mass_kg)?;
        non_negative("packed length, m", mass.packing.length_m)?;
        non_negative("packed diameter, m", mass.packing.radius_m)?;
        check_position(mass.position)?;
        let part = Part::MassComponent(MassComponent {
            mass_kg: mass.mass_kg,
            packing: mass.packing,
        });
        let component = self.component("mass", &mass.name, part, Some(mass.position));
        self.attach(component)?;
        Ok(self)
    }

    /// Puts `motor` in the motor tube, lit at launch, in place of any motor there before. The
    /// design's one configuration is named after its designation.
    ///
    /// # Errors
    ///
    /// [`Error::Order`] if there is no motor tube yet, or the rocket was read from a design.
    pub fn set_motor(&mut self, motor: Motor) -> Result<&mut Self, Error> {
        let mount = self
            .build()?
            .motor_tube
            .clone()
            .ok_or(Error::Order("a motor needs a motor tube to go in"))?;
        let id = motor.designation().to_owned();
        self.design.configurations = vec![Configuration {
            id: id.clone(),
            name: String::new(),
            motors: vec![MountedMotor {
                mount,
                designation: id.clone(),
                diameter_m: motor.diameter_m(),
                length_m: motor.length_m(),
                motor: motor.solid_motor().clone(),
                delay: motor.delay(),
                ignition: Ignition::Launch,
                failed_tubes: Vec::new(),
            }],
        }];
        self.configuration_id = Some(id);
        Ok(self)
    }

    /// Adds a recovery device: a parachute, a streamer or a tumble, with what opens it.
    /// [`Trigger::MotorDelay`](hpr_sim::Trigger::MotorDelay) with motor 0 opens it at the
    /// motor's ejection charge. The device adds drag, not mass: add its mass with
    /// [`Rocket::add_mass`].
    pub fn add_parachute(&mut self, device: Device) -> &mut Self {
        self.recovery.push(device);
        self
    }

    /// The design: the same tree a design file holds.
    #[must_use]
    pub fn design(&self) -> &hpr_design::Rocket {
        &self.design
    }

    /// The configuration the rocket flies in: the motor's designation for a built rocket, `None`
    /// before it has a motor.
    #[must_use]
    pub fn configuration_id(&self) -> Option<&str> {
        self.configuration_id.as_deref()
    }

    /// The recovery devices, in the order they were added.
    #[must_use]
    pub fn recovery(&self) -> &[Device] {
        &self.recovery
    }

    /// The parts placed and the motor in its tube ([`hpr_design::Rocket::assemble`]).
    ///
    /// # Errors
    ///
    /// [`Error::NoMotor`] before the rocket has a motor, and [`Error::Design`] for a tree that
    /// doesn't hold together, such as a motor wider than its tube's bore.
    pub fn assemble(&self) -> Result<Assembly, Error> {
        let id = self.configuration_id.as_deref().ok_or(Error::NoMotor)?;
        Ok(self.design.assemble(id)?)
    }

    /// The mass, centre of gravity and inertia `time_s` seconds after the motor lights, the
    /// motor burning as its thrust curve says. The centre of gravity is in the body frame: `z`
    /// points to the nose and is zero at its tip, so a point `s` metres aft of the tip is at
    /// `z = −s` (`docs/physics/frames.md`).
    ///
    /// # Errors
    ///
    /// As [`Rocket::assemble`].
    pub fn mass_properties(&self, time_s: f64) -> Result<MassProperties, Error> {
        Ok(self.assemble()?.mass_properties(finite("time, s", time_s)?))
    }

    /// The static stability margin `time_s` seconds after the motor lights, at Mach `mach` with
    /// the air along the axis, calibres of the reference diameter: how far the centre of pressure
    /// is behind the centre of gravity. [`Rocket::margin`] gives the rest of it.
    ///
    /// # Errors
    ///
    /// As [`Rocket::margin`], and [`Error::Domain`] with a NaN value where the margin is
    /// undefined because the rocket has no normal force.
    pub fn static_margin_cal(&self, time_s: f64, mach: f64) -> Result<f64, Error> {
        self.margin(time_s, mach)?.margin_cal.ok_or(Error::Domain {
            what: "static margin with no normal force, calibres",
            value: f64::NAN,
        })
    }

    /// The centre of pressure and the static margin `time_s` seconds after the motor lights, at
    /// Mach `mach` with the air along the axis ([`hpr_sim::metrics::margin`]).
    ///
    /// # Errors
    ///
    /// As [`Rocket::assemble`], [`Error::Aero`] for a rocket the aerodynamic model can't take,
    /// and [`Error::Sim`] for a Mach number out of its range.
    pub fn margin(&self, time_s: f64, mach: f64) -> Result<Margin, Error> {
        let assembly = self.assemble()?;
        let aero = AeroModel::new(&assembly.layout)?;
        let cg_station_m = -assembly.mass_properties(finite("time, s", time_s)?).cg_m.z;
        Ok(metrics::margin(&aero, &Flow::axial(mach), cg_station_m)?)
    }

    /// The builder's place, or [`Error::Order`] for a rocket read from a design.
    fn build(&self) -> Result<&Build, Error> {
        self.build.as_ref().ok_or(Error::Order(
            "parts can't be added to a rocket read from a design",
        ))
    }

    /// A component of the stage, its id `kind` or, if that is taken, `kind-2`, `kind-3` and on.
    fn component(
        &self,
        kind: &str,
        name: &str,
        part: Part,
        position: Option<Position>,
    ) -> Component {
        let mut ids = Vec::new();
        for stage in &self.design.stages {
            for component in &stage.components {
                ids.push(component.id.as_str());
                ids.extend(component.children.iter().map(|child| child.id.as_str()));
            }
        }
        let mut id = kind.to_owned();
        let mut n = 1;
        while ids.contains(&id.as_str()) {
            n += 1;
            id = format!("{kind}-{n}");
        }
        Component {
            id,
            name: name.to_owned(),
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

    /// Adds a body component with aft radius `aft_radius_m`; a `tube` takes the attached parts
    /// that follow.
    fn push_body(&mut self, component: Component, aft_radius_m: f64, tube: bool) {
        let components = &mut self.design.stages[0].components;
        components.push(component);
        let index = components.len() - 1;
        if let Some(build) = &mut self.build {
            build.aft_radius_m = Some(aft_radius_m);
            if tube {
                build.tube = Some(index);
            }
        }
    }

    /// Attaches `component` to the last body tube.
    fn attach(&mut self, component: Component) -> Result<(), Error> {
        let index = self.build()?.tube.ok_or(Error::Order(
            "fins, a motor tube and masses go on a body tube: add one first",
        ))?;
        self.design.stages[0].components[index]
            .children
            .push(component);
        Ok(())
    }
}

/// A wall's thickness, if it has one, checked finite and positive.
fn check_wall(what: &'static str, wall: Wall) -> Result<(), Error> {
    if let Wall::Shell { thickness_m } = wall {
        positive(what, thickness_m)?;
    }
    Ok(())
}

/// A position's offset or station, checked finite.
fn check_position(position: Position) -> Result<(), Error> {
    let (Position::Top {
        aft_offset_m: value,
    }
    | Position::Middle {
        aft_offset_m: value,
    }
    | Position::Bottom {
        aft_offset_m: value,
    }
    | Position::After {
        aft_offset_m: value,
    }
    | Position::Absolute { station_m: value }) = position;
    finite("position, m", value)?;
    Ok(())
}
