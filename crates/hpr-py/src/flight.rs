//! `Environment` and `Flight`: where a rocket flies, and its flight.

use hpr::hpr_aero::DragTable as RustDragTable;
use hpr::hpr_core::earth::GravityModel;
use hpr::hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr::hpr_sim::{Channel, Recorder};
use numpy::{PyArray1, PyArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::models::{PythonDrag, PythonWind, Raised};
use crate::rocket::Rocket;
use crate::{error, key, to_python};
use pyo3::{PyTraverseError, PyVisit};

/// Where a rocket flies: the launch site, the US Standard Atmosphere 1976, and a wind.
///
/// `Environment(latitude_deg, longitude_deg, elevation_m, *, wind_speed_m_s=0.0,
/// wind_from_deg=0.0, gravity="ellipsoidal")`: a launch site at `latitude_deg` and
/// `longitude_deg` (positive east, so negative in the Americas), `elevation_m` above sea level,
/// with no wind, or a wind of `wind_speed_m_s` blowing **from** `wind_from_deg`, clockwise from
/// true north, the same at every height. Or a wind of your own, as a Python function given as
/// `wind`: called as `wind(height_m)` with a height above sea level, m, it returns the air's
/// velocity `(east_m_s, north_m_s)`, the way the air moves (a west wind is `(+speed, 0)`). An
/// exception it raises stops the flight and is raised by `Flight`.
///
/// `gravity` says how gravity is found along the flight: `"ellipsoidal"`, the default, is the
/// full normal gravity vector at the rocket's position; `"vertical"` is its exact size along the
/// launch site's vertical; and `"vertical_taylor"` is RocketPy's formula, a Taylor series in
/// height along the vertical, for like-for-like comparisons with RocketPy.
#[pyclass(module = "hpr", frozen, skip_from_py_object)]
#[derive(Debug)]
pub struct Environment {
    environment: hpr::Environment,
    /// A Python wind function, bound to each flight's own exception slot as the flight starts.
    wind: Option<Py<PyAny>>,
}

#[pymethods]
impl Environment {
    #[new]
    #[pyo3(signature = (latitude_deg, longitude_deg, elevation_m, *, wind_speed_m_s = 0.0, wind_from_deg = 0.0, gravity = "ellipsoidal", wind = None))]
    fn new(
        latitude_deg: f64,
        longitude_deg: f64,
        elevation_m: f64,
        wind_speed_m_s: f64,
        wind_from_deg: f64,
        gravity: &str,
        wind: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        let mut environment =
            hpr::Environment::new(latitude_deg, longitude_deg, elevation_m).map_err(error)?;
        let gravity = gravity_model(gravity)?;
        if gravity != GravityModel::default() {
            environment = environment.with_gravity(gravity).map_err(error)?;
        }
        if let Some(wind) = &wind {
            callable("wind", wind)?;
            if wind_speed_m_s != 0.0 || wind_from_deg != 0.0 {
                return Err(error(
                    "a wind function and a constant wind were both given; give one",
                ));
            }
        } else if wind_speed_m_s != 0.0 || wind_from_deg != 0.0 {
            environment = environment
                .with_constant_wind(wind_speed_m_s, wind_from_deg)
                .map_err(error)?;
        }
        Ok(Self { environment, wind })
    }

    /// The wind function, for Python's garbage collector: a function that holds this environment
    /// makes a cycle it can then find.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(wind) = &self.wind {
            visit.call(wind)?;
        }
        Ok(())
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

/// Refuses a `what` function that can't be called, as it is given rather than mid-flight.
fn callable(what: &str, function: &Py<PyAny>) -> PyResult<()> {
    Python::attach(|py| {
        if function.bind(py).is_callable() {
            Ok(())
        } else {
            Err(error(format!(
                "`{what}` must be a function, and is not callable"
            )))
        }
    })
}

/// A gravity model by its name: one of the library's `GravityModel`s that takes no number, read
/// through its `serde` name so the two can't drift apart.
fn gravity_model(name: &str) -> PyResult<GravityModel> {
    let refused = || {
        error(format!(
            "no gravity `{name}`: \"ellipsoidal\", \"vertical\" or \"vertical_taylor\""
        ))
    };
    if key(name) == "constant" {
        return Err(refused());
    }
    serde_json::from_value(serde_json::json!({ "kind": key(name) })).map_err(|_| refused())
}

/// A drag table's curve from `(mach, cd)` rows: linear between them, held at the ends.
fn curve(what: &str, rows: &[Vec<f64>]) -> PyResult<Table1D> {
    let table = rows_to_curve(what, rows)?;
    not_negative(what, &table)?;
    Ok(table)
}

/// Refuses a curve with a negative coefficient: drag that would push the rocket along.
fn not_negative(what: &str, curve: &Table1D) -> PyResult<()> {
    match curve.ys().iter().position(|value| *value < 0.0) {
        Some(index) => Err(error(format!(
            "{what}'s row {index} has a drag coefficient of {}, and it can't be negative",
            curve.ys()[index]
        ))),
        None => Ok(()),
    }
}

/// The rows as a curve, before its coefficients are checked.
fn rows_to_curve(what: &str, rows: &[Vec<f64>]) -> PyResult<Table1D> {
    let mut machs = Vec::with_capacity(rows.len());
    let mut coefficients = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let &[mach, coefficient] = row.as_slice() else {
            return Err(error(format!(
                "{what}'s row {index} has {} values, not two: (mach, cd)",
                row.len()
            )));
        };
        machs.push(mach);
        coefficients.push(coefficient);
    }
    Table1D::new(
        machs,
        coefficients,
        Interpolation::Linear,
        Extrapolation::Clamp,
    )
    .map_err(|problem| error(format!("{what}: {problem}")))
}

/// A zero-lift drag coefficient `C_D0` against Mach number, flown in place of hpr's own drag
/// (`Flight(..., drag_table=...)`), as RocketPy's `power_off_drag` and `power_on_drag` are.
///
/// `DragTable(power_off, power_on=None, *, reference_diameter_m=None)`: each curve is a sequence
/// of `(mach, cd)` rows, a list of pairs or a NumPy array of two columns, with Mach numbers that
/// increase. The power-on curve is flown while a motor thrusts, and the power-off curve the rest
/// of the time, or all of it when there is no power-on curve. Between rows the coefficient is
/// interpolated linearly; past the ends it holds the end values. The coefficients are on the
/// rocket's reference area, a circle of its largest body diameter, unless
/// `reference_diameter_m` names another; then they are rescaled by the ratio of the two areas.
/// Only the drag is replaced: the normal force, centre of pressure and damping stay hpr's.
///
/// `DragTable.from_csv(power_off, power_on=None, *, reference_diameter_m=None)` reads each curve
/// from a CSV file of two columns, Mach number and `C_D0`, under an optional header row, as
/// RocketPy's drag files are.
#[pyclass(module = "hpr", frozen, skip_from_py_object)]
#[derive(Debug)]
pub struct DragTable {
    table: RustDragTable,
}

impl DragTable {
    fn with_reference(table: RustDragTable, reference_diameter_m: Option<f64>) -> PyResult<Self> {
        let table = match reference_diameter_m {
            Some(diameter_m) if diameter_m.is_finite() && diameter_m > 0.0 => {
                table.with_reference_diameter_m(diameter_m)
            }
            Some(diameter_m) => {
                return Err(error(format!(
                    "the drag table's reference diameter is {diameter_m} m, and must be finite \
                     and positive"
                )));
            }
            None => table,
        };
        Ok(Self { table })
    }
}

#[pymethods]
impl DragTable {
    #[new]
    #[pyo3(signature = (power_off, power_on = None, *, reference_diameter_m = None))]
    fn new(
        power_off: Vec<Vec<f64>>,
        power_on: Option<Vec<Vec<f64>>>,
        reference_diameter_m: Option<f64>,
    ) -> PyResult<Self> {
        let power_off = curve("power_off", &power_off)?;
        let power_on = power_on.map(|rows| curve("power_on", &rows)).transpose()?;
        Self::with_reference(
            RustDragTable::new(power_off, power_on),
            reference_diameter_m,
        )
    }

    /// A table read from CSV files: `power_off`'s path, and optionally `power_on`'s.
    #[staticmethod]
    #[pyo3(signature = (power_off, power_on = None, *, reference_diameter_m = None))]
    fn from_csv(
        power_off: std::path::PathBuf,
        power_on: Option<std::path::PathBuf>,
        reference_diameter_m: Option<f64>,
    ) -> PyResult<Self> {
        // Each file read and checked alone, so an error names the file.
        let read = |path: &std::path::Path| {
            let named =
                |problem: &dyn std::fmt::Display| error(format!("{}: {problem}", path.display()));
            let text = std::fs::read_to_string(path).map_err(|problem| named(&problem))?;
            let curve = RustDragTable::from_csv(&text, None)
                .map_err(|problem| named(&problem))?
                .power_off;
            not_negative(&path.display().to_string(), &curve)?;
            Ok::<_, PyErr>(curve)
        };
        let off = read(&power_off)?;
        let on = power_on.as_deref().map(read).transpose()?;
        Self::with_reference(RustDragTable::new(off, on), reference_diameter_m)
    }

    /// `C_D0` at `mach`, from the power-on curve when `thrusting` and the table has one, on the
    /// table's own reference diameter.
    #[pyo3(signature = (mach, *, thrusting = false))]
    fn cd0(&self, mach: f64, thrusting: bool) -> PyResult<f64> {
        Ok(self.table.lookup(mach, thrusting).map_err(error)?.value)
    }

    /// Whether the table has a power-on curve.
    #[getter]
    fn has_power_on(&self) -> bool {
        self.table.power_on.is_some()
    }

    /// The diameter the coefficients are on, m; `None` for the rocket's own.
    #[getter]
    fn reference_diameter_m(&self) -> Option<f64> {
        self.table.reference_diameter_m
    }

    fn __repr__(&self) -> String {
        format!(
            "DragTable({} power-off rows{}, reference_diameter_m={})",
            self.table.power_off.xs().len(),
            self.table
                .power_on
                .as_ref()
                .map_or_else(String::new, |on| format!(
                    ", {} power-on rows",
                    on.xs().len()
                )),
            self.table
                .reference_diameter_m
                .map_or_else(|| "None".to_owned(), |diameter_m| diameter_m.to_string())
        )
    }
}

/// A flight: `rocket` launched in `environment` from a rail `rail_length_m` long, flown to the
/// ground as soon as it is made.
///
/// `Flight(rocket, environment, rail_length_m, *, inclination_deg=90.0, heading_deg=0.0,
/// interval_s=None, drag_table=None, drag=None)`: the rail is measured from the rocket's aft end to the rail's top, and leans
/// `inclination_deg` above the horizon (90 is vertical) toward `heading_deg`, clockwise from true
/// north. The flight is recorded every `interval_s` seconds, at least 0.001 s, and at every event;
/// or at every step of the integrator when that is left out. The rocket is copied as the flight
/// starts, and a flight runs to its end: it can't be interrupted. A `DragTable` given as
/// `drag_table` is flown in place of hpr's own drag, and so is a Python function given as `drag`:
/// called as `drag(mach, thrusting)`, with `thrusting` true while a motor burns, it returns the
/// rocket's zero-lift drag coefficient `C_D0` on the rocket's reference area. An exception a
/// function raises, the drag's or the environment's wind's, stops the flight, even at a trial
/// step the integrator would have retried shorter, and is raised here as it was raised.
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
    #[pyo3(signature = (rocket, environment, rail_length_m, *, inclination_deg = 90.0, heading_deg = 0.0, interval_s = None, drag_table = None, drag = None))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of a flight"
    )]
    fn new(
        py: Python<'_>,
        rocket: &Bound<'_, Rocket>,
        environment: &Environment,
        rail_length_m: f64,
        inclination_deg: f64,
        heading_deg: f64,
        interval_s: Option<f64>,
        drag_table: Option<&DragTable>,
        drag: Option<Py<PyAny>>,
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
        // One slot for this flight's functions: its drag and its environment's wind.
        let raised = Raised::default();
        let with_wind;
        let environment = match &environment.wind {
            Some(wind) => {
                with_wind = environment
                    .environment
                    .clone()
                    .with_wind(PythonWind::new(wind.clone_ref(py), raised.clone()));
                &with_wind
            }
            None => &environment.environment,
        };
        let mut builder = hpr::Flight::builder(&rocket, environment, rail_length_m)
            .inclination_deg(inclination_deg)
            .heading_deg(heading_deg);
        if let Some(drag) = &drag {
            callable("drag", drag)?;
        }
        match (drag_table, drag) {
            (Some(_), Some(_)) => {
                return Err(error(
                    "a drag table and a drag function were both given; give one",
                ));
            }
            (Some(drag_table), None) => builder = builder.drag_table(drag_table.table.clone()),
            (None, Some(drag)) => {
                builder = builder.drag_model(PythonDrag::new(drag, raised.clone()))
            }
            (None, None) => {}
        }
        // The flight holds the GIL only while a Python function runs, so other Python threads run
        // while it flies.
        let flown = py.detach(|| builder.fly_with(&mut recorder));
        // A function's exception is raised in place of the library's error, or of a flight the
        // library finished anyway: the exception stopped this flight's functions.
        if let Some(raised) = raised.take() {
            return Err(raised);
        }
        let flight = flown.map_err(error)?;
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
    /// `east_m` and `north_m` of the pad, `distance_m`, `ground_hit_speed_m_s`,
    /// `descent_rate_m_s`, and `body`, the separated body it is about (`None`, the rocket
    /// itself, for a flight with no separation); `None` if it didn't land.
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

    /// The recording's column names, one by one: `for name in flight`.
    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        PyList::new(py, &self.columns)?
            .try_iter()
            .map(Bound::into_any)
    }

    /// The number of columns in the recording.
    fn __len__(&self) -> usize {
        self.columns.len()
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
