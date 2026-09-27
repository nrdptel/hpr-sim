"""Where OpenRocket 24.12 puts the pods of a pod set, and what it weighs.

A `.ork` pod set writes its pods' count (`instancecount`), their distance from the axis
(`radiusoffset`, with a `method`), their roll angle (`angleoffset`, degrees, with a `method`) and
its position along its tube. No published document says what a `radiusoffset` of each method is
measured from, so this script asks OpenRocket (M1.13b): it has OpenRocket read small probe
designs, each a pod set on the conventions probe's body tube (50 mm in radius) holding a short nose
and a tube 10 mm in radius (one with a wider tube between them), and records every pod set's
resolved radius, angle and instance offsets, where the parts inside a pod sit (their component
locations), and the structure's mass properties with its per-part breakdown. hpr's tests in
`hpr_validate` read the same documents and hold hpr to them.

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


def pod_set(count, radius, radius_method, angle, angle_method="relative", axial="0.1", more=""):
    """A pod set of `count` pods, each the nose and tube above, placed as the arguments say."""
    return (
        f"<podset><name>Pods</name><id>{conventions.uid(10)}</id>"
        f"<instancecount>{count}</instancecount>"
        f'<radiusoffset method="{radius_method}">{radius}</radiusoffset>'
        f'<angleoffset method="{angle_method}">{angle}</angleoffset>'
        f'<axialoffset method="top">{axial}</axialoffset>'
        f"<subcomponents>{POD_NOSE}{more}{POD_TUBE}</subcomponents></podset>"
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
    "two pods, relative 0.02, at 30 mirror_xy": pod_set(2, "0.02", "relative", "30.0", "mirror_xy"),
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
