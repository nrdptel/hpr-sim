//! Thrust curves: thrust against time, and the impulse, burn time and average thrust derived from
//! them.
//!
//! A [`ThrustCurve`] holds samples `(t_i, F_i)` and joins them with straight lines, which is how
//! the RASP `.eng` and RockSim `.rse` formats are drawn and how RocketPy reads them by default.
//! When the first sample is after `t = 0`, the curve starts from an implicit `(0, 0)`, as the RASP
//! format specifies. After the last sample the thrust is zero. Two samples at the same time make a
//! step: the thrust takes the later sample's value from that time on. Real curves use steps for an
//! abrupt burnout and where times were rounded (`docs/format/eng.md`).
//!
//! **Impulse.** The integral of the piecewise-linear thrust is exact by the trapezoid rule:
//!
//! ```text
//! I(t) = Σ ½ (F_{i} + F_{i+1}) (t_{i+1} − t_i)   over the intervals before t, plus the partial one
//! ```
//!
//! **Burn time (NFPA 1125).** From the moment the thrust first reaches 5% of its peak to the
//! moment it last falls to 5% of its peak, with the crossings found on the straight segments.
//! ThrustCurve.org's glossary uses this definition. **Average thrust** is the total impulse over
//! that burn time. See `docs/physics/motor.md`.

use serde::{Deserialize, Serialize};

use crate::error::MotorError;

/// The fraction of peak thrust that starts and ends the NFPA 1125 burn time.
pub const NFPA_1125_THRESHOLD: f64 = 0.05;

/// A thrust curve: thrust in newtons against time in seconds from ignition, joined by straight
/// lines.
///
/// Built through [`ThrustCurve::new`], which checks the samples; it serializes as its samples and
/// re-checks them when deserialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CurveData", into = "CurveData")]
pub struct ThrustCurve {
    /// Sample times, s, non-decreasing and starting at 0.
    times_s: Vec<f64>,
    /// Thrust at each time, N, non-negative.
    thrusts_n: Vec<f64>,
    /// `I(t_i)`, the impulse delivered by each sample time, N·s.
    impulse_ns: Vec<f64>,
}

/// The serialized form of a [`ThrustCurve`]: its samples as given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CurveData {
    times_s: Vec<f64>,
    thrusts_n: Vec<f64>,
}

impl TryFrom<CurveData> for ThrustCurve {
    type Error = MotorError;

    fn try_from(data: CurveData) -> Result<Self, Self::Error> {
        Self::new(data.times_s, data.thrusts_n)
    }
}

impl From<ThrustCurve> for CurveData {
    fn from(curve: ThrustCurve) -> Self {
        Self {
            times_s: curve.times_s,
            thrusts_n: curve.thrusts_n,
        }
    }
}

impl ThrustCurve {
    /// Builds a curve from sample times (s) and thrusts (N). A `(0, 0)` sample is prepended when
    /// the first time is after zero.
    ///
    /// # Errors
    ///
    /// - [`MotorError::Inconsistent`] if the columns differ in length or are empty.
    /// - [`MotorError::Domain`] for a time or thrust that is not finite, a negative time, or a
    ///   negative thrust.
    /// - [`MotorError::TimesDecreasing`] if a time is before its predecessor. Equal times are a
    ///   step.
    /// - [`MotorError::NoThrust`] if no sample has positive thrust.
    pub fn new(times_s: Vec<f64>, thrusts_n: Vec<f64>) -> Result<Self, MotorError> {
        if times_s.len() != thrusts_n.len() || times_s.is_empty() {
            return Err(MotorError::Inconsistent(format!(
                "a thrust curve needs matching, non-empty columns; got {} times and {} thrusts",
                times_s.len(),
                thrusts_n.len()
            )));
        }
        for (&t, &f) in times_s.iter().zip(&thrusts_n) {
            if !(t.is_finite() && t >= 0.0) {
                return Err(MotorError::Domain {
                    what: "thrust-curve time (s)",
                    value: t,
                });
            }
            if !(f.is_finite() && f >= 0.0) {
                return Err(MotorError::Domain {
                    what: "thrust-curve thrust (N)",
                    value: f,
                });
            }
        }
        if let Some(index) = times_s.windows(2).position(|w| w[1] < w[0]) {
            return Err(MotorError::TimesDecreasing {
                index: index + 1,
                time_s: times_s[index + 1],
            });
        }
        if !thrusts_n.iter().any(|&f| f > 0.0) {
            return Err(MotorError::NoThrust);
        }
        let (mut times_s, mut thrusts_n) = (times_s, thrusts_n);
        if times_s[0] > 0.0 {
            times_s.insert(0, 0.0);
            thrusts_n.insert(0, 0.0);
        }
        let mut impulse_ns = Vec::with_capacity(times_s.len());
        let mut total = 0.0;
        impulse_ns.push(total);
        for i in 1..times_s.len() {
            total += 0.5 * (thrusts_n[i - 1] + thrusts_n[i]) * (times_s[i] - times_s[i - 1]);
            impulse_ns.push(total);
        }
        if !total.is_finite() {
            return Err(MotorError::Domain {
                what: "total impulse (N·s)",
                value: total,
            });
        }
        Ok(Self {
            times_s,
            thrusts_n,
            impulse_ns,
        })
    }

    /// The sample times, s, starting at 0 (including a prepended `(0, 0)`).
    pub fn times_s(&self) -> &[f64] {
        &self.times_s
    }

    /// The thrust at each sample time, N.
    pub fn thrusts_n(&self) -> &[f64] {
        &self.thrusts_n
    }

    /// The last sample time, s. The thrust is zero from here on.
    pub fn end_time_s(&self) -> f64 {
        self.times_s.last().copied().unwrap_or(0.0)
    }

    /// The largest sampled thrust, N.
    pub fn peak_thrust_n(&self) -> f64 {
        self.thrusts_n.iter().copied().fold(0.0, f64::max)
    }

    /// The total impulse `I = ∫ F dt`, N·s.
    pub fn total_impulse_ns(&self) -> f64 {
        self.impulse_ns.last().copied().unwrap_or(0.0)
    }

    /// The thrust at time `t` (s after ignition), N: linear between samples, zero before ignition
    /// and after the last sample. A NaN time gives NaN.
    pub fn thrust_n(&self, t: f64) -> f64 {
        if t.is_nan() {
            return f64::NAN;
        }
        match self.segment(t) {
            Some(i) => {
                let (t0, t1) = (self.times_s[i], self.times_s[i + 1]);
                let (f0, f1) = (self.thrusts_n[i], self.thrusts_n[i + 1]);
                f0 + (f1 - f0) * (t - t0) / (t1 - t0)
            }
            None if t == self.end_time_s() => self.thrusts_n.last().copied().unwrap_or(0.0),
            None => 0.0,
        }
    }

    /// The impulse delivered by time `t`, `I(t) = ∫₀ᵗ F dt`, N·s: exact for the piecewise-linear
    /// thrust, zero before ignition and the total impulse after the last sample. A NaN time gives
    /// NaN.
    pub fn impulse_ns(&self, t: f64) -> f64 {
        if t.is_nan() {
            return f64::NAN;
        }
        if t <= 0.0 {
            return 0.0;
        }
        match self.segment(t) {
            Some(i) => {
                let (t0, t1) = (self.times_s[i], self.times_s[i + 1]);
                let (f0, f1) = (self.thrusts_n[i], self.thrusts_n[i + 1]);
                let dt = t - t0;
                let f = f0 + (f1 - f0) * dt / (t1 - t0);
                self.impulse_ns[i] + 0.5 * (f0 + f) * dt
            }
            None => self.total_impulse_ns(),
        }
    }

    /// The NFPA 1125 burn window `(start, end)`, s: when the thrust first reaches, and last falls
    /// to, [`NFPA_1125_THRESHOLD`] of its peak, interpolated on the straight segments.
    pub fn burn_window_s(&self) -> (f64, f64) {
        let level = NFPA_1125_THRESHOLD * self.peak_thrust_n();
        let n = self.times_s.len();
        // The peak is positive (checked in `new`), so some sample reaches the level, and a curve
        // starting at zero thrust crosses it on a segment.
        let first = self.thrusts_n.iter().position(|&f| f >= level).unwrap_or(0);
        let start = if first == 0 {
            self.times_s[0]
        } else {
            self.crossing(first - 1, level)
        };
        let last = self
            .thrusts_n
            .iter()
            .rposition(|&f| f >= level)
            .unwrap_or(n - 1);
        let end = if last == n - 1 {
            self.times_s[n - 1]
        } else {
            self.crossing(last, level)
        };
        (start, end)
    }

    /// The NFPA 1125 burn time, s: the length of [`ThrustCurve::burn_window_s`].
    pub fn burn_time_s(&self) -> f64 {
        let (start, end) = self.burn_window_s();
        end - start
    }

    /// The average thrust, N: the total impulse over the NFPA 1125 burn time.
    pub fn average_thrust_n(&self) -> f64 {
        self.total_impulse_ns() / self.burn_time_s()
    }

    /// The index `i` of the segment `[t_i, t_{i+1})` holding `t`, or `None` outside
    /// `[0, end_time)`. Zero-width segments (steps) never hold a time: the last sample at or
    /// before `t` starts the segment.
    fn segment(&self, t: f64) -> Option<usize> {
        if !(t >= 0.0 && t < self.end_time_s()) {
            return None;
        }
        // The first sample is at 0 <= t, so at least one time is <= t.
        Some(self.times_s.partition_point(|&ti| ti <= t) - 1)
    }

    /// The time on segment `i` where the thrust equals `level`, which lies between its ends.
    fn crossing(&self, i: usize, level: f64) -> f64 {
        let (t0, t1) = (self.times_s[i], self.times_s[i + 1]);
        let (f0, f1) = (self.thrusts_n[i], self.thrusts_n[i + 1]);
        if f1 == f0 {
            return t0;
        }
        (t0 + (t1 - t0) * (level - f0) / (f1 - f0)).clamp(t0, t1)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn triangle() -> ThrustCurve {
        ThrustCurve::new(vec![0.0, 1.0, 2.0], vec![0.0, 100.0, 0.0]).unwrap()
    }

    #[test]
    fn triangle_impulse_burn_time_and_average() {
        let curve = triangle();
        assert_eq!(curve.total_impulse_ns(), 100.0);
        assert_eq!(curve.peak_thrust_n(), 100.0);
        assert_eq!(curve.thrust_n(0.5), 50.0);
        assert_eq!(curve.thrust_n(1.5), 50.0);
        assert_eq!(curve.impulse_ns(1.0), 50.0);
        assert_eq!(curve.impulse_ns(0.5), 12.5);
        assert_eq!(curve.impulse_ns(1.5), 87.5);
        // 5% of the 100 N peak is reached at 0.05 s and left at 1.95 s.
        let (start, end) = curve.burn_window_s();
        assert!((start - 0.05).abs() < 1e-15 && (end - 1.95).abs() < 1e-15);
        assert!((curve.burn_time_s() - 1.9).abs() < 1e-15);
        assert!((curve.average_thrust_n() - 100.0 / 1.9).abs() < 1e-12);
    }

    #[test]
    fn thrust_is_zero_outside_the_curve_and_nan_propagates() {
        let curve = triangle();
        assert_eq!(curve.thrust_n(-1.0), 0.0);
        assert_eq!(curve.thrust_n(2.0), 0.0);
        assert_eq!(curve.thrust_n(3.0), 0.0);
        assert_eq!(curve.impulse_ns(-1.0), 0.0);
        assert_eq!(curve.impulse_ns(5.0), 100.0);
        assert!(curve.thrust_n(f64::NAN).is_nan());
        assert!(curve.impulse_ns(f64::NAN).is_nan());
    }

    #[test]
    fn late_first_sample_starts_from_an_implicit_origin() {
        let curve = ThrustCurve::new(vec![0.5, 1.0], vec![10.0, 0.0]).unwrap();
        assert_eq!(curve.times_s(), &[0.0, 0.5, 1.0]);
        assert_eq!(curve.thrusts_n(), &[0.0, 10.0, 0.0]);
        assert_eq!(curve.total_impulse_ns(), 5.0);
        assert_eq!(curve.thrust_n(0.25), 5.0);
    }

    #[test]
    fn thrust_at_ignition_starts_the_burn_window_at_zero() {
        let curve = ThrustCurve::new(vec![0.0, 1.0, 1.1], vec![20.0, 20.0, 0.0]).unwrap();
        let (start, end) = curve.burn_window_s();
        assert_eq!(start, 0.0);
        assert!((end - 1.095).abs() < 1e-12);
        // A curve that ends above 5% of peak ends its burn at the last sample.
        let cut = ThrustCurve::new(vec![0.0, 1.0], vec![20.0, 20.0]).unwrap();
        assert_eq!(cut.burn_window_s(), (0.0, 1.0));
        assert_eq!(cut.thrust_n(1.0), 20.0);
    }

    #[test]
    fn equal_times_make_a_step() {
        // A plateau that stops dead at 1 s, then a step up and a ramp down.
        let curve = ThrustCurve::new(
            vec![0.0, 1.0, 1.0, 2.0, 2.0, 3.0],
            vec![10.0, 10.0, 0.0, 0.0, 6.0, 0.0],
        )
        .unwrap();
        assert_eq!(curve.thrust_n(0.999), 10.0);
        assert_eq!(curve.thrust_n(1.0), 0.0);
        assert_eq!(curve.thrust_n(1.5), 0.0);
        assert_eq!(curve.thrust_n(2.0), 6.0);
        assert_eq!(curve.thrust_n(2.5), 3.0);
        assert_eq!(curve.total_impulse_ns(), 13.0);
        assert_eq!(curve.impulse_ns(1.0), 10.0);
        assert_eq!(curve.impulse_ns(2.0), 10.0);
        let (start, end) = curve.burn_window_s();
        assert_eq!(start, 0.0);
        // 5% of 10 N is 0.5 N, last passed on the final ramp at 3 − 0.5/6 s.
        assert!((end - (3.0 - 0.5 / 6.0)).abs() < 1e-12);
        // A step down at the end holds its last value only at that instant.
        let cut = ThrustCurve::new(vec![0.0, 1.0, 1.0], vec![10.0, 10.0, 0.0]).unwrap();
        assert_eq!(cut.thrust_n(1.0), 0.0);
        assert_eq!(cut.burn_window_s(), (0.0, 1.0));
    }

    #[test]
    fn rejects_bad_samples() {
        let bad = [
            (vec![], vec![]),
            (vec![0.0, 1.0], vec![1.0]),
            (vec![0.0, 2.0, 1.0], vec![0.0, 5.0, 0.0]),
            (vec![-0.1, 1.0], vec![5.0, 0.0]),
            (vec![0.0, 1.0], vec![-5.0, 0.0]),
            (vec![0.0, f64::NAN], vec![5.0, 0.0]),
            (vec![0.0, 1.0], vec![f64::INFINITY, 0.0]),
            (vec![0.0, 1.0], vec![0.0, 0.0]),
        ];
        for (times, thrusts) in bad {
            assert!(
                ThrustCurve::new(times.clone(), thrusts.clone()).is_err(),
                "{times:?} {thrusts:?}"
            );
        }
        assert_eq!(
            ThrustCurve::new(vec![0.0, 2.0, 1.0], vec![0.0, 5.0, 0.0]),
            Err(MotorError::TimesDecreasing {
                index: 2,
                time_s: 1.0
            })
        );
    }

    #[test]
    fn serde_round_trips_and_rechecks() {
        let curve = ThrustCurve::new(vec![0.1, 0.3, 1.7], vec![12.5, 30.25, 0.0]).unwrap();
        let json = serde_json::to_string(&curve).unwrap();
        assert_eq!(serde_json::from_str::<ThrustCurve>(&json).unwrap(), curve);
        let bad = r#"{"times_s":[0.0,1.0],"thrusts_n":[0.0,0.0]}"#;
        assert!(serde_json::from_str::<ThrustCurve>(bad).is_err());
    }

    fn curves() -> impl Strategy<Value = ThrustCurve> {
        prop::collection::vec((1e-3..1.0f64, 0.0..5000.0f64), 1..40).prop_filter_map(
            "needs positive thrust",
            |steps| {
                let mut t = 0.0;
                let (times, thrusts): (Vec<f64>, Vec<f64>) = steps
                    .into_iter()
                    .map(|(dt, f)| {
                        t += dt;
                        (t, f)
                    })
                    .unzip();
                ThrustCurve::new(times, thrusts).ok()
            },
        )
    }

    proptest! {
        #[test]
        fn impulse_integrates_thrust_exactly(curve in curves(), u in 0.0..1.0f64, v in 0.0..1.0f64) {
            let end = curve.end_time_s();
            let (a, b) = if u < v { (u * end, v * end) } else { (v * end, u * end) };
            let (ia, ib) = (curve.impulse_ns(a), curve.impulse_ns(b));
            prop_assert!(ib >= ia);
            // Simpson's rule is exact for the linear pieces, so split [a, b] at the samples.
            let mut knots = vec![a];
            knots.extend(curve.times_s().iter().copied().filter(|&t| t > a && t < b));
            knots.push(b);
            let quadrature: f64 = knots
                .windows(2)
                .map(|w| {
                    let m = 0.5 * (w[0] + w[1]);
                    (w[1] - w[0]) / 6.0
                        * (curve.thrust_n(w[0]) + 4.0 * curve.thrust_n(m) + right(&curve, w[1]))
                })
                .sum();
            let scale = curve.total_impulse_ns().max(1e-9);
            prop_assert!((ib - ia - quadrature).abs() <= 1e-9 * scale);
        }

        #[test]
        fn burn_window_is_inside_the_curve(curve in curves()) {
            let (start, end) = curve.burn_window_s();
            prop_assert!(0.0 <= start && start <= end && end <= curve.end_time_s());
            let level = NFPA_1125_THRESHOLD * curve.peak_thrust_n();
            prop_assert!(curve.times_s().iter().zip(curve.thrusts_n())
                .all(|(&t, &f)| f < level || (start <= t && t <= end)));
        }
    }

    /// The thrust approaching `t` from the left, which differs from [`ThrustCurve::thrust_n`] only
    /// at the last sample.
    fn right(curve: &ThrustCurve, t: f64) -> f64 {
        if t >= curve.end_time_s() {
            curve.thrusts_n().last().copied().unwrap_or(0.0)
        } else {
            curve.thrust_n(t)
        }
    }
}
