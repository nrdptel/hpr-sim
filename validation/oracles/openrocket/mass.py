"""What OpenRocket 24.12 gives a `.ork` design's structure for mass, centre of mass and inertia.

hpr lays a design out into its structure's mass properties (`Layout::structure`): every stage
together, without motors. OpenRocket computes the same thing, `MassCalculator.calculateStructure`,
for a flight configuration. This script asks it, for every design it opens, so that
`cargo xtask ork` can hold hpr's numbers to OpenRocket's (M2.2a, the mass and centre-of-gravity
checks M1.4 left for the OpenRocket oracle).

Which of OpenRocket's inertias is which is measured, not assumed: a probe design, one tube of
known size and density, is read first, and its roll and pitch inertias about the centre of mass are
worked out by hand beside what OpenRocket reports. The record keeps both, and a test holds the
mapping. OpenRocket is run, never read: its source is GPL, and nothing here comes from it; the
class and method names are the public API `javap` prints for the jar.

Each design is saved once, to a throwaway, before it is read, so that OpenRocket works it out
again and every automatic dimension is the one it settles on (ADR-054). A design whose first
reading OpenRocket refuses because of a leading comment is read without it, as the RocketSerializer
cross-check does.

Run from the repository root with the oracle environment (ADR-059 has the second one, whose JPype
also works here), and Java 17 (see `automatic_radius.py`):

    refs/venv/bin/python validation/oracles/openrocket/mass.py \\
        validation/fixtures/ork/openrocket-mass-loft-demo.json validation/fixtures/ork/loft-demo

    refs/venv/bin/python validation/oracles/openrocket/mass.py \\
        corpus-out/openrocket-mass.json refs --jar

The first writes the committed record for Loft's public demo designs; the second covers the
reference library and the jar's examples, and stays out of the repository. The output is written
to the path given, not to standard output, which OpenRocket logs to.
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

import automatic_radius  # noqa: E402 - the JVM start, the loader and the settling save
import geometry  # noqa: E402 - the same file discovery and comment retry as the cross-check

# The probe: one tube, 1 m long, 50 mm outer radius, 2 mm wall, of a bulk material at 1000 kg/m³.
PROBE_LENGTH_M = 1.0
PROBE_OUTER_M = 0.05
PROBE_WALL_M = 0.002
PROBE_DENSITY = 1000.0
GENERATED = "2026-09-23"
PROBE = f"""<?xml version='1.0' encoding='utf-8'?>
<openrocket version="1.10" creator="hpr-sim mass probe"><rocket><name>Probe</name>
<subcomponents><stage><name>Stage</name><subcomponents>
<bodytube><name>Tube</name><length>{PROBE_LENGTH_M}</length><thickness>{PROBE_WALL_M}</thickness>
<radius>{PROBE_OUTER_M}</radius>
<material type="bulk" density="{PROBE_DENSITY}">Probe</material></bodytube>
</subcomponents></stage></subcomponents></rocket></openrocket>
"""


def structure(document):
    """OpenRocket's structure for the design's selected configuration: every stage it holds
    active, no motors."""
    from info.openrocket.core.masscalc import MassCalculator

    body = MassCalculator.calculateStructure(document.getRocket().getSelectedConfiguration())
    cm = body.getCM()
    return {
        "mass_kg": float(body.getMass()),
        "cm_x_m": float(cm.x),
        "ixx": float(body.getIxx()),
        "iyy": float(body.getIyy()),
        "izz": float(body.getIzz()),
        "rotational_inertia": float(body.getRotationalInertia()),
        "longitudinal_inertia": float(body.getLongitudinalInertia()),
    }


def parts(document):
    """Each part as OpenRocket's own mass breakdown counts it: its mass in the structure (every
    instance, after overrides), its centre of mass's station, and what it says about overrides.
    This is what a design's difference is traced to, part by part, by id."""
    from info.openrocket.core.masscalc import MassCalculator

    found, skipped = [], 0
    config = document.getRocket().getSelectedConfiguration()
    for entry in MassCalculator.getCMAnalysis(config).values():
        component = entry.source
        try:
            ident = str(component.getID())
            by = component.getMassOverriddenBy()
        except Exception:  # noqa: BLE001 - an entry whose source is not a component, counted
            skipped += 1
            continue
        found.append(
            {
                "id": ident,
                "name": str(component.getName()),
                "class": str(component.getClass().getSimpleName()),
                "mass_kg": float(entry.totalCM.weight),
                "cm_x_m": float(entry.totalCM.x),
                "each_mass_kg": float(entry.eachMass),
                "component_mass_kg": float(component.getComponentMass()),
                "mass_overridden": bool(component.isMassOverridden()),
                "override_mass_kg": float(component.getOverrideMass()),
                "overridden_by": str(by.getID()) if by is not None else None,
            }
        )
    return sorted(found, key=lambda part: part["id"]), skipped


def probe(scratch):
    """The probe tube as OpenRocket reads it, beside its mass properties worked out by hand."""
    path = Path(scratch) / "probe.ork"
    path.write_text(PROBE, encoding="utf-8")
    document = automatic_radius.load(path)
    automatic_radius.saved_radii(document)
    ro, ri = PROBE_OUTER_M, PROBE_OUTER_M - PROBE_WALL_M
    mass = PROBE_DENSITY * math.pi * (ro**2 - ri**2) * PROBE_LENGTH_M
    return {
        "document": PROBE,
        "by_hand": {
            "mass_kg": mass,
            "cm_x_m": PROBE_LENGTH_M / 2,
            # A thick-walled tube about its own axis, and about a diameter through its middle.
            "roll_inertia": mass * (ro**2 + ri**2) / 2,
            "pitch_inertia": mass * ((ro**2 + ri**2) / 4 + PROBE_LENGTH_M**2 / 12),
        },
        "openrocket": structure(document),
    }


def read(path, scratch):
    record = {"sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest()}
    # A file this script cannot unpack is its own failure, not OpenRocket's refusal.
    try:
        text = geometry.document_text(path)
    except Exception as error:  # noqa: BLE001 - recorded, and the survey fails on it
        record["driver_error"] = geometry.first_line(error)
        return record
    try:
        document, stripped = geometry.opened(text, scratch)
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        record["opens"] = False
        record["refused"] = geometry.first_line(error)
        return record
    record["opens"] = True
    record["comment_removed"] = stripped
    automatic_radius.saved_radii(document)
    record["structure"] = structure(document)
    record["parts"], record["parts_skipped"] = parts(document)
    # OpenRocket weighs the stages its selected configuration holds active; hpr weighs them all.
    config = document.getRocket().getSelectedConfiguration()
    record["stages"] = {
        "active": int(config.getActiveStageCount()),
        "total": int(config.getStageCount()),
    }
    return record


def main():
    args = sys.argv[1:]
    jar = "--jar" in args
    paths = [arg for arg in args if not arg.startswith("--")]
    if len(paths) < 2 and not (jar and paths):
        sys.exit("usage: mass.py OUTPUT.json (FILE | DIR)... [--jar]")
    output, inputs = Path(paths[0]), paths[1:]
    root = Path.cwd().resolve()
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    runs = []
    with tempfile.TemporaryDirectory() as scratch:
        measured = probe(scratch)
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
                runs.append({"file": name, **read(path, scratch)})
            except Exception as error:  # noqa: BLE001 - recorded, and the survey fails on it
                runs.append({"file": name, "driver_error": geometry.first_line(error)})

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/mass.py",
                "generated": GENERATED,
                "inputs_sha256": {
                    "mass.py": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    "automatic_radius.py": hashlib.sha256(
                        Path(automatic_radius.__file__).read_bytes()
                    ).hexdigest(),
                    "geometry.py": hashlib.sha256(Path(geometry.__file__).read_bytes()).hexdigest(),
                },
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "command": " ".join(["mass.py", *sys.argv[1:]]),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "probe": measured,
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
