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

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::Configuration;
use crate::error::DesignError;
use crate::tree::{LENGTH_TOLERANCE_M, Layout, Part, Rocket};

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
    /// An internal part runs past an end of its parent (warning).
    InternalPartPastParentEnd {
        /// Component id.
        component: String,
        /// Parent id.
        parent: String,
        /// How far past, m.
        excess_m: f64,
    },
    /// An internal part reaches farther from the axis than its parent tube's inside wall (error).
    InternalPartWiderThanParent {
        /// Component id.
        component: String,
        /// Parent id.
        parent: String,
        /// How far the part reaches from the axis, m.
        radial_extent_m: f64,
        /// The parent's inner radius, m.
        parent_inner_radius_m: f64,
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
            | Self::AttachmentOffBody { .. }
            | Self::InternalPartWiderThanParent { .. } => Severity::Error,
            Self::MotorPastMountTop { .. }
            | Self::AttachmentPastBodyEnd { .. }
            | Self::InternalPartPastParentEnd { .. }
            | Self::RadiusStep { .. }
            | Self::NoNoseCone { .. } => Severity::Warning,
        }
    }
}

/// Every check on a design and all its configurations.
///
/// # Errors
///
/// As [`Rocket::layout`] and [`Rocket::place_motors`]: a design that doesn't resolve can't be
/// checked.
pub fn check(rocket: &Rocket) -> Result<Vec<Finding>, DesignError> {
    let layout = rocket.layout()?;
    let mut findings = check_layout(&layout);
    for configuration in &rocket.configurations {
        findings.extend(check_configuration(rocket, &layout, configuration)?);
    }
    Ok(findings)
}

/// Whether any finding is an error.
pub fn has_errors(findings: &[Finding]) -> bool {
    findings.iter().any(|f| f.severity() == Severity::Error)
}

/// How far `[fore, aft]` runs past `[parent_fore, parent_aft]`, m, or `None` when the two don't
/// overlap at all.
fn excess(fore: f64, aft: f64, parent_fore: f64, parent_aft: f64) -> Option<f64> {
    let overlap = aft.min(parent_aft) - fore.max(parent_fore);
    if overlap <= 0.0 {
        return None;
    }
    Some((parent_fore - fore).max(aft - parent_aft).max(0.0))
}

/// The checks on the structure alone.
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
    for component in &layout.components {
        let Some(parent_index) = component.parent else {
            continue;
        };
        let parent = &layout.components[parent_index];
        let span = excess(
            component.fore_station_m,
            component.aft_station_m(),
            parent.fore_station_m,
            parent.aft_station_m(),
        );
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
            continue;
        }
        // An internal part entirely outside its parent runs past it by at least its own length.
        let e = span.unwrap_or_else(|| {
            (parent.fore_station_m - component.fore_station_m)
                .max(component.aft_station_m() - parent.aft_station_m())
        });
        if e > LENGTH_TOLERANCE_M {
            findings.push(Finding::InternalPartPastParentEnd {
                component: component.id.clone(),
                parent: parent.id.clone(),
                excess_m: e,
            });
        }
        if let (Some(extent), Some(inner)) = (
            component.part.radial_extent_m(),
            parent.part.inner_radius_m(),
        ) && extent > inner + LENGTH_TOLERANCE_M
        {
            findings.push(Finding::InternalPartWiderThanParent {
                component: component.id.clone(),
                parent: parent.id.clone(),
                radial_extent_m: extent,
                parent_inner_radius_m: inner,
            });
        }
    }
    findings
}

/// The checks on one configuration's motors in `layout`, a resolved layout of `rocket`.
///
/// # Errors
///
/// As [`Rocket::place_motors`].
pub fn check_configuration(
    rocket: &Rocket,
    layout: &Layout,
    configuration: &Configuration,
) -> Result<Vec<Finding>, DesignError> {
    let assembly = rocket.place_motors(layout.clone(), &configuration.id)?;
    let components: BTreeMap<&str, _> = layout
        .components
        .iter()
        .map(|c| (c.id.as_str(), c))
        .collect();
    let mut findings = Vec::new();
    for motor in &assembly.motors {
        let Some(mount) = components.get(motor.mount.as_str()) else {
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
        let past = mount.fore_station_m - motor.fore_station_m();
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
                (
                    "internal_part_past_parent_end".to_owned(),
                    Severity::Warning
                ),
            ]
        );
        let Finding::InternalPartPastParentEnd { excess_m, .. } = &findings[4] else {
            panic!()
        };
        // A part wholly outside runs past by at least its length: here 0.2 m forward of the tube.
        assert!((excess_m - 0.2).abs() < 1e-12, "{excess_m}");

        // An automatic ring always fits.
        let mut design = three_fin_rocket();
        design.stages[0].components[1].children[1].auto = vec![AutoDimension::OuterRadius];
        assert!(check(&design).unwrap().is_empty());
    }
}
