//! Every part of the bundled `.orc` catalogue built with the builder, against what OpenRocket
//! 24.12 builds from it.
//!
//! `tests/fixtures/orc/openrocket-built.json` is each part OpenRocket's preset loader returns for
//! each bundled file, applied to a new component of its kind
//! (`validation/oracles/openrocket/orc_built.py`): the component's mass and centre of mass, and
//! the dimensions the file leaves unsaid that OpenRocket chose. Each part is built here with its
//! `from_catalog` and added to a rocket, and its mass and centre as laid out are held to
//! OpenRocket's. The thresholds were set before measuring: a nose cone's or transition's mass
//! within 1e-3 of OpenRocket's and its centre within 1e-3 of its length; every other part within
//! 1e-12.
//!
//! The departures `hpr::rocket::catalog` names are counted, each shown to have its cause:
//!
//! - a hollow part's shoulders: OpenRocket's weigh nothing, hpr's have the part's wall. They are
//!   taken out of hpr's mass and centre in closed form, and the rest held to OpenRocket's.
//! - a hollow part's wall: hpr's is every point within its thickness of the surface, which the
//!   test checks against integrals of its own for cones, tangent ogives and ellipsoids.
//!   OpenRocket's masses and centres follow a wall whose inner radius at each station is
//!   `r − t √(1 + r′²)`, which the test integrates on its own over the same outer profile. The
//!   parts where the two walls differ by more than the threshold are counted.
//! - the one streamer that states a mass (OpenRocket leaves it unused), masses stated in ounces
//!   (OpenRocket's ounce), and the parts the builder refuses (a material the file doesn't define;
//!   a tube or ring no narrower inside than out), which OpenRocket weighs as zero.
//!
//! It also measures what the catalogue says about the shoulder wall hpr chooses: on the hollow,
//! shouldered parts that state their mass, whether the mass from the file's density is nearer the
//! stated one with the wall or without it.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;
use std::f64::consts::PI;

use hpr::hpr_design::{Density, DesignError, Part as DesignPart, PlacedComponent, Profile};
use hpr::hpr_io::orc::{BUNDLED_FILES, Part, PartKind, Shape, read};
use hpr::rocket::{Fitting, Nose, Transition, Tube, material};
use hpr::{CatalogProblem, Error, Rocket};
use serde_json::Value;

/// OpenRocket's ounce, kg, against the exact one (ADR-132 §4).
const OPENROCKET_OUNCE_KG: f64 = 0.028_349_523_1;
const OUNCE_KG: f64 = 0.028_349_523_125;

/// The thresholds, set before measuring: relative in mass, a fraction of the part's length in
/// its centre.
const REVOLVED: f64 = 1e-3;
const EXACT: f64 = 1e-12;
/// hpr's wall against an integral of its definition worked out here: the integrals' agreement.
const INTEGRAL: f64 = 1e-9;

fn fixture() -> Value {
    let text = include_str!("fixtures/orc/openrocket-built.json");
    serde_json::from_str(text).expect("the fixture is JSON")
}

/// Largest differences found and departures counted, by group.
#[derive(Default, Debug)]
struct Survey {
    /// Parts built and held to OpenRocket, by kind and by group.
    held: BTreeMap<&'static str, usize>,
    groups: BTreeMap<&'static str, usize>,
    /// Largest relative mass difference, by group.
    mass: BTreeMap<&'static str, f64>,
    /// Largest centre difference as a fraction of length, by group.
    centre: BTreeMap<&'static str, f64>,
    /// Hollow parts whose OpenRocket mass and centre follow the station-wise wall, and the
    /// largest relative difference in mass and in centre (of length).
    wall_rule_checked: usize,
    wall_rule_mass_checked: usize,
    wall_rule_mass: f64,
    wall_rule_centre: f64,
    /// Hollow nose cones whose body hpr weighs as an integral of its own wall gives it, by
    /// shape.
    exact_walls: BTreeMap<&'static str, usize>,
    /// Hollow parts whose wall differs from OpenRocket's by more than the threshold, and the
    /// largest mass and centre differences among them.
    wall_departures: usize,
    wall_departure_mass: f64,
    wall_departure_centre: f64,
    /// Of them, those stating their mass, which depart by their centre alone, and the largest
    /// centre difference among those.
    wall_departures_stated: usize,
    wall_departure_stated_centre: f64,
    /// Hollow parts with a shoulder, which hpr gives the part's wall; of them, those stating
    /// their mass.
    hollow_shoulders: usize,
    hollow_shoulders_stated: usize,
    /// Of those stating their mass: how many the file's density weighs nearer the stated mass
    /// with the wall than without, and the stated mass over each.
    wall_nearer: usize,
    ratios_with_wall: Vec<f64>,
    ratios_without: Vec<f64>,
    /// Streamers weighing their stated mass, where OpenRocket weighs their material.
    stated_streamers: usize,
    /// Stated masses that differ by OpenRocket's ounce alone.
    ounce_masses: usize,
    /// Parts refused for a material their file doesn't define.
    undefined_materials: usize,
    /// Parachutes built with lines of a material their file doesn't define, weightless.
    undefined_lines: usize,
    /// Parachutes whose file names no line material, built with weightless lines.
    no_lines: usize,
    /// Tube-like parts refused for a bore not narrower than the outside.
    no_bore: usize,
    /// Filled conical nose cones held to their closed-form mass.
    cones_closed_form: usize,
    /// Parts outside a threshold, each with its numbers.
    misses: Vec<String>,
}

fn kind_name(part: &Part) -> &'static str {
    match &part.kind {
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
        _ => panic!("a kind this test doesn't know"),
    }
}

/// The part built with the builder, laid out: the component made from it. A transition follows
/// a tube of its fore diameter; a fitting goes on a tube wider than it.
fn build(part: &Part) -> Result<PlacedComponent, Error> {
    let kraft = material("kraft_phenolic")?;
    let mut rocket;
    match &part.kind {
        PartKind::NoseCone(_) => {
            rocket = Rocket::new("catalogue", 0.1)?;
            rocket.add_nose(Nose::from_catalog(part)?)?;
        }
        PartKind::BodyTube(_) => {
            rocket = Rocket::new("catalogue", 0.1)?;
            rocket.add_tube(Tube::from_catalog(part)?)?;
        }
        PartKind::Transition(transition) => {
            rocket = Rocket::new("catalogue", transition.fore_outer_diameter_m)?;
            rocket.add_tube(Tube::new(0.1, 0.001, kraft))?;
            rocket.add_transition(Transition::from_catalog(part)?)?;
        }
        _ => {
            rocket = Rocket::new("catalogue", 0.5)?;
            rocket.add_tube(Tube::new(1.0, 0.001, kraft))?;
            rocket.add_fitting(Fitting::from_catalog(part)?)?;
        }
    }
    let layout = rocket.design().layout()?;
    let index = match &part.kind {
        PartKind::NoseCone(_) | PartKind::BodyTube(_) => 0,
        // After the tube and its children.
        _ => layout.components.len() - 1,
    };
    Ok(layout.components[index].clone())
}

/// The part's length along the axis, m, shoulders included.
fn length_m(part: &Part) -> f64 {
    match &part.kind {
        PartKind::BodyTube(t)
        | PartKind::TubeCoupler(t)
        | PartKind::EngineBlock(t)
        | PartKind::CenteringRing(t)
        | PartKind::LaunchLug(t) => t.length_m,
        PartKind::Bulkhead(b) => b.length_m,
        PartKind::NoseCone(n) => n.length_m + n.shoulder_length_m,
        PartKind::Transition(t) => t.length_m + t.fore_shoulder_length_m + t.aft_shoulder_length_m,
        PartKind::Parachute(_) | PartKind::Streamer(_) => 1.0,
        _ => panic!("a kind this test doesn't know"),
    }
}

/// A built revolved part's density, kg/m³: the file's, or scaled for a stated mass.
fn built_density(built: &PlacedComponent) -> f64 {
    let material = match &built.part {
        DesignPart::NoseCone(n) => &n.material,
        DesignPart::Transition(t) => &t.material,
        _ => panic!("only revolved parts are asked"),
    };
    match material.density {
        Density::Bulk { kg_m3 } => kg_m3,
        _ => panic!("a bulk material"),
    }
}

/// A hollow revolved part's wall thickness, m; `None` for a filled part or any other.
fn hollow_wall_m(part: &Part) -> Option<f64> {
    let (filled, thickness_m) = match &part.kind {
        PartKind::NoseCone(n) => (n.filled, n.thickness_m),
        PartKind::Transition(t) => (t.filled, t.thickness_m),
        _ => return None,
    };
    if filled == Some(true) {
        None
    } else {
        thickness_m
    }
}

/// A hollow part's shoulders, in closed form: each one's mass at `density`, kg, and its centre,
/// m aft of the part's fore end. Each is a tube of the part's wall (or solid where that is
/// thicker than its radius) open at its end. Empty for a filled part or one with no shoulder.
fn hollow_shoulders(part: &Part, density: f64) -> Vec<(f64, f64)> {
    let Some(wall) = hollow_wall_m(part) else {
        return Vec::new();
    };
    // (diameter, length, centre aft of the fore end)
    let shoulders = match &part.kind {
        PartKind::NoseCone(n) => vec![(
            n.shoulder_diameter_m,
            n.shoulder_length_m,
            n.length_m + 0.5 * n.shoulder_length_m,
        )],
        PartKind::Transition(t) => vec![
            (
                t.fore_shoulder_diameter_m,
                t.fore_shoulder_length_m,
                -0.5 * t.fore_shoulder_length_m,
            ),
            (
                t.aft_shoulder_diameter_m,
                t.aft_shoulder_length_m,
                t.length_m + 0.5 * t.aft_shoulder_length_m,
            ),
        ],
        _ => return Vec::new(),
    };
    shoulders
        .into_iter()
        .filter(|&(d, l, _)| d > 0.0 && l > 0.0)
        .map(|(d, l, at)| {
            let r = 0.5 * d;
            let inner = r - wall.min(r);
            (PI * (r * r - inner * inner) * l * density, at)
        })
        .collect()
}

/// A revolved part's outer profile, as the builder made it.
fn profile(built: &PlacedComponent) -> Profile {
    match &built.part {
        DesignPart::NoseCone(n) => Profile::nose(n.shape, n.length_m, n.base_radius_m),
        DesignPart::Transition(t) => Profile::transition(
            t.shape,
            t.length_m,
            t.fore_radius_m,
            t.aft_radius_m,
            t.clipped,
        ),
        _ => panic!("only revolved parts are asked"),
    }
    .expect("a built part's profile")
}

/// A hollow revolved part's body (shoulders aside) under the wall OpenRocket's masses follow:
/// at each station `x` the wall's inner radius is `r − t √(1 + r′²)`, the wall's normal thickness
/// taken across the station (or zero where that is past the axis). Its mass at `density`, kg,
/// and centre, m aft of the fore end, by the midpoint rule over 20,000 slices of the outer
/// profile hpr built.
fn station_wall(built: &PlacedComponent, wall_m: f64, density: f64) -> (f64, f64) {
    let profile = profile(built);
    let steps = 20_000;
    let dx = profile.length_m() / f64::from(steps);
    let (mut volume, mut moment) = (0.0, 0.0);
    for i in 0..steps {
        let x = (f64::from(i) + 0.5) * dx;
        let (r, slope) = profile.radius_and_slope(x);
        let inner = (r - wall_m * (1.0 + slope * slope).sqrt()).max(0.0);
        let area = PI * (r * r - inner * inner);
        volume += area * dx;
        moment += area * x * dx;
    }
    (volume * density, moment / volume)
}

/// A hollow nose cone's body (its shoulder aside) by an integral of hpr's wall, every point
/// within the wall's thickness `t` of the surface, worked out here on its own: its volume, m³,
/// and its first moment about the tip, m⁴, for the shapes where that has a form of its own;
/// `None` for the others.
///
/// - A cone of half-angle `α`: the wall's inner surface is the cone `t` inside it, its tip
///   `a = t / sin α` aft of the tip and its base radius `R − t / cos α`; the wall is the
///   difference of the two cones, each of volume `πR²L/3` with its centre `3L/4` from its tip.
/// - A tangent ogive: an arc of radius `ρ = (R² + L²)/2R` about a centre at the base, `ρ − R`
///   below the axis. The wall's inner surface is the arc of radius `ρ − t` about the same
///   centre, up to where it meets the axis; both are integrated by Simpson's rule over 2,000
///   intervals.
/// - An ellipse with `L ≥ R` and `t` under its least radius of curvature `R²/L`: the region
///   between it and its inner parallel curve, which runs from `(t, 0)` at the tip to
///   `(L, R − t)` at the base. With the ellipse `x = L(1 − cos θ)`, `r = R sin θ` and `n` the
///   length of its normal `(R cos θ, L sin θ)`, the inner curve is `x + tR cos θ / n`,
///   `r − tL sin θ / n`. The wall is the half ellipsoid (volume `2πR²L/3`, centre `5L/8` from
///   the tip) less the volume that curve encloses, by Simpson's rule in `θ` over 20,000
///   intervals.
fn exact_wall(part: &Part) -> Option<(&'static str, f64, f64)> {
    let PartKind::NoseCone(nose) = &part.kind else {
        return None;
    };
    let t = hollow_wall_m(part)?;
    let (l, r) = (nose.length_m, 0.5 * nose.outer_diameter_m);
    match nose.shape {
        Shape::Conical => {
            let alpha = r.atan2(l);
            let tip = t / alpha.sin();
            let inner_l = (l - tip).max(0.0);
            let inner_r = (r - t / alpha.cos()).max(0.0);
            let outer = PI * r * r * l / 3.0;
            let inner = PI * inner_r * inner_r * inner_l / 3.0;
            let moment = outer * 0.75 * l - inner * (tip + 0.75 * inner_l);
            Some(("cones", outer - inner, moment))
        }
        Shape::Ogive => {
            let rho = (r * r + l * l) / (2.0 * r);
            let below = rho - r;
            let arc = |radius: f64, x: f64| {
                ((radius * radius - (l - x) * (l - x)).max(0.0).sqrt() - below).max(0.0)
            };
            let area = |radius: f64| move |x: f64| PI * arc(radius, x) * arc(radius, x);
            let inner_rho = rho - t;
            let reach = (inner_rho * inner_rho - below * below).max(0.0).sqrt();
            let volume =
                simpson(0.0, l, 2_000, area(rho)) - simpson(l - reach, l, 2_000, area(inner_rho));
            let moment = simpson(0.0, l, 2_000, |x| x * area(rho)(x))
                - simpson(l - reach, l, 2_000, |x| x * area(inner_rho)(x));
            Some(("tangent ogives", volume, moment))
        }
        Shape::Ellipsoid if l >= r && t < r * r / l => {
            // The inner curve's station, radius and the rate of its station along θ.
            let inner = |theta: f64| {
                let (s, c) = theta.sin_cos();
                let n = (l * l * s * s + r * r * c * c).sqrt();
                let dn = (l * l - r * r) * s * c / n;
                let x = l * (1.0 - c) + t * r * c / n;
                let radius = r * s - t * l * s / n;
                let rate = l * s + t * r * (-s * n - c * dn) / (n * n);
                (x, radius, rate)
            };
            let cavity = |power: i32| {
                simpson(0.0, 0.5 * PI, 20_000, |theta| {
                    let (x, radius, rate) = inner(theta);
                    PI * radius * radius * rate * x.powi(power)
                })
            };
            let volume = 2.0 * PI * r * r * l / 3.0 - cavity(0);
            let moment = 5.0 * PI * r * r * l * l / 12.0 - cavity(1);
            Some(("ellipsoids", volume, moment))
        }
        _ => None,
    }
}

/// Simpson's rule for `f` over `[a, b]` in `steps` (even) intervals.
fn simpson(a: f64, b: f64, steps: u32, f: impl Fn(f64) -> f64) -> f64 {
    let h = (b - a) / f64::from(steps);
    let mut sum = f(a) + f(b);
    for i in 1..steps {
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * f(a + f64::from(i) * h);
    }
    sum * h / 3.0
}

/// A filled conical nose cone's mass, kg, from its closed-form volume, `πR²L/3` and its solid
/// shoulder's `πr²l`, at `density`; `None` for any other part.
fn filled_cone_kg(part: &Part, density: f64) -> Option<f64> {
    let PartKind::NoseCone(nose) = &part.kind else {
        return None;
    };
    if nose.shape != Shape::Conical || nose.filled != Some(true) {
        return None;
    }
    let r = 0.5 * nose.outer_diameter_m;
    let s = 0.5 * nose.shoulder_diameter_m;
    Some(PI * (r * r * nose.length_m / 3.0 + s * s * nose.shoulder_length_m) * density)
}

/// A tube-like part whose bore is no narrower than its outside.
fn no_bore(part: &Part) -> bool {
    match &part.kind {
        PartKind::TubeCoupler(t)
        | PartKind::EngineBlock(t)
        | PartKind::CenteringRing(t)
        | PartKind::LaunchLug(t)
        | PartKind::BodyTube(t) => t.inner_diameter_m >= t.outer_diameter_m,
        _ => false,
    }
}

fn revolved(part: &Part) -> bool {
    matches!(part.kind, PartKind::NoseCone(_) | PartKind::Transition(_))
}

/// Records `value` as its group's largest, refusing a NaN.
fn worst(map: &mut BTreeMap<&'static str, f64>, group: &'static str, value: f64) {
    assert!(value.is_finite(), "{group}: {value}");
    let entry = map.entry(group).or_insert(0.0);
    *entry = entry.max(value);
}

/// Whether `value` is within `tolerance`; a NaN is not.
fn within(value: f64, tolerance: f64) -> bool {
    value <= tolerance
}

/// The relative difference of `ours` from `theirs`; for two zeros, none.
fn apart(ours: f64, theirs: f64) -> f64 {
    if ours == theirs {
        0.0
    } else {
        (ours - theirs).abs() / theirs.abs()
    }
}

/// Holds one part to OpenRocket's record of it, or counts its departure.
#[allow(
    clippy::too_many_lines,
    reason = "one part's comparison, in the order the departures are told apart"
)]
fn compare(survey: &mut Survey, part: &Part, record: &Value, at: &str) {
    let theirs_kg = record
        .get("override_mass_kg")
        .unwrap_or(&record["mass_kg"])
        .as_f64()
        .expect("a mass");
    let theirs_cg_m = record["cg_m"].as_f64().expect("a centre");
    let built = match build(part) {
        Ok(built) => built,
        Err(Error::Catalog {
            problem: CatalogProblem::UndefinedMaterial(name),
            ..
        }) => {
            // OpenRocket finds no density for it, and weighs it as zero.
            assert_eq!(theirs_kg, 0.0, "{at}: {name}");
            survey.undefined_materials += 1;
            return;
        }
        Err(Error::Design(error)) if no_bore(part) => {
            // Refused for its bore, a tube's as a wall of no thickness, a ring's as a bore
            // reaching its outside. OpenRocket weighs it as zero.
            let for_bore = match &error {
                DesignError::Domain { what, .. } => *what == "wall thickness",
                DesignError::Geometry(why) => why.contains("bore"),
                _ => false,
            };
            assert!(for_bore, "{at}: {error}");
            assert_eq!(theirs_kg, 0.0, "{at}");
            survey.no_bore += 1;
            return;
        }
        Err(error) => panic!("{at}: the builder refused it: {error}"),
    };
    if let PartKind::Parachute(chute) = &part.kind
        && chute
            .line_material
            .as_ref()
            .is_some_and(|lines| lines.material().is_none())
    {
        survey.undefined_lines += 1;
    }
    if let PartKind::Parachute(chute) = &part.kind
        && chute.line_material.is_none()
    {
        survey.no_lines += 1;
    }
    let ours_kg = built.own.mass_kg;
    // OpenRocket's centre is metres aft of the component's fore end; ours is a station.
    let ours_cg_m = -built.own.cg_m.z - built.fore_station_m;
    let length = length_m(part);
    let wall = hollow_wall_m(part);
    let group = match (revolved(part), wall) {
        (true, Some(_)) => "hollow revolved",
        (true, None) => "filled revolved",
        (false, _) => "other",
    };
    let tolerance = if revolved(part) { REVOLVED } else { EXACT };

    // A hollow part's shoulders, at the density it was built with, taken out of its mass and
    // centre: what is left is its body, which OpenRocket's is.
    let density = if revolved(part) {
        built_density(&built)
    } else {
        0.0
    };
    let shoulders = hollow_shoulders(part, density);
    let shoulders_kg: f64 = shoulders.iter().map(|(kg, _)| kg).sum();
    let body_kg = ours_kg - shoulders_kg;
    let body_cg_m =
        (ours_kg * ours_cg_m - shoulders.iter().map(|(kg, x)| kg * x).sum::<f64>()) / body_kg;
    if !shoulders.is_empty() {
        assert!(body_kg > 0.0 && body_cg_m.is_finite(), "{at}");
    }

    let mass_off = if part.mass_kg.is_some() {
        0.0
    } else {
        apart(body_kg, theirs_kg)
    };
    // Whether the part's wall, OpenRocket's or hpr's, puts it outside a threshold.
    let mut departed = false;
    if let Some(wall_m) = wall {
        let wall = Wall {
            built: &built,
            wall_m,
            density,
            body_kg,
            body_cg_m,
            theirs_kg,
            theirs_cg_m,
            length_m: length,
        };
        let checked = hollow_wall(survey, part, &wall, at);
        let off = (body_cg_m - theirs_cg_m).abs() / length;
        if mass_off > tolerance || off > tolerance {
            // OpenRocket's wall is its station-wise one and hpr's its own, each shown above:
            // the difference is the two walls'. Where the mass departs, hpr's is the heavier:
            // checked, so a departure the other way shows.
            assert!(checked, "{at}: a departure with hpr's wall unchecked");
            assert!(
                mass_off <= tolerance || body_kg > theirs_kg,
                "{at}: OpenRocket's wall is the heavier"
            );
            departed = true;
            if part.mass_kg.is_some() {
                survey.wall_departures_stated += 1;
                survey.wall_departure_stated_centre = survey.wall_departure_stated_centre.max(off);
            }
            survey.wall_departures += 1;
            survey.wall_departure_mass = survey.wall_departure_mass.max(mass_off);
            survey.wall_departure_centre = survey.wall_departure_centre.max(off);
        }
    }

    if let Some(stated_kg) = part.mass_kg {
        // The part weighs its stated mass, from the density that gives it.
        assert!(
            apart(ours_kg, stated_kg) <= EXACT,
            "{at}: {ours_kg} kg built, {stated_kg} kg stated"
        );
        if let PartKind::Streamer(streamer) = &part.kind {
            let density = streamer.material.density.expect("defined");
            let area_kg = density * streamer.length_m * streamer.width_m;
            assert!(
                apart(theirs_kg, area_kg) <= EXACT,
                "{at}: OpenRocket weighs the material, {area_kg} kg; it has {theirs_kg} kg"
            );
            assert!(apart(ours_kg, theirs_kg) > 1e-3, "{at}");
            survey.stated_streamers += 1;
            return;
        }
        let ratio = theirs_kg / ours_kg;
        if (ratio - 1.0).abs() <= EXACT {
            worst(&mut survey.mass, "stated", (ratio - 1.0).abs());
        } else {
            assert!(
                (ratio - OPENROCKET_OUNCE_KG / OUNCE_KG).abs() <= EXACT,
                "{at}: {theirs_kg} kg in OpenRocket, {ours_kg} kg here, not an ounce apart"
            );
            survey.ounce_masses += 1;
        }
        if !shoulders.is_empty() {
            survey.hollow_shoulders_stated += 1;
            // At the file's density, with the shoulders' wall and without it: which weighs
            // nearer the stated mass?
            let file_kg = ours_kg * file_density(part) / density;
            let without_kg = file_kg - shoulders_kg * file_density(part) / density;
            let (with, without) = (stated_kg / file_kg, stated_kg / without_kg);
            survey.wall_nearer += usize::from((with - 1.0).abs() < (without - 1.0).abs());
            survey.ratios_with_wall.push(with);
            survey.ratios_without.push(without);
        }
    } else {
        let relative = mass_off;
        assert!(relative.is_finite(), "{at}: {body_kg} kg, {theirs_kg} kg");
        if !departed {
            if relative > tolerance {
                survey.misses.push(format!(
                    "{at}: {ours_kg} kg here less {shoulders_kg} kg of hollow shoulders, \
                     {theirs_kg} kg in OpenRocket: {relative:e} apart"
                ));
            }
            worst(&mut survey.mass, group, relative);
        }
        if let Some(cone_kg) = filled_cone_kg(part, density) {
            // A cone's volume is closed-form: hpr's is it, so a difference is OpenRocket's.
            assert!(
                apart(ours_kg, cone_kg) <= EXACT,
                "{at}: {ours_kg} kg here, {cone_kg} kg by the cone's and cylinder's volumes"
            );
            survey.cones_closed_form += 1;
        }
        if !shoulders.is_empty() {
            survey.hollow_shoulders += 1;
        }
    }
    // A parachute's or streamer's centre is where it is packed, not in the catalogue.
    if !departed && !matches!(part.kind, PartKind::Parachute(_) | PartKind::Streamer(_)) {
        let off = (body_cg_m - theirs_cg_m).abs() / length;
        if !within(off, tolerance) {
            survey.misses.push(format!(
                "{at}: centre {body_cg_m} m here, shoulders aside, {theirs_cg_m} m in \
                 OpenRocket: {off:e} of {length} m"
            ));
        }
        worst(&mut survey.centre, group, off);
    }
    *survey.held.entry(kind_name(part)).or_default() += 1;
    *survey.groups.entry(group).or_default() += 1;
}

/// A hollow part's wall, each side held to its own definition: OpenRocket's mass and centre to
/// the station-wise wall (`station_wall`), within the revolved parts' threshold; hpr's body to an
/// integral of its own wall where there is one (`exact_wall`).
struct Wall<'a> {
    built: &'a PlacedComponent,
    wall_m: f64,
    density: f64,
    body_kg: f64,
    body_cg_m: f64,
    theirs_kg: f64,
    theirs_cg_m: f64,
    length_m: f64,
}

/// Whether hpr's side was checked.
fn hollow_wall(survey: &mut Survey, part: &Part, wall: &Wall<'_>, at: &str) -> bool {
    let (rule_kg, rule_cg_m) = station_wall(wall.built, wall.wall_m, wall.density);
    // A stated mass is OpenRocket's whatever its wall: only its centre tells the wall.
    let mass = if part.mass_kg.is_some() {
        0.0
    } else {
        apart(wall.theirs_kg, rule_kg)
    };
    let centre = (wall.theirs_cg_m - rule_cg_m).abs() / wall.length_m;
    assert!(
        within(mass, REVOLVED) && within(centre, REVOLVED),
        "{at}: OpenRocket's {} kg at {} m, the station-wise wall's {rule_kg} kg at {rule_cg_m} m",
        wall.theirs_kg,
        wall.theirs_cg_m
    );
    survey.wall_rule_checked += 1;
    survey.wall_rule_mass_checked += usize::from(part.mass_kg.is_none());
    survey.wall_rule_mass = survey.wall_rule_mass.max(mass);
    survey.wall_rule_centre = survey.wall_rule_centre.max(centre);
    if let Some((shape, volume_m3, moment_m4)) = exact_wall(part) {
        // At the density hpr built with, stated mass or not.
        let exact_kg = volume_m3 * wall.density;
        let exact_cg_m = moment_m4 / volume_m3;
        assert!(
            within(apart(wall.body_kg, exact_kg), INTEGRAL)
                && within(
                    (wall.body_cg_m - exact_cg_m).abs() / wall.length_m,
                    INTEGRAL
                ),
            "{at}: {} kg at {} m here, {exact_kg} kg at {exact_cg_m} m by its wall's own \
             integral",
            wall.body_kg,
            wall.body_cg_m
        );
        *survey.exact_walls.entry(shape).or_default() += 1;
        true
    } else {
        false
    }
}

/// The density of a revolved part's material as its file gives it, kg/m³.
fn file_density(part: &Part) -> f64 {
    let material = match &part.kind {
        PartKind::NoseCone(n) => &n.material,
        PartKind::Transition(t) => &t.material,
        _ => panic!("only revolved parts are asked"),
    };
    material.density.expect("a defined material")
}

/// The dimensions OpenRocket chose for what the file leaves unsaid are the ones the builder
/// chooses (`hpr::rocket::catalog`).
fn unsaid_as_builder_chooses(part: &Part, record: &Value, at: &str) {
    let flag = |key: &str| record[key].as_bool().expect("a flag");
    let value = |key: &str| record[key].as_f64().expect("a number");
    let (shape, filled) = match &part.kind {
        PartKind::NoseCone(n) => (n.shape, n.filled),
        PartKind::Transition(t) => (t.shape, t.filled),
        PartKind::Parachute(_) => {
            assert_eq!(
                value("CD"),
                0.8,
                "{at}: OpenRocket's parachute drag coefficient"
            );
            return;
        }
        _ => return,
    };
    let parameter = match shape {
        Shape::Ogive | Shape::Parabolic => 1.0,
        Shape::Power => 0.5,
        Shape::Conical | Shape::Ellipsoid | Shape::Haack => 0.0,
        _ => panic!("a shape this test doesn't know"),
    };
    assert_eq!(value("ShapeParameter"), parameter, "{at}: shape parameter");
    if let Some(clipped) = record.get("Clipped") {
        let ours = matches!(shape, Shape::Ellipsoid | Shape::Haack | Shape::Power);
        assert_eq!(clipped.as_bool(), Some(ours), "{at}: clipped");
    }
    assert_eq!(flag("Filled"), filled == Some(true), "{at}: filled");
    for key in ["AftShoulderCapped", "ForeShoulderCapped"] {
        if record.get(key).is_some() {
            assert!(!flag(key), "{at}: {key}");
        }
    }
    // OpenRocket's shoulder walls: a filled part's its radius (solid), whatever its length, a
    // hollow part's zero.
    for (key, diameter_m) in shoulder_diameters(part) {
        if let Some(thickness) = record.get(key) {
            let expected = if filled == Some(true) && diameter_m > 0.0 {
                0.5 * diameter_m
            } else {
                0.0
            };
            assert_eq!(thickness.as_f64(), Some(expected), "{at}: {key}");
        }
    }
}

fn shoulder_diameters(part: &Part) -> Vec<(&'static str, f64)> {
    match &part.kind {
        PartKind::NoseCone(n) => vec![("AftShoulderThickness", n.shoulder_diameter_m)],
        PartKind::Transition(t) => vec![
            ("ForeShoulderThickness", t.fore_shoulder_diameter_m),
            ("AftShoulderThickness", t.aft_shoulder_diameter_m),
        ],
        _ => Vec::new(),
    }
}

/// The median of `values`.
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    assert!(n > 0, "a median of nothing");
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        0.5 * (sorted[n / 2 - 1] + sorted[n / 2])
    }
}

#[test]
fn every_catalogue_part_weighs_what_openrocket_builds() {
    let fixture = fixture();
    let files = fixture["files"].as_array().expect("files");
    let names: Vec<_> = files.iter().map(|file| file["file"].as_str()).collect();
    let bundled: Vec<_> = BUNDLED_FILES.iter().map(|(name, _)| Some(*name)).collect();
    assert_eq!(
        names, bundled,
        "the fixture covers every bundled file, in order"
    );
    let mut survey = Survey::default();
    let mut parts = 0;
    let mut stated = 0;
    for ((name, text), file) in BUNDLED_FILES.iter().zip(files) {
        let read = read(text, name).expect("every bundled file reads");
        let records = file["parts"].as_array().expect("parts");
        assert_eq!(read.catalog.parts.len(), records.len(), "{name}: parts");
        for (index, (part, record)) in read.catalog.parts.iter().zip(records).enumerate() {
            let at = format!("{name} part {index} ({})", part.part_number);
            assert_eq!(
                record["PartNo"].as_str(),
                Some(part.part_number.as_str()),
                "{at}"
            );
            unsaid_as_builder_chooses(part, record, &at);
            compare(&mut survey, part, record, &at);
            parts += 1;
            stated += usize::from(part.mass_kg.is_some());
        }
    }
    let (with, without) = (
        median(&survey.ratios_with_wall),
        median(&survey.ratios_without),
    );
    println!("{parts} parts: {survey:#?}\nmedians: {with} with the wall, {without} without");
    let held: usize = survey.held.values().sum();
    assert_eq!(parts, 3_449);
    assert_eq!(stated, 229, "parts stating their mass");
    assert_eq!(
        held + survey.undefined_materials + survey.no_bore + survey.stated_streamers,
        parts
    );
    assert!(survey.misses.is_empty(), "{:#?}", survey.misses);
    let groups: Vec<_> = survey.groups.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(
        groups,
        [
            ("filled revolved", 1_029),
            ("hollow revolved", 185),
            ("other", 2_230)
        ],
        "{survey:#?}"
    );
    // The largest differences, as `docs/the-builder.md` and ADR-133 quote them: the revolved
    // parts' to two digits, the rest by a bound, their last bits being the platform's.
    let printed = |value: f64| format!("{value:.1e}");
    let quoted = [
        printed(survey.mass["filled revolved"]),
        printed(survey.centre["filled revolved"]),
        printed(survey.mass["hollow revolved"]),
        printed(survey.centre["hollow revolved"]),
        printed(survey.wall_rule_mass),
        printed(survey.wall_rule_centre),
        printed(survey.wall_departure_mass),
        printed(survey.wall_departure_centre),
        printed(survey.wall_departure_stated_centre),
    ];
    assert_eq!(
        quoted,
        [
            "2.0e-4", "7.0e-5", "6.3e-4", "9.7e-4", "2.5e-4", "1.1e-4", "4.8e-3", "1.7e-3",
            "1.0e-3"
        ],
        "{survey:#?}"
    );
    assert!(survey.mass["other"] <= 1e-14, "{survey:#?}");
    assert!(survey.centre["other"] <= 1e-13, "{survey:#?}");
    assert!(survey.mass["stated"] <= 1e-15, "{survey:#?}");
    let counts = [
        survey.wall_rule_checked,
        survey.wall_rule_mass_checked,
        survey.wall_departures,
        survey.wall_departures_stated,
        survey.cones_closed_form,
        survey.hollow_shoulders,
        survey.hollow_shoulders_stated,
        survey.wall_nearer,
        survey.ounce_masses,
        survey.undefined_materials,
        survey.undefined_lines,
        survey.no_lines,
        survey.no_bore,
        survey.stated_streamers,
    ];
    assert_eq!(
        counts,
        [185, 111, 4, 1, 85, 67, 74, 46, 185, 1, 2, 6, 3, 1],
        "{survey:#?}"
    );
    let exact: Vec<_> = survey.exact_walls.iter().map(|(k, v)| (*k, *v)).collect();
    assert_eq!(
        exact,
        [("cones", 16), ("ellipsoids", 10), ("tangent ogives", 87)],
        "{survey:#?}"
    );
    assert_eq!(
        (format!("{with:.2}"), format!("{without:.2}")),
        ("0.97".into(), "1.30".into())
    );
}

#[test]
fn probes_weigh_what_openrocket_builds() {
    let fixture = fixture();
    let probes = fixture["probes"].as_array().expect("probes");
    let mut survey = Survey::default();
    for probe in probes {
        let label = probe["probe"].as_str().expect("a label");
        let read = read(probe["text"].as_str().expect("text"), label).expect("each probe reads");
        let records = probe["parts"].as_array().expect("parts");
        assert_eq!(read.catalog.parts.len(), records.len(), "{label}");
        for (part, record) in read.catalog.parts.iter().zip(records) {
            unsaid_as_builder_chooses(part, record, label);
            compare(&mut survey, part, record, label);
        }
    }
    println!("{survey:#?}");
    assert_eq!(survey.held.values().sum::<usize>(), 6);
    assert!(survey.misses.is_empty(), "{:#?}", survey.misses);
    let printed = |value: f64| format!("{value:.1e}");
    assert_eq!(
        (
            printed(survey.mass["filled revolved"]),
            printed(survey.centre["filled revolved"])
        ),
        ("1.5e-4".into(), "4.2e-5".into()),
        "{survey:#?}"
    );
}

/// The example's nose cone (`examples/catalog_rocket.rs`), as `docs/the-builder.md` quotes it:
/// OpenRocket's mass, hpr's, and the gap, which is its shoulder's wall.
#[test]
fn the_example_nose_weighs_its_shoulder_more() {
    let fixture = fixture();
    let loc = fixture["files"]
        .as_array()
        .expect("files")
        .iter()
        .find(|file| file["file"] == "loc_precision.orc")
        .expect("LOC Precision's file");
    let theirs = loc["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .find(|part| part["PartNo"] == "PNC-2.56")
        .expect("PNC-2.56");
    let theirs_kg = theirs["mass_kg"].as_f64().expect("a mass");
    let part = hpr::hpr_io::orc::bundled().find("LOC Precision", "PNC-2.56")[0];
    let built = build(part).expect("it builds");
    let shoulders_kg: f64 = hollow_shoulders(part, built_density(&built))
        .iter()
        .map(|(kg, _)| kg)
        .sum();
    let grams = |kg: f64| format!("{:.1}", 1000.0 * kg);
    assert_eq!(
        [
            grams(theirs_kg),
            grams(built.own.mass_kg),
            grams(shoulders_kg)
        ],
        ["61.5", "87.7", "26.1"]
    );
    assert!(apart(built.own.mass_kg - shoulders_kg, theirs_kg) <= REVOLVED);
}

#[test]
fn a_part_of_another_kind_is_refused() {
    let catalog = hpr::hpr_io::orc::bundled();
    let tube = catalog
        .parts
        .iter()
        .find(|part| matches!(part.kind, PartKind::BodyTube(_)))
        .expect("a body tube");
    let Err(Error::Catalog {
        problem: CatalogProblem::Kind { found, builder },
        ..
    }) = Nose::from_catalog(tube)
    else {
        panic!("a body tube isn't a nose cone");
    };
    assert_eq!((found, builder), ("body tube", "Nose::from_catalog"));
    assert!(Fitting::from_catalog(tube).is_err());
}
