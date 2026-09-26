# ERA5 weather files

[ERA5](../glossary.md#era5) is a record of the weather over the whole Earth, every hour since
1940, from the European Centre for Medium-Range Weather Forecasts (ECMWF). It is a
[reanalysis](../glossary.md#reanalysis): a weather model run over the past and held to the
observations of the time. hpr reads ERA5's pressure-level files to fly a rocket in the weather of
a real day: the temperature, pressure and wind over the launch site, from the ground up to a few
kilometres or more.

**How far to trust it.** hpr reads these files the way RocketPy does, to 12 digits, on two real
launch days (below). What it does with the numbers differs from RocketPy in four places. Two are
measured here: the heights of the levels, and a launch between two of the file's hours. Two are
not measured yet: the pressure between levels, and the air above the file's top level. Seven
real flights have been flown in ERA5 weather and compared with their logs
([real flights](../accuracy.md#real-flights), milestone
[M2.3b](../decisions-and-roadmap.md#m2-3b)): their apogees miss by 6.04% on average. Humidity is
not read yet, so the air is taken as dry.

Code: `hpr_io::era5` and `hpr_io::netcdf`
([API reference](../api/hpr_io/era5/index.html)), written for the ERA5-weather milestone
([M2.3a](../decisions-and-roadmap.md#m2-3a)). The choices are in
[ADR-081: ERA5 weather read from netCDF classic](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-081-era5-weather-read-from-netcdf-classic-in-hpr-io-m23-split-a-to-c-2026-09-26).

## Getting a file

ERA5 comes from the Copernicus Climate Data Store. An account is free. The data's licence is
CC BY 4.0: you may use and share it if you credit it ("Contains modified Copernicus Climate Change
Service information"), and you accept it once on the dataset's page before the first download.

1. Open the dataset
   [*ERA5 hourly data on pressure levels from 1940 to present*](https://cds.climate.copernicus.eu/datasets/reanalysis-era5-pressure-levels)
   and its *Download* tab. For the product type choose **Reanalysis**.
2. Choose these variables: **Geopotential**, **Temperature**, **U-component of wind** and
   **V-component of wind**.
3. Choose the pressure levels that cover the flight. Air pressure halves about every 5.5 km, so
   1000 hPa down to 500 hPa covers the lowest 5.5 km. Include a level below the pad too (ERA5
   continues its levels beneath high ground), so the pad lies between two levels. Below the
   lowest level hpr continues the standard atmosphere and marks the air as extrapolated.
4. Choose the day and every hour around the launch, in UTC, not every third or sixth hour. hpr
   blends the two file times on either side of the launch however far apart they are, so a file
   with gaps blends weather hours apart.
5. Choose a small area around the site, at least a quarter of a degree beyond it on each side.
   ERA5's grid points are a quarter of a degree apart, and hpr needs the four around the site.
   hpr doesn't join a whole-Earth grid's last longitude to its first, so a site between them is
   refused: on a grid from 0° to 359.75° east, the last quarter degree west of 0°.
6. Choose the [NetCDF](../glossary.md#netcdf) format, then convert the file as below. hpr does not read GRIB, the other
   format offered (the weather services' own binary format).

## Converting a current file

Files from today's Data Store are netCDF-4. Inside, that is HDF5, a general container format
that hpr doesn't read. Three lines of Python, with the xarray and netCDF4 packages
(`pip install xarray netCDF4`), rewrite a file in the older netCDF classic format:

```python
import xarray
xarray.open_dataset("era5.nc").drop_vars(["number", "expver"], errors="ignore").to_netcdf(
    "era5-classic.nc", format="NETCDF3_64BIT")
```

The two dropped variables are labels the classic format can't hold: a 64-bit whole number and a
text value. Older files, like most of the ones RocketPy ships, are classic already. If hpr can't
read a file, its error says which kind the file is and gives this conversion.

This conversion is checked. The test takes NDRT 2020's launch day, downloaded from the Data Store
in 2021 as a classic file and in 2024 as netCDF-4 and converted as above. NDRT 2020 is the
University of Notre Dame's rocket for NASA's 2020 Student Launch. The two files agree at every
level to 0.23 m²/s² of [geopotential](../glossary.md#geopotential-height) (about 2 cm of height), 0.35 thousandths of a kelvin and
0.11 mm/s of wind. The older file stores its values as 16-bit whole numbers and the newer one
carries the rounding of ECMWF's own archive, so the gaps are consistent with each file's
rounding. The test pins these largest gaps.

## Reading it

The example program
[`era5_weather`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/era5_weather.rs)
reads a small cut of the file RocketPy ships for Bella Lui's flight. The Swiss student team EPFL
Rocket Team flew Bella Lui at Kaltbrunn on 22 February 2020, from a pad 407 m above sea level,
and the file covers the pad at 13:00 UTC. The program prints the levels, then the air at the pad
and 500 m above it next to the standard atmosphere. Last, it flies the rocket in both, without its
parachute. Run it from a copy of the repository with `cargo run --example era5_weather -p hpr`.
It prints:

<!-- quote: crates/hpr/examples/era5_weather.output.txt -->
```text
ERA5 over 47.213476° N, 9.003336° E at 2020-02-22 13:00 UTC

level (hPa)   height (m)   temperature (°C)   wind (m/s)   from (°)
       1000          240               14.6          1.3        212
        975          453               13.2          1.3        213
        950          670               12.1          1.1        213
        925          892               10.3          1.7        207
        900         1119                8.5          3.2        217
        875         1351                7.6          5.6        232
        850         1590                6.6          7.2        243
        825         1834                6.0          8.7        253
        800         2085                4.8         10.1        258
        775         2344                3.5         10.7        261

height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)
     0 ERA5                         980.4               13.5            1.1916
     0 standard                     965.3               12.4            1.1778
   500 ERA5                         923.4               10.2            1.1354
   500 standard                     908.9                9.1            1.1218

Bella Lui to apogee   apogee (m above the pad)   drift at apogee (m)
ERA5                                      553.4                  40.7
standard, calm                            555.2                   0.4

This design flies a stand-in motor; Bella Lui's real flight, on its own K828FJ, is
compared on the accuracy page.
```

The 1000 hPa level lies at 240 m, below the pad: ERA5 continues its levels beneath the ground,
so the pad sits between two of them. That day was warm for February, and the pressure was high:
the air at the pad was 1.2% denser than the standard's. The wind was light at the pad and grew to
10 m/s by 2 km.

**Reading your own file.** hpr reads ERA5 from Rust only for now; there is no command-line tool
yet. Copy the example and change three things: read your file from disk
(`let bytes = std::fs::read("my-file.nc")?;` then `NetCdf::parse(&bytes)`) instead of the bundled
one, and give your site's latitude, longitude and height and your launch time in UTC (the printed
heading has the date written in, so change it too). The copy still flies the example's rocket,
Bella Lui; [Your own rocket](../your-own-rocket.md) shows how to describe yours. The steps are the example's own:
`NetCdf::parse` on the file's bytes, `Era5Profile::read` with the site and time, then
`Era5Profile::sounding` gives the atmosphere and the wind for the flight's `Environment`.

## How hpr reads it

At each pressure level the file gives the [geopotential](../glossary.md#geopotential-height), the
temperature and the wind's east and north parts (`u` and `v`) at every grid point and hour.

- **At the site,** each value is interpolated between the four grid points around it, first along
  one side and then the other ("bilinear" interpolation, in degrees). This is how RocketPy 1.13
  does it, and hpr follows its formula (RocketPy is MIT-licensed).
- **At the launch time,** hpr weighs the two hours on either side by how close each is. At 13:12
  it takes 80% of 13:00 and 20% of 14:00; on the hour it takes that hour alone.
- **Heights.** Geopotential divided by `g₀ = 9.80665 m/s²` is geopotential height. hpr turns
  that into height above sea level with the World Meteorological Organization's formula for the
  site's latitude, as for any [sounding](../physics/atmosphere.md#sounding-and-forecast-profiles).
  ERA5 itself stores no height in metres; the next section compares the two ways to get one.
- **Between and above the levels,** the atmosphere is the
  [sounding profile](../physics/atmosphere.md#sounding-and-forecast-profiles):
  - Pressure falls with height as the weight of the air above requires ("hydrostatic").
    RocketPy draws a straight line in height between the levels' pressures instead.
  - Above the top level, the 1976 standard atmosphere continues from that level.
  - The wind's east and north parts are interpolated in height, as RocketPy does. With
    `WindInterpolation::SpeedDirection` hpr interpolates speed and direction instead.

## Against RocketPy

The tests read RocketPy's two ERA5 files for Bella Lui (47.2° N) and NDRT 2020 (41.8° N), plus
NDRT's day from today's Data Store. RocketPy 1.13 reads the same files.

| | hpr | RocketPy 1.13 | measured difference |
|---|---|---|---|
| Temperature, wind and geopotential at each level, on the hour | bilinear | bilinear | the same to 12 digits (5 readings, 14 or 37 levels each) |
| Height of a level | WMO's formula, with gravity at the site's latitude | ECMWF's formula, with `g₀` at every latitude | −0.0158% at Bella Lui, 47.21° N (−0.69 m at 4.4 km); +0.0343% at NDRT, 41.78° N (+1.45 m at 4.2 km) |
| Launch between two of the file's hours | both hours, weighted by time | the nearer hour | hpr's value is the weighted mean of RocketPy's two readings, to 12 digits |
| Pressure between levels | hydrostatic | straight line in height | not measured yet |
| Above the top level | the standard atmosphere, continued | the top level's values, held | not measured yet; Bella Lui's file stops at 4.4 km |

Both height readings are approximations, and neither is exact. Geopotential measures the work of
lifting air against gravity, so turning it into metres needs gravity on the way up. ECMWF's
knowledge base gives `h = R·Z/(R − Z)`, with `R` the Earth's radius and `Z` the geopotential
height, and says it neglects gravity's change across the Earth. RocketPy uses it. hpr uses the
World Meteorological Organization's formula, which takes gravity at the site's latitude: at sea
level it is 9.780 m/s² at the equator and 9.832 m/s² at the poles.

How far off each is depends on how ERA5's model builds the geopotential of its own ground, which
ECMWF doesn't say. Taking it as `g₀` times the ground's height, hpr's reading is off by a fixed
amount at every height and ECMWF's by an amount that grows with height above the ground. With
RocketPy's Earth radius in ECMWF's formula (the WGS 84 ellipsoid's distance from the Earth's
centre at the site), neither is always the smaller:

| model ground | hpr's error | ECMWF's error, at the ground | ECMWF's, 3 km above it |
|---|---|---|---|
| 407 m at 47.2° N | −0.04 m | +0.03 m | +0.49 m |
| 1400 m at 33° N (like Spaceport America's) | +1.88 m | +0.31 m | −3.07 m |

At the second site ECMWF's reading is the closer one up to 1.95 km above the ground, and hpr's
above that. hpr keeps WMO's formula because it is the one it uses for every sounding. The
derivation is in the `hpr_io::era5` module's documentation
([API reference](../api/hpr_io/era5/index.html)), and a test pins these numbers.

## The netCDF reader

netCDF is a file format for gridded data, widely used for weather and climate. hpr reads the two
classic kinds from Unidata's published specification: the original format and the 64-bit offset
format, whose files begin with the bytes `CDF` and then 1 or 2.

- **Numbers.** It reads every value exactly as stored, all six types, and the tests check each
  one against Unidata's own library on files that library wrote.
- **Packed values.** Many files store small whole numbers plus a scale and an offset. hpr unpacks
  them as the netCDF Users Guide says.
- **Missing values.** A value can mean "missing": the fill value, or a listed missing value. Where
  the Guide and the netCDF4 Python library disagree about which values are missing, hpr follows
  the Guide:

| stored value | the Guide, and hpr | netCDF4-python 1.7.4 |
|---|---|---|
| beyond the fill value, on the side away from zero (for example −32768 when the fill is −32767) | missing | a value |
| −127 in bytes with no fill value given | a value | missing |

The first row matters for ERA5. Many ERA5 files store each value as a 16-bit whole number with a
fill of −32767, and a few values sit at −32768. Two of RocketPy's other ERA5 files (both
netCDF-4) have them: 102 of 5,241,600 geopotential values and 4 temperatures in one, 298 of
5,184,000 geopotential values and 2 temperatures in the other. hpr reads those as missing, where netCDF4-python returns a
number, a temperature of 198.66 K for example, among neighbours near 301 K. Such values look
like damaged data, so reading them as missing is the safer choice. `Era5Profile::read` then
refuses the whole reading, naming the level, rather than print a wrong number. None of the files
the tests read has such values.

netCDF-4 files, and the rarer 64-bit data format (files beginning `CDF` and 5), are refused with
the conversion above.

## What it leaves out

- **Humidity.** hpr doesn't read the humidity yet, so it flies in dry air. At 20 °C and 50%
  relative humidity, dry air is about 0.4% denser than the real air.
- **Surface files.** ERA5's single-level files, with the 10 m wind and 2 m temperature, are not
  read.
- **GRIB and netCDF-4.** Convert them first, as above.
- **The longitude seam.** On a whole-Earth grid, a site between the grid's last longitude and its
  first is refused (on a 0° to 359.75° grid, the last quarter degree west of 0°). Download a
  regional area around the site instead.
- **Geoid.** ERA5's heights are above sea level. hpr has no geoid model, so a flight takes them as
  heights above the WGS 84 ellipsoid unless you give the site's geoid height, the height of sea
  level above the ellipsoid, which is up to about 100 m ([Geodesy](../physics/geodesy.md)).

## Sources

- **[U]** Unidata, *NetCDF File Format Specifications*, "The Classic Format" and "The 64-bit Offset
  Format", and the netCDF Users Guide's "Attribute Conventions", netCDF-C documentation, captured
  2026-09-26 and pinned as `unidata-netcdf-file-format` and `unidata-netcdf-attribute-conventions`
  in the [reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml).
- **[H]** H. Hersbach et al., "The ERA5 global reanalysis", *Quarterly Journal of the Royal
  Meteorological Society* 146 (2020), 1999–2049.
- **[ECMWF]** ECMWF Knowledge Base, "ERA5: compute pressure and geopotential on model levels,
  geopotential height and geometric height", captured 2026-09-26 and pinned as
  `ecmwf-era5-geometric-height`.
- **[WMO]** WMO-No. 8, *Guide to Instruments and Methods of Observation*, Vol. I (2023), eqs.
  12.15–12.16, `wmo-no8-vol1-2023`.
- **RocketPy** 1.13.0 (MIT): its reading of the same files is the reference in the tests.

## Tests that pin this

- `hpr_io::netcdf::tests::every_file_reads_as_the_unidata_library_reads_it`: every type, record
  variables, the padding cases and the packing conventions, on 12 files the Unidata library wrote
  (`validation/oracles/netcdf/write_cases.py`), with each difference from netCDF4-python listed.
- `hpr_io::netcdf::tests::each_break_of_the_grammar_is_refused_for_its_reason`,
  `variables_that_claim_more_bytes_than_the_file_holds_are_refused` and
  `a_slab_too_large_to_pad_is_refused`, with two fuzz tests: damaged or hostile files are refused,
  never a crash.
- `hpr_io::era5::tests::on_the_hour_it_reads_the_levels_rocketpy_reads`, and the height
  difference's cause, level by level (`the_two_height_readings_differ_as_the_guide_says`).
- `hpr_io::era5::tests::each_height_reading_errs_as_the_module_documentation_says`: the table of
  height errors above.
- `hpr_io::era5::tests::between_hours_it_weights_the_two_hours_in_time`.
- `hpr_io::era5::tests::the_current_data_store_file_converted_as_the_guide_says_reads_like_the_older_file`.
- The extracts and RocketPy's reading come from `validation/oracles/netcdf/era5.py`.
