# Vision

## Neer's brief, verbatim (2026-09-16; notes about how the work is run are elided)

> [...] i want the best hobby rocketsim [...] i want maximum quality. i want it to have the best
> stack it can think of. i want the base to be rust.
> first gotta build it out like rocketpy because the sim itself is what matters most so no ui or
> anything fancy visually yet. Similar to rocketpy, i do not want this to be constrained and i want
> it to be open so you can use the sim as part of other projects and plug it into whatever you
> want. I also think being able to design a rocket would be a good add. it would also be cool to be
> able to import files from other sims and then export to those formats as well. i also know there
> are some open formats for design files right now but I would also be interested in making our
> own that is better and maybe builds upon a current open one. validation is super important so
> pull down whatever free sims and data is available to validate. internet stuff like weather,
> location, parts names/stock, etc are things I want but I also want to be able to use it fully
> offline on mac, windows, linux. I own this site and this api should give live solid rocket motor
> stock which would be very cool to integrate: https://motor.fusionspace.co/api. A thing I came
> from was university competitions with challenges where you have constraints of various types on
> the rocket and you have to optimize for that. it would be cool to accommodate that. it would also
> be cool to take in flight data and compare it to the sim. if something went wrong with the
> flight, it would be cool if the project could infer what it could be. then eventually i do want a
> ui with inspiration from what already works like openrocket, rasaero, rocketsim, etc.. More
> modern visuals and more 3d stuff and 3d whole flight sims would be cool. Like for the validation
> I told you about earlier, imagine like the sim as a ghost like in mario kart and the real flight
> both on an interactive 3d sim. that would be amazing. then after that, being able to use it on the
> web would be cool and have like all of it client side so you can install the web version as a
> pwa or something maybe. then have a mobile ui and being able to use that offline using the pwa's
> or even developing mobile apps for ios and android. it would be cool if this was different and
> more modern than current solutions, be creative. For right now it would be good to stick to only
> COTs stuff and only solid rocket motors. Like using rust, optimize for a challenge, 3d stuff, the
> web version, mobile versions, stuff like that, keep on that. [...] I plan for everything to be
> open source similar to my other fusionspace projects (https://fusionspace.co/). I know there is
> already a project on the site and on my github called loft that is very similar to this. It is
> just not what I want it to be so I am going to shutdown that project. Feel free to use resources
> from that repo (fusionspace-loft) and my private repo with resources as well (loft-fixtures).

Neer's addendum, verbatim (2026-09-17; the same elision):

> something I did not mention in my original prompt is a heavy emphasis on easy to reach and read
> documentation. [...]

Neer's ideas, verbatim (2026-09-18; the same elision):

> [...] Some things I though of today that would be cool at some point is modeling airbrakes and
> then later on modeling canards. These may be hard though considering the fluid modeling. This one
> may not be as difficult. Support payloads where the weight distribution may change during
> flight, ejected payloads (this may also mean support for ejected nosecones and body tubes). For
> the ejections, it would also be cool to support landing for that as well is its under parachute.
> Another thing that would be cool and I doubt this would be free like I'd like the whole project
> to be is accounts where you can save designs and sim runs. I will say that if we expect this to
> take off a bit, I don't mind paying for my goals to be accomplished. This stuff may be on the
> eventual path anyways but multistage, transitions, clusters, sidepod would be a good add too.

Decisions Neer confirmed at kickoff:

- **Name:** keep `hpr-sim` for now.
- **Attribution:** no AI traces, the same rule as Loft.
- **Repository:** public GitHub repo from day one, with CI on macOS, Windows and Linux.

## Requirements (the brief, turned into checkable items)

| id | requirement | first milestone |
|---|---|---|
| V1 | Rust core; a library-first, RocketPy-class 6-DOF simulator; accuracy is the top priority | M1.x |
| V2 | Open and unconstrained: usable from Rust, a CLI, Python, C and WASM. Every model can be swapped through traits | M4.x |
| V3 | Design a rocket: parametric component model, parts catalog, design checks | M1.4, M5.5, M8.x |
| V4 | Import and export OpenRocket `.ork`, RockSim `.rkt`, RASAero `.CDX1`, RocketPy, and motor files `.eng`/`.rse` | M1.3, M3.x |
| V5 | Our own open design format that improves on the existing ones and builds on `.ork` | M3.3 |
| V6 | Validation against free simulators (RocketPy, OpenRocket) and real flight data, published openly | M2.x |
| V7 | Online weather, site/elevation, parts, and live motor stock from motor.fusionspace.co, all optional | M5.x |
| V8 | Works fully offline on macOS, Windows and Linux | every milestone |
| V9 | Competition challenges: constraint specs, optimizer, live stock and prices | M6.x |
| V10 | Import flight data, compare it with the simulation, and infer what went wrong | M7.x |
| V11 | Later: a modern UI inspired by OpenRocket, RASAero and RockSim, with 3D whole-flight simulation and the sim shown as a "ghost" beside the real flight | M9.x |
| V12 | Later: a client-side web version (installable PWA), then mobile (offline PWA and/or iOS/Android apps) | M9.x |
| V13 | COTS solid motors only, for now | scope rule |
| V14 | Open source, in the style of the other Fusion Space projects | M0.1 |
| V15 | Documentation that is easy to reach and easy to read: one searchable site linked from the README, plain language first, every term defined, every claim traceable to its source, test and validation, so a person can check the work without reading the code | M0.4, then every milestone |
| V16 | Airbrakes, then canards: active drag and roll control, as competitions fly them | M6.4, M6.5 |
| V17 | Ejected nose cones, body sections and payloads, each landed under its own recovery | M1.11 |
| V18 | Payload mass that moves or is released during flight | M1.12 |
| V19 | Multistage, transitions, clusters and side pods | M1.4 (transitions, done), M1.9, M1.13 |
| V20 | Optional accounts that save designs and flights; paid hosting is acceptable if the project takes off, and the rest stays free and offline | M9.5 |

## North stars

1. **Trustworthy numbers.** Every output comes from a cited model, is pinned by tests, and is
   measured against independent references. Where the model is weak, the docs say so.
2. **A simulator you can build on.** A clean core with no I/O and stable, documented extension
   points. Other people's tools should want to embed it.
3. **Fast enough to explore.** Thousands of flights per second, so Monte Carlo, optimization and
   live "what-if" sliders feel instant.
4. **Honest about uncertainty.** A flight is a distribution, not a single number. Dispersion and
   sensitivity analysis are first-class features.
5. **Offline at the pad.** Nothing essential depends on a network.
6. **Readable by people.** The documentation is how a person understands and checks this code. A
   newcomer finds any answer in two clicks and follows it without reading the source.

## Beyond the brief: ideas to consider (keep, drop, or queue each through the roadmap)

- **Flight forensics.** Compare a flight log with its simulation, rank fault hypotheses by
  likelihood, and say which extra data would tell them apart.
- **A drag model that learns.** Each logged flight updates the rocket's drag and mass estimates by
  Bayesian updating, so predictions get sharper with every flight.
- **Buy what flies.** "Which motors in stock this week get my design to 10,000 ft ±2%, under $150?"
  combines the optimizer, live stock, and the challenge rules.
- **Pad-day mode.** Take today's winds aloft, then show a drift ellipse on the field map, a
  suggested delay, and apogee with an error band. Estimates only, never a go/no-go verdict.
- **Sensitivity tornado.** Show which parameters actually move apogee and landing for *this*
  rocket.
- **A public accuracy census.** An auto-generated table of simulator-vs-reference errors across
  every validation case.
- **Git-friendly designs.** A canonical, diffable format with stable component ids. A design can
  also be shared as a compressed URL on the web.
- **Ghost replay data product.** Time-synced sim and real trajectories in a common frame, exported
  now and ready for the future 3D viewer.

## Not now

- Hybrid, liquid and research (EX) motors.
- Active control, until airbrakes (M6.4) and canards (M6.5). Steering to a target point stays out.
- Any GUI work before the core, validation and interop milestones are done (see `ROADMAP.md`).
