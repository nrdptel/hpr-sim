"""The key geometry RocketSerializer reads from a `.ork` design, with OpenRocket's own beside it.

RocketSerializer (MIT, RocketPy-Team/RocketSerializer) turns an OpenRocket design into the
parameters RocketPy flies: the nose cone, the fin sets, the transitions and the body radius. It is
a second reader of the same files hpr reads, written by other people, so where the two agree the
reading is not one program's opinion. This script runs RocketSerializer's own extractors on each
design and writes what they return, untouched.

A second reader can be wrong too. So for every component RocketSerializer reports, the script also
asks OpenRocket 24.12 itself, loaded on the same file, for the same numbers: the nose cone's length
and base radius, each transition's length and radii, each fin set's count, chords, span, sweep and
cant, and the station of every one of them, measured aft of the nose tip. `cargo xtask ork` holds
hpr's reading to RocketSerializer's, and where the two differ, OpenRocket's number says which one
read the file as OpenRocket does.

The extractors are called one by one, as RocketSerializer's `ork_extractor` calls them, rather
than through its `ork2json` command, which also needs a stored simulation to read masses from and
writes files beside its input. RocketSerializer gives each extractor two things: the XML, parsed by
BeautifulSoup, and OpenRocket's loaded document, which it walks for stations and transition radii.
Both are built here the same way: the JVM is started as
`validation/oracles/openrocket/automatic_radius.py` starts it, and OpenRocket is run, never read.

One file needs a step before OpenRocket opens it: Loft's `demo-stable.ork` begins with a comment
long enough that OpenRocket 24.12 no longer recognises the document ("Unsupported or corrupt
file"). The comment is removed from the copy OpenRocket reads, and the file's record says so.

This runs in a Python 3.11 environment of its own, every package pinned in `requirements.txt`
beside it and installed without their declared dependencies, so that RocketSerializer's `orhelper`
(GPL-2.0, and pinning JPype below 1.5) is never installed. Set it up once, from the repository root:

    uv venv -p 3.11 refs/venv-rs
    uv pip install -p refs/venv-rs --no-deps -r validation/oracles/rocketserializer/requirements.txt

Then, with Java 17 (see `automatic_radius.py`), for the public designs the repository commits:

    refs/venv-rs/bin/python validation/oracles/rocketserializer/geometry.py \\
        validation/fixtures/ork/rocketserializer-loft-demo.json validation/fixtures/ork/loft-demo

and for the reference library, whose contents stay out of the repository (only counts leave it):

    refs/venv-rs/bin/python validation/oracles/rocketserializer/geometry.py \\
        corpus-out/rocketserializer.json refs --jar

Each argument after the output is a file or a directory searched for `.ork` files; `--jar` adds the
example designs inside the OpenRocket jar. The output is written to the path given, not to
standard output, which OpenRocket logs to.
"""

import hashlib
import json
import logging
import re
import sys
import tempfile
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "openrocket"))

import automatic_radius  # noqa: E402 - the JVM start and the loader are shared

RS_COMMIT = "66d8ca8c9be36816c4157fbdf249e87fb8c1f5dc"
# Directories under refs/ that are Python environments, not designs. Everything else under refs/
# is read, as `cargo xtask ork` reads it.
SKIPPED = {"venv", "venv-rs"}


def document_text(path):
    """The design document inside a `.ork` file, as text: zipped, gzipped or bare XML."""
    raw = Path(path).read_bytes()
    if raw.startswith(b"PK"):
        with zipfile.ZipFile(path) as archive:
            names = [n for n in archive.namelist() if n.lower().endswith(".ork")]
            raw = archive.read(names[0] if names else archive.namelist()[0])
    elif raw.startswith(b"\x1f\x8b"):
        import gzip

        raw = gzip.decompress(raw)
    return raw.decode("utf-8")


def opened(text, scratch):
    """OpenRocket's document for `text`, and whether a leading comment had to be removed."""
    path = Path(scratch) / "openrocket.ork"
    path.write_text(text, encoding="utf-8")
    try:
        return automatic_radius.load(path), False
    except Exception:  # noqa: BLE001 - retried once without the comment, then reported
        bare = re.sub(r"^(<\?xml[^>]*\?>\s*)<!--.*?-->\s*", r"\1", text, count=1, flags=re.S)
        if bare == text:
            raise
        path.write_text(bare, encoding="utf-8")
        return automatic_radius.load(path), True


# The OpenRocket class each tag is read into. A nose cone is a Transition to OpenRocket too, so
# classes are told apart by their exact name.
CLASSES = {
    "nosecone": "NoseCone",
    "transition": "Transition",
    "trapezoidfinset": "TrapezoidFinSet",
    "ellipticalfinset": "EllipticalFinSet",
}
# The parts hpr keeps whole in `x-openrocket` without reading, which can hold the components above.
KEPT_PARTS = {"podset", "parallelstage"}


def in_order(component, found):
    """Every component under `component`, depth first, by class: the document's order."""
    found.setdefault(str(component.getClass().getSimpleName()), []).append(component)
    for child in component.getChildren():
        in_order(child, found)
    return found


def station(component):
    """Where OpenRocket puts the component's forward end, m aft of the nose tip."""
    return float(component.getComponentLocations()[0].x)


def earlier_siblings(component):
    """The lengths of the components before this one in its parent, summed, m.

    RocketSerializer's walk (`process_elements_position`) moves its running station on by each
    child's length as it goes, so a component that is not its parent's first child is placed that
    much further aft; recorded so that such a difference can be checked rather than asserted."""
    parent = component.getParent()
    if parent is None:
        return 0.0
    total = 0.0
    for sibling in parent.getChildren():
        if sibling == component:
            break
        total += float(sibling.getLength())
    return total


def openrocket_numbers(tag, component):
    """OpenRocket's own numbers for a component of the kind `tag`, from its public API."""
    numbers = {
        "station_m": station(component),
        "length_m": float(component.getLength()),
        "earlier_siblings_m": earlier_siblings(component),
    }
    if tag == "nosecone":
        numbers["base_radius_m"] = float(component.getAftRadius())
        numbers["shape"] = str(component.getShapeType().name())
        numbers["shape_parameter"] = float(component.getShapeParameter())
        # The profile OpenRocket draws, a quarter, a half and three quarters of the way back: what
        # settles a shape whose word and parameter say different things (an ogive of parameter 0
        # is a cone).
        length = float(component.getLength())
        numbers["profile_radii_m"] = [float(component.getRadius(f * length)) for f in (0.25, 0.5, 0.75)]
    elif tag == "transition":
        numbers["fore_radius_m"] = float(component.getForeRadius())
        numbers["aft_radius_m"] = float(component.getAftRadius())
    elif tag in ("trapezoidfinset", "ellipticalfinset"):
        # OpenRocket turns a canted fin about the middle of its root chord, which moves the front
        # of the root aft by half the chord times (1 - cos cant): 38 um for a 0.495 m root at 1
        # degree. The file places the root before it is turned, and so does hpr, so the station
        # is read with the cant set to zero, then the cant is put back; the turned one is kept.
        numbers["canted_station_m"] = numbers["station_m"]
        cant = component.getCantAngle()
        component.setCantAngle(0.0)
        numbers["station_m"] = station(component)
        component.setCantAngle(cant)
        numbers["count"] = int(component.getFinCount())
        numbers["span_m"] = float(component.getHeight())
        numbers["cant_rad"] = float(component.getCantAngle())
        if tag == "trapezoidfinset":
            numbers["root_chord_m"] = float(component.getRootChord())
            numbers["tip_chord_m"] = float(component.getTipChord())
            numbers["sweep_m"] = float(component.getSweep())
        else:
            numbers["root_chord_m"] = float(component.getLength())
    return numbers


def largest_radius(java):
    """The largest body radius OpenRocket holds, m: every nose cone's base, tube and transition,
    pods included, as RocketSerializer's `get_rocket_radius` takes every one the file writes."""
    radii = [float(c.getAftRadius()) for c in java.get("NoseCone", [])]
    radii += [float(c.getOuterRadius()) for c in java.get("BodyTube", [])]
    for c in java.get("Transition", []):
        radii += [float(c.getForeRadius()), float(c.getAftRadius())]
    return max(radii, default=0.0)


def safe(function, *args):
    """RocketSerializer's extractor, or the error it raised, as `ork_extractor` catches them."""
    try:
        return {"value": function(*args)}
    except Exception as error:  # noqa: BLE001 - the failure is part of the record
        return {"error": f"{type(error).__name__}: {error}"}


def listed(result):
    """An extractor's dict keyed by index, as a list in that order."""
    if "value" not in result:
        return result
    value = result["value"]
    if isinstance(value, dict) and all(isinstance(key, int) for key in value):
        return {"value": [value[key] for key in sorted(value)]}
    return result


def with_ids(result, elements, tag, java, numbers=None):
    """Each entry RocketSerializer returned, beside the element it came from and OpenRocket's
    numbers for that component, taken as a `numbers` (the tag's own kind unless given).

    `elements` are the elements RocketSerializer read, in its entries' order. OpenRocket builds its
    components in the document's order, so the `k`-th `<tag>` element of the document is the
    `k`-th component of the tag's class. The element's `<id>` names the component for hpr; an
    older file writes none, and then its name and place among the elements of the same tag and
    name do. OpenRocket's component is used only when its name is the element's."""
    if "value" not in result:
        return result
    rows = []
    every = [e for e in elements[0].find_parents()[-1].find_all(tag)] if elements else []
    components = java.get(CLASSES[tag], [])
    for k, entry in enumerate(result["value"]):
        element = elements[k] if k < len(elements) else None
        own = element.find("id", recursive=False) if element is not None else None
        name = element.find("name", recursive=False) if element is not None else None
        name = name.text if name is not None else None
        index = next((i for i, e in enumerate(every) if e is element), None)
        component = components[index] if index is not None and index < len(components) else None
        if component is not None and str(component.getName()) != name:
            component = None
        rows.append(
            {
                "tag": tag,
                "id": own.text.strip() if own is not None else None,
                "name": name,
                "in_kept_part": element is not None
                and any(parent.name in KEPT_PARTS for parent in element.parents),
                "rocketserializer": entry,
                "openrocket": openrocket_numbers(numbers or tag, component)
                if component is not None
                else None,
            }
        )
    return {"value": rows}


def read(path, scratch):
    from bs4 import BeautifulSoup
    from rocketserializer.components.fins import search_elliptical_fins, search_trapezoidal_fins
    from rocketserializer.components.nose_cone import search_nosecone
    from rocketserializer.components.open_rocket_wrangler import process_elements_position
    from rocketserializer.components.rocket import get_rocket_radius
    from rocketserializer.components.transition import search_transitions

    text = document_text(path)
    record = {"sha256": hashlib.sha256(Path(path).read_bytes()).hexdigest()}
    try:
        document, stripped = opened(text, scratch)
    except Exception as error:  # noqa: BLE001 - a refusal is the measurement
        record["opens"] = False
        record["refused"] = str(error).splitlines()[0]
        return record
    record["opens"] = True
    record["comment_removed"] = stripped
    bs = BeautifulSoup(text, features="xml")
    rocket = document.getRocket()
    java = in_order(rocket, {})
    elements = safe(process_elements_position, rocket, {}, 0, 0, 0)
    if "error" in elements:
        record["elements"] = elements
        return record
    elements = elements["value"]
    radius = safe(get_rocket_radius, bs)
    record["rocket_radius"] = radius
    record["openrocket_largest_radius_m"] = largest_radius(java)

    nose = safe(search_nosecone, bs, elements, radius.get("value", 0.0))
    if "value" in nose:
        # With no `<nosecone>`, RocketSerializer takes the first `<transition>` named "Nosecone".
        tag, tags = "nosecone", bs.find_all("nosecone")[:1]
        if not tags:
            tag = "transition"
            tags = [t for t in bs.find_all("transition") if getattr(t.find("name"), "text", "") == "Nosecone"][:1]
        entries = [nose["value"]] if nose["value"] else []
        nose = with_ids({"value": entries}, tags, tag, java, numbers="nosecone")
    record["nose"] = nose
    record["trapezoidal_fins"] = with_ids(
        listed(safe(search_trapezoidal_fins, bs, elements)),
        bs.find_all("trapezoidfinset"),
        "trapezoidfinset",
        java,
    )
    record["elliptical_fins"] = with_ids(
        listed(safe(search_elliptical_fins, bs, elements)),
        bs.find_all("ellipticalfinset"),
        "ellipticalfinset",
        java,
    )
    record["transitions"] = with_ids(
        listed(safe(search_transitions, bs, elements, document)),
        bs.find_all("transition"),
        "transition",
        java,
    )
    return record


def designs(arguments, root):
    """Every `.ork` named or found under the directories named, as (name, path) pairs."""
    found = []
    for argument in arguments:
        path = Path(argument)
        if path.is_file():
            found.append(path)
            continue
        for candidate in sorted(path.rglob("*")):
            parts = candidate.relative_to(path).parts
            if candidate.suffix.lower() == ".ork" and candidate.is_file() and not (
                path.name == "refs" and parts[0] in SKIPPED
            ):
                found.append(candidate)
    return [(str(p.resolve().relative_to(root)), p) for p in found]


def main():
    args = sys.argv[1:]
    jar = "--jar" in args
    paths = [arg for arg in args if not arg.startswith("--")]
    if len(paths) < 2 and not (jar and paths):
        sys.exit("usage: geometry.py OUTPUT.json (FILE | DIR)... [--jar]")
    output, inputs = Path(paths[0]), paths[1:]
    root = Path.cwd().resolve()
    logging.disable(logging.CRITICAL)
    automatic_radius.start()
    from info.openrocket.core.util import BuildProperties

    runs = []
    with tempfile.TemporaryDirectory() as scratch:
        files = designs(inputs, root)
        if jar:
            with zipfile.ZipFile(automatic_radius.JAR) as archive:
                for entry in sorted(archive.namelist()):
                    if entry.startswith("datafiles/examples/") and entry.lower().endswith(".ork"):
                        copy = Path(scratch) / f"example-{len(files)}.ork"
                        copy.write_bytes(archive.read(entry))
                        files.append((f"{automatic_radius.JAR}!{entry}", copy))
        for name, path in files:
            runs.append({"file": name, **read(path, scratch)})

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "source": "validation/oracles/rocketserializer/geometry.py",
                "rocketserializer": f"RocketPy-Team/RocketSerializer@{RS_COMMIT}",
                "openrocket": str(BuildProperties.getVersion()),
                "jar_sha256": hashlib.sha256(automatic_radius.JAR.read_bytes()).hexdigest(),
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
