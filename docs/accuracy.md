# Accuracy

This page gathers every check hpr-sim has passed so far, and every known gap, in words and numbers.
Start with the bottom line: **when both codes fly the same drag
([same-drag](glossary.md#same-drag-and-predicted-mode)), hpr's whole flights match RocketPy's in
height, speed and time, and in where they land without wind. In wind they agree for a rocket that
leaves the rail fast. For one that leaves it slowly they differ, in large part because hpr includes a
sideways force on the body that RocketPy leaves out.** With each code's own drag ([predicted](glossary.md#same-drag-and-predicted-mode)), hpr's
heights differ from RocketPy's by −7.280% to +10.302% ([report][report]), the larger gaps where
its drag differs most from the example's. No flight has been compared with a real one.

What has been checked so far:

- each model on its own, against exact answers, its published source and, in places,
  [RocketPy](glossary.md#rocketpy), an open-source flight simulator;
- the descent under a parachute, against RocketPy, for five rockets;
- whole flights from the pad to the ground, against RocketPy, for six rockets flown with one
  declared drag coefficient, one of them past Mach 1: heights, speeds and times agree, and so does
  the path, except for rockets that leave the rail slowly in a wind;
- the same flights with each code's own drag, reported against a target rather than gated, the one
  past Mach 1 included;
- the normal force and centre of pressure from Mach 0.6 to 4.63, against NASA's wind-tunnel tests
  of a sounding rocket, and against [RASAero II](glossary.md#rasaero-ii), another code
  ([fixture][nf-fixture]);
- drag from Mach 0.6 to 4.63 against the same wind-tunnel tests, the forebody only
  ([Aerodynamics](physics/aero.md#drag-against-the-arcas-robin-wind-tunnel));
- a boattail's own drag and the base pressure behind it, from Mach 0.3 to 3.24, against measured
  boattails in six NACA and NASA reports ([drag fixture][drag-fixture]);
- the roll that [canted](glossary.md#cant) fins give, from Mach 1.5 to 4.63, against NASA's
  Arcas Robin wind-tunnel tests, and the roll damping from Mach 1.5 to 3 against the Basic
  Finner's, a standard finned test body ([roll fixture][roll-fixture]);
- drag from Mach 0.1 to 2.0 against the curves labelled RASAero II in RocketPy's example rockets
  ([Aerodynamics](physics/aero.md#drag-against-rasaero-ii-through-mach-2)), and from Mach 0.5 to
  3.2 against a worked example in MIL-HDBK-762, the U.S. Army's handbook for designing unguided
  rockets
  ([Aerodynamics](physics/aero.md#drag-against-mil-hdbk-762s-sample-calculation)).

Every number here links to the page or file it comes from. [Checking a claim](checking-a-claim.md)
shows how to follow one back to its source and its test, and
[how the site keeps the two in step](checking-a-claim.md#rules-that-keep-the-trail-honest).

## How to read the numbers

- **Powers of ten.** Very small and very large numbers are written the way programs print them.
  The number after the `e` says how many places the decimal point moves, to the left when it is
  negative. So 1e-12 is a millionth of a millionth, a 1 in the twelfth decimal place, and 1e6 is a
  million. Both appear in the two [Frames](physics/frames.md) rows of the results table below.
- **Absolute differences** carry a unit: they say how far apart two values are, in that unit.
  [Geodesy](physics/geodesy.md)'s round trips return heights within 2e-8 m, twenty billionths of a
  metre.
- **Relative differences** are marked *relative*: the difference as a fraction of the value it is
  compared with. [Gravity](physics/gravity.md) within 2e-14 relative means within two parts in a
  hundred million million. A percentage is a relative difference counted in hundredths.
- **Signs.** A signed difference is hpr's value less the one it is compared with, over that one.
  A plus means hpr's value is the larger in size, a minus the smaller.

## Four kinds of evidence

A model can be checked in four ways, from the weakest to the strongest evidence that it matches
reality. They are set out in the project's [validation plan][plan].

| kind | what is compared | what agreement shows |
|---|---|---|
| **Analytic** | The code against exact answers: formulas solved by hand (closed forms), conservation laws, and round trips (converting a value and converting it back) | The code computes what its equations say |
| **Published source** | The code against a source's printed tables and worked examples | The code implements the source correctly |
| **Another code** | hpr against another simulator, such as RocketPy, flying the same inputs | The two codes agree on the physics; not that either matches reality |
| **Real flights** | hpr against measured flights | The model matches reality, within the flight's own uncertainty |

The first three check the code. Only the fourth checks the physics against the world, and no
flight has been compared yet: that is [M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone. The nearest
things so far are two recovery models checked against published drop tests
([Recovery](physics/recovery.md)), and the normal force checked against NASA's wind-tunnel tests
of the Arcas Robin sounding rocket
([Aerodynamics](physics/aero.md#normal-force-through-mach-1)): measurements, but not flights.

## Where each model stands

A tick means the model has been checked that way; a dash means it hasn't yet. *In the descents
only* means RocketPy's air density and wind were compared with hpr's at just the 23 heights its
parachute descents sample, as part of that comparison, and nowhere else
([Recovery](physics/recovery.md#against-rocketpy)).

| model | analytic | published source | another code | real flights |
|---|---|---|---|---|
| [Frames](physics/frames.md) | ✓ | — | ✓ RocketPy | — |
| [Geodesy](physics/geodesy.md) | ✓ | ✓ | — | — |
| [Gravity](physics/gravity.md) | ✓ | ✓ | ✓ RocketPy | — |
| [Atmosphere](physics/atmosphere.md) | ✓ | ✓ | ✓ RocketPy, in the descents only | — |
| [Wind](physics/wind.md) | ✓ | — | ✓ RocketPy, in the descents only | — |
| [Turbulence](physics/turbulence.md) | ✓ | — | — | — |
| [Design tree](physics/design.md) | ✓ | — | ✓ RocketPy, mass properties only | — |
| [Shapes](physics/shapes.md) | ✓ | — | — | — |
| [Mass properties](physics/mass.md) | ✓ | — | ✓ OpenRocket, the structure without motors | — |
| [Solid motors](physics/motor.md) | ✓ | — | ✓ RocketPy, ThrustCurve.org; OpenRocket for a curve file's own numbers | — |
| [Aerodynamics](physics/aero.md) | ✓ | ✓ Barrowman's examples; MIL-HDBK-762's drag example, fins left out: 6 of 12 within 10%, the body reading 6% to 10% low faster than sound | partial: drag and the normal force against RASAero II to Mach 2, the drag with the fins and finish guessed and 5% to 15% low faster than sound; and in whole flights, against a target | — (wind tunnel ✓: normal force, drag and boattails; the Arcas Robin's drag reads high at every speed) |
| [Rigid-body flight](physics/flight.md) | ✓ | — | ✓ RocketPy, with the drag given; and on each code's own drag, against a target; OpenRocket on 33 configurations of its examples ([results](format/ork.md#hprs-flights-against-openrockets)) | — |
| [Time integration](physics/integration.md) | ✓ | — | — | — |
| [Recovery](physics/recovery.md) | ✓ | ✓ | ✓ RocketPy | — (drop tests ✓) |
| [Staging](physics/staging.md) | ✓ ignition times, the mass step and momentum, against hand sums | — | ✓ OpenRocket: its two-stage, cluster and air-start examples, every flight within 5% (three cluster apogees against OpenRocket's flight with no parachute, because its parachute opened before apogee) ([results](format/ork.md#staged-clustered-and-air-start-flights)) | — |
| [Flight metrics](physics/metrics.md) | ✓ the boost's peak, the margins of the page's finless rocket with a boattail (including where a margin is withheld) and the landing placement against hand calculations; max q and top Mach against a 1 ms record; the least margins against 1 ms steps and a scan of every step; the flight margin and the optimum delay against hpr's own models and a flight with no recovery | — | — | — |
| [Interpolation](physics/interpolation.md) | ✓ | — | — | — |
| [Quadrature](physics/quadrature.md) | ✓ | — | — | — |

## Results by model

The headline results, one check to a row. Each model page opens with *In short*, and its
verification section lists every test and its [tolerance](glossary.md#tolerance), how far a result
may be from its reference and still pass.

| model | compared with | how close |
|---|---|---|
| [Frames](physics/frames.md) | RocketPy's starting attitude, worked out from the launch rail's angles, for 8 rail setups | within 1e-12 rad |
| [Frames](physics/frames.md) | an exactly solvable spin whose axis sweeps round a cone (coning), over 1e6 integration steps | attitude within 1e-9 rad |
| [Geodesy](physics/geodesy.md) | the ellipsoid values printed in Table 3.5 of the [WGS 84](glossary.md#wgs-84) standard | to their printed digits |
| [Geodesy](physics/geodesy.md) | round trips at random points from −10 km to +1000 km: latitude, longitude and height to Earth-centred x, y, z, and back | latitude within 1e-14 rad, height within 2e-8 m |
| [Gravity](physics/gravity.md) | the WGS 84 formulas, worked to 40 digits by a separate script, at 11 points from the equator to both poles and up to 200 km high | gravity's strength within 2e-14 relative |
| [Gravity](physics/gravity.md) | RocketPy's gravity formula, at 8 points | under 1e-12 relative |
| [Atmosphere](physics/atmosphere.md) | the tables of the 1976 [standard atmosphere](glossary.md#standard-atmosphere), at 32 heights from −2 to 86 km | every value within 0.1% |
| [Atmosphere](physics/atmosphere.md) | CIPM-2007 (Picard et al., 2008), a published reference formula for the density of humid air that treats air as a real gas, from 15 to 27 °C | humid-air density within 0.047% |
| [Atmosphere](physics/atmosphere.md) | RocketPy's air density, at the 23 heights its descents sample ([Recovery](physics/recovery.md#against-rocketpy)) | within 3.7e-4 relative |
| [Wind](physics/wind.md) | RocketPy's wind, at the same heights ([Recovery](physics/recovery.md#against-rocketpy)) | each component within 1e-9 m/s |
| [Wind](physics/wind.md) | the drift of RocketPy's four descents with wind ([Recovery](physics/recovery.md#against-rocketpy)) | the distance drifted within 0.28% |
| [Turbulence](physics/turbulence.md) | the [Dryden](glossary.md#turbulence-dryden) gust spectra, published formulas for how gust strength spreads over wavelength, over 2²⁰ random samples (about a million) | within 4 standard errors (the scatter expected by chance) in every octave band (a range of wavelengths spanning a factor of two): ±1–3% in the wide bands. Unvalidated for rockets |
| [Design tree](physics/design.md) | a rocket worked by hand, loaded, burning and burnt out | mass within 1e-12 kg, centre of mass within 1e-12 m, inertia within 1e-12 relative |
| [Design tree](physics/design.md) | RocketPy, for eight cases of its example rockets, at the times its equation solver computed the burning grains (up to 60 per case) | mass, centre of mass and inertia within 8.0e-10 relative (the centre as a fraction of the rocket's length) |
| [Design tree](physics/design.md) | the same, at 103 even times through the burn and after it, where RocketPy interpolates between its solver's times | mass within 1.1e-5 relative, inertia within 2.6e-5 relative |
| [Design tree](physics/design.md) | the propellant mass left in the grains, at both sets of times | within 2.4e-9 of the initial propellant mass at the solver's times, and 4.9e-5 between them |
| [Shapes](physics/shapes.md) | exact formulas (closed forms) for filled noses and transitions | within 1e-10 relative |
| [Shapes](physics/shapes.md) | separately computed high-precision integrals, for 22 noses and transitions | within 1e-12 relative |
| [Shapes](physics/shapes.md) | the same kind of integrals, for 20 hollow shells of a given wall thickness | within 1e-10 relative |
| [Mass properties](physics/mass.md) | a cone, a tube, four fins and an off-axis payload, added up by hand | within 1e-11 relative |
| [Mass properties](physics/mass.md) | fin cross-sections, against exact numerical integration | within 1e-13 relative |
| [Mass properties](physics/mass.md) | material densities, converted from the units their sources print | the sources' values, such as white ash at 678 kg/m³ |
| [Mass properties](physics/mass.md#checked-against-openrocket) | [OpenRocket](glossary.md#openrocket) 24.12's structure (every stage, no motor), on 71 compared designs in the current scratch-excluding survey | mass within 1% on 62 and centre of mass within 1% of length on 63, each file outside with a cause hpr warns of; pitch inertia within 1% on 54, the 17 outside with no named cause yet; roll inertia a median 1.686% apart, which OpenRocket's shortcut for fins accounts for, and on the cluster designs its stacking of their tubes on the axis ([clusters](physics/mass.md#clusters-and-fillets)): with the shortcut in hpr's place the median is 0.001%, and 19 files (13 distinct designs) remain outside 1% with a named cause. hpr's airfoil fins are 19.4% lighter than OpenRocket's, a departure kept on purpose ([fins](physics/mass.md#fins-rail-buttons-and-roll-inertia)). What a file leaves unsaid (a wall of no thickness, no material), which override wins, held to OpenRocket's on 32 probe designs, and each fin section and each kind of part alone on 43 more, packed parts among them ([packed parts](physics/mass.md#packed-parts)). On those that ask what a file leaves unsaid: mass within 0.001%, centre of mass within 0.001 mm, bar an elliptical fin's 0.18%, and an attached tube that writes no thickness (−2.4% on the committed probe; measured in [ADR-061][adr-061], the `.ork` conventions decision). On the override probes: two rules kept as measured departures, and flags that disagree (4.7 mm) ([probes](physics/mass.md#what-a-ork-leaves-unsaid-and-overrides)) |
| [Solid motors](physics/motor.md) | [ThrustCurve.org](glossary.md#thrustcurveorg)'s own statistics code (total impulse, burn time, average and peak thrust), on all 32 bundled curves | within 1.8e-15 relative |
| [Solid motors](physics/motor.md#validation) | [OpenRocket](glossary.md#openrocket) 24.12's own reading of the same 32 bundled curve files (total impulse, peak thrust, the 5%-of-peak burn-time window and the curve's duration) | every one bit for bit equal. Its average thrust divides the window's own impulse by the window where hpr divides the whole curve's, so hpr's is +0.0107% to +0.3147% higher (median +0.0965%). Nothing else in the motor model is compared with OpenRocket |
| [Solid motors](physics/motor.md) | RocketPy's solid-motor model, on three bundled motors, at 203 times each | total mass and inertias within 7.9e-5 relative; the propellant's own mass and inertias within 1e-4 of their values at ignition |
| [Aerodynamics](physics/aero.md) | [Barrowman's](glossary.md#barrowmans-method) five worked examples, at Mach 0 (low speed): each rocket's [normal-force slope](glossary.md#normal-force-slope) and [centre of pressure](glossary.md#centre-of-pressure-cp) | every centre of pressure within 1%. Every slope within 1% too, except the six-fin Recruiter's: +2.87% (+3.42% on its fins alone) |
| [Aerodynamics](physics/aero.md) | drag curves labelled [RASAero](glossary.md#rasaero-ii) in RocketPy's examples, at [Mach](glossary.md#mach-number) 0.3, with the fins and surface finish guessed because the curves don't record them | within 10% in four of seven cases; −18.3% for Cavour [power-on](glossary.md#power-on-and-power-off-drag) (motor burning), cause open |
| [Aerodynamics](physics/aero.md) | Valetudo's drag table, which is 1.44 times the drag in the [OpenRocket](glossary.md#openrocket) export for the same rocket | −47.0% power-off and −50.4% power-on. Against the OpenRocket export, hpr is 23.5% under as designed here, and 1.9% under with the export's own surface finish and launch lugs |
| [Aerodynamics](physics/aero.md#drag-against-rasaero-ii-through-mach-2) | the same curves every 0.05 from Mach 0.1 to 2.0, as far as each reaches; Calisto's, the one real RASAero II export, to Mach 2 | Calisto within 10% at 15 of 15 subsonic Mach numbers, 3 of 7 transonic and 8 of 17 supersonic, where hpr reads −14.9% to −5.1%, lowest at Mach 2 (−29.8% to −24.4% before the boattail's supersonic wave drag). Other plausible fins put 14 to 17 of the 17 within 10%, though none puts every row within it, so most of what is left is within the unrecorded inputs; part of it is hpr's body, which reads low faster than sound against MIL-HDBK-762 too |
| [Aerodynamics](physics/aero.md#drag-against-mil-hdbk-762s-sample-calculation) | MIL-HDBK-762's worked drag example, a rocket whose every term the handbook calculates, from Mach 0.5 to 3.2: a calculation with every input known, not a measurement. Its fins are sharp-edged wedges, which hpr can't represent, so their pressure drag is left out on both sides | 6 of 12 within 10%. From Mach 0.9 to 1.2, +12.3% to +31.9%, mostly the nose and the base; from Mach 1.6, −6.0% to −9.6%, friction and the base |
| [Rigid-body flight](physics/flight.md) | the exact motion of a tumbling, spinning rocket in a vacuum, over 22 s | the centre of mass within 1.7e-6 m of the exact parabola |
| [Aerodynamics](physics/aero.md) | NASA's wind-tunnel tests of the half-scale Arcas Robin and a longer version, Mach 0.6 to 4.63: [normal-force slope](glossary.md#normal-force-slope) and centre of pressure, 22 readings at 12 Mach numbers ([fixture][nf-fixture]) | from Mach 1.5, every slope within target: +8.8% to −3.3% (short) and +9.4% to −2.3% (long), since the committed designs fly the shock-expansion method to their base; the centre of pressure within 0.53 [calibres](glossary.md#calibre-caliber), which misses the half-calibre target on the long model at Mach 1.8 and 2.3, where that model's body alone reads 15% to 19% high (fins off, the short model's reads as much as 38% high at Mach 1.5: the crossflow excess sized by [M1.8e6](decisions-and-roadmap.md#m1-8e6)); from Mach 0.8 to 1.2, 2 of 9 within 15% and half a calibre |
| [Aerodynamics](physics/aero.md) | RASAero II's normal-force slope and centre of pressure for Calisto, Mach 0.1 to 2.0 ([fixture][nf-fixture]) | within 15% and half a calibre at 11 of 15 Mach numbers; hpr's slope rises with Mach through subsonic flow where RASAero II's stays flat (+21.9% at Mach 0.9), and from Mach 1.5, where its von Kármán nose flies the shock-expansion method behind a [Newtonian cap](physics/aero.md#blunt-tips), reads +8.3% to +12.9% |
| [Aerodynamics](physics/aero.md#checking-the-shock-expansion-method) | the body faster than sound, by a method no flight uses yet: the tables of NACA TN 3527, its source, for 144 cone- and ogive-cylinders at Mach 3 to 6.28, both its authors' own values and their wind-tunnel measurements of the normal-force slope and centre of pressure ([fixture][se-fixture]) | against the measurements, 117 of 120 slopes within ±0.2 per radian (−0.278 to +0.251) and 109 of 120 centres of pressure within 0.2 calibres (−0.540 to +0.328); against the authors' values, 102 of 144 slopes within 0.05 (−0.134 to +0.146) and 125 of 144 centres of pressure within 0.1 calibres (−0.670 to +0.257), so the targets are not met, where a separate implementation of the same equations, an uncommitted script, agrees with hpr outside the 12 rows at the method's limit ([#81](https://github.com/nrdptel/hpr-sim/issues/81)) |
| [Aerodynamics](physics/aero.md#checking-the-shock-expansion-method) | the same method on the Arcas Robin's nose and cylinder, against its measured body alone, Mach 1.5 to 4.63, 11 readings; no target, since the measurement includes the boattail and crossflow ([fixture][se-fixture]) | the short model within 5% from Mach 1.8 to 2.96, +16.4% at 1.5, −15.0% and −18.7% at 3.96 and 4.63; the long model −13.7% to −26.4% |
| [Aerodynamics](physics/aero.md#blunt-tips) | a blunt or vertical nose tip's Newtonian cap ahead of the same method: NASA TN D-4865's sphere-cone (a nose radius of 0.175 diameters on an 11.5° cone), its measured normal-force slope and centre of pressure at Mach 1.50 to 4.63, 6 readings; and the Arcas Robin's committed power-series nose on its cylinder and boattail, the lip left off, against its measured body alone, 11 readings; no target ([fixture][bt-fixture]) | the sphere-cone like for like (at its plotted angles, body lift included) −1.2% to +32.1%, high from Mach 2.96, where the report's own method reads −3.3% to +13.6% overall, and the centre of pressure within 0.06 diameters; the Arcas Robin like for like −4.8% to +37.2%, below the fitted secant ogive's +3.4% to +41.0% at every Mach number and nearer the tunnel at nine of its eleven rows. No measurement checks a tip that isn't spherical, and a nose that is nearly a cone but for a vanishing tip carries an unmeasured bias ([issue #101](https://github.com/nrdptel/hpr-sim/issues/101)) |
| [Aerodynamics](physics/aero.md#a-lip-in-a-boattails-wake) | the lip at the Arcas Robin's base, a reflexed flare behind its boattail, which hpr gives no normal force faster than sound: what the measured fins-off pitching moment says the lip's share is, 11 readings at Mach 1.5 to 4.63; no target ([fixture][lip-fixture]) | one share fitted to every row is +0.021 ± 0.019 per radian, so slender-body theory's 0.178 sits 8.4 standard errors above it; fitted to each model alone it is −0.016 ± 0.022 (short, whose six rows disagree among themselves, χ² per degree of freedom 6.4) and +0.108 ± 0.034 (long, whose five agree, 0.9). The number blames the lip for every miss in the centre of pressure, so it rejects slender-body theory's 0.178 but cannot separate zero from Seiff's embedded Newtonian bound (0.044 down to 0.014) |
| [Aerodynamics](physics/aero.md#what-a-marched-flare-is-worth) | NASA TN D-4865's model 2, the one flared body in the sources whose normal force and pitching moment are printed: a blunt 2.75° cone with an 18.5° flare, Mach 1.50 to 4.63, six readings, three of them with its boundary layer separated; no target ([fixture][flare-fixture]) | like for like (at its plotted angles, body lift included) the normal-force slope −1.9% at Mach 1.90, +7.0% at 2.30, +13.4% at 2.96 and +51.5% and +50.4% at 3.95 and 4.63, with the centre of pressure within 0.05 calibres through Mach 2.96 and 0.088 at 3.95. The report's shadowgraphs show that flare's boundary layer separated from Mach 2.96 up, though the cost only shows in the two fastest rows: the unflared model 1 reads +29.7% and +32.1% there against +12.5% at 2.96, so the flare itself adds 21.7 and 18.3 percentage points at 3.95 and 4.63 and between −1.9 and +0.9 below. Below about Mach 1.5289 there is no marched reading at all and a flared body falls back to slender-body theory, carried up over 0.3 Mach by the join: drawing the flare out to the steepest turn its shock holds lands past the steepest the march itself takes |
| [Aerodynamics](physics/aero.md#the-body-alone-against-the-15-target) | the Arcas Robin's body alone, fins off, against the 15% target the milestone set before the work began, 11 readings at Mach 1.5 to 4.63 — the same bodies as the row above, with the lip in place, which moves them under a point; the boattail cap does not reach these, whose boattail is 15° ([fixture][body-gap-fixture]) | not met on six rows: the short model +37.7% at Mach 1.5, +25.9% at 1.8, +16.9% at 2.3 and 2.96, and the long +19.4% at 1.8 and +15.5% at 2.3; at Mach 3.96 and 4.63 both are within 5%. Where the miss sits can only be told so far: the measurement's own fit trades its slope at `α → 0` against its curvature at a correlation of −0.96. At `α → 0` hpr is within 1.5 standard errors on every row outside, and on five of the six most of the gap is in the curvature body lift adds (1.3 to 2.8 times the measured, and ×12.20 at Mach 1.8 where the tunnel's curve barely bends); on the sixth, the short model at Mach 2.96, 77% of the gap is hpr's own `α → 0` slope |
| [Aerodynamics](physics/aero.md) | NASA's wind-tunnel tests of the same two models, Mach 0.6 to 4.63: drag on the forebody (the models' bases sat on a sting), fins on and off, 44 readings ([Aerodynamics](physics/aero.md#drag-against-the-arcas-robin-wind-tunnel)) | 2 of 44 within 10%, and every reading high. With the fins off, +13.5% to +24.1% from Mach 1.5 and +12.0% to +54.1% below, most of it the models' 15° boattail, which hpr over-predicts in a thick boundary layer. With the fins on, from Mach 1.5, +39.4% to +154.0%, where hpr's fins' drag stays near 0.30 and the measured falls to 0.046. hpr's base drag behind a plain cylinder is not measured by the tunnel and has been checked at no speed faster than Mach 0.3 |
| [Aerodynamics](physics/aero.md#roll-forcing-and-damping) | NASA's wind-tunnel tests of the two Arcas Robin models, Mach 1.5 to 4.63: [roll forcing](glossary.md#roll-damping-and-roll-forcing), the rolling moment per degree of cant, 11 readings ([roll fixture][roll-fixture]) | from Mach 2.3, all 8 within 5.3%; at Mach 1.5 and 1.8, +14.3% to +47.8% |
| [Aerodynamics](physics/aero.md#roll-forcing-and-damping) | The Basic Finner's roll damping measured in a wind tunnel, Mach 1.5 to 3.0, and Barrowman's computed value at Mach 0.07 ([roll fixture][roll-fixture]) | −5.9% to −16.2% against the wind tunnel, lower as the Mach number grows; −2.0% against Barrowman's computed value (a theory curve, not a measurement; it confirmed how hpr reads his method) |
| [Aerodynamics](physics/aero.md#boattails-faster-than-sound) | conical boattails measured in six NACA and NASA reports, jet off, Mach 0.3 to 3.24: the boattail's own pressure drag and the base pressure behind it ([drag fixture][drag-fixture]) | attached boattails of 3° to 10° from Mach 1.2: −21.9% to +28.3%, within 0.0123 (58 readings); from Mach 1.0 to 1.1, −18.2% to −5.4% (4), and −46.2% to +60.0% at points near Mach 1 their report calls questionable (27); through the rise from Mach 0.85 to 0.95, −77.5% to +7.6% (28); under Niskanen's rule to Mach 0.8, −100% to −83.5% (58). 16° in a boundary layer a fifth of the diameter thick, +26.4% to +54.2% (9). Separated 30° and 45°, −2.8% to +6.6% (3, which set the separation angles). The base drag behind them within 0.0102 of the measured on the cylinder's area, though behind small bases that is up to about 40% of the base's own drag (8 of the 12 set the ratio below Mach 2.5) |
| [Rigid-body flight](physics/flight.md) | RocketPy's whole flights from the pad to the ground, for six rockets, one past Mach 1, both codes flying one declared drag coefficient | heights, speeds, times and accelerations within 3% ([below](#whole-flights-against-rocketpy)), the largest +1.783% in the [report][report]; the path too, except the drifts of Juno III, Bella Lui and Prometheus 2022 in wind and NDRT 2020's apogee drift, reported, not scored, as measured differences between the models ([ADR-026][adr-026]) |
| [Rigid-body flight](format/ork.md#hprs-flights-against-openrockets) | [OpenRocket](glossary.md#openrocket) 24.12's calm flights of the 33 configurations of its examples that hpr flies, each figure by OpenRocket's own definition: apogee, largest speed, and the [stability margin](glossary.md#stability-margin) at rod clearance ([report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/openrocket-flights.md)); a code-to-code comparison, with no target set | margin within 0.016 calibres on all 33. With no named cause, apogee −4.34% to +1.03% (21 flights), and largest speed −0.69% to +0.92% (30 flights). The two-stage, cluster and air-start examples are within 5% in apogee and largest speed on every flight, the bar the [M1.9c](decisions-and-roadmap.md#m1-9c) milestone set before measuring; three cluster apogees are compared with OpenRocket's flight with no parachute, since its parachute opened before apogee. Two named causes move the apogee more than 5%: OpenRocket's parachute opening while the rocket still climbs (hpr flies no `.ork` parachute yet), up to +13.80%; and a part set to no drag, which hpr ignores ([#165](https://github.com/nrdptel/hpr-sim/issues/165)), apogee −15.47% to −19.13%, two of those three with an early parachute too. OpenRocket flying each of those six again without its causes brings five within 5% ([M2.2e4](decisions-and-roadmap.md#m2-2e4), sizing the causes). The sixth reads +7.80% like for like (the no-drag part removed from both programs), and the design's three flights are all left reading high, +1.15% to +7.80%, with a lead, not an explanation ([#177](https://github.com/nrdptel/hpr-sim/issues/177), a very blunt nose's drag) |
| [Rigid-body flight](format/ork.md#hprs-flights-of-the-private-designs) | [OpenRocket](glossary.md#openrocket) 24.12's calm flights of the 18 configurations of 5 private designs that hpr flies, published only as differences under anonymised ids ([report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/openrocket-library-flights.md)); a code-to-code comparison, with no target set | covers 5 of the 12 private designs. Apogee −4.84% to +1.17%, none more than 5% off; largest speed −0.65% to +2.28%; margin −0.0008 to +0.1108 calibres, hpr calling two designs more stable by 0.0350 to 0.0478 and 0.0564 to 0.0730 on every flight, cause not yet traced ([#172](https://github.com/nrdptel/hpr-sim/issues/172)), and a third, a two-stage design, by 0.1108, a centre-of-mass gap with a lead but no measured cause ([#186](https://github.com/nrdptel/hpr-sim/issues/186)) |
| [Time integration](physics/integration.md) | a separate line-by-line transcription of `DOPRI5`, the published Fortran integrator by Hairer and Wanner that hpr's [Dormand–Prince](glossary.md#dormandprince-and-rk4) stepper follows, on the problem Hairer's own example program for `DOPRI5` solves: the Arenstorf orbit, the closed, looping path of a small body pulled by two large ones that circle each other | the same step counts |
| [Time integration](physics/integration.md) | a vertical flight with drag that has an exact solution | apogee, deployment and landing times within 1.5e-8 s |
| [Recovery](physics/recovery.md) | RocketPy's descents under a parachute, for five rockets | every descent metric within 3% ([below](#the-descent-under-a-parachute-against-rocketpy)) |
| [Recovery](physics/recovery.md) | the same descents: the heights where the later parachutes fire, and the descent rate under the drogue | within 0.17% and 0.01% |
| [Recovery](physics/recovery.md) | published drop tests of five small models falling with nothing deployed ([tumbling](glossary.md#tumble-recovery)) | descent rate −10 to +19% off |
| [Recovery](physics/recovery.md) | Kidwell's [streamer](glossary.md#streamer) drop tests (2001) | descent rate +9% fast for his one flat streamer, and +58% fast for one folded into pleats, which hpr doesn't model |
| [Interpolation](physics/interpolation.md) | the exact formula of a smooth curve (a spline) through three points, `y = 3x/2 − x³/2` | matched |
| [Interpolation](physics/interpolation.md) | property tests, which check a rule on many randomly generated tables | every table passes exactly through its own points |
| [Quadrature](physics/quadrature.md) | polynomials, whose integrals are known exactly | exact up to degree 22 |
| [Quadrature](physics/quadrature.md) | six test integrals with known answers, one of them infinite at an end | within 1e-11 relative |

## The descent under a parachute, against RocketPy

This is one of the two comparisons the validation harness runs; the other is
[whole flights](#whole-flights-against-rocketpy). The harness is the program behind
`cargo xtask validate`, which flies every [validation case](glossary.md#validation-case) and
writes the committed report.

Five of RocketPy's [example rockets](glossary.md#example-rockets) are flown down in both codes,
set up the same way:

- the same starting state near apogee, with the first parachute opening at once;
- the same [drag areas](glossary.md#drag-area), deployment triggers and wind;
- the random noise RocketPy can add to each parachute switched off, so its runs repeat exactly;
- RocketPy's gravity formula, and its way of interpolating the wind (by its east and north
  components), in place of hpr's own defaults, to compare like with like.

The committed [validation report][report] gives the six numbers below for each descent. All of
them were scored and all are within tolerance; the largest difference is +2.865%.

Each metric must agree within 3% of RocketPy's value, with no absolute floor (a fixed allowance,
in metres or seconds, that would pass any smaller difference). Each case file argues why, for
example [NDRT's][ndrt-case]. The metrics:

- `descent_time_s`: the time from the shared start to landing.
- `impact_speed_m_s`: the vertical speed at landing; the wind adds to the speed over the ground.
- `mean_descent_rate_m_s`: the start's height over the descent time, so it repeats the descent
  time in another form.
- `drift_m`, `drift_east_m` and `drift_north_m`: how far the rocket lands from where the descent
  started, not from the pad, and that distance's east and north parts. A drift to the south or
  west is negative.

A difference is hpr's value less RocketPy's, over RocketPy's. So a positive one means hpr's value
is larger in size, in the same direction: NDRT drifts south, and hpr carries it further south.

Every result of the report, as hpr's difference from RocketPy:

| case | `descent_time_s` | `impact_speed_m_s` | `mean_descent_rate_m_s` | `drift_m` | `drift_east_m` | `drift_north_m` |
|---|---|---|---|---|---|---|
| [`descent-calisto-tests-motor-at-minus-1.373`][report] | +0.077% | −0.029% | −0.077% | +0.075% | +0.074% | +0.080% |
| [`descent-valetudo`][report] | −0.019% | +0.004% | +0.019% | −0.889% | −0.889% | −1.766% |
| [`descent-ndrt-2020-nose-to-tail`][report] | +0.705% | +0.012% | −0.700% | +0.276% | +0.214% | +2.865% |
| [`descent-prometheus-2022-generic-motor`][report] | +0.083% | −0.030% | −0.083% | +0.083% | +0.086% | +0.082% |
| [`descent-juno-iii`][report] | −0.018% | −0.008% | +0.018% | −0.018% | −0.018% | −0.018% |

Each case is named after the RocketPy example it flies. Three names carry more:

- `descent-calisto-tests-motor-at-minus-1.373` is Calisto as RocketPy's own tests build it, with
  the motor at −1.373 m in RocketPy's coordinates, where its getting-started notebook puts it at
  −1.255 m ([notes on RocketPy's example rockets][rocket-notes]).
- `descent-ndrt-2020-nose-to-tail` is the NDRT 2020 rocket, which RocketPy's example measures from
  the nose toward the tail ([notes on RocketPy's example rockets][rocket-notes]).
- `descent-prometheus-2022-generic-motor` is Prometheus 2022, whose motor RocketPy describes with
  its generic-motor model, which treats the propellant as a solid cylinder
  ([notes on RocketPy's example rockets][rocket-notes],
  [Recovery](physics/recovery.md#against-rocketpy)).

The Valetudo case flies RocketPy's own Valetudo example, not the flight on
[Getting started](getting-started.md). From the shared start, 800 m above the ground, it falls in
still air under the example's one drogue, with RocketPy's drag area of 0.4537 m², and lands at
17.627 m/s. Getting started flies the same airframe from the pad, with its own drogue, a main
parachute and a 5 m/s wind ([case file][valetudo-case],
[Recovery](physics/recovery.md#against-rocketpy)).

In still air, Valetudo's drift, 0.19 m, comes only from the Earth's rotation (the
[Coriolis acceleration](glossary.md#coriolis-acceleration)), and its north part is 19 µm. So a
small difference there is a large fraction ([Recovery](physics/recovery.md#against-rocketpy)).

The largest gap, NDRT's north drift, most likely comes from [added mass](glossary.md#added-mass):
RocketPy counts the air a canopy drags along, 15.9 kg for NDRT's main against the rocket's 20.8 kg,
and hpr has no such term, so the two respond differently as the canopy opens
([Recovery](physics/recovery.md#against-rocketpy)). That explanation fits the size of the gap, but
no test has isolated it yet.

What this shows: the two codes agree on the physics of a descent. It says nothing about whether
either matches a real parachute on a real day.

## Whole flights against RocketPy

Six of RocketPy's example rockets are flown from the pad to the ground in both codes: Calisto,
Valetudo, NDRT 2020, Juno III, Bella Lui and Prometheus 2022. They are set up the same way
([ADR-021][adr-021], the whole-flight comparison):

- one declared [drag coefficient](glossary.md#drag-coefficient), a constant 0.5
  ([case file][juno-case]), on the same reference area;
- the example's launch rail, site, parachutes and motor, with the thrust curve flown as measured
  and no correction for the thinner air at the site, as RocketPy's examples fly it;
- RocketPy's gravity formula, standard atmosphere and frictionless rail, and a declared wind;
- the random noise RocketPy can add to each parachute switched off.

This is [same-drag](glossary.md#same-drag-and-predicted-mode) mode. It checks the equations of
motion, the motor and the air, not the drag. hpr's own drag is compared
[below](#whole-flights-with-each-codes-own-drag).

**In short: how high, how fast and how long agree, and so does where the rocket goes, except for
rockets that leave the rail slowly in a wind.** The heights, speeds, times and accelerations of
six flights, one of them past Mach 1, and of three of them again in calm air, agree within the 3%
of each case's gate ([case file][juno-case]); the largest difference is +1.783%
([report][report]). Here *still air*
is an example flown with no wind (Valetudo's), and *calm air* a windy case flown again with its
wind switched off. The apogee and landing points agree too, within 2.2%, in every flight without
wind and for Calisto in wind ([ADR-026][adr-026]). Juno III and Bella Lui leave the rail slowly
in the wind, at a steep angle to the airflow. There hpr's [body lift](glossary.md#body-lift),
which RocketPy leaves out, its later release from the rail and, for Juno III, its simpler fin
model put their drifts 10.195% to 38.158% from RocketPy's. Prometheus 2022's differ by −7.292% and
+4.505%, from body lift and the rail release. NDRT 2020's apogee drift differs by −4.333%, mostly from the rail
release. These seven drifts are reported, not scored. So the landing offset
that [M2.1](decisions-and-roadmap.md#m2-1) asks for is met except where the two codes' models
differ.

Each of fifteen numbers per flight must agree within 3% of RocketPy's, with no absolute floor, or
say in its case file why it is not scored. Each case file argues why, for example
[Juno III's][juno-case]. Two more numbers compare the whole trace; they are explained after the
tables.

The numbers are measured as RocketPy defines them, with one exception. Each code's solver advances
the flight in [time steps](glossary.md#adaptive-time-step) and keeps the state at each step's end.
RocketPy takes each maximum (top speed, top Mach, top acceleration) only at those step ends. hpr
also searches between its own step ends for the true peak, so that its number does not depend on
where its solver happened to step. That search can only raise a maximum. Against hpr's old
step-end readings it raised them by at most 6.3e-5 of themselves (NDRT 2020's top speed). How much
RocketPy's own step ends miss was not measured. Either way it is far inside the 3% gate
([ADR-023][adr-023], the decision that also sets how peaks are found in both modes).

The definitions:

- `apogee_agl_m` and `apogee_time_s`: the highest point, and when.
- `flight_time_s`: the time from ignition to landing.
- `max_speed_m_s` and `max_mach`: the top speed over the ground, and the top
  [Mach number](glossary.md#mach-number).
- `rail_exit_speed_m_s` and `rail_exit_time_s`: when the forward rail button reaches the top of
  the rail, and the speed then. hpr's own rail-exit event waits for the last button, so the
  comparison finds RocketPy's instant instead.
- `burnout_altitude_agl_m` and `burnout_speed_m_s`: at the end of the thrust curve.
- `impact_speed_m_s`: the vertical speed at landing.
- `max_acceleration_power_on_m_s2`: the largest acceleration while the motor burns.
- `max_acceleration_m_s2` and `max_acceleration_time_s`: the largest over the whole flight, and
  when. For NDRT 2020 that is its main parachute opening, not a flight load
  ([case file][ndrt-flight-case]).
- `apogee_drift_m` and `landing_drift_m`: how far from the pad, along the ground, the apogee and
  the landing point are.

Speeds and accelerations are those of the rocket's [centre of dry mass](glossary.md#centre-of-dry-mass),
the point RocketPy's flight follows; hpr's own output follows the centre of mass of the loaded rocket.
Heights are measured from where that point starts, as RocketPy's are. A difference is hpr's value
less RocketPy's, over RocketPy's.

The [validation report][report] scores all but eleven of the numbers of the nine flights (the
six, and Juno III, Calisto and Bella Lui again in calm air), and all of the scored ones are within
tolerance. The eleven are measured and reported but not scored, each for a reason written in its
case file (below). Every result of the report, as hpr's difference from RocketPy:

| case | `apogee_agl_m` | `apogee_time_s` | `flight_time_s` | `max_speed_m_s` | `max_mach` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +0.052% | +0.103% | +0.123% | +0.015% | −0.120% |
| [`flight-valetudo`][report] | +0.112% | +0.238% | +0.233% | +0.035% | −0.026% |
| [`flight-ndrt-2020-nose-to-tail`][report] | +0.064% | +0.157% | +0.643% | +0.031% | +0.003% |
| [`flight-prometheus-2022-generic-motor`][report] | +1.208% | +0.652% | +0.830% | −0.006% | −0.235% |
| [`flight-juno-iii`][report] | +0.641% | +0.376% | +0.475% | +0.051% | −0.221% |
| [`flight-bella-lui`][report] | +0.342% | +0.201% | +0.229% | +0.016% | −0.071% |
| [`flight-juno-iii-calm`][report] | +0.085% | +0.073% | +0.066% | +0.015% | −0.123% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | +0.045% | +0.099% | +0.118% | +0.022% | −0.105% |
| [`flight-bella-lui-calm`][report] | +0.038% | +0.020% | −0.012% | +0.020% | −0.018% |

| case | `rail_exit_speed_m_s` | `rail_exit_time_s` | `burnout_altitude_agl_m` | `burnout_speed_m_s` | `impact_speed_m_s` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | −0.010% | −0.072% | +0.019% | +0.019% | −0.020% |
| [`flight-valetudo`][report] | −0.002% | −0.100% | +0.048% | +0.042% | +0.009% |
| [`flight-ndrt-2020-nose-to-tail`][report] | −0.009% | −0.085% | −0.004% | +0.043% | +0.022% |
| [`flight-prometheus-2022-generic-motor`][report] | −0.014% | −0.033% | +0.600% | −0.007% | −0.013% |
| [`flight-juno-iii`][report] | −0.005% | −0.142% | +0.269% | +0.056% | −0.003% |
| [`flight-bella-lui`][report] | −0.013% | −0.029% | +0.141% | +0.018% | +0.021% |
| [`flight-juno-iii-calm`][report] | −0.002% | −0.137% | +0.042% | +0.020% | +0.001% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | −0.001% | −0.074% | +0.023% | +0.025% | −0.003% |
| [`flight-bella-lui-calm`][report] | −0.000% | −0.032% | +0.012% | +0.024% | +0.021% |

| case | `max_acceleration_power_on_m_s2` | `max_acceleration_m_s2` | `max_acceleration_time_s` | `apogee_drift_m` | `landing_drift_m` |
|---|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +0.099% | +0.099% | −96.811% | −0.902% | +1.257% |
| [`flight-valetudo`][report] | +0.248% | +0.248% | +0.006% | −0.931% | −1.811% |
| [`flight-ndrt-2020-nose-to-tail`][report] | −0.020% | +83.059% | +0.197% | −4.333% | +1.627% |
| [`flight-prometheus-2022-generic-motor`][report] | +0.003% | +19.062% | +1.094% | −7.292% | +4.505% |
| [`flight-juno-iii`][report] | −0.197% | −0.197% | +0.001% | −38.158% | +36.678% |
| [`flight-bella-lui`][report] | +1.783% | +1.783% | +0.001% | −10.195% | −21.686% |
| [`flight-juno-iii-calm`][report] | −0.012% | −0.012% | +0.046% | −1.750% | −1.807% |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | +0.108% | +0.108% | −96.811% | −0.246% | −0.415% |
| [`flight-bella-lui-calm`][report] | +1.778% | +1.778% | +0.001% | −1.155% | −1.791% |

The last two numbers compare the whole trace, not one point of it. The series height RMS
(`series_height_rms_m`) is the root mean square of hpr's height less RocketPy's: square each
difference, average the squares, and take the square root. The series speed RMS
(`series_speed_rms_m_s`) is the same for speed. Both follow the centre of mass without propellant,
at RocketPy's 120 series times, from ignition until hpr lands. Both codes' clocks start at ignition
on the rail, so no time shift is fitted: a fitted shift would hide a real difference in the burn
or on the rail ([case file][juno-case]). The centre of mass without propellant is the point
RocketPy's series records. Every time counts the same, so the long descent weighs most; a
difference during the burn shows in the burnout and top-speed numbers instead.

Exact agreement would give 0, so these two are given in metres and metres per second, not as a
percentage. Each is held to 3% of RocketPy's apogee (for height) or top speed (for speed). That is
[M2.1](decisions-and-roadmap.md#m2-1)'s 3% for one number, applied to the whole trace
([case file][juno-case]).

All nine flights pass, each well inside its bound. The largest height RMS is Prometheus 2022's,
35.350931 m against its 110.3 m bound, about a third of it; its apogee is also the furthest off,
+1.208%. Body lift accounts for that too: RocketPy flown with hpr's body lift and rail release
reaches 3723.8 m, against hpr's 3723.6 ([case file][prometheus-case]). Juno III's is 14.550623 m
against 78.4 m, about a fifth, and the other seven are at an eighth of theirs or less. The speed
RMS runs from 0.022685 to 1.553509 m/s ([report][report]).

| case | `series_height_rms_m` | height bound, m | `series_speed_rms_m_s` | speed bound, m/s |
|---|---|---|---|---|
| [`flight-calisto-tests-motor-at-minus-1.373`][report] | +1.837754 | 78.3 | +0.058007 | 7.3 |
| [`flight-valetudo`][report] | +2.115504 | 23.3 | +0.171627 | 3.3 |
| [`flight-ndrt-2020-nose-to-tail`][report] | +2.425247 | 36.4 | +0.124657 | 5.4 |
| [`flight-prometheus-2022-generic-motor`][report] | +35.350931 | 110.3 | +1.553509 | 10 |
| [`flight-juno-iii`][report] | +14.550623 | 78.4 | +0.810392 | 6.7 |
| [`flight-bella-lui`][report] | +1.657682 | 15.9 | +0.273966 | 2.9 |
| [`flight-juno-iii-calm`][report] | +2.003892 | 78.6 | +0.065810 | 6.8 |
| [`flight-calisto-tests-motor-at-minus-1.373-calm`][report] | +1.821296 | 78.4 | +0.051504 | 7.3 |
| [`flight-bella-lui-calm`][report] | +0.089668 | 16.2 | +0.022685 | 2.9 |

What the two codes still do differently, and what it moves:

- **In wind: body lift, the rail release and Juno III's fins.** A rocket that leaves the rail
  slowly in a wind meets the airflow at a steep angle: Juno III leaves at 18 m/s in an 8.5 m/s
  wind, 26° off it ([ADR-026][adr-026]). Three things differ there.
  - hpr's normal force includes [body lift](glossary.md#body-lift), which grows with the square
    of that angle; RocketPy's does not. Much of it acts ahead of the rocket's centre of mass, the
    nose's above all, so it moves the centre of pressure forward and weakens the moment that
    [turns the rocket into the wind](glossary.md#weathercocking): hpr turns into it less.
  - hpr keeps the rocket guided until its last
    [rail button](glossary.md#rail-exit-and-rail-exit-velocity) leaves the rail; RocketPy frees it
    at the first.
  - Juno III's example gives its fins an airfoil lift curve, which RocketPy uses and hpr cannot
    model. RocketPy's fin slope is 7.6% steeper than hpr's flat-plate one ([ADR-026][adr-026]).

  Juno III's apogee is 245.3 m from the pad in hpr and 396.6 m in RocketPy (−38.158%). Adding
  hpr's choices to RocketPy one at a time moves RocketPy's to 360.7 m with hpr's rail release,
  286.5 m with its body lift too, and 248.3 m with its fin slope as well ([ADR-026][adr-026],
  measured by [`wind_response.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/rocketpy/wind_response.py)). Every windy drift lands within 1.3% of hpr's the
  same way. Bella Lui's drifts are −10.195% and −21.686%, Prometheus 2022's −7.292% and +4.505%
  (within 0.1% of hpr's once RocketPy has its body lift and rail release; [case
  file][prometheus-case]), and NDRT 2020's apogee drift −4.333%, mostly its rail release. These
  seven are reported but not scored, as measured differences between the models. Every other drift is scored and passes: Calisto's in wind, Valetudo's in
  still air, NDRT 2020's landing, and all six in calm air ([report][report],
  [case file][juno-case]).
- **RocketPy's own equations, corrected.** RocketPy 1.13.0 takes the turning moments during the
  burn about the wrong point: as far in front of the rocket's
  [centre of dry mass](glossary.md#centre-of-dry-mass) as the real centre of mass is behind it.
  That makes its rockets too stable while the motor burns, so they turn into the wind too far. The
  fix is proposed in [a pull request to RocketPy](https://github.com/RocketPy-Team/RocketPy/pull/1196), still open, built on
  [one that is merged](https://github.com/RocketPy-Team/RocketPy/pull/1188) but not yet released. RocketPy 1.13.0 as installed still has the
  error; the comparison applies both fixes ([ADR-026][adr-026]). Without them, Juno III's apogee drift was
  582.4 m, and hpr's drifts in wind were up to −60.8% short of RocketPy's at apogee and +151%
  beyond it at landing (the question of
  [issue #50][issue-50]).
- **On the rail,** hpr keeps the terms for the centre of mass moving inside the body as the
  propellant burns, and RocketPy's rail equation leaves them out. At a sharp ignition spike, with
  thrust and mass the same to five digits, hpr's acceleration is 1.2 to 1.3 m/s² higher. That is
  Bella Lui's +1.783%, whose peak is 7 ms after ignition ([report][report],
  [case file][bella-case]).
- **Calisto's two peaks.** Calisto's acceleration peaks twice, 0.9% apart: on the rail at 0.05 s
  and at 1.568 s. The same rail terms make hpr's first peak the higher, so `max_acceleration_time_s`
  moves from one peak to the other (−96.811%); it is reported but not scored. The peak's size is
  scored, but its +0.099% compares two instants; at 0.05 s hpr is 1.05% higher
  ([report][report], [case file][calisto-case]).
- **The main opening.** RocketPy adds the air a canopy drags along
  ([added mass](glossary.md#added-mass)), and hpr has none. So NDRT's peak deceleration as its
  main opens is +83.059% in hpr, and Prometheus 2022's +19.062%, reported but not scored. Their
  times are scored, since both codes put them where the main opens ([report][report],
  [case file][ndrt-flight-case]).

**Prometheus 2022 flies through Mach 1.** RocketPy's flight peaks at Mach 1.013 and hpr's at
1.010371 (−0.235%). Until [M1.8a](decisions-and-roadmap.md#m1-8a) hpr stopped any flight at Mach 1,
and the case was a known gap; with the normal force carried past Mach 1
([Aerodynamics](physics/aero.md#fins-through-mach-1)) it flies on its drag table to the ground.
Its scored numbers agree within 1.208% ([report][report], [case file][prometheus-case]). This flight
is a light test of the transonic normal force: no committed check measures its angle of attack
there, but a local probe found it below 0.11 degrees from Mach 0.8 to 1.2
([case file][prometheus-case]).

What this shows: with the drag given, the two codes agree on how high, how fast and how long a
rocket flies, and on where it goes, except for rockets that leave the rail slowly in a wind,
where their models differ. [M2.1](decisions-and-roadmap.md#m2-1)'s landing offset is met
everywhere else ([ADR-026][adr-026]). Which code is nearer the truth for those is for real flights
to say.
Nothing here says anything about hpr's own drag, which the next section compares, or about a real
flight. The
comparisons with OpenRocket ([M2.2](decisions-and-roadmap.md#m2-2), the OpenRocket comparison)
and with real flights ([M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone) come
after.

## Whole flights with each code's own drag

The same six flights again, in [predicted](glossary.md#same-drag-and-predicted-mode) mode: hpr
flies its own drag, from each design's shape, where same-drag mode gives it the declared one; both
modes use hpr's own [normal force](physics/aero.md), so only the drag differs. RocketPy flies the drag each of its examples ships, as RocketPy 1.13.0 flies the example:
a drag curve for Calisto, Valetudo and Juno III, a function of Mach for Prometheus 2022
([case file][prometheus-predicted-case]), a constant for NDRT 2020 (0.44,
[case file][ndrt-predicted-case]) and Bella Lui (0.43, [case file][bella-predicted-case]).
Everything else is set up as in the same-drag flights above ([ADR-023][adr-023], the
predicted-mode comparison).

**In short: hpr's heights are within 3% of RocketPy's for Calisto (−0.609%), Bella Lui
(+0.971%) and Juno III (+2.097%), well above for Valetudo (+10.113%) and NDRT 2020 (+10.302%),
where its drag is well below the example's, and below for Prometheus 2022 (−7.280%), where its
drag is above the example's through the coast** ([report][report]). These are
results, not a pass or fail. Neither code's drag is the truth: each example's drag came from
RASAero, OpenRocket or its team's own estimate. So each number is compared with the same 3% as the
same-drag flights ([M2.1](decisions-and-roadmap.md#m2-1), the validation milestone), but only as a
[target](glossary.md#gate-and-target). A miss is reported and explained in its case file
([Valetudo's][valetudo-predicted-case] and [NDRT 2020's][ndrt-predicted-case], for example), and
does not fail the test suite. The report is committed, so any number that moves shows up in
review.

Every predicted result of the report, as hpr's difference from RocketPy:

| case | `apogee_agl_m` | `apogee_time_s` | `flight_time_s` | `max_speed_m_s` | `max_mach` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | −0.609% | −0.567% | −0.288% | +0.363% | +0.232% |
| [`predicted-valetudo`][report] | +10.113% | +6.179% | +8.916% | +2.292% | +2.237% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +10.302% | +6.534% | +6.870% | +1.378% | +1.351% |
| [`predicted-prometheus-2022-generic-motor`][report] | −7.280% | −4.920% | −4.978% | +1.280% | +1.069% |
| [`predicted-juno-iii`][report] | +2.097% | +1.046% | +1.686% | +1.006% | +0.737% |
| [`predicted-bella-lui`][report] | +0.971% | +0.536% | +0.748% | +0.241% | +0.154% |

| case | `rail_exit_speed_m_s` | `rail_exit_time_s` | `burnout_altitude_agl_m` | `burnout_speed_m_s` | `impact_speed_m_s` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | −0.009% | −0.071% | +0.240% | +0.526% | −0.020% |
| [`predicted-valetudo`][report] | +0.029% | −0.109% | +1.326% | +2.883% | +0.009% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +0.000% | −0.087% | +0.723% | +1.532% | +0.022% |
| [`predicted-prometheus-2022-generic-motor`][report] | +0.001% | −0.036% | +1.227% | +1.778% | −0.013% |
| [`predicted-juno-iii`][report] | −0.001% | −0.143% | +0.772% | +1.129% | −0.003% |
| [`predicted-bella-lui`][report] | −0.011% | −0.029% | +0.270% | +0.296% | +0.021% |

| case | `max_acceleration_power_on_m_s2` | `max_acceleration_m_s2` | `max_acceleration_time_s` | `apogee_drift_m` | `landing_drift_m` |
|---|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | +0.123% | +0.123% | +0.002% | −2.233% | +0.901% |
| [`predicted-valetudo`][report] | +0.250% | +0.250% | −0.030% | +12.215% | +10.437% |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +0.656% | +83.059% | +9.646% | +12.259% | +12.648% |
| [`predicted-prometheus-2022-generic-motor`][report] | +0.848% | +19.062% | −6.903% | −17.642% | −6.167% |
| [`predicted-juno-iii`][report] | +0.105% | +0.105% | +0.002% | −36.078% | +40.482% |
| [`predicted-bella-lui`][report] | +1.797% | +1.797% | −0.029% | −9.390% | −20.805% |

The whole-trace numbers, in metres and metres per second, defined as for the same-drag flights
above. [Valetudo's][valetudo-predicted-case], [NDRT 2020's][ndrt-predicted-case] and
[Prometheus 2022's][prometheus-predicted-case] height RMS are outside the target, and so is NDRT
2020's speed RMS, for the same reason as their apogees: hpr's own drag differs from those
examples' drag. Each bound is 3% of that case's own RocketPy
apogee or top speed, so it differs from the same-drag bound: Juno III's 54.699661 m is inside its
83.9 m here ([report][report]).

| case | `series_height_rms_m` | height bound, m | `series_speed_rms_m_s` | speed bound, m/s |
|---|---|---|---|---|
| [`predicted-calisto-tests-motor-at-minus-1.373`][report] | +12.420364 | 84.6 | +0.331462 | 7.4 |
| [`predicted-valetudo`][report] | +75.375007 | 20.9 | +2.790876 | 3.2 |
| [`predicted-ndrt-2020-nose-to-tail`][report] | +116.136025 | 38.1 | +6.775468 | 5.5 |
| [`predicted-prometheus-2022-generic-motor`][report] | +263.920106 | 128.8 | +7.202224 | 10.3 |
| [`predicted-juno-iii`][report] | +54.699661 | 83.9 | +0.927234 | 6.8 |
| [`predicted-bella-lui`][report] | +5.264438 | 16.2 | +0.279185 | 2.9 |

Why the misses, largest first:

- **Valetudo and NDRT 2020 fly high: the drag.** hpr's drag coefficient at Mach 0.3 is −47.0%
  from Valetudo's table, a hand-edited table 1.44 times the drag of the OpenRocket export for the
  same rocket ([Aerodynamics](physics/aero.md#verification)). For NDRT 2020 it is 0.318 against
  the example's constant 0.44 ([case file][ndrt-predicted-case]). These drags are compared at Mach
  0.3 only. Flown on the same drag, the apogees agree with RocketPy's to +0.112% and +0.064%
  ([report][report]). Less drag also means a
  later apogee, a longer descent and further to drift, which moves their times and drifts too.
- **hpr's drag here is for the design as transcribed.** Where RocketPy's examples say nothing, the
  designs' fin thickness and edges and their surface finish are placeholders, so these results
  compare hpr's drag for those designs, not for the rockets as built. For Valetudo, the rocket's own
  OpenRocket finish and launch lugs take hpr's drag coefficient from 0.5566 to 0.714
  ([Aerodynamics](physics/aero.md#drag-verification)). So a miss here is not a gap for hpr
  to close toward the example's drag.
- **The drifts of Juno III and Bella Lui in wind:** hpr's body lift and rail release, and Juno
  III's fin slope, as in same-drag mode ([ADR-026][adr-026]).
- **Prometheus 2022 flies low: the drag again, the other way.** It passes Mach 1 on hpr's own drag
  since [M1.8b1](decisions-and-roadmap.md#m1-8b1), the drag through Mach 1, peaking at Mach 1.060
  against RocketPy's 1.048. Its coasting drag rises to about 0.49 at Mach 0.8, where the example's
  falls to 0.30, so hpr peaks −7.280% low and sooner, and its drifts and times follow. Flown on
  the same drag the apogees agree to +1.208% ([report][report],
  [case file][prometheus-predicted-case]).
- **NDRT 2020's and Prometheus 2022's peak deceleration** at their main openings, +83.059% and
  +19.062%, are the added-mass difference explained above. Their times move +9.646% and −6.903%
  with the apogee ([report][report], [case file][ndrt-predicted-case]).

What this shows: with its own drag, hpr's heights differ from RocketPy's by −7.280% to +10.302%
([report][report]), and the larger gaps are the two drags differing, not the flight. It does not
say which drag is right; only real flights can ([M2.3](decisions-and-roadmap.md#m2-3), the
real-flights milestone).

## Known gaps

These are the largest known differences and missing pieces. Each model page's *In short* lists the
rest.

- **hpr's own drag in a whole flight.** Its heights are +10.113% and +10.302% above RocketPy's for
  Valetudo and NDRT 2020, where its drag is well below the examples', and −7.280% below for
  Prometheus 2022, where it is above ([report][report]). Which drag is right is open until real
  flights ([M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone).
- **Drag faster than sound reads high** against NASA's wind tunnel, above all with fins: with the
  fins on, +39.4% at Mach 1.5 to +154.0% at 4.63. The fins take a blunt leading edge's formula,
  and nothing models a thin, sharp fin's own wave drag. Niskanen's cone, which ogives share,
  reads 45% to 105% above a measured cone through the rise near Mach 1
  ([Aerodynamics](physics/aero.md#drag-against-the-arcas-robin-wind-tunnel)).
- **A steep boattail's drag reads high** in a thick boundary layer: 16° boattails +26.4% to
  +54.2% from Mach 1.0 to 1.28, and the Arcas Robin's 15° boattail puts its forebody, fins off,
  +13.5% to +24.1% high from Mach 1.5 and +20.3% to +50.8% from Mach 1.0 to 1.2. No cited
  correction exists in the sources used (issue [#72](https://github.com/nrdptel/hpr-sim/issues/72);
  [Aerodynamics](physics/aero.md#boattails-faster-than-sound)).
- **Drag past Mach 1.6 reads low against RASAero II's Calisto**, to −14.9% at Mach 2, with the fins
  and finish the Mach 0.3 check declares; other plausible fins bring most rows within 10%. Part of it is hpr's body,
  which reads 6% to 10% low faster than sound against MIL-HDBK-762's worked example as well
  ([Aerodynamics](physics/aero.md#drag-against-rasaero-ii-through-mach-2)).
- **Roll: the forcing high near Mach 1.5, the damping low, nothing measured below Mach 1.5.**
  The roll forcing from canted fins reads +47.8% at Mach 1.5
  and +14.3% to +17.8% at 1.8 against NASA's wind tunnel, where linear theory's load climbs toward
  Mach 1 faster than the fins' does; the roll damping reads 5.9% to 16.2% low against the Basic
  Finner's. Nothing measured checks either below Mach 1.5
  ([Aerodynamics](physics/aero.md#roll-forcing-and-damping)).
- **A flared rocket's supersonic normal force rests on one measured flare.** A conical flare flies
  the shock-expansion method since [M1.8e17](decisions-and-roadmap.md#m1-8e17), which moves the
  [centre of pressure](glossary.md#centre-of-pressure-cp) of a test rocket with a 10° flare
  forward by 0.089 to 0.238 [calibres](glossary.md#calibre-caliber) against the model it had
  before, growing with Mach number. The one measurement beside it is NASA TN D-4865's model 2, an
  18.5° flare on a 2.75° cone, and it is not that rocket
  ([Aerodynamics](physics/aero.md#what-a-marched-flare-is-worth)). No other flare angle has been
  checked, and the reading a flare steeper than its corner's limit gets has been checked against
  nothing at all. A flare shallow enough to *reduce* its own element — a few thousandths of a
  degree on that rocket — is read by the older generalized method instead, which no measurement
  checks either, and which leaves a step where the corner's pressure crosses its tangent cone's
  ([Aerodynamics](physics/aero.md#a-near-flat-flare)). On a body with a short shoulder that
  crossing is not near-flat at all — 0.7° to 4.6° — and the step reaches +4.3% of the body's
  normal force and 0.19 calibres; its exact size for any conical flare is on that page.
- **The normal force near and far past Mach 1.** Against NASA's wind tunnel, between Mach 0.8
  and 1.2 hpr's slope runs up to +29.3% high and its centre of pressure up to 2.29
  [calibres](glossary.md#calibre-caliber) off. From Mach 1.5 up its slope holds to within 9.4%
  ([fixture][nf-fixture], [Aerodynamics](physics/aero.md#normal-force-through-mach-1)) and its
  centre of pressure to 0.53 calibres, which misses the half-calibre target on the long model at
  Mach 1.8 and 2.3, where the body alone reads 15% to 19% above the tunnel
  ([Aerodynamics: body lift](physics/aero.md#body-lift)).
- **Drag against the RASAero curves** at Mach 0.3 is within 10% in four of seven cases, with the
  fins and surface finish guessed, because the curves don't record them. Cavour power-on is
  −18.3%, cause open. Valetudo's −47.0% and −50.4% are against a table 1.44 times the drag in the
  OpenRocket export for the same rocket ([Aerodynamics](physics/aero.md#verification)).
- **Six fins.** The [normal-force slope](glossary.md#normal-force-slope) of Barrowman's six-fin
  Recruiter is +2.87% above his printed value, and +3.42% on the fins alone, mostly because hpr
  uses a different six-fin rule ([Aerodynamics](physics/aero.md#verification)).
- **Tumbling** is −10 to +19% off its source's own drop tests, and is used far outside the fit
  behind it. That fit comes from small models falling at 5.0 to 6.6 m/s; if Valetudo came down
  tumbling, with nothing deployed, hpr would bring it down at 37 m/s. The default streamer model
  reads +58% fast on a pleated streamer ([Recovery](physics/recovery.md)).
- **Opening loads,** the force on the rocket as a canopy opens, are no safe bound either way. With
  a [filling time](glossary.md#inflation-and-filling-time), hpr leaves out the brief rise of drag
  above its steady value while the canopy fills. Opening at once, it ignores how a light rocket
  slows while the canopy fills. The deployment speed can itself read high: a
  [separated](glossary.md#separation) body falls with no drag until its device opens
  ([Recovery](physics/recovery.md#inflation)).
- **No added mass under a canopy,** the likely cause of the 2.86% drift difference above
  ([Recovery](physics/recovery.md#against-rocketpy)), and the cause of NDRT's +83.059% and
  Prometheus 2022's +19.062% peaks as their mains open in the whole flight ([report][report],
  [case file][ndrt-flight-case]).
- **Body lift in wind.** A slow rocket leaves the rail at a steep angle to a crosswind, and there
  hpr's body lift, which RocketPy leaves out, is the largest reason its drift differs: Juno III's
  apogee drift is −38.158% against RocketPy's ([report][report]). How much body lift a rocket body
  makes is itself uncertain. hpr takes it from Jorgensen's crossflow term
  ([Aerodynamics](physics/aero.md#bodies-of-revolution)), whose factor is about 0.9 for these
  rockets at low speed. Flown in RocketPy with hpr's rail release and fins, that puts Juno III's
  apogee 248.3 m from the pad, against hpr's own 245.3 m ([case file][juno-case]). In the same
  runs, Galejs's constant `K`, which hpr used before, gives 240.2 m at 1.0, 231.1 m at 1.1 and
  194.1 m at 1.5, across its source's range, and the drift would be 328.0 m with no body lift at
  all ([ADR-026][adr-026]). Only real flights can say which is right
  ([M2.3](decisions-and-roadmap.md#m2-3)).
- **Airfoil fins.** hpr's fins use the flat-plate lift slope. It cannot model an airfoil lift
  curve such as the one Juno III's example gives its fins, which makes RocketPy's fin slope 7.6%
  steeper ([ADR-026][adr-026]).
- **Turbulence** is an aircraft model, unvalidated for rockets, and no flight uses it yet
  ([Turbulence](physics/turbulence.md)).
- **Wall and fin mass** may follow different conventions from OpenRocket's, which its documentation
  doesn't state; six are measured so far ([Mass properties](physics/mass.md#checked-against-openrocket)). Measuring a nose cone's wall thickness straight out from the axis, rather than
  square to its surface, changes the wall's volume by 1.4% on a cone three
  [calibres](glossary.md#calibre-caliber) long ([Shapes](physics/shapes.md)).

[ndrt-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-ndrt-2020-nose-to-tail.toml
[plan]: VALIDATION.md#principles
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[rocket-notes]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/rocketpy-rocket-mass.md
[valetudo-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-valetudo.toml
[adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
[adr-023]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-023-predicted-mode-each-codes-own-drag-reported-against-a-target-2026-09-18
[bella-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-bella-lui.toml
[calisto-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-calisto-tests-motor-at-minus-1.373.toml
[issue-50]: https://github.com/nrdptel/hpr-sim/issues/50
[bella-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-bella-lui.toml
[ndrt-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-ndrt-2020-nose-to-tail.toml
[prometheus-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-prometheus-2022-generic-motor.toml
[valetudo-predicted-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/predicted-valetudo.toml
[juno-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-juno-iii.toml
[adr-026]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18
[adr-061]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-061-what-a-ork-leaves-unsaid-read-as-openrocket-reads-it-overrides-measured-two-departures-kept-2026-09-21
[ndrt-flight-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-ndrt-2020-nose-to-tail.toml
[prometheus-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-prometheus-2022-generic-motor.toml
[nf-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/normal-force-vs-mach.json
[se-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/shock-expansion.json
[bt-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/blunt-tips.json
[lip-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-lip.json
[flare-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/marched-flare.json
[body-gap-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/arcas-robin-body-gap.json
[drag-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/drag-vs-mach.json
[roll-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/aero/roll-vs-mach.json
