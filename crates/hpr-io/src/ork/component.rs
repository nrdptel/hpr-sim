//! The spine of a design: its stages and the body components stacked inside them.
//!
//! A `.ork` design is a tree. Its trunk is the **spine**: the stages, and inside each of them the
//! nose cones, body tubes and transitions that stack end to end along the axis. Everything else —
//! the tubes and rings inside the body, the fins and lugs on it, the recovery gear — hangs off that
//! trunk, and is read by the milestone after this one ([M3.1b2b][roadmap]).
//!
//! What this module does is turn the spine into [`hpr_design`] types: a [`Rocket`] of [`Stage`]s of
//! [`Component`]s. It resolves nothing itself. Where OpenRocket wrote `auto`, the component carries
//! an [`AutoDimension`] and [`Rocket::layout`] works the radius out from the neighbours, which is
//! the one place that rule lives.
//!
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md

use hpr_design::parts::{BodyTube, NoseCone, Shoulder, Transition};
use hpr_design::shapes::NoseShape;
use hpr_design::solids::Wall;
use hpr_design::tree::{
    AutoDimension, Component, Overrides, Part, ReferenceDiameter, Rocket, Stage,
};
use hpr_design::{Density, Material};

use super::document::{Document, Element};
use super::value::Values;
use super::warning::{Imported, Warning, WarningKind};

/// The body tags this milestone reads. Anything else in a `<subcomponents>` is counted and left.
const BODY_TAGS: [&str; 3] = ["nosecone", "bodytube", "transition"];

/// Reads a design document's spine.
///
/// Never fails: a tag it cannot read is left out with a [`Warning`], so a design written by another
/// program still opens. The result is a [`Rocket`] whose body components are in file order, forward
/// to aft, with automatic radii marked rather than filled in.
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

    let mut ids = Ids::default();
    let mut skipped: Vec<String> = Vec::new();
    for (index, stage_element) in subcomponents(element).enumerate() {
        if stage_element.name != "stage" {
            skipped.push(stage_element.name.clone());
            continue;
        }
        rocket.stages.push(stage(
            stage_element,
            index,
            &mut ids,
            &mut skipped,
            &mut warnings,
        ));
    }
    if let Some(note) = tally(&skipped) {
        warnings.push(Warning::new(
            at,
            WarningKind::Skipped,
            format!("{note} were left out: this milestone reads the spine only"),
        ));
    }
    Imported {
        value: rocket,
        warnings,
    }
}

/// Reads one `<stage>`, and everything stacked inside it.
fn stage(
    element: &Element,
    index: usize,
    ids: &mut Ids,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Stage {
    let at = format!("openrocket/rocket/stage[{index}]");
    let mut values = Values::new(element, &at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let overrides = overrides(&mut values);
    let id = ids.take(&mut Values::new(element, &at, warnings), "stage");
    let mut components = Vec::new();
    for child in subcomponents(element) {
        if BODY_TAGS.contains(&child.name.as_str()) {
            let at = format!("{at}/{}", child.name);
            components.push(body(child, &at, ids, skipped, warnings));
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

/// Reads one body component, and counts what hangs off it.
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
    let overrides = overrides(&mut values);
    let part = match element.name.as_str() {
        "nosecone" => nose_cone(element, at, &mut auto, warnings),
        "bodytube" => body_tube(element, at, &mut auto, warnings),
        _ => transition(element, at, &mut auto, warnings),
    };
    for child in subcomponents(element) {
        skipped.push(child.name.clone());
    }
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
        finish: None,
        overrides,
        overrides_include_children: false,
        children: Vec::new(),
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
        material: material(&mut values),
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
        (Wall::Filled {}, None) => {
            values.warn_at(
                WarningKind::Dropped,
                "a filled tube whose radius is automatic; it was read as solid to the radius \
                 OpenRocket last worked out",
            );
            outer_radius_m
        }
        (Wall::Shell { thickness_m }, _) => thickness_m,
    };
    Part::BodyTube(BodyTube {
        length_m,
        outer_radius_m,
        thickness_m,
        material: material(&mut values),
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
        material: material(&mut values),
    })
}

/// A radius that may be automatic. An automatic one records `dimension` for [`Rocket::layout`] to
/// resolve, and keeps whatever OpenRocket last worked out as the value until it does.
fn radius(
    values: &mut Values<'_>,
    names: &[&str],
    dimension: AutoDimension,
    auto: &mut Vec<AutoDimension>,
) -> f64 {
    stated_radius(values, names, dimension, auto).1
}

/// The radius, and what it is worth to a reader before the layout resolves it: `None` when the file
/// says `auto`, whether or not OpenRocket cached a number with it, because the cached number is the
/// neighbour it last had and may be stale.
fn stated_radius(
    values: &mut Values<'_>,
    names: &[&str],
    dimension: AutoDimension,
    auto: &mut Vec<AutoDimension>,
) -> (Option<f64>, f64) {
    let Some(read) = values.dimension(names) else {
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
        (Some(thickness_m), _) => Wall::Shell { thickness_m },
        (None, _) => Wall::Filled {},
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
    let outer_radius_m = radius(values, &[&format!("{end}shoulderradius")], dimension, auto);
    let thickness_m = values
        .number(&[&format!("{end}shoulderthickness")])
        .unwrap_or_default();
    Some(Shoulder {
        length_m,
        outer_radius_m,
        thickness_m: thickness_m.min(outer_radius_m),
        capped: values
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

/// The bulk material a part is made of. A `.ork` stores the density with the name, so nothing is
/// looked up. A part with no material is read as weightless, with a warning.
fn material(values: &mut Values<'_>) -> Material {
    let Some(element) = values.element(&["material"]) else {
        values.warn_at(
            WarningKind::Dropped,
            "no material, so this part weighs nothing",
        );
        return Material::bulk("", 0.0);
    };
    let name = element.text().trim().to_owned();
    let density = element
        .attribute("density")
        .and_then(|text| text.parse::<f64>().ok())
        .filter(|density| density.is_finite() && *density >= 0.0);
    match density {
        Some(kg_m3) => Material {
            name,
            density: Density::Bulk { kg_m3 },
        },
        None => {
            values.warn_at(
                WarningKind::Dropped,
                format!("the material `{name}` states no density; it weighs nothing"),
            );
            Material::bulk(name, 0.0)
        }
    }
}

/// The overrides, as `hpr-design` states them. The drag override and the per-quantity flags are
/// read by [`super::value::Values::overrides`] but wait on the milestone that applies them.
fn overrides(values: &mut Values<'_>) -> Overrides {
    let read = values.overrides();
    Overrides {
        mass_kg: read.mass_kg,
        cg_aft_m: read.cg_m,
        cg_xy_m: None,
        inertia: None,
    }
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
fn subcomponents(element: &Element) -> impl Iterator<Item = &Element> {
    element
        .child("subcomponents")
        .into_iter()
        .flat_map(Element::elements)
}

/// Unique ids: a `.ork` gives most components one, but not all of them, and `hpr-design` needs
/// every id to be distinct.
#[derive(Debug, Default)]
struct Ids {
    used: usize,
}

impl Ids {
    fn take(&mut self, values: &mut Values<'_>, kind: &str) -> String {
        self.used += 1;
        match values.word(&["id"]).filter(|id| !id.is_empty()) {
            Some(id) => id,
            None => format!("{kind}-{}", self.used),
        }
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
