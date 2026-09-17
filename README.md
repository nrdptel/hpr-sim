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

Part of [Fusion Space](https://fusionspace.co). Licensed under either of MIT or Apache-2.0, at your
option.
