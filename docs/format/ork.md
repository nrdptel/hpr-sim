# OpenRocket `.ork` design files

A `.ork` file is an OpenRocket design: the tree of parts a rocket is built from, the materials they
are made of, the motors flown in it, and the results of the simulations OpenRocket last ran. It is
the format hobby designs are most often shared in, so reading it is how a design gets into hpr
without being typed again.

**What works today:** hpr opens a `.ork` file, whichever of its three containers it is in, reads
its design document into a tree that keeps everything the file said, and builds a rocket out of
that tree — the stages and body components stacked in them, and the tubes, rings, fins, lugs,
buttons and recovery gear on and inside each of those, with their shapes, materials, surface
finishes, positions and overrides — and its motor configurations, with the motors in them and
their thrust curves, when its parachutes open and its stages separate, and the simulations
OpenRocket last ran on it. That is from Rust; there is no command-line tool yet.

**How far to trust it.**

- **The shape is cross-checked.** The airframe's key geometry was compared with a second program
  that reads `.ork` files, RocketSerializer, and with OpenRocket itself. That geometry is the nose
  cone, the transitions, the fin sets, where each sits, and the body radius. Over 74 designs (all
  but one of the 75 hpr lays out; OpenRocket 24.12 will not open the 75th), hpr's value equals
  OpenRocket's for all 1,212 numbers. Where parts sit is checked against OpenRocket alone; mass
  and the centre of gravity are checked in [Mass properties](../physics/mass.md#checked-against-openrocket)
  ([checked against RocketSerializer](#checked-against-rocketserializer)).
- **Few motor configurations fly yet.** Motors are read, but a configuration flies only when every
  motor in it lights at launch, every motor has a thrust curve (in the file or in hpr's small
  bundled catalog), and the airframe was read without a warning. That is **2 of the 174 motor
  configurations** in the reference library's 75 designs
  ([motors](#motors-and-their-configurations)).
- **Recovery is read, not flown.** Recovery and separation settings are read, but no flight uses
  them yet ([when parachutes open](#when-parachutes-open-and-stages-separate)).
- **Pods and parallel stages are kept, not modelled.** A design with them is marked *reduced*, and
  none of its configurations flies ([what hpr keeps](#what-hpr-keeps-for-writing-the-file-back)).
- **Some parts are left out.** A part hpr cannot give an honest shape, such as fins on a nose cone
  or tube fins OpenRocket sizes from the body, is left out. Each one is named in a warning rather
  than guessed at ([what is left out, and why](#what-is-left-out-and-why)).
- **Every design in the reference library lays out**, meaning every part gets a position and a
  radius. A radius the file leaves with nothing to be worked out from gets OpenRocket's own
  default of 25 mm, with a warning
  ([when an automatic radius has nothing to take](#when-an-automatic-radius-has-nothing-to-take)).

## Opening a file today

This is the doctest on `hpr_io::ork`, which CI runs. It stops at the document — the tree of
elements the file said, with nothing interpreted. To go one step further and get an
`hpr_design::Rocket` out of that tree, see
[opening a design, in full](#opening-a-design-in-full).

```rust
let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Sounder</name></rocket>
</openrocket>"#;

let read = hpr_io::ork::read(xml)?;
assert_eq!(read.value.container, hpr_io::ork::Container::Xml);
assert_eq!(read.value.document.version.to_string(), "1.10");
let rocket = read.value.document.root.child("rocket").expect("a rocket");
assert_eq!(rocket.child("name").expect("a name").text(), "Sounder");
assert!(read.warnings.is_empty());
```

`read` takes bytes, not a path: `hpr-io` does no file I/O, so that it can also run in a browser.
Hand it the bytes of a real `.ork` and the three containers are all the same call. What comes back
is an `OrkFile` — the container, the document, and the archive's other entries — beside the
warnings the read raised.

Code: `hpr_io::ork` ([API reference](../api/hpr_io/ork/index.html)), written for
[M3.1a](../decisions-and-roadmap.md#m3-1a). Rules below are from the format documentation unless
marked **Observed** (seen in real files) or **Policy** (hpr's own choice).

## Sources

- **[F]** OpenRocket file format documentation,
  <https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html>, and the
  `fileformat.txt` shipped with the program. There is **no XSD**: nothing machine-checkable
  describes a `.ork`, and every OpenRocket release has added tags.
- **Observed:** 78 `.ork` files, all cached under `refs/` and never committed: 27 from the private
  design corpus, 20 hand-authored fixtures from [Loft][loft] and 1 from Debrief (hpr's two
  predecessors), 17 example designs inside the pinned `OpenRocket-24.12.jar`, 9 from the
  `openrocket-database` parts library, and 4 cached elsewhere. By the files' own `creator`
  attribute, **58 of the 76 that open were written by OpenRocket** and 18 were hand-authored, so
  where a count says what a real OpenRocket writes it is given over those 58. Every count below is
  printed by `cargo xtask ork` unless it names another source.
- OpenRocket's own Java source is **not** consulted: it is GPL, and hpr is MIT OR Apache-2.0
  ([ADR-051][adr-051]): hpr is built from published documentation and real
  files, never from another program's source, which is what "clean room" means here.

## The three containers

The first bytes say which one a file is in ([F]; [Loft lesson L56](../decisions-and-roadmap.md#l56): Loft told them apart by their magic
bytes, and a malformed file had to give an error rather than crash):

| container | first bytes | what it holds |
|---|---|---|
| zip | `50 4b 03 04` (`PK␃␄`) | an entry called `rocket.ork` — the design as XML — beside anything else the design carries |
| gzip | `1f 8b` | the design document, compressed on its own. What older OpenRocket versions wrote |
| XML | `<`, after an optional byte-order mark and blank lines | the design document itself |

**Observed:** of the 76 files that open, 73 are zip and 3 are plain XML — and two of those three
are the same design, one of them the unzipped document of the other, so plain XML rests on two
designs. No gzip `.ork` survives in the reference library at all, so that path is held by a test
rather than by a real file.

**Policy.** Inside a zip, the design is the entry named `rocket.ork`. If there is none, the first
entry whose name ends in `.ork` or `.xml` is read as the design and a warning says so. Every other
entry is kept byte for byte as an *attachment*: `thrustcurves/*.rse` motor curves (schema 1.11),
`preview.png`, lookup tables. **Observed:** 38 `.png`, 11 `.jpg`, 3 `.gif` and 3 `.rse`
attachments across the corpus — but all three `.rse` curves come from the single schema 1.11 file,
so the 1.11 attachments [M3.1c](../decisions-and-roadmap.md#m3-1c) will read
([Loft lesson L57](../decisions-and-roadmap.md#l57): Loft threw them away) are a sample of one.
Nothing reads them yet; they are kept so that
[M3.1c](../decisions-and-roadmap.md#m3-1c) can, and so an export can put them back.

## The schema version

The root element carries it: `<openrocket version="1.10" creator="OpenRocket 24.12">`. Version 1.9
is OpenRocket 23.09, 1.10 is 24.12, and 1.11 (26.xx, documented but not yet released) adds embedded
`.rse` curves, CSV lookup tables, a gravity model and `preview.png`.

**Observed**, across the corpus:

| schema | files | written by |
|---|---|---|
| 1.4 | 4 | OpenRocket 13.05, 15.03 |
| 1.5 | 10 | OpenRocket 15.03 |
| 1.8 | 5 | OpenRocket 22.02 |
| 1.9 | 3 | OpenRocket 23.09 |
| 1.10 | 53 | OpenRocket 24.12, and hand-written fixtures |
| 1.11 | 1 | an OpenRocket 26.xx snapshot |

**Policy.** A version past 1.11 is read anyway, with a warning: a newer file is mostly an older
file with tags added, and refusing it outright would help nobody. A version that is not
`major.minor` is an error.

## The document

The design document is read into a tree of elements and text and nothing is interpreted. That is
deliberate: with no schema to check against, the only way to be sure a later step has not quietly
dropped something is to keep the whole file and be able to write it back.

**Policy**, and what the tree keeps:

- Every element, in document order, with its attributes in the order they were written.
- The text of a leaf element exactly as written, including leading and trailing spaces —
  `<name> Sounder </name>` is not the same as `<name>Sounder</name>` until something decides to
  trim it.
- Text an element holds *beside* child elements. **Observed:** OpenRocket writes this, and only
  here — a simulation's `<warning>` prints its own message after its fields, in 48 elements of 19
  files, and `cargo xtask ork` finds no other tag that does it.
- The blank text that only lays the file out — the indentation between child elements — is **not**
  kept, so that writing the document out again is free to lay it out afresh.
- An XML comment or processing instruction is dropped, and counted as a warning. No `.ork` tag
  carries meaning in one. A comment, a processing instruction or a CDATA section splits a run of
  text in two as it is parsed; the pieces are joined back, because dropping the comment leaves the
  writer nowhere to put the split.

The one thing the tree does **not** keep is XML namespaces: a prefix and its declaration are
dropped, and two attributes differing only by prefix become one. No `.ork` OpenRocket writes uses
them, and a document that declares any says so in a warning.

Writing the document back out uses one fixed style (not W3C Canonical XML, which is a different
thing): two-space indentation, `<tag/>` for an empty element, and character references for the characters that would otherwise change when read again
(a carriage return in text; a tab, newline or return in an attribute). **Reading a document,
writing it, and reading it again gives the same document** — that is what "keeps everything" means
here. **Invariant (tested):**
`parse(to_xml(parse(x))) == parse(x)` — by `hpr_io::ork::tests::any_document_written_and_read_again_is_unchanged`
over generated trees, by `reading_writing_and_reading_again_gives_the_same_document` and
`awkward_text_and_attributes_survive_being_written` on awkward text, and by `cargo xtask ork` over
every real file in the corpus.

## The values inside the tags

A component's numbers sit in leaf elements, and three things about them are not obvious. All three
are mistakes [Loft][loft] made, and each is settled here by what the corpus shows rather than by a
specification, because `.ork` has none. Code: `hpr_io::ork::value`
([API reference](../api/hpr_io/ork/value/index.html)), decided in [ADR-052][adr-052].

**A dimension may be automatic.** `<aftradius>auto 0.025</aftradius>` means "OpenRocket works this
out from the neighbouring components, and 0.025 m is what it last worked out" (so OpenRocket 24.12
does; older releases may differ, [see below](#checked-against-the-answers-openrocket-cached)). A bare `auto` —
`<outerradius>auto</outerradius>` — is the same with nothing worked out yet, and is the commoner
form: 309 of the 413 automatic dimensions in the corpus cache no number, so a reader that resolves
them cannot treat the cached value as a shortcut. hpr keeps both halves: which it is, and the cached number.
Keeping only the number is
[Loft lesson L58](../decisions-and-roadmap.md#l58) — it turned automatic dimensions into hand-typed
ones the next time the design was saved. **Observed:** 413 automatic dimensions across the corpus,
on seven tags.

| tag | automatic |
|---|---|
| `outerradius` | 131 |
| `innerradius` | 80 |
| `cd` | 79 |
| `radius` | 43 |
| `packedradius` | 36 |
| `aftradius` | 30 |
| `foreradius` | 14 |

**A tag may be written under two names.** OpenRocket renamed several and writes both, so an older
reader still finds one. Two of those renames are that and nothing more, and hpr reads either name,
taking the newer. **Observed:** on every element that carries both, the two agree — on the text,
and on the `type`/`method` attribute that says what the number is measured from.

| newer | older | elements with both | agree on text | differ on frame |
|---|---|---|---|---|
| `axialoffset` | `position` | 642 | 642 | 0 |
| `instancecount` | `fincount` | 109 | 109 | 0 |

Two that disagree is not something OpenRocket writes, so it raises a warning — whether they
disagree on the number or on where the number is measured from.

**Three more pairs look the same and are not.** The newer name of each carries a `method`
attribute — the frame the number is measured in — that the older name never carries:

| newer | older | elements with both | agree on text | differ on frame |
|---|---|---|---|---|
| `angleoffset` | `rotation` | 95 | 95 | **95** |
| `angleoffset` | `radialdirection` | 26 | 26 | **26** |
| `radiusoffset` | `radialposition` | 0 | — | — |

For a long time hpr read neither name of any of them. The two angle pairs agree on the number
every time and differ on the frame every time; `radiusoffset` (on 106 elements) and
`radialposition` (on 542) are never written together at all, so nothing about *them* had been
measured. Reading one as the other would move a component without saying so.

[M3.1b3](../decisions-and-roadmap.md#m3-1b3) settled all three without ever having to say what the
older name's frame is — see
[how far off the axis](#the-parts-on-and-inside-the-body), below.

**A stated zero is a value.** `<overridecd>0.0</overridecd>` means no drag at all, not "no
override" — reading it as missing is
[Loft lesson L63](../decisions-and-roadmap.md#l63), which charged a zero-drag part full drag. A
component declares its own mass, centre of gravity and drag coefficient with three tags, and
whether each covers the components inside it with three more, all read independently.

**Observed** override tags: `overridemass` 118, `overridesubcomponentsmass` 95,
`overridesubcomponents` 20, `overridecg` 16, `overridesubcomponentscg` 9, `overridecd` 2,
`overridesubcomponentscd` 2. The third of those is the single flag that the three per-quantity ones
replaced before schema 1.9. **Policy:** it is read as setting all three — which is what it meant —
and says so in a warning. No element in the corpus carries it beside a per-quantity flag, so the
reading cannot contradict a file.

## Warnings, not failures

A design written by an older OpenRocket, or by another program, should still open. Everything that
departs from [F] but leaves the file readable is a warning that travels with the result
(`hpr_io::ork::Warning`), not an error:

| kind | means | raised for |
|---|---|---|
| `Skipped` | a whole part was left out | a **component** this reader cannot give an honest shape ([below](#what-is-left-out-and-why)); an **attachment** entry that could not be decompressed, or one that would pass the unpacking limit; a damaged *design* entry is an error, not a warning |
| `Dropped` | a value was ignored | a comment or processing instruction; an XML namespace; a tag whose text is not the number, count or flag it should be; two names for one value that disagree; a dimension the file does not give, read as zero; a fin's fillets or a rail button's screw head, whose mass hpr does not model |
| `Unusual` | read as it stands | a schema version past 1.11; no `creator` attribute; a design entry not called `rocket.ork`; the single pre-1.9 subcomponent-override flag; a surface finish or an axial-offset method this reader has no rule for; an automatic radius with nothing to take, given OpenRocket's 25 mm default; a `<rocket>` holding nothing |

**Observed:** reading the corpus's containers and documents raises **no warnings at all** — every
file that opens is ordinary. Building a *rocket* from those documents raises 57: 16 dropped, 12
skipped and 29 unusual, over 76 files. Every kind of warning the container and document readers
can raise is therefore exercised by a test rather than by a file anyone shipped.

Only these stop a read:

- bytes that are none of the three containers;
- a damaged zip or gzip;
- a zip with no entry that could be the design;
- a design that is not UTF-8, or not well-formed XML;
- a root element that is not `<openrocket>`;
- a `version` attribute that is missing or is not `major.minor`;
- a document that nests more than 64 elements deep;
- an archive that unpacks to more than 256 MiB.

### Why there are two limits

A deflate stream can expand by about a thousand to one, so a `.ork` of a megabyte can ask for more
than a gigabyte of memory, and one of forty megabytes can ask for more than a machine has. Running
out is an abort, not an error. So a read decompresses at most 256 MiB out of one archive — the
largest in the corpus unpacks to 2,052,024 bytes, document and attachments together — and an entry
that would pass the limit is left out with a warning. If the entry left out was the design, the read fails rather than
returning half a file. `hpr_io::ork::container::unpack_within` takes another limit, for a caller
with less memory to spend.



Reading descends the tree, and so does the XML parser underneath. A file written to nest deeply
enough exhausts the stack, which is a crash rather than an error, where
[Loft lesson L56](../decisions-and-roadmap.md#l56) asks for an error. Feeding `roxmltree` documents
of increasing nesting in a debug test build, on a 2 MiB test-thread stack, it read 120 levels and
died on 130 — the process aborts, so this one measurement cannot itself be a committed test, and
the figure moves with the stack a platform gives a thread. That is the argument for not relying on
it.

So hpr counts the nesting **before** the text reaches the parser, with a scan that skips comments,
CDATA and processing instructions and tracks quotes, and refuses anything past 64 levels. The scan
can only ever count *more* levels than a parser will descend — XML forbids a raw `<` in an
attribute value, so every element start it sees is a real one — and
`hpr_io::ork::tests::the_depth_scan_never_undercounts` holds it to that over generated documents.

**Observed** for the depth, over the 76 files that open: 53 nest 11 deep, and the distribution runs 4, 7, 9, 10,
11, 12, 13, 14 and 17. An ordinary single-stage design reaches 11; the deepest, at 17, is
OpenRocket's own parallel-booster example, where each nested stage or inner tube costs two levels
and a component's appearance three. So 64 leaves about three more levels of nesting than anything
anyone has written.

## Checked against real files

`cargo xtask ork` reads every `.ork` in the reference library and in the OpenRocket jar's example
set, writes them back out, reads them again, and reports counts. The private corpus stays private:
the per-file detail goes to a gitignored `corpus-out/`, and only counts are published. On
2026-09-20:

| | |
|---|---|
| files found | 78 |
| opened | 76 |
| written and read back unchanged | 76 |
| warnings raised reading the container and the document | 0 (building a *rocket* from them raises 88; see [below](#measured-on-the-reference-library)) |
| refused, not well-formed XML | 2 |
| deepest nesting | 17 |
| elements holding text beside children | 48, in 19 files |
| largest unpacked (document and attachments) | 2,052,024 bytes |

The two that do not open are hand-written fixtures for Loft's browser tests, and neither is XML: a
`<databranch>` is closed with `</flightdata>`. Python's `expat` refuses both at the same lines hpr
does, so this is the files' fault and not the reader's. They are listed by name in the survey,
which fails if either one ever behaves differently.

Here the *reference library* is every `.ork` under `refs/`, fetched by `cargo xtask refs fetch`;
the 17 example designs inside the OpenRocket jar (also under `refs/`) are counted with it unless a
table lists them apart.

**Every other file imports without an error.** An import error is a file that does not read, or a
design that reads but does not lay out. Warnings are not errors, because a warning never stops an
import. The survey counts both kinds of error for each source. On 2026-09-21:

| source | files | read | laid out | errors |
|---|---|---|---|---|
| the private design library (`loft-fixtures`) | 27 | 27 | 27 | 0 |
| the OpenRocket 24.12 jar's examples | 17 | 17 | 17 | 0 |
| everything else under `refs/` | 34 | 32 | 31 | 2 (the two Loft fixtures that are not XML) |

One file in the last row reads but holds no design, so it has nothing to lay out.

**What you can check yourself.** The corpus is not public, so these counts are not reproducible on
a fresh clone: `cargo xtask ork` needs `cargo xtask refs fetch` first, and stops with
"no .ork files found" without it. What *is* reproducible anywhere is everything the tests cover —
`cargo test -p hpr-io` — and the same command on any `.ork` files you have, with
`cargo xtask ork --dir <path>`.

[api]: https://nrdptel.github.io/hpr-sim/api/hpr_design/tree/struct.Rocket.html
[adr-051]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20
[adr-052]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-052-what-a-ork-value-means-automatic-dimensions-two-names-for-one-tag-and-overrides-2026-09-20
[loft]: https://github.com/nrdptel/fusionspace-loft
[debrief]: https://github.com/nrdptel/fusionspace-debrief

### Snapshots of public designs

Seven small designs from [Loft][loft], the project owner's earlier tool, are committed in
`validation/fixtures/ork/loft-demo/` (MIT). The test
`hpr_io::ork::tests::loft_demo_designs_read_as_snapshotted` reads each with `hpr_io::ork::design`,
and a synthetic design whose motor flies from the bundled catalog besides, and compares a summary of
what it reads with a committed [`insta`](https://insta.rs) snapshot. A snapshot is a saved copy of
the output that the next run must match. Each summary records:

- the stages and body components;
- the structure's mass and centre of mass, to nine significant figures;
- the motor configurations, and which of them fly;
- the recovery settings and the stored simulations;
- what is kept in [`x-openrocket`](#what-hpr-keeps-for-writing-the-file-back), and every warning.

**A snapshot shows that the reading has not changed, not that it is right.** None of these numbers
is compared with another program here; the geometry is, in
[the next section](#checked-against-rocketserializer). For example, `demo-stable.ork` is a 38 mm trainer on
an AeroTech H128W. Its snapshot says the structure weighs 0.540 kg and that its one configuration
does not fly (`"flown": []`), because that motor has no thrust curve in the file or in hpr's small
bundled catalog ([motors](#motors-and-their-configurations)). Each field's meaning is in the section
of this page that reads it.

**When a snapshot changes,** the test fails and shows the old and new output. Run
`cargo insta review` (from `cargo install cargo-insta`) to see them side by side, and accept only a
change you can explain (or rerun with `INSTA_UPDATE=always cargo test -p hpr-io` and read the
diff in git); the new `.snap` file goes in the same pull request. The private reference
library is never snapshotted; `cargo xtask ork` reports it only as counts, as above.

### Checked against RocketSerializer

**In short.** [RocketSerializer][rocketserializer] is a second program that reads `.ork` files.
RocketPy's team wrote it to turn an OpenRocket design into RocketPy's inputs. This check compares
hpr's reading of each design's shape with RocketSerializer's. Where the two differ, OpenRocket
itself, run on the same file, decides which is right.

On 2026-09-21 the check covered 74 designs. Of 1,212 numbers:
- hpr matched RocketSerializer on 1,102;
- on the other 110, OpenRocket gave hpr's number, not RocketSerializer's;
- hpr never differed from both.

**Every one of hpr's 1,212 numbers is also OpenRocket's.** That includes the 1,102 where it matched
RocketSerializer. So no agreement here is a mistake the two readers happen to share.

This checks the *reading*: the dimensions in the file, and where each part sits. It does not check
mass, the centre of gravity, or any physics. Comparing those with OpenRocket's is
[M2.2](../decisions-and-roadmap.md#m2-2), the next roadmap step.
The check was added by [M3.1d2](../decisions-and-roadmap.md#m3-1d2), the roadmap step that finished
reading `.ork` files, and [ADR-059][adr-059] records how it was decided.

**What is compared.** What RocketSerializer reports about the airframe's shape:

| part | numbers |
|---|---|
| the nose cone | its shape, length and base radius, and, for a Haack series nose, its shape parameter (0 for a Von Kármán nose) |
| each transition | its length, and its radius at each end |
| each trapezoidal or elliptical fin set | the number of fins, root chord, tip chord, span, sweep, [cant](../glossary.md#cant) and cross-section (square, rounded or airfoil edges) |
| each of those parts | its [station](../glossary.md#station): where its front sits, in metres aft of the nose tip |
| the rocket | its body radius: the largest radius the file writes as a number |

RocketSerializer also reports each part's name, and a fin set's sweep angle when the file writes
one (the files here write the sweep as a length); those are not compared.

**How a difference is settled.** A script, `validation/oracles/rocketserializer/geometry.py`, runs
RocketSerializer's *extractors*, the functions it uses to pull each part out of a file, on every
design. It also asks OpenRocket 24.12, loaded on the same file, for each of the same numbers. It
writes what both programs read to a JSON file, the *record*. `cargo xtask ork` then compares hpr's
reading with the record. Two numbers count as the same when they differ by less than 1 part in 10⁹:

- If hpr's number is RocketSerializer's, they **agree**.
- If it is not, but it is OpenRocket's, then **RocketSerializer differs**.
- Otherwise **hpr differs**, and the survey fails.

Whatever RocketSerializer says, the survey also fails when one of hpr's numbers is not
OpenRocket's. Otherwise a mistake hpr and RocketSerializer made together would pass as agreement.
One such case is known. OpenRocket limits a fin's cant to 15°, while hpr and RocketSerializer take
a larger cant as written, so the survey would fail on such a file.
[Issue #148](https://github.com/nrdptel/hpr-sim/issues/148) tracks what hpr should do. No file here
cants a fin past 15°.

**A worked example.** Loft's public `demo-dual-deploy.ork` puts its fin set at the bottom of the
booster tube. The same tube also holds the drogue parachute, packed 0.08 m long, listed before the
fins in the file. The parachute sits inside the tube, so it adds nothing to where the fins are.
The file places the fins relative to the tube's aft end, and OpenRocket and hpr both put the fins'
front 1.45 m aft of the nose tip. RocketSerializer adds
up the lengths of every part listed before the fins in the same tube, the parachute's 0.08 m
included, so it puts them at 1.45 + 0.08 = 1.53 m. The record keeps that sum of earlier lengths
for every part, so this cause is checked, not assumed.

**The results.** From `cargo xtask ork` on 2026-09-21, over the 78 files it reads (the reference
library under `refs/` and the 17 examples inside the OpenRocket jar):

| | all designs | each file once |
|---|---|---|
| designs compared (files OpenRocket opens) | 74 | 54 |
| numbers compared | 1,212 | 896 |
| hpr agrees with RocketSerializer | 1,102 | 813 |
| RocketSerializer differs, and hpr's number is OpenRocket's | 110 | 83 |
| hpr differs from both | **0** | **0** |
| hpr's number is OpenRocket's | 1,212 | 896 |

Some files are in the library more than once: 13 of Loft's private designs are copies of the jar's
examples, for one. The second column counts each file's content once.

The 110 differences, by cause:

| cause | count |
|---|---|
| a fin set's station: RocketSerializer adds the lengths of the parts before it in its parent, as in the worked example | 75 |
| a transition's radius: RocketSerializer looks transitions up in OpenRocket by name, and takes the first of that name | 15 |
| no cause shown | 20 |

**Stations rest on OpenRocket.** RocketSerializer does not read a station, or a transition's
radius, from the file. It loads the design into OpenRocket and walks OpenRocket's tree, adding up
lengths as it goes, which is where the worked example's extra 0.08 m comes from. So its stations
are not an independent reading. It agrees with hpr on 8 of the 96 fin sets' stations and 18 of the
22 transitions'; the rest are among the 110 above, where hpr's station is OpenRocket's. For
stations this is really a check against OpenRocket alone. For lengths, chords, spans, counts and
cant, which RocketSerializer reads from the file, it agrees with hpr every time.

For the 20 with no cause shown, the reason RocketSerializer's number differs has not been traced.
In each of them hpr's number is OpenRocket's, and the per-file record holds all three programs'
numbers:
- 17 are stations, 13 of fin sets and 4 of transitions.
- 1 is a nose cone's base radius, and 1 is the body radius.
- 1 is a nose cone's shape: a nose written as an `ogive` of shape parameter 0. OpenRocket draws it
  as a cone, and so does hpr, while RocketSerializer passes on the word `ogive`.

**Four things OpenRocket does that the check allows for.**

- **A design worked out again.** OpenRocket's first reading of an automatic radius can differ
  from the answer it settles on once it works the design out again
  ([when an automatic radius has nothing to take](#when-an-automatic-radius-has-nothing-to-take)).
  The script saves each design once, to a throwaway, before it reads anything, so every number in
  the record, RocketSerializer's included, comes from the settled design.

- **A canted fin.** OpenRocket turns a canted fin about the middle of its root chord. That moves
  the front of the root aft by half the chord times (1 − cos δ), where δ is the cant: 38 µm for a
  0.495 m root at 1°, as in the OpenRocket jar's *Simulation extensions* example. The file places the root before it is turned, and so does hpr. So the script
  reads OpenRocket's station with the cant set to zero, then puts the cant back. It keeps the
  turned station too. For all 10 canted fin sets, the turned station is aft of the unturned one by
  exactly that amount, and `cargo xtask ork` checks it.
- **A shape word.** An ogive of shape parameter 0 is a cone
  ([the spine](#the-spine-stages-and-body-components) says why). To tell which shape OpenRocket
  really draws, the script records OpenRocket's radius a quarter, a half and three quarters of the
  way along the nose. hpr's profile is compared with those three radii: all 72 noses
  RocketSerializer reports match. So a
  nose's shape counts as OpenRocket's when OpenRocket draws hpr's profile, whatever word it uses.
- **A leading comment.** OpenRocket 24.12 refuses a file that begins with a long enough comment. For
  3 files, the script removes the comment from the copy OpenRocket reads, and the record says so.

**What it leaves out.**
- RocketSerializer reports no body tube, no inner part, no mass, no fin thickness, and no freeform
  or tube fins, so none of those is compared here.
- 6 parts inside pods or parallel stages are not compared, because hpr keeps those parts unread
  ([what hpr keeps](#what-hpr-keeps-for-writing-the-file-back)).
- 3 values RocketSerializer gives nothing for are not compared: 1 body radius, in a file that
  writes every radius as `auto`, and 2 fin cross-sections, in a file that writes none.
- 4 files are not compared, because OpenRocket 24.12 does not open them. One is Loft's
  `demo-quirks.ork`, whose parallel stage sits directly under the rocket. Two are the Loft fixtures
  above that are not XML. The fourth holds no design.

**Run it yourself.** CI does not run RocketSerializer or OpenRocket. It holds hpr to their saved
output for the seven public Loft designs, `validation/fixtures/ork/rocketserializer-loft-demo.json`,
with `cargo test -p xtask rocketserializer`. OpenRocket opens six of the seven. Of their 80
numbers, 74 agree, and the other 6 are fin stations that RocketSerializer adds earlier lengths to,
as in the worked example.

To make a record yourself you need Python 3.11, [`uv`](https://docs.astral.sh/uv/), Java 17 and
the OpenRocket 24.12 jar (`cargo xtask refs fetch` downloads the jar to `refs/openrocket/`). From
the repository root:

```sh
# Once: an environment of its own, with every package pinned.
uv venv -p 3.11 refs/venv-rs
uv pip install -p refs/venv-rs --no-deps -r validation/oracles/rocketserializer/requirements.txt

# The seven public designs, written over the committed record; nothing should change:
refs/venv-rs/bin/python validation/oracles/rocketserializer/geometry.py \
    validation/fixtures/ork/rocketserializer-loft-demo.json validation/fixtures/ork/loft-demo
git diff validation/fixtures/ork/

# The whole reference library and, with --jar, the jar's example designs; then compare:
refs/venv-rs/bin/python validation/oracles/rocketserializer/geometry.py \
    corpus-out/rocketserializer.json refs --jar
cargo xtask ork
```

For your own files, give the script their directory in place of `refs`: it writes what
RocketSerializer and OpenRocket read. Comparing hpr's numbers with them is not automated yet;
`cargo xtask ork` compares hpr with the record of the reference library only.

[rocketserializer]: https://github.com/RocketPy-Team/RocketSerializer
[adr-059]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-059-the-rocketserializer-cross-check-three-readers-with-openrocket-settling-a-difference-2026-09-21

## The spine: stages and body components

The trunk of a design is its **spine**: the stages, and inside each of them the nose cones, body
tubes and transitions that stack end to end along the axis. `hpr_io::ork::rocket` turns a document's
spine into an [`hpr_design::Rocket`][api] — shapes, lengths, radii, walls, materials and overrides —
and everything else (the tubes and rings inside the body, the fins and lugs on it, the recovery
gear) is counted and left for [M3.1b3](../decisions-and-roadmap.md#m3-1b3).

**An automatic radius is marked, not filled in.** Where the file says `auto`, the component carries
the dimension's name and `Rocket::layout()` works the radius out from the neighbours — including
across a stage boundary, so a booster's first component takes its radius from the stage ahead of
it. That is [Loft lesson L59](../decisions-and-roadmap.md#l59): Loft resolved within a stage only,
and a booster came out as whatever number happened to be cached. A stated wall is also kept when
the radius it sits in is automatic; judging the wall against a radius that is not known yet threw
it away, which is half of [Loft lesson L61](../decisions-and-roadmap.md#l61).

**The shape parameter is not the same number.** OpenRocket writes the ogive's parameter as
`κ = ρ_tangent/ρ` — a tangent ogive's radius of curvature over this one's — so `κ = 1` is a tangent
ogive and `κ = 0` an infinite radius, which is a cone. `hpr-design` states the same shape the other
way up, as `ρ/ρ_tangent`, so the two are reciprocals: reading one as the other turns every secant
ogive into a bulged one. The power, parabolic and Haack parameters carry over unchanged. All four
are Niskanen's appendix A, equations A.3 and A.7 to A.9.

<a id="not-settled"></a>

**Readings this page is not sure of**, every one of them for the OpenRocket oracle
([M2.2](../decisions-and-roadmap.md#m2-2)) to settle. The first is the spine's; the rest come from
the parts. Two readings this table held are settled: a shoulder, and a tube, of zero wall thickness
weigh nothing, as in OpenRocket 24.12, measured in [M2.2b1](../decisions-and-roadmap.md#m2-2b1)
([ADR-061][adr-061]).

| what the file says | how it is read | why it is in doubt |
| --- | --- | --- |
| no `shapeclipped` on a transition | clipped | it is the shape that reaches both radii; only 1 of the 21 transitions in the corpus states it |
| an angle, **which way it turns** | the same way hpr's own frames turn | hpr measures a roll angle right-handed about an axis pointing at the **nose**; OpenRocket's technical documentation puts its own `x` axis along the centreline pointing **aft** and leaves the rest unstated. If it means what that implies, every angle read here is mirrored — see [below](#which-way-round) |
| a `polished` finish | 2 µm | the number is the author's, from 2013; a newer OpenRocket may have moved it ([below](#the-surface-finish)) |

**Measured on the reference library** (`cargo xtask ork`, 76 readable files): 73 designs' spines lay
out, over 93 stages and 285 body components — 188 body tubes, 74 nose cones, 23 transitions — with
81 automatic radii marked (41 outer, 19 base, 14 fore, 7 aft); since
[M3.1b4](../decisions-and-roadmap.md#m3-1b4) gave 7 of them OpenRocket's default, 74 remain marked. The counts are what may be
published; the per-file detail stays in the gitignored `corpus-out/`.

*(When this was written, 3 designs did not lay out, and they were thought to be waiting on parts that
were not read yet. Reading those parts, in [M3.1b3](../decisions-and-roadmap.md#m3-1b3), showed
otherwise. One document holds no design at all, and two have radii with nothing to take, which
[M3.1b4](../decisions-and-roadmap.md#m3-1b4) settled:
[when an automatic radius has nothing to take](#when-an-automatic-radius-has-nothing-to-take).
All 75 designs lay out now.)*


## The parts on and inside the body

Everything that is not the spine hangs off it, and hpr reads it into the same
[`hpr_design::Rocket`][api]. Four families, with the words this page uses for them:

- **Tubes inside a body component** — an *inner tube* (usually the motor mount), a *coupler* (the
  short tube that joins two airframe sections), an *engine block* (the ring that stops a motor
  sliding forward). Each can hold parts of its own.
- **Rings that centre them** — a *centering ring*, a disc with a hole (its **bore**) that holds a
  motor tube on the airframe's axis, and a *bulkhead*, the same disc with no hole.
- **What sits on the outside** — fin sets, tube fins, *launch lugs* (the tubes a launch rod passes
  through) and *rail buttons*.
- **What is packed in the bore** — mass objects (an altimeter, a battery), parachutes, streamers
  and shock cords. hpr reads the cylinder each takes up when packed, not what it does when it opens.

Code: [`hpr_io::ork::attached`](../api/hpr_io/ork/attached/index.html), decided in
[ADR-053: the parts on and inside a `.ork` body][adr-053].

### Opening a design, in full

`rocket` reads the airframe alone. To get the motors as well, call `hpr_io::ork::design` instead
([motors and their configurations](#motors-and-their-configurations)).

This is the doctest on [`hpr_io::ork::rocket`](../api/hpr_io/ork/component/fn.rocket.html), which CI runs.
Hand it the bytes of a `.ork`, and what comes back is a design and the warnings reading it raised:

```rust
let read = hpr_io::ork::read(xml)?;
let design = hpr_io::ork::rocket(&read.value.document);

// Anything the reader could not take at face value travels with the result. Nothing here did.
assert!(design.warnings.is_empty(), "{:?}", design.warnings);

// The ring's outer radius says `auto`, so the design carries the dimension, not a number...
use hpr_design::AutoDimension;
let ring = &design.value.stages[0].components[1].children[0];
assert!(ring.auto.contains(&AutoDimension::OuterRadius));

// ...and the layout works it out: the bore of the tube the ring sits in, 0.05 - 0.002.
let layout = design.value.layout()?;
let (_, placed) = layout.find("ring").expect("the ring");
let hpr_design::Part::CenteringRing(ring) = &placed.part else { panic!("a ring") };
assert!((ring.outer_radius_m - 0.048).abs() < 1e-12);
assert!(placed.own.mass_kg > 0.0);
```

**`design.warnings` is where a part that was left out is named.** It is a `Vec` of
[`Warning`](../api/hpr_io/ork/struct.Warning.html), each carrying where in the file it happened,
how much was lost (`Skipped`, `Dropped` or `Unusual`) and a sentence saying what was read and how —
for example "a fin set sits on a nose cone, and hpr attaches one only to a body tube; it was left
out". Read them: a design that opens cleanly raises none, and the 76 files of the reference library
raise 96 between them, every one of them explained on this page.

| `.ork` tag | read as | notes |
| --- | --- | --- |
| `innertube`, `tubecoupler`, `engineblock` | [`InnerTube`][p-inner] | may hold parts of its own |
| `centeringring` | [`CenteringRing`][p-ring] | bore and outer radius may both be automatic |
| `bulkhead` | [`CenteringRing`][p-ring] with no bore | |
| `trapezoidfinset`, `ellipticalfinset`, `freeformfinset` | [`FinSet`][p-fins] | with its tab, its cant (the angle the fins are turned to induce roll) and its section (the shape along the chord: square, rounded or airfoil) |
| `tubefinset` | [`TubeFinSet`][p-tubefins] | only when its radius is stated; neither in the corpus is, so hpr reads no tube fins from it ([#133][issue-133]) |
| `launchlug`, `railbutton` | [`LaunchLug`][p-lug], [`RailButton`][p-button] | a row of them is one part with a count and a spacing |
| `masscomponent` | [`MassComponent`][p-mass] | |
| `parachute`, `streamer`, `shockcord` | [`Parachute`][p-chute], [`Streamer`][p-streamer], [`ShockCord`][p-cord] | the packed shape, not the deployment |
| `podset`, `parallelstage` | — | kept whole ([what hpr keeps](#what-hpr-keeps-for-writing-the-file-back)); modelled in [M1.13](../decisions-and-roadmap.md#m1-13) |

**Angles in a `.ork` are degrees.** Nothing in the file says so, and every length beside them is in
metres, so it is easy to read one as radians — which would make `<angleoffset>180</angleoffset>`
more than twenty-eight turns instead of half of one.

**Observed:** of the **993** angles the corpus writes, **188** are not zero, and **178 of those are
larger than 2π** — more than a whole turn, which no component is written at. The values themselves
are 180, 90, 45, 30 and 120, carrying the float dust (`119.99999999999999`) of a conversion that
went through radians and came back. `cargo xtask ork` prints all three counts, and
`hpr_io::ork::tests::angles_are_degrees_not_radians` holds the reading.

A fin's **cant** is the same degrees: the corpus's two non-zero cants are 1.0 and −3.98, which as
radians would be 57° and 228°, a fin turned past half a turn from the airflow.

**Where a part sits** is one tag: `<axialoffset method="bottom">` on the newer name,
`<position type="bottom">` on the older, with the same five words — `top`, `middle`, `bottom`,
`after` and `absolute` — which are [`hpr_design::Position`][p-position] unchanged. **Observed:** one tube coupler
in the corpus has neither tag; it is read flush with its parent's forward end, with a warning.

**How far off the axis** a part sits is `radialposition` on some tags and `radiusoffset` on others.
[ADR-052][adr-052] read neither, for want of a source saying what the older name measures from.

That question need not be answered, because **the two never meet**: `radialposition` (542 elements)
is written on the parts *inside* a body, `radiusoffset` (106) on the parts *on* it and on the pods,
and no element carries both. They are two tags on different components, not two names for one. So
each is read where it is the only name its part has, and neither is ever read as the other.

<a id="which-way-round"></a>

**Which way round** a part sits is the angle, and there the two names do meet. `angleoffset` is the
newer one; the older is `rotation` on a fin set (95 elements carry both) and `radialdirection` on
everything else (26). On all 121, the two **agree on the number**, so which one is read cannot
change an angle. They differ on the *frame* — `relative` to the parent against `fixed` in the
rocket — but that is the same angle for every parent hpr builds, all of which sit on the rocket's
own axis. A pod set would not; a pod set is not read at all yet, so the question does not arise
until it is.

**Which *direction* the angle turns is assumed, and is not settled.** hpr measures a roll angle
from `x_B` toward `y_B`, right-handed about `+z_B`, which points at the nose
([frames](../physics/frames.md)). OpenRocket's [technical documentation][techdoc] (§3.1.4) puts its
own `x` axis along the centreline pointing **aft**, and says nothing about the other two. A
right-handed angle about an aft-pointing axis is a left-handed one about hpr's `+z_B` — so if that
is what OpenRocket means, every angle read here is **mirrored**: a mass object at 90° sits on the
other side of the airframe, and a canted fin set rolls the other way. Nothing in the reference
library can settle it, because a mirrored design is still a perfectly valid design; one
deliberately asymmetric design put through the [OpenRocket oracle](../decisions-and-roadmap.md#m2-2)
will. Until then hpr takes the number unchanged, and this is on the list of
[readings that are not settled](#not-settled).

**An override that covers the parts inside a component** is read from the mass flag. A `.ork` says
"this figure covers the components inside this one" once for the mass, once for the centre of
gravity and once for the drag; `hpr-design` says it once for the whole component, so the two cannot
always agree. Mass is the quantity the flag is written for — 95 of the 104 written in the corpus
are the mass flag — so the mass flag decides, and a centre-of-gravity flag that disagrees with it
raises a warning. Nothing in the corpus disagrees. This matters from this milestone on and not
before: until a component had parts inside it, "covers the parts inside" covered nothing.

**What a part takes from its parent** is resolved by [`Rocket::layout()`][p-layout], never here. A coupler's or
a ring's automatic outer radius is its parent's bore; a ring's automatic bore is the widest motor
tube beside it that overlaps it along the axis; a packed part fills the room left in the bore.
Automatic **outer** radii resolve in a pass before any ring's automatic **bore**, so a ring's answer
cannot depend on whether the tube inside it was written first —
[Loft lesson L60](../decisions-and-roadmap.md#l60), where Loft resolved as it walked and a bulkhead
inside a coupler stayed `NaN`.

### The surface finish

A surface finish is a **roughness height** `R_s`, the size of the bumps a painted or bare surface
leaves, which is what sets skin friction ([drag](../physics/aero.md)). OpenRocket writes one of
five words for it; what each is worth in micrometres is **not in the file format documentation**,
and these are the only numbers on this page that come from neither the documentation nor the
corpus:

| `<finish>` | OpenRocket calls it | roughness | where that number comes from | in the corpus |
| --- | --- | --- | --- | --- |
| `rough` | Rough | 500 µm | [the author][forum] | 8 |
| `unfinished` | Unfinished | 150 µm | [the author][forum] | 2 |
| `normal` | Regular paint | 60 µm | [technical documentation][techdoc] §6 **and** the [user guide's dialog][dialog] | 348 |
| `smooth` | Smooth paint | 20 µm | [the author][forum] | 88 |
| `polished` | Polished | 2 µm | [the author][forum] — **see the caveat below** | 14 |

Only the default is officially documented, and it twice over: the [technical
documentation][techdoc] section 6 says that in its test design "the 'regular paint' finish was
selected, which corresponds to an average surface roughness of 60 µm", and the [user guide's
body-tube dialog][dialog] reads "Component finish: Regular paint (2.36 mil)", which is 59.9 µm. The
[user guide][finishes] names all five and their order — "Rough, Unfinished, Regular paint, Smooth
paint, and Polished, each with a decreasing (CD) from rough to polished" — but gives **no**
numbers. The other four come from OpenRocket's author, in [The Rocketry Forum thread "Open Rocket
Finishes"][forum] (post #6, 22 August 2013): "Rough (500 µm) / Unfinished (150 µm) / Regular paint
(60 µm) / Smooth paint (20 µm) / Polished (2 µm)".

Each becomes a [`Finish::Custom`][finish-api] height — a roughness given as a number — rather than
one of hpr's named finishes, whose names ("raw wood", "dip-galvanized metal") mean other surfaces
that happen to share a height.

**`polished` is the one to doubt.** Its 2 µm rests on a 2013 forum post and on nothing else, and
the number sits oddly: in the [table the OpenRocket technical documentation itself
reprints][techdoc] (Table 3.2, from Hoerner), 2 µm is *aircraft-type sheet metal*, while "finished
and polished surface" is **0.5 µm**. So OpenRocket's "Polished" is not the row its name points at,
and a later version could reasonably have moved it. Rocketry-forum posts from 2023 onward list nine
finishes rather than five, which suggests the list has indeed changed, though neither the 23.09 nor
the 24.12 release notes mention it.

What it would cost: skin friction in the fully-rough branch goes as `R_s^0.2`
([drag](../physics/aero.md)), so 0.5 µm instead of 2 µm is a **1.32×** change in the skin-friction
coefficient of whatever part says `polished` — **14 components** in the corpus. No file in the
corpus writes any word but the five, so nothing here can settle it; the
[OpenRocket oracle](../decisions-and-roadmap.md#m2-2) can. A word hpr has no sourced roughness for
takes hpr's default and says so in a warning.

### What is left out, and why

A part hpr cannot give an honest shape is **left out with a `Skipped` warning** naming the part and
the reason, rather than guessed at. The design still opens and still lays out; what is missing is
named, never silent. Five parts in the whole corpus, over four rules — the first two catch the same
two fin sets:

| rule | in the corpus |
| --- | --- |
| an external part on anything but a body tube — a fin's root on a nose cone is not a straight line | 2 fin sets, on one design's nose |
| a freeform outline that does not end on the root, which would have to be closed along a body it never touches | the same 2 |
| a tube fin set whose radius OpenRocket sizes from the body, which hpr has no rule for ([#133][issue-133]) | 2 |
| a part whose automatic radius needs a bore its parent has not got | 1 coupler, in a nose cone |

2 + 2 + 1 = 5, because the freeform outlines that leave the root are the same two fin sets the
first row catches.

Four more things are read as the simpler part hpr models, each with a warning so that what is
missing from a mass is visible: a fin's **fillets** (5), a rail button's **screw head** (2), a
**cluster** of motor tubes read as the one tube it is written as (4), and a **row** of more than one
ring read as one.

**A tube of no wall thickness carries no mass** — among them couplers in two of OpenRocket's own
example designs. Reading those as solid would invent the mass — a solid coupler filling a 50 mm
airframe for 180 mm is a few hundred grams the design never had. Since
[M2.2b1](../decisions-and-roadmap.md#m2-2b1) it is the rule for every part, and it is not warned
of: OpenRocket 24.12 gives an inner tube, coupler, lug, nose cone, transition, body tube or shoulder
of no wall no mass either, measured on probe designs ([ADR-061][adr-061]), and a solid body
component is written `<thickness>filled</thickness>`. An inner tube, coupler or lug that writes no
thickness at all is still read as no wall, with a warning; OpenRocket gives it a wall of its own,
and no file in the library has one ([Mass properties](../physics/mass.md#what-a-ork-leaves-unsaid-and-overrides)).

### Checked against the answers OpenRocket cached

`auto 0.0125` is not just a flag: the number is what OpenRocket itself last worked out — in
OpenRocket 24.12, at least; older releases may differ, as the note below explains. That makes
an oracle for the resolution rules that needs no OpenRocket, and `cargo xtask ork` runs it over the
corpus — on every automatic dimension that caches a number and sits on a component the file gives
an `<id>`. On 2026-09-20, **67 of 71 agree** to a part in 10⁹, with 4 more cached but inside a pod
this milestone does not read. Of the 4 that disagree, the 2 body radii are settled in hpr's favour
[below](#openrocket-settles-it); the 2 packed radii follow the bore of that tube, so they are settled
with it by the bore rule, not measured (the oracle reads body radii only).

**What that number is, measured.** OpenRocket 24.12 writes the radius it resolved, not the number
it read: a tube that says `auto 0.04` and has nothing to take is saved again as `auto 0.025`. It
also ignores the number when it opens a file ([below](#when-an-automatic-radius-has-nothing-to-take)).
One older statement disagrees. The 2021 change that started writing the number,
[OpenRocket PR #998](https://github.com/openrocket/openrocket/pull/998), calls it the "manual
value", the last one typed in. So a file saved by some release between then and 24.12 may cache a
hand-typed number rather than an answer. Which releases wrote which is not settled. That is one
more reason hpr never reads the number as a radius.

It reads the number in one place only: as the wall of a `filled` tube whose radius is automatic but
reachable, which has no other number to be solid to, and says so in a warning. No file in the
corpus has one. (A tube whose radius takes OpenRocket's default is solid to that default instead,
[below](#when-an-automatic-radius-has-nothing-to-take).)

**What the oracle does not reach.** A cached answer only exists where OpenRocket wrote one, and it
never writes one for two of the tags that matter most here: across the whole corpus, `outerradius`
caches a number **0 times out of 131** and `innerradius` **0 out of 80**. So the 71 comparisons are
all `aftradius`, `foreradius`, `radius` and `packedradius` — and the two rules this milestone adds,
**an inner tube's automatic outer radius** and **a ring's automatic bore**, have *no oracle coverage
at all*. They rest on their unit tests and on the argument for them, until an OpenRocket oracle
reads those parts too: the one [below](#held-against-openrocket-itself) reads body radii only, and
[M2.2](../decisions-and-roadmap.md#m2-2) is where the rest belongs. `cargo xtask ork` prints the
per-tag denominators and names the tags nothing reaches, so the gap is in the report rather than
only here.

The four cached numbers that disagree (not the four inside a pod) are one body tube and the
parachute packed inside it (whose radius follows the tube's bore), in **OpenRocket's own "Dual
parachute deployment" example**, which the corpus holds twice — once inside the jar and once cached
beside it. Its spine is a nose cone and four body tubes; the third tube *states* a radius of
0.028321 m, every automatic radius caches 0.028321 m too, and hpr resolves them all to it — except
that the first tube caches 0.025 m.

<a id="openrocket-settles-it"></a>

**OpenRocket itself settles it, and agrees with hpr.** Run on that file
([below](#when-an-automatic-radius-has-nothing-to-take)), OpenRocket 24.12 first reads the first tube
as 0.025 m, its default. Once it works the design out again, as it does when saving, it reads
0.028321 m, and writes that. The first reading depends on an unrelated part, the coupler inside
the *second* tube, whose own radius is automatic: the tenth and eleventh rows of the table below
are the same small design without and with such a coupler, and only the one with it is first read
at the default. So the 0.025 m in the file is most likely a first reading that an earlier save wrote out: the probe
reads before and after a save, not twice without one, and nothing yet checks which radius
OpenRocket's own simulation uses after a plain open ([M2.2](../decisions-and-roadmap.md#m2-2) will).
hpr is held to the answer OpenRocket settles on.

<a id="held-against-openrocket-itself"></a>

**Held against OpenRocket itself.** `cargo xtask ork` also compares every body radius hpr resolves
with the one OpenRocket 24.12 settles on for the same file, read from the oracle's committed
results in `validation/fixtures/ork/openrocket-automatic-radius.json`, and prints the result on its
line "body radii against OpenRocket 24.12 run on the same file". It covers 18 of the 19 files
OpenRocket was run on: the 17 examples in its jar, and the parachute catalogue below (it refuses
the 19th, below). **67 of 67
agree** — body radii this time, a different 67 from the cached numbers above. Unlike the cached
answers, this reaches every body radius, fixed or automatic, including the Dual parachute tube.

### When an automatic radius has nothing to take

Sometimes a chain of automatic radii has no fixed radius anywhere along it, so the
[neighbour rule](../physics/design.md#automatic-dimensions) has nothing to work from. For
example, a nose cone's base follows the tube behind it, and that tube follows the nose cone. Two
designs in the reference library do this. hpr gives each such radius **OpenRocket's own default,
25 mm**, as a fixed radius, and raises a warning at its tag naming the radius. A document whose
`<rocket>` holds nothing at all is not a design, and is reported that way.

**If you see that warning,** the design has a radius its author never set. OpenRocket 24.12 shows a
tube there at 25 mm too, and a nose cone's base or a transition's end that looks at another
automatic radius at −1 m, which no shape can have. Neither is likely to be the rocket that was built. Set the radius in the design
(in OpenRocket, untick *Automatic* and type the diameter) and open it again.

**Why 25 mm.** OpenRocket's user guide doesn't say what happens here. Its issue tracker does:

- A maintainer: "OR returns the default radius"
  ([#1988](https://github.com/openrocket/openrocket/issues/1988#issuecomment-1397654629)).
- An open issue: a tube left with nothing to take "reverts to default diameter"
  ([#1992](https://github.com/openrocket/openrocket/issues/1992)).
- A user reports a nose cone that "may be retaining the default 1.969 in. base diameter"
  ([#871](https://github.com/openrocket/openrocket/issues/871)). That is 50.0 mm across, or 25 mm
  of radius, but it is a user's guess, so the number rests on the measurement below.

Since 2021, OpenRocket's dialogs grey out the checkbox where a radius would have nothing to take
([PR #998](https://github.com/openrocket/openrocket/pull/998)), though #1988 shows a later version
still leaving one ticked. So chains like these come mostly from older or hand-written files.

**Measured.** `validation/oracles/openrocket/automatic_radius.py` runs the OpenRocket 24.12 program
on fifteen small designs of its own and records the radius it gives each body component. It
records it twice: when the file is first opened, and once OpenRocket has worked the design out
again, as saving makes it do. The results are committed in
`validation/fixtures/ork/openrocket-automatic-radius.json`, and the test
`hpr_io::ork::tests::a_radius_with_nothing_to_take_is_openrockets_default` holds hpr to them.

In the table, radii are listed forward to aft. "30 to 20" is a transition's forward and aft
radius, and "\|" is a stage boundary. OpenRocket's column is the settled answer.

| design | what the file says | OpenRocket 24.12 | hpr |
|---|---|---|---|
| a tube | `auto` | 25 mm | 25 mm |
| a tube | `auto 0.04` | 25 mm | 25 mm |
| two tubes | `auto`, `auto 0.04` | 25, 25 mm | 25, 25 mm |
| a nose cone | `auto` | 25 mm | 25 mm |
| a nose cone | `auto 0.03` | 25 mm | 25 mm |
| a transition | `auto 0.03` to `auto 0.02` | 25 to 25 mm | 25 to 25 mm |
| a nose cone, a tube | `auto 0.03`, `auto 0.03` | **−1 m**, 25 mm | 25, 25 mm |
| a nose cone, a tube, a transition, a tube | `auto 0.033`, `auto`, `auto` to 22 mm, 22 mm | **−1 m**, 25 mm, **−1 m** to 22 mm, 22 mm | 25, 25, 25 to 22, 22 mm |
| a nose cone, a tube (the control) | `auto`, 30 mm | 30, 30 mm | 30, 30 mm |
| a nose cone, two tubes, a tube | `auto`, `auto`, `auto`, 30 mm | 30, 30, 30, 30 mm | 30, 30, 30, 30 mm |
| the same, a coupler of automatic radius in the third component | as above | 30, 30, 30, 30 mm (first read: 30, **25**, 30, 30) | 30, 30, 30, 30 mm |
| a tube, two tubes | 30 mm, `auto`, `auto` | 30, 30, 30 mm | 30, 30, 30 mm |
| a tube, a transition, a tube | 30 mm, 30 mm to `auto`, `auto` | 30, 30 to **−1 m**, 25 mm | 30, 30 to 25, 25 mm |
| a nose cone, a transition, a tube | `auto`, `auto` to 20 mm, 20 mm | **−1 m**, **−1 m** to 20, 20 mm | 25, 25 to 20, 20 mm |
| a nose cone, a tube \| a tube | `auto`, `auto` \| 30 mm | 30, 30 \| 30 mm | 30, 30 \| 30 mm |

What the table shows:

- **The number cached after `auto` is ignored.** OpenRocket gives the tube that caches 0.04 m
  25 mm, and hpr does the same. The cache is an answer OpenRocket once wrote, never an input.
- **A chain that reaches a fixed radius takes it**, in every case probed (up to two automatic radii
  in between) and across a stage boundary too, in both programs. The default is only for a chain with nothing
  fixed on it.
- **hpr departs from OpenRocket in one way, on purpose.** Where a nose cone's base or a
  transition's end looks at another automatic radius, OpenRocket gives it −1 m. No shape can have a
  negative radius, so hpr gives it 25 mm too, and the chain is one radius end to end. That is 6 of
  the 39 radii in the table.
- **OpenRocket's first reading can differ from its answer.** With a coupler of automatic radius in
  the third component, OpenRocket first reads the tube ahead of it at its default, then corrects it
  to 30 mm when it works the design out again. That is the Dual parachute example's cached 25 mm
  ([above](#checked-against-the-answers-openrocket-cached)).

**Worked example.** The eighth row is the chain in [Loft][loft]'s quirks fixture, on a small copy
the oracle script `automatic_radius.py` writes itself: a nose cone whose base is automatic (it
caches 0.033 m), an automatic tube, and a transition whose forward end is automatic and whose aft
end is fixed at 22 mm, then a 22 mm tube. Each automatic radius follows another automatic radius. A
transition's two ends never follow each other, so the fixed 22 mm stops at the transition and none
of the three ever reaches it. (The same is why, in the thirteenth row, a transition's fixed forward
end doesn't reach its automatic aft end.)

hpr gives all three 25 mm. The nose cone ends at 25 mm, the tube is 25 mm, and the transition
narrows from 25 mm to 22 mm. The cached 0.033 m is not used, and three warnings name the tags.

**A tube's wall is judged against the default.** A tube whose automatic radius takes the default
has its wall read again now there is a radius to read it against, by the rule a stated radius
gets: `filled`, or a wall at least as thick as 25 mm, is solid to 25 mm. The test
`a_tube_given_the_default_has_its_wall_judged_against_it` holds that.

**The three files this settles** (`cargo xtask ork` prints the counts):

| file | what it holds | what happens |
|---|---|---|
| [Debrief][debrief]'s `sample-design.ork` | a `<rocket>` with a name, a comment and nothing else, plus a stored simulation | holds no design; counted apart, not as a failure. Its stored simulation is read ([stored simulations](#what-openrocket-last-did-stored-simulations)) |
| the `openrocket-database` parachute catalogue | four tubes, every radius a bare `auto`, carrying the catalogue's parachutes | lays out, four tubes at 25 mm, just as OpenRocket 24.12 opens it |
| [Loft][loft]'s `demo-quirks.ork` | the worked example's chain, and a parallel stage placed directly under the rocket | lays out as in the worked example. **OpenRocket 24.12 will not open this file**: it refuses a parallel stage there, so its answers for this chain come from the oracle script's copy. hpr opens it and skips the parallel stage with a warning, as it does every parallel stage until [M3.1c](../decisions-and-roadmap.md#m3-1c) |

How it was decided, and the sources quoted in full, are in [ADR-054][adr-054].

### Measured on the reference library

`cargo xtask ork`, over the 76 readable files, on 2026-09-20:

| | |
|---|---|
| designs whose `Rocket` lays out | 75 of the 75 files that hold a design |
| documents that hold no design | 1, the 76th file |
| automatic radii given OpenRocket's default, 25 mm | 7, in 2 designs: 5 on body tubes, 1 on a nose cone, 1 on a transition |
| body radii against OpenRocket 24.12 run on the same file | 67 of 67 agree, over 18 of the 19 files it was run on |
| body components | 285 |
| parts on and inside them | 765 |
| by kind | 194 centering rings, 156 inner tubes, 135 parachutes, 107 fin sets, 84 mass components, 40 shock cords, 31 launch lugs, 16 rail buttons, 2 streamers |
| automatic dimensions marked for the layout to resolve | 320, plus the 7 above given the default: 327 in the files |
| parts left out, with a reason | 5 |
| parts that lay out weighing nothing | 14, every one explained (below) |
| warnings raised | 57: 16 dropped, 12 skipped, 29 unusual (below) |
| tags no milestone reads yet | 9 `podset`, 3 `parallelstage` |

**The 14 parts that weigh nothing** are worth checking, because a structural part with no mass is
silent by nature — the design lays out, the report is written, and the mass is simply missing. All
14 are accounted for: 7 inner tubes (couplers among them) and 4 launch lugs whose wall the file
states as zero, which OpenRocket gives no mass too ([ADR-061][adr-061]); 1 mass object the file
says weighs 0 kg; and 2 transitions the designs *override* to zero mass —
which is OpenRocket's ["base drag hack"](https://openrocket.readthedocs.io/en/latest/), a massless,
dragless transition added only to change the base geometry. `cargo xtask ork` counts them by kind,
so a new one would show up. Before
[M2.2b1](../decisions-and-roadmap.md#m2-2b1) there were 21: the 7 more (2 body tubes, 2 fin sets,
2 inner tubes and a nose cone) name no material, and now take OpenRocket's default.

**What the 57 warnings are.** Every one is a reading this page explains, and none of them means a
file is broken:

| kind | count | what raised it |
|---|---|---|
| `Unusual` | 20 | the single pre-1.9 subcomponent-override flag, read as setting all three |
| `Unusual` | 1 | a part with no axial offset |
| `Unusual` | 7 | an automatic radius with nothing along its chain to take, given OpenRocket's default ([above](#when-an-automatic-radius-has-nothing-to-take)) |
| `Unusual` | 1 | a `<rocket>` holding nothing, so the document holds no design |
| `Dropped` | 11 | a fin's fillets, a rail button's screw head, a motor cluster read as one tube |
| `Dropped` | 5 | a `packedradius` the file does not give, read as zero |
| `Skipped` | 7 | a tally of the pods and parallel stages, kept in `x-openrocket` and modelled in [M1.13](../decisions-and-roadmap.md#m1-13), one per design that has any |
| `Skipped` | 5 | the five parts left out above |

Every design lays out. The one document that doesn't is Debrief's demonstration file, which holds
no design at all: it is a stored simulation with a rocket's name on it. The two designs that needed
a radius the file doesn't give are
[above](#when-an-automatic-radius-has-nothing-to-take).

[adr-054]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-054-an-automatic-radius-with-nothing-to-take-is-openrockets-default-and-a-rocket-with-no-stage-or-component-holds-no-design-2026-09-20
[adr-053]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-053-the-parts-on-and-inside-a-ork-body-degrees-what-is-left-out-and-a-sourced-finish-2026-09-20
[techdoc]: https://openrocket.sourceforge.net/techdoc.pdf
[dialog]: https://openrocket.readthedocs.io/en/latest/_images/body_tube_config.png
[finishes]: https://openrocket.readthedocs.io/en/latest/user_guide/overrides_and_surface_finish.html
[forum]: https://www.rocketryforum.com/threads/open-rocket-finishes.57558/post-585716
[issue-133]: https://github.com/nrdptel/hpr-sim/issues/133
[finish-api]: https://nrdptel.github.io/hpr-sim/api/hpr_design/finish/enum.Finish.html
[p-inner]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.InnerTube.html
[p-ring]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.CenteringRing.html
[p-fins]: https://nrdptel.github.io/hpr-sim/api/hpr_design/fins/struct.FinSet.html
[p-tubefins]: https://nrdptel.github.io/hpr-sim/api/hpr_design/fins/struct.TubeFinSet.html
[p-lug]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.LaunchLug.html
[p-button]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.RailButton.html
[p-mass]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.MassComponent.html
[p-chute]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.Parachute.html
[p-streamer]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.Streamer.html
[p-cord]: https://nrdptel.github.io/hpr-sim/api/hpr_design/parts/struct.ShockCord.html
[p-position]: https://nrdptel.github.io/hpr-sim/api/hpr_design/tree/enum.Position.html
[p-layout]: https://nrdptel.github.io/hpr-sim/api/hpr_design/tree/struct.Rocket.html#method.layout

## Motors and their configurations

**In short.** hpr reads every motor a design names, in every configuration, with when it lights and
its ejection delay, and finds its thrust curve in the file itself or in hpr's bundled catalog. A
configuration becomes one the rocket can fly only when every motor in it has a curve and lights at
launch. Most designs in the reference library name motors the bundled catalog doesn't hold yet, so
**2 of their 174 configurations fly today**; the rest are read, kept, and say why not.

A **configuration** is one set of motors to fly the design with: OpenRocket calls it a *flight
configuration*, and a design can have several, one per motor choice. The file keeps it in two
places:

- `<rocket>` declares each one: a `configid`, a name, whether it is the one OpenRocket opens with
  (`default="true"`), and which stages fly.
- Each **motor mount** (a body tube or inner tube holding a motor) holds a `<motor configid="…">`
  per configuration: the manufacturer, the designation such as `H148R`, a `digest` (OpenRocket's
  fingerprint of the thrust curve's data), the case diameter and length, and the ejection delay. The mount also says when its motor lights, and may
  say it differently for each configuration.

So hpr collects each mount's motor into its configuration. A mount that names a configuration the
rocket never declares still gets one, with a warning ([L65](../decisions-and-roadmap.md#l65)).

### Where the thrust curve comes from

A `<motor>` names a motor; it does not describe one. hpr looks for its thrust curve in two places,
in this order:

1. **Inside the file.** From schema 1.11 (the `version` on the file's `<openrocket>` element),
   OpenRocket can save each motor's curve in the archive as `thrustcurves/<digest>.rse`, named by
   the `digest` the `<motor>` gives ([L57](../decisions-and-roadmap.md#l57)). That is exactly the
   curve the design was saved with. OpenRocket 24.12 still writes 1.10, so few files carry one yet:
   one file in the reference library does.
2. **hpr's bundled catalog**: [32 motors from ThrustCurve.org](../physics/motor.md). The
   manufacturer and the designation must both match, ignoring case, spaces and hyphens. Both are
   needed: an Estes `B4` is not a Quest `B4`.

OpenRocket's own order is the other way round: its motor database first, then the curve in the
file, because the database "may have more accurate or updated data" (its file specification). hpr's
catalog is far smaller than that database, and the curve in the file is the exact one, so hpr reads
the file first. So on a file that carries curves, hpr and OpenRocket can fly different curves for
the same motor.

hpr builds the motor from the curve file's header — its case size and its loaded and propellant
masses — as it does for any catalog motor, and does not use the mass or centre-of-gravity column
listed beside each thrust point. Where the design's case size and the curve's differ by more than a
millimetre, a warning says so: the design's size places the motor, and the curve's gives its mass. A motor found in neither place is kept with its reason. Nothing is
invented for it. A hybrid motor (solid fuel burned with a liquid or gas oxidiser) never gets a
curve: hpr flies commercial solid motors only.

To fly a motor neither place has, build the configuration yourself: read its `.eng` or `.rse` file
with `hpr_motor` ([Solid motors](../physics/motor.md)) and put it in an
`hpr_design::Configuration`.

### Delays and ignition

| the file says | it means | source |
|---|---|---|
| `<delay>6.0</delay>` | the ejection charge fires 6 s after burnout | OpenRocket's [technical documentation][techdoc], p. 8 |
| `<delay>0.0</delay>` | the charge fires at burnout | the same, p. 10: "zero-delay motors" |
| `<delay>none</delay>` | plugged: no ejection charge | the same, p. 8 ("P … stands for plugged"); OpenRocket [issue #2002](https://github.com/openrocket/openrocket/issues/2002) |
| `<ignitionevent>automatic</ignitionevent>` | the bottom stage lights at launch; a stage above lights at the ejection charge of the stage below | OpenRocket's [FAQ](https://wiki.openrocket.info/FAQ), "How do I create a staged rocket?" |
| `launch`, `burnout`, `ejectioncharge`, `never` | at launch; at the first burnout or ejection charge of the stage below; never | OpenRocket 24.12's labels, with every word measured by a committed probe ([below](#what-the-words-mean)). A word hpr does not know is kept as written |
| `<ignitiondelay>1.5</ignitiondelay>` | 1.5 s after that event | the [technical documentation][techdoc], section 4.2.6 |

A `0` means something different here than in a motor file. An `.eng` file has no word for plugged,
so hpr reads its `0` as "zero or plugged" ([ejection delay](../glossary.md#ejection-delay)). A
`.ork` has `none` for plugged, and OpenRocket flies a `0` as a charge at burnout: in its own
"Parallel booster staging" example, the E12-0's burnout and ejection charge are stored at the same
2.44 s. But until OpenRocket 23.09 put *plugged* in its delay list
([issue #2090](https://github.com/openrocket/openrocket/issues/2090)), few authors knew to type
`none`, and some used `0` to mean plugged, as OpenRocket's own examples did
([issue #2111](https://github.com/openrocket/openrocket/issues/2111)). So a `0` in an older design
may be meant as plugged; hpr reads it as the file says, as OpenRocket does. 23 motors in the
reference library have one.

A configuration's own `<ignitionconfiguration>` replaces the mount's event and delay one at a time:
whichever it leaves out, the mount's own value stands.

### Which configurations the rocket flies

hpr lights every motor in a configuration at launch, until staging and air starts arrive with
[M1.9](../decisions-and-roadmap.md#m1-9). So a configuration becomes one of the rocket's only when
all of these hold. Otherwise flying it would be wrong, for example lighting a sustainer on the pad.

- Every motor has a thrust curve, and a case diameter and length.
- Every motor lights at launch: `launch`, or `automatic` in the bottom stage (the booster, which
  OpenRocket lists last), with no delay.
- No motor sits in a part hpr doesn't read yet, such as a pod
  ([M3.1c4](../decisions-and-roadmap.md#m3-1c4)), and none sits in a
  [cluster](../glossary.md#cluster) of motor tubes, which hpr reads as one tube.
- No stage is switched off in the configuration's own stage list
  (`<stage number="1" active="false"/>`). OpenRocket leaves a switched-off stage out of the flight,
  and hpr flies every stage.
- The rocket and its motor mounts were read without a single warning: nothing left out (a pod, a
  parallel stage, a part hpr could not shape), nothing dropped or simplified (a cluster of tubes
  read as one, a flipped nose cone read pointing forward, a material that could not be read), and
  nothing assumed (a shape hpr does not know read as a cone).
  Otherwise hpr might fly a different rocket from the design, so no configuration of it is flown.
  One design in the library was held back only by its shoulders of no wall; since
  [M2.2b1](../decisions-and-roadmap.md#m2-2b1) reads them as OpenRocket does, it flies.
- No mount holds two motors for the configuration. Which one OpenRocket would fly is not known, so
  neither flies.
- The rocket has one stage. OpenRocket drops a booster when it separates; hpr would carry it to the
  ground, until separation is read ([M3.1c2](../decisions-and-roadmap.md#m3-1c2)) and flown
  ([M1.9](../decisions-and-roadmap.md#m1-9)).

Every other configuration is still read, whole, with the first reason it can't be flown. The
motors' reasons are checked first, each across every motor, and the two about the whole rocket
last.

This is the doctest on
[`hpr_io::ork::design`](../api/hpr_io/ork/fn.design.html), which CI runs. The Estes F15 has no
curve in the file, so it comes from the bundled catalog: 49.6 N·s over 3.45 s, as ThrustCurve.org
lists it. The rocket assembles with it. `assemble` takes the configuration's `configid`; its name
is for display, and may be one of OpenRocket's templates, such as `[{motors}]`.

```rust
let read = hpr_io::ork::read(xml)?;
let design = hpr_io::ork::design(&read.value).value;

// The F15 has no curve in this file, so it comes from the bundled catalog...
let motor = &design.motors.configurations[0].motors[0];
assert!(matches!(motor.curve, hpr_io::ork::Curve::Catalog { .. }));
assert_eq!(motor.delay, Some(hpr_motor::Delay::Seconds(4.0)));
let impulse_ns = motor.curve.motor().expect("a curve").curve().total_impulse_ns();
assert!((impulse_ns - 49.61).abs() < 0.01, "{impulse_ns}");

// ...and it ignites at launch, so the configuration is one the rocket flies.
let assembly = design.rocket.assemble("c1")?;
assert_eq!(assembly.motors[0].mount, "body");
```

### Motors in the reference library

`cargo xtask ork`, over the 75 designs, on 2026-09-21:

| quantity | count |
|---|---|
| motor configurations | 174, in 66 designs, over 79 motor mounts; none named only by a mount |
| motors read into their configurations | 206: 132 single-use, 65 reloads, 3 hybrids, and 6 with no type written, read like the rest |
| motors left out, in parts not read yet | 6: 4 in pod sets, 2 in parallel stages |
| thrust curve from the file itself | 4 |
| thrust curve from the bundled catalog | 2 |
| no curve | 200: 3 hybrids, and 197 in neither place |
| ejection delays, of the 206 | 128 in seconds, 23 at 0 s, 53 plugged (`none`), 2 not written |
| configurations the rocket flies | 2, in 2 designs, and both assemble |
| left out, by first reason | 166 a motor with no curve, 4 a motor in a part not read, 2 a motor lighting in flight; the one held back for an airframe not read exactly as written, a shoulder of no wall, flies since [M2.2b1](../decisions-and-roadmap.md#m2-2b1) |

The catalog is the limit, not the reader. When
[M5.1](../decisions-and-roadmap.md#m5-1) brings ThrustCurve.org's curves, many of the 197 may find
one; how many is not measured yet. How this was decided, with the sources in full, is in [ADR-055][adr-055].

[adr-055]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-055-m31c-split-and-the-motors-a-ork-flies-its-own-curve-first-and-only-what-lights-at-launch-2026-09-21

## When parachutes open and stages separate

**In short.** hpr reads when each parachute and streamer opens, with the drag coefficient it states,
and when each stage separates, in every configuration. It reads them; it does not fly them yet.
Turning them into hpr's own [recovery devices](../physics/recovery.md) waits for the flight that
uses them. Every device and stage in the reference library is read, apart from 2 parachutes inside
pods and the separations of 2 parallel stages, which hpr does not read yet
([M3.1c4](../decisions-and-roadmap.md#m3-1c4)).

A **deployment** is an event, a height for the event that needs one, and a delay after it. A
parachute or streamer states its own, and a configuration may change any of the three:

- `<deployevent>ejection</deployevent>`, `<deployaltitude>200.0</deployaltitude>` (metres) and
  `<deploydelay>0.0</deploydelay>` (seconds) are the device's own.
- `<deploymentconfiguration configid="…">` changes them for one configuration. Whichever of the
  three it leaves out, hpr keeps the device's own, as it does for a motor's ignition. That is hpr's
  reading: OpenRocket was not probed on a file that leaves one out.

A stage's **separation** is written the same way, with `<separationevent>`,
`<separationaltitude>`, `<separationdelay>` and `<separationconfiguration configid="…">`. The stage
that states it is the one that drops away: OpenRocket's labels (below) speak of the "current
stage" and the "upper stage".

### What the words mean

OpenRocket's documentation lists none of these words. The committed probe
`validation/oracles/openrocket/events.py` runs OpenRocket 24.12, sets every value of each event
through the program's public setters, saves the design, and records the word written and the label
OpenRocket shows. Its results are in `validation/fixtures/ork/openrocket-events.json`, and the test
`hpr_io::ork::tests::every_event_word_openrocket_writes_is_read` holds hpr's reader to every word.

| deploy word | OpenRocket's label |
|---|---|
| `launch` | "Launch (plus NN seconds)" |
| `ejection` | "First ejection charge of this stage" |
| `apogee` | "Apogee" |
| `altitude` | "Specific altitude during descent" |
| `lowerstageseparation` | "Lower stage separation" |
| `never` | "Never" |

| separation word | OpenRocket's label |
|---|---|
| `launch` | "Launch" |
| `ignition`, `burnout`, `ejection` | "Current stage motor ignition", "… burnout", "Current stage ejection charge" |
| `upperignition` | "Upper stage motor ignition" |
| `altitudeascending`, `apogee`, `altitudedescending` | "Specific altitude during ascent", "Apogee", "Specific altitude during descent" |
| `never` | "Never" |

The same probe measures three things the words do not say:

- **A deploy height is above the ground**, meaning the launch site: OpenRocket has no terrain. The
  probe flies in calm air with a fixed seed, so it writes the same numbers every run. On a pad
  1,000 m above sea level, a parachute set to `altitude` 30 m opened at 29.9 m above the ground,
  1,029.9 m above the sea. A height read above the sea would never have been reached.
- **Set above apogee, it did not open.** Set to 100 m, on a flight whose apogee was 51.7 m, the
  parachute never opened, and the flight reached the ground. That is one run of one design, not a
  rule OpenRocket states. hpr's own altitude trigger opens at apogee instead
  ([Recovery](../physics/recovery.md#triggers-lag-and-release)), so the two differ here, and the
  step that flies a `.ork`'s recovery will have to choose.
- **`<cd>auto</cd>`** is reported as 0.8 for a parachute, on the canopy's area: OpenRocket's
  [technical documentation][techdoc] gives 0.8 as the default (section 4.2.5), and the probe reads
  it back. For a streamer it is worked out from the strip's length and material, on the strip's
  area (appendix C, equations C.4 and C.5): 0.089, 0.060 and 0.050 for strips 0.5, 1.0 and 1.5 m
  long, in a material of 67 g/m², which the three values imply by equation C.5. The documentation puts that estimate's accuracy at about 20%, and one
  user's report ([issue #2031](https://github.com/openrocket/openrocket/issues/2031)) finds a
  2.5 by 44 in streamer's 0.06 far too small. hpr keeps the `auto` or the stated number as the file
  wrote it.

### Recovery in the reference library

`cargo xtask ork`, over the 75 designs, on 2026-09-21:

| quantity | count |
|---|---|
| parachutes and streamers read | 137: 135 parachutes, 2 streamers |
| left out, inside pod sets hpr does not read yet | 2 |
| drag coefficient | 77 `auto`, 60 stated |
| deploy event | 77 `ejection`, 35 `apogee`, 18 `altitude`, 6 `lowerstageseparation`, 1 `never` |
| devices a configuration changes | 9, with 17 changes in all |
| stages that state a separation | 18 of 93: 12 `ejection`, 4 `upperignition`, 2 `burnout`; 13 changes per configuration |
| separations left out, in parallel stages hpr does not read yet | 2 |

How this was decided is in [ADR-056][adr-056].

[adr-056]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-056-a-ork-designs-recovery-and-separation-read-as-written-with-openrockets-words-measured-2026-09-21

## What OpenRocket last did: stored simulations

**In short.** A `.ork` keeps the simulations OpenRocket last ran on the design, and hpr reads them
back: the launch conditions each was flown in, the ten summary figures, and each stage's time
series with its events. They are OpenRocket's answers, kept as it wrote them, for comparing against
later ([M2.2](../decisions-and-roadmap.md#m2-2) uses them as a second reference). The reference
library holds 178 of them, and every one is read.

A stored simulation has three parts:

- **`<conditions>`**: the launch rod, the wind, the launch site, the atmosphere and the time step.
- **The summary**: ten figures on `<flightdata>`, such as `maxaltitude` and `optimumdelay`.
- **The time series**: a `<databranch>` per stage, whose `types` name its columns (`Time`,
  `Altitude` and the rest: 58 in the files OpenRocket 24.12 writes, other versions differ) and
  whose `<datapoint>` rows give a value for each, beside the flight's `<event>`s (`launch`,
  `apogee`, `recoverydevicedeployment` and so on) and any `<warning>` OpenRocket stored.

### What the numbers mean

The file-format page gives units only for the multilevel wind (metres, m/s and radians), and its
own example writes a launch rod's direction as `90.0` and a wind's as `1.5707963267948966`. The committed probe `validation/oracles/openrocket/conditions.py`
runs OpenRocket 24.12, sets the conditions through its public setters, saves, loads them again and
flies them. Its results are in `validation/fixtures/ork/openrocket-conditions.json`, and the test
`hpr_io::ork::tests::wind_direction_is_not_rod_direction` holds hpr's reader to them.

| the file says | it means | hpr gives |
|---|---|---|
| `<launchrodangle>5.0</launchrodangle>` | the rod tilts 5 degrees from vertical | `rod_angle_rad`, 0.0873 |
| `<launchroddirection>45.0</launchroddirection>` | toward a compass bearing of 45 degrees, clockwise from north | `rod_direction_rad`, 0.785 |
| `<winddirection>0.5</winddirection>` | the wind blows **from** a bearing of 0.5 radians, about 29 degrees | `wind_from_rad`, 0.5 |
| `<windturbulence>0.1</windturbulence>` | turbulence intensity: the wind speed's standard deviation over its mean | `wind_turbulence`, 0.1 |
| `<atmosphere model="extendedisa">` with `<basetemperature>` and `<basepressure>` | the standard atmosphere from a temperature (K) and pressure (Pa) at the launch site | `Atmosphere::Extended` |

The probe also flies the example, in three ways that settle what the directions mean:

- **The rod's direction is a compass bearing.** In calm air, a rod tilted 10 degrees toward
  bearing 0 (north) lands the rocket 21.3 m north, and toward 90 (east), 21.3 m east, while the
  example's own wind setting stays at 90 degrees. OpenRocket's preferences page still calls the
  direction relative to the wind; the program does not treat it so.
- **The wind's direction is where it blows from.** From a vertical rod, in a steady 5 m/s wind
  from bearing 90 (east), the rocket lands 48.4 m west; from bearing 0, 48.4 m south.
- **`<launchintowind>` rewrites the rod.** With it true, OpenRocket overwrites the rod's direction
  with the wind's bearing, in degrees: a wind from 0.5 radians is written as a rod direction of
  28.648.

All of this is measured on OpenRocket 24.12. A file written by a much older version may have meant
the rod's direction otherwise, which is not measured.

The rod's direction and the wind's are different numbers in different units.
[Loft][loft] read the wind's direction from `launchroddirection`
([L64](../decisions-and-roadmap.md#l64)). hpr reads each from its own tag.

The probe saves a flight and compares one stored row with the same quantities as OpenRocket held
them, in eight columns: time, altitude, vertical velocity, two angles, latitude, air temperature
and air pressure. Those are SI, with the angles in radians and latitude in degrees; the other
columns and the summary are taken to follow. Values are rounded when stored: to three decimal
places (287.857 K, 0.218 rad), so a small quantity keeps few digits, and a large one to four
significant figures (100,796.6 Pa is stored as 100,800). `NaN` marks a quantity OpenRocket did not
compute at that step; hpr keeps it as `None`, so a design with stored results survives being saved
as JSON and read back.

This is from the test `hpr_io::ork::tests::stored_results_are_read_back`: a stored run reads back
through `design.simulations`, and a column comes out by its name.

```rust
let simulation = &design.value.simulations[0];
let results = simulation.results.as_ref().expect("results");
assert_eq!(results.max_altitude_m, Some(50.59));
let branch = &results.branches[0];
assert_eq!(branch.column("Altitude"), Some(vec![Some(0.0), Some(30.25)]));
```

### Stored simulations in the reference library

`cargo xtask ork`, over the 76 readable files, on 2026-09-21:

| quantity | count |
|---|---|
| stored simulations | 178, in 64 documents: 141 `uptodate`, 17 `external`, 11 `outdated`, 9 `notsimulated` |
| with launch conditions | 177; 129 state the wind's direction, and 135 launch into the wind |
| atmosphere | 172 `isa`, 2 `extendedisa`, 1 not written, and 2 with no model (an older file's own table), not read, with a warning |
| with a summary | 164 |
| with a time series | 144, over 178 stage branches and 101,955 rows |

How this was decided is in [ADR-057][adr-057].

[adr-057]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-057-a-ork-designs-stored-simulations-read-back-as-written-with-their-units-measured-2026-09-21

## What hpr keeps for writing the file back

**In short.** Some of what a `.ork` holds, hpr's design does not model: pods, parallel stages,
OpenRocket's 3D-view settings, a simulation's plug-ins, a part's colour, a material's group. hpr keeps
each whole, beside
the design, in an *extension* (a named slot for data another program wrote) called `x-openrocket`,
at a path that leads back to where it was, so that writing the file back out
([M3.2](../decisions-and-roadmap.md#m3-2)) can put it back. A design whose rocket is missing parts
this way says it is **reduced**; the flag is on the design, not on its rocket, so check it before
using the rocket on its own.

Four kinds of thing are kept:

- **Parts** hpr does not read: a pod set or a parallel stage, which hpr does not model until
  [M1.13](../decisions-and-roadmap.md#m1-13) ([L66](../decisions-and-roadmap.md#l66)), a part hpr
  cannot give an honest shape ([above](#what-is-left-out-and-why)), or a tag it has never seen.
- **Sections** of the document hpr does not read: `<photostudio>` (the 3D view), `<docprefs>` (the
  design's own materials), anything else beside `<rocket>` and `<simulations>`, and the parts of a
  stored simulation beyond its conditions and results, such as an `<extension>`.
- **Tags** no reader asks for, inside a part, stage or stored simulation hpr does read, or inside
  a tag it does read: a part's colour (`<appearance>`), a catalogue preset, a comment, a wind's
  standard deviation, or a tag hpr has never seen. hpr records every tag its readers ask for while
  it reads a file, so a tag is kept exactly when nothing asked for it.
- **Attributes** no reader asks for, on an element hpr does read: a material's `group`, an event's
  `id`, or the reference an angle or radius offset is measured from, which hpr does not read yet
  ([issue #145](https://github.com/nrdptel/hpr-sim/issues/145)).

Each is kept with its **path**, such as `openrocket/rocket/stage[0]/bodytube[1]/podset[0]`: the
podset that is the first part inside the second part of the first stage. A tag's last step starts
with `@`: `openrocket/rocket/stage[0]/nosecone[0]/@appearance[7]` is the eighth tag of that nose
cone, and a tag inside it adds another, such as `…/@wind[2]/@gusts[1]`. An attribute is kept with
the path of the element it was on, its name and its value. The function `hpr_io::ork::element_at`
follows a path back to the element.

The extension is written under its namespace when the design is saved as JSON, and reads back
unchanged. The test `hpr_io::ork::tests::unknown_content_round_trips_through_x_openrocket` checks
both, and that each kept element is the one at its path in the file:

```rust
let json = serde_json::to_string(&design.extensions).expect("JSON");
assert!(json.starts_with(r#"{"x-openrocket":"#), "{json}");
let back: Extensions = serde_json::from_str(&json).expect("read back");
assert_eq!(back, design.extensions);
```

**What is not kept.** The text of a second copy of a tag a reader takes once by name. A reader asks
for a tag by name and uses the first copy, so the second's own value is taken as read, though its
attributes and anything unread inside it are kept. 13 fin tabs in the library carry a second
`<tabposition>` this way. That text, and everything else, is still in the document itself, which
hpr keeps whole when it opens a file ([ADR-051][adr-051]); writing the file back
([M3.2](../decisions-and-roadmap.md#m3-2)) starts from both.

### Kept in the reference library

`cargo xtask ork`, over the 76 readable files, on 2026-09-21:

| quantity | count |
|---|---|
| parts kept | 17, in 10 reduced designs: 9 pod sets, 3 parallel stages, 2 freeform fin sets, 2 tube fin sets, 1 tube coupler |
| sections kept | 87: 42 `<photostudio>`, 36 `<docprefs>`, 9 simulation `<extension>`s |
| tags kept | 1,947, most often a part's `<appearance>` (274), `<radialdirection>` (166), `<instanceseparation>` (155), a wind's `<standarddeviation>` (129) and `<preset>` (126) |
| attributes kept | 3,132, most often an event's `id` (1,623), a material's `group` (552), an active stage's `number` (201) and a stored branch's optimum altitude and its time (170 each) |
| kept elements and attributes found again at their path | 5,183 of 5,183 (the survey fails if one is not) |

How this was decided is in [ADR-058][adr-058].

[adr-058]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-058-what-a-ork-holds-that-hpr-does-not-model-kept-whole-in-x-openrocket-2026-09-21

## What is not read yet

Pods and parallel stages are kept, not modelled: hpr's design has no pods until
[M1.13](../decisions-and-roadmap.md#m1-13) ([what hpr keeps](#what-hpr-keeps-for-writing-the-file-back)).
Writing a `.ork` back out as a design — rather than
as the document it was read from — is [M3.2](../decisions-and-roadmap.md#m3-2).

What keeping the whole document buys you is this: when hpr meets a part it does not model, it can
say so and carry the part's own XML along untouched in `x-openrocket`, rather than dropping it
silently the way Loft did with pods and parallel stages. The document itself is kept whole too, so
a writer ([M3.2](../decisions-and-roadmap.md#m3-2)) will have what it needs; what it chooses to do
with a part hpr does not understand is its decision, taken in the open.

[adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
