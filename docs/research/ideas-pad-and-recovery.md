# Ideas: pad, range and recovery hardware

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier. Every pad and
range output is an estimate with a range, never a go/no-go: the range safety officer (RSO) and the
safety code decide.

## Pad and range

- SOON: a flight card generator.
- SOON: a delay advisor, with the odds and how much to drill off.
- SOON: a waiver check as a bounded probability.
- SOON: the best launch hour of a day.
- SOON: wind weighting: rail tilt and heading to put the landing near a target.
- SOON: a weathercocking cone check (as RockSim Pro has).
- SOON: a range safety map, offline: the landing spread, chute failure included, against field
  edges, spectators, airspace and temporary flight restrictions (TFRs); an estimate, never a
  go/no-go.
- SOON: failure-branch impact energy per part against the 14 J level of concern, with base rates
  from the National Association of Rocketry's (NAR's)
  ["Launching Safely in the 21st Century"](https://assets.zyrosite.com/Yg21M7RM7MSaQpNz/launchsafe-TQxWMfjtrNVQrGeU.pdf)
  (6,169 flights, 8.5% failures, 17.4% for complex flights), the source of the 14 J figure.
- SOON: a launch-site library ([OpenRocket #2177](https://github.com/openrocket/openrocket/issues/2177)).
- SOON: strip location data from shared files ([OpenRocket #3323](https://github.com/openrocket/openrocket/issues/3323)).
- LATER: live landing prediction from tracker telemetry (as SondeHub does for weather balloons).
- LATER: augmented reality at the pad.
- LATER: a phone-video flight tracer.
- SOON: a GPS blackout predictor: the COCOM limits (export rules that make a GPS receiver stop
  above 515 m/s and 18 km; some receivers stop at either).
- SOON: radio link budget, and line of sight at the landing spot.
- SOON: daylight left at the predicted landing time.
- LATER: chase mode.
- SOON: input from a handheld weather meter or a phone's barometer.
- SOON: a wind polar chart.
- SOON (next to queued work): Monte Carlo over forecast ensembles (as RocketPy's Ensemble does).
- SOON: best-hour re-runs keyed by their inputs, so an unchanged forecast is not flown again.

## Recovery hardware

- SOON: parachute sizing for a target descent rate.
- SOON: shock cord loads.
- SOON: shear pin count and size.
- SOON: custom ejection charge sizing (Neer's own): black powder and CO2, per bay, corrected for
  altitude, with the shear pins; always "ground test first".
- SOON: altimeter vent-hole sizing from the avionics bay's volume ([OpenRocket #2442](https://github.com/openrocket/openrocket/issues/2442)).
- SOON: zipper risk (a shock cord tearing the airframe at deployment).
- SOON: whether the recovery gear packs into its bay.
- SOON: reefed and clustered parachutes.
- SOON: a library of vendors' parachute drag coefficients.
- SOON: parachute gore cutting patterns (DXF and SVG).
- SOON: single-separation dual deploy (Chute Release, tender descender, cable cutter).
- SOON: deployment shock loads per section.
