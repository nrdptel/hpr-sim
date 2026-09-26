//! Structural design checks: typed findings about a design that resolves but can't be built or
//! flown as described.
//!
//! A finding is an [`Severity::Error`] when the design is physically impossible and a simulation of
//! it would be wrong, usually on the flattering side: a motor wider than its mount (Loft reported
//! +69% apogee for one), or fins whose root touches no part of the tube they belong to. A
//! [`Severity::Warning`] marks something unusual that can be real, such as a motor mount that
//! sticks out of the airframe. Callers that simulate should refuse designs with errors.
//!
//! Lengths are compared with [`LENGTH_TOLERANCE_M`] of slack, so round-off never raises a finding.
//!
//! See `docs/physics/design.md`.

use serde::{Deserialize, Serialize};

use crate::config::Configuration;
use crate::error::DesignError;
use crate::tree::{LENGTH_TOLERANCE_M, Layout, Part, PlacedComponent, Rocket};

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Unusual, but it can be built and flown.
    Warning,
    /// Impossible as described; a simulation would be wrong.
    Error,
}

/// A problem found in a design. Serialized with a `kind` tag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Finding {
    /// The motor case is wider than the mount's inside diameter (error).
    MotorWiderThanMount {
        /// Configuration id.
        configuration: String,
        /// Mount id.
        mount: String,
        /// Motor case diameter, m.
        motor_diameter_m: f64,
        /// Mount inside diameter, m.
        mount_inner_diameter_m: f64,
    },
    /// The motor case doesn't overlap its mount along the axis at all, such as an overhang typed
    /// in millimetres as metres (error).
    MotorOutsideMount {
        /// Configuration id.
        configuration: String,
        /// Mount id.
        mount: String,
    },
    /// The motor case reaches forward past the mount's forward end (warning).
    MotorPastMountTop {
        /// Configuration id.
        configuration: String,
        /// Mount id.
        mount: String,
        /// How far past, m.
        excess_m: f64,
    },
    /// An external part (fin root, tube fins, lug or rail button) doesn't overlap the body tube it
    /// is attached to at all (error).
    AttachmentOffBody {
        /// Component id.
        component: String,
        /// Body tube id.
        body: String,
    },
    /// An external part runs past an end of its body tube (warning).
    AttachmentPastBodyEnd {
        /// Component id.
        component: String,
        /// Body tube id.
        body: String,
        /// How far past, m.
        excess_m: f64,
    },
    /// An internal part lies wholly forward of the nose tip or aft of the rocket's end, and touches
    /// none of the parts it hangs from (a motor mount may stick out) (error).
    PartOutsideRocket {
        /// Component id.
        component: String,
    },
    /// An internal part runs past an end of its parent (warning).
    InternalPartPastParentEnd {
        /// Component id.
        component: String,
        /// Parent id.
        parent: String,
        /// How far past, m.
        excess_m: f64,
    },
    /// An internal part reaches farther from its parent's axis than the parent has room: a tube's
    /// bore, or a nose cone's or transition's largest outer radius (error).
    InternalPartWiderThanParent {
        /// Component id.
        component: String,
        /// Parent id.
        parent: String,
        /// How far the part reaches from the parent's axis, m.
        reach_m: f64,
        /// The room in the parent, m.
        room_m: f64,
    },
    /// Two tubes of a cluster are closer than a tube's diameter, so they cross: the mass where they
    /// cross is counted twice and their motors would not fit (warning).
    ClusterTubesOverlap {
        /// Inner tube id.
        tube: String,
        /// The distance between the closest two tubes' axes, m.
        apart_m: f64,
        /// The tube's outer diameter, m.
        diameter_m: f64,
    },
    /// A centering ring overlaps an inner tube beside it, so the mass where they cross is counted
    /// twice; an automatic inner radius only clears on-axis tubes (warning).
    RingOverlapsInnerTube {
        /// The ring's id.
        ring: String,
        /// The tube's id.
        tube: String,
    },
    /// A stage with an axial centre-of-mass override (`cg_aft_m`), its own or a component's, has
    /// its centre forward of the nose tip or aft of the rocket's end although every internal part
    /// in it is on the rocket, such as a centre typed in millimetres as metres (error).
    CentreOutsideRocket {
        /// The stage's id.
        stage: String,
        /// The centre's station, m.
        station_m: f64,
    },
    /// Adjacent body components' radii differ where they meet (warning).
    RadiusStep {
        /// The forward component's id.
        fore: String,
        /// The aft component's id.
        aft: String,
        /// The forward component's aft radius, m.
        fore_radius_m: f64,
        /// The aft component's forward radius, m.
        aft_radius_m: f64,
    },
    /// The first body component is not a nose cone (warning).
    NoNoseCone {
        /// The first component's id.
        component: String,
    },
}

impl Finding {
    /// The finding's severity.
    pub fn severity(&self) -> Severity {
        match self {
            Self::MotorWiderThanMount { .. }
            | Self::MotorOutsideMount { .. }
            | Self::AttachmentOffBody { .. }
            | Self::PartOutsideRocket { .. }
            | Self::InternalPartWiderThanParent { .. }
            | Self::CentreOutsideRocket { .. } => Severity::Error,
            Self::MotorPastMountTop { .. }
            | Self::AttachmentPastBodyEnd { .. }
            | Self::InternalPartPastParentEnd { .. }
            | Self::ClusterTubesOverlap { .. }
            | Self::RingOverlapsInnerTube { .. }
            | Self::RadiusStep { .. }
            | Self::NoNoseCone { .. } => Severity::Warning,
        }
    }
}

/// Every check on a design and all its configurations.
///
/// # Errors
///
/// As [`Rocket::layout`], [`Rocket::check_configuration_ids`] and [`Layout::place_motors`]: a
/// design that doesn't resolve can't be checked.
pub fn check(rocket: &Rocket) -> Result<Vec<Finding>, DesignError> {
    rocket.check_configuration_ids()?;
    let layout = rocket.layout()?;
    let mut findings = check_layout(&layout);
    for configuration in &rocket.configurations {
        findings.extend(check_configuration(&layout, configuration)?);
    }
    Ok(findings)
}

/// Whether any finding is an error.
pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity() == Severity::Error)
}

/// The overlap of `[a0, a1]` and `[b0, b1]`, m (negative when they are apart).
fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    a1.min(b1) - a0.max(b0)
}

/// How far `[fore, aft]` runs past `[parent_fore, parent_aft]`, m, or `None` when the two don't
/// overlap at all.
fn excess(fore: f64, aft: f64, parent_fore: f64, parent_aft: f64) -> Option<f64> {
    if overlap(fore, aft, parent_fore, parent_aft) <= 0.0 {
        return None;
    }
    Some((parent_fore - fore).max(aft - parent_aft).max(0.0))
}

/// The checks on the structure alone.
///
/// The stage-centre check reads the `centre_overridden` flags that [`crate::Rocket::layout`] sets;
/// a layout built by hand without them never raises [`Finding::CentreOutsideRocket`].
pub fn check_layout(layout: &Layout) -> Vec<Finding> {
    let mut findings = Vec::new();
    let body: Vec<_> = layout.body().collect();
    if let Some(first) = body.first()
        && !matches!(first.part, Part::NoseCone(_))
    {
        findings.push(Finding::NoNoseCone {
            component: first.id.clone(),
        });
    }
    for pair in body.windows(2) {
        let (fore, aft) = (pair[0], pair[1]);
        if let (Some(a), Some(b)) = (fore.part.aft_radius_m(), aft.part.fore_radius_m())
            && (a - b).abs() > LENGTH_TOLERANCE_M
        {
            findings.push(Finding::RadiusStep {
                fore: fore.id.clone(),
                aft: aft.id.clone(),
                fore_radius_m: a,
                aft_radius_m: b,
            });
        }
    }
    // Internal parts wholly off the rocket and off every part they hang from, and the components
    // holding one.
    let length = layout.length_m;
    // A part that only touches a span at an end face is on it, within round-off.
    let apart = |c: &PlacedComponent, fore: f64, aft: f64| {
        overlap(c.fore_station_m, c.aft_station_m(), fore, aft) < -LENGTH_TOLERANCE_M
    };
    let off_rocket: Vec<bool> = layout
        .components
        .iter()
        .map(|c| {
            if c.parent.is_none() || c.part.is_external() || !apart(c, 0.0, length) {
                return false;
            }
            let mut ancestor = c.parent;
            for _ in 0..layout.components.len() {
                let Some(a) = ancestor.and_then(|i| layout.components.get(i)) else {
                    break;
                };
                if !apart(c, a.fore_station_m, a.aft_station_m()) {
                    return false;
                }
                ancestor = a.parent;
            }
            true
        })
        .collect();
    let mut holds_off = off_rocket.clone();
    for (index, component) in layout.components.iter().enumerate().rev() {
        if holds_off[index]
            && let Some(flag) = component.parent.and_then(|p| holds_off.get_mut(p))
        {
            *flag = true;
        }
    }
    for (k, stage) in layout.stages.iter().enumerate() {
        let station = -stage.mass.cg_m.z;
        let in_stage = || {
            layout
                .components
                .iter()
                .enumerate()
                .filter(|(_, c)| c.stage == k)
        };
        let holds = in_stage().any(|(i, _)| holds_off[i]);
        let overridden = stage.centre_overridden || in_stage().any(|(_, c)| c.centre_overridden);
        if overridden
            && !holds
            && stage.mass.mass_kg > 0.0
            && !(-LENGTH_TOLERANCE_M..=length + LENGTH_TOLERANCE_M).contains(&station)
        {
            findings.push(Finding::CentreOutsideRocket {
                stage: stage.id.clone(),
                station_m: station,
            });
        }
    }
    for (index, component) in layout.components.iter().enumerate() {
        let Some(parent) = component.parent.and_then(|i| layout.components.get(i)) else {
            continue;
        };
        attached_findings(component, parent, off_rocket[index], layout, &mut findings);
    }
    findings
}

/// The checks on one attached part.
fn attached_findings(
    component: &PlacedComponent,
    parent: &PlacedComponent,
    off_rocket: bool,
    layout: &Layout,
    findings: &mut Vec<Finding>,
) {
    let (fore, aft) = (component.fore_station_m, component.aft_station_m());
    let span = excess(fore, aft, parent.fore_station_m, parent.aft_station_m());
    if component.part.is_external() {
        match span {
            None => findings.push(Finding::AttachmentOffBody {
                component: component.id.clone(),
                body: parent.id.clone(),
            }),
            Some(e) if e > LENGTH_TOLERANCE_M => {
                findings.push(Finding::AttachmentPastBodyEnd {
                    component: component.id.clone(),
                    body: parent.id.clone(),
                    excess_m: e,
                });
            }
            Some(_) => {}
        }
        return;
    }
    // An internal part entirely outside its parent runs past it by at least its own length.
    let e =
        span.unwrap_or_else(|| (parent.fore_station_m - fore).max(aft - parent.aft_station_m()));
    if off_rocket {
        findings.push(Finding::PartOutsideRocket {
            component: component.id.clone(),
        });
    } else if e > LENGTH_TOLERANCE_M {
        findings.push(Finding::InternalPartPastParentEnd {
            component: component.id.clone(),
            parent: parent.id.clone(),
            excess_m: e,
        });
    }
    let room = parent
        .part
        .inner_radius_m()
        .or_else(|| parent.part.max_radius_m().ok().flatten());
    if let (Some(reach), Some(room)) = (
        component.part.reach_from_m(parent.part.axis_offset_m()),
        room,
    ) && reach > room + LENGTH_TOLERANCE_M
    {
        findings.push(Finding::InternalPartWiderThanParent {
            component: component.id.clone(),
            parent: parent.id.clone(),
            reach_m: reach,
            room_m: room,
        });
    }
    if let Part::InnerTube(inner) = &component.part {
        let apart_m = inner
            .cluster_m
            .iter()
            .enumerate()
            .flat_map(|(i, &[x, y])| {
                inner.cluster_m[i + 1..]
                    .iter()
                    .map(move |&[u, v]| (x - u).hypot(y - v))
            })
            .fold(f64::INFINITY, f64::min);
        let diameter_m = 2.0 * inner.outer_radius_m;
        if apart_m < diameter_m - LENGTH_TOLERANCE_M {
            findings.push(Finding::ClusterTubesOverlap {
                tube: component.id.clone(),
                apart_m,
                diameter_m,
            });
        }
    }
    if let Part::CenteringRing(ring) = &component.part {
        for tube in layout
            .components
            .iter()
            .filter(|c| c.parent == component.parent && c.id != component.id)
        {
            let Part::InnerTube(inner) = &tube.part else {
                continue;
            };
            // Each tube (every tube of a cluster) covers radii [d − R, d + R] about the ring's
            // (the body) axis.
            let [x, y] = tube.part.axis_offset_m();
            let tubes = if inner.cluster_m.is_empty() {
                &[[0.0, 0.0]][..]
            } else {
                inner.cluster_m.as_slice()
            };
            let radial = tubes
                .iter()
                .map(|&[u, v]| {
                    let d = (x + u).hypot(y + v);
                    (d + inner.outer_radius_m).min(ring.outer_radius_m)
                        - (d - inner.outer_radius_m).max(ring.inner_radius_m)
                })
                .fold(f64::NEG_INFINITY, f64::max);
            if overlap(fore, aft, tube.fore_station_m, tube.aft_station_m()) > LENGTH_TOLERANCE_M
                && radial > LENGTH_TOLERANCE_M
            {
                findings.push(Finding::RingOverlapsInnerTube {
                    ring: component.id.clone(),
                    tube: tube.id.clone(),
                });
            }
        }
    }
}

/// The checks on one configuration's motors in `layout`. The configuration need not be one of the
/// rocket's own, so a candidate motor can be checked before it is stored.
///
/// # Errors
///
/// As [`Layout::place_motors`].
pub fn check_configuration(
    layout: &Layout,
    configuration: &Configuration,
) -> Result<Vec<Finding>, DesignError> {
    let mut findings = Vec::new();
    // Every tube of a cluster holds the same motor the same way along the axis, so its first tube
    // speaks for the mount and each finding is made once.
    let placed = layout.place_motors(configuration)?;
    for motor in placed.into_iter().filter(|motor| motor.tube == 0) {
        let Some((_, mount)) = layout.find(&motor.mount) else {
            continue;
        };
        if let Some(inner) = mount.part.inner_radius_m() {
            let inner_diameter = 2.0 * inner;
            if motor.mounted.diameter_m > inner_diameter + LENGTH_TOLERANCE_M {
                findings.push(Finding::MotorWiderThanMount {
                    configuration: configuration.id.clone(),
                    mount: mount.id.clone(),
                    motor_diameter_m: motor.mounted.diameter_m,
                    mount_inner_diameter_m: inner_diameter,
                });
            }
        }
        let (fore, nozzle) = (motor.fore_station_m(), motor.nozzle_station_m());
        if overlap(fore, nozzle, mount.fore_station_m, mount.aft_station_m()) <= 0.0 {
            findings.push(Finding::MotorOutsideMount {
                configuration: configuration.id.clone(),
                mount: mount.id.clone(),
            });
            continue;
        }
        let past = mount.fore_station_m - fore;
        if past > LENGTH_TOLERANCE_M {
            findings.push(Finding::MotorPastMountTop {
                configuration: configuration.id.clone(),
                mount: mount.id.clone(),
                excess_m: past,
            });
        }
    }
    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        attached, body, bottom, inner_tube, mass_component, ring, rocket, stage, three_fin_rocket,
        top, tube,
    };
    use crate::{AutoDimension, MotorMount};
    use crate::{Part, Position};

    fn kinds(findings: &[Finding]) -> Vec<(String, Severity)> {
        findings
            .iter()
            .map(|f| {
                let tag = serde_json::to_value(f).unwrap()["kind"]
                    .as_str()
                    .unwrap()
                    .to_owned();
                (tag, f.severity())
            })
            .collect()
    }

    #[test]
    fn sample_rocket_passes_every_check() {
        let findings = check(&three_fin_rocket()).unwrap();
        assert!(findings.is_empty(), "{findings:?}");
    }

    /// Loft lesson L50: a 54 mm motor in a 38 mm mount flew, and flew high. Here it is an error, a
    /// motor exactly as wide as the bore is not, and a case longer than the mount only warns.
    #[test]
    fn motor_wider_than_mount_is_rejected() {
        let mut design = three_fin_rocket();
        design.configurations[0].motors[0] = crate::testing::motor("mmt", 0.054, 0.2);
        let findings = check(&design).unwrap();
        assert_eq!(
            findings,
            vec![Finding::MotorWiderThanMount {
                configuration: "main".to_owned(),
                mount: "mmt".to_owned(),
                motor_diameter_m: 0.054,
                mount_inner_diameter_m: 2.0 * (0.020 - 0.001),
            }]
        );
        assert_eq!(findings[0].severity(), Severity::Error);
        assert!(has_errors(&findings));
        // A cluster of three such tubes is still one finding: the motor is named once.
        if let Part::InnerTube(tube) = &mut design.stages[0].components[1].children[0].part {
            tube.cluster_m = vec![[0.0, 0.0], [0.04, 0.0], [-0.04, 0.0]];
        }
        let findings = check(&design).unwrap();
        let wider = findings
            .iter()
            .filter(|f| matches!(f, Finding::MotorWiderThanMount { .. }))
            .count();
        assert_eq!(wider, 1, "{findings:?}");

        // The sample's 38 mm motor fills its 38 mm bore exactly: no finding.
        let exact = three_fin_rocket();
        assert!(check(&exact).unwrap().is_empty());
        // Wider by more than round-off is an error again.
        let mut design = three_fin_rocket();
        design.configurations[0].motors[0] = crate::testing::motor("mmt", 0.038 + 1e-6, 0.2);
        assert!(has_errors(&check(&design).unwrap()));

        // A 0.35 m case in a 0.3 m mount with 0.01 m of overhang reaches 0.04 m past its top.
        let mut design = three_fin_rocket();
        design.configurations[0].motors[0] = crate::testing::motor("mmt", 0.038, 0.35);
        let findings = check(&design).unwrap();
        let [Finding::MotorPastMountTop { excess_m, .. }] = findings[..] else {
            panic!("{findings:?}")
        };
        assert!((excess_m - 0.04).abs() < 1e-12);
        assert!(!has_errors(&findings));
    }

    /// Loft lesson L50: fins could sit off the airframe. A fin root that touches none of its body
    /// tube is an error; one that runs past the tube's end warns.
    #[test]
    fn fin_root_must_touch_body() {
        let with_fins = |offset: f64| {
            let mut design = three_fin_rocket();
            design.stages[0].components[1].children[3].position = Some(bottom(offset));
            check(&design).unwrap()
        };
        // Root chord 0.1 m; the tube ends at station 1.0.
        let off = with_fins(0.15);
        assert_eq!(
            off,
            vec![Finding::AttachmentOffBody {
                component: "fins".to_owned(),
                body: "airframe".to_owned(),
            }]
        );
        assert!(has_errors(&off));
        // Touching only at the tube's end is still off the body.
        assert!(has_errors(&with_fins(0.1)));
        let past = with_fins(0.04);
        let [
            Finding::AttachmentPastBodyEnd {
                ref component,
                excess_m,
                ..
            },
        ] = past[..]
        else {
            panic!("{past:?}")
        };
        assert_eq!(component, "fins");
        assert!((excess_m - 0.04).abs() < 1e-12);
        assert!(!has_errors(&past));
        assert!(with_fins(0.0).is_empty());
        // Forward of the tube too.
        let mut design = three_fin_rocket();
        design.stages[0].components[1].children[3].position = Some(top(-0.2));
        assert!(has_errors(&check(&design).unwrap()));
    }

    #[test]
    fn internal_parts_radius_steps_and_a_missing_nose_are_found() {
        let mut airframe = body("airframe", tube(0.5, 0.03, 0.001));
        airframe.children = vec![
            // Wider than the 29 mm bore.
            attached("wide-ring", ring(0.005, 0.03, 0.01), top(0.1)),
            // Hanging 0.05 m out of the tube's aft end.
            attached("long-mmt", inner_tube(0.2, 0.01, 0.001), bottom(0.05)),
            // Entirely forward of the tube.
            attached("lost", mass_component(0.1, 0.05, 0.01), top(-0.2)),
        ];
        let mut mount = airframe.children[1].clone();
        mount.motor_mount = Some(MotorMount::default());
        airframe.children[1] = mount;
        let design = rocket(vec![stage(
            "s",
            vec![airframe, body("tail", tube(0.1, 0.02, 0.001))],
        )]);
        let findings = check(&design).unwrap();
        assert_eq!(
            kinds(&findings),
            vec![
                ("no_nose_cone".to_owned(), Severity::Warning),
                ("radius_step".to_owned(), Severity::Warning),
                (
                    "internal_part_wider_than_parent".to_owned(),
                    Severity::Error
                ),
                (
                    "internal_part_past_parent_end".to_owned(),
                    Severity::Warning
                ),
                // Wholly forward of the nose tip: one error, not also a warning.
                ("part_outside_rocket".to_owned(), Severity::Error),
            ]
        );

        // A ring left solid across the motor mount counts their mass twice.
        let mut design = three_fin_rocket();
        design.stages[0].components[1].children[1].auto = vec![AutoDimension::OuterRadius];
        assert_eq!(
            check(&design).unwrap(),
            vec![Finding::RingOverlapsInnerTube {
                ring: "ring-fore".to_owned(),
                tube: "mmt".to_owned(),
            }]
        );
    }

    /// Radial room is measured about the parent's own axis: a block centred in an off-axis pod
    /// fits, and a mass on the body axis attached to the pod doesn't. Rings in a cluster warn.
    #[test]
    fn internal_parts_are_measured_from_their_parents_axis() {
        let offset = |part: Part, r: f64| match part {
            Part::InnerTube(mut t) => {
                t.radial_offset_m = r;
                Part::InnerTube(t)
            }
            Part::MassComponent(mut m) => {
                m.packing.radial_offset_m = r;
                Part::MassComponent(m)
            }
            other => other,
        };
        let mut pod = attached(
            "pod",
            offset(inner_tube(0.3, 0.02, 0.001), 0.025),
            bottom(0.0),
        );
        pod.children = vec![attached(
            "block",
            offset(inner_tube(0.01, 0.019, 0.003), 0.025),
            top(0.0),
        )];
        let mut airframe = body("airframe", tube(0.8, 0.05, 0.002));
        airframe.children = vec![pod, attached("ring", ring(0.005, 0.048, 0.0), bottom(-0.1))];
        let design = rocket(vec![stage(
            "s",
            vec![body("nose", crate::testing::nose(0.2, 0.05)), airframe],
        )]);
        assert_eq!(
            check(&design).unwrap(),
            vec![Finding::RingOverlapsInnerTube {
                ring: "ring".to_owned(),
                tube: "pod".to_owned(),
            }]
        );

        let mut design = design;
        design.stages[0].components[1].children[0].children[0] =
            attached("on-axis", mass_component(0.1, 0.05, 0.004), top(0.1));
        let findings = check(&design).unwrap();
        let wide = findings
            .iter()
            .find_map(|f| match f {
                Finding::InternalPartWiderThanParent {
                    component,
                    reach_m,
                    room_m,
                    ..
                } if component == "on-axis" => Some((*reach_m, *room_m)),
                _ => None,
            })
            .unwrap();
        assert!(
            (wide.0 - 0.029).abs() < 1e-15 && (wide.1 - 0.019).abs() < 1e-15,
            "{wide:?}"
        );
    }

    /// A motor that isn't in its mount at all is an error, whichever way it missed.
    #[test]
    fn motor_outside_its_mount_is_rejected() {
        for overhang in [5.0, 0.2, -0.3] {
            let mut design = three_fin_rocket();
            design.stages[0].components[1].children[0].motor_mount = Some(MotorMount {
                overhang_m: overhang,
            });
            let findings = check(&design).unwrap();
            assert_eq!(
                findings,
                vec![Finding::MotorOutsideMount {
                    configuration: "main".to_owned(),
                    mount: "mmt".to_owned(),
                }],
                "{overhang}"
            );
            assert!(has_errors(&findings));
        }
    }

    /// Parts wholly off the rocket are errors, and a stage centre moved off it by an override too.
    #[test]
    fn parts_and_centres_off_the_rocket_are_rejected() {
        let mut design = three_fin_rocket();
        design.stages[0].components[1].children[4].position = Some(top(2.0));
        assert_eq!(
            check(&design).unwrap(),
            vec![Finding::PartOutsideRocket {
                component: "chute".to_owned(),
            }]
        );
        let mut design = three_fin_rocket();
        design.stages[0].overrides.cg_aft_m = Some(350.0);
        let findings = check(&design).unwrap();
        assert!(
            matches!(&findings[..], [Finding::CentreOutsideRocket { stage, station_m }]
                if stage == "sustainer" && *station_m == 350.0),
            "{findings:?}"
        );
        // A retainer on a motor mount sticking out past the airframe is on the rocket.
        let mut design = three_fin_rocket();
        let mount = &mut design.stages[0].components[1].children[0];
        mount.position = Some(bottom(0.012));
        mount.children = vec![attached(
            "retainer",
            mass_component(0.05, 0.01, 0.015),
            bottom(0.0),
        )];
        let findings = check(&design).unwrap();
        assert!(
            matches!(&findings[..], [Finding::InternalPartPastParentEnd { component, .. }]
                if component == "mmt"),
            "{findings:?}"
        );
        // So is one flush against the mount's aft end, wholly behind the airframe.
        let mut flush = design.clone();
        flush.stages[0].components[1].children[0].children[0].position = Some(top(0.3));
        let layout = flush.layout().unwrap();
        let (_, retainer) = layout.find("retainer").unwrap();
        assert!(
            retainer.fore_station_m >= layout.length_m,
            "the test needs it behind"
        );
        let findings = check(&flush).unwrap();
        assert!(
            findings
                .iter()
                .all(|f| !matches!(f, Finding::PartOutsideRocket { .. })),
            "{findings:?}"
        );
        assert!(!has_errors(&findings), "{findings:?}");
        // But one clear of the mount is off the rocket.
        let mut clear = design.clone();
        clear.stages[0].components[1].children[0].children[0].position = Some(top(0.35));
        assert_eq!(
            check(&clear).unwrap(),
            vec![
                Finding::InternalPartPastParentEnd {
                    component: "mmt".to_owned(),
                    parent: "airframe".to_owned(),
                    excess_m: check(&design)
                        .unwrap()
                        .iter()
                        .find_map(|f| match f {
                            Finding::InternalPartPastParentEnd { excess_m, .. } => Some(*excess_m),
                            _ => None,
                        })
                        .unwrap(),
                },
                Finding::PartOutsideRocket {
                    component: "retainer".to_owned(),
                },
            ]
        );
        // Fins trailing far past a light stage's end move its centre off the rocket, but with no
        // override that is geometry, not a typo.
        let mut can = body("can", tube(0.3, 0.03, 0.0005));
        let mut fins = attached("trailing-fins", crate::testing::fins(0.2, 0.1), bottom(0.0));
        if let Part::FinSet(set) = &mut fins.part {
            set.planform = crate::FinPlanform::Trapezoidal {
                root_chord_m: 0.2,
                tip_chord_m: 0.2,
                span_m: 0.1,
                sweep_m: 0.4,
            };
        }
        can.children = vec![fins];
        let design = rocket(vec![stage("s", vec![can])]);
        let layout = design.layout().unwrap();
        assert!(
            -layout.stages[0].mass.cg_m.z > layout.length_m,
            "the test needs it off"
        );
        assert!(!has_errors(&check(&design).unwrap()));
        // A mass override can't move a centre past its parts, so it doesn't arm the check.
        let mut heavier = design.clone();
        heavier.stages[0].components[0].children[0]
            .overrides
            .mass_kg = Some(1.0);
        assert!(!has_errors(&check(&heavier).unwrap()));
        // Nor can a sideways centre override.
        let mut sideways = design.clone();
        sideways.stages[0].components[0].children[0]
            .overrides
            .cg_xy_m = Some([0.0, 0.0]);
        assert!(!has_errors(&check(&sideways).unwrap()));
        // A component's axial centre override arms it.
        let mut moved = design.clone();
        moved.stages[0].components[0].children[0].overrides.cg_aft_m = Some(50.0);
        let findings = check(&moved).unwrap();
        assert!(
            matches!(&findings[..], [Finding::NoNoseCone { .. }, Finding::CentreOutsideRocket { stage, .. }] if stage == "s"),
            "{findings:?}"
        );
        // A point mass at the rocket's end is on it, whatever the round-off in the length
        // (0.1 + 0.7 is 0.7999999999999999).
        let mut design = rocket(vec![stage(
            "s",
            vec![
                body("nose", crate::testing::nose(0.1, 0.03)),
                body("airframe", tube(0.7, 0.03, 0.001)),
            ],
        )]);
        design.stages[0].components[1].children = vec![attached(
            "tail-weight",
            mass_component(0.05, 0.0, 0.01),
            Position::Absolute { station_m: 0.8 },
        )];
        assert!(design.layout().unwrap().length_m < 0.8);
        assert!(check(&design).unwrap().is_empty());
        // And a part flush behind the rocket's end is on it, however its length adds up
        // (0.1 + 0.2 is 0.30000000000000004, 0.15 + 0.15 is 0.3).
        for (nose, tube_length) in [(0.1, 0.2), (0.15, 0.15)] {
            let mut design = rocket(vec![stage(
                "s",
                vec![
                    body("nose", crate::testing::nose(nose, 0.03)),
                    body("airframe", tube(tube_length, 0.03, 0.001)),
                ],
            )]);
            design.stages[0].components[1].children = vec![attached(
                "tail-block",
                mass_component(0.05, 0.05, 0.01),
                Position::Absolute { station_m: 0.3 },
            )];
            let findings = check(&design).unwrap();
            assert!(
                matches!(&findings[..], [Finding::InternalPartPastParentEnd { component, .. }]
                    if component == "tail-block"),
                "{nose} + {tube_length}: {findings:?}"
            );
        }
        // Ballast wider than a nose cone.
        let mut design = three_fin_rocket();
        design.stages[0].components[0].children = vec![attached(
            "ballast",
            mass_component(0.1, 0.02, 0.2),
            bottom(0.0),
        )];
        assert!(matches!(
            &check(&design).unwrap()[..],
            [Finding::InternalPartWiderThanParent { component, .. }] if component == "ballast"
        ));
        // A check of a layout with a bad parent index skips it instead of panicking.
        let mut layout = three_fin_rocket().layout().unwrap();
        layout.components[2].parent = Some(999);
        let _ = check_layout(&layout);
    }

    /// A ring of three 40 mm tubes touches at `40 mm / √3` from the axis and crosses inside it.
    #[test]
    fn a_cluster_whose_tubes_cross_is_flagged() {
        let at = |r: f64| {
            let mut design = three_fin_rocket();
            if let Part::InnerTube(tube) = &mut design.stages[0].components[1].children[0].part {
                tube.cluster_m = [90.0_f64, 210.0, 330.0]
                    .iter()
                    .map(|a| [r * a.to_radians().cos(), r * a.to_radians().sin()])
                    .collect();
            }
            check(&design)
                .unwrap()
                .into_iter()
                .filter(|f| matches!(f, Finding::ClusterTubesOverlap { .. }))
                .collect::<Vec<_>>()
        };
        assert!(at(0.04 / 3.0_f64.sqrt()).is_empty());
        let [
            Finding::ClusterTubesOverlap {
                tube,
                apart_m,
                diameter_m,
            },
        ] = &at(0.02)[..]
        else {
            panic!("one finding");
        };
        assert_eq!(tube, "mmt");
        assert!((apart_m - 0.02 * 3.0_f64.sqrt()).abs() < 1e-15, "{apart_m}");
        assert_eq!(*diameter_m, 0.04);
        assert_eq!(at(0.02)[0].severity(), Severity::Warning);
    }
}
