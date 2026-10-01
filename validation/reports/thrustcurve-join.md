# Motors in stock matched to ThrustCurve.org

`hpr_net::thrustcurve::join` ([M5.4b](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m5-4b),
the ThrustCurve.org increment; why the match is by name only is in
[ADR-130](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-130-m54b-thrustcurve-searches-and-curves-through-the-cache-and-the-in-stock-join-2026-10-01))
matches (*maps*) each motor in motor.fusionspace.co's in-stock list to the one ThrustCurve.org
record with its manufacturer's full name and its designation, character for character. A motor
with no such record, or several, is a miss. This is the join on the recorded answers in
`crates/hpr-net/tests/fixtures/replay/`: the in-stock list built on 2026-10-01 at 07:07:29 UTC,
and ThrustCurve.org's records of AeroTech, Cesaroni Technology and Loki Research, searched at
08:22 UTC that day (307, 296 and 60 records). "Mapped, no data file listed" counts matched
motors whose record gives 0 data files, or leaves the count out (the API leaves out a field with
no value). The milestone asked for 95% of the motors in stock. The motor finder copies
ThrustCurve.org's designations and figures, so a full match is expected; the test also checks
that every matched record's diameter, total impulse, average thrust and burn time equal the
finder's. `crates/hpr-net/tests/thrustcurve.rs` checks this table against the join on every run,
on every system; `HPR_WRITE_THRUSTCURVE_JOIN=1` rewrites it.

<!-- join: written by crates/hpr-net/tests/thrustcurve.rs -->
282 of 282 motors mapped to one record each.

| Manufacturer | Motors | Mapped | Mapped, no data file listed | Missed |
|---|---:|---:|---:|---:|
| AeroTech | 153 | 153 | 0 | 0 |
| Cesaroni Technology | 99 | 99 | 0 | 0 |
| Loki Research | 30 | 30 | 0 | 0 |

No misses.
<!-- end of join -->
