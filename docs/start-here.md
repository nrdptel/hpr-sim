# Start here

**hpr-sim is a flight simulator for hobby and high-power rockets, and it is not ready to rely on
yet.** This site explains what it models, where each model comes from, and how well each one has
been checked. It is written for a rocketeer who knows some physics and some code, but not this
project. This page covers what hpr-sim is, what works today, what doesn't yet, how far to trust
it, and how to read the other pages.

> **Every number hpr-sim produces is an estimate from a model, not a measurement, and never a
> go/no-go verdict.** Your motor's printed data and your range safety officer are authoritative.

## What hpr-sim is

hpr-sim simulates a rocket's flight from the launch rail to landing. It is a
[6-DOF](glossary.md#6-dof-six-degrees-of-freedom) simulator: it follows all six degrees of freedom,
the rocket's position along three axes and its rotation about three. Meanwhile the motor burns, and
the rocket's mass, [centre of gravity](glossary.md#centre-of-gravity-cg) and inertia (how hard it is
to turn) change.

It is a Rust library first, meant to be built into other programs, as
[RocketPy](glossary.md#rocketpy) is. RocketPy is an open-source rocket flight simulator, written in
Python and used as a Python library. A command-line tool, Python bindings, design-file import and a
graphical app are planned for hpr-sim. None of them exists yet.

It is also built to be checked. Every model cites a published source, and tests pin every model.
The simulator is being compared against RocketPy, and will be compared against
[OpenRocket](https://openrocket.info/) and real flights, with the results committed to the
repository.

For now it covers only commercial off-the-shelf solid rocket motors
([COTS motors](glossary.md#cots-motor)), the kind you buy from a manufacturer.

## What works today

These parts are built and tested. Each page gives its sources, and most say what they leave out.

| part | what it does | pages |
|---|---|---|
| Earth | Gravity from [WGS 84](glossary.md#wgs-84) (the model of the Earth's shape and gravity that GPS uses), varying with latitude and height; the Earth's rotation; launch-site coordinates | [Frames](physics/frames.md), [Geodesy](physics/geodesy.md), [Gravity](physics/gravity.md) |
| Air | The 1976 US [Standard Atmosphere](glossary.md#standard-atmosphere) up to 86 km, with temperature offsets, humidity, and [soundings](glossary.md#sounding) (measured or forecast profiles of pressure, temperature and wind against height) | [Atmosphere](physics/atmosphere.md) |
| Wind | Constant, layered, power-law and logarithmic wind profiles; [turbulence](glossary.md#turbulence-dryden) (random gusts) from a random-number generator started from a [seed](glossary.md#seed), a number you choose: the same seed gives exactly the same gusts every time on the same platform (operating system and processor) | [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| Motors | Reads `.eng` and `.rse` [thrust curves](glossary.md#thrust-curve); thrust, mass, centre of gravity and inertia through the burn; [32 bundled motors](physics/motor.md#the-bundled-motors) | [Solid motors](physics/motor.md), [`.eng` files](format/eng.md), [`.rse` files](format/rse.md) |
| Rocket | Nose cones, body tubes, transitions (tapered sections between tubes of different diameters), fins and other parts, their materials, the whole rocket's mass properties, and design checks | [Design tree](physics/design.md), [Shapes](physics/shapes.md), [Mass properties](physics/mass.md) |
| Aerodynamics | The [centre of pressure](glossary.md#centre-of-pressure-cp) (where the aerodynamic force acts; its distance behind the centre of gravity is the [stability margin](glossary.md#stability-margin)), the [normal force](glossary.md#normal-force) (the sideways force when the rocket flies at an angle to the airflow, its [angle of attack](glossary.md#angle-of-attack)) and drag. Documented up to [Mach](glossary.md#mach-number) 0.8, for small angles of attack | [Aerodynamics](physics/aero.md) |
| Flight | The launch rail, powered flight and coast to apogee, with an [adaptive time step](glossary.md#adaptive-time-step) and [events](glossary.md#event) such as burnout and apogee | [Rigid-body flight](physics/flight.md), [Time integration](physics/integration.md) |
| Recovery | Parachutes, [streamers](glossary.md#streamer) and [tumbling](glossary.md#tumble-recovery), the [drift](glossary.md#drift) they carry the rocket downwind, and a rocket that [separates](glossary.md#separation) into bodies that each descend on their own | [Recovery](physics/recovery.md) |

## What doesn't work yet

These are the gaps you are most likely to meet. Each model page lists what its own model leaves
out.

### Speed and angle of attack

- **Nothing at or above Mach 1.** A flight that reaches Mach 1 stops with an error.
  [Transonic and supersonic](glossary.md#transonic-and-supersonic) aerodynamics are planned for
  [M1.8](decisions-and-roadmap.md#m1-8), the second aerodynamics milestone.
- **High subsonic speeds are shaky.** From Mach 0.8 to 1 the aerodynamics are extrapolations: the
  models are used beyond the speeds they are documented for, and nothing has checked them there.
  Below that, one part of the drag reads low from about Mach 0.6:
  - The air's pressure on the nose, and on any transition where the body widens, adds drag. hpr
    holds that drag at its low-speed value.
  - Its source, Niskanen's 2009 thesis, has it rise toward Mach 1. hpr doesn't add that rise yet.
  - For a 3:1 [tangent ogive](glossary.md#tangent-ogive) nose (three times as long as it is wide,
    with curved sides that meet the body tube without a kink), the
    [drag coefficient](glossary.md#drag-coefficient) reads low by 4–5% at Mach 0.8
    ([Aerodynamics](physics/aero.md#drag-limits)).
- **Small angles of attack only.** Nothing models [stall](glossary.md#stall), the loss of lift at
  a large angle of attack, yet a flight uses the same models at every angle. So results near rail
  exit in a strong crosswind, and near apogee, are the least trustworthy.

### Staging, two-stage rockets, clusters and air starts

- **No staging, so no true two-stage flight, and no delayed ignition or air starts**
  ([M1.9](decisions-and-roadmap.md#m1-9), the staging and clusters milestone). Every motor in a
  design ignites at the start of the flight, time zero.
  - A [cluster](glossary.md#cluster), several motors burning side by side, flies that way: hpr adds
    up their thrust, and a motor off the rocket's centre line adds a turning moment. No test or
    comparison has checked a cluster flight yet.
  - A two-stage design also flies with every motor lit at time zero. That is not a staged flight,
    and hpr doesn't warn about it.
  - An [air start](glossary.md#air-start), a motor lit after liftoff, isn't possible yet.
  - A rocket can [separate](glossary.md#separation) into parts for recovery, but only after the
    last motor has burnt out.

### Effects left out

- **Some effects are left out of a flight, or approximated:**
  - roll forcing and roll damping (the torques that spin a rocket up and slow its spin), planned
    for [M1.8](decisions-and-roadmap.md#m1-8), the second aerodynamics milestone;
  - [tip-off](glossary.md#tip-off) (the rocket pitching as it leaves the rail), thrust
    misalignment (a motor pushing slightly off the rocket's axis), and turbulence (the model
    exists, but a flight doesn't use it), none of which a milestone plans yet
    ([issue #39](https://github.com/nrdptel/hpr-sim/issues/39) tracks turbulence);
  - the shock load when a parachute opens, and the canopy's overshoot of its steady drag
    ([Recovery](physics/recovery.md));
  - the [internal momentum](glossary.md#internal-momentum) of the burning propellant, which is
    counted twice, as RocketPy counts it: a thrust curve measured on a test stand already includes
    its effect, and the equations of motion add it again. On Valetudo, the rocket that
    [Getting started](getting-started.md) flies, it adds 21 N to the push at liftoff and changes
    the burnout speed by at most 0.05 m/s
    ([Rigid-body flight](physics/flight.md#equations-of-motion)).

### Outputs and ways to use it

- **No stability margin output yet.** You can compute the centre of pressure and the centre of
  gravity, and [Your own rocket](your-own-rocket.md) shows how to get the margin from them. A
  margin tracked through the flight comes with [M1.10](decisions-and-roadmap.md#m1-10), the outputs milestone.
- **No way to use it without writing Rust.** A simpler library interface ([M4.1](decisions-and-roadmap.md#m4-1)), a
  command-line tool ([M4.2](decisions-and-roadmap.md#m4-2)), Python ([M4.3](decisions-and-roadmap.md#m4-3)) and OpenRocket `.ork` import
  ([M3.1](decisions-and-roadmap.md#m3-1)) are planned. Until then, [Getting started](getting-started.md) flies a first
  rocket with a short Rust program, and [Your own rocket](your-own-rocket.md) builds a design of
  your own.
- **No Monte Carlo (flying many copies of a flight with randomly scattered inputs), optimization,
  flight-log analysis or app.** They are on the [roadmap][roadmap].

## How far to trust it

[Accuracy](accuracy.md) gathers every result so far, gaps included. In brief:

- **Whole flights match RocketPy's in height, speed and time when both codes fly the same drag.**
  Five of RocketPy's example rockets agree within 3% on apogee, speeds, burnout and flight time
  ([M2.1b2](decisions-and-roadmap.md#m2-1b2), the whole-flight comparison). So does where they
  go, except for two rockets that leave the rail slowly into a wind, where hpr's body lift, which
  RocketPy leaves out, moves the drift by up to 43%
  ([Accuracy](accuracy.md#whole-flights-against-rocketpy)). With hpr's own drag, against RocketPy
  flying the drag its examples ship, hpr's heights differ from RocketPy's by −0.604% to +10.322%,
  the larger gaps where its drag is well below the example's
  ([Accuracy](accuracy.md#whole-flights-with-each-codes-own-drag)).
  No flight has been compared with OpenRocket ([M2.2](decisions-and-roadmap.md#m2-2)) or a real
  one ([M2.3](decisions-and-roadmap.md#m2-3)).
- **The descent under a parachute matches RocketPy's.** The comparison flies the descents of five
  of RocketPy's [example rockets](glossary.md#example-rockets) in both codes:
  - Each starts from the same state near apogee, with the first parachute opening at once.
  - Both codes get the same [drag areas](glossary.md#drag-area) and wind.
  - RocketPy's parachutes can add random noise, which would make each run differ; it is switched
    off.
  - hpr uses RocketPy's formula for gravity, and RocketPy's way of interpolating the wind, that
    is, of working out the wind between the heights it is given.

  Six numbers are compared for each rocket: the descent time, the mean descent rate, the descent
  rate at landing, and the [drift](glossary.md#drift) in total, to the east and to the north. All
  30 agree with RocketPy's within 3%; the largest difference is +2.865%. That shows the two codes
  agree on the descent physics, not that either matches a real flight. The committed
  [validation report][report] has every number, and [Recovery](physics/recovery.md) explains the
  comparison.
- **Each model is tested on its own**: against exact answers, and where its source prints tables
  or worked examples, against those; several parts also against RocketPy. Each test states its
  [tolerance](glossary.md#tolerance). The largest known gaps:
  - The aerodynamics were checked at Mach 0 (the normal force and centre of pressure) and at
    Mach 0.3 (drag) only.
  - The drag was compared with drag curves that come with RocketPy's example rockets, labelled as
    [RASAero II](glossary.md#rasaero-ii)'s (another rocket aerodynamics program). The curves don't
    record the fins' edges or the surface finish, so hpr's copies of the designs follow a declared
    guess. hpr is within 10% in four of the seven cases.
  - hpr's drag is 18% low for Cavour, another of RocketPy's examples, while its motor burns
    ([power-on drag](glossary.md#power-on-and-power-off-drag)); the cause is not known yet.
  - hpr's drag is 47% to 50% below Valetudo's example curve. Valetudo is the rocket that
    [Getting started](getting-started.md) flies. Its references disagree with each other, though:
    at Mach 0.3 the example curve gives a [drag coefficient](glossary.md#drag-coefficient) of
    1.05, 1.44 times the 0.728 in an [OpenRocket](glossary.md#openrocket) file of the same rocket.
    Given that OpenRocket file's own surface finish and launch lugs, hpr gives 0.714, 1.9% under
    the file's 0.728 ([Aerodynamics](physics/aero.md#drag-verification)).
  - The Recruiter is a six-fin model rocket that J. S. Barrowman, whose
    [method](glossary.md#barrowmans-method) hpr follows for the normal force, works through in
    his 1970 report Centuri TIR-33. hpr's [normal-force slope](glossary.md#normal-force-slope)
    for it, how fast the sideways force grows with angle of attack, is 2.87% above his printed
    value, and 3.42% on the fins alone. Most of that comes from a different rule for six fins
    ([Aerodynamics](physics/aero.md#verification)).
  - [Tumbling](glossary.md#tumble-recovery) drag is −10 to +19% off its source's own drop tests.
    A separated body's parachute can open at a higher speed than it would for real, because the
    body falls with no drag until then ([Recovery](physics/recovery.md)).
- **You can check it yourself.** [Getting started](getting-started.md#checking-it-yourself) says
  how: run a program that shows how much the drag moves the apogee, trace any number with
  [Checking a claim](checking-a-claim.md), or compare hpr's apogee by hand with your own
  altimeter's or another simulator's.

## Reading these pages

New here? [Getting started](getting-started.md) builds hpr-sim and flies a first rocket, and
[How a flight is simulated](how-a-flight-is-simulated.md) follows a flight from the pad to the
ground, linking the page for each model on the way.

Each model page opens with *In short*: what it models, its sources, how well it is validated and
what it leaves out. Below that, it names the code that implements the model, the sources it follows
and the tests that pin it.

Sources are cited by a short key in square brackets, such as **[N09]** for Niskanen's 2009 thesis
on model rocket simulation, with the full reference near the top of the page.

Equations are written in plain text, so they read the same here, on GitHub and in the code's
documentation. For example, once its drag balances its weight, a rocket under a parachute falls
at the steady speed `v_e = √(2 m g / (ρ C_D S))`, where `m` is its mass, `g` gravity, `ρ` the air's density and
`C_D S` the parachute's [drag area](glossary.md#drag-area)
([Recovery](physics/recovery.md#the-descent)).

Terms are defined in the [Glossary](glossary.md), and
[Checking a claim](checking-a-claim.md) shows how to trace any number to its source, its test and
its validation.

The code itself is documented in [the API reference](api.md), which Rust's documentation tool
generates from the source. It lists every public type and function, and each crate's front page
links back to the pages here that explain its models.

Three kinds of label link to the project's records on GitHub, which
[Decisions and the roadmap](decisions-and-roadmap.md) introduces:

- A **milestone**, such as [M1.8](decisions-and-roadmap.md#m1-8), is a step of the [roadmap][roadmap], the ordered plan
  of work.
- A **decision record**, such as [ADR-011][adr-011], explains a significant choice and the
  alternatives that were considered. All of them are in the [decision log][decisions].
- A **Loft lesson**, such as [Loft lesson L15](decisions-and-roadmap.md#l15), is a mistake found in Loft, the project
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
