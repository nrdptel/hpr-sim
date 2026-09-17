//! Initial-value integrators: the adaptive Dormand–Prince 5(4) pair with dense output, and the
//! classical fixed-step fourth-order Runge–Kutta method.
//!
//! [`Integrator::advance`] takes accepted steps from the current time toward a stop time and
//! returns at the stop time or at the first event on the way. A stop time is always a step
//! boundary, so a caller puts discontinuities there (burnout, a thrust-curve knot, a phase change)
//! and never lets one fall inside a step (Loft lesson L23). Each accepted step is handed to an
//! observer as a [`Step`], which carries the step's dense output.
//!
//! - **Dormand–Prince 5(4)** ([`Method::DormandPrince54`]): the pair of J. R. Dormand and
//!   P. J. Prince, "A family of embedded Runge-Kutta formulae", *J. Comput. Appl. Math.* 6 (1980)
//!   19–26, advancing with the fifth-order solution (local extrapolation). The coefficients, the
//!   error norm, the PI step-size controller, the starting step and the fourth-order continuous
//!   extension follow E. Hairer and G. Wanner's `DOPRI5` (version of 2004, BSD-2-Clause, pinned
//!   as `hairer-dopri5` in `validation/refs.lock.toml`), which implements Hairer, Nørsett and
//!   Wanner, *Solving Ordinary Differential Equations I*, 2nd ed., Springer, 1993, §II.4–II.6
//!   and §IV.2 of volume II. The port is noted in `THIRD-PARTY-NOTICES.md`.
//! - **RK4** ([`Method::Rk4`]): Kutta's classical method (HNW I, §II.1, table 1.2) with a fixed
//!   step and cubic Hermite dense output from the derivatives at both ends.
//!
//! Method, tests and limits: `docs/physics/integration.md`.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::events::{EVENT_TIME_RESOLUTION_S, EventSet, find_root};

/// A first-order system `y' = f(t, y)` with `N` components.
pub trait OdeSystem<const N: usize> {
    /// The error the derivative can return.
    type Error;

    /// The derivative `f(t, y)`.
    ///
    /// # Errors
    ///
    /// Whatever the system can't evaluate; the integration stops with
    /// [`IntegrationError::Derivative`].
    fn derivative(&mut self, t_s: f64, y: &[f64; N]) -> Result<[f64; N], Self::Error>;

    /// Per-component weights `wᵢ` on the absolute tolerance, so that components in different
    /// units share one [`Adaptive::absolute_tolerance`]: component `i` is held to
    /// `wᵢ·atol + rtol·|yᵢ|`. Every weight must be finite and positive. All ones by default.
    fn absolute_tolerance_weights(&self) -> [f64; N] {
        [1.0; N]
    }
}

/// Settings for the adaptive Dormand–Prince 5(4) method.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Adaptive {
    /// The relative tolerance `rtol` on each component.
    pub relative_tolerance: f64,
    /// The absolute tolerance `atol`, in the state's units, scaled per component by
    /// [`OdeSystem::absolute_tolerance_weights`].
    pub absolute_tolerance: f64,
    /// The first step, in seconds, or `None` for Hairer's starting-step estimate (HNW I, §II.4).
    pub initial_step_s: Option<f64>,
    /// The longest step, in seconds, or `None` for no limit.
    pub max_step_s: Option<f64>,
}

impl Default for Adaptive {
    /// `rtol = atol = 1e-8`, a free first step and no step limit.
    fn default() -> Self {
        Self {
            relative_tolerance: 1e-8,
            absolute_tolerance: 1e-8,
            initial_step_s: None,
            max_step_s: None,
        }
    }
}

/// The integration method.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Method {
    /// Adaptive Dormand–Prince 5(4) with error control and fourth-order dense output.
    DormandPrince54(Adaptive),
    /// Classical RK4 with a fixed step, shortened only to land on a stop time or an event.
    Rk4 {
        /// The step, in seconds.
        step_s: f64,
    },
}

impl Default for Method {
    /// Dormand–Prince 5(4) with the default [`Adaptive`] settings.
    fn default() -> Self {
        Self::DormandPrince54(Adaptive::default())
    }
}

impl Method {
    /// Checks that tolerances and steps are finite and positive.
    ///
    /// # Errors
    ///
    /// [`SettingsError`] naming the first bad setting.
    pub fn validate(&self) -> Result<(), SettingsError> {
        match self {
            Self::DormandPrince54(adaptive) => {
                positive("relative tolerance", adaptive.relative_tolerance)?;
                positive("absolute tolerance", adaptive.absolute_tolerance)?;
                if let Some(h) = adaptive.initial_step_s {
                    positive("initial step", h)?;
                }
                if let Some(h) = adaptive.max_step_s {
                    positive("maximum step", h)?;
                }
                Ok(())
            }
            Self::Rk4 { step_s } => positive("RK4 step", *step_s),
        }
    }
}

fn positive(what: &'static str, value: f64) -> Result<(), SettingsError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(SettingsError::NotPositive { what, value })
    }
}

/// A setting or initial value the integrator can't use.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[non_exhaustive]
pub enum SettingsError {
    /// A tolerance, step or weight that isn't finite and positive.
    #[error("the {what} must be finite and positive, not {value}")]
    NotPositive {
        /// What the value is.
        what: &'static str,
        /// The value.
        value: f64,
    },
    /// A time or state component that isn't finite.
    #[error("the {what} must be finite, not {value}")]
    NotFinite {
        /// What the value is.
        what: &'static str,
        /// The value.
        value: f64,
    },
}

/// Why an integration stopped short.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IntegrationError<E> {
    /// The system's derivative failed.
    #[error("the derivative failed at t = {t_s} s: {source}")]
    Derivative {
        /// The time of the failed evaluation.
        t_s: f64,
        /// The system's error.
        source: E,
    },
    /// The adaptive step fell below the rounding of `t`: the solution changes faster than the
    /// tolerances allow, or the system is stiff or singular there.
    #[error("the step fell to {step_s} s at t = {t_s} s")]
    StepTooSmall {
        /// Where the integration stopped.
        t_s: f64,
        /// The rejected step.
        step_s: f64,
    },
    /// The state or its derivative stopped being finite and a shorter step didn't help.
    #[error("the solution is not finite after t = {t_s} s")]
    NotFinite {
        /// The start of the failed step.
        t_s: f64,
    },
    /// The integrator's step limit was reached.
    #[error("the step limit ({limit}) was reached at t = {t_s} s")]
    StepLimit {
        /// Where the integration stopped.
        t_s: f64,
        /// The limit, counting accepted and rejected steps.
        limit: u64,
    },
    /// An event function returned a non-finite value.
    #[error("event {index} is not finite at t = {t_s} s")]
    EventNotFinite {
        /// The event's index.
        index: usize,
        /// Where it was evaluated.
        t_s: f64,
    },
    /// A stop time before the current time, or not a number.
    #[error("cannot integrate from t = {t_s} s to {t_stop_s} s")]
    Backward {
        /// The current time.
        t_s: f64,
        /// The requested stop time.
        t_stop_s: f64,
    },
    /// A bad setting or tolerance weight.
    #[error(transparent)]
    Settings(#[from] SettingsError),
}

/// How [`Integrator::advance`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Advance {
    /// The integrator is at the stop time.
    Reached,
    /// The integrator is at the first crossing of event `index`, the earliest of all events (the
    /// lowest index on a tie).
    Event {
        /// The event's index in the [`EventSet`].
        index: usize,
    },
}

/// Work counters since the integrator was built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stats {
    /// Derivative evaluations.
    pub evaluations: u64,
    /// Accepted steps, including steps shortened to end at an event.
    pub accepted_steps: u64,
    /// Rejected steps.
    pub rejected_steps: u64,
}

/// One accepted step and its dense output.
#[derive(Clone, PartialEq)]
pub struct Step<const N: usize> {
    start_s: f64,
    end_s: f64,
    /// The span of the dense-output polynomial: the step as taken, before an event shortened it.
    span_s: f64,
    start: [f64; N],
    end: [f64; N],
    dense: Dense<N>,
}

#[derive(Clone, PartialEq)]
enum Dense<const N: usize> {
    /// Hairer's `CONTD5` form: `y(θ) = y₀ + θ(Δ + (1−θ)(r₂ + θ(r₃ + (1−θ) r₄)))`.
    Quartic {
        delta: [f64; N],
        r2: [f64; N],
        r3: [f64; N],
        r4: [f64; N],
    },
    /// Cubic Hermite in the same nested form, without `r₄`.
    Cubic {
        delta: [f64; N],
        r2: [f64; N],
        r3: [f64; N],
    },
}

impl<const N: usize> fmt::Debug for Step<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("start_s", &self.start_s)
            .field("end_s", &self.end_s)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish_non_exhaustive()
    }
}

impl<const N: usize> Step<N> {
    fn quartic(
        start_s: f64,
        end_s: f64,
        start: [f64; N],
        end: [f64; N],
        k1: &[f64; N],
        k7: &[f64; N],
        dense_increment: [f64; N],
    ) -> Self {
        let h = end_s - start_s;
        let mut delta = [0.0; N];
        let mut r2 = [0.0; N];
        let mut r3 = [0.0; N];
        for i in 0..N {
            delta[i] = end[i] - start[i];
            r2[i] = h * k1[i] - delta[i];
            r3[i] = delta[i] - h * k7[i] - r2[i];
        }
        Self {
            start_s,
            end_s,
            span_s: h,
            start,
            end,
            dense: Dense::Quartic {
                delta,
                r2,
                r3,
                r4: dense_increment,
            },
        }
    }

    fn cubic(
        start_s: f64,
        end_s: f64,
        start: [f64; N],
        end: [f64; N],
        f0: &[f64; N],
        f1: &[f64; N],
    ) -> Self {
        let h = end_s - start_s;
        let mut delta = [0.0; N];
        let mut r2 = [0.0; N];
        let mut r3 = [0.0; N];
        for i in 0..N {
            delta[i] = end[i] - start[i];
            r2[i] = h * f0[i] - delta[i];
            r3[i] = delta[i] - h * f1[i] - r2[i];
        }
        Self {
            start_s,
            end_s,
            span_s: h,
            start,
            end,
            dense: Dense::Cubic { delta, r2, r3 },
        }
    }

    /// The same step ending early at `t_s` in `[start_s, end_s]`, with the dense output's state
    /// there.
    fn truncated(&self, t_s: f64) -> Self {
        let end = self.state_at(t_s);
        Self {
            end_s: t_s,
            end,
            ..self.clone()
        }
    }

    /// The time the step starts, in seconds.
    #[must_use]
    pub fn start_s(&self) -> f64 {
        self.start_s
    }

    /// The time the step ends, in seconds.
    #[must_use]
    pub fn end_s(&self) -> f64 {
        self.end_s
    }

    /// The state at the start.
    #[must_use]
    pub fn start(&self) -> &[f64; N] {
        &self.start
    }

    /// The state at the end.
    #[must_use]
    pub fn end(&self) -> &[f64; N] {
        &self.end
    }

    /// The dense output at `t`, meant for `t` in `[start_s, end_s]`: exactly the end states at
    /// the ends, fourth order inside a Dormand–Prince step and third order inside an RK4 step.
    #[must_use]
    pub fn state_at(&self, t_s: f64) -> [f64; N] {
        if t_s == self.end_s {
            return self.end;
        }
        if self.span_s == 0.0 {
            return self.start;
        }
        let theta = (t_s - self.start_s) / self.span_s;
        let theta1 = 1.0 - theta;
        let mut y = [0.0; N];
        match &self.dense {
            Dense::Quartic { delta, r2, r3, r4 } => {
                for i in 0..N {
                    let delta = delta[i];
                    y[i] = self.start[i]
                        + theta * (delta + theta1 * (r2[i] + theta * (r3[i] + theta1 * r4[i])));
                }
            }
            Dense::Cubic { delta, r2, r3 } => {
                for i in 0..N {
                    let delta = delta[i];
                    y[i] = self.start[i] + theta * (delta + theta1 * (r2[i] + theta * r3[i]));
                }
            }
        }
        y
    }
}

/// The Dormand–Prince 5(4) tableau with Hairer's error and dense-output coefficients
/// (`DOPRI5`, subroutine `CDOPRI`).
mod dp {
    pub const C2: f64 = 1.0 / 5.0;
    pub const C3: f64 = 3.0 / 10.0;
    pub const C4: f64 = 4.0 / 5.0;
    pub const C5: f64 = 8.0 / 9.0;
    pub const A21: f64 = 1.0 / 5.0;
    pub const A31: f64 = 3.0 / 40.0;
    pub const A32: f64 = 9.0 / 40.0;
    pub const A41: f64 = 44.0 / 45.0;
    pub const A42: f64 = -56.0 / 15.0;
    pub const A43: f64 = 32.0 / 9.0;
    pub const A51: f64 = 19372.0 / 6561.0;
    pub const A52: f64 = -25360.0 / 2187.0;
    pub const A53: f64 = 64448.0 / 6561.0;
    pub const A54: f64 = -212.0 / 729.0;
    pub const A61: f64 = 9017.0 / 3168.0;
    pub const A62: f64 = -355.0 / 33.0;
    pub const A63: f64 = 46732.0 / 5247.0;
    pub const A64: f64 = 49.0 / 176.0;
    pub const A65: f64 = -5103.0 / 18656.0;
    /// The fifth-order weights, also the last row (FSAL).
    pub const B1: f64 = 35.0 / 384.0;
    pub const B3: f64 = 500.0 / 1113.0;
    pub const B4: f64 = 125.0 / 192.0;
    pub const B5: f64 = -2187.0 / 6784.0;
    pub const B6: f64 = 11.0 / 84.0;
    /// `b − b̂`, the fifth-order weights minus the embedded fourth-order ones.
    pub const E1: f64 = 71.0 / 57600.0;
    pub const E3: f64 = -71.0 / 16695.0;
    pub const E4: f64 = 71.0 / 1920.0;
    pub const E5: f64 = -17253.0 / 339200.0;
    pub const E6: f64 = 22.0 / 525.0;
    pub const E7: f64 = -1.0 / 40.0;
    /// The continuous extension's last coefficient row.
    pub const D1: f64 = -12715105075.0 / 11282082432.0;
    pub const D3: f64 = 87487479700.0 / 32700410799.0;
    pub const D4: f64 = -10690763975.0 / 1880347072.0;
    pub const D5: f64 = 701980252875.0 / 199316789632.0;
    pub const D6: f64 = -1453857185.0 / 822651844.0;
    pub const D7: f64 = 69997945.0 / 29380423.0;
}

/// Hairer's step-size controller constants (`DOPRI5` defaults).
const SAFETY: f64 = 0.9;
/// The step may shrink by at most 1/0.2 = 5...
const SHRINK_LIMIT: f64 = 1.0 / 0.2;
/// ...and grow by at most 10 per step.
const GROW_LIMIT: f64 = 1.0 / 10.0;
/// The PI controller's `β` (HNW II, §IV.2).
const BETA: f64 = 0.04;
/// The error exponent `1/5 − 0.75 β`.
const EXPONENT: f64 = 0.2 - BETA * 0.75;
/// `DOPRI5`'s rounding unit.
const ROUNDING: f64 = 2.3e-16;

/// A stateful integrator for an `N`-component system.
#[derive(Debug, Clone)]
pub struct Integrator<const N: usize> {
    method: Method,
    t_s: f64,
    y: [f64; N],
    /// The next step to try (adaptive); zero until the first step.
    next_step_s: f64,
    /// The PI controller's previous error.
    previous_error: f64,
    stats: Stats,
    step_limit: u64,
    g_start: Vec<f64>,
    g_end: Vec<f64>,
}

/// The default step limit.
pub const DEFAULT_STEP_LIMIT: u64 = 1_000_000;

impl<const N: usize> Integrator<N> {
    /// An integrator at `(t0, y0)`.
    ///
    /// # Errors
    ///
    /// [`SettingsError`] for a bad method setting or a non-finite `t0` or `y0`.
    pub fn new(method: Method, t0_s: f64, y0: [f64; N]) -> Result<Self, SettingsError> {
        method.validate()?;
        let mut integrator = Self {
            method,
            t_s: 0.0,
            y: [0.0; N],
            next_step_s: 0.0,
            previous_error: 1e-4,
            stats: Stats::default(),
            step_limit: DEFAULT_STEP_LIMIT,
            g_start: Vec::new(),
            g_end: Vec::new(),
        };
        integrator.reset(t0_s, y0)?;
        integrator.next_step_s = 0.0;
        Ok(integrator)
    }

    /// Limits the steps, accepted and rejected, over the integrator's life
    /// ([`DEFAULT_STEP_LIMIT`] otherwise).
    #[must_use]
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.step_limit = steps;
        self
    }

    /// Moves the integrator to a new time and state, as after an impulse or a projection such as
    /// renormalizing a quaternion. The step-size estimate carries over.
    ///
    /// # Errors
    ///
    /// [`SettingsError::NotFinite`] for a non-finite time or state.
    pub fn reset(&mut self, t_s: f64, y: [f64; N]) -> Result<(), SettingsError> {
        if !t_s.is_finite() {
            return Err(SettingsError::NotFinite {
                what: "time",
                value: t_s,
            });
        }
        if let Some(bad) = y.iter().find(|v| !v.is_finite()) {
            return Err(SettingsError::NotFinite {
                what: "state",
                value: *bad,
            });
        }
        self.t_s = t_s;
        self.y = y;
        Ok(())
    }

    /// The method.
    #[must_use]
    pub fn method(&self) -> Method {
        self.method
    }

    /// The current time, in seconds.
    #[must_use]
    pub fn time_s(&self) -> f64 {
        self.t_s
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> &[f64; N] {
        &self.y
    }

    /// Work done so far.
    #[must_use]
    pub fn stats(&self) -> Stats {
        self.stats
    }

    /// Integrates toward `t_stop_s` (which may be infinite), stopping at the first event.
    ///
    /// - The last step is shortened to end exactly at `t_stop_s`.
    /// - Events are checked at the end of every accepted step. The earliest crossing is located
    ///   on the step's dense output; the integrator stops there with the dense output's state, just
    ///   past the crossing (within [`EVENT_TIME_RESOLUTION_S`]).
    /// - A call that starts on an event it returned doesn't return it again: the event's function
    ///   is already past zero there, and a crossing must start strictly on the near side.
    /// - `observer` sees every accepted step, including the shortened last one.
    /// - The derivative is evaluated afresh at the start of each call, so a caller can change the
    ///   system (a phase) between calls.
    ///
    /// # Errors
    ///
    /// [`IntegrationError`]: a failed derivative, a non-finite solution or event, a step that
    /// can't be made small enough, the step limit, or a stop time in the past.
    pub fn advance<S, E, O>(
        &mut self,
        system: &mut S,
        t_stop_s: f64,
        events: &mut E,
        observer: &mut O,
    ) -> Result<Advance, IntegrationError<S::Error>>
    where
        S: OdeSystem<N>,
        E: EventSet<N> + ?Sized,
        O: FnMut(&Step<N>),
    {
        if t_stop_s.is_nan() || t_stop_s < self.t_s {
            return Err(IntegrationError::Backward {
                t_s: self.t_s,
                t_stop_s,
            });
        }
        // A stop closer than the rounding of `t` is already reached.
        if t_stop_s - self.t_s <= 4.0 * ROUNDING * self.t_s.abs().max(1.0) {
            self.t_s = t_stop_s;
            return Ok(Advance::Reached);
        }
        let weights = system.absolute_tolerance_weights();
        for w in weights {
            positive("absolute tolerance weight", w)?;
        }
        self.g_start.clear();
        for index in 0..events.count() {
            let g = events.value(index, self.t_s, &self.y);
            if !g.is_finite() {
                return Err(IntegrationError::EventNotFinite {
                    index,
                    t_s: self.t_s,
                });
            }
            self.g_start.push(g);
        }
        match self.method {
            Method::DormandPrince54(adaptive) => {
                self.advance_dp(system, &adaptive, &weights, t_stop_s, events, observer)
            }
            Method::Rk4 { step_s } => self.advance_rk4(system, step_s, t_stop_s, events, observer),
        }
    }

    fn evaluate<S: OdeSystem<N>>(
        &mut self,
        system: &mut S,
        t_s: f64,
        y: &[f64; N],
    ) -> Result<[f64; N], IntegrationError<S::Error>> {
        self.stats.evaluations += 1;
        system
            .derivative(t_s, y)
            .map_err(|source| IntegrationError::Derivative { t_s, source })
    }

    fn count_step<X>(&self) -> Result<(), IntegrationError<X>> {
        if self.stats.accepted_steps + self.stats.rejected_steps >= self.step_limit {
            Err(IntegrationError::StepLimit {
                t_s: self.t_s,
                limit: self.step_limit,
            })
        } else {
            Ok(())
        }
    }

    /// Stages 2 to 6 and the fifth-order solution of a Dormand–Prince step of `h` from `(t, y)`.
    fn dp_stages<S: OdeSystem<N>>(
        &mut self,
        system: &mut S,
        t: f64,
        y: &[f64; N],
        k1: &[f64; N],
        h: f64,
    ) -> Result<DpStages<N>, IntegrationError<S::Error>> {
        use dp::*;
        let k2 = self.evaluate(system, t + C2 * h, &lin(y, h, &[(A21, k1)]))?;
        let k3 = self.evaluate(system, t + C3 * h, &lin(y, h, &[(A31, k1), (A32, &k2)]))?;
        let k4 = self.evaluate(
            system,
            t + C4 * h,
            &lin(y, h, &[(A41, k1), (A42, &k2), (A43, &k3)]),
        )?;
        let k5 = self.evaluate(
            system,
            t + C5 * h,
            &lin(y, h, &[(A51, k1), (A52, &k2), (A53, &k3), (A54, &k4)]),
        )?;
        let k6 = self.evaluate(
            system,
            t + h,
            &lin(
                y,
                h,
                &[(A61, k1), (A62, &k2), (A63, &k3), (A64, &k4), (A65, &k5)],
            ),
        )?;
        let y1 = lin(
            y,
            h,
            &[(B1, k1), (B3, &k3), (B4, &k4), (B5, &k5), (B6, &k6)],
        );
        Ok(DpStages { k3, k4, k5, k6, y1 })
    }

    fn advance_dp<S, E, O>(
        &mut self,
        system: &mut S,
        adaptive: &Adaptive,
        weights: &[f64; N],
        t_stop_s: f64,
        events: &mut E,
        observer: &mut O,
    ) -> Result<Advance, IntegrationError<S::Error>>
    where
        S: OdeSystem<N>,
        E: EventSet<N> + ?Sized,
        O: FnMut(&Step<N>),
    {
        use dp::*;
        let rtol = adaptive.relative_tolerance;
        let atol = adaptive.absolute_tolerance;
        let max_step = adaptive.max_step_s.unwrap_or(f64::INFINITY);
        let scale = |i: usize, a: f64, b: f64| weights[i] * atol + rtol * a.abs().max(b.abs());

        let (t0, y0) = (self.t_s, self.y);
        let mut k1 = self.evaluate(system, t0, &y0)?;
        let mut h = if self.next_step_s > 0.0 {
            self.next_step_s
        } else if let Some(h0) = adaptive.initial_step_s {
            h0
        } else {
            self.starting_step(system, &k1, weights, rtol, atol, t_stop_s, max_step)?
        };
        h = h.min(max_step);
        let mut rejected = false;
        let mut last_rejection_not_finite = false;
        loop {
            self.count_step()?;
            let t = self.t_s;
            let proposed = h;
            let last = t + 1.01 * h >= t_stop_s;
            if last {
                h = t_stop_s - t;
            }
            if 0.1 * h <= t.abs() * ROUNDING || h <= 0.0 {
                return Err(if last_rejection_not_finite {
                    IntegrationError::NotFinite { t_s: t }
                } else {
                    IntegrationError::StepTooSmall { t_s: t, step_s: h }
                });
            }
            let y = self.y;
            let stages = self.dp_stages(system, t, &y, &k1, h)?;
            let t1 = if last { t_stop_s } else { t + h };
            let k7 = self.evaluate(system, t1, &stages.y1)?;

            let mut sum = 0.0;
            for i in 0..N {
                let e = h
                    * (E1 * k1[i]
                        + E3 * stages.k3[i]
                        + E4 * stages.k4[i]
                        + E5 * stages.k5[i]
                        + E6 * stages.k6[i]
                        + E7 * k7[i]);
                sum += (e / scale(i, y[i], stages.y1[i])).powi(2);
            }
            let error = (sum / N.max(1) as f64).sqrt();

            if !error.is_finite() {
                self.stats.rejected_steps += 1;
                last_rejection_not_finite = true;
                rejected = true;
                h *= 1.0 / SHRINK_LIMIT;
                continue;
            }
            last_rejection_not_finite = false;
            let fac11 = error.powf(EXPONENT);
            let fac =
                (fac11 / self.previous_error.powf(BETA) / SAFETY).clamp(GROW_LIMIT, SHRINK_LIMIT);
            if error > 1.0 {
                self.stats.rejected_steps += 1;
                rejected = true;
                h /= (fac11 / SAFETY).min(SHRINK_LIMIT);
                continue;
            }

            // Accepted.
            self.previous_error = error.max(1e-4);
            let mut h_new = (h / fac).min(max_step);
            if rejected {
                h_new = h_new.min(h);
            }
            rejected = false;
            let dense = lin(
                &[0.0; N],
                h,
                &[
                    (D1, &k1),
                    (D3, &stages.k3),
                    (D4, &stages.k4),
                    (D5, &stages.k5),
                    (D6, &stages.k6),
                    (D7, &k7),
                ],
            );
            let step = Step::quartic(t, t1, y, stages.y1, &k1, &k7, dense);
            self.stats.accepted_steps += 1;

            if let Some((index, t_event)) = self.locate_event(&step, events)? {
                let shortened = step.truncated(t_event);
                self.t_s = t_event;
                self.y = shortened.end;
                self.next_step_s = if last {
                    h_new.max(proposed.min(max_step))
                } else {
                    h_new
                };
                observer(&shortened);
                return Ok(Advance::Event { index });
            }

            self.t_s = t1;
            self.y = stages.y1;
            k1 = k7;
            std::mem::swap(&mut self.g_start, &mut self.g_end);
            observer(&step);
            if last {
                // A step shortened to land on the stop time says little about the next one.
                self.next_step_s = h_new.max(proposed.min(max_step));
                return Ok(Advance::Reached);
            }
            h = h_new;
        }
    }

    /// Hairer's starting step (`DOPRI5` function `HINIT`; HNW I, §II.4): a first guess from the
    /// ratio of `‖y₀‖` to `‖f₀‖`, refined with a second-derivative estimate from one Euler step.
    #[expect(
        clippy::too_many_arguments,
        reason = "private helper sharing `advance_dp`'s validated inputs"
    )]
    fn starting_step<S: OdeSystem<N>>(
        &mut self,
        system: &mut S,
        f0: &[f64; N],
        weights: &[f64; N],
        rtol: f64,
        atol: f64,
        t_stop_s: f64,
        max_step: f64,
    ) -> Result<f64, IntegrationError<S::Error>> {
        let (mut dnf, mut dny) = (0.0, 0.0);
        for i in 0..N {
            let sk = weights[i] * atol + rtol * self.y[i].abs();
            dnf += (f0[i] / sk).powi(2);
            dny += (self.y[i] / sk).powi(2);
        }
        let mut h = if dnf <= 1e-10 || dny <= 1e-10 {
            1e-6
        } else {
            (dny / dnf).sqrt() * 0.01
        };
        h = h.min(max_step).min(t_stop_s - self.t_s);
        let y1 = lin(&self.y, h, &[(1.0, f0)]);
        let f1 = self.evaluate(system, self.t_s + h, &y1)?;
        let mut der2 = 0.0;
        for i in 0..N {
            let sk = weights[i] * atol + rtol * self.y[i].abs();
            der2 += ((f1[i] - f0[i]) / sk).powi(2);
        }
        let der2 = der2.sqrt() / h;
        let der12 = der2.abs().max(dnf.sqrt());
        let h1 = if der12 <= 1e-15 {
            (h * 1e-3).max(1e-6)
        } else {
            (0.01 / der12).powf(0.2)
        };
        let h = (100.0 * h).min(h1).min(max_step);
        if h.is_finite() && h > 0.0 {
            Ok(h)
        } else {
            Err(IntegrationError::NotFinite { t_s: self.t_s })
        }
    }

    fn advance_rk4<S, E, O>(
        &mut self,
        system: &mut S,
        step_s: f64,
        t_stop_s: f64,
        events: &mut E,
        observer: &mut O,
    ) -> Result<Advance, IntegrationError<S::Error>>
    where
        S: OdeSystem<N>,
        E: EventSet<N> + ?Sized,
        O: FnMut(&Step<N>),
    {
        let (t0, y0) = (self.t_s, self.y);
        let mut k1 = self.evaluate(system, t0, &y0)?;
        loop {
            self.count_step()?;
            let t = self.t_s;
            let y = self.y;
            let last = t + 1.01 * step_s >= t_stop_s;
            let (h, t1) = if last {
                (t_stop_s - t, t_stop_s)
            } else {
                (step_s, t + step_s)
            };
            let y1 = self.rk4_step(system, t, &y, &k1, h)?;
            let f1 = self.evaluate(system, t1, &y1)?;
            if y1.iter().chain(&f1).any(|v| !v.is_finite()) {
                return Err(IntegrationError::NotFinite { t_s: t });
            }
            let step = Step::cubic(t, t1, y, y1, &k1, &f1);
            self.stats.accepted_steps += 1;

            if let Some((index, t_event)) = self.locate_event(&step, events)? {
                let shortened = step.truncated(t_event);
                self.t_s = t_event;
                self.y = shortened.end;
                observer(&shortened);
                return Ok(Advance::Event { index });
            }

            self.t_s = t1;
            self.y = y1;
            k1 = f1;
            std::mem::swap(&mut self.g_start, &mut self.g_end);
            observer(&step);
            if last {
                return Ok(Advance::Reached);
            }
        }
    }

    /// One classical RK4 step (HNW I, table 1.2): weights 1/6, 2/6, 2/6, 1/6 at 0, ½, ½, 1.
    fn rk4_step<S: OdeSystem<N>>(
        &mut self,
        system: &mut S,
        t: f64,
        y: &[f64; N],
        k1: &[f64; N],
        h: f64,
    ) -> Result<[f64; N], IntegrationError<S::Error>> {
        let k2 = self.evaluate(system, t + 0.5 * h, &lin(y, h, &[(0.5, k1)]))?;
        let k3 = self.evaluate(system, t + 0.5 * h, &lin(y, h, &[(0.5, &k2)]))?;
        let k4 = self.evaluate(system, t + h, &lin(y, h, &[(1.0, &k3)]))?;
        Ok(lin(
            y,
            h,
            &[
                (1.0 / 6.0, k1),
                (2.0 / 6.0, &k2),
                (2.0 / 6.0, &k3),
                (1.0 / 6.0, &k4),
            ],
        ))
    }

    /// Evaluates every event at the step's end into `g_end` and returns the earliest crossing,
    /// located on the dense output.
    fn locate_event<E, X>(
        &mut self,
        step: &Step<N>,
        events: &mut E,
    ) -> Result<Option<(usize, f64)>, IntegrationError<X>>
    where
        E: EventSet<N> + ?Sized,
    {
        let count = events.count();
        self.g_end.clear();
        let mut earliest: Option<(usize, f64)> = None;
        for index in 0..count {
            let g1 = events.value(index, step.end_s, &step.end);
            if !g1.is_finite() {
                return Err(IntegrationError::EventNotFinite {
                    index,
                    t_s: step.end_s,
                });
            }
            self.g_end.push(g1);
            let g0 = self.g_start.get(index).copied().unwrap_or(0.0);
            if !events.direction(index).crosses(g0, g1) {
                continue;
            }
            let root = find_root(
                |t| events.value(index, t, &step.state_at(t)),
                step.start_s,
                step.end_s,
                g0,
                g1,
                EVENT_TIME_RESOLUTION_S,
            )
            .map_err(|t_s| IntegrationError::EventNotFinite { index, t_s })?;
            if earliest.is_none_or(|(_, t)| root < t) {
                earliest = Some((index, root));
            }
        }
        Ok(earliest)
    }
}

/// Stages kept from a Dormand–Prince step.
struct DpStages<const N: usize> {
    k3: [f64; N],
    k4: [f64; N],
    k5: [f64; N],
    k6: [f64; N],
    y1: [f64; N],
}

/// `y + h Σ aⱼ kⱼ`.
#[inline]
fn lin<const N: usize>(y: &[f64; N], h: f64, terms: &[(f64, &[f64; N])]) -> [f64; N] {
    let mut out = *y;
    for (a, k) in terms {
        let ha = h * a;
        for i in 0..N {
            out[i] += ha * k[i];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::*;
    use crate::events::{Direction, EventList};
    use crate::testing::{ConstantThrustVacuum, QuadraticDragFall, closed_form_quadratic_drag};

    /// `y' = −2 t y²`, `y(0) = 1`: `y = 1/(1 + t²)`. Nonlinear and non-autonomous, so the
    /// order conditions of every tree matter.
    struct Rational;

    impl OdeSystem<1> for Rational {
        type Error = Infallible;
        fn derivative(&mut self, t: f64, y: &[f64; 1]) -> Result<[f64; 1], Infallible> {
            Ok([-2.0 * t * y[0] * y[0]])
        }
    }

    fn rational(t: f64) -> f64 {
        1.0 / (1.0 + t * t)
    }

    /// The tableau as matrices, for the order conditions.
    fn tableau() -> ([[f64; 7]; 7], [f64; 7], [f64; 7], [f64; 7]) {
        use dp::*;
        let a = [
            [0.0; 7],
            [A21, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [A31, A32, 0.0, 0.0, 0.0, 0.0, 0.0],
            [A41, A42, A43, 0.0, 0.0, 0.0, 0.0],
            [A51, A52, A53, A54, 0.0, 0.0, 0.0],
            [A61, A62, A63, A64, A65, 0.0, 0.0],
            [B1, 0.0, B3, B4, B5, B6, 0.0],
        ];
        let c = [0.0, C2, C3, C4, C5, 1.0, 1.0];
        let b = [B1, 0.0, B3, B4, B5, B6, 0.0];
        let e = [E1, 0.0, E3, E4, E5, E6, E7];
        let b_hat: Vec<f64> = b.iter().zip(e).map(|(b, e)| b - e).collect();
        (a, c, b, b_hat.try_into().unwrap())
    }

    fn dot(u: &[f64; 7], v: &[f64; 7]) -> f64 {
        u.iter().zip(v).map(|(a, b)| a * b).sum()
    }

    fn mul(a: &[[f64; 7]; 7], v: &[f64; 7]) -> [f64; 7] {
        std::array::from_fn(|i| dot(&a[i], v))
    }

    fn hadamard(u: &[f64; 7], v: &[f64; 7]) -> [f64; 7] {
        std::array::from_fn(|i| u[i] * v[i])
    }

    /// The 17 order conditions for trees of order ≤ 5 (HNW I, §II.2, table 2.2), as
    /// `(order, value, expected)`.
    fn order_conditions(weights: &[f64; 7]) -> Vec<(u32, f64, f64)> {
        let (a, c, _, _) = tableau();
        let one = [1.0; 7];
        let c2 = hadamard(&c, &c);
        let c3 = hadamard(&c2, &c);
        let c4 = hadamard(&c3, &c);
        let ac = mul(&a, &c);
        let ac2 = mul(&a, &c2);
        let ac3 = mul(&a, &c3);
        let aac = mul(&a, &ac);
        let aac2 = mul(&a, &ac2);
        let aaac = mul(&a, &aac);
        let b = weights;
        vec![
            (1, dot(b, &one), 1.0),
            (2, dot(b, &c), 1.0 / 2.0),
            (3, dot(b, &c2), 1.0 / 3.0),
            (3, dot(b, &ac), 1.0 / 6.0),
            (4, dot(b, &c3), 1.0 / 4.0),
            (4, dot(b, &hadamard(&c, &ac)), 1.0 / 8.0),
            (4, dot(b, &ac2), 1.0 / 12.0),
            (4, dot(b, &aac), 1.0 / 24.0),
            (5, dot(b, &c4), 1.0 / 5.0),
            (5, dot(b, &hadamard(&c2, &ac)), 1.0 / 10.0),
            (5, dot(b, &hadamard(&c, &ac2)), 1.0 / 15.0),
            (5, dot(b, &hadamard(&c, &aac)), 1.0 / 30.0),
            (5, dot(b, &hadamard(&ac, &ac)), 1.0 / 20.0),
            (5, dot(b, &ac3), 1.0 / 20.0),
            (5, dot(b, &mul(&a, &hadamard(&c, &ac))), 1.0 / 40.0),
            (5, dot(b, &aac2), 1.0 / 60.0),
            (5, dot(b, &aaac), 1.0 / 120.0),
        ]
    }

    #[test]
    fn tableau_satisfies_the_order_conditions() {
        let (a, c, b, b_hat) = tableau();
        for (row, ci) in a.iter().zip(c) {
            assert!((row.iter().sum::<f64>() - ci).abs() < 1e-14);
        }
        for (order, value, expected) in order_conditions(&b) {
            assert!(
                (value - expected).abs() < 1e-14,
                "b, order {order}: {value}"
            );
        }
        let embedded = order_conditions(&b_hat);
        for (order, value, expected) in &embedded {
            if *order <= 4 {
                assert!(
                    (value - expected).abs() < 1e-14,
                    "b̂, order {order}: {value}"
                );
            }
        }
        assert!(
            embedded
                .iter()
                .any(|(order, value, expected)| *order == 5 && (value - expected).abs() > 1e-4),
            "the embedded solution must be of order 4 exactly"
        );
    }

    /// Observed orders `log₂(e(h)/e(h/2))` from errors at halving steps.
    fn observed_orders(errors: &[f64]) -> Vec<f64> {
        errors.windows(2).map(|w| (w[0] / w[1]).log2()).collect()
    }

    #[test]
    fn fixed_step_dormand_prince_converges_at_fifth_order() {
        // Take n fixed steps of the fifth-order solution with the tableau itself.
        let end = 2.0;
        let mut errors = Vec::new();
        let mut dense_errors = Vec::new();
        for n in [10, 20, 40, 80, 160, 320] {
            let h = end / f64::from(n);
            let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
            let mut dense_error: f64 = 0.0;
            for _ in 0..n {
                let (t, y) = (integrator.t_s, integrator.y);
                let k1 = integrator.evaluate(&mut Rational, t, &y).unwrap();
                let stages = integrator.dp_stages(&mut Rational, t, &y, &k1, h).unwrap();
                let k7 = integrator
                    .evaluate(&mut Rational, t + h, &stages.y1)
                    .unwrap();
                let dense = lin(
                    &[0.0; 1],
                    h,
                    &[
                        (dp::D1, &k1),
                        (dp::D3, &stages.k3),
                        (dp::D4, &stages.k4),
                        (dp::D5, &stages.k5),
                        (dp::D6, &stages.k6),
                        (dp::D7, &k7),
                    ],
                );
                let step = Step::quartic(t, t + h, y, stages.y1, &k1, &k7, dense);
                for theta in [0.25, 0.5, 0.75] {
                    let ti = t + theta * h;
                    dense_error = dense_error.max((step.state_at(ti)[0] - rational(ti)).abs());
                }
                integrator.t_s = t + h;
                integrator.y = stages.y1;
            }
            errors.push((integrator.y[0] - rational(end)).abs());
            dense_errors.push(dense_error);
        }
        // Measured: 5.88, 5.53, 5.31, 5.16, 5.09, approaching 5 from above as h shrinks.
        let orders = observed_orders(&errors);
        assert!(
            orders.windows(2).all(|w| w[1] < w[0]),
            "{errors:?} → {orders:?}"
        );
        assert!(orders.iter().all(|p| *p >= 4.9), "{errors:?} → {orders:?}");
        assert!(
            (4.9..=5.2).contains(&orders[orders.len() - 1]),
            "{orders:?}"
        );
        // The continuous extension has local order 4: its error, O(h⁵) inside each step, is not
        // carried forward, and the nodes are fifth order, so the interpolated error also falls as
        // h⁵ (measured 4.81, 4.97, 4.99, 4.99, 5.00).
        let dense_orders = observed_orders(&dense_errors);
        for order in &dense_orders[1..] {
            assert!(
                (4.9..=5.1).contains(order),
                "{dense_errors:?} → {dense_orders:?}"
            );
        }
    }

    #[test]
    fn rk4_converges_at_fourth_order() {
        let end = 2.0;
        let mut errors = Vec::new();
        let mut dense_errors = Vec::new();
        for n in [20, 40, 80, 160] {
            let step_s = end / f64::from(n);
            let mut integrator = Integrator::new(Method::Rk4 { step_s }, 0.0, [1.0]).unwrap();
            let mut dense_error: f64 = 0.0;
            let mut steps = 0;
            let outcome = integrator
                .advance(&mut Rational, end, &mut (), &mut |step: &Step<1>| {
                    steps += 1;
                    let t = 0.5 * (step.start_s() + step.end_s());
                    dense_error = dense_error.max((step.state_at(t)[0] - rational(t)).abs());
                })
                .unwrap();
            assert_eq!(outcome, Advance::Reached);
            assert_eq!(steps, n);
            assert_eq!(integrator.time_s(), end);
            errors.push((integrator.state()[0] - rational(end)).abs());
            dense_errors.push(dense_error);
        }
        let orders = observed_orders(&errors);
        for order in &orders {
            assert!((3.8..=4.3).contains(order), "{errors:?} → {orders:?}");
        }
        // Cubic Hermite interpolation: local error O(h⁴).
        let dense_orders = observed_orders(&dense_errors);
        for order in &dense_orders {
            assert!(*order >= 3.8, "{dense_errors:?} → {dense_orders:?}");
        }
    }

    #[test]
    fn adaptive_error_is_proportional_to_the_tolerance() {
        // With local extrapolation the global error of DOPRI5 scales about linearly with the
        // tolerance (HNW I, §II.4, "tolerance proportionality").
        let mut errors = Vec::new();
        for k in 0..6 {
            let tol = 1e-4 * 10f64.powi(-k);
            let method = Method::DormandPrince54(Adaptive {
                relative_tolerance: tol,
                absolute_tolerance: tol,
                ..Adaptive::default()
            });
            let mut integrator = Integrator::new(method, 0.0, [1.0]).unwrap();
            integrator
                .advance(&mut Rational, 10.0, &mut (), &mut |_: &Step<1>| {})
                .unwrap();
            let error = (integrator.state()[0] - rational(10.0)).abs();
            assert!(error < 20.0 * tol, "tol {tol}: error {error}");
            errors.push(error);
        }
        assert!(errors[5] < 1e-3 * errors[0], "{errors:?}");
    }

    #[test]
    fn apogee_converges_under_tolerance_halving() {
        // Loft lesson L21: RK4 with no error control and no convergence check on apogee. Here the
        // apogee of a vertical flight with quadratic drag is found as an event at tolerances
        // halving from 1e-5 to 2e-8; its time and height converge to the closed form, and the
        // error stays within a small multiple of the tolerance throughout.
        let flight = QuadraticDragFall::example();
        let truth = closed_form_quadratic_drag(&flight, 150.0);
        let mut errors = Vec::new();
        for k in 0..10 {
            let tol = 1e-5 / 2f64.powi(k);
            let method = Method::DormandPrince54(Adaptive {
                relative_tolerance: tol,
                absolute_tolerance: tol,
                ..Adaptive::default()
            });
            let mut integrator = Integrator::new(method, 0.0, [0.0, 150.0]).unwrap();
            let mut events = EventList::new(
                vec![Direction::Falling],
                |_: usize, _t: f64, y: &[f64; 2]| y[1],
            );
            let outcome = integrator
                .advance(
                    &mut flight.clone(),
                    100.0,
                    &mut events,
                    &mut |_: &Step<2>| {},
                )
                .unwrap();
            assert_eq!(outcome, Advance::Event { index: 0 });
            let dt = (integrator.time_s() - truth.apogee_s).abs();
            let dh = (integrator.state()[0] - truth.apogee_m).abs();
            assert!(
                dt < 1e3 * tol && dh < 1e4 * tol,
                "tol {tol}: dt {dt}, dh {dh}"
            );
            errors.push((tol, dt, dh));
        }
        let (_, dt_last, dh_last) = errors[errors.len() - 1];
        let (_, dt_first, dh_first) = errors[0];
        assert!(dt_last < 1e-6 && dh_last < 1e-5, "{errors:?}");
        assert!(
            dt_last < dt_first / 50.0 && dh_last < dh_first / 50.0,
            "{errors:?}"
        );
    }

    #[test]
    fn constant_thrust_vacuum_matches_closed_form() {
        // Loft lesson L23: burnout fell inside a step and the vacuum case was only checked to ±2%.
        // Burnout is a stop time here, so the discontinuity sits on a step boundary, and the
        // flight matches Tsiolkovsky's closed form to about 1e-10 relative.
        let mut rocket = ConstantThrustVacuum {
            gravity_mps2: 9.806_65,
            thrust_n: 2000.0,
            exhaust_velocity_m_s: 2000.0,
            burning: true,
        };
        let m0 = 20.0;
        let burnout_s = 8.0;
        for method in [Method::default(), Method::Rk4 { step_s: 0.01 }] {
            rocket.burning = true;
            let mut integrator = Integrator::new(method, 0.0, [0.0, 0.0, m0]).unwrap();
            let outcome = integrator
                .advance(&mut rocket, burnout_s, &mut (), &mut |_: &Step<3>| {})
                .unwrap();
            assert_eq!(outcome, Advance::Reached);
            assert_eq!(integrator.time_s(), burnout_s);
            let expected = rocket.powered_state(m0, burnout_s);
            for (got, want) in integrator.state().iter().zip(expected) {
                assert!(
                    (got - want).abs() <= 1e-9 * want.abs(),
                    "{method:?}: {got} vs {want}"
                );
            }

            rocket.burning = false;
            let mut events = EventList::new(
                vec![Direction::Falling],
                |_: usize, _t: f64, y: &[f64; 3]| y[1],
            );
            let outcome = integrator
                .advance(&mut rocket, 1000.0, &mut events, &mut |_: &Step<3>| {})
                .unwrap();
            assert_eq!(outcome, Advance::Event { index: 0 });
            let [h_b, v_b, _] = expected;
            let g = rocket.gravity_mps2;
            let apogee_s = burnout_s + v_b / g;
            let apogee_m = h_b + v_b * v_b / (2.0 * g);
            assert!((integrator.time_s() - apogee_s).abs() < 1e-7, "{method:?}");
            assert!(
                (integrator.state()[0] - apogee_m).abs() < 1e-9 * apogee_m,
                "{method:?}: {} vs {apogee_m}",
                integrator.state()[0]
            );
        }
    }

    #[test]
    fn a_discontinuity_inside_a_step_costs_accuracy_that_a_stop_time_keeps() {
        // The same flight with burnout switched by time inside the derivative: fixed-step RK4
        // drops to first order across the jump in thrust. With burnout as a stop time and the
        // phase set by the caller, each step sees one side of the jump only. (Deciding the phase
        // from `t` alone would not do: the last stage of the step ending at burnout is evaluated at
        // burnout and belongs to the burning side.)
        struct Switching {
            rocket: ConstantThrustVacuum,
            phase: Option<bool>,
        }
        impl OdeSystem<3> for Switching {
            type Error = Infallible;
            fn derivative(&mut self, t: f64, y: &[f64; 3]) -> Result<[f64; 3], Infallible> {
                self.rocket.burning = self.phase.unwrap_or(t < 8.005);
                self.rocket.derivative(t, y)
            }
        }
        let rocket = ConstantThrustVacuum {
            gravity_mps2: 9.806_65,
            thrust_n: 2000.0,
            exhaust_velocity_m_s: 2000.0,
            burning: true,
        };
        let m0 = 20.0;
        let exact_v = |t: f64| {
            let [_, v_b, _] = rocket.powered_state(m0, 8.005);
            v_b - rocket.gravity_mps2 * (t - 8.005)
        };
        let run = |stop_at_burnout: bool| {
            let mut integrator =
                Integrator::new(Method::Rk4 { step_s: 0.01 }, 0.0, [0.0, 0.0, m0]).unwrap();
            let mut system = Switching {
                rocket: rocket.clone(),
                phase: None,
            };
            if stop_at_burnout {
                system.phase = Some(true);
                integrator
                    .advance(&mut system, 8.005, &mut (), &mut |_: &Step<3>| {})
                    .unwrap();
                system.phase = Some(false);
            }
            integrator
                .advance(&mut system, 10.0, &mut (), &mut |_: &Step<3>| {})
                .unwrap();
            (integrator.state()[1] - exact_v(10.0)).abs()
        };
        let (inside, on_boundary) = (run(false), run(true));
        assert!(on_boundary < 1e-9, "{on_boundary}");
        // Measured: 0.56 m/s through the jump, 1e-12 m/s with the stop time.
        assert!(inside > 0.1, "{inside}");
    }

    #[test]
    fn stop_times_are_exact_and_repeated_runs_are_bit_identical() {
        let run = || {
            let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
            let mut ends = Vec::new();
            for stop in [0.1, 0.3, 1.7, 5.0] {
                integrator
                    .advance(&mut Rational, stop, &mut (), &mut |s: &Step<1>| {
                        ends.push(s.end_s().to_bits());
                    })
                    .unwrap();
                assert_eq!(integrator.time_s(), stop);
            }
            (integrator.state()[0].to_bits(), ends, integrator.stats())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn steps_join_up_and_the_dense_output_meets_the_ends() {
        let mut previous_end = 0.0;
        let mut count = 0;
        let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
        integrator
            .advance(&mut Rational, 3.0, &mut (), &mut |step: &Step<1>| {
                assert_eq!(step.start_s(), previous_end);
                assert_eq!(step.state_at(step.start_s()), *step.start());
                assert_eq!(step.state_at(step.end_s()), *step.end());
                previous_end = step.end_s();
                count += 1;
            })
            .unwrap();
        assert_eq!(previous_end, 3.0);
        let stats = integrator.stats();
        assert_eq!(stats.accepted_steps, count);
        // First-same-as-last: six new evaluations per step, plus the start and the first guess.
        assert_eq!(
            stats.evaluations,
            6 * (stats.accepted_steps + stats.rejected_steps) + 2
        );
    }

    #[derive(Debug, PartialEq)]
    struct Refused;

    #[test]
    fn failures_are_reported_as_errors() {
        // A derivative that fails past t = 1.
        struct Failing;
        impl OdeSystem<1> for Failing {
            type Error = Refused;
            fn derivative(&mut self, t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Refused> {
                if t > 1.0 { Err(Refused) } else { Ok([1.0]) }
            }
        }
        let mut integrator = Integrator::new(Method::default(), 0.0, [0.0]).unwrap();
        let error = integrator
            .advance(&mut Failing, 2.0, &mut (), &mut |_: &Step<1>| {})
            .unwrap_err();
        assert!(
            matches!(error, IntegrationError::Derivative { source: Refused, t_s } if t_s > 1.0)
        );

        // A blow-up: y' = y², y(0) = 1 has y = 1/(1 − t).
        struct BlowUp;
        impl OdeSystem<1> for BlowUp {
            type Error = Infallible;
            fn derivative(&mut self, _t: f64, y: &[f64; 1]) -> Result<[f64; 1], Infallible> {
                Ok([y[0] * y[0]])
            }
        }
        let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
        let error = integrator
            .advance(&mut BlowUp, 2.0, &mut (), &mut |_: &Step<1>| {})
            .unwrap_err();
        assert!(
            matches!(
                error,
                IntegrationError::StepTooSmall { t_s, .. } | IntegrationError::NotFinite { t_s }
                    if (t_s - 1.0).abs() < 1e-3
            ),
            "{error:?}"
        );

        // The step limit.
        let mut integrator = Integrator::new(Method::Rk4 { step_s: 0.01 }, 0.0, [1.0])
            .unwrap()
            .with_step_limit(50);
        let error = integrator
            .advance(&mut Rational, 2.0, &mut (), &mut |_: &Step<1>| {})
            .unwrap_err();
        assert_eq!(
            error,
            IntegrationError::StepLimit {
                t_s: integrator.time_s(),
                limit: 50
            }
        );
        assert!((integrator.time_s() - 0.5).abs() < 1e-12);

        // Bad settings, a backward stop and bad weights.
        assert!(Integrator::new(Method::Rk4 { step_s: 0.0 }, 0.0, [1.0]).is_err());
        assert!(Integrator::new(Method::default(), f64::NAN, [1.0]).is_err());
        assert!(Integrator::new(Method::default(), 0.0, [f64::INFINITY]).is_err());
        let error = integrator
            .advance(&mut Rational, 0.0, &mut (), &mut |_: &Step<1>| {})
            .unwrap_err();
        assert!(matches!(error, IntegrationError::Backward { .. }));
        struct Unweighted;
        impl OdeSystem<1> for Unweighted {
            type Error = Infallible;
            fn derivative(&mut self, _t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Infallible> {
                Ok([0.0])
            }
            fn absolute_tolerance_weights(&self) -> [f64; 1] {
                [0.0]
            }
        }
        let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
        let error = integrator
            .advance(&mut Unweighted, 1.0, &mut (), &mut |_: &Step<1>| {})
            .unwrap_err();
        assert!(matches!(error, IntegrationError::Settings(_)));
    }

    #[test]
    fn methods_round_trip_through_json() {
        for method in [
            Method::default(),
            Method::DormandPrince54(Adaptive {
                max_step_s: Some(0.5),
                ..Adaptive::default()
            }),
            Method::Rk4 { step_s: 0.01 },
        ] {
            let json = serde_json::to_string(&method).unwrap();
            assert_eq!(
                serde_json::from_str::<Method>(&json).unwrap(),
                method,
                "{json}"
            );
        }
        let json = r#"{"method":"dormand_prince54","relative_tolerance":1e-6}"#;
        let Method::DormandPrince54(adaptive) = serde_json::from_str(json).unwrap() else {
            panic!("{json}");
        };
        assert_eq!(adaptive.relative_tolerance, 1e-6);
        assert_eq!(adaptive.absolute_tolerance, 1e-8);
    }
}
