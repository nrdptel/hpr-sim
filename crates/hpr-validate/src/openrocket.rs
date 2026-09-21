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
    ///   ([`a_fin_section_is_weighed_as_pinned`]), fillets, which hpr leaves out, an ellipse, and a
    ///   cant (−4.19e-5, not traced). Its pitch inertia is apart by up to 0.11% where the masses
    ///   agree (0.406% on a single fin, below): OpenRocket's pitch rule for fins is not measured
    ///   here.
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
    ///
    /// [adr-062]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-062-fins-and-rail-buttons-against-openrocket-roll-inertia-explained-2026-09-21
    const ALONE: [(&str, [f64; 4]); 42] = [
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
            [-0.00808, 0.000737, 0.0, -0.00439],
        ),
        (
            "a tube and a fin set with wider fillets",
            [-0.0279, 0.00254, 0.0, -0.0152],
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
            let (ours, theirs, _) = both(probe);
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
}
