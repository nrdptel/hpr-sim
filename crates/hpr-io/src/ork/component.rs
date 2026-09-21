//! The spine of a design: its stages and the body components stacked inside them.
//!
//! A `.ork` design is a tree. Its trunk is the **spine**: the stages, and inside each of them the
//! nose cones, body tubes and transitions that stack end to end along the axis. Everything else —
//! the tubes and rings inside the body, the fins and lugs on it, the recovery gear — hangs off that
//! trunk, and is read by [`super::attached`].
//!
//! What this module does is turn the spine into [`hpr_design`] types: a [`Rocket`] of [`Stage`]s of
//! [`Component`]s, each of them carrying whatever [`super::attached`] read inside and on it. It
//! resolves nothing itself. Where OpenRocket wrote `auto`, the component carries an
//! [`AutoDimension`] and [`Rocket::layout`] works the radius out from the neighbours, which is the
//! one place that rule lives. The one exception is a radius that rule cannot reach — a chain of
//! automatic radii with no fixed radius anywhere along it — which takes OpenRocket's default,
//! [`OPENROCKET_DEFAULT_RADIUS_M`], because the design holds no other number for it.
//!
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md

use hpr_design::parts::{BodyTube, NoseCone, Shoulder, Transition};
use hpr_design::shapes::NoseShape;
use hpr_design::solids::Wall;
use hpr_design::tree::{
    AutoDimension, Component, Overrides, Part, ReferenceDiameter, Rocket, Stage,
};
use hpr_design::{Material, MotorMount};

use super::attached::{self, finish};
use super::document::{Document, Element};
use super::motors::{self, MountRead};
use super::recovery::{self, DeviceRead, SeparationRead};
use super::value::Values;
use super::warning::{Imported, Warning, WarningKind};

/// The body tags this milestone reads. Anything else in a `<subcomponents>` is counted and left.
const BODY_TAGS: [&str; 3] = ["nosecone", "bodytube", "transition"];

/// A filled tube whose automatic radius cached a number, read as solid to that number.
const FILLED_TO_CACHE: &str = "a filled tube whose radius is automatic; it was read as solid to \
                               the radius OpenRocket last worked out, which may be stale";

/// A filled tube whose automatic radius cached nothing, so it has no radius to be solid to.
const FILLED_TO_NOTHING: &str = "a filled tube whose radius is automatic and nothing cached; it \
                                 carries no mass, because there is no radius to fill until the \
                                 layout resolves one";

/// What a body component was read from: where it is in the file, and for a body tube the wall as
/// the file wrote it, before any radius was known to judge it against.
struct BodyRead {
    at: String,
    wall: Option<Wall>,
}

/// The radius OpenRocket gives an automatic body radius that has no fixed radius anywhere along its
/// chain to take, in metres: its **default radius**, 25 mm.
///
/// OpenRocket's maintainers write that such a radius is "the default radius"
/// ([openrocket#1988](https://github.com/openrocket/openrocket/issues/1988#issuecomment-1397654629))
/// and that a tube left with nothing to take "reverts to default diameter"
/// ([#1992](https://github.com/openrocket/openrocket/issues/1992)); a user guesses that default at
/// "1.969 in" of diameter ([#871](https://github.com/openrocket/openrocket/issues/871)), which is
/// 50.0 mm. No document states the number, so it was measured: OpenRocket 24.12, run on fifteen
/// small designs by `validation/oracles/openrocket/automatic_radius.py`, settles every such tube,
/// lone nose cone and lone transition at 0.025 m and ignores any number cached after `auto`
/// (`validation/fixtures/ork/openrocket-automatic-radius.json`, [ADR-054][adr-054]).
///
/// hpr departs from OpenRocket in one place, on purpose: where a nose cone's base or a transition's
/// end looks at another automatic radius, OpenRocket 24.12 settles on −1 m, which no geometry can
/// take. hpr gives it this default too, so the chain is one radius end to end.
///
/// [adr-054]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-054-an-automatic-radius-with-nothing-to-take-is-openrockets-default-and-a-rocket-with-no-stage-or-component-holds-no-design-2026-09-20
pub const OPENROCKET_DEFAULT_RADIUS_M: f64 = 0.025;

/// Reads a design document into a [`Rocket`]: its stages, the body components stacked in them, and
/// the parts on and inside each of those.
///
/// Never fails: a tag it cannot read is left out with a [`Warning`], so a design written by another
/// program still opens. The result is a [`Rocket`] whose body components are in file order, forward
/// to aft, with automatic dimensions **marked rather than filled in** — [`Rocket::layout`] resolves
/// them, and is where a part's mass and station come from.
///
/// A body tube or inner tube that holds a motor is marked as a motor mount, but the motors
/// themselves are not read here, and [`Rocket::configurations`] is left empty:
/// [`super::design`] reads them. A `<motormount>` it cannot read still warns here.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
/// <openrocket version="1.10" creator="OpenRocket 24.12">
///   <rocket><name>Sounder</name><subcomponents><stage><name>Sustainer</name><id>s</id>
///     <subcomponents>
///       <nosecone><name>Nose</name><id>nose</id><finish>smooth</finish>
///         <material type="bulk" density="680.0">Cardboard</material>
///         <length>0.3</length><thickness>0.002</thickness>
///         <shape>ogive</shape><shapeparameter>1.0</shapeparameter>
///         <aftradius>0.05</aftradius></nosecone>
///       <bodytube><name>Tube</name><id>tube</id>
///         <material type="bulk" density="680.0">Cardboard</material>
///         <length>0.6</length><thickness>0.002</thickness><radius>0.05</radius>
///         <subcomponents>
///           <centeringring><name>Ring</name><id>ring</id>
///             <material type="bulk" density="680.0">Plywood</material>
///             <axialoffset method="bottom">0.0</axialoffset><length>0.005</length>
///             <outerradius>auto</outerradius><innerradius>0.019</innerradius></centeringring>
///         </subcomponents></bodytube>
///     </subcomponents></stage></subcomponents></rocket>
/// </openrocket>"#;
///
/// let read = hpr_io::ork::read(xml)?;
/// let design = hpr_io::ork::rocket(&read.value.document);
///
/// // Anything the reader could not take at face value travels with the result. Nothing here did.
/// assert!(design.warnings.is_empty(), "{:?}", design.warnings);
///
/// // The ring's outer radius says `auto`, so the design carries the dimension, not a number...
/// use hpr_design::AutoDimension;
/// let ring = &design.value.stages[0].components[1].children[0];
/// assert!(ring.auto.contains(&AutoDimension::OuterRadius));
///
/// // ...and the layout works it out: the bore of the tube the ring sits in, 0.05 - 0.002.
/// let layout = design.value.layout()?;
/// let (_, placed) = layout.find("ring").expect("the ring");
/// let hpr_design::Part::CenteringRing(ring) = &placed.part else { panic!("a ring") };
/// assert!((ring.outer_radius_m - 0.048).abs() < 1e-12);
/// assert!(placed.own.mass_kg > 0.0);
/// # Ok(())
/// # }
/// ```
pub fn rocket(document: &Document) -> Imported<Rocket> {
    walk(document).0
}

/// [`rocket`], and every motor mount, recovery device and stage separation it read, for
/// [`super::motors::read`] and [`super::recovery::read`].
pub(super) fn walk(document: &Document) -> (Imported<Rocket>, Walked) {
    let mut warnings = Vec::new();
    let mut rocket = Rocket {
        name: String::new(),
        stages: Vec::new(),
        reference_diameter: ReferenceDiameter::default(),
        configurations: Vec::new(),
    };
    let Some(element) = document.root.child("rocket") else {
        warnings.push(Warning::new(
            "openrocket",
            WarningKind::Skipped,
            "no `rocket` element, so the document holds no design".to_owned(),
        ));
        return (
            Imported {
                value: rocket,
                warnings,
            },
            Walked::default(),
        );
    };
    let at = "openrocket/rocket";
    rocket.name = Values::new(element, at, &mut warnings)
        .word(&["name"])
        .unwrap_or_default();
    rocket.reference_diameter = reference_diameter(element, at, &mut warnings);
    if subcomponents(element).next().is_none() {
        // A document can carry a rocket's name and stored results and nothing to build: Debrief's
        // synthesized demonstration file does. There is no design in it to lay out.
        warnings.push(Warning::new(
            at,
            WarningKind::Unusual,
            "the `rocket` holds no stage or component of any kind, so the document holds no design"
                .to_owned(),
        ));
    }

    let mut ids = Ids::default();
    let mut skipped: Vec<String> = Vec::new();
    let mut reads: Vec<Vec<BodyRead>> = Vec::new();
    for (index, stage_element) in subcomponents(element).enumerate() {
        if stage_element.name != "stage" {
            skipped.push(stage_element.name.clone());
            continue;
        }
        let mut read = Vec::new();
        rocket.stages.push(stage(
            stage_element,
            index,
            &mut ids,
            &mut read,
            &mut skipped,
            &mut warnings,
        ));
        reads.push(read);
    }
    if let Some(note) = tally(&skipped) {
        warnings.push(Warning::new(
            at,
            WarningKind::Skipped,
            format!(
                "{note} were left out: a pod or a parallel stage carries a spine of its own, \
                 which is a later milestone's work"
            ),
        ));
    }
    default_radii(&mut rocket, &reads, &mut warnings);
    (
        Imported {
            value: rocket,
            warnings,
        },
        Walked {
            mounts: ids.mounts,
            devices: ids.devices,
            separations: ids.separations,
            read: ids.read,
        },
    )
}

/// Gives every automatic body radius that [`Rocket::unresolvable_body_radii`] lists
/// [`OPENROCKET_DEFAULT_RADIUS_M`], as a fixed radius, with a warning at its tag naming the radius.
///
/// A body tube filled that way has its wall judged again, now there is a radius to judge it
/// against, by the rule a stated radius gets: `filled`, or a wall at least as thick as the radius,
/// is solid. The warnings that said the radius was not known yet are withdrawn, because it is.
fn default_radii(rocket: &mut Rocket, reads: &[Vec<BodyRead>], warnings: &mut Vec<Warning>) {
    let radius_m = OPENROCKET_DEFAULT_RADIUS_M;
    for filled in rocket.fill_unresolvable_body_radii(radius_m) {
        let Some(read) = reads
            .get(filled.stage)
            .and_then(|stage| stage.get(filled.component))
        else {
            continue;
        };
        let component = &mut rocket.stages[filled.stage].components[filled.component];
        if let Part::BodyTube(tube) = &mut component.part {
            match read.wall {
                Some(Wall::Filled {}) => tube.thickness_m = radius_m,
                Some(Wall::Shell { thickness_m }) if thickness_m >= radius_m => {
                    tube.thickness_m = radius_m;
                }
                _ => {}
            }
            warnings.retain(|warning| {
                warning.at != read.at
                    || (warning.message != FILLED_TO_CACHE && warning.message != FILLED_TO_NOTHING)
            });
        }
        warnings.push(Warning::new(
            read.at.clone(),
            WarningKind::Unusual,
            format!(
                "an automatic radius with no fixed radius anywhere along its chain to take, its \
                 `{}`; it was given {:.0} mm, OpenRocket's default radius for a tube with nothing \
                 to take",
                radius_tag(filled.dimension),
                radius_m * 1e3
            ),
        ));
    }
}

/// The tag a body component's radius is written under: a nose cone's base is its `aftradius`.
fn radius_tag(dimension: AutoDimension) -> &'static str {
    match dimension {
        AutoDimension::OuterRadius => "radius",
        AutoDimension::ForeRadius => "foreradius",
        AutoDimension::BaseRadius | AutoDimension::AftRadius => "aftradius",
        other => other.name(),
    }
}

/// Reads one `<stage>`, and everything stacked inside it. What each body component was read from
/// goes into `reads`, in the stage's order, for [`default_radii`].
fn stage(
    element: &Element,
    index: usize,
    ids: &mut Ids,
    reads: &mut Vec<BodyRead>,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Stage {
    let at = format!("openrocket/rocket/stage[{index}]");
    let mut values = Values::new(element, &at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let (overrides, _) = overrides(&mut values);
    let id = ids.take(&mut Values::new(element, &at, warnings), "stage");
    if let Some(separation) = recovery::separation(element, &at, warnings) {
        ids.separations.push((id.clone(), separation));
    }
    ids.read.insert(at.clone());
    let mut components = Vec::new();
    // The index is part of the path so that a warning can be traced back to one part of the 188
    // body tubes in the reference library, the way a stage's already could (issue #132).
    for (index, child) in subcomponents(element).enumerate() {
        if BODY_TAGS.contains(&child.name.as_str()) {
            let at = format!("{at}/{}[{index}]", child.name);
            let (component, wall) = body(child, &at, ids, skipped, warnings);
            reads.push(BodyRead { at, wall });
            components.push(component);
        } else {
            skipped.push(child.name.clone());
        }
    }
    Stage {
        id,
        name,
        components,
        overrides,
    }
}

/// Reads one body component, and everything on and inside it; for a body tube, also the wall as
/// the file wrote it.
fn body(
    element: &Element,
    at: &str,
    ids: &mut Ids,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> (Component, Option<Wall>) {
    let mut auto = Vec::new();
    let mut values = Values::new(element, at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let (overrides, overrides_include_children) = overrides(&mut values);
    let finish = finish(&mut values);
    let (part, wall) = match element.name.as_str() {
        "nosecone" => (nose_cone(element, at, &mut auto, warnings), None),
        "bodytube" => {
            let (part, wall) = body_tube(element, at, &mut auto, warnings);
            (part, Some(wall))
        }
        _ => (transition(element, at, &mut auto, warnings), None),
    };
    let children = attached::children(element, &part, at, ids, skipped, warnings);
    let mount = match part {
        Part::BodyTube(_) => motors::mount(element, at, warnings),
        _ => None,
    };
    let id = ids.take(
        &mut Values::new(element, at, warnings),
        &element.name.clone(),
    );
    let motor_mount = mount.map(|mount| {
        let spec = MotorMount {
            overhang_m: mount.overhang_m,
        };
        ids.mounts.push((id.clone(), mount));
        spec
    });
    ids.read.insert(at.to_owned());
    let component = Component {
        id,
        name,
        part,
        position: None,
        auto,
        motor_mount,
        finish,
        overrides,
        overrides_include_children,
        children,
    };
    (component, wall)
}

fn nose_cone(
    element: &Element,
    at: &str,
    auto: &mut Vec<AutoDimension>,
    warnings: &mut Vec<Warning>,
) -> Part {
    let mut values = Values::new(element, at, warnings);
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (stated_m, base_radius_m) =
        stated_radius(&mut values, &["aftradius"], AutoDimension::BaseRadius, auto);
    let wall = wall(&mut values, stated_m);
    let shape = shape(&mut values);
    let shoulder = shoulder(&mut values, "aft", AutoDimension::ShoulderRadius, auto);
    if values.flag(&["isflipped"]) == Some(true) {
        values.warn_at(
            WarningKind::Dropped,
            "a flipped nose cone is a tail cone; it was read pointing forward",
        );
    }
    Part::NoseCone(NoseCone {
        shape,
        length_m,
        base_radius_m,
        wall,
        shoulder,
        material: material(&mut values, &["material"], "bulk"),
    })
}

fn body_tube(
    element: &Element,
    at: &str,
    auto: &mut Vec<AutoDimension>,
    warnings: &mut Vec<Warning>,
) -> (Part, Wall) {
    let mut values = Values::new(element, at, warnings);
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (stated_m, outer_radius_m) =
        stated_radius(&mut values, &["radius"], AutoDimension::OuterRadius, auto);
    // A body tube is a wall, not a solid of revolution, so `filled` has to be said as a wall as
    // thick as the tube. With an automatic radius there is no such number yet: the tube is read as
    // the wall it caches, and says so.
    let read = wall(&mut values, stated_m);
    let thickness_m = match (read, stated_m) {
        (Wall::Filled {}, Some(radius_m)) => radius_m,
        (Wall::Filled {}, None) if outer_radius_m > 0.0 => {
            values.warn_at(WarningKind::Dropped, FILLED_TO_CACHE);
            outer_radius_m
        }
        // Nothing cached either, so there is no number that means "solid" until the layout
        // resolves one. The tube still has to stand in the stack, so it stands as a wall of
        // nothing — and that is said as loudly as a part left out, because a solid tube read as an
        // empty one is a mass quietly missing rather than a design that fails (issue #130).
        (Wall::Filled {}, None) => {
            values.warn_at(WarningKind::Skipped, FILLED_TO_NOTHING);
            0.0
        }
        (Wall::Shell { thickness_m }, _) => thickness_m,
    };
    let part = Part::BodyTube(BodyTube {
        length_m,
        outer_radius_m,
        thickness_m,
        material: material(&mut values, &["material"], "bulk"),
    });
    (part, read)
}

fn transition(
    element: &Element,
    at: &str,
    auto: &mut Vec<AutoDimension>,
    warnings: &mut Vec<Warning>,
) -> Part {
    let mut values = Values::new(element, at, warnings);
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (stated_fore_m, fore_radius_m) = stated_radius(
        &mut values,
        &["foreradius"],
        AutoDimension::ForeRadius,
        auto,
    );
    let (stated_aft_m, aft_radius_m) =
        stated_radius(&mut values, &["aftradius"], AutoDimension::AftRadius, auto);
    // Either end may be automatic; a wall is only too thick when it is thicker than both ends that
    // are known.
    let known = match (stated_fore_m, stated_aft_m) {
        (Some(fore_m), Some(aft_m)) => Some(fore_m.max(aft_m)),
        _ => None,
    };
    let wall = wall(&mut values, known);
    let shape = shape(&mut values);
    // Only 1 of the 21 transitions in the reference corpus states `shapeclipped`, so the default
    // matters and no document this project may read states it. Clipped is taken as the default
    // because it is the shape a transition between two radii needs to reach both of them; the
    // OpenRocket oracle (M2.2) is what will settle it.
    let clipped = values.flag(&["shapeclipped"]).unwrap_or(true);
    Part::Transition(Transition {
        shape,
        clipped,
        length_m,
        fore_radius_m,
        aft_radius_m,
        wall,
        fore_shoulder: shoulder(&mut values, "fore", AutoDimension::ForeShoulderRadius, auto),
        aft_shoulder: shoulder(&mut values, "aft", AutoDimension::AftShoulderRadius, auto),
        material: material(&mut values, &["material"], "bulk"),
    })
}

/// The radius, and what it is worth to a reader before the layout resolves it: `None` when the file
/// says `auto`, whether or not OpenRocket cached a number with it, because the cached number is the
/// neighbour it last had and may be stale.
pub(super) fn stated_radius(
    values: &mut Values<'_>,
    names: &[&str],
    dimension: AutoDimension,
    auto: &mut Vec<AutoDimension>,
) -> (Option<f64>, f64) {
    let Some(read) = values.dimension(names) else {
        // A tag that is there but unreadable has already said so. A tag that is not there at all
        // is read as zero, which for a radius is a part with no width: a transition with no
        // `foreradius` lays out as a cone growing from a point, and says nothing unless it says
        // this (issue #131).
        if values.element(names).is_none() {
            let name = names.first().copied().unwrap_or("dimension").to_owned();
            values.warn_at(
                WarningKind::Dropped,
                format!("no `{name}`, so it was read as zero"),
            );
        }
        return (None, 0.0);
    };
    if read.is_automatic() {
        auto.push(dimension);
        return (None, read.value().unwrap_or_default());
    }
    let value = read.value().unwrap_or_default();
    (Some(value), value)
}

/// `<thickness>filled</thickness>` is a solid part; anything else is a wall of that thickness.
///
/// `outer_radius_m` is the radius the wall sits in, and is `None` when that radius is automatic and
/// so is not known yet. A wall at least as thick as a known radius is the part filled. It must not
/// be judged against an unknown radius: the stated thickness would be thrown away whenever the
/// radius was automatic, which is half of [Loft lesson L61][lessons] — a stated wall dropped
/// because the outer radius was `auto`.
///
/// Two readings are OpenRocket 24.12's, measured on probe designs by
/// `validation/oracles/openrocket/conventions.py` ([ADR-061][adr-061]):
///
/// - a wall of no thickness is a surface with no wall: the part keeps its shape and weighs
///   nothing, where a filled part is written `filled`;
/// - a part that writes no thickness at all has OpenRocket's 2 mm wall
///   ([`DEFAULT_WALL_M`]), whatever its radius.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
/// [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
fn wall(values: &mut Values<'_>, outer_radius_m: Option<f64>) -> Wall {
    if values.word(&["thickness"]).as_deref() == Some("filled") {
        return Wall::Filled {};
    }
    match (values.number(&["thickness"]), outer_radius_m) {
        (Some(thickness_m), Some(radius_m)) if thickness_m >= radius_m => Wall::Filled {},
        (Some(thickness_m), _) if thickness_m > 0.0 => Wall::Shell { thickness_m },
        (Some(thickness_m), _) if thickness_m == 0.0 => Wall::Shell { thickness_m: 0.0 },
        (Some(thickness_m), _) => {
            values.warn_at(
                WarningKind::Dropped,
                format!("a wall {thickness_m} m thick is no wall; it was read as none"),
            );
            Wall::Shell { thickness_m: 0.0 }
        }
        (None, _) => Wall::Shell {
            thickness_m: DEFAULT_WALL_M,
        },
    }
}

/// The wall OpenRocket 24.12 gives a nose cone, transition or body tube that writes no thickness,
/// m: measured on a 50 mm and a 30 mm probe, each the mass of a 2 mm wall ([ADR-061][adr-061]).
///
/// [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
const DEFAULT_WALL_M: f64 = 0.002;

/// A shoulder at one end, if the file gives it a length. A zero-length shoulder is no shoulder.
fn shoulder(
    values: &mut Values<'_>,
    end: &str,
    dimension: AutoDimension,
    auto: &mut Vec<AutoDimension>,
) -> Option<Shoulder> {
    let length_m = values.number(&[&format!("{end}shoulderlength")])?;
    if length_m <= 0.0 {
        return None;
    }
    let (known_m, outer_radius_m) =
        stated_radius(values, &[&format!("{end}shoulderradius")], dimension, auto);
    // A shoulder with no wall, or none written, weighs nothing in OpenRocket 24.12, capped or not
    // and whether or not the part it hangs from is filled: measured on probe designs (ADR-061).
    // So it is read as a tube of no wall, which weighs nothing.
    // The stated wall is clamped to the radius it sits in, and only when that radius is known:
    // clamping against an automatic one that has not resolved yet would throw the wall away, which
    // is half of Loft lesson L61 and what issue #130 found still open on a shoulder.
    let thickness_m = match values.number(&[&format!("{end}shoulderthickness")]) {
        Some(stated_m) if stated_m > 0.0 => {
            known_m.map_or(stated_m, |radius_m| stated_m.min(radius_m))
        }
        _ => 0.0,
    };
    Some(Shoulder {
        length_m,
        outer_radius_m,
        thickness_m,
        // A cap is as thick as the shoulder's wall, so a shoulder of no wall has none; and a solid
        // shoulder has no bore to close, so a cap on one is nothing either: reading it as a cap
        // asks `hpr-design` for a disc inside a tube that isn't hollow.
        capped: thickness_m > 0.0
            && known_m.is_none_or(|radius_m| thickness_m < radius_m)
            && values
                .flag(&[&format!("{end}shouldercapped")])
                .unwrap_or_default(),
    })
}

/// The profile shape and its parameter.
///
/// OpenRocket's shape parameter is `κ`, defined in [Niskanen's technical documentation][niskanen],
/// appendix A. For an ogive it is the ratio of a tangent ogive's radius of curvature to this one's,
/// `κ = ρ_t/ρ` (equation A.3), so `κ = 1` is a tangent ogive and `κ = 0` an infinite radius, which
/// is a cone. [`NoseShape::Ogive`] states the same shape the other way up, as `ρ/ρ_t`, so the two
/// are reciprocals. The power series (A.8) and the parabolic series (A.7) use the same `κ` this
/// crate's [`NoseShape::PowerSeries`] and [`NoseShape::ParabolicSeries`] do, and the Haack series
/// (A.9) the same `C`.
///
/// [niskanen]: https://github.com/nrdptel/hpr-sim/blob/main/docs/format/ork.md
fn shape(values: &mut Values<'_>) -> NoseShape {
    let name = values.word(&["shape"]).unwrap_or_default();
    let parameter = values.number(&["shapeparameter"]);
    match (name.as_str(), parameter) {
        ("conical", _) => NoseShape::Conical {},
        ("ellipsoid", _) => NoseShape::Elliptical {},
        // κ = 0 is the cone, and the reciprocal below would divide by it.
        ("ogive", Some(kappa)) if kappa <= 0.0 => NoseShape::Conical {},
        ("ogive", kappa) => NoseShape::Ogive {
            radius_ratio: 1.0 / kappa.unwrap_or(1.0),
        },
        ("power", Some(exponent)) => NoseShape::PowerSeries { exponent },
        ("parabolic", Some(parameter)) => NoseShape::ParabolicSeries { parameter },
        ("haack", parameter) => NoseShape::Haack {
            parameter: parameter.unwrap_or_default(),
        },
        // A power or parabolic series is the shape its parameter says it is, and there is no
        // sourced default for either — OpenRocket's own is in its source, which this project does
        // not read. So it is read as a cone and the message says why, rather than blaming the
        // shape's name (issue #131). No nose in the reference corpus omits it.
        (shape @ ("power" | "parabolic"), None) => {
            let shape = shape.to_owned();
            values.warn_at(
                WarningKind::Unusual,
                format!(
                    "a `{shape}` nose states no shape parameter, and this reader has no sourced \
                     default for one; it was read as a cone"
                ),
            );
            NoseShape::Conical {}
        }
        (other, _) => {
            let other = other.to_owned();
            values.warn_at(
                WarningKind::Unusual,
                format!("`{other}` is not a shape this reader knows; it was read as a cone"),
            );
            NoseShape::Conical {}
        }
    }
}

/// The material a part is made of. A `.ork` stores the density with the name, so nothing is
/// looked up.
///
/// `want` is the kind of density the part needs — `bulk` for anything solid, `surface` for a
/// canopy or a streamer, `line` for a shroud line or a shock cord. The file declares a kind of its
/// own in the `type` attribute, and the number is in that kind's units, so a density declared as
/// one kind cannot be converted into another: the part is built with the kind it needs and the
/// disagreement is reported. No material in the reference corpus is declared as a kind its part
/// does not want.
///
/// A part that names no material is made of the one OpenRocket 24.12 gives it, by kind
/// ([`unnamed_material`]); a rail button's is its own, so it calls [`material_or`].
pub(super) fn material(values: &mut Values<'_>, names: &[&str], want: &str) -> Material {
    material_or(values, names, want, unnamed_material(want))
}

/// The material OpenRocket 24.12 gives a part that names none, by the kind of density it needs:
/// cardboard, 680 kg/m³, for a solid part; ripstop nylon, 0.067 kg/m², for a canopy or a streamer;
/// and a 2 mm elastic cord, 0.0018 kg/m, for shroud lines and a shock cord. Each was read from a
/// probe part of each kind through OpenRocket's public `getMaterial` and `getLineMaterial`
/// (`validation/oracles/openrocket/conventions.py`, [ADR-061][adr-061]).
///
/// [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
pub(super) fn unnamed_material(want: &str) -> (&'static str, f64) {
    match want {
        "surface" => ("Ripstop nylon", 0.067),
        "line" => ("Elastic cord (round 2 mm, 1/16 in)", 0.0018),
        _ => ("Cardboard", 680.0),
    }
}

/// The material OpenRocket 24.12 gives a rail button that names none: Delrin, 1,420 kg/m³, measured
/// as [`unnamed_material`]'s are.
pub(super) const UNNAMED_RAIL_BUTTON: (&str, f64) = ("Delrin", 1420.0);

/// [`material`], with the material a part that names none is made of.
pub(super) fn material_or(
    values: &mut Values<'_>,
    names: &[&str],
    want: &str,
    (unnamed, unnamed_kg): (&str, f64),
) -> Material {
    let named = |name: &str, kg: f64| match want {
        "surface" => Material::surface(name, kg),
        "line" => Material::line(name, kg),
        _ => Material::bulk(name, kg),
    };
    let Some(element) = values.element(names) else {
        return named(unnamed, unnamed_kg);
    };
    let name = element.text().trim().to_owned();
    if let Some(declared) = element.attribute("type")
        && declared != want
    {
        let declared = declared.to_owned();
        values.warn_at(
            WarningKind::Dropped,
            format!(
                "the material `{name}` is declared `{declared}` where a `{want}` density is \
                 needed; its number was taken as a `{want}` one"
            ),
        );
    }
    let density = element
        .attribute("density")
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|density| density.is_finite() && *density >= 0.0);
    match density {
        Some(kg) => named(&name, kg),
        None => {
            values.warn_at(
                WarningKind::Dropped,
                format!("the material `{name}` states no density; it weighs nothing"),
            );
            named(&name, 0.0)
        }
    }
}

/// The overrides, as `hpr-design` states them, and whether they cover the parts inside this one.
///
/// A `.ork` says "covers the children too" once per quantity and `hpr-design` says it once for the
/// component. A part that overrides only one quantity takes that quantity's flag; OpenRocket 24.12
/// applies a centre-of-gravity override alone to the whole assembly when its flag says so
/// (measured, ADR-061). A part that overrides both, with flags that disagree, cannot be said in
/// `hpr-design`: the mass flag decides, because mass is the quantity the flag is written for (95 of
/// the 104 in the reference corpus are `overridesubcomponentsmass`), and the disagreement is
/// reported. The drag override is read by [`super::value::Values::overrides`] but waits on the
/// milestone that charges drag to a part.
pub(super) fn overrides(values: &mut Values<'_>) -> (Overrides, bool) {
    let read = values.overrides();
    let mass_flag = read.subcomponents_mass.unwrap_or_default();
    let covers_children = match (read.mass_kg, read.cg_m) {
        (None, Some(_)) => read.subcomponents_cg.unwrap_or_default(),
        _ => mass_flag,
    };
    if read.mass_kg.is_some()
        && read.cg_m.is_some()
        && read.subcomponents_cg.is_some_and(|cg| cg != mass_flag)
    {
        values.warn_at(
            WarningKind::Dropped,
            format!(
                "the mass override covers the parts inside this one ({mass_flag}) and \
                 the centre-of-gravity override does not agree; hpr states it once, so \
                 the mass flag was taken"
            ),
        );
    }
    (
        Overrides {
            mass_kg: read.mass_kg,
            cg_aft_m: read.cg_m,
            cg_xy_m: None,
            inertia: None,
        },
        covers_children,
    )
}

/// How the reference diameter is chosen. Every design in the reference corpus says `maximum`,
/// which is OpenRocket's default; anything else is left at that default and reported.
fn reference_diameter(
    element: &Element,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> ReferenceDiameter {
    let word = Values::new(element, at, warnings).word(&["referencetype"]);
    match word.as_deref() {
        None | Some("maximum") => ReferenceDiameter::Maximum {},
        Some(other) => {
            warnings.push(Warning::new(
                at,
                WarningKind::Dropped,
                format!(
                    "a reference diameter chosen by `{other}` is not read yet; the widest body \
                     component was used"
                ),
            ));
            ReferenceDiameter::Maximum {}
        }
    }
}

/// The children of an element's `<subcomponents>`, in file order.
pub(super) fn subcomponents(element: &Element) -> impl Iterator<Item = &Element> {
    element
        .child("subcomponents")
        .into_iter()
        .flat_map(Element::elements)
}

/// Unique ids: a `.ork` gives most components one, but not all of them, and `hpr-design` needs
/// every id to be distinct or it refuses the whole design.
///
/// So every id handed out is remembered, whether it came from the file or was invented here. A
/// file whose own ids repeat, or whose `<id>bodytube-4</id>` collides with the name invented for a
/// component that has none, gets a number added and a warning rather than a design that will not
/// open (issue #132).
#[derive(Debug, Default)]
pub(super) struct Ids {
    used: usize,
    taken: std::collections::BTreeSet<String>,
    /// Every motor mount read, with the id its component was given, in file order: the walk
    /// records them here because this is what it carries everywhere, and [`super::motors::read`]
    /// needs the ids.
    pub(super) mounts: Vec<(String, MountRead)>,
    /// Every parachute and streamer read, with its component's id, in file order.
    pub(super) devices: Vec<(String, DeviceRead)>,
    /// Every stage that states when it separates, with the stage's id, in file order.
    pub(super) separations: Vec<(String, SeparationRead)>,
    /// The path of every stage and component read, for [`super::extensions::read`] to keep the
    /// rest.
    pub(super) read: std::collections::BTreeSet<String>,
}

/// What the walk read besides the rocket, each with the id it gave its component.
#[derive(Debug, Default)]
pub(super) struct Walked {
    /// The motor mounts.
    pub mounts: Vec<(String, MountRead)>,
    /// The parachutes and streamers.
    pub devices: Vec<(String, DeviceRead)>,
    /// The stages' separations.
    pub separations: Vec<(String, SeparationRead)>,
    /// The path of every stage and component read.
    pub read: std::collections::BTreeSet<String>,
}

impl Ids {
    pub(super) fn take(&mut self, values: &mut Values<'_>, kind: &str) -> String {
        self.used += 1;
        let wanted = values
            .word(&["id"])
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| format!("{kind}-{}", self.used));
        if self.taken.insert(wanted.clone()) {
            return wanted;
        }
        let mut again = 2usize;
        let id = loop {
            let candidate = format!("{wanted}-{again}");
            if self.taken.insert(candidate.clone()) {
                break candidate;
            }
            again += 1;
        };
        values.warn_at(
            WarningKind::Unusual,
            format!(
                "`{wanted}` is already the id of another component; this one was called `{id}`"
            ),
        );
        id
    }
}

/// `["bodytube", "finset", "bodytube"]` as `2 bodytube, 1 finset`.
fn tally(names: &[String]) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for name in names {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    Some(
        counts
            .into_iter()
            .map(|(name, count)| format!("{count} `{name}`"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}
