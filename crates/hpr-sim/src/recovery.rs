//! Recovery devices: what opens, when it opens, and the drag area it presents.
//!
//! A [`Device`] is a drag area ([`DeviceDrag`]) with a [`Trigger`], a lag from the trigger to line
//! stretch, and an [`Inflation`] law. A flight carries a list of them ([`crate::Simulation`]); the
//! first one to open starts the descent phase ([`crate::Phase::Descent`]), where the rocket flies
//! as a point mass under the sum of the open devices' drag areas (`docs/physics/recovery.md`).
//!
//! Streamers and tumbling bodies are drag areas too, from their own sources
//! ([`StreamerModel`], [`DeviceDrag::tumbling`]).
//!
//! Canopy data comes from T. W. Knacke, *Parachute Recovery Systems Design Manual*, NWC TP 6575
//! (1991): drag coefficients on the nominal area `S₀` from Tables 5-1 and 5-2, canopy fill
//! constants from Table 5-6, the drag-area growth exponents of Pflanz's method (Figure 5-51) and
//! the infinite-mass opening-force coefficients `C_x` from the same tables. Every number is cited
//! at its accessor, with the printed page.

use hpr_core::DVec3;
use serde::{Deserialize, Serialize};

use crate::error::SimError;

/// The smallest aspect ratio `l/w` a streamer may have. A strip wider than it is long is not a
/// streamer, and both correlations run away there: Carruthers and Filippone's `C_D → ∞` as
/// `AR → 0`, and appendix C notes its own form "obtains maximum drag for a fixed surface area at
/// the limit `l → 0`, `w → ∞`" (printed page 117).
pub const MIN_STREAMER_ASPECT_RATIO: f64 = 1.0;

/// The largest `C_D0` a canopy may be given. Knacke's printed values run from 0.30 to 0.96 on the
/// nominal area; anything above this is a drag area mistaken for a coefficient.
const MAX_CANOPY_DRAG_COEFFICIENT: f64 = 2.0;

/// A canopy type with printed data in Knacke's tables.
///
/// Solid textile canopies come from Table 5-1 (printed page 5-3), slotted ones from Table 5-2
/// (5-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CanopyType {
    /// Flat circular (solid textile).
    FlatCircular,
    /// Conical (solid textile).
    Conical,
    /// Biconical (solid textile).
    Biconical,
    /// Triconical or polyconical (solid textile).
    Triconical,
    /// Extended skirt, 10% flat (solid textile).
    ExtendedSkirt10Flat,
    /// Extended skirt, 14.3% full (solid textile).
    ExtendedSkirt14Full,
    /// Hemispherical (solid textile).
    Hemispherical,
    /// Annular (solid textile).
    Annular,
    /// Cross, or cruciform (solid textile).
    Cross,
    /// Flat (FIST) ribbon (slotted).
    FlatRibbon,
    /// Conical ribbon (slotted).
    ConicalRibbon,
    /// Ringslot (slotted).
    Ringslot,
    /// Ringsail (slotted).
    Ringsail,
}

impl CanopyType {
    /// Knacke's printed range of `C_D0`, the drag coefficient on the nominal area `S₀`
    /// (Tables 5-1 and 5-2, printed pages 5-3 and 5-4).
    #[must_use]
    pub const fn drag_coefficient_range(self) -> (f64, f64) {
        match self {
            Self::FlatCircular => (0.75, 0.80),
            Self::Conical => (0.75, 0.90),
            Self::Biconical => (0.75, 0.92),
            Self::Triconical => (0.80, 0.96),
            Self::ExtendedSkirt10Flat => (0.78, 0.87),
            Self::ExtendedSkirt14Full => (0.75, 0.90),
            Self::Hemispherical => (0.62, 0.77),
            Self::Annular => (0.85, 0.95),
            Self::Cross => (0.60, 0.85),
            Self::FlatRibbon => (0.45, 0.50),
            Self::ConicalRibbon => (0.50, 0.55),
            Self::Ringslot => (0.56, 0.65),
            Self::Ringsail => (0.75, 0.85),
        }
    }

    /// The middle of [`Self::drag_coefficient_range`], which is what hpr uses when the user gives
    /// no `C_D0`. Knacke prints a range for every type and no single value; the middle is hpr's
    /// choice, not his (ADR-012).
    #[must_use]
    pub const fn drag_coefficient(self) -> f64 {
        let (low, high) = self.drag_coefficient_range();
        0.5 * (low + high)
    }

    /// The canopy fill constant `n` of `t_f = n D₀/v` (Table 5-6, printed page 5-44, unreefed
    /// column), where Knacke prints one. `None` where the table has no unreefed value for the
    /// type, which for five of these is because it has no row there at all.
    ///
    /// Knacke's rows cover types, not every variant: the ribbon row serves both ribbon entries.
    /// (The ringsail row prints 7 unreefed; its 7 to 8 is the reefed column.)
    #[must_use]
    pub const fn fill_constant(self) -> Option<f64> {
        match self {
            Self::FlatCircular => Some(8.0),
            Self::ExtendedSkirt10Flat => Some(10.0),
            Self::ExtendedSkirt14Full => Some(12.0),
            Self::Cross => Some(11.7),
            Self::FlatRibbon | Self::ConicalRibbon => Some(14.0),
            Self::Ringslot => Some(14.0),
            Self::Ringsail => Some(7.0),
            Self::Conical
            | Self::Biconical
            | Self::Triconical
            | Self::Hemispherical
            | Self::Annular => None,
        }
    }

    /// The drag-area growth exponent `j` of `(C_D S)(t) = (C_D S)₀ (t/t_f)^j`, for the types
    /// Pflanz's method names (Figure 5-51, printed page 5-59): `j = 2` for solid cloth (flat
    /// circular, conical, extended skirt, triconical) and `j = 1` for ribbon and ringslot.
    /// `None` for the types he doesn't name, which need a measured exponent.
    #[must_use]
    pub const fn growth_exponent(self) -> Option<f64> {
        match self {
            Self::FlatCircular
            | Self::Conical
            | Self::Triconical
            | Self::ExtendedSkirt10Flat
            | Self::ExtendedSkirt14Full => Some(2.0),
            Self::FlatRibbon | Self::ConicalRibbon | Self::Ringslot => Some(1.0),
            Self::Biconical
            | Self::Hemispherical
            | Self::Annular
            | Self::Cross
            | Self::Ringsail => None,
        }
    }

    /// The infinite-mass opening-force coefficient `C_x = F_x/F_c` (Tables 5-1 and 5-2; the cross
    /// canopy's printed 1.1 to 1.2 is taken at its middle). hpr reports it; it is not used in the
    /// equations, which integrate the opening force instead.
    #[must_use]
    pub const fn opening_force_coefficient(self) -> f64 {
        match self {
            Self::FlatCircular => 1.7,
            Self::Conical | Self::Biconical | Self::Triconical => 1.8,
            Self::ExtendedSkirt10Flat | Self::ExtendedSkirt14Full | Self::Annular => 1.4,
            Self::Hemispherical => 1.6,
            Self::Cross => 1.15,
            Self::FlatRibbon | Self::ConicalRibbon | Self::Ringslot => 1.05,
            Self::Ringsail => 1.10,
        }
    }

    /// Where the numbers come from, for reports.
    #[must_use]
    pub const fn source(self) -> &'static str {
        "Knacke, Parachute Recovery Systems Design Manual, NWC TP 6575 (1991): Table 5-1 (solid \
         textile canopies), Table 5-2 (slotted), Table 5-6 (fill constants) and Figure 5-51 \
         (drag-area growth)"
    }
}

/// How a streamer's drag area is estimated. A streamer of length `l` and width `w` has a
/// planform (one-side) area `S = l w` and an aspect ratio `AR = l/w`.
///
/// The two models disagree by a factor of about four in drag area, so
/// `docs/physics/recovery.md` sets out what each is fitted to and how each compares with the only
/// free-drop data in hand (ADR-013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StreamerModel {
    /// J. Carruthers and A. Filippone, "Aerodynamic Drag of Streamers and Flags", *Journal of
    /// Aircraft* 42(4), 2005, pp. 976-982, on the planform area, from wind-tunnel tests of cotton
    /// streamers at `AR` 3.3 to 30 and 6 to 18.9 m/s. It fits one power curve per planform area:
    ///
    /// - `C_D = 0.561 AR^−0.480` at `S = 0.025 m²` (eq. 2; Figure 2's trend line reads
    ///   `0.561 AR^−0.4795`),
    /// - `C_D = 0.6514 AR^−0.6075` at `S = 0.05 m²` (the trend line on Figure 3, which the text
    ///   does not repeat as an equation),
    /// - `C_D = 0.405 AR^−0.494` at `S = 0.075 m²` (eq. 1; Figure 4 reads `0.4046 AR^−0.494`).
    ///
    /// hpr interpolates between **neighbouring** curves linearly in `ln S` and holds the end
    /// curve outside the fitted range. That interpolation is hpr's, not the paper's. All three
    /// are needed because `C_D` is far from linear in `ln S`: at `AR = 3.3` the middle curve sits
    /// 0.3% *below* the smallest area's rather than 63% of the way to the largest's, so blending
    /// only the extremes reads 18% low there.
    ///
    /// The correlations are for streamers clamped at the luff; the paper measures more drag when
    /// the luff is free, which is what a descending streamer has.
    ///
    /// This is the default: on the one free-drop measurement in hand that a flat correlation
    /// should fit (Kidwell's unpleated streamer) it is 9% fast where [`Self::OpenRocket`] is 88%
    /// fast. Both are fast on his pleated streamers, which neither models
    /// (`docs/physics/recovery.md`).
    #[default]
    Filippone,
    /// The OpenRocket technical documentation v13.05, Appendix C (printed pages 113-118):
    /// `C_Dm = 0.034 ((ρ_m + 25 g/m²)/(105 g/m²)) ((l + 1 m)/l)` on the planform area, fitted to
    /// wind-tunnel tests of model-rocket streamers (`w` 0.01 to 0.09 m, `l` 0.2 to 1.0 m, surface
    /// density 10 to 80 g/m², 6 to 12 m/s), with a stated 12 to 27% error on an independent set.
    ///
    /// Use it to compare with OpenRocket. Against Kidwell's free drops it is low in drag area by
    /// 4.2 times on his flat crêpe streamer and 7.8 times on his pleated Micafilm one, which is
    /// 88% and 154% fast in descent rate (`docs/physics/recovery.md`).
    OpenRocket,
}

impl StreamerModel {
    /// Carruthers and Filippone's three fitted curves, as `(planform area m², coefficient,
    /// exponent)` with `C_D = coefficient · AR^exponent`, in order of area.
    const FILIPPONE_CURVES: [(f64, f64, f64); 3] = [
        (0.025, 0.561, -0.480),
        (0.05, 0.6514, -0.6075),
        (0.075, 0.405, -0.494),
    ];

    /// The aspect ratios `AR = l/w` the correlations were fitted over. Outside it they only
    /// extrapolate, and as `AR → 0` the power law runs away (`C_D → ∞`), which is why a flight
    /// refuses a strip wider than it is long ([`MIN_STREAMER_ASPECT_RATIO`]).
    pub const FILIPPONE_ASPECT_RATIO_RANGE: (f64, f64) = (3.3, 30.0);

    /// The drag area `C_D S` of a streamer, m².
    ///
    /// `surface_density_kg_m2` is the fabric's, which only [`Self::OpenRocket`] uses. Outside the
    /// ranges each correlation was fitted over this extrapolates; a flight refuses the shapes
    /// that make it meaningless ([`crate::Simulation::with_recovery`]).
    #[must_use]
    pub fn drag_area_m2(self, length_m: f64, width_m: f64, surface_density_kg_m2: f64) -> f64 {
        let planform_m2 = length_m * width_m;
        match self {
            Self::Filippone => {
                let aspect_ratio = length_m / width_m;
                let curve = |(_, coefficient, exponent): (f64, f64, f64)| {
                    coefficient * aspect_ratio.powf(exponent)
                };
                let curves = Self::FILIPPONE_CURVES;
                // Between two neighbouring curves, linearly in `ln S`; outside, the end curve.
                let coefficient = if planform_m2 <= curves[0].0 {
                    curve(curves[0])
                } else if planform_m2 >= curves[2].0 {
                    curve(curves[2])
                } else {
                    let upper = usize::from(planform_m2 > curves[1].0) + 1;
                    let (low_m2, high_m2) = (curves[upper - 1].0, curves[upper].0);
                    let fraction = (planform_m2 / low_m2).ln() / (high_m2 / low_m2).ln();
                    let (low, high) = (curve(curves[upper - 1]), curve(curves[upper]));
                    low + fraction * (high - low)
                };
                coefficient * planform_m2
            }
            Self::OpenRocket => {
                0.034
                    * ((surface_density_kg_m2 + 0.025) / 0.105)
                    * ((length_m + 1.0) / length_m)
                    * planform_m2
            }
        }
    }
}

/// What gives a device its drag area `C_D S`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum DeviceDrag {
    /// A drag area given directly, m² (RocketPy's `cd_s`).
    DragArea {
        /// `C_D S`, m².
        cd_s_m2: f64,
    },
    /// A streamer: a strip of fabric `length_m` by `width_m`, whose drag area comes from
    /// `model` ([`StreamerModel`]).
    Streamer {
        /// Its length `l`, m.
        length_m: f64,
        /// Its width `w`, m.
        width_m: f64,
        /// The fabric's surface density, kg/m² ([`StreamerModel::OpenRocket`] uses it).
        surface_density_kg_m2: f64,
        /// Which correlation gives its drag area.
        #[serde(default)]
        model: StreamerModel,
    },
    /// A body descending broadside, tumbling, with no device open: the drag area of its body tubes
    /// and fins. Build it with [`DeviceDrag::tumbling`], which computes it from the airframe.
    Tumble {
        /// The drag area `C_D S`, m².
        drag_area_m2: f64,
        /// The body's side profile area, m² (for reports; it and the fin area have to add up to
        /// the drag area through the model's two coefficients, so a file that leaves one out is
        /// refused by serde rather than silently defaulting to an inconsistent device).
        body_profile_m2: f64,
        /// The effective fin area, m² (for reports).
        fin_area_m2: f64,
    },
    /// A canopy of nominal diameter `D₀` with `C_D0` on the nominal area `S₀ = π D₀²/4`
    /// (Knacke's convention, printed page 5-2).
    Canopy {
        /// The nominal diameter `D₀`, m.
        nominal_diameter_m: f64,
        /// `C_D0` on `S₀`.
        drag_coefficient: f64,
        /// The canopy type, where it is one of Knacke's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<CanopyType>,
    },
}

/// The drag coefficient of a tumbling body tube, on its side profile area (the OpenRocket
/// technical documentation v13.05, §3.5, printed page 54: fitted to 22 m drop tests of five
/// models, and half the 1.12 of a circular cylinder in crossflow, as expected of a cylinder
/// falling at a random angle).
pub const TUMBLE_BODY_DRAG_COEFFICIENT: f64 = 0.56;

/// The drag coefficient of a tumbling fin set, on its effective fin area (the same source; it
/// sits between a flat plate's 1.17 and an open hemispherical cup's 1.42, and the documentation
/// says it is the less reliable of the two).
pub const TUMBLE_FIN_DRAG_COEFFICIENT: f64 = 1.42;

/// The effective fin area of a tumbling set is one fin's area times this factor, by fin count
/// (the same source, Table 3.4, printed page 55, for 1 to 8 fins). It is a fit, not a model: it
/// is not `n` times one fin, and it is not monotonic.
pub const TUMBLE_FIN_EFFICIENCY: [f64; 8] = [0.50, 1.00, 1.50, 1.41, 1.81, 1.73, 1.90, 1.85];

impl DeviceDrag {
    /// The drag area of `assembly` tumbling: `C_D,f A_f + C_D,bt A_bt` (the OpenRocket technical
    /// documentation v13.05, §3.5, eq. 3.98 and 3.99, printed pages 53 to 55).
    ///
    /// - `A_bt` is the body's side profile area, the integral of its outer diameter along the
    ///   axis. hpr takes each body component's **end** diameters, `(d_fore + d_aft)/2 · length`,
    ///   which is exact for tubes and cones and low for a curved nose: for Valetudo's tangent
    ///   ogive it is 0.0111 m² against the true 0.0148 m², 25% low on the nose and 2.2% on the
    ///   whole body.
    /// - `A_f` is, for each fin set, one fin's planform area times the efficiency factor for its
    ///   fin count ([`TUMBLE_FIN_EFFICIENCY`]). Launch lugs and rail buttons add nothing, and an
    ///   airframe with **tube fins** is refused: they are a large part of its broadside area and
    ///   the model has no factor for them.
    ///
    /// It sums **every** stage, so it is the whole stack tumbling. For a spent booster on its
    /// own, which is what the documentation's model was written for, use
    /// [`Self::tumbling_stages`] with that body's stages.
    ///
    /// The constants were fitted to 22 m drop tests of five models 44 to 103 mm across and 6.8 to
    /// 160 g, descending at 5.0 to 6.6 m/s, and predict those terminal velocities within 3 to 14%.
    /// Bodies much larger or faster than that are outside the fit: above a Reynolds number of
    /// about 3e5 a cylinder's crossflow drag falls by roughly half (`docs/physics/recovery.md`).
    ///
    /// # Errors
    ///
    /// [`SimError::Design`] if a fin planform's area can't be computed, and [`SimError::Domain`]
    /// if the airframe presents no area at all, carries tube fins, or has a fin set of more than
    /// the eight fins Table 3.4 covers.
    pub fn tumbling(assembly: &hpr_design::Assembly) -> Result<Self, SimError> {
        Self::tumbling_stages(
            assembly,
            (0, assembly.layout.stages.len().saturating_sub(1)),
        )
    }

    /// The drag area of the stages `first..=last` of `assembly` tumbling on their own, which is
    /// what a separated body does ([`Separation`]). [`Self::tumbling`] is this over every stage.
    ///
    /// # Errors
    ///
    /// As [`Self::tumbling`].
    pub fn tumbling_stages(
        assembly: &hpr_design::Assembly,
        (first, last): (usize, usize),
    ) -> Result<Self, SimError> {
        let mut body_profile_m2 = 0.0;
        let mut fin_area_m2 = 0.0;
        for component in &assembly.layout.components {
            if !(first..=last).contains(&component.stage) {
                continue;
            }
            if let (Some(fore_m), Some(aft_m)) = (
                component.part.fore_radius_m(),
                component.part.aft_radius_m(),
            ) {
                body_profile_m2 += (fore_m + aft_m) * component.length_m;
            }
            if matches!(component.part, hpr_design::Part::TubeFinSet(_)) {
                // Tube fins are a large part of such a rocket's broadside area and the model has
                // no factor for them, so hpr refuses rather than crediting a bare tube's drag.
                return Err(SimError::Domain {
                    what: "tumbling an airframe with tube fins (the model covers body tubes and \
                           fin sets only)",
                    value: 0.0,
                });
            }
            if let hpr_design::Part::FinSet(fins) = &component.part {
                let count = fins.count as usize;
                let efficiency =
                    TUMBLE_FIN_EFFICIENCY
                        .get(count.wrapping_sub(1))
                        .ok_or(SimError::Domain {
                            what: "number of fins in a tumbling set (the fitted efficiency factors \
                               cover 1 to 8)",
                            value: fins.count.into(),
                        })?;
                fin_area_m2 += fins.planform.geometry()?.area_m2 * efficiency;
            }
        }
        let drag_area_m2 = TUMBLE_FIN_DRAG_COEFFICIENT * fin_area_m2
            + TUMBLE_BODY_DRAG_COEFFICIENT * body_profile_m2;
        if !(drag_area_m2.is_finite() && drag_area_m2 > 0.0) {
            return Err(SimError::Domain {
                what: "drag area of the tumbling airframe, m²",
                value: drag_area_m2,
            });
        }
        Ok(Self::Tumble {
            drag_area_m2,
            body_profile_m2,
            fin_area_m2,
        })
    }

    /// A streamer `length_m` by `width_m` of a fabric of `surface_density_kg_m2`, by the default
    /// model ([`StreamerModel::Filippone`]).
    #[must_use]
    pub const fn streamer(length_m: f64, width_m: f64, surface_density_kg_m2: f64) -> Self {
        Self::Streamer {
            length_m,
            width_m,
            surface_density_kg_m2,
            model: StreamerModel::Filippone,
        }
    }

    /// A canopy of `nominal_diameter_m` with its type's default `C_D0`
    /// ([`CanopyType::drag_coefficient`]).
    #[must_use]
    pub const fn canopy(kind: CanopyType, nominal_diameter_m: f64) -> Self {
        Self::Canopy {
            nominal_diameter_m,
            drag_coefficient: kind.drag_coefficient(),
            kind: Some(kind),
        }
    }

    /// The fully open drag area `C_D S`, m².
    #[must_use]
    pub fn drag_area_m2(&self) -> f64 {
        match *self {
            Self::DragArea { cd_s_m2 } => cd_s_m2,
            Self::Streamer {
                length_m,
                width_m,
                surface_density_kg_m2,
                model,
            } => model.drag_area_m2(length_m, width_m, surface_density_kg_m2),
            Self::Tumble { drag_area_m2, .. } => drag_area_m2,
            Self::Canopy {
                nominal_diameter_m,
                drag_coefficient,
                ..
            } => {
                drag_coefficient * std::f64::consts::PI * nominal_diameter_m * nominal_diameter_m
                    / 4.0
            }
        }
    }

    /// The nominal diameter `D₀`, m, where the device has one.
    #[must_use]
    pub const fn nominal_diameter_m(&self) -> Option<f64> {
        match *self {
            Self::DragArea { .. } | Self::Streamer { .. } | Self::Tumble { .. } => None,
            Self::Canopy {
                nominal_diameter_m, ..
            } => Some(nominal_diameter_m),
        }
    }

    /// The canopy type, where the device has one.
    #[must_use]
    pub const fn canopy_type(&self) -> Option<CanopyType> {
        match *self {
            Self::DragArea { .. } | Self::Streamer { .. } | Self::Tumble { .. } => None,
            Self::Canopy { kind, .. } => kind,
        }
    }
}

/// When a device's charge fires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Trigger {
    /// At apogee.
    Apogee,
    /// The first time from apogee that the centre of mass is at or below this height above the
    /// launch site, m: an altimeter's main setting. A rocket whose apogee is already below it
    /// fires at apogee.
    Altitude {
        /// The height above the launch site, m.
        height_above_ground_m: f64,
    },
    /// At a time after the first ignition, s. Charges are checked in free flight and during the
    /// descent, so a time that passes on the pad or the rail fires at rail exit.
    Time {
        /// The time after ignition, s.
        time_s: f64,
    },
    /// A motor's ejection delay after that motor's burnout. The motor is its index in
    /// [`hpr_design::Assembly::motors`], and it must have a [`hpr_motor::Delay::Seconds`] delay.
    /// As [`Self::Time`], a delay that expires before the rail exit fires there.
    MotorDelay {
        /// The motor's index.
        motor: usize,
    },
}

/// How a device's drag area grows once it is deployed.
///
/// Knacke's measurements (Figure 5-40, printed page 5-47) overshoot the steady drag area by 10 to
/// 80% near the end of filling. These laws don't: they rise to the steady value and stay there.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Inflation {
    /// The full drag area from the moment the device deploys.
    Instant,
    /// `(C_D S)(t) = (C_D S)₀ (t/t_f)^j` over a filling time `t_f` fixed in advance.
    FillingTime {
        /// The filling time `t_f`, s.
        time_s: f64,
        /// The growth exponent `j` (Pflanz: 1 for ribbon and ringslot, 2 for solid cloth).
        exponent: f64,
    },
    /// The same growth law with Knacke's filling time `t_f = n D₀/v` (printed page 5-43), where
    /// `v` is the airspeed at deployment and `n` the canopy fill constant. Needs a device with a
    /// nominal diameter.
    ///
    /// Knacke states this linear form "gives satisfactory results in the medium-velocity range of
    /// about 150 to 500 ft/s" (45.7 to 152.4 m/s, printed page 5-44). A hobby main opening at 20
    /// to 30 m/s is below that range, and his alternative there
    /// (`t_f = n D₀/v^0.85`, `n = 4.0` for solid flat circular canopies) is dimensional, so it
    /// cannot be used in SI as printed. [`Inflation::FILL_CONSTANT_RANGE_M_S`] holds the range,
    /// and `docs/physics/recovery.md` says what using it outside costs.
    FillConstant {
        /// The fill constant `n` (Table 5-6).
        constant: f64,
        /// The growth exponent `j`.
        exponent: f64,
    },
}

impl Inflation {
    /// The airspeeds at line stretch where Knacke's linear filling time `t_f = n D₀/v` is stated
    /// to hold, m/s (150 to 500 ft/s, printed page 5-44).
    pub const FILL_CONSTANT_RANGE_M_S: (f64, f64) = (45.72, 152.4);

    /// Knacke's filling time and growth exponent for `kind`, where his tables print both.
    #[must_use]
    pub const fn knacke(kind: CanopyType) -> Option<Self> {
        match (kind.fill_constant(), kind.growth_exponent()) {
            (Some(constant), Some(exponent)) => Some(Self::FillConstant { constant, exponent }),
            _ => None,
        }
    }

    /// The filling time, s, for a device of nominal diameter `diameter_m` deployed at airspeed
    /// `airspeed_m_s`. Zero means the drag area appears at once.
    fn filling_time_s(&self, diameter_m: Option<f64>, airspeed_m_s: f64) -> f64 {
        match *self {
            Self::Instant => 0.0,
            Self::FillingTime { time_s, .. } => time_s,
            Self::FillConstant { constant, .. } => match diameter_m {
                // A deployment at rest has no filling time: there is no flow to fill the canopy,
                // and `n D₀/v` diverges. The canopy opens as the rocket picks up speed instead.
                Some(diameter_m) if airspeed_m_s > 0.0 => constant * diameter_m / airspeed_m_s,
                _ => 0.0,
            },
        }
    }

    /// The growth exponent `j`.
    const fn exponent(&self) -> f64 {
        match *self {
            Self::Instant => 1.0,
            Self::FillingTime { exponent, .. } | Self::FillConstant { exponent, .. } => exponent,
        }
    }
}

/// A recovery device: a drag area, when it opens, and how it fills.
///
/// Build one with [`Device::new`] and the builders: the struct is `#[non_exhaustive]` so that
/// M1.7b's streamers and separation can add fields without breaking callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Device {
    /// A name for reports.
    pub name: String,
    /// What gives it its drag area.
    pub drag: DeviceDrag,
    /// When its charge fires.
    pub trigger: Trigger,
    /// Seconds from the trigger to line stretch, when the canopy starts to fill (RocketPy's
    /// `lag`).
    #[serde(default)]
    pub lag_s: f64,
    /// How its drag area grows from line stretch.
    #[serde(default = "instant")]
    pub inflation: Inflation,
    /// The device whose opening releases this one, by its index in the flight's list: a drogue cut
    /// away once the main is fully open. A device released before its own charge fires never
    /// deploys.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_by: Option<usize>,
    /// Which body it is attached to, after a separation ([`Separation`]): body 0 keeps the nose,
    /// body 1 is the stages aft of the split. Without a separation there is only body 0.
    #[serde(default)]
    pub body: usize,
}

fn instant() -> Inflation {
    Inflation::Instant
}

impl Device {
    /// A device with no lag, opening at once and never released.
    #[must_use]
    pub fn new(name: impl Into<String>, drag: DeviceDrag, trigger: Trigger) -> Self {
        Self {
            name: name.into(),
            drag,
            trigger,
            lag_s: 0.0,
            inflation: Inflation::Instant,
            released_by: None,
            body: 0,
        }
    }

    /// The same device with `lag_s` seconds from the trigger to line stretch.
    #[must_use]
    pub fn with_lag_s(mut self, lag_s: f64) -> Self {
        self.lag_s = lag_s;
        self
    }

    /// The same device with an inflation law.
    #[must_use]
    pub fn with_inflation(mut self, inflation: Inflation) -> Self {
        self.inflation = inflation;
        self
    }

    /// The same device, released when device `index` is fully open.
    #[must_use]
    pub fn with_release_by(mut self, index: usize) -> Self {
        self.released_by = Some(index);
        self
    }

    /// The same device, carried by body `index` after a separation ([`Separation`]).
    #[must_use]
    pub fn on_body(mut self, index: usize) -> Self {
        self.body = index;
        self
    }

    /// Checks the device's numbers.
    fn validate(&self, count: usize, index: usize) -> Result<(), SimError> {
        match self.drag {
            DeviceDrag::Streamer {
                length_m,
                width_m,
                surface_density_kg_m2,
                model,
            } => {
                for (what, value) in [
                    ("streamer length, m", length_m),
                    ("streamer width, m", width_m),
                ] {
                    if !(value.is_finite() && value > 0.0) {
                        return Err(SimError::Domain { what, value });
                    }
                }
                if !(surface_density_kg_m2.is_finite() && surface_density_kg_m2 >= 0.0) {
                    return Err(SimError::Domain {
                        what: "streamer fabric surface density, kg/m²",
                        value: surface_density_kg_m2,
                    });
                }
                // A strip wider than it is long is not a streamer, and both correlations run
                // away there. Above the fitted range they only extrapolate, which is allowed.
                let _ = model;
                if length_m / width_m < MIN_STREAMER_ASPECT_RATIO {
                    return Err(SimError::Domain {
                        what: "streamer aspect ratio, length over width (Carruthers and \
                               Filippone fit 3.3 to 30, and both correlations are meaningless \
                               below 1)",
                        value: length_m / width_m,
                    });
                }
            }
            DeviceDrag::Tumble {
                drag_area_m2,
                body_profile_m2,
                fin_area_m2,
            } => {
                for (what, value) in [
                    ("tumbling body side profile area, m²", body_profile_m2),
                    ("tumbling effective fin area, m²", fin_area_m2),
                ] {
                    if !(value.is_finite() && value >= 0.0) {
                        return Err(SimError::Domain { what, value });
                    }
                }
                let parts = TUMBLE_FIN_DRAG_COEFFICIENT * fin_area_m2
                    + TUMBLE_BODY_DRAG_COEFFICIENT * body_profile_m2;
                if (drag_area_m2 - parts).abs() > 1e-9 * drag_area_m2.abs().max(1.0) {
                    return Err(SimError::Domain {
                        what: "tumbling drag area against its body and fin areas (build one with \
                               DeviceDrag::tumbling)",
                        value: drag_area_m2,
                    });
                }
            }
            DeviceDrag::DragArea { .. } | DeviceDrag::Canopy { .. } => {}
        }
        if let DeviceDrag::Canopy {
            drag_coefficient, ..
        } = self.drag
            // Knacke's tables run from 0.30 (hemisflo ribbon) to 0.96 (triconical) on the nominal
            // area. The bound is loose enough for a coefficient measured on another reference
            // area, and tight enough to catch a drag area passed as a coefficient.
            && !(drag_coefficient.is_finite()
                && drag_coefficient > 0.0
                && drag_coefficient <= MAX_CANOPY_DRAG_COEFFICIENT)
        {
            return Err(SimError::Domain {
                what: "canopy drag coefficient on the nominal area (0 to 2]",
                value: drag_coefficient,
            });
        }
        let area = self.drag.drag_area_m2();
        if !(area.is_finite() && area > 0.0) {
            return Err(SimError::Domain {
                what: "recovery device drag area, m²",
                value: area,
            });
        }
        if let Some(diameter) = self.drag.nominal_diameter_m()
            && !(diameter.is_finite() && diameter > 0.0)
        {
            return Err(SimError::Domain {
                what: "canopy nominal diameter, m",
                value: diameter,
            });
        }
        if !(self.lag_s.is_finite() && self.lag_s >= 0.0) {
            return Err(SimError::Domain {
                what: "recovery device lag, s",
                value: self.lag_s,
            });
        }
        match self.inflation {
            Inflation::Instant => {}
            Inflation::FillingTime { time_s, exponent } => {
                if !(time_s.is_finite() && time_s >= 0.0) {
                    return Err(SimError::Domain {
                        what: "canopy filling time, s",
                        value: time_s,
                    });
                }
                check_exponent(exponent)?;
            }
            Inflation::FillConstant { constant, exponent } => {
                if !(constant.is_finite() && constant > 0.0) {
                    return Err(SimError::Domain {
                        what: "canopy fill constant",
                        value: constant,
                    });
                }
                if self.drag.nominal_diameter_m().is_none() {
                    return Err(SimError::Domain {
                        what: "canopy fill constant without a nominal diameter (give a canopy, \
                               or a filling time)",
                        value: constant,
                    });
                }
                check_exponent(exponent)?;
            }
        }
        match self.trigger {
            Trigger::Apogee | Trigger::MotorDelay { .. } => {}
            Trigger::Altitude {
                height_above_ground_m,
            } => {
                if !(height_above_ground_m.is_finite() && height_above_ground_m > 0.0) {
                    return Err(SimError::Domain {
                        what: "deployment height above the launch site, m",
                        value: height_above_ground_m,
                    });
                }
            }
            Trigger::Time { time_s } => {
                if !(time_s.is_finite() && time_s >= 0.0) {
                    return Err(SimError::Domain {
                        what: "deployment time after ignition, s",
                        value: time_s,
                    });
                }
            }
        }
        if let Some(other) = self.released_by
            && (other >= count || other == index)
        {
            return Err(SimError::Domain {
                what: "index of the device that releases this one",
                value: other as f64,
            });
        }
        Ok(())
    }
}

fn check_exponent(exponent: f64) -> Result<(), SimError> {
    if exponent.is_finite() && exponent > 0.0 {
        Ok(())
    } else {
        Err(SimError::Domain {
            what: "canopy drag-area growth exponent",
            value: exponent,
        })
    }
}

/// A separation: the stack comes apart at a stage boundary and every body descends under its own
/// devices (`docs/physics/recovery.md`, ADR-014).
///
/// Bodies are contiguous runs of stages. Stages `0..=after_stage` keep the nose and are body 0;
/// the stages aft of the split are body 1. Each body flies as a point mass from the separation,
/// with the mass properties of its own stages and motors, so every body must carry at least one
/// device: the descent phase has no airframe drag (ADR-012), and a body with nothing open would
/// fall as if in a vacuum.
///
/// A separation is an ideal one: no impulse, so each body leaves with the velocity its own centre
/// of mass already had. It must come after the last burnout, because a body's mass is taken as
/// constant through its descent; powered staging is M1.9.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Separation {
    /// When the stack comes apart.
    pub trigger: Trigger,
    /// The last stage that stays with the nose. Stages after it form the aft body.
    pub after_stage: usize,
}

impl Separation {
    /// A separation at the boundary after `after_stage`, on `trigger`.
    #[must_use]
    pub const fn new(trigger: Trigger, after_stage: usize) -> Self {
        Self {
            trigger,
            after_stage,
        }
    }

    /// The stages of body `index`: body 0 keeps the nose, body 1 is the rest.
    #[must_use]
    pub const fn stages_of(&self, index: usize, stage_count: usize) -> Option<(usize, usize)> {
        match index {
            0 => Some((0, self.after_stage)),
            1 if self.after_stage + 1 < stage_count => {
                Some((self.after_stage + 1, stage_count - 1))
            }
            _ => None,
        }
    }

    /// How many bodies it makes: two.
    pub const BODIES: usize = 2;
}

/// The mass properties of the stages `first..=last` of `assembly`, with their motors, `t_s`
/// seconds after ignition. Summing over every stage gives [`hpr_design::Assembly::mass_properties`].
pub(crate) fn body_mass_properties(
    assembly: &hpr_design::Assembly,
    (first, last): (usize, usize),
    t_s: f64,
) -> hpr_design::MassProperties {
    let mut parts = vec![];
    for (index, stage) in assembly.layout.stages.iter().enumerate() {
        if (first..=last).contains(&index) {
            parts.push(stage.mass);
        }
    }
    let motors: Vec<_> = assembly
        .motors
        .iter()
        .filter(|motor| (first..=last).contains(&motor.stage))
        .map(|motor| motor.mass_properties(t_s))
        .collect();
    parts.extend(motors);
    hpr_design::MassProperties::combine(parts.iter())
}

/// Checks a flight's devices, and finds the trigger times that are known before it flies.
///
/// The result has one entry per device: `Some(t)` for a [`Trigger::Time`] or a
/// [`Trigger::MotorDelay`], `None` for the triggers the flight has to watch for.
pub(crate) fn plan(
    devices: &[Device],
    motors: &[hpr_design::PlacedMotor],
) -> Result<Vec<Option<f64>>, SimError> {
    // A cycle of releases (A released by B, B released by A) can leave every device released and
    // the rocket falling under nothing at all, which the descent phase would fly as a vacuum drop
    // (found in review). Each chain has to end.
    for start in 0..devices.len() {
        let mut at = start;
        for _ in 0..devices.len() {
            match devices[at].released_by {
                Some(next) if next < devices.len() => at = next,
                _ => break,
            }
            if at == start {
                return Err(SimError::Domain {
                    what: "recovery device releases run in a cycle, so every one of them could \
                           be released at once; index of a device in the cycle",
                    value: start as f64,
                });
            }
        }
    }
    let mut times = Vec::with_capacity(devices.len());
    for (index, device) in devices.iter().enumerate() {
        device.validate(devices.len(), index)?;
        let time = match device.trigger {
            Trigger::Time { time_s } => Some(time_s),
            Trigger::MotorDelay { motor } => {
                let placed = motors.get(motor).ok_or(SimError::Domain {
                    what: "index of the motor whose delay fires a device",
                    value: motor as f64,
                })?;
                let delay_s = match placed.mounted.delay {
                    Some(hpr_motor::Delay::Seconds(delay_s)) => delay_s,
                    _ => {
                        return Err(SimError::Domain {
                            what: "the motor firing a device has no ejection delay in seconds \
                                   (it is plugged, or its delay is unset)",
                            value: motor as f64,
                        });
                    }
                };
                if !(delay_s.is_finite() && delay_s >= 0.0) {
                    return Err(SimError::Domain {
                        what: "motor ejection delay, s",
                        value: delay_s,
                    });
                }
                Some(placed.mounted.motor.burnout_time_s() + delay_s)
            }
            Trigger::Apogee | Trigger::Altitude { .. } => None,
        };
        times.push(time);
    }
    Ok(times)
}

/// One device's progress through a flight.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct DeviceRun {
    /// When its charge fired, s after ignition.
    pub(crate) triggered_s: Option<f64>,
    /// When it will deploy (line stretch), s: the trigger plus the lag.
    pub(crate) deploy_s: Option<f64>,
    /// When it deployed (line stretch), s.
    pub(crate) deployed_s: Option<f64>,
    /// Its filling time, s, fixed at deployment.
    pub(crate) filling_time_s: f64,
    /// When it is released, s: the time the device that releases it is fully open.
    pub(crate) released_s: Option<f64>,
    /// Whether that release has been recorded.
    pub(crate) release_recorded: bool,
    /// Whether it was cut away before its own charge fired, so it never deploys.
    pub(crate) abandoned: bool,
}

/// Every device's progress through one flight. The [`crate::Simulation`] is not mutated by a run
/// (Loft lesson L24), so this lives with the flight.
///
/// Every method here indexes `devices` by a device's position in the flight's list, which is the
/// invariant [`Run::new`] establishes: a run is built with one entry per device and is only ever
/// passed the same slice. Indexing therefore cannot be out of range, and a caller that broke that
/// (a future per-body run, M1.7b) would panic here rather than silently pull the wrong canopy.
#[derive(Debug, Clone, Default)]
pub(crate) struct Run {
    pub(crate) devices: Vec<DeviceRun>,
}

impl Run {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            devices: vec![DeviceRun::default(); count],
        }
    }

    /// Fires device `index`'s charge at `t`, and returns when it will deploy.
    pub(crate) fn trigger(&mut self, devices: &[Device], index: usize, t: f64) -> f64 {
        let deploy_s = t + devices[index].lag_s;
        self.devices[index].triggered_s = Some(t);
        self.devices[index].deploy_s = Some(deploy_s);
        deploy_s
    }

    /// Deploys device `index` at `t` with airspeed `airspeed_m_s`, schedules the release of
    /// whatever it releases for the moment its own canopy is full, and returns that time.
    pub(crate) fn deploy(
        &mut self,
        devices: &[Device],
        index: usize,
        t: f64,
        airspeed_m_s: f64,
    ) -> f64 {
        let device = &devices[index];
        let filling_time_s = device
            .inflation
            .filling_time_s(device.drag.nominal_diameter_m(), airspeed_m_s);
        self.devices[index].deployed_s = Some(t);
        self.devices[index].filling_time_s = filling_time_s;
        let full_s = t + filling_time_s;
        for (other, run) in devices.iter().zip(&mut self.devices) {
            if other.released_by == Some(index) && run.released_s.is_none() {
                run.released_s = Some(full_s);
            }
        }
        full_s
    }

    /// When device `index` is released, s.
    pub(crate) fn released_s(&self, index: usize) -> Option<f64> {
        self.devices[index].released_s
    }

    /// Whether device `index`'s release has come and has not been recorded.
    pub(crate) fn release_due(&self, index: usize, t: f64) -> bool {
        let run = self.devices[index];
        !run.release_recorded && run.released_s.is_some_and(|released| t >= released)
    }

    /// Marks device `index`'s release as recorded.
    pub(crate) fn release(&mut self, index: usize) {
        self.devices[index].release_recorded = true;
    }

    /// Gives up on device `index`, which was cut away before its charge fired.
    pub(crate) fn abandon(&mut self, index: usize) {
        self.devices[index].abandoned = true;
    }

    /// Whether device `index` has been triggered but has not deployed or been abandoned.
    pub(crate) fn waiting(&self, index: usize) -> bool {
        let run = self.devices[index];
        run.triggered_s.is_some() && run.deployed_s.is_none() && !run.abandoned
    }

    /// When device `index` deploys, s: infinite until its charge fires.
    pub(crate) fn deploy_s(&self, index: usize) -> f64 {
        self.devices[index].deploy_s.unwrap_or(f64::INFINITY)
    }

    /// Whether device `index` is still waiting for its trigger.
    pub(crate) fn pending(&self, index: usize) -> bool {
        self.devices[index].triggered_s.is_none()
    }

    /// The total drag area of body `body`'s open devices at `t`, m².
    pub(crate) fn body_drag_area_m2(&self, devices: &[Device], body: usize, t: f64) -> f64 {
        devices
            .iter()
            .enumerate()
            .filter(|(_, device)| device.body == body)
            .map(|(index, _)| self.one_drag_area_m2(devices, index, t))
            .sum()
    }

    /// The total drag area of the open devices at `t`, m².
    ///
    /// A device deployed at `t_d` with filling time `t_f` and growth exponent `j` contributes
    /// `(C_D S)₀ min(1, (t − t_d)/t_f)^j`, and nothing once it is released.
    pub(crate) fn drag_area_m2(&self, devices: &[Device], t: f64) -> f64 {
        (0..devices.len())
            .map(|index| self.one_drag_area_m2(devices, index, t))
            .sum()
    }

    /// The drag area of device `index` at `t`, m²: nothing before it deploys or after it is
    /// released, and its inflation law in between.
    fn one_drag_area_m2(&self, devices: &[Device], index: usize, t: f64) -> f64 {
        let (device, run) = (&devices[index], self.devices[index]);
        let Some(deployed_s) = run.deployed_s else {
            return 0.0;
        };
        if run.released_s.is_some_and(|released| t >= released) {
            return 0.0;
        }
        let full = device.drag.drag_area_m2();
        let elapsed = t - deployed_s;
        if elapsed < 0.0 {
            return 0.0;
        }
        if run.filling_time_s <= 0.0 {
            return full;
        }
        let fraction = (elapsed / run.filling_time_s).min(1.0);
        // `j` is 1 (ribbon and ringslot) or 2 (solid cloth) for every canopy Knacke's method
        // names, so the common cases avoid `powf`.
        let growth = match device.inflation.exponent() {
            1.0 => fraction,
            2.0 => fraction * fraction,
            exponent => fraction.powf(exponent),
        };
        full * growth
    }
}

/// One separated body at an instant of its descent.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodySample {
    /// Time since the first ignition, s.
    pub time_s: f64,
    /// Its centre of mass in the launch frame, m.
    pub cg_enu_m: DVec3,
    /// That point's velocity relative to the launch frame, m/s.
    pub velocity_enu_m_s: DVec3,
    /// Its centre of mass's ellipsoidal height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The rate of that height, m/s.
    pub vertical_speed_m_s: f64,
    /// Its airspeed, m/s.
    pub airspeed_m_s: f64,
    /// The drag area `C_D S` of its open devices, m².
    pub recovery_drag_area_m2: f64,
    /// Its mass, kg.
    pub mass_kg: f64,
}

/// An event during a separated body's descent, with the body at that instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyEvent {
    /// What happened: a device's [`crate::EventKind::Trigger`], [`crate::EventKind::Deployment`],
    /// [`crate::EventKind::Release`] or the body's [`crate::EventKind::GroundHit`].
    pub kind: crate::EventKind,
    /// The body at that instant.
    pub sample: BodySample,
}

/// One separated body's descent, from the separation to its landing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyFlight {
    /// Which body: 0 keeps the nose.
    pub body: usize,
    /// The stages it is made of, inclusive.
    pub stages: (usize, usize),
    /// Its mass, kg, constant through the descent.
    pub mass_kg: f64,
    /// Where it started: the separation, with its own centre of mass and that point's velocity.
    pub start_sample: BodySample,
    /// Why its descent ended.
    pub termination: crate::Termination,
    /// Its events, in order.
    pub events: Vec<BodyEvent>,
    /// Where it ended.
    pub final_sample: BodySample,
    /// The integrator's work on it.
    pub stats: crate::Stats,
}

impl BodyFlight {
    /// The first event of `kind`.
    #[must_use]
    pub fn event(&self, kind: crate::EventKind) -> Option<&BodyEvent> {
        self.events.iter().find(|event| event.kind == kind)
    }
}

/// The equilibrium descent speed `v_e = √(2 m g/(ρ C_D S))`, m/s (Knacke, printed page 5-128).
///
/// It is the speed at which the drag area's drag balances the weight, so it is also the speed a
/// long descent settles at.
///
/// The formula is evaluated as written, with no domain checks: a zero drag area or density gives
/// infinity, and a negative mass, density, drag area or gravity gives NaN. Flights validate their
/// devices instead ([`crate::Simulation::with_recovery`]).
#[must_use]
pub fn terminal_speed_m_s(
    mass_kg: f64,
    drag_area_m2: f64,
    density_kg_m3: f64,
    gravity_m_s2: f64,
) -> f64 {
    (2.0 * mass_kg * gravity_m_s2 / (density_kg_m3 * drag_area_m2)).sqrt()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use hpr_atmos::ConstantWind;
    use hpr_core::DVec3;

    use super::*;
    use crate::environment::Environment;
    use crate::flight::{EventKind, FlightResult, FlightSettings, Simulation, Termination};
    use crate::rail::Rail;
    use crate::recorder::{Channel, Recorder};
    use crate::state::State;
    use crate::testing::{
        QuadraticDragFall, UniformAir, analytic_environment, analytic_wind_environment,
        closed_form_quadratic_drag, design, rotating_analytic_environment,
    };

    const G: f64 = 9.806_65;
    /// Valetudo's motor burns out at 3.26 s; the descents start well after that.
    const START_S: f64 = 10.0;

    /// Valetudo with `devices`, an hour's cap and a rail it never uses.
    fn flight(environment: Environment, devices: Vec<Device>, max_time_s: f64) -> Simulation {
        Simulation::new(
            &design("rocketpy-valetudo"),
            "example",
            environment,
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices)
        .unwrap()
    }

    /// The same rocket with an ejection delay of `delay_s` on every motor of every configuration.
    fn with_delay(mut rocket: hpr_design::Rocket, delay_s: f64) -> hpr_design::Rocket {
        for configuration in &mut rocket.configurations {
            for motor in &mut configuration.motors {
                motor.delay = Some(hpr_motor::Delay::Seconds(delay_s));
            }
        }
        rocket
    }

    /// A device open from the moment the descent starts.
    fn open_at_start(drag: DeviceDrag) -> Device {
        Device::new("test", drag, Trigger::Time { time_s: START_S })
    }

    /// The state at rest (`velocity_enu_m_s`) with the centre of mass `height_m` above the site,
    /// nose up.
    fn dropped(sim: &Simulation, height_m: f64, velocity_enu_m_s: DVec3) -> State {
        let attitude = Rail::vertical(3.0).attitude();
        let cg_m = sim.assembly().mass_properties(START_S).cg_m;
        State {
            position_enu_m: DVec3::new(0.0, 0.0, height_m) - attitude.mul_vec3(cg_m),
            velocity_enu_m_s,
            attitude,
            body_rate_rad_s: DVec3::ZERO,
        }
    }

    /// The column `name` of every row.
    fn column(recorder: &Recorder, name: &str) -> Vec<f64> {
        let index = recorder
            .columns()
            .iter()
            .position(|c| c == name)
            .unwrap_or_else(|| panic!("no column {name}"));
        recorder.rows().iter().map(|row| row[index]).collect()
    }

    /// The largest canopy drag force of a flight, N, over the event samples and the recorded
    /// steps: `q (C_D S)`.
    fn peak_load_n(result: &FlightResult, recorder: &Recorder) -> f64 {
        let from_events = result
            .events
            .iter()
            .map(|event| event.sample.dynamic_pressure_pa * event.sample.recovery_drag_area_m2);
        let pressure = column(recorder, "dynamic_pressure_pa");
        let area = column(recorder, "recovery_drag_area_m2");
        let from_rows = pressure.iter().zip(&area).map(|(q, s)| q * s);
        from_events.chain(from_rows).fold(0.0, f64::max)
    }

    #[test]
    fn default_canopy_cd_carries_its_citation() {
        // Loft lesson L29. Loft's parachute C_D 0.8 was copied out of OpenRocket's GPL source.
        // hpr's default is the middle of Knacke's printed range for the type, on the nominal area
        // S₀ = π D₀²/4, and says where it comes from. RocketPy's 1.4 is not a C_D0 at all: it is a
        // hemispherical canopy's coefficient on the projected area, and Knacke's hemispherical
        // range on S₀ is 0.62 to 0.77.
        let flat = CanopyType::FlatCircular;
        assert_eq!(flat.drag_coefficient_range(), (0.75, 0.80));
        assert_eq!(flat.drag_coefficient(), 0.775);
        let (low, high) = flat.drag_coefficient_range();
        assert!(low <= flat.drag_coefficient() && flat.drag_coefficient() <= high);
        let source = flat.source();
        for cited in ["Knacke", "NWC TP 6575", "Table 5-1"] {
            assert!(source.contains(cited), "{source} does not cite {cited}");
        }
        assert_eq!(
            CanopyType::Hemispherical.drag_coefficient_range(),
            (0.62, 0.77)
        );
        assert!(CanopyType::Hemispherical.drag_coefficient() < 1.4);
        // The drag area is C_D0 on the nominal area, not on the projected area.
        let canopy = DeviceDrag::canopy(flat, 2.0);
        assert!((canopy.drag_area_m2() - 0.775 * PI).abs() < 1e-15);
        assert_eq!(canopy.nominal_diameter_m(), Some(2.0));
        assert_eq!(canopy.canopy_type(), Some(flat));
        // Knacke's tables as transcribed, entry by entry: the `C_D0` range (Tables 5-1 and 5-2),
        // the unreefed fill constant (Table 5-6, `None` where the table prints "insufficient
        // data"), the drag-area growth exponent (Pflanz, Figure 5-51, `None` for the types he
        // does not name) and the infinite-mass opening-force coefficient `C_x`. A typo in any of
        // these changes a user's descent rate, so they are pinned literally.
        let table = [
            (
                CanopyType::FlatCircular,
                0.75,
                0.80,
                Some(8.0),
                Some(2.0),
                1.7,
            ),
            (CanopyType::Conical, 0.75, 0.90, None, Some(2.0), 1.8),
            (CanopyType::Biconical, 0.75, 0.92, None, None, 1.8),
            (CanopyType::Triconical, 0.80, 0.96, None, Some(2.0), 1.8),
            (
                CanopyType::ExtendedSkirt10Flat,
                0.78,
                0.87,
                Some(10.0),
                Some(2.0),
                1.4,
            ),
            (
                CanopyType::ExtendedSkirt14Full,
                0.75,
                0.90,
                Some(12.0),
                Some(2.0),
                1.4,
            ),
            (CanopyType::Hemispherical, 0.62, 0.77, None, None, 1.6),
            (CanopyType::Annular, 0.85, 0.95, None, None, 1.4),
            (CanopyType::Cross, 0.60, 0.85, Some(11.7), None, 1.15),
            (
                CanopyType::FlatRibbon,
                0.45,
                0.50,
                Some(14.0),
                Some(1.0),
                1.05,
            ),
            (
                CanopyType::ConicalRibbon,
                0.50,
                0.55,
                Some(14.0),
                Some(1.0),
                1.05,
            ),
            (
                CanopyType::Ringslot,
                0.56,
                0.65,
                Some(14.0),
                Some(1.0),
                1.05,
            ),
            (CanopyType::Ringsail, 0.75, 0.85, Some(7.0), None, 1.10),
        ];
        for (kind, low, high, fill_constant, growth_exponent, opening) in table {
            assert_eq!(kind.drag_coefficient_range(), (low, high), "{kind:?}");
            assert_eq!(kind.drag_coefficient(), 0.5 * (low + high), "{kind:?}");
            assert_eq!(kind.fill_constant(), fill_constant, "{kind:?}");
            assert_eq!(kind.growth_exponent(), growth_exponent, "{kind:?}");
            assert_eq!(kind.opening_force_coefficient(), opening, "{kind:?}");
            assert!(
                source.contains("Knacke") && kind.source() == source,
                "{kind:?}"
            );
        }

        assert_eq!(Inflation::knacke(CanopyType::Hemispherical), None);
        assert_eq!(
            Inflation::knacke(CanopyType::FlatCircular),
            Some(Inflation::FillConstant {
                constant: 8.0,
                exponent: 2.0
            })
        );
    }

    #[test]
    fn descent_rate_equals_terminal_velocity() {
        // Loft lesson L92. Loft's own case, recomputed from Knacke's equilibrium descent speed
        // v_e = √(2 W/(ρ C_D0 S₀)) (printed page 5-128): 1.1 kg under a 1 m flat canopy of C_D 0.8
        // at ρ = 1.225 comes to 5.294 m/s. Loft allowed ±30% against it; hpr's formula has to
        // print it.
        let cd_s = 0.8 * PI * 1.0 * 1.0 / 4.0;
        let loft = terminal_speed_m_s(1.1, cd_s, 1.225, G);
        assert!((loft - 5.294).abs() < 5e-4, "{loft}");

        // A flight under the same law: dropped from rest with the canopy already open, in uniform
        // air under constant gravity, the descent is the closed-form fall under quadratic drag,
        // v = −v_t tanh(g t/v_t), and it lands at the speed v_t that the formula gives.
        let air = UniformAir::sea_level();
        let rho = air.0.density_kg_m3;
        let device = open_at_start(DeviceDrag::canopy(CanopyType::FlatCircular, 1.5));
        let drag_area_m2 = device.drag.drag_area_m2();
        let sim = flight(analytic_environment(air, G), vec![device], 3600.0);
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, rho, G);
        let height_m = 2_000.0;
        let closed = closed_form_quadratic_drag(
            &QuadraticDragFall {
                gravity_mps2: G,
                k_per_m: rho * drag_area_m2 / (2.0 * mass_kg),
            },
            0.0,
        );
        assert!((closed.apogee_s).abs() < 1e-15);

        let mut recorder = Recorder::new(
            vec![
                Channel::Time,
                Channel::VerticalSpeed,
                Channel::HeightAboveGround,
            ],
            Some(5.0),
        )
        .unwrap();
        let result = sim
            .run_free(START_S, dropped(&sim, height_m, DVec3::ZERO), &mut recorder)
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        assert_eq!(result.final_sample.phase, crate::Phase::Descent);

        // The whole descent follows the closed form, and the impact speed is the terminal speed
        // (reached to 1e-9 of it after 2 km).
        let times = column(&recorder, "time_s");
        let speeds = column(&recorder, "vertical_speed_m_s");
        let mut worst: f64 = 0.0;
        for (t, v) in times.iter().zip(&speeds) {
            let expected = closed.state(t - START_S)[1];
            worst = worst.max((v - expected).abs() / terminal_m_s);
        }
        // Measured: 2.1e-8 of v_t, the integrator's own error at rtol = atol = 1e-8.
        assert!(worst < 1e-7, "{worst} of v_t");
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-7,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
        // And it lands when the closed form says, to 1e-5 s of a 200 s descent.
        let expected_s = START_S + closed.time_at_descending_height_s(-height_m);
        assert!(
            (landing.time_s - expected_s).abs() < 1e-5,
            "{} vs {expected_s}",
            landing.time_s
        );
    }

    #[test]
    fn drift_equals_the_wind_times_the_descent_time() {
        // Dropped into a steady wind with the same horizontal velocity as the air, the rocket has
        // no crossflow: the horizontal equation holds v = w for the whole descent, so the drift is
        // exactly the wind times the time of flight, and the vertical fall is unchanged.
        let air = UniformAir::sea_level();
        let device = open_at_start(DeviceDrag::canopy(CanopyType::FlatCircular, 1.5));
        let drag_area_m2 = device.drag.drag_area_m2();
        let sim = flight(
            analytic_wind_environment(air, G, ConstantWind::new(6.5, 0.7).unwrap()),
            vec![device],
            3600.0,
        );
        let wind_enu = sim.environment().wind.wind(0.0).unwrap().velocity_enu_m_s;
        assert!(wind_enu.z == 0.0 && wind_enu.length() > 6.4, "{wind_enu}");
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let closed = closed_form_quadratic_drag(
            &QuadraticDragFall {
                gravity_mps2: G,
                k_per_m: air.0.density_kg_m3 * drag_area_m2 / (2.0 * mass_kg),
            },
            0.0,
        );
        let height_m = 1_000.0;
        let start = dropped(&sim, height_m, wind_enu);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;

        let flown_s = landing.time_s - START_S;
        let drift =
            landing.cg_enu_m - start.point_enu_m(sim.assembly().mass_properties(START_S).cg_m);
        let expected = wind_enu * flown_s;
        assert!(
            (drift.x - expected.x).abs() < 1e-8 * expected.x.abs()
                && (drift.y - expected.y).abs() < 1e-8 * expected.y.abs(),
            "{drift} vs {expected}"
        );
        // The wind doesn't change the fall: the descent takes what the closed form says, to 1e-4
        // of it (the drift of 1.5 km costs 0.13 m of ellipsoidal height, which the closed form
        // over a flat Earth doesn't have).
        let expected_s = closed.time_at_descending_height_s(-height_m);
        assert!(
            (flown_s - expected_s).abs() < 1e-4 * expected_s,
            "{flown_s} vs {expected_s}"
        );
        assert!(drift.length() > 600.0, "{drift}");
    }

    #[test]
    fn coriolis_drifts_a_descent_east_by_the_analytic_amount() {
        // Every other analytic descent here runs with Earth's rotation off. With it on, a body
        // falling at `v_t` feels `−2Ω × v = 2Ω v_t cos φ` to the east, which the canopy's drag
        // balances at `v_east = 2Ω cos φ · m/(½ρ C_D S) = 2Ω cos φ · v_t²/g`. The drift is that
        // times the time of flight, once the fall has settled.
        let air = UniformAir::sea_level();
        let device = open_at_start(DeviceDrag::canopy(CanopyType::FlatCircular, 1.5));
        let drag_area_m2 = device.drag.drag_area_m2();
        let sim = flight(rotating_analytic_environment(air, G), vec![device], 3600.0);
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        let latitude_rad = sim.environment().site().latitude_rad;
        let rate_rad_s = 7.292_115e-5;
        let east_m_s = 2.0 * rate_rad_s * latitude_rad.cos() * terminal_m_s * terminal_m_s / G;
        assert!(east_m_s > 1e-3, "{east_m_s} m/s is too small to measure");

        let height_m = 3_000.0;
        let start = dropped(&sim, height_m, DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        let flown_s = landing.time_s - START_S;
        let drift =
            landing.cg_enu_m - start.point_enu_m(sim.assembly().mass_properties(START_S).cg_m);
        let expected_m = east_m_s * flown_s;
        // The drift builds up over the first few seconds, while the fall settles, so the
        // prediction is the steady one. Measured: 0.3666 m east against the steady prediction's
        // 0.3685 m over 306.0 s, and 29 µm north.
        assert!(
            (drift.x - expected_m).abs() < 0.05 * expected_m,
            "{} m east against {expected_m} m in {flown_s} s",
            drift.x
        );
        assert!(
            drift.y.abs() < 0.02 * expected_m,
            "{} m north, which should be second order",
            drift.y
        );
        // The fall itself is the same as without rotation, to the size of the drift's effect.
        let without = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        assert!(
            (-landing.vertical_speed_m_s / without - 1.0).abs() < 1e-5,
            "{} vs {without}",
            landing.vertical_speed_m_s
        );
    }

    #[test]
    fn inflation_time_limits_peak_opening_load() {
        // Loft lesson L27. Loft's canopies opened instantly, so its peak load was the whole
        // steady drag at deployment speed. Knacke's filling time t_f = n D₀/v (printed page 5-43)
        // with the drag area growing as (t/t_f)^j (Pflanz, Figure 5-51) spreads the opening out:
        // the canopy builds its area while the rocket is already slowing down.
        //
        // hpr does not model Knacke's measured overshoot (C_x = 1.7 for a flat circular canopy at
        // infinite mass), so the instant opening is hpr's upper bound on the load.
        let air = UniformAir::sea_level();
        let rho = air.0.density_kg_m3;
        let diameter_m = 1.5;
        let fall = DVec3::new(0.0, 0.0, -60.0);
        let mut loads = Vec::new();
        let mut mass_kg = 0.0;
        for inflation in [
            Inflation::Instant,
            Inflation::knacke(CanopyType::FlatCircular).unwrap(),
        ] {
            let device = open_at_start(DeviceDrag::canopy(CanopyType::FlatCircular, diameter_m))
                .with_inflation(inflation);
            let drag_area_m2 = device.drag.drag_area_m2();
            let sim = flight(analytic_environment(air, G), vec![device], 3600.0);
            let mut recorder = Recorder::new(
                vec![
                    Channel::Time,
                    Channel::DynamicPressure,
                    Channel::RecoveryDragArea,
                    Channel::VerticalSpeed,
                ],
                None,
            )
            .unwrap();
            let result = sim
                .run_free(START_S, dropped(&sim, 2_000.0, fall), &mut recorder)
                .unwrap();
            assert_eq!(result.termination, Termination::GroundHit);
            mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
            let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, rho, G);
            let landing = result.event(EventKind::GroundHit).unwrap().sample;
            assert!(
                (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
                "{}",
                landing.vertical_speed_m_s
            );
            loads.push((peak_load_n(&result, &recorder), drag_area_m2));
        }
        let (instant_n, drag_area_m2) = loads[0];
        let (filled_n, _) = loads[1];
        // The instant opening's peak is the steady drag at the deployment speed, at deployment.
        let steady_n = 0.5 * rho * drag_area_m2 * fall.z * fall.z;
        assert!(
            (instant_n - steady_n).abs() < 1e-6 * steady_n,
            "{instant_n} vs {steady_n}"
        );
        // Filling in t_f = n D₀/v = 8 · 1.5/60 = 0.2 s cuts the peak to 1,615 N, 0.53 of the
        // instant opening. That is pinned against the closed form rather than a fraction, because
        // a fraction cannot tell the growth exponents apart (`j = 1` would peak near 0.42).
        //
        // Ignoring gravity, `m dv/dt = −½ρ(C_D S)(t/t_f)² v²` separates to
        // `1/v = 1/v₀ + k t³/(3 t_f²)` with `k = ρ(C_D S)/2m`, so at the end of filling
        // `v = v₀/(1 + k v₀ t_f/3)` and the peak load is `½ρ(C_D S) v²` there (the load rises
        // while the canopy grows and falls once it is full). Gravity adds at most `g t_f` to that
        // speed, which is where the upper bound comes from.
        let k_per_m = rho * drag_area_m2 / (2.0 * mass_kg);
        let filling_s = CanopyType::FlatCircular.fill_constant().unwrap() * diameter_m / -fall.z;
        let end_m_s = -fall.z / (1.0 + k_per_m * -fall.z * filling_s / 3.0);
        let closed_n = 0.5 * rho * drag_area_m2 * end_m_s * end_m_s;
        let with_gravity_n = closed_n * (1.0 + G * filling_s / end_m_s).powi(2);
        assert!(
            (closed_n..=with_gravity_n).contains(&filled_n),
            "{filled_n} N is outside {closed_n} to {with_gravity_n} N (t_f {filling_s} s)"
        );
        assert!(filled_n < 0.6 * instant_n, "{filled_n} against {instant_n}");
    }

    #[test]
    fn a_whole_flight_deploys_a_drogue_at_apogee_and_a_main_that_releases_it() {
        // A flight from the pad: the drogue's charge fires at apogee and opens 1 s later, the main
        // fires at 300 m above the site and opens 1.5 s later, and the main's opening releases the
        // drogue. The events have to come in that order, the drag area has to follow, and each
        // stage of the descent has to settle at its own terminal speed.
        let devices = vec![
            Device::new(
                "drogue",
                DeviceDrag::DragArea { cd_s_m2: 0.45 },
                Trigger::Apogee,
            )
            .with_lag_s(1.0)
            .with_release_by(1),
            Device::new(
                "main",
                DeviceDrag::canopy(CanopyType::FlatCircular, 2.5),
                Trigger::Altitude {
                    height_above_ground_m: 300.0,
                },
            )
            .with_lag_s(1.5),
        ];
        let drogue_m2 = devices[0].drag.drag_area_m2();
        let main_m2 = devices[1].drag.drag_area_m2();
        let sim = flight(
            analytic_wind_environment(
                UniformAir::sea_level(),
                G,
                ConstantWind::new(4.0, 0.0).unwrap(),
            ),
            devices,
            3600.0,
        );
        let mut recorder = Recorder::new(
            vec![
                Channel::Time,
                Channel::HeightAboveGround,
                Channel::VerticalSpeed,
                Channel::RecoveryDragArea,
            ],
            Some(0.25),
        )
        .unwrap();
        let result = sim.run(&mut recorder).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);

        let kinds: Vec<EventKind> = result.events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::Liftoff,
                EventKind::RailExit,
                EventKind::Burnout,
                EventKind::Apogee,
                EventKind::Trigger(0),
                EventKind::Deployment(0),
                EventKind::Trigger(1),
                EventKind::Deployment(1),
                EventKind::Release(0),
                EventKind::GroundHit,
            ]
        );
        let at = |kind| result.event(kind).unwrap().sample;
        // Each charge opens its device after its lag, and the descent starts at the first opening.
        assert!(
            (at(EventKind::Deployment(0)).time_s - at(EventKind::Trigger(0)).time_s - 1.0).abs()
                < 1e-12
        );
        assert!(
            (at(EventKind::Deployment(1)).time_s - at(EventKind::Trigger(1)).time_s - 1.5).abs()
                < 1e-12
        );
        assert_eq!(at(EventKind::Apogee).phase, crate::Phase::Free);
        assert_eq!(at(EventKind::Deployment(0)).phase, crate::Phase::Descent);
        // The main fires as the centre of mass passes 300 m, descending under the drogue.
        let trigger = at(EventKind::Trigger(1));
        assert!(
            (trigger.height_above_ground_m - 300.0).abs() < 1e-6,
            "{trigger:?}"
        );
        assert!(trigger.vertical_speed_m_s < 0.0);
        // The drag area: the drogue alone, then the main alone once it releases the drogue.
        assert!((at(EventKind::Deployment(0)).recovery_drag_area_m2 - drogue_m2).abs() < 1e-12);
        assert!((at(EventKind::Trigger(1)).recovery_drag_area_m2 - drogue_m2).abs() < 1e-12);
        assert!((at(EventKind::Deployment(1)).recovery_drag_area_m2 - main_m2).abs() < 1e-12);
        assert!((at(EventKind::GroundHit).recovery_drag_area_m2 - main_m2).abs() < 1e-12);

        // Both stages settle at their own terminal speeds, so the descent rate falls when the
        // main opens. The drogue's stage is the speed just before the main's charge fires.
        let mass_kg = sim
            .assembly()
            .mass_properties(at(EventKind::Apogee).time_s)
            .mass_kg;
        let rho = UniformAir::sea_level().0.density_kg_m3;
        let under_drogue = terminal_speed_m_s(mass_kg, drogue_m2, rho, G);
        let under_main = terminal_speed_m_s(mass_kg, main_m2, rho, G);
        assert!(
            (-trigger.vertical_speed_m_s / under_drogue - 1.0).abs() < 0.02,
            "{} vs {under_drogue}",
            trigger.vertical_speed_m_s
        );
        let landing = at(EventKind::GroundHit);
        assert!(
            (-landing.vertical_speed_m_s / under_main - 1.0).abs() < 0.02,
            "{} vs {under_main}",
            landing.vertical_speed_m_s
        );
        assert!(
            under_main < 0.5 * under_drogue,
            "{under_main} {under_drogue}"
        );
        // The recorder's drag-area channel never exceeds one device's area: they never add up.
        let area = column(&recorder, "recovery_drag_area_m2");
        assert!(area.iter().all(|a| *a <= main_m2 + 1e-12));
        assert!(area.iter().any(|a| (*a - drogue_m2).abs() < 1e-12));
    }

    #[test]
    fn a_motor_delay_fires_a_device_and_a_canopy_fills_by_knackes_law() {
        // The ejection charge of Valetudo's motor (a 2 s delay after its 3.26 s burn) fires the
        // canopy, which then fills over t_f = n D₀/v from the airspeed at line stretch, its drag
        // area growing as (t/t_f)^j.
        let kind = CanopyType::FlatCircular;
        let diameter_m = 1.2;
        // Valetudo's example gives its motor no ejection charge, so the test picks one.
        let sim = Simulation::new(
            &with_delay(design("rocketpy-valetudo"), 2.0),
            "example",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_recovery(vec![
            Device::new(
                "ejection",
                DeviceDrag::canopy(kind, diameter_m),
                Trigger::MotorDelay { motor: 0 },
            )
            .with_inflation(Inflation::knacke(kind).unwrap()),
        ])
        .unwrap();
        let delay_s = match sim.assembly().motors[0].mounted.delay {
            Some(hpr_motor::Delay::Seconds(delay_s)) => delay_s,
            other => panic!("Valetudo's motor has no delay in seconds: {other:?}"),
        };
        let burnout_s = sim.assembly().motors[0].mounted.motor.burnout_time_s();
        let mut recorder =
            Recorder::new(vec![Channel::Time, Channel::RecoveryDragArea], None).unwrap();
        let result = sim.run(&mut recorder).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let trigger = result.event(EventKind::Trigger(0)).unwrap().sample;
        let deployment = result.event(EventKind::Deployment(0)).unwrap().sample;
        assert!(
            (trigger.time_s - (burnout_s + delay_s)).abs() < 1e-12,
            "{} vs {}",
            trigger.time_s,
            burnout_s + delay_s
        );
        assert_eq!(deployment.time_s, trigger.time_s, "no lag was given");
        assert_eq!(deployment.recovery_drag_area_m2, 0.0, "it starts empty");

        // Knacke's filling time from the airspeed at line stretch, and the growth law between.
        let filling_s = kind.fill_constant().unwrap() * diameter_m / deployment.airspeed_m_s;
        assert!(filling_s > 0.05 && filling_s < 0.5, "{filling_s} s");
        let full_m2 = sim.recovery()[0].drag.drag_area_m2();
        let times = column(&recorder, "time_s");
        let areas = column(&recorder, "recovery_drag_area_m2");
        let mut checked = 0;
        for (t, area) in times.iter().zip(&areas) {
            let elapsed = t - deployment.time_s;
            if elapsed <= 0.0 {
                assert_eq!(*area, 0.0, "area before line stretch at {t}");
                continue;
            }
            let fraction = (elapsed / filling_s).min(1.0);
            let expected = full_m2 * fraction.powf(kind.growth_exponent().unwrap());
            assert!(
                (area - expected).abs() < 1e-9 * full_m2,
                "{area} vs {expected} at {t}"
            );
            if elapsed < filling_s {
                checked += 1;
            }
        }
        assert!(checked >= 3, "only {checked} rows inside the filling time");
    }

    /// The comparison against RocketPy's own parachute phase
    /// (`validation/oracles/rocketpy/recovery.py`, M1.7a): the same declared state, devices, wind
    /// and site, and the descent that follows.
    #[test]
    fn descent_matches_rocketpy_examples() {
        let fixture = include_str!("../../../validation/fixtures/recovery/rocketpy-descent.json");
        let document: serde_json::Value = serde_json::from_str(fixture).unwrap();
        assert_eq!(document["oracle"], "rocketpy 1.13.0");
        let cases = document["cases"].as_array().unwrap();
        assert!(cases.len() >= 3, "{} cases", cases.len());
        let number = |value: &serde_json::Value| value.as_f64().unwrap();
        let mut rows = Vec::new();

        for case in cases {
            let name = case["name"].as_str().unwrap();
            let environment = case["environment"].clone();

            // The site: RocketPy's elevation above sea level, taken as the ellipsoidal height with
            // no geoid undulation (hpr has no geoid model, and the oracle's atmosphere and wind
            // are functions of that same height).
            let site = hpr_core::geodesy::Geodetic::from_degrees(
                number(&environment["latitude_deg"]),
                number(&environment["longitude_deg"]),
                number(&environment["elevation_m"]),
            )
            .unwrap();
            let wind = wind_of(&environment);
            let sim = Simulation::new(
                &design(&format!("rocketpy-{name}")),
                "example",
                Environment {
                    wind,
                    ..Environment::standard(site).unwrap()
                },
                // The rail is never used: these flights start in the air. It only has to be
                // long enough for the design's guides.
                Rail::vertical(6.0),
                FlightSettings {
                    max_time_s: 6000.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap();

            // First the environments: a descent compared against an oracle whose air, gravity or
            // wind differs is not comparing recovery.
            for sample in environment["samples"].as_array().unwrap() {
                let height_msl_m = number(&sample["height_msl_m"]);
                let air = sim.environment().atmosphere.air(height_msl_m).unwrap().air;
                let density = air.density_kg_m3;
                let oracle = number(&sample["density_kg_m3"]);
                // RocketPy reads its standard atmosphere off a 100-point linear pressure table
                // over 0 to 80 km, so it differs from the exact 1976 atmosphere. Measured worst
                // over all the samples: 3.7e-4 relative (NDRT at 206 m), which is 1.9e-4 in a
                // descent rate — below every difference this test goes on to report.
                assert!(
                    (density - oracle).abs() < 5e-4 * oracle,
                    "{name}: density {density} vs {oracle} at {height_msl_m} m"
                );
                // Gravity: RocketPy's "Somigliana" model is the same WGS 84 normal gravity hpr
                // uses, so these have to agree, not merely be close.
                let up = DVec3::new(0.0, 0.0, height_msl_m - number(&environment["elevation_m"]));
                let gravity = sim
                    .environment()
                    .earth
                    .gravity_enu_mps2(up)
                    .unwrap()
                    .length();
                let oracle_gravity = number(&sample["gravity_m_s2"]);
                assert!(
                    (gravity - oracle_gravity).abs() < 1e-6 * oracle_gravity,
                    "{name}: gravity {gravity} vs {oracle_gravity} at {height_msl_m} m"
                );
                let wind = sim
                    .environment()
                    .wind
                    .wind(height_msl_m)
                    .unwrap()
                    .velocity_enu_m_s;
                for (component, key) in [(wind.x, "wind_east_m_s"), (wind.y, "wind_north_m_s")] {
                    let oracle = number(&sample[key]);
                    assert!(
                        (component - oracle).abs() < 1e-9,
                        "{name}: {key} {component} vs {oracle} at {height_msl_m} m"
                    );
                }
                assert_eq!(wind.z, 0.0);
            }

            // The devices: the oracle's drag areas and triggers, with the first open from the
            // start and each one released by the next, which is how RocketPy's single `cd_s`
            // behaves when a main replaces a drogue.
            let start = case["start"].clone();
            let start_s = number(&start["time_s"]);
            let oracle_devices = case["devices"].as_array().unwrap();
            let mut devices = Vec::new();
            for (index, device) in oracle_devices.iter().enumerate() {
                let drag = DeviceDrag::DragArea {
                    cd_s_m2: number(&device["cd_s_m2"]),
                };
                let trigger = if index == 0 {
                    assert_eq!(device["trigger"]["kind"], "apogee");
                    assert_eq!(
                        number(&device["lag_s"]),
                        0.0,
                        "{name}: the first lag is zero"
                    );
                    Trigger::Time { time_s: start_s }
                } else {
                    assert_eq!(device["trigger"]["kind"], "descending_below_height_agl");
                    Trigger::Altitude {
                        height_above_ground_m: number(&device["trigger"]["height_m"]),
                    }
                };
                let mut next = Device::new(device["name"].as_str().unwrap(), drag, trigger)
                    .with_lag_s(number(&device["lag_s"]));
                if index + 1 < oracle_devices.len() {
                    next = next.with_release_by(index + 1);
                }
                devices.push(next);
            }
            let sim = sim.with_recovery(devices).unwrap();

            // The mass: RocketPy's parachute phase uses the rocket's dry mass, and hpr the
            // assembly's mass once the propellant is gone. The design is generated from the same
            // example, so they have to agree.
            let mass_kg = sim.assembly().mass_properties(start_s).mass_kg;
            let dry_mass_kg = number(&case["dry_mass_kg"]);
            assert!(
                (mass_kg - dry_mass_kg).abs() < 1e-9 * dry_mass_kg,
                "{name}: mass {mass_kg} vs the oracle's dry mass {dry_mass_kg}"
            );

            // The declared start, with the centre of mass where the oracle put it.
            let position = start["position_msl_m"].as_array().unwrap();
            let velocity = start["velocity_m_s"].as_array().unwrap();
            let cg_enu_m = DVec3::new(
                number(&position[0]),
                number(&position[1]),
                number(&position[2]) - number(&environment["elevation_m"]),
            );
            let attitude = Rail::vertical(6.0).attitude();
            let state = State {
                position_enu_m: cg_enu_m
                    - attitude.mul_vec3(sim.assembly().mass_properties(start_s).cg_m),
                velocity_enu_m_s: DVec3::new(
                    number(&velocity[0]),
                    number(&velocity[1]),
                    number(&velocity[2]),
                ),
                attitude,
                body_rate_rad_s: DVec3::ZERO,
            };
            let result = sim.run_free(start_s, state, &mut ()).unwrap();
            assert_eq!(result.termination, Termination::GroundHit, "{name}");
            let landing = result.event(EventKind::GroundHit).unwrap().sample;

            // The first device opens where both models agree, to within RocketPy's own trigger
            // sampling: it checks its triggers on a grid of `1/sampling_rate` anchored at t = 0,
            // and only over the span after its first accepted step, so its deployment comes a
            // little after hpr's, which locates the crossing exactly. Measured: 2.5 ms for the
            // four 105 Hz cases and 13 ms for Prometheus's 100 Hz one, against descents of 46 to
            // 257 s, so under 0.01% of the descent either way.
            let first = &case["events"].as_array().unwrap()[0];
            let late_s = number(&first["deploy_s"]) - start_s;
            let descent_s = number(&case["metrics"]["descent_time_s"]);
            assert!(
                (0.0..=0.02).contains(&late_s) && late_s < 1e-4 * descent_s,
                "{name}: the oracle deployed {late_s} s after the start, of a {descent_s} s descent"
            );

            // Every later device fired at its own setting, and the oracle's grid put its own
            // trigger a little below it: that difference is reported, not assumed away.
            for (index, device) in oracle_devices.iter().enumerate().skip(1) {
                let trigger = result
                    .event(EventKind::Trigger(index))
                    .unwrap_or_else(|| panic!("{name}: device {index} never fired"))
                    .sample;
                let height_m = number(&device["trigger"]["height_m"]);
                assert!(
                    (trigger.height_above_ground_m - height_m).abs() < 1e-3,
                    "{name}: device {index} fired at {} m, not its setting {height_m}",
                    trigger.height_above_ground_m
                );
                let oracle_event = &case["events"].as_array().unwrap()[index];
                // The oracle's own reported height at its trigger, which comes from RocketPy's
                // reporting spline over its stored samples, not from the dense output its
                // trigger was evaluated on. It can only have fired at or below the setting, by
                // at most one sample of fall (`v_z/sampling_rate`).
                rows.push((
                    format!(
                        "{name}: device {index} trigger height, hpr against the oracle's report"
                    ),
                    trigger.height_above_ground_m,
                    number(&oracle_event["height_above_ground_at_trigger_m"]),
                ));
                // The descent rate under the device before it, where the oracle reports its own.
                let oracle_speed = -number(&oracle_event["vertical_speed_at_trigger_m_s"]);
                let speed = -trigger.vertical_speed_m_s;
                rows.push((
                    format!("{name}: descent rate under device {}", index - 1),
                    speed,
                    oracle_speed,
                ));
            }

            // An independent anchor on both simulators: by the time it lands, the rocket is
            // descending at Knacke's equilibrium speed under the last device to open, computed
            // here from hpr's own air and gravity at the site.
            let last = oracle_devices.last().unwrap();
            let air = sim
                .environment()
                .atmosphere
                .air(number(&environment["elevation_m"]))
                .unwrap()
                .air;
            let gravity_m_s2 = sim
                .environment()
                .earth
                .gravity_enu_mps2(DVec3::ZERO)
                .unwrap()
                .length();
            let equilibrium_m_s = terminal_speed_m_s(
                mass_kg,
                number(&last["cd_s_m2"]),
                air.density_kg_m3,
                gravity_m_s2,
            );
            for (who, speed) in [
                ("hpr", -landing.vertical_speed_m_s),
                ("rocketpy", number(&case["metrics"]["impact_speed_m_s"])),
            ] {
                assert!(
                    (speed - equilibrium_m_s).abs() < 0.01 * equilibrium_m_s,
                    "{name}: {who} lands at {speed} m/s, not the equilibrium {equilibrium_m_s}"
                );
            }

            let metrics = case["metrics"].clone();
            let descent_time_s = landing.time_s - start_s;
            let drift = landing.cg_enu_m - cg_enu_m;
            rows.push((
                format!("{name}: descent time"),
                descent_time_s,
                number(&metrics["descent_time_s"]),
            ));
            rows.push((
                format!("{name}: impact descent rate"),
                -landing.vertical_speed_m_s,
                number(&metrics["impact_speed_m_s"]),
            ));
            rows.push((
                format!("{name}: drift"),
                drift.truncate().length(),
                number(&metrics["drift_m"]),
            ));
            if number(&metrics["drift_m"]) > 10.0 {
                rows.push((
                    format!("{name}: drift east"),
                    drift.x,
                    number(&metrics["drift_east_m"]),
                ));
                rows.push((
                    format!("{name}: drift north"),
                    drift.y,
                    number(&metrics["drift_north_m"]),
                ));
            }
        }

        // The milestone's tolerance: descent rate and drift within 3% of RocketPy's.
        let mut worst: f64 = 0.0;
        let mut report = String::new();
        for (what, hpr, oracle) in &rows {
            let error = (hpr - oracle) / oracle;
            worst = worst.max(error.abs());
            report.push_str(&format!(
                "{what}: hpr {hpr:.4}, rocketpy {oracle:.4}, {:+.2}%\n",
                100.0 * error
            ));
        }
        eprintln!("{report}");
        assert!(worst < 0.03, "worst error {:.2}%:\n{report}", 100.0 * worst);
    }

    /// The oracle's wind: a constant, or a profile in height above sea level. RocketPy
    /// interpolates its east and north components linearly, which is
    /// [`hpr_atmos::WindInterpolation::Components`].
    fn wind_of(environment: &serde_json::Value) -> std::sync::Arc<dyn hpr_atmos::Wind> {
        /// A level from east and north components, m/s: the speed and the direction the wind
        /// blows from, clockwise from north (`hpr_atmos::velocity_from_speed_direction`).
        fn level(height_msl_m: f64, east: f64, north: f64) -> hpr_atmos::WindLevel {
            hpr_atmos::WindLevel {
                height_msl_m,
                speed_m_s: east.hypot(north),
                direction_from_rad: (-east).atan2(-north).rem_euclid(std::f64::consts::TAU),
            }
        }
        let components = |key: &str| -> Option<Vec<(f64, f64)>> {
            environment[key].as_array().map(|rows| {
                rows.iter()
                    .map(|row| {
                        let row = row.as_array().unwrap();
                        (row[0].as_f64().unwrap(), row[1].as_f64().unwrap())
                    })
                    .collect()
            })
        };
        let levels = match (components("wind_u"), components("wind_v")) {
            (Some(east), Some(north)) => {
                assert_eq!(east.len(), north.len());
                east.iter()
                    .zip(&north)
                    .map(|((height_msl_m, east), (other, north))| {
                        assert_eq!(height_msl_m, other, "the profiles differ in height");
                        level(*height_msl_m, *east, *north)
                    })
                    .collect()
            }
            (None, None) => vec![level(
                0.0,
                environment["wind_u"].as_f64().unwrap(),
                environment["wind_v"].as_f64().unwrap(),
            )],
            _ => panic!("one wind component is a profile and the other is not"),
        };
        std::sync::Arc::new(
            hpr_atmos::LayeredWind::new(levels, hpr_atmos::WindInterpolation::Components).unwrap(),
        )
    }

    #[test]
    fn a_deployment_keeps_the_centre_of_mass_moving_as_it_was() {
        // Dropping the body rates at deployment must not move the centre of mass's momentum:
        // the state carries the nose tip's velocity, so it has to be shifted by `ω × r_cg`
        // (found in review). With `r_cg` along the axis and `ω` across it the error is across the
        // axis too, so for this nose-up drop it is about 0.9 m/s of drift rate; it becomes a
        // descent-rate error once the rocket has pitched over.
        let air = UniformAir::sea_level();
        let device = open_at_start(DeviceDrag::DragArea { cd_s_m2: 1.5 });
        let sim = flight(analytic_environment(air, G), vec![device], 3600.0);
        let rate_rad_s = DVec3::new(0.0, 0.7, 0.0);
        let mut state = dropped(&sim, 1_500.0, DVec3::new(2.0, 0.0, -12.0));
        state.body_rate_rad_s = rate_rad_s;
        let cg_m = sim.assembly().mass_properties(START_S).cg_m;
        // The centre of mass's velocity before the canopy opens: the nose tip's plus `ω × r_cg`
        // (the propellant is gone, so the centre of mass doesn't move inside the body).
        let before =
            state.velocity_enu_m_s + state.unit_attitude().mul_vec3(rate_rad_s.cross(cg_m));
        assert!(
            (before - state.velocity_enu_m_s).length() > 0.5,
            "the test needs a lever arm: {before}"
        );
        let result = sim.run_free(START_S, state, &mut ()).unwrap();
        let deployment = result.event(EventKind::Deployment(0)).unwrap().sample;
        assert_eq!(deployment.time_s, START_S);
        assert!(
            (deployment.cg_velocity_enu_m_s - before).length() < 1e-12,
            "{} vs {before}",
            deployment.cg_velocity_enu_m_s
        );
        assert_eq!(deployment.state.body_rate_rad_s, DVec3::ZERO);
        // The descent then settles at the terminal speed, as before.
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, 1.5, air.0.density_kg_m3, G);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
    }

    #[test]
    fn a_deployment_during_a_burn_keeps_the_centre_of_mass_moving_as_it_was() {
        // The same handover while propellant still burns, so the centre of mass is moving inside
        // the body as well (`v_cg = v_O + q(ω × r_cg + ṙ_cg)`). The two `ṙ_cg` terms cancel, so
        // the net shift is `q(ω × r_cg)` whatever the motor is doing; this pins that, and that
        // the canopy opening under thrust (an off-nominal case the descent phase keeps the thrust
        // for) does not move the centre of mass. Valetudo burns until 3.26 s, and the crosswind
        // gives it a body rate to shift by.
        let sim = flight(
            analytic_wind_environment(
                UniformAir::sea_level(),
                G,
                ConstantWind::new(8.0, 1.5).unwrap(),
            ),
            vec![Device::new(
                "early",
                DeviceDrag::DragArea { cd_s_m2: 0.2 },
                Trigger::Time { time_s: 2.0 },
            )],
            3600.0,
        );
        let result = sim.run(&mut ()).unwrap();
        let trigger = result.event(EventKind::Trigger(0)).unwrap().sample;
        let deployment = result.event(EventKind::Deployment(0)).unwrap().sample;
        assert_eq!(trigger.time_s, 2.0);
        assert_eq!(deployment.time_s, 2.0);
        assert!(
            trigger.thrust_n > 0.0,
            "the motor has to be burning: {trigger:?}"
        );
        assert!(
            trigger.state.body_rate_rad_s.length() > 1e-3,
            "the flight needs a body rate to shift by: {}",
            trigger.state.body_rate_rad_s
        );
        // The centre of mass keeps its velocity, and the nose tip's moved by exactly `ω × r_cg`.
        assert!(
            (deployment.cg_velocity_enu_m_s - trigger.cg_velocity_enu_m_s).length() < 1e-12,
            "{} vs {}",
            deployment.cg_velocity_enu_m_s,
            trigger.cg_velocity_enu_m_s
        );
        let cg_m = sim.assembly().mass_properties(2.0).cg_m;
        let shift = trigger
            .state
            .unit_attitude()
            .mul_vec3(trigger.state.body_rate_rad_s.cross(cg_m));
        assert!(shift.length() > 1e-3, "{shift}");
        assert!(
            (deployment.state.velocity_enu_m_s - trigger.state.velocity_enu_m_s - shift).length()
                < 1e-12,
            "{} vs {} + {shift}",
            deployment.state.velocity_enu_m_s,
            trigger.state.velocity_enu_m_s
        );
        assert_eq!(deployment.phase, crate::Phase::Descent);
        assert_eq!(result.termination, Termination::GroundHit);
    }

    #[test]
    fn devices_that_open_together_add_their_drag_areas() {
        // Two devices triggered at the same instant both open in the same pass, and the descent
        // runs under the sum of their drag areas.
        let air = UniformAir::sea_level();
        let devices = vec![
            open_at_start(DeviceDrag::DragArea { cd_s_m2: 1.0 }),
            open_at_start(DeviceDrag::DragArea { cd_s_m2: 2.0 }),
        ];
        let sim = flight(analytic_environment(air, G), devices, 3600.0);
        let result = sim
            .run_free(START_S, dropped(&sim, 1_000.0, DVec3::ZERO), &mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        for index in 0..2 {
            let deployment = result.event(EventKind::Deployment(index)).unwrap().sample;
            assert_eq!(deployment.time_s, START_S, "device {index}");
        }
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!((landing.recovery_drag_area_m2 - 3.0).abs() < 1e-12);
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, 3.0, air.0.density_kg_m3, G);
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
    }

    #[test]
    fn a_device_released_before_it_opens_never_pulls() {
        // The main opens first and releases the drogue; when the drogue's own charge fires later
        // there is nothing left to open, so it is recorded as triggered and never deploys.
        let air = UniformAir::sea_level();
        let devices = vec![
            Device::new(
                "drogue",
                DeviceDrag::DragArea { cd_s_m2: 4.0 },
                Trigger::Time {
                    time_s: START_S + 5.0,
                },
            )
            .with_release_by(1),
            open_at_start(DeviceDrag::DragArea { cd_s_m2: 1.0 }),
        ];
        let sim = flight(analytic_environment(air, G), devices, 3600.0);
        let result = sim
            .run_free(START_S, dropped(&sim, 1_000.0, DVec3::ZERO), &mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let release = result.event(EventKind::Release(0)).unwrap().sample;
        assert_eq!(
            release.time_s, START_S,
            "the main opens at once, so it releases at once"
        );
        let trigger = result.event(EventKind::Trigger(0)).unwrap().sample;
        assert_eq!(trigger.time_s, START_S + 5.0);
        assert!(
            result.event(EventKind::Deployment(0)).is_none(),
            "a released device must not deploy: {:?}",
            result.events.iter().map(|e| e.kind).collect::<Vec<_>>()
        );
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!((landing.recovery_drag_area_m2 - 1.0).abs() < 1e-12);
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, 1.0, air.0.density_kg_m3, G);
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
    }

    #[test]
    fn a_release_waits_for_the_main_to_fill_so_the_drag_area_never_dips() {
        // A drogue cut away at the main's line stretch would leave the rocket under an empty
        // canopy: the drag area would collapse and the descent would speed up. The release waits
        // until the main is full, so the drag area only ever grows here (ADR-012).
        let air = UniformAir::sea_level();
        let drogue_m2 = 0.45;
        let main_m2 = 6.0;
        let filling_s = 2.0;
        let devices = vec![
            open_at_start(DeviceDrag::DragArea { cd_s_m2: drogue_m2 }).with_release_by(1),
            Device::new(
                "main",
                DeviceDrag::DragArea { cd_s_m2: main_m2 },
                Trigger::Time {
                    time_s: START_S + 20.0,
                },
            )
            .with_inflation(Inflation::FillingTime {
                time_s: filling_s,
                exponent: 2.0,
            }),
        ];
        let sim = flight(analytic_environment(air, G), devices, 3600.0);
        let mut recorder = Recorder::new(
            vec![
                Channel::Time,
                Channel::RecoveryDragArea,
                Channel::VerticalSpeed,
            ],
            None,
        )
        .unwrap();
        let result = sim
            .run_free(START_S, dropped(&sim, 2_000.0, DVec3::ZERO), &mut recorder)
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let main = result.event(EventKind::Deployment(1)).unwrap().sample;
        let release = result.event(EventKind::Release(0)).unwrap().sample;
        assert_eq!(main.time_s, START_S + 20.0);
        assert!(
            (release.time_s - (main.time_s + filling_s)).abs() < 1e-9,
            "released at {} m, not at the end of filling {}",
            release.time_s,
            main.time_s + filling_s
        );
        // The drag area never falls below the drogue's, and the descent never speeds up after the
        // main's charge fires.
        let areas = column(&recorder, "recovery_drag_area_m2");
        let times = column(&recorder, "time_s");
        let speeds = column(&recorder, "vertical_speed_m_s");
        let mut worst_area = f64::INFINITY;
        let mut fastest = 0.0_f64;
        for ((t, area), speed) in times.iter().zip(&areas).zip(&speeds) {
            if *t < main.time_s {
                continue;
            }
            worst_area = worst_area.min(*area);
            fastest = fastest.max(-speed);
        }
        assert!(
            worst_area >= drogue_m2 - 1e-12,
            "the drag area dipped to {worst_area} m², below the drogue's {drogue_m2}"
        );
        assert!(
            fastest <= -main.vertical_speed_m_s + 1e-9,
            "the descent sped up after the main fired: {fastest} against {}",
            -main.vertical_speed_m_s
        );
        // And by the end the main alone carries it.
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!((landing.recovery_drag_area_m2 - main_m2).abs() < 1e-12);
    }

    #[test]
    fn a_recovered_flight_repeats_bit_identically() {
        // Determinism, and Loft lesson L24: the devices' progress belongs to the flight, not to
        // the simulation, so flying the same simulation twice gives bit-identical rows and events.
        let sim = flight(
            analytic_wind_environment(
                UniformAir::sea_level(),
                G,
                ConstantWind::new(3.0, 1.2).unwrap(),
            ),
            vec![
                Device::new(
                    "drogue",
                    DeviceDrag::canopy(CanopyType::FlatCircular, 0.5),
                    Trigger::Apogee,
                )
                .with_lag_s(0.75)
                .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap())
                .with_release_by(1),
                Device::new(
                    "main",
                    DeviceDrag::canopy(CanopyType::FlatCircular, 2.0),
                    Trigger::Altitude {
                        height_above_ground_m: 200.0,
                    },
                )
                .with_lag_s(1.25)
                .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap()),
            ],
            3600.0,
        );
        let fly = || {
            let mut recorder = Recorder::new(Channel::ALL.to_vec(), Some(0.1)).unwrap();
            let result = sim.run(&mut recorder).unwrap();
            (recorder.rows().to_vec(), result)
        };
        let (first_rows, first) = fly();
        let (second_rows, second) = fly();
        assert_eq!(first.termination, Termination::GroundHit);
        assert!(first_rows.len() > 100, "{} rows", first_rows.len());
        assert_eq!(first_rows, second_rows, "the rows differ between runs");
        assert_eq!(
            first.events, second.events,
            "the events differ between runs"
        );
        assert_eq!(first.final_sample, second.final_sample);
        assert_eq!(first.stats, second.stats);
        // And the events are the full sequence, once each.
        let kinds: Vec<EventKind> = first.events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EventKind::Liftoff,
                EventKind::RailExit,
                EventKind::Burnout,
                EventKind::Apogee,
                EventKind::Trigger(0),
                EventKind::Deployment(0),
                EventKind::Trigger(1),
                EventKind::Deployment(1),
                EventKind::Release(0),
                EventKind::GroundHit,
            ]
        );
    }

    #[test]
    fn an_apogee_charge_fires_on_a_flight_that_starts_descending() {
        // The apogee trigger is RocketPy's `y[5] < 0`, not only hpr's apogee event: a flight
        // restarted past its apogee (`run_free`, which M1.9's staging and flight-data replay use)
        // still deploys. Found in review: with an event-only trigger this flight fell ballistically
        // to the ground with no deployment and no error.
        let air = UniformAir::sea_level();
        let device = Device::new(
            "main",
            DeviceDrag::DragArea { cd_s_m2: 2.0 },
            Trigger::Apogee,
        );
        let drag_area_m2 = device.drag.drag_area_m2();
        let sim = flight(analytic_environment(air, G), vec![device], 3600.0);
        let result = sim
            .run_free(
                START_S,
                dropped(&sim, 1_000.0, DVec3::new(0.0, 0.0, -5.0)),
                &mut (),
            )
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let deployment = result.event(EventKind::Deployment(0)).unwrap().sample;
        assert_eq!(deployment.time_s, START_S, "it is already descending");
        assert_eq!(
            result.event(EventKind::Apogee),
            None,
            "there is no apogee to find"
        );
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
        // A climbing flight does not fire it early: the charge waits for the apogee.
        let climbing = sim
            .run_free(
                START_S,
                dropped(&sim, 1_000.0, DVec3::new(0.0, 0.0, 30.0)),
                &mut (),
            )
            .unwrap();
        let apogee = climbing.event(EventKind::Apogee).unwrap().sample;
        let fired = climbing.event(EventKind::Trigger(0)).unwrap().sample;
        assert_eq!(fired.time_s, apogee.time_s);
        assert!(apogee.time_s > START_S + 1.0, "{}", apogee.time_s);
    }

    #[test]
    fn user_events_keep_their_numbers_and_fire_during_the_descent() {
        // The event list is rebuilt per interval (`flight::Watch`), and the height triggers come
        // and go, so the user events sit after them. This pins that their indices don't move and
        // that they still fire once the descent has started.
        let air = UniformAir::sea_level();
        let devices = vec![
            open_at_start(DeviceDrag::DragArea { cd_s_m2: 0.5 }).with_release_by(1),
            Device::new(
                "main",
                DeviceDrag::DragArea { cd_s_m2: 4.0 },
                Trigger::Altitude {
                    height_above_ground_m: 400.0,
                },
            ),
        ];
        let sim = flight(analytic_environment(air, G), devices, 3600.0)
            .with_event(crate::flight::UserEvent {
                name: "through 700 m".to_owned(),
                direction: crate::events::Direction::Falling,
                function: Box::new(|sample| sample.height_above_ground_m - 700.0),
            })
            .with_event(crate::flight::UserEvent {
                name: "through 100 m".to_owned(),
                direction: crate::events::Direction::Falling,
                function: Box::new(|sample| sample.height_above_ground_m - 100.0),
            });
        let result = sim
            .run_free(START_S, dropped(&sim, 1_000.0, DVec3::ZERO), &mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        for (user, height_m) in [(0usize, 700.0), (1, 100.0)] {
            let event = result
                .event(EventKind::User(user))
                .unwrap_or_else(|| panic!("user event {user} never fired"))
                .sample;
            assert!(
                (event.height_above_ground_m - height_m).abs() < 1e-6,
                "user event {user} fired at {} m",
                event.height_above_ground_m
            );
            assert_eq!(event.phase, crate::Phase::Descent);
        }
        // The user events fire between the main's trigger and the ground, in height order.
        let kinds: Vec<EventKind> = result.events.iter().map(|event| event.kind).collect();
        let position = |kind| kinds.iter().position(|other| *other == kind).unwrap();
        assert!(position(EventKind::User(0)) < position(EventKind::Trigger(1)));
        assert!(position(EventKind::Trigger(1)) < position(EventKind::User(1)));
        assert!(position(EventKind::User(1)) < position(EventKind::GroundHit));
    }

    #[test]
    fn the_public_recovery_types_round_trip_through_json() {
        let devices = vec![
            Device::new(
                "drogue",
                DeviceDrag::DragArea { cd_s_m2: 0.45 },
                Trigger::Apogee,
            )
            .with_lag_s(1.5)
            .with_release_by(1),
            Device::new(
                "main",
                DeviceDrag::canopy(CanopyType::Ringsail, 3.0),
                Trigger::Altitude {
                    height_above_ground_m: 250.0,
                },
            )
            .with_inflation(Inflation::FillingTime {
                time_s: 1.5,
                exponent: 2.0,
            }),
            Device::new(
                "timer",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.0),
                Trigger::Time { time_s: 12.0 },
            )
            .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap()),
            Device::new(
                "ejection",
                DeviceDrag::DragArea { cd_s_m2: 1.0 },
                Trigger::MotorDelay { motor: 0 },
            ),
            Device::new(
                "streamer",
                DeviceDrag::streamer(1.2, 0.12, 0.032),
                Trigger::Apogee,
            ),
            Device::new(
                "appendix C streamer",
                DeviceDrag::Streamer {
                    length_m: 1.0,
                    width_m: 0.1,
                    surface_density_kg_m2: 0.04,
                    model: StreamerModel::OpenRocket,
                },
                Trigger::Apogee,
            ),
            Device::new(
                "tumble",
                DeviceDrag::tumbling(&design("rocketpy-valetudo").assemble("example").unwrap())
                    .unwrap(),
                Trigger::Apogee,
            ),
        ];
        let text = serde_json::to_string(&devices).unwrap();
        let back: Vec<Device> = serde_json::from_str(&text).unwrap();
        assert_eq!(devices, back);
        // The defaults are optional in a file, and unknown fields are refused.
        let terse: Device = serde_json::from_str(
            r#"{"name":"d","drag":{"drag_area":{"cd_s_m2":1.5}},"trigger":"apogee"}"#,
        )
        .unwrap();
        assert_eq!(terse.lag_s, 0.0);
        assert_eq!(terse.inflation, Inflation::Instant);
        assert_eq!(terse.released_by, None);
        assert!(
            serde_json::from_str::<Device>(
                r#"{"name":"d","drag":{"drag_area":{"cd_s_m2":1.5}},"trigger":"apogee","lg":1}"#
            )
            .is_err()
        );
        // A typo inside the drag is refused too: `model` defaults, and the two streamer models
        // differ by a factor of four in drag area, so a silent default would be a wrong number.
        let typo = r#"{"name":"s","drag":{"streamer":{"length_m":1.0,"width_m":0.1,
            "surface_density_kg_m2":0.04,"modle":"open_rocket"}},"trigger":"apogee"}"#;
        assert!(serde_json::from_str::<Device>(typo).is_err(), "{typo}");
        // And the model itself round-trips by name.
        let named: DeviceDrag = serde_json::from_str(
            r#"{"streamer":{"length_m":1.0,"width_m":0.1,"surface_density_kg_m2":0.04,
                "model":"open_rocket"}}"#,
        )
        .unwrap();
        assert_eq!(named.canopy_type(), None);
        assert!(
            (named.drag_area_m2() - StreamerModel::OpenRocket.drag_area_m2(1.0, 0.1, 0.04)).abs()
                < 1e-15
        );
    }

    #[test]
    fn streamer_models_reproduce_their_printed_equations() {
        // Carruthers and Filippone's three printed curves, on the planform area, at the areas
        // they were fitted at: `0.405 AR^−0.494` at 0.075 m² (eq. 1), `0.561 AR^−0.480` at
        // 0.025 m² (eq. 2) and `0.6514 AR^−0.6075` at 0.05 m² (the trend line on Figure 3).
        let cases: [(f64, f64, f64); 6] = [
            (10.0, 0.075, 0.405 * 10.0_f64.powf(-0.494)),
            (30.0, 0.075, 0.405 * 30.0_f64.powf(-0.494)),
            (10.0, 0.025, 0.561 * 10.0_f64.powf(-0.480)),
            (3.3, 0.025, 0.561 * 3.3_f64.powf(-0.480)),
            // The trend line on Figure 3, which the text does not repeat as an equation.
            (3.3, 0.05, 0.6514 * 3.3_f64.powf(-0.6075)),
            (30.0, 0.05, 0.6514 * 30.0_f64.powf(-0.6075)),
        ];
        for (aspect_ratio, planform_m2, expected) in cases {
            // A planform area `S` at aspect ratio `l/w` means `l = √(S·AR)`, `w = √(S/AR)`.
            let length_m = (planform_m2 * aspect_ratio).sqrt();
            let width_m = (planform_m2 / aspect_ratio).sqrt();
            let drag_area_m2 = StreamerModel::Filippone.drag_area_m2(length_m, width_m, 0.05);
            let coefficient = drag_area_m2 / planform_m2;
            assert!(
                (coefficient - expected).abs() < 1e-12,
                "AR {aspect_ratio} at {planform_m2} m²: {coefficient} vs {expected}"
            );
        }
        // Between the two areas it interpolates, and outside them it holds the end curve. A
        // planform of 0.05 m² at `AR = 10` is `l = √0.5 m` by `w = l/10`.
        let middle_m = (0.5_f64).sqrt();
        let between = StreamerModel::Filippone.drag_area_m2(middle_m, middle_m / 10.0, 0.05) / 0.05;
        let small = 0.561 * 10.0_f64.powf(-0.480);
        let large = 0.405 * 10.0_f64.powf(-0.494);
        assert!(large < between && between < small, "{between}");
        let huge = StreamerModel::Filippone.drag_area_m2(3.0, 0.3, 0.05) / 0.9;
        assert!((huge - large).abs() < 1e-12, "{huge} vs {large}");

        // The OpenRocket technical documentation's appendix C: its own reference material is
        // 80 g/m² polyethylene, where the material factor is exactly 1 and `C_Dm` is
        // `0.034 (l + 1)/l`.
        let reference = StreamerModel::OpenRocket.drag_area_m2(0.4, 0.04, 0.080) / (0.4 * 0.04);
        assert!(
            (reference - 0.034 * (0.4 + 1.0) / 0.4).abs() < 1e-12,
            "{reference}"
        );
        // And its material correction is linear in surface density about −25 g/m².
        let light = StreamerModel::OpenRocket.drag_area_m2(0.4, 0.04, 0.010);
        let heavy = StreamerModel::OpenRocket.drag_area_m2(0.4, 0.04, 0.080);
        assert!(
            (light / heavy - (0.010 + 0.025) / (0.080 + 0.025)).abs() < 1e-12,
            "{light} {heavy}"
        );
    }

    /// The average speed of a drop from rest through `height_m` under a drag area, m/s: the
    /// closed-form fall `h = (v_t²/g) ln cosh(g t/v_t)` solved for the time. A drop test measures
    /// this, not the terminal speed it is approaching.
    fn drop_average_m_s(
        mass_kg: f64,
        drag_area_m2: f64,
        density_kg_m3: f64,
        height_m: f64,
    ) -> (f64, f64) {
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, density_kg_m3, G);
        let time_s =
            (terminal_m_s / G) * (G * height_m / (terminal_m_s * terminal_m_s)).exp().acosh();
        (height_m / time_s, terminal_m_s)
    }

    #[test]
    fn streamer_models_against_kidwells_drop_tests() {
        // The only free-drop streamer data in hand: C. Kidwell, "Streamer Duration Optimization",
        // NAR R&D, NARAM-43 (2001). Sixteen 4 in × 40 in streamers, each with a weight of about
        // 5 g at one corner, dropped 20.1 m from a stadium deck.
        //
        // Two details of his method decide how to compare (both found in review):
        //
        // - his descent rates are **distance over time**, so they are averages over the drop, not
        //   terminal speeds. A 20.1 m drop averages 0.97 of terminal at 2.8 m/s and 0.89 at
        //   5.9 m/s, so the correction matters most for the model that predicts the fastest fall.
        //   The prediction here is therefore the same average, from the closed-form fall.
        // - he **normalised** each rate "by dividing by the actual mass of the attached weight
        //   and multiplying by 5 g", so the rates belong to a notional 5.000 g weight, not to
        //   Table 1's actual one.
        let length_m = 40.0 * 0.0254;
        let width_m = 4.0 * 0.0254;
        let planform_m2 = length_m * width_m;
        let rho = 1.225;
        let drop_m = 20.1;
        let cases = [
            // (material, streamer mass from Table 1, normalised descent rate, pleated)
            ("crepe paper", 3.3206e-3, 2.80, false),
            ("Micafilm", 4.3412e-3, 2.04, true),
        ];
        let mut report = String::new();
        for (material, streamer_kg, measured_m_s, pleated) in cases {
            let surface_density_kg_m2 = streamer_kg / planform_m2;
            let mass_kg = streamer_kg + 5.0e-3;
            let predict = |model: StreamerModel| {
                let drag_area_m2 = model.drag_area_m2(length_m, width_m, surface_density_kg_m2);
                let (average_m_s, terminal_m_s) =
                    drop_average_m_s(mass_kg, drag_area_m2, rho, drop_m);
                (average_m_s, terminal_m_s, drag_area_m2)
            };
            let (filippone, _, filippone_m2) = predict(StreamerModel::Filippone);
            let (open_rocket, _, open_rocket_m2) = predict(StreamerModel::OpenRocket);
            // What the drop itself says the drag area was.
            let measured_m2 = {
                // Invert the closed-form average: search the drag area whose average matches.
                let mut low = 1e-4;
                let mut high = 1.0;
                for _ in 0..200 {
                    let middle = 0.5 * (low + high);
                    if drop_average_m_s(mass_kg, middle, rho, drop_m).0 > measured_m_s {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                0.5 * (low + high)
            };
            report.push_str(&format!(
                "{material}: measured {measured_m_s:.2} m/s (C_D S {measured_m2:.5} m², C_D \
                 {:.3}), Filippone {filippone:.2} ({:+.0}%, C_D S {filippone_m2:.5}), OpenRocket \
                 {open_rocket:.2} ({:+.0}%, C_D S {open_rocket_m2:.5})\n",
                measured_m2 / planform_m2,
                100.0 * (filippone / measured_m_s - 1.0),
                100.0 * (open_rocket / measured_m_s - 1.0),
            ));
            // Both models predict a faster descent than the drop: neither knows about pleats, and
            // the correlations are for a streamer clamped at its leading edge, which the paper
            // measures as less draggy than a free one.
            assert!(filippone > measured_m_s, "{material}: {filippone}");
            assert!(open_rocket > filippone, "{material}: {open_rocket}");
            if pleated {
                // Pleats more than double the drag: measured C_D 0.341 against 0.161 flat.
                assert!(
                    (filippone / measured_m_s - 1.0) < 0.75,
                    "{material}: {filippone} against {measured_m_s}"
                );
            } else {
                // Flat streamer, flat correlation: within 10%, where appendix C is 88% fast.
                assert!(
                    (filippone / measured_m_s - 1.0) < 0.10,
                    "{material}: {filippone} against {measured_m_s}"
                );
                assert!(
                    (open_rocket / measured_m_s - 1.0) > 0.5,
                    "{material}: appendix C should be the slow one: {open_rocket}"
                );
            }
        }
        eprintln!("{report}");
    }

    #[test]
    fn a_streamer_descends_at_its_cited_terminal_velocity() {
        // A streamer's drag area is its model's, and a descent under it settles at Knacke's
        // equilibrium speed for that area, the same as a canopy's.
        let air = UniformAir::sea_level();
        let drag = DeviceDrag::streamer(1.5, 0.15, 0.040);
        let drag_area_m2 = drag.drag_area_m2();
        // 1.5 m by 0.15 m is a planform of 0.225 m², above the largest area Carruthers and
        // Filippone fitted, so their 0.075 m² curve holds: `C_D = 0.405 · 10^−0.494 = 0.12984`
        // and `C_D S = 0.029216 m²`.
        let expected_m2 = 0.405 * 10.0_f64.powf(-0.494) * 0.225;
        assert!(
            (drag_area_m2 - expected_m2).abs() < 1e-12,
            "{drag_area_m2} vs {expected_m2}"
        );
        let sim = flight(
            analytic_environment(air, G),
            vec![open_at_start(drag)],
            3600.0,
        );
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        // A streamer is a feeble decelerator: Valetudo falls at 67 m/s under this one, so the
        // drop has to be long enough to settle (the time constant is `v_t/g`, near 7 s).
        assert!(terminal_m_s > 20.0, "{terminal_m_s}");
        let result = sim
            .run_free(START_S, dropped(&sim, 6_000.0, DVec3::ZERO), &mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
    }

    #[test]
    fn the_tumble_model_against_its_own_drop_tests() {
        // The tumbling constants were fitted to the five models of the OpenRocket technical
        // documentation's Table 3.3 (printed page 54), dropped 22 m at ρ = 1.31 kg/m³ with the
        // terminal velocity read off the video to ±0.3 m/s. Replaying them through hpr's reading
        // of the model — `1.42 · eff(n) · A_1fin + 0.56 · d · l`, with one fin's area the
        // trapezoid `(C_r + C_t) s/2` — is the only independent check of it available, and it
        // does **not** reproduce the documentation's claim of 3 to 14%.
        let rho = 1.31;
        // (model, fins, root chord, tip chord, span, diameter, body length, mass, measured v0)
        let models = [
            (
                "#1", 3usize, 0.070, 0.040, 0.060, 0.044, 0.108, 18.0e-3, 5.6,
            ),
            ("#2", 4, 0.070, 0.040, 0.060, 0.044, 0.108, 22.0e-3, 6.3),
            ("#3", 3, 0.200, 0.140, 0.130, 0.103, 0.290, 160.0e-3, 6.6),
            ("#4", 0, 0.0, 0.0, 0.0, 0.044, 0.100, 6.8e-3, 5.4),
            ("#5", 4, 0.085, 0.085, 0.050, 0.0, 0.0, 11.5e-3, 5.0),
        ];
        let mut report = String::new();
        let mut worst: f64 = 0.0;
        let mut errors = Vec::new();
        for (name, fins, root_m, tip_m, span_m, diameter_m, body_m, mass_kg, measured_m_s) in models
        {
            let fin_area_m2 = if fins == 0 {
                0.0
            } else {
                let one_m2 = 0.5 * (root_m + tip_m) * span_m;
                one_m2 * TUMBLE_FIN_EFFICIENCY[fins - 1]
            };
            let body_profile_m2 = diameter_m * body_m;
            let drag_area_m2 = TUMBLE_FIN_DRAG_COEFFICIENT * fin_area_m2
                + TUMBLE_BODY_DRAG_COEFFICIENT * body_profile_m2;
            let predicted_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, rho, G);
            let error = predicted_m_s / measured_m_s - 1.0;
            worst = worst.max(error.abs());
            errors.push(error);
            report.push_str(&format!(
                "{name}: measured {measured_m_s:.1} m/s, hpr {predicted_m_s:.2} ({:+.1}%)\n",
                100.0 * error
            ));
        }
        eprintln!("{report}");
        // The spread is −10 to +19%, not the 3 to 14% the documentation claims for its own fit,
        // and the finless model is the outlier: it wants a body coefficient near 0.79 where the
        // model says 0.56. Either hpr's reading of the two areas is not the one behind the
        // constants (the text pins neither convention) or the claim is not reproducible;
        // `docs/physics/recovery.md` prints this table rather than repeating the 3 to 14%.
        //
        // Every model's error is pinned, not just the worst: a wrong efficiency factor would
        // move one of the others while the extreme stayed put.
        assert_eq!(
            errors
                .iter()
                .map(|error| format!("{:+.1}", 100.0 * error))
                .collect::<Vec<_>>(),
            ["-5.8", "-5.4", "-7.2", "+19.0", "-10.0"],
            "{report}"
        );
        assert!(worst < 0.20, "{worst} spread:\n{report}");
    }

    #[test]
    fn tumbling_refuses_an_airframe_the_model_cannot_represent() {
        // Tube fins are a large part of a tumbling rocket's broadside area and §3.5 has no factor
        // for them, so `tumbling` refuses rather than crediting a bare tube's drag.
        let mut rocket = design("rocketpy-valetudo");
        let (index, fins) = rocket.stages[0].components[1]
            .children
            .iter()
            .enumerate()
            .find_map(|(index, child)| match &child.part {
                hpr_design::Part::FinSet(fins) => Some((index, fins.clone())),
                _ => None,
            })
            .expect("Valetudo has a fin set");
        rocket.stages[0].components[1].children[index].part =
            hpr_design::Part::TubeFinSet(hpr_design::TubeFinSet {
                count: 3,
                length_m: 0.15,
                outer_radius_m: 0.02,
                thickness_m: 0.001,
                base_angle_rad: 0.0,
                material: fins.material.clone(),
            });
        let assembly = rocket.assemble("example").unwrap();
        let error = DeviceDrag::tumbling(&assembly).expect_err("tube fins");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
        // The unchanged design is fine, so the refusal is about the tube fins and nothing else.
        assert!(
            DeviceDrag::tumbling(&design("rocketpy-valetudo").assemble("example").unwrap()).is_ok()
        );
    }

    #[test]
    fn a_tumbling_body_descends_at_its_cited_terminal_velocity() {
        // The OpenRocket technical documentation's tumbling model, §3.5: the drag area is
        // `1.42 A_f + 0.56 A_bt`, with `A_bt` the body's side profile and `A_f` one fin's area
        // times the efficiency factor for the fin count. Computed here from Valetudo's own
        // geometry, and checked by a flight.
        let air = UniformAir::sea_level();
        let rocket = design("rocketpy-valetudo");
        let assembly = rocket.assemble("example").unwrap();
        let drag = DeviceDrag::tumbling(&assembly).unwrap();
        let DeviceDrag::Tumble {
            drag_area_m2,
            body_profile_m2,
            fin_area_m2,
        } = drag
        else {
            panic!("tumbling should build a Tumble: {drag:?}");
        };
        // Valetudo: a 0.274 m tangent nose and 1.884 m of 80.9 mm tube, so a side profile of
        // 0.04045·0.274 + 0.0809·1.884 = 0.1635 m², and three fins of 0.058 m root, 0.018 m tip
        // and 0.077 m span, so one fin is 2.93e-3 m² and the three-fin factor is 1.50.
        let expected_fin_m2 = 1.5
            * match &assembly
                .layout
                .components
                .iter()
                .find(|c| matches!(c.part, hpr_design::Part::FinSet(_)))
                .unwrap()
                .part
            {
                hpr_design::Part::FinSet(fins) => fins.planform.geometry().unwrap().area_m2,
                _ => unreachable!(),
            };
        assert!(
            (fin_area_m2 - expected_fin_m2).abs() < 1e-12,
            "{fin_area_m2}"
        );
        assert!(
            (drag_area_m2 - (1.42 * fin_area_m2 + 0.56 * body_profile_m2)).abs() < 1e-15,
            "{drag_area_m2}"
        );
        assert!(
            (body_profile_m2 - (0.040_45 * 0.274 + 0.080_9 * 1.884)).abs() < 1e-12,
            "{body_profile_m2}"
        );

        let sim = flight(
            analytic_environment(air, G),
            vec![open_at_start(drag)],
            3600.0,
        );
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        // Tumbling is slower than a ballistic dive but far faster than a canopy: 37 m/s for this
        // 8.3 kg rocket, well outside the 6.8 to 160 g the constants were fitted on.
        assert!(terminal_m_s > 20.0 && terminal_m_s < 60.0, "{terminal_m_s}");
        let result = sim
            .run_free(START_S, dropped(&sim, 4_000.0, DVec3::ZERO), &mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{} vs {terminal_m_s}",
            landing.vertical_speed_m_s
        );
    }

    /// The two-stage test design, whose booster and sustainer each carry a motor.
    fn two_stage() -> hpr_design::Rocket {
        design("synthetic-two-stage-75mm-54mm")
    }

    /// That design flown with `devices` and a separation, from a vertical rail.
    fn staged_flight(
        environment: Environment,
        devices: Vec<Device>,
        separation: Separation,
    ) -> Simulation {
        Simulation::new(
            &two_stage(),
            "j760-i175",
            environment,
            Rail::vertical(6.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices)
        .unwrap()
        .with_separation(separation)
        .unwrap()
    }

    #[test]
    fn a_separation_lands_every_body_and_the_masses_add_up() {
        // The stack comes apart at apogee: the sustainer descends under a canopy, the booster
        // tumbles. Both have to reach the ground, each at its own terminal speed, and their
        // masses have to add up to the whole rocket's.
        let air = UniformAir::sea_level();
        let assembly = two_stage().assemble("j760-i175").unwrap();
        // The booster tumbles on its own stage's geometry, not the whole stack's.
        let tumble = DeviceDrag::tumbling_stages(&assembly, (1, 1)).unwrap();
        let devices = vec![
            Device::new(
                "sustainer main",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.8),
                Trigger::Altitude {
                    height_above_ground_m: 1_500.0,
                },
            ),
            Device::new(
                "booster tumble",
                tumble,
                Trigger::Altitude {
                    height_above_ground_m: 1_500.0,
                },
            )
            .on_body(1),
        ];
        let sim = staged_flight(
            analytic_environment(air, G),
            devices,
            Separation::new(Trigger::Apogee, 0),
        );
        // From just past apogee: this stack goes supersonic on the way up, which the aero refuses
        // until M1.8, and the ascent is not what this test is about.
        let start = dropped(&sim, 2_000.0, DVec3::new(0.0, 0.0, -0.5));
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert_eq!(result.termination, Termination::Separated);
        assert!(result.event(EventKind::Separation).is_some());
        assert_eq!(result.bodies.len(), 2, "{:?}", result.bodies.len());

        // The bodies' masses add up to the rocket's at the separation, and each is a real share.
        let separation = result.event(EventKind::Separation).unwrap().sample;
        let whole_kg = sim.assembly().mass_properties(separation.time_s).mass_kg;
        let sum_kg: f64 = result.bodies.iter().map(|body| body.mass_kg).sum();
        assert!(
            (sum_kg - whole_kg).abs() < 1e-12 * whole_kg,
            "{sum_kg} vs {whole_kg}"
        );
        for body in &result.bodies {
            assert!(
                body.mass_kg > 0.05 * whole_kg,
                "body {} is {} kg of {whole_kg}",
                body.body,
                body.mass_kg
            );
        }
        assert_eq!(result.bodies[0].stages, (0, 0));
        assert_eq!(result.bodies[1].stages, (1, 1));

        // Every body lands, under its own device, at its own terminal speed.
        let rho = air.0.density_kg_m3;
        for body in &result.bodies {
            assert_eq!(
                body.termination,
                Termination::GroundHit,
                "body {}",
                body.body
            );
            let landing = body.event(EventKind::GroundHit).unwrap().sample;
            assert!(landing.height_above_ground_m.abs() < 1e-6, "{landing:?}");
            assert!(landing.time_s > separation.time_s, "{landing:?}");
            let device = body.body;
            let drag_area_m2 = sim.recovery()[device].drag.drag_area_m2();
            let terminal_m_s = terminal_speed_m_s(body.mass_kg, drag_area_m2, rho, G);
            assert!(
                (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-3,
                "body {}: {} vs {terminal_m_s}",
                body.body,
                landing.vertical_speed_m_s
            );
            assert!(
                (landing.recovery_drag_area_m2 - drag_area_m2).abs() < 1e-12,
                "body {}: {landing:?}",
                body.body
            );
            assert!(body.event(EventKind::Deployment(device)).is_some());
        }
        // The tumbling booster comes down much faster than the sustainer under its canopy.
        // Measured: the 0.550 kg sustainer lands at 729.0 s at 2.11 m/s under its 1.8 m canopy,
        // the 1.125 kg booster at 107.5 s at 16.74 m/s tumbling, of a 1.675 kg stack.
        let under_canopy = -result.bodies[0]
            .event(EventKind::GroundHit)
            .unwrap()
            .sample
            .vertical_speed_m_s;
        let tumbling = -result.bodies[1]
            .event(EventKind::GroundHit)
            .unwrap()
            .sample
            .vertical_speed_m_s;
        assert!(
            tumbling > 3.0 * under_canopy,
            "{tumbling} vs {under_canopy}"
        );
    }

    #[test]
    fn a_separation_conserves_momentum_and_gives_each_body_its_own_start() {
        // An ideal separation adds no impulse: each body leaves with the velocity its own centre
        // of mass already had, so the bodies' momenta add to the stack's.
        let air = UniformAir::sea_level();
        let assembly = two_stage().assemble("j760-i175").unwrap();
        let devices = vec![
            Device::new(
                "sustainer",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.5),
                Trigger::Altitude {
                    height_above_ground_m: 1_500.0,
                },
            ),
            Device::new(
                "booster",
                DeviceDrag::tumbling_stages(&assembly, (1, 1)).unwrap(),
                Trigger::Altitude {
                    height_above_ground_m: 1_500.0,
                },
            )
            .on_body(1),
        ];
        let sim = staged_flight(
            analytic_wind_environment(air, G, ConstantWind::new(5.0, 0.9).unwrap()),
            devices,
            Separation::new(Trigger::Apogee, 0),
        );
        // Separating with a body rate, so that `ω × r` is part of each body's start.
        let mut start = dropped(&sim, 2_000.0, DVec3::new(3.0, 0.0, -2.0));
        start.body_rate_rad_s = DVec3::new(0.0, 0.6, 0.0);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let separation = result.event(EventKind::Separation).unwrap().sample;
        let whole_kg = sim.assembly().mass_properties(separation.time_s).mass_kg;
        let momentum: DVec3 = result
            .bodies
            .iter()
            .map(|body| body.start_sample.velocity_enu_m_s * body.mass_kg)
            .sum();
        let expected = separation.cg_velocity_enu_m_s * whole_kg;
        assert!(
            (momentum - expected).length() < 1e-9 * expected.length(),
            "{momentum} vs {expected}"
        );
        // And each body starts where its own centre of mass was, which is not the stack's.
        let centres: Vec<DVec3> = result
            .bodies
            .iter()
            .map(|body| {
                body.events
                    .first()
                    .map_or(body.final_sample, |e| e.sample)
                    .cg_enu_m
            })
            .collect();
        assert!(
            (centres[0] - centres[1]).length() > 0.5,
            "the bodies should start apart: {centres:?}"
        );
    }

    #[test]
    fn separations_outside_their_domain_are_refused() {
        let environment = || analytic_environment(UniformAir::sea_level(), G);
        let canopy = DeviceDrag::canopy(CanopyType::FlatCircular, 1.5);
        let build = |devices: Vec<Device>, separation: Separation| {
            Simulation::new(
                &two_stage(),
                "j760-i175",
                environment(),
                Rail::vertical(6.0),
                FlightSettings::default(),
            )
            .unwrap()
            .with_recovery(devices)
            .unwrap()
            .with_separation(separation)
            .map(|_| ())
        };
        let both = || {
            vec![
                Device::new("sustainer", canopy, Trigger::Apogee),
                Device::new("booster", canopy, Trigger::Apogee).on_body(1),
            ]
        };
        // No stage aft of the split.
        let error = build(both(), Separation::new(Trigger::Apogee, 1)).expect_err("no aft stage");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
        // A body with no device would fall in a vacuum.
        let error = build(
            vec![Device::new("sustainer", canopy, Trigger::Apogee)],
            Separation::new(Trigger::Apogee, 0),
        )
        .expect_err("the booster has nothing");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
        // A device on a body the separation doesn't make.
        let error = build(
            vec![
                Device::new("sustainer", canopy, Trigger::Apogee),
                Device::new("booster", canopy, Trigger::Apogee).on_body(1),
                Device::new("ghost", canopy, Trigger::Apogee).on_body(2),
            ],
            Separation::new(Trigger::Apogee, 0),
        )
        .expect_err("a third body");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
        // And the ordinary case is accepted.
        assert!(build(both(), Separation::new(Trigger::Apogee, 0)).is_ok());
    }

    #[test]
    fn devices_outside_their_domain_are_refused() {
        let environment = || analytic_environment(UniformAir::sea_level(), G);
        let rocket = design("rocketpy-valetudo");
        let build = |devices: Vec<Device>| {
            Simulation::new(
                &rocket,
                "example",
                environment(),
                Rail::vertical(3.0),
                FlightSettings::default(),
            )
            .unwrap()
            .with_recovery(devices)
            .map(|_| ())
        };
        let canopy = DeviceDrag::canopy(CanopyType::FlatCircular, 1.0);
        let apogee = Trigger::Apogee;
        for devices in [
            vec![Device::new(
                "zero",
                DeviceDrag::DragArea { cd_s_m2: 0.0 },
                apogee,
            )],
            vec![Device::new(
                "nan",
                DeviceDrag::DragArea { cd_s_m2: f64::NAN },
                apogee,
            )],
            vec![Device::new("negative lag", canopy, apogee).with_lag_s(-1.0)],
            vec![Device::new(
                "ground",
                canopy,
                Trigger::Altitude {
                    height_above_ground_m: 0.0,
                },
            )],
            vec![Device::new(
                "before ignition",
                canopy,
                Trigger::Time { time_s: -1.0 },
            )],
            vec![Device::new(
                "no such motor",
                canopy,
                Trigger::MotorDelay { motor: 7 },
            )],
            vec![Device::new("itself", canopy, apogee).with_release_by(0)],
            vec![Device::new("no such device", canopy, apogee).with_release_by(3)],
            vec![
                Device::new("no diameter", DeviceDrag::DragArea { cd_s_m2: 1.0 }, apogee)
                    .with_inflation(Inflation::FillConstant {
                        constant: 8.0,
                        exponent: 2.0,
                    }),
            ],
            vec![Device::new("bad exponent", canopy, apogee).with_inflation(
                Inflation::FillingTime {
                    time_s: 1.0,
                    exponent: 0.0,
                },
            )],
        ] {
            let name = devices[0].name.clone();
            let error = build(devices).expect_err(&name);
            assert!(
                matches!(error, SimError::Domain { .. }),
                "{name}: {error:?}"
            );
        }
        // A cycle of releases is refused: it could leave nothing open.
        let cycle = build(vec![
            Device::new("a", canopy, apogee).with_release_by(1),
            Device::new("b", canopy, apogee).with_release_by(0),
        ])
        .expect_err("a release cycle");
        assert!(matches!(cycle, SimError::Domain { .. }), "{cycle:?}");
        // A chain that ends is fine.
        assert!(
            build(vec![
                Device::new("a", canopy, apogee).with_release_by(1),
                Device::new("b", canopy, apogee).with_release_by(2),
                Device::new("c", canopy, apogee),
            ])
            .is_ok()
        );
        // Valetudo's motor has no ejection charge in its example, so it can't fire a device.
        let error = build(vec![Device::new(
            "no charge",
            canopy,
            Trigger::MotorDelay { motor: 0 },
        )])
        .expect_err("a motor with no delay");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
        // With a delay set, it can.
        assert!(
            Simulation::new(
                &with_delay(rocket.clone(), 3.0),
                "example",
                environment(),
                Rail::vertical(3.0),
                FlightSettings::default(),
            )
            .unwrap()
            .with_recovery(vec![Device::new(
                "charge",
                canopy,
                Trigger::MotorDelay { motor: 0 }
            )])
            .is_ok()
        );
        // And a device that is fine passes.
        assert!(build(vec![Device::new("fine", canopy, apogee).with_lag_s(1.5)]).is_ok());
    }

    #[test]
    fn oversized_canopy_and_ten_km_descent_land_without_step_collapse() {
        // Loft lesson L28. Loft's explicit RK4 needed a 2e-4 s step floor under a stiff canopy,
        // and its 1200 s cap left slow descents from height unlanded: a 10 km descent at 3 m/s
        // takes an hour. Here an oversized canopy opens at 100 m/s at 10 km, and the adaptive
        // integrator flies the whole descent in a few hundred steps.
        let air = UniformAir::sea_level();
        let device = open_at_start(DeviceDrag::canopy(CanopyType::FlatCircular, 5.0));
        let drag_area_m2 = device.drag.drag_area_m2();
        let sim = flight(analytic_environment(air, G), vec![device], 3600.0);
        let mass_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let terminal_m_s = terminal_speed_m_s(mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        // Measured: 2.95 m/s under a 5 m canopy, so the descent takes about 3,400 s.
        assert!(terminal_m_s < 3.5, "{terminal_m_s}");
        let result = sim
            .run_free(
                START_S,
                dropped(&sim, 10_000.0, DVec3::new(0.0, 0.0, -100.0)),
                &mut (),
            )
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let landing = result.event(EventKind::GroundHit).unwrap().sample;
        assert!(
            (-landing.vertical_speed_m_s / terminal_m_s - 1.0).abs() < 1e-6,
            "{}",
            landing.vertical_speed_m_s
        );
        // The descent takes about 10 km / v_t, and the flight has to reach the ground inside the
        // default one-hour cap.
        let flown_s = landing.time_s - START_S;
        assert!(
            (flown_s - 10_000.0 / terminal_m_s).abs() < 60.0,
            "{flown_s} s"
        );
        assert!(flown_s < 3_600.0, "{flown_s} s");
        // No step collapse: the mean step stays near half a second, where Loft's explicit RK4
        // needed a 2e-4 s floor. Measured: 6,914 accepted steps and 2 rejected over 3,392 s, a
        // mean step of 0.49 s, against the 1.7e7 steps Loft's floor would have taken. The step is
        // limited by the tolerances (unit weights on a 10 km height), not by stiffness.
        let mean_step_s = flown_s / result.stats.accepted_steps as f64;
        assert!(
            mean_step_s > 0.1,
            "{mean_step_s} s mean step: {:?}",
            result.stats
        );
        assert!(result.stats.accepted_steps < 20_000, "{:?}", result.stats);
        assert!(result.stats.rejected_steps < 100, "{:?}", result.stats);
    }
}
