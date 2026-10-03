# Motor stock and prices

This page covers where hpr gets motor stock and prices: [motor.fusionspace.co](https://motor.fusionspace.co)
(the *motor finder*), a free site that reads a dozen U.S. vendors' public listings every hour and publishes, for every
AeroTech, Cesaroni and Loki motor of [impulse class](glossary.md#impulse-class) D and up that they
carry, who has it in stock and at what price. hpr reads the site's public data API (its
machine-readable files) and saves each answer, so the same list works later with no network. It
is for anyone choosing a motor they can actually buy: "which L motors are in stock, and what does
one cost?" hpr then matches each motor in stock to its record on
[ThrustCurve.org](glossary.md#thrustcurveorg), the public database of motor data, and can download
that record's [thrust curve](glossary.md#thrust-curve), so a motor you can buy is a motor you can
fly ([A motor from a file](physics/motor.md#a-motor-from-a-file)). At the command line,
`hpr motors search --in-stock --class L --max-price 150` lists the motors in stock by class and
price ([Motors you can buy](cli.md#motors-you-can-buy)); a Rust program calls the library, as
below.

**How far to trust it.** hpr gives back the site's values unchanged, and the saved copy gives them
back offline. That is checked on recorded answers, below: eight from motor.fusionspace.co and two
curve files from ThrustCurve.org, beside three stand-in searches in ThrustCurve.org's shape, since
ThrustCurve.org grants no licence for its motor records (its site reads "All rights under
copyright reserved"). Whether a vendor really has a motor, at that price, is the vendor's to say. The site's data is up to about an hour old when it is built,
and hpr counts its saved copy as fresh for another hour, so a fresh answer can be two hours behind
the vendor's page; a stale copy is as old as its date says. (ThrustCurve.org's answers count as
fresh for a day.) The site's terms ask you to check stock
and price on the vendor's own page before relying on them. Prices are in U.S. dollars.

The match is by name. On ThrustCurve.org's answers of 1 October 2026, all 282 motors in stock
matched exactly one ThrustCurve.org record ([below](#matching-motors-to-thrustcurveorg)), and each
matched record's size, impulse, average thrust and burn time equal the motor finder's. A curve is the file someone
uploaded to ThrustCurve.org, read by the same readers as a motor file on your disk.

The tests replay saved answers, and CI never contacts either live site. Both were contacted by
hand, over an encrypted (HTTPS) connection, to record them. If a site changes its format, you will
find out when your program, or `hpr motors search`, gets a refused answer, not from a failing test.

Code: `hpr_net::motor_finder` ([API reference](api/hpr_net/motor_finder/index.html)), written for
[M5.4a](decisions-and-roadmap.md#m5-4a), the first motor-stock increment, and
`hpr_net::thrustcurve` ([API reference](api/hpr_net/thrustcurve/index.html)), written for
[M5.4b](decisions-and-roadmap.md#m5-4b), the second. They need the `net` feature of the `hpr`
crate: `hpr = { ..., features = ["net"] }` in `Cargo.toml`, then `hpr::hpr_net::motor_finder`.
The choices are in
[ADR-129: motor stock through the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-129-m54-split-and-m54a-the-motor-finders-api-through-the-cache-2026-10-01)
and
[ADR-130: ThrustCurve.org and the match](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-130-m54b-thrustcurve-searches-and-curves-through-the-cache-and-the-in-stock-join-2026-10-01).
`hpr motors search` was added by [M5.4c](decisions-and-roadmap.md#m5-4c), the third; its choices are in
[ADR-131: the command](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-131-m54c-hpr-motors-search-stock-and-prices-at-the-command-line-2026-10-01).
How saved answers work is on [Online data and the cache](online-data.md). The motors hpr carries
with it, with their thrust curves, are on [Solid motors](physics/motor.md).

## What the site publishes

The site documents its API, and its terms, in
[its own repository](https://github.com/nrdptel/Hobby-Rocket-Motor-Finder/blob/main/docs/api.md).
The API is a handful of files, rebuilt about every hour. There is no key, no limit on how often you
ask, and no way to ask for part of a file: a program fetches a whole file and picks what it needs.

| File | What it holds | On 2026-10-01 |
|---|---|---|
| `meta.json` | when the files were built, and how many motors, motors in stock and vendors they hold | 760 bytes |
| `motors.json` | every motor some vendor lists, D class and up | 598 motors, 1.6 MB |
| `in-stock.json` | the same, only those in stock at one vendor or more | 282 motors, 0.96 MB |
| `vendors.json` | the vendors read, with how many motors each lists and has in stock | 12 vendors, 1.1 kB |
| `motors/{maker}/{motor}.json` | one motor, as in the lists | 1.4 to 8.1 kB for the four recorded |

Each motor carries its maker; its [designation](glossary.md#motor-designation) (`L1150R`,
`3683L851-P`), spelled as [ThrustCurve.org](glossary.md#thrustcurveorg) spells it; its impulse
class, diameter, [total impulse](glossary.md#total-impulse),
[average thrust](glossary.md#average-thrust) and [burn time](glossary.md#burn-time), taken from
ThrustCurve.org; its propellant; for a reload, the reusable case it needs (`RMS-29/180`); its
[delays](glossary.md#ejection-delay); whether it is discontinued (old stock only: two motors in
stock were, on the recorded morning); whether it ships as hazardous material, as the site labels it
from the propellant's weight; and every vendor's *listing* of it.

A listing is one product page: its status, its sticker price, its pack size, the price of one
motor, units on hand when the vendor shows them (267 of the 3,685 listings did), and when the site
last read it. The status is *in stock*, *out of stock*, *special order* (made or ordered for you,
often with a lead time of weeks; not counted as in stock), or *unknown*. A vendor may list one
motor several times, for different delays or pack sizes. Each motor in stock also names its
cheapest offer, by the price of one motor.

Prices are whole cents. The price of one motor is the sticker price over the pack size, to the
nearest cent: a $38.49 two-pack is $19.25 a motor (the 263 of the 3,685 listings that fall on a
half cent all round up).

### What hpr refuses

An answer is refused, and not saved, when:

- it isn't JSON, or a field is missing or of the wrong type;
- its schema version (the number the API puts on its format) isn't 1: the API puts a breaking
  change under a new address;
- its build time isn't a UTC time;
- a list's count disagrees with the list;
- a motor's impulse class isn't one capital letter, its diameter isn't above zero, or its impulse,
  thrust or burn time is below zero;
- a motor's listing count disagrees with its listings;
- a motor has a cheapest offer but isn't in stock, or is in stock with none;
- a pack holds no motors, or a motor costs more than its pack;
- `in-stock.json` holds a motor out of stock;
- a motor's own page holds another motor.

One broken value refuses the whole file: hpr keeps its last good copy rather than a list it can't
trust, and online it hands that copy back, marked stale, with the reason. Two things the API
allows are read, not refused: a cheapest offer with no price (when no vendor with the motor in
stock shows one), and a listing status the API adds later, which reads as *unknown*.

Asking for a motor the site doesn't list gets the site's "not found" page; hpr returns it as an
error naming the address, and doesn't save it. The makers can be named in full or in short:
`aerotech`, `cesaroni` and `loki`, in any case.

## Matching motors to ThrustCurve.org

The motor finder carries no thrust curves, and no ThrustCurve.org id. ThrustCurve.org has both: it
aims to hold a record for every certified motor, each with its own id (24 hexadecimal digits, such as
`5f4294d2000231000000044f`), and the simulator files people have uploaded for it. hpr asks its
[public API](https://www.thrustcurve.org/info/api.html) two things:

| Request | What it gives | hpr's call |
|---|---|---|
| a *search* | motor records: id, maker, designation, class, diameter, impulse, burn time, delays, how many data files | `thrustcurve::fetch_search` |
| a *download* | one motor's data files in one format, [RASP (`.eng`) or RockSim (`.rse`)](glossary.md#rasp-and-rocksim-files), each with who measured it and its licence | `thrustcurve::fetch_download` |

**The match.** The finder spells each designation exactly as ThrustCurve.org does. So hpr matches a
motor in stock to the record whose maker (full name, such as `Cesaroni Technology`) and
designation are the same, character for character. If no record has that name, or more than one
does, the motor is a *miss*, and the match's report (`Join::report()`) lists it with the reason.
hpr doesn't guess: it doesn't compare impulse or diameter, or try other spellings. A motor
ThrustCurve.org spells differently is reported as a miss, never matched by a guess. (The report
and the code call a matched motor *mapped*.) `thrustcurve::fetch_finder_records`
fetches the records the match needs, one search for each of the three makers the finder reads,
and `thrustcurve::join` matches them.

On ThrustCurve.org's answers of 1 October 2026, every motor in stock matched
([ADR-130: ThrustCurve.org and the match](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-130-m54b-thrustcurve-searches-and-curves-through-the-cache-and-the-in-stock-join-2026-10-01)):

| Maker | Motors in stock | Matched | Missed | ThrustCurve.org records searched |
|---|---:|---:|---:|---:|
| AeroTech | 153 | 153 | 0 | 307 |
| Cesaroni Technology | 99 | 99 | 0 | 296 |
| Loki Research | 30 | 30 | 0 | 60 |
| **All** | **282** | **282** | **0** | **663** |

Every matched record listed at least one data file, in some format, and for every match the
diameter, total impulse, average thrust and burn time were the same on both sides, so each match
was the right motor by more than its name. The motor finder copies ThrustCurve.org's names and
figures, so a full match is expected; the match is there to catch drift.

ThrustCurve.org grants no licence for its motor records; its site reads "All rights under
copyright reserved". So the repository no longer keeps those three answers, and the tests replay
stand-ins in the same shape instead
([ADR-145: the recordings' licences](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-145-recorded-answers-and-their-licences-stand-in-thrustcurve-searches-cc-by-40-for-the-motor-finder-2026-10-03)). They hold
297 records:

- 15 wholly invented, five per maker, with designations ending `-INVENTED`;
- 282, one for each motor in the finder's in-stock list. Each carries only the values that list
  states, which are ThrustCurve.org's published figures as the finder relays them under CC BY 4.0,
  plus an invented id and an invented count of data files. J450DM and F27R/L keep their real ids,
  those of the two public-domain downloads, so the match leads to a committed file. The fields
  only ThrustCurve.org states (length, weights, peak thrust, certifying body, dates, links) are
  left out.

On them, too, all 282 motors match. The tests write the match's report into
[`validation/reports/thrustcurve-join.md`](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/thrustcurve-join.md),
and check it against the match on every run. So the committed tests show that the code keeps the
rule; the full match on ThrustCurve.org's own records is the measurement of 1 October 2026 above,
which last ran at commit [5150e0d](https://github.com/nrdptel/hpr-sim/commit/5150e0d) (the test was added by [PR #277](https://github.com/nrdptel/hpr-sim/pull/277)).
The milestone's goal ([M5.4b](decisions-and-roadmap.md#m5-4b)) was 95% of the motors in stock;
the test asserts that too.

**The curve.** A download's file is read by the same readers as a motor file on your disk
([Solid motors](physics/motor.md)): `.eng` by `hpr_motor::eng`, `.rse` by `hpr_motor::rse`.
[`DataFile::read()`](api/hpr_net/thrustcurve/struct.DataFile.html#method.read) picks the reader by
the file's format, and
[`Curve::thrust_curve()`](api/hpr_net/thrustcurve/enum.Curve.html#method.thrust_curve) gives the
first motor's curve. Each file says who measured it (`cert`, a certification test; `mfr`, the
maker; or `user`) and its licence: `PD` for public domain, `free` or `other`, or none given.
ThrustCurve.org's API doesn't define `free` and `other`, and hpr doesn't interpret them; only
`PD` files are recorded for the tests. An answer with no files gives an empty list, not an error;
that is how the API answered an unknown id when tried by hand on 1 October 2026.

**What hpr refuses.** A search or download is refused, and not saved, when:

- it carries the API's error message, on the answer or on one of the search's terms;
- a search returns more motors than it says match;
- an id isn't 24 hexadecimal digits;
- a size, thrust, impulse, burn time or weight is below zero;
- a file isn't base64 (the encoding the API sends files in);
- a download holds a file of another motor or format than asked;
- a search for the match was cut short: it says more motors match than it returned (it asks for up
  to 5,000), so a match never runs on part of a maker's records;
- a search for the match holds another maker's record.

Records may leave out any field but the id, maker and designation: the API sends only the fields
that have values. A file that decodes but isn't plain text (UTF-8) is not refused: reading that
one file is an error, and the motor's other files stay readable.

## An example

[`crates/hpr/examples/motor_stock.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/motor_stock.rs)
lists the L motors in stock, cheapest first, then matches the motors in stock to ThrustCurve.org
and reads one curve. It makes these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. The
   *transport* is what fetches ([Online data and the cache](online-data.md)). A real program
   passes `hpr_net::Http::new()`, and keeps its saved answers in `Cache::platform_dir()`, the
   system's usual folder for them. This one never uses the network: a stand-in transport answers
   with the files recorded for the tests.
2. `motor_finder::fetch_meta(&client, now)` and `motor_finder::fetch_in_stock(&client, now)` fetch
   the build time and the in-stock list, and save them; `now` is the time, in seconds since
   1 January 1970.
3. The same `fetch_in_stock` two hours later, through a client in `Mode::Offline` whose transport
   has no network, answers from the saved copy, marked stale: older than an hour.
4. It keeps the motors whose `impulse_class` is `L`, and sorts them by their
   `cheapest_in_stock.unit_price_cents`, the price of one motor.
5. `thrustcurve::fetch_finder_records(&client, now)` fetches ThrustCurve.org's records of the
   three makers, and `thrustcurve::join(&in_stock.motors, &records)` matches the motors in stock.
6. `Download::new(id, Format::Rasp)` and `thrustcurve::fetch_download` fetch AeroTech J450DM's
   `.eng` file by its matched id, and `read()?.thrust_curve()?` reads it. It uses J450DM because
   the tests record only public-domain files, and this is one hpr already carries.

Run it from a copy of the repository with
`cargo run --example motor_stock -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/motor_stock.output.txt -->
```text
Motor stock data from motor.fusionspace.co, licensed CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/); aggregated from public vendor listings and ThrustCurve.org; provided as is, with no warranty: check stock and price on the vendor's own page before relying on them
built 2026-10-01T07:07:29+00:00: 598 motors listed, 282 in stock, from 12 vendors
first read: Fetched; offline two hours later: Stale, the same list: true

20 L motors in stock; the five cheapest by the price of one motor:
manufacturer         motor         dia (mm)  impulse (N·s)   one motor  cheapest at              vendors
AeroTech             L1520T              75         3715.9     $260.99  Balsa Machining Service        1
AeroTech             L850W               75         3646.2     $282.74  Sirius Rocketry                2
Cesaroni Technology  3419L645-P          75         3419.8     $286.36  Performance Hobbies            2
Cesaroni Technology  3683L851-P          75         3683.2     $290.39  Animal Motor Works             3
AeroTech             L1150R              75         3517.0     $324.99  Animal Motor Works             1

Motor data and thrust curves courtesy of ThrustCurve.org, https://www.thrustcurve.org/
297 records of the three makers in stand-in searches: 15 invented, 282 with the motor finder's copy of ThrustCurve.org's figures; 282 of 282 motors in stock matched to one each, 0 missed
AeroTech J450DM: id 5f4294d2000231000000044f, a RASP file from source cert, licence PD, 37 points from ignition
the file:  total impulse   1061.6 N·s, burn time  2.28 s, average thrust  465.6 N
record:    total impulse     1055 N·s, burn time  2.27 s, average thrust    465 N
```

CI checks that it still prints this (`cargo xtask examples --check`). The first line is the credit
the site asks for, with its caution; show it wherever a price or stock is shown. `Fetched` and
`Stale` say where an answer came from
([how long a copy stays fresh](online-data.md#how-long-a-copy-stays-fresh)). `vendors` is how many
vendors had the motor in stock. On that morning no L motor in stock cost $150 or less: the
cheapest was $260.99.

The last lines come from ThrustCurve.org, under its own credit line, except the records: the 297
are the stand-in searches ([above](#matching-motors-to-thrustcurveorg)), so the count is theirs,
and the match runs on the finder's own copy of ThrustCurve.org's figures. J450DM's file is a real
answer: the certification test's curve (`source cert`), public domain, and the very file hpr
already carries for J450DM ([Solid motors](physics/motor.md)). Its curve starts at zero thrust at
ignition, then follows the file's 36 points. hpr works its figures out from those points, so they
differ from the published figures on the record (here the motor finder's copy of them):

| | from the file | on the record | difference |
|---|---:|---:|---:|
| total impulse | 1,061.6 N·s | 1,055 N·s | +0.6% |
| burn time | 2.28 s | 2.27 s | +0.4% |
| average thrust | 465.6 N | 465 N | +0.1% |

The average thrust is the total impulse over the burn time.

## Credit and terms

### motor.fusionspace.co

The site licenses its API's answers under Creative Commons Attribution 4.0 (CC BY 4.0), with the
credit "Motor stock data from motor.fusionspace.co", and allows keeping recorded answers, such as
the test recordings below ([its API page](https://motor.fusionspace.co/api), "Data licence"). The
data is gathered from public vendor listings and from ThrustCurve.org, and comes as is, with no
warranty.
hpr puts its credit line, `motor_finder::ATTRIBUTION`, on every answer, fetched or saved; the
example above prints it first, and `hpr motors search` prints it at the top of every list, in
text and in JSON, with ThrustCurve.org's below it. The site asks programs to use its files rather
than read the vendors' pages themselves, and to keep a copy rather than fetch on every use: hpr's saved copy
counts as fresh for an hour, as often as the site rebuilds.

### ThrustCurve.org

ThrustCurve.org grants no licence for its motor records; its site reads "All rights under copyright reserved", and its API asks for no particular credit. hpr puts
`thrustcurve::ATTRIBUTION`, "Motor data and thrust curves courtesy of ThrustCurve.org", on every
answer, as it credits the 32 curves it carries. Each data file has its own licence, set by whoever
uploaded it: check it before passing a file on. hpr's saved copy counts as fresh for a day, so a
program that asks again within the day doesn't ask the site again, since motor records and
curves change seldom.

## How it is checked

The tests in [`crates/hpr-net/tests/motor_finder.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/motor_finder.rs)
replay eight answers recorded from one build of the site, on 1 October 2026 at 07:07 UTC: the four
lists, and the pages of four motors: one each from AeroTech, Cesaroni and Loki, plus AeroTech's
`F27R/L`, whose name holds a `/` (its page is `F27R~L.json`). They read the expected values from
the recordings themselves, not through the code being tested, and check that:

- each answer, read and written back out, holds exactly the recording's fields and values, and
  carries the credit line;
- the API's words read with their meaning: H128W is a reload that ships as hazardous material,
  F27R/L a single-use motor whose shipping varies, D13W a reload that doesn't, and the listings
  count 827 in stock, 2,363 out of stock and 495 on special order;
- a second read online, inside the hour, is answered from the saved copy without a fetch;
- offline, with a transport that fails the test if it is ever called, each read gives the same
  values, fresh for the hour and marked stale after it;
- each motor's page is the same motor as in `motors.json`, and a maker can be named in full or in
  short, in any case;
- the files agree with each other: `in-stock.json` is `motors.json`'s motors in stock, value for
  value, and `meta.json` counts both lists and the vendors;
- each rule in the list above refuses an answer that breaks it, naming the field, and what the API
  allows (an offer with no price, a new status) is read;
- an answer that isn't JSON is refused and not saved; when one arrives after a good copy has gone
  stale, the good copy comes back with the reason;
- a page holding another motor is refused and not saved;
- offline, a file never fetched is an error naming its address.

Some of the site's rules hpr doesn't enforce when it reads an answer. The tests confirm that the
recording keeps them: each motor's page is at the address it names, the price of one motor is the
sticker price over the pack, the cheapest offer is the lowest-priced listing in stock, a motor is
in stock exactly when one of its listings is, and the vendor counts count distinct vendors.

The tests in [`crates/hpr-net/tests/thrustcurve.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/thrustcurve.rs)
replay two answers recorded from ThrustCurve.org on 1 October 2026 at 08:22 UTC, AeroTech J450DM's
`.eng` file and AeroTech F27R/L's `.rse` file, both public domain, and the three stand-in searches
described [above](#matching-motors-to-thrustcurveorg). They check that:

- each answer, read and written back out, holds exactly the recording's fields and values, carries
  ThrustCurve.org's credit, comes from the saved copy on a second read, and works offline, fresh
  for a day and stale after;
- the test builds its own name-to-id table straight from the stand-ins' JSON, and the match
  agrees for all 282 motors; each match's diameter, impulse, average thrust and burn time equal
  the finder's; and the report is the committed one;
- a motor renamed, two records of one name, or a designation in another case is a miss with its
  reason, and the report lists it with its counts; the same record given twice counts once, and
  another maker's record of the same designation doesn't disturb the match;
- J450DM, a matched motor in stock, has its downloaded file read by `hpr_motor` to the points in
  its lines, and the file is byte for byte the one hpr carries; F27R/L's `.rse` file reads to the
  points in its XML;
- each refusal above refuses an answer changed to break it, naming the field, and a record with
  only an id, maker and designation reads;
- a download of another motor or format, a search cut short and a search holding another
  maker's records are refused and not saved; a file that isn't plain text is that file's error
  alone.

`hpr motors search` is checked by
[`crates/hpr-cli/tests/motors_search.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/motors_search.rs),
which runs the command on the recorded lists, from the file and offline from the cache
([how far to trust it](cli.md#motors-you-can-buy)).

## What it leaves out

- **No curves at the command line.** `hpr motors search` lists stock and prices; it doesn't
  match motors to ThrustCurve.org or download their curves. A program does, as in the example.
- **A match by name only.** A motor ThrustCurve.org spells differently from the finder is a miss;
  hpr doesn't fall back on impulse or size. None missed on the recording.
- **The first motor of a file.** `Curve::thrust_curve()` reads a file's first motor; the two
  recorded files hold one each.
- **No choice among files.** A motor may have several files in one format (from a certification
  test, the maker or a user); hpr gives them all, in the API's order, and leaves the choice to the
  program.
- **Prices as listed.** hpr shows a price as the site gives it, and doesn't screen out a shop's
  placeholder price.
- **U.S. vendors and dollars only,** and only the three makers the site reads.
- **No history.** Each answer is the stock of one hour; hpr keeps only the latest copy of each
  file.
