//! The launch rail: its direction, length and friction, and where a design's rail buttons or launch
//! lugs leave it.
//!
//! The rocket starts with its aft end (the aft end of its body or its aft-most nozzle exit,
//! whichever is further aft) at the foot of the rail, its axis along the rail. It is guided, with
//! one degree of freedom along the rail, until the aft edge of its aft-most guide passes the top
//! of the rail. A rocket with no rail buttons or lugs is guided until its aft end passes the top,
//! as from a tower. Between the forward guide leaving and the aft guide leaving, a real rocket can
//! pivot about the aft guide ("tip-off"); that rotation is not modelled, and the rocket leaves the
//! rail with no angular velocity.
//!
//! Friction is Coulomb friction, `μ |N|`, with `N` the rail's reaction perpendicular to the rail:
//! the component of gravity, the aerodynamic force and the variable-mass terms across the rocket's
//! axis. It opposes the motion along the rail, and on the pad it holds the rocket until the force
//! along the rail exceeds it.
//!
//! Method: `docs/physics/flight.md`.

use hpr_core::frames::LaunchAngles;
use hpr_core::{DQuat, DVec3};
use hpr_design::{Assembly, Part};
use serde::{Deserialize, Serialize};

use crate::error::SimError;

/// A launch rail.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rail {
    /// The rail's length from its foot, where the rocket's aft end starts, to its top, m.
    pub length_m: f64,
    /// The heading of the rail, clockwise from true north, rad (`frames::LaunchAngles`).
    pub azimuth_rad: f64,
    /// The rail's angle above the horizon, rad (`π/2` is vertical).
    pub elevation_rad: f64,
    /// The rocket's roll on the rail: the turn of `x_B` about the rail axis, rad.
    pub roll_rad: f64,
    /// The Coulomb friction coefficient between the guides and the rail. There is no default
    /// value in the sources; zero means a frictionless rail.
    pub friction_coefficient: f64,
}

impl Rail {
    /// A frictionless vertical rail of length `length_m`.
    #[must_use]
    pub fn vertical(length_m: f64) -> Self {
        Self {
            length_m,
            azimuth_rad: 0.0,
            elevation_rad: std::f64::consts::FRAC_PI_2,
            roll_rad: 0.0,
            friction_coefficient: 0.0,
        }
    }

    /// Checks the length, angles and friction.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for a non-positive length, a negative friction coefficient, an
    /// elevation outside `(0, π/2]` or a non-finite angle.
    pub fn validate(&self) -> Result<(), SimError> {
        let checks = [
            ("rail length", self.length_m, self.length_m > 0.0),
            (
                "rail friction coefficient",
                self.friction_coefficient,
                self.friction_coefficient >= 0.0,
            ),
            (
                "rail elevation",
                self.elevation_rad,
                self.elevation_rad > 0.0 && self.elevation_rad <= std::f64::consts::FRAC_PI_2,
            ),
            ("rail azimuth", self.azimuth_rad, true),
            ("rail roll", self.roll_rad, true),
        ];
        for (what, value, ok) in checks {
            if !value.is_finite() || !ok {
                return Err(SimError::Domain { what, value });
            }
        }
        Ok(())
    }

    /// The rocket's attitude on the rail.
    #[must_use]
    pub fn attitude(&self) -> DQuat {
        LaunchAngles {
            azimuth_rad: self.azimuth_rad,
            elevation_rad: self.elevation_rad,
            roll_rad: self.roll_rad,
        }
        .to_quaternion()
    }

    /// The unit vector along the rail, up, in the launch frame.
    #[must_use]
    pub fn direction_enu(&self) -> DVec3 {
        self.attitude().mul_vec3(DVec3::Z)
    }
}

/// Where a design meets the rail, as stations (m aft of the nose tip).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Guides {
    /// The rocket's aft end: the aft end of its body, or its aft-most nozzle exit if that is
    /// further aft. It starts at the foot of the rail.
    pub aft_station_m: f64,
    /// The aft edge of the forward-most guide (a rail button's aft edge, or a lug's aft end), or
    /// `None` without guides.
    pub first_guide_station_m: Option<f64>,
    /// The aft edge of the aft-most guide, or `None` without guides.
    pub last_guide_station_m: Option<f64>,
}

impl Guides {
    /// The guides of an assembled design: every rail button of every row (each `outer_diameter`
    /// long, `spacing` apart) and every launch lug (each `length` long).
    #[must_use]
    pub fn of(assembly: &Assembly) -> Self {
        let body_aft = assembly.layout.length_m;
        let aft_station_m = assembly
            .motors
            .iter()
            .map(|motor| motor.nozzle_station_m())
            .fold(body_aft, f64::max);
        let mut first: Option<f64> = None;
        let mut last: Option<f64> = None;
        for component in &assembly.layout.components {
            let (count, spacing, extent) = match &component.part {
                Part::RailButton(button) => {
                    (button.count, button.spacing_m, button.outer_diameter_m)
                }
                Part::LaunchLug(lug) => (lug.count, lug.spacing_m, lug.length_m),
                _ => continue,
            };
            for k in 0..count {
                let aft_edge = component.fore_station_m + f64::from(k) * spacing + extent;
                first = Some(first.map_or(aft_edge, |f| f.min(aft_edge)));
                last = Some(last.map_or(aft_edge, |l| l.max(aft_edge)));
            }
        }
        Self {
            aft_station_m,
            first_guide_station_m: first,
            last_guide_station_m: last,
        }
    }

    /// How far the rocket travels along a rail of `rail_length_m` before its last guide leaves the
    /// top: `L − (s_aft − s_guide)`, with the aft end standing in for the guide without guides.
    #[must_use]
    pub fn exit_travel_m(&self, rail_length_m: f64) -> f64 {
        let guide = self.last_guide_station_m.unwrap_or(self.aft_station_m);
        rail_length_m - (self.aft_station_m - guide)
    }

    /// How far the rocket travels before its first guide leaves the top.
    #[must_use]
    pub fn first_guide_exit_travel_m(&self, rail_length_m: f64) -> f64 {
        let guide = self.first_guide_station_m.unwrap_or(self.aft_station_m);
        rail_length_m - (self.aft_station_m - guide)
    }
}

#[cfg(test)]
mod tests;
