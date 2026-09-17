//! Attitude quaternion kinematics.
//!
//! The attitude is a unit Hamilton quaternion `q` that rotates body-frame components into
//! launch-frame (ENU) components, `v_L = q ⊗ v_B ⊗ q*` (glam's [`DQuat::mul_vec3`]); see
//! `docs/physics/frames.md`. With `ω_B` the body's angular velocity relative to the launch frame,
//! resolved in the body frame, the kinematic equation is
//!
//! ```text
//! q̇ = ½ q ⊗ (0, ω_B)
//! ```
//!
//! (J. Solà, *Quaternion kinematics for the error-state Kalman filter*, arXiv:1711.02508v1, 2017,
//! eq. 200 with the local angular rate `ω_L`; the same Hamilton convention and local-to-global
//! mapping, eq. 206.)
//!
//! A general-purpose integrator applied to `q̇` does not preserve `|q| = 1`, so the flight engine
//! projects back onto the unit sphere after every accepted step ([`renormalize`]). For a rate that
//! is constant over the step, [`step_constant_rate`] is exact up to rounding:
//! `q(t + Δt) = q(t) ⊗ exp(½ ω_B Δt)` (Solà eq. 215, zeroth-order integration).

use glam::{DQuat, DVec3};

use crate::error::CoreError;

/// The time derivative `q̇ = ½ q ⊗ (0, ω_B)` of the attitude quaternion, as a (non-unit)
/// quaternion, for a body angular velocity `omega_body_rad_s` resolved in the body frame.
#[must_use]
pub fn quaternion_derivative(q: DQuat, omega_body_rad_s: DVec3) -> DQuat {
    let omega = DQuat::from_xyzw(
        omega_body_rad_s.x,
        omega_body_rad_s.y,
        omega_body_rad_s.z,
        0.0,
    );
    (q * omega) * 0.5
}

/// Projects `q` back onto the unit sphere after an integration step.
///
/// # Errors
///
/// [`CoreError::Domain`] if `q` has zero, subnormal, infinite or NaN length, because then it no
/// longer encodes a rotation.
pub fn renormalize(q: DQuat) -> Result<DQuat, CoreError> {
    let length = q.length();
    if length.is_finite() && length >= f64::MIN_POSITIVE {
        Ok(q / length)
    } else {
        Err(CoreError::Domain {
            what: "attitude quaternion length",
            value: length,
        })
    }
}

/// Advances the attitude by `dt_s` under a body rate that is constant over the step:
/// `q ⊗ exp(½ ω_B Δt)`, then renormalized.
///
/// # Errors
///
/// As [`renormalize`], which only fails if `q` or the rotation is not finite.
pub fn step_constant_rate(
    q: DQuat,
    omega_body_rad_s: DVec3,
    dt_s: f64,
) -> Result<DQuat, CoreError> {
    let increment = DQuat::from_scaled_axis(omega_body_rad_s * dt_s);
    renormalize(q * increment)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spinning, coning body with a closed-form attitude. With
    /// `q(t) = exp(½ α t u) ⊗ exp(½ β t w)`, differentiating gives `q̇ = ½ q ⊗ (0, ω_B)` with
    /// `ω_B(t) = β w + R(exp(½ β t w))ᵀ α u`: a spin `β` about the body axis `w` while that axis
    /// precesses at `α` about the fixed axis `u`.
    struct Coning {
        alpha: f64,
        u: DVec3,
        beta: f64,
        w: DVec3,
    }

    impl Coning {
        fn attitude(&self, t: f64) -> DQuat {
            DQuat::from_scaled_axis(self.u * self.alpha * t)
                * DQuat::from_scaled_axis(self.w * self.beta * t)
        }

        fn body_rate(&self, t: f64) -> DVec3 {
            let spin = DQuat::from_scaled_axis(self.w * self.beta * t);
            self.w * self.beta + spin.inverse().mul_vec3(self.u * self.alpha)
        }
    }

    fn coning() -> Coning {
        Coning {
            alpha: 0.3,
            u: DVec3::new(0.0, 0.0, 1.0),
            beta: 5.0,
            w: DVec3::new(1.0, 2.0, 2.0) / 3.0,
        }
    }

    /// The angle of the rotation taking `a` to `b`, in radians. `atan2` stays accurate for tiny
    /// angles, where `acos` of the dot product loses half the digits.
    fn angle_between(a: DQuat, b: DQuat) -> f64 {
        let d = a.inverse() * b;
        2.0 * d.xyz().length().atan2(d.w.abs())
    }

    /// Checks the kinematic equation itself against the closed-form motion by central differences.
    #[test]
    fn derivative_matches_closed_form_motion() {
        let c = coning();
        for t in [0.0, 0.7, 3.1, 12.0] {
            let h = 1e-6;
            let numeric = (c.attitude(t + h) - c.attitude(t - h)) * (0.5 / h);
            let analytic = quaternion_derivative(c.attitude(t), c.body_rate(t));
            assert!((numeric - analytic).length() < 1e-8, "t = {t}");
        }
    }

    /// M1.1 *done when*: the norm stays within 1e-12 of one over 1e6 steps. Classical RK4 on
    /// `q̇`, renormalized after each step as the flight engine does. Measured after
    /// renormalization the bound is met by construction, so the test also bounds the drift of
    /// each raw step before renormalization (a derivative with a spurious real part grows the
    /// norm and fails it) and checks the attitude against the closed form (a wrong rotation
    /// fails that).
    #[test]
    fn rk4_with_renormalization_keeps_unit_norm_over_a_million_steps() {
        let c = coning();
        let dt = 1e-4;
        let steps = 1_000_000;
        let mut q = c.attitude(0.0);
        let mut worst_norm_error = 0.0f64;
        let mut worst_step_drift = 0.0f64;
        for n in 0..steps {
            let t = f64::from(n) * dt;
            let k1 = quaternion_derivative(q, c.body_rate(t));
            let k2 = quaternion_derivative(q + k1 * (0.5 * dt), c.body_rate(t + 0.5 * dt));
            let k3 = quaternion_derivative(q + k2 * (0.5 * dt), c.body_rate(t + 0.5 * dt));
            let k4 = quaternion_derivative(q + k3 * dt, c.body_rate(t + dt));
            let raw = q + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0);
            worst_step_drift = worst_step_drift.max((raw.length() - 1.0).abs());
            q = renormalize(raw).unwrap();
            worst_norm_error = worst_norm_error.max((q.length() - 1.0).abs());
        }
        let t_end = f64::from(steps) * dt;
        let attitude_error = angle_between(q, c.attitude(t_end));
        assert!(worst_norm_error <= 1e-12, "norm error {worst_norm_error:e}");
        // For a constant rate RK4 scales the norm by |R(iθ)| = √(1 − θ⁶/72 + θ⁸/576) ≈ 1 − θ⁶/144
        // per step, θ = |ω|Δt/2 ≈ 3e-4, about 1e-23: far below rounding. The raw drift should be
        // a few multiples of f64::EPSILON.
        assert!(
            worst_step_drift <= 16.0 * f64::EPSILON,
            "raw step drift {worst_step_drift:e}"
        );
        assert!(
            attitude_error < 1e-9,
            "attitude error {attitude_error:e} rad"
        );
    }

    /// RK4 with renormalization over the coning motion from `t = 0` to `t_end`; returns the
    /// attitude error against the closed form, rad.
    fn rk4_coning_error(dt: f64, steps: u32) -> f64 {
        let c = coning();
        let mut q = c.attitude(0.0);
        for n in 0..steps {
            let t = f64::from(n) * dt;
            let k1 = quaternion_derivative(q, c.body_rate(t));
            let k2 = quaternion_derivative(q + k1 * (0.5 * dt), c.body_rate(t + 0.5 * dt));
            let k3 = quaternion_derivative(q + k2 * (0.5 * dt), c.body_rate(t + 0.5 * dt));
            let k4 = quaternion_derivative(q + k3 * dt, c.body_rate(t + dt));
            q = renormalize(q + (k1 + k2 * 2.0 + k3 * 2.0 + k4) * (dt / 6.0)).unwrap();
        }
        angle_between(q, c.attitude(f64::from(steps) * dt))
    }

    /// Flight-engine step sizes, |ω|Δt of 0.05 to 0.2: the kinematics plus renormalization
    /// converge at RK4's fourth order to the closed-form attitude, so errors in the derivative
    /// that only show at large steps are caught.
    #[test]
    fn large_steps_converge_at_fourth_order() {
        // |ω| ≈ 5.3 rad/s; 20 s of coning at Δt = 0.04, 0.02 and 0.01 s.
        let coarse = rk4_coning_error(0.04, 500);
        let medium = rk4_coning_error(0.02, 1000);
        let fine = rk4_coning_error(0.01, 2000);
        assert!(coarse < 1e-2, "coarse error {coarse:e}");
        for (big, small) in [(coarse, medium), (medium, fine)] {
            let order = (big / small).log2();
            assert!(
                (3.7..4.3).contains(&order),
                "observed order {order}: {big:e} → {small:e}"
            );
        }
    }

    /// The exponential-map step is exact for a constant rate, over a million steps.
    #[test]
    fn constant_rate_steps_track_the_exact_rotation() {
        let omega = DVec3::new(0.4, -1.3, 7.0);
        let q0 = DQuat::from_scaled_axis(DVec3::new(0.2, 0.1, -0.3));
        let dt = 1e-4;
        let steps = 1_000_000;
        let mut q = q0;
        let mut worst_norm_error = 0.0f64;
        let mut worst_step_drift = 0.0f64;
        for _ in 0..steps {
            let raw = q * DQuat::from_scaled_axis(omega * dt);
            worst_step_drift = worst_step_drift.max((raw.length() - 1.0).abs());
            q = step_constant_rate(q, omega, dt).unwrap();
            worst_norm_error = worst_norm_error.max((q.length() - 1.0).abs());
        }
        let exact = q0 * DQuat::from_scaled_axis(omega * (f64::from(steps) * dt));
        assert!(worst_norm_error <= 1e-12, "norm error {worst_norm_error:e}");
        assert!(
            worst_step_drift <= 16.0 * f64::EPSILON,
            "raw step drift {worst_step_drift:e}"
        );
        let attitude_error = angle_between(q, exact);
        assert!(
            attitude_error < 1e-9,
            "attitude error {attitude_error:e} rad"
        );
    }

    #[test]
    fn renormalize_rejects_degenerate_quaternions() {
        assert!(renormalize(DQuat::from_xyzw(0.0, 0.0, 0.0, 0.0)).is_err());
        assert!(renormalize(DQuat::from_xyzw(f64::NAN, 0.0, 0.0, 1.0)).is_err());
        assert!(renormalize(DQuat::from_xyzw(f64::INFINITY, 0.0, 0.0, 1.0)).is_err());
        let q = renormalize(DQuat::from_xyzw(0.0, 0.0, 3.0, 4.0)).unwrap();
        assert!((q.length() - 1.0).abs() < 1e-15);
    }
}
