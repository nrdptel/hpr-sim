# Solid motors

## In short

- **What it models:** commercial solid motors: the thrust curve and its statistics (total
  impulse, burn time, average thrust, class), how the propellant burns away, the motor's mass,
  centre of mass and inertia as it burns, thrust at altitude, and ejection delays.
- **Sources:** NASA SP-8039 (1971), the National Association of Rocketry's *Standard Motor
  Codes*, ThrustCurve.org's glossary, statistics page and code, and RocketPy 1.13.0's motor code.
- **How well it is validated:** by unit tests and
  [code-to-code](../glossary.md#code-to-code-comparison), the first and third of four
  [kinds of evidence][levels]. ThrustCurve.org's own statistics code agrees to 1.8e-15 on all 32
  bundled curves. On three of them, RocketPy agrees within 7.9e-5 (relative) in total mass and
  inertias, with the centre of mass within 5.8e-6 motor lengths and the propellant's own
  quantities within 1e-4 of their values at ignition. Not compared with OpenRocket or a real
  flight.
- **What it leaves out:** anything but [commercial off-the-shelf](../glossary.md#cots-motor)
  solids. Only 32 curves are bundled, none in class A. Propellant burns in proportion to the
  impulse delivered, an approximation. With only catalog data, the centre of mass stays at
  mid-length. Commercial motor files give no nozzle exit size, so thrust at altitude goes
  uncorrected unless one is supplied.

## Using a motor

A flight needs a motor in the rocket's motor mount. There are two ways to get one:

- take one of the 32 motors that come with hpr-sim, or
- read a thrust-curve file, such as one downloaded from
  [ThrustCurve.org](../glossary.md#thrustcurveorg).

Either way, the motor then goes into one of the rocket design's configurations. A short program,
[`crates/hpr-sim/examples/motors.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/motors.rs),
shows all of this: it lists the built-in motors, reads a `.eng` file, and flies that motor in a
rocket. Run it from anywhere in the repository (the setup is on
[Getting started](../getting-started.md)):

```bash
cargo run --example motors -p hpr-sim
```

It prints this. The output is committed in
[`motors.output.txt`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/motors.output.txt),
and CI runs the program on macOS, Windows and Linux and fails if it prints anything else, so the
list below is always the list the code bundles.

<!-- quote: crates/hpr-sim/examples/motors.output.txt -->
```text
32 motors come built in:

                                           dia  length    loaded       total   average    burn
designation  maker     type        class    mm      mm    mass g impulse N·s  thrust N  time s
B4           Quest     single-use  B        18      81      19.8         4.9       4.4    1.11
C5           Estes     single-use  C        18      70      24.8         7.8       3.9    1.98
D5           Quest     single-use  D        20      96      44.1        17.6       3.8    4.59
E26W         AeroTech  single-use  E        24    87.7      44.0        27.6      23.5    1.18
26E31-15A    Cesaroni  reload      E        24      69      52.0        26.1      30.7    0.85
F52C         AeroTech  single-use  F        29   111.4      81.4        66.3      52.8    1.25
68F240-15A   Cesaroni  reload      F        24     133      91.8        68.0     236.6    0.29
F15          Estes     single-use  F        29     114     102.0        49.6      14.5    3.42
G69N         AeroTech  reload      G        38     106     201.0       136.3      72.1    1.89
131G84-10A   Cesaroni  reload      G        24     228     172.0       131.3      84.2    1.56
H170M        AeroTech  reload      H        38     191     330.0       318.0     165.4    1.92
168H54-10A   Cesaroni  reload      H        29     187     209.0       168.2      53.8    3.13
H125-CT      Loki      reload      H        38   177.8     342.0       241.7     125.0    1.93
I175WS       AeroTech  single-use  I        38     214     348.0       333.2     177.5    1.88
411I175-14A  Cesaroni  reload      I        38     245     437.5       411.4     174.1    2.36
I377-CT      Loki      reload      I        38   292.1     560.0       525.8     377.9    1.39
J450DM       AeroTech  single-use  J        54     359    1223.0      1061.6     465.6    2.28
1266J760-19A Cesaroni  reload      J        54     329    1076.8      1267.3     758.2    1.67
J300LR       Loki      reload      J        54     327    1315.0      1212.8     297.0    4.08
K400C        AeroTech  single-use  K        54     359    1194.0      1307.3     409.8    3.19
2245K1075-P  AMW       reload      K        54     728    2638.8      2255.1    1072.9    2.10
1633K940-18A Cesaroni  reload      K        54     404    1366.5      1636.1     936.9    1.75
L2500ST      AeroTech  reload      L        98     443    4989.0      4671.9    2501.7    1.87
3300L3200-P  Cesaroni  reload      L        75     486    3263.7      3299.6    3206.8    1.03
L1040LR      Loki      reload      L        54     736    2962.0      3722.5    1037.1    3.59
M1350W       AeroTech  single-use  M        75     622    4808.0      5179.5    1350.1    3.84
8187M1545-P  Cesaroni  reload      M        75    1025    7878.3      8181.8    1547.7    5.29
M1378LR      Loki      reload      M        54    1108    4331.0      5362.3    1375.6    3.90
N3300R       AeroTech  reload      N        98    1046   12054.0     14035.1    3189.9    4.40
13628N5600-P Cesaroni  reload      N        98    1010   11280.0     13633.5    5626.1    2.42
O6000W       AeroTech  single-use  O       152    1120   31520.0     39687.2    5828.8    6.81
21062O3400-P Cesaroni  reload      O        98    1239   16842.0     21041.0    3424.3    6.14

From the .eng file: I377CT by Loki, 38 mm by 292 mm, 560 g loaded, 250 g of propellant
  525.8 N·s (class I), 377.9 N average over 1.39 s, 657.0 N peak
  effective exhaust velocity 2103 m/s

Valetudo on the I377CT: 8.82 kg at liftoff
  leaves the 3 m rail at 17.7 m/s
  apogee 151.7 m above the pad, at 6.09 s
Not yet validated: see the Accuracy page before trusting these numbers.
```

The whole program is at the end of this page, in [The example program](#the-example-program).

### The bundled motors

**Licence:** each curve is a ThrustCurve.org file marked public domain, bundled unchanged; the
columns taken from the catalog are ThrustCurve.org's published values, used with attribution
([third-party notices](https://github.com/nrdptel/hpr-sim/blob/main/THIRD-PARTY-NOTICES.md)).

| column | what it is | where it comes from |
|---|---|---|
| designation | The motor's full name ([motor designation](../glossary.md#motor-designation)) | ThrustCurve.org's catalog |
| maker | The manufacturer, abbreviated | the catalog |
| type | `single-use`, or `reload`: a propellant load for a reusable case | the catalog |
| class | The [impulse class](../glossary.md#impulse-class) letter | hpr, from the curve |
| dia, length | The case's diameter and length, mm | the catalog |
| loaded mass | The motor ready to fly, g. For a reload this includes the case | the catalog |
| total impulse | [Total impulse](../glossary.md#total-impulse), N·s | hpr, from the curve |
| average thrust | [Average thrust](../glossary.md#average-thrust), N | hpr, from the curve |
| burn time | [Burn time](../glossary.md#burn-time), s | hpr, from the curve |

How the 32 were chosen:

- They are public-domain ThrustCurve.org curves of motors in production.
- Each curve's total impulse, burn time and average thrust are within 1% of the values
  ThrustCurve.org publishes for the motor, so the numbers hpr computes above agree with
  ThrustCurve.org's within 1%. A test checks it ([Validation](#validation)).
- There are up to three per impulse class, from different manufacturers. None is in class A.
- The selection, and the curves left out, are in
  [the ThrustCurve.org data notes](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/thrustcurve-data.md).

In code, `Catalog::bundled()` returns the list
([`Catalog`](../api/hpr_motor/catalog/struct.Catalog.html)). `catalog.find("I377")` finds a motor
by its designation or common name, ignoring case, spaces and hyphens. It returns every match:
`I175` finds both the AeroTech I175WS and the Cesaroni 411I175-14A. Then
`entry.bundled_motor()` builds the motor
([`CatalogMotor`](../api/hpr_motor/catalog/struct.CatalogMotor.html)), with the catalog's size and
masses and the [envelope default](#the-whole-motor) for where its mass sits.

### A motor from a file

Motor files come in two formats, and ThrustCurve.org serves both:

- [RASP `.eng`](../format/eng.md), a plain-text format named after RASP, the rocket simulation
  program it comes from. It is the more common of the two: 889 of the 1712 solid-motor files
  ThrustCurve.org held when surveyed.
- [RockSim `.rse`](../format/rse.md), an XML format from the RockSim simulator.

The example reads a `.eng` file in four steps:

1. **Read the text.** For a file you downloaded, use `std::fs::read_to_string("my-motor.eng")?`.
   hpr's motor crate never opens files itself. The example builds one of the bundled files into
   the program with `include_str!` instead, so that it runs anywhere.
2. **Parse it.** `eng::parse(&text)?` ([`eng::parse`](../api/hpr_motor/eng/fn.parse.html))
   returns the file's entries, one per motor (a file can hold several), and a list of warnings.
   The reader is lenient: it accepts the oddities real files have and reports each one, with its
   line number ([reader policy](../format/eng.md#reader-policy-lenient-with-diagnostics)). This
   file reads with no warnings.
3. **Take the curve.** `entry.thrust_curve()?` makes the [thrust curve](#thrust-curve), starting
   it from zero thrust at ignition.
4. **Build the motor.** `SolidMotor::from_envelope(curve, diameter_m, length_m, propellant_kg,
   loaded_kg)` ([`SolidMotor`](../api/hpr_motor/motor/struct.SolidMotor.html)). A `.eng` header
   gives the size in millimetres and the masses in kilograms, so the example multiplies the size
   by 0.001. A slip here is caught: a motor whose numbers imply an impossible exhaust velocity is
   refused ([the units check](#the-effective-exhaust-velocity-is-a-units-check)).

For a `.rse` file, use `rse::parse` ([`rse::parse`](../api/hpr_motor/rse/fn.parse.html)) instead.
Its motors are in `engines` rather than `entries`, and its masses are in grams
(`initial_mass_g`, `propellant_mass_g`), so multiply those by 0.001 too.

The file here is Loki Research's I377. The program prints what it read: 38 mm by 292 mm, 560 g
loaded with 250 g of propellant. From the curve it works out 525.8 N·s, an I motor, averaging
377.9 N over 1.39 s. Its effective exhaust velocity, 2103 m/s, is well inside the check's range.

Three things to know about a motor built from a file:

- **The header can be wrong.** Whoever made the file typed its size and masses. Of
  ThrustCurve.org's 554 public-domain files, 66 headers give a length more than 1 mm off the
  catalog's, and 5 a diameter more than 0.5 mm off
  ([data notes](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/thrustcurve-data.md),
  section 4). This file says 292 mm where the catalog says 292.1 mm. For one of the 32 motors
  above, `bundled_motor()` uses the catalog's values instead ([Loft lesson L43](../decisions-and-roadmap.md#l43): one
  header said 75 mm for a 54 mm motor). For any other motor, check the header against the
  motor's page on ThrustCurve.org.
- **Where its mass sits is a rough guess.** `from_envelope` spreads the propellant through the
  whole case, so the centre of mass stays at mid-length as it burns
  ([The whole motor](#the-whole-motor)). If you know the grain sizes, `SolidMotor::new` with
  `Propellant::Grains` does better
  ([Where the propellant is](#where-the-propellant-is)).
- **Its nozzle size is unknown**, so its thrust is not corrected for altitude
  ([Thrust at altitude](#thrust-at-altitude)).

### Putting it in a rocket

A rocket design keeps its motors in *configurations*: named sets of motors, at most one in each
motor mount ([Motors and configurations](design.md#motors-and-configurations)). The example adds
one to Valetudo, the rocket of [Getting started](../getting-started.md): a
[`Configuration`](../api/hpr_design/config/struct.Configuration.html) with the id `from-file`,
holding one [`MountedMotor`](../api/hpr_design/config/struct.MountedMotor.html).

| field of `MountedMotor` | what it holds |
|---|---|
| `mount` | The id of the mount in the design, here `motor-mount` |
| `designation` | A name, for display |
| `diameter_m`, `length_m` | The case's size, in metres. The design checks compare the diameter with the mount's bore: a motor wider than its mount is an error, and the flight refuses to start |
| `motor` | The motor built above |
| `delay` | The [ejection delay](../glossary.md#ejection-delay) chosen, if any. A parachute can fire on it, with the `MotorDelay` trigger ([Recovery](recovery.md#triggers-lag-and-release)) |

Valetudo's mount is 43 mm inside, so this 38 mm motor fits. `Simulation::new(&rocket,
"from-file", ...)` then flies the configuration by its id: straight up from a 3 m rail, in calm
air, with no parachutes. The rocket weighs 8.82 kg at liftoff, leaves the rail at 17.7 m/s, and
reaches 151.7 m above the pad at 6.09 s. These flight numbers are not validated; see
[Accuracy](../accuracy.md) before trusting them.

A design file holds configurations in the same form, as JSON, with the motor written out in full:
see the `configurations` list at the end of
[the Valetudo design](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/rocketpy-valetudo.json).
Every motor in a configuration lights at the same moment, `t = 0`. Staging and air starts (motors
lit in flight) are not modelled yet; they come with the staging milestone ([M1.9](../decisions-and-roadmap.md#m1-9)).

## Code and sources

Code: `hpr_motor` ([API reference](../api/hpr_motor/index.html)), in its modules `curve`, `class`,
`motor`, `grains`, `mass`, `delay` and `catalog`. The file formats are described in
[`.eng` files](../format/eng.md) and [`.rse` files](../format/rse.md).

Each source has a short key in square brackets, used below. A source that can be downloaded is
*pinned*: its address and a checksum of its bytes are in the
[reference lock file](https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml),
under the name given, so anyone can fetch the same copy ([Checking a claim](../checking-a-claim.md)).

- **[SP]** NASA SP-8039, *Solid Rocket Motor Performance Analysis and Prediction* (1971),
  [free from NASA][sp], pinned as `nasa-sp-8039`. Page notes:
  [nasa-sp-8039-motor-definitions.md](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/nasa-sp-8039-motor-definitions.md).
- **[NAR]** National Association of Rocketry, *Standard Motor Codes*, as
  [archived on 2014-02-05][nar], pinned as `nar-standard-motor-codes`.
- **[TC-G]** ThrustCurve.org's [glossary][tc-g], pinned as `thrustcurve-glossary`.
- **[TC-S]** ThrustCurve.org's ["Motor Statistics" page][tc-s], pinned as `thrustcurve-motorstats`.
- **[TC-A]** ThrustCurve.org's statistics code, [`simulate/analyze/analyze.js`][tc-a] in the
  site's source at commit `577afa6`, pinned as `thrustcurve3-analyze`. It is under the ISC
  licence, a permissive open-source licence like MIT, so hpr may read it and run it.
- **[RP]** RocketPy 1.13.0 (MIT), [`rocketpy/motors/motor.py`][rp-motor] and
  [`solid_motor.py`][rp-solid], pinned as `rocketpy`. Notes:
  [rocketpy-solid-motor.md](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/rocketpy-solid-motor.md).

Line numbers below, such as "lines 40–91", link to those lines of the source.

## Conventions

- **Time** `t` is seconds from ignition.
- **Motor axis:** positions are metres along the motor's axis **from the nozzle exit plane (where
  the exhaust leaves) toward the forward closure (the motor's front end)**, so `+z` points toward
  the nose like the body frame ([Frames](frames.md)). The design model (from [M1.4](../decisions-and-roadmap.md#m1-4), the
  design and mass-properties milestone) places the motor's nozzle exit in the body frame.
- **Inertia:** every part is symmetric about the axis. `I_a` is about the axis and `I_t` about a
  transverse axis, both through the part's own centre of mass (RocketPy's `I_33` and `I_11`).
- Motor files keep their own units (mm, kg or g). The model is SI: metres, kilograms and seconds.

## Thrust curve

A [thrust curve](../glossary.md#thrust-curve) is the motor's thrust against time, as its file
lists it.

- Samples `(t_i, F_i)` joined by straight lines, as [RP] reads them (`interpolation_method`
  `"linear"`). Before the first sample the curve starts from `(0, 0)` when `t_0 > 0`, as the RASP
  format specifies ([`.eng` files](../format/eng.md#thrust-curve-r-data-r-problems)) and [TC-A]
  integrates. From the last sample on `F = 0`, that instant included: like any step, the end
  takes the later value, so at burnout the thrust, the mass flow and the propellant left are all
  zero together.
- Equal consecutive times are a **step**: the later value holds from that time on. Real files use
  them for an abrupt burnout. Decreasing times and negative thrust are rejected, and so is a curve
  with no impulse or no burn time (defined below).
- **Where [TC-A] differs.** Before integrating, ThrustCurve's code drops leading points below
  500 µN and averages points closer than 50 µs ([lines 40–91][tc-a-40]); hpr keeps leading zeros
  and treats equal times as steps. The results are identical on every bundled curve. ThrustCurve.org
  held 1712 solid-motor files when surveyed. On the 1710 of them that read, the two agree to 1e-9
  except 17 with repeated times, where they differ by up to 1.1% in average thrust. hpr keeps the
  step because a vertical drop is what the file draws.
- **Total impulse:** the exact integral of the lines, `I = Σ ½ (F_i + F_{i+1}) (t_{i+1} − t_i)`
  ([SP] glossary p. 96: `I = ∫F dt`). `I(t)` is the same sum up to `t`, with the partial interval.
- **Burn time**, by the rule of [NFPA 1125](../glossary.md#nfpa-1125), the US National Fire
  Protection Association's code for making model and high-power rocket motors, as ThrustCurve.org
  applies it: from the moment the thrust first reaches 5% of peak to the moment it last falls to
  5% of peak, each crossing interpolated on its segment ([TC-G] "Burn Time"; [TC-S]; [TC-A]
  [lines 165–203][tc-a-165]). The last sample's time is not the burn time
  ([Loft lesson L39](../decisions-and-roadmap.md#l39): Loft took it as the burn time).
- **Average thrust:** total impulse over the NFPA 1125 burn time ([TC-G] "Average Thrust"; [TC-A]
  [line 231][tc-a-231]).
  - [TC-S] says instead "the total impulse during the 5%-defined burn time". The two differ by the
    impulse outside the window, a median 0.13% on ThrustCurve's public-domain curves
    ([data notes](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/thrustcurve-data.md)).
    hpr follows the glossary and the site's code.
- **Impulse class:** each letter covers twice the total impulse of the one before. `A` goes up to
  2.5 N·s, `B` to 5, `C` to 10, and so on: `upper(k) = 1.25 · 2^k N·s`, from `1/8A` (`k = −2`) to
  `O` (`k = 15`), and on to `Z` by doubling. **Upper limits are inclusive:** [NAR] states `C` as
  "5.01 to 10.0 N-sec". Loft gave `B` for 2.5 N·s ([Loft lesson L38](../decisions-and-roadmap.md#l38)).

## Propellant consumption

The [effective exhaust velocity](../glossary.md#effective-exhaust-velocity) `c` is the thrust
divided by the rate at which propellant mass leaves, `c = F/ṁ` ([SP] glossary p. 95). **Held
constant** through the burn, it makes the propellant burned proportional to the impulse delivered
so far: when half the impulse has been delivered, half the propellant is gone. With `I` the total
impulse, `I(t)` the impulse delivered by time `t`, and `m_p0` the propellant at ignition:

```text
c = I / m_p0,    ṁ(t) = F(t) / c,    m_p(t) = m_p0 (1 − I(t)/I)
```

- This is [RP] `SolidMotor` ([`solid_motor.py:401-418`][rp-401]; [`motor.py:483-524`][rp-483]).
  ThrustCurve's `.rse` files tabulate their mass column the same way
  ([`.rse` files](../format/rse.md)).
- **It is an approximation.** [SP] defines `c` only instantaneously. Measured
  [specific impulse](../glossary.md#specific-impulse) (`I_sp`, the same quantity divided by
  standard gravity) drifts during a burn, as the pressure inside the motor changes and the nozzle
  erodes ([SP] p. 14), so real consumption is not exactly proportional. No data published for
  commercial motors resolves the difference.
- All propellant is gone at the last sample. The whole burned mass leaves as exhaust. Inert
  material that also leaves (bits of liner and igniter) is not modelled.

## Where the propellant is

- **Column** (`Propellant::Column`): a hollow cylinder `(R, r, L)` (outer radius, inner radius,
  length) at a fixed centre. It keeps its shape and loses density as it burns, so
  `I_a = ½ m_p (R² + r²)` and `I_t = m_p ((R² + r²)/4 + L²/12)`. This is [RP] `GenericMotor`'s
  model ([`motor.py:1580-1648`][rp-1580], a solid cylinder).
- **BATES grains** (`Propellant::Grains`): `N` identical [BATES grains](../glossary.md#bates-grain)
  `(R, r₀, h₀)` (outer radius, starting bore radius, starting height), spaced `h₀ + s` apart, with
  a gap `s` between them. Each burns on its bore, the hole along its axis, and on both ends unless
  the ends are inhibited: coated so that they can't burn ([RP]
  [`solid_motor.py:487-632`][rp-487]).
  - Every grain needs a bore (`r₀ > 0`). A solid end burner, a grain with no hole that burns only
    on one face, shortens without widening, which this regression (the way the grain burns back)
    doesn't describe, and neither does RocketPy's.
  - Facing ends burn even with no gap, as in [RP].
  - Every burning surface recedes by the same depth `x`, the web burned so far (the web is the
    thickness of propellant between the burning surfaces):

  ```text
  V(x) = π (R² − (r₀ + x)²) (h₀ − 2x)     ends burning,   x ≤ min(R − r₀, h₀/2)
  V(x) = π (R² − (r₀ + x)²) h₀            ends inhibited, x ≤ R − r₀
  N ρ V(x) = m_p(t)
  ```

  Here `V` is one grain's volume and `ρ` the propellant's density, so the last line says the `N`
  grains hold the propellant left.

  - [RP] integrates `ṙ = −V̇/A_b`, `ḣ = −2ṙ` in time, with `A_b` the burning area, using LSODA: a
    general-purpose solver for ordinary differential equations (ODEs), from the Python library
    SciPy. Because `dV/dx = −A_b`, that ODE is the relation above. hpr solves it for `x` by
    safeguarded Newton iteration, exactly at any time. That is Newton's method, which improves a
    guess using the slope, kept inside an interval known to hold the answer: a step that would
    leave the interval halves it instead.
  - `m_p0 = N ρ V(0)` comes from the geometry, as in [RP].
  - About the stack's centre, which doesn't move:

    ```text
    I_a = ½ m_p (R² + r²)
    I_t = m_p ((R² + r²)/4 + h²/12) + (m_p/N) (h₀ + s)² N (N² − 1)/12
    ```

    The last term sums `m_g d_k²` over the grain offsets ([RP] [`solid_motor.py:724-740`][rp-724],
    [`784-789`][rp-784]).

## The whole motor

- **Dry mass** (case, closures, liner, nozzle: everything but the propellant) is one element:
  mass, centre and inertias about its own centre. `with_added_dry_mass` joins more hardware, such
  as a retainer (the cap or clip that holds the motor in its mount). It must be positive: at
  burnout it is the whole motor, and with no mass it has no centre.
- The motor is the dry mass and the propellant combined about the instantaneous centre with the
  [parallel-axis theorem](../glossary.md#parallel-axis-theorem): `z = Σ m_k z_k / m`,
  `I_a = Σ I_a,k`, `I_t = Σ (I_t,k + m_k (z_k − z)²)`. This is [RP] `Motor.I_11`
  ([`motor.py:585-666`][rp-585]).
- **Envelope default** (`SolidMotor::from_envelope`), when only the motor's size and masses are
  known, as from a catalog entry or a motor file's header:
  - the propellant is a solid column of radius `D/2` filling the length, centred at `L/2`;
  - the dry mass (loaded minus propellant) is a thin tube, `I_a = m r²` and
    `I_t = m (r²/2 + L²/12)`, also centred at `L/2`.

  **These are crude:** the centre of mass stays at `L/2` throughout. Loft fixed the CG at the
  midpoint with no inertia of its own ([Loft lesson L40](../decisions-and-roadmap.md#l40)); hpr gives the parts inertia,
  and moves the CG as soon as the dry and propellant centres differ.
  - ThrustCurve's loaded mass includes the reusable case ([TC-G] "Total Weight": "propellant and
    case").
  - [RP] `GenericMotor.load_from_eng` sets its chamber radius to the motor **diameter**
    ([`motor.py:1759-1761`][rp-1759]), which quadruples the `r²` inertia terms (and the default
    nozzle area it derives from that radius). hpr uses `D/2`.
- **Catalog envelope:** diameter, length and masses come from the catalog metadata, not the curve
  file's header, which can be wrong ([Loft lesson L43](../decisions-and-roadmap.md#l43): one header said 75 mm for a
  54 mm motor).

### The effective exhaust velocity is a units check

Both constructors, `SolidMotor::new` and `SolidMotor::from_envelope`, refuse a motor whose curve
and propellant mass imply an effective exhaust velocity `c = I/m_p` outside **200 to 5,000 m/s**.
It is a guard against a units slip, not a filter on propellant:

- **What it catches.** Sizes in millimetres and masses in grams, typed where metres and kilograms
  belong, which move `c` by a factor of 1,000. Nothing else in the API notices: the 411I175's envelope read that way,
  `from_envelope(curve, 38.0, 245.0, 228.9, 437.5)`, has positive, finite dimensions and a
  propellant mass below the loaded mass, and an exhaust velocity of 1.8 m/s.
- **What it lets through.** Every real motor checked. The 1,708 ThrustCurve.org files with a
  catalog propellant mass run from 236 to 3,031 m/s, with a median of 1,867 and 90% of them
  between 928 and 2,210. The 32 bundled motors run from **689.78 m/s** (a black-powder C) to
  **2,651.64 m/s** (a K).
- **Where it is tight.** The lowest real value is 1.2x above the floor. Estes and Quest normally
  count the delay grain and the ejection charge as propellant, so their small black-powder motors
  read low; [issue #11](https://github.com/nrdptel/hpr-sim/issues/11) records the fallback if one
  ever falls below 200 m/s.

`I` here is the curve's own impulse, uncorrected for air pressure. The reasoning in full, the
table of percentiles, and two figures that were once got wrong are in
[the exhaust-velocity note](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/exhaust-velocity-guard.md).

## Thrust at altitude

A motor pushes harder in thinner air. Its thrust has two parts: the momentum of the exhaust, and a
pressure term, the exhaust's pressure at the nozzle exit less the air's, times the exit area:
`F = ṁ u_e + (p_e − p_a) A_e` ([SP] eq. 2, p. 3; SP-8039's unit constant `g_c` is 1 in SI). Here
`ṁ` is the mass flow, `u_e` the exhaust speed at the exit, `p_e` the exit pressure, `p_a` the
ambient (air) pressure and `A_e` the exit area. With the flow unchanged, a curve measured at
reference pressure `p_ref` gives, at ambient `p_a`,

```text
F(p_a) = F_curve + (p_ref − p_a) A_e,    A_e = π r_e²
```

- [SP] doesn't print this conversion; it follows from eq. 2. [RP] `Motor.pressure_thrust`
  ([`motor.py:1173-1191`][rp-1173]) applies the same term.
- `p_ref` is the air pressure at the static test site, where the motor was fired on a test stand
  to measure its curve. It is stored with the nozzle (`Nozzle::reference_pressure_pa`). Motor
  files and catalogs don't record it, so standard sea-level pressure (101 325 Pa) is only a
  stand-in. For a test at 1500 m elevation (84.6 kPa in the 1976
  [standard atmosphere](../glossary.md#standard-atmosphere)), the stand-in makes the term
  16.8 kPa × `A_e` too large at every altitude.
- It holds while the exhaust fills the nozzle to its exit. A nozzle made for high altitude, tested
  at sea level, can have its flow come away from the nozzle wall (separate), and then it doesn't
  ([SP] pp. 32–34).
- hpr applies it strictly inside the burn, `0 < t < t_end` (`t_end` the curve's last time), as
  RocketPy's flight does ([`simulation/flight.py:1936-1956`][rp-flight]), but only where the
  curve's thrust is positive (RocketPy also adds it inside zero-thrust gaps, where nothing flows),
  and never lets thrust go negative. Without a known nozzle it returns the curve. Commercial motor
  files carry no exit diameter: `.eng` has no field for one, and the `.rse` format's `exitDia`
  attribute is always 0 ([`.rse` files](../format/rse.md#engine-attributes)).
- **Limits.** The full-flow term steps in just after ignition and steps to zero at `t_end` (the
  integrator should treat both as [events](../glossary.md#event)). In the ignition transient (the
  first moments, while the pressure inside the motor builds) and the tail-off (the end of the
  burn, while it falls), the real exit pressure is far from its full-flow value, so the term
  misstates thrust there. In the tail-off, where the exit pressure falls with the chamber's, it
  overstates it: on the 411I175 (a 9.5 mm exit) in vacuum it adds 3.9 N·s in the 0.14 s after the
  NFPA 1125 burn ends, which delivers 0.45 N·s itself; that is 0.95% of total impulse.

## Delays

A motor's [ejection delay](../glossary.md#ejection-delay) is the time from burnout to its ejection
charge, which deploys the recovery. [TC-G] lists every achievable delay, adjustable ones included.
A plugged motor has no ejection charge, and files mark it `P`. See
[`hpr_motor::delay`](../api/hpr_motor/delay/index.html) and [`.eng` files](../format/eng.md) for
the markers files use. A `0` is read as its own "zero or plugged" setting, because the RASP spec
says it means ejection at burnout but most files mean plugged. hpr never turns it into an
ejection event by itself: the user has to decide.

## Validation

- **Catalog** (`catalog::tests`): all 32 bundled curves are within 1% of ThrustCurve's total
  impulse, average thrust and burn time. **This is the bundling rule**, so it holds by
  construction; the test guards the bundle against drift and mis-sourced curves
  ([Loft lesson L42](../decisions-and-roadmap.md#l42): in Loft, a mis-sourced curve flew about 26% high). Of 554 public-domain
  curves, 196 pass
  ([data notes](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/thrustcurve-data.md),
  which also list the four motors whose stored values are printed coarser than 1%). Each bundled
  file is public domain, and its bytes match the SHA-256 checksum (a fingerprint of a file's exact
  bytes) recorded when it was downloaded (Loft lessons [L41](../decisions-and-roadmap.md#l41) to [L43](../decisions-and-roadmap.md#l43), on curve licences,
  loose impulse checks and wrong headers).
- **ThrustCurve's code** (`catalog::tests::every_bundled_curve_matches_thrustcurve_statistics_code`):
  [`validation/oracles/thrustcurve/analyze_stats.js`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/thrustcurve/analyze_stats.js)
  runs [TC-A] unchanged on every bundled curve, and writes its results to a
  [fixture](../glossary.md#reference-value-and-fixture),
  [`validation/fixtures/motor/thrustcurve-analyze-stats.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/motor/thrustcurve-analyze-stats.json).
  hpr's impulse, burn window, burn time, average and peak thrust agree to 1.8e-15, so the
  definitions, not only the 1% rule, are checked.
- **RocketPy** (`motor::tests::matches_rocketpy_solid_motor_for_three_bundled_motors`): three
  bundled curves with BATES loads cover radial burnout, axial burnout, inhibited ends and both
  axis orientations.
  [`validation/oracles/rocketpy/solid_motor.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/rocketpy/solid_motor.py)
  writes
  [`validation/fixtures/motor/rocketpy-solid-motor.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/motor/rocketpy-solid-motor.json),
  and regenerates it byte for byte.
  - On 203 times per motor, total mass, `I_t` and `I_a` agree with RocketPy within 7.9e-5 of
    RocketPy's own values, and the centre of mass within 5.8e-6 of the motor length.
  - Quantities that go to zero are compared against a fixed scale: propellant mass and inertias
    against their ignition values, grain height against its initial height, centres against the
    motor length. On that scale they agree within 1e-4. Relative to their own tiny values in the
    last grams of propellant they differ by up to 39%. That comes from RocketPy: it interpolates
    between the points its ODE solver stored, and its solver stops slightly early.
  - The test holds 0.1% on these scales.
- **Files:** 1710 of ThrustCurve's 1712 solid-motor files read, and write back out and read again
  with every value identical to the last bit. The other two have times that go backwards
  ([`.rse` files](../format/rse.md#checked-against-real-files)).
- **Unit and property tests** (property tests check a rule on many random inputs): exact impulse
  integration, burn windows, grain volume inversion, the parallel-axis theorem, and the
  impulse-fraction flow integrating to `m_p0`.

## The example program

This is the whole of
[`crates/hpr-sim/examples/motors.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/motors.rs),
line for line the file CI runs. [Using a motor](#using-a-motor) walks through what it does.

<!-- quote: crates/hpr-sim/examples/motors.rs -->
```rust
//! Motors: the thrust curves that come with hpr-sim, and a motor read from a RASP `.eng` file
//! like the ones ThrustCurve.org serves, put in a rocket's motor mount and flown.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example motors -p hpr-sim
//! ```
//!
//! The documentation site's *Solid motors* page (`docs/physics/motor.md`, "Using a motor") walks
//! through it. What it prints is kept next to it in `motors.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::{Configuration, MountedMotor, Rocket};
use hpr_motor::catalog::MotorType;
use hpr_motor::{Catalog, ImpulseClass, SolidMotor, eng};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation, Termination};

fn main() -> Result<(), Box<dyn Error>> {
    // 1. The motors that come built in. Size and loaded mass are ThrustCurve.org's catalog values;
    //    impulse, class, average thrust and burn time are worked out here from each motor's curve.
    let catalog = Catalog::bundled()?;
    println!("{} motors come built in:", catalog.motors.len());
    println!();
    let heading = [
        [
            "", "", "", "", "dia", "length", "loaded", "total", "average", "burn",
        ],
        [
            "designation",
            "maker",
            "type",
            "class",
            "mm",
            "mm",
            "mass g",
            "impulse N·s",
            "thrust N",
            "time s",
        ],
    ];
    for [a, b, c, d, e, f, g, h, i, j] in heading {
        println!("{a:<12} {b:<8}  {c:<10}  {d:<5} {e:>5} {f:>7} {g:>9} {h:>11} {i:>9} {j:>7}");
    }
    for entry in &catalog.motors {
        let motor = entry.bundled_motor()?;
        let curve = motor.curve();
        let kind = match entry.motor_type {
            MotorType::SingleUse => "single-use",
            MotorType::Reload => "reload",
            _ => "other",
        };
        let class = ImpulseClass::from_total_impulse(curve.total_impulse_ns())?;
        println!(
            "{:<12} {:<8}  {kind:<10}  {:<5} {:>5} {:>7} {:>9.1} {:>11.1} {:>9.1} {:>7.2}",
            entry.designation,
            entry.manufacturer_abbrev,
            class.label(),
            entry.diameter_mm,
            entry.length_mm,
            entry.total_mass_g.unwrap_or(f64::NAN),
            curve.total_impulse_ns(),
            curve.average_thrust_n(),
            curve.burn_time_s(),
        );
    }

    // 2. A motor from a file. This is one of the bundled files, built into the program so that it
    //    runs anywhere. For a file you downloaded, read its text instead with
    //    `let text = std::fs::read_to_string("path/to/your-motor.eng")?;`
    let text = include_str!("../../hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000863.eng");
    let parsed = eng::parse(text)?;
    for warning in &parsed.warnings {
        println!("warning, line {}: {}", warning.line, warning.message);
    }
    let [entry] = &parsed.value.entries[..] else {
        return Err("expected one motor in the file".into());
    };
    // The file gives the size in millimetres and the masses in kilograms; the motor takes metres
    // and kilograms.
    let diameter_m = entry.diameter_mm * 1e-3;
    let length_m = entry.length_mm * 1e-3;
    let motor = SolidMotor::from_envelope(
        entry.thrust_curve()?,
        diameter_m,
        length_m,
        entry.propellant_mass_kg,
        entry.total_mass_kg,
    )?;
    let curve = motor.curve();
    println!();
    println!(
        "From the .eng file: {} by {}, {} mm by {} mm, {:.0} g loaded, {:.0} g of propellant",
        entry.name,
        entry.manufacturer,
        entry.diameter_mm,
        entry.length_mm,
        entry.total_mass_kg * 1e3,
        entry.propellant_mass_kg * 1e3,
    );
    println!(
        "  {:.1} N·s (class {}), {:.1} N average over {:.2} s, {:.1} N peak",
        curve.total_impulse_ns(),
        ImpulseClass::from_total_impulse(curve.total_impulse_ns())?,
        curve.average_thrust_n(),
        curve.burn_time_s(),
        curve.peak_thrust_n(),
    );
    println!(
        "  effective exhaust velocity {:.0} m/s",
        motor.exhaust_velocity_m_s()
    );

    // 3. Into a rocket: a new configuration that puts the motor in the design's motor mount. The
    //    rocket is Valetudo, the rocket of `first_flight.rs`. Its mount is 43 mm inside, so this
    //    38 mm motor fits: the design checks compare the case diameter given here with the bore.
    let mut rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    rocket.configurations.push(Configuration {
        id: "from-file".to_owned(),
        name: format!("{} from its .eng file", entry.name),
        motors: vec![MountedMotor {
            mount: "motor-mount".to_owned(),
            designation: entry.name.clone(),
            diameter_m,
            length_m,
            motor,
            // No ejection delay is chosen: this flight carries no parachutes.
            delay: None,
        }],
    });

    // Fly it, straight up from a 3 m rail in calm air, with no parachutes.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let simulation = Simulation::new(
        &rocket,
        "from-file",
        Environment::standard(site)?,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?;
    let flight = simulation.run(&mut ())?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }
    let sample = |kind| {
        flight
            .event(kind)
            .map(|event| event.sample)
            .ok_or(format!("the flight has no {kind:?}"))
    };
    let (liftoff, rail_exit, apogee) = (
        sample(EventKind::Liftoff)?,
        sample(EventKind::RailExit)?,
        sample(EventKind::Apogee)?,
    );
    println!();
    println!(
        "Valetudo on the {}: {:.2} kg at liftoff",
        entry.name, liftoff.mass_kg
    );
    println!(
        "  leaves the 3 m rail at {:.1} m/s",
        rail_exit.cg_velocity_enu_m_s.length()
    );
    println!(
        "  apogee {:.1} m above the pad, at {:.2} s",
        apogee.height_above_ground_m, apogee.time_s,
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    Ok(())
}
```

[levels]: ../accuracy.md#four-kinds-of-evidence
[sp]: https://ntrs.nasa.gov/api/citations/19720011135/downloads/19720011135.pdf
[nar]: https://web.archive.org/web/20140205181530/http://www.nar.org/NARmotors.html
[tc-g]: https://www.thrustcurve.org/info/glossary.html
[tc-s]: https://www.thrustcurve.org/info/motorstats.html
[tc-a]: https://github.com/JohnCoker/thrustcurve3/blob/577afa62302f70c6b2ba04e97a39240638cd704b/simulate/analyze/analyze.js
[tc-a-40]: https://github.com/JohnCoker/thrustcurve3/blob/577afa62302f70c6b2ba04e97a39240638cd704b/simulate/analyze/analyze.js#L40-L91
[tc-a-165]: https://github.com/JohnCoker/thrustcurve3/blob/577afa62302f70c6b2ba04e97a39240638cd704b/simulate/analyze/analyze.js#L165-L203
[tc-a-231]: https://github.com/JohnCoker/thrustcurve3/blob/577afa62302f70c6b2ba04e97a39240638cd704b/simulate/analyze/analyze.js#L231
[rp-motor]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py
[rp-solid]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/solid_motor.py
[rp-401]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/solid_motor.py#L401-L418
[rp-483]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py#L483-L524
[rp-1580]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py#L1580-L1648
[rp-487]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/solid_motor.py#L487-L632
[rp-724]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/solid_motor.py#L724-L740
[rp-784]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/solid_motor.py#L784-L789
[rp-585]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py#L585-L666
[rp-1759]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py#L1759-L1761
[rp-1173]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/motors/motor.py#L1173-L1191
[rp-flight]: https://github.com/RocketPy-Team/RocketPy/blob/v1.13.0/rocketpy/simulation/flight.py#L1936-L1956
