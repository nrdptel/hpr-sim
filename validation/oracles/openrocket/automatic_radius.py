"""What OpenRocket gives an automatic body radius that has no fixed radius to take.

A `.ork` file marks a body component's radius `auto` when OpenRocket is to work it out from the
neighbours. A chain of them with no fixed radius anywhere along it has nothing to work from.
OpenRocket's maintainers write that such a radius is "the default radius" (openrocket#1988, #1992),
and a user reports that default as a diameter of 1.969 in (#871), but no document gives the number.
This script measures it: it runs OpenRocket 24.12 as an external oracle on small designs written
here, reads back the radius OpenRocket resolved for every body component, and saves each design
again to see what OpenRocket writes after `auto`.

Each radius is read twice. `opened` is what OpenRocket holds right after loading the file;
`resolved` is what it holds once saving has made it work every radius out again, which is also
what it writes. The two differ on one shape: a tube whose automatic neighbour holds a part with an
automatic radius of its own reads OpenRocket's default when opened, and its neighbour's radius once
resolved. `resolved` is the answer hpr is held to.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. JPype starts the JVM inside
this Python process (ADR-035 explains why that matters); this is a probe of one behaviour, not a
decision on how M2.2's flight oracle drives the jar.

With `--library`, the two reference-library designs whose chains motivated the question are run
too, when `refs/` holds them, and so are the example designs inside the jar, so that every body
radius OpenRocket resolves there can be held against hpr's; only their radii are recorded, never
their contents.

OpenRocket 24.12 needs Java 17 exactly. Set JAVA_HOME to a Java 17 home, or install Homebrew's
keg-only `openjdk@17`, which is found without it. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/automatic_radius.py --library \\
        validation/fixtures/ork/openrocket-automatic-radius.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import io
import json
import os
import re
import sys
import tempfile
import zipfile
from pathlib import Path

import jpype
import jpype.imports

JAR = Path("refs/openrocket/OpenRocket-24.12.jar")
LIBRARY = [
    "refs/fusionspace-loft/fixtures/demo-quirks.ork",
    "refs/openrocket-database/ork/parachutes.ork",
]
KEGS = [
    "/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
    "/usr/local/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home",
]

MATERIAL = '<material type="bulk" density="680.0">Cardboard</material>'


def tube(radius):
    return (
        f"<bodytube><name>Tube</name>{MATERIAL}<length>0.3</length>"
        f"<thickness>0.001</thickness><radius>{radius}</radius></bodytube>"
    )


def tube_holding_coupler(radius):
    """A tube with a coupler inside it whose outer radius is automatic: the tube's bore."""
    return (
        f"<bodytube><name>Tube</name>{MATERIAL}<length>0.03</length>"
        f"<thickness>0.001</thickness><radius>{radius}</radius><subcomponents>"
        f"<tubecoupler><name>Coupler</name>{MATERIAL}<length>0.1</length>"
        "<outerradius>auto</outerradius><thickness>0.002</thickness></tubecoupler>"
        "</subcomponents></bodytube>"
    )


def nose(aft):
    return (
        f"<nosecone><name>Nose</name>{MATERIAL}<length>0.2</length>"
        f"<thickness>0.001</thickness><shape>ogive</shape><aftradius>{aft}</aftradius></nosecone>"
    )


def transition(fore, aft):
    return (
        f"<transition><name>Transition</name>{MATERIAL}<length>0.1</length>"
        f"<thickness>0.001</thickness><shape>conical</shape>"
        f"<foreradius>{fore}</foreradius><aftradius>{aft}</aftradius></transition>"
    )


# Each case is a spine, forward to aft; a list of spines is one per stage. The numbers after
# `auto` are what a file would have cached; the question is whether OpenRocket reads them.
CASES = {
    "a tube alone": [tube("auto")],
    "a tube alone, caching 0.04": [tube("auto 0.04")],
    "two tubes, one caching 0.04": [tube("auto"), tube("auto 0.04")],
    "a nose cone alone": [nose("auto")],
    "a nose cone alone, caching 0.03": [nose("auto 0.03")],
    "a transition alone, caching both ends": [transition("auto 0.03", "auto 0.02")],
    "a nose cone and a tube, both caching 0.03": [nose("auto 0.03"), tube("auto 0.03")],
    "nose, tube and transition automatic, the transition's aft end fixed": [
        nose("auto 0.033"),
        tube("auto"),
        transition("auto", "0.022"),
        tube("0.022"),
    ],
    # A control: here the neighbour rule has a fixed radius to take, and must take it.
    "a nose cone before a fixed tube": [nose("auto"), tube("0.03")],
    # Chains that do reach a fixed radius, but through more than one automatic radius: where
    # OpenRocket's neighbour rule and hpr's may part.
    "a nose cone and two automatic tubes before a fixed tube": [
        nose("auto"),
        tube("auto"),
        tube("auto"),
        tube("0.03"),
    ],
    # The same, but the second tube holds a coupler whose radius is automatic: the shape of the
    # OpenRocket example "Dual parachute deployment", where OpenRocket's answer for the first tube
    # changes with that unrelated coupler.
    "the same, with a coupler of automatic radius inside the second tube": [
        nose("auto"),
        tube("auto"),
        tube_holding_coupler("auto"),
        tube("0.03"),
    ],
    "a fixed tube and two automatic tubes": [tube("0.03"), tube("auto"), tube("auto")],
    "a transition's automatic aft end before an automatic tube": [
        tube("0.03"),
        transition("0.03", "auto"),
        tube("auto"),
    ],
    "a nose cone before an automatic transition end": [
        nose("auto"),
        transition("auto", "0.02"),
        tube("0.02"),
    ],
    "a nose cone and a tube, both automatic, then a stage with a fixed tube": [
        [nose("auto"), tube("auto")],
        [tube("0.03")],
    ],
}


def document(spine):
    stages = spine if isinstance(spine[0], list) else [spine]
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        '<openrocket version="1.10" creator="hpr-sim automatic-radius probe">'
        "<rocket><name>Probe</name><subcomponents>"
        + "".join(
            f"<stage><name>Stage {k + 1}</name><subcomponents>{''.join(parts)}</subcomponents></stage>"
            for k, parts in enumerate(stages)
        )
        + "</subcomponents></rocket></openrocket>\n"
    )


def java_home():
    """The first Java 17 home: JAVA_HOME if it is one, then the Homebrew kegs."""
    for home in [os.environ.get("JAVA_HOME"), *KEGS]:
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
            jvm = str(home / lib)
            break
    else:
        sys.exit(f"no JVM library under {home}")
    jpype.startJVM(jvm, "-Djava.awt.headless=true", classpath=[str(JAR)])

    from com.google.inject import Guice
    from info.openrocket.core.database import ComponentPresetDao, ComponentPresetDatabase
    from info.openrocket.core.database.motor import MotorDatabase, ThrustCurveMotorSetDatabase
    from info.openrocket.core.plugin import PluginModule
    from info.openrocket.core.startup import Application
    from info.openrocket.swing.utils import CoreServicesModule

    @jpype.JImplements("com.google.inject.Module")
    class Databases:
        # Empty databases: loading a design needs both bound, and the graphical providers need a
        # display. No radius here depends on a motor or a preset.
        @jpype.JOverride
        def configure(self, binder):
            binder.bind(MotorDatabase).toInstance(ThrustCurveMotorSetDatabase())
            binder.bind(ComponentPresetDao).toInstance(ComponentPresetDatabase())

    Application.setInjector(Guice.createInjector(CoreServicesModule(), PluginModule(), Databases()))


def body_radii(rocket):
    """Each body component's radii as OpenRocket resolves them, forward to aft."""
    from info.openrocket.core.rocketcomponent import AxialStage, BodyTube, NoseCone, Transition

    found = []
    for stage in rocket.getChildren():
        if not isinstance(stage, AxialStage):
            continue
        for component in stage.getChildren():
            if isinstance(component, BodyTube):
                found.append({"kind": "body_tube", "outer": float(component.getOuterRadius())})
            elif isinstance(component, NoseCone):
                found.append({"kind": "nose_cone", "base": float(component.getAftRadius())})
            elif isinstance(component, Transition):
                found.append(
                    {
                        "kind": "transition",
                        "fore": float(component.getForeRadius()),
                        "aft": float(component.getAftRadius()),
                    }
                )
    return found


def saved_radii(document_):
    """The body radius tags OpenRocket writes when it saves the design again."""
    from info.openrocket.core.document import StorageOptions
    from info.openrocket.core.file.openrocket import OpenRocketSaver
    from info.openrocket.core.logging import ErrorSet, WarningSet
    from java.io import ByteArrayOutputStream

    out = ByteArrayOutputStream()
    OpenRocketSaver().save(out, document_, StorageOptions(), WarningSet(), ErrorSet())
    raw = bytes(out.toByteArray())
    # The saver writes the bare document; the zip around it is added by the layer above.
    if raw.startswith(b"PK"):
        raw = zipfile.ZipFile(io.BytesIO(raw)).read("rocket.ork")
    xml = raw.decode("utf-8")
    design = xml.split("<simulations")[0]
    return re.findall(r"<(?:radius|aftradius|foreradius)>([^<]*)<", design)


def load(path):
    from info.openrocket.core.file import GeneralRocketLoader
    from java.io import File

    return GeneralRocketLoader(File(str(path))).load()


def main():
    args = sys.argv[1:]
    library = "--library" in args
    paths = [arg for arg in args if not arg.startswith("--")]
    if len(paths) != 1:
        sys.exit("usage: automatic_radius.py [--library] OUTPUT.json")
    start()
    from info.openrocket.core.util import BuildProperties

    cases = []
    with tempfile.TemporaryDirectory() as scratch:
        for name, spine in CASES.items():
            text = document(spine)
            path = Path(scratch) / "probe.ork"
            path.write_text(text, encoding="utf-8")
            loaded = load(path)
            opened = body_radii(loaded.getRocket())
            saved = saved_radii(loaded)
            cases.append(
                {
                    "name": name,
                    "document": text,
                    "written": re.findall(r"<(?:radius|aftradius|foreradius)>([^<]*)<", text),
                    "opened": opened,
                    "saved": saved,
                    "resolved": body_radii(loaded.getRocket()),
                }
            )

    runs = []
    if library:
        with tempfile.TemporaryDirectory() as scratch:
            files = [(name, Path(name)) for name in LIBRARY if Path(name).exists()]
            with zipfile.ZipFile(JAR) as jar:
                for entry in sorted(jar.namelist()):
                    if entry.startswith("datafiles/examples/") and entry.endswith(".ork"):
                        path = Path(scratch) / Path(entry).name
                        path.write_bytes(jar.read(entry))
                        files.append((f"{JAR}!/{entry}", path))
            for name, path in files:
                try:
                    loaded = load(path)
                    opened = body_radii(loaded.getRocket())
                    saved_radii(loaded)
                    resolved = body_radii(loaded.getRocket())
                    runs.append(
                        {"file": name, "opens": True, "opened": opened, "resolved": resolved}
                    )
                except Exception as error:  # noqa: BLE001 - the refusal is the measurement
                    message = str(error).splitlines()[0]
                    runs.append({"file": name, "opens": False, "error": message})

    from java.lang import System

    fixture = {
        "source": "validation/oracles/openrocket/automatic_radius.py",
        "command": "automatic_radius.py" + (" --library" if library else ""),
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(JAR.read_bytes()).hexdigest(),
        "java": str(System.getProperty("java.version")),
        "jpype": jpype.__version__,
        "cases": cases,
        "library": runs,
    }
    text = json.dumps(fixture, indent=2, ensure_ascii=False) + "\n"
    Path(paths[0]).write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
