//! When a `.ork` design's motors light and where its stack comes apart, in the terms hpr's flight
//! takes ([M1.9c][m1-9c], decision [ADR-076][adr-076]).
//!
//! **Ignition.** Each motor's `<ignitionevent>` and `<ignitiondelay>` ([`Ignition`]) become an
//! [`hpr_design::Ignition`]. OpenRocket's words are measured by a committed probe (the
//! [`.ork` format page][format]):
//!
//! | the file says | hpr lights the motor |
//! |---|---|
//! | `launch` plus `d` | at `t = d` |
//! | `automatic` plus `d`, in the bottom stage | at `t = d` |
//! | `automatic` plus `d`, in a stage above | as `ejectioncharge` |
//! | `ejectioncharge` plus `d` | `x + d` after the burnout of the stage below's motor, whose ejection delay is `x` |
//! | `burnout` plus `d` | `d` after the burnout of the stage below's motor |
//! | `never` | never ([`hpr_design::Ignition::Never`]) |
//! | `burnout` or `ejectioncharge`, in the bottom stage | never: it has no stage below |
//! | `ejectioncharge` or `automatic`, the stage below's motor plugged | never: that fires no charge |
//!
//! "The stage below" is the next stage aft, and its motors must sit in one mount (one tube or one
//! cluster), so that its first burnout is that mount's. A motor that never lights is carried
//! loaded, with no thrust, as OpenRocket 24.12 flies it ([M2.2e10][m2-2e10]), and so is one that
//! waits on it, which has no burnout or charge to wait on (inferred: no probe chains two). `unlit_motors.py` sets the booster of OpenRocket's two-stage example so
//! that it, or the sustainer waiting on its plugged charge, never lights: OpenRocket lights only
//! the other motor and loses only that one's propellant. A configuration is not flown when a
//! motor waits on a stage below that holds no motor, or motors in more than one mount, or on the
//! charge of a motor below that states no ejection delay, or has a negative delay, or when no
//! motor of it lights at all.
//!
//! **Separation.** A stage's `<separationevent>` says when it drops away from the stage ahead of
//! it. hpr flies one separation, as a [`Staging`], when its time is known before the flight (a time
//! after launch, or a motor's burnout or ejection charge) and the part ahead of it still has a motor
//! to burn then, burning or due to light: that part is a sustainer, and hpr flies it on
//! ([Staging][staging]). The motors of the part that drops away must have burned out by then.
//!
//! A separation at its own motor's burnout or ejection charge, when that motor never lights, never
//! comes, as in the probes, and is left out. One at the ignition of a motor that never lights, or
//! at launch (hpr's flight fires a separation only once the rocket is off the rod), is not flown.
//!
//! A separation at `apogee` or at a height on the way down can only come at or after apogee, so the
//! climb is the whole stack's in both programs, as long as every motor is spent by then, and the
//! separation belongs to the descent, which hpr's flights of a `.ork` do not fly yet (the decision
//! record on reading recovery, [ADR-056][adr-056]): it is left out, and the configuration flies
//! whole. Whether every motor is spent by apogee is known only once flown; `cargo xtask
//! ork-flights` checks it of every flight it reports. Any other separation
//! with nothing ahead of it left to burn could come before apogee, where hpr would fly both parts
//! without their airframes' drag (the decision record on separation, [ADR-014][adr-014]), so the
//! configuration is not flown.
//!
//! **Flying it.** A configuration with a [`Staging`] is among the design's rocket configurations,
//! with each motor's ignition, but the separation is not part of it: flown without one, the stack
//! would carry its booster to the ground with the sustainer lit on it. `hpr::ork::separation` turns
//! a [`Staging`] into the flight's separation, and the example `ork_two_stage` in the `hpr` crate
//! flies one.
//!
//! [m1-9c]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9c
//! [m2-2e10]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e10
//! [adr-076]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-076-a-ork-files-ignitions-and-one-powered-separation-flown-against-openrocket-2026-09-25
//! [adr-056]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-056-a-ork-designs-recovery-and-separation-read-as-written-with-openrockets-words-measured-2026-09-21
//! [adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
//! [format]: https://nrdptel.github.io/hpr-sim/format/ork.html#delays-and-ignition
//! [staging]: https://nrdptel.github.io/hpr-sim/physics/staging.html

use hpr_motor::Delay;
use serde::{Deserialize, Serialize};

use super::motors::{Ignition, IgnitionEvent, OrkMotor};
use super::recovery::{SeparationEvent, StageSeparation};

/// When a powered separation fires, in the terms of hpr's flight triggers
/// (`hpr_sim::recovery::Trigger`, which this crate does not depend on).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum StagingTrigger {
    /// At a time after launch, s.
    Time {
        /// The time after launch, s.
        time_s: f64,
    },
    /// A delay after the burnout of the motor in a mount (the first tube's, for a cluster, whose
    /// tubes light together).
    Burnout {
        /// The mount's component id.
        mount: String,
        /// The delay after its burnout, s.
        delay_s: f64,
    },
}

/// The one powered separation a configuration flies: where the stack comes apart and when.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Staging {
    /// The last stage that stays with the nose; the stages after it drop away.
    pub after_stage: usize,
    /// When.
    pub trigger: StagingTrigger,
    /// When that is, s after launch: known before the flight.
    pub time_s: f64,
}

/// The motors of `motors` in stage `stage`, with the one mount they sit in, or why there is no
/// single such mount.
fn one_mount(motors: &[OrkMotor], stage: usize) -> Result<Option<&OrkMotor>, String> {
    let mut in_stage = motors.iter().filter(|m| m.stage == stage);
    let Some(first) = in_stage.next() else {
        return Ok(None);
    };
    // A mount holds one motor per configuration, so each other motor is another mount.
    let others = in_stage.filter(|m| m.mount != first.mount).count();
    if others > 0 {
        return Err(format!(
            "stage {stage} holds motors in {} mounts, and which burns out first is not worked out",
            others + 1
        ));
    }
    Ok(Some(first))
}

/// `delay_s`, or why it can't be flown: a delay is a time after its event.
fn delay(delay_s: f64, what: &str) -> Result<f64, String> {
    if delay_s.is_finite() && delay_s >= 0.0 {
        Ok(delay_s)
    } else {
        Err(format!(
            "its {what} delay, {delay_s} s, is not a time after its event"
        ))
    }
}

/// When `motor` lights in hpr's terms, given every motor of its configuration and the index of the
/// bottom stage; or why hpr can't light it as the file says.
pub(super) fn ignition(
    motor: &OrkMotor,
    motors: &[OrkMotor],
    last_stage: usize,
) -> Result<hpr_design::Ignition, String> {
    let Ignition { event, delay_s } = &motor.ignition;
    let delay_s = delay(*delay_s, "ignition")?;
    let at = |time_s: f64| {
        if time_s == 0.0 {
            hpr_design::Ignition::Launch
        } else {
            hpr_design::Ignition::Time { time_s }
        }
    };
    // The stage below's motor, or `None` in the bottom stage, which has no stage below: its event
    // never comes, and OpenRocket flies the motor unlit.
    let below = || -> Result<Option<&OrkMotor>, String> {
        if motor.stage >= last_stage {
            return Ok(None);
        }
        one_mount(motors, motor.stage + 1)?
            .ok_or_else(|| {
                format!(
                    "it lights at `{}` of the stage below, and stage {} holds no motor",
                    event.as_str(),
                    motor.stage + 1
                )
            })
            .map(Some)
    };
    let ejection = || -> Result<hpr_design::Ignition, String> {
        let Some(below) = below()? else {
            return Ok(hpr_design::Ignition::Never);
        };
        match below.delay {
            Some(Delay::Seconds(charge_s)) => Ok(hpr_design::Ignition::Burnout {
                mount: below.mount.clone(),
                delay_s: delay(charge_s, "ejection")? + delay_s,
            }),
            // Plugged: no charge fires, and OpenRocket flies the motor unlit.
            Some(Delay::Plugged) => Ok(hpr_design::Ignition::Never),
            _ => Err(format!(
                "it lights at the ejection charge of {} in the stage below, which states no \
                 delay",
                below.designation
            )),
        }
    };
    match event {
        IgnitionEvent::Launch => Ok(at(delay_s)),
        IgnitionEvent::Automatic if motor.stage == last_stage => Ok(at(delay_s)),
        IgnitionEvent::Automatic | IgnitionEvent::EjectionCharge => ejection(),
        IgnitionEvent::Burnout => Ok(below()?.map_or(hpr_design::Ignition::Never, |below| {
            hpr_design::Ignition::Burnout {
                mount: below.mount.clone(),
                delay_s,
            }
        })),
        IgnitionEvent::Never => Ok(hpr_design::Ignition::Never),
        IgnitionEvent::Other(word) => Err(format!("its ignition event `{word}` is not known")),
    }
}

/// How long `motor` burns, s.
fn burn_s(motor: &OrkMotor) -> Result<f64, String> {
    motor
        .curve
        .motor()
        .map(hpr_motor::SolidMotor::burnout_time_s)
        .ok_or_else(|| format!("{} has no thrust curve", motor.designation))
}

/// When each of `motors` lights, s after launch, given their ignitions `lit` in the same order:
/// `None` for one that never lights ([`never_lit`]). Every other one is known before the flight: a
/// `.ork` lights a motor at a time, or at the burnout of a motor in the stage below, whose time is
/// known in turn.
fn ignition_times_s(
    motors: &[OrkMotor],
    lit: &[hpr_design::Ignition],
    burns_s: &[f64],
) -> Result<Vec<Option<f64>>, String> {
    let never = never_lit(motors, lit);
    let mut times: Vec<Option<f64>> = vec![None; motors.len()];
    // Each pass settles at least one motor while any can be, so this many passes settle them all.
    for _ in 0..motors.len() {
        let mut settled = false;
        for index in 0..motors.len() {
            if times[index].is_some() || never[index] {
                continue;
            }
            let time = match &lit[index] {
                hpr_design::Ignition::Launch => Some(0.0),
                hpr_design::Ignition::Time { time_s } => Some(*time_s),
                hpr_design::Ignition::Burnout { mount, delay_s } => motors
                    .iter()
                    .position(|m| m.mount == *mount)
                    .and_then(|at| times[at].map(|time| time + burns_s[at] + delay_s)),
                _ => None,
            };
            if time.is_some() {
                times[index] = time;
                settled = true;
            }
        }
        if !settled {
            break;
        }
    }
    if (0..motors.len()).any(|index| times[index].is_none() && !never[index]) {
        return Err("a motor's ignition waits on one whose time is not known".to_owned());
    }
    Ok(times)
}

/// `lit`, the ignitions of `motors` in the same order, with each motor that never lights
/// ([`never_lit`]) written as [`hpr_design::Ignition::Never`]; or why the configuration can't be
/// flown: no motor of it lights.
pub(super) fn never_when_waiting_on_never(
    motors: &[OrkMotor],
    lit: Vec<hpr_design::Ignition>,
) -> Result<Vec<hpr_design::Ignition>, String> {
    let never = never_lit(motors, &lit);
    if never.iter().all(|&never| never) {
        return Err("no motor of it ever lights, so the rocket would not leave the pad".to_owned());
    }
    Ok(lit
        .into_iter()
        .zip(never)
        .map(|(ignition, never)| {
            if never {
                hpr_design::Ignition::Never
            } else {
                ignition
            }
        })
        .collect())
}

/// Whether each of `motors`, lit by `lit` in the same order, never lights: it is set so
/// ([`hpr_design::Ignition::Never`]), or lit by the burnout of a mount whose motors never light,
/// as hpr's flight lights it.
fn never_lit(motors: &[OrkMotor], lit: &[hpr_design::Ignition]) -> Vec<bool> {
    let mut never: Vec<bool> = lit
        .iter()
        .map(|ignition| *ignition == hpr_design::Ignition::Never)
        .collect();
    // Each pass settles at least one more link of a chain, so this many passes settle them all.
    for _ in 0..motors.len() {
        let mut changed = false;
        for index in 0..motors.len() {
            if never[index] {
                continue;
            }
            if let hpr_design::Ignition::Burnout { mount, .. } = &lit[index] {
                let mut in_mount = (0..motors.len()).filter(|&i| motors[i].mount == *mount);
                if in_mount.all(|i| never[i]) {
                    never[index] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    never
}

/// The ignition of the motors in `stage` as a separation's trigger `delay_s` after it, when they
/// light together at a time or a burnout.
fn lit_at(
    motors: &[OrkMotor],
    lit: &[hpr_design::Ignition],
    stage: usize,
    delay_s: f64,
    what: &str,
) -> Result<StagingTrigger, String> {
    let mut ignitions = motors.iter().zip(lit).filter(|(m, _)| m.stage == stage);
    let Some((_, first)) = ignitions.next() else {
        return Err(format!(
            "it separates at {what}'s ignition, and stage {stage} holds no motor"
        ));
    };
    if ignitions.any(|(_, other)| other != first) {
        return Err(format!(
            "it separates at {what}'s ignition, and stage {stage}'s motors light at different times"
        ));
    }
    match first {
        hpr_design::Ignition::Launch => Ok(StagingTrigger::Time { time_s: delay_s }),
        hpr_design::Ignition::Time { time_s } => Ok(StagingTrigger::Time {
            time_s: time_s + delay_s,
        }),
        hpr_design::Ignition::Burnout {
            mount,
            delay_s: after_s,
        } => Ok(StagingTrigger::Burnout {
            mount: mount.clone(),
            delay_s: after_s + delay_s,
        }),
        hpr_design::Ignition::Never => Err(format!(
            "it separates at {what}'s ignition, which never comes"
        )),
        // The `.ork` reading never gives one, and a separation timed from the ignition that a
        // separation causes could never fire.
        _ => Err(format!(
            "it separates at {what}'s ignition, which waits on a separation"
        )),
    }
}

/// The powered separation configuration `id` flies, `None` when it flies none, or why hpr can't
/// fly its stages as the file says. `motors` are its motors and `lit` their ignitions, in order;
/// `stages` is the count of the rocket's stages.
pub(super) fn staging(
    id: &str,
    motors: &[OrkMotor],
    lit: &[hpr_design::Ignition],
    separations: &[StageSeparation],
    stages: usize,
) -> Result<Option<Staging>, String> {
    let never = never_lit(motors, lit);
    // A stage's own motors, when it holds some and none of them ever lights: a separation at
    // their burnout or ejection charge never comes, and OpenRocket keeps the stage on.
    let unlit = |stage: usize| {
        let mut own = (0..motors.len()).filter(|&i| motors[i].stage == stage);
        own.clone().next().is_some() && own.all(|i| never[i])
    };
    let mut active = Vec::new();
    for stage in 1..stages {
        let setting = separations
            .iter()
            .find(|s| s.stage == stage)
            .map(|s| s.separation_in(id));
        match setting.and_then(|s| s.event.as_ref().map(|event| (event, s))) {
            Some((SeparationEvent::Never, _)) => {}
            Some((SeparationEvent::Burnout | SeparationEvent::Ejection, _)) if unlit(stage) => {}
            Some((event, setting)) => active.push((stage, event, setting)),
            None => return Err(format!("stage {stage} states no separation event")),
        }
    }
    let [(stage, event, setting)] = active.as_slice() else {
        return if active.is_empty() {
            Ok(None)
        } else {
            Err(format!(
                "{} stages separate, and hpr flies one separation",
                active.len()
            ))
        };
    };
    let stage = *stage;
    // At or after apogee: the climb is the whole stack's, and the descent is not flown. That
    // assumes every motor is spent by apogee, which `cargo xtask ork-flights` checks of each
    // flight.
    if matches!(
        event,
        SeparationEvent::Apogee | SeparationEvent::AltitudeDescending
    ) {
        return Ok(None);
    }
    let delay_s = delay(setting.delay_s.unwrap_or(0.0), "separation")?;
    let own = || {
        one_mount(motors, stage)?
            .ok_or_else(|| format!("stage {stage} separates at its own motor, and holds none"))
    };
    let trigger = match event {
        SeparationEvent::Apogee | SeparationEvent::AltitudeDescending => return Ok(None),
        SeparationEvent::Launch => StagingTrigger::Time { time_s: delay_s },
        SeparationEvent::Ignition => lit_at(motors, lit, stage, delay_s, "its own motor")?,
        SeparationEvent::UpperIgnition => {
            lit_at(motors, lit, stage - 1, delay_s, "the stage above")?
        }
        SeparationEvent::Burnout => StagingTrigger::Burnout {
            mount: own()?.mount.clone(),
            delay_s,
        },
        SeparationEvent::Ejection => {
            let own = own()?;
            match own.delay {
                Some(Delay::Seconds(charge_s)) => StagingTrigger::Burnout {
                    mount: own.mount.clone(),
                    delay_s: delay(charge_s, "ejection")? + delay_s,
                },
                _ => {
                    return Err(format!(
                        "stage {stage} separates at the ejection charge of {}, which has none \
                         (plugged, or no delay given)",
                        own.designation
                    ));
                }
            }
        }
        SeparationEvent::AltitudeAscending => {
            return Err(format!(
                "stage {stage} separates at a height on the way up, which hpr has no trigger for"
            ));
        }
        _ => {
            return Err(format!(
                "stage {stage} separates at `{}`, which hpr has no trigger for",
                event.as_str()
            ));
        }
    };
    let burns_s = motors.iter().map(burn_s).collect::<Result<Vec<_>, _>>()?;
    let times_s = ignition_times_s(motors, lit, &burns_s)?;
    let time_s = match &trigger {
        StagingTrigger::Time { time_s } => *time_s,
        StagingTrigger::Burnout { mount, delay_s } => {
            let at = motors
                .iter()
                .position(|m| m.mount == *mount)
                .ok_or_else(|| format!("no motor sits in mount `{mount}`"))?;
            // The stage's motors light (`unlit` let through no other), so the time is known.
            let lit_s = times_s[at]
                .ok_or_else(|| format!("stage {stage} separates at a motor that never lights"))?;
            lit_s + burns_s[at] + delay_s
        }
    };
    // hpr's test, when the separation fires: the part ahead has a motor burning or still to
    // light, and the part behind has none (flight.rs). A motor that never lights does neither.
    let burning =
        |index: usize| times_s[index].is_some_and(|lit_s| lit_s + burns_s[index] > time_s);
    if !(0..motors.len()).any(|i| motors[i].stage < stage && burning(i)) {
        return Err(format!(
            "stage {stage} separates at {time_s} s with no motor ahead of it left to burn, which \
             can come before apogee, and hpr would fly both parts without their airframes' drag"
        ));
    }
    if let Some((behind, lit_s)) = (0..motors.len())
        .filter(|&i| motors[i].stage >= stage && burning(i))
        .find_map(|i| times_s[i].map(|lit_s| (i, lit_s)))
    {
        return Err(format!(
            "stage {stage} separates at {time_s} s, while {} behind it burns until {} s",
            motors[behind].designation,
            lit_s + burns_s[behind]
        ));
    }
    // hpr's flight fires a separation only once the rocket is off the rod (flight.rs), so one at
    // launch would come late.
    if time_s <= 0.0 {
        return Err(format!(
            "stage {stage} separates at launch, on the pad, where hpr's flight fires no separation"
        ));
    }
    Ok(Some(Staging {
        after_stage: stage - 1,
        trigger,
        time_s,
    }))
}
