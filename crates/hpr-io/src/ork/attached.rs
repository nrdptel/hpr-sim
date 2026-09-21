//! The parts on and inside the body: what hangs off a `.ork` design's spine.
//!
//! [`super::component`] reads the spine — the stages and the body components stacked in them.
//! Everything else in a design hangs off that trunk, and this module reads it: the tubes, couplers,
//! rings and bulkheads **inside** a body component, the fins, tube fins, launch lugs and rail
//! buttons **on** it, and the mass objects, parachutes, streamers and shock cords **packed** in it.
//!
//! Three things run through all of them.
//!
//! - **Where a part sits** is an offset along its parent and the end it is measured from, which is
//!   [`Position`]. OpenRocket writes the end under `method` on the newer tag name and `type` on the
//!   older one, with the same five words.
//! - **What a part takes from its parent** is an automatic dimension: a coupler's outer radius is
//!   the tube's bore, a ring's is too, a ring's own bore is the motor tube inside it, and a packed
//!   part fills whatever room is left. None of that is worked out here —
//!   [`hpr_design::Rocket::layout`] does it, once, for every reader ([Loft lesson L60][lessons]:
//!   Loft resolved these as it walked, so the answer depended on the order the siblings were
//!   written in, and a bulkhead inside a coupler came out as `NaN`).
//! - **Angles are degrees in the file and radians in `hpr-design`**, which is easy to miss because
//!   nothing in the file says so: read as radians, `<angleoffset>180</angleoffset>` is more than
//!   twenty-eight turns instead of half of one. Of the 188 angles in the reference corpus that are
//!   not zero, 178 are larger than 2π, so they cannot be radians.
//!
//! A part this reader cannot give `hpr-design` an honest shape for is **left out with a warning**
//! rather than guessed at, so the rest of the design still opens. The guide lists every such rule:
//! [OpenRocket `.ork` design files][guide].
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/format/ork.html
//! [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md

use hpr_design::fins::{FinCrossSection, FinPlanform, FinSet, FinTab, TubeFinSet};
use hpr_design::parts::{
    CenteringRing, InnerTube, LaunchLug, MassComponent, Packing, Parachute, RailButton, ShockCord,
    Streamer,
};
use hpr_design::tree::{AutoDimension, Component, Part, Position};
use hpr_design::{Finish, MotorMount};

use super::component::{Ids, material, overrides, stated_radius, subcomponents};
use super::document::Element;
use super::motors;
use super::recovery;
use super::value::{AXIAL_OFFSET, INSTANCE_COUNT, Values};
use super::warning::{Warning, WarningKind};

/// The tags that hang off the spine and hold another part inside them.
const TUBES: [&str; 3] = ["innertube", "tubecoupler", "engineblock"];

/// Every tag this module knows how to read. A tag outside it belongs to a later milestone, and is
/// tallied by the caller rather than warned about one by one.
///
/// A slice, not an array: this list grows every time a milestone reads another kind of part, and
/// its length should not be part of the public API.
pub const ATTACHED_TAGS: &[&str] = &[
    "innertube",
    "tubecoupler",
    "engineblock",
    "centeringring",
    "bulkhead",
    "trapezoidfinset",
    "ellipticalfinset",
    "freeformfinset",
    "tubefinset",
    "launchlug",
    "railbutton",
    "masscomponent",
    "parachute",
    "streamer",
    "shockcord",
];

/// How many of one part a design may say there are.
///
/// `hpr-design` builds one body per instance and adds them up, so a count is an allocation: a file
/// saying `<fincount>4000000000</fincount>` would ask for hundreds of gigabytes, and running out
/// of memory is an abort rather than an error. The reader is the boundary with a file it did not
/// write, so the bound lives here, beside the unpacking and nesting limits the container and the
/// document already carry. The most instanced part in the reference library is a set of 8 fins.
const MOST_INSTANCES: u32 = 64;

/// How many parts are nested inside `element`, at any depth. A part left out takes them with it,
/// and a warning that does not say so is a mass quietly missing.
fn nested(element: &Element) -> usize {
    subcomponents(element).map(|child| 1 + nested(child)).sum()
}

/// `", and the 2 parts inside it"`, or nothing when there are none.
fn and_what_was_inside(element: &Element) -> String {
    match nested(element) {
        0 => String::new(),
        1 => ", and the one part inside it".to_owned(),
        many => format!(", and the {many} parts inside it"),
    }
}

/// A part kind's name as a reader would say it: `inner tube`, not `inner_tube`.
fn spoken(part: &Part) -> String {
    part.kind_name().replace('_', " ")
}

/// Reads everything in `element`'s `<subcomponents>` that belongs to a body component or a tube.
///
/// `parent` is what the parent was read as, which decides two things a child cannot decide for
/// itself: whether an external part may attach here at all, and whether an automatic radius has a
/// bore to take. Anything left out is counted into `skipped` or warned about, never dropped in
/// silence.
pub(super) fn children(
    element: &Element,
    parent: &Part,
    at: &str,
    ids: &mut Ids,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Vec<Component> {
    let mut components = Vec::new();
    for (index, child) in subcomponents(element).enumerate() {
        let at = format!("{at}/{}[{index}]", child.name);
        match one(child, parent, &at, ids, skipped, warnings) {
            Some(component) => components.push(component),
            // A part this module knows and could not read has already said so, with its reason;
            // only a tag no milestone reads yet goes into the tally.
            None => {
                if !ATTACHED_TAGS.contains(&child.name.as_str()) {
                    skipped.push(child.name.clone());
                }
            }
        }
    }
    components
}

/// Reads one attached part, or leaves it out.
fn one(
    element: &Element,
    parent: &Part,
    at: &str,
    ids: &mut Ids,
    skipped: &mut Vec<String>,
    warnings: &mut Vec<Warning>,
) -> Option<Component> {
    let mut auto = Vec::new();
    let mut values = Values::new(element, at, warnings);
    let part = match element.name.as_str() {
        tag if TUBES.contains(&tag) => inner_tube(&mut values, &mut auto),
        "centeringring" => ring(&mut values, &mut auto, true),
        "bulkhead" => ring(&mut values, &mut auto, false),
        "trapezoidfinset" | "ellipticalfinset" | "freeformfinset" => fin_set(element, &mut values),
        "tubefinset" => tube_fins(&mut values),
        "launchlug" => launch_lug(&mut values),
        "railbutton" => rail_button(&mut values),
        "masscomponent" => mass_component(&mut values, &mut auto),
        "parachute" => parachute(&mut values, &mut auto),
        "streamer" => streamer(&mut values, &mut auto),
        "shockcord" => shock_cord(&mut values, &mut auto),
        // Pods and parallel stages hold a spine of their own, which is M3.1c's work, and anything
        // else is a tag this reader has never seen. Both are counted by the caller.
        _ => return None,
    }?;

    // `hpr-design` attaches fins, tube fins, lugs and rail buttons to a body tube and nothing else.
    // OpenRocket lets them sit on a nose cone or a transition, where their root is not a straight
    // line; reading one onto a tube would put it on a body it does not have.
    if part.is_external() && !matches!(parent, Part::BodyTube(_)) {
        values.warn_at(
            WarningKind::Skipped,
            format!(
                "a {} sits on a {}, and hpr attaches one only to a body tube; it was left out{}",
                spoken(&part),
                spoken(parent),
                and_what_was_inside(element)
            ),
        );
        return None;
    }
    // An automatic outer or packed radius is the parent's bore, and a nose cone or transition has
    // none to give. The layout would refuse the whole design over it, so the part goes instead. A
    // ring's automatic *bore* is not in this list: it comes from the ring's siblings, and is zero
    // when none of them is a motor tube, so it needs nothing of the parent.
    let needs_a_bore = auto.iter().any(|dimension| {
        matches!(
            dimension,
            AutoDimension::OuterRadius | AutoDimension::PackedRadius
        )
    });
    if needs_a_bore && !matches!(parent, Part::BodyTube(_) | Part::InnerTube(_)) {
        values.warn_at(
            WarningKind::Skipped,
            format!(
                "the {} takes an automatic radius from its parent's bore, and a {} has none; it \
                 was left out{}",
                spoken(&part),
                spoken(parent),
                and_what_was_inside(element)
            ),
        );
        return None;
    }

    let name = values.word(&["name"]).unwrap_or_default();
    let position = position(&mut values);
    let finish = finish(&mut values);
    let (overrides, include_children) = overrides(&mut values);
    radial_offset_on_the_surface(&mut values, &part);
    // Only a tube holds other parts. `hpr-design` says the same, so anything written inside
    // another kind is said out loud here rather than tallied with the pods, whose tally carries a
    // message about a spine of their own. Nothing in the reference library does this.
    let children = if matches!(part, Part::InnerTube(_)) {
        children(element, &part, at, ids, skipped, warnings)
    } else {
        if subcomponents(element).next().is_some() {
            let (kind, inside) = (spoken(&part), and_what_was_inside(element));
            values.warn_at(
                WarningKind::Skipped,
                format!("a {kind} holds no parts in hpr; it was read without{inside}"),
            );
        }
        Vec::new()
    };
    let mount = match part {
        Part::InnerTube(_) => motors::mount(element, at, warnings),
        _ => None,
    };
    let device = match part {
        Part::Parachute(_) | Part::Streamer(_) => Some(recovery::device(element, at, warnings)),
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
    if let Some(device) = device {
        ids.devices.push((id.clone(), device));
    }
    Some(Component {
        id,
        name,
        part,
        position: Some(position),
        auto,
        motor_mount,
        finish,
        overrides,
        overrides_include_children: include_children,
        children,
    })
}

/// An inner tube, tube coupler or engine block: all three are a tube inside another one.
fn inner_tube(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Option<Part> {
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (stated_m, outer_radius_m) =
        stated_radius(values, &["outerradius"], AutoDimension::OuterRadius, auto);
    let thickness_m = tube_wall(values, stated_m)?;
    if values
        .word(&["clusterconfiguration"])
        .is_some_and(|c| c != "single")
    {
        values.warn_at(
            WarningKind::Dropped,
            "a cluster of motor tubes is read as the one tube it is written as; hpr does not \
             model clusters yet",
        );
    }
    Some(Part::InnerTube(InnerTube {
        length_m,
        outer_radius_m,
        thickness_m,
        radial_offset_m: values.number(&["radialposition"]).unwrap_or_default(),
        angle_rad: roll_angle(values),
        material: material(values, &["material"], "bulk"),
    }))
}

/// A tube's wall, in metres, or `None` when the tube cannot be read at all.
///
/// A tube of **no** wall thickness is read as exactly that, and carries no mass. It is tempting to
/// read it as solid — the rule a body component and a shoulder get on the spine, where the file
/// has a `filled` spelling to mean it — but an inner tube has no such spelling, and OpenRocket's
/// own geometry makes a tube's bore its outer radius less its wall, so a wall of nothing is a part
/// of nothing. Reading it as solid would invent the mass instead: a solid coupler filling a 50 mm
/// airframe for 180 mm is a few hundred grams the design never had. Seven tube couplers in the
/// reference corpus are written this way, three of them in two of OpenRocket's own example
/// designs, and four launch lugs say the same of themselves.
fn tube_wall(values: &mut Values<'_>, outer_radius_m: Option<f64>) -> Option<f64> {
    if values.word(&["thickness"]).as_deref() == Some("filled") {
        return match outer_radius_m {
            Some(radius_m) => Some(radius_m),
            None => {
                values.warn_at(
                    WarningKind::Skipped,
                    "a filled tube whose outer radius is automatic; there is no radius to fill \
                     until the layout resolves one, so it was left out",
                );
                None
            }
        };
    }
    let Some(thickness_m) = values.number(&["thickness"]) else {
        // A *stated* zero is OpenRocket saying the bore reaches the rim. A missing tag says
        // nothing at all, and the spine reads that as solid, because a body component can be
        // written `filled` and an absent thickness is likelier a writer that omitted it. A tube
        // has no `filled` to be confused with, so the two cases part company here: read as solid,
        // a tube whose wall the file never gave would invent the mass of a rod. Nothing in the
        // reference library omits it.
        values.warn_at(
            WarningKind::Dropped,
            "a tube with no wall thickness at all; it was read as a tube of no wall, which \
             carries no mass, rather than as the solid rod a filled one would be",
        );
        return Some(0.0);
    };
    if thickness_m == 0.0 {
        values.warn_at(
            WarningKind::Unusual,
            "a tube of no wall thickness; it carries no mass, which is what OpenRocket's own \
             geometry gives it",
        );
        return Some(0.0);
    }
    if thickness_m < 0.0 {
        values.warn_at(
            WarningKind::Skipped,
            "a tube of negative wall thickness; it was left out",
        );
        return None;
    }
    // A wall thicker than the tube is the tube solid. With an automatic radius there is no number
    // to compare against yet, so the wall stands as written and the layout decides.
    Some(match outer_radius_m {
        Some(radius_m) => thickness_m.min(radius_m),
        None => thickness_m,
    })
}

/// A centering ring, or a bulkhead, which is a ring with no bore.
fn ring(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>, bored: bool) -> Option<Part> {
    let length_m = values.number(&["length"]).unwrap_or_default();
    let (_, outer_radius_m) =
        stated_radius(values, &["outerradius"], AutoDimension::OuterRadius, auto);
    let inner_radius_m = if bored {
        stated_radius(values, &["innerradius"], AutoDimension::InnerRadius, auto).1
    } else {
        0.0
    };
    if inner_radius_m > outer_radius_m && !auto.contains(&AutoDimension::OuterRadius) {
        values.warn_at(
            WarningKind::Skipped,
            "a ring whose bore is wider than the ring; it was left out",
        );
        return None;
    }
    instanced_once(values, "ring");
    off_the_axis(values, "ring");
    Some(Part::CenteringRing(CenteringRing {
        length_m,
        outer_radius_m,
        inner_radius_m,
        material: material(values, &["material"], "bulk"),
    }))
}

/// A fin set of any of the three outlines OpenRocket writes.
fn fin_set(element: &Element, values: &mut Values<'_>) -> Option<Part> {
    let planform = match element.name.as_str() {
        "trapezoidfinset" => FinPlanform::Trapezoidal {
            root_chord_m: values.number(&["rootchord"]).unwrap_or_default(),
            tip_chord_m: values.number(&["tipchord"]).unwrap_or_default(),
            span_m: values.number(&["height"]).unwrap_or_default(),
            sweep_m: values.number(&["sweeplength"]).unwrap_or_default(),
        },
        "ellipticalfinset" => FinPlanform::Elliptical {
            root_chord_m: values.number(&["rootchord"]).unwrap_or_default(),
            span_m: values.number(&["height"]).unwrap_or_default(),
        },
        _ => outline(values)?,
    };
    let count = instances(values, "fin set")?;
    if values.number(&["filletradius"]).is_some_and(|r| r > 0.0) {
        values.warn_at(
            WarningKind::Dropped,
            "the fillets along the fin roots were dropped; hpr does not model their mass",
        );
    }
    Some(Part::FinSet(FinSet {
        count,
        thickness_m: values.number(&["thickness"]).unwrap_or_default(),
        cross_section: cross_section(values),
        tab: tab(values, planform.root_chord_m()),
        // Degrees, like every other angle in the file (see `roll_angle`). The corpus's two
        // non-zero cants are 1.0 and -3.98, which as radians would be 57° and 228° — a fin turned
        // more than half a turn from the airflow, which is not a cant anyone builds and which
        // OpenRocket's own roll model (Niskanen eq. 3.10, `δ` small) would not mean.
        cant_rad: values.number(&["cant"]).unwrap_or_default().to_radians(),
        base_angle_rad: roll_angle(values),
        material: material(values, &["material"], "bulk"),
        planform,
    }))
}

/// A freeform fin's outline, as `[x, h]` from the root leading edge.
///
/// `hpr-design` wants a polygon that starts at the root leading edge and ends on the root, because
/// that is what makes the root a chord it can integrate along. OpenRocket allows a last point off
/// the root — a fin whose trailing edge stands clear of the body — and there is no honest way to
/// read one as a fin that touches: closing it along the root would add planform the design does
/// not have. So it is left out.
fn outline(values: &mut Values<'_>) -> Option<FinPlanform> {
    let Some(points) = values.element(&["finpoints"]) else {
        values.warn_at(
            WarningKind::Skipped,
            "a freeform fin set with no outline; it was left out",
        );
        return None;
    };
    let mut points_m = Vec::new();
    for point in points.elements().filter(|point| point.name == "point") {
        let read = |name: &str| {
            point
                .attribute(name)
                .and_then(|text| text.parse::<f64>().ok())
                .filter(|value| value.is_finite())
        };
        match (read("x"), read("y")) {
            (Some(x), Some(h)) => points_m.push([x, h]),
            _ => {
                values.warn_at(
                    WarningKind::Skipped,
                    "a freeform fin outline with a point that is not two numbers; it was left out",
                );
                return None;
            }
        }
    }
    if points_m.len() < 3 {
        values.warn_at(
            WarningKind::Skipped,
            "a freeform fin outline of fewer than three points, which encloses no fin; it was \
             left out",
        );
        return None;
    }
    let ends_on_the_root = matches!(points_m.last(), Some(last) if last[1] == 0.0 && last[0] > 0.0);
    if points_m.first() != Some(&[0.0, 0.0]) || !ends_on_the_root {
        values.warn_at(
            WarningKind::Skipped,
            "a freeform fin outline that does not run from the root leading edge to the root \
             trailing edge; it was left out rather than closed along a root it never touches",
        );
        return None;
    }
    Some(FinPlanform::Freeform { points_m })
}

/// The section shape along a fin's chord.
fn cross_section(values: &mut Values<'_>) -> FinCrossSection {
    match values.word(&["crosssection"]).as_deref() {
        Some("rounded") => FinCrossSection::Rounded,
        Some("airfoil") => FinCrossSection::Airfoil,
        None | Some("square") => FinCrossSection::Square,
        Some(other) => {
            let other = other.to_owned();
            values.warn_at(
                WarningKind::Unusual,
                format!("`{other}` is not a fin section this reader knows; it was read as square"),
            );
            FinCrossSection::Square
        }
    }
}

/// The tab below a fin's root, if it has one.
///
/// OpenRocket measures the tab from the fin's front, its centre or its end, saying which in
/// `relativeto`; `hpr-design` states one thing, the distance from the root leading edge aft to the
/// tab's leading edge.
///
/// **What each word measures from is read off the corpus, not a specification.** `center` is taken
/// as centre-to-centre, which is the only reading under which the corpus's eight non-zero offsets
/// land inside their root chords — any other puts a tab off the end of the fin, which
/// `FinSet::validate` refuses. `end` is the mirror of `front` and appears in no file at all, so
/// that arm rests on symmetry alone.
fn tab(values: &mut Values<'_>, root_chord_m: f64) -> Option<FinTab> {
    let height_m = values.number(&["tabheight"])?;
    let length_m = values.number(&["tablength"]).unwrap_or_default();
    if height_m <= 0.0 || length_m <= 0.0 {
        return None;
    }
    let element = values.element(&["tabposition"]);
    let relative_to = element
        .and_then(|element| element.attribute("relativeto"))
        .unwrap_or("front")
        .to_owned();
    let offset_m = values.number(&["tabposition"]).unwrap_or_default();
    let offset_m = match relative_to.as_str() {
        // OpenRocket writes `<tabposition>` twice, the newer name's frame first. The older
        // vocabulary is the same three places under the names the axial offset uses, and across
        // the reference library the two elements always carry the same number, so either spelling
        // means the same tab.
        "front" | "top" => offset_m,
        "center" | "middle" => 0.5 * (root_chord_m - length_m) + offset_m,
        "end" | "bottom" => root_chord_m - length_m + offset_m,
        other => {
            values.warn_at(
                WarningKind::Unusual,
                format!(
                    "a fin tab measured from `{other}`, which this reader does not know; it was \
                     read from the root leading edge"
                ),
            );
            offset_m
        }
    };
    Some(FinTab {
        height_m,
        length_m,
        offset_m,
    })
}

/// A ring of tubes around the body.
fn tube_fins(values: &mut Values<'_>) -> Option<Part> {
    let mut auto = Vec::new();
    let (stated_m, outer_radius_m) =
        stated_radius(values, &["radius"], AutoDimension::OuterRadius, &mut auto);
    if stated_m.is_none() {
        // OpenRocket sizes an automatic tube-fin radius so the tubes close the ring around the
        // body. `hpr-design` has no automatic dimension for it, and guessing one here would put a
        // number in the design that no source backs. Tracked as an issue, not invented.
        values.warn_at(
            WarningKind::Skipped,
            "a tube fin set whose radius OpenRocket works out from the body; hpr does not resolve \
             that yet, so it was left out",
        );
        return None;
    }
    let thickness_m = tube_wall(values, stated_m)?;
    let count = instances(values, "tube fin set")?;
    Some(Part::TubeFinSet(TubeFinSet {
        count,
        length_m: values.number(&["length"]).unwrap_or_default(),
        outer_radius_m,
        thickness_m,
        base_angle_rad: roll_angle(values),
        material: material(values, &["material"], "bulk"),
    }))
}

/// A launch lug, or a row of them.
fn launch_lug(values: &mut Values<'_>) -> Option<Part> {
    let mut auto = Vec::new();
    let (stated_m, outer_radius_m) =
        stated_radius(values, &["radius"], AutoDimension::OuterRadius, &mut auto);
    if stated_m.is_none() {
        // A lug has no automatic radius in `hpr-design`, and the number OpenRocket cached is the
        // rod it last had, which may not be this one's. Reading it would put a stated radius where
        // the file says "work it out", and a bare `auto` would leave a lug of no radius that the
        // layout refuses — taking the whole design with it. No lug in the reference library is
        // written this way.
        values.warn_at(
            WarningKind::Skipped,
            "a launch lug whose radius OpenRocket works out for itself; hpr does not resolve that, \
             so it was left out",
        );
        return None;
    }
    let thickness_m = tube_wall(values, stated_m)?;
    let (count, spacing_m) = row(values, "row of launch lugs")?;
    Some(Part::LaunchLug(LaunchLug {
        length_m: values.number(&["length"]).unwrap_or_default(),
        outer_radius_m,
        thickness_m,
        angle_rad: roll_angle(values),
        count,
        spacing_m,
        material: material(values, &["material"], "bulk"),
    }))
}

/// A rail button, or a row of them.
fn rail_button(values: &mut Values<'_>) -> Option<Part> {
    let (count, spacing_m) = row(values, "row of rail buttons")?;
    if values.number(&["screwheight"]).is_some_and(|h| h > 0.0) {
        values.warn_at(
            WarningKind::Dropped,
            "the screw head above the rail button was dropped; hpr does not model its mass",
        );
    }
    Some(Part::RailButton(RailButton {
        outer_diameter_m: values.number(&["outerdiameter"]).unwrap_or_default(),
        inner_diameter_m: values.number(&["innerdiameter"]).unwrap_or_default(),
        height_m: values.number(&["height"]).unwrap_or_default(),
        base_height_m: values.number(&["baseheight"]).unwrap_or_default(),
        flange_height_m: values.number(&["flangeheight"]).unwrap_or_default(),
        angle_rad: roll_angle(values),
        count,
        spacing_m,
        material: material(values, &["material"], "bulk"),
    }))
}

/// A lump of mass with no geometry of its own beyond how it is packed.
fn mass_component(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Option<Part> {
    Some(Part::MassComponent(MassComponent {
        mass_kg: values.number(&["mass"]).unwrap_or_default(),
        packing: packing(values, auto),
    }))
}

/// A parachute: a canopy of fabric on a set of shroud lines, packed into the body.
fn parachute(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Option<Part> {
    Some(Part::Parachute(Parachute {
        diameter_m: values.number(&["diameter"]).unwrap_or_default(),
        canopy_material: material(values, &["material"], "surface"),
        line_count: values.count(&["linecount"]).unwrap_or_default(),
        line_length_m: values.number(&["linelength"]).unwrap_or_default(),
        line_material: material(values, &["linematerial"], "line"),
        packing: packing(values, auto),
    }))
}

/// A streamer: a strip of fabric, packed into the body.
fn streamer(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Option<Part> {
    Some(Part::Streamer(Streamer {
        length_m: values.number(&["striplength"]).unwrap_or_default(),
        width_m: values.number(&["stripwidth"]).unwrap_or_default(),
        material: material(values, &["material"], "surface"),
        packing: packing(values, auto),
    }))
}

/// A shock cord, packed into the body.
fn shock_cord(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Option<Part> {
    Some(Part::ShockCord(ShockCord {
        length_m: values.number(&["cordlength"]).unwrap_or_default(),
        material: material(values, &["material"], "line"),
        packing: packing(values, auto),
    }))
}

/// How a mass object or a recovery part is packed: the cylinder it takes up inside the body.
fn packing(values: &mut Values<'_>, auto: &mut Vec<AutoDimension>) -> Packing {
    let (_, radius_m) = stated_radius(values, &["packedradius"], AutoDimension::PackedRadius, auto);
    Packing {
        length_m: values.number(&["packedlength"]).unwrap_or_default(),
        radius_m,
        radial_offset_m: values.number(&["radialposition"]).unwrap_or_default(),
        angle_rad: roll_angle(values),
    }
}

/// How many of an instanced part there are, and how far apart they sit along the body. `None` when
/// the file asks for more than [`MOST_INSTANCES`].
fn row(values: &mut Values<'_>, what: &str) -> Option<(u32, f64)> {
    let count = instances(values, what)?;
    Some((
        count.max(1),
        values.number(&["instanceseparation"]).unwrap_or_default(),
    ))
}

/// How many of a part the file says there are, refusing a count no rocket has and a file could
/// only give by accident or on purpose. See [`MOST_INSTANCES`].
fn instances(values: &mut Values<'_>, what: &str) -> Option<u32> {
    let count = values.count(&INSTANCE_COUNT).unwrap_or(1);
    if count == 0 {
        let what = what.to_owned();
        values.warn_at(
            WarningKind::Skipped,
            format!("a {what} of none; it was left out"),
        );
        return None;
    }
    if count > MOST_INSTANCES {
        let what = what.to_owned();
        values.warn_at(
            WarningKind::Skipped,
            format!(
                "a {what} of {count}, where hpr builds each one and adds them up; it was left out \
                 rather than asked for"
            ),
        );
        return None;
    }
    Some(count)
}

/// Says so when a part is written more than once and `hpr-design` models one of it.
fn instanced_once(values: &mut Values<'_>, what: &str) {
    if values.count(&INSTANCE_COUNT).is_some_and(|count| count > 1) {
        let what = what.to_owned();
        values.warn_at(
            WarningKind::Dropped,
            format!("a row of more than one {what} was read as the one it is written as"),
        );
    }
}

/// Says so when a part that `hpr-design` keeps on the axis is written off it.
fn off_the_axis(values: &mut Values<'_>, what: &str) {
    if values
        .number(&["radialposition"])
        .is_some_and(|offset| offset != 0.0)
    {
        let what = what.to_owned();
        values.warn_at(
            WarningKind::Dropped,
            format!("a {what} off the body axis was read on it; hpr keeps one on the axis"),
        );
    }
}

/// Says so when an external part is written standing off the body's surface.
///
/// OpenRocket measures a fin's or a lug's radius from the surface (`method="surface"`), or, for
/// tube fins, from the body it wraps (`"coaxial"`). `hpr-design` sits every external part on the
/// surface, so any offset but zero is a standoff it does not model. Every one of the 95 written in
/// the reference corpus is zero.
fn radial_offset_on_the_surface(values: &mut Values<'_>, part: &Part) {
    if !part.is_external() {
        return;
    }
    if values
        .number(&["radiusoffset"])
        .is_some_and(|offset| offset != 0.0)
    {
        let kind = spoken(part);
        values.warn_at(
            WarningKind::Dropped,
            format!("a {kind} standing off the body was read sitting on it"),
        );
    }
}

/// Where a part sits along its parent.
///
/// The offset and the end it is measured from are one tag: `<axialoffset method="bottom">` on the
/// newer name, `<position type="bottom">` on the older, with the same five words. A part with
/// neither is read flush with its parent's forward end, which is what OpenRocket's own default is
/// for everything but a fin set; one tube coupler in the reference corpus is written that way.
fn position(values: &mut Values<'_>) -> Position {
    let Some(element) = values.element(&AXIAL_OFFSET) else {
        values.warn_at(
            WarningKind::Unusual,
            "no axial offset at all; the part was read flush with its parent's forward end",
        );
        return Position::Top { aft_offset_m: 0.0 };
    };
    let from = element
        .attribute("method")
        .or_else(|| element.attribute("type"))
        .unwrap_or("top")
        .to_owned();
    // From the element already found, not a second lookup: asking `Values` again would run the
    // two-name comparison a second time and warn twice about one disagreement.
    let text = element.text();
    let aft_offset_m = match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => value,
        _ => {
            let (name, text) = (element.name.clone(), text.trim().to_owned());
            values.warn_at(
                WarningKind::Dropped,
                format!("`{name}` says `{text}`, which is not a number; it was read as zero"),
            );
            0.0
        }
    };
    match from.as_str() {
        "top" => Position::Top { aft_offset_m },
        "middle" => Position::Middle { aft_offset_m },
        "bottom" => Position::Bottom { aft_offset_m },
        "after" => Position::After { aft_offset_m },
        "absolute" => Position::Absolute {
            station_m: aft_offset_m,
        },
        other => {
            values.warn_at(
                WarningKind::Unusual,
                format!(
                    "an axial offset measured from `{other}`, which this reader does not know; it \
                     was read from the parent's forward end"
                ),
            );
            Position::Top { aft_offset_m }
        }
    }
}

/// The surface finish, as a roughness height.
///
/// OpenRocket writes one of five words. What each is worth in micrometres is not in the file
/// format documentation; the numbers below are from the program's author, on The Rocketry Forum
/// (thread "Open Rocket Finishes", post #6, 22 August 2013):
///
/// | word | OpenRocket's name for it | roughness |
/// | --- | --- | --- |
/// | `rough` | Rough | 500 µm |
/// | `unfinished` | Unfinished | 150 µm |
/// | `normal` | Regular paint | 60 µm |
/// | `smooth` | Smooth paint | 20 µm |
/// | `polished` | Polished | 2 µm |
///
/// The default, `normal`, is confirmed twice over: the [OpenRocket technical documentation][doc]
/// section 6 says "the 'regular paint' finish was selected, which corresponds to an average
/// surface roughness of 60 µm", and the user guide's body-tube dialog reads "Regular paint
/// (2.36 mil)", which is 59.9 µm. The guide's [`.ork` page][guide] has the rest, including what is
/// not settled about `polished`.
///
/// [doc]: https://openrocket.sourceforge.net/techdoc.pdf
/// [guide]: https://nrdptel.github.io/hpr-sim/format/ork.html
pub(super) fn finish(values: &mut Values<'_>) -> Option<Finish> {
    let word = values.word(&["finish"])?;
    let roughness_m = match word.as_str() {
        "rough" => 500e-6,
        "unfinished" => 150e-6,
        "normal" => 60e-6,
        "smooth" => 20e-6,
        "polished" => 2e-6,
        other => {
            let other = other.to_owned();
            values.warn_at(
                WarningKind::Unusual,
                format!(
                    "`{other}` is not a surface finish this reader has a roughness for; the part \
                     took hpr's default"
                ),
            );
            return None;
        }
    };
    Some(Finish::Custom { roughness_m })
}

/// A part's roll angle around the body, in radians.
///
/// **`.ork` angles are degrees.** Nothing in the file says so, and every length beside them is in
/// metres, so reading one as radians is an easy mistake: `<angleoffset>180</angleoffset>` would be
/// more than twenty-eight turns instead of half of one. The corpus settles it. Of the 993 angles
/// written in it, 188 are not zero, and **178 of those are larger than 2π** — more than a whole
/// turn, which no component is written at. The values themselves are 180, 90, 45, 30 and 120, with
/// float dust (`119.99999999999999`) from a conversion that went through radians and back.
/// `cargo xtask ork` prints all three counts.
///
/// OpenRocket writes the angle under a newer name, `angleoffset`, and an older one — `rotation` on
/// a fin set, `radialdirection` on everything else. Those are **not** read as one tag by
/// [`Values::element`]: the newer name carries a `method` attribute that the older never does, and
/// [ADR-052][adr] left the pair alone for want of a source saying what the older name's frame is.
/// What settles it here is narrower than that question and enough for it: on the 95 fin sets and
/// 26 other parts of the reference corpus that carry both names, the two agree on the **number**
/// every time, so which one is read cannot change an angle. The frames — `relative` to the parent
/// and `fixed` in the rocket — are the same angle for every parent this reader builds, because all
/// of them sit on the rocket's own axis. A pod set does not, and a pod set is M3.1c's work.
///
/// **Which way the angle turns is assumed, not sourced.** `docs/physics/frames.md` measures a roll
/// angle from `x_B` toward `y_B`, right-handed about `+z_B`, which points at the nose; OpenRocket's
/// technical documentation (§3.1.4) puts its own `+x` along the centreline pointing *aft* and
/// leaves the other two axes unstated. A right-handed angle about an aft-pointing axis is a
/// left-handed one about `+z_B`, so if OpenRocket means that, every angle read here is mirrored —
/// a mass object at 90° sits on the other side, and a canted fin set rolls the other way. Nothing
/// in the corpus can settle it, because a mirrored design is still a valid design; one asymmetric
/// design through the OpenRocket oracle (M2.2) will. Until then this reader takes the number
/// unchanged, and the guide lists it among the readings that are not settled.
///
/// [adr]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
fn roll_angle(values: &mut Values<'_>) -> f64 {
    let newer = values.number(&["angleoffset"]);
    let older = values
        .number(&["rotation"])
        .or_else(|| values.number(&["radialdirection"]));
    if let (Some(newer), Some(older)) = (newer, older)
        && newer != older
    {
        values.warn_at(
            WarningKind::Dropped,
            format!(
                "`angleoffset` says `{newer}` and the older name says `{older}`; they are two \
                 names for one angle, so `angleoffset` was taken"
            ),
        );
    }
    newer.or(older).unwrap_or_default().to_radians()
}
