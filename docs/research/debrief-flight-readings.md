# What a flight log yields, as Debrief works it out (M7.2)

**What this covers.** The analysis half of Debrief: the readings it takes off a log, how it decides
what it may state, how it finds the events, and how it treats two recordings of one flight. **What
it is for:** whoever builds M7.2, so the method and its measured limits do not have to be
rediscovered. **How far to trust it:** numbers attributed to "the corpus" were measured by Debrief
against private flight logs, so they are stated as figures without naming the flights.

Debrief is `nrdptel/fusionspace-debrief`, Neer's own MIT-licensed browser flight-log analyzer, now
sunset (ADR-046). Everything below was read at commit `9e72db8`; paths are relative to that
repository and to the local mirror `refs/fusionspace-debrief`, which `cargo xtask refs fetch`
brings down. A citation of the form `:2194-2205` continues the file named just before it.
**Debrief's claims are leads**: nothing here has been re-derived in Rust, and every number hpr
publishes has to be re-measured against a primary source or an oracle first. The format half —
which loggers write what, and how to read their files — is
[the formats note](debrief-log-formats.md).

## Provenance is not one field

`analyzeFlight` (`lib/analyze/index.ts:1135`) takes a parser-neutral record — a time vector in
seconds, monotonic but **not uniform**, plus channels of one kind each with NaN marking a gap
(`lib/flight/types.ts:147`) — and returns series, events, metrics and warnings.

The design worth copying: there is no single `provenance` enum on a reading. What a reader is told
is **generated from typed facts**, each recording one thing that can be wrong:

| field | says |
|---|---|
| `maxVelocitySource: 'device' \| 'baro'` | an instrument measured the speed, or hpr differentiated an altitude |
| `derivedVelocityFrom: 'baro' \| 'gps' \| null` | which altitude it was differentiated from, because the two fail differently |
| `maxVelocityWithheld: 'gap' \| 'implausible' \| null` | why a reading is absent |
| `accelClipped: bool` | the sensor saturated, so the figure is a floor |
| `apogeeIsFloor: bool` | the log ended at its own peak |
| `altitudeUnproven: bool` | the climb took longer than a vertical throw of that height would |
| `burnoutSource: 'measured' \| 'derived' \| null` | a real thrust crossing, or the speed peak standing in |

The words a reader sees are written once from those facts (`lib/readings.ts:51-105, 269-272`), so
two surfaces cannot word one caveat differently: a derived speed reads "derived, which usually
reads high at the peak" — *usually*, because five of six corpus pairs read high and one reads 14%
low. A tendency, not a bound.

**A reading the log cannot support is withheld with its reason, never printed.** Six guards refuse
a peak speed (`lib/analyze/index.ts:2194-2205`, `:1929-1969`):

| guard | test | why it is there |
|---|---|---|
| absurd | above 4,000 m/s | roughly twice the fastest amateur rocket |
| noise | the ascent's worst negative exceeds 20% of the peak | a trace swinging that far has no usable sign |
| at liftoff | the peak is the liftoff sample itself | a bigger spike shrinks its own noise ratio, so the noise guard cannot see it |
| beyond the accelerometer | above `∫(a − g)dt` by more than 1.5×, floor `√(2gΔh)` | the flight's own second instrument brackets it — and only when the floor is under the ceiling, so a broken bound cannot accuse a sound barometer |
| outclimbs itself | a derived speed whose climb is under 1% of `v²/2g` | the corpus spread is 6.3% to 81.7% |
| gap | a hole in the **sampled altitude**, not merely in the clock | a ground station keeps writing rows through a dropout |

When any fires, the speed, Mach, max-Q, transonic crossing, burnout speed, coast efficiency and
rail exit all go; apogee, timings and descent still read.

**Saturation is detected from the shape of the trace** (`lib/analyze/signal.ts:263-283`,
`index.ts:2011-2015`): the longest run within `max(0.003 × peak, 0.25 m/s²)` of the trace's own
maximum, over at least `max(4, 0.05 s)` of samples. A real boost rounds over its peak, because mass
falls through the burn, so a dead-flat plateau means the sensor saturated. The maximum is then a
floor, and thrust-to-weight is withheld outright if its window was ≥97% of that maximum.

**Altitude is withheld per sample, not per reading** (`index.ts:1771-1866`), against three bounds:
a floor (the running maximum since liftoff — a climbing rocket is not below a height it passed), a
ceiling (`h_liftoff + peak speed × elapsed`, the mean value theorem, used only when the speed is
measured), and the logger's own inertial altitude, consulted only above Mach 0.9 where a static
port stops being trustworthy. Tolerance is `max(30 m, 3% of apogee)`; a contradicted sample reads
as unknown while the time, the speed and the chart are untouched. A GPS altitude is deliberately
**not** an eligible second opinion: no corpus GPS channel agrees with its own barometer over the
uncontradicted ascent, median gaps 105 to 573 m.

## The readings themselves

Apogee is the peak of a despiked altitude (a Hampel filter, described at the end of this note),
clamped back if the peak sits after a sustained descent of 3 s has begun (`index.ts:1570-1622`); at
0.5 s instead of 3 s one high-altitude corpus flight's apogee moved 28 s early. Maximum speed prefers a device channel, but only if that column
is not a finite difference of the file's own altitude — the discriminator is
`finiteDifferenceMatch` (`signal.ts:171`), where every genuine speed channel scores at most 0.44
and a differenced column scores 1.00. A barometric speed is kept **in parallel** always, because a
device speed is usually accelerometer-integrated and drifts toward zero after deployment; every
descent rate and the landing detector read the barometric one.

Maximum acceleration is **withheld entirely** when it would come from differentiating an altitude
twice: a quantised trace spikes to hundreds of g. Boost average is time-weighted, `∫a dt / T`
(16.1% on one corpus flight); thrust-to-weight differences a 0.2 s mean after liftoff against a pad
rest mean, cancelling the logger's gravity convention exactly (`:2822-2886`).

Descent rates are each leg's **own chord** between two short medians, not a mean of a smoothed
derivative (`:2562-2614`). The derivative published 15.59 m/s where the chord reads 6.36, and
across 8 groups with two or more recordings of one leg, 7 tightened and none widened. A leg must
drop more than `max(3 m, 10% of its start height)` and may not exceed `√(2g·apogee)` — three corpus
files otherwise produced "main descents" of thousands of feet per second. Where no main is found
the whole-descent rate goes in a **separate field**: letting it fall into the main's field made
four recordings of one flight disagree by 121.6%.

Coast efficiency is `(apogee − burnout height) / (v_bo²/2g)`: energy conservation on the flown
numbers, no drag model, no density, no reference area (`:2907-2917`).

## Events

Six — liftoff, burnout, apogee, drogue, main, landing (`lib/analyze/types.ts:1`):

- **Liftoff**: the first of two consecutive samples above 2 g, scanning forward but stopping once
  the altitude passes `max(5 × baseline noise, half the apogee)` — without that ceiling a lateral
  blip at ejection pins liftoff near apogee. Failing that, 3 m and 2 m/s (`:1636-1658`).
- **Burnout**: a signed-axial crossing of zero, searched from the acceleration peak to 1 s of
  **clock** past the speed peak (`:2285-2295`). The bound sits past that peak on purpose: the trace
  is specific force, so `dv/dt = a − g`, and thrust equals drag necessarily later. The corpus gap
  is 0.05 to 0.40 s over fourteen flights. Otherwise the speed peak stands in, and `burnoutSource`
  says so.
- **Main**: found **backwards from landing** (`:2503-2531`), because a drogue descent can have a
  slow patch high up. A terminal rate over the last 2 s; the walk back accepts while the rate is
  within `max(1.6 × terminal, terminal + 3)`; the candidate is refused unless the leg above it is
  1.4 times faster, so a single-deploy descent marks no main.
- **Drogue**: **not detected at all**, deliberately — the leg is assumed to start at apogee, since
  a null column would read as "no drogue" rather than "not measured".
- **Landing**: below 2 m and staying under 5 m for a second, plus an at-rest fallback that may add
  a landing but never move one. Both require the record to run longer after apogee than a vacuum
  fall from that height would take (`:994`, `:2304-2375`).

## Two recordings of one flight

Alignment is two lines (`lib/stitch.ts:196-197`): take each record's own liftoff and offset by its
negation. No cross-correlation, no lag sweep, no threshold — every board in an airframe leaves the
pad at the same instant — and the method rides in the result as the literal string
`'shared liftoff'` rather than being implied.

**Nothing is ever averaged into one number, and the type enforces it.** The result carries one
offset per recording and burn marks that keep the file they came from; there is no reducer, no
weighting, no fused series, and `verified` is hard-typed as the literal `false`
(`lib/stitch.ts:142`) so no path can set it true. Its one aggregate, the spread of burn durations,
is documented in place as a description and not a check.

Three validation rules were tried and **deleted**, each with the measurement that killed it
(`lib/stitch.ts:24-54`): altitude at start is useless, since each record has its own datum; a
pre-liftoff motion check flagged 14 of 50 ordinary flights; and a burn-duration gate has no power
at all — lined up on liftoff the gap is just the difference of the two burns — while refusing 2 of
6 sound groups. Residual alignment error between two boards in one airframe is 0.56 s on altitude
and 0.74 s on speed, so a composite may order events but must not print a time to a tenth.

## The atmosphere it needs, and the readings that need it

Three functions and five constants (`lib/analyze/index.ts:260-392`): `R = 287.05 J/(kg·K)`,
lapse `−0.0065 K/m`, `g = 9.80665 m/s²`, sea level `101325 Pa`, tropopause `11,000 m`. Density is
`ρ = p₀/(RT₀) (T/T₀)^(−g/(RL) − 1)` anchored to **pad** pressure and temperature rather than sea
level, so a mile-high site reads its own thinner air; the speed of sound is `√(1.4RT)` with the
temperature capped at the tropopause. Pressure becomes altitude through the firmware's own numeric
form, `44330 (1 − (p/p_pad)^(1/5.255))`, reproducing a logger's own heights rather than differing
in the third digit. A ground temperature outside −90 to +65 °C is discarded for the standard day: a
mis-scaled column would otherwise drive every Mach off a speed of sound three times wrong.

`hpr-atmos` already has the constants, the pad-anchored profile (`Ussa76::anchored`) and the speed
of sound, which is why `hpr-flightdata` may depend on it. **It does not have the inverse.** Nothing
in the crate turns a pressure into an altitude — its surface is `sample(height_msl_m)` and the
geopotential conversions — so M7.2 has to add one, and the whole point of that piece is to match a
logger's own published heights rather than differ with them in the third digit. Checked against
`crates/hpr-atmos/src/ussa76.rs` on 2026-09-20.

**Needs an atmosphere:** Mach, the transonic crossing, maximum dynamic pressure and its altitude,
the speed-of-sound and density series, drag and parachute coefficients, rail-exit Mach, the Mach
0.9 gate in the altitude check, pressure-to-altitude for a file with no altitude channel.

**Needs none:** apogee, time to apogee, maximum speed itself, maximum and average acceleration,
deceleration, saturation, thrust-to-weight, burn time, burnout height and speed, coast time and
efficiency, drag loss, the descent rates, descent and flight time, main deploy time, battery, roll
rate and revolutions, tilt at burnout, the GPS apogee fields, and every event.

## The published sources

Debrief cites five published sources and no others — the standard atmosphere, a barometer
datasheet, two NASA reports and the Hampel-filter paper — listed with what each is used for in
[the porting boundary](debrief-porting-boundary.md). Where a method is Debrief's own, there is
deliberately no citation and the write-up says the number came from the corpus instead, which is
what hpr's own first hard rule asks of every model page.

## For whoever ports this

The filters tolerate NaN and non-uniform steps throughout, and every window is sized in **seconds
of clock**, not samples (`index.ts:58-63`): one corpus recording published a 22.8 g apogee shock
from its CSV and 1.5 g from its eeprom, because a fixed sample count covered 0.13 s in one and
8.24 s in the other. The despiking is a Hampel filter — a sliding-window median replacing a sample
more than four robust standard deviations from its neighbours, the scale being 1.4826 times the
median absolute deviation (MAD) — over 0.3 s; a plain median of that width passes the
2-to-4-sample spike an ejection charge makes.

The structural lesson the code states about itself: a decision the analyzer had already made — this
speed cannot be trusted — was re-derived independently by four display surfaces, which republished
the very samples the headline existed to refuse. The fix was to put the decision **on the data**,
as a non-optional field whose name does not commit to a cause; in Rust, a `Result<Reading,
Refusal>` or a reading plus a reason enum, which `lib/rail.ts:61-85` already resembles. There is no
confidence score anywhere in that codebase: ambiguity is refused, never scored.
