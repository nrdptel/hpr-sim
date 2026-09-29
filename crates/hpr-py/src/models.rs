//! Models written in Python: a drag and a wind as Python functions, flown by the Rust library.
//!
//! A flight runs without the GIL (`Python::detach`), so each call of a function takes it back
//! (`Python::attach`) for as long as the call lasts. A function that raises stops the flight: the
//! library sees an error of its own kind, and the exception itself is kept in a [`Raised`] slot,
//! so `Flight` raises it in Python as it was raised, with its type and traceback.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use hpr::hpr_aero::{AeroError, DragModel, DragQuery};
use hpr::hpr_atmos::{AtmosError, Wind, WindSample};
use hpr::hpr_core::DVec3;
use pyo3::prelude::*;

/// Where a Python function's exception waits for the flight that called it to end. The first
/// exception is kept: it is what stopped the flight, and a later one is its consequence.
#[derive(Debug, Clone, Default)]
pub struct Raised(Arc<Mutex<Option<PyErr>>>);

impl Raised {
    fn keep(&self, raised: PyErr) {
        // A poisoned lock means a panic while holding it, which `keep` and `take` can't cause;
        // the slot's value is still whole, so it is used as it is.
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(raised);
        }
    }

    /// The exception a function raised since the last `take`, if any, leaving the slot empty.
    pub fn take(&self) -> Option<PyErr> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

/// A Python function as a [`DragModel`]: called as `drag(mach, thrusting)`, it returns the
/// rocket's zero-lift drag coefficient `C_D0` on the rocket's reference area.
pub struct PythonDrag {
    function: Py<PyAny>,
    raised: Raised,
}

impl PythonDrag {
    pub fn new(function: Py<PyAny>, raised: Raised) -> Self {
        Self { function, raised }
    }
}

impl fmt::Debug for PythonDrag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PythonDrag")
    }
}

impl DragModel for PythonDrag {
    fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
        let (mach, thrusting) = (query.mach(), query.conditions().thrusting);
        Python::attach(|py| {
            self.function
                .bind(py)
                .call1((mach, thrusting))
                .and_then(|value| value.extract::<f64>())
        })
        .map_err(|raised| {
            self.raised.keep(raised);
            // Never shown: `Flight` raises the Python exception in its place.
            AeroError::Unsupported(format!("the Python drag function raised at Mach {mach}"))
        })
    }
}

/// A Python function as a [`Wind`]: called as `wind(height_m)`, with the height above mean sea
/// level, it returns the air's velocity `(east_m_s, north_m_s)`.
pub struct PythonWind {
    function: Py<PyAny>,
    raised: Raised,
}

impl PythonWind {
    pub fn new(function: Py<PyAny>, raised: Raised) -> Self {
        Self { function, raised }
    }
}

impl fmt::Debug for PythonWind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PythonWind")
    }
}

impl Wind for PythonWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        let (east_m_s, north_m_s) = Python::attach(|py| {
            self.function
                .bind(py)
                .call1((height_msl_m,))
                .and_then(|value| value.extract::<(f64, f64)>())
        })
        .map_err(|raised| {
            self.raised.keep(raised);
            // Never shown: `Flight` raises the Python exception in its place.
            AtmosError::Domain {
                what: "height (m) at which the Python wind function raised",
                value: height_msl_m,
            }
        })?;
        for (what, value) in [
            ("wind from a Python function, east (m/s)", east_m_s),
            ("wind from a Python function, north (m/s)", north_m_s),
        ] {
            if !value.is_finite() {
                return Err(AtmosError::Domain { what, value });
            }
        }
        Ok(WindSample {
            velocity_enu_m_s: DVec3::new(east_m_s, north_m_s, 0.0),
            extrapolated: None,
        })
    }
}
