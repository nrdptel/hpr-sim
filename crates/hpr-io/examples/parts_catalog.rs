//! Parts from the catalogue OpenRocket ships: a nose cone and a parachute found by maker and part
//! number, a search by a piece of a part number, and what the catalogue holds.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example parts_catalog -p hpr-io
//! ```
//!
//! The guide's page *OpenRocket `.orc` parts catalogues* (`docs/format/orc.md`) quotes it and what
//! it prints, which is kept next to it in `parts_catalog.output.txt`; CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use hpr_io::orc::{PartKind, bundled};

fn main() {
    let catalog = bundled();
    println!(
        "{} parts from {} makers",
        catalog.parts.len(),
        catalog.manufacturers().len()
    );

    // One part, by maker and its whole part number. Its sizes are in metres; print millimetres.
    for part in catalog.find("LOC Precision", "PNC-3.00") {
        println!(
            "\n{} {}: {}",
            part.manufacturer, part.part_number, part.description
        );
        if let PartKind::NoseCone(nose) = &part.kind {
            println!(
                "  {:?}, {:.1} mm long, {:.1} mm across, wall {:.2} mm",
                nose.shape,
                nose.length_m * 1e3,
                nose.outer_diameter_m * 1e3,
                nose.thickness_m.unwrap_or_default() * 1e3,
            );
            println!(
                "  shoulder {:.1} mm long, {:.1} mm across",
                nose.shoulder_length_m * 1e3,
                nose.shoulder_diameter_m * 1e3,
            );
            match nose.material.density {
                Some(density) => println!("  {}, {density} kg/m³", nose.material.name),
                None => println!("  {}, density not given", nose.material.name),
            }
        }
    }

    // A parachute that states its own mass.
    for part in catalog.find("Giant Leap", "TAC-24") {
        println!(
            "\n{} {}: {}",
            part.manufacturer, part.part_number, part.description
        );
        if let PartKind::Parachute(chute) = &part.kind {
            println!(
                "  {:.0} mm across, {} sides, {} lines of {:.0} mm",
                chute.diameter_m * 1e3,
                chute.sides,
                chute.line_count,
                chute.line_length_m * 1e3,
            );
        }
        if let Some(mass_kg) = part.mass_kg {
            println!("  stated mass {:.1} g", mass_kg * 1e3);
        }
    }

    // Estes writes two numbers in one, `BT-20, 30316`: a search finds it by either.
    println!("\nEstes parts numbered with 30316:");
    for part in catalog.search(Some("Estes"), "30316") {
        println!("  {}: {}", part.part_number, part.description);
    }
}
