"""What OpenRocket 24.12 builds from each part in the `.orc` parts database it ships (M5.5b).

`orc_presets.py` records how OpenRocket reads each part. This script records what it makes of
one: each preset is applied to a new component of its kind (`RocketComponent.loadPreset`), as
OpenRocket's part chooser does, and the component's mass and centre of mass are recorded with the
dimensions the file leaves unsaid and OpenRocket chooses: a nose cone's or transition's shape
parameter, its wall, its shoulders' walls and caps, whether a transition is clipped, and a
parachute's drag coefficient. hpr's test `crates/hpr/tests/catalog_openrocket.rs` builds the same
parts with the builder (`hpr::rocket`) and holds each one's mass to these.

The bundled files are read from `crates/hpr-io/data/openrocket-database/` after checking each is
the jar's copy byte for byte. A few probe catalogues are applied too, each a part of a kind or
shape the bundled files lack (a power-series nose cone), so its unsaid dimensions are on record.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API `javap` prints for the jar (`OpenRocketComponentLoader`,
`ComponentPreset`, the component classes and their getters).

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/orc_built.py \\
        crates/hpr/tests/fixtures/orc/openrocket-built.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import sys
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start and the jar's path
from orc_presets import BUNDLED, IN_JAR, probe  # noqa: E402 - the bundled files and probe text

# Provenance written into the fixture (docs/VALIDATION.md). Update when regenerating.
GENERATED = "2026-10-01"

# What each kind records beside its mass and centre of mass: the getters, by the name the record
# gives them. Only what the file leaves unsaid; what it states, `orc_presets.py` records.
UNSAID = {
    "NOSE_CONE": [
        ("ShapeParameter", "getShapeParameter"),
        ("Thickness", "getThickness"),
        ("Filled", "isFilled"),
        ("AftShoulderThickness", "getAftShoulderThickness"),
        ("AftShoulderCapped", "isAftShoulderCapped"),
    ],
    "TRANSITION": [
        ("ShapeParameter", "getShapeParameter"),
        ("Thickness", "getThickness"),
        ("Filled", "isFilled"),
        ("Clipped", "isClipped"),
        ("ForeShoulderThickness", "getForeShoulderThickness"),
        ("ForeShoulderCapped", "isForeShoulderCapped"),
        ("AftShoulderThickness", "getAftShoulderThickness"),
        ("AftShoulderCapped", "isAftShoulderCapped"),
    ],
    "PARACHUTE": [("CD", "getCD")],
}

# A probe catalogue's one material: bulk, 1,000 kg/m³.
MATERIAL = (
    '<Material UnitsOfMeasure="kg/m3"><Name>M</Name><Density>1000</Density>'
    "<Type>BULK</Type></Material>"
)


def probe_nose(shape):
    """A filled nose cone of `shape`, 50 mm across and 200 mm long, with a 40 mm shoulder."""
    return probe(
        MATERIAL,
        f"<NoseCone><Manufacturer>Probe</Manufacturer><PartNumber>{shape.lower()}</PartNumber>"
        '<Description>probe</Description><Material Type="BULK">M</Material>'
        f"<Shape>{shape}</Shape><Filled>true</Filled>"
        '<OutsideDiameter Unit="m">0.05</OutsideDiameter>'
        '<ShoulderDiameter Unit="m">0.048</ShoulderDiameter>'
        '<ShoulderLength Unit="m">0.04</ShoulderLength>'
        '<Length Unit="m">0.2</Length></NoseCone>',
    )


def probe_transition(shape):
    """A filled transition of `shape` from 50 mm to 30 mm over 100 mm, shouldered both ends."""
    return probe(
        MATERIAL,
        f"<Transition><Manufacturer>Probe</Manufacturer><PartNumber>{shape.lower()}</PartNumber>"
        '<Description>probe</Description><Material Type="BULK">M</Material>'
        f"<Shape>{shape}</Shape><Filled>true</Filled>"
        '<ForeOutsideDiameter Unit="m">0.05</ForeOutsideDiameter>'
        '<ForeShoulderDiameter Unit="m">0.048</ForeShoulderDiameter>'
        '<ForeShoulderLength Unit="m">0.03</ForeShoulderLength>'
        '<AftOutsideDiameter Unit="m">0.03</AftOutsideDiameter>'
        '<AftShoulderDiameter Unit="m">0.028</AftShoulderDiameter>'
        '<AftShoulderLength Unit="m">0.02</AftShoulderLength>'
        '<Length Unit="m">0.1</Length></Transition>',
    )


# Probe catalogues: a part of a kind or shape the bundled files lack (no power-series nose, no
# Haack, parabolic or power-series transition), so OpenRocket's unsaid parameter and clipping for
# it are on record.
PROBES = [("power-series nose cone", probe_nose("POWER"))] + [
    (f"{shape.lower()} transition", probe_transition(shape))
    for shape in ["HAACK", "PARABOLIC", "POWER", "ELLIPSOID", "OGIVE"]
]


def component(kind):
    """A new OpenRocket component of the preset kind `kind`."""
    from info.openrocket.core import rocketcomponent as rc

    return {
        "BODY_TUBE": rc.BodyTube,
        "NOSE_CONE": rc.NoseCone,
        "TRANSITION": rc.Transition,
        "TUBE_COUPLER": rc.TubeCoupler,
        "BULK_HEAD": rc.Bulkhead,
        "CENTERING_RING": rc.CenteringRing,
        "ENGINE_BLOCK": rc.EngineBlock,
        "LAUNCH_LUG": rc.LaunchLug,
        "PARACHUTE": rc.Parachute,
        "STREAMER": rc.Streamer,
    }[kind]()


def built(data, name):
    """Each part of a catalogue as OpenRocket builds it, in the order its loader returns them."""
    from info.openrocket.core.preset.xml import OpenRocketComponentLoader
    from java.io import ByteArrayInputStream

    parts = []
    for preset in OpenRocketComponentLoader().load(ByteArrayInputStream(data), name):
        kind = str(preset.getType().name())
        part = component(kind)
        part.loadPreset(preset)
        if part.isCGOverridden():
            sys.exit(f"{name}: {preset.getPartNo()}: the preset overrode its centre")
        cg = part.getComponentCG()
        record = {
            "Type": kind,
            "PartNo": str(preset.getPartNo()),
            "mass_kg": float(part.getComponentMass()),
            # Metres aft of the component's fore end (OpenRocket's x), its weight the mass.
            "cg_m": float(cg.x),
        }
        # A part whose stated mass OpenRocket doesn't give it through its density (a parachute's
        # or a streamer's) has it as an override instead: the mass the component weighs in a
        # rocket, where `getComponentMass` stays its material's.
        if part.isMassOverridden():
            record["override_mass_kg"] = float(part.getOverrideMass())
        if abs(float(cg.weight) - record["mass_kg"]) > 1e-15 * max(1.0, record["mass_kg"]):
            sys.exit(f"{name}: {record['PartNo']}: the centre's weight is not the mass")
        for key, getter in UNSAID.get(kind, []):
            raw = getattr(part, getter)()
            record[key] = bool(raw) if getter.startswith("is") else float(raw)
        parts.append(record)
    return parts


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    automatic_radius.start()

    from info.openrocket.core.util import BuildProperties
    from java.lang import System

    files = []
    with zipfile.ZipFile(automatic_radius.JAR) as jar:
        names = sorted(
            entry[len(IN_JAR) :]
            for entry in jar.namelist()
            if entry.startswith(IN_JAR) and entry.lower().endswith(".orc")
        )
        for name in names:
            data = jar.read(IN_JAR + name)
            if (BUNDLED / name).read_bytes() != data:
                sys.exit(f"{name}: the jar's copy is not the bundled one")
            files.append(
                {
                    "file": name,
                    "sha256": hashlib.sha256(data).hexdigest(),
                    "parts": built(data, name),
                }
            )
    probes = [
        {"probe": label, "text": text, "parts": built(text.encode("utf-8"), label)}
        for label, text in PROBES
    ]

    head = {
        "source": "validation/oracles/openrocket/orc_built.py",
        "generated": GENERATED,
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
        "java": str(System.getProperty("java.version")),
    }
    # One part to a line, so a regenerated fixture's diff names the parts that moved.
    lines = ["{"]
    for key, item in head.items():
        lines.append(f"  {json.dumps(key)}: {json.dumps(item)},")
    lines.append('  "files": [')
    for index, entry in enumerate(files):
        lines.append("    {")
        lines.append(f'      "file": {json.dumps(entry["file"])},')
        lines.append(f'      "sha256": {json.dumps(entry["sha256"])},')
        lines.append('      "parts": [')
        rows = [f"        {json.dumps(part, ensure_ascii=False)}" for part in entry["parts"]]
        lines.append(",\n".join(rows))
        lines.append("      ]")
        lines.append("    }" + ("," if index + 1 < len(files) else ""))
    lines.append("  ],")
    lines.append('  "probes": [')
    rows = [f"    {json.dumps(entry, ensure_ascii=False)}" for entry in probes]
    lines.append(",\n".join(rows))
    lines.append("  ]")
    lines.append("}")
    Path(sys.argv[1]).parent.mkdir(parents=True, exist_ok=True)
    Path(sys.argv[1]).write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
