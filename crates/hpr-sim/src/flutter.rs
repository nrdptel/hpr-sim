//! Fin flutter: the speed at which a fin's bending and twisting couple and grow, from D. J.
//! Martin's criterion ([the fin-flutter milestone][m1-10b]; the guide's [Fin flutter][page] page).
//!
//! **Source.** D. J. Martin, *Summary of Flutter Experiences as a Guide to the Preliminary Design
//! of Lifting Surfaces on Missiles*, NACA TN 4197, 1958, appendix, eqs. 16 to 19, pp. 14–15, and
//! figure 3, p. 19. Martin reduces Theodorsen and Garrick's flutter speed for a bending-torsion
//! wing (eq. 1) to a few planform numbers. With `G_E` the fin's effective shear modulus, `A` the
//! panel aspect ratio (span over mid-span chord), `λ` the taper ratio (tip over root chord), `t/c`
//! the thickness ratio, `p` the static pressure and `a` the speed of sound, eq. 16 with his
//! `1/(f₁² f₂²) ≈ (λ + 1)/2` reads
//!
//! ```text
//! (V_f / a)² = G_E / D,   D = (24 ε / π) ρ a² · A³ / ((t/c)³ (A + 2)) · (λ + 1)/2
//! ```
//!
//! and with `ρ a² = γ p` (eq. 17), `ε = 0.25` and `γ = 1.4` it becomes eq. 18, whose constant
//! `24 · 0.25 · 1.4 / π · 14.696 psi = 39.29 psi` Martin prints as 39.3:
//!
//! ```text
//! (V_f / a)² = G_E / (39.3 A³ / ((t/c)³ (A + 2)) · (λ + 1)/2 · p/p₀)
//! ```
//!
//! The constant is derived, not fitted. What is empirical is the aspect-ratio correction
//! `A/(A + 2)`, the best of those Martin tried, and where the line falls between wings that fluttered
//! and wings that didn't: his figure 3 plots `D` against `G_E` for missiles and wind-tunnel models,
//! and a band separates them at `D/G_E` from 0.25 to 0.31 ([`FIGURE_3_BAND`]), a flutter speed
//! of 1.8 to 2.0 times the speed of sound. His open points are wings that flew to at least Mach 1.3
//! without failing. So eq. 18's `V_f` is a parameter his data calibrates, not a speed at which a
//! fin is known to flutter.
//!
//! Loft wrote the constant as `1.337 · (λ + 1)/2` psi, half of `39.3/14.696 = 2.674`, so its
//! flutter speed was `√2` too high, on the unsafe side ([Loft lesson L32][l32]).
//!
//! **A flutter dynamic pressure.** Since `ρ a² = 2q/M²` for any gas, eq. 16 fixes the dynamic
//! pressure at flutter, whatever the height:
//!
//! ```text
//! q_f = ½ ρ V_f² = π G_E / (24 ε K (λ + 1)),   K = A³ / ((t/c)³ (A + 2))
//! ```
//!
//! and a fin flying at dynamic pressure `q` is below eq. 18's flutter speed by the ratio
//! `V_f / V = √(q_f / q)`. The least ratio of a flight is therefore at its peak dynamic pressure,
//! which [`crate::metrics::FlightMetrics`] finds on the dense output.
//!
//! **Readings.** A trapezoidal fin's `A` is `2s/(c_r + c_t)` (span `s` over the mid-span chord) and
//! `t/c` is the thickness over the root chord: Martin's `c` is the root chord of his
//! constant-thickness-ratio wing, and for a flat fin of constant thickness the root's ratio is the
//! smallest, so the flutter speed the least. `G_E` is Martin's effective shear modulus: for a
//! solid wing he takes the material's own (his p. 6), which this module does. His definition,
//! `G_E = 6 J G / (c t³)` (eq. 12), would give a flat plate, whose torsion constant is
//! `J = c t³/3`, about twice that, and a flutter speed `√2` higher; the lower reading is kept. For
//! a NACA four-digit section it gives `0.946 G`, so an airfoiled fin's `V_f` here is up to 2.7%
//! high.
//!
//! **Left out.** Sweep, the fin's mounting and the body's own modes (Martin's figure 8), stall
//! flutter at high angles of attack, Mach number effects such as a transonic dip, and every other
//! flutter type Martin lists. It is a screening number, not a flutter analysis.
//!
//! [l32]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l32
//! [m1-10b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-10b
//! [page]: https://nrdptel.github.io/hpr-sim/physics/flutter.html

use std::f64::consts::PI;

use hpr_design::{FinPlanform, FinSet};
use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::metrics::FlightSummary;

/// Martin's `ε`: how far the section's centre of mass sits behind its quarter chord, as a fraction
/// of the chord, assumed 0.25, which puts it at mid-chord (NACA TN 4197, p. 14).
pub const CG_AFT_OF_QUARTER_CHORD: f64 = 0.25;

/// Martin's ratio of specific heats for air, 1.4 (NACA TN 4197, eq. 17, p. 14).
pub const HEAT_CAPACITY_RATIO: f64 = 1.4;

/// Where Martin's figure 3 separates wings that fluttered from wings that didn't, as `D/G_E`,
/// that is `(a/V_f)²`: its shaded band runs from 0.25 to 0.31.
///
/// Measured on the scan of NACA TN 4197's figure 3 (p. 19) rendered at 250 dpi: both log axes
/// calibrated on their tick marks (222.5 and 224.8 pixels a decade), the band's edges traced in
/// 69 columns from `G_E` = 0.05 to 10 × 10⁶ psi (0.34 to 69 GPa; the axis runs on to 20 × 10⁶ psi).
/// Its middle stays at `D/G_E` = 0.28 to 0.29 all along, and it is about 0.08 of a decade wide. Above it lie
/// mostly wings that fluttered or failed, and a few that didn't; below it, wings that flew to at
/// least Mach 1.3 without known failure.
pub const FIGURE_3_BAND: [f64; 2] = [0.25, 0.31];

/// The planform numbers Martin's criterion takes from one fin.
///
/// A birch-plywood fin with a 200 mm root, a 100 mm tip, a 120 mm span and 4 mm thick has
/// `A = 0.8`, `λ = 0.5` and `t/c = 0.02`, so `K = 0.8³ / (0.02³ · 2.8) = 22 857`. With plywood's
/// 750 MPa it flutters at `q_f = π · 750 MPa / (6 · 22 857 · 1.5) = 11.45 kPa`: 137 m/s in
/// sea-level air.
///
/// ```
/// use hpr_design::materials;
/// use hpr_sim::FlutterPanel;
///
/// let panel = FlutterPanel::new(0.8, 0.5, 0.02)?;
/// let g = materials::shear_modulus("birch_plywood").unwrap().shear_modulus_pa;
/// let q_f = panel.flutter_dynamic_pressure_pa(g)?;
/// let v_f = panel.flutter_speed_m_s(g, 101_325.0, 340.294)?;
/// assert!((q_f - 11_453.7).abs() < 0.1, "{q_f}");
/// assert!((v_f - 136.75).abs() < 0.01, "{v_f}");
/// // The same speed from q_f and sea-level density, 1.225 kg/m³.
/// assert!(((2.0 * q_f / 1.225).sqrt() - v_f).abs() < 0.01);
/// # Ok::<(), hpr_sim::SimError>(())
/// ```
///
/// Its numbers are checked when it is made, by [`FlutterPanel::new`], and when it is read from
/// JSON.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PanelNumbers")]
pub struct FlutterPanel {
    aspect_ratio: f64,
    taper_ratio: f64,
    thickness_ratio: f64,
}

/// A panel's numbers as JSON holds them, before they are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelNumbers {
    aspect_ratio: f64,
    taper_ratio: f64,
    thickness_ratio: f64,
}

impl TryFrom<PanelNumbers> for FlutterPanel {
    type Error = SimError;

    fn try_from(n: PanelNumbers) -> Result<Self, SimError> {
        Self::new(n.aspect_ratio, n.taper_ratio, n.thickness_ratio)
    }
}

impl FlutterPanel {
    /// A panel of aspect ratio `A`, taper ratio `λ` and thickness ratio `t/c`.
    ///
    /// Martin's figure 4 covers `A` from 0.5 to 3 and `t/c` from 1% to 10%; outside those the
    /// numbers are an extrapolation, which isn't refused.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `A` or `t/c` isn't positive and finite, or `λ` isn't within
    /// `[0, 1]`, the range of Martin's taper factors (NACA TN 4197, eqs. 8 and 14).
    pub fn new(
        aspect_ratio: f64,
        taper_ratio: f64,
        thickness_ratio: f64,
    ) -> Result<Self, SimError> {
        positive("flutter panel aspect ratio", aspect_ratio)?;
        positive("flutter panel thickness ratio", thickness_ratio)?;
        if !(0.0..=1.0).contains(&taper_ratio) {
            return Err(SimError::Domain {
                what: "flutter panel taper ratio (0 to 1)",
                value: taper_ratio,
            });
        }
        Ok(Self {
            aspect_ratio,
            taper_ratio,
            thickness_ratio,
        })
    }

    /// The panel of one of `fins`: `A = 2s/(c_r + c_t)`, `λ = c_t/c_r` and `t/c = t/c_r`.
    ///
    /// # Errors
    ///
    /// [`SimError::Unsupported`] for an elliptical or freeform planform, or a tip chord longer
    /// than the root, which Martin's taper factors don't cover; [`SimError::Domain`] for a root
    /// chord that isn't positive, or the panel's numbers out of range, as [`FlutterPanel::new`].
    pub fn of_fins(fins: &FinSet) -> Result<Self, SimError> {
        match fins.planform {
            FinPlanform::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                ..
            } => {
                positive("fin root chord", root_chord_m)?;
                if tip_chord_m > root_chord_m {
                    return Err(SimError::Unsupported {
                        what: "a flutter panel of a fin whose tip chord is longer than its root",
                    });
                }
                Self::new(
                    2.0 * span_m / (root_chord_m + tip_chord_m),
                    tip_chord_m / root_chord_m,
                    fins.thickness_m / root_chord_m,
                )
            }
            _ => Err(SimError::Unsupported {
                what: "a flutter panel of a planform other than a trapezoid",
            }),
        }
    }

    /// The panel aspect ratio `A`: the span over the chord at mid-span.
    #[must_use]
    pub fn aspect_ratio(&self) -> f64 {
        self.aspect_ratio
    }

    /// The taper ratio `λ`: the tip chord over the root chord, from 0 (pointed) to 1.
    #[must_use]
    pub fn taper_ratio(&self) -> f64 {
        self.taper_ratio
    }

    /// The thickness ratio `t/c`: the thickness over the root chord.
    #[must_use]
    pub fn thickness_ratio(&self) -> f64 {
        self.thickness_ratio
    }

    /// `K = A³ / ((t/c)³ (A + 2))`: Martin's `X` (eq. 19) without its constant.
    #[must_use]
    pub fn shape_factor(&self) -> f64 {
        let a = self.aspect_ratio;
        a.powi(3) / (self.thickness_ratio.powi(3) * (a + 2.0))
    }

    /// The denominator of eq. 18 at static pressure `p`, Pa:
    /// `D = (24 ε γ / π) p · K · (λ + 1)/2`, the ordinate of Martin's figure 3.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `p` isn't positive and finite.
    pub fn denominator_pa(&self, pressure_pa: f64) -> Result<f64, SimError> {
        positive("static pressure", pressure_pa)?;
        Ok(24.0 * CG_AFT_OF_QUARTER_CHORD * HEAT_CAPACITY_RATIO / PI
            * pressure_pa
            * self.shape_factor()
            * (self.taper_ratio + 1.0)
            / 2.0)
    }

    /// Martin's figure 3 reading, `D/G_E = (a/V_f)²`, at static pressure `p`: above
    /// [`FIGURE_3_BAND`] lie mostly his wings that fluttered, below it wings that didn't. Martin
    /// takes `p` where the wing flies; at the launch site's, the highest a flight sees, it is the
    /// largest. Outside his axis, `G_E` from 0.34 to 138 GPa, it is an extrapolation.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `G_E` or `p` isn't positive and finite.
    pub fn figure_3_ratio(&self, shear_modulus_pa: f64, pressure_pa: f64) -> Result<f64, SimError> {
        positive("effective shear modulus", shear_modulus_pa)?;
        Ok(self.denominator_pa(pressure_pa)? / shear_modulus_pa)
    }

    /// The dynamic pressure at which a fin of effective shear modulus `G_E` reaches eq. 18's
    /// flutter speed, Pa: `q_f = π G_E / (24 ε K (λ + 1))`, the same at every height.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `G_E` isn't positive and finite.
    pub fn flutter_dynamic_pressure_pa(&self, shear_modulus_pa: f64) -> Result<f64, SimError> {
        positive("effective shear modulus", shear_modulus_pa)?;
        Ok(PI * shear_modulus_pa
            / (24.0 * CG_AFT_OF_QUARTER_CHORD * self.shape_factor() * (self.taper_ratio + 1.0)))
    }

    /// Eq. 18's flutter speed in air of static pressure `p` and speed of sound `a`, m/s:
    /// `V_f = a √(G_E / D)`.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `G_E`, `p` or `a` isn't positive and finite.
    pub fn flutter_speed_m_s(
        &self,
        shear_modulus_pa: f64,
        pressure_pa: f64,
        sound_speed_m_s: f64,
    ) -> Result<f64, SimError> {
        positive("speed of sound", sound_speed_m_s)?;
        Ok(sound_speed_m_s / self.figure_3_ratio(shear_modulus_pa, pressure_pa)?.sqrt())
    }

    /// The fin's least ratio of eq. 18's flutter speed to its airspeed over the flight `summary`
    /// describes, at its peak dynamic pressure; `None` if the rocket never flew.
    ///
    /// The whole flight's peak is used for every fin set on it. A booster's fins leave at the
    /// separation, so the peak can come after they have gone; their true ratio is then at least
    /// the one given, as long as the booster's own dynamic pressure after the separation stays
    /// below the flight's peak, which hpr doesn't check.
    ///
    /// # Errors
    ///
    /// As [`FlutterPanel::flutter_dynamic_pressure_pa`]; [`SimError::Domain`] if the peak
    /// dynamic pressure isn't finite.
    pub fn margin(
        &self,
        shear_modulus_pa: f64,
        summary: &FlightSummary,
    ) -> Result<Option<FlutterMargin>, SimError> {
        let flutter_dynamic_pressure_pa = self.flutter_dynamic_pressure_pa(shear_modulus_pa)?;
        let Some(peak) = summary.max_dynamic_pressure_pa else {
            return Ok(None);
        };
        if !peak.value.is_finite() {
            return Err(SimError::Domain {
                what: "peak dynamic pressure",
                value: peak.value,
            });
        }
        Ok((peak.value > 0.0).then(|| FlutterMargin {
            time_s: peak.time_s,
            height_above_ground_m: peak.height_above_ground_m,
            dynamic_pressure_pa: peak.value,
            flutter_dynamic_pressure_pa,
            speed_ratio: (flutter_dynamic_pressure_pa / peak.value).sqrt(),
        }))
    }
}

/// How far below eq. 18's flutter speed a fin flew, at the flight's peak dynamic pressure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlutterMargin {
    /// When the peak came, s.
    pub time_s: f64,
    /// The centre of mass's height above the launch site then, m.
    pub height_above_ground_m: f64,
    /// The flight's peak dynamic pressure, Pa.
    pub dynamic_pressure_pa: f64,
    /// The dynamic pressure at eq. 18's flutter speed, Pa.
    pub flutter_dynamic_pressure_pa: f64,
    /// Eq. 18's flutter speed over the airspeed there, `V_f / V = √(q_f / q)`, the flight's least.
    /// Below 1 the fin flies faster than eq. 18's flutter speed. No fixed value is a safe line:
    /// Martin's band ([`FIGURE_3_BAND`]) puts `V_f` at 1.8 to 2.0 times the speed of sound, so at
    /// Mach `M` the band is at a ratio of `1.8/M` to `2.0/M`, and below it above `2.0/M`. Judge a fin by
    /// [`FlutterPanel::figure_3_ratio`].
    pub speed_ratio: f64,
}

fn positive(what: &'static str, value: f64) -> Result<(), SimError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(SimError::Domain { what, value })
    }
}

#[cfg(test)]
mod tests {
    use hpr_design::{Component, Part};

    use super::*;
    use crate::environment::Environment;
    use crate::flight::{FlightSettings, Simulation};
    use crate::metrics::{FlightMetrics, Peak};
    use crate::rail::Rail;
    use crate::recorder::{Channel, Recorder};
    use crate::testing::{design, site};

    /// One pound per square inch, Pa (NIST SP 811, 2008, B.9: 6.894 757 E+03).
    const PSI: f64 = 6_894.757;

    /// Standard sea-level pressure, Pa (U.S. Standard Atmosphere, 1976): Martin's `p₀`, 14.696
    /// psi.
    const P0: f64 = 101_325.0;

    fn panel(aspect_ratio: f64, taper_ratio: f64, thickness_ratio: f64) -> FlutterPanel {
        FlutterPanel::new(aspect_ratio, taper_ratio, thickness_ratio).unwrap()
    }

    /// Martin's `X` (eq. 19) times `(λ + 1)/2 · p/p₀`: the ordinate of his figure 3, in psi.
    fn ordinate_psi(p: &FlutterPanel, pressure_pa: f64) -> f64 {
        p.denominator_pa(pressure_pa).unwrap() / PSI
    }

    /// Eq. 18's constant is eq. 16's `24 ε γ p₀ / π` to the three figures Martin prints, and
    /// twice Loft's `1.337` (L32).
    #[test]
    fn flutter_denominator_matches_tn_4197_eq_18() {
        let constant_psi = 24.0 * CG_AFT_OF_QUARTER_CHORD * HEAT_CAPACITY_RATIO / PI * P0 / PSI;
        assert!((constant_psi - 39.3).abs() < 0.05, "{constant_psi}");
        for (a, lambda, tc) in [(2.0, 1.0, 0.04), (1.3, 0.4, 0.025), (3.1, 0.0, 0.07)] {
            let p = panel(a, lambda, tc);
            for pressure in [P0, 0.4 * P0] {
                let eq_18_psi = 39.3 * a.powi(3) / (tc.powi(3) * (a + 2.0)) * (lambda + 1.0) / 2.0
                    * (pressure / P0);
                // Only Martin's rounding of 39.29 to 39.3 apart.
                let ours = ordinate_psi(&p, pressure);
                assert!(
                    (ours / eq_18_psi - 1.0).abs() < 0.05 / 39.3,
                    "{a} {lambda} {tc}"
                );
            }
        }
        // Loft's `1.337 · (λ + 1)/2` per psi is half of `39.3 / 14.696`: its flutter speed was
        // `√2` too high.
        let per_psi = constant_psi / (P0 / PSI);
        assert!((per_psi / 1.337 - 2.0).abs() < 1e-3, "{per_psi}");
    }

    /// The flutter speed goes as `(t/c)^{3/2}`, `√G_E` and `1/√p` at a fixed speed of sound, so
    /// its dynamic pressure doesn't depend on the air, and the margin is `√(q_f/q)`.
    #[test]
    fn scaling_laws_in_thickness_shear_modulus_and_pressure() {
        let g = 26e9;
        let (p, a) = (P0, 340.3);
        let base = panel(1.5, 0.5, 0.03);
        let v = base.flutter_speed_m_s(g, p, a).unwrap();
        let thicker = panel(1.5, 0.5, 0.06).flutter_speed_m_s(g, p, a).unwrap();
        assert!((thicker / v - 2f64.powf(1.5)).abs() < 1e-12);
        let stiffer = base.flutter_speed_m_s(4.0 * g, p, a).unwrap();
        assert!((stiffer / v - 2.0).abs() < 1e-12);
        let thin_air = base.flutter_speed_m_s(g, p / 4.0, a).unwrap();
        assert!((thin_air / v - 2.0).abs() < 1e-12);
        let faster_sound = base.flutter_speed_m_s(g, p, 2.0 * a).unwrap();
        assert!((faster_sound / v - 2.0).abs() < 1e-12);
        // q_f = ½ ρ V_f² with ρ = γ p / a², at any pressure and speed of sound.
        let q_f = base.flutter_dynamic_pressure_pa(g).unwrap();
        for (p, a) in [(P0, 340.3), (0.3 * P0, 300.0), (2.0 * P0, 360.0)] {
            let v = base.flutter_speed_m_s(g, p, a).unwrap();
            let q = 0.5 * HEAT_CAPACITY_RATIO * p / (a * a) * v * v;
            assert!((q / q_f - 1.0).abs() < 1e-12, "{q} {q_f}");
        }
    }

    /// Martin's worked examples (NACA TN 4197, pp. 6–7), read at the resolution he prints: `X`
    /// for `A = 2` and 4% thickness is "about 1.25 × 10⁶" psi (eq. 19 gives 1.228 × 10⁶, which is
    /// 1.25 to the nearest 0.05 × 10⁶), and a titanium wing held to an ordinate of 0.8 × 10⁶ psi
    /// needs 2.5, 4.5 and "about 6.5" percent at `A = 1`, 2 and 3 (eq. 19 gives 2.54, 4.61 and
    /// 6.43, each those to the nearest half percent).
    ///
    /// His verdicts on the first are the margin half: in solid magnesium the wing "would plot in
    /// the flutter region", in aluminium it "would be marginal", in steel "probably safe". With
    /// the moduli he marks on figure 3's axis, each a small box measured on the 250 dpi scan
    /// (outer walls, × 10⁶ psi: magnesium 2.40 to 2.63, aluminium 3.82 to 4.28, titanium 5.78 to
    /// 6.34, steel 8.92 to 11.3), and his band, magnesium's ratio lies wholly above the band,
    /// aluminium's overlaps it, and steel's lies wholly below; his second example's titanium, held
    /// to 0.8 × 10⁶ psi, lies below it.
    #[test]
    fn martins_worked_examples() {
        let x_psi = |a: f64, tc: f64| ordinate_psi(&panel(a, 1.0, tc), P0);
        let x = x_psi(2.0, 0.04);
        assert!((x / 1e6 - 1.227_9).abs() < 1e-4, "{x}");
        assert_eq!((x / 0.05e6).round() * 0.05, 1.25);
        for (a, printed_percent) in [(1.0, 2.5), (2.0, 4.5), (3.0, 6.5)] {
            // X ∝ (t/c)⁻³: the thickness that brings X to 0.8 × 10⁶ psi.
            let tc = 0.01 * (x_psi(a, 0.01) / 0.8e6).cbrt();
            assert_eq!((200.0 * tc).round() / 2.0, printed_percent, "{a}: {tc}");
        }
        let [low, high] = FIGURE_3_BAND;
        // The figure 3 ratio over a material's box: the stiffest end gives the least.
        let ratios = |p: &FlutterPanel, [soft, stiff]: [f64; 2]| {
            [stiff, soft].map(|g| p.figure_3_ratio(g * 1e6 * PSI, P0).unwrap())
        };
        let first = panel(2.0, 1.0, 0.04);
        let magnesium = ratios(&first, [2.40, 2.63]);
        let aluminium = ratios(&first, [3.82, 4.28]);
        let steel = ratios(&first, [8.92, 11.3]);
        // The second example's titanium wing: the thickness that holds X at 0.8 × 10⁶ psi.
        let held = panel(2.0, 1.0, 0.04 * (x / 0.8e6).cbrt());
        assert!((ordinate_psi(&held, P0) / 0.8e6 - 1.0).abs() < 1e-12);
        let titanium = ratios(&held, [5.78, 6.34]);
        assert!(magnesium[0] > high, "{magnesium:?}");
        assert!(aluminium[0] < high && aluminium[1] > low, "{aluminium:?}");
        assert!(steel[1] < low, "{steel:?}");
        assert!(titanium[1] < low, "{titanium:?}");
    }

    /// Martin replaces `1/(f₁² f₂²)`, with `f₁ = 1 + 1.87 (1 − λ)^1.6` (eq. 8) and
    /// `f₂ = (1 + 3λ)/(2(1 + λ))` (eq. 14), by `(λ + 1)/2`: equal at `λ = 1`, 3% apart at `λ = 0`,
    /// and up to 47% larger between (at `λ ≈ 0.31`), which lowers the flutter speed there by up to
    /// 17.5%. The model keeps his form, since his figure 3 was drawn with it.
    #[test]
    fn taper_factor_against_the_frequency_factors() {
        let exact = |lambda: f64| {
            let f1 = 1.0 + 1.87 * (1.0 - lambda).powf(1.6);
            let f2 = (1.0 + 3.0 * lambda) / (2.0 * (1.0 + lambda));
            1.0 / (f1 * f2).powi(2)
        };
        assert!((exact(1.0) - 1.0).abs() < 1e-15);
        assert!((0.5 / exact(0.0) - 1.029).abs() < 1e-3, "{}", exact(0.0));
        let (worst, at) = (0..=1000)
            .map(|i| f64::from(i) / 1000.0)
            .map(|lambda| ((lambda + 1.0) / 2.0 / exact(lambda), lambda))
            .fold((0.0, 0.0), |x, y| if y.0 > x.0 { y } else { x });
        assert!(
            (worst - 1.469).abs() < 1e-3 && (at - 0.308).abs() < 1e-9,
            "{worst} at {at}"
        );
        assert!((1.0 - worst.sqrt().recip() - 0.175).abs() < 1e-3);
    }

    /// On a real flight the margin is at the peak dynamic pressure and is the least of every
    /// millisecond. At each row, eq. 18 at the pressure and speed of sound its `q` and Mach number
    /// imply (`p = 2q/(γM²)`, `a = V/M`) agrees with `√(q_f/q)`: the two methods are consistent.
    #[test]
    fn the_margin_is_the_flights_least_at_its_peak_dynamic_pressure() {
        fn find_fins(components: &[Component]) -> Option<&FinSet> {
            components.iter().find_map(|c| match &c.part {
                Part::FinSet(set) => Some(set),
                _ => find_fins(&c.children),
            })
        }
        let rocket = design("rocketpy-valetudo");
        let fins = rocket
            .stages
            .iter()
            .find_map(|stage| find_fins(&stage.components))
            .unwrap();
        let panel = FlutterPanel::of_fins(fins).unwrap();
        // An arbitrary shear modulus: the checks hold for any.
        let g = 3e9;
        let sim = Simulation::new(
            &rocket,
            "example",
            Environment::standard(site()).unwrap(),
            Rail::vertical(5.0),
            FlightSettings {
                max_time_s: 20.0,
                ..FlightSettings::default()
            },
        )
        .unwrap();
        let mut metrics = FlightMetrics::new();
        let result = sim.run(&mut metrics).unwrap();
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        let margin = panel.margin(g, &summary).unwrap().unwrap();
        let q_f = panel.flutter_dynamic_pressure_pa(g).unwrap();
        let peak = summary.max_dynamic_pressure_pa.unwrap();
        assert_eq!(margin.time_s, peak.time_s);
        assert_eq!(margin.dynamic_pressure_pa, peak.value);
        assert_eq!(margin.speed_ratio, (q_f / peak.value).sqrt());
        let mut recorder = Recorder::new(
            vec![Channel::DynamicPressure, Channel::Mach, Channel::Airspeed],
            Some(1e-3),
        )
        .unwrap();
        sim.run(&mut recorder).unwrap();
        let mut checked = 0;
        for row in recorder.rows() {
            let (q, mach, speed) = (row[0], row[1], row[2]);
            if mach < 0.05 {
                continue;
            }
            let ratio = (q_f / q).sqrt();
            assert!(
                ratio >= margin.speed_ratio,
                "{ratio} {}",
                margin.speed_ratio
            );
            let pressure = 2.0 * q / (HEAT_CAPACITY_RATIO * mach * mach);
            let v_f = panel.flutter_speed_m_s(g, pressure, speed / mach).unwrap();
            assert!(
                (v_f / speed / ratio - 1.0).abs() < 1e-12,
                "{v_f} {speed} {ratio}"
            );
            checked += 1;
        }
        assert!(checked > 1000, "{checked}");
        // A flight with no peak has no margin; a peak that isn't finite is refused.
        let mut never = summary.clone();
        never.max_dynamic_pressure_pa = None;
        assert_eq!(panel.margin(g, &never).unwrap(), None);
        never.max_dynamic_pressure_pa = Some(Peak {
            value: f64::NAN,
            ..peak
        });
        assert!(matches!(
            panel.margin(g, &never),
            Err(SimError::Domain {
                what: "peak dynamic pressure",
                ..
            })
        ));
    }

    #[test]
    fn a_trapezoidal_fin_set_gives_its_panel() {
        let fins: FinSet = serde_json::from_value(serde_json::json!({
            "count": 3,
            "planform": {"kind": "trapezoidal", "root_chord_m": 0.2, "tip_chord_m": 0.1,
                         "span_m": 0.12, "sweep_m": 0.1},
            "thickness_m": 0.004,
            "material": {"name": "test", "density": {"kind": "bulk", "kg_m3": 1850.0}}
        }))
        .unwrap();
        let p = FlutterPanel::of_fins(&fins).unwrap();
        assert!((p.aspect_ratio() - 0.8).abs() < 1e-15);
        assert!((p.taper_ratio() - 0.5).abs() < 1e-15);
        assert!((p.thickness_ratio() - 0.02).abs() < 1e-15);
        let with = |planform: FinPlanform, thickness_m: f64| FinSet {
            planform,
            thickness_m,
            ..fins.clone()
        };
        let trapezoid =
            |root_chord_m: f64, tip_chord_m: f64, span_m: f64| FinPlanform::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                sweep_m: 0.0,
            };
        let unsupported = |r: Result<FlutterPanel, SimError>, expected: &str| {
            assert!(
                matches!(r, Err(SimError::Unsupported { what }) if what.contains(expected)),
                "{expected}"
            );
        };
        let elliptical = FinPlanform::Elliptical {
            root_chord_m: 0.2,
            span_m: 0.1,
        };
        unsupported(FlutterPanel::of_fins(&with(elliptical, 0.004)), "trapezoid");
        unsupported(
            FlutterPanel::of_fins(&with(trapezoid(0.1, 0.2, 0.1), 0.004)),
            "tip chord is longer",
        );
        let domain = |r: Result<FlutterPanel, SimError>, expected: &str| {
            assert!(
                matches!(r, Err(SimError::Domain { what, .. }) if what == expected),
                "{expected}"
            );
        };
        domain(
            FlutterPanel::of_fins(&with(trapezoid(0.0, 0.0, 0.1), 0.004)),
            "fin root chord",
        );
        domain(
            FlutterPanel::of_fins(&with(trapezoid(0.2, 0.1, -0.1), 0.004)),
            "flutter panel aspect ratio",
        );
        domain(
            FlutterPanel::of_fins(&with(trapezoid(0.2, 0.1, 0.1), 0.0)),
            "flutter panel thickness ratio",
        );
    }

    #[test]
    fn out_of_range_inputs_are_refused() {
        let refused = |r: Result<FlutterPanel, SimError>, expected: &str| {
            assert!(
                matches!(r, Err(SimError::Domain { what, .. }) if what == expected),
                "{expected}"
            );
        };
        refused(
            FlutterPanel::new(0.0, 0.5, 0.02),
            "flutter panel aspect ratio",
        );
        refused(
            FlutterPanel::new(1.0, 1.2, 0.02),
            "flutter panel taper ratio (0 to 1)",
        );
        refused(
            FlutterPanel::new(1.0, -0.1, 0.02),
            "flutter panel taper ratio (0 to 1)",
        );
        refused(
            FlutterPanel::new(1.0, 0.5, f64::NAN),
            "flutter panel thickness ratio",
        );
        let p = panel(1.0, 0.5, 0.02);
        for (r, expected) in [
            (
                p.flutter_speed_m_s(0.0, P0, 340.0),
                "effective shear modulus",
            ),
            (p.flutter_speed_m_s(1e9, -1.0, 340.0), "static pressure"),
            (p.flutter_speed_m_s(1e9, P0, 0.0), "speed of sound"),
            (p.figure_3_ratio(1e9, f64::INFINITY), "static pressure"),
            (
                p.flutter_dynamic_pressure_pa(f64::NAN),
                "effective shear modulus",
            ),
        ] {
            assert!(
                matches!(r, Err(SimError::Domain { what, .. }) if what == expected),
                "{expected}"
            );
        }
        // JSON goes through the same checks, and round-trips.
        let json = serde_json::to_value(p).unwrap();
        let back: FlutterPanel = serde_json::from_value(json).unwrap();
        assert_eq!(back, p);
        let bad = serde_json::json!({"aspect_ratio": 1.0, "taper_ratio": -1.5,
                                     "thickness_ratio": 0.02});
        let err = serde_json::from_value::<FlutterPanel>(bad).unwrap_err();
        assert!(err.to_string().contains("taper ratio"), "{err}");
        let extra = serde_json::json!({"aspect_ratio": 1.0, "taper_ratio": 0.5,
                                       "thickness_ratio": 0.02, "extra": 1});
        assert!(serde_json::from_value::<FlutterPanel>(extra).is_err());
    }
}
