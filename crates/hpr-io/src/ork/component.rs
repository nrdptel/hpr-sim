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

use std::collections::BTreeMap;

use hpr_design::Material;
use hpr_design::parts::{BodyTube, NoseCone, Shoulder, Transition};
use hpr_design::shapes::NoseShape;
use hpr_design::solids::Wall;
use hpr_design::tree::{
    AutoDimension, Component, Overrides, Part, ReferenceDiameter, Rocket, Stage,
};

use super::attached::{self, finish};
use super::document::{Document, Element};
use super::value::Values;
use super::warning::{Imported, Warning, WarningKind};

/// The body tags this milestone reads. Anything else in a `<subcomponents>` is counted and left.
const BODY_TAGS: [&str; 3] = ["nosecone", "bodytube", "transition"];

/// The radius OpenRocket gives an automatic body radius that has no fixed radius anywhere along its
/// chain to take, in metres: its **default radius**, 25 mm.
///
/// OpenRocket's maintainers write that such a radius is "the default radius"
/// ([openrocket#1988](https://github.com/openrocket/openrocket/issues/1988#issuecomment-1397654629))
/// and that a tube left with nothing to take "reverts to default diameter"
/// ([#1992](https://github.com/openrocket/openrocket/issues/1992)); a user reports that default as
/// "1.969 in" of diameter ([#871](https://github.com/openrocket/openrocket/issues/871)), which is
/// 50.0 mm. No document states the number exactly, so it was measured: OpenRocket 24.12, run on
/// nine small designs by `validation/oracles/openrocket/automatic_radius.py`, gives every such
/// tube, lone nose cone and lone transition 0.025 m and ignores any number cached after `auto`
/// (`validation/fixtures/ork/openrocket-automatic-radius.json`, [ADR-054][adr-054]).
///
/// hpr departs from OpenRocket in one place, on purpose: where a nose cone's base or a transition's
/// forward radius looks at an automatic tube, OpenRocket 24.12 resolves it to −1 m, which no
/// geometry can take. hpr gives it this default too, so the chain is one radius end to end.
///
/// [adr-054]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-054-an-automatic-radius-with-nothing-to-take-is-openrockets-default-and-a-rocket-with-no-stage-holds-no-design-2026-09-20
pub const OPENROCKET_DEFAULT_RADIUS_M: f64 = 0.025;

/// Reads a design document into a [`Rocket`]: its stages, the body components stacked in them, and
/// the parts on and inside each of those.
///
/// Never fails: a tag it cannot read is left out with a [`Warning`], so a design written by another
/// program still opens. The result is a [`Rocket`] whose body components are in file order, forward
/// to aft, with automatic dimensions **marked rather than filled in** — [`Rocket::layout`] resolves
/// them, and is where a part's mass and station come from.
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
            "no `rocket` element, so the design has no components".to_owned(),
        ));
        return Imported {
            value: rocket,
            warnings,
        };
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
    let mut paths = BTreeMap::new();
    for (index, stage_element) in subcomponents(element).enumerate() {
        if stage_element.name != "stage" {
            skipped.push(stage_element.name.clone());
            continue;
        }
        rocket.stages.push(stage(
            stage_element,
            index,
            &mut ids,
            &mut paths,
            &mut skipped,
            &mut warnings,
        ));
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
    default_radii(&mut rocket, &paths, &mut warnings);
    Imported {
        value: rocket,
        warnings,
    }
}

/// Gives every automatic body radius that [`Rocket::unresolvable_body_radii`] lists
/// [`OPENROCKET_DEFAULT_RADIUS_M`], as a fixed radius, with a warning at its tag.
///
/// Every radius on such a chain is listed, so the whole chain takes the one radius: filling only
/// the first and letting the neighbour rule carry it along would give the same answer.
fn default_radii(
    rocket: &mut Rocket,
    paths: &BTreeMap<String, String>,
    warnings: &mut Vec<Warning>,
) {
    let radius_m = OPENROCKET_DEFAULT_RADIUS_M;
    for (id, dimension) in rocket.unresolvable_body_radii() {
        let Some(component) = rocket
            .stages
            .iter_mut()
            .flat_map(|stage| stage.components.iter_mut())
            .find(|component| component.id == id)
        else {
            continue;
        };
        match (&mut component.part, dimension) {
            (Part::NoseCone(p), AutoDimension::BaseRadius) => p.base_radius_m = radius_m,
            (Part::BodyTube(p), AutoDimension::OuterRadius) => p.outer_radius_m = radius_m,
            (Part::Transition(p), AutoDimension::ForeRadius) => p.fore_radius_m = radius_m,
            (Part::Transition(p), AutoDimension::AftRadius) => p.aft_radius_m = radius_m,
            // `unresolvable_body_radii` lists only these four pairings.
            _ => continue,
        }
        component.auto.retain(|auto| *auto != dimension);
        warnings.push(Warning::new(
            paths.get(&id).map_or("openrocket/rocket", String::as_str),
            WarningKind::Unusual,
            "an automatic radius with no fixed radius anywhere along its chain to take; it was \
             given OpenRocket's default radius, 25 mm, as OpenRocket does",
        ));
    }
}

/// Reads one `<stage>`, and everything stacked inside it. Each body component's path in the file
/// goes into `paths` under its id, for a warning raised about it later.
fn stage(
    element: &Element,
    index: usize,
    ids: &mut Ids,
    paths: &mut BTreeMap<String, String>,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Stage {
    let at = format!("openrocket/rocket/stage[{index}]");
    let mut values = Values::new(element, &at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let (overrides, _) = overrides(&mut values);
    let id = ids.take(&mut Values::new(element, &at, warnings), "stage");
    let mut components = Vec::new();
    // The index is part of the path so that a warning can be traced back to one part of the 188
    // body tubes in the reference library, the way a stage's already could (issue #132).
    for (index, child) in subcomponents(element).enumerate() {
        if BODY_TAGS.contains(&child.name.as_str()) {
            let at = format!("{at}/{}[{index}]", child.name);
            let component = body(child, &at, ids, skipped, warnings);
            paths.insert(component.id.clone(), at);
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

/// Reads one body component, and everything on and inside it.
fn body(
    element: &Element,
    at: &str,
    ids: &mut Ids,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Component {
    let mut auto = Vec::new();
    let mut values = Values::new(element, at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let (overrides, overrides_include_children) = overrides(&mut values);
    let finish = finish(&mut values);
    let part = match element.name.as_str() {
        "nosecone" => nose_cone(element, at, &mut auto, warnings),
        "bodytube" => body_tube(element, at, &mut auto, warnings),
        _ => transition(element, at, &mut auto, warnings),
    };
    let children = attached::children(element, &part, at, ids, skipped, warnings);
    Component {
        id: ids.take(
            &mut Values::new(element, at, warnings),
            &element.name.clone(),
        ),
        name,
        part,
        position: None,
        auto,
        motor_mount: None,
        finish,
        overrides,
        overrides_include_children,
        children,
    }
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
) -> Part {
    let mut values = Values::new(element, at, warnings);
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (stated_m, outer_radius_m) =
        stated_radius(&mut values, &["radius"], AutoDimension::OuterRadius, auto);
    // A body tube is a wall, not a solid of revolution, so `filled` has to be said as a wall as
    // thick as the tube. With an automatic radius there is no such number yet: the tube is read as
    // the wall it caches, and says so.
    let thickness_m = match (wall(&mut values, stated_m), stated_m) {
        (Wall::Filled {}, Some(radius_m)) => radius_m,
        (Wall::Filled {}, None) if outer_radius_m > 0.0 => {
            values.warn_at(
                WarningKind::Dropped,
                "a filled tube whose radius is automatic; it was read as solid to the radius \
                 OpenRocket last worked out, which may be stale",
            );
            outer_radius_m
        }
        // Nothing cached either, so there is no number that means "solid" until the layout
        // resolves one. The tube still has to stand in the stack, so it stands as a wall of
        // nothing — and that is said as loudly as a part left out, because a solid tube read as an
        // empty one is a mass quietly missing rather than a design that fails (issue #130).
        (Wall::Filled {}, None) => {
            values.warn_at(
                WarningKind::Skipped,
                "a filled tube whose radius is automatic and nothing cached; it carries no mass, \
                 because there is no radius to fill until the layout resolves one",
            );
            0.0
        }
        (Wall::Shell { thickness_m }, _) => thickness_m,
    };
    Part::BodyTube(BodyTube {
        length_m,
        outer_radius_m,
        thickness_m,
        material: material(&mut values, &["material"], "bulk"),
    })
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
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
fn wall(values: &mut Values<'_>, outer_radius_m: Option<f64>) -> Wall {
    if values.word(&["thickness"]).as_deref() == Some("filled") {
        return Wall::Filled {};
    }
    match (values.number(&["thickness"]), outer_radius_m) {
        (Some(thickness_m), Some(radius_m)) if thickness_m >= radius_m => Wall::Filled {},
        (Some(thickness_m), _) if thickness_m > 0.0 => Wall::Shell { thickness_m },
        // A wall of nothing has neither mass nor geometry, and `hpr-design` refuses it. Read as
        // solid, the part carries the mass a solid one has. The same rule a shoulder gets below,
        // and said out loud for the same reason.
        (Some(_), _) => {
            values.warn_at(
                WarningKind::Unusual,
                "a wall of no thickness; it was read as solid",
            );
            Wall::Filled {}
        }
        (None, _) => {
            values.warn_at(
                WarningKind::Unusual,
                "no wall thickness at all; it was read as solid",
            );
            Wall::Filled {}
        }
    }
}

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
    let stated_m = values
        .number(&[&format!("{end}shoulderthickness")])
        .unwrap_or_default();
    // A shoulder with no wall at all is not a shoulder: OpenRocket offers a shoulder the same
    // "filled" choice it offers a nose cone, and writes a filled one as a zero thickness. Read as
    // a wall it would be weightless, and `hpr-design` refuses it outright (12 designs in the
    // reference corpus). Read as solid it carries the mass a solid shoulder has. Which of the two
    // OpenRocket means is for the M2.2 oracle to settle, so it is said out loud.
    // The stated wall is clamped to the radius it sits in, and only when that radius is known:
    // clamping against an automatic one that has not resolved yet would throw the wall away, which
    // is half of Loft lesson L61 and what issue #130 found still open on a shoulder.
    let thickness_m = if stated_m > 0.0 {
        known_m.map_or(stated_m, |radius_m| stated_m.min(radius_m))
    } else if let Some(radius_m) = known_m {
        values.warn_at(
            WarningKind::Unusual,
            format!("the {end} shoulder has no wall thickness; it was read as solid"),
        );
        radius_m
    } else {
        // "Solid" is a wall as thick as the radius, and the radius is not known until the layout
        // resolves it — so there is no number here that means solid. Saying "read as solid" and
        // handing on a zero would be a shoulder of no mass wearing the wrong label.
        values.warn_at(
            WarningKind::Skipped,
            format!(
                "the {end} shoulder has no wall thickness and an automatic radius, so there is no \
                 number yet that means solid; it carries no mass"
            ),
        );
        0.0
    };
    Some(Shoulder {
        length_m,
        outer_radius_m,
        thickness_m,
        // A solid shoulder has no bore to close, so a cap on one is nothing: reading it as a cap
        // asks `hpr-design` for a disc inside a tube that isn't hollow.
        capped: known_m.is_none_or(|radius_m| thickness_m < radius_m)
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
/// looked up. A part with no material is read as weightless, with a warning.
///
/// `want` is the kind of density the part needs — `bulk` for anything solid, `surface` for a
/// canopy or a streamer, `line` for a shroud line or a shock cord. The file declares a kind of its
/// own in the `type` attribute, and the number is in that kind's units, so a density declared as
/// one kind cannot be converted into another: the part is built with the kind it needs and the
/// disagreement is reported. No material in the reference corpus is declared as a kind its part
/// does not want.
pub(super) fn material(values: &mut Values<'_>, names: &[&str], want: &str) -> Material {
    let named = |name: &str, kg: f64| match want {
        "surface" => Material::surface(name, kg),
        "line" => Material::line(name, kg),
        _ => Material::bulk(name, kg),
    };
    let Some(element) = values.element(names) else {
        values.warn_at(
            WarningKind::Dropped,
            "no material, so this part weighs nothing",
        );
        return named("", 0.0);
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
/// component, so the two cannot always agree. The mass flag decides, because mass is the quantity
/// the flag is written for: 95 of the 104 in the reference corpus are `overridesubcomponentsmass`.
/// A centre-of-gravity flag that disagrees with it is reported. The drag override is read by
/// [`super::value::Values::overrides`] but waits on the milestone that charges drag to a part.
pub(super) fn overrides(values: &mut Values<'_>) -> (Overrides, bool) {
    let read = values.overrides();
    let covers_children = read.subcomponents_mass.unwrap_or_default();
    if read.cg_m.is_some()
        && read
            .subcomponents_cg
            .is_some_and(|cg| cg != covers_children)
    {
        values.warn_at(
            WarningKind::Dropped,
            format!(
                "the mass override covers the parts inside this one ({covers_children}) and \
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
