"""Whether OpenRocket reads the motor files `hpr convert` writes as it reads the originals.

`hpr convert` writes a motor as a RASP `.eng` or a RockSim `.rse` file (ADR-107). hpr reads its
own output back exactly; this script asks the other program that reads both formats. For each of
the 32 bundled curve files it converts the file to the other format with `hpr convert`, loads the
original and the converted file with OpenRocket 24.12's own motor loaders, and compares what
OpenRocket reads from each: designation, maker, type, diameter, length, launch and burnout mass,
the thrust curve's points, and the delays, as a set: OpenRocket sorts a `.eng` file's delays, not
a `.rse` file's. The maker is the name OpenRocket's maker registry gives it, which is shared by
every file one run loads, so a maker first read as `Estes Industries, Inc.` keeps that name when
`Estes_Industries,_Inc.` is read later: it shows that OpenRocket takes both for one maker, not
that the text is the same.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar (`RASPMotorLoader`,
`RockSimMotorLoader`, `ThrustCurveMotor`).

OpenRocket 24.12 needs Java 17 exactly. Set JAVA_HOME to a Java 17 home, or install Homebrew's
keg-only `openjdk@17`, which is found without it. Build `hpr` first (`cargo build -p hpr-cli`),
then run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/motor_files.py

It prints one line per file that OpenRocket reads differently, and a summary; then whether
OpenRocket reads a `.eng` header of eight fields, a maker of two words, which is why `hpr convert`
joins such a maker with `_`. It writes nothing.
"""

import math
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import jpype
import jpype.imports

JAR = Path("refs/openrocket/OpenRocket-24.12.jar")
CURVES = Path("crates/hpr-motor/data/thrustcurve/curves")
HPR = Path("target/debug/hpr")
KEGS = [
    "/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
    "/usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
]
# Relative difference allowed between two readings of a number: OpenRocket reads `.eng` masses in
# kg and `.rse` masses in g, so the same mass can differ in its last bits.
RELATIVE = 1e-12


def java_home():
    for home in [os.environ.get("JAVA_HOME")] + KEGS:
        if not home or not (Path(home) / "release").is_file():
            continue
        release = (Path(home) / "release").read_text(encoding="utf-8")
        if re.search(r'^JAVA_VERSION="17[."]', release, re.MULTILINE):
            return Path(home)
    sys.exit("no Java 17 found: OpenRocket 24.12 refuses any other; set JAVA_HOME to a Java 17 home")


def start():
    home = java_home()
    for lib in ["lib/server/libjvm.dylib", "lib/server/libjvm.so", "bin/server/jvm.dll"]:
        if (home / lib).exists():
            jpype.startJVM(str(home / lib), "-Djava.awt.headless=true", classpath=[str(JAR)])
            return
    sys.exit(f"no JVM library under {home}")


def load(path):
    """OpenRocket's reading of the motor file at `path`: one dict per motor, or an error."""
    from java.io import StringReader
    from info.openrocket.core.file.motor import RASPMotorLoader, RockSimMotorLoader

    loader = RASPMotorLoader() if path.suffix == ".eng" else RockSimMotorLoader()
    text = path.read_text(encoding="utf-8")
    try:
        builders = loader.load(StringReader(text), path.name)
        motors = [builder.build() for builder in builders]
    except Exception as error:  # noqa: BLE001 - any refusal is the answer
        return f"refused: {error}"
    return [
        {
            "designation": str(motor.getDesignation()),
            "maker": str(motor.getManufacturer().getSimpleName()),
            "type": str(motor.getMotorType()),
            "diameter": float(motor.getDiameter()),
            "length": float(motor.getLength()),
            "launch mass": float(motor.getLaunchMass()),
            "burnout mass": float(motor.getBurnoutMass()),
            "times": [float(t) for t in motor.getTimePoints()],
            "thrusts": [float(f) for f in motor.getThrustPoints()],
            # OpenRocket sorts a `.eng` file's delays but keeps a `.rse` file's order.
            "delays": sorted(float(d) for d in motor.getStandardDelays()),
        }
        for motor in motors
    ]


def same(a, b):
    if isinstance(a, float):
        return a == b or math.isclose(a, b, rel_tol=RELATIVE, abs_tol=0.0)
    if isinstance(a, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b


def main():
    if not HPR.is_file():
        sys.exit(f"no {HPR}: run `cargo build -p hpr-cli` first")
    start()
    files = sorted(CURVES.glob("*.eng")) + sorted(CURVES.glob("*.rse"))
    loaded = {".eng": 0, ".rse": 0}
    written = {".eng": 0, ".rse": 0}
    agree = 0
    differences = {}
    with tempfile.TemporaryDirectory() as scratch:
        for original in files:
            other = ".rse" if original.suffix == ".eng" else ".eng"
            converted = Path(scratch) / (original.stem + other)
            subprocess.run(
                [str(HPR), "convert", str(original), str(converted)],
                check=True,
                capture_output=True,
            )
            before, after = load(original), load(converted)
            written[other] += 1
            if isinstance(after, str):
                print(f"{original.name} -> {other}: {after}")
                continue
            loaded[other] += 1
            if isinstance(before, str):
                print(f"{original.name}: OpenRocket refuses the original: {before}")
                continue
            fields = sorted(
                {
                    field
                    for a, b in zip(before, after)
                    for field in a
                    if not same(a[field], b[field])
                }
                | ({"motor count"} if len(before) != len(after) else set())
            )
            if fields:
                for field in fields:
                    differences[field] = differences.get(field, 0) + 1
                shown = [
                    f"{field}: {before[0].get(field)!r} -> {after[0].get(field)!r}"
                    for field in fields
                    if field not in ("times", "thrusts")
                ] + [field for field in fields if field in ("times", "thrusts")]
                print(f"{original.name} -> {other}: " + "; ".join(shown))
            else:
                agree += 1
    print(
        f"OpenRocket 24.12 loads {loaded['.eng']} of {written['.eng']} .eng and "
        f"{loaded['.rse']} of {written['.rse']} .rse files hpr convert wrote; "
        f"{agree} of {len(files)} read as their originals do"
        + (
            ", the rest differing in "
            + ", ".join(f"{field} ({count})" for field, count in sorted(differences.items()))
            if differences
            else ""
        )
    )
    with tempfile.TemporaryDirectory() as scratch:
        eight = Path(scratch) / "eight.eng"
        eight.write_text("X1 29 100 P 0.04 0.08 Some Maker\n 0.5 20\n 1 0\n", encoding="utf-8")
        joined = Path(scratch) / "joined.eng"
        joined.write_text("X1 29 100 P 0.04 0.08 Some_Maker\n 0.5 20\n 1 0\n", encoding="utf-8")
        for what, path in [("eight fields", eight), ("the maker joined, seven", joined)]:
            read = load(path)
            print(f"a .eng header of {what}: " + (read if isinstance(read, str) else "read"))


if __name__ == "__main__":
    main()
