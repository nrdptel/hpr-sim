# Flying the body's supersonic normal force (M1.8e2): the plan

What this covers: how M1.8e2 should put M1.8e1's second-order shock-expansion method
(`hpr_aero::shock_expansion`, ADR-033) into a flight. What it is for: the next session starts
from here. How far to trust it: a plan read from the code on 2026-09-19, not built or measured.

## Where the body's terms live today

| Place | What it does |
|---|---|
| `crates/hpr-aero/src/model.rs`, `//!` near line 17 | States the body's terms don't change with Mach (slender-body theory, Barrowman 1967 p. 18) |
| `body_terms`, `model.rs` near line 892 | One `BodyAero` per body component (nose, tube, transition): slope, moment slope, body lift `K · A_plan / A_ref` at the planform centroid |
| `body_term`, `model.rs` near line 875 | Scales the potential term by `sin α/α` and body lift by `sin² α/α`; Mach is not an input |
| `component_station_m`, `model.rs` near line 769 | A body's station is `moment_slope / slope`, independent of Mach |
| `crates/hpr-sim/src/dynamics.rs`, line 132 | Caches body stations once, because they don't change with Mach; fin stations are taken at each Mach |

The flight uses per-component forces at per-component stations for its local-flow damping
(ADR-011). A body's supersonic force must therefore land on components, not only on the whole.

## What the shock-expansion code gives

`ShockExpansionBody::slope` (`shock_expansion.rs`) returns one `ShockExpansionSlope`: the whole
body's `C_Nα` and its centre of pressure. It returns `Unsupported` where the method fails: a
detached tip shock, a tangent cone steeper than Fig. 2's 24°, a subsonic tip flow, and similar.
`ShockExpansionBody::new` refuses a nose that isn't pointed. TN 3527 states the method for Mach
number over nose fineness from 0.4 to 2; `slope` doesn't enforce that range.

## The decision to make first: sharing the force among components

1. **Per segment (preferred if it can be done).** Have the method report each segment's slope
   and moment. Its elements already follow the segments, so each hpr component (nose, then
   tube) would get its own share at its own station. Check first whether the element sums can
   be split by segment without changing the whole-body totals pinned by M1.8e1's fixture.
2. **A correction on the nose (fallback).** Keep the slender-body terms, and add one term for
   (shock-expansion minus slender-body over the same segments), placed so the whole-body
   moment matches the shock-expansion CP. This is simpler, but it puts the cylinder's carried
   lift at the nose's station for damping.

## The join from subsonic

- The join must be continuous in Mach. Blend the shock-expansion share in over a Mach band, and
  probe the band's ends at ±1e-9 in Mach for jumps. Prefer a form that is continuous by
  construction (a blended share) to one tuned until the probes pass (compare Loft lesson L15's
  shoulder-drag jump in `docs/research/loft-lessons.md`).
- A refusal inside a flight's Mach range would be a jump. So find, when the model is built, the
  lowest Mach at which the body's method holds, and join above that. A body the method can't
  take at all (blunt tips, M1.8e3) keeps slender-body theory at every Mach, and the report
  says so.
- Known inputs for the join: Fig. 2 is held below Mach 3 (ADR-033); the Prometheus case peaks at
  Mach 1.01 to 1.06, so it should not change; hpr's Arcas Robin nose is a power series, which the
  method takes as a pointed body (M1.8e1's fixture used a fitted secant ogive, ratio 1.744).
- Body lift (Galejs's crossflow term) stays as it is until M1.8e3.

## Checks to write first

- The join probe above, and a test that a body below the join gives exactly today's terms.
- The Arcas Robin body alone through `AeroModel` from Mach 1.5 beside TN D-4014 and M1.8a's
  values. The whole-body slope must equal `ShockExpansionBody::slope`'s at the same Mach.
- Regenerate the report with `cargo xtask validate` (debug), and list every changed row.
