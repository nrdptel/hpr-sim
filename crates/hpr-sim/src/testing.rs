//! Test problems with closed-form solutions, shared by the integrator and event tests.

use std::convert::Infallible;
use std::ops::ControlFlow;

use crate::events::Direction;
use crate::integrator::{OdeSystem, Step};

/// Vertical flight under constant gravity with quadratic drag: `h' = v`, `v' = −g − k v|v|`.
#[derive(Debug, Clone)]
pub(crate) struct QuadraticDragFall {
    pub(crate) gravity_mps2: f64,
    /// Drag per unit mass per speed squared, `k = ρ C_D A / (2m)`, in 1/m.
    pub(crate) k_per_m: f64,
}

impl QuadraticDragFall {
    /// A terminal speed of about 70 m/s.
    pub(crate) fn example() -> Self {
        Self {
            gravity_mps2: 9.806_65,
            k_per_m: 0.002,
        }
    }
}

impl OdeSystem<2> for QuadraticDragFall {
    type Error = Infallible;

    fn derivative(&mut self, _t_s: f64, y: &[f64; 2]) -> Result<[f64; 2], Infallible> {
        Ok([y[1], -self.gravity_mps2 - self.k_per_m * y[1] * y[1].abs()])
    }
}

/// The closed-form flight of [`QuadraticDragFall`] launched upward at `v0` from `h = 0`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct QuadraticDragSolution {
    g: f64,
    terminal_m_s: f64,
    phi0: f64,
    pub(crate) apogee_s: f64,
    pub(crate) apogee_m: f64,
}

/// With `v_t = √(g/k)` and `φ₀ = atan(v0/v_t)`:
///
/// - climbing, `v = v_t tan(φ₀ − g t/v_t)` and `h = (v_t²/g) ln(cos(φ₀ − g t/v_t)/cos φ₀)`;
/// - apogee at `t_a = v_t φ₀/g`, `h_a = (v_t²/2g) ln(1 + v0²/v_t²)`;
/// - falling, `v = −v_t tanh(g (t − t_a)/v_t)` and `h = h_a − (v_t²/g) ln cosh(g (t − t_a)/v_t)`.
///
/// Integrating `dv/dt = −g ∓ k v²` by separation of variables gives these.
pub(crate) fn closed_form_quadratic_drag(
    flight: &QuadraticDragFall,
    v0_m_s: f64,
) -> QuadraticDragSolution {
    let g = flight.gravity_mps2;
    let terminal_m_s = (g / flight.k_per_m).sqrt();
    let phi0 = (v0_m_s / terminal_m_s).atan();
    QuadraticDragSolution {
        g,
        terminal_m_s,
        phi0,
        apogee_s: terminal_m_s * phi0 / g,
        apogee_m: terminal_m_s * terminal_m_s / (2.0 * g)
            * (1.0 + (v0_m_s / terminal_m_s).powi(2)).ln(),
    }
}

impl QuadraticDragSolution {
    /// `[h, v]` at time `t`.
    pub(crate) fn state(&self, t_s: f64) -> [f64; 2] {
        let vt = self.terminal_m_s;
        let g = self.g;
        if t_s <= self.apogee_s {
            let angle = self.phi0 - g * t_s / vt;
            [
                vt * vt / g * (angle.cos() / self.phi0.cos()).ln(),
                vt * angle.tan(),
            ]
        } else {
            let u = g * (t_s - self.apogee_s) / vt;
            [self.apogee_m - vt * vt / g * u.cosh().ln(), -vt * u.tanh()]
        }
    }

    /// The time the falling flight passes height `h`.
    pub(crate) fn time_at_descending_height_s(&self, height_m: f64) -> f64 {
        let vt = self.terminal_m_s;
        let u = ((self.apogee_m - height_m) * self.g / (vt * vt))
            .exp()
            .acosh();
        self.apogee_s + vt * u / self.g
    }
}

/// A rocket in vacuum under constant gravity with constant thrust `F` and exhaust velocity `c`
/// until burnout, as `[h, v, m]`.
#[derive(Debug, Clone)]
pub(crate) struct ConstantThrustVacuum {
    pub(crate) gravity_mps2: f64,
    pub(crate) thrust_n: f64,
    pub(crate) exhaust_velocity_m_s: f64,
    pub(crate) burning: bool,
}

impl OdeSystem<3> for ConstantThrustVacuum {
    type Error = Infallible;

    fn derivative(&mut self, _t_s: f64, y: &[f64; 3]) -> Result<[f64; 3], Infallible> {
        if self.burning {
            Ok([
                y[1],
                self.thrust_n / y[2] - self.gravity_mps2,
                -self.thrust_n / self.exhaust_velocity_m_s,
            ])
        } else {
            Ok([y[1], -self.gravity_mps2, 0.0])
        }
    }
}

impl ConstantThrustVacuum {
    /// `[h, v, m]` at `t ≤ t_burnout` from rest at `h = 0` with mass `m0`: with `ṁ = F/c` and
    /// `m = m0 − ṁ t`, `v = c ln(m0/m) − g t` and `h = c (t + (m/ṁ) ln(m/m0)) − g t²/2`
    /// (Tsiolkovsky's equation with gravity loss, integrated once more).
    pub(crate) fn powered_state(&self, m0_kg: f64, t_s: f64) -> [f64; 3] {
        let c = self.exhaust_velocity_m_s;
        let mdot = self.thrust_n / c;
        let m = m0_kg - mdot * t_s;
        let g = self.gravity_mps2;
        [
            c * (t_s + m / mdot * (m / m0_kg).ln()) - 0.5 * g * t_s * t_s,
            c * (m0_kg / m).ln() - g * t_s,
            m,
        ]
    }
}

/// The harmonic oscillator `x'' = −x` as `[x, x']`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Oscillator;

impl OdeSystem<2> for Oscillator {
    type Error = Infallible;

    fn derivative(&mut self, _t_s: f64, y: &[f64; 2]) -> Result<[f64; 2], Infallible> {
        Ok([y[1], -y[0]])
    }
}

/// A system with events `g(index, t, y)` added, and a record of every accepted step's
/// `(start, end)` times with the dense output at requested fractions.
pub(crate) struct WithEvents<S, G> {
    pub(crate) system: S,
    directions: Vec<Direction>,
    g: G,
    /// Every accepted step, as `(start_s, end_s)`.
    pub(crate) steps: Vec<(f64, f64)>,
}

impl<S, G> WithEvents<S, G> {
    pub(crate) fn new(system: S, directions: Vec<Direction>, g: G) -> Self {
        Self {
            system,
            directions,
            g,
            steps: Vec::new(),
        }
    }
}

impl<S, G, const N: usize> OdeSystem<N> for WithEvents<S, G>
where
    S: OdeSystem<N>,
    G: FnMut(usize, f64, &[f64; N]) -> f64,
{
    type Error = S::Error;

    fn derivative(&mut self, t_s: f64, y: &[f64; N]) -> Result<[f64; N], S::Error> {
        self.system.derivative(t_s, y)
    }

    fn absolute_tolerance_weights(&self) -> [f64; N] {
        self.system.absolute_tolerance_weights()
    }

    fn event_count(&self) -> usize {
        self.directions.len()
    }

    fn event_direction(&self, index: usize) -> Direction {
        self.directions[index]
    }

    fn event_value(&mut self, index: usize, t_s: f64, y: &[f64; N]) -> f64 {
        (self.g)(index, t_s, y)
    }

    fn accept_step(&mut self, step: &Step<N>) -> ControlFlow<()> {
        self.steps.push((step.start_s(), step.end_s()));
        ControlFlow::Continue(())
    }
}

/// A system that hands every accepted step to a closure, which may stop the integration.
pub(crate) struct Watched<S, O> {
    pub(crate) system: S,
    observe: O,
}

impl<S, O> Watched<S, O> {
    pub(crate) fn new(system: S, observe: O) -> Self {
        Self { system, observe }
    }
}

impl<S, O, const N: usize> OdeSystem<N> for Watched<S, O>
where
    S: OdeSystem<N>,
    O: FnMut(&Step<N>) -> ControlFlow<()>,
{
    type Error = S::Error;

    fn derivative(&mut self, t_s: f64, y: &[f64; N]) -> Result<[f64; N], S::Error> {
        self.system.derivative(t_s, y)
    }

    fn accept_step(&mut self, step: &Step<N>) -> ControlFlow<()> {
        (self.observe)(step)
    }
}
