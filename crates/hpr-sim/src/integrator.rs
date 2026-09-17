//! Initial-value integrators: the adaptive Dormand–Prince 5(4) pair with dense output, and the
//! classical fixed-step fourth-order Runge–Kutta method.
//!
//! [`Integrator::advance`] takes accepted steps from the current time toward a stop time and
//! returns at the stop time, at the first events on the way, or when the system asks to stop. A
//! stop time is always a step boundary, so a caller puts discontinuities there (burnout, a
//! thrust-curve knot, a phase change) and never lets one fall inside a step (Loft lesson L23). The
//! system declares its events and sees each accepted step, with its dense output, through
//! [`OdeSystem`]'s provided methods.
//!
//! - **Dormand–Prince 5(4)** ([`Method::DormandPrince54`]): the pair of J. R. Dormand and
//!   P. J. Prince, "A family of embedded Runge-Kutta formulae", *J. Comput. Appl. Math.* 6 (1980)
//!   19–26, advancing with the fifth-order solution (local extrapolation). The coefficients, the
//!   error norm, the PI step-size controller, the starting step and the fourth-order continuous
//!   extension follow E. Hairer and G. Wanner's `DOPRI5` (version of 2004, BSD-2-Clause, pinned
//!   as `hairer-dopri5` in `validation/refs.lock.toml`), which implements Hairer, Nørsett and
//!   Wanner, *Solving Ordinary Differential Equations I*, 2nd ed., Springer, 1993, §II.4–II.6,
//!   and §IV.2 of volume II. The port is noted in `THIRD-PARTY-NOTICES.md`.
//! - **RK4** ([`Method::Rk4`]): Kutta's classical method (HNW I, §II.1, table 1.2) with a fixed
//!   step and cubic Hermite dense output from the derivatives at both ends.
//!
//! Method, tests and limits: `docs/physics/integration.md`.

use std::fmt;
use std::ops::ControlFlow;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::events::{Direction, EVENT_TIME_RESOLUTION_S, RootError, find_root};

/// A first-order system `y' = f(t, y)` with `N` components, its events and its step observer.
///
/// Only [`derivative`](Self::derivative) is required. The event methods declare scalar functions
/// `g_i(t, y)` whose crossings stop the integration (none by default), and
/// [`accept_step`](Self::accept_step) sees every accepted step. One type carries all three so
/// that a flight phase can record from, and stop on, its own state.
pub trait OdeSystem<const N: usize> {
    /// The error the derivative can return.
    type Error;

    /// The derivative `f(t, y)`.
    ///
    /// # Errors
    ///
    /// Whatever the system can't evaluate. In an adaptive step the integrator treats an error as a
    /// rejection and retries with a shorter step, because a long step's stages can probe states
    /// off the trajectory; the error is returned once the step can't shrink further, or at once
    /// for the step's first stage and for RK4.
    fn derivative(&mut self, t_s: f64, y: &[f64; N]) -> Result<[f64; N], Self::Error>;

    /// Per-component weights `wᵢ` on the absolute tolerance, so that components in different
    /// units share one [`Adaptive::absolute_tolerance`]: component `i` is held to
    /// `wᵢ·atol + rtol·|yᵢ|`. Every weight must be finite and positive. All ones by default.
    fn absolute_tolerance_weights(&self) -> [f64; N] {
        [1.0; N]
    }

    /// The number of event functions (none by default).
    fn event_count(&self) -> usize {
        0
    }

    /// The direction of event `index`.
    fn event_direction(&self, _index: usize) -> Direction {
        Direction::Either
    }

    /// `g_index(t, y)`. A non-finite value stops the integration with
    /// [`IntegrationError::EventNotFinite`].
    fn event_value(&mut self, _index: usize, _t_s: f64, _y: &[f64; N]) -> f64 {
        f64::NAN
    }

    /// Sees each accepted step, including one shortened to end at an event or a stop time.
    /// Returning `Break` stops the integration at the end of this step with [`Advance::Stopped`]
    /// (unless events fired there, which take precedence).
    fn accept_step(&mut self, _step: &Step<N>) -> ControlFlow<()> {
        ControlFlow::Continue(())
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
    /// The longest step, in seconds, or `None` for no limit. Events that cross and return within
    /// one step go unseen, so this also bounds the event functions' shortest detectable excursion.
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
#[serde(tag = "method", deny_unknown_fields)]
#[non_exhaustive]
pub enum Method {
    /// Adaptive Dormand–Prince 5(4) with error control and fourth-order dense output.
    #[serde(rename = "dopri5")]
    DormandPrince54(Adaptive),
    /// Classical RK4 with a fixed step, shortened only to land on a stop time or an event.
    #[serde(rename = "rk4")]
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

/// Why an integration stopped short. The integrator stays at its last accepted step, and a later
/// [`Integrator::advance`] may resume from there.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum IntegrationError<E> {
    /// The system's derivative failed.
    #[error("the derivative failed at t = {t_s} s")]
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
    /// The state, its derivative or the step stopped being finite and a shorter step didn't help.
    #[error("the solution is not finite after t = {t_s} s")]
    NotFinite {
        /// The start of the failed step.
        t_s: f64,
    },
    /// The integrator's step limit was reached. [`Integrator::set_step_limit`] can raise it.
    #[error("the step limit ({limit}) was reached at t = {t_s} s")]
    StepLimit {
        /// Where the integration stopped.
        t_s: f64,
        /// The limit, counting accepted and rejected steps.
        limit: u64,
    },
    /// An event function returned a non-finite value, or its crossing couldn't be located.
    #[error("event {index} failed at t = {t_s} s")]
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
#[non_exhaustive]
pub enum Advance {
    /// The integrator is at the stop time.
    Reached,
    /// The integrator is at the earliest event crossing; [`Integrator::fired_events`] lists every
    /// event past its zero there.
    Events,
    /// [`OdeSystem::accept_step`] asked to stop; the integrator is at the end of that step.
    Stopped,
}

/// Work counters since the integrator was built.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stats {
    /// Derivative evaluations.
    pub evaluations: u64,
    /// Accepted steps, including steps shortened to end at an event.
    pub accepted_steps: u64,
    /// Rejected steps, including steps whose stages failed.
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

/// `Δ = y₁ − y₀`, `r₂ = h f₀ − Δ` and `r₃ = Δ − h f₁ − r₂`, shared by both dense outputs.
fn hermite_terms<const N: usize>(
    h: f64,
    start: &[f64; N],
    end: &[f64; N],
    f0: &[f64; N],
    f1: &[f64; N],
) -> ([f64; N], [f64; N], [f64; N]) {
    let mut delta = [0.0; N];
    let mut r2 = [0.0; N];
    let mut r3 = [0.0; N];
    for i in 0..N {
        delta[i] = end[i] - start[i];
        r2[i] = h * f0[i] - delta[i];
        r3[i] = delta[i] - h * f1[i] - r2[i];
    }
    (delta, r2, r3)
}

impl<const N: usize> Step<N> {
    fn quartic(
        start_s: f64,
        end_s: f64,
        start: [f64; N],
        end: [f64; N],
        k1: &[f64; N],
        k7: &[f64; N],
        r4: [f64; N],
    ) -> Self {
        let h = end_s - start_s;
        let (delta, r2, r3) = hermite_terms(h, &start, &end, k1, k7);
        Self {
            start_s,
            end_s,
            span_s: h,
            start,
            end,
            dense: Dense::Quartic { delta, r2, r3, r4 },
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
        let (delta, r2, r3) = hermite_terms(h, &start, &end, f0, f1);
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

    /// The dense output at `t`, meant for `t` in `[start_s, end_s]`: the stored states at the two
    /// ends, fourth order inside a Dormand–Prince step and third order inside an RK4 step.
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
                    y[i] = self.start[i]
                        + theta * (delta[i] + theta1 * (r2[i] + theta * (r3[i] + theta1 * r4[i])));
                }
            }
            Dense::Cubic { delta, r2, r3 } => {
                for i in 0..N {
                    y[i] = self.start[i] + theta * (delta[i] + theta1 * (r2[i] + theta * r3[i]));
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
/// The step may shrink by at most a factor of 5 (`1/FAC1`, `FAC1 = 0.2`)...
const MAX_SHRINK: f64 = 5.0;
/// ...and grow by at most 10 (`1/FAC2`, `FAC2 = 10`), as a divisor `0.1`.
const MIN_DIVISOR: f64 = 0.1;
/// The PI controller's `β` (HNW II, §IV.2).
const BETA: f64 = 0.04;
/// The error exponent `1/5 − 0.75 β`.
const EXPONENT: f64 = 0.2 - BETA * 0.75;
/// `DOPRI5`'s rounding unit.
const ROUNDING: f64 = 2.3e-16;

/// Whether a step `h` at `t` is too short to change `t` meaningfully (`DOPRI5`:
/// `0.1|h| ≤ |t|·uround`), with `|t|` floored at one second so that it also applies near `t = 0`.
fn negligible_step(t: f64, h: f64) -> bool {
    0.1 * h <= t.abs().max(1.0) * ROUNDING
}

/// The default step limit.
pub const DEFAULT_STEP_LIMIT: u64 = 1_000_000;

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
    fired: Vec<usize>,
}

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
            fired: Vec::new(),
        };
        integrator.reset(t0_s, y0)?;
        Ok(integrator)
    }

    /// Limits the steps, accepted and rejected, over the integrator's life
    /// ([`DEFAULT_STEP_LIMIT`] otherwise).
    #[must_use]
    pub fn with_step_limit(mut self, steps: u64) -> Self {
        self.step_limit = steps;
        self
    }

    /// Changes the step limit, for example to continue after [`IntegrationError::StepLimit`].
    pub fn set_step_limit(&mut self, steps: u64) {
        self.step_limit = steps;
    }

    /// Moves the integrator to a new time and state, as after an impulse or a projection such as
    /// renormalizing a quaternion. The step-size estimate carries over.
    ///
    /// # Errors
    ///
    /// [`SettingsError::NotFinite`] for a non-finite time or state; the integrator is unchanged.
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
        self.fired.clear();
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

    /// The step the adaptive method will try next, once it has taken one.
    #[must_use]
    pub fn next_step_s(&self) -> Option<f64> {
        (self.next_step_s > 0.0).then_some(self.next_step_s)
    }

    /// The events that fired at the current time, ascending, after [`Advance::Events`]; empty
    /// otherwise.
    #[must_use]
    pub fn fired_events(&self) -> &[usize] {
        &self.fired
    }

    /// Integrates toward `t_stop_s` (which may be infinite), stopping at the first events.
    ///
    /// - The last step is shortened, or stretched by up to 1%, to end exactly at `t_stop_s`. A stop
    ///   within the rounding of `t` counts as reached without a step.
    /// - Events are checked at the end of every accepted step. The earliest crossing is located on
    ///   the step's dense output; the integrator stops there with the dense output's state, just
    ///   past the crossing (within [`EVENT_TIME_RESOLUTION_S`]), and reports every event that is
    ///   past its zero at that state.
    /// - A call that starts on an event it returned doesn't return it again: the event's function
    ///   is already past zero there, and a crossing must start strictly on the near side.
    /// - [`OdeSystem::accept_step`] sees every accepted step and may stop the integration.
    /// - The derivative is evaluated afresh at the start of each call, so a caller can change the
    ///   system (a phase) between calls.
    ///
    /// # Errors
    ///
    /// [`IntegrationError`]: a failed derivative, a non-finite solution or event, a step that
    /// can't be made small enough, the step limit, or a stop time in the past. The integrator is
    /// left at its last accepted step.
    pub fn advance<S>(
        &mut self,
        system: &mut S,
        t_stop_s: f64,
    ) -> Result<Advance, IntegrationError<S::Error>>
    where
        S: OdeSystem<N> + ?Sized,
    {
        self.fired.clear();
        if t_stop_s.is_nan() || t_stop_s < self.t_s {
            return Err(IntegrationError::Backward {
                t_s: self.t_s,
                t_stop_s,
            });
        }
        if negligible_step(self.t_s, t_stop_s - self.t_s) {
            self.t_s = t_stop_s;
            return Ok(Advance::Reached);
        }
        let weights = system.absolute_tolerance_weights();
        for w in weights {
            positive("absolute tolerance weight", w)?;
        }
        self.g_start.clear();
        for index in 0..system.event_count() {
            let g = system.event_value(index, self.t_s, &self.y);
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
                self.advance_dp(system, &adaptive, &weights, t_stop_s)
            }
            Method::Rk4 { step_s } => self.advance_rk4(system, step_s, t_stop_s),
        }
    }

    fn evaluate<S: OdeSystem<N> + ?Sized>(
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

    fn check_step_limit<X>(&self) -> Result<(), IntegrationError<X>> {
        if self.stats.accepted_steps + self.stats.rejected_steps >= self.step_limit {
            Err(IntegrationError::StepLimit {
                t_s: self.t_s,
                limit: self.step_limit,
            })
        } else {
            Ok(())
        }
    }

    /// Stages 2 to 7 and the fifth-order solution of a Dormand–Prince step of `h` from `(t, y)`,
    /// with `k7` evaluated at `t1`.
    fn dp_stages<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        t: f64,
        y: &[f64; N],
        k1: &[f64; N],
        h: f64,
        t1: f64,
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
            t1,
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
        let k7 = self.evaluate(system, t1, &y1)?;
        Ok(DpStages {
            k3,
            k4,
            k5,
            k6,
            k7,
            y1,
        })
    }

    fn advance_dp<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        adaptive: &Adaptive,
        weights: &[f64; N],
        t_stop_s: f64,
    ) -> Result<Advance, IntegrationError<S::Error>> {
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
        // Why the last step was rejected, if not for its error estimate: reported instead of
        // `StepTooSmall` if the step can't shrink further.
        let mut failure: Option<IntegrationError<S::Error>> = None;
        loop {
            self.check_step_limit()?;
            let t = self.t_s;
            let y = self.y;
            if !h.is_finite() || h <= 0.0 {
                return Err(failure.unwrap_or(IntegrationError::NotFinite { t_s: t }));
            }
            let proposed = h;
            // Stretch up to 1% to land on the stop, but not past the longest step (beyond the
            // rounding that equal steps accumulate).
            let last = t + 1.01 * h >= t_stop_s && t_stop_s - t <= max_step * (1.0 + 1e-9);
            if last {
                h = t_stop_s - t;
            }
            if negligible_step(t, h) {
                if last {
                    // The stop is within the rounding of `t`.
                    self.t_s = t_stop_s;
                    return Ok(Advance::Reached);
                }
                return Err(failure.unwrap_or(IntegrationError::StepTooSmall { t_s: t, step_s: h }));
            }
            let t1 = if last { t_stop_s } else { t + h };
            let stages = match self.dp_stages(system, t, &y, &k1, h, t1) {
                Ok(stages) => stages,
                Err(error) => {
                    // A long step's stages can leave the trajectory; retry shorter.
                    self.stats.rejected_steps += 1;
                    failure = Some(error);
                    rejected = true;
                    h /= MAX_SHRINK;
                    continue;
                }
            };

            let mut sum = 0.0;
            for i in 0..N {
                let e = h
                    * (E1 * k1[i]
                        + E3 * stages.k3[i]
                        + E4 * stages.k4[i]
                        + E5 * stages.k5[i]
                        + E6 * stages.k6[i]
                        + E7 * stages.k7[i]);
                sum += (e / scale(i, y[i], stages.y1[i])).powi(2);
            }
            let error = (sum / N.max(1) as f64).sqrt();
            if !error.is_finite() {
                self.stats.rejected_steps += 1;
                failure = Some(IntegrationError::NotFinite { t_s: t });
                rejected = true;
                h /= MAX_SHRINK;
                continue;
            }
            let fac11 = error.powf(EXPONENT);
            if error > 1.0 {
                self.stats.rejected_steps += 1;
                failure = None;
                rejected = true;
                h /= (fac11 / SAFETY).min(MAX_SHRINK);
                continue;
            }

            // Accepted.
            let fac =
                (fac11 / self.previous_error.powf(BETA) / SAFETY).clamp(MIN_DIVISOR, MAX_SHRINK);
            let mut h_new = (h / fac).min(max_step);
            if rejected {
                h_new = h_new.min(h);
            }
            // A step shortened to land on the stop time says little about the next one.
            let carry = if last {
                h_new.max(proposed.min(max_step))
            } else {
                h_new
            };
            let r4 = lin(
                &[0.0; N],
                h,
                &[
                    (D1, &k1),
                    (D3, &stages.k3),
                    (D4, &stages.k4),
                    (D5, &stages.k5),
                    (D6, &stages.k6),
                    (D7, &stages.k7),
                ],
            );
            let step = Step::quartic(t, t1, y, stages.y1, &k1, &stages.k7, r4);
            let located = self.locate_event(system, &step)?;
            self.previous_error = error.max(1e-4);
            self.next_step_s = carry;
            if let Some(outcome) = self.commit(system, step, located)? {
                return Ok(outcome);
            }
            if last {
                return Ok(Advance::Reached);
            }
            k1 = stages.k7;
            rejected = false;
            failure = None;
            h = h_new;
        }
    }

    /// Moves to the end of an accepted step, or to the event located in it, and tells the system.
    /// Returns the outcome if the integration stops here.
    fn commit<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        step: Step<N>,
        event: Option<(usize, f64)>,
    ) -> Result<Option<Advance>, IntegrationError<S::Error>> {
        let Some((first, t_event)) = event else {
            self.stats.accepted_steps += 1;
            self.t_s = step.end_s;
            self.y = step.end;
            std::mem::swap(&mut self.g_start, &mut self.g_end);
            return Ok(system
                .accept_step(&step)
                .is_break()
                .then_some(Advance::Stopped));
        };
        let shortened = step.truncated(t_event);
        self.fired.clear();
        for index in 0..self.g_start.len() {
            let g = system.event_value(index, t_event, &shortened.end);
            if !g.is_finite() {
                self.fired.clear();
                return Err(IntegrationError::EventNotFinite {
                    index,
                    t_s: t_event,
                });
            }
            if system
                .event_direction(index)
                .crosses(self.g_start[index], g)
            {
                self.fired.push(index);
            }
        }
        // The earliest event is past zero at its far-side bracket end by construction; include it
        // even if an event function that isn't a pure function of `(t, y)` says otherwise.
        if let Err(position) = self.fired.binary_search(&first) {
            self.fired.insert(position, first);
        }
        self.stats.accepted_steps += 1;
        self.t_s = t_event;
        self.y = shortened.end;
        // Events take precedence over a request to stop.
        let _ = system.accept_step(&shortened);
        Ok(Some(Advance::Events))
    }

    /// Hairer's starting step (`DOPRI5` function `HINIT`; HNW I, §II.4): a first guess from the
    /// ratio of `‖y₀‖` to `‖f₀‖`, refined with a second-derivative estimate from one Euler step.
    #[expect(
        clippy::too_many_arguments,
        reason = "private helper sharing `advance_dp`'s validated inputs"
    )]
    fn starting_step<S: OdeSystem<N> + ?Sized>(
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

    fn advance_rk4<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        step_s: f64,
        t_stop_s: f64,
    ) -> Result<Advance, IntegrationError<S::Error>> {
        let (t0, y0) = (self.t_s, self.y);
        let mut k1 = self.evaluate(system, t0, &y0)?;
        loop {
            self.check_step_limit()?;
            let t = self.t_s;
            let y = self.y;
            let last = t + 1.01 * step_s >= t_stop_s;
            let (h, t1) = if last {
                (t_stop_s - t, t_stop_s)
            } else {
                (step_s, t + step_s)
            };
            if negligible_step(t, h) {
                if last {
                    self.t_s = t_stop_s;
                    return Ok(Advance::Reached);
                }
                return Err(IntegrationError::StepTooSmall { t_s: t, step_s: h });
            }
            let y1 = self.rk4_step(system, t, &y, &k1, h, t1)?;
            let f1 = self.evaluate(system, t1, &y1)?;
            if y1.iter().chain(&f1).any(|v| !v.is_finite()) {
                return Err(IntegrationError::NotFinite { t_s: t });
            }
            let step = Step::cubic(t, t1, y, y1, &k1, &f1);
            let located = self.locate_event(system, &step)?;
            if let Some(outcome) = self.commit(system, step, located)? {
                return Ok(outcome);
            }
            if last {
                return Ok(Advance::Reached);
            }
            k1 = f1;
        }
    }

    /// One classical RK4 step (HNW I, table 1.2): weights 1/6, 2/6, 2/6, 1/6 at 0, ½, ½, 1.
    fn rk4_step<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        t: f64,
        y: &[f64; N],
        k1: &[f64; N],
        h: f64,
        t1: f64,
    ) -> Result<[f64; N], IntegrationError<S::Error>> {
        let k2 = self.evaluate(system, t + 0.5 * h, &lin(y, h, &[(0.5, k1)]))?;
        let k3 = self.evaluate(system, t + 0.5 * h, &lin(y, h, &[(0.5, &k2)]))?;
        let k4 = self.evaluate(system, t1, &lin(y, h, &[(1.0, &k3)]))?;
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

    /// Evaluates every event at the step's end into `g_end` and returns the earliest crossing
    /// (the lowest index on a tie) and its time, located on the dense output.
    fn locate_event<S: OdeSystem<N> + ?Sized>(
        &mut self,
        system: &mut S,
        step: &Step<N>,
    ) -> Result<Option<(usize, f64)>, IntegrationError<S::Error>> {
        self.g_end.clear();
        let mut earliest: Option<(usize, f64)> = None;
        for index in 0..self.g_start.len() {
            let g1 = system.event_value(index, step.end_s, &step.end);
            if !g1.is_finite() {
                return Err(IntegrationError::EventNotFinite {
                    index,
                    t_s: step.end_s,
                });
            }
            self.g_end.push(g1);
            let g0 = self.g_start[index];
            if !system.event_direction(index).crosses(g0, g1) {
                continue;
            }
            let root = find_root(
                |t| system.event_value(index, t, &step.state_at(t)),
                step.start_s,
                step.end_s,
                g0,
                g1,
                EVENT_TIME_RESOLUTION_S,
            )
            .map_err(|error| IntegrationError::EventNotFinite {
                index,
                t_s: match error {
                    RootError::NotFinite { x } => x,
                    _ => step.end_s,
                },
            })?;
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
    k7: [f64; N],
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
mod tests;
