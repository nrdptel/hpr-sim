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

**How far to trust it.** Converting keeps everything hpr read from the `.ork` except the reader's
warnings. hpr's checks use 75 `.ork` files and read 73. Each of the 73 goes `.ork` → `.hpr` →
`.ork` and comes back as the same
design, bit for bit, and as the `.ork` hpr writes from the original, byte for byte. That `.ork` is
close to the original file but not the same: [what the writer changes](ork.md#writing-a-ork-back-out)
says how. The 109 motor configurations that fly, spread over 30 of the designs, reach the same
apogee from the `.ork`, from the document, and from the `.ork` written from the document, bit for
bit. These counts come from a run on the developers' machine that includes other people's private
designs; the automatic checks repeat the checks, not the counts, on the 17 public ones
([checked on real designs](#checked-on-real-designs)).

**Keep your `.ork` too.** The format is version 0.2, a draft until hpr's first release. A document
of version 0.1 still reads, but 0.1 didn't record whether the rocket's airframe was read exactly as
written. When the document can't show it either, `hpr sim` refuses to fly another motor in it until
the `.ork` is converted again ([versions](#versions)).

## What a document holds

A document is an object with nine keys, always written in this order:

| key | what it holds |
|---|---|
| `format` | always `"hpr-design"`, so a program can tell a design from any other JSON |
| `version` | the format's version, `"0.2"` |
| `provenance` | the program that wrote it and that program's own version (`tool_version`, hpr's version, not the format's); the format and SHA-256 of the file the design came from; and, if the design's airframe was not read from that file exactly as written, why not |
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

**Why the airframe's reading is recorded.** Some `.ork` rockets aren't read exactly as written:
a part is left out, a value dropped, or a size assumed. hpr flies none of their motor
configurations, and `hpr sim` won't fly another motor in them either. A `.ork` written back from the
document spells out every assumed value, so reading it again can't tell that anything was assumed.
The document therefore records the reason, as `provenance.source.airframe_not_as_written`, and
`hpr sim` reads it from there.

**What the document doesn't keep: the `.ork` reader's warnings.** `hpr convert` prints them once,
when it reads the `.ork`; `hpr sim` prints them for a `.ork` but not for a `.hpr` or `.hprz`.

## The container (`.hprz`)

A `.hprz` file is a zip archive holding a design and the files that go with it: flight logs,
results, photographs, anything. Its first entry, `design.hpr`, is the design exactly as a `.hpr`
file, so unzipping a container gives a `.hpr` any reader of the format takes. Every other entry is
an attachment, kept byte for byte, under its name, in its order.

- **Names are relative paths**, with `/` between folders, such as `logs/flight-1.csv`. No part of a
  name may be empty, `.` or `..`, and a name holds no `\`, `:` or control character, so no name can
  climb out of the folder a container is unpacked into. No attachment may be named `design.hpr`, in
  any mix of capitals, nor sit in a top-level folder of that name (`design.hpr/notes.txt`).
- **Names that would unpack as one file are refused**: two names the same but for capitals, a name
  that is also another's folder (`logs` beside `logs/a.csv`), and, for Windows, a part ending in `.`
  or a space, or named as a device (`CON`, `NUL`, `COM1` and the like, with or without an
  extension). No part of a name is longer than 255 bytes, the most common file systems' limit.
  Capitals are compared as Unicode maps them, which also takes `ß` for `ss`, so `straße.csv` beside
  `strasse.csv` is refused though Windows would keep both. Two spellings of one accented letter,
  which macOS can merge, are not caught.
- **At most 256 MiB, unpacked.** The design and its attachments together hold no more: hpr refuses
  to write a bigger container, and to read one, so a small, hostile archive can't fill memory.
- **The same design and files give the same bytes.** Every entry is compressed with deflate, zip's
  usual method, and dated 1 January 1980, zip's zero date, never the clock. (This holds for one
  build of hpr; another deflate library could give other bytes that read back the same.)
- **Reading is held to the same rules**, so a container made by another program is refused with the
  reason rather than half read. The reader checks every name before it unpacks anything. It takes
  the design wherever the archive holds it, and passes over a folder's own entry, which some zip
  tools write, as long as it is empty. It refuses a symbolic link, a name beyond plain ASCII that
  isn't marked as UTF-8 (the mark zip has for it), and two entries of one name,
  which the zip library hpr uses would otherwise read as one, dropping the other without a word.

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

`--attach` puts each file at the top of the container, under its file name; a Rust program can
use folders. In Rust, the `hpr_format` library does each step
([API reference](https://nrdptel.github.io/hpr-sim/api/hpr_format/index.html), with worked
examples):

| function | what it does |
|---|---|
| `DesignFile::from_ork` | reads a `.ork` into a document |
| `to_json` | writes a document's text |
| `from_json`, `read_json` | read the text back, migrating an older version; `read_json` also says which version it was |
| `DesignFile::to_ork` | writes the `.ork` |
| `container::write`, `container::read` | write and read a `.hprz` |

**Editing a document by hand.** hpr flies what the document says, and the `.ork` written from it
carries an edit to the airframe, the recovery or the simulations. (`hpr sim` flies no parachute or
separation yet, so a recovery edit changes only the `.ork`.) The motors are the exception: they are
held twice ([what is not there yet](#what-is-not-there-yet)). `hpr sim` flies the motors under
`rocket.configurations`, and the `.ork` writer takes its motors from `motors.configurations`, so
change a motor in both places, or convert the `.ork` again. Nothing warns when the two disagree.
Any program that checks JSON against a JSON Schema can check an edited document before hpr reads
it ([the schema](#the-schema)). No other design program reads `.hpr` or `.hprz` yet.

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
  A key the reader doesn't know is refused too, so a document from a later build of hpr, with a
  key added within the same version, is refused as an unknown key.
- **While the major number is 0**, the current version may change in place only in a way that
  leaves every document already written readable, with the same meaning. Any other change takes a
  new minor version, with a migration from the one before. The old version's schema stays
  committed, with a document its program wrote, which a test migrates.
- **From 1.0**, a new minor version only adds, and a change that breaks old documents takes a new
  major version, with a migration.

| from | to | what the migration changes |
|---|---|---|
| 0.1 | 0.2 | `attachments` is renamed `source_files`, leaving "attachment" to mean a file in a `.hprz`. Whether the airframe was read exactly as written, which 0.1 didn't record, is worked out from the motor configurations, or marked unknown |

**What the 0.1 migration can't recover.** 0.1 recorded why a `.ork`'s airframe wasn't read exactly
as written only in a motor configuration left out for that reason. The `.ork` reader asks it after
the configuration's motors and before its stages' separation. So a configuration left out for the
airframe gives the reason, and one that flies, or is left out only for its separation, shows the
airframe was read as written. When every configuration was left out for an earlier reason, such as
a missing thrust curve, or there is none, nothing shows it: the migration marks it unknown, and
`hpr sim` refuses to fly another motor in the rocket, saying so. On the 73 designs of hpr's checks,
the migration:

- recovers 5 of the 8 reasons, and marks the other 3 unknown;
- marks 28 of the 65 designs read as written unknown too, so 31 of 73 in all are unknown;
- gives no wrong answer.

Converting the `.ork` again gives a 0.2 document that knows. To see whether a 0.1 document was
marked unknown, convert it to a `.hpr` and look at `provenance.source.airframe_not_as_written`.

The reasons are in [ADR-111](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29), which set the extensions and the first version policy, and
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
| the document, rewritten by the check in version 0.1's shape, migrates back to the same document, but for whether the airframe was read as written, which 0.1 didn't record | 73 of 73 |
| of the 8 whose airframe was not read exactly as written, the migration recovers the reason | 5 of 8; 3 marked unknown |
| of the 65 read as written, the migration marks unknown | 28 of 65 |

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
([the reference library's survey of motors](ork.md#motors-in-the-reference-library)). The other 61 have no
flight to compare: the `.ork` reader leaves them out of the rocket all three ways, by the first reason it finds: 24 for want of a
thrust curve, 19 for stages hpr can't separate as written, and 18 for other reasons
([which configurations fly](ork.md#which-configurations-the-rocket-flies)).

**In CI**, the automatic checks that run on every change don't have the private designs. So
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
  on the curve it embeds, and writes that `.ork` back. Other tests migrate edited 0.1 documents:
  each way the airframe can be recovered, found read as written, or marked unknown, and old
  documents that already hold a 0.2 key, which are refused. A last one migrates each of the 17
  public designs from its 0.1 shape, plus two made from them: one with a parallel stage hpr leaves
  out, and one with an invented embedded curve. Each comes back as the document first written,
  but for the airframe's answer, which is never wrong. With no motor catalog in these tests, no
  public design's configuration flies, so all 17 come out unknown, and only the invented-curve
  design's answer is recovered.
- **The container**: a design with four attachments (a text file, an image, an empty file and a
  name in accents) reads back the same and writes the same bytes again. Each name rule above, two
  entries of one name, a symbolic link, a name not marked as UTF-8, a folder's entry that holds
  bytes, a missing or damaged design, and a
  container past 256 MiB, written or read, are each refused with the reason.
- **The command line**: `hpr convert` takes a public design `.ork` → `.hpr` → `.hprz` → `.ork`, each
  file the library's own, byte for byte. `hpr sim` flies a public design from its `.hpr` and its
  `.hprz` to the same output as from its `.ork`. It refuses another motor in a rocket whose airframe
  wasn't read as written, whichever of the three it reads.

## How it compares with other design formats

Four other formats hold a hobby rocket design. hpr reads `.ork` and writes it back; it doesn't read
the other three yet ([OpenRocket's flights of the private designs](ork.md#openrockets-flights-of-the-private-designs)
lists the reference library's `.rkt` and `.CDX1` files it leaves out).

| | OpenRocket `.ork` | RockSim `.rkt` | RASAero II `.CDX1` | RocketPy `.rpy` | hpr `.hpr` / `.hprz` |
|---|---|---|---|---|---|
| encoding | XML, usually in a zip archive with other files | XML | XML | JSON | JSON, canonical text; a zip archive for `.hprz` |
| published description | a prose page, no schema | in the help of SMARTSim, Apogee's companion program | none: "not documented", its author says | none beyond the code | a JSON Schema, generated and tested |
| version in the file | yes, `major.minor` | yes | yes | RocketPy's version | yes, `major.minor`, with migrations |
| units | mostly SI, not stated in the file | fixed default units, not stated | the program's English units (inches), not stated | SI by RocketPy's convention, not stated | SI, the unit in every key's name |
| motor configurations, stages, recovery | all three | stages, motors and parachutes | a sustainer and up to two boosters; recovery at apogee and at a set altitude | one motor object per rocket (a ring cluster, several motors in a ring, is modelled as one), parachutes, no staging | all three, as `.ork` has them |
| thrust curves and other files | embedded curves, images | motors by name | motors by name | curves as data, no other files | embedded curves and images; any file in a `.hprz` |
| another program's data | a simulation's extension settings | none found | none found | none | `extensions`, one namespace per program |
| what it is | a design | a design | a design, as its outer shape for aerodynamics | a saved `Flight`: the rocket, its environment and the results | a design |

Sources: OpenRocket's [file format page](https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html)
and hpr's own reading of 73 files ([`.ork` design files](ork.md)); Apogee's
[SMARTSim manual](https://www.apogeerockets.com/downloads/PDFs/SMARTSim_manual.pdf), which says
RockSim's rocket files are XML with data "stored using prescribed default units", and sends the
reader to SMARTSim's own help for the elements; RASAero II's
author on its [file format](https://www.rocketryforum.com/threads/openrocket-cnalpha-accuracy.171000/page-2)
and its [user's manual](https://www.rasaero.com/dl_manual_ii.htm), which gives every dimension
in inches; the four `.rkt` and four `.CDX1` files in hpr's reference library, of which only these
facts are published; and RocketPy's code at the
version hpr's checks pin, where `.rpy` came with version 1.10.0
([`save_to_rpy` and `load_from_rpy`](https://github.com/RocketPy-Team/RocketPy/blob/9bd6ad3af8f97bafa3201d4e70eba2877e66e040/rocketpy/utilities.py#L716)).

**Why hpr didn't adopt one of them:**

- **`.ork`**: its page describes the structure and leaves the rest to OpenRocket's source code,
  which is GPL-licensed and which this project doesn't read. It has no schema, and the usual zip
  archive shows no useful difference in git. hpr keeps reading and writing it, as the format most
  designs are shared in.
- **`.rkt`**: RockSim is a commercial program, and its file's elements are described only in the
  help of another of its maker's programs. Its units are implied, not stated.
- **`.CDX1`**: undocumented, in English units, and it describes the rocket's outer shape for
  aerodynamics, not a design to build.
- **`.rpy`**: a saved state of one Python library's objects, not a design format. It has no schema,
  holds one motor and no stages, and a function in it (a parachute's trigger, say) is stored as a
  Python pickle, which runs code when the file is loaded. So a `.rpy` from someone else is only as
  safe as a program from them.

## What is not there yet

- **Generated TypeScript and Python types**: [M3.3c](../decisions-and-roadmap.md#m3-3c).
- **Reading `.rkt`, `.CDX1` or `.rpy`**: no milestone yet.
- **The motors are held twice**: under `motors.configurations`, every configuration in the file as
  the `.ork` holds it, and under `rocket.configurations`, the ones that fly, each motor ready to
  fly. `hpr sim` uses the second and the `.ork` writer the first. Editing one doesn't change the
  other, and nothing checks that they agree yet.
- **An embedded curve is held twice**: as the file's text under `source_files`, and as the motor
  built from it in the configuration. A flight uses the motor; a `.ork` written from the document
  uses the text. Editing one doesn't change the other, and nothing checks that they agree yet.
- **Some type names come from `.ork`**, such as `OrkMotor`, because the document holds the design
  as hpr's `.ork` reader models it. A later version can rename them, with a migration.
- **A container's attachments aren't read by anything yet.** `hpr sim` flies the design and says how
  many it leaves unread; reading a flight log from a `.hprz` waits for the commands that compare a
  flight with its simulation.
