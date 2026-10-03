# Motors in stock matched to ThrustCurve.org

`hpr_net::thrustcurve::join` ([M5.4b](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m5-4b),
the ThrustCurve.org increment; why the match is by name only is in
[ADR-130](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-130-m54b-thrustcurve-searches-and-curves-through-the-cache-and-the-in-stock-join-2026-10-01))
matches (*maps*) each motor in motor.fusionspace.co's in-stock list to the one ThrustCurve.org
record with its manufacturer's full name and its designation, character for character. A motor
with no such record, or several, is a miss. This is the join on the answers in
`crates/hpr-net/tests/fixtures/replay/`: the in-stock list built on 2026-10-01 at 07:07:29 UTC,
and three stand-in searches of AeroTech, Cesaroni Technology and Loki Research in ThrustCurve.org's
shape (158, 104 and 35 records). ThrustCurve.org grants no licence for its motor records; its site reads "All rights under copyright reserved", so the stand-ins replace its answers
([ADR-145](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-145-recorded-answers-and-their-licences-stand-in-thrustcurve-searches-cc-by-40-for-the-motor-finder-2026-10-03)): each holds five invented motors, and one record for each of that maker's
motors in the in-stock list, carrying only the list's values (ThrustCurve.org's published figures
as the finder relays them, CC BY 4.0), an invented id and an invented count of data files.

So this table tests the code, not ThrustCurve.org's own answers. "Mapped, no data file listed" counts
matched motors whose record gives 0 data files, or leaves the count out (the API leaves out a
field with no value); on the stand-ins every count is invented and at least 1, so the column is 0
by construction. Likewise the test's check that every matched record's diameter, total impulse,
average thrust and burn time equal the finder's holds by construction. The facts about
ThrustCurve.org's own records are the measurement of 2026-10-01
([ADR-130](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-130-m54b-thrustcurve-searches-and-curves-through-the-cache-and-the-in-stock-join-2026-10-01)): on its answers searched at 08:22 UTC that day (307, 296 and 60 records), the
join mapped the same 282 of 282, each matched record listed at least one data file, and each
one's diameter, total impulse, average thrust and burn time equalled the finder's. That
measurement last ran at commit [5150e0d](https://github.com/nrdptel/hpr-sim/commit/5150e0d) (added by [PR #277](https://github.com/nrdptel/hpr-sim/pull/277), M5.4b), and is
not re-run here. The milestone asked for 95% of the motors in stock.
`crates/hpr-net/tests/thrustcurve.rs` checks this table against the join on every run, on every
system; `HPR_WRITE_THRUSTCURVE_JOIN=1` rewrites it.

<!-- join: written by crates/hpr-net/tests/thrustcurve.rs -->
282 of 282 motors mapped to one record each.

| Manufacturer | Motors | Mapped | Mapped, no data file listed | Missed |
|---|---:|---:|---:|---:|
| AeroTech | 153 | 153 | 0 | 0 |
| Cesaroni Technology | 99 | 99 | 0 | 0 |
| Loki Research | 30 | 30 | 0 | 0 |

No misses.
<!-- end of join -->
