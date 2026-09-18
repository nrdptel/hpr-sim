# Solid motors

Code: `hpr_motor` (`curve`, `class`, `motor`, `grains`, `mass`, `delay`, `catalog`). File formats
are in `docs/format/eng.md` and `docs/format/rse.md`.

Sources:

- **[SP]** NASA SP-8039, *Solid Rocket Motor Performance Analysis and Prediction* (1971), pinned
  as `nasa-sp-8039`. Page notes: `docs/research/nasa-sp-8039-motor-definitions.md`.
- **[NAR]** National Association of Rocketry, *Standard Motor Codes*, `nar-standard-motor-codes`.
- **[TC-G]** ThrustCurve.org glossary, `thrustcurve-glossary`; **[TC-S]** its "Motor Statistics"
  page, `thrustcurve-motorstats`; **[TC-A]** its statistics code, `simulate/analyze/analyze.js`
  (ISC), `thrustcurve3-analyze`.
- **[RP]** RocketPy 1.13.0 (MIT), `rocketpy/motors/motor.py` and `solid_motor.py`, pinned as
  `rocketpy`. Notes: `docs/research/rocketpy-solid-motor.md`.

## Conventions

- **Time** `t` is seconds from ignition.
- **Motor axis:** positions are metres along the motor's axis **from the nozzle exit plane toward
  the forward closure**, so `+z` points toward the nose like the body frame (`frames.md`). The
  design model (from [M1.4][roadmap], the design and mass-properties milestone) places the
  motor's nozzle exit in the body frame.
- **Inertia:** every part is symmetric about the axis. `I_a` is about the axis and `I_t` about a
  transverse axis, both through the part's own centre of mass (RocketPy's `I_33` and `I_11`).
- Motor files keep their own units (mm, kg or g). The model is SI.

## Thrust curve

- Samples `(t_i, F_i)` joined by straight lines, as [RP] reads them (`interpolation_method`
  `"linear"`). Before the first sample the curve starts from `(0, 0)` when `t_0 > 0`, as the RASP
  format specifies and [TC-A] integrates. From the last sample on `F = 0`, that instant included:
  like any step, the end takes the later value, so at burnout the thrust, the mass flow and the
  propellant left are all zero together.
- Equal consecutive times are a **step**: the later value holds from that time on. Real files use
  them for an abrupt burnout. Decreasing times and negative thrust are rejected, and so is a curve
  with no impulse or no NFPA burn time.
- **Where [TC-A] differs.** Before integrating, ThrustCurve's code drops leading points below
  500 µN and averages points closer than 50 µs (lines 40–91); hpr keeps leading zeros and treats
  equal times as steps. The results are identical on every bundled curve. On the 1710 survey files
  that read, they agree to 1e-9 except 17 with repeated times, where they differ by up to 1.1% in
  average thrust. hpr keeps the step because a vertical drop is what the file draws.
- **Total impulse:** the exact integral of the lines, `I = Σ ½ (F_i + F_{i+1}) (t_{i+1} − t_i)`
  ([SP] glossary p. 96: `I = ∫F dt`). `I(t)` is the same sum up to `t`, with the partial interval.
- **Burn time (NFPA 1125):** from the moment the thrust first reaches 5% of peak to the moment it
  last falls to 5% of peak, each crossing interpolated on its segment ([TC-G] "Burn Time";
  [TC-S]; [TC-A] lines 165–203). The last sample's time is not the burn time
  ([Loft lesson L39][lessons]).
- **Average thrust:** total impulse over the NFPA burn time ([TC-G] "Average Thrust"; [TC-A]
  line 231).
  - [TC-S] says instead "the total impulse during the 5%-defined burn time". The two differ by the
    impulse outside the window, a median 0.13% on ThrustCurve's public-domain curves
    (`docs/research/thrustcurve-data.md`). hpr follows the glossary and the site's code.
- **Impulse class:** `upper(k) = 1.25 · 2^k N·s`, from `1/8A` (`k = −2`) to `O` (`k = 15`), and on
  to `Z` by doubling. **Upper limits are inclusive:** [NAR] states `C` as "5.01 to 10.0 N-sec".
  Loft gave `B` for 2.5 N·s ([Loft lesson L38][lessons]).

## Propellant consumption

The effective exhaust velocity is `c = F/ṁ` ([SP] glossary p. 95). **Held constant**, it fixes
the propellant burned in proportion to the impulse delivered:

```text
c = I / m_p0,    ṁ(t) = F(t) / c,    m_p(t) = m_p0 (1 − I(t)/I)
```

- This is [RP] `SolidMotor` (`solid_motor.py:401-418`; `motor.py:483-524`). ThrustCurve's `.rse`
  files tabulate their mass column the same way (`docs/format/rse.md`).
- **It is an approximation.** [SP] defines `c` only instantaneously. Measured `I_sp` drifts during
  a burn with chamber pressure and nozzle erosion ([SP] p. 14), so real consumption is not
  exactly proportional. No COTS data resolves the difference.
- All propellant is gone at the last sample. The whole burned mass leaves as exhaust; inert
  discharge (liner, igniter) is not modelled.

## Where the propellant is

- **Column** (`Propellant::Column`): a hollow cylinder `(R, r, L)` at a fixed centre. It keeps its
  shape and loses density as it burns, so `I_a = ½ m_p (R² + r²)` and
  `I_t = m_p ((R² + r²)/4 + L²/12)`. This is [RP] `GenericMotor`'s model (`motor.py:1580-1648`,
  a solid cylinder).
- **BATES grains** (`Propellant::Grains`): `N` identical grains `(R, r₀, h₀)`, spaced `h₀ + s`
  apart, burning on the bore and, unless inhibited, both ends ([RP] `solid_motor.py:487-632`).
  Every grain needs a bore (`r₀ > 0`): a solid end burner shortens without widening, which this
  regression (and RocketPy's) doesn't describe. Facing ends burn even with no gap, as in [RP].
  Every surface recedes by the same web `x`:

  ```text
  V(x) = π (R² − (r₀ + x)²) (h₀ − 2x)     ends burning,   x ≤ min(R − r₀, h₀/2)
  V(x) = π (R² − (r₀ + x)²) h₀            ends inhibited, x ≤ R − r₀
  N ρ V(x) = m_p(t)
  ```

  - [RP] integrates `ṙ = −V̇/A_b`, `ḣ = −2ṙ` in time with LSODA. Because `dV/dx = −A_b`, that ODE
    is the relation above. hpr solves it for `x` by safeguarded Newton iteration, exactly at any
    time.
  - `m_p0 = N ρ V(0)` comes from the geometry, as in [RP].
  - About the stack's centre, which doesn't move:

    ```text
    I_a = ½ m_p (R² + r²)
    I_t = m_p ((R² + r²)/4 + h²/12) + (m_p/N) (h₀ + s)² N (N² − 1)/12
    ```

    The last term sums `m_g d_k²` over the grain offsets ([RP] `solid_motor.py:724-740,
    784-789`).

## The whole motor

- **Dry mass** (case, closures, liner, nozzle) is one element: mass, centre and inertias about its
  own centre. `with_added_dry_mass` joins more hardware, such as a retainer. It must be positive:
  at burnout it is the whole motor, and with no mass it has no centre.
- The motor is the dry mass and the propellant combined about the instantaneous centre with the
  parallel-axis theorem: `z = Σ m_k z_k / m`, `I_a = Σ I_a,k`,
  `I_t = Σ (I_t,k + m_k (z_k − z)²)`. This is [RP] `Motor.I_11` (`motor.py:585-666`).
- **Envelope default** (`SolidMotor::from_envelope`), when only a catalog entry is known:
  - the propellant is a solid column of radius `D/2` filling the length, centred at `L/2`;
  - the dry mass (loaded minus propellant) is a thin tube, `I_a = m r²` and
    `I_t = m (r²/2 + L²/12)`, also centred at `L/2`.

  **These are crude:** the centre of mass stays at `L/2` throughout. Loft fixed the CG at the
  midpoint with no inertia of its own ([Loft lesson L40][lessons]); hpr gives the parts inertia,
  and moves the CG as soon as the dry and propellant centres differ.
  - ThrustCurve's loaded mass includes the reusable case ([TC-G] "Total Weight": "propellant and
    case").
  - [RP] `GenericMotor.load_from_eng` sets its chamber radius to the motor **diameter**
    (`motor.py:1759-1761`), which quadruples the `r²` inertia terms (and the default nozzle
    area it derives from that radius). hpr uses `D/2`.
- **Catalog envelope:** diameter, length and masses come from the catalog metadata, not the curve
  file's header, which can be wrong ([Loft lesson L43][lessons]).

### The effective exhaust velocity is a units check

Both constructors refuse a motor whose curve and propellant mass imply an effective exhaust
velocity `c = I/m_p` outside **200 to 5,000 m/s**. Nothing else in the API notices a units slip:
the 411I175's envelope in millimetres and grams read as metres and kilograms
(`from_envelope(curve, 38.0, 245.0, 228.9, 437.5)`) has positive, finite dimensions, a propellant
mass below the loaded mass, and an exhaust velocity of 1.8 m/s.

`I` is the curve's own impulse, uncorrected for ambient pressure, because that is the impulse the
model actually delivers: `state()` derives the mass flow as `F(t)/c` from the same uncorrected
curve, so `∫ṁ dt = I/c = m_p` closes exactly. The bound is therefore on `c` at the curve's
reference pressure, which for a sea-level-tested motor is a few per cent below its vacuum value.

Chemistry sets the scale, and rather than quote an `I_sp` table hpr has not read, the range comes
from the catalog itself. Measured over the 1,708 ThrustCurve.org simulator files that have both a
parsed impulse and a catalog propellant mass — the same mass `CatalogMotor::motor` uses, which prefers the metadata over
the curve file's header:

| percentile | min | p1 | p5 | p50 | p95 | p99 | max |
|---|---|---|---|---|---|---|---|
| `c`, m/s | 236 | 597 | 928 | **1,867** | 2,210 | 2,463 | 3,031 |

The bulk of that distribution is APCP; the low tail is the black-powder motors, which cluster
below about 900 m/s.

**Under that convention the bound rejects none of them.** That is the honest statement of what this
check is: a guard against a units slip, which moves `c` by a factor of 1,000, not a filter on
propellant. It has real headroom but not a great deal at the low end — the lowest real entry is
1.2x above the floor and the highest 1.65x below the ceiling — and the low tail is an accounting
artifact rather than chemistry: Estes and Quest normally record a "propellant weight" that includes
the delay grain and the ejection charge, neither of which delivers thrust impulse, so a small
black-powder motor reads lower than its propellant really is. A 1/8A recorded that way could fall
under 200 m/s; if one ever does, the fallback is issue #11's second option, applying the bound only
above a couple of grams of propellant.

Two numbers worth keeping straight, because both have been got wrong here:

- Reading the **curve file header** mass instead gives a different distribution (max 10,111 m/s,
  from a J motor whose header claims 83 g where its catalog entry says 396 g). hpr does not use
  header masses when the catalog has them ([Loft lesson L43][lessons], `catalog.rs`), so that file
  builds at 2,111 m/s and passes.
- The 32 bundled motors run **689.78 m/s** (a black-powder C) to **2,651.64 m/s** (a K), computed
  from each curve's own impulse — not from the catalog's stored `total_impulse_ns`, which differs
  by up to 0.3% and would say 2,645.

## Thrust at altitude

The thrust equation is `F = ṁ u_e + (p_e − p_a) A_e` ([SP] eq. 2, p. 3, with `g_c = 1` in SI).
With the flow unchanged, a curve measured at reference pressure `p_ref` gives, at ambient `p_a`,

```text
F(p_a) = F_curve + (p_ref − p_a) A_e,    A_e = π r_e²
```

- [SP] doesn't print this conversion; it follows from eq. 2. [RP] `Motor.pressure_thrust`
  (`motor.py:1173-1191`) applies the same term.
- `p_ref` is the ambient pressure at the static test site, stored with the nozzle
  (`Nozzle::reference_pressure_pa`). Motor files and catalogs don't record it, so standard
  sea-level pressure (101 325 Pa) is only a stand-in. For a test at 1500 m elevation (84.6 kPa
  in the 1976 standard atmosphere), the stand-in makes the term 16.8 kPa × `A_e` too large at
  every altitude.
- It holds while the nozzle flows full. Sea-level tests of altitude nozzles can separate
  ([SP] pp. 32–34).
- hpr applies it strictly inside the burn, `0 < t < t_end`, as RocketPy's flight does
  (`simulation/flight.py:1936-1956`), but only where the curve's thrust is positive (RocketPy also
  adds it inside zero-thrust gaps, where nothing flows), and never lets thrust go negative. Without a known nozzle it
  returns the curve. COTS files carry no exit diameter (`.rse` `exitDia` is always 0).
- **Limits.** The full-flow term steps in just after ignition and steps to zero at `t_end` (the
  integrator should treat both as events). In the ignition transient and the tail-off the real
  exit pressure is far from its full-flow value, so the term misstates thrust there. In the
  tail-off, where the exit pressure falls with the chamber's, it overstates it: on the 411I175 (a
  9.5 mm exit) in vacuum it adds 3.9 N·s in the 0.14 s after the NFPA burn ends, which delivers
  0.45 N·s itself; that is 0.95% of total impulse.

## Delays

[TC-G] lists every achievable delay, adjustable ones included. Plugged motors are `P`. See
`hpr_motor::delay` and `docs/format/eng.md` for the markers files use. A `0` is read as its own
"zero or plugged" setting, because the RASP spec says it means ejection at burnout but most files
mean plugged; it never becomes an ejection event without a decision.

## Validation

- **Catalog** (`catalog::tests`): all 32 bundled curves are within 1% of ThrustCurve's total
  impulse, average thrust and burn time. **This is the bundling rule**, so it holds by
  construction; the test guards the bundle against drift and mis-sourced curves
  ([Loft lesson L42][lessons]). Of 554 public-domain curves, 196 pass
  (`docs/research/thrustcurve-data.md`, which also lists the four motors whose stored values are
  printed coarser than 1%). Each bundled file is public domain and byte for byte its recorded
  SHA-256 ([Loft lessons L41–L43][lessons]).
- **ThrustCurve's code** (`catalog::tests::every_bundled_curve_matches_thrustcurve_statistics_code`):
  `validation/oracles/thrustcurve/analyze_stats.js` runs [TC-A] unchanged on every bundled curve
  (`validation/fixtures/motor/thrustcurve-analyze-stats.json`). hpr's impulse, burn window, burn
  time, average and peak thrust agree to 1.8e-15, so the definitions, not only the 1% rule, are
  checked.
- **RocketPy** (`motor::tests::matches_rocketpy_solid_motor_for_three_bundled_motors`): three
  bundled curves with BATES loads cover radial burnout, axial burnout, inhibited ends and both
  axis orientations. `validation/oracles/rocketpy/solid_motor.py` writes
  `validation/fixtures/motor/rocketpy-solid-motor.json`, and regenerates it byte for byte.
  - On 203 times per motor, total mass, `I_t` and `I_a` agree with RocketPy within 7.9e-5 of
    RocketPy's own values, and the centre of mass within 5.8e-6 of the motor length.
  - Quantities that go to zero are compared against a fixed scale: propellant mass and inertias
    against their ignition values, grain height against its initial height, centres against the
    motor length. On that scale they agree within 1e-4. Relative to their own tiny values in the
    last grams of propellant they differ by up to 39%, from RocketPy's interpolation between ODE
    knots and its early stop.
  - The test holds 0.1% on these scales.
- **Files:** 1710 of ThrustCurve's 1712 solid-motor files read and round-trip bit for bit (the other
  two have backwards times; `docs/format/`).
- **Unit and property tests:** exact impulse integration, burn windows, grain volume inversion, the
  parallel-axis theorem, and the impulse-fraction flow integrating to `m_p0`.

[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
