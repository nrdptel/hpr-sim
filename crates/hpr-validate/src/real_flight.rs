//! Real flights: hpr against the altitude logs of RocketPy's example rockets, flown in the ERA5
//! weather of their day ([M2.3b][m2-3b], decision [ADR-082][adr-082]).
//!
//! Each [`RealFlight`] is a rocket RocketPy's documentation flies against its team's log. hpr flies
//! the same rocket (its design under `validation/designs/`, built from the example's inputs), on the
//! example's own thrust file with the example's burn options, from the example's rail and site, in
//! the ERA5 file the example reads, with hpr's own aerodynamics: the way a user would fly it. The
//! log is the reference.
//!
//! The logs, most of the thrust files and the full weather files carry their own terms, so they are
//! read from the pinned RocketPy checkout under the gitignored `refs/` and never committed. What is
//! committed is [`REPORT_JSON`] and [`REPORT_MD`]: per flight, the log's apogee and hpr's, the
//! apogee error, the altitude-trace RMS, and the SHA-256 of every file read. `cargo xtask
//! real-flights` writes them and `--check` flies them again; CI, which has no `refs/`, holds the
//! summary to the rows and the page to the data ([`RealFlightReport::check_consistent`]).
//!
//! What is compared, for each flight:
//!
//! - **Heights:** a barometric altimeter reads the standard atmosphere's altitude of the pressure
//!   it measures, less the pad's, not the height it climbed: on a warm day, less. So hpr's height
//!   is read the same way, from the ERA5 pressure at its centre of mass, for a log whose
//!   [`Altimeter`] is barometric ([`Barometer`]), and taken as it is otherwise. Each
//!   log's height column is used as recorded.
//! - **Apogee:** the highest height in the log, read up to [`Log::until_s`] where the recovery's
//!   pressure transients would otherwise set it, against hpr's apogee read the same way. The error
//!   is hpr's less the log's, as a percentage of the log's.
//! - **Altitude-trace RMS:** the log's heights against hpr's over the ascent. The two clocks are
//!   aligned where each trace first reaches [`ALIGN_HEIGHT_M`], since a log's zero is its own
//!   (armed, launch detected, or power on) and not hpr's ignition. The RMS is over every log row from
//!   there until the first of the two apogees, hpr's heights linearly interpolated from a
//!   [`GRID_S`] grid of its dense output. The descent is not compared: its events (which parachute
//!   opened, when) are the team's, and one of these flights lost its main.
//!
//! Two diagnostic flights, not predictions, say where a miss may come from: the same flight on the
//! example's own drag, and where the example reshapes its thrust file, on the file as recorded.
//! Where a log keeps its pressure or the flight a satellite altitude, the report holds the
//! barometric reading against them. Each row carries the SHA-256 of its flight's entry in
//! [`FLIGHTS`], so CI sees an input change the report hasn't caught up with.
//!
//! The mean absolute apogee error is reported against the [`APOGEE_TARGET_PERCENT`] target of
//! `docs/VALIDATION.md`; it is a target, not a gate. A flight outside it is an outlier, and each
//! outlier carries its explanation in [`RealFlight::explanation`].
//!
//! [m2-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-3b
//! [adr-082]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-082-real-flights-read-from-refs-compared-over-the-ascent-with-checked-explanations-2026-09-26

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use hpr_aero::DragTable;
use hpr_atmos::{Atmosphere, Ussa76, WindInterpolation};
use hpr_core::earth::Earth;
use hpr_core::geodesy::Geodetic;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_design::Rocket;
use hpr_io::era5::{Era5Profile, Era5Request, UtcTime};
use hpr_io::netcdf::NetCdf;
use hpr_sim::{
    Environment, EventKind, FlightSettings, FlightStep, Observer, Rail, SimError, Simulation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The report, as data.
pub const REPORT_JSON: &str = "validation/reports/real-flights.json";

/// The report, as a page.
pub const REPORT_MD: &str = "validation/reports/real-flights.md";

/// The pinned RocketPy checkout the logs, thrust files and weather files are read from.
pub const ROCKETPY: &str = "refs/rocketpy";

/// The design fixture that records each example's own thrust file and burn options.
pub const MASS_FIXTURE: &str = "validation/fixtures/design/rocketpy-rocket-mass.json";

/// The target for the mean absolute apogee error, per cent (`docs/VALIDATION.md`, "Principles").
pub const APOGEE_TARGET_PERCENT: f64 = 5.0;

/// The height each trace's clock is aligned at, m above its start.
///
/// High enough to be past the pad's noise and the rail in every log here (the longest rail is
/// 12 m), low enough to be inside the first second of flight.
pub const ALIGN_HEIGHT_M: f64 = 30.0;

/// The spacing of hpr's heights the trace is compared against, s.
pub const GRID_S: f64 = 0.01;

/// How long hpr flies past the log's apogee before it stops, s: past any apogee within reach.
const PAST_APOGEE_S: f64 = 30.0;

/// Where a flight's altitude log is and how to read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Log {
    /// The CSV file, under `refs/rocketpy/data/rockets/`.
    pub file: &'static str,
    /// Header lines before the first row.
    pub header_lines: usize,
    /// The column of time, s (zero-based).
    pub time_column: usize,
    /// The column of height above the pad (zero-based), used as recorded.
    pub height_column: usize,
    /// Metres per unit of the height column.
    pub metres_per_unit: f64,
    /// Rows after this time are not read, s: where the log stops recording the rocket's height,
    /// at an ejection charge's pressure transient past apogee or a corrupted end. The flight's
    /// [`RealFlight::note`] says which.
    pub until_s: Option<f64>,
    /// What measured the height, and so what of hpr's flight it is compared with.
    pub altimeter: Altimeter,
    /// The column of the pressure the height was read from, and pascals per unit, where the log
    /// keeps it: the report gives how far the height column is from the standard atmosphere's
    /// reading of it.
    pub pressure: Option<(usize, f64)>,
    /// The flight's satellite (GNSS) altitude, where it logged one: a geometric height to hold the
    /// barometric reading against.
    pub gnss: Option<Gnss>,
}

/// Where a flight's satellite altitude is and how to read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gnss {
    /// The CSV file, under `refs/rocketpy/data/rockets/`.
    pub file: &'static str,
    /// Header lines before the first row.
    pub header_lines: usize,
    /// The column of time, s (zero-based).
    pub time_column: usize,
    /// The column of altitude (zero-based); its first row is the pad's.
    pub altitude_column: usize,
    /// Metres per unit of the altitude column.
    pub metres_per_unit: f64,
}

/// What a log's height is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Altimeter {
    /// A barometric altimeter's: the pressure's altitude in the standard atmosphere less the pad's
    /// ([`hpr_atmos::Ussa76::pressure_altitude_m`]). On a day warmer than the standard it reads
    /// less than the height climbed. hpr's is read the same way, from the ERA5 pressure at its
    /// centre of mass.
    Barometric(&'static str),
    /// Taken as barometric, read as [`Altimeter::Barometric`], without evidence of how its height
    /// was made.
    AssumedBarometric(&'static str),
    /// A height above the pad in metres, compared with hpr's as it is.
    Height(&'static str),
}

impl Altimeter {
    /// The kind's name in the report.
    #[must_use]
    pub fn kind(self) -> &'static str {
        match self {
            Self::Barometric(_) => "barometric",
            Self::AssumedBarometric(_) => "barometric, assumed",
            Self::Height(_) => "height",
        }
    }

    /// Where the kind comes from, in words.
    #[must_use]
    pub fn evidence(self) -> &'static str {
        match self {
            Self::Barometric(text) | Self::AssumedBarometric(text) | Self::Height(text) => text,
        }
    }
}

/// The drag the example flies, as its notebook gives it: the second, diagnostic flight's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExampleDrag {
    /// One `C_D`, power off and on.
    Constant(f64),
    /// `(Mach, C_D)` knots, linear between and held outside; power on is `power_on_factor` times.
    Knots {
        /// The knots, in increasing Mach number.
        points: &'static [(f64, f64)],
        /// Power on, as a multiple of power off.
        power_on_factor: f64,
    },
    /// `(Mach, C_D)` CSV files under `refs/rocketpy/data/rockets/`, linear and held outside,
    /// optionally scaled so power off reads `C_D` at a Mach number.
    Files {
        /// The power-off curve.
        power_off: &'static str,
        /// The power-on curve.
        power_on: &'static str,
        /// `(Mach, C_D)` the power-off curve is scaled to pass through, with the power-on
        /// curve scaled by the same factor.
        scaled_to: Option<(f64, f64)>,
    },
}

/// Why a flight's apogee misses the log's by more than [`APOGEE_TARGET_PERCENT`]: a claim the
/// report's own numbers are checked against ([`RealFlightReport::check_consistent`]), so a change
/// that makes it false fails the check, with the words that argue it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Explanation {
    /// Within the target: nothing to explain.
    None,
    /// Consistent with hpr's drag: on the example's own drag the apogee is within the target, and
    /// on the thrust file as recorded, where there is that flight, it is not.
    Drag(&'static str),
    /// Consistent with the motor's impulse: on the thrust file as recorded, without the reshape
    /// the example applies, the apogee is within the target, and on the example's own drag it is
    /// not.
    Thrust(&'static str),
}

impl Explanation {
    /// The claim's name in the report, empty for none.
    #[must_use]
    pub fn kind(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Drag(_) => "drag",
            Self::Thrust(_) => "thrust",
        }
    }

    /// The words that argue it, empty for none.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Drag(text) | Self::Thrust(text) => text,
        }
    }
}

/// One logged flight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RealFlight {
    /// The flight's id in the report.
    pub id: &'static str,
    /// The rocket, the team and the event, in words.
    pub title: &'static str,
    /// The design flown, by file stem under `validation/designs/`, which is also its case name in
    /// [`MASS_FIXTURE`].
    pub design: &'static str,
    /// Where the example's site, weather, rail and log come from, in RocketPy 1.13.0's notebook.
    pub source: &'static str,
    /// The site: latitude and longitude, degrees, and elevation above sea level, m, as the example
    /// gives them.
    pub site: (f64, f64, f64),
    /// The ERA5 file, under `refs/rocketpy/data/weather/`.
    pub weather: &'static str,
    /// The launch time the example reads the weather at, UTC: year, month, day, hour.
    pub utc: (i32, u32, u32, u32),
    /// The rail: length, m; inclination from the horizontal and heading from north, degrees.
    pub rail: (f64, f64, f64),
    /// The log.
    pub log: Log,
    /// The example's own drag, flown on the example's radius as a diagnostic.
    pub example_drag: ExampleDrag,
    /// Where the example's drag comes from, in the notebook.
    pub example_drag_source: &'static str,
    /// What about this flight's inputs departs from its day, in words; empty when nothing does.
    pub note: &'static str,
    /// Why hpr's apogee misses the log's by more than [`APOGEE_TARGET_PERCENT`], when it does.
    pub explanation: Explanation,
}

/// The flights, in the order the report lists them.
///
/// Each site, time, rail and log is the example notebook's (`docs/examples/<name>_flight_sim.ipynb`
/// in RocketPy 1.13.0, cited per flight). A notebook that gives a time zone gives local time; the
/// UTC hour here is that time converted (Mountain Daylight Time is UTC−6, Western European Summer
/// Time UTC+1).
pub const FLIGHTS: [RealFlight; 7] = [
    RealFlight {
        id: "bella-lui",
        title: "Bella Lui, EPFL Rocket Team, 2020 (K828FJ)",
        design: "rocketpy-bella-lui",
        source: "bella_lui_flight_sim.ipynb:109-111 (rail), :139-153 (site, date), :761-765 (log)",
        site: (47.213476, 9.003336, 407.0),
        weather: "bella_lui_weather_data_ERA5.nc",
        utc: (2020, 2, 22, 13),
        rail: (4.2, 89.0, 45.0),
        log: Log {
            file: "EPFL_Bella_Lui/bella_lui_flight_data_filtered.csv",
            header_lines: 1,
            time_column: 2,
            height_column: 3,
            metres_per_unit: 1.0,
            until_s: None,
            altimeter: Altimeter::AssumedBarometric(
                "the team's own avionics, filtered by a method the example doesn't record: \
                 barometric assumed, as hobby altimeters are",
            ),
            pressure: None,
            gnss: None,
        },
        example_drag: ExampleDrag::Knots {
            points: &[
                (0.01, 0.51),
                (0.02, 0.46),
                (0.04, 0.43),
                (0.28, 0.43),
                (0.29, 0.44),
                (0.45, 0.44),
                (0.49, 0.46),
            ],
            power_on_factor: 1.0,
        },
        example_drag_source: "bella_lui_flight_sim.ipynb:94-95, :453-484 (power off and on, times 1)",
        note: "",
        explanation: Explanation::None,
    },
    RealFlight {
        id: "ndrt-2020",
        title: "NDRT 2020, Notre Dame Rocketry Team (L1395)",
        design: "rocketpy-ndrt-2020-nose-to-tail",
        source: "ndrt_2020_flight_sim.ipynb:115-117 (rail), :148-153 (site, date), :679-683 (log)",
        site: (41.775447, -86.572467, 206.0),
        weather: "ndrt_2020_weather_data_ERA5.nc",
        utc: (2020, 2, 23, 16),
        rail: (3.353, 90.0, 181.0),
        log: Log {
            file: "NDRT_2020/ndrt_2020_flight_data.csv",
            header_lines: 1,
            time_column: 3,
            height_column: 4,
            metres_per_unit: 0.3048,
            until_s: None,
            altimeter: Altimeter::Barometric(
                "a Featherweight Raven (RocketPy's tests/acceptance/test_ndrt_2020_rocket.py:190), \
                 a barometric altimeter, in feet above the pad",
            ),
            pressure: None,
            gnss: None,
        },
        example_drag: ExampleDrag::Constant(0.44),
        example_drag_source: "ndrt_2020_flight_sim.ipynb:91 and :316-317 (the drag coefficient, power off and on)",
        note: "",
        explanation: Explanation::Drag(
            "consistent with hpr's drag. On the example's own drag, a constant 0.44 the notebook \
             gives no source for, the apogee is within the target. hpr's own drag is lower, as the \
             predicted-mode comparison with RocketPy flying the same constant found (+10.3% in \
             apogee), and the design's fin edges and finish are placeholders, since the example \
             records none.",
        ),
    },
    RealFlight {
        id: "prometheus-2022",
        title: "Prometheus, Western Engineering, Spaceport America Cup 2022 (M1520)",
        design: "rocketpy-prometheus-2022-generic-motor",
        source: "prometheus_2022_flight_sim.ipynb:65-70 (site, date), :401-406 (rail), :508-526 \
                 (log)",
        site: (32.939377, -106.911986, 1401.0),
        weather: "spaceport_america_pressure_levels_2023_hourly.nc",
        utc: (2023, 6, 24, 15),
        rail: (5.18, 80.0, 75.0),
        log: Log {
            file: "prometheus/2022-06-24-serial-5115-flight-0001-TeleMetrum.csv",
            header_lines: 1,
            time_column: 4,
            height_column: 10,
            metres_per_unit: 1.0,
            until_s: Some(29.58),
            altimeter: Altimeter::Barometric(
                "an Altus Metrum TeleMetrum: its height column is the standard atmosphere's \
                 altitude of its pressure column less the first row's (the gap is below)",
            ),
            pressure: Some((8, 1.0)),
            gnss: Some(Gnss {
                file: "prometheus/2022-06-24-serial-5115-flight-0001-TeleMetrum.csv",
                header_lines: 1,
                time_column: 4,
                altitude_column: 21,
                metres_per_unit: 1.0,
            }),
        },
        example_drag: ExampleDrag::Knots {
            points: &[
                (0.15, 0.422),
                (0.45, 0.38),
                (0.77, 0.32),
                (0.82, 0.3),
                (0.88, 0.3),
                (0.94, 0.32),
                (0.99, 0.37),
                (1.04, 0.44),
                (1.24, 0.43),
                (1.33, 0.42),
                (1.49, 0.39),
            ],
            power_on_factor: 1.02,
        },
        example_drag_source: "prometheus_2022_flight_sim.ipynb:224-261 (`prometheus_cd_at_ma` from Mach 0.15, where it \
                              starts to change, and power on 1.02 times it)",
        note: "flown in the weather of 24 June 2023, a year after the flight, as RocketPy's example \
               flies it: RocketPy has no ERA5 file of the day. The log is read to 29.58 s: a \
               pressure transient then drops the reading 600 m and returns it 8 m above the \
               highest reading before",
        explanation: Explanation::Drag(
            "consistent with hpr's drag. On the example's own drag, the team's table from Mach \
             0.15, the apogee is within the target. The reading is built on weather a year off \
             the flight's day (the note). Against the satellite heights, hpr's conversion reads \
             1 to 2 points below the altimeter here and on Juno III, which flew in its own day's \
             weather, so the wrong day's share of the miss can't be told apart.",
        ),
    },
    RealFlight {
        id: "juno-iii",
        title: "Juno III, Projeto Jupiter, Spaceport America Cup 2023 (the team's motor)",
        design: "rocketpy-juno-iii",
        source: "juno3_flight_sim.ipynb:54-67 (site, date), :448-456 (rail), :543-553 (log)",
        site: (32.939377, -106.911986, 1480.0),
        weather: "spaceport_america_pressure_levels_2023_hourly.nc",
        utc: (2023, 6, 23, 23),
        rail: (5.2, 85.0, 105.0),
        log: Log {
            file: "juno3/cots_altimeter.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 1.0,
            until_s: Some(24.60),
            altimeter: Altimeter::Barometric(
                "a Missile Works RRC3 (juno3/README.txt:21): its height column is the standard \
                 atmosphere's altitude of its pressure column less the first row's (the gap is \
                 below)",
            ),
            pressure: Some((2, 100.0)),
            gnss: Some(Gnss {
                file: "juno3/cots_GNSS.csv",
                header_lines: 1,
                time_column: 0,
                altitude_column: 1,
                metres_per_unit: 0.3048,
            }),
        },
        example_drag: ExampleDrag::Files {
            power_off: "juno3/drag_curve.csv",
            power_on: "juno3/drag_curve.csv",
            scaled_to: Some((0.6, 0.38)),
        },
        example_drag_source: "juno3_flight_sim.ipynb:244-251, :356-359 (`drag_curve.csv`, scaled to 0.38 at Mach 0.6 \
                              \"from CFD analysis\")",
        note: "the log is read to 24.60 s, as it levels off: a pressure transient there dips the \
               reading by 94 m, then lifts it 62 m above the level within 0.3 s, to the 3213.4 m \
               the team reports as its apogee, and the record's last two rows are corrupt. At the cut the \
               log's own velocity column still reads 17.5 m/s up, so its apogee may be 10 m to \
               20 m low. The thrust file's last five points are negative (-6.8 to -47.3 N); hpr, \
               which refuses a negative thrust, reads them as zero, and RocketPy's flight holds \
               its thrust at zero too",
        explanation: Explanation::Thrust(
            "consistent with the motor's impulse. The notebook reshapes the team's own motor \
             curve (`mandioca_thrust_curve.csv`) to 5.8 s and 8800 N s, 4.9% less than the file; \
             on the file as recorded the apogee is within the target, and on the team's drag it \
             is not.",
        ),
    },
    RealFlight {
        id: "cavour",
        title: "Cavour, Politecnico di Torino, EuRoC 2023 (L995)",
        design: "rocketpy-cavour",
        source: "cavour_flight_sim.ipynb:125-132 (site, date), :314-315 (rail), :379-389 (log)",
        site: (39.388692, -8.287814, 150.0),
        weather: "euroc_2023_all_windows.nc",
        utc: (2023, 10, 13, 12),
        rail: (12.0, 84.0, 133.0),
        log: Log {
            file: "polito/altimeter_cavour.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 1.0,
            until_s: None,
            altimeter: Altimeter::AssumedBarometric(
                "the CATS Vega EuRoC 2023 required, sent by radio in whole metres: a Kalman \
                 filter's estimate from a barometer and an accelerometer, barometric assumed",
            ),
            pressure: None,
            gnss: None,
        },
        example_drag: ExampleDrag::Files {
            power_off: "polito/drag_coefficient_power_off.csv",
            power_on: "polito/drag_coefficient_power_on.csv",
            scaled_to: None,
        },
        example_drag_source: "cavour_flight_sim.ipynb:250-251",
        note: "",
        explanation: Explanation::Drag(
            "consistent with hpr's drag. On the example's own curves, labelled RASAero II, the \
             apogee is within the target. hpr's drag is below them: at Mach 0.3, 8.3% below power \
             off and 18.3% below power on (the aerodynamics page's comparison; the power-on gap's \
             cause is open), and the design's fin edges are placeholders, since the example \
             records none.",
        ),
    },
    RealFlight {
        id: "genesis",
        title: "Genesis, EuRoC 2023 (L995)",
        design: "rocketpy-genesis",
        source: "genesis_flight_sim.ipynb:124-131 (site, date), :326-327 (rail), :391-406 (log)",
        site: (39.38895, -8.28837, 160.0),
        weather: "euroc_2023_all_windows.nc",
        utc: (2023, 10, 12, 13),
        rail: (12.0, 84.0, 133.0),
        log: Log {
            file: "genesis/flight_data_faraday.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 1.0,
            until_s: None,
            altimeter: Altimeter::AssumedBarometric(
                "a filtered estimate (`filtered_altitude_AGL`), probably the CATS Vega's \
                 barometer and accelerometer Kalman filter: barometric assumed",
            ),
            pressure: None,
            gnss: None,
        },
        example_drag: ExampleDrag::Files {
            power_off: "genesis/drag_coefficient_power_off.csv",
            power_on: "genesis/drag_coefficient_power_on.csv",
            scaled_to: None,
        },
        example_drag_source: "genesis_flight_sim.ipynb:241-242",
        note: "",
        explanation: Explanation::Drag(
            "consistent with hpr's drag. On the example's own curves the apogee is within the \
             target; the design's fin edges and finish are placeholders, since the example \
             records none.",
        ),
    },
    RealFlight {
        id: "lince",
        title: "Lince, EuRoC 2023 (M1101)",
        design: "rocketpy-lince",
        source: "lince_flight_sim.ipynb:123-129 (site, date), :432-437 (rail), :609-619 (log)",
        site: (39.3897, -8.288964, 158.0),
        weather: "euroc_2023_all_windows.nc",
        utc: (2023, 10, 12, 10),
        rail: (12.0, 84.0, 133.0),
        log: Log {
            file: "lince/main_data.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 1.0,
            until_s: Some(26.80),
            altimeter: Altimeter::AssumedBarometric(
                "a filtered estimate (`filtered_altitude_AGL`, as Genesis's) from a computer \
                 the example doesn't name: barometric assumed",
            ),
            pressure: None,
            gnss: None,
        },
        example_drag: ExampleDrag::Files {
            power_off: "lince/drag_coefficient_power_off.csv",
            power_on: "lince/drag_coefficient_power_on.csv",
            scaled_to: None,
        },
        example_drag_source: "lince_flight_sim.ipynb:240-241",
        note: "the log is read to 26.80 s: past it, as the recovery fires, the filtered height swings \
               by hundreds of metres, up to 3668.5 m; its highest reading before is the 3587 m \
               the team reports as its apogee",
        explanation: Explanation::None,
    },
];

/// Why a real flight could not be read, flown or reported.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RealFlightError {
    /// A file could not be read.
    #[error("{what} {path}: {source}")]
    Io {
        /// What was being read.
        what: &'static str,
        /// The file.
        path: String,
        /// The error.
        #[source]
        source: std::io::Error,
    },
    /// A file's content is not what the flight needs.
    #[error("{flight}: {what}")]
    Input {
        /// The flight.
        flight: String,
        /// What is wrong, in words.
        what: String,
    },
    /// hpr could not fly it.
    #[error("{flight}: {source}")]
    Sim {
        /// The flight.
        flight: String,
        /// The error.
        #[source]
        source: SimError,
    },
}

/// A file read and its SHA-256.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileRead {
    /// Its path, relative to the repository root.
    pub path: String,
    /// Its SHA-256, in hex.
    pub sha256: String,
}

/// One flight's result, as the report keeps it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightRow {
    /// The flight's id.
    pub id: String,
    /// The rocket, team and event.
    pub title: String,
    /// The design flown.
    pub design: String,
    /// The notebook lines the inputs come from.
    pub source: String,
    /// The log, the thrust file, the weather file and the design, with their digests.
    pub files: Vec<FileRead>,
    /// The launch time the weather is read at, UTC, ISO 8601.
    pub weather_time_utc: String,
    /// The log's rows read.
    pub log_rows: usize,
    /// The time past which the log is not read, s ([`Log::until_s`]).
    pub log_until_s: Option<f64>,
    /// What measured the log's height ([`Altimeter::kind`]).
    pub altimeter: String,
    /// Where that comes from.
    pub altimeter_evidence: String,
    /// The log's apogee: its highest reading, m.
    pub log_apogee_m: f64,
    /// hpr's apogee as the log's altimeter would read it, m: above its centre of mass's starting
    /// height, or for a barometric log, the pressure altitude of the ERA5 pressure there less the
    /// start's.
    pub hpr_apogee_m: f64,
    /// hpr's apogee above its centre of mass's starting height, m, whatever the altimeter.
    pub hpr_height_apogee_m: f64,
    /// hpr's less the log's, per cent of the log's.
    pub apogee_error_percent: f64,
    /// The time from the alignment height to apogee in the log, s.
    pub log_time_to_apogee_s: f64,
    /// The same in hpr, s.
    pub hpr_time_to_apogee_s: f64,
    /// The RMS of hpr's heights less the log's over the ascent, m.
    pub trace_rms_m: f64,
    /// That RMS as a percentage of the log's apogee.
    pub trace_rms_percent: f64,
    /// The log rows the RMS is over.
    pub trace_rows: usize,
    /// The thrust hpr flew, as a total impulse, N s.
    pub total_impulse_ns: f64,
    /// The impulse added by reading the thrust file's negative points as zero, N s.
    pub negative_thrust_zeroed_ns: f64,
    /// Where the example's drag comes from.
    pub example_drag_source: String,
    /// hpr's apogee on the example's drag, m.
    pub example_drag_apogee_m: f64,
    /// Its error against the log's, per cent.
    pub example_drag_apogee_error_percent: f64,
    /// Its trace RMS against the log's, m.
    pub example_drag_trace_rms_m: f64,
    /// For an example that reshapes its thrust file, the second diagnostic: the file flown as
    /// recorded.
    pub recorded_thrust: Option<RecordedThrust>,
    /// For a log that keeps its pressure: the largest gap between its height column and the
    /// standard atmosphere's reading of that pressure less the first row's, over the rows read, m.
    pub pressure_reading_max_m: Option<f64>,
    /// For a flight that logged a satellite altitude: its highest above its first row, m.
    pub gnss_apogee_m: Option<f64>,
    /// The SHA-256 of the flight's entry in [`FLIGHTS`] as Rust's `Debug` prints it: every input
    /// the code gives it, so CI sees a changed one. `Debug` output may change with a Rust
    /// release; CI then flags every row, and `cargo xtask real-flights` rewrites them.
    pub inputs_sha256: String,
    /// What departs from the day, in words.
    pub note: String,
    /// What the explanation claims ([`Explanation::kind`]), empty when there is none.
    pub explanation_kind: String,
    /// Why the apogee misses by more than the target, when it does.
    pub explanation: String,
}

/// The flight on the thrust file as recorded, without the example's reshape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedThrust {
    /// The total impulse flown, N s: the file's, negative points read as zero, clipped at the
    /// example's burn time where it gives one.
    pub impulse_ns: f64,
    /// hpr's apogee, read as the log's altimeter reads, m.
    pub apogee_m: f64,
    /// Its error against the log's, per cent.
    pub apogee_error_percent: f64,
}

/// What the report sums up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Flights compared.
    pub flights: usize,
    /// The mean of the absolute apogee errors, per cent.
    pub mean_absolute_apogee_error_percent: f64,
    /// The target it is reported against, per cent.
    pub target_percent: f64,
    /// Whether the mean is at or below the target.
    pub within_target: bool,
    /// The mean apogee error with its sign, per cent: a bias shows here.
    pub mean_apogee_error_percent: f64,
    /// The mean absolute apogee error were every log a height, not a barometric reading, per cent:
    /// how much the altimeters' readings matter.
    pub mean_absolute_height_apogee_error_percent: f64,
    /// The mean absolute apogee error with only the logs known to be barometric read so and the
    /// assumed ones as heights, per cent.
    pub mean_absolute_known_barometric_apogee_error_percent: f64,
    /// The flights whose apogee misses by more than the target.
    pub outliers: Vec<String>,
    /// The largest trace RMS as a percentage of its log's apogee.
    pub max_trace_rms_percent: f64,
    /// The mean absolute apogee error on each example's own drag, per cent.
    pub example_drag_mean_absolute_apogee_error_percent: f64,
}

/// The real-flight report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealFlightReport {
    /// What wrote it.
    pub generated_by: String,
    /// The alignment height, m.
    pub align_height_m: f64,
    /// The grid hpr's heights are kept on, s.
    pub grid_s: f64,
    /// Each flight.
    pub flights: Vec<FlightRow>,
    /// The summary, computed from the rows.
    pub summary: Summary,
}

fn read_bytes(root: &Path, relative: &str, what: &'static str) -> Result<Vec<u8>, RealFlightError> {
    let path = root.join(relative);
    fs::read(&path).map_err(|source| RealFlightError::Io {
        what,
        path: path.display().to_string(),
        source,
    })
}

/// The SHA-256 of `bytes`, in lower-case hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
}

/// Each row's `columns`, in file order: a row is split at commas and each column trimmed and
/// parsed, and a row where one doesn't parse as a finite number is skipped.
fn read_rows(text: &str, header_lines: usize, columns: &[usize]) -> Vec<Vec<f64>> {
    text.lines()
        .skip(header_lines)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            columns
                .iter()
                .map(|&column| {
                    fields
                        .get(column)
                        .and_then(|field| field.parse::<f64>().ok())
                        .filter(|value| value.is_finite())
                })
                .collect::<Option<Vec<f64>>>()
        })
        .collect()
}

/// A log's rows as (time, height in metres), in file order.
///
/// Each row is split at commas and the two columns trimmed and parsed; a row where either doesn't
/// parse as a number is skipped (NDRT 2020's altitude columns end before its acceleration columns
/// do). Rows past [`Log::until_s`] are not read.
///
/// # Errors
///
/// [`RealFlightError::Input`] if fewer than two rows are read, or times decrease.
pub fn parse_log(id: &str, log: &Log, text: &str) -> Result<Vec<(f64, f64)>, RealFlightError> {
    let rows: Vec<(f64, f64)> = read_rows(
        text,
        log.header_lines,
        &[log.time_column, log.height_column],
    )
    .into_iter()
    .take_while(|row| log.until_s.is_none_or(|until| row[0] <= until))
    .map(|row| (row[0], row[1] * log.metres_per_unit))
    .collect();
    let input = |what: String| RealFlightError::Input {
        flight: id.to_owned(),
        what,
    };
    if rows.len() < 2 {
        return Err(input(format!(
            "the log {} has {} rows",
            log.file,
            rows.len()
        )));
    }
    if let Some(i) = rows.windows(2).position(|w| w[1].0 < w[0].0) {
        return Err(input(format!(
            "the log {}'s time goes back at row {}",
            log.file,
            i + 1
        )));
    }
    Ok(rows)
}

/// A thrust file as RocketPy 1.13.0 reads it, with the example's burn options.
///
/// - An `.eng` file (`Motor.import_eng`, rocketpy/motors/motor.py:1130-1150): `;` starts a
///   comment, the first line left is the description, and each later one gives a time and a
///   thrust; a `(0, 0)` point is put first. So a file whose first data line is `0 0` has that
///   line read as its description.
/// - A `.csv` file: every row a time and a thrust, nothing put first.
/// - `reshape` (`Motor.reshape_thrust_curve`, motor.py:937-960): times scaled to the burn time and
///   moved to start at zero, then thrusts scaled so the trapezoidal integral is the impulse.
/// - The burn time (`Motor.clip_thrust`, motor.py:985-1024): at most the last time; the points
///   strictly inside `(0, burn time)` kept, with the curve's linear value added at each end.
///
/// hpr's curve refuses a negative thrust, so any is read as zero, and the impulse this removes is
/// returned with the curve.
///
/// # Errors
///
/// [`RealFlightError::Input`] for a file with no points, a line that doesn't read, or times that
/// decrease.
pub fn parse_thrust(
    id: &str,
    name: &str,
    text: &str,
    burn_time_s: Option<f64>,
    reshape: Option<(f64, f64)>,
) -> Result<(Vec<(f64, f64)>, f64), RealFlightError> {
    let input = |what: String| RealFlightError::Input {
        flight: id.to_owned(),
        what,
    };
    let number = |field: &str| field.trim().parse::<f64>().ok();
    let mut points: Vec<(f64, f64)> = Vec::new();
    if name.ends_with(".eng") {
        points.push((0.0, 0.0));
        let mut described = false;
        for line in text.lines() {
            let line = line.split(';').next().unwrap_or_default();
            if line.trim().is_empty() {
                continue;
            }
            if !described {
                described = true;
                continue;
            }
            let mut fields = line.split_whitespace();
            match (
                fields.next().and_then(number),
                fields.next().and_then(number),
            ) {
                (Some(t), Some(f)) => points.push((t, f)),
                _ => return Err(input(format!("{name}: the line {line:?} doesn't read"))),
            }
        }
    } else {
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let mut fields = line.split(',');
            match (
                fields.next().and_then(number),
                fields.next().and_then(number),
            ) {
                (Some(t), Some(f)) => points.push((t, f)),
                _ => return Err(input(format!("{name}: the line {line:?} doesn't read"))),
            }
        }
    }
    if points.len() < 2 || points.windows(2).any(|w| w[1].0 < w[0].0) {
        return Err(input(format!(
            "{name} has {} points or times that go back",
            points.len()
        )));
    }
    let integral = |points: &[(f64, f64)]| -> f64 {
        points
            .windows(2)
            .map(|w| 0.5 * (w[1].0 - w[0].0) * (w[0].1 + w[1].1))
            .sum()
    };
    let mut burn = burn_time_s;
    if let Some((burn_s, impulse_ns)) = reshape {
        let (first, last) = (points[0].0, points[points.len() - 1].0);
        let scale = burn_s / (last - first);
        let start = scale * first;
        let moved: Vec<(f64, f64)> = points
            .iter()
            .map(|&(t, f)| (scale * t - start, f))
            .collect();
        let factor = impulse_ns / integral(&moved);
        points = moved.into_iter().map(|(t, f)| (t, factor * f)).collect();
        burn = Some(burn_s);
    }
    let last = points[points.len() - 1].0;
    let first = points[0].0;
    let end = burn.map_or(last, |b| b.min(last));
    let at = |t: f64| -> f64 {
        // Linear, and zero outside, as RocketPy's Function with "zero" extrapolation.
        if t < first || t > last {
            return 0.0;
        }
        let i = points.partition_point(|p| p.0 <= t).max(1) - 1;
        let (t0, f0) = points[i];
        match points.get(i + 1) {
            Some(&(t1, f1)) if t1 > t0 => f0 + (f1 - f0) * (t - t0) / (t1 - t0),
            _ => f0,
        }
    };
    let begin = first.max(0.0);
    let mut clipped = vec![(begin, at(begin))];
    clipped.extend(points.iter().copied().filter(|p| p.0 > begin && p.0 < end));
    clipped.push((end, at(end)));
    let before = integral(&clipped);
    for point in &mut clipped {
        point.1 = point.1.max(0.0);
    }
    let removed = integral(&clipped) - before;
    Ok((clipped, removed))
}

/// The first time a trace reaches `height`, linearly interpolated between its rows.
fn crossing(rows: &[(f64, f64)], height: f64) -> Option<f64> {
    let i = rows.iter().position(|row| row.1 >= height)?;
    if i == 0 {
        return None;
    }
    let ((t0, h0), (t1, h1)) = (rows[i - 1], rows[i]);
    Some(t0 + (t1 - t0) * (height - h0) / (h1 - h0))
}

/// The trace's value at `t`, linearly interpolated; `None` outside it.
fn value_at(rows: &[(f64, f64)], t: f64) -> Option<f64> {
    let i = rows.partition_point(|row| row.0 <= t);
    if i == 0 || i > rows.len() {
        return None;
    }
    let (t0, h0) = rows[i - 1];
    match rows.get(i) {
        Some(&(t1, h1)) => Some(h0 + (h1 - h0) * (t - t0) / (t1 - t0)),
        None => (t == t0).then_some(h0),
    }
}

/// The highest row of a trace: its time and height.
fn highest(rows: &[(f64, f64)]) -> (f64, f64) {
    rows.iter()
        .copied()
        .fold((f64::NAN, f64::NEG_INFINITY), |best, row| {
            if row.1 > best.1 { row } else { best }
        })
}

/// What the trace comparison gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceComparison {
    /// The log's time from the alignment height to its apogee, s.
    pub log_time_to_apogee_s: f64,
    /// hpr's, s.
    pub hpr_time_to_apogee_s: f64,
    /// The RMS of hpr's heights less the log's, m.
    pub rms_m: f64,
    /// The log rows it is over.
    pub rows: usize,
}

/// The ascent of `hpr` (time since ignition, height) against the log's, each aligned where it
/// first reaches [`ALIGN_HEIGHT_M`], over the log's rows from there to the first of the two
/// apogees.
///
/// `hpr_apogee_s` is hpr's apogee time from its own event, since its grid may miss the top.
///
/// # Errors
///
/// A message when a trace never reaches the alignment height or no row falls in the window.
pub fn compare_traces(
    log: &[(f64, f64)],
    hpr: &[(f64, f64)],
    hpr_apogee_s: f64,
) -> Result<TraceComparison, String> {
    let log_align = crossing(log, ALIGN_HEIGHT_M)
        .ok_or_else(|| format!("the log never rises through {ALIGN_HEIGHT_M} m"))?;
    let hpr_align = crossing(hpr, ALIGN_HEIGHT_M)
        .ok_or_else(|| format!("hpr never rises through {ALIGN_HEIGHT_M} m"))?;
    let (log_apogee_s, _) = highest(log);
    let log_time = log_apogee_s - log_align;
    let hpr_time = hpr_apogee_s - hpr_align;
    let window = log_time.min(hpr_time);
    let mut sum = 0.0;
    let mut rows = 0;
    for &(t, h) in log {
        let since = t - log_align;
        if !(0.0..=window).contains(&since) {
            continue;
        }
        let Some(ours) = value_at(hpr, hpr_align + since) else {
            return Err(format!("hpr has no height {since} s after its alignment"));
        };
        sum += (ours - h).powi(2);
        rows += 1;
    }
    if rows == 0 {
        return Err("no log row falls between the alignment and the first apogee".to_owned());
    }
    Ok(TraceComparison {
        log_time_to_apogee_s: log_time,
        hpr_time_to_apogee_s: hpr_time,
        rms_m: (sum / rows as f64).sqrt(),
        rows,
    })
}

/// A barometric altimeter on a pad, in the air of a day: it reads the standard atmosphere's
/// altitude of the day's pressure where it is, less that at the pad, `H(p(z)) − H(p(z₀))`
/// ([`Ussa76::pressure_altitude_m`]).
///
/// In the standard atmosphere itself it reads the geopotential height climbed. On a warmer day
/// the air is thinner, pressure falls more slowly with height, and it reads less than that: in
/// the troposphere, with the sea-level pressure unchanged, by the ratio of the standard's
/// sea-level temperature to the day's.
#[derive(Debug)]
pub struct Barometer<'a> {
    day: &'a dyn Atmosphere,
    site_msl_m: f64,
    standard: Ussa76,
    pad_altitude_m: f64,
}

impl<'a> Barometer<'a> {
    /// A barometer on a pad `pad_m` above a site `site_msl_m` above sea level, in `day`'s air.
    ///
    /// # Errors
    ///
    /// [`hpr_atmos::AtmosError`] if the day has no air at the pad.
    pub fn on_pad(
        day: &'a dyn Atmosphere,
        site_msl_m: f64,
        pad_m: f64,
    ) -> Result<Self, hpr_atmos::AtmosError> {
        let standard = Ussa76::standard();
        let pad_altitude_m =
            standard.pressure_altitude_m(day.air(site_msl_m + pad_m)?.air.pressure_pa)?;
        Ok(Self {
            day,
            site_msl_m,
            standard,
            pad_altitude_m,
        })
    }

    /// What it reads at `height_m` above the site, m.
    ///
    /// # Errors
    ///
    /// [`hpr_atmos::AtmosError`] if the day has no air there.
    pub fn reading_m(&self, height_m: f64) -> Result<f64, hpr_atmos::AtmosError> {
        let pressure_pa = self.day.air(self.site_msl_m + height_m)?.air.pressure_pa;
        Ok(self.standard.pressure_altitude_m(pressure_pa)? - self.pad_altitude_m)
    }
}

/// Keeps hpr's height above its start on a fixed grid of the dense output.
struct Heights {
    /// The next grid point's index: its time is `GRID_S` times it.
    next: u32,
    rows: Vec<(f64, f64)>,
}

impl Observer for Heights {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        loop {
            let t = GRID_S * f64::from(self.next);
            if t > step.end_s() {
                return Ok(());
            }
            // Steps follow one another, so a grid time before this step's start was the last
            // step's end, already kept.
            if t >= step.start_s() {
                let sample = step.sample(t)?;
                self.rows.push((t, sample.height_above_ground_m));
            }
            self.next += 1;
        }
    }
}

/// The flight's case in [`MASS_FIXTURE`], the design fixture's record of the example.
fn fixture_case(root: &Path, flight: &RealFlight) -> Result<Value, RealFlightError> {
    let input = |what: String| RealFlightError::Input {
        flight: flight.id.to_owned(),
        what,
    };
    let bytes = read_bytes(root, MASS_FIXTURE, "reading the design fixture")?;
    let mut fixture: Value = serde_json::from_slice(&bytes)
        .map_err(|error| input(format!("{MASS_FIXTURE}: {error}")))?;
    let name = flight.design.trim_start_matches("rocketpy-");
    fixture
        .get_mut("cases")
        .and_then(Value::as_array_mut)
        .and_then(|cases| {
            cases
                .iter_mut()
                .find(|case| case["name"] == name)
                .map(Value::take)
        })
        .ok_or_else(|| input(format!("{MASS_FIXTURE} has no case {name}")))
}

/// The example's own thrust file, by its path from the repository root, with its burn time and
/// its reshape (a burn time and an impulse), each when the example gives one.
type ExampleThrust = (String, Option<f64>, Option<(f64, f64)>);

/// A file read, by its path from the repository root, and its bytes.
type ReadFile = (String, Vec<u8>);

/// The example's own thrust file and burn options, from [`MASS_FIXTURE`]'s record of the case.
fn example_thrust(root: &Path, flight: &RealFlight) -> Result<ExampleThrust, RealFlightError> {
    let input = |what: String| RealFlightError::Input {
        flight: flight.id.to_owned(),
        what,
    };
    let case = fixture_case(root, flight)?;
    let name = flight.design.trim_start_matches("rocketpy-");
    let file = case["original_thrust_source"]
        .as_str()
        .ok_or_else(|| input(format!("{name} records no thrust file")))?;
    let options = &case["original_thrust_options"];
    let burn = options["burn_time"].as_f64();
    let reshape = match options["reshape_thrust_curve"].as_array() {
        Some(pair) => match (
            pair.first().and_then(Value::as_f64),
            pair.get(1).and_then(Value::as_f64),
        ) {
            (Some(time), Some(impulse)) => Some((time, impulse)),
            _ => {
                return Err(input(format!(
                    "{name}'s reshape is not a burn time and an impulse"
                )));
            }
        },
        None => None,
    };
    // The fixture records the file's name; RocketPy keeps motors by maker.
    let motors = root.join(ROCKETPY).join("data/motors");
    let entries = fs::read_dir(&motors).map_err(|source| RealFlightError::Io {
        what: "listing RocketPy's motors",
        path: motors.display().to_string(),
        source,
    })?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(file))
        .filter(|path| path.is_file())
        .collect();
    found.sort();
    match found.as_slice() {
        [path] => {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            Ok((relative, burn, reshape))
        }
        _ => Err(input(format!(
            "{file} is in {} of RocketPy's motor folders, not one",
            found.len()
        ))),
    }
}

/// A satellite log's apogee: its highest altitude above its first row, the pad's, m.
///
/// The first row must be within [`GNSS_PAD_TOLERANCE_M`] of the site's elevation (a receiver
/// without a fix reads far from it), and the apogee must be above the pad.
fn satellite_apogee_m(gnss: &Gnss, text: &str, site_elevation_m: f64) -> Result<f64, String> {
    let rows = read_rows(
        text,
        gnss.header_lines,
        &[gnss.time_column, gnss.altitude_column],
    );
    let pad = rows
        .first()
        .map(|row| row[1])
        .ok_or_else(|| format!("{} has no altitude", gnss.file))?;
    if (pad * gnss.metres_per_unit - site_elevation_m).abs() > GNSS_PAD_TOLERANCE_M {
        return Err(format!(
            "{}'s first altitude, {} m, is not the site's {site_elevation_m} m",
            gnss.file,
            pad * gnss.metres_per_unit
        ));
    }
    let apogee_m = rows
        .iter()
        .map(|row| (row[1] - pad) * gnss.metres_per_unit)
        .fold(f64::NEG_INFINITY, f64::max);
    if apogee_m > 0.0 {
        Ok(apogee_m)
    } else {
        Err(format!("{} never rises above its first row", gnss.file))
    }
}

/// How far a satellite log's first altitude may be from the site's elevation, m: satellite
/// heights are over the ellipsoid or a geoid model, and the example's elevation is its own
/// (Juno III's pad reads 83 m below its site's).
const GNSS_PAD_TOLERANCE_M: f64 = 150.0;

/// The largest gap between a log's height column and the standard atmosphere's reading of its
/// pressure column less that of the first row with one, over the rows up to [`Log::until_s`]
/// whose time, height and pressure all read, m.
fn pressure_reading_gap_m(
    log: &Log,
    column: usize,
    pascals_per_unit: f64,
    text: &str,
) -> Result<f64, String> {
    let standard = Ussa76::standard();
    let rows: Vec<Vec<f64>> = read_rows(
        text,
        log.header_lines,
        &[log.time_column, log.height_column, column],
    )
    .into_iter()
    .take_while(|row| log.until_s.is_none_or(|until| row[0] <= until))
    .collect();
    let altitude = |row: &[f64]| {
        standard
            .pressure_altitude_m(row[2] * pascals_per_unit)
            .map_err(|error| format!("{}: {error}", log.file))
    };
    let pad = altitude(
        rows.first()
            .ok_or_else(|| format!("{} has no pressure", log.file))?,
    )?;
    rows.iter().try_fold(0.0_f64, |gap, row| {
        Ok(gap.max((row[1] * log.metres_per_unit - (altitude(row)? - pad)).abs()))
    })
}

/// One flight of hpr, as the log's altimeter would read it.
struct Flown {
    apogee_m: f64,
    height_apogee_m: f64,
    trace: TraceComparison,
    total_impulse_ns: f64,
}

/// Flies one flight and compares it with its log.
///
/// # Errors
///
/// [`RealFlightError`] if a file is missing or doesn't read, or hpr can't fly it.
pub fn fly(root: &Path, flight: &RealFlight) -> Result<FlightRow, RealFlightError> {
    let input = |what: String| RealFlightError::Input {
        flight: flight.id.to_owned(),
        what,
    };
    let sim = |source: SimError| RealFlightError::Sim {
        flight: flight.id.to_owned(),
        source,
    };
    let mut files = Vec::new();
    let mut read = |relative: String, what: &'static str| -> Result<Vec<u8>, RealFlightError> {
        let bytes = read_bytes(root, &relative, what)?;
        files.push(FileRead {
            sha256: sha256_hex(&bytes),
            path: relative,
        });
        Ok(bytes)
    };

    // The log.
    let log_path = format!("{ROCKETPY}/data/rockets/{}", flight.log.file);
    let log_bytes = read(log_path, "reading a flight log")?;
    let log_text = String::from_utf8_lossy(&log_bytes);
    let log = parse_log(flight.id, &flight.log, &log_text)?;
    let (log_apogee_s, log_apogee_m) = highest(&log);
    let pressure_reading_max_m = match flight.log.pressure {
        Some((column, pascals_per_unit)) => Some(
            pressure_reading_gap_m(&flight.log, column, pascals_per_unit, &log_text)
                .map_err(input)?,
        ),
        None => None,
    };
    let gnss_apogee_m = match flight.log.gnss {
        Some(gnss) => {
            let apogee_m = if gnss.file == flight.log.file {
                satellite_apogee_m(&gnss, &log_text, flight.site.2)
            } else {
                let path = format!("{ROCKETPY}/data/rockets/{}", gnss.file);
                let bytes = read(path, "reading a satellite log")?;
                satellite_apogee_m(&gnss, &String::from_utf8_lossy(&bytes), flight.site.2)
            };
            Some(apogee_m.map_err(input)?)
        }
        None => None,
    };

    // The design, with the example's own thrust, burn options and radius from the fixture.
    read(MASS_FIXTURE.to_owned(), "reading the design fixture")?;
    let (thrust_path, burn, reshape) = example_thrust(root, flight)?;
    let thrust_bytes = read(thrust_path.clone(), "reading a thrust file")?;
    let (curve, zeroed_ns) = parse_thrust(
        flight.id,
        &thrust_path,
        &String::from_utf8_lossy(&thrust_bytes),
        burn,
        reshape,
    )?;
    let design_path = format!("validation/designs/{}.json", flight.design);
    let design_bytes = read(design_path.clone(), "reading a design")?;
    let design: Value = serde_json::from_slice(&design_bytes)
        .map_err(|error| input(format!("{design_path}: {error}")))?;
    let with_thrust = |curve: &[(f64, f64)]| -> Result<Rocket, RealFlightError> {
        let mut design = design.clone();
        let motor = design
            .pointer_mut("/configurations/0/motors/0")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| input(format!("{design_path} has no motor")))?;
        let (times, thrusts): (Vec<f64>, Vec<f64>) = curve.iter().copied().unzip();
        motor.insert(
            "designation".to_owned(),
            Value::String(format!(
                "RocketPy's {}",
                thrust_path.rsplit('/').next().unwrap_or("")
            )),
        );
        motor
            .get_mut("motor")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| input(format!("{design_path}'s motor has no inputs")))?
            .insert(
                "curve".to_owned(),
                serde_json::json!({ "times_s": times, "thrusts_n": thrusts }),
            );
        serde_json::from_value(design)
            .map_err(|error| input(format!("{design_path} with a thrust file's curve: {error}")))
    };
    let rocket = with_thrust(&curve)?;
    // Where the example reshapes its thrust file, the file as recorded, for the diagnostic.
    let recorded = match reshape {
        Some(_) => Some(with_thrust(
            &parse_thrust(
                flight.id,
                &thrust_path,
                &String::from_utf8_lossy(&thrust_bytes),
                burn,
                None,
            )?
            .0,
        )?),
        None => None,
    };

    // The weather.
    let weather_path = format!("{ROCKETPY}/data/weather/{}", flight.weather);
    let weather_bytes = read(weather_path.clone(), "reading an ERA5 file")?;
    let file =
        NetCdf::parse(&weather_bytes).map_err(|error| input(format!("{weather_path}: {error}")))?;
    let (year, month, day, hour) = flight.utc;
    let time = UtcTime::from_civil(year, month, day, hour, 0, 0.0)
        .map_err(|error| input(error.to_string()))?;
    let (latitude_deg, longitude_deg, elevation_m) = flight.site;
    let profile = Era5Profile::read(
        &file,
        Era5Request {
            latitude_deg,
            longitude_deg,
            time,
        },
    )
    .map_err(|error| input(format!("{weather_path}: {error}")))?;
    let sounding = profile
        .sounding(WindInterpolation::Components)
        .map_err(|error| input(format!("{weather_path}: {error}")))?;
    let wind = sounding
        .wind()
        .ok_or_else(|| input(format!("{weather_path} gives no wind")))?
        .clone();

    let site = Geodetic::from_degrees(latitude_deg, longitude_deg, elevation_m)
        .map_err(|error| sim(error.into()))?;
    let earth = Earth::wgs84(site).map_err(|error| sim(error.into()))?;
    let weather = sounding.clone();
    let environment = Environment::new(earth, sounding, wind);
    let (length_m, inclination_deg, heading_deg) = flight.rail;
    let rail = Rail {
        length_m,
        azimuth_rad: heading_deg.to_radians(),
        elevation_rad: inclination_deg.to_radians(),
        roll_rad: 0.0,
        friction_coefficient: 0.0,
    };
    let settings = FlightSettings {
        max_time_s: log_apogee_s + PAST_APOGEE_S,
        ..FlightSettings::default()
    };

    // The example's drag, on the example's radius (the design fixture's record of the rocket).
    let (table, drag_files) = example_drag_table(root, flight)?;
    for (relative, bytes) in drag_files {
        files.push(FileRead {
            sha256: sha256_hex(&bytes),
            path: relative,
        });
    }

    let fly_once = |rocket: &Rocket, table: Option<DragTable>| -> Result<Flown, RealFlightError> {
        let simulation =
            Simulation::new(rocket, "example", environment.clone(), rail, settings).map_err(sim)?;
        let simulation = match table {
            Some(table) => simulation.with_drag_table(table),
            None => simulation,
        };
        let total_impulse_ns = simulation
            .assembly()
            .motors
            .iter()
            .map(|motor| motor.mounted.motor.curve().total_impulse_ns())
            .sum();
        let mut heights = Heights {
            next: 0,
            rows: Vec::new(),
        };
        let result = simulation.run(&mut heights).map_err(sim)?;
        let apogee = result
            .event(EventKind::Apogee)
            .ok_or_else(|| input("hpr's flight has no apogee".to_owned()))?
            .sample;
        let start_m = heights
            .rows
            .first()
            .map(|row| row.1)
            .ok_or_else(|| input("hpr's flight has no steps".to_owned()))?;
        // What the log's altimeter would read at a height above the site, above its start.
        let atmos = |error: hpr_atmos::AtmosError| input(format!("{weather_path}: {error}"));
        let barometer = Barometer::on_pad(&weather, elevation_m, start_m).map_err(atmos)?;
        let reading = |height_m: f64| -> Result<f64, RealFlightError> {
            match flight.log.altimeter {
                Altimeter::Height(_) => Ok(height_m - start_m),
                Altimeter::Barometric(_) | Altimeter::AssumedBarometric(_) => {
                    barometer.reading_m(height_m).map_err(atmos)
                }
            }
        };
        let hpr = heights
            .rows
            .iter()
            .map(|&(t, h)| Ok((t, reading(h)?)))
            .collect::<Result<Vec<(f64, f64)>, RealFlightError>>()?;
        let trace = compare_traces(&log, &hpr, apogee.time_s).map_err(input)?;
        Ok(Flown {
            apogee_m: reading(apogee.height_above_ground_m)?,
            height_apogee_m: apogee.height_above_ground_m - start_m,
            trace,
            total_impulse_ns,
        })
    };
    let own = fly_once(&rocket, None)?;
    let on_example_drag = fly_once(&rocket, Some(table))?;
    let on_recorded_thrust = recorded
        .as_ref()
        .map(|rocket| fly_once(rocket, None))
        .transpose()?;
    let (hpr_apogee_m, trace, total_impulse_ns) = (own.apogee_m, own.trace, own.total_impulse_ns);
    let (drag_apogee_m, drag_trace) = (on_example_drag.apogee_m, on_example_drag.trace);
    let error = |apogee_m: f64| percent_of(apogee_m, log_apogee_m);

    Ok(FlightRow {
        id: flight.id.to_owned(),
        title: flight.title.to_owned(),
        design: flight.design.to_owned(),
        source: flight.source.to_owned(),
        files,
        weather_time_utc: format!("{year:04}-{month:02}-{day:02}T{hour:02}:00Z"),
        log_rows: log.len(),
        log_until_s: flight.log.until_s,
        altimeter: flight.log.altimeter.kind().to_owned(),
        altimeter_evidence: flight.log.altimeter.evidence().to_owned(),
        log_apogee_m,
        hpr_apogee_m,
        hpr_height_apogee_m: own.height_apogee_m,
        apogee_error_percent: error(hpr_apogee_m),
        log_time_to_apogee_s: trace.log_time_to_apogee_s,
        hpr_time_to_apogee_s: trace.hpr_time_to_apogee_s,
        trace_rms_m: trace.rms_m,
        trace_rms_percent: 100.0 * trace.rms_m / log_apogee_m,
        trace_rows: trace.rows,
        total_impulse_ns,
        negative_thrust_zeroed_ns: zeroed_ns,
        example_drag_source: flight.example_drag_source.to_owned(),
        example_drag_apogee_m: drag_apogee_m,
        example_drag_apogee_error_percent: error(drag_apogee_m),
        example_drag_trace_rms_m: drag_trace.rms_m,
        recorded_thrust: on_recorded_thrust.map(|flown| RecordedThrust {
            impulse_ns: flown.total_impulse_ns,
            apogee_m: flown.apogee_m,
            apogee_error_percent: error(flown.apogee_m),
        }),
        pressure_reading_max_m,
        gnss_apogee_m,
        inputs_sha256: sha256_hex(format!("{flight:?}").as_bytes()),
        note: flight.note.to_owned(),
        explanation_kind: flight.explanation.kind().to_owned(),
        explanation: flight.explanation.text().to_owned(),
    })
}

/// The first row of each run of rows with the same Mach number, as `cargo xtask aero` reads
/// RocketPy's curves: Cavour's repeats 13 Mach numbers below 0.107, some with values 0.001 apart.
fn first_row_per_mach(rows: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut kept: Vec<(f64, f64)> = Vec::with_capacity(rows.len());
    for row in rows {
        if kept.last().is_none_or(|last| last.0 != row.0) {
            kept.push(row);
        }
    }
    kept
}

/// The example's drag as a table on its radius, and the files it was read from.
fn example_drag_table(
    root: &Path,
    flight: &RealFlight,
) -> Result<(DragTable, Vec<ReadFile>), RealFlightError> {
    let input = |what: String| RealFlightError::Input {
        flight: flight.id.to_owned(),
        what,
    };
    let table = |points: Vec<(f64, f64)>| -> Result<Table1D, RealFlightError> {
        let (machs, coefficients): (Vec<f64>, Vec<f64>) = points.into_iter().unzip();
        Table1D::new(
            machs,
            coefficients,
            Interpolation::Linear,
            Extrapolation::Clamp,
        )
        .map_err(|error| input(format!("the example's drag: {error}")))
    };
    let mut files = Vec::new();
    let (power_off, power_on) = match flight.example_drag {
        ExampleDrag::Constant(cd) => (table(vec![(0.0, cd), (1.0, cd)])?, None),
        ExampleDrag::Knots {
            points,
            power_on_factor,
        } => (
            table(points.to_vec())?,
            Some(table(
                points
                    .iter()
                    .map(|&(mach, cd)| (mach, power_on_factor * cd))
                    .collect(),
            )?),
        ),
        ExampleDrag::Files {
            power_off,
            power_on,
            scaled_to,
        } => {
            let mut curves = Vec::new();
            for file in [power_off, power_on] {
                let relative = format!("{ROCKETPY}/data/rockets/{file}");
                let bytes = read_bytes(root, &relative, "reading a drag curve")?;
                let mut rows = Vec::new();
                for line in String::from_utf8_lossy(&bytes).lines() {
                    let mut fields = line.split(',').map(str::trim);
                    match (
                        fields.next().map(str::parse::<f64>),
                        fields.next().map(str::parse::<f64>),
                    ) {
                        (Some(Ok(mach)), Some(Ok(cd))) => rows.push((mach, cd)),
                        _ if line.trim().is_empty() => {}
                        _ => return Err(input(format!("{file}: the line {line:?} doesn't read"))),
                    }
                }
                curves.push(first_row_per_mach(rows));
                if power_on != power_off || files.is_empty() {
                    files.push((relative, bytes));
                }
            }
            let [off, on] = <[Vec<(f64, f64)>; 2]>::try_from(curves)
                .map_err(|_| input("two drag curves".to_owned()))?;
            let off_table = table(off.clone())?;
            let factor = match scaled_to {
                Some((mach, cd)) => {
                    cd / off_table
                        .eval(mach)
                        .map_err(|error| input(error.to_string()))?
                }
                None => 1.0,
            };
            let scale =
                |rows: Vec<(f64, f64)>| rows.into_iter().map(|(m, c)| (m, factor * c)).collect();
            (table(scale(off))?, Some(table(scale(on))?))
        }
    };
    let radius_m = example_radius_m(root, flight)?;
    Ok((
        DragTable::new(power_off, power_on).with_reference_diameter_m(2.0 * radius_m),
        files,
    ))
}

/// The example rocket's radius, from [`MASS_FIXTURE`]'s record of it.
fn example_radius_m(root: &Path, flight: &RealFlight) -> Result<f64, RealFlightError> {
    let case = fixture_case(root, flight)?;
    case["rocket"]["radius"]
        .as_f64()
        .ok_or_else(|| RealFlightError::Input {
            flight: flight.id.to_owned(),
            what: format!("{MASS_FIXTURE} records no radius"),
        })
}

/// `value` less `reference`, per cent of `reference`.
fn percent_of(value: f64, reference: f64) -> f64 {
    100.0 * (value - reference) / reference
}

/// The summary of `rows`.
#[must_use]
pub fn summarise(rows: &[FlightRow]) -> Summary {
    let n = rows.len().max(1) as f64;
    let mean_absolute = rows
        .iter()
        .map(|row| row.apogee_error_percent.abs())
        .sum::<f64>()
        / n;
    Summary {
        flights: rows.len(),
        mean_absolute_apogee_error_percent: mean_absolute,
        target_percent: APOGEE_TARGET_PERCENT,
        within_target: mean_absolute <= APOGEE_TARGET_PERCENT,
        mean_apogee_error_percent: rows.iter().map(|row| row.apogee_error_percent).sum::<f64>() / n,
        mean_absolute_height_apogee_error_percent: rows
            .iter()
            .map(|row| percent_of(row.hpr_height_apogee_m, row.log_apogee_m).abs())
            .sum::<f64>()
            / n,
        mean_absolute_known_barometric_apogee_error_percent: rows
            .iter()
            .map(|row| {
                let apogee_m = if row.altimeter == Altimeter::AssumedBarometric("").kind() {
                    row.hpr_height_apogee_m
                } else {
                    row.hpr_apogee_m
                };
                percent_of(apogee_m, row.log_apogee_m).abs()
            })
            .sum::<f64>()
            / n,
        outliers: rows
            .iter()
            .filter(|row| row.apogee_error_percent.abs() > APOGEE_TARGET_PERCENT)
            .map(|row| row.id.clone())
            .collect(),
        max_trace_rms_percent: rows
            .iter()
            .map(|row| row.trace_rms_percent)
            .fold(0.0, f64::max),
        example_drag_mean_absolute_apogee_error_percent: rows
            .iter()
            .map(|row| row.example_drag_apogee_error_percent.abs())
            .sum::<f64>()
            / n,
    }
}

/// Flies every flight in [`FLIGHTS`] and builds the report.
///
/// # Errors
///
/// The first flight's [`RealFlightError`].
pub fn run(root: &Path) -> Result<RealFlightReport, RealFlightError> {
    let flights = FLIGHTS
        .iter()
        .map(|flight| fly(root, flight))
        .collect::<Result<Vec<_>, _>>()?;
    let summary = summarise(&flights);
    Ok(RealFlightReport {
        generated_by: "cargo xtask real-flights".to_owned(),
        align_height_m: ALIGN_HEIGHT_M,
        grid_s: GRID_S,
        flights,
        summary,
    })
}

impl RealFlightReport {
    /// Whether the report holds together without the files it was flown from, as CI checks it:
    /// each row's percentages are its metres', the summary is the rows', every outlier has an
    /// explanation that holds and no other flight has one, and the page is this report's
    /// rendering.
    ///
    /// # Errors
    ///
    /// The first thing that doesn't hold, in words.
    pub fn check_consistent(&self, markdown: &str) -> Result<(), String> {
        if self.summary != summarise(&self.flights) {
            return Err("the summary is not the rows'".to_owned());
        }
        let same = crate::report::same_but_for_platform_rounding;
        for row in &self.flights {
            for (what, kept, derived) in [
                (
                    "apogee error",
                    row.apogee_error_percent,
                    percent_of(row.hpr_apogee_m, row.log_apogee_m),
                ),
                (
                    "apogee error on the example's drag",
                    row.example_drag_apogee_error_percent,
                    percent_of(row.example_drag_apogee_m, row.log_apogee_m),
                ),
                (
                    "trace RMS share",
                    row.trace_rms_percent,
                    100.0 * row.trace_rms_m / row.log_apogee_m,
                ),
                (
                    "apogee error on the recorded thrust",
                    row.recorded_thrust
                        .as_ref()
                        .map_or(0.0, |flown| flown.apogee_error_percent),
                    row.recorded_thrust
                        .as_ref()
                        .map_or(0.0, |flown| percent_of(flown.apogee_m, row.log_apogee_m)),
                ),
            ] {
                if !same(kept, derived) {
                    return Err(format!(
                        "{}: the {what} is {kept}, its metres give {derived}",
                        row.id
                    ));
                }
            }
            let outside = row.apogee_error_percent.abs() > APOGEE_TARGET_PERCENT;
            if outside == row.explanation.trim().is_empty() {
                return Err(format!(
                    "{}: {}",
                    row.id,
                    if outside {
                        "an outlier with no explanation"
                    } else {
                        "an explanation on a flight within the target"
                    }
                ));
            }
            let (own, example) = (
                row.apogee_error_percent,
                row.example_drag_apogee_error_percent,
            );
            let recorded = row
                .recorded_thrust
                .as_ref()
                .map(|flown| flown.apogee_error_percent);
            let holds = match row.explanation_kind.as_str() {
                "" => !outside,
                "drag" => {
                    example.abs() <= APOGEE_TARGET_PERCENT
                        && recorded.is_none_or(|error| error.abs() > APOGEE_TARGET_PERCENT)
                }
                "thrust" => {
                    recorded.is_some_and(|error| error.abs() <= APOGEE_TARGET_PERCENT)
                        && example.abs() > APOGEE_TARGET_PERCENT
                }
                kind => return Err(format!("{}: no explanation is called {kind:?}", row.id)),
            };
            if !holds {
                return Err(format!(
                    "{}: the explanation {:?} doesn't hold: {own:+.3}% on hpr's drag, \
                     {example:+.3}% on the example's, {recorded:?}% on the recorded thrust",
                    row.id, row.explanation_kind
                ));
            }
        }
        if markdown != self.to_markdown() {
            return Err(format!("{REPORT_MD} is not {REPORT_JSON}'s rendering"));
        }
        Ok(())
    }

    /// Whether `other`, a run of today, reproduces this committed report: the same flights, files,
    /// words and counts, and numbers the same but for the platform's last digits.
    ///
    /// # Errors
    ///
    /// The first difference, in words.
    pub fn reproduces(&self, other: &RealFlightReport) -> Result<(), String> {
        let same = crate::report::same_but_for_platform_rounding;
        let recorded = |row: &FlightRow, value: fn(&RecordedThrust) -> f64| {
            row.recorded_thrust.as_ref().map_or(0.0, value)
        };
        if self.flights.len() != other.flights.len() {
            return Err(format!(
                "{} flights committed, {} flown",
                self.flights.len(),
                other.flights.len()
            ));
        }
        for (a, b) in self.flights.iter().zip(&other.flights) {
            let differs = [
                ("id", a.id != b.id),
                ("title", a.title != b.title),
                ("design", a.design != b.design),
                ("source", a.source != b.source),
                ("files read or their digests", a.files != b.files),
                ("weather time", a.weather_time_utc != b.weather_time_utc),
                ("log rows", a.log_rows != b.log_rows),
                ("log cut", a.log_until_s != b.log_until_s),
                ("altimeter", a.altimeter != b.altimeter),
                (
                    "altimeter's evidence",
                    a.altimeter_evidence != b.altimeter_evidence,
                ),
                ("trace rows", a.trace_rows != b.trace_rows),
                ("note", a.note != b.note),
                (
                    "explanation's kind",
                    a.explanation_kind != b.explanation_kind,
                ),
                ("explanation", a.explanation != b.explanation),
                (
                    "example drag's source",
                    a.example_drag_source != b.example_drag_source,
                ),
                ("inputs", a.inputs_sha256 != b.inputs_sha256),
                (
                    "recorded-thrust flight",
                    a.recorded_thrust.is_some() != b.recorded_thrust.is_some(),
                ),
                (
                    "pressure column",
                    a.pressure_reading_max_m.is_some() != b.pressure_reading_max_m.is_some(),
                ),
                (
                    "satellite log",
                    a.gnss_apogee_m.is_some() != b.gnss_apogee_m.is_some(),
                ),
            ];
            if let Some((what, _)) = differs.iter().find(|(_, differ)| *differ) {
                return Err(format!("{}: the {what} differs", a.id));
            }
            for (what, x, y) in [
                ("log apogee", a.log_apogee_m, b.log_apogee_m),
                ("hpr apogee", a.hpr_apogee_m, b.hpr_apogee_m),
                (
                    "hpr apogee as a height",
                    a.hpr_height_apogee_m,
                    b.hpr_height_apogee_m,
                ),
                (
                    "apogee error",
                    a.apogee_error_percent,
                    b.apogee_error_percent,
                ),
                (
                    "log time to apogee",
                    a.log_time_to_apogee_s,
                    b.log_time_to_apogee_s,
                ),
                (
                    "hpr time to apogee",
                    a.hpr_time_to_apogee_s,
                    b.hpr_time_to_apogee_s,
                ),
                ("trace RMS", a.trace_rms_m, b.trace_rms_m),
                ("trace RMS share", a.trace_rms_percent, b.trace_rms_percent),
                ("total impulse", a.total_impulse_ns, b.total_impulse_ns),
                (
                    "negative thrust zeroed",
                    a.negative_thrust_zeroed_ns,
                    b.negative_thrust_zeroed_ns,
                ),
                (
                    "apogee on the example's drag",
                    a.example_drag_apogee_m,
                    b.example_drag_apogee_m,
                ),
                (
                    "error on the example's drag",
                    a.example_drag_apogee_error_percent,
                    b.example_drag_apogee_error_percent,
                ),
                (
                    "trace RMS on the example's drag",
                    a.example_drag_trace_rms_m,
                    b.example_drag_trace_rms_m,
                ),
                (
                    "recorded thrust's impulse",
                    recorded(a, |flown| flown.impulse_ns),
                    recorded(b, |flown| flown.impulse_ns),
                ),
                (
                    "apogee on the recorded thrust",
                    recorded(a, |flown| flown.apogee_m),
                    recorded(b, |flown| flown.apogee_m),
                ),
                (
                    "error on the recorded thrust",
                    recorded(a, |flown| flown.apogee_error_percent),
                    recorded(b, |flown| flown.apogee_error_percent),
                ),
                (
                    "height column's gap from its pressure",
                    a.pressure_reading_max_m.unwrap_or(0.0),
                    b.pressure_reading_max_m.unwrap_or(0.0),
                ),
                (
                    "satellite apogee",
                    a.gnss_apogee_m.unwrap_or(0.0),
                    b.gnss_apogee_m.unwrap_or(0.0),
                ),
            ] {
                if !same(x, y) {
                    return Err(format!("{}: the {what} is {x} committed, {y} flown", a.id));
                }
            }
        }
        if (self.align_height_m, self.grid_s) != (other.align_height_m, other.grid_s)
            || self.generated_by != other.generated_by
        {
            return Err("the method differs".to_owned());
        }
        Ok(())
    }

    /// The report as a page.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let s = &self.summary;
        let _ = writeln!(out, "# hpr against the logs of real flights\n");
        let _ = writeln!(
            out,
            "Written by `{}` ([M2.3b][m2-3b], decision [ADR-082][adr-082]). How it is done, \
             and what it means, is on the [documentation site][site].\n\n\
             - **What is flown:** {} of the rockets RocketPy's documentation flies against their \
             teams' altitude logs. hpr flies each with its own aerodynamics, on the example's \
             own thrust file, from the example's rail and site, in the ERA5 weather the example \
             reads. The log is the reference.\n\
             - **Heights:** a barometric altimeter reads the standard atmosphere's altitude of \
             the pressure it measures, less the pad's. hpr's height is read the same way, from \
             the ERA5 pressure at its centre of mass, less the start's. Each log is read up to \
             its apogee, stopping before the recovery's pressure transients.\n\
             - **Trace RMS:** over the ascent, both clocks aligned where the trace first \
             reaches {} m, until the first of the two apogees.\n\
             - **Files:** the logs, thrust files and weather files are read from the pinned \
             RocketPy checkout and never committed; each one's SHA-256 is in the JSON.\n",
            self.generated_by,
            self.flights.len(),
            fmt_number(self.align_height_m, 0)
        );
        let _ = writeln!(
            out,
            "[m2-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-3b\n\
             [adr-082]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-082-real-flights-read-from-refs-compared-over-the-ascent-with-checked-explanations-2026-09-26\n\
             [site]: https://nrdptel.github.io/hpr-sim/accuracy.html#real-flights\n"
        );
        let _ = writeln!(out, "## Summary\n");
        let _ = writeln!(
            out,
            "- Flights: {}.\n- Mean absolute apogee error: {}% against a target of {}%: {}.\n\
             - Mean apogee error with its sign: {}%.\n- Mean absolute apogee error were every \
             log a height, not a barometric reading: {}%.\n- Mean absolute apogee error with \
             only the logs known to be barometric read so, the assumed ones as heights: {}%.\n\
             - Largest trace RMS: {}% of its log's apogee.\n- Outside the target: {}.\n- On \
             each example's own drag, the diagnostic flight: mean absolute apogee error {}%.\n",
            s.flights,
            fmt_number(s.mean_absolute_apogee_error_percent, 2),
            fmt_number(s.target_percent, 0),
            if s.within_target {
                "within target"
            } else {
                "outside target"
            },
            fmt_signed(s.mean_apogee_error_percent, 2),
            fmt_number(s.mean_absolute_height_apogee_error_percent, 2),
            fmt_number(s.mean_absolute_known_barometric_apogee_error_percent, 2),
            fmt_number(s.max_trace_rms_percent, 2),
            if s.outliers.is_empty() {
                "none".to_owned()
            } else {
                s.outliers.join(", ")
            },
            fmt_number(s.example_drag_mean_absolute_apogee_error_percent, 2)
        );
        let _ = writeln!(out, "## Flights\n");
        let _ = writeln!(
            out,
            "| flight | altimeter | log apogee (m) | hpr apogee (m) | apogee error | hpr apogee \
             as a height (m) | log time to apogee (s) | hpr time to apogee (s) | trace RMS (m) | \
             trace RMS (% of apogee) | rows |"
        );
        let _ = writeln!(
            out,
            "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|"
        );
        for row in &self.flights {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {}% | {} | {} | {} | {} | {}% | {} |",
                row.title,
                row.altimeter,
                fmt_number(row.log_apogee_m, 1),
                fmt_number(row.hpr_apogee_m, 1),
                fmt_signed(row.apogee_error_percent, 2),
                fmt_number(row.hpr_height_apogee_m, 1),
                fmt_number(row.log_time_to_apogee_s, 2),
                fmt_number(row.hpr_time_to_apogee_s, 2),
                fmt_number(row.trace_rms_m, 1),
                fmt_number(row.trace_rms_percent, 2),
                row.trace_rows
            );
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "## On each example's own drag\n");
        let _ = writeln!(
            out,
            "A diagnostic, not a prediction: the same flight with hpr's zero-lift drag replaced \
             by the drag the example's notebook specifies (a team's estimate: a table, a curve \
             from RASAero II or CFD, or a constant), on the example's radius. hpr's normal force \
             is its own in both. Where the error falls within the target, the miss is consistent \
             with hpr's drag; where it doesn't, it is elsewhere. The teams' drags are estimates \
             too, and one may have been tuned to its flight.\n"
        );
        let _ = writeln!(
            out,
            "| flight | example's drag | apogee (m) | apogee error | trace RMS (m) |"
        );
        let _ = writeln!(out, "|---|---|---:|---:|---:|");
        for row in &self.flights {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {}% | {} |",
                row.title,
                row.example_drag_source,
                fmt_number(row.example_drag_apogee_m, 1),
                fmt_signed(row.example_drag_apogee_error_percent, 2),
                fmt_number(row.example_drag_trace_rms_m, 1)
            );
        }
        let _ = writeln!(out);
        if self.flights.iter().any(|row| row.recorded_thrust.is_some()) {
            let _ = writeln!(out, "## On the thrust file as recorded\n");
            let _ = writeln!(
                out,
                "A second diagnostic, where an example reshapes its thrust file to a burn time and \
                 a total impulse: the same flight on the file as recorded (negative points read as \
                 zero), on hpr's own drag. Where the error falls within the target and the \
                 example's drag doesn't bring it there, the miss is consistent with the impulse \
                 the example sets.\n"
            );
            let _ = writeln!(
                out,
                "| flight | impulse as recorded (N s) | impulse reshaped (N s) | apogee (m) | apogee \
                 error |"
            );
            let _ = writeln!(out, "|---|---:|---:|---:|---:|");
            for row in &self.flights {
                if let Some(flown) = &row.recorded_thrust {
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {} | {}% |",
                        row.title,
                        fmt_number(flown.impulse_ns, 1),
                        fmt_number(row.total_impulse_ns, 1),
                        fmt_number(flown.apogee_m, 1),
                        fmt_signed(flown.apogee_error_percent, 2)
                    );
                }
            }
            let _ = writeln!(out);
        }
        if self
            .flights
            .iter()
            .any(|row| row.pressure_reading_max_m.is_some() || row.gnss_apogee_m.is_some())
        {
            let _ = writeln!(
                out,
                "## Against the logs' own pressure and satellite heights\n"
            );
            let _ = writeln!(
                out,
                "Where a log keeps the pressure its height was read from, the height column's \
                 largest gap from the standard atmosphere's reading of that pressure shows how the \
                 altimeter reads. Where the flight logged a satellite (GNSS) altitude, its apogee \
                 above the pad is a geometric height: the log's barometric apogee over it is what \
                 the day's air did to the reading, beside what hpr's conversion does to its own \
                 apogee.\n"
            );
            let _ = writeln!(
                out,
                "| flight | height against its pressure (largest gap, m) | satellite apogee (m) | \
                 log apogee over it | hpr's reading over its height |"
            );
            let _ = writeln!(out, "|---|---:|---:|---:|---:|");
            let or_dash = |value: Option<String>| value.unwrap_or_else(|| "—".to_owned());
            for row in self
                .flights
                .iter()
                .filter(|row| row.pressure_reading_max_m.is_some() || row.gnss_apogee_m.is_some())
            {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} | {} |",
                    row.title,
                    or_dash(row.pressure_reading_max_m.map(|gap| fmt_number(gap, 3))),
                    or_dash(row.gnss_apogee_m.map(|apogee| fmt_number(apogee, 1))),
                    or_dash(
                        row.gnss_apogee_m
                            .map(|apogee| fmt_number(row.log_apogee_m / apogee, 3))
                    ),
                    fmt_number(row.hpr_apogee_m / row.hpr_height_apogee_m, 3)
                );
            }
            let _ = writeln!(out);
        }
        let _ = writeln!(out, "## Each flight's inputs\n");
        for row in &self.flights {
            let _ = writeln!(
                out,
                "- **{}** (`{}`): weather at {}; inputs from RocketPy 1.13.0's {}; altimeter: \
                 {}; total impulse flown {} N s{}.{}{}",
                row.title,
                row.design,
                row.weather_time_utc,
                row.source,
                row.altimeter_evidence,
                fmt_number(row.total_impulse_ns, 1),
                if row.negative_thrust_zeroed_ns == 0.0 {
                    String::new()
                } else {
                    format!(
                        ", {} N s of it from reading the file's negative thrust as zero",
                        fmt_number(row.negative_thrust_zeroed_ns, 2)
                    )
                },
                if row.note.is_empty() {
                    String::new()
                } else {
                    format!(" Note: {}.", row.note)
                },
                if row.explanation.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Outside the target ({}): {}",
                        row.explanation_kind, row.explanation
                    )
                }
            );
        }
        out
    }
}

fn fmt_number(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}")
}

fn fmt_signed(value: f64, decimals: usize) -> String {
    format!("{value:+.decimals$}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENG: &str = "; a comment\nK1 54 579 6 1.4 2.2 AT\n 0.1 100 ; peak\n 0.5 50\n 1.0 0\n";

    /// In the standard atmosphere a barometric altimeter reads the geopotential height climbed.
    /// On a day 20 K warmer at every height, with the same sea-level pressure, each pressure's
    /// geopotential height in the troposphere is `(T₀ + ΔT) / T₀` times the standard's, so the
    /// altimeter reads exactly `T₀ / (T₀ + ΔT)` = 288.15 / 308.15 of the height climbed.
    #[test]
    fn a_barometer_reads_the_standard_atmospheres_altitude() {
        let climbed = hpr_atmos::ussa76::geopotential_from_geometric_m(4400.0).unwrap()
            - hpr_atmos::ussa76::geopotential_from_geometric_m(1401.0).unwrap();
        let standard = Ussa76::standard();
        let barometer = Barometer::on_pad(&standard, 1400.0, 1.0).unwrap();
        let reading = barometer.reading_m(3000.0).unwrap();
        assert!((reading - climbed).abs() < 1e-6, "{reading} vs {climbed}");
        assert_eq!(barometer.reading_m(1.0).unwrap(), 0.0);

        let warm = Ussa76::with_offset(20.0, 101_325.0).unwrap();
        let reading = Barometer::on_pad(&warm, 1400.0, 1.0)
            .unwrap()
            .reading_m(3000.0)
            .unwrap();
        let expected = climbed * 288.15 / 308.15;
        assert!(
            (reading - expected).abs() < 1e-9 * climbed,
            "{reading} vs {expected}"
        );
    }

    /// The pressure gap against the standard's troposphere in closed form,
    /// `p = 101325 Pa (1 − L H / T₀)^(g₀′ M₀ / (R* L))`: a height column 5 m off its pressure's
    /// reading gives a 5 m gap, and rows past the cut don't count, readable or not.
    #[test]
    fn a_height_column_is_held_to_its_pressure() {
        let pressure_hpa = |h: f64| {
            1013.25 * (1.0 - 0.0065 * h / 288.15).powf(9.806_65 * 28.9644 / (8314.32 * 0.0065))
        };
        let text = format!(
            "t,h,p\n0,0,{}\n1,1000,{}\n2,2005,{}\n3,9999,x\n4,9999,{}\n",
            pressure_hpa(0.0),
            pressure_hpa(1000.0),
            pressure_hpa(2000.0),
            pressure_hpa(0.0)
        );
        let log = Log {
            file: "x.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 1.0,
            until_s: Some(3.5),
            altimeter: Altimeter::Barometric(""),
            pressure: Some((2, 100.0)),
            gnss: None,
        };
        let gap = pressure_reading_gap_m(&log, 2, 100.0, &text).unwrap();
        assert!((gap - 5.0).abs() < 1e-6, "{gap}");
        assert!(pressure_reading_gap_m(&log, 2, 100.0, "t,h,p\n").is_err());
    }

    /// A satellite apogee is above the first row, in the file's units; a first row far from the
    /// site, or a log that never climbs, is refused.
    #[test]
    fn a_satellite_apogee_is_above_its_first_row() {
        let gnss = Gnss {
            file: "g.csv",
            header_lines: 1,
            time_column: 0,
            altitude_column: 1,
            metres_per_unit: 0.3048,
        };
        let text = "t,alt\n0,4600\n1,5100\n2,5600\n3,5400\n";
        let apogee = satellite_apogee_m(&gnss, text, 1400.0).unwrap();
        assert!((apogee - 304.8).abs() < 1e-9, "{apogee}");
        assert!(satellite_apogee_m(&gnss, text, 0.0).is_err());
        assert!(satellite_apogee_m(&gnss, "t,alt\n0,4600\n1,4600\n", 1400.0).is_err());
        assert!(satellite_apogee_m(&gnss, "t,alt\n", 1400.0).is_err());
    }

    #[test]
    fn an_eng_file_reads_as_rocketpy_reads_it() {
        let (points, removed) = parse_thrust("t", "a.eng", ENG, None, None).unwrap();
        assert_eq!(
            points,
            vec![(0.0, 0.0), (0.1, 100.0), (0.5, 50.0), (1.0, 0.0)]
        );
        assert_eq!(removed, 0.0);
        // A first data line of `0 0` is the description, as in RocketPy's `import_eng`.
        let headless = "0 0\n0.003 281.69\n1.0 0\n";
        let (points, _) = parse_thrust("t", "b.eng", headless, None, None).unwrap();
        assert_eq!(points, vec![(0.0, 0.0), (0.003, 281.69), (1.0, 0.0)]);
    }

    #[test]
    fn a_burn_time_clips_with_the_curves_value_at_the_end() {
        let (points, _) = parse_thrust("t", "a.eng", ENG, Some(0.3), None).unwrap();
        assert_eq!(points, vec![(0.0, 0.0), (0.1, 100.0), (0.3, 75.0)]);
        // Past the last time, the last time.
        let (points, _) = parse_thrust("t", "a.eng", ENG, Some(2.0), None).unwrap();
        assert_eq!(points.last(), Some(&(1.0, 0.0)));
    }

    #[test]
    fn a_reshape_scales_time_then_thrust_to_the_impulse() {
        let csv = "0,10\n1,10\n2,-2\n";
        let (points, removed) = parse_thrust("t", "a.csv", csv, None, Some((4.0, 90.0))).unwrap();
        // Times double; the trapezoid of (0,10),(2,10),(4,-2) is 20 + 8 = 28, so thrusts scale
        // by 90/28, and the negative end is read as zero, which adds its negative share back.
        let k = 90.0 / 28.0;
        assert_eq!(points[0], (0.0, 10.0 * k));
        assert_eq!(points[1], (2.0, 10.0 * k));
        assert_eq!(points[2], (4.0, 0.0));
        assert!((removed - (0.5 * 2.0 * 10.0 * k - 0.5 * 2.0 * (10.0 - 2.0) * k)).abs() < 1e-12);
    }

    #[test]
    fn a_log_skips_rows_that_do_not_read_and_stops_at_its_end() {
        let log = Log {
            file: "x.csv",
            header_lines: 1,
            time_column: 0,
            height_column: 1,
            metres_per_unit: 0.3048,
            until_s: Some(2.0),
            altimeter: Altimeter::Barometric(""),
            pressure: None,
            gnss: None,
        };
        let text = "t,h\n0,0\n1, 100\n1.5,\n2,200\n3,9999\n";
        let rows = parse_log("t", &log, text).unwrap();
        assert_eq!(rows, vec![(0.0, 0.0), (1.0, 30.48), (2.0, 60.96)]);
    }

    #[test]
    fn traces_align_at_the_height_and_compare_to_the_first_apogee() {
        // The log is hpr's trace three seconds later: aligned at 30 m, the RMS is zero whatever
        // the clocks, and it runs to the log's apogee, hpr's being later.
        let rise = |t: f64| 100.0 * t * t;
        let hpr: Vec<(f64, f64)> = (0..=1200)
            .map(|i| 0.01 * f64::from(i))
            .map(|t| (t, rise(t)))
            .collect();
        let log: Vec<(f64, f64)> = (0..=1000)
            .map(|i| 0.01 * f64::from(i))
            .map(|t| (t + 3.0, rise(t)))
            .collect();
        let got = compare_traces(&log, &hpr, 12.0).unwrap();
        assert!(got.rms_m < 1e-9, "{got:?}");
        let align = 30.0_f64.sqrt() / 10.0;
        assert!(
            (got.log_time_to_apogee_s - (10.0 - align)).abs() < 1e-3,
            "{got:?}"
        );
        assert!(
            (got.hpr_time_to_apogee_s - (12.0 - align)).abs() < 1e-3,
            "{got:?}"
        );
        // Every log row from the crossing (between 0.54 and 0.55 s) to its apogee.
        assert_eq!(got.rows, 1000 - 55 + 1);
        // Heights 10 m higher after the crossing give an RMS of 10 m.
        let higher: Vec<(f64, f64)> = log
            .iter()
            .map(|&(t, h)| (t, if h > 40.0 { h + 10.0 } else { h }))
            .collect();
        let got = compare_traces(&higher, &hpr, 12.0).unwrap();
        assert!(got.rms_m > 9.9 && got.rms_m <= 10.0, "{got:?}");
    }

    #[test]
    fn the_flights_take_their_thrust_from_the_design_fixtures_record() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../validation/fixtures/design/rocketpy-rocket-mass.json"
        ))
        .unwrap();
        for flight in &FLIGHTS {
            let name = flight.design.trim_start_matches("rocketpy-");
            let case = fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["name"] == name)
                .unwrap_or_else(|| panic!("{} has no fixture case", flight.id));
            assert!(case["original_thrust_source"].is_string(), "{}", flight.id);
        }
    }
}
