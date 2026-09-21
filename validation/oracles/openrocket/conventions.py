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
part's public `getMaterial` and `getLineMaterial`. M2.2b2 added probes of one tube and one part
each (every kind of part, fin outlines, sections, a tab, fillets, a cant, rail buttons from each
end), so that a part's roll inertia is the probe's less the tube's (ADR-062).

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


def transition(extra="", thickness="0.002", material=MATERIAL):
    """A transition 0.1 m long from 50 mm to 40 mm, with a 2 mm wall."""
    return (
        f"<transition><name>Transition</name><id>{uid(3)}</id><length>0.1</length>"
        f"<thickness>{thickness}</thickness><shape>conical</shape>"
        f"<foreradius>0.05</foreradius><aftradius>0.04</aftradius>{extra}{material}</transition>"
    )


def attached_tube(tag, n, name, thickness, material=MATERIAL):
    """An inner tube, coupler or engine block 0.1 m long and 20 mm in radius, 0.1 m below the top
    of its tube; `thickness` is written as given, or left out when it is `None`."""
    wall = "" if thickness is None else f"<thickness>{thickness}</thickness>"
    return (
        f"<{tag}><name>{name}</name><id>{uid(n)}</id><position type=\"top\">0.1</position>"
        f"<length>0.1</length><outerradius>0.02</outerradius>{wall}{material}</{tag}>"
    )


def lug(thickness, material=MATERIAL):
    """A launch lug 50 mm long and 5 mm in radius, 0.1 m below the top of its tube."""
    wall = "" if thickness is None else f"<thickness>{thickness}</thickness>"
    return (
        f"<launchlug><name>Lug</name><id>{uid(8)}</id><position type=\"top\">0.1</position>"
        f"<length>0.05</length><radius>0.005</radius>{wall}{material}</launchlug>"
    )


def attached_tubes(thickness):
    """One inner tube, one coupler and one lug, all with the same wall."""
    return (
        attached_tube("innertube", 4, "Inner", thickness)
        + attached_tube("tubecoupler", 13, "Coupler", thickness)
        + lug(thickness)
    )


# More kinds that name no material: a coupler, an engine block, an elliptical and a freeform fin
# set, each placed 0.1 m below the top of its tube.
MORE_NO_MATERIAL_CHILDREN = (
    attached_tube("tubecoupler", 13, "Coupler", "0.001", material="")
    + attached_tube("engineblock", 14, "Block", "0.005", material="")
    + f"<ellipticalfinset><name>Elliptical</name><id>{uid(15)}</id>"
    '<position type="top">0.1</position><fincount>3</fincount><rootchord>0.1</rootchord>'
    "<height>0.05</height><thickness>0.003</thickness><crosssection>square</crosssection>"
    "</ellipticalfinset>"
    f"<freeformfinset><name>Freeform</name><id>{uid(16)}</id>"
    '<position type="top">0.1</position><fincount>3</fincount><thickness>0.003</thickness>'
    '<crosssection>square</crosssection><finpoints><point x="0.0" y="0.0"/>'
    '<point x="0.05" y="0.05"/><point x="0.1" y="0.05"/><point x="0.1" y="0.0"/></finpoints>'
    "</freeformfinset>"
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
    "a transition that writes no thickness at all": [
        tube(),
        transition(thickness="").replace("<thickness></thickness>", ""),
    ],
    "a tube holding an inner tube, a coupler and a lug of no wall": [
        tube(children=attached_tubes("0.0"))
    ],
    "a tube holding an inner tube, a coupler and a lug that write no thickness": [
        tube(children=attached_tubes(None))
    ],
    "a mass override on a nose of no wall": [nose(overrides(mass_kg=0.1), thickness="0.0")],
    # Materials.
    "a nose and a tube that name no material, with one part of each kind inside": [
        nose(material=""),
        tube(children=NO_MATERIAL_CHILDREN, material=""),
    ],
    "a transition, and more kinds inside a tube, that name no material": [
        tube(children=MORE_NO_MATERIAL_CHILDREN),
        transition(material=""),
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


def fins(section="square", extra="", thickness="0.003", outline=("0.1", "0.05", "0.05", "0.05"), count=3):
    """Trapezoidal fins, three by default, 0.1 m below the top of their tube: by default 0.1 m root,
    and 0.05 m tip, sweep and span; `outline` is root, tip, sweep and span."""
    root, tip, sweep, span = outline
    return (
        f"<trapezoidfinset><name>Fins</name><id>{uid(5)}</id>"
        f'<position type="top">0.1</position><fincount>{count}</fincount>'
        f"<rootchord>{root}</rootchord><tipchord>{tip}</tipchord><sweeplength>{sweep}</sweeplength>"
        f"<height>{span}</height><thickness>{thickness}</thickness>"
        f"<crosssection>{section}</crosssection>{extra}{MATERIAL}</trapezoidfinset>"
    )


def fillets(radius):
    return (
        f"<filletradius>{radius}</filletradius>"
        '<filletmaterial type="bulk" density="1000.0">Probe</filletmaterial>'
    )


def placed(tag, n, name, body):
    """A part `tag` 0.1 m below the top of its tube, of the probe material."""
    return (
        f"<{tag}><name>{name}</name><id>{uid(n)}</id><position type=\"top\">0.1</position>"
        f"{body}{MATERIAL}</{tag}>"
    )


def button(end, offset, count):
    """A 10 mm rail button, or a row of them 0.1 m apart, placed from the tube's `end`."""
    return (
        f"<railbutton><name>Button</name><id>{uid(12)}</id>"
        f'<position type="{end}">{offset}</position><outerdiameter>0.01</outerdiameter>'
        "<innerdiameter>0.006</innerdiameter><height>0.008</height><baseheight>0.002</baseheight>"
        f"<flangeheight>0.002</flangeheight><instancecount>{count}</instancecount>"
        f"<instanceseparation>0.1</instanceseparation>{MATERIAL}</railbutton>"
    )


# Each probe (M2.2b2): one tube and one part, so the part's roll inertia is the probe's less the
# tube's, and the tube alone is held to OpenRocket's already.
PART_PROBES = {
    "a tube and a fin set of square section": fins(),
    "a tube and a fin set of rounded section": fins("rounded"),
    "a tube and a fin set of airfoil section": fins("airfoil"),
    "a tube and a thicker fin set of airfoil section": fins("airfoil", thickness="0.006"),
    "a tube and a fin set with fillets": fins(extra=fillets("0.005")),
    "a tube and a fin set with wider fillets": fins(extra=fillets("0.01")),
    "a tube and a fin set with a tab": fins(
        extra="<tabheight>0.01</tabheight><tablength>0.05</tablength>"
        '<tabposition relativeto="front">0.02</tabposition>'
    ),
    "a tube and a canted fin set": fins(extra="<cant>5.0</cant>"),
    "a tube and rectangular fins": fins(outline=("0.1", "0.1", "0.0", "0.05")),
    "a tube and rectangular fins of twice the chord": fins(outline=("0.2", "0.2", "0.0", "0.05")),
    "a tube and rectangular fins of twice the span": fins(outline=("0.1", "0.1", "0.0", "0.1")),
    "a tube and triangular fins": fins(outline=("0.1", "0.0", "0.0", "0.05")),
    "a tube and a single fin": fins(count=1),
    "a tube and an elliptical fin set": placed(
        "ellipticalfinset", 15, "Elliptical",
        "<fincount>3</fincount><rootchord>0.1</rootchord><height>0.05</height>"
        "<thickness>0.003</thickness><crosssection>square</crosssection>",
    ),
    "a tube and a freeform fin set": placed(
        "freeformfinset", 16, "Freeform",
        "<fincount>3</fincount><thickness>0.003</thickness><crosssection>square</crosssection>"
        '<finpoints><point x="0.0" y="0.0"/><point x="0.05" y="0.05"/>'
        '<point x="0.1" y="0.05"/><point x="0.1" y="0.0"/></finpoints>',
    ),
    "a tube and an inner tube": inner(),
    "a tube and a centering ring": placed(
        "centeringring", 6, "Ring",
        "<length>0.01</length><outerradius>0.048</outerradius><innerradius>0.02</innerradius>",
    ),
    "a tube and a bulkhead": placed(
        "bulkhead", 7, "Bulkhead", "<length>0.01</length><outerradius>0.048</outerradius>"
    ),
    "a tube and a launch lug": lug("0.001"),
    "a tube and a rail button": placed(
        "railbutton", 12, "Button",
        "<outerdiameter>0.01</outerdiameter><innerdiameter>0.006</innerdiameter>"
        "<height>0.008</height><baseheight>0.002</baseheight><flangeheight>0.002</flangeheight>"
        "<instancecount>1</instancecount>",
    ),
    **{
        f"a tube and {what} from the {end}": button(end, offset, count)
        for what, count in [("a rail button", 1), ("a row of two rail buttons", 2)]
        for end, offset in [("top", "0.1"), ("middle", "0.0"), ("bottom", "-0.1")]
        if (what, end) != ("a rail button", "top")
    },
    "a tube and a parachute": (
        f"<parachute><name>Chute</name><id>{uid(9)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.02</packedradius><diameter>0.5</diameter>"
        "<linecount>6</linecount><linelength>0.5</linelength>"
        '<material type="surface" density="0.05">Probe</material>'
        '<linematerial type="line" density="0.002">Probe</linematerial></parachute>'
    ),
    "a tube and a parachute with a mass override": (
        f"<parachute><name>Chute</name><id>{uid(9)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.02</packedradius><diameter>0.5</diameter>"
        "<linecount>6</linecount><linelength>0.5</linelength>"
        '<material type="surface" density="0.05">Probe</material>'
        '<linematerial type="line" density="0.002">Probe</linematerial>'
        f"{overrides(mass_kg=0.03)}</parachute>"
    ),
    "a tube and a parachute of no canopy, under a mass override": (
        f"<parachute><name>Chute</name><id>{uid(9)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.02</packedradius><diameter>0.0</diameter>"
        "<linecount>0</linecount><linelength>0.0</linelength>"
        '<material type="surface" density="0.05">Probe</material>'
        '<linematerial type="line" density="0.002">Probe</linematerial>'
        f"{overrides(mass_kg=0.03)}</parachute>"
    ),
    "a tube and a parachute that writes no packed size": (
        f"<parachute><name>Chute</name><id>{uid(9)}</id><position type=\"top\">0.1</position>"
        "<diameter>0.5</diameter><linecount>6</linecount><linelength>0.5</linelength>"
        '<material type="surface" density="0.05">Probe</material>'
        '<linematerial type="line" density="0.002">Probe</linematerial></parachute>'
    ),
    "a tube and a streamer": (
        f"<streamer><name>Streamer</name><id>{uid(11)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.01</packedradius>"
        "<striplength>1.0</striplength><stripwidth>0.05</stripwidth>"
        '<material type="surface" density="0.05">Probe</material></streamer>'
    ),
    "a tube and a shock cord": (
        f"<shockcord><name>Cord</name><id>{uid(10)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.01</packedradius>"
        '<cordlength>1.0</cordlength><material type="line" density="0.002">Probe</material>'
        "</shockcord>"
    ),
    "a tube and a mass component": (
        f"<masscomponent><name>Mass</name><id>{uid(17)}</id><position type=\"top\">0.1</position>"
        "<packedlength>0.05</packedlength><packedradius>0.02</packedradius><mass>0.1</mass>"
        "</masscomponent>"
    ),
}


def document(parts, stage_tags=""):
    return (
        "<?xml version='1.0' encoding='utf-8'?>\n"
        '<openrocket version="1.10" creator="hpr-sim conventions probe">'
        f"<rocket><name>Probe</name><id>{uid(98)}</id><subcomponents>"
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


def saved_materials():
    """The default materials this user has saved in OpenRocket's preferences, by key. OpenRocket
    keeps a part's default material there once someone changes it; with none saved, the defaults
    this script records are the ones a fresh install gives."""
    from java.util.prefs import Preferences

    root = Preferences.userRoot()
    if not root.nodeExists("OpenRocket/componentMaterials"):
        return {}
    node = root.node("OpenRocket/componentMaterials")
    return {str(key): str(node.get(key, "")) for key in node.keys()}


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: conventions.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    saved = saved_materials()
    if saved:
        sys.exit(f"OpenRocket has saved default materials, so its defaults are not a fresh install's: {saved}")

    probes = {}
    with tempfile.TemporaryDirectory() as scratch:
        everything = [(q, document(parts)) for q, parts in PROBES.items()]
        everything += [(q, document(parts, tags)) for q, (tags, parts) in STAGE_PROBES.items()]
        everything += [(q, document([tube(children=part)])) for q, part in PART_PROBES.items()]
        # The body's radius, for the fins' roll inertia: rectangular fins on a tube twice as wide.
        wide = tube(children=fins(outline=("0.1", "0.1", "0.0", "0.05")))
        wide = wide.replace("<radius>0.05</radius>", "<radius>0.1</radius>")
        everything += [("a wider tube and rectangular fins", document([wide]))]
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
                "saved_default_materials": saved,
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
