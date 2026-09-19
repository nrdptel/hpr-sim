# Decisions (ADR log)

Each entry is short: context, decision, consequences, date. Number them sequentially and never
renumber. Supersede an entry by adding a new one that points back to it.

| id | title | status |
|---|---|---|
| ADR-000 | Kickoff decisions | accepted |
| ADR-001 | License and workspace layout | accepted |
| ADR-002 | The reference library: lock file, fetch, verify and doctor | accepted |
| ADR-003 | Frames, attitude, geodesy and the gravity model | accepted |
| ADR-004 | Atmosphere, wind, turbulence and the seeded generator | accepted |
| ADR-005 | Solid motors: statistics, consumption, grains, file models and the bundled catalog | accepted |
| ADR-006 | Component geometry and mass properties: frames, shapes, walls, fins and materials | accepted |
| ADR-007 | Design tree: stations, placement, automatic radii, overrides, motors and checks | accepted |
| ADR-008 | Subsonic normal force and centre of pressure | accepted |
| ADR-009 | Subsonic drag buildup, surface finishes and drag override tables | accepted |
| ADR-010 | Time integration: Dormand–Prince with dense output, RK4, stop times and events | accepted |
| ADR-011 | Rigid-body flight: equations of motion, aerodynamic coupling, rail, phases and termination | accepted |
| ADR-012 | Recovery: drag areas, triggers, inflation and the descent phase | accepted |
| ADR-013 | Streamer and tumble drag | accepted |
| ADR-014 | Separation: bodies, their masses and their descents | accepted |
| ADR-015 | The validation harness: cases, references, tolerances and reports | accepted |
| ADR-016 | The documentation site: mdBook over `docs/`, and checks for links, labels and equations | accepted |
| ADR-017 | Model pages open with *In short*; Accuracy traces its numbers; the records stay files | accepted |
| ADR-018 | Examples run in CI against committed output; pages quote files, checked line for line | accepted |
| ADR-019 | Publishing the site and the API reference to GitHub Pages | accepted |
| ADR-020 | The reader test, and labels that lead to plain words | accepted |
| ADR-021 | Whole flights against RocketPy: what is compared, and the gaps it may declare | accepted |
| ADR-022 | Validation in CI, and regenerating references only by hand | accepted |
| ADR-023 | Predicted mode: each code's own drag, reported against a target | accepted |
| ADR-024 | The time-series RMS: aligned at ignition, held to 3% of its trace's scale | accepted |
| ADR-025 | The calm-air cases, and Juno III's drifts left to the rail release | accepted; Juno III's drifts superseded by ADR-026 |
| ADR-026 | The path in wind: RocketPy's corrected equations, and hpr's body lift | accepted; Prometheus's drifts superseded by ADR-027 |
| ADR-027 | The normal force through Mach 1: supersonic linear theory, a transonic join, and the measured references | accepted |

---

## ADR-000: Kickoff decisions (2026-09-16)

**Context.** Neer asked for a Rust hobby/HPR rocket simulator: library first, validation-heavy,
offline-first, open source. It replaces his Loft project (TypeScript/Next.js), which he is shutting
down.

**Decision.**

- **Language and layout:** Rust, in a workspace of focused crates (see `ARCHITECTURE.md`).
- **Order of work:** physics core and validation first; interop, bindings and analysis next; UI
  last.
- **License:** `MIT OR Apache-2.0`. MIT matches the other Fusion Space projects; Apache-2.0 adds
  a patent grant and is the Rust norm. ADR-001 confirms this in M0.1.
- **Clean room:** no GPL/AGPL source is read or ported. GPL tools may be run as external oracles.
- **No AI traces** in commits, PRs, code or docs. Commits are authored as Neer's GitHub noreply
  identity.
- **Private data:** `loft-fixtures` content is never committed; only derived statistics are
  published.
- **Name:** `hpr-sim` for now.
- **Scope:** COTS solid motors only.
- **Hosting:** a public GitHub repo (`nrdptel/hpr-sim`). Work ships through PRs; CI covers macOS,
  Windows and Linux; merging on green is pre-authorized.
- **How work runs:** unattended cycles, one milestone each, handing off through `STATUS.md`
  (see `docs/AUTOPILOT.md`).

**Consequences.**

- Work is slower at first, but every number is defensible.
- A few formats (`.CDX1`, parts of `.ork` 1.12) must be learned from samples rather than from
  source code.
- Publishing to crates.io or PyPI waits for Neer.

---

## ADR-001: License and workspace layout (2026-09-17)

**Context.** M0.1 turns ADR-000's provisional license and the crate map in `ARCHITECTURE.md` into
files and checks: license texts, a dependency-license policy that backs the clean-room rule, and a
workspace whose pure core can be verified.

**Decision.**

- **License:** `MIT OR Apache-2.0`, confirmed. MIT matches the other Fusion Space projects,
  Apache-2.0 adds an explicit patent grant, and the pair is the Rust norm. `LICENSE-MIT`
  (copyright "Neer Patel and the hpr-sim contributors") and `LICENSE-APACHE` (the canonical
  apache.org text, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`).
  Every crate inherits `license` from `[workspace.package]`; the README carries the usual
  dual-license contribution clause.
- **Dependency licenses (`deny.toml`):** only permissive licenses are allowed: 0BSD, Apache-2.0
  (also WITH LLVM-exception), BSD-2-Clause, BSD-3-Clause, BSL-1.0, CC0-1.0,
  CDLA-Permissive-2.0 (the data license of `webpki-roots`, which rustls needs), ISC, MIT, MIT-0,
  Unicode-3.0, Unlicense and Zlib. Everything else is rejected, weak copyleft (LGPL, MPL, EPL)
  included; widening the list, or a per-crate exception, needs a new ADR. Security and unsound
  advisories and yanked versions fail anywhere in the graph. Unmaintained notices fail only for
  direct dependencies: picking maintained crates is our call, and a notice deep in the graph must
  not block unrelated PRs in unattended runs. crates.io is the only allowed source. CI pins
  cargo-deny to the version `deny.toml` was tested with (0.20.2). Checked on 2026-09-17: a
  throwaway workspace with GPL-3.0, AGPL-3.0, LGPL-2.1 and MPL-2.0 crates fails
  `cargo deny check licenses` (exit 4), and its MIT crate passes.
- **Layout:** a virtual workspace (resolver 3, edition 2024) with `rust-version` equal to the
  toolchain pinned in `rust-toolchain.toml` (1.98.1); bump the two together. The 17 crates of the crate map live under
  `crates/`, and their internal dependency edges are declared now, so the layering holds from the
  start. `xtask` sits at the root (`xtask/`), where the cargo-xtask convention puts it, because it
  is tooling rather than product. Crates start at 0.1.0 with `publish = false` until Neer decides
  on publishing.
- **Pure core:** a crate joins it with `[package.metadata.hpr] wasm = true`. Members: `hpr-core`,
  `hpr-atmos`, `hpr-motor`, `hpr-design`, `hpr-aero`, `hpr-sim`, `hpr-analysis`,
  `hpr-flightdata`, `hpr-format`, `hpr-io`, the `hpr` facade with default features (`net` is an
  optional feature) and `hpr-wasm`. Outside it: `hpr-net` (network and cache I/O),
  `hpr-validate` (reads case files), `hpr-cli`, `hpr-py`, `hpr-ffi` and `xtask`.
  `cargo xtask wasm-check` enforces it in two steps. First, no workspace crate outside the core
  may appear in the core's normal dependency graph for any target, as resolved by `cargo tree`
  (so features one pure crate enables in another count). Second,
  `cargo clippy --target wasm32-unknown-unknown -- -D warnings` on the core. Compiling for wasm32
  does not rule out I/O (`std::fs` and `Instant::now` compile and then fail at run time), so
  `clippy.toml` also disallows the filesystem, network, clock, thread, process and environment
  APIs everywhere. Crates outside the core allow those two lints at the crate root. A unit test
  pins the membership list, so changing it is visible in review.
- **Lints:** `missing_docs`, `missing_debug_implementations` and `unsafe_code = "deny"` for rustc;
  `unwrap_used`, `expect_used`, `panic`, `print_stdout`, `print_stderr`,
  `allow_attributes_without_reason`, `dbg_macro`, `todo` and `unimplemented` for clippy.
  `clippy.toml` relaxes the panic and print lints in tests, and binaries allow printing at the
  crate root. CI fails on warnings in clippy (Linux host and wasm32), rustdoc, and the macOS and
  Windows test builds.
- **Line endings:** `.gitattributes` checks text out with LF everywhere, so formatting, snapshot
  tests and reference-data hashes behave the same on Windows.
- **CI:** one workflow on every PR and on pushes to `main`. fmt, clippy, doc, wasm-check and deny
  run on Linux; tests run on Linux, macOS and Windows. Swatinem/rust-cache caches builds;
  cargo-deny is installed as a prebuilt binary.

**Consequences.**

- The CLI binary is named `hpr`, like the facade library, so its rustdoc is off (`doc = false`) to
  avoid an output collision in `target/doc`.
- A binding crate that needs `unsafe` must allow it item by item, with a reason.
- A dependency under MPL or LGPL needs an ADR before it can be added. Known case: `directories`
  pulls in `option-ext` (MPL-2.0), so M5.1 needs either an ADR for a scoped exception or another
  platform-paths crate.
- Tests inside the pure core that read fixture files must use `include_str!`/`include_bytes!` or
  allow the lint on that test with a reason.

## ADR-002: The reference library: lock file, fetch, verify and doctor (2026-09-17)

**Context.** Validation needs other simulators, papers, motor data and private design files on
disk, reproducibly and never committed. M0.2 asks for `cargo xtask refs fetch|verify|doctor`
driven by `validation/refs.lock.toml`.

**Decision.**

- **One lock file, four kinds of item.**
  - `[[git]]`: pinned by full commit id, optionally shallow. Existing checkouts are reused and
    moved to the pin; checkouts with local changes are never touched.
  - `[[file]]`: an immutable download pinned by sha256 (the OpenRocket jar, 11 papers and the
    RockSim `.rse` spec).
  - `[[snapshot]]`: a live API (ThrustCurve, motor.fusionspace.co) pinned by the sha256 and date
    of one capture. APIs move (the motor finder hourly), so a missing snapshot that no longer
    matches fails. The new capture is kept beside it, with a record of its URL, its hash, its
    date and the pin it drifted from. `fetch --adopt-snapshots` moves that kept capture into place
    only if the entry still has the same URL and pin and the bytes are unchanged; otherwise it
    downloads a fresh capture and says why. It then rewrites only that entry's `sha256` and
    `captured` lines, using the capture's real date.
  - `[python]`: a `uv` project in `validation/oracles/` (`pyproject.toml` and `uv.lock`,
    committed), installed into `refs/venv` with `uv sync --locked`. That refuses a lock that is
    stale against `pyproject.toml` and checks every artifact against the hash in `uv.lock`;
    orhelper is pinned by git commit.

  Every destination must be a plain relative path inside the gitignored `refs/`, and
  destinations may not overlap.
- **Private repositories** (`private = true`, today `loft-fixtures`) are fetched with the
  user's git credentials, never prompting. A private repository that isn't checked out and
  doesn't answer `git ls-remote` (no access, as in CI) is skipped with a note. Any other failure,
  such as a bad pin or a checkout that can't move, fails. No partial checkout is left behind. A `sha256sum` manifest in a checkout can be pinned too;
  `verify` then hashes every listed file.
- **Idempotence:** anything already in its pinned state is left alone, so a second `fetch`
  downloads nothing. Downloads go to `<dest>.part` and are renamed into place only after the hash
  matches.
- **Verify re-hashes content**, not timestamps.
  - Files are hashed with SHA-256.
  - Git checkouts are compared, byte for byte (`core.autocrlf=false`), against a fresh temporary
    index read from the pinned commit. That makes git hash every tracked file, whereas a plain
    `git status` trusts cached stat data and can miss a same-size edit; a test shows both.
    Untracked files that the repository doesn't ignore also fail.
  - The environment must pass `uv sync --locked --check`, which compares package versions only.
    Then every installed file is re-hashed against the sha256 its package's `RECORD` lists
    (4,675 files today). Any file or symlinked directory that no `RECORD` lists fails, such as a
    `sitecustomize.py` or a `.pth` file, which would run at startup. The exception is the venv's
    own scaffolding (activation scripts, interpreter links, `pyvenv.cfg`, `_virtualenv.py`),
    which is allowed by name. Its content isn't pinned, because uv generates it, except
    `_virtualenv.pth`, whose one line is checked.
    The check runs Python with `-I`, so `PYTHONPATH` and the current directory can't leak in.
    `fetch` repairs hash failures with `uv sync --reinstall --no-cache`. Files no package owns
    can't be repaired that way, so `fetch` names them and asks for them to be deleted. uv installs by copying
    (`UV_LINK_MODE=copy`), so an edited venv file can't corrupt uv's cache through a hard link.
  - Git commands drop `GIT_DIR`-style variables, so a git hook can't redirect them at this
    repository.
- **External tools instead of crates:** `git`, `curl` and `uv` are run as commands. They exist on
  all three CI operating systems, and an HTTP and TLS stack in `xtask` would add many dependencies
  for no gain. curl is restricted to `https` (and `file` for tests), redirects included, and gives
  up on a transfer that stalls below 1 kB/s for a minute. New
  `xtask` dependencies: `serde`, `toml`, `sha2`, and `tempfile` for tests.
- **Doctor** lists the tools and their versions, a quick reference check (hashes of files,
  commits of checkouts), and whether each oracle is runnable. RocketPy: `rocketpy` imports at the
  locked version. OpenRocket: Java 17+ is found (`JAVA_HOME`, macOS `java_home`, Homebrew's
  keg-only `openjdk` formulae, then `PATH`), the jar verifies, `orhelper` and `jpype` import, and
  a smoke test starts the JVM through JPype and loads the jar's `Main-Class`. The smoke test sets
  `JAVA_HOME` from the runtime's own `java.home` property, which is reliable behind shims. The smoke test uses
  JPype (Apache-2.0) directly, so no project code calls into GPL orhelper. "Runnable" therefore
  means the runtime starts, not that a flight has been simulated; the M2.x oracle scripts do that.
- **Consistency tests:** every lock item needs a row in `THIRD-PARTY-NOTICES.md` with the same
  license and mode (and title, for papers), and every real URL uses `https`.
- **Source choices:** orhelper comes from `openrocket/orhelper` at a pinned commit, because PyPI's
  0.1.3 predates OpenRocket 24.12. The Knacke manual comes from archive.org's mirror of the DTIC
  copy, because DTIC refused automated downloads. It is a contractor report with a restrictive
  title-page notice, so it is labelled "unclear terms" and is never redistributed. RocketPy is a shallow clone of `v1.13.0`
  (about 390 MB, mostly ERA5 weather files that M2.3 needs).

**Consequences.**

- A fresh machine cannot reproduce a snapshot byte for byte once the API has moved; it has to
  adopt a new capture, and results that depend on it must name the capture date. Validation
  results that CI checks must come from committed fixtures, not from `refs/`.
- The `refs` tests need `git` and `curl` on the test machine. They use local `file://` sources
  and never touch the network.
- Java is not fetched: the user installs a JDK, and `doctor` says how.
- M2.2's oracle scripts will need a decision on importing GPL-2.0 orhelper from this repository,
  as opposed to driving the jar through JPype directly. This is recorded under "Needs Neer" in
  `STATUS.md`.

---

## ADR-003: Frames, attitude, geodesy and the gravity model (2026-09-17)

**Context.** M1.1 fixes the conventions that every later crate uses: the frames, the attitude
representation, the Earth model, and interpolation tables. Loft simulated a flat Earth with
constant 9.80665 m/s² gravity and never stated a frame. That left it 0.3% off RocketPy's gravity
at the equator (lesson L1), and its apogee datum and ground-hit frame were never defined (L35).
The comparisons in M2.1 need hpr's conventions to map exactly onto RocketPy's.

**Decision.**

- **Math types:** glam's `f64` types (`DVec3`, `DQuat`, `DMat3`) with the `f64`, `serde` and
  `std` features only. `hpr-core` re-exports them.
- **Frames** (`docs/physics/frames.md`):
  - **ECEF:** WGS 84.
  - **Launch frame `L`:** East-North-Up, origin at the pad, `z_L` along the ellipsoid normal.
    State propagates in `L`, which is Earth-fixed.
  - **Body frame `B`:** `+z_B` along the axis toward the nose, `x_B` along the design's zero
    radial direction.
  - **Attitude:** a unit Hamilton quaternion mapping body to launch components, with
    `q̇ = ½ q ⊗ (0, ω_B)` and renormalization after every step.
  - **Launch angles:** `q = R_z(−A) R_x(E − π/2) R_z(φ)`. This is RocketPy's 3-1-3 initial
    attitude with heading, inclination and rail-button angle, pinned by a RocketPy oracle
    fixture (`validation/oracles/rocketpy/attitude.py`).
  - **Rationale:** RocketPy is the main oracle, so matching its axes removes a class of sign
    errors from the comparisons. At the identity attitude the rocket stands vertical on the pad.
- **Heights:** every height in the core is ellipsoidal (`Geodetic::height_m`). Heights above sea
  level are converted at the I/O boundary. The flight engine reports altitude from the geodetic
  height of the position, not from `z_L`.
- **Geodetic conversion:** the forward conversion is NGA.STND.0036 eq. 4-14. The inverse is
  Karney's (2011, appendix B) version of Vermeille's closed form. It needs no iteration, is exact
  to rounding, and fails only within 43 km of the Earth's centre, where it returns an error.
  Bowring's iterative method was rejected: it needs an iteration count and a convergence
  tolerance, and its primary source is paywalled. Karney's paper is open.
- **Gravity:** WGS 84 normal gravity from its four defining parameters (NGA.STND.0036 ch. 4,
  app. B), with the exact ellipsoidal-harmonic closed form as the default.
  - The flight engine chooses from `constant`, `vertical_taylor` (RocketPy's), `vertical` and
    `ellipsoidal` (the default). Earth rotation contributes Coriolis only, by default. The
    centrifugal term is inside normal gravity.
  - `q` and `q′` use their `atan` series below `ε = 0.5` to avoid cancellation. That makes the
    derived constants round to Table 3.6.
  - **Rationale:** exact at every height a hobby rocket reaches, cheap enough for the inner loop
    (35.5 ns a call on the dev machine, `docs/perf.md`), and able to reproduce RocketPy's
    formula exactly when a comparison needs it.
- **Reference values:** no publication tabulates normal gravity by latitude and height.
  `validation/oracles/wgs84/normal_gravity.py` evaluates the published formulas at 40 digits with
  mpmath, which was added to the oracle environment. It checks the derived constants against the
  tables and the vector against the gradient of the normal potential. Its JSON output is committed
  under `validation/fixtures/earth/`, and the Rust tests read it with `include_str!`, so CI needs
  no Python.
- **Tables:** `Table1D` offers linear or natural-cubic interpolation, with a `clamp`, `linear` or
  `error` extrapolation policy. Every lookup reports whether it extrapolated. A monotone cubic
  waits until a model needs one (`docs/physics/interpolation.md`).
- **JSON floats:** `serde_json` has `float_roundtrip` enabled workspace-wide, so a parsed number
  is the exact `f64` that was written.

**Consequences.**

- M1.6 must:
  - use the geodetic height for ground contact and apogee;
  - decide whether its rotational dynamics include the frame rate `Ω` (at most 7.3e-5 rad/s);
  - call `Earth::gravity_enu_mps2` and `Earth::rotation_acceleration_enu_mps2` instead of a
    constant.
- M2.1 cases that compare with RocketPy select `vertical_taylor`. They must also account for
  RocketPy feeding height above sea level to its formula, and holding gravity constant above
  `max_expected_height` (80 km by default).
- M1.4 fixes the body reference point and the station mapping `z_B = z_ref − s`.

## ADR-004: Atmosphere, wind, turbulence and the seeded generator (2026-09-17)

**Context.** M1.2 builds `hpr-atmos`. Loft's atmosphere was wrong in five ways (lessons L2–L6):
geometric height fed to geopotential formulas, four layers, the wrong Sutherland constants, the
standard lapse used where a sounding existed, and a wind that stepped at the lowest forecast level.
RocketPy, the M2.1 oracle, interpolates everything linearly and ignores humidity. Several choices
had no single right answer and are recorded here.

**Decision.**

- **Height datum:** atmospheres and winds are queried with geometric height above mean sea
  level. The flight engine converts from ellipsoidal height with the site's geoid undulation.
  Every sample says whether the model extrapolated.
- **1976 standard:** implemented from its equations and constants: `R* = 8314.32`,
  `S = 110.4 K`, and `M/M₀` from 80 to 86 km, which the printed tables omit.
  - The committed fixture is transcribed from the scan and cross-checked by mpmath and
    `ambiance`.
  - Outside −5 to 86 km the lowest layer continues downward, and the air is isothermal above 86
    km. Both are flagged.
- **Temperature offsets:** `ΔT` is added at equal geopotential height, and pressure is
  integrated hydrostatically from a sea-level pressure. `anchored` fits both to a measured
  temperature and pressure.
  - Rejected: the aviation convention (offset at equal pressure altitude, ESDU 77022). It is
    equally hydrostatic, but it detaches the lapse rate from height. A launch-site measurement
    and a sounding are both keyed on height. The two differ by 0.38% in density at 3 km for
    +20 K.
- **Moist air:** an ideal mixture of dry air and water vapour.
  - Saturation vapour pressure is WMO-No. 8 eq. 4.B.1 over water at all temperatures, with no
    enhancement factor.
  - Water vapour `C_p = 4R*`.
  - Dry-air Sutherland viscosity.
  - Measured errors: density 0.047% against CIPM-2007, speed of sound 0.009% from the `C_p`
    choice, viscosity 2.1% at saturation and 30 °C. CIPM-2007 itself was rejected: it is valid
    only from 15 to 27 °C and from 600 to 1100 hPa.
- **Soundings:**
  - A profile takes its site's latitude and works in WMO geopotential height (eqs. 12.15–12.16),
    so its hydrostatics use the local normal gravity (±0.27% from `g₀`). The latitude-free
    geopotential was rejected: it is 0.1% off in pressure over 3 km at the equator and poles.
  - Temperature and humidity are linear in geopotential height, and pressure uses the
    temperature-shaped log form, which is exact for dry hydrostatic layers.
  - Given pressures must fall with height.
  - Omitted pressures are filled hydrostatically with virtual temperature.
  - Beyond the levels, the standard continues, anchored at the end level. Above the top, the
    vapour mole fraction is held and capped at saturation.
  - Geopotential heights convert with WMO-No. 8 eqs. 12.15–12.16 (latitude-dependent).
  - Rejected: linear-in-height pressure as RocketPy does it, which is up to 1.15% off between
    700 and 500 hPa.
- **Wind:**
  - Meteorological "from" directions, and ENU velocities.
  - `LayeredWind` defaults to speed-and-direction interpolation along the shorter arc, which
    keeps a veering wind's speed. A calm level takes the other level's direction.
  - `Components` interpolation reproduces RocketPy.
  - Beyond the end levels the end wind is held and flagged.
- **Turbulence:** Dryden spectra with MIL-F-8785C's scale-length convention (not
  MIL-HDBK-1797's halved transverse lengths).
  - Generated by the exact discretization of the shaping filters, not MIL-HDBK-1797's Euler
    filters, whose variance is biased by `1/(1 − aT/2)`.
  - `GustField` precomputes a realization so the integrator sees a pure function.
  - Medium/high-altitude intensities are caller-supplied: the specification gives them only as
    a graph.
- **Seeded generator:** `hpr_core::random::SeededRng` is xoshiro256++, seeded through SplitMix64,
  with Marsaglia–Bray polar normals. It is written in-house (about 60 lines) and checked bit for
  bit against `rand_xoshiro`, which is only a test dependency.
  - **Rationale:** seeded results must not change when a dependency updates, and a sampling
    crate can change its algorithm in a new release. It also keeps the pure core free of runtime
    dependencies.
  - **Frozen:** changing the algorithm, seeding or draw order needs an ADR.
  - **Portability:** the integer stream is the same on every platform. Normal deviates,
    turbulence and the atmosphere use the platform's `ln`, `exp` and `powf`, so those are
    bit-identical on one platform (the CLAUDE.md requirement) but may differ in the last bit
    across platforms.
  - **Serialized form:** checkpoints write the state as four `0x` hex strings, because JSON
    readers lose integers above 2⁵³.

**Consequences.**

- M1.6 must:
  - convert ellipsoidal height to height above sea level before calling the atmosphere or wind;
  - pick the gust field's path coordinate (distance through the air, or altitude) and how gusts
    start on the rail;
  - use the moist speed of sound and density for Mach and dynamic pressure.
- M2.1 cases against RocketPy need levels dense enough, or a linear-pressure compatibility
  option, and `Components` wind interpolation. RocketPy's Wyoming import converts heights with a
  radius only.
- M5.2 importers convert geopotential heights with the WMO formula, and clamp radiosonde
  humidity above 100% into `[0, 1]`.
- M6.1 reuses `SeededRng` for dispersions.

## ADR-005: Solid motors: statistics, consumption, grains, file models and the bundled catalog (2026-09-17)

**Context.** M1.3 builds `hpr-motor`. Loft's motor code went wrong in eight ways (lessons L36–L43):
one `.eng` block per file, lost delay markers, class letters off at band tops, the last sample as
burn time, a fixed midpoint CG with no inertia, unlicensed curves, loose impulse checks, and
header envelopes trusted over catalog data. RocketPy, the M2.1 oracle, has its own conventions.
Several choices had no single right answer (`docs/physics/motor.md`).

**Decision.**

- **Statistics:** total impulse integrates the straight-line curve exactly, from an implicit
  `(0, 0)`. Burn time follows NFPA 1125: between the 5%-of-peak crossings. Average thrust is total
  impulse over that burn time. This is ThrustCurve's glossary and its site code, which a committed
  fixture runs unchanged (hpr agrees to 1.8e-15 on the bundle); its statistics page words average
  thrust differently, and the difference is about 0.1% of impulse.
- **Impulse classes:** upper limits are inclusive (NAR: "5.01 to 10.0 N-sec" for `C`), from
  `1/8A` to `O`, and on to `Z` by doubling.
- **Consumption:** constant effective exhaust velocity, `m_p(t) = m_p0 (1 − I(t)/I)`. This is
  RocketPy's `SolidMotor` and ThrustCurve's `.rse` mass columns. It is an approximation (NASA
  SP-8039 shows `I_sp` drifting within a burn), recorded as such.
- **Propellant layout:** a fixed-shape column (the default, RocketPy's `GenericMotor` model) or
  BATES grains. The grain regression is solved exactly in burned mass instead of RocketPy's ODE in
  time. Motor mass and inertias agree with RocketPy within 7.9e-5 relative, and the centre of
  mass within 5.8e-6 of the motor length.
- **Envelope default:** from a catalog entry alone, the propellant is a solid column and the dry
  mass a thin tube, both over the full length and centred. Crude, and documented as such; motors
  with data use `SolidMotor::new`. Catalog metadata overrides the curve file's header.
- **Steps:** equal consecutive times in a curve are a step, not an error; decreasing times,
  negative thrust, and curves with no impulse or no NFPA burn time are errors. ThrustCurve's code
  instead averages points under 50 µs apart: identical on every bundled curve, and up to 1.1% in
  average thrust on 17 of 1710 survey files, all with repeated times.
- **Grains** need a bore: a solid end burner isn't a BATES grain, and its regression differs.
- **Ambient pressure:** the full-flow term `(p_ref − p_a) A_e` applies strictly inside the burn,
  as in RocketPy's flight, and only where the curve's thrust is positive. `p_ref`, the test site's
  pressure, is stored with the nozzle, since files don't record it. The term steps in after
  ignition and to zero at burnout, and overstates tail-off thrust (about 1% of
  impulse in vacuum for a 38 mm reload with a known exit); COTS data gives no exit diameter, so
  it is off by default.
- **File models:** `.eng` and `.rse` keep the file's units and points exactly, so read-write-read
  cycles are bit-exact. Readers are lenient and return warnings; writers refuse anything the
  reader would read differently. Readers take `&str`; decoding bytes is `hpr-io`'s job.
- **Delays:** the raw string is kept; `P`, `100` and `1000` read as plugged, and `0` reads as its
  own ambiguous "zero or plugged" setting, never as an ejection at burnout.
- **Curve end:** the thrust is zero from the last sample's instant on, so thrust, mass flow and
  propellant agree at burnout. The motor's dry mass must be positive.
- **Bundled catalog:** only curves ThrustCurve marks public domain, whose total impulse, burn time
  and average thrust each match ThrustCurve's stored values within 1%. That is 32 curves from B to
  O. Of 554 public-domain curves, 196 pass, so the rule leaves out 358. Most miss on burn time or
  average thrust, or their impulse is 1–5% off; the breakdown is in
  `docs/research/thrustcurve-data.md`. "Free" curves (which can be GPL) are never bundled.
- **Catalog metadata:** each bundled motor's published statistics and identifiers (names, class,
  type, dimensions, masses, impulse, thrust, burn time, delays, case) are copied from
  ThrustCurve's API into the committed index, with attribution. ThrustCurve states no terms for
  its metadata. These are facts about commercial products, mostly from certification bodies, for
  32 motors, and the milestone's 1% check and offline catalog need them. Neer confirmed on
  2026-09-18 that they are treated as facts, used with attribution; the catalog keeps them all.
- **XML:** `roxmltree`, a strict read-only parser that refuses DTDs; the `.rse` writer is
  hand-written.

**Consequences.**

- M1.4 places the motor's nozzle exit in the body frame and adds retainers with
  `with_added_dry_mass`.
- M1.6 takes mass, centre, inertias and `ṁ` from `SolidMotor::state`, and decides whether to
  apply the ambient-pressure thrust term (COTS files give no exit diameter).
- M2.1 feeds RocketPy only curves without an explicit `(0, 0)`: RocketPy prepends one, and the
  duplicate makes its impulse NaN. RocketPy's `GenericMotor.load_from_eng` uses the diameter as
  the chamber radius.
- M5 fetches the other curves into a cache and never bundles them.

## ADR-006: Component geometry and mass properties: frames, shapes, walls, fins and materials (2026-09-17)

**Context.** M1.4a builds the geometry and mass half of `hpr-design`. Loft's mass model went wrong
in the ways lessons L44–L46 and L48–L49 name: pitch inertia only, no radial terms, fin span and
tabs ignored, solid centroids for hollow parts, only a tangent ogive, and a kinked transition
profile. The published sources leave several conventions open. OpenRocket's documentation doesn't
say how wall thickness is measured, its ogive parameter is contradictory, and it doesn't say how
a boattail is oriented or where cant pivots (`docs/physics/shapes.md`, `docs/physics/mass.md`).

**Decision.**

- **Full tensor.** `MassProperties` carries mass, a 3D centre and the full inertia tensor about it
  in body axes, with positive products of inertia. Every part computes its tensor from geometry,
  so one- and two-fin sets, off-axis masses and lugs keep their products of inertia. It is not the
  "two nearly equal transverse moments" of OpenRocket's documentation (§4.2.3).
- **Component frames.** Each part has body axes with its origin on the axis at its forward end (a
  nose cone's tip), so it lies at `z ≤ 0`. The tree (M1.4b) translates and rolls it, and fixes
  the body origin `z_ref` that `frames.md` leaves to M1.4.
- **Shapes.** The ogive is Crowell's secant ogive, parameterized by `ρ/ρ_t` (1 is tangent; below
  1 bulges). OpenRocket's `κ` is not adopted, because its documentation defines it two ways; M3.1
  maps it by running the jar. The Haack parameter may reach 2/3, the monotone limit. A transition's
  shape puts its tip at the smaller end. Clipped transitions follow [TD] §A.7. Parameters out of
  range are errors, never defaults.
- **Walls are measured normal to the surface.** The wall is the part of the solid within `t` of the
  lateral surface: its inner surface is the envelope of circles of radius `t` on the profile, ends
  included and not extended, and the wall fills in near a tip. This is how a molded or laid-up
  shell is made.
  - At a cut end where the surface meets the end plane at an obtuse angle inside the wall, the
    inner corner is rounded rather than square: `t² (tan φ − φ)/2` less section per unit rim
    length, 3.1e-4 `t²` at 7°. On steep ends it is not small: 1.26% of wall mass at 56° and 2.24%
    at 60° for a 2 mm wall. M2.2 checks this against OpenRocket.
  - Extending the surface along its end tangent would cut it square. But that closes a steep end
    with a disc, and it made wall mass jump by 8.3% as an end slope rounded from finite to
    infinite, so it was dropped. Radial thickness, as in Crowell, overstates the wall by
  `√(1 + y′²)`. M2.2 measures what OpenRocket does, and any gap goes in the report, not into this
  model.
- **Fin cross-sections change the mass.** The section's thickness distribution is integrated
  exactly: square, rounded with semicircular edges, or airfoil as the NACA four-digit thickness
  distribution. OpenRocket's documentation uses the section for drag only. A slab would overstate
  an airfoiled fin by 46%; the NACA section is a definition, and it is documented as one.
- **Fin cant** pivots about the fin's span axis through the root mid-chord. Tabs are square slabs.
  The flat root sits at the body radius, and fillets wait for a milestone that needs them.
- **Recovery parts and mass components** are solid cylinders of their packed size. A parachute's
  cloth is `πD²/4`.
- **Numerics.** `hpr_core::quadrature` gains an adaptive vector G7K15 (QUADPACK's `QAG` without
  extrapolation), with substitutions and end-relative evaluation at blunt tips. Filled solids match closed forms
  to 1e-10 and 40-digit mpmath integrals to 1e-12; walls match an independent 25-digit mpmath
  envelope to 1e-10 (worst measured 5.9e-12). Principal moments
  use cyclic Jacobi, not the closed-form eigenvalue method that loses `√ε` for repeated moments.
- **Materials** are values stored in the design (name and density with its kind), not library
  keys, so designs stay complete offline. Built-in values cite primary public sources: USDA's Wood
  Handbook, manufacturers' data sheets and military specifications. Hobby tubes (cardboard, kraft
  phenolic, Blue Tube, Quantum) have no published density, so theirs is derived from the makers'
  published weights and dimensions, with the derivation recorded.
- **Crowell (1996)** is cited but not pinned: its only copy is on a plain-http mirror, and the
  Internet Archive was offline. The pinned OpenRocket documentation gives the same curves, and
  closed forms and mpmath integrals check them.

**Consequences.**

- M1.4b builds the tree on these parts: placement, auto radii, overrides, configurations with
  `SolidMotor`, reference diameter and checks. The part types hold resolved geometry. "Auto or
  fixed" radii and overrides belong to the tree's own types, so these serialized forms don't
  change when the tree arrives.
- Fin cant sign: a positive cant turns fin 0's leading edge toward `−y_B`, and a test pins it.
  M1.8 derives the roll-forcing sign from this geometry.
- M1.5 takes wetted areas, planform areas and centroids from `revolve` and the fin planforms.
- M2.2 compares OpenRocket's component masses, and reports the wall-thickness and cross-section
  conventions if they explain differences.
- M3.1 maps `.ork` shape parameters, the clipped flag, shoulders, tabs, instances and packed sizes
  onto these types, and settles the ogive parameter with the jar.

## ADR-007: Design tree: stations, placement, automatic radii, overrides, motors and checks (2026-09-17)

**Context.** M1.4b assembles M1.4a's parts into rockets. Loft's tree placed parts from OpenRocket
conventions it had read from OpenRocket's source, which this project can't do. It chose the
reference diameter from any component, internal ones included (L47). It flew a motor wider than
its mount and fins off the airframe without complaint (L50). RocketPy takes a rocket's mass,
centre and inertia as inputs and adds the motor; it computes nothing from geometry.

**Decision.**

- **Body origin at the nose tip.** `z_ref = 0`, so station `s` (aft of the tip) is `z = −s`.
- **One `Component` type holding a `Part` enum.** Body components (nose cone, body tube, transition)
  are a stage's list and stack from `s = 0` through every stage. External parts (fins, tube fins,
  lugs, rail buttons) hang off body tubes. Internal parts hang off body components or inner tubes.
  Roles are checked when the tree resolves, so importers build one type. Fins on noses and
  transitions are refused until a milestone models a sloped root.
- **Positions** are `top`, `middle`, `bottom`, `after` (the previous sibling) and `absolute`, with
  offsets positive aft. An attached part's extent is its root chord for fins, a whole row for lugs
  and buttons, and its packed length for packed parts.
- **Automatic radii** are a list of named dimensions on the component; stored values for them are
  ignored. Body radii take neighbours across stage boundaries. The previous component wins for
  tubes, and the next is a fallback. Shoulders take the adjoining tube's bore, rings their parent's
  bore and the widest overlapping on-axis sibling tube, and packed parts the bore.
- **Overrides** apply mass (rescaling the tensor with the mass), then the centre (along the axis
  from the forward end of what is overridden, and optionally off it, keeping the tensor about the
  centre), then the full tensor. A component's overrides cover it alone, or its subtree with `overrides_include_children`.
  A stage's cover the stage. Deeper overrides apply first; motors are never covered. The inertia
  override has no OpenRocket counterpart; RocketPy's examples need it.
- **Motors.** A `MotorMount` on a body tube or inner tube, with an overhang. Configurations put an
  embedded `SolidMotor`, with its case diameter and length, in each mount. The nozzle exit sits at
  the mount's aft end plus the overhang, on the mount's axis. All motors ignite at `t = 0` until
  M1.9. Catalog references wait for the facade (M4.1) and the online catalog (M5.4).
- **Reference diameter** defaults to the widest body component in any stage. `nose_base` and
  `custom` are the alternatives.
- **Checks** return typed findings with a severity.
  - Errors: a motor wider than its mount or wholly outside it, an external part that doesn't
    overlap its body tube, an internal part off the rocket or reaching past its parent's bore
    (measured about the parent's own axis), and a stage centre moved off the rocket by an override.
  - Warnings: a motor past its mount's top, attachments and internal parts past their parent's ends,
    a ring crossing an inner tube, radius steps, no nose cone.
  - Lengths compare with 1 nm of slack.
  - The flight engine (M1.6) must refuse designs with errors unless the caller accepts them.
  - `Layout::place_motors` takes any configuration, so a candidate motor is checked before it is
    stored.
- **Errors and limits.** An error inside a stage or component carries its id
  (`DesignError::InComponent`). Components nest at most 32 levels. `MassProperties::validate` now
  also refuses inertia on a body with no mass.
- **Public test designs** live in `validation/designs/` as JSON of `hpr_design::Rocket`. The format
  is provisional: M3.3 defines the open format and migrates them.
- **RocketPy comparison with substituted curves.** RocketPy's data files carry their own terms
  (`THIRD-PARTY-NOTICES.md`).
  - The committed fixture takes inputs only from RocketPy's notebooks and test code. It keeps each
    example rocket's inputs exactly, and pairs its motor with the bundled public-domain curve
    nearest in impulse.
  - Valkyrie is left out, because its inputs exist only in a data file.
  - Mass composition doesn't depend on which curve schedules the burn. Values at ignition are
    identical, and at burnout they agree to 1e-9. A local run with the examples' own curves stays
    under `refs/`.
  - The fixture also samples RocketPy at its LSODA knots. There hpr agrees to the solver's
    accuracy (1e-9), and the test holds 1e-8.

**Consequences.**

- M1.5 takes stations, radii and the reference area from `Layout`.
- M1.6 refuses designs with error findings, and takes `Assembly::mass_properties(t)` as the
  time-varying mass model.
- M1.9 adds ignition times, separation and per-stage assemblies to configurations.
- M2.2 measures OpenRocket's override order (L51) and automatic-radius rules with the jar. M3.1
  maps `.ork` positions, auto flags and overrides onto these types, and reports any rule that
  doesn't map.

## ADR-008: Subsonic normal force and centre of pressure (2026-09-17)

**Context.** M1.5a adds Barrowman's normal-force slope and centre of pressure. The sources
disagree in places. Barrowman's 1966 report fits ogive noses with 0.466 L, and his 1967 thesis
adds slender-body interference terms that the report leaves out. Niskanen (2009) keeps
`sin α/α` and adds body lift. The OpenRocket technical documentation (13.05) replaces the thesis's
roll-dependent three- and four-fin terms with plain `N/2`. Barrowman's own TIR-33 (1970) treats
six fins with its own interference factor. Loft reused the conical CP for every transition (L9),
had no fin-count correction (L8), and swapped elliptical fins for an equal-area trapezoid (L10).

**Decision.**

- **Bodies.** `(C_Nα)_B = (2/A_ref)ΔA` and `X_B = [l A(l) − V]/ΔA`, with `V` integrated from the
  real profile (no 0.466 L fit). Moments are summed as `(2/A_ref)[l A(l) − V]`, which stays well
  conditioned when `ΔA → 0`. The potential term carries `sin α/α` (Niskanen eq. 3.19). No Mach
  term.
- **Body lift.** Galejs's `K (A_plan/A_ref) sin² α` with `K = 1.1` at the planform centroid
  (Niskanen eq. 3.26–3.27), on every body component.
- **Fins.** Diederich's slope with `β = √(1 − M²)` (Barrowman 1967 eq. 3-6), the mean aerodynamic
  chord integrals with the CP at its quarter chord for all subsonic Mach (Barrowman 1967 p. 6).
  Niskanen's aft CP shift above Mach 0.5 (eq. 3.35–3.36) belongs with the supersonic fit it
  interpolates to and moves to M1.8. Freeform fins follow Niskanen pp. 27–29: the filled chord
  for the CP, the true area for the slope, the span-averaged mid-chord angle.
- **Fin count and roll.** `Σ sin² Λ_k` (exactly `N/2` for three or more fins) times 1, 0.948,
  0.913, 0.854 or 0.810 for up to 4, 5, 6, 7 or 8 fins (technical documentation eq. 3.54, from
  MIL-HDBK-762(MI) p. 5-24). More than eight fins are refused: the documentation's 0.750 has no
  source. One- and two-fin sets also report a side force across the flow's plane,
  `Σ sin Λ_k cos Λ_k`, derived from Niskanen's per-fin angle `α sin Λ_k` (eq. 3.50). Niskanen drops
  it, arguing that it cancels for two or more fins, but for two fins it adds. Fin sets at the same
  station are not combined.
- **Interference.** `K_T(B) = 1 + r_t/(s + r_t)` (Barrowman 1966 eq. 77, a fit for
  `r_t/(s + r_t) < 0.4`), not the thesis's full slender-body terms; the fin-induced body lift
  `K_B(T)` is neglected, as in the report.
- **Scope.** `0 ≤ M < 1`; `M ≥ 1` is an error until M1.8. The sources document the subsonic
  models only to Mach 0.8, and Niskanen's fin CP shift would already be 0.05 MAC aft there, so
  0.8–1 is an unvalidated extrapolation, accepted so M1.6 can fly through it until M1.8. Tube fins
  and unknown part kinds are refused. Lugs and rail buttons add no normal force. Cant is ignored
  until roll (M1.8). The model is a small-angle model; `α` is accepted over `[0, π]` so the flight
  engine can decide what to do near apogee. A slope that cancels to below 1e-12 of its terms has no
  CP; `moment_m` is always defined.
- **Six fins: MIL-HDBK-762 over TIR-33.** TIR-33 handles six fins with `K = 1 + 0.5 R/(S + R)` and
  no fin-count factor, without a derivation and for six fins only. MIL-HDBK-762 gives six and eight
  fins from slender-body theory, the technical documentation interpolates five and seven, and
  OpenRocket (the M2.2 oracle) uses the same factors, so a comparison there isolates other
  differences. With `x = r/(s + r)`, hpr's six-fin slope over TIR-33's is
  `0.913 (1 + x)/(1 + 0.5 x)`: −8.7% as `x → 0`, −4.3% at 0.1, +3.2% at 0.3 (the Recruiter) and
  +6.5% at 0.4, the edge of the interference fit's range.
- **Radius steps (an extrapolation).** Where one body component's aft radius differs from the next
  one's fore radius, the step adds `(2/A_ref)ΔA` at the joint (a zero-length transition), so the
  body's total slope is Barrowman 1966 eq. 10 over the whole body. Barrowman 1967 p. 18 assumes no
  discontinuities, so this goes beyond the source; dropping the step would lose its slope silently.
  It is reported as part of the aft component. A body that starts blunt (no nose cone) gets no term
  for its front face, as eq. 10 gives.
- **Validation, and the gap it leaves.** Barrowman's five published worked examples (NARAM-8's
  Testbed II and Aerobee 350; TIR-33's Javelin, Recruiter and Arcon-Hi), every printed component
  and total. With hpr's own model, four examples agree within 1% (worst −0.77%), and the
  Recruiter's six-fin slopes do not: +3.42% (fins) and +2.87% (total). That gap is this ADR's
  six-fin choice, measured: the rules differ by +3.22% and +2.83%, and with TIR-33's rule
  substituted in the slope and the CP weighting the Recruiter agrees within 1% (worst +0.19%). The
  test pins that exactly those two values fall outside 1% with hpr's model, and prints both.

**Consequences.**

- M1.6 builds an `AeroModel` once per design and calls `normal_force` (about 10 ns) per
  derivative evaluation, using `moment_m` for the moment (defined even with no net force). It
  decides how to treat large angles of attack.
- Unknown part kinds are refused, so a new `Part` variant needs an aerodynamic decision.
- M1.8 adds the transonic and supersonic slopes, the fin CP shift, roll forcing and damping.
- M2.2 compares CNα and CP against OpenRocket, which uses the same fin-count factors.

## ADR-009: Subsonic drag buildup, surface finishes and drag override tables (2026-09-17)

**Context.** M1.5b adds drag. Niskanen's thesis (2009, §3.4 and appendix B) is the primary
source; the OpenRocket technical documentation 13.05 reprints the same drag equations unchanged,
and Barrowman's 1967 thesis (ch. 4) is the source of its friction, roughness and leading-edge
formulas. The sources leave gaps: no coefficients for drag at angle of attack, undefined areas in
the boattail rule, an unstated diameter in the lug rule, no rail buttons, and friction that jumps
where eq. 3.81 switches branches. Loft merged fin sets (L11), used uncited constants (L12), had no
power-on base relief (L13), uncited lug drag (L14), no drag at bare steps (L15) and a silent cap of
10 (L16). The done-when compares with RocketPy's RASAero curves, whose inputs aren't recorded.

**Decision.**

- **Buildup.** `C_D0` = friction + pressure + base + parasitic on the reference area (eq. 3.75,
  3.97); `C_A = C_D0 f(α)`, positive toward the tail. Each component keeps its own terms (fin sets
  are never merged), and `AeroModel::buildup_components` reports them.
- **Friction.** Fully turbulent (Niskanen p. 43, who measured laminar runs changing apogee by under
  5%), `R` on the rocket's length (nose tip to
  the aft end of the last body component), eq. 3.78–3.84 as printed, the `R < 1e4` branch first,
  keeping the jumps at `R_crit` and at Mach 1. Roughness is per component (`hpr_design::Finish`),
  always over the rocket's length. The body form factor `1 + 1/(2 f_B)` uses body length over
  maximum body diameter; the fin factor `1 + 2t/c̄` is per fin set.
- **No laminar or transitional friction (M1.5's scope bullet names it).** Barrowman 1967's
  transitional form (eq. 4-2, 4-6: laminar below `R = 5e5`, `− 1700/R` above, turbulent throughout
  on any rough surface) and RASAero II's default laminar trip at `5e5` would lower smooth-surface
  friction by `1700/R`, about 4% of it and 2% of `C_D0` on Calisto (`R` 1.8e7). Following Niskanen,
  hpr stays fully turbulent; an option is issue #18.
- **Friction area (a departure).** A body's friction area is its axial projection
  `2π ∫ r dx = π A_plan`, not the slant surface of eq. 3.85: only the axial share of the wall shear
  makes drag. It changes slender noses by about 1% of their own friction (2.4% for a tangent ogive
  of fineness 2). With the slant surface, a shoulder's drag would stay about `C_fc ΔA/A_ref`
  (0.0015 in the L15 test) above a bare step's as its length goes to zero. M2.2 will see the
  difference against OpenRocket.
- **Pressure drag.** Noses and shoulders `0.8 sin² φ`, `φ = atan(dr/dx)` at the aft joint, on the
  increase in area (eq. 3.86), held through subsonic flow. Boattails follow eq. 3.88 on their
  decrease in area. Niskanen writes `A_base/A_boattail` without defining them and says a
  zero-length boattail drags like "the total base drag" (p. 48). Read with `A_base` as the aft
  base, a zero-length boattail would count that base twice and leave the uncovered annulus out, so
  hpr reads both as the decrease in area (Calisto's boattail: 0.052, against 0.046 the other way).
  Steps in radius are zero-length shoulders (`0.8 ΔA`) or boattails (base drag on `ΔA`); a body
  without a nose cone gets `0.8 A` on its face.
- **High subsonic.** Eq. 3.87 interpolates nose and shoulder pressure drag from eq. 3.86 at
  Mach 0 to appendix B's value and slope at Mach 1 (closed forms for cones and ogives, Stoney's
  data for other shapes); all of it arrives with M1.8. Until then the drag reads low from about
  Mach 0.6: a 3:1 tangent ogive misses 0.006 at Mach 0.7 and 0.021 (4–5% of `C_D0`) at 0.8, a 2:1
  cone 0.037 at 0.8. The buildup accepts Mach numbers up to 1 so M1.6 can fly, and sets
  `Drag::beyond_subsonic_methods` above Mach 0.8, the top of Niskanen's subsonic region (Table 3.1);
  the flag marks that edge, not the start of the error.
- **Base drag.** Eq. 3.94 on the last body component's aft area less the thrusting motors'
  cross-section (`DragConditions::thrusting` with the burning motors' area; Niskanen p. 50), down
  to zero.
- **Fins.** Eq. 3.89–3.93 by cross-section (square, rounded, airfoil) on `N t s`; `Γ_L` is
  `atan(x_t/s)` for trapezoids, the span average for freeform outlines, and a closed-form average for
  ellipses. Averaging the angle follows eq. 3.91; the drag goes as `cos² Γ`, whose span average is
  6% lower for an ellipse of `k = 1` (for M2.2).
- **Parasitic.** Launch lugs follow eq. 3.95–3.96 with `d` the outer diameter (the rail-pin passage
  on p. 52 only reads that way). Rail buttons follow the rail-pin rule, the stagnation coefficient
  on their side profile. Neither adds friction.
- **Angle of attack (derived).** Niskanen gives only the shape: 1 at 0°, 1.3 at 17°, 0 at 90°, zero
  slope at each. hpr uses the unique cubic per part that meets those conditions. Past 90° the flow
  meets the tail and hpr mirrors with the sign reversed, `f(α) = −f(180° − α)` (an assumption), so
  drag still opposes the motion. M2.2 compares with OpenRocket.
- **Refusals.** `M ≥ 1` for the buildup (the term functions are defined and finite to Mach 5),
  geometry the terms can't use, negative roughness (already at `Rocket::layout`), a coasting
  condition with a motor area, non-finite conditions and non-finite results are errors. Nothing is
  clamped.
- **Finishes.** `hpr_design::Finish` names the fifteen rows of Barrowman 1967 Table 4-1 (after
  Hoerner p. 5-3; Niskanen Table 3.2 reprints ten) plus a custom height, on `Component::finish`.
  The default is "paint in aircraft mass production", 20 µm: no source gives a hobby-rocket
  default, RASAero II defaults to smooth, and OpenRocket's "regular paint" is 60 µm (Niskanen
  p. 83).
- **Override tables.** `DragTable` holds `C_D0(M)` power-off and optionally power-on, read from CSV
  text: two columns (RocketPy's curves), or a named column of a headed file with rows at non-zero
  `Alpha` skipped (RASAero II's export). An identical repeated row is skipped; a Mach number
  repeated with another value, or out of order, is refused with its line. Linear interpolation,
  end values held, extrapolation reported. `DragConditions::thrusting` selects power-on, and an
  optional reference diameter rescales a table made on another reference area. The angle-of-attack
  factor still applies, and a model with a table accepts any Mach number for drag.
- **The comparison with RocketPy's curves.** Every RocketPy example whose drag curve is labelled
  RASAero: Calisto, Juno III, Cavour and Valetudo. At Mach 0.3 and USSA76 sea level (RASAero II
  computes its exports' Reynolds numbers at sea level, Users Manual p. 84). The curves stay in
  `refs/`; `cargo xtask aero` writes only derived numbers (each curve's value at Mach 0.3, hpr's
  `C_D0`, the error, the file's sha256) to `validation/fixtures/aero/rocketpy-drag-curves.json`,
  `hpr_aero`'s test recomputes hpr's values and the errors, and an xtask test reruns the comparison
  when `refs/rocketpy` is present. Provenance, traced through RocketPy's history:
  - Calisto's curve is a RASAero II export: the alpha-0 `CD Power-Off` column of the `CD Test.CSV`
    in RocketPy's first commit (2018). Its power-on column equals power-off (RASAero's v1 manual:
    power-on equals power-off without a nozzle exit diameter), so only power-off is compared. The
    design with the 2018 notebook's fins is the case: the export's `C_Nα` (6.09 per rad) and CP
    (66.2 in) are within 1.5% of hpr's for those fins (6.18, 66.2 in), not for the getting-started
    fins (7.32, 70.3 in), which are reported as a variant, not as more evidence.
  - Juno III's, Cavour's and Valetudo's are labelled RASAero but are 3-decimal tables with no input
    file. Juno III's serves power-off and power-on alike; Cavour's and Valetudo's have separate
    power-on tables, which are compared too (Calisto's power-on file is its power-off file; hpr's
    power-on result would be −5.0%). Cavour's power-off table has 22 extra rows repeating 13 Mach
    numbers up to 0.107, 7 with values 0.001 apart; the comparison keeps the first of each, and
    `parse_mach_csv` itself refuses that curve.
- **Inputs for the comparison: one declared rule, placeholders where nothing is known.** The
  exports record no inputs. Finish: RASAero II's documented default, smooth (p. 53,
  `Finish::Mirror`). Fin cross-section: a NACA 00xx airfoil file in the example gives an airfoil
  section that thick at the mean aerodynamic chord (Calisto's getting-started fins, NACA 0012); a
  published section is used (Juno III's team was cited for "análise de aletas com perfil de
  aerofólio truncado", an analysis of truncated-airfoil fins: a rounded leading edge and a blunt
  trailing edge, taken as rounded, with no thickness given); otherwise the M1.4b placeholder,
  square 3 mm (Calisto's 2018 fins, Cavour, Valetudo). A lift-curve airfoil says
  nothing about the edges and doesn't count. Rail buttons stay as the RocketPy examples define
  them; Calisto's come from RocketPy's test fixture, not the 2018 notebook.
- **Result, and the gaps it leaves.**

  | case | error | range over the inputs below |
  |---|---|---|
  | Calisto, 2018 fins | +4.4% (+1.8% without the buttons) | −14.0% to +12.8% |
  | Calisto, getting-started fins (variant) | −7.3% | −12.9% to +19.3% |
  | Juno III | −6.0% | −10.5% to +24.1% |
  | Cavour, power-off | −8.3% | −22.3% to −0.4% |
  | **Cavour, power-on** | **−18.3%** | −32.2% to −10.3% |
  | **Valetudo, power-off** | **−47.0%** | −59.4% to −42.5% |
  | **Valetudo, power-on** | **−50.4%** | −62.8% to −45.9% |

  The range is over square, rounded and airfoil fins (3 mm, or 12% of the chord for the airfoil),
  0 or 20 µm, with and without rail buttons. Before the published-section rule, square 3 mm fins
  gave Juno III +14.6% and the getting-started Calisto +10.7%. The test pins exactly the three
  cases outside 10%:
  - **Cavour under power, cause open.** At Mach 0.3, subtracting the motor's area removes 42% of
    Cavour's base drag (0.055) and 29% of Valetudo's (0.038). Cavour's power-on table is within its
    0.001 rounding of power-off from Mach 0.16 up (0.0001 at 0.3; 0.001 to 0.013 lower below),
    Valetudo's 0.004 lower, about a ninth of hpr's relief. The RocketPy designs have no motor case,
    so their motor diameter is the larger of the grain and nozzle exit diameters, and Cavour's
    result runs from −8.3% with no relief to −20.8% with its 75 mm motor (−18.3% at the 67 mm
    nozzle exit). The miss may be Niskanen's relief, a RASAero run with little or no nozzle exit
    diameter, or tables sampled along a flight (their uneven Mach spacing suggests it;
    unconfirmed). Flights with known motors (M2.1, M2.3) will test the relief.
  - **Valetudo's table** gives 1.05 at Mach 0.3, 1.44 times the OpenRocket export for the same
    rocket in RocketPy's RocketPaper repository (0.728). With that `.ork`'s inputs (regular paint,
    60 µm; two 14 mm × 30 mm lugs instead of rail buttons; its 3 mm square fins) hpr gives 0.714,
    1.9% under the OpenRocket export and 32% under the table. RocketPy's own notebook rescales the
    table by 0.9081/1.05.
  - **What the passing cases show.** Unrecorded inputs move each by 20% or more, and each passing
    rocket falls outside 10% under some plausible inputs. The check places hpr near RASAero's
    subsonic drag with a declared rule; it can't show agreement to 10% without the inputs. M2.2
    (OpenRocket, with known inputs) is the sharper test.

**Consequences.**

- M1.6 supplies `DragConditions` (Reynolds number per metre from the airspeed and the atmosphere's
  kinematic viscosity; whether a motor is thrusting and the burning motors' case area), uses
  `axial_coefficient` along `−z_B`, and decides what to do with `beyond_subsonic_methods`.
- M1.8 adds eq. 3.87's nose and shoulder interpolation, the transonic and supersonic branches of
  every term, and extends override tables to `C_Nα` and CP.
- M2.1's same-drag mode uses `DragTable`. It must decide whether RocketPy's drag scales with angle
  of attack as hpr's does, and how to read curves with repeated Mach numbers such as Cavour's, which
  `parse_mach_csv` refuses; its flights test the power-on base relief.
- M2.2 checks the angle-of-attack polynomial, the lug diameter, the boattail reading, the friction
  area and the leading-edge sweep average against OpenRocket.
- `AeroModel::drag` takes about 50 to 110 ns (`docs/perf.md`).

## ADR-010: Time integration: Dormand–Prince with dense output, RK4, stop times and events (2026-09-17)

**Context.** M1.6 asks for adaptive Dormand–Prince 5(4) with dense output and event
root-finding, plus a fixed-step RK4 option. M1.6 is split into M1.6a (the integrator and events)
and M1.6b (the flight). Loft's RK4 had no error control and no apogee convergence check (L21). It
didn't root-find events (L22), and it let burnout fall inside steps (L23). No crate in the
workspace had an ODE integrator or a root finder. Hairer and Wanner's `DOPRI5` is the reference
implementation of the method in their book and is BSD-2-Clause.

**Decision.**

- **One integrator in `hpr_sim::integrator`, no dependency.** It works on `[f64; N]` states
  through an `OdeSystem<N>` trait whose derivative can fail with the system's own error.
  Established Rust ODE crates either lack event location on dense output or pull in a
  linear-algebra stack. The method is a few hundred lines, and owning it lets the tests pin every
  coefficient.
- **Port `DOPRI5` as the reference.** Take its coefficients, the RMS error norm, the PI controller
  (`β = 0.04`, growth limits 1/5 to 10, safety 0.9), the starting step and the dense output
  (`CONTD5`). The stiffness detection is left out. The port is attributed in
  `THIRD-PARTY-NOTICES.md`, and the source is pinned (`hairer-dopri5`).
- **Per-component tolerance weights come from the system.** `Adaptive` holds one relative and one
  absolute tolerance, so it serializes, and `OdeSystem::absolute_tolerance_weights` scales `atol`
  per component. The default is `rtol = atol = 1e-8`. M1.6b sets the flight's weights and may
  change the defaults, from its benchmark.
- **`advance(system, t_stop)` is the only driver, and the system carries everything.**
  - `OdeSystem<N>` has the derivative and provided methods for tolerance weights, events and
    `accept_step`. One flight-phase type can then compute forces, define its events, record
    each step and stop the run, without the double borrows that separate event and observer
    arguments would force.
  - `advance` stops exactly at `t_stop`, at events (`Advance::Events`), or when `accept_step`
    breaks (`Advance::Stopped`).
  - Discontinuities are stop times, and the caller sets the phase between calls, because the step
    that ends at a stop time evaluates its last stage there.
  - Each call evaluates `f` afresh, and the step-size estimate carries over.
- **Events are sign changes between step ends, located by Brent's method on the dense output** to
  1e-12 s. The integrator stops at the end of the final bracket on the far side of the zero, with
  the dense-output state there, and reports every event past its zero at that state
  (`fired_events()`).
  - Stopping past the zero guarantees that the next call doesn't report those events again. A
    fresh full-order step can land a hair short of the zero and trigger again. Reporting all of them
    keeps coincident events, such as two deployments at apogee, from being lost.
  - The cost: the state at an event is fourth order (third for RK4) rather than fifth. The tests
    show event times within about 1.5e-8 s at the default tolerances.
  - Double crossings inside one step go unseen, and `max_step_s` bounds that. A `reset` that moves
    an event function back across zero re-fires it, so systems normalize inside their functions.
- **Departures from `DOPRI5`, all for robustness.**
  - A derivative that fails in a trial stage, or a non-finite error estimate, is a rejection. It
    becomes an error only when the step can't shrink.
  - A final interval within rounding counts as reached.
  - An overflowing step is an error.
  - The controller, the starting step and the error norm are otherwise `DOPRI5`'s, pinned by
    Hairer's Arenstorf problem, whose evaluation and step counts match an independent
    transcription.
- **RK4 is fixed-step,** shortened only to land on a stop time or an event. Its dense output is the
  cubic Hermite interpolant, whose end derivative is the next step's first stage.
- **Failures are errors, never quiet stops.**
  - The errors are `Derivative`, `StepTooSmall`, `NotFinite`, `StepLimit` (default 10⁶ attempted
    steps, adjustable with `set_step_limit`), `EventNotFinite`, `Backward` and `Settings`.
  - The integrator stays at its last accepted step and can resume.
  - M1.6b maps the errors to termination reasons (L25).
- **Settings files.**
  - `Method` is tagged `dopri5` or `rk4` and refuses unknown fields.
  - `Adaptive` keeps public fields with `Default`, for `..Adaptive::default()`. The crates are
    unpublished, so adding a field later is acceptable.

**Consequences.**

- M1.6b builds the flight as `OdeSystem<13>` phases (rail, powered, coast) separated by stop times
  (motor ignition and burnout, thrust-curve knots where they matter) and events (liftoff, rail
  exit, apogee, ground contact from the ellipsoidal height). It sets the tolerance weights for
  position, velocity, quaternion and body rate. It normalizes the quaternion where it is used, and
  resets only away from events.
- The recorder samples `Step::state_at` at its output times rather than forcing steps onto them.
- Stiff phases (a canopy opening in M1.7) will show up as small steps or `StepTooSmall`. M1.7
  decides whether they need a bounded step or a different method.

## ADR-011: Rigid-body flight: equations of motion, aerodynamic coupling, rail, phases and termination (2026-09-17)

**Context.** M1.6b builds the flight on the M1.6a integrator. It needs:

- a variable-mass rigid body with jet damping;
- aerodynamic forces from `hpr-aero`, which has no damping coefficients and refuses `M ≥ 1`;
- a rail with friction and button geometry, and events;
- a flight that names why it stopped;
- a Level 2 flight in 5 ms.

M2.1 will compare with RocketPy, whose documented equations are MIT. Loft's lessons L20, L24, L25
and L26 set tests.

**Decision.**

- **Equations: RocketPy's documented variable-mass formulation about a body-fixed point, with
  that point at the nose tip.**
  - RocketPy's technical documentation, Equations of Motion v0/v1 (Kane plus Reynolds transport,
    quasi-steady internal flow), covers centre-of-mass
    migration, `İ`, the jet's Coriolis force and jet damping in one exact form.
  - With the reference point at the body origin, the state needs no conversion to the design's
    stations.
  - The equations are written out in `flight.md` and checked against their classical limits:
    torque-free Euler motion about the centre of mass, and the classical jet damping
    `I_c ω̇ = [ṁ(r_e²/4 + l²) − İ_c] ω` (to 1e-6).
  - Measured thrust curves already contain the internal momentum terms `T04` subtracts
    (`−m r″ − 2ṁ r′ + m̈(n − r)`). They are kept, as RocketPy keeps them, for M2.1's comparison.
    The cost is at most 0.05 m/s at burnout and 21 N at Valetudo's liftoff (`flight.md`). Revisit
    after M2.3's flight data.
- **The nozzle gyration tensor comes from the integral, not from RocketPy's code.**
  - `S = (r_e²/4) diag(1, 1, 2) + |n|² 1 − n nᵀ`.
  - RocketPy 1.13.0 codes the transverse distance term as `0.25·n²`. M2.1 has to account for the
    difference.
- **Mass-property rates by central differences inside the integration interval** (half-width
  1e-4 s).
  - `hpr-motor` gives no derivatives of the grain inertia.
  - Stop times at every thrust-curve knot and burnout keep each difference on one side of every
    kink, and the mass rate matches the motor's `−F/c` to 1e-6.
  - The four `mass_properties` calls per evaluation are most of the 0.4 µs an evaluation costs
    (`perf.md`). Analytic motor derivatives are a later optimization, not a correctness need.
- **Earth's rotation: Coriolis force at the centre of mass, no Earth rate in the rotational
  equations.** It is at most 7.3e-5 rad/s against body rates of 0.01–10 rad/s. RocketPy does the
  same. Gravity is normal gravity at the centre of mass.
- **Aerodynamics component by component at the local flow.**
  - Each body and fin set sees `v_O − wind + ω × p_i` at its small-angle CP. That gives the pitch
    and yaw damping `hpr-aero` doesn't have, as RocketPy does.
  - The axial force comes from the whole rocket at the centre of mass's airspeed, power-on while a
    motor burns.
  - The linear pitch model matches the flight's period to 8e-5.
  - `hpr-aero` gains `component_count`, `component_normal_force` (allocation-free) and
    `component_station_m`.
- **Fins follow the crossflow at any angle: `C_Nα sin α` instead of `C_Nα α` in flight.**
  - This is Niskanen's substitution for bodies (eq. 3.16–3.17), applied to fins. No large-angle
    fin model is in hand, and stall is not modelled.
  - It changes nothing at small angles and makes the force vanish for tail-first axial flow.
  - With the linear force, a calm vertical flight falling tail first after apogee flipped the fin
    force with rounding noise and ran to the step limit (found in review, pinned by a test).
  - `hpr-aero`'s own `normal_force` stays linear. M1.8 should move a large-angle fin model there.
- **Motors are evaluated inside their burn at interval ends.** A stage evaluated on ignition or
  burnout takes the one-sided limit inside the burn, because the pressure correction switches
  there. The step ending at burnout otherwise saw the burnt-out thrust, and RK4 converged at first
  order.
- **Out-of-range aerodynamics stop the flight.**
  - `M ≥ 1` anywhere is a `SimError::Aero` until M1.8. There is no silent clamp.
  - Large angles of attack use the small-angle models extended as above, recorded in
    `Sample::angle_of_attack_rad`.
- **Rail.**
  - The aft end starts at the rail's foot.
  - The rocket is guided with one degree of freedom until the aft edge of its last rail button or
    lug passes the top (L26), or its aft end without guides. RocketPy ends at the forward button.
  - Tip-off rotation is not modelled.
  - Coulomb friction `μ|ΣN|` uses the net rail reaction across the axis (weight, aerodynamic and
    mass terms), not the sum over the buttons. The default `μ = 0`, for want of a cited value.
  - The pad phase holds the rocket until the force along the rail beats friction. A stall on the
    rail returns it to the pad, held where it stopped rather than sliding back.
- **Events and termination.**
  - The events are liftoff, rail exit, burnout (a stop time), apogee (the centre of mass's
    ellipsoidal-height rate), ground hit (the centre of mass at the site's ellipsoidal height) and
    user events on a `Sample`.
  - Flights end as `GroundHit`, `NoLiftoff`, `StalledOnRail`, `TimeCap` or `StepLimit` (L25).
    Everything else is an error.
  - The atmosphere takes `h − N` with the geoid undulation given in `Environment`.
- **Defaults: `rtol = atol = 1e-8`, unit weights.** A Level 2 flight takes 1.1 ms and its apogee
  is within 1.1e-6 m of the converged value. At 1e-6 it would take 0.6 ms and 7e-5 m.
- **Observation.**
  - An `Observer` trait sees every accepted step (`FlightStep`: the dense state, and a `Sample` from
    one evaluation) and every event.
  - `Recorder` keeps chosen `Channel`s at a fixed interval or at every step, plus event rows.
    It is built, not deserialized, and cleared between flights.
  - Runs don't mutate the `Simulation` (L24).
  - `Environment` shares its atmosphere and wind through `Arc`, so it clones cheaply, and
    `Simulation` is `Send + Sync` for parallel Monte Carlo.

**Consequences.**

- M1.7 adds recovery devices as phases and events on this engine. A parachute's opening is stiff;
  bound the step there.
- M1.8 replaces the Mach refusal and adds roll forcing and damping. Until then the roll rate
  changes only through inertia coupling.
- M1.9 adds ignition times and staging (new stop times and a changing assembly).
- M2.1's RocketPy comparison must align the rail-exit convention, the nozzle gyration tensor and
  RocketPy's per-surface stations.


## ADR-012: Recovery: drag areas, triggers, inflation and the descent phase (2026-09-17)

**Context.** M1.7a adds parachutes to the M1.6 flight engine. What is in hand:

- Knacke's *Parachute Recovery Systems Design Manual* (NWC TP 6575, 1991), which prints canopy
  drag coefficients on the nominal area, canopy fill constants, drag-area growth laws, opening-load
  methods and the equilibrium descent speed. Its title page limits distribution, so it is cited,
  never redistributed, and no text is copied.
- RocketPy 1.13.0 (MIT), whose parachute phase is a point mass and which M1.7a is compared against
  within 3% (the milestone's *done when*).
- Loft's lessons L27 (instant inflation, no opening load, an unsourced body term, no drogue
  release), L28 (a step floor and a time cap that left descents unlanded), L29 (a parachute `C_D`
  copied from OpenRocket's GPL source) and L92 (the terminal-descent case).
- `hpr-design` already has parachute and streamer **parts**, which carry mass and packing but no
  drag or deployment data.

**Decision.**

- **The milestone is split.** M1.7a is parachutes, triggers, inflation, release, drift and landing,
  with both of M1.7's *done when* bullets. M1.7b is streamers, tumble and separated bodies, with
  its own bullets. Streamers and tumble need drag sources Knacke does not have (he has no streamer
  data at all), and separation needs a decision about how a body's mass and drag are defined; that
  is a milestone's worth of work on its own.
- **Recovery devices live in `hpr-sim`, not in the design tree.** A `Device` is a drag area, a
  trigger, a lag and an inflation law, given to a `Simulation` through `with_recovery`. The design's
  `Parachute` part stays what it is: mass and packing. A design file that carries deployment data is
  M3.1's problem (`.ork` has it) and can map onto these types.
- **Drag areas are `C_D S`, either given directly (RocketPy's `cd_s`) or from a canopy's nominal
  diameter with `C_D0` on the nominal area `S₀ = π D₀²/4`** (Knacke's convention, printed page 5-2).
- **Canopy data comes from Knacke's tables, with the printed page at each accessor**, for thirteen
  types: the `C_D0` range (Tables 5-1 and 5-2), the fill constant (Table 5-6, unreefed), Pflanz's
  drag-area growth exponent (Figure 5-51) and the infinite-mass opening-force coefficient `C_x`.
  - **The default `C_D0` is the middle of the printed range** (flat circular: 0.775 of 0.75 to
    0.80). Knacke prints no single value, and hpr will not copy OpenRocket's 0.8 (L29). Where his
    tables say "insufficient data" the accessor returns `None` and the caller must supply a number.
  - RocketPy's default `C_D` of 1.4 is a hemispherical canopy's coefficient on the **projected**
    area, used only to turn `cd_s` into a radius for its added mass. It is not a `C_D0`.
- **Inflation: `(C_D S)(t) = (C_D S)₀ min(1, (t − t_d)/t_f)^j`**, with `t_f` either zero (instant,
  RocketPy's model), fixed, or Knacke's `t_f = n D₀/v` from the airspeed at line stretch.
  - Knacke's measured **overshoot** (10 to 80% above the steady drag area, Figure 5-40) and his
    `C_x`/`X1` opening-load methods are **not** modelled. hpr's peak load is therefore a lower
    bound on the real opening shock, and instant inflation is hpr's own upper bound. Both are
    stated in `docs/physics/recovery.md`; Ludtke's and Pflanz's laws are later work.
    - *Corrected 2026-09-18 (M0.4b's physics review):* neither bound holds in general. Opening at
      once is Knacke's infinite-mass case, and his force-reduction factor `X1` (printed page 5-50)
      falls as low as 0.02 for a canopy large for its load, so the real peak can be far below
      hpr's. Near the canopy's terminal speed, as at apogee, a filling time gives hpr a higher
      peak than an instant opening. `docs/physics/recovery.md` states the corrected limits.
  - A deployment at zero airspeed has no filling time in `n D₀/v`, so the canopy opens at once.
- **Triggers: apogee, a height above the site while descending, a time after ignition, and a
  motor's ejection delay after its burnout.** Both the apogee and the height trigger are numeric as
  well as event-driven — "descending", and "descending at or below the height" — which is
  RocketPy's own form (`y[5] < 0`) and an altimeter's behaviour, and which means a flight that
  starts past its apogee still deploys (found in review: an event-only apogee trigger fell
  ballistically to the ground with no deployment and no error). A motor with no delay in seconds
  (plugged, or unset) is refused rather than assumed.
  There is no sampling rate and no barometric noise: hpr locates the crossing with its event
  finder. Monte Carlo can perturb the setting instead (M1.10).
- **A device can be released by another's opening** (`released_by`), which cuts a drogue away under
  a main (L27). The release waits until the releasing device is **fully open**: releasing at its
  line stretch collapsed the drag area to almost nothing while the main filled, and the descent
  sped up (found in review, now a test). A device released before its own charge fires never
  deploys. Open devices otherwise **add** their drag areas, where RocketPy keeps one `cd_s` and
  replaces it, which is why the comparison gives its drogue `released_by` the main.
- **The descent is a point mass in a new `Phase::Descent`**, entered at the first deployment:
  `m a_cg = −½ ρ (C_D S) |v_cg − w| (v_cg − w) + m (g + a_Coriolis) + T`.
  - The attitude freezes and the body rates are set to zero. A tethered rocket's attitude under a
    canopy is not modelled by anything hpr can cite.
  - **The airframe's own drag is left out**, as RocketPy leaves it out, rather than carrying Loft's
    unsourced `0.5 A_ref` term (L27). For a drogue whose drag area is near the body's broadside
    area this is a real omission, and it is where M1.7b's tumble drag belongs.
  - The thrust is kept along the frozen axis, so an off-nominal deployment under thrust is not
    silently thrust-free.
  - **Added mass is not modelled.** Knacke gives no closed form (printed page 5-40 is qualitative),
    and RocketPy's `m_a = k_a ρ (2/3) π R² H` carries no citation in its code and no weight in its
    equations, so it changes no equilibrium rate, only the transient. The comparison shows the
    cost: NDRT 2020, whose main's added mass is 15.9 kg against a 20.8 kg rocket, is hpr's worst
    case at +0.71% in descent time and +2.86% in the smaller drift component.
- **Every deployment, every end of filling (which is also a release) and every known trigger time
  is a stop time**, so no step straddles a change in the drag area, and events are located as in
  M1.6a. The numeric triggers are checked once per interval, on one evaluation shared by every
  pending device, which `Stats` does not count. The event list is
  now built per interval as a `Watch` list instead of numbered by hand, because the height triggers
  come and go.
- **The comparison with RocketPy starts where both models agree.** Both simulators start from the
  same declared post-burnout state near apogee with the first device's lag overridden to zero, so
  almost no segment under either model's aerodynamics separates them (RocketPy's trigger sampling
  leaves 2.5 to 13 ms of its own 6-DOF flight, `recovery.md`), with RocketPy's noise zeroed (it
  draws on the global `np.random`) and a declared wind (the examples' own winds need the network or
  Copernicus files). Five example rockets. The oracle runs at `rtol = atol = 1e-8` and records its
  own solver's contribution per metric: at most 3.5e-6 on every compared metric (its one larger
  entry, 2.1e-3, is on a 20 µm drift component this comparison did not compare; M2.1a measures it,
  and reading 28x high there is what found the gravity-model difference of ADR-015, issue #27). The
  differences that remain
  are listed in `recovery.md`, with the trigger-sampling one measured rather than assumed away. Whole-flight comparisons, ascent included, are M2.1's.

**Consequences.**

- M1.7b adds streamers, tumble and separation. It needs a streamer drag source (Knacke has none),
  a tumble model, and a decision about how a separated body's mass and drag are defined.
- A cited apparent-mass model would close the gap against RocketPy's transient; it is the only
  known model difference left in the descent.
- `Sample` gained `recovery_drag_area_m2` and `Channel` gained `RecoveryDragArea`, so reports can
  show the opening.
- M1.10's Monte Carlo can vary deployment heights, lags and drag areas; nothing here samples.
- M3.1 maps `.ork` recovery devices onto these types.

## ADR-013: Streamer and tumble drag (2026-09-17)

**Context.** M1.7b needs a cited drag model for a streamer and for a body tumbling with nothing
deployed. Knacke's manual, which M1.7a leaned on, has **no streamer data at all** (checked: the
type tables 5-1 to 5-5, the measured-drag section 5.2.3, the miscellaneous-decelerator section
5.8.4 and the contents) and no bluff-body crossflow table. What is in hand
(`docs/research/streamer-and-tumble-drag.md`):

- The **OpenRocket technical documentation v13.05** (CC BY-SA, a published document, pinned):
  Appendix C fits a streamer drag coefficient to its own wind-tunnel tests, and §3.5 fits a
  tumbling model to 22 m drop tests. Its Java source is GPL and is never read.
- **Carruthers and Filippone (2005)**, peer-reviewed wind-tunnel measurements of streamers and
  flags, whose AIAA copy is paywalled and whose author post-print states no licence.
- **Kidwell's NARAM-43 drop tests (2001)**, OpenRocket's own reference for its appendix, with per
  streamer masses and measured descent rates. No licence stated.
- Loft's L29 lesson: its recovery defaults came from OpenRocket's **source**, which must not be
  ported.

**Decision.**

- **A streamer is a drag area from a correlation on its planform area `S = l w` and aspect ratio
  `AR = l/w`, and hpr carries both correlations** (`StreamerModel`), because they disagree by a
  factor that runs from 1.9 (at 80 g/m²) to 5.8 (at 10 g/m²) and only one of them survives a
  comparison with a free drop.
  - **The default is Carruthers and Filippone's**, all three of its printed curves:
    `0.561 AR^−0.480` at `S = 0.025 m²` (eq. 2), `0.6514 AR^−0.6075` at 0.05 m² (the trend line on
    Figure 3, which the text does not repeat) and `0.405 AR^−0.494` at 0.075 m² (eq. 1). hpr
    interpolates between neighbours linearly in `ln S` and holds the end curve outside; that is
    hpr's choice, documented as such. Blending only the two extremes, as the first draft did,
    reads 18% below the paper's own middle curve at `AR = 3.3` (found in review).
  - **`StreamerModel::OpenRocket`** is appendix C's `C_Dm = 0.034 ((ρ_m + 25)/105)((l + 1)/l)`,
    kept for comparing with OpenRocket in M2.2. It is the only one of the two that uses the
    material.
  - **The measurement that decides it.** Recomputed here from Kidwell's Table 1 and his results,
    with his normalisation to a notional 5 g weight and his distance-over-time rates compared
    against the same average from the closed-form fall (both corrected in review): his crêpe
    streamer, the one he left unpleated, descends at 2.80 m/s, a `C_D` of 0.155 on its planform.
    Filippone's correlation gives 3.05 m/s (+9%); appendix C gives 5.28 m/s (+88%). Kidwell's
    pleated streamers descend slower still (Micafilm at 2.04 m/s, `C_D` 0.338), which neither
    model reaches.
  - **Above the largest fitted area hpr holds the end curve rather than extrapolating.** The
    paper's trend would give a lower `C_D` (about `S^−0.3`), but the only free-drop measurement in
    hand is higher than either, so the clamp is the closer of the two. `recovery.md` states both.
  - **Pleats are not modelled**, so hpr predicts a faster descent for a folded streamer. That is
    the safe direction for a landing, and it is stated in `docs/physics/recovery.md`.
  - hpr's streamer descent rates will therefore differ from OpenRocket's by 1.4 to 2.4 times,
    depending on the fabric (the drag-area ratio runs 1.9 to 5.8). M2.2 will see that; it is the
    intended difference, not a defect.
- **Tumble is the technical documentation's §3.5 model**, `C_D S = 1.42 A_f + 0.56 A_bt`, computed
  from the airframe by `DeviceDrag::tumbling`: `A_bt` by integrating the outer diameter along the
  axis (each body component's mean diameter times its length), `A_f` as one fin's planform area
  times Table 3.4's efficiency factor for the fin count. More than eight fins is refused, because
  the table stops there.
  - Its constants come from Hoerner's *Fluid-Dynamic Drag*, which is copyrighted with no legal
    free copy. hpr cites the documentation, and the research note records NASA TN D-540 and
    TR R-474, which are free and carry the same numbers, for when a pinned source is needed.
  - **hpr states what it can demonstrate, not the documentation's accuracy claim.** Replaying its
    own Table 3.3 through hpr's reading of the model gives −5.8%, −5.4%, −7.2%, +19.0% and −10.0%
    on the five drop-test models, not the 3 to 14% it claims: the finless tube wants a body
    coefficient near 0.79 where the model prints 0.56, and the text pins neither area convention.
    The spread is a test (`the_tumble_model_against_its_own_drop_tests`) and is what the docs
    quote (found in review: the claim had been repeated without checking it).
  - **The fit is for small models** (44 to 103 mm, 6.8 to 160 g, 5.0 to 6.6 m/s). A high-power
    booster is outside it, and above `Re ≈ 2e5` a cylinder's crossflow drag falls by about half,
    so hpr will read slow there. The limit is documented rather than extrapolated.
- **A tumbling body is a device with a trigger, like a canopy.** hpr does not decide by itself
  when a rocket tumbles: no source in hand says when a stage becomes unstable enough, and the same
  documentation declines to model the analogous twirling streamer regime.
- **Both new sources are pinned and cite-only** (`validation/refs.lock.toml`,
  `THIRD-PARTY-NOTICES.md`): the Filippone post-print and Kidwell's report state no licence, so
  their numbers are used and their text is never copied or redistributed.
- **M1.7 is split again.** M1.7b is streamers and tumble, with the first of M1.7's remaining
  *done when* bullets; M1.7c is separated bodies, with the second. Separation needs its own
  decision about how a body's mass properties and drag are defined, and `Assembly` has no split.

**Consequences.**

- M1.7c flies separated bodies. Since the descent phase already drops airframe aerodynamics, a
  separated body needs mass properties and its own device, not an aerodynamic model — which is
  the cheapest honest way in.
- M2.2's OpenRocket comparison should compare streamers under both models, and report the gap
  rather than tune either.
- If a streamer model is ever fitted to more drop data, `StreamerModel` is the place for it; the
  research note lists what would be needed.

## ADR-014: Separation: bodies, their masses and their descents (2026-09-17)

**Context.** M1.7c has to fly every body of a separated rocket to its own landing, with its own
mass properties and drag. What is in hand:

- The descent phase (ADR-012) is a point mass: it needs a mass and a drag area, and deliberately
  has no airframe aerodynamics.
- `hpr-design` carries `MassProperties` per stage (`Layout::stages`) and a stage index on every
  placed motor, so a body's mass properties are a sum over its stages.
- `hpr-aero` needs a nose-first layout, so a partial airframe has **no** aerodynamic model. A body
  that had to fly aerodynamically would need M1.8's work and a way to build a model for a
  headless stack.
- M1.9 will add staging, where a sustainer lights after separation and keeps flying under thrust.

**Decision.**

- **A separation is a trigger plus a stage boundary** (`Separation { trigger, after_stage }`), with
  the same triggers a device has. Stages `0..=after_stage` keep the nose (body 0), the rest form
  body 1. Two bodies for M1.7c; more splits are additive and nothing in the types forbids them
  later.
- **The ascent ends at the separation**: `Termination::Separated`, a `Separation` event, and one
  `BodyFlight` per body in `FlightResult::bodies`. Nothing continues in six degrees of freedom,
  because no body has an aerodynamic model.
- **A body is its own stages and their motors.** Its mass properties are the sum, so the bodies'
  masses add to the rocket's at that instant (a test). A body's mass is then held **constant**
  through its descent, which is why a separation must follow the last burnout. That is **enforced**
  when the trigger fires, not merely documented: whether it fires before the burnout depends on the
  flight, so it is a flight-time `SimError::Domain` rather than a setup check. A release across the
  separation is refused when the devices are given, because a line cuts a device on its own body.
- **The separation adds no impulse.** Each body starts at its own centre of mass with the velocity
  that point already had (`v_O + ω × r_cg`), so the bodies' **linear** momenta add to the stack's
  (a test). Angular momentum is not conserved: the bodies drop their rotation, which discards each
  body's spin about its own centre (31% of the total at the test's 0.6 rad/s). The identity is
  exact only after burnout, because `v_cg` also carries the centre of mass's motion inside the
  body — which is the other reason the burnout rule is enforced. An ejection charge's impulse, the
  tip-off it gives each body and the tumbling that follows are not modelled; hpr says so rather
  than inventing a spring constant.
- **Every body must carry a recovery device, and that device must open.** A flight whose bodies
  are not all covered is refused when it is set up; a body that reaches the ground with no
  deployment at all is refused in flight. With no airframe drag in the descent, either case would
  be a fall in a vacuum, which is a wrong number rather than a missing feature. The corollary is
  recorded in `recovery.md`: a body coasts drag-free between the separation and its first
  deployment, which reads high in arrival speed. A spent booster's device is
  normally `DeviceDrag::tumbling_stages` over its own stages, which is §3.5's model applied to
  that body rather than to the whole stack (the whole-stack form is still there for a stack that
  tumbles without separating).
- **The bodies fly as 6-state point masses** through the same integrator, with the descent's
  equations less the thrust, and their own stop times and events: their devices' trigger times,
  the deployments, ends of filling and releases their devices already have, their own apogee, and
  their own deployment heights. Review found the first draft missing the apogee and the carried
  times, which made a body separated while climbing fall ballistically and a deployment scheduled
  before the separation vanish. They share the flight's devices and their progress, so a canopy
  that opened before the separation stays open on the body that carries it.
- **Only body 0's devices act before the separation.** A device for another body has a drag area
  computed for that body, which is not a model of the whole stack.
- **The separation's own trigger is a stop time, and its height is a located event**, as a
  device's is; review found it polled at whatever boundary happened to come next, which fired a
  timed separation 186 s late and an altitude one never.
- **A body's descent is not observed.** `Observer` sees a 13-element rigid-body step; a body's
  state is six. Its events and samples are in its `BodyFlight` (`BodySample`, `BodyEvent`), which
  is what a report needs. Wiring the observer to bodies can come with M2.1's reports.

**Consequences.**

- M1.9's staging extends this: a body that keeps flying needs an aerodynamic model for a headless
  stack, which is the real work, and the ignition times and mass variation that go with it.
- M2.1's landing metrics take the bodies, not just the final sample.
- A separation with three or more bodies needs `Separation::BODIES` generalised to a list of
  boundaries; nothing else changes.
- Because a body is a point mass, its attitude is not tracked at all after separation, so nothing
  can report how a booster is oriented as it tumbles.

## ADR-015: The validation harness: cases, references, tolerances and reports (2026-09-17)

**Context.** M2.1 is the first end-to-end milestone: hpr's numbers against other tools' for whole
flights. M2.1a builds the harness the suite runs on. What shapes it is less the plumbing than
Loft's five validation lessons (`docs/research/loft-lessons.md`): a comparison that wasn't
like-for-like (L75), a reference regenerated whenever it disagreed (L76), hand-written "stored
results" (L77), suites that skipped themselves and reported green (L78), and 10 of 12 metrics with
no gate at all (L79).

Already in hand: five oracle fixtures under `validation/fixtures/` with their generator scripts,
and M1.7a's recovery comparison, whose references are real RocketPy output.

**Decision.**

- **A case is a TOML file, a reference is JSON a generator wrote, and the harness only ever reads
  the reference.** `validation/cases/<id>.toml` says what to fly and which metrics to compare
  against which reference file and case; `validation/fixtures/**` holds what the oracle said. A run
  writes nothing but the report, so hpr cannot move its own goal posts (L76) — there is no
  `--update-references` flag, and there will not be one: a reference moves when its generator runs.
- **Every reference value carries a source** naming the oracle, the generator and the field it came
  from, and a value with a blank source is refused (L77). The harness builds those strings from the
  generator's own provenance block rather than trusting a hand-written label.
- **Every metric a case reports has a tolerance that bounds something, or a written reason why it
  is not scored** (L79). A tolerance is a fraction, an absolute difference, or both; one that
  bounds nothing — including an infinite or negative bound — accepts nothing rather than passing
  quietly, and a case that measures or publishes anything it does not account for is refused, in
  both directions: hpr may not measure a metric the case does not gate, and the reference may not
  publish one the case ignores.
- **A metric that cannot honestly be scored is declared, not smoothed over.** The descent cases
  carry no absolute floors: an absolute bound wide enough to carry Valetudo's near-zero northward
  drift would also have been 8.7x looser than 3% of that case's whole drift, which is a weakened
  check wearing a tolerance's clothes. The alternative is `not_scored = "<reason>"`: the harness
  measures and prints the metric with both numbers and the reason, and counts it apart from the
  verdict. Loft excused its two largest misses as "no single target" (L82), so a blank reason fails
  outright and the whole excused set is pinned by a test named after what it is. **No metric uses
  it today.** It was written for Valetudo's northward drift, where hpr read 28x RocketPy; the cause
  turned out to be the gravity model below, and the metric is now gated at 3% like every other. The
  mechanism stays for M2.1b, whose predicted-mode supersonic cases are gaps by construction.
- **A RocketPy comparison flies RocketPy's gravity model.** hpr's default is the full
  normal-gravity vector, which above the ellipsoid leans a few parts in 10⁶ toward the equator;
  RocketPy
  applies gravity to the vertical axis alone. The difference is invisible in every metric that
  matters and decisive in the one that does not: 5.2e-4 m of northward drift over an 800 m
  descent, against a 2.0e-5 m Coriolis signal. hpr already ships `GravityModel::VerticalTaylor`
  as RocketPy's formula "for like-for-like comparisons", so the harness uses it, and Valetudo's
  northward drift comes to −1.8% instead of +2704%. The general rule this is an instance of: when
  the oracle's model is a documented simplification of hpr's and hpr can be asked for the same
  simplification, the comparison uses it and says so, rather than reporting the modelling gap as
  a physics gap (L75).
- **The cases that must run are locked** in `validation/cases/lock.toml`, and a locked case that is
  not there is an error, not a skip (L78). A committed case that is not locked is an error too, so
  a case cannot be added and forgotten — and both checks live in the command, not only in a test.
  `--fast` may only leave out cases the lock marks slow, names them in the report, and writes
  `latest-fast.{md,json}` rather than the committed record.
- **The oracle's inputs come from the reference's own record of what it flew** (L75): the descent
  cases take the site, the wind, the devices and the state at the first deployment from the
  fixture, so a case cannot compare hpr against hpr. That extends to the vehicle: the reference
  records which design it flew and what it weighed, and the harness refuses a case that names
  another design or whose mass differs by more than 1e-9, so a copied case file cannot report the
  difference between two rockets as a difference in the physics. The one place that knows a
  generator's JSON shape is `hpr_validate::rocketpy`, and a file that does not name the oracle,
  the generator and the command that produced it is not a reference at all.
- **The report is committed**, in Markdown for people and JSON for machines, and carries no
  timestamp, so a run that changes nothing changes no bytes and a number that moves shows up in
  the diff. It carries each reference's SHA-256 and the generator's own description of what the
  oracle modelled and what it had to override, so a hand-edited reference or an unlike comparison
  shows up in the report rather than only in git history. A test asserts the committed Markdown is
  byte-for-byte what the harness produces. The JSON is pinned in two parts, because it carries
  full-precision floats and hpr's determinism promise is bit-identical results **on one platform**,
  not across three: everything that cannot differ by platform (cases, metric names, sources,
  tolerances, verdicts, reasons, hashes) is compared exactly, and the numbers through the Markdown
  the committed JSON renders, to the six decimals that report prints. That is the resolution at
  which the descents reproduce on macOS, Windows and Linux, measured, not assumed — CI failed the
  first time this was asserted at full precision. If a platform ever diverges at six decimals, the
  answer is to find out why, not to loosen the comparison.
- **M2.1a's first cases are M1.7a's descents.** They are the only references in hand that cover a
  whole hpr flight path end to end, and reusing them means the harness ships with five real cases
  rather than a demonstration. M2.1b adds the ascent cases in both modes, the CI job and the
  regeneration workflow, which is where `Flight` grows a variant.

**Consequences.**

- M2.1b adds a `Flight::WholeFlight` variant, the same-drag and predicted modes, `hpr-validate`'s
  own binary if one is wanted, and the CI job. The metric names M2.1 lists (apogee, time to
  apogee, maximum velocity and Mach, rail-exit velocity, burnout state, time-series RMS) arrive
  with it.
- M2.4's census reads `validation/reports/latest.json`.
- `hpr-validate` reads files, so it is not part of the pure core and `cargo xtask wasm-check`
  leaves it out, as `ARCHITECTURE.md` already says.

## ADR-016: The documentation site: mdBook over `docs/`, and checks for links, labels and equations (2026-09-18)

**Context.** M0.4 asks for one searchable site that a hobby rocketeer can read (VISION V15, and
CLAUDE.md's "Documentation is a deliverable"). Its first increment, M0.4a, asks for three things:
an ADR that picks the tool and the layout; one source per page, with equations that render both
on the site and on GitHub; and a CI build that fails on a broken link or a bare internal label.
What was in hand on 2026-09-18:

- **The pages.** There are 18 pages under `docs/physics/` and `docs/format/`. They write
  equations in Unicode inside ```` ```text ```` blocks and inline code (`γ_e = GM/(ab)`), with no
  LaTeX anywhere, and they had no Markdown links at all. Code comments, docs and oracles cite
  their paths 135 times. They carried 146 bare labels: `L12`, `ADR-008`, `M1.5b` and the like.
- **The tools.** Versions and licences were read from crates.io on 2026-09-18:
  - mdBook 0.5.4 (MPL-2.0).
  - mdbook-linkcheck 0.7.7 (MIT), which is built against mdBook 0.4 and pulldown-cmark 0.8 and
    does not load in 0.5.
  - lychee 0.24.2 (MIT OR Apache-2.0).
  - pulldown-cmark 0.13.4 (MIT), the Markdown parser mdBook 0.5.4 itself uses.

**Decision.**

- **mdBook 0.5.4 builds the site.** It is a single binary with built-in search. It takes its
  sidebar from a plain `SUMMARY.md`, and its pages are ordinary Markdown with no front matter, so
  GitHub renders the same file. The alternatives each break one of those:
  - Zola needs TOML front matter on every page, which GitHub would show as text.
  - MkDocs needs a Python toolchain and a YAML nav.
  - Docusaurus needs a Node toolchain.

  mdBook is MPL-2.0, and we only run it: it is never linked or ported, and `cargo deny` never
  sees it. Its theme files (MPL-2.0) and the libraries they bundle are copied into the built site
  unchanged, with their licence headers and texts. `THIRD-PARTY-NOTICES.md` lists them. CI pins
  0.5.4, and `cargo xtask site` refuses any other release series. A local build with another 0.5
  release (Homebrew installs the latest) may differ slightly from CI's; `cargo install mdbook
  --version 0.5.4 --locked` matches it exactly.
- **The site's source is `docs/` itself.**
  - `book.toml` sits at the root with `src = "docs"`, and the site builds into the gitignored
    `target/site`.
  - `docs/SUMMARY.md` lists the pages. *Start here* is `docs/start-here.md`, not `README.md`,
    because mdBook turns a link to `README.md` into `README.html`, a file it never writes.
  - `docs/physics/` and `docs/format/` are therefore in the site's source without moving. Each
    page has one source, and the 135 citations of their paths keep working.
  - The working files are not pages: `STATUS.md`, `ROADMAP.md`, `DECISIONS.md` and the research
    notes. The site links to them on GitHub. Whether the decisions and the roadmap become pages
    is M0.4b's call.
- **Equations stay in Unicode**, in ```` ```text ```` blocks and inline code.
  - They render the same on GitHub, on the site and in rustdoc, which renders no LaTeX either.
    Every existing page already writes them this way.
  - mdBook's MathJax doesn't accept the `$` delimiters GitHub uses. `mdbook-katex` would add a
    preprocessor to build and pin, for pages that don't need one.
  - `$x$`, `` $`x`$ ``, `$$` and ```` ```math ```` fail the check, because only GitHub would
    render them. Two prices in a sentence ("$5 and $10") are not math.
    Revisit this if a page needs typeset math.
- **One link rule serves both renderers.** A relative link goes only to another page of the site,
  or to a file under `docs/`, which mdBook copies. Anything else in the repository is linked by
  its `https://github.com/nrdptel/hpr-sim/blob/main/...` URL. A relative link to anything else
  works on GitHub but breaks on the site.
- **The checks are our own**, in `xtask/src/site.rs`, and read pages with `pulldown-cmark`, so
  they see the same links the site renders.
  - **On the sources**, which `cargo test` checks too:
    - every summary entry exists, and every page under `docs/physics/` and `docs/format/` is
      listed;
    - relative links and their `#fragments` resolve, with anchors derived as GitHub derives them;
    - a GitHub URL to `main` names a file in the working tree, and for Markdown, a heading in it;
    - an undefined `[text][ref]` fails, as do bare labels, math only GitHub renders, and a link
      inside a heading.
  - **On the built HTML**, which `cargo xtask site` checks after `mdbook build`: every relative
    `href` and `src` must reach a file, and every fragment an `id`. This catches what mdBook
    rewrites, and any id it derives differently from GitHub. A warning from mdBook fails the build
    too.
  - **Why not lychee.** The link rule needs a check of our own anyway: lychee accepts a relative
    link to `../ROADMAP.md`, because the file exists on disk. Labels need a Markdown parser
    anyway. A Rust check runs in `cargo test` with unit tests that show each failure, offline,
    with nothing to install but mdBook. lychee remains the candidate for fetching external links.
- **External links are counted, not fetched.** A PR's checks must not depend on the network
  (CLAUDE.md rule 5, and flaky CI). Today the site has eight: RocketPy's and OpenRocket's sites,
  the three format specs, and three links to this repository's issues. Links to files in this
  repository on GitHub (`blob/`, `tree/` or `raw/main/`) are checked offline against the working
  tree. A scheduled check of external links is tracked in
  [issue #38](https://github.com/nrdptel/hpr-sim/issues/38).
- **Bare labels fail.** `L` or `ADR-` followed by digits, and milestone ids (`M1.5b`, `M2.1b1`),
  fail as whole words outside a link's text: in prose, tables, headings, inline code, raw HTML
  and image alt text. Fenced code blocks are exempt, since they quote files verbatim. A label that
  emphasis splits in two (`**ADR**-008`) is not caught.
  - The fix is a link with a few words of meaning: `[Loft lesson L10][lessons]`,
    `[ADR-009][adr-009] (drag)`, or `[M1.8][roadmap], the supersonic aerodynamics milestone`.
    Reference definitions at the end of the page point into `DECISIONS.md` (by heading anchor),
    `ROADMAP.md` and `research/loft-lessons.md` on GitHub.
  - Some ordinary words look like labels, and are written so they don't: a motor designation in
    full (`L1150R`, not `L1150`, which the lesson pattern would catch); a certification level as
    "Level 2", not "L2"; an appendix equation as "eq. A1.3" is fine, since only `M` starts a
    milestone id.
  - Labels stay out of headings: mdBook wraps each heading in a link, so a link inside one is
    invalid HTML, and the check fails it.
- **CI.** A `site` job on Linux installs mdBook 0.5.4 through `taiki-e/install-action` and runs
  `cargo xtask site` on every PR.

**Consequences.**

- A PR that moves or renames a file a page links to fails until the page follows.
- Milestone and lesson links open the whole roadmap or lessons page, because their entries have no
  heading anchors. The link text names the label, so the browser's search finds it.
- Anchors are GitHub's slugs. mdBook's ids agree with them for every heading on the site today, and
  the built check confirms that on every build. A heading whose two anchors differ fails that
  check, and is reworded.
- The local gate gains `cargo xtask site`, which needs mdBook installed (`brew install mdbook`, or
  `cargo install mdbook --version 0.5.4 --locked`).
- The site is not published until M0.4d, which waits for Neer to turn on GitHub Pages.

---

## ADR-017: Model pages open with *In short*; Accuracy traces its numbers; the records stay files (2026-09-18)

**Context.** M0.4b asks that every model page open with *In short* (what it models, its source,
how well it is validated, what it leaves out), that a page without it fail CI, that an *Accuracy*
page give every validation result from the committed report, and that the decisions and the
roadmap be reachable from the site. ADR-016 left open whether the last two become pages. On
2026-09-18 there were 16 model pages under `docs/physics/`, one validation report with 5 cases and
30 metrics, 17 decision records and a roadmap of 806 lines.

**Decision.**

- **The form of *In short* is fixed, so a check can hold every page to it.** Right under the
  page's title, a `## In short` section holds only a bulleted list of four items, which open with
  the bold labels `What it models:`, `Sources:`, `How well it is validated:` and
  `What it leaves out:`, in that order, each followed by its answer. The labels repeat the
  milestone's own words. What pages used to open with (code paths, decisions, the source list)
  moves under a heading of its own, usually `## Code and sources`. A model page is any page under
  `docs/physics/`; the format pages under `docs/format/` describe files, not models, and are
  exempt. The check (`in_short` in `xtask/src/site.rs`) reports the first departure and its line.
- **Its answers follow the page, and say how far the evidence goes.** *How well it is validated*
  names which of the four kinds of evidence in `VALIDATION.md` the model has (analytic, published
  source, another code, real flights), with the page's own numbers, and says plainly when a model
  has not been compared with another code or a real flight. The check can't judge that; the
  physics review does.
- ***Accuracy* traces every number it quotes.** Each number in a paragraph, list item or table row
  (outside code) must appear in a repository file that the same paragraph, item or row links to:
  a model page (less its own *In short*), the report, a case file. The pages at the top of the
  site don't count, so a page can't vouch for itself. A number is anything with a decimal point,
  an exponent (`1e-6`, `10⁻¹²`), a percent sign or two digits; "Level 2", "6-DOF" and "3 fins"
  are words. It matches only a number written the same way: the same digits as a whole number,
  and the same sign and percent sign where the page writes them. So a reader checks it in one
  click, and a number that moves at its source fails the site check until the page follows. The
  check can't tell whether a number is quoted in the right context; reviews do that.
- **The report's results are checked cell by cell.** A table on *Accuracy* whose header opens
  with `case`, and names a report metric in code in each other column, must give each case's
  difference exactly as the report writes it, and all the report's results must be in such a
  table. So the page gives every validation result, and each one right. Numbers are checked, not
  generated: a generated page would read worse than one written for people, and the check gives
  the same guarantee. The page must also link every model page.
- ***In short* traces its numbers too.** Each number in a model page's *In short* must appear in
  the rest of that page, or in a file its item links to, so the summary can't claim what the page
  doesn't show.
- **The decisions and the roadmap stay files, with a page that indexes them.** Rendered as pages,
  `DECISIONS.md` and `ROADMAP.md` would carry hundreds of labels at the places that define them,
  which the label check would have to learn to accept, and they are working files that change
  with every milestone. Instead, *Decisions and the roadmap* explains the three kinds of label and
  links every decision record and every phase of the roadmap on GitHub, each with a line of
  meaning. The check fails if a record or a phase is missing from it.

**Consequences.**

- A new model page fails CI until it opens with *In short*, and until *Accuracy* links it.
- A regenerated report that moves a quoted number, or adds a case or a metric, fails CI until
  *Accuracy* follows. So does a model page whose quoted number changes.
- A new decision record fails CI until the records page links it.
- The check can't tell a true *In short* from a false one. Reviews do that, against the page's
  body, `VALIDATION.md` and the report.

## ADR-018: Examples run in CI against committed output; pages quote files, checked line for line (2026-09-18)

**Context.** M0.4c asks for a *Getting started* page whose example program runs in CI. CLAUDE.md
asks that code in the docs compile and run in CI, and that the numbers a page quotes can't go
stale. mdBook's `{{#include}}` would put a file into a page, but only on the site: GitHub shows the
directive as text, and every page must read the same on both (ADR-016). And a page that shows what
a program prints quotes numbers that move whenever a model does.

**Decision.**

- **The first flight is Valetudo, recovered.** `crates/hpr-sim/examples/first_flight.rs` flies
  RocketPy's Valetudo design (K400C curve) from a 3 m rail in a 5 m/s wind, with a drogue at
  apogee and a main at 150 m. The design is committed, the flight tests and benchmarks already fly
  it, and it stays subsonic (Mach 0.36). The synthetic 54 mm design on its I175 reaches Mach 1,
  which hpr refuses until M1.8. The design is built in with `include_str!`, so the program runs
  from any directory.
- **What an example prints is committed beside it,** as `<name>.output.txt`. `cargo xtask
  examples` runs every example target that `cargo metadata` lists and writes those files;
  `--check` writes nothing, and fails if an example fails, has no file, or prints anything else.
  CI's test job runs `--check` on macOS, Windows and Linux, which shows the output is the same on
  each. Examples print rounded values (0.1 m, 0.01 s), where the platforms agree; they agree to six
  decimals on descents (M2.1a).
- **A page quotes a file under a marker.** A `<!-- quote: <path> -->` comment right above a fenced
  code block makes the block a quote of that repository file, and `cargo xtask site` fails if the
  two differ by a line (a `\r\n` reads as `\n`). The comment shows on neither GitHub nor the site.
  So the page, the file and what the program prints on each OS are one text.
- **`Environment::with_wind`** sets a wind without `Arc` and struct-update syntax, which a first
  program shouldn't need.

**Consequences.**

- A model change that moves a printed digit fails CI until `cargo xtask examples` rewrites the
  output and the page's quote is copied again: two steps, on purpose, so the page can't go stale.
- An example that prints more digits than the platforms agree on fails CI; it must print fewer.
- The test job builds and runs every example, which adds seconds to it.
- A block without a marker is not checked, and code in prose (such as the page's suggested edits)
  is not compiled. Reviews cover those.

## ADR-019: Publishing the site and the API reference to GitHub Pages (2026-09-18)

**Context.** M0.4d asks for the site to be published from `main`, with the workspace's rustdoc
beside it and each linking the other. GitHub Pages hosts a public repository's site for free, but
turning it on is a repository setting, which only the owner may change (the project's rule 9);
until then, a deploy fails. Rustdoc merges its search index and list of crates with whatever an
earlier run left in its output. With `--no-deps`, cargo documents a workspace's crates in no set
order, and rustdoc links another crate's item only if that crate's pages already exist; otherwise
it leaves the link as text, without a warning. And the crates' documentation is also read outside
the site, in `cargo doc --open` and later perhaps on docs.rs, so it can only link the guide by its
address.

**Decision.**

- **The API reference is part of the site, under `api/`.** `cargo xtask site` builds the rustdoc
  of every library crate (not `xtask`, not the command-line binary), with all features and
  `-D warnings`, and copies it to `target/site/api`. The site is one artifact, and a local build
  has both halves.
- **Built on its own, in order.** The rustdoc build has its own target directory,
  `target/site-rustdoc`, whose documentation is cleared before each build (not its compiled
  dependencies, so a rebuild takes seconds). Crates are documented one at a time, each after the
  workspace crates it depends on, so every link between crates resolves. `api/index.html` sends a
  reader to the guide's page, *The API reference*.
- **They link each other, and the check holds both ends.** *The API reference* links each crate's
  front page by a relative link. Each crate's `//!` documentation links the guide's pages for its
  models by address, `https://nrdptel.github.io/hpr-sim/...`, which the built-site check reads as
  the local build. So a crate the site doesn't link fails, as does a crate that links no page of
  the guide, or a link to a page the guide doesn't have.
- **Every link in the reference is checked**, as the guide's are, and a link to another crate that
  rustdoc left as text fails. Three kinds of link are exempt, each named in the code: the
  implementor scripts rustdoc loads only when they exist, line ranges on its source pages, and
  links inside documentation it copies from a dependency (glam's, for the vector types), which it
  passes on as written when it can't place them.
- **The site knows its path.** Pages serves a project's site under `/hpr-sim/`. `book.toml` sets
  `site-url` to it, so mdBook's 404 page works at any address, and the check resolves
  root-absolute links, links by the site's address and a `<base href>` against it, as a browser
  would.
- **CI deploys from `main`, after every other check.** The `site` job uploads `target/site` as
  the Pages artifact on every run (`actions/upload-pages-artifact@v5`). A `deploy` job publishes
  it (`actions/deploy-pages@v5`) on `main` only, once fmt, clippy, the three test jobs, doc,
  wasm-check, deny and site have passed on that commit. It alone may write to Pages.
- **Until Pages is on, the deploy is skipped, and says so.** On `main`, the `site` job asks
  GitHub's API whether Pages is on and deploys from GitHub Actions. If not (a 404, or another
  source), it prints a warning and the deploy job is skipped, so `main` stays green. Any other
  answer from the API fails the job, so a broken check can't pass for "off". Pull requests don't
  ask, since they never deploy.
- **The README's first lines link the address,** and say the site goes live once Pages is on.

**Alternatives.**

- A workflow of its own for Pages: it would publish a commit whose other checks failed.
- Letting the deploy fail until Pages is on: `main` would be red for a reason outside the code,
  which hides real failures.
- `actions/configure-pages` with `enablement: true`, which turns Pages on from CI: a change to a
  repository setting, which is the owner's call.
- docs.rs for the rustdoc: it needs the crates published on crates.io, also the owner's call.
- Linking the guide relatively from the crates (`../../physics/aero.html`): it works inside the
  site only, not in `cargo doc --open` or on docs.rs.
- One `cargo doc` for all the crates: faster by a few seconds, but its cross-crate links depend on
  which crate cargo happens to document first.
- Not inlining glam's types (`#[doc(no_inline)]`): no copied links to exempt, but `DVec3` in every
  signature would lead nowhere, since glam's own pages aren't built.

**Consequences.**

- The deploy bullet of M0.4d waits for the owner to turn Pages on (`STATUS.md`, Needs Neer).
  Then the next push to `main` deploys, or `gh workflow run CI --ref main` without one, and the
  README's "goes live once" sentence goes.
- `cargo xtask site` also runs `cargo doc` once per library crate, about 5 s with a warm build and
  10 s from cold.
- A library crate added to the workspace fails the site until *The API reference* links it and its
  documentation links the guide.

## ADR-020: The reader test, and labels that lead to plain words (2026-09-18)

**Context.** M0.4e asks for a reviewer with no project context to answer ten new-user questions
from the site alone, and for every term it flags as unclear to be fixed. The first cold pass
answered five of the ten fully and five in part. It had four blocking findings: no way to describe
your own rocket, no path from a motor file to a flight, pages that disagreed about clusters, and an
ambiguous height for wind tables. It flagged about forty terms. One finding ran across every page:
a milestone or Loft lesson label linked to the top of a long file (`ROADMAP.md`, the lessons
table), which says nothing about the label, and about ninety labels in the API reference were bare
([issue #44](https://github.com/nrdptel/hpr-sim/issues/44)). The roadmap has no heading per
milestone to link to, and a Markdown anchor comes only from a heading, which may not hold a label
(ADR-016).

**Decision.**

- **The test reads the built site, cold.** A reviewer with no project context gets a copy of
  `target/site`, the API reference included, and nothing else from the repository, except a file that a page links on
  GitHub, as a reader would click through. The ten questions are listed in the PR. Each answer
  cites its pages; every flagged term is fixed; then a fresh reviewer, on the fixed site, answers
  the same ten again, to show the fixes read as meant.
- **Every label leads to a line of plain words.** *Decisions and the roadmap* has a row for every
  milestone and increment of the roadmap, anchored by its id (`M1.10` as `#m1-10`), saying in a
  sentence what it covers and whether it is done. Each Loft lesson a page names has a row too
  (`#l15`), which links its section of the lessons file. The guide's labels link those rows; the
  crates' documentation links them by the site's address.
- **The table can't go stale.** The site check fails if a milestone of `ROADMAP.md` has no row, a
  row names no milestone, or a row's status isn't the roadmap's: `done` when checked off, `blocked`
  when marked so, `not yet done` otherwise.
- **An `<a id="...">` is an anchor.** A page's raw HTML may name an anchor for a place that isn't a
  heading, such as a table row. mdBook keeps the `id`, and GitHub scrolls to it, so the link
  check accepts it like a heading's.
- **The API reference is checked for bare labels,** as the guide is: the visible text of every
  rustdoc page, except the source pages, which quote the code as written.
- **Questions become examples.** What a new user most asked for, and couldn't do from the pages,
  is shown by a program that CI runs and the page quotes: flying your own rocket with its centre of
  pressure and stability margin, reading a motor file and the bundled motors, each kind of wind,
  and recording a trajectory.

**Alternatives.**

- Headings for milestones and lessons on the records page: a heading can't hold a label, so its
  anchor would be words (`#staging-clusters-and-air-starts`) that a writer can't derive from the
  label.
- Linking each label to its phase of the roadmap or its section of the lessons file: closer than
  the top of the file, but still a long list to search, and the reader lands on GitHub rather than
  a line of plain words.
- Anchors in `ROADMAP.md` itself: it is a working file, edited with every milestone, and not a
  page of the site, so a reader still leaves the site to decode a label.

**Consequences.**

- Checking a milestone off in `ROADMAP.md` fails the site check until its row on *Decisions and the
  roadmap* says `done`; the error gives the row to write. A new milestone needs a row.
- A page that names a lesson without a row fails the link check until the row is added.


## ADR-021: Whole flights against RocketPy: what is compared, and the gaps it may declare (2026-09-18)

**Context.** M2.1b2 scores hpr's whole flights against the same-drag reference M2.1b1 committed,
`validation/fixtures/flight/rocketpy-whole-flight.json`. Its done-when asks for at least five cases
that pass. RocketPy's five examples in that reference include Prometheus 2022, which peaks at Mach
1.014, and hpr refuses `M ≥ 1` until M1.8 (ADR-008, ADR-011). The first run of the other four put
hpr 2.7 to 3.9% high on peak acceleration and 6 to 9% high on apogee, everywhere but NDRT 2020.
At Valetudo's peak, 0.034 s after ignition, there is no drag yet, so the difference had to be
thrust or mass. The reviews then found three definitional differences and a real one: the
landing points, recorded but not compared, were far apart in wind.

**Decision.**

- **The designs fly the thrust curve as RocketPy does.** RocketPy's
  `Motor(reference_pressure=None)` default, which every example keeps, makes its `pressure_thrust`
  zero (`motor.py:1188-1189`). `cargo xtask designs` had written hpr's 101,325 Pa stand-in
  instead, which adds `16 kPa × A_e` of thrust at a 1,400 m site; NDRT, at 206 m, was the one case
  it barely touched. `Nozzle::reference_pressure_pa` is now an `Option`, and the key is required
  (`null` for none), so a design says which it means; the transcription writes `null`. The site's
  example flights move with it: the first flight's apogee goes from 874.0 to 779.0 m.
- **A sixth rocket, not a weaker bar.** Bella Lui, on M2.1's own list of example rockets, is added
  to `flight.py` alone, with its example's site and rail and a declared wind (its example's weather
  is an ERA5 file).
- **The reference records everything the flight needs, and the harness checks hpr against it**
  (L75): the parachutes, RocketPy's `effective_1rl`, and the motor (total impulse, burn-out time,
  propellant mass, reference pressure). A design whose dry mass, reference area or motor differs
  by more than 1e-9, or a case whose drag is not the generator's declaration, is refused.
- **Metrics mean what RocketPy means by them** (L80). Speeds and accelerations are those of the
  centre of dry mass, the point RocketPy's state follows (`v_O + ω × p`,
  `a_O + ω̇ × p + ω × (ω × p)`). Heights are measured from that point's height at launch, because
  RocketPy's starts at the ground (`z_init = elevation`): the main opens and the flight lands at
  RocketPy's heights too. The rail exit is where the rocket has travelled `effective_1rl`, the
  forward button at the top; hpr's own exit is the last guide's (L26). The maxima are over both
  ends of every solver step, so a peak at an event, such as a canopy opening, is read at its
  instant. The drifts of the apogee and the landing point, and the landing speed, are added to
  `flight.py`'s metrics, as M2.1 lists them.
- **The committed report is pinned where the platforms agree.** A whole flight reproduces across
  macOS, Windows and Linux to about 1e-8 of each value, not always to the sixth decimal the report
  prints (NDRT's landing drift is 354.240893 m on macOS and 354.240895 m on Linux; one number of
  75 differed). ADR-015 asks to find out why before loosening. hpr is deterministic on each
  platform, and the likely source is the platforms' maths libraries, whose `sin`, `cos` and `exp`
  differ in their last bit between macOS, glibc and MSVC; an 84 s flight in a sheared wind carries
  that to 6e-9 of the drift. This amends ADR-015's six-decimal rule for whole flights: the test
  that holds the committed report to this run's allows a number two units of its sixth decimal
  or 1e-7 of itself, and nothing else; every word, tolerance and verdict must match. It is still
  a million times tighter than any gate.
- **A known gap is declared, checked and pinned.** A case may say `known_gap = "..."`. The harness
  accepts one kind, hpr's refusal of a real Mach number at or past 1; it checks that the reference
  reaches Mach 1 and fails the run once hpr flies the case
  ([Loft lesson L85](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md)).
  The report lists gaps in a section of their own; they count neither as a pass nor as a fail, and
  a test pins the set.
- **What does not agree is reported, not scored, and not called a pass.** In wind, hpr turns into
  the wind less than RocketPy: its apogee moves 67 to 85% as far upwind (Juno III 769 m against
  1,147 m, ending 228 m from the pad against 582 m). Flown once in calm air, the two agree on the
  apogee to 0.18% and on the drifts to 1.3 to 3.7%. The drifts of the four windy cases, and
  Valetudo's still-air landing drift (−3.41%), are open misses whose cause is unknown: they are
  printed with both numbers and pinned, issue #50 tracks them, and **M2.1's landing offset is not
  met** until it closes. A reviewer argued they should count as failures; they don't, because a
  suite that fails on a known, tracked gap can't gate anything else, but the roadmap, the status
  and *Accuracy* say plainly that the metric is not met. So are Calisto's time of peak acceleration (two peaks
  0.9% apart, which hpr's rail terms reorder) and NDRT's whole-flight peak, the main opening, where
  RocketPy has added mass and hpr has none (ADR-012). Every other metric is gated at 3% with no
  floor, as ADR-015 requires.

**Alternatives.**

- Scoring four cases and calling Prometheus the fifth: it compares nothing, so it would not be a
  pass.
- Keeping the stand-in and gating at 10%: that would hide an input difference as a model
  difference.
- Leaving the drifts out of the metrics, as M2.1b1's fixture did: M2.1 names the landing offset,
  and the difference is the largest one found.
- Gating the drifts with a tolerance wide enough to pass: that is the "no single target" excuse of
  L82 with a number on it. They are printed, pinned and tied to an issue instead.
- Flying Prometheus's subsonic part only, or giving it a drag that keeps it subsonic: a different
  flight from the reference's, or a reference tuned to suit hpr.

**Consequences.**

- The heights, speeds, times and accelerations of five flights agree within 3%, the largest
  +1.783% (Bella Lui's 7 ms ignition spike on the rail, where hpr keeps variable-mass terms that
  RocketPy's `udot_rail1` leaves out: +1.2 to 1.3 m/s² with thrust and mass the same to five
  digits) and +1.710% (Juno III's apogee, in the strongest wind). The path in wind is open
  (issue #50): each code's own normal force and damping, hpr's drag growth with the angle of
  attack and the rail release are the candidates.
- No motor gets the sea-level correction by default: catalog and `.eng` motors carry no nozzle,
  and a design's nozzle must say which reference pressure it means.
- When M1.8 lifts the Mach limit, the Prometheus case fails until its gap is removed. Its
  tolerances are already argued in its case file.

## ADR-022: Validation in CI, and regenerating references only by hand (2026-09-18)

**Context.** M2.1c asks for a CI job that runs `cargo xtask validate` against the stored references
and is green on three OSes, and for a separate workflow, triggered only by a person, that
regenerates the references and whose output is a diff to review, never a commit. `cargo test`
already reran every case and compared the result with the committed report, but inside a test
that nobody would read as "the validation suite ran", and `cargo xtask validate` itself rewrote
the report rather than checking it. hpr is bit-identical on one platform, not across three
(ADR-015), so the committed report cannot be compared byte for byte on Windows or Linux. The
references come from RocketPy, which needs the oracle environment in the gitignored `refs/` (uv,
RocketPy 1.13.0 and its dependencies); no CI job had one. Loft lesson L76 is why a reference must
never move on its own: Loft's advice to regenerate a reference whenever a check failed let the
reference follow Loft's own drag.

**Decision.**

- **`cargo xtask validate --check`** runs every locked case, writes nothing, and fails unless every
  scored metric passes and the run reproduces the committed `latest.md` and `latest.json`. The
  comparison is `Report::reproduces`, moved from the report test into `hpr-validate` so the test
  and the command share one definition: everything that cannot differ by platform (cases, sources,
  tolerances, verdicts, notes, gaps but for the Mach number the integrator narrowed onto) exactly,
  and hpr's and the reference's value at full precision from the JSON, to 2e-6 or 1e-7 of
  itself, whichever is larger. `latest.md` must be `latest.json`'s rendering exactly, since both
  come from one platform. The old test compared the rendered Markdown instead, where a printed
  percentage's last digit can round the other way on another platform.
  `--check` with `--fast` is refused: a partial run cannot check the whole suite's record.
- **A `validate` job** in `ci.yml` runs it on `ubuntu-latest`, `macos-latest` and
  `windows-latest`, with no oracle and no network; the Pages deploy waits for it.
- **`scripts/regenerate-references.sh`** runs the chain behind the references the harness reads, in
  order: `rocket_mass.py`, `cargo xtask designs`, `recovery.py`, `flight.py`, then
  `cargo xtask validate --check`. Each generator writes to a temporary file, so a failure leaves
  the committed fixture alone. The report is rewritten only when the check fails: the committed
  one holds macOS digits, and a Linux run would otherwise rewrite it on every run for last-digit
  rounding alone. A fixture that moved at all changes the hash the report records, so the report
  is then rewritten. It is written even when a metric fails, so the diff shows what moved, and the
  script then exits non-zero.
- **The *Regenerate references* workflow** (`regenerate-references.yml`) runs the script on
  `macos-latest`, an arm64 Mac like the one the committed references came from, so that an empty
  diff means something, and only on `workflow_dispatch`. Its token has `contents: read` and the checkout keeps no
  credentials, so it cannot push. It uploads `references.diff` and a summary as an artifact, and
  collects them even when the script fails.
- The script covers the harness's references and the design fixture they are built from. The
  other oracles' fixtures (atmosphere, geodesy, shapes, walls, motors, ThrustCurve) feed unit
  tests, not the harness; they keep their own commands.

**Alternatives.**

- Diffing the report `cargo xtask validate` writes with `git diff --exit-code`: exact bytes fail
  on the platforms that round the last digits differently.
- Only the existing test: it runs the same check, but a reader of CI cannot see the suite ran.
- A workflow that opens a PR or commits: that is L76 automated. A person commits the diff, in a PR
  that says why the reference moved.
- Running the oracles in CI on every PR: RocketPy's environment takes minutes to build, and a
  stored reference is the point of ADR-015.

**Consequences.**

- A change that moves a validation number cannot merge without the report that says so, on any
  of the three OSes.
- The workflow could not run before it was on `main` (GitHub only dispatches workflows the default
  branch has). The script ran locally first, on macOS: in 41 s it reproduced every committed fixture
  and the report byte for byte.
- M2.1c2's predicted-mode reference joins the chain when it lands.

## ADR-023: Predicted mode: each code's own drag, reported against a target (2026-09-18)

**Context.** M2.1 asks for every whole-flight case to run in two modes: same-drag, which M2.1b2
scores, and predicted, in which hpr flies its own aerodynamics. Its done-when asks for
predicted-mode results in the report, "with explained gaps", and for `M ≥ 1` cases to be reported
as gaps until M1.8. The committed whole-flight reference flies a declared constant `C_D0` of 0.5,
so hpr's own drag scored against it would measure hpr's drag against an arbitrary number. The
like-for-like reference is RocketPy flying each example's own drag, whose curves carry their own
terms and stay in the gitignored `refs/` (ADR-009). And neither code's drag is the truth: each
example's came from RASAero, OpenRocket or its team, and M1.5b already measured hpr's drag 47%
below Valetudo's table and 6.0% below Juno III's at Mach 0.3.

**Decision.**

- **A second reference,** `validation/fixtures/flight/rocketpy-whole-flight-own-drag.json`, from
  `flight.py --own-drag`: the same six cases, each with the drag its example flies in RocketPy
  1.13.0. That is the Calisto, Valetudo and Juno III curves read from `refs/rocketpy`, NDRT 2020's
  constant 0.44, Bella Lui's 0.43, and Prometheus 2022's `prometheus_cd_at_ma` (ported from
  RocketPy's MIT test fixtures, with 1.02 times it power-on). "As RocketPy flies it" was checked in
  RocketPy's source: `Rocket.__init__` fixes the drag the flight reads (`power_off_drag_7d`), so
  the Juno III notebook's rescaling and Bella Lui's replacement curve, both applied to the rocket
  afterwards, never reach the flight, and the reference does not apply them either. The fixture
  records each curve's path and SHA-256 and each constant, never a curve's values. It reproduces
  byte for byte, and the same-drag fixture is unchanged by the new flag.
- **A case says its mode:** `mode = "predicted"` under `[flight.whole_flight]` (`DragMode`;
  same-drag is the default). A predicted case flies the design with no drag table. Each mode
  refuses the other's reference: a predicted case against a declared table would score hpr's drag
  against a constant, and a same-drag case against the own-drag reference would fly a table the
  reference never flew. The L75 checks (design, dry mass, reference area, motor) apply to both.
- **Targets, not gates.** Each predicted metric keeps M2.1's 3% as a *target* and gets a verdict of
  `within target` or `outside target` (`Verdict::WithinTarget`, `Verdict::OutsideTarget`, from
  `Comparison::targeted`). Neither counts as scored, neither fails the run, and the rows sit in the
  report's own *Predicted mode* section, apart from the gated table and from *not scored*, the
  harness's escape hatch. Every miss is explained in its case file with its measurement. The
  report is pinned like the rest (ADR-022), so a predicted number that moves still has to be
  committed. A test pins that every predicted row, and no other, is a target row.
- **Prometheus 2022 is a known gap in predicted mode too:** RocketPy on its own drag peaks at
  Mach 1.049, and the harness checks the gap as it does the same-drag one (L85).
- **Predicted mode flies at rtol = atol = 1e-11,** same-drag mode at the default 1e-8. Its
  drag calls `ln` and `powf`, whose last bits differ between platforms' maths libraries, which
  moves the adaptive step sequence, so the answer differs by the solver's global error: at 1e-8,
  CI measured NDRT 2020's predicted apogee 1.7e-7 apart on macOS and Linux (1404.058522 against
  1404.058761 m), past ADR-022's 1e-7 reproduction bound. On macOS that apogee is 1404.058522,
  .057883, .058122 and .058145 m at 1e-8 to 1e-11, so 1e-11 converges it to about 1e-5 m, for
  0.5 s more over the suite. The bound is not loosened.
- **Peaks are found between the solver's steps, not only at them.** At 1e-11, CI still found
  NDRT 2020's predicted max Mach 6.6e-6 apart on macOS and Linux, and its max speed 1.2e-7 apart
  on macOS and Windows. The harness read each peak at the steps' ends, so a peak was off by
  wherever the step control put them, and that differs between platforms. Moving predicted mode's
  tolerance by 1e-7 of itself stands in for that: it moved NDRT's max speed by 2.4e-6 and Bella
  Lui's by 2.7e-6, but the event-located apogee by only 1.3e-9. (Changing the tolerance by one
  part in 10⁷ imitates another platform's step sequence on one machine.) Now, wherever speed, Mach or
  acceleration rises out of a step's start and falls into its end, a golden-section search on the
  step's dense output finds the peak between them. The same perturbation then moves no metric by
  more than 7.9e-9. The step-end reading had been low by up to 6.3e-5 (same-drag NDRT's max
  speed). Every verdict stands, and five of *Accuracy*'s cells moved in the third decimal. The
  bound is not loosened. This departs on purpose from measuring as RocketPy does (Loft lesson L80,
  that a metric must mean what the reference means): RocketPy reads its maxima at its solution's
  points only. hpr's peak now reads at or above its own step-end reading, by at most the 6.3e-5
  above. How much RocketPy's sampling misses depends on its steps and was not measured; its
  maxima move by at most 7.8e-6 between its tight and loose runs. Either way it is far inside 3%.
  The alternative, a platform-dependent number in a report that must reproduce on three
  platforms, is worse. The search applies in both modes, and to the power-on maximum.
  Its limits, from sampling every step at 400 points: a step whose quantity turns more than once,
  or jumps (the skin friction at the critical Reynolds number), is not searched, and none of those
  is a flight's maximum today. The computed acceleration carries about 1e-7 m/s² of numerical
  noise, so a smooth acceleration peak is found to about 1e-8 of itself and its time to about
  1e-4 s. Both are [issue #53](https://github.com/nrdptel/hpr-sim/issues/53).
- The set of predicted rows outside their target is pinned by a test, as the not-scored set is,
  so a case file's "nothing else misses" cannot go stale unnoticed.
- `scripts/regenerate-references.sh` regenerates the new reference with the others, and now
  prints how far each fixture moved, number by number. ADR-022's first dispatched run, on GitHub's
  macOS runner, showed why: RocketPy's fixtures moved in their last digits (the descents by at most
  3.6e-11 relative; in the whole flights, a landing height of 2e-8 m by 3e-9 m), which changed
  their hashes and so the report, with no printed metric moving.

**Alternatives.**

- Scoring predicted mode against the same-drag reference: it measures hpr's drag against 0.5.
- Gating it at 3%: two of five apogees miss by 10%, from drag tables that are not the truth
  either, and hpr's drag runs on placeholder fin edges and finishes where the examples record
  none. A gate would fail the suite on a disagreement nobody can yet settle, or be loosened to
  pass, which rule 2 forbids. M2.1's own done-when asks for predicted-mode results "reported, with
  explained gaps", not passed. `VALIDATION.md`'s initial targets called the 3% "targets, not
  gates, until the first report exists"; since then it is a gate in same-drag mode and, by this
  decision, a target in predicted mode.
- Declaring every predicted metric *not scored*: it would bury the same-drag suite's eleven open
  misses among 75 routine notes, and a target that is met would read like one that is not.
- Committing the examples' curves, or reading them in CI: their terms forbid the first (ADR-009),
  and CI has no RocketPy checkout.

**Consequences.**

- The heights: Calisto −0.527%, Bella Lui +1.118%, Juno III +3.181%, Valetudo +10.007% and NDRT
  2020 +10.232%, each where hpr's drag sits against the example's. Flown on the same drag, all five
  agree within 1.710%.
- A reader can see how hpr's own aerodynamics compare with the drag RocketPy's examples ship, and
  why; nothing says which drag is right until real flights (M2.3).
- When M1.8 lifts the Mach limit, both Prometheus cases fail until their gaps are removed.
- A predicted case's known gap is justified by the reference reaching Mach 1, as a same-drag
  one's is. hpr's own drag could take a rocket past Mach 1 where RocketPy's example stays below
  it; no case does that today (both Prometheus references exceed Mach 1), and such a case would
  have to leave the lock or wait for M1.8.
- A predicted flight past Mach 0.8 would fly hpr's drag beyond the range its build-up is
  documented for, and the harness does not flag it; none does today (Calisto peaks at Mach 0.746).

## ADR-024: The time-series RMS: aligned at ignition, held to 3% of its trace's scale (2026-09-18)

**Context.** M2.1 lists a "time-series RMS after alignment" among its comparisons, and M2.1a to
M2.1c built none. Each whole-flight fixture already carries a `series`: 120 rows of time since
ignition, the height of the centre of dry mass above the ground, and its speed, on a uniform grid
from ignition to RocketPy's impact. Nothing said what "alignment" means or what bound the RMS
answers to. hpr keeps no trajectory after a run, and the report's other rows are point metrics
held to 3% of their own reference.

**Decision.**

- **Two metrics per whole-flight case,** `series_height_rms_m` and `series_speed_rms_m_s`: the
  root mean square of hpr's value less RocketPy's at the reference's own series times, over every
  time at or before hpr's landing. Their reference value is 0, exact agreement. The harness
  requires every whole-flight case to name both.
- **Alignment is the shared clock.** Both codes start their clocks at ignition with the rocket on
  the rail, so the times align as they are. No time shift is fitted: a fitted shift would absorb a
  real difference in the burn or on the rail, which is what the comparison is for. hpr's values
  come from the dense output of the solver step that holds each time, not from interpolating
  between samples, so the comparison adds no error of its own beyond the step's interpolant.
- **The window is while both fly.** RocketPy's series ends at its impact; hpr's comparison stops
  at hpr's landing, the same event its `flight_time_s` measures. A landing-time difference is
  already gated by that metric, so it is not counted twice as a tail of ground-level zeros.
- **Each RMS is held to 3% of its trace's scale:** the height RMS to 3% of the reference's apogee,
  the speed RMS to 3% of its max speed, rounded down to 0.1. That is M2.1's 3% for a point metric,
  applied to a whole trace, so a trace passes only if it is on average no further off than its
  peak may be. The bound was set before the first measurement. The gate test holds each RMS gate
  no looser than that scale, so no absolute floor can widen it. In predicted mode the same bound is
  a target (ADR-023).

**Consequences.**

- Measured, same-drag: height RMS 1.4 to 39.2 m (0.12% to 1.5% of apogee, Juno III the largest)
  and speed RMS 0.13 to 2.06 m/s; all ten RMS rows pass. Both Prometheus 2022 cases name both
  metrics with their bounds but fly nothing: they stay the checked `M ≥ 1` gap until M1.8, so ten
  of the twelve whole-flight cases report an RMS. Predicted: Valetudo's height RMS and NDRT 2020's
  height and speed RMS are outside target, for the drag that puts their apogees 10% high; each is
  explained in its case file and pinned with the other misses.
- The RMS weights every grid time equally, so a long descent weighs more than a short burn. A
  difference in the burn shows in the point metrics (burnout, max speed and acceleration), which
  remain gated. The speed RMS is loose on the descent, where speeds are 5 to 25 m/s against a bound
  set by the top speed (Calisto falling under its main at twice RocketPy's rate would give about
  5 m/s, inside 7.3); the descent rate is gated by `impact_speed_m_s` instead.
- The window ends where the first code lands, so the tail where the heights differ most is not
  counted; the same-drag `flight_time_s` gate covers it, and in predicted mode it is a target.
- The horizontal path is not in the series, so the drifts stay with issue #50 (M2.1d2).

## ADR-025: The calm-air cases, and Juno III's drifts left to the rail release (2026-09-18)

**Context.** In wind, hpr's whole flights turn into the wind less than RocketPy's (issue #50), so
the windy cases report their drifts without scoring them. Issue #50 also flew three of those
rockets once with no wind, to separate the response to wind from everything else. M2.1d2 commits
those calm-air runs as cases, with the drifts scored at M2.1's 3%.

**Decision.**

- **Three calm-air cases:** Juno III, Calisto and Bella Lui, each flown by RocketPy on its windy
  case's rocket, rail, site and declared drag with both wind components zero (`flight.py`'s
  `CALM_AIR_BASES`). Only the same-drag fixture has them: they measure the response to wind, which
  predicted mode would mix with the drag. Regenerating the fixture left every earlier case
  bit-identical.
- **Every metric scored at 3%, drifts included,** with each RMS held to 3% of the calm reference's
  apogee and max speed, as ADR-024 sets. Calisto and Bella Lui pass every scored metric: drifts
  −1.258% to −2.583%, every other within 1.8%. Calisto's `max_acceleration_time_s` is not scored,
  for the reason its windy case gives (two peaks 0.9% apart).
- **Juno III's two drifts are reported, not scored,** because they miss by 3.7%, and a measured
  difference between the codes' rail models accounts for about 1.6 of those points. hpr keeps the
  rocket guided until its last rail button leaves the rail; RocketPy frees it when its first
  button does (`effective_1rl`). Neither models tip-off, the nose dipping as the rocket pivots on
  its last button (hpr's `rail.rs` says so), and tip-off would add drift, so hpr's full guidance
  is the further of the two from a real launch. Juno III's buttons are 1.41 m apart, twice Calisto's, so hpr
  guides it along its 85° rail for 1.41 m more, and its path stays steeper: apogee drift −3.670% and
  landing drift −3.695%, with the apogee within 0.060%. `rail_release.py` flies each calm case in
  RocketPy on a rail longer by its button spacing, so both codes free the rocket at the same
  point:

  | case | spacing | RocketPy drift, apogee / landing | with the longer rail | hpr against it |
  |---|---|---|---|---|
  | Juno III | 1.410 m | 570.1 / 651.3 m | 561.0 / 641.0 m | −2.1% / −2.2% |
  | Calisto | 0.700 m | 453.5 / 515.8 m | 451.9 / 514.0 m | −0.9% / −1.1% |
  | Bella Lui | 0.600 m | 21.5 / 27.7 m | 21.3 / 27.3 m | −0.5% / −1.3% |

  With the release matched, every calm drift is within 2.2%, and Juno III's would pass. The
  release closes 43% of Juno III's gap (9.1 of 20.9 m at apogee), 25% to 28% of Calisto's and 50%
  to 66% of Bella Lui's. What remains is negative in all six drifts, −0.5% to −2.2%: hpr's path is
  steeper for a second reason, not yet named. The case file gives these numbers.

**Alternatives rejected.**

- *Loosening Juno III's drifts to 4%.* That would widen a gate to fit a result, which the hard
  rules forbid, and would hide the cause.
- *Flying the calm references on the longer rail.* hpr reads its rail from the reference, so it
  would fly the longer rail too and still free the rocket later. Matching the release needs a
  change to one code's rail model, which is M2.1d3's first candidate for issue #50.
- *Leaving Juno III failing.* The suite has to be green to merge, and the miss is measured and
  partly explained. It stays visible in the report as a not-scored row with its value.
- *Scoring Juno III's drifts against a release-matched RocketPy drift, at 3%.* That keeps a live
  bound (it would pass at −2.1% and −2.2%), but it needs the release-matched run committed as a
  fixture the harness reads. M2.1d3 takes it up with the rail release.

**Consequences.**

- In calm air, the rail release is a quarter to two thirds of each drift gap, and the rest has
  one sign in every case. In wind, hpr's upwind shift is 67% to 85% of RocketPy's (issue #50):
  Juno III's in-wind gap is 378 m, against about 9 m from the release. So M2.1d3 still looks at
  the response to wind, and at the steeper path that remains in calm air.
- `rail_release.py` is a measurement, not a fixture or a check. If M2.1d3 matches the release,
  Juno III's drifts should be scored again.

## ADR-026: The path in wind: RocketPy's corrected equations, and hpr's body lift (2026-09-18)

**Context.** In wind, hpr's whole flights turned into the wind less than RocketPy 1.13.0's
(issue #50). Juno III's apogee drift was −60.8% of RocketPy's (its landing drift +151%), NDRT
2020's −18.6%, Bella Lui's −15.6% and Calisto's −5.6%. In still air, Valetudo's landing drift was −3.4%, and in calm air
Juno III's drifts were −3.7% (ADR-025). Same-drag mode shares only `C_D0`, so M2.1d3 looked at
everything else that turns a rocket in wind: the rail release, the drag's growth with angle of
attack, the normal force and the damping.

**How it was found.**

- *One candidate at a time.* In a local build with switches (not committed), hpr was flown with
  each candidate matched to RocketPy. Juno III's apogee drift went from −60.8% to −44.1% with body
  lift off, −58.3% with the rail release at the first button, −60.5% with a linear normal force and
  −60.9% with no angle-of-attack drag factor. With all four matched, and Juno III's fin slope
  raised 7.6% to RocketPy's, it was still −31.9%. Something else was at work.
- *The same state in both codes.* RocketPy's states along Juno III's windy flight were fed to
  hpr's equations of motion (a local probe of `Simulation`'s derivative). With hpr's normal force
  made linear like RocketPy's, the two agreed on the lateral acceleration to 1%. But at the rail
  exit, where the rotation rate is zero and no damping of any kind acts, RocketPy's angular
  acceleration was 1.75 times hpr's (0.3608 against 0.2066 rad/s²). After burnout the two agreed.
  With no rotation, both codes' rotational equations reduce to the moment about the centre of mass
  over the inertia there. So they disagreed about where the centre of mass was.

**The cause, in RocketPy.** `Flight.u_dot_generalized`, RocketPy's default 6-DOF equations, is
written about the centre of dry mass (CDM) and needs the vectors *from* it to the centre of mass
(`r_CM`) and to the nozzle exit (`r_NOZ`). It reads `Rocket.com_to_cdm_function` and
`Rocket.nozzle_to_cdm`, which point *to* the CDM: the note in `rocket.py:996-1025` says
`com_to_cdm_function + center_of_mass == center_of_dry_mass_position`. So while the motor burns,
RocketPy takes its moments about a point as far forward of the CDM as the true centre of mass is
behind it. For Juno III at the rail exit that point is 1.315 m from the nose tip, where
RocketPy's own `center_of_mass` puts the centre of mass at 1.639 m. The margin RocketPy flies there
is 1.75 times the one its own `static_margin` reports. After burnout `r_CM` is zero and the
error ends. A rocket that is too stable during the burn turns into the wind too much.

Upstream, an outside contributor reported the sign in issue #1186 (2026-08-25) and proposed the
fix in PR #1196 (open, not yet reviewed, head `927e771e`): three edits that negate `r_CM` with its
derivatives, negate `r_NOZ`, and flip the `r_CM ^ w_dot` term in `v_dot`, which was written for
the reversed vector. PR #1196 builds on PR #1188, merged into `develop` 2026-09-09 and not yet
released, which corrects the nozzle gyration tensor's parallel-axis term from
`0.25 * nozzle_to_cdm**2` to `nozzle_to_cdm**2`, so that the jet damping uses the whole lever.
RocketPy's own `develop` branch agrees on the convention: the tip-off phase merged there in PR #920
(2026-09-14) says, at `flight.py:2086-2087`, "The generalized EOM store r_CM / r_NOZ as (point ->
CDM) vectors, i.e. the negative of the true-frame position; hence the sign flips below", and
negates `com_to_cdm_function` for its own use.

Two checks:

- *RocketPy against itself* (`validation/oracles/rocketpy/wind_response.py` prints it for every
  case). At the rail exit of each windy case, RocketPy's angular acceleration as released equals
  the moment about the mirrored point over the inertia, to four digits. For Juno III it is 0.3608
  rad/s²; about RocketPy's own centre of mass the same moment gives 0.2067.
- *Corrected RocketPy against hpr* (the local probe). With both corrections, at RocketPy's states
  through Juno III's burn with the rocket rotating, its angular acceleration agrees with hpr's
  (normal force linear) to 0.1 to 5%, the larger where the value is small after burnout, and its
  CDM acceleration to 0.01 m/s². So hpr's variable-mass terms and jet damping match RocketPy's
  corrected ones.

**What remains is the two codes' models.** With the corrections, RocketPy's drifts in wind move
by up to 78% (Juno III's landing; 87% on its own drag). hpr against them: Juno III −42.5% (apogee drift) and +40.9%
(landing drift), Bella Lui −11.3% and −23.8%, NDRT 2020 −4.65% and +1.95%, Calisto −0.99% and
+1.43%, Valetudo −0.93% and −1.95%. In calm air every drift agrees within 2.2%. `wind_response.py`
then adds hpr's choices to the corrected RocketPy one at a time. The drifts, in metres:

| case, drift | RocketPy as released | corrected | + release at the last button | + body lift | + thin fins | hpr |
|---|---|---|---|---|---|---|
| Juno III, apogee | 582.4 | 396.6 | 360.7 | 270.6 | 231.1 | 228.0 |
| Juno III, landing | 268.3 | 478.5 | 519.6 | 624.0 | 670.2 | 674.4 |
| Bella Lui, apogee | 123.9 | 117.9 | 110.9 | 104.7 | | 104.6 |
| Bella Lui, landing | 74.6 | 67.2 | 58.9 | 51.7 | | 51.2 |
| NDRT 2020, apogee | 69.6 | 59.4 | 58.1 | 57.1 | | 56.6 |
| NDRT 2020, landing | 341.0 | 347.5 | 348.3 | 354.4 | | 354.2 |
| Calisto, apogee | 447.1 | 426.4 | 424.3 | 422.2 | | 422.2 |
| Calisto, landing | 1195.5 | 1261.5 | 1265.7 | 1278.3 | | 1279.6 |
| Valetudo, apogee | 167.7 | 165.2 | 163.6 | 163.6 | | 163.7 |
| Valetudo, landing | 206.4 | 203.3 | 201.4 | 199.5 | | 199.4 |

With hpr's three choices, RocketPy lands within 1.4% of hpr in every windy case. The local build
agrees from the other side: hpr with its normal force linear, no body lift, the first-button
release, no drag factor and Juno III's fin slope matched is within 0.7% of the corrected RocketPy
in every windy case (Juno III 396.4 against 396.6 m). The three choices:

- **Body lift** (ADR-008): `C_N = K (A_plan/A_ref) sin² α`, `K = 1.1` (Galejs), acting at each
  body component's planform centroid. It is zero at small angles, but a slow rocket leaves the rail
  at a large one: Juno III at 18 m/s in an 8.5 m/s wind, its rail leaning 5° downwind, 26° off the
  airflow, where body lift is about half its normal force. Much of it acts ahead of the loaded
  rocket's centre of mass, the nose's above all (its planform centroid is 0.35 m from the tip, the
  centre of mass 1.64 m), so at a steep angle it moves the centre of pressure forward by about
  0.3 m and weakens the moment that turns the rocket into the wind. It turns the rocket more than
  it pushes it: in RocketPy with hpr's release and fins, Juno III's apogee drift is 328.0 m with no
  body lift and 231.1 m with it, 232.6 m with the nose's alone, and 311.2 m with all of it placed
  at the centre of mass, where it can only push (Bella Lui: 110.9, 104.7, 108.2 and 110.0 m).
  RocketPy's normal force is linear in `α` and has no body term.
- **The rail release** (ADR-025): hpr guides the rocket until its last button leaves the rail;
  RocketPy frees it at the first. Neither models tip-off, the pivot about the last button between
  the two, so the real release lies between them: each is a modelling choice.
- **Juno III's fins:** the example gives them an airfoil lift curve, which RocketPy uses in place
  of the thin-plate `2π`, and which hpr does not model; RocketPy's fin slope is 7.6% steeper.

The drag's growth with the angle of attack (hpr's, up to 1.3 times at 17°) moves no drift by more
than 0.1%, and hpr's `sin α` in place of `α` moves none in wind by more than 0.9% (local
build).

**Decision.**

- **The oracle flies RocketPy 1.13.0 with PRs #1188 and #1196 applied** (`corrections.py`). The
  corrected `u_dot_generalized` is RocketPy's own source recompiled with #1196's three edits, each
  required to match exactly once, so a RocketPy that has moved stops the script instead of flying
  something else. The fixtures record both corrections (`corrections`), and their `model` names
  them. Both whole-flight references were regenerated; the mass and descent fixtures are
  bit-identical. RocketPy's apogees move by at most 1.6% (Prometheus 2022; Juno III +1.0%).
- **A drift is gated at 3% unless a measurement shows why it misses.** Six drifts reported before
  are now gated, and pass: Calisto's two in wind, Valetudo's and NDRT 2020's landing drifts, and
  Juno III's two in calm air (superseding ADR-025's decision on them). Five stay reported but not
  scored, each as a measured model difference: Juno III's and Bella Lui's two, and NDRT 2020's
  apogee drift. Prometheus 2022's drifts, excused before, are held to 3% for when hpr flies it.
- **hpr keeps its body lift and its rail release.** Body lift is a real force at these angles, and
  OpenRocket carries it too. The release at the last button is hpr's modelling choice (ADR-025);
  with no tip-off in either code, neither end is the real one.
- **Drop the corrections when RocketPy releases them.** Then re-pin the oracle and delete
  `corrections.py`, or keep only what the release still lacks.

**Alternatives rejected.**

- *Keeping RocketPy as released and the drifts unscored.* The reference would carry an error in
  its equations of motion that its own `static_margin` contradicts, that RocketPy's `develop`
  branch describes in a comment (PR #920), and that PR #1188 (merged) and PR #1196 (open)
  correct. hpr's correct answer would read as a miss:
  Juno III's apogee, +1.71% as released, is +0.70% corrected.
- *Only #1196.* It is written on top of #1188, which is merged upstream.
- *RocketPy's `develop` branch with #1196.* Unreleased, and it would move every reference for
  reasons unrelated to this one.
- *Gating the five drifts now against RocketPy flying hpr's body lift, release and fin slope.* It
  would check hpr's equations at steep angles as same-drag mode checks them with the drag shared,
  but not hpr's normal force against another's, and it needs a reference whose rail differs from
  the one hpr flies, which the harness does not read yet. Until then the five are pinned as the
  committed report pins every number (`validate --check` fails if one moves) and the regeneration
  script prints `wind_response.py` beside them. Filed as issue #61.
- *Dropping or shrinking hpr's body lift to agree.* That fits hpr to a code that leaves a real
  force out. `K` is uncertain (Galejs gives 1.0 to 1.5) but it is not zero.
- *Loosening the drifts' gate.* That would widen a gate to fit a result, which the hard rules
  forbid.

**Consequences.**

- Issue #50 closes. M2.1's landing offset is met in calm air, in still air (Valetudo), for
  Calisto in wind and for NDRT 2020's landing. It is not met for Juno III and Bella Lui in wind,
  where the two codes' models differ: body lift and the rail release by design, Juno III's airfoil
  fins a feature hpr lacks. The report shows both numbers.
- Whether `K` should be lower at these rockets' crossflow Reynolds numbers is open: crossflow
  methods other than Galejs's (Allen and Perkins; Jorgensen, NASA TR R-474) are not yet read.
- In wind, a slow rocket's drift in hpr depends on body lift's uncertain `K`. Flown in RocketPy
  with hpr's body lift, rail release and fin slope (`wind_response.py`), Juno III's apogee drift is
  240.2 m at `K = 1.0`, 231.1 m at 1.1 and 194.1 m at 1.5, and 328.0 m with no body lift; its
  landing drift is 659.5, 670.2 and 714.2 m. hpr itself gives 237, 228, 191 and 326 m (local
  build). Calisto, off the rail at 28 m/s and 11°, changes its drifts by under 0.5% across
  `K = 1.0` to 1.5. Which code is nearer a real flight is for M2.3.
- The corrections do not only move RocketPy toward hpr: Valetudo's apogee difference grew from
  +0.003% to +0.116%, and predicted NDRT 2020's apogee drift from −4.44% to +11.906% (outside its
  target before and after).
- The time-series RMS: Juno III's height RMS is 15.9 m, from 39.2, and Bella Lui's 1.9 m, from
  2.3; four others grow by under a metre (Valetudo 1.4 to 2.4 m, NDRT 2020 2.0 to 2.8 m, and
  Juno III's and Calisto's calm cases 1.3 and 1.1 to 2.1 and 2.0 m). Burnout speed now reads
  +0.018% to +0.063% in every case, where it read −0.066% to +0.040%: a small rest of one sign,
  far inside the gate.
- Predicted mode: 66 of 85 metrics within target, from 63; Calisto's drifts and Juno III's apogee
  are now within, and nothing new misses.
- `rail_release.py` now flies the corrected equations. The numbers ADR-025 quotes from it were
  measured before this correction; `wind_response.py` gives the current ones.

## ADR-027: The normal force through Mach 1: supersonic linear theory, a transonic join, and the measured references (2026-09-18)

**Context.** M1.8 is too big for one session and is split into M1.8a to M1.8d (`ROADMAP.md`).
M1.8a carries the normal force and centre of pressure through Mach 1, so that a flight on a drag
table can pass it; the drag buildup's transonic terms are M1.8b. Until now `hpr-aero` refused
`M ≥ 1` (ADR-008) and a flight stopped there (ADR-011), which left Prometheus 2022, the suite's
one supersonic rocket, a known gap in both modes. Loft lesson L7: Loft's fin slope had no
compressibility factor, so its slope and CP never moved with Mach.

Three findings shaped the method:

- *Niskanen's supersonic fin slope counts one surface.* [N09] eq. 3.48–3.49 write the fin's
  normal force as its area times one strip's pressure coefficient, `K₁α + K₂α² + K₃α³` with
  `K₁ = 2/β`. That is the pressure on one face. A flat plate carries the difference between its
  faces, `2(K₁α + K₃α³)` (the `K₂` terms cancel), whose first term is Ackeret's `4α/β`.
  Barrowman's own method sums every surface region ([B67] appendix A, pp. 82–86). Niskanen's
  thesis finds its simulated `C_Nα` for the Arcas Robin "notably lower than the experimental
  values", with the reason unknown (p. 91, over a comparison that runs to Mach 4). hpr uses both
  faces.
- *Barrowman's half-load in the tip's Mach cone is exact linear theory for a rectangle*, in lift
  and in CP: the exact cone load `(2/π) asin √t` averages one half, and [N09] eq. 3.35 (from
  Fleeman, *Tactical Missile Design*, p. 33) is the rectangle's exact CP. For tapered fins the
  strip method runs above exact linear theory: against [TN2114] eq. A7 (printed p. 18), a fin
  with a taper ratio of 0.5, an unswept trailing edge and `βA = 3` gets 4.5% more slope (worked
  by hand here; the research found +2.1% and +0.8% at `βA = 4` and 6).
- *No source gives the transonic region in closed form.* MIL-HDBK-762 reads it from McDevitt's
  transonic-similarity charts for rectangular fins (pp. 5-104–5-105), and [B67] §1.00 makes "no
  attempt" at it. RocketPy 1.13.0 flies Diederich's subsonic slope at every Mach number, with
  `β` held at 0.6 from Mach 0.8 to 1.1 (`aero_surface.py:51-56`); supersonic, that tends to
  `2π cos Γ_c/β`, about π/2 times linear theory's `4/β`. Its fin CP doesn't move with Mach.

**Decision.**

- **Supersonic fins: linear theory on the fin's outline** (`FinOutline::supersonic`). The load is
  `4α/β` per unit area, halved inside the Mach cone from the tip's leading edge, with the root a
  reflection plane (a cone that crosses the root counts its part over the mirror image). So
  `(C_Nα)₁ = (4/β)(A_fin − A_cone/2)/A_ref`, and the CP is the load's centroid. The outline is the
  planform's polygon (an ellipse as a 256-gon, 2.5e-5 short in area), clipped against the Mach
  line by Sutherland–Hodgman and summed by the shoelace formula without allocating. Only the
  first-order term: `K₃α²` adds 1% at 0.1 rad at Mach 2, but 35% at Mach 1.2, where the expansion
  itself fails.
- **Where linear theory starts:** `M_s = max(1.2, 1/cos Γ_L, 1/cos Γ_T, √(1 + 1/A²),
  √(1 + (c_t/2s)²))`: the bottom of [N09]'s supersonic region (Table 3.1), supersonic leading and
  trailing edges ([TN2114]'s case), `βA ≥ 1` with `A = 2s²/A_fin`, and the mirror fin's tip cone
  off this fin's tip, `β ≥ c_t/(2s)`. The last two came from review: `βA ≥ 1` alone let an
  inverse-tapered fin's two tip cones overlap its tip, and a property test over random
  trapezoids found forward-swept trailing edges, whose slope rose past `M_s`. Neither binds for
  any validated fin, so no reported number moved. Leading edges swept forward stay outside the
  half-load's domain (the tip ahead of the root; documented in `docs/physics/aero.md`).
- **The transonic join:** from Mach 0.8, the top of [N09]'s subsonic region and of the drag
  buildup's documented range, to `M_s`, slope and CP each linear in `M` between the subsonic
  method's values at 0.8 and linear theory's at `M_s`. Continuous, with the slope's peak at `M_s`.
  Below 0.8 nothing changes: Diederich's slope with Prandtl–Glauert at the quarter chord.
- **Bodies don't change with Mach** (slender-body theory, [B67] p. 18), and `K_T(B)` stays
  Barrowman's. `K_B(T)`, the fins' lift carried onto the body, stays out, as below Mach 1.
- **Ranges.** `AeroError::Mach` gains the refusing model's limit. The normal force covers
  `0 ≤ M < 5` (`NORMAL_FORCE_MACH_LIMIT`; [N09]'s hypersonic region starts at about 5); the drag
  buildup `0 ≤ M < 1` (`BUILDUP_MACH_LIMIT`) until M1.8b; a drag table any Mach number.
- **In flight.** A fin set's CP moves with Mach, so `AeroModel::component_station_m` takes the
  Mach number, and the flight takes each component's local airflow at its CP for the centre of
  mass's Mach number, each step.

**The references.**

- *RASAero II's Calisto export* (`refs/rocketpy-history/calisto-cd-test-2018.csv`, RocketPy's first
  commit, now pinned in the lock). Its `CNalpha (0 to 4 deg)` is the secant slope to 4° and, from
  Mach 0.95, includes a viscous cross-flow term that is zero below. So the comparison takes its
  `CN Potential` at 2° over the angle and its CP at 0°, against hpr's small-angle slope and CP. A
  code, not a measurement. The choice decides much of the result: against its 4° columns, 1 of
  the 11 rows from Mach 0.8 is within the targets, not 6, and Mach 2 is −30.6%. Below Mach 0.8
  the agreement is partly by construction, since ADR-009 gave the Calisto design the 2018 fins
  because they reproduce this export at low speed. The fixture commits the export's values at
  the 15 Mach numbers compared (30 values), more than ADR-009's one per curve; STATUS asks Neer
  whether that is fine.
- *NASA's half-scale Arcas wind-tunnel models* (TN D-4013, Mach 0.6–1.2; TN D-4014, Mach
  1.5–4.63): the Arcas Robin (short, 18.2 calibers) and the long bioscience version (23.8). The
  reports print no tables, so their plots were read on 600-dpi renders, each plot's grid
  calibrated locally, into `validation/fixtures/aero/arcas-robin-wind-tunnel.json` with every
  point's figure and page. The CP read back from C_m and C_N reproduces TN D-4014 Fig. 7 within
  1.2% of body length (0.0% at Mach 2.96). The reports plot `C_N` against `α`, so hpr is fitted
  the same way: its `C_N` at the plotted angles, a least-squares slope; its CP from its moment and
  normal force over −2° to 2°. `cargo xtask designs` builds both models from the recorded
  geometry. The nose is a coordinate table, not a tangent ogive; the design's power-series nose
  (`n = 0.6369`) has its volume, which sets the slender-body CP, and its planform within 2.4%.

**Result** (`validation/fixtures/aero/normal-force-vs-mach.json`, pinned by
`hpr_aero::tests::normal_force_against_mach`; 37 rows, 16 outside the targets). The targets were
written into `ROADMAP.md` before the comparison first ran, and landed in the same commit as it;
they have not moved since.

| reference, band | `C_Nα`, hpr against the reference | CP, calibers | within both targets |
|---|---|---|---|
| Arcas Robin, short, Mach 1.5–2.96 | −13.4% to +3.3% | −0.02 to +0.42 | 4 of 4 |
| Arcas, long, Mach 1.8–2.96 | −8.2% to +0.5% | −0.12 to −0.04 | 3 of 3 |
| Arcas Robin, short, Mach 3.96–4.63 | −19.6%, −25.0% | −0.17, −0.16 | 0 of 2 |
| Arcas, long, Mach 3.96–4.63 | −17.2%, −22.8% | −0.16, −0.19 | 0 of 2 |
| Arcas, both, Mach 0.6 | −2.4%, +3.6% | +0.06, −0.44 | 2 of 2 |
| Arcas, both, Mach 0.8–1.2 | −6.9% to +29.3% | −0.46 to +2.29 | 2 of 9 |
| Calisto RASAero II, Mach 0.1–0.7 | +0.1% to +10.1% | −0.08 to +0.43 | 4 of 4 |
| Calisto RASAero II, Mach 0.8–2.0 | −16.8% to +21.9% | −0.56 to +0.95 | 6 of 11 |

Every miss, measured:

- *Past Mach 3: the body.* Fins on less fins off, the fins' share agrees with hpr's fins within
  −1.4% to +7.0% at Mach 3.96 and 4.63. The body alone lifts 3.9 to 4.6 per radian there, fitted
  the same way, against hpr's 2.3 to 2.8 with its body lift. Slender-body theory's nose (2 per
  radian) and boattail (−1.15 here) don't change with Mach, and the real body's lift grows. The
  CP stays within 0.19 calibers, so the stability margin holds; the slope, and so the weathercock
  rate, is low. M1.8e takes this on. The fins' agreement is uncertain by about its own size: the
  design leaves out the strip where each fin's root follows the boattail below the cylinder, about
  0.32 in² of 5.8 in² (5.5%).
- *Mach 0.6 passes by cancelling errors:* hpr's body is 40% (short) and 34% (long) above the
  fins-off readings and its fins' share 9.3% and 3.9% below the measured one.
- *Transonic, Mach 0.8 to 1.2.* Fins on less fins off, the measured fin lift falls from Mach 0.6
  to 0.9 (9.5, 8.5, 7.8 per radian on the short model) and jumps at 1.0 (14.2); hpr's rises by
  Prandtl–Glauert and then along the join to linear theory's peak at `M_s = 1.2` (16.6 against
  13.0 measured there). The long model's CP jumps forward at Mach 1.0 (71.5% of its length
  against 75.6% at 0.8), 2.29 calibers from hpr's. These are the flows no closed-form method
  covers. The transonic body readings are poorly determined (±0.02 per point on a coarse grid).
- *RASAero II.* Its subsonic slope and CP stay constant to Mach 0.9; hpr's rise by Prandtl–Glauert,
  14.4% by Mach 0.8, and its fins' CP starts moving aft at 0.8, so the CP misses from Mach 0.8 to
  0.95 by 0.52 to 0.95 calibers. The wind tunnel sides with neither: at Mach 0.8 the short model's
  `C_Nα` is +11.9% in hpr against the measurement, and the long model's +12.0%. At Mach 1.3 hpr is
  just past its join's peak (+12.7%, CP +0.65); at Mach 2, −16.8% with the CP 0.56 calibers forward.
  Calisto has no fins-off data, so that miss can't be split into fins and body; the wind
  tunnel's gap at the same speeds is the body's.

Prometheus 2022, the suite's supersonic rocket, flies through Mach 1.010 on its declared drag:
14 metrics scored and passing, the largest its apogee at +1.525%. Its drifts are −9.273% (apogee)
and +6.283% (landing). `wind_response.py`, which now flies it, puts RocketPy with hpr's rail
release and body lift at 1484.3 m and 1468.5 m, within 0.1% of hpr's 1483.1 m and 1469.4 m. So
they are ADR-026's body-lift difference and are reported, not scored, as ADR-026's are. Near
Mach 1 it flies almost straight into the airflow (its angle of attack, sampled at hpr's steps by a
local probe of the harness, stays below 0.11° from Mach 0.8 to 1.2), so the transonic normal force
has little to act on. Its apogee, +1.525%, is body lift too: RocketPy with hpr's body lift and rail
release reaches 3735.4 m, against hpr's 3735.3. On its own drag it stays a known gap until M1.8b.
No other case passes Mach 0.8, so no other number in the report moved.

**Rejected.**

- *Tuning the join or `M_s` to the wind tunnel.* The misses would shrink by fitting the model to
  its own check. `M_s` and the join's start come from the sources' regions, set before measuring.
- *Niskanen's one-face slope and his quintic CP fit from Mach 0.5.* The first is half of linear
  theory; the second would move every subsonic flight's CP from Mach 0.5 with no measurement
  asking for it (the Arcas Robin's CP moves forward, not aft, from Mach 0.6 to 0.8).
- *RocketPy's Diederich slope past Mach 1.* About π/2 times linear theory.
- *The third-order Busemann terms.* See Decision.

**Consequences.**

- Flights on a drag table fly to Mach 5. On hpr's own drag they still stop at Mach 1 until M1.8b.
- M1.8e: the body's supersonic normal force, against the Arcas Robin's fins-off measurements.
- M1.8c's roll forcing and damping will use the same outline and its Mach cone; TN D-4014 Fig. 14
  (roll effectiveness per degree of cant, recorded in the digitization but not yet committed)
  and TN 2114's roll-damping formulas are its references.
- `AeroModel::component_station_m` and `FinSetAero` changed shape (`fin: FinAero`, `fore_station_m`,
  `cp_station_m(mach)`); nothing outside the workspace uses them yet.
