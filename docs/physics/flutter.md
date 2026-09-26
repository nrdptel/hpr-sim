# Fin flutter

## In short

- **What it models:** the speed at which a fin starts to [flutter](../glossary.md#flutter): its
  bending and twisting feed each other, and the fin shakes itself apart. hpr gives the flutter
  speed at any height, the [dynamic pressure](../glossary.md#dynamic-pressure) at which a fin
  flutters, and how far below its flutter speed a flight stays.
- **Sources:** one empirical criterion, D. J. Martin's in NACA TN 4197 (1958), eq. 18
  ([References](#references)). The fin's stiffness enters as its
  [shear modulus](../glossary.md#shear-modulus); hpr has one, with its source, for 14 of its
  built-in materials.
- **How well it is validated:** it is a screening number, not a flutter analysis. Martin drew it
  through missile and wind-tunnel flights that did and didn't flutter, with a band of scatter
  between. hpr reproduces his formula and both of his worked examples at the precision he printed
  them ([Tests](#tests)). It is not checked against any hobby rocket's flight. Every choice hpr
  makes where the source leaves room gives the lower flutter speed.
- **What it leaves out:** sweep, and every kind of flutter but bending-torsion. Also left out:
  fins that aren't trapezoids, and the rocket body's own bending (Martin's figure 8). Composite
  fins are taken at their weakest in-plane shear, and G10/FR-4 has no built-in modulus
  ([Shear moduli](#shear-moduli)).

## What flutter is

Air pushing on a fin twists it a little. The twist changes the fin's angle to the air, so the push
changes, and the fin bends. Below a certain speed the fin's stiffness damps this out. Above it, each
cycle feeds the next, and a fin can fail within a second. The speed depends on the fin's shape and
thickness, how stiff its material is in shear, and the air it flies through.

## The criterion

Martin starts from Theodorsen and Garrick's flutter speed for a wing that bends and twists. He
reduces it to a few numbers from one fin's outline, and fixes its constant from the flights in his
figure 3. For a trapezoidal fin with root chord `c_r`, tip chord `c_t`, span `s` and thickness `t`:

| Symbol | Meaning | For a trapezoid |
|---|---|---|
| `A` | Panel aspect ratio: span over the chord halfway out | `2s / (c_r + c_t)` |
| `λ` | Taper ratio: tip chord over root chord, 0 to 1 | `c_t / c_r` |
| `t/c` | Thickness ratio | `t / c_r` |
| `G_E` | The fin's effective shear modulus, Pa | the material's ([Shear moduli](#shear-moduli)) |
| `p`, `a` | The air's static pressure and speed of sound | from the atmosphere |

The flutter speed `V_f` is then (his eq. 18)

```text
(V_f / a)² = G_E / D,    D = (24 ε γ / π) · p · A³ / ((t/c)³ (A + 2)) · (λ + 1)/2
```

with `ε = 0.25` (where the section's mass sits) and `γ = 1.4` (air). Martin prints the constant in
pounds per square inch: `24 · 0.25 · 1.4 / π · 14.696 psi = 39.29 psi`, which he rounds to 39.3.
Stiffer or thicker fins flutter faster: `V_f` grows as `√G_E` and as `(t/c)^1.5`. Thinner air
raises it: at a fixed speed of sound, `V_f` goes as `1/√p`.

**A flutter dynamic pressure.** The air enters only through `ρ a² = γ p`, and the dynamic pressure
is `q = ½ ρ V²`. So the criterion fixes the dynamic pressure at flutter, the same at every height:

```text
q_f = π G_E / (24 ε X (λ + 1)),    X = A³ / ((t/c)³ (A + 2))
```

A fin flying at dynamic pressure `q` is below its flutter speed by the ratio `V_f / V = √(q_f / q)`.
That makes the flight's least ratio the one at its peak dynamic pressure, "max q", which
[Flight metrics](metrics.md#peaks) already finds between the integrator's steps.

**Loft's mistake.** Loft, hpr's predecessor, wrote the constant as `1.337 · (λ + 1)/2` per psi.
That is half of `39.3 / 14.696 = 2.674`, so its flutter speeds were `√2`, about 41%, too high: on
the unsafe side ([L32](../decisions-and-roadmap.md#l32), a lesson from Loft).

## Martin's worked examples

Martin illustrates the criterion with two examples read off his figure 4 (pp. 6–7):

| Example | Martin reads | Eq. 19 gives | Martin's rounding |
|---|---|---|---|
| `X` for `A = 2` and 4% thickness | "about 1.25 × 10⁶" psi | 1.228 × 10⁶ psi | to 0.05 × 10⁶ |
| Titanium held to 0.8 × 10⁶ psi, `A = 1` | 2.5% thick | 2.54% | to ½% |
| Same, `A = 2` | 4.5% | 4.61% | to ½% |
| Same, `A = 3` | "about 6.5" % | 6.43% | to ½% |

Each value hpr computes rounds to what Martin printed, at the resolution he printed it. The small
differences are chart reading: Martin read them off a log-scale plot.

## A rocket's fins

The example program `fin_flutter` (in `crates/hpr/examples/`) takes the repository's synthetic
54 mm rocket. Its three fins are 3.2 mm thick, with a 150 mm root, a 60 mm tip and a 70 mm span.
It flies the rocket on an I175 motor and prints:

<!-- quote: crates/hpr/examples/fin_flutter.output.txt -->
```text
Panel: aspect ratio 0.667, taper ratio 0.400, thickness ratio 0.0212
Max q: 72455 Pa at 2.09 s, 432 m above the pad

material            G (GPa)   q_f (kPa)   V_f 0 m (m/s)   V_f 3 km (m/s)   V_f/V at max q
aluminum_6061        26.200       836.3            1169             1356             3.40
carbon_fiber          4.826       154.1             502              582             1.46
birch_plywood         0.750        23.9             198              229             0.57
basswood              0.511        16.3             163              189             0.47
balsa                 0.138         4.4              85               99             0.25
```

By this criterion the aluminium and carbon fins stay well below their flutter speed. Plywood,
basswood and balsa fins of this size would flutter on this motor: at max q the rocket flies at
about twice plywood's flutter speed. Read `V_f/V` below 1 as "don't fly this". Read a value just
above 1 as "not proven safe", given the scatter in Martin's data. CI checks that the program still
prints exactly this.

In code, with a [`FlutterPanel`](../api/hpr_sim/flutter/struct.FlutterPanel.html):

```rust,ignore
let panel = FlutterPanel::of_fins(&fin_set)?;
let g = hpr_design::materials::shear_modulus("birch_plywood").unwrap().shear_modulus_pa;
let q_f = panel.flutter_dynamic_pressure_pa(g)?;
let v_f = panel.flutter_speed_m_s(g, pressure_pa, sound_speed_m_s)?;
let margin = panel.margin(g, &summary)?; // at the flight's max q; `None` if it never flew
```

`FlutterPanel::new` takes `A`, `λ` and `t/c` directly, for a fin that isn't a trapezoid if you can
find an equivalent one. Its API page has a worked example that CI runs.

## Readings where the source leaves room

Each of these gives the lower of the flutter speeds the source allows.

- **Thickness ratio at the root.** Martin's wings keep one thickness ratio from root to tip. A
  hobby fin keeps one thickness, so its ratio grows toward the tip. hpr takes the root's, the
  smallest.
- **A solid fin's `G_E` is its material's `G`.** Martin says a solid wing of aluminium plots at
  aluminium's modulus (p. 6), and hpr does the same. His definition, `G_E = 6 J G / (c t³)` (eq.
  12), assumes a thin airfoil's torsion constant `J ≈ c t³/6` (eq. 10). A flat plate's is `c t³/3`,
  which would double `G_E` and raise the flutter speed by `√2`. hpr keeps the lower reading.
- **Martin's taper factor.** His derivation has the factor `1/(f₁² f₂²)` from the torsional
  frequency and the chord three-quarters out (eqs. 8 and 14). He replaces it with `(λ + 1)/2`.
  The two agree at `λ = 1` and are 3% apart at `λ = 0`. Between, his is up to 47% larger (at
  `λ ≈ 0.31`), which lowers the flutter speed there by up to 17%. hpr keeps his form, since his
  figure 3 was drawn with it.
- **Every fin set sees the whole flight's max q.** A booster's fins leave at the separation, so the
  flight's max q may come after they are gone; their true margin is at least the one given.

## Shear moduli

The criterion needs the fin's shear modulus in its own plane. `hpr_design::materials::SHEAR_MODULI`
gives it for 14 built-in materials, each with its source, page and the web address it was read
from. Where a source gives a range, the lower value is kept.

| Material | `G`, GPa | Source | Basis |
|---|---|---|---|
| Aluminium 6061 | 26.2 | MIL-HDBK-5J, Table 3.6.2.0(b1): 3.8 × 10³ ksi | stated |
| Aluminium 7075 | 26.9 | MIL-HDBK-5J, Table 3.7.6.0(b1): 3.9 × 10³ ksi | stated |
| Steel (plain carbon) | 75.8 | MIL-HDBK-5J, Table 2.2.1.0(b): 11.0 × 10³ ksi | stated |
| Titanium Ti-6Al-4V | 42.7 | MIL-HDBK-5J, Table 5.4.1.0(b): 6.2 × 10³ ksi (TIMET: 6.2 and 6.66 × 10⁶ psi) | stated |
| Carbon fibre/epoxy | 4.83 | NCAMP report on Hexcel 8552 AS4 tape: in-plane `G₁₂` 0.70 Msi, room temperature | stated |
| Birch plywood | 0.750 | Riga Wood *Plywood Handbook*, Table 4.11: panel shear | stated |
| Birch (yellow) | 1.04 | Wood Handbook ratio × bending modulus × 1.10 | derived |
| Oak (northern red) | 1.11 | same | derived |
| Maple (sugar) | 0.873 | same | derived |
| Spruce (Sitka) | 0.725 | same | derived |
| Basswood | 0.511 | same | derived |
| Balsa | 0.138 | same | derived |
| Acetal (Delrin) | 1.06 | data sheet `E / (2(1 + ν))` | derived |
| Nylon 6/6 | 0.490 | data sheet `E / (2(1 + ν))`, conditioned | derived |

How the derived values are made:

- **Woods:** the Wood Handbook's ratio `G_LT / E_L` (Table 5-1), times its bending modulus at 12%
  moisture raised by 10%, as the table's footnote says to estimate `E_L`. `G_LT` is the smaller of
  the two in-plane ratios for every wood listed. Eastern white pine is not in Table 5-1, so it has
  none.
- **Plastics:** an unfilled plastic taken as isotropic, from its data sheet's tensile modulus and
  Poisson's ratio. Nylon soaks up water; the conditioned value is less than half the dry one.
- **Carbon fibre:** a unidirectional ply's in-plane `G₁₂`. That is a 0/90 laminate's in-plane
  shear modulus. Plies at ±45° raise it several times, so for such a laminate this reads low.

No source found states a shear modulus for G10/FR-4, the commonest fibreglass fin sheet: its data
sheets give flexural moduli only. There is none for PLA, ABS, PETG, polycarbonate or acrylic
either. For those, pass your own `G_E` from a measurement or your supplier. A woven glass/epoxy
laminate is not isotropic, so `E / (2(1 + ν))` doesn't apply to it.

## Tests

In `crates/hpr-sim/src/flutter.rs`, unless named otherwise:

| Test | What it pins |
|---|---|
| `flutter_denominator_matches_tn_4197_eq_18` | The constant against Martin's 39.3 psi, and the denominator against eq. 18 on three panels at two pressures, to his rounding; twice Loft's constant ([L32](../decisions-and-roadmap.md#l32)) |
| `scaling_laws_in_thickness_shear_modulus_and_pressure` | `V_f` as `(t/c)^1.5`, `√G_E`, `1/√p` and `a`, to 1e-12; `q_f` the same at three atmospheres |
| `martins_worked_examples` | Both of Martin's examples at the resolution he printed them (the table above) |
| `taper_factor_against_the_frequency_factors` | `(λ + 1)/2` against `1/(f₁² f₂²)`: 3% at `λ = 0`, at most 47% at `λ = 0.308` |
| `the_margin_is_the_flights_least_at_its_peak_dynamic_pressure` | On Valetudo's flight, the margin is at max q and at or below every row of a 1 ms record, and eq. 18 at each row's own pressure and speed of sound gives the same ratio, to 1e-12 |
| `a_trapezoidal_fin_set_gives_its_panel`, `out_of_range_inputs_are_refused` | A fin set's `A`, `λ` and `t/c`; an elliptical fin, a taper outside 0 to 1 and non-positive inputs are refused, each by name |
| `hpr_design::materials::tests::shear_moduli_reproduce_the_sources` | Each built-in modulus against its source's numbers |

## References

- D. J. Martin, *Summary of Flutter Experiences as a Guide to the Preliminary Design of Lifting
  Surfaces on Missiles*, NACA TN 4197, 1958. Appendix, eqs. 1 to 19, pp. 11–15; examples, pp. 6–7;
  figures 3 and 4, p. 19.
- MIL-HDBK-5J, *Metallic Materials and Elements for Aerospace Vehicle Structures*, 2003.
- Forest Products Laboratory, *Wood Handbook*, FPL-GTR-190, 2010, Tables 5-1, 5-3a and 5-5a.
- NCAMP, *Hexcel 8552 AS4 Unidirectional Prepreg Qualification Statistical Analysis Report*,
  NCP-RP-2010-008 Rev D, 2011, Table 3-3.
- Riga Wood, *Plywood Handbook*, 2022, Table 4.11.
- Celanese, Zytel 101L NC010 data sheet, 2023; Delrin 100P NC010 data sheet.

The decision behind these choices is [ADR-078][adr-078], from the
[M1.10b milestone](../decisions-and-roadmap.md#m1-10b).

[adr-078]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-078-fin-flutter-by-naca-tn-4197-the-lower-reading-wherever-the-source-leaves-room-2026-09-26
