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

