# Ideas: design, build, physics and trust

Part of the [ideas backlog](ideas.md) (2026-10-03). One line each, with its tier.

## Design and build

- SOON: a kit library, built from makers' published specs.
- SOON: reverse design from requirements.
- SOON: design-diff apogee attribution (which change moved the apogee, and by how much).
- LATER: a quality-diversity design atlas (MAP-Elites, CMA-ME; [arXiv 2504.02177](https://arxiv.org/abs/2504.02177)).
- LATER: evolutionary fin shapes.
- LATER: freeform fins traced from an image.
- SOON: a fabrication pack: DXF and SVG fins and rings, printable templates and marking guides,
  STL nose cones, fin jigs.
- LATER: STEP export.
- LATER: live Onshape and Fusion parametric add-ins.
- LATER: import the outer shape from CAD or STL.
- SOON: per-part drag coefficient against Mach sweeps.
- SOON: custom expressions.
- SOON: external aerodynamic tables per component, against angle of attack and Reynolds number
  (extends the existing overrides).
- LATER: per-part forces exported for finite-element analysis.
- SOON: as-built mode: measured mass, CG and a swing test update the model.
- SOON: material mass (fiberglass, epoxy, paint, 3D-print infill).
- SOON: composite layup stiffness (classical laminate theory) feeding flutter.
- SOON: measured fin torsional stiffness as an input, with a DIY twist test.
- SOON: a fin resonance warning.
- SOON: flutter as a spread across methods.
- SOON: blunt trailing-edge fins.
- SOON: motor hardware fit lookup (cases, spacers, retainers).
- SOON: launcher selection (rod whip, rail size, button loads).
- SOON: drag plates sized to cap the altitude.
- LATER: boosted darts and drag separation.
- SOON: scaling a design.
- LATER: spin cans.
- SOON: fins that differ by axis ([OpenRocket #1548](https://github.com/openrocket/openrocket/issues/1548)).
- SOON: streamer and tumble descent improvements.
- LATER: oddrocs and short, fat rockets.
- SOON: camera shrouds and other protrusions.
- LATER: a photo studio and decals.
- LATER: a phone scan compared with the as-built model.
- SOON: one-line parameter sweeps (`hpr sweep`).

## Physics and trust

NEXT, already in M1.14f: asymmetries (thrust and fin misalignment, a lateral CG offset), airframe
drag in recovery, power-on base drag, gusts in flight, parachute opening loads and swing, laminar
friction, interference and fillet drag, default nozzle exits, separation-charge push, grain CG
shift. The rest:

- SOON: propellant temperature correction.
- SOON: effective rail length from the rail buttons ([OpenRocket #2507](https://github.com/openrocket/openrocket/issues/2507); IREC and EuRoC define it).
- SOON: stability judged by shape, by dynamic stability and by competition rules together.
- SOON: structural loads, fin divergence and canted-fin roll resonance.
- SOON: aerodynamic heating past Mach 2, labelled lightly validated.
- SOON: a tuned model per rocket (a digital twin; Kennedy and O'Hagan's calibration).
- SOON: two-knob truing: impulse against drag per Mach band, from ballistics.
- SOON: normalize a flight to standard conditions.
- SOON: why-simulators-disagree attribution.
- SOON: "explain this number".
- SOON: a credibility card (NASA-STD-7009).
- SOON: an accuracy band on every result.
- SOON: exact sensitivities (automatic differentiation).
- SOON: subset simulation for rare events.
- SOON: Wilks 95/95 bounds and a sample-size advisor.
- SOON: a polynomial-chaos surrogate.
- LATER: a learned fast surrogate for the web.
- SOON: a mid-fidelity panel-method aerodynamics tier (Ironbark,
  [doi 10.1016/j.ast.2026.111758](https://doi.org/10.1016/j.ast.2026.111758); PANAIR's licence to
  check).
- LATER: flexible-airframe dynamics.
