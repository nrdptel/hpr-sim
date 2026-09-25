# hpr against OpenRocket 24.12's flights of the public designs

Written by `cargo xtask ork-flights` ([M2.2d2][m2-2d2], hpr's flights against OpenRocket's; decision [ADR-069][adr-069]) from OpenRocket 24.12's calm-air flights in `validation/fixtures/ork/openrocket-flights.json` ([M2.2d1][m2-2d1]). OR is OpenRocket. Each metric is taken by the definition hpr holds for OpenRocket 24.12: the apogee is the highest point above the launch position; the largest speed is the peak speed (hpr's up to its apogee, since it flies no parachute); the margin, in calibres, is OpenRocket's stability column at its rod-clearance step, which hpr takes at that step's time and Mach number with the air along the axis. Differences (Δ) are hpr less OpenRocket. Motors are OpenRocket's configuration names, one bracketed group per stage. The explanation is on the [documentation site][site].

[m2-2d2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d2
[m2-2d1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2d1
[m2-2e1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e1
[adr-069]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-069-hprs-flights-of-the-public-designs-against-openrockets-2026-09-25
[adr-070]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-070-m22e-split-mass-and-centre-of-mass-first-then-the-corpus-2026-09-25
[site]: https://nrdptel.github.io/hpr-sim/format/ork.html#hprs-flights-against-openrockets

- configurations flown: 21 (36 the record holds are not flown by hpr); apogee more than 5% from OpenRocket's: 5
- fastest: Dual parachute deployment [J570W-P], OpenRocket's largest Mach number 1.147
- apogee, a part's drag override not applied: 3 scored, median -16.77%, mean absolute 17.12%, from -19.13% to -15.47%
- apogee, no named cause: 12 scored, median -1.46%, mean absolute 1.65%, from -4.34% to -0.06%
- apogee, reference parachute open before apogee: 6 scored, median +0.01%, mean absolute 4.52%, from -0.64% to +13.80%
- largest speed, a part's drag override not applied: 3 scored, median -9.55%, mean absolute 8.52%, from -12.33% to -3.69%
- largest speed, no named cause: 18 scored, median +0.17%, mean absolute 0.39%, from -0.69% to +0.85%
- margin at rod clearance, no named cause: 21 scored, median -0.0007 cal, mean absolute 0.0045 cal, from -0.0151 cal to +0.0037 cal
- apogee against OpenRocket's own flight with the named causes removed, over the flights with a named cause: 9 scored, median -0.16%, mean absolute 1.94%, from -1.07% to +7.80%
- apogees more than 5% off: 5, 5 with their named causes sized; within 5% of every flight of OpenRocket's without the causes: 4 of 5

hpr's mass and centre of mass less OpenRocket's ([M2.2e1][m2-2e1], decision [ADR-070][adr-070]), over the flights not aborted: the masses in per cent of OpenRocket's, the centre of mass in OpenRocket's calibres, positive when hpr's is further aft, which shortens the margin by as much.

- mass at launch: 21 compared, median +0.016%, mean absolute 0.039%, from +0.000% to +0.209%
- mass at rod clearance: 21 compared, median +0.014%, mean absolute 0.037%, from -0.010% to +0.214%
- centre of mass at rod clearance: 21 compared, median +0.0007 cal, mean absolute 0.0046 cal, from -0.0028 cal to +0.0159 cal

| design | motors | apogee OR (m) | hpr (m) | Δ | max speed OR (m/s) | hpr (m/s) | Δ | max Mach OR | margin OR (cal) | hpr (cal) | Δ (cal) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3D printable nose cone and fins | [A8-3] | 39.0 | 39.0 | -0.06% | 24.85 | 24.89 | +0.17% | 0.073 | 1.139 | 1.139 | -0.0006 |
| 3D printable nose cone and fins | [B6-4] | 112.9 | 112.5 | -0.32% (chute 0.10 s early) | 48.64 | 48.77 | +0.26% | 0.143 | 1.071 | 1.071 | -0.0006 |
| 3D printable nose cone and fins | [C6-3] | 244.5 | 274.3 | +12.19% (chute 2.59 s early) | 83.29 | 84.00 | +0.85% | 0.245 | 0.903 | 0.903 | -0.0003 |
| 3D printable nose cone and fins | [C6-5] | 274.0 | 274.3 | +0.11% (chute 0.59 s early) | 83.29 | 84.00 | +0.85% | 0.245 | 0.903 | 0.903 | -0.0003 |
| 3D printable nose cone and fins | [C6-7] | 274.7 | 274.3 | -0.16% | 83.29 | 84.00 | +0.85% | 0.245 | 0.903 | 0.903 | -0.0003 |
| A simple model rocket | [A8-3] | 51.1 | 50.9 | -0.27% | 29.34 | 29.39 | +0.18% | 0.086 | 2.982 | 2.981 | -0.0008 |
| A simple model rocket | [B4-4] | 136.4 | 135.5 | -0.64% (chute 0.37 s early) | 53.35 | 53.49 | +0.27% | 0.157 | 2.798 | 2.797 | -0.0008 |
| A simple model rocket | [C6-3] | 280.2 | 318.9 | +13.80% (chute 2.99 s early) | 95.43 | 96.18 | +0.79% | 0.281 | 2.492 | 2.492 | -0.0007 |
| A simple model rocket | [C6-5] | 319.1 | 318.9 | -0.08% (chute 0.99 s early) | 95.43 | 96.18 | +0.79% | 0.281 | 2.492 | 2.492 | -0.0007 |
| A simple model rocket | [C6-7] | 322.4 | 318.9 | -1.07% | 95.43 | 96.18 | +0.79% | 0.281 | 2.492 | 2.492 | -0.0007 |
| Base drag hack (short-wide) | [C11-5] | 98.8 | 82.2 | -16.77% (-0.29% without the part set to no drag) | 47.25 | 45.51 | -3.69% (+0.04% without the part set to no drag) | 0.139 | 1.044 | 1.048 | +0.0037 |
| Base drag hack (short-wide) | [D12-3] | 199.2 | 168.4 | -15.47% (chute 1.80 s early) (+9.61% without the part set to no drag) | 73.82 | 66.77 | -9.55% (+1.99% without the part set to no drag) | 0.217 | 1.012 | 1.016 | +0.0036 |
| Base drag hack (short-wide) | [E12-4] | 316.3 | 255.8 | -19.13% (chute 1.24 s early) (+7.44% without the part set to no drag) | 91.55 | 80.26 | -12.33% (+5.23% without the part set to no drag) | 0.269 | 0.996 | 1.000 | +0.0037 |
| Chute release | [G40W-7] | 310.0 | 306.6 | -1.10% | 72.18 | 72.06 | -0.17% | 0.212 | 5.521 | 5.520 | -0.0014 |
| Chute release | [G80T-10] | 490.3 | 481.3 | -1.83% | 106.40 | 106.23 | -0.16% | 0.313 | 5.480 | 5.481 | +0.0014 |
| Dual parachute deployment | [H669N-P] | 593.1 | 579.9 | -2.24% | 135.42 | 135.44 | +0.02% | 0.398 | 4.460 | 4.446 | -0.0134 |
| Dual parachute deployment | [H242T-P] | 698.4 | 684.9 | -1.93% | 140.98 | 141.07 | +0.06% | 0.415 | 4.317 | 4.306 | -0.0114 |
| Dual parachute deployment | [J570W-P] | 2224.7 | 2128.2 | -4.34% | 388.59 | 385.90 | -0.69% | 1.147 | 3.276 | 3.266 | -0.0097 |
| Dual parachute deployment | [H999N-P] | 897.8 | 871.7 | -2.91% | 190.83 | 190.91 | +0.04% | 0.561 | 4.206 | 4.193 | -0.0133 |
| Dual parachute deployment | [I1299N-P] | 1159.0 | 1120.7 | -3.30% | 242.76 | 242.89 | +0.05% | 0.714 | 3.934 | 3.921 | -0.0130 |
| Dual parachute deployment | [G64W-P] | 227.4 | 226.0 | -0.63% | 58.54 | 58.48 | -0.11% | 0.172 | 4.877 | 4.862 | -0.0151 |

*Chute s early*: OpenRocket's parachute opened that long before the apogee of the same flight with nothing deployed, which the record also holds; hpr flies no parachute from a `.ork` yet. *Without the part set to no drag*: the same flight by hpr with the parts OpenRocket is told have no drag removed, which takes their mass, lift and shape away too, so it is a probe, not the override ([#165][i165]).

[i165]: https://github.com/nrdptel/hpr-sim/issues/165

The named causes, sized ([M2.2e4][m2-2e4], decision [ADR-073][adr-073]). OpenRocket flew each flight with a named cause again without it. *Nothing deployed*: the same flight with no parachute opening. *Drag settings cleared too*: also with every part's stated drag coefficient cleared, so each part has the drag of its shape, as in hpr, which reads the setting but can't apply it yet. *The parts removed*: nothing deployed and the parts set to no drag taken off, in OpenRocket and in hpr alike, which also takes away their lift. Each Δ is hpr's apogee less that flight's, in per cent of it. *Within 5% after*, for an apogee more than 5% off: whether every one of these flights with all its causes taken out is within 5% of hpr's.

| design | motors | Δ apogee | chute early (s) | OR, nothing deployed (m) | Δ | OR, drag settings cleared too (m) | Δ | OR, the parts removed (m) | hpr, the parts removed (m) | Δ | within 5% after |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 3D printable nose cone and fins | [B6-4] | -0.32% | 0.10 | 112.9 | -0.32% | — | — | — | — | — | — |
| 3D printable nose cone and fins | [C6-3] | +12.19% | 2.59 | 274.7 | -0.16% | — | — | — | — | — | yes |
| 3D printable nose cone and fins | [C6-5] | +0.11% | 0.59 | 274.7 | -0.16% | — | — | — | — | — | — |
| A simple model rocket | [B4-4] | -0.64% | 0.37 | 136.6 | -0.78% | — | — | — | — | — | — |
| A simple model rocket | [C6-3] | +13.80% | 2.99 | 322.4 | -1.07% | — | — | — | — | — | yes |
| A simple model rocket | [C6-5] | -0.08% | 0.99 | 322.4 | -1.07% | — | — | — | — | — | — |
| Base drag hack (short-wide) | [C11-5] | -16.77% | — | 98.8 | -16.77% | 81.1 | +1.34% | 97.4 | 98.5 | +1.15% | yes |
| Base drag hack (short-wide) | [D12-3] | -15.47% | 1.80 | 212.6 | -20.80% | 162.7 | +3.51% | 208.4 | 218.4 | +4.76% | yes |
| Base drag hack (short-wide) | [E12-4] | -19.13% | 1.24 | 321.8 | -20.50% | 244.1 | +4.79% | 315.3 | 339.8 | +7.80% | no |

[m2-2e4]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e4
[adr-073]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-073-each-named-cause-sized-by-openrockets-own-flight-without-it-2026-09-25

At the rod-clearance step, the parts of the margin (m from the nose tip, and kg):

| design | motors | CG OR | CG hpr | CP OR | CP hpr | reference OR | reference hpr | mass OR | mass hpr |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 3D printable nose cone and fins | [A8-3] | 0.2323 | 0.2323 | 0.2605 | 0.2605 | 0.0248 | 0.0248 | 0.0721 | 0.0721 |
| 3D printable nose cone and fins | [B6-4] | 0.2340 | 0.2340 | 0.2605 | 0.2605 | 0.0248 | 0.0248 | 0.0739 | 0.0739 |
| 3D printable nose cone and fins | [C6-3] | 0.2381 | 0.2381 | 0.2605 | 0.2605 | 0.0248 | 0.0248 | 0.0787 | 0.0788 |
| 3D printable nose cone and fins | [C6-5] | 0.2381 | 0.2381 | 0.2605 | 0.2605 | 0.0248 | 0.0248 | 0.0787 | 0.0788 |
| 3D printable nose cone and fins | [C6-7] | 0.2381 | 0.2381 | 0.2605 | 0.2605 | 0.0248 | 0.0248 | 0.0787 | 0.0788 |
| A simple model rocket | [A8-3] | 0.2448 | 0.2448 | 0.3193 | 0.3193 | 0.0250 | 0.0250 | 0.0629 | 0.0629 |
| A simple model rocket | [B4-4] | 0.2494 | 0.2494 | 0.3193 | 0.3193 | 0.0250 | 0.0250 | 0.0653 | 0.0653 |
| A simple model rocket | [C6-3] | 0.2570 | 0.2571 | 0.3193 | 0.3193 | 0.0250 | 0.0250 | 0.0696 | 0.0696 |
| A simple model rocket | [C6-5] | 0.2570 | 0.2571 | 0.3193 | 0.3193 | 0.0250 | 0.0250 | 0.0696 | 0.0696 |
| A simple model rocket | [C6-7] | 0.2570 | 0.2571 | 0.3193 | 0.3193 | 0.0250 | 0.0250 | 0.0696 | 0.0696 |
| Base drag hack (short-wide) | [C11-5] | 0.0841 | 0.0839 | 0.1663 | 0.1663 | 0.0787 | 0.0787 | 0.1521 | 0.1525 |
| Base drag hack (short-wide) | [D12-3] | 0.0866 | 0.0864 | 0.1663 | 0.1663 | 0.0787 | 0.0787 | 0.1592 | 0.1595 |
| Base drag hack (short-wide) | [E12-4] | 0.0878 | 0.0876 | 0.1663 | 0.1663 | 0.0787 | 0.0787 | 0.1751 | 0.1754 |
| Chute release | [G40W-7] | 0.4821 | 0.4822 | 0.8327 | 0.8327 | 0.0635 | 0.0635 | 1.0215 | 1.0217 |
| Chute release | [G80T-10] | 0.4848 | 0.4847 | 0.8328 | 0.8328 | 0.0635 | 0.0635 | 1.0266 | 1.0269 |
| Dual parachute deployment | [H669N-P] | 0.9513 | 0.9521 | 1.2039 | 1.2039 | 0.0566 | 0.0566 | 1.5751 | 1.5750 |
| Dual parachute deployment | [H242T-P] | 0.9592 | 0.9599 | 1.2037 | 1.2037 | 0.0566 | 0.0566 | 1.6019 | 1.6019 |
| Dual parachute deployment | [J570W-P] | 1.0184 | 1.0190 | 1.2039 | 1.2040 | 0.0566 | 0.0566 | 2.1821 | 2.1818 |
| Dual parachute deployment | [H999N-P] | 0.9658 | 0.9666 | 1.2040 | 1.2041 | 0.0566 | 0.0566 | 1.6422 | 1.6421 |
| Dual parachute deployment | [I1299N-P] | 0.9813 | 0.9821 | 1.2041 | 1.2042 | 0.0566 | 0.0566 | 1.7253 | 1.7253 |
| Dual parachute deployment | [G64W-P] | 0.9274 | 0.9283 | 1.2037 | 1.2037 | 0.0566 | 0.0566 | 1.4940 | 1.4940 |

What hpr's design checks object to, in flights flown anyway as OpenRocket flies them:

| design | motors | findings |
|---|---|---|
| 3D printable nose cone and fins | [A8-3] | `[{"component":"bc822f1b-b988-4a96-ba8e-34d472b8c9d4","kind":"internal_part_wider_than_parent","parent":"2cdfe954-7c39-4fd6-bde3-471feb6ee0b3","reach_m":0.012052299999999998,"room_m":0.011595199999999998}]` |
| 3D printable nose cone and fins | [B6-4] | `[{"component":"bc822f1b-b988-4a96-ba8e-34d472b8c9d4","kind":"internal_part_wider_than_parent","parent":"2cdfe954-7c39-4fd6-bde3-471feb6ee0b3","reach_m":0.012052299999999998,"room_m":0.011595199999999998}]` |
| 3D printable nose cone and fins | [C6-3] | `[{"component":"bc822f1b-b988-4a96-ba8e-34d472b8c9d4","kind":"internal_part_wider_than_parent","parent":"2cdfe954-7c39-4fd6-bde3-471feb6ee0b3","reach_m":0.012052299999999998,"room_m":0.011595199999999998}]` |
| 3D printable nose cone and fins | [C6-5] | `[{"component":"bc822f1b-b988-4a96-ba8e-34d472b8c9d4","kind":"internal_part_wider_than_parent","parent":"2cdfe954-7c39-4fd6-bde3-471feb6ee0b3","reach_m":0.012052299999999998,"room_m":0.011595199999999998}]` |
| 3D printable nose cone and fins | [C6-7] | `[{"component":"bc822f1b-b988-4a96-ba8e-34d472b8c9d4","kind":"internal_part_wider_than_parent","parent":"2cdfe954-7c39-4fd6-bde3-471feb6ee0b3","reach_m":0.012052299999999998,"room_m":0.011595199999999998}]` |
| Chute release | [G40W-7] | `[{"configuration":"95aac292-c018-4713-8660-bd1a4714a0c2","kind":"motor_wider_than_mount","motor_diameter_m":0.029,"mount":"d6876c66-480e-469a-9a56-91123e427a86","mount_inner_diameter_m":0.028955999999999996},{"configuration":"3c6373fb-a84b-4dd3-8b84-bfc96c7233d5","kind":"motor_wider_than_mount","motor_diameter_m":0.029,"mount":"d6876c66-480e-469a-9a56-91123e427a86","mount_inner_diameter_m":0.028955999999999996}]` |
| Chute release | [G80T-10] | `[{"configuration":"95aac292-c018-4713-8660-bd1a4714a0c2","kind":"motor_wider_than_mount","motor_diameter_m":0.029,"mount":"d6876c66-480e-469a-9a56-91123e427a86","mount_inner_diameter_m":0.028955999999999996},{"configuration":"3c6373fb-a84b-4dd3-8b84-bfc96c7233d5","kind":"motor_wider_than_mount","motor_diameter_m":0.029,"mount":"d6876c66-480e-469a-9a56-91123e427a86","mount_inner_diameter_m":0.028955999999999996}]` |

Configurations OpenRocket flew that hpr does not fly yet:

| design | motors | why |
|---|---|---|
| demo-payload-separation | [None; F50T-8] | the design's only configuration, which OpenRocket gave a new id: a motor with no curve |
| ARC payload rocket | [None; F50T-9] | more than one stage |
| Airstart timing | [3× I211W-P, K550W-P] | a motor in a cluster |
| Airstart timing | Airstart @2s | a motor in a cluster |
| Airstart timing | Airstart @1s | a motor in a cluster |
| Airstart timing | airstart @4s | a motor in a cluster |
| Airstart timing | airstart @6s | a motor in a cluster |
| Clustered motors | [4× A8-3] | a motor in a cluster |
| Clustered motors | [4× B4-4] | a motor in a cluster |
| Clustered motors | [4× C6-3] | a motor in a cluster |
| Clustered motors | [4× C6-5] | a motor in a cluster |
| Clustered motors | [4× C6-7] | a motor in a cluster |
| Deployable payload | [None; A8-3] | more than one stage |
| Deployable payload | [None; B4-4] | more than one stage |
| Deployable payload | [None; C6-3] | more than one stage |
| Deployable payload | [None; C6-5] | more than one stage |
| Deployable payload | [None; C6-7] | more than one stage |
| Parallel booster staging | [I115W-10; 2× E12-0] | a motor in a part not read |
| Parallel booster staging | [I115W-10; None] | an airframe not read exactly as written |
| Pods--airframes and winglets | [A8-3] | an airframe not read exactly as written |
| Pods--airframes and winglets | [B6-4] | an airframe not read exactly as written |
| Pods--airframes and winglets | [C6-5] | an airframe not read exactly as written |
| Pods--airframes and winglets | [C12-6] | an airframe not read exactly as written |
| Pods--airframes and winglets | [D16-6] | an airframe not read exactly as written |
| Pods--powered with recovery deployment | [C6-7; 2× A3-4, B6-0] | a motor in a part not read |
| Pods--powered with recovery deployment | [C6-7; B6-0] | a motor igniting in flight |
| Pods--powered with recovery deployment | [C6-5; None] | an airframe not read exactly as written |
| Pods--powered with recovery deployment | [None; 2× A10-3, B6-4] | a motor in a part not read |
| Simulation extensions | [2800CC172L-L540-P] | a motor with no curve |
| Simulation scripting | [2800CC172L-L540-P] | a motor with no curve |
| Three stage low power rocket | [A8-5; B6-0; B6-0] | a motor igniting in flight |
| Three stage low power rocket | [C6-5; B6-0; B6-0] | a motor igniting in flight |
| Three stage low power rocket | [C6-7; C6-0; C6-0] | a motor igniting in flight |
| Tube fin rocket | [D12-7] | an airframe not read exactly as written |
| Two stage high power rocket | [H148R-0; H148R-0] | a motor igniting in flight |
| Two stage high power rocket | [I59WN-P; I357T-14] | a motor igniting in flight |
