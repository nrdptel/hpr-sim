//! The lip behind a boattail faster than sound (M1.8e8, ADR-039): the committed Arcas Robin
//! designs through the flight's path against the measured body alone (NASA TN D-4014, fins off),
//! with the candidate shares for the lip and what the measured pitching moment says about them,
//! written by `cargo xtask aero` to `validation/fixtures/aero/arcas-robin-lip.json`.
//!
//! - **What hpr flies:** the lip, wholly in the boattail's wake, carries no potential-flow slope
//!   faster than sound, so the shock-expansion method covers the body to its base. Each row holds
//!   the committed design's fitted slope and centre of pressure beside the same design with the
//!   lip left off (M1.8e7's comparison) and beside the measurement.
//! - **The candidates:** zero (flown); slender-body theory's `2 ΔA/A_ref`, which hpr flies below
//!   the join; and Seiff's embedded Newtonian upper bound (NASA TN D-1304 eq. 9, printed p. 12:
//!   `C_Nα = 4 ∫ (q₁/q∞) cos²θ (r/r_c) d(r/r_c)`, which for a conical flare at one `q₁` is
//!   `2 (q₁/q∞) cos²θ ΔA/A_ref`), with `q₁` the dynamic pressure of the flow that has expanded
//!   through the boattail's own turn. `θ` is taken to the axis (56.8° for the Arcas lip); Seiff's
//!   is to the local stream, 15° steeper behind this boattail, which would give a smaller share,
//!   so this is the looser bound of the two.
//! - **The moment check:** each row's `implied_share` is the lip share that would put hpr's centre
//!   of pressure on the measured one, with the standard error the readings leave. The summary
//!   fits one share to every row.
//!
//! No targets: the fixture shows where hpr stands.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::BodyModel;
use hpr_aero::afterbody::prandtl_meyer_angle;
use hpr_design::{Part, Rocket};
use serde_json::{Value, json};

use crate::aero_body::{ARCAS_NOSE_R_IN, INCH, arcas_model, arcas_model_without_lip};
use crate::aero_crossflow::{CP_DEG, MOMENT};
use crate::aero_gap::{READING, slope_error};
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/arcas-robin-lip.json";

/// What the fins-off pitching-moment plots were read to, by configuration
/// (`arcas-robin-fins-off-moment.json`'s `reading`): the report itself states ±0.05 for `C_m`.
pub const MOMENT_READING: [(&str, f64); 2] =
    [("arcas-robin-short", 0.02), ("arcas-robin-long", 0.025)];

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

fn points(curve: &Value, key: &str) -> Result<Vec<(f64, f64)>, String> {
    curve[key]
        .as_array()
        .ok_or(format!("a curve without `{key}`"))?
        .iter()
        .map(|p| match (p[0].as_f64(), p[1].as_f64()) {
            (Some(a), Some(c)) => Ok((a, c)),
            _ => Err("a malformed point".to_owned()),
        })
        .collect()
}

/// The dynamic pressure of the flow that has turned through `turn_rad` away from the free stream
/// at Mach `mach`, over the free stream's: `q₁/q∞ = (p₁ M₁²)/(p∞ M∞²)`, with `M₁` from the
/// Prandtl–Meyer angle and `p₁` isentropic from it.
fn expanded_dynamic_pressure(mach: f64, turn_rad: f64) -> Result<f64, String> {
    let nu = prandtl_meyer_angle(mach).map_err(|e| e.to_string())? + turn_rad;
    // The Prandtl–Meyer angle rises with the Mach number, so bisect for the Mach number that has
    // turned this far.
    let (mut low, mut high) = (mach, 100.0_f64);
    for _ in 0..200 {
        let mid = 0.5 * (low + high);
        if mid <= low || mid >= high {
            break;
        }
        match prandtl_meyer_angle(mid) {
            Ok(angle) if angle < nu => low = mid,
            _ => high = mid,
        }
    }
    if high >= 100.0 {
        return Err(format!(
            "no Mach number under 100 has turned {} rad past Mach {mach}",
            turn_rad
        ));
    }
    let expanded = high;
    let total = |m: f64| (1.0 + 0.2 * m * m).powf(3.5);
    let pressure = total(mach) / total(expanded);
    Ok(pressure * expanded * expanded / (mach * mach))
}

/// The committed design's lip: its fore and aft radii, its length, the station of its fore end,
/// and the boattail's turn ahead of it.
struct Lip {
    fore_radius_m: f64,
    aft_radius_m: f64,
    length_m: f64,
    fore_station_m: f64,
    boattail_turn_rad: f64,
}

impl Lip {
    fn of(root: &Path, design: &str) -> Result<Self, String> {
        let text = fs::read_to_string(root.join("validation/designs").join(design))
            .map_err(|e| format!("{design}: {e}"))?;
        let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
        let layout = rocket.layout().map_err(|e| format!("{design}: {e}"))?;
        let placed: Vec<_> = layout
            .components
            .iter()
            .filter(|c| matches!(c.part, Part::Transition(_)))
            .collect();
        let [boattail, lip] = placed.as_slice() else {
            return Err(format!("{design}: not a boattail and a lip"));
        };
        let (Part::Transition(boattail_part), Part::Transition(lip_part)) =
            (&boattail.part, &lip.part)
        else {
            unreachable!("both were filtered as transitions")
        };
        Ok(Self {
            fore_radius_m: lip_part.fore_radius_m,
            aft_radius_m: lip_part.aft_radius_m,
            length_m: lip_part.length_m,
            fore_station_m: lip.fore_station_m,
            boattail_turn_rad: ((boattail_part.fore_radius_m - boattail_part.aft_radius_m)
                / boattail_part.length_m)
                .atan(),
        })
    }

    /// Its angle to the axis, rad.
    fn angle_rad(&self) -> f64 {
        ((self.aft_radius_m - self.fore_radius_m) / self.length_m).atan()
    }

    /// Slender-body theory's share, `2 ΔA/A_ref`, per radian.
    fn slender_body(&self, reference_area_m2: f64) -> f64 {
        2.0 * PI * (self.aft_radius_m.powi(2) - self.fore_radius_m.powi(2)) / reference_area_m2
    }

    /// Seiff's embedded Newtonian share at Mach `mach`, per radian.
    fn seiff(&self, mach: f64, reference_area_m2: f64) -> Result<f64, String> {
        let dynamic = expanded_dynamic_pressure(mach, self.boattail_turn_rad)?;
        Ok(dynamic * self.angle_rad().cos().powi(2) * self.slender_body(reference_area_m2))
    }

    /// Where slender-body theory puts its share, m aft of the nose tip
    /// ([`hpr_design::Profile`]'s centre of pressure for a transition).
    fn station_m(&self) -> f64 {
        let (fore, aft) = (self.fore_radius_m, self.aft_radius_m);
        // A cone frustum's centre of area growth: Barrowman 1966 eq. 28 on a straight taper.
        let volume = PI * self.length_m * (fore * fore + fore * aft + aft * aft) / 3.0;
        let delta = PI * (aft * aft - fore * fore);
        self.fore_station_m + (self.length_m * PI * aft * aft - volume) / delta
    }
}

pub fn generate(root: &Path) -> Result<Value, String> {
    let reference = read(root, WIND_TUNNEL)?;
    let moment = read(root, MOMENT)?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
    let area = 0.25 * PI * diameter_m * diameter_m;
    let mut configurations = Vec::new();
    let mut fit = (0.0, 0.0, 0.0, 0usize);
    let mut by_model: Vec<(String, f64, f64, usize)> = Vec::new();
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
        let reading = READING
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(_, u)| *u)
            .ok_or(format!("no reading accuracy for {id}"))?;
        // The moment plots were read to their own accuracy, not the normal force's.
        let moment_reading = MOMENT_READING
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(_, u)| *u)
            .ok_or(format!("no moment reading accuracy for {id}"))?;
        let committed = arcas_model(root, design, None, false, BodyModel::CURRENT)?;
        let without = arcas_model_without_lip(root, design, BodyModel::CURRENT)?;
        if committed.supersonic_body().map_or(0, |t| t.covered) != 4 {
            return Err(format!("{id}: the committed design doesn't fly the method"));
        }
        let lip = Lip::of(root, design)?;
        let scale = committed.reference_area_m2() / area;
        let lip_station = lip.station_m() / diameter_m;
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
            let low = points(curve, "alpha_deg_c_n")?;
            let alphas: Vec<f64> = low.iter().map(|p| p.0.to_radians()).collect();
            let measured_slope = slope(&alphas, &low.iter().map(|p| p.1).collect::<Vec<_>>());
            let m_points = moment_curves
                .iter()
                .find(|c| c["mach"].as_f64() == Some(mach))
                .map(|c| points(c, "alpha_deg_c_m"))
                .ok_or(format!("{MOMENT}: no curve for {id} at Mach {mach}"))??;
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
            let (normal_fit, moment_fit) = (slope(&n_a, &n_c), slope(&m_a, &m_c));
            let measured_cp = moment_center_m / diameter_m - moment_fit / normal_fit;
            // The readings leave the two fitted slopes these standard errors, so the measured
            // centre of pressure carries this one.
            let (normal_error, moment_error) = (
                slope_error(&n_a, reading),
                slope_error(&m_a, moment_reading),
            );
            let measured_cp_error = ((moment_error / normal_fit).powi(2)
                + (moment_fit * normal_error / (normal_fit * normal_fit)).powi(2))
            .sqrt();
            let mut hpr = serde_json::Map::new();
            let mut fitted_cp = 0.0;
            let mut fitted_slope = 0.0;
            for (name, model) in [("committed", &committed), ("lip_off", &without)] {
                let c_n: Vec<f64> = low
                    .iter()
                    .map(|p| hpr_force(model, mach, p.0, true).map(|f| scale * f.0))
                    .collect::<Result<_, _>>()?;
                let slope_here = slope(&alphas, &c_n);
                let forces: Vec<(f64, f64)> = low
                    .iter()
                    .filter(inner)
                    .map(|p| hpr_force(model, mach, p.0, true))
                    .collect::<Result<_, _>>()?;
                let (normal, moments): (Vec<f64>, Vec<f64>) = forces.into_iter().unzip();
                let cp = slope(&n_a, &moments) / slope(&n_a, &normal) / diameter_m;
                if name == "committed" {
                    fitted_cp = cp;
                    fitted_slope = slope(&n_a, &normal) * scale;
                }
                hpr.insert(
                    name.to_string(),
                    json!({
                        "fitted_c_n_alpha": slope_here,
                        "fitted_c_n_alpha_error": slope_here / measured_slope - 1.0,
                        "cp_calibers": cp,
                        "cp_error_calibers": cp - measured_cp,
                    }),
                );
            }
            // The share at the lip's station that would put hpr's centre of pressure on the
            // measured one, and what the reading's uncertainty leaves of it.
            let lever = lip_station - measured_cp;
            let implied = fitted_slope * (measured_cp - fitted_cp) / lever;
            let implied_error = (fitted_slope * (lip_station - fitted_cp) / (lever * lever)).abs()
                * measured_cp_error;
            if !(implied.is_finite() && implied_error.is_finite() && implied_error > 0.0) {
                return Err(format!(
                    "{id} at Mach {mach}: the lip's implied share isn't a number (its lever is \
                     {lever} calibers)"
                ));
            }
            let candidates = json!({
                "zero": 0.0,
                "slender_body": lip.slender_body(area),
                "seiff": lip.seiff(mach, area)?,
            });
            // Where each candidate would put the centre of pressure.
            let with_share = |share: f64| {
                (fitted_slope * fitted_cp + share * lip_station) / (fitted_slope + share)
            };
            fit.0 += implied / (implied_error * implied_error);
            fit.1 += 1.0 / (implied_error * implied_error);
            fit.3 += 1;
            match by_model.last_mut().filter(|(name, ..)| name == id) {
                Some(entry) => {
                    entry.1 += implied / (implied_error * implied_error);
                    entry.2 += 1.0 / (implied_error * implied_error);
                    entry.3 += 1;
                }
                None => by_model.push((
                    id.to_owned(),
                    implied / (implied_error * implied_error),
                    1.0 / (implied_error * implied_error),
                    1,
                )),
            }
            rows.push(json!({
                "mach": mach,
                "measured": {
                    "fitted_c_n_alpha": measured_slope,
                    "cp_calibers": measured_cp,
                    "cp_standard_error_calibers": measured_cp_error,
                },
                "hpr": hpr,
                "lip": {
                    "candidates_per_rad": candidates,
                    "cp_with_slender_body": with_share(lip.slender_body(area)),
                    "cp_with_seiff": with_share(lip.seiff(mach, area)?),
                    "implied_share_per_rad": implied,
                    "implied_share_standard_error": implied_error,
                },
            }));
        }
        configurations.push(json!({
            "id": id,
            "design": design,
            "lip": {
                "fore_diameter_in": 2.0 * lip.fore_radius_m / INCH,
                "aft_diameter_in": 2.0 * lip.aft_radius_m / INCH,
                "length_in": lip.length_m / INCH,
                "angle_deg": lip.angle_rad().to_degrees(),
                "station_calibers": lip_station,
                "boattail_turn_deg": lip.boattail_turn_rad.to_degrees(),
            },
            "rows": rows,
        }));
    }
    // One share fitted to every row, and how badly the rows disagree about it; then the same for
    // each model alone, since the two disagree with each other (the physics review).
    let mean = fit.0 / fit.1;
    let error = fit.1.sqrt().recip();
    let mut per_model = Vec::new();
    for (configuration, (id, weighted, weight, rows)) in configurations.iter().zip(&by_model) {
        let (share, mut chi) = (weighted / weight, 0.0);
        for row in configuration["rows"].as_array().ok_or("no rows")? {
            let implied = row["lip"]["implied_share_per_rad"]
                .as_f64()
                .ok_or("no share")?;
            let sigma = row["lip"]["implied_share_standard_error"]
                .as_f64()
                .ok_or("no error")?;
            fit.2 += ((implied - mean) / sigma).powi(2);
            chi += ((implied - share) / sigma).powi(2);
        }
        per_model.push(json!({
            "id": id,
            "share_per_rad": share,
            "standard_error": weight.sqrt().recip(),
            "chi_squared_per_degree_of_freedom": chi / (rows.saturating_sub(1).max(1) as f64),
            "rows": rows,
        }));
    }
    let dof = fit.3.saturating_sub(1).max(1) as f64;
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_lip.rs)",
        "note": "The lip behind a boattail faster than sound (M1.8e8): hpr flies a lip wholly in a \
                 boattail's wake with no potential-flow slope above the supersonic join, so the \
                 committed Arcas Robin designs fly the shock-expansion method to their base. Rows \
                 hold the design as committed and the same design with the lip left off (M1.8e7), \
                 both through the flight's path with hpr's current body model, fitted at the \
                 tunnel's plotted angles as arcas-robin-crossflow.json fits them, against TN \
                 D-4014's fins-off measurement. Slopes are per radian on the body's cross-section; \
                 centres of pressure are calibers aft of the nose tip, hpr's from the same fits \
                 over |alpha| <= 4.5 deg. candidates_per_rad holds the lip's share under the rule \
                 flown (zero), slender-body theory (2 dA/A_ref, which hpr flies below the join) \
                 and Seiff's embedded Newtonian bound (NASA TN D-1304 eq. 9). implied_share is \
                 the share at the lip's station that would put hpr on the measured centre of \
                 pressure, with the standard error the readings leave. No targets.",
        "configurations": configurations,
        "one_share_fitted_to_every_row": {
            "share_per_rad": mean,
            "standard_error": error,
            "chi_squared_per_degree_of_freedom": fit.2 / dof,
            "rows": fit.3,
        },
        "one_share_fitted_to_each_model": per_model,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// `docs/physics/aero.md`'s *A lip in a boattail's wake* holds the fixture's table, cell by
    /// cell, and quotes the shares it fits.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        let mut rows = Vec::new();
        for configuration in fixture["configurations"].as_array().unwrap() {
            let model = configuration["id"]
                .as_str()
                .unwrap()
                .rsplit('-')
                .next()
                .unwrap();
            for r in configuration["rows"].as_array().unwrap() {
                rows.push(format!(
                    "| {model} | {} | {} | {} | {} | {} | {} | {} |",
                    f(r, "/mach"),
                    num(f(r, "/measured/fitted_c_n_alpha"), 3),
                    num(f(r, "/hpr/committed/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/committed/fitted_c_n_alpha_error")),
                    num(f(r, "/measured/cp_calibers"), 2),
                    num(f(r, "/hpr/committed/cp_calibers"), 2),
                    num(f(r, "/lip/cp_with_slender_body"), 2),
                ));
            }
        }
        assert_eq!(rows.len(), 11);
        // The shares the section's table quotes: the pooled fit, each model's, and the two ends
        // of the implied range.
        let sigma = |share: f64, error: f64, from: f64| {
            format!("{} σ", num(((share - from) / error).abs(), 1))
        };
        let mut fits = vec![("all eleven", &fixture["one_share_fitted_to_every_row"])];
        let models = fixture["one_share_fitted_to_each_model"]
            .as_array()
            .unwrap();
        fits.push(("the short model's six", &models[0]));
        fits.push(("the long model's five", &models[1]));
        for (label, fit) in fits {
            let (share, error) = (f(fit, "/share_per_rad"), f(fit, "/standard_error"));
            rows.push(format!(
                "| {label} | {}{} ± {} | {} | {} | {} |",
                if share < 0.0 { "" } else { "+" },
                num(share, 3),
                num(error, 3),
                num(f(fit, "/chi_squared_per_degree_of_freedom"), 1),
                sigma(share, error, 0.0),
                sigma(share, error, 0.178),
            ));
        }
        let (low, high) = (
            &fixture["configurations"][0]["rows"][0]["lip"],
            &fixture["configurations"][1]["rows"][3]["lip"],
        );
        rows.push(format!(
            "{} ± {}",
            num(f(low, "/implied_share_per_rad"), 3),
            num(f(low, "/implied_share_standard_error"), 3)
        ));
        rows.push(format!(
            "+{} ± {}",
            num(f(high, "/implied_share_per_rad"), 3),
            num(f(high, "/implied_share_standard_error"), 3)
        ));
        for row in rows {
            assert!(guide.contains(&row), "aero.md doesn't have `{row}`");
        }
    }

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
}
