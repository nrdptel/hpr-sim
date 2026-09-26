//! Drag: the terms of Niskanen's zero-lift drag buildup and the angle-of-attack scaling of axial
//! drag, as functions of their inputs. [`crate::AeroModel::drag`] sums them over a rocket.
//!
//! The zero-lift drag coefficient on the reference area is (Niskanen 2009 eq. 3.75, 3.97)
//! `C_D0 = C_D,friction + Σ (A_T/A_ref)(C_D•)_T`, each pressure, base and parasitic term `T` taken
//! on its own area `A_T`:
//!
//! - **Skin friction** ([`skin_friction_coefficient`]): a fully turbulent boundary layer (Niskanen
//!   §3.4.1) with the Reynolds number `R = V L/ν` on the rocket's length, limited by roughness
//!   (eq. 3.78–3.81) and corrected for compressibility (eq. 3.82–3.84). Wetted areas are weighted
//!   by the body form factor `1 + 1/(2 f_B)` and the fin thickness factor `1 + 2t/c̄` (eq. 3.85).
//! - **Body pressure drag**: noses, shoulders and steps up in radius from `0.8 sin² φ` at rest
//!   (eq. 3.86) through Mach 1 to appendix B's wave drag ([`crate::nose_drag`]); boattails by the
//!   boattail rule (eq. 3.88, [`boattail_factor`]) to Mach 0.8 and their supersonic wave drag
//!   from Mach 1 ([`crate::afterbody`]); a lip in a boattail's wake loses a share of its own.
//! - **Base drag** ([`base_drag_coefficient`], eq. 3.94) on the aft base, less the thrusting
//!   motors' area, relieved behind a boattail faster than sound ([`crate::afterbody`]).
//! - **Fin pressure drag** ([`fin_pressure_drag_coefficient`], eq. 3.89–3.93) on the fins' frontal
//!   area `N t s`.
//! - **Parasitic drag** of launch lugs and rail buttons ([`launch_lug_drag`], eq. 3.95–3.96, and
//!   Niskanen's rail-pin rule, p. 52).
//! - **Angle of attack** ([`axial_drag_alpha_factor`], §3.4.7): `C_A = C_D0 f(α)`.
//!
//! Interference drag and fin-tip vortices are neglected, as in Niskanen p. 41.
//!
//! Every term has its transonic and supersonic branch, and the buildup covers Mach 0 to 5
//! ([`BUILDUP_MACH_LIMIT`]).
//!
//! See `docs/physics/aero.md` and the decision records on subsonic drag and drag override tables,
//! [ADR-009][adr-009], on drag through Mach 1, [ADR-028][adr-028], and on the afterbody faster
//! than sound, [ADR-030][adr-030].
//!
//! [adr-009]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17
//! [adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18
//! [adr-030]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-030-the-afterbody-faster-than-sound-a-boattails-wave-drag-the-base-behind-it-and-a-lip-in-its-wake-2026-09-18

use hpr_core::interp::Lookup;
use hpr_design::{FinCrossSection, FinSet, LaunchLug, NoseShape, PlacedComponent, RailButton};
use serde::{Deserialize, Serialize};

use crate::afterbody::Boattail;
use crate::body::BodyGeometry;
use crate::error::{AeroError, check_dimension};
use crate::fins::FinGeometry;
use crate::nose_drag::PressureDragCurve;

/// Reynolds number below which the friction formulas no longer hold and the coefficient is held
/// at its value there (Niskanen 2009 p. 44).
pub const LOW_REYNOLDS: f64 = 1.0e4;

/// The skin-friction coefficient below [`LOW_REYNOLDS`] (Niskanen 2009 eq. 3.81).
pub const LOW_REYNOLDS_FRICTION: f64 = 1.48e-2;

/// The top of the subsonic region, Mach 0.8 (Niskanen 2009 Table 3.1, p. 19), where Niskanen's
/// semi-empirical transonic method starts (p. 47): the lower bound `M_L` of a step's and a blunt
/// face's transonic method ([`crate::nose_drag::PressureDragCurve::step`]).
pub const SUBSONIC_MACH_LIMIT: f64 = 0.8;

/// The top of the buildup's range, which it doesn't reach: Mach 5, where the hypersonic region
/// begins (Niskanen 2009 Table 3.1, p. 19), as for the normal force. Niskanen expects the
/// simulation "to be reasonably accurate to at least Mach 1.5" (p. 94); how far it holds against
/// measurements is in `docs/physics/aero.md`.
pub const BUILDUP_MACH_LIMIT: f64 = 5.0;

/// Checks a Mach number of any speed regime: finite and non-negative.
pub(crate) fn check_mach_any(mach: f64) -> Result<(), AeroError> {
    if mach.is_finite() && mach >= 0.0 {
        Ok(())
    } else {
        Err(AeroError::Domain {
            what: "Mach number",
            value: mach,
        })
    }
}

/// Smooth fully turbulent skin friction `C_f = 1/(1.50 ln R − 5.6)²` (Niskanen 2009 eq. 3.78;
/// Barrowman 1967 eq. 4-4 in base-10 logarithms), held at [`LOW_REYNOLDS_FRICTION`] below
/// [`LOW_REYNOLDS`] (eq. 3.81). Incompressible.
fn turbulent_friction(reynolds: f64) -> f64 {
    if reynolds < LOW_REYNOLDS {
        LOW_REYNOLDS_FRICTION
    } else {
        let d = 1.50 * reynolds.ln() - 5.6;
        1.0 / (d * d)
    }
}

/// The roughness-limited critical Reynolds number `R_crit = 51 (R_s/L)^−1.039` (Niskanen 2009
/// eq. 3.79; Barrowman 1967 eq. 4-7), infinite for a perfectly smooth surface.
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite relative roughness.
pub fn critical_reynolds(relative_roughness: f64) -> Result<f64, AeroError> {
    check_dimension("relative roughness", relative_roughness, true)?;
    Ok(51.0 * relative_roughness.powf(-1.039))
}

/// Incompressible skin-friction coefficient of a fully turbulent boundary layer at Reynolds
/// number `reynolds` on a surface of relative roughness `R_s/L` (Niskanen 2009 eq. 3.81):
///
/// ```text
/// C_f = 1.48e-2                     R < 1e4
///     = 1/(1.50 ln R − 5.6)²        1e4 ≤ R < R_crit     (eq. 3.78)
///     = 0.032 (R_s/L)^0.2           R ≥ R_crit, R ≥ 1e4  (eq. 3.80)
/// ```
///
/// As printed, the piecewise form jumps at `R_crit`: eq. 3.79 is not where eq. 3.78 and 3.80
/// cross (about +9% for 60 µm on 1 m). hpr keeps the published form.
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Reynolds number or relative roughness.
pub fn incompressible_skin_friction(
    reynolds: f64,
    relative_roughness: f64,
) -> Result<f64, AeroError> {
    check_dimension("Reynolds number", reynolds, true)?;
    Ok(if roughness_limited(reynolds, relative_roughness)? {
        0.032 * relative_roughness.powf(0.2)
    } else {
        turbulent_friction(reynolds)
    })
}

/// Whether eq. 3.81 takes the roughness-limited branch: from `R_crit`, and never below `1e4`,
/// where the low-Reynolds value applies first.
fn roughness_limited(reynolds: f64, relative_roughness: f64) -> Result<bool, AeroError> {
    Ok(reynolds >= LOW_REYNOLDS && reynolds >= critical_reynolds(relative_roughness)?)
}

/// Skin-friction coefficient with compressibility (Niskanen 2009 eq. 3.82–3.84; Barrowman 1967
/// eq. 4-12, 4-13):
///
/// - `M < 1`: `C_fc = C_f (1 − 0.1 M²)` on either branch of [`incompressible_skin_friction`];
/// - `M ≥ 1`, turbulent: `C_fc = C_f/(1 + 0.15 M²)^0.58`;
/// - `M ≥ 1`, roughness-limited: `C_fc = C_f/(1 + 0.18 M²)`, but never below the turbulent
///   value at the same Reynolds number.
///
/// The subsonic and supersonic corrections differ at `M = 1` (0.900 against 0.922 of `C_f`
/// turbulent, 0.847 roughness-limited), as published; no transonic blend is given.
///
/// # Errors
///
/// As [`incompressible_skin_friction`], and [`AeroError::Domain`] for a negative or non-finite
/// Mach number.
pub fn skin_friction_coefficient(
    reynolds: f64,
    relative_roughness: f64,
    mach: f64,
) -> Result<f64, AeroError> {
    check_mach_any(mach)?;
    let cf = incompressible_skin_friction(reynolds, relative_roughness)?;
    let m2 = mach * mach;
    if mach < 1.0 {
        // Throwaway: a deliberately perturbed drag coefficient, 2% more skin friction, to show
        // the accuracy census failing CI (M2.4). Never merged.
        return Ok(1.02 * cf * (1.0 - 0.1 * m2));
    }
    let turbulent = turbulent_friction(reynolds) / (1.0 + 0.15 * m2).powf(0.58);
    Ok(if roughness_limited(reynolds, relative_roughness)? {
        (cf / (1.0 + 0.18 * m2)).max(turbulent)
    } else {
        turbulent
    })
}

/// Body friction form factor `1 + 1/(2 f_B)` for a body of fineness ratio `f_B` = body length
/// over maximum body diameter (Niskanen 2009 eq. 3.85; Barrowman 1967 eq. 4-16).
///
/// # Errors
///
/// [`AeroError::Domain`] for a non-positive or non-finite fineness ratio.
pub fn body_friction_form_factor(fineness_ratio: f64) -> Result<f64, AeroError> {
    check_dimension("body fineness ratio", fineness_ratio, false)?;
    Ok(1.0 + 0.5 / fineness_ratio)
}

/// Fin friction thickness factor `1 + 2t/c̄`, with `t` the fin thickness and `c̄` the mean
/// aerodynamic chord (Niskanen 2009 eq. 3.85).
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative thickness or a non-positive chord.
pub fn fin_friction_thickness_factor(
    thickness_m: f64,
    mac_length_m: f64,
) -> Result<f64, AeroError> {
    check_dimension("fin thickness", thickness_m, true)?;
    check_dimension("fin mean aerodynamic chord", mac_length_m, false)?;
    Ok(1.0 + 2.0 * thickness_m / mac_length_m)
}

/// Stagnation-pressure ratio `q_stag/q` (Niskanen 2009 eq. B.1, after Hoerner pp. 15-2, 16-3):
/// `1 + M²/4 + M⁴/40` below Mach 1 and `1.84 − 0.76/M² + 0.166/M⁴ + 0.035/M⁶` from Mach 1 (1.275
/// and 1.281 at `M = 1`).
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Mach number.
pub fn stagnation_pressure_ratio(mach: f64) -> Result<f64, AeroError> {
    check_mach_any(mach)?;
    Ok(stagnation_ratio(mach))
}

/// [`stagnation_pressure_ratio`] at a Mach number already checked.
pub(crate) fn stagnation_ratio(mach: f64) -> f64 {
    let m2 = mach * mach;
    if mach < 1.0 {
        1.0 + 0.25 * m2 + m2 * m2 / 40.0
    } else {
        let i2 = 1.0 / m2;
        1.84 - 0.76 * i2 + 0.166 * i2 * i2 + 0.035 * i2 * i2 * i2
    }
}

/// Pressure drag of a blunt circular cylinder face, `(C_D•)_stag = 0.85 q_stag/q` on its frontal
/// area (Niskanen 2009 eq. B.2).
///
/// # Errors
///
/// As [`stagnation_pressure_ratio`].
pub fn stagnation_drag_coefficient(mach: f64) -> Result<f64, AeroError> {
    Ok(0.85 * stagnation_pressure_ratio(mach)?)
}

/// Base drag `(C_D•)_base` on the base area: `0.12 + 0.13 M²` below Mach 1 and `0.25/M` from
/// Mach 1, continuous at 0.25 (Niskanen 2009 eq. 3.94, after Fleeman).
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Mach number.
pub fn base_drag_coefficient(mach: f64) -> Result<f64, AeroError> {
    check_mach_any(mach)?;
    Ok(if mach < 1.0 {
        0.12 + 0.13 * mach * mach
    } else {
        0.25 / mach
    })
}

/// Pressure drag at rest of a nose cone or shoulder, `(C_D•)_p,0 = 0.8 sin² φ` on its frontal
/// area (a nose's base area, or a shoulder's increase in area), with `φ` the joint angle between
/// the surface and the body axis at the aft joint (Niskanen 2009 eq. 3.86, after NAVWEPS 1488
/// p. 237). A smooth joint (`φ = 0`) has none; a bare step (`φ = π/2`) has 0.8.
///
/// It holds "only at low subsonic velocities"; eq. 3.87 carries it to the transonic method
/// ([`crate::nose_drag`]).
///
/// # Errors
///
/// [`AeroError::Domain`] for a joint angle outside `[0, π/2]`.
pub fn joint_pressure_drag_coefficient(joint_angle_rad: f64) -> Result<f64, AeroError> {
    if !(0.0..=std::f64::consts::FRAC_PI_2).contains(&joint_angle_rad) {
        return Err(AeroError::Domain {
            what: "joint angle",
            value: joint_angle_rad,
        });
    }
    let s = joint_angle_rad.sin();
    Ok(0.8 * s * s)
}

/// The boattail rule's share of base drag (Niskanen 2009 eq. 3.88), from the length ratio
/// `γ = l/(d₁ − d₂)`: 1 for `γ ≤ 1`, `(3 − γ)/2` between 1 and 3, and 0 from 3. A boattail's
/// pressure drag is this factor times the base drag coefficient on the boattail's decrease in
/// area, so a zero-length boattail drags like the base it uncovers.
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite length, or diameters that don't decrease.
pub fn boattail_factor(
    length_m: f64,
    fore_diameter_m: f64,
    aft_diameter_m: f64,
) -> Result<f64, AeroError> {
    check_dimension("boattail length", length_m, true)?;
    check_dimension("boattail aft diameter", aft_diameter_m, true)?;
    if !(fore_diameter_m.is_finite() && fore_diameter_m > aft_diameter_m) {
        return Err(AeroError::Domain {
            what: "boattail fore diameter",
            value: fore_diameter_m,
        });
    }
    let gamma = length_m / (fore_diameter_m - aft_diameter_m);
    Ok(if gamma <= 1.0 {
        1.0
    } else if gamma < 3.0 {
        0.5 * (3.0 - gamma)
    } else {
        0.0
    })
}

/// Pressure drag of a fin set on its frontal area `N t s` (Niskanen 2009 eq. 3.89–3.93): the
/// leading edge's `(C_D•)_LE⊥ cos² Γ_L` plus the trailing edge's share of base drag.
///
/// | cross-section | leading edge, `(C_D•)_LE⊥` | trailing edge |
/// |---|---|---|
/// | square | blunt face, [`stagnation_drag_coefficient`] (eq. 3.90) | base drag (eq. 3.92) |
/// | rounded | cylinder in crossflow (eq. 3.89) | half the base drag |
/// | airfoil | cylinder in crossflow (eq. 3.89) | none |
///
/// with eq. 3.89 (after Barrowman 1967 eq. 4-17–4-19): `(1 − M²)^−0.417 − 1` below Mach 0.9,
/// `1 − 1.785 (M − 0.9)` to Mach 1, and `1.214 − 0.502/M² + 0.1095/M⁴` above. `Γ_L` is the
/// leading-edge sweep, averaged over the span for curved edges (eq. 3.91).
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Mach number or a
/// sweep outside `(−π/2, π/2)`, and [`AeroError::Unsupported`] for a cross-section this model
/// doesn't know.
pub fn fin_pressure_drag_coefficient(
    cross_section: FinCrossSection,
    leading_edge_sweep_rad: f64,
    mach: f64,
) -> Result<f64, AeroError> {
    check_mach_any(mach)?;
    let half_pi = std::f64::consts::FRAC_PI_2;
    if leading_edge_sweep_rad.is_nan() || leading_edge_sweep_rad.abs() >= half_pi {
        return Err(AeroError::Domain {
            what: "fin leading-edge sweep",
            value: leading_edge_sweep_rad,
        });
    }
    let rounded = || {
        if mach < 0.9 {
            (1.0 - mach * mach).powf(-0.417) - 1.0
        } else if mach < 1.0 {
            1.0 - 1.785 * (mach - 0.9)
        } else {
            let i2 = 1.0 / (mach * mach);
            1.214 - 0.502 * i2 + 0.1095 * i2 * i2
        }
    };
    let base = base_drag_coefficient(mach)?;
    let (leading, trailing) = match cross_section {
        FinCrossSection::Square => (stagnation_drag_coefficient(mach)?, base),
        FinCrossSection::Rounded => (rounded(), 0.5 * base),
        FinCrossSection::Airfoil => (rounded(), 0.0),
        // A new cross-section needs a drag decision.
        _ => {
            return Err(AeroError::Unsupported(
                "this fin cross-section (no drag model)".to_owned(),
            ));
        }
    };
    let c = leading_edge_sweep_rad.cos();
    Ok(leading * c * c + trailing)
}

/// Parasitic drag of a launch lug (Niskanen 2009 eq. 3.95–3.96): the coefficient
/// `max{1.3 − 0.3 l/d, 1} (C_D•)_stag` on the area `π r_ext² − π r_int² max{1 − l/d, 0}`, returned
/// as `(coefficient, area_m2)`.
///
/// A short lug blocks only its wall's annulus and drags like a wire (Hoerner's 1.1, the 1.3
/// factor); a lug longer than its diameter blocks its whole face like a solid protrusion. `d` is
/// taken as the outer diameter `2 r_ext`: Niskanen's rail pin, a solid lug "with a length equal to
/// its diameter", only reads that way.
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Mach number, or for a
/// negative length, a non-positive outer radius, or an inner radius outside `[0, r_ext]`.
pub fn launch_lug_drag(
    length_m: f64,
    outer_radius_m: f64,
    inner_radius_m: f64,
    mach: f64,
) -> Result<(f64, f64), AeroError> {
    let (factor, area) = launch_lug_factor_and_area(length_m, outer_radius_m, inner_radius_m)?;
    Ok((factor * stagnation_drag_coefficient(mach)?, area))
}

/// [`launch_lug_drag`]'s Mach-independent parts: the length factor `max{1.3 − 0.3 l/d, 1}` and the
/// area, m².
fn launch_lug_factor_and_area(
    length_m: f64,
    outer_radius_m: f64,
    inner_radius_m: f64,
) -> Result<(f64, f64), AeroError> {
    check_dimension("launch lug length", length_m, true)?;
    check_dimension("launch lug outer radius", outer_radius_m, false)?;
    check_dimension("launch lug inner radius", inner_radius_m, true)?;
    if inner_radius_m > outer_radius_m {
        return Err(AeroError::Domain {
            what: "launch lug inner radius",
            value: inner_radius_m,
        });
    }
    let l_over_d = length_m / (2.0 * outer_radius_m);
    let pi = std::f64::consts::PI;
    let area = pi * outer_radius_m * outer_radius_m
        - pi * inner_radius_m * inner_radius_m * (1.0 - l_over_d).max(0.0);
    Ok(((1.3 - 0.3 * l_over_d).max(1.0), area))
}

/// Parasitic drag coefficient of a rail button on its frontal area (the side profile of its base,
/// waist and flange): Niskanen's rail-pin rule (2009 p. 52), a pin drags like a solid launch lug
/// as long as its diameter, `(C_D•)_stag` (Hoerner p. 5-8 gives 0.80 for a pin on a wall).
///
/// # Errors
///
/// As [`stagnation_drag_coefficient`].
pub fn rail_button_drag_coefficient(mach: f64) -> Result<f64, AeroError> {
    stagnation_drag_coefficient(mach)
}

/// The scaling of axial drag with angle of attack, `C_A(α) = C_D0 f(α)` (Niskanen 2009 §3.4.7),
/// with `C_A` positive along `−z_B` (toward the tail).
///
/// Niskanen describes, without coefficients, a two-part polynomial from `f = 1` at `α = 0` up to
/// 1.3 at 17° and down to 0 at 90°, with zero slope at all three. hpr uses the lowest-degree
/// polynomials that meet those conditions, a cubic on each part:
///
/// ```text
/// f = 1 + 0.3 (3t² − 2t³),   t = α/17°,           0 ≤ α ≤ 17°
/// f = 1.3 (1 − 3u² + 2u³),   u = (α − 17°)/73°,   17° ≤ α ≤ 90°
/// ```
///
/// Past 90° the flow meets the tail first and drag pushes toward the nose; hpr mirrors with the
/// sign reversed, `f(α) = −f(180° − α)` (an assumption; the source stops at 90°), so `f` is
/// continuous through 0 at 90°. The coefficients are derived, not published (the decision record
/// on subsonic drag, [ADR-009][adr-009]).
///
/// [adr-009]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17
///
/// # Errors
///
/// [`AeroError::Domain`] for an angle outside `[0, π]`.
pub fn axial_drag_alpha_factor(alpha_rad: f64) -> Result<f64, AeroError> {
    if !(0.0..=std::f64::consts::PI).contains(&alpha_rad) {
        return Err(AeroError::Domain {
            what: "angle of attack",
            value: alpha_rad,
        });
    }
    let degrees = alpha_rad.to_degrees();
    let (a, sign) = if degrees > 90.0 {
        (180.0 - degrees, -1.0)
    } else {
        (degrees, 1.0)
    };
    Ok(sign
        * if a <= 17.0 {
            let t = a / 17.0;
            1.0 + 0.3 * t * t * (3.0 - 2.0 * t)
        } else {
            let u = (a - 17.0) / 73.0;
            1.3 * (1.0 - u * u * (3.0 - 2.0 * u))
        })
}

/// What the drag buildup needs beyond the [`crate::Flow`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct DragConditions {
    /// Freestream Reynolds number per metre, `V/ν`, 1/m. The buildup multiplies it by the rocket's
    /// length (Niskanen 2009 eq. 3.12, p. 42).
    pub reynolds_per_m: f64,
    /// Whether a motor is thrusting. It selects an override table's power-on curve.
    pub thrusting: bool,
    /// Total cross-section area of the motors thrusting into the aft base, m², subtracted from
    /// the base area (Niskanen 2009 p. 50). Zero while coasting.
    pub thrusting_motor_area_m2: f64,
}

impl DragConditions {
    /// Coasting at `reynolds_per_m`.
    pub fn coasting(reynolds_per_m: f64) -> Self {
        Self {
            reynolds_per_m,
            thrusting: false,
            thrusting_motor_area_m2: 0.0,
        }
    }

    /// Thrusting at `reynolds_per_m`, with motors of total cross-section `motor_area_m2` in the aft
    /// base (zero when the area is unknown: then the base drag gets no relief).
    pub fn thrusting(reynolds_per_m: f64, motor_area_m2: f64) -> Self {
        Self {
            reynolds_per_m,
            thrusting: true,
            thrusting_motor_area_m2: motor_area_m2,
        }
    }

    /// Checks that the Reynolds number and the area are finite and non-negative, and that a
    /// coasting rocket has no thrusting area.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] otherwise.
    pub fn validate(&self) -> Result<(), AeroError> {
        check_dimension("Reynolds number per metre", self.reynolds_per_m, true)?;
        check_dimension("thrusting motor area", self.thrusting_motor_area_m2, true)?;
        if !self.thrusting && self.thrusting_motor_area_m2 > 0.0 {
            return Err(AeroError::Domain {
                what: "thrusting motor area while coasting",
                value: self.thrusting_motor_area_m2,
            });
        }
        Ok(())
    }
}

/// A rocket's drag, or one component's share of it, at a flow condition. Coefficients are on the
/// reference area.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Drag {
    /// Zero-lift drag coefficient `C_D0`: the sum of the four parts, or an override table's value.
    pub zero_lift_coefficient: f64,
    /// Axial-force coefficient `C_A = C_D0 f(α)` ([`axial_drag_alpha_factor`]), along `−z_B` when
    /// the flow meets the nose.
    pub axial_coefficient: f64,
    /// Skin friction (eq. 3.85).
    pub friction: f64,
    /// Pressure drag of noses, shoulders, boattails, steps in radius and fins (eq. 3.86–3.93).
    pub pressure: f64,
    /// Base drag of the aft base (eq. 3.94).
    pub base: f64,
    /// Parasitic drag of launch lugs and rail buttons (eq. 3.95–3.96).
    pub parasitic: f64,
    /// Set when an override table gave `C_D0` (the four parts are then zero): the lookup, on the
    /// table's own reference area, and whether it extrapolated.
    pub table: Option<Lookup>,
}

/// One component's share of the drag buildup, at zero lift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentDrag {
    /// The component's id.
    pub id: String,
    /// Its drag. A step in radius belongs to the component aft of it, and the base to the last
    /// body component.
    pub drag: Drag,
}

/// A fin set's pressure-drag inputs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FinPressureTerms {
    /// The fins' cross-section.
    pub cross_section: FinCrossSection,
    /// Leading-edge sweep `Γ_L`, rad.
    pub leading_edge_sweep_rad: f64,
    /// Frontal area `N t s` over the reference area.
    pub frontal_area_ratio: f64,
}

/// A nose's, shoulder's or step's pressure drag: its coefficient against Mach number, on an area.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct PressureDragTerm {
    /// The coefficient on the increase in area (eq. 3.86–3.87, appendix B).
    pub curve: PressureDragCurve,
    /// The increase in area over the reference area.
    pub area_ratio: f64,
}

/// A narrowing transition's pressure drag: as a boattail of its own, or, by merge weights, as its
/// share of the boattails it may continue.
///
/// A share is the drag of the cone from the start of the surface it continues through its aft
/// end, less that of the cone from the same start through its fore end; it can be below 0 where the
/// longer cone drags less, and parts of one straight cone add up to the cone exactly. Each
/// surface ahead that the flow may still follow has a weight: its hold of the flow behind the
/// boattails, times 1 for a turn of up to [`MERGE_FULL_TURN_RAD`] between this part and it, 0 from
/// [`MERGE_NONE_TURN_RAD`] (a corner), linear between, and, when either part is shallower than
/// [`MERGE_MIN_ANGLE_RAD`], times the smaller half-angle over the larger (the larger taken as at
/// most that), so a part narrowing by almost nothing is a tube and a straight cone of any angle
/// merges wholly. The part drags the weights times the shares, plus its own
/// drag times what they leave. So parts of one straight cone add up to one cone, a sharp corner
/// keeps each part its own boattail, and the drag stays between the two.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BoattailTerm {
    /// This transition as a boattail of its own ([`crate::afterbody`]).
    pub own: Boattail,
    /// Its fore area over the reference area.
    pub own_area_ratio: f64,
    /// Its shares of the boattails it continues, each with its part of the merge. Empty when it
    /// continues none.
    pub merged: Vec<MergedBoattail>,
    /// The weight of its own drag: 1 less the merges' weights.
    pub own_weight: f64,
}

/// A narrowing transition's share of a boattail it continues ([`BoattailTerm`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct MergedBoattail {
    /// The cone from the continued surface's start through this transition's aft end.
    pub through_aft: Boattail,
    /// The cone from the same start through this transition's fore end.
    pub through_fore: Boattail,
    /// The cones' fore area, at that start, over the reference area.
    pub area_ratio: f64,
    /// This share's part of the merge, from 0 to 1.
    pub weight: f64,
}

/// A lip in a boattail's wake: its step up's pressure drag is scaled by `1 − step_fraction` and its
/// shoulder's by `1 − shoulder_fraction`.
///
/// NASA's Arcas Robin models end in a lip 1.3 mm long, rising 0.17 of the boattail's drop, whose
/// effect TN D-4014 finds "masked" when the flow over the boattail separates or the boundary layer
/// thickens (Babb and Fuller 1967, p. 6), and which RASAero II's own comparison with the tunnel
/// left out as "buried in the boattail boundary layer" (Rogers 2022, slide 2). A flare back toward
/// the body's full diameter is a compression surface with a drag of its own. Between the two no
/// source gives a measure, so the fraction is a judgement: 1 while the lip's top rises no more
/// than [`WAKE_FULL_RISE`] of the boattail's drop in diameter above the boattail's aft end, 0 from
/// [`WAKE_NONE_RISE`], linear between; and it fades with any tube, step down or part between them
/// over one drop in diameter. A lip may be drawn as a shoulder, as a step up, or as both, and in
/// several parts: each takes the smallest share any top so far leaves, its step by its fore
/// radius and its shoulder by its aft radius too. With several boattails ahead, each contributes
/// its share of the flow; the fractions are summed and capped at 1.
/// A step down counts as a boattail of no length, so a lip behind a plain step, such as a motor
/// retainer behind the step down to the motor tube, is in its wake too.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct WakeTerm {
    /// The boattail ahead that holds the most of the flow: a narrowing part's own, the cone of
    /// the surface it continues, or a step down's corner, a boattail of no length.
    pub boattail: Boattail,
    /// How much of a step up's pressure drag at the lip's fore end the wake removes, from 0 to 1.
    pub step_fraction: f64,
    /// How much of the lip's shoulder's pressure drag the wake removes, from 0 to 1.
    pub shoulder_fraction: f64,
}

/// A lip behind a boattail rising up to this share of the boattail's drop in diameter is wholly
/// in its wake ([`WakeTerm`]).
pub const WAKE_FULL_RISE: f64 = 0.25;

/// A lip behind a boattail rising this share of the boattail's drop in diameter or more is not in
/// its wake ([`WakeTerm`]).
pub const WAKE_NONE_RISE: f64 = 0.5;

/// Two narrowing parts whose half-angles differ by up to this merge wholly ([`BoattailTerm`]), a
/// judgement for a curved boattail drawn in parts: 3°.
pub const MERGE_FULL_TURN_RAD: f64 = 3.0 * std::f64::consts::PI / 180.0;

/// Two narrowing parts whose half-angles differ by this or more don't merge ([`BoattailTerm`]),
/// a corner: 10°.
pub const MERGE_NONE_TURN_RAD: f64 = 10.0 * std::f64::consts::PI / 180.0;

/// The aft base behind a boattail: its drag coefficient is scaled by `1 − Σ w (1 − k)` over its
/// sources, each a boattail the base still takes relief from, with `k` that boattail's
/// base-pressure ratio ([`Boattail::base_pressure_ratio`]) and `w` its share of the flow behind
/// the boattails; the shares add up to at most 1.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BaseBehindBoattail {
    /// The boattails the base takes relief from.
    pub sources: Vec<ReliefSource>,
}

/// A boattail the aft base takes relief from ([`BaseBehindBoattail`]).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ReliefSource {
    /// The boattail: a narrowing part as its own, the cone of the surface it continues, or a step
    /// down's corner, a boattail of no length, fully separated, which gives no relief.
    pub boattail: Boattail,
    /// The base's area over the boattail's fore area, at most 1.
    pub area_ratio: f64,
    /// Its share of the flow behind the boattails, faded by the parts between it and the base,
    /// from 0 to 1.
    pub weight: f64,
}

/// A step down's boattail of no length, for the tails behind a boattail ([`couple_afterbody`]):
/// its length over its drop in radius. Any length this short is a fully separated corner, so the
/// step is the limit of a closure drawn ever shorter.
const STEP_LENGTH_RATIO: f64 = 1e-9;

/// Below this half-angle a narrowing part is partly a tube: it merges with a boattail, and a later
/// part with it, by the smaller of the two angles over the larger, the larger taken as at most
/// this, a judgement: 1°.
pub const MERGE_MIN_ANGLE_RAD: f64 = std::f64::consts::PI / 180.0;

/// 1 with no gap, falling linearly to 0 at a gap of `scale`.
fn gap_weight(gap_m: f64, scale_m: f64) -> f64 {
    if scale_m > 0.0 {
        (1.0 - gap_m / scale_m).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// How far a narrowing part of half-angle `angle_rad` merges with a boattail whose last part has
/// `previous_angle_rad`: by their turn, 1 up to [`MERGE_FULL_TURN_RAD`], 0 from
/// [`MERGE_NONE_TURN_RAD`], linear between; times the smaller angle over the larger, the larger
/// taken as at most [`MERGE_MIN_ANGLE_RAD`], at most 1. So a part narrowing by nothing is a tube
/// on either side of a boattail, and parts of one straight cone merge wholly at any angle.
fn merge_fraction(angle_rad: f64, previous_angle_rad: f64) -> f64 {
    let turn = (angle_rad - previous_angle_rad).abs();
    let smooth = ((MERGE_NONE_TURN_RAD - turn) / (MERGE_NONE_TURN_RAD - MERGE_FULL_TURN_RAD))
        .clamp(0.0, 1.0);
    let (low, high) = (
        angle_rad.min(previous_angle_rad),
        angle_rad.max(previous_angle_rad),
    );
    let steep = if high > 0.0 {
        (low / high.min(MERGE_MIN_ANGLE_RAD)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    smooth * steep
}

/// One boattail surface the flow behind it may follow, with its share of that flow and what the
/// parts since have left of it.
#[derive(Clone, Copy, PartialEq)]
struct Tail {
    /// The start `(x, r)` of the surface a next part would continue.
    start: (f64, f64),
    /// The surface's aft end `(x, r)`: a lip's rise is measured from its radius.
    end: (f64, f64),
    /// The last narrowing part's half-angle, rad: a next part's turn is measured from it.
    angle_rad: f64,
    /// The surface as a boattail: a part's own, or the cone of the surface it continues.
    cone: Boattail,
    /// The cone's drop in diameter, m: the length every fade is measured on.
    fall_m: f64,
    /// Its share of the flow behind the boattails, from 0 to 1.
    share: f64,
    /// The length it fades over since its end, m: tubes' and parts' lengths, and steps' and
    /// narrowing parts' drops in diameter.
    fade_m: f64,
    /// What the lips so far have left of the wake: the smallest share by rise of any top.
    lip: f64,
}

impl Tail {
    /// A narrowing part's own boattail, from its fore end `fore` to its aft end `aft`, `(x, r)`.
    fn own(cone: Boattail, fore: (f64, f64), aft: (f64, f64), share: f64) -> Self {
        Self {
            start: fore,
            end: aft,
            angle_rad: cone.half_angle_rad,
            cone,
            fall_m: cone.fore_diameter_m - cone.aft_diameter_m,
            share,
            fade_m: 0.0,
            lip: 1.0,
        }
    }

    /// The share of a lip whose top is at radius `top_m` that the wake removes by its rise alone.
    fn share_by_rise(&self, top_m: f64) -> f64 {
        let rise = 2.0 * (top_m - self.end.1) / self.fall_m;
        ((WAKE_NONE_RISE - rise) / (WAKE_NONE_RISE - WAKE_FULL_RISE)).clamp(0.0, 1.0)
    }

    /// What it holds of the flow: its share, faded over one fall and by the lips since.
    fn hold(&self) -> f64 {
        self.share * gap_weight(self.fade_m, self.fall_m) * self.lip
    }

    /// The same state as `other` but for the share: the two may be added.
    fn same_as(&self, other: &Self) -> bool {
        Self {
            share: other.share,
            ..*self
        } == *other
    }
}

#[cfg(test)]
thread_local! {
    /// The most tails [`couple_afterbody`] has held at once on this thread, for the tests.
    static PEAK_TAILS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Adds `tail` to `tails`, into a tail in the same state if there is one.
fn add_tail(tails: &mut Vec<Tail>, tail: Tail) {
    match tails.iter_mut().find(|t| t.same_as(&tail)) {
        Some(same) => same.share += tail.share,
        None => tails.push(tail),
    }
}

/// The wake's fraction: the tails' holds added up, at most 1.
fn wake_fraction(tails: &[Tail]) -> f64 {
    tails.iter().map(Tail::hold).sum::<f64>().min(1.0)
}

/// The tail that holds the most, for [`WakeTerm::boattail`].
fn strongest(tails: &[Tail]) -> Option<Boattail> {
    tails
        .iter()
        .fold(None::<&Tail>, |best, t| match best {
            Some(b) if b.hold() >= t.hold() => Some(b),
            _ => Some(t),
        })
        .map(|t| t.cone)
}

/// Couples a rocket's afterbody terms once every body component is built: each narrowing
/// transition's merge with the boattails before it ([`BoattailTerm`]), a lip in a boattail's
/// wake ([`WakeTerm`]), and the aft base behind a boattail ([`BaseBehindBoattail`]). `bodies` are
/// the body components in order, each as its index in `terms` and its geometry, end to end.
///
/// The flow behind the boattails is shared among their tails, the surfaces it may still follow,
/// and each tail holds its share faded by what follows it: over one fall (its drop in diameter),
/// by the length of each tube, lip and part, and the drop of each step down and narrowing part,
/// and by each lip's rise. A narrowing part moves to a continuation of each tail the share it
/// merges with ([`BoattailTerm`]), fades what it leaves as a step and a tube, and takes what no
/// tail then holds as its own boattail; a step down does the same as a boattail of no length,
/// fully separated. The lips and the base add the tails' holds, at most 1. Tails in the same state are one, so
/// there are at most as many as pairs of parts. So a part narrowing by nothing is a tube, a part
/// of no length is a step, every weight is continuous in the geometry, and a small change in a
/// radius or a length changes the drag a little.
pub(crate) fn couple_afterbody(
    terms: &mut [ComponentDragTerms],
    bodies: &[(usize, BodyGeometry)],
    reference_area_m2: f64,
) -> Result<(), AeroError> {
    use std::f64::consts::PI;
    let radius = |area: f64| (area / PI).sqrt();
    let mut tails: Vec<Tail> = Vec::new();
    let mut x = 0.0;
    let mut previous_aft_radius: Option<f64> = None;
    for (index, geometry) in bodies {
        let (x0, x1) = (x, x + geometry.length_m);
        let (r0, r1) = (radius(geometry.fore_area_m2), radius(geometry.aft_area_m2));
        x = x1;
        let before = previous_aft_radius.unwrap_or(r0);
        previous_aft_radius = Some(r1);
        // `index` comes from `body_terms_at`, built alongside `terms` in `AeroModel::new`.
        let terms = &mut terms[*index];
        let mut wake: Option<WakeTerm> = None;
        if r0 < before {
            // A step down: a corner the flow separates at, and a boattail of no length.
            for t in &mut tails {
                t.fade_m += 2.0 * (before - r0);
            }
            tails.retain(|t| t.hold() > 0.0);
            let held: f64 = tails.iter().map(Tail::hold).sum();
            if held < 1.0 {
                let corner =
                    Boattail::new(STEP_LENGTH_RATIO * (before - r0), 2.0 * before, 2.0 * r0)?;
                add_tail(
                    &mut tails,
                    Tail::own(corner, (x0, before), (x0, r0), 1.0 - held),
                );
            }
        } else if r0 > before {
            // A step up: a lip, by its top.
            for t in &mut tails {
                t.lip = t.lip.min(t.share_by_rise(r0));
            }
            wake = strongest(&tails).map(|boattail| WakeTerm {
                boattail,
                step_fraction: wake_fraction(&tails),
                shoulder_fraction: 0.0,
            });
        }
        tails.retain(|t| t.hold() > 0.0);
        if geometry.aft_area_m2 > geometry.fore_area_m2 {
            // A shoulder: a lip, by its top; then its length fades the tails.
            for t in &mut tails {
                t.lip = t.lip.min(t.share_by_rise(r1));
            }
            if let Some(boattail) = strongest(&tails) {
                wake = Some(WakeTerm {
                    boattail,
                    step_fraction: wake.map_or(0.0, |w| w.step_fraction),
                    shoulder_fraction: wake_fraction(&tails),
                });
            }
            for t in &mut tails {
                t.fade_m += geometry.length_m;
            }
        } else if let Some(term) = &mut terms.boattail {
            let own = term.own;
            let angle = own.half_angle_rad;
            let mut next: Vec<Tail> = Vec::with_capacity(2 * tails.len() + 1);
            for t in &mut tails {
                let fraction = merge_fraction(angle, t.angle_rad);
                let weight = fraction * t.hold();
                if weight > 0.0 && x0 > t.start.0 && t.start.1 > r0 {
                    let through_aft = Boattail::new(x1 - t.start.0, 2.0 * t.start.1, 2.0 * r1)?;
                    let through_fore = Boattail::new(x0 - t.start.0, 2.0 * t.start.1, 2.0 * r0)?;
                    match term
                        .merged
                        .iter_mut()
                        .find(|m| m.through_aft == through_aft && m.through_fore == through_fore)
                    {
                        Some(same) => same.weight += weight,
                        None => term.merged.push(MergedBoattail {
                            through_aft,
                            through_fore,
                            area_ratio: PI * t.start.1 * t.start.1 / reference_area_m2,
                            weight,
                        }),
                    }
                    add_tail(
                        &mut next,
                        Tail {
                            angle_rad: angle,
                            ..Tail::own(through_aft, t.start, (x1, r1), weight)
                        },
                    );
                    t.share *= 1.0 - fraction;
                }
                // What it doesn't merge carries on past this part, as a step and a tube.
                t.fade_m += geometry.length_m + 2.0 * (r0 - r1);
            }
            for t in tails.iter().filter(|t| t.hold() > 0.0) {
                add_tail(&mut next, *t);
            }
            let merged: f64 = term.merged.iter().map(|m| m.weight).sum();
            term.own_weight = (1.0 - merged).max(0.0);
            let held: f64 = next.iter().map(Tail::hold).sum();
            if held < 1.0 {
                add_tail(&mut next, Tail::own(own, (x0, r0), (x1, r1), 1.0 - held));
            }
            tails = next;
        } else {
            // A tube, or a narrowing too small to count as a boattail: a gap and a step.
            for t in &mut tails {
                t.fade_m += geometry.length_m + 2.0 * (r0 - r1).max(0.0);
            }
        }
        terms.in_wake_of = wake.filter(|w| w.step_fraction > 0.0 || w.shoulder_fraction > 0.0);
        tails.retain(|t| t.hold() > 0.0);
        #[cfg(test)]
        PEAK_TAILS.with(|p| p.set(p.get().max(tails.len())));
    }
    // The aft base, if the last body component is in a boattail's tail.
    if let Some((index, geometry)) = bodies.last()
        && geometry.aft_area_m2 > 0.0
        && !tails.is_empty()
    {
        let base = geometry.aft_area_m2;
        let mut sources: Vec<ReliefSource> = Vec::new();
        for t in &tails {
            match sources.iter_mut().find(|s| s.boattail == t.cone) {
                Some(same) => same.weight += t.hold(),
                None => sources.push(ReliefSource {
                    boattail: t.cone,
                    area_ratio: (base
                        / (0.25 * PI * t.cone.fore_diameter_m * t.cone.fore_diameter_m))
                        .min(1.0),
                    weight: t.hold(),
                }),
            }
        }
        terms[*index].base_behind = Some(BaseBehindBoattail { sources });
    }
    Ok(())
}

/// A component's precomputed drag terms, built by [`crate::AeroModel::new`]. Areas are divided by
/// the reference area.
///
/// Serialize-only, like [`crate::AeroModel`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct ComponentDragTerms {
    /// The component's id.
    pub id: String,
    /// Friction area (a body's axial projection, both sides of every fin) times the body form
    /// factor or the fin thickness factor: the friction drag is `C_fc` times this (eq. 3.85).
    pub friction_area_ratio: f64,
    /// Relative roughness `R_s/L` of the component's finish on the rocket's length.
    pub relative_roughness: f64,
    /// Pressure drag of a step up in radius at the fore end, or of a bare front face: a flat
    /// face ([`PressureDragCurve::step`]).
    pub step: Option<PressureDragTerm>,
    /// Pressure drag of a nose or shoulder: its own increase in area.
    pub shoulder: Option<PressureDragTerm>,
    /// Why the buildup refuses this component, if it does: a nose or shoulder shape with no
    /// transonic drag data (a bulged secant ogive, a Haack series past `C = ⅓`). The model still
    /// builds, so the normal force and a drag table work; [`crate::AeroModel::drag`] without a
    /// table returns [`AeroError::Unsupported`] for it.
    pub unsupported: Option<String>,
    /// A step down in radius at the fore end: its decrease in area, a boattail of no length
    /// (eq. 3.88 at `γ = 0`), times the base drag coefficient.
    pub boattail_area_ratio: f64,
    /// A transition that narrows over a length: its own pressure drag as a boattail, blended by
    /// the turn toward its share of the boattails it continues ([`BoattailTerm`]), so parts of
    /// one straight cone drag as the cone.
    pub boattail: Option<BoattailTerm>,
    /// A lip in a boattail's wake, drawn as a step up, a shoulder or both, with tubes, steps or
    /// parts between fading it: its step's and shoulder's pressure drag scaled by one less their
    /// fractions ([`WakeTerm`]).
    pub in_wake_of: Option<WakeTerm>,
    /// The boattails the aft base may take relief from, when a boattail lies ahead of it with
    /// only parts between that leave some: the base drag's factor
    /// ([`Boattail::base_pressure_ratio`], [`BaseBehindBoattail`]).
    pub base_behind: Option<BaseBehindBoattail>,
    /// A fin set's pressure-drag inputs.
    pub fins: Option<FinPressureTerms>,
    /// Launch lugs' and rail buttons' areas (a lug's times its length factor), times the
    /// stagnation drag coefficient (eq. 3.95–3.96).
    pub parasitic_area_ratio: f64,
    /// Area of the aft base, m²: the last body component's aft area, zero for the rest.
    pub base_area_m2: f64,
}

impl ComponentDragTerms {
    fn empty(component: &PlacedComponent, length_m: f64) -> Result<Self, AeroError> {
        Ok(Self {
            id: component.id.clone(),
            friction_area_ratio: 0.0,
            relative_roughness: component.finish.roughness_m()? / length_m,
            step: None,
            shoulder: None,
            unsupported: None,
            boattail_area_ratio: 0.0,
            boattail: None,
            in_wake_of: None,
            base_behind: None,
            fins: None,
            parasitic_area_ratio: 0.0,
            base_area_m2: 0.0,
        })
    }

    /// A body component's terms: friction on its surface, the step in area from the previous body
    /// component (`None` for the first, whose fore face counts as a step up from nothing), and its
    /// own pressure drag. `shape` is a nose's or transition's profile shape, `None` for a tube.
    /// A transition that narrows over a length is a [`Boattail`] of its own until
    /// [`couple_afterbody`] joins it with its neighbours.
    ///
    /// A nose or shoulder's fineness ratio is its length over its rise in diameter,
    /// `l/(d_aft − d_fore)`: a nose's `l/d`, and for a shoulder the fineness of the nose with the
    /// same surface angle ([`crate::nose_drag`], ADR-028).
    ///
    /// The friction area is the surface's projection along the axis, `2π ∫ r dx = π A_plan`: the
    /// wall shear acts along the surface, so each element's axial share is `τ cos θ dA`. Niskanen's
    /// wetted area (eq. 3.85) omits the `cos θ`; the difference is small on slender noses, and
    /// without it a shoulder's friction would tend to a flat annulus's as its length goes to zero
    /// (ADR-009).
    pub(crate) fn body(
        component: &PlacedComponent,
        geometry: &BodyGeometry,
        shape: Option<NoseShape>,
        previous_aft_area_m2: Option<f64>,
        form_factor: f64,
        length_m: f64,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        use std::f64::consts::PI;
        let mut terms = Self::empty(component, length_m)?;
        terms.friction_area_ratio =
            form_factor * PI * geometry.planform_area_m2 / reference_area_m2;
        let step = geometry.fore_area_m2 - previous_aft_area_m2.unwrap_or(0.0);
        if step > 0.0 {
            terms.step = Some(PressureDragTerm {
                curve: PressureDragCurve::step(),
                area_ratio: step / reference_area_m2,
            });
        } else if step < 0.0 {
            // A zero-length boattail: `γ = 0`.
            terms.boattail_area_ratio -= step;
        }
        let diameter = |area: f64| 2.0 * (area / PI).sqrt();
        let change = geometry.aft_area_m2 - geometry.fore_area_m2;
        let rise = diameter(geometry.aft_area_m2) - diameter(geometry.fore_area_m2);
        // A widening too small to change the diameter as computed is none.
        if change > 0.0 && rise > 0.0 {
            let shape = shape.ok_or_else(|| {
                AeroError::Layout("a body that widens needs a profile shape".to_owned())
            })?;
            let joint = geometry.aft_angle_rad.max(0.0);
            match PressureDragCurve::new(shape, geometry.length_m / rise, joint) {
                Ok(curve) => {
                    terms.shoulder = Some(PressureDragTerm {
                        curve,
                        area_ratio: change / reference_area_m2,
                    });
                }
                Err(AeroError::Unsupported(why)) => terms.unsupported = Some(why),
                Err(error) => return Err(error),
            }
        } else if change < 0.0 {
            let (fore, aft) = (
                diameter(geometry.fore_area_m2),
                diameter(geometry.aft_area_m2),
            );
            if geometry.length_m > 0.0 && fore > aft {
                terms.boattail = Some(BoattailTerm {
                    own: Boattail::new(geometry.length_m, fore, aft)?,
                    own_area_ratio: geometry.fore_area_m2 / reference_area_m2,
                    merged: Vec::new(),
                    own_weight: 1.0,
                });
            } else {
                // No length, or a narrowing too small to change the diameter as computed: the
                // rule's `γ = 0`, factor 1, as a step.
                terms.boattail_area_ratio -= change;
            }
        }
        terms.boattail_area_ratio /= reference_area_m2;
        Ok(terms)
    }

    /// A fin set's terms: friction on both sides of every fin with the thickness factor, and
    /// pressure drag on the frontal area.
    pub(crate) fn fins(
        component: &PlacedComponent,
        set: &FinSet,
        geometry: &FinGeometry,
        length_m: f64,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        let mut terms = Self::empty(component, length_m)?;
        let count = f64::from(set.count);
        let factor = fin_friction_thickness_factor(set.thickness_m, geometry.mac_length_m)?;
        // Checks the cross-section and the sweep once, here.
        fin_pressure_drag_coefficient(set.cross_section, geometry.leading_edge_sweep_rad, 0.0)?;
        terms.friction_area_ratio = 2.0 * count * geometry.area_m2 * factor / reference_area_m2;
        terms.fins = Some(FinPressureTerms {
            cross_section: set.cross_section,
            leading_edge_sweep_rad: geometry.leading_edge_sweep_rad,
            frontal_area_ratio: count * set.thickness_m * geometry.span_m / reference_area_m2,
        });
        Ok(terms)
    }

    /// A row of launch lugs (eq. 3.95–3.96).
    pub(crate) fn launch_lugs(
        component: &PlacedComponent,
        lug: &LaunchLug,
        length_m: f64,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        let mut terms = Self::empty(component, length_m)?;
        let (factor, area) = launch_lug_factor_and_area(
            lug.length_m,
            lug.outer_radius_m,
            lug.outer_radius_m - lug.thickness_m,
        )?;
        terms.parasitic_area_ratio = f64::from(lug.count) * factor * area / reference_area_m2;
        Ok(terms)
    }

    /// A row of rail buttons, each on its side profile: the base and flange at the outer diameter
    /// and the waist between them at the inner diameter.
    pub(crate) fn rail_buttons(
        component: &PlacedComponent,
        button: &RailButton,
        length_m: f64,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        let mut terms = Self::empty(component, length_m)?;
        check_dimension("rail button outer diameter", button.outer_diameter_m, false)?;
        check_dimension("rail button inner diameter", button.inner_diameter_m, true)?;
        let ends = button.base_height_m + button.flange_height_m;
        let waist = button.height_m - ends;
        check_dimension("rail button base and flange height", ends, true)?;
        check_dimension("rail button waist height", waist, true)?;
        let frontal = button.outer_diameter_m * ends + button.inner_diameter_m * waist;
        terms.parasitic_area_ratio = f64::from(button.count) * frontal / reference_area_m2;
        Ok(terms)
    }

    /// The component's drag at `mach` with the rocket's Reynolds number `reynolds` and the
    /// thrusting motors' area.
    pub(crate) fn evaluate(
        &self,
        reynolds: f64,
        mach: f64,
        thrusting_motor_area_m2: f64,
        reference_area_m2: f64,
    ) -> Result<Drag, AeroError> {
        if let Some(why) = &self.unsupported {
            return Err(AeroError::InComponent {
                id: self.id.clone(),
                source: Box::new(AeroError::Unsupported(why.clone())),
            });
        }
        let friction = if self.friction_area_ratio > 0.0 {
            skin_friction_coefficient(reynolds, self.relative_roughness, mach)?
                * self.friction_area_ratio
        } else {
            0.0
        };
        let base_coefficient = base_drag_coefficient(mach)?;
        let mut pressure = base_coefficient * self.boattail_area_ratio;
        if let Some(term) = &self.boattail {
            let mut merged = 0.0;
            for m in &term.merged {
                let share = m.area_ratio
                    * (m.through_aft.pressure_drag_coefficient(mach)?
                        - m.through_fore.pressure_drag_coefficient(mach)?);
                // It may be below 0: extending a boattail can lower its drag. Merged wholly, the
                // parts' shares add up to the whole cone's drag, which is not.
                merged += m.weight * share;
            }
            if term.own_weight > 0.0 {
                merged += term.own_weight
                    * term.own_area_ratio
                    * term.own.pressure_drag_coefficient(mach)?;
            }
            pressure += merged;
        }
        // A lip in a boattail's wake keeps `1 − fraction` of its step's and shoulder's drag.
        let wake = self.in_wake_of;
        for (term, fraction) in [
            (&self.step, wake.map_or(0.0, |w| w.step_fraction)),
            (&self.shoulder, wake.map_or(0.0, |w| w.shoulder_fraction)),
        ] {
            if let Some(term) = term {
                pressure += (1.0 - fraction) * term.area_ratio * term.curve.coefficient(mach)?;
            }
        }
        if let Some(fins) = &self.fins {
            pressure += fins.frontal_area_ratio
                * fin_pressure_drag_coefficient(
                    fins.cross_section,
                    fins.leading_edge_sweep_rad,
                    mach,
                )?;
        }
        let parasitic = if self.parasitic_area_ratio > 0.0 {
            stagnation_drag_coefficient(mach)? * self.parasitic_area_ratio
        } else {
            0.0
        };
        // The base takes each boattail's relief by its share of the flow.
        let mut relief = 1.0;
        for source in self.base_behind.iter().flat_map(|b| &b.sources) {
            let k = source
                .boattail
                .base_pressure_ratio(mach, source.area_ratio)?;
            relief -= source.weight * (1.0 - k);
        }
        // The shares add up to at most 1, so only rounding takes it below 0; a NaN stays one.
        if relief < 0.0 {
            relief = 0.0;
        }
        let base =
            base_coefficient * relief * (self.base_area_m2 - thrusting_motor_area_m2).max(0.0)
                / reference_area_m2;
        let zero_lift = friction + pressure + base + parasitic;
        Ok(Drag {
            zero_lift_coefficient: zero_lift,
            axial_coefficient: zero_lift,
            friction,
            pressure,
            base,
            parasitic,
            table: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI};

    use hpr_design::{
        FinPlanform, Finish, LaunchLug, NoseShape, Part, Position, RailButton, ReferenceDiameter,
        Rocket,
    };

    use super::*;
    use crate::table::DragTable;
    use crate::testing::{body_part, component, fin_set, material, nose, one_stage};
    use crate::{AeroModel, Flow};

    /// A flat face's pressure drag below Mach 1, by hand: the blunt cylinder
    /// `0.85 (1 + M²/4 + M⁴/40)` (Niskanen 2009 eq. B.1–B.2).
    fn step_by_hand(mach: f64) -> f64 {
        let m2 = mach * mach;
        0.85 * (1.0 + m2 / 4.0 + m2 * m2 / 40.0)
    }

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

    fn model(rocket: &Rocket) -> AeroModel {
        AeroModel::new(&rocket.layout().unwrap()).unwrap()
    }

    /// Sea level, Mach 0.3: `V/ν` for USSA76 (a = 340.294 m/s, ν = 1.4607e-5 m²/s).
    const RE_PER_M: f64 = 0.3 * 340.294 / 1.4607e-5;

    fn fins_on(
        tube: &str,
        radius: f64,
        length: f64,
        sets: Vec<hpr_design::Component>,
    ) -> hpr_design::Component {
        let mut c = component(tube, body_part(length, radius, radius), None);
        c.children = sets;
        c
    }

    fn trapezoid() -> FinPlanform {
        FinPlanform::Trapezoidal {
            root_chord_m: 0.12,
            tip_chord_m: 0.05,
            span_m: 0.06,
            sweep_m: 0.07,
        }
    }

    fn with_fin(
        id: &str,
        count: u32,
        thickness: f64,
        section: FinCrossSection,
        aft_offset: f64,
    ) -> hpr_design::Component {
        let mut part = fin_set(count, trapezoid());
        if let Part::FinSet(set) = &mut part {
            set.thickness_m = thickness;
            set.cross_section = section;
        }
        component(
            id,
            part,
            Some(Position::Bottom {
                aft_offset_m: aft_offset,
            }),
        )
    }

    /// Loft lesson L90: skin friction follows Niskanen eq. 3.81 piecewise, including its published
    /// jump at `R_crit`; friction and the other Mach-dependent terms stay finite and positive to
    /// Mach 5; a fin set split in two drags like one set; base drag is continuous at Mach 1.
    #[test]
    fn skin_friction_follows_eq_3_81_and_drag_invariants_hold() {
        // Below 1e4, the constant; from 1e4 to R_crit, eq. 3.78; from R_crit, eq. 3.80.
        let rr = 60e-6; // 60 µm on 1 m
        assert_eq!(incompressible_skin_friction(9_999.0, rr).unwrap(), 1.48e-2);
        assert_eq!(incompressible_skin_friction(0.0, rr).unwrap(), 1.48e-2);
        close(
            incompressible_skin_friction(1e4, rr).unwrap(),
            1.48e-2,
            2e-3,
            "continuous at 1e4",
        );
        for r in [1e4, 3e4, 1e5, 1e6] {
            let d = 1.50 * f64::ln(r) - 5.6;
            assert_eq!(
                incompressible_skin_friction(r, rr).unwrap(),
                1.0 / (d * d),
                "eq. 3.78 at {r}"
            );
        }
        let critical = critical_reynolds(rr).unwrap();
        assert_eq!(critical, 51.0 * rr.powf(-1.039));
        close(critical, 1.242e6, 1e-3, "R_crit for 60 µm on 1 m");
        let rough = 0.032 * rr.powf(0.2);
        assert_eq!(incompressible_skin_friction(critical, rr).unwrap(), rough);
        assert_eq!(incompressible_skin_friction(1e9, rr).unwrap(), rough);
        // The published jump at R_crit: 0.00419 below, 0.00458 at it.
        let below = incompressible_skin_friction(critical * (1.0 - 1e-12), rr).unwrap();
        close(below, 0.00419, 2e-3, "turbulent just below R_crit");
        close(rough, 0.00458, 2e-3, "roughness-limited at R_crit");
        // A mirror finish never reaches the roughness limit.
        assert_eq!(critical_reynolds(0.0).unwrap(), f64::INFINITY);
        let d = 1.50 * f64::ln(1e9) - 5.6;
        assert_eq!(
            incompressible_skin_friction(1e9, 0.0).unwrap(),
            1.0 / (d * d)
        );

        // Compressibility: 1 − 0.1 M² below Mach 1 on both branches.
        for (r, rr) in [(1e5, rr), (1e9, rr)] {
            let cf = incompressible_skin_friction(r, rr).unwrap();
            close(
                skin_friction_coefficient(r, rr, 0.8).unwrap(),
                cf * (1.0 - 0.064),
                1e-15,
                "eq. 3.82",
            );
        }
        // Supersonic: eq. 3.83 turbulent, eq. 3.84 roughness-limited but never below turbulent.
        let d = 1.50 * f64::ln(1e5) - 5.6;
        close(
            skin_friction_coefficient(1e5, rr, 2.0).unwrap(),
            1.0 / (d * d) / 1.6f64.powf(0.58),
            1e-15,
            "eq. 3.83",
        );
        close(
            skin_friction_coefficient(1e9, rr, 2.0).unwrap(),
            rough / 1.72,
            1e-15,
            "eq. 3.84",
        );
        // At Mach 5 the rough value corrected by eq. 3.84 falls below the turbulent one, which
        // then applies.
        let rr_small = 2e-6;
        for mach in (0..=100).map(|k| 0.05 * f64::from(k)) {
            for r in [0.0, 1e3, 1e4, 1e5, 1e7, 1e9] {
                for rr in [0.0, 2e-6, 60e-6, 1e-3] {
                    let cf = skin_friction_coefficient(r, rr, mach).unwrap();
                    assert!(
                        cf.is_finite() && cf > 0.0,
                        "C_f {cf} at M {mach}, R {r}, R_s/L {rr}"
                    );
                    if mach >= 1.0 {
                        let d = 1.50 * f64::ln(r.max(1e4)) - 5.6;
                        let smooth = if r < 1e4 { 1.48e-2 } else { 1.0 / (d * d) };
                        let floor = smooth / (1.0 + 0.15 * mach * mach).powf(0.58);
                        assert!(cf >= floor * (1.0 - 1e-15), "below turbulent at M {mach}");
                    }
                }
            }
            for f in [
                stagnation_drag_coefficient(mach).unwrap(),
                base_drag_coefficient(mach).unwrap(),
                fin_pressure_drag_coefficient(FinCrossSection::Rounded, 0.3, mach).unwrap(),
                fin_pressure_drag_coefficient(FinCrossSection::Square, 0.3, mach).unwrap(),
            ] {
                assert!(f.is_finite() && f >= 0.0, "term {f} at M {mach}");
            }
        }
        // At R = 1e9 on 2 µm per metre, eq. 3.84 gives 4.2e-4 at Mach 5, below the turbulent
        // 6.2e-4 at the same Reynolds number, which then applies.
        assert!(critical_reynolds(rr_small).unwrap() < 1e9);
        let rough_m5 = 0.032 * rr_small.powf(0.2) / (1.0 + 0.18 * 25.0);
        let turbulent_m5 = {
            let d = 1.50 * f64::ln(1e9) - 5.6;
            1.0 / (d * d) / (1.0f64 + 0.15 * 25.0).powf(0.58)
        };
        assert!(rough_m5 < turbulent_m5);
        assert_eq!(
            skin_friction_coefficient(1e9, rr_small, 5.0).unwrap(),
            turbulent_m5
        );
        // Very rough (R_crit below 1e4): the low-Reynolds value still applies below 1e4.
        assert!(critical_reynolds(2e-2).unwrap() < 5e3);
        assert_eq!(incompressible_skin_friction(5e3, 2e-2).unwrap(), 1.48e-2);
        assert_eq!(
            incompressible_skin_friction(2e4, 2e-2).unwrap(),
            0.032 * 2e-2f64.powf(0.2)
        );

        // Base drag is continuous at Mach 1.
        let below = base_drag_coefficient(1.0 - 1e-12).unwrap();
        let at = base_drag_coefficient(1.0).unwrap();
        close(below, 0.25, 1e-11, "base drag below Mach 1");
        assert_eq!(at, 0.25);

        // Four fins drag like two two-fin sets at the same station, 90° apart.
        let one_set = one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, 0.027),
                    None,
                ),
                fins_on(
                    "tail",
                    0.027,
                    0.6,
                    vec![with_fin("fins", 4, 0.003, FinCrossSection::Rounded, 0.0)],
                ),
            ],
            ReferenceDiameter::Maximum {},
        );
        let mut half_b = with_fin("fins-b", 2, 0.003, FinCrossSection::Rounded, 0.0);
        if let Part::FinSet(set) = &mut half_b.part {
            set.base_angle_rad = FRAC_PI_2;
        }
        let split = one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, 0.027),
                    None,
                ),
                fins_on(
                    "tail",
                    0.027,
                    0.6,
                    vec![
                        with_fin("fins-a", 2, 0.003, FinCrossSection::Rounded, 0.0),
                        half_b,
                    ],
                ),
            ],
            ReferenceDiameter::Maximum {},
        );
        let conditions = DragConditions::coasting(RE_PER_M);
        for alpha in [0.0, 0.1] {
            let flow = Flow::new(0.3, alpha, 0.0);
            let a = model(&one_set).drag(&flow, &conditions).unwrap();
            let b = model(&split).drag(&flow, &conditions).unwrap();
            close(
                b.zero_lift_coefficient,
                a.zero_lift_coefficient,
                1e-15,
                "split C_D0",
            );
            close(b.pressure, a.pressure, 1e-15, "split pressure drag");
            close(b.friction, a.friction, 1e-15, "split friction drag");
        }
    }

    /// Loft lesson L11: fin sets are separate terms, so listing them in another order changes
    /// nothing.
    #[test]
    fn drag_invariant_to_fin_set_order() {
        let sets = || {
            let mut canted = with_fin("aft", 3, 0.004, FinCrossSection::Square, 0.0);
            if let Part::FinSet(set) = &mut canted.part {
                set.planform = FinPlanform::Elliptical {
                    root_chord_m: 0.1,
                    span_m: 0.05,
                };
                set.base_angle_rad = 0.2;
            }
            (
                with_fin("fore", 4, 0.002, FinCrossSection::Airfoil, 0.3),
                canted,
            )
        };
        let build = |sets: Vec<hpr_design::Component>| {
            let rocket = one_stage(
                vec![
                    component("nose", nose(NoseShape::Conical {}, 0.2, 0.027), None),
                    fins_on("tail", 0.027, 0.8, sets),
                ],
                ReferenceDiameter::Maximum {},
            );
            model(&rocket)
        };
        let (a, b) = sets();
        let forward = build(vec![a, b]);
        let (a, b) = sets();
        let reversed = build(vec![b, a]);
        for (mach, alpha, motor) in [(0.1, 0.0, 0.0), (0.5, 0.2, 1e-3), (0.9, 1.0, 0.0)] {
            let flow = Flow::new(mach, alpha, 0.0);
            let conditions = DragConditions::thrusting(RE_PER_M, motor);
            let f = forward.drag(&flow, &conditions).unwrap();
            let r = reversed.drag(&flow, &conditions).unwrap();
            close(
                r.zero_lift_coefficient,
                f.zero_lift_coefficient,
                1e-15,
                "C_D0",
            );
            close(r.axial_coefficient, f.axial_coefficient, 1e-15, "C_A");
            close(r.pressure, f.pressure, 1e-15, "pressure");
            close(r.friction, f.friction, 1e-15, "friction");
        }
    }

    /// Loft lesson L12: the body form factor is Niskanen's `1 + 1/(2 f_B)` (1.125 at fineness 4,
    /// not Loft's 1.95), the fin factor `1 + 2t/c̄`, the subsonic friction correction
    /// `1 − 0.1 M²`, and every named roughness height is Barrowman 1967 Table 4-1's (Niskanen
    /// Table 3.2 reprints ten of them).
    #[test]
    fn form_factor_and_roughness_match_cited_values() {
        assert_eq!(body_friction_form_factor(4.0).unwrap(), 1.125);
        assert_eq!(body_friction_form_factor(10.0).unwrap(), 1.05);
        assert_eq!(fin_friction_thickness_factor(0.003, 0.1).unwrap(), 1.06);
        assert_eq!(fin_friction_thickness_factor(0.0, 0.1).unwrap(), 1.0);
        let cf = incompressible_skin_friction(1e6, 0.0).unwrap();
        close(
            skin_friction_coefficient(1e6, 0.0, 0.5).unwrap(),
            cf * 0.975,
            1e-15,
            "1 − 0.1 M²",
        );

        // Barrowman 1967 Table 4-1, p. 46, in microns, smoothest first.
        let table = [
            0.0, 0.1, 0.5, 2.0, 5.0, 15.0, 20.0, 50.0, 50.0, 100.0, 150.0, 200.0, 250.0, 500.0,
            1000.0,
        ];
        for (finish, microns) in Finish::NAMED.iter().zip(table) {
            close(
                finish.roughness_m().unwrap(),
                microns * 1e-6,
                1e-15,
                &format!("{finish:?}"),
            );
        }
        // Niskanen Table 3.2's ten rows.
        let niskanen = [
            (Finish::AverageGlass {}, 0.1),
            (Finish::Polished {}, 0.5),
            (Finish::OptimumPaint {}, 5.0),
            (Finish::PlanedWood {}, 15.0),
            (Finish::MassProductionPaint {}, 20.0),
            (Finish::SmoothCement {}, 50.0),
            (Finish::DipGalvanized {}, 150.0),
            (Finish::PoorPaint {}, 200.0),
            (Finish::RawWood {}, 500.0),
            (Finish::Concrete {}, 1000.0),
        ];
        for (finish, microns) in niskanen {
            close(
                finish.roughness_m().unwrap(),
                microns * 1e-6,
                1e-15,
                &format!("{finish:?}"),
            );
        }
        assert_eq!(Finish::default(), Finish::MassProductionPaint {});
        assert_eq!(
            Finish::Custom { roughness_m: 6e-5 }.roughness_m().unwrap(),
            6e-5
        );

        // In a model: a 1 m tube and 0.25 m cone, 0.05 m diameter: fineness 25, and the
        // component's roughness over the rocket's length.
        let mut tube = component("tube", body_part(1.0, 0.025, 0.025), None);
        tube.finish = Some(Finish::RawWood {});
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.25, 0.025), None),
                tube,
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let terms = &m.drag_terms()[1];
        close(terms.relative_roughness, 500e-6 / 1.25, 1e-15, "R_s/L");
        let a_ref = PI * 0.025 * 0.025;
        close(
            terms.friction_area_ratio,
            (1.0 + 1.0 / 50.0) * 2.0 * PI * 0.025 * 1.0 / a_ref,
            1e-14,
            "form factor × wetted area",
        );
        // A cone's friction area is its axial projection, π r L, not its slant surface.
        let cone = &m.drag_terms()[0];
        close(
            cone.friction_area_ratio,
            (1.0 + 1.0 / 50.0) * PI * 0.025 * 0.25 / a_ref,
            1e-12,
            "cone projection",
        );
    }

    /// Loft lesson L13: under power the base drag's area is the base less the thrusting motors'
    /// area (Niskanen p. 50), down to none when the motors fill the base.
    #[test]
    fn power_on_base_drag_subtracts_thrusting_motor_area() {
        let r = 0.04;
        let rocket = one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.3, r),
                    None,
                ),
                component("tube", body_part(1.2, r, r), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_base = PI * r * r;
        let motor = PI * 0.027 * 0.027;
        let flow = Flow::axial(0.5);
        let c_base = 0.12 + 0.13 * 0.25;
        let off = m.drag(&flow, &DragConditions::coasting(RE_PER_M)).unwrap();
        let on = m
            .drag(&flow, &DragConditions::thrusting(RE_PER_M, motor))
            .unwrap();
        let full = m
            .drag(&flow, &DragConditions::thrusting(RE_PER_M, a_base))
            .unwrap();
        let over = m
            .drag(&flow, &DragConditions::thrusting(RE_PER_M, 2.0 * a_base))
            .unwrap();
        close(
            off.base,
            c_base,
            1e-15,
            "coasting base drag on A_ref = A_base",
        );
        close(
            on.base,
            c_base * (a_base - motor) / a_base,
            1e-14,
            "power-on base drag",
        );
        assert_eq!(full.base, 0.0);
        assert_eq!(over.base, 0.0);
        // Nothing else changes.
        assert_eq!(on.friction, off.friction);
        assert_eq!(on.pressure, off.pressure);
        close(
            off.zero_lift_coefficient - on.zero_lift_coefficient,
            off.base - on.base,
            1e-12,
            "sum",
        );
        // The base belongs to the last body component.
        let parts = m
            .buildup_components(&flow, &DragConditions::thrusting(RE_PER_M, motor))
            .unwrap();
        assert_eq!(parts[0].drag.base, 0.0);
        assert_eq!(parts[1].drag.base, on.base);

        // An override table switches curves on the same signal.
        let table = DragTable::from_csv("0,0.5\n1,0.5\n", Some("0,0.4\n1,0.4\n")).unwrap();
        let t = m.clone().with_drag_table(table);
        assert_eq!(
            t.drag(&flow, &DragConditions::coasting(RE_PER_M))
                .unwrap()
                .zero_lift_coefficient,
            0.5
        );
        assert_eq!(
            t.drag(&flow, &DragConditions::thrusting(RE_PER_M, motor))
                .unwrap()
                .zero_lift_coefficient,
            0.4
        );
    }

    /// Loft lesson L14: a launch lug's drag is Niskanen eq. 3.95–3.96, `max{1.3 − 0.3 l/d, 1}`
    /// times the blunt-cylinder `0.85 q_stag/q`, on the annulus plus `max{1 − l/d, 0}` of the bore
    /// blocked; a rail button is a rail pin, the stagnation coefficient on its side profile.
    #[test]
    fn launch_lug_drag_matches_cited_hollow_tube_formula() {
        let (ro, ri) = (0.005, 0.004);
        let annulus = PI * (ro * ro - ri * ri);
        let face = PI * ro * ro;
        let stag = |m: f64| 0.85 * (1.0 + m * m / 4.0 + m.powi(4) / 40.0);
        // A ring (l = 0): 1.3 on the annulus.
        let (c, a) = launch_lug_drag(0.0, ro, ri, 0.0).unwrap();
        close(c, 1.3 * 0.85, 1e-15, "ring coefficient");
        close(a, annulus, 1e-15, "ring area");
        // Half a diameter long: 1.15 on the annulus plus half the bore.
        let (c, a) = launch_lug_drag(ro, ro, ri, 0.3).unwrap();
        close(c, 1.15 * stag(0.3), 1e-15, "l = d/2 coefficient");
        close(a, face - 0.5 * PI * ri * ri, 1e-15, "l = d/2 area");
        // A diameter or longer: 1.0 on the whole face.
        for l in [2.0 * ro, 0.05] {
            let (c, a) = launch_lug_drag(l, ro, ri, 0.7).unwrap();
            close(c, stag(0.7), 1e-15, "long lug coefficient");
            close(a, face, 1e-15, "long lug area");
        }
        // Continuous in length.
        let at = |l: f64| {
            let (c, a) = launch_lug_drag(l, ro, ri, 0.3).unwrap();
            c * a
        };
        close(
            at(2.0 * ro - 1e-12),
            at(2.0 * ro),
            1e-9,
            "continuous at l = d",
        );
        close(at(1e-15), at(0.0), 1e-9, "continuous at l = 0");
        assert!(launch_lug_drag(0.03, ro, 0.006, 0.3).is_err());

        // In a model, a row of two 30 mm lugs and a row of two rail buttons.
        let mut tube = component("tube", body_part(1.0, 0.03, 0.03), None);
        tube.children = vec![
            component(
                "lugs",
                Part::LaunchLug(LaunchLug {
                    length_m: 0.03,
                    outer_radius_m: ro,
                    thickness_m: ro - ri,
                    angle_rad: 0.0,
                    count: 2,
                    spacing_m: 0.5,
                    material: material(),
                }),
                Some(Position::Top { aft_offset_m: 0.1 }),
            ),
            component(
                "buttons",
                Part::RailButton(RailButton {
                    outer_diameter_m: 0.0113,
                    inner_diameter_m: 0.0064,
                    height_m: 0.0081,
                    base_height_m: 0.002,
                    flange_height_m: 0.002,
                    angle_rad: 0.0,
                    count: 2,
                    spacing_m: 0.5,
                    material: material(),
                }),
                Some(Position::Top { aft_offset_m: 0.1 }),
            ),
        ];
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.03), None),
                tube,
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_ref = PI * 0.03 * 0.03;
        let parts = m
            .buildup_components(&Flow::axial(0.3), &DragConditions::coasting(RE_PER_M))
            .unwrap();
        let find = |id: &str| parts.iter().find(|p| p.id == id).unwrap().drag;
        close(
            find("lugs").parasitic,
            2.0 * stag(0.3) * face / a_ref,
            1e-14,
            "lugs",
        );
        let profile = 0.0113 * 0.004 + 0.0064 * 0.0041;
        close(
            find("buttons").parasitic,
            2.0 * stag(0.3) * profile / a_ref,
            1e-14,
            "buttons",
        );
        assert_eq!(find("lugs").friction, 0.0);
    }

    /// Loft lesson L15: as a conical shoulder's length goes to zero its joint angle goes to 90°
    /// and its drag to a bare step's `0.8 ΔA`; a boattail's goes to the base drag of the area it
    /// uncovers, which a bare step down gets.
    #[test]
    fn shoulder_drag_continuous_as_transition_length_tends_to_zero() {
        let (small, big) = (0.02, 0.03);
        let rocket = |fore: f64, aft: f64, length: Option<f64>| {
            let mut body = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, fore),
                    None,
                ),
                component("fore-tube", body_part(0.4, fore, fore), None),
            ];
            if let Some(l) = length {
                body.push(component("change", body_part(l, fore, aft), None));
            }
            body.push(component("aft-tube", body_part(0.6, aft, aft), None));
            one_stage(body, ReferenceDiameter::Custom { diameter_m: 0.06 })
        };
        let a_ref = PI * 0.03 * 0.03;
        let delta = PI * (big * big - small * small);
        let conditions = DragConditions::coasting(RE_PER_M);
        let flow = Flow::axial(0.3);
        let pressure_of = |r: &Rocket| {
            model(r)
                .buildup_components(&flow, &conditions)
                .unwrap()
                .iter()
                .filter(|c| c.id == "change" || c.id == "aft-tube")
                .map(|c| c.drag.pressure)
                .sum::<f64>()
        };
        let total = |r: &Rocket| {
            model(r)
                .drag(&flow, &conditions)
                .unwrap()
                .zero_lift_coefficient
        };

        // Shoulder: a cone of fineness l/(2Δr) and joint angle tan φ = Δr/l on ΔA, and a bare
        // step the flat face's curve on ΔA.
        let step = rocket(small, big, None);
        close(
            pressure_of(&step),
            step_by_hand(0.3) * delta / a_ref,
            1e-13,
            "bare step up",
        );
        // By hand at l = 0.1: fineness 5, a cone of `tan ε = 0.1`, eq. 3.87 to eq. B.5–B.6.
        let s5 = 0.1f64.atan().sin();
        let rest = 0.8 * s5 * s5;
        let b = 4.0 / 2.4 * (1.0 - 0.5 * s5) / (s5 - rest);
        close(
            pressure_of(&rocket(small, big, Some(0.1))),
            (rest + (s5 - rest) * 0.3f64.powf(b)) * delta / a_ref,
            1e-12,
            "shoulder by hand",
        );
        let mut previous = f64::INFINITY;
        for l in [0.1, 0.01, 1e-3, 1e-5, 1e-8] {
            let s = rocket(small, big, Some(l));
            let phi = f64::atan((big - small) / l);
            let curve =
                PressureDragCurve::new(NoseShape::Conical {}, l / (2.0 * (big - small)), phi)
                    .unwrap();
            close(
                pressure_of(&s),
                curve.coefficient(0.3).unwrap() * delta / a_ref,
                1e-12,
                "shoulder",
            );
            let gap = (total(&s) - total(&step)).abs();
            assert!(gap < previous, "shoulder gap {gap} at l = {l}");
            previous = gap;
        }
        assert!(previous < 1e-6 * total(&step));

        // Boattail: eq. 3.88 with γ = l/(d₁ − d₂); a bare step down is γ = 0.
        let c_base = 0.12 + 0.13 * 0.09;
        let step = rocket(big, small, None);
        close(
            pressure_of(&step),
            c_base * delta / a_ref,
            1e-14,
            "bare step down",
        );
        for (l, factor) in [
            (0.01, 1.0),
            (0.02, 1.0),
            (0.04, 0.5),
            (0.06, 0.0),
            (0.1, 0.0),
        ] {
            close(
                pressure_of(&rocket(big, small, Some(l))),
                factor * c_base * delta / a_ref,
                1e-14,
                &format!("boattail at l = {l}"),
            );
        }
        let gap = (total(&rocket(big, small, Some(1e-8))) - total(&step)).abs();
        assert!(gap < 1e-6 * total(&step), "boattail gap {gap}");
    }

    /// Loft lesson L16: geometry the drag model can't use is an error naming the component, and
    /// a large drag coefficient is returned as it is, not capped.
    #[test]
    fn malformed_geometry_is_an_error_not_a_clamped_cd() {
        let base = |child: hpr_design::Component| {
            let mut tube = component("tube", body_part(1.0, 0.03, 0.03), None);
            tube.children = vec![child];
            one_stage(
                vec![
                    component("nose", nose(NoseShape::Conical {}, 0.2, 0.03), None),
                    tube,
                ],
                ReferenceDiameter::Maximum {},
            )
        };
        // A rail button whose base and flange are taller than the button.
        let button = component(
            "buttons",
            Part::RailButton(RailButton {
                outer_diameter_m: 0.0113,
                inner_diameter_m: 0.0064,
                height_m: 0.003,
                base_height_m: 0.002,
                flange_height_m: 0.002,
                angle_rad: 0.0,
                count: 1,
                spacing_m: 0.0,
                material: material(),
            }),
            Some(Position::Top { aft_offset_m: 0.1 }),
        );
        // The design model refuses it already, and the drag terms would on their own.
        let rocket = base(button);
        assert!(rocket.layout().is_err());
        let mut layout = base(with_fin("fins", 3, 0.003, FinCrossSection::Square, 0.0))
            .layout()
            .unwrap();
        let Part::RailButton(bad) = &rocket.stages[0].components[1].children[0].part else {
            unreachable!("the rail button built above");
        };
        let mut placed = layout.components[0].clone();
        placed.id = "buttons".to_owned();
        placed.part = Part::RailButton(bad.clone());
        let err = ComponentDragTerms::rail_buttons(&placed, bad, 1.0, 1e-3).unwrap_err();
        assert!(matches!(err, AeroError::Domain { .. }), "{err:?}");
        // A layout without body components has no radius for the form factor.
        layout.components.clear();
        assert!(matches!(
            AeroModel::new(&layout),
            Err(AeroError::Domain { .. })
        ));
        // A negative custom roughness: the design refuses it, and so does the model given such a
        // layout directly.
        let mut fins = with_fin("fins", 3, 0.003, FinCrossSection::Square, 0.0);
        fins.finish = Some(Finish::Custom { roughness_m: -1e-6 });
        assert!(base(fins).layout().is_err());
        let mut layout = base(with_fin("fins", 3, 0.003, FinCrossSection::Square, 0.0))
            .layout()
            .unwrap();
        let index = layout.find("fins").unwrap().0;
        layout.components[index].finish = Finish::Custom { roughness_m: -1e-6 };
        let (id, source) = match AeroModel::new(&layout) {
            Err(AeroError::InComponent { id, source }) => (id, *source),
            other => panic!("expected an error in a component, got {other:?}"),
        };
        assert_eq!(id, "fins");
        assert!(matches!(source, AeroError::Design(_)), "{source:?}");
        // Bad conditions.
        let m = model(&base(with_fin(
            "fins",
            3,
            0.003,
            FinCrossSection::Square,
            0.0,
        )));
        let flow = Flow::axial(0.3);
        for conditions in [
            DragConditions::coasting(f64::NAN),
            DragConditions::coasting(-1.0),
            DragConditions::thrusting(RE_PER_M, f64::INFINITY),
            DragConditions {
                thrusting_motor_area_m2: 1e-3,
                ..DragConditions::coasting(RE_PER_M)
            },
        ] {
            assert!(m.drag(&flow, &conditions).is_err(), "{conditions:?}");
        }
        assert!(matches!(
            m.drag(&Flow::axial(5.0), &DragConditions::coasting(RE_PER_M)),
            Err(AeroError::Mach { .. })
        ));

        // A blunt, stubby body on a tiny reference diameter: C_D0 far above 10, uncapped.
        let brick = one_stage(
            vec![component("block", body_part(0.1, 0.05, 0.05), None)],
            ReferenceDiameter::Custom { diameter_m: 0.01 },
        );
        let d = model(&brick)
            .drag(&flow, &DragConditions::coasting(RE_PER_M))
            .unwrap();
        let area_ratio = 100.0;
        close(
            d.pressure,
            step_by_hand(0.3) * area_ratio,
            1e-13,
            "flat face",
        );
        close(
            d.base,
            (0.12 + 0.13 * 0.09) * area_ratio,
            1e-14,
            "flat base",
        );
        assert!(d.zero_lift_coefficient > 90.0);
    }

    /// Loft lesson L17: Loft froze the fin leading-edge drag at its Mach 1 value and gave nose and
    /// shoulder pressure drag no Mach term. Here the rounded leading edge follows eq. 3.89's
    /// supersonic branch `1.214 − 0.502/M² + 0.1095/M⁴` and the square one the stagnation
    /// pressure, both times `cos² Γ_L`; a cone nose rises from `0.8 sin² ε` at rest to `sin ε` at
    /// Mach 1 and follows eq. B.4 from Mach 1.3, in the whole rocket's buildup.
    #[test]
    fn leading_edge_and_cone_pressure_drag_have_supersonic_branches() {
        let sweep: f64 = 0.4;
        let c2 = sweep.cos().powi(2);
        let at_1 = fin_pressure_drag_coefficient(FinCrossSection::Airfoil, sweep, 1.0).unwrap();
        close(at_1, 0.8215 * c2, 1e-12, "rounded edge at Mach 1");
        for m in [1.5f64, 2.0, 3.0, 4.5] {
            let rounded = 1.214 - 0.502 / (m * m) + 0.1095 / m.powi(4);
            close(
                fin_pressure_drag_coefficient(FinCrossSection::Airfoil, sweep, m).unwrap(),
                rounded * c2,
                1e-14,
                "rounded, supersonic",
            );
            let i2 = 1.0 / (m * m);
            let stagnation = 0.85 * (1.84 - 0.76 * i2 + 0.166 * i2 * i2 + 0.035 * i2 * i2 * i2);
            close(
                fin_pressure_drag_coefficient(FinCrossSection::Square, sweep, m).unwrap(),
                stagnation * c2 + 0.25 / m,
                1e-14,
                "square, supersonic",
            );
            assert!((rounded * c2 - at_1).abs() > 0.05, "not frozen at Mach {m}");
        }

        // A 3:1 cone on a tube: the nose's pressure drag against Mach, on its base area.
        let (r, l) = (0.025, 0.15);
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, l, r), None),
                component("tube", body_part(0.8, r, r), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let nose_pressure = |mach: f64| {
            m.buildup_components(&Flow::axial(mach), &DragConditions::coasting(RE_PER_M))
                .unwrap()[0]
                .drag
                .pressure
        };
        let s = 1.0 / 37f64.sqrt();
        close(nose_pressure(0.0), 0.8 * s * s, 1e-14, "at rest");
        close(nose_pressure(1.0), s, 1e-14, "eq. B.6 at Mach 1");
        let mut previous = f64::INFINITY;
        for mach in [1.3f64, 1.5, 2.0, 3.0, 4.9] {
            let b4 = 2.1 * s * s + 0.5 * s / (mach * mach - 1.0).sqrt();
            let got = nose_pressure(mach);
            close(got, b4, 1e-14, "eq. B.4");
            assert!(got < previous, "falls past the peak: {got} at Mach {mach}");
            previous = got;
        }
        assert!(nose_pressure(4.9) > 2.1 * s * s);
    }

    /// Code review: a nose the drag buildup has no data for (a bulged secant ogive, a Haack series
    /// past `C = ⅓`) doesn't stop the model building: the normal force and a drag table work, and
    /// only the buildup refuses it, naming the component. A widening too small to move the diameter
    /// is no shoulder.
    #[test]
    fn a_shape_without_drag_data_refuses_only_the_buildup() {
        let (r, l) = (0.03, 0.2);
        for shape in [
            NoseShape::Ogive { radius_ratio: 0.5 },
            NoseShape::Haack { parameter: 0.5 },
        ] {
            let rocket = one_stage(
                vec![
                    component("nose", nose(shape, l, r), None),
                    component("tube", body_part(0.8, r, r), None),
                ],
                ReferenceDiameter::Maximum {},
            );
            let m = model(&rocket);
            assert!(m.normal_force(&Flow::axial(0.3)).is_ok(), "{shape:?}");
            let conditions = DragConditions::coasting(RE_PER_M);
            let error = m.drag(&Flow::axial(0.3), &conditions).unwrap_err();
            assert!(
                matches!(&error, AeroError::InComponent { id, source }
                    if id == "nose" && matches!(**source, AeroError::Unsupported(_))),
                "{shape:?}: {error}"
            );
            assert!(
                m.buildup_components(&Flow::axial(0.3), &conditions)
                    .is_err()
            );
            let table = DragTable::from_csv("0,0.5\n2,0.5\n", None).unwrap();
            let with_table = m.with_drag_table(table);
            assert_eq!(
                with_table
                    .drag(&Flow::axial(0.3), &conditions)
                    .unwrap()
                    .zero_lift_coefficient,
                0.5
            );
        }
        // One ulp of widening: the areas differ, the diameters as computed may not.
        let wider = r * (1.0 + f64::EPSILON);
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, l, r), None),
                component("tube", body_part(0.4, r, r), None),
                component("flare", body_part(0.05, r, wider), None),
                component("aft", body_part(0.4, wider, wider), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let d = model(&rocket)
            .drag(&Flow::axial(0.3), &DragConditions::coasting(RE_PER_M))
            .unwrap();
        assert!(d.zero_lift_coefficient.is_finite());
    }

    /// The stagnation-pressure ratio: 1 at rest, the isentropic `(p₀ − p)/q` with
    /// `p₀/p = (1 + 0.2 M²)^3.5` and `q = 0.7 p M²` to `O(M⁶)` at low Mach, the published 1.275
    /// and 1.281 either side of Mach 1, and 1.84 far above it.
    #[test]
    fn stagnation_pressure_limits() {
        assert_eq!(stagnation_pressure_ratio(0.0).unwrap(), 1.0);
        for m in [0.05, 0.1, 0.2] {
            let isentropic = ((1.0f64 + 0.2 * m * m).powf(3.5) - 1.0) / (0.7 * m * m);
            let got = stagnation_pressure_ratio(m).unwrap();
            assert!(
                (got - isentropic).abs() < m.powi(6),
                "M {m}: {got} vs {isentropic}"
            );
        }
        close(
            stagnation_pressure_ratio(1.0 - 1e-12).unwrap(),
            1.275,
            1e-11,
            "below Mach 1",
        );
        close(
            stagnation_pressure_ratio(1.0).unwrap(),
            1.281,
            1e-14,
            "at Mach 1",
        );
        close(
            stagnation_pressure_ratio(1e6).unwrap(),
            1.84,
            1e-11,
            "hypersonic limit",
        );
        assert_eq!(stagnation_drag_coefficient(0.0).unwrap(), 0.85);
        for bad in [-0.1, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                stagnation_pressure_ratio(bad),
                Err(AeroError::Domain { .. })
            ));
        }
    }

    /// Base drag 0.12 at rest, 0.25 at Mach 1 from both sides, falling as `1/M` above; the joint
    /// term from 0 (smooth) to 0.8 (a step); the boattail factor's three pieces.
    #[test]
    fn base_joint_and_boattail_limits() {
        assert_eq!(base_drag_coefficient(0.0).unwrap(), 0.12);
        assert_eq!(base_drag_coefficient(2.0).unwrap(), 0.125);
        assert!(base_drag_coefficient(1e9).unwrap() < 1e-9);

        assert_eq!(joint_pressure_drag_coefficient(0.0).unwrap(), 0.0);
        assert_eq!(joint_pressure_drag_coefficient(FRAC_PI_2).unwrap(), 0.8);
        close(
            joint_pressure_drag_coefficient(PI / 6.0).unwrap(),
            0.2,
            1e-15,
            "0.8 sin² 30°",
        );
        for bad in [-1e-9, FRAC_PI_2 + 1e-9, f64::NAN] {
            assert!(joint_pressure_drag_coefficient(bad).is_err());
        }

        // γ = l/(d₁ − d₂) for a 20 mm drop in diameter.
        let factor = |l: f64| boattail_factor(l, 0.06, 0.04).unwrap();
        assert_eq!(factor(0.0), 1.0);
        close(factor(0.02), 1.0, 1e-15, "γ = 1");
        close(factor(0.03), 0.75, 1e-15, "γ = 1.5");
        close(factor(0.04), 0.5, 1e-15, "γ = 2");
        assert_eq!(factor(0.06), 0.0);
        assert_eq!(factor(1.0), 0.0);
        close(
            factor(0.02 * (1.0 + 1e-12)),
            1.0,
            1e-11,
            "continuous at γ = 1",
        );
        assert!(factor(0.06 * (1.0 - 1e-12)) < 1e-11, "continuous at γ = 3");
        assert!(boattail_factor(0.01, 0.04, 0.04).is_err());
        assert!(boattail_factor(0.01, 0.04, 0.06).is_err());
        assert!(boattail_factor(-0.01, 0.06, 0.04).is_err());
    }

    /// Fin pressure drag by cross-section at rest; the rounded leading edge's published joins
    /// at Mach 0.9 (0.99876 against 1) and Mach 1 (0.8215 from both sides); `cos² Γ_L` on the
    /// leading edge only; refusals at a 90° sweep.
    #[test]
    fn fin_pressure_drag_limits() {
        let at =
            |section, sweep, mach| fin_pressure_drag_coefficient(section, sweep, mach).unwrap();
        assert_eq!(at(FinCrossSection::Square, 0.0, 0.0), 0.85 + 0.12);
        assert_eq!(at(FinCrossSection::Rounded, 0.0, 0.0), 0.06);
        assert_eq!(at(FinCrossSection::Airfoil, 0.0, 0.0), 0.0);
        let rounded_le = |m: f64| at(FinCrossSection::Airfoil, 0.0, m);
        close(rounded_le(0.9 - 1e-12), 0.998_76, 1e-5, "below Mach 0.9");
        assert_eq!(rounded_le(0.9), 1.0);
        close(rounded_le(1.0 - 1e-12), 0.8215, 1e-11, "below Mach 1");
        close(rounded_le(1.0), 0.8215, 1e-15, "at Mach 1");
        close(
            rounded_le(0.5),
            (0.75f64).powf(-0.417) - 1.0,
            1e-15,
            "eq. 3.89 at Mach 0.5",
        );
        close(rounded_le(1e6), 1.214, 1e-11, "supersonic limit");
        // Sweep scales the leading edge only.
        let sweep: f64 = 0.6;
        let c2 = sweep.cos() * sweep.cos();
        close(
            at(FinCrossSection::Square, sweep, 0.5),
            stagnation_drag_coefficient(0.5).unwrap() * c2 + base_drag_coefficient(0.5).unwrap(),
            1e-15,
            "square, swept",
        );
        close(
            at(FinCrossSection::Rounded, -sweep, 0.5),
            rounded_le(0.5) * c2 + 0.5 * base_drag_coefficient(0.5).unwrap(),
            1e-15,
            "rounded, swept forward",
        );
        for bad in [FRAC_PI_2, -FRAC_PI_2, f64::NAN] {
            assert!(fin_pressure_drag_coefficient(FinCrossSection::Square, bad, 0.3).is_err());
        }
    }

    /// The angle-of-attack factor meets every condition Niskanen states (1 at 0°, 1.3 at 17°, 0 at
    /// 90°, zero slope at each), rises then falls, and mirrors with its sign reversed past 90°, so a
    /// rocket flying tail first is pushed toward its nose.
    #[test]
    fn axial_drag_alpha_factor_limits() {
        let f = |deg: f64| axial_drag_alpha_factor(deg.to_radians()).unwrap();
        assert_eq!(f(0.0), 1.0);
        close(f(17.0), 1.3, 1e-15, "peak");
        assert!(f(90.0).abs() < 1e-15);
        let slope = |deg: f64| {
            let h = 1e-6;
            (f((deg + h).min(180.0)) - f((deg - h).max(0.0))) / (2.0 * h)
        };
        for deg in [0.0, 17.0, 90.0] {
            assert!(slope(deg).abs() < 1e-5, "slope {} at {deg}°", slope(deg));
        }
        let mut previous = f(0.0);
        for k in 1..=170 {
            let deg = 0.1 * f64::from(k);
            assert!(f(deg) > previous, "rising at {deg}°");
            previous = f(deg);
        }
        for k in 171..=900 {
            let deg = 0.1 * f64::from(k);
            assert!(f(deg) < previous, "falling at {deg}°");
            previous = f(deg);
        }
        for deg in [5.0, 17.0, 45.0, 89.0] {
            close(f(180.0 - deg), -f(deg), 1e-12, "mirror");
        }
        assert_eq!(f(180.0), -1.0);
        // In a model at 135°: C_A points toward the nose.
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.03), None),
                component("tube", body_part(0.8, 0.03, 0.03), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let d = model(&rocket)
            .drag(
                &Flow::new(0.3, 135f64.to_radians(), 0.0),
                &DragConditions::coasting(RE_PER_M),
            )
            .unwrap();
        assert!(d.zero_lift_coefficient > 0.0 && d.axial_coefficient < 0.0);
        close(
            d.axial_coefficient,
            -d.zero_lift_coefficient * f(45.0),
            1e-14,
            "C_A at 135°",
        );
        assert!(axial_drag_alpha_factor(-1e-9).is_err());
        assert!(axial_drag_alpha_factor(PI + 1e-9).is_err());
    }

    /// Leading-edge sweeps: a trapezoid's `atan(x_t/s)`; the same trapezoid as a freeform
    /// outline; a kinked edge's span average; an elliptical fin's closed form against a midpoint
    /// sum, and 0 as the chord vanishes against the span.
    #[test]
    fn leading_edge_sweep_of_every_planform() {
        use crate::fins::FinGeometry;
        let trap = FinGeometry::from_planform(&trapezoid()).unwrap();
        close(
            trap.leading_edge_sweep_rad,
            (0.07f64 / 0.06).atan(),
            1e-15,
            "trapezoid",
        );
        let outline = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.07, 0.06], [0.12, 0.06], [0.12, 0.0]],
        };
        let free = FinGeometry::from_planform(&outline).unwrap();
        close(
            free.leading_edge_sweep_rad,
            trap.leading_edge_sweep_rad,
            1e-12,
            "as freeform",
        );
        // A kinked edge: straight out for half the span, then swept 45°.
        let kinked = FinPlanform::Freeform {
            points_m: vec![
                [0.0, 0.0],
                [0.0, 0.03],
                [0.03, 0.06],
                [0.1, 0.06],
                [0.1, 0.0],
            ],
        };
        let kinked = FinGeometry::from_planform(&kinked).unwrap();
        close(
            kinked.leading_edge_sweep_rad,
            0.5 * PI / 4.0,
            1e-12,
            "kinked",
        );
        for (c_r, s) in [(0.1, 0.05), (0.1, 0.1), (0.2, 0.05), (0.01, 1.0)] {
            let e = FinGeometry::from_planform(&FinPlanform::Elliptical {
                root_chord_m: c_r,
                span_m: s,
            })
            .unwrap();
            let k: f64 = 0.5 * c_r / s;
            // η = sin t makes the integrand smooth: ∫₀^{π/2} atan(k tan t) cos t dt.
            let n = 200_000;
            let h = FRAC_PI_2 / f64::from(n);
            let sum: f64 = (0..n)
                .map(|i| {
                    let t = (f64::from(i) + 0.5) * h;
                    (k * t.tan()).atan() * t.cos() * h
                })
                .sum();
            close(e.leading_edge_sweep_rad, sum, 1e-8, "elliptical average");
        }
    }

    /// The whole buildup of a simple rocket written out term by term: a 0.25 m cone on a 0.05 m
    /// diameter, a 1 m tube with three 4 mm square fins and one lug, at Mach 0.5, 5° and a
    /// Reynolds number of 5e6 per metre. Components add up to the total, and the table override
    /// replaces `C_D0` but keeps the angle-of-attack factor.
    #[test]
    fn buildup_by_hand() {
        let (r, l_nose, l_tube) = (0.025, 0.25, 1.0);
        let mut tube = component("tube", body_part(l_tube, r, r), None);
        let mut fins = with_fin("fins", 3, 0.004, FinCrossSection::Square, 0.0);
        fins.finish = Some(Finish::PlanedWood {});
        tube.children = vec![
            fins,
            component(
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
                Some(Position::Top { aft_offset_m: 0.2 }),
            ),
        ];
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, l_nose, r), None),
                tube,
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let (mach, alpha, re_per_m) = (0.5, 5f64.to_radians(), 5e6);
        let flow = Flow::new(mach, alpha, 0.0);
        let conditions = DragConditions::coasting(re_per_m);
        let got = m.drag(&flow, &conditions).unwrap();

        let length = l_nose + l_tube;
        let a_ref = PI * r * r;
        let re = re_per_m * length;
        let cf = |roughness: f64| {
            let rr: f64 = roughness / length;
            let critical = 51.0 * rr.powf(-1.039);
            let incompressible = if re < critical {
                1.0 / (1.50 * re.ln() - 5.6).powi(2)
            } else {
                0.032 * rr.powf(0.2)
            };
            incompressible * (1.0 - 0.1 * mach * mach)
        };
        let form = 1.0 + 1.0 / (2.0 * length / (2.0 * r));
        let body_friction = cf(20e-6) * form * (PI * r * l_nose + 2.0 * PI * r * l_tube) / a_ref;
        let (c_r, c_t, span) = (0.12, 0.05, 0.06);
        let fin_area = 0.5 * span * (c_r + c_t);
        let mac = 2.0 / 3.0 * (c_r * c_r + c_r * c_t + c_t * c_t) / (c_r + c_t);
        let fin_friction = cf(15e-6) * (1.0 + 2.0 * 0.004 / mac) * 2.0 * 3.0 * fin_area / a_ref;
        let stag = 0.85 * (1.0 + mach * mach / 4.0 + mach.powi(4) / 40.0);
        let c_base = 0.12 + 0.13 * mach * mach;
        let gamma_l = (0.07f64 / span).atan();
        let fin_pressure = (stag * gamma_l.cos().powi(2) + c_base) * 3.0 * 0.004 * span / a_ref;
        // Eq. 3.87 from 0.8 sin² ε at rest to eq. B.5–B.6 at Mach 1, by hand: `tan ε = r/l`.
        let s = (r / l_nose).atan().sin();
        let rest = 0.8 * s * s;
        let slope_at_1 = 4.0 / 2.4 * (1.0 - 0.5 * s);
        let b = slope_at_1 / (s - rest);
        let nose_pressure = (s - rest) * mach.powf(b) + rest;
        let base = c_base;
        let lug = stag * PI * 0.004 * 0.004 / a_ref;
        let cd0 = body_friction + fin_friction + fin_pressure + nose_pressure + base + lug;

        close(
            got.friction,
            body_friction + fin_friction,
            1e-12,
            "friction",
        );
        close(
            got.pressure,
            fin_pressure + nose_pressure,
            1e-12,
            "pressure",
        );
        close(got.base, base, 1e-14, "base");
        close(got.parasitic, lug, 1e-14, "parasitic");
        close(got.zero_lift_coefficient, cd0, 1e-12, "C_D0");
        let t = 5.0 / 17.0;
        close(
            got.axial_coefficient,
            cd0 * (1.0 + 0.3 * t * t * (3.0 - 2.0 * t)),
            1e-12,
            "C_A",
        );
        assert_eq!(got.table, None);

        let parts = m.buildup_components(&flow, &conditions).unwrap();
        let ids: Vec<&str> = parts.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, ["nose", "tube", "fins", "lug"]);
        let sum = |f: fn(&Drag) -> f64| parts.iter().map(|p| f(&p.drag)).sum::<f64>();
        close(
            sum(|d| d.zero_lift_coefficient),
            got.zero_lift_coefficient,
            1e-14,
            "C_D0 sum",
        );
        close(
            sum(|d| d.axial_coefficient),
            got.axial_coefficient,
            1e-14,
            "C_A sum",
        );

        // Thrusting with an unknown motor area: the base keeps its drag.
        let unknown = m
            .drag(&flow, &DragConditions::thrusting(re_per_m, 0.0))
            .unwrap();
        assert_eq!(unknown.base, got.base);

        // An override: the table's C_D0 (extrapolation reported), the same factor, and Mach 5
        // allowed for drag while the buildup refuses it.
        let table = DragTable::from_csv("0.1,0.6\n1.2,0.9\n", None).unwrap();
        let o = m.clone().with_drag_table(table);
        let d = o.drag(&flow, &conditions).unwrap();
        close(
            d.zero_lift_coefficient,
            0.6 + 0.3 * 0.4 / 1.1,
            1e-14,
            "table C_D0",
        );
        close(
            d.axial_coefficient / d.zero_lift_coefficient,
            got.axial_coefficient / cd0,
            1e-14,
            "factor",
        );
        assert_eq!(
            (d.friction, d.pressure, d.base, d.parasitic),
            (0.0, 0.0, 0.0, 0.0)
        );
        let fast = o.drag(&Flow::axial(5.0), &conditions).unwrap();
        assert_eq!(fast.zero_lift_coefficient, 0.9);
        assert!(fast.table.unwrap().extrapolated.is_some());
        assert!(m.drag(&Flow::axial(5.0), &conditions).is_err());
        assert!(m.drag(&Flow::axial(4.99), &conditions).is_ok());
        assert!(o.drag(&Flow::new(0.5, 4.0, 0.0), &conditions).is_err());
        assert!(
            o.buildup_components(&Flow::axial(5.0), &conditions)
                .is_err()
        );
    }

    /// A boattail drags the same however its surface is split into transitions (physics review):
    /// one conical boattail against the same cone as two, total and base, at every speed.
    #[test]
    fn a_boattail_split_in_two_drags_as_one() {
        let (big, small, l) = (0.03, 0.02, 0.04);
        let mid = 0.5 * (big + small);
        let rocket = |split: bool| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
            ];
            if split {
                components.push(component("tail-a", body_part(0.5 * l, big, mid), None));
                components.push(component("tail-b", body_part(0.5 * l, mid, small), None));
            } else {
                components.push(component("tail", body_part(l, big, small), None));
            }
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let (one, two) = (rocket(false), rocket(true));
        let conditions = DragConditions::coasting(RE_PER_M);
        for mach in [0.3, 0.85, 0.95, 1.1, 1.5, 3.0, 4.9] {
            let (a, b) = (
                one.drag(&Flow::axial(mach), &conditions).unwrap(),
                two.drag(&Flow::axial(mach), &conditions).unwrap(),
            );
            close(
                b.zero_lift_coefficient,
                a.zero_lift_coefficient,
                1e-12,
                "total",
            );
            close(b.pressure, a.pressure, 1e-12, "pressure");
            close(b.base, a.base, 1e-12, "base");
        }
        // The first part is its own cone; the second, wholly merged, drags as the whole cone less
        // the first part's.
        let terms = two.drag_terms();
        let (a, b) = (
            terms[2].boattail.clone().unwrap(),
            terms[3].boattail.clone().unwrap(),
        );
        let whole = one.drag_terms()[2].boattail.clone().unwrap().own;
        assert!(a.merged.is_empty());
        assert_eq!(b.merged.len(), 1);
        let m = b.merged[0];
        assert_eq!(m.weight, 1.0);
        close(m.through_fore.length_m, a.own.length_m, 1e-12, "first part");
        close(
            m.through_fore.aft_diameter_m,
            a.own.aft_diameter_m,
            1e-12,
            "first part's end",
        );
        close(
            m.through_aft.length_m,
            whole.length_m,
            1e-12,
            "whole length",
        );
        close(
            m.through_aft.aft_diameter_m,
            whole.aft_diameter_m,
            1e-12,
            "whole end",
        );
        close(m.area_ratio, a.own_area_ratio, 1e-12, "same fore area");
    }

    /// A sharp corner keeps two narrowing parts apart (physics review): a 15° boattail closed by a
    /// micrometre-long transition to a smaller tube drags as the same boattail and a step down,
    /// and as with a micrometre of tube between; and the merge weight is continuous in the turn,
    /// whole to 3° and none from 10°.
    #[test]
    fn a_sharp_corner_keeps_its_boattails_apart() {
        let (big, small, tip) = (0.03, 0.02, 0.008);
        let l = (big - small) / 15f64.to_radians().tan();
        let rocket = |closure: Option<f64>, gap: Option<f64>| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
                component("tail", body_part(l, big, small), None),
            ];
            if let Some(gap) = gap {
                components.push(component("gap", body_part(gap, small, small), None));
            }
            if let Some(length) = closure {
                components.push(component("closure", body_part(length, small, tip), None));
            }
            components.push(component("end", body_part(0.01, tip, tip), None));
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let conditions = DragConditions::coasting(RE_PER_M);
        let total = |m: &AeroModel, mach: f64| {
            m.drag(&Flow::axial(mach), &conditions)
                .unwrap()
                .zero_lift_coefficient
        };
        let (step, corner, apart) = (
            rocket(None, None),
            rocket(Some(1e-6), None),
            rocket(Some(1e-6), Some(1e-6)),
        );
        for mach in [0.3, 0.6, 0.9, 1.0, 1.5, 3.0] {
            let s = total(&step, mach);
            // The closure's own drag stands in for the step's; the straight line from Mach 0.8 to
            // 1 for the base drag's curve moves it by up to 0.6% of the annulus's base drag.
            assert!((total(&corner, mach) - s).abs() < 2e-3 * s, "Mach {mach}");
            assert!(
                (total(&apart, mach) - total(&corner, mach)).abs() < 1e-5 * s,
                "Mach {mach}"
            );
        }
        // Two cones meeting at a turn: whole below 3°, none from 10°, continuous between.
        let pair = |turn_deg: f64| {
            let (first, second) = (6f64.to_radians(), (6.0 + turn_deg).to_radians());
            let mid = big - 0.01 * first.tan();
            let end = mid - 0.01 * second.tan();
            let m = model(&one_stage(
                vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                        None,
                    ),
                    component("tube", body_part(0.8, big, big), None),
                    component("a", body_part(0.01, big, mid), None),
                    component("b", body_part(0.01, mid, end), None),
                ],
                ReferenceDiameter::Maximum {},
            ));
            let merges = !m.drag_terms()[3]
                .boattail
                .clone()
                .unwrap()
                .merged
                .is_empty();
            (m, merges)
        };
        assert!(pair(2.0).1 && pair(9.0).1 && !pair(10.5).1);
        for edge in [3.0, 10.0] {
            for mach in [0.5, 1.5, 2.5] {
                let (a, b) = (
                    total(&pair(edge - 1e-7).0, mach),
                    total(&pair(edge + 1e-7).0, mach),
                );
                assert!(
                    (a - b).abs() < 1e-7 * a,
                    "turn {edge}° at Mach {mach}: {a} against {b}"
                );
            }
        }
    }

    /// A lip drawn as a step up and a tube drags as the same lip drawn as a shoulder a micrometre
    /// long and the tube (physics review): the step's drag is in the wake too, and the base keeps
    /// the same share of its relief.
    #[test]
    fn a_lip_drawn_as_a_step_up_is_a_lip() {
        let (big, small, lip, l) = (0.03, 0.02, 0.0215, 0.04);
        let rocket = |shoulder: bool| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
                component("tail", body_part(l, big, small), None),
            ];
            if shoulder {
                components.push(component("rise", body_part(1e-6, small, lip), None));
            }
            components.push(component("lip", body_part(0.00135, lip, lip), None));
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let (step, shoulder) = (rocket(false), rocket(true));
        let conditions = DragConditions::coasting(RE_PER_M);
        for mach in [0.6, 0.95, 1.5, 3.0] {
            let total = |m: &AeroModel| {
                m.drag(&Flow::axial(mach), &conditions)
                    .unwrap()
                    .zero_lift_coefficient
            };
            let (a, b) = (total(&step), total(&shoulder));
            assert!((a - b).abs() < 1e-3 * a, "Mach {mach}: {a} against {b}");
        }
        let terms = step.drag_terms();
        let last = terms.iter().find(|t| t.id == "lip").unwrap();
        assert!(last.step.is_some() && last.in_wake_of.unwrap().step_fraction == 1.0);
    }

    /// A pair of narrowing parts never drags less than nothing, and drags between its two limits
    /// (physics review): as two boattails, built with a
    /// tube between them, and as the first part plus its share of the one cone through the pair's
    /// ends, built as a rocket of its own. A 15° part is followed by parts turned from −12° to
    /// +12°, at every Mach number to 4.9; at a turn of 0° the pair is that cone.
    #[test]
    fn soft_merges_stay_between_their_limits() {
        let big = 0.03;
        let conditions = DragConditions::coasting(RE_PER_M);
        let rocket = |parts: &[(&str, f64, f64, f64)]| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
            ];
            for &(id, length, fore, aft) in parts {
                components.push(component(id, body_part(length, fore, aft), None));
            }
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        for turn in (-24..=24).map(|t| f64::from(t) * 0.5) {
            let (first, second) = (15f64.to_radians(), (15.0 + turn).to_radians());
            let mid = big - 0.01 * first.tan();
            let end = mid - 0.01 * second.tan();
            let joined = rocket(&[("a", 0.01, big, mid), ("b", 0.01, mid, end)]);
            // Two boattails: a tube between them longer than the first's drop.
            let apart = rocket(&[
                ("a", 0.01, big, mid),
                ("gap", 0.05, mid, mid),
                ("b", 0.01, mid, end),
            ]);
            // The one cone through the pair's ends.
            let whole = rocket(&[("ab", 0.02, big, end)]);
            let weight: f64 = joined.drag_terms()[3]
                .boattail
                .as_ref()
                .unwrap()
                .merged
                .iter()
                .map(|m| m.weight)
                .sum();
            for step in 0..98 {
                let mach = f64::from(step) * 0.05;
                let flow = Flow::axial(mach);
                let parts = joined.buildup_components(&flow, &conditions).unwrap();
                let what = format!("turn {turn}° at Mach {mach:.2}");
                let got = parts[2].drag.pressure + parts[3].drag.pressure;
                assert!(got >= 0.0, "{what}");
                let separate = apart.buildup_components(&flow, &conditions).unwrap();
                let (own_a, own_b) = (separate[2].drag.pressure, separate[4].drag.pressure);
                let cone = whole.buildup_components(&flow, &conditions).unwrap()[2]
                    .drag
                    .pressure;
                let merged = cone;
                if turn == 0.0 {
                    assert_eq!(weight, 1.0, "{what}");
                    assert!((got - cone).abs() <= 1e-12 * cone.max(1e-3), "{what}");
                } else if weight == 0.0 {
                    assert!((got - own_a - own_b).abs() <= 1e-12, "{what}");
                }
                let (lo, hi) = ((own_a + own_b).min(merged), (own_a + own_b).max(merged));
                assert!(
                    got >= lo - 1e-12 && got <= hi + 1e-12,
                    "{what}: {got} not in [{lo}, {hi}]"
                );
            }
        }
    }

    /// Every weight is continuous in the geometry (physics and code reviews, three rounds): a
    /// part narrowing or widening by `ε` drags as a tube, a part `ε` long as a step down (with or
    /// without a lip behind it), a step up and a shoulder `ε` apart as the two in one part, and a
    /// tube tapered by `ε` before the boattail as a tube, with the difference shrinking in
    /// proportion to `ε`, behind boattails of 2°, 5°, 9° and 14° and at every speed.
    #[test]
    fn a_part_narrowing_by_nothing_is_a_tube_and_one_of_no_length_a_step() {
        let (big, l) = (0.03, 0.04);
        let conditions = DragConditions::coasting(RE_PER_M);
        let total = |m: &AeroModel, mach: f64| {
            m.drag(&Flow::axial(mach), &conditions)
                .unwrap()
                .zero_lift_coefficient
        };
        type Case = fn(f64, f64, f64, f64) -> (f64, Vec<(f64, f64, f64)>);
        // Each case, from `ε`, the boattail's aft radius, a lip's top and the boattail's drop in
        // diameter: a taper of the tube ahead of the boattail, and the parts behind it.
        let cases: [(&str, Case); 8] = [
            ("a spacer narrowing by ε before a lip", |e, s, lip, _| {
                (0.0, vec![(0.001, s, s - e), (0.0013, s - e, lip)])
            }),
            ("an aft section narrowing by ε", |e, s, _, _| {
                (0.0, vec![(0.1, s, s - e)])
            }),
            ("an aft section widening by ε", |e, s, _, _| {
                (0.0, vec![(0.1, s, s + e)])
            }),
            ("a step down drawn ε long", |e, s, _, d| {
                let low = s - 0.05 * d;
                let mut parts = vec![(0.005, low, low)];
                if e > 0.0 {
                    parts.insert(0, (e, s, low));
                }
                (0.0, parts)
            }),
            ("a step down drawn ε long, then a lip", |e, s, _, d| {
                let low = s - 0.25 * d;
                let mut parts = vec![(0.003, low, low + 0.05 * d)];
                if e > 0.0 {
                    parts.insert(0, (e, s, low));
                }
                (0.0, parts)
            }),
            ("a step up and a shoulder ε apart", |e, s, _, d| {
                let (top, high) = (s + 0.1 * d, s + 0.3 * d);
                let mut parts = vec![(0.002, top, high)];
                if e > 0.0 {
                    parts.insert(0, (e, top, top));
                }
                (0.0, parts)
            }),
            (
                "a gap of ε before a boattail's second part",
                |e, s, _, _| {
                    let end = s - 0.005 * 8f64.to_radians().tan();
                    let mut parts = vec![(0.005, s, end)];
                    if e > 0.0 {
                        parts.insert(0, (e, s, s));
                    }
                    (0.0, parts)
                },
            ),
            ("the tube ahead tapered by ε", |e, _, _, _| (e, vec![])),
        ];
        for angle in [2.0_f64, 5.0, 9.0, 14.0] {
            let small = big - l * angle.to_radians().tan();
            let drop = 2.0 * (big - small);
            let lip = small + 0.5 * 0.17 * drop;
            let rocket = |(taper, parts): (f64, Vec<(f64, f64, f64)>)| {
                let mut components = vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                        None,
                    ),
                    component("tube", body_part(0.8, big, big - taper), None),
                    component("tail", body_part(l, big - taper, small), None),
                ];
                for (i, &(length, fore, aft)) in parts.iter().enumerate() {
                    components.push(component(
                        &format!("p{i}"),
                        body_part(length, fore, aft),
                        None,
                    ));
                }
                model(&one_stage(components, ReferenceDiameter::Maximum {}))
            };
            for (what, case) in cases {
                let exact = rocket(case(0.0, small, lip, drop));
                for mach in [0.5, 0.95, 1.5, 3.0, 4.5] {
                    // A part of no length drags its own boattail drag where a step drags the base
                    // drag's: the two part between Mach 0.8 and 1.2, where the boattail's rise is
                    // a straight line (`a_sharp_corner_keeps_its_boattails_apart`); the weights
                    // don't.
                    if what.starts_with("a step down") && mach == 0.95 {
                        continue;
                    }
                    let base = total(&exact, mach);
                    let off =
                        |e: f64| (total(&rocket(case(e, small, lip, drop)), mach) - base).abs();
                    let (coarse, fine) = (off(1e-6), off(1e-8));
                    let at = format!("{what} behind {angle}° at Mach {mach}");
                    assert!(coarse < 1e-3 * base, "{at}: {coarse}");
                    assert!(
                        fine <= 0.02 * coarse + 1e-13,
                        "{at}: {fine} against {coarse}"
                    );
                }
            }
        }
        // An 8° cone behind the body tube drawn in one part or two, then a tube 0.3 of its drop
        // long and a 12° part that merges with it (physics review).
        let (aft, tip) = (big - 0.02 * 8f64.to_radians().tan(), 0.02);
        let mid = big - 0.01 * 8f64.to_radians().tan();
        let gap = 0.3 * 2.0 * (big - aft);
        let end = aft - 0.01 * 12f64.to_radians().tan();
        let cone = |parts: &[(f64, f64, f64)]| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
            ];
            for (i, &(length, fore, aft)) in parts.iter().enumerate() {
                components.push(component(
                    &format!("p{i}"),
                    body_part(length, fore, aft),
                    None,
                ));
            }
            components.push(component("end", body_part(0.01, tip, tip), None));
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let one = cone(&[(0.02, big, aft), (gap, aft, aft), (0.01, aft, end)]);
        let two = cone(&[
            (0.01, big, mid),
            (0.01, mid, aft),
            (gap, aft, aft),
            (0.01, aft, end),
        ]);
        assert!(
            !one.drag_terms()[4]
                .boattail
                .as_ref()
                .unwrap()
                .merged
                .is_empty()
        );
        for mach in [0.5, 1.5, 3.0] {
            let (a, b) = (total(&one, mach), total(&two, mach));
            assert!(
                (a - b).abs() < 1e-12 * a,
                "cone in parts at Mach {mach}: {a} against {b}"
            );
        }
    }

    /// A merge shares the flow (physics and code reviews): a lip that both limits, the two parts
    /// as one cone and as two boattails, put wholly in the wake stays wholly in it at every turn
    /// between; and the base's shares of the flow add up to 1 with nothing between.
    #[test]
    fn a_partial_merge_shares_the_flow() {
        let big = 0.03;
        let (first, length) = (5f64.to_radians(), 0.01);
        let mid = big - length * first.tan();
        for turn in (0..=24).map(|t| f64::from(t) * 0.5) {
            let second = first + turn.to_radians();
            let end = mid - length * second.tan();
            let drop = 2.0 * (big - end);
            let rocket = |lip: bool| {
                let mut components = vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                        None,
                    ),
                    component("tube", body_part(0.8, big, big), None),
                    component("a", body_part(length, big, mid), None),
                    component("b", body_part(length, mid, end), None),
                ];
                if lip {
                    components.push(component("lip", body_part(1e-4, end, end), None));
                    let top = end + 0.5 * 0.1 * 2.0 * (mid - end);
                    components.push(component("rise", body_part(1e-4, end, top), None));
                }
                model(&one_stage(components, ReferenceDiameter::Maximum {}))
            };
            let what = format!("turn {turn}°, drop {drop}");
            let bare = rocket(false);
            let sources = &bare.drag_terms()[3].base_behind.as_ref().unwrap().sources;
            let shares: f64 = sources.iter().map(|s| s.weight).sum();
            assert!((shares - 1.0).abs() < 1e-12, "{what}: {shares}");
            let with_lip = rocket(true);
            let wake = with_lip.drag_terms()[5].in_wake_of.unwrap();
            // Every tail takes the lip wholly by its rise; only the 0.1 mm tube ahead of it
            // fades them, by at most its length over the smallest fall, the second part's own.
            let least = 1.0 - 1e-4 / (2.0 * (mid - end));
            assert!(
                wake.shoulder_fraction >= least - 1e-12 && wake.shoulder_fraction <= 1.0,
                "{what}: {}",
                wake.shoulder_fraction
            );
        }
    }

    /// The tails stay few (code review): a boattail drawn as 40 parts turning 7° each way, each a
    /// partial merge with the one before, holds at most two tails per part at once, keeps at most
    /// one merge per earlier part and one base source per pair of parts, and drags.
    #[test]
    fn a_zigzag_boattail_keeps_its_tails_few() {
        let big = 0.03;
        let mut components = vec![
            component(
                "nose",
                nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                None,
            ),
            component("tube", body_part(0.8, big, big), None),
        ];
        let mut r = big;
        let parts = 40;
        for i in 0..parts {
            let angle = if i % 2 == 0 { 12f64 } else { 5.0 };
            let next = r - 0.0005 * angle.to_radians().tan();
            components.push(component(
                &format!("z{i}"),
                body_part(0.0005, r, next),
                None,
            ));
            r = next;
        }
        super::PEAK_TAILS.with(|p| p.set(0));
        let m = model(&one_stage(components, ReferenceDiameter::Maximum {}));
        // Measured: 78 at 40 parts.
        let peak = super::PEAK_TAILS.with(std::cell::Cell::get);
        assert!(peak <= 2 * parts, "{peak} tails");
        let terms = m.drag_terms();
        for t in terms.iter().skip(2) {
            let merges = t.boattail.as_ref().map_or(0, |b| b.merged.len());
            assert!(merges <= parts, "{}: {merges}", t.id);
        }
        let sources = terms
            .last()
            .unwrap()
            .base_behind
            .as_ref()
            .unwrap()
            .sources
            .len();
        // Measured: 76; at most one per pair of parts in principle.
        assert!(sources <= 2 * parts, "{sources}");
        let drag = m
            .drag(&Flow::axial(1.5), &DragConditions::coasting(RE_PER_M))
            .unwrap();
        assert!(drag.zero_lift_coefficient.is_finite());
    }

    /// A straight cone drawn in parts merges wholly and drags as the one cone, where a part's share
    /// is below 0 too (physics review, rounds 3 and 4): 0.8° and 0.5° cones 300 mm long, a 7°
    /// boattail from 98 mm to 44 mm, and a 5° one closing to an eighth of its diameter, whole and
    /// in 2, 4 and 8 parts, from Mach 0.5 to 3. Held at 0, a share had put the 7° one 1.35% high
    /// in 4 parts at Mach 1.0, and the 5° one 5% high in 2 at Mach 1.3.
    #[test]
    fn a_straight_cone_in_parts_is_one_cone() {
        let conditions = DragConditions::coasting(RE_PER_M);
        // Fore radius, half-angle (degrees), aft radius.
        let cones = [
            (0.03, 0.8_f64, 0.03 - 0.3 * 0.8_f64.to_radians().tan()),
            (0.03, 0.5, 0.03 - 0.3 * 0.5_f64.to_radians().tan()),
            (0.049, 7.0, 0.022),
            (0.049, 5.0, 0.049 / 8.0),
        ];
        for (big, angle, end) in cones {
            let length = (big - end) / angle.to_radians().tan();
            let rocket = |parts: usize| {
                let mut components = vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                        None,
                    ),
                    component("tube", body_part(0.8, big, big), None),
                ];
                let at = |k: usize| big - (big - end) * k as f64 / parts as f64;
                for k in 0..parts {
                    components.push(component(
                        &format!("c{k}"),
                        body_part(length / parts as f64, at(k), at(k + 1)),
                        None,
                    ));
                }
                model(&one_stage(components, ReferenceDiameter::Maximum {}))
            };
            let one = rocket(1);
            for parts in [2, 4, 8] {
                let split = rocket(parts);
                // Every later part merges wholly with the cone ahead of it.
                for t in split.drag_terms().iter().skip(3) {
                    let term = t.boattail.as_ref().unwrap();
                    let merged: f64 = term.merged.iter().map(|m| m.weight).sum();
                    assert!(
                        term.own_weight < 1e-12 && (merged - 1.0).abs() < 1e-12,
                        "{}",
                        t.id
                    );
                }
                for mach in [0.5, 0.95, 1.0, 1.2, 1.3, 1.5, 3.0] {
                    let (a, b) = (
                        one.drag(&Flow::axial(mach), &conditions).unwrap(),
                        split.drag(&Flow::axial(mach), &conditions).unwrap(),
                    );
                    let what = format!("{angle}° in {parts} at Mach {mach}");
                    close(
                        b.zero_lift_coefficient,
                        a.zero_lift_coefficient,
                        1e-12,
                        &what,
                    );
                }
            }
        }
    }

    /// A step down is a boattail of no length (physics and code reviews): a motor retainer that
    /// rises behind a plain step down to the motor tube sits in the corner's wake, by its rise
    /// and the motor tube's length over the step's drop in diameter. A 98 mm airframe steps down
    /// to a 54 mm motor tube showing for 12 mm, then a 62 mm retainer: the retainer's step keeps
    /// `12/44` of its drag.
    #[test]
    fn a_retainer_behind_a_step_down_is_in_its_wake() {
        let (body, motor, retainer) = (0.049, 0.027, 0.031);
        let m = model(&one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.3, body),
                    None,
                ),
                component("tube", body_part(1.0, body, body), None),
                component("motor", body_part(0.012, motor, motor), None),
                component("retainer", body_part(0.02, retainer, retainer), None),
            ],
            ReferenceDiameter::Maximum {},
        ));
        let terms = m.drag_terms();
        let wake = terms[3].in_wake_of.unwrap();
        let fall = 2.0 * (body - motor);
        close(
            wake.step_fraction,
            1.0 - 0.012 / fall,
            1e-12,
            "step fraction",
        );
        assert_eq!(wake.shoulder_fraction, 0.0);
        assert!(wake.boattail.half_angle_rad > 1.57);
    }

    /// A lip counts however it is drawn in parts (physics review): a tube a hair above the
    /// boattail's aft radius ahead of the lip changes the drag by a hair, at every speed, as
    /// the step up to it goes to zero.
    #[test]
    fn a_hairline_step_before_a_lip_changes_nothing() {
        let (big, small, lip, l) = (0.03, 0.02, 0.0215, 0.04);
        let rocket = |step: Option<f64>| {
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
                component("tail", body_part(l, big, small), None),
            ];
            if let Some(step) = step {
                components.push(component(
                    "hair",
                    body_part(0.001, small + step, small + step),
                    None,
                ));
            } else {
                components.push(component("hair", body_part(0.001, small, small), None));
            }
            components.push(component("lip", body_part(0.00135, small, lip), None));
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let conditions = DragConditions::coasting(RE_PER_M);
        let exact = rocket(None);
        for mach in [0.6, 1.5, 3.0] {
            let total = |m: &AeroModel| {
                m.drag(&Flow::axial(mach), &conditions)
                    .unwrap()
                    .zero_lift_coefficient
            };
            let base = total(&exact);
            for step in [1e-5, 1e-7, 2e-9] {
                let got = total(&rocket(Some(step)));
                assert!(
                    (got - base).abs() < 1e-3 * base,
                    "step {step} at Mach {mach}: {got} against {base}"
                );
            }
        }
    }

    /// A lip right behind a boattail is in its wake (physics review): none of a shoulder's
    /// pressure drag while its aft end rises up to a quarter of the boattail's drop in diameter
    /// above the boattail's, all of it from half, a straight line between, and the same fraction
    /// of the base's relief; a tube between fades both over one drop in diameter. Every weight is
    /// continuous: a micrometre of step or tube changes the drag by a micrometre's worth.
    #[test]
    fn a_lip_in_a_boattails_wake_fades_with_its_rise() {
        let (big, small, l) = (0.03, 0.02, 0.04);
        let drop = 2.0 * (big - small);
        // `rise` is the lip's aft rise above the boattail's aft radius, in drops in diameter.
        let rocket = |rise: f64, gap: Option<f64>, step: f64| {
            let lip_fore = small + step;
            let mut components = vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                    None,
                ),
                component("tube", body_part(0.8, big, big), None),
                component("tail", body_part(l, big, small), None),
            ];
            if let Some(gap) = gap {
                components.push(component("gap", body_part(gap, small, small), None));
            }
            components.push(component(
                "lip",
                body_part(0.001, lip_fore, small + 0.5 * rise * drop),
                None,
            ));
            model(&one_stage(components, ReferenceDiameter::Maximum {}))
        };
        let conditions = DragConditions::coasting(RE_PER_M);
        let lip = |m: &AeroModel, mach: f64| {
            let parts = m
                .buildup_components(&Flow::axial(mach), &conditions)
                .unwrap();
            let part = parts.iter().find(|p| p.id == "lip").unwrap();
            (part.drag.pressure, part.drag.base)
        };
        let fraction_of = |m: &AeroModel| {
            let terms = m.drag_terms();
            let last = terms.iter().find(|t| t.id == "lip").unwrap();
            (
                last.in_wake_of.map_or(0.0, |w| w.shoulder_fraction),
                last.base_behind
                    .as_ref()
                    .map_or(0.0, |b| b.sources.iter().map(|s| s.weight).sum::<f64>()),
            )
        };
        // The lip's own length, 1 mm, is a gap between the boattail and the base.
        let lip_gap = 1.0 - 0.001 / drop;
        for mach in [0.5, 1.5, 3.0] {
            for (rise, fraction) in [
                (0.1, 1.0),
                (0.25, 1.0),
                (0.375, 0.5),
                (0.5, 0.0),
                (0.7, 0.0),
            ] {
                // A tube longer than the boattail's drop takes the lip out of the wake.
                let (inside, outside) = (rocket(rise, None, 0.0), rocket(rise, Some(0.05), 0.0));
                let free = lip(&outside, mach).0;
                assert!(free > 0.0 && fraction_of(&outside) == (0.0, 0.0));
                let what = format!("rise {rise} at Mach {mach}");
                let want = (1.0 - fraction) * free;
                assert!((lip(&inside, mach).0 - want).abs() < 1e-9 * free, "{what}");
                let (wake, weight) = fraction_of(&inside);
                assert!((wake - fraction).abs() < 1e-9, "{what}: {wake}");
                assert!(
                    (weight - fraction * lip_gap).abs() < 1e-9,
                    "{what}: {weight}"
                );
            }
            // A tube half a drop long halves both, and the lip's length fades the base further.
            let (wake, weight) = fraction_of(&rocket(0.1, Some(0.5 * drop), 0.0));
            assert!((wake - 0.5).abs() < 1e-9 && (weight - (lip_gap - 0.5)).abs() < 1e-9);
            // Continuous across both ends of the fade.
            for edge in [0.25, 0.5] {
                let below = lip(&rocket(edge - 1e-9, None, 0.0), mach);
                let above = lip(&rocket(edge + 1e-9, None, 0.0), mach);
                assert!((below.0 - above.0).abs() < 1e-8, "pressure at {edge}");
                assert!((below.1 - above.1).abs() < 1e-8, "base at {edge}");
            }
            // A micrometre of step up, step down or tube before the lip.
            let exact = rocket(0.1, None, 0.0);
            let total = |m: &AeroModel| {
                m.drag(&Flow::axial(mach), &conditions)
                    .unwrap()
                    .zero_lift_coefficient
            };
            for other in [
                rocket(0.1, None, 1e-6),
                rocket(0.1, None, -1e-6),
                rocket(0.1, Some(1e-6), 0.0),
            ] {
                let (a, b) = (total(&exact), total(&other));
                assert!((a - b).abs() < 1e-3 * a, "Mach {mach}: {a} against {b}");
            }
        }
    }

    /// A narrowing elliptical, Haack or power-series transition ends in a blunt tip, where the
    /// profile's slope is infinite: the model still builds, and the boattail rule doesn't use the
    /// slope.
    #[test]
    fn curved_boattails_build_and_use_the_boattail_rule() {
        let (big, small, l) = (0.03, 0.02, 0.04);
        let a_ref = PI * big * big;
        let delta = PI * (big * big - small * small);
        let c_base = 0.12 + 0.13 * 0.09;
        for shape in [
            NoseShape::Elliptical {},
            NoseShape::Haack { parameter: 0.0 },
            NoseShape::PowerSeries { exponent: 0.5 },
            NoseShape::Conical {},
        ] {
            let mut tail = body_part(l, big, small);
            if let Part::Transition(t) = &mut tail {
                t.shape = shape;
            }
            let rocket = one_stage(
                vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, big),
                        None,
                    ),
                    component("tube", body_part(0.8, big, big), None),
                    component("tail", tail, None),
                ],
                ReferenceDiameter::Maximum {},
            );
            let m = model(&rocket);
            let tail = &m.bodies()[2];
            assert!(tail.geometry.aft_angle_rad <= 0.0, "{shape:?}");
            let parts = m
                .buildup_components(&Flow::axial(0.3), &DragConditions::coasting(RE_PER_M))
                .unwrap();
            // γ = 0.04/0.02 = 2: half the base drag on the decrease in area.
            close(
                parts[2].drag.pressure,
                0.5 * c_base * delta / a_ref,
                1e-14,
                "boattail",
            );
            close(
                parts[2].drag.base,
                c_base * PI * small * small / a_ref,
                1e-14,
                "base",
            );
        }
    }

    /// An override table on another reference diameter is rescaled by the ratio of the areas.
    #[test]
    fn drag_table_on_another_reference_diameter_is_rescaled() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.03), None),
                component("tube", body_part(0.8, 0.03, 0.03), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let conditions = DragConditions::coasting(RE_PER_M);
        let flow = Flow::axial(0.3);
        let table = DragTable::from_csv("0,0.5\n1,0.5\n", None).unwrap();
        let same = m.clone().with_drag_table(table.clone());
        assert_eq!(
            same.drag(&flow, &conditions).unwrap().zero_lift_coefficient,
            0.5
        );
        let wider = m
            .clone()
            .with_drag_table(table.clone().with_reference_diameter_m(0.09));
        close(
            wider
                .drag(&flow, &conditions)
                .unwrap()
                .zero_lift_coefficient,
            0.5 * 2.25,
            1e-14,
            "a 90 mm reference on a 60 mm rocket",
        );
        let bad = m.with_drag_table(table.with_reference_diameter_m(0.0));
        assert!(matches!(
            bad.drag(&flow, &conditions),
            Err(AeroError::Domain { .. })
        ));
    }

    /// The joint angle comes from the profile at the aft end: a power-series nose `(x/L)^n` meets
    /// its tube at `atan(n r/L)`, a von Kármán nose and a tangent ogive smoothly; a boattail that
    /// closes to a point leaves no base.
    #[test]
    fn joint_angles_from_the_profile_and_a_closed_tail() {
        let (r, l) = (0.03, 0.24);
        let a_ref = PI * r * r;
        let nose_pressure = |shape: NoseShape| {
            let rocket = one_stage(
                vec![
                    component("nose", nose(shape, l, r), None),
                    component("tube", body_part(0.8, r, r), None),
                ],
                ReferenceDiameter::Maximum {},
            );
            model(&rocket)
                .buildup_components(&Flow::axial(0.3), &DragConditions::coasting(RE_PER_M))
                .unwrap()[0]
                .drag
                .pressure
        };
        // At rest eq. 3.86's `0.8 sin² φ`; at Mach 0.3 the x^½ nose falls toward Stoney's 0 at
        // Mach 0.8 by the quadratic (eq. 3.87 has no rise to fit), and the cone rises by eq. 3.87.
        let phi = (0.5 * r / l).atan();
        let rest = 0.8 * phi.sin().powi(2);
        close(
            nose_pressure(NoseShape::PowerSeries { exponent: 0.5 }),
            rest * (1.0 - (0.3f64 / 0.8).powi(2)),
            1e-12,
            "x^0.5 nose",
        );
        let s = (r / l).atan().sin();
        let rest = 0.8 * s * s;
        let b = 4.0 / 2.4 * (1.0 - 0.5 * s) / (s - rest);
        close(
            nose_pressure(NoseShape::Conical {}),
            rest + (s - rest) * 0.3f64.powf(b),
            1e-12,
            "cone",
        );
        // Smooth joints: von Kármán stays at 0 until Stoney's curve leaves 0 past Mach 0.9; the
        // tangent ogive rises by eq. 3.87 toward the 4:1 cone's `sin ε` at Mach 1, by 1e-6 here.
        assert!(nose_pressure(NoseShape::Haack { parameter: 0.0 }) < 1e-20);
        let s = (0.125f64).atan().sin();
        let b = 4.0 / 2.4 * (1.0 - 0.5 * s) / s;
        close(
            nose_pressure(NoseShape::Ogive { radius_ratio: 1.0 }),
            s * 0.3f64.powf(b),
            1e-12,
            "tangent ogive",
        );

        // A 0.1 m cone closing a 30 mm tube to a point: γ = 0.1/0.06 < 3, no base.
        let rocket = one_stage(
            vec![
                component(
                    "nose",
                    nose(NoseShape::Ogive { radius_ratio: 1.0 }, l, r),
                    None,
                ),
                component("tube", body_part(0.8, r, r), None),
                component("tail", body_part(0.1, r, 0.0), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let parts = model(&rocket)
            .buildup_components(
                &Flow::axial(0.3),
                &DragConditions::thrusting(RE_PER_M, 1e-3),
            )
            .unwrap();
        let c_base = 0.12 + 0.13 * 0.09;
        let gamma: f64 = 0.1 / 0.06;
        close(
            parts[2].drag.pressure,
            0.5 * (3.0 - gamma) * c_base * a_ref / a_ref,
            1e-14,
            "tail",
        );
        assert_eq!(parts[2].drag.base, 0.0);
    }

    /// Loft lesson L18 (M1.8b2): Loft's wave drag was an invented curve, never measured against
    /// RASAero II. `cargo xtask aero` compares hpr's `C_D0` with RocketPy's RASAero curves every
    /// 0.05 from Mach 0.1 to 2.0, at sea level's Reynolds number for each Mach number, and records
    /// the errors by band in `validation/fixtures/aero/rocketpy-drag-curves.json` (the curves
    /// stay in `refs/`). This recomputes hpr's value at every row from the committed designs,
    /// checks every verdict and band summary against the rows, checks the errors where it can
    /// without the curves (at Mach 0.3 against the curve value recorded there, and between two
    /// cases on one curve), and pins how many rows of each band are within M1.8's 10%.
    /// `cargo xtask aero --check` checks every error against the curves when `refs/rocketpy` is
    /// fetched.
    ///
    /// The lesson named this test for supersonic drag within that tolerance. It isn't, and the
    /// decision records on the comparison measure why. Before the boattail's supersonic wave drag
    /// (ADR-030), Calisto's curve, the one real RASAero II export, read 24% to 30% above hpr from
    /// Mach 1.2, and no plausible fin section, thickness or finish was within 10% both below
    /// Mach 0.8 and from 1.2 (ADR-029). With it, 8 of its 17 supersonic rows are within 10% on
    /// the committed inputs (ADR-009's rule), and plausible fins bring 14 to 17 of them within 10%
    /// (`tests::calistos_rows_by_fin_and_finish`); the other curves are hand-edited, short, or
    /// disagree with their own rockets' OpenRocket files.
    #[test]
    fn supersonic_cd_against_rasaero_tables() {
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Fixture {
            tolerance_rel: f64,
            cases: Vec<Case>,
        }
        #[derive(Deserialize)]
        struct Case {
            id: String,
            design: String,
            curve: String,
            thrusting: bool,
            curve_cd0: f64,
            sweep: Sweep,
        }
        #[derive(Deserialize)]
        struct Sweep {
            usable_to_mach: Option<f64>,
            bands: Vec<Band>,
            rows: Vec<Row>,
        }
        #[derive(Deserialize)]
        struct Band {
            band: String,
            rows: usize,
            within_target: usize,
            min_error: f64,
            max_error: f64,
            rms_error: f64,
        }
        #[derive(Deserialize)]
        struct Row {
            mach: f64,
            band: String,
            hpr_cd0: f64,
            relative_error: f64,
            within_target: bool,
        }

        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/rocketpy-drag-curves.json"
        ))
        .unwrap();
        assert_eq!(fixture.tolerance_rel, 0.10);
        let air = hpr_atmos::Ussa76::standard().sample(0.0).unwrap().air;
        let band_of = |mach: f64| {
            if mach <= SUBSONIC_MACH_LIMIT {
                "subsonic"
            } else if mach < 1.2 {
                "transonic"
            } else {
                "supersonic"
            }
        };
        let mut within = Vec::new();
        // Each curve's values, as each case's rows imply them.
        let mut curves: std::collections::BTreeMap<&str, Vec<Vec<f64>>> =
            std::collections::BTreeMap::new();
        for case in &fixture.cases {
            let rocket = crate::testing::committed_design(&case.design);
            let model = AeroModel::new(&rocket.layout().unwrap()).unwrap();
            let motor_area: f64 = if case.thrusting {
                rocket.configurations[0]
                    .motors
                    .iter()
                    .map(|m| 0.25 * PI * m.diameter_m * m.diameter_m)
                    .sum()
            } else {
                0.0
            };
            let rows = &case.sweep.rows;
            // Every 0.05 from Mach 0.1, without a gap, to the curve's end or its usable limit.
            for (i, row) in rows.iter().enumerate() {
                assert_eq!(row.mach, f64::from(i as u32 + 2) / 20.0, "{}", case.id);
                assert!(
                    row.mach <= case.sweep.usable_to_mach.unwrap_or(2.0),
                    "{}@{}",
                    case.id,
                    row.mach
                );
                assert_eq!(row.band, band_of(row.mach), "{}@{}", case.id, row.mach);
                let reynolds_per_m =
                    row.mach * air.speed_of_sound_m_s / air.kinematic_viscosity_m2_s();
                let conditions = if case.thrusting {
                    DragConditions::thrusting(reynolds_per_m, motor_area)
                } else {
                    DragConditions::coasting(reynolds_per_m)
                };
                let drag = model.drag(&Flow::axial(row.mach), &conditions).unwrap();
                // A stale fixture: rerun `cargo xtask aero`.
                close(
                    drag.zero_lift_coefficient,
                    row.hpr_cd0,
                    1e-12,
                    &format!("{}@{}", case.id, row.mach),
                );
                assert_eq!(
                    row.within_target,
                    row.relative_error.abs() <= fixture.tolerance_rel,
                    "{}@{}",
                    case.id,
                    row.mach
                );
            }
            // The curves stay in `refs/`, so CI can't recompute the errors against them. What
            // it can check: at Mach 0.3 the error implies the curve value the Mach 0.3
            // comparison records, and two cases on one curve imply the same curve row by row.
            let implied = |r: &Row| r.hpr_cd0 / (1.0 + r.relative_error);
            let at_0_3 = rows.iter().find(|r| r.mach == 0.3).unwrap();
            assert!(
                (implied(at_0_3) / case.curve_cd0 - 1.0).abs() < 1e-12,
                "{}: the sweep's error at Mach 0.3 doesn't match the recorded curve",
                case.id
            );
            curves
                .entry(case.curve.as_str())
                .or_default()
                .push(rows.iter().map(implied).collect());
            let mut from = 0;
            for band in &case.sweep.bands {
                let errors: Vec<f64> = rows[from..from + band.rows]
                    .iter()
                    .map(|r| {
                        assert_eq!(r.band, band.band, "{}@{}", case.id, r.mach);
                        r.relative_error
                    })
                    .collect();
                from += band.rows;
                let count = errors.iter().filter(|e| e.abs() <= 0.10).count();
                assert_eq!(band.within_target, count, "{} {}", case.id, band.band);
                let min = errors.iter().copied().fold(f64::INFINITY, f64::min);
                let max = errors.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                let rms = (errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64).sqrt();
                assert_eq!((band.min_error, band.max_error), (min, max), "{}", case.id);
                assert!((band.rms_error - rms).abs() < 1e-15, "{}", case.id);
                within.push((case.id.as_str(), band.band.as_str(), count, band.rows));
            }
            assert_eq!(from, rows.len(), "{}: every row is in a band", case.id);
        }
        for (curve, cases) in &curves {
            for other in &cases[1..] {
                assert_eq!(other.len(), cases[0].len(), "{curve}");
                for (a, b) in cases[0].iter().zip(other) {
                    assert!((a / b - 1.0).abs() < 1e-12, "{curve}: {a} against {b}");
                }
            }
        }
        // Calisto's two designs share one export.
        assert_eq!(curves.values().filter(|c| c.len() == 2).count(), 1);
        // Rows within 10%, by case and band (ADR-029, ADR-030). Calisto's export on the 2018 fins:
        // every subsonic row, 3 of 7 transonic, and 8 of 17 supersonic, where hpr reads −14.9% to
        // −5.1% (−29.8% to −24.4% before the boattail's wave drag). The getting-started fins (a
        // variant on the same export, thick NACA 0012) now read 23% to 32% high. Juno III's
        // table is hand-edited from Mach 0.93 and Cavour's stop below Mach 0.93; Valetudo's is
        // 1.44 times its own OpenRocket export at Mach 0.3 (ADR-009).
        assert_eq!(
            within,
            [
                ("calisto-power-off", "subsonic", 15, 15),
                ("calisto-power-off", "transonic", 3, 7),
                ("calisto-power-off", "supersonic", 8, 17),
                ("calisto-getting-started-power-off", "subsonic", 12, 15),
                ("calisto-getting-started-power-off", "transonic", 0, 7),
                ("calisto-getting-started-power-off", "supersonic", 0, 17),
                ("juno-iii-power-off", "subsonic", 15, 15),
                ("juno-iii-power-off", "transonic", 0, 2),
                ("cavour-power-off", "subsonic", 6, 15),
                ("cavour-power-off", "transonic", 0, 1),
                ("cavour-power-on", "subsonic", 1, 15),
                ("cavour-power-on", "transonic", 0, 2),
                ("valetudo-power-off", "subsonic", 0, 15),
                ("valetudo-power-off", "transonic", 0, 7),
                ("valetudo-power-off", "supersonic", 0, 7),
                ("valetudo-power-on", "subsonic", 0, 15),
                ("valetudo-power-on", "transonic", 0, 7),
                ("valetudo-power-on", "supersonic", 0, 7),
            ]
        );
    }
}
