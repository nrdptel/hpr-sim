//! RASP `.eng` motor files: reading and writing, built from the public spec
//! (`docs/format/eng.md`).
//!
//! A file holds one or more entries. Each entry is optional `;` comments, a seven-field header
//! (name, diameter in mm, length in mm, delays, propellant mass in kg, total mass in kg,
//! manufacturer) and `time thrust` points, and entries are separated by comment lines.
//!
//! The file model keeps the file's units (mm and kg) and its points exactly as listed, with no
//! implicit origin, so a read-write-read cycle reproduces every value bit for bit
//! ([`EngEntry::thrust_curve`] adds the origin). The reader is lenient and reports what it
//! accepted as [`ParseWarning`]s; the writer refuses anything the reader would read differently.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::curve::ThrustCurve;
use crate::delay::DelayList;
use crate::error::MotorError;
use crate::text::{ParseWarning, Parsed, WarningKind, check_writable, finite};

const FORMAT: &str = ".eng";

/// A parsed `.eng` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngFile {
    /// The entries, in file order.
    pub entries: Vec<EngEntry>,
    /// Comments after the last entry's data, without the leading `;`.
    pub trailing_comments: Vec<String>,
}

/// One motor in a `.eng` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngEntry {
    /// Comments before the header, without the leading `;`.
    pub comments: Vec<String>,
    /// The motor's name, such as `F32` or `1266-J760-WT-19A`.
    pub name: String,
    /// Casing diameter, mm.
    pub diameter_mm: f64,
    /// Casing length, mm.
    pub length_mm: f64,
    /// The delay string as written, such as `6-10-14` or `P`; see [`EngEntry::delays`].
    pub delays: String,
    /// Propellant mass, kg.
    pub propellant_mass_kg: f64,
    /// Loaded motor mass, kg.
    pub total_mass_kg: f64,
    /// Manufacturer abbreviation, such as `AT` or `Cesaroni_Technology`.
    pub manufacturer: String,
    /// The points as listed: `(time s, thrust N)`.
    pub points: Vec<(f64, f64)>,
}

impl EngEntry {
    /// The delay settings read from [`EngEntry::delays`].
    pub fn delays(&self) -> DelayList {
        DelayList::parse(&self.delays)
    }

    /// The entry's thrust curve, starting from the implicit `(0, 0)` when the first point is after
    /// ignition.
    ///
    /// # Errors
    ///
    /// As [`ThrustCurve::new`]: negative thrust, decreasing time, or no thrust.
    pub fn thrust_curve(&self) -> Result<ThrustCurve, MotorError> {
        let (times, thrusts) = self.points.iter().copied().unzip();
        ThrustCurve::new(times, thrusts)
    }
}

/// Where the reader is within an entry.
enum State {
    /// Before a header: collecting comments.
    Header,
    /// After a header: reading points.
    Points(EngEntry, usize),
    /// After an error in an entry: skipping to the next comment line.
    Skip,
}

/// Reads a `.eng` file.
///
/// An entry with an error is skipped, with the error as a warning, when other entries in the file
/// read. The reader resumes at the next comment line.
///
/// # Errors
///
/// When no entry reads, the first entry's error: [`MotorError::Syntax`], with the line number,
/// for a header without seven fields, a bad number, a line that is neither a point nor a header,
/// an entry with no points, negative or decreasing time, a non-finite value, a non-positive
/// diameter or length, or a negative mass. A file with no entries at all is also an error.
pub fn parse(text: &str) -> Result<Parsed<EngFile>, MotorError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();
    let mut comments = Vec::new();
    let mut state = State::Header;

    for (index, raw) in text.split('\n').enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix(';') {
            if let State::Points(entry, header_line) = std::mem::replace(&mut state, State::Header)
            {
                match finish(entry, header_line, &mut warnings) {
                    Ok(entry) => entries.push(entry),
                    Err(error) => errors.push(error),
                }
            }
            if !comment.is_empty() {
                comments.push(comment.to_owned());
            }
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        let next = match std::mem::replace(&mut state, State::Skip) {
            State::Skip => Ok(State::Skip),
            State::Header => header(&fields, std::mem::take(&mut comments), line, &mut warnings)
                .map(|entry| State::Points(entry, line)),
            State::Points(mut entry, header_line) => match fields.len() {
                2 => {
                    add_point(&mut entry, &fields, line).map(|()| State::Points(entry, header_line))
                }
                // A header starts with the motor's name; a line of numbers is a corrupt point.
                n if n >= 7 && fields[0].parse::<f64>().is_err() => {
                    match finish(entry, header_line, &mut warnings) {
                        Ok(entry) => entries.push(entry),
                        Err(error) => errors.push(error),
                    }
                    warnings.push(ParseWarning::new(
                        line,
                        WarningKind::Unusual,
                        "a new header with no comment line before it",
                    ));
                    header(&fields, std::mem::take(&mut comments), line, &mut warnings)
                        .map(|entry| State::Points(entry, line))
                }
                _ => Err(syntax(
                    line,
                    format!("expected `time thrust` or a header, found {trimmed:?}"),
                )),
            },
        };
        state = next.unwrap_or_else(|error| {
            errors.push(error);
            comments.clear();
            State::Skip
        });
    }
    if let State::Points(entry, header_line) = state {
        match finish(entry, header_line, &mut warnings) {
            Ok(entry) => entries.push(entry),
            Err(error) => errors.push(error),
        }
    }
    if entries.is_empty() {
        return Err(errors
            .into_iter()
            .next()
            .unwrap_or_else(|| syntax(1, "no motor entries".into())));
    }
    for error in errors {
        let line = match &error {
            MotorError::Syntax { line, .. } => *line,
            _ => 0,
        };
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Skipped,
            format!("entry skipped: {error}"),
        ));
    }
    warnings.sort_by_key(|warning| warning.line);
    Ok(Parsed {
        value: EngFile {
            entries,
            trailing_comments: comments,
        },
        warnings,
    })
}

fn add_point(entry: &mut EngEntry, fields: &[&str], line: usize) -> Result<(), MotorError> {
    let time = number(fields[0], "time", line)?;
    let thrust = number(fields[1], "thrust", line)?;
    point(entry, time, thrust, line)
}

/// Writes a `.eng` file: each entry's comments, header and points, then a `;` separator, and the
/// trailing comments last.
///
/// # Errors
///
/// - [`MotorError::Inconsistent`] for a name that is empty, holds whitespace or starts with `;`;
///   delays that are empty or hold whitespace; a manufacturer that is empty or isn't its words
///   joined by single spaces; a comment that is empty, holds a line break or ends in whitespace;
///   an entry with no points; or a file with no entries. The reader would not read these back
///   unchanged.
/// - [`MotorError::Domain`] for the values the reader rejects: non-finite numbers, a
///   non-positive diameter or length, negative masses or times, and decreasing times.
pub fn write(file: &EngFile) -> Result<String, MotorError> {
    if file.entries.is_empty() {
        return Err(MotorError::Inconsistent(
            "a .eng file needs an entry".into(),
        ));
    }
    let mut out = String::new();
    for entry in &file.entries {
        for comment in &entry.comments {
            write_comment(&mut out, comment)?;
        }
        // The name starts the line, so `;` there would make it a comment. The reader joins the
        // manufacturer's words with single spaces.
        let one_token = |field: &str| !field.is_empty() && !field.contains(char::is_whitespace);
        let words = entry
            .manufacturer
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if !one_token(&entry.name) || entry.name.starts_with(';') {
            return Err(MotorError::Inconsistent(format!(
                "the .eng name {:?} must be one token that doesn't start with `;`",
                entry.name
            )));
        }
        if !one_token(&entry.delays) {
            return Err(MotorError::Inconsistent(format!(
                "the .eng delays {:?} must be one token",
                entry.delays
            )));
        }
        if entry.manufacturer.is_empty() || words != entry.manufacturer {
            return Err(MotorError::Inconsistent(format!(
                "the .eng manufacturer {:?} must be words separated by single spaces",
                entry.manufacturer
            )));
        }
        check_envelope(entry)?;
        if entry.points.is_empty() {
            return Err(MotorError::Inconsistent(format!(
                "the .eng entry {:?} has no points",
                entry.name
            )));
        }
        let mut previous = 0.0;
        for &(time, thrust) in &entry.points {
            check_writable(time, "time (s)", true)?;
            check_writable(thrust, "thrust (N)", false)?;
            if time < previous {
                return Err(MotorError::Domain {
                    what: "time (s), which decreases",
                    value: time,
                });
            }
            previous = time;
        }
        // `{}` prints the shortest digits that read back to the same f64, without an exponent.
        let _ = writeln!(
            out,
            "{} {} {} {} {} {} {}",
            entry.name,
            entry.diameter_mm,
            entry.length_mm,
            entry.delays,
            entry.propellant_mass_kg,
            entry.total_mass_kg,
            entry.manufacturer
        );
        for (time, thrust) in &entry.points {
            let _ = writeln!(out, "   {time} {thrust}");
        }
        out.push_str(";\n");
    }
    for comment in &file.trailing_comments {
        write_comment(&mut out, comment)?;
    }
    Ok(out)
}

fn write_comment(out: &mut String, comment: &str) -> Result<(), MotorError> {
    if comment.trim().is_empty()
        || comment.contains(['\n', '\r'])
        || comment.ends_with(char::is_whitespace)
    {
        return Err(MotorError::Inconsistent(format!(
            "the .eng comment {comment:?} must be non-empty, one line, without trailing whitespace"
        )));
    }
    out.push(';');
    out.push_str(comment);
    out.push('\n');
    Ok(())
}

fn header(
    fields: &[&str],
    comments: Vec<String>,
    line: usize,
    warnings: &mut Vec<ParseWarning>,
) -> Result<EngEntry, MotorError> {
    if fields.len() < 7 {
        return Err(syntax(
            line,
            format!(
                "a header needs 7 fields (name, diameter, length, delays, propellant mass, total \
                 mass, manufacturer); found {}",
                fields.len()
            ),
        ));
    }
    if fields.len() > 7 {
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Unusual,
            format!(
                "the header has {} fields; fields 7 onward are read as the manufacturer",
                fields.len()
            ),
        ));
    }
    let entry = EngEntry {
        comments,
        name: fields[0].to_owned(),
        diameter_mm: number(fields[1], "diameter", line)?,
        length_mm: number(fields[2], "length", line)?,
        delays: fields[3].to_owned(),
        propellant_mass_kg: number(fields[4], "propellant mass", line)?,
        total_mass_kg: number(fields[5], "total mass", line)?,
        manufacturer: fields[6..].join(" "),
        points: Vec::new(),
    };
    check_envelope(&entry).map_err(|error| syntax(line, error.to_string()))?;
    Ok(entry)
}

/// The header's dimensions must be positive and its masses non-negative.
fn check_envelope(entry: &EngEntry) -> Result<(), MotorError> {
    for (value, what) in [
        (entry.diameter_mm, "diameter (mm)"),
        (entry.length_mm, "length (mm)"),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(MotorError::Domain { what, value });
        }
    }
    check_writable(entry.propellant_mass_kg, "propellant mass (kg)", true)?;
    check_writable(entry.total_mass_kg, "total mass (kg)", true)
}

fn point(entry: &mut EngEntry, time: f64, thrust: f64, line: usize) -> Result<(), MotorError> {
    if time < 0.0 {
        return Err(syntax(line, format!("negative time {time}")));
    }
    if let Some(&(previous, _)) = entry.points.last()
        && time < previous
    {
        return Err(syntax(
            line,
            format!("time {time} s is before the previous point's {previous} s"),
        ));
    }
    entry.points.push((time, thrust));
    Ok(())
}

/// Ends an entry, warning about what the reader accepted but finds odd: masses, delays and the
/// curve's end. The warnings point at the header line.
fn finish(
    entry: EngEntry,
    header_line: usize,
    warnings: &mut Vec<ParseWarning>,
) -> Result<EngEntry, MotorError> {
    if entry.points.is_empty() {
        return Err(syntax(
            header_line,
            format!("the entry {:?} has no points", entry.name),
        ));
    }
    let mut warn = |kind: WarningKind, message: String| {
        warnings.push(ParseWarning::new(
            header_line,
            kind,
            format!("{}: {message}", entry.name),
        ));
    };
    if entry.propellant_mass_kg >= entry.total_mass_kg {
        warn(
            WarningKind::Unusual,
            format!(
                "propellant mass {} kg is not below the total mass {} kg",
                entry.propellant_mass_kg, entry.total_mass_kg
            ),
        );
    }
    for warning in entry.delays().warnings {
        warn(warning.kind, warning.message);
    }
    if let Some(&(_, last)) = entry.points.last()
        && last != 0.0
    {
        warn(
            WarningKind::Unusual,
            format!("the curve ends at {last} N, not zero"),
        );
    }
    if entry.points.iter().any(|&(_, thrust)| thrust < 0.0) {
        warn(WarningKind::Unusual, "the curve has negative thrust".into());
    }
    Ok(entry)
}

fn number(text: &str, what: &'static str, line: usize) -> Result<f64, MotorError> {
    finite(text, what).map_err(|_| syntax(line, format!("can't read the {what} {text:?}")))
}

fn syntax(line: usize, message: String) -> MotorError {
    MotorError::Syntax {
        format: FORMAT,
        line,
        message,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::delay::Delay;

    const TWO_MOTORS: &str = "\
; Two motors in one file.
A8 18 70 3-5 0.00312 0.01642 Estes
   0.041 0.512
   0.3 3.12
   0.73 0
;
; second
C6 18 70 0-3-5-7 .0108 0.0231 E
\t0.031\t0.946
   1.86 0
";

    /// Loft lesson L36: Loft read only the first header and appended the next entry's points to
    /// the first curve.
    #[test]
    fn multiple_blocks_parse_separately() {
        let parsed = parse(TWO_MOTORS).unwrap();
        let file = &parsed.value;
        assert_eq!(file.entries.len(), 2);
        let (a8, c6) = (&file.entries[0], &file.entries[1]);
        assert_eq!(a8.comments, [" Two motors in one file."]);
        assert_eq!(a8.name, "A8");
        assert_eq!(a8.points, [(0.041, 0.512), (0.3, 3.12), (0.73, 0.0)]);
        assert_eq!(c6.comments, [" second"]);
        assert_eq!(c6.name, "C6");
        assert_eq!(c6.propellant_mass_kg, 0.0108);
        assert_eq!(c6.manufacturer, "E");
        assert_eq!(c6.points, [(0.031, 0.946), (1.86, 0.0)]);
        // A header straight after the data also starts a new entry, with a warning.
        let unseparated = "A8 18 70 3 0.003 0.016 Estes\n 0.1 1\n 0.2 0\nB4 18 70 4 0.006 0.02 E\n 0.1 2\n 0.3 0\n";
        let parsed = parse(unseparated).unwrap();
        assert_eq!(parsed.value.entries.len(), 2);
        assert_eq!(parsed.value.entries[1].points, [(0.1, 2.0), (0.3, 0.0)]);
        assert!(parsed.warnings.iter().any(|w| w.line == 4));
    }

    /// Loft lesson L37: Loft split delays on `-` only, lost `P` and comma lists, and read `100` and
    /// `1000` (plugged markers) as seconds.
    #[test]
    fn delay_lists_and_plugged_markers_parse() {
        let delays = |raw: &str| {
            let text = format!("M1 54 400 {raw} 1 2 X\n 0.5 100\n 1 0\n");
            parse(&text).unwrap().value.entries[0].delays().delays
        };
        let s = Delay::Seconds;
        assert_eq!(delays("6-10-14"), [s(6.0), s(10.0), s(14.0)]);
        assert_eq!(delays("5,8,11"), [s(5.0), s(8.0), s(11.0)]);
        assert_eq!(delays("P"), [Delay::Plugged]);
        assert_eq!(
            delays("6-10-14-P"),
            [s(6.0), s(10.0), s(14.0), Delay::Plugged]
        );
        assert_eq!(delays("100"), [Delay::Plugged]);
        assert_eq!(delays("1000"), [Delay::Plugged]);
        assert_eq!(delays("14-12-10"), [s(14.0), s(12.0), s(10.0)]);
        // The raw string survives for writing.
        let text = "M1 54 400 6-10-14-P 1 2 X\n 0.5 100\n 1 0\n";
        let file = parse(text).unwrap().value;
        assert_eq!(file.entries[0].delays, "6-10-14-P");
        assert_eq!(parse(&write(&file).unwrap()).unwrap().value, file);
    }

    /// Loft lesson L39: Loft never checked that times increase.
    #[test]
    fn rejects_non_monotonic_time() {
        let text = "F1 29 100 P 0.03 0.08 X\n 0.1 20\n 0.5 25\n 0.4 10\n 0.9 0\n";
        match parse(text) {
            Err(MotorError::Syntax { line: 4, .. }) => {}
            other => panic!("expected a syntax error on line 4, got {other:?}"),
        }
        assert!(parse("F1 29 100 P 0.03 0.08 X\n -0.1 20\n 0.9 0\n").is_err());
        // Equal consecutive times are a step, and are kept.
        let step = parse("F1 29 100 P 0.03 0.08 X\n 0.1 20\n 0.5 20\n 0.5 0\n").unwrap();
        let curve = step.value.entries[0].thrust_curve().unwrap();
        assert_eq!(curve.times_s(), &[0.0, 0.1, 0.5, 0.5]);
        // The model refuses decreasing times too.
        let entry = EngEntry {
            points: vec![(0.5, 1.0), (0.4, 0.0)],
            ..step.value.entries[0].clone()
        };
        assert!(entry.thrust_curve().is_err());
        assert!(
            write(&EngFile {
                entries: vec![entry],
                trailing_comments: vec![]
            })
            .is_err()
        );
    }

    #[test]
    fn a_broken_entry_is_skipped_with_a_warning() {
        let text = "; one\nA1 18 70 3 0.003 0.016 X\n 0.1 1\n 0.05 0\n;\n; two\nB4 18 70 4 0.006 0.02 X\n 0.1 2\n 0.3 0\n;\nC6 18 70 x 0.01 0.02 X\n";
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.value.entries.len(), 1);
        assert_eq!(parsed.value.entries[0].name, "B4");
        assert_eq!(parsed.value.entries[0].comments, [" two"]);
        let skipped: Vec<usize> = parsed
            .warnings
            .iter()
            .filter(|w| w.message.starts_with("entry skipped"))
            .map(|w| w.line)
            .collect();
        assert_eq!(skipped, [4, 11]);

        // A corrupt data line of seven numbers is not a header for a motor named "0.2": its entry
        // is skipped, and reading resumes at the next comment.
        let text = "A1 18 70 3 0.003 0.016 X\n 0.1 1\n 0.2 5 1 2 3 4 5\n 0.3 0\n;\nB4 18 70 4 0.006 0.02 X\n 0.1 2\n 0.3 0\n";
        let parsed = parse(text).unwrap();
        let names: Vec<&str> = parsed
            .value
            .entries
            .iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(names, ["B4"]);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|w| w.line == 3 && w.kind == WarningKind::Skipped)
        );
    }

    #[test]
    fn reads_real_world_whitespace_and_numbers() {
        let text = "\u{feff};c\r\n  H128W   29 194  6-10-14   .0906 0.1966 AT\r\n\t0.02\t156.\r\n   068.5 0\r\n;\r\n; trailer   \r\n";
        let parsed = parse(text).unwrap();
        let entry = &parsed.value.entries[0];
        assert_eq!(entry.propellant_mass_kg, 0.0906);
        assert_eq!(entry.points, [(0.02, 156.0), (68.5, 0.0)]);
        assert_eq!(parsed.value.trailing_comments, [" trailer"]);
        // More than seven header fields: the rest is the manufacturer.
        let parsed = parse("K1 54 400 ;P 1 2 Contrail   Rockets\n 1 1\n 2 0\n").unwrap();
        assert_eq!(parsed.value.entries[0].manufacturer, "Contrail Rockets");
        assert_eq!(parsed.value.entries[0].delays, ";P");
        assert_eq!(parsed.warnings.len(), 2, "{:?}", parsed.warnings);
        let written = write(&parsed.value).unwrap();
        assert_eq!(parse(&written).unwrap().value, parsed.value);
        let mut spaced = parsed.value.clone();
        spaced.entries[0].manufacturer = "Contrail  Rockets".into();
        assert!(
            write(&spaced).is_err(),
            "a double space would read back as one"
        );
        // Exponents and signs read.
        let parsed = parse("K1 54 4e2 P +1 2E0 X\n 1e-1 1.5e3\n 2 0\n").unwrap();
        assert_eq!(parsed.value.entries[0].length_mm, 400.0);
        assert_eq!(parsed.value.entries[0].points[0], (0.1, 1500.0));
    }

    #[test]
    fn rejects_malformed_files() {
        let bad = [
            "",
            "; only a comment\n",
            "F1 29 100 P 0.03 0.08\n 0.1 20\n",
            "F1 29 100 P 0.03 0.08 X\n",
            "F1 29 100 P 0.03 0.08 X\n;\n 0.1 20\n",
            "F1 29 100 P 0.03 0.08 X\n 0.1 20 3\n",
            "F1 29 100 P 0.03 0.08 X\n 0.1 inf\n",
            "F1 29 100 P 0.03 0.08 X\n NaN 20\n",
            "F1 0 100 P 0.03 0.08 X\n 0.1 20\n",
            "F1 29 100 P -0.03 0.08 X\n 0.1 20\n",
            "F1 29 1e999 P 0.03 0.08 X\n 0.1 20\n",
        ];
        for text in bad {
            assert!(parse(text).is_err(), "{text:?}");
        }
    }

    #[test]
    fn warns_about_accepted_oddities() {
        let parsed = parse("F1 29 100 4-,6 0.09 0.08 X\n 0.1 20\n 0.2 5\n").unwrap();
        let messages: Vec<&str> = parsed.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages.len(), 3, "{messages:?}");
    }

    #[test]
    fn writes_what_it_reads() {
        let file = parse(TWO_MOTORS).unwrap().value;
        let text = write(&file).unwrap();
        assert_eq!(parse(&text).unwrap().value, file);
        assert_eq!(write(&parse(&text).unwrap().value).unwrap(), text);
        // Negative zero keeps its sign bit.
        let mut signed = file.clone();
        signed.entries[0].points[2].1 = -0.0;
        let back = parse(&write(&signed).unwrap()).unwrap().value;
        assert!(back.entries[0].points[2].1.is_sign_negative());
    }

    /// Every f64 in a file, as bits: `==` alone treats `-0.0` as `0.0`.
    pub(crate) fn bits(file: &EngFile) -> Vec<u64> {
        file.entries
            .iter()
            .flat_map(|e| {
                [
                    e.diameter_mm,
                    e.length_mm,
                    e.propellant_mass_kg,
                    e.total_mass_kg,
                ]
                .into_iter()
                .chain(e.points.iter().flat_map(|&(t, f)| [t, f]))
                .map(f64::to_bits)
                .collect::<Vec<_>>()
            })
            .collect()
    }

    fn token() -> impl Strategy<Value = String> {
        "[A-Za-z0-9_./()-]{1,12}".prop_filter("not a comment", |s| !s.starts_with(';'))
    }

    fn comment() -> impl Strategy<Value = String> {
        "[ -~]{0,20}[!-~]".prop_map(|s| s)
    }

    fn masses() -> impl Strategy<Value = f64> {
        prop_oneof![Just(-0.0), Just(0.0), 1e-300..1e3f64]
    }

    fn entries() -> impl Strategy<Value = EngEntry> {
        (
            prop::collection::vec(comment(), 0..3),
            (token(), token(), "[A-Za-z;]{1,6}( [A-Za-z0-9;]{1,6}){0,2}"),
            (1e-3..1e4f64, 1e-3..1e5f64, masses(), masses()),
            prop::collection::vec(
                (
                    prop_oneof![Just(-0.0), 0.0..10.0f64],
                    prop_oneof![Just(-0.0), -1.0..1e5f64],
                ),
                1..30,
            ),
        )
            .prop_map(
                |(comments, (name, delays, manufacturer), (d, l, p, t), mut points)| {
                    points.sort_by(|a, b| a.0.total_cmp(&b.0));
                    EngEntry {
                        comments,
                        name,
                        diameter_mm: d,
                        length_mm: l,
                        delays,
                        propellant_mass_kg: p,
                        total_mass_kg: t,
                        manufacturer,
                        points,
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn write_parse_round_trips_bit_for_bit(
            entries in prop::collection::vec(entries(), 1..4),
            trailing_comments in prop::collection::vec(comment(), 0..2),
        ) {
            let file = EngFile { entries, trailing_comments };
            let text = write(&file).unwrap();
            let back = parse(&text).unwrap().value;
            prop_assert_eq!(&back, &file);
            prop_assert_eq!(bits(&back), bits(&file));
        }
    }
}
