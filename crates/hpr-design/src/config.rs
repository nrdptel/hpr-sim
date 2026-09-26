//! Motor mounts, configurations and the assembled rocket: the structure with its motors placed,
//! and its mass properties through the burn.
//!
//! **Placement.** A motor's axis runs forward from its nozzle exit ([`hpr_motor::mass`]). In a
//! mount whose axial extent ends at station `s_aft`, the nozzle exit sits at station
//! `s_aft + overhang`, on the mount's axis (an inner tube's radial offset and angle, or the body
//! axis). A motor element at `z_m` along the motor axis is then at body
//! `(x_mount, y_mount, −(s_aft + overhang) + z_m)` ([`MassProperties::from_motor_element`]).
//!
//! **Composition.** The rocket at time `t` is the structure ([`Layout::structure`]) combined with
//! each motor's loaded, burning or spent mass properties ([`SolidMotor::state`]) at its own time
//! since ignition. [`Assembly::mass_properties`] lights every motor at `t = 0`;
//! [`Assembly::mass_properties_lit`] takes each motor's ignition time ([`Ignition`], resolved by
//! [`Assembly::ignition_times_s`]), and a motor not yet lit is loaded. This is RocketPy's
//! composition (`Rocket.total_mass`, `center_of_mass`, and the inertias of
//! `rocketpy/rocket/rocket.py`), checked against RocketPy 1.13.0 in the tests.
//!
//! See `docs/physics/design.md`.

use std::collections::BTreeSet;

use hpr_core::DVec3;
use hpr_motor::{Delay, MassElement, SolidMotor};
use serde::{Deserialize, Serialize};

use crate::error::DesignError;
use crate::mass::MassProperties;
use crate::shapes::check_dimension;
use crate::tree::{Layout, Rocket};

/// Makes a body tube or inner tube a motor mount.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotorMount {
    /// How far the nozzle exit sits aft of the mount's aft end, m (negative when recessed).
    #[serde(default)]
    pub overhang_m: f64,
}

/// A set of motors to fly with: at most one per mount. A mount that is a cluster
/// ([`InnerTube::cluster_m`](crate::InnerTube::cluster_m)) takes its motor in every tube.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    /// Unique id among the configurations.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// The motors.
    pub motors: Vec<MountedMotor>,
}

/// When a motor lights, on the flight's clock: `t = 0` is launch, when the motors that light at
/// launch ignite. A delay is counted from its event (the decision record on staging,
/// [ADR-074][adr-074]).
///
/// [adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Ignition {
    /// At launch, `t = 0`.
    #[default]
    Launch,
    /// At a time after launch: an air start on a timer.
    Time {
        /// The time after launch, s.
        time_s: f64,
    },
    /// A delay after another mount's motor burns out: a sustainer lit by the booster's burnout,
    /// or by its ejection charge with the charge's delay.
    Burnout {
        /// The id of the mount whose motor's burnout lights this one.
        mount: String,
        /// The delay after that burnout, s.
        delay_s: f64,
    },
    /// A delay after the stage aft of this motor's stage separates from it (the flight gives the
    /// separation). A motor whose stage is never freed never lights; one in the last stage, with
    /// nothing aft of it to separate, is refused.
    Separation {
        /// The delay after the separation, s.
        delay_s: f64,
    },
}

impl Ignition {
    /// Whether this is [`Ignition::Launch`].
    #[must_use]
    pub fn is_launch(&self) -> bool {
        *self == Self::Launch
    }
}

/// A motor in a mount.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountedMotor {
    /// The id of the mount component.
    pub mount: String,
    /// Designation, for display.
    #[serde(default)]
    pub designation: String,
    /// Case outer diameter, m.
    pub diameter_m: f64,
    /// Case length, m.
    pub length_m: f64,
    /// The motor.
    pub motor: SolidMotor,
    /// The ejection delay chosen, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay: Option<Delay>,
    /// When it lights.
    #[serde(default, skip_serializing_if = "Ignition::is_launch")]
    pub ignition: Ignition,
    /// The tubes whose motor fails to light, by index into the mount's tubes (a cluster's in the
    /// order of [`InnerTube::cluster_m`](crate::InnerTube::cluster_m), `0` for a single tube; a
    /// cluster inside another cluster counts the outer copies first, each with all its tubes): a
    /// motor out. Each is carried loaded and gives no thrust. An ignition on the mount's burnout
    /// takes its first motor that lights, but a recovery device or separation triggered by one
    /// motor's index waits on that motor alone: point it at a tube that lights, or it never fires.
    /// Empty (the default) when every motor lights.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failed_tubes: Vec<usize>,
}

/// A motor placed in the rocket: one per tube of its mount, so a cluster's mount gives several,
/// one after another in the order of its tubes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedMotor {
    /// The mount's id.
    pub mount: String,
    /// Index of the mount's stage.
    pub stage: usize,
    /// The nozzle exit's position in body axes, m.
    pub nozzle_m: DVec3,
    /// The motor as configured.
    pub mounted: MountedMotor,
    /// Which of the mount's tubes it is in (`0` for a single tube).
    #[serde(default)]
    pub tube: usize,
    /// Whether it fails to light ([`MountedMotor::failed_tubes`]).
    #[serde(default)]
    pub fails: bool,
}

impl PlacedMotor {
    /// Station of the nozzle exit, m aft of the nose tip.
    pub fn nozzle_station_m(&self) -> f64 {
        -self.nozzle_m.z
    }

    /// Station of the motor case's forward end, m.
    pub fn fore_station_m(&self) -> f64 {
        self.nozzle_station_m() - self.mounted.length_m
    }

    /// A motor-axis mass element in body axes.
    pub fn place(&self, element: &MassElement) -> MassProperties {
        MassProperties::from_motor_element(element, self.nozzle_m.z).translated(DVec3::new(
            self.nozzle_m.x,
            self.nozzle_m.y,
            0.0,
        ))
    }

    /// The whole motor at `t_s` seconds after ignition, in body axes.
    pub fn mass_properties(&self, t_s: f64) -> MassProperties {
        self.place(&self.mounted.motor.state(t_s).total)
    }

    /// The whole motor at flight time `t_s` when it lit at `ignition_s` (`None`: it never lights),
    /// in body axes: loaded before it lights.
    pub fn mass_properties_lit(&self, t_s: f64, ignition_s: Option<f64>) -> MassProperties {
        let since_s = ignition_s.map_or(0.0, |ignition_s| (t_s - ignition_s).max(0.0));
        self.mass_properties(since_s)
    }

    /// The motor's dry mass (case, closures, nozzle) in body axes.
    pub fn dry_mass_properties(&self) -> MassProperties {
        self.place(&self.mounted.motor.dry())
    }
}

/// A rocket with one configuration's motors in place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assembly {
    /// The configuration's id.
    pub configuration: String,
    /// The resolved structure.
    pub layout: Layout,
    /// The motors.
    pub motors: Vec<PlacedMotor>,
}

impl Assembly {
    /// The rocket `t_s` seconds after ignition: the structure and every motor, but for a motor
    /// that fails ([`PlacedMotor::fails`]), which stays loaded. Every other motor is taken as lit,
    /// even one waiting on a mount whose every motor fails; [`Self::mass_properties_lit`] with
    /// [`Self::ignition_times_s`] gives the flight's.
    pub fn mass_properties(&self, t_s: f64) -> MassProperties {
        self.motors
            .iter()
            .fold(self.layout.structure, |sum, motor| {
                let ignition = (!motor.fails).then_some(0.0);
                MassProperties::combine([&sum, &motor.mass_properties_lit(t_s, ignition)])
            })
    }

    /// The rocket at flight time `t_s`, each motor lit at its `ignition_s`, one per motor in
    /// order ([`Self::ignition_times_s`]): `None` for one that never lights, which stays loaded.
    /// A motor past the end of `ignition_s` is taken as lit at launch, as
    /// [`Self::mass_properties`] lights them all.
    pub fn mass_properties_lit(&self, t_s: f64, ignition_s: &[Option<f64>]) -> MassProperties {
        self.motors
            .iter()
            .enumerate()
            .fold(self.layout.structure, |sum, (index, motor)| {
                let ignition = ignition_s.get(index).copied().unwrap_or(Some(0.0));
                MassProperties::combine([&sum, &motor.mass_properties_lit(t_s, ignition)])
            })
    }

    /// Each motor's ignition time on the flight's clock, in order: `None` for one that never
    /// lights. `separated_s(stage)` gives the time at which the stage aft of `stage` came away, if
    /// it has; [`Ignition::Separation`] counts from it.
    ///
    /// A motor lit by another mount's burnout lights when one of that mount's motors that lights
    /// has a known ignition; a chain that never reaches a known time (a cycle is refused by
    /// [`Layout::place_motors`], so only a separation that never comes, or a mount whose every
    /// motor fails) leaves it unlit. A motor that fails ([`PlacedMotor::fails`]) never lights.
    #[must_use]
    pub fn ignition_times_s(&self, separated_s: impl Fn(usize) -> Option<f64>) -> Vec<Option<f64>> {
        ignition_times_s(&self.motors, separated_s, true)
    }

    /// The rocket with every motor spent: the structure and the motors' dry masses, but for a
    /// motor that fails ([`PlacedMotor::fails`]), which is carried loaded.
    pub fn dry_mass_properties(&self) -> MassProperties {
        self.motors
            .iter()
            .fold(self.layout.structure, |sum, motor| {
                let motor = if motor.fails {
                    motor.mass_properties_lit(0.0, None)
                } else {
                    motor.dry_mass_properties()
                };
                MassProperties::combine([&sum, &motor])
            })
    }
}

/// As [`Assembly::ignition_times_s`], over `motors`; with `failures` false, as if every motor
/// lit.
fn ignition_times_s(
    motors: &[PlacedMotor],
    separated_s: impl Fn(usize) -> Option<f64>,
    failures: bool,
) -> Vec<Option<f64>> {
    let fails = |motor: &PlacedMotor| failures && motor.fails;
    let mut times: Vec<Option<f64>> = motors
        .iter()
        .map(|motor| match &motor.mounted.ignition {
            _ if fails(motor) => None,
            Ignition::Launch => Some(0.0),
            Ignition::Time { time_s } => Some(*time_s),
            Ignition::Separation { delay_s } => separated_s(motor.stage).map(|t| t + delay_s),
            Ignition::Burnout { .. } => None,
        })
        .collect();
    // Each pass resolves at least one more link of every chain that can resolve, so as many
    // passes as there are motors settle them all.
    for _ in 0..motors.len() {
        let mut changed = false;
        for (index, motor) in motors.iter().enumerate() {
            let Ignition::Burnout { mount, delay_s } = &motor.mounted.ignition else {
                continue;
            };
            if times[index].is_some() || fails(motor) {
                continue;
            }
            // Every motor of a mount lights together, so the first that lights gives the burnout.
            let lit = motors
                .iter()
                .zip(&times)
                .filter(|(other, _)| other.mount == *mount)
                .find_map(|(other, time)| time.map(|t| t + other.mounted.motor.burnout_time_s()));
            if let Some(burnout_s) = lit {
                times[index] = Some(burnout_s + delay_s);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    times
}

impl Layout {
    /// Places `configuration`'s motors in their mounts in this layout.
    ///
    /// # Errors
    ///
    /// - [`DesignError::UnknownId`] for a mount that doesn't exist.
    /// - [`DesignError::Tree`] for a mount that isn't a motor mount, or two motors in one mount.
    /// - [`DesignError::InComponent`] (with the mount's id) wrapping [`DesignError::Domain`] for a
    ///   non-positive or non-finite motor diameter or length, or a non-finite overhang, or an
    ///   ignition time or delay that is negative or not finite.
    /// - [`DesignError::Tree`] for an ignition by the burnout of a mount with no motor in this
    ///   configuration, or by a chain of burnouts that comes back to the motor itself.
    /// - [`DesignError::InComponent`] (with the mount's id) wrapping [`DesignError::Domain`] for a
    ///   failed tube the mount doesn't have, or one named twice, or a cluster offset that is not
    ///   finite.
    pub fn place_motors(
        &self,
        configuration: &Configuration,
    ) -> Result<Vec<PlacedMotor>, DesignError> {
        let mut mounts = BTreeSet::new();
        let mut motors = Vec::with_capacity(configuration.motors.len());
        for mounted in &configuration.motors {
            let (_, mount) = self
                .find(&mounted.mount)
                .ok_or_else(|| DesignError::UnknownId {
                    what: "motor mount",
                    id: mounted.mount.clone(),
                })?;
            let in_mount = |error| DesignError::InComponent {
                id: mount.id.clone(),
                source: Box::new(error),
            };
            check_dimension("motor diameter (m)", mounted.diameter_m, false).map_err(in_mount)?;
            check_dimension("motor length (m)", mounted.length_m, false).map_err(in_mount)?;
            let Some(spec) = mount.motor_mount else {
                return Err(DesignError::Tree {
                    id: mount.id.clone(),
                    message: format!(
                        "configuration {} puts a motor here, but it is not a motor mount",
                        configuration.id
                    ),
                });
            };
            if !mounts.insert(mount.id.as_str()) {
                return Err(DesignError::Tree {
                    id: mount.id.clone(),
                    message: format!(
                        "configuration {} puts two motors in this mount",
                        configuration.id
                    ),
                });
            }
            if !spec.overhang_m.is_finite() {
                return Err(in_mount(DesignError::Domain {
                    what: "motor overhang (m)",
                    value: spec.overhang_m,
                }));
            }
            match &mounted.ignition {
                Ignition::Launch => {}
                Ignition::Time { time_s: value } => {
                    check_ignition("ignition time after launch (s)", *value).map_err(in_mount)?;
                }
                Ignition::Burnout { delay_s: value, .. } => {
                    check_ignition("ignition delay (s)", *value).map_err(in_mount)?;
                }
                Ignition::Separation { delay_s: value } => {
                    check_ignition("ignition delay (s)", *value).map_err(in_mount)?;
                    if mount.stage + 1 >= self.stages.len() {
                        return Err(DesignError::Tree {
                            id: mount.id.clone(),
                            message: format!(
                                "configuration {} lights this motor at its stage's separation, \
                                 but no stage is aft of it to separate",
                                configuration.id
                            ),
                        });
                    }
                }
            }
            let tubes = mount.contents_copies_m().map_err(in_mount)?;
            for (k, &tube) in mounted.failed_tubes.iter().enumerate() {
                if tube >= tubes.len() || mounted.failed_tubes[..k].contains(&tube) {
                    return Err(in_mount(DesignError::Domain {
                        what: "failed tube (index into the mount's tubes, each named once)",
                        value: tube as f64,
                    }));
                }
            }
            let [x, y] = mount.part.axis_offset_m();
            let z = -(mount.aft_station_m() + spec.overhang_m);
            for (tube, [u, v]) in tubes.into_iter().enumerate() {
                motors.push(PlacedMotor {
                    mount: mount.id.clone(),
                    stage: mount.stage,
                    nozzle_m: DVec3::new(x + u, y + v, z),
                    mounted: mounted.clone(),
                    tube,
                    fails: mounted.failed_tubes.contains(&tube),
                });
            }
        }
        for motor in &motors {
            if let Ignition::Burnout { mount, .. } = &motor.mounted.ignition
                && !motors.iter().any(|other| other.mount == *mount)
            {
                return Err(DesignError::Tree {
                    id: motor.mount.clone(),
                    message: format!(
                        "configuration {} lights this motor at the burnout of mount {mount}, \
                         which holds no motor in it",
                        configuration.id
                    ),
                });
            }
        }
        // With every separation at once and every motor lit, only a cycle of burnouts is left
        // unlit.
        let lit = ignition_times_s(&motors, |_| Some(0.0), false);
        if let Some(index) = lit.iter().position(Option::is_none) {
            return Err(DesignError::Tree {
                id: motors[index].mount.clone(),
                message: format!(
                    "configuration {} lights this motor by a chain of burnouts that comes back \
                     to it",
                    configuration.id
                ),
            });
        }
        Ok(motors)
    }
}

/// An ignition time or delay: finite and not negative.
fn check_ignition(what: &'static str, value: f64) -> Result<(), DesignError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(DesignError::Domain { what, value })
    }
}

impl Rocket {
    /// The configuration with id `id`.
    pub fn configuration(&self, id: &str) -> Option<&Configuration> {
        self.configurations.iter().find(|c| c.id == id)
    }

    /// Checks that configuration ids are unique and non-empty.
    ///
    /// # Errors
    ///
    /// [`DesignError::DuplicateId`] for the first empty or repeated id.
    pub fn check_configuration_ids(&self) -> Result<(), DesignError> {
        let mut ids = BTreeSet::new();
        for configuration in &self.configurations {
            if configuration.id.is_empty() || !ids.insert(configuration.id.as_str()) {
                return Err(DesignError::DuplicateId(configuration.id.clone()));
            }
        }
        Ok(())
    }

    /// Resolves the tree and places the motors of configuration `configuration_id`.
    ///
    /// # Errors
    ///
    /// - As [`Rocket::layout`] and [`Layout::place_motors`].
    /// - [`DesignError::DuplicateId`] for an empty or repeated configuration id.
    /// - [`DesignError::UnknownId`] for a configuration that doesn't exist.
    pub fn assemble(&self, configuration_id: &str) -> Result<Assembly, DesignError> {
        self.check_configuration_ids()?;
        let configuration =
            self.configuration(configuration_id)
                .ok_or_else(|| DesignError::UnknownId {
                    what: "configuration",
                    id: configuration_id.to_owned(),
                })?;
        let layout = self.layout()?;
        let motors = layout.place_motors(configuration)?;
        Ok(Assembly {
            configuration: configuration.id.clone(),
            layout,
            motors,
        })
    }
}

#[cfg(test)]
mod tests {
    use hpr_core::DMat3;

    use super::*;
    use crate::testing::{motor, three_fin_rocket};

    /// The sample rocket's mount spans stations 0.7 to 1.0 with 0.01 m of overhang, so the nozzle
    /// exit is at 1.01. Its envelope motor (1 kg loaded, 0.5 kg of propellant, 0.2 m long) puts
    /// dry mass and propellant both 0.1 m forward of the nozzle, at body z = −0.91.
    #[test]
    fn motor_sits_at_mount_aft_end_plus_overhang() {
        let design = three_fin_rocket();
        let assembly = design.assemble("main").unwrap();
        let [placed] = &assembly.motors[..] else {
            panic!("one motor")
        };
        assert!((placed.nozzle_station_m() - 1.01).abs() < 1e-15);
        assert!((placed.fore_station_m() - 0.81).abs() < 1e-15);
        assert_eq!(placed.stage, 0);
        assert_eq!((placed.nozzle_m.x, placed.nozzle_m.y), (0.0, 0.0));

        let structure = assembly.layout.structure;
        let (m_s, z_s) = (structure.mass_kg, structure.cg_m.z);
        for (t, motor_kg) in [(0.0, 1.0), (0.5, 0.75), (2.0, 0.5)] {
            let whole = assembly.mass_properties(t);
            let want_mass = m_s + motor_kg;
            assert!((whole.mass_kg - want_mass).abs() < 1e-12, "{t}");
            let want_z = (m_s * z_s + motor_kg * -0.91) / want_mass;
            assert!((whole.cg_m.z - want_z).abs() < 1e-12, "{t}");
            // Parallel axis about the rocket's centre, from the motor's own moments at `t`. The
            // launch lug puts the centre slightly off the axis, where the motor is not.
            let state = placed.mounted.motor.state(t).total;
            let (dx, dy, dz) = (-whole.cg_m.x, -whole.cg_m.y, -0.91 - whole.cg_m.z);
            let about = structure.inertia_about(whole.cg_m);
            let transverse =
                about.x_axis.x + state.transverse_inertia_kg_m2 + motor_kg * (dy * dy + dz * dz);
            assert!(
                (whole.inertia_kg_m2.x_axis.x - transverse).abs() < 1e-12 * transverse,
                "{t}"
            );
            let axial = about.z_axis.z + state.axial_inertia_kg_m2 + motor_kg * (dx * dx + dy * dy);
            assert!(
                (whole.inertia_kg_m2.z_axis.z - axial).abs() < 1e-12 * axial,
                "{t}"
            );
            assert!(whole.cg_m.x.abs() > 1e-6, "the lug is off the axis");
        }
        let dry = assembly.dry_mass_properties();
        assert!((dry.mass_kg - (m_s + 0.5)).abs() < 1e-12);
        assert_eq!(dry, assembly.mass_properties(1.0));
    }

    /// A motor in an inner tube 0.01 m off the axis at 90° sits at `y = 0.01`, and the rocket
    /// picks up the product of inertia `I_yz = −Σ m y z` that offset makes.
    #[test]
    fn motor_in_an_offset_mount_is_off_axis() {
        let mut design = three_fin_rocket();
        let mount = &mut design.stages[0].components[1].children[0];
        if let crate::Part::InnerTube(tube) = &mut mount.part {
            tube.outer_radius_m = 0.0105;
            tube.radial_offset_m = 0.01;
            tube.angle_rad = std::f64::consts::FRAC_PI_2;
        }
        // Drop the on-axis centering rings.
        design.stages[0].components[1].children.drain(1..3);
        design.configurations[0].motors = vec![motor("mmt", 0.018, 0.1)];
        let assembly = design.assemble("main").unwrap();
        let placed = &assembly.motors[0];
        assert!(placed.nozzle_m.x.abs() < 1e-18);
        assert!((placed.nozzle_m.y - 0.01).abs() < 1e-18);
        let m = placed.mass_properties(0.0);
        assert!((m.cg_m.y - 0.01).abs() < 1e-18);
        let about_origin = m.inertia_about(DVec3::ZERO);
        let own = MassProperties {
            cg_m: DVec3::ZERO,
            ..m
        };
        let offset = about_origin - own.inertia_kg_m2;
        let want_yz = -m.mass_kg * m.cg_m.y * m.cg_m.z;
        assert!((offset.z_axis.y - want_yz).abs() < 1e-15, "{offset:?}");
        // cos 90° is 6e-17, not zero.
        assert!(offset.y_axis.x.abs() < 1e-18);
        assert!(assembly.mass_properties(0.0).inertia_kg_m2.z_axis.y.abs() > 1e-6);
        assert_ne!(m.inertia_kg_m2, DMat3::ZERO);
    }

    /// The sample rocket with its mount a cluster of `tubes` 18 mm tubes (their axes where
    /// `tubes` says), holding a 0.1 m envelope motor, and no centering rings.
    fn clustered(tubes: Vec<[f64; 2]>) -> Rocket {
        let mut design = three_fin_rocket();
        let mount = &mut design.stages[0].components[1].children[0];
        if let crate::Part::InnerTube(tube) = &mut mount.part {
            tube.outer_radius_m = 0.0095;
            tube.cluster_m = tubes;
        }
        design.stages[0].components[1].children.drain(1..3);
        design.configurations[0].motors = vec![motor("mmt", 0.018, 0.1)];
        design
    }

    /// Three tubes on a ring of radius `d = 0.02` m, at 90°, 210° and 330°.
    fn ring_of_three() -> Vec<[f64; 2]> {
        [90.0_f64, 210.0, 330.0]
            .iter()
            .map(|a| [0.02 * a.to_radians().cos(), 0.02 * a.to_radians().sin()])
            .collect()
    }

    /// A motor in a 3-ring cluster is three motors, one in each tube, their nozzles where the tubes
    /// are and at the one station. The rocket weighs the structure and all three at every time,
    /// and about the body's axis its roll inertia is the structure's plus, for each motor, its own
    /// axial inertia and `m d²`, worked by hand.
    #[test]
    fn a_cluster_mount_takes_its_motor_in_every_tube() {
        let assembly = clustered(ring_of_three()).assemble("main").unwrap();
        assert_eq!(assembly.motors.len(), 3);
        for (k, (placed, [x, y])) in assembly.motors.iter().zip(ring_of_three()).enumerate() {
            assert_eq!((placed.tube, placed.fails), (k, false));
            assert_eq!(placed.mount, "mmt");
            assert_eq!((placed.nozzle_m.x, placed.nozzle_m.y), (x, y));
            assert!((placed.nozzle_station_m() - 1.01).abs() < 1e-15);
        }
        let structure = assembly.layout.structure;
        for (t, motor_kg) in [(0.0, 1.0), (0.5, 0.75), (2.0, 0.5)] {
            let whole = assembly.mass_properties(t);
            assert!((whole.mass_kg - (structure.mass_kg + 3.0 * motor_kg)).abs() < 1e-12);
            let axis = DVec3::new(0.0, 0.0, whole.cg_m.z);
            let own = assembly.motors[0].mounted.motor.state(t).total;
            let want = structure.inertia_about(axis).z_axis.z
                + 3.0 * (own.axial_inertia_kg_m2 + motor_kg * 0.02 * 0.02);
            let got = whole.inertia_about(axis).z_axis.z;
            assert!((got - want).abs() < 1e-12 * want, "{t}: {got} vs {want}");
        }
    }

    /// A motor out: the tube named in `failed_tubes` keeps its motor loaded and never lights it,
    /// the others burn. A tube the mount doesn't have, or one named twice, is refused.
    #[test]
    fn a_failed_tube_never_lights() {
        let mut design = clustered(ring_of_three());
        design.configurations[0].motors[0].failed_tubes = vec![0];
        let assembly = design.assemble("main").unwrap();
        let fails: Vec<bool> = assembly.motors.iter().map(|m| m.fails).collect();
        assert_eq!(fails, [true, false, false]);
        let lit = assembly.ignition_times_s(|_| None);
        assert_eq!(lit, [None, Some(0.0), Some(0.0)]);
        let spent = assembly.mass_properties_lit(2.0, &lit);
        let structure = assembly.layout.structure.mass_kg;
        assert!((spent.mass_kg - (structure + 1.0 + 0.5 + 0.5)).abs() < 1e-12);
        // The loaded motor pulls the centre toward its tube, at 90°: +y.
        let all_burning = assembly.mass_properties_lit(2.0, &[Some(0.0); 3]);
        assert!(spent.cg_m.y > all_burning.cg_m.y + 1e-4);
        assert_eq!(assembly.mass_properties(2.0), spent);

        for (failed, value) in [(vec![3], 3.0), (vec![1, 1], 1.0)] {
            let mut design = clustered(ring_of_three());
            design.configurations[0].motors[0].failed_tubes = failed;
            let Err(DesignError::InComponent { id, source }) = design.assemble("main") else {
                panic!("refused");
            };
            assert_eq!(id, "mmt");
            assert!(
                matches!(*source, DesignError::Domain { what, value: v }
                    if what.starts_with("failed tube") && v == value),
                "{source:?}"
            );
        }
        // One tube is tube 0.
        let mut design = three_fin_rocket();
        design.configurations[0].motors[0].failed_tubes = vec![0];
        let assembly = design.assemble("main").unwrap();
        assert_eq!(assembly.ignition_times_s(|_| None), [None]);
        // A motor that never lights is carried loaded, spent or burning as the rest may be.
        let loaded = MassProperties::combine([
            &assembly.layout.structure,
            &assembly.motors[0].mass_properties(0.0),
        ]);
        assert_eq!(assembly.dry_mass_properties(), loaded);
        assert_eq!(assembly.mass_properties(10.0), loaded);
    }

    #[test]
    fn bad_configurations_are_refused() {
        let design = three_fin_rocket();
        assert!(matches!(
            design.assemble("missing"),
            Err(DesignError::UnknownId {
                what: "configuration",
                ..
            })
        ));

        let mut d = design.clone();
        d.configurations[0].motors[0].mount = "nowhere".to_owned();
        assert!(matches!(
            d.assemble("main"),
            Err(DesignError::UnknownId {
                what: "motor mount",
                ..
            })
        ));

        let mut d = design.clone();
        d.configurations[0].motors[0].mount = "airframe".to_owned();
        assert!(
            matches!(d.assemble("main"), Err(DesignError::Tree { ref id, .. }) if id == "airframe")
        );

        let mut d = design.clone();
        let second = d.configurations[0].motors[0].clone();
        d.configurations[0].motors.push(second);
        assert!(matches!(d.assemble("main"), Err(DesignError::Tree { ref id, .. }) if id == "mmt"));

        let mut d = design.clone();
        let copy = d.configurations[0].clone();
        d.configurations.push(copy);
        assert_eq!(
            d.assemble("main"),
            Err(DesignError::DuplicateId("main".to_owned()))
        );

        let mut d = design.clone();
        d.configurations[0].motors[0].diameter_m = 0.0;
        assert!(matches!(
            d.assemble("main"),
            Err(DesignError::InComponent { ref id, ref source })
                if id == "mmt" && matches!(**source, DesignError::Domain { .. })
        ));

        let mut d = design.clone();
        d.stages[0].components[1].children[0].motor_mount = Some(MotorMount {
            overhang_m: f64::NAN,
        });
        assert!(matches!(
            d.assemble("main"),
            Err(DesignError::InComponent { .. })
        ));

        // A layout places any configuration handed to it, not only the rocket's own.
        let layout = design.layout().unwrap();
        let mut candidate = design.configurations[0].clone();
        candidate.id = "candidate".to_owned();
        assert_eq!(layout.place_motors(&candidate).unwrap().len(), 1);
        candidate.motors[0].mount = "chute".to_owned();
        assert!(matches!(
            layout.place_motors(&candidate),
            Err(DesignError::Tree { ref id, .. }) if id == "chute"
        ));
    }
    /// The RocketPy example designs in `validation/designs/`, by oracle case name.
    const ROCKETPY_DESIGNS: [(&str, &str); 10] = [
        (
            "calisto-getting-started-motor-at-minus-1.255",
            include_str!(
                "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
            ),
        ),
        (
            "calisto-tests-motor-at-minus-1.373",
            include_str!(
                "../../../validation/designs/rocketpy-calisto-tests-motor-at-minus-1.373.json"
            ),
        ),
        (
            "bella-lui",
            include_str!("../../../validation/designs/rocketpy-bella-lui.json"),
        ),
        (
            "ndrt-2020-nose-to-tail",
            include_str!("../../../validation/designs/rocketpy-ndrt-2020-nose-to-tail.json"),
        ),
        (
            "valetudo",
            include_str!("../../../validation/designs/rocketpy-valetudo.json"),
        ),
        (
            "juno-iii",
            include_str!("../../../validation/designs/rocketpy-juno-iii.json"),
        ),
        (
            "cavour",
            include_str!("../../../validation/designs/rocketpy-cavour.json"),
        ),
        (
            "genesis",
            include_str!("../../../validation/designs/rocketpy-genesis.json"),
        ),
        (
            "lince",
            include_str!("../../../validation/designs/rocketpy-lince.json"),
        ),
        (
            "prometheus-2022-generic-motor",
            include_str!("../../../validation/designs/rocketpy-prometheus-2022-generic-motor.json"),
        ),
    ];

    #[derive(Debug, serde::Deserialize)]
    struct Oracle {
        oracle: String,
        cases: Vec<OracleCase>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleCase {
        name: String,
        rocket: OracleRocket,
        motor: serde_json::Value,
        geometry: serde_json::Value,
        scalars: OracleScalars,
        series: OracleSeries,
        knot_series: Option<OracleSeries>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct OracleRocket {
        mass: f64,
        inertia: Vec<f64>,
        center_of_mass_without_motor: f64,
        coordinate_system_orientation: String,
        motor_position: f64,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(non_snake_case, reason = "RocketPy's attribute names")]
    struct OracleScalars {
        dry_mass_kg: f64,
        center_of_dry_mass_position_m: f64,
        dry_I_11_kg_m2: f64,
        dry_I_33_kg_m2: f64,
        nozzle_position_m: f64,
        propellant_initial_mass_kg: f64,
    }

    #[derive(Debug, serde::Deserialize)]
    #[allow(non_snake_case, reason = "RocketPy's attribute names")]
    struct OracleSeries {
        time_s: Vec<f64>,
        propellant_mass: Vec<f64>,
        total_mass: Vec<f64>,
        center_of_mass: Vec<f64>,
        I_11: Vec<f64>,
        I_33: Vec<f64>,
        I_11_about_cg: Vec<f64>,
    }

    /// The largest error seen for each quantity, with where.
    #[derive(Default)]
    struct Worst(Vec<(&'static str, f64, String)>);

    impl Worst {
        fn see(&mut self, what: &'static str, error: f64, at: impl Fn() -> String) {
            assert!(error.is_finite(), "{what}: {}", at());
            match self.0.iter_mut().find(|w| w.0 == what) {
                Some(w) if error > w.1 => (w.1, w.2) = (error, at()),
                Some(_) => {}
                None => self.0.push((what, error, at())),
            }
        }
    }

    fn relative(got: f64, want: f64) -> f64 {
        ((got - want) / want).abs()
    }

    fn number(v: &serde_json::Value, key: &str) -> f64 {
        v[key]
            .as_f64()
            .unwrap_or_else(|| panic!("fixture motor `{key}`"))
    }

    /// M1.4b's done-when: mass, centre of mass and inertia match RocketPy's example rockets. The
    /// fixture comes from `validation/oracles/rocketpy/rocket_mass.py` (RocketPy 1.13.0), which
    /// builds each example rocket from its own inputs, with the bundled public-domain curve nearest
    /// in impulse in place of the example's thrust file (whose terms are unclear; ADR-007). Its
    /// docstring lists the examples left out and why.
    ///
    /// The designs in `validation/designs/` are generated by `cargo xtask designs`. This test
    /// derives the mass inputs again from the fixture, independently of that generator: the stage
    /// override, the nozzle station through RocketPy's placement rule
    /// (`rocket.py:1113-1125`: `z = p + s·z_m`, `s` the product of the two orientation signs), and
    /// the motor's grains or column, dry mass and curve. Then it compares, at 103 times through the
    /// burn and after it and at up to 60 of RocketPy's LSODA knots: total mass, the centre of mass
    /// (as a station, relative to the rocket's length), `I_11` about the centre of dry mass
    /// (RocketPy's reference point), `I_11` about the centre of mass, `I_33`, and the propellant
    /// mass, plus the dry scalars.
    #[test]
    fn matches_rocketpy_example_rockets() {
        let oracle: Oracle = serde_json::from_str(include_str!(
            "../../../validation/fixtures/design/rocketpy-rocket-mass.json"
        ))
        .unwrap();
        assert_eq!(oracle.oracle, "rocketpy 1.13.0");
        assert_eq!(oracle.cases.len(), ROCKETPY_DESIGNS.len());
        let catalog = hpr_motor::Catalog::bundled().unwrap();
        let mut worst = Worst::default();
        for case in &oracle.cases {
            let (_, text) = ROCKETPY_DESIGNS
                .iter()
                .find(|(name, _)| *name == case.name)
                .unwrap_or_else(|| panic!("no design for {}", case.name));
            let design: Rocket = serde_json::from_str(text).unwrap();
            let assembly = design.assemble("example").unwrap();
            let name = case.name.as_str();

            // Stations aft of the nose tip, from RocketPy coordinates.
            let tip = case.geometry["nose"]["position"].as_f64().unwrap();
            let rocket_sign = match case.rocket.coordinate_system_orientation.as_str() {
                "tail_to_nose" => 1.0,
                "nose_to_tail" => -1.0,
                other => panic!("{other}"),
            };
            let station = |z: f64| rocket_sign * (tip - z);

            // The stage override is the example's rocket without its motor.
            let overrides = design.stages[0].overrides;
            assert_eq!(overrides.mass_kg, Some(case.rocket.mass), "{name}");
            assert_eq!(overrides.cg_xy_m, Some([0.0, 0.0]), "{name}: on the axis");
            let cg_aft = overrides.cg_aft_m.unwrap();
            assert!(
                (cg_aft - station(case.rocket.center_of_mass_without_motor)).abs() < 1e-12,
                "{name}"
            );
            let [i11, i22, i33] = case.rocket.inertia[..] else {
                panic!("{name}: inertia")
            };
            assert_eq!(i11, i22);
            assert_eq!(
                overrides.inertia,
                Some(crate::InertiaOverride::axisymmetric(i33, i11)),
                "{name}"
            );

            // The motor: RocketPy's inputs on hpr's axis, forward of the nozzle exit.
            let m = &case.motor;
            let motor_sign = match m["coordinate_system_orientation"].as_str().unwrap() {
                "nozzle_to_combustion_chamber" => 1.0,
                "combustion_chamber_to_nozzle" => -1.0,
                other => panic!("{other}"),
            };
            let nozzle = number(m, "nozzle_position");
            let to_motor_axis = |z: f64| motor_sign * (z - nozzle);
            let [placed] = &assembly.motors[..] else {
                panic!("{name}: one motor")
            };
            let rocketpy_nozzle = case.rocket.motor_position + rocket_sign * motor_sign * nozzle;
            assert!(
                (rocketpy_nozzle - case.scalars.nozzle_position_m).abs() < 1e-12,
                "{name}"
            );
            assert!(
                (placed.nozzle_station_m() - station(rocketpy_nozzle)).abs() < 1e-12,
                "{name}: nozzle at {} vs {}",
                placed.nozzle_station_m(),
                station(rocketpy_nozzle)
            );
            let motor = &placed.mounted.motor;
            let file = m["thrust_file"].as_str().unwrap();
            let (entry, curve) = catalog
                .motors
                .iter()
                .find_map(|e| e.curves.iter().find(|c| c.file == file).map(|c| (e, c)))
                .unwrap();
            assert_eq!(curve.sha256, m["thrust_file_sha256"].as_str().unwrap());
            let bundled = entry
                .thrust_curve(
                    curve,
                    hpr_motor::catalog::bundled_curve_text(&curve.file).unwrap(),
                )
                .unwrap();
            assert_eq!(motor.curve(), &bundled, "{name}: the bundled curve");
            let dry = motor.dry();
            // hpr's motor needs a positive dry mass: the generator gives Cavour's massless dry
            // parts 1e-15 kg.
            let example_dry = number(m, "dry_mass");
            let expected_dry = if example_dry == 0.0 {
                1e-15
            } else {
                example_dry
            };
            assert_eq!(dry.mass_kg, expected_dry, "{name}");
            let dry_inertia: Vec<f64> = m["dry_inertia"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            assert_eq!(dry.transverse_inertia_kg_m2, dry_inertia[0], "{name}");
            assert_eq!(dry.axial_inertia_kg_m2, dry_inertia[2], "{name}");
            match (m["motor_kind"].as_str().unwrap(), motor.propellant()) {
                ("solid", hpr_motor::Propellant::Grains(g)) => {
                    assert_eq!(
                        *g,
                        hpr_motor::BatesGrains {
                            count: u32::try_from(m["grain_number"].as_u64().unwrap()).unwrap(),
                            density_kg_m3: number(m, "grain_density"),
                            outer_radius_m: number(m, "grain_outer_radius"),
                            initial_inner_radius_m: number(m, "grain_initial_inner_radius"),
                            initial_height_m: number(m, "grain_initial_height"),
                            separation_m: number(m, "grain_separation"),
                            center_m: to_motor_axis(number(m, "grains_center_of_mass_position")),
                            inhibited_ends: m["only_radial_burn"].as_bool().unwrap(),
                        },
                        "{name}"
                    );
                    assert!(
                        (dry.cg_m - to_motor_axis(number(m, "center_of_dry_mass_position"))).abs()
                            < 1e-15,
                        "{name}"
                    );
                }
                ("generic", hpr_motor::Propellant::Column(c)) => {
                    // RocketPy puts an unset dry centre at the chamber (motor.py:1518-1523).
                    assert!(m["center_of_dry_mass_position"].is_null());
                    let chamber = to_motor_axis(number(m, "chamber_position"));
                    assert_eq!(
                        *c,
                        hpr_motor::PropellantColumn {
                            mass_kg: number(m, "propellant_initial_mass"),
                            center_m: chamber,
                            outer_radius_m: number(m, "chamber_radius"),
                            inner_radius_m: 0.0,
                            length_m: number(m, "chamber_height"),
                        },
                        "{name}"
                    );
                    assert_eq!(dry.cg_m, chamber, "{name}");
                }
                (kind, other) => panic!("{name}: {kind} vs {other:?}"),
            }
            let m_p0 = motor.propellant_initial_mass_kg();
            worst.see(
                "initial propellant mass",
                relative(m_p0, case.scalars.propellant_initial_mass_kg),
                || name.to_owned(),
            );

            // The dry rocket.
            let length = assembly.layout.length_m;
            let dry_rocket = assembly.dry_mass_properties();
            worst.see(
                "dry mass",
                relative(dry_rocket.mass_kg, case.scalars.dry_mass_kg),
                || name.to_owned(),
            );
            worst.see(
                "dry centre (of length)",
                (-dry_rocket.cg_m.z - station(case.scalars.center_of_dry_mass_position_m)).abs()
                    / length,
                || name.to_owned(),
            );
            worst.see(
                "dry I_11",
                relative(
                    dry_rocket.inertia_kg_m2.x_axis.x,
                    case.scalars.dry_I_11_kg_m2,
                ),
                || name.to_owned(),
            );
            worst.see(
                "dry I_33",
                relative(
                    dry_rocket.inertia_kg_m2.z_axis.z,
                    case.scalars.dry_I_33_kg_m2,
                ),
                || name.to_owned(),
            );

            // Through the burn, on the even grid and at RocketPy's own knots.
            let grids = [
                ("", Some(&case.series)),
                (" at knots", case.knot_series.as_ref()),
            ];
            for (grid, s) in grids.into_iter().filter_map(|(g, s)| Some((g, s?))) {
                let label = |what: &'static str| -> &'static str {
                    match (what, grid.is_empty()) {
                        (_, true) => what,
                        ("total mass", false) => "total mass at knots",
                        ("centre of mass (of length)", false) => "centre of mass at knots",
                        ("I_11 about the dry centre", false) => {
                            "I_11 about the dry centre at knots"
                        }
                        ("I_22 about the dry centre", false) => {
                            "I_22 about the dry centre at knots"
                        }
                        ("I_11 about the centre of mass", false) => {
                            "I_11 about the centre of mass at knots"
                        }
                        ("I_33", false) => "I_33 at knots",
                        ("products of inertia (of I_11)", false) => "products of inertia at knots",
                        ("propellant mass, grains (of initial)", false) => {
                            "propellant mass, grains, at knots"
                        }
                        (other, false) => panic!("no knot label for {other}"),
                    }
                };
                for (i, &t) in s.time_s.iter().enumerate() {
                    let at = || format!("{name} at t = {t} s");
                    let whole = assembly.mass_properties(t);
                    worst.see(
                        label("total mass"),
                        relative(whole.mass_kg, s.total_mass[i]),
                        at,
                    );
                    worst.see(
                        label("centre of mass (of length)"),
                        (-whole.cg_m.z - station(s.center_of_mass[i])).abs() / length,
                        at,
                    );
                    let about_dry = whole.inertia_about(dry_rocket.cg_m);
                    worst.see(
                        label("I_11 about the dry centre"),
                        relative(about_dry.x_axis.x, s.I_11[i]),
                        at,
                    );
                    worst.see(
                        label("I_22 about the dry centre"),
                        relative(about_dry.y_axis.y, s.I_11[i]),
                        at,
                    );
                    worst.see(
                        label("I_11 about the centre of mass"),
                        relative(whole.inertia_kg_m2.x_axis.x, s.I_11_about_cg[i]),
                        at,
                    );
                    worst.see(
                        label("I_33"),
                        relative(whole.inertia_kg_m2.z_axis.z, s.I_33[i]),
                        at,
                    );
                    let products = [
                        whole.inertia_kg_m2.y_axis.x,
                        whole.inertia_kg_m2.z_axis.x,
                        whole.inertia_kg_m2.z_axis.y,
                    ];
                    worst.see(
                        label("products of inertia (of I_11)"),
                        products.iter().fold(0.0f64, |a, v| a.max(v.abs())) / s.I_11[i],
                        at,
                    );
                    let what = match motor.propellant() {
                        hpr_motor::Propellant::Column(_) => "propellant mass, column (of initial)",
                        _ => "propellant mass, grains (of initial)",
                    };
                    worst.see(
                        label(what),
                        (motor.propellant_mass_kg(t) - s.propellant_mass[i]).abs() / m_p0,
                        at,
                    );
                }
            }
        }
        // Measured worst (RocketPy 1.13.0), and each tolerance's margin over it:
        //
        // - Exact quantities: dry mass, centre and inertias, initial propellant mass and the
        //   column's propellant mass agree to 2.4e-16 (tolerance 1e-12). Products of inertia are
        //   exactly zero (1e-15).
        // - At RocketPy's LSODA knots, where its grain geometry holds computed values: total mass
        //   3.3e-10, centre 1.4e-10 of the length, `I_11` 8.0e-10, `I_33` 1.8e-10, grain propellant
        //   mass 2.4e-9 of its initial value. That is the solver's own accuracy (rtol 1e-11); the
        //   tolerance is 1e-8, a margin of 4 to 70.
        // - On the even grid, between knots: total mass 1.3e-5 (Lince), centre 3.6e-6, `I_11` 2.6e-5,
        //   `I_33` 1.4e-5, grain propellant mass 4.9e-5. `SolidMotor` interpolates its grain
        //   volumes linearly between LSODA knots (`solid_motor.py:375-383`, `:603-630`) and
        //   `GenericMotor` samples its inertias at the thrust knots, while hpr's are exact for a
        //   piecewise-linear curve. Tolerances: 5e-5 (margin 3.8), 2e-5 (5.6), 1e-4 (3.9 and 7) and
        //   2.5e-4 (5).
        let tolerance = |what: &str| match what {
            "initial propellant mass"
            | "dry mass"
            | "dry centre (of length)"
            | "dry I_11"
            | "dry I_33"
            | "propellant mass, column (of initial)" => 1e-12,
            "products of inertia (of I_11)" | "products of inertia at knots" => 1e-15,
            "total mass at knots"
            | "centre of mass at knots"
            | "I_11 about the dry centre at knots"
            | "I_22 about the dry centre at knots"
            | "I_11 about the centre of mass at knots"
            | "I_33 at knots"
            | "propellant mass, grains, at knots" => 1e-8,
            "total mass" => 5e-5,
            "centre of mass (of length)" => 2e-5,
            "I_11 about the dry centre"
            | "I_22 about the dry centre"
            | "I_11 about the centre of mass"
            | "I_33" => 1e-4,
            "propellant mass, grains (of initial)" => 2.5e-4,
            other => panic!("no tolerance for {other}"),
        };
        for (what, error, at) in &worst.0 {
            println!("{what}: {error:.3e} ({at})");
        }
        assert_eq!(worst.0.len(), 22, "every quantity was compared");
        for (what, error, at) in &worst.0 {
            assert!(*error <= tolerance(what), "{what}: {error:e} at {at}");
        }
    }

    /// The synthetic two-stage design with its sustainer lit by `ignition`: a J760 in the
    /// booster (motor 0), an I175 in the sustainer (motor 1).
    fn two_stage(ignition: Ignition) -> Rocket {
        let mut rocket: Rocket = serde_json::from_str(include_str!(
            "../../../validation/designs/synthetic-two-stage-75mm-54mm.json"
        ))
        .unwrap();
        rocket.configurations[0].motors[1].ignition = ignition;
        rocket
    }

    /// A sustainer lit by a clustered booster's burnout lights at the burnout of the booster's
    /// first motor that lights, so one motor out doesn't hold it back; with every booster motor
    /// out it never lights, and that is not the cycle of burnouts assembly refuses.
    #[test]
    fn a_burnout_of_a_cluster_is_its_first_motor_that_lights() {
        let mut rocket = two_stage(Ignition::Burnout {
            mount: "booster-motor-mount".to_owned(),
            delay_s: 1.0,
        });
        let booster = rocket.stages[1].components[1]
            .children
            .iter_mut()
            .find(|c| c.id == "booster-motor-mount")
            .expect("the booster's mount");
        if let crate::Part::InnerTube(tube) = &mut booster.part {
            tube.cluster_m = vec![[-0.01, 0.0], [0.01, 0.0]];
        }
        let booster_burnout_s =
            |assembly: &Assembly| assembly.motors[0].mounted.motor.burnout_time_s();
        rocket.configurations[0].motors[0].failed_tubes = vec![0];
        let assembly = rocket.assemble("j760-i175").unwrap();
        let burnout_s = booster_burnout_s(&assembly);
        assert_eq!(
            assembly.ignition_times_s(|_| None),
            [None, Some(0.0), Some(burnout_s + 1.0)]
        );
        rocket.configurations[0].motors[0].failed_tubes = vec![1, 0];
        let assembly = rocket.assemble("j760-i175").unwrap();
        assert_eq!(assembly.ignition_times_s(|_| None), [None, None, None]);
    }

    #[test]
    fn ignition_times_follow_their_events() {
        let j760 = |assembly: &Assembly| assembly.motors[0].mounted.motor.burnout_time_s();
        // At launch, and at a time.
        let assembly = two_stage(Ignition::Launch).assemble("j760-i175").unwrap();
        assert_eq!(assembly.ignition_times_s(|_| None), [Some(0.0), Some(0.0)]);
        let assembly = two_stage(Ignition::Time { time_s: 3.5 })
            .assemble("j760-i175")
            .unwrap();
        assert_eq!(assembly.ignition_times_s(|_| None), [Some(0.0), Some(3.5)]);
        // At the booster's burnout plus a delay.
        let assembly = two_stage(Ignition::Burnout {
            mount: "booster-motor-mount".to_owned(),
            delay_s: 1.25,
        })
        .assemble("j760-i175")
        .unwrap();
        let burnout_s = j760(&assembly);
        assert!(burnout_s > 1.0, "{burnout_s}");
        assert_eq!(
            assembly.ignition_times_s(|_| None),
            [Some(0.0), Some(burnout_s + 1.25)]
        );
        // At the separation of the stage aft of the sustainer's (stage 0), and never without it.
        let assembly = two_stage(Ignition::Separation { delay_s: 0.5 })
            .assemble("j760-i175")
            .unwrap();
        assert_eq!(assembly.ignition_times_s(|_| None), [Some(0.0), None]);
        assert_eq!(
            assembly.ignition_times_s(|stage| (stage == 0).then_some(4.0)),
            [Some(0.0), Some(4.5)]
        );
        assert_eq!(
            assembly.ignition_times_s(|stage| (stage == 1).then_some(4.0)),
            [Some(0.0), None]
        );
        // A motor not yet lit is loaded; lit, it burns on its own clock.
        let lit = [Some(0.0), Some(4.5)];
        let loaded = assembly.motors[1].mass_properties(0.0).mass_kg;
        let spent = assembly.motors[1].dry_mass_properties().mass_kg;
        let at = |t: f64| assembly.mass_properties_lit(t, &lit).mass_kg;
        let structure = assembly.layout.structure.mass_kg;
        let booster = |t: f64| assembly.motors[0].mass_properties(t).mass_kg;
        assert_eq!(at(4.0), structure + booster(4.0) + loaded);
        let sustainer = assembly.motors[1].mass_properties(1.0).mass_kg;
        assert!(sustainer < loaded && sustainer > spent);
        assert!((at(5.5) - (structure + booster(5.5) + sustainer)).abs() < 1e-12);
        assert!((at(100.0) - assembly.dry_mass_properties().mass_kg).abs() < 1e-12);
    }

    #[test]
    fn bad_ignitions_are_refused() {
        let refused = |ignition: Ignition| two_stage(ignition).assemble("j760-i175").is_err();
        assert!(refused(Ignition::Time { time_s: -1.0 }));
        assert!(refused(Ignition::Time { time_s: f64::NAN }));
        assert!(refused(Ignition::Separation {
            delay_s: f64::INFINITY
        }));
        // The booster is the last stage: nothing is aft of it to separate.
        let mut rocket = two_stage(Ignition::Launch);
        rocket.configurations[0].motors[0].ignition = Ignition::Separation { delay_s: 0.0 };
        let error = rocket.assemble("j760-i175").unwrap_err();
        assert!(matches!(error, DesignError::Tree { .. }), "{error:?}");
        assert!(refused(Ignition::Burnout {
            mount: "booster-motor-mount".to_owned(),
            delay_s: -0.1,
        }));
        // A mount with no motor, or no such mount.
        assert!(refused(Ignition::Burnout {
            mount: "nose".to_owned(),
            delay_s: 0.0,
        }));
        // Its own burnout, and a cycle through the other motor.
        assert!(refused(Ignition::Burnout {
            mount: "sustainer-motor-mount".to_owned(),
            delay_s: 0.0,
        }));
        let mut rocket = two_stage(Ignition::Burnout {
            mount: "booster-motor-mount".to_owned(),
            delay_s: 0.0,
        });
        rocket.configurations[0].motors[0].ignition = Ignition::Burnout {
            mount: "sustainer-motor-mount".to_owned(),
            delay_s: 0.0,
        };
        let error = rocket.assemble("j760-i175").unwrap_err();
        assert!(matches!(error, DesignError::Tree { .. }), "{error:?}");
        // A launch is written as nothing at all, so older designs read the same.
        let json = serde_json::to_string(&two_stage(Ignition::Launch)).unwrap();
        assert!(!json.contains("ignition"));
        let timed = serde_json::to_string(&two_stage(Ignition::Time { time_s: 2.0 })).unwrap();
        assert!(
            timed.contains(r#""ignition":{"time":{"time_s":2.0}}"#),
            "{timed}"
        );
    }
}
