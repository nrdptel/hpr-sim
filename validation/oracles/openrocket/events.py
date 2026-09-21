"""The words OpenRocket writes for its events, and what it makes of a recovery device's settings.

A `.ork` names when a motor ignites, when a recovery device deploys and when a stage separates with
a word (`automatic`, `ejection`, `upperignition`, ...). OpenRocket's file-format page shows a few of
them and lists none, so this script measures every one: it runs OpenRocket 24.12 as an external
oracle, sets each value of each event through the program's public setters on one of the example
designs inside its jar, saves the design, and records the word written. It also records three
things M3.1c2 reads a recovery device by:

- the drag coefficient a parachute whose `<cd>` says `auto` is flown with, and OpenRocket's
  `Parachute.DEFAULT_CD`;
- the drag coefficient an automatic streamer is given, for three strip sizes;
- whether `<deployaltitude>` is above the ground or above sea level, and what happens when the
  rocket never climbs that high: the example's parachute is set to deploy at 30 m, then at 100 m,
  on a pad 1,000 m above sea level in calm air, the flight is simulated, and the height at the
  deployment event, if there is one, is read both ways.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. Only numbers and words are
recorded, never a design's contents.

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/events.py \\
        validation/fixtures/ork/openrocket-events.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import bisect
import hashlib
import io
import json
import sys
import tempfile
import time
import zipfile
from pathlib import Path
from xml.etree import ElementTree

import jpype
import jpype.imports

from automatic_radius import JAR, java_home

SINGLE = "datafiles/examples/A simple model rocket.ork"
STAGED = "datafiles/examples/Two stage high power rocket.ork"
STREAMERS_M = [(0.5, 0.05), (1.0, 0.1), (1.5, 0.05)]
PAD_ASL_M = 1000.0
SEED = 1
# Below the example's apogee (about 51 m), and above it.
DEPLOY_ALTITUDES_M = [30.0, 100.0]


def start():
    """OpenRocket with its motor database loaded, which a simulation needs."""
    home = java_home()
    for lib in ["lib/server/libjvm.dylib", "lib/server/libjvm.so", "bin/server/jvm.dll"]:
        if (home / lib).exists():
            jvm = str(home / lib)
            break
    else:
        sys.exit(f"no JVM library under {home}")
    # The labels are read in English whatever the machine's language, as the docs quote them.
    jpype.startJVM(
        jvm,
        "-Djava.awt.headless=true",
        "-Duser.language=en",
        "-Duser.country=US",
        classpath=[str(JAR)],
    )

    from com.google.inject import Guice
    from info.openrocket.core.plugin import PluginModule
    from info.openrocket.core.startup import Application
    from info.openrocket.swing.startup import GuiModule

    module = GuiModule()
    Application.setInjector(Guice.createInjector(module, PluginModule()))
    module.startLoader()
    # The loader fills the motor database on a thread of its own. It is ready once the example a
    # simulation is run on loads with its motor, which a missing database leaves out.
    from info.openrocket.core.rocketcomponent import MotorMount

    for _ in range(120):
        time.sleep(0.5)
        try:
            rocket = example(SINGLE).getRocket()
        except jpype.JException:
            # Before the database is in, the loader asks the graphical layer for it, which a
            # headless run refuses. Wait and try again.
            continue
        configuration = rocket.getSelectedConfiguration().getFlightConfigurationID()
        mounts = [m for m in components(rocket, MotorMount) if m.isMotorMount()]
        if mounts and mounts[0].getMotorConfig(configuration).getMotor() is not None:
            return
    sys.exit("OpenRocket's motor database did not load")


def example(entry):
    """An example design from the jar, loaded."""
    from info.openrocket.core.file import GeneralRocketLoader
    from java.io import File

    with zipfile.ZipFile(JAR) as jar:
        raw = jar.read(entry)
    with tempfile.TemporaryDirectory() as scratch:
        path = Path(scratch) / "example.ork"
        path.write_bytes(raw)
        return GeneralRocketLoader(File(str(path))).load()


def saved(document_):
    """The design document OpenRocket writes for `document_`, as text."""
    from info.openrocket.core.document import StorageOptions
    from info.openrocket.core.file.openrocket import OpenRocketSaver
    from info.openrocket.core.logging import ErrorSet, WarningSet
    from java.io import ByteArrayOutputStream

    out = ByteArrayOutputStream()
    OpenRocketSaver().save(out, document_, StorageOptions(), WarningSet(), ErrorSet())
    raw = bytes(out.toByteArray())
    if raw.startswith(b"PK"):
        raw = zipfile.ZipFile(io.BytesIO(raw)).read("rocket.ork")
    return raw.decode("utf-8")


def components(component, kind):
    """Every component under `component` (itself included) that is a `kind`."""
    found = [component] if isinstance(component, kind) else []
    for child in component.getChildren():
        found.extend(components(child, kind))
    return found


def name_of(constant):
    """An enum constant's name. Some of OpenRocket's event enums have a field called `name`, which
    hides the method, so it is called through reflection."""
    method = jpype.JClass("java.lang.Enum").class_.getMethod("name")
    return str(method.invoke(constant))


def words():
    """For each event, every value OpenRocket has and the word it writes for it."""
    from info.openrocket.core.rocketcomponent import AxialStage, MotorMount, RecoveryDevice

    document_ = example(STAGED)
    rocket = document_.getRocket()
    stages = components(rocket, AxialStage)
    booster = stages[-1]
    mount = [m for m in components(rocket, MotorMount) if m.isMotorMount()][0]
    device = components(rocket, RecoveryDevice)[0]

    separation = booster.getSeparationConfigurations().getDefault()
    deployment = device.getDeploymentConfigurations().getDefault()
    ignition = mount.getDefaultMotorConfig() if hasattr(mount, "getDefaultMotorConfig") else None
    if ignition is None:
        configuration = rocket.getSelectedConfiguration().getFlightConfigurationID()
        ignition = mount.getMotorConfig(configuration)

    found = {"ignition": {}, "deployment": {}, "separation": {}}
    # Each word is read from the one element the setting belongs to, found by its component's id:
    # the same word may be written elsewhere in the design for another component.
    cases = [
        ("separation", separation, "SeparationEvent", booster, "separationevent"),
        ("deployment", deployment, "DeployEvent", device, "deployevent"),
        ("ignition", ignition, "IgnitionEvent", mount, "motormount/ignitionevent"),
    ]
    for key, holder, event, owner, path in cases:
        getter = getattr(holder, f"get{event}")
        setter = getattr(holder, f"set{event}")
        for constant in getter().getDeclaringClass().getEnumConstants():
            setter(constant)
            element = element_with_id(saved(document_), str(owner.getID()))
            word = element.findtext(path)
            if word is None:
                sys.exit(f"no <{path}> on the component whose {key} was set")
            found[key][name_of(constant)] = {
                "word": word.strip(),
                "label": str(constant.toString()),
            }
    return found


def element_with_id(xml, id_):
    """The element of a saved design whose `<id>` is `id_`."""
    for element in ElementTree.fromstring(xml).iter():
        if (element.findtext("id") or "").strip() == id_:
            return element
    sys.exit(f"no component with id {id_} in the saved design")


def drag():
    """The drag coefficients OpenRocket gives an automatic parachute and streamer."""
    from info.openrocket.core.rocketcomponent import BodyTube, Parachute, Streamer

    document_ = example(SINGLE)
    rocket = document_.getRocket()
    chute = components(rocket, Parachute)[0]
    chute.setCDAutomatic(True)
    tube = components(rocket, BodyTube)[0]
    streamers = []
    for length_m, width_m in STREAMERS_M:
        streamer = Streamer()
        tube.addChild(streamer)
        streamer.setStripLength(length_m)
        streamer.setStripWidth(width_m)
        streamer.setCDAutomatic(True)
        streamers.append(
            {"length_m": length_m, "width_m": width_m, "cd": float(streamer.getCD())}
        )
        tube.removeChild(streamer)
    return {
        "parachute_automatic_cd": float(chute.getCD()),
        "parachute_default_cd": float(Parachute.DEFAULT_CD),
        "streamer_default_cd": float(Streamer.DEFAULT_CD),
        "streamer_max_computed_cd": float(Streamer.MAX_COMPUTED_CD),
        "streamers": streamers,
    }


def deploy_height(altitude_m):
    """Where a parachute set to deploy at `altitude_m` opens, on a pad at `PAD_ASL_M`."""
    from info.openrocket.core.rocketcomponent import DeploymentConfiguration, Parachute
    from info.openrocket.core.simulation import FlightDataType

    document_ = example(SINGLE)
    rocket = document_.getRocket()
    chute = components(rocket, Parachute)[0]
    deployment = chute.getDeploymentConfigurations().getDefault()
    deployment.setDeployEvent(DeploymentConfiguration.DeployEvent.ALTITUDE)
    deployment.setDeployAltitude(altitude_m)
    deployment.setDeployDelay(0.0)
    simulation = list(document_.getSimulations())[0]
    simulation.getOptions().setLaunchAltitude(PAD_ASL_M)
    # Calm air and a fixed seed, so that running the probe again writes the same numbers: the
    # example's own wind is turbulent, and a random seed moves the apogee by centimetres.
    options = simulation.getOptions()
    options.setWindSpeedAverage(0.0)
    options.setWindTurbulenceIntensity(0.0)
    options.setRandomSeed(SEED)
    simulation.simulate()
    branch = simulation.getSimulatedData().getBranch(0)
    times = list(branch.get(FlightDataType.TYPE_TIME))
    above_ground = list(branch.get(FlightDataType.TYPE_ALTITUDE))
    above_sea = list(branch.get(FlightDataType.TYPE_ALTITUDE_ABOVE_SEA))
    kinds = [name_of(event.getType()) for event in branch.getEvents()]
    found = {
        "pad_above_sea_m": PAD_ASL_M,
        "deploy_altitude_m": altitude_m,
        "apogee_above_ground_m": round(max(above_ground), 2),
        "reached_the_ground": "GROUND_HIT" in kinds,
        "deployed": False,
    }
    for event in branch.getEvents():
        if name_of(event.getType()) == "RECOVERY_DEVICE_DEPLOYMENT":
            time_s = float(event.getTime())
            found.update(
                {
                    "deployed": True,
                    # The heights at the event itself, between the samples either side of it.
                    "at_deployment_above_ground_m": round(at(times, above_ground, time_s), 2),
                    "at_deployment_above_sea_m": round(at(times, above_sea, time_s), 2),
                }
            )
            break
    return found


def at(times, values, time_s):
    """`values` at `time_s`, interpolated linearly between the samples either side."""
    index = bisect.bisect_left(times, time_s)
    if index <= 0:
        return values[0]
    if index >= len(times):
        return values[-1]
    t0, t1 = times[index - 1], times[index]
    share = 0.0 if t1 == t0 else (time_s - t0) / (t1 - t0)
    return values[index - 1] + share * (values[index] - values[index - 1])


def main():
    paths = sys.argv[1:]
    if len(paths) != 1:
        sys.exit("usage: events.py OUTPUT.json")
    start()
    from info.openrocket.core.util import BuildProperties

    fixture = {
        "source": "validation/oracles/openrocket/events.py",
        "command": "refs/venv/bin/python validation/oracles/openrocket/events.py "
        "validation/fixtures/ork/openrocket-events.json",
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(JAR.read_bytes()).hexdigest(),
        "java": str(jpype.java.lang.System.getProperty("java.version")),
        "jpype": jpype.__version__,
        "words": words(),
        "drag": drag(),
        "deploy_height": [deploy_height(altitude_m) for altitude_m in DEPLOY_ALTITUDES_M],
    }
    Path(paths[0]).write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
