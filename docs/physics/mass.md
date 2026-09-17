# Mass properties of components

Code: `hpr_design::mass` (`MassProperties`), `hpr_design::parts`, `hpr_design::fins`,
`hpr_design::material`, `hpr_design::materials`. The nose and transition solids are in
`shapes.md`.

Sources:

- **[MK]** J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*, appendix B (moments
  of inertia of standard solids, the parallel-axis theorem and the inertia tensor).
- **[GVL]** G. H. Golub and C. F. Van Loan, *Matrix Computations*, 4th ed. (2013), §8.5 (the
  Jacobi eigenvalue method).
- **[TD]** S. Niskanen, *OpenRocket technical documentation* v13.05 (2013), §3.2.2, pp. 25–29 (fin
  planforms), §3.4.4, pp. 49–50 (cross-sections), §4.2.3, p. 66 and Table 5.1, p. 75 (component
  masses), pinned as `openrocket-techdoc-13.05`.
- **[AvD]** I. H. Abbott and A. E. von Doenhoff, *Theory of Wing Sections*, Dover (1959), eq. 6.2
  (NACA four-digit thickness distribution).

## Frames and conventions (ADR-006)

- **Body axes** follow `frames.md`: `z` along the axis toward the nose, `x` the zero radial
  direction, `y = z × x`. Roll angles run from `x` toward `y`.
- **A component's frame** has body axes and its origin on the axis at the component's forward end,
  or at a nose cone's tip, so the component lies at `z ≤ 0`. The design tree (M1.4b) places it by
  translating and rolling.
- **`MassProperties`** holds the mass, the centre of mass in body axes, and the **full** inertia
  tensor about the centre of mass. The tensor is taken with the positive products-of-inertia
  convention: `I = ∫ (|r|² E − r rᵀ) dm`, so `I_xy = −∫ x y dm`.
- **Operations** ([MK]):
  - Parallel axis: `I_p = I_cg + m (|d|² E − d dᵀ)`, with `d = cg − p`.
  - Rotation: `cg′ = R cg`, `I′ = R I Rᵀ`.
  - Combination: sum the masses, mass-weight the centres, and sum each tensor moved to the common
    centre.
  - Zero total mass gives the plain average of the centres, so placeholders stay finite.
- **Validity.** `validate` requires a finite, non-negative mass, and a tensor that is symmetric (to
  1e-9 of its largest entry) with non-negative principal moments obeying `I₁ + I₂ ≥ I₃`. That
  condition is the same as `J = tr(I)/2 E − I = ∫ r rᵀ dm` being positive semidefinite. Principal
  moments come from cyclic Jacobi ([GVL] algorithm 8.5.1), accurate for repeated eigenvalues where
  the closed-form trigonometric method loses `√ε`.

## Standard solids ([MK])

- **Hollow cylinder**, radii `R > r`, length `L`: `I_axis = m(R² + r²)/2` and
  `I_across = m((R² + r²)/4 + L²/12)`. This covers body tubes, inner tubes and couplers, centering
  rings, bulkheads (`r = 0`), launch lugs, tube fins, and shoulders.
  - Loft used `mL²/12` with no radial term, and no roll inertia at all (lesson L44).
- **Solid cylinder**, radius `a`, height `h`: `I_axis = m a²/2` and `I_across = m(3a² + h²)/12`.
  This covers mass components, packed parachutes, streamers and shock cords ([TD] Table 5.1
  treats recovery parts as cylinders too), and each disc of a rail button.
- **Rail button:**
  - Three coaxial discs stacked outward on a radial line: base (outer diameter), waist (inner
    diameter), flange (outer diameter).
  - The waist height is the total height less the base and flange.
  - Buttons and lugs may repeat along the axis (`count`, `spacing_m`).
- **Shoulder:** a hollow cylinder beyond the profile's end. A capped shoulder adds a disc of its
  inner radius and wall thickness, flush with its far end. The step ring between a nose's base
  radius and its shoulder is not modeled.
- **Parachute:** `m = ρ_s π D²/4 + n ℓ ρ_l`, the nominal area of a flat circular canopy plus its
  shroud lines. A conical or hemispherical canopy has more cloth than `πD²/4`; give its mass
  through an override (M1.4b) or a matching nominal diameter.
- **Streamer:** `ρ_s × length × width`. **Shock cord:** `ρ_l × length`.

## Fins

- **Planforms** ([TD] §3.2.2):
  - Trapezoidal: root chord `c_r`, tip chord `c_t` parallel to the body, span `s`, and sweep `x_t`
    from the root leading edge to the tip leading edge.
  - Elliptical: `c(h) = c_r √(1 − (h/s)²)`, centred on the root chord (implied by [TD] eq. 3.71).
  - Freeform: a simple polygon from the root leading edge `[0, 0]` to the root trailing edge
    `[c_r, 0]`, closed along the root. Crossing edges, points below the root, and outlines that
    don't run from the origin aft along the root are errors.
- **Cross-sections.** Each chord from `a` to `b` has a thickness distribution `t(x)`. [TD] uses
  the cross-section for drag only; hpr also counts the volume it removes.
  - **Square:** `t(x) = t`.
  - **Rounded:** semicircular edges of radius `a_r = min(t, c)/2`, so a chord shorter than `t`
    near a pointed tip is a disc of diameter `c`. Its moments are closed forms in `a_r`:
    `D₀ = a_r²(2 − π/2)`, `D₁ = a_r D₀ − a_r³/3`, `D₂ = a_r² D₀ − π a_r⁴/8` for the removed edge
    material, and `E₀ = 8a_r⁴ − 3π a_r⁴/2` for `∫t³`. A wide rounded chord loses
    `(1 − π/4) t²` of section area.
  - **Airfoil:** `t(x) = 10 t P(ξ)` with the NACA four-digit polynomial
    `P = 0.2969√ξ − 0.1260ξ − 0.3516ξ² + 0.2843ξ³ − 0.1015ξ⁴` ([AvD]).
    - Its maximum is `1.0003 t` at `ξ = 0.2998`.
    - Its moments `10∫ξᵏP = 0.685083, 0.288033, 0.158919` hold term by term, and
      `1000∫P³ = 0.4728895` comes from mpmath.
    - An airfoiled fin weighs 68.5% of the square slab, with its centroid at 42% chord.
    - A hand-sanded "airfoil" is between the two; [TD] doesn't define the section.
- **Integrals.** Per fin, over the span `h` with `r = R_b + h`, and chordwise moments
  `M_k = ∫ x^k t dx` and `T = ∫ t³/12 dx`, taken per unit density:
  `V = ∫M₀`, `∫r = ∫rM₀`, `∫r² = ∫r²M₀`, `∫x = ∫M₁`, `∫x² = ∫M₂`, `∫rx = ∫rM₁` and `∫τ² = ∫T`.
  The span integration is split at every vertex height and runs adaptively.
  - With the fin at roll 0 (points at `(r, τ, −x)`):
    `I_xx = ∫(τ² + x²)`, `I_yy = ∫(r² + x²)`, `I_zz = ∫(r² + τ²)` and `I_xz = ∫ r x`, all `dm`.
  - Loft ignored the span and fixed a freeform fin's CG at `0.42 c_r` (lessons L44, L45).
- **Tabs** are square slabs below the root, `−h_tab ≤ h ≤ 0`, with closed-form integrals. A tab
  must lie along the root chord and reach no deeper than the body radius. Loft never
  read them (lesson L46).
- **Root.** The flat root is placed at radius `R_b`; the sliver between it and the curved tube,
  `t²/8R_b` deep, is ignored. Fillets are not modeled yet.
- **Cant** `δ` turns each fin and its tab about the fin's outward span axis through the root
  mid-chord, right-handed, so a positive cant turns fin 0's leading edge toward `−y_B`
  (`positive_cant_turns_the_leading_edge_toward_negative_y`). [TD] doesn't state the pivot. Mass and trace are unchanged; the
  products of inertia in the fin's own frame grow as `sin 2δ`.
- **Sets** roll the fin to `φ_k = φ₀ + 2πk/N` and combine. Three or more fins are isotropic across
  the axis; one or two are not, and the full tensor keeps the difference.

## Materials

`Density` is `bulk` (kg/m³), `surface` (kg/m²) or `line` (kg/m). A part asking for the wrong kind
is an error. A design stores the values, not a library key. The built-in values and their sources
are in `hpr_design::materials` and summarized below.

`hpr_design::materials::BUILTIN` holds 49 materials. Each carries its source (with table or page),
the URL it was read from, and a basis:

- **published:** the source states the value.
- **derived:** computed from the source's numbers.
- **maximum:** a specification's upper weight limit.
- **vendor:** a seller's figure, used where no specification exists.

| group | values | sources | basis |
|---|---|---|---|
| hobby tubes | cardboard 790, kraft phenolic 950, Blue Tube 1250, Quantum 1090 kg/m³ | LOC Precision and Public Missiles weight tables (mass over wall volume); Always Ready Rocketry's own material file | derived; published |
| composites | G10/FR-4 1800, filament-wound E-glass 1990, carbon/epoxy 1580 kg/m³ | Norplex-Micarta NP130, Comptec, Hexcel HexPly 8552 data sheets | published |
| metals | Al 6061 2700, Al 7075 2800, steel 7850, Ti-6Al-4V 4430, brass 8500 kg/m³ | Kaiser Aluminum, MIL-HDBK-5J, TIMET, Copper Development Association | published |
| woods | balsa 180; basswood, yellow birch, Sitka spruce, eastern white pine, sugar maple, northern red oak from `G₁₂`; birch plywood 680 kg/m³ | Wood Handbook FPL-GTR-190 (pinned as `fpl-gtr-190-wood-handbook`), p. 2-21 and Table 5-3a; Riga Wood Plywood Handbook | published; derived |
| plastics | PLA 1240, ABS 1040, PETG 1270, nylon 6/6 1140, PC 1200, PMMA 1190, acetal 1420, PS 1040, PVC 1400, HDPE 955, epoxy 1180, Depron 40, paper 755 kg/m³ | manufacturers' data sheets (NatureWorks, SABIC, Eastman, Celanese, Covestro, Röhm, AmSty, Charlotte Pipe, Chevron Phillips, West System, Depron, HP) | published (paper derived) |
| fabrics | ripstop 1.1 and 1.6 oz/yd², Mylar and LDPE film at 1 mil, paper 80 g/m², Nomex cloth, silnylon | MIL-C-7020H, DuPont Teijin, Dow, HP, MIL-C-83429B; a seller for silnylon | maximum, derived, published, vendor |
| cords | nylon cord types I and III, tubular nylon ½", 9/16", 1", ⅛" and ¼" Kevlar, ¼" bungee, Tex 80 Kevlar thread | MIL-C-5040H, MIL-W-5625K, MIL-C-5651D, A-A-55220; Giant Leap Rocketry's measurements for Kevlar | maximum, vendor, published |

- **Wood** at 12% moisture: `ρ = 1000 G₁₂ (1.12)` (Wood Handbook eq. 4-12). The handbook's own
  example, white ash at `G₁₂ = 0.605`, gives 678 kg/m³.
- **Specification maxima** overstate typical cloth and webbing: Giant Leap's measured 9/16"
  tubular nylon is 12% under MIL-W-5625K's limit.
- **openrocket-database** (Apache-2.0) was used only as a cross-check. Two problems turned up in it:
  - Its ripstop weights use 31 g/m² per oz/yd² (the factor is 33.906), so they are 8.6% low.
  - Its "Plywood, aircraft" at 337–361 kg/m³ is lite-ply, not birch.

## Verification

- **By hand:**
  - `mass::tests`: two boxes make one box; point masses give the products of inertia; rolling
    swaps and mixes axes as `I′_xy = (I_xx − I_yy) sin θ cos θ`; rotation keeps the principal
    moments; motor elements land on the axis.
  - `tests::composite_rocket_inertia_matches_hand_calculation` (crate root) combines a filled cone, a tube,
    four fins and an off-axis payload. Each part's moments come from its own formula and the six
    tensor terms are written out; the result agrees to 1e-11.
  - `parts::tests::a_nose_cone_with_a_capped_shoulder_adds_up_by_hand` checks the cone, tube and
    cap to 1e-11.
  - `fins::tests`:
    - A rectangular fin is a box, and four fins are the sum of rotated boxes.
    - A swept fin's `I_xz` matches quadrature of the planform.
    - A fin canted 90° is the box turned.
    - Tube fins match the parallel-axis theorem.
- **Closed forms against quadrature:**
  - Each cross-section's `M₀, M₁, M₂, T` matches exact quadrature of `t(x)` to 1e-13, on a wide
    chord and one narrower than `t`.
  - The airfoil constants match to 1e-14.
  - Trapezoidal, elliptical and freeform areas and centroids match to 1e-12.
- **Properties** (`mass::tests`, proptest): combining is associative and order-free, turning a
  body keeps its principal moments, and the inertia about any point exceeds that about the centre.
- **Materials:** ids are unique, sources present, and the unit conversions reproduce the sources
  (1.1 oz/yd² = 37.3 g/m², 225 ft/lb = 6.61 g/m, white ash 678 kg/m³).
- **Lessons:**
  - L44 `thin_tube_inertia_includes_radial_term`.
  - L45 `hollow_transition_and_freeform_fin_cg_are_exact_centroids`: a conical wall's exact
    centroid, and an M-shaped fin against the shoelace centroid.
  - L46 `fin_tab_and_rail_button_mass_counted`.
