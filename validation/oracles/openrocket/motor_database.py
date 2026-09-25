"""OpenRocket 24.12's own motor database, and what it makes of the curves designs embed.

A `<motor>` in a `.ork` names a motor and records OpenRocket's digest of its thrust curve. Most
designs do not carry the curve itself: OpenRocket finds it in the motor database shipped inside its
jar. hpr bundles only 32 curves, so without that database most of a design library cannot fly
(M2.2c2). This script asks OpenRocket for every motor in the database it loads, with its digest,
its sampled curve and its own integrals, so that `cargo xtask ork` can supply the curve a design's
digest names and hold hpr's integration of each curve to OpenRocket's. It also hands every curve a
design embeds (`thrustcurves/<digest>.rse`) to OpenRocket's own loader, as `motors.py` does for the
bundled curves (M2.2c1). Last, it opens every design with that database bound, as the program
does, and records the digests of the motors OpenRocket puts in each configuration, so that the
survey can check that OpenRocket flies the very curves hpr is given for it.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names are the public API `javap` prints for the jar. The database's contents come from
ThrustCurve.org by way of OpenRocket; their terms are unstated, so the record is written only under
the gitignored `corpus-out/`, and so are the embedded curves' numbers, which belong to the designs'
owners.

Run from the repository root with the oracle environment and Java 17 (see `automatic_radius.py`):

    refs/venv/bin/python validation/oracles/openrocket/motor_database.py \\
        corpus-out/openrocket-motors.json refs --jar

The record is written to the path given, not to standard output, which OpenRocket logs to. Each
input after it may be a `.ork` file or a directory, found as `mass.py` finds them (the survey's
own set, without `refs/scratch/`); `--jar` adds the example designs inside the jar, as the survey
does. A design OpenRocket refuses for a leading comment is opened without it, as `mass.py` opens
it. The database is the jar's alone: the script stops if OpenRocket's user motor directories hold any file,
since OpenRocket would load those too.
"""

import hashlib
import io
import json
import logging
import sys
import tempfile
import zipfile
from pathlib import Path

import jpype

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "rocketserializer"))

import automatic_radius  # noqa: E402 - the JVM start, with empty databases
import geometry  # noqa: E402 - the file discovery and comment retry mass.py uses
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


def embedded(designs):
    """Every distinct embedded curve, by the SHA-256 of its bytes, through OpenRocket's loader."""
    curves = {}
    for _, design in designs:
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


def flown(name, path, scratch):
    """The digests of the motors OpenRocket places in each configuration of one design."""
    entry = {"file": name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    try:
        text = geometry.document_text(path)
    except Exception as error:  # noqa: BLE001 - recorded; the survey reports it
        return {**entry, "driver_error": geometry.first_line(error)}
    try:
        document, stripped = geometry.opened(text, scratch)
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        return {**entry, "opens": False, "refused": geometry.first_line(error)}
    rocket = document.getRocket()
    configurations = {}
    for fcid in rocket.getIds():
        placed = []
        for motor_configuration in rocket.getFlightConfiguration(fcid).getAllMotors():
            motor = motor_configuration.getMotor()
            if motor is not None:
                placed.append(str(motor.getDigest()))
        configurations[str(fcid.key)] = sorted(placed)
    return {**entry, "opens": True, "comment_removed": stripped, "configurations": configurations}


def main():
    args = sys.argv[1:]
    jar = "--jar" in args
    paths = [arg for arg in args if not arg.startswith("--")]
    if not paths:
        sys.exit(f"usage: {Path(__file__).name} <output.json> [.ork file or directory]... [--jar]")
    output = Path(paths[0])
    here = Path(__file__).resolve().parent
    logging.disable(logging.CRITICAL)

    automatic_radius.start()
    from java.lang import System
    from info.openrocket.core.util import BuildProperties

    held = database()
    # Sorted on everything, so that two motors alike but for a last bit sort the same every run.
    listed = sorted(
        (described(m) for s in held.getMotorSets() for m in s.getMotors()),
        key=lambda m: (m["digest"], json.dumps(m, sort_keys=True)),
    )
    bind(held)
    with tempfile.TemporaryDirectory() as scratch:
        designs = geometry.designs(paths[1:], Path.cwd().resolve())
        if jar:
            with zipfile.ZipFile(automatic_radius.JAR) as archive:
                for entry in sorted(archive.namelist()):
                    if entry.startswith("datafiles/examples/") and entry.lower().endswith(".ork"):
                        copy = Path(scratch) / f"example-{len(designs)}.ork"
                        copy.write_bytes(archive.read(entry))
                        designs.append((f"{automatic_radius.JAR}!{entry}", copy))
        embedded_curves = embedded(designs)
        readback = []
        for name, path in designs:
            try:
                readback.append(flown(name, path, scratch))
            except Exception as error:  # noqa: BLE001 - recorded; the survey reports it
                readback.append({"file": name, "driver_error": geometry.first_line(error)})
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/motor_database.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    name: hashlib.sha256((here / name).read_bytes()).hexdigest()
                    for name in ["motor_database.py", "motors.py", "automatic_radius.py"]
                } | {
                    "geometry.py": hashlib.sha256(Path(geometry.__file__).read_bytes()).hexdigest()
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
