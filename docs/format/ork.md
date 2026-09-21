# OpenRocket `.ork` design files

A `.ork` file is an OpenRocket design: the tree of parts a rocket is built from, the materials they
are made of, the motors flown in it, and the results of the simulations OpenRocket last ran. It is
the format hobby designs are most often shared in, so reading it is how a design gets into hpr
without being typed again.

**What works today:** hpr opens a `.ork` file, whichever of its three containers it is in, reads
its design document into a tree that keeps everything the file said, and builds a rocket out of
that tree — the stages and body components stacked in them, and the tubes, rings, fins, lugs,
buttons and recovery gear on and inside each of those, with their shapes, materials, surface
finishes, positions and overrides. That is from Rust; there is no command-line tool yet. **What it
does not yet read is motors, recovery settings, pods and parallel stages**
([M3.1c](../decisions-and-roadmap.md#m3-1c)), so a design that opens still cannot be flown.

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

**Two more pairs look the same and are not**, which is why hpr does not yet read either name of
them. The newer name of each carries a `method` attribute — the frame — that the older name never
carries:

| newer | older | elements with both | agree on text | differ on frame |
|---|---|---|---|---|
| `angleoffset` | `radialdirection` | 26 | 26 | **26** |
| `radiusoffset` | `radialposition` | 0 | — | — |

`angleoffset` and `radialdirection` agree on the number every time and differ on the frame every
time. `radiusoffset` (on 106 elements) and `radialposition` (on 542) are never written together at
all, so nothing about them has been measured. Reading one as the other would move a component
without saying so, so what the older name's frame is belongs to the milestone that places
components, [M3.1b2](../decisions-and-roadmap.md#m3-1b2), with a source.

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
| `Skipped` | a whole part was left out | an **attachment** entry that could not be decompressed, or one that would pass the unpacking limit; a damaged *design* entry is an error, not a warning |
| `Dropped` | a value was ignored | a comment or processing instruction; an XML namespace; a tag whose text is not the number, count or flag it should be; two names for one value that disagree |
| `Unusual` | read as it stands | a schema version past 1.11; no `creator` attribute; a design entry not called `rocket.ork`; the single pre-1.9 subcomponent-override flag |

**Observed:** reading the corpus's containers and documents raises **no warnings at all** — every
file that opens is ordinary. So every row of the table above is exercised by a test rather than by
a file anyone shipped.

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
| warnings raised | 0 |
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

**Two readings this page is not sure of**, both said out loud as warnings and both for the
OpenRocket oracle ([M2.2](../decisions-and-roadmap.md#m2-2)) to settle:

| what the file says | how it is read | why |
| --- | --- | --- |
| a shoulder of zero wall thickness | solid | a wall of nothing has no mass and no geometry; 12 designs in the reference corpus have one |
| no `shapeclipped` on a transition | clipped | it is the shape that reaches both radii; only 1 of the 21 transitions in the corpus states it |

**Measured on the reference library** (`cargo xtask ork`, 76 readable files): 73 designs' spines lay
out, over 93 stages and 285 body components — 188 body tubes, 74 nose cones, 23 transitions — with
81 automatic radii marked (41 outer, 19 base, 14 fore, 7 aft). The 3 that do not lay out all depend
on parts that are not read yet: one design's stages are parallel stages, and two have no fixed
radius anywhere on the spine to resolve an automatic one against. The counts are what may be
published; the per-file detail stays in the gitignored `corpus-out/`.


## The parts on and inside the body

Everything that is not the spine hangs off it, and hpr reads it into the same
[`hpr_design::Rocket`][api]: the **tubes** inside a body component (inner tubes, couplers, engine
blocks), the **rings** that centre them (centering rings, bulkheads), the **fins, tube fins, launch
lugs and rail buttons** on the outside, and the **mass objects, parachutes, streamers and shock
cords** packed in the bore. Code: `hpr_io::ork::attached`
([API reference](../api/hpr_io/ork/attached/index.html)), decided in [ADR-053][adr-053].

| `.ork` tag | read as | notes |
| --- | --- | --- |
| `innertube`, `tubecoupler`, `engineblock` | `InnerTube` | may hold parts of its own |
| `centeringring` | `CenteringRing` | bore and outer radius may both be automatic |
| `bulkhead` | `CenteringRing` with no bore | |
| `trapezoidfinset`, `ellipticalfinset`, `freeformfinset` | `FinSet` | with its tab, cant and section |
| `tubefinset` | `TubeFinSet` | |
| `launchlug`, `railbutton` | `LaunchLug`, `RailButton` | a row of them is one part with a count and a spacing |
| `masscomponent` | `MassComponent` | |
| `parachute`, `streamer`, `shockcord` | `Parachute`, `Streamer`, `ShockCord` | the packed shape, not the deployment |
| `podset`, `parallelstage` | — | a spine of their own: [M3.1c](../decisions-and-roadmap.md#m3-1c) |

**Angles in a `.ork` are degrees.** Nothing in the file says so, and every length beside them is in
metres, so it is easy to read one as radians and turn a fin more than fourteen times round instead
of half a turn. **Observed:** 86 of the angles in the corpus are larger than 2π — more than a whole
turn — and the values themselves are 180, 90, 45, 30 and 120, carrying the float dust
(`119.99999999999999`) of a conversion that went through radians and came back.
`hpr_io::ork::tests::angles_are_degrees_not_radians` holds it.

**Where a part sits** is one tag: `<axialoffset method="bottom">` on the newer name,
`<position type="bottom">` on the older, with the same five words — `top`, `middle`, `bottom`,
`after` and `absolute` — which are `hpr_design::Position` unchanged. **Observed:** one tube coupler
in the corpus has neither tag; it is read flush with its parent's forward end, with a warning.

**How far off the axis** is two tags that look like the two names of one and are not.
[ADR-052][adr-052] left both unread for want of a source saying what the older name measures from.
It turns out the question need not be answered: the corpus shows the two never meet.
`radialposition` (542 elements) is written on the parts *inside* a body, `radiusoffset` (106) on the
parts *on* it, and **no element carries both**. So each is read where it is the only name its part
has, and neither is ever read as the other. The **angle** is different again: `angleoffset` and the
older `rotation` (on a fin set) or `radialdirection` (on everything else) agree on the number on all
121 elements that carry both, so which is read cannot change an angle, and the frames they differ on
— `relative` to the parent against `fixed` in the rocket — are the same angle for every parent hpr
builds, all of which sit on the rocket's own axis. A pod set does not, and a pod set is not read.

**What a part takes from its parent** is resolved by `Rocket::layout()`, never here. A coupler's or
a ring's automatic outer radius is its parent's bore; a ring's automatic bore is the widest motor
tube beside it that overlaps it along the axis; a packed part fills the room left in the bore.
Automatic **outer** radii resolve in a pass before any ring's automatic **bore**, so a ring's answer
cannot depend on whether the tube inside it was written first —
[Loft lesson L60](../decisions-and-roadmap.md#l60), where Loft resolved as it walked and a bulkhead
inside a coupler stayed `NaN`.

### The surface finish

OpenRocket writes one of five words. What each is worth is **not in the file format
documentation**, and the numbers below are the one thing on this page that comes from neither the
documentation nor the corpus:

| `<finish>` | OpenRocket calls it | roughness | in the corpus |
| --- | --- | --- | --- |
| `rough` | Rough | 500 µm | 8 |
| `unfinished` | Unfinished | 150 µm | 2 |
| `normal` | Regular paint | 60 µm | 348 |
| `smooth` | Smooth paint | 20 µm | 88 |
| `polished` | Polished | 2 µm | 14 |

The default is confirmed twice over. The [OpenRocket technical documentation][techdoc] section 6
says that in its test design "the 'regular paint' finish was selected, which corresponds to an
average surface roughness of 60 µm", and the [user guide's body-tube dialog][dialog] reads
"Component finish: Regular paint (2.36 mil)", which is 59.9 µm. The [user guide][finishes] names
all five and their order — "Rough, Unfinished, Regular paint, Smooth paint, and Polished, each with
a decreasing (CD) from rough to polished" — but gives no numbers; those come from OpenRocket's
author, in [The Rocketry Forum thread "Open Rocket Finishes"][forum] (post #6, 22 August 2013):
"Rough (500 µm) / Unfinished (150 µm) / Regular paint (60 µm) / Smooth paint (20 µm) / Polished
(2 µm)". Each becomes a `Finish::Custom` height rather than one of hpr's
[named finishes](../physics/aero.md), whose names are other surfaces entirely.

**One of these is not settled.** OpenRocket 23.09 added four more finishes, and in the newer list
the 2 µm row carries a different label, so `polished` written by a recent OpenRocket may mean
0.5 µm rather than 2 µm. No file in the corpus writes any word but the five, so nothing here can
settle it; the [OpenRocket oracle](../decisions-and-roadmap.md#m2-2) can. A word hpr has no sourced
roughness for takes hpr's default and says so.

### What is left out, and why

A part hpr cannot give an honest shape is **left out with its reason** rather than guessed at. The
design still opens and still lays out; what is missing is named, never silent. Five parts in the
whole corpus:

| rule | in the corpus |
| --- | --- |
| an external part on anything but a body tube — a fin's root on a nose cone is not a straight line | 2 fin sets, on one design's nose |
| a freeform outline that does not end on the root, which would have to be closed along a body it never touches | the same 2 |
| a tube fin set whose radius OpenRocket sizes from the body, which hpr has no rule for ([#133][issue-133]) | 2 |
| a part whose automatic radius needs a bore its parent has not got | 1 coupler, in a nose cone |

Four more things are read as the simpler part hpr models, each with a warning so that what is
missing from a mass is visible: a fin's **fillets** (5), a rail button's **screw head** (2), a
**cluster** of motor tubes read as the one tube it is written as (4), and a **row** of more than one
ring read as one.

**A tube of no wall thickness carries no mass** — 12 elements, among them two of OpenRocket's own
example designs. That is the opposite of the rule the spine gives a body component and a shoulder,
and deliberately: a body component has a `filled` spelling in the file, so a zero there is
ambiguous, while an inner tube has none and OpenRocket's own geometry makes a tube's bore its outer
radius less its wall. Reading those as solid would invent the mass instead — a solid coupler
filling a 50 mm airframe for 180 mm is a few hundred grams the design never had.

### Checked against the answers OpenRocket cached

`auto 0.0125` is not just a flag: the number is what OpenRocket itself last worked out. That makes
an oracle for the resolution rules that needs no OpenRocket, and `cargo xtask ork` runs it over the
corpus — on every automatic dimension that caches a number and sits on a component the file gives
an `<id>`. On 2026-09-20, **67 of 71 agree** to a part in 10⁹.

The four that do not are one body tube and the parachute packed inside it (whose radius follows the
tube's bore), in a single design the corpus holds twice. There the cache contradicts the file's own
other caches: the nose cone ahead of that tube caches 0.028321 m for the radius of the very tube
that caches 0.025 m, and 0.025 m is OpenRocket's default body-tube radius — the number a tube
starts at, not one worked out from anything. hpr resolves that tube to 0.028321 m, which is what
every other cached radius in the design says. This is [ADR-052][adr-052]'s warning about stale
caches, made visible rather than argued.

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
| warnings raised | 29 dropped, 12 skipped, 47 unusual |
| tags no milestone reads yet | 9 `podset`, 3 `parallelstage` |

The three designs that do not lay out are [M3.1b4](../decisions-and-roadmap.md#m3-1b4)'s work: one
document holds no `<rocket>` with any components in it at all — it is a synthesized demonstration
file carrying a simulation and nothing else — and two have a chain of automatic radii with no fixed
radius anywhere to resolve against. None of the three is a design a part could have completed.

[adr-053]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-053-the-parts-on-and-inside-a-ork-body-degrees-what-is-left-out-and-a-sourced-finish-2026-09-20
[techdoc]: https://openrocket.sourceforge.net/techdoc.pdf
[dialog]: https://openrocket.readthedocs.io/en/latest/_images/body_tube_config.png
[finishes]: https://openrocket.readthedocs.io/en/latest/user_guide/overrides_and_surface_finish.html
[forum]: https://www.rocketryforum.com/threads/open-rocket-finishes.57558/post-585716
[issue-133]: https://github.com/nrdptel/hpr-sim/issues/133

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
