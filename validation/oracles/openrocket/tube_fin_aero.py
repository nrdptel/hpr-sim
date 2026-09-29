"""OpenRocket 24.12's normal force and centre of pressure for tube fins, per component (M2.2f).

Loft lesson L19 asks that hpr put a tube fin set's centre of pressure within a quarter calibre of
OpenRocket's. On OpenRocket's *Tube fin rocket* hpr's is about a calibre forward of it (ADR-099),
from a look at OpenRocket's per-component output that was never kept. This script keeps it: it runs
OpenRocket 24.12 as an external oracle on small designs written here, and on the example inside
the jar, and records what its Barrowman calculator gives each component: the normal-force slope
`C_Nα` on the reference area and the centre of pressure, at several Mach numbers.

The probes are one airframe, a 12.5 mm radius body with a nose, and a tube fin set at its foot.
Each varies one thing from the first (six tubes of automatic radius, 75 mm long, a 0.3 mm wall):
the tubes' length, their count at a stated radius, their radius, their wall. Every part has a fixed
id, so hpr and OpenRocket name each one the same way. `hpr_validate::openrocket`'s tests read the
same documents with `hpr_io::ork` and hold hpr's slopes and centres to these.

Each (probe, Mach) pair is asked of a newly loaded design and a new calculator: one calculator
asked again after a part changes answers from its cache. The angle of attack is 1e-4 rad; the
slope OpenRocket reports is the centre's `weight`.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. OpenRocket 24.12 needs Java 17
exactly (see `automatic_radius.py`). Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/tube_fin_aero.py \\
        validation/fixtures/ork/openrocket-tube-fin-aero.json
"""

import hashlib
import json
import logging
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start and the loader

# Provenance written into the fixture (docs/VALIDATION.md). Update when regenerating.
GENERATED = "2026-09-28"

EXAMPLE = "datafiles/examples/Tube fin rocket.ork"

#: The Mach numbers each probe is asked at: hpr's tube-fin model stops short of 0.8.
MACHS = [0.05, 0.3, 0.5, 0.6, 0.75]

#: The angle of attack each is asked at, rad.
ALPHA_RAD = 1e-4

MATERIAL = '<material type="bulk" density="1000.0">Probe</material>'


def uid(n):
    """A fixed id, so that hpr and OpenRocket name each part the same way."""
    return f"00000000-0000-4000-8000-{n:012d}"


def tube_fin_set(count=6, radius="auto", length="0.075", thickness="0.0003"):
    """`count` tubes at the foot of the body, `length` long with a `thickness` wall, of `radius`
    (`auto`: touching the body and each other)."""
    return (
        f"<tubefinset><name>Tube fins</name><id>{uid(3)}</id>"
        '<axialoffset method="bottom">0.0</axialoffset><position type="bottom">0.0</position>'
        f"<instancecount>{count}</instancecount><fincount>{count}</fincount>"
        '<radiusoffset method="coaxial">0.0</radiusoffset>'
        '<angleoffset method="fixed">0.0</angleoffset><rotation>0.0</rotation>'
        f"<finish>normal</finish>{MATERIAL}<radius>{radius}</radius><length>{length}</length>"
        f"<thickness>{thickness}</thickness></tubefinset>"
    )


def document(fins):
    """An ogive nose 0.12 m long and a body tube 0.45 m long, both 12.5 mm in radius, with `fins`
    at the body's foot."""
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        '<openrocket version="1.10" creator="hpr-sim tube fin probe">'
        f"<rocket><name>Probe</name><id>{uid(98)}</id>"
        "<referencetype>maximum</referencetype><subcomponents>"
        f"<stage><name>Stage</name><id>{uid(99)}</id><subcomponents>"
        f"<nosecone><name>Nose</name><id>{uid(1)}</id><finish>normal</finish>{MATERIAL}"
        "<length>0.12</length><thickness>0.001</thickness><shape>ogive</shape>"
        "<shapeparameter>1.0</shapeparameter><aftradius>0.0125</aftradius>"
        "<aftshoulderradius>0.0</aftshoulderradius><aftshoulderlength>0.0</aftshoulderlength>"
        "<aftshoulderthickness>0.0</aftshoulderthickness>"
        "<aftshouldercapped>false</aftshouldercapped></nosecone>"
        f"<bodytube><name>Body</name><id>{uid(2)}</id><finish>normal</finish>{MATERIAL}"
        "<length>0.45</length><thickness>0.0005</thickness><radius>0.0125</radius>"
        f"<subcomponents>{fins}</subcomponents></bodytube>"
        "</subcomponents></stage></subcomponents></rocket></openrocket>\n"
    )


PROBES = {
    "six touching tubes": tube_fin_set(),
    **{
        f"six touching tubes {length} m long": tube_fin_set(length=length)
        for length in ["0.025", "0.05", "0.15", "0.3"]
    },
    **{
        f"{count} tubes of 6 mm radius": tube_fin_set(count=count, radius="0.006")
        for count in [3, 4, 5, 6, 8]
    },
    **{
        f"six tubes of {radius} m radius": tube_fin_set(radius=radius)
        for radius in ["0.004", "0.008"]
    },
    **{
        f"six touching tubes with a {thickness} m wall": tube_fin_set(thickness=thickness)
        for thickness in ["0.001", "0.003"]
    },
}


def forces(path, mach):
    """Each component's `C_Nα` and centre of pressure, by id, and the tube fin sets' geometry as
    OpenRocket resolved it, from a new load of `path` and a new calculator at `mach`."""
    from info.openrocket.core.aerodynamics import BarrowmanCalculator, FlightConditions
    from info.openrocket.core.logging import WarningSet
    from info.openrocket.core.rocketcomponent import TubeFinSet
    from info.openrocket.core.util import Coordinate

    rocket = automatic_radius.load(path).getRocket()
    configuration = rocket.getSelectedConfiguration()
    conditions = FlightConditions(configuration)
    conditions.setMach(mach)
    conditions.setAOA(ALPHA_RAD)
    warnings = WarningSet()
    analysis = BarrowmanCalculator().getForceAnalysis(configuration, conditions, warnings)
    components = {}
    for component, force in analysis.items():
        cp = force.getCP()
        entry = {
            "class": str(component.getClass().getSimpleName()),
            "name": str(component.getName()),
            "cna_per_rad": float(cp.weight),
            "cp_x_m": float(cp.x),
        }
        if isinstance(component, TubeFinSet):
            entry["leading_edge_x_m"] = float(component.toAbsolute(Coordinate(0, 0, 0))[0].x)
            entry["length_m"] = float(component.getLength())
            entry["outer_radius_m"] = float(component.getOuterRadius())
            entry["inner_radius_m"] = float(component.getInnerRadius())
            entry["body_radius_m"] = float(component.getBodyRadius())
            entry["count"] = int(component.getFinCount())
        components[str(component.getID())] = entry
    return {
        "mach": mach,
        "reference_area_m2": float(conditions.getRefArea()),
        "reference_length_m": float(conditions.getRefLength()),
        "warnings": sorted(str(warning) for warning in warnings),
        "components": components,
    }


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: tube_fin_aero.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    probes = {}
    with tempfile.TemporaryDirectory() as scratch:
        for name, fins in PROBES.items():
            text = document(fins)
            path = Path(scratch) / "probe.ork"
            path.write_text(text, encoding="utf-8")
            probes[name] = {
                "document": text,
                "machs": [forces(path, mach) for mach in MACHS],
            }
        with zipfile.ZipFile(automatic_radius.JAR) as jar:
            raw = jar.read(EXAMPLE)
        path = Path(scratch) / "example.ork"
        path.write_bytes(raw)
        example = {
            "file": f"{automatic_radius.JAR}!/{EXAMPLE}",
            "sha256": hashlib.sha256(raw).hexdigest(),
            "machs": [forces(path, mach) for mach in MACHS],
        }

    fixture = {
        "source": "validation/oracles/openrocket/tube_fin_aero.py",
        "command": "tube_fin_aero.py",
        "generated": GENERATED,
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
        "java": str(System.getProperty("java.version")),
        "jpype": jpype.__version__,
        "alpha_rad": ALPHA_RAD,
        "probes": probes,
        "example": example,
    }


    text = json.dumps(fixture, indent=2, ensure_ascii=False) + "\n"
    output.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
