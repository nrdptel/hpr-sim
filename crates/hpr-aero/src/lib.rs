//! Aerodynamics: Barrowman normal force and centre of pressure with extensions, drag buildup,
//! compressibility, damping and override tables.
//!
//! - [`body`]: nose cones, body tubes and transitions: Barrowman's slope and centre of pressure,
//!   and Galejs's body lift.
//! - [`fins`]: fin sets: Barrowman's slope with Prandtl–Glauert, the mean aerodynamic chord,
//!   fin-count and roll terms, and fin–body interference.
//! - [`model`]: a rocket's terms built from a [`hpr_design::Layout`] and summed at a [`Flow`].
//!
//! Status: M1.5a covers subsonic (`M < 1`) normal force and centre of pressure. Drag and override
//! tables arrive in M1.5b, transonic and supersonic flow in M1.8.
//!
//! Physics: `docs/physics/aero.md`.

pub mod body;
pub mod error;
pub mod fins;
pub mod model;

pub use body::{BODY_LIFT_K, BodyGeometry};
pub use error::AeroError;
pub use fins::{FinGeometry, fin_count_factor, interference_factor, roll_sum, side_sum};
pub use model::{AeroModel, BodyAero, ComponentNormalForce, FinSetAero, Flow, NormalForce};

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use hpr_design::{
        FinPlanform, NoseShape, Overrides, Position, ReferenceDiameter, Rocket, Stage,
    };
    use serde::Deserialize;

    use super::*;
    use crate::testing::{body_part, component, fin_set, nose, one_stage};

    const INCH: f64 = 0.0254;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    fn component_force(model: &AeroModel, id: &str) -> NormalForce {
        model
            .components(&Flow::axial(0.0))
            .unwrap()
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .normal_force
    }

    /// Loft lesson L89: Barrowman hand values. A cone has `C_Nα = 2` and its CP at `2L/3`; a
    /// conical transition from 20 to 40 mm radius over 0.1 m, with a 40 mm reference radius, has
    /// `C_Nα = 1.5` and its CP 0.05556 m aft of its fore end; a boattail back to 20 mm has −1.5 at
    /// the mirrored CP; an elliptical fin's CP is 0.28779 `c_r` aft of its root leading edge.
    #[test]
    fn barrowman_hand_values() {
        let cone = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.025), None),
                component("tube", body_part(0.6, 0.025, 0.025), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let model = AeroModel::new(&cone.layout().unwrap()).unwrap();
        let total = model.normal_force(&Flow::axial(0.0)).unwrap();
        close(total.slope_per_rad, 2.0, 1e-15, "cone slope");
        close(
            total.cp_station_m.unwrap(),
            0.2 * 2.0 / 3.0,
            1e-11,
            "cone CP",
        );
        assert_eq!(component_force(&model, "tube").slope_per_rad, 0.0);
        assert_eq!(component_force(&model, "tube").cp_station_m, None);

        let shoulder = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.1, 0.02), None),
                component("shoulder", body_part(0.1, 0.02, 0.04), None),
                component("tube", body_part(0.5, 0.04, 0.04), None),
                component("boattail", body_part(0.1, 0.04, 0.02), None),
            ],
            ReferenceDiameter::Custom { diameter_m: 0.08 },
        );
        let model = AeroModel::new(&shoulder.layout().unwrap()).unwrap();
        let s = component_force(&model, "shoulder");
        close(s.slope_per_rad, 1.5, 1e-15, "shoulder slope");
        close(s.cp_station_m.unwrap() - 0.1, 0.05556, 1e-4, "shoulder CP");
        close(
            s.cp_station_m.unwrap() - 0.1,
            0.1 / 3.0 * (1.0 + 1.0 / 1.5),
            1e-11,
            "eq. 44",
        );
        let b = component_force(&model, "boattail");
        close(b.slope_per_rad, -1.5, 1e-15, "boattail slope");
        close(
            b.cp_station_m.unwrap() - 0.7,
            0.1 / 3.0 * (1.0 + 1.0 / 3.0),
            1e-11,
            "boattail CP",
        );

        let (c_r, s) = (0.1, 0.06);
        let mut tube = component("tube", body_part(0.6, 0.025, 0.025), None);
        tube.children = vec![component(
            "fins",
            fin_set(
                4,
                FinPlanform::Elliptical {
                    root_chord_m: c_r,
                    span_m: s,
                },
            ),
            Some(Position::Bottom { aft_offset_m: 0.0 }),
        )];
        let finned = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.025), None),
                tube,
            ],
            ReferenceDiameter::Maximum {},
        );
        let layout = finned.layout().unwrap();
        let model = AeroModel::new(&layout).unwrap();
        let root_le = layout.find("fins").unwrap().1.fore_station_m;
        let fins = component_force(&model, "fins");
        close(
            (fins.cp_station_m.unwrap() - root_le) / c_r,
            0.28779,
            2e-5,
            "elliptical fin CP",
        );
        close(
            (fins.cp_station_m.unwrap() - root_le) / c_r,
            0.5 - 2.0 / (3.0 * PI),
            1e-12,
            "elliptical fin CP, exact",
        );
    }

    #[derive(Deserialize)]
    struct Fixture {
        tolerance_rel: f64,
        examples: Vec<Example>,
    }

    #[derive(Deserialize)]
    struct Example {
        id: String,
        reference_diameter_in: f64,
        station_offset_in: f64,
        nose: NoseIn,
        body: Vec<BodyIn>,
        fin_sets: Vec<FinsIn>,
        printed: Vec<Printed>,
    }

    #[derive(Deserialize)]
    struct NoseIn {
        shape: String,
        length_in: f64,
        base_diameter_in: f64,
    }

    #[derive(Deserialize)]
    struct BodyIn {
        id: String,
        #[serde(default)]
        stage: Option<String>,
        length_in: f64,
        fore_diameter_in: f64,
        aft_diameter_in: f64,
    }

    #[derive(Deserialize)]
    struct FinsIn {
        id: String,
        on: String,
        count: u32,
        root_chord_in: f64,
        tip_chord_in: f64,
        span_in: f64,
        sweep_in: f64,
        root_leading_edge_station_in: f64,
    }

    #[derive(Deserialize)]
    struct Printed {
        what: String,
        cn_alpha: f64,
        cp_in: f64,
        #[serde(default)]
        six_fin_rule: bool,
    }

    /// The design of a worked example, and of its first stage alone.
    fn example_rockets(example: &Example) -> (Rocket, Rocket) {
        let offset = example.station_offset_in;
        let shape = match example.nose.shape.as_str() {
            "cone" => NoseShape::Conical {},
            "tangent_ogive" => NoseShape::Ogive { radius_ratio: 1.0 },
            other => panic!("unknown nose shape {other}"),
        };
        let mut stages = vec![Stage {
            id: "sustainer".to_owned(),
            name: String::new(),
            components: vec![component(
                "nose",
                nose(
                    shape,
                    example.nose.length_in * INCH,
                    0.5 * example.nose.base_diameter_in * INCH,
                ),
                None,
            )],
            overrides: Overrides::default(),
        }];
        for b in &example.body {
            let mut part = component(
                &b.id,
                body_part(
                    b.length_in * INCH,
                    0.5 * b.fore_diameter_in * INCH,
                    0.5 * b.aft_diameter_in * INCH,
                ),
                None,
            );
            part.children = example
                .fin_sets
                .iter()
                .filter(|f| f.on == b.id)
                .map(|f| {
                    component(
                        &f.id,
                        fin_set(
                            f.count,
                            FinPlanform::Trapezoidal {
                                root_chord_m: f.root_chord_in * INCH,
                                tip_chord_m: f.tip_chord_in * INCH,
                                span_m: f.span_in * INCH,
                                sweep_m: f.sweep_in * INCH,
                            },
                        ),
                        Some(Position::Absolute {
                            station_m: (f.root_leading_edge_station_in + offset) * INCH,
                        }),
                    )
                })
                .collect();
            match &b.stage {
                Some(stage) if stages.last().is_some_and(|s| &s.id != stage) => {
                    stages.push(Stage {
                        id: stage.clone(),
                        name: String::new(),
                        components: vec![part],
                        overrides: Overrides::default(),
                    });
                }
                _ => stages.last_mut().unwrap().components.push(part),
            }
        }
        let reference = ReferenceDiameter::Custom {
            diameter_m: example.reference_diameter_in * INCH,
        };
        let whole = Rocket {
            name: example.id.clone(),
            stages,
            reference_diameter: reference,
            configurations: Vec::new(),
        };
        let first = Rocket {
            stages: whole.stages[..1].to_vec(),
            ..whole.clone()
        };
        (whole, first)
    }

    /// TIR-33's six-fin rule over hpr's for a six-fin set of span `s` on a body of radius `r`:
    /// `(1 + 0.5 r/(s + r)) / (0.913 (1 + r/(s + r)))` (TIR-33 p. 25; ADR-008).
    fn six_fin_rule_ratio(set: &FinSetAero) -> f64 {
        let r_over = (set.interference - 1.0).max(0.0);
        (1.0 + 0.5 * r_over) / (set.count_factor * set.interference)
    }

    /// M1.5a done-when: `C_Nα` and CP reproduce Barrowman's worked examples within 1%: Testbed II
    /// and the Aerobee 350 (NARAM-8, 1966), and the Javelin, Recruiter and Arcon-Hi (TIR-33,
    /// 1970), component by component and in total. Inputs and printed results are in
    /// `validation/fixtures/aero/barrowman-worked-examples.json`, with page numbers and notes.
    ///
    /// The Recruiter's six-fin slopes follow TIR-33's own six-fin rule. They are checked with that
    /// rule substituted into the slope and the CP weighting; hpr's own values are reported, and
    /// must miss the print by about the rules' difference, which exceeds the tolerance.
    #[test]
    fn barrowman_worked_examples() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/barrowman-worked-examples.json"
        ))
        .unwrap();
        let tolerance = fixture.tolerance_rel;
        assert_eq!(tolerance, 0.01);
        let flow = Flow::axial(0.0);
        let mut worst: f64 = 0.0;
        let mut worst_own = (0.0, String::new());
        let mut outside_own = Vec::new();
        assert_eq!(fixture.examples.len(), 5);
        let printed_values: usize = fixture.examples.iter().map(|e| e.printed.len()).sum();
        assert_eq!(printed_values, 19);
        for example in &fixture.examples {
            let (whole, first) = example_rockets(example);
            let model = AeroModel::new(&whole.layout().unwrap()).unwrap();
            let components = model.components(&flow).unwrap();
            let six_fin: Option<&FinSetAero> = model.fin_sets().iter().find(|s| s.count == 6);
            for printed in &example.printed {
                let force = match printed.what.as_str() {
                    "total" => model.normal_force(&flow).unwrap(),
                    "sustainer-alone" => AeroModel::new(&first.layout().unwrap())
                        .unwrap()
                        .normal_force(&flow)
                        .unwrap(),
                    id => {
                        components
                            .iter()
                            .find(|c| c.id == id)
                            .unwrap_or_else(|| panic!("{}: no component {id}", example.id))
                            .normal_force
                    }
                };
                let mut slope = force.slope_per_rad;
                let mut cp_m = force.cp_station_m.unwrap();
                let own_cp_err = (cp_m / INCH - example.station_offset_in) / printed.cp_in - 1.0;
                let own = (slope / printed.cn_alpha - 1.0).abs().max(own_cp_err.abs());
                if own > worst_own.0 {
                    worst_own = (own, format!("{} {}", example.id, printed.what));
                }
                if own > tolerance {
                    outside_own.push(format!("{} {}", example.id, printed.what));
                }
                if printed.six_fin_rule {
                    let set = six_fin.expect("a six-fin set");
                    let set_slope = set
                        .geometry
                        .single_fin_slope(model.reference_area_m2(), 0.0)
                        .unwrap()
                        * roll_sum(set.count, set.base_angle_rad, 0.0)
                        * set.count_factor
                        * set.interference;
                    let extra = set_slope * (six_fin_rule_ratio(set) - 1.0);
                    let tir33_slope = slope + extra;
                    // The two six-fin rules differ by more than the tolerance, and hpr's own slope
                    // misses the print by about that difference.
                    let rule_gap = slope / tir33_slope - 1.0;
                    let printed_gap = slope / printed.cn_alpha - 1.0;
                    assert!(
                        rule_gap > 0.02 && (printed_gap - rule_gap).abs() < tolerance,
                        "{} {}: rule gap {rule_gap:e}, gap to print {printed_gap:e}",
                        example.id,
                        printed.what
                    );
                    eprintln!(
                        "{} {}: hpr C_Nα {slope:.4} is {:+.2}% from the print; \
                         TIR-33's six-fin rule alone accounts for {:+.2}%",
                        example.id,
                        printed.what,
                        100.0 * printed_gap,
                        100.0 * rule_gap
                    );
                    eprintln!(
                        "{} {}: with hpr's rule, CP {:.4} in",
                        example.id,
                        printed.what,
                        cp_m / INCH - example.station_offset_in
                    );
                    cp_m = (cp_m * slope + extra * set.cp_station_m) / tir33_slope;
                    slope = tir33_slope;
                }
                let cp_in = cp_m / INCH - example.station_offset_in;
                let slope_err = slope / printed.cn_alpha - 1.0;
                let cp_err = cp_in / printed.cp_in - 1.0;
                eprintln!(
                    "{} {}: C_Nα {slope:.4} vs {} ({:+.3}%), CP {cp_in:.4} in vs {} ({:+.3}%)",
                    example.id,
                    printed.what,
                    printed.cn_alpha,
                    100.0 * slope_err,
                    printed.cp_in,
                    100.0 * cp_err
                );
                assert!(
                    slope_err.abs() <= tolerance && cp_err.abs() <= tolerance,
                    "{} {}: C_Nα error {slope_err:e}, CP error {cp_err:e}",
                    example.id,
                    printed.what
                );
                worst = worst.max(slope_err.abs()).max(cp_err.abs());
            }
        }
        eprintln!(
            "worst relative error, hpr's own model: {:.2}% ({}); with TIR-33's six-fin rule for \
             the Recruiter: {:.3}%",
            100.0 * worst_own.0,
            worst_own.1,
            100.0 * worst
        );
        // With hpr's own six-fin rule (ADR-008), exactly the Recruiter's six-fin slopes miss 1%.
        assert_eq!(outside_own, ["recruiter fins", "recruiter total"]);
    }
}
