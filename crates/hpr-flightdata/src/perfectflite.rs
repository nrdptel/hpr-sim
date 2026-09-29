//! PerfectFlite's `.pf2` flight logs: the Pnut, the StratoLogger and the StratoLoggerCF.
//!
//! A `.pf2` is text: the logger's name on the first line, a preamble of `Key: value` lines, a
//! `Data:` line naming the columns, then one row per sample, comma-separated, at about 20 Hz:
//!
//! ```text
//! PerfectFlite Pnut
//! Firmware: 1.0
//! Apogee: 1280' AGL
//! Ground Elevation: 600' MSL
//! NumSamps: 3
//! Flight Number: 2
//! Data: (Time, Altitude, Velocity, Temperature (F), Voltage)
//! 0.00, 0, 0, 70.00, 4.20
//! 0.05, 0, 0
//! 0.10, 1, 8, 70.00, 4.20
//! ```
//!
//! Times are seconds, altitudes feet above the logger's reading on the pad, speeds feet per
//! second, temperatures degrees Fahrenheit, voltages volts. A row may leave its last columns out
//! (the temperature and voltage are logged less often); a column left out is a gap, `NaN`. The
//! velocity is the logger's own, worked out from its barometric altitude: a PerfectFlite has no
//! accelerometer.
//!
//! **Where this comes from.** PerfectFlite publishes no specification of the format. This reader
//! follows the one in Debrief (`lib/parsers/perfectflite.ts`, MIT, the project owner's own; see
//! `THIRD-PARTY-NOTICES.md`), which was written from exported files and cites no document; the
//! units above are its reading, borne out by the Pnut fixture Debrief ships, whose preamble states
//! its apogee with a foot mark. The one departure: Debrief assumes the column order, and this reader
//! takes it from the `Data:` line when there is one. A stated apogee or elevation in anything but
//! feet is refused rather than guessed at.

use crate::error::LogError;
use crate::log::{FlightLog, LogFormat, Stated};

/// The format's name in errors.
const FORMAT: &str = ".pf2";

/// Metres in a foot, exactly (the international foot).
pub const FOOT_M: f64 = 0.3048;

/// The columns a `.pf2` has when it has no `Data:` line: the order Debrief reads and the Pnut
/// fixture's `Data:` line states.
const DEFAULT_COLUMNS: [Column; 5] = [
    Column::Time,
    Column::Altitude,
    Column::Velocity,
    Column::Temperature(TemperatureUnit::Fahrenheit),
    Column::Voltage,
];

/// A column of the data rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Column {
    Time,
    Altitude,
    Velocity,
    Temperature(TemperatureUnit),
    Voltage,
    /// A column the reader doesn't know, dropped with a note.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TemperatureUnit {
    Fahrenheit,
    Celsius,
}

impl TemperatureUnit {
    fn kelvin(self, value: f64) -> f64 {
        match self {
            Self::Fahrenheit => (value - 32.0) * 5.0 / 9.0 + 273.15,
            Self::Celsius => value + 273.15,
        }
    }
}

/// Reads a `.pf2` flight log from its text.
///
/// # Errors
///
/// [`LogError::NotThisFormat`] if the first line doesn't name PerfectFlite;
/// [`LogError::Syntax`] for a row that isn't numbers, a time that doesn't increase, or a `Data:`
/// line without a time or an altitude column; [`LogError::Unit`] for a stated height that isn't
/// in feet; [`LogError::NoData`] if there are no rows.
pub fn read(text: &str) -> Result<FlightLog, LogError> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()));
    let logger = loop {
        match lines.next() {
            Some((_, "")) => {}
            Some((_, line)) if line.to_ascii_lowercase().contains("perfectflite") => {
                break line.to_owned();
            }
            Some((_, line)) => {
                return Err(LogError::NotThisFormat {
                    format: FORMAT,
                    message: format!("its first line, {line:?}, doesn't name PerfectFlite"),
                });
            }
            None => {
                return Err(LogError::NotThisFormat {
                    format: FORMAT,
                    message: "the file is empty".to_owned(),
                });
            }
        }
    };

    let mut log = FlightLog {
        format: LogFormat::PerfectFlitePf2,
        logger,
        serial_number: None,
        firmware: None,
        flight_number: None,
        stated: Stated::default(),
        time_s: Vec::new(),
        altitude_m: Vec::new(),
        vertical_speed_m_s: None,
        temperature_k: None,
        battery_v: None,
        notes: Vec::new(),
    };
    let mut columns: Option<Vec<Column>> = None;
    let mut rows: Vec<(usize, Vec<f64>)> = Vec::new();

    for (number, line) in lines {
        if line.is_empty() {
            continue;
        }
        if is_data_row(line) {
            rows.push((number, data_row(line, number)?));
            continue;
        }
        if !rows.is_empty() {
            return Err(syntax(
                number,
                format!("{line:?} after the data rows began isn't a row of numbers"),
            ));
        }
        let Some((key, value)) = line.split_once(':') else {
            log.notes.push(format!(
                "line {number}, {line:?}, isn't a `Key: value` line and was skipped"
            ));
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "data" => columns = Some(data_columns(value, number, &mut log.notes)?),
            "apogee" => log.stated.apogee_m = stated_feet(value, number, "apogee", &mut log.notes)?,
            "ground elevation" => {
                log.stated.ground_elevation_msl_m =
                    stated_feet(value, number, "ground elevation", &mut log.notes)?;
            }
            "numsamps" => log.stated.samples = value.parse().ok(),
            "flight number" => log.flight_number = value.parse().ok(),
            "serial number" if !value.is_empty() => log.serial_number = Some(value.to_owned()),
            "firmware" if !value.is_empty() => log.firmware = Some(value.to_owned()),
            // The software version, comments and anything else describe the file, not the flight.
            _ => {}
        }
    }

    if rows.is_empty() {
        return Err(LogError::NoData { format: FORMAT });
    }
    let columns = columns.unwrap_or_else(|| {
        log.notes.push(
            "the file has no `Data:` line; its columns were read as time, altitude, velocity, \
             temperature (°F) and voltage"
                .to_owned(),
        );
        DEFAULT_COLUMNS.to_vec()
    });
    fill(&mut log, &columns, &rows)?;
    if let Some(stated) = log.stated.samples
        && stated != log.time_s.len()
    {
        log.notes.push(format!(
            "the file states {stated} samples and holds {}",
            log.time_s.len()
        ));
    }
    Ok(log)
}

/// Whether a line is a data row: it starts with a number and has a comma, as Debrief tells them.
fn is_data_row(line: &str) -> bool {
    let first = line.trim_start_matches(['-', '+']);
    first.starts_with(|c: char| c.is_ascii_digit() || c == '.') && line.contains(',')
}

fn syntax(line: usize, message: String) -> LogError {
    LogError::Syntax {
        format: FORMAT,
        line,
        message,
    }
}

/// A data row's cells as numbers.
fn data_row(line: &str, number: usize) -> Result<Vec<f64>, LogError> {
    line.split(',')
        .map(|cell| {
            let cell = cell.trim();
            cell.parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| syntax(number, format!("{cell:?} isn't a finite number")))
        })
        .collect()
}

/// The columns a `Data:` line names, such as `(Time, Altitude, Velocity, Temperature (F), Voltage)`.
fn data_columns(
    value: &str,
    number: usize,
    notes: &mut Vec<String>,
) -> Result<Vec<Column>, LogError> {
    let inner = value.strip_prefix('(').unwrap_or(value);
    let inner = inner.strip_suffix(')').unwrap_or(inner);
    let columns: Vec<Column> = inner
        .split(',')
        .map(|name| {
            let name = name.trim().to_ascii_lowercase();
            let column = if name.starts_with("time") {
                Column::Time
            } else if name.starts_with("altitude") {
                Column::Altitude
            } else if name.starts_with("velocity") {
                Column::Velocity
            } else if name.starts_with("temperature") && name.contains("(f)") {
                Column::Temperature(TemperatureUnit::Fahrenheit)
            } else if name.starts_with("temperature") && name.contains("(c)") {
                Column::Temperature(TemperatureUnit::Celsius)
            } else if name.starts_with("voltage") {
                Column::Voltage
            } else {
                Column::Unknown
            };
            if column == Column::Unknown {
                notes.push(format!(
                    "the column {name:?} isn't one this reader knows, and was left out"
                ));
            }
            column
        })
        .collect();
    for required in [Column::Time, Column::Altitude] {
        if columns.iter().filter(|column| **column == required).count() != 1 {
            return Err(syntax(
                number,
                format!("the `Data:` line must name one {required:?} column: {value:?}"),
            ));
        }
    }
    Ok(columns)
}

/// A stated height, such as `1009' AGL`, in metres; `None`, with a note, for one that isn't a
/// number, such as `PWRLOSS`.
fn stated_feet(
    value: &str,
    number: usize,
    what: &str,
    notes: &mut Vec<String>,
) -> Result<Option<f64>, LogError> {
    let end = value
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+')))
        .unwrap_or(value.len());
    let (digits, rest) = value.split_at(end);
    let Some(feet) = digits.parse::<f64>().ok().filter(|feet| feet.is_finite()) else {
        notes.push(format!(
            "the file states its {what} as {value:?}, which isn't a height"
        ));
        return Ok(None);
    };
    if !rest.trim_start().starts_with('\'') {
        return Err(LogError::Unit {
            format: FORMAT,
            line: number,
            message: format!(
                "the {what} {value:?} isn't marked as feet ('), the only unit this reader knows"
            ),
        });
    }
    Ok(Some(feet * FOOT_M))
}

/// Puts the rows' values into the log's channels, in SI.
fn fill(
    log: &mut FlightLog,
    columns: &[Column],
    rows: &[(usize, Vec<f64>)],
) -> Result<(), LogError> {
    let has = |wanted: fn(&Column) -> bool| columns.iter().any(wanted);
    let mut speed = has(|c| *c == Column::Velocity).then(Vec::new);
    let mut temperature = has(|c| matches!(c, Column::Temperature(_))).then(Vec::new);
    let mut battery = has(|c| *c == Column::Voltage).then(Vec::new);
    for (number, cells) in rows {
        if cells.len() > columns.len() {
            return Err(syntax(
                *number,
                format!(
                    "the row has {} values and the columns are {}",
                    cells.len(),
                    columns.len()
                ),
            ));
        }
        let mut time = None;
        let mut altitude = None;
        let (mut v, mut t, mut u) = (f64::NAN, f64::NAN, f64::NAN);
        for (column, value) in columns.iter().zip(cells) {
            match column {
                Column::Time => time = Some(*value),
                Column::Altitude => altitude = Some(*value * FOOT_M),
                Column::Velocity => v = *value * FOOT_M,
                Column::Temperature(unit) => t = unit.kelvin(*value),
                Column::Voltage => u = *value,
                Column::Unknown => {}
            }
        }
        let (Some(time), Some(altitude)) = (time, altitude) else {
            return Err(syntax(
                *number,
                "the row leaves out its time or its altitude".to_owned(),
            ));
        };
        if let Some(&last) = log.time_s.last()
            && time <= last
        {
            return Err(syntax(
                *number,
                format!("the time {time} s doesn't come after the row before's {last} s"),
            ));
        }
        log.time_s.push(time);
        log.altitude_m.push(altitude);
        for (channel, value) in [(&mut speed, v), (&mut temperature, t), (&mut battery, u)] {
            if let Some(channel) = channel {
                channel.push(value);
            }
        }
    }
    log.vertical_speed_m_s = speed;
    log.temperature_k = temperature;
    log.battery_v = battery;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEAD: &str = "PerfectFlite Pnut\r\nFirmware: 1.0\r\nSerial Number: 17\r\n\
                        Apogee: 1280' AGL\r\nGround Elevation: 600' MSL\r\nNumSamps: 3\r\n\
                        Flight Number: 2\r\nComments: \r\n\r\n\
                        Data: (Time, Altitude, Velocity, Temperature (F), Voltage)\r\n";

    /// Every value in SI, a short row's missing cells gaps, and the preamble's statements read.
    #[test]
    fn reads_the_preamble_and_rows_in_si() {
        let text =
            format!("{HEAD}0.00, 0, 0, 32.00, 4.20\r\n0.05, -1, 10\r\n0.10, 10, 20, 212, 4.1\r\n");
        let log = read(&text).unwrap();
        assert_eq!(log.format, LogFormat::PerfectFlitePf2);
        assert_eq!(log.logger, "PerfectFlite Pnut");
        assert_eq!(log.serial_number.as_deref(), Some("17"));
        assert_eq!(log.firmware.as_deref(), Some("1.0"));
        assert_eq!(log.flight_number, Some(2));
        assert_eq!(log.stated.apogee_m, Some(1280.0 * 0.3048));
        assert_eq!(log.stated.ground_elevation_msl_m, Some(600.0 * 0.3048));
        assert_eq!(log.stated.samples, Some(3));
        assert_eq!(log.time_s, [0.0, 0.05, 0.10]);
        assert_eq!(log.altitude_m, [0.0, -0.3048, 3.048]);
        assert_eq!(
            log.vertical_speed_m_s.as_deref(),
            Some(&[0.0, 3.048, 6.096][..])
        );
        let temperature = log.temperature_k.unwrap();
        assert_eq!(temperature[0], 273.15);
        assert!(temperature[1].is_nan());
        assert!((temperature[2] - 373.15).abs() < 1e-12);
        let battery = log.battery_v.unwrap();
        assert_eq!((battery[0], battery[2]), (4.2, 4.1));
        assert!(battery[1].is_nan());
        assert!(log.notes.is_empty(), "{:?}", log.notes);
    }

    /// The columns come from the `Data:` line, in its order; a column it doesn't know is noted
    /// and left out; without the line, Debrief's order is assumed and noted.
    #[test]
    fn columns_follow_the_data_line() {
        let text = "PerfectFlite StratoLogger\nData: (Altitude, Time, Pressure)\n100, 0.5, 9\n";
        let log = read(text).unwrap();
        assert_eq!((log.time_s[0], log.altitude_m[0]), (0.5, 30.48));
        assert!(log.vertical_speed_m_s.is_none() && log.temperature_k.is_none());
        assert!(log.notes[0].contains("\"pressure\""), "{:?}", log.notes);

        let log = read("PerfectFlite Pnut\n0.0, 5, 1, 50, 4\n").unwrap();
        assert_eq!(log.altitude_m, [5.0 * 0.3048]);
        assert!(log.notes[0].contains("no `Data:` line"), "{:?}", log.notes);
    }

    /// `PWRLOSS` is no height: noted, not refused. A height in another unit is refused.
    #[test]
    fn stated_heights_are_feet_or_nothing() {
        let log = read("PerfectFlite Pnut\nApogee: PWRLOSS\n0, 0, 0\n").unwrap();
        assert_eq!(log.stated.apogee_m, None);
        assert!(log.notes[0].contains("PWRLOSS"), "{:?}", log.notes);
        let error = read("PerfectFlite Pnut\nApogee: 390 m AGL\n0, 0, 0\n").unwrap_err();
        assert!(
            matches!(&error, LogError::Unit { line: 2, message, .. } if message.contains("\"390 m AGL\"")),
            "{error}"
        );
    }

    /// Each refusal names its line and what is wrong.
    #[test]
    fn malformed_files_are_refused_where_they_go_wrong() {
        let cases = [
            ("", "the file is empty"),
            ("Time, Altitude\n0, 0\n", "doesn't name PerfectFlite"),
            ("PerfectFlite Pnut\nApogee: 5'\n", "no data rows"),
            (
                "PerfectFlite Pnut\n0, 0, 0\n0, 1, 0\n",
                "line 3: the time 0 s doesn't come after",
            ),
            (
                "PerfectFlite Pnut\n0, 0, x\n",
                "line 2: \"x\" isn't a finite number",
            ),
            (
                "PerfectFlite Pnut\n0, 0, 0\nEnd\n",
                "line 3: \"End\" after the data rows",
            ),
            (
                "PerfectFlite Pnut\n0, 0, 0, 1, 2, 3\n",
                "line 2: the row has 6 values",
            ),
            ("PerfectFlite Pnut\n0\n", "no data rows"),
            (
                "PerfectFlite Pnut\nData: (Time, Velocity)\n0, 0\n",
                "line 2: the `Data:` line",
            ),
            (
                "PerfectFlite Pnut\nData: (Time, Altitude)\n0.5\n",
                "no data rows",
            ),
            (
                "PerfectFlite Pnut\nData: (Time, Altitude)\n0.5,\n",
                "line 3: \"\" isn't",
            ),
            (
                "PerfectFlite Pnut\n0, inf, 0\n",
                "line 2: \"inf\" isn't a finite number",
            ),
        ];
        for (text, expected) in cases {
            let error = read(text).unwrap_err().to_string();
            assert!(error.contains(expected), "{text:?}: {error}");
        }
        // A short row that stops before the altitude's column leaves it out.
        let error =
            read("PerfectFlite Pnut\nData: (Time, Velocity, Altitude)\n0, 5\n").unwrap_err();
        assert!(
            error.to_string().contains("line 3: the row leaves out"),
            "{error}"
        );
    }

    /// A stated sample count the rows don't match is noted.
    #[test]
    fn a_sample_count_that_differs_is_noted() {
        let log = read("PerfectFlite Pnut\nNumSamps: 3\n0, 0, 0\n0.05, 0, 0\n").unwrap();
        assert_eq!(
            log.notes.last().unwrap(),
            "the file states 3 samples and holds 2"
        );
    }

    proptest::proptest! {
        /// Any text is read or refused, never a panic; what is read has increasing finite times.
        #[test]
        fn any_text_is_read_or_refused(text in "(PerfectFlite Pnut\n)?([-0-9., A-Za-z:'()]{0,20}\n){0,12}") {
            if let Ok(log) = read(&text) {
                proptest::prop_assert!(log.time_s.windows(2).all(|pair| pair[1] > pair[0]));
                proptest::prop_assert_eq!(log.time_s.len(), log.altitude_m.len());
            }
        }
    }
}
