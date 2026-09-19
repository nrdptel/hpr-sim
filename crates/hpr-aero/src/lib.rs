//! Aerodynamics: Barrowman normal force and centre of pressure with extensions, drag buildup,
//! compressibility and override tables.
//!
//! **Guide:** [Aerodynamics][guide-aero]: the models, their sources, how well they are validated
//! and what they leave out.
//!
//! [guide-aero]: https://nrdptel.github.io/hpr-sim/physics/aero.html
//! [guide-cp]: https://nrdptel.github.io/hpr-sim/physics/aero.html#your-rockets-centre-of-pressure
//! [guide-flight]: https://nrdptel.github.io/hpr-sim/physics/flight.html#aerodynamics-in-flight
//! [guide-roll]: https://nrdptel.github.io/hpr-sim/physics/aero.html#roll-forcing-and-damping
//! [guide-fins-mach]: https://nrdptel.github.io/hpr-sim/physics/aero.html#fins-through-mach-1
//! [guide-drag-mach]: https://nrdptel.github.io/hpr-sim/physics/aero.html#drag-through-mach-1
//! [guide-override]: https://nrdptel.github.io/hpr-sim/physics/aero.html#the-normal-force-from-rasaero-ii
//!
//! - [`body`]: nose cones, body tubes and transitions: Barrowman's slope and centre of pressure.
//! - [`crossflow`]: body lift, the crossflow's push on a body at an angle of attack: Jorgensen's
//!   `η C_dn` against the body's fineness and the crossflow Mach number, or Galejs's constant.
//! - [`fins`]: fin sets: Barrowman's slope with Prandtl–Glauert, the mean aerodynamic chord,
//!   supersonic linear theory and the transonic join between them, fin-count and roll terms, and
//!   fin–body interference.
//! - [`drag`]: the terms of Niskanen's zero-lift drag buildup, and axial drag at an angle of
//!   attack.
//! - [`nose_drag`]: the pressure drag of noses, shoulders and steps from rest through Mach 1 to
//!   supersonic speeds, with Stoney's measured curves.
//! - [`afterbody`]: a boattail's wave drag faster than sound, and the base pressure behind it.
//! - [`shock_expansion`]: the second-order shock-expansion method for a pointed body faster than
//!   sound (NACA TN 3527).
//! - [`supersonic_boattail`]: a boattail's measured share of the normal force faster than sound
//!   (Washington and Pettis, RD-TM-68-5).
//! - [`table`]: override tables from another tool: the drag coefficient against Mach number, and
//!   the normal force and centre of pressure against Mach number and angle of attack, read from
//!   RASAero II's export.
//! - [`model`]: a rocket's terms built from a [`hpr_design::Layout`] and summed at a [`Flow`].
//!
//! A rocket's centre of pressure is [`NormalForce::cp_station_m`], in metres aft of the nose tip,
//! from [`AeroModel::normal_force`] at [`Flow::axial`] ([Your rocket's centre of
//! pressure][guide-cp] in the guide).
//!
//! Status: the normal force and centre of pressure from Mach 0 to 5 (fins through the transonic
//! region to supersonic linear theory, [Fins through Mach 1][guide-fins-mach]); the drag buildup
//! from Mach 0 to 5 (noses, shoulders and steps through Mach 1 by Niskanen's appendix B,
//! [Drag through Mach 1][guide-drag-mach]); drag override tables at any Mach number; normal-force
//! override tables from RASAero II's export ([The normal force from RASAero II][guide-override]);
//! the roll forcing of canted fins and the roll damping from Mach 0 to 5 ([`AeroModel::roll`],
//! [Roll: forcing and damping][guide-roll]).
//!
//! - Pitch and yaw damping in a flight come only from the flight engine (`hpr_sim`) evaluating
//!   each component in its own local flow, which includes the speed the rocket's rotation adds
//!   there ([Rigid-body flight][guide-flight] in the guide). The crate has no pitch or yaw damping
//!   coefficients; they would have to replace the local-flow damping, not add to it.
//! - Only components with a normal-force slope give that damping: nose cones, transitions and fin
//!   sets. Body tubes give none at small angles: their own slope is 0, and their body lift grows
//!   with `sin² α`.

pub mod afterbody;
pub mod blunt_tip;
pub mod body;
pub mod crossflow;
pub mod drag;
pub mod error;
pub mod fins;
pub mod model;
pub mod nose_drag;
pub mod shock_expansion;
pub mod supersonic_boattail;
pub mod table;

pub use afterbody::Boattail;
pub use body::{BODY_LIFT_K, BodyGeometry};
pub use crossflow::BodyLift;
pub use drag::{
    BaseBehindBoattail, BoattailTerm, ComponentDrag, ComponentDragTerms, Drag, DragConditions,
    MergedBoattail, PressureDragTerm, ReliefSource, WakeTerm,
};
pub use error::AeroError;
pub use fins::{
    FinAero, FinGeometry, FinLoading, FinOutline, FinRoll, FinRollTerms, fin_count_factor,
    interference_factor, roll_damping_interference, roll_forcing_interference, roll_sum, side_sum,
};
pub use model::{
    AeroModel, BodyAero, BodyModel, ComponentNormalForce, FinSetAero, Flow, MAX_CANT_RAD,
    NORMAL_FORCE_MACH_LIMIT, NormalForce, Roll, SUPERSONIC_JOIN_START_MACH,
    SUPERSONIC_JOIN_WIDTH_MACH, SupersonicBoattail, SupersonicBody,
};
pub use nose_drag::{PressureDragCurve, StoneyNose};
pub use table::{
    DragTable, NormalForceColumn, NormalForceLookup, NormalForceTable, TableReference,
    parse_mach_csv,
};

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
    use crate::testing::{body_part, committed_design, component, fin_set, nose, one_stage};

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
                // which RASAero II's constant subsonic slope doesn't have, and the linear join's
                // peak at M_s = 1.28. Since M1.8e7 its von Karman nose's vertical tip flies the
                // shock-expansion method behind a Newtonian cap, so its body carries lift on the
                // cylinder past Mach 1.2: Mach 2 reads +8.8% (-16.8% on slender-body theory) and
                // Mach 1.5 +13.2% (-3.1%; ADR-038).
                "calisto-rasaero-ii@0.8",
                "calisto-rasaero-ii@0.9",
                "calisto-rasaero-ii@0.95",
                "calisto-rasaero-ii@1.3",
                // The Arcas Robin: the measured transonic dip in the fins' lift and the join's
                // peak (Mach 0.8 to 1.2). Since M1.8e8 the committed designs fly the
                // shock-expansion method to their base (their vertical tip behind a Newtonian cap
                // since M1.8e7, their lip carrying nothing in the boattail's wake since e8), so
                // every row from Mach 1.5 is within the slope's target, where the short model read
                // -16.3% to -28.0% before. What is left on the long model at Mach 1.8 and 2.3 is
                // the centre of pressure, 0.52 and 0.53 calibers forward of the measured, where
                // the body reads 15% to 19% high fins off (ADR-039).
                "arcas-robin-short@0.8",
                "arcas-robin-short@0.9",
                "arcas-robin-short@0.95",
                "arcas-robin-short@1.2",
                "arcas-robin-long@0.9",
                "arcas-robin-long@1",
                "arcas-robin-long@1.2",
                "arcas-robin-long@1.8",
                "arcas-robin-long@2.3",
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
        c_a: Option<f64>,
        chamber_c_a: Option<f64>,
        forebody_c_a_chamber_only: Option<f64>,
    }

    /// M1.8b1 done-when: hpr's forebody drag (`C_D0` less the base drag) against the Arcas Robin
    /// wind-tunnel models' forebody axial force at every Mach number the reports give, fins on
    /// and off (TN D-4013's `C_A,corr`, Mach 0.6–1.2; TN D-4014's `C_A` less its chamber force,
    /// Mach 1.5–4.63). `cargo xtask aero` writes `validation/fixtures/aero/drag-vs-mach.json`;
    /// this test recomputes every hpr value from the committed designs at the tunnels' Reynolds
    /// number, checks every measured point is compared, and pins the rows within the 10% target
    /// set before measuring, so every other row is a pinned miss (42 of 44). Each miss is
    /// explained in `docs/physics/aero.md`, ADR-028 and ADR-030.
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
                // TN D-4014's forebody is its `C_A` less the chamber's force over the whole base,
                // `1.383 C_A,c` with 1.383 = (1.470/1.250)², to the readings' 4 decimals; the
                // chamber-only bound is `C_A − C_A,c`.
                if let (Some(c_a), Some(chamber)) = (point.c_a, point.chamber_c_a) {
                    let factor = (1.470f64 / 1.250).powi(2);
                    assert!(
                        (point.forebody_c_a - (c_a - factor * chamber)).abs() < 1.5e-4,
                        "{what}"
                    );
                    let bound = point.forebody_c_a_chamber_only.unwrap_or(f64::NAN);
                    assert!((bound - (c_a - chamber)).abs() < 1.5e-4, "{what}");
                }
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
        // Since the boattail's supersonic wave drag and the lip in its wake (ADR-030): before,
        // 8 rows were within 10%, six of them because the lip's 0.085 made up for the missing
        // wave drag.
        assert_eq!(
            within,
            ["arcas-robin-short@1 fins on", "arcas-robin-long@1 fins on",]
        );
    }

    /// M1.8b2 and M1.8b3 (ADR-029, ADR-030): Calisto's rows within 10% of its RASAero II export
    /// over the inputs the export doesn't record. The export's values at the sweep's Mach numbers
    /// come back from the committed fixture (hpr's value over one plus its error), and hpr flies
    /// the 2018 design with square, rounded and airfoil fins 2 to 6.35 mm thick, smooth or painted
    /// (20 µm), every 0.05 from Mach 0.1 to 2.0 at sea level. Before the boattail's supersonic
    /// wave drag no combination had rows within 10% in both the subsonic and the supersonic bands
    /// (the test was named `calistos_supersonic_gap_survives_every_plausible_fin_and_finish`).
    /// Now the committed inputs (square, 3 mm, smooth, ADR-009's rule) have 15, 3 and 8, and
    /// rounded fins 4.76 mm thick, smooth, have 15, 4 and 14: most of the supersonic gap left is
    /// within what the unrecorded inputs span. The committed design keeps ADR-009's rule.
    #[test]
    fn calistos_rows_by_fin_and_finish() {
        use hpr_design::{Component, FinCrossSection, Finish, Part};

        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/rocketpy-drag-curves.json"
        ))
        .unwrap();
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == "calisto-power-off")
            .unwrap();
        let rows: Vec<(f64, f64)> = case["sweep"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let hpr = r["hpr_cd0"].as_f64().unwrap();
                let error = r["relative_error"].as_f64().unwrap();
                (r["mach"].as_f64().unwrap(), hpr / (1.0 + error))
            })
            .collect();
        assert_eq!(rows.len(), 39);
        let air = hpr_atmos::Ussa76::standard().sample(0.0).unwrap().air;
        fn set(components: &mut [Component], section: FinCrossSection, t: f64, finish: Finish) {
            for c in components {
                c.finish = Some(finish);
                if let Part::FinSet(fins) = &mut c.part {
                    fins.cross_section = section;
                    fins.thickness_m = t;
                }
                set(&mut c.children, section, t, finish);
            }
        }
        let (mut counts, mut ranges) = (Vec::new(), Vec::new());
        for section in [
            FinCrossSection::Square,
            FinCrossSection::Rounded,
            FinCrossSection::Airfoil,
        ] {
            for t in [0.002, 0.003, 0.00476, 0.00635] {
                for finish in [Finish::Mirror {}, Finish::MassProductionPaint {}] {
                    let mut rocket =
                        committed_design("rocketpy-calisto-tests-motor-at-minus-1.373.json");
                    set(&mut rocket.stages[0].components, section, t, finish);
                    let model = AeroModel::new(&rocket.layout().unwrap()).unwrap();
                    // Rows within 10% by band: subsonic to 0.8, transonic below 1.2, supersonic.
                    let mut within = [0usize; 3];
                    let mut errors: [Vec<f64>; 3] = Default::default();
                    for &(mach, curve) in &rows {
                        let conditions = DragConditions::coasting(
                            mach * air.speed_of_sound_m_s / air.kinematic_viscosity_m2_s(),
                        );
                        let drag = model.drag(&Flow::axial(mach), &conditions).unwrap();
                        let error = drag.zero_lift_coefficient / curve - 1.0;
                        let band = if mach <= 0.8 {
                            0
                        } else if mach < 1.2 {
                            1
                        } else {
                            2
                        };
                        if error.abs() <= 0.10 {
                            within[band] += 1;
                        }
                        errors[band].push(error);
                    }
                    counts.push(within);
                    // Each band's least and greatest error, in percent to 0.1.
                    let pct = |e: f64| (e * 1000.0).round() / 10.0;
                    ranges.push(errors.map(|band| {
                        let min = band.iter().copied().fold(f64::INFINITY, f64::min);
                        let max = band.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        (pct(min), pct(max))
                    }));
                }
            }
        }
        // By section (square, rounded, airfoil), thickness (2, 3, 4.76, 6.35 mm) and finish
        // (smooth, painted): rows within 10% of 15 subsonic, 7 transonic and 17 supersonic.
        assert_eq!(
            counts,
            [
                [15, 3, 1],
                [7, 4, 8],
                [15, 3, 8],
                [3, 4, 15],
                [0, 3, 17],
                [0, 3, 17],
                [0, 3, 17],
                [0, 1, 2],
                [9, 4, 0],
                [14, 3, 4],
                [15, 3, 1],
                [14, 4, 9],
                [15, 4, 14],
                [12, 3, 17],
                [14, 4, 17],
                [9, 3, 17],
                [3, 4, 0],
                [14, 3, 2],
                [7, 3, 0],
                [13, 4, 7],
                [10, 4, 9],
                [13, 4, 17],
                [11, 4, 17],
                [13, 3, 17],
            ]
        );
        // The committed inputs (square, 3 mm, smooth) and the best of the rest (rounded,
        // 4.76 mm, smooth): no combination has every row within 10%.
        assert_eq!(ranges[2], [(3.9, 8.9), (-10.1, 16.4), (-14.9, -5.1)]);
        assert_eq!(ranges[12], [(-8.4, 5.8), (-8.9, 22.5), (-11.7, -3.4)]);
        assert!(counts.iter().all(|c| c != &[15, 7, 17]));
    }

    /// M1.8b3 (ADR-030): hpr's boattail pressure drag and the base pressure behind a boattail
    /// against measured conical boattails, and the boattail chart against Jack's second-order
    /// theory. `cargo xtask aero` writes each comparison into
    /// `validation/fixtures/aero/drag-vs-mach.json` from the transcribed references in
    /// `measured-boattails.json`; this recomputes every hpr value from the rows' geometry and pins
    /// the ranges the guide quotes. The targets were M1.8's 10%; how far each group misses is
    /// explained in `docs/physics/aero.md`.
    #[test]
    fn boattails_against_measurements() {
        use crate::afterbody::Boattail;
        let references: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/measured-boattails.json"
        ))
        .unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/drag-vs-mach.json"
        ))
        .unwrap();
        let f = |row: &serde_json::Value, key: &str| row[key].as_f64().unwrap();
        // The subsonic rule gives long boattails exactly 0.
        let same = |got: f64, want: f64, what: &str| {
            assert!(
                (got - want).abs() <= 1e-12 * want.abs().max(1e-3),
                "{what}: {got} against {want}"
            );
        };
        let boattail = |row: &serde_json::Value| {
            let (l, r) = (f(row, "length_ratio"), f(row, "diameter_ratio").min(1.0));
            Boattail::new(l, 1.0, r).unwrap()
        };
        let rows = |section: &str| {
            let reference = references[section]["rows"].as_array().unwrap();
            let compared = fixture[section]["rows"].as_array().unwrap();
            assert_eq!(reference.len(), compared.len(), "{section}");
            compared.clone()
        };
        // Errors in percent to 0.1, as the guide quotes them.
        let range = |errors: &[f64]| {
            let pct = |e: f64| (e * 1000.0).round() / 10.0;
            let min = errors.iter().copied().fold(f64::INFINITY, f64::min);
            let max = errors.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (errors.len(), pct(min), pct(max))
        };

        // Measured boattail pressure drag, grouped by angle and speed: attached boattails of 12°
        // and gentler under Niskanen's rule (to Mach 0.8), in the straight-line rise (to Mach 1),
        // where the Mach 1.2 value is held (to 1.2), and faster; Cubbage's 16° ones; and his
        // separated 30° and 45° ones. Compton's points near Mach 1 that his report calls
        // questionable are a group of their own.
        let mut groups: std::collections::BTreeMap<&str, Vec<f64>> = Default::default();
        let mut absolute: Vec<f64> = Vec::new();
        for row in rows("boattails") {
            let b = boattail(&row);
            let mach = f(&row, "mach");
            let hpr = b.pressure_drag_coefficient(mach).unwrap();
            same(f(&row, "hpr"), hpr, "boattail");
            let error = hpr / f(&row, "cd") - 1.0;
            assert!((f(&row, "error") - error).abs() < 1e-12);
            let deg = b.half_angle_rad.to_degrees();
            let group = if row["flow"] == "separated" {
                "separated"
            } else if row["questionable"] == true {
                "questionable"
            } else if deg > 12.0 {
                if mach >= 1.0 {
                    "steep"
                } else {
                    "steep, subsonic"
                }
            } else if mach >= 1.2 {
                absolute.push(hpr - f(&row, "cd"));
                "supersonic"
            } else if mach >= 1.0 {
                "held"
            } else if mach > 0.8 {
                "rise"
            } else {
                "rule"
            };
            groups.entry(group).or_default().push(error);
        }
        let got: Vec<(&str, (usize, f64, f64))> =
            groups.iter().map(|(k, v)| (*k, range(v))).collect();
        assert_eq!(
            got,
            [
                ("held", (4, -18.2, -5.4)),
                ("questionable", (27, -46.2, 60.0)),
                ("rise", (28, -77.5, 7.6)),
                ("rule", (58, -100.0, -83.5)),
                ("separated", (3, -2.8, 6.6)),
                ("steep", (9, 26.4, 54.2)),
                ("steep, subsonic", (6, -30.2, 60.4)),
                ("supersonic", (58, -21.9, 28.3)),
            ]
        );
        // From Mach 1.2 the biggest misses in percent are the smallest drags, 3° and 5°
        // boattails of 0.01 to 0.02; in drag coefficient every row is within 0.0123.
        assert!(absolute.iter().all(|e| e.abs() < 0.0123));

        // The base pressure behind a boattail: the error in base drag on the cylinder's area.
        let mut differences = Vec::new();
        for row in rows("base_pressures") {
            let b = boattail(&row);
            let k = b
                .base_pressure_ratio(f(&row, "mach"), b.area_ratio)
                .unwrap();
            same(f(&row, "k_hpr"), k, "base ratio");
            let measured = f(&row, "boattail_cp") / f(&row, "cylinder_cp");
            let difference = (k - measured) * -f(&row, "cylinder_cp") * b.area_ratio;
            assert!((f(&row, "base_cd_difference") - difference).abs() < 1e-12);
            differences.push(difference);
        }
        assert_eq!(differences.len(), 12);
        assert!(differences.iter().all(|d| d.abs() < 0.0102));
        assert_eq!(differences.iter().filter(|d| d.abs() <= 0.004).count(), 10);

        // The chart, held to the 2D limit, against Jack's second-order theory.
        let (mut inside, mut past, mut shallow) = (Vec::new(), Vec::new(), Vec::new());
        for row in rows("second_order_theory") {
            let a = f(&row, "area_ratio");
            let theta = f(&row, "half_angle_deg").to_radians();
            let r = a.sqrt();
            let b = Boattail::new((1.0 - r) / (2.0 * theta.tan()), 1.0, r).unwrap();
            let mach = f(&row, "mach");
            let hpr = b.attached_pressure_drag(mach).unwrap();
            same(f(&row, "hpr"), hpr, "theory");
            let error = hpr / f(&row, "cd") - 1.0;
            assert!((f(&row, "error") - error).abs() < 1e-12);
            if a > 0.6 + 1e-9 {
                shallow.push(error);
            } else if f(&row, "x") <= 1.4 {
                inside.push(error);
            } else {
                past.push(error);
            }
        }
        assert_eq!(range(&inside), (83, -10.4, 8.0));
        assert_eq!(range(&past), (28, -2.7, 8.0));
        // The 0.7 and 0.8 curves read high against Jack past x ≈ 1.
        assert_eq!(range(&shallow), (40, -1.5, 32.4));
    }

    #[derive(Deserialize)]
    struct HandbookDrag {
        calculations: Vec<HandbookCalculation>,
    }

    #[derive(Deserialize)]
    struct HandbookCalculation {
        design: String,
        rows: Vec<HandbookComparison>,
    }

    #[derive(Deserialize)]
    struct HandbookComparison {
        mach: f64,
        reynolds_per_m: f64,
        reference: HandbookParts,
        hpr: HandbookParts,
        compared: HandbookCompared,
        error: f64,
        within_target: bool,
    }

    #[derive(Deserialize)]
    struct HandbookCompared {
        reference: f64,
        hpr: f64,
    }

    #[derive(Deserialize)]
    struct HandbookParts {
        friction: f64,
        nose: f64,
        fins: f64,
        base: f64,
        #[serde(default)]
        other: f64,
        total: f64,
    }

    #[derive(Deserialize)]
    struct HandbookReference {
        rows: Vec<HandbookRow>,
    }

    #[derive(Deserialize)]
    struct HandbookRow {
        mach: f64,
        reynolds_per_m: f64,
        body_friction: f64,
        fin_friction: f64,
        friction: f64,
        nose_wave: Option<f64>,
        fin_wave: Option<f64>,
        fin_base: f64,
        fins: f64,
        base: f64,
        total_jet_off: f64,
    }

    /// M1.8b2: hpr's `C_D0`, base drag included, against MIL-HDBK-762's sample drag calculation
    /// (Table 5-4, pp. 5-58 to 5-66) for the rocket of its Fig. 5-155, term by term, at the
    /// table's Reynolds numbers. A calculation with every input known, not a measurement: it
    /// shows which way hpr's methods lean where RASAero II's curves can't, because their inputs
    /// are unrecorded (ADR-029). The handbook's fins are single wedges, sharp at the leading edge,
    /// which hpr can't represent, so the fins' pressure drag is recorded but compared on neither
    /// side. `cargo xtask aero` writes the comparison to
    /// `validation/fixtures/aero/drag-vs-mach.json`; this checks the transcription's sums,
    /// recomputes hpr's terms, and pins the rows within M1.8's 10%.
    #[test]
    fn drag_against_mil_hdbk_762_sample() {
        let reference: HandbookReference = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/mil-hdbk-762-sample-drag.json"
        ))
        .unwrap();
        // Each printed row's parts add up to its totals, to the table's 3 decimals.
        for row in &reference.rows {
            let what = format!("Table 5-4 at Mach {}", row.mach);
            assert!(
                (row.body_friction + row.fin_friction - row.friction).abs() < 1.5e-3,
                "{what}"
            );
            assert!(
                (row.fin_wave.unwrap_or(0.0) + row.fin_base - row.fins).abs() < 1.5e-3,
                "{what}"
            );
            let sum = row.friction + row.nose_wave.unwrap_or(0.0) + row.fins + row.base;
            assert!((sum - row.total_jet_off).abs() < 1.5e-3, "{what}");
            assert_eq!(row.nose_wave.is_none(), row.mach < 0.9, "{what}");
            assert_eq!(row.fin_wave.is_none(), row.mach < 0.95, "{what}");
        }
        let fixture: HandbookDrag = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/drag-vs-mach.json"
        ))
        .unwrap();
        let [calculation] = fixture.calculations.as_slice() else {
            panic!("one calculation")
        };
        let model =
            AeroModel::new(&committed_design(&calculation.design).layout().unwrap()).unwrap();
        assert_eq!(calculation.rows.len(), reference.rows.len());
        let mut within = Vec::new();
        for (row, printed) in calculation.rows.iter().zip(&reference.rows) {
            let what = format!("Mach {}", row.mach);
            assert_eq!(
                (row.mach, row.reynolds_per_m),
                (printed.mach, printed.reynolds_per_m)
            );
            let r = &row.reference;
            assert_eq!(
                (r.friction, r.nose, r.fins, r.base, r.total),
                (
                    printed.friction,
                    printed.nose_wave.unwrap_or(0.0),
                    printed.fins,
                    printed.base,
                    printed.total_jet_off
                ),
                "{what}"
            );
            let conditions = DragConditions::coasting(printed.reynolds_per_m);
            let flow = Flow::axial(row.mach);
            let drag = model.drag(&flow, &conditions).unwrap();
            let parts = model.buildup_components(&flow, &conditions).unwrap();
            let pressure = |id: &str| {
                parts
                    .iter()
                    .filter(|p| p.id == id)
                    .map(|p| p.drag.pressure)
                    .sum::<f64>()
            };
            let h = &row.hpr;
            // To 1e-12, not bit for bit: the nose's drag goes through `powf`, `ln` and `atan`,
            // whose last bit differs between platforms' maths libraries.
            close(h.total, drag.zero_lift_coefficient, 1e-12, &what);
            close(h.friction, drag.friction, 1e-12, &what);
            close(h.base, drag.base, 1e-12, &what);
            close(h.nose, pressure("nose"), 1e-12, &what);
            close(h.fins, pressure("fins"), 1e-12, &what);
            assert_eq!(
                h.other, 0.0,
                "{what}: only the nose and fins have pressure drag"
            );
            assert!(
                (h.friction + h.nose + h.fins + h.base - h.total).abs() < 1e-12,
                "{what}"
            );
            let c = &row.compared;
            close(c.hpr, h.total - h.fins, 1e-12, &what);
            close(
                c.reference,
                printed.total_jet_off - printed.fins,
                1e-12,
                &what,
            );
            let error = c.hpr / c.reference - 1.0;
            assert!((row.error - error).abs() < 1e-12, "{what}");
            assert_eq!(row.within_target, error.abs() <= 0.10, "{what}");
            if row.within_target {
                within.push(row.mach);
            }
        }
        // Within 10% at Mach 0.7 and from 1.6 (ADR-029). From Mach 0.9 to 1.2 hpr reads 12% to
        // 32% high, the nose (Niskanen's ogive, 0.234 against the handbook's 0.109 at Mach 1.1)
        // and the base (Fleeman's 0.25 against 0.183 at Mach 1.0). From Mach 1.6 it reads 6% to
        // 10% low: friction (hpr's body form factor 1.02 against the handbook's 1.15) and base
        // drag (0.125 against 0.147 at Mach 2). At Mach 0.5 it is 10.3% low, the friction.
        assert_eq!(within, [0.7, 1.6, 2.0, 2.4, 2.8, 3.2]);
    }

    /// The Arcas Robin comparison's two input choices (validation audit): of the 44 rows, none
    /// within 10% with the square section and the default 20 µm finish, none with the airfoil
    /// section, none with a polished finish, and 2 with both (the committed designs); before the
    /// boattail's supersonic wave drag (ADR-030) these were 3, 5, 6 and 8. Taking the chamber's
    /// force over the chamber alone changes none of the four. Allowing each reading its
    /// uncertainty and the reports' ±0.004, neither of the committed design's 2 could fall the
    /// other side of 10%.
    #[test]
    fn drag_against_mach_depends_on_the_fins_and_finish() {
        use hpr_design::{FinCrossSection, Finish, Part};
        let tunnel: serde_json::Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/arcas-robin-wind-tunnel.json"
        ))
        .unwrap();
        let conditions = DragConditions::coasting(3.0e6 / 0.3048);
        let mut counts = Vec::new();
        for (section, finish) in [
            (FinCrossSection::Square, None),
            (FinCrossSection::Airfoil, None),
            (FinCrossSection::Square, Some(Finish::Polished {})),
            (FinCrossSection::Airfoil, Some(Finish::Polished {})),
        ] {
            let (mut within, mut chamber_only, mut fragile) = (0, 0, 0);
            for configuration in tunnel["configurations"].as_array().unwrap() {
                let mut rocket = committed_design(configuration["design"].as_str().unwrap());
                for component in &mut rocket.stages[0].components {
                    component.finish = finish;
                    for child in &mut component.children {
                        child.finish = finish;
                        if let Part::FinSet(set) = &mut child.part {
                            set.cross_section = section;
                        }
                    }
                }
                let model = AeroModel::new(&rocket.layout().unwrap()).unwrap();
                let fin_ids: Vec<&str> = model.fin_sets().iter().map(|f| f.id.as_str()).collect();
                for point in configuration["axial_force"].as_array().unwrap() {
                    let mach = point["mach"].as_f64().unwrap();
                    let fins = point["fins"] == "on";
                    let hpr: f64 = model
                        .buildup_components(&Flow::axial(mach), &conditions)
                        .unwrap()
                        .iter()
                        .filter(|part| fins || !fin_ids.contains(&part.id.as_str()))
                        .map(|part| part.drag.friction + part.drag.pressure + part.drag.parasitic)
                        .sum();
                    let measured = point["forebody_c_a"].as_f64().unwrap();
                    let inside = |reference: f64| (hpr / reference - 1.0).abs() <= 0.10;
                    within += usize::from(inside(measured));
                    let bound = point["forebody_c_a_chamber_only"]
                        .as_f64()
                        .unwrap_or(measured);
                    chamber_only += usize::from(inside(bound));
                    let spread = point["uncertainty"].as_f64().unwrap() + 0.004;
                    fragile += usize::from(
                        inside(measured)
                            && !(inside(measured - spread) && inside(measured + spread)),
                    );
                }
            }
            assert_eq!(within, chamber_only, "{section:?}, {finish:?}");
            counts.push((within, fragile));
        }
        assert_eq!(counts.iter().map(|c| c.0).collect::<Vec<_>>(), [0, 0, 0, 2]);
        assert_eq!(counts[3].1, 0);
    }

    /// M1.8c: hpr's roll forcing against the Arcas Robin's measured roll effectiveness (TN D-4014
    /// Fig. 14) and its roll damping against the Basic Finner's (Barrowman 1967 Fig. 5-7), as
    /// `cargo xtask aero` writes them: every row recomputed from the committed designs and
    /// references, and the ranges the guide quotes.
    #[test]
    fn roll_against_mach() {
        use serde_json::Value;
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/roll-vs-mach.json"
        ))
        .unwrap();
        let tunnel: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/arcas-robin-wind-tunnel.json"
        ))
        .unwrap();
        let finner: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/aero/basic-finner-roll-damping.json"
        ))
        .unwrap();
        let f = |v: &Value| v.as_f64().unwrap();
        let mut errors: Vec<(f64, f64)> = Vec::new();
        for configuration in fixture["arcas_robin"].as_array().unwrap() {
            let design = configuration["design"].as_str().unwrap();
            let model = AeroModel::new(&committed_design(design).layout().unwrap()).unwrap();
            let set = &model.fin_sets()[0];
            let measured = tunnel["configurations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["design"] == design)
                .unwrap();
            for row in configuration["rows"].as_array().unwrap() {
                let mach = f(&row["mach"]);
                let what = format!("{design} at Mach {mach}");
                let fin = set
                    .fin
                    .roll(mach, set.body_radius_m, model.reference_diameter_m())
                    .unwrap();
                let hpr =
                    f64::from(set.count) * fin.forcing_per_rad * set.roll_forcing_interference * PI
                        / 180.0;
                close(f(&row["hpr_c_l_delta_per_deg"]), hpr, 1e-12, &what);
                let reading = measured["roll_effectiveness"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| f(&c["mach"]) == mach)
                    .unwrap()["alpha_deg_c_l_delta_per_deg"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|p| f(&p[0]).abs() < 0.5)
                    .map(|p| f(&p[1]))
                    .unwrap();
                assert_eq!(f(&row["reference_c_l_delta_per_deg"]), reading, "{what}");
                close(f(&row["error"]), hpr / reading - 1.0, 1e-12, &what);
                errors.push((mach, hpr / reading - 1.0));
            }
        }
        // The guide's summary: from Mach 2.3, all 8 within 5.3%; below, at Mach 1.5 and 1.8, 3
        // rows 14.3% to 47.8% high.
        assert_eq!(errors.len(), 11);
        let high: Vec<f64> = errors.iter().filter(|e| e.0 >= 2.3).map(|e| e.1).collect();
        let low: Vec<f64> = errors.iter().filter(|e| e.0 < 2.3).map(|e| e.1).collect();
        assert_eq!(high.len(), 8);
        assert!(high.iter().all(|e| e.abs() < 0.0535));
        let (lo, hi) = low
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &e| {
                (a.min(e), b.max(e))
            });
        assert!(
            (lo - 0.143).abs() < 5e-4 && (hi - 0.478).abs() < 5e-4,
            "{lo} {hi}"
        );

        // The Basic Finner: four square fins on a body one diameter across.
        let basic = &fixture["basic_finner"];
        let d = 1.0;
        let fin = FinAero::new(
            &FinPlanform::Trapezoidal {
                root_chord_m: d,
                tip_chord_m: d,
                span_m: d,
                sweep_m: 0.0,
            },
            0.25 * PI * d * d,
        )
        .unwrap();
        let k_r = roll_damping_interference(d, 0.5 * d, 1.0).unwrap();
        close(f(&basic["roll_damping_interference"]), k_r, 1e-15, "k_R(B)");
        for (key, source) in [
            ("wind_tunnel", "wind_tunnel_c_lp"),
            ("barrowman_theory", "barrowman_theory_c_lp"),
        ] {
            let rows = basic[key].as_array().unwrap();
            let readings = finner[source].as_array().unwrap();
            assert_eq!(rows.len(), readings.len());
            for (row, reading) in rows.iter().zip(readings) {
                let mach = f(&row["mach"]);
                assert_eq!(mach, f(&reading["mach"]));
                assert_eq!(f(&row["reference_c_lp"]), f(&reading["c_lp"]));
                let hpr = 4.0 * fin.roll(mach, 0.5 * d, d).unwrap().damping * k_r;
                close(f(&row["hpr_c_lp"]), hpr, 1e-12, key);
                close(
                    f(&row["error"]),
                    hpr / f(&reading["c_lp"]) - 1.0,
                    1e-12,
                    key,
                );
            }
        }
        // The guide's summary: 5.9% to 16.2% low against the wind tunnel, 2.0% against
        // Barrowman's own computed value.
        let tunnel_errors: Vec<f64> = basic["wind_tunnel"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| f(&r["error"]))
            .collect();
        let (lo, hi) = tunnel_errors
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &e| {
                (a.min(e), b.max(e))
            });
        assert!(
            (lo + 0.162).abs() < 5e-4 && (hi + 0.059).abs() < 5e-4,
            "{lo} {hi}"
        );
        let theory = f(&basic["barrowman_theory"][0]["error"]);
        assert!((theory + 0.020).abs() < 5e-4, "{theory}");
    }

    /// The whole rocket's roll sums its fin sets, each with its cant, count and body factors: two
    /// sets canted each way at Mach 0.5 to 4.5; a positive cant rolls toward `−z_B`; Mach 5 is
    /// refused; no cant, no steady roll.
    #[test]
    fn a_rocket_rolls_by_the_sum_of_its_fin_sets() {
        let fins = |count: u32, cant: f64, planform: FinPlanform| {
            let mut part = fin_set(count, planform);
            if let hpr_design::Part::FinSet(set) = &mut part {
                set.cant_rad = cant;
            }
            part
        };
        let rocket = |cants: [f64; 2]| {
            let mut tube = component("tube", body_part(0.8, 0.03, 0.03), None);
            tube.children = vec![
                component(
                    "fore",
                    fins(
                        3,
                        cants[0],
                        FinPlanform::Trapezoidal {
                            root_chord_m: 0.05,
                            tip_chord_m: 0.02,
                            span_m: 0.03,
                            sweep_m: 0.02,
                        },
                    ),
                    Some(Position::Top { aft_offset_m: 0.1 }),
                ),
                component(
                    "aft",
                    fins(
                        4,
                        cants[1],
                        FinPlanform::Elliptical {
                            root_chord_m: 0.08,
                            span_m: 0.05,
                        },
                    ),
                    Some(Position::Bottom { aft_offset_m: 0.0 }),
                ),
            ];
            one_stage(
                vec![
                    component(
                        "nose",
                        nose(NoseShape::Ogive { radius_ratio: 1.0 }, 0.2, 0.03),
                        None,
                    ),
                    tube,
                ],
                ReferenceDiameter::Maximum {},
            )
        };
        let (a, b) = (0.02, -0.01);
        let model = AeroModel::new(&rocket([a, b]).layout().unwrap()).unwrap();
        let d = model.reference_diameter_m();
        for mach in [0.5, 0.9, 1.1, 2.0, 4.5] {
            let roll = model.roll(mach).unwrap();
            let (mut forcing, mut damping) = (0.0, 0.0);
            for set in model.fin_sets() {
                let fin = set.fin.roll(mach, set.body_radius_m, d).unwrap();
                let n = f64::from(set.count);
                forcing -= n * fin.forcing_per_rad * set.roll_forcing_interference * set.cant_rad;
                damping += n * fin.damping * set.roll_damping_interference;
            }
            close(roll.forcing, forcing, 1e-13, "forcing");
            close(roll.damping, damping, 1e-13, "damping");
            assert!(roll.damping < 0.0);
        }
        let one = AeroModel::new(&rocket([a, 0.0]).layout().unwrap()).unwrap();
        assert!(one.roll(0.5).unwrap().forcing < 0.0);
        assert!(model.roll(5.0).is_err());
        let still = AeroModel::new(&rocket([0.0, 0.0]).layout().unwrap()).unwrap();
        // Past 15° of cant, or a NaN, the model refuses.
        for cant in [0.3, -0.3, f64::NAN] {
            let refused = rocket([cant, 0.0])
                .layout()
                .map_or(true, |layout| AeroModel::new(&layout).is_err());
            assert!(refused, "cant {cant}");
        }
        assert_eq!(still.steady_roll_rate_rad_s(0.5, 170.0).unwrap(), 0.0);
        assert!(model.steady_roll_rate_rad_s(0.5, -1.0).is_err());
    }
}
