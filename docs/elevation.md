# A launch site's elevation

This page covers looking up the height of the ground at a launch site from
[Open-Meteo](https://open-meteo.com), a free weather and terrain service. The answer is saved, so
the same lookup works later with no network, on a field with no signal. It is for anyone who needs
a site's height above sea level to start a flight at the right air pressure and density.

**How far to trust it.** hpr gives back Open-Meteo's number unchanged, and the cache gives it
back offline. That is checked on two recorded answers, below. The number is the height of a
terrain model whose cells are about 90 m across, rounded by Open-Meteo to whole metres. How close
that is to the ground under your launch rail depends on the terrain model, and nothing here
measures that. The tests replay recorded answers; the live, encrypted (HTTPS) connection to
Open-Meteo was used to record them, not in CI.

Code: `hpr_net::elevation` ([API reference](api/hpr_net/elevation/index.html)), written for
[M5.3b](decisions-and-roadmap.md#m5-3b), the second launch-site increment. It needs the `net`
feature of the `hpr` crate. The weather over a site is on [Launch-day weather](weather.md), and
the compass's offset from true north on
[The magnetic field and declination](physics/magnetic.md). The choices are in
[ADR-126: Open-Meteo's elevation through the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-126-m53b-open-meteos-elevation-through-the-cache-2026-09-30).

## What hpr asks for

Open-Meteo's elevation service ([its documentation](https://open-meteo.com/en/docs/elevation-api))
takes a list of places, each a latitude and longitude in degrees, and answers one height for each,
in the same order. hpr asks for one place or up to 100 in one request, the service's limit:

| what | value |
|---|---|
| address | `https://api.open-meteo.com/v1/elevation` |
| a request for two places | `?latitude=32.99,31.5&longitude=-106.97,35.5` |
| its answer | `{"elevation":[1400.0, -427.0]}` |
| a saved answer stays fresh for | a year |

The heights come from the Copernicus DEM GLO-90, a *digital elevation model* (DEM): a grid of
ground heights over the whole Earth, one every 3 seconds of arc (about 90 m), made by the European
Union's Copernicus programme from radar satellites. The ground doesn't move, so a saved answer
stays fresh for a year. Open-Meteo's own forecasts use the same model: the weather recorded for
Spaceport America ([Launch-day weather](weather.md)) gives the ground at 1,400 m, the height this
service answers there.

hpr refuses an answer with another number of heights than places asked for, a height that is not
a number, or a height outside −1,000 m to 9,000 m. The lowest land is the Dead Sea's shore, about
−430 m, and the highest Everest's summit, 8,849 m, so a height outside is a broken answer, such as
the value −32,768 that some terrain files use for "no data". An answer hpr refuses is never saved,
as on [Online data and the cache](online-data.md#what-it-promises).

## What the height means

The height is above mean sea level: the model's heights are measured from the EGM2008 geoid, the
shape the sea's surface would take if it were still. That is the
[height above sea level](glossary.md#height-above-sea-level-msl) the atmosphere is looked up by.
Over the open sea the model reads 0 m, and below sea level a negative height.

A flight's launch site takes its height above the [WGS 84](glossary.md#wgs-84) ellipsoid instead
(its [ellipsoidal height](glossary.md#ellipsoidal-height)). The two differ by the *geoid
undulation* `N`, the height of sea level above the ellipsoid there, which is between about −107 m
and +86 m around the world. hpr has no model of `N`. If you know it for your site, give the site
the height plus `N`, and the environment `N` with `with_geoid_undulation_m`. If you don't, give the
site the height as it is and leave `N` at 0. The atmosphere then sees the right height above sea
level, and only the site's place in space is off, by `N`. That moves gravity by about 3×10⁻⁴ m/s²
per 100 m, 0.003%.

## An example

[`crates/hpr/examples/site_elevation.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/site_elevation.rs)
looks up three places in one request: Spaceport America's launch area, the Dead Sea's shore and a
point in the open Atlantic. It makes these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. The
   *transport* is what fetches ([Online data and the cache](online-data.md)). A real program
   passes `hpr_net::Http::new()` and keeps its cache in `Cache::platform_dir()`. This one never
   uses the network: a stand-in transport answers with the answer recorded for the tests.
2. `ElevationRequest::new(places)` says what to ask for, and
   `elevation::fetch(&client, &request, now)` fetches the heights and saves them.
3. The same `elevation::fetch` a day later, through a client in `Mode::Offline` whose transport
   has no network, answers from the saved copy.
4. `Ussa76::standard().air(height)` gives the
   [standard atmosphere](glossary.md#standard-atmosphere) at each ground, and
   `Environment::standard(site)` puts a flight's start on the first.

Run it from a copy of the repository with
`cargo run --example site_elevation -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/site_elevation.output.txt -->
```text
Elevation data by Open-Meteo.com, from the Copernicus DEM GLO-90: © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved
first lookup: Fetched; offline a day later: Cached, the same heights: true

place                            lat (°)     ground (m)   pressure (hPa)  density / sea
Spaceport America, New Mexico      32.99           1400            856.0          0.872
the Dead Sea's shore               31.50           -427           1065.6          1.042
the open Atlantic                   0.00              0           1013.2          1.000

A flight from Spaceport America, New Mexico starts 1400 m above sea level, at 856.0 hPa.
```

The first line is the credit that Open-Meteo and the terrain model's licence ask for; show it
wherever the height is shown. At Spaceport America's 1,400 m the standard air is 13% thinner than
at sea level, so a rocket's drag there is 13% lower at the same speed.

## How it is checked

The tests in [`crates/hpr-net/tests/elevation.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/elevation.rs)
replay two answers recorded from the service on 1 October 2026, one for Spaceport America alone
and one for the three places above. They read the expected heights from the recordings
themselves, not through the code being tested, and check that:

- a lookup gives each recorded height exactly, in order, with the credit line;
- a second lookup online, inside the year, is answered from the saved copy without a fetch;
- offline, with a transport that fails the test if it is ever called, the same lookup gives the
  same heights, still fresh an hour later and marked stale a year later;
- offline, a place never looked up is an error naming its address;
- an answer with no heights is refused and not saved; a later good answer is saved, and when an
  answer that isn't JSON arrives after the year, the good copy comes back, marked stale with the
  reason.

The code's own tests check the address hpr builds, for one place and three, and that it refuses
no places, 101 places, a latitude or longitude out of range or not a number, and a bad server
address. They check that the reader takes negative heights and the range's edges, and refuses
each kind of broken answer above, Open-Meteo's own error message included.

## What it leaves out

- **No command.** The command line has no elevation command yet; a program calls the library.
- **No geoid model.** The height is above sea level, and turning it into a height above the
  ellipsoid needs the undulation `N`, which you give (above).
- **No accuracy check.** Nothing compares the model's heights with surveyed ones. The model's own
  makers state its accuracy; hpr hasn't measured it.
- **One number per place.** The answer is the model's height there, at about 90 m spacing; a pad
  on a hill or beside a cliff can sit metres off it. Open-Meteo answers in whole metres: all 100
  random places asked on 1 October 2026 came back whole.
- **Your own terrain file** (a GeoTIFF) is planned for
  [M5.3c](decisions-and-roadmap.md#m5-3c).
- **The cache key is the address,** so the same place written with other digits is looked up
  again.
