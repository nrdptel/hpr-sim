# Decisions and the roadmap

This page is the way into the project's three records: the decision log, the roadmap and the
lessons from Loft. They are files in the repository, not pages of this site, because they are
written as the work happens. Every label you meet on the other pages, such as a decision record
(`ADR-` and a number), a milestone (`M` and a number) or a Loft lesson, links into one of them. For
how hpr behaves today, trust the model pages; the records say why it behaves that way and what
comes next.

## Decision records

A decision record, or ADR (architecture decision record), explains one significant choice: the
problem, the options that were considered, what was chosen and why, and what it costs. Records
are numbered in order and never renumbered. A record is not rewritten when a choice changes; a
new record replaces it and points back. All of them are in the [decision log][decisions].

| record | what it decides | where it shows |
|---|---|---|
| [ADR-000: Kickoff decisions][adr-000] | A Rust library first; physics and validation before interfaces; the licence; a clean room (no GPL source is read); commercial solid motors only | [Start here](start-here.md) |
| [ADR-001: License and workspace layout][adr-001] | `MIT OR Apache-2.0`, and how the code is split into crates | [Start here](start-here.md) |
| [ADR-002: The reference library][adr-002] | How published sources and reference programs are pinned by hash, fetched and checked | [Checking a claim](checking-a-claim.md) |
| [ADR-003: Frames, attitude, geodesy and gravity][adr-003] | The axes and sign conventions, WGS 84 gravity, and the Earth's rotation | [Frames](physics/frames.md), [Gravity](physics/gravity.md) |
| [ADR-004: Atmosphere, wind and turbulence][adr-004] | The standard atmosphere, wind by height above sea level, and seeded turbulence | [Atmosphere](physics/atmosphere.md), [Wind](physics/wind.md), [Turbulence](physics/turbulence.md) |
| [ADR-005: Solid motors][adr-005] | How a thrust curve becomes impulse, mass and inertia over the burn, and which curves are bundled | [Solid motors](physics/motor.md) |
| [ADR-006: Component geometry and mass properties][adr-006] | How each part's shape, walls, fins and material give its mass, centre of gravity and inertia | [Mass properties](physics/mass.md), [Shapes](physics/shapes.md) |
| [ADR-007: The design tree][adr-007] | Where parts sit, automatic sizes, overrides, motor mounts and design checks | [Design tree](physics/design.md) |
| [ADR-008: Subsonic normal force and centre of pressure][adr-008] | The body and fin lift models behind the centre of pressure | [Aerodynamics](physics/aero.md) |
| [ADR-009: Subsonic drag][adr-009] | The drag buildup, surface finishes and drag tables that override it | [Aerodynamics](physics/aero.md) |
| [ADR-010: Time integration][adr-010] | The adaptive integrator, and how events such as burnout and apogee are found | [Time integration](physics/integration.md) |
| [ADR-011: Rigid-body flight][adr-011] | The equations of motion, the launch rail, the phases of a flight and when it stops | [Rigid-body flight](physics/flight.md) |
| [ADR-012: Recovery][adr-012] | Parachute drag areas, when devices open, how they fill, and the descent | [Recovery](physics/recovery.md) |
| [ADR-013: Streamer and tumble drag][adr-013] | Which published data sets give a streamer's and a tumbling body's drag | [Recovery](physics/recovery.md) |
| [ADR-014: Separation][adr-014] | How a rocket splits into bodies that each descend on their own | [Recovery](physics/recovery.md) |
| [ADR-015: The validation harness][adr-015] | How comparisons with other programs are run, gated and reported | [Accuracy](accuracy.md), [Checking a claim](checking-a-claim.md) |
| [ADR-016: The documentation site][adr-016] | This site: how it is built, and the checks every page passes | [Start here](start-here.md) |
| [ADR-017: In short, traced numbers and this page][adr-017] | How every model page opens, how Accuracy's numbers are checked against their sources, and why these records are files rather than pages | [Accuracy](accuracy.md) |
| [ADR-018: Examples and quotes][adr-018] | Every example program runs in CI and must print its committed output, and a page's quote of a file must match it line for line | [Getting started](getting-started.md) |
| [ADR-019: Publishing the site][adr-019] | How this site and the API reference are built together, link each other, and are published from `main` | [The API reference](api.md) |
| [ADR-020: The reader test][adr-020] | How the site was tested on a new reader, and why every milestone and lesson label links a row of plain words on this page | [Decisions and the roadmap](#every-milestone) |
| [ADR-021: Whole flights against RocketPy][adr-021] | What a whole-flight comparison measures and how, why a sixth rocket was added, and how a case declares a limit of hpr's as a known gap | [Accuracy](accuracy.md#whole-flights-against-rocketpy) |
| [ADR-022: Validation in CI, and regenerating references only by hand][adr-022] | How every pull request reruns the validation cases on three operating systems, and why the references change only when a person regenerates them and reviews the diff | [Checking a claim](checking-a-claim.md#rules-that-keep-the-trail-honest) |
| [ADR-023: Predicted mode][adr-023] | How hpr's own aerodynamics are compared with RocketPy flying each example's own drag, and why those results are reported against a target rather than gated | [Accuracy](accuracy.md#whole-flights-with-each-codes-own-drag) |
| [ADR-024: The time-series RMS][adr-024] | How each whole flight's height and speed over time are compared with RocketPy's, on what clock, and why each is held to 3% of its apogee or max speed | [Accuracy](accuracy.md#whole-flights-against-rocketpy) |
| [ADR-025: The calm-air cases][adr-025] | Juno III, Calisto and Bella Lui flown with no wind, and why Juno III's drifts were at first reported but not scored: the two codes free the rocket from the rail at different points | [Accuracy](accuracy.md#whole-flights-against-rocketpy) |
| [ADR-026: The path in wind][adr-026] | Why hpr turned into the wind less than RocketPy: RocketPy's equations took the turning moments about the wrong point during the burn (corrected upstream, and in the comparison), and hpr's body lift, which RocketPy leaves out, pushes a slow rocket downwind | [Accuracy](accuracy.md#whole-flights-against-rocketpy) |
| [ADR-027: The normal force through Mach 1][adr-027] | How the fins' normal force and centre of pressure carry through Mach 1: Barrowman's subsonic method to Mach 0.8, supersonic linear theory once it holds, a straight-line join between, and how they compare with a wind tunnel and with RASAero II | [Aerodynamics](physics/aero.md#fins-through-mach-1) |
| [ADR-028: Drag through Mach 1][adr-028] | How noses, shoulders and steps drag through Mach 1: Niskanen's method with Stoney's measured nose curves, and how the drag compares with NASA's wind tunnel | [Aerodynamics](physics/aero.md#drag-through-mach-1) |
| [ADR-029: Drag against RASAero II through Mach 2][adr-029] | How hpr's drag compares with RASAero II's curves by speed band, why it misses faster than sound, and a second reference with every input known: MIL-HDBK-762's worked example | [Aerodynamics](physics/aero.md#drag-against-rasaero-ii-through-mach-2) |
| [ADR-030: The afterbody faster than sound][adr-030] | A boattail's supersonic wave drag from MIL-HDBK-762's chart held to the Prandtl–Meyer limit, separation on steep boattails, the base pressure behind them, and a lip in a boattail's wake; checked against measured boattails, and why its targets aren't met | [Aerodynamics](physics/aero.md#boattails-faster-than-sound) |
| [ADR-031: Roll from canted fins][adr-031] | Roll forcing from canted fins and roll damping by Barrowman's strip theory with his body factors, the fin's own slope in the damping, and the comparison with NASA's measured roll effectiveness and the Basic Finner's roll damping | [Aerodynamics](physics/aero.md#roll-forcing-and-damping) |
| [ADR-032: Normal-force overrides][adr-032] | Flying RASAero II's normal force and centre of pressure: how its export is read, the slope at 0°, the angles past its last, and why the damping stays hpr's | [Aerodynamics](physics/aero.md#the-normal-force-from-rasaero-ii) |
| [ADR-033: The body faster than sound][adr-033] | The lift a body's cylinder carries behind its nose faster than sound, by the second-order shock-expansion method: the report's tangent body, its limit, and the cone slopes read by hand | [Aerodynamics](physics/aero.md#bodies-faster-than-sound) |
| [ADR-034: The body's supersonic normal force in flight][adr-034] | How a flight uses that method: a table of each part's share every 0.05 in Mach, joined in a straight line from slender-body theory over Mach 1.2 to 1.5 | [Aerodynamics](physics/aero.md#the-body-faster-than-sound-in-a-flight) |
| [ADR-035: Drop the orhelper dependency][adr-035] | The GPL-2.0 wrapper for the OpenRocket jar is removed from the oracle environment, unused; how [the OpenRocket oracle milestone](#m2-2) drives the jar is decided when it starts | [How correctness is proven](https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md) |
| [ADR-036: The Arcas Robin's supersonic body gap][adr-036] | How hpr's body faster than sound is compared with NASA's wind tunnel from now on (as the tunnel measures, at its angles), and why [M1.8e6](#m1-8e6) takes the size of crossflow lift and the boattail's share together | [Aerodynamics](physics/aero.md#checking-the-shock-expansion-method) |
| [ADR-037: Body lift by Jorgensen's crossflow][adr-037] | Body lift's size at every speed from Jorgensen's crossflow drag, how his two `η` figures are combined, a boattail's share faster than sound from Washington and Pettis's measurements, and why [blunt tips moved to M1.8e7](#m1-8e7) | [Aerodynamics](physics/aero.md#body-lift) |
| [ADR-038: Blunt tips by a Newtonian cap][adr-038] | How a nose with a blunt or vertical tip (power-series, Haack, elliptical) flies the shock-expansion method faster than sound: NASA TN D-4865's Newtonian cap and handover, the method started behind it from the tangent cone rather than the report's own start, and how both were checked | [Aerodynamics](physics/aero.md#blunt-tips) |
| [ADR-039: A lip in a boattail's wake carries nothing][adr-039] | Why a short flare at the very base, behind a boattail, gets no normal force faster than sound, what the tunnel's pitching moment says about that, and how far the shelter reaches | [Aerodynamics](physics/aero.md#a-lip-in-a-boattails-wake) |
| [ADR-040: A steep boattail's correlation, and the 15% target judged][adr-040] | How steep a boattail hpr still reads its measured share for, how footnote 8 and the tube behind it were integrated by hand, and where the Arcas Robin's body alone still misses its 15% target | [Aerodynamics](physics/aero.md#the-body-alone-against-the-15-target) |
| [ADR-041: A lip's shelter weighed, not switched][adr-041] | Why a lip rising out of its boattail's wake now moves a rocket between the two supersonic body models smoothly, and how big the switches that remain are | [Aerodynamics](physics/aero.md#a-lip-in-a-boattails-wake) |
| [ADR-042: Cone slopes past Fig. 2's edge, from Sims][adr-042] | Where the slopes for tangent cones of 24° to 30° come from, how they were checked against the chart they extend, and what a cone steeper than 30° still costs | [Aerodynamics](physics/aero.md#bodies-faster-than-sound) |
| [ADR-043: The blunt tip's handover cap][adr-043] | What moving a blunt tip's handover from 24° to the cone tables' 30° would be worth on the report's own sphere-cone, what it does to the march on a nose that flattens fast, and why the cap stays where it is | [Aerodynamics](physics/aero.md#what-the-cap-is-worth) |
| [ADR-044: What the answer follows when it follows the mesh][adr-044] | That it is the surface pressure crossing its tangent cone's, not the method being reduced, that marks a reading whose answer moves with the element count — how far that goes, and what it re-aims issue #108 at | [Aerodynamics](physics/aero.md#what-a-crossing-is-and-what-it-costs) |
| [ADR-045: Where a flare's march stops][adr-045] | That what stops the march is the corner's isentropic turn, not the flare's shock detaching; that the two limits cross near Mach 1.55, so neither bounds the other; and that above Mach 2.13 neither binds — the cone tables' 30° does | [Aerodynamics](physics/aero.md#where-a-flares-march-stops) |
| [ADR-046: Debrief folded in, and analysis that stands on its own][adr-046] | That a universal flight log analyzer becomes part of this project, and that reading a log never needs the simulator: `hpr-flightdata` may not depend on `hpr-sim`, a check enforces it, and comparing a flight with a simulation lives in `hpr-forensics` | [Start here](start-here.md#what-doesnt-work-yet)  What the analyzer is being built from is written up in [the log formats](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/debrief-log-formats.md), [the readings](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/debrief-flight-readings.md) and [what may be ported](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/debrief-porting-boundary.md). |
| [ADR-047: A flare flies the method where its corner's shock is attached][adr-047] | That a flared body now flies the shock-expansion method; that the test for the corner's shock is NACA 1135's wedge limit read at the flow reaching it, the same one a blunt tip's cap already uses; that a steeper flare is read as one of the same radii drawn out to that limit, so nothing jumps across the boundary; and what still switches — a band of flares a third of a millimetre tall the march refuses (read through since [ADR-050][adr-050]) | [Aerodynamics](physics/aero.md#a-flare-through-the-method) |
| [ADR-048: What a marched flare is worth][adr-048] | That TN D-4865 model 2's readings are committed and compared: an 18.5° flare read −1.9% and +7.0% against the wind tunnel at Mach 1.9 and 2.3, +13.4% at 2.96 and about +51% at 3.95 and 4.63, whose flare the report's shadowgraphs show separated; with no reading at all below Mach 1.5289, where drawing the flare out to its shock's limit lands past the march's own | [Aerodynamics](physics/aero.md#what-a-marched-flare-is-worth) |
| [ADR-049: What a step in radius costs][adr-049] | That a step's cost is measured and published rather than modelled: it takes the whole body off the shock-expansion method, worth −8.65% and 1.03 calibres at its threshold and −12.55% and 1.36 calibres at a 2 mm step down, and −11.34% and 1.10 calibres on a boattailed body, whose threshold is 1.3e−13 m rather than 2.7e−11 m; and that stopping the march at the step instead was built, measured and rejected, because the mixed reading lands outside both pure models and misses the boattail's band | [Aerodynamics](physics/aero.md#a-step-in-radius) |
| [ADR-050: A reduced element read by the generalized method][adr-050] | That where the second-order method's exponential form cannot hold — the pressure behind a corner on the far side of its tangent cone's from where its own gradient points — the element is read by the generalized method wherever it has a tangent cone of its own, so a near-flat flare no longer takes the whole body off the method; that the region's two edges are solved from the corner's own state rather than bisected, reproducing all three published angles; and that what is left is the loading's step at the crossing, +0.129% and 0.0051 calibres on the tests' rocket but not bounded by it — +4.3% and 0.19 calibres on a body with a short shoulder, and −2.8% between two adjacent Mach rows | [Aerodynamics](physics/aero.md#a-near-flat-flare) |
| [ADR-051: M3.1 split, and a `.ork` document kept whole][adr-051] | That reading an OpenRocket file is split into four increments, the container and the document first; that the document is read into a tree and interpreted by nobody, because with no schema for `.ork` keeping the whole file is the only way to be sure nothing was dropped; that reading it, writing it and reading it again gives the same document, checked over generated trees and over all 76 corpus files that open; that nesting is counted before the text is parsed, since the XML parser underneath overflows the stack past 120 levels; and that the corpus survey names the two files that are not XML rather than skipping them quietly | [OpenRocket `.ork` design files](format/ork.md) |
| [ADR-052: What a `.ork` value means][adr-052] | That an automatic dimension keeps both its flag and the number OpenRocket last worked out, rather than becoming a hand-typed one; that either name may be read of the two renames that are only renames, because where OpenRocket writes both it agrees with itself on the number and on the frame every time (642 and 109 elements), while two more pairs that look the same are left unread, their newer name carrying a frame the older never does; that a stated zero is a value, which is what an override to no drag at all needs; and that the single flag the three subcomponent-override flags replaced is read as setting all three, out loud, since no file carries both forms | [`.ork` design files](format/ork.md#the-values-inside-the-tags) |
| [ADR-053: The parts on and inside a `.ork` body][adr-053] | That `.ork` angles are degrees, which nothing in the file says and 178 of its 188 non-zero ones show by exceeding a whole turn; that `radialposition` and `radiusoffset` are each read on the parts that carry them, which no element carries both of, so [the question ADR-052 left open][adr-052] need not be answered to read them; that a part this reader cannot shape honestly is left out with its reason rather than guessed at (5 in the corpus); that a tube of no wall carries no mass, which is OpenRocket's own geometry, where a body component's ambiguous zero is read as solid; that an inner tube's automatic outer radius is its parent's bore, resolved in a pass before any ring's bore so no answer depends on sibling order; and that the five surface-finish words take the roughness heights OpenRocket's author published, with `polished` flagged as possibly moved in a newer OpenRocket | [`.ork` design files](format/ork.md#the-parts-on-and-inside-the-body) |
| [ADR-054: An automatic radius with nothing to take][adr-054] | That an automatic body radius with no fixed radius anywhere along its chain takes OpenRocket's default radius, 25 mm, which OpenRocket's maintainers call "the default radius" and OpenRocket 24.12 settles on in a committed oracle run, never the number cached after `auto`, which OpenRocket itself ignores; that hpr departs from OpenRocket only where OpenRocket answers −1 m, a radius no shape can have; that hpr is held to OpenRocket's settled answer rather than its first reading, which settles the Dual parachute example's odd cache in hpr's favour (67 of 67 body radii agree over 18 designs); and that a `<rocket>` holding nothing is a document with no design rather than a design that fails | [`.ork` design files](format/ork.md#when-an-automatic-radius-has-nothing-to-take) |
| [ADR-055: M3.1c split, and the motors a `.ork` flies][adr-055] | That reading a `.ork` file's motors, recovery, stored results and pods is split into four increments. That a motor's thrust curve comes from the file first and hpr's bundled catalog second, matched on manufacturer and designation both. That `none` is a plugged motor and `0` a charge at burnout. And that the rocket flies a configuration only when every motor in it has a curve and lights at launch, on a one-stage rocket read without a warning, because hpr lights every motor at launch and flies one body until staging arrives: 1 of the library's 174 configurations does | [`.ork` design files](format/ork.md#motors-and-their-configurations) |
| [ADR-056: A `.ork` design's recovery and separation][adr-056] | That when each parachute and streamer opens, and when each stage separates, is read as the file wrote it and not yet flown. That the words are OpenRocket's own, measured by a committed probe. That a deploy height is above the ground. And that two choices are left open, in plain view, for the step that flies a `.ork`: a deploy height the rocket never reaches, which OpenRocket never opens, and an automatic drag coefficient | [`.ork` design files](format/ork.md#when-parachutes-open-and-stages-separate) |
| [ADR-057: A `.ork` design's stored simulations][adr-057] | That the simulations OpenRocket last ran on a design are read back as it wrote them: the launch conditions, the summary, and each stage's time series and events. That their units are measured by a committed probe: the rod's angle and direction in degrees, a compass bearing; the wind's direction in radians, where it blows from. And that they are OpenRocket's answers to compare against, not flights hpr makes | [`.ork` design files](format/ork.md#what-openrocket-last-did-stored-simulations) |
| [ADR-058: What a `.ork` holds that hpr does not model][adr-058] | That the parts and sections of a `.ork` hpr does not read — pods, parallel stages, OpenRocket's 3D-view settings, a simulation's plug-ins — are kept whole beside the design, in an extension called `x-openrocket`, at a path that leads back to where each was, so that writing the file back can put them back; that a design missing parts this way says it is reduced; and that the same goes for every tag and attribute no reader asks for, recorded as hpr reads | [`.ork` design files](format/ork.md#what-hpr-keeps-for-writing-the-file-back) |
| [ADR-059: The RocketSerializer cross-check][adr-059] | That hpr's reading of a design's key geometry (the nose cone, transitions, fin sets, where each sits, and the body radius) is held to RocketSerializer's, a second program that reads `.ork` files, with OpenRocket itself run on the same file to settle any difference; that "agrees" means no number of hpr's is apart from both; that a cause is named only where the record proves it; and that an import error is a file that does not read or a design that does not lay out | [`.ork` design files](format/ork.md#checked-against-rocketserializer) |
| [ADR-060: M2.2 split, and the structure's mass held to OpenRocket's][adr-060] | [The OpenRocket comparison](#m2-2) goes mass first, then OpenRocket's mass conventions, the motors OpenRocket flies, flights on public designs, and the corpus. Each design's structure is held to OpenRocket's within 1% in mass and 1% of length in centre of mass, thresholds set before measuring, and every design outside is given its cause. Which of OpenRocket's inertias is roll is measured on a tube worked out by hand | [Mass properties](physics/mass.md#checked-against-openrocket) |

## The roadmap

The [roadmap][roadmap] is the ordered plan of work. It is split into phases, and each phase into
milestones. A milestone's id is `M`, a topic number, a dot and its place in that topic. The topic
is not the phase: 0 is foundations, 1 physics, 2 validation, 3 file formats, 4 library interfaces,
5 online data, 6 Monte Carlo and optimization, 7 flight logs, 8 design help and 9 the app. Phases
mix topics, so [M2.3](#m2-3), the real-flights milestone, sits in Phase 1 beside
[M1.8](#m1-8), the second aerodynamics milestone. A milestone too big to ship at once is split
into increments with a letter, and sometimes a digit after it, such as [M2.1b2](#m2-1b2). Each one
ends with *done when* conditions, and it is checked off only when all of them hold. Every
milestone has a row in [the table below](#every-milestone).

The work runs in this order: the physics and its validation first, then the ways to use it, then
the analysis tools, and an app last.

- [Phase 0: Foundations][phase-0]: the workspace, the reference library, the lessons from Loft
  and this site.
- [Phase 1: Physics core, with validation interleaved][phase-1]: the models on this site, then
  transonic and supersonic aerodynamics, staging, and flight outputs such as the stability margin,
  with comparisons against RocketPy, OpenRocket and real flights along the way.
- [Phase 2: Library surfaces and interop][phase-2]: a simpler interface, a command-line tool,
  Python bindings, and design files in and out.
- [Phase 3: Uncertainty, optimization, challenges][phase-3]: Monte Carlo runs and design
  optimization for competitions.
- [Phase 4: More formats and embeddings][phase-4]: RockSim, RASAero and RocketPy files, and use
  from C and the web.
- [Phase 5: Flight data and forensics][phase-5]: reading flight logs and taking a flight's
  readings off them — planned to work on its own, with no design file and no simulation — and then
  comparing a real flight with its simulation.
- [Phase 6: Design experience][phase-6]: a design assistant for apps to build on.
- [Phase 7: UI, 3D, web, mobile][phase-7]: a desktop app, 3D flight replay, a web app and mobile.

What is done so far, and what is not, is on [Start here](start-here.md#what-works-today).

### Every milestone

Each milestone and increment has a row here, so a milestone label on any page leads to a line of
plain words. The label in the first column opens its phase of the roadmap, where its full plan and
its *done when* conditions are. The status is the roadmap's: the site check fails if a row is
missing or its status disagrees.

| milestone | what it covers | status |
|---|---|---|
| <a id="m0-1"></a>[M0.1][phase-0] | The code workspace, the automated checks (CI) and the licences | done |
| <a id="m0-2"></a>[M0.2][phase-0] | The library of published sources and reference programs, each pinned so results can be reproduced | done |
| <a id="m0-3"></a>[M0.3][phase-0] | The lessons from Loft, the project that came before this one | done |
| <a id="m0-4"></a>[M0.4][phase-0] | This documentation site | done |
| <a id="m0-4a"></a>[M0.4a][phase-0] | The site itself, and the checks on its links and labels | done |
| <a id="m0-4b"></a>[M0.4b][phase-0] | The model pages' *In short*, Accuracy, the Glossary and Checking a claim | done |
| <a id="m0-4c"></a>[M0.4c][phase-0] | Getting started, and How a flight is simulated | done |
| <a id="m0-4d"></a>[M0.4d][phase-0] | Publishing the site and the API reference on the web | done |
| <a id="m0-4e"></a>[M0.4e][phase-0] | A new reader answers ten questions from the site alone, and what they find unclear is fixed | done |
| <a id="m1-1"></a>[M1.1][phase-1] | Vectors and rotations, frames, the Earth's shape and gravity | done |
| <a id="m1-2"></a>[M1.2][phase-1] | The atmosphere and wind | done |
| <a id="m1-3"></a>[M1.3][phase-1] | Solid motors: thrust curves, motor files, and mass through the burn | done |
| <a id="m1-4"></a>[M1.4][phase-1] | The rocket's design and its mass properties | done |
| <a id="m1-4a"></a>[M1.4a][phase-1] | Part shapes and materials, and each part's mass, centre of gravity and inertia | done |
| <a id="m1-4b"></a>[M1.4b][phase-1] | The design tree: where parts sit, motor configurations and design checks | done |
| <a id="m1-5"></a>[M1.5][phase-1] | Aerodynamics below the speed of sound | done |
| <a id="m1-5a"></a>[M1.5a][phase-1] | Normal force and centre of pressure | done |
| <a id="m1-5b"></a>[M1.5b][phase-1] | Drag, and tables that override it | done |
| <a id="m1-6"></a>[M1.6][phase-1] | The six-degree-of-freedom flight | done |
| <a id="m1-6a"></a>[M1.6a][phase-1] | The time integrator, and how events such as burnout and apogee are found | done |
| <a id="m1-6b"></a>[M1.6b][phase-1] | The rigid-body flight: the rail, the equations of motion and the phases of a flight | done |
| <a id="m1-7"></a>[M1.7][phase-1] | Recovery | done |
| <a id="m1-7a"></a>[M1.7a][phase-1] | Parachutes, and the descent under them | done |
| <a id="m1-7b"></a>[M1.7b][phase-1] | Streamers and tumble recovery | done |
| <a id="m1-7c"></a>[M1.7c][phase-1] | Separated bodies, each flown to its own landing | done |
| <a id="m2-1"></a>[M2.1][phase-1] | The validation harness, and comparisons with RocketPy | done |
| <a id="m2-1a"></a>[M2.1a][phase-1] | The harness itself: cases, reference data, tolerances and reports | done |
| <a id="m2-1b"></a>[M2.1b][phase-1] | Whole flights against RocketPy, with both codes given the same drag | done |
| <a id="m2-1b1"></a>[M2.1b1][phase-1] | The script that flies RocketPy's example rockets from pad to landing, as the reference | done |
| <a id="m2-1b2"></a>[M2.1b2][phase-1] | hpr's whole flights compared with that reference | done |
| <a id="m2-1c"></a>[M2.1c][phase-1] | The same cases flown with each code's own drag, a CI job, and regenerating references | done |
| <a id="m2-1c1"></a>[M2.1c1][phase-1] | A CI job that checks every case against its stored reference, and a workflow, run only by hand, that regenerates the references | done |
| <a id="m2-1c2"></a>[M2.1c2][phase-1] | The same cases flown with each code's own drag | done |
| <a id="m2-1d"></a>[M2.1d][phase-1] | The time-series comparison, and where a rocket goes in wind ([issue #50](https://github.com/nrdptel/hpr-sim/issues/50), why hpr turned into the wind less) | done |
| <a id="m2-1d1"></a>[M2.1d1][phase-1] | Each whole flight's height and speed, compared over time with RocketPy's | done |
| <a id="m2-1d2"></a>[M2.1d2][phase-1] | Juno III, Calisto and Bella Lui in still air, as committed cases to measure the wind against ([issue #50](https://github.com/nrdptel/hpr-sim/issues/50), why hpr turned into the wind less) | done |
| <a id="m2-1d3"></a>[M2.1d3][phase-1] | Why hpr turned into the wind less than RocketPy ([issue #50](https://github.com/nrdptel/hpr-sim/issues/50)): RocketPy's equations, corrected, and hpr's body lift | done |
| <a id="m1-8"></a>[M1.8][phase-1] | Transonic and supersonic aerodynamics, damping, and overriding the aerodynamics | not yet done |
| <a id="m1-8a"></a>[M1.8a][phase-1] | The normal force and centre of pressure through Mach 1 | done |
| <a id="m1-8b"></a>[M1.8b][phase-1] | Drag through Mach 1: the transonic rise and supersonic wave drag | done |
| <a id="m1-8b1"></a>[M1.8b1][phase-1] | The drag of noses, shoulders and steps through Mach 1, against NASA's Arcas Robin wind tunnel | done |
| <a id="m1-8b2"></a>[M1.8b2][phase-1] | Drag against RASAero II's from Mach 0.1 to 2 | done |
| <a id="m1-8b3"></a>[M1.8b3][phase-1] | The drag of a boattail faster than sound, and the base behind it | done |
| <a id="m1-8c"></a>[M1.8c][phase-1] | Roll from canted fins, roll damping, and pitch and yaw damping (kept as each part's local-flow damping) | done |
| <a id="m1-8d"></a>[M1.8d][phase-1] | Tables that override the normal force and centre of pressure, read from RASAero II | done |
| <a id="m1-8e"></a>[M1.8e][phase-1] | The body's normal force faster than sound, which slender-body theory underestimates past Mach 3 | not yet done |
| <a id="m1-8e1"></a>[M1.8e1][phase-1] | The second-order shock-expansion method: the lift a body's cylinder carries behind its nose faster than sound, checked against its report's tables | done |
| <a id="m1-8e2"></a>[M1.8e2][phase-1] | The body's supersonic normal force in a flight: the nose and its cylinder, joined to the subsonic model | done |
| <a id="m1-8e3"></a>[M1.8e3][phase-1] | Faster than sound: the blend into the shock-expansion method now starts at the exact Mach where the method starts to hold, not rounded to a 0.05 step | done |
| <a id="m1-8e4"></a>[M1.8e4][phase-1] | Faster than sound: a boattail, and a tube behind it, take their share of the body's normal force from the shock-expansion method | done |
| <a id="m1-8e5"></a>[M1.8e5][phase-1] | Faster than sound: how much of the Arcas Robin's remaining gap each missing effect (crossflow, the blunt tip, and others) explains, measured before modelling | done |
| <a id="m1-8e6"></a>[M1.8e6][phase-1] | The size of crossflow lift at every speed (Jorgensen) and a boattail's measured share faster than sound (Washington and Pettis), the causes that measurement ranked first, in flight | done |
| <a id="m1-8e7"></a>[M1.8e7][phase-1] | Faster than sound: blunt and vertical nose tips (power-series, Haack and elliptical noses) by a Newtonian cap ahead of the shock-expansion method | done |
| <a id="m1-8e8"></a>[M1.8e8][phase-1] | Faster than sound: the lip behind a boattail, so that the committed Arcas Robin designs fly the shock-expansion method | done |
| <a id="m1-8e9"></a>[M1.8e9][phase-1] | Faster than sound: a bound on the boattail angle, footnote 8's size pinned by hand, and the Arcas Robin wind tunnel's 15% target judged | done |
| <a id="m1-8e10"></a>[M1.8e10][phase-1] | A lip's shelter weighed as the drag buildup weighs it, instead of switching the whole body's model at a threshold, and the size of every switch that remains | done |
| <a id="m1-8e11"></a>[M1.8e11][phase-1] | Cone slopes from 24° to 30°, from NASA SP-3007, where TN 3527's own chart stops | done |
| <a id="m1-8e12"></a>[M1.8e12][phase-1] | What a blunt tip's handover cap is worth once the cone slopes reach 30°, and what stops it moving there | done |
| <a id="m1-8e13"></a>[M1.8e13][phase-1] | What puts a marched answer at the mercy of the mesh: the surface pressure crossing its tangent cone's, counted and told apart from a reduced element | done |
| <a id="m1-8e14"></a>[M1.8e14][phase-1] | Where the method stops marching a flare, bisected over Mach, and what the edge is made of | done |
| <a id="m1-8e17"></a>[M1.8e17][phase-1] | A flared body flown through the method where its shock is attached, with nothing jumping across that boundary as the model changes (the second of the three the old [M1.8e14](#m1-8e14) splits into) | done |
| <a id="m1-8e18"></a>[M1.8e18][phase-1] | NASA TN D-4865 model 2's readings committed, and what a marched flare is worth (the third of the three the old [M1.8e14](#m1-8e14) splits into) | done |
| <a id="m1-8e15"></a>[M1.8e15][phase-1] | What a step in radius still switches, how big it is, and what a model of one would need | done |
| <a id="m1-8e19"></a>[M1.8e19][phase-1] | The band of near-flat flares the march used to refuse, which took the whole body off the method as a shape crossed it (found by [M1.8e17](#m1-8e17)); now read by the generalized method | done |
| <a id="m1-8e16"></a>[M1.8e16][phase-1] | A blunt tip's handover moved past 24°, once the march has a rule for the loading through a crossing (the rest of what [M1.8e13](#m1-8e13) used to be, renumbered so the flare and the step keep their ids). Blocked on [issue #108](https://github.com/nrdptel/hpr-sim/issues/108): the loading through a tangent-cone crossing has no reading that settles as the nose is cut finer | blocked |
| <a id="m3-1"></a>[M3.1][phase-1] | Reading OpenRocket `.ork` design files | done |
| <a id="m3-1a"></a>[M3.1a][phase-1] | The container a `.ork` arrives in, and its design document read whole | done |
| <a id="m3-1b"></a>[M3.1b][phase-1] | The component tree: parts, shapes, materials, finishes and overrides into a design | done |
| <a id="m3-1b1"></a>[M3.1b1][phase-1] | What a `.ork` value means: dimensions OpenRocket works out for itself, tags written under two names, and overrides | done |
| <a id="m3-1b2"></a>[M3.1b2][phase-1] | The spine: the stages and the body components stacked in them, with their automatic radii marked for the layout to resolve | done |
| <a id="m3-1b3"></a>[M3.1b3][phase-1] | The parts on and inside the body, with their positions and the dimensions they take from their parents | done |
| <a id="m3-1b4"></a>[M3.1b4][phase-1] | The three designs of the 76 whose rocket still did not lay out: a radius with nothing to take, and a document with no design | done |
| <a id="m3-1c"></a>[M3.1c][phase-1] | Motors, recovery, stages, and what OpenRocket last simulated | done |
| <a id="m3-1c1"></a>[M3.1c1][phase-1] | A design's motor configurations, the motors in them, and their thrust curves | done |
| <a id="m3-1c2"></a>[M3.1c2][phase-1] | When each parachute and streamer opens, and when each stage separates | done |
| <a id="m3-1c3"></a>[M3.1c3][phase-1] | The launch conditions and results of the simulations OpenRocket stored | done |
| <a id="m3-1c4"></a>[M3.1c4][phase-1] | Pods, parallel stages, and every part, section, tag and attribute hpr does not read, kept for writing the file back | done |
| <a id="m3-1d"></a>[M3.1d][phase-1] | The corpus and the cross-check against RocketSerializer | done |
| <a id="m3-1d1"></a>[M3.1d1][phase-1] | Snapshots of what hpr reads from public `.ork` designs | done |
| <a id="m3-1d2"></a>[M3.1d2][phase-1] | Every `.ork` imports without an error, and the cross-check against RocketSerializer | done |
| <a id="m2-2"></a>[M2.2][phase-1] | OpenRocket as a reference program, and a corpus of designs to compare | not yet done |
| <a id="m2-2a"></a>[M2.2a][phase-1] | Each design's structure (every stage, no motor): mass, centre of mass and inertia against OpenRocket's | done |
| <a id="m2-2b"></a>[M2.2b][phase-1] | OpenRocket's mass conventions: a wall-less shoulder, clusters, fillets, a part with no material, and roll inertia | not yet done |
| <a id="m2-2c"></a>[M2.2c][phase-1] | The motors OpenRocket flies, for the configurations held back for want of a thrust curve | not yet done |
| <a id="m2-2d"></a>[M2.2d][phase-1] | Flights to apogee on the public designs, against OpenRocket | not yet done |
| <a id="m2-2e"></a>[M2.2e][phase-1] | The corpus, with a hypothesis for every apogee miss over 5% | not yet done |
| <a id="m1-9"></a>[M1.9][phase-1] | Staging, clusters and air starts, for COTS motors | not yet done |
| <a id="m1-10"></a>[M1.10][phase-1] | Flight outputs: the stability margin over the flight, the best ejection delay, the peak dynamic pressure, fin flutter and the landing point | not yet done |
| <a id="m2-3"></a>[M2.3][phase-1] | Comparisons with real flights | not yet done |
| <a id="m2-4"></a>[M2.4][phase-1] | A summary of accuracy for the README, and CI that fails on any regression | not yet done |
| <a id="m1-11"></a>[M1.11][phase-1] | Ejected nose cones, body sections and payloads, each flown to its own landing | not yet done |
| <a id="m1-12"></a>[M1.12][phase-1] | Payload mass that moves, or is released, during the flight | not yet done |
| <a id="m1-13"></a>[M1.13][phase-1] | Pods: bodies mounted beside the airframe, with or without motors | not yet done |
| <a id="m4-1"></a>[M4.1][phase-2] | A simpler interface, with a builder for environments, motors, rockets and flights | not yet done |
| <a id="m4-2"></a>[M4.2][phase-2] | A command-line tool | not yet done |
| <a id="m3-2"></a>[M3.2][phase-2] | Writing OpenRocket `.ork` files | not yet done |
| <a id="m3-3"></a>[M3.3][phase-2] | hpr's own open design file format | not yet done |
| <a id="m4-3"></a>[M4.3][phase-2] | Python bindings | not yet done |
| <a id="m5-1"></a>[M5.1][phase-2] | The online layer, with an on-disk cache for working offline | not yet done |
| <a id="m5-2"></a>[M5.2][phase-2] | Weather forecasts, turned into atmosphere and wind profiles | not yet done |
| <a id="m5-3"></a>[M5.3][phase-2] | Launch-site data: ground elevation and magnetic declination | not yet done |
| <a id="m5-4"></a>[M5.4][phase-2] | Motor stock and prices | not yet done |
| <a id="m5-5"></a>[M5.5][phase-2] | A catalogue of parts | not yet done |
| <a id="m6-1"></a>[M6.1][phase-3] | Monte Carlo runs and sensitivity | not yet done |
| <a id="m6-2"></a>[M6.2][phase-3] | Design optimization | not yet done |
| <a id="m6-3"></a>[M6.3][phase-3] | Competition rules as files, with scoring, limits and presets | not yet done |
| <a id="m6-4"></a>[M6.4][phase-3] | Airbrakes, with a controller that aims for a target apogee | not yet done |
| <a id="m6-5"></a>[M6.5][phase-3] | Canards, fixed and then movable for roll control | not yet done |
| <a id="m3-4"></a>[M3.4][phase-4] | RockSim `.rkt` files in and out | not yet done |
| <a id="m3-5"></a>[M3.5][phase-4] | RASAero `.CDX1` files in and out | not yet done |
| <a id="m3-6"></a>[M3.6][phase-4] | RocketPy scripts and files in and out | not yet done |
| <a id="m4-4"></a>[M4.4][phase-4] | Use from C, and in the browser through WebAssembly | not yet done |
| <a id="m7-1"></a>[M7.1][phase-5] | Reading flight logs from altimeters and trackers, with no design file and no simulation | not yet done |
| <a id="m7-2"></a>[M7.2][phase-5] | The readings a flight gives, each with where it came from, and reconstructing the flight from its log | not yet done |
| <a id="m7-3"></a>[M7.3][phase-5] | A flight against its simulation: residuals, and fitting drag, mass, impulse and wind to the log | not yet done |
| <a id="m7-4"></a>[M7.4][phase-5] | Diagnosing what went wrong in a flight | not yet done |
| <a id="m8-1"></a>[M8.1][phase-6] | A design assistant | not yet done |
| <a id="m8-2"></a>[M8.2][phase-6] | An editing model for apps: commands, undo and stable ids | not yet done |
| <a id="m9-0"></a>[M9.0][phase-7] | Choosing how the app is built, with trial builds | not yet done |
| <a id="m9-1"></a>[M9.1][phase-7] | A desktop app | not yet done |
| <a id="m9-2"></a>[M9.2][phase-7] | 3D flight replay, the real flight beside the simulated one | not yet done |
| <a id="m9-3"></a>[M9.3][phase-7] | A web app that works offline | not yet done |
| <a id="m9-4"></a>[M9.4][phase-7] | Mobile apps | not yet done |
| <a id="m9-5"></a>[M9.5][phase-7] | Optional accounts that save designs and flights and sync them across devices | not yet done |

## Lessons from Loft

Loft was the project that came before hpr-sim. Its mistakes are listed as numbered lessons, such as
[Loft lesson L18](#l18), its made-up transonic drag curve, each with the test that guards against it
here, or the milestone that will add one. The [list of lessons][lessons] says what went wrong, where
in Loft, and how hpr avoids it.

### The lessons these pages name

Each lesson that a page of this site names has a row here. Its label opens its section of the list
of lessons, which gives Loft's evidence and the name of the test that guards it; the last column
is the milestone that added or will add that test.

| lesson | what went wrong in Loft, or what the lesson records | guarded by |
|---|---|---|
| <a id="l1"></a>[L1][lessons-physics] | Loft used one constant gravity on a flat Earth, so it read low against RocketPy | [M1.1](#m1-1) |
| <a id="l2"></a>[L2][lessons-physics] | Loft treated geometric altitude as geopotential, so its temperature and pressure were off at 11 km | [M1.2](#m1-2) |
| <a id="l3"></a>[L3][lessons-physics] | Loft's atmosphere had four layers, and its last temperature gradient ran on forever: 335 K at 70 km, against 219.6 K | [M1.2](#m1-2) |
| <a id="l4"></a>[L4][lessons-physics] | Loft's air viscosity constants weren't the 1976 standard's, 1.3% high at sea level | [M1.2](#m1-2) |
| <a id="l5"></a>[L5][lessons-physics] | Loft's "today's conditions" kept the standard temperature gradient above the field, with dry air and no sounding temperatures | [M1.2](#m1-2) |
| <a id="l6"></a>[L6][lessons-physics] | Loft flew one wind vector; forecast profiles stepped at the lowest level; no gusts | [M1.2](#m1-2) |
| <a id="l7"></a>[L7][lessons-physics] | Loft's fin lift had no compressibility factor, so its slope and centre of pressure never changed with Mach | [M1.8a](#m1-8a) |
| <a id="l8"></a>[L8][lessons-physics] | Loft's fin lift grew in proportion to the fin count, with no correction for 5 to 8 fins | [M1.5a](#m1-5a) |
| <a id="l9"></a>[L9][lessons-physics] | Loft used the conical transition's centre-of-pressure formula for every transition shape | [M1.5a](#m1-5a) |
| <a id="l10"></a>[L10][lessons-physics] | Loft took elliptical fins' lift from an equal-area trapezoid with the wrong sweep | [M1.5a](#m1-5a) |
| <a id="l11"></a>[L11][lessons-physics] | Loft merged fin sets into one for drag, so the result depended on their order | [M1.5b](#m1-5b) |
| <a id="l12"></a>[L12][lessons-physics] | Loft's drag used uncited constants, such as a body form factor of 1.95 where Niskanen gives 1.13 | [M1.5b](#m1-5b) |
| <a id="l13"></a>[L13][lessons-physics] | Loft had no power-on base drag relief, which Niskanen's drag method includes | [M1.5b](#m1-5b) |
| <a id="l14"></a>[L14][lessons-physics] | Loft's launch-lug drag was uncited, and rail buttons dragged as lugs | [M1.5b](#m1-5b) |
| <a id="l15"></a>[L15][lessons-physics] | Loft gave a bare step in diameter no drag, and shoulder drag jumped to zero as a transition shrank | [M1.5b](#m1-5b) |
| <a id="l16"></a>[L16][lessons-physics] | Loft silently capped the drag coefficient at 10, which hid malformed designs | [M1.5b](#m1-5b) |
| <a id="l17"></a>[L17][lessons-physics] | Loft froze the fins' leading-edge drag at its Mach 1 value, and gave nose and shoulder pressure drag no Mach term | [M1.8](#m1-8) |
| <a id="l18"></a>[L18][lessons-physics] | Loft's transonic wave drag was an invented curve, never checked against RASAero | [M1.8](#m1-8) |
| <a id="l20"></a>[L20][lessons-physics] | Loft flew in 3-DOF: no angle of attack, body lift, damping or roll, and no weathercocking | [M1.6b](#m1-6b) |
| <a id="l21"></a>[L21][lessons-physics] | Loft stepped with RK4 without error control, and never tested that the apogee converges | [M1.6a](#m1-6a) |
| <a id="l22"></a>[L22][lessons-physics] | Loft didn't locate events exactly: apogee fell on a step, altitude deployments overshot, landings were below ground | [M1.6a](#m1-6a) |
| <a id="l23"></a>[L23][lessons-physics] | Loft let discontinuities such as burnout fall inside steps, and checked its vacuum case to only ±2% | [M1.6a](#m1-6a) |
| <a id="l24"></a>[L24][lessons-physics] | Loft's `simulate()` changed its inputs, so a second run of the same flight differed from the first | [M1.6b](#m1-6b) |
| <a id="l25"></a>[L25][lessons-physics] | Loft labelled any early stop "step budget", even a rocket that never lifted off | [M1.6b](#m1-6b) |
| <a id="l26"></a>[L26][lessons-physics] | Loft's launch rail had no friction and no button geometry | [M1.6b](#m1-6b) |
| <a id="l35"></a>[L35][lessons-physics] | Loft used zeros for "never happened", and never said which height apogee was measured from | [M1.10](#m1-10) |
| <a id="l36"></a>[L36][lessons-motors] | Loft's `.eng` reader read only the first header, and appended a second motor's points to the first curve | [M1.3](#m1-3) |
| <a id="l37"></a>[L37][lessons-motors] | Loft's delay parsing lost `P` (plugged) and lists of delays, and read marker values as seconds | [M1.3](#m1-3) |
| <a id="l38"></a>[L38][lessons-motors] | Loft's impulse class letter was off by one at the top of each band | [M1.3](#m1-3) |
| <a id="l39"></a>[L39][lessons-motors] | Loft didn't check that a curve's times increase, and took the last point as burnout instead of NFPA 1125's rule | [M1.3](#m1-3) |
| <a id="l40"></a>[L40][lessons-motors] | Loft fixed the motor's centre of gravity at the casing's middle, with no inertia of its own | [M1.3](#m1-3) |
| <a id="l41"></a>[L41][lessons-motors] | Loft bundled thrust curves under mixed or unknown licences | [M1.3](#m1-3) |
| <a id="l42"></a>[L42][lessons-motors] | Loft's impulse checks were loose (±8%); a mis-sourced curve flew about 26% high until caught | [M1.3](#m1-3) |
| <a id="l43"></a>[L43][lessons-motors] | ThrustCurve's data must override a `.eng` header's size: one said 75 mm for a 54 mm motor | [M1.3](#m1-3) |
| <a id="l56"></a>[L56][lessons-formats] | Loft told a design file's container apart by its first bytes, and a malformed one had to give an error rather than crash | [M3.1a](#m3-1a) |
| <a id="l57"></a>[L57][lessons-formats] | Loft threw away the thrust curves stored inside a `.ork` archive | [M3.1a](#m3-1a), [M3.1c1](#m3-1c1) |
| <a id="l59"></a>[L59][lessons-formats] | Loft resolved an automatic radius within a stage only, so a booster's first component took a stale number | [M3.1b2](#m3-1b2) |
| <a id="l60"></a>[L60][lessons-formats] | Loft resolved automatic dimensions as it walked the tree, so a ring's bore depended on sibling order and a bulkhead inside a coupler stayed `NaN` | [M3.1b3](#m3-1b3) |
| <a id="l61"></a>[L61][lessons-formats] | Loft weighed a ring with an automatic bore at 0 g, and dropped a stated wall when the outer radius was automatic | [M3.1b2](#m3-1b2), [M3.1b3](#m3-1b3) |
| <a id="l58"></a>[L58][lessons-formats] | Loft kept an automatic dimension's number but lost the flag, so saving turned it into a hand-typed one | [M3.1b1](#m3-1b1) |
| <a id="l62"></a>[L62][lessons-formats] | Loft met a tag written under two names, and read a stated `0` as a missing value | [M3.1b1](#m3-1b1) |
| <a id="l63"></a>[L63][lessons-formats] | Loft read neither the drag override nor the subcomponent flags, so a part set to no drag was still charged drag | [M3.1b1](#m3-1b1) |
| <a id="l64"></a>[L64][lessons-formats] | Loft read the wind's direction from the launch rod's, and dropped it from the stored conditions | [M3.1c3](#m3-1c3) |
| <a id="l65"></a>[L65][lessons-formats] | Loft read a configuration from only one of the two places a `.ork` keeps it, and fired a motor whose mount it could not find from the top stage | [M3.1c1](#m3-1c1) |
| <a id="l66"></a>[L66][lessons-formats] | Loft dropped pods, parallel stages and booster sets, and its export then lost the note that the rocket was reduced | [M3.1c4](#m3-1c4) |
| <a id="l44"></a>[L44][lessons-motors] | Loft's inertia was pitch only, with simplified formulas, and zero for rings and masses | [M1.4a](#m1-4a) |
| <a id="l45"></a>[L45][lessons-motors] | Loft put a hollow transition's centre of gravity at the solid's centroid | [M1.4a](#m1-4a) |
| <a id="l46"></a>[L46][lessons-motors] | Loft never read fin tabs (100 to 120 g lost on two designs), and rail buttons weighed nothing | [M1.4a](#m1-4a) |
| <a id="l47"></a>[L47][lessons-motors] | Loft's reference diameter was the widest part, even an internal one | [M1.4b](#m1-4b) |
| <a id="l48"></a>[L48][lessons-motors] | Loft had tangent ogives only, a silent default nose shape, and swapped Haack names | [M1.4a](#m1-4a) |
| <a id="l49"></a>[L49][lessons-motors] | Loft's transitions used a kinked profile, never checked against what OpenRocket means | [M1.4a](#m1-4a), [M3.1](#m3-1) |
| <a id="l50"></a>[L50][lessons-motors] | Loft let a motor wider than its mount fly (+69% apogee), and fins could sit off the airframe | [M1.4b](#m1-4b) |
| <a id="l51"></a>[L51][lessons-motors] | Loft's rule for which centre-of-gravity override wins was unsettled (up to 133 mm) and came from OpenRocket's source | [M2.2](#m2-2) |
| <a id="l75"></a>[L75][lessons-validation] | Loft's RocketPy check wasn't like for like: a different atmosphere, unstated latitude and gravity, and Loft's own drag and mass fed to RocketPy | [M2.1b](#m2-1b), [M2.1b2](#m2-1b2) |
| <a id="l76"></a>[L76][lessons-validation] | Loft's advice to regenerate a reference when a check failed let the reference follow Loft's own drag | [M2.1a](#m2-1a) |
| <a id="l77"></a>[L77][lessons-validation] | Loft shipped hand-written "stored" results, one set inconsistent with itself | [M2.1a](#m2-1a) |
| <a id="l78"></a>[L78][lessons-validation] | Loft's test suites skipped themselves when their data was missing, and still reported a pass | [M2.1a](#m2-1a) |
| <a id="l79"></a>[L79][lessons-validation] | Only 2 of Loft's 12 metrics had a tolerance per case; a deployment speed 204% off passed as "ungated" | [M2.1a](#m2-1a) |
| <a id="l85"></a>[L85][lessons-validation] | Loft's check that a known gap had closed used half the tolerance, so it missed gaps that closed in between | [M2.1b2](#m2-1b2), [M2.4](#m2-4) |
| <a id="l82"></a>[L82][lessons-validation] | References 60% apart were excused as "no single target", and known issues excused the two largest misses | [M2.2](#m2-2) |
| <a id="l89"></a>[L89][lessons-tests] | Barrowman's hand-worked values for a cone, a conical transition and an elliptical fin, which hpr's tests check | [M1.5a](#m1-5a) |
| <a id="l90"></a>[L90][lessons-tests] | Properties any drag model must keep, such as split fin sets dragging like one set, which hpr's tests check | [M1.5b](#m1-5b) |
| <a id="l91"></a>[L91][lessons-tests] | Exact volumes of nose cones (cone, tangent ogive, Haack), which hpr's tests check | [M1.4a](#m1-4a) |

[adr-000]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-000-kickoff-decisions-2026-09-16
[adr-001]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-001-license-and-workspace-layout-2026-09-17
[adr-002]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-002-the-reference-library-lock-file-fetch-verify-and-doctor-2026-09-17
[adr-003]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-003-frames-attitude-geodesy-and-the-gravity-model-2026-09-17
[adr-004]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-004-atmosphere-wind-turbulence-and-the-seeded-generator-2026-09-17
[adr-005]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-005-solid-motors-statistics-consumption-grains-file-models-and-the-bundled-catalog-2026-09-17
[adr-006]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-006-component-geometry-and-mass-properties-frames-shapes-walls-fins-and-materials-2026-09-17
[adr-007]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17
[adr-008]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-008-subsonic-normal-force-and-centre-of-pressure-2026-09-17
[adr-009]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-009-subsonic-drag-buildup-surface-finishes-and-drag-override-tables-2026-09-17
[adr-010]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-010-time-integration-dormandprince-with-dense-output-rk4-stop-times-and-events-2026-09-17
[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[adr-012]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-012-recovery-drag-areas-triggers-inflation-and-the-descent-phase-2026-09-17
[adr-013]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-013-streamer-and-tumble-drag-2026-09-17
[adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
[adr-015]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-015-the-validation-harness-cases-references-tolerances-and-reports-2026-09-17
[adr-016]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-016-the-documentation-site-mdbook-over-docs-and-checks-for-links-labels-and-equations-2026-09-18
[adr-017]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-017-model-pages-open-with-in-short-accuracy-traces-its-numbers-the-records-stay-files-2026-09-18
[adr-018]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-018-examples-run-in-ci-against-committed-output-pages-quote-files-checked-line-for-line-2026-09-18
[adr-019]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-019-publishing-the-site-and-the-api-reference-to-github-pages-2026-09-18
[adr-020]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-020-the-reader-test-and-labels-that-lead-to-plain-words-2026-09-18
[adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
[adr-022]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-022-validation-in-ci-and-regenerating-references-only-by-hand-2026-09-18
[adr-023]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-023-predicted-mode-each-codes-own-drag-reported-against-a-target-2026-09-18
[adr-024]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-024-the-time-series-rms-aligned-at-ignition-held-to-3-of-its-traces-scale-2026-09-18
[adr-025]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-025-the-calm-air-cases-and-juno-iiis-drifts-left-to-the-rail-release-2026-09-18
[adr-026]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18
[adr-027]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-027-the-normal-force-through-mach-1-supersonic-linear-theory-a-transonic-join-and-the-measured-references-2026-09-18
[adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18
[adr-029]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-029-drag-against-rasaero-ii-through-mach-2-the-gap-by-band-mil-hdbk-762s-sample-calculation-and-the-boattails-wave-drag-2026-09-18
[adr-030]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-030-the-afterbody-faster-than-sound-a-boattails-wave-drag-the-base-behind-it-and-a-lip-in-its-wake-2026-09-18
[adr-031]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-031-roll-from-canted-fins-and-roll-damping-by-barrowmans-strip-theory-2026-09-19
[adr-032]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-032-normal-force-overrides-from-rasaero-ii-the-static-force-replaced-hprs-damping-kept-2026-09-19
[adr-033]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-033-the-body-faster-than-sound-syvertson-and-denniss-second-order-shock-expansion-method-2026-09-19
[adr-034]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-034-the-bodys-supersonic-normal-force-in-flight-tabulated-shock-expansion-shares-joined-linearly-from-mach-12-2026-09-19
[adr-035]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-035-drop-the-orhelper-dependency-how-m22-drives-openrocket-is-decided-when-m22-starts-2026-09-19
[adr-036]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-036-the-arcas-robins-supersonic-body-gap-judged-as-the-tunnel-measures-m18e6-takes-crossflows-size-and-the-boattail-2026-09-19
[adr-037]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-037-body-lift-by-jorgensens-crossflow-at-every-speed-and-a-boattails-measured-share-faster-than-sound-2026-09-19
[adr-038]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-038-blunt-and-vertical-nose-tips-faster-than-sound-by-a-newtonian-cap-the-method-started-from-the-tangent-cone-2026-09-19
[adr-039]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-039-a-lip-in-a-boattails-wake-carries-nothing-faster-than-sound-2026-09-19
[adr-040]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-040-a-steep-boattail-reads-its-measured-correlation-no-steeper-than-16-and-m18es-15-target-judged-2026-09-19
[adr-041]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-041-a-lips-shelter-is-weighed-as-the-drag-buildup-weighs-it-not-switched-at-a-threshold-2026-09-20
[adr-042]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-042-cone-slopes-from-24-to-30-come-from-simss-tables-where-tn-3527s-chart-stops-2026-09-20
[adr-043]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-043-the-blunt-tips-handover-cap-what-it-is-worth-and-what-stops-it-moving-2026-09-20
[adr-044]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-044-what-the-answer-follows-when-it-follows-the-mesh-is-a-crossing-of-the-tangent-cone-not-a-reduced-element-2026-09-20
[adr-045]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-045-where-a-flares-march-stops-is-the-corners-isentropic-turn-not-the-shock-detaching-2026-09-20
[adr-046]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-046-debrief-folded-in-and-flight-log-analysis-that-stands-without-the-simulator-2026-09-20
[adr-047]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-047-a-flare-flies-the-method-where-its-corners-shock-is-attached-and-is-read-drawn-out-where-it-is-not-2026-09-20
[adr-048]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-048-what-a-marched-flare-is-worth-measured-against-tn-d-4865s-model-2-2026-09-20
[adr-049]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-049-what-a-step-in-radius-costs-and-why-the-obvious-fix-is-not-taken-yet-2026-09-20
[adr-050]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-050-a-reduced-element-is-read-by-the-generalized-method-wherever-it-has-a-tangent-cone-of-its-own-2026-09-20
[adr-051]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-051-m31-split-and-the-ork-document-kept-whole-rather-than-interpreted-2026-09-20
[adr-054]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-054-an-automatic-radius-with-nothing-to-take-is-openrockets-default-and-a-rocket-with-no-stage-or-component-holds-no-design-2026-09-20
[adr-055]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-055-m31c-split-and-the-motors-a-ork-flies-its-own-curve-first-and-only-what-lights-at-launch-2026-09-21
[adr-056]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-056-a-ork-designs-recovery-and-separation-read-as-written-with-openrockets-words-measured-2026-09-21
[adr-057]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-057-a-ork-designs-stored-simulations-read-back-as-written-with-their-units-measured-2026-09-21
[adr-058]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-058-what-a-ork-holds-that-hpr-does-not-model-kept-whole-in-x-openrocket-2026-09-21
[adr-059]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-059-the-rocketserializer-cross-check-three-readers-with-openrocket-settling-a-difference-2026-09-21
[adr-060]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-060-m22-split-and-the-structures-mass-held-to-openrockets-2026-09-21
[adr-053]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-053-the-parts-on-and-inside-a-ork-body-degrees-what-is-left-out-and-a-sourced-finish-2026-09-20
[adr-052]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-052-what-a-ork-value-means-automatic-dimensions-two-names-for-one-tag-and-overrides-2026-09-20
[decisions]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[lessons-formats]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md#file-formats
[lessons-motors]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md#motors-mass-and-design-checks
[lessons-physics]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md#physics-and-numerics
[lessons-tests]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md#tests-worth-porting-closed-forms-lofts-tolerances-were-loose
[lessons-validation]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md#validation
[phase-0]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-0-foundations
[phase-1]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-1-physics-core-the-heart-with-validation-interleaved
[phase-2]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-2-library-surfaces-and-interop
[phase-3]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-3-uncertainty-optimization-challenges
[phase-4]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-4-more-formats-and-embeddings
[phase-5]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-5-flight-data-and-forensics
[phase-6]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-6-design-experience-library-level
[phase-7]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-7-ui-3d-web-mobile-only-after-the-phases-above
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
