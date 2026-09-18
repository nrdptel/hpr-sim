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

## The roadmap

The [roadmap][roadmap] is the ordered plan of work. It is split into phases, and each phase into
milestones. A milestone's id is `M`, a topic number, a dot and its place in that topic. The topic
is not the phase: 0 is foundations, 1 physics, 2 validation, 3 file formats, 4 library interfaces,
5 online data, 6 Monte Carlo and optimization, 7 flight logs, 8 design help and 9 the app. Phases
mix topics, so [M2.3][roadmap], the real-flights milestone, sits in Phase 1 beside
[M1.8][roadmap], the second aerodynamics milestone. A milestone too big to ship at once is split
into increments with a letter, and sometimes a digit after it, such as [M2.1b2][roadmap]. Each one
ends with *done when* conditions, and it is checked off only when all of them hold. To find one,
search the roadmap for its id.

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
- [Phase 5: Flight data and forensics][phase-5]: reading flight logs, and comparing a real flight
  with its simulation.
- [Phase 6: Design experience][phase-6]: a design assistant for apps to build on.
- [Phase 7: UI, 3D, web, mobile][phase-7]: a desktop app, 3D flight replay, a web app and mobile.

What is done so far, and what is not, is on [Start here](start-here.md#what-works-today).

## Lessons from Loft

Loft was the project that came before hpr-sim. Its mistakes are listed as numbered lessons, such as
[Loft lesson L18][lessons], each with the test that guards against it here, or the milestone that
will add one. The [list of lessons][lessons] says what went wrong and how hpr avoids it.

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
[decisions]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md
[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[phase-0]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-0-foundations
[phase-1]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-1-physics-core-the-heart-with-validation-interleaved
[phase-2]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-2-library-surfaces-and-interop
[phase-3]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-3-uncertainty-optimization-challenges
[phase-4]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-4-more-formats-and-embeddings
[phase-5]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-5-flight-data-and-forensics
[phase-6]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-6-design-experience-library-level
[phase-7]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-7-ui-3d-web-mobile-only-after-the-phases-above
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
