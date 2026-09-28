"""Whether OpenRocket 24.12 takes a burning motor's cross-section off the base drag.

Niskanen's thesis, which OpenRocket's documentation follows, says a burning motor fills part of
the base, "if the base is the same size as the motor itself, no base drag" (2009, pp. 50-51), and
hpr takes the motor's cross-section off the base while it burns. `C06/1`, a private flight, pointed
to OpenRocket not doing so (ADR-097). This script measures it on public designs: it runs
OpenRocket 24.12 as an external oracle on every motor configuration of every example design inside
the jar, flown as `flights.py` flies them with nothing deployed, and records, launch to apogee:

- the base-drag column (`TYPE_BASE_DRAG_COEFF`) over the whole base's coefficient at the row's
  Mach number, `0.12 + 0.13 M^2` below Mach 1 and `0.25/M` above (Niskanen eq. 3.94): its
  smallest and largest on the rows with thrust, and on the rows without;
- the configuration's motors' cross-section over the reference area, `motor_area_fraction`: about
  what the ratio would drop by while they all burn if OpenRocket took their area off the base, and
  `pod_motors`, how many of those motors sit in a pod;
- the largest difference, on the rows with thrust, between the drag coefficient
  (`TYPE_DRAG_COEFF`) and the sum of its friction, pressure and base columns, `sum_residual`:
  near zero if the base-drag column is what OpenRocket flies.

If the ratio is the same with thrust as without, OpenRocket keeps the whole base while a motor
burns. Only rows faster than Mach 0.01 count, where the column is not the rounding of a number
near zero.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. OpenRocket 24.12 needs Java
17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/base_drag.py \\
        validation/fixtures/ork/openrocket-base-drag.json

The record is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import logging
import math
import sys
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "rocketserializer"))

import automatic_radius  # noqa: E402 - the jar's path
import events  # noqa: E402 - the JVM start with the motor database loaded
import flights  # noqa: E402 - the same flight setup, conditions and seed

GENERATED = "2026-09-28"
SLOWEST_MACH = 0.01


def whole_base(mach):
    """Niskanen's eq. 3.94: the base drag coefficient on the base's own area."""
    return 0.12 + 0.13 * mach * mach if mach < 1.0 else 0.25 / mach


def spread(values):
    return {"rows": len(values), "min": min(values), "max": max(values)} if values else {"rows": 0}


def in_a_pod(component):
    """Whether `component` sits inside a pod set."""
    while component is not None:
        if str(component.getClass().getSimpleName()) == "PodSet":
            return True
        component = component.getParent()
    return False


def flown(document, configuration):
    """`configuration` flown with nothing deployed, as its base-drag ratios."""
    from info.openrocket.core.document import Simulation
    from info.openrocket.core.simulation import FlightDataType as F

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
        return {"refused": str(error).splitlines()[0]}
    branch = simulation.getSimulatedData().getBranch(0)
    column = lambda kind: flights.column(branch, kind)  # noqa: E731
    altitude = column(F.TYPE_ALTITUDE)
    highest = altitude.index(flights.peak(altitude))
    mach = column(F.TYPE_MACH_NUMBER)
    base = column(F.TYPE_BASE_DRAG_COEFF)
    thrust = column(F.TYPE_THRUST_FORCE)
    reference = column(F.TYPE_REFERENCE_AREA)
    total = column(F.TYPE_DRAG_COEFF)
    friction = column(F.TYPE_FRICTION_DRAG_COEFF)
    pressure = column(F.TYPE_PRESSURE_DRAG_COEFF)
    burning, coasting, residual = [], [], []
    for i in range(highest + 1):
        if not (math.isfinite(mach[i]) and math.isfinite(base[i])) or mach[i] <= SLOWEST_MACH:
            continue
        (burning if thrust[i] > 0.0 else coasting).append(base[i] / whole_base(mach[i]))
        if thrust[i] > 0.0:
            residual.append(abs(total[i] - friction[i] - pressure[i] - base[i]))
    held = document.getRocket().getFlightConfiguration(configuration)
    motors = list(held.getActiveMotors())
    motor_area = sum(math.pi * float(m.getMotor().getDiameter()) ** 2 / 4.0 for m in motors)
    return {
        "branches": int(simulation.getSimulatedData().getBranchCount()),
        "motor_area_fraction": motor_area / reference[0] if reference and reference[0] else None,
        "pod_motors": sum(1 for m in motors if in_a_pod(m.getMount())),
        "burning": spread(burning),
        "coasting": spread(coasting),
        "sum_residual": max(residual) if residual else None,
    }


def main():
    paths = sys.argv[1:]
    if len(paths) != 1:
        sys.exit("usage: base_drag.py OUTPUT.json")
    output = Path(paths[0])
    logging.disable(logging.CRITICAL)
    events.start()
    from info.openrocket.core.util import BuildProperties

    runs = []
    with zipfile.ZipFile(automatic_radius.JAR) as archive:
        entries = sorted(
            e for e in archive.namelist()
            if e.startswith("datafiles/examples/") and e.lower().endswith(".ork")
        )
    for entry in entries:
        rocket = events.example(entry).getRocket()
        for configuration in rocket.getIds():
            if not rocket.getFlightConfiguration(configuration).hasMotors():
                continue
            # A new copy for each flight, so one flight's changes don't carry to the next.
            document = events.example(entry)
            runs.append({
                "file": f"{automatic_radius.JAR}!{entry}",
                "configuration": str(configuration.toString()),
                **flown(document, configuration),
            })
    output.parent.mkdir(parents=True, exist_ok=True)
    scripts = [Path(__file__), Path(flights.__file__), Path(events.__file__)]
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/base_drag.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in scripts
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["base_drag.py", *sys.argv[1:]]),
                "slowest_mach": SLOWEST_MACH,
                "flights": runs,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
