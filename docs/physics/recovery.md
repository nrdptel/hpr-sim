# Recovery

How hpr flies a rocket under a parachute: the drag area of a device, when it opens, how it fills,
and the equations of the descent. Streamers, tumble and separated bodies are M1.7b and are not
here yet. Code: `crates/hpr-sim/src/recovery.rs` and the descent branch of
`crates/hpr-sim/src/dynamics.rs`. Decisions: ADR-012.

Sources:

- T. W. Knacke, *Parachute Recovery Systems Design Manual*, NWC TP 6575 (1991), for canopy drag
  coefficients, filling times, drag-area growth and the equilibrium descent speed. Its title page
  limits distribution, so it is cited, never redistributed (`docs/VALIDATION.md`).
- RocketPy 1.13.0 (MIT), `rocketpy/simulation/flight.py:2710-2790` and
  `rocketpy/rocket/parachute.py`, for the point-mass descent that M1.7a is compared against.

## Drag area

A device's drag is set by its drag area `C_D S`, m²:

- **Given directly** (`DeviceDrag::DragArea`), which is RocketPy's `cd_s`.
- **From a canopy** (`DeviceDrag::Canopy`): `C_D S = C_D0 · π D₀²/4`, with `D₀` the nominal
  diameter and `C_D0` the drag coefficient on the **nominal** area `S₀ = π D₀²/4`, Knacke's
  convention (printed page 5-2; `D₀ = √(4 S₀/π)`, so `S₀` includes the vent and every opening).

`CanopyType` carries Knacke's printed data for thirteen canopy types: the `C_D0` range (Table 5-1
for solid textile canopies, Table 5-2 for slotted), the fill constant `n` (Table 5-6, unreefed),
the drag-area growth exponent of Pflanz's method (Figure 5-51) and the infinite-mass opening-force
coefficient `C_x`. Knacke prints a **range** of `C_D0` for every type and no single value; hpr's
default is the middle of the range (flat circular: 0.75 to 0.80, so 0.775). Where Knacke prints
"insufficient data" hpr has `None` and the user has to supply the number.

RocketPy's default parachute `C_D` of 1.4 is not a `C_D0` in this sense: it is a hemispherical
canopy's coefficient on the projected area, which it uses only to turn `cd_s` into a radius for its
added mass. Knacke's hemispherical range on `S₀` is 0.62 to 0.77.

## Triggers, lag and release

A device's charge fires at its `Trigger`:

- `Apogee`: when the apogee event fires (the centre of mass's height rate falling through zero).
- `Altitude { height_above_ground_m }`: the first time the centre of mass is **descending** and at
  or below that height above the launch site. This is an altimeter's main setting. A rocket whose
  apogee is already below the setting fires at apogee, because no crossing follows; that is
  RocketPy's numeric trigger (`y[5] < 0 and h < trigger`, `parachute.py:354-364`).
- `Time { time_s }`: a time after the first ignition.
- `MotorDelay { motor }`: that motor's ejection delay after its own burnout. The motor must have a
  delay in seconds; a plugged motor or one with no delay set is refused.

Charges are only checked in free flight and during the descent, so a `Time` or `MotorDelay`
trigger whose time passes while the rocket is still on the pad or the rail fires at rail exit.

`lag_s` seconds after the trigger the device **deploys** (line stretch) and starts to fill. The
first deployment of a flight switches it to the descent phase. A device can name another whose
deployment **releases** it (`released_by`), which is how a drogue is cut away when the main opens;
a released device contributes nothing from that instant.

Events, in the order a two-device flight records them: `Apogee`, `Trigger(drogue)`,
`Deployment(drogue)`, `Trigger(main)`, `Deployment(main)`, `Release(drogue)`, `GroundHit`.

Trigger times that are known before the flight (a time, or a motor delay) and every deployment and
end of filling are stop times, so no integration step straddles a change in the drag area.

## Inflation

The open devices' drag area at time `t` is the sum over the devices that have deployed and are not
released, each contributing

```text
(C_D S)(t) = (C_D S)₀ · min(1, (t − t_d)/t_f)^j
```

with `t_d` its deployment, `t_f` its filling time and `j` its growth exponent. `Inflation` chooses
`t_f`:

- `Instant`: `t_f = 0`, the full drag area at line stretch. This is RocketPy's model, and the
  upper bound on hpr's opening load.
- `FillingTime { time_s, exponent }`: a filling time fixed in advance.
- `FillConstant { constant, exponent }`: Knacke's `t_f = n D₀/v` (printed page 5-43), with `v` the
  airspeed at line stretch and `n` the canopy fill constant. A deployment at rest has no filling
  time in this law (`n D₀/v` diverges), so the canopy is taken as open at once and fills as the
  rocket picks up speed.

`j = 1` is linear growth (Knacke's ribbon and ringslot canopies) and `j = 2` the concave growth of
solid cloth (Pflanz, Figure 5-51). Knacke's measured drag area **overshoots** the steady value by
10 to 80% near the end of filling (Figure 5-40, printed page 5-47), and his infinite-mass opening
force is `C_x = 1.7` for a flat circular canopy. hpr models neither: its drag area rises to the
steady value and stays. The peak load hpr reports is therefore a lower bound on the real opening
shock, and instant inflation is hpr's own upper bound. Ludtke's law and Pflanz's `X1` reduction
factor are candidates for a later milestone.

## The descent

Once a device is open the rocket is a point mass. In the launch frame, with `m` the mass, `r_cg`
the centre of mass, `v_cg` its velocity, `w` the wind, `ρ` the density at its height and `g` normal
gravity:

```text
m a_cg = −½ ρ (C_D S)(t) |v_cg − w| (v_cg − w) + m (g + a_Coriolis) + T
```

- The drag acts at the centre of mass along the air's relative velocity, so it exerts no moment.
- The attitude and the body rates **freeze** at deployment (the body rates are set to zero), and
  the state's reference point, the nose tip, keeps its rigid offset from the centre of mass.
- The airframe's own drag is **left out**, as RocketPy leaves it out. A rocket's attitude under a
  canopy, and so the area it presents, is not modelled. For a drogue whose drag area is close to
  the airframe's broadside area this is a real omission; it is the same omission the oracle makes,
  and M1.7b's tumble model is where a body's own drag belongs.
- The thrust `T` is kept, along the frozen axis, so a device that opens while a motor still burns
  (an off-nominal case) is not silently thrust-free. Its direction is wrong the moment the rocket
  would have swung under the canopy.
- Gravity, the Coriolis force, the atmosphere and the wind are the same models the rest of the
  flight uses (`docs/physics/flight.md`).
- **Added mass is not modelled.** Knacke gives no closed-form apparent mass (printed page 5-40
  says only that it is the enclosed volume times density times a form factor), and RocketPy's
  `m_a = k_a ρ (2/3) π R² H` has no citation in its code. It carries no weight in RocketPy either,
  so it changes no equilibrium descent rate, only the transient right after an opening. It is not
  small: the fixture records 5.6 kg for Calisto's main against a 16.2 kg rocket and 15.9 kg for
  NDRT's against a 20.8 kg one (both at the start height; both grow with density as the rocket
  descends). The comparison below shows what leaving it out costs.

The equilibrium descent speed is Knacke's (printed page 5-128), and `recovery::terminal_speed_m_s`
computes it:

```text
v_e = √(2 m g / (ρ C_D S))
```

## Verification

`crates/hpr-sim/src/recovery.rs`'s tests, all analytic unless they name the oracle:

| What | Result |
|---|---|
| Knacke's `v_e` against Loft's case (1.1 kg, 1 m flat canopy, `C_D` 0.8, ρ 1.225) | 5.294 m/s, as Loft printed |
| A descent from rest against the closed-form fall under quadratic drag (2 km, uniform air, constant gravity) | 2.1e-8 of `v_t` over the whole descent; the landing time within 1e-5 s of the closed form's 204 s |
| Drift in a steady wind, entered drifting with the air | exactly the wind times the time of flight (1e-8); the fall itself within 1e-4 of the closed form |
| Knacke's filling law, `t_f = n D₀/v` and `(t/t_f)^j` | the recorded drag area to 1e-9 of `(C_D S)₀` |
| Inflation against instant opening (deployed at 60 m/s under a 1.5 m flat circular canopy) | peak load 1,615 N against 3,020 N instant, 0.53 of it |
| An oversized canopy (5 m) opening at 100 m/s, 10 km of descent at 2.95 m/s | lands in 3,392 s in 6,914 accepted steps (a mean step of 0.49 s, where Loft's explicit RK4 needed a 2e-4 s floor) |
| A whole flight: drogue at apogee with a lag, main at 300 m, drogue released | events in order; each stage settles within 2% of its own `v_e` |
| Two devices triggered at the same instant | both open in the same pass, and the descent settles at the `v_e` of the **sum** of their drag areas |
| A device released before its own charge fires | it deploys into a release that has already happened and adds nothing; the descent stays at the open device's `v_e` |
| The same recovered flight flown twice | bit-identical rows, events, final sample and step counts (Loft lesson L24: a run does not mutate the simulation) |

An independent anchor on the comparison: in every one of the five cases below, both hpr and
RocketPy land within 1% of Knacke's `v_e` for the device that is open at landing, computed from
hpr's own air and gravity at the site. The test asserts it for both.

### Against RocketPy

`validation/oracles/rocketpy/recovery.py` flies RocketPy's own parachute phase for five of its
example rockets and writes `validation/fixtures/recovery/rocketpy-descent.json`;
`descent_matches_rocketpy_examples` replays each case in hpr. Both start from the same declared
state after burnout, near apogee, with the first device opening at once (its lag is overridden to
zero, so no ballistic segment under either model's aerodynamics separates them), the same `C_D S`,
triggers, sampling rates and wind, and RocketPy's noise set to zero. The oracle runs at
`rtol = atol = 1e-9`. The remaining differences are the models': hpr has no added mass, hpr's
drogue is released by the main where RocketPy replaces the drag area, hpr uses the exact 1976
standard atmosphere where RocketPy interpolates a 100-point pressure table (0.05% in density), and
hpr uses WGS 84 normal gravity where RocketPy uses Somigliana's formula. The test checks the
environments agree first, then the descent.

Measured (hpr against RocketPy, 2026-09-17):

| case | descent time | impact descent rate | drift | worst drift component |
|---|---|---|---|---|
| Calisto (drogue 1.0 m², main 10 m² at 800 m, wind 5 E / 2 N) | +0.08% (257.27 s) | −0.03% (5.454 m/s) | +0.06% (1,385.8 m) | +0.06% |
| Valetudo (drogue 0.4537 m², no wind) | −0.02% (45.76 s) | +0.00% (17.627 m/s) | −0.89% (0.19 m, Coriolis only) | — |
| NDRT 2020 (drogue 0.438 m², main 16.05 m² at 167.6 m, sheared wind) | +0.71% (61.60 s) | +0.01% (4.604 m/s) | +0.27% (327.9 m) | +2.87% (north, −50.8 m) |
| Prometheus 2022 (drogue 0.467 m², main 5.78 m² at 457.2 m) | +0.08% (153.50 s) | −0.03% (7.323 m/s) | +0.07% (1,236.9 m) | +0.07% |
| Juno III (drogue 0.885 m²) | −0.02% (53.56 s) | −0.01% (22.431 m/s) | −0.03% (457.9 m) | −0.03% |

Every metric is inside the milestone's 3%. The descent rate under the drogue, where a case has a
main, agrees to 0.01%. The two largest gaps are both NDRT's, whose main has a drag area of 16 m²:
RocketPy's added mass for it is 15.9 kg against the rocket's 20.8 kg, so its response to the
opening is slower, which lengthens the descent (+0.71%) and, in a wind that shears with height,
moves the smaller drift component by 2.87%. Adding a cited apparent-mass model would close that
gap.
