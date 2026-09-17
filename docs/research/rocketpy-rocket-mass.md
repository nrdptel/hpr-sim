# RocketPy's rocket mass properties

RocketPy v1.13.0 (MIT), read from `refs/rocketpy`; paths below are relative to `rocketpy/`. This is
the reference for M1.4b's comparison ("mass, CG and inertia match RocketPy's example rockets where
RocketPy exposes them"). RocketPy's `Rocket` takes its own mass, CG and inertia as inputs and adds
the motor. So the comparison exercises hpr's composition with a placed `SolidMotor`, not its
geometry.

## Composition

- **Signs:**
  - Rocket: `+1` for `tail_to_nose`, `−1` for `nose_to_tail` (`rocket/rocket.py:312-316`).
  - Motor: `+1` for `nozzle_to_combustion_chamber`, `−1` for `combustion_chamber_to_nozzle`
    (`motors/motor.py:269-273`).
  - `s` is their product.
- **Placement.** `add_motor(motor, p)` (`rocket.py:1113-1125`) places the motor's coordinate
  origin, not its nozzle, at `p`. A motor point at `z_m` lands at `s z_m + p`, and that applies to
  the propellant centre, the motor centre, the dry centre and the nozzle alike.
- **Masses and centres.** `M` is the rocket's mass without the motor and `z_r` its centre:
  - `dry_mass = M + m_dry` (`rocket.py:536`)
  - `total_mass(t) = M + m_prop(t) + m_dry` (`rocket.py:516`)
  - `center_of_dry_mass_position = (z_r M + z_mcdm m_dry)/dry_mass`, a constant (`rocket.py:595-598`)
  - `center_of_mass(t) = (z_r M + z_mcm(t) m_motor(t))/total_mass(t)` (`rocket.py:574-577`)
- **Inertia reference point.** `I_11(t)` and `I_22(t)` are about the **centre of dry mass**
  `z_cdm`, not the instantaneous centre of mass (`rocket.py:855-878`, `:930-949`):
  - `dry_I_11 = I11_r + M (z_r − z_cdm)² + motor.dry_I_11 + m_dry (z_mcdm − z_cdm)²`
  - `I_11(t) = dry_I_11 + prop_I_11(t) + m_prop(t) (z_cdm − z_prop(t))²`
  - `I_33 = I33_r + motor.dry_I_33 + prop_I_33(t)`, with no offset term.
  - Products of inertia are plain sums, and zero unless a 6-element inertia is given.
  - To compare about the CG, shift back: `I_11,cg = I_11 − m(t) (z_cm − z_cdm)²`. The flight
    equations do this (`simulation/flight.py:2524-2525`).
- **`motor.I_11`**, about the motor's own instantaneous CG, is not what `Rocket` uses. The rocket
  uses the motor's dry and propellant parts separately.

## Example rockets with solid motors

Given as radius (m), mass without motor (kg), `I_11`/`I_33` (kg·m²), CG without motor (m), and
motor position (m). The rocket is `tail_to_nose` and the motor `nozzle_to_combustion_chamber`
unless noted.

| rocket (source) | inputs | motor |
|---|---|---|
| Calisto (`docs/notebooks/getting_started.ipynb`) | 0.0635, 14.426, 6.321/0.034, 0, **−1.255** (RocketPy's tests use **−1.373**, `tests/fixtures/rockets/rocket_fixtures.py:50`) | `cesaroni/Cesaroni_M1670.eng`, burn 3.9 s, dry 1.815 kg, dry I 0.125/0.002, dry CG 0.317; 5 BATES grains ρ 1815, R 0.033, r 0.015, h 0.12, gap 0.005, grains CG 0.397 |
| Bella Lui (`docs/examples/bella_lui_flight_sim.ipynb`) | 0.078, 18.226, 0.78267/0.064244, 0, −1.1356 | `aerotech/AeroTech_K828FJ.eng`, 3 grains; dry mass 0.001, so propellant only |
| NDRT 2020 (`docs/examples/ndrt_2020_flight_sim.ipynb`), `nose_to_tail`, motor `combustion_chamber_to_nozzle` | 0.1015, 18.998, 73.316/0.15982, 1.3, 3.391 | `cesaroni/Cesaroni_4895L1395-P.eng`, dry 1.848, dry I 0, 5 grains |
| Valetudo (`docs/examples/valetudo_flight_sim.ipynb`) | 0.04045, 8.257, 3.675/0.007, 0, −1.024 | `projeto-jupiter/keron_thrust_curve.csv`, 6 grains, dry 0.001 |
| Juno III (`docs/examples/juno3_flight_sim.ipynb`) | 0.0655, 24.05, 15.07/0.067, 0, 0 | `projeto-jupiter/mandioca_thrust_curve.csv` reshaped to 5.8 s and 8800 N·s, 5 grains; nozzle at −1.294, dry and grain CG −0.683, dry mass 1e-11 |
| Valkyrie (`docs/examples/valkyrie_flight_sim.ipynb`, values from `data/rockets/valkyrie/VLK.json`) | 0.055, 5.65, 1.806/0.0183, 0.844, 0 | `cesaroni/Cesaroni_1997K650-21A.eng`, dry 0.6449 at 0.246, dry I 0.0343/0.0008, 5 grains |

Every thrust file named exists under `refs/rocketpy/data/motors/`. Prometheus uses a
`GenericMotor`, which models the propellant as a solid cylinder, so it tests a different model.
Several other data-file rockets (Andromeda, Astra, Cavour, Genesis, Erebus 11, Lince, Camoes)
have zero dry mass and inertia, so they test only the propellant.

## Reproduced numbers (Calisto)

Running RocketPy 1.13.0 from `refs/venv` on 2026-09-17 with the getting-started inputs:

| motor at | t (s) | total mass (kg) | CG (m) | `I_11` about `z_cdm` | `I_33` | `I_11` about CG |
|---|---|---|---|---|---|---|
| −1.255 | 0 | 19.196912 | −0.220798 | 9.638151 | 0.037942 | 9.379959 |
| −1.255 | 3.9 | 16.241000 | −0.104825 | 7.864455 | 0.036000 | 7.864455 |
| −1.373 | 0 | 19.196912 | −0.250124 | 10.516648 | 0.037942 | 10.181595 |
| −1.373 | 3.9 | 16.241000 | −0.118012 | 8.243784 | 0.036000 | 8.243784 |

The −1.373 value at `t = 0`, 10.516648, matches RocketPy's own pinned test value, 10.516647727
(`tests/unit/rocket/test_rocket.py:492`).

## Outcome (M1.4b)

- **Fixture.** `validation/oracles/rocketpy/rocket_mass.py` writes
  `validation/fixtures/design/rocketpy-rocket-mass.json`, with eight cases: all six rockets above,
  Calisto at both motor positions, and Prometheus. Prometheus's `GenericMotor` is hpr's
  propellant column: a fixed centre, solid-cylinder inertia, and impulse-fraction consumption
  (`motors/motor.py:470-524`, `:1566-1661`).
- **Curves.** RocketPy's motor data files carry their own terms, so each case keeps the example's
  inputs but uses the bundled public-domain curve nearest in impulse (ADR-007). Nothing at `t = 0`
  depends on the curve. With `--example-curves` the script reproduces the table above exactly;
  that output stays under `refs/`.
- **Calisto's tests rocket** has no geometry in RocketPy's tests, so its design takes the
  `calisto_robust` fixture's surfaces, which sit about 0.118 m aft of the notebook's.
- **Agreement** (`hpr_design::config::tests::matches_rocketpy_example_rockets`):
  - Dry scalars match to 2e-16, and the products of inertia are exactly zero.
  - Through the burn: total mass to 1.4e-5, centre to 3.9e-6 of the length, `I_11` to 2.6e-5
    and `I_33` to 1.4e-5.
  - The residual is RocketPy's resampling. `SolidMotor` grain volumes are interpolated
    linearly between LSODA knots (`motors/solid_motor.py:375-383`, `:603-630`), and
    `GenericMotor` inertias are sampled at thrust knots.
