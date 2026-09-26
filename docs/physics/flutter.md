# Fin flutter

## In short

- **What it models:** whether a fin may [flutter](../glossary.md#flutter), meaning its bending and
  twisting feed each other until the fin shakes itself apart. hpr gives two readings of one
  criterion. The first is Martin's own chart check: is the fin on the flutter side of the line his
  flight data draws? The second is the ratio of the criterion's flutter speed to the rocket's
  airspeed at the flight's peak [dynamic pressure](../glossary.md#dynamic-pressure).
- **Sources:** D. J. Martin's criterion in NACA TN 4197 (1958), eq. 18, and his figure 3, which
  separates missile and wind-tunnel wings that fluttered from those that didn't
  ([References](#references)). The fin's stiffness enters as its
  [shear modulus](../glossary.md#shear-modulus); hpr has one, with its source, for 14 of its
  built-in materials.
- **How well it is validated:** hpr reproduces Martin's formula and both of his worked examples,
  including his verdicts on three metals and his titanium design ([Tests](#tests)). His
  safe/unsafe line is a band that hpr measured off his printed chart: 0.25 to 0.31
  ([The line in Martin's data](#the-line-in-martins-data)). His unfailed wings flew to at least
  Mach 1.3; nothing here is checked against a hobby rocket. Where the source allows two readings, hpr takes
  the one with the lower flutter speed. That doesn't make the whole result conservative: nothing
  shows that it is.
- **What it leaves out:** sweep, and how the fin is mounted: Martin's wings were clamped at the
  root, and a fin glued to a tube is less stiff there. Also left out are stall flutter at high
  angles of attack, Mach-number effects such as a dip near Mach 1, the rocket body's own bending,
  and fins that aren't trapezoids. G10/FR-4, the commonest fibreglass fin sheet, has no built-in
  modulus ([Shear moduli](#shear-moduli)).

## What flutter is

Air pushing on a fin twists it a little. The twist changes the fin's angle to the air, so the push
changes, and the fin bends. Below a certain speed the fin's stiffness damps this out. Above it, each
cycle feeds the next, and a fin can fail within a second. The speed depends on the fin's shape and
thickness, how stiff its material is in shear, and the air it flies through.

## The criterion

Martin starts from Theodorsen and Garrick's flutter speed for a wing that bends and twists (his
eq. 1, from their NACA Report 685 of 1940). He reduces it to a few numbers from one fin's outline.
For a trapezoidal fin with root chord `c_r`, tip chord `c_t`, span `s` and thickness `t`:

| Symbol | Meaning | For a trapezoid |
|---|---|---|
| `A` | Panel aspect ratio: span over the chord halfway out | `2s / (c_r + c_t)` |
| `λ` | Taper ratio: tip chord over root chord, 0 to 1 | `c_t / c_r` |
| `t/c` | Thickness ratio | `t / c_r` |
| `G_E` | The fin's effective shear modulus, Pa | the material's ([Shear moduli](#shear-moduli)) |
| `p`, `a` | The air's static pressure and speed of sound | from the atmosphere |
| `ε` | Where the section's mass sits: this fraction of the chord behind the quarter chord | 0.25, at mid-chord |
| `γ` | Air's ratio of specific heats | 1.4 |

Eq. 18 gives a flutter speed `V_f` through a denominator `D`, in pascals:

```text
(V_f / a)² = G_E / D,    D = (24 ε γ / π) · p · K · (λ + 1)/2,    K = A³ / ((t/c)³ (A + 2))
```

Martin prints the constant in pounds per square inch (psi) at sea-level pressure `p₀`:
`24 · 0.25 · 1.4 / π · 14.696 psi = 39.29 psi`, which he rounds to 39.3. His `X` (eq. 19) is
`39.3 K` psi, and `D = X · (λ + 1)/2 · p/p₀`. The constant is derived, not fitted. Stiffer or
thicker fins flutter faster: `V_f` grows as `√G_E` and as `(t/c)^1.5`. At a fixed speed of sound,
thinner air raises it as `1/√p`.

**A flutter dynamic pressure.** The air enters only through `ρ a² = γ p`, and the dynamic pressure
is `q = ½ ρ V²`. So the criterion fixes a dynamic pressure at `V_f`, the same at every height:

```text
q_f = π G_E / (24 ε K (λ + 1))
```

A fin flying at dynamic pressure `q` is below `V_f` by the ratio `V_f / V = √(q_f / q)`. That makes
the flight's least ratio the one at its peak dynamic pressure, "max q", which
[Flight metrics](metrics.md#peaks) already finds between the integrator's steps.

## The line in Martin's data

Eq. 18's `V_f` is not the speed at which a fin is known to flutter. Martin plots `D` against `G_E`
for missiles and wind-tunnel models (his figure 3). Wings that fluttered or failed lie above a
shaded band, and wings that flew to at least Mach 1.3 without known failure lie below it.

hpr measured the band on a 250 dots-per-inch scan of the figure. Both log axes were calibrated on
their tick marks, and the band's edges were traced in 69 pixel columns. The band runs at
`D / G_E` = 0.25 to 0.31 along the whole axis, from wood to steel. In eq. 18's terms that is
`(V_f/a)²` = 3.2 to 4.0, so Martin's line sits where `V_f` is 1.8 to 2.0 times the speed of sound.
hpr calls this ratio `D / G_E` the *figure 3 ratio*:

| Figure 3 ratio `D / G_E` | Martin's data |
|---|---|
| above 0.31 | mostly wings that fluttered or failed (a few that didn't lie there too) |
| 0.25 to 0.31 | the band: marginal |
| below 0.25 | wings that flew to at least Mach 1.3 without known failure |

Martin takes `p` where the wing flies. At the launch site's pressure, the highest a flight sees, the
ratio is at its largest. Figure 3's axis runs from 0.05 to 10 × 10⁶ psi (0.34 to 69 GPa) in `G_E`;
balsa's modulus is left of it, so its ratio is an extrapolation.

**Which reading decides.** The figure 3 ratio does: it is Martin's own check, and a fin must be
below the band. `V_f / V` alone is not a safe line, at 1 or at any fixed number. At the flight's
max q the two are tied by `D / G_E = 1 / (M · V_f/V)²`, with `M` the Mach number there, so the
band's edge is at `V_f / V` of about 1.8/M to 2.0/M: 1.5 at Mach 1.3, but 3.6 at Mach 0.5.
Martin's data show nothing about a fin above the band on a rocket slower than Mach 1.3: treat it as
not shown to be safe.

**Loft's mistake.** Loft, hpr's predecessor, wrote the constant as `1.337 · (λ + 1)/2` per psi,
half of `39.3 / 14.696 = 2.674`. So its flutter speeds were `√2` too high, about 41%, on the unsafe
side ([L32](../decisions-and-roadmap.md#l32), a lesson from Loft).

## Martin's worked examples

Martin gives two examples (pp. 6–7). The first is a wing with `A = 2`, 4% thick, untapered and
ground-launched. He reads its `X` off his figure 4 as "about 1.25 × 10⁶ psi" and judges the wing by
material. The second picks titanium and holds figure 3's ordinate to 0.8 × 10⁶ psi, well under
titanium's modulus, "to allow a reasonable margin of safety". It then asks how thick the wing must
be at each aspect ratio.

| Example | Martin | hpr, from eq. 19 | Difference |
|---|---|---|---|
| `X` for `A = 2`, 4% thick | "about 1.25 × 10⁶" psi | 1.228 × 10⁶ psi | −1.8% |
| Titanium at 0.8 × 10⁶ psi, `A = 1` | 2.5% thick | 2.54% | +1.6% |
| Same, `A = 2` | 4.5% | 4.61% | +2.4% |
| Same, `A = 3` | "about 6.5"% | 6.43% | −1.1% |

Martin read these off a log-scale chart and printed them on grids of 0.05 × 10⁶ psi and half a
percent (his thicknesses all end in .5). Each of hpr's values rounds to his on that grid.

His verdicts on the first wing are the margin half of the example; the titanium row is his second
example, at the ordinate he chose. hpr checks them with the moduli Martin marks on figure 3's axis,
each a small box read off the same scan:

| Material | Martin's mark, 10⁶ psi | Figure 3 ratio | Martin says | Against the band |
|---|---|---|---|---|
| Magnesium | 2.40 to 2.63 | 0.47 to 0.51 | "in the flutter region" | above |
| Aluminium | 3.82 to 4.28 | 0.29 to 0.32 | "marginal" | on it |
| Steel | 8.92 to 11.3 | 0.11 to 0.14 | "probably safe" | below |
| Titanium, his second example at 0.8 × 10⁶ psi | 5.78 to 6.34 | 0.13 to 0.14 | his design, with "a reasonable margin of safety" | below |

## A rocket's fins

The example program `fin_flutter` (in `crates/hpr/examples/`) takes the repository's synthetic
54 mm rocket. Its three fins are 3.2 mm thick, with a 150 mm root, a 60 mm tip and a 70 mm span.
It flies the rocket on an I175 motor from a site 200 m up, and prints:

<!-- quote: crates/hpr/examples/fin_flutter.output.txt -->
```text
Panel: aspect ratio 0.667, taper ratio 0.400, thickness ratio 0.0212
Max q: 72455 Pa at 2.09 s, 432 m above the pad
Top speed: 355 m/s at 2.10 s; top Mach number: 1.05 at 2.10 s
Martin's band (figure 3): D/G_E from 0.25 to 0.31

material            G (GPa)   q_f (kPa)   V_f 0 m (m/s)   V_f 3 km (m/s)   V_f/V at max q   D/G_E
aluminum_6061        26.200       836.3            1169             1356             3.40   0.083
carbon_fiber          4.826       154.1             502              582             1.46   0.450
birch_plywood         0.750        23.9             198              229             0.57   2.893
basswood              0.511        16.3             163              189             0.47   4.246
balsa                 0.138         4.4              85               99             0.25  15.680
```

The columns `V_f 0 m` and `V_f 3 km` are eq. 18's flutter speed in standard air at sea level and
3 km above it. `D/G_E` is the figure 3 ratio at the launch site's pressure.

- **Aluminium** passes both readings: its figure 3 ratio is well below the band, and at max q the
  rocket flies at under a third of `V_f`.
- **Carbon fibre** at 3.2 mm has `V_f / V` of 1.46, but its figure 3 ratio, 0.45, is above the
  band, where Martin's wings failed: not shown to be safe. Its modulus is a unidirectional ply's of
  one aerospace prepreg; a ±45° layup of it would be stiffer, but wet-laid or woven hobby sheet may
  be softer.
- **Plywood, basswood and balsa** fins of this size fail both: at max q the rocket flies at 1.75
  times plywood's `V_f`.

CI checks that the program still prints exactly this.

In code, with a [`FlutterPanel`](../api/hpr_sim/flutter/struct.FlutterPanel.html) (its API page
has a worked example that CI runs):

```rust,ignore
// `fin_set` is a design's `FinSet`; `summary` a flight's `FlightSummary` (Flight metrics).
let panel = FlutterPanel::of_fins(&fin_set)?;
let g = materials::shear_modulus_of(&fin_set.material).unwrap().shear_modulus_pa;
let chart = panel.figure_3_ratio(g, launch_pressure_pa)?; // against FIGURE_3_BAND
let margin = panel.margin(g, &summary)?; // V_f/V at max q; `None` if it never flew
```

`FlutterPanel::new` takes `A`, `λ` and `t/c` directly. Martin's figure 4 covers `A` from 0.5 to 3
and `t/c` from 1% to 10%; the example's 0.667 is inside, and outside that range the numbers are an
extrapolation.

## Readings where the source leaves room

Most of these give the lower of the flutter speeds the source allows. Two can go the other way:
an airfoiled fin's modulus, and a booster's max q.

- **Thickness ratio at the root.** Martin's wings keep one thickness ratio from root to tip. A
  hobby fin keeps one thickness, so its ratio grows toward the tip. hpr takes the root's, the
  smallest.
- **A solid fin's `G_E` is its material's `G`.** Martin says a solid wing of aluminium plots at
  aluminium's modulus (p. 6), and hpr does the same. His definition, `G_E = 6 J G / (c t³)` (eq.
  12), assumes a thin airfoil's torsion constant `J ≈ c t³/6` (eq. 10). A flat plate's is `c t³/3`,
  which would double `G_E` and raise the flutter speed by `√2`. hpr keeps the lower reading. The
  exception runs the other way: for a fin with an airfoil section, eq. 12 gives `0.946 G`, so hpr's
  `V_f` for such a fin is up to 2.7% high.
- **Martin's taper factor.** His derivation carries a factor `1/(f₁² f₂²)`. Here `f₁` corrects the
  twisting frequency for taper (his eq. 8) and `f₂` gives the chord three-quarters of the way out
  (his eq. 14). He replaces the factor with `(λ + 1)/2`. The two agree at `λ = 1` and are 3% apart
  at `λ = 0`. Between, `(λ + 1)/2` is up to 47% larger (at `λ ≈ 0.31`), which lowers `V_f` there by
  up to 17.5%. hpr keeps his form, since his figure 3 was drawn with it.
- **Every fin set sees the whole flight's max q.** A booster's fins leave at the separation, so the
  flight's max q can come after they are gone. Their true `V_f / V` is then at least the one given,
  as long as the booster's own dynamic pressure after the separation stays below the flight's
  peak. hpr doesn't check that.

## Shear moduli

The criterion needs the fin's shear modulus in its own plane. `hpr_design::materials::SHEAR_MODULI`
gives it for 14 built-in materials, each with its source, page and the web address it was read
from, and `shear_modulus_of` finds a design's copy of a built-in material by its name and density. Where a source gives a
range, the lower value is kept. Metals are given in ksi (1000 psi) and Msi (10⁶ psi); 1 Msi is
6.895 GPa.

| Material | `G`, GPa | Source | Basis |
|---|---|---|---|
| Aluminium 6061 | 26.2 | MIL-HDBK-5J, Table 3.6.2.0(b1): 3.8 × 10³ ksi | stated |
| Aluminium 7075 | 26.9 | MIL-HDBK-5J, Table 3.7.6.0(b1): 3.9 × 10³ ksi | stated |
| Steel (plain carbon) | 75.8 | MIL-HDBK-5J, Table 2.2.1.0(b): 11.0 × 10³ ksi | stated |
| Titanium Ti-6Al-4V | 42.7 | MIL-HDBK-5J, Table 5.4.1.0(b): 6.2 × 10³ ksi (TIMET: 6.2 and 6.66 Msi) | stated |
| Carbon fibre/epoxy | 4.83 | NCAMP (a composite-material qualification programme) report on Hexcel 8552 AS4 tape: in-plane `G₁₂` 0.70 Msi, room temperature | stated |
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

- **Woods:** `G_LT` is the shear modulus in the plane along the grain and along the growth rings.
  hpr takes the Wood Handbook's ratio `G_LT / E_L` (Table 5-1) and multiplies it by the wood's
  bending modulus at 12% moisture raised by 10%, which is how the table's footnote says to estimate
  the stiffness along the grain, `E_L`. `G_LT` is the smaller of the two in-plane ratios for every
  wood listed. Eastern white pine is not in Table 5-1, so it has none. Martin marks solid wood at
  0.070 to 0.120 × 10⁶ psi (0.48 to 0.83 GPa); the handbook's birch and oak are above that.
- **Plastics:** an unfilled plastic taken as isotropic (the same in every direction), from its data
  sheet's tensile modulus `E` and Poisson's ratio `ν`. Nylon soaks up water. Its *conditioned*
  value, measured after it has, is less than half the dry one.
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
| `flutter_denominator_matches_tn_4197_eq_18` | The constant against Martin's 39.3 psi, and `D` against eq. 18 on three panels at two pressures, to his rounding; twice Loft's constant ([L32](../decisions-and-roadmap.md#l32)) |
| `martins_worked_examples` | Both of Martin's examples on his grid, and his three verdicts and his titanium design against the band, through `figure_3_ratio` (the tables above) |
| `scaling_laws_in_thickness_shear_modulus_and_pressure` | `V_f` as `(t/c)^1.5`, `√G_E`, `1/√p` and `a`, to 1e-12; `q_f` the same in three atmospheres |
| `taper_factor_against_the_frequency_factors` | `(λ + 1)/2` against `1/(f₁² f₂²)`: 3% at `λ = 0`, at most 47% at `λ = 0.308` |
| `the_margin_is_the_flights_least_at_its_peak_dynamic_pressure` | On the flight of Valetudo (a RocketPy example rocket), the margin is at max q and at or below every row of a 1 ms record; `V_f` from each row's pressure and speed of sound agrees with `√(q_f/q)`, to 1e-12; no peak gives `None`, a peak that isn't a number is refused |
| `a_trapezoidal_fin_set_gives_its_panel`, `out_of_range_inputs_are_refused` | A fin set's `A`, `λ` and `t/c`; elliptical and reverse-tapered fins, a taper outside 0 to 1 and inputs that aren't positive are refused by name, from code and from JSON |
| `hpr_design::materials::tests::shear_moduli_reproduce_the_sources` | Each built-in modulus against its source's numbers |

## References

- D. J. Martin, *Summary of Flutter Experiences as a Guide to the Preliminary Design of Lifting
  Surfaces on Missiles*, NACA TN 4197, 1958. Appendix, eqs. 1 to 19, pp. 11–15; examples, pp. 6–7;
  figures 3 and 4, p. 19.
- T. Theodorsen and I. E. Garrick, *Mechanism of Flutter: A Theoretical and Experimental
  Investigation of the Flutter Problem*, NACA Report 685, 1940. Martin's ref. 6, the source of his
  eq. 1; not read for hpr.
- MIL-HDBK-5J, *Metallic Materials and Elements for Aerospace Vehicle Structures*, 2003.
- Forest Products Laboratory, *Wood Handbook*, FPL-GTR-190, 2010, Tables 5-1, 5-3a and 5-5a.
- NCAMP, *Hexcel 8552 AS4 Unidirectional Prepreg Qualification Statistical Analysis Report*,
  NCP-RP-2010-008 Rev D, 2011, Table 3-3.
- Riga Wood, *Plywood Handbook*, 2022, Table 4.11.
- Celanese, Zytel 101L NC010 data sheet, 2023; Delrin 100P NC010 data sheet.

The decision behind these choices is [ADR-078][adr-078], from the
[M1.10b milestone](../decisions-and-roadmap.md#m1-10b).

[adr-078]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-078-fin-flutter-by-naca-tn-4197-the-lower-reading-wherever-the-source-leaves-room-2026-09-26
