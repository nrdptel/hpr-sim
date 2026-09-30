# Weather-balloon soundings

This page covers fetching a weather balloon's measurements from the
[University of Wyoming's archive](https://weather.uwyo.edu/upperair/sounding.shtml) and flying a
rocket through them. The balloon carries a [radiosonde](glossary.md#radiosonde), which measures
the air on the way up, from the ground to 30 km or more. What it records is a
[sounding](glossary.md#sounding): temperature, pressure, humidity and wind at a column of heights.
Hundreds of stations release one at 00 and 12 UTC (universal time, the time at Greenwich) every
day. This page is for anyone who wants a flight in the air that was measured near a launch,
instead of a forecast ([Launch-day weather](weather.md)) or the
[standard atmosphere](glossary.md#standard-atmosphere).

**How far to trust it.**

- hpr turns the archive's answer into a profile that gives back the pressure, temperature,
  humidity and wind of every level it keeps, to rounding error. That is checked on three recorded
  soundings, below.
- A sounding is a measurement, but only at its station and time. The nearest station can be
  100 km or more from a launch site (Santa Teresa, below, is about 130 km from Spaceport
  America), and the balloon goes up hours before or after the flight. Nothing here measures how
  much that changes a flight.
- Each row's height is checked against its pressure and temperature, so a gross error (a digit
  lost) is left out, and an answer with many is refused. A wrong wind is not caught.
- The archive serves two versions of most soundings, and they can disagree near the ground. In
  the example below, Calisto is 402 m from the pad at apogee in one and 563 m in the other.
- The tests replay recorded answers; the live connection to the archive is not tested in CI.

Code: `hpr_net::wyoming` ([API reference](api/hpr_net/wyoming/index.html)), written for the
second weather increment, [M5.2b](decisions-and-roadmap.md#m5-2b). It needs the `net` feature of
the `hpr` crate. The choices are in
[ADR-120: University of Wyoming soundings](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-120-university-of-wyoming-soundings-2026-09-30).

## What hpr asks for

A request names a station and the sounding's hour. The station is its World Meteorological
Organization (WMO) number, such as 72364 for Santa Teresa, New Mexico; the archive's page has a
map of them. `WyomingRequest::latest_before(station, launch_time)` picks the last 00 or 12 UTC
sounding at or before a launch, with times in seconds since 1 January 1970 UTC (Unix time). The
balloon goes up about an hour before that hour.

hpr asks for the archive's comma-separated text, one row per level, and reads these columns.
Pressures are in hectopascals (hPa, 100 Pa; sea-level pressure is about 1013 hPa).

| column | read as |
|---|---|
| time | when the balloon was released, from the first row |
| latitude, longitude | where it was released, from the first row |
| pressure, in hPa | the level's pressure |
| geopotential height, in m | its [geopotential height](glossary.md#geopotential-height) above sea level |
| temperature, in °C | its temperature |
| relative humidity, in % | its humidity, relative to liquid water (the file gives it relative to ice too) |
| wind direction, in degrees, and wind speed, in m/s | the wind, and where it blows from, clockwise from north |

The units are in the column names, and hpr refuses a file with any other. The archive serves two
versions of most soundings:

| version | what it is | rows |
|---|---|---|
| Coded message, the default | the message stations have sent for decades (WMO's TEMP code, FM 35) | about 200 |
| BUFR file | the station's newer digital file (WMO's Binary Universal Form for the Representation of meteorological data), a row every second of the climb | about 6,000 |

The coded message holds the standard pressure levels (850, 700, 500 hPa and so on) and the
*significant* levels between them, where the temperature or wind changes its trend.

The answer goes through `hpr-net`'s cache ([Online data and the cache](online-data.md)), and
`wyoming::fetch(&client, &request, now)` takes the time now, in Unix seconds, to judge how fresh a
saved copy is. For a day after its hour the archive's copy of a sounding can still fill in, as a
station's later messages arrive:

| the sounding's age | a saved copy stays fresh for |
|---|---|
| under a day | an hour |
| over a day | 30 days, if it was saved after the sounding's first day; an earlier copy is fetched again |

Offline mode answers from the disk only, however old the copy. An answer hpr can't read is never saved, so it can't replace a good
copy. A sounding the archive doesn't have comes back as an HTTP error (404, or 400 for a BUFR file
the station doesn't send), which the request reports; no test covers that.

The archive states no terms of use. Show the credit "Sounding from the University of Wyoming's
radiosonde archive" wherever you show the sounding. Every answer carries it.

## How the answer becomes a sounding

**Heights.** The file gives geopotential metres, which hpr converts to heights above sea level
at the first row's latitude with the World Meteorological Organization's formula (WMO-No. 8
eqs. 12.15 and 12.16, as the [atmosphere page](physics/atmosphere.md) explains). The balloon
drifts as it climbs. Converting at the latitude it reached instead would move a height by about
0.8 m per degree of drift at 10 km, and 2.5 m at 30 km; the example's balloon drifted 0.06°.

**Why geopotential.** Air pressure falls with height at a rate set by the air's temperature and
humidity. The [hypsometric equation](glossary.md#hypsometric-equation) turns that into how thick
each layer between two pressures must be, in geopotential metres:

```text
thickness = (R_d T̄_v / g₀) ln(p_lower / p_upper)
```

Here `R_d` is dry air's gas constant (287.05 J/(kg·K)), `g₀` standard gravity (9.80665 m/s²) and
`T̄_v` the mean of the two rows' [virtual temperatures](glossary.md#virtual-temperature): the
temperature, raised a little for the humidity (WMO-No. 8, eqs. 12.17 and 12.18). In the three
recorded soundings, across the 13 layers between the standard levels from 850 to 10 hPa, the
recorded thicknesses match that to 0.01% to 0.03% on average; single layers are off by 0.05% to
0.13% on average, either way. Read as metres above sea level instead, the layers would be 0.51% to
0.60% too thin on average. So they are geopotential metres, as the column says. The test holds the
average under 0.1% as geopotential metres, and beyond −0.4% as metres above sea level.

**Each row is checked against the one before it.** A row with a pressure, height and temperature
*fits* when its height above the last row that fit is that thickness, give or take an
*allowance*: 5% of the thickness, plus what rounding the two pressures can move it, plus 30 m.
The pressures are rounded to 1 hPa in the coded message at 100 hPa or more, and to 0.1 hPa
elsewhere. For example, from the coded message's row at 557 hPa to the next at 549 hPa:

| | value |
|---|---|
| virtual temperatures | 272.24 K and 271.42 K (−1.7 °C and −2.5 °C, 79% and 81% humidity) |
| `R_d T̄_v / g₀` | 7,956.8 m |
| thickness | 7,956.8 m × ln(557/549) = 115.1 m |
| recorded | 5,151 m − 5,035 m = 116 m, a miss of 0.9 m |
| allowance | 5.8 m (5%) + 14.4 m (rounding each pressure by 0.5 hPa) + 30 m = 50.1 m |

A 557 hPa row with a digit lost (57 hPa at the same 5,035 m) would need to be 18.6 km above the
570 hPa row, not 183 m, so it is left out. Without the check it would be kept, and every good row
up to 57 hPa, about 20 km, would lie below it and be dropped. The rows that fit form a chain from
the ground. A row missing only its wind or humidity still joins it, so a gap in the wind keeps the
layers checked thin. In the three recordings every row fits, missing by at most 1 m beyond
rounding, so the 30 m and the 5% are margin. They leave room for a height coded to 10 m (the
coded message's, from 500 hPa up) and for a layer whose inner rows have no temperature, where
the mean of its two ends' temperatures gives its thickness less well. An error under 30 m moves
a level's pressure by under 0.4%, so the check is for gross errors, not small ones.

Then hpr keeps:

- **The ground, the first row**: the pressure, temperature, humidity and wind at the station
  when the balloon was released. A first row with a value missing or impossible refuses the
  answer.
- **One row of each run with the same pressure, the middle one.** The BUFR file gives pressures
  to 0.1 hPa, and high up the balloon climbs tens of metres while the pressure falls that much,
  so runs of rows share one pressure. The rounded value is the pressure at about the middle of
  its run. In the example, 1,931 of the BUFR file's 5,851 rows are the other rows of such runs.
  The coded message has none.
- **Each such row that fits and lies above the last row kept**, higher and at a lower pressure.
  Rows that fall or float, a balloon coming down, fit but lie below, and are left out however
  many.

It leaves out:

- **A row that doesn't fit**, as above.
- **A row with a value missing or impossible**: the last row often has no wind, and a pressure
  below 0.1 hPa or above 1,200 hPa, a temperature outside −150 to 80 °C, a height outside −1 to
  60 km, a wind speed below zero or above 300 m/s, a humidity below zero or a direction beyond
  360° is dropped the same way.

The profile lists every row it left out, and why. More than 10 rows in a row that don't fit
refuse the answer. Then the row they follow is wrong (a ground pressure with a digit lost, 87.2
hPa for 872, would leave out every row up to 17.6 km), or all of them are (a block of heights
1 km off). A refused answer is not cached; the other version, or the sounding 12 hours earlier,
may read.

- **Humidity above 100%**, which radiosondes can report in cloud, is kept in the level as
  recorded and taken as 100% in the profile.
- **Between levels** the sounding works as for any other: the temperature and humidity are
  linear, and the pressure is hydrostatic (as the [atmosphere page](physics/atmosphere.md)
  explains). The wind is interpolated by its speed and direction; pass
  `WindInterpolation::Components` to interpolate its east and north parts instead, as
  [RocketPy](glossary.md#rocketpy) does. Above the top level the standard atmosphere continues,
  and the air is marked as extrapolated.

## An example

[`crates/hpr/examples/wyoming_sounding.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/wyoming_sounding.rs)
fetches the Santa Teresa sounding a launch at 15:30 UTC on 21 June 2025 would have had, in both
versions, and flies Calisto (one of the [example rockets](glossary.md#example-rockets)) from the
station without its parachutes. It makes these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. A real
   program passes `hpr_net::Http::new()` as the *transport* (what fetches) and keeps its cache in
   `Cache::platform_dir()`. This one never uses the network: a stand-in transport answers with
   the two versions recorded for the tests.
2. `WyomingRequest::latest_before("72364", launch_time)` picks the 12 UTC sounding, and
   `wyoming::fetch(&client, &request, now)` fetches and reads it. Setting the request's `version`
   to `WyomingVersion::Bufr` asks for the BUFR file instead.
3. `sounding.sounding(WindInterpolation::SpeedDirection)` makes the profile, and
   `profile.wind()` its wind.
4. `hpr_sim::Environment::new(earth, profile, wind)` puts both in a flight's environment.

It prints the ground and five standard levels, with heights in metres above sea level, and each
version's second row. Then it prints the air at the pad and above it next to the standard
atmosphere, and where Calisto is at apogee: east and north of the pad, negative for west and
south. Each flight starts on its own sounding's ground. The third flight keeps the BUFR file's
ground row and its rows from 1,438 geopotential metres up (the height of the coded message's
second row), dropping those between. Run it from a copy of the repository with
`cargo run --example wyoming_sounding -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/wyoming_sounding.output.txt -->
```text
Santa Teresa, New Mexico (72364), 2025-06-21 12 UTC
Sounding from the University of Wyoming's radiosonde archive
freshness: Fetched
released 58 minutes before 12 UTC
coded message: 227 levels kept, 1 left out
BUFR file: 3920 levels kept, 1931 left out

level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)
        872         1254               28.4             31          5.7        265   the ground
        850         1482               26.6             32         11.8        270
        700         3165               14.6             36          2.6        215
        500         5903               -5.3             63          8.2        180
        250        11002              -40.1              7         13.9        260
        100        16724              -73.1             13         13.9        255

second row       above the ground (m)   wind (m/s)   from (°)
coded message                     186         10.7        269
BUFR file                           8         11.1        264

height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)
     0 sounding                     872.0               28.4            1.0022
     0 standard                     871.4                6.9            1.0842
  1000 sounding                     778.5               22.3            0.9151
  1000 standard                     770.3                0.4            0.9811
  3000 sounding                     613.5                4.9            0.7664
  3000 standard                     596.5              -12.6            0.7977

Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)
coded message                              2848.1                -401.4       -23.4
BUFR file                                  2822.5                -561.3       -48.0
BUFR above 1,438 m                         2845.9                -419.1       -19.1
standard, calm                             2811.0                  -5.1         0.0
```

`freshness: Fetched` means the answer came from the transport, not the cache. That early
morning (6 a.m. local) was 21.5 °C warmer at the station than the standard atmosphere, and the
air was 7.6% less dense at the ground and 3.9% less dense 3 km up. Calisto climbs 1.3% higher in
it than in the standard atmosphere with no wind; the thinner air and the wind both play a part.

At apogee Calisto is about 400 m **west** of the pad, upwind: the wind blows from the west, and a
rocket just off the rail, still slow, turns into the wind
([weathercocking](glossary.md#weathercocking)) and flies that way. The calm flight's 5.1 m west
is Earth's rotation: a climbing rocket is pushed west (the Coriolis effect), as on the
[Launch-day weather](weather.md#an-example) page.

The two versions put Calisto's apogee 26 m apart, and the BUFR flight 160 m further west. Most of
that is the first 186 m of the climb. Both start from the station's wind at the ground, 5.7 m/s.
The coded message's second row, 186 m up, is 10.7 m/s from the west, so its wind grows steadily
over that climb. In the BUFR file the wind is already 11.1 m/s 8 m above the ground, where the
rocket is slowest and turns into the wind the most. Without its rows below 1,438 geopotential
metres the BUFR flight is 419 m west at apogee and peaks at 2,846 m, within 18 m and 2 m of the
coded message's. Which of the two is
nearer the wind a rocket meets is not measured: a balloon's first seconds of drift are not a
steady wind either.

## How it is checked

The tests in [`crates/hpr-net/tests/wyoming.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/wyoming.rs)
use three answers recorded from the archive: Santa Teresa on 21 June 2025 at 12 UTC in both
versions, and Salt Lake City on 15 January 2025 at 12 UTC, a winter sounding. No test uses the
network.

- The sounding is sampled at every kept row's height. It gives back the recorded pressure to a
  relative 1e-12, the temperature to 1e-9 K, the humidity to 1e-15 and the wind to 1e-9 m/s. The
  test reads the expected values from the recording itself, not through the code under test, and
  applies the rules above on its own.
- The rows left out are exactly those the rules leave out: the last row of each coded message (no
  wind), and 1,931 rows of the BUFR file (the other rows of a run). The coded messages keep 227
  and 241 rows, the BUFR file 3,920.
- Every BUFR row, kept or not, lies within 0.08 hPa of the profile's pressure at its height; the
  worst is 0.071 hPa. Keeping the first row of each run instead would miss by 0.093 hPa.
- The ground's wind, 5.7 m/s from 265°, is checked against its east and north parts worked out by
  hand: 5.678 m/s east and 0.497 m/s north.
- An edited file drops a row with a missing temperature; one with a humidity below zero, a
  temperature of −999 °C, a negative wind speed or pressure, or a direction of 361°; one that
  fits but lies below the last row kept (and not merely the row before); and a row at the
  ground's pressure. The rows left out are listed in line order. A humidity of 103% is kept and
  taken as 100%, and a wind from 360° reads as from north.
- Every row with a pressure, height and temperature in the three recordings fits the one before
  it, missing by at most 1 m beyond rounding (none in the coded messages, 0.97 m in the BUFR
  file). The test works out the thickness on its own, with the file's mixing ratio (grams of
  water vapour per kilogram of dry air) for the humidity. Unit tests pin the check to thicknesses
  and allowances worked out by hand, 0.01 m either side of the edge.
- Rows left out as not fitting, with the rest of the answer kept:
  - A pressure missing a digit (57 hPa at 5 km); the profile is the recording's without the row.
  - A height raised 850 m, and a height lowered to 4,000 m.
  - Two bad rows in a row, and two blocks of six raised rows with a good row between them.
  - A bad row after which the balloon bursts and falls back.
  - A row whose water vapour would outweigh its air (16 hPa at 30 °C and saturation).
- Rows kept or left out as they fit:
  - With no wind from 415 to 90.4 hPa (81 rows), every row after the gap fits and is kept.
  - Rows falling or floating at the end, and a float that crosses the last row kept, are only
    left out; so is a float followed by a row that fits, which is kept.
  - A row at 0.1 hPa and −150 °C, 55.6 km up, fits the top and makes a profile.
- Answers refused:
  - A block of 10 rows raised 1 km is left out; 11 refuse the answer.
  - A ground at 87.2 hPa (872 with a digit lost) or at 125 m (1,252 m with a digit lost), and a
    ground at 1,200 hPa, at −1,000 m or at −150 °C. With only 10 rows after the 87.2 hPa ground,
    all left out, the sounding is the ground alone; an 11th row refuses it.
- Values just past the bounds above are left out. Unit tests keep values at each bound; at the top
  row, temperatures of −150 and 80 °C and a wind of 300 m/s are kept and make a profile.
- The request's address is the one recorded, so a replayed answer fills the cache. A second
  request is answered from the cache, and offline mode answers without the network. A copy saved
  while the sounding was young is fetched again once it has settled.
- An answer that isn't the archive's text (such as an error page) is not saved: with no earlier
  copy the request fails, and with one, the earlier copy comes back, marked stale.
- Text with no header, a missing or doubled column, a unit other than the one required, a row
  with too few or too many fields, a field that isn't a number, a first row with no date, and a
  first row with a value missing are refused. A header or a row of a million commas, and more
  than 100,000 rows, are refused without being read further.
- The heights are geopotential, by the hypsometric check above: the test holds each average
  under 0.1% as geopotential metres, and beyond −0.4% as metres above sea level.

## What it leaves out

- How far a station's sounding is from the air over a launch site is not measured, in distance
  or in time. A rocket flown hours from the balloon, 100 km away, flies other air.
- Which version is nearer the truth near the ground, where they differ most, is not known.
- A level with any value missing is left out whole, even when its other values are good.
- The check catches a gross error in a row's pressure or height. It keeps:
  - A wrong wind, or a wrong humidity: the check doesn't use the wind, and humidity moves the
    thickness by only a few percent (1.6% for saturated air at 30 °C). Only a wind outside 0 to
    300 m/s is caught.
  - A wrong temperature that keeps the thickness within the allowance, and a height error within
    it: about 50 m on the example's thin layers.
  - A row whose pressure and height are both wrong yet fit each other.
  - A wrong ground with 10 rows or fewer after it, and a ground temperature that fits the first
    layer (80 °C instead of 28.4 °C does).
- Only the archive's comma-separated text is read, not its other formats, and there is no list
  of stations to search by place.
- Its terms of use are not stated; only soundings from U.S. stations, which are U.S. government
  works, are committed as test data
  ([ADR-120](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-120-university-of-wyoming-soundings-2026-09-30)).
- The command line's `hpr weather` (planned in [M5.2d](decisions-and-roadmap.md#m5-2d)) and the
  Python package don't fetch soundings yet.
