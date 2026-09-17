"""Survey of ThrustCurve.org thrust curves for the M1.3 offline motor catalog.

For every simulator file (simfile) of a solid motor (hybrids excluded) this script:

1. counts files by license (PD, free, other, none), data source (cert, mfr, user) and format
   (RASP .eng, RockSim .rse), overall and for motors still in production;
2. parses each file (RASP and RockSim, from the public format descriptions) and checks the parse
   against the `samples` array that ThrustCurve returns for the same file;
3. computes from the curve:
   - total impulse: trapezoid rule over every sample, with an implicit (0 s, 0 N) point before the
     first sample when the file does not start at t = 0;
   - NFPA 1125 burn time: from the first time thrust rises to 5% of peak to the last time it falls
     to 5% of peak, crossings linearly interpolated (the curve is piecewise linear); if the last
     sample is still above 5% of peak the burn ends at the last sample;
   - "last action" burn time: time of the last sample;
   - average thrust three ways: whole-curve impulse / NFPA burn time; impulse inside the NFPA
     window / NFPA burn time; whole-curve impulse / last-action burn time;
4. compares them with the motor-level metadata (totImpulseNs, burnTimeS, avgThrustN) and reports
   how many files match all three within 1% under each definition, the error distribution, and
   why public-domain files fail;
5. flags files whose text mentions a copyright or OpenRocket (provenance to check by hand), and
   whose header diameter or length disagrees with the metadata;
6. proposes a bundle: up to three current-production, public-domain curves per impulse class, from
   distinct manufacturers, that pass all three checks under the ThrustCurve definition and carry
   no provenance flag, with each file's download URL and sha256 for pinning.

Definitions are those on https://www.thrustcurve.org/info/motorstats.html (Burn Time
Normalization, based on NFPA 1125) and https://www.thrustcurve.org/info/glossary.html.

Inputs (all cached under the gitignored refs/; nothing is fetched):

- refs/snapshots/thrustcurve/motors.json: GET https://www.thrustcurve.org/api/v1/search.json
  ?maxResults=10000 (pinned in validation/refs.lock.toml).
- refs/samples/thrustcurve/download/response-NN.json: POST
  https://www.thrustcurve.org/api/v1/download.json with the JSON body in request-NN.json, that is
  {"motorIds": [...250 ids...], "data": "both"}, for every non-hybrid motor with dataFiles > 0,
  in motors.json order, a few seconds apart.

Run from the repository root (standard library only):

    refs/venv/bin/python validation/oracles/thrustcurve/survey.py            # summary
    refs/venv/bin/python validation/oracles/thrustcurve/survey.py --failures # plus PD failures
    refs/venv/bin/python validation/oracles/thrustcurve/survey.py --passing  # plus PD passes
    refs/venv/bin/python validation/oracles/thrustcurve/survey.py --json out.json  # per file
"""

import argparse
import base64
import glob
import hashlib
import json
import math
import os
import re
import statistics
import sys
import xml.etree.ElementTree as ET
from collections import Counter, defaultdict

MOTORS = "refs/snapshots/thrustcurve/motors.json"
DOWNLOADS = "refs/samples/thrustcurve/download/response-*.json"
TOL = 0.01  # the M1.3 done-when tolerance
CUTOFF = 0.05  # NFPA 1125: 5% of peak thrust
CLASSES = "ABCDEFGHIJKLMNO"
SOURCE_RANK = {"cert": 0, "mfr": 1, "user": 2}
PER_CLASS = 3  # bundle up to this many curves per impulse class, from distinct manufacturers


# ---------------------------------------------------------------------------------------------
# Parsers


def parse_rasp(text):
    """RASP (.eng): ';' comments, a header line, then time/thrust pairs (s, N)."""
    header, points, issues = None, [], []
    for raw in text.splitlines():
        line = raw.split(";", 1)[0].strip()
        if not line:
            continue
        if header is None:
            f = line.split()
            header = {"name": f[0]}
            try:
                header["diameter_mm"] = float(f[1])
                header["length_mm"] = float(f[2])
                header["delays"] = f[3]
                header["prop_mass_kg"] = float(f[4])
                header["total_mass_kg"] = float(f[5])
                header["mfr"] = " ".join(f[6:])
            except (IndexError, ValueError):
                issues.append("bad header")
            continue
        f = line.split()
        if len(f) != 2:
            issues.append("extra content")
            break
        try:
            points.append((float(f[0]), float(f[1])))
        except ValueError:
            issues.append("bad data line")
            break
    return header or {}, points, issues


def parse_rse(text):
    """RockSim (.rse): XML engine element with eng-data t (s) and f (N) samples."""
    issues = []
    try:
        root = ET.fromstring(text.strip())
    except ET.ParseError:
        return {}, [], ["bad XML"]
    engines = root.findall(".//engine")
    if not engines:
        return {}, [], ["no engine"]
    if len(engines) > 1:
        issues.append("multiple engines")
    e = engines[0]
    header = {"name": e.get("code", "")}
    for key, attr in (("diameter_mm", "dia"), ("length_mm", "len")):
        try:
            header[key] = float(e.get(attr))
        except (TypeError, ValueError):
            pass
    points = []
    for d in e.findall("./data/eng-data"):
        try:
            points.append((float(d.get("t")), float(d.get("f"))))
        except (TypeError, ValueError):
            issues.append("bad eng-data")
    return header, points, issues


# ---------------------------------------------------------------------------------------------
# Curve statistics


def crossing(p0, p1, level):
    """Time at which the segment p0-p1 reaches `level` (linear)."""
    (t0, f0), (t1, f1) = p0, p1
    if f1 == f0 or t1 == t0:
        return t0
    return t0 + (level - f0) * (t1 - t0) / (f1 - f0)


def segment_area(p0, p1, ta, tb):
    """Integral of the linear segment p0-p1 over [ta, tb] (clipped to the segment)."""
    (t0, f0), (t1, f1) = p0, p1
    a, b = max(ta, t0), min(tb, t1)
    if b <= a or t1 == t0:
        return 0.0

    def f(t):
        return f0 + (f1 - f0) * (t - t0) / (t1 - t0)

    return 0.5 * (f(a) + f(b)) * (b - a)


def curve_stats(points, cutoff=CUTOFF):
    issues = []
    if len(points) < 2:
        return None, ["too few points"]
    if any(t < 0 or f < 0 for t, f in points):
        issues.append("negative value")
    if any(b[0] < a[0] for a, b in zip(points, points[1:])):
        issues.append("time goes backwards")
        return None, issues
    if any(b[0] == a[0] for a, b in zip(points, points[1:])):
        # A repeated time is a vertical step; it adds no area and is kept.
        issues.append("repeated time")
    pts = list(points)
    if pts[0][0] > 0:
        # A leading zero-thrust sample after t = 0 is kept here; ThrustCurve's own analysis drops
        # leading zero samples first, which moves the implicit origin triangle. Flag it.
        if pts[0][1] == 0:
            issues.append("leading zero sample after t=0")
        pts.insert(0, (0.0, 0.0))
    peak = max(f for _, f in pts)
    if peak <= 0:
        return None, issues + ["no thrust"]
    level = cutoff * peak
    first = next(i for i, (_, f) in enumerate(pts) if f >= level)
    last = max(i for i, (_, f) in enumerate(pts) if f >= level)
    start = pts[0][0] if first == 0 else crossing(pts[first - 1], pts[first], level)
    if last == len(pts) - 1:
        end = pts[-1][0]
        issues.append("ends above 5% of peak")
    else:
        end = crossing(pts[last], pts[last + 1], level)
    total = sum(0.5 * (a[1] + b[1]) * (b[0] - a[0]) for a, b in zip(pts, pts[1:]))
    window = sum(segment_area(a, b, start, end) for a, b in zip(pts, pts[1:]))
    t_nfpa = end - start
    t_last = pts[-1][0]
    return {
        "peak_n": peak,
        "impulse_ns": total,
        "impulse_window_ns": window,
        "burn_nfpa_s": t_nfpa,
        "burn_last_s": t_last,
        "burn_start_s": start,
        "burn_end_s": end,
        "avg_total_over_nfpa_n": total / t_nfpa if t_nfpa > 0 else math.nan,
        "avg_window_over_nfpa_n": window / t_nfpa if t_nfpa > 0 else math.nan,
        "avg_total_over_last_n": total / t_last if t_last > 0 else math.nan,
    }, issues


def provenance_flags(text):
    """Text in a curve file that contradicts a public-domain flag: check such files by hand."""
    flags = []
    if re.search(r"copyright|\(c\)|\u00a9", text, re.I):
        flags.append("mentions copyright")
    if re.search(r"openrocket", text, re.I):
        flags.append("mentions OpenRocket")
    return flags


# The definition sets compared with the metadata: (impulse, burn time, average thrust).
DEFINITIONS = {
    "A ThrustCurve/NFPA 1125: I whole curve, t 5%-5%, F = I/t": (
        "impulse_ns", "burn_nfpa_s", "avg_total_over_nfpa_n"),
    "B I whole curve, t 5%-5%, F = I(window)/t": (
        "impulse_ns", "burn_nfpa_s", "avg_window_over_nfpa_n"),
    "C last action: I whole curve, t = last sample, F = I/t": (
        "impulse_ns", "burn_last_s", "avg_total_over_last_n"),
}
META = ("totImpulseNs", "burnTimeS", "avgThrustN")
QUANTITY = ("impulse", "burn time", "avg thrust")


def rel(a, b):
    return a / b - 1.0 if b else math.inf


def resolution(v):
    """Relative half-unit of the last printed digit of a metadata value (rounding limit)."""
    s = repr(float(v))
    if "e" in s or "E" in s:
        return 0.0
    decimals = len(s.split(".")[1].rstrip("0")) if "." in s else 0
    return 0.5 * 10 ** (-decimals) / abs(v) if v else math.inf


# ---------------------------------------------------------------------------------------------


def load():
    motors = {m["motorId"]: m for m in json.load(open(MOTORS))["results"]}
    files = []
    paths = sorted(glob.glob(DOWNLOADS))
    if not paths:
        sys.exit(f"no cached downloads matching {DOWNLOADS}; see the module docstring")
    for p in paths:
        files += json.load(open(p))["results"]
    return motors, files


def pct(x):
    return f"{100 * x:+.2f}%"


def quantiles(values):
    v = sorted(values)
    if not v:
        return "n/a"
    q = lambda p: v[min(len(v) - 1, int(round(p * (len(v) - 1))))]  # noqa: E731
    return (f"median {100 * q(0.5):.2f}%  p90 {100 * q(0.9):.2f}%  p95 {100 * q(0.95):.2f}%  "
            f"max {100 * v[-1]:.1f}%")


def buckets(values):
    edges = [0.005, 0.01, 0.02, 0.05, 0.10]
    counts = [0] * (len(edges) + 1)
    for x in values:
        counts[next((i for i, e in enumerate(edges) if x <= e), len(edges))] += 1
    labels = ["<=0.5%", "<=1%", "<=2%", "<=5%", "<=10%", ">10%"]
    return "  ".join(f"{l} {c}" for l, c in zip(labels, counts))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--failures", action="store_true", help="list failing PD files")
    ap.add_argument("--passing", action="store_true", help="list passing PD files")
    ap.add_argument("--json", help="write per-file results to this path")
    args = ap.parse_args()
    if not os.path.exists(MOTORS):
        sys.exit(f"missing {MOTORS}; run `cargo xtask refs fetch`")

    motors, files = load()
    solids = {k: m for k, m in motors.items() if m["type"] != "hybrid"}
    print(f"motors.json: {len(motors)} motors; {len(solids)} solid "
          f"({Counter(m['type'] for m in solids.values())}); availability "
          f"{dict(Counter(m['availability'] for m in solids.values()))}; with data files "
          f"{sum(1 for m in solids.values() if m['dataFiles'])}")
    files = [f for f in files if f["motorId"] in solids]
    print(f"simfiles downloaded for solids: {len(files)} "
          f"(metadata dataFiles sum {sum(m['dataFiles'] for m in solids.values())})")

    def lic(f):
        return f.get("license") or "none"

    print("\n== Licenses and sources ==")
    for label, sel in (("all solids", files),
                       ("in production", [f for f in files
                                          if solids[f["motorId"]]["availability"] != "OOP"])):
        print(f"{label}: {len(sel)} files")
        print("  license:", dict(Counter(lic(f) for f in sel)))
        print("  source: ", dict(Counter(f["source"] for f in sel)))
        print("  format: ", dict(Counter(f["format"] for f in sel)))
        pairs = Counter((lic(f), f["source"]) for f in sel)
        print("  license x source:", dict(sorted(pairs.items())))
    pd_files = [f for f in files if lic(f) == "PD"]
    print(f"PD: {len(pd_files)} files for {len({f['motorId'] for f in pd_files})} motors")

    results = []
    curves = {}
    sample_mismatch = 0
    for f in files:
        m = solids[f["motorId"]]
        text = base64.b64decode(f["data"]).decode("latin-1")
        if f["format"] == "RASP":
            header, points, issues = parse_rasp(text)
        else:
            header, points, issues = parse_rse(text)
        api = [(s["time"], s["thrust"]) for s in f.get("samples", [])]
        if api != points:
            sample_mismatch += 1
            issues.append("parse differs from API samples")
        curves[f["simfileId"]] = points
        stats, more = curve_stats(points)
        issues += more
        flags = provenance_flags(text)
        if "diameter_mm" in header and abs(header["diameter_mm"] - m["diameter"]) > 0.5:
            flags.append("header diameter differs")
        if "length_mm" in header and abs(header["length_mm"] - m["length"]) > 1.0:
            flags.append("header length differs")
        r = {
            "simfileId": f["simfileId"], "motorId": f["motorId"], "format": f["format"],
            "source": f["source"], "license": lic(f), "manufacturer": m["manufacturerAbbrev"],
            "designation": m["designation"], "commonName": m["commonName"],
            "impulseClass": m["impulseClass"], "availability": m["availability"],
            "certOrg": m["certOrg"], "points": len(points), "issues": issues, "flags": flags,
            "dataUrl": "https://www.thrustcurve.org" + f["dataUrl"],
            "sha256": hashlib.sha256(base64.b64decode(f["data"])).hexdigest(),
            "stats": stats, "errors": {},
        }
        if stats:
            for name, keys in DEFINITIONS.items():
                r["errors"][name] = [rel(stats[k], m[mk]) for k, mk in zip(keys, META)]
        results.append(r)
    print(f"\nparsed files differing from ThrustCurve's samples: {sample_mismatch}")
    print("issues (all files):", dict(Counter(i for r in results for i in r["issues"])))
    print("flags (PD files):  ", dict(Counter(x for r in results if r["license"] == "PD"
                                             for x in r["flags"])))

    def passes(r, name):
        e = r["errors"].get(name)
        return e is not None and all(abs(x) <= TOL for x in e)

    print("\n== Which definitions the metadata follows (files within 1%) ==")
    for label, sel in (("all solid files", results), ("PD files", [r for r in results
                                                                   if r["license"] == "PD"])):
        n = len(sel)
        print(f"{label} (n={n}):")
        for name in DEFINITIONS:
            each = [sum(1 for r in sel if name in r["errors"] and abs(r["errors"][name][i]) <= TOL)
                    for i in range(3)]
            allp = sum(1 for r in sel if passes(r, name))
            print(f"  {name}\n    impulse {each[0]}  burn time {each[1]}  avg thrust {each[2]}"
                  f"  -> all three {allp} ({100 * allp / n:.1f}%)")

    print("\n== Burn time: which cut-off matches burnTimeS within 1% ==")
    for label, sel in (("all solid files", results), ("PD files", [r for r in results
                                                                   if r["license"] == "PD"])):
        row = []
        for cut in ("last sample", 0.02, 0.03, 0.05, 0.07, 0.10):
            n = 0
            for r in sel:
                if cut == "last sample":
                    st = r["stats"]
                    t = st["burn_last_s"] if st else None
                else:
                    st, _ = curve_stats(curves[r["simfileId"]], cut)
                    t = st["burn_nfpa_s"] if st else None
                meta = solids[r["motorId"]]["burnTimeS"]
                n += t is not None and abs(rel(t, meta)) <= TOL
            row.append(f"{cut if isinstance(cut, str) else f'{100 * cut:.0f}%'} {n}")
        print(f"  {label} (n={len(sel)}): " + "  ".join(row))

    print("\n== Metadata self-consistency: avgThrustN * burnTimeS / totImpulseNs ==")
    consistent = {}
    for k, m in solids.items():
        consistent[k] = abs(m["avgThrustN"] * m["burnTimeS"] / m["totImpulseNs"] - 1) <= TOL
    ratios = [m["avgThrustN"] * m["burnTimeS"] / m["totImpulseNs"] - 1 for m in solids.values()]
    print(f"  solid motors within 1%: {sum(consistent.values())} of {len(solids)}; "
          f"median {pct(statistics.median(ratios))}")
    orgs = defaultdict(list)
    for k, m in solids.items():
        orgs[m["certOrg"]].append(consistent[k])
    print("  by certOrg: " + ", ".join(f"{o} {sum(v)}/{len(v)}" for o, v in sorted(orgs.items())))

    A = next(iter(DEFINITIONS))
    pd = [r for r in results if r["license"] == "PD" and r["stats"]]
    print("\n== PD error distribution, definition A (|computed/metadata - 1|) ==")
    for i, q in enumerate(QUANTITY):
        v = [abs(r["errors"][A][i]) for r in pd]
        print(f"  {q:10s} {quantiles(v)}\n  {'':10s} {buckets(v)}")
    for name in list(DEFINITIONS)[1:]:
        i = 2 if name.startswith("B") else 1
        v = [abs(r["errors"][name][i]) for r in pd]
        print(f"  {QUANTITY[i]} under {name[:1]}: {quantiles(v)}")

    print("\n== PD pass rate (definition A) by group ==")
    for key in ("source", "format", "availability", "impulseClass", "manufacturer"):
        groups = defaultdict(list)
        for r in [r for r in results if r["license"] == "PD"]:
            groups[r[key]].append(r)
        print(f"  {key}: " + ", ".join(f"{g} {sum(passes(r, A) for r in rs)}/{len(rs)}"
                                       for g, rs in sorted(groups.items())))

    print("\n== Why PD files fail (definition A) ==")
    reasons = Counter()
    fail = [r for r in results if r["license"] == "PD" and not passes(r, A)]
    for r in fail:
        if not r["stats"]:
            reasons["unparseable: " + "; ".join(r["issues"])] += 1
            continue
        m = solids[r["motorId"]]
        bad = [i for i, e in enumerate(r["errors"][A]) if abs(e) > TOL]
        if len(bad) == 1 and bad[0] == 1 and "ends above 5% of peak" in r["issues"]:
            reasons["burn time only; curve ends above 5% of peak"] += 1
            continue
        coarse = [i for i in bad if resolution(m[META[i]]) > TOL]
        what = "+".join(QUANTITY[i] for i in bad)
        if coarse and len(coarse) == len(bad):
            reasons[f"{what} (metadata rounded coarser than 1%)"] += 1
        elif abs(r["errors"][A][0]) > TOL:
            big = abs(r["errors"][A][0]) > 0.05
            reasons[f"{what} (impulse off {'>5%' if big else '1-5%'})"] += 1
        else:
            reasons[f"{what} (impulse within 1%)"] += 1
    for k, n in reasons.most_common():
        print(f"  {n:4d}  {k}")
    pd_all = [r for r in results if r["license"] == "PD"]
    pd_cons = [r for r in pd_all if consistent[r["motorId"]]]
    n_cons = sum(passes(r, A) for r in pd_cons)
    n_all = sum(passes(r, A) for r in pd_all)
    print(f"PD files whose motor metadata is self-consistent within 1%: {len(pd_cons)}, passing "
          f"{n_cons}; inconsistent: {len(pd_all) - len(pd_cons)}, passing {n_all - n_cons}")
    motors_any = {r["motorId"] for r in results if r["license"] == "PD" and passes(r, A)}
    motors_pd = {r["motorId"] for r in results if r["license"] == "PD"}
    print(f"PD motors with at least one passing PD file: {len(motors_any)} of {len(motors_pd)}")

    if args.failures:
        print("\n== Failing PD files (definition A): impulse, burn time, avg thrust errors ==")
        for r in sorted(fail, key=lambda r: (CLASSES.find(r["impulseClass"]), r["manufacturer"])):
            e = r["errors"].get(A)
            es = " ".join(pct(x) for x in e) if e else "-"
            print(f"  {r['impulseClass']} {r['manufacturer']:9s} {r['designation']:18s} "
                  f"{r['simfileId']} {r['format']:7s} {r['source']:4s} {es}  "
                  f"{'; '.join(r['issues'] + r['flags'])}")

    good = [r for r in results if r["license"] == "PD" and passes(r, A) and not
            any(x.startswith("mentions") for x in r["flags"]) and "non-monotonic time" not in
            r["issues"]]
    if args.passing:
        print("\n== Passing PD files (definition A, no provenance flag) ==")
        for r in sorted(good, key=lambda r: (CLASSES.find(r["impulseClass"]), r["manufacturer"],
                                             r["designation"])):
            e = r["errors"][A]
            print(f"  {r['impulseClass']} {r['manufacturer']:9s} {r['designation']:18s} "
                  f"{r['motorId']} {r['simfileId']} {r['format']:7s} {r['source']:4s} "
                  f"{r['availability']:10s} {' '.join(pct(x) for x in e)} {'; '.join(r['flags'])}")

    print(f"\n== Proposed bundle: up to {PER_CLASS} per class from distinct manufacturers; in "
          "production; cert > mfr > user, RASP first, then smallest worst error ==")
    by_class = defaultdict(list)
    for r in good:
        if r["availability"] == "OOP":
            continue
        worst = max(abs(x) for x in r["errors"][A])
        key = (SOURCE_RANK[r["source"]], r["format"] != "RASP", bool(r["flags"]), worst)
        by_class[r["impulseClass"]].append((key, r))
    bundle = []
    for c in CLASSES:
        seen_mfr, seen_motor = set(), set()
        for _, r in sorted(by_class[c], key=lambda kr: kr[0]):
            if r["manufacturer"] in seen_mfr or r["motorId"] in seen_motor:
                continue
            seen_mfr.add(r["manufacturer"])
            seen_motor.add(r["motorId"])
            bundle.append(r)
            if len(seen_mfr) == PER_CLASS:
                break
    for r in bundle:
        print(f"  {r['impulseClass']} {r['manufacturer']:9s} {r['designation']:14s} "
              f"motor {r['motorId']} simfile {r['simfileId']} {r['format']:7s} {r['source']:4s} "
              f"{' '.join(pct(x) for x in r['errors'][A])} {'; '.join(r['flags'])}\n"
              f"      {r['dataUrl']} sha256 {r['sha256']}")
    missing = [c for c in CLASSES if not any(r["impulseClass"] == c for r in bundle)]
    print(f"  {len(bundle)} curves; classes without a candidate: {missing or 'none'}")

    if args.json:
        with open(args.json, "w") as fh:
            json.dump({"results": results, "bundle": [r["simfileId"] for r in bundle]}, fh,
                      indent=1)
        print(f"wrote {args.json}")


if __name__ == "__main__":
    main()
