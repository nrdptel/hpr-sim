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
  32 motors, and the milestone's 1% check and offline catalog need them. Recorded under "Needs
  Neer" (not blocking) to confirm; the fallback is to keep only the numbers the checks use, or to
  take them from manufacturers' data sheets.
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
    case at +0.71% in descent time and +2.87% in the smaller drift component.
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
  entry, 2.1e-3, is on a 20 µm drift component that is not compared). The differences that remain
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
