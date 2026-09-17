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
//! - **Body pressure drag**: `0.8 sin² φ` at a nose's or shoulder's aft joint (eq. 3.86) and the
//!   boattail rule (eq. 3.88) ([`joint_pressure_drag_coefficient`], [`boattail_factor`]).
//! - **Base drag** ([`base_drag_coefficient`], eq. 3.94) on the aft base, less the thrusting
//!   motors' area.
//! - **Fin pressure drag** ([`fin_pressure_drag_coefficient`], eq. 3.89–3.93) on the fins' frontal
//!   area `N t s`.
//! - **Parasitic drag** of launch lugs and rail buttons ([`launch_lug_drag`], eq. 3.95–3.96, and
//!   Niskanen's rail-pin rule, p. 52).
//! - **Angle of attack** ([`axial_drag_alpha_factor`], §3.4.7): `C_A = C_D0 f(α)`.
//!
//! Interference drag and fin-tip vortices are neglected, as in Niskanen p. 41.
//!
//! See `docs/physics/aero.md` and ADR-009.

use hpr_core::interp::Lookup;
use hpr_design::{FinCrossSection, FinSet, LaunchLug, PlacedComponent, RailButton};
use serde::{Deserialize, Serialize};

use crate::body::BodyGeometry;
use crate::error::{AeroError, check_dimension};
use crate::fins::FinGeometry;

/// Reynolds number below which the friction formulas no longer hold and the coefficient is held
/// at its value there (Niskanen 2009 p. 44).
pub const LOW_REYNOLDS: f64 = 1.0e4;

/// The skin-friction coefficient below [`LOW_REYNOLDS`] (Niskanen 2009 eq. 3.81).
pub const LOW_REYNOLDS_FRICTION: f64 = 1.48e-2;

/// The top of the subsonic region, Mach 0.8 (Niskanen 2009 Table 3.1, p. 19), where Niskanen's
/// semi-empirical transonic method starts (p. 47). The buildup accepts Mach numbers up to 1 and
/// flags results above this ([`Drag::beyond_subsonic_methods`]). The flag marks the region's edge,
/// not the start of the error: without eq. 3.87's high-subsonic interpolation (M1.8), nose and
/// shoulder pressure drag already reads low from about Mach 0.6.
pub const SUBSONIC_MACH_LIMIT: f64 = 0.8;

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
        return Ok(cf * (1.0 - 0.1 * m2));
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
    let m2 = mach * mach;
    Ok(if mach < 1.0 {
        1.0 + 0.25 * m2 + m2 * m2 / 40.0
    } else {
        let i2 = 1.0 / m2;
        1.84 - 0.76 * i2 + 0.166 * i2 * i2 + 0.035 * i2 * i2 * i2
    })
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

/// Low-subsonic pressure drag of a nose cone or shoulder, `(C_D•)_p = 0.8 sin² φ` on its frontal
/// area (a nose's base area, or a shoulder's increase in area), with `φ` the joint angle between
/// the surface and the body axis at the aft joint (Niskanen 2009 eq. 3.86, after NAVWEPS 1488
/// p. 237). A smooth joint (`φ = 0`) has none; a bare step (`φ = π/2`) has 0.8.
///
/// Niskanen interpolates from this value toward the transonic method above low subsonic speeds
/// (eq. 3.87); that arrives with the transonic method in M1.8, and until then the value is held.
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
/// continuous through 0 at 90°. The coefficients are derived, not published (ADR-009).
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
    /// Whether the buildup ran above [`SUBSONIC_MACH_LIMIT`], the top of Niskanen's subsonic
    /// region. Nose, shoulder and step pressure drag miss their rise toward Mach 1 until M1.8, so
    /// `C_D0` is low there (and somewhat low from about Mach 0.6). Never set with an override
    /// table.
    pub beyond_subsonic_methods: bool,
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
    /// Pressure drag of a nose or shoulder joint and of a step up in radius (eq. 3.86), which the
    /// subsonic model holds constant.
    pub joint_pressure: f64,
    /// Boattails and steps down in radius: `Σ` factor × decrease in area (eq. 3.88), times the
    /// base drag coefficient.
    pub boattail_area_ratio: f64,
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
            joint_pressure: 0.0,
            boattail_area_ratio: 0.0,
            fins: None,
            parasitic_area_ratio: 0.0,
            base_area_m2: 0.0,
        })
    }

    /// A body component's terms: friction on its surface, the step in area from the previous body
    /// component (`None` for the first, whose fore face counts as a step up from nothing), and its
    /// own pressure drag.
    ///
    /// The friction area is the surface's projection along the axis, `2π ∫ r dx = π A_plan`: the
    /// wall shear acts along the surface, so each element's axial share is `τ cos θ dA`. Niskanen's
    /// wetted area (eq. 3.85) omits the `cos θ`; the difference is small on slender noses, and
    /// without it a shoulder's friction would tend to a flat annulus's as its length goes to zero
    /// (ADR-009).
    pub(crate) fn body(
        component: &PlacedComponent,
        geometry: &BodyGeometry,
        previous_aft_area_m2: Option<f64>,
        form_factor: f64,
        length_m: f64,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        use std::f64::consts::{FRAC_PI_2, PI};
        let mut terms = Self::empty(component, length_m)?;
        terms.friction_area_ratio =
            form_factor * PI * geometry.planform_area_m2 / reference_area_m2;
        let step = geometry.fore_area_m2 - previous_aft_area_m2.unwrap_or(0.0);
        if step > 0.0 {
            terms.joint_pressure += joint_pressure_drag_coefficient(FRAC_PI_2)? * step;
        } else if step < 0.0 {
            // A zero-length boattail: `γ = 0`.
            terms.boattail_area_ratio -= step;
        }
        let change = geometry.aft_area_m2 - geometry.fore_area_m2;
        if change > 0.0 {
            let joint = geometry.aft_angle_rad.max(0.0);
            terms.joint_pressure += joint_pressure_drag_coefficient(joint)? * change;
        } else if change < 0.0 {
            let diameter = |area: f64| 2.0 * (area / PI).sqrt();
            let factor = boattail_factor(
                geometry.length_m,
                diameter(geometry.fore_area_m2),
                diameter(geometry.aft_area_m2),
            )?;
            terms.boattail_area_ratio -= factor * change;
        }
        terms.joint_pressure /= reference_area_m2;
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
        let friction = if self.friction_area_ratio > 0.0 {
            skin_friction_coefficient(reynolds, self.relative_roughness, mach)?
                * self.friction_area_ratio
        } else {
            0.0
        };
        let base_coefficient = base_drag_coefficient(mach)?;
        let mut pressure = self.joint_pressure + base_coefficient * self.boattail_area_ratio;
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
        let base = base_coefficient * (self.base_area_m2 - thrusting_motor_area_m2).max(0.0)
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
            beyond_subsonic_methods: mach > SUBSONIC_MACH_LIMIT,
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

        // Shoulder: 0.8 sin² φ ΔA with tan φ = Δr/l, and a bare step 0.8 ΔA.
        let step = rocket(small, big, None);
        close(
            pressure_of(&step),
            0.8 * delta / a_ref,
            1e-14,
            "bare step up",
        );
        let mut previous = f64::INFINITY;
        for l in [0.1, 0.01, 1e-3, 1e-5, 1e-8] {
            let s = rocket(small, big, Some(l));
            let phi = f64::atan((big - small) / l);
            close(
                pressure_of(&s),
                0.8 * phi.sin().powi(2) * delta / a_ref,
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
            m.drag(&Flow::axial(1.0), &DragConditions::coasting(RE_PER_M)),
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
        close(d.pressure, 0.8 * area_ratio, 1e-14, "flat face");
        close(
            d.base,
            (0.12 + 0.13 * 0.09) * area_ratio,
            1e-14,
            "flat base",
        );
        assert!(d.zero_lift_coefficient > 90.0);
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
        let cone_joint = (r / l_nose).atan();
        let nose_pressure = 0.8 * cone_joint.sin().powi(2);
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
        assert!(!got.beyond_subsonic_methods);
        let fast = m.drag(&Flow::axial(0.85), &conditions).unwrap();
        assert!(fast.beyond_subsonic_methods);
        assert!(
            !m.drag(&Flow::axial(0.8), &conditions)
                .unwrap()
                .beyond_subsonic_methods
        );

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

        // An override: the table's C_D0 (extrapolation reported), the same factor, and Mach 1.5
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
        let fast = o.drag(&Flow::axial(1.5), &conditions).unwrap();
        assert_eq!(fast.zero_lift_coefficient, 0.9);
        assert!(fast.table.unwrap().extrapolated.is_some());
        assert!(!fast.beyond_subsonic_methods);
        assert!(m.drag(&Flow::axial(1.5), &conditions).is_err());
        assert!(o.drag(&Flow::new(0.5, 4.0, 0.0), &conditions).is_err());
        assert!(
            o.buildup_components(&Flow::axial(1.5), &conditions)
                .is_err()
        );
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
        let phi = (0.5 * r / l).atan();
        close(
            nose_pressure(NoseShape::PowerSeries { exponent: 0.5 }),
            0.8 * phi.sin().powi(2),
            1e-12,
            "x^0.5 nose",
        );
        let cone = (r / l).atan();
        close(
            nose_pressure(NoseShape::Conical {}),
            0.8 * cone.sin().powi(2),
            1e-12,
            "cone",
        );
        assert!(nose_pressure(NoseShape::Haack { parameter: 0.0 }) < 1e-20);
        assert!(nose_pressure(NoseShape::Ogive { radius_ratio: 1.0 }) < 1e-20);

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
}
