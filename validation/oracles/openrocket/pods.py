"""Where OpenRocket 24.12 puts the pods of a pod set, and what it weighs.

A `.ork` pod set writes its pods' count (`instancecount`), their distance from the axis
(`radiusoffset`, with a `method`), their roll angle (`angleoffset`, degrees, with a `method`) and
its position along its tube. No published document says what a `radiusoffset` of each method is
measured from, so this script asks OpenRocket (M1.13b): it has OpenRocket read small probe
designs, each a pod set on the conventions probe's body tube (50 mm in radius) holding a short nose
and a tube 10 mm in radius (two with a wider tube between them, one of those with the nose's radius
stated and another narrow tube first, so that the widest part is neither the first part, the first
tube nor the last, and one placed from the tube's bottom), and records every pod set's
resolved radius, angle and instance offsets, where the parts inside a pod sit (their component
locations), and the structure's mass properties with its per-part breakdown. hpr's tests in
`hpr_validate` read the same documents and hold hpr to them.

M1.13b2 adds the pods of no length OpenRocket's own example draws to hang winglets off the axis:
a tube of no length and no wall (a "phantom body") holding fins, four ways, or a launch lug, two
ways; and a pod set that holds nothing at all, with and without a mass override.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The methods used
are the public API `javap` prints for the jar (`PodSet`'s `getInstanceCount`, `getRadiusOffset`,
`getRadiusMethod`, `getAngleOffset`, `getInstanceOffsets`; `RocketComponent`'s
`getComponentLocations`). Each probe is saved once, to a throwaway, before it is read, as
`mass.py` does.

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/pods.py \\
        validation/fixtures/ork/openrocket-pods.json

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
import conventions  # noqa: E402 - the probe parts, written the same way
import mass  # noqa: E402 - the structure and the per-part breakdown, asked the same way

GENERATED = "2026-09-27"

POD_NOSE = (
    f"<nosecone><name>Pod nose</name><id>{conventions.uid(11)}</id><length>0.05</length>"
    "<thickness>0.001</thickness><shape>conical</shape><aftradius>auto</aftradius>"
    f"{conventions.MATERIAL}</nosecone>"
)
POD_TUBE = (
    f"<bodytube><name>Pod tube</name><id>{conventions.uid(12)}</id><length>0.2</length>"
    f"<thickness>0.001</thickness><radius>0.01</radius>{conventions.MATERIAL}</bodytube>"
)

POD_WIDE_TUBE = (
    f"<bodytube><name>Pod wide tube</name><id>{conventions.uid(13)}</id><length>0.1</length>"
    f"<thickness>0.001</thickness><radius>0.015</radius>{conventions.MATERIAL}</bodytube>"
)
POD_STATED_NOSE = POD_NOSE.replace("<aftradius>auto</aftradius>", "<aftradius>0.01</aftradius>")
POD_FIRST_TUBE = (
    f"<bodytube><name>Pod first tube</name><id>{conventions.uid(14)}</id><length>0.05</length>"
    f"<thickness>0.001</thickness><radius>0.01</radius>{conventions.MATERIAL}</bodytube>"
)


def phantom(children, radius="0.0"):
    """A tube of no length, as OpenRocket's own example writes the pod that hangs its winglets
    (M1.13b2): no wall, 0 m in radius unless `radius` says otherwise, holding `children`."""
    return (
        f"<bodytube><name>(phantom body)</name><id>{conventions.uid(15)}</id><length>0.0</length>"
        f"<thickness>0.0</thickness><radius>{radius}</radius>{conventions.MATERIAL}"
        f"<subcomponents>{children}</subcomponents></bodytube>"
    )


def pod_fins(count, angle):
    """Trapezoidal fins at the aft end of their tube, the way the example's winglets are written:
    `count` of them, the first at `angle` degrees; 30 mm root, 15 mm tip, 10 mm sweep and 20 mm
    span."""
    return (
        f"<trapezoidfinset><name>Pod fins</name><id>{conventions.uid(16)}</id>"
        f"<instancecount>{count}</instancecount><fincount>{count}</fincount>"
        '<radiusoffset method="surface">0.0</radiusoffset>'
        f'<angleoffset method="relative">{angle}</angleoffset><rotation>{angle}</rotation>'
        '<axialoffset method="bottom">0.0</axialoffset><position type="bottom">0.0</position>'
        "<rootchord>0.03</rootchord><tipchord>0.015</tipchord><sweeplength>0.01</sweeplength>"
        "<height>0.02</height><thickness>0.003</thickness><crosssection>square</crosssection>"
        f"{conventions.MATERIAL}</trapezoidfinset>"
    )


def pod_lug(angle):
    """A launch lug 40 mm long and 3 mm in radius, centred on its tube, at `angle` degrees."""
    return (
        f"<launchlug><name>Pod lug</name><id>{conventions.uid(17)}</id>"
        f'<instancecount>1</instancecount><angleoffset method="relative">{angle}</angleoffset>'
        f"<radialdirection>{angle}</radialdirection>"
        '<axialoffset method="middle">0.0</axialoffset><position type="middle">0.0</position>'
        "<length>0.04</length><radius>0.003</radius><thickness>0.0005</thickness>"
        f"{conventions.MATERIAL}</launchlug>"
    )


def pod_set(
    count,
    radius,
    radius_method,
    angle,
    angle_method="relative",
    axial="0.1",
    more="",
    nose=POD_NOSE,
    axial_method="top",
    parts=None,
    extra="",
):
    """A pod set of `count` pods, each the nose and tube above unless `parts` gives the pod, placed
    as the arguments say."""
    inside = f"{nose}{more}{POD_TUBE}" if parts is None else parts
    return (
        f"<podset><name>Pods</name><id>{conventions.uid(10)}</id>"
        f"<instancecount>{count}</instancecount>"
        f'<radiusoffset method="{radius_method}">{radius}</radiusoffset>'
        f'<angleoffset method="{angle_method}">{angle}</angleoffset>'
        f'<axialoffset method="{axial_method}">{axial}</axialoffset>{extra}'
        f"<subcomponents>{inside}</subcomponents></podset>"
    )


PROBES = {
    "two pods, relative 0.02, at 30": pod_set(2, "0.02", "relative", "30.0"),
    "one pod, relative 0.0, at 0": pod_set(1, "0.0", "relative", "0.0"),
    "three pods, relative 0.005, at -45": pod_set(3, "0.005", "relative", "-45.0"),
    "two pods, surface 0.02, at 30": pod_set(2, "0.02", "surface", "30.0"),
    "two pods, free 0.08, at 30": pod_set(2, "0.08", "free", "30.0"),
    "two pods, relative 0.02, at 30 fixed": pod_set(2, "0.02", "relative", "30.0", "fixed"),
    "two pods, relative 0.02, at 30, a wider tube between": pod_set(
        2, "0.02", "relative", "30.0", more=POD_WIDE_TUBE
    ),
    "two pods, relative 0.02, at 30, a stated nose, a tube, a wider tube, a tube": pod_set(
        2, "0.02", "relative", "30.0", more=POD_FIRST_TUBE + POD_WIDE_TUBE, nose=POD_STATED_NOSE
    ),
    "two pods, relative 0.02, at 30 mirror_xy": pod_set(2, "0.02", "relative", "30.0", "mirror_xy"),
    "two pods, relative 0.02, at 30, bottom -0.01": pod_set(
        2, "0.02", "relative", "30.0", axial="-0.01", axial_method="bottom"
    ),
    "one pod of no length, relative 0.03, bottom 0.0, two fins at 90": pod_set(
        1, "0.03", "relative", "0.0", axial="0.0", axial_method="bottom",
        parts=phantom(pod_fins(2, "90.0")),
    ),
    "two pods of no length, relative 0.02, at 45, three fins": pod_set(
        2, "0.02", "relative", "45.0", parts=phantom(pod_fins(3, "0.0"))
    ),
    "one pod of no length 0.01 in radius, relative 0.02, two fins at 90": pod_set(
        1, "0.02", "relative", "0.0", parts=phantom(pod_fins(2, "90.0"), radius="0.01")
    ),
    "two pods of no length 0.01 in radius, relative 0.02, at 30, three fins at 20": pod_set(
        2, "0.02", "relative", "30.0", parts=phantom(pod_fins(3, "20.0"), radius="0.01")
    ),
    "one pod of no length, relative 0.012, middle 0.0, a launch lug at 180": pod_set(
        1, "0.012", "relative", "0.0", axial="0.0", axial_method="middle",
        parts=phantom(pod_lug("180.0")),
    ),
    "two pods of no length, relative 0.012, at 30, a launch lug at 0": pod_set(
        2, "0.012", "relative", "30.0", parts=phantom(pod_lug("0.0"))
    ),
    "an empty pod set, relative 0.004": pod_set(1, "0.004", "relative", "0.0", parts=""),
    "an empty pod set, relative 0.004, its mass overridden to 0.1 kg": pod_set(
        1, "0.004", "relative", "0.0", parts="",
        extra=conventions.overrides(mass_kg="0.1", children_mass="true"),
    ),
}


def placed(component, found):
    """Every pod set's resolved placement and every part's locations, by id."""
    name = str(component.getClass().getSimpleName())
    entry = {
        "class": name,
        "locations_m": [[float(c.x), float(c.y), float(c.z)] for c in component.getComponentLocations()],
    }
    if name == "PodSet":
        entry.update(
            {
                "count": int(component.getInstanceCount()),
                "radius_offset_m": float(component.getRadiusOffset()),
                "radius_method": str(component.getRadiusMethod()),
                "angle_offset_rad": float(component.getAngleOffset()),
                "instance_offsets_m": [
                    [float(c.x), float(c.y), float(c.z)] for c in component.getInstanceOffsets()
                ],
            }
        )
    found[str(component.getID())] = entry
    for child in component.getChildren():
        placed(child, found)


def measure(text, scratch, name):
    path = Path(scratch) / f"{name}.ork"
    path.write_text(text, encoding="utf-8")
    document_ = automatic_radius.load(path)
    automatic_radius.saved_radii(document_)
    found = {}
    placed(document_.getRocket(), found)
    parts, skipped = mass.parts(document_)
    return {
        "document": text,
        "components": found,
        "structure": mass.structure(document_),
        "parts": parts,
        "parts_skipped": skipped,
    }


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: pods.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    measured = {}
    with tempfile.TemporaryDirectory() as scratch:
        for k, (question, part) in enumerate(PROBES.items()):
            text = conventions.document([conventions.nose(), conventions.tube(children=part)])
            measured[question] = measure(text, scratch, f"probe-{k}")

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/pods.py",
                "command": " ".join(
                    ["refs/venv/bin/python", "validation/oracles/openrocket/pods.py"]
                    + sys.argv[1:]
                ),
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
                "java": str(System.getProperty("java.version")),
                "jpype": jpype.__version__,
                "generated": GENERATED,
                "inputs_sha256": {
                    name: hashlib.sha256(Path(module.__file__).read_bytes()).hexdigest()
                    for name, module in [
                        ("pods.py", sys.modules[__name__]),
                        ("conventions.py", conventions),
                        ("automatic_radius.py", automatic_radius),
                        ("mass.py", mass),
                    ]
                },
                "probes": measured,
            },
            indent=1,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
