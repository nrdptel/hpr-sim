//! The flight state: the body origin's position and velocity, the attitude and the body rates.

use hpr_core::{DQuat, DVec3};
use serde::{Deserialize, Serialize};

/// The number of components in a [`State`] array.
pub const STATE_LEN: usize = 13;

/// A rigid body's state in the launch frame `L` (`docs/physics/frames.md`).
///
/// The reference point is the body origin, the nose tip (ADR-007), which is fixed in the body; the
/// centre of mass moves relative to it as propellant burns. As an array the order is position,
/// velocity, attitude `(w, x, y, z)` and body rates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// The nose tip's position in `L`, m.
    pub position_enu_m: DVec3,
    /// The nose tip's velocity relative to `L`, m/s.
    pub velocity_enu_m_s: DVec3,
    /// The attitude, body to launch frame. The integrated quaternion's norm drifts slightly; every
    /// use normalizes it.
    pub attitude: DQuat,
    /// The body's angular velocity relative to `L`, in body axes, rad/s.
    pub body_rate_rad_s: DVec3,
}

impl State {
    /// The state as the integrator's array.
    #[must_use]
    pub fn to_array(&self) -> [f64; STATE_LEN] {
        let (p, v, q, w) = (
            self.position_enu_m,
            self.velocity_enu_m_s,
            self.attitude,
            self.body_rate_rad_s,
        );
        [
            p.x, p.y, p.z, v.x, v.y, v.z, q.w, q.x, q.y, q.z, w.x, w.y, w.z,
        ]
    }

    /// The state from the integrator's array.
    #[must_use]
    pub fn from_array(y: &[f64; STATE_LEN]) -> Self {
        Self {
            position_enu_m: DVec3::new(y[0], y[1], y[2]),
            velocity_enu_m_s: DVec3::new(y[3], y[4], y[5]),
            attitude: DQuat::from_xyzw(y[7], y[8], y[9], y[6]),
            body_rate_rad_s: DVec3::new(y[10], y[11], y[12]),
        }
    }

    /// The attitude scaled to unit length.
    #[must_use]
    pub fn unit_attitude(&self) -> DQuat {
        self.attitude.normalize()
    }

    /// A point given in body axes (m from the nose tip), in `L`.
    #[must_use]
    pub fn point_enu_m(&self, body_m: DVec3) -> DVec3 {
        self.position_enu_m + self.unit_attitude().mul_vec3(body_m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrays_round_trip() {
        let state = State {
            position_enu_m: DVec3::new(1.0, 2.0, 3.0),
            velocity_enu_m_s: DVec3::new(4.0, 5.0, 6.0),
            attitude: DQuat::from_xyzw(0.1, 0.2, 0.3, 0.9),
            body_rate_rad_s: DVec3::new(7.0, 8.0, 9.0),
        };
        assert_eq!(State::from_array(&state.to_array()), state);
        assert_eq!(state.to_array()[6], 0.9);
    }
}
