"""A whole GFS file's values from `hpr_io::grib2` against ecCodes', and a few of its messages cut
out as the committed fixture's reading.

ecCodes (Apache-2.0, ECMWF) is run as an outside decoder; none of its code is ported. It is pinned
as in `eccodes_dump.py` (ecCodes 2.49.0 from the `eccodeslib` 2.49.0.30 wheel, in `refs/venv`).

`compare FILE` runs `cargo xtask grib2-values FILE` (a release build: its 6 GB of output is
quicker to write) and reads ecCodes' values for the same messages, one message at a time. It
writes to `validation/oracles/grib2/gfs-whole-file.json`: the file's SHA-256; ecCodes' survey of
its messages (data representation and product templates, bytes per extra descriptor, order of
differencing, missing-value management, bitmaps); the values compared, the points without a
value (which must be the same points) and the largest relative difference between the two
decoders' values; and, since the file is the run the recorded NOMADS cut
(`crates/hpr-net/tests/fixtures/replay/nomads-gfs.grib2`) was taken from, the largest relative
difference between ecCodes' values in the two files at the cut's grid points. Run from the
repository root:

    refs/venv/bin/python validation/oracles/grib2/whole_file.py compare \\
        refs/gfs/gfs.t00z.pgrb2.0p25.f018

`cut FILE` writes the messages named in `KINDS` (by their place in the file, from 0), unchanged,
to `crates/hpr-io/tests/fixtures/gfs-messages.grib2`, and ecCodes' reading of them to
`gfs-messages-eccodes.json` beside it: per message, the keys that say what it is (and, for
template 4.8, its statistic, the interval's length and unit, and its end), its point and
missing counts, the correctly rounded sums (Python's `math.fsum`) of its values and of each value
times its point's index plus one, and every value at the points whose index is a multiple of
`STRIDE`. Floats are written by Python's `repr`, which round-trips a double exactly.

    refs/venv/bin/python validation/oracles/grib2/whole_file.py cut \\
        refs/gfs/gfs.t00z.pgrb2.0p25.f018

`cut-rap FILE` does the same for JPEG 2000 (template 5.40), which RAP's pressure-level files use
throughout, writing every message of FILE to `rap-jpeg2000.grib2` and its reading to
`rap-jpeg2000-eccodes.json`, with every 13th value (`RAP_STRIDE`: prime, so the samples cross
rows on both grids). FILE is
four messages cut by byte range (from each file's `.idx`) from NOMADS's RAP run of 00 UTC,
2026-09-30, hour 0: the 500 hPa temperature and the cloud base and top heights on grid 200
(`rap.t00z.awp200f00.grib2`, bytes 377355-379589, 885476-898443 and 898444-900958), and the 500
hPa temperature on grid 130 (`rap.t00z.awp130pgrbf00.grib2`, bytes 4742901-4773295). NOAA's
model output is in the public domain; NOMADS keeps a run for about two days.

    refs/venv/bin/python validation/oracles/grib2/whole_file.py cut-rap \\
        refs/scratch/jpeg/public-540.grib2
"""

import collections
import glob
import hashlib
import json
import math
import os
import subprocess
import sys

import eccodes
import numpy as np

VERSION = "2.49.0"
MISSING = 9.87654321e300
OUT = "crates/hpr-io/tests/fixtures/"
STRIDE = 997
# Messages of each kind in GFS's 00 UTC run of 2026-09-30, hour 18 (`pgrb2.0p25`): for 1-, 2- and
# 3-byte descriptors and template 4.8, the smallest over 2,000 bytes, for a message of some size;
# the smallest with a bitmap, with missing values in the data, and of template 4.8; and the one
# simple-packed field. `KINDS` names each.
KINDS = {
    126: "5.3, 1-byte descriptors",
    204: "5.0, 0 bits",
    233: "5.3, 2-byte descriptors",
    278: "5.3, 3-byte descriptors",
    524: "5.3 with a bitmap",
    604: "5.3, 1-byte descriptors, template 4.8",
    605: "5.3, template 4.8, the smallest",
    730: "5.3 with primary missing values",
}
# The four RAP messages in JPEG 2000 `cut-rap` reads: two without a bitmap, two with one.
RAP_KINDS = {
    0: "5.40, 6 bits, grid 200",
    1: "5.40, 15 bits, a bitmap",
    2: "5.40, 16 bits, a bitmap marking 434 of 10,152 points",
    3: "5.40, 9 bits, grid 130",
}
RAP_STRIDE = 13
STATISTICS = [
    "typeOfStatisticalProcessing",
    "indicatorOfUnitForTimeRange",
    "lengthOfTimeRange",
    "yearOfEndOfOverallTimeInterval",
    "monthOfEndOfOverallTimeInterval",
    "dayOfEndOfOverallTimeInterval",
    "hourOfEndOfOverallTimeInterval",
    "minuteOfEndOfOverallTimeInterval",
    "secondOfEndOfOverallTimeInterval",
]
CUT_FILE = "crates/hpr-net/tests/fixtures/replay/nomads-gfs.grib2"
REPORT = "validation/oracles/grib2/gfs-whole-file.json"
KEYS = [
    "discipline",
    "parameterCategory",
    "parameterNumber",
    "productDefinitionTemplateNumber",
    "dataRepresentationTemplateNumber",
    "typeOfFirstFixedSurface",
    "forecastTime",
    "Ni",
    "Nj",
    "shortName",
]


def pinned():
    if eccodes.codes_get_api_version() != VERSION:
        sys.exit(f"ecCodes {VERSION} is pinned; this is {eccodes.codes_get_api_version()}")


def values(h):
    """ecCodes' values, with NaN where there is none."""
    eccodes.codes_set(h, "missingValue", MISSING)
    v = np.asarray(eccodes.codes_get_values(h), dtype=np.float64)
    if np.any(v[v != MISSING] >= MISSING / 2):
        sys.exit("a value is near the missing-value marker")
    v[v == MISSING] = np.nan
    return v


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def between(path):
    """The largest relative difference between ecCodes' values in the recorded cut and in the
    whole file, at the cut's grid points, over the cut's fields (template 4.0)."""

    def fields(p, keep=None):
        out = {}
        with open(p, "rb") as f:
            while (h := eccodes.codes_grib_new_from_file(f)) is not None:
                k = (
                    eccodes.codes_get(h, "shortName"),
                    eccodes.codes_get(h, "typeOfLevel"),
                    eccodes.codes_get(h, "level"),
                )
                if eccodes.codes_get(h, "productDefinitionTemplateNumber") == 0 and (
                    keep is None or k in keep
                ):
                    out[k] = (
                        eccodes.codes_get_array(h, "latitudes"),
                        eccodes.codes_get_array(h, "longitudes"),
                        eccodes.codes_get_values(h),
                    )
                eccodes.codes_release(h)
        return out

    cut = fields(CUT_FILE)
    whole = fields(path, set(cut))
    worst, points = 0.0, 0
    for k, (lat, lon, v) in cut.items():
        wlat, wlon, w = whole[k]
        # GFS's 0.25° grid from 90° N, 0° E, north to south.
        idx = (np.rint((90 - lat) / 0.25) * 1440 + np.rint(lon / 0.25) % 1440).astype(int)
        if not (np.allclose(wlat[idx], lat) and np.allclose(wlon[idx] % 360, lon % 360)):
            sys.exit(f"{k}: the grid points don't line up")
        w = w[idx]
        scale = np.maximum(np.abs(v), np.abs(w))
        rel = np.where(scale > 0, np.abs(v - w) / np.where(scale > 0, scale, 1), 0.0)
        worst = max(worst, float(rel.max()))
        points += len(v)
    return {"fields": len(cut), "values": points, "largest_relative_difference": worst}


def compare(path):
    pinned()
    hpr = subprocess.Popen(
        ["cargo", "run", "--quiet", "--release", "-p", "xtask", "--", "grib2-values", path],
        stdout=subprocess.PIPE,
    )
    survey = collections.Counter()
    compared = missing = 0
    worst, worst_at = 0.0, None
    with open(path, "rb") as f:
        k = 0
        while (h := eccodes.codes_grib_new_from_file(f)) is not None:
            get = lambda key: eccodes.codes_get(h, key)  # noqa: E731
            packing = get("dataRepresentationTemplateNumber")
            survey["data representation 5.%d" % packing] += 1
            survey["product definition 4.%d" % get("productDefinitionTemplateNumber")] += 1
            if packing == 3:
                survey["order %d" % get("orderOfSpatialDifferencing")] += 1
                survey["%d-byte descriptors" % get("numberOfOctetsExtraDescriptors")] += 1
            if packing in (2, 3) and get("missingValueManagementUsed"):
                survey["missing values in the data"] += 1
            if get("bitMapIndicator") == 0:
                survey["a bitmap"] += 1
            theirs = values(h)
            eccodes.codes_release(h)
            n = int.from_bytes(hpr.stdout.read(8), "little")
            ours = np.frombuffer(hpr.stdout.read(8 * n), dtype="<f8")
            if n != len(theirs):
                sys.exit(f"message {k}: {n} points, ecCodes {len(theirs)}")
            gaps = np.isnan(theirs)
            if not np.array_equal(gaps, np.isnan(ours)):
                sys.exit(f"message {k}: the points without a value differ")
            a, b = ours[~gaps], theirs[~gaps]
            if not (np.all(np.isfinite(a)) and np.all(np.isfinite(b))):
                sys.exit(f"message {k}: a value is not finite")
            scale = np.maximum(np.abs(a), np.abs(b))
            diff = np.abs(a - b)
            rel = np.where(scale > 0, diff / np.where(scale > 0, scale, 1), 0.0)
            if len(rel) and rel.max() > worst:
                worst, worst_at = float(rel.max()), k
            compared += int((~gaps).sum())
            missing += int(gaps.sum())
            k += 1
    if hpr.stdout.read(1):
        sys.exit("hpr decoded more fields than ecCodes")
    if hpr.wait() != 0:
        sys.exit("cargo xtask grib2-values failed")
    # What decoded it: the SHA-256 of the decoder's sources, in name order.
    sources = sorted(glob.glob("crates/hpr-io/src/grib2/*.rs"))
    digest = hashlib.sha256(b"".join(open(p, "rb").read() for p in sources)).hexdigest()
    report = {
        "eccodes": VERSION,
        "decoder_sources": [p.rsplit("/", 1)[-1] for p in sources],
        "decoder_sha256": digest,
        "file": path.rsplit("/", 1)[-1],
        "bytes": os.path.getsize(path),
        "sha256": sha256(path),
        "messages": k,
        "survey": dict(sorted(survey.items())),
        "values_compared": compared,
        "points_without_a_value": missing,
        "largest_relative_difference": worst,
        "at_message": worst_at,
        "cut_against_file": between(path),
    }
    with open(REPORT, "w") as f:
        json.dump(report, f, indent=1)
        f.write("\n")


def cut(path, kinds=None, name="gfs-messages", stride=STRIDE):
    pinned()
    kinds = kinds or KINDS
    raw, read = [], []
    with open(path, "rb") as f:
        k = 0
        while (h := eccodes.codes_grib_new_from_file(f)) is not None:
            if k in kinds:
                raw.append(eccodes.codes_get_message(h))
                m = {"message_in_file": k, "kind": kinds[k]}
                m.update(
                    {
                        key: eccodes.codes_get(h, key, ktype=str if key == "shortName" else int)
                        for key in KEYS
                    }
                )
                if m["productDefinitionTemplateNumber"] == 8:
                    m["statistics"] = {key: eccodes.codes_get(h, key) for key in STATISTICS}
                v = values(h)
                present = ~np.isnan(v)
                index = np.arange(1, len(v) + 1, dtype=np.float64)
                m["points"] = len(v)
                m["missing"] = int((~present).sum())
                m["sum"] = math.fsum(v[present].tolist())
                m["index_weighted_sum"] = math.fsum((v[present] * index[present]).tolist())
                m["samples"] = [
                    None if math.isnan(x) else float(x) for x in v[::stride].tolist()
                ]
                read.append(m)
            eccodes.codes_release(h)
            k += 1
    if len(raw) != len(kinds):
        sys.exit(f"{len(raw)} of the {len(kinds)} messages asked for are in {path}")
    with open(OUT + name + ".grib2", "wb") as f:
        f.write(b"".join(raw))
    out = {
        "eccodes": VERSION,
        "from": path.rsplit("/", 1)[-1],
        "from_sha256": sha256(path),
        "sha256": sha256(OUT + name + ".grib2"),
        "stride": stride,
    }
    # One message per line, so a change shows as a line in a diff.
    with open(OUT + name + "-eccodes.json", "w") as f:
        f.write(json.dumps(out)[:-1] + ', "messages": [\n')
        f.write(",\n".join(" " + json.dumps(m) for m in read))
        f.write("\n]}\n")


def cut_rap(path):
    cut(path, RAP_KINDS, "rap-jpeg2000", RAP_STRIDE)


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in ("compare", "cut", "cut-rap"):
        sys.exit(__doc__)
    {"compare": compare, "cut": cut, "cut-rap": cut_rap}[sys.argv[1]](sys.argv[2])
