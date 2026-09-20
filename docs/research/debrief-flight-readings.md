# What a flight log yields, as Debrief works it out (M7.2)

**What this covers.** The analysis half of Debrief, the owner's own flight-log analyzer, now folded
into this project (ADR-046): the readings it takes off a log, how it decides what it may state, how
it finds the events, and how it treats two recordings of one flight. **What it is for:** whoever
builds M7.2, so the method and its measured limits do not have to be rediscovered. **How far to
trust it:** everything here was read from `refs/fusionspace-debrief` at commit `9e72db8`, file and
line cited; numbers attributed to "the corpus" were measured there against private flight logs, so
they are stated as figures without naming the flights. None of it has been re-derived in Rust yet.

The format half — which loggers write what, and how to read their files — is
[the formats note](debrief-log-formats.md).

## The shape of it

`analyzeFlight(flight, opts) -> FlightAnalysis` (`lib/analyze/index.ts:1135`) takes a
parser-neutral record and returns series, events, metrics and warnings. The record
(`lib/flight/types.ts:147`) is a time vector in seconds — monotonic but **not uniform** — plus
channels of one kind each, with NaN marking a gap. That is the shape `hpr-flightdata`'s canonical
record needs: a rocket's logger writes at 20 Hz, then at 1 Hz on the way down, and drops rows.

`FlightMetrics` carries 45 fields (`lib/analyze/types.ts:22-164`); 21 are shown to a reader.

## Provenance is not one field

The design worth copying: there is no single `provenance` enum on a reading. What a reader is told
is **generated from typed facts**, each recording one thing that can be wrong:

| field | says |
|---|---|
| `maxVelocitySource: 'device' \| 'baro'` | an instrument measured the speed, or hpr differentiated an altitude |
| `derivedVelocityFrom: 'baro' \| 'gps' \| null` | which altitude it was differentiated from, because the two fail differently |
| `maxVelocityWithheld: 'gap' \| 'implausible' \| null` | why a reading is absent |
| `accelClipped: bool` | the sensor railed, so the figure is a floor |
| `apogeeIsFloor: bool` | the log ended at its own peak |
| `altitudeUnproven: bool` | the climb took longer than a vertical throw of that height would |
| `burnoutSource: 'measured' \| 'derived' \| null` | a real thrust crossing, or the speed peak standing in |

The words a reader sees are written once from those facts (`lib/readings.ts:51-105, 269-272`), so
two surfaces cannot word the same caveat differently: a derived speed reads "derived, which usually
reads high at the peak" — *usually*, because five of six corpus pairs read high and one reads 14%
low, which is a tendency and not a bound.

**A reading the log cannot support is withheld with its reason, never printed.** Five guards refuse
a peak speed (`lib/analyze/index.ts:2194-2205`): above 4,000 m/s; an ascent whose noise floor is
worse than 20% of the peak; a peak at the liftoff sample itself; a peak the flight's own
accelerometer cannot bracket (ceiling `∫(a − g)dt`, floor `√(2gΔh)`, margin 1.5, and only when the
floor is under the ceiling, so a broken bound cannot accuse a sound barometer); and a derived speed
whose climb is under 1% of `v²/2g` when the corpus spread is 6.3% to 81.7%. A sixth reason is a
hole in the sampled altitude — not merely in the clock, because a ground station keeps writing rows
through a dropout (`:1929-1969`). When any fires, the speed, Mach, max-Q, transonic crossing,
burnout speed, coast efficiency and rail exit all go; apogee, timings and descent still read.

**Saturation is detected from the shape of the trace** (`lib/analyze/signal.ts:263-283`,
`index.ts:2011-2015`): the longest run of samples within `max(0.003 × peak, 0.25 m/s²)` of the
trace's own maximum, over at least `max(4, 0.05 s)` of samples. A real boost rounds over its peak,
because mass falls through the burn, so a dead-flat plateau means the sensor railed. The reported
maximum then becomes a floor, and thrust-to-weight is withheld outright if its window was ≥97% of
the clipped maximum.

**Altitude is withheld per sample, not per reading** (`index.ts:1771-1866`). An ascent instant is
placed against three bounds: a floor (the running maximum since liftoff — a climbing rocket is not
below a height it passed), a ceiling (`h_liftoff + peak speed × elapsed`, the mean value theorem,
used only when the speed is measured), and the logger's own inertial altitude, consulted only above
Mach 0.9 where a static port stops being trustworthy. Tolerance is `max(30 m, 3% of apogee)`. A
contradicted sample reads as unknown; the time, the speed and the chart are untouched. A GPS
altitude is deliberately **not** an eligible second opinion: no corpus GPS channel agrees with its
own barometer over the uncontradicted ascent, median gaps 105 to 573 m.

## The readings themselves

Apogee is the peak of a Hampel-cleaned altitude, clamped back if the peak sits after a sustained
descent of 3 s has begun (`index.ts:1570-1622`); at 0.5 s instead of 3 s one high-altitude corpus
flight's apogee moved 28 s early. Maximum speed prefers a device channel, but only if that column
is not a finite difference of the file's own altitude — the discriminator is
`finiteDifferenceMatch` (`signal.ts:171`), where every genuine speed channel scores at most 0.44
and a differenced column scores 1.00. A barometric speed is kept **in parallel** always, because a
device speed is usually accelerometer-integrated and drifts toward zero after deployment; every
descent rate and the landing detector read the barometric one.

Maximum acceleration is **withheld entirely** when it would come from differentiating an altitude
twice: a quantised trace spikes to hundreds of g. Boost average is time-weighted rather than
sample-weighted, `∫a dt / T`, which moves one corpus flight by 16.1%. Thrust-to-weight differences
a 0.2 s mean after liftoff against a pad rest mean, which cancels the logger's gravity convention
exactly (`index.ts:2822-2886`).

Descent rates are each leg's **own chord** between two short medians, not a mean of a smoothed
derivative (`index.ts:2562-2614`). The derivative published 15.59 m/s where the chord reads 6.36;
across 8 groups with two or more recordings of one leg, 7 tightened and none widened. A leg must
drop more than `max(3 m, 10% of its start height)` and may not exceed `√(2g·apogee)` — three corpus
files otherwise produced "main descents" of thousands of feet per second. Where no main is found
the whole-descent rate goes in a **separate field**, because letting it fall into the main's field
made four recordings of one flight disagree by 121.6%.

Coast efficiency is `(apogee − burnout height) / (v_bo²/2g)` — energy conservation on the flown
numbers, with no drag model, no density and no reference area (`index.ts:2907-2917`).

## Events

Six: liftoff, burnout, apogee, drogue, main, landing (`lib/analyze/types.ts:1`).

- **Liftoff**: the first of two consecutive samples above 2 g, scanning forward but stopping once
  the altitude passes `max(5 × baseline noise, half the apogee)` — without that ceiling a lateral
  blip at ejection pins liftoff near apogee. Failing that, 3 m and 2 m/s together (`:1636-1658`).
- **Burnout**: a genuine signed-axial crossing of zero, searched from the acceleration peak to 1 s
  of **clock** past the speed peak (`:2285-2295`). The bound sits past the speed peak on purpose:
  the trace is specific force, so `dv/dt = a − g`, and thrust equals drag necessarily after the
  speed peak. The corpus gap is 0.05 to 0.40 s over fourteen flights. Otherwise the speed peak
  stands in, and `burnoutSource` says so.
- **Main**: found **backwards from landing** (`:2503-2531`), because a drogue descent can have a
  slow patch high up. A terminal rate is taken over the last 2 s, the walk back accepts while the
  rate is within `max(1.6 × terminal, terminal + 3)`, and the candidate is refused unless the leg
  above it is at least 1.4 times faster. A single-deploy descent fails that and no main is marked.
- **Drogue**: **not detected at all**, deliberately. The drogue leg is assumed to start at apogee,
  because a null column would read as "no drogue" rather than "not measured".
- **Landing**: below 2 m, staying under 5 m for a second, plus an at-rest fallback that may add a
  landing but never move one. Both require the record to run longer after apogee than a vacuum fall
  from that height would take (`:994`, `:2304-2375`).

## Two recordings of one flight

Alignment is two lines (`lib/stitch.ts:196-197`): take each record's own liftoff, offset by its
negation. No cross-correlation, no lag sweep, no threshold. Every board in an airframe leaves the
pad at the same instant, so the liftoffs are the same moment, and the method is carried in the
result as the literal string `'shared liftoff'` rather than implied.

**Nothing is ever averaged into one number, and the type is what enforces it.** The result carries
one offset per recording and per-stage burn marks that keep the file they came from; there is no
reducer, no weighting and no fused series in the file, and `verified` is hard-typed as the literal
`false` (`lib/stitch.ts:142`) so no code path can set it true. The one aggregate, the spread of
burn durations, is documented in place as a description and not a check.

Three validation rules were tried and **deleted**, each with the measurement that killed it
(`lib/stitch.ts:24-54`): altitude at start is useless because each record has its own datum;
a pre-liftoff motion check flagged 14 of 50 ordinary flights; and a burn-duration agreement gate
has no power at all — lined up on liftoff the gap is just the difference of the two burns, with the
staging delay not a term — while refusing 2 of 6 sound redundant-board groups. The genuine staged
pair agrees to 0.290 s and an unrelated flight was accepted at 0.750 s, so no tolerance separates
them. Residual alignment error between two boards in one airframe is 0.56 s on altitude and 0.74 s
on speed, which is why a composite may order events and must not print a composite time to a tenth.

## The atmosphere it needs, and the readings that need it

Three functions and five constants (`lib/analyze/index.ts:260-392`): `R = 287.05 J/(kg·K)`,
lapse `−0.0065 K/m`, `g = 9.80665 m/s²`, sea level `101325 Pa`, tropopause `11,000 m`. Density is
`ρ = p₀/(RT₀) (T/T₀)^(−g/(RL) − 1)` anchored to **pad** pressure and temperature rather than sea
level, so a mile-high site reads its own thinner air; the speed of sound is `√(1.4RT)` with the
temperature capped at the tropopause. Pressure becomes altitude through the firmware's own numeric
form, `44330 (1 − (p/p_pad)^(1/5.255))`, so a logger's own heights are reproduced rather than
differed with in the third digit. Ground temperature outside −90 to +65 °C is discarded for the
standard day, because a mis-scaled temperature column would otherwise drive every Mach number off
a speed of sound three times wrong.

hpr has all of this in `hpr-atmos` already, which is why `hpr-flightdata` may depend on it.

**Needs an atmosphere:** Mach, the transonic crossing, maximum dynamic pressure and its altitude,
the speed-of-sound and density series, drag and parachute coefficients, rail-exit Mach, the Mach
0.9 gate inside the altitude check, and pressure-to-altitude for a file with no altitude channel.

**Needs none:** apogee, time to apogee, maximum speed itself, maximum and average acceleration,
deceleration, clipping, thrust-to-weight, burn time, burnout height and speed, coast time and
efficiency, drag loss, all three descent rates, descent and flight time, main deploy time, battery,
roll rate and revolutions, tilt at burnout, the GPS apogee fields, and every event.

## The published sources

Five, and only five, each fetched and read before it was written down
(`lib/methods/references.ts`). The file's own header records why the list is short: a fabricated
citation puts a false claim of provenance on a safety-relevant number.

| source | used for |
|---|---|
| [U.S. Standard Atmosphere, 1976](https://ntrs.nasa.gov/api/citations/19770009539/downloads/19770009539.pdf) (NOAA/NASA/USAF) | lapse rate, speed of sound, pressure–altitude, sea-level fallbacks |
| [Bosch BMP180 data sheet, 2013](https://cdn-shop.adafruit.com/datasheets/BST-BMP180-DS000-09.pdf) | the numeric barometric form altimeter firmware itself uses |
| [Gracey, *Measurement of Aircraft Speed and Altitude*, NASA RP-1046, 1980](https://ntrs.nasa.gov/api/citations/19800015804/downloads/19800015804.pdf) | Mach against the local speed of sound; the transonic static-port error from about Mach 0.9 |
| [Talay, *Introduction to the Aerodynamics of Flight*, NASA SP-367, 1975](https://ntrs.nasa.gov/api/citations/19760003955/downloads/19760003955.pdf) | dynamic pressure as ½ρv² |
| [Pearson et al., *The Class of Generalized Hampel Filters*, EUSIPCO 2015](https://www.eurasip.org/Proceedings/Eusipco/Eusipco2015/papers/1570096433.pdf) | the Hampel filter the altitude is despiked with |

Where a method is Debrief's own — the transonic threshold, the max-Q window, every corpus-measured
bound above — there is deliberately no citation, and the write-up says the number came from the
corpus instead. hpr's own rule (hard rule 1) asks the same of every model page.

## For whoever ports this

The filters tolerate NaN and non-uniform steps throughout, and every window is sized in **seconds
of clock**, not samples (`index.ts:58-63`): one corpus recording published a 22.8 g apogee shock
from its CSV and 1.5 g from its eeprom, because a fixed sample count covered 0.13 s in one and
8.24 s in the other. The despiking is a Hampel filter, `σ = 1.4826 × MAD`, 4σ, over a 0.3 s window;
a plain median of the same width passes the 2-to-4-sample spike an ejection charge makes.

The structural lesson the code states about itself: a decision the analyzer had already made — this
speed cannot be trusted — was re-derived independently by four display surfaces, which then
republished the very samples the headline existed to refuse. The fix was to put the decision **on
the data** as a non-optional field whose name does not commit to a cause. In Rust that is a
`Result<Reading, Refusal>` or a reading plus a reason enum, which `lib/rail.ts:61-85` is already
shaped like.

There is no confidence score anywhere in the codebase. Ambiguity is refused, never scored.
