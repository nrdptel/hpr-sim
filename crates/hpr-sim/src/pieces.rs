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
use crate::recovery::{Separation, Trigger};

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
    /// that carries it, as a payload pushed out of a body tube does.
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
/// An ejection adds no impulse, as a separation adds none. When the airframe first comes apart
/// each body starts at its own centre of mass with the velocity that point had. A body already
/// flying as a point mass has no attitude to place its pieces by, so they start where it was, at
/// its velocity. Every motor must have burned out by the time an ejection fires. The decision
/// record on ejected pieces, [ADR-085][adr-085], has the reasoning.
///
/// [adr-085]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-085-ejected-pieces-an-airframe-that-parts-at-any-joint-2026-09-26
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ejection {
    /// When the piece leaves: the same triggers as a recovery device's.
    pub trigger: Trigger,
    /// Where the airframe parts.
    pub parting: Parting,
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
        }
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
}

impl Pieces {
    /// The pieces of `assembly` (assembled from `rocket`) under `separation` and `ejections`.
    ///
    /// # Errors
    ///
    /// [`SimError::Parting`] for an ejection that names a component the design doesn't have, a
    /// joint aft of an internal component or of the last body component, a payload that is a body
    /// component or one of several copies in a cluster, two partings at one joint or of one
    /// payload, or a split through a stage or component whose overridden mass doesn't say how it
    /// divides; [`SimError::Domain`] for a separation with no stage aft of it.
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
                    let at = top.iter().position(|&body| body == index).unwrap_or(0) + 1;
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
                    if components[index].copies_m.len() != 1 {
                        return Err(refuse(
                            "a payload in a cluster of tubes (it is one of several copies)",
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
                .get(stage)
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
    use hpr_atmos::ConstantWind;
    use hpr_core::DVec3;
    use hpr_design::{Part, Position};

    use super::*;
    use crate::flight::{EventKind, FlightResult, FlightSettings, Simulation, Termination};
    use crate::rail::Rail;
    use crate::recovery::{CanopyType, Device, DeviceDrag, terminal_speed_m_s};
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
        // No ejection, no separation: one piece, and no bodies.
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
        .unwrap()
        .with_ejections(Vec::new())
        .unwrap();
        let result: FlightResult = sim.run(&mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(result.bodies.is_empty());
    }
}
