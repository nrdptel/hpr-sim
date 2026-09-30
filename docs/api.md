# The API reference

**The API reference documents hpr-sim's code: every public type, function and constant, generated
from the source by rustdoc, Rust's documentation tool.** Use it when you write a program with
hpr-sim, as [Getting started](getting-started.md) does. This page says which crate holds what, and
links each crate's reference. The library is pre-alpha: none of its interface is stable yet, any of
it can change, and the simplest way in is the `hpr` crate's builder ([The builder](the-builder.md)).

**Where to read it.** On the site, the crate names below open the reference. On GitHub they lead
nowhere, because the reference is built rather than stored in the repository. Build it on your own
machine instead:

```sh
cargo doc --workspace --no-deps --open
```

That opens one crate's front page; the others are in the list of crates on the left. `cargo xtask
site` builds this whole site, with the reference beside the guide.

The reference is built from the same source that CI, the project's automated checks, compiles on
every change, so it describes the code as it is. Every link in it is checked in CI, apart from a
few inside the vector types' documentation, which rustdoc copies from glam, the vector library
hpr-sim uses. Physics items give their equation and cite their source, as the model pages here do.
Each crate's front page links back to the pages here that explain its models.

## The crates

hpr-sim is split into [crates](glossary.md#crate), Rust's packages, so that a program takes only
what it needs, and so that the models, which don't read or write files or use the network, also
build for the web. Eleven crates hold most of the code today:

| crate | what it holds | not yet | the guide's pages |
|---|---|---|---|
| [`hpr`](api/hpr/index.html) | One crate to depend on: the builder for environments, motors, rockets and flights, the other crates re-exported by name, [`hpr::ork::separation`](api/hpr/ork/fn.separation.html), which turns a `.ork` file's staging into a flight's separation ([example](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/ork_two_stage.rs)), a drag model of your own in hpr's place, and a guide in the reference, [`hpr::guide`](api/hpr/guide/index.html) | One motor per built rocket | [The builder](the-builder.md), [Models of your own](custom-models.md) |
| [`hpr_core`](api/hpr_core/index.html) | Vectors and quaternions (a compact way to store a rotation), frames, the Earth's shape and gravity, interpolation tables and numerical integration of functions | | [Frames](physics/frames.md), [Geodesy](physics/geodesy.md), [Gravity](physics/gravity.md), [Interpolation tables](physics/interpolation.md), [Adaptive quadrature](physics/quadrature.md) |
| [`hpr_atmos`](api/hpr_atmos/index.html) | The standard atmosphere, humidity, soundings, wind profiles and turbulence | A flight doesn't use the turbulence yet | [Atmosphere](physics/atmosphere.md), [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| [`hpr_motor`](api/hpr_motor/index.html) | Solid motors: thrust curves, mass and inertia through the burn, `.eng` and `.rse` files, and the [32 bundled curves](physics/motor.md#the-bundled-motors) | Hybrid and liquid motors, which are out of scope | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| [`hpr_design`](api/hpr_design/index.html) | The rocket: its tree of parts, their shapes and materials, mass properties and design checks | | [The design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| [`hpr_format`](api/hpr_format/index.html) | hpr's own design format: a design as one JSON document (`.hpr`) with its JSON Schema, read from and written to `.ork`, in a zip container with other files (`.hprz`), older versions migrated | Generated TypeScript and Python types ([M3.3c](decisions-and-roadmap.md#m3-3c)) | [The hpr design format](format/hpr.md) |
| [`hpr_aero`](api/hpr_aero/index.html) | Aerodynamics: normal force, centre of pressure and drag to Mach 5, tables from other programs for the drag and the normal force, and drag models of your own ([Models of your own](custom-models.md)) | Faster than sound a flight takes a pointed nose, its cylinder and a boattail behind them from the [shock-expansion method](physics/aero.md#the-body-faster-than-sound-in-a-flight), the boattail's share unvalidated; a blunt or vertical tip flies the method behind a [Newtonian cap](physics/aero.md#blunt-tips), checked on a sphere-cone only; a conical flare flush with the part ahead of it flies the method too and ends the run, checked against [one measured flare](physics/aero.md#what-a-marched-flare-is-worth); any other widening shape, or any step, behind the nose keeps slender-body theory, which reads low past Mach 3 | [Aerodynamics](physics/aero.md) |
| [`hpr_sim`](api/hpr_sim/index.html) | The flight: the launch rail, the equations of motion, time integration, events and recovery | Staging and [air starts](glossary.md#air-start) fly, checked by tests and against OpenRocket's two-stage, cluster and air-start examples, each flight within 5% in apogee and largest speed ([M1.9c](decisions-and-roadmap.md#m1-9c), a two-stage and a cluster design against OpenRocket). Three cluster apogees are compared with OpenRocket's flight with no parachute, since its parachute opened before apogee. A [cluster](glossary.md#cluster) flies, one mount of several tubes or one mount per motor, and so does a motor out, checked by tests against a hand calculation. A nose cone, a section or a payload can leave the airframe, optionally pushed by its charge, and land on its own under its own parachute or tumbling ([ejection](glossary.md#ejection), [M1.11a](decisions-and-roadmap.md#m1-11a), [M1.11b](decisions-and-roadmap.md#m1-11b)), checked against exact answers only. | [How a flight is simulated](how-a-flight-is-simulated.md), [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md), [Recovery](physics/recovery.md), [Staging](physics/staging.md) |
| [`hpr_flightdata`](api/hpr_flightdata/index.html) | Reading a flight log on its own, with no design and no simulation: PerfectFlite's `.pf2` so far, and liftoff, apogee, the top speed, landing and the descent, each saying where it came from or why it was withheld. Depend on this crate directly rather than on `hpr`, which pulls in the simulator | Other loggers' files ([M7.1](decisions-and-roadmap.md#m7-1)); the descent's legs, Mach number and the rest of the readings ([M7.2](decisions-and-roadmap.md#m7-2)) | [Reading a flight log](reading-a-flight-log.md), [Flight-log readings](physics/log-readings.md), [`.pf2` files](format/pf2.md) |
| [`hpr_validate`](api/hpr_validate/index.html) | The validation harness: cases, reference data, metrics and reports | Whole flights against RocketPy ([M2.1b2](decisions-and-roadmap.md#m2-1b2)) | [Accuracy](accuracy.md), [Checking a claim](checking-a-claim.md) |
| [`hpr_py`](api/hpr_py/index.html) | The `hpr` Python package: the builder's environment, motor, rocket and flight from Python, designs read from files, and a flight's recording as NumPy arrays. Built by maturin into one wheel per operating system; not on PyPI | A drag table and RocketPy's example flight ([M4.3b](decisions-and-roadmap.md#m4-3b)); a drag and a wind written in Python ([M4.3c](decisions-and-roadmap.md#m4-3c)) | [Python](python.md) |

The other six crates are for planned work. Each has a front page that says what it will hold.
One already holds some code, `hpr_io`, as its row says:

| crate | what it will hold | planned in |
|---|---|---|
| [`hpr_io`](api/hpr_io/index.html) | Import and export of OpenRocket, RockSim and RASAero designs, export to RocketPy, and OpenRocket's parts database. A `.ork` file's container, design document and components are read into a design ([the format page](format/ork.md)); so are its motors, when each lights, and its recovery settings. One powered separation comes out as a [`Staging`](api/hpr_io/ork/staging/struct.Staging.html): its time is known before the flight, and then a motor ahead of it is still burning or yet to light and none behind it is. Any other separation that could come before apogee is refused, a sustainer already burnt out at the split included. The configuration's rocket doesn't carry the `Staging`: [`hpr::ork::separation`](api/hpr/ork/fn.separation.html) turns it into the flight's separation, which you pass to the flight yourself, with a recovery device on each part, since hpr refuses the flight without them ([example](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/ork_two_stage.rs)); parachutes are read but not flown yet. A design is written back out as a `.ork` with `hpr_io::ork::export` ([writing a `.ork`](format/ork.md#writing-a-ork-back-out)). An ERA5 weather file (netCDF classic) gives the atmosphere over a launch site at launch time ([ERA5 weather files](format/era5.md)) | [M3.1a](decisions-and-roadmap.md#m3-1a), [M3.2a](decisions-and-roadmap.md#m3-2a) and [M2.3a](decisions-and-roadmap.md#m2-3a) done; [M3.1b](decisions-and-roadmap.md#m3-1b) to [M3.6](decisions-and-roadmap.md#m3-6), [M5.5](decisions-and-roadmap.md#m5-5) next |
| [`hpr_net`](api/hpr_net/index.html) | Optional online data, cached for offline use: weather, soundings, elevation and motor data. The cache, the offline mode and HTTP work today ([Online data and the cache](online-data.md)); there is no data source yet | [M5.1](decisions-and-roadmap.md#m5-1) done; [M5.2](decisions-and-roadmap.md#m5-2) next |
| [`hpr_analysis`](api/hpr_analysis/index.html) | Monte Carlo dispersion (flying many copies of a flight with randomly scattered inputs), sensitivity analysis, optimization, and competition challenges such as a target apogee | [M6.1](decisions-and-roadmap.md#m6-1) to [M6.3](decisions-and-roadmap.md#m6-3) |
| [`hpr_forensics`](api/hpr_forensics/index.html) | A flown flight against a simulation of it: what will differ, what that says about drag, mass, impulse and wind, and what went wrong | [M7.3](decisions-and-roadmap.md#m7-3) to [M7.4](decisions-and-roadmap.md#m7-4) |
| [`hpr_ffi`](api/hpr_ffi/index.html) | A C interface, for other languages | [M4.4](decisions-and-roadmap.md#m4-4) |
| [`hpr_wasm`](api/hpr_wasm/index.html) | WebAssembly bindings, for the browser | [M4.4](decisions-and-roadmap.md#m4-4) |

The command-line tool, `hpr-cli`, is a program rather than a library, so it has no reference here:
[The command line](cli.md) documents its commands, output and exit codes.

## Using it from your own program

hpr-sim is not on crates.io, Rust's public package registry, yet. A program outside this
repository can depend on a crate straight from GitHub, pinned to a commit, since anything can
change between commits. Take the commit's hash from
[the history of `main`](https://github.com/nrdptel/hpr-sim/commits/main), and change it only on
purpose:

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

