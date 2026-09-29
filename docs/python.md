# Python

This page shows how to fly a rocket from Python with the `hpr` package. The package has the same
four types as the Rust [builder](the-builder.md): an `Environment`, a `Motor`, a `Rocket` and a
`Flight`. You describe the rocket part by part from the nose back, or read it from a design file,
then fly it from a rail. A flight's metrics come back as numbers and dictionaries, and its
recording as [NumPy](https://numpy.org/) arrays. You need Python 3.10 or later and some Python.
Today you also build the package yourself, which needs the Rust toolchain
([Install it](#install-it)).

> **How far to trust it.** The package adds no physics. Each call hands its numbers to the Rust
> builder and returns what that returns, so a flight made in Python runs the same code as one the
> Rust builder makes from the same numbers. A test builds [The builder](the-builder.md)'s example
> rocket in Python and matches every digit that Rust example prints
> ([`test_prints_what_the_rust_example_prints`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-py/tests/test_builder.py)).
> The last digits of a number can still differ between builds, as a release build rounds a little
> differently from a debug one. So what that page and [Accuracy](accuracy.md) say about the
> numbers holds here too. The rocket below has never been flown for real, so no flight checks
> it. Where it lands in a wind is the least certain number of all: on two of RocketPy's example
> rockets, hpr's drift and RocketPy's differ by 11 to 43%
> ([Getting started](getting-started.md#how-far-to-trust-it) explains why). The package is new
> and covers less than the Rust library ([What is not here yet](#what-is-not-here-yet)). It is
> built and tested on Linux, macOS and Windows, but not published on PyPI, Python's package index.

## Install it

The package isn't on PyPI yet, so you build it from this repository. You need the Rust toolchain
from [Getting started](getting-started.md) and [maturin](https://www.maturin.rs/), the tool that
builds Python packages from Rust. From the repository's root, make a
[virtual environment](https://docs.python.org/3/library/venv.html) and build into it:

```bash
python3 -m venv .venv
source .venv/bin/activate          # on Windows: .venv\Scripts\activate
pip install maturin
maturin develop --release --manifest-path crates/hpr-py/Cargo.toml
```

`maturin develop` builds the package and installs it into the environment, with NumPy. It links
the environment to the build, so after a change to the Rust code, run it again.

To install elsewhere, build a *wheel* instead, the file `pip install` takes:
`maturin build --release --manifest-path crates/hpr-py/Cargo.toml` writes it to `target/wheels/`.
One wheel serves every CPython (the standard Python) from 3.10 on, on the operating system and
processor that built it. `scripts/python-tests.sh` builds one and runs the package's tests on it,
which needs [uv](https://docs.astral.sh/uv/).

The package imports as `hpr`. Its distribution name, the one `pip` lists, is `hpr-sim`.

## A first flight

This builds a simpler version of the rocket in [Your own rocket](your-own-rocket.md) and
[The builder](the-builder.md): a 54 mm airframe with an ogive nose, three fins and a Cesaroni
H54, whose recovery bay here is a point mass and whose nose has no shoulder. It flies from a 1.8 m
rail leaning 5° into a 5 m/s west wind, with a parachute opened by the motor's ejection charge.

```python
import hpr

environment = hpr.Environment(32.99, -106.97, 1400.0, wind_speed_m_s=5.0, wind_from_deg=270.0)

rocket = hpr.Rocket("My 54 mm rocket", 0.0563)
rocket.add_nose("ogive", 0.22, "abs", wall_m=0.0015)
rocket.add_tube(0.9, 0.00115, "kraft_phenolic")
rocket.add_motor_tube(0.2, 0.029, 0.001, "kraft_phenolic", overhang_m=0.005)
rocket.add_fins(
    3,
    root_chord_m=0.1,
    tip_chord_m=0.04,
    span_m=0.045,
    sweep_m=0.05,
    thickness_m=0.003175,
    material="birch_plywood",
    cross_section="rounded",
)
rocket.add_mass(0.2, position="top", offset_m=0.07, name="recovery bay")
rocket.set_motor(hpr.Motor.from_catalog("H54", delay_s=10.0))
rocket.add_parachute("parachute", diameter_m=0.9, trigger="motor_delay")

print(f"Stability at liftoff: {rocket.static_margin_cal(0.0, 0.3):.2f} calibres")

flight = hpr.Flight(rocket, environment, 1.8, inclination_deg=85.0, heading_deg=270.0)
print(f"Apogee:    {flight.apogee_m:.1f} m at {flight.apogee_time_s:.2f} s")
print(f"Top speed: {flight.max_speed_m_s:.0f} m/s (Mach {flight.max_mach:.2f})")
landing = flight.landing
print(f"Landing:   {landing['distance_m']:.0f} m from the pad, descending at {landing['descent_rate_m_s']:.1f} m/s")
```

It prints:

```text
Stability at liftoff: 2.12 calibres
Apogee:    1118.3 m at 13.67 s
Top speed: 190 m/s (Mach 0.57)
Landing:   905 m from the pad, descending at 4.6 m/s
```

Line by line:

- `Environment` takes the launch site's latitude and longitude in degrees (longitude positive
  east, so negative in the Americas) and its elevation in metres. The air is the
  [US Standard Atmosphere 1976](physics/atmosphere.md). The wind is the same at every height, and
  blows **from** `wind_from_deg`, clockwise from north.
- `Rocket` takes a name and the airframe's outside diameter, m.
- The nose is a [tangent ogive](glossary.md#tangent-ogive) 0.22 m long, of ABS, hollow with a
  1.5 mm wall. The tube is 0.9 m long with a 1.15 mm wall. The motor tube is 0.2 m long, with a
  29 mm bore and a 1 mm wall.
- Parts go on from the nose back: the nose, then tubes. Fins, a motor tube and masses go on or in
  the tube before them. Each part names its material by an id from the built-in list,
  `hpr.materials()`, and takes its mass from its shape and that material's density.
- The fins' lengths are named, so two can't swap unseen: the root chord, tip chord and span, and
  the sweep, how far aft of the root's leading edge the tip's is.
- `Motor.from_catalog` finds one of the [32 bundled motors](physics/motor.md#the-bundled-motors)
  by its name. It doesn't take the delay from the designation (the H54's full one is 168H54-10A),
  so give it as `delay_s`: a parachute opened by the motor needs one. `Motor.from_file` reads a
  RASP [`.eng`](format/eng.md) or RockSim [`.rse`](format/rse.md) file.
- `static_margin_cal(0.0, 0.3)` is the stability margin at ignition (0 s) and Mach 0.3.
- `Flight` flies the rocket as soon as it is made. The rail is 1.8 m long and leans
  `inclination_deg` above the horizon, 90 by default, toward `heading_deg`, clockwise from north.
- The landing's `descent_rate_m_s` is how fast it comes down. Its `ground_hit_speed_m_s` is
  faster, as it adds the wind's drift.

The stability margin is the distance from the
[centre of gravity](glossary.md#centre-of-gravity-cg) back to the
[centre of pressure](glossary.md#centre-of-pressure-cp), in body diameters
([calibres](glossary.md#calibre-caliber)). It is 2.12 here, where The builder's example says
1.92. That example packs its 200 g recovery bay into a 15 cm cylinder, which moves the bay's
centre, and so the rocket's centre of gravity, aft: about 0.4 calibres less. It also gives the
nose a capped shoulder, which adds mass at the front: about 0.2 calibres more. A test holds both
steps to these sizes.

## The recording

A flight is recorded at every step of the [integrator](physics/integration.md), or every
`interval_s` seconds (at least 0.001 s) if you give `Flight` one, with a row at every event too.
`flight.columns` names the columns, each with its unit, and `flight["name"]` gives one as a NumPy
array. `flight.series` gives them all, as a dictionary of arrays. Continuing from above:

```python
height_m = flight["height_above_ground_m"]
time_s = flight["time_s"]
print(len(flight.columns), "columns:", ", ".join(flight.columns[:3]), "...")
print(f"Highest recorded height: {height_m.max():.1f} m at {time_s[height_m.argmax()]:.2f} s")
for event in flight.events[:4]:
    print(f"{event['kind']:>10} at {event['time_s']:6.3f} s")
```

```text
30 columns: time_s, position_east_m, position_north_m ...
Highest recorded height: 1118.3 m at 13.67 s
   liftoff at  0.001 s
 rail_exit at  0.174 s
   burnout at  3.500 s
   trigger at 13.500 s
```

The columns are the ones [Recording a trajectory](recording-a-trajectory.md) lists: time, position
and velocity east, north and up of the pad, attitude, height, airspeed, Mach number, angle of
attack, thrust, mass and more. Heights are the centre of gravity's, above the pad; it stands on the
rail at the start, so the first height isn't zero. The arrays are read-only copies, so a slip
can't change the flight's record. To plot one, hand the arrays to any plotting library, for
example `matplotlib.pyplot.plot(time_s, height_m)`.

`flight.events` lists what happened, in the order it happened. Each event is a dictionary with its
`kind`, the `index` of the parachute or motor it is about, its `time_s`, and the flight's state
then, `sample`. The kinds are `"liftoff"`, `"rail_exit"`, `"burnout"`, `"apogee"`, `"trigger"` (a
parachute's charge fires), `"deployment"` (it is open) and `"ground_hit"`. Here the parachute's
charge fires at 13.5 s, the H54's 3.5 s burn plus its 10 s delay, 0.17 s before apogee.

`flight.landing` is a dictionary of the landing: its `time_s`, `latitude_deg`, `longitude_deg`,
`east_m` and `north_m` of the pad, `distance_m`, `ground_hit_speed_m_s` and `descent_rate_m_s`.
`flight.summary` has every metric of the flight as a dictionary: the apogee, top speed, Mach
number and dynamic pressure, the stability margins and the landings, each explained in
[Flight metrics](physics/metrics.md). `flight.to_json()` is the whole record as JSON text.

## A design from a file

`Rocket.from_file` reads a design instead of building one: an hpr design (`.hpr` or `.hprz`,
[The hpr design format](format/hpr.md)), an OpenRocket `.ork` file ([`.ork` design
files](format/ork.md)), or a rocket's JSON. It flies the
[motor configuration](glossary.md#configuration) you name, or the design's only one.

A design read from a file flies **without** the parachutes and stage separations it stores. Add
parachutes with `add_parachute`; until you do, it falls to the ground unslowed, and its landing
numbers mean nothing. `rocket.notes` lists what reading the file said: the `.ork` reader's
warnings, a design written by an older version of the format, and what the file holds that isn't
flown.

This one is [RocketPy](glossary.md#rocketpy)'s example rocket, Calisto, from the file the
validation suite uses, with its two parachutes as RocketPy's example gives them:

```python
calisto = hpr.Rocket.from_file("validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json")
calisto.add_parachute("drogue", cd_s_m2=1.0, lag_s=1.5)
calisto.add_parachute("main", cd_s_m2=10.0, trigger="altitude", altitude_m=800.0, lag_s=1.5)
site = hpr.Environment(32.990254, -106.974998, 1400.0)
flight = hpr.Flight(calisto, site, 5.2, inclination_deg=85.0)
print(f"{calisto.configuration}: apogee {flight.apogee_m:.0f} m, landing at {flight.landing['descent_rate_m_s']:.1f} m/s")
```

```text
example: apogee 2807 m, landing at 5.2 m/s
```

This flight uses hpr's own drag, in calm air. The validation suite flies the same design with
hpr's own drag too, in a wind and in RocketPy's atmosphere, and its apogee is within 0.609% of
RocketPy's ([Whole flights with each code's own drag](accuracy.md#whole-flights-with-each-codes-own-drag),
[same-drag and predicted mode](glossary.md#same-drag-and-predicted-mode)). Flying Calisto from
Python as that suite does, and comparing it with RocketPy, is the next step
([M4.3b](decisions-and-roadmap.md#m4-3b)).

## When something is wrong

A value out of its range, a part out of order, or a motor or material that isn't there raises
`hpr.HprError`, a kind of `ValueError`, with the library's own message. So does an argument that
doesn't apply, such as a `canopy` beside a `cd_s_m2`:

```python
try:
    hpr.Motor.from_catalog("Z9000")
except hpr.HprError as error:
    print(error)
```

```text
no motor with a bundled thrust curve matches `Z9000`
```

## Names and units

Every value is in SI units, and every argument and attribute that carries one names it, as the Rust
API does: `length_m`, `mass_kg`, `apogee_m`, `max_speed_m_s`, `wind_from_deg`. Names that pick one
of several things are strings, in any case, with spaces or hyphens for the underscores:

| argument | the choices |
|---|---|
| a nose's or transition's `shape` | `"conical"`, `"ogive"`, `"elliptical"`, `"power_series"`, `"parabolic_series"`, `"haack"` ([Shapes](physics/shapes.md)). `parameter` is an ogive's radius ratio (1, a tangent ogive, by default), a Haack series' `C` (0, the von Kármán, by default), and a power series' exponent or a parabolic series' `K`, which those two need |
| fins' `cross_section` | `"square"` (the default), `"rounded"`, `"airfoil"` |
| a part's `position` | `"top"`, `"middle"`, `"bottom"` or `"after"` the tube it is in, then `offset_m` aft of that. A mass is at the top by default; fins and a motor tube sit flush with the tube's aft end |
| a parachute's `trigger` | `"apogee"` (the default), `"altitude"` with `altitude_m`, `"time"` with `time_s`, `"motor_delay"` with `motor`, the motor's number (0, the first, by default) |
| a parachute's `canopy` | `"flat_circular"` (the default), `"conical"`, `"biconical"`, `"triconical"`, `"extended_skirt10_flat"`, `"extended_skirt14_full"`, `"hemispherical"`, `"annular"`, `"cross"`, `"flat_ribbon"`, `"conical_ribbon"`, `"ringslot"`, `"ringsail"` ([Recovery](physics/recovery.md)) |

A parachute is either a canopy `diameter_m` across, with its type's drag coefficient or your own
`drag_coefficient`, or a drag area `cd_s_m2`, as RocketPy's `cd_s`.

Each class and method has a docstring listing its arguments: `help(hpr.Rocket.add_fins)`, for
example. The Rust reference for the package, [`hpr_py`](api/hpr_py/index.html), says how it is
built.

## What is not here yet

The package covers the Rust builder, not the whole library. Not yet:

- A drag curve of your own, from RocketPy or a wind tunnel, and RocketPy's example flight
  reproduced in Python ([M4.3b](decisions-and-roadmap.md#m4-3b)).
- Drag, wind and other models written as Python functions
  ([M4.3c](decisions-and-roadmap.md#m4-3c)). Rust programs have them today
  ([Models of your own](custom-models.md)).
- Staging and mass shifts. A staged design read from a file flies as one stack whose upper motors
  never light, and `rocket.notes` says so.
- Parachutes and separations stored in a design file, as above.
- Wind that changes with height, and weather files: only the Rust library flies them.
- Type stubs, so an editor sees the docstrings but not the arguments' types.
