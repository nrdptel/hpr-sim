"""How OpenRocket 24.12 reads each part in the `.orc` parts database it ships (M5.5a).

OpenRocket's jar carries the `openrocket-database` files (Apache-2.0) that `refs.lock.toml` pins,
byte for byte: the script checks each against the copy bundled in
`crates/hpr-io/data/openrocket-database/` before reading it. Each file is read by OpenRocket's
own preset loader, `OpenRocketComponentLoader.load`, and every value of every part it returns is
recorded: the part's kind, maker and part number, each number in SI as OpenRocket holds it, and
each material's name, kind and density. hpr's test `crates/hpr-io/tests/orc_openrocket.rs` reads
the same files and holds hpr's reading to these, part by part.

It also reads small probe catalogues written here, each asking one question about the format:
every unit of density, length and mass the database's `docs/TechnicalInfo.md` lists, values with
no units, and what OpenRocket does with a part or a file it can't read. Each probe's text is
recorded with OpenRocket's parts, or its refusal, and the same test reads each one.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API `javap` prints for the jar (`ComponentPreset`, its
`ORDERED_KEY_LIST` of `TypedKey`s, `Material`, `Manufacturer`).

OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/orc_presets.py \\
        crates/hpr-io/tests/fixtures/orc/openrocket-presets.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import json
import sys
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import automatic_radius  # noqa: E402 - the JVM start and the jar's path

# Provenance written into the fixture (docs/VALIDATION.md). Update when regenerating.
GENERATED = "2026-10-01"

BUNDLED = Path("crates/hpr-io/data/openrocket-database")
IN_JAR = "datafiles/components/database/"


def probe(materials, parts):
    """A probe catalogue: `materials` and `parts` (XML) in an otherwise minimal file."""
    return (
        '<?xml version="1.0" encoding="UTF-8"?>\n<OpenRocketComponent><Version>0.1</Version>'
        f"<Materials>{materials}</Materials><Components>{parts}</Components></OpenRocketComponent>"
    )


def material(name, kind, unit, density="3"):
    """A material element; `unit` None leaves `UnitsOfMeasure` out."""
    units = "" if unit is None else f' UnitsOfMeasure="{unit}"'
    return (
        f"<Material{units}><Name>{name}</Name><Density>{density}</Density>"
        f"<Type>{kind}</Type></Material>"
    )


def tube(number, material_name="M", kind="BULK", length='<Length Unit="in">10</Length>', extra=""):
    """A body tube 1 in inside, 1.1 in outside, in `material_name` of `kind`."""
    return (
        f"<BodyTube><Manufacturer>Probe</Manufacturer><PartNumber>{number}</PartNumber>"
        f'<Description>probe</Description><Material Type="{kind}">{material_name}</Material>'
        '<InsideDiameter Unit="in">1</InsideDiameter><OutsideDiameter Unit="in">1.1</OutsideDiameter>'
        f"{length}{extra}</BodyTube>"
    )


def chute(number, canopy="S", line=None, extra=""):
    """A parachute of canopy `canopy`, with shroud lines of `line` if given."""
    lines = "" if line is None else f'<LineMaterial Type="LINE">{line}</LineMaterial>'
    return (
        f"<Parachute><Manufacturer>Probe</Manufacturer><PartNumber>{number}</PartNumber>"
        f'<Description>probe</Description><Material Type="SURFACE">{canopy}</Material>'
        '<Diameter Unit="in">12</Diameter><Sides>6</Sides><LineCount>6</LineCount>'
        f'<LineLength Unit="in">12</LineLength>{lines}{extra}</Parachute>'
    )


SURFACE = material("S", "SURFACE", "kg/m2")


def probes():
    """Probe catalogues, each asking OpenRocket one question about reading the format: every
    density, length and mass unit the format documents, values with no units, and what it does
    with a part or file it can't read. Each is (name, text)."""
    found = []
    for unit in ["kg/m3", "g/cm3", "kg/dm3", "lb/ft3"]:
        found.append((f"bulk {unit}", probe(material("M", "BULK", unit), tube(unit))))
    for unit in ["kg/m2", "g/m2", "g/cm2", "oz/in2", "oz/ft2", "lb/ft2"]:
        found.append((f"surface {unit}", probe(material("S", "SURFACE", unit), chute(unit))))
    for unit in ["kg/m", "g/m", "g/cm", "oz/ft", "oz/in"]:
        materials = SURFACE + material("L", "LINE", unit)
        found.append((f"line {unit}", probe(materials, chute(unit, line="L"))))
    bulk = material("M", "BULK", "kg/m3")
    for unit in ["m", "cm", "mm", "in", "ft", "in/64", "furlong"]:
        length = f'<Length Unit="{unit}">3</Length>'
        found.append((f"length {unit}", probe(bulk, tube(unit, length=length))))
    for unit in ["kg", "g", "oz", "lb"]:
        found.append((f"mass {unit}", probe(SURFACE, chute(unit, extra=f'<Mass Unit="{unit}">3</Mass>'))))
    found.append(("no units", probe(material("M", "BULK", None, "700"), tube("plain", length="<Length>0.5</Length>"))))
    found.append(("wrong kind", probe(bulk, tube("wrong kind", kind="SURFACE"))))
    found.append(("unknown field", probe(bulk, tube("unknown field", extra="<Colour>red</Colour>"))))
    found.append(("unknown part", probe(bulk, "<Widget><Manufacturer>Probe</Manufacturer><PartNumber>w</PartNumber></Widget>" + tube("after"))))
    found.append(("no length", probe(bulk, tube("no length", length=""))))
    nose = (
        "<NoseCone><Manufacturer>Probe</Manufacturer><PartNumber>n</PartNumber><Description>probe"
        '</Description><Material Type="BULK">M</Material><Shape>BULLET</Shape>'
        "<OutsideDiameter>0.04</OutsideDiameter><ShoulderDiameter>0</ShoulderDiameter>"
        "<ShoulderLength>0</ShoulderLength><Length>0.1</Length></NoseCone>"
    )
    found.append(("unknown shape", probe(bulk, nose)))
    return found


def reading(data, name):
    """OpenRocket's reading of a catalogue: its parts, or its refusal's first line."""
    from info.openrocket.core.preset import ComponentPreset
    from info.openrocket.core.preset.xml import OpenRocketComponentLoader
    from java.io import ByteArrayInputStream

    try:
        presets = OpenRocketComponentLoader().load(ByteArrayInputStream(data), name)
    except Exception as error:  # noqa: BLE001 - the refusal is the measurement
        return None, str(error).splitlines()[0]
    parts = []
    for preset in presets:
        # The part's kind is not among the ordered keys; `getType` gives it.
        values = {"Type": str(preset.getType().name())}
        for key in ComponentPreset.ORDERED_KEY_LIST:
            if preset.has(key):
                values[str(key.getName())] = value(preset.get(key))
        parts.append(values)
    return parts, None


def value(raw):
    """A recorded value: a number, a flag, text, or a material as name, kind and density."""
    if hasattr(raw, "getDensity"):
        return {
            "name": str(raw.getName()),
            "kind": str(raw.getType().name()),
            "density": float(raw.getDensity()),
        }
    if hasattr(raw, "getDisplayName"):
        return str(raw.getDisplayName())
    text = str(raw.getClass().getSimpleName())
    if text == "Double":
        return float(raw)
    if text == "Integer":
        return int(raw)
    if text == "Boolean":
        return bool(raw)
    if hasattr(raw, "name") and callable(raw.name):
        return str(raw.name())
    return str(raw)


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
            parts, error = reading(data, name)
            if error is not None:
                sys.exit(f"{name}: OpenRocket refused it: {error}")
            files.append(
                {"file": name, "sha256": hashlib.sha256(data).hexdigest(), "parts": parts}
            )

    head = {
        "source": "validation/oracles/openrocket/orc_presets.py",
        "generated": GENERATED,
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
        "java": str(System.getProperty("java.version")),
    }
    probe_readings = []
    for name, text in probes():
        parts, error = reading(text.encode("utf-8"), name)
        probe_readings.append({"probe": name, "text": text, "parts": parts, "error": error})

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
    rows = [f"    {json.dumps(entry, ensure_ascii=False)}" for entry in probe_readings]
    lines.append(",\n".join(rows))
    lines.append("  ]")
    lines.append("}")
    Path(sys.argv[1]).write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
