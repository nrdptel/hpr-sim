# hpr-sim

[![vs RocketPy, same inputs: the gated metrics that pass](docs/images/census-badge.svg)](docs/accuracy.md#the-census)

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
- Online weather, terrain and live motor stock from [motor.fusionspace.co](https://motor.fusionspace.co),
  all optional. Everything works offline on macOS, Windows and Linux.
- Monte Carlo dispersion, sensitivity analysis, and optimization for competition challenges.
- Flight-log import, comparison with the simulation, and diagnosis of what went wrong.
- Later: a modern desktop/web/mobile UI with 3D flight replay.

Scope for now: commercial off-the-shelf solid rocket motors.

**Every figure this tool produces is an estimate from a model, not a measurement, and never a
go/no-go verdict.** The motor's printed data and your RSO are authoritative.

## Accuracy at a glance

Every number the validation reports compare, counted once. "Code-to-code" rows say how closely hpr
agrees with another simulator, not which of the two is right; only the real flights are
measurements. CI fails when any row moves by more than 0.1% of its bar, better or worse, until the
change is accepted with a written reason ([the census](docs/accuracy.md#the-census)).

<!-- census: written by `cargo xtask census --accept` from validation/reports/census.json; do not edit -->

| compared with | kind | held to | flights (speed) | result |
|---|---|---|---|---|
| [rocketpy 1.13.0](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): descents under a parachute | code-to-code, same inputs | gate: each metric's tolerance, at most 3% | 5 descents (5 under a parachute) | 30 of 30 gated metrics pass |
| [rocketpy 1.13.0 with upstream PRs #1188 and #1196 applied (corrections.py)](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): whole flights on the same drag | code-to-code, same inputs | gate: each metric's tolerance, at most 3% | 9 flights (8 subsonic, 1 transonic) | 142 of 142 gated metrics pass; 11 not scored, each for a written reason; apogee +0.04% to +1.21% |
| [rocketpy 1.13.0 with upstream PRs #1188 and #1196 applied (corrections.py)](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): whole flights, each code on its own drag | code-to-code, each code's own drag | target: 3% on each metric | 6 flights (5 subsonic, 1 transonic) | 75 of 102 metrics within target; apogee -7.28% to +10.30% |
| [OpenRocket 24.12](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): calm flights of OpenRocket's examples | code-to-code, each code's own model | no target; over 5% needs a written cause | 33 flights (32 subsonic, 1 transonic) | 91 of 99 differences within the bar; apogee -19.13% to +13.80% |
| [OpenRocket 24.12](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): calm flights of the private designs | code-to-code, each code's own model | no target; over 5% needs a written cause | 18 flights (13 subsonic, 5 transonic) | 54 of 54 differences within the bar; apogee -4.84% to +1.17% |
| [the teams' altimeter logs, from RocketPy 1.13.0's examples](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md): real flights | measured: the teams' altimeter logs | target: mean absolute apogee error 5% | 7 flights (3 subsonic, 4 transonic) | mean absolute apogee error 6.04% against the 5% target, outside it; apogee -8.90% to +10.40%, 2 of 7 within 5% |

<!-- census: end -->

## Building

The toolchain is pinned in `rust-toolchain.toml`, so [rustup](https://rustup.rs) installs the right
version on first use.

```bash
cargo test --workspace --all-features   # unit tests
cargo xtask wasm-check                  # the pure core builds for wasm32-unknown-unknown
cargo deny check                        # dependency licenses, advisories and sources
cargo xtask site                        # build the documentation site and check its links
```

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
