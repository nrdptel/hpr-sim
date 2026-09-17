# ThrustCurve.org motor data for the M1.3 catalog

Checked 2026-09-17 with `validation/oracles/thrustcurve/survey.py` (pinned `motors.json`, API
captures cached in the gitignored `refs/samples/thrustcurve/`). ThrustCurve's site code
(`JohnCoker/thrustcurve3`, ISC, permissive) was read at `577afa6` only to confirm definitions.

## 1. Motor metadata (`search.json`: 1156 motors)

- Always present: `motorId` (24-hex), `manufacturer`, `manufacturerAbbrev`, `designation`,
  `commonName` (`1/4A3`, `K550`), `impulseClass` (`A`..`O`; 1/8A to 1/2A are `A`), `diameter` and
  `length` (mm), `type` (`SU` 220, `reload` 784, `hybrid` 152), `certOrg` (full name: TRA 607, CAR
  303, NAR 203, `Unknown` 27, `Uncertified` 13, AMRS 3), `totImpulseNs` (N·s), `avgThrustN` (N),
  `burnTimeS` (s), `dataFiles`, `delays` (`"4,6,8"` or `"P"`), `delayAdjustable`, `sparky`,
  `updatedOn`, `availability` (`regular` 789, `OOP` 340, `occasional` 27) and `source_url`.
- Sometimes present: `maxThrustN` (N, 965), `totalWeightG` and `propWeightG` (g; 1011, 1143),
  `caseInfo` 846, `propInfo` 933, `infoUrl` (cert data sheet) 223. `metadata.json` has 27
  `manufacturers`, 5 `certOrgs` (no `Unknown`), `types`, 18 `diameters` (mm) and `impulseClasses`.
  The API page says SI "except for motor mount tubes", but search results are in mm and g.

## 2. How ThrustCurve defines the statistics

- Glossary (https://www.thrustcurve.org/info/glossary.html): **Average Thrust** is "The Total
  Impulse divided by the Burn Time". **Burn Time**: "(Per NFPA 1125, the beginning and end of the
  curve, where the thrust is below 5% of the maximum, are excluded.)"
- Motor Statistics, Burn Time Normalization (https://www.thrustcurve.org/info/motorstats.html):
  "The burn time is from 5% of peak thrust, at both the beginning and end of the curve." "The total
  impulse is measured over the whole thrust curve (not the 5%-defined burn time)." Dividing by the
  "last action" burn time is "a common error". Its average-thrust line ("the total impulse during
  the 5%-defined burn time, divided by the burn time") conflicts with the glossary.
- Site code (`simulate/analyze/analyze.js`, ISC): the peak is the largest sample; the 5% crossings
  are linearly interpolated, from (0, 0) if the first sample is already above 5%; the impulse is a
  trapezoid plus a triangle from (0, 0) to the first sample; `avgThrust = totalImpulse / burnTime`.
  First it sorts, drops leading samples below 0.5 mN, and merges samples less than 50 µs apart.
- **Motor metadata is stored, not recomputed from simfiles.** The motor schema
  (`database/schema/schema.js`) has its own `totalImpulse`, `avgThrust` and `burnTime`. The values
  come "from the certification organization ... or derived from them and/or the thrust curve"
  (motorstats.html). Each simfile page shows the file's statistics next to the motor's.
- Measured: `avgThrustN*burnTimeS/totImpulseNs` is within 1% of 1 for 749 of 1004 solid motors
  (median exactly 0; CAR 274/293, NAR 162/202, TRA 282/466, Uncertified 2/13). The other 255 are
  inconsistent by more than 1%. Curves can't separate F = I/t from I(window)/t (only 0.13% of the
  impulse lies outside the window, median), so the glossary and the code decide: I/t.

## 3. Download API (https://www.thrustcurve.org/info/api.html, `/api/v1/swagger.json` v1.0.3)

- `/api/v1/download.json` takes GET (query) or POST (JSON): `motorId` or `motorIds` (an array;
  repeat the key in a GET), `format` (`RASP`|`RockSim`), `license` (`PD`|`free`|`other`), `data`
  (`file` default, `samples`, `both`) and `maxResults`. POST batches of 250 ids worked.
- Result fields: `motorId`, `simfileId`, `format`, `source` (`cert`|`mfr`|`user`), `license`,
  `data` (base64 file), `samples` ([{`time` s, `thrust` N}]: parsed points, no implicit origin),
  `infoUrl`, `dataUrl` (relative) and `source_url`. The server writes `license || ''`, and JSON
  omits the empty key (1125 of 1712 files). The bytes at `https://www.thrustcurve.org{dataUrl}`
  equal `data` (checked for N3300R).
- Licenses, as the uploader picks them (https://www.thrustcurve.org/info/contribute.html#license):
  Public Domain, "anyone may do anything with the file"; Free Usage, "traditional free software
  licenses (I.e., GPL, Creative Commons, Apache)"; Other (restricted), "such as an explicit
  copyright"; Unknown. No site-wide data terms; the Swagger "ISC license" covers the API spec.
  **Bundle `PD` only:** `free` may be GPL or share-alike, unstated; `other` is restricted; no
  value means unknown. Also exclude PD files whose own text contradicts the flag (section 4).

## 4. Survey: solid motors (hybrids excluded)

- 1004 motors (278 OOP), 924 with data, 1712 simfiles (= the `dataFiles` sum). License: none
  1125, **PD 554** (392 motors), free 33, other 0. Source: cert 761, user 701, mfr 250. Format:
  RASP 889, RockSim 823. PD by source: cert 367, user 152, mfr 35. In production: 1333, 510 PD.
- The script's parses equal ThrustCurve's `samples` for all 1712 files. Two non-PD RockSim files
  go backwards in time, 69 repeat a time, and 1 ends above 5% of peak.
- PD files contradicted by their own text (exclude): 12 Klima (A6, B4, C2, C6, D3, D9, both
  formats) say "Created by Leo Nutz for OpenRocket"; 2 KBA (I310S, I370F) carry a Tripoli Motor
  Testing copyright. 66 PD headers differ on length (>1 mm) and 5 on diameter (>0.5 mm): L43.

## 5. Curve against metadata (1% on total impulse, burn time and average thrust)

Files within 1%. Impulse: trapezoid from an implicit (0, 0). Crossings: linear interpolation.

| Definition | Impulse | Burn time | Avg thrust | All three, PD (n=554) | All three, all (n=1712) |
| --- | --- | --- | --- | --- | --- |
| A: I whole, t 5%-5%, F = I/t (ThrustCurve) | 377 | 247 | 264 | **196 (35.4%)** | 416 |
| B: as A, F = I(window)/t | 377 | 247 | 262 | 198 | 423 |
| C: t = last sample, F = I/t_last | 377 | 65 | 56 | 35 (6.3%) | 387 |

- Burn-time cutoff sweep, PD files within 1%: last sample 65, 2% 87, 3% 131, **5% 247**, 7% 187,
  10% 91. Over all files, "last sample" ties 5% (567 vs 566) because user files often end at the
  published burn time, as contribute.html teaches.
- PD |error| under A: impulse median 0.50%, p90 4.0%, max 21%; burn time median 1.26%, p90 9.1%,
  max 38%; average thrust median 1.11%, p90 9.0%, max 36%.
- PD passes by source: cert 153/367, user 35/152, mfr 8/35. By class: A 0/7, B 2/10, C 2/12,
  D 1/16, E 4/16, F 11/37, G 18/48, H 7/50, I 14/61, J 30/73, K 30/66, L 31/64, M 27/57, N 10/25,
  O 9/12. By maker: Cesaroni 110/147, AeroTech 57/280, Loki 16/41, others 13/86.
- 121 of 392 PD motors have a passing PD file. Of Loft's 45 PD curves: 10 pass, 34 fail, 1 is a
  hybrid.

## 6. Bundle (`validation/oracles/thrustcurve/bundle.py`)

- **Rule:** license PD; within 1% on all three under definition A; solid, in production; no
  provenance flag; up to 3 per class from distinct manufacturers, ranked cert > mfr > user, RASP
  first, then smallest worst error. **32 curves, 32 motors, classes B to O** (no class A passes).
- **Curves:** B Quest B4; C Estes C5; D Quest D5 (RSE, user); E CTI 26E31, AT E26W; F AT F52C,
  CTI 68F240, Estes F15 (RSE, user); G CTI 131G84, AT G69N; H CTI 168H54, Loki H125-CT, AT H170M
  (RSE, mfr); I CTI 411I175, AT I175WS, Loki I377-CT; J CTI 1266J760, Loki J300LR, AT J450DM;
  K CTI 1633K940, AT K400C, AMW 2245K1075; L CTI 3300L3200, AT L2500ST, Loki L1040LR; M CTI
  8187M1545, Loki M1378LR, AT M1350W; N CTI 13628N5600, AT N3300R; O CTI 21062O3400, AT O6000W.
  The ids, sha256 and URLs are in `crates/hpr-motor/data/thrustcurve/catalog.json`.
- The script downloads each `data.eng`/`.rse`, checks that it parses to the API's samples, that
  it is PD and that it passes, then writes the curve bytes unchanged (31 of 32 are CRLF),
  `catalog.json` and `src/bundled.rs`. Two runs give byte-identical output. `.gitattributes` marks
  the curves `-text` so git keeps their bytes.
- Notes: N3300R's header says 1060 mm long, the metadata 1046 mm (an L43 test). The AMW K1075
  header names CTI. D5 and F15 are user curves matched to NAR's averaged data.
- **Excluded:** of 554 PD files, 358 fail the 1% check (A), including all 14 whose text
  contradicts PD (section 4). 157 PD files belong to motors whose stored stats already disagree by
  more than 1%, and only 5 of those pass. Of the 196 passes, the rest lose on the per-class cap,
  on availability (4 OOP), or to the same data in another format.

| Why a PD curve fails | Files |
| --- | --- |
| Burn time and/or average thrust off by more than 1%, impulse within 1% | 149 |
| Impulse off 1-5% | 134 |
| Impulse off more than 5% (up to 21%; likely a wrong curve, L42) | 41 |
| Only on a value printed coarser than 1% (for example 0.23 s) | 34 |

## Pins

- Pinned in M1.3 (`validation/refs.lock.toml`): the glossary, motor-statistics, contribute, RASP
  and simulators pages, and `analyze.js` at commit `577afa6`.
- Not pinned: `info/api.html` (221e4814…) and `api/v1/swagger.json` (566009ab…), which only
  describe the API.
- The bundled curves are committed with their sha256 in `catalog.json`, so they need no refs pin.
  The 4 survey POST captures (2026-09-17T09:53Z) need a request body in snapshot pins; until
  then, they stay an unpinned cache, and `bundle.py` checks licenses and samples against them.
- **Coarse metadata.** Four bundled motors pass against stored values printed coarser than 1%:
  Quest B4 (burn time 1.1 s, average thrust 4.4 N), CTI 68F240 (0.29 s), Loki J300LR (4.1 s) and
  AeroTech K400C (3.2 s). Their 1% agreement is real but only as tight as those digits.
