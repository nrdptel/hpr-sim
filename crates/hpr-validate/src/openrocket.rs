//! hpr held to OpenRocket's own answers ([M2.2][m2-2]): the tests, and one rule of OpenRocket's
//! that `cargo xtask ork` needs to explain a gap, [`openrocket_fin_set_roll_kg_m2`].
//!
//! `validation/fixtures/ork/openrocket-conventions.json` is OpenRocket 24.12 reading small probe
//! designs, each asking one question a `.ork` leaves to its reader. What does a shoulder written
//! with no wall weigh? What is a part that names no material made of? Where is a centre of gravity
//! override measured from? Which override wins when a part and the parts inside it both have one?
//! The record is written by `validation/oracles/openrocket/conventions.py`, which runs OpenRocket
//! and never reads it ([ADR-061][adr-061]). These tests read the same documents with `hpr_io::ork`
//! and hold hpr's structure to OpenRocket's. Where hpr keeps a rule of its own, a *departure*, the
//! test pins how far apart the two are, so a change on either side shows.
//!
//! [m2-2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2
//! [adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21

use hpr_design::fins::FinSet;

/// The roll inertia OpenRocket 24.12 gives a fin set about its own centre of mass, kg·m², **inferred
/// from its output, not taken from its source** (which is GPL and not read). For two or more fins,
/// whose centre is on the rocket's axis, it is the set's mass `m` spread evenly along a thin rod
/// from the body at `R` out to `R + hₑ`,
///
/// ```text
/// I = m (R² + R hₑ + hₑ²/3),   hₑ² = A h / c_r,
/// ```
///
/// where `A` is one fin's planform area, `h` its span and `c_r` its root chord: a rectangle's own
/// span, shorter for a fin that narrows outward. For one fin it is the same rod about its own
/// middle, `m hₑ²/12`. `validation/oracles/openrocket/conventions.py` measured it on rectangles of
/// two chords, two spans and two body radii, a trapezoid, a triangle, an ellipse (OpenRocket's
/// 30-sided polygon), one fin, and fins with a rounded or airfoil section, a tab or fillets: every
/// one to 1e-12 on the probe's structure but the ellipse (by its polygon) and a cant (2.66e-5). A
/// tab's mass takes the planform's value, and the section and thickness play no part. hpr's own is
/// the exact integral of `r²` over the fin ([`FinSet::mass_properties`]); the two agree on a
/// rectangle without a tab, but for the thickness ([ADR-062][adr-062]).
///
/// `mass_kg` is the set's, all fins together. `None` for a planform `hpr-design` refuses, a count
/// of zero, or a radius or mass that is negative or not finite.
///
/// [adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21
#[must_use]
pub fn openrocket_fin_set_roll_kg_m2(
    fins: &FinSet,
    body_radius_m: f64,
    mass_kg: f64,
) -> Option<f64> {
    let area_m2 = fins.planform.geometry().ok()?.area_m2;
    rod_roll_kg_m2(
        fins.count,
        area_m2,
        fins.planform.span_m(),
        fins.planform.root_chord_m(),
        body_radius_m,
        mass_kg,
    )
}

/// [`openrocket_fin_set_roll_kg_m2`] from the numbers it uses, so that a test can give it
/// OpenRocket's polygon for an ellipse.
fn rod_roll_kg_m2(
    count: u32,
    area_m2: f64,
    span_m: f64,
    root_m: f64,
    body_radius_m: f64,
    mass_kg: f64,
) -> Option<f64> {
    let usable = |x: f64| x.is_finite() && x >= 0.0;
    if count == 0
        || !(usable(area_m2) && usable(span_m) && usable(body_radius_m) && usable(mass_kg))
        || !(root_m.is_finite() && root_m > 0.0)
    {
        return None;
    }
    let reach2 = area_m2 * span_m / root_m;
    if count == 1 {
        return Some(mass_kg * reach2 / 12.0);
    }
    let r = body_radius_m;
    Some(mass_kg * (r * r + r * reach2.sqrt() + reach2 / 3.0))
}

#[cfg(test)]
mod tests {
    use hpr_design::tree::{Layout, Part};
    use hpr_io::ork::{self, StoredReferenceExclusion, StoredSimulation};
    use serde_json::Value;

    fn record() -> Value {
        let text = include_str!("../../../validation/fixtures/ork/openrocket-conventions.json");
        serde_json::from_str(text).expect("the committed record is JSON")
    }

    fn probe<'a>(record: &'a Value, question: &str) -> &'a Value {
        let probe = &record["probes"][question];
        assert!(probe.is_object(), "no probe asks {question:?}");
        probe
    }

    /// hpr's layout of a probe, and the warnings it raised reading it.
    fn hpr(probe: &Value) -> (Layout, Vec<String>) {
        let document = probe["document"].as_str().expect("the probe's document");
        let read = ork::read(document.as_bytes()).expect("a probe reads");
        let spine = ork::rocket(&read.value.document);
        let layout = spine.value.layout().expect("a probe lays out");
        let warnings = spine.warnings.iter().map(|w| w.message.clone()).collect();
        (layout, warnings)
    }

    /// Mass (kg), the centre of mass's station aft of the tip (m), and the roll and pitch inertias
    /// about it (kg·m²): hpr's and OpenRocket's. Pitch is the mean of the two inertias across the
    /// axis, and OpenRocket's `ixx` is roll: both as `cargo xtask ork` measured on its probe tube
    /// (ADR-060).
    fn both(probe: &Value) -> ([f64; 4], [f64; 4], Vec<String>) {
        let (layout, warnings) = hpr(probe);
        let s = &layout.structure;
        let i = s.inertia_kg_m2;
        let ours = [
            s.mass_kg,
            // hpr's `+z` points at the nose from the tip, so a station aft of it is `-z`.
            -s.cg_m.z,
            i.z_axis.z,
            (i.x_axis.x + i.y_axis.y) / 2.0,
        ];
        let or = &probe["structure"];
        let number = |key: &str| or[key].as_f64().expect("a number");
        let theirs = [
            number("mass_kg"),
            number("cm_x_m"),
            number("ixx"),
            (number("iyy") + number("izz")) / 2.0,
        ];
        (ours, theirs, warnings)
    }

    fn relative(ours: f64, theirs: f64) -> f64 {
        (ours - theirs) / theirs
    }

    fn stored(status: &str, results_xml: Option<&str>) -> StoredSimulation {
        let xml = format!(
            r#"<openrocket version="1.10"><rocket><name>R</name></rocket><simulations>
              <simulation status="{status}"><name>stored</name><simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>{results}</simulation>
            </simulations></openrocket>"#,
            results = results_xml.unwrap_or_default(),
        );
        ork::design(
            &ork::read(xml.as_bytes())
                .expect("stored fixture reads")
                .value,
        )
        .value
        .simulations
        .into_iter()
        .next()
        .expect("stored simulation")
    }

    fn plausible_results_xml() -> &'static str {
        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/>"#
    }

    fn stored_with_provenance(
        simulator: Option<&str>,
        calculator: Option<&str>,
    ) -> StoredSimulation {
        let simulator = simulator.map_or_else(String::new, |value| {
            format!("<simulator>{value}</simulator>")
        });
        let calculator = calculator.map_or_else(String::new, |value| {
            format!("<calculator>{value}</calculator>")
        });
        let xml = format!(
            r#"<openrocket version="1.10"><rocket><name>R</name></rocket><simulations>
              <simulation status="uptodate"><name>stored</name>{simulator}{calculator}{results}</simulation>
            </simulations></openrocket>"#,
            results = plausible_results_xml(),
        );
        ork::design(
            &ork::read(xml.as_bytes())
                .expect("stored fixture reads")
                .value,
        )
        .value
        .simulations
        .into_iter()
        .next()
        .expect("stored simulation")
    }

    fn stored_with_results(results_xml: &str) -> StoredSimulation {
        stored("uptodate", Some(results_xml))
    }

    fn results_with_branches(apogee: &str, branches: &str) -> String {
        format!(
            r#"<flightdata maxaltitude="{apogee}" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20">{branches}</flightdata>"#
        )
    }

    /// Stored results remain readable, but stale, missing and physically contradictory values are
    /// not allowed to become validation references or inflate a reference census.
    #[test]
    fn stale_and_implausible_stored_results_are_excluded_from_gates_and_census() {
        let cases = [
            (stored("uptodate", Some(plausible_results_xml())), None),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"><databranch name="Sustainer" types="Time,Altitude"><datapoint>1,1</datapoint><datapoint>2,1</datapoint></databranch></flightdata>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::InconsistentResults),
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"><databranch name="Sustainer" types="Time,Altitude"><datapoint>1,100</datapoint><datapoint>2,100.05</datapoint></databranch></flightdata>"#,
                    ),
                ),
                None,
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"><databranch name="Sustainer" types="Time,Altitude"><datapoint>1,100</datapoint><datapoint>2,100.2</datapoint></databranch></flightdata>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::InconsistentResults),
            ),
            (
                stored("outdated", Some(plausible_results_xml())),
                Some(StoredReferenceExclusion::Outdated),
            ),
            (
                stored("notsimulated", Some(plausible_results_xml())),
                Some(StoredReferenceExclusion::NotSimulated),
            ),
            (
                stored("uptodate", None),
                Some(StoredReferenceExclusion::MissingResults),
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="-1" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::ImpossibleSummary),
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="NaN" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::MissingSummary),
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"><databranch name="Sustainer" types="Time,Altitude"><datapoint>2,10</datapoint><datapoint>1,20</datapoint></databranch></flightdata>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::InconsistentResults),
            ),
            (
                stored(
                    "uptodate",
                    Some(
                        r#"<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"><databranch name="Sustainer" types="Time,Altitude"><datapoint>1,-1</datapoint><datapoint>2,20</datapoint></databranch></flightdata>"#,
                    ),
                ),
                Some(StoredReferenceExclusion::ImpossibleSummary),
            ),
        ];
        let classified = cases
            .iter()
            .filter(|(simulation, exclusion)| simulation.reference_exclusion() == *exclusion)
            .count();
        assert_eq!(classified, cases.len());
        assert_eq!(
            cases
                .iter()
                .filter(|(_, exclusion)| exclusion.is_none())
                .count(),
            2
        );
        assert_eq!(
            StoredReferenceExclusion::Outdated.reason(),
            "status-outdated"
        );
        assert_eq!(
            StoredReferenceExclusion::MissingResults.reason(),
            "missing-results"
        );
        assert_eq!(
            stored_with_provenance(None, Some("BarrowmanCalculator")).reference_exclusion(),
            Some(StoredReferenceExclusion::MissingSimulator)
        );
        assert_eq!(
            stored_with_provenance(Some("OtherSimulator"), Some("BarrowmanCalculator"))
                .reference_exclusion(),
            Some(StoredReferenceExclusion::UnsupportedSimulator)
        );
        assert_eq!(
            stored_with_provenance(Some("RK4Simulator"), None).reference_exclusion(),
            Some(StoredReferenceExclusion::MissingCalculator)
        );
        assert_eq!(
            stored_with_provenance(Some("RK4Simulator"), Some("OtherCalculator"))
                .reference_exclusion(),
            Some(StoredReferenceExclusion::UnsupportedCalculator)
        );
    }

    /// The apogee summary maps to OpenRocket's primary branch, not whichever branch happens to
    /// have the numerically closest maximum. Secondary stages may also record apogee events.
    #[test]
    fn stored_apogee_uses_the_first_apogee_branch() {
        let branches = r#"
          <databranch name="Sustainer" types="Time,Altitude">
            <event time="5" type="apogee"/><datapoint>0,0</datapoint><datapoint>5,100</datapoint>
          </databranch>
          <databranch name="Booster" types="Time,Altitude">
            <event time="4" type="apogee"/><datapoint>0,0</datapoint><datapoint>4,99.9</datapoint>
          </databranch>"#;
        let simulation = stored_with_results(&results_with_branches("100", branches));
        assert_eq!(simulation.reference_exclusion(), None);
    }

    /// A single altitude-bearing branch identifies the summary without an apogee event, but two
    /// such branches do not give enough information to identify which series owns it.
    #[test]
    fn stored_apogee_branch_requires_unambiguous_branch_evidence() {
        let single = r#"<databranch name="Sustainer" types="Time,Altitude">
          <datapoint>0,0</datapoint><datapoint>5,100</datapoint>
        </databranch>"#;
        assert_eq!(
            stored_with_results(&results_with_branches("100", single)).reference_exclusion(),
            None
        );
        let multiple = format!(
            r#"{single}<databranch name="Booster" types="Time,Altitude"><datapoint>0,0</datapoint><datapoint>5,99</datapoint></databranch>"#
        );
        assert_eq!(
            stored_with_results(&results_with_branches("100", &multiple)).reference_exclusion(),
            Some(StoredReferenceExclusion::UninspectableSeries)
        );
    }

    /// A stored maximum matches at the exact allowance boundary; just beyond it does not. The
    /// relative allowance takes over at 1 m, where it equals the fixed 1 mm allowance.
    #[test]
    fn stored_apogee_comparison_uses_the_two_sided_allowance() {
        let check = |apogee: &str, series: &str| {
            let branch = format!(
                r#"<databranch name="Sustainer" types="Time,Altitude"><datapoint>0,0</datapoint><datapoint>5,{series}</datapoint></databranch>"#
            );
            stored_with_results(&results_with_branches(apogee, &branch)).reference_exclusion()
        };
        assert_eq!(check("100", "100.1"), None);
        assert_eq!(
            check("100", "100.1000001"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
        assert_eq!(check("1", "1.001"), None);
        assert_eq!(
            check("1", "1.0010001"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
        assert_eq!(
            check("0.5", "0.501"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
        assert_eq!(check("0.5", "0.5009999"), None);
        assert_eq!(check("100", "99.9"), None);
        assert_eq!(
            check("100", "99.8999999"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
    }

    #[test]
    fn stored_times_agree_with_flight_and_apogee_summaries() {
        let branch = |last_time: &str, apogee_time: &str| {
            format!(
                r#"<databranch name="Sustainer" types="Time,Altitude"><event time="{apogee_time}" type="apogee"/><datapoint>0,0</datapoint><datapoint>{last_time},100</datapoint></databranch>"#
            )
        };
        let run = |last_time: &str, apogee_time: &str| {
            stored_with_results(&results_with_branches(
                "100",
                &branch(last_time, apogee_time),
            ))
            .reference_exclusion()
        };
        assert_eq!(run("20.0009", "5"), None);
        assert_eq!(run("20", "5.0009"), None);
        assert_eq!(
            run("20.0011", "5"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
        assert_eq!(
            run("20", "5.0011"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
    }

    #[test]
    fn non_finite_event_times_exclude_stored_results() {
        let mut simulation = stored_with_results(&results_with_branches(
            "100",
            r#"<databranch name="Sustainer" types="Time,Altitude"><event time="5" type="launch"/><datapoint>0,0</datapoint><datapoint>5,100</datapoint></databranch>"#,
        ));
        simulation.results.as_mut().unwrap().branches[0].events[0].time_s = f64::NAN;
        assert_eq!(
            simulation.reference_exclusion(),
            Some(StoredReferenceExclusion::ImpossibleSummary)
        );
    }

    #[test]
    fn fatal_events_and_events_outside_the_flight_exclude_stored_results() {
        let with_event = |time: &str, kind: &str| {
            let branch = format!(
                r#"<databranch name="Sustainer" types="Time,Altitude"><event time="{time}" type="{kind}"/><datapoint>0,0</datapoint><datapoint>5,100</datapoint></databranch>"#
            );
            stored_with_results(&results_with_branches("100", &branch)).reference_exclusion()
        };
        assert_eq!(
            with_event("6", "SIM_ABORT"),
            Some(StoredReferenceExclusion::FatalEvent)
        );
        assert_eq!(
            with_event("6", "Simulation-Abort"),
            Some(StoredReferenceExclusion::FatalEvent)
        );
        assert_eq!(
            with_event("-0.1", "launch"),
            Some(StoredReferenceExclusion::ImpossibleSummary)
        );
        assert_eq!(
            with_event("21", "simulationend"),
            Some(StoredReferenceExclusion::InconsistentResults)
        );
    }

    /// The override probes, each with how far hpr's centre of mass is from OpenRocket's, in metres
    /// (hpr's station less OpenRocket's). A zero is agreement; the last two are the departure
    /// ADR-061 keeps for a mass override that covers the parts inside and states no centre.
    const PRECEDENCE: [(&str, f64); 12] = [
        (
            "a centre of gravity override on a nose with a shoulder",
            0.0,
        ),
        (
            "a centre of gravity override on a transition with a fore shoulder",
            0.0,
        ),
        ("a mass override on a nose with a shoulder", 0.0),
        ("a mass override on a tube, not the part inside", 0.0),
        ("a mass override on the part inside only", 0.0),
        (
            "a centre of gravity override on a tube, not the part inside",
            0.0,
        ),
        (
            "a centre of gravity override on a tube and the part inside",
            0.0,
        ),
        ("a mass override on the stage", 0.0),
        ("both overrides on the stage", 0.0),
        ("a stage override over a part's own", 0.0),
        ("a mass override on a tube and the part inside", -0.003686),
        (
            "a mass override on a tube and the part inside, which has its own",
            -0.019690,
        ),
    ];

    /// Both overrides on one part, with flags that disagree, and the centre's distance as above.
    const DISAGREEING: [(&str, f64); 2] = [
        (
            "both overrides on a tube, the centre covering the part inside and the mass not",
            0.004672,
        ),
        (
            "both overrides on a tube, the mass covering the part inside and the centre not",
            0.0,
        ),
    ];

    /// Loft lesson L51: Loft's rule for which centre-of-gravity override wins came from
    /// OpenRocket's source and was unsettled by up to 133 mm. Here it is measured.
    ///
    /// On every override probe hpr's mass is OpenRocket's, so the winning mass override is the
    /// same one: a parent's override that covers its children wins over a child's own, and a
    /// stage's over everything in it. A centre of gravity override is measured from the part's
    /// front, not its shoulder's, and moves the shoulder with the part. A centre of gravity override
    /// alone that covers the parts inside sets the assembly's centre in both programs (how each
    /// places the parts inside differs, which shows only in the inertia: see
    /// `inertia_under_an_override_departs_as_written`).
    ///
    /// Two cases are hpr's own rule, written in ADR-061 and pinned here in metres:
    ///
    /// - a mass override that covers the parts inside and states no centre: OpenRocket puts the
    ///   centre at the overriding part's own, leaving out the parts inside; hpr keeps the
    ///   assembly's, as its parts lay it out;
    /// - both overrides, with flags that disagree: hpr states one scope and takes the mass's, and
    ///   says so.
    #[test]
    fn override_precedence_matches_oracle() {
        let record = record();
        for (question, apart_m) in PRECEDENCE {
            let (ours, theirs, warnings) = both(probe(&record, question));
            assert!(
                relative(ours[0], theirs[0]).abs() < 1e-5,
                "{question}: mass {ours:?} {theirs:?}"
            );
            assert!(
                (ours[1] - theirs[1] - apart_m).abs() < 1e-6,
                "{question}: centre {} m apart, not {apart_m}",
                ours[1] - theirs[1]
            );
            assert!(warnings.is_empty(), "{question}: {warnings:?}");
        }
        // Flags that disagree cannot be said in `hpr-design`; the mass flag decides, out loud.
        // Where the mass covers the parts inside, the centre lands on OpenRocket's anyway.
        for (question, apart_m) in DISAGREEING {
            let (ours, theirs, warnings) = both(probe(&record, question));
            assert!(relative(ours[0], theirs[0]).abs() < 1e-9, "{question}");
            assert!(
                (ours[1] - theirs[1] - apart_m).abs() < 1e-6,
                "{question}: centre {} m apart, not {apart_m}",
                ours[1] - theirs[1]
            );
            assert_eq!(warnings.len(), 1, "{question}: {warnings:?}");
            assert!(
                warnings[0].contains("the mass flag was taken"),
                "{warnings:?}"
            );
        }
    }

    /// The single subcomponent-override flag written before schema 1.9, and how far hpr's centre
    /// of mass is from OpenRocket's, in metres, on each probe of it.
    const OLD_FLAG: [(&str, f64); 11] = [
        (
            "the old flag on both of a stage's overrides, schema 1.4",
            0.0,
        ),
        (
            "the old flag on a tube's mass override, schema 1.4",
            -0.003686,
        ),
        (
            "the old flag on a tube's mass override, schema 1.8",
            -0.003686,
        ),
        (
            "the old flag, false, on a tube's mass override, schema 1.8",
            0.0,
        ),
        (
            "the old flag on a tube's mass override, schema 1.10",
            -0.003686,
        ),
        ("the old flag after a mass flag that says false", -0.003686),
        ("the old flag before a mass flag that says false", 0.0),
        ("the old flag before a drag flag that says false", -0.003686),
        ("the old flag after a drag flag that says false", -0.003686),
        (
            "the old flag, false, after a centre flag that says true",
            0.0,
        ),
        (
            "the old flag, false, before a centre flag that says true",
            0.0,
        ),
    ];

    /// The element at or under `element` whose `<id>` is `id`.
    fn with_id<'a>(element: &'a ork::Element, id: &str) -> Option<&'a ork::Element> {
        if element
            .child("id")
            .is_some_and(|own| own.text().trim() == id)
        {
            return Some(element);
        }
        element.elements().find_map(|child| with_id(child, id))
    }

    /// OpenRocket 24.12 reads the single flag the three per-quantity ones replaced before schema
    /// 1.9 as setting all three, in schema 1.4, 1.8 and 1.10 alike, and where a part writes both
    /// forms the later one wins, quantity by quantity. hpr reads it so (M2.2e6): part by part, the
    /// three flags are OpenRocket's, nothing is warned of, the mass is OpenRocket's, and the centre
    /// is OpenRocket's but where a mass override covering the parts inside states no centre, which
    /// is ADR-061's departure, the same 3.686 mm as the per-quantity flag's probe above.
    #[test]
    fn the_old_subcomponent_flag_reads_as_openrocket_reads_it() {
        let record = record();
        for (question, apart_m) in OLD_FLAG {
            let probe = probe(&record, question);
            let (ours, theirs, warnings) = both(probe);
            assert!(
                relative(ours[0], theirs[0]).abs() < 1e-9,
                "{question}: mass {ours:?} {theirs:?}"
            );
            assert!(
                (ours[1] - theirs[1] - apart_m).abs() < 1e-6,
                "{question}: centre {} m apart, not {apart_m}",
                ours[1] - theirs[1]
            );
            assert!(warnings.is_empty(), "{question}: {warnings:?}");

            let document = probe["document"].as_str().expect("the probe's document");
            let parsed = ork::read(document.as_bytes()).expect("a probe reads");
            let flags = probe["override_flags"].as_object().expect("the flags");
            assert!(flags.len() >= 3, "{question}: {flags:?}");
            for (id, recorded) in flags {
                let element = with_id(&parsed.value.document.root, id).expect("the part");
                let mut part_warnings = Vec::new();
                let overrides = ork::Values::new(element, "probe", &mut part_warnings).overrides();
                let flags_ours = [
                    overrides.subcomponents_mass,
                    overrides.subcomponents_cg,
                    overrides.subcomponents_cd,
                ]
                .map(Option::unwrap_or_default);
                let flags_theirs =
                    ["mass", "cg", "cd"].map(|k| recorded[k].as_bool().expect("a flag"));
                assert_eq!(flags_ours, flags_theirs, "{question}: {id}");
                assert!(part_warnings.is_empty(), "{question}: {part_warnings:?}");
            }
        }
    }

    /// The probes whose inertias are compared, with hpr's roll and pitch relative to OpenRocket's.
    const INERTIA: [(&str, [f64; 2]); 11] = [
        ("a mass override on a nose with a shoulder", [0.0, 0.0]),
        ("a mass override on a tube, not the part inside", [0.0, 0.0]),
        ("a mass override on the part inside only", [0.0, 0.0]),
        (
            "a mass override on a tube and the part inside",
            [-0.0693, -0.0667],
        ),
        (
            "a mass override on a tube and the part inside, which has its own",
            [-0.3713, -0.3724],
        ),
        (
            "a centre of gravity override on a tube and the part inside",
            [0.0, -0.0265],
        ),
        (
            "both overrides on a tube, the centre covering the part inside and the mass not",
            [0.0, -0.0010],
        ),
        (
            "both overrides on a tube, the mass covering the part inside and the centre not",
            [-0.0693, -0.0818],
        ),
        ("a mass override on the stage", [4.0050, 4.0050]),
        ("both overrides on the stage", [4.0050, 3.4526]),
        ("a stage override over a part's own", [1.4756, 1.4756]),
    ];

    /// Inertia under an override is hpr's own rule, a departure written in ADR-061. hpr scales a
    /// mass override's inertia with its mass, over everything the override covers, so the extra
    /// mass sits where the parts' does; OpenRocket scales only the overriding part's own inertia,
    /// keeps the inertias of the parts inside (a stage has none of its own, so nothing is scaled),
    /// and leaves the covered parts' masses out. For a centre of gravity override that covers the
    /// parts inside, hpr moves the assembly whole; OpenRocket moves the part alone and adds the
    /// parts inside where they were. On a lone part both scale, and agree. The relative
    /// differences, hpr's to OpenRocket's, are pinned here as roll and pitch.
    #[test]
    fn inertia_under_an_override_departs_as_written() {
        let record = record();
        for (question, pinned) in INERTIA {
            let (ours, theirs, _) = both(probe(&record, question));
            let found = [2, 3].map(|k| (relative(ours[k], theirs[k]) * 1e4).round() / 1e4);
            assert_eq!(found, pinned, "{question}");
        }
    }

    /// The probes whose readings hpr takes from OpenRocket, where no rule of hpr's own is in play.
    const READINGS: [&str; 18] = [
        "a nose with no shoulder",
        "a nose with a walled shoulder",
        "a nose whose shoulder has no wall",
        "a nose whose shoulder has no wall, capped",
        "a filled nose whose shoulder has no wall",
        "a filled nose with a walled shoulder",
        "a transition whose shoulders have no wall",
        "a nose of no wall",
        "a transition of no wall",
        "a tube of no wall",
        "a tube holding an inner tube, a coupler and a lug of no wall",
        "a mass override on a nose of no wall",
        "a nose, a shoulder and a tube that write no thickness at all",
        "a narrower nose and tube that write no thickness at all",
        "a transition that writes no thickness at all",
        "a nose and a tube that name no material, with one part of each kind inside",
        "a transition, and more kinds inside a tube, that name no material",
        // Not a reading hpr takes: pinned below.
        "a tube holding an inner tube, a coupler and a lug that write no thickness",
    ];

    /// The gaps a reading probe shows, each pinned rather than hidden: a part's class, and how far
    /// hpr's is from OpenRocket's, as its mass relative to OpenRocket's and its station in metres.
    ///
    /// - An elliptical fin set: hpr's planform is the exact ellipse, `π c h / 4`; OpenRocket's
    ///   weighs 0.18% less, which matches a 30-sided polygon inscribed at equal angles,
    ///   `(π/30) / sin(π/30) − 1`, to 13 digits: an inference from its output, not its source
    ///   (M2.2b2, with the fins).
    const PART_GAPS: [(&str, f64, f64); 1] = [("EllipticalFinSet", 0.001830, 0.0)];

    /// hpr reads OpenRocket's words for walls, shoulders and materials as OpenRocket 24.12 does
    /// (ADR-061): a wall or a shoulder of no thickness weighs nothing, capped or not, on a filled
    /// nose or a hollow one, and so does an inner tube, coupler or lug of no wall; a nose,
    /// transition or tube that writes no thickness has a 2 mm wall, and a shoulder that writes none
    /// has no wall; a part that names no material is made of OpenRocket's cardboard, ripstop nylon,
    /// elastic cord or (a rail button) Delrin; and a mass override on a part that weighs nothing is
    /// a point mass at the middle of its length, unless the part is packed (ADR-063, below). So the
    /// mass and the centre of mass agree, part by part as well as whole, and nothing is warned of;
    /// where no fin, rail button or recovery part is in the probe, the inertias agree as well.
    ///
    /// The worst mass is a transition's, 2.9e-6 from OpenRocket's (hpr measures its wall normal to
    /// the surface); the worst centre, 1.5e-7 m. The bounds are a few times those. The two gaps in
    /// [`PART_GAPS`] are pinned, and so is the one reading hpr does not take: an inner tube or lug
    /// that writes no thickness, which OpenRocket gives a wall of its own (0.5 mm on the 20 mm
    /// inner tube, 1 mm on the 5 mm lug, none on the coupler) and hpr reads as none, with a warning.
    #[test]
    fn walls_shoulders_and_unnamed_materials_read_as_openrocket_does() {
        let record = record();
        for question in READINGS {
            let probe = probe(&record, question);
            let unwritten = question.ends_with("that write no thickness");
            let (ours, theirs, warnings) = both(probe);
            if unwritten {
                assert_eq!(warnings.len(), 3, "{question}: {warnings:?}");
            } else {
                assert!(warnings.is_empty(), "{question}: {warnings:?}");
            }
            if theirs[0] == 0.0 {
                // A structure that weighs nothing has no centre to compare.
                assert_eq!(ours[0], 0.0, "{question}");
                assert!(ours[1].is_finite(), "{question}");
                continue;
            }
            let (layout, _) = hpr(probe);
            // The whole is the parts: whatever the parts' gaps add up to, and nothing more.
            let mut gap_kg = 0.0;
            let mut gap_moment = 0.0;
            let mut plain = true;
            for part in probe["parts"].as_array().expect("parts") {
                let class = part["class"].as_str().expect("a class");
                if matches!(class, "Rocket" | "AxialStage") {
                    continue; // they weigh nothing of their own, and hpr lays out no part for them
                }
                let id = part["id"].as_str().expect("an id");
                let (_, placed) = layout
                    .find(id)
                    .unwrap_or_else(|| panic!("{question}: no part {id} in hpr"));
                let mass = part["mass_kg"].as_f64().expect("a mass");
                let station = part["cm_x_m"].as_f64().expect("a station");
                plain &= !matches!(
                    class,
                    "TrapezoidFinSet"
                        | "EllipticalFinSet"
                        | "FreeformFinSet"
                        | "RailButton"
                        | "Parachute"
                        | "Streamer"
                        | "ShockCord"
                );
                let (mass_gap, station_gap) =
                    if unwritten && matches!(class, "InnerTube" | "LaunchLug") {
                        // OpenRocket's own wall, which hpr does not give: all of the part's mass.
                        (-1.0, 0.0)
                    } else {
                        PART_GAPS
                            .iter()
                            .find(|(kind, _, _)| *kind == class)
                            .map_or((0.0, 0.0), |(_, m, z)| (*m, *z))
                    };
                if mass == 0.0 {
                    assert_eq!(placed.own.mass_kg, 0.0, "{question}: {id}");
                    continue;
                }
                let found = relative(placed.own.mass_kg, mass);
                assert!(
                    (found - mass_gap).abs() < 1e-5,
                    "{question}: {id} is {found} from OpenRocket's mass, not {mass_gap}"
                );
                if placed.own.mass_kg > 0.0 {
                    let apart_m = -placed.own.cg_m.z - station;
                    assert!(
                        (apart_m - station_gap).abs() < 1e-6,
                        "{question}: {id} is {apart_m} m from OpenRocket's station"
                    );
                }
                gap_kg += placed.own.mass_kg - mass;
                gap_moment += placed.own.mass_kg * (-placed.own.cg_m.z) - mass * station;
            }
            assert!(
                (ours[0] - theirs[0] - gap_kg).abs() < 1e-5 * theirs[0],
                "{question}: mass {ours:?} {theirs:?}"
            );
            let expected_m = (theirs[0] * theirs[1] + gap_moment) / (theirs[0] + gap_kg);
            assert!(
                (ours[1] - expected_m).abs() < 1e-6,
                "{question}: centre {ours:?} {theirs:?}"
            );
            if plain && !unwritten {
                for k in [2, 3] {
                    let (a, b) = (ours[k], theirs[k]);
                    assert!(
                        (a - b).abs() <= 1e-5 * b.abs().max(1e-12),
                        "{question}: inertia {ours:?} {theirs:?}"
                    );
                }
            }
        }
    }

    /// The probes of one tube and one part (M2.2b2, [ADR-062][adr-062]), each with how far hpr's
    /// structure is from OpenRocket's: the mass (relative), the centre of mass (metres, hpr's less
    /// OpenRocket's), the roll inertia (relative) with OpenRocket's fin rule on OpenRocket's own fin
    /// mass in place of hpr's ([`super::openrocket_fin_set_roll_kg_m2`]), and the pitch inertia
    /// (relative). Each is pinned to three figures, and a zero to 1e-12. The tube alone is
    /// OpenRocket's, so each row is its one part's:
    ///
    /// - A bulkhead, centering ring, inner tube, mass component, parachute (under an override too),
    ///   shock cord and streamer: OpenRocket's in all four.
    /// - A fin set: its roll is OpenRocket's rule exactly, whatever its outline, section, tab or
    ///   fillets, but for an ellipse (OpenRocket's is a 30-sided polygon) and a cant (−2.66e-5).
    ///   Its mass is OpenRocket's but for a rounded or airfoil section
    ///   ([`a_fin_section_is_weighed_as_pinned`]), an ellipse, and a cant (−4.19e-5, not traced).
    ///   Its pitch inertia is apart by up to 0.11% where the masses agree (0.406% on a single fin,
    ///   below): OpenRocket's pitch rule for fins is not measured here.
    /// - Fillets (M2.2e7, [ADR-096][adr-096]): OpenRocket's in mass and centre to 1e-15 (held
    ///   here to 1e-12), in their own material or cardboard's when none is named, on any outline,
    ///   count or tube; before, hpr left them out (−0.808% and −2.79% of the probe's mass at 5 and
    ///   10 mm). The pitch inertia is apart by −0.0077% to −0.638%, largest with the 30 mm fillets,
    ///   and +0.425% on a single fin: hpr's is the exact prism, and OpenRocket's pitch rule for fins
    ///   is not measured.
    /// - A rail button, one or a row, from any end: OpenRocket's in mass and centre (#151), its
    ///   inertias apart by 7.16e-6 and 9.23e-5 (one), 1.43e-5 and 4.99e-4 (two).
    /// - A launch lug: its pitch inertia is apart by 3.13e-4.
    /// - A packed part that writes no packed size, or only half of one: OpenRocket packs it 25 mm
    ///   long and 12.5 mm in radius, whatever the tube, and so does hpr now (ADR-063; before, hpr
    ///   read zero and was 0.611 mm off in the centre, −0.167% in roll).
    /// - A packed part that weighs nothing, under a mass override: OpenRocket spreads the override
    ///   over its packing, `m r²/2` in roll, and so does hpr now (ADR-063; before, a point mass,
    ///   −0.805%).
    /// - A single fin: the rule about the fin's own centre, `m hₑ²/12`, agrees; its pitch is
    ///   apart by 0.406%.
    /// - A 3-ring clustered inner tube: hpr reads all three tubes (M1.9b), so the mass and centre
    ///   agree; the roll inertia is +5.11% and the pitch +0.273%, the tubes' parallel-axis terms
    ///   `3 m d²` and half of it, which OpenRocket leaves out (ADR-075). Before M1.9b hpr read one
    ///   tube: −12.85% mass, +5.95 mm centre, −2.43% roll and −3.68% pitch.
    ///
    /// [adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21
    /// [adr-064]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-064-clusters-fillets-and-unread-parts-remain-visible-departures-2026-09-22
    /// [adr-096]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-096-fin-fillets-and-an-automatic-radius-inside-a-nose-cone-read-as-openrocket-reads-them-2026-09-28
    const ALONE: [(&str, [f64; 4]); 50] = [
        ("a tube and a bulkhead", [0.0, 0.0, 0.0, 0.0]),
        (
            "a tube and a canted fin set",
            [-4.19e-5, -4.54e-6, -2.66e-5, 7.58e-5],
        ),
        ("a tube and a centering ring", [0.0, 0.0, 0.0, 0.0]),
        (
            "a tube and a fin set of airfoil section",
            [-0.0165, 0.000935, 0.0, -0.00488],
        ),
        (
            "a tube and a fin set of rounded section",
            [0.00014, -1.24e-5, 0.0, 7.37e-5],
        ),
        (
            "a tube and a fin set of square section",
            [0.0, 0.0, 0.0, 4.57e-5],
        ),
        ("a tube and a fin set with a tab", [0.0, 0.0, 0.0, -0.00109]),
        (
            "a tube and a fin set with fillets",
            [0.0, 0.0, 0.0, -0.00035],
        ),
        (
            "a tube and a fin set with wider fillets",
            [0.0, 0.0, 0.0, -0.00129],
        ),
        (
            "a tube and a fin set with fillets of 30 mm",
            [0.0, 0.0, 0.0, -0.00638],
        ),
        (
            "a tube and a fin set with fillets of their own material",
            [0.0, 0.0, 0.0, -0.000749],
        ),
        (
            "a tube and a fin set with fillets that name no material",
            [0.0, 0.0, 0.0, -0.000223],
        ),
        (
            "a tube and a single fin with fillets",
            [0.0, 0.0, 7.78e-6, 0.00425],
        ),
        (
            "a tube and four fins of rounded section with fillets",
            [0.000179, -1.52e-5, 0.0, -0.000428],
        ),
        (
            "a tube and a freeform fin set with fillets",
            [0.0, 0.0, 0.0, -7.74e-5],
        ),
        (
            "a wider tube and a fin set with fillets",
            [0.0, 0.0, 0.0, -0.00123],
        ),
        ("a tube and a freeform fin set", [0.0, 0.0, 0.0, 4.57e-5]),
        ("a tube and a launch lug", [0.0, 0.0, 0.0, 0.000313]),
        ("a tube and a mass component", [0.0, 0.0, 0.0, 0.0]),
        ("a tube and a parachute", [0.0, 0.0, 0.0, 0.0]),
        (
            "a tube and a parachute that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a parachute that writes only a packed length",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a parachute that writes only a packed radius",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a streamer that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a shock cord that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a mass component that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a mass component of no mass, under a mass override",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a shock cord of no length, under a mass override",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a wider tube and a parachute that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a narrow tube and a parachute that writes no packed size",
            [0.0, 0.0, 0.0, 0.0],
        ),
        (
            "a tube and a parachute with a mass override",
            [0.0, 0.0, 0.0, 0.0],
        ),
        ("a tube and a rail button", [0.0, 0.0, 7.16e-6, 9.23e-5]),
        (
            "a tube and a rail button from the bottom",
            [0.0, 0.0, 7.16e-6, 9.23e-5],
        ),
        (
            "a tube and a rail button from the middle",
            [0.0, 0.0, 7.16e-6, 9.24e-5],
        ),
        (
            "a tube and a row of two rail buttons from the bottom",
            [0.0, 0.0, 1.43e-5, 0.000497],
        ),
        (
            "a tube and a row of two rail buttons from the middle",
            [0.0, 0.0, 1.43e-5, 0.000499],
        ),
        (
            "a tube and a row of two rail buttons from the top",
            [0.0, 0.0, 1.43e-5, 0.000499],
        ),
        ("a tube and a shock cord", [0.0, 0.0, 0.0, 0.0]),
        ("a tube and a streamer", [0.0, 0.0, 0.0, 0.0]),
        (
            "a tube and a thicker fin set of airfoil section",
            [-0.0305, 0.00155, 0.0, -0.00842],
        ),
        (
            "a tube and an elliptical fin set",
            [0.000188, -1.69e-5, 0.000122, -0.000328],
        ),
        ("a tube and an inner tube", [0.0, 0.0, 0.0, 0.0]),
        (
            "a tube and a clustered inner tube",
            [0.0, 0.0, 0.0511, 0.00273],
        ),
        ("a tube and rectangular fins", [0.0, 0.0, 0.0, 2.3e-6]),
        (
            "a tube and rectangular fins of twice the chord",
            [0.0, 0.0, 0.0, 4.49e-6],
        ),
        (
            "a tube and rectangular fins of twice the span",
            [0.0, 0.0, 0.0, 4.2e-6],
        ),
        ("a tube and a single fin", [0.0, 0.0, 0.0, 0.00406]),
        (
            "a tube and a parachute of no canopy, under a mass override",
            [0.0, 0.0, 0.0, 0.0],
        ),
        ("a tube and triangular fins", [0.0, 0.0, 0.0, 0.00028]),
        ("a wider tube and rectangular fins", [0.0, 0.0, 0.0, 1e-6]),
    ];

    #[test]
    fn each_part_alone_is_openrocket_s_or_pinned() {
        let record = record();
        for (question, pinned) in ALONE {
            let probe = probe(&record, question);
            let (ours, theirs, warnings) = both(probe);
            assert!(warnings.is_empty(), "{question}: {warnings:?}");
            let (layout, _) = hpr(probe);
            let mut roll = ours[2];
            for part in probe["parts"].as_array().expect("parts") {
                let Some((_, placed)) = part["id"].as_str().and_then(|id| layout.find(id)) else {
                    continue;
                };
                if let Part::FinSet(fins) = &placed.part {
                    let mass_kg = part["mass_kg"].as_f64().expect("a mass");
                    let radius_m = placed.body_radius_m.expect("fins sit on a tube");
                    let rule = super::openrocket_fin_set_roll_kg_m2(fins, radius_m, mass_kg)
                        .expect("three fins");
                    roll += rule - placed.own.inertia_kg_m2.z_axis.z;
                }
            }
            let found = [
                relative(ours[0], theirs[0]),
                ours[1] - theirs[1],
                relative(roll, theirs[2]),
                relative(ours[3], theirs[3]),
            ];
            for (k, (found, pinned)) in found.iter().zip(pinned).enumerate() {
                let bound = if pinned == 0.0 {
                    1e-12
                } else {
                    5e-3 * pinned.abs()
                };
                assert!(
                    (found - pinned).abs() <= bound,
                    "{question}: quantity {k} is {found:e}, not {pinned:e}"
                );
            }
        }
    }

    /// hpr's own fin roll inertia against OpenRocket's, set by set: the departure
    /// [ADR-062][adr-062] keeps. OpenRocket's set is its structure's roll inertia less everything
    /// else, which is hpr's (every other part of these probes is OpenRocket's to 1e-15). Pinned to
    /// three figures: the guide's table quotes them.
    ///
    /// [adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21
    #[test]
    fn hpr_s_own_fin_roll_departs_as_pinned() {
        let record = record();
        let departures = [
            ("a tube and rectangular fins", 0.000_129),
            ("a tube and a fin set of square section", 0.0241),
            ("a tube and triangular fins", -0.0214),
            ("a tube and a fin set with a tab", -0.0512),
        ];
        for (question, pinned) in departures {
            let probe = probe(&record, question);
            let (ours, theirs, _) = both(probe);
            let (layout, _) = hpr(probe);
            let fins = layout
                .components
                .iter()
                .find(|placed| matches!(placed.part, Part::FinSet(_)))
                .expect("a fin set");
            let hpr_fins = fins.own.inertia_kg_m2.z_axis.z;
            let their_fins = theirs[2] - (ours[2] - hpr_fins);
            let found = relative(hpr_fins, their_fins);
            assert!(
                (found - pinned).abs() <= 5e-3 * pinned.abs(),
                "{question}: {found:e}, not {pinned:e}"
            );
        }
    }

    /// OpenRocket weighs a fin set as its outline times its thickness times a factor for its
    /// section, whatever the thickness: 1 for square, 0.99 for rounded and 0.85 for airfoil (read from
    /// its output, at 3 mm and 6 mm for the airfoil). hpr integrates the section: a rounded edge is a
    /// semicircle, and an airfoil is NACA's four-digit section, `0.6851 t c` (Abbott and von
    /// Doenhoff). hpr keeps its own ([ADR-062][adr-062]); the ratios are pinned.
    ///
    /// [adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21
    #[test]
    fn a_fin_section_is_weighed_as_pinned() {
        let record = record();
        let fins = |question: &str| -> (f64, f64) {
            let probe = probe(&record, question);
            let (layout, _) = hpr(probe);
            let theirs = probe["parts"]
                .as_array()
                .expect("parts")
                .iter()
                .find(|part| part["class"] == "TrapezoidFinSet")
                .expect("a fin set");
            let id = theirs["id"].as_str().expect("an id");
            let (_, ours) = layout.find(id).expect("hpr's fin set");
            (
                ours.own.mass_kg,
                theirs["mass_kg"].as_f64().expect("a mass"),
            )
        };
        let square = fins("a tube and a fin set of square section");
        assert!((square.0 - square.1).abs() < 1e-15, "{square:?}");
        let sections = [
            (
                "a tube and a fin set of rounded section",
                1.0,
                0.99,
                0.991_416,
            ),
            (
                "a tube and a fin set of airfoil section",
                1.0,
                0.85,
                0.685_083,
            ),
            (
                "a tube and a thicker fin set of airfoil section",
                2.0,
                0.85,
                0.685_083,
            ),
        ];
        for (question, thickness, openrocket, ours) in sections {
            let (hpr, theirs) = fins(question);
            let slab = thickness * square.1;
            assert!(
                (theirs / slab - openrocket).abs() < 1e-12,
                "{question}: {}",
                theirs / slab
            );
            assert!(
                (hpr / slab - ours).abs() < 1e-6,
                "{question}: {}",
                hpr / slab
            );
        }
    }

    /// Loft's `demo-boattail.ork`, whose roll inertia is 2.64% from OpenRocket's, 0.093% with
    /// OpenRocket's fin rule in place of hpr's (`cargo xtask ork`). Its fins are elliptical, and
    /// OpenRocket's ellipse is a 30-sided polygon, of `sin(π/30) / (π/30)` the area and so the
    /// mass: with that in the rule too, the two are 1.5e-6 apart. The design and OpenRocket's
    /// record of it are both committed (ADR-060).
    #[test]
    fn the_loft_boattail_s_roll_is_the_rule_on_openrocket_s_ellipse() {
        let text = include_str!("../../../validation/fixtures/ork/openrocket-mass-loft-demo.json");
        let record: Value = serde_json::from_str(text).expect("the committed record is JSON");
        let file = "validation/fixtures/ork/loft-demo/demo-boattail.ork";
        let design = record["designs"]
            .as_array()
            .expect("designs")
            .iter()
            .find(|design| design["file"] == file)
            .expect("the boattail's record");
        let theirs = design["structure"]["ixx"].as_f64().expect("a roll inertia");
        let bytes = include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-boattail.ork");
        let read = ork::read(bytes.as_slice()).expect("the design reads");
        let layout = ork::design(&read.value)
            .value
            .rocket
            .layout()
            .expect("it lays out");
        let polygon = (std::f64::consts::PI / 30.0).sin() / (std::f64::consts::PI / 30.0);
        let mut ours = layout.structure.inertia_kg_m2.z_axis.z;
        let mut fins = 0;
        for placed in &layout.components {
            let Part::FinSet(set) = &placed.part else {
                continue;
            };
            assert!(matches!(
                set.planform,
                hpr_design::fins::FinPlanform::Elliptical { .. }
            ));
            // The rule on the polygon: its mass, and its area in `hₑ² = A h / c_r`.
            let rule = super::rod_roll_kg_m2(
                set.count,
                set.planform.geometry().expect("an outline").area_m2 * polygon,
                set.planform.span_m(),
                set.planform.root_chord_m(),
                placed.body_radius_m.expect("fins sit on a tube"),
                placed.own.mass_kg * polygon,
            )
            .expect("three fins");
            ours += rule - placed.own.inertia_kg_m2.z_axis.z;
            fins += 1;
        }
        assert_eq!(fins, 1);
        let apart = relative(ours, theirs);
        assert!(apart.abs() < 2e-6, "{apart:e}");
    }

    fn pods() -> Value {
        let text = include_str!("../../../validation/fixtures/ork/openrocket-pods.json");
        serde_json::from_str(text).expect("the committed record is JSON")
    }

    /// Every part inside every pod OpenRocket 24.12 was asked about sits where OpenRocket puts it:
    /// each copy across the axis to 1e-15 m, with OpenRocket's `(y, z)` read as hpr's `(x, y)`,
    /// and along it to 1e-15 m. That holds the reader's distance from the axis for each
    /// `radiusoffset` method (`relative`, `surface`, `free`), a pod whose widest part is in its
    /// middle, one, two and three pods, a negative angle, every angle method, and a pod set placed
    /// from its tube's bottom (M1.13b). A pod of no length holds its fins and its lug on a tube of
    /// its own radius, most often none, turned with the pod: each fin's root and the lug's axis are
    /// where OpenRocket puts them, matched as sets, since OpenRocket lists a fin set's fins across
    /// its pods in an order of its own. On a tube 10 mm in radius, two pods at 30° with three fins
    /// each at 20° show every fin turned with its pod (M1.13b2).
    #[test]
    fn every_pod_is_where_openrocket_puts_it() {
        let record = pods();
        let probes = record["probes"].as_object().expect("probes");
        assert_eq!(probes.len(), 18);
        for (question, probe) in probes {
            let (layout, warnings) = hpr(probe);
            if question.ends_with("its mass overridden to 0.1 kg") {
                assert_eq!(warnings.len(), 1, "{question}: {warnings:?}");
                assert!(warnings[0].contains("at the rocket's tip"), "{warnings:?}");
            } else {
                assert!(warnings.is_empty(), "{question}: {warnings:?}");
            }
            let mut inside = 0;
            for (id, theirs) in probe["components"].as_object().expect("components") {
                let Some((_, placed)) = layout.find(id) else {
                    continue;
                };
                if !in_a_pod(&layout, id) {
                    continue;
                }
                inside += 1;
                let theirs: Vec<[f64; 3]> = theirs["locations_m"]
                    .as_array()
                    .expect("locations")
                    .iter()
                    .map(|l| {
                        let at = |k: usize| l[k].as_f64().expect("a number");
                        [at(1), at(2), at(0)]
                    })
                    .collect();
                let ours = where_hpr_puts(placed);
                assert_eq!(ours.len(), theirs.len(), "{question}: {id}");
                let near = |a: &[f64; 3], b: &[f64; 3]| {
                    (a[0] - b[0]).hypot(a[1] - b[1]) <= 1e-15 && (a[2] - b[2]).abs() <= 1e-15
                };
                if matches!(placed.part, Part::FinSet(_)) {
                    let mut left = ours.clone();
                    for place in &theirs {
                        let k = left.iter().position(|o| near(o, place));
                        let k = k.unwrap_or_else(|| panic!("{question}: {id} {place:?} {ours:?}"));
                        left.swap_remove(k);
                    }
                } else {
                    for (o, t) in ours.iter().zip(&theirs) {
                        assert!(near(o, t), "{question}: {id} {o:?} vs {t:?}");
                    }
                }
            }
            let least = if question.starts_with("an empty") {
                0
            } else {
                2
            };
            assert!(inside >= least, "{question}: {inside} parts in the pod");
        }
    }

    /// Whether the part `id` hangs, at any depth, from a pod set.
    fn in_a_pod(layout: &Layout, id: &str) -> bool {
        let mut at = layout.find(id).expect("the part").1.parent;
        while let Some(index) = at {
            let parent = &layout.components[index];
            if matches!(parent.part, Part::PodSet(_)) {
                return true;
            }
            at = parent.parent;
        }
        false
    }

    /// Where hpr puts a part in a pod, as OpenRocket's component locations name it: `[x, y]`
    /// across the axis and the station along it, for every copy. A body component is its fore
    /// end on its pod's axis; a fin is its root's leading edge, on its tube's surface at its own
    /// angle; a lug is its axis's fore end, its own radius out from its tube's surface. What a pod
    /// holds turns with its pod.
    fn where_hpr_puts(placed: &hpr_design::tree::PlacedComponent) -> Vec<[f64; 3]> {
        let fore = placed.fore_station_m;
        let out = |offset: [f64; 2], radius: f64, angle: f64| {
            [
                offset[0] + radius * angle.cos(),
                offset[1] + radius * angle.sin(),
                fore,
            ]
        };
        let body = placed.body_radius_m.unwrap_or_default();
        let mut places = Vec::new();
        for copy in &placed.copies {
            match &placed.part {
                Part::FinSet(set) => {
                    let step = std::f64::consts::TAU / f64::from(set.count);
                    for k in 0..set.count {
                        let angle = copy.roll_rad + set.base_angle_rad + step * f64::from(k);
                        places.push(out(copy.offset_m, body, angle));
                    }
                }
                Part::LaunchLug(lug) => {
                    let angle = copy.roll_rad + lug.angle_rad;
                    places.push(out(copy.offset_m, body + lug.outer_radius_m, angle));
                }
                _ => places.push(out(copy.offset_m, 0.0, 0.0)),
            }
        }
        places
    }

    /// A design with pods weighs what OpenRocket 24.12 says, on every probe.
    ///
    /// - **Part by part.** Every part OpenRocket weighs in a pod, and the pod set itself, is found
    ///   in hpr with OpenRocket's mass, all its copies together, and its centre along the axis: a
    ///   tube, a fin set or a lug within 2e-15, relative (9.5e-16 at worst), so each pod is
    ///   weighed where it sits (ADR-089), a pod of no length and an empty pod set weigh nothing,
    ///   and a pod's fins and lug weigh what OpenRocket's do. A pod's nose cone is 8.3e-8 from
    ///   OpenRocket's in mass and 4.0e-9 in its centre, held within 1e-7 and 5e-9. OpenRocket
    ///   weighs a nose cone differently from hpr: the probes' own nose, on the airframe, is
    ///   5.10e-7 apart in mass and −4.64e-7 in its centre, pinned here to 0.5%, and the
    ///   airframe's tube agrees to 2e-15.
    /// - **The whole.** The empty pod set's probe is the bare airframe: 1.17e-7 from OpenRocket's
    ///   in mass and −1.125e-7 in the centre, relative, pinned. No probe's mass
    ///   or centre is further from OpenRocket's than that, so no pod here adds a gap of its own.
    ///   The mass is within 1.2e-7 on every probe; the centre within 1.1e-7 where the pods have
    ///   a length (M1.13b1's), and within 1.13e-7 where they have none and weigh little.
    /// - **Roll**, within 2e-9, with each fin set in a pod given OpenRocket's own roll inertia
    ///   about its pod's axis ([`super::openrocket_fin_set_roll_kg_m2`], ADR-062) in place of hpr's.
    /// - **Pitch.** OpenRocket gives one pitch inertia, the same number as `iyy` and `izz`. Where
    ///   the pods hold no fins it is hpr's inertia about `x_B` (OpenRocket's `y`) within 6.1e-7,
    ///   and so the mean across the axis only where the pods leave the two equal (three pods):
    ///   with one or two of M1.13b1's pods, OpenRocket's `izz` is not the inertia about its `z`
    ///   axis, and hpr's is, 0.3% to 1.1% apart. Where they hold fins, the gap about `x_B` is
    ///   pinned to 0.5% of itself, as on the airframe: OpenRocket's pitch rule for fins is not
    ///   measured.
    /// - **An override on an empty pod set.** OpenRocket adds its mass at the rocket's tip, on the
    ///   axis: its structure is the bare airframe's but 0.1 kg heavier, with the same first
    ///   moment and roll inertia. hpr's reader drops the override, with a warning, so hpr's is the
    ///   bare airframe exactly.
    #[test]
    fn pods_weigh_as_openrocket_s() {
        // Mass and centre, relative: every tube, fin set and lug in a pod; a pod's nose cone.
        const PART: (f64, f64) = (2e-15, 2e-15);
        const NOSE: (f64, f64) = (1e-7, 5e-9);
        // hpr's pitch about `x_B` less OpenRocket's, relative, where the pods hold fins.
        const FIN_PITCH: [(&str, f64); 4] = [
            (
                "one pod of no length 0.01 in radius, relative 0.02, two fins at 90",
                3.35e-5,
            ),
            (
                "one pod of no length, relative 0.03, bottom 0.0, two fins at 90",
                9.35e-6,
            ),
            (
                "two pods of no length 0.01 in radius, relative 0.02, at 30, three fins at 20",
                2.58e-7,
            ),
            (
                "two pods of no length, relative 0.02, at 45, three fins",
                -9.05e-7,
            ),
        ];
        let pinned = |got: f64, want: f64| (got - want).abs() <= 0.005 * want.abs();
        let record = pods();
        let probes = record["probes"].as_object().expect("probes");
        let empty = &probes["an empty pod set, relative 0.004"];
        let (ours, theirs, _) = both(empty);
        let bare = [relative(ours[0], theirs[0]), relative(ours[1], theirs[1])];
        assert!(pinned(bare[0], 1.17e-7), "{bare:?}");
        assert!(pinned(bare[1], -1.125e-7), "{bare:?}");
        let (layout, _) = hpr(empty);
        let nose = &empty["parts"][0];
        assert_eq!(nose["class"], "NoseCone");
        let (_, placed) = layout.find(conventions_id(1)).expect("the nose");
        let apart = relative(
            placed.own.mass_kg,
            nose["mass_kg"].as_f64().expect("a mass"),
        );
        assert!(pinned(apart, 5.10e-7), "{apart:e}");
        let apart = relative(
            -placed.own.cg_m.z,
            nose["cm_x_m"].as_f64().expect("a station"),
        );
        assert!(pinned(apart, -4.64e-7), "{apart:e}");
        // The tube is OpenRocket's, so the nose is all of the bare airframe's gap.
        let tube = &empty["parts"][1];
        assert_eq!(tube["class"], "BodyTube");
        let (_, placed) = layout.find(conventions_id(2)).expect("the tube");
        let apart = relative(
            placed.own.mass_kg,
            tube["mass_kg"].as_f64().expect("a mass"),
        );
        assert!(apart.abs() < PART.0, "{apart:e}");

        for (question, probe) in probes {
            let (ours, theirs, _) = both(probe);
            let (layout, _) = hpr(probe);
            if question.ends_with("its mass overridden to 0.1 kg") {
                let (bare_ours, bare_theirs, _) = both(empty);
                assert_eq!(ours, bare_ours, "{question}");
                let mass = theirs[0] - bare_theirs[0];
                assert!((mass - 0.1).abs() < 1e-15, "{question}: {mass:e}");
                let moment = theirs[0] * theirs[1] - bare_theirs[0] * bare_theirs[1];
                assert!(moment.abs() < 1e-15, "{question}: {moment:e}");
                assert_eq!(theirs[2], bare_theirs[2], "{question}");
                continue;
            }
            let airframe = [conventions_id(1), conventions_id(2)];
            let mut weighed = 0;
            let mut in_pods = 0;
            for part in probe["parts"].as_array().expect("parts") {
                let id = part["id"].as_str().expect("an id");
                let class = part["class"].as_str().expect("a class");
                if matches!(class, "Rocket" | "AxialStage") || airframe.contains(&id) {
                    continue;
                }
                in_pods += 1;
                let (_, placed) = layout
                    .find(id)
                    .unwrap_or_else(|| panic!("{question}: no part {id} in hpr"));
                assert!(
                    in_a_pod(&layout, id) || matches!(placed.part, Part::PodSet(_)),
                    "{question}: {id}"
                );
                weighed += 1;
                let mass = part["mass_kg"].as_f64().expect("a mass");
                let own = placed.own;
                if mass == 0.0 {
                    assert_eq!(own.mass_kg, 0.0, "{question}: {id}");
                    continue;
                }
                let (mass_bound, centre_bound) = if matches!(placed.part, Part::NoseCone(_)) {
                    NOSE
                } else {
                    PART
                };
                let apart = relative(own.mass_kg, mass);
                assert!(apart.abs() < mass_bound, "{question}: {id} mass {apart:e}");
                let station = part["cm_x_m"].as_f64().expect("a station");
                let apart = relative(-own.cg_m.z, station);
                assert!(
                    apart.abs() < centre_bound,
                    "{question}: {id} centre {apart:e}"
                );
            }
            assert_eq!(weighed, in_pods, "{question}");
            let holds_nothing = question.starts_with("an empty");
            let least = if holds_nothing { 1 } else { 3 };
            assert!(in_pods >= least, "{question}: {in_pods}");

            let of_length = !(holds_nothing || question.contains("of no length"));
            let centre_bound = if of_length { 1.1e-7 } else { 1.13e-7 };
            for (k, bound) in [(0, 1.2e-7), (1, centre_bound)] {
                let apart = relative(ours[k], theirs[k]);
                assert!(apart.abs() < bound, "{question}: quantity {k}, {apart:e}");
                assert!(
                    apart.abs() <= bare[k].abs(),
                    "{question}: quantity {k}, {apart:e}"
                );
            }

            let mut roll = ours[2];
            for placed in &layout.components {
                let Part::FinSet(set) = &placed.part else {
                    continue;
                };
                let radius = placed.body_radius_m.expect("fins sit on a tube");
                let one = placed
                    .part
                    .mass_properties(Some(radius))
                    .expect("a fin set");
                let copies = f64::from(u32::try_from(placed.copies.len()).expect("a few pods"));
                let rule = super::openrocket_fin_set_roll_kg_m2(set, radius, one.mass_kg)
                    .expect("two or more fins");
                roll += copies * (rule - one.inertia_kg_m2.z_axis.z);
            }
            let apart = relative(roll, theirs[2]);
            assert!(apart.abs() < 2e-9, "{question}: roll {apart:e}");

            let i = layout.structure.inertia_kg_m2;
            let pitch = probe["structure"]["iyy"].as_f64().expect("a number");
            assert_eq!(
                probe["structure"]["izz"].as_f64(),
                Some(pitch),
                "{question}"
            );
            let about_x = relative(i.x_axis.x, pitch);
            if let Some((_, want)) = FIN_PITCH.iter().find(|(q, _)| q == question) {
                assert!(pinned(about_x, *want), "{question}: {about_x:e}");
                continue;
            }
            assert!(
                !layout
                    .components
                    .iter()
                    .any(|c| matches!(c.part, Part::FinSet(_))),
                "{question}: fins not pinned"
            );
            assert!(about_x.abs() < 6.1e-7, "{question}: {about_x:e}");
            let about_y = relative(i.y_axis.y, pitch);
            if question.starts_with("three") {
                assert!(about_y.abs() < 6.1e-7, "{question}: {about_y:e}");
            } else if of_length {
                assert!((0.002..0.011).contains(&about_y), "{question}: {about_y:e}");
            }
        }
    }

    /// The probes' fixed ids, as `conventions.py` writes them.
    fn conventions_id(n: u32) -> &'static str {
        match n {
            1 => "00000000-0000-4000-8000-000000000001",
            2 => "00000000-0000-4000-8000-000000000002",
            _ => unreachable!("only the probes' nose and tube are asked for"),
        }
    }

    fn clusters() -> Value {
        let text = include_str!("../../../validation/fixtures/ork/openrocket-clusters.json");
        serde_json::from_str(text).expect("the committed record is JSON")
    }

    /// Every tube of every cluster OpenRocket 24.12 was asked about sits where OpenRocket puts it,
    /// to 1e-15 m: each of its fourteen patterns at scale 1, a scale, a rotation, a radial offset,
    /// and all three at once, with OpenRocket's `(y, z)` read as hpr's `(x, y)` (ADR-075). A
    /// pattern OpenRocket has no name for is one tube, as OpenRocket reads it, with a warning.
    #[test]
    fn every_tube_of_a_cluster_is_where_openrocket_puts_it() {
        let record = clusters();
        let patterns = record["patterns"].as_object().expect("patterns");
        assert_eq!(patterns.len(), 14);
        for name in patterns.keys() {
            assert!(
                record["probes"][format!("{name} at scale 1")].is_object(),
                "no probe of {name}"
            );
        }
        for (question, probe) in record["probes"].as_object().expect("probes") {
            let (layout, warnings) = hpr(probe);
            let unnamed = question == "a pattern OpenRocket has no name for";
            assert_eq!(
                warnings.len(),
                usize::from(unnamed),
                "{question}: {warnings:?}"
            );
            if unnamed {
                assert!(
                    warnings[0].contains("is not one of OpenRocket's cluster patterns"),
                    "{warnings:?}"
                );
            }
            for (id, tube) in probe["tubes"].as_object().expect("tubes") {
                let (_, placed) = layout.find(id).expect("the tube");
                let Part::InnerTube(inner) = &placed.part else {
                    panic!("{question}: {id} is not an inner tube");
                };
                let [x, y] = placed.part.axis_offset_m();
                let ours: Vec<[f64; 2]> = inner
                    .tubes_m()
                    .expect("finite")
                    .iter()
                    .map(|[u, v]| [x + u, y + v])
                    .collect();
                let theirs: Vec<[f64; 2]> = tube["instance_offsets_m"]
                    .as_array()
                    .expect("offsets")
                    .iter()
                    .map(|c| [c[1].as_f64().expect("y"), c[2].as_f64().expect("z")])
                    .collect();
                assert_eq!(ours.len(), theirs.len(), "{question}");
                assert_eq!(
                    tube["count"].as_u64(),
                    Some(ours.len() as u64),
                    "{question}"
                );
                for (ours, theirs) in ours.iter().zip(&theirs) {
                    let apart = (ours[0] - theirs[0]).hypot(ours[1] - theirs[1]);
                    assert!(apart <= 1e-15, "{question}: {ours:?} vs {theirs:?}");
                }
            }
        }
    }

    /// A cluster weighs what OpenRocket 24.12 says, but for the spread of its own tubes about the
    /// cluster's axis. The mass and centre agree to 1e-12 on every probe: every tube counted, an
    /// engine block inside counted once in each tube, a mass override on the cluster its whole
    /// mass, and an automatic ring's bore the tube's own radius, as if the cluster were one tube on
    /// the axis. OpenRocket weighs a cluster's tubes stacked on its axis (a 3-ring at scale 1 and at
    /// 1.5 have the same inertias), while hpr places each where it is: so on every cluster on the
    /// body's axis, hpr's roll inertia is OpenRocket's plus `Σ m |c|²` over the tubes, `c` each
    /// tube's offset and `m` its mass, and the pitch inertia (the mean across the axis) plus half
    /// of it, to 1e-12, worked here by hand (ADR-075). Off the axis the roll rule still holds for a
    /// cluster, but three departures are left, pinned as measured, less the spread: a lone tube
    /// 10 mm off the axis, whose offset OpenRocket leaves out of both inertias, and the pitch of
    /// two clusters off the axis, 0.015% and 0.021%, which hpr has not traced.
    #[test]
    fn a_cluster_weighs_as_openrocket_s_but_for_its_tubes_spread() {
        const OFF_AXIS: [(&str, [f64; 2]); 3] = [
            (
                "an unclustered tube 10 mm off the axis at 30",
                [0.00303, 0.000164],
            ),
            ("a 2-row 10 mm off the axis at 30", [0.0, 0.00015]),
            (
                "a 3-ring 10 mm off the axis at 30, turned 20",
                [0.0, 0.000206],
            ),
        ];
        let record = clusters();
        for (question, probe) in record["probes"].as_object().expect("probes") {
            let (ours, theirs, _) = both(probe);
            assert!(
                relative(ours[0], theirs[0]).abs() <= 1e-12,
                "{question}: mass"
            );
            assert!((ours[1] - theirs[1]).abs() <= 1e-12, "{question}: centre");
            let (layout, _) = hpr(probe);
            let mut spread = 0.0;
            for id in probe["tubes"].as_object().expect("tubes").keys() {
                let (_, placed) = layout.find(id).expect("the tube");
                let Part::InnerTube(inner) = &placed.part else {
                    panic!("{question}: {id} is not an inner tube");
                };
                let tubes = inner.tubes_m().expect("finite");
                let each_kg = placed.own.mass_kg / tubes.len() as f64;
                spread += tubes
                    .iter()
                    .map(|[u, v]| each_kg * (u * u + v * v))
                    .sum::<f64>();
            }
            // Roll and pitch less the spread, against OpenRocket's.
            let found = [
                relative(ours[2] - spread, theirs[2]),
                relative(ours[3] - spread / 2.0, theirs[3]),
            ];
            let pinned = OFF_AXIS
                .iter()
                .find(|(q, _)| q == question)
                .map_or([0.0, 0.0], |(_, pinned)| *pinned);
            for (k, (found, pinned)) in found.iter().zip(pinned).enumerate() {
                let bound = if pinned == 0.0 {
                    1e-12
                } else {
                    5e-3 * pinned.abs()
                };
                assert!(
                    (found - pinned).abs() <= bound,
                    "{question}: inertia {k} less the spread is {found:e}, not {pinned:e}"
                );
            }
        }
    }

    /// The cluster record was written by the script it names, from OpenRocket 24.12, and every
    /// probe in it is one the tests above read, which read them all.
    #[test]
    fn the_cluster_record_is_openrocket_s() {
        let record = clusters();
        assert_eq!(record["openrocket"], "24.12");
        assert_eq!(
            record["source"],
            "validation/oracles/openrocket/clusters.py"
        );
        assert!(record["probes"].as_object().expect("probes").len() >= 22);
    }

    /// The probes of an automatic outer radius inside a nose cone or transition, and of an inner
    /// tube written `auto` (M2.2e7).
    const BORES: [&str; 13] = [
        "a nose holding a coupler of automatic radius at its bottom",
        "an ogive nose holding a coupler of automatic radius at its bottom",
        "a nose holding a coupler of automatic radius past its base",
        "a nose with a shoulder, holding a coupler of automatic radius past its base",
        "a nose holding a long coupler of automatic radius from its middle",
        "a nose holding an engine block of automatic radius",
        "a nose holding a centering ring and a bulkhead of automatic radius",
        "a nose holding a coupler of automatic radius and a wall thicker than its bore",
        "a nose holding a coupler of automatic radius with a mass inside",
        "a transition holding a coupler of automatic radius",
        "a tube holding a coupler of automatic radius and a wall thicker than its bore",
        "a tube holding an inner tube of automatic radius",
        "a nose holding an inner tube of automatic radius",
    ];

    /// The one bore probe hpr refuses: a coupler at the tip, where the cone's wall meets the axis.
    const BORE_REFUSED: &str = "a nose holding a coupler of automatic radius at its tip";

    /// M2.2e7 ([ADR-096][adr-096]): inside a hollow nose cone or transition, an automatic outer
    /// radius is the parent's outer radius at the part's narrower end less its wall, the shoulder
    /// left out; a wall thicker than that is the tube solid, in a body tube too; and an
    /// `innertube` written `auto` keeps 9.5 mm, with no warning, as OpenRocket reads it. So every
    /// coupler, engine block, inner tube, ring and bulkhead weighs OpenRocket's mass to 1e-14 at
    /// OpenRocket's station to 1e-15.
    ///
    /// The nose cones and the transition carry the gaps their walls already had: 5.1e-7 of the
    /// cone's mass, 3.9e-5 of the ogive's and 2.8e-6 of the transition's, and centres within
    /// 2.5e-6 m (hpr measures a wall normal to the surface). One mass component is pinned apart: packed with an
    /// automatic radius inside the coupler, OpenRocket shortens it to 8.49 mm, keeping the volume
    /// of its 12.5 mm by 50 mm default, and hpr keeps its written 50 mm, 20.75 mm further aft of
    /// its centre ([#186][i186]). A coupler at the tip, where the wall meets the axis, has no
    /// radius: OpenRocket weighs it as nothing, and hpr refuses the design.
    ///
    /// [adr-096]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-096-fin-fillets-and-an-automatic-radius-inside-a-nose-cone-read-as-openrocket-reads-them-2026-09-28
    /// [i186]: https://github.com/nrdptel/hpr-sim/issues/186
    #[test]
    fn an_automatic_radius_inside_a_nose_reads_as_openrocket_does() {
        let record = record();
        for question in BORES {
            let probe = probe(&record, question);
            let (layout, warnings) = hpr(probe);
            assert!(warnings.is_empty(), "{question}: {warnings:?}");
            for part in probe["parts"].as_array().expect("parts") {
                let class = part["class"].as_str().expect("a class");
                if matches!(class, "Rocket" | "AxialStage") {
                    continue;
                }
                let id = part["id"].as_str().expect("an id");
                let (_, placed) = layout
                    .find(id)
                    .unwrap_or_else(|| panic!("{question}: no part {id} in hpr"));
                let mass = part["mass_kg"].as_f64().expect("a mass");
                let station = part["cm_x_m"].as_f64().expect("a station");
                let (mass_bound, station_gap, station_bound) = match class {
                    "NoseCone" | "Transition" => (5e-5, 0.0, 5e-6),
                    "MassComponent" => (1e-15, 0.020755, 1e-6),
                    _ => (1e-14, 0.0, 1e-15),
                };
                let found = relative(placed.own.mass_kg, mass);
                assert!(
                    found.abs() <= mass_bound,
                    "{question}: {class} {id} is {found:e} from OpenRocket's mass"
                );
                let apart_m = -placed.own.cg_m.z - station;
                assert!(
                    (apart_m - station_gap).abs() <= station_bound,
                    "{question}: {class} {id} is {apart_m:e} m from OpenRocket's station"
                );
            }
        }

        let probe = probe(&record, BORE_REFUSED);
        let coupler = probe["parts"]
            .as_array()
            .expect("parts")
            .iter()
            .find(|part| part["class"] == "TubeCoupler")
            .expect("the coupler");
        assert_eq!(coupler["mass_kg"].as_f64(), Some(0.0));
        let document = probe["document"].as_str().expect("the probe's document");
        let read = ork::read(document.as_bytes()).expect("a probe reads");
        let error = ork::rocket(&read.value.document)
            .value
            .layout()
            .expect_err("a tube of no radius");
        assert!(
            matches!(
                &error,
                hpr_design::DesignError::InComponent { source, .. }
                    if matches!(
                        **source,
                        hpr_design::DesignError::Domain { what: "outer radius", value }
                            if value == 0.0
                    )
            ),
            "{error:?}"
        );
    }

    /// M2.2e8's probes: tube fin sets on a 50 mm or a 20 mm tube, most written `auto` (ADR-098).
    const TUBE_FINS: [&str; 19] = [
        "a tube and 1 tube fins of automatic radius",
        "a tube and 2 tube fins of automatic radius",
        "a tube and 3 tube fins of automatic radius",
        "a tube and 4 tube fins of automatic radius",
        "a tube and 5 tube fins of automatic radius",
        "a tube and 6 tube fins of automatic radius",
        "a tube and 8 tube fins of automatic radius",
        "a tube and 9 tube fins of automatic radius",
        "a tube and 12 tube fins of automatic radius",
        "a tube and 20 tube fins of automatic radius",
        "a tube and 6 tube fins of a stated radius",
        "a tube and 6 tube fins of automatic radius and a wall thicker than it",
        "a tube and 6 tube fins of a stated radius, offset from the body",
        "a tube of automatic radius and 4 tube fins of automatic radius",
        "a tube and 12 tube fins of a stated radius",
        "a tube and 100 tube fins of automatic radius",
        "a 20 mm tube and 1 tube fins of automatic radius",
        "a 20 mm tube and 2 tube fins of automatic radius",
        "a 20 mm tube and 5 tube fins of automatic radius",
    ];

    /// A tube fin set written `auto` has the radius OpenRocket 24.12 works out, the ring closed
    /// around the body for three tubes or more and the body's radius for one or two; a wall thicker
    /// than it is cut to it; more than 8 tubes are 8; and the tubes weigh what OpenRocket's do,
    /// centred where its are (ADR-098). A written radial offset moves nothing in OpenRocket, and
    /// hpr does not read one. Two inertias depart, and hpr keeps its hollow tubes at `R_b + r`:
    /// OpenRocket's roll exceeds what any mass inside the ring can have, `(R_b + 2r)²` a unit of
    /// mass, and its pitch is `N` times one tube's own, with no term for the ring's spread.
    #[test]
    fn a_tube_fin_sets_automatic_radius_reads_as_openrocket_does() {
        let record = record();
        for question in TUBE_FINS {
            let probe = probe(&record, question);
            let (layout, warnings) = hpr(probe);
            // More than 8 tubes are read as 8 out loud; an offset from the body is read as none,
            // out loud, which is also what OpenRocket's numbers show.
            let capped = ["9", "12", "20", "100"]
                .iter()
                .any(|n| question.contains(&format!(" and {n} tube fins")));
            let offset = question.contains("offset");
            assert_eq!(
                warnings.len(),
                usize::from(capped || offset),
                "{question}: {warnings:?}"
            );
            let said = if offset {
                "read sitting on it"
            } else {
                "reads at most 8"
            };
            assert!(
                warnings.iter().all(|w| w.contains(said)),
                "{question}: {warnings:?}"
            );
            for part in probe["parts"].as_array().expect("parts") {
                let class = part["class"].as_str().expect("a class");
                if matches!(class, "Rocket" | "AxialStage") {
                    continue;
                }
                let id = part["id"].as_str().expect("an id");
                let (_, placed) = layout
                    .find(id)
                    .unwrap_or_else(|| panic!("{question}: no part {id} in hpr"));
                // A nose cone's shell is the bore probes' bound (the nose is only on one probe).
                let (mass_bound, station_bound) = match class {
                    "NoseCone" => (5e-5, 5e-6),
                    _ => (1e-14, 1e-15),
                };
                let mass = part["mass_kg"].as_f64().expect("a mass");
                let found = relative(placed.own.mass_kg, mass);
                assert!(
                    found.abs() <= mass_bound,
                    "{question}: {class} is {found:e} from OpenRocket's mass"
                );
                let apart_m = -placed.own.cg_m.z - part["cm_x_m"].as_f64().expect("a station");
                assert!(
                    apart_m.abs() <= station_bound,
                    "{question}: {class} is {apart_m:e} m from OpenRocket's"
                );
            }
            let (id, theirs) = probe["tube_fins"]
                .as_object()
                .and_then(|sets| sets.iter().next())
                .expect("one tube fin set");
            let (_, placed) = layout.find(id).expect("the tube fin set");
            let Part::TubeFinSet(ours) = &placed.part else {
                panic!("{question}: {:?}", placed.part);
            };
            let number = |key: &str| theirs[key].as_f64().expect(key);
            assert_eq!(
                theirs["automatic"].as_bool(),
                Some(!question.contains("stated"))
            );
            assert_eq!(
                Some(u64::from(ours.count)),
                theirs["count"].as_u64(),
                "{question}"
            );
            let r = number("outer_radius_m");
            assert!(
                relative(ours.outer_radius_m, r).abs() <= 1e-15,
                "{question}: radius {} against OpenRocket's {r}",
                ours.outer_radius_m
            );
            assert!(
                relative(ours.thickness_m, number("thickness_m")).abs() <= 1e-15,
                "{question}: wall {} against {}",
                ours.thickness_m,
                number("thickness_m")
            );
            let body_radius_m = number("body_radius_m");
            assert_eq!(placed.body_radius_m, Some(body_radius_m), "{question}");
            // Per unit mass, about the set's centre: one tube's own inertias, across and about its
            // axis, and how far its axis is from the body's.
            let (r_i, length_m) = (number("inner_radius_m"), 0.1);
            let across = (r * r + r_i * r_i) / 4.0 + length_m * length_m / 12.0;
            let about = (r * r + r_i * r_i) / 2.0;
            let d2 = (body_radius_m + r).powi(2);
            let unit = |i: f64| i / placed.own.mass_kg;
            let inertia = &placed.own.inertia_kg_m2;
            // OpenRocket's pitch is each tube's own inertia across it, times the count, with no term
            // for the tubes' distance from the axis: a second departure (ADR-098).
            let their_pitch = number("longitudinal_unit_inertia_m2");
            let n = f64::from(ours.count);
            assert!(
                relative(their_pitch, n * across).abs() <= 1e-14,
                "{question}: {their_pitch}"
            );
            if ours.count == 1 {
                // One tube: its centre `R_b + r` off the axis, and its own roll, in both codes.
                let [_, y, z] =
                    [0, 1, 2].map(|k| theirs["component_cg_xyz_m"][k].as_f64().expect("a centre"));
                let off = y.hypot(z);
                assert!(
                    (off - (body_radius_m + r)).abs() <= 1e-15,
                    "{question}: {off}"
                );
                let ours_off = placed.own.cg_m.x.hypot(placed.own.cg_m.y);
                assert!(
                    (ours_off - off).abs() <= 1e-15,
                    "{question}: hpr's centre {ours_off}"
                );
                let roll = number("rotational_unit_inertia_m2");
                assert!(relative(roll, about).abs() <= 1e-15, "{question}: {roll}");
                assert!(
                    relative(unit(inertia.z_axis.z), roll).abs() <= 1e-14,
                    "{question}"
                );
                // And its own pitch, about its own centre: the one count where the two agree.
                assert!(
                    relative(unit(inertia.x_axis.x), across).abs() <= 1e-13,
                    "{question}"
                );
                assert!(relative(their_pitch, across).abs() <= 1e-14, "{question}");
            } else {
                // A ring: hpr's tubes at `R_b + r`, which OpenRocket's roll exceeds past any mass
                // inside `R_b + 2r`.
                assert!(
                    relative(unit(inertia.z_axis.z), about + d2).abs() <= 1e-13,
                    "{question}"
                );
                let bound = (body_radius_m + 2.0 * r).powi(2);
                let roll = number("rotational_unit_inertia_m2");
                assert!(
                    roll > bound,
                    "{question}: OpenRocket's {roll} within {bound}"
                );
                if ours.count >= 3 {
                    let pitch = unit(inertia.x_axis.x);
                    assert!(
                        relative(pitch, across + d2 / 2.0).abs() <= 1e-13,
                        "{question}: {pitch}"
                    );
                    assert!(
                        relative(unit(inertia.y_axis.y), pitch).abs() <= 1e-13,
                        "{question}"
                    );
                    assert!(
                        their_pitch > pitch * 1.2,
                        "{question}: {their_pitch} against hpr's {pitch}"
                    );
                }
            }
        }
    }

    /// The record was written by the script it names, from OpenRocket 24.12 with no default
    /// materials saved in its preferences, and every probe it holds is one a test here reads: a
    /// probe added to the script and not to a test would be a question nobody checks the answer
    /// to.
    #[test]
    fn every_probe_is_checked() {
        let record = record();
        assert_eq!(record["openrocket"], "24.12");
        assert_eq!(
            record["source"],
            "validation/oracles/openrocket/conventions.py"
        );
        assert_eq!(record["saved_default_materials"], serde_json::json!({}));
        let mut read: Vec<&str> = READINGS.to_vec();
        read.extend(PRECEDENCE.iter().map(|(question, _)| *question));
        read.extend(DISAGREEING.iter().map(|(question, _)| *question));
        read.extend(INERTIA.iter().map(|(question, _)| *question));
        read.extend(ALONE.iter().map(|(question, _)| *question));
        read.extend(OLD_FLAG.iter().map(|(question, _)| *question));
        read.extend(BORES);
        read.push(BORE_REFUSED);
        read.extend(TUBE_FINS);
        let probes: Vec<&str> = record["probes"]
            .as_object()
            .expect("probes")
            .keys()
            .map(String::as_str)
            .collect();
        for question in &probes {
            assert!(
                read.contains(question),
                "no test reads the probe {question:?}"
            );
        }
        for question in &read {
            assert!(probes.contains(question), "no probe asks {question:?}");
        }
    }

    /// Where hpr puts a tube fin set, against OpenRocket 24.12 (Loft lesson L19, ADR-102).
    struct TubeFinsSeen {
        probe: String,
        mach: f64,
        /// OpenRocket's set slope over hpr's.
        slope_ratio: f64,
        /// OpenRocket's set slope over `N π d²/A_ref`, the long-ring limit of `N` isolated rings.
        over_long_ring_limit: f64,
        /// hpr's rocket centre of pressure less OpenRocket's, calibres.
        gap_cal: f64,
        /// How far hpr's centre moves, calibres, with the tubes at OpenRocket's centre alone, and
        /// with OpenRocket's slope alone.
        centre_only_cal: f64,
        slope_only_cal: f64,
    }

    /// Every probe of `openrocket-tube-fin-aero.json` at each Mach number below the tube-fin
    /// model's limit, with the checks that hold whatever the two codes' models: the same geometry,
    /// the same nose and body, and OpenRocket's rule as its record shows it.
    fn tube_fins_seen() -> Vec<TubeFinsSeen> {
        let text = include_str!("../../../validation/fixtures/ork/openrocket-tube-fin-aero.json");
        let record: Value = serde_json::from_str(text).expect("the committed record is JSON");
        let number = |value: &Value| value.as_f64().expect("a number");
        let mut seen = Vec::new();
        let probes = record["probes"].as_object().expect("probes");
        assert_eq!(probes.len(), 14);
        for (name, probe) in probes {
            let document = probe["document"].as_str().expect("the probe's document");
            let read = ork::read(document.as_bytes()).expect("a probe reads");
            let layout = ork::rocket(&read.value.document)
                .value
                .layout()
                .expect("a probe lays out");
            let model = hpr_aero::AeroModel::new(&layout).expect("hpr models the probe");
            let set = &model.tube_fin_sets()[0];
            let mut slopes = Vec::new();
            for asked in probe["machs"].as_array().expect("machs") {
                let mach = number(&asked["mach"]);
                let cal = number(&asked["reference_length_m"]);
                let components = asked["components"].as_object().expect("components");
                let of = |class: &str| {
                    let mut found = components.values().filter(|c| c["class"] == class);
                    let one = found.next().expect("the component");
                    assert!(found.next().is_none(), "{name}: two of {class}");
                    one
                };
                let (tubes, rocket) = (of("TubeFinSet"), of("Rocket"));
                // The same rocket: reference area, the tubes' count, leading edge and length.
                assert!(
                    (model.reference_area_m2() / number(&asked["reference_area_m2"]) - 1.0).abs()
                        <= 1e-12,
                    "{name}"
                );
                assert_eq!(
                    tubes["count"].as_u64(),
                    Some(u64::from(set.count)),
                    "{name}"
                );
                let fore_m = number(&tubes["leading_edge_x_m"]);
                let length_m = number(&tubes["length_m"]);
                assert!((set.fore_station_m - fore_m).abs() <= 1e-12, "{name}");
                assert!((set.length_m - length_m).abs() <= 1e-12, "{name}");
                // OpenRocket's rule, as its record shows it: one slope at every Mach number, and
                // the centre a quarter of the length aft of the leading edge up to Mach 0.5, at
                // the leading edge from Mach 0.6.
                let (theirs, their_x) = (number(&tubes["cna_per_rad"]), number(&tubes["cp_x_m"]));
                slopes.push(theirs);
                let quarter = if mach <= 0.5 { 0.25 } else { 0.0 };
                assert!(
                    ((their_x - fore_m) / length_m - quarter).abs() <= 1e-9,
                    "{name} at Mach {mach}"
                );
                if mach >= hpr_aero::tube_fins::TUBE_FIN_MACH_LIMIT {
                    continue;
                }
                let (ours, our_x) = set.loading(mach).expect("below the limit");
                let whole = model
                    .normal_force(&hpr_aero::Flow::new(mach, 0.0, 0.0))
                    .expect("hpr's normal force");
                let our_cp = whole.cp_station_m.expect("a centre of pressure");
                let (their_whole, their_cp) =
                    (number(&rocket["cna_per_rad"]), number(&rocket["cp_x_m"]));
                // The rest of the rocket, the nose and the body, is nearly the same in both
                // codes: OpenRocket gives the body tube a slope of 0.003 at its middle, hpr none,
                // which puts their centre 0.015 calibres aft of hpr's nose's.
                let rest = whole.slope_per_rad - ours;
                let rest_moment = whole.moment_slope_m - ours * our_x;
                let (their_rest, their_rest_moment) = (
                    their_whole - theirs,
                    their_whole * their_cp - theirs * their_x,
                );
                assert!(
                    (rest - their_rest).abs() <= 0.005,
                    "{name}: {rest} {their_rest}"
                );
                assert!(
                    (rest_moment / rest - their_rest_moment / their_rest).abs() <= 0.02 * cal,
                    "{name}"
                );
                let centre = |slope: f64, x: f64| (rest_moment + slope * x) / (rest + slope);
                // So the gap is the tubes': hpr with OpenRocket's slope and centre for them is
                // OpenRocket's rocket.
                assert!(
                    (centre(theirs, their_x) - their_cp).abs() <= 0.01 * cal,
                    "{name} at Mach {mach}: {}",
                    (centre(theirs, their_x) - their_cp) / cal
                );
                let pi_d2 = std::f64::consts::PI * set.mean_diameter_m * set.mean_diameter_m;
                seen.push(TubeFinsSeen {
                    probe: name.clone(),
                    mach,
                    slope_ratio: theirs / ours,
                    over_long_ring_limit: theirs
                        / (f64::from(set.count) * pi_d2 / model.reference_area_m2()),
                    gap_cal: (our_cp - their_cp) / cal,
                    centre_only_cal: (centre(ours, their_x) - our_cp) / cal,
                    slope_only_cal: (centre(theirs, our_x) - our_cp) / cal,
                });
            }
            assert!(
                slopes.iter().all(|s| (s / slopes[0] - 1.0).abs() <= 1e-12),
                "{name}: {slopes:?}"
            );
        }
        seen
    }

    /// Loft lesson L19: Loft put a tube fin set's centre of pressure about 0.9 calibres forward of
    /// OpenRocket's. hpr's is further forward still, and the lesson's bar, a quarter calibre, is
    /// not met: this test measures the gap on OpenRocket 24.12's probes and on its *Tube fin
    /// rocket*, and pins it, so a change to either code shows (ADR-102).
    ///
    /// The whole gap is the tubes': the probes' geometry, nose and body agree, and with
    /// OpenRocket's slope and centre for the tubes hpr's rocket is OpenRocket's. Both of
    /// OpenRocket's differ from hpr's cited ring wing (ADR-099). Its slope is 1.26 to 1.86 times
    /// hpr's, and more than the long-ring limit of isolated rings, `N π d²/A_ref`, on every probe,
    /// tubes that touch nothing but the body included; its centre is a quarter of the tube's
    /// length aft of the leading edge up to Mach 0.5, a flat fin's rule, and the leading edge from
    /// Mach 0.6, a jump its maintainers call a bug and fixed after 24.12 (openrocket#3235).
    #[test]
    fn tube_fin_cp_against_the_oracle_measured_and_pinned() {
        let seen = tube_fins_seen();
        assert_eq!(seen.len(), 14 * 5);
        let close = |found: f64, pinned: f64, what: &str| {
            assert!(
                (found - pinned).abs() <= 5e-4,
                "{what}: {found} against {pinned}"
            );
        };
        let range = |values: Vec<f64>| {
            let least = values.iter().copied().fold(f64::INFINITY, f64::min);
            (
                least,
                values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            )
        };
        // OpenRocket's slope against hpr's, and against the most isolated rings can lift.
        let (least, most) = range(seen.iter().map(|s| s.slope_ratio).collect());
        close(least, 1.2631, "the least slope ratio");
        close(most, 1.8638, "the largest slope ratio");
        let (least, most) = range(seen.iter().map(|s| s.over_long_ring_limit).collect());
        close(least, 1.2038, "the least share of the long-ring limit");
        close(most, 1.8556, "the largest share of the long-ring limit");
        // The probe built like the *Tube fin rocket*: the gap at each Mach number, and at Mach
        // 0.05 what OpenRocket's centre alone and its slope alone would move hpr's by.
        let touching: Vec<&TubeFinsSeen> = seen
            .iter()
            .filter(|s| s.probe == "six touching tubes")
            .collect();
        let pinned = [-1.0427, -1.0501, -1.0647, -0.3634, -0.3867];
        assert_eq!(touching.len(), pinned.len());
        for (s, gap) in touching.iter().zip(pinned) {
            close(
                s.gap_cal,
                gap,
                &format!("six touching tubes at Mach {}", s.mach),
            );
            assert!(
                s.gap_cal.abs() > 0.25,
                "L19's bar is met at Mach {}",
                s.mach
            );
        }
        close(
            touching[0].centre_only_cal,
            0.4952,
            "OpenRocket's centre alone",
        );
        close(
            touching[0].slope_only_cal,
            0.5319,
            "OpenRocket's slope alone",
        );
        // Up to Mach 0.5, where OpenRocket takes the quarter length, no probe is within L19's
        // bar; from Mach 0.6, where it takes the leading edge, five of the 28 are. That jump is a
        // bug in 24.12, fixed after it (openrocket#3235).
        let (least, most) = range(
            seen.iter()
                .filter(|s| s.mach <= 0.5)
                .map(|s| s.gap_cal.abs())
                .collect(),
        );
        close(least, 0.4197, "the least gap to Mach 0.5");
        close(most, 2.9984, "the largest gap to Mach 0.5");
        let within: Vec<(&str, f64)> = seen
            .iter()
            .filter(|s| s.gap_cal.abs() <= 0.25)
            .map(|s| (s.probe.as_str(), s.mach))
            .collect();
        assert_eq!(
            within,
            [
                ("six touching tubes 0.025 m long", 0.6),
                ("six touching tubes 0.025 m long", 0.75),
                ("six touching tubes 0.3 m long", 0.6),
                ("six touching tubes 0.3 m long", 0.75),
                ("six touching tubes with a 0.003 m wall", 0.6),
            ]
        );
        // OpenRocket's own *Tube fin rocket*: the probe record's example and the flight record's
        // are the same OpenRocket answer, and hpr's centre, in the flight record, is 1.07
        // calibres forward of it at rod clearance.
        let probes: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/ork/openrocket-tube-fin-aero.json"
        ))
        .expect("the probe record");
        let flights: Value = serde_json::from_str(include_str!(
            "../../../validation/reports/openrocket-flights.json"
        ))
        .expect("the flight record");
        let flight = flights["flights"]
            .as_array()
            .and_then(|all| all.iter().find(|f| f["design"] == "Tube fin rocket"))
            .expect("the Tube fin rocket's flight");
        let at = &flight["at_rod_clearance"];
        let number = |value: &Value| value.as_f64().expect("a number");
        let example = &probes["example"]["machs"][0];
        assert_eq!(number(&example["mach"]), 0.05);
        let rocket = example["components"]
            .as_object()
            .and_then(|all| all.values().find(|c| c["class"] == "Rocket"))
            .expect("the example's rocket");
        let theirs = number(&at["openrocket"]["cp_from_nose_m"]);
        // The flight record's is OpenRocket's flight data at rod clearance; it is 1.4e-5 m from
        // its calculator's answer asked directly.
        assert!((number(&rocket["cp_x_m"]) - theirs).abs() <= 1e-4);
        assert!(
            number(&at["mach"]) < 0.5,
            "OpenRocket's quarter-length centre"
        );
        let gap = (number(&at["hpr"]["cp_from_nose_m"]) - theirs)
            / number(&at["openrocket"]["reference_length_m"]);
        close(gap, -1.0747, "the Tube fin rocket at rod clearance");
    }
}
