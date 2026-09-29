//! Models written in Python: a drag and a wind as Python functions, flown by the Rust library.
//!
//! A flight runs without the GIL (`Python::detach`), so each call of a function takes it back
//! (`Python::attach`) for as long as the call lasts. A function that raises stops the flight: the
//! exception is kept in the flight's [`Raised`] slot, the library sees an error of its own kind,
//! and `Flight` raises the exception in Python as it was raised, with its type and traceback.
//!
//! The integrator retries a step whose evaluation failed with a shorter one, so an error alone
//! would not stop a flight. The slot does: once it holds an exception, every function of the
//! flight answers with an error and no call to Python, so the retries fail at once, and `Flight`
//! raises the exception whether or not the library's flight ended in an error.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use hpr::hpr_aero::{AeroError, DragModel, DragQuery};
use hpr::hpr_atmos::{AtmosError, Wind, WindSample};
use hpr::hpr_core::DVec3;
use pyo3::prelude::*;

/// Where a Python function's exception waits for the flight that called it to end: one slot per
/// flight, shared by its drag and its wind. The first exception is kept; once there is one, the
/// functions are not called again.
#[derive(Debug, Clone, Default)]
pub struct Raised(Arc<Mutex<Option<PyErr>>>);

impl Raised {
    fn holds(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_some()
    }

    fn keep(&self, raised: PyErr) {
        // A poisoned lock means a panic while holding it, which `keep` and `take` can't cause;
        // the slot's value is still whole, so it is used as it is.
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(raised);
        }
    }

    /// The exception a function raised, if any, leaving the slot empty.
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
        // Never shown: `Flight` raises the Python exception in its place.
        let refused =
            || AeroError::Unsupported(format!("the Python drag function raised at Mach {mach}"));
        if self.raised.holds() {
            return Err(refused());
        }
        Python::attach(|py| {
            self.function
                .bind(py)
                .call1((mach, thrusting))
                .and_then(|value| value.extract::<f64>())
                // Kept with the GIL held.
                .map_err(|raised| self.raised.keep(raised))
        })
        .map_err(|()| refused())
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
        // Never shown: `Flight` raises the Python exception in its place.
        let refused = || AtmosError::Domain {
            what: "height (m) at which the Python wind function raised",
            value: height_msl_m,
        };
        if self.raised.holds() {
            return Err(refused());
        }
        // Any sequence of two numbers: a tuple, a list or a NumPy array.
        let [east_m_s, north_m_s] = Python::attach(|py| {
            self.function
                .bind(py)
                .call1((height_msl_m,))
                .and_then(|value| value.extract::<[f64; 2]>())
                // Kept with the GIL held.
                .map_err(|raised| self.raised.keep(raised))
        })
        .map_err(|()| refused())?;
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
