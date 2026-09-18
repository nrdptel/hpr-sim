# The design tree, configurations and checks

## In short

- **What it models:** how parts become a rocket: where each part sits, automatic radii,
  overrides, the reference diameter and the motor in its mount. It gives the rocket's mass,
  centre of mass and inertia through the burn, and flags designs that can't exist, such as a
  motor wider than its mount. Most of it is convention, not physics.
- **Sources:** RocketPy 1.13.0's `Rocket` code, and Meriam and Kraige's *Engineering Mechanics:
  Dynamics* for the parallel-axis theorem.
- **How well it is validated:** by analytic tests and code-to-code, the first and third of four
  [kinds of evidence][levels]. A hand-worked rocket agrees to 1e-12 through the burn. For eight
  cases of RocketPy's example rockets, a given structure with its motor placed agrees in mass,
  centre and inertia within 8.0e-10 (relative) at the times RocketPy computed, and within 1.1e-5
  in mass and 2.6e-5 in inertia between them; grain propellant mass within 2.4e-9 and 4.9e-5.
  Placement, automatic radii and overrides are checked by hand only. Not compared with OpenRocket
  or a real flight.
- **What it leaves out:** all motors ignite together at `t = 0` until staging arrives
  ([M1.9][roadmap]). Fins on a nose cone or transition are refused. OpenRocket has its own
  conventions for positions, radii and overrides; the OpenRocket comparison ([M2.2][roadmap]) will
  map them.

## Code and sources

Code: `hpr_design::tree` (the tree, placement, automatic radii, overrides, reference diameter),
`hpr_design::config` (motor mounts, configurations, assembly) and `hpr_design::checks`. Decisions:
[ADR-007][adr-007] (stations, placement, automatic radii, overrides, motors and checks). Part
geometry and mass are in [Mass properties](mass.md) and [Shapes](shapes.md).

Sources:

- **[RP]** RocketPy v1.13.0 (MIT), `rocketpy/rocket/rocket.py`: how a rocket's mass, centre of
  mass and inertia combine with a placed motor. `docs/research/rocketpy-rocket-mass.md` has the
  formulas with line numbers.
- **[MK]** Meriam and Kraige, *Engineering Mechanics: Dynamics*, appendix B: the parallel-axis
  theorem ([Mass properties](mass.md)).

Most of this file defines conventions rather than physical models. OpenRocket has its own
conventions for positions, automatic radii and overrides. The clean-room rule rules out its
source, so the planned OpenRocket import ([M3.1][roadmap]) and comparison ([M2.2][roadmap]) will
map them by running OpenRocket itself.

## Stations and the body origin

- A **station** `s` is a distance aft of the nose tip, the way design files give positions.
- The body frame's origin is the nose tip, on the axis: `z_ref = 0` in [Frames](frames.md). Station `s` is
  body `z = −s`, and the rocket lies at `z ≤ 0`.
- A part's own frame has its origin at its forward end ([Mass properties](mass.md)). A part placed at station `s` is
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
- **Packed parts** (mass components and recovery parts) take the parent tube's inner radius, less
  the distance from the parent's axis to the part's axis. An offset outside the bore is refused.

## Overrides

`Overrides` replace computed mass properties `(m, c, I)`, in this order:

1. **Mass** `m′`: `I′ = I m′/m`, same centre. The body keeps its shape. A body with `m = 0` becomes
   a point mass `m′` at `c`.
2. **Centre** `a` (`cg_aft_m`): `c′_z = −(s_fore + a)`, measured from the component's own forward
   end (a stage's for a stage, and never a shoulder's), whether or not the children are covered.
   `cg_xy_m` sets `c′_x` and `c′_y`; without it they are kept. The tensor about the centre is
   unchanged.
3. **Inertia**: the tensor about the centre is replaced. `InertiaOverride` gives its six entries
   with the sign convention of [Mass properties](mass.md) (`I_xy = −∫ x y dm`); the off-diagonal ones default to zero.

The result must pass `MassProperties::validate`, which also refuses inertia on a body with no mass.
Errors inside a stage or component name it (`DesignError::InComponent`).

- **Scope.** A component's overrides cover the component alone. With
  `overrides_include_children`, they cover the component and everything attached to it. A stage's
  overrides cover the whole stage, measured from its forward end. Motors are never covered.
- **Precedence.** Deeper overrides apply first. A child's override is inside its parent's subtree
  total, and the stage override applies last.
- Scaling the tensor with the mass keeps the radii of gyration. That is the natural reading of
  "this part weighs more than its geometry says", but other tools may differ. How OpenRocket orders
  its overrides on parts with shoulders ([Loft lesson L51][lessons]) will be measured by the planned
  OpenRocket oracle ([M2.2][roadmap]).

## Reference diameter

- `maximum` (the default): twice the largest outer radius of any body component in any stage,
  including a bulged ogive's peak (`Profile::max_radius_m`). Internal parts, shoulders, fins, tube
  fins, lugs and rail buttons never count. In Loft an internal part could set it
  ([Loft lesson L47][lessons]).
- `nose_base`: the first nose cone's base diameter.
- `custom`: a given diameter.

The reference area is `π d²/4`.

## Motors and configurations

- **Mounts.** A `MotorMount` on a body tube or inner tube holds a motor. Its `overhang_m` is how far
  the nozzle exit sits aft of the mount's aft end.
- **Configurations.** A `Configuration` puts at most one `MountedMotor` in each mount. A mounted
  motor is a `SolidMotor` with its case diameter and length (for the checks) and an optional
  delay.
- **Placement.** The motor's axis runs forward from the nozzle exit ([Solid motors](motor.md)). The nozzle exit
  is at station `s_aft + overhang`, on the mount's axis: the inner tube's
  `(r cos θ, r sin θ)`, or the body axis. A motor element at `z_m` is at body
  `z = −(s_aft + overhang) + z_m`.
- **Composition** at `t` seconds after ignition: `Assembly::mass_properties(t)` combines the
  structure with each motor's `SolidMotor::state(t).total` ([MK]). The dry assembly uses each
  motor's dry element. Every motor ignites at `t = 0`; staging, delays between stages and air
  starts will come with the planned staging milestone ([M1.9][roadmap]).
- **Against RocketPy** [RP]: `total_mass(t)` and `center_of_mass(t)` are the same combination. Its
  `I_11(t)` is taken about the centre of dry mass, so hpr's tensor is moved there before
  comparing. `I_33` sums the axial moments (every element is on the axis).

## Checks

`checks::check` resolves a design and returns typed `Finding`s. Lengths compare with 1 nm of slack
(`LENGTH_TOLERANCE_M`), so round-off never raises one.

| finding | severity | when |
|---|---|---|
| `motor_wider_than_mount` | error | case diameter > mount inner diameter ([Loft lesson L50][lessons]) |
| `motor_outside_mount` | error | the case doesn't overlap its mount along the axis at all (an overhang typed in mm as m) |
| `attachment_off_body` | error | an external part's extent (a fin root) doesn't overlap its body tube at all ([Loft lesson L50][lessons]) |
| `part_outside_rocket` | error | an internal part lies wholly forward of the nose tip or aft of the rocket's end, and touches none of the parts it hangs from |
| `internal_part_wider_than_parent` | error | an internal part reaches farther from its parent's axis than the parent's bore (a nose cone's or transition's largest outer radius) |
| `centre_outside_rocket` | error | a stage with an axial centre-of-mass override (`cg_aft_m`, its own or a component's) has its centre off the rocket although its parts aren't |
| `motor_past_mount_top` | warning | the case's forward end is forward of the mount's |
| `attachment_past_body_end` | warning | an external part runs past an end of its body tube |
| `internal_part_past_parent_end` | warning | an internal part runs past an end of its parent |
| `ring_overlaps_inner_tube` | warning | a centering ring crosses an inner tube beside it (a cluster's off-axis tubes), counting that mass twice |
| `radius_step` | warning | adjacent body components' radii differ where they meet |
| `no_nose_cone` | warning | the first body component isn't a nose cone |

- **Radial reach** is measured about the parent's own axis: the distance between the part's axis
  and the parent's, plus the part's radius. A block centred in an off-axis pod fits; a part on the
  body axis inside that pod doesn't. Centering rings have no offset, so they sit on the body axis.
- Parts wholly outside the rocket report only `part_outside_rocket`, and a stage holding one is
  spared `centre_outside_rocket`. A single check covers each fault.
- A retainer on a motor mount that sticks out past the airframe touches its mount, so it is on the
  rocket, even flush against the mount's end. Only the mount gets a warning. Touching an end face
  counts as touching, within `LENGTH_TOLERANCE_M`, for every part.
- Fins may sweep past the rocket's end, and a heavy part may run past its tube's, so a stage's
  centre can leave the rocket without an override, and neither a mass override nor a sideways
  centre override (`cg_xy_m`) can move a centre past its parts. It is an error only when an axial
  centre override (`cg_aft_m`) is involved; no component-level centre is checked.

Errors mark designs that can't exist as described. A simulation of one would be wrong, usually
on the flattering side: Loft flew a 54 mm motor in a 38 mm mount 69% high. The flight engine
refuses them with `SimError::DesignChecks` unless the caller sets
`FlightSettings::accept_design_errors`.

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
- **Overrides** (`overrides_rescale_move_and_replace`, `nested_overrides_apply_deepest_first`):
  each step, the scopes, a stage override, deeper overrides first, the massless case, and refusal
  of non-finite and unphysical results.
- **Property** (proptest): randomly placed masses sum to the structure's mass and centre, and
  sliding every part moves the centre rigidly without changing the tensor.
- **Checks** (`checks::tests`): each finding and its severity. A cluster pod's block fits and an
  on-axis part in the pod doesn't. Motors miss their mounts in both directions. Parts and a stage
  centre lie off the rocket. A layout with a corrupt parent index is skipped, not a panic.
- **Against RocketPy 1.13.0** (`config::tests::matches_rocketpy_example_rockets`):
  - Eight cases of `validation/fixtures/design/rocketpy-rocket-mass.json`: seven example rockets
    (Calisto at two motor positions) and Prometheus's `GenericMotor`. Cavour (added for its drag
    curve in [M1.5b][roadmap], the drag milestone) has no motor dry mass; its design gives the
    motor 1e-15 kg, since hpr needs a positive one.
    `docs/research/rocketpy-rocket-mass.md` gives the curve substitution and the examples left out.
  - The test derives the stage override, nozzle station and motor inputs from the fixture itself,
    independently of the design generator.
  - The rockets are compared at 103 even times through the burn and after it, and at up to 60 of
    RocketPy's LSODA knots.
  - Worst measured, with the test's tolerance:

    | quantity | worst | tolerance |
    |---|---|---|
    | dry mass, centre, `I_11`, `I_33`; initial and column propellant mass | 2.4e-16 | 1e-12 |
    | products of inertia (of `I_11`) | 0 | 1e-15 |
    | at LSODA knots: total mass, centre (of length), `I_11`, `I_33` | 8.0e-10 | 1e-8 |
    | at LSODA knots: grain propellant mass (of initial) | 2.4e-9 | 1e-8 |
    | even grid: total mass | 1.1e-5 (Cavour) | 5e-5 |
    | even grid: centre of mass (of the rocket's length) | 3.6e-6 (Cavour) | 2e-5 |
    | even grid: `I_11` about the dry centre and about the centre of mass | 2.6e-5 | 1e-4 |
    | even grid: `I_33` | 1.4e-5 | 1e-4 |
    | even grid: grain propellant mass (of initial) | 4.9e-5 | 2.5e-4 |

  - At RocketPy's knots, agreement is the ODE solver's own accuracy (rtol 1e-11).
  - Between knots, the residual is RocketPy's resampling: it interpolates grain volumes linearly
    between LSODA knots and samples `GenericMotor` inertias at thrust knots. hpr's values are
    exact for a piecewise-linear curve.
  - The comparison sets mass, centre and inertia together. So the override steps (rescaling the
    tensor with mass, moving the centre) are checked by hand-worked tests, not against RocketPy.
- **Public designs** (`validation/designs/`, written by `cargo xtask designs`, which a test keeps in
  sync): the eight RocketPy cases and two synthetic rockets resolve with no findings and assemble
  into valid bodies at ignition, mid-burn and burnout.
- **Lessons:** [Loft lesson L47][lessons] `tests::reference_diameter_ignores_internal_components`;
  [Loft lesson L50][lessons] `checks::tests::motor_wider_than_mount_is_rejected` and
  `checks::tests::fin_root_must_touch_body`.

[adr-007]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[levels]: ../accuracy.md#four-kinds-of-evidence
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
