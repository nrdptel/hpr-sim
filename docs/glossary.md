# Glossary

This page defines the terms the other pages use, each in a sentence or two, with a link to the page
that models the term or uses it most. Terms are listed alphabetically, and each has its own
heading, so any page can link straight to one. Where hpr uses a term in a particular way (a sign, a
frame, a reference point), the entry says so. The definitions follow the model pages; if this page
and a model page ever disagree, the model page is the authority.

## 6-DOF (six degrees of freedom)

A simulation that follows all six ways a rigid body can move: its position along three axes and its
rotation about three. hpr flies a rocket in 6-DOF from rail exit on. On the rail it has one degree
of freedom, along the rail, and under an open parachute it is a point mass whose attitude is
frozen. See [Rigid-body flight](physics/flight.md#phases).


## Adaptive time step

A time step the integrator (the part that steps the equations forward in time) chooses for itself.
It estimates each step's error, and shrinks or grows the next step to keep that error within a
[tolerance](#tolerance). hpr's default method, Dormand–Prince 5(4) at `rtol = atol = 1e-8`, puts
Valetudo's apogee (one of RocketPy's example rockets) within 1.1e-6 m of a far tighter run. See
[Time integration and events](physics/integration.md#methods).


## Added mass

The air a canopy has to push along with it while it speeds up or slows down relative to the air,
counted as extra mass that carries no weight; in steady descent it changes nothing.
RocketPy includes it in its parachute descent and hpr doesn't. For NDRT 2020's main, one of the
[example rockets](#example-rockets), RocketPy's added mass is 15.9 kg against the rocket's 20.8 kg,
which makes it the likely cause of the largest difference between the two codes' descents. See
[Recovery](physics/recovery.md#against-rocketpy).

## AGL (above ground level)

Height above the launch site. A recovery device's altitude trigger is a height above the launch
site, and a flight ends when the centre of mass comes back down to the site's height. See
[Recovery](physics/recovery.md#triggers-lag-and-release) and
[Rigid-body flight](physics/flight.md#events-and-termination).


## Air start

A motor lit in flight, after liftoff, rather than on the pad. hpr can't fly one yet, because every motor in a configuration ignites at time zero. Air starts are planned for [M1.9](decisions-and-roadmap.md#m1-9), the staging and clusters milestone. See [The design tree](physics/design.md#motors-and-configurations).

## Angle of attack

The angle between the rocket's axis and the airflow it meets. In hpr it is the total angle `α`
between the body axis, pointing to the nose, and the rocket's velocity relative to the air, from 0
to π (180°). hpr's aerodynamics are small-angle models, so results at large angles, off the rail in
a strong crosswind and near apogee, are the least trustworthy. See
[Frames](physics/frames.md#aerodynamic-angles) and
[Aerodynamics](physics/aero.md#validity-and-open-questions).


## API reference

The documentation of hpr-sim's code: every public type, function and constant, generated from the
source by rustdoc, Rust's documentation tool. It is part of this site, and on GitHub it has to be
built; see [The API reference](api.md).


## Apogee

The highest point of a flight, where the rocket stops climbing. hpr finds it as the moment the
centre of mass's rate of climb above the [WGS 84](#wgs-84) ellipsoid falls through zero. It does
not use the launch frame's up axis, whose flat plane rises above the curved Earth with distance
(7.8 m at 10 km). See [Rigid-body flight](physics/flight.md#events-and-termination).


## Average thrust

A motor's [total impulse](#total-impulse) divided by its [burn time](#burn-time), in newtons (N), as
ThrustCurve.org defines it. It is the number in a [motor designation](#motor-designation). See
[Solid motors](physics/motor.md#thrust-curve).


## Barrowman's method

J. S. Barrowman's 1966–67 method for the [normal-force slope](#normal-force-slope) and
[centre of pressure](#centre-of-pressure-cp) of a slender finned rocket: each nose, transition and
fin set is worked out on its own, and the results are summed. hpr follows it, with extensions from
Niskanen's 2009 thesis. Four of Barrowman's five printed examples agree within 1%, and the six-fin
Recruiter's slope is 2.87% high. See [Aerodynamics](physics/aero.md#verification).


## Base drag

The drag on a rocket's flat aft end, its base. hpr works it out on the base's area, with a coefficient of `0.12 + 0.13 M²` below Mach 1 and `0.25/M` above it (`M` the [Mach number](#mach-number)). While a motor burns, the part of the base the motor covers has no base drag, so hpr subtracts the burning motors' cross-section from that area ([power-on drag](#power-on-and-power-off-drag)). See [Aerodynamics](physics/aero.md#drag).

## BATES grain

A cylindrical propellant grain with a hole along its axis (the bore), which burns on the bore and,
unless inhibited, on both ends. hpr can place a motor's propellant as a stack of identical BATES
grains, which sets how its centre of mass and inertia change as it burns. See
[Solid motors](physics/motor.md#where-the-propellant-is).


## Bearing

A direction on the ground, measured clockwise from true north: 0° is north, 90° east, 180° south
and 270° west. [Getting started](getting-started.md#what-it-printed) gives the landing point as a
distance and a bearing from the pad. A [wind direction](#wind-direction) is a bearing too: the one
the wind blows from.


## Boattail

A transition at the tail that narrows toward the aft end. Its normal-force slope is negative, so it moves the [centre of pressure](#centre-of-pressure-cp) forward. hpr counts its pressure drag as a share of the base drag on the area it removes: all of it for a short, steep boattail and none for a long, gentle one. See [Aerodynamics](physics/aero.md#drag).

## Body frame

The axes fixed to the rocket. The origin is the nose tip, on the axis. `z_B` points along the axis
toward the nose, `x_B` is the design's zero direction around the body (the one fins and rail
buttons are placed from), and `y_B` completes a right-handed set, so every part lies at `z_B ≤ 0`.
See [Frames](physics/frames.md#body-frame-b).


## Body lift

A sideways force on the body itself, not the fins, when the rocket flies at an angle to the
airflow. It grows with the square of the sine of the [angle of attack](#angle-of-attack), so it is
nothing at small angles and large at steep ones, such as a slow rocket leaving the rail into a
crosswind. hpr includes it (Galejs's method, with a constant `K` whose value is uncertain); RocketPy
leaves it out. See [Aerodynamics](physics/aero.md#bodies-of-revolution).


## Burn time

How long a motor burns, by the NFPA 1125 rule that ThrustCurve.org uses: from the moment the thrust
first reaches 5% of its peak to the moment it last falls to 5% of its peak, in seconds. It is not
the time of the thrust curve's last point, which is [burnout](#burnout). See
[Solid motors](physics/motor.md#thrust-curve).


## Burnout

The moment a motor stops producing thrust. In hpr it is the time of the thrust curve's last point:
from then on the thrust, the propellant flow and the propellant left are all zero. A flight records
a burnout event and steps exactly to it, because the equations change there. See
[Solid motors](physics/motor.md#thrust-curve).


## Calibre (caliber)

A length measured in body diameters. A [stability margin](#stability-margin) is usually quoted in
calibres, and a nose cone three calibres long is three times as long as its base diameter. See
[Shapes](physics/shapes.md#solids-of-revolution).


## Canard

A fin set near the nose, ahead of the main fins. Its normal force acts well forward, so it moves the [centre of pressure](#centre-of-pressure-cp) forward, and as speed rises its slope grows too. See [Aerodynamics](physics/aero.md#your-rockets-centre-of-pressure).

## Centre of dry mass

The centre of mass of the rocket with its motor's propellant gone: the empty rocket, motor casing
included. It does not move during the flight, so RocketPy follows this point, and the comparisons
with RocketPy measure speeds and drifts there. The [centre of gravity](#centre-of-gravity-cg) of
the loaded rocket lies aft of it while the motor burns. See
[Accuracy](accuracy.md#whole-flights-against-rocketpy).


## Centre of gravity (CG)

The point where the rocket's mass balances. The model pages call it the centre of mass, which in
uniform gravity is the same point. It moves as the propellant burns, and hpr recomputes it at every
instant from the parts and the motors. See
[The design tree](physics/design.md#motors-and-configurations) and
[Mass properties](physics/mass.md).


## Centre of pressure (CP)

The point along the rocket where the [normal force](#normal-force) acts, given as a
[station](#station) aft of the nose tip. hpr finds it by averaging each component's own centre of
pressure, weighted by its [normal-force slope](#normal-force-slope), at small angles of attack. A
rocket is statically stable when its CP is behind its [centre of gravity](#centre-of-gravity-cg).
See [Aerodynamics](physics/aero.md#conventions).


## CIPM-2007

The formula for the density of moist air, from its temperature, pressure and humidity, that the International Committee for Weights and Measures (CIPM) adopted in 2007. hpr's humid-air density is checked against it, within 0.047% over 15 to 27 °C. See [Atmosphere](physics/atmosphere.md).

## Closed form

An answer written as an exact formula, such as the parabola a body follows in a vacuum, rather than one computed step by step. hpr's analytic tests compare the code with closed forms, the first of the [four kinds of evidence](accuracy.md#four-kinds-of-evidence) on Accuracy.

## Cluster

Several motors in one rocket, burning side by side, each in its own mount. hpr lights every motor in a configuration together at time zero. It sums their thrusts along the rocket's axis, and adds the turning moment of any motor set off the axis. No test or comparison checks a cluster flight yet, and clusters with delayed ignition come with [M1.9](decisions-and-roadmap.md#m1-9), the staging and clusters milestone. See [The design tree](physics/design.md#motors-and-configurations).

## Code-to-code comparison

Flying the same rocket, or the same part of a flight, in hpr and in another simulator from the same
inputs, and comparing the numbers. It is the third of four [kinds of evidence][levels], and it
shows that two codes agree, not that either matches a real flight. hpr's parachute descents match
RocketPy's within 3% on all 30 numbers compared; whole flights come next. See
[Recovery](physics/recovery.md#against-rocketpy).


## Configuration

One choice of motors for a rocket design, at most one in each motor mount, named by an id such as `h54`. A design can hold several, such as the same rocket on different motors, and a flight names the one it flies. Every motor in a configuration lights at time zero until staging arrives with [M1.9](decisions-and-roadmap.md#m1-9). See [Your own rocket](your-own-rocket.md#the-program-step-by-step) and [The design tree](physics/design.md#motors-and-configurations).

## Coriolis acceleration

The sideways acceleration that anything moving over the rotating Earth appears to have, `−2Ω × v`,
with `Ω` the Earth's rotation (7.292115e-5 rad/s) and `v` the velocity in the Earth-fixed launch
frame. It is the only rotating-Earth term hpr adds, since the centrifugal part is already inside
[normal gravity](#normal-gravity), and it is on by default. It is small: in one test it moves the
landing point of a 3 km parachute descent 0.37 m east. See
[Gravity](physics/gravity.md#what-normal-gravity-is).


## COTS motor

A commercial off-the-shelf motor: a solid rocket motor bought from a manufacturer, single-use or as
a reload for a reusable case. hpr-sim covers only these for now, and bundles 32 of their thrust
curves, listed under [The bundled motors](physics/motor.md#the-bundled-motors). See [Solid motors](physics/motor.md) and [Start here](start-here.md#what-hpr-sim-is).


## Crate

A Rust package: the unit that Rust code is built, versioned and shared in. hpr-sim is split into
crates, such as `hpr-core` for the maths and the Earth and `hpr-sim` for the flight, so a program
takes only the ones it needs. See [The API reference](api.md#the-crates).


## Decision record (ADR)

A short record of a significant choice, the alternatives considered and why one was picked,
labelled like [ADR-011][adr-011] (the rigid-body flight decision). All of them are in the
[decision log][decisions]. See [Decisions and the roadmap](decisions-and-roadmap.md).


## Deployment

The moment a recovery device, such as a parachute, comes out to its full line length (line stretch)
and starts to fill. In hpr it follows the device's trigger (apogee, a height above the ground, a
time, or a motor's [ejection delay](#ejection-delay)) after a set lag. The flight's first deployment
switches the rocket to the descent, as a point mass. See
[Recovery](physics/recovery.md#triggers-lag-and-release).


## Descent rate

How fast a rocket falls under its recovery device, in m/s. Once drag balances weight it settles at
the equilibrium descent speed `v_e = √(2 m g / (ρ C_D S))`, with `m` the mass, `g` gravity, `ρ` the
air density and `C_D S` the [drag area](#drag-area). A 1.1 kg rocket under a 1 m flat canopy with
`C_D` 0.8, in air of 1.225 kg/m³, falls at 5.294 m/s. See
[Recovery](physics/recovery.md#the-descent).


## Design file

A rocket design saved as text, so it can be kept, shared and read back. Today it is the JSON of hpr's `Rocket` type: each key is a Rust field's name, with its unit in the name (`length_m`), and a mounted motor is written out in full, thrust curve included. The format is provisional until the open design format, [M3.3](decisions-and-roadmap.md#m3-3). See [Your own rocket](your-own-rocket.md#as-a-design-file).

## Dormand–Prince and RK4

Two ways of stepping the equations of motion through time. Dormand–Prince 5(4), also called
`DOPRI5`, makes two estimates on each step and shrinks or grows the step to keep their difference
under a [tolerance](#tolerance); it is hpr's default. RK4, the classic fourth-order Runge–Kutta
method, takes steps of a fixed size. See [Time integration](physics/integration.md#methods).

## Drag area

A recovery device's drag coefficient times the area that coefficient is measured on, `C_D S`, in m².
The drag force is the [dynamic pressure](#dynamic-pressure) times it. hpr takes it directly
(RocketPy's `cd_s`) or from a canopy's diameter and its coefficient on the
[nominal area](#nominal-area), and adds up the drag areas of every open device. See
[Recovery](physics/recovery.md#drag-area).


## Drag coefficient

Drag divided by [dynamic pressure](#dynamic-pressure) and an area: a number without units that says
how draggy a shape is. For the rocket, `C_D0` is the coefficient at zero
[angle of attack](#angle-of-attack) on the [reference area](#reference-area), built up from skin
friction, pressure, base and fin terms, or read from a table instead. For a parachute, Knacke's
`C_D0` is on the canopy's [nominal area](#nominal-area), a different convention. See
[Aerodynamics](physics/aero.md#drag) and [Recovery](physics/recovery.md#drag-area).


## Drag crisis

A sudden fall in a blunt body's drag coefficient over a narrow range of [Reynolds number](#reynolds-number), as the flow along its surface turns turbulent and stays attached further round. For a cylinder lying across the flow it comes at a Reynolds number of a few hundred thousand. For streamers, Carruthers and Filippone report a sudden drop near 7.2e5, which they put down to a change in how the streamer oscillates. hpr's recovery models include none. See [Recovery](physics/recovery.md#tumble).

## Drift

How far the wind carries a rocket sideways while it descends, in metres. In the comparison with
RocketPy it is the horizontal distance from where the descent starts to the landing point, with an
east and a north part. In still air a small drift remains from the
[Coriolis acceleration](#coriolis-acceleration): 0.19 m for Valetudo's descent. See
[Recovery](physics/recovery.md#against-rocketpy).


## Drogue and main

The two parachutes of a dual-deployment recovery. The small drogue opens at or near apogee, so the
rocket falls fast but steadily; the large main opens lower down for a slow landing, and in hpr it
can release (cut away) the drogue once it is fully open. RocketPy's Calisto example flies a drogue
of 1.0 m² [drag area](#drag-area) and a 10 m² main that opens at 800 m. See
[Recovery](physics/recovery.md#triggers-lag-and-release).


## Dynamic pressure

The pressure of the oncoming air due to its motion, `q = ½ ρ V²`, with `ρ` the air density and `V`
the airspeed, in pascals (Pa). Every aerodynamic force is `q` times a reference area times a
coefficient, so forces grow with the square of airspeed. Near apogee, where the rocket is slow, `q`
is small. See [Frames](physics/frames.md#aerodynamic-angles) and
[Rigid-body flight](physics/flight.md#aerodynamics-in-flight).


## ECEF (Earth-centred, Earth-fixed)

The x, y, z frame that turns with the Earth. Its origin is the Earth's centre of mass, `+Z` points
along the rotation axis to the north pole, `+X` through the prime meridian at the equator, and `+Y`
to 90° E. hpr converts between it and latitude, longitude and height on the [WGS 84](#wgs-84)
ellipsoid. See [Frames](physics/frames.md#earth-centred-earth-fixed-ecef) and
[Geodesy](physics/geodesy.md).


## Effective exhaust velocity

A motor's thrust divided by its propellant mass flow, `c = F/ṁ`, in m/s. hpr holds it constant
through the burn, `c = I/m_p` (total impulse over propellant mass), so propellant burns in
proportion to the impulse delivered. It also refuses a motor whose `c` falls outside 200 to
5,000 m/s, which catches a propellant mass given in the wrong unit, such as grams for
kilograms; ThrustCurve.org's catalog has a median of 1,867 m/s. See
[Solid motors](physics/motor.md#propellant-consumption).


## Ejection delay

The time from a motor's [burnout](#burnout) to its ejection charge, in seconds. Motor files list
the delays available; `P` means plugged, with no ejection charge, and hpr reads a `0` as "zero or
plugged" rather than as ejection at burnout, because most files mean plugged. A recovery device can
use a motor's delay as its trigger. See [Solid motors](physics/motor.md#delays) and
[Recovery](physics/recovery.md#triggers-lag-and-release).


## Ellipsoidal height

Height above the [WGS 84](#wgs-84) ellipsoid, measured along the ellipsoid's normal, in metres. It
is hpr's internal height, and it is not
[height above sea level](#height-above-sea-level-msl): the two differ by up to about 100 m. See
[Frames](physics/frames.md#earth-centred-earth-fixed-ecef).


## Event

A moment the simulator locates exactly and records, such as liftoff, rail exit, burnout, apogee and
ground hit, and in recovery a trigger, a deployment, a release or a separation. The integrator finds
each as the zero of a function of the state, adding at most about 2e-12 s of error to the
solution's own, and stops there so the flight can change phase. See
[Time integration and events](physics/integration.md#events) and
[Rigid-body flight](physics/flight.md#events-and-termination).


## Example rockets

The rockets hpr is compared on. Calisto, Valetudo, NDRT 2020, Prometheus, Juno III, Cavour and Bella
Lui come from RocketPy's own examples; their designs, as hpr reads them, are in the repository's
[`validation/designs/`][designs] folder. The Recruiter and Barrowman's other rockets are worked
examples from his papers, with their printed values in
[`barrowman-worked-examples.json`][barrowman]. See [Accuracy](accuracy.md).

## Fineness ratio

A nose cone's length divided by its base diameter. A 3:1 tangent ogive has a fineness ratio of 3,
which the pages also write as "fineness 3". See
[Aerodynamics](physics/aero.md#bodies-of-revolution).


## Gate and target

Two ways a validation result is held to a bound. A **gate** fails the test suite when a number
falls outside it, so a change that breaks agreement cannot merge. A **target** is the same kind of
bound, but a miss is only reported, with its explanation in the case file, and never fails the
suite: it is used where neither side of the comparison is known to be right, as for hpr's own drag
against the drag RocketPy's examples ship. Either way the report is committed, so any number that
moves shows up in review. See [Accuracy](accuracy.md#whole-flights-with-each-codes-own-drag).

## Geodetic latitude

Latitude as maps and GPS give it: the angle between the equator's plane and the
[WGS 84](#wgs-84) ellipsoid's normal through the point, positive north. Gravity depends on it: at
the surface it runs from 9.780 m/s² at the equator to 9.832 m/s² at the poles. See
[Frames](physics/frames.md#earth-centred-earth-fixed-ecef) and
[Gravity](physics/gravity.md#formulas).


## Height above sea level (MSL)

Height above mean sea level, as field elevations and soundings give it. hpr queries the atmosphere
and the wind with it, and gets it from [ellipsoidal height](#ellipsoidal-height) by subtracting the
geoid undulation `N`, the height of sea level above the ellipsoid (up to about 100 m). hpr has no
geoid model, so a flight takes `N` at the site as an input. See
[Atmosphere](physics/atmosphere.md#height-datum) and
[Frames](physics/frames.md#earth-centred-earth-fixed-ecef).


## Impulse class

The letter that ranks a motor by its [total impulse](#total-impulse), also called its motor class.
Class C covers 5.01 to 10.0 N·s, each letter after it doubles the top of the range, and the upper
limits are inclusive. See [Solid motors](physics/motor.md#thrust-curve).


## Inflation and filling time

How a parachute opens: from [deployment](#deployment), its drag area grows over the filling time
`t_f` to its full value. hpr can open it at once (RocketPy's model), over a fixed time, or over
Knacke's `t_f = n D₀/v`, with `n` the canopy's fill constant, `D₀` its nominal diameter and `v` the
airspeed. hpr leaves out the drag's overshoot as a canopy fills, and opening at once it ignores how
a light rocket slows while the canopy fills, so the opening load it reports is no safe bound either
way. See [Recovery](physics/recovery.md#inflation).


## Internal momentum

The momentum of the propellant and gas moving inside a burning motor. A thrust curve measured on a test stand already includes its effect. hpr's equations of motion, like RocketPy's, add it again, so it is counted twice; hpr keeps it that way so the two codes can be compared like for like. On Valetudo it adds 21 N to the push at liftoff and changes the burnout speed by at most 0.05 m/s. See [Rigid-body flight](physics/flight.md#equations-of-motion).

## Jet damping

The damping of a rocket's turning by its own exhaust: gas leaving the nozzle carries away some of the rocket's rotation. hpr includes it through RocketPy's equations of motion, so it acts only while a motor burns. See [Rigid-body flight](physics/flight.md#equations-of-motion).

## Launch frame (ENU)

The frame fixed at the launch pad, which the flight's position and velocity are kept in: `x_L`
east, `y_L` north and `z_L` up along the ellipsoid's normal at the pad, hence East-North-Up (ENU).
It turns with the Earth, so the equations add the
[Coriolis acceleration](#coriolis-acceleration). It is a flat plane, so `z_L` is not altitude: 10 km
from the pad the plane is 7.8 m above the ellipsoid. See
[Frames](physics/frames.md#launch-frame-l-east-north-up).


## Liftoff

The moment the rocket starts to move up the rail: the push up the rail, mostly the thrust, first
beats the weight's pull down it and the rail's friction. The motor ignites a
little earlier, at time zero. If the motors burn out first, the flight ends on the pad. See
[Rigid-body flight](physics/flight.md#phases).


## Loft lesson

A mistake found in Loft, the project that came before hpr-sim, such as
[Loft lesson L15](decisions-and-roadmap.md#l15) (a shoulder's drag as its length goes to zero). A test here guards
against each one, or will once its milestone ships. See
[Start here](start-here.md#reading-these-pages) and [Lessons from Loft][lessons].


## Mach cone

Faster than sound, a disturbance, such as a fin's tip, can only affect the air downstream of it
inside a cone that opens backward at the Mach angle, `atan(1/√(M² − 1))`: 30° at Mach 2. hpr
halves the fin's lift inside the cone from each tip. See
[Aerodynamics](physics/aero.md#fins-through-mach-1).


## Mach number

Airspeed divided by the local speed of sound, which the atmosphere gives from the air's
temperature. hpr's normal force and drag both cover Mach 0 to 5, and a flight that reaches Mach 5
stops with an error. Both were checked against a wind tunnel from Mach 0.6 to 4.63, where the drag
reads high at most speeds; the drag was also checked at Mach 0.3 against other programs' curves. See
[Aerodynamics](physics/aero.md#fins-through-mach-1) and
[Aerodynamics](physics/aero.md#drag-through-mach-1).


## Mean aerodynamic chord (MAC)

An average of a fin's chords (its lengths along the airflow, root to tip), weighted so that the long chords count more: `c̄ = (1/A)∫c² dy` over the span, `A` being one fin's area. hpr puts each fin set's [centre of pressure](#centre-of-pressure-cp) a quarter of the way back along it up to Mach 0.8, and moves it aft from there to the centre of the load supersonic linear theory gives. See [Aerodynamics](physics/aero.md#fins-through-mach-1).

## Metric

One number a [validation case](#validation-case) compares between hpr and its reference, such as
the descent time or the drift to the north. Each metric has its own [tolerance](#tolerance), also
called its gate. See [Accuracy](accuracy.md#the-descent-under-a-parachute-against-rocketpy).

## Milestone

A step of the [roadmap][roadmap], the ordered plan of work, labelled like [M1.8](decisions-and-roadmap.md#m1-8)
(transonic and supersonic aerodynamics). The pages link a milestone where they say what it will
add. See [Decisions and the roadmap](decisions-and-roadmap.md).


## Motor designation

A motor's name, such as `F32` or `L1150R`: the [impulse class](#impulse-class) letter, then the
[average thrust](#average-thrust) in newtons. Makers add their own codes around it, such as a
propellant letter (the `R` of `L1150R`), the total impulse in N·s in front (`411I175`) or a delay
after a dash. A RASP `.eng` file's name field is meant to hold only the class and average thrust,
but often holds the full designation. See [RASP `.eng` files](format/eng.md#header-fields-r-header).


## NFPA 1125

The US National Fire Protection Association's code for making model and high-power rocket motors. ThrustCurve.org measures a motor's [burn time](#burn-time) by its rule: from the moment the thrust first reaches 5% of its peak to the moment it last falls to 5%. hpr does the same. See [Solid motors](physics/motor.md#thrust-curve).

## Nominal area

A parachute canopy's reference area, `S₀ = π D₀²/4`, from its nominal diameter `D₀`; it includes
the vent and every other opening. Knacke's canopy drag coefficients, which hpr uses, are on this
area: 0.75 to 0.80 for a flat circular canopy, whose middle, 0.775, is hpr's default. RocketPy's
default parachute coefficient of 1.4 is on a different area, so the two can't be compared directly.
See [Recovery](physics/recovery.md#drag-area).


## Normal force

The sideways aerodynamic force on a rocket flying at an [angle of attack](#angle-of-attack), square
to its axis and in the plane of the airflow. It acts at the
[centre of pressure](#centre-of-pressure-cp), and its coefficient `C_N` is positive in the
direction the crossing air pushes the body. It is what makes a stable rocket
[weathercock](#weathercocking). See [Aerodynamics](physics/aero.md#conventions) and
[Frames](physics/frames.md#aerodynamic-angles).


## Normal gravity

The gravity of a smooth, spinning model Earth: the pull of the [WGS 84](#wgs-84) ellipsoid plus the
centrifugal effect of the Earth's rotation, which changes with latitude and height. By default hpr
applies the full normal-gravity vector at the rocket's position, leaving out the real Earth's local
anomalies, typically within ±1e-4 of it. The standard gravity 9.80665 m/s² is a unit convention,
not a model of local gravity. See [Gravity](physics/gravity.md#what-normal-gravity-is).


## Normal-force slope

How fast the [normal force](#normal-force) coefficient grows with
[angle of attack](#angle-of-attack) at small angles, `C_Nα`, per radian. Each nose, transition and
fin set has its own, and the rocket's is their sum; a pointed nose cone's is 2. The
[centre of pressure](#centre-of-pressure-cp) is the components' positions averaged with these
slopes as weights. See [Aerodynamics](physics/aero.md#conventions).


## Octave band

A range of frequencies, or of wavelengths, whose top is twice its bottom. The turbulence test splits the gust spectrum into octave bands and checks each against the Dryden formula. See [Turbulence](physics/turbulence.md#tests-that-pin-this).

## Opening load

The peak force a parachute puts on the rocket as it opens. Knacke writes it as `F = (C_D S) q C_x X1`: the steady drag at the [dynamic pressure](#dynamic-pressure) `q` at line stretch, times `C_x` for the canopy's overshoot when the load doesn't slow (1.7 for a flat circular canopy), times `X1` for how much the rocket slows while the canopy fills. hpr leaves out the overshoot. With a filling time it already includes the slowing, so `X1` must not be applied on top; a canopy that opens at once has none. So the opening load it reports is no safe bound either way: don't size recovery hardware from it. See [Recovery](physics/recovery.md#the-opening-load).

## OpenRocket

A widely used open-source rocket design and simulation program. hpr may run it as an external
program to compare results, but never reads or copies its source code, whose licence (GPL) is
incompatible with hpr's. The comparison with it is planned for [M2.2](decisions-and-roadmap.md#m2-2), the OpenRocket
milestone.

## Oracle

An independent program run to produce [reference values](#reference-value-and-fixture) for hpr's
tests. Usually it is another simulator: RocketPy 1.13.0 now, and OpenRocket later. Scripts that
evaluate a published formula in high precision, and ThrustCurve.org's own statistics code, serve as
oracles too; all of them live under `validation/oracles/`. See
[Recovery](physics/recovery.md#against-rocketpy) and the [list of simulator oracles][oracles].


## Override

A measured mass, centre of mass or inertia that replaces the value hpr computes, for one part, a part with everything attached to it, or a whole stage. Motors are never covered by one. See [The design tree](physics/design.md#overrides).

## Parallel-axis theorem

The rule for moving a moment of inertia from an axis through a part's own centre of mass to a
parallel axis: add the part's mass times the square of the perpendicular distance between the two
axes. A part on the rocket's centre line adds nothing to the roll inertia this way, only to pitch
and yaw. hpr uses its tensor form to add up the inertias of a rocket's parts. See
[Mass properties](physics/mass.md#frames-and-conventions).

## Power-on and power-off drag

Drag while a motor burns, and while the rocket coasts. Under power, the part of the base the
burning motor covers has no base drag, so hpr subtracts the burning motors' cross-section from the
base area, and a drag table can carry separate power-on and power-off curves. A flight uses
power-on drag while any motor burns. See [Aerodynamics](physics/aero.md#drag).


## Property test

A test that checks a rule on many randomly generated inputs rather than a few chosen ones, such as that every interpolation table passes exactly through its own points. hpr writes them with the `proptest` library. See [Interpolation tables](physics/interpolation.md).

## QUADPACK

A library of routines for computing integrals numerically, published by Piessens and others in 1983 and in the public domain. hpr's adaptive quadrature uses its 15-point Gauss–Kronrod rule, without its extrapolation. See [Adaptive quadrature](physics/quadrature.md).

## Rail exit and rail-exit velocity

Rail exit is the moment the rocket leaves the launch rail and starts to fly free; the rail-exit
velocity is its speed then. hpr keeps the rocket guided until the aft edge of its aft-most rail
button or launch lug passes the top of the rail (RocketPy stops at the forward button), and it
leaves with no rotation, since [tip-off](#tip-off) is not modelled. The slower the exit in a
crosswind, the larger the [angle of attack](#angle-of-attack) just after it, where hpr's models are
least trustworthy. See [Rigid-body flight](physics/flight.md#phases).


## RASAero II

A rocket aerodynamics and flight program. Several of RocketPy's [example rockets](#example-rockets)
carry drag curves labelled as RASAero's, though only Calisto's traces to an export. hpr's drag at
Mach 0.3 is compared with those curves, with the fin shapes and surface finish guessed, because
the curves don't record them. See
[Aerodynamics](physics/aero.md#verification).

## RASP and RockSim files

The two thrust-curve file formats [ThrustCurve.org](#thrustcurveorg) serves. A RASP `.eng` file is plain text, named after RASP, the rocket simulation program it comes from; a RockSim `.rse` file is XML, from the RockSim simulator. hpr reads and writes both. See [Solid motors](physics/motor.md#a-motor-from-a-file).

## Reference area

The area every aerodynamic coefficient of the rocket is divided by, `A_ref = π d²/4`. By default
`d` is the largest body diameter; it can be set to the nose's base diameter or to a given value,
and a drag table with its own reference diameter is rescaled to the rocket's. Coefficients from two
programs compare only on the same reference area. See
[The design tree](physics/design.md#reference-diameter).


## Reference value and fixture

A reference value is a number hpr's result is checked against: a value printed in a source, a
worked example, a closed-form answer or an [oracle](#oracle)'s output. A fixture is a committed
file under `validation/fixtures/` holding reference values and where they came from, written by a
generator under `validation/oracles/` or transcribed from print. Tests read fixtures and never
write them: a reference changes only when its generator runs again. See
[the validation harness][harness].


## Relative and absolute difference

Two ways to say how far a result is from its reference. An absolute difference carries a unit, such as 2e-8 m. A relative difference is the gap as a fraction of the reference value, so 2e-14 relative means two parts in a hundred million million, and a percentage is a relative difference in hundredths. hpr's pages mark relative differences with the word *relative* or a percent sign. See [Accuracy](accuracy.md#how-to-read-the-numbers).

## Reynolds number

The ratio of the air's inertia to its viscosity over a length, `R = V L/ν` (often written Re),
without units, with `V` the airspeed, `L` a length and `ν` the air's kinematic viscosity. For skin
friction hpr takes `L` as the whole rocket, nose tip to the aft end of the last body component, and
treats the flow as fully turbulent. See [Aerodynamics](physics/aero.md#drag).


## RocketPy

An open-source (MIT) rocket flight simulator written in Python, and hpr's main partner for
[code-to-code comparisons](#code-to-code-comparison). hpr runs RocketPy 1.13.0, pinned to one
commit, to produce its reference values. See [Checking a claim](checking-a-claim.md).

## Roughness length

The height above the ground at which the logarithmic wind law's wind falls to zero, written `z₀`. Rougher ground has a larger one: 0.03 m for open flat terrain with grass, and 0.001–0.01 m for mown grass. hpr's `LogLawWind` takes it as `roughness_length_m`. See [Wind](physics/wind.md#models).

## Same-drag and predicted mode

Two ways of comparing hpr's whole flights with another simulator's. In **same-drag** mode both
codes fly one drag coefficient that the comparison declares, so a difference comes from the
equations of motion, the motor or the air, not the drag. In **predicted** mode hpr works out its
own aerodynamics from the rocket's shape, and the other code flies the drag its example ships
(from RASAero, OpenRocket or the team). A difference is then mostly the two drags. Neither drag is
known to be right, so those results are held to a [target, not a gate](#gate-and-target). See
[Accuracy](accuracy.md#whole-flights-against-rocketpy) and
[Accuracy](accuracy.md#whole-flights-with-each-codes-own-drag).

## Scientific notation

Writing a very small or very large number as a power of ten, the way programs print it. The number after the `e` says how many places the decimal point moves, to the left when it is negative: 1e-12 is a millionth of a millionth, and 1e6 is a million. 2²⁰ is 2 multiplied by itself 20 times, about a million. See [Accuracy](accuracy.md#how-to-read-the-numbers).

## Seed

A number that starts a random-number generator. The same seed gives the same sequence of numbers, so a run that uses random numbers, such as [turbulence](#turbulence-dryden), repeats exactly on the same platform (operating system and processor). The program that runs it chooses the seed. On another platform each random draw can differ in its last binary digit, and over a whole flight such differences can grow. See [Turbulence](physics/turbulence.md#generator).

## Separation

A stack coming apart for recovery. At its trigger hpr splits the rocket into bodies (body 0 keeps
the nose), and each descends on its own under its own recovery devices, which it must have. It adds
no impulse and must come after the last burnout; staging under power is planned for
[M1.9](decisions-and-roadmap.md#m1-9). See [Recovery](physics/recovery.md#separation).


## Shoulder

The sleeve at the end of a nose cone or transition that slides into the next body tube. It counts toward the rocket's mass, but it is inside the body, so it adds no aerodynamic force. The drag buildup also uses the word for a transition that widens toward the tail, whose pressure drag is counted like a nose's. See [Aerodynamics](physics/aero.md#bodies-of-revolution).

## SI units

The International System of Units: metres, kilograms and seconds, and units built from them, such as newtons for force and pascals for pressure. hpr works in SI throughout, with angles in radians. Degrees appear only where values come in or go out, as in `Geodetic::from_degrees`. In the code, a quantity's name ends in its unit, such as `mass_kg` or `vertical_speed_m_s`. See [Frames](physics/frames.md#units-and-angles).

## Slender-body theory

A way to work out the air's forces on a long, thin body from how fast its cross-section grows
along its length. It gives a nose as wide as the reference a normal-force slope of 2 per radian whatever its shape, a
transition `2ΔA/A_ref`, and a plain tube nothing, at any Mach number. Barrowman's method uses it
for every body part, and hpr does at every speed; NASA's wind tunnel shows a real body lifting
more past Mach 3. See [Aerodynamics](physics/aero.md#bodies-of-revolution).


## Sounding

A measured or forecast profile of the air against height: pressure, temperature and wind, and
sometimes humidity, as from a weather balloon or a forecast service. hpr can fly one in place of
the [standard atmosphere](#standard-atmosphere), which carries on above its top level. Flights
above a few kilometres need one, because an offset to the standard for field conditions holds all
the way up. See [Atmosphere](physics/atmosphere.md#sounding-and-forecast-profiles).


## Specific impulse

Impulse per unit weight of propellant, `I_sp = I/(m_p g₀)`, in seconds, with `g₀ = 9.80665 m/s²`:
the [effective exhaust velocity](#effective-exhaust-velocity) divided by `g₀`. RockSim `.rse` files
carry it as `Isp`; hpr works with the exhaust velocity instead. See
[Solid motors](physics/motor.md#propellant-consumption) and
[RockSim `.rse` files](format/rse.md#engine-attributes).


## Stability margin

How far the [centre of pressure](#centre-of-pressure-cp) lies behind the
[centre of gravity](#centre-of-gravity-cg), usually in [calibres](#calibre-caliber). A positive
margin turns the rocket's nose back into the oncoming air when it is disturbed, which in a
crosswind is not quite its flight path ([weathercocking](#weathercocking)). It changes through a
flight as propellant burns and speed changes. hpr doesn't report it yet: you can compute both
centres, as [Your own rocket](your-own-rocket.md) shows, and a margin over the flight comes with
the outputs milestone, [M1.10](decisions-and-roadmap.md#m1-10). See
[Start here](start-here.md#what-doesnt-work-yet).


## Stage

A section of a rocket's stack in the design tree, listed forward to aft. A [separation](#separation) splits the rocket at the boundary between two stages, so a rocket that stays in one piece needs only one stage. Stages don't yet fire in sequence: every motor ignites at time zero until [M1.9](decisions-and-roadmap.md#m1-9), the staging and clusters milestone. See [The design tree](physics/design.md#the-tree).

## Stall

The pages use the word in two ways. In aerodynamics it is the loss of lift at a large
[angle of attack](#angle-of-attack); hpr models none, so it overstates forces at large angles. In
the flight engine it is the rocket's speed along the rail falling to zero, which ends a flight as
`StalledOnRail` when it happens after burnout. See
[Aerodynamics](physics/aero.md#validity-and-open-questions) and
[Rigid-body flight](physics/flight.md#events-and-termination).


## Standard atmosphere

An agreed model of the air's temperature, pressure and density against height. hpr uses the 1976
U.S. Standard Atmosphere from −5 to 86 km (288.15 K and 101,325 Pa at sea level). It can be offset
to match conditions at the field, or replaced by a [sounding](#sounding). See
[Atmosphere](physics/atmosphere.md#the-1976-standard-5-km-to-86-km).


## Standard error

The scatter expected by chance in an average taken from a random sample; it shrinks as the sample grows. hpr's turbulence test requires each band's average to lie within 4 standard errors of the Dryden formula. See [Turbulence](physics/turbulence.md#tests-that-pin-this).

## Station

A position along the rocket, in metres aft of the nose tip, the way design files give positions.
Station `s` is `z_B = −s` in the [body frame](#body-frame). See
[The design tree](physics/design.md#stations-and-the-body-origin).


## Stiff problem

A problem in which some motion is so fast, and so strongly damped, that a method such as
[Dormand–Prince](#dormandprince-and-rk4) has to take tiny steps to stay stable. hpr doesn't detect
stiffness: the run stops with an error when its steps get too small or too many. See
[Time integration](physics/integration.md#defaults-and-limits).

## Stop time

A time the integrator steps to exactly rather than across, because the equations change there:
thrust-curve points, burnouts, deployments and the end of a canopy's filling. A Runge–Kutta step
across such a jump loses most of its accuracy. See
[Time integration and events](physics/integration.md#stop-times-and-discontinuities).


## Streamer

A long strip of fabric used instead of a parachute to slow a rocket's fall. Its drag is taken on
its one-side area, length times width. hpr's default model reads 9% fast on the one flat streamer
in Kidwell's drop tests, and models no pleats, so it predicts a faster descent for a pleated one.
See [Recovery](physics/recovery.md#streamers).


## Supersonic linear theory

The small-angle theory of thin surfaces faster than sound: a flat plate at an angle `α` feels a
pressure proportional to `α/√(M² − 1)`, the same all along its chord. hpr uses it for fins from
the speed where it holds (about Mach 1.2 or later, set by each fin's sweep and shape). See
[Aerodynamics](physics/aero.md#fins-through-mach-1).


## Surface layer

The air nearest the ground, where friction with the ground sets how fast the wind grows with height. hpr's power-law and log-law winds describe it, but keep growing above it, so winds aloft should come from a table of levels. See [Wind](physics/wind.md#models).

## Tangent ogive

A nose cone whose sides are a circular arc that meets the body tube without a kink: its slope is zero at the base. A 3:1 tangent ogive is three times as long as its base is wide, a [fineness ratio](#fineness-ratio) of 3. hpr models it as the secant ogive whose arc radius equals the tangent radius. See [Shapes](physics/shapes.md#profiles).

## Thrust curve

A motor's thrust against time since ignition: (time, thrust) points joined by straight lines, read
from a RASP `.eng` or RockSim `.rse` file. hpr starts it from zero thrust at ignition, takes the
thrust as zero from the last point on, and treats two points at the same time as a sudden step. See
[Solid motors](physics/motor.md#thrust-curve).


## ThrustCurve.org

A public database of motor thrust curves and data. hpr bundles 32 of its curves, and runs its
statistics code to check its own. A file downloaded from it can be read and flown
([A motor from a file](physics/motor.md#a-motor-from-a-file)). See [Solid motors](physics/motor.md).

## Tip-off

The unwanted turn a rocket picks up as it is let go. Leaving a launch rail, it pivots about its last
guide; the recovery page uses the word for a separation too. hpr models neither: a rocket leaves
the rail with no rotation, a separated body starts with no spin of its own, and no milestone plans
either yet. See [Rigid-body flight](physics/flight.md#phases) and
[Recovery](physics/recovery.md#separation).


## Tolerance

How far a result may be from its reference and still pass, such as the 3% every parachute-descent
number is held to. Each test and each [validation case](#validation-case) states its own, next to
the reason for its size. The integrator's `rtol` and `atol` are tolerances in a second sense: how
much error each [adaptive time step](#adaptive-time-step) may make. See the
[validation report][report] and [Time integration and events](physics/integration.md#methods).


## Total impulse

A motor's total push, the area under its [thrust curve](#thrust-curve), `I = ∫ F dt`, in
newton-seconds (N·s). It sets the [impulse class](#impulse-class). hpr integrates the straight-line
curve exactly. See [Solid motors](physics/motor.md#thrust-curve).


## Transonic and supersonic

Flight near the speed of sound (transonic) and above it (supersonic), where shock waves change
the drag and the lift. Since [M1.8a](decisions-and-roadmap.md#m1-8a) hpr carries the normal force
and centre of pressure through both, to Mach 5: supersonic linear theory for the fins, joined to
the subsonic method between Mach 0.8 and the speed where that theory holds. Since
[M1.8b1](decisions-and-roadmap.md#m1-8b1) (drag through Mach 1) its drag goes to Mach 5 too:
Niskanen's semi-empirical method, with the [wave drag](#wave-drag) of noses and shoulders from
closed forms and from Stoney's 1961 NASA measurements. See
[Aerodynamics](physics/aero.md#fins-through-mach-1) and
[Aerodynamics](physics/aero.md#drag-through-mach-1).

## Tumble recovery

Recovery with nothing deployed: the body falls broadside, tumbling, and its own drag slows it. hpr
takes its drag area from the OpenRocket technical documentation's fit to the fin and side-profile
areas, which comes out −10% to +19% off that source's own drop tests. hpr doesn't decide by itself
when a body tumbles: you give the tumble a trigger. See [Recovery](physics/recovery.md#tumble).


## Turbulence (Dryden)

Random gusts on top of the steady wind. hpr models them with the Dryden spectra of MIL-F-8785C, a
military aircraft specification, from a seeded generator that repeats bit for bit for the same seed
on one platform. No flight uses turbulence yet, none is planned
([issue #39](https://github.com/nrdptel/hpr-sim/issues/39)), and Dryden is unvalidated for
rockets. See [Turbulence](physics/turbulence.md).


## Validation case

One comparison the validation suite runs: what to fly, which numbers (metrics) to compare, each
with its own [tolerance](#tolerance), and against which
[reference values](#reference-value-and-fixture). `cargo xtask validate` runs every case and writes
the committed [validation report][report], with every number and its verdict. See
[the validation harness][harness].


## Verification and validation

Verification checks that the code does what its model says, against closed-form answers, printed
tables and worked examples. Validation checks that the model matches the real world. hpr ranks its
evidence in [four kinds][levels], named as on Accuracy: *analytic* (exact answers), *published
source* (printed tables and worked examples), *another code*
([code-to-code comparison](#code-to-code-comparison)), and *real flights*. See
[Accuracy](accuracy.md).


## Wave drag

The drag from the shock waves that form on a rocket at and above the speed of sound. Air meeting a
nose, a shoulder that widens or a fin's leading edge passes through a shock, which raises the
pressure pushing back on the surface. It is much of the steep rise in drag near Mach 1. hpr has no
separate term for it: it is part of the pressure drag of each nose, shoulder, step and fin, which
Niskanen's 2009 method carries through Mach 1. See
[Aerodynamics](physics/aero.md#drag-through-mach-1).


## Weathercocking

A stable rocket turning into the wind it feels. Off the rail in a crosswind, the airflow meets the
rocket partly from the side, and the [normal force](#normal-force), acting behind the centre of
gravity, swings the nose toward it, so the rocket climbs upwind. In hpr's test, a 5 m/s wind from
the west puts Valetudo's apogee 86 m upwind. See
[Rigid-body flight](physics/flight.md#verification).


## WGS 84

The World Geodetic System 1984: the model of the Earth's shape and gravity that GPS uses. Its
ellipsoid has an equatorial radius `a = 6378137.0 m` and a flattening `1/f = 298.257223563`. hpr's
latitudes, heights and [normal gravity](#normal-gravity) are all on it. See
[Geodesy](physics/geodesy.md#wgs-84-ellipsoid).


## Wind direction

hpr gives the wind's direction the meteorological way: where it blows *from*, clockwise from true
north, so a wind from the west (3π/2 rad, 270°) blows toward the east. RocketPy's wind heading is
where it blows *toward*, 180° from this. The wind itself is the air's velocity in the
[launch frame](#launch-frame-enu)'s east and north axes. See [Wind](physics/wind.md#conventions).

[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[decisions]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
[harness]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md#the-harness-m21a
[barrowman]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/barrowman-worked-examples.json
[designs]: https://github.com/nrdptel/hpr-sim/tree/main/validation/designs
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[levels]: accuracy.md#four-kinds-of-evidence
[oracles]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md#reference-simulators-oracles
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
