# hpr against the logs of real flights

Written by `cargo xtask real-flights` ([M2.3b][m2-3b], decision [ADR-082][adr-082]). How it is done, and what it means, is on the [documentation site][site].

- **What is flown:** 7 of the rockets RocketPy's documentation flies against their teams' altitude logs. hpr flies each with its own aerodynamics, on the example's own thrust file, from the example's rail and site, in the ERA5 weather the example reads. The log is the reference.
- **Heights:** a barometric altimeter reads the standard atmosphere's altitude of the pressure it measures, less the pad's. hpr's height is read the same way, from the ERA5 pressure at its centre of mass, less the start's. Each log is read up to its apogee, stopping before the recovery's pressure transients.
- **Trace RMS:** over the ascent, both clocks aligned where the trace first reaches 30 m, until the first of the two apogees.
- **Files:** the logs, thrust files and weather files are read from the pinned RocketPy checkout and never committed; each one's SHA-256 is in the JSON.

[m2-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-3b
[adr-082]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-082-real-flights-read-from-refs-compared-over-the-ascent-with-checked-explanations-2026-09-26
[site]: https://nrdptel.github.io/hpr-sim/accuracy.html#real-flights

## Summary

- Flights: 7.
- Mean absolute apogee error: 6.04% against a target of 5%: outside target.
- Mean apogee error with its sign: -0.25%.
- Mean absolute apogee error were every log a height, not a barometric reading: 4.47%.
- Mean absolute apogee error with only the logs known to be barometric read so, the assumed ones as heights: 6.63%.
- Largest trace RMS: 7.26% of its log's apogee.
- Outside the target: ndrt-2020, prometheus-2022, juno-iii, cavour, genesis.
- On each example's own drag, the diagnostic flight: mean absolute apogee error 3.50%.

## Flights

| flight | altimeter | log apogee (m) | hpr apogee (m) | apogee error | hpr apogee as a height (m) | log time to apogee (s) | hpr time to apogee (s) | trace RMS (m) | trace RMS (% of apogee) | rows | hpr's largest Mach |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Bella Lui, EPFL Rocket Team, 2020 (K828FJ) | barometric, assumed | 459.0 | 462.9 | +0.86% | 463.6 | 9.46 | 9.48 | 3.2 | 0.70% | 161 | 0.264 |
| NDRT 2020, Notre Dame Rocketry Team (L1395) | barometric | 1320.4 | 1457.6 | +10.40% | 1432.8 | 15.76 | 16.71 | 89.9 | 6.81% | 316 | 0.536 |
| Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520) | barometric | 3895.8 | 3549.3 | -8.90% | 3809.9 | 28.44 | 26.55 | 190.9 | 4.90% | 2875 | 0.959 |
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | barometric | 3151.5 | 2923.4 | -7.24% | 3174.5 | 23.62 | 25.48 | 228.8 | 7.26% | 473 | 0.733 |
| Cavour, Politecnico di Torino, EuRoC 2023 (L995) | barometric, assumed | 2789.0 | 2946.0 | +5.63% | 3052.1 | 23.07 | 22.68 | 125.3 | 4.49% | 122 | 0.933 |
| Genesis, EuRoC 2023 (L995) | barometric, assumed | 2916.7 | 2746.0 | -5.85% | 2875.3 | 23.33 | 22.45 | 95.2 | 3.26% | 2244 | 0.854 |
| Lince, EuRoC 2023 (M1101) | barometric, assumed | 3587.7 | 3709.2 | +3.39% | 3874.3 | 25.25 | 26.40 | 63.9 | 1.78% | 2526 | 0.993 |

## On each example's own drag

A diagnostic, not a prediction: the same flight with hpr's zero-lift drag replaced by the drag the example's notebook specifies (a team's estimate: a table, a curve from RASAero II or CFD, or a constant), on the example's radius. hpr's normal force is its own in both. Where the error falls within the target, the miss is consistent with hpr's drag; where it doesn't, it is elsewhere. The teams' drags are estimates too, and one may have been tuned to its flight.

| flight | example's drag | apogee (m) | apogee error | trace RMS (m) |
|---|---|---:|---:|---:|
| Bella Lui, EPFL Rocket Team, 2020 (K828FJ) | bella_lui_flight_sim.ipynb:94-95, :453-484 (power off and on, times 1) | 460.1 | +0.24% | 2.0 |
| NDRT 2020, Notre Dame Rocketry Team (L1395) | ndrt_2020_flight_sim.ipynb:91 and :316-317 (the drag coefficient, power off and on) | 1322.6 | +0.17% | 31.7 |
| Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520) | prometheus_2022_flight_sim.ipynb:224-261 (`prometheus_cd_at_ma` from Mach 0.15, where it starts to change, and power on 1.02 times it) | 3913.5 | +0.45% | 98.4 |
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | juno3_flight_sim.ipynb:244-251, :356-359 (`drag_curve.csv`, scaled to 0.38 at Mach 0.6 "from CFD analysis") | 2866.1 | -9.05% | 255.3 |
| Cavour, Politecnico di Torino, EuRoC 2023 (L995) | cavour_flight_sim.ipynb:250-251 | 2725.0 | -2.29% | 26.9 |
| Genesis, EuRoC 2023 (L995) | genesis_flight_sim.ipynb:241-242 | 2914.5 | -0.08% | 32.7 |
| Lince, EuRoC 2023 (M1101) | lince_flight_sim.ipynb:240-241 | 3149.9 | -12.20% | 279.9 |

## On the thrust file as recorded

A second diagnostic, where an example reshapes its thrust file to a burn time and a total impulse: the same flight on the file as recorded (negative points read as zero), on hpr's own drag. Where the error falls within the target and the example's drag doesn't bring it there, the miss is consistent with the impulse the example sets.

| flight | impulse as recorded (N s) | impulse reshaped (N s) | apogee (m) | apogee error |
|---|---:|---:|---:|---:|
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | 9251.7 | 8802.6 | 3187.2 | +1.13% |

## Against the logs' own pressure and satellite heights

Where a log keeps the pressure its height was read from, the height column's largest gap from the standard atmosphere's reading of that pressure shows how the altimeter reads. Where the flight logged a satellite (GNSS) altitude, its apogee above the pad is a geometric height: the log's barometric apogee over it is what the day's air did to the reading, beside what hpr's conversion does to its own apogee.

| flight | height against its pressure (largest gap, m) | satellite apogee (m) | log apogee over it | hpr's reading over its height |
|---|---:|---:|---:|---:|
| Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520) | 0.195 | 4133.0 | 0.943 | 0.932 |
| Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor) | 1.321 | 3369.3 | 0.935 | 0.921 |

## Each flight's inputs

- **Bella Lui, EPFL Rocket Team, 2020 (K828FJ)** (`rocketpy-bella-lui`): weather at 2020-02-22T13:00Z; inputs from RocketPy 1.13.0's bella_lui_flight_sim.ipynb:109-111 (rail), :139-153 (site, date), :761-765 (log); altimeter: the team's own avionics, filtered by a method the example doesn't record: barometric assumed, as hobby altimeters are; total impulse flown 2071.0 N s.
- **NDRT 2020, Notre Dame Rocketry Team (L1395)** (`rocketpy-ndrt-2020-nose-to-tail`): weather at 2020-02-23T16:00Z; inputs from RocketPy 1.13.0's ndrt_2020_flight_sim.ipynb:115-117 (rail), :148-153 (site, date), :679-683 (log); altimeter: a Featherweight Raven (RocketPy's tests/acceptance/test_ndrt_2020_rocket.py:190), a barometric altimeter, in feet above the pad; total impulse flown 4894.9 N s. Outside the target (drag): consistent with hpr's drag. On the example's own drag, a constant 0.44 the notebook gives no source for, the apogee is within the target. hpr's own drag is lower, as the predicted-mode comparison with RocketPy flying the same constant found (+10.3% in apogee), and the design's fin edges and finish are placeholders, since the example records none.
- **Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520)** (`rocketpy-prometheus-2022-generic-motor`): weather at 2023-06-24T15:00Z; inputs from RocketPy 1.13.0's prometheus_2022_flight_sim.ipynb:65-70 (site, date), :401-406 (rail), :508-526 (log); altimeter: an Altus Metrum TeleMetrum: its height column is the standard atmosphere's altitude of its pressure column less the first row's (the gap is below); total impulse flown 7579.1 N s. Note: flown in the weather of 24 June 2023, a year after the flight, as RocketPy's example flies it: RocketPy has no ERA5 file of the day. The log is read to 29.58 s: a pressure transient then drops the reading 600 m and returns it 8 m above the highest reading before. Outside the target (drag): consistent with hpr's drag. On the example's own drag, the team's table from Mach 0.15, the apogee is within the target. The reading is built on weather a year off the flight's day (the note). Against the satellite heights, hpr's conversion reads 1 to 2 points below the altimeter here and on Juno III, which flew in its own day's weather, so the wrong day's share of the miss can't be told apart.
- **Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor)** (`rocketpy-juno-iii`): weather at 2023-06-23T23:00Z; inputs from RocketPy 1.13.0's juno3_flight_sim.ipynb:54-67 (site, date), :448-456 (rail), :543-553 (log); altimeter: a Missile Works RRC3 (juno3/README.txt:21): its height column is the standard atmosphere's altitude of its pressure column less the first row's (the gap is below); total impulse flown 8802.6 N s, 2.62 N s of it from reading the file's negative thrust as zero. Note: the log is read to 24.60 s, as it levels off: a pressure transient there dips the reading by 94 m, then lifts it 62 m above the level within 0.3 s, to the 3213.4 m the team reports as its apogee, and the record's last two rows are corrupt. At the cut the log's own velocity column still reads 17.5 m/s up, so its apogee may be 10 m to 20 m low. The thrust file's last five points are negative (-6.8 to -47.3 N); hpr, which refuses a negative thrust, reads them as zero, and RocketPy's flight holds its thrust at zero too. Outside the target (thrust): consistent with the motor's impulse. The notebook reshapes the team's own motor curve (`mandioca_thrust_curve.csv`) to 5.8 s and 8800 N s, 4.9% less than the file; on the file as recorded the apogee is within the target, and on the team's drag it is not.
- **Cavour, Politecnico di Torino, EuRoC 2023 (L995)** (`rocketpy-cavour`): weather at 2023-10-13T12:00Z; inputs from RocketPy 1.13.0's cavour_flight_sim.ipynb:125-132 (site, date), :314-315 (rail), :379-389 (log); altimeter: the CATS Vega EuRoC 2023 required, sent by radio in whole metres: a Kalman filter's estimate from a barometer and an accelerometer, barometric assumed; total impulse flown 3618.0 N s. Outside the target (drag): consistent with hpr's drag. On the example's own curves, labelled RASAero II, the apogee is within the target. hpr's drag is below them: at Mach 0.3, 8.3% below power off and 18.3% below power on (the aerodynamics page's comparison; the power-on gap's cause is open), and the design's fin edges are placeholders, since the example records none.
- **Genesis, EuRoC 2023 (L995)** (`rocketpy-genesis`): weather at 2023-10-12T13:00Z; inputs from RocketPy 1.13.0's genesis_flight_sim.ipynb:124-131 (site, date), :326-327 (rail), :391-406 (log); altimeter: a filtered estimate (`filtered_altitude_AGL`), probably the CATS Vega's barometer and accelerometer Kalman filter: barometric assumed; total impulse flown 3618.0 N s. Outside the target (drag): consistent with hpr's drag. On the example's own curves the apogee is within the target; the design's fin edges and finish are placeholders, since the example records none.
- **Lince, EuRoC 2023 (M1101)** (`rocketpy-lince`): weather at 2023-10-12T10:00Z; inputs from RocketPy 1.13.0's lince_flight_sim.ipynb:123-129 (site, date), :432-437 (rail), :609-619 (log); altimeter: a filtered estimate (`filtered_altitude_AGL`, as Genesis's) from a computer the example doesn't name: barometric assumed; total impulse flown 5199.6 N s. Note: the log is read to 26.80 s: past it, as the recovery fires, the filtered height swings by hundreds of metres, up to 3668.5 m; its highest reading before is the 3587 m the team reports as its apogee.
