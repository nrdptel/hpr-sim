# Wind

Code: `hpr_atmos::wind`. Turbulence is in `turbulence.md`.

Sources:

- **[8785C]** MIL-F-8785C, *Flying Qualities of Piloted Airplanes* (1980), §3.7.3.2, pinned as
  `mil-f-8785c`.
- **[TM]** NASA/TM-2008-215633, *Terrestrial Environment (Climatic) Criteria Guidelines for Use
  in Aerospace Vehicle Development* (2008), §2.2.5.2 and Table 2-21, `nasa-tm-2008-215633`.
- **[WMO]** WMO-No. 8 (2023), Vol. I, chapter 5 and its annex, `wmo-no8-vol1-2023`.

## Conventions

- **Velocity:** a wind model returns the velocity of the air in the launch frame's
  East-North-Up axes, m/s. The flight engine's airspeed is the rocket's velocity minus this.
  Mean wind is horizontal.
- **Direction:** meteorological, the direction the wind blows **from**, clockwise from true
  north, in radians: `v_E = −V sin θ`, `v_N = −V cos θ`. A wind from the west (`3π/2`) blows
  toward `+E`. RocketPy's *heading* is the direction the wind blows toward, `θ + π`.
- **Height:** models are queried with geometric height above mean sea level, like the
  atmosphere. The laws written in height above ground carry the ground's height above sea level.

## Models

- **`ConstantWind`:** one velocity at every height.
- **`PowerLawWind`:** `V = V_ref (z/z_ref)^α` for `z > 0` above ground, and zero at or below
  ground (flagged below).
  - [TM] eq. 2.1 gives it for peak winds below 150 m, with `z_ref = 18.3 m`.
  - [TM] Table 2-1: `α = 0.2` for 7–22 m/s and 0.14 above 22 m/s. Eq. 2.22 gives `1/7` with
    `z_ref = 10 m` for strong 10 m winds.
  - The exponents describe peak-wind profiles, and [TM] eq. 2.22 uses `1/7` for strong 10 m
    winds. Pick the exponent for the site; no single value fits mean winds everywhere.
  - Above the surface layer the law keeps growing, so pair it with measured winds aloft.
- **`LogLawWind`:** `V = V_ref ln(z/z₀)/ln(z_ref/z₀)` for `z > z₀`, and zero from the ground to
  `z₀`.
  - This is the neutral surface-layer law `V = (u*/κ) ln(z/z₀)` ([WMO] ch. 5 annex; [TM] eq.
    2.8 with `Ψ = 0`), written through a reference wind.
  - [8785C] §3.7.3.2 uses it with `z_ref = 20 ft`.
  - Roughness lengths: open flat terrain with grass 0.03 m (Davenport–Wieringa class 3,
    [WMO]); mown grass 0.001–0.01 m and low grass or steppe 0.01–0.04 m ([TM] Table 2-21).
- **`LayeredWind`:** speed and direction tabulated at heights, as from a sounding or forecast.
  - `SpeedDirection` (the default) interpolates speed linearly and turns the direction along
    the shorter arc. Exactly opposite directions veer clockwise. A turning wind keeps its speed.
  - A calm level (speed 0) takes the other level's direction, because reports give calm as
    "0 from 0°". Without that rule a wind growing out of calm would swing through a
    meaningless direction and create a crosswind neither level has.
  - `Components` interpolates East and North linearly, as RocketPy does. Between levels 90°
    apart the speed dips by up to 29%.
  - Beyond the end levels the end wind is held and the sample is flagged.
  - Put the surface wind (for example the 10 m observation) in as the lowest level, so the
    profile blends up from it.
- **`WindModel`:** any of these, tagged by `model` in JSON.

**[Loft lesson L6][lessons]:** design-file runs used one wind vector, and forecast profiles
stepped at the lowest level instead of blending from the surface.

## Not yet modelled

- Wind that changes with time, or across the field.
- Vertical mean wind.
- Terrain effects.
- A blend below the lowest tabulated level (the table holds the lowest level's wind).

The flight-engine milestone ([M1.6][roadmap]) and the planned weather milestone
([M5.2][roadmap]) decide how the flight engine composes a surface law with levels aloft.

## Tests that pin this

- **`wind::tests::layered_wind_interpolates_speed_and_heading`** ([Loft lesson L6][lessons]):
  - Halfway between 4 m/s from 350° and 12 m/s from 30°, the wind is 8 m/s from 10°, turning
    through north.
  - There is no step just above the surface level.
- **`components_interpolation_averages_the_vectors`**, **`opposite_directions_turn_clockwise`**
  and **`wind_grows_out_of_calm_without_turning`**.
- **`layered_wind_holds_and_flags_beyond_its_levels`**.
- **`meteorological_direction_convention`**, plus the power and log laws through their
  references, below ground, and at `z₀`.
- Invalid inputs, and the tagged JSON round trip.

[lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
