# RockSim `.rse` motor files

A `.rse` file is a motor database in XML, a text format of nested, named elements, from the RockSim
flight simulator. It holds one or more motors ("engines"). Each has its size, masses and delays
as attributes, and a table of thrust, mass and [centre of gravity](../glossary.md#centre-of-gravity-cg)
(CG) over time. [ThrustCurve.org](../glossary.md#thrustcurveorg) serves it beside
[RASP `.eng`](eng.md).

**To fly a motor from a `.rse` file**, see
[A motor from a file](../physics/motor.md#a-motor-from-a-file) on the Solid motors page. It reads a
`.eng` file, and says what changes for a `.rse` one.

This page is the reference for hpr's reader and writer. The spec is thin and disagrees with every
real file on element and attribute names, so a reader must follow the observed structure. Of the
823 RockSim files ThrustCurve.org held on 2026-09-17, 821 read and write back with every value
unchanged; the other two have a time that goes backwards
([Checked against real files](#checked-against-real-files)).

Code: `hpr_motor::rse` ([API reference](../api/hpr_motor/rse/index.html)), written for the
solid-motor milestone ([M1.3](../decisions-and-roadmap.md#m1-3)). The rules below are from the spec unless marked
**Observed** (seen in real files) or **Policy** (hpr's own choice).

## Sources

- **[P]** "RockSim & EngEdit – Engine File (.rse) Format Guide", 3 pp., PDF hosted by ThrustCurve.org,
  pinned as `rocksim-rse-spec` (sha256 `c47a04f4…`) in the
  [reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml),
  which records each source's address and checksum, and fetched to
  `refs/papers/rocksim-rse-spec.pdf`, a local folder that is never committed. Cited as [P p.N].
- **[S]** ThrustCurve.org, "Flight Simulators", RockSim section,
  <https://www.thrustcurve.org/info/simulators.html>, captured 2026-09-17 and pinned as
  `thrustcurve-simulators` (sha256 `5bb2bad7…`).
- **[X]** W3C, *XML 1.0* 5th ed., <https://www.w3.org/TR/xml/>, §2.11 (end of line), §3.3.3 (attributes).
- **Observed:** 715 engines and 15,149 points in the ThrustCurve manufacturer file sets (9 `.rse`
  sets) and 10 single-motor API downloads, fetched 2026-09-17 into a local cache,
  `refs/samples/formats/`, that is never committed.

## Structure

Spec: the minimal example is a bare `<engine …>` holding `<data>` with `<point t="…" f="…"/>`
children [P p.2]. Observed in 715 of 715 engines:

```text
<engine-database>
  <engine-list>
    <engine ATTRIBUTES>              repeated; one list holds every engine in the file
      <comments>free text</comments> optional (415 of 715), before <data>
      <data>
        <eng-data t="…" f="…" m="…" cg="…"/>   one per sample, in file order
      </data>
    </engine>
  </engine-list>
</engine-database>                   no XML declaration, DTD or namespace
```

## `<engine>` attributes

"Req" is the spec's Required column [P p.1]. Units are from [P p.1, p.3] unless noted.

| Attribute | Req | Unit | Meaning |
|-----------|-----|------|---------|
| `mfg` | yes | — | Manufacturer name |
| `code` | yes | — | "Unique engine identifier", the "primary lookup key" |
| `Type` | yes | — | Engine type [P p.1 spells it `type`; files always use `Type`]. Seen: `reloadable`, `single-use`, `hybrid` [S], `unspecified`, `Single-Use` |
| `dia` | yes | mm | Diameter |
| `len` | yes | mm | Length |
| `initWt` | yes | g (spec: "kg or g") | Initial (loaded) motor mass |
| `propWt` | yes | g (spec: "kg or g") | Propellant mass |
| `delays` | yes | s | "Comma-separated delay values" |
| `Itot` | yes | N·s | Total impulse |
| `avgThrust` | yes | N | Average thrust |
| `peakThrust` | yes | N | Peak thrust |
| `burn-time` | yes | s | Burn duration |
| `auto-calc-mass` | no | flag | `1`: RockSim computes mass as m(t) = initMass − (propMass / burnTime)·t [P p.2] |
| `auto-calc-cg` | no | flag | `1`: "CG is fixed at engine center" [P p.2] |
| `massFrac` | — | % | Not in spec. Observed: 100·m₀/initWt (703 of 710), where m₀ is the first point's `m` |
| `Isp` | — | s | [Specific impulse](../glossary.md#specific-impulse). Not in spec. Observed: Itot / (m₀[kg]·9.80665), within ±0.006 in 696 of 710 |
| `throatDia`, `exitDia` | — | mm? | Nozzle throat and exit diameters. Not in spec. Always `0.` |
| `tDiv tStep tFix FDiv FStep FFix mDiv mStep mFix cgDiv cgStep cgFix` | no | — | "Rendering attributes … control how graphs are drawn" [P p.2]. Always Div `10`, Step `-1.`, Fix `1` |

The spec contradicts itself: its summary names `initMass` and `propMass` [P p.2], while its
table and example use `initWt` and `propWt` [P p.1–2]. Only `initWt`/`propWt` occur in real files.

**Mass unit (verified).** Of 557 `.rse` engines whose `code` also appears in the `.eng` sets,
497 have `initWt` within 1% of 1000 × the `.eng` total mass. None match the kg value; the other 60
are different data (another diameter or mass). `massFrac` and `Isp` are consistent with grams.
The smallest `initWt` is a 1.2 g micro motor.

## `<eng-data>` point attributes

| Attr | Req | Unit | Meaning |
|------|-----|------|---------|
| `t` | yes | s | Time since ignition [P p.1] |
| `f` | yes | N | Thrust [P p.1] |
| `m` | no | g | "Mass … over time", needed if auto-calc is off [P p.1]. **Observed:** propellant mass *remaining*. m₀ = `propWt` in 696 of 715; last `m` = 0 in 715 of 715. In 698 of the 700 set engines with m₀ > 0, m(t) = m₀·(1 − I(t)/I_total), with I the trapezoidal impulse, to 1e-3 relative (median 1.3e-6). That is *not* the spec's linear auto-calc formula. |
| `cg` | no | mm | Motor CG over time [P p.1], [S]. **Observed:** constant = `len`/2 in 685 of 715. The datum (forward or aft end) is stated nowhere: **unverified**. |

With every observed file setting both auto-calc flags to `1`, RockSim may ignore `m` and `cg`
[P p.2]. **Policy:** hpr-sim preserves them for round trips but derives mass and CG itself.

## Thrust curve

- The spec example lists an explicit `t="0.0" f="0.0"` first point and ends with `f="0.0"` at
  `burn-time` [P p.2]. Unlike `.eng`, the origin is written out. The spec is silent on ordering,
  duplicates and a nonzero last point.
- **Observed:** the first point is (0, 0) in 715 of 715. The last `f` is 0 in 713; 2 put a nonzero
  point after the zero at the same `t`. 70 engines have equal consecutive `t`, and 1 has `t`
  decreasing. `burn-time` differs from the last `t` in 281 (rounded). `Itot` matches the trapezoidal
  integral within 0.1% in 712, and `avgThrust` = `Itot`/(last `t`) within 0.1% in 712.
  `peakThrust` ≠ max `f` in 9.

## Observed in real files (other)

- **Delays:** missing in 5. Always comma integers; never `P`. Descending lists are common
  (`15,12,10,8,6`). Checked against ThrustCurve search metadata, `1000` means plugged in 126 of 128
  single-token cases and `0` in 24 of 25. Lists can end in `1000` (`10,14,1000`).
- **Strings:** `mfg` has 20+ spellings, including leading or trailing spaces (` Aerotech`), commas
  (`Estes Industries, Inc.`) and underscores. `code` can hold spaces (`Micro Maxx II`), commas and
  parentheses.
- **Numbers:** 6 significant digits, like C `%g`, with a trailing `.` on integral values (`35.`,
  `0.`, `-0.`). One exponent (`1.51982e-05`); some padded (`0.00`).
- **XML text:** start tags wrap over several lines; two spaces after the element name. Attribute
  order is either RockSim's (490) or alphabetical (220); XML gives order no meaning. 6 of 19 files
  use CRLF, and many lack a final newline. `<comments>` spans lines in 197 engines; 2 use CDATA, one
  holding a raw `&`. There are no entity references, and everything is ASCII.
- **Bad values:** `propWt` 10× `m₀` (a typo), and `cg` 0.1 × len/2. Five engines (four hybrids)
  have every `m` and `cg` = 0, so m₀ = 0 cannot mean "no propellant".

## Reader policy (lenient, with diagnostics)

1. Use a conforming XML parser (`roxmltree`): it handles CDATA (raw-text sections), entities
   (escapes such as `&amp;`) and end-of-line normalization [X §2.11]. Refuse DTDs and external
   entities, declarations that can make a parser fetch other files (core crates do no I/O). The reader
   takes text and strips a BOM; decoding bytes is the caller's job, in `hpr-io`.
2. Take every `engine` element at any depth, so a bare `<engine>` root [P p.2] also works, but not
   one nested inside another engine. An engine with an error is skipped, with the error as a
   warning, when others in the file read. With repeated `<data>` or `<comments>`, the last is read,
   with a warning. `<comments>` keeps only text: XML comments and processing instructions inside
   are not part of it, and nested markup contributes its text. Every warning carries a kind:
   skipped (an engine lost), dropped (a value ignored) or unusual (read as it stands).
   Points are `eng-data` children of `data`; also accept `point` [P p.2].
3. Match attribute names exactly, then ASCII case-insensitively (`Type`/`type`); accept
   `initMass`/`propMass` as aliases [P p.2]. Ignore the rendering attributes silently; warn on and
   ignore other unknown attributes and elements.
4. Required: `code`, `dia`, `len`, `initWt`, `propWt`, and at least 2 points with `t` and `f`. A
   missing `mfg` reads as empty, with a warning. Everything else is `Option`; a missing `delays`
   means no delay information. If only some points carry `m` (or `cg`), treat it as absent for all
   of them, and warn. `auto-calc-*` flags read `1` or `0`; anything else is ignored with a
   warning.
5. Numbers: trim XML whitespace, then parse with Rust's f64 parser. Reject non-finite values:
   Rust accepts `inf`, `infinity` and `NaN`. Warn if `propWt` ≥ `initWt`.
   Keep file units (g, mm) in the file model, in fields named for them (`initial_mass_g`). Convert
   to SI only when building the physical motor: g → kg → g is not bit-exact (`4030.` comes back
   as `4030.0000000000005`; 14 of 1330 distinct mass and length values in the sets fail).
6. Keep strings verbatim, including the spaces in `mfg`; normalize names in the catalog layer.
   Delays: keep the raw string; interpret it as in [`.eng` files](eng.md) (`1000`/`100` → plugged, `0` ambiguous).
7. Point checks match `.eng`: error on negative or decreasing `t`; warn on a last `f` ≠ 0, on
   negative thrust, and on an `Itot` or `peakThrust` that disagrees with the curve by more than 1%.
   `burn-time` is not checked: files round it from the last time.

## Writer policy (strict, round-trip stable)

Round-trip stable: a file hpr writes reads back to exactly the values it was written from.

- Emit `<engine-database>`, `<engine-list>` and one `<engine>` per motor. Use 2-space indent, LF
  endings, a final newline, UTF-8, and no XML declaration (none was observed; whether RockSim
  accepts one is unverified).
- Write attributes in RockSim's order: `mfg code Type dia len initWt propWt delays auto-calc-mass
  auto-calc-cg avgThrust peakThrust throatDia exitDia Itot burn-time massFrac Isp tDiv tStep tFix
  FDiv FStep FFix mDiv mStep mFix cgDiv cgStep cgFix`. Write only the ones present in the model.
  Points are written as `t f m cg`, verbatim, with no origin added or removed.
- Numbers use Rust `{}` (Display): the shortest digits that round-trip exactly, no exponent, and
  `-0` for negative zero.
- Escape attribute values as `&amp; &lt; &gt; &quot;`, and tab, LF and CR as `&#9; &#10; &#13;`.
  A literal one would read back as a space [X §3.3.3]. Escape `<comments>` text as
  `&amp; &lt; &gt;` with CR as `&#13;`, without CDATA, and write it verbatim (no trimming).
- Flags are written `1` or `0`. The rendering attributes are not kept, so they are not written.
- Not implemented yet: converting from `.eng` would fill what `.eng` lacks the way the observed
  files do: a (0, 0) origin, `Itot` (trapezoid), `peakThrust`, `burn-time` = last `t`, `avgThrust`
  = Itot/burn-time, `m` by the impulse fraction above, `cg` = len/2, and both auto-calc flags
  `1`.
- Invariant (test it): whenever `write(parse(x))` succeeds, `parse(write(parse(x))) == parse(x)`,
  with f64 compared by bits and strings compared exactly.

## Checked against real files

On 2026-09-17, 821 of the 823 RockSim files in ThrustCurve.org's solid-motor survey read, and
write-parse-write reproduces every value bit for bit. The other two have a time that goes
backwards and are rejected with its line. In the manufacturer sets, 704 engines read; one engine
with a backwards time is skipped with a warning. The files are cached under `refs/samples/` and
never committed.

