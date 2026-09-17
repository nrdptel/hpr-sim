# Streamer and tumble drag: the sources M1.7b can use

Written while shipping M1.7a, whose sources cover canopies only: Knacke's manual has **no streamer
data at all** (checked: the type tables 5-1 to 5-5, the measured-drag section 5.2.3, the
miscellaneous-decelerator section 5.8.4 and the contents; his "ribbon" is always a ribbon canopy).
This note records what M1.7b can cite instead. Equations marked **verified** were read here from
the pinned PDF; the rest are leads to reproduce before they are used.

## Streamers

**Verified.** OpenRocket Technical Documentation v13.05 (CC BY-SA, pinned as
`openrocket-techdoc-13.05`), Appendix C "Streamer drag coefficient estimation", printed page 117:

```text
C_Dm = 0.034 · (ρ_m + 25 g/m²)/(105 g/m²) · (l + 1 m)/l          (C.6)
```

on the reference area `A_ref = w · l` (one side of the strip), with `l` the length, `w` the width
and `ρ_m` the material's surface density. The same appendix appears in Niskanen's 2009 thesis
(printed page 115, pinned as `niskanen-2009-thesis`, CC BY-NC-ND: read, never copy).

Its own validation, printed page 117: an independent set of streamers gives 12 to 27% error in the
normalized drag coefficient, which the text puts at 10 to 15% in descent velocity. The fit came
from a 40×40×120 cm wind tunnel at 6, 9 and 12 m/s, `w` 0.01 to 0.09 m, `l` 0.2 to 1.0 m and `ρ_m`
10 to 80 g/m² (polyethylene, cellophane, crêpe paper). The appendix says the rocket hangs below the
streamer as a point mass and that the twirling ("labile") regime cannot be measured in a tunnel.

**Leads, not yet reproduced.**

- **Carruthers and Filippone, "Aerodynamic Drag of Streamers and Flags", *J. Aircraft* 42(4), 2005,
  pp. 976–982, DOI 10.2514/1.9754.** Peer-reviewed and independent:
  `C_D = 0.405 AR^(−0.494)` for a 0.075 m² planform and `C_D = 0.561 AR^(−0.480)` for 0.025 m², on
  the one-side planform area, for `AR = L/W` of 10 to 30 at 6 to 18.9 m/s. The AIAA copy is
  paywalled; an author post-print is free at
  `https://pure.manchester.ac.uk/ws/files/29884977/POST-PEER-REVIEW-NON-PUBLISHERS.PDF` with no
  stated licence, so cite it and never redistribute it. Not pinned.
- **Kidwell, "Streamer Duration Optimization", NAR R&D, NARAM-43 (2001)**, OpenRocket's own
  reference for the appendix, at `https://www.narhams.org/library/rnd/StreamerDuration.pdf` (no
  stated terms, not pinned). It gives no equation, but it gives free-drop data: 4 in × 40 in
  streamers with about 5 g of corner weight over 20.1 m, descending at 2.04 m/s (Micafilm) to
  2.80 m/s (crêpe paper).
- **A claim worth reproducing first.** A subagent integrated `dv/dt = g − ½ρ C_D A v²/m` against
  Kidwell's times and reported that the drag coefficient needed to match his measured descent is
  about 4.5 times C.6's (0.166 against 0.037 for crêpe paper at `AR = 10`), while Filippone's
  correlation lands within 25 to 40%. That would mean C.6 under-predicts descent drag by about a
  factor of two in velocity at the top of its own fit range, plausibly because the tunnel mounting
  suppressed flapping. **This is a claim, not a fact:** M1.7b should redo the integration from
  Kidwell's printed times before trusting either model, and report both.

**Recommendation.** Implement C.6 as the default (it is what the OpenRocket oracle uses, so M2.2
can compare like for like), keep Filippone's correlation as a second, selectable model, add
Kidwell's cases under `validation/`, and let whatever gap survives show in the report.

## Tumble

**Verified.** The same technical documentation, §3.5 "Tumbling bodies", printed pages 53–55 (this
section is *not* in the 2009 thesis; it was added in the 2013 edition):

```text
½ ρ v₀² (C_D,f A_f + C_D,bt A_bt) = m g                           (3.98)
C_D = (C_D,f A_f + C_D,bt A_bt) / A_ref                           (3.99)
C_D,bt = 0.56  on the body tube's profile (side) area
C_D,f  = 1.42  on the effective fin area: one fin's area times an efficiency factor
```

Table 3.4, printed page 55, the efficiency factors by fin count: 1 → 0.50, 2 → 1.00, 3 → 1.50,
4 → 1.41, 5 → 1.81, 6 → 1.73, 7 → 1.90, 8 → 1.85. The constants were fitted to 22 m drop tests of
five models and predict terminal velocity within 3 to 14%. The text says the fin half is the
unreliable one (changing the error function moved `C_D,f` but barely moved `C_D,bt`), and notes
that 0.56 is half the 1.12 of a circular cylinder in crossflow, as expected of a cylinder falling
at a random angle, and that 1.42 sits between a flat plate's 1.17 and an open hemispherical cup's
1.42. Its citation for those, Hoerner's *Fluid-Dynamic Drag* (1965), is copyrighted with no legal
free copy, so it can't be pinned.

**Free replacements for Hoerner's numbers (leads, hashes reported by a subagent, not verified
here).** Both are US Government works:

- **NASA TN D-540** (McKinney, 1960), crosswind drag of circular cylinders: Figure 5, printed page
  13, `C_D` on the projected side area at a subcritical Reynolds number of 88,000 — `l/d` 1 → 0.63,
  3 → 0.74, 10 → 0.82, 20 → 0.91, with a flat plate at 1.27 for `l/w = 10`. Its own measurements at
  `Re` 5e5 to 1.65e6 fall to 0.24–0.45.
  `https://ntrs.nasa.gov/api/citations/20040047039/downloads/20040047039.pdf`
- **NASA TR R-474** (Jorgensen, 1977): Figure 2, printed page 76, the 2-D cylinder crossflow
  coefficient 1.2 up to the critical `Re` of 2e5; Figure 4, printed page 77, the finite-length
  ratio `η` ≈ 0.70 at `l/d = 10`.
  `https://ntrs.nasa.gov/api/citations/19770026166/downloads/19770026166.pdf`
- **Avoid NACA TN 3038** (Delany and Sorensen, 1953) as a primary source: its own text says the
  circular-cylinder coefficient came out near 1.0 instead of 1.2 because of uncorrected end
  leakage.

Those two agree with each other (0.70 × 1.17 ≈ 0.82) and make OpenRocket's 0.56 plausible as an
attitude average, from sources hpr can pin.

**Reynolds number is the open limit.** A 41 mm booster at 6 m/s is at `Re_d` ≈ 1.6e4 and a 100 mm
one at 25 m/s at 1.7e5, both subcritical, where these constants were fitted. Above about 3e5 the
drag crisis roughly halves the coefficient, so a large body would really descend about 1.4 times
faster than the constants say. Nothing in the pinned set covers that; M1.7b must document the
limit rather than extrapolate.

## The transition between tumble and a stable fall

No citable source found, in the pinned PDFs or in the open literature. §3.5 only says it applies to
stages that "normally are not aerodynamically stable", and Appendix C declines to model the
analogous twirling streamer regime, saying it would need flight tests. So tumble should be an
explicit choice a user makes, not something hpr infers. If hpr ever infers it, the threshold is an
unsourced modelling choice and needs an ADR.
