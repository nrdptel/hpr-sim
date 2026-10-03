# Ideas: flight data

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier. All are SOON,
with the flight-log milestones [M7.1][m7-1] to [M7.4][m7-4] (reading logs, readings and
reconstruction, a flight against its simulation, fault diagnosis).

- SOON: a log-health card, in the style of PX4's Flight Review (PX4 is an open-source drone
  autopilot; Flight Review is its web log checker).
- SOON: spectrograms (motor chuffing, flutter, roll resonance).
- SOON: measured stability from the gyro's pitch oscillation.
- SOON: system identification of the normal-force slope, pitch and roll damping, with error bars.
- SOON: drag fitted separately for the burn and the coast.
- SOON: drag from apogee-only altimeters ([arXiv 2512.22248](https://arxiv.org/abs/2512.22248)).
- SOON: wind estimated during the ascent
  ([MDPI *Atmosphere* 12(4):470](https://www.mdpi.com/2073-4433/12/4/470)) and from the GPS
  descent.
- SOON: the thrust curve recovered from the inertial measurement unit (IMU) against the certified
  curve, and the apogee spread across a motor's curve files.
- SOON: sensor calibration across a season.
- SOON: video-to-log sync by sound, and a telemetry overlay.
- SOON: event-aligned delta traces between flights.
- SOON: a multi-flight workspace, in the style of MoTeC (motorsport data-analysis software).
- SOON: record-ready KML.
- SOON: AltosUI CSV and Blue Raven text import (documented formats only; AltOS is GPL).
- SOON: motor failure counts from MESS, NAR's Malfunctioning Engine Statistical Survey (cache
  only; the licence is unclear).
- SOON: incident report pre-fill.
- SOON: a season flight recorder with NAR's (the National Association of Rocketry's) outcome codes.
- SOON: a "flight package" archive format.

[m7-1]: ../decisions-and-roadmap.md#m7-1
[m7-4]: ../decisions-and-roadmap.md#m7-4
