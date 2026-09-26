"""Where OpenRocket 24.12 puts the tubes of a clustered inner tube, and what it weighs.

An inner tube in a `.ork` may be a cluster: `clusterconfiguration` names a pattern (`3-ring`,
`4-square` and so on), `clusterscale` spreads it and `clusterrotation` turns it. No published
document gives the patterns' geometry, so this script asks OpenRocket (M1.9b): it records each
pattern's own points as OpenRocket lists them, then has OpenRocket read small probe designs, each
one clustered tube in a body tube, and records where it puts every tube (the instance offsets), the
separation it derives, the structure's mass properties and its per-part breakdown. hpr's tests in
`hpr_io::ork` read the same documents and hold hpr to the answers.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API `javap` prints for the jar (`ClusterConfiguration`'s
`CONFIGURATIONS`, `getXMLName`, `getClusterCount`, `getPoints`; `InnerTube`'s
`getClusterSeparation`, `getClusterRotation`, `getInstanceOffsets`). Each probe is saved once, to a
throwaway, before it is read, as `mass.py` does.

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/clusters.py \\
        validation/fixtures/ork/openrocket-clusters.json

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

# Provenance written into the fixture (docs/VALIDATION.md). Update when regenerating.
GENERATED = "2026-09-25"

# An engine block, 10 mm long, filling the inner tube's bore, at its top.
BLOCK = (
    f"<engineblock><name>Block</name><id>{conventions.uid(5)}</id>"
    '<position type="top">0.0</position><length>0.01</length>'
    "<outerradius>0.019</outerradius><thickness>0.005</thickness>"
    f"{conventions.MATERIAL}</engineblock>"
)


# A centering ring, 10 mm thick, filling the body tube's bore, with an automatic bore, beside the
# inner tube.
RING = (
    f"<centeringring><name>Ring</name><id>{conventions.uid(6)}</id>"
    '<position type="top">0.15</position><length>0.01</length>'
    "<outerradius>auto</outerradius><innerradius>auto</innerradius>"
    f"{conventions.MATERIAL}</centeringring>"
)


def clustered(pattern, scale="1.0", rotation="0.0", extra="", children=""):
    """The conventions probe's inner tube, clustered."""
    inside = f"<subcomponents>{children}</subcomponents>" if children else ""
    return conventions.inner(
        extra=f"<clusterconfiguration>{pattern}</clusterconfiguration>"
        f"<clusterscale>{scale}</clusterscale><clusterrotation>{rotation}</clusterrotation>"
        f"{extra}{inside}"
    )


def probes(patterns):
    """Every pattern as written, and the questions about scale, rotation, offset and contents."""
    found = {f"{p} at scale 1": clustered(p) for p in patterns}
    found.update(
        {
            "a 3-ring at scale 1.5": clustered("3-ring", scale="1.5"),
            "a 3-ring turned 30": clustered("3-ring", rotation="30.0"),
            "a 4-ring at scale 1.25 turned 10": clustered("4-ring", "1.25", "10.0"),
            "a pattern OpenRocket has no name for": clustered("4-square"),
            "an unclustered tube 10 mm off the axis at 30": conventions.inner(
                extra="<radialposition>0.01</radialposition><radialdirection>30.0</radialdirection>"
            ),
            "a 2-row 10 mm off the axis at 30": clustered(
                "double",
                extra="<radialposition>0.01</radialposition><radialdirection>30.0</radialdirection>",
            ),
            "a 3-ring 10 mm off the axis at 30, turned 20": clustered(
                "3-ring",
                rotation="20.0",
                extra="<radialposition>0.01</radialposition><radialdirection>30.0</radialdirection>",
            ),
            "a 3-ring with an engine block": clustered("3-ring", children=BLOCK),
            "a 3-ring beside a ring with an automatic bore": clustered("3-ring") + RING,
            "a 3-ring with its mass overridden": clustered(
                "3-ring", extra=conventions.overrides(mass_kg="0.03")
            ),
        }
    )
    return found


def offsets(component, found):
    """Each inner tube's cluster as OpenRocket resolves it, by id."""
    if str(component.getClass().getSimpleName()) == "InnerTube":
        found[str(component.getID())] = {
            "pattern": str(component.getClusterConfiguration().getXMLName()),
            "count": int(component.getInstanceCount()),
            "separation_m": float(component.getClusterSeparation()),
            "rotation": float(component.getClusterRotation()),
            "instance_offsets_m": [
                [float(c.x), float(c.y), float(c.z)] for c in component.getInstanceOffsets()
            ],
        }
    for child in component.getChildren():
        offsets(child, found)


def measure(text, scratch, name):
    path = Path(scratch) / f"{name}.ork"
    path.write_text(text, encoding="utf-8")
    document_ = automatic_radius.load(path)
    automatic_radius.saved_radii(document_)
    found = {}
    offsets(document_.getRocket(), found)
    parts, skipped = mass.parts(document_)
    return {
        "document": text,
        "tubes": found,
        "structure": mass.structure(document_),
        "parts": parts,
        "parts_skipped": skipped,
    }


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: clusters.py OUTPUT.json")
    output = Path(sys.argv[1])
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    import jpype
    from info.openrocket.core.rocketcomponent import ClusterConfiguration
    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    patterns = {
        str(c.getXMLName()): {
            "count": int(c.getClusterCount()),
            "points": [float(v) for v in c.getPoints()],
        }
        for c in ClusterConfiguration.CONFIGURATIONS
    }
    measured = {}
    with tempfile.TemporaryDirectory() as scratch:
        for k, (question, part) in enumerate(probes(patterns).items()):
            text = conventions.document([conventions.tube(children=part)])
            measured[question] = measure(text, scratch, f"probe-{k}")

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/openrocket/clusters.py",
                "command": " ".join(
                    ["refs/venv/bin/python", "validation/oracles/openrocket/clusters.py"]
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
                        ("clusters.py", sys.modules[__name__]),
                        ("conventions.py", conventions),
                        ("automatic_radius.py", automatic_radius),
                        ("mass.py", mass),
                    ]
                },
                "patterns": patterns,
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
