//! The readings taken from a flight log on its own: liftoff, apogee, the top speed, landing and
//! the descent, with no design file and no simulation.
//!
//! Each is a [`Reading`]: a value with where it came from, or [`Reading::Withheld`] with the reason
//! the log can't support it. A reading is never guessed at; one the log can't support is left out
//! and says why.
//!
//! **Method.** Every reading of a height or a time comes from the altitude after a running median
//! over [`MEDIAN_WINDOW_S`] ([`crate::filter::running_median`]), which takes out the pressure
//! pulse an ejection charge punches into a barometric trace. Then:
//!
//! - the **climb** begins at the first sample [`LIFTOFF_HEIGHT_M`] above where the log starts;
//! - the **pad** is the median of the altitude before it first rises [`PAD_RISE_M`], which must be
//!   within [`LIFTOFF_HEIGHT_M`] of the logger's zero, as a logger that zeroes itself on the pad
//!   records;
//! - **liftoff** is the last sample before the climb within half the altitude's resolution of the
//!   pad: the rocket had risen less than that then, and more within one sample after;
//! - **apogee** is the filtered altitude's highest value, at the middle of the run of samples
//!   that hold it (the altitude's resolution leaves a peak flat for a few samples);
//! - the **top speed** is the highest of the logger's own vertical speed from liftoff to apogee,
//!   refused as Debrief refuses one ([`IMPLAUSIBLE_SPEED_M_S`], [`ASCENT_NOISE_FRACTION`], and a
//!   peak on the liftoff sample itself);
//! - **landing** is the first sample after apogee below [`LANDING_HEIGHT_M`] above the pad that
//!   stays under [`LANDED_CEILING_M`] for [`LANDED_FOR_S`], and no sooner, give or take a sample,
//!   than a fall from rest in vacuum would lose that height, `√(2h/g)`: drag only slows a fall;
//! - the **top acceleration** is withheld when the log has no accelerometer: differencing an
//!   altitude twice turns its resolution into spikes of many g.
//!
//! The thresholds are Debrief's (`lib/analyze/index.ts`, MIT, the project owner's own), set on its
//! corpus of flight logs rather than taken from a published source; the running median in place of
//! Debrief's Hampel filter is hpr's, for the reason on [`MEDIAN_WINDOW_S`].

use hpr_core::gravity::STANDARD_GRAVITY_MPS2;
use serde::{Deserialize, Serialize};

use crate::filter::{median, running_median};
use crate::log::{FlightLog, LogFormat};

/// The running median's span, s: 0.3 s, Debrief's despiking window. Debrief runs a Hampel filter
/// over it ([`crate::filter::hampel`], threshold 4); hpr takes the plain median (threshold 0). On
/// the public Pnut log Debrief ships, the ejection charge's pulse peaks at 1,028 ft, against the
/// 1,009 ft apogee the logger states. The samples around the pulse, a dip before it and a lasting
/// drop after it, widen its window's spread until the Hampel filter keeps it, and an apogee read
/// after it is the pulse. After the median it reads 1,010 ft. The median removes any pulse up to
/// half its window wide, and reads a noise-free peak bent by gravity alone no more than
/// [`peak_bound_m`] low, 0.077 m at 20 Hz. The Pnut numbers are checked only where the log has been
/// fetched into `refs/` (it isn't committed); CI checks an invented log of the same shape.
pub const MEDIAN_WINDOW_S: f64 = 0.3;
/// The most samples either side of its centre the running median takes: hpr's choice, the
/// [`MEDIAN_WINDOW_S`] window at over 6 kHz, faster than any logger hpr reads. A log sampled
/// faster is withheld ([`Reason::SampledTooFast`]): the median's cost grows with its window.
pub const MAX_MEDIAN_HALF_WINDOW: usize = 1000;
/// The climb above the pad that marks a flight, m (Debrief's 3 m).
pub const LIFTOFF_HEIGHT_M: f64 = 3.0;
/// The pad is the median of the samples before the altitude first rises this far above where the
/// log starts, m: hpr's choice, a third of [`LIFTOFF_HEIGHT_M`], so that a log which begins just
/// before liftoff lends the pad few samples that are already climbing.
pub const PAD_RISE_M: f64 = 1.0;
/// How close to the pad the altitude must come to mark landing, m (Debrief's 2 m).
pub const LANDING_HEIGHT_M: f64 = 2.0;
/// The height above the pad the altitude must then stay under, m (Debrief's 5 m)…
pub const LANDED_CEILING_M: f64 = 5.0;
/// …and for how long, s (Debrief's 1 s).
pub const LANDED_FOR_S: f64 = 1.0;
/// A top speed above this is refused, m/s (Debrief's ceiling).
pub const IMPLAUSIBLE_SPEED_M_S: f64 = 4000.0;
/// A top speed is refused when the climb's most negative speed is more than this share of it: a
/// trace swinging that far has no usable sign (Debrief's 20%).
pub const ASCENT_NOISE_FRACTION: f64 = 0.2;

/// A reading, or why the log can't support it.
///
/// Serialized, the status goes in beside the reading's own fields (`"status": "read"`), so `T`
/// must serialize as a map, as the reading structs here do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Reading<T> {
    /// The reading, with where it came from.
    Read(T),
    /// The log can't support the reading.
    Withheld(Withheld),
}

impl<T> Reading<T> {
    /// The reading, if it wasn't withheld.
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Read(value) => Some(value),
            Self::Withheld(_) => None,
        }
    }

    fn withheld(reason: Reason, detail: impl Into<String>) -> Self {
        Self::Withheld(Withheld {
            reason,
            detail: detail.into(),
        })
    }
}

/// Why a reading was withheld.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Withheld {
    /// The reason, as a code.
    pub reason: Reason,
    /// The reason in words, with the log's own numbers.
    pub detail: String,
}

/// Declares a fieldless enum and its `ALL`, every variant in the order declared, from one list,
/// so that a new variant can't be left out of `ALL`.
macro_rules! with_all {
    ($(#[$meta:meta])* pub enum $name:ident { $($(#[$variant_meta:meta])* $variant:ident,)* }) => {
        $(#[$meta])*
        pub enum $name {
            $($(#[$variant_meta])* $variant,)*
        }

        impl $name {
            /// Every variant, in the order declared: what a program that maps them, as the
            /// command line does, checks itself against.
            pub const ALL: &'static [Self] = &[$(Self::$variant),*];
        }
    };
}

with_all! {
/// The reason a reading was withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Reason {
    /// The log has fewer than three samples.
    TooShort,
    /// The altitude never climbs [`LIFTOFF_HEIGHT_M`] above where the log starts.
    NoClimb,
    /// The pad, the median altitude before the first [`PAD_RISE_M`] of rise, is more than
    /// [`LIFTOFF_HEIGHT_M`] from the logger's zero: the log didn't start on the pad.
    StartsOffThePad,
    /// The log ends before the rocket is seen to land.
    EndsBeforeLanding,
    /// The altitude reaches the ground sooner than a fall from rest at apogee in vacuum could.
    FasterThanFreeFall,
    /// The log has no speed column.
    NoSpeedColumn,
    /// The top speed is above [`IMPLAUSIBLE_SPEED_M_S`].
    ImplausibleSpeed,
    /// The climb's speed swings negative by more than [`ASCENT_NOISE_FRACTION`] of its top.
    NoisySpeed,
    /// The top speed falls on the liftoff sample itself: a spike, not a climb.
    SpeedPeakAtLiftoff,
    /// The log has no accelerometer.
    NoAccelerometer,
    /// The reading needs another, which was withheld.
    Needs,
    /// The record breaks what every reader guarantees: channels as long as the clock, and finite
    /// times that increase. Only a record built by hand can.
    BadRecord,
    /// The samples come so often that the [`MEDIAN_WINDOW_S`] window would hold more than
    /// [`MAX_MEDIAN_HALF_WINDOW`] either side.
    SampledTooFast,
}
}

with_all! {
/// Where a reading's value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Source {
    /// The logger's barometric altitude, after the running median.
    Barometer,
    /// A speed column the logger computed from its own barometric altitude.
    LoggerSpeedFromBarometer,
}
}

/// Liftoff.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Liftoff {
    /// The last sample on the pad, s on the log's clock: the filtered altitude was within half
    /// the altitude's resolution of the pad then, and rose past it within one sample interval
    /// after.
    pub time_s: f64,
    /// Where it came from.
    pub source: Source,
}

/// The highest point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Apogee {
    /// When, s on the log's clock: the middle of the run of samples at the filtered altitude's
    /// highest value.
    pub time_s: f64,
    /// When, s after liftoff; `None` if liftoff was withheld.
    pub time_after_liftoff_s: Option<f64>,
    /// The filtered altitude there, m above the logger's zero.
    pub altitude_m: f64,
    /// Whether the peak lies within half the median's window of the log's end, so the log may
    /// have stopped before the rocket did: the altitude is then a floor.
    pub is_floor: bool,
    /// The highest sample the log holds before the filter: above the apogee when the median set a
    /// pulse aside.
    pub highest_sample: Sample,
    /// Where it came from.
    pub source: Source,
}

/// One sample of the altitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// When, s on the log's clock.
    pub time_s: f64,
    /// The altitude, m above the logger's zero.
    pub altitude_m: f64,
}

/// The top vertical speed in the climb.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaxSpeed {
    /// The speed, m/s, up.
    pub speed_m_s: f64,
    /// When, s on the log's clock.
    pub time_s: f64,
    /// The filtered altitude then, m above the logger's zero.
    pub altitude_m: f64,
    /// Where it came from.
    pub source: Source,
}

/// The top acceleration in the climb. No reader fills it yet: the one format read so far has no
/// accelerometer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MaxAcceleration {
    /// The acceleration, m/s².
    pub acceleration_m_s2: f64,
    /// When, s on the log's clock.
    pub time_s: f64,
}

/// Landing, and the descent before it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landing {
    /// The first sample within [`LANDING_HEIGHT_M`] of the pad, s on the log's clock: before
    /// touchdown by the time the last 2 m took.
    pub time_s: f64,
    /// From liftoff to landing, s.
    pub flight_time_s: f64,
    /// From apogee to landing, s.
    pub descent_time_s: f64,
    /// The mean rate of descent from apogee to landing, m/s: the height lost over the time taken,
    /// drogue and main together.
    pub mean_descent_rate_m_s: f64,
    /// Where it came from.
    pub source: Source,
}

/// What was read from a flight log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Readings {
    /// The median interval between samples, s; `None` for a log too short to read, or withheld
    /// whole as [`Reason::BadRecord`].
    pub sample_interval_s: Option<f64>,
    /// The running median's span, s, whole samples of the interval: [`MEDIAN_WINDOW_S`] rounded.
    pub median_window_s: Option<f64>,
    /// How far below its true peak the running median can read a peak bent by gravity alone, m
    /// ([`peak_bound_m`]).
    pub peak_bound_m: Option<f64>,
    /// The pad: the median of the altitude before it first rises [`PAD_RISE_M`], m above the
    /// logger's zero; `None` when every reading is withheld, [`Reason::NoClimb`] included.
    pub pad_altitude_m: Option<f64>,
    /// Liftoff.
    pub liftoff: Reading<Liftoff>,
    /// The highest point.
    pub apogee: Reading<Apogee>,
    /// The top vertical speed from liftoff to apogee.
    pub max_speed: Reading<MaxSpeed>,
    /// The top acceleration.
    pub max_acceleration: Reading<MaxAcceleration>,
    /// Landing, and the descent before it.
    pub landing: Reading<Landing>,
}

/// Takes the readings from a flight log.
pub fn read(log: &FlightLog) -> Readings {
    let max_acceleration = no_accelerometer(log);
    let every = |reason: Reason, detail: &str| Readings {
        sample_interval_s: None,
        median_window_s: None,
        peak_bound_m: None,
        pad_altitude_m: None,
        liftoff: Reading::withheld(reason, detail),
        apogee: Reading::withheld(reason, detail),
        max_speed: Reading::withheld(reason, detail),
        max_acceleration: max_acceleration.clone(),
        landing: Reading::withheld(reason, detail),
    };
    if let Err(detail) = check_record(log) {
        return every(Reason::BadRecord, &detail);
    }
    let time = &log.time_s;
    let n = time.len();
    let mut steps: Vec<f64> = time.windows(2).map(|pair| pair[1] - pair[0]).collect();
    let (Some(interval), true) = (median(&mut steps), n >= 3) else {
        return every(
            Reason::TooShort,
            &format!("the log has {n} samples, too few to take a reading from"),
        );
    };
    let half = half_window(interval);
    if half > MAX_MEDIAN_HALF_WINDOW {
        let mut readings = every(
            Reason::SampledTooFast,
            &format!(
                "the log's samples come every {interval:.3e} s, so a {MEDIAN_WINDOW_S} s running \
                 median would take more than {MAX_MEDIAN_HALF_WINDOW} samples either side: faster \
                 than any logger hpr reads"
            ),
        );
        readings.sample_interval_s = Some(interval);
        return readings;
    }
    let filtered = running_median(&log.altitude_m, half);
    let bound = peak_bound_m(half, interval);
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window of a few samples converts exactly"
    )]
    let window_s = 2.0 * half as f64 * interval;
    let mut readings = every(Reason::TooShort, "");
    readings.sample_interval_s = Some(interval);
    readings.median_window_s = Some(window_s);
    readings.peak_bound_m = Some(bound);

    // The climb: the first sample 3 m above where the log starts.
    let start = filtered[0];
    let (Some(apogee_index), Some(climbed)) = (
        arg_max(&filtered),
        filtered.iter().position(|h| *h >= start + LIFTOFF_HEIGHT_M),
    ) else {
        let detail =
            format!("the altitude never climbs {LIFTOFF_HEIGHT_M} m above where the log starts");
        readings.liftoff = Reading::withheld(Reason::NoClimb, detail.clone());
        readings.apogee = Reading::withheld(Reason::NoClimb, detail.clone());
        readings.max_speed = Reading::withheld(Reason::NoClimb, detail.clone());
        readings.landing = Reading::withheld(Reason::NoClimb, detail);
        return readings;
    };
    // The pad: the median of the samples before the altitude first rises [`PAD_RISE_M`], so no
    // one sample's jitter sets it.
    let risen = filtered
        .iter()
        .position(|h| *h >= start + PAD_RISE_M)
        .unwrap_or(climbed);
    let mut before: Vec<f64> = log.altitude_m[..risen.max(1)]
        .iter()
        .copied()
        .filter(|h| h.is_finite())
        .collect();
    let pad = median(&mut before).unwrap_or(start);
    readings.pad_altitude_m = Some(pad);
    let top = filtered[apogee_index];

    let liftoff_index = if pad.abs() > LIFTOFF_HEIGHT_M {
        readings.liftoff = Reading::withheld(
            Reason::StartsOffThePad,
            format!(
                "the log starts {pad:.2} m from the logger's zero, more than {LIFTOFF_HEIGHT_M} m: \
                 it didn't start on the pad"
            ),
        );
        None
    } else {
        // Back from the climb to the last sample on the pad, to within half the altitude's
        // resolution.
        let level = pad + 0.5 * log.format.altitude_resolution_m();
        let index = filtered[..climbed]
            .iter()
            .rposition(|h| *h <= level)
            .unwrap_or(0);
        readings.liftoff = Reading::Read(Liftoff {
            time_s: time[index],
            source: Source::Barometer,
        });
        Some(index)
    };

    // The altitude's resolution leaves the peak flat over a run of samples: the apogee is the
    // run's middle.
    let run_end = apogee_index
        + filtered[apogee_index..]
            .iter()
            .take_while(|h| **h == top)
            .count()
        - 1;
    let apogee_s = 0.5 * (time[apogee_index] + time[run_end]);
    let highest = arg_max(&log.altitude_m).unwrap_or(apogee_index);
    readings.apogee = Reading::Read(Apogee {
        time_s: apogee_s,
        time_after_liftoff_s: liftoff_index.map(|index| apogee_s - time[index]),
        altitude_m: top,
        is_floor: run_end + half >= n - 1,
        highest_sample: Sample {
            time_s: time[highest],
            altitude_m: log.altitude_m[highest],
        },
        source: Source::Barometer,
    });

    let Some(liftoff) = liftoff_index else {
        readings.max_speed = Reading::withheld(
            Reason::Needs,
            "the top speed is taken from liftoff to apogee, and liftoff was withheld",
        );
        readings.landing = Reading::withheld(
            Reason::Needs,
            "landing is found against the pad, and the log didn't start on it",
        );
        return readings;
    };
    readings.max_speed = max_speed(log, &filtered, liftoff, apogee_index);
    readings.landing = landing(
        time,
        &filtered,
        pad,
        (apogee_index, apogee_s),
        (liftoff, interval),
        log.format.altitude_resolution_m(),
    );
    readings
}

/// Whole samples either side of the running median's centre: [`MEDIAN_WINDOW_S`] over two
/// intervals, rounded half up, at least one. At 20 Hz it is 3. A log's intervals are written to a
/// few decimals, so a rounding tie, such as 1.5 at 10 Hz, must not turn on the last bit of the
/// interval's median: a part in 10⁹ settles it upwards.
fn half_window(interval: f64) -> usize {
    let exact = MEDIAN_WINDOW_S / (2.0 * interval);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive, finite count of samples, rounded, clamped to at least one"
    )]
    let half = (exact * (1.0 + 1e-9)).round().max(1.0) as usize;
    half
}

/// How far below its true peak the running median of `half` samples either side can read a trace
/// sampled every `interval` s and bent by gravity alone near its peak, m:
/// `g ((⌈half/2⌉ + ½) Δt)² / 2`. At the highest sample, `half + 1` of the window's samples lie
/// within `⌈half/2⌉` places of it, so the median is no lower than they are, and the true peak lies
/// within half a sample of the highest sample. At 20 Hz, 0.077 m.
pub fn peak_bound_m(half: usize, interval: f64) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a window of a few samples converts exactly"
    )]
    let reach = (half.div_ceil(2) as f64 + 0.5) * interval;
    0.5 * STANDARD_GRAVITY_MPS2 * reach * reach
}

/// Whether the record holds to what every reader guarantees: one time per altitude, each channel
/// as long as the clock, and finite times that increase.
fn check_record(log: &FlightLog) -> Result<(), String> {
    let n = log.time_s.len();
    let channels = [
        ("altitude", Some(log.altitude_m.len())),
        ("speed", log.vertical_speed_m_s.as_ref().map(Vec::len)),
        ("temperature", log.temperature_k.as_ref().map(Vec::len)),
        ("battery", log.battery_v.as_ref().map(Vec::len)),
    ];
    for (name, len) in channels {
        if let Some(len) = len
            && len != n
        {
            return Err(format!(
                "the record has {n} times and {len} {name} samples: a reader gives each channel \
                 one sample per time"
            ));
        }
    }
    if let Some(index) = log
        .time_s
        .windows(2)
        .position(|pair| !(pair[0].is_finite() && pair[1].is_finite() && pair[1] > pair[0]))
    {
        return Err(format!(
            "the record's times don't increase at sample {}: {} s, then {} s",
            index + 1,
            log.time_s[index],
            log.time_s[index + 1]
        ));
    }
    Ok(())
}

/// The index of the first greatest finite value.
fn arg_max(values: &[f64]) -> Option<usize> {
    values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .fold(
            None,
            |best: Option<(usize, f64)>, (index, value)| match best {
                Some((_, top)) if *value <= top => best,
                _ => Some((index, *value)),
            },
        )
        .map(|(index, _)| index)
}

/// The top acceleration, withheld: no format read so far records one.
fn no_accelerometer(log: &FlightLog) -> Reading<MaxAcceleration> {
    let detail = match log.format {
        LogFormat::PerfectFlitePf2 => {
            "a PerfectFlite logger has no accelerometer; hpr doesn't difference the altitude \
             twice to make one, as its one-foot steps would read as spikes of many g"
        }
    };
    Reading::withheld(Reason::NoAccelerometer, detail)
}

/// The top vertical speed from liftoff to apogee, or why it is withheld.
fn max_speed(
    log: &FlightLog,
    filtered: &[f64],
    liftoff: usize,
    apogee: usize,
) -> Reading<MaxSpeed> {
    let Some(speed) = &log.vertical_speed_m_s else {
        return Reading::withheld(
            Reason::NoSpeedColumn,
            "the log has no speed column, and hpr doesn't difference the altitude to make one yet",
        );
    };
    let Some(offset) = speed.get(liftoff..=apogee).and_then(arg_max) else {
        return Reading::withheld(
            Reason::NoSpeedColumn,
            "the speed column has no value from liftoff to apogee",
        );
    };
    let climb = &speed[liftoff..=apogee];
    let (index, top) = (liftoff + offset, climb[offset]);
    let worst = climb
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::INFINITY, f64::min);
    if top > IMPLAUSIBLE_SPEED_M_S {
        return Reading::withheld(
            Reason::ImplausibleSpeed,
            format!("the speed column peaks at {top:.1} m/s, above {IMPLAUSIBLE_SPEED_M_S} m/s"),
        );
    }
    if top <= 0.0 || -worst > ASCENT_NOISE_FRACTION * top {
        return Reading::withheld(
            Reason::NoisySpeed,
            format!(
                "the speed from liftoff to apogee swings from {worst:.1} to {top:.1} m/s: a \
                 negative swing over {:.0}% of the top has no usable sign",
                ASCENT_NOISE_FRACTION * 100.0
            ),
        );
    }
    if index == liftoff {
        return Reading::withheld(
            Reason::SpeedPeakAtLiftoff,
            format!("the speed peaks at {top:.1} m/s on the liftoff sample itself: a spike"),
        );
    }
    Reading::Read(MaxSpeed {
        speed_m_s: top,
        time_s: log.time_s[index],
        altitude_m: filtered[index],
        source: Source::LoggerSpeedFromBarometer,
    })
}

/// Landing and the descent, or why they are withheld.
fn landing(
    time: &[f64],
    filtered: &[f64],
    pad: f64,
    (apogee, apogee_s): (usize, f64),
    (liftoff, interval): (usize, f64),
    rounding: f64,
) -> Reading<Landing> {
    let end = time[filtered.len() - 1];
    let landed = (apogee + 1..filtered.len()).find(|&index| {
        filtered[index] < pad + LANDING_HEIGHT_M
            && time[index] + LANDED_FOR_S <= end
            && filtered[index..]
                .iter()
                .zip(&time[index..])
                .take_while(|(_, t)| **t <= time[index] + LANDED_FOR_S)
                .all(|(h, _)| *h < pad + LANDED_CEILING_M)
    });
    let Some(index) = landed else {
        return Reading::withheld(
            Reason::EndsBeforeLanding,
            format!(
                "the log ends at {end:.2} s, {:.1} m above the pad, before the altitude comes \
                 within {LANDING_HEIGHT_M} m of it and stays under {LANDED_CEILING_M} m for \
                 {LANDED_FOR_S} s",
                filtered[filtered.len() - 1] - pad
            ),
        );
    };
    let descent_time_s = time[index] - apogee_s;
    let drop = filtered[apogee] - filtered[index];
    // The quickest any fall from rest at apogee can lose that height: in vacuum. Both heights
    // are rounded, so the drop can read up to one resolution long; and the apogee's time, the
    // middle of its flat run, can sit about half a sample from the true one, so a sample is
    // allowed. A test passes a vacuum fall in feet at 10 to 100 samples a second, wherever its
    // apogee falls within a foot and its liftoff between samples, and fails without either.
    let quickest = (2.0 * (drop - rounding).max(0.0) / STANDARD_GRAVITY_MPS2).sqrt();
    if descent_time_s + interval < quickest {
        return Reading::withheld(
            Reason::FasterThanFreeFall,
            format!(
                "the altitude reaches the pad {descent_time_s:.2} s after apogee, sooner than a \
                 fall from rest in vacuum could ({quickest:.2} s): the trace isn't a height there"
            ),
        );
    }
    Reading::Read(Landing {
        time_s: time[index],
        flight_time_s: time[index] - time[liftoff],
        descent_time_s,
        mean_descent_rate_m_s: drop / descent_time_s,
        source: Source::Barometer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::Stated;

    const DT: f64 = 0.05;

    /// A log sampled every 0.05 s to `end_s`, its height and speed from `flight`.
    fn log(end_s: f64, flight: impl Fn(f64) -> (f64, f64)) -> FlightLog {
        log_every(DT, end_s, flight)
    }

    /// A log sampled every `dt` s to `end_s`, its height and speed from `flight`.
    fn log_every(dt: f64, end_s: f64, flight: impl Fn(f64) -> (f64, f64)) -> FlightLog {
        let times: Vec<f64> = (0..)
            .map(|i| f64::from(i) * dt)
            .take_while(|t| *t <= end_s + 1e-9)
            .collect();
        let (altitude_m, speed): (Vec<f64>, Vec<f64>) = times.iter().map(|t| flight(*t)).unzip();
        FlightLog {
            format: LogFormat::PerfectFlitePf2,
            logger: "PerfectFlite Pnut".to_owned(),
            serial_number: None,
            firmware: None,
            flight_number: None,
            stated: Stated::default(),
            time_s: times,
            altitude_m,
            vertical_speed_m_s: Some(speed),
            temperature_k: None,
            battery_v: None,
            notes: Vec::new(),
        }
    }

    /// Up at 20 m/s from 1 s to 6 s (100 m), then down at `descent` m/s to the pad.
    fn flight(descent: f64) -> impl Fn(f64) -> (f64, f64) {
        move |t| {
            if t <= 1.0 {
                (0.0, 0.0)
            } else if t < 6.0 {
                (20.0 * (t - 1.0), 20.0)
            } else {
                let h = 100.0 - descent * (t - 6.0);
                if h > 0.0 { (h, -descent) } else { (0.0, 0.0) }
            }
        }
    }

    fn reason<T>(reading: &Reading<T>) -> Option<Reason> {
        match reading {
            Reading::Read(_) => None,
            Reading::Withheld(withheld) => Some(withheld.reason),
        }
    }

    /// The test flight reads: liftoff at 1 s, apogee at 6 s, landing below 2 m.
    #[test]
    fn a_plain_flight_reads() {
        let read = read(&log(20.0, flight(10.0)));
        assert_eq!(read.liftoff.value().unwrap().time_s, 1.0);
        // The peak is a corner, which the median takes a metre off: the fourth highest of the
        // seven samples around it, 99 m, held from 5.95 s to 6.1 s.
        let apogee = read.apogee.value().unwrap();
        assert_eq!(apogee.altitude_m, 99.0);
        assert!((apogee.time_s - 6.025).abs() < 1e-12, "{}", apogee.time_s);
        assert_eq!(apogee.highest_sample.altitude_m, 100.0);
        assert_eq!(read.max_speed.value().unwrap().speed_m_s, 20.0);
        let landing = read.landing.value().unwrap();
        // 100 m at 10 m/s from 6 s: below 2 m after 9.8 s, the sample after 15.8 s, at 1.5 m.
        assert!((landing.time_s - 15.85).abs() < 1e-9, "{}", landing.time_s);
        assert!((landing.mean_descent_rate_m_s - 97.5 / 9.825).abs() < 1e-9);
        assert_eq!(
            reason(&read.max_acceleration),
            Some(Reason::NoAccelerometer)
        );
    }

    /// Each withheld reading names the reason that fired, and only the readings that need it go.
    #[test]
    fn each_refusal_names_its_reason() {
        let short = read(&log(0.05, flight(10.0)));
        for reason_of in [
            reason(&short.liftoff),
            reason(&short.apogee),
            reason(&short.max_speed),
            reason(&short.landing),
        ] {
            assert_eq!(reason_of, Some(Reason::TooShort));
        }

        let flat = read(&log(10.0, |_| (0.5, 0.0)));
        assert_eq!(reason(&flat.apogee), Some(Reason::NoClimb));
        assert_eq!(reason(&flat.landing), Some(Reason::NoClimb));

        // Starting 50 m up: no liftoff and no pad, so no top speed and no landing; apogee reads.
        let aloft = read(&log(30.0, |t| flight(10.0)(t + 3.5)));
        assert_eq!(reason(&aloft.liftoff), Some(Reason::StartsOffThePad));
        assert_eq!(reason(&aloft.max_speed), Some(Reason::Needs));
        assert_eq!(reason(&aloft.landing), Some(Reason::Needs));
        assert_eq!(aloft.apogee.value().unwrap().time_after_liftoff_s, None);

        // A record a reader would never give: refused whole, and saying why.
        let mut short_speed = log(20.0, flight(10.0));
        short_speed.vertical_speed_m_s.as_mut().unwrap().pop();
        let refused = read(&short_speed);
        assert_eq!(reason(&refused.apogee), Some(Reason::BadRecord));
        assert_eq!(reason(&refused.landing), Some(Reason::BadRecord));
        let mut repeated = log(20.0, flight(10.0));
        repeated.time_s[10] = repeated.time_s[9];
        assert_eq!(reason(&read(&repeated).liftoff), Some(Reason::BadRecord));

        let cut = read(&log(12.0, flight(10.0)));
        assert_eq!(reason(&cut.landing), Some(Reason::EndsBeforeLanding));
        assert!(cut.apogee.value().is_some());

        // 100 m in 3 s: a vacuum fall from rest takes √(200/g) = 4.5 s.
        let dropped = read(&log(20.0, flight(100.0 / 3.0)));
        assert_eq!(reason(&dropped.landing), Some(Reason::FasterThanFreeFall));

        // Still climbing at the end: the apogee is a floor.
        let climbing = read(&log(4.0, flight(10.0)));
        assert!(climbing.apogee.value().unwrap().is_floor);
        assert_eq!(reason(&climbing.landing), Some(Reason::EndsBeforeLanding));
    }

    /// The pad is the median of everything before the climb: a jitter in the first samples
    /// doesn't move liftoff to the start of the log.
    #[test]
    fn early_jitter_leaves_the_pad_where_it_is() {
        let foot = crate::perfectflite::FOOT_M;
        let mut log = log(20.0, |t| flight(10.0)(t - 2.0));
        for (index, feet) in [0.0, -1.0, -1.0, 0.0].into_iter().enumerate() {
            log.altitude_m[index] = feet * foot;
        }
        let read = read(&log);
        assert_eq!(read.pad_altitude_m, Some(0.0));
        assert_eq!(read.liftoff.value().unwrap().time_s, 3.0);
    }

    /// A vacuum hop from `t0` to `apogee_m`, falling back at `fall_g` times gravity, its heights
    /// rounded to whole feet as a PerfectFlite writes them; and when it is back on the pad.
    fn hop(apogee_m: f64, t0: f64, fall_g: f64) -> (impl Fn(f64) -> (f64, f64), f64) {
        let g = STANDARD_GRAVITY_MPS2;
        let foot = crate::perfectflite::FOOT_M;
        let v0 = (2.0 * g * apogee_m).sqrt();
        let top = t0 + v0 / g;
        let fall = fall_g * g;
        let down = top + (2.0 * apogee_m / fall).sqrt();
        let flight = move |t: f64| {
            let (h, v) = if t <= t0 {
                (0.0, 0.0)
            } else if t <= top {
                (
                    v0 * (t - t0) - 0.5 * g * (t - t0).powi(2),
                    v0 - g * (t - t0),
                )
            } else if t < down {
                (apogee_m - 0.5 * fall * (t - top).powi(2), -fall * (t - top))
            } else {
                (0.0, 0.0)
            };
            ((h / foot).round() * foot, v)
        };
        (flight, down)
    }

    /// The free-fall check passes a fall in vacuum wherever it falls between samples, with the
    /// heights rounded to feet, at 10 to 100 samples a second, and refuses one at 1.3 g.
    #[test]
    fn a_vacuum_fall_lands_and_a_faster_one_is_refused() {
        let foot = crate::perfectflite::FOOT_M;
        // Low hops, their apogees a fiftieth of a foot apart, where the rounding matters most;
        // and higher ones.
        let apogees: Vec<f64> = (0..50)
            .map(|k| (16.0 + f64::from(k) / 50.0) * foot)
            .chain([30.0, 300.0, 3000.0])
            .collect();
        for dt in [0.01, 0.05, 0.1] {
            for &apogee_m in &apogees {
                for step in 0..13 {
                    let t0 = 1.0 + f64::from(step) * dt / 13.0;
                    let (flight, down) = hop(apogee_m, t0, 1.0);
                    let read = read(&log_every(dt, down + 3.0, flight));
                    assert!(
                        read.landing.value().is_some(),
                        "{dt} s, {apogee_m} m, from {t0} s: {:?}",
                        read.landing
                    );
                }
            }
        }
        for apogee_m in [100.0, 1000.0] {
            let (flight, down) = hop(apogee_m, 1.0, 1.3);
            let read = read(&log(down + 3.0, flight));
            assert_eq!(
                reason(&read.landing),
                Some(Reason::FasterThanFreeFall),
                "{apogee_m} m"
            );
        }
    }

    /// A log that starts just before liftoff: the pad is read from before the first metre of
    /// rise, so the climb's first samples don't lift it and liftoff stays at the sample it was.
    /// Taken from before the 3 m climb instead, the pad would read 1.1 m and liftoff 0.35 s.
    #[test]
    fn a_short_pad_isnt_lifted_by_the_climb() {
        let read = read(&log(5.0, |t| {
            if t <= 0.1 {
                (0.0, 0.0)
            } else {
                (5.0 * (t - 0.1), 5.0)
            }
        }));
        assert!((read.pad_altitude_m.unwrap() - 0.125).abs() < 1e-12);
        assert!((read.liftoff.value().unwrap().time_s - 0.15).abs() < 1e-12);
    }

    /// A pad whose median falls between two feet: a sample a foot up is still on the pad, as it
    /// lies within half a foot of that median, so liftoff is the last sample before the climb,
    /// at 1.05 s, not the 0.5 s where the altitude last read zero.
    #[test]
    fn a_pad_between_two_feet_takes_half_a_foot_either_way() {
        let foot = crate::perfectflite::FOOT_M;
        let read = read(&log(20.0, |t| {
            if t < 0.525 {
                (0.0, 0.0)
            } else if t < 1.075 {
                (foot, 0.0)
            } else {
                (foot + 20.0 * (t - 1.05), 20.0)
            }
        }));
        assert_eq!(read.pad_altitude_m, Some(foot / 2.0));
        assert_eq!(read.liftoff.value().unwrap().time_s, 21.0 * DT);
    }

    /// A clock too fine for the median's window is withheld whole, saying so, at the edge: 0.3 s
    /// is 1,000 samples either side at 0.15 ms, and 1,001 at 0.3/2002 s.
    #[test]
    fn a_clock_too_fine_for_the_window_is_withheld() {
        let with_interval = |interval: f64| {
            let mut log = log(20.0, flight(10.0));
            for (index, time) in log.time_s.iter_mut().enumerate() {
                *time = f64::from(u32::try_from(index).unwrap()) * interval;
            }
            read(&log)
        };
        for interval in [1e-30, 1e-6, MEDIAN_WINDOW_S / 2002.0] {
            let read = with_interval(interval);
            assert_eq!(
                reason(&read.apogee),
                Some(Reason::SampledTooFast),
                "{interval}"
            );
            assert_eq!(reason(&read.landing), Some(Reason::SampledTooFast));
            assert_eq!(read.sample_interval_s.map(|s| s > 0.0), Some(true));
        }
        // At 0.15 ms the window is exactly 1,000 samples either side, and the log is read.
        let read = with_interval(0.000_15);
        assert_ne!(reason(&read.apogee), Some(Reason::SampledTooFast));
        let window_s = read.median_window_s.unwrap();
        assert!((window_s - 0.3).abs() < 1e-12, "{window_s}");
    }

    /// The window's half-width at a rounding tie doesn't turn on the interval's last bit.
    #[test]
    fn the_window_settles_ties_upwards() {
        assert_eq!(half_window(0.05), 3);
        for interval in [0.1, 0.1 + 1e-15, 0.1 - 1e-15, 0.2 - 0.1] {
            assert_eq!(half_window(interval), 2, "{interval}");
        }
        assert_eq!(half_window(1.0), 1);
        assert!((peak_bound_m(3, 0.05) - 0.076_614).abs() < 1e-6);
    }

    proptest::proptest! {
        /// The median's peak bound holds wherever the true peak falls between samples, at any
        /// half-width, and the median never reads above the peak.
        #[test]
        fn the_peak_bound_holds(phase in 0.0..1.0_f64, half in 1_usize..7, dt in 0.01..0.2_f64) {
            let trace: Vec<f64> = (-60..=60)
                .map(|i| {
                    let t = (f64::from(i) - phase) * dt;
                    -0.5 * STANDARD_GRAVITY_MPS2 * t * t
                })
                .collect();
            let top = running_median(&trace, half)
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max);
            proptest::prop_assert!(top <= 0.0);
            proptest::prop_assert!(-top <= peak_bound_m(half, dt) * (1.0 + 1e-12));
        }
    }

    /// The top speed's guards, each on its own.
    #[test]
    fn a_top_speed_is_refused_as_debrief_refuses_one() {
        let mut no_column = log(20.0, flight(10.0));
        no_column.vertical_speed_m_s = None;
        assert_eq!(
            reason(&read(&no_column).max_speed),
            Some(Reason::NoSpeedColumn)
        );

        let speed_at = |t: f64, value: f64| {
            let mut log = log(20.0, flight(10.0));
            let index = (t / DT).round() as usize;
            log.vertical_speed_m_s.as_mut().unwrap()[index] = value;
            read(&log).max_speed
        };
        assert_eq!(
            reason(&speed_at(3.0, 4000.5)),
            Some(Reason::ImplausibleSpeed)
        );
        assert!(speed_at(3.0, 4000.0).value().is_some());
        // 20 m/s at the top: a swing to −4 m/s is 20%, and passes; −4.5 m/s doesn't.
        assert!(speed_at(3.0, -4.0).value().is_some());
        assert_eq!(reason(&speed_at(3.0, -4.5)), Some(Reason::NoisySpeed));
        // Liftoff is the sample at 1.0 s: a peak there is a spike.
        assert_eq!(
            reason(&speed_at(1.0, 50.0)),
            Some(Reason::SpeedPeakAtLiftoff)
        );
        assert_eq!(speed_at(1.05, 50.0).value().unwrap().speed_m_s, 50.0);
    }

    /// `ALL` is built from the enum's own list, so it can't miss a variant; each is listed once,
    /// with its own code.
    #[test]
    fn all_lists_every_reason_and_source_once() {
        let codes = |json: Vec<serde_json::Value>| {
            let mut codes: Vec<String> = json.iter().map(ToString::to_string).collect();
            let listed = codes.len();
            codes.sort();
            codes.dedup();
            (listed, codes.len())
        };
        let reasons = Reason::ALL.iter().map(|r| serde_json::json!(r)).collect();
        assert_eq!(codes(reasons), (13, 13));
        let sources = Source::ALL.iter().map(|s| serde_json::json!(s)).collect();
        assert_eq!(codes(sources), (2, 2));
    }

    /// Serialized, a reading carries its status beside its fields.
    #[test]
    fn a_reading_serializes_with_its_status() {
        let read = read(&log(20.0, flight(10.0)));
        let json = serde_json::to_value(&read).unwrap();
        assert_eq!(json["apogee"]["status"], "read");
        assert_eq!(json["apogee"]["altitude_m"], 99.0);
        assert_eq!(json["max_acceleration"]["status"], "withheld");
        assert_eq!(json["max_acceleration"]["reason"], "no_accelerometer");
        let back: Readings = serde_json::from_value(json).unwrap();
        assert_eq!(back, read);
    }
}
