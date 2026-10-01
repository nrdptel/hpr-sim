# A launch site's elevation

This page covers two ways to get a launch site's elevation, its altitude above sea level (the
field elevation a club lists). The first looks it up from [Open-Meteo](https://open-meteo.com), a
free weather and terrain service; the answer is saved, so the same lookup works later with no
network, on a field with no signal. The second reads it from an elevation file you have, a
GeoTIFF ([below](#from-an-elevation-file-of-your-own)). It is for anyone who needs a site's height
above sea level to start a flight at the right air pressure and density. There is no `hpr`
command for either yet: a Rust program calls the library.

**How far to trust it, online.** hpr gives back Open-Meteo's number unchanged, and the saved
copy gives it back offline. That is checked on two recorded answers, below. The number comes from a
terrain model whose cells are about 90 m across, and it is a *surface* height: over trees or
buildings it sits above the bare ground. The model's makers state its accuracy as better than 4 m
for 90% of points, averaged over the world outside Antarctica and Greenland; in about 1 area in 90
it is worse than 10 m. hpr hasn't measured it. In the standard atmosphere, 10 m of height error
changes the air's density by about 0.1%: the example below shows it 12.8% thinner over 1,400 m.
The recorded heights are whole metres; Open-Meteo doesn't document its rounding.

The tests replay two saved answers and never contact Open-Meteo. The live service was contacted
by hand, over an encrypted (HTTPS) connection, to record them. So a change in Open-Meteo's answers
would show only when a program runs, as a refused answer.

**How far to trust it, from a file.** At a given place, hpr reads the same stored value as GDAL,
the library most mapping programs read terrain with. That is checked at 2,800 places in seven
small files, in CI, and at 2,000 places in a whole tile from the US Geological Survey (USGS), on
machines that have downloaded it, not in CI ([how](#how-the-file-reader-is-checked)). Only files on a latitude and longitude grid are read. How
accurate the height itself is depends on who made the file; hpr gives back what is stored.

Code:

- **Online:** `hpr_net::elevation` ([API reference](api/hpr_net/elevation/index.html)), written for
  [M5.3b](decisions-and-roadmap.md#m5-3b), the second launch-site increment. It needs the `net`
  feature of the `hpr` crate. The choices are in
  [ADR-126: Open-Meteo's elevation through the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-126-m53b-open-meteos-elevation-through-the-cache-2026-09-30).
- **From a file:** `hpr_io::geotiff` ([API reference](api/hpr_io/geotiff/index.html)), which the
  `hpr` crate re-exports as `hpr::hpr_io::geotiff`, with no feature needed. It was written for
  [M5.3c2](decisions-and-roadmap.md#m5-3c2), with its choices in
  [ADR-128: a site's height from a user's GeoTIFF](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-128-m53c2-a-sites-height-from-a-users-geotiff-held-to-rasterios-reading-2026-09-30).

The weather over a site is on [Launch-day weather](weather.md), and the compass's offset from true
north on [The magnetic field and declination](physics/magnetic.md).

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

A request can name another server instead (`ElevationRequest::endpoint`), for a copy of
Open-Meteo you run yourself.

hpr writes each coordinate to 5 decimal places, about 1 m on the ground. So a place given to 8
decimals or fewer, which your program keeps in radians and turns back into degrees (that can
change its last digits), still finds its saved answer.

**A saved answer is found again only by the same request:** the same places, in the same order.
If you look up three club fields in one request at home, a later lookup of one of them alone is a
new request, and offline it fails. To use a site offline, look it up alone, or repeat the exact
list you used before.

### The terrain model

The heights come from the Copernicus DEM GLO-90, a *digital elevation model* (DEM): a grid of
heights over the whole Earth, made by the European Union's Copernicus programme from radar
satellites. Its points are 3 seconds of arc apart north to south, about 93 m. East to west they
are 93 m apart at the equator and 60 m at 50° of latitude; further north and south the spacing
widens in steps ([product handbook](https://dataspace.copernicus.eu/sites/default/files/media/files/2024-06/geo1988-copernicusdem-spe-002_producthandbook_i5.0.pdf),
issue 5.0, Table 3, page 15). It is a *digital surface model*: its heights include buildings and
vegetation ([the dataset's readme](https://copernicus-dem-30m.s3.amazonaws.com/readme.html)).

Its stated accuracy is under 4 m for 90% of points (handbook, Table 1, page 10). That is a mean
over the world outside Antarctica and Greenland, and the makers warn that it varies from place to
place: of the 16,363 tiles there, each about a degree across, 184 (1.1%) are worse than 10 m
(Table 12, page 31, which prints 0.9%, a share of all tiles, Antarctica and Greenland
included).

The ground doesn't move, so a saved answer stays fresh for a year. Open-Meteo's forecast for
Spaceport America ([Launch-day weather](weather.md)) gives the same ground height there, 1,400 m.

### What hpr refuses

hpr refuses an answer with another number of heights than places asked for, a height that is not
a number, or a height outside −1,000 m to 9,000 m. The lowest land, by the Dead Sea, lies a little
over 400 m below sea level, and the highest, Everest's summit, 8,849 m above it, so a height
outside is a broken answer, such as
the value −32,768 that some terrain files use for "no data". An answer hpr refuses is never saved,
as on [Online data and the cache](online-data.md#what-it-promises).

When Open-Meteo itself refuses a request (it answers with HTTP status 400 and a reason), the
HTTP transport reports a failed fetch naming the status, without Open-Meteo's reason. hpr checks
each place's latitude and longitude before asking, so that is rare.

## What the height means

The height is above mean sea level. The model's heights are measured from EGM2008, a worldwide
model of the *geoid*: the shape the sea's surface would take if it were still, extended under the
land ([product handbook](https://dataspace.copernicus.eu/sites/default/files/media/files/2024-06/geo1988-copernicusdem-spe-002_producthandbook_i5.0.pdf),
section 1.2.1, page 13). That is the
[height above sea level](glossary.md#height-above-sea-level-msl)
the atmosphere is looked up by ([Atmosphere: the height datum](physics/atmosphere.md#height-datum)).
The sea has no tiles in the model and reads 0 m, so 0 m can also mean "no data". The Dead Sea
reads −427 m, its surface when the radar satellites measured it, between December 2010 and
January 2015 (handbook, page 30); the lake has fallen since.

A flight's launch site takes its height above the [WGS 84](glossary.md#wgs-84) ellipsoid instead
(its [ellipsoidal height](glossary.md#ellipsoidal-height)). The two differ by the *geoid
undulation* `N`, the height of sea level above the ellipsoid there: between about −107 m and
+86 m around the world. hpr has no model of `N`; a geoid calculator gives it for a place, such as
[GeographicLib's GeoidEval](https://geographiclib.sourceforge.io/cgi-bin/GeoidEval), which gives
EGM2008's −23.85 m at Spaceport America (32.99° N, 106.97° W). Then:

- **If you know `N`,** place the site with `Geodetic::from_degrees(latitude, longitude, H + N)`,
  where `H` is the height from this page, and give the flight's environment `N` with
  `Environment::standard(site)?.with_geoid_undulation_m(N)`. At Spaceport America, `H` = 1,400 m
  and `N` = −23.85 m, so the site's ellipsoidal height is 1,376.15 m.
- **If you don't,** place the site at `H` and leave `N` at 0, as the example below does. The
  atmosphere still sees the right height above sea level, so the air, the drag and the flight are
  right. The site's place in space is off by `N`, which moves gravity by about 3×10⁻⁴ m/s² per
  100 m, 0.003% (normal gravity's [height term](physics/gravity.md#formulas)). A flight exported as
  KML keeps its heights above sea level, which stay right; the GeoJSON export's heights are above
  the ellipsoid, and are off by `N`.

## An example

[`crates/hpr/examples/site_elevation.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/site_elevation.rs)
looks up the three places above in one request. It makes these calls:

1. `Client::new(transport, Cache::new(folder), Mode::Online)` sets up the fetching. The
   *transport* is what fetches ([Online data and the cache](online-data.md)). A real program
   passes `hpr_net::Http::new()`, and keeps its saved answers in `Cache::platform_dir()`, the
   system's usual folder for them. This one never uses the network: a stand-in transport answers
   with the answer recorded for the tests.
2. `ElevationRequest::new(places)` says what to ask for, and
   `elevation::fetch(&client, &request, now)` fetches the heights and saves them; `now` is the
   time, in seconds since 1 January 1970. Each comes back
   with its place, as `height_msl_m`, the height above mean sea level.
3. The same `elevation::fetch` a day later, through a client in `Mode::Offline` whose transport
   has no network, answers from the saved copy.
4. `Ussa76::standard().air(height)` gives the
   [standard atmosphere](glossary.md#standard-atmosphere) (the 1976 US Standard Atmosphere) at
   each height, and `Environment::standard(site)` puts a flight's start on the first place,
   Spaceport America.

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
show it wherever the height is shown. The column `density / sea` is the air's density as a
fraction of sea level's. `Fetched` and `Cached` say where an answer came from
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
- a place turned into radians and back, which changes its last digits, finds its saved answer
  offline.

The code's own tests check the address hpr builds, for one place and three, and that it is the
same after a trip through radians for every longitude in steps of 0.01°, and for 720,000 values
given to 6 or 8 decimals, the 6-decimal ones all sitting on a rounding half step. They check that
it refuses no places, 101 places, a latitude or longitude out of range or not a number (naming the
place, counting from 0), and a bad server address. They check that the reader takes negative heights and the
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
- **Your own terrain file** is read by a separate reader, [below](#from-an-elevation-file-of-your-own).

## From an elevation file of your own

A [DEM](glossary.md#dem-digital-elevation-model) (digital elevation model) is a grid of ground
heights. Most are published as [GeoTIFF](glossary.md#geotiff) files: an image whose pixels are
heights, with tags saying where on Earth each pixel lies (the
[OGC GeoTIFF Standard 1.1](https://docs.ogc.org/is/19-008r4/19-008r4.html)). The United States
Geological Survey (USGS) publishes the United States this way, free and in the public domain,
through its [3D Elevation Program](https://www.usgs.gov/3d-elevation-program):

- Its 1-arc-second tiles (about 30 m) are what hpr is tested on. Its 1/3- and 1/9-arc-second
  tiles use the same latitude and longitude grid, but none has been tried.
- Its 1 m files are on a map projection (UTM) and are refused; GDAL's free `gdalwarp` tool
  converts them first, as the table below says.
- Copernicus and NASA publish the world as GeoTIFFs too; no file of theirs has been tried.

### Reading your own file

1. Bring the reader in: `use hpr::hpr_io::geotiff::ElevationRaster;`.
2. Read the whole file into memory: `let bytes = std::fs::read(path)?;`.
3. `ElevationRaster::parse(&bytes)` reads its tags. `info()` then holds what they say: the size,
   the corner and pixel size in degrees, the [CRS](glossary.md#crs-coordinate-reference-system),
   the sample type, the scale and offset, the unit and whether the file states it
   (`vertical_unit_stated`), the vertical datum's code (`vertical_crs_epsg`) and the nodata value.
4. `height_at(latitude_deg, longitude_deg)`, in degrees, gives
   `Result<Option<f64>, GeoTiffError>`: `Ok(Some(height))` in metres, `Ok(None)` where the file
   has no data, or an error from the table below.
5. For many places, `values_at(&[(lat, lon), …])` reads them in one pass. It gives a `Vec` with
   one `Result<Option<f64>, GeoTiffError>` per place, in order. Each is the raw stored value, not
   metres: `info().metres(value)` applies the scale, offset and unit.

The [API reference](api/hpr_io/geotiff/index.html) shows the same steps as a short program that CI
compiles and runs. The height is the value of the pixel the place falls in, as GDAL gives it.
There is no smoothing between pixels, so on a 1-arc-second grid it is the ground within about 20 m
of the site. A lookup decodes only the tile or strip of the file that holds the place, so it adds
about one tile's memory to the file's.

| error | from | means |
|---|---|---|
| `Outside` | a lookup | the place is off the file; the message names the file's edges |
| `Location` | a lookup | the latitude is past ±90° or a number is not finite |
| `Unsupported` | `parse` | the file uses something hpr doesn't read (the table below); the message names it and, where there is one, the GDAL command that converts it |
| `Missing`, `Malformed` | `parse`; rarely a lookup | the file lacks a tag a GeoTIFF needs, or holds one the standard doesn't allow |
| `TooLarge` | `parse`, `values` | a tile is larger than 256 MiB decoded; for `values`, which reads every pixel, the whole raster is past 2²⁸ pixels |
| `Tiff` | any | the image itself can't be decoded |

### What it reads

You rarely need this table: a USGS 1-arc-second tile reads as it comes. It is for checking an odd
file.

| what | read | refused |
|---|---|---|
| the grid | latitude and longitude in degrees from Greenwich, on a datum within a few metres of [WGS 84](glossary.md#wgs-84): WGS 84 itself, NAD83 and its updates, ETRS89, GDA94, GDA2020, NZGD2000, JGD2011, SIRGAS 2000 and CGCS2000 (the [EPSG codes](glossary.md#epsg-code) are `NEAR_WGS84` in the API) | a map projection such as UTM, or an older datum such as NAD27, tens to hundreds of metres from WGS 84: the error names the code, and `gdalwarp -t_srs EPSG:4326 in.tif out.tif` converts the file |
| the pixels | one band of 8-, 16- or 32-bit integers, or 32- or 64-bit floats; tiles or strips; uncompressed, LZW, Deflate or PackBits; either byte order; BigTIFF | several bands; zstd, JPEG and other compression (`gdal_translate -co COMPRESS=DEFLATE` rewrites the file); a separate mask of missing pixels |
| where the pixels lie | one tiepoint (a pixel tied to a longitude and latitude) with a pixel size, or a matrix without rotation; *pixel is area* or *pixel is point* (whether the tiepoint names a pixel's corner or its centre); longitudes 0° to 360° as well as −180° to 180° | a rotated grid; several tiepoints (ground control points); a south-up pixel size, which GDAL and the standard read differently |
| the heights | a scale and offset as GDAL applies them (from the file's pixel scale, or from GDAL's own metadata tag); metres, feet or US survey feet as the file states (in its keys or in GDAL's metadata tag), or as its [vertical datum](glossary.md#vertical-datum) implies; metres if it says nothing, with `vertical_unit_stated` then `false` | a vertical datum outside a short list hpr knows (`VERTICAL_CRS_UNITS` in the API: EGM2008, EGM96, NAVD88 and a few more), since its unit could be feet; a height scale in the file's pixel-scale tag when the file names a vertical datum in a way GDAL may not apply (hpr can't tell what GDAL would do); vertical keys GDAL ignores, unit and all, or reads by rules of its own (a private code, keys beside WGS 84 3D, datum 6030 beside WGS 84 with a model type, or keys with no model type and no unit); a scale, offset or unit in GDAL's metadata tag with a namespace, capitals in an attribute's name, a sample that isn't plain digits, a value that isn't plain text, a blank written as a character reference, or the `IMAGE_STRUCTURE` domain, forms GDAL reads with quirks of its own; a unit other than these, or two that disagree |
| no data | the file's nodata value, and NaN, read as `None` | |

**Which height it is.** The height is above the file's own vertical datum. The reader reports the
datum's EPSG code (`vertical_crs_epsg`) when the file names one, and doesn't change the height.
Check two things in `info()`, the first one first:

- **`vertical_unit_stated` is `false`:** the file states no unit, and hpr took metres. A file in
  feet read as metres gives heights 3.28 times too high.
- **`vertical_crs_epsg` is `None`:** the file names no datum. The USGS tile in the example below
  names none; the USGS's own description of its data says its heights are above NAVD88, the North
  American vertical datum. Check your publisher's description.

EGM2008 is a model of the [geoid](#what-the-height-means). NAVD88 was defined by levelling, and
the US National Geodetic Survey puts it about half a metre off the best geoid models, tilted by
about a metre from coast to coast ([NGS, new datums](https://geodesy.noaa.gov/datums/newdatums/index.shtml)): a metre or two at most. hpr takes a height above either as the `H` of
[What the height means](#what-the-height-means): the height above sea level the air is looked up
by. A metre's difference changes the air's density by about 0.01%. That section also shows how to
place a flight's site from `H` and the geoid undulation `N`.

### An example with a file

[`crates/hpr/examples/site_geotiff.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/site_geotiff.rs)
reads two small files cut from the USGS tile around Spaceport America, both with some pixels
blanked out as nodata: one in metres, one in whole US survey feet. It makes these calls:

1. `ElevationRaster::parse(bytes)` reads the tags; `info()` holds what they say.
2. `height_at(lat, lon)` gives the height at three places: the runway, a place in a blanked-out
   hole (`None`) and Albuquerque, off the file (an error naming the file's edges).
3. `value_at` gives the feet file's raw value, and `height_at` the same in metres.
4. `Ussa76::standard().air(height)` gives the standard atmosphere at the runway's height.

Run it from a copy of the repository with `cargo run --example site_geotiff -p hpr`. It prints:

<!-- quote: crates/hpr/examples/site_geotiff.output.txt -->
```text
70 by 50 pixels of 1.000" by 1.000", PixelIsArea, CRS EPSG:4269, values F32 in Metre (stated: false)
latitudes 32.98333 to 32.99722, longitudes -106.98500 to -106.96556

place                            lat (°)     lon (°)  height
Spaceport America, New Mexico    32.9900   -106.9700  1400.691 m
a hole in the data               32.9960   -106.9832  no data
Albuquerque, New Mexico          35.0844   -106.6504  refused: latitude 35.0844° and longitude -106.6504° are outside the raster, which spans latitudes 32.983333° to 32.997222° and longitudes -106.985000° to -106.965556°

in feet: 4595 UsSurveyFoot (vertical CRS EPSG:6360) = 1400.559 m, -0.132 m from the metres file
at 1400.7 m: 855.95 hPa, density 0.872 of sea level's
```

- `1.000"` is one arc-second, `PixelIsArea` says the tiepoint is a pixel's corner, and
  `EPSG:4269` is NAD83.
- `F32` is 32-bit floating point, and `Metre (stated: false)` says the file states no unit, so
  hpr took metres.
- In the feet file, `EPSG:6360` is NAVD88 in US survey feet, which the file states.

The runway reads 1,400.691 m. Open-Meteo's answer for the same place, [above](#an-example), is
1,400 m, from a different terrain model whose cells are 90 m across, above EGM2008. The feet file
stores 4,595 US survey feet, which is 1,400.559 m, 0.132 m from the metres file: rounding to whole feet moves
a height by up to 0.152 m.

### How the file reader is checked

The [oracle](glossary.md#oracle), the outside program hpr is held to, is GDAL 3.12.2, run through rasterio 1.5.2, its Python interface. A script,
[`validation/oracles/geotiff/dem.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/geotiff/dem.py),
cut 70 by 50 pixels around Spaceport America from the USGS's 1-arc-second tile `n33w107` and wrote
them in seven encodings:

| file | pixels | layout | georeferencing and heights |
|---|---|---|---|
| `usgs-f32-lzw-fp-tiles.tif` | 32-bit floats, LZW, floating-point predictor | 16-pixel tiles | NAD83, pixel is area, nodata holes |
| `usgs-i16-deflate-strips-be.tif` | 16-bit integers, Deflate, horizontal predictor | 7-row strips, big-endian | WGS 84 |
| `usgs-f64-raw-bigtiff-point.tif` | 64-bit floats, uncompressed | BigTIFF | WGS 84, pixel is point |
| `usgs-u16-packbits-ftus-nodata.tif` | 16-bit unsigned, PackBits | strips | NAD83 with NAVD88 in US survey feet, nodata holes |
| `usgs-u32-cm-scaled-egm2008.tif` | 32-bit unsigned, Deflate | strips | WGS 84 with EGM2008; centimetres above 1,000 m by the pixel scale |
| `usgs-u8-metadata-scaled.tif` | 8-bit unsigned, LZW | strips | WGS 84; quarter feet above 4,585 ft, scale, offset and unit all by GDAL's metadata |
| `usgs-i32-lzw-tiles-lon360.tif` | 32-bit integers, LZW, horizontal predictor | 32-pixel tiles | WGS 84, longitudes past 180° |

The script recorded what rasterio reads from each, and from the whole 3,612 by 3,612 tile:

- the corner, the pixel size, the CRS, the vertical datum and unit, the scale and offset, and the
  nodata value;
- the exactly rounded sum of every value, and of each value times its index (row × width +
  column), so a swapped pair of pixels would show;
- at 400 places in each small file and 2,000 in the tile (some off the edge), which pixel rasterio
  puts the place in, and its value.

The tests in
[`crates/hpr-io/tests/geotiff_rasterio.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-io/tests/geotiff_rasterio.rs)
hold hpr to all of it exactly:

| check | result |
|---|---|
| corner and pixel size | the same to the last bit, in all eight files |
| the two sums over every pixel | exact, 13 million pixels in the tile |
| places in the seven files | 2,800: the same pixel and value at all 2,368 on a file (9 of them nodata), and "outside" at the other 432 |
| places in the tile | 2,000: the same pixel and value at all 1,696 on it, and "outside" at the other 304 |
| unit | GDAL's wherever GDAL reports one, and `vertical_unit_stated` false wherever it reports none |
| heights | at every 97th place, hpr's height is GDAL's scale and offset applied to the value, in the file's unit |

CI checks the seven small files; the whole tile (45 MB, fetched by `cargo xtask refs fetch`) is
checked where it has been downloaded. The code's own tests build files to check each refusal in
the table above, the half-pixel move of pixel is point, the units, the scale and offset, and
nodata. Property tests check, on random strip and tile layouts, that every place reads the pixel it
is placed in, and that a file with bytes changed is read or refused without a crash.

The places are random, so none sits exactly on a pixel's edge. Rounding can put a place on either
side of an edge, in hpr and in GDAL, if it is closer to the edge than about 10⁻¹³ of a pixel's
width. A coordinate typed to a few decimals never comes that close.

### What the file reader leaves out

- **No map projections.** A UTM file, or one on an older datum, needs `gdalwarp` first.
- **No smoothing.** A place reads its pixel's value; on a slope the next pixel can be metres
  higher.
- **No datum changes.** A datum within a few metres of WGS 84 is taken as WGS 84, and the vertical
  datum is reported, not converted. Near the rupture of a large earthquake since a datum was
  fixed (Chile's in 2010 for SIRGAS 2000, for example) the ground has moved further, by several
  metres.
- **The file's accuracy is the file's.** The reader gives back what is stored; the USGS and other
  publishers state their own accuracy.
- **No command.** As for the online lookup, a program calls the library.
