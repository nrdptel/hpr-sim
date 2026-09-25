# hpr against OpenRocket 24.12's flights of the public designs

Written by `cargo xtask ork-flights` (milestone M2.2d2, ADR-069) from OpenRocket's calm-air flights in `validation/fixtures/ork/openrocket-flights.json` (M2.2d1, ADR-068). Each metric is taken by the definition hpr holds for OpenRocket 24.12: the apogee and largest speed are peaks over the flight, and the margin is OpenRocket's stability column at its rod-clearance step, which hpr takes at that step's time and Mach number with the air along the axis. Differences are hpr less OpenRocket. hpr flies no recovery device from a `.ork` yet, so a flight whose reference parachute opened before its apogee is marked and summarised apart.

- configurations flown: 21 (36 the record holds are not flown by hpr); apogee more than 5% from OpenRocket's: 5
- apogee, a part's drag override not applied: 3 scored, median -16.43%, mean size 16.92%, from -19.03% to -15.31%
- apogee, no named cause: 12 scored, median -1.37%, mean size 1.57%, from -4.32% to +0.21%
- apogee, reference parachute open before apogee: 6 scored, median +0.05%, mean size 4.50%, from -0.53% to +13.85%
- largest speed, a part's drag override not applied: 3 scored, median -9.55%, mean size 8.52%, from -12.33% to -3.69%
- largest speed, no named cause: 18 scored, median +0.17%, mean size 0.39%, from -0.69% to +0.85%
- margin at rod clearance, no named cause: 21 scored, median -0.001 cal, mean size 0.005 cal, from -0.015 cal to +0.004 cal

| design | motors | apogee OR (m) | hpr (m) | Δ | max speed OR (m/s) | hpr (m/s) | Δ | margin OR (cal) | hpr (cal) | Δ (cal) |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 3D printable nose cone and fins | [A8-3] | 39.0 | 39.1 | +0.21% | 24.85 | 24.89 | +0.17% | 1.139 | 1.139 | -0.001 |
| 3D printable nose cone and fins | [B6-4] | 112.9 | 112.6 | -0.23% (chute 0.06 s before) | 48.64 | 48.77 | +0.26% | 1.071 | 1.071 | -0.001 |
| 3D printable nose cone and fins | [C6-3] | 244.5 | 274.3 | +12.23% (chute 0.58 s before) | 83.29 | 84.00 | +0.85% | 0.903 | 0.903 | -0.000 |
| 3D printable nose cone and fins | [C6-5] | 274.0 | 274.3 | +0.14% (chute 0.37 s before) | 83.29 | 84.00 | +0.85% | 0.903 | 0.903 | -0.000 |
| 3D printable nose cone and fins | [C6-7] | 274.7 | 274.3 | -0.13% | 83.29 | 84.00 | +0.85% | 0.903 | 0.903 | -0.000 |
| A simple model rocket | [A8-3] | 51.1 | 51.1 | +0.05% | 29.34 | 29.39 | +0.18% | 2.982 | 2.981 | -0.001 |
| A simple model rocket | [B4-4] | 136.4 | 135.7 | -0.53% (chute 0.28 s before) | 53.35 | 53.49 | +0.27% | 2.798 | 2.797 | -0.001 |
| A simple model rocket | [C6-3] | 280.2 | 319.0 | +13.85% (chute 0.57 s before) | 95.43 | 96.18 | +0.79% | 2.492 | 2.492 | -0.001 |
| A simple model rocket | [C6-5] | 319.1 | 319.0 | -0.03% (chute 0.46 s before) | 95.43 | 96.18 | +0.79% | 2.492 | 2.492 | -0.001 |
| A simple model rocket | [C6-7] | 322.4 | 319.0 | -1.03% | 95.43 | 96.18 | +0.79% | 2.492 | 2.492 | -0.001 |
| Base drag hack (short-wide) | [C11-5] | 98.8 | 82.6 | -16.43% (-0.20% without the drag-overridden part) | 47.25 | 45.51 | -3.69% | 1.044 | 1.048 | +0.004 |
| Base drag hack (short-wide) | [D12-3] | 199.2 | 168.7 | -15.31% (chute 0.53 s before) (+9.66% without the drag-overridden part) | 73.82 | 66.77 | -9.55% | 1.012 | 1.016 | +0.004 |
| Base drag hack (short-wide) | [E12-4] | 316.3 | 256.1 | -19.03% (chute 0.50 s before) (+7.46% without the drag-overridden part) | 91.55 | 80.26 | -12.33% | 0.996 | 1.000 | +0.004 |
| Chute release | [G40W-7] | 310.0 | 307.1 | -0.92% | 72.18 | 72.06 | -0.17% | 5.521 | 5.520 | -0.001 |
| Chute release | [G80T-10] | 490.3 | 481.9 | -1.72% | 106.40 | 106.23 | -0.16% | 5.480 | 5.481 | +0.001 |
| Dual parachute deployment | [H669N-P] | 593.1 | 580.4 | -2.15% | 135.42 | 135.44 | +0.02% | 4.460 | 4.446 | -0.013 |
| Dual parachute deployment | [H242T-P] | 698.4 | 685.4 | -1.86% | 140.98 | 141.07 | +0.06% | 4.317 | 4.306 | -0.011 |
| Dual parachute deployment | [J570W-P] | 2224.7 | 2128.7 | -4.32% | 388.59 | 385.90 | -0.69% | 3.276 | 3.266 | -0.010 |
| Dual parachute deployment | [H999N-P] | 897.8 | 872.2 | -2.85% | 190.83 | 190.91 | +0.04% | 4.206 | 4.193 | -0.013 |
| Dual parachute deployment | [I1299N-P] | 1159.0 | 1121.2 | -3.26% | 242.76 | 242.89 | +0.05% | 3.934 | 3.921 | -0.013 |
| Dual parachute deployment | [G64W-P] | 227.4 | 226.6 | -0.38% | 58.54 | 58.48 | -0.11% | 4.877 | 4.862 | -0.015 |

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
