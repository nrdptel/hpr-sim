//! An invented flight, written as a PerfectFlite `.pf2`, whose every reading is known in closed
//! form: `validation/fixtures/logs/synthetic-pnut.pf2`, which the command-line tests and the guide
//! read. No real flight's data is in it.
//!
//! The flight: half a second on the pad; 1.6 s of constant 50 m/s² upward acceleration; a coast
//! with no drag, decelerating at `g`, to apogee; a fall from rest until 25 m/s; a drogue's steady
//! 25 m/s down to 150 m; a main's 6 m/s to the ground; 3 s on the ground. One second after apogee
//! an ejection charge's pulse moves seven samples by −8, +1, +16, +15, +2, −3 and −3 m, roughly
//! the shape of the pulse on the public Pnut log Debrief ships. Sampled every
//! 0.05 s, rounded as a Pnut rounds: whole feet, whole feet per second.

use hpr_core::gravity::STANDARD_GRAVITY_MPS2 as G;

use crate::perfectflite::FOOT_M;

/// Where the file is committed, from the repository's root.
pub(crate) const PATH: &str = "validation/fixtures/logs/synthetic-pnut.pf2";

/// The sample interval, s.
pub(crate) const DT: f64 = 0.05;
/// Liftoff, s.
pub(crate) const LIFTOFF_S: f64 = 0.5;
const BOOST_M_S2: f64 = 50.0;
const BOOST_S: f64 = 1.6;
const DROGUE_M_S: f64 = 25.0;
const MAIN_M: f64 = 150.0;
const MAIN_M_S: f64 = 6.0;
const ON_THE_GROUND_S: f64 = 3.0;
/// The ejection pulse: how far after apogee, and each sample's offset from the trace, m.
const PULSE_AFTER_APOGEE_S: f64 = 1.0;
const PULSE_M: [f64; 7] = [-8.0, 1.0, 16.0, 15.0, 2.0, -3.0, -3.0];

/// The flight's events, exact.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Truth {
    pub(crate) burnout_s: f64,
    pub(crate) burnout_speed_m_s: f64,
    pub(crate) burnout_m: f64,
    pub(crate) apogee_s: f64,
    pub(crate) apogee_m: f64,
    drogue_s: f64,
    drogue_m: f64,
    main_s: f64,
    pub(crate) landing_s: f64,
}

pub(crate) fn truth() -> Truth {
    let burnout_s = LIFTOFF_S + BOOST_S;
    let burnout_speed_m_s = BOOST_M_S2 * BOOST_S;
    let burnout_m = 0.5 * BOOST_M_S2 * BOOST_S * BOOST_S;
    let apogee_s = burnout_s + burnout_speed_m_s / G;
    let apogee_m = burnout_m + burnout_speed_m_s * burnout_speed_m_s / (2.0 * G);
    let drogue_s = apogee_s + DROGUE_M_S / G;
    let drogue_m = apogee_m - DROGUE_M_S * DROGUE_M_S / (2.0 * G);
    let main_s = drogue_s + (drogue_m - MAIN_M) / DROGUE_M_S;
    let landing_s = main_s + MAIN_M / MAIN_M_S;
    Truth {
        burnout_s,
        burnout_speed_m_s,
        burnout_m,
        apogee_s,
        apogee_m,
        drogue_s,
        drogue_m,
        main_s,
        landing_s,
    }
}

/// The height, m, and the vertical speed, m/s, at `t`.
pub(crate) fn state(truth: &Truth, t: f64) -> (f64, f64) {
    if t <= LIFTOFF_S {
        (0.0, 0.0)
    } else if t <= truth.burnout_s {
        let s = t - LIFTOFF_S;
        (0.5 * BOOST_M_S2 * s * s, BOOST_M_S2 * s)
    } else if t <= truth.drogue_s {
        let s = t - truth.burnout_s;
        (
            truth.burnout_m + truth.burnout_speed_m_s * s - 0.5 * G * s * s,
            truth.burnout_speed_m_s - G * s,
        )
    } else if t <= truth.main_s {
        (
            truth.drogue_m - DROGUE_M_S * (t - truth.drogue_s),
            -DROGUE_M_S,
        )
    } else if t <= truth.landing_s {
        (MAIN_M - MAIN_M_S * (t - truth.main_s), -MAIN_M_S)
    } else {
        (0.0, 0.0)
    }
}

/// The sample index at `t`, s.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a positive time of under a minute, in samples"
)]
fn index(t: f64) -> usize {
    (t / DT).round() as usize
}

/// The file's text.
pub(crate) fn text() -> String {
    let truth = truth();
    let samples = index(truth.landing_s + ON_THE_GROUND_S) + 1;
    let pulse = index(truth.apogee_s + PULSE_AFTER_APOGEE_S);
    let mut rows = String::new();
    for i in 0..samples {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a sample count of about a thousand"
        )]
        let t = i as f64 * DT;
        let (mut height, speed) = state(&truth, t);
        if let Some(lift) = i.checked_sub(pulse).and_then(|k| PULSE_M.get(k)) {
            height += lift;
        }
        let feet = (height / FOOT_M).round();
        let feet_s = (speed / FOOT_M).round();
        // Every fourth row leaves the temperature and voltage out, as a Pnut's slower channels do.
        if i % 4 == 3 {
            rows.push_str(&format!("{t:.2}, {feet}, {feet_s}\r\n"));
        } else {
            rows.push_str(&format!("{t:.2}, {feet}, {feet_s}, 70.00, 4.20\r\n"));
        }
    }
    format!(
        "PerfectFlite Pnut\r\nFirmware: 1.0\r\nSoftware: 1.1\r\nSerial Number: 0\r\n\
         Apogee: {}' AGL\r\nGround Elevation: 600' MSL\r\nNumSamps: {samples}\r\n\
         Flight Number: 1\r\n\
         Comments: invented for hpr-sim's tests; no real flight's data\r\n\r\n\
         Data: (Time, Altitude, Velocity, Temperature (F), Voltage)\r\n{rows}",
        (truth.apogee_m / FOOT_M).round()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readings::{self, LIFTOFF_HEIGHT_M, Reason, Source};
    use crate::{log::FlightLog, perfectflite};

    fn committed() -> &'static str {
        include_str!("../../../validation/fixtures/logs/synthetic-pnut.pf2")
    }

    /// The committed file is what [`text`] writes. To write it again, run this test with
    /// `HPR_WRITE_SYNTHETIC_LOG=1`.
    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "a test that writes its own fixture when asked, as `cargo xtask` outputs are"
    )]
    fn the_committed_file_is_current() {
        let text = text();
        if std::env::var_os("HPR_WRITE_SYNTHETIC_LOG").is_some() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(PATH);
            std::fs::write(path, &text).unwrap();
        }
        assert!(
            committed() == text,
            "{PATH} is stale: run `HPR_WRITE_SYNTHETIC_LOG=1 cargo test -p hpr-flightdata \
             the_committed_file_is_current`"
        );
    }

    fn log() -> FlightLog {
        perfectflite::read(committed()).unwrap()
    }

    /// The flight's own numbers, before any reading: the file says what it should.
    #[test]
    fn the_file_holds_the_invented_flight() {
        let truth = truth();
        let log = log();
        assert!(log.notes.is_empty(), "{:?}", log.notes);
        assert_eq!(log.stated.samples, Some(log.time_s.len()));
        assert_eq!(
            log.stated.apogee_m,
            Some((truth.apogee_m / FOOT_M).round() * FOOT_M)
        );
        // 80 m/s at burnout, 2.1 s; apogee 390.31 m at 10.26 s; landing at 46.14 s.
        assert_eq!(truth.burnout_speed_m_s, 80.0);
        assert!(
            (truth.apogee_m - 390.309_188).abs() < 1e-6,
            "{}",
            truth.apogee_m
        );
        assert!(
            (truth.apogee_s - 10.257_730).abs() < 1e-6,
            "{}",
            truth.apogee_s
        );
        assert!(
            (truth.landing_s - 46.144_742).abs() < 1e-6,
            "{}",
            truth.landing_s
        );
        // Continuous at every joint.
        for t in [
            truth.burnout_s,
            truth.drogue_s,
            truth.main_s,
            truth.landing_s,
        ] {
            let (before, _) = state(&truth, t - 1e-9);
            let (after, _) = state(&truth, t + 1e-9);
            assert!(
                (before - after).abs() < 1e-6,
                "{t}: {before} against {after}"
            );
        }
    }

    /// Debrief's Hampel filter (0.3 s, threshold 4) keeps the pulse, as it keeps the public Pnut
    /// log's: the dip before it widens its window's spread. The running median takes it out.
    #[test]
    fn a_hampel_filter_keeps_the_pulse_and_the_median_takes_it_out() {
        let log = log();
        let highest = |trace: &[f64]| trace.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let hampel = crate::filter::hampel(&log.altitude_m, 3, 4.0);
        assert_eq!(highest(&hampel), highest(&log.altitude_m));
        let median = crate::filter::running_median(&log.altitude_m, 3);
        assert!(highest(&median) < truth().apogee_m + 0.5 * FOOT_M);
    }

    /// Each reading against the invented flight, to within what the file's rounding and the
    /// filter allow, each bound worked out below.
    #[test]
    fn readings_recover_the_invented_flight() {
        let truth = truth();
        let log = log();
        let read = readings::read(&log);
        let half_foot = 0.5 * FOOT_M;
        assert!((read.sample_interval_s.unwrap() - DT).abs() < 1e-12);
        assert_eq!(read.pad_altitude_m, Some(0.0));

        // Liftoff: the last sample at 0 ft. The rocket rises past half a foot, and rounds to
        // 1 ft, `√(2 · 0.1524 / 50)` = 0.078 s after liftoff: one sample.
        let liftoff = read.liftoff.value().unwrap();
        assert_eq!(liftoff.source, Source::Barometer);
        let rounds_up = (2.0 * half_foot / BOOST_M_S2).sqrt();
        assert!(
            liftoff.time_s >= LIFTOFF_S - DT && liftoff.time_s <= LIFTOFF_S + rounds_up,
            "{}",
            liftoff.time_s
        );

        // Apogee: at most half a foot of rounding plus the median's `g (3 · Δt)² / 2` low, and
        // never above the rounded peak.
        let apogee = read.apogee.value().unwrap();
        let bound = half_foot + 0.5 * G * (3.0 * DT) * (3.0 * DT);
        assert!(
            (apogee.altitude_m - truth.apogee_m).abs() <= bound,
            "{} against {} ± {bound}",
            apogee.altitude_m,
            truth.apogee_m
        );
        // Within the span over which the rounded trace can sit at its top: the heights within
        // 1 ft of the peak, `√(2 · 0.3048 / g)` = 0.25 s either side.
        let plateau = (2.0 * FOOT_M / G).sqrt();
        assert!(
            (apogee.time_s - truth.apogee_s).abs() <= plateau,
            "{}",
            apogee.time_s
        );
        assert!(!apogee.is_floor);
        assert_eq!(
            apogee.time_after_liftoff_s,
            Some(apogee.time_s - liftoff.time_s)
        );
        // The pulse's third sample is the highest the file holds, and the median set it aside.
        let pulse_s = ((index(truth.apogee_s + PULSE_AFTER_APOGEE_S) + 2) as f64) * DT;
        assert!((apogee.highest_sample.time_s - pulse_s).abs() < 1e-9);
        assert!(apogee.highest_sample.altitude_m > apogee.altitude_m + 5.0);

        // The top speed: burnout's 80 m/s, to half a foot per second.
        let speed = read.max_speed.value().unwrap();
        assert_eq!(speed.source, Source::LoggerSpeedFromBarometer);
        assert!((speed.speed_m_s - truth.burnout_speed_m_s).abs() <= half_foot);
        assert!(
            (speed.time_s - truth.burnout_s).abs() < 1e-9,
            "{}",
            speed.time_s
        );

        // No accelerometer: withheld.
        assert!(matches!(
            &read.max_acceleration,
            readings::Reading::Withheld(w) if w.reason == Reason::NoAccelerometer
        ));

        // Landing: the first sample below 2 m, on the main at 6 m/s: at most 2/6 s and one
        // sample before touchdown.
        let landing = read.landing.value().unwrap();
        let early = readings::LANDING_HEIGHT_M / MAIN_M_S + DT;
        assert!(
            landing.time_s <= truth.landing_s && landing.time_s >= truth.landing_s - early,
            "{} against {}",
            landing.time_s,
            truth.landing_s
        );
        assert_eq!(landing.flight_time_s, Some(landing.time_s - liftoff.time_s));
        assert_eq!(landing.descent_time_s, landing.time_s - apogee.time_s);
        // The mean rate: the true heights at the two readings' times, over the same span, to
        // within the heights' own bounds.
        let (true_top, _) = state(&truth, apogee.time_s);
        let (true_bottom, _) = state(&truth, landing.time_s);
        let expected = (true_top - true_bottom) / landing.descent_time_s;
        let slack = 2.0 * bound / landing.descent_time_s;
        assert!(
            (landing.mean_descent_rate_m_s - expected).abs() <= slack,
            "{} against {expected}",
            landing.mean_descent_rate_m_s
        );
        assert!(LIFTOFF_HEIGHT_M < truth.apogee_m);
    }
}
