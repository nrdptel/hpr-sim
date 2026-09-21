# OpenRocket `.ork` design files

A `.ork` file is an OpenRocket design: the tree of parts a rocket is built from, the materials they
are made of, the motors flown in it, and the results of the simulations OpenRocket last ran. It is
the format hobby designs are most often shared in, so reading it is how a design gets into hpr
without being typed again.

**What works today:** hpr opens a `.ork` file, whichever of its three containers it is in, reads
its design document into a tree that keeps everything the file said, and builds a rocket out of
that tree — the stages and body components stacked in them, and the tubes, rings, fins, lugs,
buttons and recovery gear on and inside each of those, with their shapes, materials, surface
finishes, positions and overrides. That is from Rust; there is no command-line tool yet.

**How far to trust it.** **Motors, recovery settings, pods and parallel stages are not read**
([M3.1c](../decisions-and-roadmap.md#m3-1c)), so a design that opens still cannot be flown. A part
hpr cannot give an honest shape — fins on a nose cone, tube fins OpenRocket sizes from the body —
is **left out**, each one named in a warning rather than guessed at
([what is left out, and why](#what-is-left-out-and-why)). And 3 of the 76 designs in the reference
library still produce no rocket at all
([M3.1b4](../decisions-and-roadmap.md#m3-1b4)).

## Opening a file today

This is the doctest on `hpr_io::ork`, which CI runs:

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
out from the neighbouring components, and 0.025 m is what it last worked out". A bare `auto` —
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
| `Unusual` | read as it stands | a schema version past 1.11; no `creator` attribute; a design entry not called `rocket.ork`; the single pre-1.9 subcomponent-override flag; a tube of no wall; a surface finish or an axial-offset method this reader has no rule for |

**Observed:** reading the corpus's containers and documents raises **no warnings at all** — every
file that opens is ordinary. Building a *rocket* from those documents raises 88: 29 dropped, 12
skipped and 47 unusual, over 76 files. Every kind of warning the container and document readers
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

**What you can check yourself.** The corpus is not public, so these counts are not reproducible on
a fresh clone: `cargo xtask ork` needs `cargo xtask refs fetch` first, and stops with
"no .ork files found" without it. What *is* reproducible anywhere is everything the tests cover —
`cargo test -p hpr-io` — and the same command on any `.ork` files you have, with
`cargo xtask ork --dir <path>`.

[api]: https://nrdptel.github.io/hpr-sim/api/hpr_design/tree/struct.Rocket.html
[adr-051]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20
[adr-052]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-052-what-a-ork-value-means-automatic-dimensions-two-names-for-one-tag-and-overrides-2026-09-20
[loft]: https://github.com/nrdptel/fusionspace-loft

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
([M2.2](../decisions-and-roadmap.md#m2-2)) to settle. The first two are the spine's; the rest come
from the parts:

| what the file says | how it is read | why it is in doubt |
| --- | --- | --- |
| a shoulder of zero wall thickness | solid | a wall of nothing has no mass and no geometry; 12 designs in the reference corpus have one |
| no `shapeclipped` on a transition | clipped | it is the shape that reaches both radii; only 1 of the 21 transitions in the corpus states it |
| an angle, **which way it turns** | the same way hpr's own frames turn | hpr measures a roll angle right-handed about an axis pointing at the **nose**; OpenRocket's technical documentation puts its own `x` axis along the centreline pointing **aft** and leaves the rest unstated. If it means what that implies, every angle read here is mirrored — see [below](#which-way-round) |
| a `polished` finish | 2 µm | the number is the author's, from 2013; a newer OpenRocket may have moved it ([below](#the-surface-finish)) |
| a tube of zero wall thickness | no mass | OpenRocket's own geometry says a tube's bore is its outer radius less its wall, so this follows — but the spine reads the same zero on a *body component* as solid, and only the oracle can say whether OpenRocket agrees with either |

**Measured on the reference library** (`cargo xtask ork`, 76 readable files): 73 designs' spines lay
out, over 93 stages and 285 body components — 188 body tubes, 74 nose cones, 23 transitions — with
81 automatic radii marked (41 outer, 19 base, 14 fore, 7 aft). The counts are what may be
published; the per-file detail stays in the gitignored `corpus-out/`.

*(When this was written, the 3 designs that do not lay out were thought to be waiting on parts that
were not read yet. Reading those parts, in
[M3.1b3](../decisions-and-roadmap.md#m3-1b3), showed otherwise: none of the three could have been
completed by a part. [What they actually need](#measured-on-the-reference-library) is below.)*


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
raise 88 between them, every one of them explained on this page.

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
| `podset`, `parallelstage` | — | a spine of their own: [M3.1c](../decisions-and-roadmap.md#m3-1c) |

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

**A tube of no wall thickness carries no mass** — 12 elements, among them two of OpenRocket's own
example designs. That is the opposite of the rule the spine gives a body component and a shoulder,
and deliberately: a body component can be written `<thickness>filled</thickness>`, so a zero there
is ambiguous, while an inner tube has no such spelling and OpenRocket's own geometry makes a tube's bore its outer
radius less its wall. Reading those as solid would invent the mass instead — a solid coupler
filling a 50 mm airframe for 180 mm is a few hundred grams the design never had.

### Checked against the answers OpenRocket cached

`auto 0.0125` is not just a flag: the number is what OpenRocket itself last worked out. That makes
an oracle for the resolution rules that needs no OpenRocket, and `cargo xtask ork` runs it over the
corpus — on every automatic dimension that caches a number and sits on a component the file gives
an `<id>`. On 2026-09-20, **67 of 71 agree** to a part in 10⁹, with 4 more cached but inside a pod
this milestone does not read.

**What the oracle does not reach.** A cached answer only exists where OpenRocket wrote one, and it
never writes one for two of the tags that matter most here: across the whole corpus, `outerradius`
caches a number **0 times out of 131** and `innerradius` **0 out of 80**. So the 71 comparisons are
all `aftradius`, `foreradius`, `radius` and `packedradius` — and the two rules this milestone adds,
**an inner tube's automatic outer radius** and **a ring's automatic bore**, have *no oracle
coverage at all*. They rest on their unit tests and on the argument for them, until
[M2.2](../decisions-and-roadmap.md#m2-2) can run OpenRocket itself. `cargo xtask ork` prints the
per-tag denominators and names the tags nothing reaches, so the gap is in the report rather than
only here.

The four that do not are one body tube and the parachute packed inside it (whose radius follows the
tube's bore), in **OpenRocket's own "Dual parachute deployment" example**, which the corpus holds
twice — once inside the jar and once cached beside it — so anyone with OpenRocket can check this.
**That file's caches contradict each other.** Its spine is a nose cone and four body tubes; the third tube *states* a radius of
0.028321 m, and every automatic radius on the spine caches 0.028321 m too — except the first tube,
which caches 0.025 m. But the nose cone's own cached base radius is 0.028321 m, and a nose cone's
automatic base radius is the radius of the component behind it, which is that first tube. So the
file says that tube is both 0.025 m and 0.028321 m. hpr resolves it to 0.028321 m, agreeing with
the design's other three cached radii and its one stated radius against the single odd one.

That is an argument from the file, not a proof: it says the cache is inconsistent, not which half
is stale. Which one OpenRocket would compute today is for the
[OpenRocket oracle](../decisions-and-roadmap.md#m2-2) to settle, and it is the reason
[ADR-052][adr-052] treats a cached number as an answer that may have gone stale rather than as an
input.

### Measured on the reference library

`cargo xtask ork`, over the 76 readable files, on 2026-09-20:

| | |
|---|---|
| designs whose `Rocket` lays out | 73 of 76 |
| body components | 285 |
| parts on and inside them | 765 |
| by kind | 194 centering rings, 156 inner tubes, 135 parachutes, 107 fin sets, 84 mass components, 40 shock cords, 31 launch lugs, 16 rail buttons, 2 streamers |
| automatic dimensions marked | 327 |
| parts left out, with a reason | 5 |
| parts that lay out weighing nothing | 21, every one explained (below) |
| warnings raised | 88: 29 dropped, 12 skipped, 47 unusual (below) |
| tags no milestone reads yet | 9 `podset`, 3 `parallelstage` |

**The 21 parts that weigh nothing** are worth checking, because a structural part with no mass is
silent by nature — the design lays out, the report is written, and the mass is simply missing. All
21 are accounted for: 6 parts in one hand-written fixture that gives no material at all, 9 inner
tubes and 4 launch lugs whose wall the file states as zero (above), 1 mass object the file says
weighs 0 kg, and 1 transition the design *overrides* to zero mass — which is
OpenRocket's ["base drag hack"](https://openrocket.readthedocs.io/en/latest/), a massless, dragless
transition added only to change the base geometry. `cargo xtask ork` counts them by kind, so a new
one would show up.

**What the 88 warnings are.** Every one is a reading this page explains, and none of them means a
file is broken:

| kind | count | what raised it |
|---|---|---|
| `Unusual` | 20 | the single pre-1.9 subcomponent-override flag, read as setting all three |
| `Unusual` | 13 | a shoulder of no wall thickness, read as solid |
| `Unusual` | 12 | a tube of no wall thickness, carrying no mass |
| `Unusual` | 2 | a body component with no wall thickness at all, and a part with no axial offset |
| `Dropped` | 13 | a part with no material, so it weighs nothing |
| `Dropped` | 11 | a fin's fillets, a rail button's screw head, a motor cluster read as one tube |
| `Dropped` | 5 | a `packedradius` the file does not give, read as zero |
| `Skipped` | 7 | a tally of the pods and parallel stages left for [M3.1c](../decisions-and-roadmap.md#m3-1c), one per design that has any |
| `Skipped` | 5 | the five parts left out above |

The three designs that do not lay out are [M3.1b4](../decisions-and-roadmap.md#m3-1b4)'s work: one
document holds no `<rocket>` with any components in it at all — it is a synthesized demonstration
file carrying a simulation and nothing else — and two have a chain of automatic radii with no fixed
radius anywhere to resolve against. None of the three could have been fixed by reading more parts.

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

## What is not read yet

Motor configurations and the embedded `.rse` curves, recovery settings (when a parachute opens, at
what delay, at what altitude — the chute's *shape* is read, its deployment is not), pods and
parallel stages, stored launch conditions and simulation results
([M3.1c](../decisions-and-roadmap.md#m3-1c)). Three designs of the 76 still do not lay out
([M3.1b4](../decisions-and-roadmap.md#m3-1b4)). Writing a `.ork` back out as a design — rather than
as the document it was read from — is [M3.2](../decisions-and-roadmap.md#m3-2).

What keeping the whole document buys you is this: when
[M3.1b](../decisions-and-roadmap.md#m3-1b) meets a part hpr does not model, it
can say so and carry the part's own XML along untouched, rather than dropping it silently the way
Loft did with pods and parallel stages. Nothing is lost between opening a file and writing it back;
what a later step chooses to do with a part it does not understand is that step's decision, taken
in the open.
