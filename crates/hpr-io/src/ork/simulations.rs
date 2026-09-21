//! The simulations OpenRocket last ran on a `.ork` design: the launch conditions each was flown in
//! and the results it stored.
//!
//! **Where a `.ork` keeps them.** `<simulations>` holds one `<simulation>` per run, with a name, a
//! `status`, the `<conditions>` it was flown in, and, when OpenRocket saved them, its results in
//! `<flightdata>`: ten summary figures as attributes, and a `<databranch>` per stage, each a time
//! series (`<datapoint>` rows, comma-separated, in the order its `types` attribute names) with the
//! flight's `<event>`s ([the file specification][spec], *Simulation Data*).
//!
//! **What the numbers mean.** The file-format page gives units only for the multilevel wind
//! (metres, m/s and radians), and its own example writes the launch rod's direction as `90.0` and
//! the wind's as `1.5707963267948966`. A committed probe
//! (`validation/oracles/openrocket/conditions.py`, results in
//! `validation/fixtures/ork/openrocket-conditions.json`) runs OpenRocket 24.12 and settles it:
//!
//! - `launchrodangle` and `launchroddirection` are **degrees**; this reader gives them in radians.
//!   The direction is a compass bearing, clockwise from north: a rod tilted toward 90 lands the
//!   rocket to the east. With `launchintowind`, OpenRocket writes the wind's bearing, in degrees,
//!   as the rod's.
//! - `winddirection` is **radians**, the bearing the wind blows **from**: in a wind from the east a
//!   rocket drifts west. It is never the rod's direction ([Loft lesson L64][l64]: Loft read it from
//!   `launchroddirection`).
//! - In the eight columns the probe compared, the time series is SI with angles in radians and
//!   latitude in degrees, rounded when stored: to three decimal places (287.857 K), and a large
//!   value to four significant figures (100,796.6 Pa is stored as 100,800). `NaN` marks a quantity
//!   OpenRocket did not compute at that step, and this reader keeps it as `None`.
//!
//! All of this is measured on OpenRocket 24.12; a file written by a much older version may have
//! meant the rod's direction otherwise, which is not measured.
//!
//! [spec]: https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html
//! [l64]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l64

use serde::{Deserialize, Serialize};

use super::document::{Document, Element};
use super::value::Values;
use super::warning::{Warning, WarningKind};

/// A simulation stored in a `.ork`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredSimulation {
    /// `<name>`.
    pub name: String,
    /// The `status` attribute as written: `uptodate`, `outdated`, `loaded`, `external`,
    /// `notsimulated`, `cantrun` or `aborted` in the files OpenRocket 24.12 writes.
    pub status: Option<String>,
    /// `<simulator>`, such as `RK4Simulator`.
    pub simulator: Option<String>,
    /// `<calculator>`, such as `BarrowmanCalculator`.
    pub calculator: Option<String>,
    /// `<conditions>`: what the run was flown in.
    pub conditions: Option<LaunchConditions>,
    /// `<flightdata>`: what it gave, when OpenRocket saved it.
    pub results: Option<StoredResults>,
}

/// The launch conditions a stored simulation was flown in. Each is `None` where the file does not
/// say.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LaunchConditions {
    /// `<configid>`: the motor configuration flown.
    pub configuration: Option<String>,
    /// `<launchrodlength>`, m.
    pub rod_length_m: Option<f64>,
    /// `<launchrodangle>`: the rod's tilt from vertical, rad (the file writes degrees).
    pub rod_angle_rad: Option<f64>,
    /// `<launchroddirection>`: the compass bearing the rod tilts toward, clockwise from north, rad
    /// (the file writes degrees).
    pub rod_direction_rad: Option<f64>,
    /// `<launchintowind>`: whether the rod is pointed into the wind, which OpenRocket then writes
    /// as the rod's direction.
    pub into_wind: Option<bool>,
    /// `<windaverage>`, or the average wind's `<speed>`: the mean wind speed, m/s.
    pub wind_speed_m_s: Option<f64>,
    /// `<windturbulence>`: the turbulence intensity, the standard deviation of the wind speed over
    /// its mean.
    pub wind_turbulence: Option<f64>,
    /// `<winddirection>`, or the average wind's `<direction>`: the compass bearing the wind blows
    /// from, rad (the file writes radians).
    pub wind_from_rad: Option<f64>,
    /// `<windmodeltype>`: which wind the run flew, `Average` or the multilevel one. OpenRocket
    /// writes both winds whichever it flew.
    pub wind_model: Option<String>,
    /// `<wind model="multilevel">`'s levels, lowest first as written.
    pub wind_levels: Vec<WindLevel>,
    /// The multilevel wind's `altituderef`: whether its altitudes are above the ground (`agl`) or
    /// the sea (`msl`).
    pub wind_levels_above: Option<String>,
    /// `<launchaltitude>`: the launch site's height above sea level, m.
    pub launch_altitude_m: Option<f64>,
    /// `<launchlatitude>`, degrees north.
    pub latitude_deg: Option<f64>,
    /// `<launchlongitude>`, degrees east.
    pub longitude_deg: Option<f64>,
    /// `<geodeticmethod>`: `flat`, `spherical` or `wgs84`.
    pub geodetic_method: Option<String>,
    /// `<atmosphere>`.
    pub atmosphere: Option<Atmosphere>,
    /// `<timestep>`, s.
    pub time_step_s: Option<f64>,
    /// `<maxtime>`, s.
    pub max_time_s: Option<f64>,
}

/// One level of a multilevel wind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WindLevel {
    /// Its altitude, m, above the ground or the sea as [`LaunchConditions::wind_levels_above`]
    /// says.
    pub altitude_m: Option<f64>,
    /// Its mean speed, m/s.
    pub speed_m_s: Option<f64>,
    /// The bearing it blows from, rad.
    pub from_rad: Option<f64>,
    /// The standard deviation of its speed, m/s.
    pub standard_deviation_m_s: Option<f64>,
}

/// The atmosphere a stored simulation was flown in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "model")]
#[non_exhaustive]
pub enum Atmosphere {
    /// `isa`: the International Standard Atmosphere.
    Isa,
    /// `extendedisa`: the standard atmosphere from a temperature and pressure at the launch site.
    Extended {
        /// `<basetemperature>`, K.
        temperature_k: Option<f64>,
        /// `<basepressure>`, Pa.
        pressure_pa: Option<f64>,
    },
    /// A model not listed here, or none, kept by name only.
    Other {
        /// The `model` attribute; empty when there is none.
        name: String,
    },
}

/// What a stored simulation gave: its summary and its time series.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredResults {
    /// `maxaltitude`, m.
    pub max_altitude_m: Option<f64>,
    /// `maxvelocity`, m/s.
    pub max_speed_m_s: Option<f64>,
    /// `maxacceleration`, m/s².
    pub max_acceleration_m_s2: Option<f64>,
    /// `maxmach`.
    pub max_mach: Option<f64>,
    /// `timetoapogee`, s.
    pub time_to_apogee_s: Option<f64>,
    /// `flighttime`, s.
    pub flight_time_s: Option<f64>,
    /// `groundhitvelocity`, m/s.
    pub ground_hit_speed_m_s: Option<f64>,
    /// `launchrodvelocity`: the speed leaving the rod, m/s.
    pub rod_exit_speed_m_s: Option<f64>,
    /// `deploymentvelocity`: the speed at the first deployment, m/s.
    pub deployment_speed_m_s: Option<f64>,
    /// `optimumdelay`: the ejection delay that would have fired at apogee, s.
    pub optimum_delay_s: Option<f64>,
    /// The time series, one per stage, in file order.
    pub branches: Vec<StoredBranch>,
    /// The `<warning>`s OpenRocket stored with the results, each as its text.
    pub warnings: Vec<String>,
}

/// One stage's stored time series.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredBranch {
    /// `name`: the stage's name.
    pub name: String,
    /// `types`: each column's name as OpenRocket shows it, such as `Time` or `Altitude`.
    pub types: Vec<String>,
    /// The `<datapoint>` rows, each a value per column, as written: SI, angles in radians, latitude
    /// and longitude in degrees. `None` where the file says `NaN`, a quantity OpenRocket did not
    /// compute at that step.
    pub rows: Vec<Vec<Option<f64>>>,
    /// The `<event>`s, in file order.
    pub events: Vec<StoredEvent>,
}

impl StoredBranch {
    /// The column called `name`, one value per row; `None` where OpenRocket did not compute it.
    pub fn column(&self, name: &str) -> Option<Vec<Option<f64>>> {
        let index = self.types.iter().position(|column| column == name)?;
        Some(
            self.rows
                .iter()
                .map(|row| row.get(index).copied().flatten())
                .collect(),
        )
    }
}

/// An event a stored simulation logged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StoredEvent {
    /// `time`, s.
    pub time_s: f64,
    /// `type`, such as `apogee` or `recoverydevicedeployment`.
    pub kind: String,
    /// `source`: the id of the component it came from, if any.
    pub source: Option<String>,
}

/// Reads every stored simulation in `document`.
pub(super) fn read(document: &Document, warnings: &mut Vec<Warning>) -> Vec<StoredSimulation> {
    let Some(simulations) = document.root.child("simulations") else {
        return Vec::new();
    };
    simulations
        .children_named("simulation")
        .enumerate()
        .map(|(index, element)| {
            let at = format!("openrocket/simulations/simulation[{index}]");
            simulation(element, &at, warnings)
        })
        .collect()
}

fn simulation(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> StoredSimulation {
    // Looked up directly below, not through `Values`.
    super::reads::note(element, "conditions");
    super::reads::note(element, "flightdata");
    let mut values = Values::new(element, at, warnings);
    let name = values.word(&["name"]).unwrap_or_default();
    let simulator = values.word(&["simulator"]);
    let calculator = values.word(&["calculator"]);
    let conditions = element
        .child("conditions")
        .map(|c| conditions(c, &format!("{at}/conditions"), warnings));
    let results = element
        .child("flightdata")
        .map(|f| results(f, &format!("{at}/flightdata"), warnings));
    StoredSimulation {
        name,
        status: element.attribute("status").map(str::to_owned),
        simulator,
        calculator,
        conditions,
        results,
    }
}

fn conditions(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> LaunchConditions {
    // Looked up directly below, not through `Values`.
    super::reads::note(element, "wind");
    super::reads::note(element, "atmosphere");
    let average = element
        .children_named("wind")
        .find(|wind| wind.attribute("model") == Some("average"));
    let multilevel = element
        .children_named("wind")
        .find(|wind| wind.attribute("model") == Some("multilevel"));
    let (average_speed, average_from) = match average {
        Some(wind) => {
            let mut values = Values::new(wind, at, warnings);
            (values.number(&["speed"]), values.number(&["direction"]))
        }
        None => (None, None),
    };
    let wind_levels = multilevel.map_or_else(Vec::new, |wind| {
        super::reads::note(wind, "windlevel");
        wind.children_named("windlevel")
            .map(|level| {
                let mut number = |name: &str| attribute_number(level, name, at, warnings);
                WindLevel {
                    altitude_m: number("altitude"),
                    speed_m_s: number("speed"),
                    from_rad: number("direction"),
                    standard_deviation_m_s: number("standarddeviation"),
                }
            })
            .collect()
    });
    let atmosphere = element.child("atmosphere").map(|atmosphere| {
        let model = atmosphere.attribute("model").unwrap_or_default();
        let unread = match model {
            "isa" | "extendedisa" => None,
            "" => Some(
                "an atmosphere with no `model`, as older files write their own table; it was not \
                 read"
                    .to_owned(),
            ),
            other => Some(format!(
                "an atmosphere whose model is `{other}`, which OpenRocket 24.12 does not write; it \
                 was kept by name only"
            )),
        };
        if let Some(message) = unread {
            warnings.push(Warning::new(
                format!("{at}/atmosphere"),
                WarningKind::Unusual,
                message,
            ));
        }
        match model {
            "isa" => Atmosphere::Isa,
            "extendedisa" => {
                let mut values = Values::new(atmosphere, at, warnings);
                Atmosphere::Extended {
                    temperature_k: values.number(&["basetemperature"]),
                    pressure_pa: values.number(&["basepressure"]),
                }
            }
            other => Atmosphere::Other {
                name: other.to_owned(),
            },
        }
    });
    let mut values = Values::new(element, at, warnings);
    let legacy_speed = values.number(&["windaverage"]);
    let legacy_from = values.number(&["winddirection"]);
    for (what, legacy, average) in [
        ("speed", legacy_speed, average_speed),
        ("direction", legacy_from, average_from),
    ] {
        if let (Some(legacy), Some(average)) = (legacy, average)
            && legacy != average
        {
            values.warn_at(
                WarningKind::Dropped,
                format!(
                    "the wind's {what} is {legacy} in the legacy tag and {average} in the average \
                     `wind`; the legacy tag's was taken"
                ),
            );
        }
    }
    LaunchConditions {
        configuration: values.word(&["configid"]).filter(|id| !id.is_empty()),
        rod_length_m: values.number(&["launchrodlength"]),
        rod_angle_rad: values.number(&["launchrodangle"]).map(f64::to_radians),
        rod_direction_rad: values.number(&["launchroddirection"]).map(f64::to_radians),
        into_wind: values.flag(&["launchintowind"]),
        wind_speed_m_s: legacy_speed.or(average_speed),
        wind_turbulence: values.number(&["windturbulence"]),
        wind_from_rad: legacy_from.or(average_from),
        wind_model: values.word(&["windmodeltype"]),
        wind_levels,
        wind_levels_above: multilevel
            .and_then(|wind| wind.attribute("altituderef"))
            .map(str::to_owned),
        launch_altitude_m: values.number(&["launchaltitude"]),
        latitude_deg: values.number(&["launchlatitude"]),
        longitude_deg: values.number(&["launchlongitude"]),
        geodetic_method: values.word(&["geodeticmethod"]),
        atmosphere,
        time_step_s: values.number(&["timestep"]),
        max_time_s: values.number(&["maxtime"]),
    }
}

/// An attribute's number, or `None` with a warning when it is there and not a finite number.
fn attribute_number(
    element: &Element,
    name: &str,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> Option<f64> {
    let text = element.attribute(name)?;
    match text.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => Some(value),
        _ => {
            warnings.push(Warning::new(
                at,
                WarningKind::Dropped,
                format!("`{name}` says `{text}`, which is not a number; it was ignored"),
            ));
            None
        }
    }
}

fn results(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> StoredResults {
    // Looked up directly below, not through `Values`.
    super::reads::note(element, "warning");
    super::reads::note(element, "databranch");
    let mut number = |name: &str| attribute_number(element, name, at, warnings);
    let mut results = StoredResults {
        max_altitude_m: number("maxaltitude"),
        max_speed_m_s: number("maxvelocity"),
        max_acceleration_m_s2: number("maxacceleration"),
        max_mach: number("maxmach"),
        time_to_apogee_s: number("timetoapogee"),
        flight_time_s: number("flighttime"),
        ground_hit_speed_m_s: number("groundhitvelocity"),
        rod_exit_speed_m_s: number("launchrodvelocity"),
        deployment_speed_m_s: number("deploymentvelocity"),
        optimum_delay_s: number("optimumdelay"),
        branches: Vec::new(),
        warnings: element
            .children_named("warning")
            .map(|warning| warning.text().trim().to_owned())
            .collect(),
    };
    for (index, element) in element.children_named("databranch").enumerate() {
        let at = format!("{at}/databranch[{index}]");
        results.branches.push(branch(element, &at, warnings));
    }
    results
}

fn branch(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> StoredBranch {
    let types: Vec<String> = element
        .attribute("types")
        .map(|types| types.split(',').map(|t| t.trim().to_owned()).collect())
        .unwrap_or_default();
    let mut rows = Vec::new();
    let mut dropped = 0usize;
    let mut infinite = 0usize;
    super::reads::note(element, "datapoint");
    super::reads::note(element, "event");
    for point in element.children_named("datapoint") {
        let text = point.text();
        // `NaN` is a value OpenRocket did not compute, kept as `None`; a number that does not read
        // spoils the row. An infinity is kept as `None` too, and counted.
        let row: Option<Vec<Option<f64>>> = text
            .split(',')
            .map(|value| match value.trim().parse::<f64>() {
                Ok(v) if v.is_nan() => Some(None),
                Ok(v) if v.is_finite() => Some(Some(v)),
                Ok(_) => {
                    infinite += 1;
                    Some(None)
                }
                Err(_) => None,
            })
            .collect();
        match row {
            Some(row) if row.len() == types.len() => rows.push(row),
            _ => dropped += 1,
        }
    }
    if infinite > 0 {
        warnings.push(Warning::new(
            at,
            WarningKind::Dropped,
            format!("{infinite} infinite value(s) in the rows were kept as not computed"),
        ));
    }
    if dropped > 0 {
        warnings.push(Warning::new(
            at,
            WarningKind::Dropped,
            format!(
                "{dropped} `datapoint` row(s) are not {} numbers, one per column; they were left \
                 out",
                types.len()
            ),
        ));
    }
    let mut events = Vec::new();
    for event in element.children_named("event") {
        let (Some(time_s), Some(kind)) = (
            attribute_number(event, "time", at, warnings),
            event
                .attribute("type")
                .filter(|kind| !kind.trim().is_empty()),
        ) else {
            warnings.push(Warning::new(
                at,
                WarningKind::Dropped,
                "an `event` with no time or no type; it was left out",
            ));
            continue;
        };
        events.push(StoredEvent {
            time_s,
            kind: kind.trim().to_owned(),
            source: event.attribute("source").map(str::to_owned),
        });
    }
    StoredBranch {
        name: element.attribute("name").unwrap_or_default().to_owned(),
        types,
        rows,
        events,
    }
}
