"""The recorded GFS cut repacked by ecCodes into complex packing, so the committed tests read
templates 5.2 and 5.3 through `hpr_net::nomads` and `hpr weather` (a whole GFS file is 550 MB).

ecCodes (Apache-2.0, ECMWF) is run as an outside encoder; none of its code is ported. It is pinned
as in `eccodes_dump.py`. Each message of `crates/hpr-net/tests/fixtures/replay/nomads-gfs.grib2`
is repacked in turn as template 5.2, 5.3 with first-order differences, and 5.3 with second-order
differences, in 24 bits at its decimal scale (so every value is kept to within half the step
it was packed with, which the script checks), and written to
`crates/hpr-net/tests/fixtures/nomads-gfs-complex.grib2`. `eccodes_dump.py` then reads it like
the recordings. Run from the repository root:

    refs/venv/bin/python validation/oracles/grib2/repack.py
"""

import sys

import eccodes

VERSION = "2.49.0"
SOURCE = "crates/hpr-net/tests/fixtures/replay/nomads-gfs.grib2"
OUT = "crates/hpr-net/tests/fixtures/nomads-gfs-complex.grib2"
PACKINGS = [
    ("grid_complex", None),
    ("grid_complex_spatial_differencing", 1),
    ("grid_complex_spatial_differencing", 2),
]


def main():
    if eccodes.codes_get_api_version() != VERSION:
        sys.exit(f"ecCodes {VERSION} is pinned; this is {eccodes.codes_get_api_version()}")
    out = []
    with open(SOURCE, "rb") as f:
        k = 0
        while (h := eccodes.codes_grib_new_from_file(f)) is not None:
            packing, order = PACKINGS[k % len(PACKINGS)]
            values = eccodes.codes_get_values(h)
            scale = eccodes.codes_get(h, "decimalScaleFactor")
            step = 2.0 ** eccodes.codes_get(h, "binaryScaleFactor") * 10.0**-scale
            eccodes.codes_set(h, "packingType", packing)
            # Changing the packing alone leaves 0 bits on a field this small, which flattens it;
            # 24 bits at the field's decimal scale keep every value to far less than its step.
            eccodes.codes_set(h, "bitsPerValue", 24)
            eccodes.codes_set(h, "decimalScaleFactor", scale)
            if order is not None:
                eccodes.codes_set(h, "orderOfSpatialDifferencing", order)
            eccodes.codes_set_values(h, values)
            repacked = eccodes.codes_get_values(h)
            if max(abs(a - b) for a, b in zip(values, repacked)) > step / 2:
                sys.exit(f"message {k}: repacking changed a value")
            template = eccodes.codes_get(h, "dataRepresentationTemplateNumber")
            if template != (2 if order is None else 3) or (
                order is not None and eccodes.codes_get(h, "orderOfSpatialDifferencing") != order
            ):
                sys.exit(f"message {k}: ecCodes wrote template 5.{template}")
            out.append(eccodes.codes_get_message(h))
            eccodes.codes_release(h)
            k += 1
    with open(OUT, "wb") as f:
        f.write(b"".join(out))


if __name__ == "__main__":
    main()
