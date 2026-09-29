# Flight-log readings

## In short

- **What it models:** the readings taken from a [flight log](../glossary.md#flight-log) on its own,
  with no design file and no simulation: liftoff, apogee and the time to it, the top speed in the
  climb, landing, and the mean rate of descent. Each reading is a value that says where it came
  from, or is **withheld** with the reason the log can't support it. It never guesses.
- **Sources:** the [running median](../glossary.md#running-median) is the Hampel filter at
  threshold zero, as Pearson and colleagues define it ([References](#references)). The thresholds
  (a 3 m climb, landing within 2 m, a 4,000 m/s ceiling, a 20% noise share) are those of Debrief, the
  project owner's earlier flight-log analyzer, set on its collection of real logs rather than
  taken from a published source. The landing check is a fall from rest in vacuum: Newton's, with
  no model to doubt.
- **How well it is validated:** against an invented flight whose every reading is known exactly,
  each reading lands within the bound the log's rounding and the filter allow
  ([Checked against](#checked-against)). On a real log, Debrief's public Pnut file, hpr reads
  1,010 ft where the altimeter states 1,009 ft. That is one flight. The private collection of logs
  Debrief was built on hasn't been read yet: that is [M7.1](../decisions-and-roadmap.md#m7-1)'s
  work.
- **What it leaves out:** the drogue and main descent rates each on its own, Mach number,
  dynamic pressure, burnout, and anything an accelerometer or GPS would give. It takes the
  logger's altitude as the logger converted it, with no correction for the day's air
  ([What it leaves out](#what-it-leaves-out)).

## Why these rules

Code: `hpr_flightdata::readings`
([API reference](../api/hpr_flightdata/readings/index.html)), on the command line
[`hpr analyze`](../cli.md#hpr-analyze). It came with [M4.2d](../decisions-and-roadmap.md#m4-2d)
([ADR-108][adr-108]). The one log format read so far is [PerfectFlite's `.pf2`](../format/pf2.md).

A barometric altimeter's altitude is a good record of a flight's heights, with two flaws a
reading must survive. It moves in steps of its resolution, one foot for a PerfectFlite, so the top
of the climb sits flat for several samples. And when the ejection charge fires, the pressure
inside the electronics bay jumps, and for a tenth of a second the altitude swings tens of feet
away from the truth. Take the highest sample as the apogee and you read that swing.

So every height and time comes from the altitude after a running median, which removes the
swing, and each reading has a rule that says when the log can't support it.

## The running median

The running median replaces each sample with the median of the samples within `K` places of it,
the window cut short at either end of the log. hpr sets `K` from the log's own sample interval
`Δt` so that the window spans 0.3 s: at a PerfectFlite's 20 samples a second, `K = 3`, seven
samples.

Pearson and colleagues define the Hampel filter by the window's median `m_k` and its scale
`S_k = 1.4826 × median |x_(k−j) − m_k|`: a sample more than `t·S_k` from `m_k` is replaced by `m_k`.
At `t = 0` every sample is replaced, and the filter is the running median (§1, eqs. 1 and 2).

Two properties make it the right tool here:

- **It removes any pulse up to `K` samples wide.** Such a pulse holds at most `K` of the window's
  `2K + 1` samples, so the median is never one of them. On the public log below, the ejection
  pulse's high side is two samples wide.
- **It reads a smooth peak low, never high, and by little.** At the highest sample, `K + 1` of the
  window's samples lie within `⌈K/2⌉` places of it, so the median is at least the altitude that
  far from the peak. A coasting rocket's altitude near apogee bends at `g`: the vertical speed,
  and so drag's vertical share, is zero there. Counting half a sample for the true peak falling
  between samples, the apogee reads low by at most `g (K·Δt)² / 2` = 9.81 × 0.15² / 2 = **0.11 m**.

Debrief uses the Hampel filter at `t = 4` over the same window. hpr doesn't, because of what
the public Pnut log shows. Its ejection pulse dips 26 ft below the trace, then rises 48 ft above
it. The dip sits in the rise's own window, widens that window's scale `S_k`, and the Hampel filter
keeps the rise: its highest value is the pulse's 1,028 ft, 19 ft above the 1,009 ft the altimeter
states. After the running median, the apogee reads 1,010 ft. The invented flight below repeats that
shape, so the tests show the difference where anyone can run them.

## Each reading

In the rules below, the **pad** is the filtered altitude at the log's first sample. A
PerfectFlite zeroes its altitude on the pad, so a pad more than 3 m from zero means the log didn't
start there.

| reading | rule | where it comes from |
|---|---|---|
| liftoff | the last sample at or below the pad before the filtered altitude first climbs 3 m above it | the barometer |
| apogee | the filtered altitude's highest value; its time the middle of the run of samples that hold it | the barometer |
| time to apogee | apogee's time less liftoff's | the barometer |
| top speed | the highest of the logger's own vertical speed from liftoff to apogee | the logger's speed column, which it works out from its barometer |
| top acceleration | withheld when the log has no accelerometer | none |
| landing | the first sample after apogee within 2 m of the pad that stays under 5 m for a second | the barometer |
| mean descent rate | the filtered height lost from apogee to landing, over the time taken | the barometer |

Liftoff is the last sample before the altitude shows the rocket moving. The rocket had risen less
than the altitude's resolution then, and passed it within the next sample.

Landing is the first sample within 2 m of the pad, so it comes before touchdown by the time the
last 2 m took: a third of a second under a main at 6 m/s. The mean descent rate covers the drogue
and the main together. Splitting it into each leg's own rate is
[M7.2](../decisions-and-roadmap.md#m7-2)'s work.

The top acceleration is withheld rather than worked out from the altitude. Differencing twice
turns the altitude's one-foot steps into spikes: at 20 samples a second, one step differenced
twice is 0.3048 / 0.05² = 122 m/s², over 12 g.

## When a reading is withheld

A withheld reading has a code and a sentence with the log's own numbers. Only the readings that
need the missing one go: a log that starts in the air loses its liftoff and top speed, but keeps
its apogee and landing.

| code | when | readings withheld |
|---|---|---|
| `too_short` | the log has fewer than 3 samples | all |
| `no_climb` | the filtered altitude never climbs 3 m above the pad | all |
| `starts_off_the_pad` | the pad is more than 3 m from the logger's zero | liftoff, and so the top speed |
| `ends_before_landing` | the log ends before the altitude comes within 2 m of the pad and stays under 5 m for a second | landing |
| `faster_than_free_fall` | the altitude reaches the pad sooner after apogee than a fall from rest in vacuum could, `√(2h/g)` for an apogee `h` above the pad | landing |
| `no_speed_column` | the log has no speed column | the top speed |
| `implausible_speed` | the speed column peaks above 4,000 m/s, about twice the fastest amateur rocket | the top speed |
| `noisy_speed` | from liftoff to apogee the speed swings below zero by more than 20% of its top | the top speed |
| `speed_peak_at_liftoff` | the speed peaks on the liftoff sample itself: a spike, not a climb | the top speed |
| `no_accelerometer` | the log has no accelerometer | the top acceleration |
| `needs` | the reading needs another that was withheld | as the sentence says |

An apogee within half a window of the log's end is read, but marked `is_floor`: the log may have
stopped before the rocket did.

## Checked against

**An invented flight.** The file
[`synthetic-pnut.pf2`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/logs/synthetic-pnut.pf2)
holds a flight made up for the tests. It sits on the pad for half a second, then accelerates at
50 m/s² for 1.6 s and coasts with no drag. It falls from rest to 25 m/s, descends at 25 m/s to
150 m and at 6 m/s to the ground. It is written as a Pnut writes one, rounded to whole feet and
feet per second, with an ejection pulse of the public log's shape a second after apogee. Every
reading is known in closed form. The
[tests](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-flightdata/src/synthetic.rs)
hold each within the bound worked out beside it:

| reading | true | hpr reads | allowed |
|---|---|---|---|
| liftoff | 0.50 s | 0.55 s | from one sample before to 0.078 s after, when the height first rounds to a foot |
| apogee | 390.31 m (1,280.5 ft) | 390.14 m (1,280 ft) | 0.26 m: half a foot of rounding and the median's 0.11 m |
| apogee time | 10.26 s | 10.28 s | 0.25 s, the span over which the rounded peak sits flat |
| top speed | 80.0 m/s at 2.10 s | 79.9 m/s (262 ft/s) at 2.10 s | half a foot per second |
| landing | 46.14 s, touchdown | 45.85 s | up to 0.38 s early: 2 m at 6 m/s, and one sample |

The file's highest sample is the pulse's, 400.5 m. The Hampel filter keeps it, and the median
sets it aside; a test holds both.

**A real flight.** Debrief ships a public PerfectFlite Pnut log, trimmed from a publicly shared
flight. Its terms upstream are unclear, so it isn't committed here. Where it has been fetched, a
[test](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/cli.rs) reads it and
holds these numbers:

| | the altimeter states | hpr reads |
|---|---|---|
| apogee | 1,009 ft | 1,010 ft (307.8 m) |
| highest sample | | 1,028 ft, the ejection pulse, set aside |
| top speed | | 257 ft/s (78.3 m/s) |

A Featherweight Raven flew on the same flight, and Debrief reports it agreeing at about 1,009 ft.
Reading the Raven's file is [M7.1](../decisions-and-roadmap.md#m7-1)'s work.

## What it leaves out

- **The day's air.** The altitude is the logger's own conversion of its pressure, which assumes a
  standard atmosphere. On a warm day a barometric altimeter reads low, 6.5% on a day 20 K warmer
  than standard ([Atmosphere](atmosphere.md#pressure-altitude-what-a-barometric-altimeter-reads)).
  hpr prints what the logger recorded and doesn't correct it.
- **Fast flights.** Debrief stops trusting a barometer's altitude above Mach 0.9
  ([the readings note](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/debrief-flight-readings.md)).
  hpr has no such check yet, so a fast flight's top speed from a barometer may be off.
- **Each leg's descent rate, and deployment events.** One mean rate covers drogue and main.
- **Mach number, dynamic pressure and burnout.** These need an atmosphere or an accelerometer
  ([M7.2](../decisions-and-roadmap.md#m7-2)).
- **Other loggers.** Only PerfectFlite's `.pf2` is read ([M7.1](../decisions-and-roadmap.md#m7-1)).

## Tests

- `hpr_flightdata::synthetic`: the invented flight, its readings against the truth, and the
  Hampel filter keeping the pulse the median removes.
- `hpr_flightdata::readings`: each withheld code on a log built to trigger it, and each rule at
  its edge. For example, a 20% swing passes and 22.5% is refused.
- `hpr_flightdata::filter`: the median's bound at a parabola's peak, a pulse removed, a ramp
  untouched, the Hampel filter at zero equal to the median, and a property test that every output
  lies within its window.
- `crates/hpr-cli/tests/cli.rs`: `hpr analyze` on a log alone in a folder, its JSON checked against
  the published schema; the public Pnut log, where fetched.

## References

- R. K. Pearson, Y. Neuvo, J. Astola and M. Gabbouj, *The Class of Generalized Hampel Filters*,
  23rd European Signal Processing Conference (EUSIPCO), 2015, §1, eqs. 1 and 2. Pinned as
  `pearson-2015-generalized-hampel` in the
  [reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml).
- Debrief, `lib/analyze/index.ts` and `lib/parsers/perfectflite.ts` (MIT, the project owner's
  own), for the thresholds and the format; notes in
  [the readings note](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/debrief-flight-readings.md).

[adr-108]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-108-a-flight-log-read-alone-perfectflites-pf2-first-heights-after-a-running-median-an-invented-log-in-ci-2026-09-29
