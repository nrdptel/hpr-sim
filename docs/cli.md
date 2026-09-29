# The command line

`hpr` is hpr-sim's command-line tool. This page is for anyone who wants to use it from a terminal
or a script. It says what each command does, shows its output, and lists the exit codes.

Today `hpr` does two things. It looks up motors, from the catalog built into it or from a motor
file of your own, and it writes shell completion scripts. Its other commands, starting with flying
a design, are registered but not available yet: each refuses and names the
[milestone](glossary.md#milestone), the step of the roadmap, that brings it
([the table below](#the-commands)). Until then, flights are flown from Rust, as
[Getting started](getting-started.md) shows.

> **How far to trust it.** `hpr motors show` works out each figure from the motor's
> [thrust curve](glossary.md#thrust-curve) with the same code a flight uses. On all 32 bundled
> curves, that code matches ThrustCurve.org's own statistics code to 1.8e-15, relative
> ([Solid motors](physics/motor.md#validation)). `hpr motors list` only repeats the catalog's
> figures as [ThrustCurve.org](glossary.md#thrustcurveorg) states them. The tests in
> [`crates/hpr-cli/tests/cli.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-cli/tests/cli.rs)
> run every command as a user would, and check each `--json` document against its
> [published schema](#json-output).

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
| `hpr sim` | Fly a design and print its flight summary | - | - | not yet: [M4.2b](decisions-and-roadmap.md#m4-2b) |
| `hpr validate` | Run the committed validation cases and report them | - | - | not yet: [M4.2c](decisions-and-roadmap.md#m4-2c) |
| `hpr convert` | Convert motor and design files between formats | - | - | not yet: [M4.2c](decisions-and-roadmap.md#m4-2c) |
| `hpr motors` | Look up motors in the bundled catalog, or read a .eng or .rse motor file | `.eng`, `.rse`, the bundled catalog | text, JSON | available ([how to use it](cli.md#hpr-motors)) |
| `hpr weather` | Fetch a launch day's weather as atmosphere and wind profiles | - | - | not yet: [M5.2](decisions-and-roadmap.md#m5-2) |
| `hpr mc` | Fly a design many times, each with randomly scattered inputs | - | - | not yet: [M6.1](decisions-and-roadmap.md#m6-1) |
| `hpr optimize` | Search a design's parameters for a goal | - | - | not yet: [M6.2](decisions-and-roadmap.md#m6-2) |
| `hpr compare` | Compare a flight log with its simulation | - | - | not yet: [M7.3](decisions-and-roadmap.md#m7-3) |
| `hpr analyze` | Read a flight log and print its readings, with no design file | - | - | not yet: [M4.2d](decisions-and-roadmap.md#m4-2d) |
| `hpr diagnose` | Diagnose what went wrong in a flight from its log | - | - | not yet: [M7.4](decisions-and-roadmap.md#m7-4) |
| `hpr completions` | Print a shell completion script for hpr | - | a script for bash, elvish, fish, powershell, zsh, JSON | available ([how to use it](cli.md#hpr-completions)) |

<!-- cli: end -->

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
to that: for the AeroTech I175WS, hpr's is 3% below ThrustCurve.org's.

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

## JSON output

With `--json`, a command prints exactly one [JSON](https://www.json.org) document on standard
output, and nothing on standard error. That holds when the command fails, too. Each document has
a published [JSON Schema](https://json-schema.org), which describes its fields and units:

| output | schema |
|---|---|
| `hpr motors list` | [`motors-list.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/motors-list.schema.json) |
| `hpr motors show` | [`motors-show.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/motors-show.schema.json) |
| `hpr completions` | [`completions.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/completions.schema.json) |
| any failure | [`error.schema.json`](https://github.com/nrdptel/hpr-sim/blob/main/schema/cli/error.schema.json) |

Units are SI, and each field's name says its unit: `total_impulse_ns` is in newton-seconds and
`diameter_m` in metres. The one exception is `hpr motors list`, which keeps the catalog's
millimetres, and says so in its field names. Numbers are not rounded, so a converted value can
end in digits such as `0.0036000000000000003`; they carry no more precision than the curve file.

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

A failure prints an error document. Its `kind` matches the exit status:

<!-- cli: example `hpr sim rocket.ork --json`, exits 3; written by `cargo xtask cli`; do not edit -->

```text
$ hpr sim rocket.ork --json
{
  "error": {
    "kind": "not_available",
    "message": "hpr sim is not available yet: it arrives with milestone M4.2b (https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m4-2b)",
    "command": "sim",
    "milestone": "M4.2b"
  }
}
$ echo $?
3
```

<!-- cli: end -->

## Exit codes

| status | `kind` in JSON | meaning |
|---|---|---|
| 0 | - | The command did what was asked. |
| 1 | `input` | An input was missing, unreadable or refused, such as a motor the catalog doesn't have. |
| 2 | `usage` | The command line was wrong: an unknown command or option, or a missing argument. |
| 3 | `not_available` | The command is registered, but the milestone that brings it hasn't come yet. |

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

- **No flights yet.** Flying a design is `hpr sim`, which arrives in
  [M4.2b](decisions-and-roadmap.md#m4-2b). Checking the validation cases (`hpr validate`) and
  converting files (`hpr convert`) come in [M4.2c](decisions-and-roadmap.md#m4-2c), and reading a
  flight log (`hpr analyze`) in [M4.2d](decisions-and-roadmap.md#m4-2d).
- **Only 32 motors are built in.** Any other motor needs its `.eng` or `.rse` file. `hpr` never
  goes online to fetch one.
- **No ready-built program.** `hpr` is built from source with Rust; downloads for macOS, Windows
  and Linux wait until the project publishes releases.
- **The text output is for people.** Its layout may change between versions; scripts should read
  `--json`, whose schemas are published.
