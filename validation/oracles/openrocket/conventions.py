"""What OpenRocket 24.12 weighs when a `.ork` leaves something unsaid or says it twice.

M2.2a held each design's structure to OpenRocket's and found the files outside a threshold fall
into a handful of causes (ADR-060). Some of them are not physics but readings: what a shoulder
written with no wall weighs, what a part written with no material is made of, where a centre of
gravity override is measured from, and which override wins when a part and the parts inside it
both have one. OpenRocket's answer is what the file means, so this script measures it (M2.2b):
it writes small probe designs, each asking one question, has OpenRocket 24.12 read them, and
records its structure (mass, centre of mass, inertias) and its own per-part breakdown. hpr's test
`hpr_validate::openrocket::tests` reads the same documents and holds hpr to the answers.

It also records the material OpenRocket gives each kind of part that names none, read through the
part's public `getMaterial` and `getLineMaterial`.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API `javap` prints for the jar. Each probe is saved once, to a
throwaway, before it is read, as `mass.py` does, so that every automatic dimension is settled.

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/conventions.py \\
        validation/fixtures/ork/openrocket-conventions.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import logging
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start, the loader and the settling save
import mass  # noqa: E402 - the structure and the per-part breakdown, asked the same way


def uid(n):
    """A fixed id, so that hpr and OpenRocket name each part the same way."""
    return f"00000000-0000-4000-8000-{n:012d}"


# One material for every part that names one: 1,000 kg/m³, so masses read as volumes.
MATERIAL = '<material type="bulk" density="1000.0">Probe</material>'


def nose(extra="", thickness="0.002", material=MATERIAL):
    """A conical nose, 0.3 m long on a 50 mm base, with a 2 mm wall."""
    return (
        f"<nosecone><name>Nose</name><id>{uid(1)}</id><length>0.3</length>"
        f"<thickness>{thickness}</thickness><shape>conical</shape><aftradius>0.05</aftradius>"
        f"{extra}{material}</nosecone>"
    )


def shoulder(end, thickness, capped="false"):
    """A shoulder 0.1 m long, 48 mm in radius, at one end of a nose or transition."""
    return (
        f"<{end}shoulderradius>0.048</{end}shoulderradius>"
        f"<{end}shoulderlength>0.1</{end}shoulderlength>"
        f"<{end}shoulderthickness>{thickness}</{end}shoulderthickness>"
        f"<{end}shouldercapped>{capped}</{end}shouldercapped>"
    )


def tube(extra="", children="", thickness="0.002", material=MATERIAL):
    """A body tube 0.5 m long, 50 mm in radius, with a 2 mm wall."""
    inside = f"<subcomponents>{children}</subcomponents>" if children else ""
    return (
        f"<bodytube><name>Tube</name><id>{uid(2)}</id><length>0.5</length>"
        f"<thickness>{thickness}</thickness><radius>0.05</radius>{extra}{material}{inside}"
        "</bodytube>"
    )


def transition(extra="", thickness="0.002"):
    """A transition 0.1 m long from 50 mm to 40 mm, with a 2 mm wall."""
    return (
        f"<transition><name>Transition</name><id>{uid(3)}</id><length>0.1</length>"
        f"<thickness>{thickness}</thickness><shape>conical</shape>"
        f"<foreradius>0.05</foreradius><aftradius>0.04</aftradius>{extra}{MATERIAL}</transition>"
    )


def inner(extra="", material=MATERIAL):
    """An inner tube 0.2 m long, 20 mm in radius, 1 mm wall, 0.1 m below its parent's top."""
    return (
        f"<innertube><name>Inner</name><id>{uid(4)}</id><position type=\"top\">0.1</position>"
        "<length>0.2</length><outerradius>0.02</outerradius><thickness>0.001</thickness>"
        f"{extra}{material}</innertube>"
    )


def overrides(mass_kg=None, cg_m=None, children_mass=None, children_cg=None):
    """The override tags, each written only when given."""
    tags = []
    if mass_kg is not None:
        tags.append(f"<overridemass>{mass_kg}</overridemass>")
    if cg_m is not None:
        tags.append(f"<overridecg>{cg_m}</overridecg>")
    if children_mass is not None:
        tags.append(f"<overridesubcomponentsmass>{children_mass}</overridesubcomponentsmass>")
    if children_cg is not None:
        tags.append(f"<overridesubcomponentscg>{children_cg}</overridesubcomponentscg>")
    return "".join(tags)


# The parts that name no material, for the defaults: one of each kind hpr reads, each placed
# 0.1 m below the top of its tube so that only the material is in question.
NO_MATERIAL_CHILDREN = (
    f"<innertube><name>Inner</name><id>{uid(4)}</id><position type=\"top\">0.1</position>"
    "<length>0.2</length><outerradius>0.02</outerradius><thickness>0.001</thickness></innertube>"
    f"<trapezoidfinset><name>Fins</name><id>{uid(5)}</id><position type=\"top\">0.1</position><fincount>3</fincount>"
    "<rootchord>0.1</rootchord><tipchord>0.05</tipchord><sweeplength>0.05</sweeplength>"
    "<height>0.05</height><thickness>0.003</thickness><crosssection>square</crosssection>"
    "</trapezoidfinset>"
    f"<centeringring><name>Ring</name><id>{uid(6)}</id><position type=\"top\">0.1</position><length>0.01</length>"
    "<outerradius>auto</outerradius><innerradius>0.02</innerradius></centeringring>"
    f"<bulkhead><name>Bulkhead</name><id>{uid(7)}</id><position type=\"top\">0.1</position><length>0.01</length>"
    "<outerradius>auto</outerradius></bulkhead>"
    f"<launchlug><name>Lug</name><id>{uid(8)}</id><position type=\"top\">0.1</position><length>0.05</length>"
    "<radius>0.005</radius><thickness>0.001</thickness></launchlug>"
    f"<parachute><name>Chute</name><id>{uid(9)}</id><position type=\"top\">0.1</position><packedlength>0.05</packedlength>"
    "<packedradius>0.02</packedradius><diameter>0.5</diameter><linecount>6</linecount><linelength>0.5</linelength></parachute>"
    f"<shockcord><name>Cord</name><id>{uid(10)}</id><position type=\"top\">0.1</position><packedlength>0.05</packedlength>"
    "<packedradius>0.01</packedradius><cordlength>1.0</cordlength></shockcord>"
    f"<streamer><name>Streamer</name><id>{uid(11)}</id><position type=\"top\">0.1</position><packedlength>0.05</packedlength>"
    "<packedradius>0.01</packedradius><striplength>1.0</striplength><stripwidth>0.05</stripwidth></streamer>"
    f"<railbutton><name>Button</name><id>{uid(12)}</id><position type=\"top\">0.1</position><outerdiameter>0.01</outerdiameter>"
    "<innerdiameter>0.006</innerdiameter><height>0.008</height><baseheight>0.002</baseheight>"
    "<flangeheight>0.002</flangeheight><instancecount>1</instancecount></railbutton>"
)

# Each probe: what it asks, and the stage's parts. A probe of several stages gives a list of them.
PROBES = {
    # Shoulders and walls.
    "a nose with no shoulder": [nose()],
    "a nose with a walled shoulder": [nose(shoulder("aft", "0.002"))],
    "a nose whose shoulder has no wall": [nose(shoulder("aft", "0.0"))],
    "a nose whose shoulder has no wall, capped": [nose(shoulder("aft", "0.0", "true"))],
    "a filled nose whose shoulder has no wall": [nose(shoulder("aft", "0.0"), "filled")],
    "a filled nose with a walled shoulder": [nose(shoulder("aft", "0.002"), "filled")],
    "a transition whose shoulders have no wall": [
        transition(shoulder("fore", "0.0") + shoulder("aft", "0.0"))
    ],
    "a nose of no wall": [nose(thickness="0.0")],
    "a transition of no wall": [transition(thickness="0.0")],
    "a tube of no wall": [tube(thickness="0.0")],
    "a nose, a shoulder and a tube that write no thickness at all": [
        nose(shoulder("aft", "0.002").replace(
            "<aftshoulderthickness>0.002</aftshoulderthickness>", ""
        ), thickness="").replace("<thickness></thickness>", ""),
        tube(thickness="").replace("<thickness></thickness>", ""),
    ],
    "a narrower nose and tube that write no thickness at all": [
        nose(thickness="").replace("<thickness></thickness>", "").replace(
            "<aftradius>0.05</aftradius>", "<aftradius>0.03</aftradius>"
        ),
        tube(thickness="").replace("<thickness></thickness>", "").replace(
            "<radius>0.05</radius>", "<radius>0.03</radius>"
        ),
    ],
    # Materials.
    "a nose and a tube that name no material, with one part of each kind inside": [
        nose(material=""),
        tube(children=NO_MATERIAL_CHILDREN, material=""),
    ],
    # Overrides on a part with a shoulder (Loft lesson L51).
    "a centre of gravity override on a nose with a shoulder": [
        nose(shoulder("aft", "0.002") + overrides(cg_m=0.25))
    ],
    "a mass override on a nose with a shoulder": [
        nose(shoulder("aft", "0.002") + overrides(mass_kg=0.5))
    ],
    "a centre of gravity override on a transition with a fore shoulder": [
        tube(),
        transition(shoulder("fore", "0.002") + overrides(cg_m=0.02)),
    ],
    # Overrides on a part with a part inside.
    "a mass override on a tube, not the part inside": [
        tube(overrides(mass_kg=0.5, children_mass="false"), inner())
    ],
    "a mass override on a tube and the part inside": [
        tube(overrides(mass_kg=0.5, children_mass="true"), inner())
    ],
    "a centre of gravity override on a tube, not the part inside": [
        tube(overrides(cg_m=0.1, children_cg="false"), inner())
    ],
    "a centre of gravity override on a tube and the part inside": [
        tube(overrides(cg_m=0.1, children_cg="true"), inner())
    ],
    "both overrides on a tube, the mass covering the part inside and the centre not": [
        tube(overrides(0.5, 0.1, "true", "false"), inner())
    ],
    "both overrides on a tube, the centre covering the part inside and the mass not": [
        tube(overrides(0.5, 0.1, "false", "true"), inner())
    ],
    "a mass override on a tube and the part inside, which has its own": [
        tube(overrides(mass_kg=0.5, children_mass="true"), inner(overrides(mass_kg=0.2)))
    ],
    "a mass override on the part inside only": [tube(children=inner(overrides(mass_kg=0.2)))],
}

# Stage overrides are written on the stage, so these probes carry the stage's tags too.
STAGE_PROBES = {
    "a mass override on the stage": (overrides(mass_kg=2.0, children_mass="true"), [nose(), tube()]),
    "both overrides on the stage": (
        overrides(2.0, 0.4, "true", "true"),
        [nose(), tube()],
    ),
    "a stage override over a part's own": (
        overrides(mass_kg=2.0, children_mass="true"),
        [nose(overrides(mass_kg=0.5)), tube()],
    ),
}


def document(parts, stage_tags=""):
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        '<openrocket version="1.10" creator="hpr-sim conventions probe">'
        "<rocket><name>Probe</name><subcomponents>"
        f"<stage><name>Stage</name><id>{uid(99)}</id>{stage_tags}<subcomponents>"
        + "".join(parts)
        + "</subcomponents></stage></subcomponents></rocket></openrocket>\n"
    )


def materials(component, found):
    """The material each part is read with, by id, through its public getters."""
    entry = {}
    for getter, key in [("getMaterial", "material"), ("getLineMaterial", "line_material")]:
        if hasattr(component, getter):
            material = getattr(component, getter)()
            entry[key] = {
                "name": str(material.getName()),
                "density": float(material.getDensity()),
                "kind": str(material.getType()).lower(),
            }
    if entry:
        found[str(component.getID())] = {
            "class": str(component.getClass().getSimpleName()),
            **entry,
        }
    for child in component.getChildren():
        materials(child, found)


def measure(text, scratch, name):
    path = Path(scratch) / f"{name}.ork"
    path.write_text(text, encoding="utf-8")
    document_ = automatic_radius.load(path)
    automatic_radius.saved_radii(document_)
    found = {}
    materials(document_.getRocket(), found)
    parts, skipped = mass.parts(document_)
    return {
        "document": text,
        "structure": mass.structure(document_),
        "parts": parts,
        "parts_skipped": skipped,
        "materials": found,
    }


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: conventions.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    probes = {}
    with tempfile.TemporaryDirectory() as scratch:
        everything = [(q, document(parts)) for q, parts in PROBES.items()]
        everything += [(q, document(parts, tags)) for q, (tags, parts) in STAGE_PROBES.items()]
        for k, (question, text) in enumerate(everything):
            probes[question] = measure(text, scratch, f"probe-{k}")

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/conventions.py",
                "command": " ".join(
                    ["refs/venv/bin/python", "validation/oracles/openrocket/conventions.py"]
                    + sys.argv[1:]
                ),
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
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
