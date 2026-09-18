# The effective exhaust velocity guard: why 200 to 5,000 m/s

This note holds the reasoning and the measurements behind the units check in `hpr_motor`
(`EXHAUST_VELOCITY_RANGE_M_S` in `crates/hpr-motor/src/motor.rs`). The Solid motors page
(`docs/physics/motor.md`, "The effective exhaust velocity is a units check") gives the short
version. It was moved here from that page in M0.4e, unchanged in substance.

## What the check is

Both motor constructors (`SolidMotor::new` and `SolidMotor::from_envelope`) refuse a motor whose
curve and propellant mass imply an effective exhaust velocity `c = I/m_p` (total impulse over
propellant mass) outside **200 to 5,000 m/s**. Nothing else in the API notices a units slip: the
411I175's envelope in millimetres and grams read as metres and kilograms
(`from_envelope(curve, 38.0, 245.0, 228.9, 437.5)`) has positive, finite dimensions, a propellant
mass below the loaded mass, and an exhaust velocity of 1.8 m/s.

## Which impulse

`I` is the curve's own impulse, uncorrected for ambient pressure, because that is the impulse the
model actually delivers: `state()` derives the mass flow as `F(t)/c` from the same uncorrected
curve, so `∫ṁ dt = I/c = m_p` closes exactly. The bound is therefore on `c` at the curve's
reference pressure, which for a sea-level-tested motor is a few per cent below its vacuum value.

## Where the range comes from

Chemistry sets the scale. Rather than quote a specific-impulse table hpr has not read, the range
comes from the catalog itself. It was measured over the 1,708 ThrustCurve.org simulator files that
have both a parsed impulse and a catalog propellant mass. That is the same mass
`CatalogMotor::motor` uses, which prefers the catalog's metadata over the curve file's header.

A percentile says how much of the list lies below a value: `p5` is the value 5% of the files fall
below, `p50` the median (half below, half above), and so on.

| percentile | min | p1 | p5 | p50 | p95 | p99 | max |
|---|---|---|---|---|---|---|---|
| `c`, m/s | 236 | 597 | 928 | **1,867** | 2,210 | 2,463 | 3,031 |

So 90% of the files lie between 928 and 2,210 m/s. The bulk of that distribution is APCP
(ammonium perchlorate composite propellant); the low tail is the black-powder motors, which
cluster below about 900 m/s.

## What that means for the check

**Under that convention the bound rejects none of them.** That is the honest statement of what
this check is: a guard against a units slip, which moves `c` by a factor of 1,000, not a filter on
propellant.

It has real headroom, but not a great deal at the low end: the lowest real entry is 1.2x above the
floor, and the highest 1.65x below the ceiling. The low tail is an accounting artifact rather than
chemistry. Estes and Quest normally record a "propellant weight" that includes the delay grain and
the ejection charge, neither of which delivers thrust impulse, so a small black-powder motor reads
lower than its propellant really is. A 1/8A recorded that way could fall under 200 m/s. If one ever
does, the fallback is issue #11's second option: apply the bound only above a couple of grams of
propellant.

## Two numbers worth keeping straight

Both have been got wrong here before.

- Reading the **curve file header** mass instead gives a different distribution (max 10,111 m/s,
  from a J motor whose header claims 83 g where its catalog entry says 396 g). hpr does not use
  header masses when the catalog has them (Loft lesson L43, `catalog.rs`), so that file builds at
  2,111 m/s and passes.
- The 32 bundled motors run **689.78 m/s** (the Estes C5, a black-powder C) to **2,651.64 m/s**
  (the AeroTech K400C), computed from each curve's own impulse. The catalog's stored
  `total_impulse_ns` differs from the curve's by up to 0.3%, and would say 2,645.
  `motor::tests::every_bundled_motor_is_inside_the_range` measures both ends, so the figures can't
  drift.
