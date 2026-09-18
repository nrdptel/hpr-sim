# hpr-sim

**Pre-alpha, under active construction.** Nothing here is ready to rely on yet.

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

## Building

The toolchain is pinned in `rust-toolchain.toml`, so [rustup](https://rustup.rs) installs the right
version on first use.

```bash
cargo test --workspace --all-features   # unit tests
cargo xtask wasm-check                  # the pure core builds for wasm32-unknown-unknown
cargo deny check                        # dependency licenses, advisories and sources
cargo xtask site                        # build the documentation site and check its links (mdBook 0.5.4)
```

The documentation site is built from `docs/` into `target/site`; open `target/site/index.html`, or
read [`docs/start-here.md`](docs/start-here.md) on GitHub.

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
