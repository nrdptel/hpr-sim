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
