# How a flight is simulated

This page follows a flight from ignition to landing, and says in plain words what hpr computes at
each stage and which model does it. Read it to learn what lies behind a number hpr prints, before
the model pages it links. It describes the method, not how well it works. When both codes fly
the same drag, whole flights match RocketPy's in height, speed and time, and in where they go,
except for rockets that leave the rail slowly in a wind
([Accuracy](accuracy.md#whole-flights-against-rocketpy)). With hpr's own drag, against RocketPy
flying the drag its examples ship, heights differ by −6.985% to +10.306%
([Accuracy](accuracy.md#whole-flights-with-each-codes-own-drag)). [Accuracy](accuracy.md) keeps
every result so far.

![A rocket's flight seen from the side. It lifts off the pad, leaves the rail, burns out, and coasts to apogee west of the pad, into the wind. It then drifts east under a drogue and a main parachute, and lands east of the pad. Seven numbered points mark the events: liftoff, rail exit, burnout, apogee, the drogue opening, the main opening, and landing.](images/flight-phases.svg)

The drawing is the shape of the [Getting started](getting-started.md) example's flight, not to
scale: its apogee is 779 m up and 86 m west of the pad, and it lands 94 m east of it.

## What goes in

Building a simulation gathers five inputs, and works out once everything that doesn't change
during the flight.

| input | what hpr takes from it | pages |
|---|---|---|
| the rocket | A tree of parts, such as a nose cone, body tubes and a fin set, with their positions. Their shapes and materials give each part's mass, [centre of gravity](glossary.md#centre-of-gravity-cg) and inertia. The design's checks run first, and a rocket that can't exist, such as one with a motor wider than its mount, is refused | [Design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| the motor | The thrust at every instant, from its thrust curve. Its mass, centre of gravity and inertia as its propellant burns away, so the whole rocket gets lighter, and its centre of gravity moves, during the burn | [Solid motors](physics/motor.md) |
| the aerodynamics | Built from the rocket's shape and surface: the [normal force](glossary.md#normal-force) on each body part and fin set, the [centre of pressure](glossary.md#centre-of-pressure-cp) where it acts, and the drag. They depend on the [Mach number](glossary.md#mach-number), the [angle of attack](glossary.md#angle-of-attack) and the [Reynolds number](glossary.md#reynolds-number) | [Aerodynamics](physics/aero.md) |
| the surroundings | The launch site on the [WGS 84](glossary.md#wgs-84) model of the Earth's shape, and the [launch frame](glossary.md#launch-frame-enu): east, north and up from the pad. Gravity that changes with latitude and height, and the [Coriolis acceleration](glossary.md#coriolis-acceleration), a small sideways push that anything moving over the rotating Earth appears to feel. The air's density, pressure, temperature and speed of sound, and the wind, at each height. A turbulence model exists, but no flight uses it yet | [Geodesy](physics/geodesy.md), [Frames](physics/frames.md), [Gravity](physics/gravity.md), [Atmosphere](physics/atmosphere.md), [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| the rail, recovery and settings | The rail's length, direction and friction; each parachute or streamer and when it fires; how finely to step through time | [Rigid-body flight](physics/flight.md), [Recovery](physics/recovery.md), [Time integration](physics/integration.md) |

## From the pad to the ground

The numbers match the drawing.

1. **Ignition and liftoff.** Every motor lights at time zero unless its design gives it a later
   ignition: an [air start](glossary.md#air-start), or a sustainer lit after its booster
   ([Staging](physics/staging.md)). A two-stage design that says nothing lights both stages on the
   pad. The rocket stands on the rail, its aft end
   at the rail's foot, and holds still until the push up the rail, mostly the thrust, beats the
   weight's pull down it and the rail's friction. That instant is [liftoff](glossary.md#liftoff).
   If the motors burn out first, the flight ends on the pad.
2. **On the rail.** The rocket slides along the rail without turning: one degree of freedom.
   [Rail exit](glossary.md#rail-exit-and-rail-exit-velocity) comes when its last rail guide, a
   rail button or a launch lug (a short tube on the body), leaves the top of the rail. A rocket
   that stops on the rail comes to rest there ([Rigid-body flight](physics/flight.md#phases)).
3. **Powered flight, to burnout.** Off the rail, the rocket is a rigid body free to move and turn
   in every direction, the six degrees of freedom of a
   [6-DOF](glossary.md#6-dof-six-degrees-of-freedom) simulator. At every instant hpr adds up the
   forces and their turning effects:
   - the thrust of each burning motor, along the rocket's axis. So a
     [cluster](glossary.md#cluster) of motors that all light at once is flown, with their thrusts
     added, and a motor off the centre line adds a turning effect, as a motor that fails to light
     does ([Clusters](physics/design.md#clusters)). Tests check it; no other simulator has yet;
   - the weight and the [Coriolis](glossary.md#coriolis-acceleration) force, at the centre of
     gravity;
   - the air's forces, from the air's velocity past the rocket, wind included: the drag along the
     axis, and each body part's and fin set's normal force at its own centre of pressure;
   - the burning propellant's effects: the centre of gravity moving inside the rocket, jet
     damping (the exhaust carrying away some of any turning motion), and the propellant's
     [internal momentum](glossary.md#internal-momentum), which is counted twice (see
     [What is left out](#what-is-left-out)).

   From these it works out how the rocket speeds up and turns
   ([Rigid-body flight](physics/flight.md)). A crosswind meets the rocket partly from the side, and
   the fins' normal force, behind the centre of gravity, swings the nose into it, so the rocket
   climbs upwind ([weathercocking](glossary.md#weathercocking)). The top speed usually comes just
   before [burnout](glossary.md#burnout), the end of the last motor's thrust curve, once the thrust
   no longer beats the drag and the weight.
4. **Coast, to apogee.** The same equations with no thrust: drag and gravity slow the climb.
   [Apogee](glossary.md#apogee) is where the vertical speed falls through zero.
5. **The drogue.** Each recovery device fires its charge at its trigger: apogee, a height on the
   way down, a time, or a motor's [ejection delay](glossary.md#ejection-delay). After its lag, a
   set time from the charge to its lines stretching, it [deploys](glossary.md#deployment). From
   the first deployment the rocket is a single point with mass: its attitude (which way it
   points) freezes, and it falls under gravity and the open devices' drag while the wind carries
   it along, which is its [drift](glossary.md#drift).
6. **The main.** A second device, typically the main set to a height above the ground, adds its
   drag to the drogue's. A device can also cut another away as it opens, and a rocket can
   [separate](glossary.md#separation) into parts that each come down on their own.
   [Streamers](glossary.md#streamer) and [tumbling](glossary.md#tumble-recovery) are recovery
   devices too ([Recovery](physics/recovery.md)).
7. **Landing.** The flight ends when the centre of gravity comes back down to the launch site's
   height. The ground is flat, at the height of the pad: there is no terrain.

A flight can also end in other ways, and hpr reports each by name
([Rigid-body flight](physics/flight.md#events-and-termination)):

| ending | what happened |
|---|---|
| on the pad | the motors burnt out before the rocket lifted off |
| stalled on the rail | it lifted off, then stopped on the rail after the motors burnt out |
| time cap | the flight reached its time limit, 3,600 s (an hour) after ignition by default, before landing |
| step limit | the integrator (below) used up its budget of steps, a million attempted steps by default, before landing |
| separated | the rocket split into parts, and each part's own descent says where it landed |

The time cap and the step limit are safety stops, so that a flight which doesn't land still ends.
A [stiff](glossary.md#stiff-problem) stretch of flight, one that forces very short steps, can show
up as the step limit ([Time integration](physics/integration.md#defaults-and-limits)). Both limits
are fields of `FlightSettings` (`max_time_s` and `step_limit`), and a program can change them.

Anything else that stops a flight is an error: reaching Mach 5, for example, the top of the
speeds hpr's normal force and drag cover.

## How hpr steps through time

At any instant, the rocket's state is 13 numbers: where it is (three), how fast it moves (three),
which way it points (four, as a quaternion, a compact way to store a rotation) and how fast it
turns (three). The equations above turn a state into its rate of change. An
integrator builds the flight from them by stepping forward in time, one short step after another
([Time integration](physics/integration.md)).

- **Steps that size themselves.** hpr's default method is
  [Dormand–Prince 5(4)](glossary.md#dormandprince-and-rk4). On each step it makes two estimates of
  the new state, one of fifth order and one of fourth; that is the "5(4)". The higher a method's
  order, the faster its error shrinks as the step gets shorter. hpr keeps the fifth-order estimate,
  and takes the difference between the two as the step's error. It sizes the next step to keep that
  error within a [tolerance](glossary.md#tolerance). Steps are short where things
  change fast, at liftoff and burnout, and long in a steady descent
  ([adaptive time step](glossary.md#adaptive-time-step)). At the default settings, the Getting
  started flight's apogee is within about a micrometre of the answer at much tighter settings,
  and a whole flight takes about a millisecond of computing
  ([Rigid-body flight](physics/flight.md#integration-settings)).
- **Stop times.** Moments known in advance where a force changes abruptly, such as each point of
  the thrust curve and burnout, are [stop times](glossary.md#stop-time): a step always ends exactly
  there, so none straddles a jump.
- **Events.** Moments found during the flight, such as liftoff, rail exit, apogee, an altitude
  trigger and landing, are [events](glossary.md#event). When the quantity that defines one changes
  sign within a step (the vertical speed, for apogee), hpr finds the instant it crossed zero inside
  that step.
- **What you get back.** Every event, with a snapshot of the flight at that instant: time,
  position, velocity, height, airspeed, Mach number, angle of attack, thrust, mass and more. A
  program can also watch every step as it happens, as
  [Getting started](getting-started.md#the-program-step-by-step) does to find the top speed, or
  record chosen quantities at a fixed interval with a `Recorder`
  ([Recording a trajectory](recording-a-trajectory.md)).

## What is left out

Each model page lists what its model leaves out. These are the gaps that matter most for a whole
flight:

- **Near and past Mach 1, the drag is lightly checked.** The normal force, centre of pressure and
  drag all carry a flight from Mach 0 to 5. The normal force was checked against a wind tunnel to
  Mach 4.63 ([Aerodynamics](physics/aero.md#normal-force-through-mach-1)). The drag was checked at
  Mach 0.3 against other programs' curves, and against the same wind tunnel from Mach 0.6 to 4.63,
  where it reads high at most speeds, most of all with fins past Mach 1
  ([Aerodynamics](physics/aero.md#drag-against-the-arcas-robin-wind-tunnel)). Near and above the
  speed of sound it is Niskanen's semi-empirical method (formulas fitted to measurements), not yet
  compared with [RASAero II](glossary.md#rasaero-ii)'s
  ([M1.8b2](decisions-and-roadmap.md#m1-8b2), the drag against RASAero II).
- **Large angles of attack.** The aerodynamics are for small angles, with no
  [stall](glossary.md#stall), but a flight uses them at every angle: just off the rail in a strong
  crosswind, and near apogee.
- **Staging and air starts are checked only by tests.** Each motor lights at its own time, and a
  sustainer flies on after a powered separation ([Staging](physics/staging.md)), but no staged
  flight has been compared with another simulator yet ([M1.9c](decisions-and-roadmap.md#m1-9c)).
  A cluster is flown and checked by tests, including a motor that fails to light, but not yet
  against another simulator ([Clusters](physics/design.md#clusters)).
- [Tip-off](glossary.md#tip-off), thrust misalignment (a motor pushing slightly off the rocket's
  axis) and turbulence, which no milestone plans yet. Roll from canted fins and roll damping are
  modelled, and checked against measurements only from Mach 1.5 up
  ([Roll: forcing and damping](physics/aero.md#roll-forcing-and-damping)).
- **One term counted twice.** A thrust curve measured on a test stand already includes the
  propellant's [internal momentum](glossary.md#internal-momentum), and the equations of motion add
  it again, as RocketPy's do. hpr keeps it so that the two codes can be compared like for like. On
  the Getting started rocket it adds 21 N to the push at liftoff and changes the burnout speed by
  at most 0.05 m/s ([Rigid-body flight](physics/flight.md#equations-of-motion)).
- **Under a parachute:** the drag overshoot as a canopy fills, so the opening load hpr reports is
  no safe bound (by default a canopy opens at once); the air carried along with it
  ([added mass](glossary.md#added-mass)); the airframe's own drag; and the rocket swinging below
  the canopy ([Recovery](physics/recovery.md)).
- **Terrain.** The ground is flat, at the pad's height.

