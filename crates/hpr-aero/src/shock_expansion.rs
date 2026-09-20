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
    /// Whether the element is tangent to one of the nose's segments ([`nose_segments`]): the first
    /// one, and behind a spherical cap the curved segments that carry the nose on past it.
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
    /// The cap on the handover slope, rad ([`crate::blunt_tip::MAX_HANDOVER_RAD`] as flown).
    handover_cap_rad: f64,
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
    /// The total pressure the march expands from, over the free stream's static pressure: what
    /// turns a surface pressure back into a surface Mach number ([`mach_from_pressure`]).
    total: f64,
}

/// The flow on one element of the tangent body ([`ShockExpansionBody::element_flows`]): its state
/// just behind the element's corner, the tangent cone it relaxes toward, and how fast it does so.
/// At an axial distance `x` aft of its corner the pressure is `p_c − (p_c − p₂) e^(−η)` and the
/// loading `(1 − e^(−η)) Λ_c + e^(−η) Λ₂`, with `η = `[`Self::decay_per_m`]` · x` (TN 3527 eqs. 8,
/// 9 and 19). Pressures are over the free stream's, `p₀`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementFlowReport {
    /// Where the element starts, m aft of the vertex.
    pub corner_x_m: f64,
    /// Its angle to the axis, rad; negative on a boattail.
    pub angle_rad: f64,
    /// `p₂/p₀`, the pressure just behind its corner.
    pub pressure_ratio: f64,
    /// `Λ₂`, the loading just behind its corner, per radian of angle of attack.
    pub loading_per_rad: f64,
    /// `p_c/p₀` on its tangent cone (the free stream's for a cylinder, and footnote 8's for a
    /// boattail).
    pub tangent_cone_pressure_ratio: f64,
    /// `Λ_c = tan δ (dC_N/dα)_tc`, the loading it relaxes toward, per radian of angle of attack.
    pub tangent_cone_loading_per_rad: f64,
    /// `dη/dx`, per m of axial distance aft of the corner, not of distance along the surface;
    /// zero where the pressure already sits at its tangent cone's, or where the element is
    /// reduced ([issue #81: the gradient a reduced element carries
    /// on](https://github.com/nrdptel/hpr-sim/issues/81)).
    pub decay_per_m: f64,
    /// The radius at its corner, m: the `r` of eq. 19's `∫ Λ r dx`.
    pub corner_radius_m: f64,
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

/// The surface flow the march delivers to a body's aft end
/// ([`ShockExpansionBody::aft_flow`]): what a corner behind that body — the juncture of a flare,
/// say — turns.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AftFlow {
    /// The Mach number on the surface at the aft end, from the pressure the last element's decay
    /// has reached there (TN 3527 eq. 8) expanded back through the march's total pressure.
    pub surface_mach: f64,
    /// The surface's angle to the axis there, rad; zero on a cylinder, negative on a boattail.
    pub angle_rad: f64,
    /// `p₁/p₀`, the surface pressure there over the free stream's.
    pub pressure_ratio: f64,
    /// `(∂p/∂s)₁`, the pressure gradient the last element carries to there, in units of the free
    /// stream's pressure per m of **axial** distance, not of distance along the surface
    /// (TN 3527 eq. 10). Positive where the pressure is still climbing.
    pub gradient_p0_per_m: f64,
    /// The free-stream Mach number the march was run at, so that a corner behind this flow can be
    /// read without being told it again ([`flare_reduction_turns_rad`]).
    pub free_stream_mach: f64,
    /// The body's radius there, m: eq. 4's `r` at a corner behind it. This is the **profile's**
    /// radius at the aft end, so where the tangent body's last corner is not at the aft end — a
    /// body that ends in a curve, or one whose last tangency point was merged as nearly parallel
    /// — it sits a little off the element's own corner radius, as [`Self::angle_rad`] does. On a
    /// body that ends in a cylinder or a cone, which is every body a flare joins in a flight
    /// today, the two are the same.
    pub radius_m: f64,
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
    /// one a single element. Behind a blunt tip the steps are counted from the handover aft, laid
    /// out at each Mach number ([`crate::blunt_tip`]); the segments its cap covers get none.
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
            handover_cap_rad: crate::blunt_tip::MAX_HANDOVER_RAD,
        })
    }

    /// This body with its march behind a blunt tip's cap starting from `start`; a pointed body
    /// is unchanged.
    #[must_use]
    pub fn with_handover_start(mut self, start: HandoverStart) -> Self {
        self.handover_start = start;
        self
    }

    /// This body with its blunt tip handing over no steeper than `cap_rad` instead of the flown
    /// [`crate::blunt_tip::MAX_HANDOVER_RAD`]; a pointed body is unchanged. A steeper cap follows
    /// TN D-4865's own rule to a higher Mach number and starts the march from a steeper cone;
    /// what each is worth is measured in [ADR-043][adr-043].
    ///
    /// The cap is checked when the handover is taken ([`Self::handover_m`]), which refuses one
    /// outside `(0, `[`crate::blunt_tip::CONE_TABLE_CAP_RAD`]`]`. A pointed body has no handover,
    /// so it ignores the cap and never reports a bad one.
    ///
    /// ```
    /// use hpr_aero::blunt_tip::CONE_TABLE_CAP_RAD;
    /// use hpr_aero::shock_expansion::{BodySegment, DEFAULT_ELEMENTS_PER_CURVE, ShockExpansionBody};
    /// use hpr_design::{NoseShape, Profile};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let nose = BodySegment::Profile {
    ///     profile: Profile::nose(NoseShape::PowerSeries { exponent: 0.5 }, 0.5, 0.05)?,
    /// };
    /// let body = ShockExpansionBody::new(&[nose], DEFAULT_ELEMENTS_PER_CURVE)?;
    /// // At Mach 3 the wedge detaches past either cap, so each hands over at its own slope, and
    /// // the steeper one leaves the shorter cap. This nose is r = R√(x/L), whose slope is
    /// // R/(2√(xL)), so the handover sits at x = (R/(2 tan δ))²/L: 6.31 mm of nose at 24°,
    /// // 3.75 mm at 30°.
    /// let station = |degrees: f64| (0.05 / (2.0 * degrees.to_radians().tan())).powi(2) / 0.5;
    /// let flown = body.handover_m(3.0)?.expect("a vertical tip hands over");
    /// let steeper = body
    ///     .clone()
    ///     .with_handover_cap_rad(CONE_TABLE_CAP_RAD)
    ///     .handover_m(3.0)?
    ///     .expect("a vertical tip hands over");
    /// assert!((flown - station(24.0)).abs() < 1e-9 && (flown - 0.006_31).abs() < 5e-6);
    /// assert!((steeper - station(30.0)).abs() < 1e-9 && (steeper - 0.003_75).abs() < 5e-6);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [adr-043]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-043-the-blunt-tips-handover-cap-what-it-is-worth-and-what-stops-it-moving-2026-09-20
    #[must_use]
    pub fn with_handover_cap_rad(mut self, cap_rad: f64) -> Self {
        self.handover_cap_rad = cap_rad;
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
    /// where the body's slope first falls to [`crate::blunt_tip::handover_angle_rad`], or to the
    /// body's own cap ([`Self::with_handover_cap_rad`]). `None` for a pointed tip.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a Mach number that isn't finite and above 1, or a handover cap
    ///   outside `(0, `[`crate::blunt_tip::CONE_TABLE_CAP_RAD`]`]`.
    /// - [`AeroError::Unsupported`] if the body is steeper than the handover's slope all the way
    ///   to its end.
    pub fn handover_m(&self, mach: f64) -> Result<Option<f64>, AeroError> {
        check_mach(mach)?;
        if !self.blunt {
            return Ok(None);
        }
        let angle = crate::blunt_tip::handover_angle_capped_rad(mach, self.handover_cap_rad)?;
        let target = angle.tan();
        // The cap ends where the nose's slope first falls to the handover's, which needn't be in
        // the first segment: behind a spherical cap the nose can carry on through another curved
        // segment ([`nose_segments`]). Take the first of the nose's segments that is shallower
        // than the handover at its aft end, and bisect inside it — the slope falls from infinite
        // at the tip, so the segments ahead of that one are steeper all through. The search stops
        // where the nose does: a cap that reached a cylinder would hand over at no angle at all,
        // with none of the total pressure the tip took out of the flow.
        for (start, segment) in &self.segments[..nose_segments(&self.segments)] {
            let length = segment.length_m();
            if segment.radius_and_slope(length).1 > target {
                continue;
            }
            // Bisect to the last bit of an `f64` for the first station where the slope is at most
            // the handover's.
            let (mut low, mut high) = (0.0_f64, length);
            for _ in 0..HANDOVER_BISECTIONS {
                let mid = 0.5 * (low + high);
                if mid <= low || mid >= high {
                    break;
                }
                if segment.radius_and_slope(mid).1 > target {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            return Ok(Some(start + high));
        }
        Err(AeroError::Unsupported(format!(
            "the nose is steeper than the blunt tip's handover slope, {}°, all the way to its \
             end at Mach {mach}",
            angle.to_degrees()
        )))
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
    ///   detaches, a tangent cone steeper than the cone tables' 30°, a corner the flow can't turn
    ///   supersonically, a tip cone whose surface flow is subsonic, a cylinder's or a boattail's
    ///   element whose pressure moves away from the one it relaxes toward (the free stream's and
    ///   footnote 8's; neither is a tangent cone of that element's own flow, so the reduction of
    ///   [`flare_reduction_turns_rad`] is not read there), or a lift that doesn't sum to a
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

    /// The flow the method computes on each element of the tangent body at Mach `mach`, in order
    /// from the vertex or a blunt tip's handover: what a hand calculation of eq. 19,
    /// `C_Nα = (2π/A_ref) ∫ Λ r dx`, needs. Behind a blunt tip the list starts at the handover
    /// ([`Self::handover_m`]), and the lift of the Newtonian cap ahead of it is not in the list.
    ///
    /// # Errors
    ///
    /// As [`Self::slope`], less the check that the lift sums to a positive force.
    pub fn element_flows(&self, mach: f64) -> Result<Vec<ElementFlowReport>, AeroError> {
        Ok(self
            .flows(mach)?
            .flows
            .iter()
            .map(|flow| ElementFlowReport {
                corner_x_m: flow.corner_x_m,
                angle_rad: flow.angle_rad,
                pressure_ratio: flow.pressure,
                loading_per_rad: flow.load,
                tangent_cone_pressure_ratio: flow.cone_pressure,
                tangent_cone_loading_per_rad: flow.angle_rad.tan() * flow.cone_slope,
                decay_per_m: flow.decay_rate(),
                corner_radius_m: flow.corner_radius_m,
            })
            .collect())
    }

    /// The surface flow the march delivers to the body's aft end at Mach `mach`: everything a
    /// corner behind the body needs — the surface Mach number and angle there, the pressure, the
    /// gradient the last element carries to it, the radius, and the free stream's Mach number.
    ///
    /// The angle is the **last element's**, so on a body that ends in a curve it is that element's
    /// chord rather than the tangent at the very end, and it moves a little with
    /// `elements_per_curve`. On a body that ends in a cylinder or a cone — every body a flare
    /// joins in a flight today — the two are the same.
    ///
    /// This is the flow a corner *behind* the body turns. The march is downstream-only — TN 3527
    /// eq. 3 fixes each element from the one ahead of it and nothing behind — so a flare added at
    /// the aft end cannot change it, and the limit on that flare's corner
    /// ([`flare_corner_limit_rad`]) can be read from this body before the flare is drawn.
    ///
    /// # Errors
    ///
    /// As [`Self::slope`], less the check that the lift sums to a positive force, and
    /// [`AeroError::Unsupported`] where the surface flow at the aft end is not supersonic.
    pub fn aft_flow(&self, mach: f64) -> Result<AftFlow, AeroError> {
        check_mach(mach)?;
        let march = self.flows(mach)?;
        // `flows` starts with the vertex's or the handover's element, so it is never empty.
        let last = march.flows[march.flows.len() - 1];
        let (pressure, _) = last.at(self.length_m);
        Ok(AftFlow {
            surface_mach: mach_from_pressure(march.total, pressure)?,
            angle_rad: last.angle_rad,
            pressure_ratio: pressure,
            gradient_p0_per_m: last.gradient_at(pressure),
            free_stream_mach: mach,
            // `new` refuses a body with no segments, so there is always a last one.
            radius_m: self.segments[self.segments.len() - 1].1.aft_radius_m(),
        })
    }

    /// How many of the body's elements the march reduces to the generalized method at Mach
    /// `mach`: those where the gradient behind the corner points away from the tangent cone's
    /// pressure (`η < 0`, TN 3527 p. 13), which carry no gradient on (see
    /// [issue #81](https://github.com/nrdptel/hpr-sim/issues/81)). Zero means the result doesn't
    /// depend on that reading.
    ///
    /// Which turns a corner reduces is a property of its own state, and
    /// [`flare_reduction_turns_rad`] solves for the two that bound them.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a Mach number that isn't finite and above 1.
    /// - [`AeroError::Unsupported`] where the march fails, as for [`Self::slope`]: a detached tip
    ///   shock, a tangent cone past the cone tables' 30°, a corner the flow can't turn, subsonic surface
    ///   flow, or a cylinder's or boattail's element that would be reduced. An `Ok` count doesn't
    ///   promise that [`Self::slope`] succeeds: it also needs a positive total lift.
    pub fn reduced_elements(&self, mach: f64) -> Result<usize, AeroError> {
        check_mach(mach)?;
        Ok(self
            .flows(mach)?
            .flows
            .iter()
            .filter(|f| f.is_reduced())
            .count())
    }

    /// How many times the marched surface pressure crosses its own tangent cone's at Mach `mach`:
    /// the number of elements whose gap `p_c − p₂` has the opposite sign to the last element that
    /// had one, counting only pairs within one segment of the body. A pair that straddles a
    /// segment's start does not count: there `p_c` itself steps — from a cone's pressure to the
    /// free stream's where a nose meets a cylinder, to footnote 8's where a boattail begins, or
    /// to a steeper cone's where a flare does — so the gap changes sign without ever passing
    /// through zero, and there is no pole. Within a segment the profile is continuous, so `p_c`
    /// is too, and a sign change means the gap really closed.
    ///
    /// **A crossing is what marks an answer that moves with the element count.** Along an element
    /// the method
    /// relaxes the pressure and the loading toward the tangent cone's as `e^(−η)` with
    /// `η = k (x − x₂)` (eqs. 8, 9 and 19), where the rate per unit length is
    /// `k = (∂p/∂s)₂ / ((p_c − p₂) cos δ₂)`. Where the pressure crosses its tangent cone's the gap
    /// passes through zero while the gradient does not, so `k` has a pole. The pressure itself
    /// rides through it — `k (p_c − p) cos δ₂` is just the gradient, which stays finite — but the
    /// loading borrows the pressure's `k` (eq. 19) while its own gap `Λ_c − Λ` does not close
    /// with it, so the loading is driven onto the tangent cone's arbitrarily fast. A march applies
    /// `k` from the corner over a whole element, so how much of that lands depends on where the
    /// crossing falls between corners, and the answer follows the element count instead of
    /// settling.
    ///
    /// **It is a flag, not a verdict, at either end.** A count of zero does not promise an answer
    /// settled: whether a crossing is seen depends on the mesh, and the count is not even
    /// monotone in it — readings that cross at 40 and 160 elements per curve can show none at 10.
    /// Nor does a count above zero promise the answer never settles: one reading of hpr's own
    /// sweep crosses at every mesh and still holds to 0.003 per radian from 60 elements on. What
    /// is measured is that over 10, 40 and 160 elements the crossings, and only the crossings,
    /// mark the readings that move
    /// ([ADR-044](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md)). Nothing in a
    /// flight calls this: it is a tool for studying a body, not a guard.
    ///
    /// The fineness-3 ogive TN 3527 prints values for never crosses at the Mach numbers hpr can
    /// check it at; there `η < 0` comes from the gradient changing sign with the gap all one way,
    /// which is bounded and settles. So this count, not [`Self::reduced_elements`], is the one to
    /// read when an answer moves with the element count; see
    /// [issue #108](https://github.com/nrdptel/hpr-sim/issues/108).
    ///
    /// An element whose gap is exactly zero is skipped rather than given a sign: behind a blunt
    /// tip the march starts on its own tangent cone under the default
    /// [`HandoverStart::TangentCone`], and that element has no side to be on. Under
    /// [`HandoverStart::Newtonian`] it does, because its pressure and its tangent cone's come
    /// from different models, and the count then includes that mismatch.
    ///
    /// # Errors
    ///
    /// As [`Self::reduced_elements`].
    pub fn tangent_cone_crossings(&self, mach: f64) -> Result<usize, AeroError> {
        check_mach(mach)?;
        // Where `p_c` may step: the start of every segment after the first.
        let starts: Vec<f64> = self
            .segments
            .iter()
            .skip(1)
            .map(|(start, _)| *start)
            .collect();
        let within_one_segment = |from: f64, to: f64| {
            !starts
                .iter()
                .any(|start| *start > from && *start <= to + 1e-12 * self.length_m)
        };
        let mut crossings = 0;
        // The last element that had a gap: where it starts, and which side of its cone it is on.
        let mut last: Option<(f64, bool)> = None;
        for flow in &self.flows(mach)?.flows {
            let gap = flow.cone_pressure - flow.pressure;
            if gap == 0.0 {
                continue;
            }
            if let Some((at_m, was_positive)) = last
                && was_positive != (gap > 0.0)
                && within_one_segment(at_m, flow.corner_x_m)
            {
                crossings += 1;
            }
            last = Some((flow.corner_x_m, gap > 0.0));
        }
        Ok(crossings)
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
        let March { cap, flows, .. } = self.flows(mach)?;
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
                corner_radius_m: 0.0,
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
                corner_radius_m: element.corner_radius_m,
                angle_rad: element.angle_rad,
                pressure: p2,
                gradient: gradient2,
                load: load2,
                cone_pressure,
                cone_slope,
            };
            // Where the element has no tangent cone of its own — a cylinder's is the free
            // stream, a boattail's is footnote 8's — there is nothing for the reduction to
            // relax toward, and a reduced element would carry its corner's loading over any
            // length. Nothing measures what that is worth, so hpr refuses those (issue #123).
            if flow.is_reduced() && element.angle_rad <= CONE_ANGLE_FLOOR_RAD {
                return Err(AeroError::Unsupported(format!(
                    "behind the corner at {} m, where the element has no tangent cone of its \
                     own, the pressure moves away from the one it relaxes toward",
                    element.corner_x_m
                )));
            }
            flows.push(flow);
        }
        Ok(March { cap, flows, total })
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
            corner_radius_m: handover.corner_radius_m,
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

/// How many leading segments are the nose.
///
/// Normally one: a nose is a single [`Profile`](hpr_design::Profile), and everything behind it is
/// the afterbody. A [`BodySegment::SphericalCap`] is the exception — it is a *piece* of a nose,
/// never a whole one — so behind a cap the nose carries on through the curved segments that
/// follow it, and stops at the first that is straight or doesn't widen. TN D-4865's own model 2
/// needs that: its nose is
/// a 0.257-diameter sphere blended into a 2.75° cone by a 0.429 arc, and the sphere is still at
/// 38.3° where the arc takes over, steeper than the handover's 24° cap at any Mach number
/// (M1.8e18, ADR-048).
///
/// It says where a blunt tip's cap may hand the flow over ([`ShockExpansionBody::handover_m`]),
/// and which elements the rule on a reduced element treats as the nose's. Deliberately narrow: a
/// body that isn't led by a cap reads exactly as it did before, so no committed number moved.
///
/// [adr-048]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-048-what-a-marched-flare-is-worth-measured-against-tn-d-4865s-model-2-2026-09-20
fn nose_segments(segments: &[(f64, BodySegment)]) -> usize {
    if !matches!(
        segments.first(),
        Some((_, BodySegment::SphericalCap { .. }))
    ) {
        return 1;
    }
    segments
        .iter()
        .take_while(|(_, segment)| {
            // A curved segment that narrows is a boattail, not the nose: letting a cap reach one
            // would hand the flow over on a falling surface, at a slope the handover angle meets
            // from the wrong side.
            !segment.is_straight()
                && segment.radius_and_slope(segment.length_m()).0 > segment.radius_and_slope(0.0).0
        })
        .count()
        .max(1)
}

/// The tangent body's elements over `segments` (each with its fore station, the body `length_m`
/// long): tangent at `elements_per_curve` equal steps along a curved segment (a straight one
/// takes one element), counted from `start_m` — the vertex, or a blunt tip's handover, whose cap
/// may cover whole segments. The first element starts at `start_m`.
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
    let nose_segments = nose_segments(segments);
    // The tangency points: (x, r, slope, on the nose).
    let mut points = Vec::new();
    for (index, (start, segment)) in segments.iter().enumerate() {
        // A blunt tip's cap can cover whole segments: those carry no elements, and the one the
        // handover falls in starts there.
        if start + segment.length_m() <= start_m {
            continue;
        }
        let from = (start_m - start).max(0.0);
        let span = segment.length_m() - from;
        let steps = if segment.is_straight() {
            1
        } else {
            elements_per_curve
        };
        let count = if segment.is_straight() { 1 } else { steps + 1 };
        for i in 0..count {
            let local = from + span * i as f64 / steps as f64;
            let (r, slope) = segment.radius_and_slope(local);
            points.push((start + local, r, slope, index < nose_segments));
        }
    }

    let Some(&(x0, r0, t0, _)) = points.first() else {
        return Err(AeroError::Unsupported(format!(
            "a blunt tip's cap reaches the body's end at {start_m} m"
        )));
    };
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
/// degrees and never merge. Which points merge behind a blunt tip's cap changes with the handover,
/// so a body's slope steps by about 1e-6 per radian as they do.
const NEARLY_PARALLEL_RAD: f64 = 1e-6;

/// Enough halvings to find a blunt tip's handover to the last bit of an `f64`; the loop stops
/// sooner, when no `f64` lies between the ends.
const HANDOVER_BISECTIONS: usize = 1100;

/// The flow along one element: its state just behind its corner, and its tangent cone.
#[derive(Debug, Clone, Copy, PartialEq)]
struct ElementFlow {
    corner_x_m: f64,
    /// The radius at the corner, m.
    corner_radius_m: f64,
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

/// The steepest turn the method reads at a flare's corner, rad, where the surface flow reaching
/// that corner is `surface_mach` ([`ShockExpansionBody::aft_flow`]): the largest deflection
/// behind an attached plane oblique shock.
///
/// This is a bound on the **turn**, measured from the surface just ahead of the corner. The cone
/// tables' [`crate::blunt_tip::CONE_TABLE_CAP_RAD`] bounds the flare's **surface angle** instead,
/// since that is what an element's tangent cone is looked up by, and a caller that draws a flare
/// to this turn must cap the angle it draws separately.
///
/// **The attachment test.** A flare's shock springs from a circular corner, not from a point, so
/// where it forms the flow is two-dimensional: the body's radius is the scale over which the
/// axisymmetric relief acts, and at the corner itself there is none of it yet. The test is
/// therefore NACA Report 1135's largest deflection behind an attached plane oblique shock
/// ([`crate::blunt_tip::wedge_detachment_angle_rad`], eq. 168 into eq. 138), read at the flow
/// reaching the corner rather than at the free stream — the same test TN D-4865 p. 5 uses to hand
/// a blunt tip's cap over to this method ([`crate::blunt_tip::handover_angle_rad`]). A cone's
/// shock holds to steeper angles than a wedge's and a conical flare on a cylinder sits between
/// the two ([ADR-045][adr-045]), so this is the conservative
/// side of the boundary: it stops reading some flares whose shock is in fact still attached, and
/// never marches one whose shock is not.
///
/// **Where it is read matters.** The march is downstream-only, so `surface_mach` is the flow the
/// body ahead delivers to the corner, not the free stream: on a flare behind an ogive nose and a
/// tube it comes out a little below the free stream, on one behind a cone and a tube a little
/// above ([ADR-047][adr-047]).
///
/// # Errors
///
/// As [`crate::blunt_tip::wedge_detachment_angle_rad`] for the Mach number.
///
/// [adr-045]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-045-where-a-flares-march-stops-is-the-corners-isentropic-turn-not-the-shock-detaching-2026-09-20
/// [adr-047]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-047-a-flare-flies-the-method-where-its-corners-shock-is-attached-and-is-read-drawn-out-where-it-is-not-2026-09-20
pub fn flare_corner_limit_rad(surface_mach: f64) -> Result<f64, AeroError> {
    crate::blunt_tip::wedge_detachment_angle_rad(surface_mach)
}

/// The two turns that bound where the second-order method's exponential form does not hold at a
/// corner behind a body ([`flare_reduction_turns_rad`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReductionTurns {
    /// The turn whose pressure just behind the corner lands exactly on its tangent cone's,
    /// `p₂ = p_c`. `η` has a pole here, because eq. 9 divides by that gap.
    pub crossing_rad: f64,
    /// What the crossing's solution left behind: `p₂ − p_c` there, in units of the free stream's
    /// pressure. **Read it before trusting the turn.** How small it can be made is the tangent
    /// cone's accuracy, not the solver's: below [`SLENDER_CONE_RAD`] the cone flow is
    /// slender-cone theory's closed form and this closes to the last bits of an `f64`, while
    /// above it the cone flow is an integration and what is left is that integration's own.
    pub crossing_residual_p0: f64,
    /// The turn whose own compression exactly cancels the pressure gradient the body ahead
    /// delivers to the corner, `(∂p/∂s)₂ = 0` (TN 3527 eq. 4). `η` is zero here, so the method is
    /// already the generalized one.
    pub balance_rad: f64,
    /// What the balance's solution left behind: `(∂p/∂s)₂` there, `p₀` per m of axial distance.
    pub balance_residual_p0_per_m: f64,
}

/// Where the second-order shock-expansion method's exponential form fails at a corner behind
/// `aft`, the flow a body delivers to its aft end ([`ShockExpansionBody::aft_flow`]): the two
/// turns between which the march reduces the element behind that corner to the generalized
/// method.
///
/// **An element is reduced exactly when its turn lies strictly between the two**, in whichever
/// order they come. Eq. 9's rate is `η/(x − x₂) = (∂p/∂s)₂ / ((p_c − p₂) cos δ₂)`, and TN 3527
/// p. 13 keeps the exponential form only where `η ≥ 0`, so a reduced element is one whose
/// gradient behind the corner and whose gap to its tangent cone have opposite signs. Each of
/// those two is a continuous function of the turn, and — on every corner state measured for
/// [ADR-050][adr-050], an observation rather than a proof — each has a single zero, so the signs
/// disagree on exactly the open interval between them and nowhere else.
///
/// Both zeros are properties of the corner's own state. With `δ₁` the angle ahead, `r` the radius
/// at the corner, `B = γpM²/(2(M² − 1))` (eq. 6) and `Ω = A/A*` (eq. 7), all read from `aft`:
///
/// - the balance solves `sin(δ₁ + θ) = (Ω₁/Ω₂(θ)) (sin δ₁ + r (∂p/∂s)₁ / B₁)`, which is eq. 4 set
///   to zero and rearranged. `Ω₁/Ω₂` is `1 + O(θ)`, so iterating on it contracts;
/// - the crossing solves `p₂(θ) = p_c(δ₁ + θ)`, the isentropic turn's pressure against its
///   tangent cone's ([`cone_flow`]), by false position from the turn that would bring `p₂` back
///   to the free stream's pressure, `θ ≈ (1/p₁ − 1)√(M₁² − 1)/(γM₁²)`.
///
/// Neither is a search over the march's own refusal, which is a sign test on two pressures within
/// a thousandth of each other and so carries about nine significant digits
/// ([issue #117](https://github.com/nrdptel/hpr-sim/issues/117)). What is left is the accuracy of
/// `aft` and of the tangent cone, and each solution reports what it left behind —
/// [`ReductionTurns::crossing_residual_p0`] and
/// [`ReductionTurns::balance_residual_p0_per_m`] — because neither is promised to be zero.
/// Differentiating the crossing's equation, a change `Δp₁` in the pressure the body delivers
/// moves the crossing by about `Δp₁ √(M₁² − 1) / (γ p₁ M₁²)`.
///
/// # Errors
///
/// - [`AeroError::Unsupported`] where the corner's state can't carry a turn — a surface that
///   isn't supersonic, no radius, a free-stream Mach number at or below 1 — or where either root
///   lies outside the turns a widening corner can make: between zero surface angle and the
///   shallower of the isentropic turn's end and the cone tables' 30°.
///
/// [adr-050]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-050-a-reduced-element-is-read-by-the-generalized-method-wherever-it-has-a-tangent-cone-of-its-own-2026-09-20
pub fn flare_reduction_turns_rad(aft: &AftFlow) -> Result<ReductionTurns, AeroError> {
    let (m1, p1, d1) = (aft.surface_mach, aft.pressure_ratio, aft.angle_rad);
    let mach = aft.free_stream_mach;
    if !(m1 > 1.0
        && mach > 1.0
        && p1 > 0.0
        && aft.radius_m > 0.0
        && aft.gradient_p0_per_m.is_finite()
        && d1.abs() < 0.5 * PI)
    {
        return Err(AeroError::Unsupported(format!(
            "a corner behind a surface at Mach {m1} in a Mach {mach} stream, {p1} of the free \
             stream's pressure and {} m of radius, turns nothing",
            aft.radius_m
        )));
    }
    let nu1 = prandtl_meyer(m1);
    let total = p1 * total_over_static(m1);
    // The surface Mach number and pressure just behind a corner turning the flow by `turn`,
    // compressing where that is positive. A turn of nothing is the state itself: round-tripping
    // it through the isentropic relations would leave a bit of noise where the answer is exact.
    let behind = |turn: f64| -> Option<(f64, f64)> {
        if turn == 0.0 {
            return Some((m1, p1));
        }
        let nu2 = nu1 - turn;
        (nu2 > 0.0 && nu2 < MAX_TURNING_RAD).then(|| {
            let m2 = inverse_prandtl_meyer(nu2);
            (m2, total / total_over_static(m2))
        })
    };
    // The turns a widening corner can make: from a surface lying along the axis up to, but not
    // including, the shallower of the isentropic turn running out and the cone tables' cap.
    let (lowest, highest) = (
        -d1,
        (crate::blunt_tip::CONE_TABLE_CAP_RAD - d1)
            .min(nu1)
            .next_down(),
    );
    if lowest >= highest || !highest.is_finite() {
        return Err(AeroError::Unsupported(format!(
            "a corner behind a surface at Mach {m1} lying {}° to the axis has no widening turn \
             the method holds",
            d1.to_degrees()
        )));
    }
    let outside = |what: &str| {
        AeroError::Unsupported(format!(
            "the {what} of a corner behind a surface at Mach {m1} in a Mach {mach} stream lies \
             outside the turns a widening corner can make"
        ))
    };

    // The balance: eq. 4 set to zero. `k` is the corner's own state, and the only turn left in
    // the equation is through the `Ω₂` that turn reaches.
    let (o1, b1) = (area_ratio(m1), b_factor(p1, m1));
    let k = d1.sin() + aft.radius_m * aft.gradient_p0_per_m / b1;
    if !(-1.0..=1.0).contains(&k) {
        return Err(outside("balance"));
    }
    let mut balance = k.asin() - d1;
    for _ in 0..REDUCTION_ITERATIONS {
        // A fixed point that wants to sit outside the widening turns has no root among them:
        // say so rather than iterate against an end and hand that back as one.
        if !(lowest..=highest).contains(&balance) {
            return Err(outside("balance"));
        }
        let (m2, _) = behind(balance).ok_or_else(|| outside("balance"))?;
        let next = (o1 / area_ratio(m2) * k).asin() - d1;
        if !next.is_finite() {
            return Err(outside("balance"));
        }
        let step = next - balance;
        balance = next;
        if step.abs() <= f64::EPSILON * (1.0 + balance.abs()) {
            break;
        }
    }
    if !(lowest..=highest).contains(&balance) {
        return Err(outside("balance"));
    }
    let (m2, p2) = behind(balance).ok_or_else(|| outside("balance"))?;
    let (o2, b2) = (area_ratio(m2), b_factor(p2, m2));
    let balance_left = b2 / aft.radius_m * (o1 / o2 * d1.sin() - (d1 + balance).sin())
        + b2 * o1 / (b1 * o2) * aft.gradient_p0_per_m;

    // The crossing: the isentropic turn's pressure against its tangent cone's. Swept over the
    // widening turns first, because the gap is not promised to have one zero — a blunt shoulder
    // at high Mach has three, and a bracket taken on the ends alone would hide two of them and
    // return whichever root the solver happened to walk to. Where the sweep finds exactly one
    // sign change, false position inside that bracket lands on the root and stays there.
    let gap = |turn: f64| -> Option<f64> {
        let (_, p2) = behind(turn)?;
        let angle = d1 + turn;
        if !(0.0..crate::blunt_tip::CONE_TABLE_CAP_RAD).contains(&angle) {
            return None;
        }
        let cone = if angle <= CONE_ANGLE_FLOOR_RAD {
            1.0
        } else {
            cone_flow(mach, angle).ok()?.surface_pressure_ratio
        };
        Some(p2 - cone)
    };
    let start = gap(lowest).ok_or_else(|| outside("crossing"))?;
    // A body that has handed the free stream's own pressure to the corner meets its tangent
    // cone's at a turn of nothing, which is the first end rather than a station.
    let mut bracket = (start == 0.0).then_some(((lowest, start), (lowest, start)));
    let mut crossings = usize::from(start == 0.0);
    let mut before = (lowest, start);
    for station in 1..=REDUCTION_STATIONS {
        // The last station is the end itself: stepping to it can round a hair past it, and a
        // hair past is where the isentropic turn has run out.
        let turn = if station == REDUCTION_STATIONS {
            highest
        } else {
            lowest + (highest - lowest) * station as f64 / REDUCTION_STATIONS as f64
        };
        let here = (turn, gap(turn).ok_or_else(|| outside("crossing"))?);
        if here.1 == 0.0 || before.1.signum() != here.1.signum() {
            crossings += 1;
            bracket = Some((before, here));
        }
        before = here;
    }
    if crossings != 1 {
        return Err(AeroError::Unsupported(format!(
            "the pressure behind a corner behind a surface at Mach {m1} in a Mach {mach} stream \
             meets its tangent cone's {crossings} times over the turns a widening corner can \
             make, so the element it reduces is not one band of turns"
        )));
    }
    // `crossings == 1` put a bracket there.
    let ((mut low, mut at_low), (mut high, mut at_high)) =
        bracket.ok_or_else(|| outside("crossing"))?;
    let mut crossing = if at_low == 0.0 {
        low
    } else if at_high == 0.0 {
        high
    } else {
        // Start from the linearized guess where it falls inside the bracket, the midpoint where
        // it does not.
        let beta1 = (m1 * m1 - 1.0).sqrt();
        let guess = (1.0 / p1 - 1.0) * beta1 / (GAMMA * m1 * m1);
        if guess > low && guess < high {
            guess
        } else {
            0.5 * (low + high)
        }
    };
    let mut here = gap(crossing).ok_or_else(|| outside("crossing"))?;
    for _ in 0..REDUCTION_ITERATIONS {
        if here == 0.0 {
            break;
        }
        if here.signum() == at_low.signum() {
            (low, at_low) = (crossing, here);
            at_high *= 0.5;
        } else {
            (high, at_high) = (crossing, here);
            at_low *= 0.5;
        }
        let next = (low * at_high - high * at_low) / (at_high - at_low);
        let next = if next.is_finite() && next > low && next < high {
            next
        } else {
            0.5 * (low + high)
        };
        let moved = next - crossing;
        crossing = next;
        here = gap(crossing).ok_or_else(|| outside("crossing"))?;
        if moved.abs() <= f64::EPSILON * (1.0 + crossing.abs()) {
            break;
        }
    }
    Ok(ReductionTurns {
        crossing_rad: crossing,
        crossing_residual_p0: here,
        balance_rad: balance,
        balance_residual_p0_per_m: balance_left,
    })
}

/// How many stations [`flare_reduction_turns_rad`] sweeps the widening turns at before bracketing
/// the crossing. A pair of extra roots closer together than a hundredth of that range would not
/// be seen, and the gap would be reported as one band when it is three.
const REDUCTION_STATIONS: usize = 100;

/// Enough steps for either solution of [`flare_reduction_turns_rad`] to stop moving; both take
/// well under twenty, and the loops break when they do.
const REDUCTION_ITERATIONS: usize = 64;

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
const CONE_ANGLES_DEG: [f64; 22] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0,
    22.0, 24.0, 25.0, 27.5, 30.0,
];

/// The Mach numbers of [`CONE_SLOPES`]' rows.
const CONE_MACHS: [f64; 6] = [3.0, 4.0, 5.0, 6.0, 8.0, 10.0];

/// `dC_N/dα` at `α = 0` for cones, per radian on the base area, from two sources.
///
/// To 24°, TN 3527 Fig. 2 (p. 40, from its ref. 14), read by hand at [`CONE_ANGLES_DEG`] for each
/// of [`CONE_MACHS`] from a 400-dpi render against the chart's 0.2° by 0.002 grid, to about
/// ±0.001 (±0.0025 below 3°, where the Mach 8 and 10 curves merge; read as crossing, so the
/// values stay ordered in Mach). Interpolated as [`cone_normal_force_slope`] does, it gives all
/// 12 of Table I's cone-alone values (4.1° to 9.5°, Mach 3 to 6.28) to their printed two
/// decimals.
///
/// Past 24°, where the chart stops, the last three columns are J. L. Sims, *Tables for Supersonic
/// Flow Around Right Circular Cones at Small Angle of Attack*, NASA SP-3007 (1964), Table 2
/// (printed p. 20), at his own 25°, 27.5° and 30° — the same theory the chart plots (Stone's,
/// which Sims says gives expressions "identical to those found by Kopal", p. 7; Fig. 2 plots
/// Kopal's tables), tabulated rather than drawn, on the same base area and for `γ = 1.4`. His
/// Mach rows include all six of [`CONE_MACHS`] exactly, so nothing is interpolated between
/// sources. Where the two overlap they agree to about the chart's own reading error: at 22.5°,
/// the steepest angle both cover, this reading of Fig. 2 and Sims's value differ by 0.0005 to
/// 0.0021 per radian, the largest at Mach 6 (1.7013 read against his 1.6992) — twice the ±0.001
/// the chart is read to, so the hand reading is the looser of the two there
/// (`sims_and_fig_2_agree_where_they_overlap`). The chart's columns are kept below 24° rather
/// than replaced by Sims's so that nothing already validated moves; M1.8e12 revisits that.
const CONE_SLOPES: [[f64; 22]; 6] = [
    // Mach 3
    [
        2.000, 1.976, 1.953, 1.931, 1.911, 1.892, 1.874, 1.858, 1.843, 1.831, 1.820, 1.810, 1.799,
        1.776, 1.750, 1.721, 1.687, 1.648, 1.605, 1.5798551, 1.5174588, 1.4497109,
    ],
    // Mach 4
    [
        2.000, 1.963, 1.935, 1.912, 1.893, 1.877, 1.865, 1.856, 1.849, 1.844, 1.838, 1.831, 1.823,
        1.805, 1.782, 1.753, 1.718, 1.678, 1.634, 1.6096523, 1.5454397, 1.4756774,
    ],
    // Mach 5
    [
        2.000, 1.958, 1.927, 1.904, 1.885, 1.873, 1.865, 1.863, 1.863, 1.862, 1.859, 1.853, 1.847,
        1.828, 1.805, 1.775, 1.740, 1.699, 1.652, 1.6272149, 1.5613461, 1.4900257,
    ],
    // Mach 6
    [
        2.000, 1.950, 1.917, 1.890, 1.878, 1.874, 1.874, 1.876, 1.879, 1.880, 1.877, 1.872, 1.865,
        1.847, 1.822, 1.790, 1.754, 1.713, 1.666, 1.6381839, 1.5710540, 1.4986224,
    ],
    // Mach 8
    [
        2.000, 1.926, 1.891, 1.883, 1.884, 1.890, 1.899, 1.904, 1.907, 1.908, 1.905, 1.899, 1.891,
        1.870, 1.843, 1.811, 1.771, 1.727, 1.678, 1.6503536, 1.5816157, 1.5078364,
    ],
    // Mach 10
    [
        2.000, 1.904, 1.885, 1.887, 1.897, 1.908, 1.916, 1.921, 1.924, 1.924, 1.921, 1.916, 1.907,
        1.884, 1.855, 1.819, 1.779, 1.734, 1.684, 1.6564935, 1.5868584, 1.5123524,
    ],
];

/// A cone's normal-force slope at `α → 0`, per radian on its base area, interpolated linearly
/// between the table's angles and Mach numbers; below Mach 3 its Mach 3 row, above 10 its Mach 10
/// row. The slopes are TN 3527's Fig. 2 to 24°, and NASA SP-3007's tables of the same theory from
/// there to 30° ([ADR-042: cone slopes past Fig. 2's edge][adr-042]).
///
/// [adr-042]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-042-cone-slopes-from-24-to-30-come-from-simss-tables-where-tn-3527s-chart-stops-2026-09-20
///
/// # Errors
///
/// - [`AeroError::Domain`] for a negative or non-finite half-angle, or a Mach number that isn't
///   finite and above 1.
/// - [`AeroError::Unsupported`] for a half-angle past the tables' 30°.
pub fn cone_normal_force_slope(mach: f64, half_angle_rad: f64) -> Result<f64, AeroError> {
    let degrees = half_angle_rad.to_degrees();
    if !(degrees.is_finite() && degrees >= 0.0) {
        return Err(AeroError::Domain {
            what: "tangent-cone half-angle",
            value: half_angle_rad,
        });
    }
    let last = CONE_ANGLES_DEG[CONE_ANGLES_DEG.len() - 1];
    // A millionth of a degree over admits 30° itself through the degree conversion's rounding.
    if degrees > last + 1e-6 {
        return Err(AeroError::Unsupported(format!(
            "a tangent cone of {degrees}° is past the cone tables' {last}° (NASA SP-3007 Table 2)"
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

    /// The two sources of [`CONE_SLOPES`] agree where they overlap. TN 3527's Fig. 2 is read by
    /// hand to about ±0.001 per radian and stops at 24°; Sims's tables (NASA SP-3007 Table 2,
    /// printed p. 20) are printed to eight digits and start their 2.5° grid well below that. At
    /// 22.5°, the steepest angle both cover, the chart's reading and Sims's value differ by no
    /// more than the chart's own error — which is the check that the two are the same theory and
    /// that the columns line up.
    #[test]
    fn sims_and_fig_2_agree_where_they_overlap() {
        // Sims's 22.5° column at each of `CONE_MACHS`, the rows the table holds.
        let sims = [
            1.6362061, 1.6674853, 1.6867491, 1.6991506, 1.7132534, 1.7205191,
        ];
        for (index, mach) in CONE_MACHS.iter().enumerate() {
            let read = cone_normal_force_slope(*mach, 22.5f64.to_radians()).unwrap();
            // 0.0022: the measured worst, at Mach 6, twice the ±0.001 the chart is read to.
            assert!(
                (read - sims[index]).abs() <= 2.2e-3,
                "Mach {mach}: the chart reads {read}, Sims has {}",
                sims[index]
            );
        }
        // And the join at 24° is smooth to the same order: the chart's last value against Sims's
        // first, a degree apart, differ by less than the chart's error times that gap's slope.
        for mach in CONE_MACHS {
            let (at_24, at_25) = (
                cone_normal_force_slope(mach, 24f64.to_radians()).unwrap(),
                cone_normal_force_slope(mach, 25f64.to_radians()).unwrap(),
            );
            let step = (at_24 - at_25) / 1.0;
            // Over 22° to 24° the chart falls about 0.022 per degree; the first Sims step should
            // be of that order, not a jump.
            assert!(
                (0.015..=0.035).contains(&step),
                "Mach {mach}: {at_24} to {at_25} across the sources' join"
            );
        }
    }

    /// [`ShockExpansionBody::aft_flow`] is the flow the march has reached at the body's aft end:
    /// the last element's pressure decayed to that station (TN 3527 eq. 8), read back as a Mach
    /// number through the total pressure the march expands from, and that element's angle.
    ///
    /// The hand calculation is exact here: a conical nose's vertex fixes the total pressure, and
    /// the flow along the cylinder behind it is an isentropic expansion from it.
    #[test]
    fn the_aft_flow_is_what_the_march_has_reached_at_the_end() {
        let segments = [
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::Conical {}, 0.25, 0.027).unwrap(),
            },
            BodySegment::Cylinder {
                length_m: 0.7,
                radius_m: 0.027,
            },
        ];
        let body = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        let half_angle_rad = (0.027_f64 / 0.25).atan();
        for mach in [1.3, 2.0, 3.0, 5.0] {
            let aft = body.aft_flow(mach).unwrap();
            let flows = body.element_flows(mach).unwrap();
            let last = *flows.last().unwrap();
            assert_eq!(aft.angle_rad, last.angle_rad);
            // The total pressure the march expands from: the vertex cone's own state.
            let cone = cone_flow(mach, half_angle_rad).unwrap();
            let total = cone.surface_pressure_ratio * total_over_static(cone.surface_mach);
            // The pressure the last element's decay has reached at the body's aft end, 0.95 m.
            let decay = (-last.decay_per_m * (0.95 - last.corner_x_m)).exp();
            let pressure = last.tangent_cone_pressure_ratio
                - (last.tangent_cone_pressure_ratio - last.pressure_ratio) * decay;
            let want = mach_from_pressure(total, pressure).unwrap();
            assert!(
                (aft.surface_mach - want).abs() < 1e-12 * want,
                "Mach {mach}: the aft flow is Mach {}, by hand {want}",
                aft.surface_mach
            );
            // A cylinder behind a cone has expanded past the free stream by the aft end.
            assert!(aft.surface_mach > mach, "Mach {mach}: {}", aft.surface_mach);
            // The rest of the corner's state: the same pressure, the gradient the last element
            // carries to the aft end, the free stream it was read in, and the radius **there**,
            // which on this body is the tube's and not the vertex's.
            assert_eq!(aft.pressure_ratio, pressure);
            assert_eq!(aft.free_stream_mach, mach);
            assert_eq!(aft.radius_m, 0.027);
            let want_gradient = if last.decay_per_m == 0.0 {
                0.0
            } else {
                (last.tangent_cone_pressure_ratio - pressure)
                    * last.decay_per_m
                    * (last.angle_rad.cos())
            };
            assert!(
                (aft.gradient_p0_per_m - want_gradient).abs() <= 1e-12 * want_gradient.abs(),
                "Mach {mach}: the gradient is {}, by hand {want_gradient}",
                aft.gradient_p0_per_m
            );
            // Below the free stream and still climbing toward it, which is what puts a near-flat
            // flare's corner in the region `flare_reduction_turns_rad` solves for.
            assert!(aft.pressure_ratio < 1.0 && aft.gradient_p0_per_m > 0.0);
        }
        // A flare at the aft end cannot change it: the march is downstream-only, so every
        // element ahead of the flare's corner carries the same flow with it and without it.
        let mut flared = segments.to_vec();
        flared.push(BodySegment::Profile {
            profile: Profile::transition(NoseShape::Conical {}, 0.3, 0.027, 0.08, false).unwrap(),
        });
        let with_flare = ShockExpansionBody::new(&flared, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        for mach in [2.0, 3.0, 5.0] {
            let bare = body.element_flows(mach).unwrap();
            let both = with_flare.element_flows(mach).unwrap();
            assert!(both.len() > bare.len());
            assert_eq!(&both[..bare.len()], &bare[..]);
            // Its own aft flow is the flare's, not the cylinder's.
            let aft = with_flare.aft_flow(mach).unwrap();
            assert_eq!(aft.angle_rad, both[bare.len()].angle_rad);
            assert!((aft.angle_rad - (0.053_f64 / 0.3).atan()).abs() < 1e-12);
        }
    }

    /// The turn a flare's corner is read at is the largest deflection an attached plane oblique
    /// shock can turn the flow through, at the flow reaching the corner rather than at the free
    /// stream (M1.8e17, ADR-047 in `docs/DECISIONS.md`). The cone tables' 30° bounds the flare's
    /// surface angle instead, and the model applies it there.
    #[test]
    fn a_flares_corner_turns_no_more_than_an_attached_shock_can() {
        // Below the cap the limit is the largest deflection an attached plane shock can turn the
        // flow through, which is the maximum of NACA 1135 eq. 138's θ over the shock angle β.
        // Swept here rather than read from eq. 168, so the closed form is checked, not restated.
        let swept = |mach: f64| {
            let (g, m2) = (GAMMA, mach * mach);
            let start = (1.0_f64 / mach).asin();
            (0..=2_000_000)
                .map(|i| start + (PI / 2.0 - start) * i as f64 / 2_000_000.0)
                .map(|beta| {
                    let s2 = beta.sin() * beta.sin();
                    (2.0 / beta.tan() * (m2 * s2 - 1.0) / (m2 * (g + (2.0 * beta).cos()) + 2.0))
                        .atan()
                })
                .fold(f64::NEG_INFINITY, f64::max)
        };
        for mach in [1.2, 1.5, 2.0, 2.5, 3.0, 5.0] {
            let limit = flare_corner_limit_rad(mach).unwrap();
            assert!(
                (limit - swept(mach)).abs() < 1e-6,
                "Mach {mach}: the corner is read to {}°, the swept maximum is {}°",
                limit.to_degrees(),
                swept(mach).to_degrees()
            );
        }
        // Where the cone tables' 30° takes over from the shock, bisected: a flare on a cylinder
        // is drawn no steeper than that above it, whatever the flow could turn through.
        let (mut low, mut high) = (2.0_f64, 3.0_f64);
        loop {
            let middle = 0.5 * (low + high);
            if middle <= low || middle >= high {
                break;
            }
            if flare_corner_limit_rad(middle).unwrap() < crate::blunt_tip::CONE_TABLE_CAP_RAD {
                low = middle;
            } else {
                high = middle;
            }
        }
        assert!(
            (high - 2.519_203_426_042).abs() < 5e-12,
            "the tables bind from Mach {high}"
        );
        assert!(flare_corner_limit_rad(1.0).is_err());
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
        // A fineness-1 cone is 26.57°, past TN 3527's chart and inside Sims's tables, so it flies
        // since M1.8e11; a fineness-0.8 cone is 32.0°, past 30°, and does not.
        assert!(body(false, 1.0, 2.0, 1).slope(3.0, area).is_ok());
        assert!(matches!(
            body(false, 0.8, 2.0, 1).slope(3.0, area),
            Err(AeroError::Unsupported(_))
        ));
        assert!(cone_normal_force_slope(3.0, 30f64.to_radians()).is_ok());
        assert!(matches!(
            cone_normal_force_slope(3.0, 30.001f64.to_radians()),
            Err(AeroError::Unsupported(_))
        ));
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

    /// [`crate::blunt_tip::CONE_TABLE_CAP_RAD`] is the steepest cone the tables carry, so a cap at
    /// it reads a slope and a hair over it does not. If [`CONE_ANGLES_DEG`] ever grows or shrinks,
    /// this is what says the constant has to follow.
    #[test]
    fn the_handovers_ceiling_is_the_cone_tables_last_angle() {
        let ceiling = crate::blunt_tip::CONE_TABLE_CAP_RAD;
        let last = CONE_ANGLES_DEG[CONE_ANGLES_DEG.len() - 1];
        // Equal to the last bit the two conversions allow: 30° → rad → 30° lands a bit low.
        assert!(
            (ceiling.to_degrees() - last).abs() <= 4.0 * f64::EPSILON * last,
            "the handover's ceiling is {}°, the tables' last angle {last}°",
            ceiling.to_degrees()
        );
        assert!(cone_normal_force_slope(3.0, ceiling).is_ok());
        assert!(matches!(
            cone_normal_force_slope(3.0, ceiling * (1.0 + 1e-6)),
            Err(AeroError::Unsupported(_))
        ));
    }

    /// What stops a blunt tip's handover moving to the cone tables' 30° (M1.8e12, ADR-043).
    /// Under the flown cap the committed Arcas Robin nose's march holds its answer to 0.01 per
    /// radian from 10 elements to 160 at every Mach, reducing at most 2 of 160 elements to the
    /// generalized method. Under the tables' cap the same nose reduces 109 of 160 at Mach 4.63
    /// and 145 at Mach 5, and the answer moves with the element count: 0.21 per radian at Mach
    /// 4.63, 7% of it. That is the method, not the arithmetic — both readings are unchanged when
    /// the Mach number is nudged by eight of its last bits.
    #[test]
    fn a_steeper_handover_moves_the_march_out_of_its_range() {
        use crate::blunt_tip::{CONE_TABLE_CAP_RAD, MAX_HANDOVER_RAD};
        let radius = 1.125 * 0.0254;
        let area = PI * radius * radius;
        let body = |steps: usize, cap_rad: f64| {
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
            .with_handover_cap_rad(cap_rad)
        };
        let counts = [
            DEFAULT_ELEMENTS_PER_CURVE,
            4 * DEFAULT_ELEMENTS_PER_CURVE,
            16 * DEFAULT_ELEMENTS_PER_CURVE,
        ];
        let read = |cap_rad: f64, mach: f64| {
            let slopes: Vec<f64> = counts
                .iter()
                .map(|&n| body(n, cap_rad).slope(mach, area).unwrap().slope_per_rad)
                .collect();
            let high = slopes.iter().copied().fold(f64::MIN, f64::max);
            let low = slopes.iter().copied().fold(f64::MAX, f64::min);
            let reduced = body(counts[2], cap_rad).reduced_elements(mach).unwrap();
            (slopes[0], high - low, reduced)
        };
        // The cap hpr flies: the element count is worth a thousandth of the answer, all the way
        // to Mach 5, and the march barely leaves the second-order method.
        for mach in [1.5, 2.3, 2.96, 3.96, 4.63, 5.0] {
            let (_, spread, reduced) = read(MAX_HANDOVER_RAD, mach);
            assert!(
                spread < 0.01 && reduced <= 2,
                "the flown cap at Mach {mach}: the count is worth {spread:.4} per radian, \
                 {reduced} of {} elements reduced",
                counts[2]
            );
        }
        // The cone tables' cap costs nothing below Mach 4 — that is not where it is blocked.
        for mach in [1.5, 2.3, 2.96, 3.96] {
            let (_, spread, reduced) = read(CONE_TABLE_CAP_RAD, mach);
            assert!(
                spread < 0.02 && reduced <= 2,
                "the tables' cap at Mach {mach}: the count is worth {spread:.4} per radian, \
                 {reduced} of {} elements reduced",
                counts[2]
            );
        }
        // Above it the march reduces most of the nose and the answer follows the element count.
        for (mach, least_reduced, least_spread) in [(4.63, 100, 0.2), (5.0, 140, 0.04)] {
            let (coarse, spread, reduced) = read(CONE_TABLE_CAP_RAD, mach);
            let (flown, _, _) = read(MAX_HANDOVER_RAD, mach);
            assert!(
                reduced >= least_reduced && spread >= least_spread,
                "the tables' cap at Mach {mach}: {reduced} of {} elements reduced, the count \
                 worth {spread:.4} per radian",
                counts[2]
            );
            assert!(
                coarse - flown > 0.01,
                "the tables' cap at Mach {mach} should read above the flown cap's \
                 ({coarse:.4} against {flown:.4})"
            );
        }
        // The arithmetic isn't what moves. Which elements reduce is a decision on the sign of
        // `η`, and none of them is close enough to zero to turn on rounding: nudge the Mach
        // number by eight of its last bits and the same elements reduce, for an answer that
        // follows to a part in a billion.
        for cap in [MAX_HANDOVER_RAD, CONE_TABLE_CAP_RAD] {
            for mach in [4.63_f64, 5.0] {
                let nudged = mach * (1.0 + 8.0 * f64::EPSILON);
                let (slope, _, reduced) = read(cap, mach);
                let (nudged_slope, _, nudged_reduced) = read(cap, nudged);
                assert_eq!(
                    (reduced, (nudged_slope / slope - 1.0).abs() < 1e-9),
                    (nudged_reduced, true),
                    "{}° at Mach {mach} against {nudged}: {slope} against {nudged_slope}",
                    cap.to_degrees()
                );
            }
        }
    }

    /// What a reduced element costs, and what a crossing costs (M1.8e13, ADR-044). On TN 3527's
    /// own fineness-3 ogive the march reduces 27 of 160 elements at Mach 5.05 and 50 at Mach
    /// 6.28, and the answer still settles to 0.002 per radian from 10 elements to 160:
    /// there `η < 0` is the gradient changing sign, with the surface pressure below its tangent
    /// cone's the whole way. The report's bodies never cross, so the report never had to say
    /// what a crossing does — which is why hpr's reading of `η < 0`
    /// ([issue #81](https://github.com/nrdptel/hpr-sim/issues/81)) is not what
    /// [issue #108](https://github.com/nrdptel/hpr-sim/issues/108) turns on.
    #[test]
    #[allow(
        clippy::approx_constant,
        reason = "6.28 is one of TN 3527's test Mach numbers, not 2π"
    )]
    fn a_reduced_element_settles_where_tn3527s_own_bodies_never_cross() {
        let body = |steps: usize| {
            ShockExpansionBody::new(
                &[BodySegment::Profile {
                    profile: Profile::nose(NoseShape::TANGENT_OGIVE, 3.0, 0.5).unwrap(),
                }],
                steps,
            )
            .unwrap()
        };
        let counts = [DEFAULT_ELEMENTS_PER_CURVE, 16 * DEFAULT_ELEMENTS_PER_CURVE];
        for (mach, coarse_want, fine_want) in [(5.05, 2, 27), (6.28, 3, 50)] {
            let read = |n: usize| {
                let b = body(n);
                (
                    b.slope(mach, 0.25 * PI).unwrap().slope_per_rad,
                    b.reduced_elements(mach).unwrap(),
                    b.tangent_cone_crossings(mach).unwrap(),
                )
            };
            let (coarse, coarse_reduced, _) = read(counts[0]);
            let (fine, fine_reduced, _) = read(counts[1]);
            // Exact, because the guide and the ADR quote these counts.
            assert_eq!(
                (coarse_reduced, fine_reduced),
                (coarse_want, fine_want),
                "the ogive at Mach {mach} should reduce {coarse_want} of {} elements and \
                 {fine_want} of {}",
                counts[0],
                counts[1]
            );
            for n in counts {
                assert_eq!(
                    read(n).2,
                    0,
                    "the ogive at Mach {mach} on {n} elements should never cross its tangent cone"
                );
            }
            assert!(
                (fine - coarse).abs() < 2e-3,
                "the ogive at Mach {mach} should settle: {coarse} on {} elements against {fine} \
                 on {}",
                counts[0],
                counts[1]
            );
        }
    }

    /// Why a crossing costs what it does (M1.8e13, ADR-044). Along an element the method relaxes
    /// the pressure and the loading toward the tangent cone's as `e^(−η)`, `η = k (x − x₂)` with
    /// `k = (∂p/∂s)₂/((p_c − p₂) cos δ₂)`, so where the pressure crosses its tangent cone's the gap
    /// closes while the gradient carries on and `k` has a pole. The pressure rides through it;
    /// the loading does not, because eq. 19 borrows the pressure's `k` while its own gap stays
    /// open. Here, at the case [issue #108](https://github.com/nrdptel/hpr-sim/issues/108)
    /// reports — the committed nose under the cone tables' cap at Mach 4.63 — the march crosses
    /// twice. Between the crossings nearly every element is reduced, so the loading never relaxes
    /// and stands about a quarter above its tangent cone's by the second one. The element there is
    /// back inside the method, and how much of that gap it sheds in one step is the mesh's to
    /// choose: 98% of it on the 40-element march against 12% on the 160-element one. The cap hpr
    /// flies never crosses at all.
    #[test]
    fn a_crossing_is_a_pole_in_the_rate_the_march_relaxes_at() {
        use crate::blunt_tip::{CONE_TABLE_CAP_RAD, MAX_HANDOVER_RAD};
        let mach = 4.63;
        let radius = 1.125 * 0.0254;
        let body = |steps: usize, cap_rad: f64| {
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
            .with_handover_cap_rad(cap_rad)
        };
        // The elements a crossing falls between, as `tangent_cone_crossings` counts them: the gap
        // changes sign between two elements that have one and share a kind of tangent cone.
        let crossings = |steps: usize, cap_rad: f64| {
            let flows = body(steps, cap_rad).element_flows(mach).unwrap();
            let gap = |e: &ElementFlowReport| e.tangent_cone_pressure_ratio - e.pressure_ratio;
            let mut last: Option<(usize, f64)> = None;
            let mut at = Vec::new();
            for (index, flow) in flows.iter().enumerate() {
                let this = gap(flow);
                if this == 0.0 {
                    continue;
                }
                if let Some((previous, was)) = last
                    && (this > 0.0) != (was > 0.0)
                    && (flows[previous].angle_rad > CONE_ANGLE_FLOOR_RAD)
                        == (flow.angle_rad > CONE_ANGLE_FLOOR_RAD)
                {
                    at.push(index);
                }
                last = Some((index, this));
            }
            (flows, at)
        };
        // What the mesh decides: the share of the loading's gap the element at the second
        // crossing sheds in its own length, `1 − e^(−η)`.
        let mut shares = Vec::new();
        for steps in [
            4 * DEFAULT_ELEMENTS_PER_CURVE,
            16 * DEFAULT_ELEMENTS_PER_CURVE,
        ] {
            let (flows, at) = crossings(steps, CONE_TABLE_CAP_RAD);
            assert_eq!(
                (
                    at.len(),
                    body(steps, CONE_TABLE_CAP_RAD)
                        .tangent_cone_crossings(mach)
                        .unwrap()
                ),
                (2, 2),
                "the tables' cap on {steps} elements should cross its tangent cone twice, and \
                 the shipped count should agree with the rule spelled out here"
            );
            // The sign test behind the count is not a coin flip: the gap either side of a
            // crossing is far larger than the 1e-12 a march reproduces to across platforms.
            for &index in &at {
                let margin =
                    (flows[index].tangent_cone_pressure_ratio - flows[index].pressure_ratio).abs()
                        / flows[index].pressure_ratio;
                assert!(
                    margin > 1e-8,
                    "the crossing at element {index} on {steps} elements leaves a gap of \
                     {margin:.2e} of the pressure, too near the noise to decide a sign on"
                );
            }
            assert!(
                at[1] + 1 < flows.len(),
                "a crossing needs an element after it"
            );
            let second = &flows[at[1]];
            let length_m = flows[at[1] + 1].corner_x_m - second.corner_x_m;
            let loading_gap = (second.tangent_cone_loading_per_rad - second.loading_per_rad)
                / second.loading_per_rad;
            // The step is taken by an element the method still owns, across a gap the reduced
            // stretch behind it left wide open. This is why a rule for `η < 0` alone would not
            // settle the answer, and why it cannot be judged apart from the crossing either.
            assert!(
                second.decay_per_m > 0.0 && loading_gap < -0.15,
                "on {steps} elements the second crossing's element should be inside the method \
                 (rate {}) with its loading {:.1}% from its tangent cone's",
                second.decay_per_m,
                100.0 * loading_gap
            );
            shares.push(1.0 - (-second.decay_per_m * length_m).exp());
        }
        assert!(
            shares[0] > 0.9 && shares[1] < 0.2,
            "how much of the gap one step sheds should be the mesh's answer, not the model's: \
             {:.0}% on {} elements against {:.0}% on {}",
            100.0 * shares[0],
            4 * DEFAULT_ELEMENTS_PER_CURVE,
            100.0 * shares[1],
            16 * DEFAULT_ELEMENTS_PER_CURVE
        );
        // The cap hpr flies marches the same nose at the same Mach numbers without crossing.
        for steps in [
            DEFAULT_ELEMENTS_PER_CURVE,
            4 * DEFAULT_ELEMENTS_PER_CURVE,
            16 * DEFAULT_ELEMENTS_PER_CURVE,
        ] {
            for mach in [4.63, 5.0] {
                assert_eq!(
                    body(steps, MAX_HANDOVER_RAD)
                        .tangent_cone_crossings(mach)
                        .unwrap(),
                    0,
                    "the flown cap at Mach {mach} on {steps} elements should not cross"
                );
            }
        }
    }

    /// A crossing is a flag, not a verdict (M1.8e13, ADR-044). It says the answer moved over the
    /// meshes the cap sweep holds — 10, 40 and 160 elements per curve — not that no mesh settles
    /// it. Under a 28° cap at Mach 5 the committed nose crosses at every mesh, and its 0.69 per
    /// radian spread over the sweep's three is all in the coarse end: from 60 elements on it holds
    /// to 0.005, tighter than the worst reading in the sweep that never crosses. Under the cone
    /// tables' cap at Mach 4.63 it is the other kind, still moving by 0.2 per radian from 60
    /// elements to 640. The guide says both.
    #[test]
    fn a_crossing_says_the_answer_moved_not_that_it_never_settles() {
        use crate::blunt_tip::CONE_TABLE_CAP_RAD;
        let radius = 1.125 * 0.0254;
        let area = PI * radius * radius;
        let read = |steps: usize, cap_rad: f64, mach: f64| {
            let body = ShockExpansionBody::new(
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
            .with_handover_cap_rad(cap_rad);
            (
                body.slope(mach, area).unwrap().slope_per_rad,
                body.tangent_cone_crossings(mach).unwrap(),
            )
        };
        // Past the coarse end: six, sixteen and sixty-four times the flown element count.
        let fine = [
            6 * DEFAULT_ELEMENTS_PER_CURVE,
            16 * DEFAULT_ELEMENTS_PER_CURVE,
            64 * DEFAULT_ELEMENTS_PER_CURVE,
        ];
        let spread = |cap_rad: f64, mach: f64| {
            let slopes: Vec<(f64, usize)> = fine.iter().map(|&n| read(n, cap_rad, mach)).collect();
            assert!(
                slopes.iter().all(|&(_, crossings)| crossings > 0),
                "this case should cross at every mesh: {slopes:?}"
            );
            let high = slopes.iter().map(|&(s, _)| s).fold(f64::MIN, f64::max);
            let low = slopes.iter().map(|&(s, _)| s).fold(f64::MAX, f64::min);
            high - low
        };
        // Crossing, and settled once the mesh is fine enough.
        let settles = spread(28_f64.to_radians(), 5.0);
        assert!(
            settles < 0.005,
            "28° at Mach 5 crosses but settles: it moves {settles:.4} per radian over {fine:?}"
        );
        // Crossing, and still moving there.
        let wanders = spread(CONE_TABLE_CAP_RAD, 4.63);
        assert!(
            wanders > 0.2,
            "the tables' cap at Mach 4.63 crosses and keeps moving: {wanders:.4} per radian over \
             {fine:?}"
        );
    }

    /// The Arcas Robin's committed nose (a power series, `n` = 0.6369, 9.375 in long on a
    /// 2.25-in body) and the short model's cylinder: four times the default elements move its
    /// slope by under 0.01 per radian and its centre of pressure by under 0.01 calibers, through
    /// Mach 5. Sixteen times the default move a five-calibre elliptical or von Kármán nose's by
    /// under 0.02: the tangent body settles more slowly on a nose whose slope changes fastest
    /// just behind the cap.
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
        // Five-calibre elliptical and von Kármán noses on a cylinder, the default elements
        // against sixteen times as many.
        for shape in [NoseShape::Elliptical {}, NoseShape::VON_KARMAN] {
            let body = |steps| {
                ShockExpansionBody::new(
                    &[
                        BodySegment::Profile {
                            profile: Profile::nose(shape, 5.0, 0.5).unwrap(),
                        },
                        BodySegment::Cylinder {
                            length_m: 5.0,
                            radius_m: 0.5,
                        },
                    ],
                    steps,
                )
                .unwrap()
            };
            let (coarse, fine) = (body(DEFAULT_ELEMENTS_PER_CURVE), body(160));
            for mach in [1.5, 3.0, 5.0] {
                let a = coarse.slope(mach, 0.25 * PI).unwrap();
                let b = fine.slope(mach, 0.25 * PI).unwrap();
                assert!(
                    (a.slope_per_rad - b.slope_per_rad).abs() < 0.02
                        && (a.centre_of_pressure_m - b.centre_of_pressure_m).abs() < 0.02,
                    "{shape:?} at Mach {mach}: {a:?} against {b:?}"
                );
            }
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

    /// A cap that shrinks to nothing doesn't reach the cone it sits on: the march starts from the
    /// tangent cone at the handover, and carries that cone's total pressure the whole way, however
    /// small the cap. A power-series nose of `n` = 0.99, four calibres long, is a 7.1° cone but for
    /// a tip 1e-55 calibres across, yet at Mach 4 its cylinder carries 1.21 per radian where the
    /// cone's carries 1.37, because the march runs on the 24° cone's total pressure (107 free
    /// streams against the 7.1° cone's 151). The noses themselves agree to 0.01. Pinned so the
    /// limit is visible and any fix shows here ([issue #101](https://github.com/nrdptel/hpr-sim/issues/101)).
    #[test]
    fn a_vanishing_cap_does_not_reach_the_cone_it_sits_on() {
        let area = 0.25 * PI;
        let body = |exponent| {
            ShockExpansionBody::new(
                &[
                    BodySegment::Profile {
                        profile: Profile::nose(NoseShape::PowerSeries { exponent }, 4.0, 0.5)
                            .unwrap(),
                    },
                    BodySegment::Cylinder {
                        length_m: 6.0,
                        radius_m: 0.5,
                    },
                ],
                DEFAULT_ELEMENTS_PER_CURVE,
            )
            .unwrap()
        };
        let blunt = body(0.99);
        let cone = body(1.0);
        assert!(blunt.has_blunt_tip() && !cone.has_blunt_tip());
        let (blunt, cone) = (
            blunt.segment_slopes(4.0, area).unwrap(),
            cone.segment_slopes(4.0, area).unwrap(),
        );
        let near = |got: f64, want: f64, tol: f64, what: &str| {
            assert!(
                (got - want).abs() <= tol,
                "{what}: {got} against {want} ± {tol}"
            );
        };
        near(
            blunt[0].slope_per_rad,
            cone[0].slope_per_rad,
            0.01,
            "the noses",
        );
        near(
            blunt[1].slope_per_rad,
            1.211,
            5e-3,
            "the cylinder behind the vanishing cap",
        );
        near(cone[1].slope_per_rad, 1.374, 5e-3, "the cone's cylinder");
    }

    /// A blunt nose can take more than one segment, and the cap hands over wherever its slope
    /// falls to the handover's — but never past the nose (M1.8e18).
    ///
    /// TN D-4865's model 2 is a 0.257-diameter sphere blended into a 2.75° cone by a 0.429 arc,
    /// and the sphere is still at 38.3° where the arc takes over, steeper than the handover's 24°
    /// cap at any Mach number. The handover is on the arc. A cylinder behind a nose is not the
    /// nose, so a cap that reaches one is refused instead of handing over at no angle at all,
    /// with none of the total pressure the tip took out of the flow.
    #[test]
    fn a_blunt_nose_hands_over_on_a_later_segment_but_never_past_the_nose() {
        // Model 2's nose, in base diameters, as `xtask/src/aero_flare.rs` builds it.
        let (sphere_end_x, sphere_end_r) = (0.097_751_728_849_191_7, 0.201_715_116_279_069_8);
        let (nose_end_x, nose_end_r) = (0.342_995_992_350_487_5, 0.293_505_957_802_043_8);
        let arc = Profile::transition(
            NoseShape::Ogive {
                radius_ratio: 1.148_551_684_394_344_9,
            },
            nose_end_x - sphere_end_x,
            sphere_end_r,
            nose_end_r,
            false,
        )
        .unwrap();
        let cone = Profile::transition(
            NoseShape::Conical {},
            0.757_619_879_544_843_6,
            nose_end_r,
            0.329_897_050_227_035_4,
            false,
        )
        .unwrap();
        let segments = [
            BodySegment::SphericalCap {
                radius_m: 0.257,
                length_m: sphere_end_x,
            },
            BodySegment::Profile { profile: arc },
            BodySegment::Profile { profile: cone },
        ];
        let body = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        for mach in [1.6, 2.0, 3.0, 4.63] {
            let handover = body.handover_m(mach).unwrap().expect("a blunt tip");
            assert!(
                handover > sphere_end_x && handover < nose_end_x,
                "Mach {mach}: the handover is at {handover}, not on the blend arc"
            );
            let angle = crate::blunt_tip::handover_angle_rad(mach)
                .unwrap()
                .min(crate::blunt_tip::MAX_HANDOVER_RAD);
            let slope = arc.radius_and_slope(handover - sphere_end_x).1;
            assert!(
                (slope.atan() - angle).abs() < 1e-12,
                "Mach {mach}: the handover is at {}°, not the handover's {}°",
                slope.atan().to_degrees(),
                angle.to_degrees()
            );
            // The cap is still the sphere plus part of the arc, so the march starts behind it.
            body.slope(mach, 0.25 * PI).expect("model 2's nose marches");
        }
        // A curved segment that narrows is a boattail, so a cap may not reach one either: the
        // handover would land on its fore end at a slope of zero, with none of the total pressure
        // the tip took out of the flow.
        let boattail = [
            BodySegment::SphericalCap {
                radius_m: 0.5,
                length_m: 0.1,
            },
            BodySegment::Profile {
                profile: Profile::transition(NoseShape::TANGENT_OGIVE, 0.4, 0.3, 0.2, false)
                    .unwrap(),
            },
        ];
        let boattail = ShockExpansionBody::new(&boattail, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        for mach in [1.5, 2.0, 3.0] {
            let err = boattail
                .handover_m(mach)
                .expect_err("a cap that reaches a boattail")
                .to_string();
            assert!(err.contains("all the way to its end"), "Mach {mach}: {err}");
        }
        // A pointed nose is one segment however many curved shapes follow it, so a curved
        // widening transition behind one is still the afterbody and a reduced element there is
        // still refused: only a spherical cap, which is a piece of a nose rather than a whole
        // one, carries the nose past its own segment.
        let pointed = [
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::TANGENT_OGIVE, 1.0, 0.25).unwrap(),
            },
            BodySegment::Profile {
                profile: Profile::transition(
                    NoseShape::Ogive { radius_ratio: 2.0 },
                    0.4,
                    0.25,
                    0.35,
                    false,
                )
                .unwrap(),
            },
        ];
        let pointed = ShockExpansionBody::new(&pointed, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        assert_eq!(
            super::nose_segments(&pointed.segments),
            1,
            "a pointed nose is one segment"
        );
        // A cylinder behind a nose is not the nose: the cap may not reach it.
        let steep = [
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::PowerSeries { exponent: 0.6369 }, 4.17, 0.5)
                    .unwrap(),
            },
            BodySegment::Cylinder {
                length_m: 4.0,
                radius_m: 0.5,
            },
        ];
        let steep = ShockExpansionBody::new(&steep, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
        let err = steep
            .handover_m(1.2)
            .expect_err("a cap that reaches the cylinder")
            .to_string();
        assert!(err.contains("all the way to its end"), "{err}");
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
    /// A pointed 2.75° cone, five calibres of tube, and a conical flare of `flare_deg`. The two
    /// angles are TN D-4865 model 2's; the layout is **not** (that model is blunt-nosed and has no
    /// tube), and the edge below depends on the tube, which sets the flow reaching the flare. The
    /// method is inviscid, so only angles and ratios of lengths to radii matter.
    fn flared_body(flare_deg: f64) -> Vec<BodySegment> {
        let radius_m = 0.1;
        let flare_length_m = 0.3;
        vec![
            BodySegment::Profile {
                profile: Profile::nose(
                    NoseShape::Conical {},
                    radius_m / 2.75_f64.to_radians().tan(),
                    radius_m,
                )
                .unwrap(),
            },
            BodySegment::Cylinder {
                length_m: 1.0,
                radius_m,
            },
            BodySegment::Profile {
                profile: Profile::transition(
                    NoseShape::Conical {},
                    flare_length_m,
                    radius_m,
                    radius_m + flare_length_m * flare_deg.to_radians().tan(),
                    false,
                )
                .unwrap(),
            },
        ]
    }

    /// Whether the method marches that body at `mach`.
    fn flare_marches(mach: f64, flare_deg: f64) -> Result<(), AeroError> {
        ShockExpansionBody::new(&flared_body(flare_deg), DEFAULT_ELEMENTS_PER_CURVE)?
            .slope(mach, PI * 0.01)
            .map(|_| ())
    }

    /// The steepest flare the method marches at `mach`, bisected to f64 resolution: the last angle
    /// that returns a slope, with the first that doesn't a bit above it.
    ///
    /// The marchable set is not an interval — a band of very shallow flares, under a degree on
    /// this body, is refused because the pressure behind the corner moves away from its tangent
    /// cone's — so the bracket's lower end is asserted to march rather than assumed.
    fn steepest_flare_deg(mach: f64) -> f64 {
        let (mut lo, mut hi) = (1.0_f64, 45.0_f64);
        assert!(
            flare_marches(mach, lo).is_ok(),
            "Mach {mach}: 1° already fails, so the shallow band this brackets above has moved"
        );
        assert!(flare_marches(mach, hi).is_err(), "Mach {mach}: 45° marches");
        loop {
            let mid = 0.5 * (lo + hi);
            if mid <= lo || mid >= hi {
                return lo;
            }
            if flare_marches(mach, mid).is_ok() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
    }

    /// The body the tests' flared rocket puts ahead of its flare: an ogive nose 0.25 m long on a
    /// 27 mm radius and a 0.7 m tube, which is what delivers the flow to the flare's corner.
    fn ahead_of_the_flare() -> Vec<BodySegment> {
        vec![
            BodySegment::Profile {
                profile: Profile::nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027)
                    .unwrap(),
            },
            BodySegment::Cylinder {
                length_m: 0.7,
                radius_m: 0.027,
            },
        ]
    }

    /// That body with a conical flare 0.3 m long of `deg` on the back of it.
    fn with_a_flare(deg: f64) -> Vec<BodySegment> {
        let mut segments = ahead_of_the_flare();
        segments.push(BodySegment::Profile {
            profile: Profile::transition(
                NoseShape::Conical {},
                0.3,
                0.027,
                0.027 + 0.3 * deg.to_radians().tan(),
                false,
            )
            .unwrap(),
        });
        segments
    }

    /// Which turns the march reduces to the generalized method is a property of the corner's own
    /// state, and [`flare_reduction_turns_rad`] solves for the two that bound them: the crossing,
    /// where the pressure behind the corner lands on its tangent cone's, and the balance, where
    /// the corner's own compression cancels the gradient the body ahead delivers. An element is
    /// reduced strictly between them, in whichever order they come — on this body the crossing is
    /// the shallower from about Mach 1.5 up and the deeper below it.
    ///
    /// The three angles [issue #117](https://github.com/nrdptel/hpr-sim/issues/117) reported,
    /// each found by bisecting the model's own refusal, come back out of those two equations: the
    /// band's edges are the crossing at Mach 4.70 and the balance at Mach 5, and the shallowest
    /// angle that lifted the join's start is the crossing at Mach 2.20.
    ///
    /// **What is left is the tangent cone's own accuracy, not the search's.** Below
    /// [`SLENDER_CONE_RAD`] the tangent cone is slender-cone theory's closed form and the
    /// crossing solves to the last bits of an `f64`; above it the cone flow is a Taylor–Maccoll
    /// integration, and the residual is that integration's, about 1e-10 of the free stream's
    /// pressure. Dividing by the gap's slope in the turn, that is about 2e-10° — which is the
    /// spread the three platforms of CI showed on the band's lower edge.
    #[test]
    fn the_turns_a_reduced_element_lies_between_come_from_the_corners_own_state() {
        let ahead = ShockExpansionBody::new(&ahead_of_the_flare(), DEFAULT_ELEMENTS_PER_CURVE)
            .expect("the body ahead of the flare");
        let turns = |mach: f64| flare_reduction_turns_rad(&ahead.aft_flow(mach).unwrap()).unwrap();
        // The three angles issue #117 quoted, from the corner's state instead of a bisection.
        // The two past Mach 4 are held to 2e-9°, ten times the 2e-10° the tangent cone's own
        // integration leaves in them (see the residuals at the end): pinning them tighter would
        // be a statement about one machine.
        assert!(
            (turns(4.7).crossing_rad.to_degrees() - 0.038_161_270_2).abs() < 2e-9,
            "the band's lower edge is {}°",
            turns(4.7).crossing_rad.to_degrees()
        );
        assert!(
            (turns(5.0).balance_rad.to_degrees() - 0.058_820_517_4).abs() < 2e-9,
            "the band's upper edge is {}°",
            turns(5.0).balance_rad.to_degrees()
        );
        // This one's tangent cone is slender-cone theory's closed form, so it carries its digits.
        assert!(
            (turns(2.2).crossing_rad.to_degrees() - 0.000_901_824_655).abs() < 5e-12,
            "the join's shallowest step is at {}°",
            turns(2.2).crossing_rad.to_degrees()
        );
        // A turn strictly between the two is reduced, and one outside is not. The march's own
        // reading of it: a reduced element carries no decay (`η = 0`).
        let reduced = |mach: f64, deg: f64| {
            let body = ShockExpansionBody::new(&with_a_flare(deg), DEFAULT_ELEMENTS_PER_CURVE)
                .expect("a flared body");
            let flows = body.element_flows(mach).expect("a march over it");
            flows[flows.len() - 1].decay_per_m == 0.0
        };
        for mach in [2.0, 2.2, 3.0, 4.0, 4.3, 4.65, 4.7, 5.0] {
            let both = turns(mach);
            let (low, high) = (
                both.crossing_rad.min(both.balance_rad).to_degrees(),
                both.crossing_rad.max(both.balance_rad).to_degrees(),
            );
            // From Mach 1.5 up on this body the crossing is the shallower of the two; which one
            // is, though, is not part of the rule, and it swaps below that (see the end).
            assert!(low == both.crossing_rad.to_degrees(), "Mach {mach}");
            for (deg, want) in [
                (low * (1.0 - 1e-6), false),
                (low * (1.0 + 1e-6), true),
                (0.5 * (low + high), true),
                (high * (1.0 - 1e-6), true),
                (high * (1.0 + 1e-6), false),
            ] {
                assert_eq!(
                    reduced(mach, deg),
                    want,
                    "Mach {mach}: {deg}° should{} be reduced, between {low}° and {high}°",
                    if want { "" } else { " not" }
                );
            }
        }
        // The residual each solution left: `p₂ − p_c` at the crossing and `(∂p/∂s)₂` at the
        // balance. The balance is trigonometry and closes to the last bits of an `f64`; so does
        // the crossing while its tangent cone is slender-cone theory's closed form (Mach 3's turn
        // is 0.0066°, under `SLENDER_CONE_RAD`'s 0.029°). Above that angle the cone flow is an
        // integration and the residual is its own, four orders larger — which is the whole point
        // of reporting it.
        assert!(turns(3.0).crossing_rad < SLENDER_CONE_RAD);
        assert!(turns(4.7).crossing_rad > SLENDER_CONE_RAD);
        for mach in [2.0, 2.2, 3.0] {
            assert!(
                turns(mach).crossing_residual_p0.abs() < 2e-14,
                "Mach {mach} leaves {} of the pressure at the crossing",
                turns(mach).crossing_residual_p0
            );
        }
        for mach in [4.3, 4.65, 4.7, 5.0] {
            let left = turns(mach).crossing_residual_p0.abs();
            assert!(
                left < 1e-9,
                "Mach {mach} leaves {left} of the pressure at the crossing"
            );
        }
        for mach in [2.0, 3.0, 4.7, 5.0] {
            assert!(
                turns(mach).balance_residual_p0_per_m.abs() < 2e-14,
                "Mach {mach} leaves {} of the gradient at the balance",
                turns(mach).balance_residual_p0_per_m
            );
        }
        // Which of the two is the shallower is not part of the rule, and on this body it swaps
        // between Mach 1.46 and Mach 1.51. Both turns are under `NEARLY_PARALLEL_RAD` there, so a
        // flare of either angle is merged into the tube ahead of it and there is no corner to
        // reduce: nothing switches down there because nothing is drawn.
        for mach in [1.2, 1.46] {
            let both = turns(mach);
            assert!(
                both.balance_rad < both.crossing_rad,
                "at Mach {mach} the balance is {} rad and the crossing {} rad",
                both.balance_rad,
                both.crossing_rad
            );
            assert!(
                both.crossing_rad < NEARLY_PARALLEL_RAD,
                "at Mach {mach} a flare of {} rad would still be drawn",
                both.crossing_rad
            );
        }
        assert!(turns(1.51).crossing_rad < turns(1.51).balance_rad);
    }

    /// A body long enough to hand the free stream's own pressure to a corner behind it still has
    /// both turns, and the crossing is a turn of nothing rather than a refusal.
    ///
    /// The crossing solves `p₂(θ) = p_c(δ₁ + θ)`, and where the body ahead has relaxed all the
    /// way back to the free stream both sides already agree at `θ = 0`. Round-tripping that turn
    /// through the isentropic relations instead of returning the state itself would leave a bit
    /// of noise there, and the sign of that bit would decide whether the answer came back at all
    /// — a different set of Mach rows on each platform. So `behind` short-circuits a turn of
    /// nothing, and the root is bracketed over the turns a widening corner can make rather than
    /// chased from a guess.
    #[test]
    fn a_corner_behind_a_relaxed_body_still_has_both_turns() {
        let mut relaxed = 0;
        for tube_m in [0.7_f64, 3.0, 6.0] {
            let segments = [
                BodySegment::Profile {
                    profile: Profile::nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.25, 0.027)
                        .unwrap(),
                },
                BodySegment::Cylinder {
                    length_m: tube_m,
                    radius_m: 0.027,
                },
            ];
            let body = ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).unwrap();
            let mut mach = 1.2;
            while mach < 5.0 {
                let aft = body.aft_flow(mach).expect("a march to the aft end");
                let turns = flare_reduction_turns_rad(&aft)
                    .unwrap_or_else(|e| panic!("a {tube_m} m tube at Mach {mach}: {e}"));
                assert!(
                    turns.crossing_rad.is_finite() && turns.balance_rad.is_finite(),
                    "a {tube_m} m tube at Mach {mach}"
                );
                if aft.pressure_ratio == 1.0 {
                    relaxed += 1;
                    assert_eq!(turns.crossing_rad, 0.0);
                    assert_eq!(turns.crossing_residual_p0, 0.0);
                }
                mach += 0.1;
            }
        }
        assert!(
            relaxed > 20,
            "only {relaxed} rows reached the free stream's pressure"
        );
    }

    /// A body of a nose, a tube and a conical flare, for measuring what the crossing costs.
    fn nosed_tube_and_flare(
        cone_deg: Option<f64>,
        radius_m: f64,
        tube_m: f64,
        flare_m: f64,
        flare_deg: f64,
    ) -> Vec<BodySegment> {
        let (shape, nose_m) = match cone_deg {
            Some(deg) => (NoseShape::Conical {}, radius_m / deg.to_radians().tan()),
            None => (NoseShape::Ogive { radius_ratio: 1.0 }, 0.25),
        };
        let mut segments = vec![
            BodySegment::Profile {
                profile: Profile::nose(shape, nose_m, radius_m).unwrap(),
            },
            BodySegment::Cylinder {
                length_m: tube_m,
                radius_m,
            },
        ];
        if flare_m > 0.0 {
            segments.push(BodySegment::Profile {
                profile: Profile::transition(
                    NoseShape::Conical {},
                    flare_m,
                    radius_m,
                    radius_m + flare_m * flare_deg.to_radians().tan(),
                    false,
                )
                .unwrap(),
            });
        }
        segments
    }

    /// What the crossing costs is **not** bounded by the tests' rocket, and it is crossed in the
    /// Mach number as well as in the flare's angle.
    ///
    /// [`the_turns_a_reduced_element_lies_between_come_from_the_corners_own_state`] shows where the
    /// march reduces an element; where a drawn flare's angle sweeps past the crossing the loading
    /// steps, because `η` has a pole there. The whole rocket of
    /// `a_near_flat_flare_reads_through_and_leaves_only_the_corners_crossing` puts that step at
    /// +0.129% at worst — but that is one body, one flare length and one place to measure. On the
    /// body alone it is an order larger, it **grows with the flare's length**, and shortening the
    /// tube ahead of the corner moves the whole region from thousandths of a degree to degrees,
    /// where real flares live. None of these is a bound either: what is claimed is only that the
    /// tests' rocket's figure is not one.
    ///
    /// The same pole is crossed in Mach at a fixed angle, and the table's rows are 0.05 Mach
    /// apart, so a flight reads it as a step between two adjacent rows. Before
    /// [M1.8e19](https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md) the reduced rows
    /// were refused, so the table stopped above them and the join covered the pole; it is now
    /// inside the table. That trade is the milestone's, and this test is its size.
    #[test]
    fn what_the_crossing_costs_is_not_bounded_by_the_tests_rocket() {
        // Either side of a body's own crossing, on the body alone: the fraction its `C_Nα` moves
        // and how far its centre of pressure moves, in calibres of the tube ahead of the flare.
        let across = |cone_deg: Option<f64>,
                      radius_m: f64,
                      tube_m: f64,
                      flare_m: f64,
                      mach: f64|
         -> (f64, f64) {
            let ahead = ShockExpansionBody::new(
                &nosed_tube_and_flare(cone_deg, radius_m, tube_m, 0.0, 0.0),
                DEFAULT_ELEMENTS_PER_CURVE,
            )
            .expect("the body ahead");
            let turns = flare_reduction_turns_rad(&ahead.aft_flow(mach).expect("its aft flow"))
                .expect("its crossing");
            let deg = turns.crossing_rad.to_degrees();
            let area = PI * radius_m * radius_m;
            let read = |flare_deg: f64| {
                ShockExpansionBody::new(
                    &nosed_tube_and_flare(cone_deg, radius_m, tube_m, flare_m, flare_deg),
                    DEFAULT_ELEMENTS_PER_CURVE,
                )
                .expect("a flared body")
                .slope(mach, area)
                .expect("its slope")
            };
            let (below, above) = (read(deg * (1.0 - 1e-6)), read(deg * (1.0 + 1e-6)));
            (
                above.slope_per_rad / below.slope_per_rad - 1.0,
                (above.centre_of_pressure_m - below.centre_of_pressure_m) / (2.0 * radius_m),
            )
        };
        // The tests' rocket's body, then the same body with a flare seven times as long, then
        // with the tube cut from 0.7 m to 0.1 m, then a 10° cone in front instead of an ogive.
        for (label, cone, tube, flare, mach, force, calibres) in [
            (
                "as the rocket has it",
                None,
                0.7,
                0.3,
                5.0,
                0.004_045,
                0.064,
            ),
            ("a 2 m flare", None, 0.7, 2.0, 5.0, 0.026_058, 0.761),
            ("a 0.1 m tube", None, 0.1, 0.3, 5.0, 0.043_396, 0.186),
            (
                "a 10° cone, 0.3 m tube",
                Some(10.0),
                0.3,
                0.3,
                5.0,
                0.038_235,
                0.251,
            ),
        ] {
            let (moved, shifted) = across(cone, 0.027, tube, flare, mach);
            assert!(
                (moved - force).abs() < 0.03 * force
                    && (shifted - calibres).abs() < 0.03 * calibres,
                "{label}: the crossing moves the body's slope {moved:+.6} and its centre of \
                 pressure {shifted:+.4} calibres"
            );
        }
        // And in the Mach number, on the table's own 0.05 grid. A 1° flare on the short-tubed
        // body: the rows below Mach 2.95 are the reduced ones, and the first row above them is
        // where the reading steps.
        let body = ShockExpansionBody::new(
            &nosed_tube_and_flare(None, 0.027, 0.1, 0.3, 1.0),
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .expect("the short-tubed flared body");
        let area = PI * 0.027 * 0.027;
        let row = |step: usize| {
            // The table's own grid: `SUPERSONIC_STEPS_PER_MACH` rows to the Mach number.
            let mach = step as f64 / 20.0;
            let slope = body.slope(mach, area).expect("a slope at every row");
            let flows = body.element_flows(mach).expect("a march at every row");
            (
                slope.slope_per_rad,
                slope.centre_of_pressure_m,
                flows[flows.len() - 1].decay_per_m == 0.0,
            )
        };
        for step in 56..=58 {
            assert!(row(step).2, "Mach {} should be reduced", step as f64 / 20.0);
        }
        for step in 59..=62 {
            assert!(!row(step).2, "Mach {} should not be", step as f64 / 20.0);
        }
        let (before, after) = (row(58), row(59));
        let moved = after.0 / before.0 - 1.0;
        let shifted = (after.1 - before.1) / 0.054;
        assert!(
            (moved + 0.027_72).abs() < 5e-5 && (shifted + 0.137_5).abs() < 5e-4,
            "Mach 2.90 to 2.95 moves the slope {moved:+.5} and the centre of pressure \
             {shifted:+.4} calibres"
        );
        // Its neighbours move by a fifth of that or less, so it is the pole and not the trend.
        for pair in [(56, 57), (57, 58), (59, 60), (60, 61)] {
            let step = row(pair.1).0 / row(pair.0).0 - 1.0;
            assert!(
                step.abs() < 0.2 * moved.abs(),
                "Mach {} to {} moves the slope {step:+.5}",
                pair.0 as f64 / 20.0,
                pair.1 as f64 / 20.0
            );
        }
    }

    /// The gap between the pressure behind a corner and its tangent cone's is not promised to
    /// have one zero, and where it has three the element it reduces is two bands rather than one.
    ///
    /// On a 25° cone with 0.02 m of tube behind it at Mach 7, `p₂ − p_c` vanishes at about 0.91°,
    /// 7.3° and 24°, so a 0.1 m flare is reduced from 0.91° to 3.88° **and again** from 7.3° to
    /// 24°. [`flare_reduction_turns_rad`] sweeps the widening turns before it brackets, so it
    /// reports that rather than returning whichever root it walked to.
    #[test]
    fn a_corner_whose_gap_has_three_zeros_is_refused_rather_than_guessed_at() {
        let ahead = ShockExpansionBody::new(
            &nosed_tube_and_flare(Some(25.0), 0.027, 0.02, 0.0, 0.0),
            DEFAULT_ELEMENTS_PER_CURVE,
        )
        .expect("the blunt-shouldered body");
        let aft = ahead.aft_flow(7.0).expect("its aft flow");
        let Err(AeroError::Unsupported(why)) = flare_reduction_turns_rad(&aft) else {
            panic!("a corner whose gap has three zeros should not report one band");
        };
        assert!(
            why.contains("meets its tangent cone's 3 times"),
            "it reported: {why}"
        );
        // The march's own flag either side of the second band, which is what makes it real.
        let reduced = |deg: f64| {
            ShockExpansionBody::new(
                &nosed_tube_and_flare(Some(25.0), 0.027, 0.02, 0.1, deg),
                DEFAULT_ELEMENTS_PER_CURVE,
            )
            .expect("a flared body")
            .element_flows(7.0)
            .map(|flows| flows[flows.len() - 1].decay_per_m == 0.0)
        };
        for (deg, want) in [
            (0.5, false),
            (3.0, true),
            (6.0, false),
            (10.0, true),
            (22.0, true),
            (25.0, false),
        ] {
            assert_eq!(
                reduced(deg),
                Ok(want),
                "a {deg}° flare on that body at Mach 7"
            );
        }
    }

    /// The method's flare limit is the corner's **isentropic** turn running out — the flow reaching
    /// the flare turned to Mach 1 — and not the shock detaching, which is a different angle on
    /// either side of it (M1.8e14, ADR-045 in `docs/DECISIONS.md`).
    ///
    /// Second-order shock-expansion fixes the pressure just behind a corner from the Prandtl and
    /// Meyer turn there (TN 3527 pp. 7-8, the first of eq. 3's three conditions; `ν` itself is
    /// NACA 1135 eq. 171c), so the march stops where `ν` reaches zero. The wedge's largest
    /// deflection ([`crate::blunt_tip::wedge_detachment_angle_rad`], NACA 1135) is a conservative
    /// stand-in for the flare's own boundary, not the boundary itself: a cone's shock holds to
    /// steeper angles, so only the side where the march stops **below** the wedge's angle proves
    /// anything. On this body it stops 0.18° short of it at Mach 1.5 and marches 3.5° past it at
    /// Mach 2, and which side is the tube's doing — so a march that returns a number is not on its
    /// own evidence the flare's shock is attached.
    #[test]
    fn a_flare_marches_to_the_isentropic_turn_not_to_detachment() {
        let edge_1_5 = steepest_flare_deg(1.5);
        let edge_2 = steepest_flare_deg(2.0);
        assert!(
            (edge_1_5 - 11.931_217_467_660).abs() < 1e-9,
            "Mach 1.5 edge {edge_1_5}"
        );
        assert!(
            (edge_2 - 26.471_403_089_0).abs() < 1e-9,
            "Mach 2 edge {edge_2}"
        );
        let detach = |mach: f64| {
            crate::blunt_tip::wedge_detachment_angle_rad(mach)
                .unwrap()
                .to_degrees()
        };
        // Short of detachment at Mach 1.5, past it at Mach 2: the two orders both happen.
        assert!(
            (detach(1.5) - edge_1_5 - 0.181_451).abs() < 1e-6,
            "Mach 1.5: detaches at {}, marches to {edge_1_5}",
            detach(1.5)
        );
        assert!(
            (edge_2 - detach(2.0) - 3.497_871).abs() < 1e-6,
            "Mach 2: detaches at {}, marches to {edge_2}",
            detach(2.0)
        );
        // Where they cross, bisected: below it the method stops before the shock detaches, above
        // it the method runs on past a shock that is already detached.
        let (mut lo, mut hi) = (1.5_f64, 2.0_f64);
        loop {
            let mid = 0.5 * (lo + hi);
            if mid <= lo || mid >= hi {
                break;
            }
            if steepest_flare_deg(mid) < detach(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        assert!(
            (lo - 1.547_787_962_528).abs() < 1e-9,
            "the crossing is at Mach {lo}"
        );
    }

    /// Above Mach 2.1297 the flare's limit is not the flow at all: it is where the cone tables
    /// stop (NASA SP-3007 Table 2, 30°), a limit of the reference data and not of the physics
    /// (M1.8e14, ADR-045 in `docs/DECISIONS.md`).
    ///
    /// The crossing is bisected to f64 resolution. Above it every Mach number gives the same
    /// edge, 30° plus the millionth of a degree [`cone_normal_force_slope`] admits for the
    /// degree conversion's rounding.
    #[test]
    fn past_mach_2_13_the_flare_stops_where_the_cone_tables_do() {
        let (mut lo, mut hi) = (1.5_f64, 6.0_f64);
        loop {
            let mid = 0.5 * (lo + hi);
            if mid <= lo || mid >= hi {
                break;
            }
            if steepest_flare_deg(mid) < 30.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        assert!(
            (hi - 2.129_702_032_593).abs() < 1e-9,
            "the tables bind from Mach {hi}"
        );
        // At the crossing itself the corner's turn runs out at 30° exactly; above it the tables
        // bind first, and the edge sits a millionth of a degree past 30°.
        let at_crossing = steepest_flare_deg(hi);
        assert!(
            (at_crossing - 30.0).abs() < 1e-9,
            "at the crossing the edge is {at_crossing}"
        );
        for mach in [2.13, 2.5, 3.0, 4.63, 5.0] {
            let edge = steepest_flare_deg(mach);
            assert!(
                (edge - 30.000_001).abs() < 1e-9,
                "Mach {mach}: the edge is {edge}, not the tables' 30°"
            );
            // Just past the edge: it is the tables that refuse, not the corner's turn.
            let err = flare_marches(mach, edge * (1.0 + 1e-12))
                .expect_err(&format!("Mach {mach}: {edge}° plus a part in 1e12 marched"))
                .to_string();
            assert!(err.contains("cone tables"), "Mach {mach}: {err}");
        }
        // Below the crossing it is the corner's turn that stops the march, not the tables.
        let err = flare_marches(2.0, 27.0)
            .expect_err("Mach 2: a 27° flare marched")
            .to_string();
        assert!(err.contains("can't turn through"), "Mach 2: {err}");
    }
}
