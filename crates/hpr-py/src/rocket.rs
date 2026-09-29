//! `Motor` and `Rocket`: the builder's motor and rocket, part by part.

use std::path::Path;

use hpr::rocket::{Fins, Mass, MotorTube, Nose, Transition, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, FinCrossSection, FinPlanform, NoseShape, Position, Trigger,
};
use numpy::{PyArray1, ToPyArray};
use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::{by_name, error, key, to_python};

/// A solid rocket motor: its thrust curve, masses and size, and its ejection delay.
#[pyclass(module = "hpr", frozen, from_py_object)]
#[derive(Debug, Clone)]
pub struct Motor {
    pub(crate) motor: hpr::Motor,
}

impl Motor {
    fn delayed(motor: hpr::Motor, delay_s: Option<f64>) -> PyResult<Self> {
        let motor = match delay_s {
            Some(delay_s) => motor.with_delay_s(delay_s).map_err(error)?,
            None => motor,
        };
        Ok(Self { motor })
    }
}

#[pymethods]
impl Motor {
    /// The motor in hpr-sim's built-in catalog whose designation or common name is `name`
    /// (`"H54"`, `"168H54-10A"`), ignoring case, spaces and hyphens, with its first bundled
    /// thrust curve; `delay_s` sets its ejection delay. The designation's own delay is not used.
    #[staticmethod]
    #[pyo3(signature = (name, *, delay_s = None))]
    fn from_catalog(name: &str, delay_s: Option<f64>) -> PyResult<Self> {
        Self::delayed(hpr::Motor::from_catalog(name).map_err(error)?, delay_s)
    }

    /// The one motor in a RASP `.eng` file's text.
    #[staticmethod]
    #[pyo3(signature = (text, *, delay_s = None))]
    fn from_eng(text: &str, delay_s: Option<f64>) -> PyResult<Self> {
        Self::delayed(hpr::Motor::from_eng(text).map_err(error)?, delay_s)
    }

    /// The one motor in a RockSim `.rse` file's text.
    #[staticmethod]
    #[pyo3(signature = (text, *, delay_s = None))]
    fn from_rse(text: &str, delay_s: Option<f64>) -> PyResult<Self> {
        Self::delayed(hpr::Motor::from_rse(text).map_err(error)?, delay_s)
    }

    /// The one motor in a `.eng` or `.rse` file, by its extension.
    #[staticmethod]
    #[pyo3(signature = (path, *, delay_s = None))]
    fn from_file(path: std::path::PathBuf, delay_s: Option<f64>) -> PyResult<Self> {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| error(format!("{}: {e}", path.display())))?;
        match extension(&path).as_deref() {
            Some("eng") => Self::from_eng(&text, delay_s),
            Some("rse") => Self::from_rse(&text, delay_s),
            _ => Err(error(format!(
                "{}: a motor file is a RASP .eng or a RockSim .rse",
                path.display()
            ))),
        }
    }

    /// The designation, such as `"H54"`.
    #[getter]
    fn designation(&self) -> &str {
        self.motor.designation()
    }

    /// The case diameter, m.
    #[getter]
    fn diameter_m(&self) -> f64 {
        self.motor.diameter_m()
    }

    /// The case length, m.
    #[getter]
    fn length_m(&self) -> f64 {
        self.motor.length_m()
    }

    /// The ejection delay, s, where it is a number of seconds; `None` for none, or a plugged
    /// motor.
    #[getter]
    fn delay_s(&self) -> Option<f64> {
        match self.motor.delay() {
            Some(hpr::hpr_motor::Delay::Seconds(delay_s)) => Some(delay_s),
            _ => None,
        }
    }

    /// The total impulse, N·s.
    #[getter]
    fn total_impulse_ns(&self) -> f64 {
        self.motor.solid_motor().curve().total_impulse_ns()
    }

    /// The propellant's mass at ignition, kg.
    #[getter]
    fn propellant_mass_kg(&self) -> f64 {
        self.motor.solid_motor().propellant_initial_mass_kg()
    }

    /// The thrust curve's points: two NumPy arrays, the times after ignition, s, and the
    /// thrusts, N.
    fn thrust_curve<'py>(
        &self,
        py: Python<'py>,
    ) -> (Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>) {
        let curve = self.motor.solid_motor().curve();
        (
            curve.times_s().to_pyarray(py),
            curve.thrusts_n().to_pyarray(py),
        )
    }

    fn __repr__(&self) -> String {
        format!("Motor({:?})", self.motor.designation())
    }
}

/// A rocket: built part by part from the nose back, or read from a design file.
///
/// `Rocket(name, diameter_m)`: a rocket called `name`, `diameter_m` across outside, its first
/// tube's diameter and its nose's.
///
/// Parts go on in order: the nose, then tubes and transitions; fins, a motor tube and masses go
/// on or in the tube before them. Every part names its material by a built-in id
/// (`hpr.materials()`). Each method returns the rocket, so calls can be chained.
#[pyclass(module = "hpr", from_py_object)]
#[derive(Debug, Clone)]
pub struct Rocket {
    pub(crate) rocket: hpr::Rocket,
    /// What reading its design file said: warnings, and what isn't flown.
    notes: Vec<String>,
}

#[pymethods]
impl Rocket {
    #[new]
    fn new(name: &str, diameter_m: f64) -> PyResult<Self> {
        Ok(Self {
            rocket: hpr::Rocket::new(name, diameter_m).map_err(error)?,
            notes: Vec::new(),
        })
    }

    /// A rocket read from a design file, flown in its motor configuration `configuration`, by
    /// id: an hpr design (`.hpr` or `.hprz`), an OpenRocket `.ork` file, or a rocket's JSON
    /// (`.json`, an `hpr_design::Rocket`). Left out, the configuration is the design's only one.
    /// Parts can't be added to it; parachutes can.
    #[staticmethod]
    #[pyo3(signature = (path, configuration = None))]
    fn from_file(path: std::path::PathBuf, configuration: Option<&str>) -> PyResult<Self> {
        let (design, motors, notes) = read_design(&path)?;
        let configuration = match &motors {
            Some(motors) => ork_configuration(&path, motors, configuration)?,
            None => match configuration {
                Some(id) => id.to_owned(),
                None => match design.configurations.as_slice() {
                    [only] => only.id.clone(),
                    many => {
                        let ids: Vec<&str> = many.iter().map(|c| c.id.as_str()).collect();
                        return Err(error(format!(
                            "{}: name one of the design's {} configurations: {}",
                            path.display(),
                            ids.len(),
                            ids.join(", ")
                        )));
                    }
                },
            },
        };
        Ok(Self {
            rocket: hpr::Rocket::from_design(design, &configuration).map_err(error)?,
            notes,
        })
    }

    /// Adds the nose cone, which goes first. `shape` is `"conical"`, `"ogive"` (`parameter`, the
    /// ogive's radius ratio, 1 for a tangent ogive, by default), `"elliptical"`,
    /// `"power_series"` (`parameter`, its exponent), `"parabolic_series"` (`parameter`, `K`) or
    /// `"haack"` (`parameter`, `C`: 0, the default, is the von Kármán). A `wall_m` makes it
    /// hollow, and leaving it out solid; a `shoulder_length_m` and `shoulder_wall_m` add a
    /// shoulder, capped at its aft end with `capped_shoulder`.
    #[pyo3(signature = (
        shape, length_m, material, *, wall_m = None, parameter = None,
        shoulder_length_m = None, shoulder_wall_m = None, capped_shoulder = false, name = None
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of the builder's nose"
    )]
    fn add_nose<'py>(
        mut slf: PyRefMut<'py, Self>,
        shape: &str,
        length_m: f64,
        material: &str,
        wall_m: Option<f64>,
        parameter: Option<f64>,
        shoulder_length_m: Option<f64>,
        shoulder_wall_m: Option<f64>,
        capped_shoulder: bool,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let shape = nose_shape(shape, parameter)?;
        let material = material_of(material)?;
        let mut nose = match wall_m {
            Some(wall_m) => Nose::hollow(shape, length_m, wall_m, material),
            None => Nose::solid(shape, length_m, material),
        };
        nose = match (shoulder_length_m, shoulder_wall_m) {
            (Some(length_m), Some(wall_m)) if capped_shoulder => {
                nose.with_capped_shoulder(length_m, wall_m)
            }
            (Some(length_m), Some(wall_m)) => nose.with_shoulder(length_m, wall_m),
            (None, None) if !capped_shoulder => nose,
            _ => {
                return Err(error(
                    "a shoulder needs both shoulder_length_m and shoulder_wall_m",
                ));
            }
        };
        if let Some(name) = name {
            nose = nose.named(name);
        }
        slf.rocket.add_nose(nose).map_err(error)?;
        Ok(slf)
    }

    /// Adds a body tube `length_m` long with a wall `wall_m` thick, behind the part before it:
    /// the rocket's diameter across, or `diameter_m`.
    #[pyo3(signature = (length_m, wall_m, material, *, diameter_m = None, name = None))]
    fn add_tube<'py>(
        mut slf: PyRefMut<'py, Self>,
        length_m: f64,
        wall_m: f64,
        material: &str,
        diameter_m: Option<f64>,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut tube = Tube::new(length_m, wall_m, material_of(material)?);
        if let Some(diameter_m) = diameter_m {
            tube = tube.with_diameter_m(diameter_m);
        }
        if let Some(name) = name {
            tube = tube.named(name);
        }
        slf.rocket.add_tube(tube).map_err(error)?;
        Ok(slf)
    }

    /// Adds a transition `length_m` long from the part before it to `aft_diameter_m`: a
    /// shoulder or a boattail. It is conical unless `shape` names one of the nose's shapes, and
    /// hollow with `wall_m` unless `solid`.
    #[pyo3(signature = (
        length_m, aft_diameter_m, wall_m, material, *, shape = None, parameter = None,
        solid = false, name = None
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of the builder's transition"
    )]
    fn add_transition<'py>(
        mut slf: PyRefMut<'py, Self>,
        length_m: f64,
        aft_diameter_m: f64,
        wall_m: f64,
        material: &str,
        shape: Option<&str>,
        parameter: Option<f64>,
        solid: bool,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut transition =
            Transition::conical(length_m, aft_diameter_m, wall_m, material_of(material)?);
        match shape {
            Some(shape) => transition = transition.with_shape(nose_shape(shape, parameter)?),
            None if parameter.is_some() => {
                return Err(error("a transition's parameter needs its shape"));
            }
            None => {}
        }
        if solid {
            transition = transition.solid();
        }
        if let Some(name) = name {
            transition = transition.named(name);
        }
        slf.rocket.add_transition(transition).map_err(error)?;
        Ok(slf)
    }

    /// Adds `count` trapezoidal fins `thickness_m` thick to the tube before them: a root chord,
    /// tip chord and span, and the sweep, how far aft of the root's leading edge the tip's is.
    /// Every argument after `count` is named, so two lengths can't swap unseen.
    /// `cross_section` is `"square"`, the default, `"rounded"` or `"airfoil"`. They sit flush
    /// with the tube's aft end unless `position` (`"top"`, `"middle"`, `"bottom"` or `"after"`)
    /// and `offset_m`, aft of it, place them.
    #[pyo3(signature = (
        count, *, root_chord_m, tip_chord_m, span_m, sweep_m, thickness_m, material,
        cross_section = "square", cant_deg = 0.0, position = None, offset_m = 0.0, name = None
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of the builder's fins"
    )]
    fn add_fins<'py>(
        mut slf: PyRefMut<'py, Self>,
        count: u32,
        root_chord_m: f64,
        tip_chord_m: f64,
        span_m: f64,
        sweep_m: f64,
        thickness_m: f64,
        material: &str,
        cross_section: &str,
        cant_deg: f64,
        position: Option<&str>,
        offset_m: f64,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let planform = FinPlanform::Trapezoidal {
            root_chord_m,
            tip_chord_m,
            span_m,
            sweep_m,
        };
        let cross_section: FinCrossSection = by_name("fin cross-section", cross_section)?;
        let mut fins = Fins::new(count, planform, thickness_m, material_of(material)?)
            .with_cross_section(cross_section)
            .with_cant_deg(cant_deg);
        if let Some(position) = placed(position, offset_m)? {
            fins = fins.at(position);
        }
        if let Some(name) = name {
            fins = fins.named(name);
        }
        slf.rocket.add_fins(fins).map_err(error)?;
        Ok(slf)
    }

    /// Adds a motor tube `length_m` long with an `inner_diameter_m` bore and a wall `wall_m`
    /// thick, in the tube before it: flush with that tube's aft end unless `position` and
    /// `offset_m` place it. The motor's nozzle sticks out `overhang_m` past its aft end.
    #[pyo3(signature = (
        length_m, inner_diameter_m, wall_m, material, *, overhang_m = 0.0, position = None,
        offset_m = 0.0, name = None
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of the builder's motor tube"
    )]
    fn add_motor_tube<'py>(
        mut slf: PyRefMut<'py, Self>,
        length_m: f64,
        inner_diameter_m: f64,
        wall_m: f64,
        material: &str,
        overhang_m: f64,
        position: Option<&str>,
        offset_m: f64,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut tube = MotorTube::new(length_m, inner_diameter_m, wall_m, material_of(material)?)
            .with_overhang_m(overhang_m);
        if let Some(position) = placed(position, offset_m)? {
            tube = tube.at(position);
        }
        if let Some(name) = name {
            tube = tube.named(name);
        }
        slf.rocket.add_motor_tube(tube).map_err(error)?;
        Ok(slf)
    }

    /// Adds a mass of `mass_kg` in the tube before it, placed by `position` (`"top"`, the
    /// default, `"middle"`, `"bottom"` or `"after"`) and `offset_m` aft of there: a point, or
    /// packed in a cylinder `packed_length_m` long and `packed_diameter_m` across.
    #[pyo3(signature = (
        mass_kg, *, position = "top", offset_m = 0.0, packed_length_m = None,
        packed_diameter_m = None, name = None
    ))]
    fn add_mass<'py>(
        mut slf: PyRefMut<'py, Self>,
        mass_kg: f64,
        position: &str,
        offset_m: f64,
        packed_length_m: Option<f64>,
        packed_diameter_m: Option<f64>,
        name: Option<&str>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let position =
            placed(Some(position), offset_m)?.ok_or_else(|| error("a mass needs a position"))?;
        let mut mass = Mass::new(mass_kg, position);
        mass = match (packed_length_m, packed_diameter_m) {
            (Some(length_m), Some(diameter_m)) => mass.packed(length_m, diameter_m),
            (None, None) => mass,
            _ => {
                return Err(error(
                    "a packed mass needs both packed_length_m and packed_diameter_m",
                ));
            }
        };
        if let Some(name) = name {
            mass = mass.named(name);
        }
        slf.rocket.add_mass(mass).map_err(error)?;
        Ok(slf)
    }

    /// Puts the motor in the motor tube.
    fn set_motor<'py>(
        mut slf: PyRefMut<'py, Self>,
        motor: &Motor,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.rocket.set_motor(motor.motor.clone()).map_err(error)?;
        Ok(slf)
    }

    /// Adds a parachute called `name`: a canopy `diameter_m` across (its `canopy` type's drag
    /// coefficient, `"flat_circular"` by default, unless `drag_coefficient` gives one), or a
    /// drag area `cd_s_m2`, m², as RocketPy's `cd_s`. It opens when `trigger` fires:
    /// `"apogee"`; `"altitude"`, on the way down past `altitude_m` above the pad; `"time"`, at
    /// `time_s` after launch; or `"motor_delay"`, at the ejection delay of the motor numbered
    /// `motor` (0, the first, by default), which needs a delay in seconds. It is fully open
    /// `lag_s` seconds later. With `released_by`, the number of another parachute (0 for the
    /// first added, 1 for the second, and so on), it is cut away once that one is fully open, as
    /// a drogue is when the main opens; RocketPy flies only its last parachute to open, which is
    /// the same. An argument that doesn't apply to the choices made is refused, and a
    /// `released_by` that names no other parachute is refused when the rocket flies.
    #[pyo3(signature = (
        name, *, diameter_m = None, canopy = None, drag_coefficient = None,
        cd_s_m2 = None, trigger = "apogee", altitude_m = None, time_s = None, motor = None,
        lag_s = 0.0, released_by = None
    ))]
    #[expect(
        clippy::too_many_arguments,
        reason = "Python's keyword arguments, one per option of a recovery device"
    )]
    fn add_parachute<'py>(
        mut slf: PyRefMut<'py, Self>,
        name: &str,
        diameter_m: Option<f64>,
        canopy: Option<&str>,
        drag_coefficient: Option<f64>,
        cd_s_m2: Option<f64>,
        trigger: &str,
        altitude_m: Option<f64>,
        time_s: Option<f64>,
        motor: Option<usize>,
        lag_s: f64,
        released_by: Option<usize>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let drag = match (diameter_m, cd_s_m2) {
            (Some(nominal_diameter_m), None) => {
                let kind: Option<CanopyType> = match (canopy, drag_coefficient) {
                    (Some(canopy), _) => Some(by_name("canopy type", canopy)?),
                    (None, Some(_)) => None,
                    (None, None) => Some(CanopyType::FlatCircular),
                };
                DeviceDrag::Canopy {
                    nominal_diameter_m,
                    drag_coefficient: match (drag_coefficient, kind) {
                        (Some(drag_coefficient), _) => drag_coefficient,
                        (None, Some(kind)) => kind.drag_coefficient(),
                        (None, None) => CanopyType::FlatCircular.drag_coefficient(),
                    },
                    kind,
                }
            }
            (None, Some(cd_s_m2)) if drag_coefficient.is_none() && canopy.is_none() => {
                DeviceDrag::DragArea { cd_s_m2 }
            }
            _ => {
                return Err(error(
                    "a parachute takes diameter_m (and optionally canopy and drag_coefficient) or \
                     cd_s_m2 alone",
                ));
            }
        };
        let trigger = match (key(trigger).as_str(), altitude_m, time_s, motor) {
            ("apogee", None, None, None) => Trigger::Apogee,
            ("altitude", Some(height_above_ground_m), None, None) => Trigger::Altitude {
                height_above_ground_m,
            },
            ("time", None, Some(time_s), None) => Trigger::Time { time_s },
            ("motor_delay", None, None, motor) => Trigger::MotorDelay {
                motor: motor.unwrap_or(0),
            },
            _ => {
                return Err(error(format!(
                    "trigger `{trigger}`: \"apogee\", \"altitude\" with altitude_m, \"time\" \
                     with time_s, or \"motor_delay\" with an optional motor, and nothing else"
                )));
            }
        };
        let mut device = Device::new(name, drag, trigger).with_lag_s(lag_s);
        if let Some(index) = released_by {
            device = device.with_release_by(index);
        }
        slf.rocket.add_parachute(device);
        Ok(slf)
    }

    /// The rocket's name.
    #[getter]
    fn name(&self) -> String {
        self.rocket.design().name.clone()
    }

    /// The motor configuration flown, by id; `None` before a motor is set.
    #[getter]
    fn configuration(&self) -> Option<&str> {
        self.rocket.configuration_id()
    }

    /// The mass, centre of gravity and inertia `time_s` after ignition: a dictionary of
    /// `mass_kg`, `cg_m` (x, y, z in the body frame, where a point `s` metres aft of the nose
    /// tip is at `z = -s`) and `inertia_kg_m2` (the 3×3 tensor about the centre of gravity, a
    /// list of its rows).
    #[pyo3(signature = (time_s = 0.0))]
    fn mass_properties<'py>(&self, py: Python<'py>, time_s: f64) -> PyResult<Bound<'py, PyDict>> {
        let properties = self.rocket.mass_properties(time_s).map_err(error)?;
        let inertia = properties.inertia_kg_m2;
        let rows: Vec<[f64; 3]> = (0..3).map(|i| inertia.row(i).to_array()).collect();
        let dict = PyDict::new(py);
        dict.set_item("mass_kg", properties.mass_kg)?;
        dict.set_item("cg_m", properties.cg_m.to_array())?;
        dict.set_item("inertia_kg_m2", rows)?;
        Ok(dict)
    }

    /// What reading the design file said, one line each: the `.ork` reader's warnings, an older
    /// format version migrated, an airframe not flown as written, and the recovery devices and
    /// stage separations the file holds, which a rocket read from a file doesn't fly. Empty for a
    /// rocket built part by part.
    #[getter]
    fn notes(&self) -> Vec<String> {
        self.notes.clone()
    }

    /// The static stability margin, calibres, `time_s` after ignition at `mach`, with the air
    /// along the axis; `None` where the rocket has no normal force to find a centre of pressure
    /// from.
    #[pyo3(signature = (time_s = 0.0, mach = 0.3))]
    fn static_margin_cal(&self, time_s: f64, mach: f64) -> PyResult<Option<f64>> {
        self.rocket.static_margin_cal(time_s, mach).map_err(error)
    }

    /// The whole stability picture `time_s` after ignition at `mach`: the margin, the centre of
    /// pressure (`cp_station_m`, metres aft of the nose tip) and the slopes they come from, as a
    /// dictionary.
    #[pyo3(signature = (time_s = 0.0, mach = 0.3))]
    fn margin<'py>(&self, py: Python<'py>, time_s: f64, mach: f64) -> PyResult<Bound<'py, PyAny>> {
        to_python(py, &self.rocket.margin(time_s, mach).map_err(error)?)
    }

    /// The design as JSON text: the `hpr_design::Rocket` a rocket's JSON file holds.
    fn design_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(self.rocket.design()).map_err(error)
    }

    fn __repr__(&self) -> String {
        format!("Rocket({:?})", self.rocket.design().name)
    }
}

/// A file's extension, in lower case.
fn extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
}

/// The rocket a design file holds, read as `hpr sim` reads it, with what the reading said: the
/// `.ork` reader's warnings, an older format version migrated, an airframe not flown as written, and
/// the recovery devices and stage separations the file holds, which a rocket read from a file
/// doesn't fly.
fn read_design(
    path: &Path,
) -> PyResult<(
    hpr::hpr_design::Rocket,
    Option<hpr::hpr_io::ork::Motors>,
    Vec<String>,
)> {
    let failed = |e: &dyn std::fmt::Display| error(format!("{}: {e}", path.display()));
    let bytes = std::fs::read(path).map_err(|e| failed(&e))?;
    let mut notes = Vec::new();
    let design = match extension(path).as_deref() {
        Some("ork") => {
            let file = hpr::hpr_io::ork::read(&bytes).map_err(|e| failed(&e))?;
            let design = hpr::hpr_io::ork::design(&file.value);
            notes.extend(
                file.warnings
                    .iter()
                    .chain(&design.warnings)
                    .map(|warning| format!("{}: {}", warning.at, warning.message)),
            );
            if let Some(why) = hpr::hpr_io::ork::airframe_not_as_written(&file.value) {
                notes.push(format!("the airframe is not flown as written: {why}"));
            }
            design.value
        }
        Some(kind @ ("hpr" | "hprz")) => {
            let (document, written_as) = if kind == "hpr" {
                let text = String::from_utf8(bytes).map_err(|e| failed(&e))?;
                let opened = hpr::hpr_format::read_json(&text).map_err(|e| failed(&e))?;
                (opened.value, opened.written_as)
            } else {
                let opened = hpr::hpr_format::container::read(&bytes).map_err(|e| failed(&e))?;
                let attachments = opened.value.attachments.len();
                if attachments > 0 {
                    notes.push(format!(
                        "the container's {attachments} attachment(s) are not read"
                    ));
                }
                (opened.value.design, opened.written_as)
            };
            if written_as != hpr::hpr_format::VERSION {
                notes.push(format!(
                    "the design is version {written_as} of the hpr design format, read as version \
                     {}",
                    hpr::hpr_format::VERSION
                ));
            }
            if let Some(why) = document
                .provenance
                .source
                .as_ref()
                .and_then(|source| source.airframe_not_as_written.clone())
            {
                notes.push(format!("the airframe is not flown as written: {why}"));
            }
            document.design()
        }
        Some("json") => {
            let rocket: hpr::hpr_design::Rocket =
                serde_json::from_slice(&bytes).map_err(|e| failed(&e))?;
            stack_note(&rocket, &mut notes);
            return Ok((rocket, None, notes));
        }
        _ => {
            return Err(error(format!(
                "{}: a design is an hpr design (.hpr, .hprz), an OpenRocket .ork file or a \
                 rocket's JSON (.json)",
                path.display()
            )));
        }
    };
    let devices = design.recovery.devices.len();
    if devices > 0 {
        notes.push(format!(
            "the file's {devices} recovery device(s) are not flown; the parachutes added with \
             add_parachute are"
        ));
    }
    stack_note(&design.rocket, &mut notes);
    Ok((design.rocket, Some(design.motors), notes))
}

/// The motor configuration of a `.ork` file, or of an hpr design read from one, that a rocket
/// read from it flies, chosen as `hpr sim` chooses: the one named, or the file's default, or its
/// only one. One that stages under power, or that hpr can't fly as written, is refused with why.
fn ork_configuration(
    path: &Path,
    motors: &hpr::hpr_io::ork::Motors,
    wanted: Option<&str>,
) -> PyResult<String> {
    let configurations = &motors.configurations;
    let ids = || {
        configurations
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let chosen = match wanted {
        Some(id) => configurations.iter().find(|c| c.id == id).ok_or_else(|| {
            error(format!(
                "{}: the file has no motor configuration `{id}`; its configurations: {}",
                path.display(),
                ids()
            ))
        })?,
        None => match (motors.default_configuration(), configurations.as_slice()) {
            (Some(chosen), _) | (None, [chosen]) => chosen,
            (None, []) => {
                return Err(error(format!(
                    "{}: the file has no motor configuration",
                    path.display()
                )));
            }
            (None, _) => {
                return Err(error(format!(
                    "{}: name one of the file's {} motor configurations: {}",
                    path.display(),
                    configurations.len(),
                    ids()
                )));
            }
        },
    };
    if let Some(staging) = &chosen.staging {
        return Err(error(format!(
            "{}: configuration {} separates under power: stage {} drops away at {:.3} s, which \
             the Python package doesn't fly yet; the Rust library does, as its example \
             ork_two_stage shows",
            path.display(),
            chosen.id,
            staging.after_stage + 1,
            staging.time_s
        )));
    }
    if let Some(left_out) = &chosen.left_out {
        return Err(error(format!(
            "{}: configuration {} doesn't fly as written: {}",
            path.display(),
            chosen.id,
            left_out.message
        )));
    }
    Ok(chosen.id.clone())
}

/// The note that a design of several stages flies as one stack.
fn stack_note(rocket: &hpr::hpr_design::Rocket, notes: &mut Vec<String>) {
    if rocket.stages.len() > 1 {
        notes.push(
            "the stages fly as one stack: no stage drops away, a motor lit by another's burnout \
             lights on the whole stack, and one lit by a separation never lights"
                .to_owned(),
        );
    }
}

/// A built-in material, by id.
fn material_of(id: &str) -> PyResult<hpr::hpr_design::Material> {
    material(id).map_err(error)
}

/// A nose or transition shape by name, with its parameter where it takes one.
fn nose_shape(shape: &str, parameter: Option<f64>) -> PyResult<NoseShape> {
    let name = key(shape);
    match (name.as_str(), parameter) {
        ("conical", None) => Ok(NoseShape::Conical {}),
        ("elliptical", None) => Ok(NoseShape::Elliptical {}),
        ("ogive", radius_ratio) => Ok(NoseShape::Ogive {
            radius_ratio: radius_ratio.unwrap_or(1.0),
        }),
        ("haack", parameter) => Ok(NoseShape::Haack {
            parameter: parameter.unwrap_or(0.0),
        }),
        ("power_series", Some(exponent)) => Ok(NoseShape::PowerSeries { exponent }),
        ("parabolic_series", Some(parameter)) => Ok(NoseShape::ParabolicSeries { parameter }),
        ("conical" | "elliptical", Some(_)) => {
            Err(error(format!("a {name} shape takes no parameter")))
        }
        ("power_series" | "parabolic_series", None) => {
            Err(error(format!("a {name} shape needs its parameter")))
        }
        _ => Err(error(format!(
            "no nose shape `{shape}`: \"conical\", \"ogive\", \"elliptical\", \"power_series\", \
             \"parabolic_series\" or \"haack\""
        ))),
    }
}

/// A position in the part before, by name, `offset_m` aft of it; `None` for none given, when
/// the offset must be zero.
fn placed(position: Option<&str>, offset_m: f64) -> PyResult<Option<Position>> {
    let aft_offset_m = offset_m;
    Ok(Some(match position.map(key).as_deref() {
        None if offset_m == 0.0 => return Ok(None),
        None => return Err(error("an offset_m needs its position")),
        Some("top") => Position::Top { aft_offset_m },
        Some("middle") => Position::Middle { aft_offset_m },
        Some("bottom") => Position::Bottom { aft_offset_m },
        Some("after") => Position::After { aft_offset_m },
        Some(_) => {
            return Err(error(format!(
                "no position `{}`: \"top\", \"middle\", \"bottom\" or \"after\"",
                position.unwrap_or_default()
            )));
        }
    }))
}
