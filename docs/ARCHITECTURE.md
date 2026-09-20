# Architecture (initial, revisable through ADRs)

This is the starting design. Claude Code owns it. Any change to the crate map, a core dependency,
or a public trait gets an entry in `DECISIONS.md` first.

## Principles

- **A pure core.** Physics crates hold data plus functions: no filesystem, network, threads or
  clocks. That keeps them testable, deterministic, WASM-ready and embeddable.
- **One canonical model.** Every importer maps into the `hpr-design` model, and every exporter maps
  out of it. The solver only ever sees the canonical model, never a foreign file. (Loft proved
  this pattern works.)
- **Traits at the seams.** Atmosphere, wind, aerodynamics, motor, integrator, event trigger,
  recovery device and observer/recorder are all traits. Built-in implementations are defaults,
  not the only option.
- **Features for weight.** `net`, `python`, `ffi`, `parallel` and `serde` are cargo features, so a
  minimal embed stays small.
- **Analysing a flight stands on its own.** Reading a flight log and working out what it says is a
  use of this project in its own right: someone with a log, no design file and no wish to simulate
  anything is a first-class user. So `hpr-flightdata` must not depend on `hpr-sim`, which its
  manifest states as `forbids = ["hpr-sim"]` and `cargo xtask wasm-check` enforces. Anything that
  needs both a flight and a simulation of it lives in `hpr-forensics` (ADR-046).

## Crate map (workspace members under `crates/`; `xtask/` sits at the root, per ADR-001)

| crate | role | depends on |
|---|---|---|
| `hpr-core` | math (`glam` f64: `DVec3`/`DQuat`/`DMat3`), unit helpers, frames, Earth/gravity models, interpolation tables, error types | — |
| `hpr-atmos` | USSA76/ISA, custom soundings, wind models (profiles, turbulence) | core |
| `hpr-motor` | solid-motor model (thrust, mass, CG and inertia over time), `.eng`/`.rse` I/O, catalog types | core |
| `hpr-design` | component tree, shapes, materials, mass properties, configurations/stages, design checks | core, motor |
| `hpr-aero` | Barrowman plus extensions, drag buildup, compressibility, damping, override tables | core, design |
| `hpr-sim` | 6-DOF engine: state, rail phase, integrators, events, recovery, staging, recorder | core, atmos, motor, design, aero |
| `hpr-analysis` | Monte Carlo, sensitivity, optimization, challenge specs (`parallel` feature uses rayon) | sim |
| `hpr-flightdata` | flight-log importers, the canonical flight record, filtering and smoothing, time alignment, and the readings taken from a flight with the provenance of each | core, atmos |
| `hpr-forensics` | a flight against a simulation of it: residuals, parameter identification, fault diagnosis | flightdata, sim, analysis |
| `hpr-format` | the new open design format: types, JSON Schema, versioning and migrations, container | design |
| `hpr-io` | foreign formats: `.ork`, `.rkt`, `.CDX1`, RocketPy export, `.orc` parts DB | design, format |
| `hpr-net` | optional online sources plus the on-disk cache: Open-Meteo, NOAA GFS/RAP, soundings, elevation, ThrustCurve, motor.fusionspace.co | atmos, motor |
| `hpr` (facade) | re-exports plus a RocketPy-like builder API (`Environment`, `Motor`, `Rocket`, `Flight`) | the crates above |
| `hpr-cli` | `hpr` binary: `sim`, `validate`, `convert`, `motors`, `weather`, `mc`, `optimize`, `compare`, `diagnose` | facade |
| `hpr-py` | PyO3/maturin bindings (abi3 wheels), with numpy outputs | facade |
| `hpr-ffi` | stable C ABI plus a cbindgen header | facade |
| `hpr-wasm` | wasm-bindgen package with generated TS types (`tsify`) | facade (no net) |
| `hpr-validate` | validation harness: cases, oracle references, metrics, reports | facade, flightdata |
| `xtask` | dev automation: `wasm-check`, `refs`, `validate`, `site` (the documentation site, ADR-016), `schema`, `bench`, `notices` | — |

**Pure core** (ADR-001): the crates that do no I/O and must build for `wasm32-unknown-unknown`
declare `[package.metadata.hpr] wasm = true`. They are `hpr-core`, `hpr-atmos`, `hpr-motor`,
`hpr-design`, `hpr-aero`, `hpr-sim`, `hpr-analysis`, `hpr-flightdata`, `hpr-format`, `hpr-io`,
`hpr-forensics`, `hpr` (without `net`) and `hpr-wasm`. `cargo xtask wasm-check` enforces both the
build and the rule that they depend on no workspace crate outside the core; `clippy.toml` bans the
I/O, clock and thread APIs.

**Layering rules within the core** (ADR-046): a crate that has to be usable on its own names the
crates that must never reach it, directly or through anything else, in its own manifest:

```toml
[package.metadata.hpr]
forbids = ["hpr-sim"]
```

`cargo xtask wasm-check` walks the workspace graph and fails with the path it found, so the rule
catches a forbidden crate arriving through an innocent-looking helper, not just a direct
dependency. `hpr-flightdata` is the crate this was written for.

Crate names on crates.io are **not** reserved yet. Publishing is a "Needs Neer" item.

## Recommended dependencies

Check each one on crates.io before adding it, and record any change in an ADR.

- **Math:** `glam` (f64 types) for state; `nalgebra` or `faer` only where real linear algebra is
  needed (estimation, optimization).
- **Integration:** in-house adaptive Dormand–Prince 5(4) with dense output and event
  root-finding, plus fixed-step RK4. Integration is core IP and needs event handling tuned for
  flight phases. `diffsol` or `ode_solvers` can serve as cross-checks in tests.
- **Serialization:** `serde`, `serde_json`, `toml`, `schemars` (JSON Schema), `quick-xml` or
  `roxmltree` (XML), `zip`, `csv`, optionally `arrow`/`parquet` for large outputs.
- **Errors, logging, CLI:** `thiserror`, `anyhow` (binaries only), `tracing`, `clap`.
- **Parallelism and bindings:** `rayon`; `pyo3` + `maturin` + `numpy`; `wasm-bindgen` + `tsify`;
  `cbindgen`; later `uniffi` for native mobile bindings.
- **Networking:** `ureq` or `reqwest` (blocking is fine in `hpr-net`; `rustls` only), `grib` (pure
  Rust GRIB2).
- **Cache and platform:** `rusqlite` (bundled) or plain files for the cache, `directories` for
  platform paths.
- **Testing:** `proptest`, `insta`, `criterion`, `approx`, `cargo-nextest`, `cargo-deny`.
- **Optimization:** `argmin`, `egobox` (Bayesian/EGO). Implement CMA-ES and NSGA-II in-house if
  the crates are stale; the `cmaes` crate has low activity.
- **Avoid:** `serde_yaml`/`serde_yml`, `hdf5` (abandoned; use `hdf5-metno` only if netCDF4 is
  truly needed), and GPL crates.

## Physics scope (target fidelity)

- **Dynamics:** 6-DOF rigid body with variable mass and inertia, including jet damping. Launch
  rail/tower phase with guide geometry and friction. Optional rotating Earth
  (Coriolis/centrifugal). WGS84 gravity (Somigliana) varying with altitude. A launch-site
  local-tangent-plane (ENU) frame with geodetic conversion.
- **Atmosphere:** USSA76 up to 86 km, ISA offsets, measured or forecast profiles (pressure,
  temperature, humidity, wind vs altitude), seeded turbulence (Dryden).
- **Aerodynamics:**
  - Barrowman component normal force and CP, with Mach corrections.
  - Body lift at angle of attack.
  - Fin–body interference.
  - Pitch, yaw and roll damping; roll forcing from fin cant.
  - Component drag buildup (skin friction with Reynolds number and roughness, nose and
    transition pressure drag, base drag power-on and power-off, fin profile, protuberances).
  - Transonic drag rise and supersonic wave drag.
  - Override tables (Cd/CNα/CP vs Mach and AoA) from RASAero exports, CFD or wind-tunnel data.
- **Motors:**
  - Thrust curve with interpolation.
  - Propellant mass tracking impulse fraction (the default) or a grain-geometry model.
  - CG and inertia evolution; nozzle exit area for power-on base drag.
  - Ejection delay; ambient-pressure correction as an option.
  - Clusters and staging/airstarts (COTS only).
- **Recovery:** drogue and main, streamers, deployment triggers (apogee, altitude, timer, motor
  delay, barometric), inflation dynamics, drift with the wind profile, tumble recovery, separate
  bodies after separation.
- **Outputs:** full time series; event log; static and dynamic stability margin; derived metrics;
  optimum delay; landing point; flutter margin (NACA TN 4197 family). Exports to CSV, JSON,
  Parquet, KML and GeoJSON.

## The new design format (brief for M3.3)

**Goals:**

- Open and language-neutral: a JSON Schema published with the spec.
- Lossless for everything `.ork` expresses; a superset where `.ork` is weak.
- Git-friendly: canonical key order, pretty-printed, stable UUIDv7 component ids.
- Versioned, with automatic migrations.
- Offline-complete: can embed thrust curves, part data and override tables.
- Records provenance: tool, version, source-file hash.
- Extensible through namespaced `extensions` (for example `x-openrocket`) that keep foreign data
  we don't model, so export back to the source format is lossless.

**Two forms:**

- A plain `.json` design.
- A zip container for design plus attachments: flight logs, results, images. Name both extensions
  in an ADR.

**Beyond `.ork`:**

- Explicit units (SI only, documented).
- Multiple configurations and a design-check history.
- References to catalog parts (manufacturer + part number) and motors (ThrustCurve id), with an
  embedded snapshot.
- Simulation cases with their environment and seeds, and optional attached flight data.
- Generated types for TypeScript and Python.

**Deliverables:** a spec in `docs/format/`, compared against `.ork`, `.rkt`, `.CDX1` and RocketPy
`.rpy`, including why we didn't simply adopt one of them.

## Interfaces for "plug it into anything"

| surface | contents |
|---|---|
| Rust | `hpr` facade with a stable builder API; traits for custom models; observers for streaming state |
| CLI | every command has `--json` output; stdin/stdout friendly; exit codes documented |
| Python | `import hpr_sim`; RocketPy-flavored classes; numpy arrays; callbacks for custom models |
| C ABI | opaque handles, a JSON-in/JSON-out simulate call, and a streaming callback; for C/C++/C#/MATLAB/Julia/LabVIEW |
| WASM | an npm-style package for the future web UI and third-party pages |
| Files | the published JSON Schema for designs, simulation cases and results |

## Future UI stack (decide by ADR in M9.0, not before)

The leading option is one web UI codebase (TypeScript with a modern framework, plus three.js or
Babylon.js for 3D) that runs on the WASM core:

- As a static, client-side PWA for web and install (it matches Loft's hosting on Cloudflare
  Pages).
- Wrapped by **Tauri v2** for desktop and iOS/Android, where the native Rust core is used for
  heavy Monte Carlo and optimization.

The alternative to evaluate is all-Rust (Leptos/Dioxus + wgpu/Bevy). Decide on 3D quality, bundle
size, mobile performance and development speed. The 3D "ghost" replay consumes the M7.2 data
product.

## Testing layers

1. Unit and analytic tests.
2. Property tests.
3. Snapshot tests for I/O.
4. Component-level references (tables and worked examples from papers).
5. Code-to-code against RocketPy and OpenRocket.
6. Real flights.
7. Benchmarks and performance budgets.

`docs/VALIDATION.md` has the details. CI runs 1–4 plus the stored-reference comparisons for 5–6.
Regenerating the references (Python/Java) is a separate, manually triggered workflow.
