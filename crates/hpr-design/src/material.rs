//! Materials: a name and a density per volume, per area or per length.
//!
//! Solid parts (tubes, nose cones, fins, rings) take a **bulk** density, fabrics (parachute
//! canopies, streamers) a **surface** density, and cords (shroud lines, shock cords) a **line**
//! density. A design stores the material's values, not a reference into a library, so it stays
//! complete offline; [`crate::materials`] lists built-in values with their sources.

use serde::{Deserialize, Serialize};

use crate::error::DesignError;

/// A density, with its units in the variant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Density {
    /// Mass per volume.
    Bulk {
        /// kg/m³.
        kg_m3: f64,
    },
    /// Mass per area, for sheets and fabrics.
    Surface {
        /// kg/m².
        kg_m2: f64,
    },
    /// Mass per length, for cords and lines.
    Line {
        /// kg/m.
        kg_m: f64,
    },
}

impl Density {
    /// The kind's name, for messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Bulk { .. } => "bulk",
            Self::Surface { .. } => "surface",
            Self::Line { .. } => "line",
        }
    }
}

/// A named material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    /// Name, for display and for matching imported designs.
    pub name: String,
    /// Density.
    pub density: Density,
}

impl Material {
    /// A bulk material of `kg_m3`.
    pub fn bulk(name: impl Into<String>, kg_m3: f64) -> Self {
        Self {
            name: name.into(),
            density: Density::Bulk { kg_m3 },
        }
    }

    /// A surface material of `kg_m2`.
    pub fn surface(name: impl Into<String>, kg_m2: f64) -> Self {
        Self {
            name: name.into(),
            density: Density::Surface { kg_m2 },
        }
    }

    /// A line material of `kg_m`.
    pub fn line(name: impl Into<String>, kg_m: f64) -> Self {
        Self {
            name: name.into(),
            density: Density::Line { kg_m },
        }
    }

    /// The value of a density of the kind `expected` names, for `part`.
    fn value(&self, part: &'static str, expected: &'static str) -> Result<f64, DesignError> {
        let value = match (self.density, expected) {
            (Density::Bulk { kg_m3 }, "bulk") => kg_m3,
            (Density::Surface { kg_m2 }, "surface") => kg_m2,
            (Density::Line { kg_m }, "line") => kg_m,
            _ => {
                return Err(DesignError::MaterialKind {
                    part,
                    material: self.name.clone(),
                    expected,
                    actual: self.density.kind_name(),
                });
            }
        };
        if value.is_finite() && value >= 0.0 {
            Ok(value)
        } else {
            Err(DesignError::Domain {
                what: "material density",
                value,
            })
        }
    }

    /// The bulk density for `part`, kg/m³.
    ///
    /// # Errors
    ///
    /// [`DesignError::MaterialKind`] if the material isn't bulk, [`DesignError::Domain`] if its
    /// density is negative or not finite.
    pub fn bulk_kg_m3(&self, part: &'static str) -> Result<f64, DesignError> {
        self.value(part, "bulk")
    }

    /// The surface density for `part`, kg/m²; errors as [`Material::bulk_kg_m3`].
    ///
    /// # Errors
    ///
    /// As [`Material::bulk_kg_m3`].
    pub fn surface_kg_m2(&self, part: &'static str) -> Result<f64, DesignError> {
        self.value(part, "surface")
    }

    /// The line density for `part`, kg/m; errors as [`Material::bulk_kg_m3`].
    ///
    /// # Errors
    ///
    /// As [`Material::bulk_kg_m3`].
    pub fn line_kg_m(&self, part: &'static str) -> Result<f64, DesignError> {
        self.value(part, "line")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_checked_and_serialized_with_their_units() {
        let ply = Material::bulk("Birch plywood", 630.0);
        assert_eq!(ply.bulk_kg_m3("fin set").unwrap(), 630.0);
        assert_eq!(
            ply.surface_kg_m2("parachute canopy"),
            Err(DesignError::MaterialKind {
                part: "parachute canopy",
                material: "Birch plywood".to_owned(),
                expected: "surface",
                actual: "bulk",
            })
        );
        let json = serde_json::to_string(&Material::line("Kevlar", 0.0097)).unwrap();
        assert_eq!(
            json,
            r#"{"name":"Kevlar","density":{"kind":"line","kg_m":0.0097}}"#
        );
        assert!(Material::bulk("bad", -1.0).bulk_kg_m3("tube").is_err());
        assert!(
            serde_json::from_str::<Material>(r#"{"name":"x","density":{"kind":"bulk","kg_m2":1}}"#)
                .is_err()
        );
    }
}
