//! What a marched flare is worth (M1.8e18): hpr's reading of NASA TN D-4865's model 2 — a blunt
//! 2.75° cone with an 18.5° flare — against the forces and moments its Fig. 8(b) plots, written by
//! `cargo xtask aero` to `validation/fixtures/aero/marched-flare.json`.
//!
//! M1.8e17 gave a flared body the shock-expansion method (ADR-047) and compared it with nothing
//! measured. This is that comparison, on the one flared body in the sources whose normal force and
//! pitching moment are printed: the readings are committed in `tn-d-4865-flared-cone.json`, and
//! hpr's `C_Nα` and centre of pressure are fitted the way `aero_blunt` fits the same report's
//! model 1, so the two sections of the same report are read the same way.

use std::f64::consts::PI;
use std::path::Path;

use hpr_aero::AeroError;
use hpr_aero::blunt_tip::CONE_TABLE_CAP_RAD;
use hpr_aero::shock_expansion::{
    BodySegment, DEFAULT_ELEMENTS_PER_CURVE, SegmentSlope, ShockExpansionBody,
    flare_corner_limit_rad,
};
use hpr_design::{NoseShape, Profile};
use serde_json::{Value, json};

use crate::aero_blunt::{Planform, flown_fit, pairs, read};
use crate::aero_gap::fit3;
use crate::aero_mach::slope;

pub const FIXTURE: &str = "validation/fixtures/aero/marched-flare.json";

/// The committed readings of TN D-4865's model 2, from its Fig. 8(b).
pub const READINGS: &str = "validation/fixtures/aero/tn-d-4865-flared-cone.json";

/// The measurement's reading error in `C_N`, for the zero-angle fits' standard errors: the
/// plotting's own, about 0.004 (the readings file's `reading`).
const READING: f64 = 0.004;

/// How model 2's drawing is closed. Its printed dimensions close on the length exactly — the nose
/// derived from its three radii is 0.3429960 long, and 0.3429960 + 0.743 + 0.523 = 1.6090 is the
/// printed total — but the two printed half-angles then carry the base to 1.0084 diameters
/// instead of 1.000. One printed number has to give.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Closure {
    /// Keep the nose, both half-angles, the total length 1.609 and the base 1.000 — every number
    /// the measured coefficients are normalised by, and every angle the flow turns through — and
    /// solve for the split of the remaining length between the cone and the flare. It moves the
    /// juncture 0.0146 diameters aft of the printed one and nothing else.
    OnTheBase,
    /// Keep the printed lengths and scale the whole body until its base is 1.000. Every angle and
    /// every length ratio is the drawing's; the body is then 1.5956 diameters long rather than the
    /// printed 1.609, so its `C_m` would be on a different reference length. Carried only to
    /// measure what the drawing's disagreement is worth.
    PrintedLengths,
    /// Keep the nose, the printed lengths, the printed total and the base, and give up the
    /// **flare's half-angle** instead — the printed 18.5° becomes whatever closes the base.
    ///
    /// The disagreement is the flare's alone: the cone closes on its own printed numbers
    /// (0.586 + 2 × 0.743 tan 2.75° = 0.65737 against the printed 0.657), while the flare does not
    /// (0.657 + 2 × 0.523 tan 18.5° = 1.0074 against 1.000). This is the closure worth carrying,
    /// because the flare's angle is what sets the corner's turn, the attachment test and most of
    /// the normal force — the other two closures both hold it at exactly 18.5°, so neither bounds
    /// it. It is not the one hpr flies: both half-angles are the report's *text* (printed p. 8),
    /// to three decimals, while the lengths are only its drawing.
    PrintedFlareAngle,
}

/// Model 2's geometry in base diameters: the base radius is 0.5, so a "metre" is a calibre and the
/// reference area is `π/4`.
struct Geometry {
    /// The nose's spherical radius, and the station and radius where its blend arc takes over.
    sphere_radius: f64,
    sphere_end_x: f64,
    sphere_end_r: f64,
    /// The blend arc's radius, its centre, and the `radius_ratio` that gives hpr's ogive
    /// transition that radius.
    arc_radius: f64,
    arc_centre_x: f64,
    arc_centre_r: f64,
    arc_ratio: f64,
    /// Where the 2.75° cone starts — the nose's end — and its radius there.
    nose_length: f64,
    nose_end_r: f64,
    /// The cone, the flare, and the juncture between them.
    cone_length: f64,
    juncture_r: f64,
    flare_length: f64,
    /// The flare's half-angle as this closure reads it, rad.
    flare_half_angle_rad: f64,
    /// The body's length and base radius.
    length: f64,
    base_r: f64,
}

impl Geometry {
    /// Model 2 from the printed dimensions in the readings file.
    fn read(readings: &Value, closure: Closure) -> Result<Self, String> {
        let number = |key: &str| {
            readings["model"][key]
                .as_f64()
                .ok_or(format!("{READINGS}: no `{key}`"))
        };
        let sphere_radius = number("nose_sphere_radius_over_base_diameter")?;
        let arc_radius = number("nose_second_arc_radius_over_base_diameter")?;
        let offset = number("nose_second_arc_centre_below_axis_over_base_diameter")?;
        let cone_half_angle_rad = number("cone_half_angle_deg")?.to_radians();
        let flare_half_angle_rad = number("flare_half_angle_deg")?.to_radians();
        let printed_cone = number("cone_length_over_base_diameter")?;
        let printed_flare = number("flare_length_over_base_diameter")?;
        let printed_length = number("drawn_length_over_base_diameter")?;

        // The blend arc's centre: the two nose arcs are internally tangent, so their centres are
        // `arc_radius − sphere_radius` apart, and the drawing puts that centre `offset` below the
        // axis. The sphere's centre is on the axis, `sphere_radius` aft of the tip.
        let gap = arc_radius - sphere_radius;
        if !(gap > 0.0 && gap > offset.abs()) {
            return Err(format!("{READINGS}: the nose's two arcs can't be tangent"));
        }
        let along = (gap * gap - offset * offset).sqrt();
        let arc_centre_x = sphere_radius + along;
        let arc_centre_r = -offset;
        // The join: on the line through the two centres, `arc_radius` from the blend arc's.
        let (ux, ur) = (
            (sphere_radius - arc_centre_x) / gap,
            (0.0 - arc_centre_r) / gap,
        );
        let sphere_end_x = arc_centre_x + arc_radius * ux;
        let sphere_end_r = arc_centre_r + arc_radius * ur;
        // The nose ends where the blend arc is tangent to the cone: along the cone's own outward
        // normal from the arc's centre.
        let nose_length = arc_centre_x - arc_radius * cone_half_angle_rad.sin();
        let nose_end_r = arc_centre_r + arc_radius * cone_half_angle_rad.cos();

        let (cone_length, flare_length, flare_half_angle_rad, scale) = match closure {
            Closure::OnTheBase => {
                let rest = printed_length - nose_length;
                let rise = 0.5 - nose_end_r - rest * cone_half_angle_rad.tan();
                let flare = rise / (flare_half_angle_rad.tan() - cone_half_angle_rad.tan());
                (rest - flare, flare, flare_half_angle_rad, 1.0)
            }
            Closure::PrintedLengths => {
                let base = nose_end_r
                    + printed_cone * cone_half_angle_rad.tan()
                    + printed_flare * flare_half_angle_rad.tan();
                (
                    printed_cone,
                    printed_flare,
                    flare_half_angle_rad,
                    0.5 / base,
                )
            }
            Closure::PrintedFlareAngle => {
                let juncture_r = nose_end_r + printed_cone * cone_half_angle_rad.tan();
                let angle = ((0.5 - juncture_r) / printed_flare).atan();
                (printed_cone, printed_flare, angle, 1.0)
            }
        };
        let juncture_r = nose_end_r + cone_length * cone_half_angle_rad.tan();
        let base_r = juncture_r + flare_length * flare_half_angle_rad.tan();
        let length = nose_length + cone_length + flare_length;

        // hpr's circular-arc ogive takes its arc radius as a ratio of the tangent ogive's for the
        // same length and rise: `ρ = ratio · (λ² + 1)/2` in units of the rise, with `λ` the length
        // over the rise (`hpr_design`'s `Arc::new`). Solve that for the drawing's `arc_radius`.
        let rise = nose_end_r - sphere_end_r;
        let arc_length = nose_length - sphere_end_x;
        let lambda = arc_length / rise;
        let arc_ratio = 2.0 * arc_radius / (rise * (lambda * lambda + 1.0));

        let mut geometry = Self {
            sphere_radius,
            sphere_end_x,
            sphere_end_r,
            arc_radius,
            arc_centre_x,
            arc_centre_r,
            arc_ratio,
            nose_length,
            nose_end_r,
            cone_length,
            juncture_r,
            flare_length,
            flare_half_angle_rad,
            length,
            base_r,
        };
        if scale != 1.0 {
            geometry.scale(scale);
        }
        Ok(geometry)
    }

    /// Scales every length, leaving every angle and every ratio alone.
    fn scale(&mut self, s: f64) {
        for x in [
            &mut self.sphere_radius,
            &mut self.sphere_end_x,
            &mut self.sphere_end_r,
            &mut self.arc_radius,
            &mut self.arc_centre_x,
            &mut self.arc_centre_r,
            &mut self.nose_length,
            &mut self.nose_end_r,
            &mut self.cone_length,
            &mut self.juncture_r,
            &mut self.flare_length,
            &mut self.length,
            &mut self.base_r,
        ] {
            *x *= s;
        }
    }

    /// The body the method marches: a spherical cap, the blend arc, the cone and the flare.
    fn segments(&self) -> Result<Vec<BodySegment>, String> {
        let arc = Profile::transition(
            NoseShape::Ogive {
                radius_ratio: self.arc_ratio,
            },
            self.nose_length - self.sphere_end_x,
            self.sphere_end_r,
            self.nose_end_r,
            false,
        )
        .map_err(|e| format!("model 2's blend arc: {e}"))?;
        let cone = Profile::transition(
            NoseShape::Conical {},
            self.cone_length,
            self.nose_end_r,
            self.juncture_r,
            false,
        )
        .map_err(|e| format!("model 2's cone: {e}"))?;
        let flare = Profile::transition(
            NoseShape::Conical {},
            self.flare_length,
            self.juncture_r,
            self.base_r,
            false,
        )
        .map_err(|e| format!("model 2's flare: {e}"))?;
        Ok(vec![
            BodySegment::SphericalCap {
                radius_m: self.sphere_radius,
                length_m: self.sphere_end_x,
            },
            BodySegment::Profile { profile: arc },
            BodySegment::Profile { profile: cone },
            BodySegment::Profile { profile: flare },
        ])
    }

    /// The body's planform — the side view's area `∫ 2r dx` over the base area, and its centroid
    /// aft of the tip — which carries its body lift. The two curved pieces by the midpoint rule,
    /// the two straight ones as trapezoids.
    fn planform(&self) -> Planform {
        let steps = 200_000;
        let (mut area, mut moment) = (0.0, 0.0);
        let mut strip = |width: f64, x: f64, h: f64| {
            area += width * h;
            moment += width * x * h;
        };
        let h = self.sphere_end_x / f64::from(steps);
        for i in 0..steps {
            let x = (f64::from(i) + 0.5) * h;
            strip(
                2.0 * (x * (2.0 * self.sphere_radius - x)).max(0.0).sqrt(),
                x,
                h,
            );
        }
        let arc_length = self.nose_length - self.sphere_end_x;
        let h = arc_length / f64::from(steps);
        for i in 0..steps {
            let x = self.sphere_end_x + (f64::from(i) + 0.5) * h;
            let dx = x - self.arc_centre_x;
            let r = self.arc_centre_r
                + (self.arc_radius * self.arc_radius - dx * dx)
                    .max(0.0)
                    .sqrt();
            strip(2.0 * r, x, h);
        }
        for (fore_x, length, fore_r, aft_r) in [
            (
                self.nose_length,
                self.cone_length,
                self.nose_end_r,
                self.juncture_r,
            ),
            (
                self.nose_length + self.cone_length,
                self.flare_length,
                self.juncture_r,
                self.base_r,
            ),
        ] {
            let (fore, aft) = (2.0 * fore_r, 2.0 * aft_r);
            let piece = 0.5 * (fore + aft) * length;
            area += piece;
            moment += piece * (fore_x + length * (fore + 2.0 * aft) / (3.0 * (fore + aft)));
        }
        Planform {
            ratio: area / (PI * self.base_r * self.base_r),
            centroid_calibers: moment / area,
        }
    }
}

/// What the method does with a body's flare at one Mach number.
#[derive(Clone, Copy)]
struct Corner {
    /// The Mach number of the flow the march delivers to the corner.
    mach: f64,
    /// The surface angle ahead of the corner, degrees.
    ahead_deg: f64,
    /// The steepest surface angle whose corner an attached shock can hold there, degrees: the
    /// corner's turn limit on that flow plus the surface's own angle, under the cone tables' 30°.
    limit_deg: f64,
    /// The flare's own surface angle, degrees.
    angle_deg: f64,
    /// Whether the flare was read drawn out, decided on the radians the march works in rather
    /// than on the degrees the fixture prints.
    drawn_out: bool,
    /// The length of the flare the method marched, calibers: the real one's where its corner is
    /// within the limit, and a longer one drawn to the limit where it is not.
    marched_length: f64,
    /// The steepest flare of these radii the march itself gets through, where that was asked for.
    march_edge: Option<MarchEdge>,
}

/// The steepest flare of a given pair of radii the march itself gets through at one Mach number:
/// the corner's **isentropic** turn running out, which is what stops a march (M1.8e14, ADR-045)
/// and is a different bound from the shock staying attached.
#[derive(Clone, Copy, PartialEq)]
enum MarchEdge {
    /// Bisected to `f64` resolution, degrees.
    At(f64),
    /// The march still holds at the cone tables' 30°, which is as far as an element has a tangent
    /// cone to relax toward: the edge is at least that, and is not measured here.
    AtLeastTheConeTableCap,
}

impl MarchEdge {
    /// The number to publish, or `None` where the search only reached its own cap.
    fn degrees(self) -> Option<f64> {
        match self {
            Self::At(deg) => Some(deg),
            Self::AtLeastTheConeTableCap => None,
        }
    }
}

/// What the method made of a flared body at one Mach number.
struct Marched {
    /// What the flare's corner was read as. `None` where the method declined the body before the
    /// corner was reached at all — a blunt nose whose cap can't hand the flow over, say.
    corner: Option<Corner>,
    /// The whole reading, or `None` with the reason where the method refused the row.
    read: Option<Reading>,
    /// Why it refused, where it did.
    refused: Option<String>,
}

/// One Mach number's reading, in the body's own units (a calibre, since the base diameter is 1).
struct Reading {
    /// The body's slope, per radian, and its centre of pressure aft of the vertex.
    slope_per_rad: f64,
    cp_m: f64,
    /// The flare's own share of that slope, per radian, and where that share acts.
    flare_slope_per_rad: f64,
    flare_station_m: f64,
}

/// What the method reads for a body whose **last segment is a conical flare**, the way a flight
/// reads one (`SupersonicFlare::Marched`, ADR-047): the body as drawn while its corner's shock
/// stays attached, one of the same radii **drawn out** to the steepest attached turn where it is
/// not, and the centre of pressure put back on the real flare at the same fraction along it.
///
/// This repeats, on a hand-built body, what `hpr-aero`'s private `SupersonicRun::shares` runs for
/// a flare: `hpr-design` has no spherical-cap nose, so model 2 cannot be flown through a `Rocket`
/// and the run that would do this is out of reach. `the_flare_is_read_as_the_model_reads_it`
/// pins the two against each other, share by share, on a flared body the design route *can*
/// express — above the limit and below it. That pins the arithmetic, not the physics: an error
/// shared by both, in the rule itself, passes by construction.
fn march(
    segments: &[BodySegment],
    mach: f64,
    reference_area_m2: f64,
    want_edge: bool,
) -> Result<Marched, String> {
    let (flare, ahead_segments) = segments
        .split_last()
        .ok_or("a body with no segments".to_string())?;
    let BodySegment::Profile { profile } = flare else {
        return Err("the body's last segment isn't a flare".to_string());
    };
    if !matches!(profile.shape(), NoseShape::Conical {}) {
        return Err("the body's last segment isn't a *conical* flare".to_string());
    }
    let (fore_radius_m, aft_radius_m) = (profile.fore_radius_m(), profile.aft_radius_m());
    if aft_radius_m <= fore_radius_m {
        return Err("the body's last segment doesn't widen".to_string());
    }
    let ahead = ShockExpansionBody::new(ahead_segments, DEFAULT_ELEMENTS_PER_CURVE)
        .map_err(|e| format!("the body ahead of the flare: {e}"))?;
    let aft = match ahead.aft_flow(mach) {
        Ok(aft) => aft,
        // The body ahead of the flare didn't march, so the corner was never reached: the method
        // declines the row, as it would in a flight.
        Err(AeroError::Unsupported(why)) => {
            return Ok(Marched {
                corner: None,
                read: None,
                refused: Some(rounded(&format!("the body ahead of the flare: {why}"))),
            });
        }
        Err(e) => return Err(format!("the flow reaching the corner at Mach {mach}: {e}")),
    };
    let rise_m = aft_radius_m - fore_radius_m;
    let angle_rad = (rise_m / profile.length_m()).atan();
    let turn_limit_rad = match flare_corner_limit_rad(aft.surface_mach) {
        Ok(limit) => limit,
        Err(AeroError::Unsupported(why)) => {
            return Ok(Marched {
                corner: None,
                read: None,
                refused: Some(rounded(&format!("the corner's limit: {why}"))),
            });
        }
        Err(e) => return Err(format!("the corner's limit at Mach {mach}: {e}")),
    };
    let angle_limit_rad = (turn_limit_rad + aft.angle_rad).min(CONE_TABLE_CAP_RAD);
    let drawn_out = angle_rad > angle_limit_rad;
    let corner = Corner {
        mach: aft.surface_mach,
        ahead_deg: aft.angle_rad.to_degrees(),
        limit_deg: angle_limit_rad.to_degrees(),
        angle_deg: angle_rad.to_degrees(),
        drawn_out,
        marched_length: if drawn_out {
            rise_m / angle_limit_rad.tan()
        } else {
            profile.length_m()
        },
        march_edge: match want_edge {
            true => Some(march_edge(
                ahead_segments,
                profile,
                mach,
                reference_area_m2,
            )?),
            false => None,
        },
    };
    let refuse = |why: String| {
        Ok(Marched {
            corner: Some(corner),
            read: None,
            refused: Some(rounded(&why)),
        })
    };
    if drawn_out && !(angle_limit_rad > 0.0 && angle_limit_rad.is_finite()) {
        return refuse(format!("no flare can be drawn to {angle_limit_rad} rad"));
    }
    let mut shares = match at_angle(
        ahead_segments,
        profile,
        corner.marched_length,
        mach,
        reference_area_m2,
    )? {
        Outcome::Read(shares) => shares,
        Outcome::Refused(why) => return refuse(why),
    };
    if drawn_out {
        // The length only enters the march: the centre of pressure stays on the real flare, at
        // the same fraction along it as the drawn-out one reads. The flare's fore station, aft of
        // the vertex, is the length of everything ahead of it.
        let fore_m = ahead.length_m();
        let index = shares.len() - 1;
        let share = shares[index];
        if share.slope_per_rad > 0.0 {
            let along =
                (share.moment_slope_m / share.slope_per_rad - fore_m) / corner.marched_length;
            shares[index].moment_slope_m =
                share.slope_per_rad * (fore_m + along * profile.length_m());
        }
    }
    // `SupersonicRun::shares` refuses a row unless **every** segment's share is positive with its
    // station on its own segment; nothing here has a boattail, which is the one exemption, so the
    // same gate applies to all four. A row that missed it would be published by this fixture and
    // refused by a flight.
    let mut fore_m = 0.0;
    for (index, (share, segment)) in shares.iter().zip(segments).enumerate() {
        let aft_m = fore_m + length_of(segment);
        let station_m = share.moment_slope_m / share.slope_per_rad;
        let slack = 1e-9 * aft_m;
        if !(share.slope_per_rad > 0.0 && station_m >= fore_m - slack && station_m <= aft_m + slack)
        {
            return refuse(format!(
                "segment {index}'s share is {} per rad at {station_m}, off its own {fore_m} to \
                 {aft_m}",
                share.slope_per_rad
            ));
        }
        fore_m = aft_m;
    }
    let slope_per_rad: f64 = shares.iter().map(|s| s.slope_per_rad).sum();
    let moment_m: f64 = shares.iter().map(|s| s.moment_slope_m).sum();
    if slope_per_rad <= 0.0 {
        return refuse(format!("the slope isn't positive ({slope_per_rad})"));
    }
    let own = shares[shares.len() - 1];
    Ok(Marched {
        corner: Some(corner),
        read: Some(Reading {
            slope_per_rad,
            cp_m: moment_m / slope_per_rad,
            flare_slope_per_rad: own.slope_per_rad,
            flare_station_m: own.moment_slope_m / own.slope_per_rad,
        }),
        refused: None,
    })
}

/// A refusal message with every number in it rounded, for committing.
///
/// The message is `hpr-aero`'s own, and it prints `f64`s in full. Those last digits differ between
/// macOS, Windows and Linux — the same refusal reads `Mach 1.5250956752494207` on one and
/// `Mach 1.525095675249415` on another — and a fixture's *numbers* are compared to 1e-12 relative
/// but its *strings* are compared as text. Six decimals is far more than the reading is worth and
/// far less than the platforms disagree at.
fn rounded(message: &str) -> String {
    let text: Vec<char> = message.chars().collect();
    let mut out = String::with_capacity(message.len());
    let mut i = 0;
    while i < text.len() {
        let start = i;
        if text[i] == '-' && text.get(i + 1).is_some_and(char::is_ascii_digit) {
            i += 1;
        }
        if !text[i].is_ascii_digit() {
            out.push(text[start]);
            i = start + 1;
            continue;
        }
        while text.get(i).is_some_and(|c| c.is_ascii_digit() || *c == '.') {
            i += 1;
        }
        if text.get(i).is_some_and(|c| *c == 'e' || *c == 'E') {
            let mut j = i + 1;
            if text.get(j).is_some_and(|c| *c == '-' || *c == '+') {
                j += 1;
            }
            if text.get(j).is_some_and(char::is_ascii_digit) {
                while text.get(j).is_some_and(char::is_ascii_digit) {
                    j += 1;
                }
                i = j;
            }
        }
        let token: String = text[start..i].iter().collect();
        match token.parse::<f64>() {
            Ok(x) if token.contains('.') && x.abs() >= 1e-4 => out.push_str(&format!("{x:.6}")),
            Ok(x) if token.contains('.') => out.push_str(&format!("{x:.3e}")),
            _ => out.push_str(&token),
        }
    }
    out
}

/// A segment's length, in the body's own units.
fn length_of(segment: &BodySegment) -> f64 {
    match segment {
        BodySegment::Profile { profile } => profile.length_m(),
        BodySegment::Cylinder { length_m, .. } | BodySegment::SphericalCap { length_m, .. } => {
            *length_m
        }
        _ => f64::NAN,
    }
}

/// What one march came to: the shares it read, or the method declining the body.
enum Outcome {
    Read(Vec<SegmentSlope>),
    Refused(String),
}

/// The method's shares for `ahead` and a flare of `flare`'s radii drawn `length_m` long. `Err` is
/// a body that couldn't be built at all; a method that declines the body it was given is
/// [`Outcome::Refused`], which is what `SupersonicRun::shares` returns `None` for.
fn at_angle(
    ahead: &[BodySegment],
    flare: &Profile,
    length_m: f64,
    mach: f64,
    reference_area_m2: f64,
) -> Result<Outcome, String> {
    let mut segments = ahead.to_vec();
    segments.push(BodySegment::Profile {
        profile: Profile::transition(
            NoseShape::Conical {},
            length_m,
            flare.fore_radius_m(),
            flare.aft_radius_m(),
            flare.clipped(),
        )
        .map_err(|e| format!("a flare {length_m} long: {e}"))?,
    });
    let shares = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE)
        .and_then(|body| body.segment_slopes(mach, reference_area_m2));
    match shares {
        Ok(shares) => Ok(Outcome::Read(shares)),
        Err(AeroError::Unsupported(why)) => Ok(Outcome::Refused(why)),
        Err(e) => Err(format!("a flare {length_m} long at Mach {mach}: {e}")),
    }
}

/// The steepest flare of `flare`'s radii the march itself gets through at `mach`, bisected to
/// `f64` resolution between a flare of 1e-4° and the cone tables' 30°.
///
/// The bisection assumes the march holds below the edge and fails above it, which is how a
/// corner's isentropic turn runs out; it is checked at both ends, and an end that disagrees is an
/// error rather than a number.
fn march_edge(
    ahead: &[BodySegment],
    flare: &Profile,
    mach: f64,
    reference_area_m2: f64,
) -> Result<MarchEdge, String> {
    let rise_m = flare.aft_radius_m() - flare.fore_radius_m();
    let marches = |deg: f64| -> Result<bool, String> {
        Ok(matches!(
            at_angle(
                ahead,
                flare,
                rise_m / deg.to_radians().tan(),
                mach,
                reference_area_m2,
            )?,
            Outcome::Read(_)
        ))
    };
    let (mut low, mut high) = (1e-4_f64, CONE_TABLE_CAP_RAD.to_degrees());
    if !marches(low)? {
        return Err(format!(
            "at Mach {mach} the march refuses even a flare of {low}°, so its edge isn't bracketed"
        ));
    }
    if marches(high)? {
        return Ok(MarchEdge::AtLeastTheConeTableCap);
    }
    for _ in 0..200 {
        let mid = 0.5 * (low + high);
        if mid <= low || mid >= high {
            break;
        }
        if marches(mid)? { low = mid } else { high = mid }
    }
    Ok(MarchEdge::At(low))
}

pub fn generate(root: &Path) -> Result<Value, String> {
    let readings = read(root, READINGS)?;
    let closed = rows(&readings, Closure::OnTheBase)?;
    let beside = beside_model_1(root, &closed)?;
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_flare.rs)",
        "note": "What a marched flare is worth (M1.8e18): hpr's reading of NASA TN D-4865's \
                 model 2 — a blunt 2.75 deg cone with an 18.5 deg flare — against the normal force \
                 and pitching moment its Fig. 8(b) plots (readings in tn-d-4865-flared-cone.json). \
                 hpr's C_N_alpha is on the base area and its centre of pressure is measured from \
                 the nose tip in base diameters. measured: the plotted circles fitted with a \
                 straight line (with an intercept) over the plotted angles 0 to 12 deg, as \
                 aero_blunt fits the same report's model 1; zero_alpha is the same points fitted \
                 with a curve and read at alpha -> 0. report_theory: the report's own method, \
                 fitted the same way through the origin's zero. hpr.flown is hpr as a flight flies \
                 it, fitted at those same angles with body lift included; hpr.zero_alpha is the \
                 method's own slope and centre of pressure at alpha -> 0, body lift left out. \
                 Errors are hpr's over the measured, minus 1; centre-of-pressure errors hpr's \
                 minus the measured, in calibers. corner: what the method did with the flare — the \
                 flow its march delivers to the corner, the steepest surface angle an attached \
                 shock holds there (limit_deg, itself capped at the cone tables' 30 deg), the \
                 steepest the march itself gets through (march_edge_deg, bisected to f64 \
                 resolution, and null with march_edge_at_the_cone_table_cap where the march still \
                 holds at that same 30 deg, so the edge is only known to be at least it), and the \
                 length of the flare it marched (longer than the real one where the flare is read \
                 drawn out, ADR-047). separated: whether the report's \
                 own shadowgraphs show the laminar boundary layer separating ahead of the juncture \
                 at that Mach number, which no attached-flow method models. printed_lengths and \
                 printed_flare_angle: the same rows for the two other ways of closing the drawing, \
                 which between them bound what its 0.84 percent disagreement is worth — the first \
                 keeps its printed lengths and scales the body to a 1.000 base, the second keeps \
                 its printed lengths and the base and gives up the flare's stated half-angle \
                 instead. No targets.",
        "geometry": geometry_section(&readings)?,
        "reads_from_mach": reads_from_mach(&readings)?,
        "rows": closed,
        "printed_lengths": rows(&readings, Closure::PrintedLengths)?,
        "printed_flare_angle": rows(&readings, Closure::PrintedFlareAngle)?,
        "beside_model_1": beside,
    }))
}

/// The Mach numbers swept to find where model 2's reading starts, and the step between them.
const SWEEP: (f64, f64, f64) = (1.05, 4.63, 0.005);

/// The slowest flow hpr has a reading for model 2 in: below it the flare's corner turns the flow
/// further than the march can take it isentropically, even drawn out to the steepest turn an
/// attached shock holds (ADR-045 and ADR-047 bound different things, and below their crossing the
/// march's is the tighter one).
///
/// A flare's march can refuse a *band* of Mach numbers rather than everything below a threshold
/// (`the_march_refuses_two_bands_of_flare_and_the_model_keeps_slender_body_theory` in
/// `hpr-aero`), so bisection alone would find some crossing and call it the first. This sweeps
/// [`SWEEP`] across the whole range the report plots, refuses to publish a number unless the
/// reading turns on exactly once, and only then bisects that one crossing to `f64` resolution.
fn reads_from_mach(readings: &Value) -> Result<Value, String> {
    let g = Geometry::read(readings, Closure::OnTheBase)?;
    let segments = g.segments()?;
    let area = PI * g.base_r * g.base_r;
    let reads = |mach: f64| -> Result<bool, String> {
        Ok(march(&segments, mach, area, false)?.read.is_some())
    };
    let (from, to, step) = SWEEP;
    let steps = ((to - from) / step).round() as u32;
    let mut crossings = Vec::new();
    let mut was = reads(from)?;
    if was {
        return Err(format!("model 2 already reads at Mach {from}"));
    }
    for i in 1..=steps {
        let mach = from + f64::from(i) * step;
        let now = reads(mach)?;
        if now != was {
            crossings.push((mach - step, mach, now));
            was = now;
        }
    }
    match crossings.as_slice() {
        [(low, high, true)] => {
            let (mut low, mut high) = (*low, *high);
            for _ in 0..200 {
                let mid = 0.5 * (low + high);
                if mid <= low || mid >= high {
                    break;
                }
                if reads(mid)? { high = mid } else { low = mid }
            }
            let below = march(&segments, low, area, true)?;
            let below_corner = below
                .corner
                .ok_or(format!("model 2 has no corner at Mach {low}"))?;
            Ok(json!({
                "note": format!(
                    "The slowest flow hpr reads model 2 in: below it the march refuses, so a \
                     flared body drops to slender-body theory however the flare is drawn out. The \
                     refusal is the corner's isentropic turn running out (ADR-045), not the shock \
                     detaching (ADR-047). Swept every {step} Mach from {from} to {to}, where the \
                     reading turns on exactly once, and that one crossing bisected to f64 \
                     resolution.",
                ),
                "mach": high,
                "swept": [from, to, step],
                "refused_just_below": below.refused,
                "wedge_limit_deg_just_below": below_corner.limit_deg,
                "march_edge_deg_just_below": below_corner.march_edge.and_then(MarchEdge::degrees),
            }))
        }
        other => Err(format!(
            "model 2's reading turns on and off {} time(s) between Mach {from} and {to}, not once: \
             {other:?}",
            other.len()
        )),
    }
}

/// The same report's model 1 — the sphere-cone of `blunt-tips.json`, read from the same figure,
/// in the same tunnel, fitted the same way, and **without a flare** — beside model 2, so that
/// what the flare itself costs can be told from what the rest of the body costs. The two are not
/// the same body with and without a flare: model 1 is an 11.5° cone on a 0.175-diameter nose
/// radius, 1.755 diameters long, and model 2 a 2.75° cone on a 0.257-diameter one, 1.609 long.
/// What they share is the report, the tunnel, the figure, the reading and the fit.
fn beside_model_1(root: &Path, flared: &Value) -> Result<Value, String> {
    let model_1 = crate::aero_blunt::sphere_cone_rows(root)?;
    let plain = model_1["rows"].as_array().ok_or("model 1 has no rows")?;
    let flared = flared.as_array().ok_or("model 2 has no rows")?;
    if plain.len() != flared.len() {
        return Err(format!(
            "model 1 has {} rows and model 2 has {}",
            plain.len(),
            flared.len()
        ));
    }
    let mut rows = Vec::new();
    for (one, two) in plain.iter().zip(flared) {
        let mach = one["mach"]
            .as_f64()
            .ok_or("model 1: a row without `mach`")?;
        if two["mach"].as_f64() != Some(mach) {
            return Err(format!("the two models' rows disagree at Mach {mach}"));
        }
        let plain_error = one["hpr"]["fitted_c_n_alpha_error"]
            .as_f64()
            .ok_or("model 1: a row without an error")?;
        let flared_error = two["hpr"]["fitted_c_n_alpha_error"].as_f64();
        rows.push(json!({
            "mach": mach,
            "model_1_error": plain_error,
            "model_2_error": flared_error,
            "the_flare_adds": flared_error.map(|e| e - plain_error),
            "model_1_cp_error_calibers": one["hpr"]["cp_error_calibers"],
            "model_2_cp_error_calibers": two["hpr"]["cp_error_calibers"],
            "model_1_report_method_error": one["report_method"]["fitted_c_n_alpha_error"],
            "model_2_report_method_error": two["report_method"]["fitted_c_n_alpha_error"],
        }));
    }
    Ok(json!({
        "note": "hpr's error in C_N_alpha on TN D-4865's model 1 (no flare, blunt-tips.json) and \
                 model 2 (an 18.5 deg flare), both fitted over the plotted angles with body lift \
                 in, and the difference in points. The report's own method is beside each. The two \
                 are different bodies, not one body with and without a flare, so the difference \
                 bounds what the flare costs rather than measuring it exactly.",
        "rows": rows,
    }))
}

/// The body hpr builds, and how the drawing was closed to build it.
fn geometry_section(readings: &Value) -> Result<Value, String> {
    let g = Geometry::read(readings, Closure::OnTheBase)?;
    let printed = Geometry::read(readings, Closure::PrintedLengths)?;
    let angled = Geometry::read(readings, Closure::PrintedFlareAngle)?;
    let segments = g.segments()?;
    let body = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE)
        .map_err(|e| format!("model 2's body: {e}"))?;
    let planform = g.planform();
    // The blend arc hpr draws against the circle the drawing dimensions, at every tenth of it.
    let arc = &segments[1];
    let BodySegment::Profile { profile } = arc else {
        return Err("model 2's blend arc isn't a profile".to_string());
    };
    let mut worst: f64 = 0.0;
    for i in 0..=100 {
        let along = profile.length_m() * f64::from(i) / 100.0;
        let (r, _) = profile.radius_and_slope(along);
        let dx = g.sphere_end_x + along - g.arc_centre_x;
        let want = g.arc_centre_r + (g.arc_radius * g.arc_radius - dx * dx).max(0.0).sqrt();
        worst = worst.max((r - want).abs());
    }
    Ok(json!({
        "note": "Model 2 in base diameters, from Fig. 3(b)'s printed numbers. The nose is a \
                 sphere blended into the 2.75 deg cone by a second arc, and its three printed \
                 radii fix where each piece ends. The drawing closes on the printed length but \
                 carries the base to 1.0084 diameters, so hpr keeps the nose, both half-angles, \
                 the length and the base and solves for the split of the rest between the cone \
                 and the flare; printed_lengths keeps the printed lengths instead and scales the \
                 whole body to a 1.000 base.",
        "nose": {
            "sphere_radius": g.sphere_radius,
            "sphere_ends_at": g.sphere_end_x,
            "sphere_ends_at_radius": g.sphere_end_r,
            "blend_arc_radius": g.arc_radius,
            "blend_arc_centre": [g.arc_centre_x, g.arc_centre_r],
            "blend_arc_ogive_radius_ratio": g.arc_ratio,
            "blend_arc_worst_radius_error": worst,
            "length": g.nose_length,
            "ends_at_diameter": 2.0 * g.nose_end_r,
        },
        "closed_on_the_base": {
            "cone_length": g.cone_length,
            "flare_length": g.flare_length,
            "juncture_at": g.nose_length + g.cone_length,
            "juncture_diameter": 2.0 * g.juncture_r,
            "base_diameter": 2.0 * g.base_r,
            "length": g.length,
        },
        "printed_lengths": {
            "cone_length": printed.cone_length,
            "flare_length": printed.flare_length,
            "juncture_at": printed.nose_length + printed.cone_length,
            "juncture_diameter": 2.0 * printed.juncture_r,
            "base_diameter": 2.0 * printed.base_r,
            "length": printed.length,
            "scale": printed.length / g.length,
        },
        "printed_flare_angle": {
            "cone_length": angled.cone_length,
            "flare_length": angled.flare_length,
            "flare_half_angle_deg": angled.flare_half_angle_rad.to_degrees(),
            "juncture_at": angled.nose_length + angled.cone_length,
            "juncture_diameter": 2.0 * angled.juncture_r,
            "base_diameter": 2.0 * angled.base_r,
            "length": angled.length,
        },
        "planform_over_base_area": planform.ratio,
        "planform_centroid": planform.centroid_calibers,
        "fineness": body.length_m() / (2.0 * g.base_r),
    }))
}

/// One row per Mach number the report plots.
fn rows(readings: &Value, closure: Closure) -> Result<Value, String> {
    let g = Geometry::read(readings, closure)?;
    let segments = g.segments()?;
    let planform = g.planform();
    let area = PI * g.base_r * g.base_r;
    let calibers = 2.0 * g.base_r;
    let fineness = g.length / calibers;
    let length_calibers = readings["reference"]["length_over_base_diameter"]
        .as_f64()
        .ok_or(format!("{READINGS}: no reference length"))?;
    // The Mach number from which the report's own shadowgraphs show the laminar boundary layer
    // separating ahead of the flare's juncture, so that the measured rows are a separated flare
    // and no attached-flow method models them. The readings file carries the report's words for
    // it beside the number, so the two cannot drift.
    let separates_from = readings["report_says"]["separates_from_mach"]
        .as_f64()
        .ok_or(format!("{READINGS}: no separation Mach number"))?;
    let mut rows = Vec::new();
    for row in readings["rows"]
        .as_array()
        .ok_or(format!("{READINGS}: no rows"))?
    {
        let mach = row["mach"].as_f64().ok_or("a row without `mach`")?;
        let (n_a, n_c) = pairs(row, "alpha_deg_c_n")?;
        let (m_a, m_c) = pairs(row, "alpha_deg_c_m")?;
        let measured = slope(&n_a, &n_c);
        let measured_cp = -slope(&m_a, &m_c) * length_calibers / measured;
        let ([_, zero_alpha, _], [_, zero_alpha_error, _], _) =
            fit3(&n_a, &n_c, |a| [1.0, a, a * a.abs()], READING)?;
        // The report's method passes through zero at zero angle.
        let (mut t_a, mut t_n) = pairs(row, "report_theory_alpha_deg_c_n")?;
        let (_, mut t_m) = pairs(row, "report_theory_alpha_deg_c_m")?;
        t_a.insert(0, 0.0);
        t_n.insert(0, 0.0);
        t_m.insert(0, 0.0);
        let theory = slope(&t_a, &t_n);
        let theory_cp = -slope(&t_a, &t_m) * length_calibers / theory;

        let marched = march(&segments, mach, area, true)?;
        let corner = marched
            .corner
            .ok_or(format!("model 2 at Mach {mach}: no corner was reached"))?;
        let hpr = marched.read.as_ref().map(|r| {
            // The march works in the body's own units; the centre of pressure is in calibers.
            let hpr_cp = r.cp_m / calibers;
            let (fitted, fitted_cp) =
                flown_fit(r.slope_per_rad, hpr_cp, &n_a, mach, fineness, planform);
            json!({
                "zero_alpha_c_n_alpha": r.slope_per_rad,
                "zero_alpha_cp_calibers": hpr_cp,
                "zero_alpha_error": r.slope_per_rad / zero_alpha - 1.0,
                "fitted_c_n_alpha": fitted,
                "fitted_c_n_alpha_error": fitted / measured - 1.0,
                "cp_calibers": fitted_cp,
                "cp_error_calibers": fitted_cp - measured_cp,
                "flare_share_c_n_alpha": r.flare_slope_per_rad,
                "flare_share_of_the_body": r.flare_slope_per_rad / r.slope_per_rad,
                "flare_station_calibers": r.flare_station_m / calibers,
            })
        });
        let numbers = [measured, measured_cp, zero_alpha, theory, theory_cp];
        if numbers.iter().any(|x| !x.is_finite()) {
            return Err(format!("model 2 at Mach {mach}: a result isn't a number"));
        }
        if let Some(hpr) = &hpr {
            for (key, value) in hpr.as_object().ok_or("a malformed reading")? {
                if !value.as_f64().is_some_and(f64::is_finite) {
                    return Err(format!("model 2 at Mach {mach}: `{key}` isn't a number"));
                }
            }
        }
        rows.push(json!({
            "mach": mach,
            "separated": mach >= separates_from,
            "measured": {
                "fitted_c_n_alpha": measured,
                "cp_calibers": measured_cp,
                "zero_alpha_c_n_alpha": zero_alpha,
                "zero_alpha_standard_error": zero_alpha_error,
            },
            "report_method": {
                "fitted_c_n_alpha": theory,
                "cp_calibers": theory_cp,
                "fitted_c_n_alpha_error": theory / measured - 1.0,
                "cp_error_calibers": theory_cp - measured_cp,
            },
            "hpr": hpr,
            "refused": marched.refused,
            "corner": {
                "flow_mach": corner.mach,
                "surface_ahead_deg": corner.ahead_deg,
                "limit_deg": corner.limit_deg,
                "flare_deg": corner.angle_deg,
                "drawn_out": corner.drawn_out,
                "marched_length": corner.marched_length,
                "march_edge_deg": corner.march_edge.and_then(MarchEdge::degrees),
                "march_edge_at_the_cone_table_cap":
                    corner.march_edge == Some(MarchEdge::AtLeastTheConeTableCap),
            },
        }));
    }
    Ok(Value::Array(rows))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use hpr_aero::AeroModel;
    use hpr_design::Rocket;

    use super::*;

    /// A nose, a tube and a conical flare of `flare_deg`: the shape M1.8e17 flew through the
    /// method, without the tail tube and fins, so that the body *is* the run.
    fn flared_design(flare_deg: f64) -> (String, Vec<BodySegment>, f64) {
        let (fore_r, nose_l, tube_l, flare_l) = (0.027, 0.25, 0.7, 0.3);
        let aft_r = fore_r + flare_l * flare_deg.to_radians().tan();
        let part = |id: &str, body: String| {
            format!(
                "{{\"id\":\"{id}\",\"name\":\"\",\"part\":{{{body}}},\
                 \"finish\":{{\"kind\":\"polished\"}}}}"
            )
        };
        let material = "\"material\":{\"name\":\"Steel (plain carbon)\",\"density\":{\"kind\":\"bulk\",\"kg_m3\":7850.0}}";
        let nose = part(
            "nose",
            format!(
                "\"nose_cone\":{{\"shape\":{{\"kind\":\"ogive\",\"radius_ratio\":1.0}},\
             \"length_m\":{nose_l},\"base_radius_m\":{fore_r},\"wall\":{{\"kind\":\"filled\"}},\
             \"shoulder\":null,{material}}}"
            ),
        );
        let tube = part(
            "tube",
            format!(
                "\"body_tube\":{{\"length_m\":{tube_l},\"outer_radius_m\":{fore_r},\
             \"thickness_m\":{fore_r},{material}}}"
            ),
        );
        let flare = part(
            "flare",
            format!(
                "\"transition\":{{\"shape\":{{\"kind\":\"conical\"}},\"clipped\":false,\
             \"length_m\":{flare_l},\"fore_radius_m\":{fore_r},\"aft_radius_m\":{aft_r},\
             \"wall\":{{\"kind\":\"filled\"}},\"fore_shoulder\":null,\"aft_shoulder\":null,\
             {material}}}"
            ),
        );
        let design = format!(
            "{{\"name\":\"flared\",\"stages\":[{{\"id\":\"s\",\"name\":\"\",\
             \"components\":[{nose},{tube},{flare}]}}],\
             \"reference_diameter\":{{\"kind\":\"maximum\"}},\"configurations\":[]}}"
        );
        let segments = vec![
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::Ogive { radius_ratio: 1.0 }, nose_l, fore_r)
                    .unwrap(),
            },
            BodySegment::Cylinder {
                length_m: tube_l,
                radius_m: fore_r,
            },
            BodySegment::Profile {
                profile: Profile::transition(NoseShape::Conical {}, flare_l, fore_r, aft_r, false)
                    .unwrap(),
            },
        ];
        (design, segments, PI * aft_r * aft_r)
    }

    /// [`march`] is the model's own rule, not a second one that happens to agree.
    ///
    /// `hpr-design` has no spherical-cap nose, so TN D-4865's model 2 cannot be flown through a
    /// `Rocket`, and the fixture reads it by repeating what `hpr-aero`'s private
    /// `SupersonicRun::shares` does for a flare. This pins the two together on a flared body the
    /// design route *can* express: share by share, at angles the corner's shock holds and at
    /// angles it does not, where the reading is the same radii drawn out (ADR-047).
    #[test]
    fn the_flare_is_read_as_the_model_reads_it() {
        let (mut drawn_out, mut as_drawn) = (0, 0);
        for flare_deg in [10.0, 18.5, 30.0] {
            let (design, segments, area) = flared_design(flare_deg);
            let rocket: Rocket = serde_json::from_str(&design).unwrap();
            let layout = rocket.layout().unwrap();
            let model = AeroModel::new(&layout).unwrap();
            assert!(
                (model.reference_area_m2() - area).abs() < 1e-15 * area,
                "{flare_deg}°: the reference areas differ"
            );
            let table = model
                .supersonic_body()
                .unwrap_or_else(|| panic!("{flare_deg}°: no supersonic table"));
            // Rows the table stores: every twentieth of a Mach number, so no interpolation.
            // Below the join's start the table has no row of its own and `share` carries the
            // lead row's, which the join's weight then takes to nothing.
            for step in [30, 32, 34, 36, 40, 50, 60, 70, 80, 90] {
                let mach = f64::from(step) / 20.0;
                if mach < table.join_start_mach {
                    continue;
                }
                let Some((slope, moment)) = table.share(0, mach) else {
                    continue;
                };
                let mut want = (slope, moment);
                for index in 1..3 {
                    let (s, m) = table
                        .share(index, mach)
                        .unwrap_or_else(|| panic!("{flare_deg}° at Mach {mach}: no share {index}"));
                    want = (want.0 + s, want.1 + m);
                }
                let marched = march(&segments, mach, area, false).unwrap();
                let Some(got) = &marched.read else {
                    panic!(
                        "{flare_deg}° at Mach {mach}: the model read it and `march` refused: {:?}",
                        marched.refused
                    )
                };
                assert!(
                    (got.slope_per_rad - want.0).abs() <= 1e-12 * want.0.abs(),
                    "{flare_deg}° at Mach {mach}: {} against the model's {}",
                    got.slope_per_rad,
                    want.0
                );
                let want_cp = want.1 / want.0;
                assert!(
                    (got.cp_m - want_cp).abs() <= 1e-12 * want_cp.abs(),
                    "{flare_deg}° at Mach {mach}: the centre of pressure is {} against the \
                     model's {want_cp}",
                    got.cp_m
                );
                if marched.corner.expect("a corner").drawn_out {
                    drawn_out += 1;
                } else {
                    as_drawn += 1;
                }
            }
        }
        assert!(
            drawn_out >= 3 && as_drawn >= 3,
            "{drawn_out} row(s) read a flare drawn out and {as_drawn} read one as drawn; both \
             sides of the corner's limit have to be pinned"
        );
    }

    fn fixture() -> Value {
        let root = crate::designs::root().unwrap();
        read(&root, FIXTURE).unwrap()
    }

    /// `cargo xtask aero` writes what is committed.
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
        format!("{x:.digits$}").replace('-', "\u{2212}")
    }

    /// A signed percentage as the guide writes it.
    fn pct(x: f64) -> String {
        format!("{:+.1}%", 100.0 * x).replace('-', "\u{2212}")
    }

    fn f(value: &Value, pointer: &str) -> f64 {
        value
            .pointer(pointer)
            .and_then(Value::as_f64)
            .unwrap_or_else(|| panic!("no {pointer}"))
    }

    /// `docs/physics/aero.md`'s *What a marched flare is worth* holds the fixture's three tables,
    /// cell by cell, and quotes the numbers its prose turns on.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        // Only this section's tables: a stale row left behind elsewhere on the page must not
        // stand in for one of these, and one left behind here must be caught.
        let heading = "#### What a marched flare is worth\n";
        let from = guide.find(heading).expect("the section") + heading.len();
        let rest = &guide[from..];
        let to = rest.find("\n### ").unwrap_or(rest.len());
        let section = &rest[..to];
        let mut rows = Vec::new();
        for r in fixture["rows"].as_array().unwrap() {
            let mach = f(r, "/mach");
            let measured = num(f(r, "/measured/fitted_c_n_alpha"), 3);
            let cp = num(f(r, "/measured/cp_calibers"), 3);
            rows.push(if r["hpr"].is_null() {
                format!("| {mach} | {measured} | none | none | {cp} | none | none |")
            } else {
                format!(
                    "| {mach} | {measured} | {} | {} | {cp} | {} | {} |",
                    num(f(r, "/hpr/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/fitted_c_n_alpha_error")),
                    num(f(r, "/hpr/cp_calibers"), 3),
                    num(f(r, "/hpr/cp_error_calibers"), 3),
                )
            });
            let corner = &r["corner"];
            let read = match (r["hpr"].is_null(), corner["drawn_out"].as_bool().unwrap()) {
                (true, _) => "drawn out, then refused",
                (false, true) => "drawn out",
                (false, false) => "as drawn",
            };
            let edge = match corner["march_edge_at_the_cone_table_cap"].as_bool() {
                Some(true) => "\u{2265} 30\u{b0} (the tables)".to_string(),
                _ => format!("{}\u{b0}", num(f(corner, "/march_edge_deg"), 4)),
            };
            rows.push(format!(
                "| {mach} | Mach {} | {}\u{b0} | {edge} | {read} |",
                num(f(corner, "/flow_mach"), 4),
                num(f(corner, "/limit_deg"), 4),
            ));
        }
        for r in fixture["beside_model_1"]["rows"].as_array().unwrap() {
            let adds = r["the_flare_adds"].as_f64();
            rows.push(format!(
                "| {} | {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                pct(f(r, "/model_1_error")),
                r["model_2_error"].as_f64().map_or("none".to_string(), pct),
                adds.map_or("none".to_string(), |a| {
                    format!("{:+.1} points", 100.0 * a).replace('-', "\u{2212}")
                }),
                pct(f(r, "/model_1_report_method_error")),
                pct(f(r, "/model_2_report_method_error")),
            ));
        }
        // Three tables, one row per Mach number in each.
        let plotted = fixture["rows"].as_array().unwrap().len();
        assert_eq!(plotted, 6, "the report plots six Mach numbers");
        assert_eq!(
            fixture["beside_model_1"]["rows"].as_array().unwrap().len(),
            plotted,
            "model 1 doesn't have a row for every Mach number"
        );
        assert_eq!(
            rows.len(),
            3 * plotted,
            "the fixture's tables changed shape"
        );
        for row in &rows {
            assert!(
                section.contains(row),
                "the guide's section doesn't have the row `{row}`"
            );
        }
        // And nothing else: a row the fixture no longer writes can't be left behind.
        let printed: Vec<&str> = section
            .lines()
            .map(str::trim_end)
            .filter(|line| {
                line.starts_with("| ")
                    && line[2..]
                        .split('|')
                        .next()
                        .is_some_and(|cell| cell.trim().parse::<f64>().is_ok())
            })
            .collect();
        assert_eq!(
            printed.len(),
            rows.len(),
            "the guide's section has {} table rows against the fixture's {}: {printed:#?}",
            printed.len(),
            rows.len()
        );
        // The numbers the section's prose turns on, written as it writes them.
        let geometry = &fixture["geometry"];
        let quoted = [
            num(f(geometry, "/nose/length"), 7),
            num(f(geometry, "/nose/ends_at_diameter"), 4),
            num(f(geometry, "/nose/blend_arc_centre/0"), 7),
            num(f(geometry, "/closed_on_the_base/cone_length"), 7),
            num(f(geometry, "/closed_on_the_base/flare_length"), 7),
            num(f(geometry, "/closed_on_the_base/juncture_diameter"), 4),
            // What the printed lengths and angles carry the base to, before scaling.
            num(1.0 / f(geometry, "/printed_lengths/scale"), 4),
            // Seven figures: the guide can't quote more than the three platforms agree on.
            num(f(&fixture, "/reads_from_mach/mach"), 7),
            num(
                f(&fixture, "/reads_from_mach/wedge_limit_deg_just_below"),
                7,
            ),
            num(f(&fixture, "/reads_from_mach/march_edge_deg_just_below"), 7),
        ];
        // The one row the prose quotes to more digits than its table: model 1 at Mach 1.90, where
        // the flared body is the worse of the two and the unflared one is nearly exact.
        let nearly_exact = fixture["beside_model_1"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| f(r, "/mach") == 1.9)
            .map(|r| num(100.0 * f(r, "/model_1_error"), 3))
            .expect("model 1 at Mach 1.9");
        assert!(
            guide.contains(&format!("+{nearly_exact}%")),
            "aero.md doesn't quote model 1's +{nearly_exact}% at Mach 1.9"
        );
        // The flare's own share of the body, which the prose quotes at both ends and at its
        // smallest, and where that share acts.
        let shares: Vec<f64> = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["hpr"].is_null())
            .map(|r| f(r, "/hpr/flare_share_of_the_body"))
            .collect();
        let stations: Vec<f64> = fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["hpr"].is_null())
            .map(|r| f(r, "/hpr/flare_station_calibers"))
            .collect();
        let least = shares.iter().copied().fold(f64::INFINITY, f64::min);
        let quoted = quoted.into_iter().chain([
            format!("{:.1}%", 100.0 * shares[0]),
            format!("{:.1}%", 100.0 * shares[shares.len() - 1]),
            format!("{:.1}%", 100.0 * least),
            num(stations.iter().copied().fold(f64::INFINITY, f64::min), 3),
            num(stations.iter().copied().fold(0.0_f64, f64::max), 3),
        ]);
        for number in quoted {
            assert!(
                guide.contains(&number),
                "aero.md doesn't quote `{number}` from the fixture"
            );
        }
    }

    /// The blend arc hpr draws is the circle the drawing dimensions, and the body closes on the
    /// printed numbers it was told to keep.
    #[test]
    fn the_body_is_the_drawings() {
        let geometry = &fixture()["geometry"];
        let worst = geometry["nose"]["blend_arc_worst_radius_error"]
            .as_f64()
            .unwrap();
        assert!(
            worst < 1e-12,
            "the blend arc misses its circle by {worst} calibres"
        );
        let closed = &geometry["closed_on_the_base"];
        assert!((closed["base_diameter"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        assert!((closed["length"].as_f64().unwrap() - 1.609).abs() < 1e-12);
        let printed = &geometry["printed_lengths"];
        assert!((printed["base_diameter"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        // The two closures put the juncture 0.0146 calibres apart, which is the drawing's own
        // disagreement: it is the spread the fixture's `printed_lengths` rows measure.
        let apart = closed["juncture_at"].as_f64().unwrap()
            - printed["juncture_at"].as_f64().unwrap() / printed["scale"].as_f64().unwrap();
        assert!(
            (apart - 0.014_617_174_571_5).abs() < 1e-9,
            "the two closures' junctures are {apart} calibres apart"
        );
    }
}
