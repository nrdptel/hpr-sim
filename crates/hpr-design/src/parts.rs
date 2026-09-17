//! Components other than fins: nose cones, body tubes, transitions, inner tubes, centering rings
//! and bulkheads, launch lugs, rail buttons, mass components, parachutes, streamers and shock
//! cords, each with its mass properties from geometry.
//!
//! **Frame.** Each part's frame has body axes and its origin on the body axis at the part's forward
//! end (a nose cone's tip), so the part lies at `z ≤ 0` (`crate::mass`). Radial placements use a
//! distance from the axis and a roll angle from `x_B` toward `y_B`.
//!
//! **Standard solids** (Meriam and Kraige, appendix B), for mass `m`:
//!
//! ```text
//! hollow cylinder, radii R > r, length L:  I_axis = m (R² + r²)/2,  I_across = m ((R² + r²)/4 + L²/12)
//! solid cylinder, radius a, length h:      I_axis = m a²/2,          I_across = m (3a² + h²)/12
//! ```
//!
//! Nose cones and transitions use [`crate::solids::revolve`]. **Shoulders** are hollow cylinders
//! beyond the profile's end, and a capped shoulder is closed by a disc of the shoulder's inner
//! radius and wall thickness, flush with its far end. **Recovery parts and mass components** are
//! packed into solid cylinders (OpenRocket technical documentation v13.05, Table 5.1, p. 75, treats
//! them the same way). A parachute's mass is its canopy, `π D²/4` times the fabric's surface
//! density (the nominal area of a flat circular canopy), plus its shroud lines, count times length
//! times line density. See `docs/physics/mass.md`.

use std::f64::consts::PI;

use hpr_core::DVec3;
use serde::{Deserialize, Serialize};

use crate::error::DesignError;
use crate::mass::MassProperties;
use crate::material::Material;
use crate::shapes::{NoseShape, Profile, check_dimension};
use crate::solids::{Wall, revolve};

/// A hollow cylinder of `density` on the axis with its forward end at the origin. A thickness
/// equal to the outer radius gives a solid cylinder.
pub(crate) fn hollow_cylinder(
    part: &'static str,
    density_kg_m3: f64,
    length_m: f64,
    outer_radius_m: f64,
    thickness_m: f64,
) -> Result<MassProperties, DesignError> {
    check_dimension("length", length_m, false)?;
    check_dimension("outer radius", outer_radius_m, false)?;
    check_dimension("wall thickness", thickness_m, false)?;
    if thickness_m > outer_radius_m {
        return Err(DesignError::Geometry(format!(
            "{part}: wall thickness {thickness_m} m exceeds the outer radius {outer_radius_m} m"
        )));
    }
    let (big, small) = (outer_radius_m, outer_radius_m - thickness_m);
    let mass = density_kg_m3 * PI * (big * big - small * small) * length_m;
    let radii = big * big + small * small;
    Ok(MassProperties::axisymmetric(
        mass,
        DVec3::new(0.0, 0.0, -0.5 * length_m),
        0.5 * mass * radii,
        mass * (0.25 * radii + length_m * length_m / 12.0),
    ))
}

/// A solid cylinder of mass `mass_kg` along the axis, forward end at the origin.
fn solid_cylinder(mass_kg: f64, length_m: f64, radius_m: f64) -> MassProperties {
    MassProperties::axisymmetric(
        mass_kg,
        DVec3::new(0.0, 0.0, -0.5 * length_m),
        0.5 * mass_kg * radius_m * radius_m,
        mass_kg * (3.0 * radius_m * radius_m + length_m * length_m) / 12.0,
    )
}

/// Checks that an angle is finite.
fn check_angle(what: &'static str, value: f64) -> Result<(), DesignError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DesignError::Domain { what, value })
    }
}

/// A cylindrical extension of a nose cone or transition that fits inside the adjoining tube.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shoulder {
    /// Length, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Wall thickness, m.
    pub thickness_m: f64,
    /// Whether a disc closes the shoulder's far end.
    #[serde(default)]
    pub capped: bool,
}

impl Shoulder {
    /// Mass properties with the shoulder's near end at `z = near_z_m`, extending forward
    /// (`forward`) or aft.
    fn mass_properties(
        &self,
        density_kg_m3: f64,
        near_z_m: f64,
        forward: bool,
    ) -> Result<MassProperties, DesignError> {
        let tube = hollow_cylinder(
            "shoulder",
            density_kg_m3,
            self.length_m,
            self.outer_radius_m,
            self.thickness_m,
        )?;
        // The tube's frame has its forward end at the origin; put that end in place.
        let fore_z = if forward {
            near_z_m + self.length_m
        } else {
            near_z_m
        };
        let mut parts = vec![tube.translated(DVec3::new(0.0, 0.0, fore_z))];
        let inner = self.outer_radius_m - self.thickness_m;
        if self.capped && inner > 0.0 && self.thickness_m <= self.length_m {
            let cap = hollow_cylinder(
                "shoulder cap",
                density_kg_m3,
                self.thickness_m,
                inner,
                inner,
            )?;
            let cap_fore = if forward {
                fore_z
            } else {
                fore_z - self.length_m + self.thickness_m
            };
            parts.push(cap.translated(DVec3::new(0.0, 0.0, cap_fore)));
        } else if self.capped {
            return Err(DesignError::Geometry(
                "a capped shoulder needs a hollow tube at least as long as its wall is thick"
                    .to_owned(),
            ));
        }
        Ok(MassProperties::combine(&parts))
    }
}

/// A nose cone: a profile with its tip forward, and an optional shoulder aft of its base.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoseCone {
    /// Profile shape.
    pub shape: NoseShape,
    /// Length from tip to base, m.
    pub length_m: f64,
    /// Base radius, m.
    pub base_radius_m: f64,
    /// Filled, or a wall of a thickness.
    pub wall: Wall,
    /// Optional shoulder.
    #[serde(default)]
    pub shoulder: Option<Shoulder>,
    /// Material (bulk).
    pub material: Material,
}

impl NoseCone {
    /// The outer profile.
    ///
    /// # Errors
    ///
    /// As [`Profile::nose`].
    pub fn profile(&self) -> Result<Profile, DesignError> {
        Profile::nose(self.shape, self.length_m, self.base_radius_m)
    }

    /// Mass properties in the nose cone's frame (origin at the tip).
    ///
    /// # Errors
    ///
    /// Geometry, material and numerical errors from the profile, the solid and the shoulder.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        let density = self.material.bulk_kg_m3("nose cone")?;
        let body = revolved(&self.profile()?, self.wall, density)?;
        match &self.shoulder {
            None => Ok(body),
            Some(shoulder) => {
                let aft = shoulder.mass_properties(density, -self.length_m, false)?;
                Ok(MassProperties::combine([&body, &aft]))
            }
        }
    }
}

/// Mass properties of a revolved profile of `density`, forward end at the origin.
fn revolved(profile: &Profile, wall: Wall, density: f64) -> Result<MassProperties, DesignError> {
    let g = revolve(profile, wall)?;
    Ok(MassProperties::axisymmetric(
        density * g.volume_m3,
        DVec3::new(0.0, 0.0, -g.centroid_m),
        density * g.axial_m5,
        density * g.transverse_m5,
    ))
}

/// A transition between two radii, with optional shoulders at either end.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    /// Profile shape.
    pub shape: NoseShape,
    /// Whether the profile is clipped from a longer nose cone (`crate::shapes`).
    #[serde(default)]
    pub clipped: bool,
    /// Length, m.
    pub length_m: f64,
    /// Radius at the forward end, m.
    pub fore_radius_m: f64,
    /// Radius at the aft end, m.
    pub aft_radius_m: f64,
    /// Filled, or a wall of a thickness.
    pub wall: Wall,
    /// Optional shoulder forward of the fore end.
    #[serde(default)]
    pub fore_shoulder: Option<Shoulder>,
    /// Optional shoulder aft of the aft end.
    #[serde(default)]
    pub aft_shoulder: Option<Shoulder>,
    /// Material (bulk).
    pub material: Material,
}

impl Transition {
    /// The outer profile.
    ///
    /// # Errors
    ///
    /// As [`Profile::transition`].
    pub fn profile(&self) -> Result<Profile, DesignError> {
        Profile::transition(
            self.shape,
            self.length_m,
            self.fore_radius_m,
            self.aft_radius_m,
            self.clipped,
        )
    }

    /// Mass properties in the transition's frame (origin at its fore end, so a fore shoulder lies
    /// at `z > 0`).
    ///
    /// # Errors
    ///
    /// Geometry, material and numerical errors from the profile, the solid and the shoulders.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        let density = self.material.bulk_kg_m3("transition")?;
        let mut parts = vec![revolved(&self.profile()?, self.wall, density)?];
        if let Some(shoulder) = &self.fore_shoulder {
            parts.push(shoulder.mass_properties(density, 0.0, true)?);
        }
        if let Some(shoulder) = &self.aft_shoulder {
            parts.push(shoulder.mass_properties(density, -self.length_m, false)?);
        }
        Ok(MassProperties::combine(&parts))
    }
}

/// An airframe tube.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyTube {
    /// Length, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Wall thickness, m.
    pub thickness_m: f64,
    /// Material (bulk).
    pub material: Material,
}

impl BodyTube {
    /// Mass properties in the tube's frame.
    ///
    /// # Errors
    ///
    /// Geometry and material errors.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        let density = self.material.bulk_kg_m3("body tube")?;
        hollow_cylinder(
            "body tube",
            density,
            self.length_m,
            self.outer_radius_m,
            self.thickness_m,
        )
    }
}

/// A tube inside the airframe: a coupler, a motor mount tube, an engine block or thrust ring. It
/// may sit off the axis, as in a cluster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InnerTube {
    /// Length, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Wall thickness, m.
    pub thickness_m: f64,
    /// Distance of the tube's axis from the body axis, m.
    #[serde(default)]
    pub radial_offset_m: f64,
    /// Roll angle of that offset from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub angle_rad: f64,
    /// Material (bulk).
    pub material: Material,
}

impl InnerTube {
    /// Mass properties in the tube's frame.
    ///
    /// # Errors
    ///
    /// Geometry and material errors.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        check_dimension("inner tube radial offset", self.radial_offset_m, true)?;
        check_angle("inner tube angle", self.angle_rad)?;
        let density = self.material.bulk_kg_m3("inner tube")?;
        let tube = hollow_cylinder(
            "inner tube",
            density,
            self.length_m,
            self.outer_radius_m,
            self.thickness_m,
        )?;
        Ok(tube
            .translated(DVec3::new(self.radial_offset_m, 0.0, 0.0))
            .rolled(self.angle_rad))
    }
}

/// A flat annular ring, or a bulkhead when the inner radius is zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CenteringRing {
    /// Thickness along the axis, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Inner radius, m (zero for a bulkhead).
    pub inner_radius_m: f64,
    /// Material (bulk).
    pub material: Material,
}

impl CenteringRing {
    /// A bulkhead: a solid disc.
    pub fn bulkhead(length_m: f64, radius_m: f64, material: Material) -> Self {
        Self {
            length_m,
            outer_radius_m: radius_m,
            inner_radius_m: 0.0,
            material,
        }
    }

    /// Mass properties in the ring's frame.
    ///
    /// # Errors
    ///
    /// Geometry and material errors, including an inner radius not below the outer.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        check_dimension("ring inner radius", self.inner_radius_m, true)?;
        let density = self.material.bulk_kg_m3("centering ring")?;
        hollow_cylinder(
            "centering ring",
            density,
            self.length_m,
            self.outer_radius_m,
            self.outer_radius_m - self.inner_radius_m,
        )
    }
}

/// Copies of a part spaced along the axis: the first at the part's own position, each next one
/// `spacing_m` further aft.
fn instances(
    one: MassProperties,
    count: u32,
    spacing_m: f64,
) -> Result<MassProperties, DesignError> {
    if count == 0 {
        return Err(DesignError::Domain {
            what: "instance count",
            value: 0.0,
        });
    }
    check_dimension("instance spacing", spacing_m, true)?;
    let copies: Vec<MassProperties> = (0..count)
        .map(|k| one.translated(DVec3::new(0.0, 0.0, -spacing_m * f64::from(k))))
        .collect();
    Ok(MassProperties::combine(&copies))
}

/// A launch lug: a tube on the outside of the airframe, parallel to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchLug {
    /// Length, m.
    pub length_m: f64,
    /// Outer radius, m.
    pub outer_radius_m: f64,
    /// Wall thickness, m.
    pub thickness_m: f64,
    /// Roll angle from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub angle_rad: f64,
    /// Number of lugs in a row.
    #[serde(default = "one")]
    pub count: u32,
    /// Axial distance between the forward ends of consecutive lugs, m.
    #[serde(default)]
    pub spacing_m: f64,
    /// Material (bulk).
    pub material: Material,
}

fn one() -> u32 {
    1
}

impl LaunchLug {
    /// Mass properties in the lug's frame, touching a body of radius `body_radius_m`.
    ///
    /// # Errors
    ///
    /// Geometry and material errors.
    pub fn mass_properties(&self, body_radius_m: f64) -> Result<MassProperties, DesignError> {
        check_dimension("body radius", body_radius_m, true)?;
        check_angle("launch lug angle", self.angle_rad)?;
        let density = self.material.bulk_kg_m3("launch lug")?;
        let tube = hollow_cylinder(
            "launch lug",
            density,
            self.length_m,
            self.outer_radius_m,
            self.thickness_m,
        )?;
        let placed = tube
            .translated(DVec3::new(body_radius_m + self.outer_radius_m, 0.0, 0.0))
            .rolled(self.angle_rad);
        instances(placed, self.count, self.spacing_m)
    }
}

/// A rail button: a base disc on the airframe, a narrower waist, and a flange that rides in the
/// rail, stacked outward along a radial line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RailButton {
    /// Diameter of the base and the flange, m.
    pub outer_diameter_m: f64,
    /// Diameter of the waist, m.
    pub inner_diameter_m: f64,
    /// Total height above the airframe, m.
    pub height_m: f64,
    /// Height of the base, m.
    pub base_height_m: f64,
    /// Height of the flange, m.
    pub flange_height_m: f64,
    /// Roll angle from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub angle_rad: f64,
    /// Number of buttons in a row.
    #[serde(default = "one")]
    pub count: u32,
    /// Axial distance between the forward edges of consecutive buttons, m.
    #[serde(default)]
    pub spacing_m: f64,
    /// Material (bulk).
    pub material: Material,
}

impl RailButton {
    /// Mass properties in the button's frame (origin on the body axis at the button's forward
    /// edge), on a body of radius `body_radius_m`. Each disc is a solid cylinder whose axis points
    /// outward.
    ///
    /// # Errors
    ///
    /// Geometry and material errors, including a base and flange taller than the button or a waist
    /// wider than the flange.
    pub fn mass_properties(&self, body_radius_m: f64) -> Result<MassProperties, DesignError> {
        check_dimension("body radius", body_radius_m, true)?;
        check_dimension("rail button outer diameter", self.outer_diameter_m, false)?;
        check_dimension("rail button inner diameter", self.inner_diameter_m, false)?;
        check_dimension("rail button height", self.height_m, false)?;
        check_dimension("rail button base height", self.base_height_m, true)?;
        check_dimension("rail button flange height", self.flange_height_m, true)?;
        check_angle("rail button angle", self.angle_rad)?;
        let waist = self.height_m - self.base_height_m - self.flange_height_m;
        if waist < 0.0 || self.inner_diameter_m > self.outer_diameter_m {
            return Err(DesignError::Geometry(
                "a rail button's base and flange must fit in its height, and its waist in its flange"
                    .to_owned(),
            ));
        }
        let density = self.material.bulk_kg_m3("rail button")?;
        let centre_z = -0.5 * self.outer_diameter_m;
        let disc = |diameter: f64, height: f64, from: f64| -> MassProperties {
            let a = 0.5 * diameter;
            let mass = density * PI * a * a * height;
            let across = mass * (3.0 * a * a + height * height) / 12.0;
            MassProperties {
                mass_kg: mass,
                cg_m: DVec3::new(body_radius_m + from + 0.5 * height, 0.0, centre_z),
                inertia_kg_m2: hpr_core::DMat3::from_diagonal(DVec3::new(
                    0.5 * mass * a * a,
                    across,
                    across,
                )),
            }
        };
        let stack = [
            disc(self.outer_diameter_m, self.base_height_m, 0.0),
            disc(self.inner_diameter_m, waist, self.base_height_m),
            disc(
                self.outer_diameter_m,
                self.flange_height_m,
                self.base_height_m + waist,
            ),
        ];
        let one = MassProperties::combine(&stack).rolled(self.angle_rad);
        instances(one, self.count, self.spacing_m)
    }
}

/// Where and how compactly a mass or a recovery part is stowed: a solid cylinder.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Packing {
    /// Length, m.
    pub length_m: f64,
    /// Radius, m.
    pub radius_m: f64,
    /// Distance of its axis from the body axis, m.
    #[serde(default)]
    pub radial_offset_m: f64,
    /// Roll angle of that offset from `x_B` toward `y_B`, rad.
    #[serde(default)]
    pub angle_rad: f64,
}

impl Packing {
    /// A cylinder of `mass_kg` stowed this way, forward end at the origin.
    fn place(&self, mass_kg: f64) -> Result<MassProperties, DesignError> {
        check_dimension("mass", mass_kg, true)?;
        check_dimension("packed length", self.length_m, true)?;
        check_dimension("packed radius", self.radius_m, true)?;
        check_dimension("radial offset", self.radial_offset_m, true)?;
        check_angle("packing angle", self.angle_rad)?;
        Ok(solid_cylinder(mass_kg, self.length_m, self.radius_m)
            .translated(DVec3::new(self.radial_offset_m, 0.0, 0.0))
            .rolled(self.angle_rad))
    }
}

/// A mass of known value: an altimeter bay, ballast, a payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MassComponent {
    /// Mass, kg.
    pub mass_kg: f64,
    /// Its extent.
    pub packing: Packing,
}

impl MassComponent {
    /// Mass properties in the component's frame.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a negative or non-finite value.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        self.packing.place(self.mass_kg)
    }
}

/// A parachute, packed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parachute {
    /// Nominal (flat) canopy diameter, m.
    pub diameter_m: f64,
    /// Canopy fabric (surface).
    pub canopy_material: Material,
    /// Number of shroud lines.
    pub line_count: u32,
    /// Length of each shroud line, m.
    pub line_length_m: f64,
    /// Shroud line (line).
    pub line_material: Material,
    /// How it is packed.
    pub packing: Packing,
}

impl Parachute {
    /// Canopy plus lines, kg.
    ///
    /// # Errors
    ///
    /// Dimension and material errors.
    pub fn mass_kg(&self) -> Result<f64, DesignError> {
        check_dimension("parachute diameter", self.diameter_m, true)?;
        check_dimension("shroud line length", self.line_length_m, true)?;
        let canopy = self.canopy_material.surface_kg_m2("parachute canopy")?;
        let line = self.line_material.line_kg_m("shroud line")?;
        Ok(canopy * PI * self.diameter_m * self.diameter_m / 4.0
            + f64::from(self.line_count) * self.line_length_m * line)
    }

    /// Mass properties in the parachute's frame.
    ///
    /// # Errors
    ///
    /// As [`Parachute::mass_kg`].
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        self.packing.place(self.mass_kg()?)
    }
}

/// A streamer, packed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Streamer {
    /// Length, m.
    pub length_m: f64,
    /// Width, m.
    pub width_m: f64,
    /// Material (surface).
    pub material: Material,
    /// How it is packed.
    pub packing: Packing,
}

impl Streamer {
    /// Mass properties in the streamer's frame.
    ///
    /// # Errors
    ///
    /// Dimension and material errors.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        check_dimension("streamer length", self.length_m, true)?;
        check_dimension("streamer width", self.width_m, true)?;
        let density = self.material.surface_kg_m2("streamer")?;
        self.packing.place(density * self.length_m * self.width_m)
    }
}

/// A shock cord, packed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShockCord {
    /// Length, m.
    pub length_m: f64,
    /// Material (line).
    pub material: Material,
    /// How it is packed.
    pub packing: Packing,
}

impl ShockCord {
    /// Mass properties in the cord's frame.
    ///
    /// # Errors
    ///
    /// Dimension and material errors.
    pub fn mass_properties(&self) -> Result<MassProperties, DesignError> {
        check_dimension("shock cord length", self.length_m, true)?;
        let density = self.material.line_kg_m("shock cord")?;
        self.packing.place(density * self.length_m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hpr_core::DMat3;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = if want == 0.0 {
            got.abs()
        } else {
            ((got - want) / want).abs()
        };
        assert!(err <= rel, "{what}: {got} vs {want} (relative {err:e})");
    }

    fn cardboard() -> Material {
        Material::bulk("cardboard", 790.0)
    }

    #[test]
    fn a_nose_cone_with_a_capped_shoulder_adds_up_by_hand() {
        // A conical nose, filled, with a capped shoulder: every piece has a closed form.
        let (l, r, rho) = (0.2, 0.03, 1240.0);
        let (ls, rs, ts) = (0.05, 0.028, 0.002);
        let nose = NoseCone {
            shape: NoseShape::Conical {},
            length_m: l,
            base_radius_m: r,
            wall: Wall::Filled {},
            shoulder: Some(Shoulder {
                length_m: ls,
                outer_radius_m: rs,
                thickness_m: ts,
                capped: true,
            }),
            material: Material::bulk("PLA", rho),
        };
        let g = nose.mass_properties().unwrap();
        let cone_m = rho * PI * r * r * l / 3.0;
        let tube_m = rho * PI * (rs * rs - (rs - ts).powi(2)) * ls;
        let cap_m = rho * PI * (rs - ts).powi(2) * ts;
        close(g.mass_kg, cone_m + tube_m + cap_m, 1e-12, "mass");
        let moment = cone_m * (-0.75 * l) + tube_m * (-l - ls / 2.0) + cap_m * (-l - ls + ts / 2.0);
        close(
            g.cg_m.z,
            moment / (cone_m + tube_m + cap_m),
            1e-12,
            "centre",
        );
        // Axial: cone 3/10 m R², tube m(R² + r²)/2, cap m a²/2.
        let ri = rs - ts;
        let axial =
            0.3 * cone_m * r * r + 0.5 * tube_m * (rs * rs + ri * ri) + 0.5 * cap_m * ri * ri;
        close(g.inertia_kg_m2.z_axis.z, axial, 1e-11, "axial");
        // Transverse about the combined centre: cone 3/80 m (4R² + L²) about its own centre.
        let zc = g.cg_m.z;
        let transverse = 3.0 / 80.0 * cone_m * (4.0 * r * r + l * l)
            + cone_m * (-0.75 * l - zc).powi(2)
            + tube_m * ((rs * rs + ri * ri) / 4.0 + ls * ls / 12.0)
            + tube_m * (-l - ls / 2.0 - zc).powi(2)
            + cap_m * (3.0 * ri * ri + ts * ts) / 12.0
            + cap_m * (-l - ls + ts / 2.0 - zc).powi(2);
        close(g.inertia_kg_m2.x_axis.x, transverse, 1e-11, "transverse");
    }

    #[test]
    fn shoulders_sit_beyond_the_right_ends() {
        let shoulder = Shoulder {
            length_m: 0.04,
            outer_radius_m: 0.02,
            thickness_m: 0.001,
            capped: false,
        };
        let t = Transition {
            shape: NoseShape::Conical {},
            clipped: false,
            length_m: 0.1,
            fore_radius_m: 0.021,
            aft_radius_m: 0.03,
            wall: Wall::Shell { thickness_m: 0.002 },
            fore_shoulder: Some(shoulder),
            aft_shoulder: None,
            material: cardboard(),
        };
        let fore = shoulder.mass_properties(790.0, 0.0, true).unwrap();
        close(fore.cg_m.z, 0.02, 1e-15, "fore shoulder centre");
        let aft = shoulder.mass_properties(790.0, -0.1, false).unwrap();
        close(aft.cg_m.z, -0.12, 1e-15, "aft shoulder centre");
        let whole = t.mass_properties().unwrap();
        let body = Transition {
            fore_shoulder: None,
            ..t.clone()
        }
        .mass_properties()
        .unwrap();
        close(whole.mass_kg, body.mass_kg + fore.mass_kg, 1e-14, "mass");
    }

    #[test]
    fn off_axis_parts_carry_their_offsets() {
        // An inner tube in a cluster, a lug and a mass component, each against the parallel-axis
        // theorem.
        let tube = InnerTube {
            length_m: 0.3,
            outer_radius_m: 0.015,
            thickness_m: 0.001,
            radial_offset_m: 0.03,
            angle_rad: PI / 2.0,
            material: cardboard(),
        };
        let g = tube.mass_properties().unwrap();
        assert!((g.cg_m - DVec3::new(0.0, 0.03, -0.15)).length() < 1e-15);
        let own = hollow_cylinder("t", 790.0, 0.3, 0.015, 0.001).unwrap();
        // About the body axis, the axial moment gains m d².
        close(
            g.inertia_about(DVec3::new(0.0, 0.0, -0.15)).z_axis.z,
            own.inertia_kg_m2.z_axis.z + own.mass_kg * 0.03 * 0.03,
            1e-13,
            "axial about the body axis",
        );

        let lug = LaunchLug {
            length_m: 0.05,
            outer_radius_m: 0.003,
            thickness_m: 0.0005,
            angle_rad: 0.0,
            count: 2,
            spacing_m: 0.4,
            material: Material::bulk("brass", 8500.0),
        };
        let g = lug.mass_properties(0.02).unwrap();
        let one = hollow_cylinder("l", 8500.0, 0.05, 0.003, 0.0005).unwrap();
        close(g.mass_kg, 2.0 * one.mass_kg, 1e-14, "two lugs");
        assert!((g.cg_m - DVec3::new(0.023, 0.0, -0.225)).length() < 1e-15);
        close(
            g.inertia_kg_m2.x_axis.x,
            one.inertia_kg_m2.x_axis.x * 2.0 + 2.0 * one.mass_kg * 0.2 * 0.2,
            1e-13,
            "two lugs transverse",
        );

        let mass = MassComponent {
            mass_kg: 0.25,
            packing: Packing {
                length_m: 0.1,
                radius_m: 0.02,
                radial_offset_m: 0.01,
                angle_rad: PI,
            },
        };
        let g = mass.mass_properties().unwrap();
        assert!((g.cg_m - DVec3::new(-0.01, 0.0, -0.05)).length() < 1e-15);
        let expected = DMat3::from_diagonal(DVec3::new(
            0.25 * (3.0 * 0.0004 + 0.01) / 12.0,
            0.25 * (3.0 * 0.0004 + 0.01) / 12.0,
            0.25 * 0.0004 / 2.0,
        ));
        let diff = (g.inertia_kg_m2 - expected)
            .to_cols_array()
            .iter()
            .fold(0.0f64, |m, v| m.max(v.abs()));
        assert!(diff < 1e-18, "{g:?}");
    }

    #[test]
    fn recovery_parts_weigh_their_fabric_and_lines() {
        let chute = Parachute {
            diameter_m: 0.9,
            canopy_material: Material::surface("ripstop nylon", 0.0373),
            line_count: 8,
            line_length_m: 0.9,
            line_material: Material::line("nylon line", 0.0016),
            packing: Packing {
                length_m: 0.1,
                radius_m: 0.03,
                radial_offset_m: 0.0,
                angle_rad: 0.0,
            },
        };
        let canopy = 0.0373 * PI * 0.81 / 4.0;
        let lines = 8.0 * 0.9 * 0.0016;
        close(chute.mass_kg().unwrap(), canopy + lines, 1e-15, "parachute");
        let streamer = Streamer {
            length_m: 1.5,
            width_m: 0.1,
            material: Material::surface("mylar", 0.0353),
            packing: chute.packing,
        };
        close(
            streamer.mass_properties().unwrap().mass_kg,
            1.5 * 0.1 * 0.0353,
            1e-15,
            "streamer",
        );
        let cord = ShockCord {
            length_m: 6.0,
            material: Material::line("Kevlar", 0.00968),
            packing: chute.packing,
        };
        let g = cord.mass_properties().unwrap();
        close(g.mass_kg, 6.0 * 0.00968, 1e-15, "cord");
        close(
            g.inertia_kg_m2.z_axis.z,
            0.5 * g.mass_kg * 0.03 * 0.03,
            1e-15,
            "cord axial",
        );
        let wrong = ShockCord {
            material: cardboard(),
            ..cord
        };
        assert!(matches!(
            wrong.mass_properties(),
            Err(DesignError::MaterialKind { .. })
        ));
    }
}
