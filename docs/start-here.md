# Start here

**hpr-sim is a flight simulator for hobby and high-power rockets, and it is not ready to rely on
yet.** This site explains what it models, where each model comes from, and how well each one has
been checked. It is written for a rocketeer who knows some physics and some code, but not this
project. This page covers what hpr-sim is, what works today, what doesn't yet, how far to trust
it, and how to read the other pages.

> **Every number hpr-sim produces is an estimate from a model, not a measurement, and never a
> go/no-go verdict.** Your motor's printed data and your range safety officer are authoritative.

## What hpr-sim is

hpr-sim simulates a rocket's flight from the launch rail to landing. It is a *6-DOF* simulator: it
follows all six degrees of freedom, the rocket's position along three axes and its rotation about
three, while the motor burns and the rocket's mass, centre of gravity and inertia change.

It is a Rust library first, meant to be built into other programs, in the spirit of
[RocketPy](https://github.com/RocketPy-Team/RocketPy). A command-line tool, Python bindings,
design-file import and a graphical app are planned. None of them exists yet.

It is also built to be checked. Every model cites a published source, and tests pin every model.
The simulator is compared against other simulators, and later against real flights, and the
results are committed to the repository.

For now it covers only commercial off-the-shelf (COTS) solid rocket motors.

## What works today

These parts are built and tested. Each page gives its sources and says what it leaves out.

| part | what it does | pages |
|---|---|---|
| Earth | Gravity from WGS 84 (the model of the Earth's shape and gravity that GPS uses), varying with latitude and height; the Earth's rotation; launch-site coordinates | [Gravity](physics/gravity.md), [Geodesy](physics/geodesy.md), [Frames](physics/frames.md) |
| Air | The 1976 US Standard Atmosphere up to 86 km, with temperature offsets, humidity and measured soundings | [Atmosphere](physics/atmosphere.md) |
| Wind | Constant, layered, power-law and logarithmic wind profiles; seeded turbulence, which repeats exactly for the same seed | [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| Motors | Reads `.eng` and `.rse` thrust curves; thrust, mass, centre of gravity and inertia through the burn; 32 bundled curves | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| Rocket | Nose cones, body tubes, transitions, fins and other parts, their materials, the whole rocket's mass properties, and design checks | [Shapes](physics/shapes.md), [Mass](physics/mass.md), [Design tree](physics/design.md) |
| Aerodynamics | Centre of pressure, normal force (the sideways force at an angle of attack) and drag, below Mach 1 | [Aerodynamics](physics/aero.md) |
| Flight | The launch rail, powered flight and coast to apogee, with an adaptive time step and events such as burnout and apogee | [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md) |
| Recovery | Parachutes, streamers and tumbling, and a rocket that separates into bodies that each descend on their own | [Recovery](physics/recovery.md) |

## What doesn't work yet

- **Nothing at or above Mach 1.** The aerodynamics are only valid below Mach 1, so a flight that
  reaches it stops with an error. Drag also reads low from about Mach 0.6. Transonic and supersonic
  aerodynamics are planned for [M1.8][roadmap], the second aerodynamics milestone.
- **No staging under power, clusters or air starts** ([M1.9][roadmap]). A rocket can separate for
  recovery, after the motor has burnt out.
- **Some effects are left out of a flight:** roll forcing and roll damping, tip-off as the rocket
  leaves the rail, thrust misalignment, and turbulence (the model exists, but a flight doesn't use
  it yet). Each page lists what it leaves out.
- **No way to use it without writing Rust.** A simpler library interface ([M4.1][roadmap]), a
  command-line tool ([M4.2][roadmap]), Python ([M4.3][roadmap]) and OpenRocket `.ork` import
  ([M3.1][roadmap]) are planned. A first runnable example comes with the *Getting started* page
  ([M0.4c][roadmap]).
- **No Monte Carlo, optimization, flight-log analysis or app.** They are on the
  [roadmap][roadmap].

## How far to trust it

- **Each model is checked against its published source**: printed tables, worked examples and
  closed-form results. Its page names the source, and its tests pin the numbers.
- **Whole flights have not been validated yet.** The one comparison so far covers descent only.
  Five of RocketPy's example rockets descend under their own parachutes, starting from the state
  RocketPy recorded. hpr's descent time, landing speed and drift agree with RocketPy's within 3% on
  all 30 numbers compared. The largest difference is 2.87%. That shows the two codes agree on the
  descent physics. It does not show that either one matches a real flight. The results are in the
  committed [validation report][report].
- **Next come** whole flights against RocketPy ([M2.1b2][roadmap]), comparisons with OpenRocket
  ([M2.2][roadmap]), and real flights ([M2.3][roadmap]).

## Reading these pages

Each model page names the code that implements it, the sources it follows and the tests that pin
it. Sources are cited by a short key in square brackets, such as **[NGA]**, with the full reference
near the top of the page. Equations are written in plain text, such as `γ_e = GM/(ab)`, so they
read the same here, on GitHub and in the code's documentation.

Three kinds of label link to the project's records on GitHub:

- A **milestone**, such as [M1.8][roadmap], is a step of the [roadmap][roadmap], the ordered plan
  of work.
- A **decision record**, such as [ADR-011][adr-011], explains a significant choice and the
  alternatives that were considered. All of them are in the [decision log][decisions].
- A **Loft lesson**, such as [Loft lesson L15][lessons], is a mistake found in Loft, the project
  that came before hpr-sim, which a test here now guards against. They are listed in
  [Lessons from Loft][lessons].

Every page's source is a Markdown file in the repository's
[`docs/` folder](https://github.com/nrdptel/hpr-sim/tree/main/docs). The pencil icon at the top of
a page opens it on GitHub.

[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[decisions]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
