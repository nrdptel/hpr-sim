//! Python bindings for hpr-sim: the `hpr` Python package.
//!
//! **Guide:** [Python][guide-python] shows the package in use, and [Start here][guide-start]
//! says what works today and how far to trust it.
//!
//! [guide-python]: https://nrdptel.github.io/hpr-sim/python.html
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//!
//! The package wraps the [`hpr`] builder, the same four types in the same order:
//!
//! - `Environment`: the launch site, the standard atmosphere, a constant wind and a gravity model.
//! - `Motor`: a motor from the built-in catalog, or a RASP `.eng` or RockSim `.rse` file.
//! - `Rocket`: parts added from the nose back, the motor and the parachutes; or a design read
//!   from a `.hpr`, `.hprz`, `.ork` or rocket JSON file.
//! - `Flight`: the rocket flown from a rail as soon as it is made, with its metrics, its events
//!   and its recording as NumPy arrays; with a `DragTable`, another tool's drag in place of hpr's.
//!
//! **How far to trust it:** the bindings add no physics. Each call hands its arguments to the
//! [`hpr`] builder and returns what it returns, so a flight made in Python runs the same code as
//! the one the Rust builder makes from the same numbers; the package's tests fly the builder's
//! example rocket and match every digit the Rust example prints (the last digits can differ
//! between a release and a debug build). The guide's [Accuracy][guide-accuracy] page says how
//! good the models themselves are.
//!
//! [guide-accuracy]: https://nrdptel.github.io/hpr-sim/accuracy.html
//!
//! Values cross into Python in SI units, named in every argument and attribute (`length_m`,
//! `apogee_m`, `max_speed_m_s`), as in the Rust API. Records such as a flight's summary cross as
//! dictionaries, made from the same `serde` form the Rust types write as JSON. Errors raise
//! `hpr.HprError`, a `ValueError`, whose message is the Rust error's.
//!
//! The crate is built by maturin (`crates/hpr-py/pyproject.toml`) into one wheel per operating
//! system, on CPython's stable ABI (abi3), so it serves CPython 3.10 and later. Its tests are Python's, in
//! `crates/hpr-py/tests/`, run by pytest on the built wheel.

#![allow(
    clippy::disallowed_methods,
    reason = "the bindings are an I/O boundary, as the command line is: `Motor.from_file` and \
              `Rocket.from_file` read the files Python names"
)]

mod flight;
mod rocket;

use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

create_exception!(
    hpr,
    HprError,
    PyValueError,
    "An error from hpr-sim: an input out of its domain, a part out of order, a motor or \
     material that isn't there, or a flight that failed. The message is the library's."
);

/// The Python exception for a library error.
fn error(error: impl std::fmt::Display) -> PyErr {
    HprError::new_err(error.to_string())
}

/// A value the library serializes, as the Python object `json.loads` makes of its JSON: numbers,
/// strings, lists, dictionaries and `None`.
fn to_python<'py>(py: Python<'py>, value: &impl Serialize) -> PyResult<Bound<'py, PyAny>> {
    let text = serde_json::to_string(value).map_err(error)?;
    py.import("json")?.call_method1("loads", (text,))
}

/// A name as the bindings compare it: trimmed, in lower case, with hyphens and spaces as
/// underscores, so `"Flat circular"` and `"flat-circular"` are `"flat_circular"`.
fn key(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace(['-', ' '], "_")
}

/// A unit enum of the library, by its `serde` name: `"flat_circular"` is
/// `CanopyType::FlatCircular`. The names are the ones the library writes in JSON, so the two
/// can't drift apart.
fn by_name<T: DeserializeOwned>(what: &str, name: &str) -> PyResult<T> {
    serde_json::from_value(serde_json::Value::String(key(name)))
        .map_err(|_| error(format!("no {what} `{name}`")))
}

/// The built-in materials' ids, the names `Rocket`'s parts take: `"abs"`, `"kraft_phenolic"`,
/// `"birch_plywood"` and the rest, each with its density's source in the Rust documentation of
/// `hpr_design::materials`.
#[pyfunction]
fn materials() -> Vec<&'static str> {
    hpr::hpr_design::materials::BUILTIN
        .iter()
        .map(|material| material.id)
        .collect()
}

/// The `hpr` package's native module. `hpr/__init__.py` re-exports it.
#[pymodule]
fn _hpr(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("HprError", m.py().get_type::<HprError>())?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(materials, m)?)?;
    m.add_class::<flight::Environment>()?;
    m.add_class::<rocket::Motor>()?;
    m.add_class::<rocket::Rocket>()?;
    m.add_class::<flight::DragTable>()?;
    m.add_class::<flight::Flight>()?;
    Ok(())
}
