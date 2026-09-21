//! hpr's key geometry held to RocketSerializer's reading of the same `.ork` files (M3.1d2,
//! ADR-059), with OpenRocket's own numbers to settle a difference.
//!
//! `validation/oracles/rocketserializer/geometry.py` runs RocketSerializer's extractors on each
//! design and asks OpenRocket 24.12 for the same numbers. Each quantity hpr reads is held to
//! RocketSerializer's. Where the two differ, OpenRocket's number says which of them read the file
//! as OpenRocket does: when it is hpr's, RocketSerializer is counted apart, with the cause where
//! the record shows one; when it is not, hpr is apart, and the survey fails.

use std::collections::BTreeMap;

use hpr_design::tree::{Component, Layout, Part, PlacedComponent, Rocket};
use hpr_design::{FinCrossSection, FinPlanform, NoseShape};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// RocketSerializer and OpenRocket run on Loft's public demo designs, committed.
#[cfg(test)]
const PUBLIC: &str = "validation/fixtures/ork/rocketserializer-loft-demo.json";

/// The same run over the reference library and the jar's examples, which stays out of the
/// repository: the library is other people's designs (CLAUDE.md rule 4), and the examples are GPL.
pub(crate) const LIBRARY: &str = "corpus-out/rocketserializer.json";

/// The cause named when RocketSerializer places a component further aft than OpenRocket by
/// exactly the lengths of the components before it in its parent: its walk
/// (`process_elements_position`) moves its running station on by each child's length.
const EARLIER_SIBLINGS: &str = "its walk adds the lengths of the parts before it in its parent";

/// The cause named when RocketSerializer gives a transition the radii OpenRocket gives the first
/// transition of the same name: its `search_transitions` looks OpenRocket's transitions up by name.
const FIRST_OF_ITS_NAME: &str = "it takes the radii of the first transition of the same name";

/// How one quantity compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Verdict {
    /// hpr's number is RocketSerializer's.
    Agree,
    /// RocketSerializer's number is not OpenRocket's, and hpr's is.
    RocketSerializerApart,
    /// hpr's number is neither RocketSerializer's nor OpenRocket's.
    HprApart,
}

/// One quantity of one component, compared.
#[derive(Debug, Clone)]
pub(crate) struct Compared {
    /// The component's tag and the quantity, such as `trapezoidfinset station_m`.
    pub(crate) quantity: String,
    pub(crate) verdict: Verdict,
    /// Why RocketSerializer is apart, when the record shows it.
    pub(crate) cause: Option<&'static str>,
    /// Whether hpr's number is OpenRocket's too, when OpenRocket gave one: what shows that an
    /// agreement is not a mistake the two readers share.
    pub(crate) openrocket_same: Option<bool>,
    pub(crate) detail: Value,
}

/// One design's comparison.
#[derive(Debug, Default)]
pub(crate) struct DesignCheck {
    pub(crate) compared: Vec<Compared>,
    /// Entries inside a part hpr keeps whole without reading it: a pod or a parallel stage.
    pub(crate) in_kept_parts: usize,
    /// Quantities RocketSerializer gives no number for.
    pub(crate) not_stated: usize,
    /// Entries hpr has no component for.
    pub(crate) unmatched: Vec<String>,
    /// Extractors that raised instead of returning.
    pub(crate) extractor_errors: Vec<String>,
    /// Canted fin sets, and how many of them OpenRocket turns as `canted_shift` says.
    pub(crate) canted: [usize; 2],
    /// Nose profiles compared with OpenRocket's three radii, and how many are hpr's.
    pub(crate) profiles: [usize; 2],
}

/// How far aft OpenRocket moves the front of a canted fin's root, m: it turns the fin about the
/// middle of its root chord `c` by the cant `δ`, so the front moves `(c/2)(1 − cos δ)`. The record
/// keeps OpenRocket's turned station beside the one read with the cant set to zero, and this is
/// checked against every canted fin set, since the zero-cant reading rests on it.
fn canted_shift(root_chord_m: f64, cant_rad: f64) -> f64 {
    root_chord_m / 2.0 * (1.0 - cant_rad.cos())
}

impl DesignCheck {
    fn number(
        &mut self,
        quantity: String,
        ours: f64,
        rocketserializer: Option<f64>,
        openrocket: Option<f64>,
        cause: impl Fn(f64, f64) -> Option<&'static str>,
    ) {
        let Some(theirs) = rocketserializer else {
            self.not_stated += 1;
            return;
        };
        let verdict = if same(ours, theirs) {
            Verdict::Agree
        } else if openrocket.is_some_and(|or| same(ours, or) && !same(theirs, or)) {
            Verdict::RocketSerializerApart
        } else {
            Verdict::HprApart
        };
        let cause = match (verdict, openrocket) {
            (Verdict::RocketSerializerApart, Some(or)) => cause(theirs, or),
            _ => None,
        };
        self.compared.push(Compared {
            quantity,
            verdict,
            cause,
            openrocket_same: openrocket.map(|or| same(ours, or)),
            detail: json!({ "hpr": ours, "rocketserializer": theirs, "openrocket": openrocket }),
        });
    }

    fn word(
        &mut self,
        quantity: String,
        ours: &str,
        rocketserializer: Option<&str>,
        openrocket: Option<&str>,
    ) {
        // RocketSerializer's empty word is its default for a tag the file does not write.
        let Some(theirs) = rocketserializer.filter(|word| !word.is_empty()) else {
            self.not_stated += 1;
            return;
        };
        let verdict = if ours == theirs {
            Verdict::Agree
        } else if openrocket == Some(ours) {
            Verdict::RocketSerializerApart
        } else {
            Verdict::HprApart
        };
        self.compared.push(Compared {
            quantity,
            verdict,
            cause: None,
            openrocket_same: openrocket.map(|or| or == ours),
            detail: json!({ "hpr": ours, "rocketserializer": theirs, "openrocket": openrocket }),
        });
    }
}

/// Two readings of one decimal number, the second taken as the reference, as the survey compares
/// OpenRocket's radii.
fn same(ours: f64, theirs: f64) -> bool {
    (ours - theirs).abs() <= 1e-9 * theirs.abs().max(1e-6)
}

/// The tag a part is read from, for the parts RocketSerializer reports.
fn tag_of(part: &Part) -> Option<&'static str> {
    match part {
        Part::NoseCone(_) => Some("nosecone"),
        Part::Transition(_) => Some("transition"),
        Part::FinSet(fins) => match fins.planform {
            FinPlanform::Trapezoidal { .. } => Some("trapezoidfinset"),
            FinPlanform::Elliptical { .. } => Some("ellipticalfinset"),
            _ => None,
        },
        _ => None,
    }
}

/// A nose cone's shape in RocketSerializer's words: the file's `shape`, with a Haack series
/// called `Von Karman` when its parameter is zero and `lvhaack` otherwise.
fn nose_kind(shape: &NoseShape) -> &'static str {
    match shape {
        NoseShape::Conical {} => "conical",
        NoseShape::Ogive { .. } => "ogive",
        NoseShape::Elliptical {} => "ellipsoid",
        NoseShape::PowerSeries { .. } => "power",
        NoseShape::ParabolicSeries { .. } => "parabolic",
        NoseShape::Haack { parameter } if *parameter == 0.0 => "Von Karman",
        NoseShape::Haack { .. } => "lvhaack",
        // A shape added later has no word here, and so fails to agree until it is given one.
        _ => "(unnamed)",
    }
}

/// A fin section in the file's words, which RocketSerializer passes on.
fn section_word(section: FinCrossSection) -> &'static str {
    match section {
        FinCrossSection::Square => "square",
        FinCrossSection::Rounded => "rounded",
        FinCrossSection::Airfoil => "airfoil",
        // A section added later has no word here, and so fails to agree until it is given one.
        _ => "(unnamed)",
    }
}

/// OpenRocket's shape for a nose cone, in the same words.
fn openrocket_kind(openrocket: &Value) -> Option<String> {
    let shape = openrocket["shape"].as_str()?;
    Some(match shape {
        "HAACK" if openrocket["shape_parameter"].as_f64() == Some(0.0) => "Von Karman".to_owned(),
        "HAACK" => "lvhaack".to_owned(),
        other => other.to_ascii_lowercase(),
    })
}

/// Every component of the kinds RocketSerializer reports, depth first: the document's order.
fn visit<'a>(component: &'a Component, found: &mut Vec<(&'static str, &'a str, &'a str)>) {
    if let Some(tag) = tag_of(&component.part) {
        found.push((tag, &component.id, &component.name));
    }
    for child in &component.children {
        visit(child, found);
    }
}

/// hpr's reading of one design against the record of RocketSerializer and OpenRocket run on it.
pub(crate) fn compare(record: &Value, rocket: &Rocket, layout: &Layout) -> DesignCheck {
    let mut check = DesignCheck::default();
    let mut ours = Vec::new();
    for stage in &rocket.stages {
        for component in &stage.components {
            visit(component, &mut ours);
        }
    }

    // RocketSerializer's rocket radius is the largest radius the file writes as a number.
    let radius = &record["rocket_radius"];
    if let Some(error) = radius["error"].as_str() {
        check
            .extractor_errors
            .push(format!("rocket radius: {error}"));
    } else if let Some(radius) = radius["value"].as_f64() {
        let largest = layout
            .body()
            .flat_map(|placed| match &placed.part {
                Part::NoseCone(p) => vec![p.base_radius_m],
                Part::BodyTube(p) => vec![p.outer_radius_m],
                Part::Transition(p) => vec![p.fore_radius_m, p.aft_radius_m],
                _ => Vec::new(),
            })
            .fold(0.0, f64::max);
        // A file that writes every radius `auto` gives RocketSerializer nothing, and it says 0.
        check.number(
            "rocket radius_m".to_owned(),
            largest,
            (radius > 0.0).then_some(radius),
            record["openrocket_largest_radius_m"].as_f64(),
            |_, _| None,
        );
    }

    for (key, tag) in [
        ("nose", "nosecone"),
        ("transitions", "transition"),
        ("trapezoidal_fins", "trapezoidfinset"),
        ("elliptical_fins", "ellipticalfinset"),
    ] {
        let entry = &record[key];
        if let Some(error) = entry["error"].as_str() {
            check.extractor_errors.push(format!("{key}: {error}"));
            continue;
        }
        // An older file writes no ids; its components are matched by name, in order.
        let mut occurrences: BTreeMap<&str, usize> = BTreeMap::new();
        for row in entry["value"].as_array().into_iter().flatten() {
            if row["in_kept_part"].as_bool() == Some(true) {
                check.in_kept_parts += 1;
                continue;
            }
            let name = row["name"].as_str().unwrap_or_default();
            // RocketSerializer's nose can be a `<transition>` named "Nosecone".
            let tag = row["tag"].as_str().unwrap_or(tag);
            let found = match row["id"].as_str() {
                Some(id) => ours.iter().find(|(t, i, _)| *t == tag && *i == id),
                None => {
                    let seen = occurrences.entry(name).or_default();
                    *seen += 1;
                    ours.iter()
                        .filter(|(t, _, n)| *t == tag && *n == name)
                        .nth(*seen - 1)
                }
            };
            let Some((_, placed)) = found.and_then(|(_, id, _)| layout.find(id)) else {
                check.unmatched.push(format!("{tag} `{name}`"));
                continue;
            };
            let first = entry["value"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|other| other["name"] == row["name"]);
            let label = if key == "nose" { "nosecone" } else { tag };
            quantities(&mut check, label, key == "nose", row, first, placed);
        }
    }
    check
}

/// The quantities of one component, its dimensions and its station, taken as RocketSerializer's
/// nose when `as_nose`; `first` is the first entry of the same tag and name, which RocketSerializer
/// takes a transition's radii from.
fn quantities(
    check: &mut DesignCheck,
    tag: &str,
    as_nose: bool,
    row: &Value,
    first: Option<&Value>,
    placed: &PlacedComponent,
) {
    let rs = &row["rocketserializer"];
    let or = &row["openrocket"];
    let q = |name: &str| format!("{tag} {name}");
    let none = |_: f64, _: f64| None;
    // RocketSerializer's nose, read by hpr as a nose cone or, when the file has none, as the
    // transition RocketSerializer took for it.
    let nose = match &placed.part {
        _ if !as_nose => None,
        Part::NoseCone(nose) => Some((
            nose.shape,
            nose.length_m,
            nose.base_radius_m,
            nose.profile(),
        )),
        Part::Transition(t) => Some((t.shape, t.length_m, t.aft_radius_m, t.profile())),
        _ => return,
    };
    match (&placed.part, nose) {
        (_, Some((shape, length_m, base_radius_m, profile))) => {
            // OpenRocket's shape is its profile: when it draws hpr's, it names hpr's shape,
            // whatever word the file uses for it.
            let draws_ours = profile.ok().is_some_and(|profile| {
                let radii = or["profile_radii_m"].as_array();
                radii.is_some_and(|radii| {
                    radii.len() == 3
                        && [0.25, 0.5, 0.75]
                            .iter()
                            .zip(radii)
                            .all(|(fraction, radius)| {
                                radius.as_f64().is_some_and(|radius| {
                                    same(profile.radius_m(fraction * length_m), radius)
                                })
                            })
                })
            });
            if or["profile_radii_m"].is_array() {
                check.profiles[0] += 1;
                check.profiles[1] += usize::from(draws_ours);
            }
            let kind = if draws_ours {
                Some(nose_kind(&shape).to_owned())
            } else {
                openrocket_kind(or)
            };
            check.word(
                q("kind"),
                nose_kind(&shape),
                rs["kind"].as_str(),
                kind.as_deref(),
            );
            // OpenRocket's own word beside the one its profile settled on: an ogive of parameter
            // 0 draws as the cone hpr reads, and OpenRocket still calls it an ogive.
            if let Some(last) = check.compared.last_mut() {
                last.detail["openrocket_word"] = json!(openrocket_kind(or));
                last.detail["openrocket_draws_hprs_profile"] = json!(draws_ours);
            }
            // RocketSerializer passes a Haack series's parameter on, and only a Haack's.
            if let NoseShape::Haack { parameter } = shape {
                check.number(
                    q("haack_parameter"),
                    parameter,
                    rs["noseShapeParameter"].as_f64(),
                    or["shape_parameter"].as_f64(),
                    none,
                );
            }
            check.number(
                q("length_m"),
                length_m,
                rs["length"].as_f64(),
                or["length_m"].as_f64(),
                none,
            );
            check.number(
                q("base_radius_m"),
                base_radius_m,
                rs["base_radius"].as_f64(),
                or["base_radius_m"].as_f64(),
                none,
            );
        }
        (Part::Transition(transition), None) => {
            check.number(
                q("length_m"),
                transition.length_m,
                rs["length"].as_f64(),
                or["length_m"].as_f64(),
                none,
            );
            // RocketSerializer looks a transition's radii up in OpenRocket by name, and takes the
            // first transition it finds of that name.
            let first = first.filter(|first| !std::ptr::eq(*first, row));
            let borrowed = |key: &'static str| {
                move |theirs: f64, _: f64| {
                    let theirs_first = first?["openrocket"][key].as_f64()?;
                    same(theirs, theirs_first).then_some(FIRST_OF_ITS_NAME)
                }
            };
            check.number(
                q("fore_radius_m"),
                transition.fore_radius_m,
                rs["top_radius"].as_f64(),
                or["fore_radius_m"].as_f64(),
                borrowed("fore_radius_m"),
            );
            check.number(
                q("aft_radius_m"),
                transition.aft_radius_m,
                rs["bottom_radius"].as_f64(),
                or["aft_radius_m"].as_f64(),
                borrowed("aft_radius_m"),
            );
        }
        (Part::FinSet(fins), None) => {
            check.number(
                q("count"),
                f64::from(fins.count),
                rs["number"].as_f64(),
                or["count"].as_f64(),
                none,
            );
            let section = or["cross_section"].as_str().map(str::to_ascii_lowercase);
            check.word(
                q("cross_section"),
                section_word(fins.cross_section),
                rs["section"].as_str(),
                section.as_deref(),
            );
            let (root, tip, span, sweep) = match fins.planform {
                FinPlanform::Trapezoidal {
                    root_chord_m,
                    tip_chord_m,
                    span_m,
                    sweep_m,
                } => (root_chord_m, Some(tip_chord_m), span_m, Some(sweep_m)),
                FinPlanform::Elliptical {
                    root_chord_m,
                    span_m,
                } => (root_chord_m, None, span_m, None),
                _ => return,
            };
            check.number(
                q("root_chord_m"),
                root,
                rs["root_chord"].as_f64(),
                or["root_chord_m"].as_f64(),
                none,
            );
            if let Some(tip) = tip {
                check.number(
                    q("tip_chord_m"),
                    tip,
                    rs["tip_chord"].as_f64(),
                    or["tip_chord_m"].as_f64(),
                    none,
                );
            }
            check.number(
                q("span_m"),
                span,
                rs["span"].as_f64(),
                or["span_m"].as_f64(),
                none,
            );
            if let Some(sweep) = sweep {
                check.number(
                    q("sweep_m"),
                    sweep,
                    rs["sweep_length"].as_f64(),
                    or["sweep_m"].as_f64(),
                    none,
                );
            }
            // RocketSerializer passes the file's cant on in degrees, as the file writes it.
            check.number(
                q("cant_rad"),
                fins.cant_rad,
                rs["cant_angle"].as_f64().map(f64::to_radians),
                or["cant_rad"].as_f64(),
                none,
            );
        }
        _ => return,
    }
    if let (Some(cant), Some(root), Some(turned), Some(station)) = (
        or["cant_rad"].as_f64().filter(|cant| *cant != 0.0),
        or["root_chord_m"].as_f64(),
        or["canted_station_m"].as_f64(),
        or["station_m"].as_f64(),
    ) {
        check.canted[0] += 1;
        let shift = canted_shift(root, cant);
        if (turned - station - shift).abs() <= 1e-9 * shift.abs() {
            check.canted[1] += 1;
        }
    }
    let earlier = or["earlier_siblings_m"].as_f64();
    check.number(
        q("station_m"),
        placed.fore_station_m,
        rs["position"].as_f64(),
        or["station_m"].as_f64(),
        |theirs, or| {
            let earlier = earlier?;
            (earlier > 0.0 && same(theirs - or, earlier)).then_some(EARLIER_SIBLINGS)
        },
    );
}

/// A record's designs by the survey's key for each, with the record's own description of the run.
fn designs(record: &Value) -> BTreeMap<String, Value> {
    record["designs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|design| Some((key(design["file"].as_str()?), design.clone())))
        .collect()
}

/// The key a design is filed under: its path from the repository root, or its entry in the jar.
pub(crate) fn key(name: &str) -> String {
    let name = name.replace('\\', "/");
    match name.rsplit_once('!') {
        Some((_, entry)) => entry.trim_start_matches('/').to_owned(),
        None => name,
    }
}

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct GeometryTally {
    /// The record, by design, when the library run has been made.
    record: Option<(String, BTreeMap<String, Value>)>,
    compared_designs: usize,
    refused: usize,
    /// The files the record holds, and how many of them OpenRocket opened.
    recorded: [usize; 2],
    /// Designs the survey laid out that the record does not hold: the record is out of date.
    missing: Vec<String>,
    /// The content hashes of the designs compared, and the verdict totals over the first design
    /// of each: the reference library holds some files more than once.
    distinct: std::collections::BTreeSet<String>,
    distinct_total: [usize; 3],
    /// Nose profiles compared with OpenRocket's radii, and how many are hpr's.
    profiles: [usize; 2],
    /// Designs in the record that hpr reads but does not lay out, so has no geometry to compare.
    not_laid_out: Vec<String>,
    stale: Vec<String>,
    by_quantity: BTreeMap<String, [usize; 3]>,
    causes: BTreeMap<String, usize>,
    in_kept_parts: usize,
    not_stated: usize,
    canted: [usize; 2],
    /// Numbers OpenRocket gave, and how many of them are hpr's; then the same for the numbers
    /// hpr and RocketSerializer agree on.
    openrocket_same: [usize; 2],
    agreeing_openrocket_same: [usize; 2],
    unmatched: Vec<String>,
    extractor_errors: BTreeMap<String, usize>,
    hpr_apart: Vec<Value>,
}

impl GeometryTally {
    /// The tally for a survey of the library, with the library's record if it has been written.
    pub(crate) fn load(root: &std::path::Path, library: bool) -> Result<Self, String> {
        let path = root.join(LIBRARY);
        if !library || !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{LIBRARY}: {error}"))?;
        let record: Value =
            serde_json::from_str(&text).map_err(|error| format!("{LIBRARY}: {error}"))?;
        let by = record["rocketserializer"]
            .as_str()
            .unwrap_or("?")
            .to_owned();
        let designs = designs(&record);
        let opened = designs
            .values()
            .filter(|design| design["opens"] == true)
            .count();
        Ok(Self {
            recorded: [designs.len(), opened],
            record: Some((by, designs)),
            ..Self::default()
        })
    }

    /// Notes a design the survey read but could not lay out: not compared, and not missing.
    pub(crate) fn not_laid_out(&mut self, name: &str) {
        if let Some((_, designs)) = &mut self.record
            && designs.remove(&key(name)).is_some()
        {
            self.not_laid_out.push(key(name));
        }
    }

    /// Compares one design the survey laid out, when the record holds it, and returns the detail.
    pub(crate) fn add(
        &mut self,
        name: &str,
        bytes: &[u8],
        rocket: &Rocket,
        layout: &Layout,
    ) -> Value {
        let Some((_, designs)) = &mut self.record else {
            return Value::Null;
        };
        let Some(design) = designs.remove(&key(name)) else {
            self.missing.push(key(name));
            return Value::Null;
        };
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if design["sha256"].as_str() != Some(digest.as_str()) {
            self.stale.push(format!(
                "{} has changed since RocketSerializer was run on it",
                key(name)
            ));
            return Value::Null;
        }
        if design["opens"].as_bool() != Some(true) {
            self.refused += 1;
            return json!({ "openrocket_refuses": design["refused"] });
        }
        self.compared_designs += 1;
        let check = compare(&design, rocket, layout);
        let first_copy = self.distinct.insert(digest);
        self.profiles[0] += check.profiles[0];
        self.profiles[1] += check.profiles[1];
        for compared in &check.compared {
            if first_copy {
                self.distinct_total[compared.verdict as usize] += 1;
            }
            if let Some(same) = compared.openrocket_same {
                self.openrocket_same[0] += 1;
                self.openrocket_same[1] += usize::from(same);
                if compared.verdict == Verdict::Agree {
                    self.agreeing_openrocket_same[0] += 1;
                    self.agreeing_openrocket_same[1] += usize::from(same);
                }
            }
            let counts = self
                .by_quantity
                .entry(compared.quantity.clone())
                .or_default();
            counts[compared.verdict as usize] += 1;
            if compared.verdict == Verdict::RocketSerializerApart {
                *self
                    .causes
                    .entry(format!(
                        "{}: {}",
                        compared.quantity,
                        compared.cause.unwrap_or("no cause shown")
                    ))
                    .or_default() += 1;
            }
            if compared.verdict == Verdict::HprApart {
                self.hpr_apart.push(json!({
                    "file": key(name),
                    "quantity": compared.quantity,
                    "values": compared.detail,
                }));
            }
        }
        self.in_kept_parts += check.in_kept_parts;
        self.canted[0] += check.canted[0];
        self.canted[1] += check.canted[1];
        self.not_stated += check.not_stated;
        self.unmatched.extend(
            check
                .unmatched
                .iter()
                .map(|entry| format!("{}: {entry}", key(name))),
        );
        for error in &check.extractor_errors {
            *self.extractor_errors.entry(error.clone()).or_default() += 1;
        }
        json!({
            "compared": check.compared.iter().map(|compared| json!({
                "quantity": compared.quantity,
                "verdict": format!("{:?}", compared.verdict),
                "cause": compared.cause,
                "values": compared.detail,
            })).collect::<Vec<_>>(),
            "in_kept_parts": check.in_kept_parts,
            "not_stated": check.not_stated,
            "unmatched": check.unmatched,
            "extractor_errors": check.extractor_errors,
        })
    }

    pub(crate) fn print(&self) {
        let Some((by, _)) = &self.record else {
            println!(
                "  key geometry against RocketSerializer: not run; \
                 validation/oracles/rocketserializer/geometry.py writes {LIBRARY}"
            );
            return;
        };
        let total: [usize; 3] = self.by_quantity.values().fold([0; 3], |sum, counts| {
            [sum[0] + counts[0], sum[1] + counts[1], sum[2] + counts[2]]
        });
        println!(
            "  key geometry against {by}, OpenRocket 24.12 settling a difference ({LIBRARY}): \
             {} file(s) recorded, {} of them opened by OpenRocket; {} design(s) compared, {} \
             distinct by content",
            self.recorded[0],
            self.recorded[1],
            self.compared_designs,
            self.distinct.len()
        );
        println!(
            "    quantities: {} compared, {} agree, {} where RocketSerializer is not OpenRocket and \
             hpr is, {} where hpr is neither",
            total.iter().sum::<usize>(),
            total[0],
            total[1],
            total[2]
        );
        println!(
            "    over the distinct designs only: {} compared, {} agree, {} where RocketSerializer \
             is not OpenRocket and hpr is, {} where hpr is neither",
            self.distinct_total.iter().sum::<usize>(),
            self.distinct_total[0],
            self.distinct_total[1],
            self.distinct_total[2]
        );
        println!(
            "    nose profiles OpenRocket draws as hpr does, at a quarter, a half and three \
             quarters of the length: {} of {} (what settles a nose's shape word)",
            self.profiles[1], self.profiles[0]
        );
        println!(
            "    hpr's number is OpenRocket's too: {} of the {} OpenRocket gave, and {} of the {} \
             hpr and RocketSerializer agree on",
            self.openrocket_same[1],
            self.openrocket_same[0],
            self.agreeing_openrocket_same[1],
            self.agreeing_openrocket_same[0]
        );
        println!(
            "    by quantity, agree/RocketSerializer apart/hpr apart: {}",
            self.by_quantity
                .iter()
                .map(|(quantity, [a, b, c])| format!("{quantity} {a}/{b}/{c}"))
                .collect::<Vec<_>>()
                .join(", ")
        );
        if !self.causes.is_empty() {
            println!(
                "    RocketSerializer apart, by cause: {}",
                self.causes
                    .iter()
                    .map(|(cause, count)| format!("{cause} {count}"))
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
        println!(
            "    canted fin sets whose turned station OpenRocket puts (c/2)(1 - cos cant) aft of \
             the unturned one: {} of {}",
            self.canted[1], self.canted[0]
        );
        println!(
            "    not compared: {} entries in parts hpr keeps unread, {} quantities RocketSerializer \
             gives no value for, {} file(s) OpenRocket does not open ({} of them a design hpr lays \
             out), {} hpr does not lay out, extractor errors: {}",
            self.in_kept_parts,
            self.not_stated,
            self.recorded[0] - self.recorded[1],
            self.refused,
            self.not_laid_out.len(),
            if self.extractor_errors.is_empty() {
                "none".to_owned()
            } else {
                self.extractor_errors
                    .iter()
                    .map(|(error, count)| format!("{error} ({count})"))
                    .collect::<Vec<_>>()
                    .join("; ")
            }
        );
    }

    /// Why the survey fails on the geometry, if it does.
    pub(crate) fn failure(&self) -> Option<String> {
        let (_, left) = self.record.as_ref()?;
        let mut problems = self.stale.clone();
        problems.extend(
            self.missing.iter().map(|file| {
                format!("{file} lays out but is not in {LIBRARY}: run geometry.py again")
            }),
        );
        if self.canted[1] != self.canted[0] {
            problems.push(format!(
                "{} of {} canted fin sets are not turned about the middle of the root chord, which \
                 reading a station with the cant set to zero assumes",
                self.canted[0] - self.canted[1],
                self.canted[0]
            ));
        }
        // A design OpenRocket refused has nothing to compare, and hpr may refuse it too.
        problems.extend(
            left.iter()
                .filter(|(_, design)| design["opens"] == true)
                .map(|(file, _)| format!("{file} is in {LIBRARY} but the survey did not read it")),
        );
        problems.extend(
            self.unmatched
                .iter()
                .map(|entry| format!("hpr has no component for {entry}")),
        );
        problems.extend(self.hpr_apart.iter().map(|apart| {
            format!(
                "{} {}: {}",
                apart["file"].as_str().unwrap_or("?"),
                apart["quantity"],
                apart["values"]
            )
        }));
        (!problems.is_empty()).then(|| {
            format!(
                "the RocketSerializer cross-check fails:\n  {}",
                problems.join("\n  ")
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hpr_io::ork;
    use std::path::Path;

    /// The committed record of RocketSerializer and OpenRocket run on Loft's seven public demo
    /// designs, held to hpr's reading of the same files, so CI checks what the library run checks.
    #[test]
    fn rocketserializer_agrees_on_the_loft_demos() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let text = std::fs::read_to_string(root.join(PUBLIC)).unwrap();
        let record: Value = serde_json::from_str(&text).unwrap();
        let mut tally = GeometryTally {
            record: Some(("RocketSerializer".to_owned(), designs(&record))),
            ..GeometryTally::default()
        };
        for file in designs(&record).keys() {
            let bytes = std::fs::read(root.join(file)).unwrap();
            let read = ork::read(&bytes).unwrap();
            let spine = ork::rocket(&read.value.document);
            let layout = spine.value.layout().unwrap();
            tally.add(file, &bytes, &spine.value, &layout);
        }
        assert_eq!(tally.failure(), None);
        let total = tally.by_quantity.values().fold([0; 3], |sum, counts| {
            [sum[0] + counts[0], sum[1] + counts[1], sum[2] + counts[2]]
        });
        // Six designs open in OpenRocket (demo-quirks does not: a parallel stage directly under
        // the rocket). Every fin set is a tube's later child, so RocketSerializer's walk places
        // each one further aft by the parts before it, and hpr's station is OpenRocket's.
        assert_eq!((tally.compared_designs, tally.refused), (6, 1));
        assert_eq!(total, [74, 6, 0]);
        // And every one of hpr's 80 numbers is OpenRocket's, the 74 agreements included, with
        // each nose drawn as OpenRocket draws it.
        assert_eq!(
            (tally.openrocket_same, tally.agreeing_openrocket_same),
            ([80, 80], [74, 74])
        );
        assert_eq!(tally.profiles, [6, 6]);
        assert_eq!(
            tally.causes,
            BTreeMap::from([
                (format!("ellipticalfinset station_m: {EARLIER_SIBLINGS}"), 1),
                (format!("trapezoidfinset station_m: {EARLIER_SIBLINGS}"), 5),
            ])
        );
    }

    /// One design: a 0.3 m nose cone written as an ogive of parameter 0, on a tube with one
    /// trapezoidal fin set, and a record of RocketSerializer and OpenRocket for it.
    fn one_design(profile_radii_m: [f64; 3]) -> (Rocket, Layout, Value) {
        let xml = br#"<openrocket version="1.10" creator="test"><rocket><name>R</name>
            <subcomponents><stage><name>S</name><subcomponents>
            <nosecone><name>N</name><id>n</id><length>0.3</length><shape>ogive</shape>
            <shapeparameter>0</shapeparameter><aftradius>0.04</aftradius><thickness>0.002</thickness>
            </nosecone>
            <bodytube><name>T</name><id>t</id><length>0.6</length><thickness>0.002</thickness>
            <radius>0.04</radius><subcomponents>
            <trapezoidfinset><name>F</name><id>f</id><fincount>3</fincount>
            <position type="bottom">0.0</position><rootchord>0.1</rootchord><tipchord>0.05</tipchord>
            <sweeplength>0.05</sweeplength><height>0.06</height><thickness>0.003</thickness>
            </trapezoidfinset></subcomponents></bodytube>
            </subcomponents></stage></subcomponents></rocket></openrocket>"#;
        let read = ork::read(xml).unwrap();
        let spine = ork::rocket(&read.value.document);
        let layout = spine.value.layout().unwrap();
        let record = json!({
            "opens": true,
            "rocket_radius": { "value": 0.04 },
            "openrocket_largest_radius_m": 0.04,
            "nose": { "value": [{
                "id": "n", "name": "N", "in_kept_part": false,
                "rocketserializer": {
                    "kind": "ogive", "length": 0.3, "base_radius": 0.04, "position": 0.0,
                },
                "openrocket": {
                    "station_m": 0.0, "length_m": 0.3, "earlier_siblings_m": 0.0,
                    "base_radius_m": 0.04, "shape": "OGIVE", "shape_parameter": 0.0,
                    "profile_radii_m": profile_radii_m,
                },
            }]},
            "transitions": { "value": [] },
            "elliptical_fins": { "value": [] },
            "trapezoidal_fins": { "value": [{
                "id": "f", "name": "F", "in_kept_part": false,
                "rocketserializer": {
                    "number": 3, "root_chord": 0.1, "tip_chord": 0.05, "span": 0.06,
                    "sweep_length": 0.05, "cant_angle": 0.0, "position": 0.8,
                },
                "openrocket": {
                    "station_m": 0.8, "length_m": 0.1, "earlier_siblings_m": 0.0, "count": 3,
                    "root_chord_m": 0.1, "tip_chord_m": 0.05, "span_m": 0.06, "sweep_m": 0.05,
                    "cant_rad": 0.0,
                },
            }]},
        });
        (spine.value, layout, record)
    }

    fn verdict(check: &DesignCheck, quantity: &str) -> Option<Verdict> {
        check
            .compared
            .iter()
            .find(|compared| compared.quantity == quantity)
            .map(|compared| compared.verdict)
    }

    /// OpenRocket's turned station for a 0.495 m root at 1° of cant, from the jar's
    /// `Simulation extensions.ork`, less its unturned one.
    #[test]
    fn a_canted_fin_turns_about_the_middle_of_its_root() {
        let measured = 2.180_037_695_448_793_2 - 2.18;
        let shift = canted_shift(0.495, 1f64.to_radians());
        assert!(
            (shift - measured).abs() <= 1e-9 * measured,
            "{shift} against {measured}"
        );
    }

    /// A number hpr reads differently from both other readers fails; one RocketSerializer alone
    /// reads differently is counted apart, and so is a shape word when OpenRocket draws hpr's
    /// profile: an ogive of parameter 0 is the cone hpr reads, whatever the file calls it.
    #[test]
    fn a_difference_is_settled_by_openrocket() {
        let cone = [0.01, 0.02, 0.03];
        let (rocket, layout, record) = one_design(cone);
        let check = compare(&record, &rocket, &layout);
        assert!(check.unmatched.is_empty(), "{:?}", check.unmatched);
        assert_eq!(check.compared.len(), 12);
        assert_eq!(
            verdict(&check, "nosecone kind"),
            Some(Verdict::RocketSerializerApart)
        );
        assert!(
            check
                .compared
                .iter()
                .filter(|compared| compared.quantity != "nosecone kind")
                .all(|compared| compared.verdict == Verdict::Agree),
            "{:?}",
            check.compared
        );

        // Had OpenRocket drawn a tangent ogive, hpr's cone would be apart from both readers.
        let (rocket, layout, record) = one_design([0.0175, 0.03, 0.0375]);
        let check = compare(&record, &rocket, &layout);
        assert_eq!(verdict(&check, "nosecone kind"), Some(Verdict::HprApart));

        // A root chord both other readers put at 0.12 m is hpr's to explain.
        let (rocket, layout, mut record) = one_design(cone);
        record["trapezoidal_fins"]["value"][0]["rocketserializer"]["root_chord"] = json!(0.12);
        record["trapezoidal_fins"]["value"][0]["openrocket"]["root_chord_m"] = json!(0.12);
        let check = compare(&record, &rocket, &layout);
        assert_eq!(
            verdict(&check, "trapezoidfinset root_chord_m"),
            Some(Verdict::HprApart)
        );

        // RocketSerializer's walk, 0.3 m further aft for a part before the fins in their tube.
        let (rocket, layout, mut record) = one_design(cone);
        record["trapezoidal_fins"]["value"][0]["rocketserializer"]["position"] = json!(1.1);
        record["trapezoidal_fins"]["value"][0]["openrocket"]["earlier_siblings_m"] = json!(0.3);
        let check = compare(&record, &rocket, &layout);
        let station = check
            .compared
            .iter()
            .find(|compared| compared.quantity == "trapezoidfinset station_m")
            .unwrap();
        assert_eq!(
            (station.verdict, station.cause),
            (Verdict::RocketSerializerApart, Some(EARLIER_SIBLINGS))
        );
    }
}
