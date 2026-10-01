# A launch site's elevation

This page covers looking up a launch site's elevation, its altitude above sea level (the field
elevation a club lists), from [Open-Meteo](https://open-meteo.com), a free weather and terrain
service. The answer is saved, so the same lookup works later with no network, on a field with no
signal. It is for anyone who needs a site's height above sea level to start a flight at the right
air pressure and density. There is no `hpr` command for it yet: a Rust program calls the library.

**How far to trust it.** hpr gives back Open-Meteo's number unchanged, and the saved copy gives
it back offline. That is checked on two recorded answers, below. The number comes from a terrain
model whose cells are about 90 m across, and it is a *surface* height: over trees or buildings it
sits above the bare ground. The model's makers state its accuracy as better than 4 m for 90% of
points, outside Antarctica and Greenland; hpr hasn't measured it. In the standard atmosphere,
10 m of height error changes the air's density by about 0.1%. The recorded heights are whole
metres; Open-Meteo doesn't document its rounding.

The tests replay two saved answers and never contact Open-Meteo; the live, encrypted (HTTPS)
connection was used only to record them, by hand.

Code: `hpr_net::elevation` ([API reference](api/hpr_net/elevation/index.html)), written for
[M5.3b](decisions-and-roadmap.md#m5-3b), the second launch-site increment. It needs the `net`
feature of the `hpr` crate. The weather over a site is on [Launch-day weather](weather.md), and
the compass's offset from true north on
[The magnetic field and declination](physics/magnetic.md). The choices are in
[ADR-126: Open-Meteo's elevation through the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-126-m53b-open-meteos-elevation-through-the-cache-2026-09-30).

## What hpr asks for

Open-Meteo's elevation service ([its documentation](https://open-meteo.com/en/docs/elevation-api))
takes a list of places, each a latitude and longitude in degrees, and answers one height for each,
in the same order. hpr asks for one place or up to 100 in one request, the service's limit. This
is a request recorded for the tests, for Spaceport America, the Dead Sea and a point in the open
Atlantic:

| what | value |
|---|---|
| address | `https://api.open-meteo.com/v1/elevation` |
| the request's places | `?latitude=32.99,31.5,0&longitude=-106.97,35.5,-30` |
| its answer | `{"elevation":[1400.0, -427.0, 0.0]}` |
| a saved answer stays fresh for | a year |

hpr writes each coordinate to 5 decimal places, about 1 m on the ground. So a place your program
keeps in radians and turns back into degrees, which can change its last digits, still finds its
saved answer.

**A saved answer is found again only by the same request:** the same places, in the same order.
If you look up three club fields in one request at home, a later lookup of one of them alone is a
new request, and offline it fails. To use a site offline, look it up alone, or repeat the exact
list you used before.

### The terrain model

The heights come from the Copernicus DEM GLO-90, a *digital elevation model* (DEM): a grid of
heights over the whole Earth, made by the European Union's Copernicus programme from radar
satellites. Its points are 3 seconds of arc apart north to south (about 90 m); east to west the
spacing widens above 50° of latitude, to keep cells about 90 m across. It is a *digital surface
model*: its heights include buildings and vegetation
([the dataset's readme](https://copernicus-dem-30m.s3.amazonaws.com/readme.html)). Its stated
accuracy is under 4 m for 90% of points, outside Antarctica and Greenland
([product handbook](https://dataspace.copernicus.eu/sites/default/files/media/files/2024-06/geo1988-copernicusdem-spe-002_producthandbook_i5.0.pdf),
issue 5.0, Table 1, page 9).

The ground doesn't move, so a saved answer stays fresh for a year. Open-Meteo's own forecasts use
the same model: the weather recorded for Spaceport America ([Launch-day weather](weather.md))
gives the ground at 1,400 m, the height this service answers there.

### What hpr refuses

hpr refuses an answer with another number of heights than places asked for, a height that is not
a number, or a height outside −1,000 m to 9,000 m. The lowest land, by the Dead Sea, is about
−440 m, and the highest, Everest's summit, 8,849 m, so a height outside is a broken answer, such as
the value −32,768 that some terrain files use for "no data". An answer hpr refuses is never saved,
as on [Online data and the cache](online-data.md#what-it-promises).

When Open-Meteo itself refuses a request (it answers with HTTP status 400 and a reason), the
HTTP transport reports a failed fetch naming the status, without Open-Meteo's reason. hpr checks
each place's latitude and longitude before asking, so that is rare.

## What the height means

The height is above mean sea level. The model's heights are measured from EGM2008, a worldwide
model of the *geoid*: the shape the sea's surface would take if it were still, extended under the
land ([product handbook](https://dataspace.copernicus.eu/sites/default/files/media/files/2024-06/geo1988-copernicusdem-spe-002_producthandbook_i5.0.pdf),
section 1.2.1, page 12). That is the [height above sea level](glossary.md#height-above-sea-level-msl)
the atmosphere is looked up by ([Atmosphere: the height datum](physics/atmosphere.md#height-datum)).
The sea has no tiles in the model and reads 0 m, so 0 m can also mean "no data". The Dead Sea
reads −427 m, its surface when the radar satellites measured it, between 2010 and 2015; the lake has
fallen since.

A flight's launch site takes its height above the [WGS 84](glossary.md#wgs-84) ellipsoid instead
(its [ellipsoidal height](glossary.md#ellipsoidal-height)). The two differ by the *geoid
undulation* `N`, the height of sea level above the ellipsoid there: between about −107 m and
+86 m around the world. hpr has no model of `N`; a geoid calculator for EGM2008 gives it for a
place. Then:

- **If you know `N`,** place the site with `Geodetic::from_degrees(latitude, longitude, H + N)`,
  where `H` is the height from this page, and give the flight's environment `N` with
  `Environment::standard(site)?.with_geoid_undulation_m(N)`. For example, with `H` = 1,400 m and
  an `N` of −25 m, the site's ellipsoidal height is 1,375 m.
- **If you don't,** place the site at `H` and leave `N` at 0, as the example below does. The
  atmosphere still sees the right height above sea level, so the air, the drag and the flight are
  right. The site's place in space is off by `N`, which moves gravity by about 3×10⁻⁴ m/s² per
  100 m, 0.003% (normal gravity's [height term](physics/gravity.md#formulas)). A flight exported as KML keeps its
  heights above sea level, which stay right; the GeoJSON export's heights are above the ellipsoid,
  and are off by `N`.

## An example

[`crates/hpr/examples/site_elevation.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/site_elevation.rs)
looks up the three places above in one request. It makes these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. The
   *transport* is what fetches ([Online data and the cache](online-data.md)). A real program
   passes `hpr_net::Http::new()`, and keeps its saved answers in `Cache::platform_dir()`, the
   system's usual folder for them. This one never uses the network: a stand-in transport answers
   with the answer recorded for the tests.
2. `ElevationRequest::new(places)` says what to ask for, and
   `elevation::fetch(&client, &request, now)` fetches the heights and saves them. Each comes back
   with its place, as `height_msl_m`, the height above mean sea level.
3. The same `elevation::fetch` a day later, through a client in `Mode::Offline` whose transport
   has no network, answers from the saved copy.
4. `Ussa76::standard().air(height)` gives the
   [standard atmosphere](glossary.md#standard-atmosphere) (the 1976 US Standard Atmosphere) at
   each height, and `Environment::standard(site)` puts a flight's start on the first.

Run it from a copy of the repository with
`cargo run --example site_elevation -p hpr --features net`. It prints:

<!-- quote: crates/hpr/examples/site_elevation.output.txt -->
```text
Elevation data by Open-Meteo.com (CC BY 4.0), from the Copernicus DEM GLO-90: © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved
first lookup: Fetched; offline a day later: Cached, the same heights: true

place                            lat (°)     height (m)   pressure (hPa)  density / sea
Spaceport America, New Mexico      32.99           1400           856.02          0.872
the Dead Sea (its surface)         31.50           -427          1065.61          1.042
the open Atlantic                   0.00              0          1013.25          1.000

A flight from Spaceport America, New Mexico starts 1400 m above sea level, at 856.02 hPa.
```

The first line is the credit that Open-Meteo's licence and the terrain model's licence ask for;
show it wherever the height is shown. `Fetched` and `Cached` say where an answer came from
([how long a copy stays fresh](online-data.md#how-long-a-copy-stays-fresh)). At Spaceport
America's 1,400 m the standard air is about 13% thinner than at sea level, so at the same speed
and drag coefficient the drag there is about 13% lower.

## How it is checked

The tests in [`crates/hpr-net/tests/elevation.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/elevation.rs)
replay two answers recorded from the service on 1 October 2026 (UTC), one for Spaceport America
alone and one for the three places above. They read the expected heights from the recordings
themselves, not through the code being tested, and check that:

- a lookup gives each recorded height exactly, in order, each with the place asked for, and with
  the credit line;
- a second lookup online, inside the year, is answered from the saved copy without a fetch;
- offline, with a transport that fails the test if it is ever called, the same lookup gives the
  same heights, still fresh an hour later and marked stale a year later;
- offline, a place never looked up is an error naming its address;
- an answer with no heights is refused and not saved; a later good answer is saved, and when an
  answer that isn't JSON arrives after the year, the good copy comes back, marked stale with the
  reason;
- a saved copy hpr would refuse (a −32,768 m height) is an error offline, naming the height, and
  online is fetched again and replaced;
- a place turned into radians and back finds its saved answer offline.

The code's own tests check the address hpr builds, for one place and three, and that every
longitude in steps of 0.01°, and every latitude in steps of 0.005°, gives the same address after a
trip through radians. They check
that it refuses no places, 101 places, a latitude or longitude out of range or not a number (naming
the place), and a bad server address. They check that the reader takes negative heights and the
range's edges, and refuses each kind of broken answer above.

## What it leaves out

- **No command.** The command line has no elevation command yet, and `hpr sim` doesn't look a
  site up; a program calls the library and passes the height on.
- **No geoid model.** The height is above sea level, and turning it into a height above the
  ellipsoid needs the undulation `N`, which you give (above).
- **No accuracy check.** Nothing compares the model's heights with surveyed ones; the 4 m is the
  makers' statement.
- **A surface, not the ground.** Over a tree line or buildings the height can sit metres above the
  pad; for a pad cut out of forest, use a surveyed height.
- **One number per place.** The answer is the model's height there, at about 90 m spacing; a pad
  on a hill or beside a cliff can sit metres off it.
- **Your own terrain file** (a GeoTIFF) is planned for
  [M5.3c](decisions-and-roadmap.md#m5-3c).
