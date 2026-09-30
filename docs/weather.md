# Launch-day weather

This page covers fetching the weather over a launch site at launch time from
[Open-Meteo](https://open-meteo.com), a free weather service, and flying a rocket through it.
The weather arrives as a [sounding](glossary.md#sounding): temperature, pressure, humidity and wind
at a column of heights, from the ground up to about 24 km. It is for anyone who wants a flight in a
day's forecast instead of the [standard atmosphere](glossary.md#standard-atmosphere) with one wind.

**How far to trust it.** hpr turns Open-Meteo's answer into a profile that gives back the
pressure, temperature and wind of every level it keeps, to rounding error (checked on two
recorded answers, below). How good the forecast is depends on the weather model behind it, and nothing
here measures that: no flight has been flown in Open-Meteo's weather and compared with its log,
and no forecast has been compared with a weather balloon. The wind on the launch rail is the model's
wind 10 m above the ground.

Code: `hpr_net::open_meteo` ([API reference](api/hpr_net/open_meteo/index.html)), written for
the first weather increment, [M5.2a](decisions-and-roadmap.md#m5-2a). It needs the `net` feature
of the `hpr` crate. Weather from a file you download is on
[ERA5 weather files](format/era5.md). The choices are in
[ADR-119: Open-Meteo's pressure levels as a sounding](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-119-m52-split-open-meteos-pressure-levels-as-a-sounding-2026-09-30).

## What hpr asks for

Open-Meteo has two services with winds above the ground. Both give the output of numerical
weather models, hour by hour, on 19 *pressure levels*: heights named by their air pressure, from
1000 hPa (about sea level) to 30 hPa (about 24 km).

| service | covers | a saved answer stays fresh for |
|---|---|---|
| Forecast | about 16 days ahead and 3 months back | 1 hour |
| Historical forecast | the same forecasts, archived, for a past launch | 30 days |

hpr asks for the two whole hours around the launch time, in UTC. On each level it asks for the
temperature, relative humidity, wind speed and direction, and the level's
[geopotential height](glossary.md#geopotential-height). At the ground it asks for the pressure,
the temperature and humidity 2 m up, and the wind 10 m up. The request names the units, and hpr
refuses an answer in any other.

The answer goes through `hpr-net`'s cache ([Online data and the cache](online-data.md)), so a
second request inside the time in the table is answered from the disk, and offline mode answers
from the disk only. Open-Meteo's data is licensed CC BY 4.0: show the credit
"Weather data by Open-Meteo.com (CC BY 4.0)" wherever you show the weather. Every answer carries it.
The free service is for non-commercial use, under 10,000 calls a day. A self-hosted Open-Meteo
server can stand in for Open-Meteo's own: set the request's `endpoint`.

## How the answer becomes a sounding

- **The ground** is at the answer's elevation, with the ground pressure, the 2 m temperature and
  humidity, and the 10 m wind. Putting the 10 m wind at the ground makes it the wind on the launch
  rail, rather than jumping to the lowest level's wind above it
  ([Loft lesson L6](decisions-and-roadmap.md#l6): a forecast profile that stepped at its lowest
  level).
- **Levels below the ground are left out.** The weather models report all 19 levels everywhere,
  inventing values beneath high ground. A level is kept only when its pressure is below the ground
  pressure and its height is above the ground. A level with a missing value at either hour is left
  out too. The profile lists every level it left out, and why.
- **Heights.** Open-Meteo gives geopotential metres, which are converted to heights above sea
  level at the site's latitude with the World Meteorological Organization's formula (WMO-No. 8
  eq. 12.16, as the [atmosphere page](physics/atmosphere.md) explains). The two recorded
  answers bear this out. From 500 to 30 hPa, the height between each pair of levels matches the
  thickness the levels' temperatures and humidities give (the hypsometric equation) to
  −0.03% to −0.13% on average. Read as heights above sea level, the recorded heights would fall
  0.58% to 0.68% short.
- **Between the two hours** every value is linear in time. The wind is interpolated by its east
  and north parts, so a wind that turns through the hour takes the shorter way round.
- **Between levels** the sounding works as for any other: the temperature and humidity are
  linear, the pressure is hydrostatic, and the wind is interpolated by its speed and direction (or
  by its east and north parts, as RocketPy does, if you ask). Above the top level the standard
  atmosphere continues, and the air is marked as extrapolated.
- **Humidity** is taken as relative to liquid water. A model that reports it relative to ice at
  cold levels changes the density there by well under 0.1%, because the water vapour in air that
  cold is a tiny share of it.

## An example

[`crates/hpr/examples/open_meteo_weather.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/open_meteo_weather.rs)
asks for the weather over Spaceport America at 15:30 UTC (9:30 local) on 21 June 2025. It never
uses the network: a stand-in transport answers with the historical-forecast answer recorded for
the tests, which holds 15:00 and 16:00. A real program passes `hpr_net::Http::new()` instead, and
keeps its cache in `Cache::platform_dir()`. The program prints the ground and the levels below
6 km, then the air at the pad and above it next to the standard atmosphere. Last, it flies
RocketPy's Calisto in both, without its parachutes. Run it from a copy of the repository with
`cargo run --example open_meteo_weather -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/open_meteo_weather.output.txt -->
```text
Open-Meteo over 32.99° N, -106.97° E at 2025-06-21 15:30 UTC
Weather data by Open-Meteo.com (CC BY 4.0) (Fetched)

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

Calisto to apogee      apogee (m above the pad)   drift at apogee (m)
Open-Meteo                                 2880.9                 156.4
standard, calm                             2821.3                   5.1
```

The ground is at 1,400 m, so the five levels from 1000 to 900 hPa are left out. That June morning
was 24 °C warmer at the pad than the standard atmosphere, and the air was 8% less dense there and
4% less dense 3 km up. Calisto climbs 2.1% higher in it and drifts 156 m with the wind, which
turns from the south-southeast at the ground to the southwest by 2 km.

## How it is checked

The tests in [`crates/hpr-net/tests/open_meteo.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/open_meteo.rs)
use two answers recorded from Open-Meteo for Spaceport America: one from the historical-forecast
service (21 June 2025, 15:00 and 16:00 UTC) and one from the forecast service (2 October 2026,
18:00 and 19:00 UTC). No test uses the network.

- At each recorded hour, the sounding is sampled at every kept level's height. It gives back the
  recorded pressure to a relative 1e-12, the temperature to 1e-9 K and the wind to 1e-9 m/s. The
  expected values are read from the recording by the test itself, not by the code under test. The
  ground is checked the same way.
- Both answers leave out exactly the five levels below the 1,400 m ground, and keep 14.
- Half past the hour is the average of the two hours, and the wind is the average of its parts.
- The request's address is the one recorded, so a replayed answer fills the cache. A second
  request is answered from the cache, and offline mode answers without the network.
  [`tests/http.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/http.rs)
  does the same over HTTP, from a test server on the machine's own address.
- Text that is not JSON, Open-Meteo's error answer, a unit other than the one asked for, a
  missing field and a time outside the answer's hours are refused. A missing value at the ground
  is refused; on a level, the level is left out.
- The heights are geopotential, by the hypsometric check above.

## What it leaves out

- Nothing checks a forecast against the weather that came. Weather-balloon soundings from the
  University of Wyoming are planned next ([M5.2b](decisions-and-roadmap.md#m5-2b)), then NOAA's
  GFS and RAP model files ([M5.2c](decisions-and-roadmap.md#m5-2c)) and files you download
  yourself ([M5.2d](decisions-and-roadmap.md#m5-2d)).
- The 2 m temperature and humidity and the 10 m wind are placed at the ground itself.
- An answer with a relative humidity above 100% is refused, not trimmed.
- When Open-Meteo refuses a request (a place or date it doesn't cover), the error names the HTTP
  status but not Open-Meteo's reason, because the HTTP transport drops the body of a failed answer.
- Only one place and one launch time per request, and only Open-Meteo's own choice of model
  unless you name one.
- The command line's `hpr weather` (planned in [M5.2d](decisions-and-roadmap.md#m5-2d)) and the
  Python package don't fetch weather yet.
