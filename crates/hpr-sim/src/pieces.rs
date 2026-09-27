//! Ejected pieces: an airframe that comes apart at any joint, or lets a payload out, and the pieces
//! that fly on to their own landings.
//!
//! A [`Separation`] parts the stack at a stage boundary. An [`Ejection`] parts it anywhere else:
//! at the joint aft of any body component (a nose cone pushed off its airframe, say), or around a
//! payload carried inside (an internal component and everything inside it). Each has a trigger, as
//! a recovery device does. The pieces are fixed before the flight by where the airframe can part.
//! At any moment a body is the pieces still joined, and its mass is the sum of theirs (the
//! decision record on ejected pieces, [ADR-085][adr-085], which extends the one on separation,
//! [ADR-014][adr-014]).
//!
//! Method: `docs/physics/recovery.md`, *Ejected pieces*.
//!
//! [adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
//! [adr-085]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-085-ejected-pieces-an-airframe-that-parts-at-any-joint-2026-09-26

use hpr_design::{Assembly, Component, MassProperties, Rocket};
use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::recovery::{DeviceDrag, Separation, Trigger};

/// Where an airframe parts at an [`Ejection`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Parting {
    /// At the joint just aft of the body component with this id. Everything aft of the joint, up
    /// to the next joint that can part, is a new piece. The part forward of it keeps its number,
    /// so an ejected nose cone is the forward side of the joint after the nose.
    AftOf {
        /// The id of the body component forward of the joint.
        component: String,
    },
    /// Around the internal component with this id: it and everything inside it leave the piece
    /// that carries it, as a payload pushed out of a body tube does. It must be inside the
    /// airframe, one part (not one copy of a cluster's), and not inside another payload.
    Payload {
        /// The id of the internal component that leaves.
        component: String,
    },
}

/// A piece of the airframe leaving the rest on a trigger: the airframe parts at a joint, or lets a
/// payload out.
///
/// The piece it makes flies on as a body of its own, numbered after the separation's aft body:
/// with a [`Separation`] the first ejection makes body 2, without one body 1, and each ejection
/// after it the next number, in the order given. A body is the pieces still joined, and it is
/// numbered by the piece nearest the nose among them, so the body that keeps the nose is always
/// body 0. Pieces joined by a shock cord fly as one, so a joint whose pieces stay tied together
/// is not an ejection at all.
///
/// When the airframe first comes apart each body starts at its own centre of mass with the
/// velocity that point had. A body already flying as a point mass has no attitude to place its
/// pieces by, so they start where it was, at its velocity. Every motor must have burned out by the
/// time an ejection fires. The decision record on ejected pieces, [ADR-085][adr-085], has the
/// reasoning.
///
/// An ejection can push its two sides apart with an impulse `J` ([`Self::with_impulse`]), the
/// charge's or spring's: the side forward of the joint, or a payload, which leaves forward, takes
/// `+J` along the airframe's axis toward the nose, and the other side `−J`, so each changes
/// velocity by `J/m` for its own mass `m` and the momentum is unchanged. The axis is the
/// airframe's while it flies with nothing open. A body with no attitude of its own (a point mass,
/// or a stack whose attitude froze when a device opened earlier) is taken to point its forward end
/// against its velocity through the air if it hangs from a device, any but a tumble, open just
/// before that instant, and along it, as a stable airframe does, if nothing is open or it only
/// tumbles; below 1 mm/s through the air, up ([ADR-086][adr-086]). Partings at one instant part a
/// body together. A separation adds no impulse, and a pushed payload in the nose's piece is
/// refused.
///
/// [adr-085]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-085-ejected-pieces-an-airframe-that-parts-at-any-joint-2026-09-26
/// [adr-086]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-086-ejection-impulse-and-tumbling-pieces-2026-09-26
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Ejection {
    /// When the piece leaves: the same triggers as a recovery device's.
    pub trigger: Trigger,
    /// Where the airframe parts.
    pub parting: Parting,
    /// The impulse `J` that pushes the two sides apart, N·s: zero (the default) for none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub impulse_n_s: f64,
}

/// Whether an impulse is zero, so a file without one writes none.
fn is_zero(impulse_n_s: &f64) -> bool {
    *impulse_n_s == 0.0
}

impl Ejection {
    /// The airframe parting at the joint aft of body component `component`, on `trigger`.
    #[must_use]
    pub fn aft_of(trigger: Trigger, component: impl Into<String>) -> Self {
        Self {
            trigger,
            parting: Parting::AftOf {
                component: component.into(),
            },
            impulse_n_s: 0.0,
        }
    }

    /// Internal component `component` and what it holds leaving the airframe, on `trigger`.
    #[must_use]
    pub fn payload(trigger: Trigger, component: impl Into<String>) -> Self {
        Self {
            trigger,
            parting: Parting::Payload {
                component: component.into(),
            },
            impulse_n_s: 0.0,
        }
    }

    /// The same ejection, pushing its two sides apart with the impulse `impulse_n_s`, N·s: the
    /// side forward of the joint, or the payload, gets it toward the nose, and the other side its
    /// opposite. [`crate::Simulation::with_ejections`] refuses one that is negative or not
    /// finite. A pushed payload needs a way out forward: one in the nose's piece is refused when
    /// the flight starts, and one whose section's forward joint hasn't parted when it leaves is
    /// an error in flight.
    #[must_use]
    pub fn with_impulse(mut self, impulse_n_s: f64) -> Self {
        self.impulse_n_s = impulse_n_s;
        self
    }
}

/// The pieces a design can come apart into, fixed before the flight by its separation and its
/// ejections.
///
/// Piece 0 is the one with the nose. The separation, when there is one, makes piece 1, and
/// ejection `k` makes the piece after it. A split's index in that order is its piece's minus one.
#[derive(Debug, Clone)]
pub(crate) struct Pieces {
    /// The piece that carries each piece, when it is a payload; `None` for a section.
    host: Vec<Option<usize>>,
    /// The section pieces, nose to tail.
    sections: Vec<usize>,
    /// The structure of each piece: mass properties that add up to it, motors not included.
    structure: Vec<Vec<MassProperties>>,
    /// The piece of each placed motor.
    motor_piece: Vec<usize>,
    /// The first and last stage each piece has a component in.
    stages: Vec<(usize, usize)>,
    /// The piece of each placed component.
    piece_of: Vec<usize>,
}

impl Pieces {
    /// The pieces of `assembly` (assembled from `rocket`) under `separation` and `ejections`.
    ///
    /// # Errors
    ///
    /// [`SimError::Parting`] for an ejection that names a component the design doesn't have, a
    /// joint aft of an internal component or of the last body component, a payload that is a body
    /// component, an external part, one of several copies in a cluster or inside another payload,
    /// two partings at one joint or of one payload, or a split through a stage or component whose
    /// overridden mass doesn't say how it divides; [`SimError::Domain`] for a separation with no
    /// stage aft of it.
    pub(crate) fn new(
        rocket: &Rocket,
        assembly: &Assembly,
        separation: Option<Separation>,
        ejections: &[Ejection],
    ) -> Result<Self, SimError> {
        let components = &assembly.layout.components;
        let refuse = |what: &'static str, component: &str| SimError::Parting {
            what,
            component: component.to_owned(),
        };
        let find = |id: &str| {
            components
                .iter()
                .position(|component| component.id == id)
                .ok_or_else(|| refuse("an ejection names a component the design doesn't have", id))
        };
        let top: Vec<usize> = (0..components.len())
            .filter(|&index| components[index].parent.is_none())
            .collect();
        let count = 1 + usize::from(separation.is_some()) + ejections.len();

        // Where each section starts, as a position in `top`, and each payload.
        let mut starts: Vec<(usize, usize)> = Vec::new();
        let mut payloads: Vec<(usize, usize)> = Vec::new();
        let mut piece = 1;
        if let Some(separation) = separation {
            let Some(at) = top
                .iter()
                .position(|&index| components[index].stage > separation.after_stage)
            else {
                return Err(SimError::Domain {
                    what: "stage boundary of a separation (there is no stage aft of it)",
                    value: separation.after_stage as f64,
                });
            };
            starts.push((at, piece));
            piece += 1;
        }
        for ejection in ejections {
            match &ejection.parting {
                Parting::AftOf { component } => {
                    let index = find(component)?;
                    if components[index].parent.is_some() {
                        return Err(refuse(
                            "a joint is aft of a body component, and this one is inside another \
                             (a part carried inside is a `Parting::Payload`)",
                            component,
                        ));
                    }
                    // `index` is a body component, so it is in `top`.
                    let Some(at) = top.iter().position(|&body| body == index).map(|at| at + 1)
                    else {
                        return Err(refuse(
                            "a body component missing from the layout",
                            component,
                        ));
                    };
                    if at >= top.len() {
                        return Err(refuse(
                            "a joint aft of the last body component (nothing is aft of it)",
                            component,
                        ));
                    }
                    if starts.iter().any(|(start, _)| *start == at) {
                        return Err(refuse(
                            "two partings at the joint aft of this component (a separation's \
                             stage boundary is a joint too)",
                            component,
                        ));
                    }
                    starts.push((at, piece));
                }
                Parting::Payload { component } => {
                    let index = find(component)?;
                    if components[index].parent.is_none() {
                        return Err(refuse(
                            "a payload is carried inside the airframe, and this is a body \
                             component (a body component leaves at a joint, `Parting::AftOf`)",
                            component,
                        ));
                    }
                    if components[index].body_radius_m.is_some() {
                        return Err(refuse(
                            "a payload is carried inside the airframe, and this part is outside it",
                            component,
                        ));
                    }
                    if components[index].copies_m.len() != 1 {
                        return Err(refuse(
                            "a payload that isn't exactly one part (one of several copies in a \
                             cluster of tubes, or none)",
                            component,
                        ));
                    }
                    if payloads.iter().any(|(payload, _)| *payload == index) {
                        return Err(refuse("two ejections of one payload", component));
                    }
                    payloads.push((index, piece));
                }
            }
            piece += 1;
        }

        // A payload inside another would need a piece carried by a piece; not flown yet.
        for &(payload, _) in &payloads {
            let mut ancestor = components[payload].parent;
            while let Some(index) = ancestor {
                if payloads.iter().any(|(other, _)| *other == index) {
                    return Err(refuse(
                        "a payload inside another payload",
                        &components[payload].id,
                    ));
                }
                ancestor = components[index].parent;
            }
        }

        // Every component's piece. A parent is placed before its children, so its piece is known.
        let mut piece_of = vec![0; components.len()];
        let mut host = vec![None; count];
        let mut sections = vec![0];
        let mut section = 0;
        let mut position = 0;
        for index in 0..components.len() {
            let carried = match components[index].parent {
                None => {
                    if let Some(&(_, start)) = starts.iter().find(|(at, _)| *at == position) {
                        section = start;
                        sections.push(start);
                    }
                    position += 1;
                    section
                }
                Some(parent) => piece_of[parent],
            };
            piece_of[index] = match payloads.iter().find(|(payload, _)| *payload == index) {
                Some(&(_, payload)) => {
                    host[payload] = Some(carried);
                    payload
                }
                None => carried,
            };
        }

        // Whether a component's subtree spans more than one piece.
        let mut mixed = vec![false; components.len()];
        for index in (0..components.len()).rev() {
            if let Some(parent) = components[index].parent
                && (mixed[index] || piece_of[index] != piece_of[parent])
            {
                mixed[parent] = true;
            }
        }
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); components.len()];
        for (index, component) in components.iter().enumerate() {
            if let Some(parent) = component.parent {
                children[parent].push(index);
            }
        }

        // The structure: a stage wholly in one piece by its own mass, which carries the stage's
        // overrides; otherwise its components, each by its subtree's mass where that subtree is in
        // one piece and by its own mass plus its children's where it isn't.
        let mut structure: Vec<Vec<MassProperties>> = vec![Vec::new(); count];
        for (stage, placed) in assembly.layout.stages.iter().enumerate() {
            let pieces: Vec<usize> = (0..components.len())
                .filter(|&index| components[index].stage == stage)
                .map(|index| piece_of[index])
                .collect();
            if let Some(&first) = pieces.first()
                && pieces.iter().all(|&piece| piece == first)
            {
                structure[first].push(placed.mass);
                continue;
            }
            if rocket
                .stages
                .iter()
                .find(|stage| stage.id == placed.id)
                .is_some_and(|stage| !stage.overrides.is_empty())
            {
                return Err(refuse(
                    "a parting through a stage whose mass is overridden (the override doesn't \
                     say which piece its mass is in)",
                    &placed.id,
                ));
            }
            let mut pending: Vec<usize> = top
                .iter()
                .copied()
                .filter(|&index| components[index].stage == stage)
                .rev()
                .collect();
            while let Some(index) = pending.pop() {
                let component = &components[index];
                if !mixed[index] {
                    structure[piece_of[index]].push(component.with_children);
                    continue;
                }
                if node(rocket, &component.id).is_some_and(|node| {
                    node.overrides_include_children && !node.overrides.is_empty()
                }) {
                    return Err(refuse(
                        "a parting inside a component whose overridden mass covers what it holds \
                         (the override doesn't say which piece that mass is in)",
                        &component.id,
                    ));
                }
                structure[piece_of[index]].push(component.own);
                pending.extend(children[index].iter().rev());
            }
        }

        let motor_piece = assembly
            .motors
            .iter()
            .map(|motor| {
                components
                    .iter()
                    .position(|component| component.id == motor.mount)
                    .map(|index| piece_of[index])
                    .ok_or_else(|| refuse("a motor's mount is not in the design", &motor.mount))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut stages = vec![(usize::MAX, 0); count];
        for (index, component) in components.iter().enumerate() {
            let (first, last) = &mut stages[piece_of[index]];
            *first = (*first).min(component.stage);
            *last = (*last).max(component.stage);
        }
        for (first, last) in &mut stages {
            if *first == usize::MAX {
                // A piece with no component can't happen: every parting leaves one on each side.
                (*first, *last) = (0, 0);
            }
        }

        Ok(Self {
            host,
            sections,
            structure,
            motor_piece,
            stages,
            piece_of,
        })
    }

    /// How many pieces there are.
    pub(crate) fn count(&self) -> usize {
        self.host.len()
    }

    /// The body each piece is in when the splits with `open[split]` have happened: the body's
    /// number is its lead piece's, the section nearest the nose among its pieces, or the payload
    /// itself for a payload flying alone.
    pub(crate) fn leaders(&self, open: &[bool]) -> Vec<usize> {
        let mut leader: Vec<usize> = (0..self.count()).collect();
        let mut current = 0;
        for &section in &self.sections {
            if section == 0 || open[section - 1] {
                current = section;
            }
            leader[section] = current;
        }
        for piece in 0..self.count() {
            leader[piece] = self.lead(piece, open, &leader);
        }
        leader
    }

    /// The lead piece of `piece`'s body, with the sections' in `sections_led`.
    fn lead(&self, piece: usize, open: &[bool], sections_led: &[usize]) -> usize {
        match self.host[piece] {
            None => sections_led[piece],
            Some(_) if open[piece - 1] => piece,
            Some(host) => self.lead(host, open, sections_led),
        }
    }

    /// The mass properties of the pieces for which `member` is true, with their motors at flight
    /// time `t_s`, each lit at its `ignition_s`.
    pub(crate) fn mass_properties(
        &self,
        member: impl Fn(usize) -> bool,
        assembly: &Assembly,
        t_s: f64,
        ignition_s: &[Option<f64>],
    ) -> MassProperties {
        let mut parts: Vec<MassProperties> = Vec::new();
        for (piece, structure) in self.structure.iter().enumerate() {
            if member(piece) {
                parts.extend(structure);
            }
        }
        for ((motor, ignition), piece) in assembly
            .motors
            .iter()
            .zip(ignition_s)
            .zip(&self.motor_piece)
        {
            if member(*piece) {
                parts.push(motor.mass_properties_lit(t_s, *ignition));
            }
        }
        MassProperties::combine(parts.iter())
    }

    /// Refuses a pushed payload in the nose's piece: it leaves forward, and that piece is closed at
    /// the nose. It needs the flight's separation as well as its `ejections`, so it runs when the
    /// flight starts, whatever order the builders came in.
    ///
    /// # Errors
    ///
    /// [`SimError::Parting`], naming the payload.
    pub(crate) fn check_pushed_payloads(
        &self,
        ejections: &[Ejection],
        first_ejection: usize,
    ) -> Result<(), SimError> {
        for (index, ejection) in ejections.iter().enumerate() {
            if let Parting::Payload { component } = &ejection.parting
                && ejection.impulse_n_s > 0.0
                && self.host[first_ejection + index] == Some(0)
            {
                return Err(SimError::Parting {
                    what: "a pushed payload leaves forward, and this one is in the nose's piece, \
                           which is closed at the nose (part a joint forward of it, or give it no \
                           impulse)",
                    component: component.clone(),
                });
            }
        }
        Ok(())
    }

    /// The piece that carries `piece`, when it is a payload.
    pub(crate) fn host(&self, piece: usize) -> Option<usize> {
        self.host[piece]
    }

    /// The piece on the other side of the split that makes `piece` (not piece 0), and whether
    /// `piece` is forward of that split: a section is aft of its joint, with the section forward
    /// of the joint on the other side, and a payload leaves its host forward.
    pub(crate) fn across(&self, piece: usize) -> (usize, bool) {
        match self.host[piece] {
            Some(host) => (host, true),
            None => {
                let at = self.sections.iter().position(|&section| section == piece);
                // Every section is in the list, piece 0 first; piece 0 makes no split, so the
                // section a split makes has one ahead of it.
                debug_assert!(
                    at.is_some_and(|at| at > 0),
                    "piece {piece} is no split's section"
                );
                (self.sections[at.unwrap_or(0).saturating_sub(1)], false)
            }
        }
    }

    /// The drag area of piece `piece` tumbling on its own: its own components' body profile and
    /// fins ([`DeviceDrag::tumbling_where`]).
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for a piece that isn't one, or a payload (it is inside the airframe,
    /// with no body tube or fin of its own for the model to take), and as
    /// [`DeviceDrag::tumbling`].
    pub(crate) fn tumbling(
        &self,
        piece: usize,
        assembly: &Assembly,
    ) -> Result<DeviceDrag, SimError> {
        if piece >= self.count() {
            return Err(SimError::Domain {
                what: "piece to tumble (there is one per split, and the nose's)",
                value: piece as f64,
            });
        }
        if self.host[piece].is_some() {
            return Err(SimError::Domain {
                what: "piece to tumble (a payload has no body tube or fin of its own, which the \
                       tumble model covers)",
                value: piece as f64,
            });
        }
        DeviceDrag::tumbling_where(assembly, |index, _| self.piece_of[index] == piece)
    }

    /// The first and last stage the pieces for which `member` is true have a component in.
    pub(crate) fn stages(&self, member: impl Fn(usize) -> bool) -> (usize, usize) {
        let mut span: Option<(usize, usize)> = None;
        for (piece, &(first, last)) in self.stages.iter().enumerate() {
            if member(piece) {
                span = Some(span.map_or((first, last), |(a, b)| (a.min(first), b.max(last))));
            }
        }
        span.unwrap_or((0, 0))
    }
}

/// The design's component with id `id`, searched through every stage.
fn node<'a>(rocket: &'a Rocket, id: &str) -> Option<&'a Component> {
    fn within<'a>(components: &'a [Component], id: &str) -> Option<&'a Component> {
        components.iter().find_map(|component| {
            if component.id == id {
                Some(component)
            } else {
                within(&component.children, id)
            }
        })
    }
    rocket
        .stages
        .iter()
        .find_map(|stage| within(&stage.components, id))
}

#[cfg(test)]
mod tests {
    use hpr_atmos::{ConstantWind, Wind};
    use hpr_core::DVec3;
    use hpr_design::{Part, Position};

    use super::*;
    use crate::flight::{EventKind, FlightResult, FlightSettings, Simulation, Termination};
    use crate::rail::Rail;
    use crate::recovery::{
        BodyEvent, CanopyType, Device, DeviceDrag, Inflation, terminal_speed_m_s,
    };
    use crate::state::State;
    use crate::testing::{UniformAir, analytic_environment, analytic_wind_environment, design};

    const G: f64 = 9.806_65;
    /// The I175 burns out at 2.5 s; a drop starts well after that.
    const START_S: f64 = 10.0;
    const PAYLOAD_KG: f64 = 0.25;

    /// The 54 mm single-stage test design with a 0.25 kg payload carried in its airframe, 0.15 m
    /// aft of the airframe's forward end.
    fn with_payload() -> Rocket {
        let mut rocket = design("synthetic-54mm-three-fin");
        let airframe = &mut rocket.stages[0].components[1];
        assert_eq!(airframe.id, "sustainer-airframe");
        let mut payload = airframe
            .children
            .iter()
            .find(|child| child.id == "altimeter")
            .cloned()
            .unwrap();
        payload.id = "payload".to_owned();
        let Part::MassComponent(mass) = &mut payload.part else {
            panic!("the altimeter is a mass component");
        };
        mass.mass_kg = PAYLOAD_KG;
        payload.position = Some(Position::Top { aft_offset_m: 0.15 });
        airframe.children.push(payload);
        rocket
    }

    /// A canopy of `diameter_m` on `body`, fired by `trigger`.
    fn canopy(body: usize, diameter_m: f64, trigger: Trigger) -> Device {
        Device::new(
            "canopy",
            DeviceDrag::canopy(CanopyType::FlatCircular, diameter_m),
            trigger,
        )
        .on_body(body)
    }

    /// The nose cone under its own 0.45 m canopy from apogee, the airframe under a 0.9 m one from
    /// apogee, and the payload under a 0.6 m one as it leaves at 300 m.
    fn devices() -> Vec<Device> {
        let payload_height = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        vec![
            canopy(0, 0.45, Trigger::Apogee),
            canopy(1, 0.9, Trigger::Apogee),
            canopy(2, 0.6, payload_height),
        ]
    }

    /// The nose cone pushed off at apogee, and the payload out of the airframe at 300 m.
    fn ejections() -> Vec<Ejection> {
        vec![
            Ejection::aft_of(Trigger::Apogee, "nose"),
            Ejection::payload(
                Trigger::Altitude {
                    height_above_ground_m: 300.0,
                },
                "payload",
            ),
        ]
    }

    fn simulation(environment: crate::Environment) -> Simulation {
        Simulation::new(
            &with_payload(),
            "i175",
            environment,
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices())
        .unwrap()
        .with_ejections(ejections())
        .unwrap()
    }

    /// The state with the centre of mass `height_m` above the site, nose up, moving at
    /// `velocity_enu_m_s` and turning at `body_rate_rad_s`.
    fn dropped(
        sim: &Simulation,
        height_m: f64,
        velocity_enu_m_s: DVec3,
        body_rate_rad_s: DVec3,
    ) -> State {
        let attitude = Rail::vertical(3.0).attitude();
        let cg_m = sim.assembly().mass_properties(START_S).cg_m;
        State {
            position_enu_m: DVec3::new(0.0, 0.0, height_m) - attitude.mul_vec3(cg_m),
            velocity_enu_m_s,
            attitude,
            body_rate_rad_s,
        }
    }

    /// The mass of the layout's component `id` with everything inside it, kg.
    fn component_kg(sim: &Simulation, id: &str) -> f64 {
        sim.assembly()
            .layout
            .components
            .iter()
            .find(|component| component.id == id)
            .unwrap()
            .with_children
            .mass_kg
    }

    fn close(a: f64, b: f64, relative: f64, what: &str) {
        assert!((a - b).abs() <= relative * b.abs(), "{what}: {a} vs {b}");
    }

    #[test]
    fn an_ejected_nose_cone_and_payload_each_land_under_their_own_canopy() {
        // The milestone's design, flown from the pad in a 4 m/s wind from the west: the nose cone
        // leaves at apogee, the payload at 300 m, and each piece reaches the ground on its own,
        // somewhere of its own.
        let wind = ConstantWind::new(4.0, 1.5 * std::f64::consts::PI).unwrap();
        let sim = simulation(analytic_wind_environment(UniformAir::sea_level(), G, wind));
        let result = sim.run(&mut ()).unwrap();
        assert_eq!(result.termination, Termination::Separated);
        let apogee = result.event(EventKind::Apogee).unwrap().sample;
        let ejected = result.event(EventKind::Ejection(0)).unwrap().sample;
        assert!((ejected.time_s - apogee.time_s).abs() < 1e-9, "{ejected:?}");
        assert!(result.event(EventKind::Ejection(1)).is_none());
        assert_eq!(result.bodies.len(), 3);
        for (index, body) in result.bodies.iter().enumerate() {
            assert_eq!(body.body, index);
            assert_eq!(body.termination, Termination::GroundHit, "body {index}");
            assert_eq!(body.pieces, vec![index]);
            // The nose cone's canopy opens on the stack in the pass that parts it, so its
            // deployment is the flight's; the others open on their own bodies.
            let deployment = EventKind::Deployment(index);
            assert!(
                body.event(deployment).is_some() || result.event(deployment).is_some(),
                "body {index}"
            );
        }
        // The payload leaves the airframe's body at its height, and starts its own flight there.
        let airframe = &result.bodies[1];
        let payload = &result.bodies[2];
        let parting = airframe.event(EventKind::Ejection(1)).unwrap().sample;
        assert!(
            (parting.height_above_ground_m - 300.0).abs() < 1e-6,
            "{parting:?}"
        );
        assert_eq!(payload.start_sample.time_s, parting.time_s);
        assert!(result.bodies_landed());
        let landings = result.landings();
        assert_eq!(landings.len(), 3);
        for landing in &landings {
            assert!(landing.height_above_ground_m.abs() < 1e-6, "{landing:?}");
        }
        // Measured: apogee at 14.985 s; the nose cone (0.063 kg) lands at 559.21 s, 2 048 m
        // downwind, the airframe (0.556 kg) at 331.76 s and the payload (0.250 kg), let out at
        // 261.17 s, at 331.33 s, both near 1 135 m.
        let pinned = [
            (559.212, 2_047.87),
            (331.760, 1_136.24),
            (331.332, 1_134.53),
        ];
        for (landing, (time_s, east_m)) in landings.iter().zip(pinned) {
            assert!((landing.time_s - time_s).abs() < 0.05, "{landing:?}");
            assert!((landing.cg_enu_m.x - east_m).abs() < 0.5, "{landing:?}");
        }
        // Under a canopy a piece drifts with the wind, so the nose cone's extra time aloft puts it
        // that many seconds of wind further on: 911.6 m against 4 m/s × 227.5 s = 909.8 m.
        let apart_m = landings[0].cg_enu_m.x - landings[1].cg_enu_m.x;
        let drift_m = 4.0 * (landings[0].time_s - landings[1].time_s);
        close(apart_m, drift_m, 0.01, "the nose cone's extra drift");
    }

    #[test]
    fn the_pieces_masses_add_up_and_each_split_conserves_momentum() {
        // Split with wind, a sideways velocity and a body rate, so that `ω × r` is in each
        // body's start and the check is not the trivial one.
        let air = UniformAir::sea_level();
        let sim = simulation(analytic_wind_environment(
            air,
            G,
            ConstantWind::new(5.0, 0.9).unwrap(),
        ));
        let start = dropped(
            &sim,
            1_500.0,
            DVec3::new(3.0, 0.0, -2.0),
            DVec3::new(0.0, 0.6, 0.0),
        );
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let [nose, airframe, payload] = &result.bodies[..] else {
            panic!("{} bodies", result.bodies.len());
        };

        // At the first split the two bodies are the whole rocket, and the nose cone and the
        // payload are their own components' masses.
        let first = result.event(EventKind::Ejection(0)).unwrap().sample;
        let whole = sim.assembly().mass_properties(first.time_s);
        close(
            nose.start_sample.mass_kg + airframe.start_sample.mass_kg,
            whole.mass_kg,
            1e-12,
            "the two bodies' mass",
        );
        close(
            nose.mass_kg,
            component_kg(&sim, "nose"),
            1e-12,
            "the nose cone",
        );
        close(payload.mass_kg, PAYLOAD_KG, 1e-12, "the payload");
        close(
            nose.mass_kg + airframe.mass_kg + payload.mass_kg,
            whole.mass_kg,
            1e-12,
            "the three pieces' mass",
        );

        // Momentum at the first split: each body leaves with its own centre of mass's velocity,
        // and together they carry the stack's.
        let momentum = nose.start_sample.cg_velocity_enu_m_s * nose.start_sample.mass_kg
            + airframe.start_sample.cg_velocity_enu_m_s * airframe.start_sample.mass_kg;
        let expected = first.cg_velocity_enu_m_s * whole.mass_kg;
        assert!(
            (momentum - expected).length() < 1e-9 * expected.length(),
            "{momentum} vs {expected}"
        );
        // Each starts where its own centre of mass was: the nose cone's is its component's.
        let state = result.final_sample.state;
        let nose_cg_m = sim
            .assembly()
            .layout
            .components
            .iter()
            .find(|component| component.id == "nose")
            .unwrap()
            .with_children
            .cg_m;
        let expected = state.point_enu_m(nose_cg_m);
        assert!(
            (nose.start_sample.cg_enu_m - expected).length() < 1e-12,
            "{} vs {expected}",
            nose.start_sample.cg_enu_m
        );
        let gap = (nose.start_sample.cg_enu_m - airframe.start_sample.cg_enu_m).length();
        assert!(gap > 0.3, "{gap}");

        // Momentum at the second, on the way down: the payload leaves the airframe's body at
        // its point and velocity, and the masses before and after agree.
        let parting = airframe.event(EventKind::Ejection(1)).unwrap().sample;
        close(
            parting.mass_kg,
            airframe.mass_kg + payload.mass_kg,
            1e-12,
            "the airframe's body before the payload left",
        );
        assert_eq!(payload.start_sample.cg_enu_m, parting.cg_enu_m);
        let before = parting.cg_velocity_enu_m_s * parting.mass_kg;
        let after = parting.cg_velocity_enu_m_s * airframe.mass_kg
            + payload.start_sample.cg_velocity_enu_m_s * payload.mass_kg;
        assert!(
            (after - before).length() < 1e-9 * before.length(),
            "{after} vs {before}"
        );
    }

    #[test]
    fn each_piece_comes_down_at_its_own_terminal_speed() {
        // In uniform air each body ends its descent at `√(2 m g/(ρ C_D S))` for its own mass and
        // canopy: the airframe at its mass after the payload left.
        let air = UniformAir::sea_level();
        let sim = simulation(analytic_environment(air, G));
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert!(result.bodies_landed());
        let rho = air.0.density_kg_m3;
        for body in &result.bodies {
            let landing = body.event(EventKind::GroundHit).unwrap().sample;
            let drag_area_m2 = sim.recovery()[body.body].drag.drag_area_m2();
            let terminal_m_s = terminal_speed_m_s(body.mass_kg, drag_area_m2, rho, G);
            close(
                -landing.vertical_speed_m_s,
                terminal_m_s,
                1e-3,
                &format!("body {}", body.body),
            );
            assert_eq!(landing.mass_kg, body.mass_kg);
            assert!((landing.recovery_drag_area_m2 - drag_area_m2).abs() < 1e-12);
        }
    }

    /// The milestone's design and devices, with `ejections` in place of its own.
    fn simulation_with(environment: crate::Environment, ejections: Vec<Ejection>) -> Simulation {
        Simulation::new(
            &with_payload(),
            "i175",
            environment,
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices())
        .unwrap()
        .with_ejections(ejections)
        .unwrap()
    }

    /// The same ejections as [`ejections`], each pushing with `impulse_n_s`.
    fn pushed(impulse_n_s: f64) -> Vec<Ejection> {
        ejections()
            .into_iter()
            .map(|ejection| ejection.with_impulse(impulse_n_s))
            .collect()
    }

    fn close_vec(a: DVec3, b: DVec3, relative: f64, what: &str) {
        assert!(
            (a - b).length() <= relative * b.length(),
            "{what}: {a} vs {b}"
        );
    }

    /// As [`dropped`], from the attitude of a rail at `elevation_deg` and `azimuth_deg`.
    fn dropped_at(
        sim: &Simulation,
        (elevation_deg, azimuth_deg): (f64, f64),
        height_m: f64,
        velocity_enu_m_s: DVec3,
        body_rate_rad_s: DVec3,
    ) -> State {
        let attitude = Rail {
            elevation_rad: elevation_deg.to_radians(),
            azimuth_rad: azimuth_deg.to_radians(),
            ..Rail::vertical(3.0)
        }
        .attitude();
        let cg_m = sim.assembly().mass_properties(START_S).cg_m;
        State {
            position_enu_m: DVec3::new(0.0, 0.0, height_m) - attitude.mul_vec3(cg_m),
            velocity_enu_m_s,
            attitude,
            body_rate_rad_s,
        }
    }

    /// The change in the velocity each body starts with, between two flights that are the same
    /// until they part.
    fn start_change(pushed: &FlightResult, unpushed: &FlightResult, body: usize) -> DVec3 {
        pushed.bodies[body].start_sample.cg_velocity_enu_m_s
            - unpushed.bodies[body].start_sample.cg_velocity_enu_m_s
    }

    #[test]
    fn an_impulse_changes_each_pieces_velocity_by_j_over_m_and_conserves_momentum() {
        // 1 N·s on each ejection, in wind, from a stack tilted 60° up toward 30° east of north,
        // moving sideways and turning at 0.6 rad/s. The first parting, from the stack in free
        // flight, pushes along its axis. At the second the airframe hangs under its canopy, so
        // the payload leaves up its velocity through the air, toward the canopy.
        const J: f64 = 1.0;
        let wind = ConstantWind::new(5.0, 0.9).unwrap();
        let wind_enu = wind.wind(0.0).unwrap().velocity_enu_m_s;
        let fly = |impulse_n_s: f64| {
            let environment = analytic_wind_environment(UniformAir::sea_level(), G, wind);
            let sim = simulation_with(environment, pushed(impulse_n_s));
            let start = dropped_at(
                &sim,
                (60.0, 30.0),
                1_500.0,
                DVec3::new(3.0, 0.0, -2.0),
                DVec3::new(0.0, 0.6, 0.0),
            );
            let result = sim.run_free(START_S, start, &mut ()).unwrap();
            (sim, result)
        };
        let (sim, pushed) = fly(J);
        let (_, unpushed) = fly(0.0);
        let [nose, airframe, payload] = &pushed.bodies[..] else {
            panic!("{} bodies", pushed.bodies.len());
        };

        // The first parting: by hand, the axis of a rail 60° up toward 30° east of north is
        // (cos 60° sin 30°, cos 60° cos 30°, sin 60°) in east, north, up. The nose cone, forward
        // of the joint, gains `J/m` along it, and the airframe with the payload inside loses
        // `J/m` for its own mass.
        let first = pushed.event(EventKind::Ejection(0)).unwrap().sample;
        assert_eq!(
            first,
            unpushed.event(EventKind::Ejection(0)).unwrap().sample
        );
        let (elevation, azimuth) = (60_f64.to_radians(), 30_f64.to_radians());
        let axis = DVec3::new(
            elevation.cos() * azimuth.sin(),
            elevation.cos() * azimuth.cos(),
            elevation.sin(),
        );
        let whole = sim.assembly().mass_properties(first.time_s);
        let nose_kg = component_kg(&sim, "nose");
        let rest_kg = whole.mass_kg - nose_kg;
        close(nose.start_sample.mass_kg, nose_kg, 1e-12, "the nose cone");
        close(airframe.start_sample.mass_kg, rest_kg, 1e-12, "the rest");
        close_vec(
            start_change(&pushed, &unpushed, 0),
            axis * (J / nose_kg),
            1e-9,
            "the nose cone's push",
        );
        close_vec(
            start_change(&pushed, &unpushed, 1),
            -axis * (J / rest_kg),
            1e-9,
            "the airframe's push",
        );
        // 1 N·s on the 0.063 kg nose cone is 15.9 m/s.
        close(J / nose_kg, 15.85, 1e-3, "the nose cone's change of speed");
        let momentum = nose.start_sample.cg_velocity_enu_m_s * nose.start_sample.mass_kg
            + airframe.start_sample.cg_velocity_enu_m_s * airframe.start_sample.mass_kg;
        close_vec(
            momentum,
            first.cg_velocity_enu_m_s * whole.mass_kg,
            1e-9,
            "the momentum at the first parting",
        );

        // The second, on the way down: by hand, 1 N·s on the 0.25 kg payload is 4 m/s, against
        // the airframe's velocity through the air.
        let parting = airframe.event(EventKind::Ejection(1)).unwrap();
        let (before, after) = (parting.sample, parting.after.unwrap());
        assert!(before.recovery_drag_area_m2 > 0.0, "{before:?}");
        let through_air = (before.cg_velocity_enu_m_s - wind_enu).normalize();
        close(
            payload.start_sample.mass_kg,
            PAYLOAD_KG,
            1e-12,
            "the payload",
        );
        close(
            after.mass_kg,
            before.mass_kg - PAYLOAD_KG,
            1e-12,
            "the airframe after",
        );
        close_vec(
            payload.start_sample.cg_velocity_enu_m_s - before.cg_velocity_enu_m_s,
            -through_air * 4.0,
            1e-9,
            "the payload's push",
        );
        close_vec(
            after.cg_velocity_enu_m_s - before.cg_velocity_enu_m_s,
            through_air * (J / after.mass_kg),
            1e-9,
            "the airframe's push",
        );
        assert_eq!(after.cg_enu_m, before.cg_enu_m);
        assert_eq!(payload.start_sample.cg_enu_m, before.cg_enu_m);
        close_vec(
            after.cg_velocity_enu_m_s * after.mass_kg
                + payload.start_sample.cg_velocity_enu_m_s * payload.start_sample.mass_kg,
            before.cg_velocity_enu_m_s * before.mass_kg,
            1e-9,
            "the momentum at the second parting",
        );

        // Every piece still lands, each at its own terminal speed once the push has died away.
        assert!(pushed.bodies_landed());
        let rho = UniformAir::sea_level().0.density_kg_m3;
        for body in &pushed.bodies {
            let landing = body.event(EventKind::GroundHit).unwrap().sample;
            let drag_area_m2 = sim.recovery()[body.body].drag.drag_area_m2();
            let terminal_m_s = terminal_speed_m_s(body.mass_kg, drag_area_m2, rho, G);
            close(
                landing.airspeed_m_s,
                terminal_m_s,
                1e-3,
                &format!("body {}", body.body),
            );
        }
    }

    #[test]
    fn a_body_with_nothing_open_is_pushed_along_its_flight() {
        // The airframe's canopy waits for 200 m, so at 300 m it falls with nothing open, nose
        // first as a stable airframe does, and the payload leaves down its velocity through the
        // air.
        let at_200_m = Trigger::Altitude {
            height_above_ground_m: 200.0,
        };
        let sim = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(vec![
            canopy(0, 0.45, Trigger::Apogee),
            canopy(1, 0.9, at_200_m),
            canopy(2, 0.6, at_200_m),
        ])
        .unwrap()
        .with_ejections(pushed(1.0))
        .unwrap();
        let start = dropped(&sim, 1_500.0, DVec3::new(2.0, 1.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let airframe = &result.bodies[1];
        let payload = &result.bodies[2];
        let parting = airframe.event(EventKind::Ejection(1)).unwrap();
        let (before, after) = (parting.sample, parting.after.unwrap());
        assert_eq!(before.recovery_drag_area_m2, 0.0);
        let through_air = before.cg_velocity_enu_m_s.normalize();
        close_vec(
            payload.start_sample.cg_velocity_enu_m_s - before.cg_velocity_enu_m_s,
            through_air * 4.0,
            1e-9,
            "the payload's push",
        );
        close_vec(
            after.cg_velocity_enu_m_s - before.cg_velocity_enu_m_s,
            -through_air * (1.0 / after.mass_kg),
            1e-9,
            "the airframe's push",
        );
        assert!(result.bodies_landed());
    }

    #[test]
    fn a_stack_already_hanging_from_a_device_is_pushed_up_its_flight() {
        // A drogue on the stack at apogee, from a stack tilted 60°, and the nose cone off at
        // 300 m. By then the stack has hung from the drogue for minutes: its attitude is the one
        // frozen at apogee, which says nothing of how it hangs, so the push goes up its velocity
        // through the air, as on a body under a canopy (found in review: it went along the
        // frozen axis, nearly sideways).
        let at_300_m = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        let wind = ConstantWind::new(4.0, 2.0).unwrap();
        let wind_enu = wind.wind(0.0).unwrap().velocity_enu_m_s;
        let fly = |impulse_n_s: f64| {
            let sim = Simulation::new(
                &with_payload(),
                "i175",
                analytic_wind_environment(UniformAir::sea_level(), G, wind),
                Rail::vertical(3.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.3, Trigger::Apogee),
                canopy(1, 0.9, at_300_m),
            ])
            .unwrap()
            .with_ejections(vec![
                Ejection::aft_of(at_300_m, "nose").with_impulse(impulse_n_s),
            ])
            .unwrap();
            let start = dropped_at(
                &sim,
                (60.0, 30.0),
                1_500.0,
                DVec3::new(3.0, 0.0, -2.0),
                DVec3::ZERO,
            );
            let result = sim.run_free(START_S, start, &mut ()).unwrap();
            (sim, result)
        };
        let (sim, pushed) = fly(1.0);
        let (_, unpushed) = fly(0.0);
        let first = pushed.event(EventKind::Ejection(0)).unwrap().sample;
        assert!(
            (first.height_above_ground_m - 300.0).abs() < 1e-6,
            "{first:?}"
        );
        let through_air = (first.cg_velocity_enu_m_s - wind_enu).normalize();
        let nose_kg = component_kg(&sim, "nose");
        let rest_kg = sim.assembly().mass_properties(first.time_s).mass_kg - nose_kg;
        let nose_push = start_change(&pushed, &unpushed, 0);
        close_vec(
            nose_push,
            -through_air * (1.0 / nose_kg),
            1e-9,
            "the nose cone's push",
        );
        close_vec(
            start_change(&pushed, &unpushed, 1),
            through_air * (1.0 / rest_kg),
            1e-9,
            "the airframe's push",
        );
        // Not the frozen axis: that points 60° up, the push nearly straight up.
        let frozen_axis = pushed.final_sample.state.attitude.mul_vec3(DVec3::Z);
        assert!(nose_push.normalize().dot(frozen_axis) < 0.95, "{nose_push}");
        assert!(pushed.bodies_landed());
    }

    #[test]
    fn a_push_in_still_air_is_taken_as_up() {
        // Climbing straight up in calm air, the nose cone leaves on a timer; the payload leaves
        // the airframe's body at its own apogee, where it doesn't move through the air, so the
        // push is up: 4 m/s on the payload. The airframe's canopy waits for 200 m, so nothing is
        // open, and along its velocity (a hair downward, past the apogee) would point down.
        let sim = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(vec![
            canopy(0, 0.45, Trigger::Apogee),
            canopy(
                1,
                0.9,
                Trigger::Altitude {
                    height_above_ground_m: 200.0,
                },
            ),
            canopy(2, 0.6, Trigger::Apogee),
        ])
        .unwrap()
        .with_ejections(vec![
            Ejection::aft_of(Trigger::Time { time_s: START_S }, "nose"),
            Ejection::payload(Trigger::Apogee, "payload").with_impulse(1.0),
        ])
        .unwrap();
        let start = dropped(&sim, 1_000.0, DVec3::new(0.0, 0.0, 20.0), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let airframe = &result.bodies[1];
        let payload = &result.bodies[2];
        let parting = airframe.event(EventKind::Ejection(1)).unwrap().sample;
        assert!(parting.airspeed_m_s < 1e-3, "{parting:?}");
        assert_eq!(parting.recovery_drag_area_m2, 0.0);
        close_vec(
            payload.start_sample.cg_velocity_enu_m_s - parting.cg_velocity_enu_m_s,
            DVec3::new(0.0, 0.0, 4.0),
            1e-9,
            "the payload's push",
        );
        assert!(result.bodies_landed());
    }

    #[test]
    fn a_separation_with_pushed_ejections_pushes_only_the_ejections_sides() {
        // The two-stage design parts at its stage boundary and pushes its nose cone off with
        // 1 N·s, both at apogee from the stack in free flight: the booster (body 1, the
        // separation's) gets no push, and the nose cone and the sustainer's airframe (body 2)
        // share the one push along the axis.
        let fly = |impulse_n_s: f64| {
            let rocket = design("synthetic-two-stage-75mm-54mm");
            let assembly = rocket.assemble("j760-i175").unwrap();
            let tumble = DeviceDrag::tumbling_stages(&assembly, (1, 1)).unwrap();
            let sim = Simulation::new(
                &rocket,
                "j760-i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(6.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
            .with_separation(Separation::new(Trigger::Apogee, 0))
            .unwrap()
            .with_ejections(vec![
                Ejection::aft_of(Trigger::Apogee, "nose").with_impulse(impulse_n_s),
            ])
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.6, Trigger::Apogee),
                Device::new("booster", tumble, Trigger::Apogee).on_body(1),
                canopy(2, 1.2, Trigger::Apogee),
            ])
            .unwrap();
            let start = dropped_at(
                &sim,
                (70.0, 200.0),
                1_500.0,
                DVec3::new(0.0, 0.0, -0.5),
                DVec3::ZERO,
            );
            let result = sim.run_free(START_S, start, &mut ()).unwrap();
            (sim, result)
        };
        let (sim, pushed) = fly(1.0);
        let (_, unpushed) = fly(0.0);
        let axis = pushed.final_sample.state.attitude.mul_vec3(DVec3::Z);
        assert_eq!(start_change(&pushed, &unpushed, 1), DVec3::ZERO);
        let nose_kg = pushed.bodies[0].start_sample.mass_kg;
        let airframe_kg = pushed.bodies[2].start_sample.mass_kg;
        close(nose_kg, component_kg(&sim, "nose"), 1e-12, "the nose cone");
        close_vec(
            start_change(&pushed, &unpushed, 0),
            axis * (1.0 / nose_kg),
            1e-9,
            "the nose cone's push",
        );
        close_vec(
            start_change(&pushed, &unpushed, 2),
            -axis * (1.0 / airframe_kg),
            1e-9,
            "the airframe's push",
        );
        let first = pushed.event(EventKind::Separation).unwrap().sample;
        let whole_kg = sim.assembly().mass_properties(first.time_s).mass_kg;
        let momentum: DVec3 = pushed
            .bodies
            .iter()
            .map(|body| body.start_sample.cg_velocity_enu_m_s * body.start_sample.mass_kg)
            .sum();
        close_vec(
            momentum,
            first.cg_velocity_enu_m_s * whole_kg,
            1e-9,
            "the momentum",
        );
    }

    #[test]
    fn pushed_sections_parting_together_on_the_way_down_push_as_the_stack_does() {
        // The booster section leaves at apogee; at 300 m the nose and the interstage both leave
        // the body left, each with 1 N·s, in one pass, as the stack's partings do at the first:
        // each final body takes the pushes of the joints on its sides. By hand, on the way the
        // nose points (up the velocity through the air, under the canopy it hangs from since
        // apogee): the nose cone `+J/m`, the sustainer's airframe between the two joints `−J/m +
        // J/m = 0`, and the interstage `−J/m`. Given in either order, the same (found in review:
        // taken one at a time, the order moved the nose cone's push from 15.85 to 17.66 m/s).
        let at_300_m = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        let orders = [
            vec![
                Ejection::aft_of(Trigger::Apogee, "interstage"),
                Ejection::aft_of(at_300_m, "nose").with_impulse(1.0),
                Ejection::aft_of(at_300_m, "sustainer-airframe").with_impulse(1.0),
            ],
            vec![
                Ejection::aft_of(Trigger::Apogee, "interstage"),
                Ejection::aft_of(at_300_m, "sustainer-airframe").with_impulse(1.0),
                Ejection::aft_of(at_300_m, "nose").with_impulse(1.0),
            ],
        ];
        let mut landings = Vec::new();
        for (order, ejections) in orders.into_iter().enumerate() {
            let (sim, result) = two_stage_pieces(ejections, 4);
            assert!(result.bodies_landed(), "order {order}");
            // Both partings are the nose's body's, at the one instant.
            let nose = &result.bodies[0];
            let partings: Vec<&BodyEvent> = nose
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Ejection(_)))
                .collect();
            assert_eq!(partings.len(), 2, "order {order}");
            let (before, after) = (partings[0].sample, partings[0].after.unwrap());
            assert_eq!(partings[1].sample, before);
            let nose_ward = -before.cg_velocity_enu_m_s.normalize();
            let (airframe, interstage) = if order == 0 {
                (&result.bodies[2], &result.bodies[3])
            } else {
                (&result.bodies[3], &result.bodies[2])
            };
            close(
                after.mass_kg,
                component_kg(&sim, "nose"),
                1e-12,
                "the nose cone",
            );
            let change = |velocity: DVec3| velocity - before.cg_velocity_enu_m_s;
            close_vec(
                change(after.cg_velocity_enu_m_s),
                nose_ward * (1.0 / after.mass_kg),
                1e-9,
                "the nose cone's push",
            );
            assert!(
                change(airframe.start_sample.cg_velocity_enu_m_s).length() < 1e-12,
                "order {order}: {}",
                change(airframe.start_sample.cg_velocity_enu_m_s)
            );
            close_vec(
                change(interstage.start_sample.cg_velocity_enu_m_s),
                -nose_ward * (1.0 / interstage.start_sample.mass_kg),
                1e-9,
                "the interstage's push",
            );
            let momentum = after.cg_velocity_enu_m_s * after.mass_kg
                + airframe.start_sample.cg_velocity_enu_m_s * airframe.start_sample.mass_kg
                + interstage.start_sample.cg_velocity_enu_m_s * interstage.start_sample.mass_kg;
            close_vec(
                momentum,
                before.cg_velocity_enu_m_s * before.mass_kg,
                1e-9,
                "the momentum",
            );
            landings.push([
                nose.final_sample.cg_enu_m,
                airframe.final_sample.cg_enu_m,
                interstage.final_sample.cg_enu_m,
            ]);
        }
        for (first, second) in landings[0].iter().zip(&landings[1]) {
            assert!((*first - *second).length() < 1e-6, "{first} vs {second}");
        }
    }

    #[test]
    fn a_device_opening_as_its_body_parts_is_not_yet_hung_from() {
        // The airframe falls with nothing open, and its canopy and the payload's parting both
        // come at 300 m. At that instant it doesn't yet hang from the canopy, so the payload
        // leaves down its flight, whether the canopy opens at once or fills over a second (found
        // in review: the push flipped with the inflation law).
        let at_300_m = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        let changes: Vec<DVec3> = [
            Inflation::Instant,
            Inflation::FillingTime {
                time_s: 1.0,
                exponent: 2.0,
            },
        ]
        .into_iter()
        .map(|inflation| {
            let sim = Simulation::new(
                &with_payload(),
                "i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(3.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.45, Trigger::Apogee),
                canopy(1, 0.9, at_300_m).with_inflation(inflation),
                canopy(2, 0.6, at_300_m),
            ])
            .unwrap()
            .with_ejections(pushed(1.0))
            .unwrap();
            let start = dropped(&sim, 1_500.0, DVec3::new(2.0, 1.0, -0.5), DVec3::ZERO);
            let result = sim.run_free(START_S, start, &mut ()).unwrap();
            let parting = result.bodies[1]
                .event(EventKind::Ejection(1))
                .unwrap()
                .sample;
            let change =
                result.bodies[2].start_sample.cg_velocity_enu_m_s - parting.cg_velocity_enu_m_s;
            close_vec(
                change,
                parting.cg_velocity_enu_m_s.normalize() * 4.0,
                1e-9,
                "the payload's push",
            );
            change
        })
        .collect();
        assert!(changes[0].z < -3.9, "{}", changes[0]);
    }

    #[test]
    fn a_pushed_payload_whose_way_forward_is_still_closed_is_refused_in_flight() {
        // The payload is pushed out at apogee, but the joint forward of its airframe only parts
        // at 300 m: it has no way out forward then.
        let sim = simulation_with(
            analytic_environment(UniformAir::sea_level(), G),
            vec![
                Ejection::aft_of(
                    Trigger::Altitude {
                        height_above_ground_m: 300.0,
                    },
                    "nose",
                ),
                Ejection::payload(Trigger::Apogee, "payload").with_impulse(1.0),
            ],
        );
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let error = sim.run_free(START_S, start, &mut ()).unwrap_err();
        assert!(
            matches!(error, SimError::Domain { what, value }
                if what.starts_with("time of a pushed payload's ejection") && value == START_S),
            "{error:?}"
        );
    }

    /// The milestone's design with `devices` and the nose cone pushed off at 300 m with
    /// `impulse_n_s`, dropped from a stack tilted 60° in a 4 m/s wind.
    fn nose_off_at_300_m(devices: Vec<Device>, impulse_n_s: f64) -> (Simulation, FlightResult) {
        let at_300_m = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        let sim = Simulation::new(
            &with_payload(),
            "i175",
            analytic_wind_environment(
                UniformAir::sea_level(),
                G,
                ConstantWind::new(4.0, 2.0).unwrap(),
            ),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices)
        .unwrap()
        .with_ejections(vec![
            Ejection::aft_of(at_300_m, "nose").with_impulse(impulse_n_s),
        ])
        .unwrap();
        let start = dropped_at(
            &sim,
            (60.0, 30.0),
            1_500.0,
            DVec3::new(3.0, 0.0, -2.0),
            DVec3::ZERO,
        );
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        (sim, result)
    }

    /// The nose cone's change of velocity at its parting from [`nose_off_at_300_m`] with 1 N·s,
    /// against the same flight unpushed, over the unit velocity through the air there and the
    /// push's size `J/m`.
    fn nose_push_through_air(devices: impl Fn() -> Vec<Device>) -> f64 {
        let (sim, pushed) = nose_off_at_300_m(devices(), 1.0);
        let (_, unpushed) = nose_off_at_300_m(devices(), 0.0);
        let first = pushed.event(EventKind::Ejection(0)).unwrap().sample;
        let wind_enu = ConstantWind::new(4.0, 2.0)
            .unwrap()
            .wind(0.0)
            .unwrap()
            .velocity_enu_m_s;
        let through_air = (first.cg_velocity_enu_m_s - wind_enu).normalize();
        let push = start_change(&pushed, &unpushed, 0);
        let size_m_s = 1.0 / component_kg(&sim, "nose");
        // Along the velocity through the air, one way or the other, and nowhere else.
        let along = push.dot(through_air) / size_m_s;
        close_vec(
            push,
            through_air * along * size_m_s,
            1e-9,
            "the push's line",
        );
        along
    }

    #[test]
    fn a_stack_tumbling_since_apogee_is_pushed_along_its_flight() {
        // Only a tumble on the stack since apogee: its attitude froze then, so the push goes by
        // its velocity through the air, but it hangs from nothing, so the nose cone is pushed
        // along it, as a stable airframe's nose would point.
        let along = nose_push_through_air(|| {
            let tumble = DeviceDrag::tumbling(&with_payload().assemble("i175").unwrap()).unwrap();
            vec![
                Device::new("tumble", tumble, Trigger::Apogee),
                canopy(
                    1,
                    0.9,
                    Trigger::Altitude {
                        height_above_ground_m: 300.0,
                    },
                ),
            ]
        });
        close(along, 1.0, 1e-9, "along the flight");
    }

    #[test]
    fn a_drogue_released_as_the_body_parts_is_still_hung_from() {
        // A drogue on the stack since apogee, cut away by a main that opens at 300 m, as the nose
        // cone leaves. Up to that instant the stack hung from the drogue, so the nose cone goes
        // up its velocity through the air whether the main opens at once, releasing the drogue at
        // that instant, or fills over a second (found in review: the push flipped with the law).
        for inflation in [
            Inflation::Instant,
            Inflation::FillingTime {
                time_s: 1.0,
                exponent: 2.0,
            },
        ] {
            let along = nose_push_through_air(|| {
                vec![
                    canopy(0, 0.3, Trigger::Apogee).with_release_by(1),
                    canopy(
                        0,
                        0.9,
                        Trigger::Altitude {
                            height_above_ground_m: 300.0,
                        },
                    )
                    .with_inflation(inflation),
                    canopy(
                        1,
                        0.9,
                        Trigger::Altitude {
                            height_above_ground_m: 300.0,
                        },
                    ),
                ]
            });
            close(along, -1.0, 1e-9, &format!("{inflation:?}"));
        }
    }

    #[test]
    fn one_charge_can_push_off_the_nose_cone_and_let_the_payload_out() {
        // Both at apogee from the stack in free flight, 1 N·s each: by hand, along the axis, the
        // nose cone `+1 N·s`, the payload, leaving forward through the joint that opens with it,
        // `+1 N·s`, and the airframe between them `−2 N·s`, each over its own mass.
        let fly = |impulse_n_s: f64| {
            let sim = Simulation::new(
                &with_payload(),
                "i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(3.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.45, Trigger::Apogee),
                canopy(1, 0.9, Trigger::Apogee),
                canopy(2, 0.6, Trigger::Apogee),
            ])
            .unwrap()
            .with_ejections(vec![
                Ejection::aft_of(Trigger::Apogee, "nose").with_impulse(impulse_n_s),
                Ejection::payload(Trigger::Apogee, "payload").with_impulse(impulse_n_s),
            ])
            .unwrap();
            let start = dropped_at(
                &sim,
                (70.0, 120.0),
                1_500.0,
                DVec3::new(0.0, 0.0, -0.5),
                DVec3::ZERO,
            );
            sim.run_free(START_S, start, &mut ()).unwrap()
        };
        let (pushed, unpushed) = (fly(1.0), fly(0.0));
        assert!(pushed.bodies_landed());
        let axis = pushed.final_sample.state.attitude.mul_vec3(DVec3::Z);
        for (body, impulse_n_s) in [(0, 1.0), (1, -2.0), (2, 1.0)] {
            let kg = pushed.bodies[body].start_sample.mass_kg;
            close_vec(
                start_change(&pushed, &unpushed, body),
                axis * (impulse_n_s / kg),
                1e-9,
                &format!("body {body}"),
            );
        }
        close(
            pushed.bodies[2].start_sample.mass_kg,
            PAYLOAD_KG,
            1e-12,
            "the payload",
        );
    }

    #[test]
    fn a_pushed_payload_behind_a_separation_is_accepted_in_either_builder_order() {
        // The two-stage design's booster electronics, pushed out at apogee, are in the booster's
        // piece, forward of which the separation parts: a way out, whichever builder comes first.
        // Without the separation they would be in the nose's piece, and refused at the start.
        let rocket = design("synthetic-two-stage-75mm-54mm");
        let tumble =
            DeviceDrag::tumbling_stages(&rocket.assemble("j760-i175").unwrap(), (1, 1)).unwrap();
        let devices = || {
            vec![
                canopy(0, 1.2, Trigger::Apogee),
                Device::new("booster", tumble, Trigger::Apogee).on_body(1),
                canopy(2, 0.3, Trigger::Apogee),
            ]
        };
        let ejections =
            || vec![Ejection::payload(Trigger::Apogee, "booster-electronics").with_impulse(1.0)];
        let base = || {
            Simulation::new(
                &rocket,
                "j760-i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(6.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
        };
        let separation = Separation::new(Trigger::Apogee, 0);
        let orders = [
            base()
                .with_separation(separation)
                .unwrap()
                .with_ejections(ejections())
                .unwrap()
                .with_recovery(devices())
                .unwrap(),
            base()
                .with_ejections(ejections())
                .unwrap()
                .with_separation(separation)
                .unwrap()
                .with_recovery(devices())
                .unwrap(),
        ];
        for sim in orders {
            let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
            let result = sim.run_free(START_S, start, &mut ()).unwrap();
            assert!(result.bodies_landed());
        }
        let error = base()
            .with_ejections(ejections())
            .unwrap()
            .with_recovery(vec![
                canopy(0, 1.2, Trigger::Apogee),
                canopy(1, 0.3, Trigger::Apogee),
            ])
            .unwrap()
            .run(&mut ())
            .unwrap_err();
        assert!(
            matches!(&error, SimError::Parting { what, component }
                if what.starts_with("a pushed payload leaves forward")
                    && component == "booster-electronics"),
            "{error:?}"
        );
    }

    #[test]
    fn a_parting_without_an_impulse_records_the_body_after_it_unpushed() {
        // With no impulse the body after a parting on the way down has the same point and
        // velocity, and only its mass steps.
        let sim = simulation(analytic_environment(UniformAir::sea_level(), G));
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let airframe = &result.bodies[1];
        let parting = airframe.event(EventKind::Ejection(1)).unwrap();
        let after = parting.after.unwrap();
        assert_eq!(
            after.cg_velocity_enu_m_s,
            parting.sample.cg_velocity_enu_m_s
        );
        assert_eq!(after.cg_enu_m, parting.sample.cg_enu_m);
        close(after.mass_kg, airframe.mass_kg, 1e-12, "the airframe after");
        // Only partings carry one.
        for body in &result.bodies {
            for event in &body.events {
                let parts = matches!(event.kind, EventKind::Ejection(_) | EventKind::Separation);
                assert_eq!(event.after.is_some(), parts, "{event:?}");
            }
        }
    }

    #[test]
    fn a_tumbling_nose_cone_lands_at_its_tumble_models_terminal_speed() {
        // The nose cone pushed off at apogee with nothing but its own tumble; the airframe, with
        // the payload still inside, under its canopy.
        let air = UniformAir::sea_level();
        let base = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(air, G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_ejections(vec![
            Ejection::aft_of(Trigger::Apogee, "nose").with_impulse(0.5),
        ])
        .unwrap();
        let tumble = base.tumbling_piece(0).unwrap();
        let DeviceDrag::Tumble {
            drag_area_m2,
            body_profile_m2,
            fin_area_m2,
        } = tumble
        else {
            panic!("{tumble:?}");
        };
        // By hand: the nose is a 0.25 m tangent ogive on the 54 mm airframe, whose arc has radius
        // `ρ = (R² + L²)/(2R)`, and whose side area is `L√(ρ² − L²) + ρ² asin(L/ρ) + 2(R − ρ)L`.
        // It has no fins, so its drag area is 0.56 times that.
        let nose = sim_component(&base, "nose");
        let (length_m, radius_m) = (0.25_f64, nose.part.aft_radius_m().unwrap());
        let arc_m = (radius_m * radius_m + length_m * length_m) / (2.0 * radius_m);
        let side_m2 = length_m * (arc_m * arc_m - length_m * length_m).sqrt()
            + arc_m * arc_m * (length_m / arc_m).asin()
            + 2.0 * (radius_m - arc_m) * length_m;
        close(body_profile_m2, side_m2, 1e-12, "the nose cone's side area");
        assert_eq!(fin_area_m2, 0.0);
        close(
            drag_area_m2,
            0.56 * side_m2,
            1e-12,
            "the nose cone's drag area",
        );

        let sim = base
            .with_recovery(vec![
                Device::new("nose tumble", tumble, Trigger::Apogee).on_body(0),
                canopy(1, 0.9, Trigger::Apogee),
            ])
            .unwrap();
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert!(result.bodies_landed());
        let nose_body = &result.bodies[0];
        close(
            nose_body.mass_kg,
            component_kg(&sim, "nose"),
            1e-12,
            "the nose cone",
        );
        let landing = nose_body.event(EventKind::GroundHit).unwrap().sample;
        let terminal_m_s =
            terminal_speed_m_s(nose_body.mass_kg, drag_area_m2, air.0.density_kg_m3, G);
        close(
            -landing.vertical_speed_m_s,
            terminal_m_s,
            1e-6,
            "the nose cone's landing",
        );
        // Measured: 13.849 m/s for the 0.0631 kg nose cone on its 5.27e-3 m² drag area.
        assert!((terminal_m_s - 13.849).abs() < 1e-3, "{terminal_m_s}");
    }

    /// The layout's component `id`.
    fn sim_component<'a>(sim: &'a Simulation, id: &str) -> &'a hpr_design::PlacedComponent {
        sim.assembly()
            .layout
            .components
            .iter()
            .find(|component| component.id == id)
            .unwrap()
    }

    #[test]
    fn the_pieces_tumbling_areas_add_up_to_the_whole_airframes() {
        // Cut into sections, the airframe's tumble is theirs summed: here the nose cone, and the
        // airframe with its fins.
        let sim = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_ejections(vec![
            Ejection::aft_of(Trigger::Apogee, "nose"),
            Ejection::payload(Trigger::Apogee, "payload"),
        ])
        .unwrap();
        let area = |drag: DeviceDrag| match drag {
            DeviceDrag::Tumble {
                drag_area_m2,
                body_profile_m2,
                fin_area_m2,
            } => [drag_area_m2, body_profile_m2, fin_area_m2],
            other => panic!("{other:?}"),
        };
        let whole = area(DeviceDrag::tumbling(sim.assembly()).unwrap());
        let nose = area(sim.tumbling_piece(0).unwrap());
        let rest = area(sim.tumbling_piece(1).unwrap());
        for k in 0..3 {
            close(nose[k] + rest[k], whole[k], 1e-12, "the sum");
        }
        assert!(nose[2] == 0.0 && rest[2] > 0.0, "{nose:?} {rest:?}");

        // A payload has no tube or fin of its own, and piece 3 is not made.
        for (piece, starts) in [
            (2, "piece to tumble (a payload has no body tube or fin"),
            (3, "piece to tumble (there is one per split"),
        ] {
            let error = sim.tumbling_piece(piece).unwrap_err();
            assert!(
                matches!(error, SimError::Domain { what, value }
                    if what.starts_with(starts) && value == piece as f64),
                "{error:?}"
            );
        }
    }

    #[test]
    fn an_ejection_with_a_separation_numbers_the_bodies_after_it() {
        // The two-stage design parts at its stage boundary and pushes its nose cone off, both at
        // apogee: the booster is body 1, the nose cone keeps body 0, and the sustainer's airframe
        // is the ejection's body 2.
        let air = UniformAir::sea_level();
        let rocket = design("synthetic-two-stage-75mm-54mm");
        let assembly = rocket.assemble("j760-i175").unwrap();
        let tumble = DeviceDrag::tumbling_stages(&assembly, (1, 1)).unwrap();
        let sim = Simulation::new(
            &rocket,
            "j760-i175",
            analytic_environment(air, G),
            Rail::vertical(6.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_ejections(vec![Ejection::aft_of(Trigger::Apogee, "nose")])
        .unwrap()
        .with_recovery(vec![
            canopy(0, 0.6, Trigger::Apogee),
            Device::new("booster", tumble, Trigger::Apogee).on_body(1),
            canopy(2, 1.2, Trigger::Apogee),
        ])
        .unwrap()
        .with_separation(Separation::new(Trigger::Apogee, 0))
        .unwrap();
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert!(result.event(EventKind::Separation).is_some());
        assert!(result.event(EventKind::Ejection(0)).is_some());
        assert!(result.bodies_landed());
        let stages: Vec<(usize, usize)> = result.bodies.iter().map(|body| body.stages).collect();
        assert_eq!(stages, vec![(0, 0), (1, 1), (0, 0)]);
        let whole_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let sum_kg: f64 = result.bodies.iter().map(|body| body.mass_kg).sum();
        close(sum_kg, whole_kg, 1e-12, "the bodies' mass");
        close(
            result.bodies[0].mass_kg,
            component_kg(&sim, "nose"),
            1e-12,
            "the nose cone",
        );
        // The booster is its stage and motor, as a separation alone makes it.
        let lit = vec![Some(0.0); sim.assembly().motors.len()];
        let booster = crate::recovery::body_mass_properties(sim.assembly(), (1, 1), START_S, &lit);
        close(
            result.bodies[1].mass_kg,
            booster.mass_kg,
            1e-12,
            "the booster",
        );
    }

    #[test]
    fn a_split_on_the_way_down_waits_for_its_own_trigger() {
        // The payload's height is below where the airframe's body starts, so it leaves only on
        // the way down; a payload whose height is never reached lands inside its airframe.
        let sim = simulation(analytic_environment(UniformAir::sea_level(), G));
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        let parting = result.bodies[1]
            .event(EventKind::Ejection(1))
            .unwrap()
            .sample;
        assert!(parting.time_s > START_S + 10.0, "{parting:?}");
        assert!(parting.vertical_speed_m_s < 0.0);

        // With the payload let out on a timer after the landing, it never leaves.
        let late = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices())
        .unwrap()
        .with_ejections(vec![
            Ejection::aft_of(Trigger::Apogee, "nose"),
            Ejection::payload(Trigger::Time { time_s: 3_000.0 }, "payload"),
        ])
        .unwrap();
        let result = late.run_free(START_S, start, &mut ()).unwrap();
        assert_eq!(result.bodies.len(), 2);
        assert_eq!(result.bodies[1].pieces, vec![1, 2]);
        assert!(result.bodies[1].event(EventKind::Ejection(1)).is_none());
        let whole_kg = late.assembly().mass_properties(START_S).mass_kg;
        close(
            result.bodies[0].mass_kg + result.bodies[1].mass_kg,
            whole_kg,
            1e-12,
            "the two bodies",
        );
    }

    /// The two-stage test design with no separation, flown from a drop at 1,500 m with a canopy
    /// on each of its `bodies` and `ejections`.
    fn two_stage_pieces(ejections: Vec<Ejection>, bodies: usize) -> (Simulation, FlightResult) {
        let air = UniformAir::sea_level();
        let sim = Simulation::new(
            &design("synthetic-two-stage-75mm-54mm"),
            "j760-i175",
            analytic_environment(air, G),
            Rail::vertical(6.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(
            (0..bodies)
                .map(|body| canopy(body, 0.8, Trigger::Apogee))
                .collect(),
        )
        .unwrap()
        .with_ejections(ejections)
        .unwrap();
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        (sim, result)
    }

    #[test]
    fn partings_that_fire_together_on_the_way_down_count_each_piece_once() {
        // The booster section leaves at apogee. At 300 m two more partings fire in the same pass
        // on the body left: aft of the nose and aft of the sustainer's airframe. They part it
        // together, whichever is listed first, and each piece is counted once (found in review:
        // taken one at a time, the interstage was counted twice, 1.7651 kg landed from a
        // 1.6752 kg rocket). Both are the parting body's events.
        let at_300_m = Trigger::Altitude {
            height_above_ground_m: 300.0,
        };
        let orders = [
            vec![
                Ejection::aft_of(Trigger::Apogee, "interstage"),
                Ejection::aft_of(at_300_m, "nose"),
                Ejection::aft_of(at_300_m, "sustainer-airframe"),
            ],
            vec![
                Ejection::aft_of(Trigger::Apogee, "interstage"),
                Ejection::aft_of(at_300_m, "sustainer-airframe"),
                Ejection::aft_of(at_300_m, "nose"),
            ],
        ];
        for (order, ejections) in orders.into_iter().enumerate() {
            let (sim, result) = two_stage_pieces(ejections, 4);
            assert!(result.bodies_landed(), "order {order}");
            let pieces: Vec<Vec<usize>> = result
                .bodies
                .iter()
                .map(|body| body.pieces.clone())
                .collect();
            assert_eq!(
                pieces,
                vec![vec![0], vec![1], vec![2], vec![3]],
                "order {order}"
            );
            let whole_kg = sim.assembly().mass_properties(START_S).mass_kg;
            let sum_kg: f64 = result.bodies.iter().map(|body| body.mass_kg).sum();
            close(
                sum_kg,
                whole_kg,
                1e-12,
                &format!("order {order}: the bodies' mass"),
            );
            // Each body is its own components: the nose, and the interstage alone.
            close(
                result.bodies[0].mass_kg,
                component_kg(&sim, "nose"),
                1e-12,
                "the nose cone",
            );
            let interstage = if order == 0 { 3 } else { 2 };
            close(
                result.bodies[interstage].mass_kg,
                component_kg(&sim, "interstage"),
                1e-12,
                &format!("order {order}: the interstage"),
            );
            for kind in [EventKind::Ejection(1), EventKind::Ejection(2)] {
                assert!(
                    result.bodies[0].event(kind).is_some(),
                    "order {order}: {kind:?}"
                );
                assert!(
                    result.bodies[2].event(kind).is_none(),
                    "order {order}: {kind:?}"
                );
            }
        }
    }

    /// The parting error of `ejections` on the payload design, with a device on every body.
    fn refused(ejections: Vec<Ejection>) -> SimError {
        let devices = (0..=ejections.len())
            .map(|body| canopy(body, 0.5, Trigger::Apogee))
            .collect();
        Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_recovery(devices)
        .unwrap()
        .with_ejections(ejections)
        .unwrap_err()
    }

    fn what(error: SimError) -> (&'static str, String) {
        match error {
            SimError::Parting { what, component } => (what, component),
            SimError::Domain { what, .. } => (what, String::new()),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn partings_the_design_cant_make_are_refused() {
        let apogee = Trigger::Apogee;
        let cases: Vec<(Vec<Ejection>, &str, &str)> = vec![
            (
                vec![Ejection::aft_of(apogee, "fairing")],
                "an ejection names a component the design doesn't have",
                "fairing",
            ),
            (
                vec![Ejection::aft_of(apogee, "payload")],
                "a joint is aft of a body component, and this one is inside another (a part \
                 carried inside is a `Parting::Payload`)",
                "payload",
            ),
            (
                vec![Ejection::aft_of(apogee, "sustainer-airframe")],
                "a joint aft of the last body component (nothing is aft of it)",
                "sustainer-airframe",
            ),
            (
                vec![Ejection::payload(apogee, "sustainer-fins")],
                "a payload is carried inside the airframe, and this part is outside it",
                "sustainer-fins",
            ),
            (
                vec![Ejection::payload(apogee, "nose")],
                "a payload is carried inside the airframe, and this is a body component (a body \
                 component leaves at a joint, `Parting::AftOf`)",
                "nose",
            ),
            (
                vec![
                    Ejection::aft_of(apogee, "nose"),
                    Ejection::aft_of(Trigger::Time { time_s: 20.0 }, "nose"),
                ],
                "two partings at the joint aft of this component (a separation's stage boundary \
                 is a joint too)",
                "nose",
            ),
            (
                vec![
                    Ejection::payload(apogee, "payload"),
                    Ejection::payload(apogee, "payload"),
                ],
                "two ejections of one payload",
                "payload",
            ),
        ];
        for (ejections, expected, component) in cases {
            assert_eq!(
                what(refused(ejections)),
                (expected, component.to_owned()),
                "{expected}"
            );
        }

        // A payload inside another payload.
        let mut rocket = with_payload();
        let airframe = &mut rocket.stages[0].components[1];
        let mut inner = airframe
            .children
            .iter()
            .find(|child| child.id == "altimeter")
            .cloned()
            .unwrap();
        inner.id = "inner".to_owned();
        inner.position = Some(Position::Top { aft_offset_m: 0.0 });
        let outer = airframe
            .children
            .iter_mut()
            .find(|child| child.id == "sustainer-motor-mount")
            .unwrap();
        outer.children.push(inner);
        let error = Simulation::new(
            &rocket,
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_ejections(vec![
            Ejection::payload(apogee, "sustainer-motor-mount"),
            Ejection::payload(apogee, "inner"),
        ])
        .unwrap_err();
        assert_eq!(
            what(error),
            ("a payload inside another payload", "inner".to_owned())
        );

        // A stage whose mass is overridden can't be divided between pieces.
        let mut rocket = with_payload();
        rocket.stages[0].overrides.mass_kg = Some(1.0);
        let error = Simulation::new(
            &rocket,
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_ejections(vec![Ejection::payload(apogee, "payload")])
        .unwrap_err();
        assert_eq!(
            what(error),
            (
                "a parting through a stage whose mass is overridden (the override doesn't say \
                 which piece its mass is in)",
                "sustainer".to_owned()
            )
        );

        // Nor can an airframe whose override covers what it holds lose a payload from inside.
        let mut rocket = with_payload();
        rocket.stages[0].components[1].overrides.mass_kg = Some(1.0);
        rocket.stages[0].components[1].overrides_include_children = true;
        let error = Simulation::new(
            &rocket,
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_ejections(vec![Ejection::payload(apogee, "payload")])
        .unwrap_err();
        assert_eq!(
            what(error),
            (
                "a parting inside a component whose overridden mass covers what it holds (the \
                 override doesn't say which piece that mass is in)",
                "sustainer-airframe".to_owned()
            )
        );
    }

    #[test]
    fn ejections_outside_their_domain_are_refused() {
        // During the burn: every body is a point mass of constant mass.
        let error = refused(vec![Ejection::aft_of(
            Trigger::Time { time_s: 0.5 },
            "nose",
        )]);
        assert_eq!(
            what(error).0,
            "time of an ejection (every motor must have burned out by then; this is when one of \
             them does)"
        );
        let error = refused(vec![Ejection::aft_of(
            Trigger::Altitude {
                height_above_ground_m: -1.0,
            },
            "nose",
        )]);
        assert_eq!(
            what(error).0,
            "height above the launch site at which a piece is ejected, m"
        );
        // A pushed payload leaves forward, and with no joint forward of it that is the nose tip:
        // refused when the flight starts, since a separation given later could open one. Without
        // a push it may still leave.
        let payload_only = |impulse_n_s: f64| {
            Simulation::new(
                &with_payload(),
                "i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(3.0),
                FlightSettings::default(),
            )
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.5, Trigger::Apogee),
                canopy(1, 0.5, Trigger::Apogee),
            ])
            .unwrap()
            .with_ejections(vec![
                Ejection::payload(Trigger::Apogee, "payload").with_impulse(impulse_n_s),
            ])
            .unwrap()
            .run(&mut ())
        };
        let (what_, component) = what(payload_only(1.0).unwrap_err());
        assert!(
            what_.starts_with("a pushed payload leaves forward, and this one is in the nose's")
                && component == "payload",
            "{what_}"
        );
        assert!(payload_only(0.0).is_ok());
        for impulse_n_s in [-1.0, f64::NAN, f64::INFINITY] {
            let error = refused(vec![
                Ejection::aft_of(Trigger::Apogee, "nose").with_impulse(impulse_n_s),
            ]);
            assert!(
                matches!(error, SimError::Domain { what, value }
                    if what == "impulse of an ejection, N·s (zero or more: it pushes the two \
                                sides apart)"
                        && (value == impulse_n_s || value.is_nan() && impulse_n_s.is_nan())),
                "{error:?}"
            );
        }

        // Every body needs a device, and no device may name a body that isn't made.
        let base = || {
            Simulation::new(
                &with_payload(),
                "i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(3.0),
                FlightSettings::default(),
            )
            .unwrap()
        };
        let error = base()
            .with_recovery(vec![canopy(0, 0.5, Trigger::Apogee)])
            .unwrap()
            .with_ejections(vec![Ejection::aft_of(Trigger::Apogee, "nose")])
            .unwrap_err();
        assert!(
            matches!(error, SimError::Domain { what, value }
                if what.starts_with("recovery devices on a separated body") && value == 1.0),
            "{error:?}"
        );
        // That one only when the flight starts: a separation given later would make body 2.
        let error = base()
            .with_ejections(vec![Ejection::aft_of(Trigger::Apogee, "nose")])
            .unwrap()
            .with_recovery(vec![
                canopy(0, 0.5, Trigger::Apogee),
                canopy(1, 0.5, Trigger::Apogee),
                canopy(2, 0.5, Trigger::Apogee),
            ])
            .unwrap()
            .run(&mut ())
            .unwrap_err();
        assert!(
            matches!(error, SimError::Domain { what, value }
                if what.starts_with("body a device is attached to") && value == 2.0),
            "{error:?}"
        );
    }

    #[test]
    fn a_flight_without_partings_is_unchanged() {
        // No ejection, no separation: one piece, no bodies, and the same flight to the bit as one
        // never given the empty list.
        let sim = Simulation::new(
            &with_payload(),
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(vec![canopy(0, 0.9, Trigger::Apogee)])
        .unwrap();
        let plain: FlightResult = sim.run(&mut ()).unwrap();
        let result = sim
            .with_ejections(Vec::new())
            .unwrap()
            .run(&mut ())
            .unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(result.bodies.is_empty());
        assert_eq!(result, plain);
    }

    #[test]
    fn an_override_that_covers_only_its_own_part_still_divides() {
        // The airframe's own mass overridden, not what it holds: the payload's mass is still its
        // own, and the pieces still add up to the rocket.
        let mut rocket = with_payload();
        rocket.stages[0].components[1].overrides.mass_kg = Some(0.5);
        let sim = Simulation::new(
            &rocket,
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 3600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(devices())
        .unwrap()
        .with_ejections(ejections())
        .unwrap();
        let start = dropped(&sim, 1_500.0, DVec3::new(0.0, 0.0, -0.5), DVec3::ZERO);
        let result = sim.run_free(START_S, start, &mut ()).unwrap();
        assert!(result.bodies_landed());
        let whole_kg = sim.assembly().mass_properties(START_S).mass_kg;
        let sum_kg: f64 = result.bodies.iter().map(|body| body.mass_kg).sum();
        close(sum_kg, whole_kg, 1e-12, "the pieces' mass");
        close(result.bodies[2].mass_kg, PAYLOAD_KG, 1e-12, "the payload");
    }

    #[test]
    fn refusals_in_flight_name_what_fired() {
        // The two-stage design with its sustainer lit by the separation half a second after the
        // booster burns out.
        let mut rocket = design("synthetic-two-stage-75mm-54mm");
        rocket.configurations[0]
            .motors
            .iter_mut()
            .find(|motor| motor.mount == "sustainer-motor-mount")
            .unwrap()
            .ignition = hpr_design::Ignition::Separation { delay_s: 0.0 };
        let build = || {
            Simulation::new(
                &rocket,
                "j760-i175",
                analytic_environment(UniformAir::sea_level(), G),
                Rail::vertical(6.0),
                FlightSettings {
                    max_time_s: 3600.0,
                    ..FlightSettings::default()
                },
            )
            .unwrap()
        };
        let booster = build()
            .assembly()
            .motors
            .iter()
            .position(|motor| motor.mount == "booster-motor-mount")
            .unwrap();
        let sustainer = 1 - booster;
        let devices = (0..3)
            .map(|body| canopy(body, 0.8, Trigger::Time { time_s: 0.0 }))
            .collect::<Vec<_>>();
        let domain = |error: SimError| match error {
            SimError::Domain { what, .. } => what,
            other => panic!("{other:?}"),
        };

        // A powered separation in a flight with ejections: the sustainer's pieces aren't tracked.
        let error = build()
            .with_recovery(devices.clone())
            .unwrap()
            .with_separation(Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: 0.5,
                },
                0,
            ))
            .unwrap()
            .with_ejections(vec![Ejection::aft_of(Trigger::Apogee, "nose")])
            .unwrap()
            .run(&mut ())
            .unwrap_err();
        assert_eq!(
            domain(error),
            "time of a powered separation in a flight with ejections (the pieces of a sustainer \
             aren't tracked)"
        );

        // An ejection ahead of a separation that would light the sustainer.
        let error = build()
            .with_recovery(devices.clone())
            .unwrap()
            .with_separation(Separation::new(Trigger::Time { time_s: 3_000.0 }, 0))
            .unwrap()
            .with_ejections(vec![Ejection::aft_of(Trigger::Apogee, "nose")])
            .unwrap()
            .run(&mut ())
            .unwrap_err();
        assert_eq!(
            domain(error),
            "time of an ejection before the separation that lights a motor (the pieces would \
             never light it)"
        );

        // An ejection timed from the sustainer, which has no ignition before the flight.
        let error = build()
            .with_recovery(devices)
            .unwrap()
            .with_separation(Separation::new(Trigger::Time { time_s: 3_000.0 }, 0))
            .unwrap()
            .with_ejections(vec![Ejection::aft_of(
                Trigger::MotorDelay { motor: sustainer },
                "nose",
            )])
            .unwrap_err();
        assert_eq!(
            domain(error),
            "index of the motor an ejection is timed from (it has no ignition time before the \
             flight, so the ejection could never fire)"
        );
    }

    #[test]
    fn an_ejection_reads_and_writes_as_json() {
        let ejection = Ejection::payload(
            Trigger::Altitude {
                height_above_ground_m: 300.0,
            },
            "payload",
        );
        let text = serde_json::to_string(&ejection).unwrap();
        assert_eq!(
            text,
            r#"{"trigger":{"altitude":{"height_above_ground_m":300.0}},"parting":{"payload":{"component":"payload"}}}"#
        );
        assert_eq!(serde_json::from_str::<Ejection>(&text).unwrap(), ejection);
        // An impulse is written when there is one, and read back.
        let pushed = ejection.clone().with_impulse(0.75);
        let text = serde_json::to_string(&pushed).unwrap();
        assert!(text.ends_with(r#""impulse_n_s":0.75}"#), "{text}");
        assert_eq!(serde_json::from_str::<Ejection>(&text).unwrap(), pushed);
        // A misspelt field is refused, inside the parting as well as outside it.
        for text in [
            r#"{"trigger":"apogee","parting":{"aft_of":{"component":"nose","x":1}}}"#,
            r#"{"trigger":"apogee","parting":{"aft_of":{"component":"nose"}},"x":1}"#,
        ] {
            assert!(serde_json::from_str::<Ejection>(text).is_err(), "{text}");
        }
        let read: Ejection = serde_json::from_str(
            r#"{"trigger":"apogee","parting":{"aft_of":{"component":"nose"}}}"#,
        )
        .unwrap();
        assert_eq!(read, Ejection::aft_of(Trigger::Apogee, "nose"));
    }
}
