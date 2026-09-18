# The API reference

**The API reference documents hpr-sim's code: every public type, function and constant, generated
from the source by rustdoc, Rust's documentation tool.** Use it when you write a program with
hpr-sim, as [Getting started](getting-started.md) does. This page says which crate holds what, and
links each crate's reference. The library is pre-alpha, so its interface can change at any time,
and a simpler one is planned ([M4.1][roadmap]).

The reference is built from the same source that CI compiles on every change, so it describes the
code as it is. A broken link inside it fails CI. Physics items give their equation and cite their
source, as the model pages here do. Each crate's front page links back to the pages here that
explain its models.

## The crates

hpr-sim is split into [crates](glossary.md#crate), Rust's packages, so that a program takes only
what it needs, and so that the models, which do no file or network I/O, also build for the web.
Seven crates hold code today:

| crate | what it holds | the guide's pages |
|---|---|---|
| [`hpr_core`](api/hpr_core/index.html) | Vectors and quaternions, frames, the Earth's shape and gravity, interpolation tables and numerical integration of functions | [Frames](physics/frames.md), [Geodesy](physics/geodesy.md), [Gravity](physics/gravity.md), [Interpolation tables](physics/interpolation.md), [Adaptive quadrature](physics/quadrature.md) |
| [`hpr_atmos`](api/hpr_atmos/index.html) | The standard atmosphere, humidity, soundings, wind profiles and turbulence | [Atmosphere](physics/atmosphere.md), [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| [`hpr_motor`](api/hpr_motor/index.html) | Solid motors: thrust curves, mass and inertia through the burn, `.eng` and `.rse` files, and the 32 bundled curves | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| [`hpr_design`](api/hpr_design/index.html) | The rocket: its tree of parts, their shapes and materials, mass properties and design checks | [The design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| [`hpr_aero`](api/hpr_aero/index.html) | Aerodynamics: normal force, centre of pressure, drag and drag tables | [Aerodynamics](physics/aero.md) |
| [`hpr_sim`](api/hpr_sim/index.html) | The flight: the launch rail, the equations of motion, time integration, events and recovery | [How a flight is simulated](how-a-flight-is-simulated.md), [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md), [Recovery](physics/recovery.md) |
| [`hpr_validate`](api/hpr_validate/index.html) | The validation harness: cases, reference data, metrics and reports | [Accuracy](accuracy.md), [Checking a claim](checking-a-claim.md) |

The other nine crates are placeholders for planned work. Each has a front page that says what it
will hold, and nothing else yet:

| crate | what it will hold | planned in |
|---|---|---|
| [`hpr`](api/hpr/index.html) | One crate to depend on, with a builder for environments, motors, rockets and flights | [M4.1][roadmap] |
| [`hpr_format`](api/hpr_format/index.html) | hpr's own design file format | [M3.3][roadmap] |
| [`hpr_io`](api/hpr_io/index.html) | Import and export of OpenRocket, RockSim and RASAero designs, and OpenRocket's parts database | [M3.1][roadmap] to [M3.6][roadmap], [M5.5][roadmap] |
| [`hpr_net`](api/hpr_net/index.html) | Optional online data, cached for offline use: weather, soundings, elevation and motor data | [M5.1][roadmap] |
| [`hpr_analysis`](api/hpr_analysis/index.html) | Monte Carlo dispersion, sensitivity analysis and optimization | [M6.1][roadmap] to [M6.3][roadmap] |
| [`hpr_flightdata`](api/hpr_flightdata/index.html) | Flight-log import, filtering and analysis | [M7.1][roadmap] to [M7.4][roadmap] |
| [`hpr_py`](api/hpr_py/index.html) | Python bindings | [M4.3][roadmap] |
| [`hpr_ffi`](api/hpr_ffi/index.html) | A C interface, for other languages | [M4.4][roadmap] |
| [`hpr_wasm`](api/hpr_wasm/index.html) | WebAssembly bindings, for the browser | [M4.4][roadmap] |

The command-line tool, `hpr-cli`, is a program rather than a library, so it has no reference here.
It is planned in [M4.2][roadmap].

## Reading it

- **Names.** Code names a crate with underscores (`use hpr_sim::Simulation;`), and `Cargo.toml`
  names its package with hyphens (`hpr-sim`). They are the same crate.
- **Search.** The search box at the top of every reference page, or the **S** key, finds a type or
  function in any crate.
- **Source.** Each item's **Source** link shows the code it documents.
- **On GitHub** the links above lead nowhere: the reference is built, not stored in the
  repository. Read it on the site, at <https://nrdptel.github.io/hpr-sim/api.html>, or build it on
  your own machine and open it in a browser:

  ```sh
  cargo doc --workspace --no-deps --open
  ```

[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
