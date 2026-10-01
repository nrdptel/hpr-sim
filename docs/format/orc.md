# OpenRocket `.orc` parts catalogues

A `.orc` file is a parts catalogue for OpenRocket, which calls its parts *component presets*. It
lists nose cones, body tubes, couplers,
centering rings, bulkheads, transitions, launch lugs, parachutes and streamers, as their makers
sell them, each under its maker and part number, with its sizes and its material. OpenRocket 24.12
ships 16 of them, from the `openrocket-database` project: 3,449 parts from Estes, LOC Precision,
Giant Leap, Madcow, SEMROC and others.

hpr has those 16 files built in. A program can look a part up by maker and part number, or search
for one. This page is the reference for that reader: what a file holds, how each value is read,
and where hpr's reading differs from OpenRocket's.

**How far to trust it.** OpenRocket's own reader was run on the same files as an
[oracle](../glossary.md#oracle), a program whose answers hpr is checked against. hpr reads every
part OpenRocket reads, in the same order. Of the 18,306 numbers compared, 17,911 come out equal to
the last bit. The other 395 that differ are counted, and
each has a known cause ([Where hpr and OpenRocket differ](#where-hpr-and-openrocket-differ)). A
part is only as right as its file, though. The database's README warns that its data may be
wrong for your rocket and that you should weigh your real parts.

**What it doesn't do yet.** The builder can't take a catalogue part yet; that is the next step,
[M5.5b](../decisions-and-roadmap.md#m5-5b) (catalogue parts in the builder). Today a program reads
a part's sizes and material, and builds the part itself from them with
[the builder](../the-builder.md). Where a part states its mass, trust that over the weight the
builder works out from sizes and a material: it is what the maker weighed.

Code: `hpr_io::orc` ([API reference](../api/hpr_io/orc/index.html)), written for
[M5.5a](../decisions-and-roadmap.md#m5-5a), the parts reader. The decisions are in
[ADR-132: OpenRocket's parts catalogues](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-132-m55a-openrockets-orc-parts-catalogues-held-to-openrockets-reading-2026-10-01).

## From a program

`hpr_io::orc::bundled()` is the built-in catalogue. `find` takes a maker and a whole part number;
`search` finds every part whose number or description holds some text. This program looks up a
LOC Precision nose cone and a Giant Leap parachute, then searches Estes' parts:

<!-- quote: crates/hpr-io/examples/parts_catalog.rs -->
```rust
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
                "  {:?}, {:.1} mm long, {:.1} mm across",
                nose.shape,
                nose.length_m * 1e3,
                nose.outer_diameter_m * 1e3,
            );
            // A nose is hollow with a wall, or filled; the file may say either.
            match (nose.filled, nose.thickness_m) {
                (Some(true), _) => println!("  filled"),
                (_, Some(wall_m)) => println!("  wall {:.2} mm", wall_m * 1e3),
                _ => println!("  wall not given"),
            }
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
```

It prints:

<!-- quote: crates/hpr-io/examples/parts_catalog.output.txt -->
```text
3449 parts from 16 makers

LOC Precision PNC-3.00: Nose cone, polypropylene, PNC-3.00, ogive, 12.5"
  Ogive, 285.8 mm long, 78.7 mm across
  wall 1.96 mm
  shoulder 95.2 mm long, 75.9 mm across
  Polypropylene, bulk, 946 kg/m³

Giant Leap TAC-24: Parachute, TAC-1 type, 24 in., 4 shrouds
  610 mm across, 4 sides, 4 lines of 914 mm
  stated mass 153.1 g

Estes parts numbered with 30316:
  BT-20, 30316: Body tube, BT-20, 18 in.
```

Every size comes back in metres, every mass in kilograms, and every density in kg/m³ (solids),
kg/m² (fabric) or kg/m (cord), all [SI units](../glossary.md#si-units). A file of your own reads
with `hpr_io::orc::read(text, name)`: `text` is the file's contents, and `name` is the file's name,
which each part keeps so you can tell where it came from. The program does the reading from disk;
the reader itself does no I/O.

Two things to know when looking parts up:

- **A part number is matched whole.** Many makers write several numbers in one. Estes' BT-20 is
  `BT-20, 30316`, so `find("Estes", "BT-20")` finds nothing; `search` finds it by either piece.
  The maker matches in any case.
- **A few numbers name two parts.** In the built-in files, 21 part numbers name two parts of the
  same kind; 3 of those pairs are identical. `find` returns all of them, in file order.

## What a file holds

A `.orc` file is XML with three parts:

- `<Version>`: `0.1` in 15 of the built-in files, `1.0` in one.
- `<Materials>`: each material's name, its density, the density's units, and its kind: `BULK`
  (per volume, for solid parts), `SURFACE` (per area, for canopy and streamer fabric) or `LINE`
  (per length, for shroud lines).
- `<Components>`: the parts. Each names its maker, part number and description, and its material
  by name and kind. That material must be defined in the same file.

Each size carries its own `Unit` attribute, such as `<Length Unit="in">10</Length>`. There is no
published schema. The fields and units below are the ones the database project documents in its
`docs/TechnicalInfo.md`, and their meaning is what OpenRocket does with them.

| part | sizes it gives | in the built-in files |
|---|---|---|
| `BodyTube`, `TubeCoupler`, `EngineBlock`, `CenteringRing`, `LaunchLug` | inside and outside diameter, length | 1,089, 237, 38, 499 and 59 |
| `BulkHead` | outside diameter, length | 115 |
| `NoseCone` | shape, length, base diameter, shoulder diameter and length, filled or a wall thickness | 855 |
| `Transition` | shape, length, each end's diameter, shoulder diameter and shoulder length, filled or a wall thickness | 360 |
| `Parachute` | diameter, number of sides, number and length of shroud lines, the lines' material | 151 |
| `Streamer` | length, width, thickness | 46 |

Any part may also state its `Mass`; 229 of the built-in parts do: 207 solid parts, and 22
parachutes and streamers. A centering ring's length is its
thickness. A coupler with an inside diameter of zero is a solid nose block.

The units read are the ones OpenRocket 24.12 reads:

| quantity | units |
|---|---|
| length | `m`, `cm`, `mm`, `in`, `ft` |
| mass | `kg`, `g`, `oz`, `lb` |
| bulk density | `kg/m3`, `g/cm3`, `kg/dm3`, `lb/ft3` |
| surface density | `kg/m2`, `g/m2`, `g/cm2`, `oz/in2`, `oz/ft2`, `lb/ft2` |
| line density | `kg/m`, `g/m`, `g/cm`, `oz/ft` |

A value with no unit is in SI, as OpenRocket reads it. Each unit is its exact definition: the inch
is 0.0254 m, the foot 0.3048 m, the pound 0.45359237 kg and the ounce a sixteenth of that,
0.028349523125 kg (NIST Handbook 44, Appendix C).

## What a file leaves unsaid

- **A shape's parameter.** A nose cone's or transition's shape is one of `CONICAL`, `OGIVE`,
  `ELLIPSOID`, `PARABOLIC`, `HAACK` and `POWER` (`OGIVE` is a
  [tangent ogive](../glossary.md#tangent-ogive)). The last four have a parameter (an ogive's
  radius, a Haack series' C), but no `.orc` field can give it. OpenRocket uses its own default
  when the part goes into a design.
- **A shoulder's wall.** A [shoulder](../glossary.md#shoulder) has a diameter and a length but no
  wall thickness, and no end cap. For a hollow part, the database's own usage notes say OpenRocket
  weighs it as having no wall. For a filled part, OpenRocket weighs the shoulder as a solid
  cylinder: the check of stated masses below uses that volume, and it agrees.
- **A parachute's drag.** No field gives a drag coefficient.
- **A filled part's walls.** `Filled` says a nose cone or transition is solid. Where it is absent,
  a `Thickness` gives the wall instead. Eight nose cones and two transitions give both. hpr keeps
  both, as OpenRocket's reading does; which one a built part follows is for
  [M5.5b](../decisions-and-roadmap.md#m5-5b) to measure.

## Where hpr and OpenRocket differ

The test [`crates/hpr-io/tests/orc_openrocket.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-io/tests/orc_openrocket.rs) reads every built-in part and compares it with
OpenRocket 24.12's reading, value by value. The two agree everywhere except in these places. Each
is counted, and each count is checked:

| what | parts | hpr | OpenRocket |
|---|---|---|---|
| a mass stated in ounces | 185 | the exact ounce, 0.028349523125 kg | 0.0283495231 kg, so 8.8 parts in 10 billion lighter |
| two makers' names | 252 | as the file writes them | "LOC/Precision" and "Public Missiles, Ltd." for "LOC Precision" and "Public Missiles" |
| a solid part that states its mass | 207 | the material keeps the file's density; the part keeps its stated mass | replaces the density with the one that gives the part that mass |
| a material the file names but doesn't define | 3 | no density | a density of zero |

The third row is the one that matters for a rocket's weight. For example, an Estes balsa nose cone
that states its mass gets, in OpenRocket, a balsa density that makes the cone weigh exactly that.
hpr keeps both numbers.

The test shows that a stated mass is the cause on all 207. The oracle also reads each file with
every `<Mass>` taken out, and then OpenRocket's density equals hpr's on every part, these 207
included. On the 54 of them that are simple solids (tubes, rings, bulkheads, and filled conical
nose cones and transitions), the test also checks that OpenRocket's replaced density times the
part's volume gives the stated mass, to 4 parts in 10¹⁵.
When the builder takes catalogue parts ([M5.5b](../decisions-and-roadmap.md#m5-5b)), a part that
states its mass will weigh that mass.

OpenRocket also rounds a few other imperial factors: pounds per cubic foot, ounces per square inch
or foot, pounds per square foot, and ounces per foot. No built-in file uses them.

## Warnings, not failures

Only text that isn't XML, or whose top element isn't `<OpenRocketComponent>`, is refused. Anything
else that can't be read is left out with a warning, and reading goes on:

- **A part** with a missing or unreadable size, a unit or shape the format doesn't have, a
  material of the wrong kind, or an element that isn't a kind of part. OpenRocket 24.12 refuses
  the whole file for most of these.
- **A material** with an unknown unit or kind, or no density. Parts that name it read with no
  density.
- **A field** a part's kind doesn't have is ignored, with a warning. The 37 built-in nose cones
  that state an inside diameter read this way.
- **A field stated twice** keeps the last, as OpenRocket does. Three built-in parts state two
  descriptions.
- **A material defined twice** with two densities: parts take the first.
- **A second `<Materials>` or `<Components>` list**, or anything else beside them, is ignored.
- **A value that is read as written but looks wrong**: a tube whose inside diameter is not less
  than its outside diameter, or a fabric lighter than 1 g/m², which is likely a density written in g/m² that means kg/m².

A file with more than 1,000 warnings lists the first 1,000 and then says how many more there were.

`in/64` (sixty-fourths of an inch), which the project's notes list as a length unit, is refused,
because OpenRocket reads it as whole inches: `3` in `in/64` would come out as 3 inches.

On the built-in files there are 52 warnings, all of kinds listed above:

| warning | count |
|---|---|
| a nose cone's inside diameter, ignored | 37 |
| a description stated twice | 3 |
| a material not defined (`Carpet Thread` twice in `mpc.orc`, and a balsa in `semroc.orc`) | 3 |
| a tube-like part no narrower inside than out (one Quest, two SEMROC) | 3 |
| a fabric under 1 g/m² (five in `generic_materials.orc`, one in `giantleaprocketry.orc`) | 6 |

Nothing is changed on account of them: each such part reads as written.

## Checked against OpenRocket

[`validation/oracles/openrocket/orc_presets.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/openrocket/orc_presets.py) runs OpenRocket 24.12's preset loader, its public
API (run, never read: its source is GPL), on each of the 16 files. It first checks that the jar's
copy of each file is byte for byte the one built into hpr. It records every value of every part to
[`crates/hpr-io/tests/fixtures/orc/openrocket-presets.json`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-io/tests/fixtures/orc/openrocket-presets.json),
one part to a line, and each part's densities as OpenRocket reads the file with its masses taken
out.

It also has OpenRocket read 33 small [probe](../glossary.md#probe-design) files, each asking one question: every unit above,
values with no units, and a part or a file OpenRocket can't read. The same test reads each probe:

| probes | result |
|---|---|
| 20 | read as OpenRocket reads them, to the bit |
| 6 | in a unit whose factor OpenRocket rounds: within 2 parts in 10⁹, by hpr's exact factor |
| 4 | OpenRocket refuses the file; hpr leaves the part out (for `oz/in` it leaves the material out and reads the part without the cord's density) |
| 3 | OpenRocket reads the part (`in/64` as inches, a length of `ten` as zero, a material of the wrong kind with a density of zero); hpr leaves it out |

Moving hpr's inch to the next number a 64-bit float can hold above 0.0254 makes both tests fail,
so they check what they say they do.

## Sources and licence

- **The files:** `openrocket/openrocket-database`, <https://github.com/openrocket/openrocket-database>,
  at commit `1512874a` (2025-07-27), pinned as `openrocket-database` in the
  [reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml).
  Apache License 2.0. Created by Dave Cook and maintained by the OpenRocket team. They are built
  into hpr unchanged, in
  [`crates/hpr-io/data/openrocket-database/`](https://github.com/nrdptel/hpr-sim/tree/main/crates/hpr-io/data/openrocket-database)
  with the project's `LICENSE`.
- **The format:** the project's `docs/TechnicalInfo.md` and `docs/Usage.md` at that commit.
- **The units:** NIST Handbook 44 (2024), Appendix C, *General Tables of Units of Measurement*.
