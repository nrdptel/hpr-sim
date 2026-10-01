# hpr-sim

[![vs RocketPy, same inputs: the gated metrics that pass](docs/images/census-badge.svg)](docs/accuracy.md#the-census)
[![real flights: the mean apogee error against its target](docs/images/real-flights-badge.svg)](docs/accuracy.md#real-flights)

**Pre-alpha, under active construction.** Nothing here is ready to rely on yet.

**Documentation: <https://nrdptel.github.io/hpr-sim/>**, a searchable guide with the API
reference (rustdoc) beside it. The same pages also read here on GitHub: [Start here](docs/start-here.md) says what works, what
doesn't and how far to trust it; [Getting started](docs/getting-started.md) flies a first rocket;
[Accuracy](docs/accuracy.md) has every validation result. `cargo xtask site` builds the site on
your machine.

hpr-sim is an open-source flight simulator for hobby and high-power rockets, written in Rust. It
is a library first: a 6-DOF simulator in the spirit of [RocketPy](https://github.com/RocketPy-Team/RocketPy),
built to be embedded in other tools through Rust, a CLI, Python, C and WebAssembly. It is also
built to be checked: every model is cited and tested, and the simulator is measured against
independent simulators and real flight data, with the results published.

Planned:

- 6-DOF simulation with variable mass, detailed aerodynamics (subsonic to supersonic), real
  atmospheres and winds, and recovery with drift.
- Designing rockets from a parts catalog; import and export of OpenRocket, RockSim, RASAero and
  RocketPy files; a new open design format.
- Online weather (Open-Meteo's forecasts, weather-balloon soundings and NOAA's GFS and RAP, in the
  library and `hpr weather` today),
  [a site's elevation](https://nrdptel.github.io/hpr-sim/elevation.html) (Open-Meteo's, or from a
  GeoTIFF terrain file of your own, in the library today) and
  [live motor stock and prices](https://nrdptel.github.io/hpr-sim/motor-stock.html) from
  [motor.fusionspace.co](https://motor.fusionspace.co), matched to ThrustCurve.org's thrust
  curves (in the library today),
  all optional. Everything works offline on macOS, Windows and Linux.
- Monte Carlo dispersion, sensitivity analysis, and optimization for competition challenges.
- Flight-log import, comparison with the simulation, and diagnosis of what went wrong.
- Later: a modern desktop/web/mobile UI with 3D flight replay.

Scope for now: commercial off-the-shelf solid rocket motors.

**Every figure this tool produces is an estimate from a model, not a measurement, and never a
go/no-go verdict.** The motor's printed data and your RSO are authoritative.

## Python

`crates/hpr-py` builds `hpr`, a Python package over the library's builder: build a rocket part by
part or read a design file, fly it, and get its recording as NumPy arrays. It isn't on PyPI yet;
[Python](https://nrdptel.github.io/hpr-sim/python.html) says how to build and use it.

## The command line

`hpr` is the command-line tool; [The command line](https://nrdptel.github.io/hpr-sim/cli.html)
shows each command's output. This table is generated from the commands the tool registers, so it
lists only what exists: a "not yet" command refuses, with exit status 3, until its milestone.

<!-- cli: commands, written by `cargo xtask cli` from the registered commands; do not edit -->

| command | what it does | reads | prints | status |
|---|---|---|---|---|
| `hpr sim` | Fly a .ork, an .hpr or .hprz design, or a rocket's .json from a rail and print its flight; export its recording | `.ork`, `.hpr` or `.hprz`, a rocket's `.json`, a motor from the bundled catalog, `.eng` or `.rse` | text, JSON, a recording as `.csv`, `.json`, `.parquet`, `.geojson` or `.kml` | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-sim)) |
| `hpr validate` | Run the validation cases and check them against the committed reports and the census | a copy of the hpr-sim repository: its cases, references and committed reports | text, JSON | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-validate)) |
| `hpr convert` | Convert a motor file between .eng and .rse, or a catalog motor to either; or a design between .ork, .hpr and .hprz | `.eng`, `.rse`, the bundled catalog, a design as `.ork`, `.hpr` or `.hprz` | `.eng` or `.rse`, `.ork`, `.hpr` or `.hprz`, text, JSON | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-convert)) |
| `hpr motors` | Look up motors in the bundled catalog, or read a .eng or .rse motor file | `.eng`, `.rse`, the bundled catalog | text, JSON | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-motors)) |
| `hpr weather` | Fetch a launch day's weather, or read a weather file, as a profile of air and wind | Open-Meteo, a University of Wyoming sounding, GFS or RAP, fetched or saved, a whole GFS file, an ERA5 `.nc` | text, JSON, a profile as `.json` | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-weather)) |
| `hpr mc` | Fly a design many times, each with randomly scattered inputs | - | - | not yet: [M6.1](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m6-1) |
| `hpr optimize` | Search a design's parameters for a goal | - | - | not yet: [M6.2](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m6-2) |
| `hpr compare` | Compare a flight log with its simulation | - | - | not yet: [M7.3](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m7-3) |
| `hpr analyze` | Read a flight log and print its readings, with no design file | a PerfectFlite `.pf2` flight log | text, JSON | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-analyze)) |
| `hpr diagnose` | Diagnose what went wrong in a flight from its log | - | - | not yet: [M7.4](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m7-4) |
| `hpr completions` | Print a shell completion script for hpr | - | a bash, elvish, fish, powershell or zsh script, JSON | available ([how to use it](https://nrdptel.github.io/hpr-sim/cli.html#hpr-completions)) |

<!-- cli: end -->

## Accuracy at a glance

What hpr has been compared with, and how it came out. A "code-to-code" line says how closely hpr
agrees with another simulator, not which of the two is right; only the real flights are
measurements, and on those hpr misses its target. Each number behind this table is also a
check: CI fails when one moves, better or worse, until the change is accepted with a written
reason. CI flies the RocketPy comparisons again on every change; the OpenRocket and real-flight
ones need files CI doesn't have, so it holds their committed numbers
([the census](docs/accuracy.md#the-census)).

<!-- census: written by `cargo xtask census --accept` from validation/reports/census.json; do not edit -->

| compared with | kind | held to | flights (speed) | result |
|---|---|---|---|---|
| [RocketPy 1.13.0](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): descents under a parachute | code-to-code, same inputs | gate: each metric's tolerance, at most 3% | 5 descents | 30 of 30 gated metrics pass |
| [RocketPy 1.13.0, patched](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): whole flights on the same drag | code-to-code, same inputs | gate: each metric's tolerance, at most 3% | 9 flights (8 subsonic, 1 transonic) | 142 of 142 gated metrics pass; apogee +0.04% to +1.21%; 11 not scored, each for a written reason |
| [RocketPy 1.13.0, patched](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): whole flights, each code on its own drag | code-to-code, each code's own drag | target: 3% on each metric, reported, not enforced | 6 flights (5 subsonic, 1 transonic) | 75 of 102 metrics within target; apogee -7.28% to +10.30% |
| [OpenRocket 24.12](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): calm flights of OpenRocket's examples | code-to-code, each code's own model | no target; an apogee more than 5% off needs a written cause | 34 flights (33 subsonic, 1 transonic); 23 more not flown | apogee -19.13% to +13.80%; apogee within 5% on 27 of 34, largest speed within 5% on 31 of 34, margin within 0.5 calibres on 33 of 34, launch mass within 1% on 34 of 34, mass at rod clearance within 1% on 33 of 34, centre of mass at rod clearance within 0.5 calibres on 34 of 34 |
| [OpenRocket 24.12](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): calm flights of the private designs | code-to-code, each code's own model | no target; an apogee more than 5% off needs a written cause | 35 flights (28 subsonic, 6 transonic, 1 supersonic); 2 more not flown | apogee -4.84% to +13.60%; apogee within 5% on 34 of 35, largest speed within 5% on 34 of 35, margin within 0.5 calibres on 35 of 35, launch mass within 1% on 35 of 35, mass at rod clearance within 1% on 35 of 35, centre of mass at rod clearance within 0.5 calibres on 35 of 35 |
| [the teams' altimeter logs](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): real flights | measured | target: mean absolute apogee error 5% | 7 flights (3 subsonic, 4 transonic) | mean absolute apogee error 6.04% (target 5%, missed); apogee -8.90% to +10.40%; apogee within 5% on 2 of 7, climb's RMS height error within 3% of apogee on 2 of 7 |

<!-- census: end -->

"Patched" means RocketPy 1.13.0 run with two fixes from RocketPy's own pull requests, which
correct the point it takes the burn's turning moments about
([why](docs/accuracy.md#whole-flights-against-rocketpy)).

## Building

The toolchain is pinned in `rust-toolchain.toml`, so [rustup](https://rustup.rs) installs the right
version on first use.

```bash
cargo test --workspace --all-features   # unit tests
cargo xtask wasm-check                  # the pure core builds for wasm32-unknown-unknown
cargo deny check                        # dependency licenses, advisories and sources
cargo xtask site                        # build the documentation site and check its links
```

`cargo test` also runs the design format's generated TypeScript and Python readers, so it needs
Node.js 22.18 or later and Python 3.11 or later on the path
([TypeScript and Python](docs/format/hpr.md#typescript-and-python)).

`cargo xtask site` needs mdBook 0.5: `cargo install mdbook --version 0.5.4 --locked` (the version
CI uses) or `brew install mdbook`. The site is built from `docs/` into `target/site`: open
`target/site/index.html`, or read [`docs/start-here.md`](docs/start-here.md) on GitHub.

Validation work uses a local reference library (other simulators, papers, motor data), pinned in
`validation/refs.lock.toml` and downloaded into the gitignored `refs/`. Fetching it needs `git`,
`curl` and [uv](https://docs.astral.sh/uv/); the OpenRocket oracle also needs Java 17+.

```bash
cargo xtask refs fetch    # fetch everything to its pinned state (safe to rerun)
cargo xtask refs verify   # re-hash everything against the pins
cargo xtask refs doctor   # which tools and oracles work on this machine
```

The workspace crates live under `crates/`; `docs/ARCHITECTURE.md` describes what each one is for.
The roadmap is in `docs/ROADMAP.md` and progress in `docs/STATUS.md`.

## License

Part of [Fusion Space](https://fusionspace.co). Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option. Third-party sources and their licenses are listed in
[THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
