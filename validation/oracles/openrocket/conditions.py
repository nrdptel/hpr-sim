"""The units and meaning of a stored simulation's launch conditions and flight data in a `.ork`.

A `.ork` stores, beside the design, the simulations OpenRocket last ran: the launch conditions
(`<conditions>`) and the results (`<flightdata>`, with one time series per stage). OpenRocket's
file-format page shows the tags without their units, and one example there writes the launch rod's
direction as `90.0` and the wind's as `1.5707963267948966`, which cannot both be one unit. This
script measures what the program means, running OpenRocket 24.12 as an external oracle on the
"A simple model rocket" example inside its jar:

- `written`: the conditions are set through the program's public setters, the design is saved, and
  the text written for each is recorded beside the value set (OpenRocket holds angles in radians);
- `loaded`: that saved text is loaded again and read back through the getters;
- `direction`: in calm air, the rod tilted 10 degrees, the rocket is flown with the rod's direction
  set to 0 and to 90 degrees, and where it lands (east and north of the pad) is recorded, which says
  whether the direction is a compass bearing;
- `into_wind`: a file with the rod pointing 90 degrees, wind from 0, and `launchintowind` true is
  loaded, and the rod direction OpenRocket then holds is recorded;
- `datapoints`: a flight is saved with its data, and one row of the stored time series is recorded
  beside the same quantities as the program holds them, which gives each column's unit.

OpenRocket is run, never read: its source is GPL, and nothing here comes from it. The class and
method names used are the public API that `javap` prints for the jar. Only numbers are recorded,
never a design's contents. OpenRocket 24.12 needs Java 17 exactly; see `automatic_radius.py`. Run
from the repository root:

    refs/venv/bin/python validation/oracles/openrocket/conditions.py \\
        validation/fixtures/ork/openrocket-conditions.json

The fixture is written to the path given, not to standard output, which OpenRocket logs to.
"""

import hashlib
import io
import json
import math
import sys
import tempfile
import zipfile
from pathlib import Path
from xml.etree import ElementTree

import jpype
import jpype.imports

from automatic_radius import JAR
from events import SINGLE, example, name_of, start

SEED = 1
TAGS = [
    "launchrodlength",
    "launchrodangle",
    "launchroddirection",
    "launchintowind",
    "windaverage",
    "windturbulence",
    "winddirection",
    "launchaltitude",
    "launchlatitude",
    "launchlongitude",
]
COLUMNS = [
    "Time",
    "Altitude",
    "Vertical velocity",
    "Vertical orientation (zenith)",
    "Lateral direction",
    "Latitude",
    "Air temperature",
    "Air pressure",
]


def save(document_, with_data):
    """The design document OpenRocket writes for `document_`, as text."""
    from info.openrocket.core.document import StorageOptions
    from info.openrocket.core.file.openrocket import OpenRocketSaver
    from info.openrocket.core.logging import ErrorSet, WarningSet
    from java.io import ByteArrayOutputStream

    options = StorageOptions()
    options.setSaveSimulationData(with_data)
    out = ByteArrayOutputStream()
    OpenRocketSaver().save(out, document_, options, WarningSet(), ErrorSet())
    raw = bytes(out.toByteArray())
    if raw.startswith(b"PK"):
        raw = zipfile.ZipFile(io.BytesIO(raw)).read("rocket.ork")
    return raw.decode("utf-8")


def load_text(xml):
    """A design loaded from its document's text."""
    from info.openrocket.core.file import GeneralRocketLoader
    from java.io import File

    with tempfile.TemporaryDirectory() as scratch:
        path = Path(scratch) / "probe.ork"
        path.write_text(xml, encoding="utf-8")
        return GeneralRocketLoader(File(str(path))).load()


def conditions_of(xml):
    """The first simulation's `<conditions>` element in a saved document."""
    return ElementTree.fromstring(xml).find("simulations/simulation/conditions")


def options(document_):
    return list(document_.getSimulations())[0].getOptions()


def held(options_):
    """The conditions as the program holds them."""
    return {
        "launchrodlength": float(options_.getLaunchRodLength()),
        "launchrodangle": float(options_.getLaunchRodAngle()),
        "launchroddirection": float(options_.getLaunchRodDirection()),
        "launchintowind": bool(options_.getLaunchIntoWind()),
        "windaverage": float(options_.getWindSpeedAverage()),
        "windturbulence": float(options_.getWindTurbulenceIntensity()),
        "winddirection": float(options_.getWindDirection()),
        "launchaltitude": float(options_.getLaunchAltitude()),
        "launchlatitude": float(options_.getLaunchLatitude()),
        "launchlongitude": float(options_.getLaunchLongitude()),
        "isa": bool(options_.isISAAtmosphere()),
        "launch_temperature": float(options_.getLaunchTemperature()),
        "launch_pressure": float(options_.getLaunchPressure()),
    }


def written_and_loaded():
    """Conditions set in radians and SI, the text written for each, and what loading it gives."""
    document_ = example(SINGLE)
    set_ = options(document_)
    set_.setLaunchRodLength(1.5)
    set_.setLaunchRodAngle(math.radians(5.0))
    set_.setLaunchRodDirection(math.radians(45.0))
    set_.setLaunchIntoWind(False)
    set_.setWindSpeedAverage(3.0)
    set_.setWindTurbulenceIntensity(0.1)
    set_.setWindDirection(0.5)
    set_.setLaunchAltitude(200.0)
    set_.setLaunchLatitude(40.0)
    set_.setLaunchLongitude(-105.0)
    set_.setISAAtmosphere(False)
    set_.setLaunchTemperature(300.0)
    set_.setLaunchPressure(95000.0)
    before = held(set_)
    xml = save(document_, False)
    conditions = conditions_of(xml)
    written = {tag: (conditions.findtext(tag) or "").strip() for tag in TAGS}
    atmosphere = conditions.find("atmosphere")
    written["atmosphere"] = {
        "model": atmosphere.get("model") if atmosphere is not None else None,
        "basetemperature": (atmosphere.findtext("basetemperature") or "").strip()
        if atmosphere is not None
        else None,
        "basepressure": (atmosphere.findtext("basepressure") or "").strip()
        if atmosphere is not None
        else None,
    }
    return {"set": before, "written": written, "loaded": held(options(load_text(xml)))}


def landing(direction_deg):
    """Where the example lands, east and north of the pad, from a rod tilted 10 degrees toward
    `direction_deg`, in calm air."""
    from info.openrocket.core.simulation import FlightDataType

    document_ = example(SINGLE)
    simulation = list(document_.getSimulations())[0]
    set_ = simulation.getOptions()
    set_.setWindSpeedAverage(0.0)
    set_.setWindTurbulenceIntensity(0.0)
    set_.setLaunchIntoWind(False)
    set_.setLaunchRodAngle(math.radians(10.0))
    set_.setLaunchRodDirection(math.radians(direction_deg))
    set_.setRandomSeed(SEED)
    simulation.simulate()
    branch = simulation.getSimulatedData().getBranch(0)
    east = list(branch.get(FlightDataType.TYPE_POSITION_X))
    north = list(branch.get(FlightDataType.TYPE_POSITION_Y))
    return {
        "rod_direction_deg": direction_deg,
        "landed_east_m": round(east[-1], 2),
        "landed_north_m": round(north[-1], 2),
    }


def into_wind():
    """The rod direction OpenRocket holds after loading a file that points the rod at 90 degrees,
    has the wind from 0 and says `launchintowind`."""
    document_ = example(SINGLE)
    set_ = options(document_)
    set_.setLaunchRodDirection(math.radians(90.0))
    set_.setWindDirection(0.0)
    set_.setLaunchIntoWind(True)
    xml = save(document_, False)
    conditions = conditions_of(xml)
    return {
        "written": {
            tag: (conditions.findtext(tag) or "").strip()
            for tag in ["launchroddirection", "winddirection", "launchintowind"]
        },
        "loaded_rod_direction_rad": float(options(load_text(xml)).getLaunchRodDirection()),
    }


def datapoints():
    """One row of a saved time series beside the same quantities as the program holds them."""
    from info.openrocket.core.simulation import FlightDataType

    document_ = example(SINGLE)
    simulation = list(document_.getSimulations())[0]
    set_ = simulation.getOptions()
    set_.setWindSpeedAverage(0.0)
    set_.setWindTurbulenceIntensity(0.0)
    set_.setRandomSeed(SEED)
    simulation.simulate()
    branch = simulation.getSimulatedData().getBranch(0)
    xml = save(document_, True)
    stored = ElementTree.fromstring(xml).find("simulations/simulation/flightdata/databranch")
    types = [name.strip() for name in stored.get("types").split(",")]
    rows = [point.text.split(",") for point in stored.findall("datapoint")]
    row = len(rows) // 2
    held_by_name = {}
    for kind in FlightDataType.ALL_TYPES:
        held_by_name[str(kind.getName())] = kind
    found = []
    for column in COLUMNS:
        index = types.index(column)
        values = list(branch.get(held_by_name[column]))
        found.append(
            {
                "column": column,
                "stored": float(rows[row][index]),
                "held": float(values[row]),
                "unit": str(held_by_name[column].getUnitGroup().getSIUnit().getUnit()),
            }
        )
    return {
        "branch": str(stored.get("name")),
        "types": len(types),
        "rows": len(rows),
        "row": row,
        "columns": found,
        "event_types": sorted({event.get("type") for event in stored.findall("event")}),
    }


def main():
    paths = sys.argv[1:]
    if len(paths) != 1:
        sys.exit("usage: conditions.py OUTPUT.json")
    start()
    from info.openrocket.core.util import BuildProperties

    fixture = {
        "source": "validation/oracles/openrocket/conditions.py",
        "command": "refs/venv/bin/python validation/oracles/openrocket/conditions.py "
        "validation/fixtures/ork/openrocket-conditions.json",
        "openrocket": str(BuildProperties.getVersion()),
        "jar_sha256": hashlib.sha256(JAR.read_bytes()).hexdigest(),
        "java": str(jpype.java.lang.System.getProperty("java.version")),
        "jpype": jpype.__version__,
        "conditions": written_and_loaded(),
        "direction": [landing(0.0), landing(90.0)],
        "into_wind": into_wind(),
        "datapoints": datapoints(),
    }
    Path(paths[0]).write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
