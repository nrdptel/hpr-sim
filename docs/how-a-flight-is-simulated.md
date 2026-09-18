# How a flight is simulated

This page follows a flight from ignition to landing, and says in plain words what hpr computes at
each stage and which model does it. Read it to learn what lies behind a number hpr prints, before
the model pages it links. It describes the method, not how well it works: no whole flight has been
validated yet, and [Accuracy](accuracy.md) keeps every result so far.

![A rocket's flight seen from the side. It lifts off the pad, leaves the rail, burns out, and coasts to apogee west of the pad, into the wind. It then drifts east under a drogue and a main parachute, and lands east of the pad. Seven numbered points mark the events: liftoff, rail exit, burnout, apogee, the drogue opening, the main opening, and landing.](images/flight-phases.svg)

The drawing is the shape of the [Getting started](getting-started.md) example's flight, not to
scale: its apogee is 874 m up and about 100 m west of the pad, and it lands about 100 m east.

## What goes in

Building a simulation gathers five inputs, and works out once everything that doesn't change
during the flight.

- **The rocket.** A tree of parts, such as a nose cone, body tubes and a fin set, with their
  positions ([The design tree](physics/design.md)). Their shapes
  ([Nose cones and transitions](physics/shapes.md)) and materials give each part's mass,
  [centre of gravity](glossary.md#centre-of-gravity-cg) and inertia
  ([Mass properties](physics/mass.md)). The design's checks run first, and a rocket that can't
  exist, such as one with a motor wider than its mount, is refused.
- **The motor.** Its thrust curve gives the thrust at every instant. The way its propellant burns
  away gives the motor's mass, centre of gravity and inertia as it burns
  ([Solid motors](physics/motor.md)). So the whole rocket gets lighter, and its centre of gravity
  moves, during the burn.
- **The aerodynamics.** From the rocket's shape and surface, hpr builds its model of the air's
  forces: the [normal force](glossary.md#normal-force) on each body part and fin set, the
  [centre of pressure](glossary.md#centre-of-pressure-cp) where it acts, and the drag. They depend
  on the [Mach number](glossary.md#mach-number), the
  [angle of attack](glossary.md#angle-of-attack) and the [Reynolds number](glossary.md#reynolds-number)
  ([Aerodynamics](physics/aero.md)).
- **The surroundings.** The launch site, on the WGS 84 model of the Earth's shape
  ([Geodesy](physics/geodesy.md)), with the [launch frame](glossary.md#launch-frame-enu): east,
  north and up from the pad ([Frames](physics/frames.md)). Gravity that changes with latitude and
  height, and the Coriolis effect of the Earth's rotation ([Gravity](physics/gravity.md)). The air's
  density, pressure, temperature and speed of sound at each height
  ([Atmosphere](physics/atmosphere.md)), and the wind at each height ([Wind](physics/wind.md)).
  A turbulence model exists ([Turbulence](physics/turbulence.md)), but no flight uses it yet.
- **The rail, the recovery devices and the settings.** The rail's length, direction and friction;
  each parachute or streamer and when it fires; and how finely to step through time.

## From the pad to the ground

The numbers match the drawing.

1. **Ignition and liftoff.** Every motor ignites at time zero; there is no staging or delayed
   ignition yet. The rocket stands on the rail, its aft end at the rail's foot, and holds still
   until the forces along the rail, mostly the thrust against the weight, push it up harder than
   the rail's friction holds it. That instant is liftoff. If the motors burn out first, the flight
   ends on the pad.
2. **On the rail.** The rocket slides along the rail without turning: one degree of freedom.
   [Rail exit](glossary.md#rail-exit-and-rail-exit-velocity) comes when its last rail button or
   lug leaves the top of the rail. A rocket that stops on the rail comes to rest there
   ([Rigid-body flight](physics/flight.md#phases)).
3. **Powered flight, to burnout.** Off the rail, the rocket is a rigid body free to move and turn
   in every direction, the six degrees of freedom of a
   [6-DOF](glossary.md#6-dof-six-degrees-of-freedom) simulator. At every instant hpr adds up the
   forces and their turning effects:
   - the thrust, along the rocket's axis;
   - the weight and the Coriolis force, at the centre of gravity;
   - the air's forces, from the air's velocity past the rocket, wind included: the drag along the
     axis, and each body part's and fin set's normal force at its own centre of pressure;
   - the burning propellant's effects: the centre of gravity moving inside the rocket, and jet
     damping, the exhaust carrying away some of any turning motion.

   From these it works out how the rocket speeds up and turns
   ([Rigid-body flight](physics/flight.md)). A crosswind meets the rocket partly from the side, and
   the fins' normal force, behind the centre of gravity, swings the nose into it, so the rocket
   climbs upwind ([weathercocking](glossary.md#weathercocking)). The top speed comes just before
   [burnout](glossary.md#burnout), the end of the last motor's thrust curve, once the thrust no
   longer beats the drag and the weight.
4. **Coast, to apogee.** The same equations with no thrust: drag and gravity slow the climb.
   [Apogee](glossary.md#apogee) is where the vertical speed falls through zero.
5. **The drogue.** Each recovery device fires its charge at its trigger: apogee, a height on the
   way down, a time, or a motor's [ejection delay](glossary.md#ejection-delay). After its lag, it
   [deploys](glossary.md#deployment). From the first deployment the rocket is a single point with
   mass: its attitude freezes, and it falls under gravity and the open devices' drag while the
   wind carries it along, which is its [drift](glossary.md#drift).
6. **The main.** A second device, typically the main set to a height above the ground, adds its
   drag to the drogue's. A device can also cut another away as it opens, and a rocket can
   [separate](glossary.md#separation) into parts that each come down on their own. Streamers and
   [tumbling](glossary.md#tumble-recovery) are recovery devices too ([Recovery](physics/recovery.md)).
7. **Landing.** The flight ends when the centre of gravity comes back down to the launch site's
   height. The ground is flat, at the height of the pad: there is no terrain.

A flight can also end on the pad, stalled on the rail, at a time cap (an hour by default) or at a
step limit, and each of these is reported by name. Anything else that stops a flight is an error:
reaching Mach 1, for example, because there are no supersonic aerodynamics yet
([Rigid-body flight](physics/flight.md#events-and-termination)).

## How hpr steps through time

At any instant, the rocket's state is 13 numbers: where it is, how fast it moves, which way it
points and how fast it turns. The equations above turn a state into its rate of change. An
integrator builds the flight from them by stepping forward in time, one short step after another
([Time integration](physics/integration.md)).

- **Steps that size themselves.** hpr's default method,
  [Dormand–Prince 5(4)](glossary.md#dormandprince-and-rk4), estimates each step's error and sizes
  the next step to keep it within a [tolerance](glossary.md#tolerance). Steps are short where things
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
  program can also watch every step as it happens, or record chosen quantities at a fixed
  interval; [Getting started](getting-started.md#the-program-step-by-step) does both.

## What is left out

Each model page lists what its model leaves out. These are the gaps that matter most for a whole
flight:

- **Mach 1 and above.** A flight that reaches Mach 1 stops with an error until the transonic and
  supersonic aerodynamics of [M1.8][roadmap] arrive, and from Mach 0.8 to 1 the aerodynamics are
  unvalidated ([Aerodynamics](physics/aero.md)).
- **Large angles of attack.** The aerodynamics are for small angles, with no
  [stall](glossary.md#stall), but a flight uses them at every angle: just off the rail in a strong
  crosswind, and near apogee.
- **Staging, clusters with delayed ignition, and air starts**, planned for [M1.9][roadmap].
- **Roll**, the torques that spin a rocket up and slow its spin, planned for [M1.8][roadmap];
  [tip-off](glossary.md#tip-off), thrust misalignment and turbulence, which no milestone plans yet.
- **Under a parachute:** the shock and overshoot as a canopy opens, the air carried along with it
  ([added mass](glossary.md#added-mass)), the airframe's own drag, and the rocket swinging below
  the canopy ([Recovery](physics/recovery.md)).
- **Terrain.** The ground is flat, at the pad's height.

[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
