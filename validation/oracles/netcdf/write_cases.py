"""netCDF classic files written by the Unidata netCDF library, and what that library reads back.

`hpr_io::netcdf` is checked against these: the files are written here by the netCDF C library
(through netCDF4-python, `netCDF4` in validation/oracles/pyproject.toml), never by hpr, and the
library's own reading of every value is recorded next to them. Each case is written twice, in the
classic format (`CDF\\x01`) and the 64-bit offset format (`CDF\\x02`). The cases cover every
type as a variable and as an attribute, scalars and several dimensions, record variables
interleaved with padding, the specification's two "Note on padding" cases (a lone record variable
of shorts or bytes, stored unpadded), an empty record dimension, and the attribute conventions for
packed and missing data. The values are invented for the tests, except that one packed variable
borrows the scale and offset of Bella Lui's ERA5 geopotential.

Run from the repository root:

    refs/venv/bin/python validation/oracles/netcdf/write_cases.py \
        > validation/fixtures/weather/netcdf-reads.json
"""

import hashlib
import json
import os
import sys

import netCDF4
import numpy as np

OUT = "validation/fixtures/weather/netcdf"
# The date of the committed run: change it when the fixtures are regenerated.
GENERATED = "2026-09-26"
COMMAND = (
    "refs/venv/bin/python validation/oracles/netcdf/write_cases.py "
    "> validation/fixtures/weather/netcdf-reads.json"
)
FORMATS = {"classic": "NETCDF3_CLASSIC", "offset64": "NETCDF3_64BIT_OFFSET"}
TYPES = {"i1": "byte", "S1": "char", "i2": "short", "i4": "int", "f4": "float", "f8": "double"}


def raw(var):
    """Every stored value of `var`, flattened, as JSON numbers (characters as byte values)."""
    var.set_auto_maskandscale(False)
    var.set_auto_chartostring(False)
    data = np.asarray(var[:])
    if data.dtype.kind == "S":
        return [int(b[0]) if len(b) else 0 for b in data.reshape(-1)]
    return [float(x) if data.dtype.kind == "f" else int(x) for x in data.reshape(-1)]


def unpacked(var):
    """Every value as the library unpacks and masks it: `None` where it is missing."""
    var.set_auto_maskandscale(True)
    data = np.ma.asarray(var[:]).reshape(-1)
    mask = np.ma.getmaskarray(data)
    return [None if m else float(x) for x, m in zip(data.data, mask)]


def attribute(value):
    if isinstance(value, str):
        return {"type": "char", "text": value}
    array = np.atleast_1d(np.asarray(value))
    kind = TYPES[array.dtype.str[1:]]
    values = [float(x) if array.dtype.kind == "f" else int(x) for x in array]
    return {"type": kind, "values": values}


def chars(strings, width):
    """`strings` as a character array `width` wide, padded with NUL bytes."""
    rows = [[bytes([b]) for b in s.encode().ljust(width, b"\0")] for s in strings]
    return np.array(rows, dtype="S1")


def case_types(d):
    d.createDimension("x", 3)
    d.createDimension("y", 2)
    d.createDimension("strlen", 5)
    d.title = "every type"
    d.setncattr("a_byte", np.array([-128, 0, 127], dtype="i1"))
    d.setncattr("a_short", np.array([-32768, 32767], dtype="i2"))
    d.setncattr("an_int", np.int32(-2147483648))
    d.setncattr("a_float", np.array([0.1, -3.0e38], dtype="f4"))
    d.setncattr("a_double", np.float64(1.0 / 3.0))
    s = d.createVariable("s", "f8", ())
    s[...] = np.pi
    b = d.createVariable("b", "i1", ("x",))
    b[:] = np.array([-128, -1, 127], dtype="i1")
    b.note = "odd bytes pad"
    c = d.createVariable("c", "S1", ("y", "strlen"))
    c[:] = chars(["abcde", "xyz"], 5)
    h = d.createVariable("h", "i2", ("y", "x"))
    h[:] = np.array([[-32768, 0, 32767], [1, -2, 3]], dtype="i2")
    i = d.createVariable("i", "i4", ("x",))
    i[:] = np.array([-2147483648, 7, 2147483647], dtype="i4")
    f = d.createVariable("f", "f4", ("y", "x"))
    f[:] = np.array([[0.1, -1.5, 3.4e38], [1e-40, 2.0, -0.0]], dtype="f4")
    f.setncattr("scale_hint", np.float32(0.25))
    g = d.createVariable("d", "f8", ("x",))
    g[:] = np.array([0.1, -2.5e300, 5e-324])
    g.units = "m"


def case_records(d):
    d.createDimension("time", None)
    d.createDimension("x", 3)
    d.createDimension("strlen", 2)
    lon = d.createVariable("lon", "f4", ("x",))
    lon[:] = [8.0, 8.25, 8.5]
    rb = d.createVariable("rb", "i1", ("time",))
    rh = d.createVariable("rh", "i2", ("time", "x"))
    rd = d.createVariable("rd", "f8", ("time",))
    rc = d.createVariable("rc", "S1", ("time", "strlen"))
    for n in range(4):
        rb[n] = n - 2
        rh[n, :] = [100 * n + k for k in range(3)]
        rd[n] = 0.5 * n + 0.1
        rc[n, :] = chars([f"r{n}"], 2)[0]


def case_one_record_short(d):
    d.createDimension("t", None)
    d.createDimension("x", 3)
    level = d.createVariable("level", "i4", ("x",))
    level[:] = [1000, 975, 950]
    h = d.createVariable("h", "i2", ("t", "x"))
    for n in range(5):
        h[n, :] = [10 * n, -10 * n, n]


def case_one_record_byte(d):
    d.createDimension("t", None)
    b = d.createVariable("b", "i1", ("t",))
    b[:] = np.array([1, -2, 3, -4, 5, -6, 7], dtype="i1")


def case_no_records(d):
    d.createDimension("t", None)
    d.createDimension("x", 2)
    fixed = d.createVariable("fixed", "f8", ("x",))
    fixed[:] = [1.5, 2.5]
    d.createVariable("empty", "f4", ("t", "x"))


def case_packing(d):
    d.createDimension("n", 6)

    def var(name, kind, values, fill=None, **attributes):
        v = d.createVariable(name, kind, ("n",), fill_value=fill)
        v.set_auto_maskandscale(False)
        v[:] = np.array(values, dtype=kind)
        for key, value in attributes.items():
            v.setncattr(key, value)

    # ERA5's packing: shorts with a double scale and offset, an explicit fill and missing value.
    var("era5_like", "i2", [-32768, -32767, -32766, 0, 32767, -1], fill=np.int16(-32767),
        scale_factor=np.float64(0.623424994373064), add_offset=np.float64(22680.694146877813),
        missing_value=np.int16(-32767))
    # A second missing value, as a vector.
    var("missing_vector", "i2", [1, 2, 3, 4, 5, 6], missing_value=np.array([2, 5], dtype="i2"))
    # No fill attribute: the short's default fill (-32767) is missing, and bounds the range.
    var("default_fill", "i2", [-32768, -32767, -32766, 0, 1, 2])
    # A valid range in the packed domain.
    var("valid_range", "i2", [-101, -100, 0, 100, 101, 7],
        valid_range=np.array([-100, 100], dtype="i2"), scale_factor=np.float64(0.5))
    # A lone valid minimum and a lone valid maximum.
    var("valid_min", "f4", [-2.0, -1.0, 0.0, 1.0, 2.0, 1e30], valid_min=np.float32(-1.0))
    var("valid_max", "i4", [5, 6, 7, 8, 9, 10], valid_max=np.int32(8))
    # Bytes with no fill attribute: every value is valid, the default fill (-127) too.
    var("byte_no_fill", "i1", [-128, -127, -1, 0, 1, 127])
    # Bytes with an explicit fill.
    var("byte_fill", "i1", [-128, -127, -1, 0, 1, 127], fill=np.int8(-1))
    # A positive explicit fill: values at or above it are missing.
    var("positive_fill", "i4", [97, 98, 99, 100, -5, 0], fill=np.int32(99))
    # Doubles with no fill attribute: the default fill is missing, and so is anything above it.
    var("double_default", "f8", [1.0, 9.969209968386869e36, 1e37, -1e37, 0.0, 2.0])
    # Float scale and offset attributes on bytes.
    var("float_packed", "i1", [-128, 0, 127, 10, 20, 30],
        scale_factor=np.float32(0.1), add_offset=np.float32(5.0))


CASES = {
    "types": case_types,
    "records": case_records,
    "one-record-short": case_one_record_short,
    "one-record-byte": case_one_record_byte,
    "no-records": case_no_records,
    "packing": case_packing,
}


def dump(path):
    d = netCDF4.Dataset(path)
    out = {
        "file": os.path.basename(path),
        "format": d.data_model,
        "dimensions": [
            {"name": n, "length": len(dim), "is_record": dim.isunlimited()}
            for n, dim in d.dimensions.items()
        ],
        "attributes": {k: attribute(d.getncattr(k)) for k in d.ncattrs()},
        "variables": [],
    }
    for name, var in d.variables.items():
        entry = {
            "name": name,
            "type": TYPES[var.dtype.str[1:]],
            "dimensions": list(var.dimensions),
            "shape": list(var.shape),
            "attributes": {k: attribute(var.getncattr(k)) for k in var.ncattrs()},
            "raw": raw(var),
        }
        if entry["type"] != "char":
            entry["unpacked"] = unpacked(var)
        out["variables"].append(entry)
    d.close()
    return out


def main():
    os.makedirs(OUT, exist_ok=True)
    files = []
    for case, build in CASES.items():
        for suffix, fmt in FORMATS.items():
            path = os.path.join(OUT, f"{case}-{suffix}.nc")
            if os.path.exists(path):
                os.remove(path)
            d = netCDF4.Dataset(path, "w", format=fmt)
            build(d)
            d.close()
            files.append(dump(path))
    with open(__file__, "rb") as f:
        script = hashlib.sha256(f.read()).hexdigest()
    json.dump(
        {
            "source": "written and read back by the Unidata netCDF C library "
            f"{netCDF4.__netcdf4libversion__} through netCDF4-python {netCDF4.__version__} "
            "(validation/oracles/netcdf/write_cases.py)",
            "generator": "validation/oracles/netcdf/write_cases.py",
            "tool": f"netCDF4-python {netCDF4.__version__}, netCDF-C {netCDF4.__netcdf4libversion__}",
            "generated": GENERATED,
            "command": COMMAND,
            "inputs_sha256": {"script": script},
            "files": files,
        },
        sys.stdout,
        indent=1,
    )
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
