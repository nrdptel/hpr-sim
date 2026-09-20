# OpenRocket `.ork` design files

A `.ork` file is an OpenRocket design: the tree of parts a rocket is built from, the materials they
are made of, the motors flown in it, and the results of the simulations OpenRocket last ran. It is
the format most hobby rocketeers already have their rockets in, so reading it is how a design gets
into hpr without being typed again.

**What works today:** hpr opens a `.ork` file, whichever of its three containers it is in, and
reads its design document into a tree that keeps everything the file said, with the schema version
and the program that wrote it. **It does not yet build a rocket from that tree** — no components,
no materials, no motors. That is the next increment
([M3.1b](../decisions-and-roadmap.md#m3-1b)), and until it lands there is nothing here to fly.

Code: `hpr_io::ork` ([API reference](../api/hpr_io/ork/index.html)), written for
[M3.1a](../decisions-and-roadmap.md#m3-1a). Rules below are from the format documentation unless
marked **Observed** (seen in real files) or **Policy** (hpr's own choice).

## Sources

- **[F]** OpenRocket file format documentation,
  <https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html>, and the
  `fileformat.txt` shipped with the program. There is **no XSD**: nothing machine-checkable
  describes a `.ork`, and every OpenRocket release has added tags.
- **Observed:** 78 `.ork` files — the OpenRocket example designs inside the pinned
  `OpenRocket-24.12.jar`, the `openrocket-database` parts library, and the private design corpus.
  They are cached under `refs/` and never committed; the counts below are over those files and are
  reproduced by `cargo xtask ork`.
- OpenRocket's own Java source is **not** consulted: it is GPL, and hpr is MIT OR Apache-2.0
  ([ADR-051](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20),
  clean-room rule).

## The three containers

The first bytes say which one a file is in ([F]; Loft lesson
[L56][l56], containers sniffed by magic bytes):

| container | first bytes | what it holds |
|---|---|---|
| zip | `50 4b 03 04` (`PK␃␄`) | an entry called `rocket.ork` — the design as XML — beside anything else the design carries |
| gzip | `1f 8b` | the design document, compressed on its own. What older OpenRocket versions wrote |
| XML | `<`, after an optional byte-order mark and blank lines | the design document itself |

**Observed:** of the 78 files, 73 are zip and 3 are plain XML. No gzip `.ork` survives in the
reference library, so that path is held by a test rather than by a real file.

**Policy.** Inside a zip, the design is the entry named `rocket.ork`. If there is none, the first
entry whose name ends in `.ork` or `.xml` is read as the design and a warning says so. Every other
entry is kept byte for byte as an *attachment*: `thrustcurves/*.rse` motor curves (schema 1.11),
`preview.png`, lookup tables. **Observed:** 38 `.png`, 11 `.jpg`, 3 `.gif` and 3 `.rse`
attachments across the corpus. Nothing reads them yet; they are kept so that
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
- Text an element holds *beside* child elements. **Observed:** OpenRocket writes this. A
  simulation's `<warning>` prints its own message after its fields, in 48 elements of 19 files.
- The blank text that only lays the file out — the indentation between child elements — is **not**
  kept, so that writing the document out again is free to lay it out afresh.
- An XML comment or processing instruction is dropped, and counted as a warning. No `.ork` tag
  carries meaning in one.

Writing the document back out gives canonical XML: two-space indentation, `<tag/>` for an empty
element, and character references for the characters that would otherwise change when read again
(a carriage return in text; a tab, newline or return in an attribute). **Reading a document,
writing it, and reading it again gives the same document** — that is what "keeps everything" means
here, and it is checked two ways: by a property test over generated trees, and over every real file
in the corpus.

## Warnings, not failures

A design written by an older OpenRocket, or by another program, should still open. Everything that
departs from [F] but leaves the file readable is a warning that travels with the result
(`hpr_io::ork::Warning`), not an error:

| kind | means | raised for |
|---|---|---|
| `Skipped` | a whole part was left out | an archive entry that could not be decompressed |
| `Dropped` | a value was ignored | a comment or processing instruction |
| `Unusual` | read as it stands | a schema version past 1.11; no `creator` attribute; a design entry not called `rocket.ork` |

**Observed:** the corpus raises **no warnings at all** — every file that opens is ordinary.

Only these stop a read: bytes that are none of the three containers, a damaged zip or gzip, a zip
with no entry that could be the design, a design that is not UTF-8 or not well-formed XML, a root
that is not `<openrocket>`, an unreadable version, and a document that nests more than 64 elements
deep.

### Why there is a depth limit

Reading descends the tree, and so does the XML parser underneath. A file written to nest deeply
enough exhausts the stack, which is a crash rather than an error — and [L56] asks for an error.
Measured on a debug test build with a 2 MiB stack, `roxmltree` survives 120 levels of nesting and
dies somewhere before 130. So hpr counts the nesting **before** the text reaches the parser, with a
scan that skips comments, CDATA and processing instructions and tracks quotes, and refuses anything
past 64 levels. A `.ork` design nests about ten deep; the deepest in the corpus reaches 11.

[l56]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[L56]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md

## Checked against real files

`cargo xtask ork` reads every `.ork` in the reference library and in the OpenRocket jar's example
set, writes them back out, reads them again, and reports counts. The private corpus stays private:
the per-file detail goes to a gitignored `corpus-out/`, and only counts are published. On
2026-09-20:

| | |
|---|---|
| files read | 78 |
| opened | 76 |
| unchanged by a write and a read | 76 |
| warnings | 0 |
| not well-formed XML | 2 |

The two that do not open are hand-written fixtures for Loft's browser tests, and neither is XML: a
`<databranch>` is closed with `</flightdata>`. Python's `expat` refuses them at the same line, so
this is the files' fault and not the reader's. They are listed by name in the survey, which fails
if either one ever behaves differently.

## What is not read yet

Everything above the document: components, shapes, materials, finishes, overrides and automatic
dimensions ([M3.1b](../decisions-and-roadmap.md#m3-1b)); motor configurations, the embedded `.rse`
curves, recovery devices, stages, pods, stored launch conditions and simulation results
([M3.1c](../decisions-and-roadmap.md#m3-1c)). Writing a `.ork` back out as a design — rather than
as the document it was read from — is [M3.2](../decisions-and-roadmap.md#m3-2).
