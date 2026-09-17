"""Build hpr-motor's bundled ThrustCurve.org catalog from a fixed list of public-domain curves.

The selection below came from `survey.py`: public-domain (license PD) curves of in-production
solid motors whose total impulse, NFPA 1125 burn time and average thrust are each within 1% of
ThrustCurve's stored motor values, up to three per impulse class from distinct manufacturers,
leaving out files whose own text contradicts the PD flag. See docs/research/thrustcurve-data.md.

For each selected curve this script:

1. downloads the file with GET https://www.thrustcurve.org/simfiles/<simfileId>/download/data.eng
   (or data.rse for RockSim) into refs/samples/thrustcurve/simfiles/, unless it is already there
   (`--refresh` downloads again); the download date is kept in retrieved.json beside the files;
2. checks the bytes are UTF-8 (they are embedded with include_str!) and parse to exactly the
   samples ThrustCurve's download API returned in the survey captures
   (refs/samples/thrustcurve/download/response-*.json), and that the API gave the file license PD;
3. re-checks, with survey.py's definitions, that total impulse, burn time and average thrust are
   within 1% of the metadata in the pinned refs/snapshots/thrustcurve/motors.json, that the motor
   is a solid in production, and that the file text raises no provenance flag;
4. if every curve passes, writes, byte for byte and replacing what was there:
   - crates/hpr-motor/data/thrustcurve/curves/<simfileId>.eng|.rse (the downloaded bytes, line
     endings untouched; other curve files in that directory are removed);
   - crates/hpr-motor/data/thrustcurve/catalog.json (ThrustCurve's field values and units
     verbatim: numbers are copied as written in motors.json);
   - crates/hpr-motor/src/bundled.rs (include_str! of the index and every curve).

Any failed check stops the script before it writes any output. The output depends only on the
cached inputs, so a second run reproduces it byte for byte.

Run from anywhere (standard library only; needs the survey captures described in survey.py):

    refs/venv/bin/python validation/oracles/thrustcurve/bundle.py
    refs/venv/bin/python validation/oracles/thrustcurve/bundle.py --refresh
"""

import argparse
import base64
import datetime
import glob
import hashlib
import json
import os
import sys
import time
import tomllib
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, HERE)
os.chdir(ROOT)

import survey  # noqa: E402  (after the path set-up above)

LOCK = "validation/refs.lock.toml"
SNAPSHOT_NAME = "thrustcurve-motors"
CACHE = "refs/samples/thrustcurve/simfiles"
RETRIEVED = os.path.join(CACHE, "retrieved.json")
DATA = "crates/hpr-motor/data/thrustcurve"
CURVES = os.path.join(DATA, "curves")
CATALOG = os.path.join(DATA, "catalog.json")
RUST = "crates/hpr-motor/src/bundled.rs"
SITE = "https://www.thrustcurve.org"
USER_AGENT = "hpr-sim catalog builder (https://github.com/nrdptel/hpr-sim)"
EXTENSION = {"RASP": "eng", "RockSim": "rse"}
RUSTFMT_MAX_WIDTH = 100

# (motorId, simfileId), in impulse-class order.
SELECTED = [
    ("5f4294d2000231000000045c", "5f4294d20002e9000000088e"),  # Quest B4
    ("5f4294d20002310000000014", "5f4294d20002e900000008c0"),  # Estes C5
    # Quest D5: the only passing D curve; a user RockSim file matched to NAR's averaged data.
    ("5f4294d20002310000000376", "5f923e071bca580004171648"),
    ("5f4294d200023100000003cf", "5f4294d20002e900000007d3"),  # Cesaroni 26E31-15A
    ("60159d7db94d0e00040a8404", "60159eecb94d0e00040a8433"),  # AeroTech E26W
    ("5f5e57811e865c0004c955d8", "5f5e5a6a1e865c0004c95620"),  # AeroTech F52C
    ("5f4294d20002310000000392", "5f4294d20002e9000000072c"),  # Cesaroni 68F240-15A
    # Estes F15: a user RockSim file matched to NAR's averaged data; the only passing Estes F.
    ("5f4294d200023100000003f2", "5f923edb1bca5800041716ab"),
    ("5f4294d20002310000000396", "5f4294d20002e90000000724"),  # Cesaroni 131G84-10A
    ("5f4294d200023100000001d5", "5f4294d20002e90000000876"),  # AeroTech G69N
    ("5f4294d20002310000000398", "5f4294d20002e90000000735"),  # Cesaroni 168H54-10A
    ("5f4294d20002310000000447", "5f4294d20002e90000000862"),  # Loki H125-CT
    # AeroTech H170M: a manufacturer RockSim file, the only passing AeroTech H curve.
    ("5f4294d20002310000000387", "5f4294d20002e90000000719"),
    ("5f4294d200023100000003a0", "5f4294d20002e9000000075a"),  # Cesaroni 411I175-14A
    ("5f4294d2000231000000046f", "5f4294d20002e900000008bf"),  # AeroTech I175WS
    ("5f4294d20002310000000448", "5f4294d20002e90000000863"),  # Loki I377-CT
    ("5f4294d200023100000003a8", "5f4294d20002e9000000074a"),  # Cesaroni 1266J760-19A
    ("5f4294d20002310000000433", "5f4294d20002e90000000823"),  # Loki J300LR
    ("5f4294d2000231000000044f", "5f4294d20002e9000000086b"),  # AeroTech J450DM
    ("5f4294d200023100000003a9", "5f4294d20002e90000000749"),  # Cesaroni 1633K940-18A
    ("5f4294d20002310000000450", "5f4294d20002e9000000086c"),  # AeroTech K400C
    # AMW 2245K1075-P: the file header names CTI; ThrustCurve's metadata (AMW) is used.
    ("5f4294d200023100000003af", "5f4294d20002e9000000073d"),
    ("5f4294d200023100000003d8", "5f4294d20002e900000007cc"),  # Cesaroni 3300L3200-P
    ("5f4294d20002310000000444", "5f4294d20002e9000000085f"),  # AeroTech L2500ST
    ("5f4294d2000231000000043d", "5f4294d20002e90000000839"),  # Loki L1040LR
    ("5f4294d200023100000003ad", "5f4294d20002e90000000741"),  # Cesaroni 8187M1545-P
    ("5f4294d2000231000000044c", "5f4294d20002e90000000867"),  # Loki M1378LR
    ("5f4294d20002310000000431", "5f4294d20002e90000000875"),  # AeroTech M1350W
    ("5f4294d200023100000003f4", "5f4294d20002e900000007e0"),  # Cesaroni 13628N5600-P
    # AeroTech N3300R: the header says 1060 mm long, the metadata 1046 mm; the metadata wins.
    ("5f4294d2000231000000030d", "5f4294d20002e90000000884"),
    ("5f4294d200023100000003ee", "5f4294d20002e900000007c0"),  # Cesaroni 21062O3400-P
    ("656e92d97f1a4b00027f1588", "656e92ee7f1a4b00027f15ad"),  # AeroTech O6000W
]

SELECTION = (
    "Public-domain (license PD) curves of in-production solid motors whose total impulse, "
    "NFPA 1125 burn time (between the 5%-of-peak crossings) and average thrust (total impulse over "
    "that burn time) are each within 1% of ThrustCurve's stored motor values. Up to three per "
    "impulse class from distinct manufacturers; files whose text contradicts the PD flag are left "
    "out."
)

# catalog.json motor keys, in output order, and the motors.json field each one copies.
MOTOR_FIELDS = [
    ("motor_id", "motorId"),
    ("manufacturer", "manufacturer"),
    ("manufacturer_abbrev", "manufacturerAbbrev"),
    ("designation", "designation"),
    ("common_name", "commonName"),
    ("impulse_class", "impulseClass"),
    ("motor_type", "type"),
    ("availability", "availability"),
    ("cert_org", "certOrg"),
    ("case_info", "caseInfo"),
    ("prop_info", "propInfo"),
    ("delays", "delays"),
    ("delay_adjustable", "delayAdjustable"),
    ("diameter_mm", "diameter"),
    ("length_mm", "length"),
    ("total_impulse_ns", "totImpulseNs"),
    ("average_thrust_n", "avgThrustN"),
    ("max_thrust_n", "maxThrustN"),
    ("burn_time_s", "burnTimeS"),
    ("propellant_mass_g", "propWeightG"),
    ("total_mass_g", "totalWeightG"),
    ("updated_on", "updatedOn"),
]


class Number(str):
    """A JSON number kept exactly as written in the source text."""


def dump(value, level=0):
    """JSON with 2-space indentation, like json.dumps(indent=2), writing Number verbatim."""
    inner, outer = "  " * (level + 1), "  " * level
    if isinstance(value, Number):
        return str(value)
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, dict):
        if not value:
            return "{}"
        items = [f"{inner}{json.dumps(k)}: {dump(v, level + 1)}" for k, v in value.items()]
        return "{\n" + ",\n".join(items) + "\n" + outer + "}"
    if isinstance(value, list):
        if not value:
            return "[]"
        return "[\n" + ",\n".join(inner + dump(v, level + 1) for v in value) + "\n" + outer + "]"
    raise TypeError(f"cannot write {type(value).__name__} to the catalog")


def fail(errors):
    print("bundle: FAILED, nothing written:", file=sys.stderr)
    for e in errors:
        print(f"  - {e}", file=sys.stderr)
    sys.exit(1)


def snapshot_pin():
    with open(LOCK, "rb") as fh:
        lock = tomllib.load(fh)
    pin = next((s for s in lock.get("snapshot", []) if s["name"] == SNAPSHOT_NAME), None)
    if pin is None:
        fail([f"no [[snapshot]] named {SNAPSHOT_NAME} in {LOCK}"])
    with open(pin["dest"], "rb") as fh:
        digest = hashlib.sha256(fh.read()).hexdigest()
    if digest != pin["sha256"]:
        fail([f"{pin['dest']} has sha256 {digest}, but {LOCK} pins {pin['sha256']}"])
    return pin


def api_records():
    records = {}
    for path in sorted(glob.glob(survey.DOWNLOADS)):
        with open(path, encoding="utf-8") as fh:
            for r in json.load(fh)["results"]:
                records[r["simfileId"]] = r
    if not records:
        fail([f"no survey captures matching {survey.DOWNLOADS}; see survey.py"])
    return records


def fetch(simfile_id, ext, refresh, retrieved):
    path = os.path.join(CACHE, f"{simfile_id}.{ext}")
    if refresh or not os.path.exists(path) or simfile_id not in retrieved:
        url = f"{SITE}/simfiles/{simfile_id}/download/data.{ext}"
        request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read()
        os.makedirs(CACHE, exist_ok=True)
        with open(path, "wb") as fh:
            fh.write(data)
        retrieved[simfile_id] = datetime.datetime.now(datetime.UTC).date().isoformat()
        write(RETRIEVED, (json.dumps(dict(sorted(retrieved.items())), indent=2) + "\n").encode())
        print(f"  downloaded {url} ({len(data)} bytes)")
        time.sleep(1.0)
    with open(path, "rb") as fh:
        return fh.read()


def write(path, data):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as fh:
        fh.write(data)


def rust_entry(file):
    path = f'"{file}"'
    include = f'include_str!("../data/thrustcurve/{file}")'
    line = f"    ({path}, {include}),"
    if len(line) <= RUSTFMT_MAX_WIDTH:
        return line + "\n"
    return f"    (\n        {path},\n        {include},\n    ),\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--refresh", action="store_true", help="download every curve again")
    args = ap.parse_args()

    pin = snapshot_pin()
    with open(pin["dest"], encoding="utf-8") as fh:
        text = fh.read()
    motors = {m["motorId"]: m for m in json.loads(text)["results"]}
    verbatim = json.loads(text, parse_float=Number, parse_int=Number)["results"]
    raw = {m["motorId"]: m for m in verbatim}
    records = api_records()
    retrieved = {}
    if os.path.exists(RETRIEVED):
        with open(RETRIEVED, encoding="utf-8") as fh:
            retrieved = json.load(fh)

    errors, curves = [], {}
    definition = next(iter(survey.DEFINITIONS))
    keys = survey.DEFINITIONS[definition]
    if len({s for _, s in SELECTED}) != len(SELECTED):
        errors.append("a simfile is selected twice")
    for motor_id, simfile_id in SELECTED:
        m, rec = motors.get(motor_id), records.get(simfile_id)
        name = f"{simfile_id} ({m['manufacturerAbbrev']} {m['designation']})" if m else simfile_id
        if m is None:
            errors.append(f"{name}: motor {motor_id} is not in {pin['dest']}")
            continue
        if rec is None or rec["motorId"] != motor_id:
            errors.append(f"{name}: no survey capture of this file for motor {motor_id}")
            continue
        if m["type"] == "hybrid" or m["availability"] == "OOP":
            errors.append(f"{name}: motor is {m['type']}, {m['availability']}")
        if rec.get("license") != "PD":
            errors.append(f"{name}: license is {rec.get('license') or 'unset'}, not PD")
        ext = EXTENSION[rec["format"]]
        data = fetch(simfile_id, ext, args.refresh, retrieved)
        try:
            body = data.decode("utf-8")
        except UnicodeDecodeError as e:
            errors.append(f"{name}: not UTF-8 ({e})")
            continue
        parse = survey.parse_rasp if rec["format"] == "RASP" else survey.parse_rse
        _, points, issues = parse(body)
        expected = [(s["time"], s["thrust"]) for s in rec["samples"]]
        if points != expected:
            errors.append(f"{name}: download parses to {len(points)} samples that differ from the "
                          f"{len(expected)} the API returned")
            continue
        if data != base64.b64decode(rec["data"]):
            print(f"  note: {name}: bytes differ from the survey capture; samples are identical")
        flags = survey.provenance_flags(body)
        if flags:
            errors.append(f"{name}: {', '.join(flags)}")
        stats, more = survey.curve_stats(points)
        if stats is None:
            errors.append(f"{name}: no statistics ({'; '.join(issues + more)})")
            continue
        errs = [survey.rel(stats[k], m[mk]) for k, mk in zip(keys, survey.META)]
        bad = [f"{q} {100 * e:+.2f}%" for q, e in zip(survey.QUANTITY, errs)
               if abs(e) > survey.TOL]
        if bad:
            errors.append(f"{name}: outside {100 * survey.TOL:g}%: {', '.join(bad)}")
        curves[simfile_id] = (motor_id, rec, ext, data, errs)
    if errors:
        fail(errors)

    # Everything passed: write the outputs.
    entries = []
    for simfile_id, (motor_id, rec, ext, data, errs) in curves.items():
        file = f"curves/{simfile_id}.{ext}"
        write(os.path.join(DATA, file), data)
        entries.append(file)
        m = motors[motor_id]
        print(f"  {m['impulseClass']} {m['manufacturerAbbrev']:9s} {m['designation']:14s} {file} "
              + " ".join(f"{100 * e:+.2f}%" for e in errs))
    keep = {os.path.basename(f) for f in entries}
    for stale in sorted(glob.glob(os.path.join(CURVES, "*"))):
        if os.path.basename(stale) not in keep:
            os.remove(stale)
            print(f"  removed stale {stale}")

    order = sorted(SELECTED, key=lambda p: (motors[p[0]]["impulseClass"],
                                            motors[p[0]]["manufacturer"],
                                            motors[p[0]]["designation"]))
    by_motor = {}
    for motor_id, simfile_id in order:
        by_motor.setdefault(motor_id, []).append(simfile_id)
    catalog_motors = []
    for motor_id, simfile_ids in by_motor.items():
        r = raw[motor_id]
        entry = {key: r.get(field) for key, field in MOTOR_FIELDS}
        entry["curves"] = []
        for simfile_id in sorted(simfile_ids):
            _, rec, ext, data, _ = curves[simfile_id]
            entry["curves"].append({
                "simfile_id": simfile_id,
                "format": rec["format"],
                "source": rec["source"],
                "license": rec["license"],
                "file": f"curves/{simfile_id}.{ext}",
                "sha256": hashlib.sha256(data).hexdigest(),
                "url": f"{SITE}/simfiles/{simfile_id}/download/data.{ext}",
                "info_url": SITE + rec["infoUrl"],
                "retrieved": retrieved[simfile_id],
            })
        catalog_motors.append(entry)
    catalog = {
        "source": "ThrustCurve.org API v1",
        "snapshot": {
            "motors_url": pin["url"],
            "captured": str(pin["captured"]),
            "sha256": pin["sha256"],
        },
        "selection": SELECTION,
        "motors": catalog_motors,
    }
    write(CATALOG, (dump(catalog) + "\n").encode("utf-8"))

    rust = [
        "//! The bundled ThrustCurve.org catalog, generated by "
        "`validation/oracles/thrustcurve/bundle.py`.\n",
        "//! Regenerate it with that script instead of editing it.\n",
        "\n",
        "/// The catalog index: ThrustCurve.org metadata and per-curve provenance.\n",
        'pub(crate) const CATALOG_JSON: &str = include_str!("../data/thrustcurve/catalog.json");\n',
        "\n",
        "/// Each bundled curve file's text, by the path in the index.\n",
        "pub(crate) const CURVE_FILES: &[(&str, &str)] = &[\n",
        *(rust_entry(f) for f in sorted(entries)),
        "];\n",
    ]
    write(RUST, "".join(rust).encode("utf-8"))

    for path in [CATALOG, RUST, *(os.path.join(DATA, f) for f in sorted(entries))]:
        with open(path, "rb") as fh:
            print(f"{hashlib.sha256(fh.read()).hexdigest()}  {path}")
    print(f"bundle: {len(catalog_motors)} motors, {len(entries)} curves")


if __name__ == "__main__":
    main()
