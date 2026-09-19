# The Arcas Robin's supersonic body gap: the smaller sources (M1.8e5)

**What this covers.** The four causes that rank third to sixth in
[the Arcas Robin's supersonic body gap](body-supersonic-gap.md): the lip, the blunt tip, TN 3527's
Fig. 2 below Mach 3, and issue #81. **What it is for:** the sizes behind that note's ranking.
**How far to trust it:** the numbers are in
[`validation/fixtures/aero/arcas-robin-gap.json`](../../validation/fixtures/aero/arcas-robin-gap.json)
(`sources`), as in that note; the blunt tip's rests on an assumed scaling. Slopes are per radian,
on the body's cross-section.

## The lip

Behind the boattail the model flares out again in a short lip, from 1.308 to 1.470 in across, its
face square to the boattail (TN D-4014 Fig. 1(a), Detail A). The tunnel measures it; the method
can't take it, so M1.8e4 leaves it out. Slender-body theory gives it `2 ΔA/A_ref` = +0.178 at
every Mach number; nothing here measures its real share. It would make hpr read higher still.

## The blunt tip

The tunnel's nose has a 0.062-in tip radius, 0.055 of its base radius (TN D-4014 Fig. 1(a)); the
fitted ogive is sharp. In slender-body theory a nose's lift depends only on its base area, so a
blunt tip acts only through the pressures behind it. No source found measures a tip this small.

- **Measured, four times blunter.** Butler, Sears and Pallas (AFATL-TR-77-8, 1977, Table 3,
  printed pp. 10 to 13) tested a 4-caliber tangent ogive on a 9-caliber midsection and 1-caliber
  afterbody, sharp (nose N22) and truncated to a hemispherical tip of 0.25 of its base radius
  (N23, 4.15 in long against 4.80). The tip changed `C_Nα` by nothing the table resolves at Mach
  1.5 and 2 (0.048 and 0.053 per degree both; its last digit is about ±2%), −1.7% at Mach 3 (0.060
  to 0.059, one step in that digit) and −9.5% at Mach 4 (0.063 to 0.057).
- **A design manual's theory.** Mason and others (NSWC TR 81-156, 1981, p. 112): "blunting the
  nose up to a bluntness ratio of RN/Rref = .1 has a negligible effect" on `C_Nα`. Its figure is
  small-perturbation and second-order shock-expansion theory, the family of hpr's own method, so
  it isn't independent evidence.

Each row takes AFATL's larger change at its Mach numbers at or around the row's, scaled by
`(0.055/0.25)ⁿ` with `n` from 1 to 2 (an assumption), on hpr's slope at `α → 0`: nothing the
table resolves at Mach 1.5 and 1.8 (its last digit would allow up to about 0.01), a loss of 0.002
to 0.011 at 2.3 and 2.96, and 0.015 to 0.07 at 3.96 and 4.63.
Mach 4.63 is past AFATL's Mach 4, an extrapolation. It ranks fourth; at `n` = 2 it falls below
Fig. 2's bound, and the two act at different Mach numbers. It would lower hpr's slope: toward the
tunnel's `b`; against the `α³` reading past Mach 3, toward it on the short model and away on the
long.

## TN 3527's Fig. 2 below Mach 3

The method needs the normal-force slope of each piece's *tangent cone* (the cone tangent to the
profile there). TN 3527 plots it in Fig. 2 (p. 40) from Mach 3 to 10; below, hpr holds Mach 3's.
Sims (NASA SP-3007, 1964, Table 2, printed p. 20) tabulates the same theory from Mach 1.5; his
expressions are "identical to those found by Kopal" (p. 7), whose tables Fig. 2 plots. At Mach 3
his table agrees with hpr's reading of Fig. 2 within 0.003 from 5° to 12.5°, 0.01 at 2.5°.

Each element's lift carries its tangent cone's slope with a positive weight (TN 3527 eq. 19), so
replacing the held slopes by Sims's scales the nose and cylinder's share by a ratio between the
least and greatest of his over the held ones: over his angles to 12.5° (the nose's cones run to
10.76°) and Mach numbers at or around each row, −1.1% to +2.2% at Mach 1.5 and −0.6% to +0.9% at
2.96, or −0.031 to +0.056 in slope, nothing from Mach 3. At Mach 1.5 only the 12.5° column falls
below 1, so the likelier change raises hpr's slope. The boattail's share could carry some of the
nose's loading too, but on this body it doesn't: lengthening the cylinder to 1000 in moves that
share by under 0.002 (`the_boattail_share_carries_almost_nothing_from_the_nose`).

## Issue #81

Where the gradient behind a corner points away from its tangent cone's pressure (TN 3527's
`η < 0`), hpr reduces the element to the generalized method; #81 records where that departs.
`ShockExpansionBody::reduced_elements` counts such elements: none on the Arcas Robin's body, both
lengths with the boattail, at all six Mach numbers. So #81 changes nothing here.

**Not sized:** the boundary layer, which thickens like a slight flare (the reports give no
thickness); the secant ogive's 0.003-in rms miss of the nose, too little to matter.
