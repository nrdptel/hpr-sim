use std::cell::Cell;
use std::convert::Infallible;
use std::ops::ControlFlow;

use super::*;
use crate::events::Direction;
use crate::testing::{
    ConstantThrustVacuum, QuadraticDragFall, Watched, WithEvents, closed_form_quadratic_drag,
};

/// `y' = −2 t y²`, `y(0) = 1`: `y = 1/(1 + t²)`. Nonlinear and non-autonomous, so the order
/// conditions of every tree matter.
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

/// `y' = 1`: the dense output of either method is exact.
struct Clock;

impl OdeSystem<1> for Clock {
    type Error = Infallible;
    fn derivative(&mut self, _t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Infallible> {
        Ok([1.0])
    }
}

/// `(A, c, b, b̂, d)`: the tableau as matrices, for the order conditions.
type Tableau = ([[f64; 7]; 7], [f64; 7], [f64; 7], [f64; 7], [f64; 7]);

fn tableau() -> Tableau {
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
    let d = [D1, 0.0, D3, D4, D5, D6, D7];
    let b_hat = std::array::from_fn(|i| b[i] - e[i]);
    (a, c, b, b_hat, d)
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
/// `(order, Σ bᵢ Φᵢ, 1/γ)`. For a continuous extension at `θ`, the right side is `θ^order/γ`.
fn order_conditions(weights: &[f64; 7], theta: f64) -> Vec<(u32, f64, f64)> {
    let (a, c, _, _, _) = tableau();
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
    let rows = [
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
    ];
    rows.into_iter()
        .map(|(order, value, gamma)| (order, value, gamma * theta.powi(order as i32)))
        .collect()
}

#[test]
fn tableau_satisfies_the_order_conditions() {
    let (a, c, b, b_hat, d) = tableau();
    for (row, ci) in a.iter().zip(c) {
        assert!((row.iter().sum::<f64>() - ci).abs() < 1e-14);
    }
    for (order, value, expected) in order_conditions(&b, 1.0) {
        assert!(
            (value - expected).abs() < 1e-14,
            "b, order {order}: {value}"
        );
    }
    let embedded = order_conditions(&b_hat, 1.0);
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
    // The continuous extension's weights: with Δ = h Σ bⱼkⱼ, r₂ = h k₁ − Δ, r₃ = 2Δ − h k₁ − h k₇
    // and r₄ = h Σ dⱼkⱼ, `y(θ) = y₀ + h Σ bⱼ(θ) kⱼ` with
    // `bⱼ(θ) = θ bⱼ + θ(1−θ)(δ₁ⱼ − bⱼ + θ(2bⱼ − δ₁ⱼ − δ₇ⱼ + (1−θ) dⱼ))`. They satisfy the
    // conditions to order 4 at every θ (HNW I, §II.6), which pins D1–D7.
    for theta in [0.1, 0.3, 0.5, 0.7, 0.9] {
        let weights: [f64; 7] = std::array::from_fn(|j| {
            let delta1 = if j == 0 { 1.0 } else { 0.0 };
            let delta7 = if j == 6 { 1.0 } else { 0.0 };
            theta * b[j]
                + theta
                    * (1.0 - theta)
                    * (delta1 - b[j]
                        + theta * (2.0 * b[j] - delta1 - delta7 + (1.0 - theta) * d[j]))
        });
        let conditions = order_conditions(&weights, theta);
        for (order, value, expected) in &conditions {
            if *order <= 4 {
                assert!(
                    (value - expected).abs() < 1e-14,
                    "b(θ = {theta}), order {order}: {value} vs {expected}"
                );
            }
        }
    }
}

/// Observed orders `log₂(e(h)/e(h/2))` from errors at halving steps.
fn observed_orders(errors: &[f64]) -> Vec<f64> {
    errors.windows(2).map(|w| (w[0] / w[1]).log2()).collect()
}

/// Integrates `Rational` over `[0, 2]` in `n` equal steps, returning the end error and the largest
/// dense-output error at θ = ¼, ½, ¾.
fn equal_steps(method: Method) -> (f64, f64, u64) {
    let end = 2.0;
    let dense_error = Cell::new(0.0_f64);
    let mut system = Watched::new(Rational, |step: &Step<1>| {
        let h = step.end_s() - step.start_s();
        for theta in [0.25, 0.5, 0.75] {
            let t = step.start_s() + theta * h;
            let error = (step.state_at(t)[0] - rational(t)).abs();
            dense_error.set(dense_error.get().max(error));
        }
        ControlFlow::Continue(())
    });
    let mut integrator = Integrator::new(method, 0.0, [1.0]).unwrap();
    let outcome = integrator.advance(&mut system, end).unwrap();
    assert_eq!(outcome, Advance::Reached);
    assert_eq!(integrator.time_s(), end);
    (
        (integrator.state()[0] - rational(end)).abs(),
        dense_error.get(),
        integrator.stats().accepted_steps,
    )
}

#[test]
fn fixed_step_dormand_prince_converges_at_fifth_order() {
    // Tolerances too loose to reject anything, with the first and longest step both h, make the
    // adaptive driver take equal steps.
    let mut errors = Vec::new();
    let mut dense_errors = Vec::new();
    for n in [10_u32, 20, 40, 80, 160] {
        let h = 2.0 / f64::from(n);
        let method = Method::DormandPrince54(Adaptive {
            relative_tolerance: 1e6,
            absolute_tolerance: 1e6,
            initial_step_s: Some(h),
            max_step_s: Some(h),
        });
        let (error, dense_error, accepted) = equal_steps(method);
        assert_eq!(accepted, u64::from(n));
        errors.push(error);
        dense_errors.push(dense_error);
    }
    // Measured: 5.88, 5.53, 5.31, 5.17, approaching 5 from above as h shrinks (at 320 steps the
    // error, 6e-15, reaches rounding).
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
    // carried forward, and the nodes are fifth order, so the interpolated error also falls as h⁵
    // (measured 4.81, 4.97, 4.99, 4.99).
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
    let mut errors = Vec::new();
    let mut dense_errors = Vec::new();
    for n in [20_u32, 40, 80, 160] {
        let step_s = 2.0 / f64::from(n);
        let (error, dense_error, accepted) = equal_steps(Method::Rk4 { step_s });
        assert_eq!(accepted, u64::from(n));
        errors.push(error);
        dense_errors.push(dense_error);
    }
    // Measured: 4.04, 4.02, 4.01, and 3.96, 3.99, 4.00 for the Hermite dense output (local error
    // O(h⁴)).
    let orders = observed_orders(&errors);
    for order in &orders {
        assert!((3.95..=4.1).contains(order), "{errors:?} → {orders:?}");
    }
    let dense_orders = observed_orders(&dense_errors);
    for order in &dense_orders {
        assert!(
            (3.9..=4.05).contains(order),
            "{dense_errors:?} → {dense_orders:?}"
        );
    }
}

/// The restricted three-body problem of Hairer's `DOPRI5` driver (`dr_dopri5.f`): the Arenstorf
/// orbit, periodic with period `x_end`.
struct Arenstorf;

impl OdeSystem<4> for Arenstorf {
    type Error = Infallible;
    fn derivative(&mut self, _t: f64, y: &[f64; 4]) -> Result<[f64; 4], Infallible> {
        let mu = 0.012_277_471;
        let mu1 = 1.0 - mu;
        let r1 = ((y[0] + mu).powi(2) + y[1] * y[1]).powf(1.5);
        let r2 = ((y[0] - mu1).powi(2) + y[1] * y[1]).powf(1.5);
        Ok([
            y[2],
            y[3],
            y[0] + 2.0 * y[3] - mu1 * (y[0] + mu) / r1 - mu * (y[0] - mu1) / r2,
            y[1] - 2.0 * y[2] - mu1 * y[1] / r1 - mu * y[1] / r2,
        ])
    }
}

#[test]
fn arenstorf_orbit_takes_dopri5s_steps() {
    // An independent line-by-line transcription of `DOPCOR` and `HINIT` on Hairer's driver
    // problem gives these counts of evaluations, attempted steps and accepted steps, and the end
    // position. `DOPCOR` doesn't count rejections before the first accepted step; hpr does, so the
    // attempted steps are compared.
    // 17.0652165601579625588917206249 and −2.00158510637908252240537862224 in the driver.
    let x_end = 17.065_216_560_157_964;
    let cases = [
        (
            1e-4,
            494,
            82,
            64,
            [0.996_595_343_976_327_4, -0.001_375_566_905_016_863_5],
        ),
        (
            1e-7,
            1442,
            240,
            216,
            [0.994_002_101_581_418_9, 8.911_185_607_728_632e-6],
        ),
        (
            1e-10,
            5060,
            843,
            841,
            [0.993_999_994_324_563_2, -1.478_425_974_804_192_2e-8],
        ),
    ];
    for (tol, evaluations, attempted, accepted, end) in cases {
        let method = Method::DormandPrince54(Adaptive {
            relative_tolerance: tol,
            absolute_tolerance: tol,
            ..Adaptive::default()
        });
        let y0 = [0.994, 0.0, 0.0, -2.001_585_106_379_082_4];
        let mut integrator = Integrator::new(method, 0.0, y0).unwrap();
        integrator.advance(&mut Arenstorf, x_end).unwrap();
        let stats = integrator.stats();
        assert_eq!(stats.evaluations, evaluations, "tol {tol}: {stats:?}");
        assert_eq!(stats.accepted_steps, accepted, "tol {tol}: {stats:?}");
        assert_eq!(
            stats.accepted_steps + stats.rejected_steps,
            attempted,
            "tol {tol}: {stats:?}"
        );
        // The orbit amplifies rounding, which the transcription does in a different order.
        for (got, want) in integrator.state()[..2].iter().zip(end) {
            assert!((got - want).abs() < 1e-9, "tol {tol}: {got} vs {want}");
        }
    }
}

#[test]
fn adaptive_error_is_proportional_to_the_tolerance() {
    // With local extrapolation the global error of DOPRI5 scales about linearly with the
    // tolerance (HNW I, §II.4, "tolerance proportionality"). Measured: 0.22 to 0.28 of the
    // tolerance from 1e-4 to 1e-9.
    for k in 0..6 {
        let tol = 1e-4 * 10f64.powi(-k);
        let method = Method::DormandPrince54(Adaptive {
            relative_tolerance: tol,
            absolute_tolerance: tol,
            ..Adaptive::default()
        });
        let mut integrator = Integrator::new(method, 0.0, [1.0]).unwrap();
        integrator.advance(&mut Rational, 10.0).unwrap();
        let error = (integrator.state()[0] - rational(10.0)).abs();
        assert!(
            (0.07 * tol..=0.85 * tol).contains(&error),
            "tol {tol}: error {error}"
        );
    }
}

/// The least-squares slope of `ln y` against `ln x`.
fn log_log_slope(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    let (sx, sy, sxx, sxy) = points.iter().fold((0.0, 0.0, 0.0, 0.0), |acc, (x, y)| {
        let (lx, ly) = (x.ln(), y.ln());
        (acc.0 + lx, acc.1 + ly, acc.2 + lx * lx, acc.3 + lx * ly)
    });
    (n * sxy - sx * sy) / (n * sxx - sx * sx)
}

#[test]
fn apogee_converges_under_tolerance_halving() {
    // Loft lesson L21: RK4 with no error control and no convergence check on apogee. Here the
    // apogee of a vertical flight with quadratic drag is found as an event at tolerances halving
    // from 1e-5 to 2e-8 and compared with the closed form. The drag `v|v|` has a kink at apogee,
    // so single halvings aren't monotone (the time error went 2.5e-8 s → 2.8e-6 s once), but the
    // errors converge: over the 512× drop in tolerance the height error falls 4100× and the time
    // error 48,000×. Measured worst cases: 117·tol for the time and 91·tol for the height.
    let flight = QuadraticDragFall::example();
    let truth = closed_form_quadratic_drag(&flight, 150.0);
    let mut heights = Vec::new();
    let mut times = Vec::new();
    for k in 0..10 {
        let tol = 1e-5 / 2f64.powi(k);
        let method = Method::DormandPrince54(Adaptive {
            relative_tolerance: tol,
            absolute_tolerance: tol,
            ..Adaptive::default()
        });
        let mut integrator = Integrator::new(method, 0.0, [0.0, 150.0]).unwrap();
        let mut system = WithEvents::new(
            flight.clone(),
            vec![Direction::Falling],
            |_: usize, _t: f64, y: &[f64; 2]| y[1],
        );
        let outcome = integrator.advance(&mut system, 100.0).unwrap();
        assert_eq!(outcome, Advance::Events);
        let dt = (integrator.time_s() - truth.apogee_s).abs();
        let dh = (integrator.state()[0] - truth.apogee_m).abs();
        assert!(
            dt < 350.0 * tol && dh < 280.0 * tol,
            "tol {tol}: dt {dt}, dh {dh}"
        );
        times.push((tol, dt));
        heights.push((tol, dh));
    }
    let (dt_first, dt_last) = (times[0].1, times[9].1);
    let (dh_first, dh_last) = (heights[0].1, heights[9].1);
    assert!(dt_last < 4e-8 && dh_last < 3e-7, "{times:?} {heights:?}");
    assert!(
        dt_first / dt_last > 1e4 && dh_first / dh_last > 1e3,
        "{times:?} {heights:?}"
    );
    // Tolerance proportionality in the height error, with the kink's scatter (measured slope 1.36).
    let slope = log_log_slope(&heights);
    assert!((1.1..=1.6).contains(&slope), "slope {slope}: {heights:?}");
}

#[test]
fn constant_thrust_vacuum_matches_closed_form() {
    // Loft lesson L23: burnout fell inside a step and the vacuum case was only checked to ±2%.
    // Burnout is a stop time here, so the discontinuity sits on a step boundary. Measured: burnout
    // state within 2.2e-10 (Dormand–Prince, default tolerances) and 6.6e-11 (RK4, 0.01 s)
    // relative; apogee within 6.3e-9 s and 5.2e-6 m of 48.8 km, and 4e-11 s and 1.6e-8 m.
    let mut rocket = ConstantThrustVacuum {
        gravity_mps2: 9.806_65,
        thrust_n: 2000.0,
        exhaust_velocity_m_s: 2000.0,
        burning: true,
    };
    let m0 = 20.0;
    let burnout_s = 8.0;
    for (method, state_rel, apogee_dt, apogee_rel) in [
        (Method::default(), 7e-10, 2e-8, 3.3e-10),
        (Method::Rk4 { step_s: 0.01 }, 2e-10, 2e-10, 1e-12),
    ] {
        rocket.burning = true;
        let mut integrator = Integrator::new(method, 0.0, [0.0, 0.0, m0]).unwrap();
        let outcome = integrator.advance(&mut rocket, burnout_s).unwrap();
        assert_eq!(outcome, Advance::Reached);
        assert_eq!(integrator.time_s(), burnout_s);
        let expected = rocket.powered_state(m0, burnout_s);
        for (got, want) in integrator.state().iter().zip(expected) {
            assert!(
                (got - want).abs() <= state_rel * want.abs(),
                "{method:?}: {got} vs {want}"
            );
        }

        rocket.burning = false;
        let mut system = WithEvents::new(
            rocket.clone(),
            vec![Direction::Falling],
            |_: usize, _t: f64, y: &[f64; 3]| y[1],
        );
        let outcome = integrator.advance(&mut system, 1000.0).unwrap();
        assert_eq!(outcome, Advance::Events);
        let [h_b, v_b, _] = expected;
        let g = rocket.gravity_mps2;
        let apogee_s = burnout_s + v_b / g;
        let apogee_m = h_b + v_b * v_b / (2.0 * g);
        let dt = (integrator.time_s() - apogee_s).abs();
        let dh = (integrator.state()[0] - apogee_m).abs();
        assert!(dt < apogee_dt, "{method:?}: dt {dt}");
        assert!(dh < apogee_rel * apogee_m, "{method:?}: dh {dh}");
    }
}

#[test]
fn a_discontinuity_inside_a_step_costs_accuracy_that_a_stop_time_keeps() {
    // The same flight with burnout switched by time inside the derivative: fixed-step RK4 drops to
    // first order across the jump in thrust. With burnout as a stop time and the phase set by the
    // caller, each step sees one side of the jump only. (Deciding the phase from `t` alone would
    // not do: the last stage of the step ending at burnout is evaluated at burnout and belongs to
    // the burning side.)
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
            integrator.advance(&mut system, 8.005).unwrap();
            system.phase = Some(false);
        }
        integrator.advance(&mut system, 10.0).unwrap();
        (integrator.state()[1] - exact_v(10.0)).abs()
    };
    let (inside, on_boundary) = (run(false), run(true));
    // Measured: 0.56 m/s through the jump, 1.0e-12 m/s with the stop time.
    assert!(on_boundary < 3e-12, "{on_boundary}");
    assert!(inside > 0.1, "{inside}");
}

#[test]
fn stop_times_are_exact_and_repeated_runs_are_bit_identical() {
    let run = || {
        let mut ends = Vec::new();
        let mut system = Watched::new(Rational, |s: &Step<1>| {
            ends.push(s.end_s().to_bits());
            ControlFlow::Continue(())
        });
        let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
        for stop in [0.1, 0.3, 1.7, 5.0] {
            integrator.advance(&mut system, stop).unwrap();
            assert_eq!(integrator.time_s(), stop);
        }
        (integrator.state()[0].to_bits(), ends, integrator.stats())
    };
    assert_eq!(run(), run());
}

#[test]
fn stops_within_a_few_ulps_are_reached() {
    // A stop time computed by a different sum can sit a few ulps past the current time. Both
    // methods must land on it, not fail for a step too short to take.
    for method in [Method::default(), Method::Rk4 { step_s: 0.5 }] {
        for start in [0.0, 3.0, 100.0, 1000.0] {
            for ulps in 1..=40_u32 {
                let mut integrator = Integrator::new(method, 0.0, [1.0]).unwrap();
                if start > 0.0 {
                    integrator.advance(&mut Rational, start).unwrap();
                }
                let mut stop = start;
                for _ in 0..ulps {
                    stop = stop.next_up();
                }
                let outcome = integrator.advance(&mut Rational, stop);
                assert_eq!(
                    outcome,
                    Ok(Advance::Reached),
                    "{method:?} {start} + {ulps} ulps"
                );
                assert_eq!(integrator.time_s(), stop);
            }
        }
    }
}

#[test]
fn steps_join_up_respect_the_limit_and_interpolate_to_their_ends() {
    let max_step_s = 0.07;
    let method = Method::DormandPrince54(Adaptive {
        max_step_s: Some(max_step_s),
        ..Adaptive::default()
    });
    let mut previous_end = 0.0;
    let mut count = 0;
    let mut worst: f64 = 0.0;
    let mut system = Watched::new(Rational, |step: &Step<1>| {
        assert_eq!(step.start_s(), previous_end);
        assert!(step.end_s() - step.start_s() <= max_step_s * (1.0 + 1e-9));
        assert_eq!(step.state_at(step.start_s()), *step.start());
        assert_eq!(step.state_at(step.end_s()), *step.end());
        // Just inside the end, the dense output is as good as the step.
        let t = step.end_s() - 1e-9 * (step.end_s() - step.start_s());
        worst = worst.max((step.state_at(t)[0] - rational(t)).abs());
        previous_end = step.end_s();
        count += 1;
        ControlFlow::Continue(())
    });
    let mut integrator = Integrator::new(method, 0.0, [1.0]).unwrap();
    // 3.0 is 42.86 longest steps: the last one must not stretch past the limit.
    integrator.advance(&mut system, 3.0).unwrap();
    assert_eq!(previous_end, 3.0);
    assert!(worst < 1e-8, "{worst}");
    let stats = integrator.stats();
    assert_eq!(stats.accepted_steps, count);
    assert!(
        count >= 43,
        "{count} steps of at most {max_step_s} s over 3 s"
    );
    // First-same-as-last: six new evaluations per attempted step, plus the start and the first
    // guess.
    assert_eq!(
        stats.evaluations,
        6 * (stats.accepted_steps + stats.rejected_steps) + 2
    );
    // A stop just over one longest step away is not reached by stretching the step past the limit.
    let method = Method::DormandPrince54(Adaptive {
        initial_step_s: Some(0.1),
        max_step_s: Some(0.1),
        ..Adaptive::default()
    });
    let mut longest: f64 = 0.0;
    let mut clock = Watched::new(Clock, |step: &Step<1>| {
        longest = longest.max(step.end_s() - step.start_s());
        ControlFlow::Continue(())
    });
    let mut stretched = Integrator::new(method, 0.0, [0.0]).unwrap();
    stretched.advance(&mut clock, 0.9).unwrap();
    stretched.advance(&mut clock, 0.9 + 0.1005).unwrap();
    assert!(longest <= 0.1 * (1.0 + 1e-9), "{longest}");

    // The step estimate carries over a reset.
    let next = integrator.next_step_s();
    assert!(next.is_some());
    integrator.reset(3.0, [0.1]).unwrap();
    assert_eq!(integrator.next_step_s(), next);
}

#[test]
fn a_system_can_stop_the_integration() {
    let mut system = Watched::new(Rational, |step: &Step<1>| {
        if step.end_s() > 1.0 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
    assert_eq!(integrator.advance(&mut system, 5.0), Ok(Advance::Stopped));
    assert!(integrator.time_s() > 1.0 && integrator.time_s() < 5.0);
}

#[derive(Debug, PartialEq)]
struct Refused;

#[test]
fn failures_are_errors_and_the_integrator_can_resume() {
    // A derivative that fails past t = 1: trial stages beyond it are rejections until the step
    // can't shrink, and the error is reported with the integrator just short of t = 1.
    struct Failing;
    impl OdeSystem<1> for Failing {
        type Error = Refused;
        fn derivative(&mut self, t: f64, _y: &[f64; 1]) -> Result<[f64; 1], Refused> {
            if t > 1.0 { Err(Refused) } else { Ok([1.0]) }
        }
    }
    let mut integrator = Integrator::new(Method::default(), 0.0, [0.0]).unwrap();
    let error = integrator.advance(&mut Failing, 2.0).unwrap_err();
    assert!(
        matches!(error, IntegrationError::Derivative { source: Refused, t_s } if t_s > 1.0),
        "{error:?}"
    );
    assert!(1.0 - integrator.time_s() < 1e-9, "{}", integrator.time_s());
    // RK4 has no shorter step to try.
    let mut integrator = Integrator::new(Method::Rk4 { step_s: 0.3 }, 0.0, [0.0]).unwrap();
    let error = integrator.advance(&mut Failing, 2.0).unwrap_err();
    assert!(matches!(error, IntegrationError::Derivative { .. }));
    assert!((integrator.time_s() - 0.9).abs() < 1e-12);

    // A blow-up: y' = y², y(0) = 1 has y = 1/(1 − t).
    struct BlowUp;
    impl OdeSystem<1> for BlowUp {
        type Error = Infallible;
        fn derivative(&mut self, _t: f64, y: &[f64; 1]) -> Result<[f64; 1], Infallible> {
            Ok([y[0] * y[0]])
        }
    }
    let mut integrator = Integrator::new(Method::default(), 0.0, [1.0]).unwrap();
    let error = integrator.advance(&mut BlowUp, 2.0).unwrap_err();
    assert!(
        matches!(
            error,
            IntegrationError::StepTooSmall { t_s, .. } | IntegrationError::NotFinite { t_s }
                if (t_s - 1.0).abs() < 1e-3
        ),
        "{error:?}"
    );

    // An infinite stop with nothing to end it: y' = 1 has no error, the step grows tenfold per
    // step until it overflows, and that is an error, not a spin to the step limit.
    let mut integrator = Integrator::new(Method::default(), 0.0, [0.0]).unwrap();
    let error = integrator.advance(&mut Clock, f64::INFINITY).unwrap_err();
    assert!(
        matches!(error, IntegrationError::NotFinite { .. }),
        "{error:?}"
    );
    assert!(
        integrator.stats().evaluations < 3000,
        "{:?}",
        integrator.stats()
    );

    // The step limit, and resuming past it.
    let mut integrator = Integrator::new(Method::Rk4 { step_s: 0.01 }, 0.0, [1.0])
        .unwrap()
        .with_step_limit(50);
    let error = integrator.advance(&mut Rational, 2.0).unwrap_err();
    assert_eq!(
        error,
        IntegrationError::StepLimit {
            t_s: integrator.time_s(),
            limit: 50
        }
    );
    assert!((integrator.time_s() - 0.5).abs() < 1e-12);
    integrator.set_step_limit(1000);
    assert_eq!(integrator.advance(&mut Rational, 2.0), Ok(Advance::Reached));
    assert!((integrator.state()[0] - rational(2.0)).abs() < 1e-8);

    // Bad settings, a backward stop and bad weights.
    assert!(Integrator::new(Method::Rk4 { step_s: 0.0 }, 0.0, [1.0]).is_err());
    assert!(Integrator::new(Method::default(), f64::NAN, [1.0]).is_err());
    assert!(Integrator::new(Method::default(), 0.0, [f64::INFINITY]).is_err());
    let error = integrator.advance(&mut Rational, 0.0).unwrap_err();
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
    let error = integrator.advance(&mut Unweighted, 1.0).unwrap_err();
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
    let json = r#"{"method":"dopri5","relative_tolerance":1e-6}"#;
    let Method::DormandPrince54(adaptive) = serde_json::from_str(json).unwrap() else {
        panic!("{json}");
    };
    assert_eq!(adaptive.relative_tolerance, 1e-6);
    assert_eq!(adaptive.absolute_tolerance, 1e-8);
    for typo in [
        r#"{"method":"rk4","step_s":0.01,"stepsize":1}"#,
        r#"{"method":"dopri5","relative_tolerence":1e-6}"#,
        r#"{"method":"dormand_prince54"}"#,
    ] {
        assert!(serde_json::from_str::<Method>(typo).is_err(), "{typo}");
    }
}
