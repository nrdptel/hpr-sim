//! Blunt and vertical nose tips faster than sound (M1.8e7, ADR-038): hpr's Newtonian cap ahead of
//! the second-order shock-expansion method against its two checks, written by `cargo xtask aero`
//! to `validation/fixtures/aero/blunt-tips.json`.
//!
//! - **TN D-4865's sphere-cone** (model 1: a 0.175-diameter nose radius on an 11.5° cone; the
//!   readings are committed in `tn-d-4865-sphere-cone.json`): hpr's `C_Nα` and centre of pressure
//!   at `α → 0` against the report's measured `C_N` and `C_m`, each fitted with a straight line
//!   over its plotted angles (0° to 12°), with the report's own method beside them.
//! - **The Arcas Robin's committed nose** (a power series with a vertical tip) on the cylinder
//!   and the boattail, the lip left off, through the flight's path (hpr's current body model),
//!   against TN D-4014's body alone as `arcas-robin-crossflow.json` fits it, beside the secant
//!   ogive fitted to the report's coordinates, which M1.8e6 compared.
//!
//! No targets: the fixture shows where hpr stands.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::blunt_tip::{
    CONE_TABLE_CAP_RAD, MAX_HANDOVER_RAD, handover_angle_capped_rad, handover_angle_rad,
    wedge_detachment_angle_rad,
};
use hpr_aero::crossflow::crossflow_factor;
use hpr_aero::shock_expansion::{
    BodySegment, DEFAULT_ELEMENTS_PER_CURVE, HandoverStart, ShockExpansionBody, ShockExpansionSlope,
};
use hpr_aero::{AeroModel, BodyModel};
use hpr_design::{NoseShape, Part, Profile, Rocket};
use serde_json::{Value, json};

use crate::aero_body::{
    ARCAS_NOSE_R_IN, INCH, arcas_model, arcas_model_without_lip, arcas_nose_ratio, flight_bodies,
};
use crate::aero_crossflow::{CP_DEG, MOMENT};
use crate::aero_gap::fit3;
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/blunt-tips.json";

/// The committed readings of TN D-4865's model 1.
pub const READINGS: &str = "validation/fixtures/aero/tn-d-4865-sphere-cone.json";

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

/// `[α°, value]` pairs as radians and values.
fn pairs(value: &Value, key: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    value[key]
        .as_array()
        .ok_or(format!("no `{key}`"))?
        .iter()
        .map(|p| match (p[0].as_f64(), p[1].as_f64()) {
            (Some(a), Some(c)) => Ok((a.to_radians(), c)),
            _ => Err(format!("a malformed `{key}` point")),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|p| p.into_iter().unzip())
}

pub fn generate(root: &Path) -> Result<Value, String> {
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_blunt.rs)",
        "note": "Blunt and vertical nose tips faster than sound (M1.8e7): TN D-4865's Newtonian cap \
                 and handover ahead of TN 3527's method, which starts behind it from the tangent \
                 cone's flow. sphere_cone: TN D-4865's model 1 (readings in \
                 tn-d-4865-sphere-cone.json), hpr's C_N_alpha on the base area and centre of \
                 pressure from the nose tip in base diameters at alpha -> 0, against the measured \
                 C_N and C_m fitted with a straight line (with an intercept) over the plotted \
                 angles 0 to 12 deg, and the report's own method fitted the same way through the \
                 origin's zero. arcas_robin: the committed design's power-series nose, cylinder and \
                 boattail, the lip left off, through the flight's path with hpr's current body \
                 model, fitted at the tunnel's plotted angles as arcas-robin-crossflow.json fits \
                 them (per radian on the body's cross-section), beside the fitted secant ogive's \
                 (M1.8e6's comparison) and the slopes at alpha -> 0. Errors are hpr's over the \
                 measured, minus 1; centre-of-pressure errors hpr's minus the measured, calibers. \
                 No targets.",
        "sphere_cone": sphere_cone_rows(root)?,
        "arcas_robin": arcas_robin(root)?,
        "starts": starts(root)?,
        "handover_caps": handover_caps(root)?,
        "coverage": coverage(root)?,
    }))
}

/// The Mach numbers the cap's reach is reported at.
pub const COVERAGE_MACHS: [f64; 4] = [1.25, 1.5, 2.0, 3.0];

/// How far back the cap reaches on a few noses: its end as a share of the nose's length and of
/// its base radius. The report's own sphere-cone beside them, and the Arcas Robin's committed
/// nose from its design.
fn coverage(root: &Path) -> Result<Value, String> {
    let readings = read(root, READINGS)?;
    let sphere_cone = sphere_cone_body(&readings)?;
    let sphere_cone_length = readings["model"]["drawn_length_over_base_diameter"]
        .as_f64()
        .ok_or("no drawn length")?;
    let design = COMMITTED_NOSE_DESIGN;
    let text = fs::read_to_string(root.join("validation/designs").join(design))
        .map_err(|e| format!("{design}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
    let Some(Part::NoseCone(nose)) = rocket
        .stages
        .first()
        .and_then(|s| s.components.first())
        .map(|c| &c.part)
    else {
        return Err(format!("{design}: its first component isn't a nose cone"));
    };
    let arcas = nose.profile().map_err(|e| e.to_string())?;
    let noses: [(&str, Profile, f64); 4] = [
        ("arcas robin, the committed nose", arcas, nose.base_radius_m),
        (
            "von Karman, five calibres",
            Profile::nose(NoseShape::VON_KARMAN, 5.0, 0.5).map_err(|e| e.to_string())?,
            0.5,
        ),
        (
            "power series n = 0.5, five calibres",
            Profile::nose(NoseShape::PowerSeries { exponent: 0.5 }, 5.0, 0.5)
                .map_err(|e| e.to_string())?,
            0.5,
        ),
        (
            "elliptical, two calibres",
            Profile::nose(NoseShape::Elliptical {}, 2.0, 0.5).map_err(|e| e.to_string())?,
            0.5,
        ),
    ];
    let mut rows = Vec::new();
    for (name, profile, base_radius_m) in noses {
        let body = ShockExpansionBody::new(
            &[
                BodySegment::Profile { profile },
                BodySegment::Cylinder {
                    length_m: 5.0,
                    radius_m: base_radius_m,
                },
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .map_err(|e| e.to_string())?;
        let mut by_mach = serde_json::Map::new();
        for mach in COVERAGE_MACHS {
            let value = match body.handover_m(mach).map_err(|e| e.to_string())? {
                Some(x) => json!({
                    "length_share": x / profile.length_m(),
                    "radius_share": profile.radius_m(x) / base_radius_m,
                }),
                None => json!({ "reaches": "the nose's end" }),
            };
            by_mach.insert(format!("{mach}"), value);
        }
        rows.push(json!({ "nose": name, "cap_ends_at": Value::Object(by_mach) }));
    }
    // The report's own model, for scale: its cap is the sphere, and it ends where the cone starts.
    let mut by_mach = serde_json::Map::new();
    for mach in COVERAGE_MACHS {
        // Below about Mach 1.4 the sphere is steeper than the handover all the way to the cone,
        // so the method doesn't hold there at all.
        let value = match sphere_cone.handover_m(mach) {
            Ok(Some(x)) => json!({
                "length_share": x / sphere_cone_length,
                "radius_share": (x * (2.0 * 0.175 - x)).sqrt() / 0.5,
            }),
            Ok(None) => json!({ "reaches": "no cap" }),
            Err(_) => json!({ "reaches": "past the sphere: the method doesn't hold" }),
        };
        by_mach.insert(format!("{mach}"), value);
    }
    rows.push(json!({
        "nose": "TN D-4865's sphere-cone (model 1)",
        "cap_ends_at": Value::Object(by_mach),
    }));
    Ok(json!(rows))
}

/// The caps on the handover slope the sweep reads, rad: the flown
/// [`MAX_HANDOVER_RAD`] (24°), two steps between, and the steepest cone the method's tables carry,
/// [`CONE_TABLE_CAP_RAD`] (30°). The ends are the library's own constants rather than
/// `24.0_f64.to_radians()` and `30.0_f64.to_radians()`, which are a bit away from them: the flown
/// column has to be the flown cap, and a cap a bit over the tables' is refused outright.
const CAPS_RAD: [f64; 4] = [
    MAX_HANDOVER_RAD,
    26.0 * PI / 180.0,
    28.0 * PI / 180.0,
    CONE_TABLE_CAP_RAD,
];

/// The element counts the committed nose's march is read at: the flown default, four times it,
/// and sixteen times.
const CAP_ELEMENTS: [usize; 3] = [
    DEFAULT_ELEMENTS_PER_CURVE,
    4 * DEFAULT_ELEMENTS_PER_CURVE,
    16 * DEFAULT_ELEMENTS_PER_CURVE,
];

/// A number of the cap sweep, rounded to six decimals.
///
/// Under a cap it does not fly, above Mach 4, the march reduces most of the nose's elements
/// (issue #108) and chains their loadings through the `λ₂/λ₁` ratio at every corner. That
/// amplifies the last bits of `exp` and `powf`, which a platform's library is free to round its
/// own way: the 30° cap's Mach 4.63 slope at 160 elements read 3.259763045663582 here and
/// 3.2597630456558506 on CI's Linux, 7.7e-12 apart and 2.4e-12 of it, where the fixture check
/// allows 1e-12 relative. Six
/// decimals is far more than the three the guide quotes and far less than the march can promise
/// there. The value must also sit clear of the rounding boundary, or two machines would round it
/// two ways and the check would flicker; `what` names it if it doesn't.
fn sweep_number(what: &str, x: f64) -> Result<f64, String> {
    let scaled = x * 1e6;
    if !scaled.is_finite() {
        return Err(format!("{what} isn't a number: {x}"));
    }
    // Clear of where the rounding turns over by a hundred times the drift measured (2.4e-12 of
    // the value), and by 1e-10 whatever the value: the closest any number of the sweep comes is
    // 1.7e-9, so the margin is not what decides anything today.
    let margin = (1e-10_f64).max(2.4e-10 * x.abs()) * 1e6;
    if (scaled - scaled.round()).abs() > 0.5 - margin {
        return Err(format!(
            "{what} = {x} sits on the sixth decimal's rounding boundary, where two machines \
             would round it two ways"
        ));
    }
    Ok(scaled.round() / 1e6)
}

/// The Mach number a cap starts to bind at: where the wedge's largest deflection reaches it,
/// bisected. The handover is the lesser of the two, so below this the cap costs nothing.
fn binds_from(cap_rad: f64) -> Result<f64, String> {
    let (mut low, mut high) = (1.0_f64, 20.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (low + high);
        if wedge_detachment_angle_rad(mid).map_err(|e| e.to_string())? < cap_rad {
            low = mid;
        } else {
            high = mid;
        }
    }
    Ok(0.5 * (low + high))
}

/// What the cap on a blunt tip's handover ([`hpr_aero::blunt_tip::MAX_HANDOVER_RAD`], 24° as
/// flown) is worth, and what stops it moving to the cone tables' 30° (M1.8e12, ADR-043).
///
/// For each cap in [`CAPS_RAD`]: the Mach number it starts to bind at, TN D-4865's sphere-cone
/// read under it (the report's own case, whose handover rule this is), and the committed Arcas
/// Robin nose's march read at [`CAP_ELEMENTS`] elements, with how many elements the march reduces
/// to the generalized method (`η < 0`, issue #81).
fn handover_caps(root: &Path) -> Result<Value, String> {
    let case = SphereConeCase::read(root)?;
    let area = 0.25 * PI;
    let mut sphere_cone = Vec::new();
    for row in case.rows()? {
        let mach = row["mach"].as_f64().ok_or("a row without `mach`")?;
        let (n_a, measured, measured_cp) = case.measured(row)?;
        let mut by_cap = serde_json::Map::new();
        for cap in CAPS_RAD {
            let body = case.body.clone().with_handover_cap_rad(cap);
            let hpr = body
                .slope(mach, area)
                .map_err(|e| format!("model 1 at Mach {mach} under {}°: {e}", cap_key(cap)))?;
            let (fitted, fitted_cp) = flown_fit(&hpr, &n_a, mach, case.fineness, case.planform);
            let at = |what: &str| {
                format!(
                    "the sphere-cone at Mach {mach} under {}°: {what}",
                    cap_key(cap)
                )
            };
            by_cap.insert(
                cap_key(cap),
                json!({
                    "handover_deg": sweep_number(
                        &at("the handover"),
                        handover_angle_capped_rad(mach, cap)
                            .map_err(|e| e.to_string())?
                            .to_degrees(),
                    )?,
                    "fitted_c_n_alpha": sweep_number(&at("the fitted slope"), fitted)?,
                    "fitted_c_n_alpha_error": sweep_number(
                        &at("the slope's error"),
                        fitted / measured - 1.0,
                    )?,
                    "cp_error_calibers": sweep_number(
                        &at("the centre of pressure's error"),
                        fitted_cp - measured_cp,
                    )?,
                }),
            );
        }
        sphere_cone.push(json!({ "mach": mach, "by_cap": Value::Object(by_cap) }));
    }

    let design = COMMITTED_NOSE_DESIGN;
    let (profile, tube_length, tube_radius, base_radius) = committed_nose(root, design)?;
    let nose_area = PI * base_radius * base_radius;
    let mut nose_rows = Vec::new();
    for mach in START_MACHS {
        let mut by_cap = serde_json::Map::new();
        for cap in CAPS_RAD {
            let mut by_count = serde_json::Map::new();
            for elements in CAP_ELEMENTS {
                let body = ShockExpansionBody::new(
                    &[
                        BodySegment::Profile { profile },
                        BodySegment::Cylinder {
                            length_m: tube_length,
                            radius_m: tube_radius,
                        },
                    ],
                    elements,
                )
                .map_err(|e| e.to_string())?
                .with_handover_cap_rad(cap);
                let at = |what: &str| {
                    format!(
                        "the committed nose at Mach {mach} under {}°, {elements} elements: {what}",
                        cap_key(cap)
                    )
                };
                let value = match (body.slope(mach, nose_area), body.reduced_elements(mach)) {
                    (Ok(slope), Ok(reduced)) => {
                        json!({
                            "c_n_alpha_per_rad": sweep_number(&at("the slope"), slope.slope_per_rad)?,
                            "cp_calibers": sweep_number(
                                &at("the centre of pressure"),
                                slope.centre_of_pressure_m / (2.0 * base_radius),
                            )?,
                            "reduced_elements": reduced,
                        })
                    }
                    (Err(e), _) | (_, Err(e)) => json!({ "fails": e.to_string() }),
                };
                by_count.insert(elements.to_string(), value);
            }
            by_cap.insert(
                cap_key(cap),
                json!({
                    "handover_deg": sweep_number(
                        &format!("the nose's handover at Mach {mach} under {}°", cap_key(cap)),
                        handover_angle_capped_rad(mach, cap)
                            .map_err(|e| e.to_string())?
                            .to_degrees(),
                    )?,
                    "elements": Value::Object(by_count),
                }),
            );
        }
        nose_rows.push(json!({ "mach": mach, "by_cap": Value::Object(by_cap) }));
    }

    Ok(json!({
        "note": "What the cap on a blunt tip's handover slope is worth (M1.8e12). hpr hands over \
                 at the lesser of the wedge's largest deflection and the cap; 24 deg is flown, 30 \
                 deg is the steepest cone the method's normal-force tables carry. sphere_cone: TN \
                 D-4865's model 1, whose handover rule this is, fitted as `sphere_cone` above \
                 fits it. committed_nose: the Arcas Robin's committed power-series nose on the \
                 short model's cylinder, nothing aft, at alpha -> 0 per radian on its \
                 cross-section, read at 10, 40 and 160 elements; reduced_elements counts the \
                 elements the march reduces to the generalized method (eta < 0, issue #81). What \
                 the element count is worth is the spread of the three, which is not stored: it \
                 is a difference of nearly equal numbers, and the last digits of one of them \
                 move between machines. Every measured number here is rounded to six decimals for \
                 the same \
                 reason: where most of the nose is reduced the march chains its loadings through \
                 one ratio per corner, which amplifies the last bits of exp and powf, and those \
                 are a platform's to round. No targets.",
        "caps": CAPS_RAD.map(cap_key),
        "caps_rad": CAPS_RAD,
        "binds_from_mach": CAPS_RAD
            .iter()
            .map(|&c| {
                sweep_number(&format!("where {}° starts to bind", cap_key(c)), binds_from(c)?)
            })
            .collect::<Result<Vec<_>, _>>()?,
        "sphere_cone": { "rows": sphere_cone },
        "committed_nose": {
            "design": design,
            "body": "the committed nose and the short model's cylinder, nothing aft",
            "elements": CAP_ELEMENTS,
            "rows": nose_rows,
        },
    }))
}

/// A cap's key in the fixture and in the guide's column headings: its angle in degrees, rounded
/// to the tenth, so that a cap a bit away from a whole number still keys as one.
fn cap_key(cap_rad: f64) -> String {
    format!("{:.1}", cap_rad.to_degrees())
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

/// The committed nose's profile, the cylinder behind it and the base radius, from a design.
fn committed_nose(root: &Path, design: &str) -> Result<(Profile, f64, f64, f64), String> {
    let text = fs::read_to_string(root.join("validation/designs").join(design))
        .map_err(|e| format!("{design}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
    let stage = rocket.stages.first().ok_or(format!("{design}: no stage"))?;
    let (Some(Part::NoseCone(nose)), Some(Part::BodyTube(tube))) = (
        stage.components.first().map(|c| &c.part),
        stage.components.get(1).map(|c| &c.part),
    ) else {
        return Err(format!("{design}: not a nose and a body tube"));
    };
    Ok((
        nose.profile().map_err(|e| e.to_string())?,
        tube.length_m,
        tube.outer_radius_m,
        nose.base_radius_m,
    ))
}

/// The design both the starts and the cap sweep read: the Arcas Robin's short wind-tunnel model,
/// whose nose is the committed power series with a vertical tip.
const COMMITTED_NOSE_DESIGN: &str = "wind-tunnel-arcas-robin-short.json";

/// The Mach numbers the two starts are compared at: the tunnel's, Mach 3.5 between them, and 5.
pub const START_MACHS: [f64; 8] = [1.5, 1.8, 2.3, 2.96, 3.5, 3.96, 4.63, 5.0];

/// The two starts behind the cap ([`HandoverStart`]) on the Arcas Robin's committed nose and the
/// short model's cylinder at `α → 0`, per radian on its cross-section, with the default elements
/// and four times as many: the slope and how many elements the march reduces (issue #81), or why
/// it fails.
fn starts(root: &Path) -> Result<Value, String> {
    let design = COMMITTED_NOSE_DESIGN;
    let (profile, tube_length, tube_radius, base_radius) = committed_nose(root, design)?;
    let area = PI * base_radius * base_radius;
    let mut rows = Vec::new();
    for mach in START_MACHS {
        let mut row = serde_json::Map::new();
        row.insert("mach".into(), json!(mach));
        for (name, start) in [
            ("tangent_cone", HandoverStart::TangentCone),
            ("newtonian", HandoverStart::Newtonian),
        ] {
            let mut by_count = serde_json::Map::new();
            for elements in [DEFAULT_ELEMENTS_PER_CURVE, 4 * DEFAULT_ELEMENTS_PER_CURVE] {
                let body = ShockExpansionBody::new(
                    &[
                        BodySegment::Profile { profile },
                        BodySegment::Cylinder {
                            length_m: tube_length,
                            radius_m: tube_radius,
                        },
                    ],
                    elements,
                )
                .map_err(|e| e.to_string())?
                .with_handover_start(start);
                let value = match (body.slope(mach, area), body.reduced_elements(mach)) {
                    (Ok(slope), Ok(reduced)) => json!({
                        "c_n_alpha_per_rad": slope.slope_per_rad,
                        "cp_calibers": slope.centre_of_pressure_m / (2.0 * base_radius),
                        "reduced_elements": reduced,
                    }),
                    (Err(e), _) | (_, Err(e)) => json!({ "fails": e.to_string() }),
                };
                by_count.insert(elements.to_string(), value);
            }
            row.insert(name.into(), Value::Object(by_count));
        }
        rows.push(Value::Object(row));
    }
    Ok(json!({
        "design": design,
        "body": "the committed nose and the short model's cylinder, nothing aft",
        "rows": rows,
    }))
}

/// TN D-4865's model 1 as hpr builds it, one base diameter across: the sphere's cap to its
/// tangent with the cone, and the cone to the base.
pub fn sphere_cone_body(readings: &Value) -> Result<ShockExpansionBody, String> {
    let model = &readings["model"];
    let radius = model["nose_radius_over_base_diameter"]
        .as_f64()
        .ok_or("no nose radius")?;
    let half_angle = model["cone_half_angle_deg"]
        .as_f64()
        .ok_or("no cone half-angle")?
        .to_radians();
    let tangent_x = radius * (1.0 - half_angle.sin());
    let tangent_r = radius * half_angle.cos();
    let cone = Profile::transition(
        NoseShape::Conical {},
        (0.5 - tangent_r) / half_angle.tan(),
        tangent_r,
        0.5,
        false,
    )
    .map_err(|e| e.to_string())?;
    ShockExpansionBody::new(
        &[
            BodySegment::SphericalCap {
                radius_m: radius,
                length_m: tangent_x,
            },
            BodySegment::Profile { profile: cone },
        ],
        DEFAULT_ELEMENTS_PER_CURVE,
    )
    .map_err(|e| e.to_string())
}

/// The measurement's reading error in `C_N`, for the curved fits' standard errors: the plotting's
/// own, about 0.005 (the readings file's `reading`).
const READING: f64 = 0.005;

/// Model 1's planform (the side view's area, `∫ 2r dx`) over its base area, and its centroid from
/// the tip, in base diameters: the cap by the midpoint rule on `r = √(x(2R − x))`, the cone a
/// trapezoid.
fn sphere_cone_planform(radius: f64, half_angle: f64) -> (f64, f64) {
    let tangent_x = radius * (1.0 - half_angle.sin());
    let tangent_r = radius * half_angle.cos();
    let steps = 200_000;
    let h = tangent_x / f64::from(steps);
    let (mut area, mut moment) = (0.0, 0.0);
    for i in 0..steps {
        let x = (f64::from(i) + 0.5) * h;
        let width = 2.0 * (x * (2.0 * radius - x)).sqrt();
        area += width * h;
        moment += width * x * h;
    }
    let cone_length = (0.5 - tangent_r) / half_angle.tan();
    let (fore, aft) = (2.0 * tangent_r, 1.0);
    let cone_area = 0.5 * (fore + aft) * cone_length;
    let cone_centroid = tangent_x + cone_length * (fore + 2.0 * aft) / (3.0 * (fore + aft));
    area += cone_area;
    moment += cone_area * cone_centroid;
    (area / (0.25 * PI), moment / area)
}

/// TN D-4865's model 1 set up once: the body, the planform its body lift acts on, and the
/// reference length its pitching moments are on. [`sphere_cone_rows`] and [`handover_caps`] both
/// read it, so the two sections cannot drift apart in how they fit the report's measurements.
struct SphereConeCase {
    readings: Value,
    body: ShockExpansionBody,
    /// The planform's area over the base's, and its centroid in base diameters from the tip.
    planform: Planform,
    /// The body's length in base diameters: the reference length its `C_m` is on, and the
    /// fineness a flight takes for body lift.
    length_calibers: f64,
    fineness: f64,
}

/// A body's planform, which carries its body lift ([`crossflow_factor`]).
#[derive(Clone, Copy)]
struct Planform {
    /// The planform's area over the reference area.
    ratio: f64,
    /// Its centroid, base diameters aft of the tip.
    centroid_calibers: f64,
}

impl SphereConeCase {
    fn read(root: &Path) -> Result<Self, String> {
        let readings = read(root, READINGS)?;
        let body = sphere_cone_body(&readings)?;
        let length_calibers = readings["reference"]["length_over_base_diameter"]
            .as_f64()
            .ok_or(format!("{READINGS}: no reference length"))?;
        let model = &readings["model"];
        let radius = model["nose_radius_over_base_diameter"]
            .as_f64()
            .ok_or("no nose radius")?;
        let half_angle = model["cone_half_angle_deg"]
            .as_f64()
            .ok_or("no cone half-angle")?
            .to_radians();
        let (ratio, centroid_calibers) = sphere_cone_planform(radius, half_angle);
        let fineness = body.length_m();
        Ok(Self {
            readings,
            body,
            planform: Planform {
                ratio,
                centroid_calibers,
            },
            length_calibers,
            fineness,
        })
    }

    /// The report's rows.
    fn rows(&self) -> Result<&Vec<Value>, String> {
        self.readings["rows"]
            .as_array()
            .ok_or(format!("{READINGS}: no rows"))
    }

    /// The measured slope and centre of pressure of one row, each fitted with a straight line over
    /// the plotted angles, and those angles: `C_m` is about the nose tip on the body length,
    /// nose-up positive, so the centre of pressure sits `−C_m l / C_N` aft of the tip.
    fn measured(&self, row: &Value) -> Result<(Vec<f64>, f64, f64), String> {
        let (n_a, n_c) = pairs(row, "alpha_deg_c_n")?;
        let (m_a, m_c) = pairs(row, "alpha_deg_c_m")?;
        let slope_per_rad = slope(&n_a, &n_c);
        let cp = -slope(&m_a, &m_c) * self.length_calibers / slope_per_rad;
        Ok((n_a, slope_per_rad, cp))
    }
}

/// A body's slope and centre of pressure fitted as a flight flies it at the plotted angles
/// `alphas_rad`: the method's slope as `sin α`, and body lift, Jorgensen's
/// `η C_dn (A_plan/A_ref) sin² α` at the planform's centroid.
fn flown_fit(
    hpr: &ShockExpansionSlope,
    alphas_rad: &[f64],
    mach: f64,
    fineness: f64,
    planform: Planform,
) -> (f64, f64) {
    let (c_n, moments): (Vec<f64>, Vec<f64>) = alphas_rad
        .iter()
        .map(|&a| {
            let lift =
                crossflow_factor(fineness, mach * a.sin()) * planform.ratio * a.sin() * a.sin();
            let attached = hpr.slope_per_rad * a.sin();
            (
                attached + lift,
                attached * hpr.centre_of_pressure_m + lift * planform.centroid_calibers,
            )
        })
        .unzip();
    let fitted = slope(alphas_rad, &c_n);
    (fitted, slope(alphas_rad, &moments) / fitted)
}

fn sphere_cone_rows(root: &Path) -> Result<Value, String> {
    let case = SphereConeCase::read(root)?;
    let body = &case.body;
    let reports = body.clone().with_handover_start(HandoverStart::Newtonian);
    let area = 0.25 * PI;
    let mut rows = Vec::new();
    for row in case.rows()? {
        let mach = row["mach"].as_f64().ok_or("a row without `mach`")?;
        let (n_a, measured, measured_cp) = case.measured(row)?;
        let (_, n_c) = pairs(row, "alpha_deg_c_n")?;
        let ([_, zero_alpha, _], [_, zero_alpha_error, _], _) =
            fit3(&n_a, &n_c, |a| [1.0, a, a * a.abs()], READING)?;
        let ([_, zero_alpha_cubic, _], [_, zero_alpha_cubic_error, _], _) =
            fit3(&n_a, &n_c, |a| [1.0, a, a * a * a], READING)?;
        // The report's method passes through zero at zero angle.
        let (mut t_a, mut t_n) = pairs(row, "report_theory_alpha_deg_c_n")?;
        let (_, mut t_m) = pairs(row, "report_theory_alpha_deg_c_m")?;
        t_a.insert(0, 0.0);
        t_n.insert(0, 0.0);
        t_m.insert(0, 0.0);
        let theory = slope(&t_a, &t_n);
        let theory_cp = -slope(&t_a, &t_m) * case.length_calibers / theory;
        let hpr = body
            .slope(mach, area)
            .map_err(|e| format!("model 1 at Mach {mach}: {e}"))?;
        let newtonian = reports
            .slope(mach, area)
            .map_err(|e| format!("model 1 at Mach {mach}, the report's start: {e}"))?;
        let (fitted, fitted_cp) = flown_fit(&hpr, &n_a, mach, case.fineness, case.planform);
        let handover_x = body
            .handover_m(mach)
            .map_err(|e| e.to_string())?
            .ok_or("model 1 has no handover")?;
        let handover_deg = handover_angle_rad(mach)
            .map_err(|e| e.to_string())?
            .to_degrees();
        let numbers = [
            measured,
            measured_cp,
            zero_alpha,
            zero_alpha_cubic,
            theory,
            theory_cp,
            fitted,
            fitted_cp,
            newtonian.slope_per_rad,
        ];
        if numbers.iter().any(|x| !x.is_finite()) {
            return Err(format!("model 1 at Mach {mach}: a result isn't a number"));
        }
        rows.push(json!({
            "mach": mach,
            "measured": {
                "fitted_c_n_alpha": measured,
                "cp_calibers": measured_cp,
                "zero_alpha_c_n_alpha": zero_alpha,
                "zero_alpha_standard_error": zero_alpha_error,
                "zero_alpha_cubic_c_n_alpha": zero_alpha_cubic,
                "zero_alpha_cubic_standard_error": zero_alpha_cubic_error,
            },
            "report_method": {
                "fitted_c_n_alpha": theory,
                "fitted_c_n_alpha_error": theory / measured - 1.0,
                "cp_calibers": theory_cp,
            },
            "hpr": {
                "fitted_c_n_alpha": fitted,
                "fitted_c_n_alpha_error": fitted / measured - 1.0,
                "cp_calibers": fitted_cp,
                "cp_error_calibers": fitted_cp - measured_cp,
                "zero_alpha_c_n_alpha": hpr.slope_per_rad,
                "zero_alpha_cp_calibers": hpr.centre_of_pressure_m,
                "handover_deg": handover_deg,
                "handover_calibers": handover_x,
            },
            "hpr_newtonian_start": {
                "zero_alpha_c_n_alpha": newtonian.slope_per_rad,
            },
        }));
    }
    Ok(json!({
        "body": {
            "length_calibers": body.length_m(),
            "planform_over_base_area": case.planform.ratio,
            "planform_centroid_calibers": case.planform.centroid_calibers,
        },
        "rows": rows,
    }))
}

/// Where the committed nose's cap hands over at `mach`, as its radius over the base's.
fn handover_radius_ratio(root: &Path, design: &str, mach: f64) -> Result<f64, String> {
    let text = fs::read_to_string(root.join("validation/designs").join(design))
        .map_err(|e| format!("{design}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
    let Some(Part::NoseCone(nose)) = rocket
        .stages
        .first()
        .and_then(|s| s.components.first())
        .map(|c| &c.part)
    else {
        return Err(format!("{design}: its first component isn't a nose cone"));
    };
    let profile = nose.profile().map_err(|e| e.to_string())?;
    let body = ShockExpansionBody::new(&[BodySegment::Profile { profile }], 10)
        .map_err(|e| e.to_string())?;
    let x = body
        .handover_m(mach)
        .map_err(|e| e.to_string())?
        .ok_or(format!("{design}: its nose has no handover"))?;
    Ok(profile.radius_m(x) / nose.base_radius_m)
}

fn arcas_robin(root: &Path) -> Result<Value, String> {
    let reference = read(root, WIND_TUNNEL)?;
    let moment = read(root, MOMENT)?;
    let (ratio, _) = arcas_nose_ratio()?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
    let area = 0.25 * PI * diameter_m * diameter_m;
    let mut configurations = Vec::new();
    for configuration in reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?
    {
        let id = configuration["id"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `id`"))?;
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no design"))?;
        let committed = arcas_model_without_lip(root, design, BodyModel::CURRENT)?;
        let fitted = arcas_model(root, design, Some(ratio), true, BodyModel::CURRENT)?;
        let models: [(&str, &AeroModel); 2] = [("committed", &committed), ("fitted", &fitted)];
        if committed.supersonic_body().is_none() {
            return Err(format!("{id}: the committed nose doesn't fly the method"));
        }
        let moment_center_m = moment["moment_center_in"][id]
            .as_f64()
            .ok_or(format!("{MOMENT}: no moment center for {id}"))?
            * INCH;
        let moment_curves = moment["configurations"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c["id"] == id))
            .and_then(|c| c["cm_fins_off"].as_array())
            .ok_or(format!("{MOMENT} has no curves for {id}"))?;
        let mut rows = Vec::new();
        for curve in configuration["cn_alpha_fins_off"]
            .as_array()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no fins-off curves"))?
        {
            let mach = curve["mach"].as_f64().ok_or("a curve without `mach`")?;
            if mach < 1.5 {
                continue;
            }
            let low: Vec<(f64, f64)> = curve["alpha_deg_c_n"]
                .as_array()
                .ok_or("a curve without `alpha_deg_c_n`")?
                .iter()
                .map(|p| {
                    (
                        p[0].as_f64().unwrap_or(f64::NAN),
                        p[1].as_f64().unwrap_or(f64::NAN),
                    )
                })
                .collect();
            if low.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
                return Err(format!(
                    "{WIND_TUNNEL}: {id} at Mach {mach} has a bad point"
                ));
            }
            let alphas: Vec<f64> = low.iter().map(|p| p.0.to_radians()).collect();
            let measured: Vec<f64> = low.iter().map(|p| p.1).collect();
            let measured_slope = slope(&alphas, &measured);
            // The body's centre of pressure at low angles, as arcas-robin-crossflow.json takes it.
            let m_points: Vec<(f64, f64)> = moment_curves
                .iter()
                .find(|c| c["mach"].as_f64() == Some(mach))
                .and_then(|c| c["alpha_deg_c_m"].as_array())
                .ok_or(format!("{MOMENT}: no curve for {id} at Mach {mach}"))?
                .iter()
                .map(|p| {
                    (
                        p[0].as_f64().unwrap_or(f64::NAN),
                        p[1].as_f64().unwrap_or(f64::NAN),
                    )
                })
                .collect();
            let inner = |p: &&(f64, f64)| p.0.abs() <= CP_DEG;
            let (n_a, n_c): (Vec<f64>, Vec<f64>) = low
                .iter()
                .filter(inner)
                .map(|p| (p.0.to_radians(), p.1))
                .unzip();
            let (m_a, m_c): (Vec<f64>, Vec<f64>) = m_points
                .iter()
                .filter(inner)
                .map(|p| (p.0.to_radians(), p.1))
                .unzip();
            let measured_cp = moment_center_m / diameter_m - slope(&m_a, &m_c) / slope(&n_a, &n_c);
            if !(measured_slope.is_finite() && measured_cp.is_finite()) {
                return Err(format!(
                    "{id} at Mach {mach}: the measured slope or centre of pressure isn't a number"
                ));
            }
            let mut hpr = serde_json::Map::new();
            for (name, model) in models {
                let scale = model.reference_area_m2() / area;
                let c_n: Vec<f64> = low
                    .iter()
                    .map(|p| hpr_force(model, mach, p.0, true).map(|f| scale * f.0))
                    .collect::<Result<_, _>>()?;
                let fitted_slope = slope(&alphas, &c_n);
                let (_, at_zero, _) = flight_bodies(model, mach, area)?;
                let forces: Vec<(f64, f64)> = low
                    .iter()
                    .filter(inner)
                    .map(|p| hpr_force(model, mach, p.0, true))
                    .collect::<Result<_, _>>()?;
                let (normal, moments): (Vec<f64>, Vec<f64>) = forces.into_iter().unzip();
                let cp = slope(&n_a, &moments) / slope(&n_a, &normal) / diameter_m;
                if !(fitted_slope.is_finite() && at_zero.is_finite() && cp.is_finite()) {
                    return Err(format!("{id} at Mach {mach}: {name} isn't a number"));
                }
                hpr.insert(
                    name.to_string(),
                    json!({
                        "fitted_c_n_alpha": fitted_slope,
                        "fitted_c_n_alpha_error": fitted_slope / measured_slope - 1.0,
                        "zero_alpha_c_n_alpha": at_zero,
                        "cp_calibers": cp,
                        "cp_error_calibers": cp - measured_cp,
                    }),
                );
            }
            rows.push(json!({
                "mach": mach,
                "measured": {
                    "fitted_c_n_alpha": measured_slope,
                    "cp_calibers": measured_cp,
                    "points": [low.len(), n_a.len()],
                },
                "hpr": hpr,
                "handover_radius_ratio": handover_radius_ratio(root, design, mach)?,
            }));
        }
        configurations.push(json!({
            "id": id,
            "design": design,
            "rows": rows,
        }));
    }
    Ok(json!(configurations))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Mach numbers the cap sweep breaks at, which the guide turns on its side: the tunnel's
    /// fastest, and the fastest hpr's aerodynamics claim.
    const BREAKS_AT: [f64; 2] = [4.63, 5.0];

    #[test]
    fn committed_fixture_is_current() {
        let root = crate::designs::root().unwrap();
        let committed = read(&root, FIXTURE).unwrap();
        let fresh = generate(&root).unwrap();
        assert!(
            crate::designs::same(&committed, &fresh),
            "{FIXTURE} differs from `cargo xtask aero`: {}",
            crate::designs::difference(&committed, &fresh).unwrap_or_default()
        );
    }

    /// A number as the guide writes it: `digits` decimals, a Unicode minus.
    fn num(x: f64, digits: usize) -> String {
        format!("{x:.digits$}").replace('-', "−")
    }

    /// A signed percentage as the guide writes it.
    fn pct(x: f64) -> String {
        format!("{:+.1}%", 100.0 * x).replace('-', "−")
    }

    fn f(value: &Value, pointer: &str) -> f64 {
        value
            .pointer(pointer)
            .and_then(Value::as_f64)
            .unwrap_or_else(|| panic!("no {pointer}"))
    }

    /// `docs/physics/aero.md`'s *Blunt tips* holds the fixture's three tables, cell by cell.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        let mut rows = Vec::new();
        let sphere_cone = fixture["sphere_cone"]["rows"].as_array().unwrap();
        for r in sphere_cone {
            rows.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                num(f(r, "/measured/fitted_c_n_alpha"), 3),
                num(f(r, "/report_method/fitted_c_n_alpha"), 3),
                pct(f(r, "/report_method/fitted_c_n_alpha_error")),
                num(f(r, "/hpr/fitted_c_n_alpha"), 3),
                pct(f(r, "/hpr/fitted_c_n_alpha_error")),
                num(f(r, "/measured/cp_calibers"), 2),
                num(f(r, "/hpr/cp_calibers"), 2),
            ));
            rows.push(format!(
                "| {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                num(f(r, "/measured/zero_alpha_c_n_alpha"), 3),
                num(f(r, "/measured/zero_alpha_cubic_c_n_alpha"), 3),
                num(f(r, "/hpr/zero_alpha_c_n_alpha"), 3),
                num(f(r, "/hpr_newtonian_start/zero_alpha_c_n_alpha"), 3),
            ));
        }
        for c in fixture["arcas_robin"].as_array().unwrap() {
            let model = c["id"].as_str().unwrap().rsplit('-').next().unwrap();
            for r in c["rows"].as_array().unwrap() {
                rows.push(format!(
                    "| {model} | {} | {} | {} | {} | {} | {} | {} |",
                    f(r, "/mach"),
                    num(f(r, "/measured/fitted_c_n_alpha"), 3),
                    num(f(r, "/hpr/committed/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/committed/fitted_c_n_alpha_error")),
                    num(f(r, "/hpr/fitted/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/fitted/fitted_c_n_alpha_error")),
                    num(f(r, "/handover_radius_ratio"), 3),
                ));
            }
        }
        // A cell says how many of the march's elements were reduced, out of how many the body
        // was cut into: 5 of 10 is half the nose, 5 of 160 is nothing.
        let cell = |v: &Value, of: usize| match v.get("c_n_alpha_per_rad").and_then(Value::as_f64) {
            None => "fails".to_owned(),
            Some(slope) => {
                let reduced = v["reduced_elements"].as_u64().unwrap();
                if reduced > 0 {
                    format!("{} ({reduced} of {of} reduced)", num(slope, 3))
                } else {
                    num(slope, 3)
                }
            }
        };
        for r in fixture["starts"]["rows"].as_array().unwrap() {
            rows.push(format!(
                "| {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                cell(&r["tangent_cone"]["10"], DEFAULT_ELEMENTS_PER_CURVE),
                cell(&r["tangent_cone"]["40"], 4 * DEFAULT_ELEMENTS_PER_CURVE),
                cell(&r["newtonian"]["10"], DEFAULT_ELEMENTS_PER_CURVE),
                cell(&r["newtonian"]["40"], 4 * DEFAULT_ELEMENTS_PER_CURVE),
            ));
        }
        for r in fixture["handover_caps"]["sphere_cone"]["rows"]
            .as_array()
            .unwrap()
        {
            let by_cap = &r["by_cap"];
            let cells: Vec<String> = CAPS_RAD
                .iter()
                .map(|&cap| pct(f(&by_cap[cap_key(cap)], "/fitted_c_n_alpha_error")))
                .collect();
            rows.push(format!("| {} | {} |", f(r, "/mach"), cells.join(" | ")));
        }
        for r in fixture["handover_caps"]["committed_nose"]["rows"]
            .as_array()
            .unwrap()
        {
            let by_cap = &r["by_cap"];
            // The guide shows the two ends of the sweep; the fixture holds all four caps.
            let cells: Vec<String> = [CAPS_RAD[0], CAPS_RAD[CAPS_RAD.len() - 1]]
                .iter()
                .flat_map(|&cap| {
                    let elements = &by_cap[cap_key(cap)]["elements"];
                    [CAP_ELEMENTS[0], CAP_ELEMENTS[CAP_ELEMENTS.len() - 1]]
                        .map(|n| cell(&elements[n.to_string()], n))
                })
                .collect();
            rows.push(format!("| {} | {} |", f(r, "/mach"), cells.join(" | ")));
        }
        // The third table turns the sweep on its side at the two Mach numbers where it breaks:
        // a row per cap, the flown element count and sixteen times it.
        let nose_rows = fixture["handover_caps"]["committed_nose"]["rows"]
            .as_array()
            .unwrap();
        for (index, &cap) in CAPS_RAD.iter().enumerate() {
            let label = if index == 0 {
                format!("{}°, as flown", cap_key(cap))
            } else {
                format!("{}°", cap_key(cap))
            };
            let cells: Vec<String> = BREAKS_AT
                .iter()
                .flat_map(|mach| {
                    let row = nose_rows
                        .iter()
                        .find(|r| r["mach"].as_f64() == Some(*mach))
                        .unwrap_or_else(|| panic!("no row at Mach {mach}"));
                    let elements = &row["by_cap"][cap_key(cap)]["elements"];
                    [CAP_ELEMENTS[0], CAP_ELEMENTS[CAP_ELEMENTS.len() - 1]]
                        .map(|n| cell(&elements[n.to_string()], n))
                })
                .collect();
            rows.push(format!("| {label} | {} |", cells.join(" | ")));
        }
        for r in fixture["coverage"].as_array().unwrap() {
            let by_mach = &r["cap_ends_at"];
            let cells: Vec<String> = COVERAGE_MACHS
                .iter()
                .map(|mach| {
                    let cell = &by_mach[format!("{mach}")];
                    match cell.get("length_share").and_then(Value::as_f64) {
                        Some(length) => format!(
                            "{} / {}",
                            format!("{:.1}%", 100.0 * length).replace('-', "−"),
                            num(f(cell, "/radius_share"), 2)
                        ),
                        None => cell["reaches"].as_str().unwrap().to_owned(),
                    }
                })
                .collect();
            rows.push(format!(
                "| {} | {} |",
                r["nose"].as_str().unwrap(),
                cells.join(" | ")
            ));
        }

        // hpr's range like for like, which the section's opening quotes.
        let errors: Vec<f64> = sphere_cone
            .iter()
            .map(|r| f(r, "/hpr/fitted_c_n_alpha_error"))
            .collect();
        let (low, high) = errors
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &e| {
                (a.min(e), b.max(e))
            });
        rows.push(format!("hpr reads {} to {}", pct(low), pct(high)));
        let cap_sweep = fixture["handover_caps"]["sphere_cone"]["rows"]
            .as_array()
            .unwrap()
            .len()
            + START_MACHS.len()
            + CAPS_RAD.len();
        assert_eq!(
            rows.len(),
            2 * sphere_cone.len() + 11 + START_MACHS.len() + cap_sweep + 5 + 1
        );
        for row in rows {
            assert!(guide.contains(&row), "aero.md doesn't have the row `{row}`");
        }
    }
}
