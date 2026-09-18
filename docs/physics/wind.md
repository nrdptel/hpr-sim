# Wind

## In short

- **What it models:** the steady wind's speed and direction at each height: constant, growing
  with height from one measured wind (a power or log law), or from a table of levels, such as a
  forecast.
- **Sources:** the US military flying-qualities specification MIL-F-8785C (1980), NASA's climatic
  criteria for aerospace vehicles NASA/TM-2008-215633 (2008), and the World Meteorological
  Organization's observing guide WMO-No. 8 (2023).
- **How well it is validated:** unit tests pin each model: with speed and direction interpolated
  separately (the default), halfway between 4 m/s from 350° and 12 m/s from 30° a table gives 8 m/s
  from 10°. In [RocketPy](../glossary.md#rocketpy)'s parachute descents, hpr's wind, interpolated
  by components as RocketPy does, matches RocketPy's samples to 1e-9 m/s (in
  [scientific notation](../glossary.md#scientific-notation): a thousand-millionth of a metre per
  second). That includes the wind of NDRT 2020, one of the
  [example rockets](../glossary.md#example-rockets), which changes with height. In the four
  descents with wind, the total [drift](../glossary.md#drift) agrees within 0.28%, and each east or
  north part within 2.9% ([Recovery](recovery.md#against-rocketpy)). No real flights yet.
- **What it leaves out:** wind varying in time or across the field, vertical wind, terrain, and
  gusts (no flight uses [Turbulence](turbulence.md)). A flight takes one wind model, so a power or
  log law, which keeps growing with height, can't hand over to winds aloft.

## Code and sources

Code: [`hpr_atmos::wind`](../api/hpr_atmos/wind/index.html) in the API reference. Random gusts on
top of the steady wind are in [Turbulence](turbulence.md). The example program under
[An example of each](#an-example-of-each) builds every model.

Sources. Each is pinned under the id given, in the [reference lock file][lock], which records where
to download it and its SHA-256 hash ([Checking a claim](../checking-a-claim.md#the-trail) explains
how to fetch and check it):

- **[8785C]** MIL-F-8785C, *Flying Qualities of Piloted Airplanes* (1980), §3.7.3.2, pinned as
  `mil-f-8785c`.
- **[TM]** NASA/TM-2008-215633, *Terrestrial Environment (Climatic) Criteria Guidelines for Use
  in Aerospace Vehicle Development* (2008), §2.2.5.2 and Table 2-21, `nasa-tm-2008-215633`.
- **[WMO]** WMO-No. 8 (2023), Vol. I, chapter 5 and its annex, `wmo-no8-vol1-2023`.

## Conventions

- **Velocity:** a wind model returns the velocity of the air, in m/s, along the east, north and up
  axes of the [launch frame](../glossary.md#launch-frame-enu). The flight engine's airspeed is the
  rocket's velocity minus this. Mean wind (the steady wind, without gusts) is horizontal, so its up
  part is zero.
- **Direction:** [meteorological](../glossary.md#wind-direction): the direction the wind blows
  **from**, clockwise from true north, in radians. For a wind of speed `V` from direction `θ`, the
  east and north parts are `v_E = −V sin θ` and `v_N = −V cos θ`. A wind from the west (`3π/2`,
  270°) blows toward the east, `+E`. RocketPy's *heading* is the direction the wind blows toward,
  `θ + π`.
- **Height:** a flight asks every model for the wind at a height above mean sea level, as it asks
  the [atmosphere](atmosphere.md#height-datum). Some models are built from heights above the
  ground instead: [Which height?](#which-height) says which.
- **Flags:** each answer a wind model gives carries a flag, which is set when the height is
  outside what the model covers. That is below a table's lowest level or above its highest, or
  below the ground for a power or log law.
  - The model still answers: a table holds its end level's wind, and a law gives calm below the
    ground. The flag says that the answer was filled in, not taken from the model's data.
  - In code it is the answer's `extrapolated` field: `None` normally, and `Some(Side::Below)` or
    `Some(Side::Above)` when flagged.
  - The power and log laws are not flagged high above the ground, although they describe only
    the air near it.
  - **A flight doesn't read the flag yet** ([issue #45](https://github.com/nrdptel/hpr-sim/issues/45)).
    A rocket that climbs above a table's top level flies on the held wind, and nothing in its
    result says so. Ask the model yourself before flying a table, as
    [the example](#an-example-of-each) does: it marks each flagged answer with a star.

## Models

In the laws below, `V` is the wind speed at height `z` above the ground, and `V_ref` the speed
measured at a reference height `z_ref` above the ground, such as the 10 m mast of a weather
station. Each law blows from one direction at every height.

- **`ConstantWind`:** one velocity at every height.
- **`PowerLawWind`:** the speed grows as a power of height, `V = V_ref (z/z_ref)^α` for `z > 0`,
  and is zero at and below the ground. A height below the ground is [flagged](#conventions).
  - `α` is the exponent: the larger it is, the faster the wind grows with height.
  - [TM] eq. 2.1 gives the law for peak winds below 150 m, with `z_ref = 18.3 m`. A peak wind is
    the strongest speed over a period, gusts included, not the steady mean wind hpr flies.
  - [TM] Table 2-1: `α = 0.2` for 7–22 m/s and 0.14 above 22 m/s. Eq. 2.22 gives `1/7` with
    `z_ref = 10 m` for strong 10 m winds.
  - Those exponents describe profiles of peak winds, and of strong 10 m winds. No single value
    fits mean winds everywhere, so treat any exponent as a starting point, not a measurement of
    your field.
  - [The example](#an-example-of-each) uses `α = 1/7` (about 0.14) with a 10 m reference height,
    the pairing of [TM] eq. 2.22. Its *power law* column shows what that does to a 5.0 m/s wind at
    10 m: 4.0 m/s at 2 m, 6.9 m/s at 100 m, and 10.7 m/s at 2,000 m, where it is still growing.
  - The law describes the surface layer: the air nearest the ground, where friction with the
    ground sets how fast the wind grows with height ([TM] fits it below 150 m). Above that layer the
    law keeps growing, so pair it with measured winds aloft.
- **`LogLawWind`:** the speed grows with the logarithm of height,
  `V = V_ref ln(z/z₀)/ln(z_ref/z₀)` for `z > z₀`, and is zero from the ground up to `z₀`.
  - `z₀` is the roughness length: the height at which the law's wind falls to zero. It is set by
    how rough the ground is (see the values below).
  - This is the neutral surface-layer law `V = (u*/κ) ln(z/z₀)` ([WMO] ch. 5 annex; [TM] eq.
    2.8 with `Ψ = 0`), written through a reference wind. There `u*` is the friction velocity, a
    speed that measures how hard the wind drags on the ground, and `κ` is the von Kármán
    constant, a fixed number of the theory. Writing the law through the wind measured at `z_ref`
    cancels both, so hpr needs neither.
  - `Ψ` in [TM] eq. 2.8 corrects for the air's stability: air warmed from below mixes more, and
    air cooled from below mixes less, which changes how the wind grows with height. `Ψ = 0` is
    neutral air, where neither happens. hpr has only the neutral law.
  - [8785C] §3.7.3.2 uses it with `z_ref = 20 ft`.
  - Roughness lengths: 0.03 m for open flat terrain with grass ([WMO]; this is class 3 of the
    Davenport–Wieringa classification there, which gives a roughness length for each kind of
    terrain); 0.001–0.01 m for mown grass and 0.01–0.04 m for low grass or steppe ([TM] Table
    2-21).
- **`LayeredWind`:** speed and direction tabulated at heights, as from a
  [sounding](../glossary.md#sounding) or a forecast. Between levels it interpolates in one of two
  ways, set by `WindInterpolation`.
  - `SpeedDirection` (the default) interpolates speed linearly and turns the direction along
    the shorter arc. Between exactly opposite directions it turns clockwise with height. A
    turning wind keeps its speed.
  - With `SpeedDirection`, a calm level (speed 0) takes the other level's direction, because
    reports give calm as "0 from 0°". Without that rule a wind growing out of calm would swing
    through a meaningless direction and create a crosswind neither level has.
  - `Components` interpolates the east and north parts linearly, as RocketPy does. Between levels
    90° apart the speed dips by up to 29%.
  - Beyond the end levels the end wind is held and the sample is flagged.
  - Put the surface wind (for example the 10 m observation) in as the lowest level, so the
    profile blends up from it.
- **`WindModel`:** any one of the four, as a flight or a file takes it. In JSON its `model` field
  names which one ([In JSON](#in-json)).

**[Loft lesson L6](../decisions-and-roadmap.md#l6)**, a mistake found in Loft, the project before hpr-sim: design-file
runs used one wind vector, and forecast profiles stepped at the lowest level instead of blending
from the surface.

## Which height?

A flight asks the wind model for the wind at the rocket's
[height above mean sea level](../glossary.md#height-above-sea-level-msl). The laws are written in
height above the ground, so they take the ground's height above sea level as well. Each input's
name says which height it is: `_msl_` is above mean sea level, and `_agl_` is above the ground
([AGL](../glossary.md#agl-above-ground-level)).

| Model | Heights you give it | Measured from |
|---|---|---|
| `ConstantWind` | none: the same wind at every height | — |
| `PowerLawWind` | the reference height, `reference_height_agl_m` | the ground |
| | the ground's height, `ground_msl_m` | sea level |
| `LogLawWind` | the reference height, `reference_height_agl_m`, and the roughness length, `roughness_length_m` | the ground |
| | the ground's height, `ground_msl_m` | sea level |
| `LayeredWind` | each level's `height_msl_m` | sea level |
| A sounding's wind (`SoundingProfile`) | each level's `height_msl_m` | sea level |

So a table's levels, from a forecast or a sounding, are **above sea level**. At a site 1,400 m
above sea level, a forecast's 10 m wind goes in at 1,410 m. Entered at 10 m instead, the whole
profile sits 1,400 m too low. The [example](#an-example-of-each)'s last column shows what the
flight would then see: 11.6 m/s from 298° near the pad instead of 5.0 m/s from 270°, and the top
level's 12.0 m/s from 300° from 100 m up. Above that, every sample is
[flagged](#conventions) as beyond the table, which is the sign to look for.

**What to enter for your field's elevation.** The launch site's height, the third number in
`Geodetic::from_degrees(latitude, longitude, height)`, is an
[ellipsoidal height](../glossary.md#ellipsoidal-height): height above the
[WGS 84](../glossary.md#wgs-84) ellipsoid, the smooth shape hpr gives the Earth. A field's
elevation, as a map gives it, is height above sea level instead. The two differ by the geoid
undulation `N`, the height of sea level above the ellipsoid at that place, which can be up to
about 100 m. hpr has no map of `N`, so there are two ways to set up a site:

- **The simple way, which the examples take.** Enter your field's elevation above sea level as
  the site's height, and leave `N` at 0, as `Environment::standard` sets it. The air and the wind
  are then looked up at the right heights above sea level. What is off is the height above the
  ellipsoid, by your field's `N`. A flight uses that only to work out gravity, which changes by
  about 0.003% over 100 m of height.
- **The exact way, if you know `N` at your field.** Enter the elevation plus `N` as the site's
  height, and give the environment `N` with `with_geoid_undulation_m`
  ([`Environment`](../api/hpr_sim/environment/struct.Environment.html)).

Either way, the ground is the site's `height_m` less the environment's `geoid_undulation_m` above
sea level, which is how the example works it out. A power or log law needs that number as its
`ground_msl_m`, and doesn't take it from the site by itself. If its `ground_msl_m` is wrong, its
whole profile is shifted up or down by the error.

Three more things to watch:

- **Geopotential heights.** Soundings and forecasts often give heights in geopotential metres
  above sea level, not geometric metres (the metres a tape measure would give).
  - A geopotential metre measures height by the work done lifting a mass against gravity: it is
    the climb that takes as much work as one metre does where gravity is 9.80665 m/s².
  - Gravity varies with height and latitude, so the two differ by an amount that depends on
    both. 30 km above sea level is 29.7785 km of geopotential at the equator and 29.932 km at
    80° N.
  - Convert them first with
    [`geometric_from_wmo_geopotential_m`](../api/hpr_atmos/profile/fn.geometric_from_wmo_geopotential_m.html)
    ([Atmosphere](atmosphere.md#sounding-and-forecast-profiles) explains).
- **A sounding's wind** is a `LayeredWind` (`SoundingProfile::wind`). A flight flies it only if
  you also pass it to `with_wind`: the atmosphere and the wind are separate parts of
  `Environment`.
- **The flags.** A table flags a height below or above its levels, and a law a height below the
  ground ([Conventions](#conventions) says how to read a flag). A flight doesn't report flags yet
  ([issue #45](https://github.com/nrdptel/hpr-sim/issues/45)), so ask the model yourself before
  flying a table, as the example does.

## An example of each

The example program
[`wind_profiles.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/wind_profiles.rs)
builds each model for the launch site of [Getting started](../getting-started.md), 1,400 m above
sea level. It prints the wind each one gives at a few heights above the ground, and flies nothing.
Run it from anywhere in the repository:

```bash
cargo run --example wind_profiles -p hpr-sim
```

It prints this:

<!-- quote: crates/hpr-sim/examples/wind_profiles.output.txt -->
```text
Wind at a site 1400 m above sea level: speed in m/s, from a direction in °

above ground (m)      constant      power law        log law        layered    wrong datum
               2  5.0 from 270   4.0 from 270   3.6 from 270   5.0 from 270* 11.6 from 298
              10  5.0 from 270   5.0 from 270   5.0 from 270   5.0 from 270  11.6 from 298
              50  5.0 from 270   6.3 from 270   6.4 from 270   5.2 from 271  11.8 from 299
             100  5.0 from 270   6.9 from 270   7.0 from 270   5.6 from 272  12.0 from 300
             500  5.0 from 270   8.7 from 270   8.4 from 270   8.0 from 280  12.0 from 300*
            1000  5.0 from 270   9.7 from 270   9.0 from 270  10.0 from 290  12.0 from 300*
            2000  5.0 from 270  10.7 from 270   9.6 from 270  12.0 from 300* 12.0 from 300*

* beyond the table's levels: the end level's wind, held, and the sample flagged

{
  "model": "layered",
  "levels": [
    {
      "height_msl_m": 1410.0,
      "speed_m_s": 5.0,
      "direction_from_rad": 4.71238898038469
    },
    {
      "height_msl_m": 1900.0,
      "speed_m_s": 8.0,
      "direction_from_rad": 4.886921905584122
    },
    {
      "height_msl_m": 2900.0,
      "speed_m_s": 12.0,
      "direction_from_rad": 5.235987755982989
    }
  ],
  "interpolation": "speed_direction"
}
```

Reading it:

- **Each law passes through its reference wind,** 5.0 m/s at 10 m, and blows from 270° at every
  height.
- **The power and log laws** agree near their reference height and part away from it: 4.0 against
  3.6 m/s at 2 m, and 10.7 against 9.6 m/s at 2,000 m. Both keep growing far above the surface
  layer they describe, which is why winds aloft should come from a table.
- **The layered wind** blends from 5.0 m/s from 270° at 10 m to 8.0 m/s from 280° at 500 m,
  turning with height. At 2 m, below its lowest level, it holds the 10 m wind; at 2,000 m, above
  its top level, it holds 12.0 m/s from 300°. The star marks both as flagged.
- **The wrong datum** column is the same table with its heights entered above the ground by
  mistake ([Which height?](#which-height)).
- **The JSON** at the end is the layered wind as a file holds it ([In JSON](#in-json)).

To fly one of them, give it to the flight's environment, as the program does for each column:
`Environment::standard(site)?.with_wind(wind)`. Then build the `Simulation` from that environment,
as [Getting started](../getting-started.md) does with its constant wind.

The program, which CI compiles and runs on macOS, Windows and Linux:

<!-- quote: crates/hpr-sim/examples/wind_profiles.rs -->
```rust
//! Wind profiles: each kind of steady wind hpr can fly, built for a launch site 1,400 m above sea
//! level, with the wind each one gives at a few heights above the ground, and how a flight takes
//! one. Nothing is flown.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example wind_profiles -p hpr-sim
//! ```
//!
//! The documentation site's *Wind* page (`docs/physics/wind.md`) walks through it. What it prints
//! is kept next to it in `wind_profiles.output.txt`, and CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::{
    ConstantWind, LayeredWind, LogLawWind, PowerLawWind, WindInterpolation, WindLevel, WindModel,
};
use hpr_core::DVec3;
use hpr_core::geodesy::Geodetic;
use hpr_sim::Environment;

fn main() -> Result<(), Box<dyn Error>> {
    // The launch site of Getting started: New Mexico, 1,400 m up.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let calm = Environment::standard(site)?; // no wind until it is given one

    // A flight asks for the wind at its height above mean sea level: its height above the WGS 84
    // ellipsoid, which is how hpr takes the site's height too, less the geoid undulation N (0
    // unless you set it). So the ground is this high above sea level:
    let ground_msl_m = site.height_m - calm.geoid_undulation_m;
    let from_west = 270_f64.to_radians();

    // 1. The same wind at every height: 5 m/s from the west. It takes no height.
    let constant = ConstantWind::new(5.0, from_west)?;

    // 2. A power law through 5 m/s at 10 m above the ground, with exponent α = 1/7. The
    //    reference height is above the ground; the ground's height is above sea level.
    let power_law = PowerLawWind::new(5.0, 10.0, 1.0 / 7.0, from_west, ground_msl_m)?;

    // 3. A log law through the same 5 m/s at 10 m, over grass: roughness length z₀ = 0.03 m.
    let log_law = LogLawWind::new(5.0, 10.0, 0.03, from_west, ground_msl_m)?;

    // 4. A table of levels, as from a forecast. Its heights are above sea level, so a level given
    //    above the ground goes in at the ground's height plus its own. The 10 m wind is the
    //    lowest level, so the profile blends up from it.
    let level = |height_agl_m: f64, speed_m_s: f64, from_deg: f64| WindLevel {
        height_msl_m: ground_msl_m + height_agl_m,
        speed_m_s,
        direction_from_rad: from_deg.to_radians(),
    };
    let forecast = vec![
        level(10.0, 5.0, 270.0),
        level(500.0, 8.0, 280.0),
        level(1500.0, 12.0, 300.0),
    ];
    let layered = LayeredWind::new(forecast.clone(), WindInterpolation::SpeedDirection)?;

    // The same table with its heights entered above the ground by mistake: 1,400 m too low.
    let wrong_levels = forecast
        .iter()
        .map(|level| WindLevel {
            height_msl_m: level.height_msl_m - ground_msl_m,
            ..*level
        })
        .collect();
    let wrong_datum = LayeredWind::new(wrong_levels, WindInterpolation::SpeedDirection)?;

    // Any of them goes to a flight the same way, through the flight's environment. Each
    // environment here is the one a `Simulation` from this site would fly in.
    let columns = [
        ("constant", WindModel::Constant(constant)),
        ("power law", WindModel::PowerLaw(power_law)),
        ("log law", WindModel::LogLaw(log_law)),
        ("layered", WindModel::Layered(layered.clone())),
        ("wrong datum", WindModel::Layered(wrong_datum)),
    ];
    let mut environments = Vec::new();
    for (_, wind) in &columns {
        environments.push(Environment::standard(site)?.with_wind(wind.clone()));
    }

    println!(
        "Wind at a site {ground_msl_m:.0} m above sea level: speed in m/s, from a direction in °"
    );
    println!();
    let mut header = "above ground (m)".to_owned();
    for (name, _) in &columns {
        header += &format!("{name:>14} ");
    }
    println!("{}", header.trim_end());
    for height_agl_m in [2.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 2000.0_f64] {
        let mut row = format!("{height_agl_m:>16.0}");
        for environment in &environments {
            // What the flight engine asks for: the wind at a height above sea level.
            let sample = environment.wind.wind(ground_msl_m + height_agl_m)?;
            let (speed_m_s, from_deg) = speed_and_direction_from(sample.velocity_enu_m_s);
            let cell = format!("{speed_m_s:.1} from {from_deg:.0}");
            // A star marks a height beyond a table's levels, where the model holds the end
            // level's wind and flags the sample.
            let flag = if sample.extrapolated.is_some() {
                "*"
            } else {
                " "
            };
            row += &format!("{cell:>14}{flag}");
        }
        println!("{}", row.trim_end());
    }
    println!();
    println!("* beyond the table's levels: the end level's wind, held, and the sample flagged");
    println!();

    // The table as JSON, tagged by `model`. Directions are in radians.
    println!(
        "{}",
        serde_json::to_string_pretty(&WindModel::Layered(layered))?
    );
    Ok(())
}

/// The speed of a wind velocity (east, north, up), m/s, and the direction it blows from,
/// clockwise from north, rounded to a whole degree in `[0, 360)`.
fn speed_and_direction_from(velocity_enu_m_s: DVec3) -> (f64, f64) {
    let (east, north) = (velocity_enu_m_s.x, velocity_enu_m_s.y);
    let from_deg = (-east).atan2(-north).to_degrees().rem_euclid(360.0);
    // `% 360.0` turns a 359.6° into 0°, and `+ 0.0` turns a -0 into 0.
    (east.hypot(north), from_deg.round() % 360.0 + 0.0)
}
```

## In JSON

`WindModel` reads and writes JSON as one object. Its `model` field names the kind of wind, and the
other fields are that model's inputs:

| `model` | Other fields |
|---|---|
| `constant` | `speed_m_s`, `direction_from_rad` |
| `power_law` | `reference_speed_m_s`, `reference_height_agl_m`, `exponent`, `direction_from_rad`, `ground_msl_m` |
| `log_law` | `reference_speed_m_s`, `reference_height_agl_m`, `roughness_length_m`, `direction_from_rad`, `ground_msl_m` |
| `layered` | `levels`, each with `height_msl_m`, `speed_m_s` and `direction_from_rad`; and `interpolation`, `speed_direction` (the default if left out) or `components` |

- Directions are in radians, the direction the wind blows from: 270° is 4.71238898038469, as the
  example's JSON shows.
- Reading a file checks it as the model's constructor does: a negative speed, or a field the model
  doesn't have, is an error.

## Not yet modelled

- Wind that changes with time, or across the field.
- Vertical mean wind.
- Terrain effects.
- A blend below the lowest tabulated level (the table holds the lowest level's wind).
- Reporting during a flight that the wind was held beyond a table's levels.

A flight takes one wind model, so it can't join a power or log law near the ground to a table of
levels aloft. The planned weather milestone ([M5.2](../decisions-and-roadmap.md#m5-2)) decides how to.

## Tests that pin this

- **`wind::tests::layered_wind_interpolates_speed_and_heading`** ([Loft lesson L6](../decisions-and-roadmap.md#l6), the
  forecast profile that stepped at its lowest level):
  - Halfway between 4 m/s from 350° and 12 m/s from 30°, the wind is 8 m/s from 10°, turning
    through north.
  - There is no step just above the surface level.
- **`components_interpolation_averages_the_vectors`**, **`opposite_directions_turn_clockwise`**
  and **`wind_grows_out_of_calm_without_turning`**.
- **`layered_wind_holds_and_flags_beyond_its_levels`**.
- **`meteorological_direction_convention`**, plus the power and log laws through their
  references, below ground, and at `z₀`.
- **`invalid_inputs_are_rejected`**, and **`wind_models_round_trip_through_json`**: each model
  written to JSON and read back gives the same wind.
- **Against RocketPy**, in the recovery test `descent_matches_rocketpy_examples`
  ([Recovery](recovery.md#against-rocketpy)):
  - At every height its parachute descents sample, hpr's wind matches RocketPy's within 1e-9 m/s
    in its east and north parts.
  - Four of the five descents fly in wind: Calisto, NDRT 2020, Prometheus and Juno III. Their total
    drift agrees within 0.28%, and each east or north part within 2.9%. The largest gap is the
    north part of NDRT 2020's drift, +2.865% (the
    [validation report](https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md)).
- The example above, which CI runs and compares with its committed output.

[lock]: https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml
