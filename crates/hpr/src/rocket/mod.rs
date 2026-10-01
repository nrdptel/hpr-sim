//! A rocket built part by part, from the nose back.
//!
//! [`Rocket::new`] starts an empty rocket of one body diameter. Body parts ([`Nose`], [`Tube`],
//! [`Transition`]) stack from the nose tip in the order they are added. Attached parts ([`Fins`],
//! [`MotorTube`], [`Mass`]) go on the last body tube added, where their [`Position`] puts them.
//! [`Fitting`]s (couplers, centering rings, bulkheads, launch lugs, packed parachutes and
//! streamers) go there too. [`Rocket::set_motor`] puts a [`Motor`] in the motor tube, and
//! [`Rocket::add_parachute`] adds a recovery device. What the builder makes is an ordinary [`hpr_design::Rocket`], the same tree a
//! design file holds ([`Rocket::design`]); a design you already have flies through
//! [`Rocket::from_design`].
//!
//! Every part names its material. [`material`] finds a built-in one by id; the list, with each
//! density's source, is [`hpr_design::materials`]. The builder has no default materials or wall
//! thicknesses, because each one would be a guess at your rocket's mass. Every outer surface has
//! the design's default finish, mass-production paint ([`hpr_design::Finish`]), which sets its
//! skin friction; the builder can't change it yet.
//!
//! Parts can also come from a parts catalogue, such as the one OpenRocket ships
//! ([`hpr_io::orc::bundled`]): [`Nose::from_catalog`], [`Tube::from_catalog`],
//! [`Transition::from_catalog`], [`MotorTube::from_catalog`] and [`Fitting::from_catalog`] make
//! each part as the catalogue gives it, its material and stated mass included. What a catalogue
//! leaves unsaid is chosen as OpenRocket chooses it, but for a hollow part's shoulder wall
//! ([`catalog`] lists each choice).

use hpr_aero::{AeroModel, Flow};
use hpr_design::checks::{check, has_errors};
use hpr_design::{
    Assembly, AutoDimension, BodyTube, Component, Configuration, FinCrossSection, FinPlanform,
    FinSet, Ignition, InnerTube, MassComponent, MassProperties, Material, MotorMount, MountedMotor,
    NoseCone, NoseShape, Overrides, Packing, Part, Position, Profile, ReferenceDiameter, Shoulder,
    Stage, Wall, materials,
};
use hpr_sim::Device;
use hpr_sim::metrics::{self, Margin};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Order, finite, non_negative, positive};
use crate::motor::Motor;

pub mod catalog;

pub use catalog::Fitting;

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

/// A nose cone. Its base takes the rocket's diameter, unless it states its own, as a catalogue
/// part does ([`Nose::from_catalog`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nose {
    shape: NoseShape,
    length_m: f64,
    wall: Wall,
    /// The shoulder; an outer radius of zero takes the inner radius of the tube behind it.
    shoulder: Option<Shoulder>,
    material: Material,
    name: String,
    /// The base diameter, m, where the nose states its own: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    diameter_m: Option<f64>,
    /// The mass the nose weighs, kg, where it states one: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mass_kg: Option<f64>,
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
            diameter_m: None,
            mass_kg: None,
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

    /// The same nose with a shoulder, the sleeve behind its base that fits inside the tube
    /// behind it: `length_m` long, `wall_m` thick, open at its aft end. Its outer radius is that
    /// tube's inner radius.
    #[must_use]
    pub fn with_shoulder(mut self, length_m: f64, wall_m: f64) -> Self {
        self.shoulder = Some(Shoulder {
            length_m,
            outer_radius_m: 0.0,
            thickness_m: wall_m,
            capped: false,
        });
        self
    }

    /// The same nose with a shoulder as [`Nose::with_shoulder`] makes it, closed at its aft end
    /// by a disc as thick as its wall.
    #[must_use]
    pub fn with_capped_shoulder(self, length_m: f64, wall_m: f64) -> Self {
        let mut nose = self.with_shoulder(length_m, wall_m);
        if let Some(shoulder) = &mut nose.shoulder {
            shoulder.capped = true;
        }
        nose
    }

    /// The same nose with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }
}

/// A body tube: a length of the airframe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tube {
    length_m: f64,
    wall_m: f64,
    diameter_m: Option<f64>,
    material: Material,
    name: String,
    /// The mass the tube weighs, kg, where it states one: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mass_kg: Option<f64>,
}

impl Tube {
    /// A tube `length_m` long with a wall `wall_m` thick. Its outer diameter is the aft diameter
    /// of the body part before it, or the rocket's if it is the first.
    #[must_use]
    pub fn new(length_m: f64, wall_m: f64, material: Material) -> Self {
        Self {
            length_m,
            wall_m,
            diameter_m: None,
            material,
            name: String::new(),
            mass_kg: None,
        }
    }

    /// The same tube with outer diameter `diameter_m`. A diameter other than the aft diameter of
    /// the part before it makes a step in the airframe, which the design's checks warn of; a
    /// [`Transition`] joins two diameters smoothly.
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

/// A transition between two diameters: a boattail narrowing toward the tail, or a cone stepping
/// the airframe up or down. Its fore end takes the diameter of the body part before it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    shape: NoseShape,
    length_m: f64,
    aft_diameter_m: f64,
    wall: Wall,
    material: Material,
    name: String,
    /// The fore diameter, m, where the transition states its own: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fore_diameter_m: Option<f64>,
    /// Whether its profile is cut from a whole nose cone ([`hpr_design::Transition::clipped`]).
    #[serde(default)]
    clipped: bool,
    /// The shoulder ahead of its fore end, its outer radius stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fore_shoulder: Option<Shoulder>,
    /// The shoulder behind its aft end, its outer radius stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    aft_shoulder: Option<Shoulder>,
    /// The mass the transition weighs, kg, where it states one: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mass_kg: Option<f64>,
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
            fore_diameter_m: None,
            clipped: false,
            fore_shoulder: None,
            aft_shoulder: None,
            mass_kg: None,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fins {
    set: FinSet,
    position: Position,
    name: String,
}

impl Fins {
    /// `count` fins of the shape `planform`, `thickness_m` thick. The planform names its
    /// dimensions: a trapezoid's root and tip chords, its span, and how far aft of the root's
    /// leading edge the tip's is ([`FinPlanform`]).
    ///
    /// ```
    /// use hpr::FinPlanform;
    /// use hpr::rocket::{Fins, material};
    ///
    /// let planform = FinPlanform::Trapezoidal {
    ///     root_chord_m: 0.1,
    ///     tip_chord_m: 0.04,
    ///     span_m: 0.045,
    ///     sweep_m: 0.05,
    /// };
    /// let fins = Fins::new(3, planform, 0.003175, material("birch_plywood")?);
    /// # Ok::<(), hpr::Error>(())
    /// ```
    #[must_use]
    pub fn new(count: u32, planform: FinPlanform, thickness_m: f64, material: Material) -> Self {
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

    /// The same fins at `position` along their tube: [`Position`] places their root's leading
    /// edge (`Top`, `After`, `Absolute`), its trailing edge (`Bottom`) or its middle (`Middle`).
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorTube {
    length_m: f64,
    inner_diameter_m: f64,
    wall_m: f64,
    overhang_m: f64,
    position: Position,
    material: Material,
    name: String,
    /// The mass the tube weighs, kg, where it states one: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mass_kg: Option<f64>,
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
            mass_kg: None,
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
/// It is a solid cylinder along the axis, as long and as wide as its packing, or a point until
/// [`Mass::packed`] gives it a size. Its [`Position`] places the packing's fore end (`Top`,
/// `After`, `Absolute`), its aft end (`Bottom`) or its middle (`Middle`). So packing a mass
/// moves its centre by half the packed length, unless it is placed by its middle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mass {
    mass_kg: f64,
    position: Position,
    packed_length_m: f64,
    packed_diameter_m: f64,
    name: String,
}

impl Mass {
    /// `mass_kg` at `position` along the body tube it is in.
    #[must_use]
    pub fn new(mass_kg: f64, position: Position) -> Self {
        Self {
            mass_kg,
            position,
            packed_length_m: 0.0,
            packed_diameter_m: 0.0,
            name: String::new(),
        }
    }

    /// The same mass packed in a cylinder `length_m` long and `diameter_m` across.
    #[must_use]
    pub fn packed(mut self, length_m: f64, diameter_m: f64) -> Self {
        self.packed_length_m = length_m;
        self.packed_diameter_m = diameter_m;
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
/// It serializes, for a record of what was flown, but doesn't deserialize: the builder's methods
/// check what goes in, so a rocket is built with them, or read from a design with
/// [`Rocket::from_design`].
///
/// ```
/// use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
/// use hpr::{FinPlanform, Motor, NoseShape, Position, Rocket};
///
/// let mut rocket = Rocket::new("Small", 0.0563)?;
/// let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
/// let planform = FinPlanform::Trapezoidal {
///     root_chord_m: 0.1,
///     tip_chord_m: 0.04,
///     span_m: 0.045,
///     sweep_m: 0.05,
/// };
/// rocket
///     .add_nose(Nose::hollow(ogive, 0.22, 0.0015, material("abs")?))?
///     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
///     .add_fins(Fins::new(3, planform, 0.003175, material("birch_plywood")?))?
///     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
///     .set_motor(Motor::from_catalog("H54")?)?;
///
/// // Unstable as it stands: the motor's weight puts the centre of gravity behind the centre of
/// // pressure, and the margin, in calibres, is negative.
/// assert!(rocket.static_margin_cal(0.0, 0.3)?.is_some_and(|margin| margin < 0.0));
/// // 200 g near the top of the tube, a recovery bay, brings it forward of it.
/// rocket.add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?;
/// assert!(rocket.static_margin_cal(0.0, 0.3)?.is_some_and(|margin| margin > 1.0));
/// # Ok::<(), hpr::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rocket {
    design: hpr_design::Rocket,
    configuration_id: Option<String>,
    recovery: Vec<Device>,
    /// Where the builder has got to; `None` for a rocket read from a design. Not part of the
    /// record.
    #[serde(skip)]
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
    /// An empty rocket called `name`, of outer body diameter `diameter_m`: the nose's base
    /// diameter, and the first tube's, unless they say otherwise (a catalogue part states its
    /// own). Its reference diameter, the one
    /// the stability margin is counted in, is its largest body diameter.
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

    /// A rocket from a design you already have, flown in its configuration `configuration_id`:
    /// a design file read with `serde_json`, an OpenRocket file read with [`hpr_io::ork`], or a
    /// built rocket's [`Rocket::design`] changed with [`hpr_design`] to hold what the builder
    /// can't, such as a cluster or pods. Parts can't be added to it; recovery devices can.
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
    /// - [`Error::Domain`] for a diameter, length, wall or shoulder dimension that isn't finite
    ///   and positive, a shoulder wall thicker than its stated radius, or a stated mass that is
    ///   negative or not finite; [`Error::Design`] for a shape whose parameter is out of its
    ///   range.
    pub fn add_nose(&mut self, nose: Nose) -> Result<&mut Self, Error> {
        let build = self.build()?;
        if build.aft_radius_m.is_some() {
            return Err(Error::Order(Order::NoseNotFirst));
        }
        let radius_m = 0.5
            * positive(
                "nose diameter, m",
                nose.diameter_m.unwrap_or(build.diameter_m),
            )?;
        positive("nose length, m", nose.length_m)?;
        check_wall("nose wall, m", nose.wall)?;
        Profile::nose(nose.shape, nose.length_m, radius_m)?;
        let mut auto = Vec::new();
        if let Some(shoulder) = &nose.shoulder {
            check_shoulder(&NOSE_SHOULDER, shoulder)?;
            if shoulder.outer_radius_m == 0.0 {
                auto.push(AutoDimension::ShoulderRadius);
            }
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
        component.overrides.mass_kg = stated_mass(nose.mass_kg)?;
        self.push_body(component, radius_m, false)?;
        Ok(self)
    }

    /// Adds a body tube behind the last body part.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if the rocket was read from a design.
    /// - [`Error::Domain`] for a length, wall or diameter that isn't finite and positive, or a
    ///   stated mass that is negative or not finite.
    pub fn add_tube(&mut self, tube: Tube) -> Result<&mut Self, Error> {
        let build = self.build()?;
        let before_m = build
            .aft_radius_m
            .map_or(build.diameter_m, |radius_m| 2.0 * radius_m);
        let diameter_m = positive("tube diameter, m", tube.diameter_m.unwrap_or(before_m))?;
        let part = Part::BodyTube(BodyTube {
            length_m: positive("tube length, m", tube.length_m)?,
            outer_radius_m: 0.5 * diameter_m,
            thickness_m: positive("tube wall, m", tube.wall_m)?,
            material: tube.material,
        });
        let mut component = self.component("tube", &tube.name, part, None);
        component.overrides.mass_kg = stated_mass(tube.mass_kg)?;
        self.push_body(component, 0.5 * diameter_m, true)?;
        Ok(self)
    }

    /// Adds a transition behind the last body part, starting at its diameter unless it states its
    /// own (a catalogue part does).
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body part before it, or the rocket was read from a
    ///   design.
    /// - [`Error::Domain`] for a length, wall, stated fore diameter or shoulder dimension that
    ///   isn't finite and positive, an aft diameter that is negative or not finite, a shoulder
    ///   wall thicker than its radius, or a stated mass that is negative or not finite;
    ///   [`Error::Design`] for a shape whose parameter is out of its range.
    pub fn add_transition(&mut self, transition: Transition) -> Result<&mut Self, Error> {
        let before_m = self
            .build()?
            .aft_radius_m
            .ok_or(Error::Order(Order::NothingBeforeTransition))?;
        let fore_radius_m = match transition.fore_diameter_m {
            Some(diameter_m) => 0.5 * positive("transition fore diameter, m", diameter_m)?,
            None => before_m,
        };
        positive("transition length, m", transition.length_m)?;
        check_wall("transition wall, m", transition.wall)?;
        let aft_radius_m =
            0.5 * non_negative("transition aft diameter, m", transition.aft_diameter_m)?;
        Profile::transition(
            transition.shape,
            transition.length_m,
            fore_radius_m,
            aft_radius_m,
            transition.clipped,
        )?;
        for shoulder in [&transition.fore_shoulder, &transition.aft_shoulder]
            .into_iter()
            .flatten()
        {
            // A transition's shoulders state their radius: the builder finds none for them.
            positive("transition shoulder radius, m", shoulder.outer_radius_m)?;
            check_shoulder(&TRANSITION_SHOULDER, shoulder)?;
        }
        let part = Part::Transition(hpr_design::Transition {
            shape: transition.shape,
            clipped: transition.clipped,
            length_m: transition.length_m,
            fore_radius_m,
            aft_radius_m,
            wall: transition.wall,
            fore_shoulder: transition.fore_shoulder,
            aft_shoulder: transition.aft_shoulder,
            material: transition.material,
        });
        let mut component = self.component("transition", &transition.name, part, None);
        component.overrides.mass_kg = stated_mass(transition.mass_kg)?;
        self.push_body(component, aft_radius_m, false)?;
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
    ///   or stated mass that is negative or not finite, or a position that isn't finite.
    pub fn add_motor_tube(&mut self, tube: MotorTube) -> Result<&mut Self, Error> {
        if self.build()?.motor_tube.is_some() {
            return Err(Error::Order(Order::SecondMotorTube));
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
        component.overrides.mass_kg = stated_mass(tube.mass_kg)?;
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
        non_negative("packed length, m", mass.packed_length_m)?;
        non_negative("packed diameter, m", mass.packed_diameter_m)?;
        check_position(mass.position)?;
        let part = Part::MassComponent(MassComponent {
            mass_kg: mass.mass_kg,
            packing: Packing {
                length_m: mass.packed_length_m,
                radius_m: 0.5 * mass.packed_diameter_m,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
        });
        let component = self.component("mass", &mass.name, part, Some(mass.position));
        self.attach(component)?;
        Ok(self)
    }

    /// Adds a fitting to the last body tube: a coupler, a centering ring, a bulkhead, a launch
    /// lug, or a packed parachute or streamer.
    ///
    /// # Errors
    ///
    /// - [`Error::Order`] if there is no body tube yet, or the rocket was read from a design.
    /// - [`Error::Design`] for a part the design can't weigh: a dimension that isn't finite and
    ///   positive, a ring or tube whose bore isn't narrower than its outside, a material of the
    ///   wrong kind.
    /// - [`Error::Domain`] for a stated mass that is negative or not finite, or a position that
    ///   isn't finite.
    pub fn add_fitting(&mut self, fitting: Fitting) -> Result<&mut Self, Error> {
        let id = fitting.id();
        let (part, position, name, mass_kg) = fitting.into_parts();
        check_position(position)?;
        let mass_kg = stated_mass(mass_kg)?;
        // Weighed now, on the tube it goes on, so a part the design can't take is refused where
        // it is added.
        part.mass_properties(Some(self.tube_radius_m()?))?;
        let mut component = self.component(id, &name, part, Some(position));
        component.overrides.mass_kg = mass_kg;
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
            .ok_or(Error::Order(Order::NoMotorTube))?;
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
    /// [`Trigger::MotorDelay`](hpr_sim::Trigger::MotorDelay) with motor 0, the first motor, opens
    /// it at the motor's ejection charge, which needs the motor's delay set
    /// ([`Motor::with_delay_s`]). The device adds drag, not mass: add its mass with
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

    /// The parts placed and the motor in its tube ([`hpr_design::Rocket::assemble`]), once the
    /// design's checks ([`hpr_design::checks`]) find no errors: the same checks a flight runs
    /// with the default settings, so what this weighs is what [`crate::Flight`] would fly. A
    /// flight told to accept a design's errors
    /// ([`FlightSettings::accept_design_errors`](hpr_sim::FlightSettings::accept_design_errors))
    /// flies what this refuses; `design().assemble(id)` weighs it.
    ///
    /// # Errors
    ///
    /// - [`Error::NoMotor`] before the rocket has a motor.
    /// - [`Error::DesignChecks`] with every finding if the checks find an error, such as a motor
    ///   wider than its tube's bore or a motor tube wider than the airframe.
    /// - [`Error::Design`] for a tree that doesn't hold together.
    pub fn assemble(&self) -> Result<Assembly, Error> {
        let id = self.configuration_id.as_deref().ok_or(Error::NoMotor)?;
        let findings = check(&self.design)?;
        if has_errors(&findings) {
            return Err(Error::DesignChecks(findings));
        }
        Ok(self.design.assemble(id)?)
    }

    /// The mass, centre of gravity and inertia `time_s` seconds after the motor lights, the
    /// motor burning as its thrust curve says. The centre of gravity is in the body frame: `z`
    /// points to the nose and is zero at its tip, so a point `s` metres aft of the tip is at
    /// `z = −s` (`docs/physics/frames.md`).
    ///
    /// # Errors
    ///
    /// As [`Rocket::assemble`], and [`Error::Domain`] for a time that is negative or not finite.
    pub fn mass_properties(&self, time_s: f64) -> Result<MassProperties, Error> {
        let time_s = non_negative("time, s", time_s)?;
        Ok(self.assemble()?.mass_properties(time_s))
    }

    /// The static stability margin `time_s` seconds after the motor lights, at Mach `mach` with
    /// the air along the axis, calibres of the reference diameter: how far the centre of pressure
    /// is behind the centre of gravity. `None` where [`hpr_sim::metrics::margin`] leaves it
    /// undefined: a normal force that doesn't restore, or a centre of pressure too ill-conditioned
    /// to place. [`Rocket::margin`] gives the rest of it.
    ///
    /// # Errors
    ///
    /// As [`Rocket::margin`].
    pub fn static_margin_cal(&self, time_s: f64, mach: f64) -> Result<Option<f64>, Error> {
        Ok(self.margin(time_s, mach)?.margin_cal)
    }

    /// The centre of pressure and the static margin `time_s` seconds after the motor lights, at
    /// Mach `mach` with the air along the axis ([`hpr_sim::metrics::margin`]). Its centre of
    /// pressure is a station: metres aft of the nose tip, positive.
    ///
    /// # Errors
    ///
    /// As [`Rocket::mass_properties`], [`Error::Aero`] for a rocket the aerodynamic model can't
    /// take, and [`Error::Sim`] for a Mach number out of its range.
    pub fn margin(&self, time_s: f64, mach: f64) -> Result<Margin, Error> {
        let time_s = non_negative("time, s", time_s)?;
        let assembly = self.assemble()?;
        let cg_station_m = -assembly.mass_properties(time_s).cg_m.z;
        let aero = AeroModel::new(&assembly.layout)?;
        Ok(metrics::margin(&aero, &Flow::axial(mach), cg_station_m)?)
    }

    /// The builder's place, or [`Error::Order`] for a rocket read from a design.
    fn build(&self) -> Result<&Build, Error> {
        self.build
            .as_ref()
            .ok_or(Error::Order(Order::ReadFromDesign))
    }

    /// The builder's stage: its one, the first.
    fn stage(&mut self) -> Result<&mut Stage, Error> {
        self.design
            .stages
            .first_mut()
            .ok_or(Error::Order(Order::ReadFromDesign))
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
    fn push_body(
        &mut self,
        component: Component,
        aft_radius_m: f64,
        tube: bool,
    ) -> Result<(), Error> {
        let components = &mut self.stage()?.components;
        components.push(component);
        let index = components.len() - 1;
        if let Some(build) = &mut self.build {
            build.aft_radius_m = Some(aft_radius_m);
            if tube {
                build.tube = Some(index);
            }
        }
        Ok(())
    }

    /// The last body tube's outer radius, m.
    fn tube_radius_m(&self) -> Result<f64, Error> {
        let index = self.build()?.tube.ok_or(Error::Order(Order::NoTube))?;
        match self
            .design
            .stages
            .first()
            .and_then(|stage| stage.components.get(index))
            .map(|component| &component.part)
        {
            Some(Part::BodyTube(tube)) => Ok(tube.outer_radius_m),
            _ => Err(Error::Order(Order::NoTube)),
        }
    }

    /// Attaches `component` to the last body tube.
    fn attach(&mut self, component: Component) -> Result<(), Error> {
        let index = self.build()?.tube.ok_or(Error::Order(Order::NoTube))?;
        self.stage()?
            .components
            .get_mut(index)
            .ok_or(Error::Order(Order::NoTube))?
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

/// A shoulder's length and wall, checked finite and positive, and, where its outer radius is
/// stated (zero is the tube's, found later), that radius too, with the wall no thicker than it.
/// `names` names those four numbers in an error: [`NOSE_SHOULDER`], [`TRANSITION_SHOULDER`].
fn check_shoulder(names: &[&'static str; 4], shoulder: &Shoulder) -> Result<(), Error> {
    let [length, wall, radius, past] = *names;
    positive(length, shoulder.length_m)?;
    positive(wall, shoulder.thickness_m)?;
    if shoulder.outer_radius_m != 0.0 {
        positive(radius, shoulder.outer_radius_m)?;
        if shoulder.thickness_m > shoulder.outer_radius_m {
            return Err(Error::Domain {
                what: past,
                value: shoulder.thickness_m,
            });
        }
    }
    Ok(())
}

/// A nose shoulder's numbers, as [`check_shoulder`] names them.
const NOSE_SHOULDER: [&str; 4] = [
    "nose shoulder length, m",
    "nose shoulder wall, m",
    "nose shoulder radius, m",
    "nose shoulder wall past its radius, m",
];

/// A transition shoulder's numbers, as [`check_shoulder`] names them.
const TRANSITION_SHOULDER: [&str; 4] = [
    "transition shoulder length, m",
    "transition shoulder wall, m",
    "transition shoulder radius, m",
    "transition shoulder wall past its radius, m",
];

/// A part's stated mass, if it has one, checked finite and not negative: the mass override that
/// makes it weigh that.
fn stated_mass(mass_kg: Option<f64>) -> Result<Option<f64>, Error> {
    mass_kg
        .map(|mass_kg| non_negative("stated mass, kg", mass_kg))
        .transpose()
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
