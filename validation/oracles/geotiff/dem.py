"""GeoTIFF elevation files and rasterio's reading of them, the outside reading `hpr_io::geotiff`
is checked against (M5.3c2).

rasterio 1.5.2 (BSD-3-Clause) is run as the outside reader, with the GDAL 3.12 its wheel bundles
(MIT); none of its code is ported. It is pinned in `validation/oracles/pyproject.toml` and lives in
`refs/venv`. The source file is a public-domain USGS 3D Elevation Program tile, one arc-second,
32-33 N, 107-106 W (Spaceport America sits in it), pinned in `validation/refs.lock.toml` as
`usgs-3dep-1-n33w107`.

`cut` writes small fixtures cut from that tile to `crates/hpr-io/tests/fixtures/geotiff/`, each in
a different encoding a DEM can arrive in (sample type, codec, predictor, tiles or strips, byte
order, classic or BigTIFF, pixel-is-area or pixel-is-point, compound vertical units, a scale and
offset in the pixel scale or in GDAL's metadata, a unit in GDAL's metadata, nodata, longitudes
past 180), plus a projected one and a Zstandard-compressed one the reader must refuse. `read`
writes rasterio's
reading of every fixture, and of the whole tile, to `rasterio.json` beside them:

- the raster's size, its affine transform, its area-or-point flag, its nodata value, its CRS
  (the horizontal EPSG code, and the vertical CRS's code and unit's length in metres where it has
  one), and the band's scale, offset and unit as GDAL sets them;
- the correctly rounded sums (Python's `math.fsum`) of its values and of each value times its
  pixel's row-major index plus one, over the pixels that are not nodata, and the nodata count;
- at seeded random points (some outside the raster) the pixel rasterio's `index` puts them in
  and that pixel's value, `null` for nodata and `"outside"` off the raster. Each in-raster value
  is checked against rasterio's own `sample` before it is written.

Floats are written by Python's `repr`, which round-trips a double exactly. Run from the repository
root (the whole tile is 44.7 MB, fetched by `cargo xtask refs fetch`):

    refs/venv/bin/python validation/oracles/geotiff/dem.py cut
    refs/venv/bin/python validation/oracles/geotiff/dem.py read
"""

import hashlib
import json
import math
import random
import re
import sys
from pathlib import Path

import numpy as np
import rasterio
from rasterio.crs import CRS
from rasterio.transform import Affine
from rasterio.windows import Window

SOURCE = Path("refs/dem/USGS_1_n33w107.tif")
OUT = Path("crates/hpr-io/tests/fixtures/geotiff")
# The cut: 70 columns by 50 rows around Spaceport America's runway (32.99 N, 106.97 W). Neither is
# a multiple of the tile sizes below, so the last tiles and strips are partly padding.
WINDOW = Window(60, 16, 70, 50)
# Blocks of nodata punched into the fixtures that have a nodata value: (row, col, rows, cols).
HOLES = [(3, 5, 4, 3), (40, 66, 10, 4)]
POINTS_PER_FIXTURE = 400
POINTS_WHOLE_TILE = 2000
# Spaceport America's runway, the point every reading includes (lon, lat).
SITE = (-106.9750, 32.9903)


def holes(a, nodata):
    a = a.copy()
    for r, c, nr, nc in HOLES:
        a[r : r + nr, c : c + nc] = nodata
    return a


def fixtures(a):
    """Each fixture: file name, values, rasterio profile additions, CRS, area-or-point."""
    feet = np.round(a.astype(np.float64) * 3937.0 / 1200.0)
    return [
        # USGS's own encoding, on smaller tiles: float32, LZW, the floating-point predictor.
        (
            "usgs-f32-lzw-fp-tiles.tif",
            holes(a, -999999.0).astype("float32"),
            dict(dtype="float32", nodata=-999999.0, compress="lzw", predictor=3,
                 tiled=True, blockxsize=16, blockysize=16),
            CRS.from_epsg(4269),
            "Area",
        ),
        # Whole metres as int16, Deflate with the horizontal predictor, 7-row strips, big-endian.
        (
            "usgs-i16-deflate-strips-be.tif",
            np.round(a).astype("int16"),
            dict(dtype="int16", compress="deflate", predictor=2, blockysize=7,
                 ENDIANNESS="BIG"),
            CRS.from_epsg(4326),
            "Area",
        ),
        # float64, uncompressed, BigTIFF, pixel-is-point.
        (
            "usgs-f64-raw-bigtiff-point.tif",
            a.astype("float64") + 0.125,
            dict(dtype="float64", BIGTIFF="YES"),
            CRS.from_epsg(4326),
            "Point",
        ),
        # NAVD88 heights in US survey feet as uint16, PackBits, nodata 0.
        (
            "usgs-u16-packbits-ftus-nodata.tif",
            holes(feet, 0).astype("uint16"),
            dict(dtype="uint16", nodata=0, compress="packbits"),
            CRS.from_user_input("EPSG:4269+6360"),
            "Area",
        ),
        # uint32 centimetres above 1,000 m, Deflate, a scale and offset GDAL writes into the
        # pixel scale's S_z and the tiepoint's height, as it does for a file with a vertical CRS
        # (here EGM2008).
        (
            "usgs-u32-cm-scaled-egm2008.tif",
            np.round((a.astype(np.float64) - 1000.0) * 100.0).astype("uint32"),
            dict(dtype="uint32", compress="deflate", scales=(0.01,), offsets=(1000.0,)),
            CRS.from_user_input("EPSG:4326+3855"),
            "Area",
        ),
        # uint8 quarter feet above 4,585 ft, with no vertical CRS: GDAL writes the scale, the
        # offset and the unit into its GDAL_METADATA tag.
        (
            "usgs-u8-metadata-scaled.tif",
            np.round((a.astype(np.float64) / 0.3048 - 4585.0) * 4.0).astype("uint8"),
            dict(dtype="uint8", compress="lzw", scales=(0.25,), offsets=(4585.0,),
                 units=("ft",)),
            CRS.from_epsg(4326),
            "Area",
        ),
        # int32 decimetres, LZW with the horizontal predictor, 32-pixel tiles; longitudes 0-360.
        (
            "usgs-i32-lzw-tiles-lon360.tif",
            np.round(a.astype(np.float64) * 10.0).astype("int32"),
            dict(dtype="int32", compress="lzw", predictor=2, tiled=True, blockxsize=32,
                 blockysize=32, lon360=True),
            CRS.from_epsg(4326),
            "Area",
        ),
    ]


def cut():
    with rasterio.open(SOURCE) as src:
        a = src.read(1, window=WINDOW)
        transform = src.window_transform(WINDOW)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, values, extra, crs, area_or_point in fixtures(a):
        extra = dict(extra)
        scales = extra.pop("scales", None)
        offsets = extra.pop("offsets", None)
        units = extra.pop("units", None)
        t = transform
        if extra.pop("lon360", False):
            t = Affine(t.a, t.b, t.c + 360.0, t.d, t.e, t.f)
        profile = dict(driver="GTiff", width=values.shape[1], height=values.shape[0], count=1,
                       crs=crs, transform=t, **extra)
        with rasterio.open(OUT / name, "w", **profile) as dst:
            if area_or_point == "Point":
                dst.update_tags(AREA_OR_POINT="Point")
            if scales is not None:
                dst.scales = scales
                dst.offsets = offsets
            if units is not None:
                dst.units = units
            dst.write(values, 1)
    # A projected file (UTM zone 13 N), which the reader refuses: 8 by 8 pixels of 30 m.
    t = Affine(30.0, 0.0, 309000.0, 0.0, -30.0, 3652000.0)
    with rasterio.open(OUT / "utm13n-refused.tif", "w", driver="GTiff", width=8, height=8,
                       count=1, dtype="int16", crs=CRS.from_epsg(32613), transform=t) as dst:
        dst.write(np.arange(64, dtype="int16").reshape(8, 8), 1)
    # A Zstandard-compressed file, a codec the reader leaves out: 4 by 4 invented values.
    t = Affine(0.5, 0.0, 10.0, 0.0, -0.25, 20.0)
    with rasterio.open(OUT / "zstd-refused.tif", "w", driver="GTiff", width=4, height=4,
                       count=1, dtype="int16", crs=CRS.from_epsg(4326), transform=t,
                       compress="zstd") as dst:
        dst.write(np.arange(16, dtype="int16").reshape(4, 4), 1)


def vertical_unit_m(crs):
    """The vertical unit's length in metres, from the WKT's VERT_CS, or None without one."""
    wkt = crs.to_wkt()
    m = re.search(r'VERT_CS\[.*?UNIT\["[^"]*",([0-9.eE+-]+)', wkt)
    return float(m.group(1)) if m else None


def vertical_epsg(crs):
    """The EPSG code of the VERT_CS node: its last direct AUTHORITY."""
    wkt = crs.to_wkt()
    m = re.search(r"VERT_CS\[", wkt)
    if not m:
        return None
    return own_authority(wkt, m.end() - 1)


def own_authority(wkt, start):
    """The EPSG code of the WKT node opening at `start`: its last AUTHORITY at depth 1."""
    depth = 0
    for i in range(start, len(wkt)):
        depth += {"[": 1, "]": -1}.get(wkt[i], 0)
        if depth == 0:
            node = wkt[start : i + 1]
            break
    own = None
    depth = 0
    for i, ch in enumerate(node):
        depth += {"[": 1, "]": -1}.get(ch, 0)
        if depth == 1 and node.startswith("AUTHORITY[", i + 1):
            own = re.match(r'AUTHORITY\["EPSG","(\d+)"\]', node[i + 1 :])
    return int(own.group(1)) if own else None


def horizontal_epsg(crs):
    """The EPSG code of the horizontal CRS: the last AUTHORITY directly inside its GEOGCS or
    PROJCS node (the ones before it name its datum, spheroid and units)."""
    wkt = crs.to_wkt()
    m = re.search(r"(?:PROJCS|GEOGCS)\[", wkt)
    if not m:
        return None
    depth, start = 0, m.end() - 1
    for i in range(start, len(wkt)):
        depth += {"[": 1, "]": -1}.get(wkt[i], 0)
        if depth == 0:
            node = wkt[start : i + 1]
            break
    # The node's own AUTHORITY is its last child: depth 1 inside the node.
    own = None
    depth = 0
    for i, ch in enumerate(node):
        depth += {"[": 1, "]": -1}.get(ch, 0)
        if depth == 1 and node.startswith("AUTHORITY[", i + 1):
            own = re.match(r'AUTHORITY\["EPSG","(\d+)"\]', node[i + 1 :])
    return int(own.group(1)) if own else None


def reading(path, n_points, seed):
    with rasterio.open(path) as ds:
        arr = ds.read(1)
        t = ds.transform
        nodata = ds.nodata
        valid = np.ones(arr.shape, dtype=bool)
        if nodata is not None:
            valid &= arr != np.array(nodata).astype(arr.dtype)
        if arr.dtype.kind == "f":
            valid &= ~np.isnan(arr)
        flat = arr.astype(np.float64).ravel()
        vflat = valid.ravel()
        idx = np.arange(1, flat.size + 1, dtype=np.float64)
        total = math.fsum(flat[vflat].tolist())
        weighted = math.fsum((flat[vflat] * idx[vflat]).tolist())
        rng = random.Random(seed)
        w, h = ds.width, ds.height
        x0, y0 = t.c, t.f
        x1, y1 = t.c + t.a * w, t.f + t.e * h
        pad_x, pad_y = 0.04 * (x1 - x0), 0.04 * (y0 - y1)
        pts = [SITE if x0 <= 0 else (SITE[0] + 360.0, SITE[1])]
        for _ in range(n_points - 1):
            pts.append((rng.uniform(x0 - pad_x, x1 + pad_x), rng.uniform(y1 - pad_y, y0 + pad_y)))
        out = []
        for lon, lat in pts:
            row, col = ds.index(lon, lat)
            if 0 <= row < h and 0 <= col < w:
                if valid[row, col]:
                    value = float(arr[row, col])
                    (sampled,) = next(ds.sample([(lon, lat)]))
                    assert float(sampled) == value, (path, lon, lat, sampled, value)
                else:
                    value = None
            else:
                value = "outside"
            out.append([lon, lat, int(row), int(col), value])
        return {
            "file": path.name,
            "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "width": w,
            "height": h,
            "transform": [t.a, t.b, t.c, t.d, t.e, t.f],
            "area_or_point": ds.tags().get("AREA_OR_POINT"),
            "dtype": str(arr.dtype),
            "nodata": nodata,
            "horizontal_epsg": horizontal_epsg(ds.crs),
            "vertical_unit_m": vertical_unit_m(ds.crs),
            "vertical_epsg": vertical_epsg(ds.crs),
            "scale": ds.scales[0],
            "offset": ds.offsets[0],
            "units": ds.units[0] or None,
            "sum": total,
            "weighted_sum": weighted,
            "nodata_count": int((~valid).sum()),
            "points": out,
        }


def read():
    a = arr_fixture_names()
    readings = [reading(OUT / name, POINTS_PER_FIXTURE, i + 1) for i, name in enumerate(a)]
    readings.append(reading(SOURCE, POINTS_WHOLE_TILE, 0))
    readings[-1]["file"] = "refs/dem/" + SOURCE.name
    # One point per line: the file stays readable and its diffs small.
    blocks = []
    for r in readings:
        points = r.pop("points")
        head = json.dumps(r, indent=2)[:-2]
        rows = ",\n".join("    " + json.dumps(p) for p in points)
        blocks.append(f'{head},\n  "points": [\n{rows}\n  ]\n}}')
    text = (f'{{"rasterio": {json.dumps(rasterio.__version__)}, '
            f'"gdal": {json.dumps(rasterio.__gdal_version__)}, "readings": [\n'
            + ",\n".join(blocks) + "\n]}\n")
    json.loads(text)
    (OUT / "rasterio.json").write_text(text)


def arr_fixture_names():
    return sorted(p.name for p in OUT.glob("usgs-*.tif"))


if __name__ == "__main__":
    {"cut": cut, "read": read}[sys.argv[1]]()
