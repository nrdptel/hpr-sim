"""What OpenRocket 24.12 does with a motor whose ignition never comes (M2.2e10).

A `.ork` lights each motor at an event: `launch`, the `burnout` or `ejectioncharge` of the stage
below, or `never`. Some settings name an event that can never happen: a motor in the bottom stage
has no stage below to burn out or fire a charge, and a plugged motor below fires no charge. hpr
refused such a configuration until M2.2e10. This script measures what OpenRocket does instead, on
the jar's *Two stage high power rocket* example, second configuration (an I59WN-P in the
sustainer, an I357T-14 in the booster). It flies the configuration as written, then with

- the booster's motor set `never`, the sustainer's at `launch`, and the booster's separation
  `never` (the example separates it at the sustainer's ignition, here at launch);
- the booster's motor at `burnout` of the stage below, which it has not, the sustainer's at
  `launch`, and the booster's separation at its own motor's burnout;
- the same with `ejectioncharge` and a separation at its own motor's charge;
- the booster's motor at `launch` and plugged, the sustainer's at `ejectioncharge`, and the
  booster's separation `never`;
- the same with the sustainer's at `automatic`, which in a stage above means the charge;
- a control for the charge: the booster's motor not plugged, with its 14 s delay, and the
  sustainer's at `ejectioncharge`, which should light it 14 s after the booster's burnout.

Each is flown in a new simulation in calm air, with OpenRocket's default launch conditions and
nothing deployed, so the whole climb is on the stack. The record holds, for each flight, the
branch count, every event's kind and time, the mass at the first row and at the row of the largest
altitude, that altitude, and each motor's launch and burnout mass as OpenRocket's motor gives them.
A motor that never lights leaves no ignition event and keeps its propellant, so the mass the stack
loses from launch to apogee is the lit motor's propellant alone.

The two motors differ on purpose. The first configuration has an H148R-0 in each stage, and there
OpenRocket's mass column follows neither motor: `h148r-pair-booster-never` flies it with the
booster `never` and the sustainer at `launch`, as the second probe does the second, and records
what the column says (issue #185).

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. Only numbers and the names
of OpenRocket's events are recorded. OpenRocket 24.12 needs Java 17 exactly; see
`automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/unlit_motors.py \\
        validation/fixtures/ork/openrocket-unlit-motors.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import logging
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the jar's path
import events  # noqa: E402 - the JVM start, the example loader and the component walk

GENERATED = "2026-09-28"
SEED = 1

#: Each probe: the booster's ignition event, the sustainer's, the booster's separation event, and
#: whether the booster's motor is plugged. `None` keeps the example's own setting.
PROBES = {
    "as-written": (None, None, None, False),
    "booster-never": ("NEVER", "LAUNCH", "NEVER", False),
    "booster-burnout": ("BURNOUT", "LAUNCH", "BURNOUT", False),
    "booster-ejection-charge": ("EJECTION_CHARGE", "LAUNCH", "EJECTION", False),
    "booster-plugged": ("LAUNCH", "EJECTION_CHARGE", "NEVER", True),
    "booster-plugged-automatic": ("LAUNCH", "AUTOMATIC", "NEVER", True),
    "sustainer-at-charge": ("LAUNCH", "EJECTION_CHARGE", "NEVER", False),
}
#: The configuration each probe flies, by its place in the example: the second, but for the probe
#: of the first's like motors.
CONFIGURATION = {probe: 1 for probe in PROBES}
PROBES["h148r-pair-booster-never"] = PROBES["booster-never"]
CONFIGURATION["h148r-pair-booster-never"] = 0


def flight(probe):
    """The example's configuration `CONFIGURATION` names, changed as `probe` says, flown with
    nothing deployed."""
    from info.openrocket.core.document import Simulation
    from info.openrocket.core.motor import IgnitionEvent, Motor
    from info.openrocket.core.rocketcomponent import (
        AxialStage,
        DeploymentConfiguration,
        MotorMount,
        RecoveryDevice,
    )
    from info.openrocket.core.rocketcomponent.StageSeparationConfiguration import (
        SeparationEvent,
    )
    from info.openrocket.core.simulation import FlightDataType

    booster_lights, sustainer_lights, separates, plugged = PROBES[probe]
    document = events.example(events.STAGED)
    rocket = document.getRocket()
    configuration = rocket.getFlightConfigurationByIndex(
        CONFIGURATION[probe]
    ).getFlightConfigurationID()
    stages = events.components(rocket, AxialStage)
    if len(stages) != 2:
        sys.exit(f"the example has {len(stages)} stages, not 2")
    mounts = [m for m in events.components(rocket, MotorMount) if m.isMotorMount()]
    by_stage = {}
    for mount in mounts:
        held = mount.getMotorConfig(configuration)
        if held.getMotor() is not None:
            by_stage.setdefault(int(mount.getStage().getStageNumber()), []).append(held)
    if sorted(by_stage) != [0, 1] or any(len(held) != 1 for held in by_stage.values()):
        sys.exit(f"the example's configuration {probe} flies is not one motor in each of two stages")
    sustainer, booster = by_stage[0][0], by_stage[1][0]
    if booster_lights is not None:
        booster.setIgnitionEvent(getattr(IgnitionEvent, booster_lights))
        booster.setIgnitionDelay(0.0)
    if sustainer_lights is not None:
        sustainer.setIgnitionEvent(getattr(IgnitionEvent, sustainer_lights))
        sustainer.setIgnitionDelay(0.0)
    if plugged:
        booster.setEjectionDelay(Motor.PLUGGED_DELAY)
    separation = stages[1].getSeparationConfigurations().get(configuration)
    if separates is not None:
        separation.setSeparationEvent(getattr(SeparationEvent, separates))
        separation.setSeparationDelay(0.0)
    for device in events.components(rocket, RecoveryDevice):
        held = device.getDeploymentConfigurations().get(configuration)
        held.setDeployEvent(DeploymentConfiguration.DeployEvent.NEVER)

    simulation = Simulation(document, rocket)
    simulation.setFlightConfigurationId(configuration)
    options = simulation.getOptions()
    options.setWindSpeedAverage(0.0)
    options.setWindTurbulenceIntensity(0.0)
    options.setRandomSeed(SEED)
    simulation.simulate()
    data = simulation.getSimulatedData()
    branch = data.getBranch(0)
    altitude = [float(v) for v in branch.get(FlightDataType.TYPE_ALTITUDE)]
    mass = [float(v) for v in branch.get(FlightDataType.TYPE_MASS)]
    highest = altitude.index(max(altitude))

    def motor(held):
        return {
            "designation": str(held.getMotor().getDesignation()),
            "ignition_event": events.name_of(held.getIgnitionEvent()),
            "ejection_delay_s": (
                None if held.getEjectionDelay() == Motor.PLUGGED_DELAY
                else float(held.getEjectionDelay())
            ),
            "launch_mass_kg": float(held.getMotor().getLaunchMass()),
            "burnout_mass_kg": float(held.getMotor().getBurnoutMass()),
        }

    return {
        "configuration_index": CONFIGURATION[probe],
        "booster_separation_event": events.name_of(separation.getSeparationEvent()),
        "sustainer": motor(sustainer),
        "booster": motor(booster),
        "branches": int(data.getBranchCount()),
        "events": [
            {"type": events.name_of(e.getType()), "time_s": float(e.getTime())}
            for e in branch.getEvents()
        ],
        "launch_mass_kg": mass[0],
        "apogee_mass_kg": mass[highest],
        "max_altitude_m": altitude[highest],
    }


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: unlit_motors.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    events.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    probes = {probe: flight(probe) for probe in PROBES}
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/unlit_motors.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    name: hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
                    for name, module in [
                        ("unlit_motors.py", sys.modules[__name__]),
                        ("events.py", events),
                    ]
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "example": events.STAGED,
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "seed": SEED,
                "probes": probes,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
