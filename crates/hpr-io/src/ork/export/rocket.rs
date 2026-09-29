//! The rocket: its stages, the body components stacked in them, and the parts on and inside each
//! of those, written as [`super::super::component`] and [`super::super::attached`] read them.
//!
//! Each tag is the one those readers ask for, under its newest name, with the value the design
//! holds, so that reading the written rocket gives back the same rocket. Three things take more
//! than writing a number down.
//!
//! - **A reader that converts a value** is inverted so the value reads back bit for bit: an angle
//!   is degrees in the file and radians in the design, an ogive's shape parameter is the
//!   reciprocal of the design's radius ratio, and a rail button's offset is moved by its radius.
//!   [`preimage`] finds a number the reader turns into exactly the design's, the shortest decimal
//!   that does when there is one.
//! - **One design part can come from several tags**: a tube coupler, an engine block and an inner
//!   tube are all an inner tube; a bulkhead is a centering ring with no bore; a nose cone turned
//!   round is a transition to a point. The tag written is the one the file had, when what the
//!   design keeps of that file says so (a kept tag or attribute of the part names its path), and
//!   otherwise the plainest that reads back the same.
//! - **What the design keeps but does not model** is put back by [`super::kept`], at paths that
//!   count every part the file had. So each part here is written at the path it was read from:
//!   the kept parts' places among their siblings are left free for them.

use std::collections::BTreeSet;

use hpr_design::fins::{FinCrossSection, FinPlanform, FinSet, TubeFinSet};
use hpr_design::parts::{
    BodyTube, CenteringRing, InnerTube, LaunchLug, NoseCone, Packing, PodSet, RailButton, Shoulder,
    Transition,
};
use hpr_design::shapes::NoseShape;
use hpr_design::solids::Wall;
use hpr_design::tree::{
    AutoDimension, Component, Overrides, Part, Position, ReferenceDiameter, Stage,
};
use hpr_design::{Density, Finish, Material};

use super::super::Design;
use super::super::attached::cluster_pattern;
use super::super::document::Element;
use super::super::extensions::{OpenRocketExtension, tag_key};
use super::super::value::Dimension;
use super::super::warning::{Warning, WarningKind};
use super::xml::{self, Build as _};
use super::{motors, recovery};

/// The path of the rocket element, which every part's path starts with.
const ROCKET: &str = "openrocket/rocket";

/// The `<rocket>` element for `design`.
pub(super) fn rocket(design: &Design, warnings: &mut Vec<Warning>) -> Element {
    Writer::new(design, warnings).rocket()
}

/// Writes the rocket, knowing where the kept parts go and which part tags the file had.
struct Writer<'a> {
    /// The design written.
    design: &'a Design,
    /// What the design keeps of the file it was read from.
    kept: &'a OpenRocketExtension,
    /// The path of every element something kept lies in or is: the parts' own paths, and every
    /// part above a kept part, tag or attribute. A part written at one of these is the tag its
    /// path names.
    known: BTreeSet<String>,
    /// Where what cannot be written so it reads back the same is said.
    warnings: &'a mut Vec<Warning>,
}

impl<'a> Writer<'a> {
    /// A writer for `design`, with every element path its kept items lie under.
    fn new(design: &'a Design, warnings: &'a mut Vec<Warning>) -> Self {
        let kept = &design.extensions.x_openrocket;
        let mut known = BTreeSet::new();
        let paths = kept
            .parts
            .iter()
            .chain(&kept.tags)
            .map(|kept| kept.at.as_str())
            .chain(kept.attributes.iter().map(|kept| kept.at.as_str()));
        for at in paths {
            let Some(rest) = at.strip_prefix(ROCKET).and_then(|r| r.strip_prefix('/')) else {
                continue;
            };
            let mut path = ROCKET.to_owned();
            for step in rest.split('/').take_while(|step| !step.starts_with('@')) {
                path.push('/');
                path.push_str(step);
                known.insert(path.clone());
            }
        }
        Self {
            design,
            kept,
            known,
            warnings,
        }
    }

    /// Records that something in the design could not be written so it reads back the same.
    fn warn(&mut self, at: &str, message: impl Into<String>) {
        self.warnings
            .push(Warning::new(at, WarningKind::Dropped, message.into()));
    }

    /// The places among the parts of the element at `parent` that `count` written parts take, in
    /// order: every place but the kept parts'.
    fn places(&self, parent: &str, count: usize) -> Vec<usize> {
        let taken: BTreeSet<usize> = self
            .kept
            .parts
            .iter()
            .filter_map(|part| {
                let (above, last) = part.at.rsplit_once('/')?;
                if above != parent {
                    return None;
                }
                let (_, rest) = last.split_once('[')?;
                rest.strip_suffix(']')?.parse().ok()
            })
            .collect();
        (0..)
            .filter(|place| !taken.contains(place))
            .take(count)
            .collect()
    }

    /// Writes `id` as the `<tag>` element's `<id>`, at `at`, if it is a UUID, the only kind
    /// OpenRocket 24.12 opens a file with: it refuses the whole file over one that is not. Any
    /// other id is left out, and OpenRocket gives the part one of its own. The reader invents an
    /// id for a part that has none (`bodytube-3`), and reading the written file invents the same
    /// one again, since it counts the parts in the same order; nor is anything lost when the
    /// design keeps the file's own `<id>` there, as it does for a part whose id another part
    /// already had. Any other id left out is warned of: the part reads back under another.
    fn id(&mut self, element: &mut Element, at: &str, tag: &str, id: &str) {
        if is_uuid(id) {
            element.leaf("id", id.to_owned());
        } else if !invented(id, tag) && !self.keeps_tag(at, "id") {
            self.warn(
                at,
                format!(
                    "the id `{id}` is not a UUID, which OpenRocket 24.12 refuses a file over; it \
                     was left out, and the part will read back under another"
                ),
            );
        }
    }

    /// Whether what the design keeps holds a `<name>` tag of the element at `at` itself.
    fn keeps_tag(&self, at: &str, name: &str) -> bool {
        let prefix = format!("{at}/@{name}[");
        self.kept.tags.iter().any(|kept| {
            kept.at
                .strip_prefix(&prefix)
                .is_some_and(|rest| !rest.contains('/'))
        })
    }

    /// Whether what the design keeps says the part at `place` in `parent` was a `<name>`.
    fn was(&self, parent: &str, place: usize, name: &str) -> bool {
        self.known.contains(&format!("{parent}/{name}[{place}]"))
    }

    /// Whether `component` was read from a `<name>`: the design keeps something at that path, or
    /// its id is the one the reader invents for a `<name>` with none (`engineblock-7`).
    fn was_tag(&self, component: &Component, parent: &str, place: usize, name: &str) -> bool {
        self.was(parent, place, name) || invented(&component.id, name)
    }

    /// The `<rocket>`: its name, how its reference diameter is chosen, the motors' tags, and its
    /// stages.
    fn rocket(&mut self) -> Element {
        let design = self.design;
        let rocket = &design.rocket;
        let mut element = xml::element("rocket");
        element.leaf("name", rocket.name.clone());
        match rocket.reference_diameter {
            ReferenceDiameter::Maximum {} => {
                element.leaf("referencetype", "maximum");
            }
            _ => self.warn(
                ROCKET,
                "a reference diameter chosen other than by the widest body component, which a \
                 `.ork` is read only as; OpenRocket's default, the widest, was written",
            ),
        }
        self.copies(&mut element, ROCKET);
        for tag in motors::rocket(design) {
            element.push(tag);
        }
        let mut stages = xml::element("subcomponents");
        let places = self.places(ROCKET, rocket.stages.len());
        for ((index, stage), place) in rocket.stages.iter().enumerate().zip(places) {
            let at = format!("{ROCKET}/stage[{place}]");
            stages.push(self.stage(index, stage, &at));
        }
        element.push(stages);
        element
    }

    /// Stage `index`, at path `at`: its name, id and overrides, when it separates, and its body
    /// components.
    fn stage(&mut self, index: usize, stage: &Stage, at: &str) -> Element {
        let design = self.design;
        let mut element = xml::element("stage");
        element.leaf("name", stage.name.clone());
        self.id(&mut element, at, "stage", &stage.id);
        // A stage's override is the whole stage's, so it covers everything inside it.
        let covers = !stage.overrides.is_empty();
        self.overrides(&mut element, at, &stage.overrides, covers);
        self.copies(&mut element, at);
        for tag in recovery::stage(design, index, stage) {
            element.push(tag);
        }
        let components = self.parts(at, &stage.components, Holder::Stack);
        element.push(components);
        element
    }

    /// The `<subcomponents>` holding `components`, the parts of the element at `at`.
    fn parts(&mut self, at: &str, components: &[Component], holder: Holder) -> Element {
        let mut element = xml::element("subcomponents");
        let places = self.places(at, components.len());
        for (component, place) in components.iter().zip(places) {
            let name = self.tag(component, at, place);
            let path = format!("{at}/{name}[{place}]");
            let body = matches!(
                component.part,
                Part::NoseCone(_) | Part::BodyTube(_) | Part::Transition(_)
            );
            if body != (holder == Holder::Stack) {
                let (what, where_) = match holder {
                    Holder::Stack => ("a part that is not a body component", "a stage or a pod"),
                    Holder::Tube => ("a body component", "another part"),
                };
                self.warn(
                    &path,
                    format!(
                        "{what} sits in {where_}, where a `.ork` reader does not read one; it was \
                         written, and will read back left out"
                    ),
                );
            }
            element.push(self.component(component, name, &path));
        }
        element
    }

    /// The tag `component` is written as, at `place` among the parts of `parent`.
    fn tag(&self, component: &Component, parent: &str, place: usize) -> &'static str {
        let auto = |dimension| component.auto.contains(&dimension);
        match &component.part {
            Part::NoseCone(_) => "nosecone",
            Part::BodyTube(_) => "bodytube",
            Part::Transition(transition) => {
                if self.was_tag(component, parent, place, "nosecone")
                    && turned_nose(component, transition)
                {
                    "nosecone"
                } else {
                    "transition"
                }
            }
            Part::InnerTube(_) => {
                // An `innertube` reads an automatic outer radius as a fixed one, so one that is
                // automatic is written as a coupler, which keeps it.
                let automatic = auto(AutoDimension::OuterRadius);
                ["innertube", "tubecoupler", "engineblock"]
                    .into_iter()
                    .find(|name| {
                        self.was_tag(component, parent, place, name)
                            && !(*name == "innertube" && automatic)
                    })
                    .unwrap_or(if automatic {
                        "tubecoupler"
                    } else {
                        "innertube"
                    })
            }
            Part::CenteringRing(ring) => {
                let bulkhead = ring.inner_radius_m == 0.0 && !auto(AutoDimension::InnerRadius);
                if self.was_tag(component, parent, place, "centeringring") || !bulkhead {
                    "centeringring"
                } else {
                    "bulkhead"
                }
            }
            Part::FinSet(fins) => match fins.planform {
                FinPlanform::Elliptical { .. } => "ellipticalfinset",
                FinPlanform::Freeform { .. } => "freeformfinset",
                _ => "trapezoidfinset",
            },
            Part::TubeFinSet(_) => "tubefinset",
            Part::LaunchLug(_) => "launchlug",
            Part::RailButton(_) => "railbutton",
            Part::PodSet(_) => "podset",
            Part::Parachute(_) => "parachute",
            Part::Streamer(_) => "streamer",
            Part::ShockCord(_) => "shockcord",
            Part::MassComponent(_) => "masscomponent",
            // A kind added to `hpr-design` after this writer: `component` warns of it, and a mass
            // component is the tag that claims least about a part.
            _ => "masscomponent",
        }
    }

    /// The element for one component, everything on and inside it included.
    fn component(&mut self, component: &Component, name: &str, at: &str) -> Element {
        let design = self.design;
        let mut element = xml::element(name);
        element.leaf("name", component.name.clone());
        self.id(&mut element, at, name, &component.id);
        let body = matches!(
            component.part,
            Part::NoseCone(_) | Part::BodyTube(_) | Part::Transition(_)
        );
        match (body, component.position) {
            (true, None) => {}
            (true, Some(_)) => self.warn(
                at,
                "a body component placed by an offset, where a `.ork` stacks them; the offset was \
                 left out",
            ),
            (false, Some(position)) => {
                let position = match &component.part {
                    Part::RailButton(button) => self.button_position(at, position, button),
                    _ => position,
                };
                axial_offset(&mut element, position);
            }
            (false, None) => self.warn(
                at,
                "a part with no position, which a `.ork` reader reads flush with its parent's \
                 forward end; none was written",
            ),
        }
        let covers = component.overrides_include_children;
        match &component.part {
            Part::NoseCone(nose) => self.nose_cone(&mut element, at, component, nose),
            Part::BodyTube(tube) => self.body_tube(&mut element, at, component, tube),
            Part::Transition(transition) if name == "nosecone" => {
                self.turned_nose_cone(&mut element, at, component, transition);
            }
            Part::Transition(transition) => {
                self.transition(&mut element, at, component, transition);
            }
            Part::InnerTube(tube) => self.inner_tube(&mut element, at, component, tube),
            Part::CenteringRing(ring) => self.ring(&mut element, at, component, ring, name),
            Part::FinSet(fins) => self.fin_set(&mut element, at, fins),
            Part::TubeFinSet(fins) => self.tube_fins(&mut element, at, component, fins),
            Part::LaunchLug(lug) => self.launch_lug(&mut element, at, lug),
            Part::RailButton(button) => self.rail_button(&mut element, at, button),
            Part::PodSet(pods) => self.pod_set(&mut element, at, pods),
            Part::MassComponent(mass) => {
                element.number("mass", mass.mass_kg);
                self.packing(&mut element, at, component, &mass.packing);
            }
            Part::Parachute(chute) => {
                element.number("diameter", chute.diameter_m);
                self.material(
                    &mut element,
                    at,
                    "material",
                    &chute.canopy_material,
                    "surface",
                );
                element
                    .leaf("linecount", chute.line_count.to_string())
                    .number("linelength", chute.line_length_m);
                self.material(
                    &mut element,
                    at,
                    "linematerial",
                    &chute.line_material,
                    "line",
                );
                self.packing(&mut element, at, component, &chute.packing);
            }
            Part::Streamer(streamer) => {
                element
                    .number("striplength", streamer.length_m)
                    .number("stripwidth", streamer.width_m);
                self.material(&mut element, at, "material", &streamer.material, "surface");
                self.packing(&mut element, at, component, &streamer.packing);
            }
            Part::ShockCord(cord) => {
                element.number("cordlength", cord.length_m);
                self.material(&mut element, at, "material", &cord.material, "line");
                self.packing(&mut element, at, component, &cord.packing);
            }
            other => {
                let kind = other.kind_name();
                self.warn(
                    at,
                    format!("a part of kind `{kind}`, which a `.ork` has no tag for; only its name was written"),
                );
            }
        }
        self.overrides(&mut element, at, &component.overrides, covers);
        self.finish(&mut element, at, component.finish);
        self.copies(&mut element, at);
        if let Some(mount) = motors::mount(design, component) {
            element.push(mount);
        }
        for tag in recovery::device(design, component) {
            element.push(tag);
        }
        if !component.children.is_empty() {
            let holder = if matches!(component.part, Part::PodSet(_)) {
                Holder::Stack
            } else {
                Holder::Tube
            };
            let parts = self.parts(at, &component.children, holder);
            element.push(parts);
        }
        element
    }

    /// A body tube's length, radius, wall and material.
    fn body_tube(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        tube: &BodyTube,
    ) {
        element.number("length", tube.length_m).leaf(
            "radius",
            dimension(component, AutoDimension::OuterRadius, tube.outer_radius_m),
        );
        // A wall as thick as the tube is the tube filled, which OpenRocket writes `filled`. With a
        // stated radius the number would read back the same; with an automatic one the reader
        // gives a filled tube the radius it cached, which is how a wall this thick came to be.
        if tube.thickness_m == tube.outer_radius_m && tube.outer_radius_m > 0.0 {
            element.leaf("thickness", "filled");
        } else {
            element.number("thickness", tube.thickness_m);
        }
        self.material(element, at, "material", &tube.material, "bulk");
    }

    /// A centering ring, or a bulkhead, which has no bore to write.
    fn ring(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        ring: &CenteringRing,
        name: &str,
    ) {
        element.number("length", ring.length_m).leaf(
            "outerradius",
            dimension(component, AutoDimension::OuterRadius, ring.outer_radius_m),
        );
        if name == "centeringring" {
            element.leaf(
                "innerradius",
                dimension(component, AutoDimension::InnerRadius, ring.inner_radius_m),
            );
        }
        self.material(element, at, "material", &ring.material, "bulk");
    }

    /// A nose cone's length, base radius, wall, shape, shoulder and material.
    fn nose_cone(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        nose: &NoseCone,
    ) {
        element.number("length", nose.length_m).leaf(
            "aftradius",
            dimension(component, AutoDimension::BaseRadius, nose.base_radius_m),
        );
        wall(element, nose.wall);
        self.shape(element, at, nose.shape);
        shoulder(
            element,
            component,
            "aft",
            nose.shoulder.as_ref(),
            AutoDimension::ShoulderRadius,
        );
        self.material(element, at, "material", &nose.material, "bulk");
        element.flag("isflipped", false);
    }

    /// A nose cone turned round, which the design holds as a transition from its base to a point.
    fn turned_nose_cone(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        transition: &Transition,
    ) {
        element.number("length", transition.length_m).leaf(
            "aftradius",
            dimension(
                component,
                AutoDimension::ForeRadius,
                transition.fore_radius_m,
            ),
        );
        wall(element, transition.wall);
        self.shape(element, at, transition.shape);
        shoulder(
            element,
            component,
            "aft",
            transition.fore_shoulder.as_ref(),
            AutoDimension::ForeShoulderRadius,
        );
        self.material(element, at, "material", &transition.material, "bulk");
        element.flag("isflipped", true);
    }

    /// A transition's length, two radii, wall, shape, clipping, shoulders and material.
    fn transition(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        transition: &Transition,
    ) {
        element
            .number("length", transition.length_m)
            .leaf(
                "foreradius",
                dimension(
                    component,
                    AutoDimension::ForeRadius,
                    transition.fore_radius_m,
                ),
            )
            .leaf(
                "aftradius",
                dimension(component, AutoDimension::AftRadius, transition.aft_radius_m),
            );
        wall(element, transition.wall);
        self.shape(element, at, transition.shape);
        element.flag("shapeclipped", transition.clipped);
        shoulder(
            element,
            component,
            "fore",
            transition.fore_shoulder.as_ref(),
            AutoDimension::ForeShoulderRadius,
        );
        shoulder(
            element,
            component,
            "aft",
            transition.aft_shoulder.as_ref(),
            AutoDimension::AftShoulderRadius,
        );
        self.material(element, at, "material", &transition.material, "bulk");
    }

    /// An inner tube, tube coupler or engine block: its length, radius, wall, place off the axis,
    /// material and cluster.
    fn inner_tube(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        tube: &InnerTube,
    ) {
        element
            .number("length", tube.length_m)
            .leaf(
                "outerradius",
                dimension(component, AutoDimension::OuterRadius, tube.outer_radius_m),
            )
            .number("thickness", tube.thickness_m)
            .number("radialposition", tube.radial_offset_m);
        // OpenRocket 24.12 reads an inner tube's roll angle only under its older name.
        self.older_angle(element, at, tube.angle_rad);
        self.material(element, at, "material", &tube.material, "bulk");
        if tube.cluster_m.is_empty() {
            return;
        }
        match cluster_written(tube) {
            Some((name, scale, rotation_deg)) => {
                element
                    .leaf("clusterconfiguration", name)
                    .number("clusterscale", scale)
                    .number("clusterrotation", rotation_deg);
            }
            None => self.warn(
                at,
                "a clustered tube whose places are not one of OpenRocket's patterns at any scale \
                 and rotation that reads back as the same places; it was written as one tube",
            ),
        }
    }

    /// A fin set: its count, roll angle, material, section, cant, fillet, outline and tab.
    fn fin_set(&mut self, element: &mut Element, at: &str, fins: &FinSet) {
        element
            .leaf("instancecount", fins.count.to_string())
            .number("radiusoffset", 0.0);
        self.angle(element, at, "angleoffset", fins.base_angle_rad);
        self.material(element, at, "material", &fins.material, "bulk");
        element.number("thickness", fins.thickness_m).leaf(
            "crosssection",
            match fins.cross_section {
                FinCrossSection::Rounded => "rounded",
                FinCrossSection::Airfoil => "airfoil",
                _ => "square",
            },
        );
        self.angle(element, at, "cant", fins.cant_rad);
        if let Some(fillet) = &fins.fillet {
            if fillet.radius_m > 0.0 {
                element.number("filletradius", fillet.radius_m);
                self.material(element, at, "filletmaterial", &fillet.material, "bulk");
            } else {
                self.warn(
                    at,
                    "a fin fillet of no radius, which reads back as none; it was left out",
                );
            }
        }
        match &fins.planform {
            FinPlanform::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                sweep_m,
            } => {
                element
                    .number("rootchord", *root_chord_m)
                    .number("tipchord", *tip_chord_m)
                    .number("sweeplength", *sweep_m)
                    .number("height", *span_m);
            }
            FinPlanform::Elliptical {
                root_chord_m,
                span_m,
            } => {
                element
                    .number("rootchord", *root_chord_m)
                    .number("height", *span_m);
            }
            FinPlanform::Freeform { points_m } => {
                let mut points = xml::element("finpoints");
                for [x, h] in points_m {
                    let mut point = xml::element("point");
                    point
                        .with_attribute("x", xml::number(*x))
                        .with_attribute("y", xml::number(*h));
                    points.push(point);
                }
                element.push(points);
            }
            _ => self.warn(
                at,
                "a fin outline a `.ork` has no tag for; none was written",
            ),
        }
        if let Some(tab) = &fins.tab {
            if tab.height_m <= 0.0 || tab.length_m <= 0.0 {
                self.warn(
                    at,
                    "a fin tab of no height or length, which reads back as none; it was left out",
                );
                return;
            }
            let root_m = fins.planform.root_chord_m();
            // OpenRocket writes the tab's place twice, and what the design keeps of the second
            // copy says which end the file measured from: that end is written if the offset from
            // it reads back exactly, and otherwise the root's leading edge, which always does.
            let from = self.kept.attributes.iter().find_map(|kept| {
                (kept.at == format!("{at}/@tabposition[1]")
                    && (kept.name == "relativeto" || kept.name == "type"))
                    .then_some(kept.value.as_str())
            });
            let base_m = root_m - tab.length_m;
            let (frame, offset_m) = match from {
                Some("middle" | "center") => (
                    "center",
                    preimage(tab.offset_m, tab.offset_m - 0.5 * base_m, |o| {
                        0.5 * base_m + o
                    }),
                ),
                Some("bottom" | "end") => (
                    "end",
                    preimage(tab.offset_m, tab.offset_m - base_m, |o| base_m + o),
                ),
                _ => ("front", Some(tab.offset_m)),
            };
            let (frame, offset_m) = match offset_m {
                Some(offset_m) => (frame, offset_m),
                None => ("front", tab.offset_m),
            };
            let mut position = xml::tag("tabposition", xml::number(offset_m));
            position.with_attribute("relativeto", frame);
            element
                .number("tabheight", tab.height_m)
                .number("tablength", tab.length_m)
                .push(position);
        }
    }

    /// A ring of tube fins: its count, roll angle, material, length, radius and wall.
    fn tube_fins(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        fins: &TubeFinSet,
    ) {
        element
            .leaf("instancecount", fins.count.to_string())
            .number("radiusoffset", 0.0);
        self.angle(element, at, "angleoffset", fins.base_angle_rad);
        self.material(element, at, "material", &fins.material, "bulk");
        element
            .number("length", fins.length_m)
            .leaf(
                "radius",
                dimension(component, AutoDimension::OuterRadius, fins.outer_radius_m),
            )
            .number("thickness", fins.thickness_m);
    }

    /// A launch lug or a row of them.
    fn launch_lug(&mut self, element: &mut Element, at: &str, lug: &LaunchLug) {
        element
            .leaf("instancecount", lug.count.to_string())
            .number("instanceseparation", lug.spacing_m);
        self.angle(element, at, "angleoffset", lug.angle_rad);
        self.material(element, at, "material", &lug.material, "bulk");
        element
            .number("radius", lug.outer_radius_m)
            .number("length", lug.length_m)
            .number("thickness", lug.thickness_m);
    }

    /// A rail button or a row of them. Its offset is [`Self::button_position`]'s.
    fn rail_button(&mut self, element: &mut Element, at: &str, button: &RailButton) {
        element
            .leaf("instancecount", button.count.to_string())
            .number("instanceseparation", button.spacing_m);
        self.angle(element, at, "angleoffset", button.angle_rad);
        self.material(element, at, "material", &button.material, "bulk");
        element
            .number("outerdiameter", button.outer_diameter_m)
            .number("innerdiameter", button.inner_diameter_m)
            .number("height", button.height_m)
            .number("baseheight", button.base_height_m)
            .number("flangeheight", button.flange_height_m);
    }

    /// A pod set: its pods' distance from the axis is written as the distance itself
    /// (`method="free"`), which is the number the design holds, where any other method would be
    /// read back through the tube's and the pod's radii.
    fn pod_set(&mut self, element: &mut Element, at: &str, pods: &PodSet) {
        let mut offset = xml::tag("radiusoffset", xml::number(pods.radial_offset_m));
        offset.with_attribute("method", "free");
        element
            .leaf("instancecount", pods.count.to_string())
            .push(offset);
        self.angle(element, at, "angleoffset", pods.angle_rad);
    }

    /// How a mass component or recovery part is packed.
    fn packing(
        &mut self,
        element: &mut Element,
        at: &str,
        component: &Component,
        packing: &Packing,
    ) {
        element
            .number("packedlength", packing.length_m)
            .leaf(
                "packedradius",
                dimension(component, AutoDimension::PackedRadius, packing.radius_m),
            )
            .number("radialposition", packing.radial_offset_m);
        // OpenRocket 24.12 reads a packed part's roll angle only under its older name.
        self.older_angle(element, at, packing.angle_rad);
    }

    /// The roll angle of a part OpenRocket 24.12 reads it on only as `radialdirection`: an inner
    /// tube, coupler or engine block, a mass component or a recovery part.
    ///
    /// When what the design keeps holds an older name for the angle at `at` — the file gave it
    /// as `angleoffset` and, under `radialdirection` or `rotation`, as another angle, and the
    /// reader took `angleoffset` — the design's angle is written as `angleoffset` instead, and the
    /// kept older names go back beside it, so the file says what it said: reading the export takes
    /// the design's angle and warns of the other again, and OpenRocket reads the angle it read
    /// before. OpenRocket 24.12 writes no `angleoffset` on these parts, and warns that it ignores
    /// one, so it is written only then.
    fn older_angle(&mut self, element: &mut Element, at: &str, radians: f64) {
        let kept_older = ["radialdirection", "rotation"]
            .iter()
            .any(|name| self.keeps_tag(at, name));
        let name = if kept_older {
            "angleoffset"
        } else {
            "radialdirection"
        };
        self.angle(element, at, name, radians);
    }

    /// An angle in radians, written as the degrees a `.ork` reader turns into exactly it.
    fn angle(&mut self, element: &mut Element, at: &str, name: &str, radians: f64) {
        let degrees = preimage(radians, radians.to_degrees(), f64::to_radians).unwrap_or_else(|| {
            self.warn(
                at,
                format!("an angle of {radians} rad that no number of degrees reads back as exactly; the nearest was written"),
            );
            radians.to_degrees()
        });
        element.number(name, degrees);
    }

    /// Where a rail button's offset must say it is for the reader, which moves it by the button's
    /// radius and its row's length, to put it back where the design has it.
    fn button_position(&mut self, at: &str, position: Position, button: &RailButton) -> Position {
        let radius_m = 0.5 * button.outer_diameter_m;
        let row_m = button.spacing_m * f64::from(button.count.saturating_sub(1));
        let found = match position {
            Position::Top { aft_offset_m } => {
                preimage(aft_offset_m, aft_offset_m + radius_m, |a| a - radius_m)
                    .map(|aft_offset_m| Position::Top { aft_offset_m })
            }
            Position::Middle { aft_offset_m } => {
                preimage(aft_offset_m, aft_offset_m - 0.5 * row_m, |a| {
                    a + 0.5 * row_m
                })
                .map(|aft_offset_m| Position::Middle { aft_offset_m })
            }
            Position::Bottom { aft_offset_m } => {
                preimage(aft_offset_m, aft_offset_m - row_m - radius_m, |a| {
                    a + radius_m + row_m
                })
                .map(|aft_offset_m| Position::Bottom { aft_offset_m })
            }
            Position::After { aft_offset_m } => {
                preimage(aft_offset_m, aft_offset_m + radius_m, |a| a - radius_m)
                    .map(|aft_offset_m| Position::After { aft_offset_m })
            }
            Position::Absolute { station_m } => {
                preimage(station_m, station_m + radius_m, |a| a - radius_m)
                    .map(|station_m| Position::Absolute { station_m })
            }
        };
        found.unwrap_or_else(|| {
            self.warn(
                at,
                "a rail button whose offset no number reads back as exactly; the nearest was \
                 written",
            );
            position
        })
    }

    /// The profile's name and its parameter, as the reader reads them into `shape`.
    fn shape(&mut self, element: &mut Element, at: &str, shape: NoseShape) {
        match shape {
            NoseShape::Conical {} => {
                element.leaf("shape", "conical");
            }
            NoseShape::Elliptical {} => {
                element.leaf("shape", "ellipsoid");
            }
            // The reader takes the reciprocal of OpenRocket's parameter.
            NoseShape::Ogive { radius_ratio } => {
                let kappa = preimage(radius_ratio, 1.0 / radius_ratio, |kappa| 1.0 / kappa)
                    .unwrap_or_else(|| {
                        self.warn(
                            at,
                            format!("an ogive radius ratio of {radius_ratio} that no shape parameter reads back as exactly; the nearest was written"),
                        );
                        1.0 / radius_ratio
                    });
                element
                    .leaf("shape", "ogive")
                    .number("shapeparameter", kappa);
            }
            NoseShape::PowerSeries { exponent } => {
                element
                    .leaf("shape", "power")
                    .number("shapeparameter", exponent);
            }
            NoseShape::ParabolicSeries { parameter } => {
                element
                    .leaf("shape", "parabolic")
                    .number("shapeparameter", parameter);
            }
            NoseShape::Haack { parameter } => {
                element
                    .leaf("shape", "haack")
                    .number("shapeparameter", parameter);
            }
            _ => {
                self.warn(
                    at,
                    "a nose shape a `.ork` has no name for; a cone was written",
                );
                element.leaf("shape", "conical");
            }
        }
    }

    /// A material as `<name type="…" density="…">Name</name>`.
    fn material(
        &mut self,
        element: &mut Element,
        at: &str,
        name: &str,
        material: &Material,
        want: &str,
    ) {
        let (kind, density) = match material.density {
            Density::Bulk { kg_m3 } => ("bulk", kg_m3),
            Density::Surface { kg_m2 } => ("surface", kg_m2),
            Density::Line { kg_m } => ("line", kg_m),
        };
        if kind != want {
            self.warn(
                at,
                format!(
                    "the material `{}` has a {kind} density where the part needs a {want} one; \
                     it reads back as a {want} density of the same number",
                    material.name
                ),
            );
        }
        let mut tag = xml::tag(name, material.name.clone());
        tag.with_attribute("type", kind)
            .with_attribute("density", xml::number(density));
        element.push(tag);
    }

    /// The overrides and whether they cover the parts inside, as the reader puts them back
    /// together: the mass flag decides, unless only the centre of gravity is overridden.
    ///
    /// Where the reader kept the file's flags as written, all together — flags that disagree, a
    /// flag written twice or unreadable, the single older flag beside a drag override — no flag is
    /// written: the kept ones go back as the file had them, and a flag written beside them that
    /// the file did not have would change what they say. A kept drag flag alone is not one of
    /// those; the reader keeps it apart from the others.
    fn overrides(&mut self, element: &mut Element, at: &str, overrides: &Overrides, covers: bool) {
        element
            .maybe_number("overridemass", overrides.mass_kg)
            .maybe_number("overridecg", overrides.cg_aft_m);
        let kept_flags = [
            "overridesubcomponents",
            "overridesubcomponentsmass",
            "overridesubcomponentscg",
        ]
        .iter()
        .any(|name| self.keeps_tag(at, name));
        if !kept_flags && (overrides.mass_kg.is_some() || (overrides.cg_aft_m.is_none() && covers))
        {
            element.flag("overridesubcomponentsmass", covers);
        }
        if !kept_flags && overrides.cg_aft_m.is_some() {
            element.flag("overridesubcomponentscg", covers);
        }
        if overrides.cg_xy_m.is_some() || overrides.inertia.is_some() {
            self.warn(
                at,
                "an override of the centre of gravity off the axis or of the inertia, which a \
                 `.ork` has no tag for; it was left out",
            );
        }
    }

    /// The finish, by the word the reader turns into its roughness.
    fn finish(&mut self, element: &mut Element, at: &str, finish: Option<Finish>) {
        let Some(finish) = finish else {
            return;
        };
        let word = match finish {
            Finish::Custom { roughness_m } => [
                ("rough", 500e-6),
                ("unfinished", 150e-6),
                ("normal", 60e-6),
                ("smooth", 20e-6),
                ("polished", 2e-6),
            ]
            .into_iter()
            .find_map(|(word, roughness)| (roughness == roughness_m).then_some(word)),
            _ => None,
        };
        match word {
            Some(word) => {
                element.leaf("finish", word);
            }
            None => self.warn(
                at,
                "a surface finish that is none of OpenRocket's five; it was left out",
            ),
        }
    }

    /// Writes a second copy of a tag where what the design keeps of the file lies in one: a tag
    /// OpenRocket writes twice, such as a fin tab's place, has its second copy's attributes kept,
    /// and they need it there to go back to. The copy carries the first one's text, as
    /// OpenRocket's does.
    ///
    /// A tag that belongs to one configuration is written once for it by its own writer, and is
    /// not copied. A count is only believed as far as there could be copies to fill: the tags
    /// written here and everything kept in this element, one copy each. A design edited or built
    /// by hand could say `@length[2000000]`; what is kept there then has no place, and the splice
    /// warns of it, rather than two million copies being written.
    fn copies(&self, element: &mut Element, at: &str) {
        let prefix = format!("{at}/@");
        let paths: Vec<&str> = self
            .kept
            .tags
            .iter()
            .map(|kept| kept.at.as_str())
            .chain(self.kept.attributes.iter().map(|kept| kept.at.as_str()))
            .filter_map(|path| path.strip_prefix(&prefix))
            .collect();
        let most = element.elements().count().saturating_add(paths.len());
        let steps: Vec<(String, usize)> = paths
            .iter()
            .filter_map(|rest| {
                let step = rest.split('/').next()?;
                let (name, index) = step.split_once('[')?;
                Some((name.to_owned(), index.strip_suffix(']')?.parse().ok()?))
            })
            .filter(|(name, index)| !name.contains('(') && *index < most)
            .collect();
        for (name, index) in steps {
            let plain = |child: &&Element| tag_key(child) == name;
            let have = element.elements().filter(plain).count();
            let Some(text) = element.elements().find(plain).map(Element::text) else {
                continue;
            };
            for _ in have..=index {
                element.leaf(&name, text.clone());
            }
        }
    }
}

/// Which parts an element holds: the body components of a stage or a pod, or the parts on and
/// inside a body component or a tube.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Holder {
    /// A stage or a pod: nose cones, body tubes and transitions.
    Stack,
    /// A body component or an inner tube: everything else.
    Tube,
}

/// Whether `transition` is what the reader makes of a nose cone turned round: its aft end a
/// stated point, unclipped, with no aft shoulder.
fn turned_nose(component: &Component, transition: &Transition) -> bool {
    transition.aft_radius_m == 0.0
        && !component.auto.contains(&AutoDimension::AftRadius)
        && !transition.clipped
        && transition.aft_shoulder.is_none()
}

/// A nose cone's or transition's wall.
fn wall(element: &mut Element, wall: Wall) {
    match wall {
        Wall::Filled {} => {
            element.leaf("thickness", "filled");
        }
        Wall::Shell { thickness_m } => {
            element.number("thickness", thickness_m);
        }
    }
}

/// A shoulder at one end, when there is one.
fn shoulder(
    element: &mut Element,
    component: &Component,
    end: &str,
    shoulder: Option<&Shoulder>,
    which: AutoDimension,
) {
    let Some(shoulder) = shoulder else {
        return;
    };
    element
        .leaf(
            &format!("{end}shoulderradius"),
            dimension(component, which, shoulder.outer_radius_m),
        )
        .number(&format!("{end}shoulderlength"), shoulder.length_m)
        .number(&format!("{end}shoulderthickness"), shoulder.thickness_m);
    // The reader asks whether a shoulder is capped only when a cap could be there: a wall, and a
    // bore inside it when the radius is stated. Written otherwise, the tag would come back as one
    // the reader never asked for; and a cap there reads back as none whatever is written.
    let automatic = component.auto.contains(&which);
    if shoulder.thickness_m > 0.0 && (automatic || shoulder.thickness_m < shoulder.outer_radius_m) {
        element.flag(&format!("{end}shouldercapped"), shoulder.capped);
    }
}

/// Where a part sits along its parent, under the newer name with the end it is measured from.
fn axial_offset(element: &mut Element, position: Position) {
    let (method, value) = match position {
        Position::Top { aft_offset_m } => ("top", aft_offset_m),
        Position::Middle { aft_offset_m } => ("middle", aft_offset_m),
        Position::Bottom { aft_offset_m } => ("bottom", aft_offset_m),
        Position::After { aft_offset_m } => ("after", aft_offset_m),
        Position::Absolute { station_m } => ("absolute", station_m),
    };
    let mut tag = xml::tag("axialoffset", xml::number(value));
    tag.with_attribute("method", method);
    element.push(tag);
}

/// A dimension's text: the number, or `auto` with the number the reader reads back from it when
/// the component works it out for itself. A bare `auto` reads back as zero.
fn dimension(component: &Component, which: AutoDimension, value: f64) -> String {
    if !component.auto.contains(&which) {
        return xml::number(value);
    }
    let cached = (value.to_bits() != 0.0_f64.to_bits()).then_some(value);
    xml::dimension(Dimension::Automatic { cached })
}

/// The cluster pattern, scale and rotation in degrees that the reader turns into exactly `tube`'s
/// places, or `None` when there is none.
///
/// The reader puts pattern point `p` at `2 R s · Rot(θ − ρ) · p`. The scale `s` and rotation `ρ`
/// are estimated from the farthest point, and the shortest decimals near them tried until every
/// place comes back bit for bit.
fn cluster_written(tube: &InnerTube) -> Option<(&'static str, f64, f64)> {
    const NAMES: [&str; 13] = [
        "double", "3-row", "3-ring", "3-star", "4-row", "4-ring", "4-star", "5-ring", "5-star",
        "6-ring", "6-star", "9-grid", "9-star",
    ];
    let places = &tube.cluster_m;
    let (radius_m, angle_rad) = (tube.outer_radius_m, tube.angle_rad);
    let placed = |points: &[[f64; 2]], scale: f64, rotation_deg: f64| {
        let separation_m = 2.0 * radius_m * scale;
        let (sin, cos) = (angle_rad - rotation_deg.to_radians()).sin_cos();
        points.len() == places.len()
            && points.iter().zip(places).all(|(&[u, v], &[x, y])| {
                separation_m * (u * cos - v * sin) == x && separation_m * (u * sin + v * cos) == y
            })
    };
    for name in NAMES {
        let Some(points) = cluster_pattern(name) else {
            continue;
        };
        if points.len() != places.len() {
            continue;
        }
        if placed(&points, 1.0, 0.0) {
            return Some((name, 1.0, 0.0));
        }
        // The farthest point from the pattern's centre says the most about both numbers.
        let Some(far) = (0..points.len()).max_by(|&a, &b| {
            let (pa, pb) = (points[a], points[b]);
            pa[0].hypot(pa[1]).total_cmp(&pb[0].hypot(pb[1]))
        }) else {
            continue;
        };
        let ([u, v], [x, y]) = (points[far], places[far]);
        let reach = u.hypot(v);
        if reach == 0.0 || radius_m == 0.0 {
            continue;
        }
        let scale = x.hypot(y) / reach / (2.0 * radius_m);
        let rotation_deg = (angle_rad - (y.atan2(x) - v.atan2(u))).to_degrees();
        for rotation_deg in [rotation_deg, rotation_deg - 360.0, rotation_deg + 360.0] {
            for scale_digits in 1..=17 {
                let scale = shortest(scale, scale_digits);
                for rotation_digits in 1..=17 {
                    let rotation_deg = shortest(rotation_deg, rotation_digits);
                    if placed(&points, scale, rotation_deg) {
                        return Some((name, scale, rotation_deg));
                    }
                }
            }
        }
    }
    None
}

/// `value` rounded to `digits` significant decimal digits.
fn shortest(value: f64, digits: usize) -> f64 {
    format!("{:.*e}", digits.saturating_sub(1), value)
        .parse()
        .unwrap_or(value)
}

/// A number the reader's conversion `read` turns into `target` bit for bit, near `guess`: the one
/// with the fewest significant digits when rounding `guess` finds one, and otherwise the nearest
/// found by stepping `guess` one unit in the last place at a time, up to 256 each way. `None` when
/// neither finds one, which a target the reader produced from some number in the first place
/// makes unlikely.
fn preimage(target: f64, guess: f64, read: impl Fn(f64) -> f64) -> Option<f64> {
    let hits = |x: f64| x.is_finite() && read(x).to_bits() == target.to_bits();
    if !guess.is_finite() {
        return None;
    }
    if let Some(found) = (1..=17)
        .map(|digits| shortest(guess, digits))
        .find(|x| hits(*x))
    {
        return Some(found);
    }
    let (mut up, mut down) = (guess, guess);
    for _ in 0..256 {
        up = up.next_up();
        if hits(up) {
            return Some(up);
        }
        down = down.next_down();
        if hits(down) {
            return Some(down);
        }
    }
    None
}

/// Whether `id` is one the reader invents for a `<tag>` with no id of its own: the tag, a hyphen
/// and the part's count (`bodytube-3`).
fn invented(id: &str, tag: &str) -> bool {
    id.strip_prefix(tag)
        .and_then(|rest| rest.strip_prefix('-'))
        .is_some_and(|count| !count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether `text` is a UUID as OpenRocket writes one: 8, 4, 4, 4 and 12 hexadecimal digits,
/// joined by hyphens.
fn is_uuid(text: &str) -> bool {
    let groups: Vec<&str> = text.split('-').collect();
    groups.len() == 5
        && groups.iter().zip([8, 4, 4, 4, 12]).all(|(group, length)| {
            group.len() == length && group.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::super::{Design, design, read};
    use super::super::document;
    use super::*;

    /// A design read from `xml`, written out, and read back: the design, the one read back, the
    /// written text, and the export's warnings.
    fn round_trip(xml: &str) -> (Design, Design, String, Vec<Warning>) {
        let xml = uuids(xml);
        let file = read(xml.as_bytes()).expect("the test design reads").value;
        let original = design(&file).value;
        let written = document(&original);
        let text = written.value.to_xml();
        let file = read(text.as_bytes())
            .expect("the written design reads")
            .value;
        let back = design(&file).value;
        (original, back, text, written.warnings)
    }

    /// `xml` with each `<id>name</id>` made a UUID, as OpenRocket writes ids: the export leaves
    /// any other id out, since OpenRocket will not open a file holding one ([`id`]). The same name
    /// always gives the same UUID, so the tests can name their parts.
    pub(in super::super) fn uuids(xml: &str) -> String {
        let mut out = String::with_capacity(xml.len());
        let mut rest = xml;
        while let Some(start) = rest.find("<id>") {
            let (before, after) = rest.split_at(start + "<id>".len());
            out.push_str(before);
            let end = after.find("</id>").unwrap_or(after.len());
            let name = &after[..end];
            if is_uuid(name) {
                out.push_str(name);
            } else {
                // FNV-1a: a fixed hash, so the UUID is the same on every run.
                let hash = name.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                    (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
                });
                out.push_str(&format!(
                    "{:08x}-0000-4000-8000-{:012x}",
                    hash >> 32,
                    hash & 0xffff_ffff_ffff
                ));
            }
            rest = &after[end..];
        }
        out.push_str(rest);
        out
    }

    /// `xml`'s design reads back the same, with nothing warned of, and is written the same again.
    fn same(xml: &str) -> String {
        let (original, back, text, warnings) = round_trip(xml);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(back, original, "{text}");
        assert_eq!(document(&back).value.to_xml(), text);
        text
    }

    /// A document holding one stage of `parts`, with nothing in it hpr keeps unread.
    fn rocket_of(parts: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Probe</name><referencetype>maximum</referencetype>
    <subcomponents><stage><name>Sustainer</name><id>sustainer</id><subcomponents>{parts}
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#
        )
    }

    /// A body tube 50 mm in radius holding `parts`, after a nose cone.
    fn tube_of(parts: &str) -> String {
        rocket_of(&format!(
            r#"
      <nosecone><name>Nose</name><id>nose</id>
        <material type="bulk" density="1000.0">Invented plastic</material>
        <length>0.25</length><thickness>0.002</thickness><shape>conical</shape>
        <aftradius>auto</aftradius></nosecone>
      <bodytube><name>Tube</name><id>tube</id>
        <material type="bulk" density="700.0">Invented paper</material>
        <length>0.8</length><thickness>0.0015</thickness><radius>0.05</radius>
        <subcomponents>{parts}</subcomponents></bodytube>"#
        ))
    }

    /// The spine: every shape, walls stated, filled and left out, shoulders capped and not,
    /// automatic radii with and without a cached number, finishes, and overrides.
    #[test]
    fn the_body_components_round_trip() {
        let text = same(&rocket_of(
            r#"
      <nosecone><name>Nose</name><id>nose</id><finish>polished</finish>
        <overridemass>0.3</overridemass><overridecg>0.12</overridecg>
        <overridesubcomponentsmass>true</overridesubcomponentsmass>
        <overridesubcomponentscg>true</overridesubcomponentscg>
        <material type="bulk" density="1250.0">Invented plastic</material>
        <length>0.2</length><thickness>0.0015</thickness>
        <shape>ogive</shape><shapeparameter>0.7</shapeparameter>
        <aftradius>auto 0.03</aftradius><aftshoulderradius>auto</aftshoulderradius>
        <aftshoulderlength>0.04</aftshoulderlength><aftshoulderthickness>0.0015</aftshoulderthickness>
        <aftshouldercapped>true</aftshouldercapped><isflipped>false</isflipped></nosecone>
      <bodytube><name>Upper</name><id>upper</id><finish>smooth</finish>
        <material type="bulk" density="700.0">Invented paper</material>
        <length>0.5</length><thickness>0.0012</thickness><radius>0.03</radius></bodytube>
      <transition><name>Taper</name><id>taper</id><finish>rough</finish>
        <material type="bulk" density="700.0">Invented paper</material>
        <length>0.06</length><thickness>0.002</thickness><shape>haack</shape>
        <shapeparameter>0.3333333333333333</shapeparameter><shapeclipped>false</shapeclipped>
        <foreradius>auto</foreradius><aftradius>0.025</aftradius>
        <foreshoulderradius>0.0285</foreshoulderradius><foreshoulderlength>0.03</foreshoulderlength>
        <foreshoulderthickness>0.001</foreshoulderthickness><foreshouldercapped>true</foreshouldercapped>
        <aftshoulderradius>0.0235</aftshoulderradius><aftshoulderlength>0.02</aftshoulderlength>
        <aftshoulderthickness>0.001</aftshoulderthickness><aftshouldercapped>false</aftshouldercapped>
      </transition>
      <bodytube><name>Lower</name><id>lower</id><finish>unfinished</finish>
        <overridesubcomponentsmass>true</overridesubcomponentsmass>
        <material type="bulk" density="700.0">Invented paper</material>
        <length>0.4</length><radius>auto 0.025</radius></bodytube>
      <bodytube><name>Solid</name><id>solid</id><finish>normal</finish>
        <material type="bulk" density="700.0">Invented wood</material>
        <length>0.05</length><thickness>filled</thickness><radius>0.025</radius></bodytube>
      <transition><name>Boat tail</name><id>tail</id>
        <material type="bulk" density="1250.0">Invented plastic</material>
        <length>0.05</length><thickness>filled</thickness><shape>power</shape>
        <shapeparameter>0.5</shapeparameter><foreradius>0.025</foreradius>
        <aftradius>0.018</aftradius></transition>
      <nosecone><name>Tail cone</name><id>tailcone</id>
        <material type="bulk" density="1250.0">Invented plastic</material>
        <length>0.04</length><thickness>0.001</thickness><shape>parabolic</shape>
        <shapeparameter>0.75</shapeparameter><aftradius>0.018</aftradius>
        <isflipped>true</isflipped></nosecone>"#,
        ));
        assert!(text.contains("<thickness>filled</thickness>"), "{text}");
        assert!(
            text.contains("<shapeparameter>0.7</shapeparameter>"),
            "{text}"
        );
    }

    /// A shoulder's cap is asked for only where a cap could be, so where it could not the tag is
    /// left for what the design keeps of the file, not written: a shoulder whose wall fills it.
    #[test]
    fn a_shoulder_cap_is_written_only_where_it_is_read() {
        let text = same(&rocket_of(
            r#"
      <nosecone><name>Nose</name><id>nose</id>
        <length>0.2</length><thickness>0.002</thickness><shape>conical</shape>
        <aftradius>0.03</aftradius><aftshoulderradius>0.028</aftshoulderradius>
        <aftshoulderlength>0.04</aftshoulderlength><aftshoulderthickness>0.028</aftshoulderthickness>
        </nosecone>"#,
        ));
        assert!(!text.contains("shouldercapped"), "{text}");
    }

    /// A stage's override covers everything inside it, and a second stage's parts count from
    /// their own stage.
    #[test]
    fn stages_and_their_overrides_round_trip() {
        same(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Two stages</name><referencetype>maximum</referencetype>
    <subcomponents>
      <stage><name>Sustainer</name><id>upper-stage</id>
        <overridemass>1.25</overridemass><overridecg>0.4</overridecg>
        <overridesubcomponentsmass>true</overridesubcomponentsmass>
        <overridesubcomponentscg>true</overridesubcomponentscg>
        <subcomponents>
          <nosecone><name>Nose</name><id>nose</id><length>0.2</length><thickness>0.002</thickness>
            <shape>ellipsoid</shape><aftradius>0.02</aftradius></nosecone>
          <bodytube><name>Tube</name><id>tube</id><length>0.4</length><thickness>0.001</thickness>
            <radius>auto</radius></bodytube>
        </subcomponents></stage>
      <stage><name>Booster</name><id>booster</id><overridecg>0.2</overridecg>
        <overridesubcomponentscg>true</overridesubcomponentscg>
        <subcomponents>
          <bodytube><name>Tube</name><id>booster-tube</id><length>0.3</length>
            <thickness>0.001</thickness><radius>0.02</radius></bodytube>
        </subcomponents></stage>
    </subcomponents></rocket>
</openrocket>"#,
        );
    }

    /// Every part inside a body tube: a coupler whose radius is the tube's bore with a bulkhead in
    /// it, a motor tube in a cluster, rings with automatic radii, and the packed parts, one of
    /// them off the axis and with a centre-of-gravity override alone.
    #[test]
    fn the_parts_inside_round_trip() {
        let text = same(&tube_of(
            r#"
          <tubecoupler><name>Coupler</name><id>coupler</id>
            <axialoffset method="bottom">0.05</axialoffset>
            <material type="bulk" density="700.0">Invented paper</material>
            <length>0.1</length><outerradius>auto</outerradius><thickness>0.002</thickness>
            <subcomponents>
              <bulkhead><name>Bulkhead</name><id>bulkhead</id>
                <axialoffset method="top">0.0</axialoffset>
                <material type="bulk" density="600.0">Invented plywood</material>
                <length>0.006</length><outerradius>auto 0.0465</outerradius></bulkhead>
            </subcomponents></tubecoupler>
          <innertube><name>Motor tube</name><id>mmt</id>
            <axialoffset method="bottom">0.01</axialoffset>
            <material type="bulk" density="700.0">Invented paper</material>
            <length>0.2</length><outerradius>0.0127</outerradius><thickness>0.0005</thickness>
            <radialposition>0.0</radialposition><angleoffset>30.0</angleoffset>
            <clusterconfiguration>4-ring</clusterconfiguration><clusterscale>1.1</clusterscale>
            <clusterrotation>15.0</clusterrotation></innertube>
          <centeringring><name>Ring</name><id>ring</id>
            <axialoffset method="bottom">-0.01</axialoffset>
            <material type="bulk" density="600.0">Invented plywood</material>
            <length>0.005</length><outerradius>auto</outerradius><innerradius>auto 0.0127</innerradius>
            </centeringring>
          <centeringring><name>Stated ring</name><id>ring-2</id>
            <axialoffset method="middle">0.0</axialoffset>
            <material type="bulk" density="600.0">Invented plywood</material>
            <length>0.005</length><outerradius>0.0485</outerradius><innerradius>0.01</innerradius>
            </centeringring>
          <parachute><name>Main</name><id>main</id>
            <axialoffset method="top">0.02</axialoffset>
            <packedlength>0.08</packedlength><packedradius>auto</packedradius>
            <radialposition>0.0</radialposition><angleoffset>0.0</angleoffset>
            <material type="surface" density="0.067">Invented nylon</material>
            <diameter>0.9</diameter><linecount>8</linecount><linelength>0.8</linelength>
            <linematerial type="line" density="0.0023">Invented line</linematerial></parachute>
          <streamer><name>Streamer</name><id>streamer</id>
            <axialoffset method="after">0.01</axialoffset>
            <packedlength>0.03</packedlength><packedradius>0.02</packedradius>
            <radialposition>0.0</radialposition><angleoffset>0.0</angleoffset>
            <material type="surface" density="0.05">Invented film</material>
            <striplength>1.2</striplength><stripwidth>0.05</stripwidth></streamer>
          <shockcord><name>Cord</name><id>cord</id>
            <axialoffset method="absolute">0.5</axialoffset>
            <packedlength>0.04</packedlength><packedradius>0.015</packedradius>
            <radialposition>0.0</radialposition><angleoffset>0.0</angleoffset>
            <material type="line" density="0.01">Invented nylon strap</material>
            <cordlength>2.5</cordlength></shockcord>
          <masscomponent><name>Altimeter</name><id>altimeter</id>
            <axialoffset method="top">0.1</axialoffset>
            <overridecg>0.01</overridecg><overridesubcomponentscg>true</overridesubcomponentscg>
            <packedlength>0.03</packedlength><packedradius>0.01</packedradius>
            <radialposition>0.02</radialposition><angleoffset>119.99999999999999</angleoffset>
            <mass>0.05</mass></masscomponent>"#,
        ));
        for tag in [
            "<tubecoupler>",
            "<innertube>",
            "<bulkhead>",
            "<centeringring>",
        ] {
            assert!(text.contains(tag), "{tag}: {text}");
        }
        assert!(text.contains("<clusterconfiguration>4-ring</clusterconfiguration>"));
    }

    /// Every part on a body tube: each fin outline, with tabs measured from the front, a fillet
    /// and a cant; tube fins of an automatic radius; a row of lugs; rail buttons measured from
    /// each end; and a pod set, placed from the tube's surface in the file and written as the
    /// distance it is.
    #[test]
    fn the_parts_outside_round_trip() {
        let text = same(&tube_of(
            r#"
          <trapezoidfinset><name>Fins</name><id>fins</id>
            <instancecount>3</instancecount><fincount>3</fincount><radiusoffset>0.0</radiusoffset>
            <angleoffset>0.0</angleoffset><rotation>0.0</rotation>
            <axialoffset method="bottom">0.0</axialoffset><position type="bottom">0.0</position>
            <material type="bulk" density="600.0">Invented plywood</material>
            <thickness>0.003</thickness><crosssection>airfoil</crosssection><cant>1.5</cant>
            <tabheight>0.01</tabheight><tablength>0.05</tablength>
            <tabposition relativeto="center">0.004</tabposition>
            <filletradius>0.005</filletradius>
            <filletmaterial type="bulk" density="1100.0">Invented epoxy</filletmaterial>
            <rootchord>0.12</rootchord><tipchord>0.05</tipchord><sweeplength>0.06</sweeplength>
            <height>0.08</height></trapezoidfinset>
          <ellipticalfinset><name>Canards</name><id>canards</id>
            <instancecount>4</instancecount><angleoffset>45.0</angleoffset>
            <axialoffset method="top">0.1</axialoffset>
            <material type="bulk" density="600.0">Invented plywood</material>
            <thickness>0.002</thickness><crosssection>rounded</crosssection>
            <tabheight>0.005</tabheight><tablength>0.02</tablength>
            <tabposition relativeto="end">-0.01</tabposition>
            <rootchord>0.06</rootchord><height>0.03</height></ellipticalfinset>
          <freeformfinset><name>Strakes</name><id>strakes</id>
            <instancecount>2</instancecount><angleoffset>90.0</angleoffset>
            <axialoffset method="middle">0.0</axialoffset>
            <material type="bulk" density="600.0">Invented plywood</material>
            <thickness>0.002</thickness><crosssection>square</crosssection><cant>-0.25</cant>
            <finpoints><point x="0.0" y="0.0"/><point x="0.03" y="0.02"/>
              <point x="0.1" y="0.02"/><point x="0.12" y="0.0"/></finpoints></freeformfinset>
          <tubefinset><name>Tube fins</name><id>tube-fins</id>
            <instancecount>6</instancecount><angleoffset>10.0</angleoffset>
            <axialoffset method="bottom">-0.02</axialoffset>
            <material type="bulk" density="700.0">Invented paper</material>
            <length>0.08</length><radius>auto</radius><thickness>0.0008</thickness></tubefinset>
          <launchlug><name>Lugs</name><id>lugs</id>
            <instancecount>2</instancecount><instanceseparation>0.1</instanceseparation>
            <angleoffset>45.0</angleoffset><axialoffset method="middle">0.02</axialoffset>
            <material type="bulk" density="700.0">Invented paper</material>
            <radius>0.004</radius><length>0.03</length><thickness>0.0005</thickness></launchlug>
          <railbutton><name>Buttons</name><id>buttons</id>
            <instancecount>2</instancecount><instanceseparation>0.3</instanceseparation>
            <angleoffset>180.0</angleoffset><axialoffset method="bottom">-0.05</axialoffset>
            <material type="bulk" density="1400.0">Invented acetal</material>
            <outerdiameter>0.0102</outerdiameter><innerdiameter>0.0062</innerdiameter>
            <height>0.0071</height><baseheight>0.0015</baseheight>
            <flangeheight>0.0016</flangeheight></railbutton>
          <railbutton><name>Top button</name><id>button-top</id>
            <axialoffset method="top">0.123</axialoffset>
            <outerdiameter>0.0102</outerdiameter><innerdiameter>0.0062</innerdiameter>
            <height>0.0071</height><baseheight>0.0015</baseheight>
            <flangeheight>0.0016</flangeheight></railbutton>
          <railbutton><name>Middle button</name><id>button-middle</id>
            <instancecount>3</instancecount><instanceseparation>0.07</instanceseparation>
            <axialoffset method="middle">0.011</axialoffset>
            <outerdiameter>0.0102</outerdiameter><innerdiameter>0.0062</innerdiameter>
            <height>0.0071</height><baseheight>0.0015</baseheight>
            <flangeheight>0.0016</flangeheight></railbutton>
          <railbutton><name>Placed button</name><id>button-absolute</id>
            <axialoffset method="absolute">0.777</axialoffset>
            <outerdiameter>0.0102</outerdiameter><innerdiameter>0.0062</innerdiameter>
            <height>0.0071</height><baseheight>0.0015</baseheight>
            <flangeheight>0.0016</flangeheight></railbutton>
          <podset><name>Pods</name><id>pods</id>
            <instancecount>2</instancecount><radiusoffset method="surface">0.0</radiusoffset>
            <angleoffset>90.0</angleoffset><axialoffset method="top">0.1</axialoffset>
            <subcomponents>
              <bodytube><name>Pod</name><id>pod</id>
                <material type="bulk" density="700.0">Invented paper</material>
                <length>0.15</length><thickness>0.001</thickness><radius>0.012</radius></bodytube>
            </subcomponents></podset>"#,
        ));
        assert!(text.contains(r#"<radiusoffset method="free">"#), "{text}");
        // With nothing kept to say which end the file measured a tab from, it is the front.
        assert_eq!(text.matches(r#"relativeto="front""#).count(), 2, "{text}");
    }

    /// What the design keeps of the file goes back where it was, and says which tag a part was
    /// written as: a part hpr does not read keeps its place among the parts it stood between;
    /// a nose cone turned round and an engine block come back as themselves, not as the
    /// transition and inner tube the design holds; a fin tab's second place is written for its
    /// kept frame to go back on, and the first is measured from that same end.
    #[test]
    fn what_the_design_keeps_goes_back_where_it_was() {
        let text = same(&rocket_of(
            r#"
      <bodytube><name>Tube</name><id>tube</id>
        <length>0.8</length><thickness>0.0015</thickness><radius>0.05</radius>
        <subcomponents>
          <masscomponent><name>First</name><id>first</id>
            <axialoffset method="top">0.1</axialoffset><mass>0.01</mass>
            <appearance><paint red="255" green="0" blue="0"/></appearance></masscomponent>
          <parallelstage><name>Booster</name><id>kept</id></parallelstage>
          <engineblock><name>Block</name><id>block</id>
            <axialoffset method="top">0.3</axialoffset>
            <material type="bulk" density="700.0" group="PaperProducts">Invented paper</material>
            <length>0.005</length><outerradius>0.019</outerradius><thickness>0.004</thickness>
            </engineblock>
          <trapezoidfinset><name>Fins</name><id>fins</id>
            <instancecount>3</instancecount><radiusoffset method="surface">0.0</radiusoffset>
            <angleoffset method="relative">0.0</angleoffset>
            <axialoffset method="bottom">0.0</axialoffset><thickness>0.003</thickness>
            <tabheight>0.01</tabheight><tablength>0.05</tablength>
            <tabposition relativeto="center">0.004</tabposition>
            <tabposition relativeto="middle">0.004</tabposition>
            <rootchord>0.12</rootchord><tipchord>0.05</tipchord><sweeplength>0.06</sweeplength>
            <height>0.08</height></trapezoidfinset>
          <masscomponent><name>Second</name><id>second</id>
            <axialoffset method="top">0.2</axialoffset><mass>0.02</mass>
            <appearance><paint red="0" green="0" blue="255"/></appearance></masscomponent>
        </subcomponents></bodytube>
      <nosecone><name>Tail cone</name><id>tailcone</id>
        <material type="bulk" density="1250.0" group="Plastics">Invented plastic</material>
        <length>0.04</length><thickness>0.001</thickness><shape>conical</shape>
        <aftradius>0.05</aftradius><isflipped>true</isflipped></nosecone>"#,
        ));
        let first = text.find("<name>First</name>").expect("the first part");
        let kept = text.find("<parallelstage>").expect("the kept part");
        let second = text.find("<name>Second</name>").expect("the second part");
        assert!(first < kept && kept < second, "{text}");
        assert_eq!(text.matches("<appearance>").count(), 2, "{text}");
        assert!(text.contains("<engineblock>"), "{text}");
        assert!(text.contains("<isflipped>true</isflipped>"), "{text}");
        assert!(text.contains(r#"<tabposition relativeto="center">0.004</tabposition>"#));
        assert!(text.contains(r#"<tabposition relativeto="middle">0.004</tabposition>"#));
    }

    /// Each of OpenRocket's cluster patterns, turned and scaled, is written as a pattern, scale
    /// and rotation that read back to the same places.
    #[test]
    fn every_cluster_reads_back_to_the_same_places() {
        let patterns = [
            "double", "3-row", "3-ring", "3-star", "4-row", "4-ring", "4-star", "5-ring", "5-star",
            "6-ring", "6-star", "9-grid", "9-star",
        ];
        for (k, name) in (0_u32..).zip(patterns) {
            let k = f64::from(k);
            let (scale, rotation, angle) = (1.0 + 0.05 * k, 7.5 * k - 20.0, 12.0 * k);
            same(&tube_of(&format!(
                r#"
          <innertube><name>Cluster</name><id>cluster</id>
            <axialoffset method="bottom">0.0</axialoffset>
            <length>0.2</length><outerradius>0.0095</outerradius><thickness>0.0005</thickness>
            <radialposition>0.0</radialposition><angleoffset>{angle}</angleoffset>
            <clusterconfiguration>{name}</clusterconfiguration><clusterscale>{scale}</clusterscale>
            <clusterrotation>{rotation}</clusterrotation></innertube>"#
            )));
        }
    }

    /// An angle in radians is written as degrees that read back as exactly it, a whole-degree
    /// angle as the whole number, including one read from float dust.
    #[test]
    fn angles_read_back_to_the_same_bits() {
        for degrees in [
            0.0,
            -0.0,
            1.5,
            -3.98,
            19.0,
            45.0,
            90.0,
            119.999_999_999_999_99,
            120.0,
            180.0,
            359.9,
            1e-9,
            7200.25,
        ] {
            let radians = f64::to_radians(degrees);
            let written = preimage(radians, radians.to_degrees(), f64::to_radians)
                .expect("an angle the reader made has degrees");
            assert_eq!(
                written.to_radians().to_bits(),
                radians.to_bits(),
                "{degrees}"
            );
        }
        let right = 90.0_f64.to_radians();
        assert_eq!(
            preimage(right, right.to_degrees(), f64::to_radians),
            Some(90.0)
        );
    }

    /// The ogive's radius ratio is the reciprocal of the file's parameter; the parameter written
    /// reads back as the ratio exactly.
    #[test]
    fn ogive_parameters_read_back_to_the_same_bits() {
        for kappa in [
            1.0,
            0.7,
            0.3,
            0.123_456_789,
            0.999_999_999_999,
            1e-6,
            0.1 + 0.2,
        ] {
            let ratio = 1.0 / kappa;
            let written = preimage(ratio, 1.0 / ratio, |k| 1.0 / k).expect("a parameter");
            assert_eq!((1.0 / written).to_bits(), ratio.to_bits(), "{kappa}");
        }
    }

    /// What a design holds that a `.ork` cannot say is warned of rather than written wrong.
    #[test]
    fn what_a_ork_cannot_say_is_warned_of() {
        let xml = rocket_of(
            r#"
      <bodytube><name>Tube</name><id>tube</id><finish>normal</finish><length>0.3</length>
        <thickness>0.001</thickness><radius>0.02</radius></bodytube>"#,
        );
        let file = read(uuids(&xml).as_bytes()).expect("reads").value;
        let mut original = design(&file).value;
        let tube = &mut original.rocket.stages[0].components[0];
        tube.finish = Some(Finish::Custom { roughness_m: 33e-6 });
        tube.overrides.cg_xy_m = Some([0.001, 0.0]);
        original.rocket.reference_diameter = ReferenceDiameter::NoseBase {};
        let warnings = document(&original).warnings;
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings.iter().all(|w| w.kind == WarningKind::Dropped));
    }

    /// Text the written file must hold, and how many times.
    type Holds<'a> = [(&'a str, usize)];

    /// `xml` read, written and read back, where the reader drops or simplifies a value: the
    /// reader warns of it (with `warning` in the message, when there is one), the design and every
    /// warning come back the same, the written text holds each of `tags` exactly the number of
    /// times given, as the file wrote it, and it is written the same again. Returns the design and
    /// the text.
    fn kept_as_written(xml: &str, warning: Option<&str>, tags: &Holds<'_>) -> (Design, String) {
        let xml = uuids(xml);
        let file = read(xml.as_bytes()).expect("the test design reads").value;
        let original = design(&file);
        match warning {
            Some(warning) => assert!(
                original
                    .warnings
                    .iter()
                    .any(|w| w.message.contains(warning)),
                "`{warning}` not in {:?}",
                original.warnings
            ),
            None => assert!(original.warnings.is_empty(), "{:?}", original.warnings),
        }
        let written = document(&original.value);
        assert!(written.warnings.is_empty(), "{:?}", written.warnings);
        let text = written.value.to_xml();
        let file = read(text.as_bytes())
            .expect("the written design reads")
            .value;
        let back = design(&file);
        assert_eq!(back.value, original.value, "{text}");
        assert_eq!(back.warnings, original.warnings, "{text}");
        for (tag, times) in tags {
            assert_eq!(text.matches(tag).count(), *times, "{tag} in {text}");
        }
        assert_eq!(document(&back.value).value.to_xml(), text);
        (original.value, text)
    }

    /// A body tube 50 mm in radius holding `parts` and a motor for configuration `c1`, which the
    /// rocket declares: a rocket that flies unless its airframe was not read as written.
    fn flying_tube_of(parts: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Probe</name><referencetype>maximum</referencetype>
    <motorconfiguration configid="c1" default="true"><name>F15</name></motorconfiguration>
    <subcomponents><stage><name>Sustainer</name><id>sustainer</id><subcomponents>
      <nosecone><name>Nose</name><id>nose</id>
        <length>0.25</length><thickness>0.002</thickness><shape>conical</shape>
        <aftradius>auto</aftradius></nosecone>
      <bodytube><name>Tube</name><id>tube</id>
        <length>0.8</length><thickness>0.0015</thickness><radius>0.05</radius>
        <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
          <overhang>0.005</overhang>
          <motor configid="c1"><type>single</type><manufacturer>Estes</manufacturer>
            <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
            <delay>4.0</delay></motor></motormount>
        <subcomponents>{parts}</subcomponents></bodytube>
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#
        )
    }

    /// A rail button's screw head, which hpr does not model, is kept as written: the export
    /// writes it back, so reading the export warns of it again and flies none of the rocket's
    /// configurations, as reading the original did, and OpenRocket reads the screw it had.
    #[test]
    fn a_rail_buttons_screw_height_is_written_back() {
        let button = |screw: &str| {
            format!(
                r#"
          <railbutton><name>Button</name><id>button</id>
            <axialoffset method="top">0.2</axialoffset>
            <outerdiameter>0.0102</outerdiameter><innerdiameter>0.0062</innerdiameter>
            <height>0.0071</height><baseheight>0.0015</baseheight>
            <flangeheight>0.0016</flangeheight>{screw}</railbutton>"#
            )
        };
        let (flown, _) = kept_as_written(&flying_tube_of(&button("")), None, &[]);
        assert!(flown.motors.configurations[0].left_out.is_none());
        let (screwed, _) = kept_as_written(
            &flying_tube_of(&button("<screwheight>0.004</screwheight>")),
            Some("screw head"),
            &[("<screwheight>0.004</screwheight>", 1)],
        );
        assert!(screwed.motors.configurations[0].left_out.is_some());
        // A screw of no height is no loss, but the design holds no screw to write either.
        kept_as_written(
            &flying_tube_of(&button("<screwheight>0.0</screwheight>")),
            None,
            &[("<screwheight>0.0</screwheight>", 1)],
        );
    }

    /// Each value a reader drops or simplifies, with a warning, is written back as the file had
    /// it, once, in place of what the writer would write from the design.
    #[test]
    fn a_value_the_reader_drops_is_written_back() {
        let lug = |inside: &str| {
            tube_of(&format!(
                r#"
          <launchlug><name>Lug</name><id>lug</id>
            <axialoffset method="top">0.1</axialoffset>{inside}
            <radius>0.004</radius><length>0.03</length><thickness>0.0005</thickness></launchlug>"#
            ))
        };
        let ring = |inside: &str| {
            tube_of(&format!(
                r#"
          <centeringring><name>Ring</name><id>ring</id>
            <axialoffset method="bottom">0.0</axialoffset>{inside}
            <length>0.005</length><outerradius>0.0485</outerradius><innerradius>0.01</innerradius>
            </centeringring>"#
            ))
        };
        let fins = |inside: &str| {
            tube_of(&format!(
                r#"
          <trapezoidfinset><name>Fins</name><id>fins</id>
            <instancecount>3</instancecount><axialoffset method="bottom">0.0</axialoffset>
            <thickness>0.003</thickness>{inside}
            <rootchord>0.12</rootchord><tipchord>0.05</tipchord><sweeplength>0.06</sweeplength>
            <height>0.08</height></trapezoidfinset>"#
            ))
        };
        let nose = |inside: &str| {
            rocket_of(&format!(
                r#"
      <nosecone><name>Nose</name><id>nose</id>
        <length>0.2</length><aftradius>0.03</aftradius>{inside}</nosecone>"#
            ))
        };
        let inner = |inside: &str| {
            tube_of(&format!(
                r#"
          <innertube><name>Motor tube</name><id>mmt</id>
            <axialoffset method="bottom">0.0</axialoffset>
            <length>0.2</length><outerradius>0.0127</outerradius><thickness>0.0005</thickness>
            <radialposition>0.01</radialposition>{inside}</innertube>"#
            ))
        };
        let mass = |inside: &str| {
            tube_of(&format!(
                r#"
          <masscomponent><name>Altimeter</name><id>altimeter</id>
            <axialoffset method="top">0.1</axialoffset>
            <packedlength>0.03</packedlength><packedradius>0.01</packedradius>
            <radialposition>0.02</radialposition>{inside}<mass>0.05</mass></masscomponent>"#
            ))
        };
        let cases: [(String, &str, &Holds<'_>); 26] = [
            (
                ring(
                    "<instancecount>3</instancecount><instanceseparation>0.02</instanceseparation>",
                ),
                "a row of more than one ring",
                &[
                    ("<instancecount>3</instancecount>", 1),
                    ("<instancecount>", 1),
                ],
            ),
            (
                ring("<radialposition>0.01</radialposition>"),
                "off the body axis",
                &[("<radialposition>0.01</radialposition>", 1)],
            ),
            (
                fins(r#"<radiusoffset method="surface">0.01</radiusoffset>"#),
                "standing off the body",
                &[
                    (r#"<radiusoffset method="surface">0.01</radiusoffset>"#, 1),
                    ("<radiusoffset", 1),
                ],
            ),
            (
                fins("<crosssection>wedge</crosssection>"),
                "not a fin section",
                &[
                    ("<crosssection>wedge</crosssection>", 1),
                    ("<crosssection>", 1),
                ],
            ),
            (
                fins(
                    r#"<tabheight>0.01</tabheight><tablength>0.05</tablength>
            <tabposition relativeto="nowhere">0.01</tabposition>"#,
                ),
                "a fin tab measured from `nowhere`",
                &[
                    (r#"<tabposition relativeto="nowhere">0.01</tabposition>"#, 1),
                    ("<tabposition", 1),
                ],
            ),
            (
                fins("<cant>steep</cant>"),
                "not a number",
                &[("<cant>steep</cant>", 1), ("<cant>", 1)],
            ),
            (
                tube_of(
                    r#"
          <tubefinset><name>Tube fins</name><id>tube-fins</id>
            <instancecount>12</instancecount><axialoffset method="bottom">0.0</axialoffset>
            <length>0.08</length><radius>0.01</radius><thickness>0.0008</thickness></tubefinset>"#,
                ),
                "a tube fin set of 12 tubes",
                &[
                    ("<instancecount>12</instancecount>", 1),
                    ("<instancecount>", 1),
                ],
            ),
            (
                tube_of(
                    r#"
          <innertube><name>Motor tube</name><id>mmt</id>
            <axialoffset method="bottom">0.0</axialoffset>
            <length>0.2</length><outerradius>0.0127</outerradius><thickness>0.0005</thickness>
            <clusterconfiguration>7-flower</clusterconfiguration></innertube>"#,
                ),
                "not one of OpenRocket's cluster patterns",
                &[("<clusterconfiguration>7-flower</clusterconfiguration>", 1)],
            ),
            (
                lug("<finish>mirror</finish>"),
                "not a surface finish",
                &[("<finish>mirror</finish>", 1), ("<finish>", 1)],
            ),
            (
                lug(r#"<position type="top">0.2</position>"#),
                "two names for one value",
                &[
                    (r#"<position type="top">0.2</position>"#, 1),
                    (r#"<axialoffset method="top">0.1</axialoffset>"#, 1),
                ],
            ),
            (
                lug("<angleoffset>30.0</angleoffset><radialdirection>60.0</radialdirection>"),
                "two names for one angle",
                &[
                    ("<radialdirection>60.0</radialdirection>", 1),
                    ("<angleoffset>30", 1),
                ],
            ),
            // On an inner tube and a packed part, OpenRocket 24.12 reads the angle only as
            // `radialdirection`, which the export writes the design's angle under; when the file
            // gave another angle under an older name, that one is kept and goes back, and the
            // design's angle is written as `angleoffset`, which the reader takes again.
            (
                inner("<angleoffset>30.0</angleoffset><radialdirection>45.0</radialdirection>"),
                "two names for one angle",
                &[
                    ("<radialdirection>45.0</radialdirection>", 1),
                    ("<radialdirection>", 1),
                    ("<angleoffset>30", 1),
                ],
            ),
            (
                inner("<angleoffset>30.0</angleoffset><rotation>45.0</rotation>"),
                "two names for one angle",
                &[
                    ("<rotation>45.0</rotation>", 1),
                    ("<angleoffset>30", 1),
                    ("<radialdirection>", 0),
                ],
            ),
            (
                mass("<angleoffset>30.0</angleoffset><radialdirection>45.0</radialdirection>"),
                "two names for one angle",
                &[
                    ("<radialdirection>45.0</radialdirection>", 1),
                    ("<radialdirection>", 1),
                    ("<angleoffset>30", 1),
                ],
            ),
            (
                lug(r#"<material type="surface" density="0.5">Invented felt</material>"#),
                "is declared `surface`",
                &[(
                    r#"<material type="surface" density="0.5">Invented felt</material>"#,
                    1,
                )],
            ),
            (
                lug(r#"<material type="bulk" density="heavy">Invented lead</material>"#),
                "states no density",
                &[(
                    r#"<material type="bulk" density="heavy">Invented lead</material>"#,
                    1,
                )],
            ),
            (
                lug(
                    r#"<overridemass>0.02</overridemass><overridecg>0.01</overridecg>
            <overridesubcomponentsmass>true</overridesubcomponentsmass>
            <overridesubcomponentscg>false</overridesubcomponentscg>"#,
                ),
                "does not agree",
                &[
                    (
                        "<overridesubcomponentsmass>true</overridesubcomponentsmass>",
                        1,
                    ),
                    (
                        "<overridesubcomponentscg>false</overridesubcomponentscg>",
                        1,
                    ),
                    ("<overridesubcomponentscg>", 1),
                ],
            ),
            // Both overridden and only the mass flag stated: the missing flag reads as `false`,
            // which disagrees, so the flags are kept, and none is written beside them.
            (
                lug(
                    r#"<overridemass>0.02</overridemass><overridecg>0.01</overridecg>
            <overridesubcomponentsmass>true</overridesubcomponentsmass>"#,
                ),
                "does not agree",
                &[
                    (
                        "<overridesubcomponentsmass>true</overridesubcomponentsmass>",
                        1,
                    ),
                    ("<overridesubcomponentsmass>", 1),
                    ("<overridesubcomponentscg>", 0),
                ],
            ),
            (
                lug(r#"<overridemass>0.02</overridemass>
            <overridesubcomponentsmass>true</overridesubcomponentsmass>
            <overridesubcomponentsmass>false</overridesubcomponentsmass>"#),
                "written 2 times",
                &[
                    (
                        "<overridesubcomponentsmass>true</overridesubcomponentsmass>",
                        1,
                    ),
                    (
                        "<overridesubcomponentsmass>false</overridesubcomponentsmass>",
                        1,
                    ),
                ],
            ),
            (
                lug(r#"<axialoffset method="sideways">0.1</axialoffset>"#).replacen(
                    r#"<axialoffset method="top">0.1</axialoffset>"#,
                    "",
                    1,
                ),
                "measured from `sideways`",
                &[
                    (r#"<axialoffset method="sideways">0.1</axialoffset>"#, 1),
                    ("<axialoffset", 1),
                ],
            ),
            (
                nose("<thickness>-0.001</thickness><shape>conical</shape>"),
                "is no wall",
                &[("<thickness>-0.001</thickness>", 1), ("<thickness>", 1)],
            ),
            (
                nose(
                    "<thickness>0.002</thickness><shape>conical</shape>
        <aftshoulderradius>0.028</aftshoulderradius><aftshoulderlength>0.04</aftshoulderlength>
        <aftshoulderthickness>-0.001</aftshoulderthickness>",
                ),
                "shoulder's wall",
                &[("<aftshoulderthickness>-0.001</aftshoulderthickness>", 1)],
            ),
            (
                nose(
                    "<thickness>0.002</thickness><shape>bulbous</shape><shapeparameter>0.5</shapeparameter>",
                ),
                "not a shape this reader knows",
                &[
                    ("<shape>bulbous</shape>", 1),
                    ("<shapeparameter>0.5</shapeparameter>", 1),
                    ("<shape>", 1),
                ],
            ),
            (
                nose("<thickness>0.002</thickness><shape>power</shape>"),
                "states no shape parameter",
                &[("<shape>power</shape>", 1), ("<shape>", 1)],
            ),
            (
                rocket_of(
                    r#"
      <bodytube><name>Tube</name><id>00000000-0000-4000-8000-00000000000a</id>
        <length>0.3</length><thickness>0.001</thickness><radius>0.02</radius></bodytube>
      <bodytube><name>Tube</name><id>00000000-0000-4000-8000-00000000000a</id>
        <length>0.3</length><thickness>0.001</thickness><radius>0.02</radius></bodytube>"#,
                ),
                "already the id of another component",
                &[("<id>00000000-0000-4000-8000-00000000000a</id>", 2)],
            ),
            (
                rocket_of(
                    r#"
      <bodytube><name>Tube</name><id>tube</id><length>0.3</length><thickness>0.001</thickness>
        <radius>0.02</radius></bodytube>"#,
                )
                .replace(
                    "<referencetype>maximum</referencetype>",
                    "<referencetype>nosecone</referencetype>",
                ),
                "a reference diameter chosen by `nosecone`",
                &[
                    ("<referencetype>nosecone</referencetype>", 1),
                    ("<referencetype>", 1),
                ],
            ),
        ];
        for (xml, warning, tags) in &cases {
            kept_as_written(xml, Some(warning), tags);
        }
    }

    /// The drag override `hpr-design` does not hold is written back as the file had it, with no
    /// warning, and with the single older flag, which covers drag too, the flags are all kept in
    /// the order that says which wins.
    #[test]
    fn a_drag_override_is_written_back() {
        let lug = |inside: &str| {
            tube_of(&format!(
                r#"
          <launchlug><name>Lug</name><id>lug</id>
            <axialoffset method="top">0.1</axialoffset>{inside}
            <radius>0.004</radius><length>0.03</length><thickness>0.0005</thickness></launchlug>"#
            ))
        };
        kept_as_written(
            &lug(
                "<overridecd>0.0</overridecd><overridesubcomponentscd>false</overridesubcomponentscd>",
            ),
            None,
            &[
                ("<overridecd>0.0</overridecd>", 1),
                (
                    "<overridesubcomponentscd>false</overridesubcomponentscd>",
                    1,
                ),
            ],
        );
        let (_, text) = kept_as_written(
            &lug(
                "<overridesubcomponents>true</overridesubcomponents><overridemass>0.02</overridemass>
            <overridecd>0.4</overridecd><overridesubcomponentsmass>false</overridesubcomponentsmass>",
            ),
            None,
            &[
                ("<overridecd>0.4</overridecd>", 1),
                ("<overridesubcomponents>true</overridesubcomponents>", 1),
                ("<overridesubcomponentsmass>false</overridesubcomponentsmass>", 1),
                ("<overridesubcomponentsmass>", 1),
            ],
        );
        let older = text
            .find("<overridesubcomponents>")
            .expect("the older flag");
        let newer = text
            .find("<overridesubcomponentsmass>")
            .expect("the mass flag");
        assert!(older < newer, "{text}");
    }

    /// A pod set placed by a method the reader does not know, and one whose override does not
    /// cover its pods, are written back as the file had them.
    #[test]
    fn a_pod_sets_dropped_values_are_written_back() {
        let pods = |inside: &str| {
            tube_of(&format!(
                r#"
          <podset><name>Pods</name><id>pods</id>
            <instancecount>2</instancecount><axialoffset method="top">0.1</axialoffset>{inside}
            <subcomponents>
              <bodytube><name>Pod</name><id>pod</id>
                <length>0.15</length><thickness>0.001</thickness><radius>0.012</radius></bodytube>
            </subcomponents></podset>"#
            ))
        };
        kept_as_written(
            &pods(r#"<radiusoffset method="elsewhere">0.01</radiusoffset>"#),
            Some("measured by `elsewhere`"),
            &[
                (r#"<radiusoffset method="elsewhere">0.01</radiusoffset>"#, 1),
                ("<radiusoffset", 1),
            ],
        );
        kept_as_written(
            &pods(
                r#"<radiusoffset method="free">0.07</radiusoffset><overridemass>0.2</overridemass>
            <overridesubcomponentsmass>false</overridesubcomponentsmass>"#,
            ),
            Some("does not cover its pods"),
            &[
                (
                    "<overridesubcomponentsmass>false</overridesubcomponentsmass>",
                    1,
                ),
                ("<overridesubcomponentsmass>", 1),
            ],
        );
    }

    /// A motor's delay that is neither `none` nor a number, and a recovery device's empty
    /// deployment event, are written back as the file had them.
    #[test]
    fn a_motors_and_a_recoverys_dropped_values_are_written_back() {
        let delayed = flying_tube_of("").replace("<delay>4.0</delay>", "<delay>soon</delay>");
        kept_as_written(
            &delayed,
            Some("neither `none` nor a delay"),
            &[("<delay>soon</delay>", 1), ("<delay>", 1)],
        );
        let (_, text) = kept_as_written(
            &tube_of(
                r#"
          <parachute><name>Main</name><id>main</id>
            <axialoffset method="top">0.02</axialoffset>
            <packedlength>0.08</packedlength><packedradius>0.02</packedradius>
            <diameter>0.9</diameter><linecount>8</linecount><linelength>0.8</linelength>
            <deployevent></deployevent></parachute>"#,
            ),
            Some("`deployevent` is empty"),
            &[],
        );
        assert_eq!(text.matches("<deployevent").count(), 1, "{text}");
    }
}
