# RASP `.eng` motor files

Code: `hpr_motor::eng` (M1.3). A plain-text thrust curve with a one-line header. The rules below
are from the spec unless marked **Observed** or **Policy**.

## Sources

- **[R]** ThrustCurve.org, "RASP File Format", <https://www.thrustcurve.org/info/raspformat.html>,
  captured 2026-09-17 and pinned as `thrustcurve-rasp-format` (sha256 `69573e9f…`). Sections are
  cited as [R Header], [R Data],
  [R Problems]. The page names the RASP C source as "the ultimate authority"; its license is not
  stated, so it was not consulted.
- **Observed:** 734 entries in the ThrustCurve manufacturer file sets (12 `.eng` sets) and 13
  single-motor API downloads, fetched 2026-09-17 into `refs/samples/formats/` (never committed).
  Counts below are over those entries.

## Grammar

```text
file    := entry+
entry   := (blank | comment)* header point+ (blank | comment)*
comment := ';' text                    whole line; [R Header], [R Problems]
header  := name dia len delays prop total mfg    seven fields "separated by spaces" [R Header]
point   := time thrust                 "usually preceded by a few spaces" [R Data]
```

- Blank and `;` lines are ignored before the header. After the last point an entry "may end or
  contain comments and blank lines, but nothing else" [R Header], [R Data].
- "All seven must be present for the entry to be read successfully" [R Header].
- Points start "immediately after the header line" [R Data].
- A file may hold several entries; "make sure that each entry is separated by at least one comment
  line". A lone `;` after the data is the customary separator [R Data], [R Problems].

## Header fields [R Header]

| # | Field | Unit | Meaning |
|---|-------|------|---------|
| 1 | name | — | Common name: "just the impulse class and average thrust" (`F32`) |
| 2 | diameter | mm | Casing diameter |
| 3 | length | mm | Casing length |
| 4 | delays | s | Available delays "separated by dashes"; `0` = ejection charge, no delay; `P` = plugged, no ejection charge |
| 5 | propellant mass | kg | "Weight of all consumables" (the propellant, for a solid) |
| 6 | total mass | kg | Motor "loaded and ready for flight" |
| 7 | manufacturer | — | Abbreviation, per the NAR combined motor list |

## Thrust curve [R Data], [R Problems]

- Each point is a time (s) and a thrust (N), as "floating-point numbers".
- An implicit first point at (0, 0) "is assumed and should not be specified explicitly". An
  explicit (0, 0) is called "a common mistake".
- "The final point must have a thrust of zero and it indicates the motor's burn time." A zero
  thrust anywhere else is rejected by ThrustCurve. (ThrustCurve's *metadata* burn time uses the
  NFPA 1125 5% rule instead; see Loft lesson L39.)
- Points "must be in order of time". ThrustCurve rejects a point "before the previous point" and a
  first point at negative time. Equal times are not addressed.
- RASP allowed at most 32 points, including the final zero; modern tools don't enforce this.

## Where the spec is silent

Number syntax (exponents, sign; its own example uses `.0377`). Whether tabs or several spaces
separate fields. Line endings, encoding and BOM. Delay lists that mix numbers with `P`, or use other
separators. Equal consecutive times. Inline comments. An entry with no comment separator before the
next header.

## Observed in real files

- **Whitespace:** tabs separate data fields in 4 of 12 sets. 52 headers use several spaces or
  column alignment; one header is indented. Trailing spaces are common. 8 of 25 files use CRLF;
  13 of 13 single downloads lack a final newline. There is no BOM, no non-ASCII byte, no inline
  `;`, no blank or comment line inside the data, and no indented comment.
- **Entries:** always separated by a comment; there are 470 lone `;` lines. A reader that stops
  at the first header loses the rest of the file (Loft L36).
- **Header:** never more than 7 fields; spaces in a manufacturer name become `_`
  (`Contrail_Rockets`). Manufacturer spellings vary (`AT`, `A`, `Aerotech`, `AERO`, `AT-RMS`,
  `AT/RCS`; `CTI`, `Ces`, `CSR`, `Pro38`). The name is often the full designation, not class plus
  thrust (`1266-J760-WT-19A`, `I216-CL(I)`, `O25,000-VM-P`, `1/2A3T`, `C6-0`).
- **Numbers:** leading dot (`.0377`, 21 headers), trailing dot (`2415.`), leading zeros
  (`068.8604`), padded decimals (`0.000`), binary float noise (`0.060700000000000004`, 72
  headers), up to 15 significant digits. Diameters and lengths can be fractional (`114.3`). No
  exponents or signs were seen.
- **Delays:** 574 dash lists, 143 `P`, 4 comma lists (`5,8,11`), 8 lists ending in `P`
  (`6-10-14-P`), 1 lowercase `p`, and 4 malformed (`4-7-10,`, `-`, `1-3--4-6-7-9-10`). 58 lists
  are descending (`14-12-10-8-6`). 27 contain `100` or `1000`. Checked against ThrustCurve search
  metadata, `100`/`1000` mean plugged in 14 of 14 cases, and `0` means plugged in 120 of 149, against
  the spec's "no delay" (Loft L37).
- **Curve:** 32 entries have an explicit first point at t = 0 with nonzero thrust. Loft's bundle
  also had an explicit (0, 0). 4 entries don't end at zero thrust. 4 have equal consecutive
  times (a vertical drop to zero, or rounded times). 69 have more than 32 points. None have
  decreasing time, negative thrust or an interior zero.

## Reader policy (lenient, with diagnostics)

1. The reader takes text and strips a UTF-8 BOM. Decoding bytes (and any Windows-1252 fallback)
   is the caller's job, in `hpr-io`. Lines split on LF, CRLF or a bare CR and are trimmed.
2. A line whose first non-blank character is `;` is a comment. Fields split on runs of spaces
   and tabs.
3. State machine per entry. Before the header, skip blanks and collect comments. The header is the
   first other line. In the data, a 2-field numeric line is a point; skip blank lines; a comment
   ends the data, and an entry that ends with no points is an error. A line with 7 or more fields
   starts a new entry, with a warning about the missing separator. Anything else is an error, with
   its line number.
4. The header needs 7 or more fields. With more than 7, join fields 7 onward with single spaces
   as the manufacturer, and warn. Dia and len must be finite and > 0, and the masses finite and
   ≥ 0; warn if prop ≥ total. Keep file units (mm, kg) in the file model, in fields named for them
   (`diameter_mm`). Convert to SI only when building the physical motor: mm → m → mm is not
   bit-exact (`502.1 / 1000 * 1000 != 502.1`; with `* 1e-3` and `* 1e3`, 30 of 290 distinct
   header lengths fail).
5. Numbers: Rust `str::parse::<f64>` accepts `.5`, `5.`, `068.8`, `+1` and `1e3`, **but also
   `inf`, `infinity` and `NaN`**. Reject every non-finite value.
6. Points are kept verbatim; no implicit origin is inserted into the file model. Errors: no points,
   negative time, time decreasing. Warnings, on the header's line: last thrust ≠ 0, negative
   thrust, and delay pieces the delay reader drops or flags. Equal consecutive times are accepted
   (a step in the physical curve). The RASP 32-point limit is not enforced.
7. Delays: keep the raw token verbatim. The derived view splits on `-` or `,`, drops empty pieces
   (with a warning), maps `P`/`p` → plugged and `100`/`1000` → plugged, reads `0` as its own
   "zero or plugged" setting with a warning (spec: no delay; files: usually plugged), and reads
   other pieces as seconds. A `0` never becomes an ejection at burnout without a decision. Physics
   should prefer catalog metadata for delays.
8. An entry with an error is skipped, with the error as a warning, when other entries in the file
   read; the reader resumes at the next comment line. With no entry read, the first error returns.
   Every warning carries a kind: skipped (an entry lost), dropped (a value ignored) or unusual
   (read as it stands).
9. Comments: store the text after `;` verbatim, but drop comments that are empty after trimming.
   Comments between two entries belong to the next entry; comments after the last entry are file
   trailer comments.

Conversion to a physical curve (`EngEntry::thrust_curve`, not part of the file model): prepend
(0, 0) when the first time is > 0. A leading (0, F) stays a step at ignition. Negative thrust is
an error there.

## Writer policy (strict, round-trip stable)

- Each entry: its comments as `;text`, then the header with single spaces, then one `   t F` line
  per point, then a lone `;`. Finally the trailer comments. LF endings, a final newline, UTF-8.
- The name must be one token that doesn't start with `;` (the line would read as a comment), and
  the delays one token. The manufacturer may hold spaces if its words are separated by single
  spaces, since the reader joins extra fields that way. Comment text must be non-empty, on one
  line, and without trailing whitespace (the reader would trim it). Otherwise return an error,
  never a silent substitution.
- Numbers use Rust `{}` (Display). It prints the shortest digits that parse back to the same bits,
  never uses an exponent (older readers may not accept one), and writes `-0.0` as `-0`.
- Points are written as stored: no origin is added or removed. The writer rejects negative or
  decreasing times, the same rules as the reader.
- Invariant (test it): whenever `write(parse(x))` succeeds, `parse(write(parse(x))) == parse(x)`,
  with f64 compared by bits. Dropping empty comments is what keeps the writer's `;` separator
  stable. A lenient read the writer cannot represent (a manufacturer containing spaces) is a
  write error, not a changed value.
- Leading and trailing whitespace inside comment text: the reader trims only trailing whitespace,
  and the writer never adds any, so `; text` round-trips as ` text`.

## Checked against real files

On 2026-09-17, all 889 RASP files in ThrustCurve.org's solid-motor survey
(`docs/research/thrustcurve-data.md`) and the 721 entries of its manufacturer sets read, and
write-parse-write reproduces every value bit for bit. The files are cached under `refs/samples/`
and never committed; the committed test covers the bundled curves.
