//! `Environment` and `Flight`: where a rocket flies, and its flight.

use hpr::hpr_sim::{Channel, Recorder};
use numpy::{PyArray1, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::rocket::Rocket;
use crate::{error, to_python};

/// Where a rocket flies: the launch site, the US Standard Atmosphere 1976, and a wind.
///
/// `Environment(latitude_deg, longitude_deg, elevation_m, *, wind_speed_m_s=0.0,
/// wind_from_deg=0.0)`: a launch site at `latitude_deg` and `longitude_deg` (positive east, so
/// negative in the Americas), `elevation_m` above sea level, with no wind, or a wind of
/// `wind_speed_m_s` blowing **from** `wind_from_deg`, clockwise from true north, the same at
/// every height.
#[pyclass(module = "hpr", frozen, skip_from_py_object)]
#[derive(Debug)]
pub struct Environment {
    environment: hpr::Environment,
}

#[pymethods]
impl Environment {
    #[new]
    #[pyo3(signature = (latitude_deg, longitude_deg, elevation_m, *, wind_speed_m_s = 0.0, wind_from_deg = 0.0))]
    fn new(
        latitude_deg: f64,
        longitude_deg: f64,
        elevation_m: f64,
        wind_speed_m_s: f64,
        wind_from_deg: f64,
    ) -> PyResult<Self> {
        let mut environment =
            hpr::Environment::new(latitude_deg, longitude_deg, elevation_m).map_err(error)?;
        if wind_speed_m_s != 0.0 || wind_from_deg != 0.0 {
            environment = environment
                .with_constant_wind(wind_speed_m_s, wind_from_deg)
                .map_err(error)?;
        }
        Ok(Self { environment })
    }

    fn __repr__(&self) -> String {
        let site = self.environment.sim().site();
        format!(
            "Environment(latitude_deg={}, longitude_deg={}, elevation_m={})",
            site.latitude_rad.to_degrees(),
            site.longitude_rad.to_degrees(),
            site.height_m
        )
    }
}

/// A flight: `rocket` launched in `environment` from a rail `rail_length_m` long, flown to the
/// ground as soon as it is made.
///
/// `Flight(rocket, environment, rail_length_m, *, inclination_deg=90.0, heading_deg=0.0,
/// interval_s=None)`: the rail is measured from the rocket's aft end to the rail's top, and leans
/// `inclination_deg` above the horizon (90 is vertical) toward `heading_deg`, clockwise from true
/// north. The flight is recorded every `interval_s` seconds, at least 0.001 s, and at every event;
/// or at every step of the integrator when that is left out. The rocket is copied as the flight
/// starts, and a flight runs to its end: it can't be interrupted.
///
/// Heights are the rocket's centre of gravity's, above the launch site: it stands on the rail at
/// the start, so the first height isn't zero. Speeds are relative to the ground.
#[pyclass(module = "hpr", frozen, skip_from_py_object)]
#[derive(Debug)]
pub struct Flight {
    flight: hpr::Flight,
    /// The recording's column names, and each column's values.
    columns: Vec<String>,
    values: Vec<Vec<f64>>,
}

/// The shortest recording interval, s, as `hpr sim`'s: a shorter one fills memory with rows long
/// before it adds anything.
const MIN_INTERVAL_S: f64 = 0.001;

#[pymethods]
impl Flight {
    #[new]
    #[pyo3(signature = (rocket, environment, rail_length_m, *, inclination_deg = 90.0, heading_deg = 0.0, interval_s = None))]
    fn new(
        py: Python<'_>,
        rocket: &Bound<'_, Rocket>,
        environment: &Environment,
        rail_length_m: f64,
        inclination_deg: f64,
        heading_deg: f64,
        interval_s: Option<f64>,
    ) -> PyResult<Self> {
        if let Some(interval_s) = interval_s
            && (interval_s.is_nan() || interval_s < MIN_INTERVAL_S)
        {
            return Err(error(format!(
                "the recording's interval is {interval_s} s, and at least {MIN_INTERVAL_S} s"
            )));
        }
        let mut recorder = Recorder::new(Channel::ALL.to_vec(), interval_s).map_err(error)?;
        // A copy, so the rocket isn't held borrowed while the flight runs without the GIL.
        let rocket = rocket.borrow().rocket.clone();
        let builder = hpr::Flight::builder(&rocket, &environment.environment, rail_length_m)
            .inclination_deg(inclination_deg)
            .heading_deg(heading_deg);
        // The flight holds no Python object, so other Python threads run while it flies.
        let flight = py
            .detach(|| builder.fly_with(&mut recorder))
            .map_err(error)?;
        let columns = recorder.columns();
        let rows = recorder.rows();
        let values = (0..columns.len())
            .map(|index| {
                rows.iter()
                    .map(|row| row.get(index).copied().unwrap_or(f64::NAN))
                    .collect()
            })
            .collect();
        Ok(Self {
            flight,
            columns,
            values,
        })
    }

    /// The apogee, m above the launch site; `None` if the flight has none.
    #[getter]
    fn apogee_m(&self) -> Option<f64> {
        self.flight.apogee_m()
    }

    /// The time of the apogee after launch, s.
    #[getter]
    fn apogee_time_s(&self) -> Option<f64> {
        self.flight.apogee_time_s()
    }

    /// The top speed, m/s.
    #[getter]
    fn max_speed_m_s(&self) -> Option<f64> {
        self.flight.max_speed_m_s()
    }

    /// The top Mach number.
    #[getter]
    fn max_mach(&self) -> Option<f64> {
        self.flight.max_mach()
    }

    /// The speed leaving the rail, m/s.
    #[getter]
    fn rail_exit_speed_m_s(&self) -> Option<f64> {
        self.flight.rail_exit_speed_m_s()
    }

    /// Where and how it landed, as a dictionary: `time_s`, `latitude_deg`, `longitude_deg`,
    /// `east_m` and `north_m` of the pad, `distance_m`, `ground_hit_speed_m_s` and
    /// `descent_rate_m_s`; `None` if it didn't land.
    #[getter]
    fn landing<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_python(py, &self.flight.landing())
    }

    /// Every metric of the flight, as a dictionary: apogee, peaks, stability margins and
    /// landings, as `hpr_sim::metrics::FlightSummary` writes them in JSON.
    #[getter]
    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        to_python(py, self.flight.summary())
    }

    /// The flight's events in order, each a dictionary: its `kind` (`"liftoff"`, `"rail_exit"`,
    /// `"burnout"`, `"apogee"`, `"deployment"`, `"ground_hit"` and others), the `index` of the
    /// device or motor it is about (`None` for none), its `time_s` after launch, and the
    /// flight's state then, `sample`, as `hpr_sim::Sample` writes it in JSON (`cg_enu_m` is the
    /// centre of gravity east, north and up of the pad, m).
    #[getter]
    fn events<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        self.flight
            .result()
            .events
            .iter()
            .map(|event| {
                let (kind, index) = match serde_json::to_value(event.kind).map_err(error)? {
                    serde_json::Value::String(kind) => (kind, None),
                    serde_json::Value::Object(map) => match map.into_iter().next() {
                        Some((kind, index)) => (kind, index.as_u64()),
                        None => return Err(error("an event with no kind")),
                    },
                    other => return Err(error(format!("an event written as {other}"))),
                };
                let dict = PyDict::new(py);
                dict.set_item("kind", kind)?;
                dict.set_item("index", index)?;
                dict.set_item("time_s", event.sample.time_s)?;
                dict.set_item("sample", to_python(py, &event.sample)?)?;
                Ok(dict)
            })
            .collect()
    }

    /// The recording's column names, each with its unit: `time_s`, `height_above_ground_m`,
    /// `position_east_m`, `airspeed_m_s`, `mach`, `thrust_n`, `mass_kg` and the rest.
    #[getter]
    fn columns(&self) -> Vec<String> {
        self.columns.clone()
    }

    /// The recording, one NumPy array per column, by name.
    #[getter]
    fn series<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let series = PyDict::new(py);
        for (index, name) in self.columns.iter().enumerate() {
            series.set_item(name, self.column(py, index))?;
        }
        Ok(series)
    }

    /// Whether the recording has a column called `name`: `"mach" in flight`.
    fn __contains__(&self, name: &str) -> bool {
        self.columns.iter().any(|column| column == name)
    }

    /// The recording's column names, as `columns`: with `flight[name]`, a flight reads as a
    /// dictionary of arrays.
    fn keys(&self) -> Vec<String> {
        self.columns.clone()
    }

    /// One column of the recording as a NumPy array: `flight["height_above_ground_m"]`.
    fn __getitem__<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let index = self
            .columns
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err(format!(
                    "no column `{name}`; Flight.columns lists them"
                ))
            })?;
        Ok(self.column(py, index))
    }

    /// The flight as JSON text: its result (its events, how it ended, the integrator's work) and
    /// its summary, as the Rust `hpr::Flight` writes them.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.flight).map_err(error)
    }

    fn __repr__(&self) -> String {
        match self.flight.apogee_m() {
            Some(apogee_m) => format!("Flight(apogee_m={apogee_m:.1})"),
            None => "Flight(no apogee)".to_owned(),
        }
    }
}

impl Flight {
    /// Column `index` as a new, read-only NumPy array.
    fn column<'py>(&self, py: Python<'py>, index: usize) -> Bound<'py, PyArray1<f64>> {
        let array = PyArray1::from_slice(py, self.values.get(index).map_or(&[], Vec::as_slice));
        // The recording is the flight's record: the arrays Python gets are read-only copies.
        array.readwrite().make_nonwriteable();
        array
    }
}
