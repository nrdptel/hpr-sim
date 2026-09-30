"""A whole GFS file's values from `hpr_io::grib2` against ecCodes', and a few of its messages cut
out as the committed fixture's reading.

ecCodes (Apache-2.0, ECMWF) is run as an outside decoder; none of its code is ported. It is pinned
as in `eccodes_dump.py` (ecCodes 2.49.0 from the `eccodeslib` 2.49.0.30 wheel, in `refs/venv`).

`compare FILE` runs `cargo xtask grib2-values FILE` (a release build: its 6 GB of output is
quicker to write) and reads ecCodes' values for the same messages, one message at a time. It
prints, as JSON: the file's SHA-256, the messages by data representation and product template,
the values compared, the points without a value (which must be the same points), and the largest
relative difference between the two decoders' values. Run from the repository root:

    refs/venv/bin/python validation/oracles/grib2/whole_file.py compare \\
        refs/gfs/gfs.t00z.pgrb2.0p25.f018

`cut FILE` writes the messages named in `CUT` (by their place in the file, from 0), unchanged,
to `crates/hpr-io/tests/fixtures/gfs-messages.grib2`, and ecCodes' reading of them to
`gfs-messages-eccodes.json` beside it: per message, the keys that say what it is, its point and
missing counts, the correctly rounded sums (Python's `math.fsum`) of its values and of each value
times its point's index plus one, and every value at the points whose index is a multiple of
`STRIDE`. Floats are written by Python's `repr`, which round-trips a double exactly.

    refs/venv/bin/python validation/oracles/grib2/whole_file.py cut \\
        refs/gfs/gfs.t00z.pgrb2.0p25.f018
"""

import collections
import hashlib
import json
import math
import subprocess
import sys

import eccodes
import numpy as np

VERSION = "2.49.0"
MISSING = 9.87654321e300
OUT = "crates/hpr-io/tests/fixtures/"
STRIDE = 997
# The smallest message of each kind in GFS's 00 UTC run of 2026-09-30, hour 18 (`pgrb2.0p25`),
# that is not the same value everywhere, and a second template 4.8. `KINDS` names each.
CUT = [26, 204, 246, 278, 524, 604, 605, 730]
KINDS = {
    26: "5.3, 1-byte descriptors",
    204: "5.0, 0 bits",
    246: "5.3, 2-byte descriptors",
    278: "5.3, 3-byte descriptors",
    524: "5.3 with a bitmap",
    604: "5.3, 1-byte descriptors, template 4.8",
    605: "5.3, template 4.8, one value at all but 96 points",
    730: "5.3 with primary missing values",
}
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


def compare(path):
    pinned()
    hpr = subprocess.Popen(
        ["cargo", "run", "--quiet", "--release", "-p", "xtask", "--", "grib2-values", path],
        stdout=subprocess.PIPE,
    )
    kinds = collections.Counter()
    compared = missing = 0
    worst, worst_at = 0.0, None
    with open(path, "rb") as f:
        k = 0
        while (h := eccodes.codes_grib_new_from_file(f)) is not None:
            kinds[
                "5.%d, 4.%d"
                % (
                    eccodes.codes_get(h, "dataRepresentationTemplateNumber"),
                    eccodes.codes_get(h, "productDefinitionTemplateNumber"),
                )
            ] += 1
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
    json.dump(
        {
            "eccodes": VERSION,
            "file": path.rsplit("/", 1)[-1],
            "sha256": sha256(path),
            "messages": k,
            "templates": dict(sorted(kinds.items())),
            "values_compared": compared,
            "points_without_a_value": missing,
            "largest_relative_difference": worst,
            "at_message": worst_at,
        },
        sys.stdout,
        indent=1,
    )
    print()


def cut(path):
    pinned()
    raw, read = [], []
    with open(path, "rb") as f:
        k = 0
        while (h := eccodes.codes_grib_new_from_file(f)) is not None:
            if k in CUT:
                raw.append(eccodes.codes_get_message(h))
                m = {"message_in_file": k, "kind": KINDS[k]}
                m.update(
                    {
                        key: eccodes.codes_get(h, key, ktype=str if key == "shortName" else int)
                        for key in KEYS
                    }
                )
                v = values(h)
                present = ~np.isnan(v)
                index = np.arange(1, len(v) + 1, dtype=np.float64)
                m["points"] = len(v)
                m["missing"] = int((~present).sum())
                m["sum"] = math.fsum(v[present].tolist())
                m["index_weighted_sum"] = math.fsum((v[present] * index[present]).tolist())
                m["samples"] = [
                    None if math.isnan(x) else float(x) for x in v[::STRIDE].tolist()
                ]
                read.append(m)
            eccodes.codes_release(h)
            k += 1
    with open(OUT + "gfs-messages.grib2", "wb") as f:
        f.write(b"".join(raw))
    out = {
        "eccodes": VERSION,
        "from": path.rsplit("/", 1)[-1],
        "from_sha256": sha256(path),
        "sha256": sha256(OUT + "gfs-messages.grib2"),
        "stride": STRIDE,
    }
    # One message per line, so a change shows as a line in a diff.
    with open(OUT + "gfs-messages-eccodes.json", "w") as f:
        f.write(json.dumps(out)[:-1] + ', "messages": [\n')
        f.write(",\n".join(" " + json.dumps(m) for m in read))
        f.write("\n]}\n")


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] not in ("compare", "cut"):
        sys.exit(__doc__)
    {"compare": compare, "cut": cut}[sys.argv[1]](sys.argv[2])
