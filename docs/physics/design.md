# The design tree, configurations and checks

## In short

- **What it models:** how parts become a rocket: where each part sits, automatic radii (taken
  from the neighbouring parts), overrides (measured values that replace computed masses, centres
  and inertias), the [reference diameter](../glossary.md#reference-area) and the motor in its
  mount. It gives the
  rocket's mass, [centre of mass](../glossary.md#centre-of-gravity-cg) and inertia (its resistance
  to turning) through the burn, and flags designs that can't exist, such as a motor wider than its
  mount. Most of it is convention, not physics.
- **Sources:** RocketPy 1.13.0's `Rocket` code, and Meriam and Kraige's *Engineering Mechanics:
  Dynamics* for the [parallel-axis theorem](../glossary.md#parallel-axis-theorem).
- **How well it is validated:** by analytic tests and
  [code-to-code comparison](../glossary.md#code-to-code-comparison), the first and third of four
  [kinds of evidence][levels]. A hand-worked rocket agrees to 1e-12 through the burn. For eight
  cases of [RocketPy](../glossary.md#rocketpy)'s [example rockets](../glossary.md#example-rockets),
  a given structure with its motor placed agrees in mass, centre and inertia within 8.0e-10
  (relative) at the times RocketPy computed, and within 1.1e-5 in mass and 2.6e-5 in inertia
  between them; the propellant grains' mass within 2.4e-9 and 4.9e-5 of its initial value.
  Placement, automatic radii and overrides are checked by hand; the whole structure against
  OpenRocket on 71 compared designs, within 1% in mass on 58 and in centre of mass on 59
  ([mass properties](mass.md#checked-against-openrocket)); and body radii against OpenRocket in
  the `.ork` import ([`.ork` design files](../format/ork.md)). Not compared with a real flight.
- **What it leaves out:** staged flight. Every motor in a configuration ignites together at
  `t = 0`, on the pad. So a [cluster](../glossary.md#cluster) whose motors all light together is
  flown (no test or comparison checks one yet), but a two-stage design flies with every motor lit
  at once, which is not a staged flight, and nothing warns. Staging under power, delayed ignition
  and [air starts](../glossary.md#air-start) are planned for [M1.9](../decisions-and-roadmap.md#m1-9) (staging, clusters
  and air starts). Fins on a nose cone or transition are refused.
  [OpenRocket](../glossary.md#openrocket) has its own conventions for positions, radii and
  overrides; the OpenRocket comparison ([M2.2](../decisions-and-roadmap.md#m2-2)) is mapping them,
  and the mass conventions it has found are on the [mass page](mass.md#checked-against-openrocket).

## Code and sources

Code: [`hpr_design::tree`](../api/hpr_design/tree/index.html) (the tree, placement, automatic
radii, overrides, reference diameter), [`hpr_design::config`](../api/hpr_design/config/index.html)
(motor mounts, configurations, assembly) and [`hpr_design::checks`](../api/hpr_design/checks/index.html).
Decisions:
[ADR-007][adr-007] (stations, placement, automatic radii, overrides, motors and checks). Part
geometry and mass are in [Mass properties](mass.md) and [Shapes](shapes.md).

Sources:

- **[RP]** RocketPy v1.13.0 (MIT), `rocketpy/rocket/rocket.py`: how a rocket's mass, centre of
  mass and inertia combine with a placed motor.
  [`docs/research/rocketpy-rocket-mass.md`](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/rocketpy-rocket-mass.md)
  has the formulas with line numbers.
- **[MK]** Meriam and Kraige, *Engineering Mechanics: Dynamics*, appendix B: the parallel-axis
  theorem ([Mass properties](mass.md)).

Most of this file defines conventions rather than physical models. OpenRocket has its own
conventions for positions, automatic radii and overrides. hpr never reads OpenRocket's source
code, whose licence (GPL) is incompatible with hpr's (the clean-room rule). So the planned
OpenRocket import ([M3.1](../decisions-and-roadmap.md#m3-1), reading `.ork` files) and comparison ([M2.2](../decisions-and-roadmap.md#m2-2)) will
map them by running OpenRocket itself.

## Stations and the body origin

- A **[station](../glossary.md#station)** `s` is a distance aft of the nose tip, the way design
  files give positions.
- The [body frame](../glossary.md#body-frame)'s origin is the nose tip, on the axis: `z_ref = 0` in
  [Frames](frames.md). Its `z` axis points along the rocket toward the nose, so station `s` is body
  `z = −s`, and the rocket lies at `z ≤ 0`.
- A part's own frame has its origin at its forward end ([Mass properties](mass.md)). A part placed at station `s` is
  translated by `(0, 0, −s)`. Radial offsets and roll angles stay as the part states them, always
  measured from the body axis.

## The tree

A [`Rocket`](../api/hpr_design/tree/struct.Rocket.html) has stages: sections of the stack, forward
to aft. A [separation](../glossary.md#separation) splits the rocket at a boundary between two
stages, so a rocket that stays in one piece needs only one.
Each [`Stage`](../api/hpr_design/tree/struct.Stage.html) lists **body components**, and each
[`Component`](../api/hpr_design/tree/struct.Component.html) holds a
[`Part`](../api/hpr_design/tree/enum.Part.html) and its children.

| role | parts | where |
|---|---|---|
| body | nose cone, body tube, transition | a stage's list; they stack |
| external | fin set, tube fin set, launch lug, rail button | children of a body tube; they take its outer radius |
| internal | inner tube, centering ring, mass component, parachute, streamer, shock cord | children of a body component or an inner tube |

- **Stacking.** Body components start at `s = 0` and follow one another through every stage, forward
  to aft. Each one's extent is its length without shoulders (the sleeves of a nose or transition
  that slide into the next tube).
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
  [`Rocket::unresolvable_body_radii`](../api/hpr_design/tree/struct.Rocket.html#method.unresolvable_body_radii)
  lists exactly those radii. The `.ork` importer gives them OpenRocket's own default of 25 mm
  before laying the design out
  ([when an automatic radius has nothing to take](../format/ork.md#when-an-automatic-radius-has-nothing-to-take)).
  The tree itself never invents a radius.
- **Shoulders** take the inner radius (`R − t`, outer radius less wall thickness) of the adjoining
  body tube: behind a nose; ahead of a transition for its forward shoulder, behind it for its aft
  one.
- **Centering rings:** the outer radius is the parent's inner radius. The inner radius is the
  outer radius of the widest on-axis inner tube among its siblings that overlaps it along the axis
  (by a positive length). With none, it is zero: a bulkhead.
- **Packed parts** (mass components and recovery parts) take the parent tube's inner radius, less
  the distance from the parent's axis to the part's axis. An offset outside the bore is refused.

## Overrides

[`Overrides`](../api/hpr_design/tree/struct.Overrides.html) replace computed mass properties with
measured ones: the mass `m`, the centre of mass `c` and the inertia tensor `I` (the 3×3 table of
moments and products of inertia; see [Mass properties](mass.md)). Primes mark the new values.
They apply in this order:

1. **Mass** `m′`: `I′ = I m′/m`, same centre. The body keeps its shape. A body with `m = 0` becomes
   a point mass `m′` at `c`. A packed part (a mass component, parachute, streamer or shock cord)
   with `m = 0` instead becomes a solid cylinder of `m′` filling its packing, as OpenRocket's does
   ([Packed parts](mass.md#packed-parts)).
2. **Centre** `a` (`cg_aft_m`): `c′_z = −(s_fore + a)`, with `s_fore` the station of the
   component's own forward end (a stage's for a stage, and never a shoulder's), whether or not the
   children are covered.
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
- Scaling the tensor with the mass keeps the radii of gyration (`√(I/m)`: how far out, on
  average, the mass sits). That is the natural reading of
  "this part weighs more than its geometry says".
- OpenRocket differs in two ways, which hpr keeps as measured departures: it scales only the
  overriding part's own inertia, and when a mass override covers the parts inside and gives no
  centre, it puts the centre at the overriding part's own. Which override wins, and where a centre
  override is measured from, the two agree on
  ([Loft lesson L51](../decisions-and-roadmap.md#l51), measured on probe designs in
  [M2.2b1](../decisions-and-roadmap.md#m2-2b1); see
  [Mass properties](mass.md#what-a-ork-leaves-unsaid-and-overrides)).

## Reference diameter

- `maximum` (the default): twice the largest outer radius of any body component in any stage,
  including a bulged ogive's peak (`Profile::max_radius_m`). Internal parts, shoulders, fins, tube
  fins, lugs and rail buttons never count. In Loft, the project before hpr-sim, an internal part
  could set it ([Loft lesson L47](../decisions-and-roadmap.md#l47)).
- `nose_base`: the first nose cone's base diameter.
- `custom`: a given diameter.

The reference area is `π d²/4`.

## Motors and configurations

- **Mounts.** A [`MotorMount`](../api/hpr_design/config/struct.MotorMount.html) on a body tube or
  inner tube holds a motor. Its `overhang_m` is how far the nozzle exit sits aft of the mount's aft
  end. Each mount holds at most one motor, so a cluster has a mount per motor, such as inner tubes
  set off the body axis.
- **Configurations.** A [`Configuration`](../api/hpr_design/config/struct.Configuration.html)
  puts at most one [`MountedMotor`](../api/hpr_design/config/struct.MountedMotor.html) in each
  mount. A mounted motor is a [`SolidMotor`](../api/hpr_motor/motor/struct.SolidMotor.html) with its case diameter and length (for the checks) and
  an optional [ejection delay](../glossary.md#ejection-delay): the time from burnout to its
  ejection charge. It doesn't delay ignition.
- **Placement.** The motor's axis runs forward from the nozzle exit ([Solid motors](motor.md)).
  With `s_aft` the station of the mount's aft end, the nozzle exit is at station
  `s_aft + overhang`, on the mount's axis: for an inner tube offset `r` from the body axis at
  angle `θ` (from `x_B` toward `y_B`), at `(r cos θ, r sin θ)`; otherwise on the body axis. A motor
  element at `z_m` along the motor's own axis is at body `z = −(s_aft + overhang) + z_m`.
- **Composition** at `t` seconds after ignition:
  [`Assembly::mass_properties`](../api/hpr_design/config/struct.Assembly.html#method.mass_properties)`(t)`
  combines the structure with each motor's `SolidMotor::state(t).total` ([MK]). The dry assembly
  uses each motor's dry element.
- **More than one motor.** Every motor in a configuration ignites together at `t = 0`, on the pad.
  In a flight, each burning motor's thrust points along the rocket's axis (`z_B`) and acts at its
  own nozzle exit, and the thrusts and their moments are summed
  ([Rigid-body flight](flight.md#equations-of-motion)):
  - A cluster whose motors all light together is flown, and a motor off the body axis adds a
    turning moment. No test or comparison checks a cluster flight yet.
  - A two-stage design flies with every motor lit at `t = 0`, booster and sustainer together.
    That is not a staged flight, and
    [`Simulation::new`](../api/hpr_sim/flight/struct.Simulation.html#method.new) doesn't warn.
  - Staging under power, delayed ignition and air starts are planned for [M1.9](../decisions-and-roadmap.md#m1-9), the
    staging, clusters and air starts milestone.
- **Against RocketPy** [RP]: `total_mass(t)` and `center_of_mass(t)` are the same combination.
  RocketPy names the moment of inertia in pitch and yaw `I_11` and the one in roll `I_33`. Its
  `I_11(t)` is taken about the centre of dry mass (the rocket without propellant), so hpr's tensor
  is moved there before comparing. `I_33` sums the axial moments (every element is on the axis).

## Checks

[`checks::check`](../api/hpr_design/checks/fn.check.html) resolves a design and returns typed
[`Finding`](../api/hpr_design/checks/enum.Finding.html)s, each an error (impossible as described,
so a simulation would be wrong) or a warning (unusual, but it can be built and flown). Lengths compare with 1 nm of
slack (`LENGTH_TOLERANCE_M`), so round-off never raises one.

| finding | severity | when |
|---|---|---|
| `motor_wider_than_mount` | error | case diameter > mount inner diameter ([Loft lesson L50](../decisions-and-roadmap.md#l50)) |
| `motor_outside_mount` | error | the case doesn't overlap its mount along the axis at all (an overhang typed in mm as m) |
| `attachment_off_body` | error | an external part's extent (a fin root) doesn't overlap its body tube at all ([Loft lesson L50](../decisions-and-roadmap.md#l50)) |
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
refuses them with [`SimError::DesignChecks`](../api/hpr_sim/error/enum.SimError.html#variant.DesignChecks)
unless the caller sets
[`FlightSettings::accept_design_errors`](../api/hpr_sim/flight/struct.FlightSettings.html#structfield.accept_design_errors).

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
- **Property** (a proptest, which checks a rule on many random inputs): randomly placed masses sum to the structure's mass and centre, and
  sliding every part moves the centre rigidly without changing the tensor.
- **Checks** (`checks::tests`): each finding and its severity. A cluster pod's block fits and an
  on-axis part in the pod doesn't. Motors miss their mounts in both directions. Parts and a stage
  centre lie off the rocket. A layout with a corrupt parent index is skipped, not a panic.
- **Against RocketPy 1.13.0** (`config::tests::matches_rocketpy_example_rockets`):
  - Eight cases of the [fixture](../glossary.md#reference-value-and-fixture)
    [`validation/fixtures/design/rocketpy-rocket-mass.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/design/rocketpy-rocket-mass.json):
    seven [example rockets](../glossary.md#example-rockets) (Calisto at two motor positions) and
    Prometheus's `GenericMotor` (RocketPy's motor described by its masses alone, with no grain
    geometry). Cavour (added for its drag
    curve in [M1.5b](../decisions-and-roadmap.md#m1-5b), the drag milestone) has no motor dry mass; its design gives the
    motor 1e-15 kg, since hpr needs a positive one.
    [`docs/research/rocketpy-rocket-mass.md`](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/rocketpy-rocket-mass.md)
    gives the curve substitution and the examples left out.
  - The test derives the stage override, nozzle station and motor inputs from the fixture itself,
    independently of the design generator.
  - The rockets are compared at 103 even times through the burn and after it, and at up to 60 of
    RocketPy's LSODA knots: the times at which its ODE solver, LSODA, computed the grain geometry.
    Errors are relative: to the value itself, or to what a row names in brackets (the rocket's
    length for a centre).
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
- **Public designs** ([`validation/designs/`](https://github.com/nrdptel/hpr-sim/tree/main/validation/designs), written by `cargo xtask designs`, which a test keeps in
  sync): the eight RocketPy cases and two synthetic rockets resolve with no findings and assemble
  into valid bodies at ignition, mid-burn and burnout.
- **Lessons:** [Loft lesson L47](../decisions-and-roadmap.md#l47) `tests::reference_diameter_ignores_internal_components`;
  [Loft lesson L50](../decisions-and-roadmap.md#l50) `checks::tests::motor_wider_than_mount_is_rejected` and
  `checks::tests::fin_root_must_touch_body`.

[adr-007]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17
[levels]: ../accuracy.md#four-kinds-of-evidence
