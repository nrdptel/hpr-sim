# The hpr design format (`.hpr` and `.hprz`)

An `.hpr` file is a rocket design written as one JSON document: hpr's own format, open and
described by a published [JSON Schema](#the-schema). It holds everything hpr reads from an
OpenRocket `.ork`: the rocket's parts, every motor [configuration](../glossary.md#configuration),
when each parachute opens and each stage separates, the simulations the file stored, what the file
holds that hpr doesn't model, and the other files inside it, such as an embedded thrust curve or a
decal image. It is plain text, so a design can be kept in git and a change to it reads as a small
diff. A `.hprz` is the same document in a zip archive, with other files beside it, such as the
design's flight logs and photographs ([the container](#the-container-hprz)).

**What works today:** `hpr convert` turns a `.ork` into a `.hpr` or `.hprz` and back, and `hpr sim`
flies a `.hpr` or `.hprz` as it flies the `.ork` ([reading and writing one](#reading-and-writing-one)).
A document of the older version 0.1 is migrated when it is read. Rust programs can do all of this
through the `hpr_format` library. Generated TypeScript and Python types come with the format's next
step, [M3.3c](../decisions-and-roadmap.md#m3-3c).

**How far to trust it.** Nothing is lost on the way through. hpr's checks use 75 `.ork` files and
read 73. Each of the 73 goes `.ork` → `.hpr` → `.ork` and comes back as the same design, bit for
bit, and as the `.ork` hpr writes from the original, byte for byte. The 109 motor configurations
among them that fly, in 30 of the designs, reach the same apogee every way, bit for bit, with
OpenRocket's motor database supplying most of their curves. Each document, written as version 0.1,
migrates back to the same document, but for one fact 0.1 didn't record
([checked on real designs](#checked-on-real-designs)).

**Keep your `.ork` too.** The format is version 0.2, a draft until hpr's first release. A change
that would stop an older document from reading comes with a migration, but a migration can't always
recover what the older version didn't record ([versions](#versions)).

## What a document holds

A document is an object with nine keys, always written in this order:

| key | what it holds |
|---|---|
| `format` | always `"hpr-design"`, so a program can tell a design from any other JSON |
| `version` | the format's version, `"0.2"` |
| `provenance` | the program that wrote it and its version; the format and SHA-256 of the file the design came from; and, if the design's airframe was not read from that file exactly as written, why not |
| `rocket` | the stages and their parts, with every motor configuration that flies |
| `motors` | every motor configuration, flown or not, with why one is not |
| `recovery` | when each parachute and streamer opens, and when each stage separates |
| `simulations` | the simulations the source file stored, with their conditions and results |
| `extensions` | what the source file holds that hpr doesn't model, kept for writing it back |
| `source_files` | the source file's other files, such as a `.ork` archive's embedded thrust curves and decal images |

Every number is in SI units, and a key says its unit where it has one: `length_m` is metres,
`kg_m3` kilograms per cubic metre. A [SHA-256](https://en.wikipedia.org/wiki/SHA-2) is a
fingerprint of a file's bytes: it names the source without giving its path or name, which can
name a person. Here is the start of the
[stable trainer](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/ork/loft-demo/demo-stable.ork),
one of the public demonstration designs from [Loft](../glossary.md#loft-lesson), the project
before hpr, as a document:

```json
{
  "format": "hpr-design",
  "version": "0.2",
  "provenance": {
    "tool": "hpr-sim",
    "tool_version": "0.1.0",
    "source": {
      "format": "ork",
      "sha256": "199f71f9fc774d5f7f29df70ecac94d8ad46e682a001ef7e67b144970934a234"
    }
  },
  "rocket": {
    "name": "Loft Demo 38mm — stable trainer",
    "stages": [
      {
        "id": "10f70005-0000-4000-8000-000000000002",
        "name": "Sustainer",
        "components": [
          {
            "id": "10f70005-0000-4000-8000-000000000003",
            "name": "Nose cone",
            "part": {
              "nose_cone": {
                "shape": {
                  "kind": "ogive",
                  "radius_ratio": 1.0
                },
                "length_m": 0.25,
                "base_radius_m": 0.019,
```

A motor that flies is written out in full, its thrust curve included, so a document flies with no
motor database. That has a consequence for sharing. When a program supplies curves from a motor
database, as hpr's checks supply OpenRocket's, the document holds those curves, and OpenRocket's
database publishes no terms for reusing them
([where the curves come from](ork.md#motors-in-the-reference-library)). Check before sharing such a
document.

The source file's other files are under `source_files`, in the order the file held them. A text
file, such as an embedded thrust curve, is written as its text. Any other file, such as an image, is
written in [base64](https://www.rfc-editor.org/rfc/rfc4648#section-4), a standard way to spell
bytes with 64 printable characters. A `.ork` written from the document puts every one back.

**Why the airframe's reading is recorded.** hpr's `.ork` reader flies no configuration of a rocket
whose airframe it couldn't read exactly as written: a part it left out, a value it dropped, or a
size it had to assume. `hpr sim` flies no other motor in such a rocket either. The `.ork` hpr writes
from a document states outright what the original left to be assumed, so reading that `.ork` again
can't tell. The document keeps the reason under `provenance.source.airframe_not_as_written`, and
`hpr sim` takes it from there.

## The container (`.hprz`)

A `.hprz` file is a zip archive holding a design and the files that go with it: flight logs,
results, photographs, anything. Its first entry, `design.hpr`, is the design exactly as a `.hpr`
file, so unzipping a container gives a `.hpr` any reader of the format takes. Every other entry is
an attachment, kept byte for byte, under its name, in its order.

- **Names are relative paths**, with `/` between folders, such as `logs/flight-1.csv`. No part of a
  name may be empty, `.` or `..`, and a name holds no `\`, `:` or control character, so unpacking a
  container can't write outside the folder it is unpacked into on any system. `design.hpr` is
  taken, in any mix of capitals, and no two names may be the same ignoring case, which a file
  system that ignores case would unpack as one file.
- **The same design and files always give the same bytes.** Every entry is compressed the same way
  (deflate) and dated 1 January 1980, zip's zero date, never the clock.
- **Reading is held to the same rules**, so a container made by another program is refused with the
  reason rather than half read. The reader passes over a folder's own entry, which some zip tools
  write. It stops at 256 MiB of unpacked content, so a small, hostile archive can't fill memory. It
  refuses an archive with two entries of one name, which the zip library hpr uses would otherwise
  read as one, dropping the other without a word.

A `.hpr` has no place for attachments, and a `.ork` keeps only the source file's own. So
`hpr convert` names each attachment it leaves out, as a warning.

## Reading and writing one

`hpr convert` takes a design between `.ork`, `.hpr` and `.hprz`, choosing each format by the file's
extension ([`hpr convert`](../cli.md#converting-a-design)):

```bash
hpr convert my-rocket.ork my-rocket.hpr
hpr convert my-rocket.hpr my-rocket.hprz --attach flight-1.csv --attach pad.jpg
hpr sim my-rocket.hprz
```

In Rust, `hpr_format::DesignFile::from_ork` reads a `.ork` into a document, `hpr_format::to_json`
writes its text, `hpr_format::from_json` reads the text back, migrating an older version, and
`DesignFile::to_ork` writes a `.ork`. `hpr_format::container::write` and `container::read` do the
same for a `.hprz`. The API reference has worked examples
([`hpr_format`](https://nrdptel.github.io/hpr-sim/api/hpr_format/index.html)).

The text is canonical, meaning there is exactly one way to write a given design: two-space indents,
keys in the order above, and a final newline. So the same design always gives the same bytes.

Before it returns, the writer reads its own text back and
compares it with the design. A value JSON can't carry, such as an infinite number, is refused with
an error rather than written as something else.

## Versions

A version is two numbers, `major.minor`. The reader checks `format` and `version` before anything
else, so a file of another kind or another version is refused with that reason, not with the
first key it doesn't know.

- **A reader takes its own version and migrates older ones.** A migration rewrites an old document
  into the current version's shape, one version at a time, and then reads it as a current document,
  so it is held to every current rule. A newer version is refused as "written by a newer program".
- **While the major number is 0**, the current version may change in place only in a way that
  leaves every document already written readable, with the same meaning. Any other change takes a
  new minor version, with a migration from the one before. The old version's schema stays
  committed, with a document its program wrote, which a test migrates.
- **From 1.0**, a new minor version only adds, and a change that breaks old documents takes a new
  major version, with a migration.

| from | to | what the migration changes |
|---|---|---|
| 0.1 | 0.2 | `attachments` is renamed `source_files`, leaving "attachment" to mean a file in a `.hprz`. The airframe's reason, which 0.1 didn't record, is taken from the first configuration left out for it |

**What the 0.1 migration can't recover.** 0.1 kept the reason a `.ork`'s airframe wasn't read
exactly as written only in a configuration left out for it. The `.ork` reader asks that after it
checks a configuration's motors, so a configuration left out earlier, such as for want of a thrust
curve, doesn't carry it. Of the 8 corpus designs that have such a reason, the migration recovers it
for 5. A 0.1 document of one of the other 3 reads as though its airframe were read as written, and
`hpr sim` would fly another motor in it where its `.ork` refuses. Converting the `.ork` again gives a
0.2 document that knows.

The reasons are in [ADR-111](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29), which set the extensions and the first policy, and
[ADR-112](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-112-m33b-the-hprz-container-and-migrations-2026-09-29), which added the container and the first migration.

## Extensions and unknown keys

What a source file holds that hpr doesn't model is kept under `extensions`, in a namespace: a key
that starts with `x-` and names the program the data belongs to. Version 0.2 has one, `x-openrocket`: the parts, sections, tags and attributes of
a `.ork` that hpr doesn't read, each kept whole with where it was
([what hpr keeps](ork.md#what-hpr-keeps-for-writing-the-file-back)). That is how a `.ork` written
from a document gets them back.

**A key the version doesn't define is refused, not dropped**, anywhere in the document, an unknown
namespace included. A document either reads whole or says why it doesn't. Keeping another
program's namespace as it is waits for a later version.

## The schema

[`schema/format/hpr-design-0.2.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.2.schema.json)
is the document's JSON Schema. A JSON Schema is a machine-readable description of a JSON
document's keys and types, which many languages can check a file against; this one follows the
standard's 2020-12 edition. It is generated from the Rust types by `cargo xtask format` (one of the
repository's development commands), and a test fails if the committed file is stale. Its
descriptions are the types' documentation. It refuses every key the version doesn't define, as
the reader does. A few rules it can't express, so a document can pass the schema and still be
refused by hpr: a source file's base64 must decode, no two source files share a name, a name can't
be `rocket.ork` or a folder's, and every embedded thrust curve must be among the source files.
Version 0.1's schema stays beside it,
[`hpr-design-0.1.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.1.schema.json).

## Checked on real designs

The `.ork` reader and writer are checked on 75 `.ork` files, 73 of which hpr reads
([`.ork` design files](ork.md#checked-in-openrocket)). Some are other people's private designs, so
only counts are published. `cargo xtask ork` takes each one through the format:

| check | designs |
|---|---|
| its document follows the committed schema | 73 of 73 |
| the document reads back as the same document | 73 of 73 |
| the `.ork` written from it is the `.ork` hpr writes from the file itself, other files and all, byte for byte | 73 of 73 |
| that `.ork` reads back as the design first read, bit for bit | 73 of 73 |
| the document, written as version 0.1, migrates back to the same document, but for the airframe's reason 0.1 didn't record | 73 of 73 |
| of the 8 whose airframe was not read exactly as written, the migration recovers the reason | 5 of 8 |

Between them, the documents carry the files' 55 other files: 3 as text, the thrust curves the
designs embed, and 52 as base64, their images.

Then each motor configuration that flies is flown three ways: the design first read, the design
read back from its document, and the design read back from the `.ork` written from the document.
Each flight is in calm air of the [standard atmosphere](../glossary.md#standard-atmosphere) at sea
level, off a 1.5 m vertical rail, with its stages separating if they do. Of the designs' 170
configurations, **109 fly, and each reaches the same apogee all three ways, bit for bit**. The
format's first step, [M3.3a](../decisions-and-roadmap.md#m3-3a), asked for 1 part in 10⁹.

These 109 fly because the check supplies OpenRocket's own motor database for the curves the files
name but don't carry. With only the files' own curves and hpr's bundled motors, 4 fly
([motors in the reference library](ork.md#motors-in-the-reference-library)). The other 61 have no
flight to compare: the `.ork` reader leaves them out of the rocket all three ways, by the first reason it finds: 24 for want of a
thrust curve, 19 for stages hpr can't separate as written, and 18 for other reasons
([which configurations fly](ork.md#which-configurations-the-rocket-flies)).

**In CI**, the automatic checks run on every change, where the private designs aren't:
`hpr-format`'s tests take the 17 public `.ork` designs under `validation/fixtures/ork/` through the
same checks. None carries a curve hpr can fly, so [bundled motors](../physics/motor.md#the-bundled-motors)
stand in: each motor gets the bundled motor of its designation, or the one nearest its diameter.
18 configurations fly, one per design and two for `demo-multi-config`. From the document they reach
the same apogee bit for bit. Through the written `.ork` the test holds them to 1 part in 10⁹: the
stand-in motors are not written into a `.ork`, so they are supplied again when it is read. A test
also takes each key out of two documents in turn, and checks that the schema and the reader agree on
whether the document is still valid.

The tests also cover the rest of this page:

- **Migration**: a document version 0.1's program wrote, committed as
  [`embedded-curve-0.1.hpr`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-format/fixtures/embedded-curve-0.1.hpr).
  0.1's schema takes it and 0.2's doesn't. It reads as 0.2, key for key the old document with
  `attachments` renamed. It is the document the current reader makes from the same `.ork`, flies
  on the curve it embeds, and writes that `.ork` back.
- **The container**: a design with four attachments (a text file, an image, an empty file and a
  name in accents) reads back the same and writes the same bytes again. Names that could leave
  their folder, two entries of one name, a missing or damaged design, and an archive past its
  unpacking limit are each refused with the reason.
- **The command line**: `hpr convert` takes a public design `.ork` → `.hpr` → `.hprz` → `.ork`, each
  file the library's own, byte for byte. `hpr sim` flies a public design from its `.hpr` and its
  `.hprz` to the same output as from its `.ork`. It refuses another motor in a rocket whose airframe
  wasn't read as written, whichever of the three it reads.

## How it compares with other design formats

Four other formats hold a hobby rocket design. hpr reads `.ork` and writes it back; it doesn't read
the other three yet ([`.ork` design files](ork.md#checked-in-openrocket) lists the library's
`.rkt` and `.CDX1` files it leaves out).

| | OpenRocket `.ork` | RockSim `.rkt` | RASAero II `.CDX1` | RocketPy `.rpy` | hpr `.hpr` / `.hprz` |
|---|---|---|---|---|---|
| encoding | XML, usually in a zip archive with other files | XML | XML | JSON | JSON, canonical text; a zip archive for `.hprz` |
| published description | a prose page, no schema | in RockSim's own help, inside the program | none: "not documented", its author says | none beyond the code | a JSON Schema, generated and tested |
| version in the file | yes, `major.minor` | yes | yes | RocketPy's version | yes, `major.minor`, with migrations |
| units | mostly SI, not stated in the file | fixed default units, not stated | the program's English units (inches), not stated | SI by RocketPy's convention, not stated | SI, the unit in every key's name |
| motor configurations, stages, recovery | all three | stages, motors and parachutes | a sustainer and up to two boosters; recovery at apogee and at a set altitude | one motor per rocket, parachutes, no staging | all three, as `.ork` has them |
| thrust curves and other files | embedded curves, images | motors by name | motors by name | curves as data, no other files | embedded curves and images; any file in a `.hprz` |
| another program's data | a simulation's extension settings | none found | none found | none | `extensions`, one namespace per program |
| what it is | a design | a design | a design, as its outer shape for aerodynamics | a saved `Flight`: the rocket, its environment and the results | a design |

Sources: OpenRocket's [file format page](https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html)
and hpr's own reading of 73 files ([`.ork` design files](ork.md)); Apogee's
[SMARTSim manual](https://www.apogeerockets.com/downloads/PDFs/SMARTSim_manual.pdf), which says
RockSim's rocket files are XML with data "stored using prescribed default units"; RASAero II's
author on its [file format](https://www.rocketryforum.com/threads/openrocket-cnalpha-accuracy.171000/page-2)
and its [user's manual](https://www.rasaero.com/dl_software_ii.htm), which gives every dimension
in inches; the four `.rkt` and four `.CDX1` files in hpr's reference library, of which only these
facts are published; and RocketPy's code at the
version hpr's checks pin, where `.rpy` came with version 1.10.0
([`save_to_rpy` and `load_from_rpy`](https://github.com/RocketPy-Team/RocketPy/blob/9bd6ad3af8f97bafa3201d4e70eba2877e66e040/rocketpy/utilities.py#L716)).

**Why hpr didn't adopt one of them:**

- **`.ork`**: its page describes the structure and leaves the rest to OpenRocket's source code,
  which is GPL-licensed and which this project doesn't read. It has no schema, and the usual zip
  archive shows no useful difference in git. hpr keeps reading and writing it, as the format most
  designs are shared in.
- **`.rkt`**: RockSim is a commercial program, and the full description of its file is inside it.
  Its units are implied, not stated.
- **`.CDX1`**: undocumented, in English units, and it describes the rocket's outer shape for
  aerodynamics, not a design to build.
- **`.rpy`**: a saved state of one Python library's objects, not a design format. It has no schema,
  holds one motor and no stages, and a function in it (a parachute's trigger, say) is stored as a
  Python pickle, which runs code when the file is loaded. So a `.rpy` from someone else is only as
  safe as a program from them.

## What is not there yet

- **Generated TypeScript and Python types**: [M3.3c](../decisions-and-roadmap.md#m3-3c).
- **Reading `.rkt`, `.CDX1` or `.rpy`**: no milestone yet.
- **An embedded curve is held twice**: as the file's text under `source_files`, and as the motor
  built from it in the configuration. A flight uses the motor; a `.ork` written from the document
  uses the text. Editing one doesn't change the other, and nothing checks that they agree yet.
- **Some type names come from `.ork`**, such as `OrkMotor`, because the document holds the design
  as hpr's `.ork` reader models it. A later version can rename them, with a migration.
- **A container's attachments aren't read by anything yet.** `hpr sim` flies the design and says how
  many it leaves unread; reading a flight log from a `.hprz` waits for the commands that compare a
  flight with its simulation.
