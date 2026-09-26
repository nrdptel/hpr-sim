# hpr against the logs of real flights

Written by `cargo xtask real-flights` ([M2.3b][m2-3b], decision [ADR-082][adr-082]). hpr flies each rocket of RocketPy's documentation that has a flight log, with its own aerodynamics, on the example's own thrust file, from the example's rail and site, in the ERA5 weather the example reads. The log is the reference. Heights are above each one's start: the log's pad reading, hpr's centre of mass on the rail. The trace RMS is over the ascent, both clocks aligned where the trace first reaches 30 m, until the first of the two apogees. The logs, thrust files and weather files are read from the pinned RocketPy checkout and never committed; each one's SHA-256 is in the JSON. The explanation is on the [documentation site][site].

[m2-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-3b
[adr-082]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-082-real-flights-read-from-refs-compared-over-the-ascent-with-checked-explanations-2026-09-26
[site]: https://nrdptel.github.io/hpr-sim/accuracy.html#real-flights

## Summary

- Flights: 7.
- Mean absolute apogee error: 4.23% against a target of 5%: within target.
- Mean apogee error with its sign: +2.79%.
- Largest trace RMS: 7.50% of its log's apogee.
- Outside the target: ndrt-2020, cavour, lince.
- On each example's own drag, the diagnostic flight: mean absolute apogee error 4.12%.

## Flights

| flight | log apogee (m) | hpr apogee (m) | apogee error | log time to apogee (s) | hpr time to apogee (s) | trace RMS (m) | trace RMS (% of apogee) | rows |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Bella Lui, EPFL Rocket Team, 2020 (K828FJ) | 459.0 | 463.6 | +1.01% | 9.46 | 9.48 | 3.6 | 0.79% | 161 |
| NDRT 2020, Notre Dame Rocketry Team (L1395) | 1320.4 | 1432.8 | +8.52% | 15.76 | 16.70 | 71.4 | 5.41% | 316 |
| Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520) | 3903.8 | 3809.9 | -2.41% | 28.55 | 26.58 | 149.9 | 3.84% | 2878 |
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | 3213.4 | 3174.5 | -1.21% | 25.37 | 25.53 | 74.4 | 2.32% | 508 |
| Cavour, Politecnico di Torino, EuRoC 2023 (L995) | 2789.0 | 3052.1 | +9.43% | 23.07 | 22.69 | 209.3 | 7.50% | 122 |
| Genesis, EuRoC 2023 (L995) | 2916.7 | 2875.3 | -1.42% | 23.33 | 22.47 | 55.6 | 1.91% | 2247 |
| Lince, EuRoC 2023 (M1101) | 3668.5 | 3874.3 | +5.61% | 28.06 | 26.41 | 194.2 | 5.29% | 2642 |

## On each example's own drag

A diagnostic, not a prediction: the same flight with hpr's zero-lift drag replaced by the drag the example's notebook flies (a team's estimate: RASAero II, CFD or a fitted constant), on the example's radius. hpr's normal force is its own in both. Where the error shrinks, the miss was hpr's drag; where it doesn't, it is elsewhere.

| flight | example's drag | apogee (m) | apogee error | trace RMS (m) |
|---|---|---:|---:|---:|
| Bella Lui, EPFL Rocket Team, 2020 (K828FJ) | bella_lui_flight_sim.ipynb:94-95, :453-484 (power off and on, times 1) | 460.7 | +0.39% | 2.4 |
| NDRT 2020, Notre Dame Rocketry Team (L1395) | ndrt_2020_flight_sim.ipynb:91 and :316-317 (the drag coefficient, power off and on) | 1299.1 | -1.61% | 20.0 |
| Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520) | prometheus_2022_flight_sim.ipynb:224-261 (`prometheus_cd_at_ma` from Mach 0.15, where it starts to change, and power on 1.02 times it) | 4197.0 | +7.51% | 264.1 |
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | juno3_flight_sim.ipynb:244-251, :356-359 (`drag_curve.csv`, scaled to 0.38 at Mach 0.6 "from CFD analysis") | 3113.4 | -3.11% | 91.9 |
| Cavour, Politecnico di Torino, EuRoC 2023 (L995) | cavour_flight_sim.ipynb:250-251 | 2825.0 | +1.29% | 70.0 |
| Genesis, EuRoC 2023 (L995) | genesis_flight_sim.ipynb:241-242 | 3050.8 | +4.60% | 107.6 |
| Lince, EuRoC 2023 (M1101) | lince_flight_sim.ipynb:240-241 | 3289.6 | -10.33% | 178.4 |

## Each flight's inputs

- **Bella Lui, EPFL Rocket Team, 2020 (K828FJ)** (`rocketpy-bella-lui`): weather at 2020-02-22T13:00Z; inputs from RocketPy 1.13.0's bella_lui_flight_sim.ipynb:109-111 (rail), :139-153 (site, date), :761-765 (log); total impulse flown 2071.0 N s.
- **NDRT 2020, Notre Dame Rocketry Team (L1395)** (`rocketpy-ndrt-2020-nose-to-tail`): weather at 2020-02-23T16:00Z; inputs from RocketPy 1.13.0's ndrt_2020_flight_sim.ipynb:115-117 (rail), :148-153 (site, date), :679-683 (log); total impulse flown 4894.9 N s. Outside the target (drag): hpr's drag. On the example's own drag (a constant 0.44) the apogee is within the target. hpr's own drag is lower, as predicted mode found against RocketPy flying the same constant (+10.3% in apogee, ADR-023); the design's fin edges and finish are placeholders, since the example records none.
- **Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520)** (`rocketpy-prometheus-2022-generic-motor`): weather at 2023-06-24T15:00Z; inputs from RocketPy 1.13.0's prometheus_2022_flight_sim.ipynb:65-70 (site, date), :401-406 (rail), :508-526 (log); total impulse flown 7579.1 N s. Note: flown in the weather of 24 June 2023, a year after the flight, as RocketPy's example flies it: RocketPy has no ERA5 file of the day.
- **Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor)** (`rocketpy-juno-iii`): weather at 2023-06-23T23:00Z; inputs from RocketPy 1.13.0's juno3_flight_sim.ipynb:54-67 (site, date), :448-456 (rail), :543-553 (log); total impulse flown 8802.6 N s, 2.62 N s of it from reading the file's negative thrust as zero. Note: the log's last two rows (30.45 s and 30.50 s, at the drogue's firing) read 10700.59 m and -2490.543 m, a corrupted end, and are not read; the thrust file's last five points are negative (-6.8 to -47.3 N), and hpr, which refuses a negative thrust, reads them as zero.
- **Cavour, Politecnico di Torino, EuRoC 2023 (L995)** (`rocketpy-cavour`): weather at 2023-10-13T12:00Z; inputs from RocketPy 1.13.0's cavour_flight_sim.ipynb:125-132 (site, date), :314-315 (rail), :379-389 (log); total impulse flown 3618.0 N s. Outside the target (drag): hpr's drag. On the example's own RASAero II curves the apogee is within the target. hpr's drag is below those curves: at Mach 0.3, 8.3% below the power-off curve and 18.3% below the power-on one (ADR-009; the power-on gap's cause is open), and its design's fin edges are placeholders, since the example records none.
- **Genesis, EuRoC 2023 (L995)** (`rocketpy-genesis`): weather at 2023-10-12T13:00Z; inputs from RocketPy 1.13.0's genesis_flight_sim.ipynb:124-131 (site, date), :326-327 (rail), :391-406 (log); total impulse flown 3618.0 N s.
- **Lince, EuRoC 2023 (M1101)** (`rocketpy-lince`): weather at 2023-10-12T10:00Z; inputs from RocketPy 1.13.0's lince_flight_sim.ipynb:123-129 (site, date), :432-437 (rail), :609-619 (log); total impulse flown 5199.6 N s. Outside the target (drag between): the drag that meets the log lies between hpr's and the example's. On its own drag hpr flies above the log; on the example's (the team's table, eight points to Mach 1) it flies below it, as RocketPy's notebook does (3284 m simulated). The log's apogee, 3668.5 m, is itself 81.5 m (2.3%) above the flight card's 3587 m, which the notebook records as the official one.
