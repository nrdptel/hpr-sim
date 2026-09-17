//! Event directions and their location: zero crossings of `g(t, y)` found on an integrator's dense
//! output and polished with Brent's method.
//!
//! An event is a sign change of a scalar function `g(t, y)` across an accepted step, in the
//! direction the event asks for. A system declares its events through
//! [`crate::integrator::OdeSystem::event_count`] and its sibling methods.
//! [`crate::integrator::Integrator::advance`] evaluates every event function at the end of each
//! accepted step and locates the earliest crossing on the step's dense output with [`find_root`]
//! to [`EVENT_TIME_RESOLUTION_S`]. The integration stops at the end of the final bracket on the far
//! side of the crossing, with the dense output's state there. Every event past its zero at that
//! state is reported together, so coincident events are never lost, and none of them is reported
//! again when the integration resumes.
//!
//! Crossings are seen only as sign changes between step ends: a function that crosses zero and
//! comes back inside one step goes unseen. Bound the step (`Adaptive::max_step_s`) when that
//! matters.
//!
//! Method: `docs/physics/integration.md`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The time resolution of event location, in seconds. Roots are polished to within this (plus
/// four units of rounding in `t`) of the dense output's zero; the accuracy of the event itself is
/// the integration's.
pub const EVENT_TIME_RESOLUTION_S: f64 = 1e-12;

/// Which sign changes of `g` count as an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// `g` goes from negative to zero or positive.
    Rising,
    /// `g` goes from positive to zero or negative.
    Falling,
    /// Either of the two.
    Either,
}

impl Direction {
    /// Whether `g` moving from `g0` to `g1` is a crossing in this direction.
    ///
    /// The start must be strictly on one side: a `g0` of exactly zero is an event that has already
    /// happened, so a step that starts on the root doesn't report it again.
    #[must_use]
    pub fn crosses(self, g0: f64, g1: f64) -> bool {
        let rising = g0 < 0.0 && g1 >= 0.0;
        let falling = g0 > 0.0 && g1 <= 0.0;
        match self {
            Self::Rising => rising,
            Self::Falling => falling,
            Self::Either => rising || falling,
        }
    }
}

/// Why [`find_root`] found no root.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
#[non_exhaustive]
pub enum RootError {
    /// The function or an end is not finite at `x`.
    #[error("the function is not finite at {x}")]
    NotFinite {
        /// Where.
        x: f64,
    },
    /// The ends have the same strict sign.
    #[error("the ends don't bracket a root")]
    NotBracketed,
    /// The tolerance is negative or not a number.
    #[error("the tolerance must not be negative or NaN, not {tolerance}")]
    Tolerance {
        /// The tolerance.
        tolerance: f64,
    },
    /// The bracket didn't shrink to the tolerance within the iteration limit.
    #[error("no convergence within the iteration limit")]
    NotConverged,
}

/// A zero of `f` in `[a, b]` by Brent's method, given `f(a)` and `f(b)` of opposite signs (or one
/// of them zero).
///
/// Brent's algorithm (R. P. Brent, *Algorithms for Minimization without Derivatives*,
/// Prentice-Hall, 1973, ch. 4) combines bisection, the secant rule and inverse quadratic
/// interpolation. It keeps a bracket `[b, c]` with `|f(b)| ≤ |f(c)|` and stops when
/// `|c − b|/2 ≤ 2ε|b| + tolerance/2`. It converges superlinearly on smooth functions and falls
/// back on bisection otherwise.
///
/// The result is the end of the final bracket on `b`'s side of the root: an evaluated point whose
/// sign matches `f(b)`, or a point where `f` is zero, within about twice the tolerance of the zero.
/// Callers that stop at an event use it to stand past the crossing.
///
/// # Errors
///
/// [`RootError`]: a non-finite value, ends that don't bracket, a bad tolerance, or no convergence
/// in 500 iterations.
pub fn find_root(
    mut f: impl FnMut(f64) -> f64,
    a: f64,
    b: f64,
    fa: f64,
    fb: f64,
    tolerance: f64,
) -> Result<f64, RootError> {
    if tolerance.is_nan() || tolerance < 0.0 {
        return Err(RootError::Tolerance { tolerance });
    }
    for (x, fx) in [(a, fa), (b, fb)] {
        if !x.is_finite() || !fx.is_finite() {
            return Err(RootError::NotFinite { x });
        }
    }
    if fa == 0.0 {
        return Ok(a);
    }
    if fb == 0.0 {
        return Ok(b);
    }
    if (fa > 0.0) == (fb > 0.0) {
        return Err(RootError::NotBracketed);
    }
    let (mut a, mut b, mut fa, mut fb) = (a, b, fa, fb);
    let far_side_positive = fb > 0.0;
    // The bracket end on the far side: `b` or `c`, whichever has `f(b)`'s original sign. The
    // bracket always holds one end of each sign, so this is never on the near side.
    let far = |b: f64, fb: f64, c: f64| {
        if fb == 0.0 || (fb > 0.0) == far_side_positive {
            b
        } else {
            c
        }
    };
    let (mut c, mut fc) = (a, fa);
    let mut d = b - a;
    let mut e = d;
    // Brent's safeguards bisect whenever interpolation fails to shrink the bracket enough, so the
    // iterations are bounded by a small multiple of bisection's (at most about 2100 halvings for
    // any f64 interval, about 50 for an integration step).
    for _ in 0..500 {
        if (fb > 0.0) == (fc > 0.0) {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol = 2.0 * f64::EPSILON * b.abs() + 0.5 * tolerance;
        let m = 0.5 * (c - b);
        if m.abs() <= tol || fb == 0.0 {
            return Ok(far(b, fb, c));
        }
        if e.abs() >= tol && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if a == c {
                // Secant step.
                p = 2.0 * m * s;
                q = 1.0 - s;
            } else {
                // Inverse quadratic interpolation.
                let qa = fa / fc;
                let r = fb / fc;
                p = s * (2.0 * m * qa * (qa - r) - (b - a) * (r - 1.0));
                q = (qa - 1.0) * (r - 1.0) * (s - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if 2.0 * p < (3.0 * m * q - (tol * q).abs()).min((e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = m;
                e = m;
            }
        } else {
            d = m;
            e = m;
        }
        a = b;
        fa = fb;
        b += if d.abs() > tol { d } else { tol.copysign(m) };
        fb = f(b);
        if !fb.is_finite() {
            return Err(RootError::NotFinite { x: b });
        }
    }
    Err(RootError::NotConverged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrator::{Advance, Integrator, Method, OdeSystem};
    use crate::testing::{Oscillator, QuadraticDragFall, WithEvents, closed_form_quadratic_drag};

    #[test]
    fn directions_need_a_strict_start_side() {
        assert!(Direction::Rising.crosses(-1.0, 0.0));
        assert!(Direction::Rising.crosses(-1.0, 2.0));
        assert!(!Direction::Rising.crosses(0.0, 1.0));
        assert!(!Direction::Rising.crosses(1.0, -1.0));
        assert!(Direction::Falling.crosses(1.0, 0.0));
        assert!(!Direction::Falling.crosses(0.0, -1.0));
        assert!(Direction::Either.crosses(1.0, -1.0));
        assert!(Direction::Either.crosses(-1.0, 1.0));
        assert!(!Direction::Either.crosses(1.0, 1.0));
        assert!(!Direction::Either.crosses(f64::NAN, 1.0));
    }

    #[test]
    fn brent_finds_smooth_flat_and_discontinuous_roots() {
        // Smooth: cos x = x at 0.739085133215160641655312087673873404...
        let x = find_root(|x| x.cos() - x, 0.0, 1.0, 1.0, 1.0_f64.cos() - 1.0, 1e-15).unwrap();
        assert!((x - 0.739_085_133_215_160_6).abs() < 1e-14, "{x}");

        // A triple root, where interpolation stalls and bisection has to carry it.
        let cube = |x: f64| (x - 0.3).powi(3);
        let x = find_root(cube, 0.0, 1.0, cube(0.0), cube(1.0), 1e-12).unwrap();
        assert!((x - 0.3).abs() < 1e-11, "{x}");

        // A step: the "root" is the jump, found to the tolerance, on the far side.
        let step = |x: f64| if x < 0.123_456 { -1.0 } else { 1.0 };
        let mut calls = 0;
        let x = find_root(
            |x| {
                calls += 1;
                step(x)
            },
            0.0,
            1.0,
            -1.0,
            1.0,
            1e-12,
        )
        .unwrap();
        assert!((x - 0.123_456).abs() < 1e-12, "{x}");
        assert_eq!(step(x), 1.0, "the far side of the jump");
        assert!(calls < 60, "{calls} evaluations");
        // Reversed signs: the far side is now the negative one.
        let x = find_root(|x| -step(x), 0.0, 1.0, 1.0, -1.0, 1e-12).unwrap();
        assert_eq!(-step(x), -1.0);

        // Ends that are roots, ends that don't bracket, a function that turns NaN, and bad
        // tolerances.
        assert_eq!(find_root(|x| x, 0.0, 1.0, 0.0, 1.0, 1e-12), Ok(0.0));
        assert_eq!(find_root(|x| x - 1.0, 0.0, 1.0, -1.0, 0.0, 1e-12), Ok(1.0));
        assert_eq!(
            find_root(|x| x + 1.0, 0.0, 1.0, 1.0, 2.0, 1e-12),
            Err(RootError::NotBracketed)
        );
        assert!(matches!(
            find_root(|_| f64::NAN, -1.0, 1.0, -1.0, 1.0, 1e-12),
            Err(RootError::NotFinite { .. })
        ));
        assert!(matches!(
            find_root(|x| x, -1.0, 1.0, -1.0, 1.0, f64::NAN),
            Err(RootError::Tolerance { .. })
        ));
    }

    /// Runs to `t_stop`, collecting every fired event with its time and state.
    fn collect_events<S: OdeSystem<N>, const N: usize>(
        integrator: &mut Integrator<N>,
        system: &mut S,
        t_stop: f64,
    ) -> Vec<(usize, f64, [f64; N])>
    where
        S::Error: std::fmt::Debug,
    {
        let mut found = Vec::new();
        loop {
            match integrator.advance(system, t_stop).unwrap() {
                Advance::Reached => return found,
                Advance::Events => {
                    for index in integrator.fired_events() {
                        found.push((*index, integrator.time_s(), *integrator.state()));
                    }
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn oscillator_crossings_are_located_within_1e_6_s_by_both_methods() {
        // x = cos t crosses zero at π/2 + kπ: falling at even k. The events are falling zeros of
        // x and every extremum (zeros of x').
        for method in [Method::default(), Method::Rk4 { step_s: 0.01 }] {
            let mut system = WithEvents::new(
                Oscillator,
                vec![Direction::Falling, Direction::Either],
                |i: usize, _t: f64, y: &[f64; 2]| y[i],
            );
            let mut integrator = Integrator::new(method, 0.0, [1.0, 0.0]).unwrap();
            let found = collect_events(&mut integrator, &mut system, 20.0);
            let pi = std::f64::consts::PI;
            let falling: Vec<f64> = (0..3).map(|k| pi / 2.0 + 2.0 * pi * f64::from(k)).collect();
            let extrema: Vec<f64> = (1..7).map(|k| pi * f64::from(k)).collect();
            let got = |index: usize| -> Vec<f64> {
                found
                    .iter()
                    .filter(|(i, _, _)| *i == index)
                    .map(|(_, t, _)| *t)
                    .collect()
            };
            for (expected, actual) in [(falling, got(0)), (extrema, got(1))] {
                assert_eq!(expected.len(), actual.len(), "{method:?}: {found:?}");
                for (e, a) in expected.iter().zip(&actual) {
                    assert!((e - a).abs() <= 1e-6, "{method:?}: {a} vs {e}");
                }
            }
        }
    }

    #[test]
    fn apogee_landing_and_altitude_deploy_located_within_1e_6_s() {
        // Loft lesson L22: events were never root-found, so apogee was quantised to the step and
        // an altitude deploy overshot by v·dt. A vertical flight with quadratic drag has closed
        // forms for all three events (`testing::closed_form_quadratic_drag`), and its drag term
        // `v|v|` is not smooth at apogee, which a polynomial test would hide.
        let flight = QuadraticDragFall::example();
        let truth = closed_form_quadratic_drag(&flight, 150.0);
        let deploy_m = 300.0;
        let g = move |i: usize, _t: f64, y: &[f64; 2]| match i {
            0 => y[1],
            1 => y[0] - deploy_m,
            _ => y[0],
        };
        for method in [Method::default(), Method::Rk4 { step_s: 0.01 }] {
            let mut system = WithEvents::new(flight.clone(), vec![Direction::Falling; 3], g);
            let mut integrator = Integrator::new(method, 0.0, [0.0, 150.0]).unwrap();
            let landing_s = truth.time_at_descending_height_s(0.0);
            let found = collect_events(&mut integrator, &mut system, landing_s + 1.0);
            // Landing is the last event; the flight continues underground to the stop time.
            let [apogee, deploy, landing] = found.as_slice() else {
                panic!("{method:?}: {found:?}");
            };
            let expected = [
                (0, truth.apogee_s),
                (1, truth.time_at_descending_height_s(deploy_m)),
                (2, landing_s),
            ];
            for ((index, t, y), (want_index, want_t)) in
                [apogee, deploy, landing].into_iter().zip(expected)
            {
                assert_eq!(*index, want_index, "{method:?}");
                assert!(
                    (t - want_t).abs() <= 1e-6,
                    "{method:?} event {index}: {t} vs {want_t}"
                );
                // The state is the dense output's at the stop: on the root to the integration's
                // accuracy.
                let value = g(*index, *t, y);
                assert!(value.abs() < 1e-5, "{method:?} event {index}: g = {value}");
            }
            assert!((apogee.2[0] - truth.apogee_m).abs() < 1e-6, "{method:?}");
            let landing_speed = truth.state(landing.1)[1];
            assert!((landing.2[1] - landing_speed).abs() < 1e-6, "{method:?}");
        }
    }

    #[test]
    fn coincident_events_are_all_reported_once() {
        // Two identical apogee events and a third that crosses at the same instant but is written
        // differently: all three fire together, once per crossing.
        let g = |i: usize, _t: f64, y: &[f64; 2]| match i {
            0 | 1 => y[1],
            _ => 2.0 * y[1],
        };
        for method in [Method::default(), Method::Rk4 { step_s: 0.01 }] {
            let mut system = WithEvents::new(Oscillator, vec![Direction::Falling; 3], g);
            let mut integrator = Integrator::new(method, 0.0, [0.0, 1.0]).unwrap();
            let found = collect_events(&mut integrator, &mut system, 10.0);
            let indices: Vec<usize> = found.iter().map(|(i, _, _)| *i).collect();
            assert_eq!(indices, [0, 1, 2, 0, 1, 2], "{method:?}: {found:?}");
            let pi = std::f64::consts::PI;
            for (k, (_, t, _)) in found.iter().enumerate() {
                let want = pi / 2.0 + 2.0 * pi * (k / 3) as f64;
                assert!((t - want).abs() < 1e-6, "{method:?}: {found:?}");
            }
        }
    }

    #[test]
    fn event_times_are_resolved_to_the_root_finder_tolerance() {
        // On y' = 1 both dense outputs are exact, so the only error left is the root finder's.
        // The event functions are nonlinear in t, where a secant step is not exact.
        struct Clock;
        impl OdeSystem<1> for Clock {
            type Error = std::convert::Infallible;
            fn derivative(&mut self, _t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Self::Error> {
                Ok([1.0])
            }
        }
        let g = |i: usize, _t: f64, y: &[f64; 1]| match i {
            0 => y[0].powi(3) - 0.3,
            _ => y[0].sin() - 0.5,
        };
        // Each g crosses once on [0, 2], so no step can hide a return.
        let roots = [0.3_f64.cbrt(), std::f64::consts::FRAC_PI_6];
        for method in [Method::default(), Method::Rk4 { step_s: 0.9 }] {
            for (index, root) in roots.into_iter().enumerate() {
                let mut system = WithEvents::new(
                    Clock,
                    vec![Direction::Rising],
                    move |_: usize, t: f64, y: &[f64; 1]| g(index, t, y),
                );
                let mut integrator = Integrator::new(method, 0.0, [0.0]).unwrap();
                assert_eq!(integrator.advance(&mut system, 2.0), Ok(Advance::Events));
                let t = integrator.time_s();
                let error = t - root;
                assert!(
                    (-1e-15..=2.5e-12).contains(&error),
                    "{method:?} event {index}: {error:e} past the root"
                );
                assert!(g(index, t, integrator.state()) >= 0.0, "on the far side");
            }
        }
    }

    #[test]
    fn a_restart_on_an_event_does_not_report_it_again() {
        let mut system = WithEvents::new(
            Oscillator,
            vec![Direction::Either],
            |_: usize, _t: f64, y: &[f64; 2]| y[0],
        );
        let mut integrator = Integrator::new(Method::default(), 0.0, [1.0, 0.0]).unwrap();
        let mut times = Vec::new();
        for _ in 0..3 {
            let outcome = integrator.advance(&mut system, 100.0).unwrap();
            assert_eq!(outcome, Advance::Events);
            assert_eq!(integrator.fired_events(), [0]);
            times.push(integrator.time_s());
        }
        let pi = std::f64::consts::PI;
        for (k, t) in times.iter().enumerate() {
            let want = pi / 2.0 + pi * k as f64;
            assert!((t - want).abs() < 1e-6, "{times:?}");
        }
    }

    #[test]
    fn the_earliest_of_several_crossings_in_one_step_wins() {
        // One long RK4 step holds crossings of x = t − 0.7, x = t − 0.2 and x = t − 0.5; the
        // integration stops at 0.2 and then finds 0.5 and 0.7 in turn.
        struct Clock;
        impl OdeSystem<1> for Clock {
            type Error = std::convert::Infallible;
            fn derivative(&mut self, _t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Self::Error> {
                Ok([1.0])
            }
        }
        let offsets = [0.7, 0.2, 0.5];
        let mut system = WithEvents::new(
            Clock,
            vec![Direction::Rising; 3],
            |i: usize, _t: f64, y: &[f64; 1]| y[0] - offsets[i],
        );
        let mut integrator = Integrator::new(Method::Rk4 { step_s: 10.0 }, 0.0, [0.0]).unwrap();
        let order = collect_events(&mut integrator, &mut system, 5.0);
        assert_eq!(order.len(), 3, "{order:?}");
        for ((index, t, _), (want_index, want_t)) in
            order.iter().zip([(1, 0.2), (2, 0.5), (0, 0.7)])
        {
            assert_eq!(*index, want_index);
            assert!((t - want_t).abs() < 1e-12, "{order:?}");
        }
        assert_eq!(integrator.time_s(), 5.0);
    }
}
