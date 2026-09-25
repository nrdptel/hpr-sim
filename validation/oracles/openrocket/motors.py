"""What OpenRocket 24.12 makes of each thrust-curve file hpr flies.

hpr reads a RASP `.eng` or RockSim `.rse` file into a `ThrustCurve` and integrates it for the
motor's total impulse. OpenRocket reads the same files with its own loaders and reports its own
estimate. This script asks it, file by file, so that a test can hold hpr's total impulse to
OpenRocket's (M2.2c1) and so that any other quantity that differs is measured rather than assumed.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names are the public API `javap` prints for the jar.

Run from the repository root with the oracle environment and Java 17 (see `automatic_radius.py`):

    refs/venv/bin/python validation/oracles/openrocket/motors.py \\
        validation/fixtures/motor/openrocket-curve-stats.json \\
        crates/hpr-motor/data/thrustcurve/curves

The record is written to the path given, not to standard output, which OpenRocket logs to. Each
input may be a curve file or a directory of them; directories are walked in sorted order. Paths in
the record are relative to the repository root, so the record is the same wherever it is written.
"""

import hashlib
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start, with empty databases

GENERATED = "2026-09-25"

#: The extensions OpenRocket's general motor loader accepts and hpr reads.
SUFFIXES = (".eng", ".rse")


def curve_files(inputs):
    """Every curve file under the inputs, sorted, as repository-relative paths."""
    found = []
    for raw in inputs:
        path = Path(raw)
        if path.is_dir():
            found.extend(
                sorted(p for p in path.rglob("*") if p.suffix.lower() in SUFFIXES and p.is_file())
            )
        elif path.suffix.lower() in SUFFIXES:
            found.append(path)
        else:
            sys.exit(f"not a curve file or a directory: {raw}")
    return found


def delay_of(value):
    """One standard delay, in seconds, or `"plugged"` for OpenRocket's infinite delay."""
    return float(value) if math.isfinite(float(value)) else "plugged"


def read(path):
    """Every motor in one curve file, as OpenRocket builds it."""
    from java.io import File
    from info.openrocket.swing.file.motor import MotorLoaderHelper

    motors = []
    for builder in MotorLoaderHelper.load(File(str(path.resolve()))):
        motor = builder.build()
        motors.append(
            {
                "designation": str(motor.getDesignation()),
                "common_name": str(motor.getCommonName()),
                "manufacturer": str(motor.getManufacturer().getSimpleName()),
                "digest": str(motor.getDigest()),
                "motor_type": str(motor.getMotorType().getName()),
                "diameter_m": float(motor.getDiameter()),
                "length_m": float(motor.getLength()),
                "launch_mass_kg": float(motor.getLaunchMass()),
                "burnout_mass_kg": float(motor.getBurnoutMass()),
                "propellant_mass_kg": float(motor.getPropellantMass()),
                "delays_s": [delay_of(value) for value in motor.getStandardDelays()],
                "points": int(len(motor.getTimePoints())),
                "first_time_s": float(motor.getTimePoints()[0]),
                "last_time_s": float(motor.getTimePoints()[-1]),
                "total_impulse_ns": float(motor.getTotalImpulseEstimate()),
                "average_thrust_n": float(motor.getAverageThrustEstimate()),
                "max_thrust_n": float(motor.getMaxThrustEstimate()),
                "burn_time_s": float(motor.getBurnTime()),
            }
        )
    return motors


def main():
    if len(sys.argv) < 3:
        sys.exit(f"usage: {Path(__file__).name} <output.json> <curve file or directory>...")
    output = Path(sys.argv[1])
    files = curve_files(sys.argv[2:])
    if not files:
        sys.exit("no curve files found")

    automatic_radius.start()
    import jpype
    from java.lang import System
    from info.openrocket.core.util import BuildProperties

    curves = []
    for path in files:
        entry = {"file": path.as_posix(), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        try:
            entry["motors"] = read(path)
        except Exception as error:  # noqa: BLE001 - recorded, and the test fails on it
            entry["driver_error"] = str(error).splitlines()[0]
        curves.append(entry)

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/motors.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    "motors.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "automatic_radius.py": hashlib.sha256(
                        Path(automatic_radius.__file__).read_bytes()
                    ).hexdigest(),
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["motors.py", *sys.argv[1:]]),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "curves": curves,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
