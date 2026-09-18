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
The simulator is being compared against RocketPy, and will be compared against
[OpenRocket](https://openrocket.info/) and real flights, with the results committed to the
repository.

For now it covers only commercial off-the-shelf (COTS) solid rocket motors.

## What works today

These parts are built and tested. Each page gives its sources, and most say what they leave out.

| part | what it does | pages |
|---|---|---|
| Earth | Gravity from WGS 84 (the model of the Earth's shape and gravity that GPS uses), varying with latitude and height; the Earth's rotation; launch-site coordinates | [Frames](physics/frames.md), [Geodesy](physics/geodesy.md), [Gravity](physics/gravity.md) |
| Air | The 1976 US Standard Atmosphere up to 86 km, with temperature offsets, humidity, and soundings (measured or forecast profiles of pressure, temperature and wind against height) | [Atmosphere](physics/atmosphere.md) |
| Wind | Constant, layered, power-law and logarithmic wind profiles; seeded turbulence, which repeats exactly for the same seed on the same platform | [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| Motors | Reads `.eng` and `.rse` thrust curves; thrust, mass, centre of gravity and inertia through the burn; 32 bundled curves | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| Rocket | Nose cones, body tubes, transitions, fins and other parts, their materials, the whole rocket's mass properties, and design checks | [Design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| Aerodynamics | The centre of pressure (where the aerodynamic force acts; its distance behind the centre of gravity is the stability margin), the normal force (the sideways force when the rocket flies at an angle to the airflow, its *angle of attack*) and drag. Documented up to Mach 0.8, for small angles of attack | [Aerodynamics](physics/aero.md) |
| Flight | The launch rail, powered flight and coast to apogee, with an adaptive time step and events such as burnout and apogee | [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md) |
| Recovery | Parachutes, streamers and tumbling, the drift they carry the rocket downwind, and a rocket that separates into bodies that each descend on their own | [Recovery](physics/recovery.md) |

## What doesn't work yet

- **Nothing at or above Mach 1.** A flight that reaches Mach 1 stops with an error. From Mach 0.8 to
  1 the aerodynamics are unvalidated extrapolations. Nose and shoulder pressure drag is held at its
  low-speed value, so from about Mach 0.6 it reads low against its own source's high-subsonic
  correction: for a 3:1 tangent ogive nose, by 4–5% of the drag coefficient at Mach 0.8. Transonic
  and supersonic aerodynamics are planned for [M1.8][roadmap], the second aerodynamics milestone.
- **Small angles of attack only.** Nothing models stall, yet a flight uses the same models at
  every angle, so results near rail exit in a strong crosswind, and near apogee, are the least
  trustworthy.
- **No staging under power, clusters or air starts** ([M1.9][roadmap]). A rocket can separate for
  recovery, after the motor has burnt out.
- **Some effects are left out of a flight:**
  - roll forcing and roll damping (the torques that spin a rocket up and slow its spin), planned
    for [M1.8][roadmap];
  - tip-off (the rocket pitching as it leaves the rail), thrust misalignment, and turbulence (the
    model exists, but a flight doesn't use it), none of which a milestone plans yet
    ([issue #39](https://github.com/nrdptel/hpr-sim/issues/39) tracks turbulence);
  - the shock load when a parachute opens, and the canopy's overshoot of its steady drag
    ([Recovery](physics/recovery.md)).
- **No stability margin output yet.** You can compute the centre of pressure and the centre of
  gravity; a margin over the flight comes with [M1.10][roadmap], the outputs milestone.
- **No way to use it without writing Rust.** A simpler library interface ([M4.1][roadmap]), a
  command-line tool ([M4.2][roadmap]), Python ([M4.3][roadmap]) and OpenRocket `.ork` import
  ([M3.1][roadmap]) are planned. Until then, [Getting started](getting-started.md) flies a first
  rocket with a short Rust program.
- **No Monte Carlo, optimization, flight-log analysis or app.** They are on the
  [roadmap][roadmap].

## How far to trust it

[Accuracy](accuracy.md) gathers every result so far, gaps included. In brief:

- **No whole flight has been validated.** hpr's apogee, top speed and landing point have not yet
  been compared with another simulator's or with a real flight's. That is the next work: whole
  flights against RocketPy ([M2.1b2][roadmap]), then OpenRocket ([M2.2][roadmap]) and real
  flights ([M2.3][roadmap]).
- **The descent under a parachute matches RocketPy's.** Five of RocketPy's example rockets start
  from the same state near apogee in both codes, with the first parachute opening at once, the same
  drag areas and wind, RocketPy's random noise off, and RocketPy's gravity formula and wind
  interpolation. hpr's descent time, vertical landing speed and drift from that start agree with
  RocketPy's within 3% on all 30 numbers compared; the largest difference is +2.865%. That shows the
  two codes agree on the descent physics, not that either matches a real flight. The committed
  [validation report][report] has every number, and [Recovery](physics/recovery.md) explains the
  comparison.
- **Each model is tested on its own**: against exact answers, and where its source prints tables
  or worked examples, against those; several parts also against RocketPy. Each test states its
  tolerance. The largest known gaps:
  - The aerodynamics were checked at Mach 0 (the normal force and centre of pressure) and at
    Mach 0.3 (drag) only. Drag against the RASAero curves in RocketPy's examples, whose fins and
    finish were guessed, is within 10% in four of seven cases. It is 18% low for Cavour under
    power, and 47% to 50% low for Valetudo, whose table is 1.44 times its own OpenRocket export
    ([Aerodynamics](physics/aero.md)).
  - The normal-force slope of Barrowman's six-fin Recruiter example is 2.87% above his printed
    value, 3.42% on the fins alone ([Aerodynamics](physics/aero.md)).
  - Tumbling drag is −10 to +19% off its source's own drop tests, and a separated body's
    parachute can open at a higher speed than it would for real, because the body falls with no
    drag until then ([Recovery](physics/recovery.md)).

## Reading these pages

New here? [Getting started](getting-started.md) builds hpr-sim and flies a first rocket, and
[How a flight is simulated](how-a-flight-is-simulated.md) follows a flight from the pad to the
ground, linking the page for each model on the way.

Each model page opens with *In short*: what it models, its sources, how well it is validated and
what it leaves out. Below that, it names the code that implements the model, the sources it follows
and the tests that pin it. Sources are cited by a short key in square brackets, such as **[NGA]**,
with the full reference near the top of the page. Equations are written in plain text, such as the
normal gravity on the ellipsoid, `γ = γ_e (1 + k sin²φ)/√(1 − e² sin²φ)`, so they read the same
here, on GitHub and in the code's documentation.

Terms are defined in the [Glossary](glossary.md), and
[Checking a claim](checking-a-claim.md) shows how to trace any number to its source, its test and
its validation.

The code itself is documented in [the API reference](api.md), which Rust's documentation tool
generates from the source. It lists every public type and function, and each crate's front page
links back to the pages here that explain its models.

Three kinds of label link to the project's records on GitHub, which
[Decisions and the roadmap](decisions-and-roadmap.md) introduces:

- A **milestone**, such as [M1.8][roadmap], is a step of the [roadmap][roadmap], the ordered plan
  of work.
- A **decision record**, such as [ADR-011][adr-011], explains a significant choice and the
  alternatives that were considered. All of them are in the [decision log][decisions].
- A **Loft lesson**, such as [Loft lesson L15][lessons], is a mistake found in Loft, the project
  that came before hpr-sim. A test here guards against it, or will once its milestone ships. They
  are listed in [Lessons from Loft][lessons].

Every page's source is a Markdown file in the repository's
[`docs/` folder](https://github.com/nrdptel/hpr-sim/tree/main/docs). The pencil icon at the top of
a page opens its source in GitHub's editor, where you can propose a fix.

[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[decisions]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
