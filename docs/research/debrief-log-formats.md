# Ten logger families, and how their files are read (M7.1)

**What this covers.** The format half of Debrief, the owner's own flight-log analyzer, folded into
this project (ADR-046): which loggers write what, how each file is told apart from the others, and
the traps that only show up on real files. **What it is for:** whoever builds M7.1, so that the
byte layouts and the quirks do not have to be rediscovered from flights. **How far to trust it:**
read from `refs/fusionspace-debrief` at commit `9e72db8`, file and line cited; the accuracy figures
are that project's own, measured against a private corpus of 61 logs across ten families, and
nothing here has been re-derived in Rust. The readings taken off a parsed flight are
[the readings note](debrief-flight-readings.md).

## How a file finds its parser

Each parser answers `detect(input) -> 0..1` and the highest score wins, ties going to the earlier
entry in the list (`lib/parsers/index.ts:32-55`). Below 0.6 nothing is claimed and a generic
column-mapping path takes over. Two decisions worth keeping:

- **Detection reads bytes and text both** (`lib/parsers/types.ts:8-23`). A binary flight record —
  an AltOS `.eeprom`, an RRC3 `.rff` — cannot be recognised from a decoded string at all, and its
  text view is mojibake.
- **A refusal is a result.** A parser may throw a *guidance* error that reaches the user as a
  sentence (`types.ts:51-56`): this is the high-rate half of a pair, this is a design file and not
  a flight, this is a raw card dump and here is what to export it with. Any other failure falls
  through to column mapping. The alternative, tested and rejected, is telling someone their flight
  log is not a flight log, which sends them after the wrong problem
  (`lib/parsers/rawDownload.ts:1-12`).

Text decoding has to handle a UTF-8 BOM, UTF-16LE and UTF-16BE by BOM, **and** BOM-less UTF-16 by
NUL density (≥20% NULs, 8:1 even-to-odd), because one vendor's text export and Excel's "Unicode
Text" are both UTF-16 (`lib/encoding.ts:27-61`). Delimited text tries comma, tab, semicolon and
pipe, scoring each by how consistent the column count is over the first 50 lines; when semicolon
wins, cells like `1,5` are decimal commas and are rewritten (`lib/csv.ts:11-47, 108-118`).

## The families

| family | file | told apart by | units in the file |
|---|---|---|---|
| Altus Metrum AltOS | CSV | header has `state_name`, `height`, `pressure` | SI |
| Altus Metrum AltOS | `.eeprom` | JSON head with `log_format`, then hex bytes | raw counts |
| Blue Raven (Featherweight) | app CSV, serial | `flight_time` plus an inertial or quaternion column; `LOG_LOW` | feet, °F, atm |
| Featherweight Raven | FIP CSV | contains `bILBA` and `time@` | atm, ft/s, g, °F |
| Featherweight GPS | tracker and ground-station CSV | `unixtime`+`lat`+`lon`+`#sats`; or `tracker lat`+`gs lat` | feet, usually |
| PerfectFlite | `.pf2`, CSV | extension, or a vendor word plus data-shaped rows | feet, ft/s, °F |
| Missile Works RRC3 | mDACS text | UTF-16, tab or semicolon | feet **or** metres |
| Missile Works RRC3 | `.rff` | .NET serialisation header | tenths of a millibar |
| Eggtimer | CSV | `t`/`alt` plus `vraw`+`vfilt`, or `veloc` plus event columns | feet **or** metres |
| Entacore AIM | CSV | `pressure msl` and `pressure agl` | Pa, g, °C |
| Mercury / AltimeterCloud | CSV | `acceleration_total(mg)`, `tilt`, `time(ms)` | mg, ms, centi-°C |
| spreadsheet | `.xlsx` | ZIP magic plus extension | whatever the sheet says |
| OpenRocket | `.ork` | ZIP holding `rocket.ork` | SI (mostly inferred) |

**Units are frequently not in the file.** Eggtimer and the RRC3 write an identical header whether
the device is set to feet or metres; reading feet when it wrote metres is 3.3× wrong in the
direction that flatters the flyer. Debrief reads the vendor default, says so in a note, and — for
the RRC3 — resolves it where the file also carries pressure, by testing the stated altitude against
the pressure it recorded.

## The two binary formats, which are the expensive ones

**AltOS `.eeprom`** (`lib/parsers/altosEeprom.ts`) is a JSON header followed by the flash contents
as ASCII hex. Records are fixed size, little-endian, with a type byte and a 16-bit tick at a 100 Hz
clock: 8 bytes for TeleMetrum v1 (`log_format` 1), 32 bytes for the Mega family (10, 15, 16, 19,
21, 22). **Any other format number is refused rather than guessed**, because a wrong record size
does not fail loudly — it produces a plausible flight out of misaligned bytes (`:291-298`).
Pressure comes from the MS5607's own datasheet compensation, including the second-order branch
below 20 °C, with every division floored rather than truncated toward zero. Four traps:

- **Ticks are 16 bits and wrap every 655.36 s**, and records legitimately arrive out of order at
  the boundary. Each tick is placed on the turn nearest the previous one; without that, one flight
  gained a spurious 655.36 s and reported 975 seconds (`:143-156`).
- **Erased flash is `0xFF`** and must be skipped, or the clock is dragged to 65535 (`:163-165`).
- **A calibration sentinel of 2147483647** means "this board has none"; a plain finite check lets
  it through and yields pressures in the billions (`:417-419`).
- **The decoded pressures are checked against the pressure the file states for the pad**, within
  2%, which is what makes an unmeasured format number safe to attempt at all; on the corpus the two
  agree to 4 Pa (`:463-469`).

Accuracy claimed: all 6,820 MS5607 pressures bit-identical to the vendor's own export; TeleMetrum
v1 none bit-identical but worst disagreement 0.0035 Pa over 2,206 samples (`:11-15`). The cold
branch and the MS5611 variant are exercised by no corpus flight and are held only against a second
reading of the same datasheet page — which, as the file says, catches a transcription slip and not
a misreading, since both readings are the same person's.

**Missile Works `.rff`** (`lib/parsers/missileworksRff.ts`) is a .NET `BinaryFormatter` stream
holding a `List<Int16>`. The array is found by scanning for the primitive-array record and its
matching member reference, and **only the list's `size` slots are real** — the rest is the array's
spare capacity, and reading it appends a run of zero-pressure samples (`:91-95`). Each 16-bit word
below `0x4000` is a barometer sample in tenths of a millibar; at or above it, an auxiliary word,
written in pairs once a second. The threshold is physics rather than a fitted constant: 0x4000 in
those units is 1,638 mbar, half again the highest pressure recorded at sea level. It is a
**threshold and not a bit test**, because a word with bit 15 set and bit 14 clear would slip
through a bit test and read back as 3,277 mbar (`:32-39`). There are no timestamps, so the 20 Hz
clock is an assumption — and it is tested: one auxiliary pair per second against 20 samples per
second, within 10% plus 2, or the file is refused (`:189-200`). What the auxiliary pair *means* is
deliberately not decoded, because the calibration is not in the file.

## Traps that generalise

- **A second altitude column is a second instrument, not a duplicate.** AltOS writes `altitude`
  twice; the later one is the GPS receiver's and is found by position after `longitude`, because
  the name collides (`altusmetrum.ts:169-181`).
- **Acceleration conventions differ and the file rarely says.** AltOS writes acceleration net of
  gravity, which reads about zero on the pad; taken as specific force it is a full g low, including
  in the thrust-to-weight quoted against the 5:1 rail rule. It was proved from the data: one file
  reads −0.98 in that column while its body-axis column reads 9.78 on the same sample
  (`altusmetrum.ts:89-94`).
- **Pre-lock GPS writes (0, 0) and holds the last fix.** Both must be blanked, or the ground track
  runs to the equator. A satellite count of 0 means the position beside it is held over, and a
  *tracked* count is not a count in the fix — one ground-station file reads 16 to 19 tracked on
  rows whose own fix column says none (`featherweightGps.ts:109-115`).
- **An inertial altitude wraps and drifts.** Across four Blue Raven flights the inertial channel
  ran to ±32,767 ft (a 16-bit wrap) on two, and integrated away to −151,147 ft on a third. Two
  bounds cut it: a single-sample step near 65,536 ft, and a disagreement with the barometer larger
  than the whole flight. Neither is tuned, and neither may fire when the inertial channel is the
  only altitude there is (`blueraven.ts:216-283`).
- **The barometer reads the rocket descending through Mach 1**, as the shock crosses the static
  port; one corpus flight reads 307 ft below its pad. That is exactly where the inertial channel is
  the good one (`blueraven.ts:160-164`).
- **Units can be read off the data when the header is bare.** Blue Raven's high-rate columns state
  no units, so they are settled by magnitude: gyros rail at 2,291–2,294, which is degrees per
  second (radians would be 365 revolutions per second), and accelerometers sit at 0.9935–0.9947 on
  the pad, which is g (`blueraven.ts:434-480`).
- **Saturation is counted, not assumed from a value.** Four files rail at four different numbers,
  so what is counted is samples sitting at exactly the channel's own maximum: railed axes repeat it
  13 to 6,729 times, unrailed ones once or twice, with nothing between (`blueraven.ts:513-523`).
- **Windows must be spans of clock, not counts of samples**, throughout — see the readings note.
- **Several time bases in one file** are normal (Raven FIP, Entacore AIM): each channel carries its
  own time column, the densest becomes the master clock, and the rest are resampled onto it with
  the ends clamped rather than extrapolated (`multiTimebase.ts:14-61`).
- **Re-importing an analyzer's own export is a silent corruption**, not an error: derived columns
  claim the roles the recorded ones should have. Measured across the corpus, 19 of 48 recordings
  shifted their peak acceleration, worst +41.4%, and 16 flipped a reading's provenance. Debrief's
  own export is therefore recognised and refused (`canonical.ts:23-45`).

## What every parser produces

One canonical record: a time vector in seconds from the file's own zero, monotonic but not
uniform, plus channels of one kind each in SI, NaN marking a gap, and the source label kept
verbatim (`lib/flight/types.ts:1-4, 77-91, 147-189`). The kinds are fixed at 23, each with its unit
decided by the kind. Three of those distinctions were paid for:

- `rollAngle` is not `rollRate`: a ±180° column read as a rate reports a plausible 179.99 deg/s.
- `satellites` means satellites *in the fix*; a tracked count goes to `other`.
- A high-rate stream's `accelAxis`/`angularRate`/`attitudeQuaternion` exist so the analysis
  **cannot** read them as flight channels, since it only ever sees that stream as an envelope.

A device's own headline figures ride along as declared values with their source named, never merged
into the measurements and never merged with a simulator's (`types.ts:98-145`).

## Clean room: what may be ported, and the one thing that may not

Debrief is MIT and the owner's own, so its parsers may be ported with a note
(`THIRD-PARTY-NOTICES.md`). Several cite their sources and those citations carry over: the MS5607
and MP3H6115A datasheets and the AltOS record layout (`altosEeprom.ts:9-15`); PKWARE APPNOTE.TXT
and ECMA-376 for the spreadsheet and ZIP reading (`xlsx.ts:3-7`); the u-blox protocol specification
UBX-13003221 for GPS fix types (`lib/gpsFix.ts:67-70`); Featherweight's September 2025 manual for
the Blue Raven, quoted once to *reject* what it says — the vendor's stated method for finding the
rocket's long axis picks the wrong axis on two of four corpus files, and gravity at rest is used
instead (`blueraven.ts:356-374`). The AltOS CSV, Eggtimer, Entacore, Raven FIP, PerfectFlite and
AltimeterCloud parsers cite no outside document and were written from exported files.

**The `.ork` parser is clean**, and this was checked rather than assumed. Its header states it was
written from OpenRocket's published file-format page, which names the ten `flightdata` attributes,
and that the `status` vocabulary was deliberately not read because the page defers it to the GPL-3
reference implementation (`openrocket.ts:10-14`). The code bears that out: `status` is carried
verbatim and never branched on (`:92-96, 320, 477`). Two corrections to that header, from reading
the code: the `<databranch>` time series **is** read, by exact column name, which a later change
added — but from the file's own `types=` header rather than from any source, so the clean room
holds; and of the ten attributes' units, only one is proved, by `maxvelocity / maxmach` landing on
the speed of sound, with the rest inferred from a developer-guide sentence.

**What must not be ported is `COMPETITION.md`.** Two of its rows (45, 38) carry OpenRocket's
simulation-status vocabulary and behaviour taken from named GPL-3 Java files, with links. That is
research read out of copyleft source, and hpr-sim's clean room (CLAUDE.md rule 3) does not permit
it as an input. The mirror is for `lib/`, its tests and its fixtures; `COMPETITION.md`'s
OpenRocket rows are out of bounds for M3.1 and anything else.

One more licensing fact to carry: OpenRocket's own example design is GPL-3, which is why Debrief
has no public `.ork` fixture and three of its tests skip in CI. hpr-sim may not vendor one either.

## Fixtures

Debrief ships 13 public fixtures in its own repository — AltimeterCloud, AltOS CSV, Blue Raven
(low-rate, high-rate and summary), Entacore AIM, Raven FIP, both Featherweight GPS variants, both
PerfectFlite shapes, and a spreadsheet — each trimmed from a publicly shared flight with its source
and ground truth recorded. **Those are the ones that may appear in hpr's examples and doctests.**
Two of them are the same physical flight from two altimeters and cross-check at about 1,009 ft.
There is no public fixture for the AltOS `.eeprom`, the `.rff`, Eggtimer or the RRC3; those live in
the private corpus (`refs/debrief-fixtures`), which stays uncommitted and unquoted, and only counts
and error statistics computed from it may be published (CLAUDE.md rule 4).
