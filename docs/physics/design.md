# The design tree, configurations and checks

Code: `hpr_design::tree` (the tree, placement, automatic radii, overrides, reference diameter),
`hpr_design::config` (motor mounts, configurations, assembly) and `hpr_design::checks`. Decisions:
ADR-007. Part geometry and mass are in `mass.md` and `shapes.md`.

Sources:

- **[RP]** RocketPy v1.13.0 (MIT), `rocketpy/rocket/rocket.py`: how a rocket's mass, centre of
  mass and inertia combine with a placed motor. `docs/research/rocketpy-rocket-mass.md` has the
  formulas with line numbers.
- **[MK]** Meriam and Kraige, *Engineering Mechanics: Dynamics*, appendix B: the parallel-axis
  theorem (`mass.md`).

Most of this file defines conventions rather than physical models. OpenRocket has its own
conventions for positions, automatic radii and overrides. The clean-room rule rules out its
source, so M3.1 (import) and M2.2 (the jar as an oracle) map them by running the jar.

## Stations and the body origin

- A **station** `s` is a distance aft of the nose tip, the way design files give positions.
- The body frame's origin is the nose tip, on the axis: `z_ref = 0` in `frames.md`. Station `s` is
  body `z = −s`, and the rocket lies at `z ≤ 0`.
- A part's own frame has its origin at its forward end (`mass.md`). A part placed at station `s` is
  translated by `(0, 0, −s)`. Radial offsets and roll angles stay as the part states them, always
  measured from the body axis.

## The tree

A `Rocket` has stages. Each `Stage` lists **body components**, and each `Component` holds a
`Part` and its children.

| role | parts | where |
|---|---|---|
| body | nose cone, body tube, transition | a stage's list; they stack |
| external | fin set, tube fin set, launch lug, rail button | children of a body tube; they take its outer radius |
| internal | inner tube, centering ring, mass component, parachute, streamer, shock cord | children of a body component or an inner tube |

- **Stacking.** Body components start at `s = 0` and follow one another through every stage, forward
  to aft. Each one's extent is its length without shoulders.
- **Axial extent** of an attached part:
  - a fin set's root chord;
  - a row of lugs or buttons from the first one's forward end to the last one's aft end,
    `(n − 1)·spacing + length` (for a button, its diameter);
  - a packed part's packed length;
  - otherwise the part's length.
- **Refused trees** (`DesignError::Tree`): no stages, an empty stage, a part in the wrong role, an
  attached part without a position, a body component with one, children under anything but a body
  component or inner tube, and motor mounts on anything but a body tube or inner tube.
  `DesignError::DuplicateId` covers an empty or repeated id.
- Fins on a nose cone or transition are refused for now. Their root would follow a curved or
  sloped surface, which `FinSet` doesn't model.

## Positions

An attached part of extent `L` in a parent spanning stations `[p, p + P]`, with offset `a`
(positive aft):

| `from` | forward end at |
|---|---|
| `top` | `p + a` |
| `middle` | `p + (P − L)/2 + a` |
| `bottom` | `p + P − L + a` |
| `after` | the previous sibling's aft end `+ a`, or `p + a` for the first child |
| `absolute` | `station_m` |

## Automatic dimensions

An `auto` list names dimensions that the tree resolves. The part's stored value for them is ignored.

- **Body radii** follow neighbours through every stage, stage boundaries included:
  - a nose cone's base, and a transition's aft radius, take the next component's forward radius;
  - a body tube, and a transition's forward radius, take the previous component's aft radius.
- **Order.** Sources are followed until nothing changes. A body tube still unresolved then takes
  the next component's forward radius instead: only the first such tube, and then the sweep
  repeats. So a fixed radius forward of a tube wins over one aft of it. A radius with no fixed
  radius to reach is refused.
- **Shoulders** take the inner radius (`R − t`) of the adjoining body tube: behind a nose; ahead
  of a transition for its forward shoulder, behind it for its aft one.
- **Centering rings:** the outer radius is the parent's inner radius. The inner radius is the
  outer radius of the widest on-axis inner tube among its siblings that overlaps it along the axis
  (by a positive length). With none, it is zero: a bulkhead.
- **Packed parts** (mass components and recovery parts) take the parent tube's inner radius.

## Overrides

`Overrides` replace computed mass properties `(m, c, I)`, in this order:

1. **Mass** `m′`: `I′ = I m′/m`, same centre. The body keeps its shape. A body with `m = 0` becomes
   a point mass `m′` at `c`.
2. **Centre** `a` (`cg_aft_m`): `c′_z = −(s_fore + a)`, measured from the forward end of what is
   overridden. `cg_xy_m` sets `c′_x` and `c′_y`; without it they are kept. The tensor about the
   centre is unchanged.
3. **Inertia**: the tensor about the centre is replaced. `InertiaOverride` gives its six entries
   with the sign convention of `mass.md` (`I_xy = −∫ x y dm`); the off-diagonal ones default to zero.

The result must pass `MassProperties::validate`.

- **Scope.** A component's overrides cover the component alone. With
  `overrides_include_children`, they cover the component and everything attached to it. A stage's
  overrides cover the whole stage, measured from its forward end. Motors are never covered.
- **Precedence.** Deeper overrides apply first. A child's override is inside its parent's subtree
  total, and the stage override applies last.
- Scaling the tensor with the mass keeps the radii of gyration. That is the natural reading of
  "this part weighs more than its geometry says", but other tools may differ. How OpenRocket orders
  its overrides on parts with shoulders (Loft lesson L51) is measured in M2.2.

## Reference diameter

- `maximum` (the default): twice the largest outer radius of any body component in any stage,
  including a bulged ogive's peak (`Profile::max_radius_m`). Internal parts, shoulders, fins, tube
  fins, lugs and rail buttons never count (Loft lesson L47, where an internal part could set it).
- `nose_base`: the first nose cone's base diameter.
- `custom`: a given diameter.

The reference area is `π d²/4`.

## Motors and configurations

- **Mounts.** A `MotorMount` on a body tube or inner tube holds a motor. Its `overhang_m` is how far
  the nozzle exit sits aft of the mount's aft end.
- **Configurations.** A `Configuration` puts at most one `MountedMotor` in each mount. A mounted
  motor is a `SolidMotor` with its case diameter and length (for the checks) and an optional
  delay.
- **Placement.** The motor's axis runs forward from the nozzle exit (`motor.md`). The nozzle exit
  is at station `s_aft + overhang`, on the mount's axis: the inner tube's
  `(r cos θ, r sin θ)`, or the body axis. A motor element at `z_m` is at body
  `z = −(s_aft + overhang) + z_m`.
- **Composition** at `t` seconds after ignition: `Assembly::mass_properties(t)` combines the
  structure with each motor's `SolidMotor::state(t).total` ([MK]). The dry assembly uses each
  motor's dry element. Every motor ignites at `t = 0`; staging, delays between stages and air
  starts come with M1.9.
- **Against RocketPy** [RP]: `total_mass(t)` and `center_of_mass(t)` are the same combination. Its
  `I_11(t)` is taken about the centre of dry mass, so hpr's tensor is moved there before
  comparing. `I_33` sums the axial moments (every element is on the axis).

## Checks

`checks::check` resolves a design and returns typed `Finding`s. Lengths compare with 1 nm of slack
(`LENGTH_TOLERANCE_M`), so round-off never raises one.

| finding | severity | when |
|---|---|---|
| `motor_wider_than_mount` | error | case diameter > mount inner diameter (L50) |
| `attachment_off_body` | error | an external part's extent (a fin root) doesn't overlap its body tube at all (L50) |
| `internal_part_wider_than_parent` | error | an internal part reaches past the parent tube's bore |
| `motor_past_mount_top` | warning | the case's forward end is forward of the mount's |
| `attachment_past_body_end` | warning | an external part runs past an end of its body tube |
| `internal_part_past_parent_end` | warning | an internal part runs past an end of its parent |
| `radius_step` | warning | adjacent body components' radii differ where they meet |
| `no_nose_cone` | warning | the first body component isn't a nose cone |

Errors mark designs that can't exist as described. A simulation of one would be wrong, usually
on the flattering side: Loft flew a 54 mm motor in a 38 mm mount 69% high. The flight engine
(M1.6) must refuse them unless the caller explicitly accepts them.

## Verification

- **Placement and resolution** (`tree::tests`): each position rule at stations worked by hand;
  body components stacking through two stages; automatic radii across a stage boundary, the
  fallback, precedence and the unresolvable case; ring, shoulder and packed radii; every refused
  tree; a JSON round trip that refuses unknown fields.
- **Composition by hand:**
  - `tree_structure_matches_parts_placed_by_hand`: the sample rocket's structure equals its eight
    parts placed at hand-worked stations and combined, to 1e-13.
  - `config::tests`: the placed motor's nozzle station, and the rocket's mass, centre and inertia
    at loaded, burning and burnt out, by the parallel-axis theorem, to 1e-12; an off-axis mount's
    `I_yz = −m y z`.
- **Overrides** (`overrides_rescale_move_and_replace`): each step, the scopes, a stage override,
  the massless case, and refusal of unphysical results.
- **Property** (proptest): randomly placed masses sum to the structure's mass and centre, and
  sliding every part moves the centre rigidly without changing the tensor.
- **Against RocketPy 1.13.0** (`config::tests::matches_rocketpy_example_rockets`):
  - The eight example-rocket cases of `validation/fixtures/design/rocketpy-rocket-mass.json` are
    compared at 103 times each. The fixture's generator and its curve substitution are described
    in `docs/research/rocketpy-rocket-mass.md`.
  - The test derives the stage override, nozzle station and motor inputs from the fixture itself,
    independently of the design generator.
  - Worst measured, with the test's tolerance:

    | quantity | worst | tolerance |
    |---|---|---|
    | dry mass, centre, `I_11`, `I_33`; column propellant mass | 2.4e-16 | 1e-12 |
    | products of inertia (of `I_11`) | 0 | 1e-15 |
    | total mass | 1.4e-5 | 1e-4 |
    | centre of mass (of the rocket's length) | 3.9e-6 | 1e-4 |
    | `I_11` about the dry centre, and about the centre of mass | 2.6e-5 | 1e-4 |
    | `I_33` | 1.4e-5 | 1e-4 |
    | grain propellant mass (of initial) | 9.7e-5 | 5e-4 |

  - The residual is RocketPy's own resampling: grain volumes between LSODA knots, and
    `GenericMotor` inertias at thrust knots. hpr's values are exact for a piecewise-linear curve.
- **Public designs** (`validation/designs/`, written by `cargo xtask designs`, which a test keeps in
  sync): the eight RocketPy examples and two synthetic rockets all resolve, pass the checks without
  errors, and assemble into valid bodies. Valkyrie's one warning is real: its fins run 22 mm onto
  the boattail, as in RocketPy's example.
- **Lessons:** L47 `tests::reference_diameter_ignores_internal_components`; L50
  `checks::tests::motor_wider_than_mount_is_rejected` and `checks::tests::fin_root_must_touch_body`.
