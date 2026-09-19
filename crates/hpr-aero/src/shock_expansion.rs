//! The normal force of a pointed body of revolution faster than sound, by Syvertson and Dennis's
//! second-order shock-expansion method (NACA TN 3527, 1956, also NACA Report 1328).
//!
//! **Flown faster than sound** for a pointed nose and the cylinders behind it since the milestone
//! [M1.8e2](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e2), the body's
//! supersonic normal force in flight, and for boattails and cylinders behind those since
//! [M1.8e4](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e4), and behind a
//! blunt or vertical nose tip's Newtonian cap ([`crate::blunt_tip`]) since
//! [M1.8e7](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e7), through [`crate::model::SupersonicBody`]. The guide's
//! [Bodies faster than sound](https://nrdptel.github.io/hpr-sim/physics/aero.html#bodies-faster-than-sound)
//! explains the method and how it was checked.
//!
//! ```
//! use hpr_aero::shock_expansion::{BodySegment, DEFAULT_ELEMENTS_PER_CURVE, ShockExpansionBody};
//! use hpr_design::{NoseShape, Profile};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // A cone five calibres long on a cylinder of four, 1 m across, at Mach 4.24.
//! let body = ShockExpansionBody::new(
//!     &[
//!         BodySegment::Profile {
//!             profile: Profile::nose(NoseShape::Conical {}, 5.0, 0.5)?,
//!         },
//!         BodySegment::Cylinder {
//!             length_m: 4.0,
//!             radius_m: 0.5,
//!         },
//!     ],
//!     DEFAULT_ELEMENTS_PER_CURVE,
//! )?;
//! let slope = body.slope(4.24, std::f64::consts::PI / 4.0)?;
//! // Slender-body theory: 2 per radian. TN 3527's own value: 2.91; its wind tunnel: 2.84.
//! assert!((slope.slope_per_rad - 2.922).abs() < 5e-4);
//! # Ok(())
//! # }
//! ```
//!
//! Slender-body theory gives a body's nose `C_Nα = 2` and its cylinder nothing, at every Mach
//! number ([B67] p. 18). Faster than sound the cylinder behind a nose carries lift too: the flow
//! that expands around the shoulder recovers toward free-stream pressure along the cylinder, and
//! at an angle of attack it recovers unevenly around it. The method computes that loading, at
//! `α → 0`, for a pointed body whose flow is supersonic everywhere.
//!
//! **The tangent body.** The profile is replaced by straight elements tangent to it (TN 3527
//! sketch (a), p. 6): the first tangent at the vertex, so the flow there is exactly a cone's
//! (Taylor–Maccoll), the rest meeting at corners. Around each corner the flow turns by
//! Prandtl–Meyer; along each element the surface pressure relaxes exponentially from its value
//! behind the corner toward the pressure on a cone tangent to the body there (eq. 8):
//!
//! `p = p_c − (p_c − p₂) e^(−η)`, `η = (∂p/∂s)₂ (x − x₂) / ((p_c − p₂) cos δ₂)` (eq. 9),
//!
//! where the gradient just behind a corner comes from the one ahead of it (eq. 4, straight
//! elements):
//!
//! `(∂p/∂s)₂ = (B₂/r)(Ω₁/Ω₂ · sin δ₁ − sin δ₂) + (B₂Ω₁)/(B₁Ω₂) · (∂p/∂s)₁`,
//!
//! with `B = γpM²/(2(M² − 1))` (eq. 6), `Ω` the one-dimensional area ratio `A/A*` (eq. 7), and at
//! the element's end `(∂p/∂s)₃ = (p_c − p₃)/(p_c − p₂) · (∂p/∂s)₂` (eq. 10).
//!
//! **The loading.** Near `α = 0` the lifting pressures follow the same law (eq. 19):
//!
//! `Λ = (1 − e^(−η)) tan δ · (dC_N/dα)_tc + (λ₂/λ₁) e^(−η) Λ₁`, `λ = 2γp / sin 2μ` (eq. 5),
//!
//! where `(dC_N/dα)_tc` is the tangent cone's slope (Fig. 2, read by hand into
//! [`cone_normal_force_slope`]) and `Λ₁` the loading just ahead of the corner; on the vertex
//! cone `Λ = tan δ_v · (dC_N/dα)_tcv`. The slope and moment follow by integration over the body
//! (eqs. 14 and 21):
//!
//! `C_Nα = (2π/A_ref) ∫ Λ r dx`, `x_cp = ∫ Λ r x dx / ∫ Λ r dx` (from the vertex).
//!
//! A cylinder element's tangent cone is the free stream (`p_c = p₀`, `tan δ = 0`), so its
//! loading decays to zero. A boattail element has no tangent cone; footnote 8 (p. 12) takes
//! `p_c = p₀` and `(dC_N/dα)_tc = 2`, which the report found reasonable "for bodies having
//! moderate amounts of boattail". That is unvalidated here.
//!
//! **Limits.** The report states the method for `M/f_n` (Mach number over nose fineness) from
//! 0.4 to 2, within ±0.2 per radian and ±0.2 calibers of its measurements (Summary, p. 1). A
//! pointed tip's cone shock must be attached; a blunt or vertical tip (an infinite slope, or a
//! [`BodySegment::SphericalCap`]) takes TN D-4865's Newtonian cap and starts the march at its
//! handover ([`crate::blunt_tip`], [`HandoverStart`]). Fig. 2 spans Mach 3 to 10; below Mach 3 its
//! Mach 3 curve is held, and above 10 its Mach 10 curve, both assumptions. Viscous crossflow is
//! not part of it: the method is the slope at `α → 0`.
//!
//! [B67]: https://ntrs.nasa.gov/citations/19660030728

use hpr_core::quadrature::{Tolerance, integrate};
use hpr_design::{NoseShape, Profile};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

use crate::afterbody::{GAMMA, MAX_TURNING_RAD, inverse_prandtl_meyer, prandtl_meyer};
use crate::error::{AeroError, check_dimension};

/// `(γ − 1)/2`.
const G1: f64 = 0.5 * (GAMMA - 1.0);

/// The number of straight elements a curved segment's tangent body gets by default: TN 3527's
/// own, tangent at `x/l = 0, 0.1, …, 1.0` ("in all applications of the present method to curved
/// bodies", footnote 9, p. 15). Four times as many move the report's ogive-cylinders by under
/// 0.01 per radian and 0.01 calibers (test `curved_elements_converge`).
pub const DEFAULT_ELEMENTS_PER_CURVE: usize = 10;

/// The most elements a curved segment may take, far past where the result stops changing.
pub const MAX_ELEMENTS_PER_CURVE: usize = 1000;

/// One piece of a body of revolution, listed from the nose aft.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum BodySegment {
    /// A nose cone (the first segment, pointed or with a vertical tip) or a transition.
    Profile {
        /// The profile.
        profile: Profile,
    },
    /// A cylinder.
    Cylinder {
        /// Length, m.
        length_m: f64,
        /// Radius, m.
        radius_m: f64,
    },
    /// A sphere's cap from its pole, the first segment of a sphere-cone: the first `length_m` of
    /// a sphere of `radius_m`, up to a hemisphere. Its tip is blunt, so the body flies
    /// TN D-4865's Newtonian cap ahead of the method ([`crate::blunt_tip`]).
    SphericalCap {
        /// The sphere's radius, m.
        radius_m: f64,
        /// Length along the axis from the pole, m, in `(0, radius_m]`.
        length_m: f64,
    },
}

impl BodySegment {
    fn length_m(&self) -> f64 {
        match self {
            Self::Profile { profile } => profile.length_m(),
            Self::Cylinder { length_m, .. } | Self::SphericalCap { length_m, .. } => *length_m,
        }
    }

    /// Radius and slope `dr/dx` at `x_m` aft of the segment's forward end; the slope is infinite
    /// at a blunt tip.
    fn radius_and_slope(&self, x_m: f64) -> (f64, f64) {
        match self {
            Self::Profile { profile } => profile.radius_and_slope(x_m),
            Self::Cylinder { radius_m, .. } => (*radius_m, 0.0),
            Self::SphericalCap { radius_m, length_m } => {
                let x = x_m.clamp(0.0, *length_m);
                let r = (x * (2.0 * radius_m - x)).max(0.0).sqrt();
                if r == 0.0 {
                    (0.0, f64::INFINITY)
                } else {
                    (r, (radius_m - x) / r)
                }
            }
        }
    }

    fn fore_radius_m(&self) -> f64 {
        self.radius_and_slope(0.0).0
    }

    fn aft_radius_m(&self) -> f64 {
        self.radius_and_slope(self.length_m()).0
    }

    /// Whether the profile is straight, so one element covers it.
    fn is_straight(&self) -> bool {
        match self {
            Self::Profile { profile } => matches!(profile.shape(), NoseShape::Conical {}),
            Self::Cylinder { .. } => true,
            Self::SphericalCap { .. } => false,
        }
    }
}

/// A straight element of the tangent body: where it starts (its corner with the element ahead,
/// the vertex, or a blunt tip's handover) and its angle to the axis.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Element {
    /// Whether the element is tangent to the first segment, the nose.
    on_nose: bool,
    corner_x_m: f64,
    corner_radius_m: f64,
    angle_rad: f64,
}

/// A body of revolution laid out for the second-order shock-expansion method: its segments and
/// the straight elements of its tangent body. A pointed nose's elements are laid out once; a blunt
/// tip's start at a handover that moves with the Mach number ([`crate::blunt_tip`]), so they are
/// laid out at each.
#[derive(Debug, Clone, PartialEq)]
pub struct ShockExpansionBody {
    /// Each segment with the station of its forward end, m aft of the vertex.
    segments: Vec<(f64, BodySegment)>,
    length_m: f64,
    elements_per_curve: usize,
    /// The tangent body's elements from the vertex; empty for a blunt tip.
    elements: Vec<Element>,
    /// Whether the tip is blunt or vertical (an infinite slope at the vertex).
    blunt: bool,
    /// Where the march behind a blunt tip's cap starts from.
    handover_start: HandoverStart,
}

/// Where the method's march starts behind a blunt tip's Newtonian cap
/// ([`crate::blunt_tip`]; the decision record on it, [ADR-038][adr-038]).
///
/// [adr-038]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-038-blunt-and-vertical-nose-tips-faster-than-sound-by-a-newtonian-cap-the-method-started-from-the-tangent-cone-2026-09-19
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HandoverStart {
    /// As the method starts at a pointed vertex: the flow on the cone tangent to the body at the
    /// handover, that cone's loading and no pressure gradient (TN 3527 sketch (a), p. 6). hpr's
    /// choice, and what a flight takes.
    #[default]
    TangentCone,
    /// TN D-4865's own: the Newtonian pressure and Mach number there (eqs. 1 and 2), the total
    /// pressure behind the normal shock and no gradient, with the loading of a handover fixed in
    /// the wind ([`crate::blunt_tip::handover_loading`]). Kept to compare: on the Arcas Robin's
    /// nose the march then fails from Mach 3.96.
    Newtonian,
}

/// A blunt tip's Newtonian cap at one Mach number: where it hands over and its `C_p,max`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cap {
    end_x_m: f64,
    c_p_max: f64,
}

/// The flow over the body at one Mach number: a blunt tip's cap, then each element's flow.
#[derive(Debug, Clone, PartialEq)]
struct March {
    cap: Option<Cap>,
    flows: Vec<ElementFlow>,
}

/// One segment's share of the body's normal-force slope at `α → 0`
/// ([`ShockExpansionBody::segment_slopes`]).
///
/// A share can be negative or zero (a boattail's, TN 3527 footnote 8, p. 12), so `moment_slope_m /
/// slope_per_rad` need not lie within its segment and is unbounded where a share crosses zero:
/// carry the moment, not a station.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SegmentSlope {
    /// The segment's `C_Nα`, per radian, on the reference area given.
    pub slope_per_rad: f64,
    /// That slope's moment about the vertex, `C_Nα · x̄`, m per radian (x̄ aft of the vertex).
    pub moment_slope_m: f64,
}

/// The body's normal-force slope at `α → 0` and where it acts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ShockExpansionSlope {
    /// `C_Nα`, per radian, on the reference area given.
    pub slope_per_rad: f64,
    /// The centre of pressure, m aft of the vertex.
    pub centre_of_pressure_m: f64,
}

impl ShockExpansionBody {
    /// Lays out a body from its segments, nose first: a curved segment gets
    /// `elements_per_curve` equal steps in `x` (tangent at both ends and between), a straight
    /// one a single element. A blunt tip's first segment gets its steps from the handover aft,
    /// laid out at each Mach number ([`crate::blunt_tip`]).
    ///
    /// # Errors
    ///
    /// - [`AeroError::Unsupported`] if the first segment doesn't close to a point at its front
    ///   (pointed, or blunt with a vertical tip), a spherical cap isn't the first segment, the
    ///   radius steps between segments (by more than a millionth of it), the radius falls to zero
    ///   anywhere but the tip, or, for a pointed nose, the tangent lines of consecutive elements
    ///   don't meet in order along the body (a profile the tangent body can't follow).
    /// - [`AeroError::Domain`] for no segments, elements per curve outside
    ///   `1..=`[`MAX_ELEMENTS_PER_CURVE`], a negative or non-finite cylinder dimension, or a
    ///   spherical cap whose radius isn't finite and positive or whose length isn't in
    ///   `(0, radius]`.
    pub fn new(segments: &[BodySegment], elements_per_curve: usize) -> Result<Self, AeroError> {
        let Some(first) = segments.first() else {
            return Err(AeroError::Domain {
                what: "number of body segments",
                value: 0.0,
            });
        };
        if !(1..=MAX_ELEMENTS_PER_CURVE).contains(&elements_per_curve) {
            return Err(AeroError::Domain {
                what: "elements per curved segment",
                value: elements_per_curve as f64,
            });
        }
        for (index, segment) in segments.iter().enumerate() {
            match segment {
                BodySegment::Cylinder { length_m, radius_m } => {
                    check_dimension("cylinder length", *length_m, true)?;
                    check_dimension("cylinder radius", *radius_m, false)?;
                }
                BodySegment::SphericalCap { radius_m, length_m } => {
                    check_dimension("spherical cap radius", *radius_m, false)?;
                    if !(*length_m > 0.0 && *length_m <= *radius_m) {
                        return Err(AeroError::Domain {
                            what: "spherical cap length",
                            value: *length_m,
                        });
                    }
                    if index > 0 {
                        return Err(AeroError::Unsupported(
                            "a spherical cap can only be the body's first segment".to_owned(),
                        ));
                    }
                }
                BodySegment::Profile { .. } => {}
            }
        }
        let tip_slope = first.radius_and_slope(0.0).1;
        // A NaN slope compares as nothing, so it is refused too.
        if matches!(first, BodySegment::Cylinder { .. })
            || first.fore_radius_m() != 0.0
            || tip_slope.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater)
        {
            return Err(AeroError::Unsupported(
                "the second-order shock-expansion method needs a nose that closes to a point at \
                 its front, pointed or blunt"
                    .to_owned(),
            ));
        }
        let blunt = tip_slope.is_infinite();
        let mut laid = Vec::with_capacity(segments.len());
        let mut station = 0.0;
        let mut previous_aft: Option<f64> = None;
        for segment in segments {
            if let Some(aft) = previous_aft {
                let fore = segment.fore_radius_m();
                if (fore - aft).abs() > 1e-6 * aft.max(fore) {
                    return Err(AeroError::Unsupported(format!(
                        "the second-order shock-expansion method needs a continuous profile; the \
                         radius steps from {aft} m to {fore} m at {station} m"
                    )));
                }
            }
            laid.push((station, *segment));
            station += segment.length_m();
            previous_aft = Some(segment.aft_radius_m());
        }
        let length_m = station;
        let elements = if blunt {
            Vec::new()
        } else {
            lay_out(&laid, length_m, elements_per_curve, 0.0)?
        };
        Ok(Self {
            segments: laid,
            length_m,
            elements_per_curve,
            elements,
            blunt,
            handover_start: HandoverStart::default(),
        })
    }

    /// This body with its march behind a blunt tip's cap starting from `start`; a pointed body
    /// is unchanged.
    #[must_use]
    pub fn with_handover_start(mut self, start: HandoverStart) -> Self {
        self.handover_start = start;
        self
    }

    /// The body's length, m.
    pub fn length_m(&self) -> f64 {
        self.length_m
    }

    /// The tip's half-angle, rad: `π/2` for a blunt or vertical tip.
    pub fn vertex_angle_rad(&self) -> f64 {
        // `new` refuses a body without segments.
        self.segments[0].1.radius_and_slope(0.0).1.atan()
    }

    /// Whether the tip is blunt or vertical, so the body flies TN D-4865's Newtonian cap ahead of
    /// the method ([`crate::blunt_tip`]).
    pub fn has_blunt_tip(&self) -> bool {
        self.blunt
    }

    /// Where a blunt tip's cap hands over to the method at Mach `mach`, m aft of the vertex:
    /// where the first segment's slope falls to [`crate::blunt_tip::handover_angle_rad`]. `None`
    /// for a pointed tip.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a Mach number that isn't finite and above 1.
    /// - [`AeroError::Unsupported`] if the first segment is steeper than the handover's slope all
    ///   the way to its end.
    pub fn handover_m(&self, mach: f64) -> Result<Option<f64>, AeroError> {
        check_mach(mach)?;
        if !self.blunt {
            return Ok(None);
        }
        let angle = crate::blunt_tip::handover_angle_rad(mach)?;
        let target = angle.tan();
        // `new` refuses a body without segments.
        let first = &self.segments[0].1;
        let length = first.length_m();
        if first.radius_and_slope(length).1 > target {
            return Err(AeroError::Unsupported(format!(
                "the nose is steeper than the blunt tip's handover slope, {}°, all the way to its \
                 end at Mach {mach}",
                angle.to_degrees()
            )));
        }
        // The slope falls from infinite at the tip; bisect to the last bit of an `f64` for the
        // first station where it is at most the handover's.
        let (mut low, mut high) = (0.0_f64, length);
        for _ in 0..HANDOVER_BISECTIONS {
            let mid = 0.5 * (low + high);
            if mid <= low || mid >= high {
                break;
            }
            if first.radius_and_slope(mid).1 > target {
                low = mid;
            } else {
                high = mid;
            }
        }
        Ok(Some(high))
    }

    fn radius_and_slope_m(&self, x_m: f64) -> (f64, f64) {
        let index = self
            .segments
            .partition_point(|(start, _)| *start <= x_m)
            .saturating_sub(1);
        let (start, segment) = &self.segments[index];
        segment.radius_and_slope(x_m - start)
    }

    fn radius_m(&self, x_m: f64) -> f64 {
        self.radius_and_slope_m(x_m).0
    }

    /// `C_Nα` (per radian, on `reference_area_m2`) and the centre of pressure at Mach `mach`, by
    /// TN 3527's multi-step method.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a Mach number that isn't above 1, or a reference area that
    ///   isn't positive.
    /// - [`AeroError::Unsupported`] where the method doesn't hold: a tip cone whose shock
    ///   detaches, a tangent cone steeper than Fig. 2's 24°, a corner the flow can't turn
    ///   supersonically, a tip cone whose surface flow is subsonic, an element aft of the nose
    ///   whose pressure moves away from its tangent cone's, or a lift that doesn't sum to a
    ///   positive force; and for a blunt tip, whose elements are laid out at each Mach number, a
    ///   nose steeper than the handover's slope all the way to its end, or a tangent body whose
    ///   elements don't meet in order behind the handover (as [`Self::new`] says for a pointed
    ///   one).
    ///
    /// The report states the method for Mach number over nose fineness from 0.4 to 2 (Summary,
    /// p. 1); `slope` doesn't enforce that range, and its own Mach 6.28 rows are at 2.09.
    pub fn slope(
        &self,
        mach: f64,
        reference_area_m2: f64,
    ) -> Result<ShockExpansionSlope, AeroError> {
        let windows = self.windows(mach, reference_area_m2)?;
        let (force, moment) = total_lift(&windows)?;
        Ok(ShockExpansionSlope {
            slope_per_rad: 2.0 * PI * force / reference_area_m2,
            centre_of_pressure_m: moment / force,
        })
    }

    /// Each segment's share of [`Self::slope`], in the order of the segments: its `C_Nα` (per
    /// radian, on `reference_area_m2`) and that slope's moment about the vertex. The shares sum
    /// to the whole body's slope and moment up to rounding: every segment's start is a break of
    /// the integral, so each piece of it lies inside one segment.
    ///
    /// # Errors
    ///
    /// As [`Self::slope`].
    pub fn segment_slopes(
        &self,
        mach: f64,
        reference_area_m2: f64,
    ) -> Result<Vec<SegmentSlope>, AeroError> {
        let windows = self.windows(mach, reference_area_m2)?;
        total_lift(&windows)?;
        let per_unit = 2.0 * PI / reference_area_m2;
        let mut shares = vec![SegmentSlope::default(); self.segments.len()];
        for (start_m, [force, moment]) in windows {
            let index = self
                .segments
                .partition_point(|(start, _)| *start <= start_m)
                .saturating_sub(1);
            shares[index].slope_per_rad += per_unit * force;
            shares[index].moment_slope_m += per_unit * moment;
        }
        Ok(shares)
    }

    /// How many of the nose's elements the march reduces to the generalized method at Mach
    /// `mach`: those where the gradient behind the corner points away from the tangent cone's
    /// pressure (`η < 0`, TN 3527 p. 13), which carry no gradient on (see
    /// [issue #81](https://github.com/nrdptel/hpr-sim/issues/81)). Zero means the result doesn't
    /// depend on that reading.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a Mach number that isn't finite and above 1.
    /// - [`AeroError::Unsupported`] where the march fails, as for [`Self::slope`]: a detached tip
    ///   shock, a tangent cone past Fig. 2's 24°, a corner the flow can't turn, subsonic surface
    ///   flow, or an element aft of the nose that would be reduced. An `Ok` count doesn't promise
    ///   that [`Self::slope`] succeeds: it also needs a positive total lift.
    pub fn reduced_elements(&self, mach: f64) -> Result<usize, AeroError> {
        check_mach(mach)?;
        Ok(self
            .flows(mach)?
            .flows
            .iter()
            .filter(|f| f.is_reduced())
            .count())
    }

    /// The integrals of the lift per unit length and of its moment about the vertex (both over
    /// `2π`), one per piece between consecutive corners, segment starts, a blunt tip's handover
    /// and the body's end, keyed by the piece's forward end.
    fn windows(
        &self,
        mach: f64,
        reference_area_m2: f64,
    ) -> Result<Vec<(f64, [f64; 2])>, AeroError> {
        check_mach(mach)?;
        check_dimension("reference area", reference_area_m2, false)?;
        let March { cap, flows } = self.flows(mach)?;
        let cap_end_m = cap.map_or(0.0, |c| c.end_x_m);
        let loading = |x: f64| {
            if let Some(cap) = cap.filter(|c| x < c.end_x_m) {
                return crate::blunt_tip::newtonian_loading_at_slope(
                    cap.c_p_max,
                    self.radius_and_slope_m(x).1,
                );
            }
            let index = flows
                .partition_point(|f| f.corner_x_m <= x)
                .saturating_sub(1);
            flows[index].at(x).1
        };
        let mut breaks: Vec<f64> = flows
            .iter()
            .map(|f| f.corner_x_m)
            .chain(self.segments.iter().map(|(start, _)| *start))
            .chain([0.0, cap_end_m, self.length_m])
            .filter(|x| *x >= 0.0 && *x <= self.length_m)
            .collect();
        breaks.sort_by(f64::total_cmp);
        breaks.dedup();
        let scale = self.length_m * self.radius_m(self.length_m).max(1e-12);
        let tolerance = Tolerance {
            relative: 1e-11,
            absolute: 1e-13 * scale,
            max_intervals: 4000,
        };
        breaks
            .windows(2)
            .map(|pair| {
                let integral = integrate(
                    |x| {
                        let lr = loading(x) * self.radius_m(x);
                        [lr, lr * x]
                    },
                    pair[0],
                    pair[1],
                    tolerance,
                )?;
                Ok((pair[0], integral.value))
            })
            .collect()
    }

    /// Marches the flow over the tangent body's elements at Mach `mach`: from the vertex's cone
    /// for a pointed tip, or from a blunt tip's handover, behind its Newtonian cap.
    fn flows(&self, mach: f64) -> Result<March, AeroError> {
        let (cap, elements, first, total) = if self.blunt {
            let (cap, elements, first, total) = self.handover_flow(mach)?;
            (cap, std::borrow::Cow::Owned(elements), first, total)
        } else {
            // `new` lays out a pointed body's elements, the vertex's first.
            let vertex = self.elements[0];
            let cone = cone_flow(mach, vertex.angle_rad)?;
            if cone.surface_mach <= 1.0 {
                return Err(AeroError::Unsupported(format!(
                    "the flow on the tip's cone is subsonic (Mach {}) at Mach {mach}",
                    cone.surface_mach
                )));
            }
            let total = cone.surface_pressure_ratio * total_over_static(cone.surface_mach);
            let vertex_slope = cone_normal_force_slope(mach, vertex.angle_rad)?;
            let first = ElementFlow {
                corner_x_m: 0.0,
                angle_rad: vertex.angle_rad,
                pressure: cone.surface_pressure_ratio,
                gradient: 0.0,
                load: vertex.angle_rad.tan() * vertex_slope,
                cone_pressure: cone.surface_pressure_ratio,
                cone_slope: vertex_slope,
            };
            (
                None,
                std::borrow::Cow::Borrowed(&self.elements[..]),
                first,
                total,
            )
        };
        let mut flows = vec![first];
        for element in &elements[1..] {
            let before = flows[flows.len() - 1];
            let (p1, load1) = before.at(element.corner_x_m);
            let gradient1 = before.gradient_at(p1);
            let m1 = mach_from_pressure(total, p1)?;
            let nu2 = prandtl_meyer(m1) + (before.angle_rad - element.angle_rad);
            if !(nu2 > 0.0 && nu2 < MAX_TURNING_RAD) {
                return Err(AeroError::Unsupported(format!(
                    "the flow at Mach {m1} can't turn through {} rad supersonically at {} m",
                    before.angle_rad - element.angle_rad,
                    element.corner_x_m
                )));
            }
            let m2 = inverse_prandtl_meyer(nu2);
            let p2 = total / total_over_static(m2);
            let (b1, b2) = (b_factor(p1, m1), b_factor(p2, m2));
            let (o1, o2) = (area_ratio(m1), area_ratio(m2));
            let gradient2 = b2 / element.corner_radius_m
                * (o1 / o2 * before.angle_rad.sin() - element.angle_rad.sin())
                + b2 * o1 / (b1 * o2) * gradient1;
            let load2 = lambda(p2, m2) / lambda(p1, m1) * load1;
            let (cone_pressure, cone_slope) = if element.angle_rad > CONE_ANGLE_FLOOR_RAD {
                (
                    cone_flow(mach, element.angle_rad)?.surface_pressure_ratio,
                    cone_normal_force_slope(mach, element.angle_rad)?,
                )
            } else {
                // A cylinder's tangent cone is the free stream; a boattail's is footnote 8's.
                (1.0, 2.0)
            };
            let flow = ElementFlow {
                corner_x_m: element.corner_x_m,
                angle_rad: element.angle_rad,
                pressure: p2,
                gradient: gradient2,
                load: load2,
                cone_pressure,
                cone_slope,
            };
            // A reduced element keeps the loading behind its corner all along it. On a nose that
            // is one short element; aft of the nose, a cylinder, boattail or long shallow flare
            // would carry that loading over any length, so hpr refuses it there.
            if flow.is_reduced() && (element.angle_rad <= CONE_ANGLE_FLOOR_RAD || !element.on_nose)
            {
                return Err(AeroError::Unsupported(format!(
                    "behind the corner at {} m, aft of the nose, the pressure moves away from its \
                     tangent cone's",
                    element.corner_x_m
                )));
            }
            flows.push(flow);
        }
        Ok(March { cap, flows })
    }

    /// A blunt tip at Mach `mach` ([`crate::blunt_tip`]): its Newtonian cap and handover
    /// (TN D-4865), the elements from the handover aft, the flow just behind the handover (as
    /// [`HandoverStart`] says) and the total pressure the march expands from.
    fn handover_flow(
        &self,
        mach: f64,
    ) -> Result<(Option<Cap>, Vec<Element>, ElementFlow, f64), AeroError> {
        // A blunt body always has a handover where the method holds.
        let Some(end_x_m) = self.handover_m(mach)? else {
            return Err(AeroError::Unsupported(
                "a blunt tip without a handover".to_owned(),
            ));
        };
        let elements = lay_out(
            &self.segments,
            self.length_m,
            self.elements_per_curve,
            end_x_m,
        )?;
        // `lay_out` always returns the handover's element first.
        let handover = elements[0];
        let cone = cone_flow(mach, handover.angle_rad)?;
        let cone_slope = cone_normal_force_slope(mach, handover.angle_rad)?;
        let (pressure, load, total) = match self.handover_start {
            HandoverStart::TangentCone => {
                if cone.surface_mach <= 1.0 {
                    return Err(AeroError::Unsupported(format!(
                        "the flow on the cone tangent at the blunt tip's handover is subsonic \
                         (Mach {}) at Mach {mach}",
                        cone.surface_mach
                    )));
                }
                (
                    cone.surface_pressure_ratio,
                    handover.angle_rad.tan() * cone_slope,
                    cone.surface_pressure_ratio * total_over_static(cone.surface_mach),
                )
            }
            HandoverStart::Newtonian => {
                use crate::blunt_tip::{
                    handover_loading, newtonian_pressure_ratio, newtonian_surface_mach,
                    pitot_pressure_ratio,
                };
                let pressure = newtonian_pressure_ratio(mach, handover.angle_rad)?;
                let surface_mach = newtonian_surface_mach(mach, pressure)?;
                if surface_mach <= 1.0 {
                    return Err(AeroError::Unsupported(format!(
                        "the flow at the blunt tip's handover is subsonic (Mach {surface_mach}) \
                         at Mach {mach}"
                    )));
                }
                (
                    pressure,
                    handover_loading(mach, pressure, surface_mach)?,
                    pitot_pressure_ratio(mach)?,
                )
            }
        };
        let first = ElementFlow {
            corner_x_m: end_x_m,
            angle_rad: handover.angle_rad,
            pressure,
            gradient: 0.0,
            load,
            cone_pressure: cone.surface_pressure_ratio,
            cone_slope,
        };
        let cap = Cap {
            end_x_m,
            c_p_max: crate::blunt_tip::newtonian_pressure_coefficient_max(mach)?,
        };
        Ok((Some(cap), elements, first, total))
    }
}

/// The tangent body's elements over `segments` (each with its fore station, the body `length_m`
/// long): tangent at `elements_per_curve` equal steps along a curved segment (a straight one
/// takes one element), the first segment's steps from `start_m`, the vertex or a blunt tip's
/// handover. The first element starts at `start_m`.
///
/// # Errors
///
/// [`AeroError::Unsupported`] where the tangent lines of consecutive elements don't meet in order
/// along the body, meet at no radius, or are parallel but apart.
fn lay_out(
    segments: &[(f64, BodySegment)],
    length_m: f64,
    elements_per_curve: usize,
    start_m: f64,
) -> Result<Vec<Element>, AeroError> {
    // The tangency points: (x, r, slope, on the nose).
    let mut points = Vec::new();
    for (index, (start, segment)) in segments.iter().enumerate() {
        let (from, span) = if index == 0 {
            (start_m, segment.length_m() - start_m)
        } else {
            (0.0, segment.length_m())
        };
        let steps = if segment.is_straight() {
            1
        } else {
            elements_per_curve
        };
        let count = if segment.is_straight() { 1 } else { steps + 1 };
        for i in 0..count {
            let local = from + span * i as f64 / steps as f64;
            let (r, slope) = segment.radius_and_slope(local);
            points.push((start + local, r, slope, index == 0));
        }
    }

    // `segments` isn't empty, so `points` holds at least the first segment's start.
    let (x0, r0, t0, _) = points[0];
    let mut elements = vec![Element {
        on_nose: true,
        corner_x_m: x0,
        corner_radius_m: r0,
        angle_rad: t0.atan(),
    }];
    let (mut xp, mut rp, mut tp) = (x0, r0, t0);
    for &(x, r, t, on_nose) in &points[1..] {
        // A point on the previous element's line adds nothing. Nor does one whose tangent turns
        // by under `NEARLY_PARALLEL_RAD`: its corner with the previous tangent would be lost in the
        // profile's rounding (a blunt tip's handover close to the nose's end packs its elements
        // into a few nanometres), and so small a turn changes nothing the method computes.
        let on_line = r - (rp + tp * (x - xp));
        if (t.atan() - tp.atan()).abs() <= NEARLY_PARALLEL_RAD
            && on_line.abs() <= 1e-9 * r.max(1e-12)
        {
            continue;
        }
        if (t - tp).abs() <= 1e-12 * (1.0 + tp.abs()) {
            return Err(AeroError::Unsupported(format!(
                "the tangent body's elements at {xp} m and {x} m are parallel but apart"
            )));
        }
        let corner_x = (r - rp + tp * xp - t * x) / (tp - t);
        let corner_r = rp + tp * (corner_x - xp);
        if corner_r.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err(AeroError::Unsupported(format!(
                "the tangent body's corner at {corner_x} m has no radius: the method needs the \
                 body open everywhere but its tip"
            )));
        }
        // `elements` starts with the first element's.
        let last = elements[elements.len() - 1].corner_x_m;
        if !(corner_x.is_finite() && corner_x >= last && corner_x <= x + 1e-12 * length_m) {
            return Err(AeroError::Unsupported(format!(
                "the tangent body's corner at {corner_x} m falls outside [{last}, {x}] m: the \
                 profile turns too quickly for its elements"
            )));
        }
        elements.push(Element {
            on_nose,
            corner_x_m: corner_x,
            corner_radius_m: corner_r,
            angle_rad: t.atan(),
        });
        (xp, rp, tp) = (x, r, t);
    }
    Ok(elements)
}

/// The method's Mach number: finite and above 1.
fn check_mach(mach: f64) -> Result<(), AeroError> {
    if mach.is_finite() && mach > 1.0 {
        Ok(())
    } else {
        Err(AeroError::Domain {
            what: "Mach number of the second-order shock-expansion method",
            value: mach,
        })
    }
}

/// Below this angle an element is a cylinder: its tangent cone is the free stream.
const CONE_ANGLE_FLOOR_RAD: f64 = 1e-9;

/// A tangency point turning the tangent body by less than this, rad, on the previous element's
/// line, is merged into that element. Corners of so small a turn are ill-conditioned: two tangents
/// a distance `h` apart on a curve of curvature `κ` meet at `h/2` from a numerator of order `κh²`,
/// while the profile's radius carries rounding of order `ε r`; at this turn the corner's error is
/// under 1e-4 of `h` for `rκ` up to 1 (a sphere's is at most 1). Pointed noses' ten elements turn by
/// degrees and never merge.
const NEARLY_PARALLEL_RAD: f64 = 1e-6;

/// Enough halvings to find a blunt tip's handover to the last bit of an `f64`; the loop stops
/// sooner, when no `f64` lies between the ends.
const HANDOVER_BISECTIONS: usize = 1100;

/// The flow along one element: its state just behind its corner, and its tangent cone.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ElementFlow {
    corner_x_m: f64,
    angle_rad: f64,
    /// `p₂/p₀` just behind the corner.
    pressure: f64,
    /// `(∂p/∂s)₂`, `p₀` per m.
    gradient: f64,
    /// `Λ` just behind the corner.
    load: f64,
    /// `p_c/p₀` on the tangent cone.
    cone_pressure: f64,
    /// `(dC_N/dα)` of the tangent cone, per rad.
    cone_slope: f64,
}

impl ElementFlow {
    /// `dη/dx` from eq. 9: zero where the pressure already sits at its tangent cone's.
    fn eta_rate(&self) -> f64 {
        let gap = self.cone_pressure - self.pressure;
        if gap == 0.0 || self.gradient == 0.0 {
            0.0
        } else {
            self.gradient / (gap * self.angle_rad.cos())
        }
    }

    /// Whether the element is reduced to the generalized method (see [`Self::decay_rate`]).
    fn is_reduced(&self) -> bool {
        self.eta_rate() < 0.0
    }

    /// The rate `η` grows at along the element. The exponential form holds only where the
    /// gradient behind the corner has the sign of `p_c − p₂`, `η ≥ 0` (TN 3527 p. 13), which the
    /// report states as a condition of the method without saying how it continued where the
    /// condition fails. hpr's reading, not the report's rule: there it takes `η = 0`, where "all
    /// equations reduce to those given by the generalized shock-expansion method" (p. 13), so the
    /// pressure and loading stay at their values behind the corner and no gradient is passed to
    /// the next corner (the generalized method's constant pressure along an element, p. 5).
    /// Eq. 10 read literally would pass the gradient on; that diverges as elements are added.
    /// This happens on sharp noses at high Mach number, and it departs from the report's values
    /// on its fineness-3 ogive at Mach 5.05 (issue #81).
    fn decay_rate(&self) -> f64 {
        self.eta_rate().max(0.0)
    }

    /// `p/p₀` and `Λ` at `x_m` (eqs. 8, 9 and 19).
    fn at(&self, x_m: f64) -> (f64, f64) {
        let decay = (-self.decay_rate() * (x_m - self.corner_x_m)).exp();
        let pressure = self.cone_pressure - (self.cone_pressure - self.pressure) * decay;
        let load = (1.0 - decay) * self.angle_rad.tan() * self.cone_slope + decay * self.load;
        (pressure, load)
    }

    /// `∂p/∂s` where the pressure is `pressure` (eq. 10); zero on an element of the generalized
    /// method (see [`Self::decay_rate`]).
    fn gradient_at(&self, pressure: f64) -> f64 {
        let gap = self.cone_pressure - self.pressure;
        if gap == 0.0 || self.is_reduced() {
            0.0
        } else {
            (self.cone_pressure - pressure) / gap * self.gradient
        }
    }
}

/// `B = γpM²/(2(M² − 1))` (eq. 6), `p` in units of `p₀`.
fn b_factor(pressure: f64, mach: f64) -> f64 {
    GAMMA * pressure * mach * mach / (2.0 * (mach * mach - 1.0))
}

/// `λ = 2γp / sin 2μ` (eq. 5), with `sin 2μ = 2√(M² − 1)/M²`.
fn lambda(pressure: f64, mach: f64) -> f64 {
    let m2 = mach * mach;
    2.0 * GAMMA * pressure / (2.0 * (m2 - 1.0).sqrt() / m2)
}

/// `Ω = A/A* = (1/M)[(1 + (γ − 1)M²/2)/((γ + 1)/2)]^((γ + 1)/(2(γ − 1)))` (eq. 7).
fn area_ratio(mach: f64) -> f64 {
    ((1.0 + G1 * mach * mach) / (0.5 * (GAMMA + 1.0))).powf(0.5 * (GAMMA + 1.0) / (GAMMA - 1.0))
        / mach
}

/// `p_t/p = (1 + (γ − 1)M²/2)^(γ/(γ − 1))`.
fn total_over_static(mach: f64) -> f64 {
    (1.0 + G1 * mach * mach).powf(GAMMA / (GAMMA - 1.0))
}

/// The Mach number where isentropic flow of total pressure `total` has static pressure
/// `pressure` (both in units of `p₀`).
fn mach_from_pressure(total: f64, pressure: f64) -> Result<f64, AeroError> {
    let m2 = ((total / pressure).powf((GAMMA - 1.0) / GAMMA) - 1.0) / G1;
    if !(m2.is_finite() && m2 > 1.0) {
        return Err(AeroError::Unsupported(format!(
            "the surface flow isn't supersonic (Mach² {m2}) where the method needs it"
        )));
    }
    Ok(m2.sqrt())
}

/// The flow over a cone at zero angle of attack.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ConeFlow {
    /// The conical shock's angle to the axis, rad.
    pub shock_angle_rad: f64,
    /// The Mach number on the cone's surface.
    pub surface_mach: f64,
    /// The surface pressure over the free stream's, `p_c/p₀`.
    pub surface_pressure_ratio: f64,
}

/// The flow over a cone of half-angle `half_angle_rad` at Mach `mach` and zero angle of attack:
/// the Taylor–Maccoll equation (NACA Report 1135, 1953, eq. 177, p. 628) integrated from the
/// shock to the surface, the shock angle found so the surface falls on the cone. The weak,
/// attached solution.
///
/// In `V′ = V/V_max` with `V_θ = dV_r/dθ`:
/// `V_r″ = [V_θ² V_r − ((γ − 1)/2)(1 − V_r² − V_θ²)(2V_r + V_θ cot θ)] /
/// [((γ − 1)/2)(1 − V_r² − V_θ²) − V_θ²]`, started behind the oblique shock (eqs. 148 to 153,
/// p. 623) and stopped where `V_θ = 0`.
///
/// Below [`SLENDER_CONE_RAD`] (0.029°) the start behind so weak a shock is too near the
/// equation's singular line to integrate, and the flow is linearized slender-cone theory's:
/// `C_p = δ²(2 ln(2/(βδ)) − 1)`, `β = √(M² − 1)`, the shock on the Mach angle and the surface
/// Mach number isentropic from the free stream. From there to twice that angle the two are
/// blended linearly, so the flow is continuous in the half-angle. At a millidegree scale these
/// pressures differ from the free stream's by under 1e-5. Near Mach 1 (1.01) the integration can
/// still fail just above that angle; it then returns an error.
///
/// # Errors
///
/// - [`AeroError::Domain`] for a Mach number that isn't above 1 or a half-angle outside
///   `[0, π/2)`.
/// - [`AeroError::Unsupported`] where the shock detaches (the half-angle exceeds the steepest
///   cone an attached shock allows at this Mach number), or should the integration fail to
///   reach the cone.
pub fn cone_flow(mach: f64, half_angle_rad: f64) -> Result<ConeFlow, AeroError> {
    if !(mach.is_finite() && mach > 1.0) {
        return Err(AeroError::Domain {
            what: "Mach number of a cone's flow",
            value: mach,
        });
    }
    if !(half_angle_rad.is_finite() && (0.0..0.5 * PI).contains(&half_angle_rad)) {
        return Err(AeroError::Domain {
            what: "cone half-angle",
            value: half_angle_rad,
        });
    }
    if half_angle_rad <= SLENDER_CONE_RAD {
        return Ok(slender_cone_flow(mach, half_angle_rad));
    }
    let exact = taylor_maccoll_cone_flow(mach, half_angle_rad)?;
    if half_angle_rad >= 2.0 * SLENDER_CONE_RAD {
        return Ok(exact);
    }
    let slender = slender_cone_flow(mach, half_angle_rad);
    let w = half_angle_rad / SLENDER_CONE_RAD - 1.0;
    let blend = |a: f64, b: f64| (1.0 - w) * a + w * b;
    Ok(ConeFlow {
        shock_angle_rad: blend(slender.shock_angle_rad, exact.shock_angle_rad),
        surface_mach: blend(slender.surface_mach, exact.surface_mach),
        surface_pressure_ratio: blend(slender.surface_pressure_ratio, exact.surface_pressure_ratio),
    })
}

/// Below this half-angle, 5e-4 rad (0.029°), [`cone_flow`] takes slender-cone theory; up to twice
/// it, a blend.
pub const SLENDER_CONE_RAD: f64 = 5e-4;

/// Linearized slender-cone theory's flow (see [`cone_flow`]).
fn slender_cone_flow(mach: f64, half_angle_rad: f64) -> ConeFlow {
    let mach_angle = (1.0 / mach).asin();
    if half_angle_rad <= 0.0 {
        return ConeFlow {
            shock_angle_rad: mach_angle,
            surface_mach: mach,
            surface_pressure_ratio: 1.0,
        };
    }
    let beta = (mach * mach - 1.0).sqrt();
    let delta = half_angle_rad;
    let cp = delta * delta * (2.0 * (2.0 / (beta * delta)).ln() - 1.0);
    let pressure = 1.0 + 0.5 * GAMMA * mach * mach * cp;
    // Isentropic from the free stream: the shock's loss is of higher order still.
    let m2 = ((total_over_static(mach) / pressure).powf((GAMMA - 1.0) / GAMMA) - 1.0) / G1;
    ConeFlow {
        shock_angle_rad: mach_angle,
        surface_mach: m2.sqrt(),
        surface_pressure_ratio: pressure,
    }
}

/// The Taylor–Maccoll solution of [`cone_flow`], above [`SLENDER_CONE_RAD`].
fn taylor_maccoll_cone_flow(mach: f64, half_angle_rad: f64) -> Result<ConeFlow, AeroError> {
    let mach_angle = (1.0 / mach).asin();
    let not_converged = || {
        AeroError::Unsupported(format!(
            "the flow over a cone of half-angle {}° at Mach {mach} didn't converge",
            half_angle_rad.to_degrees()
        ))
    };
    let cone_at = |shock: f64| -> Result<f64, AeroError> {
        let angle = cone_behind_shock(mach, shock).0;
        if angle.is_finite() {
            Ok(angle)
        } else {
            Err(not_converged())
        }
    };
    // Bracket the shock angle: the cone angle grows with it from zero at the Mach angle up to the
    // detachment limit, then falls.
    let step = 0.5_f64.to_radians();
    let mut before = mach_angle;
    let mut lo = mach_angle;
    let mut lo_angle = 0.0;
    let mut hi = mach_angle;
    let mut hi_angle;
    loop {
        hi += step;
        if hi >= 0.5 * PI {
            return Err(detached(mach, half_angle_rad));
        }
        hi_angle = cone_at(hi)?;
        if hi_angle >= half_angle_rad {
            break;
        }
        if hi_angle < lo_angle {
            // Past the steepest cone: find it between the last two steps (golden section), in
            // case it reaches the half-angle between them.
            let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
            let (mut a, mut b) = (before, hi);
            for _ in 0..80 {
                let c = b - golden * (b - a);
                let d = a + golden * (b - a);
                if cone_at(c)? > cone_at(d)? {
                    b = d;
                } else {
                    a = c;
                }
            }
            let peak = 0.5 * (a + b);
            let peak_angle = cone_at(peak)?;
            if peak_angle < half_angle_rad {
                return Err(detached(mach, half_angle_rad));
            }
            // The cone angle rises from the bracket's low end to the peak: from `lo` when the
            // peak lies past it, from `before` when it lies between the two (the falling side
            // past the peak is the strong shock's).
            if peak <= lo {
                (lo, lo_angle) = (before, cone_at(before)?);
            }
            (hi, hi_angle) = (peak, peak_angle);
            break;
        }
        before = lo;
        (lo, lo_angle) = (hi, hi_angle);
    }
    // Regula falsi (Illinois) on the cone angle against the shock angle.
    let (mut f_lo, mut f_hi) = (lo_angle - half_angle_rad, hi_angle - half_angle_rad);
    let mut side = 0;
    let mut shock = hi;
    for _ in 0..100 {
        let next = if f_hi != f_lo {
            (lo * f_hi - hi * f_lo) / (f_hi - f_lo)
        } else {
            0.5 * (lo + hi)
        };
        let next = if next > lo && next < hi {
            next
        } else {
            0.5 * (lo + hi)
        };
        let settled = (next - shock).abs() <= 4.0 * f64::EPSILON * next;
        shock = next;
        if settled {
            break;
        }
        let f = cone_at(shock)? - half_angle_rad;
        if f == 0.0 {
            break;
        }
        if f > 0.0 {
            hi = shock;
            f_hi = f;
            if side == 1 {
                f_lo *= 0.5;
            }
            side = 1;
        } else {
            lo = shock;
            f_lo = f;
            if side == -1 {
                f_hi *= 0.5;
            }
            side = -1;
        }
    }
    let (angle, surface_speed) = cone_behind_shock(mach, shock);
    // The shock angle found must put the surface on the cone: near the slender limit to the
    // integration's own accuracy there (2e-4 of the angle at 0.029°), far inside the 45% and more
    // of a run that never reached the surface.
    let miss = (angle - half_angle_rad).abs();
    if !(miss.is_finite() && miss <= 1e-3 * half_angle_rad) {
        return Err(not_converged());
    }
    let surface_mach =
        (surface_speed * surface_speed / (G1 * (1.0 - surface_speed * surface_speed))).sqrt();
    let normal = mach * shock.sin();
    let total_ratio = normal_shock_total_pressure_ratio(normal);
    let flow = ConeFlow {
        shock_angle_rad: shock,
        surface_mach,
        surface_pressure_ratio: total_over_static(mach) * total_ratio
            / total_over_static(surface_mach),
    };
    if !(flow.surface_mach.is_finite() && flow.surface_pressure_ratio.is_finite()) {
        return Err(not_converged());
    }
    Ok(flow)
}

fn detached(mach: f64, half_angle_rad: f64) -> AeroError {
    AeroError::Unsupported(format!(
        "a cone of half-angle {}° at Mach {mach} has a detached shock",
        half_angle_rad.to_degrees()
    ))
}

/// `p_t2/p_t1` across a normal shock at normal Mach number `normal` (NACA Report 1135, eq. 99).
fn normal_shock_total_pressure_ratio(normal: f64) -> f64 {
    let m2 = normal * normal;
    ((GAMMA + 1.0) * m2 / ((GAMMA - 1.0) * m2 + 2.0)).powf(GAMMA / (GAMMA - 1.0))
        * ((GAMMA + 1.0) / (2.0 * GAMMA * m2 - (GAMMA - 1.0))).powf(1.0 / (GAMMA - 1.0))
}

/// The largest Taylor–Maccoll integration step in `θ`, rad.
const TM_STEP_RAD: f64 = 1e-3;

/// The most Taylor–Maccoll steps one shock angle may take before the integration gives up.
const TM_MAX_STEPS: usize = 1_000_000;

/// The Taylor–Maccoll step at `state`: at most [`TM_STEP_RAD`] and half the angle left, and small
/// enough that the equation's denominator `D = a′² − V_θ²` (where the flow normal to the rays is
/// sonic) changes by at most 2% of itself. Behind a weak shock (a slender cone) the flow starts
/// nearly sonic normal to the shock, `D` starts near zero and the solution turns sharply; a fixed
/// step there gives nonsense. The step is a continuous function of the state, not an error
/// estimate's accept-or-reject, so a last-bit difference between platforms moves the answer by
/// last bits too.
fn tm_step(theta: f64, [vr, vt]: [f64; 2]) -> f64 {
    let denominator = G1 * (1.0 - vr * vr - vt * vt) - vt * vt;
    let acceleration = taylor_maccoll(theta, [vr, vt])[1];
    // dD/dθ, with dV_r/dθ = V_θ.
    let rate = (2.0 * G1 * vr * vt + 2.0 * (1.0 + G1) * vt * acceleration).abs();
    let limit = if rate > 0.0 {
        0.02 * denominator.abs() / rate
    } else {
        TM_STEP_RAD
    };
    TM_STEP_RAD.min(0.5 * theta).min(limit)
}

/// For a conical shock at `shock_rad`, the cone angle where the flow behind it meets the surface
/// and the speed `V/V_max` there; NaN if the integration doesn't reach the surface.
fn cone_behind_shock(mach: f64, shock_rad: f64) -> (f64, f64) {
    let normal = mach * shock_rad.sin();
    if normal <= 1.0 {
        return (0.0, speed_ratio(mach));
    }
    let n2 = normal * normal;
    let normal_after = ((1.0 + G1 * n2) / (GAMMA * n2 - G1)).sqrt();
    // The flow deflection behind an oblique shock (NACA Report 1135, eq. 138).
    let deflection = (2.0 / shock_rad.tan() * (n2 - 1.0)
        / (mach * mach * (GAMMA + (2.0 * shock_rad).cos()) + 2.0))
        .atan();
    let mach_after = normal_after / (shock_rad - deflection).sin();
    let speed = speed_ratio(mach_after);
    let mut state = [
        speed * (shock_rad - deflection).cos(),
        -speed * (shock_rad - deflection).sin(),
    ];
    let mut theta = shock_rad;
    for _ in 0..TM_MAX_STEPS {
        let h = tm_step(theta, state);
        let next = rk4(theta, state, -h);
        if next[1] >= 0.0 {
            // The surface lies within this step: refine the step length by regula falsi.
            let (mut a, mut b) = (0.0, h);
            let (mut fa, mut fb) = (state[1], next[1]);
            let mut s = h;
            for _ in 0..60 {
                let trial = if fb != fa {
                    a - fa * (b - a) / (fb - fa)
                } else {
                    0.5 * (a + b)
                };
                let trial = if trial > a && trial < b {
                    trial
                } else {
                    0.5 * (a + b)
                };
                if (trial - s).abs() <= 4.0 * f64::EPSILON * theta {
                    s = trial;
                    break;
                }
                s = trial;
                let f = rk4(theta, state, -s)[1];
                if f == 0.0 {
                    break;
                }
                if f > 0.0 {
                    (b, fb) = (s, f);
                } else {
                    (a, fa) = (s, f);
                }
            }
            let surface = rk4(theta, state, -s);
            return (theta - s, surface[0]);
        }
        state = next;
        theta -= h;
        if theta <= 1e-9 {
            return (0.0, state[0]);
        }
    }
    (f64::NAN, f64::NAN)
}

/// `V/V_max = (2/((γ − 1)M²) + 1)^(−1/2)`.
fn speed_ratio(mach: f64) -> f64 {
    (2.0 / ((GAMMA - 1.0) * mach * mach) + 1.0).powf(-0.5)
}

/// The Taylor–Maccoll right-hand side: `d(V_r, V_θ)/dθ`.
fn taylor_maccoll(theta: f64, [vr, vt]: [f64; 2]) -> [f64; 2] {
    let b = G1 * (1.0 - vr * vr - vt * vt);
    [
        vt,
        (vt * vt * vr - b * (2.0 * vr + vt / theta.tan())) / (b - vt * vt),
    ]
}

/// One classical Runge–Kutta step of the Taylor–Maccoll equation.
fn rk4(theta: f64, y: [f64; 2], h: f64) -> [f64; 2] {
    let add = |y: [f64; 2], k: [f64; 2], s: f64| [y[0] + s * k[0], y[1] + s * k[1]];
    let k1 = taylor_maccoll(theta, y);
    let k2 = taylor_maccoll(theta + 0.5 * h, add(y, k1, 0.5 * h));
    let k3 = taylor_maccoll(theta + 0.5 * h, add(y, k2, 0.5 * h));
    let k4 = taylor_maccoll(theta + h, add(y, k3, h));
    [
        y[0] + h / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]),
        y[1] + h / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]),
    ]
}

/// The semivertex angles of [`CONE_SLOPES`], degrees.
const CONE_ANGLES_DEG: [f64; 19] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0,
    22.0, 24.0,
];

/// The Mach numbers of [`CONE_SLOPES`]' rows.
const CONE_MACHS: [f64; 6] = [3.0, 4.0, 5.0, 6.0, 8.0, 10.0];

/// `dC_N/dα` at `α = 0` for cones, per radian on the base area: TN 3527 Fig. 2 (p. 40, from its
/// ref. 14), read by hand at [`CONE_ANGLES_DEG`] for each of [`CONE_MACHS`] from a 400-dpi render
/// against the chart's 0.2° by 0.002 grid, to about ±0.001 (±0.0025 below 3°, where the Mach 8
/// and 10 curves merge; read as crossing, so the values stay ordered in Mach). Interpolated as
/// [`cone_normal_force_slope`] does, it gives all 12 of Table I's cone-alone values (4.1° to
/// 9.5°, Mach 3 to 6.28) to their printed two decimals.
const CONE_SLOPES: [[f64; 19]; 6] = [
    // Mach 3
    [
        2.000, 1.976, 1.953, 1.931, 1.911, 1.892, 1.874, 1.858, 1.843, 1.831, 1.820, 1.810, 1.799,
        1.776, 1.750, 1.721, 1.687, 1.648, 1.605,
    ],
    // Mach 4
    [
        2.000, 1.963, 1.935, 1.912, 1.893, 1.877, 1.865, 1.856, 1.849, 1.844, 1.838, 1.831, 1.823,
        1.805, 1.782, 1.753, 1.718, 1.678, 1.634,
    ],
    // Mach 5
    [
        2.000, 1.958, 1.927, 1.904, 1.885, 1.873, 1.865, 1.863, 1.863, 1.862, 1.859, 1.853, 1.847,
        1.828, 1.805, 1.775, 1.740, 1.699, 1.652,
    ],
    // Mach 6
    [
        2.000, 1.950, 1.917, 1.890, 1.878, 1.874, 1.874, 1.876, 1.879, 1.880, 1.877, 1.872, 1.865,
        1.847, 1.822, 1.790, 1.754, 1.713, 1.666,
    ],
    // Mach 8
    [
        2.000, 1.926, 1.891, 1.883, 1.884, 1.890, 1.899, 1.904, 1.907, 1.908, 1.905, 1.899, 1.891,
        1.870, 1.843, 1.811, 1.771, 1.727, 1.678,
    ],
    // Mach 10
    [
        2.000, 1.904, 1.885, 1.887, 1.897, 1.908, 1.916, 1.921, 1.924, 1.924, 1.921, 1.916, 1.907,
        1.884, 1.855, 1.819, 1.779, 1.734, 1.684,
    ],
];

/// A cone's normal-force slope at `α → 0`, per radian on its base area, from TN 3527 Fig. 2:
/// linear between the hand-read angles and Mach numbers; below Mach 3 the Mach 3 curve, above 10
/// the Mach 10 curve.
///
/// # Errors
///
/// - [`AeroError::Domain`] for a negative or non-finite half-angle, or a Mach number that isn't
///   finite and above 1.
/// - [`AeroError::Unsupported`] for a half-angle past Fig. 2's 24°.
pub fn cone_normal_force_slope(mach: f64, half_angle_rad: f64) -> Result<f64, AeroError> {
    let degrees = half_angle_rad.to_degrees();
    if !(degrees.is_finite() && degrees >= 0.0) {
        return Err(AeroError::Domain {
            what: "tangent-cone half-angle",
            value: half_angle_rad,
        });
    }
    let last = CONE_ANGLES_DEG[CONE_ANGLES_DEG.len() - 1];
    // A millionth of a degree over admits 24° itself through the degree conversion's rounding.
    if degrees > last + 1e-6 {
        return Err(AeroError::Unsupported(format!(
            "a tangent cone of {degrees}° is past TN 3527 Fig. 2's {last}°"
        )));
    }
    let degrees = degrees.min(last);
    if !(mach.is_finite() && mach > 1.0) {
        return Err(AeroError::Domain {
            what: "Mach number of a cone's normal-force slope",
            value: mach,
        });
    }
    let row = |i: usize| linear(&CONE_ANGLES_DEG, &CONE_SLOPES[i], degrees);
    let m = mach.clamp(CONE_MACHS[0], CONE_MACHS[CONE_MACHS.len() - 1]);
    let j = CONE_MACHS
        .partition_point(|&c| c <= m)
        .clamp(1, CONE_MACHS.len() - 1);
    let (m0, m1) = (CONE_MACHS[j - 1], CONE_MACHS[j]);
    let w = (m - m0) / (m1 - m0);
    Ok((1.0 - w) * row(j - 1) + w * row(j))
}

/// Linear interpolation in `xs` (increasing), `x` within `[xs[0], xs[n − 1]]`.
fn linear(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let i = xs.partition_point(|&c| c <= x).clamp(1, xs.len() - 1);
    let w = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
    (1.0 - w) * ys[i - 1] + w * ys[i]
}

/// The body's lift and its moment, summed in order over `windows`, where they place a centre of
/// pressure.
fn total_lift(windows: &[(f64, [f64; 2])]) -> Result<(f64, f64), AeroError> {
    let (mut force, mut moment) = (0.0, 0.0);
    for (_, [f, m]) in windows {
        force += f;
        moment += m;
    }
    if !(force.is_finite() && moment.is_finite() && force > 0.0) {
        return Err(AeroError::Unsupported(format!(
            "the body's lift sums to {force}, which places no centre of pressure"
        )));
    }
    Ok((force, moment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A body of unit diameter: a cone or tangent ogive of `fineness` calibers and a cylinder of
    /// `afterbody` calibers, as TN 3527 tested them.
    fn body(ogive: bool, fineness: f64, afterbody: f64, steps: usize) -> ShockExpansionBody {
        let shape = if ogive {
            NoseShape::TANGENT_OGIVE
        } else {
            NoseShape::Conical {}
        };
        let mut segments = vec![BodySegment::Profile {
            profile: Profile::nose(shape, fineness, 0.5).unwrap(),
        }];
        if afterbody > 0.0 {
            segments.push(BodySegment::Cylinder {
                length_m: afterbody,
                radius_m: 0.5,
            });
        }
        ShockExpansionBody::new(&segments, steps).unwrap()
    }

    #[test]
    #[allow(
        clippy::approx_constant,
        reason = "6.28 is one of TN 3527's test Mach numbers, not 2π"
    )]
    fn a_cone_alone_carries_its_fig_2_slope_at_two_thirds_its_length() {
        // On a cone the loading is uniform, `Λ = tan δ (dC_N/dα)_tc`, so eq. 14 returns Fig. 2's
        // slope and the CP sits at the centroid of `r`, two thirds of the length.
        for (fineness, mach) in [(3.0, 3.0), (5.0, 4.24), (7.0, 6.28)] {
            let cone = body(false, fineness, 0.0, 1);
            let s = cone.slope(mach, 0.25 * PI).unwrap();
            let expected = cone_normal_force_slope(mach, (0.5 / fineness).atan()).unwrap();
            assert!(
                (s.slope_per_rad - expected).abs() < 1e-12,
                "{fineness} {mach}"
            );
            assert!((s.centre_of_pressure_m - 2.0 / 3.0 * fineness).abs() < 1e-12);
        }
    }

    #[test]
    fn a_cylinder_adds_lift_that_grows_with_its_length() {
        let mut last = 0.0;
        for afterbody in [0.0, 2.0, 4.0, 6.0, 10.0] {
            let s = body(false, 5.0, afterbody, 1)
                .slope(4.24, 0.25 * PI)
                .unwrap();
            assert!(s.slope_per_rad > last, "{afterbody}: {}", s.slope_per_rad);
            last = s.slope_per_rad;
        }
    }

    #[test]
    fn a_boattail_takes_lift_off_by_footnote_8() {
        // Footnote 8's tangent cone for a boattail element: the free stream's pressure and a
        // slope of 2, so the loading relaxes toward a negative one.
        let body_with = |tail: bool| {
            let mut segments = vec![
                BodySegment::Profile {
                    profile: Profile::nose(NoseShape::TANGENT_OGIVE, 4.0, 0.5).unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: 8.0,
                    radius_m: 0.5,
                },
            ];
            if tail {
                segments.push(BodySegment::Profile {
                    profile: Profile::transition(NoseShape::Conical {}, 1.0, 0.5, 0.35, false)
                        .unwrap(),
                });
            }
            ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap()
        };
        for mach in [2.0, 3.0, 4.63] {
            let (bare, tailed) = (
                body_with(false).slope(mach, 0.25 * PI).unwrap(),
                body_with(true).slope(mach, 0.25 * PI).unwrap(),
            );
            assert!(tailed.slope_per_rad < bare.slope_per_rad, "Mach {mach}");
            assert!(
                tailed.centre_of_pressure_m < bare.centre_of_pressure_m,
                "Mach {mach}"
            );
        }
    }

    #[test]
    #[allow(
        clippy::approx_constant,
        reason = "6.28 is one of TN 3527's test Mach numbers, not 2π"
    )]
    fn segment_shares_sum_to_the_body_and_follow_its_segments() {
        // A tangent ogive of 4 calibers, a cylinder of 8 and a conical boattail of 1: the nose
        // and the cylinder carry lift, and footnote 8's boattail takes some off. The nose's share
        // is the nose alone's slope, and the cylinder's the difference the cylinder makes, which a
        // piece given to the wrong segment would break.
        let segments = [
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::TANGENT_OGIVE, 4.0, 0.5).unwrap(),
            },
            BodySegment::Cylinder {
                length_m: 8.0,
                radius_m: 0.5,
            },
            BodySegment::Profile {
                profile: Profile::transition(NoseShape::Conical {}, 1.0, 0.5, 0.35, false).unwrap(),
            },
        ];
        let body_of = |count: usize| {
            ShockExpansionBody::new(&segments[..count], DEFAULT_ELEMENTS_PER_CURVE).unwrap()
        };
        let (nose, forebody, body) = (body_of(1), body_of(2), body_of(3));
        let area = 0.25 * PI;
        for mach in [2.0, 3.0, 4.63, 6.28] {
            let whole = body.slope(mach, area).unwrap();
            let nose_alone = nose.slope(mach, area).unwrap().slope_per_rad;
            let with_cylinder = forebody.slope(mach, area).unwrap().slope_per_rad;
            let shares = body.segment_slopes(mach, area).unwrap();
            assert_eq!(shares.len(), 3);
            let slope: f64 = shares.iter().map(|s| s.slope_per_rad).sum();
            let moment: f64 = shares.iter().map(|s| s.moment_slope_m).sum();
            let cp = moment / slope;
            assert!(
                (slope - whole.slope_per_rad).abs() <= 1e-12 * whole.slope_per_rad,
                "Mach {mach}: {slope} against {}",
                whole.slope_per_rad
            );
            assert!(
                (cp - whole.centre_of_pressure_m).abs() <= 1e-12 * whole.centre_of_pressure_m,
                "Mach {mach}: {cp} against {}",
                whole.centre_of_pressure_m
            );
            assert!(
                (shares[0].slope_per_rad - nose_alone).abs() <= 1e-13 * nose_alone,
                "Mach {mach}: nose {} against {nose_alone}",
                shares[0].slope_per_rad
            );
            let cylinder = with_cylinder - nose_alone;
            assert!(
                (shares[1].slope_per_rad - cylinder).abs() <= 1e-12 * with_cylinder,
                "Mach {mach}: cylinder {} against {cylinder}",
                shares[1].slope_per_rad
            );
            // The nose's and the cylinder's loadings are positive, so their stations lie within
            // them; the boattail's share is negative here, but its station isn't bounded.
            let station = |s: &SegmentSlope| s.moment_slope_m / s.slope_per_rad;
            assert!(shares[0].slope_per_rad > 0.0 && shares[1].slope_per_rad > 0.0);
            assert!(shares[2].slope_per_rad < 0.0, "Mach {mach}");
            assert!((0.0..=4.0).contains(&station(&shares[0])), "Mach {mach}");
            assert!((4.0..=12.0).contains(&station(&shares[1])), "Mach {mach}");
        }
    }

    #[test]
    fn slender_cones_approach_linear_theory() {
        // As the cone thins, Taylor–Maccoll tends to linearized slender-cone theory,
        // `C_p = δ²(2 ln(2/(βδ)) − 1)`, `β = √(M² − 1)`, whose error is of higher order in δ:
        // within 3% of `C_p` at 0.5° and 1°.
        for mach in [1.5, 2.0, 3.0, 5.0] {
            for degrees in [0.5, 1.0] {
                let delta = f64::to_radians(degrees);
                let beta = f64::sqrt(mach * mach - 1.0);
                let linear = delta * delta * (2.0 * (2.0 / (beta * delta)).ln() - 1.0);
                let flow = cone_flow(mach, delta).unwrap();
                let cp = (flow.surface_pressure_ratio - 1.0) / (0.5 * GAMMA * mach * mach);
                assert!(
                    (cp / linear - 1.0).abs() < 0.03,
                    "Mach {mach}, {degrees}°: {cp} {linear}"
                );
            }
        }
    }

    #[test]
    #[allow(
        clippy::approx_constant,
        reason = "6.28 is one of TN 3527's test Mach numbers, not 2π"
    )]
    fn curved_elements_converge() {
        // DEFAULT_ELEMENTS_PER_CURVE's claim, on TN 3527's tangent ogives with long cylinders,
        // across its Mach numbers: fineness 3 at Mach 5.05 and 6.28 runs through elements of the
        // generalized method near the tip.
        for (fineness, mach) in [
            (3.0, 3.0),
            (3.0, 5.05),
            (3.0, 6.28),
            (5.0, 4.24),
            (7.0, 3.0),
        ] {
            let coarse = body(true, fineness, 10.0, DEFAULT_ELEMENTS_PER_CURVE);
            let fine = body(true, fineness, 10.0, 4 * DEFAULT_ELEMENTS_PER_CURVE);
            let (a, b) = (
                coarse.slope(mach, 0.25 * PI).unwrap(),
                fine.slope(mach, 0.25 * PI).unwrap(),
            );
            assert!(
                (a.slope_per_rad - b.slope_per_rad).abs() < 0.01,
                "{fineness} {mach}"
            );
            assert!((a.centre_of_pressure_m - b.centre_of_pressure_m).abs() < 0.01);
        }
    }

    #[test]
    fn reduced_elements_are_counted_where_issue_81_bites() {
        // The fineness-3 ogive at Mach 5.05, where hpr departs from TN 3527 (#81), reduces two of
        // its nose's elements near the tip; a cone's nose is one element, the tip's, and is never
        // reduced; the same ogive at Mach 3 has none either.
        let ogive = body(true, 3.0, 10.0, DEFAULT_ELEMENTS_PER_CURVE);
        assert_eq!(ogive.reduced_elements(5.05).unwrap(), 2);
        assert_eq!(ogive.reduced_elements(3.0).unwrap(), 0);
        assert_eq!(
            body(false, 3.0, 10.0, DEFAULT_ELEMENTS_PER_CURVE)
                .reduced_elements(5.05)
                .unwrap(),
            0
        );
        assert!(matches!(
            ogive.reduced_elements(1.0),
            Err(AeroError::Domain { .. })
        ));
    }

    #[test]
    fn cone_flow_agrees_with_naca_1135_charts() {
        // NACA Report 1135's cone charts, read by tracing the 300-dpi scan: Chart 5 (shock angle,
        // p. 660), Chart 6 (surface pressure coefficient, p. 662) and Chart 7 (surface Mach
        // number, p. 664), each to about ±0.15°, ±0.002 and ±0.007. The charts are drawn for
        // γ = 1.405, hpr uses 1.4, so the bounds are twice the reading uncertainty.
        for (mach, cone_deg, shock_deg, pressure_coefficient, surface_mach) in [
            (2.0, 10.0, 31.25, 0.1035, 1.838),
            (3.0, 20.0, 29.62, 0.283, 2.280),
            (1.5, 10.0, 42.68, 0.123, 1.378),
        ] {
            let flow = cone_flow(mach, f64::to_radians(cone_deg)).unwrap();
            let what = format!("Mach {mach}, {cone_deg}°");
            assert!(
                (flow.shock_angle_rad.to_degrees() - shock_deg).abs() < 0.3,
                "{what}"
            );
            let cp = (flow.surface_pressure_ratio - 1.0) / (0.5 * GAMMA * mach * mach);
            assert!((cp - pressure_coefficient).abs() < 0.004, "{what}: {cp}");
            assert!((flow.surface_mach - surface_mach).abs() < 0.015, "{what}");
        }
    }

    #[test]
    fn cone_flow_rises_smoothly_with_the_cone_angle() {
        // Slender cones start almost sonic normal to the shock, where the Taylor–Maccoll equation
        // is nearly singular; a fixed step once gave pressures that jumped and NaN there.
        // Below SLENDER_CONE_RAD slender-cone theory takes over, blended up to twice it.
        // Nearer Mach 1 the start is nearer still to the singular line, and just above
        // SLENDER_CONE_RAD the integration may refuse (at Mach 1.01, 0.029°): an error, not a
        // wrong answer.
        assert!(cone_flow(1.01, 1e-8).unwrap().surface_pressure_ratio - 1.0 < 1e-12);
        for mach in [1.2, 1.5, 1.97, 2.0, 3.0, 5.0, 7.0, 10.0] {
            let mut last = 1.0;
            let mut angles: Vec<f64> = (0..=54)
                .map(|i| 1e-6 * 10f64.powf(0.05 * f64::from(i)))
                .collect();
            angles.extend((1..=100).map(|i| f64::to_radians(0.05 * f64::from(i))));
            for angle in angles {
                let p = cone_flow(mach, angle).unwrap().surface_pressure_ratio;
                assert!(p.is_finite() && p > last, "Mach {mach}, {angle} rad: {p}");
                last = p;
            }
        }
    }

    #[test]
    fn the_slope_is_smooth_in_mach() {
        // Every tangent ogive's elements near the shoulder are cones of a degree or two.
        for fineness in [3.0, 5.0, 7.0] {
            let ogive = body(true, fineness, 4.0, DEFAULT_ELEMENTS_PER_CURVE);
            let mut last: Option<f64> = None;
            for i in 0..=300 {
                let mach = 3.0 + 0.01 * f64::from(i);
                let s = ogive.slope(mach, 0.25 * PI).unwrap().slope_per_rad;
                if let Some(last) = last {
                    assert!((s - last).abs() < 0.01, "fineness {fineness}, Mach {mach}");
                }
                last = Some(s);
                // A last-bit change in the Mach number moves the slope by last bits only.
                let nudged = ogive.slope(mach * (1.0 + f64::EPSILON), 0.25 * PI).unwrap();
                assert!(
                    (nudged.slope_per_rad - s).abs() < 1e-12,
                    "fineness {fineness}, Mach {mach}"
                );
            }
        }
    }

    #[test]
    fn cone_flow_keeps_the_weak_shock_up_to_detachment() {
        // The weak shock's angle rises with the cone's up to the steepest attached cone; the
        // strong shock's falls. Just under it the solver once returned the strong one.
        for mach in [1.4, 3.0, 7.4] {
            // The steepest attached cone, to 1e-7°.
            let (mut ok, mut bad) = (10.0_f64, 60.0_f64);
            while bad - ok > 1e-7 {
                let mid = 0.5 * (ok + bad);
                if cone_flow(mach, mid.to_radians()).is_ok() {
                    ok = mid;
                } else {
                    bad = mid;
                }
            }
            let mut last = 0.0;
            for below in [0.5, 0.1, 0.01, 0.003, 0.002, 0.001, 0.0003, 0.0001] {
                let flow = cone_flow(mach, (ok - below).to_radians()).unwrap();
                assert!(flow.shock_angle_rad > last, "Mach {mach}, {below}° under");
                last = flow.shock_angle_rad;
            }
        }
        // A weak-branch value just under the steepest cone at Mach 1.4 (the physics review's
        // independent solver: 69.180°, where the strong branch is 69.527°).
        let flow = cone_flow(1.4, 27.494_845_f64.to_radians()).unwrap();
        assert!((flow.shock_angle_rad.to_degrees() - 69.180).abs() < 0.01);
    }

    #[test]
    fn cone_flow_tends_to_the_free_stream() {
        let flow = cone_flow(2.5, 0.0).unwrap();
        assert_eq!(flow.surface_pressure_ratio, 1.0);
        assert_eq!(flow.surface_mach, 2.5);
        let thin = cone_flow(2.5, 0.2_f64.to_radians()).unwrap();
        assert!((thin.shock_angle_rad - (1.0 / 2.5_f64).asin()).abs() < 1e-3);
        assert!((thin.surface_pressure_ratio - 1.0).abs() < 1e-3);
        // Steeper cones compress the flow more.
        let (a, b) = (
            cone_flow(2.5, 10f64.to_radians()).unwrap(),
            cone_flow(2.5, 20f64.to_radians()).unwrap(),
        );
        assert!(
            b.surface_pressure_ratio > a.surface_pressure_ratio && b.surface_mach < a.surface_mach
        );
    }

    #[test]
    fn refuses_what_the_method_does_not_cover() {
        let area = 0.25 * PI;
        // A blunt tip steeper than the handover's slope all the way to its end: a power-series
        // nose one radius long (45° at its base).
        let stubby = BodySegment::Profile {
            profile: Profile::nose(NoseShape::PowerSeries { exponent: 0.5 }, 0.5, 0.5).unwrap(),
        };
        let stubby = ShockExpansionBody::new(&[stubby], 10).unwrap();
        assert!(stubby.has_blunt_tip());
        for mach in [1.5, 3.0, 5.0] {
            assert!(matches!(
                stubby.slope(mach, area),
                Err(AeroError::Unsupported(_))
            ));
        }
        // A spherical cap anywhere but first, or longer than a hemisphere.
        let cap = |length_m| BodySegment::SphericalCap {
            radius_m: 0.5,
            length_m,
        };
        let cylinder = BodySegment::Cylinder {
            length_m: 1.0,
            radius_m: 0.5,
        };
        assert!(matches!(
            ShockExpansionBody::new(&[cap(0.5), cylinder, cap(0.5)], 10),
            Err(AeroError::Unsupported(_))
        ));
        for length in [0.0, 0.6, f64::NAN] {
            assert!(matches!(
                ShockExpansionBody::new(&[cap(length), cylinder], 10),
                Err(AeroError::Domain { .. })
            ));
        }
        // A body that doesn't start with a nose.
        assert!(matches!(
            ShockExpansionBody::new(&[cylinder], 10),
            Err(AeroError::Unsupported(_))
        ));
        // A step in radius.
        let nose = BodySegment::Profile {
            profile: Profile::nose(NoseShape::Conical {}, 3.0, 0.5).unwrap(),
        };
        let wider = BodySegment::Cylinder {
            length_m: 1.0,
            radius_m: 0.6,
        };
        assert!(matches!(
            ShockExpansionBody::new(&[nose, wider], 10),
            Err(AeroError::Unsupported(_))
        ));
        // Subsonic, and a detached shock (a 40° cone at Mach 1.5).
        let cone = body(false, 3.0, 2.0, 1);
        assert!(matches!(
            cone.slope(0.9, area),
            Err(AeroError::Domain { .. })
        ));
        assert!(matches!(
            cone_flow(1.5, 40f64.to_radians()),
            Err(AeroError::Unsupported(_))
        ));
        // A cone steeper than Fig. 2's 24°, and angles Fig. 2 can't have.
        assert!(matches!(
            body(false, 1.0, 2.0, 1).slope(3.0, area),
            Err(AeroError::Unsupported(_))
        ));
        assert!(cone_normal_force_slope(3.0, 24f64.to_radians()).is_ok());
        for (mach, angle) in [(3.0, -0.1), (3.0, f64::NAN), (0.5, 0.1)] {
            assert!(matches!(
                cone_normal_force_slope(mach, angle),
                Err(AeroError::Domain { .. })
            ));
        }
        // Elements per curve outside 1 to 1000.
        let ogive = BodySegment::Profile {
            profile: Profile::nose(NoseShape::TANGENT_OGIVE, 3.0, 0.5).unwrap(),
        };
        for count in [0, MAX_ELEMENTS_PER_CURVE + 1, usize::MAX] {
            assert!(matches!(
                ShockExpansionBody::new(&[ogive], count),
                Err(AeroError::Domain { .. })
            ));
        }
        // A body that closes to a point and opens again.
        let closing = BodySegment::Profile {
            profile: Profile::transition(NoseShape::Conical {}, 3.0, 0.5, 0.0, false).unwrap(),
        };
        assert!(matches!(
            ShockExpansionBody::new(&[nose, closing, nose], 10),
            Err(AeroError::Unsupported(_))
        ));
    }

    /// M1.8e1 done-when: `cargo xtask aero` writes `validation/fixtures/aero/shock-expansion.json`;
    /// this test recomputes every hpr value from the committed tables (TN 3527's Tables I and
    /// II, `tn3527-bodies.json`) and the Arcas Robin's geometry, so the recorded errors can't go
    /// stale, and pins the set of rows outside the targets set before measuring: 0.05 per radian
    /// and 0.1 calibers of the report's second-order values, and its stated ±0.2 per radian and
    /// ±0.2 calibers of its measurements. The targets are not met; ADR-033 and
    /// `docs/physics/aero.md` record every miss.
    #[test]
    fn against_tn3527_and_the_arcas_robin() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/shock-expansion.json"
        ))
        .unwrap();
        let tables: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/tn3527-bodies.json"
        ))
        .unwrap();
        let targets = &fixture["targets"];
        let limits = [
            ("second_order", "c_n_alpha", 0.05),
            ("second_order", "cp_calibers", 0.1),
            ("experiment", "c_n_alpha", 0.2),
            ("experiment", "cp_calibers", 0.2),
        ];
        for (reference, quantity, limit) in limits {
            assert_eq!(targets[format!("{reference}_{quantity}")], limit);
        }
        assert_eq!(fixture["elements_per_curve"], DEFAULT_ELEMENTS_PER_CURVE);
        let rows = fixture["tn3527"].as_array().unwrap();
        let references = tables["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 144);
        assert_eq!(references.len(), 144);
        let close = |a: f64, b: f64, what: &str| {
            assert!(
                (a - b).abs() <= 1e-12 * b.abs().max(1.0),
                "{what}: {a} against {b}"
            );
        };
        let mut misses = Vec::new();
        for (row, reference) in rows.iter().zip(references) {
            let nose = reference["nose"].as_str().unwrap();
            let fineness = reference["fineness"].as_f64().unwrap();
            let mach = reference["mach"].as_f64().unwrap();
            let afterbody = reference["afterbody_calibers"].as_f64().unwrap();
            let what = format!("{nose}-{fineness}-{afterbody}@{mach}");
            assert_eq!(row["nose"], nose, "{what}");
            let hpr = body(
                nose == "ogive",
                fineness,
                afterbody,
                DEFAULT_ELEMENTS_PER_CURVE,
            )
            .slope(mach, 0.25 * PI);
            let hpr = match (hpr, row["refused"].as_str()) {
                (Ok(hpr), None) => hpr,
                (Err(e), Some(refused)) => {
                    assert_eq!(e.to_string(), refused, "{what}");
                    misses.push(format!("{what} refused"));
                    continue;
                }
                (hpr, refused) => panic!("{what}: {hpr:?} where the fixture has {refused:?}"),
            };
            close(
                row["hpr"]["c_n_alpha"].as_f64().unwrap(),
                hpr.slope_per_rad,
                &what,
            );
            close(
                row["hpr"]["cp_calibers"].as_f64().unwrap(),
                hpr.centre_of_pressure_m,
                &what,
            );
            for (reference_name, quantity, limit) in limits {
                let (value, source) = match quantity {
                    "c_n_alpha" => (hpr.slope_per_rad, &reference["c_n_alpha"]),
                    _ => (hpr.centre_of_pressure_m, &reference["cp"]),
                };
                let compared = &row[reference_name][quantity];
                let Some(r) = source[reference_name].as_f64() else {
                    assert!(compared.is_null(), "{what}");
                    continue;
                };
                close(compared["reference"].as_f64().unwrap(), r, &what);
                close(compared["error"].as_f64().unwrap(), value - r, &what);
                let within = (value - r).abs() <= limit + 1e-12;
                assert_eq!(compared["within"], within, "{what}");
                if !within {
                    misses.push(format!("{what} {reference_name} {quantity}"));
                }
            }
        }
        // Every miss, in the tables' order (docs/physics/aero.md, ADR-033). Against the report's
        // own values: inside the method's limit, hpr and a separate implementation of the same
        // equations agree within 0.001 per radian where the printed values depart (the
        // fineness-7 cone on long cylinders high, the ogives low); at the limit, the fineness-3
        // ogive at Mach 5.05 and 6.28, hpr's reduction departs from the report (issue #81).
        // Against its measurements: the same fineness-7 cones, three rows where the report is
        // itself 0.20 to 0.22 off, the fineness-3 ogive at Mach 5.05 (#81), and one at -0.206.
        let pinned = [
            "cone-7-6@3 experiment cp_calibers",
            "cone-7-8@3 second_order c_n_alpha",
            "cone-7-10@3 second_order c_n_alpha",
            "cone-7-6@4.24 second_order c_n_alpha",
            "cone-7-8@4.24 second_order c_n_alpha",
            "cone-7-8@4.24 second_order cp_calibers",
            "cone-7-10@4.24 second_order c_n_alpha",
            "cone-7-10@4.24 second_order cp_calibers",
            "cone-7-10@4.24 experiment c_n_alpha",
            "cone-7-10@4.24 experiment cp_calibers",
            "cone-7-4@5.05 second_order c_n_alpha",
            "cone-7-6@5.05 second_order c_n_alpha",
            "cone-7-6@5.05 second_order cp_calibers",
            "cone-7-8@5.05 second_order c_n_alpha",
            "cone-7-8@5.05 second_order cp_calibers",
            "cone-7-10@5.05 second_order c_n_alpha",
            "cone-7-10@5.05 second_order cp_calibers",
            "cone-7-10@5.05 experiment cp_calibers",
            "cone-7-4@6.28 second_order c_n_alpha",
            "cone-7-6@6.28 second_order c_n_alpha",
            "cone-7-6@6.28 second_order cp_calibers",
            "cone-7-8@6.28 second_order c_n_alpha",
            "cone-7-8@6.28 second_order cp_calibers",
            "cone-7-10@6.28 second_order c_n_alpha",
            "cone-7-10@6.28 second_order cp_calibers",
            "cone-7-10@6.28 experiment c_n_alpha",
            "cone-7-10@6.28 experiment cp_calibers",
            "cone-5-10@6.28 second_order c_n_alpha",
            "cone-5-10@6.28 experiment cp_calibers",
            "cone-3-10@6.28 experiment cp_calibers",
            "ogive-7-4@3 second_order cp_calibers",
            "ogive-7-8@3 second_order c_n_alpha",
            "ogive-7-2@4.24 second_order c_n_alpha",
            "ogive-7-2@5.05 second_order c_n_alpha",
            "ogive-7-4@5.05 second_order c_n_alpha",
            "ogive-7-4@5.05 experiment cp_calibers",
            "ogive-7-6@5.05 second_order c_n_alpha",
            "ogive-7-8@5.05 second_order c_n_alpha",
            "ogive-7-2@6.28 second_order c_n_alpha",
            "ogive-7-10@6.28 second_order cp_calibers",
            "ogive-5-2@3 second_order c_n_alpha",
            "ogive-5-4@3 second_order c_n_alpha",
            "ogive-5-4@3 second_order cp_calibers",
            "ogive-5-6@3 second_order c_n_alpha",
            "ogive-5-6@3 second_order cp_calibers",
            "ogive-5-8@3 second_order c_n_alpha",
            "ogive-5-8@3 second_order cp_calibers",
            "ogive-5-10@3 second_order c_n_alpha",
            "ogive-5-10@3 second_order cp_calibers",
            "ogive-5-4@4.24 second_order c_n_alpha",
            "ogive-5-6@4.24 second_order c_n_alpha",
            "ogive-5-10@5.05 experiment cp_calibers",
            "ogive-5-2@6.28 second_order c_n_alpha",
            "ogive-5-4@6.28 second_order c_n_alpha",
            "ogive-3-2@4.24 second_order c_n_alpha",
            "ogive-3-4@4.24 second_order c_n_alpha",
            "ogive-3-6@4.24 second_order c_n_alpha",
            "ogive-3-8@4.24 second_order c_n_alpha",
            "ogive-3-10@4.24 second_order c_n_alpha",
            "ogive-3-2@5.05 second_order c_n_alpha",
            "ogive-3-2@5.05 second_order cp_calibers",
            "ogive-3-4@5.05 second_order c_n_alpha",
            "ogive-3-4@5.05 second_order cp_calibers",
            "ogive-3-4@5.05 experiment cp_calibers",
            "ogive-3-6@5.05 second_order c_n_alpha",
            "ogive-3-6@5.05 second_order cp_calibers",
            "ogive-3-6@5.05 experiment cp_calibers",
            "ogive-3-8@5.05 second_order c_n_alpha",
            "ogive-3-8@5.05 second_order cp_calibers",
            "ogive-3-10@5.05 second_order c_n_alpha",
            "ogive-3-10@5.05 second_order cp_calibers",
            "ogive-3-10@5.05 experiment c_n_alpha",
            "ogive-3-10@5.05 experiment cp_calibers",
            "ogive-3-2@6.28 second_order c_n_alpha",
            "ogive-3-4@6.28 second_order c_n_alpha",
        ];
        assert_eq!(misses, pinned);

        // The Arcas Robin: hpr's values rebuilt from the recorded nose and the report's
        // dimensions; the measured slopes are M1.8a's fits of the committed points.
        let tunnel: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/arcas-robin-wind-tunnel.json"
        ))
        .unwrap();
        let arcas = &fixture["arcas_robin"];
        let ratio = arcas["nose"]["radius_ratio"].as_f64().unwrap();
        let inch = 0.0254;
        let (radius, nose_length) = (1.125 * inch, 9.375 * inch);
        let area = PI * radius * radius;
        let mut rows = 0;
        for configuration in arcas["configurations"].as_array().unwrap() {
            let id = configuration["id"].as_str().unwrap();
            let end = tunnel["geometry"]["cylinder_ends_in"][id].as_f64().unwrap();
            assert_eq!(configuration["cylinder_ends_in"], end);
            let mut segments = vec![
                BodySegment::Profile {
                    profile: Profile::nose(
                        NoseShape::Ogive {
                            radius_ratio: ratio,
                        },
                        nose_length,
                        radius,
                    )
                    .unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: end * inch - nose_length,
                    radius_m: radius,
                },
            ];
            let bare = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
            segments.push(BodySegment::Profile {
                profile: Profile::transition(
                    NoseShape::Conical {},
                    1.757 * inch,
                    radius,
                    0.5 * 1.308 * inch,
                    false,
                )
                .unwrap(),
            });
            let tailed = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
            let curves = tunnel["configurations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == id)
                .unwrap()["cn_alpha_fins_off"]
                .as_array()
                .unwrap();
            for row in configuration["rows"].as_array().unwrap() {
                let mach = row["mach"].as_f64().unwrap();
                let what = format!("{id} at Mach {mach}");
                let curve = curves.iter().find(|c| c["mach"] == mach).unwrap();
                let points = curve["alpha_deg_c_n"].as_array().unwrap();
                let alphas: Vec<f64> = points
                    .iter()
                    .map(|p| p[0].as_f64().unwrap().to_radians())
                    .collect();
                let c_n: Vec<f64> = points.iter().map(|p| p[1].as_f64().unwrap()).collect();
                let n = alphas.len() as f64;
                let (ma, mc) = (alphas.iter().sum::<f64>() / n, c_n.iter().sum::<f64>() / n);
                let sxy: f64 = alphas
                    .iter()
                    .zip(&c_n)
                    .map(|(a, c)| (a - ma) * (c - mc))
                    .sum();
                let sxx: f64 = alphas.iter().map(|a| (a - ma) * (a - ma)).sum();
                let measured = sxy / sxx;
                close(row["measured_c_n_alpha"].as_f64().unwrap(), measured, &what);
                for (key, body) in [("nose_and_cylinder", &bare), ("with_boattail", &tailed)] {
                    let s = body.slope(mach, area).unwrap();
                    let entry = &row[key];
                    close(entry["c_n_alpha"].as_f64().unwrap(), s.slope_per_rad, &what);
                    close(
                        entry["cp_calibers"].as_f64().unwrap(),
                        s.centre_of_pressure_m / (2.0 * radius),
                        &what,
                    );
                    close(
                        entry["c_n_alpha_error"].as_f64().unwrap(),
                        s.slope_per_rad / measured - 1.0,
                        &what,
                    );
                }
                rows += 1;
            }
        }
        // Mach 1.5 to 4.63 on the short model, 1.8 to 4.63 on the long.
        assert_eq!(rows, 11);
    }

    /// A hemisphere on a cylinder, reference its cross-section.
    fn hemisphere_cylinder() -> ShockExpansionBody {
        ShockExpansionBody::new(
            &[
                BodySegment::SphericalCap {
                    radius_m: 0.5,
                    length_m: 0.5,
                },
                BodySegment::Cylinder {
                    length_m: 4.0,
                    radius_m: 0.5,
                },
            ],
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .unwrap()
    }

    /// The handover sits where a sphere's slope is the handover's, `x = R(1 − sin δ)`, found to
    /// the last few bits of an `f64`. The tip's half-angle is 90°.
    #[test]
    fn a_blunt_tip_hands_over_where_its_slope_falls_to_the_wedges() {
        let sphere = hemisphere_cylinder();
        assert!(sphere.has_blunt_tip());
        assert_eq!(sphere.vertex_angle_rad(), 0.5 * PI);
        for mach in [1.3, 1.5, 2.0, 3.0, 5.0] {
            let delta = crate::blunt_tip::handover_angle_rad(mach).unwrap();
            let want = 0.5 * (1.0 - delta.sin());
            let got = sphere.handover_m(mach).unwrap().unwrap();
            assert!(
                (got - want).abs() <= 1e-14,
                "Mach {mach}: {got} against {want}"
            );
        }
        // A pointed nose has none.
        let pointed = body(false, 3.0, 2.0, DEFAULT_ELEMENTS_PER_CURVE);
        assert!(!pointed.has_blunt_tip());
        assert_eq!(pointed.handover_m(2.0).unwrap(), None);
        assert!(matches!(
            sphere.handover_m(1.0),
            Err(AeroError::Domain { .. })
        ));
    }

    /// The cap's Newtonian loading integrates to its closed form: on a sphere of radius `R`, with
    /// `θ` from the pole, `C_Nα = 2 C_p,max ∫ cos θ sin³θ dθ = C_p,max sin⁴θ_h / 2` on `πR²` to
    /// the handover's `θ_h = 90° − δ_h`.
    #[test]
    fn the_cap_carries_its_newtonian_loading() {
        let body = hemisphere_cylinder();
        let area = 0.25 * PI;
        for mach in [1.5, 3.0] {
            let windows = body.windows(mach, area).unwrap();
            // The first window runs from the pole to the handover.
            let (start, [force, _]) = windows[0];
            assert_eq!(start, 0.0);
            let cap = 2.0 * PI * force / area;
            let theta = 0.5 * PI - crate::blunt_tip::handover_angle_rad(mach).unwrap();
            let c_p_max = crate::blunt_tip::newtonian_pressure_coefficient_max(mach).unwrap();
            let want = 0.5 * c_p_max * theta.sin().powi(4);
            assert!(
                (cap - want).abs() <= 1e-9 * want,
                "Mach {mach}: {cap} against {want}"
            );
            // The rest of the body carries lift too, and the whole places a centre of pressure
            // on the body.
            let slope = body.slope(mach, area).unwrap();
            assert!(slope.slope_per_rad > cap);
            assert!(slope.centre_of_pressure_m > 0.0 && slope.centre_of_pressure_m < 4.5);
        }
    }

    /// The Arcas Robin's committed nose (a power series, `n` = 0.6369, 9.375 in long on a
    /// 2.25-in body) and the short model's cylinder: four times the default elements move its
    /// slope by under 0.01 per radian and its centre of pressure by under 0.01 calibers, through
    /// Mach 5.
    #[test]
    fn a_vertical_tip_converges_as_elements_are_added() {
        let radius = 1.125 * 0.0254;
        let area = PI * radius * radius;
        let body = |steps| {
            ShockExpansionBody::new(
                &[
                    BodySegment::Profile {
                        profile: Profile::nose(
                            NoseShape::PowerSeries { exponent: 0.6369 },
                            9.375 * 0.0254,
                            radius,
                        )
                        .unwrap(),
                    },
                    BodySegment::Cylinder {
                        length_m: (39.14 - 9.375) * 0.0254,
                        radius_m: radius,
                    },
                ],
                steps,
            )
            .unwrap()
        };
        let (coarse, fine) = (body(DEFAULT_ELEMENTS_PER_CURVE), body(40));
        for mach in [1.5, 2.3, 2.96, 3.96, 4.63, 5.0] {
            let a = coarse.slope(mach, area).unwrap();
            let b = fine.slope(mach, area).unwrap();
            assert!(
                (a.slope_per_rad - b.slope_per_rad).abs() < 0.01,
                "Mach {mach}: {a:?} against {b:?}"
            );
            assert!(
                (a.centre_of_pressure_m - b.centre_of_pressure_m).abs() < 0.01 * 2.0 * radius,
                "Mach {mach}: {a:?} against {b:?}"
            );
        }
    }

    /// TN D-4865's own start behind the cap, kept to compare (ADR-038): on the Arcas Robin's
    /// committed nose its march fails from Mach 3.96, where the tangent cone's start holds; on a
    /// pointed body the choice changes nothing. Its JSON form is snake case.
    #[test]
    fn the_reports_own_start_is_kept_to_compare() {
        let radius = 1.125 * 0.0254;
        let area = PI * radius * radius;
        let nose = BodySegment::Profile {
            profile: Profile::nose(
                NoseShape::PowerSeries { exponent: 0.6369 },
                9.375 * 0.0254,
                radius,
            )
            .unwrap(),
        };
        let cylinder = BodySegment::Cylinder {
            length_m: 0.75,
            radius_m: radius,
        };
        let body = ShockExpansionBody::new(&[nose, cylinder], DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        let reports = body.clone().with_handover_start(HandoverStart::Newtonian);
        assert!(body.slope(3.96, area).is_ok());
        assert!(matches!(
            reports.slope(3.96, area),
            Err(AeroError::Unsupported(_))
        ));
        let (a, b) = (
            body.slope(2.3, area).unwrap(),
            reports.slope(2.3, area).unwrap(),
        );
        assert!((a.slope_per_rad - b.slope_per_rad).abs() > 0.1);
        let pointed = body_of_cone();
        assert_eq!(
            pointed.slope(3.0, area).unwrap(),
            pointed
                .clone()
                .with_handover_start(HandoverStart::Newtonian)
                .slope(3.0, area)
                .unwrap()
        );
        assert_eq!(
            serde_json::to_string(&HandoverStart::TangentCone).unwrap(),
            "\"tangent_cone\""
        );
        let back: HandoverStart = serde_json::from_str("\"newtonian\"").unwrap();
        assert_eq!(back, HandoverStart::Newtonian);
    }

    fn body_of_cone() -> ShockExpansionBody {
        body(false, 3.0, 2.0, DEFAULT_ELEMENTS_PER_CURVE)
    }

    /// Just above the Mach number where a blunt tip's handover first falls on its nose (its
    /// slope at the nose's end), the method holds, and just below it doesn't, at every offset
    /// from 1e-15 to 1e-3: the nose's elements, packed into nanometres there, merge rather than
    /// meet at corners lost in rounding. TN D-4865's sphere-cone and two power-series noses.
    #[test]
    fn a_blunt_tip_holds_from_where_its_handover_first_falls_on_the_nose() {
        let (radius, half_angle) = (0.175_f64, 11.5_f64.to_radians());
        let tangent_r = radius * half_angle.cos();
        let sphere_cone = [
            BodySegment::SphericalCap {
                radius_m: radius,
                length_m: radius * (1.0 - half_angle.sin()),
            },
            BodySegment::Profile {
                profile: Profile::transition(
                    NoseShape::Conical {},
                    (0.5 - tangent_r) / half_angle.tan(),
                    tangent_r,
                    0.5,
                    false,
                )
                .unwrap(),
            },
        ];
        let power = |exponent: f64, length_m: f64| {
            [
                BodySegment::Profile {
                    profile: Profile::nose(NoseShape::PowerSeries { exponent }, length_m, 0.5)
                        .unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: 4.0,
                    radius_m: 0.5,
                },
            ]
        };
        for segments in [sphere_cone, power(0.6369, 4.17), power(0.5, 2.28)] {
            let body = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
            let first = segments[0];
            let end_angle = first.radius_and_slope(first.length_m()).1.atan();
            let (mut low, mut high) = (1.0 + 1e-9, 3.0);
            for _ in 0..200 {
                let mid = 0.5 * (low + high);
                if crate::blunt_tip::handover_angle_rad(mid).unwrap() < end_angle {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            for k in 0..=12 {
                for step in [1.0, 2.0, 5.0] {
                    let d = step * 10f64.powi(-15 + k);
                    assert!(
                        body.slope(high + d, 0.25 * PI).is_ok(),
                        "{segments:?}: fails {d} above Mach {high}"
                    );
                    assert!(
                        body.slope(high - d, 0.25 * PI).is_err(),
                        "{segments:?}: holds {d} below Mach {high}"
                    );
                }
            }
        }
    }
}
