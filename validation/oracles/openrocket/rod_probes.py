"""The pod probes' airframe launched from a tilted rod, for OpenRocket 24.12 to fly (M2.2e5).

`cargo xtask ork-flights` flew only a vertical launch rod until M2.2e5 (issue #173). hpr's rail
takes a heading clockwise from true north and an angle above the horizon; OpenRocket's
`launchroddirection` and `launchrodangle` are, by `conditions.py`'s measurement, a compass
bearing and an angle from the vertical. These probes check that reading on a flight both codes
fly: `pod_probes.py`'s airframe with no pods (`pods-none`, the control, flown from OpenRocket's
default vertical rod) with a stored simulation that tilts the rod

- 5 degrees toward north;
- 10 degrees toward east;
- 10 degrees toward south-west, the same tilt as the last in another direction, which in calm
  air should move little but where the rocket goes;
- 20 degrees toward east.

Every other condition is OpenRocket 24.12's default, as `pods-none` flies it: a 1 m rod, no wind,
the standard atmosphere at sea level at 28.61 N 80.6 W, spherical geodetic computation and a
0.05 s time step. The stored simulation is written as OpenRocket 24.12 writes one (read from a file
it saved), with its angles in degrees, as `conditions.py` found it writes them.

`flights.py` flies them with the other public designs, taking each file's stored conditions, and
records where each rocket is at apogee; `cargo xtask ork-flights` flies hpr from the same rod and
compares. Run from the repository root, then `flights.py` and `motor_database.py` as the status
page's handoff says:

    refs/venv/bin/python validation/oracles/openrocket/rod_probes.py \\
        validation/fixtures/ork/rod-flights

OpenRocket is not run to write them, but `pod_probes` needs the oracle environment (JPype). The
files are plain XML, which OpenRocket and hpr both read.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import pod_probes  # noqa: E402 - the airframe, its motor configuration and fixed ids

#: Each probe's rod: its angle from the vertical and its direction, a compass bearing, in degrees.
PROBES = {
    "rod-5-north": ("5.0", "0.0"),
    "rod-10-east": ("10.0", "90.0"),
    "rod-10-southwest": ("10.0", "225.0"),
    "rod-20-east": ("20.0", "90.0"),
}


def simulation(angle, direction):
    """A stored simulation of the probe's configuration from a rod `angle` degrees from the
    vertical toward the bearing `direction` degrees, every other condition OpenRocket's default."""
    return (
        '<simulations><simulation status="notsimulated"><name>Tilted rod</name>'
        "<simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>"
        f"<conditions><configid>{pod_probes.CONFIGURATION}</configid>"
        "<launchrodlength>1.0</launchrodlength><launchintowind>false</launchintowind>"
        f"<launchrodangle>{angle}</launchrodangle>"
        f"<launchroddirection>{direction}</launchroddirection>"
        "<windaverage>0.0</windaverage><windturbulence>0.0</windturbulence>"
        "<winddirection>1.5707963267948966</winddirection><windmodeltype>Average</windmodeltype>"
        "<launchaltitude>0.0</launchaltitude><launchlatitude>28.61</launchlatitude>"
        "<launchlongitude>-80.6</launchlongitude><geodeticmethod>spherical</geodeticmethod>"
        '<atmosphere model="isa"/><timestep>0.05</timestep><maxtime>1200.0</maxtime>'
        "</conditions></simulation></simulations>"
    )


def document(name, angle, direction):
    """`pods-none`'s document, renamed `name`, with the tilted rod's stored simulation."""
    text = pod_probes.document(name, "")
    end = "</openrocket>\n"
    if not text.endswith(end):
        raise RuntimeError("pod_probes.document no longer ends its document as expected")
    return text[: -len(end)] + simulation(angle, direction) + end


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: rod_probes.py DIRECTORY")
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for name, (angle, direction) in PROBES.items():
        text = document(name, angle, direction)
        (out / f"{name}.ork").write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
