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
  design model (M1.4) places the motor's nozzle exit in the body frame.
- **Inertia:** every part is symmetric about the axis. `I_a` is about the axis and `I_t` about a
  transverse axis, both through the part's own centre of mass (RocketPy's `I_33` and `I_11`).
- Motor files keep their own units (mm, kg or g). The model is SI.

## Thrust curve

- Samples `(t_i, F_i)` joined by straight lines, as [RP] reads them (`interpolation_method`
  `"linear"`). Before the first sample the curve starts from `(0, 0)` when `t_0 > 0`, as the RASP
  format specifies and [TC-A] integrates. After the last sample `F = 0`.
- Equal consecutive times are a **step**: the later value holds from that time on. Real files use
  them for an abrupt burnout. Decreasing times and negative thrust are rejected.
- **Total impulse:** the exact integral of the lines, `I = Σ ½ (F_i + F_{i+1}) (t_{i+1} − t_i)`
  ([SP] glossary p. 96: `I = ∫F dt`). `I(t)` is the same sum up to `t`, with the partial interval.
- **Burn time (NFPA 1125):** from the moment the thrust first reaches 5% of peak to the moment it
  last falls to 5% of peak, each crossing interpolated on its segment ([TC-G] "Burn Time";
  [TC-S]; [TC-A] lines 165–203). The last sample's time is not the burn time (Loft lesson L39).
- **Average thrust:** total impulse over the NFPA burn time ([TC-G] "Average Thrust"; [TC-A]
  line 231).
  - [TC-S] says instead "the total impulse during the 5%-defined burn time". The two differ by the
    impulse outside the window, a median 0.13% on ThrustCurve's public-domain curves
    (`docs/research/thrustcurve-data.md`). hpr follows the glossary and the site's code.
- **Impulse class:** `upper(k) = 1.25 · 2^k N·s`, from `1/8A` (`k = −2`) to `O` (`k = 15`), and on
  to `Z` by doubling. **Upper limits are inclusive:** [NAR] states `C` as "5.01 to 10.0 N-sec".
  Loft gave `B` for 2.5 N·s (lesson L38).

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
  own centre. `with_added_dry_mass` joins more hardware, such as a retainer.
- The motor is the dry mass and the propellant combined about the instantaneous centre with the
  parallel-axis theorem: `z = Σ m_k z_k / m`, `I_a = Σ I_a,k`,
  `I_t = Σ (I_t,k + m_k (z_k − z)²)`. This is [RP] `Motor.I_11` (`motor.py:585-666`).
- **Envelope default** (`SolidMotor::from_envelope`), when only a catalog entry is known:
  - the propellant is a solid column of radius `D/2` filling the length, centred at `L/2`;
  - the dry mass (loaded minus propellant) is a thin tube, `I_a = m r²` and
    `I_t = m (r²/2 + L²/12)`, also centred at `L/2`.

  **These are crude:** the centre of mass stays at `L/2` throughout. Loft fixed the CG at the
  midpoint with no inertia of its own (lesson L40); hpr gives the parts inertia, and moves the CG
  as soon as the dry and propellant centres differ.
  - ThrustCurve's loaded mass includes the reusable case ([TC-G] "Total Weight": "propellant and
    case").
  - [RP] `GenericMotor.load_from_eng` sets its chamber radius to the motor **diameter**
    (`motor.py:1759-1761`), which doubles the radial terms. hpr uses `D/2`.
- **Catalog envelope:** diameter, length and masses come from the catalog metadata, not the curve
  file's header, which can be wrong (Loft lesson L43).

## Thrust at altitude

The thrust equation is `F = ṁ u_e + (p_e − p_a) A_e` ([SP] eq. 2, p. 3, with `g_c = 1` in SI).
With the flow unchanged, a curve measured at reference pressure `p_ref` gives, at ambient `p_a`,

```text
F(p_a) = F_curve + (p_ref − p_a) A_e,    A_e = π r_e²
```

- [SP] doesn't print this conversion; it follows from eq. 2. [RP] `Motor.pressure_thrust`
  (`motor.py:1173-1191`) applies the same term.
- It holds while the nozzle flows full. Sea-level tests of altitude nozzles can separate
  ([SP] pp. 32–34).
- hpr applies it only while the curve's thrust is positive, and never lets thrust go negative.
  Without a known nozzle it returns the curve. COTS files carry no exit diameter (`.rse`
  `exitDia` is always 0).

## Delays

[TC-G] lists every achievable delay, adjustable ones included. Plugged motors are `P`. See
`hpr_motor::delay` and `docs/format/eng.md` for the markers files use.

## Validation

- **Catalog** (`catalog::tests`): all 32 bundled curves are within 1% of ThrustCurve's total
  impulse, average thrust and burn time. Each is public domain, byte for byte its recorded SHA-256
  (lessons L41–L43).
- **RocketPy** (`motor::tests::matches_rocketpy_solid_motor_for_three_bundled_motors`): three
  bundled curves with BATES loads cover radial burnout, axial burnout, inhibited ends and both
  axis orientations. `validation/oracles/rocketpy/solid_motor.py` writes
  `validation/fixtures/motor/rocketpy-solid-motor.json`.
  - Every quantity is within 1e-4 of RocketPy on 203 times per motor: total and propellant mass,
    centres, both inertias, bore and height.
  - The residual is RocketPy's ODE tolerance and knot interpolation. The test holds 0.1%.
- **Files:** 1710 of ThrustCurve's 1712 solid-motor files read and round-trip bit for bit (the other
  two have backwards times; `docs/format/`).
- **Unit and property tests:** exact impulse integration, burn windows, grain volume inversion, the
  parallel-axis theorem, and the impulse-fraction flow integrating to `m_p0`.
