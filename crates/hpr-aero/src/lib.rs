//! Aerodynamics: Barrowman normal force and centre of pressure with extensions, drag buildup,
//! compressibility and override tables.
//!
//! **Guide:** [Aerodynamics][guide-aero]: the models, their sources, how well they are validated
//! and what they leave out.
//!
//! [guide-aero]: https://nrdptel.github.io/hpr-sim/physics/aero.html
//! [guide-cp]: https://nrdptel.github.io/hpr-sim/physics/aero.html#your-rockets-centre-of-pressure
//! [guide-flight]: https://nrdptel.github.io/hpr-sim/physics/flight.html#aerodynamics-in-flight
//! [guide-fins-mach]: https://nrdptel.github.io/hpr-sim/physics/aero.html#fins-through-mach-1
//! [guide-drag-mach]: https://nrdptel.github.io/hpr-sim/physics/aero.html#drag-through-mach-1
//! [m1-8]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8
//!
//! - [`body`]: nose cones, body tubes and transitions: Barrowman's slope and centre of pressure,
//!   and Galejs's body lift.
//! - [`fins`]: fin sets: Barrowman's slope with Prandtl–Glauert, the mean aerodynamic chord,
//!   supersonic linear theory and the transonic join between them, fin-count and roll terms, and
//!   fin–body interference.
//! - [`drag`]: the terms of Niskanen's zero-lift drag buildup, and axial drag at an angle of
//!   attack.
//! - [`nose_drag`]: the pressure drag of noses, shoulders and steps from rest through Mach 1 to
//!   supersonic speeds, with Stoney's measured curves.
//! - [`table`]: drag override tables, the drag coefficient against Mach number from another tool.
//! - [`model`]: a rocket's terms built from a [`hpr_design::Layout`] and summed at a [`Flow`].
//!
//! A rocket's centre of pressure is [`NormalForce::cp_station_m`], in metres aft of the nose tip,
//! from [`AeroModel::normal_force`] at [`Flow::axial`] ([Your rocket's centre of
//! pressure][guide-cp] in the guide).
//!
//! Status: the normal force and centre of pressure from Mach 0 to 5 (fins through the transonic
//! region to supersonic linear theory, [Fins through Mach 1][guide-fins-mach]); the drag buildup
//! from Mach 0 to 5 (noses, shoulders and steps through Mach 1 by Niskanen's appendix B,
//! [Drag through Mach 1][guide-drag-mach]); drag override tables at any Mach number. The crate has
//! no damping coefficients.
//!
//! - Pitch and yaw damping in a flight come only from the flight engine (`hpr_sim`) evaluating
//!   each component in its own local flow, which includes the speed the rocket's rotation adds
//!   there ([Rigid-body flight][guide-flight] in the guide).
//! - Only components with a normal-force slope give that damping: nose cones, transitions and fin
//!   sets. Body tubes give none at small angles: their own slope is 0, and their body lift grows
//!   with `sin² α`.
//! - Damping coefficients for pitch, yaw and roll, and roll forcing from canted fins are planned
//!   for [M1.8][m1-8], the second aerodynamics milestone. For pitch and yaw, those coefficients will have to replace the local-flow damping,
//!   not add to it.

pub mod body;
pub mod drag;
pub mod error;
pub mod fins;
pub mod model;
pub mod nose_drag;
pub mod table;

pub use body::{BODY_LIFT_K, BodyGeometry};
pub use drag::{ComponentDrag, ComponentDragTerms, Drag, DragConditions, PressureDragTerm};
pub use error::AeroError;
pub use fins::{
    FinAero, FinGeometry, FinLoading, FinOutline, fin_count_factor, interference_factor, roll_sum,
    side_sum,
};
pub use model::{
    AeroModel, BodyAero, ComponentNormalForce, FinSetAero, Flow, NORMAL_FORCE_MACH_LIMIT,
    NormalForce,
};
pub use nose_drag::{PressureDragCurve, StoneyNose};
pub use table::{DragTable, parse_mach_csv};

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
                        .fin
                        .geometry()
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
                    cp_m = (cp_m * slope + extra * set.cp_station_m(0.0).unwrap()) / tir33_slope;
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

    #[derive(Deserialize)]
    struct DragCurves {
        mach: f64,
        reynolds_per_m: f64,
        tolerance_rel: f64,
        cases: Vec<DragCurveCase>,
    }

    #[derive(Deserialize)]
    struct DragCurveCase {
        id: String,
        design: String,
        variant_of: Option<String>,
        thrusting: bool,
        curve_cd0: f64,
        hpr_cd0: f64,
        relative_error: f64,
    }

    fn committed_design(name: &str) -> Rocket {
        let text = match name {
            "rocketpy-calisto-tests-motor-at-minus-1.373.json" => include_str!(
                "../../../validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
            ),
            "rocketpy-calisto-getting-started-motor-at-minus-1.255.json" => include_str!(
                "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
            ),
            "rocketpy-juno-iii.json" => {
                include_str!("../../../validation/designs/rocketpy-juno-iii.json")
            }
            "rocketpy-valetudo.json" => {
                include_str!("../../../validation/designs/rocketpy-valetudo.json")
            }
            "rocketpy-cavour.json" => {
                include_str!("../../../validation/designs/rocketpy-cavour.json")
            }
            "wind-tunnel-arcas-robin-short.json" => {
                include_str!("../../../validation/designs/wind-tunnel-arcas-robin-short.json")
            }
            "wind-tunnel-arcas-robin-long.json" => {
                include_str!("../../../validation/designs/wind-tunnel-arcas-robin-long.json")
            }
            other => panic!("no committed design {other}"),
        };
        serde_json::from_str(text).unwrap()
    }

    /// M1.5b done-when: subsonic `C_D0` of RocketPy's example rockets against the drag curves
    /// that ship with them, at Mach 0.3 and USSA76 sea level (RASAero II computes its exports'
    /// Reynolds numbers at sea level). The curves have unclear terms and stay in
    /// `refs/`; `cargo xtask aero` writes the relative errors to
    /// `validation/fixtures/aero/rocketpy-drag-curves.json`. This test recomputes hpr's values
    /// from the committed designs, so the recorded errors can't go stale, and checks them against
    /// the tolerance.
    #[test]
    fn rocketpy_drag_curves_at_mach_0_3() {
        let fixture: DragCurves = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/rocketpy-drag-curves.json"
        ))
        .unwrap();
        assert_eq!(fixture.mach, 0.3);
        assert_eq!(fixture.tolerance_rel, 0.10);
        let air = hpr_atmos::Ussa76::standard().sample(0.0).unwrap().air;
        close(
            fixture.reynolds_per_m,
            0.3 * air.speed_of_sound_m_s / air.kinematic_viscosity_m2_s(),
            1e-12,
            "sea-level Reynolds number per metre",
        );
        let mut outside = Vec::new();
        for case in &fixture.cases {
            let rocket = committed_design(&case.design);
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
            let conditions = if case.thrusting {
                DragConditions::thrusting(fixture.reynolds_per_m, motor_area)
            } else {
                DragConditions::coasting(fixture.reynolds_per_m)
            };
            let drag = model.drag(&Flow::axial(fixture.mach), &conditions).unwrap();
            // A stale fixture: rerun `cargo xtask aero`.
            close(drag.zero_lift_coefficient, case.hpr_cd0, 1e-12, &case.id);
            let error = drag.zero_lift_coefficient / case.curve_cd0 - 1.0;
            assert!(
                (error - case.relative_error).abs() < 1e-12,
                "{}: recorded error {}, recomputed {error}",
                case.id,
                case.relative_error
            );
            eprintln!(
                "{}: C_D0 {:.4} ({:.4} friction, {:.4} pressure, {:.4} base, {:.4} parasitic), \
                 {:+.1}% from the curve",
                case.id,
                drag.zero_lift_coefficient,
                drag.friction,
                drag.pressure,
                drag.base,
                drag.parasitic,
                100.0 * case.relative_error
            );
            if error.abs() > fixture.tolerance_rel {
                outside.push(case.id.as_str());
            }
        }
        // Six comparisons over four rockets, and one variant (Calisto's getting-started fins on
        // the same export).
        assert_eq!(fixture.cases.len(), 7);
        let variants: Vec<&str> = fixture
            .cases
            .iter()
            .filter(|c| c.variant_of.is_some())
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(variants, ["calisto-getting-started-power-off"]);
        // Outside the tolerance (ADR-009): Cavour under power, where the table carries at most
        // 0.001 of relief at Mach 0.3 and hpr's depends on the unrecorded motor diameter; and
        // Valetudo's table, 1.44 times the OpenRocket export for the same rocket, which hpr
        // matches to 2% with that file's inputs.
        assert_eq!(
            outside,
            ["cavour-power-on", "valetudo-power-off", "valetudo-power-on"]
        );
    }

    #[derive(Deserialize)]
    struct NormalForceVsMach {
        targets: Targets,
        references: Vec<NormalForceReference>,
    }

    #[derive(Deserialize)]
    struct Targets {
        cp_calibers: f64,
        cn_alpha_rel: f64,
    }

    #[derive(Deserialize)]
    struct NormalForceReference {
        id: String,
        design: String,
        reference_diameter_m: f64,
        rows: Vec<NormalForceRow>,
    }

    #[derive(Deserialize)]
    struct NormalForceRow {
        mach: f64,
        reference_cn_alpha_per_rad: f64,
        reference_cp_m: f64,
        hpr_cn_alpha_per_rad: f64,
        hpr_cp_m: f64,
        cn_alpha_error: f64,
        cp_error_calibers: f64,
        within_targets: bool,
        reference_body_cn_alpha_per_rad: Option<f64>,
        hpr_body_cn_alpha_per_rad: Option<f64>,
    }

    #[derive(Deserialize)]
    struct WindTunnel {
        configurations: Vec<WindTunnelConfiguration>,
    }

    #[derive(Deserialize)]
    struct WindTunnelConfiguration {
        id: String,
        length_m: f64,
        cn_alpha: Vec<WindTunnelCurve>,
        cn_alpha_fins_off: Vec<WindTunnelCurve>,
        cp: Vec<WindTunnelCp>,
    }

    #[derive(Deserialize)]
    struct WindTunnelCurve {
        mach: f64,
        alpha_deg_c_n: Vec<[f64; 2]>,
    }

    #[derive(Deserialize)]
    struct WindTunnelCp {
        mach: f64,
        percent_length: f64,
    }

    /// The least-squares slope of `ys` against `xs`, with an intercept.
    fn lsq_slope(xs: &[f64], ys: &[f64]) -> f64 {
        let n = xs.len() as f64;
        let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
        let sxy: f64 = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum();
        let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
        sxy / sxx
    }

    /// hpr's `C_N` and its moment about the nose tip at `alpha_deg`, odd in the angle, of the
    /// whole rocket or of its bodies alone.
    fn force_at(model: &AeroModel, mach: f64, alpha_deg: f64, bodies_only: bool) -> (f64, f64) {
        let parts = model
            .components(&Flow::new(mach, alpha_deg.abs().to_radians(), 0.0))
            .unwrap();
        let kept = if bodies_only {
            &parts[..model.bodies().len()]
        } else {
            &parts[..]
        };
        let sign = alpha_deg.signum();
        (
            sign * kept.iter().map(|p| p.normal_force.coefficient).sum::<f64>(),
            sign * kept.iter().map(|p| p.normal_force.moment_m).sum::<f64>(),
        )
    }

    /// M1.8a done-when: hpr's `C_Nα` and CP against Mach, against RASAero II's Calisto export
    /// (Mach 0.1–2.0; hpr's small-angle values) and NASA's Arcas Robin wind-tunnel models
    /// (TN D-4013 and TN D-4014, Mach 0.6–4.63; hpr fitted as the plots are). `cargo xtask aero`
    /// writes `validation/fixtures/aero/normal-force-vs-mach.json`; this test recomputes every
    /// hpr value from the committed designs, and every wind-tunnel slope from the committed
    /// points, so the recorded errors can't go stale, and pins the set of rows outside the
    /// targets set before measuring (CP within 0.5 calibers, `C_Nα` within 15%). Each miss is
    /// explained in `docs/physics/aero.md` and ADR-027.
    #[test]
    fn normal_force_against_mach() {
        let fixture: NormalForceVsMach = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/normal-force-vs-mach.json"
        ))
        .unwrap();
        let tunnel: WindTunnel = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/arcas-robin-wind-tunnel.json"
        ))
        .unwrap();
        assert_eq!(fixture.targets.cp_calibers, 0.5);
        assert_eq!(fixture.targets.cn_alpha_rel, 0.15);
        let cp_angles: Vec<f64> = [-2.0_f64, -1.0, 0.0, 1.0, 2.0]
            .iter()
            .map(|a| a.to_radians())
            .collect();
        let mut misses = Vec::new();
        for reference in &fixture.references {
            let model =
                AeroModel::new(&committed_design(&reference.design).layout().unwrap()).unwrap();
            let d = (4.0 * model.reference_area_m2() / PI).sqrt();
            close(reference.reference_diameter_m, d, 1e-15, &reference.id);
            let configuration = tunnel.configurations.iter().find(|c| c.id == reference.id);
            for row in &reference.rows {
                let what = format!("{} at Mach {}", reference.id, row.mach);
                let (cn_alpha, cp_m, hpr_cn_alpha, hpr_cp_m) = match configuration {
                    // RASAero II's export: hpr's small-angle slope and CP.
                    None => {
                        let force = model.normal_force(&Flow::axial(row.mach)).unwrap();
                        (
                            row.reference_cn_alpha_per_rad,
                            row.reference_cp_m,
                            force.slope_per_rad,
                            force.cp_station_m.unwrap(),
                        )
                    }
                    // The wind tunnel: both slopes fitted at the plotted angles, the CP over
                    // -2 to 2 degrees.
                    Some(configuration) => {
                        let fit = |curves: &[WindTunnelCurve], bodies_only: bool| {
                            let curve = curves.iter().find(|c| c.mach == row.mach).unwrap();
                            let alphas: Vec<f64> = curve
                                .alpha_deg_c_n
                                .iter()
                                .map(|p| p[0].to_radians())
                                .collect();
                            let measured: Vec<f64> =
                                curve.alpha_deg_c_n.iter().map(|p| p[1]).collect();
                            let hpr: Vec<f64> = curve
                                .alpha_deg_c_n
                                .iter()
                                .map(|p| force_at(&model, row.mach, p[0], bodies_only).0)
                                .collect();
                            (lsq_slope(&alphas, &measured), lsq_slope(&alphas, &hpr))
                        };
                        let (measured, hpr) = fit(&configuration.cn_alpha, false);
                        if let Some(curve_body) = row.reference_body_cn_alpha_per_rad {
                            let (body, hpr_body) = fit(&configuration.cn_alpha_fins_off, true);
                            close(curve_body, body, 1e-12, &format!("{what}: body"));
                            close(
                                row.hpr_body_cn_alpha_per_rad.unwrap(),
                                hpr_body,
                                1e-12,
                                &format!("{what}: hpr's body"),
                            );
                        }
                        let cp = configuration
                            .cp
                            .iter()
                            .find(|c| c.mach == row.mach)
                            .unwrap();
                        let forces: Vec<(f64, f64)> = [-2.0, -1.0, 0.0, 1.0, 2.0]
                            .iter()
                            .map(|&a| force_at(&model, row.mach, a, false))
                            .collect();
                        let normal: Vec<f64> = forces.iter().map(|f| f.0).collect();
                        let moment: Vec<f64> = forces.iter().map(|f| f.1).collect();
                        (
                            measured,
                            0.01 * cp.percent_length * configuration.length_m,
                            hpr,
                            lsq_slope(&cp_angles, &moment) / lsq_slope(&cp_angles, &normal),
                        )
                    }
                };
                // A stale fixture: rerun `cargo xtask aero`.
                close(row.reference_cn_alpha_per_rad, cn_alpha, 1e-12, &what);
                close(row.reference_cp_m, cp_m, 1e-12, &what);
                close(row.hpr_cn_alpha_per_rad, hpr_cn_alpha, 1e-12, &what);
                close(row.hpr_cp_m, hpr_cp_m, 1e-12, &what);
                let cn_error = hpr_cn_alpha / cn_alpha - 1.0;
                let cp_error = (hpr_cp_m - cp_m) / d;
                assert!((row.cn_alpha_error - cn_error).abs() < 1e-12, "{what}");
                assert!((row.cp_error_calibers - cp_error).abs() < 1e-12, "{what}");
                let within = cn_error.abs() <= 0.15 && cp_error.abs() <= 0.5;
                assert_eq!(row.within_targets, within, "{what}");
                eprintln!(
                    "{what}: C_Na {hpr_cn_alpha:.3} against {cn_alpha:.3} ({:+.1}%), CP {:+.2} \
                     calibers",
                    100.0 * cn_error,
                    cp_error
                );
                if !within {
                    misses.push(format!("{}@{}", reference.id, row.mach));
                }
            }
        }
        assert_eq!(fixture.references.len(), 3);
        assert_eq!(
            misses,
            [
                // Calisto against RASAero II: hpr's Prandtl-Glauert rise and aft CP near Mach 1,
                // which RASAero II's constant subsonic slope doesn't have, the linear join's peak
                // at M_s = 1.28, and at Mach 2 a gap that no fins-off data can split (the wind
                // tunnel's points to the body).
                "calisto-rasaero-ii@0.8",
                "calisto-rasaero-ii@0.9",
                "calisto-rasaero-ii@0.95",
                "calisto-rasaero-ii@1.3",
                "calisto-rasaero-ii@2",
                // The Arcas Robin: the measured transonic dip in the fins' lift and the join's
                // peak (Mach 0.8 to 1.2); the body, which grows with Mach where slender-body
                // theory's doesn't (Mach 3.96 and 4.63).
                "arcas-robin-short@0.8",
                "arcas-robin-short@0.9",
                "arcas-robin-short@0.95",
                "arcas-robin-short@1.2",
                "arcas-robin-short@3.96",
                "arcas-robin-short@4.63",
                "arcas-robin-long@0.9",
                "arcas-robin-long@1",
                "arcas-robin-long@1.2",
                "arcas-robin-long@3.96",
                "arcas-robin-long@4.63",
            ]
        );
    }

    #[derive(Deserialize)]
    struct DragVsMach {
        target_rel: f64,
        reynolds_per_m: f64,
        references: Vec<DragReference>,
    }

    #[derive(Deserialize)]
    struct DragReference {
        id: String,
        design: String,
        rows: Vec<DragRow>,
    }

    #[derive(Deserialize)]
    struct DragRow {
        mach: f64,
        fins: String,
        reference_forebody_c_a: f64,
        hpr_forebody_c_d: f64,
        hpr_base: f64,
        error: f64,
        within_target: bool,
    }

    #[derive(Deserialize)]
    struct AxialTunnel {
        configurations: Vec<AxialConfiguration>,
    }

    #[derive(Deserialize)]
    struct AxialConfiguration {
        id: String,
        design: String,
        axial_force: Vec<AxialPoint>,
    }

    #[derive(Deserialize)]
    struct AxialPoint {
        mach: f64,
        fins: String,
        forebody_c_a: f64,
    }

    /// M1.8b1 done-when: hpr's forebody drag (`C_D0` less the base drag) against the Arcas Robin
    /// wind-tunnel models' forebody axial force at every Mach number the reports give, fins on
    /// and off (TN D-4013's `C_A,corr`, Mach 0.6–1.2; TN D-4014's `C_A` less its chamber force,
    /// Mach 1.5–4.63). `cargo xtask aero` writes `validation/fixtures/aero/drag-vs-mach.json`;
    /// this test recomputes every hpr value from the committed designs at the tunnels' Reynolds
    /// number, checks every measured point is compared, and pins the rows within the 10% target
    /// set before measuring, so every other row is a pinned miss (36 of 44). Each miss is
    /// explained in `docs/physics/aero.md` and ADR-028.
    #[test]
    fn drag_against_mach() {
        let fixture: DragVsMach = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/drag-vs-mach.json"
        ))
        .unwrap();
        let tunnel: AxialTunnel = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/arcas-robin-wind-tunnel.json"
        ))
        .unwrap();
        assert_eq!(fixture.target_rel, 0.10);
        assert_eq!(fixture.reynolds_per_m, 3.0e6 / 0.3048);
        let conditions = DragConditions::coasting(fixture.reynolds_per_m);
        let (mut within, mut rows) = (Vec::new(), 0);
        assert_eq!(fixture.references.len(), tunnel.configurations.len());
        for (reference, configuration) in fixture.references.iter().zip(&tunnel.configurations) {
            assert_eq!(reference.id, configuration.id);
            assert_eq!(reference.design, configuration.design);
            assert_eq!(
                reference.rows.len(),
                configuration.axial_force.len(),
                "{}: every measured point is compared",
                reference.id
            );
            let model =
                AeroModel::new(&committed_design(&reference.design).layout().unwrap()).unwrap();
            let fin_ids: Vec<&str> = model.fin_sets().iter().map(|f| f.id.as_str()).collect();
            for (row, point) in reference.rows.iter().zip(&configuration.axial_force) {
                let what = format!("{}@{} fins {}", reference.id, row.mach, row.fins);
                assert_eq!((row.mach, &row.fins), (point.mach, &point.fins), "{what}");
                assert_eq!(row.reference_forebody_c_a, point.forebody_c_a, "{what}");
                let parts = model
                    .buildup_components(&Flow::axial(row.mach), &conditions)
                    .unwrap();
                let kept = parts
                    .iter()
                    .filter(|p| row.fins == "on" || !fin_ids.contains(&p.id.as_str()));
                let (mut forebody, mut base) = (0.0, 0.0);
                for part in kept {
                    forebody += part.drag.friction + part.drag.pressure + part.drag.parasitic;
                    base += part.drag.base;
                }
                close(row.hpr_forebody_c_d, forebody, 1e-12, &what);
                close(row.hpr_base, base, 1e-12, &what);
                let error = forebody / point.forebody_c_a - 1.0;
                assert!((row.error - error).abs() < 1e-12, "{what}");
                assert_eq!(
                    row.within_target,
                    error.abs() <= fixture.target_rel,
                    "{what}"
                );
                rows += 1;
                if row.within_target {
                    within.push(what);
                }
            }
        }
        assert_eq!(rows, 44);
        assert_eq!(
            within,
            [
                "arcas-robin-short@0.95 fins on",
                "arcas-robin-short@1 fins off",
                "arcas-robin-short@1.2 fins on",
                "arcas-robin-short@1.2 fins off",
                "arcas-robin-short@1.5 fins off",
                "arcas-robin-short@1.8 fins off",
                "arcas-robin-long@1 fins on",
                "arcas-robin-long@1.2 fins off",
            ]
        );
    }
}
