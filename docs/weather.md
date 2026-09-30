# Launch-day weather

This page covers fetching the weather over a launch site at launch time from
[Open-Meteo](https://open-meteo.com), a free weather service, and flying a rocket through it.
The weather arrives as a [sounding](glossary.md#sounding): temperature, pressure, humidity and wind
at a column of heights, from the ground up to about 24 km. It is for anyone who wants a flight in a
day's forecast instead of the [standard atmosphere](glossary.md#standard-atmosphere) with one wind.

**How far to trust it.** hpr turns Open-Meteo's answer into a profile that gives back the
pressure, temperature, humidity and wind of every level it keeps, to rounding error. That is
checked on two recorded answers, below. How good the forecast is depends on the weather model
behind it, and nothing here measures that: no flight has been flown in Open-Meteo's weather and
compared with its log, and no forecast has been compared with a weather balloon. The wind on the
launch rail is the model's wind 10 m above the ground. The tests replay recorded answers; the live,
encrypted (HTTPS) connection to Open-Meteo was checked once by hand, not in CI.

Code: `hpr_net::open_meteo` ([API reference](api/hpr_net/open_meteo/index.html)), written for
the first weather increment, [M5.2a](decisions-and-roadmap.md#m5-2a). It needs the `net` feature
of the `hpr` crate. Weather from a file you download is on
[ERA5 weather files](format/era5.md), the air a weather balloon measured is on
[Weather-balloon soundings](soundings.md), and NOAA's own GFS and RAP forecasts are on
[NOAA forecasts: GFS and RAP](nomads.md). The choices are in
[ADR-119: Open-Meteo's pressure levels as a sounding](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-119-m52-split-open-meteos-pressure-levels-as-a-sounding-2026-09-30).

## What hpr asks for

Open-Meteo has two services with winds above the ground ([its documentation](https://open-meteo.com/en/docs)).
Both give the output of numerical weather models, hour by hour, on 19 *pressure levels*: heights
named by the air pressure there, in hectopascals (hPa; sea-level pressure is about 1013 hPa). They
run from 1000 hPa, about sea level, to 30 hPa, about 24 km up.

| service | covers | a saved answer stays fresh for |
|---|---|---|
| Forecast | about 16 days ahead and 3 months back | 1 hour |
| Historical forecast | the same forecasts, archived, for a past launch | 30 days |

hpr asks for the two whole hours around the launch time, in UTC (universal time, the time at
Greenwich). On each level it asks for the temperature, relative humidity, wind speed and
direction, and the level's [geopotential height](glossary.md#geopotential-height). At the ground
it asks for the pressure, the temperature and humidity 2 m up, and the wind 10 m up. The request
names the units, and hpr refuses an answer in any other. Wind directions are where the wind blows
from, clockwise from north, as weather services give them.

The answer goes through `hpr-net`'s cache ([Online data and the cache](online-data.md)), so a
second request within the "stays fresh" time above is answered from the disk, and offline mode
answers from the disk only. An answer hpr can't read is never saved, so it can't replace a good
copy. Open-Meteo's data is licensed
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/): show the credit "Weather data by
Open-Meteo.com (CC BY 4.0)" wherever you show the weather. Every answer carries it. The free
service is for non-commercial use, under 10,000 calls a day
([Open-Meteo's terms](https://open-meteo.com/en/terms)). A self-hosted Open-Meteo server can stand
in for Open-Meteo's own: set the request's `endpoint`.

## How the answer becomes a sounding

- **The ground** is at the answer's elevation, with the ground pressure, the 2 m temperature and
  humidity, and the 10 m wind. Putting the 10 m wind at the ground makes it the wind on the launch
  rail, rather than jumping to the lowest level's wind above it
  ([Loft lesson L6](decisions-and-roadmap.md#l6): a forecast profile that stepped at its lowest
  level).
- **Levels below the ground are left out.** The weather models report all 19 levels everywhere,
  inventing values beneath high ground. A level is kept only when its pressure is below the ground
  pressure and its height is above the ground. A level with a missing value at either hour is left
  out too, and so is one whose relative humidity is outside 0 to 100%. The profile lists every
  level it left out, and why.
- **Heights.** The models give a level's height in geopotential metres, which hpr converts to
  heights above sea level at the site's latitude with the World Meteorological Organization's
  formula (WMO-No. 8 eq. 12.16, as the [atmosphere page](physics/atmosphere.md) explains).
  Open-Meteo's documentation calls the value an altitude above sea level, so hpr checks which it
  is on the two recorded answers. Air pressure falls with height at a rate set by the air's
  temperature and humidity (the *hypsometric equation*), which fixes how thick each layer between
  two levels must be, in geopotential metres. From 500 to 30 hPa, the recorded layers match that
  to within 0.03% to 0.13% on average. Read as metres above sea level instead, they would be
  0.58% to 0.68% too thin. So they are geopotential metres.
- **Between the two hours** every value is linear in time. The wind is interpolated by its east
  and north parts, so a wind that turns through the hour takes the shorter way round.
- **Between levels** the sounding works as for any other: the temperature and humidity are
  linear, the pressure is hydrostatic, and the wind is interpolated by its speed and direction (or
  by its east and north parts, as RocketPy does, if you ask). Above the top level the standard
  atmosphere continues, and the air is marked as extrapolated.
- **Humidity** is taken as relative to liquid water. A model that reports it relative to ice at
  cold levels changes the density there by very little: the two differ by at most 27 Pa of water
  vapour (near −12 °C), which moves the density by under 0.03% at 400 hPa. Higher up the air is
  colder and the gap smaller: about 6 Pa at −40 °C, 0.08% even at 30 hPa.

## An example

[`crates/hpr/examples/open_meteo_weather.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/open_meteo_weather.rs)
asks for the weather over Spaceport America at 15:30 UTC (9:30 local) on 21 June 2025. It makes
five calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. The
   *transport* is what fetches ([Online data and the cache](online-data.md)). A real program
   passes `hpr_net::Http::new()` and keeps its cache in `Cache::platform_dir()`. This one never
   uses the network: a stand-in transport answers with the historical-forecast answer recorded
   for the tests, which holds 15:00 and 16:00.
2. `OpenMeteoRequest::new(latitude, longitude, time, OpenMeteoApi::HistoricalForecast)` says what
   to ask for, and `open_meteo::fetch(&client, &request, now)` fetches it and reads it at the
   launch time.
3. `profile.sounding(WindInterpolation::SpeedDirection)` makes the sounding, and
   `sounding.wind()` its wind.
4. `hpr_sim::Environment::new(earth, sounding, wind)` puts both in a flight's environment.

The program prints the ground and the levels below 6 km, then the air at the pad and above it next
to the standard atmosphere. Last, it flies RocketPy's Calisto (one of the
[example rockets](glossary.md#example-rockets)) in both, without its parachutes. Run it from a
copy of the repository with `cargo run --example open_meteo_weather -p hpr --features net`. It
prints:

<!-- quote: crates/hpr/examples/open_meteo_weather.output.txt -->
```text
Open-Meteo over 32.99° N, 106.97° W at 2025-06-21 15:30 UTC
Weather data by Open-Meteo.com (CC BY 4.0)
freshness: Fetched

level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)
      859.5         1400               29.6             18          3.3        162   the ground
        850         1482               28.2             16          2.9        189
        800         2014               23.4             16          2.8        225
        700         3161               14.5             28          6.8        236
        600         4439                3.3             60          9.8        219
        500         5895               -7.0             70          9.9        200
Below the ground, left out: 1000, 975, 950, 925, 900 hPa

height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)
     0 Open-Meteo                   859.5               29.6            0.9858
     0 standard                     856.0                5.9            1.0687
  1000 Open-Meteo                   765.2               20.4            0.9058
  1000 standard                     756.3               -0.6            0.9667
  3000 Open-Meteo                   602.9                3.6            0.7565
  3000 standard                     585.2              -13.6            0.7854

Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)
Open-Meteo                                 2880.9                  15.7      -155.6
standard, calm                             2821.3                  -5.1         0.0
```

`freshness: Fetched` means the answer came from the transport, not the cache; a second call within
the hour would say `Cached`. The ground is at 1,400 m, so the five levels from 1000 to 900 hPa are
left out. That June morning was 24 °C warmer at the pad than the standard atmosphere, and the air
was 8% less dense there and 4% less dense 3 km up. Calisto climbs 2.1% higher in it. At apogee
it is 156 m south of the pad (and 16 m east), upwind: the wind blows from the south-southeast at
the ground, turning to the southwest by 600 m above the pad (2,014 m above sea level), and a
rocket just off the rail, still slow, turns into the wind
([weathercocking](glossary.md#weathercocking)) and flies that way. The calm flight's 5.1 m west
is Earth's rotation: a climbing rocket is pushed west (the Coriolis effect), and with the rotation
turned off it drifts 0.006 m.

## How it is checked

The tests in [`crates/hpr-net/tests/open_meteo.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/open_meteo.rs)
use two answers recorded from Open-Meteo for Spaceport America: one from the historical-forecast
service (21 June 2025, 15:00 and 16:00 UTC) and one from the forecast service (2 October 2026,
18:00 and 19:00 UTC). No test uses the network.

- At each recorded hour, the sounding is sampled at every kept level's height. It gives back the
  recorded pressure to a relative 1e-12, the temperature to 1e-9 K and the wind to 1e-9 m/s, and
  holds the recorded humidity. The expected values are read from the recording by the test
  itself, not by the code under test. The ground is checked the same way.
- Both answers leave out exactly the five levels below the 1,400 m ground, and keep 14. Each half
  of the rule (the pressure, the height) leaves a level out on its own, in an edited answer.
- At 10 and 30 minutes past the hour, every value is the weighted average of the two hours, and
  the wind the weighted average of its parts. A wind from 350° then 10° is from due north at
  half past, and a direction never reads 360°.
- A humidity of 100% at both hours stays 100% at any time between; above 100%, a level is left out
  and the ground refused.
- The request's address is the one recorded, so a replayed answer fills the cache. A second
  request is answered from the cache, and offline mode answers without the network.
  [`tests/http.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/http.rs)
  does the same over HTTP, from a test server on the machine's own address.
- An answer with an hour's values missing is not saved: with no earlier copy the request fails,
  and with one, the earlier copy comes back, marked stale, and stays saved. A saved answer that
  can't be read at a later launch time in the same hour is fetched again online, and is the error
  offline.
- Text that is not JSON, Open-Meteo's error answer, a unit other than the one asked for, a missing
  field, hours outside 1970 to 9999, and a time outside the answer's hours are refused.
- The heights are geopotential, by the hypsometric check above; the test holds each average to
  the ranges quoted there.

## What it leaves out

- Nothing checks a forecast against the weather that came. Weather-balloon soundings, the
  measured air, can be fetched too ([Weather-balloon soundings](soundings.md)), but no forecast has
  been compared with one. NOAA's Global Forecast System and Rapid Refresh (GFS and RAP) can be
  fetched by name, or read from a whole GFS file you download ([NOAA forecasts: GFS and
  RAP](nomads.md)).
- The 2 m temperature and humidity and the 10 m wind are placed at the ground itself, so the
  whole launch rail sees the 10 m wind. A real wind is weaker close to the ground; how much that
  changes a rocket's turn into the wind off the rail is not measured.
- A relative humidity above 100% at the ground refuses the answer rather than trimming it.
- When Open-Meteo refuses a request (a place or date it doesn't cover), the error names the HTTP
  status but not Open-Meteo's reason, because the HTTP transport drops the body of a failed answer.
- Only one place and one launch time per request, and Open-Meteo's own choice of weather model
  unless you name one.
- The command line fetches it with [`hpr weather open-meteo`](cli.md#hpr-weather), but `hpr sim`
  doesn't fly it yet (issue [#265](https://github.com/nrdptel/hpr-sim/issues/265)), and the Python
  package doesn't fetch weather.
