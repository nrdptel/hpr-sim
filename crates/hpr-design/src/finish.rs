//! Surface finishes: the roughness height that limits a surface's skin friction.
//!
//! The named finishes are the rows of Barrowman 1967 Table 4-1 ("Approximate Surface Roughness
//! Heights of Physical Surfaces", p. 46, after Hoerner, *Fluid-Dynamic Drag*, 1965, p. 5-3).
//! Niskanen 2009 Table 3.2 (p. 44) reprints ten of its fifteen rows with the same heights. Any
//! other roughness is a [`Finish::Custom`] height. See `docs/physics/aero.md`.

use serde::{Deserialize, Serialize};

use crate::error::DesignError;
use crate::shapes::check_dimension;

/// A surface finish, by its roughness height `R_s`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Finish {
    /// A mirror-like surface, 0 µm (Barrowman Table 4-1).
    Mirror {},
    /// Average glass, 0.1 µm.
    AverageGlass {},
    /// A finished and polished surface, 0.5 µm.
    Polished {},
    /// An aircraft-type sheet-metal surface, 2 µm (Barrowman Table 4-1 only).
    SheetMetal {},
    /// An optimum paint-sprayed surface, 5 µm.
    OptimumPaint {},
    /// Planed wooden boards, 15 µm.
    PlanedWood {},
    /// Paint in aircraft mass production, 20 µm. The default.
    MassProductionPaint {},
    /// Bare steel plating, 50 µm (Barrowman Table 4-1 only).
    BareSteel {},
    /// A smooth cement surface, 50 µm.
    SmoothCement {},
    /// A surface with an asphalt-type coating, 100 µm (Barrowman Table 4-1 only).
    AsphaltCoating {},
    /// A dip-galvanized metal surface, 150 µm.
    DipGalvanized {},
    /// Incorrectly sprayed aircraft paint, 200 µm.
    PoorPaint {},
    /// The natural surface of cast iron, 250 µm (Barrowman Table 4-1 only).
    CastIron {},
    /// Raw wooden boards, 500 µm.
    RawWood {},
    /// An average concrete surface, 1000 µm.
    Concrete {},
    /// A given roughness height.
    Custom {
        /// Roughness height, m.
        roughness_m: f64,
    },
}

impl Default for Finish {
    fn default() -> Self {
        Finish::MassProductionPaint {}
    }
}

impl Finish {
    /// Every named finish, smoothest first.
    pub const NAMED: &'static [Finish] = &[
        Finish::Mirror {},
        Finish::AverageGlass {},
        Finish::Polished {},
        Finish::SheetMetal {},
        Finish::OptimumPaint {},
        Finish::PlanedWood {},
        Finish::MassProductionPaint {},
        Finish::BareSteel {},
        Finish::SmoothCement {},
        Finish::AsphaltCoating {},
        Finish::DipGalvanized {},
        Finish::PoorPaint {},
        Finish::CastIron {},
        Finish::RawWood {},
        Finish::Concrete {},
    ];

    /// Roughness height `R_s`, m.
    ///
    /// # Errors
    ///
    /// [`DesignError::Domain`] for a custom height that is negative or not finite.
    pub fn roughness_m(&self) -> Result<f64, DesignError> {
        const MICRON: f64 = 1e-6;
        Ok(match *self {
            Finish::Mirror {} => 0.0,
            Finish::AverageGlass {} => 0.1 * MICRON,
            Finish::Polished {} => 0.5 * MICRON,
            Finish::SheetMetal {} => 2.0 * MICRON,
            Finish::OptimumPaint {} => 5.0 * MICRON,
            Finish::PlanedWood {} => 15.0 * MICRON,
            Finish::MassProductionPaint {} => 20.0 * MICRON,
            Finish::BareSteel {} | Finish::SmoothCement {} => 50.0 * MICRON,
            Finish::AsphaltCoating {} => 100.0 * MICRON,
            Finish::DipGalvanized {} => 150.0 * MICRON,
            Finish::PoorPaint {} => 200.0 * MICRON,
            Finish::CastIron {} => 250.0 * MICRON,
            Finish::RawWood {} => 500.0 * MICRON,
            Finish::Concrete {} => 1000.0 * MICRON,
            Finish::Custom { roughness_m } => {
                check_dimension("roughness height", roughness_m, true)?;
                roughness_m
            }
        })
    }
}
