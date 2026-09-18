# The API reference

**The API reference documents hpr-sim's code: every public type, function and constant, generated
from the source by rustdoc, Rust's documentation tool.** Use it when you write a program with
hpr-sim, as [Getting started](getting-started.md) does. This page says which crate holds what, and
links each crate's reference. The library is pre-alpha: none of its interface is stable yet, any of
it can change, and a simpler one is planned ([M4.1](decisions-and-roadmap.md#m4-1), a builder API).

**Where to read it.** On the site, the crate names below open the reference. On GitHub they lead
nowhere, because the reference is built rather than stored in the repository. Build it on your own
machine instead:

```sh
cargo doc --workspace --no-deps --open
```

That opens one crate's front page; the others are in the list of crates on the left. `cargo xtask
site` builds this whole site, with the reference beside the guide.

The reference is built from the same source that CI compiles on every change, so it describes the
code as it is. Every link in it is checked in CI, apart from a few inside the vector types'
documentation, which rustdoc copies from the glam crate. Physics items give their equation and cite
their source, as the model pages here do. Each crate's front page links back to the pages here that
explain its models.

## The crates

hpr-sim is split into [crates](glossary.md#crate), Rust's packages, so that a program takes only
what it needs, and so that the models, which do no file or network I/O, also build for the web.
Seven crates hold code today:

| crate | what it holds | not yet | the guide's pages |
|---|---|---|---|
| [`hpr_core`](api/hpr_core/index.html) | Vectors and quaternions, frames, the Earth's shape and gravity, interpolation tables and numerical integration of functions | | [Frames](physics/frames.md), [Geodesy](physics/geodesy.md), [Gravity](physics/gravity.md), [Interpolation tables](physics/interpolation.md), [Adaptive quadrature](physics/quadrature.md) |
| [`hpr_atmos`](api/hpr_atmos/index.html) | The standard atmosphere, humidity, soundings, wind profiles and turbulence | A flight doesn't use the turbulence yet | [Atmosphere](physics/atmosphere.md), [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| [`hpr_motor`](api/hpr_motor/index.html) | Solid motors: thrust curves, mass and inertia through the burn, `.eng` and `.rse` files, and the [32 bundled curves](physics/motor.md#the-bundled-motors) | Hybrid and liquid motors, which are out of scope | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| [`hpr_design`](api/hpr_design/index.html) | The rocket: its tree of parts, their shapes and materials, mass properties and design checks | | [The design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| [`hpr_aero`](api/hpr_aero/index.html) | Aerodynamics: normal force, centre of pressure, drag and drag tables | Mach 1 and above ([M1.8](decisions-and-roadmap.md#m1-8)) | [Aerodynamics](physics/aero.md) |
| [`hpr_sim`](api/hpr_sim/index.html) | The flight: the launch rail, the equations of motion, time integration, events and recovery | Staging under power, clusters and air starts ([M1.9](decisions-and-roadmap.md#m1-9)) | [How a flight is simulated](how-a-flight-is-simulated.md), [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md), [Recovery](physics/recovery.md) |
| [`hpr_validate`](api/hpr_validate/index.html) | The validation harness: cases, reference data, metrics and reports | Whole flights against RocketPy ([M2.1b2](decisions-and-roadmap.md#m2-1b2)) | [Accuracy](accuracy.md), [Checking a claim](checking-a-claim.md) |

The other nine crates are placeholders for planned work. Each has a front page that says what it
will hold, and nothing else yet:

| crate | what it will hold | planned in |
|---|---|---|
| [`hpr`](api/hpr/index.html) | One crate to depend on, with a builder for environments, motors, rockets and flights | [M4.1](decisions-and-roadmap.md#m4-1) |
| [`hpr_format`](api/hpr_format/index.html) | hpr's own design file format | [M3.3](decisions-and-roadmap.md#m3-3) |
| [`hpr_io`](api/hpr_io/index.html) | Import and export of OpenRocket, RockSim and RASAero designs, export to RocketPy, and OpenRocket's parts database | [M3.1](decisions-and-roadmap.md#m3-1) to [M3.6](decisions-and-roadmap.md#m3-6), [M5.5](decisions-and-roadmap.md#m5-5) |
| [`hpr_net`](api/hpr_net/index.html) | Optional online data, cached for offline use: weather, soundings, elevation and motor data | [M5.1](decisions-and-roadmap.md#m5-1) |
| [`hpr_analysis`](api/hpr_analysis/index.html) | Monte Carlo dispersion, sensitivity analysis, optimization, and competition challenges such as a target apogee | [M6.1](decisions-and-roadmap.md#m6-1) to [M6.3](decisions-and-roadmap.md#m6-3) |
| [`hpr_flightdata`](api/hpr_flightdata/index.html) | Flight-log import, filtering and analysis | [M7.1](decisions-and-roadmap.md#m7-1) to [M7.4](decisions-and-roadmap.md#m7-4) |
| [`hpr_py`](api/hpr_py/index.html) | Python bindings | [M4.3](decisions-and-roadmap.md#m4-3) |
| [`hpr_ffi`](api/hpr_ffi/index.html) | A C interface, for other languages | [M4.4](decisions-and-roadmap.md#m4-4) |
| [`hpr_wasm`](api/hpr_wasm/index.html) | WebAssembly bindings, for the browser | [M4.4](decisions-and-roadmap.md#m4-4) |

The command-line tool, `hpr-cli`, is a program rather than a library, so it has no reference here.
It is planned in [M4.2](decisions-and-roadmap.md#m4-2).

## Using it from your own program

hpr-sim is not on crates.io yet. A program outside this repository can depend on a crate straight
from GitHub, pinned to a commit, since anything can change between commits. Take the commit's hash
from [the history of `main`](https://github.com/nrdptel/hpr-sim/commits/main), and change it only
on purpose:

```toml
[dependencies]
hpr-sim = { git = "https://github.com/nrdptel/hpr-sim", rev = "<commit>" }
```

## Reading it

- **Names.** Code names a crate with underscores (`use hpr_sim::Simulation;`), and `Cargo.toml`
  names its package with hyphens (`hpr-sim`). They are the same crate.
- **Search.** The search box at the top of every reference page, or the **S** key, finds a type or
  function in any crate.
- **Source.** Each item's **Source** link shows the code it documents.
- **Links to the guide** on the crates' front pages go to the published site,
  <https://nrdptel.github.io/hpr-sim/>. Until it is live, read the same pages in the repository's
  [`docs/` folder](https://github.com/nrdptel/hpr-sim/tree/main/docs).

