# Motor stock and prices

This page covers where hpr gets motor stock and prices: [motor.fusionspace.co](https://motor.fusionspace.co),
a free site that reads a dozen U.S. vendors' public listings every hour and publishes, for every
AeroTech, Cesaroni and Loki motor of [impulse class](glossary.md#impulse-class) D and up that they
carry, who has it in stock and at what price. hpr reads the site's public data API (its
machine-readable files) and saves each answer, so the same list works later with no network. It
is for anyone choosing a motor they can actually buy: "which L motors are in stock, and what does
one cost?" There is no `hpr` command for it yet: a Rust program calls the library. Nor does it yet
connect a motor in stock to a [thrust curve](glossary.md#thrust-curve) you can fly; that is
[M5.4b](decisions-and-roadmap.md#m5-4b), the next increment.

**How far to trust it.** hpr gives back the site's values unchanged, and the saved copy gives them
back offline. That is checked on eight recorded answers, below. Whether a vendor really has a motor,
at that price, is the vendor's to say. The site's data is up to about an hour old when it is built,
and hpr counts its saved copy as fresh for another hour, so a fresh answer can be two hours behind
the vendor's page; a stale copy is as old as its date says. The site's terms ask you to check stock
and price on the vendor's own page before relying on them. Prices are in U.S. dollars.

The tests replay saved answers, and CI never contacts the live site. It was contacted by hand, over
an encrypted (HTTPS) connection, to record them. If the site changes its format, you will find out
when your program gets a refused answer, not from a failing test.

Code: `hpr_net::motor_finder` ([API reference](api/hpr_net/motor_finder/index.html)), written for
[M5.4a](decisions-and-roadmap.md#m5-4a), the first motor-stock increment. It needs the `net`
feature of the `hpr` crate: `hpr = { ..., features = ["net"] }` in `Cargo.toml`, then
`hpr::hpr_net::motor_finder`. The choices are in
[ADR-129: motor stock through the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-129-m54-split-and-m54a-the-motor-finders-api-through-the-cache-2026-10-01).
How saved answers work is on [Online data and the cache](online-data.md). The motors hpr carries
with it, with their thrust curves, are on [Solid motors](physics/motor.md); this list is not
connected to them yet.

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

## An example

[`crates/hpr/examples/motor_stock.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/motor_stock.rs)
lists the L motors in stock, cheapest first. It makes these calls:

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

Run it from a copy of the repository with
`cargo run --example motor_stock -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/motor_stock.output.txt -->
```text
Motor stock and prices from motor.fusionspace.co, aggregated from public vendor listings and ThrustCurve.org; provided as is, with no warranty: check stock and price on the vendor's own page before relying on them
built 2026-10-01T07:07:29+00:00: 598 motors listed, 282 in stock, from 12 vendors
first read: Fetched; offline two hours later: Stale, the same list: true

20 L motors in stock; the five cheapest by the price of one motor:
manufacturer         motor         dia (mm)  impulse (N·s)   one motor  cheapest at              vendors
AeroTech             L1520T              75         3715.9     $260.99  Balsa Machining Service        1
AeroTech             L850W               75         3646.2     $282.74  Sirius Rocketry                2
Cesaroni Technology  3419L645-P          75         3419.8     $286.36  Performance Hobbies            2
Cesaroni Technology  3683L851-P          75         3683.2     $290.39  Animal Motor Works             3
AeroTech             L1150R              75         3517.0     $324.99  Animal Motor Works             1
```

CI checks that it still prints this (`cargo xtask examples --check`). The first line is the credit
the site asks for, with its caution; show it wherever a price or stock is shown. `Fetched` and
`Stale` say where an answer came from
([how long a copy stays fresh](online-data.md#how-long-a-copy-stays-fresh)). `vendors` is how many
vendors had the motor in stock. On that morning no L motor in stock cost $150 or less: the
cheapest was $260.99.

## Credit and terms

The site's terms: "Free to use; attribution to motor.fusionspace.co is appreciated." The data is
gathered from public vendor listings and from ThrustCurve.org, and comes as is, with no warranty.
hpr puts its credit line, `motor_finder::ATTRIBUTION`, on every answer, fetched or saved, and the
example above prints it first. The site asks programs to use its files rather than read the
vendors' pages themselves, and to keep a copy rather than fetch on every use: hpr's saved copy
counts as fresh for an hour, as often as the site rebuilds.

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

## What it leaves out

- **No command.** `hpr motors` doesn't read stock yet;
  [M5.4c](decisions-and-roadmap.md#m5-4c) will add `hpr motors search --in-stock --class L
  --max-price 150`.
- **No thrust curves.** The site names a motor as ThrustCurve.org does, but carries no
  ThrustCurve.org id or curve. Matching the two by name, so that a motor in stock can be flown, is
  [M5.4b](decisions-and-roadmap.md#m5-4b).
- **Prices as listed.** hpr shows a price as the site gives it, and doesn't screen out a shop's
  placeholder price.
- **U.S. vendors and dollars only,** and only the three makers the site reads.
- **No history.** Each answer is the stock of one hour; hpr keeps only the latest copy of each
  file.
