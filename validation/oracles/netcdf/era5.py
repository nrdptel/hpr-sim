"""ERA5 extracts around two launch sites, and RocketPy's reading of the full files at each.

`hpr_io::era5` is checked against these. For each case this script:

1. cuts, from a weather file in RocketPy v1.13.0's `data/weather/` (`refs/rocketpy`), the grid
   points around the site (every time and level, the bracketing latitudes and longitudes and one
   more on each side) into a 64-bit offset netCDF file under
   `validation/fixtures/weather/era5/`. A file already in a classic format is copied value for
   value, packed as it was. The netCDF-4 file from the current Climate Data Store is written by
   the conversion the guide gives, `xarray.open_dataset(...).drop_vars(["number", "expver"],
   errors="ignore").to_netcdf(..., format="NETCDF3_64BIT")`, after selecting the same window;
2. reads the full source file with RocketPy 1.13.0's `Environment` (`type="Reanalysis"`,
   `dictionary="ECMWF"`) at the site, elevation and time of that flight's acceptance test, and
   records the levels RocketPy builds: pressure, geometric height, temperature and the wind's
   components. (The tests also pass `gravity=9.81`, left out here: the reanalysis reading divides
   by RocketPy's fixed standard gravity whatever the environment's gravity is.);
3. reads the extract the same way and checks it gives the same levels bit for bit, so the extract
   holds everything the reading uses.

The extracts contain modified Copernicus Climate Change Service information (2020): ERA5 hourly
data on pressure levels (Hersbach et al., 2023, doi:10.24381/cds.bd0915c6), as redistributed by
RocketPy.

Run from the repository root with the oracle environment:

    refs/venv/bin/python validation/oracles/netcdf/era5.py \
        > validation/fixtures/weather/era5-rocketpy.json
"""

import bisect
import datetime
import hashlib
import importlib.metadata
import json
import os
import sys
import warnings

import netCDF4
import numpy as np
import xarray
from rocketpy import Environment

WEATHER = "refs/rocketpy/data/weather"
# The date of the committed run: change it when the fixtures are regenerated.
GENERATED = "2026-09-26"
COMMAND = (
    "refs/venv/bin/python validation/oracles/netcdf/era5.py "
    "> validation/fixtures/weather/era5-rocketpy.json"
)
OUT = "validation/fixtures/weather/era5"
ROCKETPY = importlib.metadata.version("rocketpy")
ATTRIBUTION = (
    "Contains modified Copernicus Climate Change Service information 2020 (ERA5 hourly data on "
    "pressure levels), cut from RocketPy v1.13.0's data/weather/{source} by "
    "validation/oracles/netcdf/era5.py."
)

# Site, launch time (UTC) and file of each flight, as RocketPy's acceptance tests give them:
# tests/acceptance/test_bella_lui_rocket.py and test_ndrt_2020_rocket.py. The NDRT test flies both
# the older file and the current Climate Data Store's (`_new`).
CASES = [
    {
        "id": "bella-lui",
        "source": "bella_lui_weather_data_ERA5.nc",
        "latitude_deg": 47.213476,
        "longitude_deg": 9.003336,
        "elevation_m": 407,
        # The file holds 00, 06, 12, 13 and 18 h; the hours either side of a launch off the hour
        # check hpr's weighting in time.
        "times": [(2020, 2, 22, 13), (2020, 2, 22, 12), (2020, 2, 22, 18)],
    },
    {
        "id": "ndrt-2020",
        "source": "ndrt_2020_weather_data_ERA5.nc",
        "latitude_deg": 41.775447,
        "longitude_deg": -86.572467,
        "elevation_m": 206,
        "times": [(2020, 2, 23, 16)],
    },
    {
        "id": "ndrt-2020-cds",
        "source": "ndrt_2020_weather_data_ERA5_new.nc",
        "latitude_deg": 41.775447,
        "longitude_deg": -86.572467,
        "elevation_m": 206,
        "times": [(2020, 2, 23, 16)],
    },
]


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def window(values, x):
    """Index slice of the two grid values around `x` and one more on each side."""
    values = list(values)
    descending = values[0] > values[-1]
    keys = [-v for v in values] if descending else values
    i = bisect.bisect_left(keys, -x if descending else x)
    return slice(max(i - 2, 0), min(i + 2, len(values)))


def cut_classic(source, out, case):
    src = netCDF4.Dataset(source)
    src.set_auto_maskandscale(False)
    lat = window(src["latitude"][:], case["latitude_deg"])
    lon = window(src["longitude"][:], case["longitude_deg"])
    dst = netCDF4.Dataset(out, "w", format="NETCDF3_64BIT_OFFSET")
    dst.set_auto_maskandscale(False)
    for name, dim in src.dimensions.items():
        size = {"latitude": lat, "longitude": lon}.get(name)
        length = len(range(*size.indices(len(dim)))) if size else len(dim)
        dst.createDimension(name, None if dim.isunlimited() else length)
    for key in src.ncattrs():
        dst.setncattr(key, src.getncattr(key))
    dst.setncattr("comment", ATTRIBUTION.format(source=case["source"]))
    for name, var in src.variables.items():
        attributes = {k: var.getncattr(k) for k in var.ncattrs()}
        fill = attributes.pop("_FillValue", None)
        new = dst.createVariable(name, var.dtype, var.dimensions, fill_value=fill)
        new.set_auto_maskandscale(False)
        for key, value in attributes.items():
            new.setncattr(key, value)
        index = tuple({"latitude": lat, "longitude": lon}.get(d, slice(None)) for d in var.dimensions)
        new[...] = var[index]
    dst.close()
    src.close()


def cut_cds(source, out, case):
    ds = xarray.open_dataset(source)
    lat = window(ds["latitude"].values, case["latitude_deg"])
    lon = window(ds["longitude"].values, case["longitude_deg"])
    ds = ds.isel(latitude=lat, longitude=lon)
    ds.attrs["comment"] = ATTRIBUTION.format(source=case["source"])
    # The guide's conversion, word for word after the window.
    ds.drop_vars(["number", "expver"], errors="ignore").to_netcdf(out, format="NETCDF3_64BIT")


def rocketpy_levels(path, case, when):
    env = Environment(
        latitude=case["latitude_deg"],
        longitude=case["longitude_deg"],
        elevation=case["elevation_m"],
        date=when,
    )
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        env.set_atmospheric_model(
            type="Reanalysis", file=path, dictionary="ECMWF", pressure_conversion_factor="hPa"
        )
    heights = env.pressure.source[:, 0]
    for f in (env.temperature, env.wind_velocity_x, env.wind_velocity_y):
        assert np.array_equal(f.source[:, 0], heights)
    return {
        "earth_radius_m": float(env.earth_radius),
        "levels": [
            {
                "pressure_pa": float(p),
                "height_m": float(h),
                "temperature_k": float(t),
                "wind_east_m_s": float(u),
                "wind_north_m_s": float(v),
            }
            for h, p, t, u, v in zip(
                heights,
                env.pressure.source[:, 1],
                env.temperature.source[:, 1],
                env.wind_velocity_x.source[:, 1],
                env.wind_velocity_y.source[:, 1],
            )
        ],
    }


def main():
    os.makedirs(OUT, exist_ok=True)
    out = []
    for case in CASES:
        source = os.path.join(WEATHER, case["source"])
        extract = os.path.join(OUT, f"{case['id']}.nc")
        if os.path.exists(extract):
            os.remove(extract)
        with open(source, "rb") as f:
            classic = f.read(4) in (b"CDF\x01", b"CDF\x02")
        (cut_classic if classic else cut_cds)(source, extract, case)
        readings = []
        for when in case["times"]:
            full = rocketpy_levels(source, case, when)
            cut = rocketpy_levels(extract, case, when)
            assert full == cut, f"{case['id']} {when}: the extract reads differently"
            unix = datetime.datetime(*when, tzinfo=datetime.timezone.utc).timestamp()
            readings.append({"time": list(when), "unix_s": unix, "rocketpy": full})
        out.append(
            {
                "id": case["id"],
                "source": f"refs/rocketpy/data/weather/{case['source']}",
                "source_format": "netCDF classic" if classic else "netCDF-4 (HDF5)",
                "extract": f"validation/fixtures/weather/era5/{case['id']}.nc",
                "latitude_deg": case["latitude_deg"],
                "longitude_deg": case["longitude_deg"],
                "readings": readings,
            }
        )
    inputs = {"script": sha256(__file__)}
    for case in CASES:
        inputs[case["source"]] = sha256(os.path.join(WEATHER, case["source"]))
    json.dump(
        {
            "source": f"RocketPy {ROCKETPY} Environment(type='Reanalysis', dictionary='ECMWF') "
            "on the full files, which read the same on the extracts "
            "(validation/oracles/netcdf/era5.py)",
            "generator": "validation/oracles/netcdf/era5.py",
            "tool": f"RocketPy {ROCKETPY}, netCDF4-python {netCDF4.__version__}, "
            f"xarray {xarray.__version__}",
            "generated": GENERATED,
            "command": COMMAND,
            "inputs_sha256": inputs,
            "cases": out,
        },
        sys.stdout,
        indent=1,
    )
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
