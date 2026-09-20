# Ten logger families, and how their files are read (M7.1)

**What this covers.** The format half of Debrief: which loggers write what, how each file is told apart from the others, and the traps that only show up on real files. **What it is for:** whoever builds M7.1, so that the byte layouts and the quirks do not have to be rediscovered from flights. **How far to trust it:** the accuracy figures are Debrief's own, measured against a private corpus of 61 logs across ten device families. The readings taken off a parsed flight are [the readings note](debrief-flight-readings.md).

Debrief is `nrdptel/fusionspace-debrief`, Neer's own MIT-licensed browser flight-log analyzer, now
sunset (ADR-046). Everything below was read at commit `9e72db8`; paths are relative to that
repository and to the local mirror `refs/fusionspace-debrief`, which `cargo xtask refs fetch`
brings down. A citation of the form `:463-469` continues the file named just before it. **Debrief's
claims are leads**: nothing here has been re-derived in Rust, and every number hpr publishes has to
be re-measured against a primary source or an oracle before it is published.

## How a file finds its parser

Each parser answers `detect(input) -> 0..1` and the highest score wins, ties going to the earlier
entry in the list (`lib/parsers/index.ts:32-55`). Below 0.6 nothing is claimed and a generic
column-mapping path takes over. Two decisions worth keeping:

- **Detection reads bytes and text both** (`lib/parsers/types.ts:8-23`). A binary flight record —
  an AltOS `.eeprom`, an RRC3 `.rff` — cannot be recognised from a decoded string at all, and its
  text view is mojibake.
- **A refusal is a result.** A parser may throw a *guidance* error that reaches the user as a
  sentence (`types.ts:51-56`): this is a design file and not a flight, or a raw card dump with the
  vendor tool that exports it named. Any other failure falls through to column mapping. The
  alternative, tested and rejected, is telling someone their flight log is not a flight log, which
  sends them after the wrong problem (`lib/parsers/rawDownload.ts:1-12`).

Text decoding handles a UTF-8 BOM, UTF-16LE and UTF-16BE by BOM, **and** BOM-less UTF-16 by NUL
density — at least a fifth of the bytes NUL, and eight times as many at even byte offsets as odd —
because one vendor's text export and Excel's "Unicode Text" are both UTF-16 (`lib/encoding.ts:27-61`).
Delimited text tries comma, tab, semicolon and pipe, scoring each by how consistent the column
count is over the first 50 lines; when semicolon wins, cells like `1,5` are decimal commas and are
rewritten (`lib/csv.ts:11-47, 108-118`).

## The families

Ten device families, some of which write two different files, plus two inputs that are not loggers
at all: a spreadsheet export and an OpenRocket design.

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
the device is set to feet or metres, and the two readings differ by 3.28× in whichever direction
the guess is wrong: read a file written in feet as metres and every height reads **3.28× high**,
which is the direction that flatters the flyer; assume feet on a metric file, as these parsers do
by default, and it reads **3.28× low**.

**Do not inherit Debrief's note on this: it has the direction backwards.** Its Eggtimer parser
declares the column as feet (`eggtimer.ts:102`) and a declared unit is multiplied into metres by
its factor, 0.3048 (`lib/units.ts:22-33`), so a metric file comes out low — while the note it shows
the flyer says "if yours was set to metric, these figures read about 3.3× too high"
(`eggtimer.ts:113`). Checked in both files on 2026-09-20. M7.1 settles this with a fixture in each
unit, not by copying either sentence.

The RRC3 parser has the fix worth porting: where the file also carries pressure, the ambiguity is
**resolved from physics** rather than guessed, by testing the stated altitude against the apogee
the pressure drop implies (`missileworksRrc3.ts:49-58`), falling back to the vendor default only
when the pressure cannot settle it.

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
v1 none bit-identical, worst disagreement 0.0035 Pa over 2,206 samples (`:11-15`). The cold branch
and the MS5611 variant are exercised by no corpus flight and rest on a second reading of the same
datasheet page — which, as the file says, catches a transcription slip and not a misreading, since
both readings are the same person's.

**Missile Works `.rff`** (`lib/parsers/missileworksRff.ts`) is a .NET `BinaryFormatter`
serialisation stream holding a list of 16-bit integers. Four things to carry over:

- The array is found by scanning for the primitive-array record and its matching member reference,
  and **only the list's `size` slots are real** — the rest is spare capacity, and reading it
  appends a run of zero-pressure samples (`:91-95`).
- A word below `0x4000` is a barometer sample in tenths of a millibar; at or above it, an auxiliary
  word, written in pairs once a second.
- That threshold is physics, not a fitted constant: 0x4000 in those units is 1,638 mbar, half again
  the highest pressure recorded at sea level. It is a **threshold and not a bit test**, because a
  word with bit 15 set and bit 14 clear would pass a bit test and read back as 3,277 mbar
  (`:32-39`).
- There are no timestamps, so the 20 Hz clock is an assumption — and it is tested: one auxiliary
  pair per second against 20 samples per second, within 10% plus 2, or the file is refused
  (`:189-200`). What the pair *means* is deliberately not decoded: the calibration is not in the
  file.

## Traps that generalise

- **A second altitude column is a second instrument, not a duplicate.** AltOS writes `altitude`
  twice; the later one is the GPS receiver's, found by position after `longitude` because the name
  collides (`altusmetrum.ts:169-181`).
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
- **The barometer reads the rocket descending through Mach 1** as the shock crosses the static
  port — one corpus flight reads 307 ft below its pad, which is exactly where the inertial channel
  is the good one (`blueraven.ts:160-164`).
- **Units can be read off the data when the header is bare.** Blue Raven's high-rate columns state
  no units, so they are settled by magnitude: gyros saturate at 2,291–2,294, which is degrees per
  second (radians would be 365 revolutions per second), and accelerometers sit at 0.9935–0.9947 on
  the pad, which is g (`blueraven.ts:434-480`).
- **Saturation is counted, not assumed from a value.** Four files saturate at four different
  numbers, so what is counted is samples sitting at exactly the channel's own maximum: a saturated
  axis repeats it 13 to 6,729 times, an unsaturated one once or twice, with nothing between
  (`blueraven.ts:513-523`). ("Saturated", not "railed", throughout these notes: in this project a
  rail is the launch rail.)
- **Windows must be spans of clock, not counts of samples**, throughout — see
  [the readings note](debrief-flight-readings.md).
- **Several time bases in one file** are normal (Raven FIP, Entacore AIM): each channel has its own
  time column, the densest becomes the master clock, and the rest are resampled onto it with ends
  clamped rather than extrapolated (`multiTimebase.ts:14-61`).
- **Re-importing an analyzer's own export is a silent corruption**, not an error: derived columns
  claim the roles the recorded ones should have. Measured across the corpus, 19 of 48 recordings
  shifted their peak acceleration, worst +41.4%, and 16 flipped a reading's provenance. Debrief's
  own export is therefore recognised and refused (`canonical.ts:23-45`).

## What every parser produces

One canonical record, whose shape [the readings note](debrief-flight-readings.md) describes. What
belongs here is the channel vocabulary: 23 kinds, each with its unit fixed by the kind
(`lib/flight/types.ts:9-91`). Three of those distinctions were paid for — `rollAngle` is not
`rollRate`, since a ±180° column read as a rate reports a plausible 179.99 deg/s; `satellites`
means satellites *in the fix*, with a merely tracked count sent elsewhere; and a high-rate stream's
axis, rate and quaternion kinds exist so the analysis **cannot** read them as flight channels,
because it only ever sees that stream as an envelope. A device's own headline figures ride along as
declared values with their source named, never merged into the measurements (`types.ts:98-145`).

## What may be ported from it, and what may not

A separate question, with its own answer: [the porting boundary](debrief-porting-boundary.md) sets
out which parsers cite which published sources, why the `.ork` parser is clean room while
`COMPETITION.md` is not, and which fixtures may appear in hpr's examples.
