//! hpr held to OpenRocket's own answers ([M2.2][m2-2]). For now this is only tests.
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

#[cfg(test)]
mod tests {
    use hpr_design::tree::Layout;
    use hpr_io::ork;
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

    /// Loft lesson L51: Loft's rule for which centre-of-gravity override wins came from
    /// OpenRocket's source and was unsettled by up to 133 mm. Here it is measured.
    ///
    /// On every override probe hpr's mass is OpenRocket's, so the winning mass override is the
    /// same one: a parent's override that covers its children wins over a child's own, and a
    /// stage's over everything in it. A centre of gravity override is measured from the part's
    /// front, not its shoulder's, and moves the shoulder with the part. A centre of gravity override
    /// alone that covers the parts inside moves the whole assembly.
    ///
    /// Two probes are hpr's own rule, written as departures in ADR-061 and pinned here in metres:
    ///
    /// - a mass override that covers the parts inside and states no centre: OpenRocket puts the
    ///   centre at the overriding part's own, leaving out the parts inside; hpr keeps the
    ///   assembly's, as its parts lay it out;
    /// - both overrides, with flags that disagree: hpr states one scope and takes the mass's, and
    ///   says so.
    #[test]
    fn override_precedence_matches_oracle() {
        let record = record();
        let cases: [(&str, f64); 12] = [
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
            // The departures, hpr's station less OpenRocket's.
            ("a mass override on a tube and the part inside", -0.003686),
            (
                "a mass override on a tube and the part inside, which has its own",
                -0.019690,
            ),
        ];
        for (question, apart_m) in cases {
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
        for (question, apart_m) in [
            (
                "both overrides on a tube, the centre covering the part inside and the mass not",
                0.004672,
            ),
            (
                "both overrides on a tube, the mass covering the part inside and the centre not",
                0.0,
            ),
        ] {
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

    /// Inertia under an override is hpr's own rule, a departure written in ADR-061. hpr scales a
    /// mass override's inertia with its mass, over everything the override covers, so the extra
    /// mass sits where the parts' does; OpenRocket scales only the overriding part's own inertia,
    /// keeps the inertias of the parts inside (a stage has none of its own, so nothing is scaled),
    /// and leaves the covered parts' masses out. For a centre of gravity override that covers the
    /// parts inside, hpr moves the assembly whole and OpenRocket moves the part alone. On a lone
    /// part both scale, and agree. The relative differences, hpr's to OpenRocket's, are pinned
    /// here as roll and pitch.
    #[test]
    fn inertia_under_an_override_departs_as_written() {
        let record = record();
        let cases: [(&str, [f64; 2]); 11] = [
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
        for (question, pinned) in cases {
            let (ours, theirs, _) = both(probe(&record, question));
            let found = [2, 3].map(|k| (relative(ours[k], theirs[k]) * 1e4).round() / 1e4);
            assert_eq!(found, pinned, "{question}");
        }
    }

    /// A wall or a shoulder of no thickness weighs nothing, capped or not, on a filled nose or a
    /// hollow one; a nose, transition or tube that writes no thickness has a 2 mm wall, and a
    /// shoulder that writes none has no wall; a part that names no material is made of
    /// OpenRocket's cardboard, ripstop nylon, elastic cord or (a rail button) Delrin. hpr reads
    /// each as OpenRocket 24.12 does (ADR-061), so the mass and the centre of mass agree, part by
    /// part as well as whole, and nothing is warned of.
    #[test]
    fn walls_shoulders_and_unnamed_materials_read_as_openrocket_does() {
        let record = record();
        for question in [
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
            "a nose, a shoulder and a tube that write no thickness at all",
            "a narrower nose and tube that write no thickness at all",
            "a nose and a tube that name no material, with one part of each kind inside",
        ] {
            let probe = probe(&record, question);
            let (ours, theirs, warnings) = both(probe);
            assert!(warnings.is_empty(), "{question}: {warnings:?}");
            if theirs[0] == 0.0 {
                // A structure that weighs nothing has no centre to compare.
                assert_eq!(ours[0], 0.0, "{question}");
                continue;
            }
            // A transition's wall is hpr's, measured normal to its surface; its mass is 2e-5 from
            // OpenRocket's.
            assert!(
                relative(ours[0], theirs[0]).abs() < 5e-5,
                "{question}: mass {ours:?} {theirs:?}"
            );
            // One gap is known: hpr places a rail button by its forward edge and gives it its
            // diameter as a length, where OpenRocket gives it none, so hpr's button sits one
            // radius, 5 mm, further aft (#151). It is the whole of the centre's difference.
            let (layout, _) = hpr(probe);
            let mut button_moment = 0.0;
            for part in probe["parts"].as_array().expect("parts") {
                let id = part["id"].as_str().expect("an id");
                let Some((_, placed)) = layout.find(id) else {
                    continue; // the rocket and the stage, which weigh nothing of their own
                };
                let mass = part["mass_kg"].as_f64().expect("a mass");
                assert!(
                    (placed.own.mass_kg - mass).abs() <= 5e-5 * mass,
                    "{question}: {id} weighs {} in hpr and {mass} in OpenRocket",
                    placed.own.mass_kg
                );
                let apart_m = -placed.own.cg_m.z - part["cm_x_m"].as_f64().expect("a station");
                let expected_m = if part["class"] == "RailButton" {
                    button_moment += mass * 0.005;
                    0.005
                } else {
                    0.0
                };
                assert!(
                    (apart_m - expected_m).abs() < 1e-6,
                    "{question}: {id} is {apart_m} m from OpenRocket's station"
                );
            }
            assert!(
                (ours[1] - theirs[1] - button_moment / theirs[0]).abs() < 1e-6,
                "{question}: centre {ours:?} {theirs:?}"
            );
        }
    }

    /// The record was written by the script it names, from OpenRocket 24.12, and every probe it
    /// holds is one a test here reads: a probe added to the script and not to a test would be a
    /// question nobody checks the answer to.
    #[test]
    fn every_probe_is_checked() {
        let record = record();
        assert_eq!(record["openrocket"], "24.12");
        assert_eq!(
            record["source"],
            "validation/oracles/openrocket/conventions.py"
        );
        let source = include_str!("openrocket.rs");
        for question in record["probes"].as_object().expect("probes").keys() {
            assert!(
                source.contains(&format!("\"{question}\"")),
                "no test reads the probe {question:?}"
            );
        }
    }
}
