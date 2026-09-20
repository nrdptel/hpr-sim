//! A rocket's normal force and centre of pressure: every component's terms, built once from a
//! [`Layout`] and summed at each flow condition.
//!
//! The coefficient at an angle of attack `α` is `C_N = C_Nα(α) α`, with the slope defined as
//! `C_N/α` (Niskanen 2009 eq. 3.8) and the centre of pressure as the moment sum
//! `X = Σ C_Nα,i X_i / Σ C_Nα,i` (Barrowman 1966 p. 38; Niskanen eq. 3.29). The terms:
//!
//! - bodies of revolution, `(2/A_ref)ΔA · sin α/α` at `X_B` ([`crate::body`]), plus body lift
//!   `η C_dn (A_plan/A_ref) sin² α / α` at the planform centroid, Jorgensen's crossflow term
//!   ([`crate::crossflow`]);
//! - a step in radius where one body component meets the next, `(2/A_ref)ΔA · sin α/α` at the
//!   joint, reported with the aft component. This extrapolates Barrowman 1966 eq. 10 over the whole
//!   body to a transition of zero length; Barrowman 1967 p. 18 assumes no discontinuities;
//! - fin sets, `(C_Nα)₁ Σ sin² Λ_k · f_N · K_T(B)` at the fin's centre of pressure, both at the
//!   flow's Mach number ([`crate::fins::FinAero`]), and for one or two fins the side force
//!   `(C_Nα)₁ Σ sin Λ cos Λ · K_T(B)` across the flow's plane ([`crate::fins::side_sum`]).
//!
//! Below the speed of sound the bodies' potential-flow terms don't change with Mach:
//! slender-body theory's slope and centre of pressure hold at any Mach number (Barrowman 1967
//! p. 18). Body lift changes with the crossflow Mach number `M sin α` at any speed. Faster than
//! sound, a pointed nose and the cylinders straight behind it take their potential-flow slope and
//! moment from the second-order shock-expansion method ([`crate::shock_expansion`], NACA TN
//! 3527), and a boattail behind them Washington and Pettis's measured increment
//! ([`crate::supersonic_boattail`]), joined to slender-body theory linearly in Mach
//! ([`SupersonicBody`]; the decision records on flying them, [ADR-034][adr-034] and
//! [ADR-037][adr-037]).
//! Other bodies keep slender-body theory's terms. [`BodyModel`] chooses the body-lift and boattail
//! rules; the default is hpr's current one.
//!
//! Launch lugs and rail buttons add drag only, and internal parts sit inside the body. Tube fins
//! have no cited normal-force method yet and are refused, as is any part kind this model doesn't
//! know. Stations are metres aft of the nose tip.
//!
//! [adr-034]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-034-the-bodys-supersonic-normal-force-in-flight-tabulated-shock-expansion-shares-joined-linearly-from-mach-12-2026-09-19
//! [adr-037]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-037-body-lift-by-jorgensens-crossflow-at-every-speed-and-a-boattails-measured-share-faster-than-sound-2026-09-19

use std::f64::consts::PI;
use std::sync::{Arc, OnceLock};

use hpr_design::{Layout, Part, PlacedComponent};
use serde::{Deserialize, Serialize};

use crate::afterbody::SEPARATION_ONSET_RAD;
use crate::body::{BodyGeometry, sinc};
use crate::crossflow::BodyLift;
use crate::drag::{
    BUILDUP_MACH_LIMIT, ComponentDrag, ComponentDragTerms, Drag, DragConditions,
    axial_drag_alpha_factor, body_friction_form_factor, couple_afterbody,
};
use crate::error::{AeroError, check_dimension, check_mach};
use crate::fins::{
    FinAero, FinLoading, FinRollTerms, fin_count_factor, interference_factor,
    roll_damping_interference, roll_forcing_interference, roll_sum, side_sum,
};
use crate::shock_expansion::{
    BodySegment, DEFAULT_ELEMENTS_PER_CURVE, SegmentSlope, ShockExpansionBody,
};
use crate::supersonic_boattail::{wp_centre_fraction, wp_slope};
use crate::table::{DragTable, NormalForceLookup, NormalForceTable, TableReference};

/// The largest fin cant the roll model takes, 15°: past it a fin stalls, where its lift stops
/// growing with the angle, a judgement ([`AeroModel::roll`]).
pub const MAX_CANT_RAD: f64 = 15.0 * std::f64::consts::PI / 180.0;

/// The top of the normal force's range: Mach 5, where the hypersonic region begins (Niskanen 2009
/// Table 3.1, p. 19). [`AeroModel::normal_force`] refuses it and anything faster.
pub const NORMAL_FORCE_MACH_LIMIT: f64 = 5.0;

/// The lowest Mach number at which the body's supersonic join can start ([`SupersonicBody`]):
/// below it every body keeps slender-body theory's terms. A judgement: the first body that TN
/// 3527's method covers and TN D-4014 measured is at Mach 1.5, the join's end from here.
pub const SUPERSONIC_JOIN_START_MACH: f64 = 1.2;

/// Steps of the shock-expansion table per unit Mach: one row every 0.05.
const SUPERSONIC_STEPS_PER_MACH: f64 = 20.0;

/// The table's first row, Mach 1.2 ([`SUPERSONIC_JOIN_START_MACH`]).
const SUPERSONIC_FIRST_STEP: usize = 24;

/// The table's last row, Mach 5 ([`NORMAL_FORCE_MACH_LIMIT`]).
const SUPERSONIC_LAST_STEP: usize = 100;

/// Rows across the join.
const SUPERSONIC_JOIN_STEPS: usize = 6;

/// The most halvings of the 0.05 step in which the method starts to hold. Bisection stops sooner,
/// after about 48, when no `f64` lies between its ends: the join's start is then as exact as the
/// number allows, so it moves with the body's shape, not in 0.05 steps
/// ([issue #87](https://github.com/nrdptel/hpr-sim/issues/87)). It has to be that exact: the
/// shares climb from zero like `√(M − M_start)` there, so a start off by `δ` puts `√δ`-sized
/// shares in the table's first row.
const SUPERSONIC_JOIN_BISECTIONS: usize = 64;

/// The smallest share of shelter worth a table: below this a lip counts as out of its boattail's
/// wake, since weighing the method in at under a millionth changes no number anyone can read and
/// building the table costs half a second ([`SupersonicBody::shape_weight`]).
const SUPERSONIC_SHELTER_FLOOR: f64 = 1e-6;

/// The width of the body's supersonic join in Mach, 0.3: over it the shock-expansion shares
/// replace slender-body theory's linearly ([`SupersonicBody`]).
pub const SUPERSONIC_JOIN_WIDTH_MACH: f64 =
    SUPERSONIC_JOIN_STEPS as f64 / SUPERSONIC_STEPS_PER_MACH;

/// The second-order shock-expansion method's share of each body component it covers, tabulated in
/// Mach, and where it joins slender-body theory (the decision record on flying it,
/// [ADR-034][adr-034]).
///
/// The method ([`crate::shock_expansion`]) covers a pointed nose, the cylinders straight behind
/// it, and boattails and cylinders behind those, up to the first other body (a flare), step in
/// radius or gap. It flies only if no body after them has a potential-flow slope of its own (a
/// flare, a step). A boattail doesn't take slender-body theory's share: the method's nose and
/// cylinder beside slender-body theory's boattail would put the body's centre of pressure further
/// off than slender-body theory alone (milestone
/// [M1.8e4](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e4)). By default
/// ([`SupersonicBoattail::WashingtonPettis`]) it takes the share the method gives a cylinder of
/// its length and fore radius in its place, plus Washington and Pettis's measured increment at
/// their centre of pressure ([`crate::supersonic_boattail`]); [`SupersonicBoattail::Footnote8`]
/// keeps the method's own, TN 3527 footnote 8. Cylinders behind a boattail take the method's
/// shares either way. A boattail's share, and a cylinder's behind it, may cross zero, so those
/// parts keep slender-body theory's station ([`AeroModel::component_station_m`]).
///
/// The method is too slow to run at each step of a flight, so [`AeroModel::supersonic_body`] tabulates each covered segment's slope and moment
/// every 0.05 in Mach, from Mach 5 down to the lowest Mach from which the method holds, and a
/// flight interpolates linearly between rows. Where the method stops holding above
/// [`SUPERSONIC_JOIN_START_MACH`], bisection finds that Mach to the last bit of an `f64` and the
/// table gains a row there, so the join's start moves with the body's shape rather than in 0.05 steps.
/// The join starts at that Mach, or at
/// [`SUPERSONIC_JOIN_START_MACH`] if higher: at Mach `M`, a covered component's potential-flow
/// slope and moment, and a nose's or cylinder's station, are slender-body theory's plus
/// `w (shock-expansion − slender-body)`, `w = `[`SupersonicBody::weight`]`(M)`: the join's own
/// `(M − M_join)/`[`SUPERSONIC_JOIN_WIDTH_MACH`] clamped to `[0, 1]`, times
/// [`SupersonicBody::shape_weight`], which is 1 unless a lip rides along only partly inside its
/// boattail's wake. Everything is linear in Mach and in that share of shape, so nothing jumps.
///
/// [adr-034]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-034-the-bodys-supersonic-normal-force-in-flight-tabulated-shock-expansion-shares-joined-linearly-from-mach-12-2026-09-19
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SupersonicBody {
    /// How many body components the method covers: the first entries of [`AeroModel::bodies`].
    /// The last of them may be lips in a covered boattail's wake, which the method doesn't march:
    /// the method gives them nothing at any Mach number, so what such a lip carries is
    /// `(1 − `[`SupersonicBody::weight`]`(M))` of slender-body theory's share, none of it above
    /// the join where the wake covers it wholly ([`crate::drag::WakeTerm`]; the decision record on
    /// the lip, [ADR-039][adr-039]).
    ///
    /// [adr-039]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-039-a-lip-in-a-boattails-wake-carries-nothing-faster-than-sound-2026-09-19
    pub covered: usize,
    /// The Mach number where the join starts; the shares count in full from
    /// [`SUPERSONIC_JOIN_WIDTH_MACH`] above it.
    pub join_start_mach: f64,
    /// The table's first even row is at Mach `first_step / 20`.
    first_step: usize,
    /// Each even row's shares, one per covered component: slope per radian and its moment about
    /// the nose tip, m per radian. Every share with a station is positive.
    rows: Vec<Vec<SegmentSlope>>,
    /// Whether each covered component's share has a station of its own: a nose's and a
    /// cylinder's do, a boattail's and those behind it don't (they may cross zero).
    stationed: Vec<bool>,
    /// The shares at `join_start_mach`, where the method starts to hold, when that lies between
    /// even rows: the table's first row.
    lead: Option<Vec<SegmentSlope>>,
    /// How much of the method the body's shape takes, in `(0, 1]`, which multiplies the join's
    /// weight: below 1 where a lip rides along only partly inside its boattail's wake
    /// ([`crate::drag::WakeTerm`], [issue #87: the body's normal force jumps with small changes of
    /// shape](https://github.com/nrdptel/hpr-sim/issues/87), [ADR-041: a lip's shelter weighed,
    /// not switched][adr-041]). Slender-body theory takes the rest, so a
    /// lip drawn a hair taller moves the body between the models continuously instead of
    /// switching it.
    ///
    /// [adr-041]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-041-a-lips-shelter-is-weighed-as-the-drag-buildup-weighs-it-not-switched-at-a-threshold-2026-09-20
    pub shape_weight: f64,
}

/// How a boattail that the shock-expansion method covers takes its share of the normal force
/// faster than sound ([`SupersonicBody`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SupersonicBoattail {
    /// The share the method gives a cylinder of the boattail's length and fore radius in its
    /// place, plus Washington and Pettis's measured increment at their centre of pressure
    /// ([`crate::supersonic_boattail`]): hpr's rule since [M1.8e6](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e6) (the decision
    /// record, [ADR-037](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-037-body-lift-by-jorgensens-crossflow-at-every-speed-and-a-boattails-measured-share-faster-than-sound-2026-09-19)).
    #[default]
    WashingtonPettis,
    /// The method's own share, TN 3527 footnote 8's tangent cone: hpr's rule from the milestone
    /// that first flew a boattail by the method ([M1.8e4](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e4)) until
    /// [M1.8e6](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e6).
    Footnote8,
}

/// The choices in the bodies' normal-force model ([`AeroModel::with_body_model`]). The default is
/// hpr's current model, [`BodyModel::CURRENT`]; [`BodyModel::BEFORE_M1_8E6`] reproduces earlier
/// results. Change one choice with [`BodyModel::with_body_lift`] or
/// [`BodyModel::with_supersonic_boattail`]. In JSON, for example
/// `{"body_lift": {"kind": "galejs", "k": 1.1}, "supersonic_boattail": "footnote8"}`; a missing
/// field takes the current choice.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
#[non_exhaustive]
pub struct BodyModel {
    /// How body lift is sized.
    pub body_lift: BodyLift,
    /// How a boattail takes its share faster than sound.
    pub supersonic_boattail: SupersonicBoattail,
}

impl BodyModel {
    /// hpr's current body model: Jorgensen's body lift and Washington and Pettis's boattail.
    pub const CURRENT: Self = Self {
        body_lift: BodyLift::JORGENSEN,
        supersonic_boattail: SupersonicBoattail::WashingtonPettis,
    };

    /// hpr's body model before [M1.8e6](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e6) sized body lift and the boattail: Galejs's
    /// `K` = 1.1 and TN 3527 footnote 8's boattail.
    pub const BEFORE_M1_8E6: Self = Self {
        body_lift: BodyLift::GALEJS,
        supersonic_boattail: SupersonicBoattail::Footnote8,
    };

    /// This model with body lift sized by `body_lift`.
    #[must_use]
    pub const fn with_body_lift(mut self, body_lift: BodyLift) -> Self {
        self.body_lift = body_lift;
        self
    }

    /// This model with a supersonic boattail's share by `supersonic_boattail`.
    #[must_use]
    pub const fn with_supersonic_boattail(
        mut self,
        supersonic_boattail: SupersonicBoattail,
    ) -> Self {
        self.supersonic_boattail = supersonic_boattail;
        self
    }
}

/// A boattail in the shock-expansion run, for Washington and Pettis's increment: the method's
/// body with a cylinder of the boattail's length and fore radius in its place (its last segment),
/// and the boattail's radii and length, m.
#[derive(Debug, Clone, PartialEq)]
struct RunBoattail {
    in_its_place: Vec<BodySegment>,
    fore_radius_m: f64,
    aft_radius_m: f64,
    length_m: f64,
}

/// The segments the shock-expansion method covers, the station of the nose's tip, and each
/// segment's fore and aft stations, m aft of the nose tip: `None` for a boattail and anything
/// behind it, whose shares can be negative or cross zero (TN 3527 footnote 8, or Washington and
/// Pettis's increment) and so needn't have a station on their segments.
#[derive(Debug, Clone, PartialEq)]
struct SupersonicRun {
    segments: Vec<BodySegment>,
    vertex_m: f64,
    bounds_m: Vec<Option<(f64, f64)>>,
    /// Each segment's fore station, m aft of the nose tip.
    fore_m: Vec<f64>,
    /// Each segment that is a boattail, when its share is Washington and Pettis's.
    boattails: Vec<Option<RunBoattail>>,
    /// How many bodies behind the marched segments are lips wholly in a covered boattail's wake
    /// ([`crate::drag::WakeTerm`]): they widen the body, carry nothing faster than sound, and so
    /// the run covers them with a share of zero.
    sheltered_lips: usize,
    /// How much of the run's shape the method takes, in `(0, 1]`: the smallest share of the wake
    /// covering a sheltered lip — its rise, the tube between it and the boattail, and anything
    /// else in the way (issue #87). One where no lip rides along, or where the wake covers it
    /// wholly.
    shape_weight: f64,
    // `segments`, `bounds_m`, `fore_m` and `boattails` hold one entry per segment: `from_design`
    // pushes all four in the same branch, and `shares` indexes them together; `shares` then adds
    // one zero for each sheltered lip.
}

impl SupersonicRun {
    /// The method's shares at `mach`, moments about the nose tip, or `None` where it fails or a
    /// nose's or cylinder's share isn't positive with its station on its own segment (a share
    /// that crosses zero has no station, and one that is positive but small could put a part's
    /// damping station far off the rocket). A boattail's share, and those behind it, may take
    /// either sign: their damping stations stay slender-body theory's
    /// ([`AeroModel::component_station_m`]). A boattail with `in_its_place`, the method's body
    /// with a cylinder in its place, takes that cylinder's share plus Washington and Pettis's
    /// increment ([`SupersonicBoattail::WashingtonPettis`]).
    fn shares(
        &self,
        body: &ShockExpansionBody,
        in_its_place: &[Option<ShockExpansionBody>],
        mach: f64,
        reference_area_m2: f64,
    ) -> Option<Vec<SegmentSlope>> {
        let mut shares = body.segment_slopes(mach, reference_area_m2).ok()?;
        let vertex_m = self.vertex_m;
        // A lip in a boattail's wake carries nothing faster than sound (ADR-039).
        shares.extend(std::iter::repeat_n(
            SegmentSlope::default(),
            self.sheltered_lips,
        ));
        for (index, (boattail, cylinder_body)) in
            self.boattails.iter().zip(in_its_place).enumerate()
        {
            let (Some(boattail), Some(cylinder_body)) = (boattail, cylinder_body) else {
                continue;
            };
            // The cylinder in its place is that body's last segment; its moment is about the
            // vertex, as the method's shares are.
            let cylinder = *cylinder_body
                .segment_slopes(mach, reference_area_m2)
                .ok()?
                .last()?;
            let fore_radius_m = boattail.fore_radius_m;
            // Issue #90's cap. Washington and Pettis measured boattails of 4° to 9.5°, whose
            // flow follows the surface; past about 16° it separates (Cubbage, the angle the drag
            // buildup uses, `crate::afterbody::SEPARATION_ONSET_RAD`) and nothing measures what
            // the body then carries. So the correlation is read no steeper than 16°: a boattail
            // past it takes the increment of one of the same radii drawn out to that angle. The
            // length only enters the correlation; the centre of pressure stays on the real
            // boattail. Continuous in shape (at 16° the two lengths are equal), and it holds the
            // lift the boattail takes off rather than letting it go to zero, which would move the
            // centre of pressure aft and make a steep boattail look more stable than measured.
            let drop_m = fore_radius_m - boattail.aft_radius_m;
            let read = |length_m| wp_slope(mach, fore_radius_m, boattail.aft_radius_m, length_m);
            let at_true_angle = read(boattail.length_m).ok()?;
            let held = read(boattail.length_m.max(drop_m / SEPARATION_ONSET_RAD.tan())).ok()?;
            // How far the holding may go. Reading a longer boattail walks Fig. 5's argument
            // toward zero, where the curve comes from the report's lowest supersonic runs
            // (`crate::supersonic_boattail`) and passes Munk's slender-body line, which
            // RD-TM-68-5 plots for comparison at subsonic speeds (p. 3). hpr does not invent a
            // length and then read that branch: the *extra* the holding removes stops at
            // potential flow's `2 (A_aft − A_fore)/A_fore`. A boattail's read at its own length
            // is never clipped, whatever it says — that is the correlation as published, and a
            // genuinely long boattail reads the same branch unbounded. At 16° the two reads are
            // equal, so this is continuous in shape.
            let ratio = boattail.aft_radius_m / fore_radius_m;
            let measured = held.max(at_true_angle.min(2.0 * (ratio * ratio - 1.0)));
            let increment = measured * PI * fore_radius_m * fore_radius_m / reference_area_m2;
            let centre_m =
                self.fore_m[index] + wp_centre_fraction(mach) * boattail.length_m - vertex_m;
            shares[index] = SegmentSlope {
                slope_per_rad: cylinder.slope_per_rad + increment,
                moment_slope_m: cylinder.moment_slope_m + increment * centre_m,
            };
        }
        let on_segment = shares.iter().zip(&self.bounds_m).all(|(s, bounds)| {
            let Some((fore, aft)) = *bounds else {
                return s.slope_per_rad.is_finite() && s.moment_slope_m.is_finite();
            };
            let station = (s.moment_slope_m + s.slope_per_rad * vertex_m) / s.slope_per_rad;
            let slack = 1e-9 * (aft - vertex_m);
            s.slope_per_rad > 0.0 && station >= fore - slack && station <= aft + slack
        });
        on_segment.then(|| {
            shares
                .into_iter()
                .map(|s| SegmentSlope {
                    slope_per_rad: s.slope_per_rad,
                    moment_slope_m: s.moment_slope_m + s.slope_per_rad * vertex_m,
                })
                .collect()
        })
    }
}

/// The table, built once and shared by a model's clones. It follows from the covered segments, so
/// two models compare equal whether or not either has built it.
#[derive(Debug, Clone, Default)]
struct SupersonicTable(Arc<OnceLock<Option<SupersonicBody>>>);

impl PartialEq for SupersonicTable {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl SupersonicBody {
    /// Tabulates the method's shares of `run`'s segments, whose vertex is at `run.vertex_m` aft
    /// of the nose tip, or `None` where the method can't take the body or doesn't hold across a
    /// whole join below Mach 5.
    fn new(run: &SupersonicRun, reference_area_m2: f64) -> Option<Self> {
        let body = ShockExpansionBody::new(&run.segments, DEFAULT_ELEMENTS_PER_CURVE).ok()?;
        let mut in_its_place = Vec::with_capacity(run.boattails.len());
        for boattail in &run.boattails {
            in_its_place.push(match boattail {
                Some(b) => Some(
                    ShockExpansionBody::new(&b.in_its_place, DEFAULT_ELEMENTS_PER_CURVE).ok()?,
                ),
                None => None,
            });
        }
        let at = |step: f64| {
            run.shares(
                &body,
                &in_its_place,
                step / SUPERSONIC_STEPS_PER_MACH,
                reference_area_m2,
            )
        };
        let mut rows = Vec::new();
        let mut first_step = SUPERSONIC_LAST_STEP + 1;
        // From Mach 5 down, until the method first fails or a share isn't on its segment.
        for step in (SUPERSONIC_FIRST_STEP..=SUPERSONIC_LAST_STEP).rev() {
            let Some(row) = at(step as f64) else {
                break;
            };
            rows.push(row);
            first_step = step;
        }
        rows.reverse();
        // The join needs its whole width inside the table.
        if first_step + SUPERSONIC_JOIN_STEPS > SUPERSONIC_LAST_STEP {
            return None;
        }
        // Where the method stops holding between two rows, bisect for that Mach: `high` holds,
        // `low` doesn't. The join's start then moves continuously with the body's shape, except
        // where the table itself appears or vanishes: the guard above near Mach 4.7, and the
        // model switches of issue #87.
        let mut lead = None;
        let mut join_start_mach = first_step as f64 / SUPERSONIC_STEPS_PER_MACH;
        if first_step > SUPERSONIC_FIRST_STEP {
            let (mut low, mut high) = (first_step as f64 - 1.0, first_step as f64);
            let mut held = None;
            for _ in 0..SUPERSONIC_JOIN_BISECTIONS {
                let mid = 0.5 * (low + high);
                if mid <= low || mid >= high {
                    break;
                }
                match at(mid) {
                    Some(row) => {
                        high = mid;
                        held = Some(row);
                    }
                    None => low = mid,
                }
            }
            if let Some(row) = held {
                join_start_mach = high / SUPERSONIC_STEPS_PER_MACH;
                lead = Some(row);
            }
        }
        Some(Self {
            covered: run.segments.len() + run.sheltered_lips,
            join_start_mach,
            first_step,
            rows,
            lead,
            shape_weight: run.shape_weight,
            stationed: run
                .bounds_m
                .iter()
                .map(Option::is_some)
                // A sheltered lip keeps slender-body theory's station, as a boattail does.
                .chain(std::iter::repeat_n(false, run.sheltered_lips))
                .collect(),
        })
    }

    /// How much of the method the body takes at `mach`, in `[0, 1]`: the join's own weight, 0 at
    /// [`Self::join_start_mach`] and below and 1 from [`SUPERSONIC_JOIN_WIDTH_MACH`] above it,
    /// times [`Self::shape_weight`]. Slender-body theory takes the rest, so this is the single
    /// definition of how the two models blend.
    #[must_use]
    pub fn weight(&self, mach: f64) -> f64 {
        self.shape_weight
            * ((mach - self.join_start_mach) / SUPERSONIC_JOIN_WIDTH_MACH).clamp(0.0, 1.0)
    }

    /// Covered component `index`'s share at `mach`, interpolated linearly between rows (clamped
    /// to the table's ends): slope per radian and moment about the nose tip, m per radian. A
    /// boattail's share, and a cylinder's behind it, may be negative or cross zero, so their
    /// moment over slope need not lie on the part.
    pub fn share(&self, index: usize, mach: f64) -> Option<(f64, f64)> {
        let x = mach * SUPERSONIC_STEPS_PER_MACH - self.first_step as f64;
        let (a, b, t) = match &self.lead {
            // Between the lead row and the first even row.
            Some(lead) if x < 0.0 => {
                let lead_x =
                    self.join_start_mach * SUPERSONIC_STEPS_PER_MACH - self.first_step as f64;
                let t = ((x - lead_x) / -lead_x).clamp(0.0, 1.0);
                (lead.get(index)?, self.rows[0].get(index)?, t)
            }
            _ => {
                // `new` keeps at least `SUPERSONIC_JOIN_STEPS + 1` even rows.
                let i = (x.floor().max(0.0) as usize).min(self.rows.len() - 2);
                let t = (x - i as f64).clamp(0.0, 1.0);
                (self.rows[i].get(index)?, self.rows[i + 1].get(index)?, t)
            }
        };
        Some((
            a.slope_per_rad + t * (b.slope_per_rad - a.slope_per_rad),
            a.moment_slope_m + t * (b.moment_slope_m - a.moment_slope_m),
        ))
    }
}

/// The whole rocket's rolling moment coefficients at one Mach number ([`AeroModel::roll`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Roll {
    /// `C_l0`: the rolling moment about `+z_B` at no roll rate, from the fins' cant.
    pub forcing: f64,
    /// `C_lp = ∂C_l/∂(p d/2V)`, negative: the damping.
    pub damping: f64,
}

/// The air-relative flow at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Flow {
    /// Mach number: in `[0, 5)` for the normal force and the drag buildup.
    pub mach: f64,
    /// Total angle of attack between the body axis `+z_B` and the air-relative velocity, rad, in
    /// `[0, π]`. The models are small-angle models (see `docs/physics/aero.md`).
    pub alpha_rad: f64,
    /// Roll angle of the lateral airflow, rad from `x_B` toward `y_B`: the direction in which the
    /// air crosses the body. Only fin sets of one or two fins depend on it.
    pub roll_rad: f64,
}

impl Flow {
    /// A flow at `mach`, angle of attack `alpha_rad` and lateral-flow roll `roll_rad`. Checked
    /// when used ([`Flow::validate`]).
    pub fn new(mach: f64, alpha_rad: f64, roll_rad: f64) -> Self {
        Self {
            mach,
            alpha_rad,
            roll_rad,
        }
    }

    /// Straight into the wind at `mach`.
    pub fn axial(mach: f64) -> Self {
        Self::new(mach, 0.0, 0.0)
    }

    /// Checks the Mach number against the normal force's range, and the angles.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 5)` ([`NORMAL_FORCE_MACH_LIMIT`]), and
    /// [`AeroError::Domain`] for an angle of attack outside `[0, π]` or a non-finite roll.
    pub fn validate(&self) -> Result<(), AeroError> {
        check_mach(self.mach, NORMAL_FORCE_MACH_LIMIT, "the normal force")?;
        self.validate_angles()
    }

    /// As [`Flow::validate`], for the drag buildup's range `[0, 5)`, which names the buildup when
    /// it refuses.
    fn validate_for_buildup(&self) -> Result<(), AeroError> {
        check_mach(self.mach, BUILDUP_MACH_LIMIT, "the drag buildup")?;
        self.validate_angles()
    }

    /// Checks the angles only.
    fn validate_angles(&self) -> Result<(), AeroError> {
        if !(0.0..=PI).contains(&self.alpha_rad) {
            return Err(AeroError::Domain {
                what: "angle of attack",
                value: self.alpha_rad,
            });
        }
        if !self.roll_rad.is_finite() {
            return Err(AeroError::Domain {
                what: "flow roll angle",
                value: self.roll_rad,
            });
        }
        Ok(())
    }
}

/// The normal force of a whole rocket, or of one component, at a flow condition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NormalForce {
    /// Normal-force coefficient `C_N` on the reference area, in the plane of the flow.
    pub coefficient: f64,
    /// `C_N/α` per radian; at `α = 0`, the slope `∂C_N/∂α`.
    pub slope_per_rad: f64,
    /// `Σ C_N,i X_i`, m: the normal force's moment about the nose tip per unit dynamic pressure and
    /// reference area, defined even when the net force is zero.
    pub moment_m: f64,
    /// Centre of pressure, m aft of the nose tip; `None` when the slope is zero, or so small
    /// against its terms (below 1e-12 of `Σ |C_Nα,i|`) that the ratio would be noise.
    pub cp_station_m: Option<f64>,
    /// Side-force coefficient across the plane of the flow, along `z_B` × the lateral-flow
    /// direction. Only fin sets of one or two fins produce it ([`crate::fins::side_sum`]).
    pub side_coefficient: f64,
    /// `Σ C_Y,i X_i`, m: the side force's moment about the nose tip per unit dynamic pressure and
    /// reference area.
    pub side_moment_m: f64,
    /// The override table's lookup, on the table's reference area, when the whole rocket's normal
    /// force came from one ([`AeroModel::with_normal_force_table`]): whether the Mach number was
    /// outside a column's range, or the angle past the last column's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub table: Option<NormalForceLookup>,
}

/// One component's contributions per radian: slope, moment slope about the nose tip, side slope
/// and side moment slope, and `Σ |terms|` of the slope to judge cancellation.
#[derive(Clone, Copy, Default)]
struct Term {
    slope: f64,
    moment: f64,
    side: f64,
    side_moment: f64,
    scale: f64,
}

impl Term {
    fn add(self, other: Term) -> Term {
        Term {
            slope: self.slope + other.slope,
            moment: self.moment + other.moment,
            side: self.side + other.side,
            side_moment: self.side_moment + other.side_moment,
            scale: self.scale + other.scale,
        }
    }
}

impl NormalForce {
    fn new(term: Term, alpha_rad: f64) -> Self {
        let cancelled = term.slope.abs() <= 1e-12 * term.scale;
        Self {
            coefficient: term.slope * alpha_rad,
            slope_per_rad: term.slope,
            moment_m: term.moment * alpha_rad,
            cp_station_m: (term.slope != 0.0 && !cancelled).then(|| term.moment / term.slope),
            side_coefficient: term.side * alpha_rad,
            side_moment_m: term.side_moment * alpha_rad,
            table: None,
        }
    }
}

/// One component's share of the normal force.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentNormalForce {
    /// The component's id.
    pub id: String,
    /// Its normal force.
    pub normal_force: NormalForce,
}

/// A body component's precomputed terms.
///
/// Serialize-only, like [`AeroModel`]: the terms are computed by [`AeroModel::new`], not read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BodyAero {
    /// The component's id.
    pub id: String,
    /// Station of its fore end, m.
    pub fore_station_m: f64,
    /// Its geometry.
    pub geometry: BodyGeometry,
    /// The step in cross-section area from the previous body component's aft end to this one's
    /// fore end, m² (zero for the first body component).
    pub step_area_m2: f64,
    /// Slender-body theory's potential-flow slope at `α → 0`, per radian, with the step. Faster
    /// than sound a body the shock-expansion method covers takes its share instead
    /// ([`SupersonicBody`]).
    pub slope_per_rad: f64,
    /// Slender-body theory's potential-flow moment slope about the nose tip, m per radian, with
    /// the step.
    pub moment_slope_m: f64,
    /// Its planform over the reference area, `A_plan / A_ref`: body lift is
    /// `C_N = factor · planform_ratio · sin² α`, the factor from the model's [`BodyLift`].
    pub planform_ratio: f64,
    /// Station of the body lift, m.
    pub lift_station_m: f64,
}

/// A fin set's precomputed terms.
///
/// Serialize-only, like [`AeroModel`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FinSetAero {
    /// The component's id.
    pub id: String,
    /// Number of fins.
    pub count: u32,
    /// Roll angle of the first fin, rad.
    pub base_angle_rad: f64,
    /// One fin's normal force through the speed regimes, and its geometry.
    pub fin: FinAero,
    /// Fin–fin factor `f_N`.
    pub count_factor: f64,
    /// Fin–body interference `K_T(B)`.
    pub interference: f64,
    /// Station of the fins' root leading edge, m aft of the nose tip.
    pub fore_station_m: f64,
    /// Cant, rad: positive turns fin 0's leading edge toward `−y_B` (`hpr_design::FinSet`).
    pub cant_rad: f64,
    /// Radius of the body tube at the fins, m.
    pub body_radius_m: f64,
    /// The body's interference with the roll forcing, `k_T(B)` ([`roll_forcing_interference`]).
    pub roll_forcing_interference: f64,
    /// The body's interference with the roll damping, `k_R(B)` ([`roll_damping_interference`]).
    pub roll_damping_interference: f64,
    /// One fin's roll terms on this body ([`FinAero::roll_terms`]).
    pub roll: FinRollTerms,
}

impl FinSetAero {
    /// The set's centre of pressure at `mach`, m aft of the nose tip.
    ///
    /// # Errors
    ///
    /// As [`FinAero::loading`].
    pub fn cp_station_m(&self, mach: f64) -> Result<f64, AeroError> {
        Ok(self.fore_station_m + self.fin.loading(mach)?.cp_m)
    }
}

/// A rocket's aerodynamic model: normal force, centre of pressure and drag.
///
/// Serialize-only, for inspection: a model is built from a [`Layout`] by [`AeroModel::new`], which
/// checks what it builds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AeroModel {
    reference_area_m2: f64,
    reference_diameter_m: f64,
    length_m: f64,
    /// The largest radius of the bodies, m: RASAero II's reference.
    max_body_radius_m: f64,
    /// The body's length over its largest diameter, for body lift's `η` ([`crate::crossflow`]).
    fineness: f64,
    /// Fig. 4's `η` at that fineness, computed once.
    crossflow_eta_low: f64,
    /// The body-lift and boattail rules.
    body_model: BodyModel,
    bodies: Vec<BodyAero>,
    /// The nose, the cylinders and boattails behind it, for the shock-expansion method.
    #[serde(skip)]
    supersonic_run: Option<SupersonicRun>,
    /// Their tabulated shares, built the first time a flow faster than
    /// [`SUPERSONIC_JOIN_START_MACH`] needs them: building takes up to about 125 runs of the
    /// method.
    #[serde(skip)]
    supersonic: SupersonicTable,
    fin_sets: Vec<FinSetAero>,
    drag_terms: Vec<ComponentDragTerms>,
    drag_table: Option<DragTable>,
    normal_force_table: Option<NormalForceTable>,
}

impl AeroModel {
    /// Builds the terms of every component of `layout`, with hpr's current body model
    /// ([`BodyModel::default`]).
    ///
    /// # Errors
    ///
    /// As [`Self::with_body_model`].
    pub fn new(layout: &Layout) -> Result<Self, AeroError> {
        Self::with_body_model(layout, BodyModel::default())
    }

    /// Builds the terms of every component of `layout`, its bodies' lift and supersonic boattails
    /// by `body_model`.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a non-positive reference diameter, rocket length or body radius.
    /// - [`AeroError::InComponent`] naming the component, around:
    ///   - [`AeroError::Unsupported`] for tube fins, or a part kind or fin cross-section this model
    ///     doesn't know (a nose shape the drag buildup has no data for builds, and the buildup
    ///     refuses it when asked: [`AeroModel::drag`]);
    ///   - [`AeroError::Domain`] for a fin set of more than eight fins, a non-finite station, or a
    ///     drag input out of range (a negative fin thickness, a launch lug's wall thicker than its
    ///     radius, a rail button's base and flange taller than the button, a negative roughness);
    ///   - [`AeroError::Layout`] for a fin set without the radius of its body tube;
    ///   - design errors from a profile, a planform or a volume integral.
    /// - [`AeroError::Domain`] for a body-lift `K` that isn't finite and non-negative.
    pub fn with_body_model(layout: &Layout, body_model: BodyModel) -> Result<Self, AeroError> {
        body_model.body_lift.validate()?;
        check_dimension("reference diameter", layout.reference_diameter_m, false)?;
        let reference_area_m2 = layout.reference_area_m2();
        let length_m = layout.length_m;
        check_dimension("rocket length", length_m, false)?;
        let mut max_radius: f64 = 0.0;
        for component in layout.body() {
            if let Some(radius) = component.part.max_radius_m()? {
                max_radius = max_radius.max(radius);
            }
        }
        check_dimension("maximum body radius", max_radius, false)?;
        let fineness = length_m / (2.0 * max_radius);
        let form_factor = body_friction_form_factor(fineness)?;
        let mut bodies = Vec::new();
        let mut fin_sets = Vec::new();
        let mut drag_terms = Vec::new();
        let mut previous_aft_area: Option<f64> = None;
        let mut body_terms_at = Vec::new();
        let mut last_body_terms: Option<usize> = None;
        // The nose, the cylinders and boattails behind it, for the shock-expansion method: the
        // run stops at the first other body (a flare), step in radius or gap.
        let mut supersonic_segments = Vec::new();
        let mut supersonic_bounds = Vec::new();
        let mut supersonic_fore = Vec::new();
        let mut supersonic_boattails = Vec::new();
        let mut supersonic_open = true;
        let mut behind_boattail = false;
        let (mut vertex_m, mut supersonic_end_m) = (0.0, 0.0);
        for component in &layout.components {
            let in_component = |e: AeroError| AeroError::InComponent {
                id: component.id.clone(),
                source: Box::new(e),
            };
            if !component.fore_station_m.is_finite() {
                return Err(in_component(AeroError::Domain {
                    what: "component station",
                    value: component.fore_station_m,
                }));
            }
            let body = match &component.part {
                Part::NoseCone(nose) => Some(
                    nose.profile()
                        .map_err(AeroError::from)
                        .and_then(|p| BodyGeometry::from_profile(&p)),
                ),
                Part::Transition(transition) => Some(
                    transition
                        .profile()
                        .map_err(AeroError::from)
                        .and_then(|p| BodyGeometry::from_profile(&p)),
                ),
                Part::BodyTube(tube) => {
                    Some(BodyGeometry::cylinder(tube.length_m, tube.outer_radius_m))
                }
                Part::FinSet(set) => {
                    let terms = (|| {
                        let fin = FinAero::new(&set.planform, reference_area_m2)?;
                        let body_radius = component.body_radius_m.ok_or_else(|| {
                            AeroError::Layout(
                                "a fin set needs the radius of the body tube it is on".to_owned(),
                            )
                        })?;
                        // Past 15° a fin stalls, where the linear roll model means nothing, and a
                        // single canted fin pushes sideways, which the model doesn't carry.
                        if !set.cant_rad.is_finite() || set.cant_rad.abs() > MAX_CANT_RAD {
                            return Err(AeroError::Domain {
                                what: "fin cant",
                                value: set.cant_rad,
                            });
                        }
                        if set.count < 2 && set.cant_rad != 0.0 {
                            return Err(AeroError::Unsupported(
                                "cant on a single fin, whose side force isn't modelled".to_owned(),
                            ));
                        }
                        let span = fin.geometry().span_m;
                        let taper = fin.outline().tip_chord_m() / set.planform.root_chord_m();
                        let roll = fin.roll_terms(body_radius, layout.reference_diameter_m)?;
                        Ok(FinSetAero {
                            id: component.id.clone(),
                            count: set.count,
                            base_angle_rad: set.base_angle_rad,
                            count_factor: fin_count_factor(set.count)?,
                            interference: interference_factor(span, body_radius)?,
                            fore_station_m: component.fore_station_m,
                            cant_rad: set.cant_rad,
                            body_radius_m: body_radius,
                            roll_forcing_interference: roll_forcing_interference(
                                span,
                                body_radius,
                            )?,
                            roll_damping_interference: roll_damping_interference(
                                span,
                                body_radius,
                                taper,
                            )?,
                            roll,
                            fin,
                        })
                    })()
                    .map_err(in_component)?;
                    drag_terms.push(
                        ComponentDragTerms::fins(
                            component,
                            set,
                            terms.fin.geometry(),
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    fin_sets.push(terms);
                    None
                }
                Part::TubeFinSet(_) => {
                    return Err(in_component(AeroError::Unsupported(
                        "tube fins (no cited normal-force method yet)".to_owned(),
                    )));
                }
                // Drag only.
                Part::LaunchLug(lug) => {
                    drag_terms.push(
                        ComponentDragTerms::launch_lugs(
                            component,
                            lug,
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    None
                }
                Part::RailButton(button) => {
                    drag_terms.push(
                        ComponentDragTerms::rail_buttons(
                            component,
                            button,
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    None
                }
                // Inside the body.
                Part::InnerTube(_)
                | Part::CenteringRing(_)
                | Part::MassComponent(_)
                | Part::Parachute(_)
                | Part::Streamer(_)
                | Part::ShockCord(_) => None,
                other => {
                    return Err(in_component(AeroError::Unsupported(format!(
                        "a {} part",
                        other.kind_name()
                    ))));
                }
            };
            if let Some(geometry) = body {
                let geometry = geometry.map_err(in_component)?;
                let step = previous_aft_area.map_or(0.0, |aft| geometry.fore_area_m2 - aft);
                last_body_terms = Some(drag_terms.len());
                drag_terms.push(
                    ComponentDragTerms::body(
                        component,
                        &geometry,
                        match &component.part {
                            Part::NoseCone(nose) => Some(nose.shape),
                            Part::Transition(transition) => Some(transition.shape),
                            _ => None,
                        },
                        previous_aft_area,
                        form_factor,
                        length_m,
                        reference_area_m2,
                    )
                    .map_err(in_component)?,
                );
                body_terms_at.push((drag_terms.len() - 1, geometry));
                let segment = match &component.part {
                    Part::NoseCone(nose) if bodies.is_empty() => nose
                        .profile()
                        .ok()
                        .map(|profile| BodySegment::Profile { profile }),
                    Part::BodyTube(tube)
                        if !bodies.is_empty()
                            && step.abs() <= 1e-6 * geometry.fore_area_m2
                            && (component.fore_station_m - supersonic_end_m).abs()
                                <= 1e-9 * length_m =>
                    {
                        Some(BodySegment::Cylinder {
                            length_m: tube.length_m,
                            radius_m: tube.outer_radius_m,
                        })
                    }
                    // A boattail: footnote 8's tangent cone for its elements.
                    Part::Transition(transition)
                        if !bodies.is_empty()
                            && transition.aft_radius_m < transition.fore_radius_m
                            && step.abs() <= 1e-6 * geometry.fore_area_m2
                            && (component.fore_station_m - supersonic_end_m).abs()
                                <= 1e-9 * length_m =>
                    {
                        transition
                            .profile()
                            .ok()
                            .map(|profile| BodySegment::Profile { profile })
                    }
                    _ => None,
                };
                match segment {
                    Some(segment) if supersonic_open => {
                        if supersonic_segments.is_empty() {
                            vertex_m = component.fore_station_m;
                        }
                        // Washington and Pettis's boattail: the run so far with a cylinder of its
                        // length and fore radius in its place.
                        supersonic_boattails.push(match (&component.part, body_model) {
                            (
                                Part::Transition(transition),
                                BodyModel {
                                    supersonic_boattail: SupersonicBoattail::WashingtonPettis,
                                    ..
                                },
                            ) => {
                                let mut in_its_place = supersonic_segments.clone();
                                in_its_place.push(BodySegment::Cylinder {
                                    length_m: transition.length_m,
                                    radius_m: transition.fore_radius_m,
                                });
                                Some(RunBoattail {
                                    in_its_place,
                                    fore_radius_m: transition.fore_radius_m,
                                    aft_radius_m: transition.aft_radius_m,
                                    length_m: transition.length_m,
                                })
                            }
                            _ => None,
                        });
                        supersonic_segments.push(segment);
                        let fore_m = component.fore_station_m;
                        supersonic_fore.push(fore_m);
                        supersonic_end_m = fore_m + geometry.length_m;
                        // From a boattail aft, the shares may cross zero: the tail behind it
                        // carries the decay of the boattail's expansion.
                        behind_boattail |= matches!(&component.part, Part::Transition(_));
                        supersonic_bounds
                            .push((!behind_boattail).then_some((fore_m, supersonic_end_m)));
                    }
                    _ => supersonic_open = false,
                }
                previous_aft_area = Some(geometry.aft_area_m2);
                bodies.push(body_terms(component, geometry, step, reference_area_m2));
            }
        }
        // The aft base belongs to the last body component.
        if let (Some(index), Some(last)) = (last_body_terms, bodies.last()) {
            drag_terms[index].base_area_m2 = last.geometry.aft_area_m2;
        }
        // Boattails, a lip in a boattail's wake, and the base behind them.
        couple_afterbody(&mut drag_terms, &body_terms_at, reference_area_m2)?;
        // The method flies only a body it covers to the end, or whose later bodies carry no
        // potential-flow slope: its shares beside slender-body theory's for a flare or step would
        // mix the models the way a boattail did before M1.8e4 (physics review, ADR-034).
        let marched = supersonic_segments.len();
        // A lip wholly in a covered boattail's wake carries nothing faster than sound (ADR-039),
        // so the run may cover it; the drag buildup's wake already measures the shelter
        // ([`crate::drag::WakeTerm`]), and takes the lip's own drag away at the same threshold.
        let mut shelter_weight = 1.0_f64;
        let sheltered_lips = (marched..bodies.len())
            .take_while(|&index| {
                let (term, geometry) = body_terms_at[index];
                // A lip widens the body: a narrowing part behind the run is a boattail the method
                // hasn't covered, whatever its drag takes from the wake, and it keeps its
                // slender-body share.
                if bodies[index].slope_per_rad <= 0.0 {
                    return false;
                }
                // It must be short enough to stay in the wake, whose scale is the boattail's own
                // drop in diameter; a longer flare grows out of it.
                let Some(wake) = drag_terms[term].in_wake_of else {
                    return false;
                };
                let boattail = &wake.boattail;
                if geometry.length_m > boattail.fore_diameter_m - boattail.aft_diameter_m {
                    return false;
                }
                // Whatever it widens by, a step at its fore end or its own shoulder, must be
                // wholly in the wake. The geometry says which it has: a drag term can be missing
                // for reasons of its own (a shape the buildup has no curve for).
                let steps = body_terms_at[index.saturating_sub(1)]
                    .1
                    .aft_area_m2
                    .lt(&geometry.fore_area_m2);
                let shoulders = geometry.aft_area_m2 > geometry.fore_area_m2;
                // How much of it the wake covers: the drag buildup's whole fraction, which fades
                // with the lip's rise (a quarter of the boattail's drop in diameter to a half),
                // with any tube between it and the boattail, and with anything else in the way
                // (`crate::drag::WakeTerm`). The normal force now reads the same
                // number rather than a threshold on it: the method's share is weighed by it
                // ([`SupersonicBody::weight`]), so a lip drawn a hair taller no longer switches
                // the whole body between the two models (issue #87, ADR-041 in DECISIONS.md).
                let mut covered = 1.0_f64;
                if steps {
                    covered = covered.min(wake.step_fraction);
                }
                if shoulders {
                    covered = covered.min(wake.shoulder_fraction);
                }
                // Not `<=`, so a share that somehow came out NaN falls out of the wake rather
                // than reading as full shelter through `f64::min`.
                if !covered.is_finite() || covered <= SUPERSONIC_SHELTER_FLOOR {
                    return false;
                }
                shelter_weight = shelter_weight.min(covered);
                true
            })
            .count();
        let covered = marched + sheltered_lips;
        let rest_carries_nothing = bodies[covered..]
            .iter()
            .all(|body| body.slope_per_rad.abs() <= 1e-9);
        let supersonic_run = (marched > 0 && rest_carries_nothing).then_some(SupersonicRun {
            segments: supersonic_segments,
            vertex_m,
            bounds_m: supersonic_bounds,
            fore_m: supersonic_fore,
            boattails: supersonic_boattails,
            sheltered_lips,
            shape_weight: shelter_weight,
        });
        Ok(Self {
            reference_area_m2,
            reference_diameter_m: layout.reference_diameter_m,
            length_m,
            max_body_radius_m: max_radius,
            fineness,
            crossflow_eta_low: crate::crossflow::crossflow_eta_low(fineness),
            body_model,
            bodies,
            supersonic_run,
            supersonic: SupersonicTable::default(),
            fin_sets,
            drag_terms,
            drag_table: None,
            normal_force_table: None,
        })
    }

    /// This model with `table` replacing the drag buildup's zero-lift drag
    /// ([`crate::table`]).
    #[must_use]
    pub fn with_drag_table(mut self, table: DragTable) -> Self {
        self.drag_table = Some(table);
        self
    }

    /// The drag override table, if any.
    pub fn drag_table(&self) -> Option<&DragTable> {
        self.drag_table.as_ref()
    }

    /// This model with `table` replacing the whole rocket's normal force and centre of pressure
    /// ([`AeroModel::normal_force`]). Each component's own terms stay hpr's
    /// ([`AeroModel::components`], [`AeroModel::component_normal_force`]): a flight engine takes
    /// its pitch and yaw damping from them, which a table doesn't give.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a centre of pressure in the table outside the rocket, from its
    /// nose tip to its aft end: the sign of a length in the wrong unit or from another datum. Only
    /// the Mach numbers a flight can use are checked, up to [`NORMAL_FORCE_MACH_LIMIT`] and the
    /// first past it; a hypersonic row may move where it likes.
    pub fn with_normal_force_table(mut self, table: NormalForceTable) -> Result<Self, AeroError> {
        for column in table.columns() {
            let knots = column.cp_station_m.xs();
            let used = knots.partition_point(|&mach| mach <= NORMAL_FORCE_MACH_LIMIT) + 1;
            for &cp in column.cp_station_m.ys().iter().take(used) {
                if !(0.0..=self.length_m).contains(&cp) {
                    return Err(AeroError::Domain {
                        what: "normal-force table centre of pressure, m aft of the nose tip",
                        value: cp,
                    });
                }
            }
        }
        self.normal_force_table = Some(table);
        Ok(self)
    }

    /// The normal-force override table, if any.
    pub fn normal_force_table(&self) -> Option<&NormalForceTable> {
        self.normal_force_table.as_ref()
    }

    /// The components' precomputed drag terms, in layout order.
    pub fn drag_terms(&self) -> &[ComponentDragTerms] {
        &self.drag_terms
    }

    /// Rocket length for the Reynolds number: nose tip to the aft end of the last body component,
    /// m.
    pub fn length_m(&self) -> f64 {
        self.length_m
    }

    /// The whole rocket's drag at `flow` and `conditions`: the zero-lift drag of the buildup, or of
    /// the override table when there is one, and the axial coefficient at the flow's angle of
    /// attack.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Mach`] outside `[0, 5)` for the buildup
    ///   ([`crate::drag::BUILDUP_MACH_LIMIT`]); with an override table any finite Mach number from
    ///   0 is accepted ([`AeroError::Domain`] otherwise).
    /// - Without a table, [`AeroError::InComponent`] around [`AeroError::Unsupported`] for a nose or
    ///   shoulder shape the buildup has no drag data for
    ///   ([`crate::drag::ComponentDragTerms::unsupported`]).
    /// - [`AeroError::Domain`] for an angle of attack outside `[0, π]` or a non-finite roll.
    /// - As [`DragConditions::validate`].
    /// - [`AeroError::Table`] from the table lookup, and [`AeroError::Domain`] if the drag isn't
    ///   finite.
    pub fn drag(&self, flow: &Flow, conditions: &DragConditions) -> Result<Drag, AeroError> {
        conditions.validate()?;
        let factor = axial_drag_alpha_factor(flow.alpha_rad)?;
        let mut drag = if let Some(table) = &self.drag_table {
            flow.validate_angles()?;
            let lookup = table.lookup(flow.mach, conditions.thrusting)?;
            let scale = match table.reference_diameter_m {
                Some(d) => {
                    check_dimension("drag table reference diameter", d, false)?;
                    0.25 * PI * d * d / self.reference_area_m2
                }
                None => 1.0,
            };
            Drag {
                zero_lift_coefficient: lookup.value * scale,
                table: Some(lookup),
                ..Drag::default()
            }
        } else {
            flow.validate_for_buildup()?;
            let reynolds = conditions.reynolds_per_m * self.length_m;
            let mut sum = Drag::default();
            for terms in &self.drag_terms {
                let d = terms.evaluate(
                    reynolds,
                    flow.mach,
                    conditions.thrusting_motor_area_m2,
                    self.reference_area_m2,
                )?;
                sum.friction += d.friction;
                sum.pressure += d.pressure;
                sum.base += d.base;
                sum.parasitic += d.parasitic;
            }
            sum.zero_lift_coefficient = sum.friction + sum.pressure + sum.base + sum.parasitic;
            sum
        };
        drag.axial_coefficient = drag.zero_lift_coefficient * factor;
        if !(drag.zero_lift_coefficient.is_finite() && drag.axial_coefficient.is_finite()) {
            return Err(AeroError::Domain {
                what: "drag coefficient",
                value: drag.zero_lift_coefficient,
            });
        }
        Ok(drag)
    }

    /// Each component's share of the drag buildup at `flow` and `conditions`, in layout order: its
    /// zero-lift coefficient and parts, and its axial coefficient at the flow's angle of attack.
    ///
    /// These are always the buildup's terms. With an override table, [`AeroModel::drag`] returns
    /// the table's value instead of their sum, so they don't add up to it.
    ///
    /// # Errors
    ///
    /// As [`AeroModel::drag`] without a table, and [`DragConditions::validate`].
    pub fn buildup_components(
        &self,
        flow: &Flow,
        conditions: &DragConditions,
    ) -> Result<Vec<ComponentDrag>, AeroError> {
        flow.validate_for_buildup()?;
        conditions.validate()?;
        let factor = axial_drag_alpha_factor(flow.alpha_rad)?;
        let reynolds = conditions.reynolds_per_m * self.length_m;
        self.drag_terms
            .iter()
            .map(|terms| {
                let mut drag = terms.evaluate(
                    reynolds,
                    flow.mach,
                    conditions.thrusting_motor_area_m2,
                    self.reference_area_m2,
                )?;
                drag.axial_coefficient *= factor;
                Ok(ComponentDrag {
                    id: terms.id.clone(),
                    drag,
                })
            })
            .collect()
    }

    /// Reference area, m².
    pub fn reference_area_m2(&self) -> f64 {
        self.reference_area_m2
    }

    /// Reference diameter, m: the length the rolling moment is taken on.
    pub fn reference_diameter_m(&self) -> f64 {
        self.reference_diameter_m
    }

    /// The whole rocket's rolling moment at `mach` about `+z_B`, on the reference area and
    /// diameter: `C_l = C_l0 + C_lp (p d/2V)`, with `p` the roll rate about `+z_B` and `V` the
    /// airspeed. Each fin set adds `C_l0 = −N C_lδ k_T(B) δ` and `N C_lp k_R(B)` ([`FinAero::roll`],
    /// [`roll_forcing_interference`], [`roll_damping_interference`]); a positive cant `δ` turns
    /// each fin's leading edge toward `−y_B` at fin 0, so its lift rolls the rocket toward `−z_B`.
    /// Fin–fin interference is not applied to roll, as in Niskanen 2009 eq. 3.66. The bodies of
    /// revolution add nothing.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 5)`, as [`AeroModel::normal_force`].
    pub fn roll(&self, mach: f64) -> Result<Roll, AeroError> {
        check_mach(mach, NORMAL_FORCE_MACH_LIMIT, "the roll moment")?;
        let mut roll = Roll::default();
        for set in &self.fin_sets {
            let fin = set.fin.roll_with(&set.roll, mach);
            let n = f64::from(set.count);
            roll.forcing -= n * fin.forcing_per_rad * set.roll_forcing_interference * set.cant_rad;
            roll.damping += n * fin.damping * set.roll_damping_interference;
        }
        Ok(roll)
    }

    /// The steady roll rate about `+z_B`, rad/s, at `mach` and airspeed `speed_m_s` in axial flow:
    /// where the fins' forcing and damping balance, `p = −(C_l0/C_lp)(2V/d)`. Zero with no fins.
    ///
    /// # Errors
    ///
    /// As [`AeroModel::roll`], and [`AeroError::Domain`] for a negative or non-finite airspeed.
    pub fn steady_roll_rate_rad_s(&self, mach: f64, speed_m_s: f64) -> Result<f64, AeroError> {
        check_dimension("airspeed", speed_m_s, true)?;
        let roll = self.roll(mach)?;
        Ok(if roll.damping < 0.0 {
            -roll.forcing / roll.damping * 2.0 * speed_m_s / self.reference_diameter_m
        } else {
            0.0
        })
    }

    /// The body components' terms.
    pub fn bodies(&self) -> &[BodyAero] {
        &self.bodies
    }

    /// The shock-expansion method's shares of the nose, the cylinders and boattails behind it,
    /// and where they join slender-body theory; `None` where the method can't take the body (a nose
    /// steeper than a blunt tip's handover all the way to its base, a tangent cone past TN 3527's
    /// Fig. 2, a later body with a slope of its own such as a flare) or doesn't hold across a
    /// whole join below Mach 5.
    ///
    /// The first call builds the table, which takes up to about 125 runs of the method; a flow no
    /// faster than [`SUPERSONIC_JOIN_START_MACH`] never needs it.
    pub fn supersonic_body(&self) -> Option<&SupersonicBody> {
        let run = self.supersonic_run.as_ref()?;
        self.supersonic
            .0
            .get_or_init(|| SupersonicBody::new(run, self.reference_area_m2))
            .as_ref()
    }

    /// [`Self::supersonic_body`] where `mach` is past the earliest join, and `None` below it
    /// without building the table.
    fn supersonic_at(&self, mach: f64) -> Option<&SupersonicBody> {
        (mach > SUPERSONIC_JOIN_START_MACH)
            .then(|| self.supersonic_body())
            .flatten()
    }

    /// Body `index`'s potential-flow slope (per radian) and moment slope about the nose tip (m per
    /// radian) at a checked `mach`: slender-body theory's, joined to the shock-expansion share
    /// where the method covers it ([`SupersonicBody`]).
    fn body_potential(&self, index: usize, body: &BodyAero, mach: f64) -> (f64, f64) {
        let (slope, moment) = (body.slope_per_rad, body.moment_slope_m);
        match self.supersonic_at(mach) {
            Some(s) if index < s.covered && mach > s.join_start_mach => {
                let w = s.weight(mach);
                let Some((se_slope, se_moment)) = s.share(index, mach) else {
                    return (slope, moment);
                };
                (
                    slope + w * (se_slope - slope),
                    moment + w * (se_moment - moment),
                )
            }
            _ => (slope, moment),
        }
    }

    /// The body-lift factor at `flow`, on `(A_plan/A_ref) sin² α`: Jorgensen's `η C_dn` at the
    /// body's fineness and the crossflow Mach number `M sin α`, or Galejs's `K`
    /// ([`BodyModel::body_lift`]).
    pub fn body_lift_factor(&self, flow: &Flow) -> f64 {
        self.lift_factor_at(flow.mach, flow.alpha_rad.sin())
    }

    /// [`Self::body_lift_factor`] at `mach` and `sin α`.
    fn lift_factor_at(&self, mach: f64, sin_alpha: f64) -> f64 {
        let crossflow_mach = mach * sin_alpha.abs();
        match self.body_model.body_lift {
            BodyLift::Jorgensen {} => crate::crossflow::crossflow_factor_from_eta_low(
                self.crossflow_eta_low,
                crossflow_mach,
            ),
            other => other.factor(self.fineness, crossflow_mach),
        }
    }

    /// The per-radian potential-flow and body-lift factors at `flow` ([`alpha_factors`]), the
    /// second with the model's body-lift factor, skipped where it multiplies nothing.
    fn body_factors(&self, flow: &Flow) -> (f64, f64) {
        let (potential, lift, sin_alpha) = alpha_factors(flow.alpha_rad);
        if lift == 0.0 {
            (potential, 0.0)
        } else {
            (potential, lift * self.lift_factor_at(flow.mach, sin_alpha))
        }
    }

    /// The body model: its body-lift and supersonic-boattail rules.
    pub fn body_model(&self) -> BodyModel {
        self.body_model
    }

    /// The body's length over its largest diameter, which sets body lift's `η`.
    pub fn fineness(&self) -> f64 {
        self.fineness
    }

    /// The fin sets' terms.
    pub fn fin_sets(&self) -> &[FinSetAero] {
        &self.fin_sets
    }

    /// Each component's id and contributions at a validated `flow`: bodies first, then fin sets,
    /// in layout order.
    fn terms<'a>(&'a self, flow: &Flow) -> impl Iterator<Item = (&'a str, Term)> + 'a {
        let (potential, lift) = self.body_factors(flow);
        let (mach, roll) = (flow.mach, flow.roll_rad);
        let bodies = self.bodies.iter().enumerate().map(move |(index, body)| {
            let (slope, moment) = self.body_potential(index, body, mach);
            (
                body.id.as_str(),
                body_term(body, slope, moment, potential, lift),
            )
        });
        let fins = self
            .fin_sets
            .iter()
            .map(move |set| (set.id.as_str(), fin_term(set, mach, roll)));
        bodies.chain(fins)
    }

    /// The number of components with a normal-force term: the bodies, then the fin sets, in the
    /// order of [`Self::components`].
    pub fn component_count(&self) -> usize {
        self.bodies.len() + self.fin_sets.len()
    }

    /// Component `index`'s normal force at `flow`, in the order of [`Self::components`], without
    /// allocating. A flight engine evaluates each component at its own local flow, which includes
    /// the airspeed the body's rotation adds at the component.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`], and [`AeroError::Domain`] for an index past
    /// [`Self::component_count`].
    pub fn component_normal_force(
        &self,
        index: usize,
        flow: &Flow,
    ) -> Result<NormalForce, AeroError> {
        flow.validate()?;
        let term = if let Some(body) = self.bodies.get(index) {
            let (potential, lift) = self.body_factors(flow);
            let (slope, moment) = self.body_potential(index, body, flow.mach);
            body_term(body, slope, moment, potential, lift)
        } else if let Some(set) = self.fin_sets.get(index - self.bodies.len()) {
            fin_term(set, flow.mach, flow.roll_rad)
        } else {
            return Err(AeroError::Domain {
                what: "component index",
                value: index as f64,
            });
        };
        Ok(NormalForce::new(term, flow.alpha_rad))
    }

    /// The station, m aft of the nose tip, of component `index`'s small-angle centre of
    /// pressure at `mach`: where a flight engine takes the component's local airspeed. A body with
    /// no potential-flow slope (a cylinder) uses its body-lift station; a fin set's moves with
    /// Mach, and so does a nose's or cylinder's that the shock-expansion method covers, joined
    /// linearly from slender-body theory's station as its slope is ([`SupersonicBody`]). A covered
    /// boattail, and a cylinder behind it, keep slender-body theory's station: their shares may
    /// cross zero, where a station would run off to infinity.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 5)`, and [`AeroError::Domain`] for an index past
    /// [`Self::component_count`].
    pub fn component_station_m(&self, index: usize, mach: f64) -> Result<f64, AeroError> {
        check_mach(mach, NORMAL_FORCE_MACH_LIMIT, "the normal force")?;
        if let Some(body) = self.bodies.get(index) {
            // As `NormalForce`'s CP: a slope that cancels to rounding (a step in radius offsetting
            // a taper) has no potential-flow station.
            let step_slope = 2.0 * body.step_area_m2 / self.reference_area_m2;
            let scale = (body.slope_per_rad - step_slope).abs() + step_slope.abs();
            let station = if body.slope_per_rad.abs() <= 1e-12 * scale {
                body.lift_station_m
            } else {
                body.moment_slope_m / body.slope_per_rad
            };
            // A boattail's share, and those behind it, may cross zero, so their stations stay
            // slender-body theory's, on their own segments, while slopes and moments take the
            // method's.
            Ok(match self.supersonic_at(mach) {
                Some(s) if index < s.covered && s.stationed[index] && mach > s.join_start_mach => {
                    match s.share(index, mach) {
                        // Every tabulated share with a station is positive, so the interpolated
                        // one is.
                        Some((slope, moment)) => {
                            station + s.weight(mach) * (moment / slope - station)
                        }
                        None => station,
                    }
                }
                _ => station,
            })
        } else {
            let set = self
                .fin_sets
                .get(index - self.bodies.len())
                .ok_or(AeroError::Domain {
                    what: "component index",
                    value: index as f64,
                })?;
            Ok(set.fore_station_m + set.fin.loading_at(mach).cp_m)
        }
    }

    /// The whole rocket's normal force at `flow`: the sum of its components, or the override
    /// table's when there is one ([`AeroModel::with_normal_force_table`]).
    ///
    /// A table's normal force acts in the plane of the flow at the table's centre of pressure,
    /// with no side force; its coefficients are rescaled to the rocket's reference area from the
    /// table's ([`crate::table::TableReference`]).
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`]. With a table, any finite Mach number from 0 is accepted
    /// ([`AeroError::Domain`] otherwise), and table errors are returned.
    pub fn normal_force(&self, flow: &Flow) -> Result<NormalForce, AeroError> {
        if let Some(table) = &self.normal_force_table {
            flow.validate_angles()?;
            let lookup = table.lookup_within(flow.mach, flow.alpha_rad, (0.0, self.length_m))?;
            let area_m2 = match table.reference() {
                TableReference::Diameter { diameter_m } => 0.25 * PI * diameter_m * diameter_m,
                TableReference::LargestBody => PI * self.max_body_radius_m * self.max_body_radius_m,
                // `TableReference` is non-exhaustive only for other crates.
                TableReference::Rocket => self.reference_area_m2,
            };
            let scale = area_m2 / self.reference_area_m2;
            let coefficient = lookup.coefficient * scale;
            let slope = lookup.slope_per_rad * scale;
            return Ok(NormalForce {
                coefficient,
                slope_per_rad: slope,
                moment_m: coefficient * lookup.cp_station_m,
                cp_station_m: (slope != 0.0).then_some(lookup.cp_station_m),
                side_coefficient: 0.0,
                side_moment_m: 0.0,
                table: Some(lookup),
            });
        }
        flow.validate()?;
        let total = self
            .terms(flow)
            .fold(Term::default(), |sum, (_, term)| sum.add(term));
        Ok(NormalForce::new(total, flow.alpha_rad))
    }

    /// Each component's normal force at `flow`, bodies first, then fin sets, in layout order. A
    /// step in radius is part of the component aft of it.
    ///
    /// These are always hpr's own terms. With a normal-force table, [`AeroModel::normal_force`]
    /// returns the table's value instead of their sum.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`].
    pub fn components(&self, flow: &Flow) -> Result<Vec<ComponentNormalForce>, AeroError> {
        flow.validate()?;
        Ok(self
            .terms(flow)
            .map(|(id, term)| ComponentNormalForce {
                id: id.to_owned(),
                normal_force: NormalForce::new(term, flow.alpha_rad),
            })
            .collect())
    }
}

/// A fin set's contribution at a checked `mach` and flow roll `roll`, per radian of `α`.
fn fin_term(set: &FinSetAero, mach: f64, roll: f64) -> Term {
    let FinLoading {
        slope_per_rad,
        cp_m,
    } = set.fin.loading_at(mach);
    let per_set = slope_per_rad * set.count_factor * set.interference;
    let station = set.fore_station_m + cp_m;
    let slope = per_set * roll_sum(set.count, set.base_angle_rad, roll);
    let side = per_set * side_sum(set.count, set.base_angle_rad, roll);
    Term {
        slope,
        moment: slope * station,
        side,
        side_moment: side * station,
        scale: slope.abs(),
    }
}

/// A body's contribution from its potential-flow `slope` and `moment` slope at the flow's Mach
/// number ([`AeroModel::body_potential`]), at the potential-flow factor of [`alpha_factors`] and
/// its body-lift factor times the model's ([`AeroModel::body_lift_factor`]).
fn body_term(body: &BodyAero, slope: f64, moment: f64, potential: f64, lift: f64) -> Term {
    let (attached, lift) = (slope * potential, body.planform_ratio * lift);
    Term {
        slope: attached + lift,
        moment: moment * potential + lift * body.lift_station_m,
        scale: attached.abs() + lift.abs(),
        ..Term::default()
    }
}

/// The per-radian factors of the potential-flow term (`sin α/α`) and of body lift
/// (`sin² α/α = sin α · sin α/α`), and `sin α`.
fn alpha_factors(alpha_rad: f64) -> (f64, f64, f64) {
    let (s, sin) = (sinc(alpha_rad), alpha_rad.sin());
    (s, sin * s, sin)
}

fn body_terms(
    component: &PlacedComponent,
    geometry: BodyGeometry,
    step_area_m2: f64,
    a_ref: f64,
) -> BodyAero {
    let station = component.fore_station_m;
    let step_slope = 2.0 * step_area_m2 / a_ref;
    let slope = geometry.normal_force_slope(a_ref);
    BodyAero {
        id: component.id.clone(),
        fore_station_m: station,
        geometry,
        step_area_m2,
        slope_per_rad: slope + step_slope,
        moment_slope_m: (slope + step_slope) * station + geometry.moment_slope_m(a_ref),
        planform_ratio: geometry.planform_area_m2 / a_ref,
        lift_station_m: station + geometry.planform_centroid_m,
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

    use hpr_design::{
        FinPlanform, LaunchLug, NoseShape, Part, Position, ReferenceDiameter, TubeFinSet,
    };
    use proptest::prelude::*;

    use super::*;
    use crate::BODY_LIFT_K;
    use crate::testing::{body_part, component, fin_set, finned_rocket, material, nose, one_stage};

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = if want == 0.0 {
            got.abs()
        } else {
            ((got - want) / want).abs()
        };
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    fn model(rocket: &hpr_design::Rocket) -> AeroModel {
        AeroModel::new(&rocket.layout().unwrap()).unwrap()
    }

    fn flow(mach: f64, alpha_rad: f64, roll_rad: f64) -> Flow {
        Flow::new(mach, alpha_rad, roll_rad)
    }

    /// A cone on a cylinder, broadside and at small angles: the potential term scales with
    /// `sin α`, body lift with `sin² α` at the planform centroids (a cone's `½ L D` at `2L/3`, a
    /// cylinder's `L D` at its middle; Galejs Table 1) times the model's factor at the flow's
    /// crossflow Mach number, and the slope at `α → 0` is the sum of the Barrowman slopes.
    #[test]
    fn angle_of_attack_terms() {
        let (l_n, l_t, r) = (0.2, 0.8, 0.03);
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, l_n, r), None),
                component("tube", body_part(l_t, r, r), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_ref = PI * r * r;
        // Jorgensen's η C_dn at fineness 1.0/0.06 and the crossflow Mach number M sin α.
        let k = |alpha: f64| crate::crossflow::crossflow_factor(1.0 / 0.06, 0.3 * alpha.sin());
        close(m.fineness(), 1.0 / 0.06, 1e-15, "fineness");
        let lift_nose = k(FRAC_PI_2) * r * l_n / a_ref;
        let lift_tube = k(FRAC_PI_2) * 2.0 * r * l_t / a_ref;

        let broadside = m.normal_force(&flow(0.3, FRAC_PI_2, 0.0)).unwrap();
        let want = 2.0 + lift_nose + lift_tube;
        close(broadside.coefficient, want, 1e-10, "C_N at 90°");
        let moment =
            2.0 * (2.0 * l_n / 3.0) + lift_nose * (2.0 * l_n / 3.0) + lift_tube * (l_n + 0.5 * l_t);
        close(
            broadside.cp_station_m.unwrap(),
            moment / want,
            1e-10,
            "CP at 90°",
        );

        let zero = m.normal_force(&Flow::axial(0.3)).unwrap();
        assert_eq!(zero.coefficient, 0.0);
        close(zero.slope_per_rad, 2.0, 1e-15, "slope at 0");
        // C_N(α)/α tends to the slope, with body lift adding (η C_dn A_plan/A_ref) α.
        for alpha in [1e-6, 1e-3, 0.05] {
            let f = m.normal_force(&flow(0.3, alpha, 0.0)).unwrap();
            let lift = (lift_nose + lift_tube) * k(alpha) / k(FRAC_PI_2);
            let want = 2.0 * alpha.sin() + lift * alpha.sin().powi(2);
            close(f.coefficient, want, 1e-12, "C_N");
            close(f.slope_per_rad, want / alpha, 1e-12, "C_N/α");
        }
        // Body lift pulls the CP aft as α grows.
        let cp = |alpha| {
            m.normal_force(&flow(0.3, alpha, 0.0))
                .unwrap()
                .cp_station_m
                .unwrap()
        };
        assert!(cp(0.02) > cp(0.0) && cp(0.2) > cp(0.02));
        // The components add up.
        let parts = m.components(&flow(0.3, 0.2, 0.0)).unwrap();
        let total = m.normal_force(&flow(0.3, 0.2, 0.0)).unwrap();
        let sum: f64 = parts.iter().map(|c| c.normal_force.coefficient).sum();
        close(sum, total.coefficient, 1e-14, "component sum");
    }

    /// Galejs's constant, hpr's body lift before M1.8e6, is `K` = 1.1 at any flow; Jorgensen's
    /// is below it at low crossflow Mach number and above it near `M sin α` = 1.
    #[test]
    fn body_lift_models() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.03), None),
                component("tube", body_part(0.8, 0.03, 0.03), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let layout = rocket.layout().unwrap();
        let old = AeroModel::with_body_model(&layout, BodyModel::BEFORE_M1_8E6).unwrap();
        let new = AeroModel::new(&layout).unwrap();
        assert_eq!(new.body_model(), BodyModel::default());
        for (mach, alpha) in [(0.3, 0.1), (2.0, 0.5), (4.0, 1.2)] {
            assert_eq!(old.body_lift_factor(&flow(mach, alpha, 0.0)), BODY_LIFT_K);
        }
        let slow = new.body_lift_factor(&flow(0.3, 0.1, 0.0));
        assert!(slow > 0.85 && slow < 0.9, "{slow}");
        let near_one = new.body_lift_factor(&flow(2.0, 0.5, 0.0));
        assert!(near_one > 1.4, "{near_one}");
        let k = BodyModel::BEFORE_M1_8E6.with_body_lift(BodyLift::Galejs { k: -0.5 });
        assert!(AeroModel::with_body_model(&layout, k).is_err());
        // The precomputed Fig. 4 `η` gives the library function's factor.
        for (mach, alpha) in [(0.3, 0.1), (2.0, 0.5), (4.0, 1.2)] {
            let f = flow(mach, alpha, 0.0);
            assert_eq!(
                new.body_lift_factor(&f),
                crate::crossflow::crossflow_factor(new.fineness(), mach * alpha.sin())
            );
        }
    }

    /// The body model's JSON form is a file format: it round-trips, a missing field takes the
    /// current choice, and an unknown one is refused.
    #[test]
    fn body_model_in_json() {
        assert_eq!(BodyModel::default(), BodyModel::CURRENT);
        let old = serde_json::to_string(&BodyModel::BEFORE_M1_8E6).unwrap();
        assert_eq!(
            old,
            r#"{"body_lift":{"kind":"galejs","k":1.1},"supersonic_boattail":"footnote8"}"#
        );
        assert_eq!(
            serde_json::from_str::<BodyModel>(&old).unwrap(),
            BodyModel::BEFORE_M1_8E6
        );
        assert_eq!(
            serde_json::from_str::<BodyModel>(r#"{"supersonic_boattail":"washington_pettis"}"#)
                .unwrap(),
            BodyModel::CURRENT
        );
        assert_eq!(
            serde_json::from_str::<BodyModel>("{}").unwrap(),
            BodyModel::CURRENT
        );
        for bad in [
            r#"{"boattail":"footnote8"}"#,
            r#"{"supersonic_boattail":"slender_body"}"#,
        ] {
            assert!(serde_json::from_str::<BodyModel>(bad).is_err(), "{bad}");
        }
    }

    /// Through subsonic flow, Mach changes the fins' slope by Prandtl–Glauert and nothing else;
    /// the bodies and every CP stay put.
    #[test]
    fn mach_changes_only_the_fins() {
        let m = model(&finned_rocket(4));
        let at = |mach| m.components(&Flow::axial(mach)).unwrap();
        let (slow, fast) = (at(0.0), at(0.8));
        for (a, b) in slow.iter().zip(&fast) {
            assert_eq!(
                a.normal_force.cp_station_m, b.normal_force.cp_station_m,
                "{}",
                a.id
            );
            if a.id == "fins" {
                let set = &m.fin_sets()[0];
                let ratio = set
                    .fin
                    .geometry()
                    .single_fin_slope(m.reference_area_m2(), 0.8)
                    .unwrap()
                    / set
                        .fin
                        .geometry()
                        .single_fin_slope(m.reference_area_m2(), 0.0)
                        .unwrap();
                assert!(ratio > 1.05, "{ratio}");
                close(
                    b.normal_force.slope_per_rad / a.normal_force.slope_per_rad,
                    ratio,
                    1e-14,
                    "fin ratio",
                );
            } else {
                assert_eq!(
                    a.normal_force.slope_per_rad, b.normal_force.slope_per_rad,
                    "{}",
                    a.id
                );
            }
        }
    }

    /// Four fins don't care about roll; two fins lift only when the flow crosses them.
    #[test]
    fn two_fin_sets_depend_on_roll() {
        let four = model(&finned_rocket(4));
        let slope =
            |m: &AeroModel, roll| m.normal_force(&flow(0.2, 0.0, roll)).unwrap().slope_per_rad;
        close(slope(&four, 0.0), slope(&four, 0.4), 1e-15, "four fins");

        let two = model(&finned_rocket(2));
        let bodies: f64 = two.bodies().iter().map(|b| b.slope_per_rad).sum();
        let set = &two.fin_sets()[0];
        let one_fin = set
            .fin
            .geometry()
            .single_fin_slope(two.reference_area_m2(), 0.2)
            .unwrap()
            * set.interference;
        // Flow along the fins' plane: no fin force. Across it: both fins at sin² = 1.
        close(slope(&two, 0.0), bodies, 1e-13, "along the fins");
        close(
            slope(&two, FRAC_PI_2),
            bodies + 2.0 * one_fin,
            1e-13,
            "across the fins",
        );
        // A two-fin set across the flow matches four fins' N/2 = 2.
        close(
            slope(&two, FRAC_PI_2),
            slope(&four, 0.0),
            1e-13,
            "two across = four",
        );
    }

    /// Refusals: tube fins, nine fins, Mach 5, angles outside `[0, π]`. Lugs add no normal force.
    #[test]
    fn unsupported_inputs_are_refused() {
        let mut rocket = crate::testing::finned_rocket(4);
        rocket.stages[0].components[3].children.push(component(
            "tube-fins",
            Part::TubeFinSet(TubeFinSet {
                count: 6,
                length_m: 0.1,
                outer_radius_m: 0.01,
                thickness_m: 0.001,
                base_angle_rad: 0.0,
                material: material(),
            }),
            Some(Position::Bottom { aft_offset_m: 0.0 }),
        ));
        let err = AeroModel::new(&rocket.layout().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, source } if id == "tube-fins"
                && matches!(**source, AeroError::Unsupported(_))),
            "{err}"
        );

        let err = AeroModel::new(&finned_rocket(9).layout().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, .. } if id == "fins"),
            "{err}"
        );

        let m = model(&finned_rocket(4));
        for bad in [
            flow(NORMAL_FORCE_MACH_LIMIT, 0.0, 0.0),
            flow(-0.01, 0.0, 0.0),
            flow(f64::NAN, 0.0, 0.0),
        ] {
            assert!(matches!(
                m.normal_force(&bad),
                Err(AeroError::Mach { limit, .. }) if limit == NORMAL_FORCE_MACH_LIMIT
            ));
        }
        // Both fly on past Mach 1 and stop at 5, each naming itself (since M1.8b1).
        assert!(m.normal_force(&flow(1.0, 0.1, 0.0)).is_ok());
        let coasting = DragConditions::coasting(1e7);
        assert!(m.drag(&flow(1.0, 0.0, 0.0), &coasting).is_ok());
        assert!(matches!(
            m.drag(&flow(5.0, 0.0, 0.0), &coasting),
            Err(AeroError::Mach { limit, model, .. })
                if limit == BUILDUP_MACH_LIMIT && model == "the drag buildup"
        ));
        assert!(
            m.buildup_components(&flow(5.0, 0.0, 0.0), &coasting)
                .is_err()
        );
        for bad in [
            flow(0.3, -1e-9, 0.0),
            flow(0.3, PI + 1e-9, 0.0),
            flow(0.3, f64::NAN, 0.0),
            flow(0.3, 0.1, f64::INFINITY),
        ] {
            assert!(matches!(
                m.normal_force(&bad),
                Err(AeroError::Domain { .. })
            ));
            assert!(m.components(&bad).is_err());
        }

        let mut lugged = finned_rocket(4);
        lugged.stages[0].components[1].children.push(component(
            "lug",
            Part::LaunchLug(LaunchLug {
                length_m: 0.05,
                outer_radius_m: 0.004,
                thickness_m: 0.0005,
                angle_rad: 0.0,
                count: 1,
                spacing_m: 0.0,
                material: material(),
            }),
            Some(Position::Middle { aft_offset_m: 0.0 }),
        ));
        let with = model(&lugged).normal_force(&flow(0.3, 0.1, 0.0)).unwrap();
        assert_eq!(with, m.normal_force(&flow(0.3, 0.1, 0.0)).unwrap());
    }

    /// A bare tube has no normal force, and so no CP, at `α = 0`; body lift gives it one at its
    /// middle at any other angle.
    #[test]
    fn a_bare_tube_has_no_cp_at_zero_incidence() {
        let tube = one_stage(
            vec![component("tube", body_part(1.0, 0.05, 0.05), None)],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&tube);
        assert_eq!(
            m.normal_force(&Flow::axial(0.5)).unwrap().cp_station_m,
            None
        );
        let f = m.normal_force(&flow(0.5, 0.1, 0.0)).unwrap();
        close(f.cp_station_m.unwrap(), 0.5, 1e-15, "tube lift CP");
    }

    proptest! {
        /// Barrowman's coefficients are dimensionless: scaling every length by `k` leaves the slope
        /// alone and scales the CP by `k`. A custom reference diameter scales the slope by
        /// `(d/d′)²` and leaves the CP alone.
        #[test]
        fn components_sum_to_the_total(
            count in 1u32..=8,
            mach in 0.0f64..0.99,
            alpha in 0.0f64..PI,
            roll in -4.0f64..4.0,
        ) {
            let m = model(&finned_rocket(count));
            let f = flow(mach, alpha, roll);
            let total = m.normal_force(&f).unwrap();
            let parts = m.components(&f).unwrap();
            let (c, moment) = parts.iter().fold((0.0, 0.0), |(c, x), p| {
                (c + p.normal_force.coefficient, x + p.normal_force.moment_m)
            });
            let tol = 1e-12 * (1.0 + total.coefficient.abs());
            prop_assert!((c - total.coefficient).abs() <= tol);
            prop_assert!((moment - total.moment_m).abs() <= 1e-12 * (1.0 + total.moment_m.abs()));
            let side: f64 = parts.iter().map(|p| p.normal_force.side_coefficient).sum();
            prop_assert!((side - total.side_coefficient).abs() <= 1e-12 * (1.0 + total.side_coefficient.abs()));
            let slope: f64 = parts.iter().map(|p| p.normal_force.slope_per_rad).sum();
            prop_assert!((slope - total.slope_per_rad).abs() <= 1e-12 * total.slope_per_rad.abs());
            // The allocation-free per-component path gives the same terms, and each component's
            // small-angle station is its centre of pressure as α → 0.
            prop_assert_eq!(m.component_count(), parts.len());
            let small = flow(mach, 1e-6, roll);
            for (index, part) in parts.iter().enumerate() {
                prop_assert_eq!(&m.component_normal_force(index, &f).unwrap(), &part.normal_force);
                let station = m.component_station_m(index, mach).unwrap();
                if let Some(cp) = m.component_normal_force(index, &small).unwrap().cp_station_m {
                    prop_assert!((station - cp).abs() <= 1e-6 * (1.0 + cp.abs()));
                }
            }
            prop_assert!(m.component_normal_force(parts.len(), &f).is_err());
            prop_assert!(m.component_station_m(parts.len(), mach).is_err());
        }

        #[test]
        fn scaling_leaves_slopes_and_scales_the_cp(
            k in 0.1f64..10.0,
            nose_fineness in 1.5f64..8.0,
            radius in 0.01f64..0.1,
            root in 0.02f64..0.3,
            tip_ratio in 0.0f64..1.0,
            span in 0.01f64..0.3,
            sweep in -0.1f64..0.3,
            count in 1u32..=8,
            alpha in 0.0f64..0.5,
            reference in 0.5f64..2.0,
        ) {
            let build = |s: f64, custom: Option<f64>| {
                let (r, l) = (s * radius, s * (root + 0.5));
                let mut tube = component("tube", body_part(l, r, r), None);
                let planform = FinPlanform::Trapezoidal {
                    root_chord_m: s * root,
                    tip_chord_m: s * root * tip_ratio,
                    span_m: s * span,
                    sweep_m: s * sweep,
                };
                tube.children = vec![component(
                    "fins",
                    fin_set(count, planform),
                    Some(Position::Bottom { aft_offset_m: 0.0 }),
                )];
                let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
                let nose = component("nose", nose(ogive, 2.0 * nose_fineness * r, r), None);
                let mut rocket = one_stage(vec![nose, tube], ReferenceDiameter::Maximum {});
                if let Some(d) = custom {
                    rocket.reference_diameter = ReferenceDiameter::Custom { diameter_m: d };
                }
                model(&rocket).normal_force(&flow(0.4, alpha, 0.3)).unwrap()
            };
            let rel = |a: f64, b: f64| (a / b - 1.0).abs();
            let base = build(1.0, None);
            let base_cp = base.cp_station_m.unwrap();
            let scaled = build(k, None);
            prop_assert!(rel(scaled.slope_per_rad, base.slope_per_rad) < 1e-9);
            prop_assert!(rel(scaled.cp_station_m.unwrap(), k * base_cp) < 1e-9);
            let d = 2.0 * radius * reference;
            let custom = build(1.0, Some(d));
            let factor = (2.0 * radius / d).powi(2);
            prop_assert!(rel(custom.slope_per_rad, factor * base.slope_per_rad) < 1e-12);
            prop_assert!(rel(custom.cp_station_m.unwrap(), base_cp) < 1e-12);
        }
    }

    /// A step in radius where two body components meet counts as a zero-length transition at the
    /// joint, so the body's total slope is Barrowman 1966 eq. 10 over the whole body: `2` for any
    /// pointed body however its radii step.
    #[test]
    fn radius_steps_count_at_the_joint() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.027), None),
                component("tube", body_part(0.5, 0.029, 0.029), None),
                component("tail", body_part(0.3, 0.025, 0.025), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_ref = PI * 0.029 * 0.029;
        let total = m.normal_force(&Flow::axial(0.3)).unwrap();
        close(
            total.slope_per_rad,
            2.0 * PI * 0.025 * 0.025 / a_ref,
            1e-14,
            "eq. 10",
        );
        let parts = m.components(&Flow::axial(0.3)).unwrap();
        let tube = parts[1].normal_force;
        close(
            tube.slope_per_rad,
            2.0 * PI * (0.029f64.powi(2) - 0.027f64.powi(2)) / a_ref,
            1e-14,
            "step up",
        );
        close(
            tube.cp_station_m.unwrap(),
            0.2,
            1e-14,
            "step up at the joint",
        );
        let tail = parts[2].normal_force;
        assert!(tail.slope_per_rad < 0.0);
        close(
            tail.cp_station_m.unwrap(),
            0.7,
            1e-14,
            "step down at the joint",
        );
        assert_eq!(m.bodies()[0].step_area_m2, 0.0);
    }

    /// A freeform fin set through the model equals the same trapezoid given as a trapezoid.
    #[test]
    fn freeform_fins_through_the_model() {
        let trapezoid = finned_rocket(3);
        let mut freeform = trapezoid.clone();
        if let Part::FinSet(set) = &mut freeform.stages[0].components[3].children[0].part {
            set.planform = FinPlanform::Freeform {
                points_m: vec![[0.0, 0.0], [0.07, 0.06], [0.12, 0.06], [0.12, 0.0]],
            };
        }
        let (a, b) = (model(&trapezoid), model(&freeform));
        let f = flow(0.7, 0.1, 0.0);
        let (fa, fb) = (a.normal_force(&f).unwrap(), b.normal_force(&f).unwrap());
        close(fb.coefficient, fa.coefficient, 1e-13, "C_N");
        close(
            fb.cp_station_m.unwrap(),
            fa.cp_station_m.unwrap(),
            1e-13,
            "CP",
        );
    }

    /// `Flow` and `NormalForce` round-trip through JSON, and a misspelt flow field is refused.
    #[test]
    fn flow_and_results_round_trip() {
        let f = flow(0.3, 0.1, -0.2);
        let back: Flow = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
        assert!(
            serde_json::from_str::<Flow>(
                r#"{"mach":0.3,"alpha_rad":0.1,"roll_rad":0,"aoa_deg":5}"#
            )
            .is_err()
        );
        let n = model(&finned_rocket(4)).normal_force(&f).unwrap();
        let back: NormalForce = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(back, n);
    }

    /// Layouts that don't hold together: fins without a body radius, a non-finite station.
    #[test]
    fn inconsistent_layouts_are_refused() {
        let layout = finned_rocket(4).layout().unwrap();
        let (fins, _) = layout.find("fins").unwrap();
        let mut no_radius = layout.clone();
        no_radius.components[fins].body_radius_m = None;
        let err = AeroModel::new(&no_radius).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, source } if id == "fins"
                && matches!(**source, AeroError::Layout(_))),
            "{err}"
        );
        assert_eq!(err, err.clone());
        let mut nan = layout;
        nan.components[0].fore_station_m = f64::NAN;
        assert!(matches!(
            AeroModel::new(&nan),
            Err(AeroError::InComponent { .. })
        ));
    }

    /// A two-fin set pushes along its fins' common normal: at 45° to the flow its side share
    /// equals its in-plane share, with the side moment at the fins' CP. Four fins have none.
    #[test]
    fn two_fin_sets_push_across_the_flow() {
        let two = model(&finned_rocket(2));
        let set = &two.fin_sets()[0];
        let alpha = 0.05;
        let f = two.normal_force(&flow(0.4, alpha, FRAC_PI_4)).unwrap();
        let one_fin = set
            .fin
            .geometry()
            .single_fin_slope(two.reference_area_m2(), 0.4)
            .unwrap()
            * set.interference;
        close(f.side_coefficient, one_fin * alpha, 1e-13, "side");
        close(
            f.side_moment_m,
            one_fin * alpha * set.cp_station_m(0.4).unwrap(),
            1e-13,
            "side moment",
        );
        let fins = &two.components(&flow(0.4, alpha, FRAC_PI_4)).unwrap()[4];
        close(
            fins.normal_force.coefficient,
            one_fin * alpha,
            1e-13,
            "in plane",
        );
        let four = model(&finned_rocket(4))
            .normal_force(&flow(0.4, alpha, 0.3))
            .unwrap();
        assert_eq!((four.side_coefficient, four.side_moment_m), (0.0, 0.0));
    }

    /// A body whose areas cancel to round-off has no CP rather than a CP at 1e14 m; its moment is
    /// still reported.
    #[test]
    fn a_cancelled_slope_has_no_cp() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.0254), None),
                component("tube", body_part(0.5, 0.0254, 0.0254), None),
                component("tail", body_part(0.3, 0.0254, 1e-9), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let f = m.normal_force(&Flow::axial(0.3)).unwrap();
        assert!(f.slope_per_rad.abs() < 1e-12, "{}", f.slope_per_rad);
        assert_eq!(f.cp_station_m, None);
        let moving = m.normal_force(&flow(0.3, 0.01, 0.0)).unwrap();
        assert!(moving.moment_m.is_finite() && moving.cp_station_m.is_some());
    }

    /// A normal-force table replaces the whole rocket's normal force and centre of pressure, on
    /// its own reference area; the components stay hpr's, for the flight's damping.
    #[test]
    fn a_normal_force_table_replaces_the_sum() {
        use crate::table::{NormalForceColumn, NormalForceTable};
        use hpr_core::interp::{Extrapolation, Interpolation, Table1D};

        let m = model(&finned_rocket(4));
        let flat = |value| {
            Table1D::new(
                vec![0.0, 2.0],
                vec![value, value],
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .unwrap()
        };
        let table = NormalForceTable::new(vec![NormalForceColumn::new(0.0, flat(10.0), flat(0.9))])
            .unwrap();
        let with = m.clone().with_normal_force_table(table.clone()).unwrap();
        let at = flow(0.5, 0.02, 0.3);
        let replaced = with.normal_force(&at).unwrap();
        close(replaced.coefficient, 10.0 * 0.02_f64.sin(), 1e-15, "C_N");
        close(
            replaced.moment_m,
            replaced.coefficient * 0.9,
            1e-15,
            "moment",
        );
        assert_eq!(replaced.cp_station_m, Some(0.9));
        assert_eq!(
            (replaced.side_coefficient, replaced.side_moment_m),
            (0.0, 0.0)
        );
        // One column at 0°, so any angle is past it, and the lookup says so.
        assert!(replaced.table.is_some_and(|lookup| lookup.beyond_alpha));
        assert_eq!(m.normal_force(&at).unwrap().table, None);
        assert_eq!(with.components(&at).unwrap(), m.components(&at).unwrap());
        assert_eq!(
            with.component_normal_force(3, &at).unwrap(),
            m.component_normal_force(3, &at).unwrap()
        );
        assert_ne!(m.normal_force(&at).unwrap(), replaced);
        // A table on a 108 mm reference, twice the rocket's 54 mm, gives four times the coefficient.
        let wider = m
            .clone()
            .with_normal_force_table(table.clone().with_reference_diameter_m(0.108).unwrap())
            .unwrap();
        let scaled = wider.normal_force(&at).unwrap();
        close(
            scaled.coefficient,
            4.0 * replaced.coefficient,
            1e-14,
            "rescaled",
        );
        assert_eq!(scaled.cp_station_m, Some(0.9));
        // RASAero II's reference, the largest body: 54 mm here, the rocket's own.
        let largest = m
            .clone()
            .with_normal_force_table(
                table
                    .clone()
                    .with_reference(TableReference::LargestBody)
                    .unwrap(),
            )
            .unwrap();
        close(
            largest.normal_force(&at).unwrap().coefficient,
            replaced.coefficient,
            1e-14,
            "largest body",
        );
        // On a rocket whose reference is half its largest body, RASAero II's reference, the
        // largest body, is four times the area.
        let mut half = finned_rocket(4);
        half.reference_diameter = ReferenceDiameter::Custom { diameter_m: 0.027 };
        let half = model(&half)
            .with_normal_force_table(
                table
                    .clone()
                    .with_reference(TableReference::LargestBody)
                    .unwrap(),
            )
            .unwrap();
        close(
            half.normal_force(&at).unwrap().coefficient,
            4.0 * replaced.coefficient,
            1e-14,
            "largest body on a half-size reference",
        );
        // Past Mach 5 a centre of pressure may leave the rocket; below, it may not.
        let hypersonic = |cp_at_6: f64| {
            let cps = Table1D::new(
                vec![0.0, 5.0, 6.0, 25.0],
                vec![0.9, 0.9, cp_at_6, -3.0],
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .unwrap();
            NormalForceTable::new(vec![NormalForceColumn::new(0.0, flat(10.0), cps)]).unwrap()
        };
        assert!(m.clone().with_normal_force_table(hypersonic(0.8)).is_ok());
        assert!(m.clone().with_normal_force_table(hypersonic(-0.1)).is_err());
        // A centre of pressure behind the tail or ahead of the nose is refused.
        for cp in [-0.01, 1.4] {
            let outside =
                NormalForceTable::new(vec![NormalForceColumn::new(0.0, flat(10.0), flat(cp))])
                    .unwrap();
            assert!(matches!(
                m.clone().with_normal_force_table(outside),
                Err(AeroError::Domain { .. })
            ));
        }
        // A table takes any Mach number; hpr's own normal force stops at Mach 5.
        assert!(with.normal_force(&flow(6.0, 0.02, 0.0)).is_ok());
        assert!(matches!(
            m.normal_force(&flow(6.0, 0.02, 0.0)),
            Err(AeroError::Mach { .. })
        ));
    }

    /// The finned rocket with its boattail and tail at the body's radius: a nose and three
    /// cylinders, which the shock-expansion method covers to the end.
    fn straight_rocket() -> hpr_design::Rocket {
        let mut rocket = crate::testing::finned_rocket(4);
        rocket.stages[0].components[2].part = body_part(0.05, 0.027, 0.027);
        rocket.stages[0].components[3].part = body_part(0.3, 0.027, 0.027);
        rocket
    }

    /// The straight rocket's body as the method takes it.
    fn straight_rocket_body() -> ShockExpansionBody {
        let nose =
            hpr_design::Profile::nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027).unwrap();
        let cylinder = |length_m| BodySegment::Cylinder {
            length_m,
            radius_m: 0.027,
        };
        ShockExpansionBody::new(
            &[
                BodySegment::Profile { profile: nose },
                cylinder(0.7),
                cylinder(0.05),
                cylinder(0.3),
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap()
    }

    /// Each body's slope and moment slope at `α = 0` (body lift vanishes there), and its station.
    fn body_values(model: &AeroModel, mach: f64) -> Vec<[f64; 3]> {
        (0..model.bodies().len())
            .map(|index| {
                let force = model
                    .component_normal_force(index, &flow(mach, 0.0, 0.0))
                    .unwrap();
                // At `α = 0` the moment is zero; the slope's moment is the CP times the slope.
                let moment = force
                    .cp_station_m
                    .map_or(0.0, |cp| cp * force.slope_per_rad);
                [
                    force.slope_per_rad,
                    moment,
                    model.component_station_m(index, mach).unwrap(),
                ]
            })
            .collect()
    }

    /// Asserts that no body's slope, moment or station jumps across `mach` at ±1e-9.
    fn no_jump(model: &AeroModel, mach: f64) {
        let below = body_values(model, mach - 1e-9);
        let above = body_values(model, mach + 1e-9);
        for (b, a) in below.iter().zip(&above) {
            for k in 0..3 {
                let scale = b[k].abs().max(a[k].abs()).max(1.0);
                assert!(
                    (a[k] - b[k]).abs() <= 1e-7 * scale,
                    "a jump at Mach {mach}: {b:?} to {a:?}"
                );
            }
        }
    }

    #[test]
    fn the_supersonic_join_has_no_jump() {
        let model = model(&straight_rocket());
        let join = model.supersonic_body().unwrap();
        assert_eq!(join.covered, 4);
        assert_eq!(join.join_start_mach, SUPERSONIC_JOIN_START_MACH);
        // The join's ends, rows of the table, points between rows, and the table's last row.
        let start = join.join_start_mach;
        for mach in [
            start,
            start + SUPERSONIC_JOIN_WIDTH_MACH,
            1.35,
            2.0,
            2.05,
            3.0,
            4.63,
            4.95,
            4.999,
        ] {
            no_jump(&model, mach);
        }
        // The join moves the body: the cylinder carries lift past it.
        let (low, high) = (body_values(&model, 1.0), body_values(&model, 2.0));
        assert_eq!(low[1][0], 0.0);
        assert!(high[1][0] > 0.1, "{:?}", high[1]);
    }

    /// M1.8e6's done-when: at an angle of attack, where body lift acts, the whole rocket's
    /// normal force and centre of pressure don't jump at ±1e-9 in Mach: across the supersonic
    /// join, the table's rows, and every Mach number where the crossflow Mach number `M sin α`
    /// meets a row of Jorgensen's tables, with either boattail rule.
    #[test]
    fn crossflow_and_the_boattail_fly_without_a_jump() {
        use crate::crossflow::{CROSSFLOW_DRAG_MACHS, ETA_MACHS};
        let layout = finned_rocket(4).layout().unwrap();
        for boattail in [
            SupersonicBoattail::WashingtonPettis,
            SupersonicBoattail::Footnote8,
        ] {
            let model = AeroModel::with_body_model(
                &layout,
                BodyModel::CURRENT.with_supersonic_boattail(boattail),
            )
            .unwrap();
            let start = model.supersonic_body().unwrap().join_start_mach;
            for alpha_deg in [10.0_f64, 30.0] {
                let s = alpha_deg.to_radians().sin();
                let mut machs = vec![start, start + SUPERSONIC_JOIN_WIDTH_MACH, 2.0, 2.05, 4.999];
                machs.extend(
                    CROSSFLOW_DRAG_MACHS
                        .iter()
                        .chain(&ETA_MACHS)
                        .map(|m| m / s)
                        .filter(|&m| m > 1e-3 && m < 4.999),
                );
                for mach in machs {
                    let at = |m: f64| {
                        let f = model
                            .normal_force(&flow(m, alpha_deg.to_radians(), 0.0))
                            .unwrap();
                        [f.coefficient, f.cp_station_m.unwrap()]
                    };
                    let (below, above) = (at(mach - 1e-9), at(mach + 1e-9));
                    for k in 0..2 {
                        let scale = below[k].abs().max(1.0);
                        assert!(
                            (above[k] - below[k]).abs() <= 1e-7 * scale,
                            "{boattail:?} at {alpha_deg}° and Mach {mach}: {below:?} to {above:?}"
                        );
                    }
                }
            }
        }
    }

    /// Vertical tips (power-series noses below `n` = 1, the von Kármán and L-V Haack, an
    /// elliptical nose) fly the method behind TN D-4865's Newtonian cap (M1.8e7), with a boattail
    /// and without, and nothing jumps at ±1e-9 in Mach: across the join, every row of the table, three
    /// points between each pair of rows,
    /// and Mach 2.1, near where the cap's handover reaches Fig. 2's 24°.
    #[test]
    fn vertical_tips_fly_the_method_without_a_jump() {
        for shape in [
            NoseShape::PowerSeries { exponent: 0.6369 },
            NoseShape::PowerSeries { exponent: 0.5 },
            NoseShape::VON_KARMAN,
            NoseShape::LV_HAACK,
            NoseShape::Elliptical {},
        ] {
            for boattail in [true, false] {
                let mut rocket = if boattail {
                    finned_rocket(4)
                } else {
                    straight_rocket()
                };
                rocket.stages[0].components[0].part = nose(shape, 0.25, 0.027);
                let model = model(&rocket);
                let table = model
                    .supersonic_body()
                    .unwrap_or_else(|| panic!("{shape:?}: no table"));
                assert_eq!(table.covered, 4, "{shape:?}");
                // On a row, the straight body's shares are the method's own.
                if !boattail {
                    let Part::NoseCone(cone) = &rocket.stages[0].components[0].part else {
                        unreachable!("`nose` builds a nose cone")
                    };
                    let cylinder = |length_m| BodySegment::Cylinder {
                        length_m,
                        radius_m: 0.027,
                    };
                    let body = ShockExpansionBody::new(
                        &[
                            BodySegment::Profile {
                                profile: cone.profile().unwrap(),
                            },
                            cylinder(0.7),
                            cylinder(0.05),
                            cylinder(0.3),
                        ],
                        DEFAULT_ELEMENTS_PER_CURVE,
                    )
                    .unwrap();
                    let method = body.segment_slopes(3.0, model.reference_area_m2()).unwrap();
                    let (nose_slope, _) = table.share(0, 3.0).unwrap();
                    assert!(
                        (nose_slope - method[0].slope_per_rad).abs() <= 1e-12,
                        "{shape:?}: {nose_slope} against {:?}",
                        method[0]
                    );
                }
                let start = table.join_start_mach;
                let mut machs = vec![start, start + SUPERSONIC_JOIN_WIDTH_MACH, 2.1, 4.999];
                // Every row of the table, and three points between each pair: a jump could hide
                // at a row, and between them the model is more than the interpolation (body lift
                // takes the flow's own Mach number).
                machs.extend(
                    (SUPERSONIC_FIRST_STEP..SUPERSONIC_LAST_STEP).flat_map(|step| {
                        let row = step as f64 / SUPERSONIC_STEPS_PER_MACH;
                        [row, row + 0.013, row + 0.027, row + 0.041]
                    }),
                );
                for alpha_deg in [1.0_f64, 10.0] {
                    for &mach in &machs {
                        let at = |m: f64| {
                            let f = model
                                .normal_force(&flow(m, alpha_deg.to_radians(), 0.0))
                                .unwrap();
                            [f.coefficient, f.cp_station_m.unwrap()]
                        };
                        let (below, above) = (at(mach - 1e-9), at(mach + 1e-9));
                        for k in 0..2 {
                            let scale = below[k].abs().max(1.0);
                            assert!(
                                (above[k] - below[k]).abs() <= 1e-7 * scale,
                                "{shape:?} at {alpha_deg}° and Mach {mach}: {below:?} to {above:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// A blunt tip's join starts where its cap first ends on the nose: at the Mach number whose
    /// handover angle is the nose's slope at its base, found without the method from
    /// [`crate::blunt_tip::handover_angle_rad`]. The method holds on one side of it only, with no
    /// flicker from corners too close to place (the handover packs the nose's elements into
    /// nanometres there; issue found when CI's Linux and Windows runs bisected a different start).
    #[test]
    fn a_blunt_tips_join_starts_where_its_cap_first_ends_on_the_nose() {
        let mut rocket = crate::testing::committed_design("wind-tunnel-arcas-robin-short.json");
        // The nose, the cylinder and the boattail; the lip left off.
        rocket.stages[0].components.truncate(3);
        let model = model(&rocket);
        let table = model.supersonic_body().unwrap();
        let Part::NoseCone(cone) = &rocket.stages[0].components[0].part else {
            unreachable!("the committed design starts with its nose")
        };
        let profile = cone.profile().unwrap();
        let base_angle = profile.radius_and_slope(profile.length_m()).1.atan();
        let (mut low, mut high) = (1.0 + 1e-9, 2.0);
        for _ in 0..200 {
            let mid = 0.5 * (low + high);
            if crate::blunt_tip::handover_angle_rad(mid).unwrap() < base_angle {
                low = mid;
            } else {
                high = mid;
            }
        }
        assert!(
            (table.join_start_mach - high).abs() <= 1e-12,
            "{} against {high}",
            table.join_start_mach
        );
        let run = model.supersonic_run.as_ref().unwrap();
        let body = ShockExpansionBody::new(&run.segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        let in_its_place: Vec<Option<ShockExpansionBody>> = run
            .boattails
            .iter()
            .map(|b| {
                b.as_ref().map(|b| {
                    ShockExpansionBody::new(&b.in_its_place, DEFAULT_ELEMENTS_PER_CURVE).unwrap()
                })
            })
            .collect();
        let holds = |m: f64| {
            run.shares(&body, &in_its_place, m, model.reference_area_m2())
                .is_some()
        };
        for i in 1..=2000 {
            let d = f64::from(i) * 1e-10;
            assert!(
                !holds(table.join_start_mach - d),
                "holds {d} below the start"
            );
            assert!(
                holds(table.join_start_mach + d),
                "fails {d} above the start"
            );
        }
    }

    /// A lip in a boattail's wake carries nothing faster than sound (M1.8e8): the committed Arcas
    /// Robin designs fly the method to their base, the lip's share is zero above the join and
    /// slender-body theory's below it, and nothing jumps at ±1e-9 in Mach. A lip that rises too
    /// far out of the wake still keeps the whole body on slender-body theory.
    #[test]
    fn a_lip_in_a_boattails_wake_carries_nothing() {
        for name in [
            "wind-tunnel-arcas-robin-short.json",
            "wind-tunnel-arcas-robin-long.json",
        ] {
            let rocket = crate::testing::committed_design(name);
            let model = model(&rocket);
            let table = model
                .supersonic_body()
                .unwrap_or_else(|| panic!("{name}: no table"));
            // The nose, the tube, the boattail and the lip.
            assert_eq!(table.covered, 4, "{name}");
            let lip = model.bodies().last().unwrap();
            assert!(lip.slope_per_rad > 0.1, "{name}: the lip is a flare");
            for mach in [1.3, 2.0, 3.0, 5.0] {
                let (slope, moment) = table.share(3, mach).unwrap();
                assert_eq!((slope, moment), (0.0, 0.0), "{name} at Mach {mach}");
            }
            // Below the join the lip keeps slender-body theory's share; above it, nothing.
            let start = table.join_start_mach;
            // At zero angle body lift vanishes, so this is the potential-flow share alone.
            let lip_slope = |mach: f64| {
                model
                    .components(&Flow::axial(mach))
                    .unwrap()
                    .into_iter()
                    .find(|c| c.id == "lip")
                    .unwrap()
                    .normal_force
                    .slope_per_rad
            };
            let below = lip_slope(1.0);
            assert!(
                (below - lip.slope_per_rad).abs() <= 0.01 * lip.slope_per_rad,
                "{name}: {below} against slender-body theory's {}",
                lip.slope_per_rad
            );
            let above = lip_slope(start + SUPERSONIC_JOIN_WIDTH_MACH);
            assert!(above.abs() <= 1e-12, "{name}: {above} above the join");
            for alpha_deg in [1.0_f64, 10.0] {
                let mut machs = vec![start, start + SUPERSONIC_JOIN_WIDTH_MACH, 4.999];
                machs.extend(
                    (SUPERSONIC_FIRST_STEP..SUPERSONIC_LAST_STEP).flat_map(|step| {
                        let row = step as f64 / SUPERSONIC_STEPS_PER_MACH;
                        [row, row + 0.017, row + 0.033]
                    }),
                );
                for mach in machs {
                    let at = |m: f64| {
                        let f = model
                            .normal_force(&flow(m, alpha_deg.to_radians(), 0.0))
                            .unwrap();
                        [f.coefficient, f.cp_station_m.unwrap()]
                    };
                    let (below, above) = (at(mach - 1e-9), at(mach + 1e-9));
                    for k in 0..2 {
                        let scale = below[k].abs().max(1.0);
                        assert!(
                            (above[k] - below[k]).abs() <= 1e-7 * scale,
                            "{name} at {alpha_deg}° and Mach {mach}: {below:?} to {above:?}"
                        );
                    }
                }
            }
        }
        // The shelter, weighed. The drag buildup's wake takes a lip rising a quarter of the
        // boattail's drop in diameter wholly (`crate::drag::WAKE_FULL_RISE`) and one rising half
        // of it not at all, grading between; the method reads the same number as its weight, so
        // the body moves between the two models continuously as the lip is drawn taller.
        let lipped = |rise: f64| {
            let mut rocket = crate::testing::finned_rocket(4);
            // The nose, the tube and the boattail, which drops from 0.027 m to 0.022 m in radius:
            // 0.010 m in diameter. The lip sits straight behind it, since a wake fades over any
            // tube between them.
            rocket.stages[0].components.truncate(3);
            rocket.stages[0].components.push(component(
                "lip",
                body_part(0.01, 0.022, 0.022 + 0.5 * rise * 0.010),
                None,
            ));
            model(&rocket)
        };
        assert_eq!(lipped(0.2).supersonic_body().map(|t| t.covered), Some(4));
        assert!(lipped(0.6).supersonic_body().is_none());
        let at = |rise: f64| {
            let model = lipped(rise);
            let f = model
                .normal_force(&flow(3.0, 4f64.to_radians(), 0.0))
                .unwrap();
            (f.coefficient, f.cp_station_m.unwrap())
        };
        // Across the old threshold, a quarter of the drop: before M1.8e10a the whole rocket's
        // normal force fell by a third here and its centre of pressure jumped 1.8 calibres
        // forward (issue #87). What is left is the weight ramping off its clamp, proportional to
        // the change in shape — a ten-thousandth of the force over a ten-thousandth of the drop.
        let (below, above) = (at(0.2499), at(0.2501));
        assert!(
            (above.0 - below.0).abs() <= 1e-3 * below.0.abs()
                && (above.1 - below.1).abs() <= 1e-3 * below.1.abs(),
            "{below:?} to {above:?} across the wake's full-shelter rise"
        );
        // How far apart the two models are at one shape, which is what a lip in the band is
        // uncertain by: take the same rocket out of the wake by making the lip a hair longer
        // than the boattail's drop, which changes no radius and no angle.
        let out_of_wake = {
            let mut rocket = crate::testing::finned_rocket(4);
            rocket.stages[0].components.truncate(3);
            rocket.stages[0].components.push(component(
                "lip",
                body_part(0.0101, 0.022, 0.022 + 0.5 * 0.25 * 0.010),
                None,
            ));
            let m = model(&rocket);
            assert!(
                m.supersonic_body().is_none(),
                "out of the wake by its length"
            );
            let f = m.normal_force(&flow(3.0, 4f64.to_radians(), 0.0)).unwrap();
            (f.coefficient, f.cp_station_m.unwrap() / 0.054)
        };
        let in_wake = at(0.25);
        let gap_force = out_of_wake.0 / in_wake.0 - 1.0;
        let gap_calibers = out_of_wake.1 - in_wake.1 / 0.054;
        assert!(
            (gap_force + 0.3295).abs() < 5e-4 && (gap_calibers + 1.774).abs() < 5e-3,
            "at one shape the two models differ by {gap_force} in force and {gap_calibers} \
             calibres in centre of pressure"
        );
        // What the band is worth, end to end: the jump is gone, but the same difference between
        // the two models is spread over it, and the guide quotes these numbers.
        let (full, nearly_none) = (at(0.25), at(0.4999));
        let calibers = (nearly_none.1 - full.1) / 0.054;
        assert!(
            (nearly_none.0 / full.0 - 1.0 + 0.291).abs() < 5e-4 && (calibers + 0.932).abs() < 5e-3,
            "across the band: {full:?} to {nearly_none:?}, {calibers} calibres"
        );
        // The weight is the wake's own share, and it carries the body to slender-body theory by
        // the far edge: at half the drop the method is gone, and just inside it is nearly gone.
        assert!((lipped(0.25).supersonic_body().unwrap().shape_weight - 1.0).abs() < 1e-12);
        // The ramp's shape, not just its ends: linear in the rise, as the wake's own fraction
        // is. A smoothstep through the same ends would read 0.896 at a rise of 0.3.
        for (rise, want) in [(0.3_f64, 0.8_f64), (0.375, 0.5), (0.45, 0.2)] {
            let weight = lipped(rise).supersonic_body().unwrap().shape_weight;
            assert!(
                (weight - want).abs() < 1e-9,
                "at a rise of {rise} the wake covers {weight}, not {want}"
            );
        }
        // Just inside the far edge the weight is all but gone; at the edge itself there is no
        // run, since a share of shelter under a millionth is not worth a table.
        assert!(
            lipped(0.49999).supersonic_body().unwrap().shape_weight < 1e-4,
            "a hair inside the wake's far edge the method has almost no weight left"
        );
        assert!(
            lipped(0.5).supersonic_body().is_none(),
            "at the wake's far edge there is no run"
        );
        // The rise is not the only way out of the wake: it fades with any tube between the
        // boattail and the lip, and the weight follows that too.
        let gapped = |gap_m: f64| {
            let mut rocket = crate::testing::finned_rocket(4);
            rocket.stages[0].components.truncate(3);
            rocket.stages[0].components.push(component(
                "gap",
                body_part(gap_m, 0.022, 0.022),
                None,
            ));
            rocket.stages[0].components.push(component(
                "lip",
                body_part(0.01, 0.022, 0.022 + 0.5 * 0.17 * 0.010),
                None,
            ));
            let m = model(&rocket);
            let weight = m.supersonic_body().map_or(0.0, |t| t.shape_weight);
            let f = m.normal_force(&flow(3.0, 4f64.to_radians(), 0.0)).unwrap();
            (weight, f.coefficient, f.cp_station_m.unwrap() / 0.054)
        };
        let (near, far) = (gapped(1e-6), gapped(0.010));
        assert!(
            near.0 > 0.999 && far.0 == 0.0,
            "a tube of the boattail's own drop in diameter carries the lip out of the wake: \
             {near:?} to {far:?}"
        );
        assert!(
            (far.2 - near.2 + 1.973).abs() < 0.01 && (far.1 / near.1 - 1.0 + 0.3381).abs() < 5e-4,
            "over that tube the force moves {} and the centre of pressure {} calibres",
            far.1 / near.1 - 1.0,
            far.2 - near.2
        );
        // Where the method is weighed in only partly, a component's station and its own centre of
        // pressure part company: the station blends stations, the force blends slopes and
        // moments, and the two agree only at the ends (issue #106). At half weight the tube's
        // station sits 0.114 m — about two calibres — behind its own centre of pressure.
        let half = lipped(0.375);
        let tube = half
            .component_normal_force(1, &flow(3.0, 0.0, 0.0))
            .unwrap();
        let station = half.component_station_m(1, 3.0).unwrap();
        assert!(
            (station - tube.cp_station_m.unwrap() - 0.1142).abs() < 5e-4,
            "the tube's station {station} against its centre of pressure {:?}",
            tube.cp_station_m
        );
        assert!(
            lipped(0.55).supersonic_body().is_none(),
            "past the wake there is no run at all"
        );
        // And no jump anywhere across the band, at either end or inside it.
        for rise in [0.2499_f64, 0.25, 0.3, 0.375, 0.45, 0.4999] {
            let (low, high) = (at(rise - 1e-9), at(rise + 1e-9));
            assert!(
                (high.0 - low.0).abs() <= 1e-7 * low.0.abs().max(1.0)
                    && (high.1 - low.1).abs() <= 1e-7 * low.1.abs().max(1.0),
                "at a rise of {rise}: {low:?} to {high:?}"
            );
        }
        // The shelter follows the geometry, not the drag buildup's tables: a lip out of the wake
        // is refused whatever its shape, including one whose drag curve the buildup has none for
        // (a Haack series past C = 1/3; the physics review found this).
        for shape in [NoseShape::Conical {}, NoseShape::Haack { parameter: 0.5 }] {
            let mut rocket = crate::testing::committed_design("wind-tunnel-arcas-robin-short.json");
            let components = &mut rocket.stages[0].components;
            let last = components.len() - 1;
            let Part::Transition(lip) = &mut components[last].part else {
                unreachable!("the committed design ends in its lip")
            };
            // Raised to 0.60 of the boattail's drop, past the wake's far edge.
            lip.aft_radius_m = lip.fore_radius_m + 0.60 * (0.028575 - 0.0166116);
            lip.shape = shape;
            assert!(
                model(&rocket).supersonic_body().is_none(),
                "{shape:?} out of the wake"
            );
        }
        // A flare longer than the boattail's drop in diameter grows out of the wake, however
        // little it rises.
        let mut rocket = finned_rocket(4);
        rocket.stages[0].components.truncate(3);
        rocket.stages[0].components.push(component(
            "long flare",
            body_part(1.0, 0.022, 0.0229),
            None,
        ));
        assert!(model(&rocket).supersonic_body().is_none());
        // A narrowing part behind the run is a boattail the method hasn't covered, not a lip,
        // even where the wake takes its drag: it keeps slender-body theory's share.
        let mut rocket = finned_rocket(4);
        rocket.stages[0].components.truncate(3);
        rocket.stages[0].components.push(component(
            "second boattail",
            body_part(0.03, 0.0231, 0.021),
            None,
        ));
        let narrowing = model(&rocket);
        assert!(narrowing.bodies().last().unwrap().slope_per_rad < 0.0);
        assert!(narrowing.supersonic_body().is_none());
    }

    /// Washington and Pettis's correlation is read no steeper than the angle where the flow
    /// separates, 16° (Cubbage, [issue #90](https://github.com/nrdptel/hpr-sim/issues/90)): a
    /// steeper boattail takes the increment of one of the same radii drawn out to 16°. Shallower
    /// boattails are untouched, the increment is continuous in the angle, and it never runs away
    /// or falls to zero, which would move the centre of pressure aft of where anything measured.
    #[test]
    fn a_separating_boattail_reads_the_correlation_at_its_steepest_measured_angle() {
        // Mach 1.5, where the held read and the true reads at 17° and 23° sit on Fig. 5's curve
        // (30° and 40° run past its last entry and clamp, but against a held read that does not).
        // At Mach 3 every read clamps to the same number and the assertions below would all be
        // identities: the test then passes with the hold deleted, inverted or moved.
        let mach = 1.5;
        let ogive = nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027);
        let Part::NoseCone(ogive) = ogive else {
            unreachable!("`nose` builds a nose cone")
        };
        // The test rocket's boattail redrawn at each angle from the same fore radius, and what the
        // method gives a cylinder of the same length in its place.
        let kept_pair = |at: f64, half_angle_deg: f64| {
            let length_m = (0.027 - 0.022) / half_angle_deg.to_radians().tan();
            let mut rocket = finned_rocket(4);
            rocket.stages[0].components[2].part = body_part(length_m, 0.027, 0.022);
            let model = model(&rocket);
            let a_ref = model.reference_area_m2();
            let table = model
                .supersonic_body()
                .unwrap_or_else(|| panic!("a table at {half_angle_deg}°"));
            let share = table.share(2, at).expect("the boattail's share").0;
            let in_its_place = ShockExpansionBody::new(
                &[
                    BodySegment::Profile {
                        profile: ogive.profile().unwrap(),
                    },
                    BodySegment::Cylinder {
                        length_m: 0.7,
                        radius_m: 0.027,
                    },
                    BodySegment::Cylinder {
                        length_m,
                        radius_m: 0.027,
                    },
                ],
                DEFAULT_ELEMENTS_PER_CURVE,
            )
            .unwrap();
            let cylinder = in_its_place.segment_slopes(at, a_ref).unwrap()[2].slope_per_rad;
            let raw = |l: f64| {
                crate::supersonic_boattail::wp_slope(at, 0.027, 0.022, l).unwrap()
                    * PI
                    * 0.027
                    * 0.027
                    / a_ref
            };
            // The increment the rocket flies, over the correlation read at the true angle.
            (share - cylinder, raw(length_m))
        };
        let kept = |half_angle_deg: f64| kept_pair(mach, half_angle_deg);
        let kept_at = |at: f64, half_angle_deg: f64| kept_pair(at, half_angle_deg).0;
        let at_16 = (0.027 - 0.022) / 16.0_f64.to_radians().tan();
        let close = |got: f64, want: f64, what: &str| {
            assert!(
                (got - want).abs() < 0.02 * want.abs(),
                "{what}: {got} against {want}"
            );
        };
        // Shallower than the onset: the correlation as measured, untouched.
        for angle in [8.0, 15.0, 16.0] {
            let (flown, raw) = kept(angle);
            close(flown, raw, "read at the true angle");
        }
        // Steeper: held at the 16° geometry, the same for every angle past it, never zero, and
        // strictly more lift taken off than reading the true angle would give. Past about 45° the
        // method no longer covers the body at all and the rocket keeps slender-body theory (a
        // switch of its own, issue #87), so the cap is read below that.
        // Scaled onto the rocket's reference area, as the flown increment is.
        let a_ref = model(&finned_rocket(4)).reference_area_m2();
        let at_16_read = crate::supersonic_boattail::wp_slope(mach, 0.027, 0.022, at_16).unwrap()
            * PI
            * 0.027
            * 0.027
            / a_ref;
        for angle in [17.0, 23.0, 30.0, 40.0] {
            let (flown, raw) = kept(angle);
            let (held, _) = kept(16.0);
            close(flown, held, "held at 16°");
            // The cap's own angle, read straight from the correlation at the 16° length.
            close(flown, at_16_read, "the correlation at the 16° geometry");
            assert!(flown < 0.0, "{angle}°: the boattail still takes lift off");
            // The correlation read at the true angle takes off strictly less, which is the
            // optimistic side: holding it keeps the centre of pressure forward of that.
            assert!(flown < raw, "{angle}°: {flown} against {raw} read raw");
        }
        // What the choice is worth, and which way it runs: the body's centre of pressure with the
        // increment held at 16°, against the same body with the boattail's increment faded to
        // nothing (the other honest limit for separated flow, a cylinder's share alone). Letting
        // it fade moves the centre of pressure aft, so the rocket reads more stable.
        let length_m = (0.027 - 0.022) / 30.0_f64.to_radians().tan();
        let mut steep = finned_rocket(4);
        steep.stages[0].components[2].part = body_part(length_m, 0.027, 0.022);
        steep.stages[0].components.truncate(3);
        let steep = model(&steep);
        let body_cp_calibers = |at: f64, increment_kept: bool| {
            let parts = steep.components(&Flow::axial(at)).unwrap();
            let (mut slope, mut moment) = (0.0, 0.0);
            for (index, part) in parts.iter().take(steep.bodies().len()).enumerate() {
                let mut share = part.normal_force.slope_per_rad;
                let station = part.normal_force.cp_station_m.unwrap_or(0.0);
                if index == 2 && !increment_kept {
                    share -= kept_at(at, 30.0);
                }
                slope += share;
                moment += share * station;
            }
            moment / slope / 0.054
        };
        // The gap grows as the speed falls, so it is quoted as a range, not one number.
        let gaps: Vec<(f64, f64)> = [1.5_f64, 2.0, 3.0, 4.63]
            .iter()
            .map(|at| {
                (
                    *at,
                    body_cp_calibers(*at, false) - body_cp_calibers(*at, true),
                )
            })
            .collect();
        for (at, gap) in &gaps {
            assert!(
                *gap > 0.6,
                "Mach {at}: the fading rule sits {gap} calibres aft"
            );
        }
        // Every value the guide's table quotes, pinned.
        for (at, want) in [(1.5, 1.35), (2.0, 0.91), (3.0, 0.75), (4.63, 0.67)] {
            let got = gaps
                .iter()
                .find(|(m, _)| (m - at).abs() < 1e-9)
                .expect("a measured gap")
                .1;
            assert!(
                (got - want).abs() < 0.02,
                "Mach {at}: the gap is {got} calibres, the guide says {want}"
            );
        }
        // Continuous in the angle, at the cap and either side of it. Below the cap the read
        // moves with the angle, so this is not zero: over ±1e-6 of a degree it is a few times
        // 1e-8, where a switch at the cap would show as the 7e-3 that separates the held and raw
        // reads just past 16°.
        for angle in [15.9_f64, 16.0, 16.1] {
            let (below, above) = (kept(angle - 1e-6).0, kept(angle + 1e-6).0);
            assert!((above - below).abs() < 1e-6, "{angle}°: {below} to {above}");
        }
        assert!((at_16 - (0.027 - 0.022) / 16.0_f64.to_radians().tan()).abs() < 1e-15);
    }

    /// Holding the correlation at 16° reads it at a longer boattail, which walks left along
    /// Fig. 5 toward the peak near Mach 1 its points come from. That branch passes Munk's
    /// slender-body line, so the extra the holding takes off stops at potential flow. The read at
    /// the boattail's true angle is never clipped: that is the measurement, wherever it sits.
    #[test]
    fn holding_the_correlation_stops_at_potential_flow() {
        // A 30° boattail to a twentieth of the radius at Mach 1.42: held −2.077, true −0.981
        // and potential flow −1.995 per radian on the boattail's own area, so the bound bites.
        // The window is narrow — this shape's table starts at Mach 1.3906 and the held read
        // stops passing potential flow at 1.4509 — so the Mach is checked against the join.
        let (fore_radius_m, aft_radius_m, mach) = (0.027_f64, 0.05 * 0.027_f64, 1.42_f64);
        let slender = 2.0 * ((aft_radius_m / fore_radius_m).powi(2) - 1.0);
        let length_m = (fore_radius_m - aft_radius_m) / 30.0_f64.to_radians().tan();
        let mut rocket = finned_rocket(4);
        rocket.stages[0].components[2].part = body_part(length_m, fore_radius_m, aft_radius_m);
        rocket.stages[0].components[3].part = body_part(0.3, aft_radius_m, aft_radius_m);
        let steep = model(&rocket);
        let a_ref = steep.reference_area_m2();
        let per_boattail_area = PI * fore_radius_m * fore_radius_m / a_ref;
        let table = steep.supersonic_body().expect("the method covers it");
        // Inside the table: below its start `share` clamps to the lead row, and the numbers
        // above would belong to a Mach number the assertions never touch.
        assert!(
            mach > table.join_start_mach,
            "Mach {mach} is below the table's start, {}",
            table.join_start_mach
        );
        let cylinder = ShockExpansionBody::new(
            &[
                BodySegment::Profile {
                    profile: match nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027) {
                        Part::NoseCone(ogive) => ogive.profile().unwrap(),
                        _ => unreachable!("`nose` builds a nose cone"),
                    },
                },
                BodySegment::Cylinder {
                    length_m: 0.7,
                    radius_m: fore_radius_m,
                },
                BodySegment::Cylinder {
                    length_m,
                    radius_m: fore_radius_m,
                },
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap()
        .segment_slopes(mach, a_ref)
        .unwrap()[2]
            .slope_per_rad;
        let flown = table.share(2, mach).expect("the boattail's share").0 - cylinder;
        // It flies potential flow's value exactly, not the held read that would pass it.
        assert!(
            (flown / per_boattail_area - slender).abs() < 2e-3,
            "{} against potential flow's {slender}",
            flown / per_boattail_area
        );
        let held = crate::supersonic_boattail::wp_slope(
            mach,
            fore_radius_m,
            aft_radius_m,
            (fore_radius_m - aft_radius_m) / SEPARATION_ONSET_RAD.tan(),
        )
        .unwrap();
        assert!(
            held < slender - 0.05,
            "the held read {held} must pass the bound's {slender} to pin it"
        );
        // And a boattail inside the measured angles keeps its own read, even where that read is
        // itself past potential flow: the bound belongs to the holding, not to the measurement.
        let gentle_m = (fore_radius_m - 0.6 * fore_radius_m) / 4.0_f64.to_radians().tan();
        let gentle =
            crate::supersonic_boattail::wp_slope(1.5, fore_radius_m, 0.6 * fore_radius_m, gentle_m)
                .unwrap();
        let gentle_slender = 2.0 * (0.6_f64.powi(2) - 1.0);
        assert!(
            gentle < gentle_slender,
            "the 4° read {gentle} should pass {gentle_slender}"
        );
        let mut gentle_rocket = finned_rocket(4);
        gentle_rocket.stages[0].components[2].part =
            body_part(gentle_m, fore_radius_m, 0.6 * fore_radius_m);
        gentle_rocket.stages[0].components[3].part =
            body_part(0.3, 0.6 * fore_radius_m, 0.6 * fore_radius_m);
        let gentle_model = model(&gentle_rocket);
        let gentle_table = gentle_model.supersonic_body().expect("a table");
        let gentle_cylinder = ShockExpansionBody::new(
            &[
                BodySegment::Profile {
                    profile: match nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027) {
                        Part::NoseCone(ogive) => ogive.profile().unwrap(),
                        _ => unreachable!("`nose` builds a nose cone"),
                    },
                },
                BodySegment::Cylinder {
                    length_m: 0.7,
                    radius_m: fore_radius_m,
                },
                BodySegment::Cylinder {
                    length_m: gentle_m,
                    radius_m: fore_radius_m,
                },
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap()
        .segment_slopes(1.5, gentle_model.reference_area_m2())
        .unwrap()[2]
            .slope_per_rad;
        let gentle_flown = gentle_table.share(2, 1.5).expect("the share").0 - gentle_cylinder;
        let gentle_area = PI * fore_radius_m * fore_radius_m / gentle_model.reference_area_m2();
        assert!(
            (gentle_flown / gentle_area - gentle).abs() < 2e-3,
            "the 4° boattail flies {} and its correlation reads {gentle}",
            gentle_flown / gentle_area
        );
    }

    /// How far the potential-flow bound reaches, read back out of the table rather than
    /// recomputed. Over the boattails swept below — 16° to 53.6°, narrowing to between a
    /// thousandth and three tenths of the fore radius — this pins three things: at the table's
    /// rows a boattail never takes off more than potential flow, **except** where its own read
    /// already passes it, since the bound never clips that; there, the table carries the
    /// correlation as published; and the most the bound moves a **printed** coefficient, after
    /// the join's weight. Deleting the bound fails this, and so does clipping the floor.
    #[test]
    fn what_the_potential_flow_bound_reaches() {
        let fore_radius_m = 0.027_f64;
        let ogive = nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027);
        let Part::NoseCone(ogive) = ogive else {
            unreachable!("`nose` builds a nose cone")
        };
        let mut worst = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        let mut steepest_tabled = 0.0_f64;
        let mut floor_angles: Vec<f64> = Vec::new();
        for angle_deg in [16.0_f64, 16.5, 17.0, 17.25, 17.5, 30.0, 53.0, 53.5, 53.6] {
            for ratio in [0.001_f64, 0.02, 0.25, 0.3] {
                let aft_radius_m = ratio * fore_radius_m;
                let drop_m = fore_radius_m - aft_radius_m;
                let length_m = drop_m / angle_deg.to_radians().tan();
                let held_length_m = drop_m / SEPARATION_ONSET_RAD.tan();
                let ceiling = 2.0 * (ratio * ratio - 1.0);
                let read = |mach: f64, length: f64| {
                    crate::supersonic_boattail::wp_slope(mach, fore_radius_m, aft_radius_m, length)
                        .unwrap()
                };
                // Skip shapes the bound cannot touch at any Mach the table covers: building a
                // supersonic table is the expensive part of this sweep.
                if read(1.2, held_length_m) >= ceiling {
                    continue;
                }
                let mut rocket = finned_rocket(4);
                rocket.stages[0].components[2].part =
                    body_part(length_m, fore_radius_m, aft_radius_m);
                rocket.stages[0].components[3].part = body_part(0.3, aft_radius_m, aft_radius_m);
                let flown = model(&rocket);
                let Some(table) = flown.supersonic_body() else {
                    continue;
                };
                steepest_tabled = steepest_tabled.max(angle_deg);
                let a_ref = flown.reference_area_m2();
                let per_area = PI * fore_radius_m * fore_radius_m / a_ref;
                // The method's share for a cylinder of the boattail's length in its place, which
                // the flown share is the increment on top of.
                let cylinder_body = ShockExpansionBody::new(
                    &[
                        BodySegment::Profile {
                            profile: ogive.profile().unwrap(),
                        },
                        BodySegment::Cylinder {
                            length_m: 0.7,
                            radius_m: fore_radius_m,
                        },
                        BodySegment::Cylinder {
                            length_m,
                            radius_m: fore_radius_m,
                        },
                    ],
                    DEFAULT_ELEMENTS_PER_CURVE,
                )
                .unwrap();
                // The shares are computed at the table's rows: its lead row at the join, then
                // every 0.05 Mach. Between rows the table interpolates, so a printed value can
                // sit a little past the ceiling beside a row on the floor branch below; the
                // guarantee belongs to the rows.
                let mut rows = vec![table.join_start_mach];
                let mut step = (table.join_start_mach * SUPERSONIC_STEPS_PER_MACH).ceil();
                while step / SUPERSONIC_STEPS_PER_MACH <= 1.55 {
                    rows.push(step / SUPERSONIC_STEPS_PER_MACH);
                    step += 1.0;
                }
                for mach in rows {
                    let Some((share, _)) = table.share(2, mach) else {
                        break;
                    };
                    let cylinder =
                        cylinder_body.segment_slopes(mach, a_ref).unwrap()[2].slope_per_rad;
                    let increment = (share - cylinder) / per_area;
                    if read(mach, length_m) >= ceiling {
                        // The usual case: the boattail's own read is inside potential flow, so
                        // the bound is what stops the hold. Read out of the table, the share a
                        // rocket flies never passes potential flow.
                        assert!(
                            increment >= ceiling - 1e-12,
                            "{angle_deg}° to {ratio} of the radius at Mach {mach}: the boattail \
                             takes {increment} off, past potential flow's {ceiling}"
                        );
                    } else {
                        // The floor: the boattail's own read already passes potential flow, and
                        // that read is never clipped, so the hold does nothing here.
                        assert!(
                            increment <= ceiling,
                            "{angle_deg}° to {ratio} at Mach {mach}: {increment} against {ceiling}"
                        );
                        // The boattail's own read, as published: the bound never clips it, so
                        // the table carries the correlation itself here.
                        let published = read(mach, length_m);
                        assert!(
                            (increment - published).abs() < 1e-9,
                            "{angle_deg}° to {ratio} at Mach {mach}: {increment} against the \
                             correlation's own {published}"
                        );
                        if !floor_angles.contains(&angle_deg) {
                            floor_angles.push(angle_deg);
                        }
                    }
                    // And how much of the holding that costs, against the unbounded read.
                    let moved = (increment - read(mach, held_length_m)).abs()
                        * per_area
                        * table.weight(mach);
                    if moved > worst.0 {
                        worst = (moved, angle_deg, ratio, mach);
                    }
                }
            }
        }
        // The floor is a sliver just above the hold's own angle, on the angles swept: the
        // condition is that the boattail's own read passes the curve's Munk crossing, so it
        // closes as the angle or the Mach number rises.
        assert_eq!(
            floor_angles,
            [16.0, 16.5, 17.0, 17.25],
            "the swept angles where a boattail's own read already passes potential flow"
        );
        // The steepest shape this sweep both tables and can bind: the method refuses steeper
        // bodies, at an angle that depends on how far the boattail narrows.
        assert!(
            (steepest_tabled - 53.5).abs() < 1e-12,
            "the steepest boattail swept that the method tables is {steepest_tabled}°"
        );
        assert!(
            (worst.0 - 0.060).abs() < 5e-4,
            "the bound moves a printed coefficient by at most {:.4} per rad, at {}° to {} of the \
             radius at Mach {:.3}",
            worst.0,
            worst.1,
            worst.2,
            worst.3
        );
    }

    /// Footnote 8's size, by hand, on a boattail **and the tube behind it**
    /// ([issue #90](https://github.com/nrdptel/hpr-sim/issues/90)). On each straight element the
    /// method's loading is `Λ(x) = (1 − e^(−η)) Λ_c + e^(−η) Λ₂`, `x` axial from its corner, and
    /// `C_Nα = (2π/A_ref) ∫ Λ r dx` over it (TN 3527 eqs. 8, 9, 19). Footnote 8 gives a boattail
    /// element the free stream's pressure and a tangent cone of 2 per radian (p. 12), checked
    /// here against hard-coded values; the tube behind relaxes from the boattail's loading toward
    /// `Λ_c = 0`, so its share is set by the decay rate alone.
    ///
    /// What this pins: the integration, the footnote's two tangent-cone terms, and that the two
    /// segments' shares follow from the reported flow. What it does not pin: the decay rate `η`
    /// itself, which comes from eq. 9 and is read from the method here.
    #[test]
    fn footnote_eights_boattail_share_by_hand() {
        let a_ref = PI * 0.027 * 0.027;
        let ogive = nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027);
        let Part::NoseCone(ogive) = ogive else {
            unreachable!("`nose` builds a nose cone")
        };
        let (length_m, fore_radius_m, aft_radius_m) = (0.05, 0.027, 0.022);
        const TUBE_LENGTH_M: f64 = 0.2;
        let body = ShockExpansionBody::new(
            &[
                BodySegment::Profile {
                    profile: ogive.profile().unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: 0.7,
                    radius_m: 0.027,
                },
                BodySegment::Profile {
                    profile: hpr_design::Profile::transition(
                        NoseShape::Conical {},
                        length_m,
                        fore_radius_m,
                        aft_radius_m,
                        false,
                    )
                    .unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: TUBE_LENGTH_M,
                    radius_m: aft_radius_m,
                },
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap();
        let mach = 2.0;
        let shares = body.segment_slopes(mach, a_ref).unwrap();
        // The boattail is one straight element: the corner at its fore end, then a cone of
        // half-angle −δ to its aft end.
        let flows = body.element_flows(mach).unwrap();
        let boattail = flows
            .iter()
            .rev()
            .nth(1)
            .copied()
            .expect("the boattail's element");
        let (load, decay) = (boattail.loading_per_rad, boattail.decay_per_m);
        // Eq. 19's `r` at the corner is the boattail's fore radius.
        assert!((boattail.corner_radius_m - fore_radius_m).abs() < 1e-12);
        // Footnote 8 gives a boattail element the free stream's pressure and a tangent cone of 2
        // per radian, so its loading relaxes toward `tan δ · 2`.
        let delta = ((aft_radius_m - fore_radius_m) / length_m).atan();
        let cone_load = delta.tan() * 2.0;
        assert!((boattail.tangent_cone_loading_per_rad - cone_load).abs() < 1e-12);
        assert!((boattail.tangent_cone_pressure_ratio - 1.0).abs() < 1e-12);
        // C_Nα = (2π/A_ref) ∫ Λ(x) r(x) dx over a segment, by Simpson's rule on 4001 points.
        let integrate = |span_m: f64, fore_r: f64, aft_r: f64, load: f64, cone: f64, decay: f64| {
            let steps = 4000;
            let mut sum = 0.0;
            for i in 0..=steps {
                let t = f64::from(i) / f64::from(steps);
                let x = t * span_m;
                let r = fore_r + (aft_r - fore_r) * t;
                let e = (-decay * x).exp();
                let lambda = (1.0 - e) * cone + e * load;
                let weight = if i == 0 || i == steps {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                sum += weight * lambda * r;
            }
            2.0 * PI * (sum * span_m / (3.0 * f64::from(steps))) / a_ref
        };
        let by_hand = integrate(
            length_m,
            fore_radius_m,
            aft_radius_m,
            load,
            cone_load,
            decay,
        );
        assert!(
            (shares[2].slope_per_rad - by_hand).abs() < 1e-6,
            "{} against {by_hand}",
            shares[2].slope_per_rad
        );
        // The tube behind it, which issue #90 asks for too: a cylinder's tangent cone carries no
        // loading, so the decay alone takes its share from the boattail's exit loading to zero.
        let tube = flows.last().expect("the tube's element");
        assert!(tube.angle_rad.abs() < 1e-12 && tube.tangent_cone_loading_per_rad.abs() < 1e-12);
        let tube_by_hand = integrate(
            TUBE_LENGTH_M,
            aft_radius_m,
            aft_radius_m,
            tube.loading_per_rad,
            0.0,
            tube.decay_per_m,
        );
        assert!(
            (shares[3].slope_per_rad - tube_by_hand).abs() < 1e-6,
            "the tube: {} against {tube_by_hand}",
            shares[3].slope_per_rad
        );
        // The tube carries the larger part of the pair, so the decay sets most of the answer.
        assert!(
            tube_by_hand < by_hand && tube_by_hand < 0.0,
            "the tube's {tube_by_hand} against the boattail's {by_hand}"
        );
        // And its size: footnote 8 takes far less lift off than slender-body theory's
        // 2 (A_aft − A_fore)/A_ref.
        let slender = 2.0 * (aft_radius_m.powi(2) - fore_radius_m.powi(2)) / (0.027 * 0.027);
        assert!(
            by_hand < 0.0 && by_hand / slender < 0.2,
            "{by_hand} against slender-body theory's {slender}"
        );
    }

    /// Washington and Pettis's boattail: the method's share for a cylinder of the boattail's
    /// length and fore radius in its place, plus the measured increment at its centre of
    /// pressure; footnote 8's is the method's own segment share.
    #[test]
    fn the_boattail_takes_washington_and_pettis_increment() {
        let layout = finned_rocket(4).layout().unwrap();
        let current = AeroModel::new(&layout).unwrap();
        let before = AeroModel::with_body_model(&layout, BodyModel::BEFORE_M1_8E6).unwrap();
        let a_ref = current.reference_area_m2();
        let ogive = nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027);
        let Part::NoseCone(ogive) = ogive else {
            unreachable!("`nose` builds a nose cone")
        };
        let segments = |last: BodySegment| {
            [
                BodySegment::Profile {
                    profile: ogive.profile().unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: 0.7,
                    radius_m: 0.027,
                },
                last,
            ]
        };
        let in_its_place = ShockExpansionBody::new(
            &segments(BodySegment::Cylinder {
                length_m: 0.05,
                radius_m: 0.027,
            }),
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap();
        let rocket = finned_rocket(4);
        let Part::Transition(tail) = &rocket.stages[0].components[2].part else {
            unreachable!("the test rocket's third part is its boattail")
        };
        let with_boattail = ShockExpansionBody::new(
            &segments(BodySegment::Profile {
                profile: tail.profile().unwrap(),
            }),
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap();
        for mach in [2.0, 3.0, 4.5] {
            let cylinder = in_its_place.segment_slopes(mach, a_ref).unwrap()[2];
            let increment = crate::supersonic_boattail::wp_slope(mach, 0.027, 0.022, 0.05).unwrap()
                * PI
                * 0.027
                * 0.027
                / a_ref;
            let centre = 0.25 + 0.7 + wp_centre_fraction(mach) * 0.05;
            let (slope, moment) = current.supersonic_body().unwrap().share(2, mach).unwrap();
            close(
                slope,
                cylinder.slope_per_rad + increment,
                1e-12,
                "W&P slope",
            );
            close(
                moment,
                cylinder.moment_slope_m + increment * centre,
                1e-12,
                "W&P moment",
            );
            // The increment is most of it: the nose's lift has decayed along the cylinder.
            assert!(increment < 0.0 && cylinder.slope_per_rad.abs() < 0.1 * increment.abs());
            let footnote_8 = with_boattail.segment_slopes(mach, a_ref).unwrap()[2];
            let (old, _) = before.supersonic_body().unwrap().share(2, mach).unwrap();
            close(old, footnote_8.slope_per_rad, 1e-12, "footnote 8");
            // The measured increment takes off more lift than footnote 8, less than
            // slender-body theory's -2[1 - (0.022/0.027)^2].
            let slender = -2.0 * (1.0 - (0.022_f64 / 0.027).powi(2));
            assert!(slender < slope && slope < old, "Mach {mach}: {slope} {old}");
        }
    }

    /// Two boattails in one run, the second last: each takes the method's share for a cylinder
    /// of its length and fore radius in its place plus its own Washington and Pettis increment,
    /// and the tube between them keeps the method's share, marched through the first boattail by
    /// footnote 8. The nose and first tube are the method's either way.
    #[test]
    fn two_boattails_each_take_their_own_increment() {
        let (l_n, l_1, l_b1, l_2, l_b2) = (0.25, 0.4, 0.05, 0.3, 0.04);
        let (r_0, r_1, r_2) = (0.027, 0.024, 0.02);
        let rocket = one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, l_n, r_0),
                    None,
                ),
                component("tube", body_part(l_1, r_0, r_0), None),
                component("boattail", body_part(l_b1, r_0, r_1), None),
                component("waist", body_part(l_2, r_1, r_1), None),
                component("tail", body_part(l_b2, r_1, r_2), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let model = AeroModel::new(&rocket.layout().unwrap()).unwrap();
        let before =
            AeroModel::with_body_model(&rocket.layout().unwrap(), BodyModel::BEFORE_M1_8E6)
                .unwrap();
        let table = model.supersonic_body().unwrap();
        assert_eq!(table.covered, 5);
        let a_ref = model.reference_area_m2();
        let profile = |part: &Part| match part {
            Part::NoseCone(n) => n.profile().unwrap(),
            Part::Transition(t) => t.profile().unwrap(),
            _ => unreachable!("the test's profiled parts are a nose and transitions"),
        };
        let parts: Vec<Part> = rocket.stages[0]
            .components
            .iter()
            .map(|c| c.part.clone())
            .collect();
        let cylinder = |length_m, radius_m| BodySegment::Cylinder { length_m, radius_m };
        let profiled = |i: usize| BodySegment::Profile {
            profile: profile(&parts[i]),
        };
        let body = |segments: &[BodySegment]| {
            ShockExpansionBody::new(segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap()
        };
        let whole = body(&[
            profiled(0),
            cylinder(l_1, r_0),
            profiled(2),
            cylinder(l_2, r_1),
            profiled(4),
        ]);
        let first_in_place = body(&[profiled(0), cylinder(l_1, r_0), cylinder(l_b1, r_0)]);
        let second_in_place = body(&[
            profiled(0),
            cylinder(l_1, r_0),
            profiled(2),
            cylinder(l_2, r_1),
            cylinder(l_b2, r_1),
        ]);
        let increment = |mach: f64, fore: f64, aft: f64, length: f64| {
            crate::supersonic_boattail::wp_slope(mach, fore, aft, length).unwrap()
                * PI
                * fore
                * fore
                / a_ref
        };
        for mach in [2.0, 3.5] {
            let method = whole.segment_slopes(mach, a_ref).unwrap();
            let share = |i| table.share(i, mach).unwrap().0;
            for i in [0, 1, 3] {
                close(
                    share(i),
                    method[i].slope_per_rad,
                    1e-12,
                    "the method's share",
                );
            }
            let first = first_in_place.segment_slopes(mach, a_ref).unwrap()[2].slope_per_rad
                + increment(mach, r_0, r_1, l_b1);
            let second = second_in_place.segment_slopes(mach, a_ref).unwrap()[4].slope_per_rad
                + increment(mach, r_1, r_2, l_b2);
            close(share(2), first, 1e-12, "first boattail");
            close(share(4), second, 1e-12, "second boattail");
            // Footnote 8 is the method's own share for both.
            let old = before.supersonic_body().unwrap();
            close(
                old.share(2, mach).unwrap().0,
                method[2].slope_per_rad,
                1e-12,
                "fn 8",
            );
            close(
                old.share(4, mach).unwrap().0,
                method[4].slope_per_rad,
                1e-12,
                "fn 8",
            );
            assert!(share(2) < method[2].slope_per_rad && share(4) < method[4].slope_per_rad);
        }
    }

    #[test]
    fn a_boattailed_body_flies_the_method_without_a_jump() {
        // M1.8e4: the finned rocket's 5.7° boattail and the tail behind it join the covered run;
        // since M1.8e6 the boattail takes Washington and Pettis's share, which takes lift off.
        let model = model(&finned_rocket(4));
        let join = model.supersonic_body().unwrap();
        assert_eq!(join.covered, 4);
        let start = join.join_start_mach;
        for mach in [
            start,
            start + SUPERSONIC_JOIN_WIDTH_MACH,
            1.35,
            2.0,
            2.05,
            3.0,
            4.63,
            4.95,
            4.999,
        ] {
            no_jump(&model, mach);
        }
        let (low, high) = (body_values(&model, 1.0), body_values(&model, 2.0));
        let (boattail, share) = (&model.bodies()[2], join.share(2, 2.0).unwrap());
        assert!(boattail.slope_per_rad < 0.0 && share.0 < 0.0, "{share:?}");
        // The join's weight is 1 at Mach 2; the rest is the join's rounding.
        assert!(
            (high[2][0] - share.0).abs() <= 1e-12 * share.0.abs(),
            "{share:?}"
        );
        // Its station stays slender-body theory's, on its own segment, at every Mach number;
        // the tail's, behind it, is its body-lift station, while its share is negative too.
        assert_eq!(low[2][2], high[2][2]);
        assert!((0.95..=1.0).contains(&high[2][2]), "{:?}", high[2]);
        assert!(high[3][0] < 0.0, "{:?}", high[3]);
        for mach in [1.2, 1.3, 1.5, 3.0, 4.999] {
            let at = body_values(&model, mach);
            assert_eq!((at[2][2], at[3][2]), (low[2][2], low[3][2]), "Mach {mach}");
        }
        assert!((1.0..=1.3).contains(&low[3][2]), "{:?}", low[3]);
        // The cylinder behind the nose carries lift past the join, as on a straight body.
        assert_eq!(low[1][0], 0.0);
        assert!(high[1][0] > 0.1, "{:?}", high[1]);
    }

    #[test]
    fn a_blunter_cone_joins_where_the_method_starts_to_hold() {
        // A 20° cone's shock detaches, or its surface flow turns subsonic, above Mach 1.2, so its
        // join starts at the table's first row and still doesn't jump.
        let mut rocket = straight_rocket();
        let length = 0.027 / 20.0_f64.to_radians().tan();
        rocket.stages[0].components[0].part = nose(NoseShape::Conical {}, length, 0.027);
        let model = model(&rocket);
        let join = model.supersonic_body().unwrap();
        let start = join.join_start_mach;
        assert!(start > SUPERSONIC_JOIN_START_MACH, "{start}");
        for mach in [start, start + SUPERSONIC_JOIN_WIDTH_MACH, start + 0.5] {
            no_jump(&model, mach);
        }
        // Below its join the terms are slender-body theory's.
        assert_eq!(body_values(&model, start), body_values(&model, 0.5));
    }

    #[test]
    fn the_joins_start_moves_with_the_nose_not_in_steps() {
        // Issue #87: the start used to snap to the table's 0.05 grid in Mach, so a steeper cone
        // moved it in steps. It now sits where the method starts to hold.
        let start_at = |degrees: f64| {
            let mut rocket = straight_rocket();
            let length = 0.027 / degrees.to_radians().tan();
            rocket.stages[0].components[0].part = nose(NoseShape::Conical {}, length, 0.027);
            let model = model(&rocket);
            let start = model.supersonic_body().unwrap().join_start_mach;
            (model, start)
        };
        let (model, start) = start_at(20.0);
        let grid = start * SUPERSONIC_STEPS_PER_MACH;
        assert!((grid - grid.round()).abs() > 1e-3, "on the grid: {start}");
        // Nothing jumps at the start, at the first even row after it, or across the join.
        let first_row = grid.ceil() / SUPERSONIC_STEPS_PER_MACH;
        for mach in [start, first_row, start + SUPERSONIC_JOIN_WIDTH_MACH] {
            no_jump(&model, mach);
        }
        assert_eq!(body_values(&model, start), body_values(&model, 0.5));
        // The start the aerodynamics page quotes, found by the method, not on the grid (1.35).
        assert!((start - 1.341910).abs() < 1e-6, "{start}");
        // The lead row is the method's own run there, where the cylinder's share climbs from zero
        // like the root of the distance in Mach (0.41 per radian at the first even row), not a
        // copy of that row. A start exact to the last bit leaves about 1e-7; 24 halvings of the
        // 0.05 step left 2.2e-5 to 2.5e-4, a sawtooth as the nose changed (issue #87).
        let join = model.supersonic_body().unwrap();
        let (lead, row) = (
            join.share(1, start).unwrap(),
            join.share(1, first_row).unwrap(),
        );
        assert!(lead.0 < 1e-5 && row.0 > 0.3, "{lead:?} against {row:?}");
        // A millionth of a degree moves the start by 2.7e-8.
        let (_, nudged) = start_at(20.0 + 1e-6);
        assert!((nudged - start).abs() < 1e-7, "{start} to {nudged}");
        // Steeper cones start later, each a little: no two share a grid row's Mach.
        let starts: Vec<f64> = [20.0, 20.1, 20.2, 20.3, 20.4, 20.5]
            .into_iter()
            .map(|degrees| start_at(degrees).1)
            .collect();
        assert!((starts[5] - 1.355500).abs() < 1e-6, "{starts:?}");
        for pair in starts.windows(2) {
            assert!(pair[1] > pair[0] && pair[1] - pair[0] < 0.02, "{starts:?}");
        }
    }

    #[test]
    fn below_the_join_the_bodies_keep_slender_body_terms() {
        let model = model(&straight_rocket());
        let slender: Vec<[f64; 3]> = model
            .bodies()
            .iter()
            .enumerate()
            .map(|(index, body)| {
                [
                    body.slope_per_rad,
                    body.moment_slope_m,
                    model.component_station_m(index, 0.0).unwrap(),
                ]
            })
            .collect();
        for mach in [0.0, 0.5, 0.99, 1.1, SUPERSONIC_JOIN_START_MACH] {
            let got = body_values(&model, mach);
            for (index, (g, want)) in got.iter().zip(&slender).enumerate() {
                assert_eq!(g[0], want[0], "body {index} at Mach {mach}");
                assert_eq!(g[2], want[2], "body {index} at Mach {mach}");
                if want[0] != 0.0 {
                    close(g[1], want[1], 1e-14, "moment");
                }
            }
        }
    }

    #[test]
    fn past_the_join_the_covered_bodies_take_the_method() {
        let model = model(&straight_rocket());
        let area = model.reference_area_m2();
        let body = straight_rocket_body();
        let join_end = SUPERSONIC_JOIN_START_MACH + SUPERSONIC_JOIN_WIDTH_MACH;
        // On the table's rows, the method itself; between them, within interpolation.
        for (mach, rel) in [
            (1.5, 1e-12),
            (2.0, 1e-12),
            (3.0, 1e-12),
            (4.95, 1e-12),
            (2.96, 1e-3),
        ] {
            assert!(mach >= join_end);
            let want = body.slope(mach, area).unwrap();
            let values = body_values(&model, mach);
            let slope: f64 = values.iter().map(|v| v[0]).sum();
            let moment: f64 = values.iter().map(|v| v[1]).sum();
            close(slope, want.slope_per_rad, rel, "slope");
            close(
                moment / slope,
                want.centre_of_pressure_m,
                rel,
                "centre of pressure",
            );
            // Each covered body's station is its share's centre of pressure, on its segment.
            let bounds = [(0.0, 0.25), (0.25, 0.95), (0.95, 1.0), (1.0, 1.3)];
            for (v, (fore, aft)) in values.iter().zip(bounds) {
                assert!(v[2] > fore && v[2] < aft, "{v:?} not in {fore} to {aft}");
            }
        }
    }

    /// The size of each switch issue #87 lists, measured on one rocket at Mach 3 and 4°: the
    /// whole rocket's normal force and centre of pressure either side of the threshold, as a
    /// share and in calibres. The lip's is gone since M1.8e10; these are what remain, and
    /// ADR-041 and the guide quote them from here.
    #[test]
    fn issue_87s_switches_are_this_big() {
        let at = |rocket: &hpr_design::Rocket| {
            let model = model(rocket);
            let force = model
                .normal_force(&flow(3.0, 4f64.to_radians(), 0.0))
                .unwrap();
            (
                force.coefficient,
                force.cp_station_m.unwrap() / model.reference_diameter_m(),
                model.supersonic_body().is_some(),
            )
        };
        // A step in radius, past a millionth of the cylinder's area: the run stops at it.
        let stepped = |drop_m: f64| {
            let mut rocket = straight_rocket();
            // The nose and its first tubes stay at 0.027 m; the last tube steps down, which is
            // where the run stops once the step passes a millionth of the cylinder's area.
            rocket.stages[0].components[3].part = body_part(0.3, 0.027 - drop_m, 0.027 - drop_m);
            rocket
        };
        // A flare behind the run, however small.
        let flared = |rise_m: f64| {
            let mut rocket = crate::testing::finned_rocket(4);
            rocket.stages[0].components.truncate(3);
            rocket.stages[0].components.push(component(
                "flare",
                body_part(1.0, 0.022, 0.022 + rise_m),
                None,
            ));
            rocket
        };
        // A pointed tip at TN 3527 Fig. 2's edge, 24°.
        let coned = |half_angle_deg: f64| {
            let mut rocket = straight_rocket();
            rocket.stages[0].components[0].part = nose(
                NoseShape::Conical {},
                0.027 / half_angle_deg.to_radians().tan(),
                0.027,
            );
            rocket
        };
        // A vertical tip whose base slope is steeper than the cap's handover, at 24°.
        let blunt = |length_m: f64| {
            let mut rocket = straight_rocket();
            rocket.stages[0].components[0].part =
                nose(NoseShape::PowerSeries { exponent: 0.5 }, length_m, 0.027);
            rocket
        };
        let at_16 = 0.027 / 24.0_f64.to_radians().tan();
        // Where the step's threshold sits, bracketed: a billionth of the radius, which is the
        // tangent body's own tolerance for two elements parallel but apart
        // (`shock_expansion::lay_out`), not the coverage gate's millionth of the area.
        assert!(model(&stepped(2.6e-11)).supersonic_body().is_some());
        assert!(model(&stepped(2.8e-11)).supersonic_body().is_none());
        // (what it is, the covered side, the bare side, what the docs say it is worth); a side
        // is (force, calibres, covered), and the expected pair is signed: bare over covered less
        // one, and bare's centre of pressure less covered's, in calibres.
        type Side = (f64, f64, bool);
        let switches: [(&str, Side, Side, (f64, f64)); 4] = [
            (
                "a step in radius",
                at(&stepped(1e-12)),
                at(&stepped(1e-9)),
                (-0.0865, 1.0285),
            ),
            (
                "a flare behind the run",
                at(&flared(1e-12)),
                at(&flared(1e-9)),
                (-0.2749, 0.2872),
            ),
            (
                "a pointed tip past 24°",
                at(&coned(23.999)),
                at(&coned(24.002)),
                (-0.1041, 1.1395),
            ),
            (
                "a vertical tip steeper than the handover",
                at(&blunt(0.5 * at_16 * 1.0002)),
                at(&blunt(0.5 * at_16 * 0.9998)),
                (-0.0699, 0.6383),
            ),
        ];
        for (what, covered, bare, (want_force, want_calibers)) in switches {
            assert!(
                covered.2 && !bare.2,
                "{what}: the method should cover one side only ({covered:?}, {bare:?})"
            );
            let force = bare.0 / covered.0 - 1.0;
            let calibers = bare.1 - covered.1;
            assert!(
                (force - want_force).abs() < 5e-4 && (calibers - want_calibers).abs() < 5e-4,
                "{what}: the force moves {force:.4} and the centre of pressure {calibers:.4} \
                 calibres, against {want_force} and {want_calibers}"
            );
        }
    }

    #[test]
    fn a_body_the_method_cannot_finish_keeps_slender_body_terms() {
        let at = |m: &AeroModel, mach| body_values(m, mach);
        // A flare (a boattail flies the method since M1.8e4), a vertical tip steeper than the
        // blunt tip's handover all the way to its base (a power-series nose one radius long;
        // longer ones fly the method since M1.8e7), a step in radius behind the nose.
        let mut rocket = straight_rocket();
        rocket.stages[0].components[2].part = body_part(0.05, 0.027, 0.032);
        rocket.stages[0].components[3].part = body_part(0.3, 0.032, 0.032);
        let flared = model(&rocket);
        // A boattail followed by a flare (a lip), and a boattail at a step down: each leaves a
        // body with a slope of its own behind the run.
        let mut rocket = crate::testing::finned_rocket(4);
        rocket.stages[0]
            .components
            .push(component("lip", body_part(0.01, 0.022, 0.025), None));
        let lipped = model(&rocket);
        let mut rocket = crate::testing::finned_rocket(4);
        rocket.stages[0].components[2].part = body_part(0.05, 0.026, 0.022);
        let stepped_boattail = model(&rocket);
        let mut rocket = straight_rocket();
        rocket.stages[0].components[0].part =
            nose(NoseShape::PowerSeries { exponent: 0.5 }, 0.027, 0.027);
        let blunt = model(&rocket);
        let mut rocket = straight_rocket();
        rocket.stages[0].components[1].part = body_part(0.7, 0.03, 0.03);
        let stepped = model(&rocket);
        for model in [&flared, &lipped, &stepped_boattail, &blunt, &stepped] {
            assert!(model.supersonic_body().is_none());
            assert_eq!(at(model, 3.0), at(model, 0.5));
        }
    }

    #[test]
    fn clones_share_the_table_and_compare_equal() {
        let model = model(&straight_rocket());
        let clone = model.clone();
        assert_eq!(model, clone);
        let built = model.supersonic_body().unwrap() as *const SupersonicBody;
        assert_eq!(model, clone);
        assert!(std::ptr::eq(built, clone.supersonic_body().unwrap()));
    }
}
