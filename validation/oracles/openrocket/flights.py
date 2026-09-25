"""OpenRocket 24.12's flights of the public designs, and what each of its summary words measures.

A `.ork` stores OpenRocket's summary of a flight in ten words: `maxaltitude`, `maxvelocity`,
`launchrodvelocity`, `deploymentvelocity`, `groundhitvelocity`, `optimumdelay` and others. The
file format page names them but not what each measures, and a word can mean a different quantity
in another tool or another OpenRocket version (Loft lesson L80). This script runs OpenRocket 24.12
as an external oracle and records, for every motor configuration of every public design, its
summary beside the quantities of its own time series that the summary could mean:

- the largest altitude and the largest total and vertical velocity over the first branch;
- each event's time and the two rows either side of it, with their time and total velocity, so a
  velocity can be taken at the event itself and each recovery device's deployment told apart;
- the total velocity and time of the last row, and the mass at the first.

It also records the stability margin at launch rod clearance with the centres of pressure and mass
and the reference length it is taken from, so hpr's margin can be compared on the same instant.

The public designs are the example designs inside the OpenRocket jar and the seven Loft demos in
`validation/fixtures/ork/loft-demo/`. Each configuration is flown in a new simulation that takes
the launch conditions of the design's first stored simulation (OpenRocket's defaults when there is
none), in calm air: no wind and no turbulence, so the flight is repeatable and hpr can fly the same
one. The conditions used are recorded with each flight. Simulation extensions are not carried over.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. Only numbers and the
designs' public file names are recorded. OpenRocket 24.12 needs Java 17 exactly; see
`automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/flights.py \\
        validation/fixtures/ork/openrocket-flights.json validation/fixtures/ork/loft-demo --jar

The record is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import logging
import math
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "rocketserializer"))

import automatic_radius  # noqa: E402 - the jar's path
import events  # noqa: E402 - the JVM start with the motor database loaded
import geometry  # noqa: E402 - the same file discovery and comment retry as the cross-check

GENERATED = "2026-09-25"
SEED = 1


def finite(value):
    """`value` as a float, or `None` for OpenRocket's `NaN` (a quantity it did not compute)."""
    value = float(value)
    return value if math.isfinite(value) else None


def column(branch, kind):
    return [float(x) for x in branch.get(kind)]


def row_at(times, time):
    """The last row at or before `time`: an event between two rows is recorded at the earlier."""
    index = 0
    for i, t in enumerate(times):
        if t <= time + 1e-9:
            index = i
    return index


def conditions(options):
    """The launch conditions a flight used, SI with angles in radians."""
    return {
        "rod_length_m": float(options.getLaunchRodLength()),
        "rod_angle_rad": float(options.getLaunchRodAngle()),
        "rod_direction_rad": float(options.getLaunchRodDirection()),
        "launch_altitude_m": float(options.getLaunchAltitude()),
        "launch_latitude_deg": float(options.getLaunchLatitude()),
        "launch_longitude_deg": float(options.getLaunchLongitude()),
        "isa_atmosphere": bool(options.isISAAtmosphere()),
        "launch_temperature_k": float(options.getLaunchTemperature()),
        "launch_pressure_pa": float(options.getLaunchPressure()),
        "time_step_s": float(options.getTimeStep()),
        "wind_average_m_s": float(options.getWindSpeedAverage()),
        "wind_turbulence": float(options.getWindTurbulenceIntensity()),
        "geodetic": str(options.getGeodeticComputation().name()),
    }


def flight(document, configuration, base):
    """One configuration flown in calm air, as OpenRocket summarises it and as its data says."""
    from info.openrocket.core.document import Simulation
    from info.openrocket.core.simulation import FlightDataType

    simulation = Simulation(document, document.getRocket())
    if base is not None:
        simulation.copySimulationOptionsFrom(base.getOptions())
    simulation.setFlightConfigurationId(configuration)
    options = simulation.getOptions()
    options.setWindSpeedAverage(0.0)
    options.setWindTurbulenceIntensity(0.0)
    options.setLaunchIntoWind(False)
    options.setRandomSeed(SEED)
    record = {"conditions": conditions(options)}
    try:
        simulation.simulate()
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        record["refused"] = geometry.first_line(error)
        return record
    data = simulation.getSimulatedData()
    record["summary"] = {
        "max_altitude_m": finite(data.getMaxAltitude()),
        "max_velocity_m_s": finite(data.getMaxVelocity()),
        "max_acceleration_m_s2": finite(data.getMaxAcceleration()),
        "max_mach": finite(data.getMaxMachNumber()),
        "time_to_apogee_s": finite(data.getTimeToApogee()),
        "flight_time_s": finite(data.getFlightTime()),
        "ground_hit_velocity_m_s": finite(data.getGroundHitVelocity()),
        "launch_rod_velocity_m_s": finite(data.getLaunchRodVelocity()),
        "deployment_velocity_m_s": finite(data.getDeploymentVelocity()),
        "optimum_delay_s": finite(data.getOptimumDelay()),
    }
    record["branches"] = int(data.getBranchCount())
    branch = data.getBranch(0)
    times = column(branch, FlightDataType.TYPE_TIME)
    altitude = column(branch, FlightDataType.TYPE_ALTITUDE)
    speed = column(branch, FlightDataType.TYPE_VELOCITY_TOTAL)
    vertical = column(branch, FlightDataType.TYPE_VELOCITY_Z)
    stability = column(branch, FlightDataType.TYPE_STABILITY)
    cg = column(branch, FlightDataType.TYPE_CG_LOCATION)
    cp = column(branch, FlightDataType.TYPE_CP_LOCATION)
    reference = column(branch, FlightDataType.TYPE_REFERENCE_LENGTH)
    mach = column(branch, FlightDataType.TYPE_MACH_NUMBER)
    mass = column(branch, FlightDataType.TYPE_MASS)
    record["series"] = {
        "rows": len(times),
        "max_altitude_m": finite(max(altitude)),
        "max_total_velocity_m_s": finite(max(speed)),
        "max_vertical_velocity_m_s": finite(max(vertical)),
        "last_total_velocity_m_s": finite(speed[-1]),
        "last_time_s": finite(times[-1]),
        "launch_mass_kg": finite(mass[0]),
    }
    def at(row):
        return {
            "time_s": times[row],
            "total_velocity_m_s": finite(speed[row]),
        }

    found = []
    for event in branch.getEvents():
        time = float(event.getTime())
        row = row_at(times, time)
        found.append(
            {
                "type": events.name_of(event.getType()),
                "time_s": time,
                "before": at(row),
                "after": at(min(row + 1, len(times) - 1)),
            }
        )
    record["events"] = found
    clearance = [e for e in found if e["type"] == "LAUNCHROD"]
    if clearance:
        row = row_at(times, clearance[0]["time_s"])
        record["rod_clearance"] = {
            "time_s": times[row],
            "stability_cal": finite(stability[row]),
            "cg_from_nose_m": finite(cg[row]),
            "cp_from_nose_m": finite(cp[row]),
            "reference_length_m": finite(reference[row]),
            "mach": finite(mach[row]),
            "mass_kg": finite(mass[row]),
        }
    return record


def design(path, scratch):
    """Every motor configuration of the design at `path`, flown."""
    record = {"sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest()}
    try:
        text = geometry.document_text(path)
        document, _ = geometry.opened(text, scratch)
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        record["refused"] = geometry.first_line(error)
        return record
    rocket = document.getRocket()
    stored = list(document.getSimulations())
    base = stored[0] if stored else None
    record["conditions_from"] = "first stored simulation" if base is not None else "defaults"
    flights = []
    for configuration in rocket.getIds():
        held = rocket.getFlightConfiguration(configuration)
        entry = {
            "configuration": str(configuration.toString()),
            "name": str(held.getName()),
            "active_stages": int(held.getActiveStageCount()),
            "has_motors": bool(held.hasMotors()),
        }
        if held.hasMotors():
            entry.update(flight(document, configuration, base))
        flights.append(entry)
    record["flights"] = flights
    return record


def main():
    args = sys.argv[1:]
    jar = "--jar" in args
    paths = [arg for arg in args if not arg.startswith("--")]
    if len(paths) < 2 and not (jar and paths):
        sys.exit("usage: flights.py OUTPUT.json (FILE | DIR)... [--jar]")
    output, inputs = Path(paths[0]), paths[1:]
    root = Path.cwd().resolve()
    logging.disable(logging.CRITICAL)
    events.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    runs = []
    with tempfile.TemporaryDirectory() as scratch:
        files = geometry.designs(inputs, root)
        if jar:
            with zipfile.ZipFile(automatic_radius.JAR) as archive:
                for entry in sorted(archive.namelist()):
                    if entry.startswith("datafiles/examples/") and entry.lower().endswith(".ork"):
                        copy = Path(scratch) / f"example-{len(files)}.ork"
                        copy.write_bytes(archive.read(entry))
                        files.append((f"{automatic_radius.JAR}!{entry}", copy))
        for name, path in files:
            try:
                runs.append({"file": name, **design(path, scratch)})
            except Exception as error:  # noqa: BLE001 - recorded, and the test fails on it
                runs.append({"file": name, "driver_error": geometry.first_line(error)})

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/flights.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    "flights.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "events.py": hashlib.sha256(Path(events.__file__).read_bytes()).hexdigest(),
                    "geometry.py": hashlib.sha256(Path(geometry.__file__).read_bytes()).hexdigest(),
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["flights.py", *sys.argv[1:]]),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "seed": SEED,
                "designs": runs,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
