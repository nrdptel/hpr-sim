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
  OpenRocket on 71 compared designs, within 1% in mass on 62 and in centre of mass on 63
  ([mass properties](mass.md#checked-against-openrocket)); and body radii against OpenRocket in
  the `.ork` import ([`.ork` design files](../format/ork.md)). A
  [cluster](../glossary.md#cluster)'s tubes sit where OpenRocket puts them, to 1e-15 m, and a motor
  out turns the rocket as the hand calculation says, to 3.7e-7 ([below](#clusters)). Not
  compared with a real flight, and a cluster's flight not yet with another simulator's
  ([M1.9c](../decisions-and-roadmap.md#m1-9c)).
- **What it leaves out:** each motor lights at its own `ignition`, at launch unless told
  otherwise, so a two-stage design whose file says nothing flies with every motor lit at once, and
  nothing warns; a staged flight gives the sustainer its ignition ([Staging](staging.md)). A
  cluster's motors light together, or not at all: no spread in ignition and no thrust misalignment.
  Fins on a nose cone or transition are refused.
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
  end. A mount that is a cluster of tubes holds the motor in every tube ([below](#clusters)).
- **Configurations.** A [`Configuration`](../api/hpr_design/config/struct.Configuration.html)
  puts at most one [`MountedMotor`](../api/hpr_design/config/struct.MountedMotor.html) in each
  mount. A mounted motor is a [`SolidMotor`](../api/hpr_motor/motor/struct.SolidMotor.html) with its case diameter and length (for the checks),
  an optional [ejection delay](../glossary.md#ejection-delay) (the time from burnout to its
  ejection charge, which doesn't delay ignition) and its
  [`ignition`](../api/hpr_design/config/enum.Ignition.html): at launch unless told otherwise
  ([Staging](staging.md#when-a-motor-lights)).
- **Placement.** The motor's axis runs forward from the nozzle exit ([Solid motors](motor.md)).
  With `s_aft` the station of the mount's aft end, the nozzle exit is at station
  `s_aft + overhang`, on the mount's axis: for an inner tube offset `r` from the body axis at
  angle `θ` (from `x_B` toward `y_B`), at `(r cos θ, r sin θ)`; otherwise on the body axis. A motor
  element at `z_m` along the motor's own axis is at body `z = −(s_aft + overhang) + z_m`.
- **Composition** at time `t`:
  [`Assembly::mass_properties`](../api/hpr_design/config/struct.Assembly.html#method.mass_properties)`(t)`
  combines the structure with each motor's `SolidMotor::state(t).total` ([MK]), every motor lit at
  `t = 0`.
  [`Assembly::mass_properties_lit`](../api/hpr_design/config/struct.Assembly.html#method.mass_properties_lit)
  takes each motor's own ignition time: a motor burns on its own clock, `t − t_ignition`, and one
  not yet lit is loaded. The dry assembly uses each motor's dry element.
- **More than one motor.** In a flight, each burning motor's thrust points along the rocket's axis
  (`z_B`) and acts at its own nozzle exit, and the thrusts and their moments are summed
  ([Rigid-body flight](flight.md#equations-of-motion)):
  - A cluster is flown, whether its motors share one mount ([below](#clusters)) or each has its
    own, and a motor off the body axis adds a turning moment.
  - A two-stage design fires in sequence when its motors are given their ignitions, and drops its
    booster at a separation ([Staging](staging.md)). Its design file alone lights every motor at
    launch, booster and sustainer together, unless it says otherwise.
- **Against RocketPy** [RP]: `total_mass(t)` and `center_of_mass(t)` are the same combination.
  RocketPy names the moment of inertia in pitch and yaw `I_11` and the one in roll `I_33`. Its
  `I_11(t)` is taken about the centre of dry mass (the rocket without propellant), so hpr's tensor
  is moved there before comparing. `I_33` sums the axial moments (every element is on the axis).

## Clusters

A cluster is several like motors side by side. In hpr it is one inner tube repeated: the tube's
[`cluster_m`](../api/hpr_design/parts/struct.InnerTube.html#structfield.cluster_m) lists each
tube's axis, `[x, y]` in metres in [body axes](frames.md), measured from the point the tube's
`radial_offset_m` and `angle_rad` set (the body's axis when both are 0). An empty list is one tube.
Motors of different kinds need one mount per kind. The decision record is [ADR-075][adr-075].

In a JSON design, a mount of three tubes 25 mm from the axis, with the first tube's motor out, is
these two fields (the rest of the inner tube and the mounted motor as usual):

```json
"cluster_m": [[0.025, 0.0], [-0.0125, 0.021650635], [-0.0125, -0.021650635]]
```

```json
"failed_tubes": [0]
```


- **Mass.** The tube weighs all its copies, each with its own
  [parallel-axis](../glossary.md#parallel-axis-theorem) term `m d²`. Whatever the tube holds (an
  engine block, a mass) is repeated in every tube the same way. A mass override on the cluster
  sets the whole cluster's mass; one on a part inside it (its mass, centre or inertia) sets each
  copy's, as OpenRocket does for the mass.
- **Motors.** The configuration names one motor for the mount, and placing it gives one motor per
  tube, one after another in the order of the tubes, each nozzle on its tube's axis. Their thrusts,
  masses and moments add up like any other motors'.
- **A motor out.** A mounted motor's
  [`failed_tubes`](../api/hpr_design/config/struct.MountedMotor.html#structfield.failed_tubes)
  names tubes whose motor never lights, counted from 0 in the order of `cluster_m`. That motor
  stays loaded and pushes nothing, which is how a cluster most often fails. The lit motors then
  push off-centre, and the rocket turns toward the motor that is out.
- **Motor numbers.** Every tube's motor counts as a motor, so a cluster of three before another
  mount moves that mount's motor from index 1 to 3. An ignition on the cluster's burnout takes its
  first motor that lights. A recovery trigger or separation on one motor's burnout or delay
  ([recovery](recovery.md)) waits on that motor alone: point it at a tube that lights, or with that
  motor out your parachute never opens.
- **From a `.ork` file.** The reader turns OpenRocket's named pattern into the list
  ([`.ork` design files](../format/ork.md#clusters)).

**Worked example.** The tests' single-stage rocket
([`synthetic-54mm-three-fin`](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/synthetic-54mm-three-fin.json)),
with its mount made a ring of three tubes `A = 0.02` m from the axis, at 0°, 120° and 240°
(measured from the body's `x` axis toward its `y` axis, as in [frames](frames.md)), and a Cesaroni
411I175-14A in each. It is an equation check, not a buildable rocket: three 38 mm motors don't fit
a 54 mm body, and the design checks say so. It is held at rest in a vacuum, so no air and no motion
add anything to the motors' push. With the motor at 0° out, 1 s into the burn:

| quantity | value |
|---|---|
| each lit motor's thrust `T`, in a vacuum | 193.98 N |
| the loaded motor, and each lit one at 1 s | 0.4375 kg, 0.3332 kg |
| the rocket's mass `m` | 1.584 kg |
| centre of mass across the axis, `c_x` (the loaded motor pulls it toward itself) | 1.33 mm |
| pitch moment `M = T (A + 2 c_x)` about the centre of mass | 4.396 N m |
| pitch inertia `I_yy` about the centre of mass | 0.1052 kg m² |
| pitch acceleration `M / I_yy`, at rest | 41.80 rad/s² |

The centre of mass moves toward the loaded motor: `(0.4375 × 20 − 2 × 0.3332 × 10) mm / 1.584`
is 1.32 mm, and the structure's own centre, 0.10 mm off the axis from its rail buttons, adds the
rest. The two lit motors sit at `x = −A/2` each, so about the centre of mass their thrust has the
lever `A/2 + c_x` twice. The flight's equations give the same angular acceleration to 3.7e-7 (the
full inertia tensor, not only `I_yy`, turns the moment into a turn); the difference is the
mass-flow terms (the centre of mass moving as two motors burn and one doesn't, and the jets). With
all three lit, the thrusts balance, and the rocket turns 225 times slower (0.186 rad/s²), from its
centre of mass sitting 0.033 mm off the axis. The numbers are pinned by the test
`cluster_motor_out_produces_pitch_moment` in `hpr-sim`.

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
| `cluster_tubes_overlap` | warning | two tubes of a [cluster](#clusters) are closer than a tube's diameter, so they cross and that mass counts twice |
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
- **Clusters by hand:**
  - `parts::tests::a_cluster_is_its_tubes_each_with_its_parallel_axis_term`: four tubes' mass,
    centre and roll inertia about the body's axis, to 1e-15.
  - `tree::tests::a_cluster_repeats_what_it_holds_in_every_tube`: an engine block in every tube of
    a 3-ring, the structure gaining two tubes and two blocks, and the checks' warnings.
  - `config::tests`: a motor in a 3-ring is three motors at their tubes, the rocket's roll inertia
    gaining each one's `m d²`; a failed tube stays loaded and unlit, a tube the mount lacks is
    refused; a sustainer lit by a clustered booster's burnout lights with one booster motor out.
  - `hpr_sim::staging::tests`: three motors' thrust and mass summed, and the motor out above
    (`cluster_motor_out_produces_pitch_moment`, [Loft lesson L31](../decisions-and-roadmap.md#l31));
    a clustered sustainer lit after a powered separation.
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
[adr-075]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-075-a-cluster-is-one-tube-repeated-and-a-motor-in-it-one-motor-per-tube-2026-09-25
