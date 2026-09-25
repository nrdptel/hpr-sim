"""OpenRocket 24.12's own motor database, and what it makes of the curves designs embed.

A `<motor>` in a `.ork` names a motor and records OpenRocket's digest of its thrust curve. Most
designs do not carry the curve itself: OpenRocket finds it in the motor database shipped inside its
jar. hpr bundles only 32 curves, so without that database most of a design library cannot fly
(M2.2c2). This script asks OpenRocket for every motor in the database it loads, with its digest,
its sampled curve and its own integrals, so that `cargo xtask ork` can supply the curve a design's
digest names and hold hpr's integration of each curve to OpenRocket's. It also hands every curve a
design embeds (`thrustcurves/<digest>.rse`) to OpenRocket's own loader, as `motors.py` does for the
bundled curves (M2.2c1). Last, it opens every design with that database bound, as the program
does, and records the digest of the motor OpenRocket puts in each mount of each configuration, so
that the survey can say whether OpenRocket flies the very curve a design's digest names.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names are the public API `javap` prints for the jar. The database's contents come from
ThrustCurve.org by way of OpenRocket; their terms are unstated, so the record is written only under
the gitignored `corpus-out/`, and so are the embedded curves' numbers, which belong to the designs'
owners.

Run from the repository root with the oracle environment and Java 17 (see `automatic_radius.py`):

    refs/venv/bin/python validation/oracles/openrocket/motor_database.py \\
        corpus-out/openrocket-motors.json refs

The record is written to the path given, not to standard output, which OpenRocket logs to. Each
input after it may be a `.ork` file or a directory, walked in sorted order for `.ork` files and
skipping `refs/scratch/`, as `cargo xtask ork` does. The database is the jar's alone: the script stops if OpenRocket's user motor directories hold any file,
since OpenRocket would load those too.
"""

import collections
import gzip
import hashlib
import io
import json
import sys
import tempfile
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

import jpype

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start, with empty databases
import motors  # noqa: E402 - one curve file through OpenRocket's own loader

GENERATED = "2026-09-25"


def database():
    """The motor database OpenRocket loads at startup, and the user directories it would read."""
    from info.openrocket.core.database import MotorDatabaseLoader
    from info.openrocket.core.startup import Application

    user = [Path(str(f)) for f in Application.getPreferences().getUserThrustCurveFiles()]
    for place in user:
        if place.is_file() or (place.is_dir() and any(place.iterdir())):
            sys.exit(f"{place} holds motor files; the record must be the jar's database alone")
    loader = MotorDatabaseLoader()
    loader.startLoading()
    loader.cancelStartupDelay()
    loader.blockUntilLoaded()
    return loader.getDatabase()


def described(motor):
    """One database motor, as OpenRocket holds it: identity, envelope, samples and integrals."""
    return {
        "digest": str(motor.getDigest()),
        "designation": str(motor.getDesignation()),
        "common_name": str(motor.getCommonName()),
        "manufacturer": str(motor.getManufacturer().getSimpleName()),
        "motor_type": str(motor.getMotorType().getName()),
        "diameter_m": float(motor.getDiameter()),
        "length_m": float(motor.getLength()),
        "launch_mass_kg": float(motor.getLaunchMass()),
        "burnout_mass_kg": float(motor.getBurnoutMass()),
        "propellant_mass_kg": float(motor.getPropellantMass()),
        "delays_s": [motors.delay_of(value) for value in motor.getStandardDelays()],
        "time_s": [float(t) for t in motor.getTimePoints()],
        "thrust_n": [float(f) for f in motor.getThrustPoints()],
        "total_impulse_ns": float(motor.getTotalImpulseEstimate()),
        "average_thrust_n": float(motor.getAverageThrustEstimate()),
        "max_thrust_n": float(motor.getMaxThrustEstimate()),
        "burn_time_s": float(motor.getBurnTime()),
        "burn_time_estimate_s": float(motor.getBurnTimeEstimate()),
        "launch_cg_x_m": float(motor.getLaunchCGx()),
        "burnout_cg_x_m": float(motor.getBurnoutCGx()),
        "unit_longitudinal_inertia_m2": float(motor.getUnitLongitudinalInertia()),
        "unit_rotational_inertia_m2": float(motor.getUnitRotationalInertia()),
    }


def design_files(inputs):
    """Every `.ork` under the inputs, sorted."""
    found = []
    for raw in inputs:
        path = Path(raw)
        if path.is_dir():
            found.extend(
                sorted(
                    p
                    for p in path.rglob("*.ork")
                    if p.is_file() and "refs/scratch/" not in p.as_posix()
                )
            )
        elif path.suffix.lower() == ".ork":
            found.append(path)
        else:
            sys.exit(f"not a .ork file or a directory: {raw}")
    return found


def embedded(designs):
    """Every distinct embedded curve, by the SHA-256 of its bytes, through OpenRocket's loader."""
    curves = {}
    for design in designs:
        try:
            archive = zipfile.ZipFile(io.BytesIO(design.read_bytes()))
        except zipfile.BadZipFile:
            continue  # a bare or gzipped document embeds nothing
        for name in sorted(archive.namelist()):
            if not (name.startswith("thrustcurves/") and name.lower().endswith(".rse")):
                continue
            data = archive.read(name)
            sha = hashlib.sha256(data).hexdigest()
            if sha in curves:
                continue
            with tempfile.TemporaryDirectory() as scratch:
                path = Path(scratch) / Path(name).name
                path.write_bytes(data)
                try:
                    found = {"motors": motors.read(path)}
                except Exception as error:  # noqa: BLE001 - OpenRocket's refusal is the finding
                    found = {"driver_error": str(error)}
            curves[sha] = {"entry": name, **found}
    return [{"sha256": sha, **curves[sha]} for sha in sorted(curves)]


def bind(held):
    """Binds `held` as the motor database designs are loaded with, as the program does."""
    from com.google.inject import Guice
    from info.openrocket.core.database import ComponentPresetDao, ComponentPresetDatabase
    from info.openrocket.core.database.motor import MotorDatabase
    from info.openrocket.core.plugin import PluginModule
    from info.openrocket.core.startup import Application
    from info.openrocket.swing.utils import CoreServicesModule

    @jpype.JImplements("com.google.inject.Module")
    class Loaded:
        @jpype.JOverride
        def configure(self, binder):
            binder.bind(MotorDatabase).toInstance(held)
            binder.bind(ComponentPresetDao).toInstance(ComponentPresetDatabase())

    Application.setInjector(Guice.createInjector(CoreServicesModule(), PluginModule(), Loaded()))


def written_digests(design):
    """The `<digest>` of every `<motor>` the design document writes, or `None` if it does not parse."""
    data = design.read_bytes()
    try:
        archive = zipfile.ZipFile(io.BytesIO(data))
        names = [n for n in archive.namelist() if n.lower().endswith(".ork")]
        data = archive.read(names[0]) if names else b""
    except zipfile.BadZipFile:
        if data[:2] == b"\x1f\x8b":
            data = gzip.decompress(data)
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        return None
    return [
        (m.findtext("digest") or "").strip()
        for m in root.iter("motor")
        if (m.findtext("digest") or "").strip()
    ]


def flown(design):
    """Per design: how many motors name a digest, and how many OpenRocket flies with that curve."""
    written = written_digests(design)
    entry = {"file": design.as_posix(), "sha256": hashlib.sha256(design.read_bytes()).hexdigest()}
    if written is None:
        return {**entry, "driver_error": "the design document is not well-formed XML"}
    try:
        rocket = automatic_radius.load(design.resolve()).getRocket()
    except Exception as error:  # noqa: BLE001 - OpenRocket's refusal is the finding
        first = (str(error).splitlines() or [type(error).__name__])[0]
        return {**entry, "driver_error": first or type(error).__name__}
    assigned = collections.Counter()
    for fcid in rocket.getIds():
        for placed in rocket.getFlightConfiguration(fcid).getAllMotors():
            motor = placed.getMotor()
            if motor is not None:
                assigned[str(motor.getDigest())] += 1
    named = collections.Counter(written)
    return {
        **entry,
        "motors_naming_a_digest": sum(named.values()),
        "flown_with_that_digest": sum((named & assigned).values()),
        "digests_not_flown": sorted((named - assigned).elements()),
    }


def main():
    if len(sys.argv) < 2:
        sys.exit(f"usage: {Path(__file__).name} <output.json> [.ork file or directory]...")
    output = Path(sys.argv[1])
    designs = design_files(sys.argv[2:])
    here = Path(__file__).resolve().parent

    automatic_radius.start()
    from java.lang import System
    from info.openrocket.core.util import BuildProperties

    held = database()
    # Sorted on everything, so that two motors alike but for a last bit sort the same every run.
    listed = sorted(
        (described(m) for s in held.getMotorSets() for m in s.getMotors()),
        key=lambda m: (m["digest"], json.dumps(m, sort_keys=True)),
    )
    embedded_curves = embedded(designs)
    bind(held)
    readback = [flown(design) for design in designs]
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/motor_database.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    name: hashlib.sha256((here / name).read_bytes()).hexdigest()
                    for name in ["motor_database.py", "motors.py", "automatic_radius.py"]
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["motor_database.py", *sys.argv[1:]]),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "designs": len(designs),
                "database": listed,
                "embedded": embedded_curves,
                "flown": readback,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
