# The hpr design format (`.hpr`)

An `.hpr` file is a rocket design written as one JSON document: hpr's own format, open and
described by a published [JSON Schema](#the-schema). It holds everything hpr reads from an
OpenRocket `.ork`: the rocket's parts, every motor configuration, when each parachute opens and
each stage separates, the simulations the file stored, and what the file holds that hpr doesn't
model. It is plain text, so a design can be kept in git and a change to it reads as a small diff.

**What works today:** a Rust program turns a `.ork` into a document, writes it as text, reads it
back, and writes a `.ork` from it again, with the `hpr_format` library. The zip container that
also carries attachments such as flight logs, migrations from older versions, a comparison with
the other design formats, and the command line reading and writing `.hpr` come next, in
[M3.3b](../decisions-and-roadmap.md#m3-3b) (the format's second step); generated TypeScript and
Python types in [M3.3c](../decisions-and-roadmap.md#m3-3c) (its third).

**How far to trust it.** Nothing is lost on the way through. Each of the 73 designs hpr reads
among the `.ork` files its checks use goes `.ork` → `.hpr` → `.ork` and comes back as the same
design, bit for bit, and flies to the same apogee ([checked on real designs](#checked-on-real-designs)).
The format is version 0.1, a draft: until hpr's first release it can change without a new version
number ([versions](#versions)).

## What a document holds

A document is an object with eight keys, always written in this order:

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

Every number is in SI units, and a key says its unit where it has one: `length_m` is metres,
`kg_m3` kilograms per cubic metre. A [SHA-256](https://en.wikipedia.org/wiki/SHA-2) is a
fingerprint of a file's bytes: it names the source without giving its path or name, which can
name a person. Here is the start of Loft's
[stable trainer](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/ork/loft-demo/demo-stable.ork),
one of the public demonstration designs, as a document:

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

The whole document is 392 lines. A motor that flies is written out in full, its thrust curve
included, so a document flies with no motor database. A `.ork` can embed a motor's curve as a
file inside its archive; the document keeps that file's text, and a `.ork` written from the
document puts it back.

## Reading and writing one

`hpr_format::DesignFile::from_ork` reads a `.ork` into a document, `hpr_format::to_json` writes
its text, `hpr_format::from_json` reads the text back, and `DesignFile::to_ork` writes a `.ork`.
The API reference has a worked example
([`hpr_format`](https://nrdptel.github.io/hpr-sim/api/hpr_format/index.html)).

The text is canonical: two-space indents, keys in the order above, and a final newline, so the
same design always gives the same bytes. Before it returns, the writer reads its own text back and
compares it with the design. A value JSON can't carry, such as an infinite number, is refused with
an error rather than written as something else.

## Versions

A version is two numbers, `major.minor`. The reader checks `format` and `version` before anything
else, so a file of another kind or another version is refused with that reason, not with the
first key it doesn't know:

- **While the major number is 0**, any new minor version may change the document. A reader takes
  its own version, will take older ones by migrating them
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

What a source file holds that hpr doesn't model is kept under `extensions`, by a namespace that
starts with `x-`. Version 0.1 has one, `x-openrocket`: the parts, sections, tags and attributes of
a `.ork` that hpr doesn't read, each kept whole with where it was
([what hpr keeps](ork.md#what-hpr-keeps-for-writing-the-file-back)). That is how a `.ork` written
from a document gets them back.

**A key the version doesn't define is refused, not dropped**, anywhere in the document, an unknown
namespace included. A document either reads whole or says why it doesn't. Keeping another
program's namespace as it is waits for a later version.

## The schema

[`schema/format/hpr-design-0.1.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.1.schema.json)
is the document's JSON Schema, draft 2020-12. A JSON Schema is a machine-readable description of a
JSON document's keys and types, which many languages can check a file against. It is generated
from the Rust types by `cargo xtask format`, and a test fails if the committed file is stale. Its
descriptions are the types' documentation. It refuses every key the version doesn't define, as
the reader does.

## Checked on real designs

The `.ork` reader and writer are checked on 75 `.ork` files, 73 of which hpr reads
([`.ork` design files](ork.md#checked-in-openrocket)). Some are other
people's private designs, so only counts are published. `cargo xtask ork` takes each one through
the format:

| check | designs |
|---|---|
| its document follows the committed schema | 73 of 73 |
| the document reads back as the same document | 73 of 73 |
| the `.ork` written from it is the `.ork` the design itself writes | 73 of 73 |
| that `.ork` reads back as the design first read, bit for bit | 73 of 73 |

Then each motor configuration that flies is flown from both ends: the design first read, and the
design read back from the document and from its `.ork`. Each flight is in calm, standard air at
sea level, off a 1.5 m vertical rail, with its stages separating if they do. Of the designs' 170
configurations, **109 fly, and every one reaches the same apogee from both ends, bit for bit**.
The milestone asked for 1 part in 10⁹. The other 61 have no flight to compare: the `.ork` reader
leaves them out of the rocket both times, most for want of a thrust curve
([which configurations fly](ork.md#which-configurations-the-rocket-flies)).

**In CI**, where the private designs aren't, `hpr-format`'s tests take the 17 public `.ork` designs
under `validation/fixtures/ork/` through the same checks. None carries a curve hpr can fly, so
bundled motors stand in, one for each motor in each design's own size. The 18 configurations fly
to the same apogee from the document, bit for bit, and within 1 part in 10⁹ through the written
`.ork`.

## What is not there yet

- **The zip container** (`.hprz`), for a design with its attachments: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **Migrations** from older versions: [M3.3b](../decisions-and-roadmap.md#m3-3b), when there is an older version.
- **A comparison** with `.ork`, RockSim's `.rkt`, RASAero's `.CDX1` and RocketPy's `.rpy`, and why
  hpr didn't adopt one of them: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **The command line**: `hpr sim` reads a `.ork` or the bare JSON of a rocket, not yet an `.hpr`,
  and `hpr convert` doesn't write one: [M3.3b](../decisions-and-roadmap.md#m3-3b).
- **Generated TypeScript and Python types**: [M3.3c](../decisions-and-roadmap.md#m3-3c).
- **Some type names come from `.ork`**, such as `OrkMotor`, because the document holds the design
  as hpr's `.ork` reader models it. A later version can rename them, with a migration.
