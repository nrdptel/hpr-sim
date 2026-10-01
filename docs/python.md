# Python

This page shows how to fly a rocket from Python with the `hpr` package. The package has the same
four types as the Rust [builder](the-builder.md): an `Environment`, a `Motor`, a `Rocket` and a
`Flight`. You describe the rocket part by part from the nose back, or read it from a design file,
then fly it from a rail. A flight's metrics come back as numbers and dictionaries, and its
recording as [NumPy](https://numpy.org/) arrays. You need Python 3.10 or later and some
familiarity with it. Today you also build the package yourself, which needs the Rust toolchain
([Install it](#install-it)).

> **How far to trust it.** The package adds no physics. Each call hands its numbers to the Rust
> builder and returns what that returns, so a flight made in Python runs the same code as one the
> Rust builder makes from the same numbers. A test builds [The builder](the-builder.md)'s example
> rocket in Python and matches every digit that Rust example prints
> ([`test_prints_what_the_rust_example_prints`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-py/tests/test_builder.py)).
> Digits beyond those printed can differ between a release and a debug build. Because the code is
> the same, what that page and [Accuracy](accuracy.md) say about the numbers holds here too. The
> rocket below has never been flown for real, so no flight checks it. Where it lands in a wind is the least certain number of all: on two of RocketPy's example
> rockets, hpr's drift and RocketPy's differ by 10 to 38%
> ([Getting started](getting-started.md#how-far-to-trust-it) explains why). The package is new
> and covers less than the Rust library ([What is not here yet](#what-is-not-here-yet)). Two gaps
> matter most if you bring a design file: it flies without the parachutes it stores, and most
> `.ork` configurations are refused ([A design from a file](#a-design-from-a-file)). The package
> is built and tested on Linux, macOS and Windows, but not published on PyPI, Python's package
> index.

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
One wheel serves every CPython (the standard Python) from 3.10 on (tested on 3.10 and 3.13), on
the operating system and processor that built it. `scripts/python-tests.sh` builds one and runs the package's tests on it,
which needs [uv](https://docs.astral.sh/uv/).

The package imports as `hpr`. Its distribution name, the one `pip` lists, is `hpr-sim`.

## A first flight

This builds a simpler version of the rocket in [Your own rocket](your-own-rocket.md) and
[The builder](the-builder.md): a 54 mm airframe with an ogive nose, three fins and a Cesaroni
H54, whose recovery bay here is a point mass and whose nose has no shoulder. It flies from a 1.8 m
rail leaning 5° into a 5 m/s west wind, with a parachute opened by the motor's ejection charge at
the end of its [ejection delay](glossary.md#ejection-delay).

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
  the tube before them.
- Each part names its material by an id from the built-in list, `hpr.materials()`, and takes its
  mass from its shape and that material's density.
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
steps to these sizes
([`test_the_margin_moves_as_the_python_page_says`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-py/tests/test_options.py)).

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
`east_m` and `north_m` of the pad, `distance_m`, `ground_hit_speed_m_s`, `descent_rate_m_s`, and
`body`, which separated body landed (`None` for a rocket that didn't come apart).
`flight.summary` has every metric of the flight as a dictionary: the apogee, top speed, Mach
number and dynamic pressure, the stability margins and the landings, each explained in
[Flight metrics](physics/metrics.md). `flight.to_json()` is the whole record as JSON text.

## A design from a file

`Rocket.from_file` reads a design instead of building one: an hpr design (`.hpr` or `.hprz`,
[The hpr design format](format/hpr.md)), an OpenRocket `.ork` file ([`.ork` design
files](format/ork.md)), or a rocket's JSON. It flies the
[motor configuration](glossary.md#configuration) you name, or the file's default one, or its only
one, as `hpr sim` does.

Most `.ork` files won't fly yet. A configuration flies only if every motor lights at launch and
has a thrust curve, in the file or among hpr's 32 bundled motors: that is 4 of the 170
configurations in the reference library. Otherwise `from_file` raises `HprError` and says why, and
Python can't yet swap in a motor as `hpr sim --motor` does.

A design read from a file flies **without** the parachutes and stage separations it stores. Add
parachutes with `add_parachute`; until you do, it falls to the ground unslowed, and its landing
numbers mean nothing. `rocket.notes` lists what reading the file said: the `.ork` reader's
warnings, a design written by an older version of the format, and what the file holds that isn't
flown.

This one is [RocketPy](glossary.md#rocketpy)'s example rocket, Calisto, from the file the
validation suite uses, with its two parachutes as RocketPy's example gives them. The path is the
repository's, so run it from the repository's root:

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
hpr's own drag too, in a wind and in RocketPy's atmosphere, and its apogee is 0.609% below
RocketPy's. That is the closest of the suite's six rockets: on the others, hpr's own drag puts the
apogee from 7.280% below RocketPy's to 10.302% above
([Whole flights with each code's own drag](accuracy.md#whole-flights-with-each-codes-own-drag),
[same-drag and predicted mode](glossary.md#same-drag-and-predicted-mode)). The next section flies
Calisto from Python as that suite does, and compares it with RocketPy.

## RocketPy's example, flown as the suite flies it

This section flies Calisto again, this time the way the validation suite flies it against
RocketPy. It is for checking hpr against [RocketPy](glossary.md#rocketpy), or for moving a RocketPy
script across. Both codes fly the same drag and the same inputs, so the comparison tests the rest
of the physics: the equations of motion, the atmosphere, the rail and the parachutes. It is a
comparison between two codes, not with a real flight.

Three options make it RocketPy's flight:

- **A drag table.** `DragTable` holds a zero-lift drag coefficient `C_D0` against Mach number, as
  RocketPy's `power_off_drag` and `power_on_drag` do. Its power-on curve is flown while the motor
  burns, its power-off curve the rest of the time. Pass it to a flight as `drag_table=`; it
  replaces hpr's own drag, and nothing else. `DragTable.from_csv` reads RocketPy's two-column
  drag files.
- **RocketPy's gravity.** `Environment(..., gravity="vertical_taylor")` uses RocketPy's formula
  for gravity instead of hpr's default. Near the ground the two differ in size by about one part
  in 10⁸, but hpr's default also turns with the rocket's position over the ground. On this
  flight the switch moves the apogee by 2 parts in 10⁷ and the landing point by 0.3 m
  ([Gravity](physics/gravity.md)).
- **One parachute at a time.** RocketPy flies only the last parachute to open, and hpr adds
  together every one that is open. `released_by` is another parachute's number, counted from 0
  in the order they were added: `add_parachute(..., released_by=1)` cuts this one away once the
  second parachute is fully open ([Recovery](physics/recovery.md#triggers-lag-and-release)).

Here is an invented table with less drag while the motor burns. Calisto flies it in calm air,
as in the section above, to 2650 m above the pad:

```python
table = hpr.DragTable([(0.0, 0.5), (3.0, 0.5)], [(0.0, 0.45), (3.0, 0.45)])
print(f"C_D0 at Mach 0.6: {table.cd0(0.6):.2f} coasting, {table.cd0(0.6, thrusting=True):.2f} burning")
flight = hpr.Flight(calisto, site, 5.2, inclination_deg=85.0, drag_table=table)
print(f"apogee {flight.apogee_m:.0f} m")
```

```text
C_D0 at Mach 0.6: 0.50 coasting, 0.45 burning
apogee 2650 m
```

The example
[`crates/hpr-py/examples/calisto.py`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-py/examples/calisto.py)
puts it all together. It is written as notebook cells (`# %%`), which editors such as VS Code run
one at a time. It reads the rocket, the wind, the drag and the parachutes from the repository's
files, then flies them. Then it measures each metric as RocketPy defines it, and not always as
hpr's own summary does:

- RocketPy follows the rocket's [dry centre of mass](glossary.md#centre-of-dry-mass), its centre
  of mass without propellant, and starts that point at ground level. hpr stands the rocket on its
  rail, so the same point starts 1.250 m up (the first line printed below). The example subtracts
  1.250 m from every height, and opens the main 1.250 m above RocketPy's 800 m.
- RocketPy's rail exit is when its forward rail button leaves the rail, after 3.745 m of travel
  (RocketPy's `effective_1rl`). hpr's is when its aft-most rail guide clears the top of the 5.2 m
  rail, after 4.45 m of travel, so the example reads RocketPy's off the recording.
- RocketPy's flight ends when the dry centre of mass is back at its starting height.

Run from the repository's root, `python crates/hpr-py/examples/calisto.py` prints:

<!-- calisto.py prints -->
```text
The dry centre of mass starts 1.250 m above the ground.
metric                                  hpr   RocketPy  difference
apogee_agl_m                        2613.59    2612.22      +0.05%
apogee_time_s                         22.88      22.85      +0.10%
max_speed_m_s                        243.56     243.52      +0.01%
max_mach                             0.7321     0.7329      -0.12%
max_acceleration_m_s2                112.35     112.24      +0.10%
max_acceleration_power_on_m_s2       112.35     112.24      +0.10%
rail_exit_speed_m_s                   28.20      28.20      -0.01%
rail_exit_time_s                     0.2945     0.2947      -0.07%
burnout_altitude_agl_m               684.23     684.10      +0.02%
burnout_speed_m_s                    235.65     235.60      +0.02%
flight_time_s                        260.86     260.53      +0.12%
apogee_drift_m                       422.52     426.36      -0.90%
landing_drift_m                     1277.36    1261.50      +1.26%
impact_speed_m_s                     5.4549     5.4560      -0.02%
largest difference 1.26%, within 3%: True
```

Every metric is within 3% of RocketPy's, the bound the validation suite holds this flight to
([M2.1](decisions-and-roadmap.md#m2-1)). These are the suite's numbers: a test holds each to within
0.001% of what the suite's report records for the same flight
([`test_calisto.py`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-py/tests/test_calisto.py)).
Because both codes fly the same drag, this says nothing about hpr's own drag, the largest source
of difference: flying its own, hpr puts the suite's six rockets' apogees 7.280% below RocketPy's to
10.302% above, as the section before says. The drifts differ most here. On two of RocketPy's other
example rockets, Juno III and Bella Lui, flown in a wind, the drifts differ by 10 to 38%
([Getting started](getting-started.md#how-far-to-trust-it) explains why).

## Drag and wind of your own

A flight can fly a drag and a wind you write as Python functions, in place of hpr's own drag and
the environment's wind: a drag curve from a wind tunnel or another program, say, or a wind
profile from a weather balloon. hpr calls them as it flies, several times for each time step of
its integrator, and keeps everything else its own: the rest of the aerodynamics, the thrust, the
masses and the atmosphere. They are the Python side of the Rust library's
[models of your own](custom-models.md). **How far to trust it:** as far as your functions. hpr
checks that each number is finite, and that a drag is not negative, but not that it is right.

| Given as | Called as | Returns |
| --- | --- | --- |
| `Flight(..., drag=f)` | `f(mach, thrusting)`, with `thrusting` true while a motor burns | the whole rocket's zero-lift [drag coefficient](glossary.md#drag-coefficient) `C_D0`, on its reference area (the largest body diameter's circle) |
| `Environment(..., wind=f)` | `f(height_m)`, the height above sea level in metres, not above the launch site | the air's velocity `(east_m_s, north_m_s)`, as a tuple, list or array |

Three things about the drag number, as for a Rust drag model:

- It is not rescaled. A `DragTable` can carry the diameter it was measured on; a function's
  number must already be on the rocket's reference area.
- At an angle of attack hpr scales it, as it scales its own.
- Under a parachute the drag is the parachute's, not the function's.

The wind's two numbers are the way the air moves, so a wind from the west is `(+speed, 0)`: the
opposite of `wind_from_deg`, which names where the wind comes from.

Here the drag rises through Mach 0.8 to 1.1 and drops 10% under power, and the wind strengthens
with height:

```python
def drag(mach, thrusting):
    """0.5 below Mach 0.8, rising to 0.8 by Mach 1.1; 10% less while the motor burns."""
    rise = min(max((mach - 0.8) / 0.3, 0.0), 1.0)
    cd0 = 0.5 + 0.3 * rise
    return 0.9 * cd0 if thrusting else cd0


def wind(height_m):
    """From the west, 3 m/s at the 1400 m site, 1 m/s stronger every 100 m above it."""
    above_m = max(height_m - 1400.0, 0.0)
    return (3.0 + above_m / 100.0, 0.0)


windy = hpr.Environment(32.99, -106.97, 1400.0, wind=wind)
flight = hpr.Flight(rocket, windy, 1.8, drag=drag)
print(f"apogee {flight.apogee_m:.0f} m, landed {flight.landing['east_m']:.0f} m east")

table = hpr.DragTable([(0.0, 0.5), (3.0, 0.5)])
same = hpr.Flight(rocket, windy, 1.8, drag=lambda mach, thrusting: 0.5)
print(same.apogee_m == hpr.Flight(rocket, windy, 1.8, drag_table=table).apogee_m)


def refuses(mach, thrusting):
    if mach > 0.3:
        raise LookupError("no drag measured past Mach 0.3")
    return 0.5


try:
    hpr.Flight(rocket, windy, 1.8, drag=refuses)
except LookupError as error:
    print(type(error).__name__, error)
```

```text
apogee 1109 m, landed 1963 m east
True
LookupError no drag measured past Mach 0.3
```

A function that always returns the same number flies as a drag table of that number does. At 0.5
the two agree to the last digit; at some numbers, 0.3 or 0.45 say, they differ in the eleventh,
because the table's interpolation rounds its own constant.

A function that raises stops the flight, and `Flight` raises its exception unchanged, with its
type and traceback. That holds even for an exception at a trial step: the integrator tries a step,
and when a model refuses, it would normally retry with a shorter one. It asks at speeds and heights
a little past the flight's own (the landing's search, for one, asks the wind a few metres below the
ground), so a function that refuses past its data wants some margin.

A function can't be given alongside its constant counterpart: `drag` with `drag_table`, or `wind`
with `wind_speed_m_s` or `wind_from_deg`, raises `hpr.HprError`, as does something that can't be
called. So does a drag coefficient that is negative or not finite, or a wind that isn't finite; an
answer of the wrong type, such as a string, raises Python's `TypeError`.

Each call takes Python's global interpreter lock, so a flight with a Python function runs slower
than one without, and flights in other threads wait while it calls. The functions should give the
same answer to the same question, or a flight can't be repeated.

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
of several things are strings, in any case, with a space or hyphen allowed where the name has an
underscore:

| argument | the choices |
|---|---|
| a nose's or transition's `shape` | `"conical"`, `"ogive"`, `"elliptical"`, `"power_series"`, `"parabolic_series"`, `"haack"` ([Shapes](physics/shapes.md)). `parameter` is an ogive's radius ratio (1, a tangent ogive, by default), a Haack series' `C` (0, the von Kármán, by default), and a power series' exponent or a parabolic series' `K`, which those two need |
| fins' `cross_section` | `"square"` (the default), `"rounded"`, `"airfoil"` |
| a part's `position` | `"top"`, `"middle"`, `"bottom"` or `"after"` the tube it is in, then `offset_m` aft of that. A mass is at the top by default; fins and a motor tube sit flush with the tube's aft end |
| a parachute's `trigger` | `"apogee"` (the default); `"altitude"` with `altitude_m`, on the way down past that height above the pad; `"time"` with `time_s` after launch; `"motor_delay"` with `motor`, the motor's number (0, the first, by default) |
| a parachute's `canopy` | `"flat_circular"` (the default), `"conical"`, `"biconical"`, `"triconical"`, `"extended_skirt10_flat"`, `"extended_skirt14_full"`, `"hemispherical"`, `"annular"`, `"cross"`, `"flat_ribbon"`, `"conical_ribbon"`, `"ringslot"`, `"ringsail"` ([Recovery](physics/recovery.md)) |

A parachute is either a canopy `diameter_m` across, with its type's drag coefficient or your own
`drag_coefficient`, or a [drag area](glossary.md#drag-area) `cd_s_m2`, its drag coefficient
times its area in m², as RocketPy's `cd_s`.

## Every class and method

| call | what it does |
|---|---|
| `Environment(latitude_deg, longitude_deg, elevation_m, wind_speed_m_s=, wind_from_deg=, gravity=, wind=)` | the launch site, a wind, and the gravity model: `"ellipsoidal"` (the default), `"vertical"` or `"vertical_taylor"` (RocketPy's); `wind=` is a wind function, `wind(height_m) -> (east_m_s, north_m_s)` ([Drag and wind of your own](#drag-and-wind-of-your-own)) |
| `DragTable(power_off, power_on=, reference_diameter_m=)`, `DragTable.from_csv(power_off, power_on=, reference_diameter_m=)` | a drag coefficient against Mach number, from `(mach, cd)` rows or CSV files; its `cd0(mach, thrusting=)`, `has_power_on` and `reference_diameter_m` |
| `Motor.from_catalog(name, delay_s=)`, `Motor.from_file(path, delay_s=)`, `Motor.from_eng(text)`, `Motor.from_rse(text)` | a motor; its `designation`, `diameter_m`, `length_m`, `delay_s`, `total_impulse_ns`, `propellant_mass_kg` and `thrust_curve()`, two arrays |
| `Rocket(name, diameter_m)`, `Rocket.from_file(path, configuration=)` | a rocket, built or read; its `name`, `configuration` and `notes` |
| `add_nose(shape, length_m, material, wall_m=, parameter=, shoulder_length_m=, shoulder_wall_m=, capped_shoulder=, name=)` | the nose: hollow with `wall_m`, solid without; a shoulder, capped or not |
| `add_tube(length_m, wall_m, material, diameter_m=, name=)` | a body tube |
| `add_transition(length_m, aft_diameter_m, wall_m, material, shape=, parameter=, solid=, name=)` | a shoulder or boattail between tubes |
| `add_fins(count, root_chord_m=, tip_chord_m=, span_m=, sweep_m=, thickness_m=, material=, cross_section=, cant_deg=, position=, offset_m=, name=)` | trapezoidal fins |
| `add_motor_tube(length_m, inner_diameter_m, wall_m, material, overhang_m=, position=, offset_m=, name=)` | the motor tube |
| `add_mass(mass_kg, position=, offset_m=, packed_length_m=, packed_diameter_m=, name=)` | a mass, at a point or packed in a cylinder |
| `set_motor(motor)`, `add_parachute(name, ..., released_by=)` | the motor, and a parachute, cut away once parachute `released_by` is fully open |
| `mass_properties(time_s=)`, `static_margin_cal(time_s=, mach=)`, `margin(time_s=, mach=)`, `design_json()` | the mass, centre of gravity and inertia; the margin; the design as JSON |
| `Flight(rocket, environment, rail_length_m, inclination_deg=, heading_deg=, interval_s=, drag_table=, drag=)` | the flight, `drag=` a drag function, `drag(mach, thrusting) -> C_D0` ([Drag and wind of your own](#drag-and-wind-of-your-own)); its `apogee_m`, `apogee_time_s`, `max_speed_m_s`, `max_mach`, `rail_exit_speed_m_s`, `landing`, `summary`, `events`, `columns`, `series`, `flight[name]` and `to_json()` |


An argument written `name=` above can be left out, and is given by name: `add_fins`' lengths
have no default, but are named too. Each class and method has a docstring that says what its
arguments mean: `help(hpr.Rocket.add_fins)`, for example. The Rust reference for the package, [`hpr_py`](api/hpr_py/index.html), says how it is
built.

## What is not here yet

The package covers the Rust builder, not the whole library. Not yet:

- An atmosphere written in Python. Rust programs have one today
  ([Models of your own](custom-models.md#an-atmosphere)).
- Staging and mass shifts. A configuration that drops a stage under power is refused. Any other
  design of several stages flies as one stack: no stage drops away, a motor lit by another's
  burnout lights on the whole stack, one lit by a separation never does, and `rocket.notes` says
  so.
- Swapping a motor into a design read from a file, as `hpr sim --motor` does.
- Parachutes and separations stored in a design file, as above.
- The library's built-in winds that change with height (power law, log law, layers) and weather
  files fly only from Rust. From Python you can write any of them as a wind function
  ([Drag and wind of your own](#drag-and-wind-of-your-own)).
- Parts from a parts catalogue, which only Rust programs build with
  ([Parts from a catalogue](the-builder.md#parts-from-a-catalogue)).
- Type stubs, so an editor sees the docstrings but not the arguments' types.
