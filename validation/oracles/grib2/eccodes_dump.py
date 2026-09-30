"""What ecCodes decodes from the recorded NOMADS cuts, for `hpr_io::grib2` to be checked against.

ecCodes (Apache-2.0, ECMWF) is run as an outside decoder; none of its code is ported. For each
recorded GRIB2 cut in `crates/hpr-net/tests/fixtures/replay/`, and the GFS cut as ecCodes repacks
it (`repack.py`, `nomads-gfs-complex.grib2`), this script writes, per message, the
keys that say what the field is (discipline, parameter category and number, the first fixed
surface's type, scale factor and scaled value, the reference date and time, the forecast time and
its unit), whether the winds are along the grid, and every value as ecCodes unpacks it; and, per
file, every grid point's latitude and longitude as ecCodes computes them. Floats are written by
Python's `repr`, which round-trips a double exactly.

The library is ecCodes 2.49.0, from the `eccodeslib` 2.49.0.30 wheel, driven by the `eccodes` 2.48.0
Python interface; both are pinned in `validation/oracles/pyproject.toml` and `uv.lock`, which
`cargo xtask refs fetch` installs into `refs/venv`. Each file's SHA-256 is written with its reading.
Run from the repository root:

    refs/venv/bin/python validation/oracles/grib2/eccodes_dump.py \
        > crates/hpr-net/tests/fixtures/nomads-eccodes.json
"""

import hashlib
import json
import sys

import eccodes

VERSION = "2.49.0"
FILES = ["replay/nomads-gfs.grib2", "replay/nomads-rap.grib2", "nomads-gfs-complex.grib2"]
DIR = "crates/hpr-net/tests/fixtures/"
KEYS = [
    "discipline",
    "parameterCategory",
    "parameterNumber",
    "typeOfFirstFixedSurface",
    "scaleFactorOfFirstFixedSurface",
    "scaledValueOfFirstFixedSurface",
    "dataDate",
    "dataTime",
    "forecastTime",
    "indicatorOfUnitOfTimeRange",
    "Ni",
    "Nj",
    "uvRelativeToGrid",
    "shortName",
]


def main():
    if eccodes.codes_get_api_version() != VERSION:
        sys.exit(f"ecCodes {VERSION} is pinned; this is {eccodes.codes_get_api_version()}")
    out = {"eccodes": VERSION, "files": {}}
    for name in FILES:
        messages = []
        points = None
        with open(DIR + name, "rb") as f:
            while (h := eccodes.codes_grib_new_from_file(f)) is not None:
                m = {
                    k: eccodes.codes_get(h, k, ktype=str if k == "shortName" else int)
                    for k in KEYS
                }
                m["values"] = [float(v) for v in eccodes.codes_get_values(h)]
                here = [
                    [float(a), float(b)]
                    for a, b in zip(
                        eccodes.codes_get_array(h, "latitudes"),
                        eccodes.codes_get_array(h, "longitudes"),
                    )
                ]
                if points is None:
                    points = here
                elif points != here:
                    sys.exit(f"{name}: the grid changes between messages")
                messages.append(m)
                eccodes.codes_release(h)
        with open(DIR + name, "rb") as f:
            sha256 = hashlib.sha256(f.read()).hexdigest()
        out["files"][name.removeprefix("replay/")] = {
            "sha256": sha256,
            "points": points,
            "messages": messages,
        }
    # One message per line, so a change shows as a line in a diff.
    w = sys.stdout.write
    w('{"eccodes": "%s", "files": {\n' % VERSION)
    for n, (name, f) in enumerate(out["files"].items()):
        w(' "%s": {\n  "sha256": "%s",\n' % (name, f["sha256"]))
        w('  "points": %s,\n  "messages": [\n' % json.dumps(f["points"]))
        w(",\n".join("   " + json.dumps(m) for m in f["messages"]))
        w("\n  ]\n }%s\n" % ("," if n + 1 < len(out["files"]) else ""))
    w("}}\n")


if __name__ == "__main__":
    main()
