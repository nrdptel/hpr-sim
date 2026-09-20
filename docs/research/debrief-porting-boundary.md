# What may be ported from Debrief, and what may not (ADR-046)

**What this covers.** Where Debrief's format knowledge came from, parser by parser; why its
OpenRocket parser is clean room while one of its other files is not; and which of its flight
fixtures may be used in this repository. **What it is for:** M7.1 and M3.1, and anyone porting from
the mirror. **How far to trust it:** every claim below was read in the source on 2026-09-20, with
the file and line given, rather than taken from a summary.

Debrief is `nrdptel/fusionspace-debrief`, Neer's own MIT-licensed flight-log analyzer, now sunset
(ADR-046), mirrored read-only into `refs/fusionspace-debrief` at commit `9e72db8` by `cargo xtask
refs fetch`. Being MIT and Neer's own, its code may be ported with a note in
`THIRD-PARTY-NOTICES.md` — the same footing as Loft. The formats themselves are
[the formats note](debrief-log-formats.md); the reading methods are
[the readings note](debrief-flight-readings.md).

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

## The fixtures, and which may be used here

Debrief ships 12 public fixtures in its own repository — AltimeterCloud, AltOS CSV, Blue Raven
(low-rate, high-rate and summary), Entacore AIM, Raven FIP, both Featherweight GPS variants, both
PerfectFlite shapes, and a spreadsheet — each trimmed from a publicly shared flight with its source
and ground truth recorded. **Those are the ones that may appear in hpr's examples and doctests.**
Two of them are the same physical flight recorded by two altimeters — a PerfectFlite Pnut and a
Featherweight Raven — which Debrief's own fixture notes report agreeing at about 1,009 ft, the
figure the Pnut file states for itself (`lib/parsers/__fixtures__/README.md:14, 21-22`). That pair
is the obvious first test for a Rust reader, and the one claim here a reader can check without the
private corpus.
There is no public fixture for the AltOS `.eeprom`, the `.rff`, Eggtimer or the RRC3; those live in
the private corpus (`refs/debrief-fixtures`), which stays uncommitted and unquoted, and only counts
and error statistics computed from it may be published (CLAUDE.md rule 4).

## The five sources it cites

Debrief cites five published sources and no others; its file header records that each was fetched
and read first, and why the list is short — a fabricated citation puts a false claim of provenance
on a safety-relevant number (`lib/methods/references.ts:1-21`). **None of the five is in hpr's
reference lock yet**, so anything hpr cites from them is fetched and pinned under ADR-002 first.

| source | used for |
|---|---|
| [U.S. Standard Atmosphere, 1976](https://ntrs.nasa.gov/api/citations/19770009539/downloads/19770009539.pdf) (NOAA/NASA/USAF) | lapse rate, speed of sound, pressure–altitude, sea-level fallbacks |
| [Bosch BMP180 data sheet, 2013](https://cdn-shop.adafruit.com/datasheets/BST-BMP180-DS000-09.pdf) | the numeric barometric form altimeter firmware itself uses |
| [Gracey, *Measurement of Aircraft Speed and Altitude*, NASA RP-1046, 1980](https://ntrs.nasa.gov/api/citations/19800015804/downloads/19800015804.pdf) | Mach against the local speed of sound; the transonic static-port error from about Mach 0.9 |
| [Talay, *Introduction to the Aerodynamics of Flight*, NASA SP-367, 1975](https://ntrs.nasa.gov/api/citations/19760003955/downloads/19760003955.pdf) | dynamic pressure as ½ρv² |
| [Pearson et al., *The Class of Generalized Hampel Filters*, EUSIPCO 2015](https://www.eurasip.org/Proceedings/Eusipco/Eusipco2015/papers/1570096433.pdf) | the Hampel filter the altitude is despiked with |

Where a method is Debrief's own — the transonic threshold, the max-Q window, every corpus-measured
bound above — there is deliberately no citation and the write-up says the number came from the
corpus, which is what hpr's own first hard rule asks of every model page. What may be ported from
Debrief, and what may not, is [the porting boundary](debrief-porting-boundary.md).
