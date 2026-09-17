# Public test designs

Rocket designs that tests and later milestones use in place of the private corpus. Each file is
the JSON of an `hpr_design::Rocket`. The format is provisional: M3.3 defines the open design format
and migrates these files.

`cargo xtask designs` writes every file here, and `cargo xtask designs --check` (run by a test)
fails if a committed file differs from what it writes. Don't edit them by hand.

## RocketPy examples (`rocketpy-*.json`)

These are built from `validation/fixtures/design/rocketpy-rocket-mass.json`, which
`validation/oracles/rocketpy/rocket_mass.py` generates by running RocketPy 1.13.0.

- **Mass:** the stage override is the example's mass, centre of mass (on the axis) and inertia
  without its motor, as RocketPy takes them.
- **Motor:** the example's grains or chamber, dry mass, nozzle and position, on hpr's motor axis.
  The thrust curve is the bundled public-domain curve nearest in impulse, because RocketPy's motor
  files carry their own terms (ADR-007). The configuration's name says which curve replaces which.
- **Case size:** the examples give none, so the case is the propellant's envelope. The mount is a
  tube that fits it exactly, in the last body tube forward of the nozzle.
- **Geometry**, for the aerodynamics milestones:
  - The example's nose, tails (as conical transitions), trapezoidal fins and rail buttons, at the
    example's positions.
  - Body tubes fill the gaps, and a closing tube reaches the nozzle or the fins when no tail
    does.
  - Walls, fin thickness and materials are placeholders; the override replaces their mass.
  - Parachutes are left out: RocketPy gives only their drag area.
- **Drag inputs** (ADR-009):
  - Fins with a NACA 00xx airfoil in the example get an airfoil section that thick at the mean
    aerodynamic chord. Juno III's fins take the rounded section its team published (a truncated
    airfoil). The rest keep square 3 mm edges; a lift-curve airfoil says nothing about the edges.
  - The rockets whose curves RocketPy labels RASAero (both Calisto designs, Juno III, Cavour,
    Valetudo) take RASAero II's default smooth finish (`mirror`), because the exports don't record
    theirs. The others keep the default finish.
- **Cavour** has no motor dry mass in RocketPy; hpr's motor needs a positive one, so it gets
  1e-15 kg.
- **Calisto at −1.373 m:** RocketPy's tests give no geometry for this rocket, so it takes the
  `calisto_robust` test fixture's surfaces.

`hpr_design::config::tests::matches_rocketpy_example_rockets` compares these designs' mass, centre
and inertia through the burn with RocketPy's (`docs/physics/design.md`).

## Synthetic rockets (`synthetic-*.json`)

Their mass comes from geometry, with built-in materials and bundled motors:

- `synthetic-54mm-three-fin.json`: a single-stage 54 mm rocket on a 411I175-14A.
- `synthetic-two-stage-75mm-54mm.json`: a 54 mm sustainer on a 411I175-14A over a 75 mm booster on
  a 1266J760-19A, joined by a transition with automatic radii. Both motors ignite at liftoff until
  staging arrives in M1.9.
