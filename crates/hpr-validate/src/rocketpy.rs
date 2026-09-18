//! Reading RocketPy's generator output into the harness's shapes.
//!
//! The fixtures under `validation/fixtures/` are written by the scripts under
//! `validation/oracles/rocketpy/`, in each script's own shape. This module is the only place that
//! knows those shapes: it turns one of the generator's cases into a [`Reference`] (what the oracle
//! said, with a source for every value) and the inputs hpr must fly, a [`DescentSetup`] for
//! `recovery.py`'s descents or a [`WholeFlightSetup`] for `flight.py`'s flights from the pad.
//!
//! [Loft lesson L75][l75]: the inputs come from **the reference's own record of what it flew**,
//! never from an hpr output, so a case cannot quietly compare hpr against itself. That includes
//! which rocket it flew and what that rocket weighed, which the harness checks rather than trusts.
//!
//! [l75]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l75

use serde_json::Value;

use crate::metrics::{Reference, ReferenceValue};
use crate::run::{
    DescentDevice, DescentSetup, FlightDevice, FlightMotor, SeriesRow, WholeFlightSetup,
};

/// The case named `case` of a recovery-descent fixture, as the oracle's answers and the inputs it
/// flew. `None` if the document has no such case, does not name the run that produced it, or is
/// not that fixture's shape.
///
/// `file` is the reference's path from the repository root and `sha256` its hash; both go into the
/// report, so a hand-edited reference changes the report rather than only the file.
#[must_use]
pub fn descent_case(
    document: &Value,
    case: &str,
    file: &str,
    sha256: &str,
) -> Option<(Reference, DescentSetup)> {
    let (reference, found) = reference_of(document, case, file, sha256)?;
    let environment = found.get("environment")?;
    let elevation_m = number(environment.get("elevation_m"))?;
    let start = found.get("start")?;
    let position = start.get("position_msl_m")?.as_array()?;
    let velocity = start.get("velocity_m_s")?.as_array()?;
    let devices = found
        .get("devices")?
        .as_array()?
        .iter()
        .map(|device| {
            Some(DescentDevice {
                name: device.get("name")?.as_str()?.to_owned(),
                cd_s_m2: number(device.get("cd_s_m2"))?,
                lag_s: number(device.get("lag_s"))?,
                height_above_ground_m: match device.get("trigger")?.get("kind")?.as_str()? {
                    "apogee" => None,
                    "descending_below_height_agl" => {
                        Some(number(device.get("trigger")?.get("height_m"))?)
                    }
                    _ => return None,
                },
            })
        })
        .collect::<Option<Vec<_>>>()?;

    let setup = DescentSetup {
        design: text(found.get("design"))?,
        dry_mass_kg: positive(number(found.get("dry_mass_kg"))?)?,
        start_height_above_ground_m: number(start.get("height_above_ground_m"))?,
        latitude_deg: number(environment.get("latitude_deg"))?,
        longitude_deg: number(environment.get("longitude_deg"))?,
        elevation_m,
        wind: wind_levels(environment, elevation_m)?,
        start_time_s: number(start.get("time_s"))?,
        start_position_m: (
            number(position.first())?,
            number(position.get(1))?,
            number(position.get(2))?,
        ),
        start_velocity_m_s: (
            number(velocity.first())?,
            number(velocity.get(1))?,
            number(velocity.get(2))?,
        ),
        devices,
    };
    Some((reference, setup))
}

/// The case named `case` of a whole-flight fixture (`flight.py`'s), as the oracle's answers and
/// the inputs it flew: the site and wind, the rail, the declared drag table (or, in an own-drag
/// fixture, where the example's own drag came from) with the reference
/// area it was flown on, the dry mass, and the parachutes in the order they open. `None` if the
/// document has no such case, does not name the run that produced it, or is not that shape.
///
/// Every input comes from the case's own record, which is what the oracle flew
/// ([Loft lesson L75][l75]); none is filled in from hpr's design, which the harness instead checks
/// against it. `file` and `sha256` are as for [`descent_case`].
///
/// [l75]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l75
#[must_use]
pub fn whole_flight_case(
    document: &Value,
    case: &str,
    file: &str,
    sha256: &str,
) -> Option<(Reference, WholeFlightSetup)> {
    let (reference, found) = reference_of(document, case, file, sha256)?;
    let environment = found.get("environment")?;
    let elevation_m = number(environment.get("elevation_m"))?;
    let rail = found.get("rail")?;
    let drag = found.get("drag")?;
    let motor = found.get("motor")?;
    let devices = found
        .get("devices")?
        .as_array()?
        .iter()
        .map(|device| {
            let trigger = device.get("trigger")?;
            Some(FlightDevice {
                name: text(device.get("name"))?,
                cd_s_m2: positive(number(device.get("cd_s_m2"))?)?,
                lag_s: number(device.get("lag_s"))?,
                height_above_ground_m: match trigger.get("kind")?.as_str()? {
                    "apogee" => None,
                    "descending_below_height_agl" => Some(number(trigger.get("height_m"))?),
                    _ => return None,
                },
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let setup = WholeFlightSetup {
        design: text(found.get("design"))?,
        dry_mass_kg: positive(number(found.get("dry_mass_kg"))?)?,
        latitude_deg: number(environment.get("latitude_deg"))?,
        longitude_deg: number(environment.get("longitude_deg"))?,
        elevation_m,
        wind: wind_levels(environment, elevation_m)?,
        rail_length_m: positive(number(rail.get("rail_length_m"))?)?,
        inclination_deg: number(rail.get("inclination_deg"))?,
        heading_deg: number(rail.get("heading_deg"))?,
        effective_1rl_m: positive(number(rail.get("effective_1rl_m"))?)?,
        motor: FlightMotor {
            total_impulse_ns: positive(number(motor.get("total_impulse_ns"))?)?,
            burn_out_time_s: positive(number(motor.get("burn_out_time_s"))?)?,
            propellant_initial_mass_kg: positive(number(motor.get("propellant_initial_mass_kg"))?)?,
            // RocketPy's None is JSON's null: no correction, which is a value, not a gap.
            reference_pressure_pa: match motor.get("reference_pressure_pa")? {
                Value::Null => None,
                value => Some(number(Some(value))?),
            },
        },
        // A same-drag reference records the table its case flew and the one its generator
        // declares; an own-drag reference records where each example's drag came from instead.
        cd0_vs_mach: match drag.get("own") {
            Some(_) => None,
            None => Some(pairs(drag.get("cd0_vs_mach"))?),
        },
        declared_cd0_vs_mach: match drag.get("own") {
            Some(_) => None,
            None => Some(pairs(document.get("declared_drag")?.get("cd0_vs_mach"))?),
        },
        own_drag_source: match drag.get("own") {
            Some(_) => Some(text(drag.get("source"))?),
            None => None,
        },
        reference_radius_m: positive(number(drag.get("reference_radius_m"))?)?,
        reference_area_m2: positive(number(drag.get("reference_area_m2"))?)?,
        devices,
        series: series(found.get("series"))?,
    };
    let mut reference = reference;
    for name in ["series_height_rms_m", "series_speed_rms_m_s"] {
        reference.values.insert(
            name.to_owned(),
            ReferenceValue {
                value: 0.0,
                source: format!(
                    "0, exact agreement with {}, case {case}, series: the RMS of hpr's value less \
                     the reference's at its times",
                    reference.generator
                ),
            },
        );
    }
    Some((reference, setup))
}

/// The provenance a fixture's header gives and the values its case `case` publishes, with that
/// case's own record for the caller to read its inputs from.
fn reference_of<'a>(
    document: &'a Value,
    case: &str,
    file: &str,
    sha256: &str,
) -> Option<(Reference, &'a Value)> {
    let oracle = text(document.get("oracle"))?;
    let generator = text(document.get("generator"))?;
    let command = text(document.get("command"))?;
    let model = text(document.get("model"))?;
    let overrides = text(document.get("overrides"))?;
    let found = document
        .get("cases")?
        .as_array()?
        .iter()
        .find(|entry| entry.get("name").and_then(Value::as_str) == Some(case))?;

    let metrics = found.get("metrics")?.as_object()?;
    let mut values = std::collections::BTreeMap::new();
    for (name, value) in metrics {
        values.insert(
            name.clone(),
            ReferenceValue {
                value: value.as_f64()?,
                source: format!("{oracle}, {generator}, case {case}, metrics.{name}"),
            },
        );
    }
    let reference = Reference {
        oracle,
        generator,
        command,
        model,
        overrides,
        case: case.to_owned(),
        file: file.to_owned(),
        sha256: sha256.to_owned(),
        values,
    };
    Some((reference, found))
}

/// A table of `[x, y]` rows.
fn pairs(value: Option<&Value>) -> Option<Vec<(f64, f64)>> {
    value?
        .as_array()?
        .iter()
        .map(|row| {
            let row = row.as_array()?;
            (row.len() == 2).then_some(())?;
            Some((number(row.first())?, number(row.get(1))?))
        })
        .collect()
}

/// The `(time, height, speed)` rows of a whole flight's `series`, if its columns are those, in that
/// order, and its times rise from 0.
fn series(value: Option<&Value>) -> Option<Vec<SeriesRow>> {
    let value = value?;
    let columns: Vec<&str> = value
        .get("columns")?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect::<Option<_>>()?;
    (columns == ["time_s", "height_above_ground_m", "speed_m_s"]).then_some(())?;
    let rows = value
        .get("rows")?
        .as_array()?
        .iter()
        .map(|row| {
            let row = row.as_array()?;
            (row.len() == 3).then_some(())?;
            Some((
                number(row.first())?,
                number(row.get(1))?,
                number(row.get(2))?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    let starts_at_ignition = rows.first()?.0 == 0.0;
    let rising = rows.windows(2).all(|pair| pair[1].0 > pair[0].0);
    (starts_at_ignition && rising).then_some(rows)
}

/// The wind the oracle flew, as `(height above sea level, east, north)` levels. A scalar is one
/// level at the site; a profile is the pairs the generator recorded.
fn wind_levels(environment: &Value, elevation_m: f64) -> Option<Vec<(f64, f64, f64)>> {
    let east = environment.get("wind_u")?;
    let north = environment.get("wind_v")?;
    match (east.as_array(), north.as_array()) {
        (Some(east), Some(north)) if east.len() == north.len() => east
            .iter()
            .zip(north)
            .map(|(east, north)| {
                let east = east.as_array()?;
                let north = north.as_array()?;
                let height = number(east.first())?;
                if (height - number(north.first())?).abs() > 0.0 {
                    return None;
                }
                Some((height, number(east.get(1))?, number(north.get(1))?))
            })
            .collect::<Option<Vec<_>>>(),
        (None, None) => Some(vec![(
            elevation_m,
            number(Some(east))?,
            number(Some(north))?,
        )]),
        _ => None,
    }
}

/// A JSON string that says something.
fn text(value: Option<&Value>) -> Option<String> {
    let text = value?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// A JSON number, however it is written.
fn number(value: Option<&Value>) -> Option<f64> {
    value?.as_f64()
}

/// A number that can be a mass.
fn positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

#[cfg(test)]
mod tests;
