//! Reading RocketPy's generator output into the harness's shapes.
//!
//! The fixtures under `validation/fixtures/` are written by the scripts under
//! `validation/oracles/rocketpy/`, in each script's own shape. This module is the only place that
//! knows those shapes: it turns one of the generator's cases into a [`Reference`] (what the oracle
//! said, with a source for every value) and a [`DescentSetup`] (the inputs hpr must fly).
//!
//! [Loft lesson L75][l75]: the inputs come from **the reference's own record of what it flew**,
//! never from an hpr output, so a case cannot quietly compare hpr against itself. That includes
//! which rocket it flew and what that rocket weighed, which the harness checks rather than trusts.
//!
//! [l75]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l75

use serde_json::Value;

use crate::metrics::{Reference, ReferenceValue};
use crate::run::{DescentDevice, DescentSetup};

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
    Some((reference, setup))
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
