"""OpenRocket 24.12's drag coefficient along its own flights, as two curves in Mach number.

`cargo xtask ork-flights` needs a written, sized cause for every apogee more than 5% from
OpenRocket's (ADR-073). A cause in the drag is sized by hpr flying the same configuration on
OpenRocket's drag instead of its own: if hpr's apogee then comes within 5% of OpenRocket's, the
difference was the drag (ADR-097). This script records that drag. It runs OpenRocket 24.12 as an
external oracle and, for every motor configuration of every design given, flies it as `flights.py`
flies it with nothing deployed (the same conditions, calm air and seed), and takes from the first
branch, launch to the row of the largest altitude:

- `power_on`: the rows with thrust, each at a Mach number above every earlier one's, so the curve
  climbs as the rocket speeds up under power and a row at a Mach number already passed (the end
  of a burn, slowing) is left out;
- `power_off`: the rows after the last row with thrust, each at a Mach number below every earlier
  one's, written in rising Mach number.

Each curve is `[mach, drag coefficient]` pairs, the drag coefficient OpenRocket's
`TYPE_DRAG_COEFF` on its reference area, which the record gives as a reference length. A flight
OpenRocket refused, or a design it could not open, is recorded with the reason.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. OpenRocket 24.12 needs Java
17 exactly; see `automatic_radius.py`. Run from the repository root, for the private library:

    refs/venv/bin/python validation/oracles/openrocket/drag_curves.py \\
        corpus-out/openrocket-drag-curves.json refs

The record is written to the path given, not to standard output, which OpenRocket logs to. A
private design's curves are its values: they stay under the gitignored `corpus-out/`.
"""

import hashlib
import json
import logging
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "rocketserializer"))

import automatic_radius  # noqa: E402 - the jar's path
import events  # noqa: E402 - the JVM start with the motor database loaded
import flights  # noqa: E402 - the same flight setup, conditions and seed
import geometry  # noqa: E402 - the same file discovery and comment retry

GENERATED = "2026-09-28"


def curves(mach, drag, thrust, altitude):
    """The power-on and power-off curves of one branch, launch to the largest altitude."""
    highest = altitude.index(flights.peak(altitude))
    burning = [i for i in range(highest + 1) if thrust[i] > 0.0]
    last_burning = burning[-1] if burning else -1
    power_on, power_off = [], []
    for i in burning:
        if mach[i] > 0.0 and drag[i] == drag[i] and (not power_on or mach[i] > power_on[-1][0]):
            power_on.append([mach[i], drag[i]])
    for i in range(last_burning + 1, highest + 1):
        if mach[i] > 0.0 and drag[i] == drag[i] and (not power_off or mach[i] < power_off[-1][0]):
            power_off.append([mach[i], drag[i]])
    power_off.reverse()
    return {"rows": highest + 1, "power_on": power_on, "power_off": power_off}


def flight(text, scratch, configuration):
    """`configuration` of the design `text` flown with nothing deployed, as its two curves."""
    from info.openrocket.core.document import Simulation
    from info.openrocket.core.simulation import FlightDataType as F

    document, _ = geometry.opened(text, scratch)
    flights.never_deploy(document, configuration)
    stored = list(document.getSimulations())
    simulation = Simulation(document, document.getRocket())
    if stored:
        simulation.copySimulationOptionsFrom(stored[0].getOptions())
    simulation.setFlightConfigurationId(configuration)
    options = simulation.getOptions()
    options.setWindSpeedAverage(0.0)
    options.setWindTurbulenceIntensity(0.0)
    options.setLaunchIntoWind(False)
    options.setRandomSeed(flights.SEED)
    try:
        simulation.simulate()
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        return {"refused": geometry.first_line(error)}
    data = simulation.getSimulatedData()
    branch = data.getBranch(0)
    column = lambda kind: flights.column(branch, kind)  # noqa: E731
    found = [events.name_of(e.getType()) for e in branch.getEvents()]
    reference = column(F.TYPE_REFERENCE_LENGTH)
    return {
        "branches": int(data.getBranchCount()),
        "aborted": "SIM_ABORT" in found,
        "max_altitude_m": flights.finite(data.getMaxAltitude()),
        "reference_length_m": flights.finite(reference[0]) if reference else None,
        **curves(
            column(F.TYPE_MACH_NUMBER),
            column(F.TYPE_DRAG_COEFF),
            column(F.TYPE_THRUST_FORCE),
            column(F.TYPE_ALTITUDE),
        ),
    }


def design(path, scratch):
    """Every motor configuration of the design at `path`, as its curves."""
    record = {"sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest()}
    try:
        text = geometry.document_text(path)
        document, _ = geometry.opened(text, scratch)
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        record["refused"] = geometry.first_line(error)
        return record
    rocket = document.getRocket()
    record["flights"] = [
        {"configuration": str(configuration.toString()), **flight(text, scratch, configuration)}
        for configuration in rocket.getIds()
        if rocket.getFlightConfiguration(configuration).hasMotors()
    ]
    return record


def main():
    paths = sys.argv[1:]
    if len(paths) < 2:
        sys.exit("usage: drag_curves.py OUTPUT.json (FILE | DIR)...")
    output, inputs = Path(paths[0]), paths[1:]
    root = Path.cwd().resolve()
    logging.disable(logging.CRITICAL)
    events.start()
    from info.openrocket.core.util import BuildProperties

    runs = []
    with tempfile.TemporaryDirectory() as scratch:
        for name, path in geometry.designs(inputs, root):
            try:
                runs.append({"file": name, **design(path, scratch)})
            except Exception as error:  # noqa: BLE001 - recorded, and the report stops on it
                runs.append({"file": name, "driver_error": geometry.first_line(error)})
    output.parent.mkdir(parents=True, exist_ok=True)
    scripts = [Path(__file__), Path(flights.__file__), Path(events.__file__),
               Path(geometry.__file__)]
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/drag_curves.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in scripts
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["drag_curves.py", *sys.argv[1:]]),
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
