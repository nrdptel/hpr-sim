# NOAA forecasts: GFS and RAP

This page covers fetching the United States weather service's own forecasts over a launch site
and flying a rocket through them. NOAA's National Centers for Environmental Prediction (NCEP) run
two weather models whose output hpr reads:

- the **Global Forecast System (GFS)**, which covers the whole Earth on a grid a quarter of a
  degree apart (about 28 km), and
- the **Rapid Refresh (RAP)**, which covers the contiguous United States and nearby parts of
  Canada and Mexico on a grid 13 km apart, and is started again every hour.

hpr asks NOAA's download server, NOMADS (NOAA Operational Model Archive and Distribution System),
for a small piece of one forecast around the site. The piece arrives as a [GRIB2](glossary.md#grib2)
file, the World Meteorological Organization's binary format for weather on a grid, and hpr reads
it with its own decoder. The result is a [sounding](glossary.md#sounding): temperature, pressure,
humidity and wind at a column of heights, from the ground up to about 31 km (GFS) or 16 km (RAP).
You can also download a whole GFS file and read it offline, which goes up to about 79 km
([A whole GFS file](#a-whole-gfs-file)).
This page is for anyone who wants a flight in a named NOAA model's forecast, rather than
Open-Meteo's choice of model ([Launch-day weather](weather.md)) or a weather balloon's
measurement ([Weather-balloon soundings](soundings.md)).

**How far to trust it.**

- hpr's decoder gives every value in the two recorded files that ecCodes, the European weather
  centre's reference decoder, gives: all 6,123 values within 2.2e-16 of each other, relatively
  (one rounding in the last binary digit). The profile gives back those values, interpolated to
  the site, at every level it keeps. That is checked below.
- A whole GFS file you download yourself reads too ([A whole GFS file](#a-whole-gfs-file)): every
  one of the 746,770,303 values in one such file is within 4.4e-16 of ecCodes', relatively, and
  its profile at Spaceport America is the recorded cut's to 1.04e-7. That check was run by hand;
  CI checks eight of the file's messages.
- How good a forecast is depends on the model, and nothing here measures that: no forecast has
  been compared with a weather balloon or a flight log.
- The pad sits on the model's ground, which is smoothed: at Spaceport America it is 1,476 m in
  GFS and 1,429 m in RAP, where Open-Meteo gives 1,400 m
  ([Launch-day weather's example](weather.md#an-example)).
- The tests replay two recorded answers; the live connection to NOMADS is not tested in CI.

Code: `hpr_net::nomads` ([API reference](api/hpr_net/nomads/index.html)), with the decoder in
`hpr_io::grib2` ([API reference](api/hpr_io/grib2/index.html)), written for the third weather
increment, [M5.2c](decisions-and-roadmap.md#m5-2c), and extended to whole GFS files in
[M5.2d2](decisions-and-roadmap.md#m5-2d2). It needs the `net` feature of the `hpr` crate. The
choices are in
[ADR-121: GFS and RAP from NOMADS' grib filter, read by an in-house GRIB2 decoder](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-121-gfs-and-rap-from-nomads-grib-filter-read-by-an-in-house-grib2-decoder-2026-09-30)
and
[ADR-123: Complex packing, and a whole GFS file](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-123-complex-packing-and-a-whole-gfs-file-2026-09-30).

## What hpr asks for

A forecast comes from a *run*: the model starts from the weather observed at one hour, its
[cycle](glossary.md#forecast-run-cycle), and steps forward. Each *forecast hour* is the forecast
for that many hours after the cycle. Times are in UTC (universal time, the time at Greenwich) and
given in seconds since 1 January 1970 UTC (Unix time). Pressures are in hectopascals (hPa, 100 Pa;
sea-level pressure is about 1013 hPa). A *pressure level* is a height named by the pressure there.

| model | grid | runs start | forecast hours | pressure levels asked for |
|---|---|---|---|---|
| GFS | 0.25° of latitude and longitude, the whole Earth | every 6 hours (00, 06, 12, 18 UTC) | every hour to 120, then every third hour to 384 | 28, from 1000 to 10 hPa (about 31 km) |
| RAP | 13 km, on a [Lambert conformal](glossary.md#lambert-conformal-projection) map of the contiguous United States and nearby parts of Canada and Mexico (not Alaska or Hawaii) | every hour | every hour to 21; to 51 only from the 03, 09, 15 and 21 UTC runs | 37, from 1000 to 100 hPa every 25 (about 16 km) |

`NomadsRequest::new(latitude, longitude, model, cycle, forecast_hour)` says what to ask for; it
can't fail. The request's `url()`, and so `nomads::fetch`, refuse a cycle the model doesn't run
or a forecast hour its run doesn't have (`NomadsModel::has_forecast_hour(cycle_hour, hour)` says
which it has). hpr picks neither for you: to fly at 18 UTC, ask for GFS's 00 UTC run at hour 18,
say, or RAP's 12 UTC run at hour 6. A site outside the model's grid is refused once the answer arrives.

The request's address is NOMADS' *grib filter*, a web form that cuts chosen variables, levels and
a box of latitude and longitude out of a run's file. hpr asks for a box 0.3° each way around the
site, which holds 9 GFS or about 25 RAP [grid points](glossary.md#grid-point), among them the four
around the site: about 28 KB for GFS and 40 KB for RAP. It asks for:

| variable | where |
|---|---|
| [geopotential height](glossary.md#geopotential-height) | the ground (the model's terrain) and each pressure level |
| pressure | the ground |
| temperature and relative humidity | 2 m above the ground and each pressure level |
| wind, as two components | 10 m above the ground and each pressure level |

GRIB2 fixes each variable's unit, so there are no units to check. The filter also sends the
ground's own (skin) temperature, since it asks for temperature at the surface; hpr decodes it but
doesn't use it. With it, a cut holds 147 GFS or 192 RAP
[messages](glossary.md#grib2), one variable on one level each. A cut of more than 1,000 is
refused unread.

The answer goes through `hpr-net`'s cache ([Online data and the cache](online-data.md)), and
`nomads::fetch(&client, &request, now)` takes the time now, in Unix seconds, to judge how fresh a
saved copy is. A run's files don't change once NOAA writes them, so a saved copy stays fresh for
30 days. NOMADS keeps only recent runs; on 2026-09-30 its grib filters listed 10 days of GFS
runs and 2 of RAP, so a past launch needs a copy saved while NOMADS still had it. A run appears
on NOMADS some hours after its start time, and NOMADS limits how often one may ask, so keep to
that. Offline mode answers from the disk only. Only an
answer that decodes, makes a sounding, and is for the run and hour asked for is saved, so a bad
answer can't replace a good copy.

NCEP's forecasts are U.S. government works, free of copyright. Every answer carries the credit
"Forecast data from NOAA/NCEP (GFS, RAP), via NOMADS"; show it wherever you show the forecast.

## How the answer becomes a sounding

**Between grid points.** A model gives each value only at its grid points. hpr takes the four
around the site and blends them *bilinearly*: each point's weight grows as the site nears it, in
the grid's own rows and columns, and the four weights add up to 1. A cut whose grid steps and
projection aren't the model's (GFS's 0.25° steps, RAP's 13,545 m cells about 265° E) is refused
and not saved.

**The ground** is at the model's terrain height at the site, with the surface pressure, the 2 m
temperature and humidity, and the 10 m wind. Putting the 10 m wind at the ground makes it the wind
on the launch rail, rather than jumping to the lowest level's wind above it
([Loft lesson L6](decisions-and-roadmap.md#l6): a forecast profile that stepped at its lowest
level). This is what Open-Meteo's page does too.

**Levels below the ground are left out.** The models give every level everywhere, inventing
values beneath high ground. A level is kept only when its pressure is below the ground's and its
height is above the ground. A level with a value missing at any of the four grid points is left
out too. At Spaceport America, about 1,400 m up, six levels are underground in each model:
1000 to 850 hPa in GFS and 1000 to 875 hPa in RAP. GFS keeps 22 of its 28 levels and RAP 31 of
its 37. The profile lists every level it left out, and why.

**Heights.** GRIB2 gives heights in geopotential metres, which hpr converts to heights above sea
level at the site's latitude with the World Meteorological Organization's formula (WMO-No. 8 eq.
12.16, as the [atmosphere page](physics/atmosphere.md) explains). The model's terrain height is also given in
geopotential metres and converted the same way. At this latitude that adds about 2 m: GFS's
1,474.4 gpm becomes 1,476.4 m. If the model's terrain were really a height above sea level, as for
[Open-Meteo's elevation](weather.md#how-the-answer-becomes-a-sounding), the ground would sit about
2 m too high.

**RAP's winds are turned to east and north.** RAP's map is a cone unrolled flat (a
[Lambert conformal](glossary.md#lambert-conformal-projection) projection), and it gives each wind
as two components along its map's rows and columns, not east and north. "Up" on that map points
true north only along one line of longitude, 95° W. Elsewhere the lines of longitude lean towards
the map's centre, so the map's "up" is turned from true north by an angle that grows with the
distance from 95° W:

```text
θ = n (λ − λ₀),   n = sin 25°,   λ₀ = 265° (95° W)
```

Here `λ` is the site's longitude, and 25° N is where RAP's cone touches the Earth. At Spaceport
America, 106.97° W (253.03° E), `θ = 0.4226 × (253.03° − 265°) = −5.06°`: the map's "up" points
5.06° west of true north. hpr turns the wind by that angle, with `u` and `v` the components along
the map's rows and columns:

```text
east  =  u cos θ + v sin θ
north = −u sin θ + v cos θ
```

Left unturned, a 20 m/s wind would be about 1.8 m/s off sideways (20 m/s × sin 5.06°). The
components are blended between grid points first and turned once, at the site; across one 13 km
cell `θ` changes by about 0.06°. GFS gives its winds east and north already.

**Humidity** is taken as relative to liquid water, as for Open-Meteo. Whether NCEP's models give
it relative to ice at cold levels is not settled here; if they do, the density there moves by
under 0.1% ([Launch-day weather](weather.md#how-the-answer-becomes-a-sounding) explains the
bound). A humidity above 100% is kept in the level as given and taken as 100% in the sounding.

**Between levels** the sounding works as for any other: the temperature and humidity are linear,
and the pressure is hydrostatic. The wind is interpolated by its speed and direction; pass
`WindInterpolation::Components` to interpolate its east and north parts instead, as
[RocketPy](glossary.md#rocketpy) does.

## An example

[`crates/hpr/examples/nomads_forecast.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/nomads_forecast.rs)
fetches both models' forecasts for Spaceport America at 18 UTC (noon local) on 30 September 2026:
GFS's 00 UTC run at hour 18 and RAP's 12 UTC run at hour 6. It flies Calisto (one of the
[example rockets](glossary.md#example-rockets)) through each, without its parachutes. It makes
these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. A real
   program passes `hpr_net::Http::new()` as the *transport* (what fetches) and keeps its cache in
   `Cache::platform_dir()`. This one never uses the network: a stand-in transport answers with
   the two files recorded for the tests.
2. `NomadsRequest::new(latitude, longitude, NomadsModel::Gfs, cycle, 18)` says what to ask for
   (and `NomadsModel::Rap` for RAP), and `nomads::fetch(&client, &request, now)` fetches it and
   reads it at the site. Here it reads the two recorded forecasts, as a real program would fetch
   them.
3. `profile.sounding(WindInterpolation::SpeedDirection)` makes the sounding, and
   `sounding.wind()` its wind.
4. `hpr_sim::Environment::new(earth, sounding, wind)` puts both in a flight's environment.

It prints each model's ground and the standard levels 850, 700, 500, 250 and 100 hPa that lie
above it, with heights in metres above sea level and the wind's direction as where it blows from,
clockwise from north. Then it prints where Calisto is at apogee: east and north of the pad,
negative for west and south. Each flight starts on its own model's ground; the standard-atmosphere
flight starts on GFS's. Run it from a copy of the repository with
`cargo run --example nomads_forecast -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/nomads_forecast.output.txt -->
```text
Spaceport America, 2026-09-30 18 UTC
Forecast data from NOAA/NCEP (GFS, RAP), via NOMADS
freshness: Fetched
GFS: run of 00 UTC, hour 18: 22 levels kept, 6 left out
RAP: run of 12 UTC, hour 6: 31 levels kept, 6 left out
RAP's winds turned by -5.06° to east and north

GFS   level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)
              848         1476               19.3             42          5.5        242   the ground
              700         3077                2.8             90          7.3        262
              500         5720              -12.9             55         17.0        246
              250        10822              -32.7              1         45.8        230
              100        16741              -70.6             17         17.7        209

RAP   level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)
              854         1429               16.9             70          3.7        246   the ground
              850         1463               16.1             67          4.4        249
              700         3076                2.6             77          8.1        272
              500         5722              -12.0             39         20.5        242
              250        10822              -32.6              1         42.9        231
              100        16739              -70.8             10         13.0        211

Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)
GFS                                        2842.8                -253.0      -131.7
RAP                                        2841.3                -204.3       -83.9
standard, calm                             2826.9                  -5.1         0.0
```

`freshness: Fetched` means the answer came from the transport, not the cache; a second call within
30 days would say `Cached`. GFS has no 850 hPa row because its ground, at 848 hPa, is above that
level. The two models agree closely aloft: at 500 hPa their heights are 2 m apart and their
temperatures 0.9 °C. Near the ground they differ more: GFS's ground is 47 m higher and 2.4 °C
warmer, and its 10 m wind 1.8 m/s stronger.

Calisto climbs 14 to 16 m higher in either forecast than in the standard atmosphere with no wind.
At apogee it is west-southwest of the pad, upwind: the wind at the ground blows from the
west-southwest (242° and 246°), and a rocket just off the rail, still slow, turns into the wind
([weathercocking](glossary.md#weathercocking)) and flies that way. GFS's stronger wind near the
ground turns it further into the wind; it ends 49 m further west and 48 m further south than in
RAP's forecast. How much of that comes from the ground's wind and how much from the winds aloft is
not separated. The calm flight's 5.1 m west
is Earth's rotation: a climbing rocket is pushed west (the Coriolis effect), as on the
[Launch-day weather](weather.md#an-example) page. Which model is nearer the air Calisto would have
met is not measured.

## A whole GFS file

Instead of asking NOMADS for a cut, you can download a whole GFS file and read it offline, for
example before driving out to a launch with no signal. NOMADS keeps the most recent runs, and
NOAA's open data bucket on Amazon's cloud older ones. A file holds one run's forecast for one hour
over the whole Earth, about 550 MB:

```text
https://nomads.ncep.noaa.gov/pub/data/nccf/com/gfs/prod/gfs.20260930/00/atmos/gfs.t00z.pgrb2.0p25.f018
https://noaa-gfs-bdp-pds.s3.amazonaws.com/gfs.20260930/00/atmos/gfs.t00z.pgrb2.0p25.f018
```

Here `20260930/00` and `t00z` are the run (00 UTC on 30 September 2026) and `f018` the forecast
hour. Only the 0.25° files (`pgrb2.0p25`) read: the smaller 0.5° and 1° files are refused, since
hpr checks the file is on GFS's 0.25° grid. The [command line](cli.md#hpr-weather) reads the file
as it reads a saved cut, with the site you want:
`hpr weather gfs --latitude 32.99 --longitude -106.97 --cycle 2026-09-30T00Z --hour 18 --from
gfs.t00z.pgrb2.0p25.f018 --output profile.json`. It took about a third of a second on a Mac.

NCEP packs whole files more tightly than the grib filter's cuts, with GRIB2's *complex packing*:
the values are split into groups, each with its own smallest value and number of bits, and most
fields store the differences between neighbouring values rather than the values (*spatial
differencing*). GRIB2 numbers these packings as templates 5.2 and 5.3; hpr's decoder reads both.
A whole file also holds 743 [messages](glossary.md#grib2) where a cut holds 147:

- totals and averages over a time span, such as the rain that fell in the last six hours, which
  hpr reads but the profile doesn't use;
- values for a layer between two heights, such as the humidity from the ground to mid-air, which
  hpr skips because they don't belong to one level;
- every pressure level up to 0.01 hPa, so the profile goes on above 10 hPa, where a cut stops.
  In this file the 0.01 hPa level is 79.2 km up. The highest levels are near the top of the
  model, and nothing here checks how good its forecast is there.

The grid runs all the way round the Earth, so hpr joins its last column to its first: a site
between 359.75° and 0° of longitude still has four grid points around it (checked by
`a_grid_round_the_earth_wraps_its_columns` in `crates/hpr-net/tests/nomads.rs`).

For that run and hour at Spaceport America, the whole file's profile has the cut's 22 levels,
within 1.04e-7 of each value, relatively, and 13 more above them. Why they differ at all: NOMADS
packs its cuts again in fewer bits. Read with ecCodes, without hpr, the two files already differ
by up to 2.4e-6, relatively, at the cut's grid points.

## How it is checked

The tests in [`crates/hpr-net/tests/nomads.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/nomads.rs)
use the two files the example reads, recorded unchanged from NOMADS' grib filter on 30 September
2026 for Spaceport America (32.99° N, 106.97° W): GFS's 00 UTC run at hour 18 (27,586 bytes) and
RAP's 12 UTC run at hour 6 (40,026 bytes), both for 18 UTC. No test uses the network. The expected
values come from [ecCodes](https://confluence.ecmwf.int/display/ECC) 2.49.0, the European Centre
for Medium-Range Weather Forecasts' decoder, run on the same files by
[`validation/oracles/grib2/eccodes_dump.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/grib2/eccodes_dump.py)
and committed as `crates/hpr-net/tests/fixtures/nomads-eccodes.json`. The tests read that file
directly, not through the decoder under test.

Each line gives what was measured, then the bound the test holds it to.

All the measurements below were made on macOS; CI runs the same tests with the same bounds on
Linux and Windows.

- **Every value.** Each of the 147 GFS messages (9 grid points each) and 192 RAP messages (25 grid
  points each) names the same variable, level and times as ecCodes, and all 6,123 values are within
  2.2e-16 of ecCodes', relatively (bound 2.5e-16). That is one rounding: ecCodes multiplies by an
  inexact `10^−D` where hpr divides by an exact `10^D`. It uses only basic arithmetic, which is the
  same on every platform.
- **Every grid point's position** is within 5.7e-14° of ecCodes' (bound 1e-12°, since
  other platforms' maths libraries can differ in the last digits). The four points' weights add
  up to 1, and the blend of their positions is the site, to 1e-5°.
- **RAP's wind turn** is `sin 25° × (λ − 265°)`, −5.06° at the site. ecCodes' own positions of two
  neighbouring grid points show the map's rows bearing 90° + θ from north, to 2.2e-6° (bound
  3e-6°), which pins the sign. GFS's winds are not turned.
- **The profile.** The sounding is sampled at the ground and every kept level's height. It gives
  back ecCodes' values interpolated to the site: the pressure within a relative 1e-13, the
  temperature within 1e-11 K and the wind within 1e-11 m/s, and each level holds the humidity
  within 1e-14. The six underground levels of each model are the ones left out.
- **The heights are geopotential metres.** From 500 hPa up, each layer's thickness matches what
  the [hypsometric equation](glossary.md#hypsometric-equation) gives it when the file's heights are
  read as geopotential metres, on average to −0.002% (GFS, 15 layers to 10 hPa) and −0.08% (RAP,
  16 layers to 100 hPa). Read as metres above sea level, the layers would be 0.63% and 0.51% too
  thin. The test holds each average to those figures as rounded.
- **The two models read alike** for the same hour: their 2 m temperatures within 3 K, their
  500 hPa heights within 30 m and their 500 hPa winds within 5 m/s. That is a guard against a
  misread file, not a check of either forecast.
- **The cache.** The request's address is the one recorded, so a replayed answer fills the cache.
  A second request is answered from the cache, and offline mode answers without the network, 40
  days later, [marked stale](online-data.md#what-it-promises).
- **Wrong answers aren't saved.** A RAP request answered with a GFS cut, and a cut of another run
  or hour than the one asked for, are refused and not saved. A GFS cut whose steps are changed to
  0.5° still reads, but is not GFS's grid, so it would be refused too.
- **Humidity.** A humidity over 100% is kept in the level and taken as 100% in the sounding (a
  unit test).
- **Missing and hostile data.** A level whose field has no value at one of the four points around
  the site is left out as missing data; a cut of more than 1,000 fields is refused before it is
  read (a real one has 147 or 192).
- **A whole GFS file** (the 00 UTC run of 30 September 2026 at hour 18, 550 MB, not committed):

  | check | where it runs | measured | bound |
  |---|---|---|---|
  | every one of its 746,770,303 values against ecCodes', relatively | a script, run once outside CI | 4.4e-16 | none |
  | its 24,642,017 grid points without a value are ecCodes' | the same script | all the same | all |
  | eight whole messages cut from it, one of each kind: which field each is, its points without a value, and every 997th value | CI | 4.4e-16 or less | 4.5e-16 |
  | the same eight messages: two sums over all their values | CI | within the bound | 1.35e-15 of the terms' sizes |
  | `hpr weather` writes the cut's profile from it, relatively | a test skipped in CI, which doesn't have the file | 1.04e-7 | 1.1e-7 |

  The script is
  [`validation/oracles/grib2/whole_file.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/grib2/whole_file.py);
  its record, with ecCodes' survey of the file, is
  [`gfs-whole-file.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/grib2/gfs-whole-file.json).
  The sums catch a value decoded wrong anywhere in a message: off by a single packing step, a
  value would move a sum at least 23,000 times more than the bound allows.
- **Complex packing in CI.** The recorded GFS cut, packed again by ecCodes with complex packing
  and both orders of differencing
  ([`repack.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/grib2/repack.py)),
  goes through every check above: its 1,323 values equal ecCodes' (bound 2.5e-16), and its profile
  matches at every level. `hpr weather` writes its profile, which is the cut's within 6.9e-8,
  relatively (bound 7e-8).
- **Refusals.** A site outside the file's grid, text that isn't GRIB2, a cycle the model doesn't
  run and a forecast hour its run doesn't have are refused. The decoder's own tests build small
  files by hand to pin the unpacking formula, bitmaps (masks of grid points with no value), both kinds of grid,
  complex packing's groups, missing values and both orders of differencing, and refusals of every
  other kind of file by name.

## What it leaves out

- **JPEG 2000.** Some GRIB2 files are compressed with JPEG 2000 (template 5.40), which hpr refuses
  by name for now; reading it is planned in [M5.2d3, JPEG 2000](decisions-and-roadmap.md#m5-2d3).
  Whole GFS files use complex packing, which hpr reads. RAP's whole files have not been tried.
- **Coarser GFS files.** GFS's 0.5° and 1° files are refused: hpr checks for the 0.25° grid.
- **Forecast accuracy.** Nothing here checks a forecast against the weather that came.
- **Time.** One forecast hour per request, with no interpolation between hours: you pick the run
  and the hour closest to your launch.
- **Above the top level.** RAP stops at 100 hPa, about 16 km, and GFS as asked for here at
  10 hPa, about 31 km (a whole GFS file goes on to 0.01 hPa, about 79 km, and the layer
  thickness check above stops at 10 hPa). Above that the sounding continues as the
  [standard atmosphere](glossary.md#standard-atmosphere), shifted to pass through the top level's
  temperature and pressure, with the top level's share of water vapour (capped where the air
  saturates). The wind holds the top level's wind. Both are marked as extrapolated. A flight above
  16 km in RAP's forecast is flying on that guess.
- **Underground grid points.** A kept level is blended from the four grid points even where one
  of them has that level underground. In the RAP cut, 850 hPa takes 13% of its weight from a point
  whose ground is at 845.8 hPa; the effect here is about 0.02 K, and larger in steep terrain.
- **The ground.** The pad is placed on the model's smoothed terrain, not the site's real height;
  hpr doesn't shift the profile to the real ground.
- **Humidity over ice.** Whether NCEP gives humidity over ice at cold levels is unsettled here; it
  moves the density by under 0.1%.
- **The live connection** to NOMADS is not tested in CI, and nothing tells you which runs NOMADS
  still holds before you ask.
- The command line fetches them with [`hpr weather gfs` and `rap`](cli.md#hpr-weather), but
  `hpr sim` doesn't fly them yet (issue [#265](https://github.com/nrdptel/hpr-sim/issues/265)), and
  the Python package doesn't fetch NOAA's forecasts.
