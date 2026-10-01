//! Parts from a parts catalogue, and the fittings they add to the builder.
//!
//! A catalogue part ([`hpr_io::orc::Part`], such as one found in OpenRocket's catalogue with
//! [`hpr_io::orc::bundled`]) becomes a builder part with the `from_catalog` of the part it is:
//!
//! | Catalogue kind | Builder part |
//! |---|---|
//! | nose cone | [`Nose::from_catalog`] |
//! | body tube | [`Tube::from_catalog`], or [`MotorTube::from_catalog`] for a motor tube |
//! | transition | [`Transition::from_catalog`] |
//! | tube coupler, engine block, centering ring, bulkhead, launch lug, parachute, streamer | [`Fitting::from_catalog`] |
//!
//! Each takes the catalogue's sizes, its material with the file's density, and its name, the
//! maker and part number. A part that states its mass weighs that: the mass is the component's
//! mass override ([`hpr_design::Overrides::mass_kg`]), which scales its density, so its centre
//! of mass stays where its shape puts it. OpenRocket does the same for a solid part, by changing
//! its density, and for a parachute, by overriding its mass; it leaves a streamer's stated mass
//! unused, and the one streamer in its catalogue that states a mass weighs it here.
//!
//! A catalogue leaves some dimensions unsaid. Each is chosen as OpenRocket 24.12 chooses it when
//! it builds the part, as measured by `validation/oracles/openrocket/orc_built.py`, but for one:
//!
//! - a shape's parameter: a tangent ogive (`radius_ratio` 1), a parabola (`K′` 1), a von Kármán
//!   Haack series (`C` 0) and a power series of exponent ½;
//! - a transition's profile is clipped (cut from a whole nose cone, [`hpr_design::shapes`]) for
//!   the elliptical, Haack and power series, and not for the others;
//! - a filled part's shoulders are solid;
//! - **a hollow part's shoulders have its wall**, or are solid where the wall is thicker than
//!   their radius. OpenRocket gives them a wall of zero, so they weigh nothing; a molded nose
//!   cone's shoulder is a tube of the same plastic.
//!
//! The builder refuses a part whose file names a material it doesn't define
//! ([`CatalogProblem::UndefinedMaterial`]), where OpenRocket weighs it as zero. A parachute whose
//! file names no line material has lines that weigh nothing, as in OpenRocket; the six in its
//! catalogue state their mass.

use hpr_design::{
    CenteringRing, InnerTube, LaunchLug, Material, NoseShape, Packing, Parachute, Part, Position,
    Shoulder, Streamer, Wall,
};
use hpr_io::orc::{self, MaterialRef, PartKind, Shape};
use serde::{Deserialize, Serialize};

use super::{MotorTube, Nose, Transition, Tube};
use crate::error::{CatalogProblem, Error};

/// A part fitted to the last body tube: a coupler, a centering ring, a bulkhead, a launch lug,
/// or a packed parachute or streamer. It sits flush with the tube's aft end unless placed
/// elsewhere with [`Fitting::at`].
///
/// A coupler, a ring or a bulkhead goes inside the tube; a launch lug on its outside. A
/// parachute or a streamer from a catalogue is its mass, packed into a point; its drag is a
/// recovery device ([`super::Rocket::add_parachute`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fitting {
    part: Part,
    position: Position,
    name: String,
    /// The mass the part weighs, kg, where it states one: a catalogue part's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mass_kg: Option<f64>,
}

impl Fitting {
    fn new(part: Part) -> Self {
        Self {
            part,
            position: Position::Bottom { aft_offset_m: 0.0 },
            name: String::new(),
            mass_kg: None,
        }
    }

    /// A tube inside the body tube, `length_m` long, `outer_diameter_m` across, with a wall
    /// `wall_m` thick: a coupler joining two tubes, or an engine block.
    #[must_use]
    pub fn coupler(length_m: f64, outer_diameter_m: f64, wall_m: f64, material: Material) -> Self {
        Self::new(Part::InnerTube(InnerTube {
            length_m,
            outer_radius_m: 0.5 * outer_diameter_m,
            thickness_m: wall_m,
            radial_offset_m: 0.0,
            angle_rad: 0.0,
            material,
            cluster_m: Vec::new(),
        }))
    }

    /// A centering ring `length_m` thick, `outer_diameter_m` across with a hole
    /// `inner_diameter_m` across.
    #[must_use]
    pub fn centering_ring(
        length_m: f64,
        outer_diameter_m: f64,
        inner_diameter_m: f64,
        material: Material,
    ) -> Self {
        Self::new(Part::CenteringRing(CenteringRing {
            length_m,
            outer_radius_m: 0.5 * outer_diameter_m,
            inner_radius_m: 0.5 * inner_diameter_m,
            material,
        }))
    }

    /// A bulkhead: a solid disc `length_m` thick and `diameter_m` across.
    #[must_use]
    pub fn bulkhead(length_m: f64, diameter_m: f64, material: Material) -> Self {
        Self::new(Part::CenteringRing(CenteringRing::bulkhead(
            length_m,
            0.5 * diameter_m,
            material,
        )))
    }

    /// A launch lug on the outside of the tube, `length_m` long, `outer_diameter_m` across, with
    /// a wall `wall_m` thick.
    #[must_use]
    pub fn launch_lug(
        length_m: f64,
        outer_diameter_m: f64,
        wall_m: f64,
        material: Material,
    ) -> Self {
        Self::new(Part::LaunchLug(LaunchLug {
            length_m,
            outer_radius_m: 0.5 * outer_diameter_m,
            thickness_m: wall_m,
            angle_rad: 0.0,
            count: 1,
            spacing_m: 0.0,
            material,
        }))
    }

    /// The catalogue part `part`, a tube coupler, engine block, centering ring, bulkhead, launch
    /// lug, parachute or streamer, as the [module](self) says, named by its maker and number.
    /// An engine block is a coupler; a parachute's or streamer's packing is a point.
    ///
    /// # Errors
    ///
    /// [`Error::Catalog`]: [`CatalogProblem::Kind`] for a part of another kind, and
    /// [`CatalogProblem::UndefinedMaterial`] for one whose material its file doesn't define.
    pub fn from_catalog(part: &orc::Part) -> Result<Self, Error> {
        let built = match &part.kind {
            PartKind::TubeCoupler(tube) | PartKind::EngineBlock(tube) => Self::coupler(
                tube.length_m,
                tube.outer_diameter_m,
                tube.thickness_m(),
                material(part, &tube.material)?,
            ),
            PartKind::CenteringRing(tube) => Self::centering_ring(
                tube.length_m,
                tube.outer_diameter_m,
                tube.inner_diameter_m,
                material(part, &tube.material)?,
            ),
            PartKind::Bulkhead(bulkhead) => Self::bulkhead(
                bulkhead.length_m,
                bulkhead.outer_diameter_m,
                material(part, &bulkhead.material)?,
            ),
            PartKind::LaunchLug(tube) => Self::launch_lug(
                tube.length_m,
                tube.outer_diameter_m,
                tube.thickness_m(),
                material(part, &tube.material)?,
            ),
            PartKind::Parachute(chute) => {
                let line_material = match &chute.line_material {
                    Some(line) => material(part, line)?,
                    // No line material named: lines that weigh nothing, as in OpenRocket.
                    None => Material::line("none named", 0.0),
                };
                Self::new(Part::Parachute(Parachute {
                    diameter_m: chute.diameter_m,
                    canopy_material: material(part, &chute.material)?,
                    line_count: chute.line_count,
                    line_length_m: chute.line_length_m,
                    line_material,
                    packing: POINT,
                }))
            }
            PartKind::Streamer(streamer) => Self::new(Part::Streamer(Streamer {
                length_m: streamer.length_m,
                width_m: streamer.width_m,
                material: material(part, &streamer.material)?,
                packing: POINT,
            })),
            _ => return Err(wrong_kind(part, "Fitting::from_catalog")),
        };
        Ok(Self {
            name: label(part),
            mass_kg: part.mass_kg,
            ..built
        })
    }

    /// The same fitting at `position` along the body tube it is on.
    #[must_use]
    pub fn at(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// The same fitting with `name`, which the design file keeps.
    #[must_use]
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        self
    }

    /// The design's component id for the fitting's kind, before any number added to make it
    /// unique.
    pub(super) fn id(&self) -> &'static str {
        match &self.part {
            Part::InnerTube(_) => "coupler",
            Part::CenteringRing(ring) if ring.inner_radius_m == 0.0 => "bulkhead",
            Part::CenteringRing(_) => "ring",
            Part::LaunchLug(_) => "launch-lug",
            Part::Parachute(_) => "parachute",
            Part::Streamer(_) => "streamer",
            _ => "fitting",
        }
    }

    /// Its parts: the design's part, where it sits, its name and its stated mass.
    pub(super) fn into_parts(self) -> (Part, Position, String, Option<f64>) {
        (self.part, self.position, self.name, self.mass_kg)
    }
}

/// A packing of no size: a point at the fitting's position.
const POINT: Packing = Packing {
    length_m: 0.0,
    radius_m: 0.0,
    radial_offset_m: 0.0,
    angle_rad: 0.0,
};

impl Nose {
    /// The catalogue nose cone `part`, as the [module](self) says: its base its own diameter, its
    /// shoulder (if any) its own radius, named by its maker and number.
    ///
    /// # Errors
    ///
    /// [`Error::Catalog`]: [`CatalogProblem::Kind`] for a part of another kind,
    /// [`CatalogProblem::UndefinedMaterial`] for one whose material its file doesn't define, and
    /// [`CatalogProblem::NoWall`] for one neither filled nor given a wall, and
    /// [`CatalogProblem::Shape`] for a shape the builder doesn't know.
    pub fn from_catalog(part: &orc::Part) -> Result<Self, Error> {
        let PartKind::NoseCone(nose) = &part.kind else {
            return Err(wrong_kind(part, "Nose::from_catalog"));
        };
        let wall = wall(part, nose.filled, nose.thickness_m)?;
        Ok(Self {
            shape: shape(part, nose.shape)?,
            length_m: nose.length_m,
            wall,
            shoulder: shoulder(nose.shoulder_diameter_m, nose.shoulder_length_m, wall),
            material: material(part, &nose.material)?,
            name: label(part),
            diameter_m: Some(nose.outer_diameter_m),
            mass_kg: part.mass_kg,
        })
    }
}

impl Tube {
    /// The catalogue body tube `part`: its length, outer diameter and wall, named by its maker
    /// and number. A catalogue tube is often sold longer than it is flown:
    /// [`Tube::with_length_m`] cuts it.
    ///
    /// # Errors
    ///
    /// [`Error::Catalog`]: [`CatalogProblem::Kind`] for a part of another kind, and
    /// [`CatalogProblem::UndefinedMaterial`] for one whose material its file doesn't define.
    pub fn from_catalog(part: &orc::Part) -> Result<Self, Error> {
        let PartKind::BodyTube(tube) = &part.kind else {
            return Err(wrong_kind(part, "Tube::from_catalog"));
        };
        Ok(Self {
            length_m: tube.length_m,
            wall_m: tube.thickness_m(),
            diameter_m: Some(tube.outer_diameter_m),
            material: material(part, &tube.material)?,
            name: label(part),
            mass_kg: part.mass_kg,
        })
    }

    /// The same tube cut to `length_m`. A stated mass is cut in proportion, the tube being the
    /// same all along.
    #[must_use]
    pub fn with_length_m(mut self, length_m: f64) -> Self {
        self.mass_kg = self
            .mass_kg
            .map(|mass_kg| mass_kg * length_m / self.length_m);
        self.length_m = length_m;
        self
    }
}

impl MotorTube {
    /// The catalogue body tube `part` as the motor tube: its length, bore and wall, named by its
    /// maker and number. A catalogue tube is often sold longer than it is flown:
    /// [`MotorTube::with_length_m`] cuts it.
    ///
    /// # Errors
    ///
    /// [`Error::Catalog`]: [`CatalogProblem::Kind`] for a part of another kind, and
    /// [`CatalogProblem::UndefinedMaterial`] for one whose material its file doesn't define.
    pub fn from_catalog(part: &orc::Part) -> Result<Self, Error> {
        let PartKind::BodyTube(tube) = &part.kind else {
            return Err(wrong_kind(part, "MotorTube::from_catalog"));
        };
        Ok(Self {
            name: label(part),
            mass_kg: part.mass_kg,
            ..Self::new(
                tube.length_m,
                tube.inner_diameter_m,
                tube.thickness_m(),
                material(part, &tube.material)?,
            )
        })
    }

    /// The same tube cut to `length_m`. A stated mass is cut in proportion, the tube being the
    /// same all along.
    #[must_use]
    pub fn with_length_m(mut self, length_m: f64) -> Self {
        self.mass_kg = self
            .mass_kg
            .map(|mass_kg| mass_kg * length_m / self.length_m);
        self.length_m = length_m;
        self
    }
}

impl Transition {
    /// The catalogue transition `part`, as the [module](self) says: both ends their own
    /// diameters, each shoulder (if any) its own radius, named by its maker and number.
    ///
    /// # Errors
    ///
    /// [`Error::Catalog`]: [`CatalogProblem::Kind`] for a part of another kind,
    /// [`CatalogProblem::UndefinedMaterial`] for one whose material its file doesn't define, and
    /// [`CatalogProblem::NoWall`] for one neither filled nor given a wall, and
    /// [`CatalogProblem::Shape`] for a shape the builder doesn't know.
    pub fn from_catalog(part: &orc::Part) -> Result<Self, Error> {
        let PartKind::Transition(transition) = &part.kind else {
            return Err(wrong_kind(part, "Transition::from_catalog"));
        };
        let wall = wall(part, transition.filled, transition.thickness_m)?;
        Ok(Self {
            shape: shape(part, transition.shape)?,
            length_m: transition.length_m,
            aft_diameter_m: transition.aft_outer_diameter_m,
            wall,
            material: material(part, &transition.material)?,
            name: label(part),
            fore_diameter_m: Some(transition.fore_outer_diameter_m),
            clipped: clipped(transition.shape),
            fore_shoulder: shoulder(
                transition.fore_shoulder_diameter_m,
                transition.fore_shoulder_length_m,
                wall,
            ),
            aft_shoulder: shoulder(
                transition.aft_shoulder_diameter_m,
                transition.aft_shoulder_length_m,
                wall,
            ),
            mass_kg: part.mass_kg,
        })
    }
}

/// A catalogue part's name in the design: its maker and part number.
fn label(part: &orc::Part) -> String {
    format!("{} {}", part.manufacturer, part.part_number)
}

/// The refusal of a part of a kind `builder` doesn't make.
fn wrong_kind(part: &orc::Part, builder: &'static str) -> Error {
    let found = match &part.kind {
        PartKind::BodyTube(_) => "body tube",
        PartKind::TubeCoupler(_) => "tube coupler",
        PartKind::EngineBlock(_) => "engine block",
        PartKind::CenteringRing(_) => "centering ring",
        PartKind::LaunchLug(_) => "launch lug",
        PartKind::Bulkhead(_) => "bulkhead",
        PartKind::NoseCone(_) => "nose cone",
        PartKind::Transition(_) => "transition",
        PartKind::Parachute(_) => "parachute",
        PartKind::Streamer(_) => "streamer",
        _ => "part of a kind the builder doesn't know",
    };
    catalog_error(part, CatalogProblem::Kind { found, builder })
}

fn catalog_error(part: &orc::Part, problem: CatalogProblem) -> Error {
    Error::Catalog {
        part: format!("{} ({})", label(part), part.file),
        problem,
    }
}

/// The part's material with its file's density.
fn material(part: &orc::Part, material: &MaterialRef) -> Result<Material, Error> {
    material.material().ok_or_else(|| {
        catalog_error(
            part,
            CatalogProblem::UndefinedMaterial(material.name.clone()),
        )
    })
}

/// A nose cone's or transition's wall: solid if the file says it is filled, else the wall it
/// gives.
fn wall(part: &orc::Part, filled: Option<bool>, thickness_m: Option<f64>) -> Result<Wall, Error> {
    match (filled, thickness_m) {
        (Some(true), _) => Ok(Wall::Filled {}),
        (_, Some(thickness_m)) => Ok(Wall::Shell { thickness_m }),
        (_, None) => Err(catalog_error(part, CatalogProblem::NoWall)),
    }
}

/// A shoulder `diameter_m` across and `length_m` long, if it has both, open at its end: solid in
/// a filled part, its part's wall in a hollow one, solid where that wall is thicker than its
/// radius.
fn shoulder(diameter_m: f64, length_m: f64, wall: Wall) -> Option<Shoulder> {
    if !(diameter_m > 0.0 && length_m > 0.0) {
        return None;
    }
    let radius_m = 0.5 * diameter_m;
    let thickness_m = match wall {
        Wall::Filled {} => radius_m,
        Wall::Shell { thickness_m } => thickness_m.min(radius_m),
    };
    Some(Shoulder {
        length_m,
        outer_radius_m: radius_m,
        thickness_m,
        capped: false,
    })
}

/// The profile OpenRocket gives a catalogue shape, whose file gives no parameter.
fn shape(part: &orc::Part, shape: Shape) -> Result<NoseShape, Error> {
    Ok(match shape {
        Shape::Conical => NoseShape::Conical {},
        Shape::Ogive => NoseShape::TANGENT_OGIVE,
        Shape::Ellipsoid => NoseShape::Elliptical {},
        Shape::Parabolic => NoseShape::ParabolicSeries { parameter: 1.0 },
        Shape::Haack => NoseShape::VON_KARMAN,
        Shape::Power => NoseShape::PowerSeries { exponent: 0.5 },
        _ => return Err(catalog_error(part, CatalogProblem::Shape)),
    })
}

/// Whether OpenRocket clips a catalogue transition of `shape`.
fn clipped(shape: Shape) -> bool {
    matches!(shape, Shape::Ellipsoid | Shape::Haack | Shape::Power)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        reason = "tests state their expectations by unwrapping and panicking"
    )]

    use hpr_design::DesignError;
    use hpr_io::orc::bundled;

    use super::*;
    use crate::rocket::material;
    use crate::{Order, Rocket};

    /// The one bundled part `maker` numbers `number`.
    fn find(maker: &str, number: &str) -> &'static orc::Part {
        let found = bundled().find(maker, number);
        assert_eq!(found.len(), 1, "{maker} {number}");
        found[0]
    }

    /// The rocket's laid-out component `id`: its mass, kg.
    fn mass_kg(rocket: &Rocket, id: &str) -> f64 {
        let layout = rocket.design().layout().unwrap();
        layout.find(id).unwrap().1.own.mass_kg
    }

    #[test]
    fn a_cut_tube_keeps_its_stated_mass_per_length() {
        let part = bundled()
            .parts
            .iter()
            .find(|part| matches!(part.kind, PartKind::BodyTube(_)) && part.mass_kg.is_some())
            .expect("a body tube that states its mass");
        let stated_kg = part.mass_kg.unwrap();
        let tube = Tube::from_catalog(part).unwrap();
        let quarter_m = 0.25 * tube.length_m;
        let mut whole = Rocket::new("whole", 0.1).unwrap();
        whole.add_tube(tube.clone()).unwrap();
        let mut cut = Rocket::new("cut", 0.1).unwrap();
        cut.add_tube(tube.with_length_m(quarter_m)).unwrap();
        assert!((mass_kg(&whole, "tube") - stated_kg).abs() <= 1e-15 * stated_kg);
        assert!((mass_kg(&cut, "tube") - 0.25 * stated_kg).abs() <= 1e-15 * stated_kg);

        // A motor tube too, and one with no stated mass weighs its material's, cut or not.
        let motor_tube = MotorTube::from_catalog(part)
            .unwrap()
            .with_length_m(quarter_m);
        assert!((motor_tube.mass_kg.unwrap() - 0.25 * stated_kg).abs() <= 1e-15 * stated_kg);
        let plain = find("LOC Precision", "BT-2.56");
        assert_eq!(
            Tube::from_catalog(plain)
                .unwrap()
                .with_length_m(0.3)
                .mass_kg,
            None
        );
    }

    #[test]
    fn a_catalogue_nose_sets_the_diameter_behind_it() {
        let nose = find("LOC Precision", "PNC-2.56");
        let PartKind::NoseCone(cone) = &nose.kind else {
            panic!("a nose cone");
        };
        let mut rocket = Rocket::new("nose", 0.5).unwrap();
        rocket
            .add_nose(Nose::from_catalog(nose).unwrap())
            .unwrap()
            .add_tube(Tube::new(0.3, 0.001, material("kraft_phenolic").unwrap()))
            .unwrap();
        let layout = rocket.design().layout().unwrap();
        let Part::BodyTube(tube) = &layout.find("tube").unwrap().1.part else {
            panic!("a body tube");
        };
        assert_eq!(tube.outer_radius_m, 0.5 * cone.outer_diameter_m);
        // The rocket's diameter, 0.5 m, is nowhere: the reference is the largest body's.
        assert_eq!(layout.reference_diameter_m, cone.outer_diameter_m);
        assert_eq!(
            layout
                .find("nose")
                .map(|(_, nose)| nose.part.clone())
                .and_then(|part| match part {
                    Part::NoseCone(cone) => Some(cone.base_radius_m),
                    _ => None,
                }),
            Some(0.5 * cone.outer_diameter_m)
        );
    }

    #[test]
    fn shoulders_are_solid_or_the_parts_wall() {
        // A hollow nose: its shoulder has its wall, not OpenRocket's zero.
        let hollow = Nose::from_catalog(find("LOC Precision", "PNC-2.56")).unwrap();
        let Wall::Shell { thickness_m } = hollow.wall else {
            panic!("PNC-2.56 is hollow");
        };
        let shoulder = hollow.shoulder.unwrap();
        assert_eq!(shoulder.thickness_m, thickness_m);
        assert_eq!(shoulder.outer_radius_m, 0.5 * 0.06477);
        // A filled one: solid.
        let filled = Nose::from_catalog(find("SEMROC", "BNC-5RA")).unwrap();
        assert_eq!(filled.wall, Wall::Filled {});
        let shoulder = filled.shoulder.unwrap();
        assert_eq!(shoulder.thickness_m, shoulder.outer_radius_m);
        // A wall thicker than the shoulder's radius leaves it solid.
        let thick = shoulder_of(0.01, 0.02, Wall::Shell { thickness_m: 0.007 });
        assert_eq!(thick.map(|s| s.thickness_m), Some(0.005));
        // No length or no diameter, no shoulder.
        assert_eq!(shoulder_of(0.01, 0.0, Wall::Filled {}), None);
        assert_eq!(shoulder_of(0.0, 0.02, Wall::Filled {}), None);
    }

    fn shoulder_of(diameter_m: f64, length_m: f64, wall: Wall) -> Option<Shoulder> {
        shoulder(diameter_m, length_m, wall)
    }

    #[test]
    fn a_part_naming_an_undefined_material_is_refused() {
        let part = find("SEMROC", "BNC-5NAS");
        let Err(Error::Catalog {
            part: named,
            problem,
        }) = Nose::from_catalog(part)
        else {
            panic!("its material is undefined");
        };
        assert_eq!(
            problem,
            CatalogProblem::UndefinedMaterial("Balsa, bulk, Estes typical".to_owned())
        );
        assert_eq!(named, "SEMROC BNC-5NAS (semroc.orc)");
    }

    #[test]
    fn fittings_go_on_a_tube_and_need_a_bore() {
        let ring = find("LOC Precision", "CR-2.56-38mm");
        let mut rocket = Rocket::new("fittings", 0.0668).unwrap();
        let Err(Error::Order(Order::NoTube)) =
            rocket.add_fitting(Fitting::from_catalog(ring).unwrap())
        else {
            panic!("a fitting goes on a tube");
        };
        rocket
            .add_tube(Tube::from_catalog(find("LOC Precision", "BT-2.56")).unwrap())
            .unwrap();
        let wood = material("birch_plywood").unwrap();
        // A ring whose hole is as wide as it is: refused where it is added, and no part added.
        let Err(Error::Design(DesignError::Geometry(_) | DesignError::Domain { .. })) =
            rocket.add_fitting(Fitting::centering_ring(0.003, 0.06, 0.06, wood.clone()))
        else {
            panic!("a ring needs a bore narrower than its outside");
        };
        assert!(rocket.design().stages[0].components[0].children.is_empty());
        rocket
            .add_fitting(Fitting::from_catalog(ring).unwrap())
            .unwrap()
            .add_fitting(Fitting::bulkhead(0.006, 0.064, wood.clone()))
            .unwrap()
            .add_fitting(Fitting::coupler(0.15, 0.0649, 0.001, wood.clone()))
            .unwrap()
            .add_fitting(Fitting::launch_lug(0.05, 0.008, 0.0005, wood))
            .unwrap()
            .add_fitting(Fitting::from_catalog(find("LOC Precision", "LP-36-2022")).unwrap())
            .unwrap()
            .add_fitting(Fitting::from_catalog(find("LOC Precision", "CR-2.56-38mm")).unwrap())
            .unwrap();
        let ids: Vec<_> = rocket.design().stages[0].components[0]
            .children
            .iter()
            .map(|child| child.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "ring",
                "bulkhead",
                "coupler",
                "launch-lug",
                "parachute",
                "ring-2"
            ]
        );
        // The catalogue's ring weighs its plywood annulus.
        let PartKind::CenteringRing(tube) = &ring.kind else {
            panic!("a ring");
        };
        let annulus_kg = std::f64::consts::PI / 4.0
            * (tube.outer_diameter_m.powi(2) - tube.inner_diameter_m.powi(2))
            * tube.length_m
            * tube.material.density.unwrap();
        assert!((mass_kg(&rocket, "ring") - annulus_kg).abs() <= 1e-15 * annulus_kg);
    }

    #[test]
    fn shapes_take_openrockets_parameters() {
        let part = find("LOC Precision", "PNC-2.56");
        let cases = [
            (Shape::Conical, NoseShape::Conical {}, false),
            (Shape::Ogive, NoseShape::Ogive { radius_ratio: 1.0 }, false),
            (Shape::Ellipsoid, NoseShape::Elliptical {}, true),
            (
                Shape::Parabolic,
                NoseShape::ParabolicSeries { parameter: 1.0 },
                false,
            ),
            (Shape::Haack, NoseShape::Haack { parameter: 0.0 }, true),
            (Shape::Power, NoseShape::PowerSeries { exponent: 0.5 }, true),
        ];
        for (catalogue, ours, clips) in cases {
            assert_eq!(shape(part, catalogue).unwrap(), ours, "{catalogue:?}");
            assert_eq!(clipped(catalogue), clips, "{catalogue:?}");
        }
    }

    #[test]
    fn a_part_of_another_kind_names_both() {
        let chute = find("LOC Precision", "LP-36-2022");
        for (result, builder) in [
            (Nose::from_catalog(chute).err(), "Nose::from_catalog"),
            (Tube::from_catalog(chute).err(), "Tube::from_catalog"),
            (
                MotorTube::from_catalog(chute).err(),
                "MotorTube::from_catalog",
            ),
            (
                Transition::from_catalog(chute).err(),
                "Transition::from_catalog",
            ),
        ] {
            let Some(Error::Catalog {
                problem:
                    CatalogProblem::Kind {
                        found,
                        builder: named,
                    },
                ..
            }) = result
            else {
                panic!("{builder} takes no parachute");
            };
            assert_eq!((found, named), ("parachute", builder));
        }
        let nose = find("LOC Precision", "PNC-2.56");
        let Err(Error::Catalog {
            problem: CatalogProblem::Kind { found, builder },
            ..
        }) = Fitting::from_catalog(nose)
        else {
            panic!("a nose cone isn't a fitting");
        };
        assert_eq!((found, builder), ("nose cone", "Fitting::from_catalog"));
    }
}
