//! The design tree: a rocket of stages, each a stack of body components from the nose aft with
//! parts attached to them, and how the tree resolves into placed parts and the rocket's structural
//! mass properties.
//!
//! **Stations and the body origin.** A *station* `s` is a distance aft of the nose tip, the way
//! design files state positions. The body frame's origin is the nose tip (`z_ref = 0` in
//! `docs/physics/frames.md`), so station `s` is body `z = −s`. A part's own frame has its origin at
//! its forward end ([`crate::mass`]), so a part whose forward end is at station `s` is translated by
//! `(0, 0, −s)`.
//!
//! **Body components** (nose cones, body tubes and transitions) are the stages' component lists.
//! They stack: each starts where the one before it ends, through every stage, from `s = 0`.
//! Shoulders don't count toward the stack.
//!
//! **Attached parts** are children of a component, placed along it by a [`Position`]. Fins, tube
//! fins, launch lugs and rail buttons attach to the outside of a body tube and take its outer
//! radius. Inner tubes, centering rings, mass components and recovery parts go inside a body
//! component or an inner tube. Only inner tubes have children of their own. Radial offsets are
//! always measured from the body axis.
//!
//! **Automatic radii** ([`AutoDimension`]) are taken from neighbours and parents when the tree
//! resolves; a stored value for an automatic dimension is ignored.
//!
//! **Mass.** A component's own mass properties come from its part's geometry, placed. Overrides
//! ([`Overrides`]) replace them, or the component together with everything attached to it; a
//! stage's overrides replace the whole stage. Overrides nested deeper apply first. Motors are never
//! part of the structure ([`crate::config`]).
//!
//! See `docs/physics/design.md` and the decision record on the design tree, [ADR-007][adr-007].
//!
//! [adr-007]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17

use std::collections::BTreeSet;

use hpr_core::{DMat3, DVec3};
use serde::{Deserialize, Serialize};

use crate::config::{Configuration, MotorMount};
use crate::error::DesignError;
use crate::finish::Finish;
use crate::fins::{FinSet, TubeFinSet};
use crate::mass::MassProperties;
use crate::parts::{
    BodyTube, CenteringRing, InnerTube, LaunchLug, MassComponent, NoseCone, Parachute, RailButton,
    ShockCord, Streamer, Transition,
};
use crate::shapes::check_dimension;

/// Slack when comparing lengths that should agree, m: far below any build tolerance and far above
/// the round-off in stations summed from millimetre inputs.
pub const LENGTH_TOLERANCE_M: f64 = 1e-9;

/// A rocket design: its stages, how its reference diameter is chosen, and its motor
/// configurations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rocket {
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Stages from the nose aft. The first holds the nose cone.
    pub stages: Vec<Stage>,
    /// How the reference diameter is chosen.
    #[serde(default)]
    pub reference_diameter: ReferenceDiameter,
    /// Motor configurations.
    #[serde(default)]
    pub configurations: Vec<Configuration>,
}

/// A stage: body components stacked from its forward end aft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    /// Unique id.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Body components (nose cones, body tubes, transitions), forward to aft.
    pub components: Vec<Component>,
    /// Overrides for the whole stage without its motors, applied after every override inside it. A
    /// centre-of-mass override is measured aft of the stage's forward end.
    #[serde(default, skip_serializing_if = "Overrides::is_empty")]
    pub overrides: Overrides,
}

/// A node of the design tree: a part, where it sits, and what hangs off it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    /// Unique id.
    pub id: String,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// The part, with its geometry and material.
    pub part: Part,
    /// Where an attached part sits along its parent. Body components (a stage's own list) have
    /// none: they stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    /// Dimensions taken from neighbours or the parent instead of the part's stored values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub auto: Vec<AutoDimension>,
    /// Makes a body tube or an inner tube a motor mount.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motor_mount: Option<MotorMount>,
    /// The outer surface's finish, for skin friction; `None` means [`Finish::default`]. Parts
    /// inside the body ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish: Option<Finish>,
    /// Mass, centre-of-mass and inertia overrides.
    #[serde(default, skip_serializing_if = "Overrides::is_empty")]
    pub overrides: Overrides,
    /// Whether the overrides replace this component together with everything attached to it
    /// (`true`), or this component alone (`false`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overrides_include_children: bool,
    /// Attached parts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Component>,
}

/// A part in the tree. Serialized as an object with one key, the part's kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Part {
    /// A nose cone (body component).
    NoseCone(NoseCone),
    /// A body tube (body component).
    BodyTube(BodyTube),
    /// A transition (body component).
    Transition(Transition),
    /// An inner tube (internal): a coupler, motor mount tube or engine block.
    InnerTube(InnerTube),
    /// A centering ring or bulkhead (internal).
    CenteringRing(CenteringRing),
    /// A fin set (external, on a body tube). Its axial extent is the root chord.
    FinSet(FinSet),
    /// Tube fins (external, on a body tube).
    TubeFinSet(TubeFinSet),
    /// Launch lugs (external, on a body tube). The extent covers the whole row.
    LaunchLug(LaunchLug),
    /// Rail buttons (external, on a body tube). The extent covers the whole row.
    RailButton(RailButton),
    /// A mass component (internal).
    MassComponent(MassComponent),
    /// A parachute (internal).
    Parachute(Parachute),
    /// A streamer (internal).
    Streamer(Streamer),
    /// A shock cord (internal).
    ShockCord(ShockCord),
}

/// Where a part belongs in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// A stage's stacked component.
    Body,
    /// On the outside of a body tube.
    External,
    /// Inside a body component or an inner tube.
    Internal,
}

impl Part {
    /// The part's kind, as serialized.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::NoseCone(_) => "nose_cone",
            Self::BodyTube(_) => "body_tube",
            Self::Transition(_) => "transition",
            Self::InnerTube(_) => "inner_tube",
            Self::CenteringRing(_) => "centering_ring",
            Self::FinSet(_) => "fin_set",
            Self::TubeFinSet(_) => "tube_fin_set",
            Self::LaunchLug(_) => "launch_lug",
            Self::RailButton(_) => "rail_button",
            Self::MassComponent(_) => "mass_component",
            Self::Parachute(_) => "parachute",
            Self::Streamer(_) => "streamer",
            Self::ShockCord(_) => "shock_cord",
        }
    }

    fn role(&self) -> Role {
        match self {
            Self::NoseCone(_) | Self::BodyTube(_) | Self::Transition(_) => Role::Body,
            Self::FinSet(_) | Self::TubeFinSet(_) | Self::LaunchLug(_) | Self::RailButton(_) => {
                Role::External
            }
            Self::InnerTube(_)
            | Self::CenteringRing(_)
            | Self::MassComponent(_)
            | Self::Parachute(_)
            | Self::Streamer(_)
            | Self::ShockCord(_) => Role::Internal,
        }
    }

    /// Whether this is a body component: a nose cone, body tube or transition.
    pub fn is_body(&self) -> bool {
        self.role() == Role::Body
    }

    /// Whether this attaches to the outside of a body tube: fins, tube fins, lugs, rail buttons.
    pub fn is_external(&self) -> bool {
        self.role() == Role::External
    }

    /// The axial extent used to place the part, m: a body component's length without shoulders, a
    /// fin set's root chord, a row of lugs or buttons from the first one's forward end to the last
    /// one's aft end, and a packed part's packed length.
    pub fn length_m(&self) -> f64 {
        let row =
            |count: u32, one: f64, spacing: f64| one + spacing * f64::from(count.saturating_sub(1));
        match self {
            Self::NoseCone(p) => p.length_m,
            Self::BodyTube(p) => p.length_m,
            Self::Transition(p) => p.length_m,
            Self::InnerTube(p) => p.length_m,
            Self::CenteringRing(p) => p.length_m,
            Self::FinSet(p) => p.planform.root_chord_m(),
            Self::TubeFinSet(p) => p.length_m,
            Self::LaunchLug(p) => row(p.count, p.length_m, p.spacing_m),
            Self::RailButton(p) => row(p.count, p.outer_diameter_m, p.spacing_m),
            Self::MassComponent(p) => p.packing.length_m,
            Self::Parachute(p) => p.packing.length_m,
            Self::Streamer(p) => p.packing.length_m,
            Self::ShockCord(p) => p.packing.length_m,
        }
    }

    /// A body component's outer radius at its forward end, m (zero at a nose tip).
    pub fn fore_radius_m(&self) -> Option<f64> {
        match self {
            Self::NoseCone(_) => Some(0.0),
            Self::BodyTube(p) => Some(p.outer_radius_m),
            Self::Transition(p) => Some(p.fore_radius_m),
            _ => None,
        }
    }

    /// A body component's outer radius at its aft end, m.
    pub fn aft_radius_m(&self) -> Option<f64> {
        match self {
            Self::NoseCone(p) => Some(p.base_radius_m),
            Self::BodyTube(p) => Some(p.outer_radius_m),
            Self::Transition(p) => Some(p.aft_radius_m),
            _ => None,
        }
    }

    /// A body component's largest outer radius anywhere along it, m.
    ///
    /// # Errors
    ///
    /// Profile errors for a nose cone or transition.
    pub fn max_radius_m(&self) -> Result<Option<f64>, DesignError> {
        Ok(match self {
            Self::NoseCone(p) => Some(p.profile()?.max_radius_m()),
            Self::BodyTube(p) => Some(p.outer_radius_m),
            Self::Transition(p) => Some(p.profile()?.max_radius_m()),
            _ => None,
        })
    }

    /// The inside radius of a body tube or inner tube, m: where internal parts fit.
    pub fn inner_radius_m(&self) -> Option<f64> {
        match self {
            Self::BodyTube(p) => Some(p.outer_radius_m - p.thickness_m),
            Self::InnerTube(p) => Some(p.outer_radius_m - p.thickness_m),
            _ => None,
        }
    }

    /// Where the part's own axis crosses the `x`–`y` plane, `[x, y]` in body axes, m: an inner
    /// tube's or packed part's radial offset turned by its angle, and the body axis for every
    /// other part.
    pub fn axis_offset_m(&self) -> [f64; 2] {
        let turned = |r: f64, angle: f64| [r * angle.cos(), r * angle.sin()];
        match self {
            Self::InnerTube(p) => turned(p.radial_offset_m, p.angle_rad),
            Self::MassComponent(p) => turned(p.packing.radial_offset_m, p.packing.angle_rad),
            Self::Parachute(p) => turned(p.packing.radial_offset_m, p.packing.angle_rad),
            Self::Streamer(p) => turned(p.packing.radial_offset_m, p.packing.angle_rad),
            Self::ShockCord(p) => turned(p.packing.radial_offset_m, p.packing.angle_rad),
            _ => [0.0, 0.0],
        }
    }

    /// An internal part's outer radius about its own axis, m: an inner tube's or ring's outer
    /// radius, or a packed part's packed radius.
    pub fn outer_radius_about_axis_m(&self) -> Option<f64> {
        match self {
            Self::InnerTube(p) => Some(p.outer_radius_m),
            Self::CenteringRing(p) => Some(p.outer_radius_m),
            Self::MassComponent(p) => Some(p.packing.radius_m),
            Self::Parachute(p) => Some(p.packing.radius_m),
            Self::Streamer(p) => Some(p.packing.radius_m),
            Self::ShockCord(p) => Some(p.packing.radius_m),
            _ => None,
        }
    }

    /// How far an internal part reaches from `axis` (`[x, y]` in body axes, m): the distance
    /// between the two axes plus the part's outer radius.
    pub fn reach_from_m(&self, axis: [f64; 2]) -> Option<f64> {
        let [x, y] = self.axis_offset_m();
        self.outer_radius_about_axis_m()
            .map(|r| (x - axis[0]).hypot(y - axis[1]) + r)
    }

    /// Mass properties in the part's own frame. External attachments need the radius of the body
    /// they sit on.
    ///
    /// # Errors
    ///
    /// The part's own geometry, material and numerical errors, and [`DesignError::Geometry`] when
    /// an external attachment has no body radius.
    pub fn mass_properties(
        &self,
        body_radius_m: Option<f64>,
    ) -> Result<MassProperties, DesignError> {
        let body = || {
            body_radius_m.ok_or_else(|| {
                DesignError::Geometry(format!(
                    "a {} needs the radius of the body it sits on",
                    self.kind_name()
                ))
            })
        };
        match self {
            Self::NoseCone(p) => p.mass_properties(),
            Self::BodyTube(p) => p.mass_properties(),
            Self::Transition(p) => p.mass_properties(),
            Self::InnerTube(p) => p.mass_properties(),
            Self::CenteringRing(p) => p.mass_properties(),
            Self::FinSet(p) => p.mass_properties(body()?),
            Self::TubeFinSet(p) => p.mass_properties(body()?),
            Self::LaunchLug(p) => p.mass_properties(body()?),
            Self::RailButton(p) => p.mass_properties(body()?),
            Self::MassComponent(p) => p.mass_properties(),
            Self::Parachute(p) => p.mass_properties(),
            Self::Streamer(p) => p.mass_properties(),
            Self::ShockCord(p) => p.mass_properties(),
        }
    }
}

/// Where an attached part sits along its parent. Offsets are positive aft.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "from", rename_all = "snake_case", deny_unknown_fields)]
pub enum Position {
    /// The part's forward end is `aft_offset_m` aft of the parent's forward end.
    Top {
        /// Offset, m.
        #[serde(default)]
        aft_offset_m: f64,
    },
    /// The part's middle is `aft_offset_m` aft of the parent's middle.
    Middle {
        /// Offset, m.
        #[serde(default)]
        aft_offset_m: f64,
    },
    /// The part's aft end is `aft_offset_m` aft of the parent's aft end.
    Bottom {
        /// Offset, m.
        #[serde(default)]
        aft_offset_m: f64,
    },
    /// The part's forward end is `aft_offset_m` aft of the previous sibling's aft end, or of the
    /// parent's forward end for the first child.
    After {
        /// Offset, m.
        #[serde(default)]
        aft_offset_m: f64,
    },
    /// The part's forward end is at station `station_m`, measured aft of the nose tip.
    Absolute {
        /// Station, m.
        station_m: f64,
    },
}

/// A dimension resolved from the tree instead of stored in the part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AutoDimension {
    /// A nose cone's base radius: the next body component's forward radius.
    BaseRadius,
    /// A body tube's outer radius: the previous body component's aft radius, or, when that can't
    /// be resolved, the next one's forward radius. A centering ring's or inner tube's outer
    /// radius: its parent's inner radius, which is how a coupler or an engine block fills the tube
    /// it sits in.
    OuterRadius,
    /// A transition's forward radius: the previous body component's aft radius.
    ForeRadius,
    /// A transition's aft radius: the next body component's forward radius.
    AftRadius,
    /// A nose cone's shoulder outer radius: the inner radius of the body tube behind it.
    ShoulderRadius,
    /// A transition's forward shoulder outer radius: the inner radius of the body tube ahead of it.
    ForeShoulderRadius,
    /// A transition's aft shoulder outer radius: the inner radius of the body tube behind it.
    AftShoulderRadius,
    /// A centering ring's inner radius: the outer radius of the widest on-axis inner tube among
    /// its siblings that overlaps it along the axis, or zero when none does.
    InnerRadius,
    /// A mass component's or recovery part's packed radius: its parent's inner radius less the
    /// part's radial offset.
    PackedRadius,
}

impl AutoDimension {
    /// The dimension's name, as serialized.
    pub fn name(self) -> &'static str {
        match self {
            Self::BaseRadius => "base_radius",
            Self::OuterRadius => "outer_radius",
            Self::ForeRadius => "fore_radius",
            Self::AftRadius => "aft_radius",
            Self::ShoulderRadius => "shoulder_radius",
            Self::ForeShoulderRadius => "fore_shoulder_radius",
            Self::AftShoulderRadius => "aft_shoulder_radius",
            Self::InnerRadius => "inner_radius",
            Self::PackedRadius => "packed_radius",
        }
    }

    fn applies_to(self, part: &Part) -> bool {
        use AutoDimension as A;
        matches!(
            (self, part),
            (A::BaseRadius | A::ShoulderRadius, Part::NoseCone(_))
                | (
                    A::OuterRadius,
                    Part::BodyTube(_) | Part::CenteringRing(_) | Part::InnerTube(_)
                )
                | (
                    A::ForeRadius | A::AftRadius | A::ForeShoulderRadius | A::AftShoulderRadius,
                    Part::Transition(_)
                )
                | (A::InnerRadius, Part::CenteringRing(_))
                | (
                    A::PackedRadius,
                    Part::MassComponent(_)
                        | Part::Parachute(_)
                        | Part::Streamer(_)
                        | Part::ShockCord(_)
                )
        )
    }
}

/// Values that replace the mass properties computed from geometry. Each applies in turn:
///
/// 1. **Mass** `m′`: the body is rescaled, `I′ = I m′/m`, keeping its centre and shape. A body with
///    no mass becomes a point mass at its centre.
/// 2. **Centre of mass**: the centre moves along the axis to `cg_aft_m` aft of the component's
///    forward end (a stage's, for a stage), with or without its children, and off the axis to
///    `cg_xy_m` when given (otherwise it keeps its offset); the tensor about the centre is
///    unchanged.
/// 3. **Inertia**: the tensor about the (new) centre is replaced.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    /// Mass, kg.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mass_kg: Option<f64>,
    /// Centre of mass, m aft of the forward end of the component (or the stage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cg_aft_m: Option<f64>,
    /// Centre of mass off the axis, `[x, y]` in body axes, m.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cg_xy_m: Option<[f64; 2]>,
    /// Inertia tensor about the centre of mass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inertia: Option<InertiaOverride>,
}

/// An inertia tensor about the centre of mass in body axes, kg·m². The off-diagonal entries are
/// the tensor's, `I_xy = −∫ x y dm` ([`crate::mass`]); they default to zero.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InertiaOverride {
    /// `I_xx`, kg·m².
    pub xx_kg_m2: f64,
    /// `I_yy`, kg·m².
    pub yy_kg_m2: f64,
    /// `I_zz`, about the rocket's axis, kg·m².
    pub zz_kg_m2: f64,
    /// `I_xy`, kg·m².
    #[serde(default)]
    pub xy_kg_m2: f64,
    /// `I_xz`, kg·m².
    #[serde(default)]
    pub xz_kg_m2: f64,
    /// `I_yz`, kg·m².
    #[serde(default)]
    pub yz_kg_m2: f64,
}

impl InertiaOverride {
    /// A tensor symmetric about the body axis: `diag(I_t, I_t, I_a)`.
    pub fn axisymmetric(axial_kg_m2: f64, transverse_kg_m2: f64) -> Self {
        Self {
            xx_kg_m2: transverse_kg_m2,
            yy_kg_m2: transverse_kg_m2,
            zz_kg_m2: axial_kg_m2,
            xy_kg_m2: 0.0,
            xz_kg_m2: 0.0,
            yz_kg_m2: 0.0,
        }
    }

    /// The tensor.
    pub fn tensor(&self) -> DMat3 {
        DMat3::from_cols(
            DVec3::new(self.xx_kg_m2, self.xy_kg_m2, self.xz_kg_m2),
            DVec3::new(self.xy_kg_m2, self.yy_kg_m2, self.yz_kg_m2),
            DVec3::new(self.xz_kg_m2, self.yz_kg_m2, self.zz_kg_m2),
        )
    }
}

impl Overrides {
    /// Whether the centre of mass is overridden, along the axis or off it.
    pub fn sets_centre(&self) -> bool {
        self.cg_aft_m.is_some() || self.cg_xy_m.is_some()
    }

    /// Whether the centre of mass is moved along the axis (`cg_aft_m`).
    pub fn sets_axial_centre(&self) -> bool {
        self.cg_aft_m.is_some()
    }

    /// Whether nothing is overridden.
    pub fn is_empty(&self) -> bool {
        self.mass_kg.is_none()
            && self.cg_aft_m.is_none()
            && self.cg_xy_m.is_none()
            && self.inertia.is_none()
    }

    /// Applies the overrides to `mass`, measuring a centre-of-mass override from station
    /// `fore_station_m`.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a negative or non-finite mass or a non-finite centre, and
    /// [`DesignError::UnphysicalInertia`] when the result is not a real body
    /// ([`MassProperties::validate`]).
    pub fn apply(
        &self,
        mass: MassProperties,
        fore_station_m: f64,
    ) -> Result<MassProperties, DesignError> {
        if self.is_empty() {
            return Ok(mass);
        }
        let mut out = mass;
        if let Some(m) = self.mass_kg {
            check_dimension("mass override (kg)", m, true)?;
            out = if out.mass_kg > 0.0 {
                out.scaled(m / out.mass_kg)
            } else {
                MassProperties::point(m, out.cg_m)
            };
        }
        if let Some(aft) = self.cg_aft_m {
            if !aft.is_finite() {
                return Err(DesignError::Domain {
                    what: "centre-of-mass override (m)",
                    value: aft,
                });
            }
            out.cg_m.z = -(fore_station_m + aft);
        }
        if let Some([x, y]) = self.cg_xy_m {
            for value in [x, y] {
                if !value.is_finite() {
                    return Err(DesignError::Domain {
                        what: "off-axis centre-of-mass override (m)",
                        value,
                    });
                }
            }
            out.cg_m.x = x;
            out.cg_m.y = y;
        }
        if let Some(inertia) = self.inertia {
            out.inertia_kg_m2 = inertia.tensor();
        }
        out.validate()?;
        Ok(out)
    }
}

/// How the reference diameter (for aerodynamic coefficients) is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum ReferenceDiameter {
    /// The widest body component (nose cone, body tube or transition) in any stage. Internal
    /// parts, shoulders, fins, tube fins, lugs and rail buttons don't count.
    Maximum {},
    /// The base of the first nose cone.
    NoseBase {},
    /// A given diameter.
    Custom {
        /// Diameter, m.
        diameter_m: f64,
    },
}

impl Default for ReferenceDiameter {
    fn default() -> Self {
        Self::Maximum {}
    }
}

/// A component resolved into place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedComponent {
    /// The component's id.
    pub id: String,
    /// Index of its stage in [`Layout::stages`].
    pub stage: usize,
    /// Index of its parent in [`Layout::components`]; `None` for a body component.
    pub parent: Option<usize>,
    /// The part with every automatic dimension filled in.
    pub part: Part,
    /// Station of its forward end (the start of its axial extent), m aft of the nose tip.
    pub fore_station_m: f64,
    /// Axial extent ([`Part::length_m`]), m.
    pub length_m: f64,
    /// The outer surface's finish ([`Component::finish`], with the default filled in).
    #[serde(default)]
    pub finish: Finish,
    /// For an external attachment, the radius of the body tube it sits on, m.
    pub body_radius_m: Option<f64>,
    /// Its motor mount, if it is one.
    pub motor_mount: Option<MotorMount>,
    /// Its own mass properties in body axes, after overrides that cover it alone.
    pub own: MassProperties,
    /// It with everything attached to it, after every override that applies within.
    pub with_children: MassProperties,
    /// Whether its own overrides move its centre of mass along the axis (`cg_aft_m`).
    #[serde(default)]
    pub centre_overridden: bool,
}

impl PlacedComponent {
    /// Station of the aft end of its axial extent, m.
    pub fn aft_station_m(&self) -> f64 {
        self.fore_station_m + self.length_m
    }
}

/// A stage resolved into place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacedStage {
    /// The stage's id.
    pub id: String,
    /// Station of its forward end, m.
    pub fore_station_m: f64,
    /// Station of its aft end, m.
    pub aft_station_m: f64,
    /// Its mass properties in body axes, without motors, after its overrides.
    pub mass: MassProperties,
    /// Whether the stage's own overrides move its centre of mass along the axis (`cg_aft_m`).
    #[serde(default)]
    pub centre_overridden: bool,
}

/// A design resolved into placed parts, with its structural mass properties (no motors).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    /// Every component, depth first: each body component followed by its attached parts.
    pub components: Vec<PlacedComponent>,
    /// The stages, forward to aft.
    pub stages: Vec<PlacedStage>,
    /// Length from the nose tip to the aft end of the last body component, m.
    pub length_m: f64,
    /// Reference diameter, m.
    pub reference_diameter_m: f64,
    /// Every stage together, without motors.
    pub structure: MassProperties,
}

impl Layout {
    /// The component with id `id`, and its index.
    pub fn find(&self, id: &str) -> Option<(usize, &PlacedComponent)> {
        self.components.iter().enumerate().find(|(_, c)| c.id == id)
    }

    /// The body components, forward to aft.
    pub fn body(&self) -> impl Iterator<Item = &PlacedComponent> {
        self.components.iter().filter(|c| c.parent.is_none())
    }

    /// Reference area `π d²/4`, m².
    pub fn reference_area_m2(&self) -> f64 {
        0.25 * std::f64::consts::PI * self.reference_diameter_m * self.reference_diameter_m
    }
}

/// An automatic body radius that has nothing to take: where it is, and which radius
/// ([`Rocket::unresolvable_body_radii`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnresolvableRadius {
    /// Index of the component's stage in [`Rocket::stages`].
    pub stage: usize,
    /// Index of the component in that stage's [`Stage::components`].
    pub component: usize,
    /// The component's id.
    pub id: String,
    /// Which of its radii: [`AutoDimension::BaseRadius`], [`AutoDimension::OuterRadius`],
    /// [`AutoDimension::ForeRadius`] or [`AutoDimension::AftRadius`].
    pub dimension: AutoDimension,
}

impl Rocket {
    /// Resolves the tree: checks its structure, fills in automatic dimensions, places every part,
    /// and computes the mass properties with overrides.
    ///
    /// # Errors
    ///
    /// - [`DesignError::DuplicateId`] for an empty or repeated stage or component id.
    /// - [`DesignError::Tree`] for a rocket with no stages, an empty stage, a part in the wrong
    ///   place (an attached part in a stage's list, a body component attached, fins on anything but
    ///   a body tube, children under anything but a body component or inner tube), a missing or
    ///   unexpected position, an automatic dimension that doesn't apply or can't be resolved, or a
    ///   motor mount on anything but a body tube or inner tube.
    /// - Any part's geometry, material or numerical error, a custom finish's negative or
    ///   non-finite roughness, and override errors.
    pub fn layout(&self) -> Result<Layout, DesignError> {
        if self.stages.is_empty() {
            return Err(DesignError::Tree {
                id: self.name.clone(),
                message: "a rocket needs at least one stage".to_owned(),
            });
        }
        let mut ids = BTreeSet::new();
        for stage in &self.stages {
            unique(&mut ids, &stage.id)?;
            if stage.components.is_empty() {
                return Err(tree(&stage.id, "a stage needs at least one body component"));
            }
            for component in &stage.components {
                check_node(&mut ids, component, None, 0)?;
            }
        }

        let nodes: Vec<(usize, &Component)> = self
            .stages
            .iter()
            .enumerate()
            .flat_map(|(k, stage)| stage.components.iter().map(move |c| (k, c)))
            .collect();
        let mut parts: Vec<Part> = nodes.iter().map(|(_, c)| c.part.clone()).collect();
        let autos: Vec<&[AutoDimension]> = nodes.iter().map(|(_, c)| c.auto.as_slice()).collect();
        let ids: Vec<&str> = nodes.iter().map(|(_, c)| c.id.as_str()).collect();
        resolve_body_radii(&mut parts, &autos, &ids)?;
        resolve_shoulders(&mut parts, &autos, &ids)?;

        let mut components = Vec::new();
        let mut stage_masses: Vec<Vec<MassProperties>> = vec![Vec::new(); self.stages.len()];
        let mut stage_ends: Vec<Option<(f64, f64)>> = vec![None; self.stages.len()];
        let mut station = 0.0;
        for ((stage, node), part) in nodes.iter().zip(parts) {
            let length = part.length_m();
            check_dimension("body component length", length, false)
                .map_err(|e| within(&node.id, e))?;
            let placed = place(&part, None, station, &node.id)?;
            let index = components.len();
            components.push(PlacedComponent {
                id: node.id.clone(),
                stage: *stage,
                parent: None,
                part,
                fore_station_m: station,
                length_m: length,
                finish: node.finish.unwrap_or_default(),
                body_radius_m: None,
                motor_mount: node.motor_mount,
                own: placed,
                with_children: placed,
                centre_overridden: node.overrides.sets_axial_centre(),
            });
            let with_children = finish(&mut components, index, node)?;
            stage_masses[*stage].push(with_children);
            let ends = stage_ends[*stage].get_or_insert((station, station));
            ends.1 = station + length;
            station += length;
        }

        let mut stages = Vec::with_capacity(self.stages.len());
        for ((stage, masses), ends) in self.stages.iter().zip(&stage_masses).zip(&stage_ends) {
            let (fore, aft) = ends.unwrap_or((0.0, 0.0));
            let mass = stage
                .overrides
                .apply(MassProperties::combine(masses), fore)
                .map_err(|e| within(&stage.id, e))?;
            stages.push(PlacedStage {
                id: stage.id.clone(),
                fore_station_m: fore,
                aft_station_m: aft,
                mass,
                centre_overridden: stage.overrides.sets_axial_centre(),
            });
        }
        let structure = MassProperties::combine(stages.iter().map(|s| &s.mass));
        let reference_diameter_m = self.reference_diameter_m(&components)?;
        Ok(Layout {
            components,
            stages,
            length_m: station,
            reference_diameter_m,
            structure,
        })
    }

    /// The automatic body radii that [`layout`](Self::layout) cannot resolve, forward to aft.
    ///
    /// These are the radii on a chain of automatic radii with no fixed radius anywhere along it:
    /// a nose cone whose base looks back at a tube that looks forward at it, or a stage of tubes
    /// that all say "automatic". Neighbours are followed by the rule each [`AutoDimension`]
    /// documents. `layout` refuses a design for which this list is not empty; what such a radius
    /// should be is not in the design, so an importer that knows its source program's convention
    /// fills them in first ([`fill_unresolvable_body_radii`](Self::fill_unresolvable_body_radii)).
    ///
    /// Only the body components' own radii are listed. An automatic shoulder with no body tube
    /// beside it, or an automatic dimension a part does not have, is refused by `layout` with its
    /// own error instead.
    pub fn unresolvable_body_radii(&self) -> Vec<UnresolvableRadius> {
        let nodes: Vec<(usize, usize, &Component)> = self
            .stages
            .iter()
            .enumerate()
            .flat_map(|(k, stage)| {
                stage
                    .components
                    .iter()
                    .enumerate()
                    .map(move |(i, c)| (k, i, c))
            })
            .collect();
        let parts: Vec<&Part> = nodes.iter().map(|(_, _, c)| &c.part).collect();
        let autos: Vec<&[AutoDimension]> =
            nodes.iter().map(|(_, _, c)| c.auto.as_slice()).collect();
        let (fore, aft) = sweep_body_radii(&parts, &autos);
        let mut unresolvable = Vec::new();
        for (n, &(stage, component, node)) in nodes.iter().enumerate() {
            let mut add = |dimension| {
                unresolvable.push(UnresolvableRadius {
                    stage,
                    component,
                    id: node.id.clone(),
                    dimension,
                });
            };
            match node.part {
                Part::NoseCone(_) if aft[n].is_none() => add(AutoDimension::BaseRadius),
                Part::BodyTube(_) if aft[n].is_none() => add(AutoDimension::OuterRadius),
                Part::Transition(_) => {
                    if fore[n].is_none() {
                        add(AutoDimension::ForeRadius);
                    }
                    if aft[n].is_none() {
                        add(AutoDimension::AftRadius);
                    }
                }
                _ => {}
            }
        }
        unresolvable
    }

    /// Gives every radius [`unresolvable_body_radii`](Self::unresolvable_body_radii) lists the
    /// fixed radius `radius_m`, drops its automatic mark, and returns what it filled.
    ///
    /// Every radius on such a chain is listed, so the whole chain takes the one radius. Radii the
    /// neighbour rule could already reach are left as they are and resolve as before. The design
    /// no longer records that the filled radii were automatic, which is the point: it now says
    /// what they are.
    pub fn fill_unresolvable_body_radii(&mut self, radius_m: f64) -> Vec<UnresolvableRadius> {
        let unresolvable = self.unresolvable_body_radii();
        for radius in &unresolvable {
            let component = &mut self.stages[radius.stage].components[radius.component];
            match (&mut component.part, radius.dimension) {
                (Part::NoseCone(p), AutoDimension::BaseRadius) => p.base_radius_m = radius_m,
                (Part::BodyTube(p), AutoDimension::OuterRadius) => p.outer_radius_m = radius_m,
                (Part::Transition(p), AutoDimension::ForeRadius) => p.fore_radius_m = radius_m,
                (Part::Transition(p), AutoDimension::AftRadius) => p.aft_radius_m = radius_m,
                // `unresolvable_body_radii` lists only the four pairings above.
                _ => continue,
            }
            component.auto.retain(|auto| *auto != radius.dimension);
        }
        unresolvable
    }

    fn reference_diameter_m(&self, components: &[PlacedComponent]) -> Result<f64, DesignError> {
        let diameter = match self.reference_diameter {
            ReferenceDiameter::Maximum {} => {
                let mut widest = 0.0f64;
                for c in components.iter().filter(|c| c.parent.is_none()) {
                    if let Some(r) = c.part.max_radius_m()? {
                        widest = widest.max(r);
                    }
                }
                2.0 * widest
            }
            ReferenceDiameter::NoseBase {} => components
                .iter()
                .find_map(|c| match &c.part {
                    Part::NoseCone(nose) => Some(2.0 * nose.base_radius_m),
                    _ => None,
                })
                .ok_or_else(|| {
                    tree(
                        &self.name,
                        "the reference diameter is the nose base, but there is no nose cone",
                    )
                })?,
            ReferenceDiameter::Custom { diameter_m } => diameter_m,
        };
        check_dimension("reference diameter (m)", diameter, false)?;
        Ok(diameter)
    }
}

/// `error` in the stage or component `id`.
fn within(id: &str, error: DesignError) -> DesignError {
    DesignError::InComponent {
        id: id.to_owned(),
        source: Box::new(error),
    }
}

/// How many levels a body component and the parts nested in it may span, the body component being
/// the first: inner tubes in inner tubes far beyond any real rocket, and shallow enough that
/// resolving never exhausts a small (wasm) stack.
pub const MAX_DEPTH: usize = 32;

fn tree(id: &str, message: impl Into<String>) -> DesignError {
    DesignError::Tree {
        id: id.to_owned(),
        message: message.into(),
    }
}

fn unique(ids: &mut BTreeSet<String>, id: &str) -> Result<(), DesignError> {
    if id.is_empty() || !ids.insert(id.to_owned()) {
        return Err(DesignError::DuplicateId(id.to_owned()));
    }
    Ok(())
}

/// Checks a node's id, role, position, automatic dimensions and motor mount against where it sits,
/// and then its children. `parent` is `None` for a body component.
fn check_node(
    ids: &mut BTreeSet<String>,
    node: &Component,
    parent: Option<&Part>,
    depth: usize,
) -> Result<(), DesignError> {
    unique(ids, &node.id)?;
    if let Some(finish) = node.finish {
        finish.roughness_m().map_err(|e| within(&node.id, e))?;
    }
    if depth >= MAX_DEPTH {
        return Err(tree(
            &node.id,
            format!("components nest more than {MAX_DEPTH} deep"),
        ));
    }
    let kind = node.part.kind_name();
    match (parent, node.part.role()) {
        (None, Role::Body) => {
            if node.position.is_some() {
                return Err(tree(
                    &node.id,
                    "a body component stacks and takes no position",
                ));
            }
        }
        (None, _) => {
            return Err(tree(
                &node.id,
                format!("a {kind} can't be a body component; attach it to one"),
            ));
        }
        (Some(_), Role::Body) => {
            return Err(tree(
                &node.id,
                format!("a {kind} is a body component; list it in a stage"),
            ));
        }
        (Some(parent), Role::External) if !matches!(parent, Part::BodyTube(_)) => {
            return Err(tree(
                &node.id,
                format!(
                    "a {kind} attaches to a body tube, not a {}",
                    parent.kind_name()
                ),
            ));
        }
        (Some(parent), Role::Internal)
            if !(parent.is_body() || matches!(parent, Part::InnerTube(_))) =>
        {
            return Err(tree(
                &node.id,
                format!("a {kind} can't go inside a {}", parent.kind_name()),
            ));
        }
        (Some(_), _) => {
            if node.position.is_none() {
                return Err(tree(&node.id, "an attached part needs a position"));
            }
        }
    }
    for auto in &node.auto {
        if !auto.applies_to(&node.part) {
            return Err(tree(
                &node.id,
                format!("a {kind} has no automatic {} dimension", auto.name()),
            ));
        }
    }
    if node.motor_mount.is_some() && !matches!(node.part, Part::BodyTube(_) | Part::InnerTube(_)) {
        return Err(tree(&node.id, format!("a {kind} can't be a motor mount")));
    }
    if !node.children.is_empty()
        && !(node.part.is_body() || matches!(node.part, Part::InnerTube(_)))
    {
        return Err(tree(
            &node.id,
            format!("a {kind} can't have attached parts"),
        ));
    }
    for child in &node.children {
        check_node(ids, child, Some(&node.part), depth + 1)?;
    }
    Ok(())
}

/// Fills in the body components' automatic outer radii from their neighbours, through every stage,
/// by [`sweep_body_radii`]'s rule; a radius the sweep leaves unknown is an error.
fn resolve_body_radii(
    parts: &mut [Part],
    autos: &[&[AutoDimension]],
    ids: &[&str],
) -> Result<(), DesignError> {
    let (fore, aft) = sweep_body_radii(parts, autos);
    for (i, part) in parts.iter_mut().enumerate() {
        let missing = || {
            tree(
                ids[i],
                "an automatic radius has no fixed radius among its neighbours to take",
            )
        };
        match part {
            Part::NoseCone(p) => p.base_radius_m = aft[i].ok_or_else(missing)?,
            Part::BodyTube(p) => p.outer_radius_m = aft[i].ok_or_else(missing)?,
            Part::Transition(p) => {
                p.fore_radius_m = fore[i].ok_or_else(missing)?;
                p.aft_radius_m = aft[i].ok_or_else(missing)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Each body component's forward and aft radius, following automatic radii to their sources;
/// `None` where a radius is automatic and nothing reaches it. `parts` are the body components,
/// forward to aft through every stage.
///
/// Each automatic radius has a source: a nose cone's base and a transition's aft radius take the
/// next component's forward radius; a body tube and a transition's forward radius take the previous
/// component's aft radius. Sources are followed until nothing changes. Then a body tube still
/// unresolved takes the next component's forward radius instead (the first one that can), and the
/// sweep repeats.
fn sweep_body_radii<P: std::borrow::Borrow<Part>>(
    parts: &[P],
    autos: &[&[AutoDimension]],
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = parts.len();
    let is_auto = |i: usize, a: AutoDimension| autos[i].contains(&a);
    let mut fore: Vec<Option<f64>> = Vec::with_capacity(n);
    let mut aft: Vec<Option<f64>> = Vec::with_capacity(n);
    for (i, part) in parts.iter().enumerate() {
        let (f, a) = match part.borrow() {
            Part::NoseCone(p) => (
                Some(0.0),
                (!is_auto(i, AutoDimension::BaseRadius)).then_some(p.base_radius_m),
            ),
            Part::BodyTube(p) => {
                let r = (!is_auto(i, AutoDimension::OuterRadius)).then_some(p.outer_radius_m);
                (r, r)
            }
            Part::Transition(p) => (
                (!is_auto(i, AutoDimension::ForeRadius)).then_some(p.fore_radius_m),
                (!is_auto(i, AutoDimension::AftRadius)).then_some(p.aft_radius_m),
            ),
            _ => (None, None),
        };
        fore.push(f);
        aft.push(a);
    }
    loop {
        let mut progress = false;
        for i in 0..n {
            let previous_aft = if i > 0 { aft[i - 1] } else { None };
            let next_fore = fore.get(i + 1).copied().flatten();
            match parts[i].borrow() {
                Part::NoseCone(_) => {
                    if aft[i].is_none()
                        && let Some(r) = next_fore
                    {
                        aft[i] = Some(r);
                        progress = true;
                    }
                }
                Part::BodyTube(_) => {
                    if fore[i].is_none()
                        && let Some(r) = previous_aft
                    {
                        fore[i] = Some(r);
                        aft[i] = Some(r);
                        progress = true;
                    }
                }
                Part::Transition(_) => {
                    if fore[i].is_none()
                        && let Some(r) = previous_aft
                    {
                        fore[i] = Some(r);
                        progress = true;
                    }
                    if aft[i].is_none()
                        && let Some(r) = next_fore
                    {
                        aft[i] = Some(r);
                        progress = true;
                    }
                }
                _ => {}
            }
        }
        if progress {
            continue;
        }
        let fallback = (0..n).find(|&i| {
            matches!(parts[i].borrow(), Part::BodyTube(_))
                && fore[i].is_none()
                && fore.get(i + 1).copied().flatten().is_some()
        });
        match fallback {
            Some(i) => {
                fore[i] = fore[i + 1];
                aft[i] = fore[i + 1];
            }
            None => break,
        }
    }
    (fore, aft)
}

/// Fills in automatic shoulder radii from the adjoining body tubes' inner radii.
fn resolve_shoulders(
    parts: &mut [Part],
    autos: &[&[AutoDimension]],
    ids: &[&str],
) -> Result<(), DesignError> {
    let inner = |parts: &[Part], i: Option<usize>| -> Option<f64> {
        match i.and_then(|i| parts.get(i)) {
            Some(Part::BodyTube(t)) => Some(t.outer_radius_m - t.thickness_m),
            _ => None,
        }
    };
    for i in 0..parts.len() {
        let previous = inner(parts, i.checked_sub(1));
        let next = inner(parts, Some(i + 1));
        let fit = |shoulder: &mut Option<crate::parts::Shoulder>,
                   radius: Option<f64>,
                   which: &str|
         -> Result<(), DesignError> {
            let Some(shoulder) = shoulder else {
                return Err(tree(
                    ids[i],
                    format!("an automatic {which} shoulder radius needs a shoulder"),
                ));
            };
            shoulder.outer_radius_m = radius.ok_or_else(|| {
                tree(
                    ids[i],
                    format!("an automatic {which} shoulder radius needs a body tube there"),
                )
            })?;
            Ok(())
        };
        match &mut parts[i] {
            Part::NoseCone(p) if autos[i].contains(&AutoDimension::ShoulderRadius) => {
                fit(&mut p.shoulder, next, "aft")?;
            }
            Part::Transition(p) => {
                if autos[i].contains(&AutoDimension::ForeShoulderRadius) {
                    fit(&mut p.fore_shoulder, previous, "forward")?;
                }
                if autos[i].contains(&AutoDimension::AftShoulderRadius) {
                    fit(&mut p.aft_shoulder, next, "aft")?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// A part's mass properties placed with its forward end at `fore_station_m`.
fn place(
    part: &Part,
    body_radius_m: Option<f64>,
    fore_station_m: f64,
    id: &str,
) -> Result<MassProperties, DesignError> {
    let mass = part
        .mass_properties(body_radius_m)
        .map_err(|e| within(id, e))?;
    Ok(mass.translated(DVec3::new(0.0, 0.0, -fore_station_m)))
}

/// Places a component's children, applies its overrides, and returns it with its children.
fn finish(
    components: &mut Vec<PlacedComponent>,
    index: usize,
    node: &Component,
) -> Result<MassProperties, DesignError> {
    let parent = &components[index];
    let (p_fore, p_length, stage) = (parent.fore_station_m, parent.length_m, parent.stage);
    let (p_kind, p_inner) = (parent.part.kind_name(), parent.part.inner_radius_m());
    let p_axis = parent.part.axis_offset_m();
    let p_tube_radius = match &parent.part {
        Part::BodyTube(tube) => Some(tube.outer_radius_m),
        _ => None,
    };
    let own = if node.overrides_include_children {
        parent.own
    } else {
        node.overrides
            .apply(parent.own, p_fore)
            .map_err(|e| within(&node.id, e))?
    };

    // Positions first: they don't depend on any automatic radius, and a ring's inner radius needs
    // its siblings' places.
    let mut stations = Vec::with_capacity(node.children.len());
    let mut previous_aft = None;
    for child in &node.children {
        let length = child.part.length_m();
        check_dimension("attached part length", length, true).map_err(|e| within(&child.id, e))?;
        let fore = match child.position {
            Some(Position::Top { aft_offset_m }) => p_fore + aft_offset_m,
            Some(Position::Middle { aft_offset_m }) => {
                p_fore + 0.5 * (p_length - length) + aft_offset_m
            }
            Some(Position::Bottom { aft_offset_m }) => p_fore + p_length - length + aft_offset_m,
            Some(Position::After { aft_offset_m }) => previous_aft.unwrap_or(p_fore) + aft_offset_m,
            Some(Position::Absolute { station_m }) => station_m,
            None => return Err(tree(&child.id, "an attached part needs a position")),
        };
        if !fore.is_finite() {
            return Err(within(
                &child.id,
                DesignError::Domain {
                    what: "attached part position (m)",
                    value: fore,
                },
            ));
        }
        stations.push((fore, length));
        previous_aft = Some(fore + length);
    }

    let bore = |id: &str| {
        p_inner.ok_or_else(|| {
            tree(
                id,
                format!("an automatic radius needs a tube's inner radius, and a {p_kind} has none"),
            )
        })
    };
    // Automatic outer radii come first, in a pass of their own. They need nothing but the parent's
    // bore, while a ring's automatic *inner* radius reads its siblings' outer radii — so resolving
    // both in one pass would give a ring whose bore depended on whether the tube inside it was
    // written before or after it. That is [Loft lesson L60][lessons], and the reason this is two
    // passes rather than one.
    //
    // [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
    let mut resolved: Vec<Part> = node.children.iter().map(|c| c.part.clone()).collect();
    for (part, child) in resolved.iter_mut().zip(&node.children) {
        if child.auto.contains(&AutoDimension::OuterRadius) {
            match part {
                Part::CenteringRing(ring) => ring.outer_radius_m = bore(&child.id)?,
                Part::InnerTube(tube) => tube.outer_radius_m = bore(&child.id)?,
                _ => {}
            }
        }
    }

    let mut parts = vec![own];
    for (k, child) in node.children.iter().enumerate() {
        let (fore, length) = stations[k];
        let mut part = resolved[k].clone();
        let parent_inner = || bore(&child.id);
        // A packed part fills the parent's bore on its side of the parent's axis.
        let packed = |packing: &mut crate::parts::Packing| -> Result<(), DesignError> {
            let (x, y) = (
                packing.radial_offset_m * packing.angle_rad.cos(),
                packing.radial_offset_m * packing.angle_rad.sin(),
            );
            let room = parent_inner()? - (x - p_axis[0]).hypot(y - p_axis[1]);
            if !(room.is_finite() && room > 0.0) {
                return Err(tree(
                    &child.id,
                    "an automatic packed radius needs a radial offset inside the parent's bore",
                ));
            }
            packing.radius_m = room;
            Ok(())
        };
        for auto in &child.auto {
            match (auto, &mut part) {
                // Already done in the pass above.
                (AutoDimension::OuterRadius, _) => {}
                (AutoDimension::InnerRadius, Part::CenteringRing(ring)) => {
                    let aft = fore + length;
                    // A ring centres something *narrower than itself*. A sibling as wide as the
                    // ring is not what the ring holds — it is whatever the ring is bolted to — and
                    // taking its radius would leave the ring no material at all, which is how a
                    // full-bore coupler brushing a ring by a tenth of a millimetre made the ring
                    // weigh nothing. The ring's own outer radius is already resolved above.
                    let outer_radius_m = ring.outer_radius_m;
                    ring.inner_radius_m = resolved
                        .iter()
                        .zip(&stations)
                        .filter_map(|(sibling, &(s_fore, s_length))| match sibling {
                            Part::InnerTube(tube)
                                if tube.radial_offset_m == 0.0
                                    && tube.outer_radius_m < outer_radius_m
                                    && s_fore.max(fore) < (s_fore + s_length).min(aft) =>
                            {
                                Some(tube.outer_radius_m)
                            }
                            _ => None,
                        })
                        .fold(0.0, f64::max);
                }
                (AutoDimension::PackedRadius, Part::MassComponent(p)) => packed(&mut p.packing)?,
                (AutoDimension::PackedRadius, Part::Parachute(p)) => packed(&mut p.packing)?,
                (AutoDimension::PackedRadius, Part::Streamer(p)) => packed(&mut p.packing)?,
                (AutoDimension::PackedRadius, Part::ShockCord(p)) => packed(&mut p.packing)?,
                _ => {}
            }
        }
        let body_radius_m = if part.is_external() {
            Some(
                p_tube_radius
                    .ok_or_else(|| tree(&child.id, "external parts attach to a body tube"))?,
            )
        } else {
            None
        };
        let placed = place(&part, body_radius_m, fore, &child.id)?;
        let child_index = components.len();
        components.push(PlacedComponent {
            id: child.id.clone(),
            stage,
            parent: Some(index),
            part,
            fore_station_m: fore,
            length_m: length,
            finish: child.finish.unwrap_or_default(),
            body_radius_m,
            motor_mount: child.motor_mount,
            own: placed,
            with_children: placed,
            centre_overridden: child.overrides.sets_axial_centre(),
        });
        parts.push(finish(components, child_index, child)?);
    }

    let mut with_children = MassProperties::combine(&parts);
    if node.overrides_include_children {
        with_children = node
            .overrides
            .apply(with_children, p_fore)
            .map_err(|e| within(&node.id, e))?;
    }
    components[index].own = own;
    components[index].with_children = with_children;
    Ok(with_children)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::parts::Shoulder;
    use crate::testing::{
        attached, body, bottom, fins, inner_tube, mass_component, nose, ring, rocket, stage,
        three_fin_rocket, top, tube,
    };

    fn close(got: f64, want: f64, tol: f64, what: &str) {
        assert!((got - want).abs() <= tol, "{what}: {got} vs {want}");
    }

    fn station(layout: &Layout, id: &str) -> f64 {
        layout.find(id).unwrap().1.fore_station_m
    }

    /// Body components stack through both stages; each position rule puts a child where its doc
    /// says, worked by hand from the parent tube at stations 0.2 to 1.0.
    #[test]
    fn stacks_body_components_and_places_attached_parts() {
        let mut airframe = body("airframe", tube(0.8, 0.03, 0.001));
        let mut coupler = attached("coupler", inner_tube(0.2, 0.029, 0.001), top(0.1));
        coupler.children = vec![attached(
            "bay",
            mass_component(0.1, 0.05, 0.02),
            Position::After { aft_offset_m: 0.02 },
        )];
        airframe.children = vec![
            coupler,
            attached(
                "middle",
                mass_component(0.1, 0.1, 0.02),
                Position::Middle { aft_offset_m: 0.02 },
            ),
            attached("bottom", mass_component(0.1, 0.1, 0.02), bottom(-0.01)),
            attached(
                "after",
                mass_component(0.1, 0.004, 0.02),
                Position::After {
                    aft_offset_m: 0.001,
                },
            ),
            attached(
                "absolute",
                mass_component(0.1, 0.05, 0.02),
                Position::Absolute { station_m: 0.4 },
            ),
        ];
        let design = rocket(vec![
            stage("upper", vec![body("nose", nose(0.2, 0.03)), airframe]),
            stage(
                "booster",
                vec![
                    body(
                        "interstage",
                        Part::Transition(Transition {
                            shape: crate::NoseShape::Conical {},
                            clipped: false,
                            length_m: 0.1,
                            fore_radius_m: 0.03,
                            aft_radius_m: 0.04,
                            wall: crate::Wall::Shell { thickness_m: 0.002 },
                            fore_shoulder: None,
                            aft_shoulder: None,
                            material: crate::testing::cardboard(),
                        }),
                    ),
                    body("booster-tube", tube(0.5, 0.04, 0.001)),
                ],
            ),
        ]);
        let layout = design.layout().unwrap();
        let tol = 1e-15;
        close(station(&layout, "nose"), 0.0, tol, "nose");
        close(station(&layout, "airframe"), 0.2, tol, "airframe");
        close(station(&layout, "coupler"), 0.3, tol, "top");
        // The first child of the coupler goes after the coupler's forward end.
        close(station(&layout, "bay"), 0.32, tol, "after, first child");
        close(station(&layout, "middle"), 0.2 + 0.35 + 0.02, tol, "middle");
        close(station(&layout, "bottom"), 1.0 - 0.1 - 0.01, tol, "bottom");
        close(station(&layout, "after"), 0.99 + 0.001, tol, "after");
        close(station(&layout, "absolute"), 0.4, tol, "absolute");
        close(station(&layout, "interstage"), 1.0, tol, "second stage");
        close(station(&layout, "booster-tube"), 1.1, tol, "booster tube");
        close(layout.length_m, 1.6, tol, "length");
        let [upper, booster] = &layout.stages[..] else {
            panic!("two stages")
        };
        close(upper.fore_station_m, 0.0, tol, "upper fore");
        close(upper.aft_station_m, 1.0, tol, "upper aft");
        close(booster.fore_station_m, 1.0, tol, "booster fore");
        close(booster.aft_station_m, 1.6, tol, "booster aft");
        let (bay, placed) = layout.find("bay").unwrap();
        assert_eq!(
            layout.components[placed.parent.unwrap()].id,
            "coupler",
            "parent"
        );
        assert_eq!(layout.components[bay].stage, 0);
        assert_eq!(layout.find("booster-tube").unwrap().1.stage, 1);
        // A placed part's centre is its own-frame centre moved to its station.
        let own = mass_component(0.1, 0.05, 0.02)
            .mass_properties(None)
            .unwrap();
        close(placed.own.cg_m.z, own.cg_m.z - 0.32, 1e-15, "placed centre");
        // The structure is every stage, and each stage every component with its children.
        let sum: f64 = layout.body().map(|c| c.with_children.mass_kg).sum();
        close(layout.structure.mass_kg, sum, 1e-15, "structure mass");
    }

    /// Automatic radii: a nose takes the tube behind it, a transition's forward radius crosses a
    /// stage boundary, a tube takes the transition ahead of it, and a tube with nothing resolvable
    /// ahead takes the next fixed radius.
    #[test]
    fn automatic_radii_follow_neighbours_across_stages() {
        let auto = |mut c: Component, dims: &[AutoDimension]| {
            c.auto = dims.to_vec();
            c
        };
        let transition = Part::Transition(Transition {
            shape: crate::NoseShape::Conical {},
            clipped: false,
            length_m: 0.1,
            fore_radius_m: 0.0,
            aft_radius_m: 0.04,
            wall: crate::Wall::Filled {},
            fore_shoulder: Some(Shoulder {
                length_m: 0.05,
                outer_radius_m: 0.0,
                thickness_m: 0.002,
                capped: false,
            }),
            aft_shoulder: None,
            material: crate::testing::cardboard(),
        });
        let mut nose_cone = auto(body("nose", nose(0.2, 0.0)), &[AutoDimension::BaseRadius]);
        if let Part::NoseCone(n) = &mut nose_cone.part {
            n.shoulder = Some(Shoulder {
                length_m: 0.05,
                outer_radius_m: 0.0,
                thickness_m: 0.002,
                capped: true,
            });
        }
        nose_cone.auto.push(AutoDimension::ShoulderRadius);
        let design = rocket(vec![
            stage(
                "upper",
                vec![nose_cone, body("upper-tube", tube(0.5, 0.03, 0.001))],
            ),
            stage(
                "booster",
                vec![
                    auto(
                        body("interstage", transition),
                        &[AutoDimension::ForeRadius, AutoDimension::ForeShoulderRadius],
                    ),
                    auto(
                        body("booster-tube", tube(0.5, 0.0, 0.001)),
                        &[AutoDimension::OuterRadius],
                    ),
                ],
            ),
        ]);
        let layout = design.layout().unwrap();
        let part = |id: &str| layout.find(id).unwrap().1.part.clone();
        let Part::NoseCone(n) = part("nose") else {
            panic!()
        };
        assert_eq!(n.base_radius_m, 0.03);
        assert_eq!(n.shoulder.unwrap().outer_radius_m, 0.03 - 0.001);
        let Part::Transition(t) = part("interstage") else {
            panic!()
        };
        assert_eq!(t.fore_radius_m, 0.03);
        assert_eq!(t.fore_shoulder.unwrap().outer_radius_m, 0.03 - 0.001);
        let Part::BodyTube(b) = part("booster-tube") else {
            panic!()
        };
        assert_eq!(b.outer_radius_m, 0.04);

        // Nothing ahead of the first tube is fixed, so it takes the tube behind it.
        let design = rocket(vec![stage(
            "only",
            vec![
                auto(body("nose", nose(0.2, 0.0)), &[AutoDimension::BaseRadius]),
                auto(
                    body("a", tube(0.3, 0.0, 0.001)),
                    &[AutoDimension::OuterRadius],
                ),
                body("b", tube(0.3, 0.05, 0.001)),
            ],
        )]);
        let layout = design.layout().unwrap();
        assert_eq!(layout.find("a").unwrap().1.part.aft_radius_m(), Some(0.05));
        assert_eq!(
            layout.find("nose").unwrap().1.part.aft_radius_m(),
            Some(0.05)
        );
        // A fixed radius ahead wins over one behind.
        let design = rocket(vec![stage(
            "only",
            vec![
                body("nose", nose(0.2, 0.02)),
                auto(
                    body("a", tube(0.3, 0.0, 0.001)),
                    &[AutoDimension::OuterRadius],
                ),
                body("b", tube(0.3, 0.05, 0.001)),
            ],
        )]);
        let layout = design.layout().unwrap();
        assert_eq!(layout.find("a").unwrap().1.part.aft_radius_m(), Some(0.02));

        // Automatic all the way round has nothing to take.
        let design = rocket(vec![stage(
            "only",
            vec![
                auto(body("nose", nose(0.2, 0.0)), &[AutoDimension::BaseRadius]),
                auto(
                    body("a", tube(0.3, 0.0, 0.001)),
                    &[AutoDimension::OuterRadius],
                ),
            ],
        )]);
        assert!(matches!(
            design.layout(),
            Err(DesignError::Tree { ref id, .. }) if id == "nose"
        ));
    }

    /// A conical transition with a 1 mm wall.
    fn cone_transition(fore_radius_m: f64, aft_radius_m: f64) -> Part {
        Part::Transition(Transition {
            shape: crate::NoseShape::Conical {},
            clipped: false,
            length_m: 0.1,
            fore_radius_m,
            aft_radius_m,
            wall: crate::Wall::Shell { thickness_m: 0.001 },
            fore_shoulder: None,
            aft_shoulder: None,
            material: crate::testing::cardboard(),
        })
    }

    /// The chain in Loft's quirks fixture: a nose's base, a tube and a transition's forward end all
    /// automatic, with only the transition's aft end fixed. Each automatic radius there looks at
    /// another automatic one, so all three are listed, forward to aft, and filling them is all
    /// `layout` needs. The fixed tube behind is never listed, and a radius the neighbour rule can
    /// reach is never filled.
    #[test]
    fn unresolvable_body_radii_are_the_chain_with_nothing_fixed() {
        let auto = |mut c: Component, dims: &[AutoDimension]| {
            c.auto = dims.to_vec();
            c
        };
        let mut design = rocket(vec![stage(
            "only",
            vec![
                auto(body("nose", nose(0.3, 0.0)), &[AutoDimension::BaseRadius]),
                auto(
                    body("upper", tube(0.5, 0.0, 0.002)),
                    &[AutoDimension::OuterRadius],
                ),
                auto(
                    body("shoulder", cone_transition(0.0, 0.022)),
                    &[AutoDimension::ForeRadius],
                ),
                body("lower", tube(0.45, 0.022, 0.0018)),
            ],
        )]);
        let listed = |design: &Rocket| -> Vec<(usize, usize, String, AutoDimension)> {
            design
                .unresolvable_body_radii()
                .into_iter()
                .map(|r| (r.stage, r.component, r.id, r.dimension))
                .collect()
        };
        let chain = vec![
            (0, 0, "nose".to_owned(), AutoDimension::BaseRadius),
            (0, 1, "upper".to_owned(), AutoDimension::OuterRadius),
            (0, 2, "shoulder".to_owned(), AutoDimension::ForeRadius),
        ];
        assert_eq!(listed(&design), chain);
        assert!(design.layout().is_err());

        let filled = design.fill_unresolvable_body_radii(0.025);
        let filled: Vec<_> = filled
            .into_iter()
            .map(|r| (r.stage, r.component, r.id, r.dimension))
            .collect();
        assert_eq!(filled, chain);
        assert!(listed(&design).is_empty());
        assert!(
            design.stages[0]
                .components
                .iter()
                .all(|c| c.auto.is_empty())
        );
        let layout = design.layout().unwrap();
        let radius = |id: &str| layout.find(id).unwrap().1.part.aft_radius_m();
        assert_eq!(radius("nose"), Some(0.025));
        assert_eq!(radius("upper"), Some(0.025));
        let Part::Transition(t) = &layout.find("shoulder").unwrap().1.part else {
            panic!("a transition")
        };
        assert_eq!((t.fore_radius_m, t.aft_radius_m), (0.025, 0.022));

        // A design that resolves lists nothing, and filling it changes nothing.
        let mut design = three_fin_rocket();
        let before = design.clone();
        assert!(design.unresolvable_body_radii().is_empty());
        assert!(design.fill_unresolvable_body_radii(0.025).is_empty());
        assert_eq!(design, before);
        design.layout().unwrap();
    }

    /// Rings take the tube's bore and the mount tube's outside; the parachute packs to the bore; a
    /// ring beside no inner tube is a bulkhead.
    #[test]
    fn ring_and_packed_radii_come_from_parent_and_siblings() {
        let layout = three_fin_rocket().layout().unwrap();
        let part = |id: &str| layout.find(id).unwrap().1.part.clone();
        for id in ["ring-fore", "ring-aft"] {
            let Part::CenteringRing(r) = part(id) else {
                panic!()
            };
            assert_eq!(r.outer_radius_m, 0.027 - 0.0015, "{id}");
            assert_eq!(r.inner_radius_m, 0.020, "{id}");
        }
        let Part::Parachute(p) = part("chute") else {
            panic!()
        };
        assert_eq!(p.packing.radius_m, 0.027 - 0.0015);

        let mut design = three_fin_rocket();
        let airframe = &mut design.stages[0].components[1];
        // Move the fore ring forward of the mount tube, which spans stations 0.7 to 1.0.
        airframe.children[1].position = Some(top(0.1));
        let layout = design.layout().unwrap();
        let Part::CenteringRing(r) = layout.find("ring-fore").unwrap().1.part.clone() else {
            panic!()
        };
        assert_eq!(r.inner_radius_m, 0.0);
    }

    /// The tree's structure equals the parts placed by hand at their stations and combined.
    #[test]
    fn tree_structure_matches_parts_placed_by_hand() {
        let design = three_fin_rocket();
        let layout = design.layout().unwrap();
        let bore = 0.027 - 0.0015;
        let placed = |part: Part, body_radius: Option<f64>, s: f64| {
            part.mass_properties(body_radius)
                .unwrap()
                .translated(DVec3::new(0.0, 0.0, -s))
        };
        let mut chute = design.stages[0].components[1].children[4].part.clone();
        if let Part::Parachute(p) = &mut chute {
            p.packing.radius_m = bore;
        }
        let parts = [
            placed(nose(0.2, 0.027), None, 0.0),
            placed(tube(0.8, 0.027, 0.0015), None, 0.2),
            placed(inner_tube(0.3, 0.020, 0.001), None, 0.7),
            placed(ring(0.006, bore, 0.020), None, 1.0 - 0.006 - 0.25),
            placed(ring(0.006, bore, 0.020), None, 1.0 - 0.006 - 0.02),
            placed(fins(0.1, 0.06), Some(0.027), 0.9),
            placed(chute, None, 0.25),
            placed(
                design.stages[0].components[1].children[5].part.clone(),
                Some(0.027),
                0.2 + 0.5 * (0.8 - 0.05),
            ),
        ];
        let want = MassProperties::combine(&parts);
        let got = layout.structure;
        close(got.mass_kg, want.mass_kg, 1e-15 * want.mass_kg, "mass");
        assert!(
            (got.cg_m - want.cg_m).length() < 1e-14,
            "{:?} vs {:?}",
            got.cg_m,
            want.cg_m
        );
        let scale = want
            .inertia_kg_m2
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        let diff = (got.inertia_kg_m2 - want.inertia_kg_m2)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(diff < 1e-13 * scale, "{diff:e}");
        assert_eq!(layout.stages[0].mass, got);
    }

    fn tensor_close(a: DMat3, b: DMat3, rel: f64) {
        let scale = b.to_cols_array().iter().fold(0.0f64, |m, v| m.max(v.abs()));
        let diff = (a - b)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(diff <= rel * scale, "{a:?}\nvs\n{b:?}");
    }

    /// Each override does what `Overrides` says, alone or over the children, nested overrides
    /// apply first, and a stage's overrides cover the stage.
    #[test]
    fn overrides_rescale_move_and_replace() {
        let bay = || attached("bay", mass_component(0.4, 0.1, 0.02), top(0.3));
        let mut airframe = body("airframe", tube(0.8, 0.03, 0.001));
        airframe.children = vec![bay()];
        let plain = rocket(vec![stage(
            "s",
            vec![body("nose", nose(0.2, 0.03)), airframe.clone()],
        )]);
        let base = plain.layout().unwrap();
        let (_, tube_plain) = base.find("airframe").unwrap();
        let (_, bay_plain) = base.find("bay").unwrap();

        // Mass alone: the tube doubles, keeping its centre, and its tensor doubles.
        let mut overridden = airframe.clone();
        overridden.overrides.mass_kg = Some(2.0 * tube_plain.own.mass_kg);
        let design = rocket(vec![stage(
            "s",
            vec![body("nose", nose(0.2, 0.03)), overridden.clone()],
        )]);
        let layout = design.layout().unwrap();
        let (_, t) = layout.find("airframe").unwrap();
        close(
            t.own.mass_kg,
            2.0 * tube_plain.own.mass_kg,
            1e-15,
            "scaled mass",
        );
        assert!((t.own.cg_m - tube_plain.own.cg_m).length() < 1e-15);
        tensor_close(
            t.own.inertia_kg_m2,
            tube_plain.own.inertia_kg_m2 * 2.0,
            1e-14,
        );
        let with = MassProperties::combine([&t.own, &bay_plain.with_children]);
        assert_eq!(t.with_children, with, "the child is still added");

        // Centre and inertia over the tube and its child together.
        overridden.overrides = Overrides {
            mass_kg: Some(1.5),
            cg_aft_m: Some(0.35),
            cg_xy_m: Some([0.001, -0.002]),
            inertia: Some(InertiaOverride::axisymmetric(0.001, 0.08)),
        };
        overridden.overrides_include_children = true;
        let design = rocket(vec![stage(
            "s",
            vec![body("nose", nose(0.2, 0.03)), overridden.clone()],
        )]);
        let layout = design.layout().unwrap();
        let (_, t) = layout.find("airframe").unwrap();
        assert_eq!(t.own, tube_plain.own, "own mass untouched");
        close(t.with_children.mass_kg, 1.5, 0.0, "mass");
        close(
            t.with_children.cg_m.z,
            -(0.2 + 0.35),
            1e-15,
            "centre from the tube's forward end",
        );
        assert_eq!(
            (t.with_children.cg_m.x, t.with_children.cg_m.y),
            (0.001, -0.002)
        );
        assert_eq!(
            t.with_children.inertia_kg_m2,
            DMat3::from_diagonal(DVec3::new(0.08, 0.08, 0.001))
        );

        // A stage override applies last, over everything, from the stage's forward end.
        let mut design = design;
        design.stages[0].overrides = Overrides {
            mass_kg: Some(3.0),
            cg_aft_m: Some(0.5),
            cg_xy_m: None,
            inertia: None,
        };
        let layout = design.layout().unwrap();
        let stage_mass = layout.stages[0].mass;
        close(stage_mass.mass_kg, 3.0, 0.0, "stage mass");
        close(stage_mass.cg_m.z, -0.5, 1e-15, "stage centre");
        let bodies: Vec<MassProperties> = layout.body().map(|c| c.with_children).collect();
        let unscaled = MassProperties::combine(&bodies);
        // Without `cg_xy_m` the stage keeps its offset, which the airframe's override gave it.
        assert!(unscaled.cg_m.x.abs() > 1e-5);
        assert_eq!(
            (stage_mass.cg_m.x, stage_mass.cg_m.y),
            (unscaled.cg_m.x, unscaled.cg_m.y)
        );
        tensor_close(
            stage_mass.inertia_kg_m2,
            unscaled.inertia_kg_m2 * (3.0 / unscaled.mass_kg),
            1e-14,
        );
        assert_eq!(layout.structure, stage_mass);

        // A massless part given a mass becomes a point mass at its centre.
        airframe.children = vec![attached(
            "ballast",
            mass_component(0.0, 0.1, 0.02),
            top(0.3),
        )];
        airframe.children[0].overrides.mass_kg = Some(0.25);
        let design = rocket(vec![stage(
            "s",
            vec![body("nose", nose(0.2, 0.03)), airframe],
        )]);
        let layout = design.layout().unwrap();
        let (_, b) = layout.find("ballast").unwrap();
        assert_eq!(b.own.mass_kg, 0.25);
        close(
            b.own.cg_m.z,
            -(0.2 + 0.3 + 0.05),
            1e-15,
            "point mass centre",
        );
        assert_eq!(b.own.inertia_kg_m2, DMat3::ZERO);

        // Overrides that make no real body are refused.
        let in_stage = |design: &Rocket| match design.layout() {
            Err(DesignError::InComponent { id, source }) if id == "s" => *source,
            other => panic!("{other:?}"),
        };
        let mut bad = plain.clone();
        bad.stages[0].overrides.mass_kg = Some(-1.0);
        assert!(matches!(in_stage(&bad), DesignError::Domain { .. }));
        let mut bad = plain.clone();
        bad.stages[0].overrides.inertia = Some(InertiaOverride::axisymmetric(1.0, 0.1));
        assert!(matches!(in_stage(&bad), DesignError::UnphysicalInertia(_)));
        for broken in [
            Overrides {
                cg_aft_m: Some(f64::NAN),
                ..Overrides::default()
            },
            Overrides {
                cg_xy_m: Some([0.0, f64::INFINITY]),
                ..Overrides::default()
            },
        ] {
            let mut bad = plain.clone();
            bad.stages[0].overrides = broken;
            assert!(matches!(in_stage(&bad), DesignError::Domain { .. }));
        }
        let mut bad = plain.clone();
        bad.stages[0].overrides.inertia = Some(InertiaOverride {
            xy_kg_m2: f64::NAN,
            ..InertiaOverride::axisymmetric(0.1, 1.0)
        });
        assert!(matches!(in_stage(&bad), DesignError::UnphysicalInertia(_)));
        // Zero mass scales the tensor to zero, but a massless body can't be given an inertia.
        let mut zero = plain.clone();
        zero.stages[0].overrides.mass_kg = Some(0.0);
        assert_eq!(zero.layout().unwrap().structure.inertia_kg_m2, DMat3::ZERO);
        let mut bad = plain;
        bad.stages[0].overrides.mass_kg = Some(0.0);
        bad.stages[0].overrides.inertia = Some(InertiaOverride::axisymmetric(1.0, 5.0));
        assert!(matches!(in_stage(&bad), DesignError::UnphysicalInertia(_)));
    }

    /// A tree that doesn't hold together is refused with the offending id.
    #[test]
    fn malformed_trees_are_refused() {
        let base = || {
            rocket(vec![stage(
                "s",
                vec![
                    body("nose", nose(0.2, 0.03)),
                    body("tube", tube(0.5, 0.03, 0.001)),
                ],
            )])
        };
        let tree_error = |design: Rocket, want: &str| match design.layout() {
            Err(DesignError::Tree { id, .. }) => assert_eq!(id, want),
            other => panic!("{want}: {other:?}"),
        };

        let mut d = base();
        d.stages[0].components[1].children =
            vec![attached("mmt", inner_tube(0.2, 0.01, 0.001), top(0.0))];
        d.stages[0].components[1].children[0].children =
            vec![attached("fins", fins(0.1, 0.05), top(0.0))];
        tree_error(d, "fins");

        let mut d = base();
        d.stages[0].components[0].children = vec![attached("fins", fins(0.1, 0.05), top(0.0))];
        tree_error(d, "fins");

        let mut d = base();
        d.stages[0].components[1].children =
            vec![attached("inner-body", tube(0.1, 0.02, 0.001), top(0.0))];
        tree_error(d, "inner-body");

        let mut d = base();
        d.stages[0]
            .components
            .push(body("ballast", mass_component(0.1, 0.1, 0.01)));
        tree_error(d, "ballast");

        let mut d = base();
        d.stages[0].components[1].children = vec![body("unplaced", mass_component(0.1, 0.1, 0.01))];
        tree_error(d, "unplaced");

        let mut d = base();
        d.stages[0].components[1].position = Some(top(0.0));
        tree_error(d, "tube");

        let mut d = base();
        d.stages[0].components[1].auto = vec![AutoDimension::BaseRadius];
        tree_error(d, "tube");

        let mut d = base();
        d.stages[0].components[1].children =
            vec![attached("ring", ring(0.005, 0.02, 0.0), top(0.0))];
        d.stages[0].components[1].children[0].motor_mount = Some(MotorMount::default());
        tree_error(d, "ring");

        let mut d = base();
        d.stages[0].components[1].children =
            vec![attached("bay", mass_component(0.1, 0.1, 0.01), top(0.0))];
        d.stages[0].components[1].children[0].children =
            vec![attached("x", mass_component(0.1, 0.1, 0.01), top(0.0))];
        tree_error(d, "bay");

        let mut d = base();
        d.stages[0].components[0].children =
            vec![attached("weight", mass_component(0.1, 0.05, 0.0), top(0.0))];
        d.stages[0].components[0].children[0].auto = vec![AutoDimension::PackedRadius];
        tree_error(d, "weight");

        let mut d = base();
        d.stages.push(stage("empty", Vec::new()));
        tree_error(d, "empty");

        let mut d = base();
        d.stages[0].components[1].id = "nose".to_owned();
        assert_eq!(d.layout(), Err(DesignError::DuplicateId("nose".to_owned())));
        let mut d = base();
        d.stages[0].components[1].id = String::new();
        assert!(matches!(d.layout(), Err(DesignError::DuplicateId(_))));

        let mut d = base();
        d.stages.clear();
        assert!(matches!(d.layout(), Err(DesignError::Tree { .. })));
    }

    /// A child's own override applies before its parent's override over the subtree: the parent's
    /// mass override rescales a total that already holds the child's overridden mass.
    #[test]
    fn nested_overrides_apply_deepest_first() {
        let mut airframe = body("airframe", tube(0.8, 0.03, 0.001));
        let mut bay = attached("bay", mass_component(0.4, 0.1, 0.02), top(0.3));
        bay.overrides.mass_kg = Some(1.0);
        airframe.children = vec![bay];
        let plain = rocket(vec![stage("s", vec![airframe.clone()])]);
        let layout = plain.layout().unwrap();
        let (_, t) = layout.find("airframe").unwrap();
        let (_, b) = layout.find("bay").unwrap();
        assert_eq!(b.own.mass_kg, 1.0);
        close(
            t.with_children.mass_kg,
            t.own.mass_kg + 1.0,
            1e-15,
            "child override inside",
        );

        airframe.overrides.mass_kg = Some(3.0);
        airframe.overrides_include_children = true;
        let layout = rocket(vec![stage("s", vec![airframe])]).layout().unwrap();
        let (_, t2) = layout.find("airframe").unwrap();
        close(
            t2.with_children.mass_kg,
            3.0,
            0.0,
            "parent override over the total",
        );
        // The rescaled subtree keeps the centre of the tube plus the 1 kg bay, not the 0.4 kg bay.
        close(
            t2.with_children.cg_m.z,
            t.with_children.cg_m.z,
            1e-15,
            "centre kept",
        );
    }

    /// The aft radius and aft shoulder take the tube behind a transition; missing shoulders, a
    /// shoulder with no tube, a nose-base reference with no nose, too deep a tree, and a bad part
    /// are all refused with the offending id.
    #[test]
    fn aft_radii_and_resolution_errors() {
        let boattail = |aft_shoulder: bool| {
            let mut c = body(
                "flare",
                Part::Transition(Transition {
                    shape: crate::NoseShape::Conical {},
                    clipped: false,
                    length_m: 0.1,
                    fore_radius_m: 0.03,
                    aft_radius_m: 0.0,
                    wall: crate::Wall::Shell { thickness_m: 0.002 },
                    fore_shoulder: None,
                    aft_shoulder: aft_shoulder.then_some(Shoulder {
                        length_m: 0.04,
                        outer_radius_m: 0.0,
                        thickness_m: 0.002,
                        capped: false,
                    }),
                    material: crate::testing::cardboard(),
                }),
            );
            c.auto = vec![AutoDimension::AftRadius, AutoDimension::AftShoulderRadius];
            c
        };
        let with = |flare: Component, last: Component| {
            rocket(vec![stage(
                "s",
                vec![
                    body("nose", nose(0.2, 0.03)),
                    body("upper", tube(0.5, 0.03, 0.001)),
                    flare,
                    last,
                ],
            )])
        };
        let layout = with(boattail(true), body("lower", tube(0.4, 0.04, 0.0015)))
            .layout()
            .unwrap();
        let Part::Transition(t) = layout.find("flare").unwrap().1.part.clone() else {
            panic!()
        };
        assert_eq!(t.aft_radius_m, 0.04);
        assert_eq!(t.aft_shoulder.unwrap().outer_radius_m, 0.04 - 0.0015);

        let tree_id = |result: Result<Layout, DesignError>| match result {
            Err(DesignError::Tree { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        // No shoulder to size.
        let lower = || body("lower", tube(0.4, 0.04, 0.0015));
        assert_eq!(tree_id(with(boattail(false), lower()).layout()), "flare");
        // A shoulder into a transition, not a tube.
        let mut cone = boattail(true);
        cone.id = "cone".to_owned();
        cone.auto = vec![AutoDimension::AftShoulderRadius];
        if let Part::Transition(t) = &mut cone.part {
            t.aft_radius_m = 0.02;
        }
        assert_eq!(tree_id(with(boattail(true), cone).layout()), "flare");

        let mut no_nose = with(boattail(true), lower());
        no_nose.stages[0].components.remove(0);
        no_nose.reference_diameter = ReferenceDiameter::NoseBase {};
        assert!(matches!(no_nose.layout(), Err(DesignError::Tree { .. })));

        // A body tube holding `n` nested inner tubes: `n + 1` levels.
        let nest = |n: usize| {
            let mut deepest = attached("t0", inner_tube(0.1, 0.02, 0.001), top(0.0));
            for k in 1..n {
                let mut outer = attached(&format!("t{k}"), inner_tube(0.1, 0.02, 0.001), top(0.0));
                outer.children = vec![deepest];
                deepest = outer;
            }
            let mut airframe = body("airframe", tube(0.5, 0.03, 0.001));
            airframe.children = vec![deepest];
            rocket(vec![stage("s", vec![airframe])])
        };
        nest(MAX_DEPTH - 1).layout().unwrap();
        assert_eq!(tree_id(nest(MAX_DEPTH).layout()), "t0");

        // A part's own error names the part.
        let bad = with(boattail(true), body("lower", tube(0.4, 0.04, -0.001)));
        assert!(matches!(
            bad.layout(),
            Err(DesignError::InComponent { ref id, ref source })
                if id == "lower" && matches!(**source, DesignError::Domain { .. })
        ));
        let mut bad = with(boattail(true), lower());
        bad.stages[0].components[1].children = vec![attached(
            "lost",
            mass_component(0.1, 0.1, 0.01),
            top(f64::NAN),
        )];
        assert!(matches!(
            bad.layout(),
            Err(DesignError::InComponent { ref id, .. }) if id == "lost"
        ));
    }

    /// An automatic packed radius fills the bore on the part's side of the axis, and an offset
    /// outside the bore is refused.
    #[test]
    fn packed_radius_leaves_room_for_the_offset() {
        let mut design = three_fin_rocket();
        if let Part::Parachute(chute) = &mut design.stages[0].components[1].children[4].part {
            chute.packing.radial_offset_m = 0.005;
        }
        let layout = design.layout().unwrap();
        let Part::Parachute(chute) = layout.find("chute").unwrap().1.part.clone() else {
            panic!()
        };
        close(
            chute.packing.radius_m,
            0.0255 - 0.005,
            1e-15,
            "packed radius",
        );
        assert!(crate::checks::check(&design).unwrap().is_empty());
        if let Part::Parachute(chute) = &mut design.stages[0].components[1].children[4].part {
            chute.packing.radial_offset_m = 0.03;
        }
        assert!(matches!(
            design.layout(),
            Err(DesignError::Tree { ref id, .. }) if id == "chute"
        ));
    }

    #[test]
    fn design_round_trips_through_json() {
        let design = three_fin_rocket();
        let text = serde_json::to_string_pretty(&design).unwrap();
        let back: Rocket = serde_json::from_str(&text).unwrap();
        assert_eq!(back, design);
        assert_eq!(back.layout().unwrap(), design.layout().unwrap());
        // Unknown fields are refused, not dropped.
        let bad = text.replacen("\"auto\"", "\"autos\"", 1);
        assert!(serde_json::from_str::<Rocket>(&bad).is_err());
    }

    /// A random spine for the property below: a kind (0 a nose cone, first only; 1 a tube; 2 a
    /// transition), whether each end is automatic, and each end's fixed radius in centimetres.
    type SpineSpec = Vec<(usize, bool, bool, u8, u8)>;

    fn random_spine(spec: &SpineSpec, split: usize) -> Rocket {
        let mut components = Vec::new();
        for (k, &(kind, fore_auto, aft_auto, fore_cm, aft_cm)) in spec.iter().enumerate() {
            let (fore, aft) = (f64::from(fore_cm) * 0.01, f64::from(aft_cm) * 0.01);
            let id = format!("c{k}");
            let mut c = match kind {
                0 if k == 0 => body(&id, nose(0.2, aft)),
                2 => body(&id, cone_transition(fore, aft)),
                _ => body(&id, tube(0.3, aft, 0.001)),
            };
            c.auto = match &c.part {
                Part::NoseCone(_) if aft_auto => vec![AutoDimension::BaseRadius],
                Part::BodyTube(_) if aft_auto => vec![AutoDimension::OuterRadius],
                Part::Transition(_) => [
                    (fore_auto, AutoDimension::ForeRadius),
                    (aft_auto, AutoDimension::AftRadius),
                ]
                .into_iter()
                .filter_map(|(on, dimension)| on.then_some(dimension))
                .collect(),
                _ => Vec::new(),
            };
            components.push(c);
        }
        // Split into two stages somewhere, so the property crosses a stage boundary too.
        let split = split.clamp(1, components.len());
        let aft = components.split_off(split);
        let mut stages = vec![stage("upper", components)];
        if !aft.is_empty() {
            stages.push(stage("lower", aft));
        }
        rocket(stages)
    }

    proptest! {
        /// Over random spines, `unresolvable_body_radii` is empty exactly when `layout` succeeds;
        /// filling what it lists is all `layout` then needs; and every radius the neighbour rule
        /// could already reach comes out as it did before the fill.
        #[test]
        fn unresolvable_radii_are_exactly_what_layout_refuses(
            spec in proptest::collection::vec((0usize..3, any::<bool>(), any::<bool>(), 1u8..5, 1u8..5), 1..7),
            split in 1usize..7,
        ) {
            let mut design = random_spine(&spec, split);
            let unresolvable = design.unresolvable_body_radii();
            prop_assert_eq!(unresolvable.is_empty(), design.layout().is_ok(), "{:?}", unresolvable);

            let body = |design: &Rocket| -> (Vec<Part>, Vec<Vec<AutoDimension>>) {
                design
                    .stages
                    .iter()
                    .flat_map(|s| s.components.iter())
                    .map(|c| (c.part.clone(), c.auto.clone()))
                    .unzip()
            };
            let (parts, autos) = body(&design);
            let autos: Vec<&[AutoDimension]> = autos.iter().map(Vec::as_slice).collect();
            let (fore, aft) = sweep_body_radii(&parts, &autos);

            let filled = design.fill_unresolvable_body_radii(0.025);
            prop_assert_eq!(&filled, &unresolvable);
            prop_assert!(design.unresolvable_body_radii().is_empty());
            let layout = design.layout();
            prop_assert!(layout.is_ok(), "{:?}", layout.as_ref().err());
            let layout = layout.unwrap();
            for (n, placed) in layout.body().enumerate() {
                let (f, a) = match &placed.part {
                    Part::NoseCone(p) => (None, Some(p.base_radius_m)),
                    Part::BodyTube(p) => (Some(p.outer_radius_m), Some(p.outer_radius_m)),
                    Part::Transition(p) => (Some(p.fore_radius_m), Some(p.aft_radius_m)),
                    _ => (None, None),
                };
                // A radius the sweep reached before the fill is the one layout gives after it.
                if let (Some(before), Some(after)) = (fore[n], f) {
                    prop_assert_eq!(before, after, "forward radius of {}", placed.id);
                }
                if let (Some(before), Some(after)) = (aft[n], a) {
                    prop_assert_eq!(before, after, "aft radius of {}", placed.id);
                }
            }
        }

        /// Placed at random along a tube and rolled, point-like masses sum to the structure's mass
        /// and centre, and sliding every part aft by `d` slides the centre by `d` without changing
        /// the tensor.
        #[test]
        fn structure_is_the_mass_weighted_sum_and_slides_rigidly(
            parts in proptest::collection::vec((0.01f64..2.0, 0.0f64..0.7, 0.0f64..0.02, -3.0f64..3.0), 1..6),
            d in -0.1f64..0.1,
        ) {
            let build = |shift: f64| {
                let mut airframe = body("airframe", tube(1.0, 0.03, 0.001));
                airframe.children = parts
                    .iter()
                    .enumerate()
                    .map(|(k, &(m, s, r, angle))| {
                        let mut c = attached(&format!("m{k}"), mass_component(m, 0.05, 0.005), top(s + shift));
                        if let Part::MassComponent(p) = &mut c.part {
                            p.packing.radial_offset_m = r;
                            p.packing.angle_rad = angle;
                        }
                        c
                    })
                    .collect();
                // A massless tube leaves only the masses.
                if let Part::BodyTube(t) = &mut airframe.part {
                    t.material = crate::Material::bulk("none", 0.0);
                }
                rocket(vec![stage("s", vec![airframe])]).layout().unwrap()
            };
            let layout = build(0.0);
            let total: f64 = parts.iter().map(|p| p.0).sum();
            prop_assert!((layout.structure.mass_kg - total).abs() <= 1e-12 * total);
            let moment: f64 = parts.iter().map(|&(m, s, _, _)| m * -(s + 0.025)).sum();
            prop_assert!((layout.structure.cg_m.z - moment / total).abs() <= 1e-12);
            let slid = build(d);
            prop_assert!((slid.structure.cg_m.z - (layout.structure.cg_m.z - d)).abs() <= 1e-12);
            let scale = layout.structure.inertia_kg_m2.to_cols_array().iter().fold(1e-12f64, |m, v| m.max(v.abs()));
            let diff = (slid.structure.inertia_kg_m2 - layout.structure.inertia_kg_m2)
                .to_cols_array()
                .iter()
                .fold(0.0f64, |m, v| m.max(v.abs()));
            prop_assert!(diff <= 1e-9 * scale, "{diff:e} of {scale:e}");
        }
    }
}
