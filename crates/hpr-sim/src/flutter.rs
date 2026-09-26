//! Fin flutter: the speed at which a fin's bending and twisting couple and grow, from D. J.
//! Martin's criterion ([the fin-flutter milestone][m1-10b]; the guide's [Fin flutter][page] page).
//!
//! **Source.** D. J. Martin, *Summary of Flutter Experiences as a Guide to the Preliminary Design
//! of Lifting Surfaces on Missiles*, NACA TN 4197, 1958, appendix, eqs. 16 to 19, pp. 14–15. The
//! criterion is empirical: Martin reduces Theodorsen and Garrick's flutter speed for a
//! bending-torsion wing (eq. 1) to a few planform numbers and sets its constant from missile and
//! wind-tunnel flights (his figure 3). With `G_E` the fin's effective shear modulus, `A` the panel
//! aspect ratio (span over mid-span chord), `λ` the taper ratio (tip over root chord), `t/c` the
//! thickness ratio, `p` the static pressure and `a` the speed of sound, eq. 16 with his
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
//! Loft wrote the constant as `1.337 · (λ + 1)/2` psi, half of `39.3/14.696 = 2.674`, so its
//! flutter speed was `√2` too high, on the unsafe side ([Loft lesson L32][l32]).
//!
//! **A flutter dynamic pressure.** Since `ρ a² = 2q/M²` for any gas, eq. 16 fixes the dynamic
//! pressure at flutter, whatever the height:
//!
//! ```text
//! q_f = ½ ρ V_f² = π G_E / (24 ε X (λ + 1)),   X = A³ / ((t/c)³ (A + 2))
//! ```
//!
//! and a fin flying at dynamic pressure `q` is below its flutter speed by the ratio
//! `V_f / V = √(q_f / q)`. The least ratio of a flight is therefore at its peak dynamic pressure,
//! which [`crate::metrics::FlightMetrics`] finds on the dense output.
//!
//! **Readings.** A trapezoidal fin's `A` is `2s/(c_r + c_t)` (span `s` over the mid-span chord) and
//! `t/c` is the thickness over the root chord: Martin's `c` is the root chord of his
//! constant-thickness-ratio wing, and for a flat fin of constant thickness the root's ratio is the
//! smallest, so the flutter speed the least. `G_E` is Martin's effective shear modulus: for a
//! solid wing he takes the material's own (his p. 6), which this module does. His definition,
//! `G_E = 6 J G / (c t³)` (eq. 12), would give a flat plate, whose torsion constant is
//! `J = c t³/3`, about twice that, and a flutter speed `√2` higher; the lower reading is kept.
//!
//! **Left out.** Sweep, the fin's mounting and the body's own modes (Martin's figure 8), stall
//! flutter at high angles of attack, and every other flutter type Martin lists. The criterion
//! separates his figure 3's safe and failed wings with a band of scatter, not a sharp line; it is a
//! screening number, not a flutter analysis.
//!
//! [l32]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l32
//! [m1-10b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-10b
//! [page]: https://nrdptel.github.io/hpr-sim/physics/flutter.html

use std::f64::consts::PI;

use hpr_design::{FinPlanform, FinSet};
use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::metrics::FlightSummary;

/// Martin's `ε`, the section's centre of mass behind its quarter chord as a fraction of the chord,
/// assumed 0.25 (NACA TN 4197, p. 14).
pub const EPSILON: f64 = 0.25;

/// Martin's ratio of specific heats for air, 1.4 (NACA TN 4197, eq. 17, p. 14).
pub const HEAT_CAPACITY_RATIO: f64 = 1.4;

/// The planform numbers Martin's criterion takes from one fin.
///
/// A birch-plywood fin with a 200 mm root, a 100 mm tip, a 120 mm span and 4 mm thick has
/// `A = 0.8`, `λ = 0.5` and `t/c = 0.02`, so `X = 0.8³ / (0.02³ · 2.8) = 22 857`. With plywood's
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlutterPanel {
    /// The panel aspect ratio `A`: the span over the chord at mid-span.
    pub aspect_ratio: f64,
    /// The taper ratio `λ`: the tip chord over the root chord, from 0 (pointed) to 1.
    pub taper_ratio: f64,
    /// The thickness ratio `t/c`: the thickness over the root chord.
    pub thickness_ratio: f64,
}

impl FlutterPanel {
    /// A panel of aspect ratio `A`, taper ratio `λ` and thickness ratio `t/c`.
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
    /// [`SimError::Unsupported`] for an elliptical or freeform planform, whose taper Martin's
    /// factors don't define; [`SimError::Domain`] for a root chord that isn't positive, or the
    /// trapezoid's numbers out of range, as [`FlutterPanel::new`].
    pub fn of_fins(fins: &FinSet) -> Result<Self, SimError> {
        match fins.planform {
            FinPlanform::Trapezoidal {
                root_chord_m,
                tip_chord_m,
                span_m,
                ..
            } => {
                positive("fin root chord", root_chord_m)?;
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

    /// Martin's `X` without its constant, `A³ / ((t/c)³ (A + 2))` (NACA TN 4197, eq. 19).
    #[must_use]
    pub fn shape_factor(&self) -> f64 {
        let a = self.aspect_ratio;
        a.powi(3) / (self.thickness_ratio.powi(3) * (a + 2.0))
    }

    /// The denominator of eq. 18 at static pressure `p`, Pa:
    /// `D = (24 ε γ / π) p · A³ / ((t/c)³ (A + 2)) · (λ + 1)/2`.
    #[must_use]
    pub fn denominator_pa(&self, pressure_pa: f64) -> f64 {
        24.0 * EPSILON * HEAT_CAPACITY_RATIO / PI
            * pressure_pa
            * self.shape_factor()
            * (self.taper_ratio + 1.0)
            / 2.0
    }

    /// The dynamic pressure at which a fin of effective shear modulus `G_E` flutters, Pa:
    /// `q_f = π G_E / (24 ε X (λ + 1))`, the same at every height.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if `G_E` isn't positive and finite.
    pub fn flutter_dynamic_pressure_pa(&self, shear_modulus_pa: f64) -> Result<f64, SimError> {
        positive("effective shear modulus", shear_modulus_pa)?;
        Ok(PI * shear_modulus_pa
            / (24.0 * EPSILON * self.shape_factor() * (self.taper_ratio + 1.0)))
    }

    /// The flutter speed in air of static pressure `p` and speed of sound `a`, m/s:
    /// `V_f = a √(G_E / D)` (eq. 18).
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
        positive("effective shear modulus", shear_modulus_pa)?;
        positive("static pressure", pressure_pa)?;
        positive("speed of sound", sound_speed_m_s)?;
        Ok(sound_speed_m_s * (shear_modulus_pa / self.denominator_pa(pressure_pa)).sqrt())
    }

    /// The fin's least flutter margin over the flight `summary` describes, at its peak dynamic
    /// pressure; `None` if the rocket never flew.
    ///
    /// The whole flight's peak is used for every fin set on it: for a booster's fins, which leave
    /// at the separation, it can be later and higher than any they saw, so their margin is at most
    /// the one given.
    ///
    /// # Errors
    ///
    /// As [`FlutterPanel::flutter_dynamic_pressure_pa`].
    pub fn margin(
        &self,
        shear_modulus_pa: f64,
        summary: &FlightSummary,
    ) -> Result<Option<FlutterMargin>, SimError> {
        let flutter_dynamic_pressure_pa = self.flutter_dynamic_pressure_pa(shear_modulus_pa)?;
        Ok(summary
            .max_dynamic_pressure_pa
            .filter(|peak| peak.value > 0.0)
            .map(|peak| FlutterMargin {
                time_s: peak.time_s,
                height_above_ground_m: peak.height_above_ground_m,
                dynamic_pressure_pa: peak.value,
                flutter_dynamic_pressure_pa,
                speed_ratio: (flutter_dynamic_pressure_pa / peak.value).sqrt(),
            }))
    }
}

/// How far below its flutter speed a fin flew, at the flight's peak dynamic pressure.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlutterMargin {
    /// When the peak came, s.
    pub time_s: f64,
    /// The centre of mass's height above the launch site then, m.
    pub height_above_ground_m: f64,
    /// The flight's peak dynamic pressure, Pa.
    pub dynamic_pressure_pa: f64,
    /// The dynamic pressure at which the fin flutters, Pa.
    pub flutter_dynamic_pressure_pa: f64,
    /// The flutter speed over the airspeed there, `V_f / V = √(q_f / q)`: above 1 the fin is below
    /// its flutter speed, and the flight's least ratio is this one.
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
    use super::*;
    use crate::environment::Environment;
    use crate::flight::{FlightSettings, Simulation};
    use crate::metrics::FlightMetrics;
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

    /// Eq. 18's constant is eq. 16's `24 ε γ p₀ / π` to the three figures Martin prints, and
    /// twice Loft's `1.337` (L32).
    #[test]
    fn flutter_denominator_matches_tn_4197_eq_18() {
        let constant_psi = 24.0 * EPSILON * HEAT_CAPACITY_RATIO / PI * P0 / PSI;
        assert!((constant_psi - 39.3).abs() < 0.05, "{constant_psi}");
        for (a, lambda, tc) in [(2.0, 1.0, 0.04), (1.3, 0.4, 0.025), (3.1, 0.0, 0.07)] {
            let p = panel(a, lambda, tc);
            for pressure in [P0, 0.4 * P0] {
                let eq_18_psi = 39.3 * a.powi(3) / (tc.powi(3) * (a + 2.0)) * (lambda + 1.0) / 2.0
                    * (pressure / P0);
                let ours_psi = p.denominator_pa(pressure) / PSI;
                // Only Martin's rounding of 39.29 to 39.3 apart.
                assert!(
                    (ours_psi / eq_18_psi - 1.0).abs() < 0.05 / 39.3,
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
    #[test]
    fn martins_worked_examples() {
        let x_psi = |a: f64, tc: f64| panel(a, 1.0, tc).denominator_pa(P0) / PSI;
        let x = x_psi(2.0, 0.04);
        assert!((x / 1e6 - 1.227_9).abs() < 1e-4, "{x}");
        assert_eq!((x / 0.05e6).round() * 0.05, 1.25);
        for (a, printed_percent) in [(1.0, 2.5), (2.0, 4.5), (3.0, 6.5)] {
            // X ∝ (t/c)⁻³: the thickness that brings X to 0.8 × 10⁶ psi.
            let tc = 0.01 * (x_psi(a, 0.01) / 0.8e6).cbrt();
            assert_eq!((200.0 * tc).round() / 2.0, printed_percent, "{a}: {tc}");
        }
    }

    /// Martin replaces `1/(f₁² f₂²)`, with `f₁ = 1 + 1.87 (1 − λ)^1.6` (eq. 8) and
    /// `f₂ = (1 + 3λ)/(2(1 + λ))` (eq. 14), by `(λ + 1)/2`: equal at `λ = 1`, 3% apart at `λ = 0`,
    /// and up to 47% larger between (at `λ ≈ 0.31`), which lowers the flutter speed there by up to
    /// 17%. The model keeps his form, since his figure 3 was drawn with it.
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
    /// millisecond, and eq. 18 at each instant's own pressure and speed of sound gives the same
    /// ratio: `p = 2q/(γM²)` and `a = V/M` in the simulator's air, whose `γ` is Martin's 1.4.
    #[test]
    fn the_margin_is_the_flights_least_at_its_peak_dynamic_pressure() {
        fn find_fins(value: &serde_json::Value) -> Option<&serde_json::Value> {
            match value {
                serde_json::Value::Object(map) => map
                    .get("fin_set")
                    .or_else(|| map.values().find_map(find_fins)),
                serde_json::Value::Array(items) => items.iter().find_map(find_fins),
                _ => None,
            }
        }
        let text = include_str!("../../../validation/designs/rocketpy-valetudo.json");
        let value: serde_json::Value = serde_json::from_str(text).unwrap();
        let fins: FinSet = serde_json::from_value(find_fins(&value).unwrap().clone()).unwrap();
        let panel = FlutterPanel::of_fins(&fins).unwrap();
        // An arbitrary shear modulus: the checks hold for any.
        let g = 3e9;
        let sim = Simulation::new(
            &design("rocketpy-valetudo"),
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
        assert!((p.aspect_ratio - 0.8).abs() < 1e-15);
        assert!((p.taper_ratio - 0.5).abs() < 1e-15);
        assert!((p.thickness_ratio - 0.02).abs() < 1e-15);
        let elliptical = FinSet {
            planform: FinPlanform::Elliptical {
                root_chord_m: 0.2,
                span_m: 0.1,
            },
            ..fins
        };
        assert!(matches!(
            FlutterPanel::of_fins(&elliptical),
            Err(SimError::Unsupported { what }) if what.contains("trapezoid")
        ));
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
        ] {
            assert!(
                matches!(r, Err(SimError::Domain { what, .. }) if what == expected),
                "{expected}"
            );
        }
    }
}
