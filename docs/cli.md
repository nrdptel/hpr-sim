# The command line

`hpr` is hpr-sim's command-line tool. This page is for anyone who wants to use it from a terminal
or a script. It says what each command does, shows its output, and lists the exit codes.

Today `hpr` does five things:

- It flies a design, read from an OpenRocket file or an hpr
  [design file](glossary.md#design-file), and prints how the flight went.
- It looks up motors, from the catalog built into it or from a motor file of your own.
- It converts motor files between the two common formats.
- It re-runs hpr-sim's validation against RocketPy and checks the results against the
  published ones.
- It writes shell completion scripts.

Its other commands are registered but not available yet: each refuses and names the
[milestone](glossary.md#milestone), the step of the roadmap, that brings it
([the table below](#the-commands)).

> **How far to trust it.**
>
> - `hpr sim` prints what the library computes: a
>   [test](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/cli.rs) flies the
>   rocket of [the example below](#flying-a-design) through `hpr sim` and through the Rust library,
>   and gets identical numbers, to the last bit. How close those are to a real flight is the
>   [Accuracy](accuracy.md) page's subject. `hpr sim` flies no parachute or stage separation yet,
>   so where and how fast its rocket comes down are not predictions
>   ([what it leaves out](#what-hpr-sim-doesnt-fly-yet)).
> - `hpr motors show` works out each figure from the motor's
>   [thrust curve](glossary.md#thrust-curve) with the same code a flight uses. On all 32 bundled
>   curves, that code matches ThrustCurve.org's own statistics code to 1.8e-15, relative
>   ([Solid motors](physics/motor.md#validation)). `hpr motors list` only repeats the catalog's
>   figures as [ThrustCurve.org](glossary.md#thrustcurveorg) states them.
> - `hpr convert` keeps the thrust curve, the size and the masses. On all 32 bundled motor files,
>   converting to the other format and back gives each of them again, bit for bit, except two
>   masses written with 17 digits, each flagged by a warning
>   ([test](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-motor/src/convert.rs)).
>   OpenRocket 24.12 opens all 32 converted files and reads 31 as it reads the originals
>   ([what a conversion keeps](#what-a-conversion-keeps)). Whether RockSim opens them is not
>   checked.
> - `hpr validate` makes the check the project's automated tests make on every change, with the
>   same code. It re-flies the 20 cases compared with RocketPy; the OpenRocket comparisons and the
>   real flights it only checks against the census ([what it checks](#hpr-validate)).
> - The tests in
>   [`crates/hpr-cli/tests/cli.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/cli.rs)
>   run every command as a user would, and check each `--json` document against its
>   [published schema](#json-output).

## Running it

There is no ready-built download yet. `hpr` needs Rust and a copy of the repository, set up as
[Getting started](getting-started.md#build-it-and-fly) shows. From that copy, run it through
Cargo:

```bash
cargo run -p hpr-cli -- motors list
```

Or install it once, so that `hpr` works from any directory:

```bash
cargo install --path crates/hpr-cli --locked
hpr --help
```

## The commands

This table is written from the tool's own list of commands, so it names only what `hpr` has, and
only the files each command really reads. "Not yet" commands exit with
[status 3](#exit-codes).

<!-- cli: commands, written by `cargo xtask cli` from the registered commands; do not edit -->

| command | what it does | reads | prints | status |
|---|---|---|---|---|
| `hpr sim` | Fly a .ork or hpr design from a rail and print its flight; export its recording | `.ork`, hpr design `.json`, a motor from the bundled catalog, `.eng` or `.rse` | text, JSON, a recording as `.csv`, `.json`, `.parquet`, `.geojson` or `.kml` | available ([how to use it](cli.md#hpr-sim)) |
| `hpr validate` | Run the validation cases and check them against the committed reports and the census | a copy of the hpr-sim repository: its cases, references and committed reports | text, JSON | available ([how to use it](cli.md#hpr-validate)) |
| `hpr convert` | Convert a motor file between .eng and .rse, or write a catalog motor as either | `.eng`, `.rse`, the bundled catalog | `.eng` or `.rse`, text, JSON | available ([how to use it](cli.md#hpr-convert)) |
| `hpr motors` | Look up motors in the bundled catalog, or read a .eng or .rse motor file | `.eng`, `.rse`, the bundled catalog | text, JSON | available ([how to use it](cli.md#hpr-motors)) |
| `hpr weather` | Fetch a launch day's weather as atmosphere and wind profiles | - | - | not yet: [M5.2](decisions-and-roadmap.md#m5-2) |
| `hpr mc` | Fly a design many times, each with randomly scattered inputs | - | - | not yet: [M6.1](decisions-and-roadmap.md#m6-1) |
| `hpr optimize` | Search a design's parameters for a goal | - | - | not yet: [M6.2](decisions-and-roadmap.md#m6-2) |
| `hpr compare` | Compare a flight log with its simulation | - | - | not yet: [M7.3](decisions-and-roadmap.md#m7-3) |
| `hpr analyze` | Read a flight log and print its readings, with no design file | - | - | not yet: [M4.2d](decisions-and-roadmap.md#m4-2d) |
| `hpr diagnose` | Diagnose what went wrong in a flight from its log | - | - | not yet: [M7.4](decisions-and-roadmap.md#m7-4) |
| `hpr completions` | Print a shell completion script for hpr | - | a bash, elvish, fish, powershell or zsh script, JSON | available ([how to use it](cli.md#hpr-completions)) |

<!-- cli: end -->

## `hpr sim`

`hpr sim` flies a design from a launch rail to the ground, and prints what happened: its
[events](glossary.md#event), its [apogee](glossary.md#apogee) and top speed, its
[stability margin](glossary.md#stability-margin) as it leaves the rail, and where it came down. It
reads an [OpenRocket](glossary.md#openrocket) `.ork` file, or an hpr design file (`.json`, the
tree [Your own rocket](your-own-rocket.md) describes). It runs the same simulation code as the
Rust library, so a Rust program flying the same design gets the same numbers.

### Flying a design

This flies one of the repository's own test rockets, a small single-stage OpenRocket design. Its
file names an AeroTech H128W, which isn't among the 32 motors built into hpr, so `--motor H54`
puts the catalog's Cesaroni H54 in its motor mount instead:

<!-- cli: example `hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --motor H54`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --motor H54
pods-none (pods-none.ork), configuration 00000000-0000-4000-8000-000000000097
  1 × 168H54-10A (from the bundled catalog) in "Motor mount" (00000000-0000-4000-8000-000000000003), lit at launch
  launched at 0° N, 0° E, 0 m above sea level, from a 1.5 m vertical rail, in calm air
See the Accuracy page before trusting these numbers: https://nrdptel.github.io/hpr-sim/accuracy.html
note: the file has no recovery device, so the rocket falls from apogee on its airframe alone, on aerodynamics that hold only at small angles of attack: its landing time, speed and place, and any peak it sets in the fall, are not a prediction

event                   time     height       speed
liftoff               0.00 s      0.3 m     0.0 m/s
rail exit             0.15 s      1.8 m    21.3 m/s
burnout               3.50 s    466.0 m   136.3 m/s
apogee               11.21 s    846.1 m     0.1 m/s
ground hit           29.71 s      0.0 m    65.9 m/s
(heights are the centre of gravity's above the site; speeds are over the ground)

apogee                846.1 m above the site at 11.21 s
top speed             178.3 m/s at 2.31 s
top Mach number       0.525
rail exit speed       21.3 m/s
static margin, rail   2.69 calibres
least static margin   2.69 calibres at 0.15 s, before apogee
landing               17.4 m from the pad at 29.71 s, at 65.9 m/s: with no recovery device flown, not a prediction
```

<!-- cli: end -->

What each part says:

| lines | what they say |
|---|---|
| the first | the rocket's name, its file, and the [configuration](glossary.md#configuration) flown, by its name in quotes when the file gives one and by its id; other configurations follow on a line of their own |
| `1 × ...` | the motor, how many of it fly (a [cluster](glossary.md#cluster)'s tubes and a [pod](glossary.md#pod) set's pods each carry one), and the motor mount it sits in, by name and id |
| `launched at ...` | the site, the rail and the wind, [below](#the-launch) |
| `note:` | what the flight leaves out of the design, and which numbers that spoils |
| `warning:` | what hpr's `.ork` or motor-file reader accepted with a caveat, such as a part it left out, and what the [design's checks](physics/design.md#checks) found unusual but buildable |
| the events | each [event](glossary.md#event)'s time, the height of the [centre of gravity](glossary.md#centre-of-gravity-cg) above the launch site, and the speed over the ground; the height at liftoff isn't zero, as the rocket stands on the rail |
| the figures | the apogee; the top speed and [Mach number](glossary.md#mach-number); the speed at [rail exit](glossary.md#rail-exit-and-rail-exit-velocity); the static margin there, in [calibres](glossary.md#calibre-caliber), at Mach 0; its least value from the rail exit to apogee; and the landing |

**Expect to need `--motor`.** A `.ork` file names its motor but rarely carries its
[thrust curve](glossary.md#thrust-curve). hpr flies the file's own motor only if the file embeds
the curve or the motor is one of the 32 in its catalog, and few real designs' motors are
([`.ork` files](format/ork.md)). Otherwise `hpr sim` refuses and says why:

```text
$ hpr sim validation/fixtures/ork/pod-flights/pods-none.ork
error: configuration 00000000-0000-4000-8000-000000000097 can't be flown as the file has it: no thrust curve for H128W: ...; give a motor with --motor
```

Download the motor's `.eng` or `.rse` file from [ThrustCurve.org](glossary.md#thrustcurveorg)
and give it: `hpr sim my-rocket.ork --motor AeroTech_H128W.eng`.

**The landing is not a prediction.** `hpr sim` flies no parachute yet, so the rocket falls from
apogee on its airframe alone. hpr's aerodynamics hold only at small
[angles of attack](glossary.md#angle-of-attack), and a falling airframe turns far past them, so
where it lands, and how fast, are artifacts of the model; one test design glides tail-first far
from the pad in calm air (issue [#241](https://github.com/nrdptel/hpr-sim/issues/241)). So is a
top speed or Mach number set in the fall: a rocket that falls faster than it climbed shows its
top speed after apogee, and `hpr sim` marks it "in the fall: not a prediction" (`after_apogee` in
the JSON). The ascent, up to apogee, is what to read.

### The launch

Every flight starts from a rail, at a site, in the
[standard atmosphere](glossary.md#standard-atmosphere). Without options, the site is at sea level
at 0° N, 0° E, the rail is vertical and 1.5 m long, and the air is calm. The rail has no
friction. `hpr sim` doesn't read the launch conditions an OpenRocket file stores with its
simulations: give them with these options.

**Set `--elevation` for any real field.** The air thins with height, so the same rocket flies
higher from a high site: from 1,400 m, the rocket above climbs about 8% higher than from sea
level. Latitude changes gravity only a little: at 45° N its apogee moves by less than 0.2%. Set
`--latitude` and `--longitude` too before exporting a map, which is drawn where you say the pad
is.

| option | what it sets | default |
|---|---|---|
| `--latitude DEG` | the site's latitude, degrees north (south is negative) | 0 |
| `--longitude DEG` | the site's longitude, degrees east (west is negative) | 0 |
| `--elevation M` | the site's height above sea level, m | 0 |
| `--rail-length M` | the rail's length, from the rocket's aft end to the rail's top, m | 1.5 |
| `--inclination DEG` | the rail's angle above the horizon, degrees: 90 is vertical | 90 |
| `--heading DEG` | the direction the rail leans toward, clockwise from north, degrees | 0 |
| `--wind M_S` | a wind of this speed at every height, m/s | calm |
| `--wind-from DEG` | where the wind blows from, clockwise from north, degrees: 270 is a west wind | 0 |

OpenRocket measures its launch rod's angle from the vertical instead, so its 5° is 85 here. This
flies the same rocket from Spaceport America's field, on a rail leaning 5° into a west wind:

<!-- cli: example `hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --motor H54 --latitude 32.99 --longitude -106.97 --elevation 1400 --rail-length 3 --inclination 85 --heading 270 --wind 5 --wind-from 270`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --motor H54 --latitude 32.99 --longitude -106.97 --elevation 1400 --rail-length 3 --inclination 85 --heading 270 --wind 5 --wind-from 270
pods-none (pods-none.ork), configuration 00000000-0000-4000-8000-000000000097
  1 × 168H54-10A (from the bundled catalog) in "Motor mount" (00000000-0000-4000-8000-000000000003), lit at launch
  launched at 32.99° N, 106.97° W, 1400 m above sea level, from a 3 m rail 85° above the horizon, leaning toward 270°, in a 5 m/s wind from 270°
See the Accuracy page before trusting these numbers: https://nrdptel.github.io/hpr-sim/accuracy.html
note: the file has no recovery device, so the rocket falls from apogee on its airframe alone, on aerodynamics that hold only at small angles of attack: its landing time, speed and place, and any peak it sets in the fall, are not a prediction

event                   time     height       speed
liftoff               0.00 s      0.3 m     0.0 m/s
rail exit             0.21 s      3.3 m    30.2 m/s
burnout               3.50 s    468.6 m   144.4 m/s
apogee               11.48 s    877.4 m    11.3 m/s
ground hit           28.71 s      0.0 m    70.3 m/s
(heights are the centre of gravity's above the site; speeds are over the ground)

apogee                877.4 m above the site at 11.48 s
top speed             184.3 m/s at 2.35 s
top Mach number       0.556
rail exit speed       30.2 m/s
static margin, rail   2.71 calibres
least static margin   2.71 calibres at 0.21 s, before apogee
landing               310.4 m from the pad at 28.71 s, at 70.3 m/s: with no recovery device flown, not a prediction
```

<!-- cli: end -->

It climbs about 4% higher than the same rocket at sea level, in the first example, not the 8% the
altitude alone gives: the rocket turns into the wind as it climbs, and the rail already leans that
way, so its climb tips away from the vertical. The wind does most of it.

### The motor and the configuration

- `--config ID` flies the configuration of that id. The output's first lines list the file's
  configurations. Without it, `hpr sim` flies the file's default configuration, or its only one;
  if it can't tell which, it refuses and lists the ids.
- `--motor NAME` flies a motor of the built-in catalog, by its
  [designation](glossary.md#motor-designation) or common name (`H54`, `168H54-10A`);
  `hpr motors list` shows the catalog. `--motor FILE` flies the motor in a `.eng` or `.rse` file
  ([RASP and RockSim files](glossary.md#rasp-and-rocksim-files)). Either way, the motor goes in
  the configuration's motor mount, in place of the file's motor, and lights at launch.
- `--mount ID` says which mount `--motor` goes in, when the design has several and no
  configuration says, or to move the motor to another.
- `--accept-design-errors` flies a design whose [checks](physics/design.md#checks) find errors,
  such as a motor wider than its mount. The flight's notes then list the errors: such a rocket
  can't be built as drawn.

`hpr sim` refuses, with the reason, rather than fly something other than the design:

- with `--motor`, a configuration with motors in more than one mount, as `--motor` flies one;
- a rocket whose stages separate under power, or a motor lit by a stage's separation;
- with `--motor`, a `.ork` rocket of more than one stage, or a configuration that switches a
  stage off;
- a `.ork` rocket hpr couldn't read exactly as written, such as one with a parallel stage;
- a design whose checks find errors, unless `--accept-design-errors` is given;
- a hybrid motor in a `.rse` file, which says so: hpr flies solid motors only. A `.eng` file
  doesn't say what kind of motor it holds, so a hybrid's is flown as a solid; check the motor.

### Exporting the recording

`--export FILE` writes the flight's recording: every quantity hpr tracks, every 0.01 s (set it
with `--interval`, down to 0.001 s), and at every event. The file's extension picks the format;
repeat `--export` for several files. `hpr sim` won't write over a file it reads.

| extension | what it holds |
|---|---|
| `.csv` | one row per sample, with a header naming each column and its unit |
| `.json` | the same columns and rows |
| `.parquet` | the same, as Apache Parquet, for data tools such as pandas |
| `.geojson` | the rocket's path over the Earth, for web maps |
| `.kml` | the same path, for Google Earth |

The maps draw the whole path but mark no landing point: with no recovery device flown, the path
after apogee and where it ends are not predictions.

[Exporting a flight](exporting-a-flight.md) says what each column and field means.

### What `hpr sim` doesn't fly yet

- **Recovery devices.** A `.ork` file's parachutes and streamers are read but not flown, and an
  hpr design file holds none. The fall from apogee is not a prediction; the output's notes say so.
  A Rust program can fly them ([Getting started](getting-started.md)); `hpr sim` will, with issue
  [#240](https://github.com/nrdptel/hpr-sim/issues/240).
- **Separation.** A design's stages fly as one [stack](glossary.md#stage). A `.ork` configuration
  whose [booster](glossary.md#booster) drops away under power is refused; the library flies it,
  as [Staging](physics/staging.md#using-it-today) shows.
- **Real weather.** One wind at every height, in the standard atmosphere. A launch day's weather
  comes with `hpr weather`, in [M5.2](decisions-and-roadmap.md#m5-2).

## `hpr motors`

`hpr motors` looks motors up. The catalog built into hpr holds 32 motors, from class B to class O,
each with a public-domain thrust curve from ThrustCurve.org
([Solid motors](physics/motor.md#the-bundled-motors) says how they were chosen). For any other
motor, download its `.eng` or `.rse` file ([RASP and RockSim files](glossary.md#rasp-and-rocksim-files))
from ThrustCurve.org and show that.

### Listing the catalog

`hpr motors list` lists the catalog. Three filters narrow it, and each one given must match:

- `--class` takes an [impulse class](glossary.md#impulse-class), such as `J`.
- `--diameter` takes a casing diameter in millimetres, such as `54`, and matches within 0.5 mm.
- `--manufacturer` takes a maker as the `maker` column spells it (`AeroTech`, `Cesaroni`, `Loki`,
  `Estes`, `Quest`, `AMW`) or its full name (`Cesaroni Technology`), in any case.

A class or diameter the catalog has no motor for lists none, and exits with 0. A class that
doesn't exist, a diameter that isn't a positive number, or a maker the catalog doesn't know is
refused with status 1: a misspelt maker would otherwise look like a maker with no motors.

The figures are ThrustCurve.org's, as the catalog states them. In the `delays` column, `P` means
plugged: the motor has no ejection charge ([ejection delay](glossary.md#ejection-delay)).

<!-- cli: example `hpr motors list --class J`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr motors list --class J
3 motors from ThrustCurve.org API v1, captured 2026-09-17; figures as ThrustCurve.org states them.

designation   maker     class  dia mm  len mm  impulse N·s  avg N  burn s  delays
J450DM        AeroTech  J          54     359       1055.0  465.0    2.27  6,8,10,12,14
1266J760-19A  Cesaroni  J          54     329       1265.7  757.7    1.67  9,10,11,12,13,14,15,16,17,18,19
J300LR        Loki      J          54     327       1208.0  297.0    4.10  P
```

<!-- cli: end -->

### A motor's figures

`hpr motors show` takes a motor's name or a file's path. A name can be the full
[designation](glossary.md#motor-designation) (`1266J760-19A`) or the common name (`J760`), in any
case, with or without spaces and hyphens. When several motors share a name, it shows each of them.
An argument ending in `.eng` or `.rse` is read as a file; anything else is looked up by name.

It works each figure out from the thrust curve, the list of time and thrust points in the motor's
file:

- The [total impulse](glossary.md#total-impulse) is the area under the curve, with its points
  joined by straight lines. It sets the [impulse class](glossary.md#impulse-class).
- The [burn time](glossary.md#burn-time) is measured the [NFPA 1125](glossary.md#nfpa-1125) way:
  from when the thrust first reaches 5% of its peak to when it last falls back to it.
- The [average thrust](glossary.md#average-thrust) is the total impulse divided by that burn time.
- The masses and the casing size are the catalog's, or the file's for a file.

For a catalog motor, the last line sets ThrustCurve.org's stated figures beside hpr's. Total
impulse, average thrust and burn time agree within 1% for every catalog motor, because that is how
the 32 were chosen ([Solid motors](physics/motor.md#the-bundled-motors)). Peak thrust is not held
to that. hpr's peak is the curve file's highest point, and across the catalog it runs from 16.7%
below ThrustCurve.org's (Cesaroni 26E31-15A) to 2.1% above (Loki M1378LR); a
[test](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/cli.rs) pins that range.

<!-- cli: example `hpr motors show J760`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr motors show J760
1266J760-19A (Cesaroni Technology), from the bundled catalog
  impulse class    J
  total impulse    1267.3 N·s
  average thrust   758.2 N
  peak thrust      938.6 N
  burn time        1.67 s, from 0.001 s to 1.672 s (NFPA 1125, 5% of peak)
  propellant       576.0 g of 1076.8 g loaded
  casing           54 mm across, 329 mm long
  delays           9 s, 10 s, 11 s, 12 s, 13 s, 14 s, 15 s, 16 s, 17 s, 18 s, 19 s
  ThrustCurve.org  J class, 1265.7 N·s, 757.7 N average, 937.3 N peak, 1.67 s burn
```

<!-- cli: end -->

A motor file shows every motor in it, as one `.eng` or `.rse` file can hold several. The
repository holds the 32 catalog motors' files to try, such as:

```bash
hpr motors show crates/hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000724.eng
```

Some motor data is ambiguous. A delay of `0` can mean an ejection charge at burnout, or a plugged
motor with no charge. hpr shows it as "0 (at burnout, or plugged)" and ends the output with a
warning; the Estes F15 in the catalog is one example. The warning's "RASP spec" is the `.eng`
format's description ([RASP and RockSim files](glossary.md#rasp-and-rocksim-files)).

## `hpr convert`

`hpr convert` writes a motor as a RASP `.eng` file or a RockSim `.rse` file, the two formats
ThrustCurve.org offers ([RASP and RockSim files](glossary.md#rasp-and-rocksim-files)). Give it the
file to read and the file to write; the extensions pick the formats. It reads a motor file, or a
motor of the bundled catalog by its name:

```bash
hpr convert H170M H170M.eng
```

A catalog motor is written with the size and masses the catalog gives, which are the ones hpr
flies; where its curve file's header says otherwise, a warning says so. An existing file of the
output's name is replaced, but never the file being read: to rewrite a `.eng` file in hpr's
layout, convert it to a new `.eng` file name. Here the Estes F15's `.rse` file, from the catalog's
curves, becomes a `.eng` file:

<!-- cli: example `hpr convert crates/hpr-motor/data/thrustcurve/curves/5f923edb1bca5800041716ab.rse F15.eng`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr convert crates/hpr-motor/data/thrustcurve/curves/5f923edb1bca5800041716ab.rse F15.eng
read   5f923edb1bca5800041716ab.rse
wrote  F15.eng: F15
warning: line 3: engine "F15": delay 0 in "0,4,6,8" is ambiguous: the RASP spec means an ejection charge with no delay, but files mostly mean plugged
warning: F15: a .eng maker is one word, so "Estes Industries, Inc." is written "Estes_Industries,_Inc."
warning: F15: dropped the comments' blank lines and the spaces that end their lines: a .eng comment is one line of text
warning: F15: dropped Type, auto-calc-mass, auto-calc-cg, avgThrust, peakThrust, throatDia, exitDia, Itot, burn-time, massFrac, Isp, m, cg: a .eng file has no place for them. hpr doesn't use them for a solid motor: it works the mass and centre of gravity out from the curve and the masses
```

<!-- cli: end -->

### What a conversion keeps

Both formats give a motor's name, maker, diameter and length, its loaded and propellant masses,
its delays, and its [thrust curve](glossary.md#thrust-curve). The curve, the size and the masses
are kept: converting a file and converting the result back gives each of them again, bit for bit.
The formats write some things differently, and `hpr convert` translates:

| | `.eng` | `.rse` |
|---|---|---|
| masses | kilograms | grams |
| delays | `6-10-14` | `6,10,14` |
| a plugged motor's delay | `P` | `1000` |
| the curve's first point, zero thrust at ignition | left out | written |
| a name or maker of several words | joined by `_` | as it is |

Masses move between kilograms and grams by moving the decimal point, not by multiplying, which
would round: `0.0041` kg times 1000 is `4.1000000000000005` in a computer's arithmetic, while
moving the point gives `4.1`. A mass written with 16 or 17 digits may still change in its last
digit; a warning says so.

Some things come back written differently, though they mean the same:

- Delays spelled `p`, `1000` or with spaces come back in the table's spelling. hpr reads them as
  the same delays.
- A name or maker of several words comes back with `_` between the words. A `.eng` header is
  seven fields split by spaces, and OpenRocket refuses more; a warning says so.
- Comments lose their blank lines and the spaces ending their lines, and comments after the last
  motor of a `.eng` file are dropped; a warning says so.

A `.rse` file also gives figures a `.eng` file has no place for: the motor's type, its total
impulse, average and peak thrust, burn time, mass fraction,
[specific impulse](glossary.md#specific-impulse), nozzle throat and exit diameters, two flags
saying whether RockSim works the mass and centre of gravity out itself, and the mass and centre of
gravity at each point of the curve. Going to `.eng`, they are dropped, and a warning names them.
hpr doesn't use them for a solid motor: it works the mass and centre of gravity out from the curve
and the masses, whichever format it reads. Going to `.rse`, they are filled in the way
ThrustCurve.org's `.rse` files are: the total impulse by adding up the curve, the remaining
propellant falling in step with the impulse delivered, the centre of gravity at half the length,
both flags set, and the type `unspecified`. The rules, and the counts of real files behind them,
are in the [`.rse` format notes](format/rse.md#writer-policy-strict-round-trip-stable).

`hpr convert` refuses to write a hybrid motor as `.eng`, which couldn't mark it as a hybrid (hpr
models solid motors only). A `.eng` header must give delays, so a `.rse` motor without them is
refused until `--delays` gives them, such as `--delays P` for a plugged motor.

**Other programs.** OpenRocket 24.12 opens all 32 files `hpr convert` writes from the bundled
curves, and reads 31 of them as it reads the originals. The other is a `.eng` file whose delay
is written `1000`: OpenRocket reads no delay from it, and a plugged motor from the converted
`.rse`, as hpr reads both. A [script](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/openrocket/motor_files.py)
repeats the check; the project's automated tests don't run it, as they have no OpenRocket.
Whether RockSim opens the files is not checked.

## `hpr validate`

`hpr validate` re-runs hpr-sim's [validation cases](glossary.md#validation-case): the 20 cases
that fly a rocket in hpr and compare it with [RocketPy](glossary.md#rocketpy), another simulator,
[metric](glossary.md#metric) by metric ([Accuracy](accuracy.md) explains them). It then checks
three things, and fails, with exit status 1, if any is wrong:

| check | what it catches |
|---|---|
| every scored metric within its [tolerance](glossary.md#tolerance) | a change that made hpr less accurate than its [gate](glossary.md#gate-and-target) allows |
| the run reproduces the report committed to the repository, `validation/reports/latest.md` and `.json` | a published report that no longer says what the code computes |
| the committed reports hold to the [accepted accuracy census](glossary.md#accuracy-census) | a published result that got worse without anyone accepting it in writing |

The census also holds the comparisons with OpenRocket and with real flights on the
[Accuracy](accuracy.md) page. `hpr validate` doesn't fly those again, as they need OpenRocket and
private files; it checks that their published numbers still match the census.

It is the check the project's automated tests make on every change, `cargo xtask validate
--check`, run by the same code, so the two pass and fail together. It writes nothing. It needs a
copy of the repository, as the cases, their references and the reports are files in it. Run it
from the copy's folder, or name the folder with `--root`, with `hpr` built from the same commit as
the copy: a newer copy's reports can differ from what an older `hpr` computes.

```bash
cargo run -p hpr-cli -- validate
```

It prints a line per case, then the totals, the census and the report's check. A case with a
tolerance shows its largest scored difference, and names any metric it doesn't score. A case in
[predicted mode](glossary.md#same-drag-and-predicted-mode) flies with hpr's own drag instead of
RocketPy's, so it is judged against a target, not a tolerance; it shows how many metrics fall
within their target. A [known gap](accuracy.md#known-gaps) is a case hpr refuses to fly, with its
reason. In outline:

```text
<case>: <n> metric(s), worst scored <±x.xx>%[, not scored: <metrics>]
<case>: predicted, <n> metric(s) reported, <k> within target, largest <metric> <±x.xx>%
<case>: known gap, <n> metric(s) not scored: hpr <why it refuses>
validate: <cases> case(s), <metrics> metric(s) (<n> not scored, <n> predicted, against a target, <n> outside it), ok
census: the committed reports hold to the accepted census (<rows> rows)
validate: the committed report reproduces
```

The totals line counts the metrics a case doesn't score, the predicted ones, and those outside
their target, which never fail the run. When the check fails, the exit status is 1, standard
error lists each reason, the totals line ends in `FAILED` if a metric is outside its tolerance,
and "the committed report reproduces" is not printed. The run takes a few seconds.

## JSON output

With `--json`, a command prints exactly one [JSON](https://www.json.org) document on standard
output, and nothing on standard error. That holds when the command fails, too. A failure prints an
error document, except that `hpr validate` prints its own document when its check fails, with
`passed` set to `false` and the reasons in `problems`. Each document has
a published [JSON Schema](https://json-schema.org), which describes its fields and units:

| output | schema |
|---|---|
| `hpr sim` | [`sim.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/sim.schema.json) |
| `hpr motors list` | [`motors-list.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/motors-list.schema.json) |
| `hpr motors show` | [`motors-show.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/motors-show.schema.json) |
| `hpr convert` | [`convert.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/convert.schema.json) |
| `hpr validate` | [`validate.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/validate.schema.json) |
| `hpr completions` | [`completions.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/completions.schema.json) |
| any failure | [`error.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/error.schema.json) |

Units are SI, and each field's name says its unit: `total_impulse_ns` is in newton-seconds and
`diameter_m` in metres. The exceptions say so in their names too: `hpr sim` gives angles in
degrees (`latitude_deg`) and margins in calibres (`margin_cal`), and `hpr motors list` keeps the
catalog's millimetres (`diameter_mm`). Numbers are not rounded, so a converted value can end in
digits such as `0.0036000000000000003`; a motor's figures carry no more precision than its curve
file.

hpr is pre-alpha, so the fields may still change. The schemas are published beside the code, and a
change to a document changes its schema in the same commit.

<!-- cli: example `hpr motors show B4 --json`; written by `cargo xtask cli`; do not edit -->

```text
$ hpr motors show B4 --json
{
  "motors": [
    {
      "name": "B4",
      "manufacturer": "Quest Aerospace",
      "source": {
        "kind": "catalog",
        "curve_url": "https://www.thrustcurve.org/simfiles/5f4294d20002e9000000088e/",
        "format": "eng"
      },
      "diameter_m": 0.018,
      "length_m": 0.081,
      "propellant_mass_kg": 0.0036000000000000003,
      "loaded_mass_kg": 0.0198,
      "total_impulse_ns": 4.892141999999999,
      "impulse_class": "B",
      "average_thrust_n": 4.403753380057823,
      "peak_thrust_n": 7.2,
      "burn_time_s": 1.110902808988764,
      "burn_start_s": 0.01745,
      "burn_end_s": 1.128352808988764,
      "curve_end_s": 1.184,
      "delays": [
        {
          "kind": "seconds",
          "value": 4.0
        },
        {
          "kind": "seconds",
          "value": 6.0
        }
      ],
      "stated": {
        "impulse_class": "B",
        "total_impulse_ns": 4.86,
        "average_thrust_n": 4.4,
        "max_thrust_n": 7.5,
        "burn_time_s": 1.1
      }
    }
  ],
  "warnings": []
}
```

<!-- cli: end -->

A failure prints an error document. Its `kind` matches the exit status, and `milestone` is set
only for a command not available yet. Here `hpr sim` refuses the test rocket above without
`--motor`, as hpr has no curve for its motor:

<!-- cli: example `hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --json`, exits 1; written by `cargo xtask cli`; do not edit -->

```text
$ hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --json
{
  "error": {
    "kind": "input",
    "message": "configuration 00000000-0000-4000-8000-000000000097 can't be flown as the file has it: no thrust curve for H128W: no embedded curve, and no motor of that manufacturer and designation in the bundled catalog; give a motor with --motor",
    "command": "sim",
    "milestone": null
  }
}
$ echo $?
1
```

<!-- cli: end -->

## Exit codes

| status | `kind` in JSON | meaning |
|---|---|---|
| 0 | - | The command did what was asked. |
| 1 | `input` | An input was missing, unreadable or refused, such as a motor the catalog doesn't have. |
| 2 | `usage` | The command line was wrong: an unknown command or option, or a missing argument. |
| 3 | `not_available` | The command is registered, but the milestone that brings it hasn't come yet. |

`hpr validate` also exits with 1 when its check fails. With `--json` it then prints its own
document, with `passed` set to `false`, rather than an error document, so it has no `kind`.

`hpr --help` and `hpr --version` print text and exit with 0, even with `--json`.

## `hpr completions`

`hpr completions` writes a script that lets your shell complete `hpr`'s commands and options when
you press Tab. Save it where your shell looks for completions:

| shell | command |
|---|---|
| bash | `hpr completions bash > ~/.local/share/bash-completion/completions/hpr` |
| zsh | `hpr completions zsh > ~/.zfunc/_hpr`, with `fpath+=~/.zfunc` before `compinit` in `~/.zshrc` |
| fish | `hpr completions fish > ~/.config/fish/completions/hpr.fish` |
| PowerShell | `hpr completions powershell >> $PROFILE` |
| elvish | `hpr completions elvish >> ~/.config/elvish/rc.elv` |

## What it leaves out

- **Parachutes and stage separation.** `hpr sim` flies the stack whole and brings it down with
  no recovery device ([what it doesn't fly yet](#what-hpr-sim-doesnt-fly-yet)).
- **Commands still to come.** Reading a flight log (`hpr analyze`) comes in
  [M4.2d](decisions-and-roadmap.md#m4-2d).
- **Design files aren't converted.** `hpr convert` converts motor files only. Writing a design
  as an OpenRocket file is [M3.2](decisions-and-roadmap.md#m3-2)'s work.
- **`hpr validate` needs the repository.** The cases and their reference results are files in it,
  not part of the tool. It re-flies only the RocketPy comparisons.
- **RockSim is unchecked.** OpenRocket opens the files `hpr convert` writes; whether RockSim
  does is not checked.
- **Only 32 motors are built in.** Any other motor needs its `.eng` or `.rse` file. `hpr` never
  goes online to fetch one.
- **No ready-built program.** `hpr` is built from source with Rust; downloads for macOS, Windows
  and Linux wait until the project publishes releases.
- **The text output is for people.** Its layout may change between versions; scripts should read
  `--json`, whose schemas are published.
