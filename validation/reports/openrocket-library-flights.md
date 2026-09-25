# hpr against OpenRocket 24.12's flights of the private design library

This report compares hpr's flights of 4 of the library's 12 private designs with OpenRocket 24.12's, and publishes only the differences. It is a code-to-code comparison with no target: agreeing with OpenRocket is not agreeing with a real flight. hpr's stability margin is larger than OpenRocket's by up to 0.0730 calibres, so it can call a rocket more stable than OpenRocket does ([#172](https://github.com/nrdptel/hpr-sim/issues/172)). With the [public report](openrocket-flights.md) it covers 9 designs of the 20 that [M2.2][m2-2] (the OpenRocket comparison) asks for: not met. Nobody without the private library can fly these again; CI checks only that this report adds up and names nothing of a design.

- Written by `cargo xtask ork-flights --library` ([M2.2e3][m2-2e3], hpr's flights of the library; decision [ADR-072][adr-072]) from OpenRocket's flights of it ([M2.2e2][m2-2e2]), flown and compared as in the public report, by the same definitions. The explanation is on the [documentation site][site].
- A design is an id, `C01` onwards, in the order of its file's SHA-256 hash, and a flight is the design's id and the configuration's place in the file: `C09/2` is the second configuration of `C09`. The ids hold while the library's files do; `ids_sha256` in the report's JSON changes when they would move.
- Differences (Δ) are hpr less OpenRocket: apogee, largest speed and mass in per cent of OpenRocket's; the stability margin, the centre of mass (CG) and the centre of pressure (CP) at rod clearance in OpenRocket's calibres (its reference diameter), the CG and CP positive when hpr's is further aft. A positive margin Δ means hpr calls the rocket more stable; margin Δ = CP Δ − CG Δ when the reference diameters agree. The *five spreads* are apogee, largest speed, margin, mass and CG.
- A flight is *scored* in a spread when both programs have the number; a *named cause* is a known difference in how the two flights were set up that can move a number, and flights with one get their own summary line, for that spread only. The *reference parachute* is OpenRocket's.
- *Mach* is the class of OpenRocket's largest Mach number: subsonic below 0.8, transonic to 1.2, supersonic above. *Site* is whether the launch site is at sea level.
- No value of a design is written: with its difference it would give back OpenRocket's. The JSON holds differences to 6 decimals (masses to 3), so no rounding residue gives a value back, and the tables here show fewer; the one time given is how early OpenRocket's parachute opened, to 2 decimals of a second.

[m2-2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2
[m2-2e3]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e3
[m2-2e2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e2
[adr-072]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-072-hprs-flights-of-the-private-library-under-anonymised-ids-2026-09-25
[site]: https://nrdptel.github.io/hpr-sim/format/ork.html#hprs-flights-of-the-private-designs

- designs: 27 files in the library; 0 OpenRocket did not open, 15 public designs (the same file, or an edited copy of the same rocket) left to the public report, 0 found twice; 12 private designs, 4 with a flight compared in all five spreads
- with the public report's 5, 9 designs in all, against [M2.2][m2-2]'s bar of 20 (not met)
- configurations flown: 17 (20 OpenRocket flew are not flown by hpr); apogee more than 5% from OpenRocket's: 0; the same reference diameter as OpenRocket's: 17
- apogee, no named cause: 14 scored, median -1.95%, mean absolute 2.25%, from -4.84% to +1.17%
- apogee, reference parachute open before apogee: 3 scored, median -2.80%, mean absolute 2.19%, from -3.09% to +0.68%
- largest speed, no named cause: 17 scored, median -0.01%, mean absolute 0.49%, from -0.65% to +2.28%
- margin at rod clearance, no named cause: 17 scored, median +0.0444 cal, mean absolute 0.0407 cal, from -0.0008 cal to +0.0730 cal
- mass at launch: 17 compared, median +0.000%, mean absolute 0.001%, from +0.000% to +0.004%
- mass at rod clearance: 17 compared, median +0.004%, mean absolute 0.013%, from -0.007% to +0.065%
- centre of mass at rod clearance: 17 compared, median -0.0145 cal, mean absolute 0.0128 cal, from -0.0273 cal to +0.0042 cal

| flight | Mach | site | apogee Δ | max speed Δ | margin Δ (cal) | CG Δ (cal) | CP Δ (cal) | launch mass Δ | rod-clearance mass Δ | named cause |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| C03/1 | subsonic | above sea level | -2.00% | +2.28% | +0.0582 | +0.0025 | +0.0607 | +0.003% | +0.057% |  |
| C03/2 | transonic | above sea level | -4.09% | +1.00% | +0.0564 | +0.0042 | +0.0605 | +0.002% | +0.065% |  |
| C03/3 | subsonic | above sea level | -3.09% | -0.30% | +0.0604 | +0.0002 | +0.0607 | +0.004% | +0.004% | chute 0.08 s early |
| C03/4 | subsonic | above sea level | -2.80% | -0.13% | +0.0605 | +0.0002 | +0.0607 | +0.004% | +0.003% | chute 0.40 s early |
| C03/5 | transonic | above sea level | -4.83% | -0.65% | +0.0730 | -0.0124 | +0.0606 | +0.002% | +0.009% |  |
| C07/1 | subsonic | sea level | +0.68% | +1.06% | +0.0032 | -0.0005 | +0.0027 | +0.000% | -0.007% | chute 0.55 s early |
| C09/1 | subsonic | above sea level | -0.49% | -0.06% | +0.0455 | -0.0249 | +0.0205 | +0.000% | +0.000% |  |
| C09/2 | subsonic | above sea level | -0.18% | -0.01% | +0.0446 | -0.0240 | +0.0206 | +0.000% | +0.000% |  |
| C09/3 | subsonic | above sea level | -0.25% | -0.06% | +0.0441 | -0.0236 | +0.0206 | +0.000% | -0.004% |  |
| C09/4 | subsonic | above sea level | -1.48% | -0.04% | +0.0444 | -0.0239 | +0.0205 | +0.000% | +0.001% |  |
| C09/5 | subsonic | above sea level | -1.90% | -0.03% | +0.0350 | -0.0145 | +0.0205 | +0.000% | +0.001% |  |
| C09/6 | subsonic | above sea level | -2.57% | -0.07% | +0.0478 | -0.0273 | +0.0205 | +0.000% | +0.007% |  |
| C09/7 | subsonic | above sea level | -3.21% | +0.02% | +0.0402 | -0.0197 | +0.0205 | +0.000% | +0.011% |  |
| C09/8 | subsonic | above sea level | -3.54% | +0.28% | +0.0397 | -0.0193 | +0.0204 | +0.000% | +0.007% |  |
| C09/9 | transonic | above sea level | -4.84% | +0.26% | +0.0385 | -0.0183 | +0.0202 | +0.000% | +0.003% |  |
| C11/2 | transonic | sea level | +1.17% | +1.22% | +0.0002 | +0.0004 | +0.0007 | +0.000% | +0.008% |  |
| C11/3 | transonic | sea level | +0.94% | +0.94% | -0.0008 | +0.0015 | +0.0007 | +0.000% | +0.027% |  |

*Chute s early*: OpenRocket's parachute opened that long before the apogee of the same flight with nothing deployed; hpr flies no parachute from a `.ork` yet. An early parachute lowers OpenRocket's apogee, so it can explain an apogee Δ above zero, not one below.

What hpr's [design checks](https://nrdptel.github.io/hpr-sim/physics/design.html#checks) object to, by kind, in flights flown anyway as OpenRocket flies them:

| flight | findings |
|---|---|
| C03/1 | `internal_part_wider_than_parent` |
| C03/2 | `internal_part_wider_than_parent` |
| C03/3 | `internal_part_wider_than_parent` |
| C03/4 | `internal_part_wider_than_parent` |
| C03/5 | `internal_part_wider_than_parent` |
| C07/1 | `internal_part_wider_than_parent` |
| C11/2 | `internal_part_wider_than_parent` |
| C11/3 | `internal_part_wider_than_parent` |

Configurations OpenRocket flew that hpr does not fly here, by reason:

| why | how many | flights |
|---|---:|---|
| a curve found by name, which OpenRocket may not fly | 1 | C11/4 |
| a launch rod not vertical | 3 | C12/1, C12/2, C12/3 |
| a motor igniting in flight | 2 | C04/1, C08/1 |
| an airframe not read exactly as written | 14 | C01/1, C01/2, C02/1, C02/2, C02/3, C02/4, C02/5, C05/1, C05/2, C05/3, C05/4, C05/5, C06/1, C10/1 |
