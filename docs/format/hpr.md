# The hpr design format (`.hpr`)

An `.hpr` file is a rocket design written as one JSON document: hpr's own format, open and
described by a published [JSON Schema](#the-schema). It holds everything hpr reads from an
OpenRocket `.ork`: the rocket's parts, every motor [configuration](../glossary.md#configuration),
when each parachute opens and each stage separates, the simulations the file stored, what the file
holds that hpr doesn't model, and the other files inside it, such as an embedded thrust curve or a
decal image. It is plain text, so a design can be kept in git and a change to it reads as a small
diff.

**What works today:** only a Rust program can make one, and only from a `.ork`. The `hpr_format`
library reads a `.ork` into a document, writes it as text, reads it back, and writes the `.ork`
again ([reading and writing one](#reading-and-writing-one)). The command line doesn't read or write
`.hpr` yet. That comes with the format's next step,
[M3.3b](../decisions-and-roadmap.md#m3-3b), with a zip container for a design with its flight logs and other files,
migrations from older versions, and a comparison with the other design formats. Generated
TypeScript and Python types follow in [M3.3c](../decisions-and-roadmap.md#m3-3c), the step after.

**How far to trust it.** Nothing is lost on the way through. hpr's checks use 75 `.ork` files and
read 73. Each of the 73 goes `.ork` → `.hpr` → `.ork` and comes back as the same design, bit for
bit, and as the `.ork` hpr writes from the original, byte for byte. The 109 motor configurations
among them that fly, in 30 of the designs, reach the same apogee every way, bit for bit, with
OpenRocket's motor database supplying most of their curves
([checked on real designs](#checked-on-real-designs)).

**Keep your `.ork`.** The format is version 0.1, a draft: until hpr's first release it can change
without a new version number ([versions](#versions)). A `.hpr` saved today may be refused by a later
build of hpr, and nothing will convert it. Keep the `.ork` as the master copy until version 1.0.

## What a document holds

A document is an object with nine keys, always written in this order:

| key | what it holds |
|---|---|
| `format` | always `"hpr-design"`, so a program can tell a design from any other JSON |
| `version` | the format's version, `"0.1"` |
| `provenance` | the program that wrote it, its version, and the format and SHA-256 of the file the design came from |
| `rocket` | the stages and their parts, with every motor configuration that flies |
| `motors` | every motor configuration, flown or not, with why one is not |
| `recovery` | when each parachute and streamer opens, and when each stage separates |
| `simulations` | the simulations the source file stored, with their conditions and results |
| `extensions` | what the source file holds that hpr doesn't model, kept for writing it back |
| `attachments` | the source file's other files, such as a `.ork` archive's embedded thrust curves and decal images |

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
  "version": "0.1",
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

The source file's other files are under `attachments`, in the order the file held them. A text
file, such as an embedded thrust curve, is written as its text. Any other file, such as an image, is
written in [base64](https://www.rfc-editor.org/rfc/rfc4648#section-4), a standard way to spell
bytes with 64 printable characters. A `.ork` written from the document puts every one back.

## Reading and writing one

`hpr_format::DesignFile::from_ork` reads a `.ork` into a document, `hpr_format::to_json` writes
its text, `hpr_format::from_json` reads the text back, and `DesignFile::to_ork` writes a `.ork`.
The API reference has a worked example
([`hpr_format`](https://nrdptel.github.io/hpr-sim/api/hpr_format/index.html)).

The text is canonical, meaning there is exactly one way to write a given design: two-space indents,
keys in the order above, and a final newline. So the same design always gives the same bytes.
 Before it returns, the writer reads its own text back and
compares it with the design. A value JSON can't carry, such as an infinite number, is refused with
an error rather than written as something else.

## Versions

A version is two numbers, `major.minor`. The reader checks `format` and `version` before anything
else, so a file of another kind or another version is refused with that reason, not with the
first key it doesn't know:

- **While the major number is 0**, any new minor version may change the document. A reader takes
  its own version, will take older ones by migrating them (rewriting an old document into the
  current version's shape)
  ([M3.3b](../decisions-and-roadmap.md#m3-3b)), and refuses newer ones as
  "written by a newer program".
- **From 1.0**, a new minor version only adds, and a change that breaks old documents takes a new
  major version, with a migration.
- **Version 0.1 is a draft** until hpr's first release: it changes in place, and its schema is
  regenerated. After a release, a change takes a new version, and the old schema stays beside the
  new one.

The reasons are in [ADR-111](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29), the decision that set them,
with the file extensions: `.hpr` for a document, `.hprz` for the zip container to come.

## Extensions and unknown keys

What a source file holds that hpr doesn't model is kept under `extensions`, in a namespace: a key
that starts with `x-` and names the program the data belongs to. Version 0.1 has one, `x-openrocket`: the parts, sections, tags and attributes of
a `.ork` that hpr doesn't read, each kept whole with where it was
([what hpr keeps](ork.md#what-hpr-keeps-for-writing-the-file-back)). That is how a `.ork` written
from a document gets them back.

**A key the version doesn't define is refused, not dropped**, anywhere in the document, an unknown
namespace included. A document either reads whole or says why it doesn't. Keeping another
program's namespace as it is waits for a later version.

## The schema

[`schema/format/hpr-design-0.1.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.1.schema.json)
is the document's JSON Schema. A JSON Schema is a machine-readable description of a JSON
document's keys and types, which many languages can check a file against; this one follows the
standard's 2020-12 edition. It is generated from the Rust types by `cargo xtask format` (one of the
repository's development commands), and a test fails if the committed file is stale. Its
descriptions are the types' documentation. It refuses every key the version doesn't define, as
the reader does.

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
flight to compare: the `.ork` reader leaves them out of the rocket all three ways, 24 for want of a
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

## What is not there yet

- **The zip container** (`.hprz`), for a design with its flight logs and other files: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **Migrations** from older versions: [M3.3b](../decisions-and-roadmap.md#m3-3b), when there is an older version.
- **A comparison** with `.ork`, RockSim's `.rkt`, RASAero's `.CDX1` and RocketPy's `.rpy`, and why
  hpr didn't adopt one of them: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **The command line**: `hpr sim` reads a `.ork` or the bare JSON of a rocket, not yet an `.hpr`,
  and `hpr convert` doesn't write one: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **Generated TypeScript and Python types**: [M3.3c](../decisions-and-roadmap.md#m3-3c).
- **An embedded curve is held twice**: as the file's text under `attachments`, and as the motor
  built from it in the configuration. A flight uses the motor; a `.ork` written from the document
  uses the text. Editing one doesn't change the other, and nothing checks that they agree yet.
- **Some type names come from `.ork`**, such as `OrkMotor`, because the document holds the design
  as hpr's `.ork` reader models it. A later version can rename them, with a migration.
