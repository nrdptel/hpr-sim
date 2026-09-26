# ERA5 weather files

[ERA5](../glossary.md#era5) is a record of the weather over the whole Earth, every hour since
1940, from the European Centre for Medium-Range Weather Forecasts (ECMWF). It is a
[reanalysis](../glossary.md#reanalysis): a weather model run over the past and held to the
observations of the time. hpr reads ERA5's pressure-level files to fly a rocket in the weather of
a real day: the temperature, pressure and wind over the launch site, from the ground up to a few
kilometres or more.

**How far to trust it.** hpr reads these files the way RocketPy does, to 12 digits, on two real
launch days (below). What it does with the numbers differs from RocketPy in three places, each
measured here: heights, times between the hours, and the air above the file's top level. No
flight in ERA5 weather has been compared with a real flight yet; that is milestone
[M2.3b](../decisions-and-roadmap.md#m2-3b), RocketPy's logged flights. Humidity is not read yet,
so the air is taken as dry.

Code: `hpr_io::era5` and `hpr_io::netcdf`
([API reference](../api/hpr_io/era5/index.html)), written for the ERA5-weather milestone
([M2.3a](../decisions-and-roadmap.md#m2-3a)). The choices are in
[ADR-081: ERA5 weather read from netCDF classic](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-081-era5-weather-read-from-netcdf-classic-in-hpr-io-m23-split-a-to-c-2026-09-26).

## Getting a file

ERA5 comes from the Copernicus Climate Data Store. An account is free, and the licence lets you
use and share the data if you credit it ("Contains modified Copernicus Climate Change Service
information").

1. Open the dataset *ERA5 hourly data on pressure levels from 1940 to present*.
2. Choose these variables: **Geopotential**, **Temperature**, **U-component of wind** and
   **V-component of wind**.
3. Choose the pressure levels that cover the flight. Air pressure halves about every 5.5 km, so
   1000 hPa down to 500 hPa covers the lowest 5.5 km. Include a level below the pad too (ERA5
   continues its levels beneath high ground), so the pad lies between two levels. Below the
   lowest level hpr continues the standard atmosphere and marks the air as extrapolated.
4. Choose the day and the hours on each side of the launch, in UTC.
5. Choose a small area around the site, at least a quarter of a degree beyond it on each side.
   ERA5's grid points are a quarter of a degree apart, and hpr needs the four around the site.
6. Choose the NetCDF format, then convert the file as below. hpr does not read GRIB.

## Converting a current file

Files from today's Data Store are netCDF-4, which is HDF5 inside, and hpr doesn't read HDF5. One
line of Python, with the xarray package, rewrites a file in the older netCDF classic format:

```python
import xarray
xarray.open_dataset("era5.nc").drop_vars(["number", "expver"], errors="ignore").to_netcdf(
    "era5-classic.nc", format="NETCDF3_64BIT")
```

The two dropped variables are labels the classic format can't hold: a 64-bit whole number and a
text value. Older files, like most of the ones RocketPy ships, are classic already. If hpr can't
read a file, its error says which kind the file is and gives this line.

This conversion is checked. The test takes NDRT 2020's launch day, downloaded from the Data Store
in 2021 as a classic file and in 2024 as netCDF-4 and converted with the line above. The two agree
at every level to 0.35 thousandths of a kelvin and 0.11 mm/s of wind. Each file rounds its values
a little differently, and these gaps are that rounding.

## Reading it

The example program `era5_weather` (in `crates/hpr/examples/`) reads a small cut of the file
RocketPy ships for Bella Lui's flight. The Swiss student team EPFL Rocket Team flew Bella Lui at
Kaltbrunn on 22 February 2020, and the file covers the pad at 13:00 UTC. The program prints the
levels, then the air at the pad and 500 m above it next to the standard atmosphere. Last, it flies
the rocket in both, without its parachute:

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

Not yet validated against the real flight: that is milestone M2.3b.
```

That day was warm for February, and the pressure was high: the air at the pad was 1.2% denser
than the standard's. The wind was light at the pad and grew to 10 m/s by 2 km. A program reads a
file of its own with `NetCdf::parse` on the file's bytes, then `Era5Profile::read` with the site
and time, and flies with `Era5Profile::sounding` as its atmosphere and wind.

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
- **Between and above the levels,** the atmosphere is the
  [sounding profile](../physics/atmosphere.md#sounding-and-forecast-profiles):
  - Pressure falls with height as the weight of the air above requires.
  - Above the top level, the 1976 standard atmosphere continues from that level.
  - The wind's east and north parts are interpolated in height, as RocketPy does. With
    `WindInterpolation::SpeedDirection` hpr interpolates speed and direction instead.

## Against RocketPy

The tests read RocketPy's two ERA5 files for Bella Lui (47.2° N) and NDRT 2020 (41.8° N), plus
NDRT's day from today's Data Store. RocketPy 1.13 reads the same files.

| | hpr | RocketPy 1.13 | measured difference |
|---|---|---|---|
| Temperature, wind and geopotential at each level, on the hour | bilinear | bilinear | the same to 12 digits (5 readings, 14 or 37 levels each) |
| Height of a level | the site's own gravity, from its latitude | `g₀` at every latitude | −0.0158% at 47.2° N (−0.69 m at 4.4 km); +0.0343% at 41.8° N (+1.45 m at 4.2 km) |
| Launch between two of the file's hours | both hours, weighted by time | the nearer hour | hpr's value is the weighted mean of RocketPy's two readings, to 12 digits |
| Pressure between levels | hydrostatic | straight line in height | not measured yet |
| Above the top level | the standard atmosphere, continued | the top level's values, held | Bella Lui's file stops at 4.4 km |

The height difference is a gravity difference. Gravity at sea level is 9.780 m/s² at the equator
and 9.832 m/s² at the poles. hpr uses the value at the site, and RocketPy always uses 9.80665 m/s².

## The netCDF reader

netCDF is a file format for gridded data, widely used for weather and climate. hpr reads the two
classic kinds, `CDF\x01` and `CDF\x02`, from Unidata's published specification.

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

ERA5's files don't store such values. netCDF-4 files, and the rarer 64-bit data format (`CDF\x05`),
are refused with the conversion above.

## What it leaves out

- **Humidity.** hpr doesn't read the humidity yet, so it flies in dry air. At 20 °C and 50%
  relative humidity, dry air is about 0.4% denser than the real air.
- **Surface files.** ERA5's single-level files, with the 10 m wind and 2 m temperature, are not
  read.
- **GRIB and netCDF-4.** Convert them first, as above.
- **Geoid.** ERA5's heights are above sea level. hpr has no geoid model, so a flight takes them as
  heights above the WGS 84 ellipsoid unless you give the site's geoid height.

## Sources

- **[U]** Unidata, *NetCDF File Format Specifications*, "The Classic Format" and "The 64-bit Offset
  Format", and the netCDF Users Guide's "Attribute Conventions", netCDF-C documentation, captured
  2026-09-26 and pinned as `unidata-netcdf-file-format` and `unidata-netcdf-attribute-conventions`
  in the [reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml).
- **[H]** H. Hersbach et al., "The ERA5 global reanalysis", *Quarterly Journal of the Royal
  Meteorological Society* 146 (2020), 1999–2049.
- **[WMO]** WMO-No. 8, *Guide to Instruments and Methods of Observation*, Vol. I (2023), eqs.
  12.15–12.16, `wmo-no8-vol1-2023`.
- **RocketPy** 1.13.0 (MIT): its reading of the same files is the reference in the tests.

## Tests that pin this

- `hpr_io::netcdf::tests::every_file_reads_as_the_unidata_library_reads_it`: every type, record
  variables, the padding cases and the packing conventions, on 12 files the Unidata library wrote
  (`validation/oracles/netcdf/write_cases.py`), with each difference from netCDF4-python listed.
- `hpr_io::era5::tests::on_the_hour_it_reads_the_levels_rocketpy_reads`, and the height
  difference's cause, level by level.
- `hpr_io::era5::tests::between_hours_it_weights_the_two_hours_in_time`.
- `hpr_io::era5::tests::the_current_data_store_file_converted_as_the_guide_says_reads_like_the_older_file`.
- The extracts and RocketPy's reading come from `validation/oracles/netcdf/era5.py`.
