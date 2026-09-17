//! Drag override tables: the drag coefficient against Mach number, power-off and power-on, taken
//! from another tool or a measurement instead of the drag buildup.
//!
//! An override lets the flight engine fly with an oracle's drag, so that a comparison isolates the
//! dynamics, the environment and the motor from the aerodynamic prediction (`docs/VALIDATION.md`,
//! M2.1's same-drag mode). A table gives the zero-lift drag coefficient `C_D0(M)` on the rocket's
//! reference area; [`crate::AeroModel::drag`] applies the same angle-of-attack scaling to it as to
//! the buildup.
//!
//! [`parse_mach_csv`] reads CSV text (no I/O: the caller supplies the text):
//!
//! - **Two columns**: Mach number and `C_D`, optionally under one header row, as in the drag
//!   curves of RocketPy's Calisto, Juno III and Valetudo examples.
//! - **A header row**: the Mach column is the first whose name contains `mach`, and the value
//!   column is the one named by the caller (compared without case or surrounding space). Rows
//!   with an angle-of-attack column (`alpha`) other than zero are skipped, which reads RASAero II's
//!   aerodynamic exports.
//!
//! Tables interpolate linearly and hold their end values outside their range; every lookup says
//! whether it extrapolated. See `docs/physics/aero.md`.

use hpr_core::interp::{Extrapolation, Interpolation, Lookup, Table1D};
use serde::{Deserialize, Serialize};

use crate::error::AeroError;

/// Drag coefficient against Mach number, power-off and optionally power-on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct DragTable {
    /// `C_D0(M)` with no motor thrusting.
    pub power_off: Table1D,
    /// `C_D0(M)` while a motor thrusts; the power-off table applies when this is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_on: Option<Table1D>,
    /// The reference diameter the coefficients are on, m. When it differs from the rocket's,
    /// [`crate::AeroModel::drag`] rescales by the ratio of the reference areas; `None` takes the
    /// coefficients as on the rocket's reference area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_diameter_m: Option<f64>,
}

impl DragTable {
    /// A table with a power-off curve and an optional power-on curve, on the rocket's reference
    /// area.
    pub fn new(power_off: Table1D, power_on: Option<Table1D>) -> Self {
        Self {
            power_off,
            power_on,
            reference_diameter_m: None,
        }
    }

    /// This table with its coefficients on a reference diameter of `diameter_m`.
    #[must_use]
    pub fn with_reference_diameter_m(mut self, diameter_m: f64) -> Self {
        self.reference_diameter_m = Some(diameter_m);
        self
    }

    /// Reads a power-off curve and an optional power-on curve from two-column CSV text
    /// ([`parse_mach_csv`] with no column name).
    ///
    /// # Errors
    ///
    /// As [`parse_mach_csv`].
    pub fn from_csv(power_off: &str, power_on: Option<&str>) -> Result<Self, AeroError> {
        Ok(Self::new(
            parse_mach_csv(power_off, None)?,
            power_on
                .map(|text| parse_mach_csv(text, None))
                .transpose()?,
        ))
    }

    /// `C_D0` at `mach` on the table's own reference area, from the power-on curve when
    /// `thrusting` and it exists.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a negative or non-finite Mach number, and table errors.
    pub fn lookup(&self, mach: f64, thrusting: bool) -> Result<Lookup, AeroError> {
        crate::drag::check_mach_any(mach)?;
        let table = match (&self.power_on, thrusting) {
            (Some(on), true) => on,
            _ => &self.power_off,
        };
        Ok(table.lookup(mach)?)
    }
}

/// Splits a CSV line into trimmed fields. A field in double quotes may contain commas; `""`
/// inside quotes is a quote. Empty fields at the end of the line (a trailing comma) are dropped.
fn split_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => fields.push(std::mem::take(&mut field).trim().to_owned()),
            _ => field.push(c),
        }
    }
    fields.push(field.trim().to_owned());
    while fields.len() > 1 && fields.last().is_some_and(String::is_empty) {
        fields.pop();
    }
    fields
}

/// Reads a table of a value against Mach number from CSV text.
///
/// With `column` `None`, the text has two numeric columns (Mach, value), optionally under one
/// header row. With `column` `Some(name)`, the first non-blank row must be a header; the Mach
/// column is the first whose name contains `mach` and the value column the one named `name` (both
/// without regard to case or surrounding space). A column whose name starts with `alpha` (the
/// angle of attack) selects the rows where it is zero.
///
/// A row is a header only if none of its fields is a number. A leading byte-order mark, blank
/// lines, `\r\n` line ends, quoted fields, trailing commas and leading zeros (`01.05`) are
/// accepted. A row identical to the one before it is skipped. Otherwise the Mach numbers must be
/// finite and strictly increase: a Mach number repeated with another value, or out of order, is
/// refused, not sorted. The table interpolates linearly and holds its end values outside its range.
///
/// # Errors
///
/// - [`AeroError::Csv`] naming the 1-based line for a row that doesn't parse, a missing column or
///   header, a non-finite value, a Mach number that doesn't increase, or no rows (line 0).
/// - [`AeroError::Table`] for fewer than two rows.
pub fn parse_mach_csv(text: &str, column: Option<&str>) -> Result<Table1D, AeroError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .peekable();
    let csv = |line: usize, message: String| AeroError::Csv { line, message };
    let number = |f: &String| f.parse::<f64>().ok();

    let (first_line, first) = lines
        .peek()
        .map(|&(n, line)| (n, split_fields(line)))
        .ok_or_else(|| csv(0, "no rows".to_owned()))?;
    let header = first.iter().all(|f| number(f).is_none());
    let (mach_col, value_col, alpha_col, expected) = match column {
        None => {
            if header {
                lines.next();
            }
            (0, 1, None, Some(2))
        }
        Some(name) => {
            if !header {
                return Err(csv(
                    first_line,
                    format!("expected a header naming `{name}`"),
                ));
            }
            lines.next();
            let names: Vec<String> = first.iter().map(|f| f.to_lowercase()).collect();
            let mach = names
                .iter()
                .position(|f| f.contains("mach"))
                .ok_or_else(|| csv(first_line, "no Mach column".to_owned()))?;
            let wanted = name.trim().to_lowercase();
            let value = names
                .iter()
                .position(|f| *f == wanted)
                .ok_or_else(|| csv(first_line, format!("no column named `{name}`")))?;
            let alpha = names.iter().position(|f| f.starts_with("alpha"));
            (mach, value, alpha, None)
        }
    };

    let (mut xs, mut ys): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    for (n, line) in lines {
        let values: Vec<f64> = split_fields(line)
            .iter()
            .map(number)
            .collect::<Option<_>>()
            .ok_or_else(|| csv(n, format!("not a row of numbers: `{line}`")))?;
        if let Some(expected) = expected
            && values.len() != expected
        {
            return Err(csv(
                n,
                format!("expected {expected} columns, found {}", values.len()),
            ));
        }
        let get = |index: usize| {
            values
                .get(index)
                .copied()
                .ok_or_else(|| csv(n, format!("missing column {}", index + 1)))
        };
        if let Some(alpha) = alpha_col
            && get(alpha)? != 0.0
        {
            continue;
        }
        let (mach, value) = (get(mach_col)?, get(value_col)?);
        if !(mach.is_finite() && value.is_finite()) {
            return Err(csv(n, format!("not finite: Mach {mach}, value {value}")));
        }
        if let (Some(&last), Some(&last_value)) = (xs.last(), ys.last()) {
            // A repeated row is harmless; RocketPy's Cavour curve repeats its first one.
            if mach == last && value == last_value {
                continue;
            }
            if mach <= last {
                return Err(csv(
                    n,
                    format!("Mach {mach} doesn't increase from the previous row's {last}"),
                ));
            }
        }
        xs.push(mach);
        ys.push(value);
    }
    if xs.is_empty() {
        return Err(csv(0, "no rows".to_owned()));
    }
    Ok(Table1D::new(
        xs,
        ys,
        Interpolation::Linear,
        Extrapolation::Clamp,
    )?)
}

#[cfg(test)]
mod tests {
    use hpr_core::interp::Side;

    use super::*;

    #[test]
    fn reads_two_column_curves_with_their_quirks() {
        // CRLF line ends, a leading zero, a trailing blank line.
        let text = "0.01,0.949\r\n0.02,01.05\r\n0.30,0.40\r\n\r\n";
        let table = parse_mach_csv(text, None).unwrap();
        assert_eq!(table.xs(), [0.01, 0.02, 0.30]);
        assert_eq!(table.ys(), [0.949, 1.05, 0.40]);
        let with_header = parse_mach_csv("Mach,Cd\n0.1,0.5\n0.2,0.6\n", None).unwrap();
        assert_eq!(with_header.ys(), [0.5, 0.6]);
    }

    /// RASAero II's aerodynamic export: its header row (as in the export behind RocketPy's Calisto
    /// curve), one row per Mach number and angle of attack.
    #[test]
    fn named_columns_and_zero_alpha_rows() {
        let header = "Mach,Alpha,CD,CD Power-Off,CD Power-On,CA Power-Off,CA Power-On,CL,CN,\
                      CN Potential,CN Viscous,CNalpha (0 to 4 deg) (per rad),CP,CP (0 to 4 deg),\
                      Reynolds Number";
        let text = format!(
            "{header}\n\
             0.1,0,0.51,0.50,0.40,0.50,0.40,0,0,0,0,6.1,66.2,66.2,5931000\n\
             0.1,2,0.56,0.55,0.45,0.55,0.45,0.1,0.2,0.1,0.1,6.1,66.0,66.2,5931000\n\
             0.2,0,0.53,0.52,0.42,0.52,0.42,0,0,0,0,6.1,66.2,66.2,11862000\n\
             0.2,4,0.58,0.57,0.47,0.57,0.47,0.4,0.4,0.2,0.2,6.1,65.8,66.2,11862000\n"
        );
        let text = text.as_str();
        let off = parse_mach_csv(text, Some("cd power-off")).unwrap();
        let on = parse_mach_csv(text, Some(" CD Power-On ")).unwrap();
        assert_eq!(
            (off.xs(), off.ys()),
            ([0.1, 0.2].as_slice(), [0.50, 0.52].as_slice())
        );
        assert_eq!(on.ys(), [0.40, 0.42]);
        let table = DragTable::new(off, Some(on));
        let mid = table.lookup(0.15, true).unwrap();
        assert!((mid.value - 0.41).abs() < 1e-15);
        assert_eq!(mid.extrapolated, None);
        assert_eq!(table.lookup(0.15, false).unwrap().value, 0.51);
        let high = table.lookup(3.0, false).unwrap();
        assert_eq!((high.value, high.extrapolated), (0.52, Some(Side::Above)));
    }

    #[test]
    fn malformed_text_names_the_line() {
        let err = |text: &str, column| parse_mach_csv(text, column).unwrap_err();
        let line = |text: &str, column| match err(text, column) {
            AeroError::Csv { line, .. } => line,
            other => panic!("expected a CSV error, got {other:?}"),
        };
        assert_eq!(line("", None), 0);
        assert_eq!(line("0.1,0.5\n0.2,x\n", None), 2);
        assert_eq!(line("0.1,0.5,0.4\n", None), 1);
        assert_eq!(line("0.1,0.5\n", Some("cd")), 1);
        assert_eq!(line("Mach,CD\n0.1,0.5\n", Some("CA")), 1);
        // A first row with a number in it is a bad row, not a header.
        assert_eq!(line("0.1,0.5x\n0.2,0.6\n0.3,0.7\n", None), 1);
        // Duplicate, unsorted and non-finite rows name their line, counting skipped lines.
        assert_eq!(line("0.2,0.5\n\n0.1,0.5\n", None), 3);
        assert_eq!(line("0.1,0.5\n0.1,0.6\n", None), 2);
        // An identical repeated row is skipped.
        let repeated = parse_mach_csv("0.1,0.5\n0.1,0.5\n0.1,0.5\n0.2,0.6\n", None).unwrap();
        assert_eq!(repeated.xs(), [0.1, 0.2]);
        assert_eq!(line("0.1,0.5\n0.2,nan\n", None), 2);
        assert_eq!(line("0.1,0.5\n0.2,inf\n", None), 2);
        let alpha = "Mach,Alpha,CD\n0.1,0,0.5\n0.1,2,0.6\n0.1,0,0.7\n";
        assert_eq!(line(alpha, Some("CD")), 4);
        assert!(matches!(err("0.2,0.5\n", None), AeroError::Table(_)));
        let table = DragTable::from_csv("0.1,0.5\n0.2,0.6\n", None).unwrap();
        assert!(matches!(
            table.lookup(-0.1, false),
            Err(AeroError::Domain { .. })
        ));
        assert!(matches!(
            table.lookup(f64::NAN, false),
            Err(AeroError::Domain { .. })
        ));
        // Mach 3 is fine for a table.
        assert_eq!(table.lookup(3.0, false).unwrap().value, 0.6);
        // No power-on curve: the power-off one applies while thrusting.
        assert_eq!(table.lookup(0.1, true).unwrap().value, 0.5);
    }

    /// A byte-order mark, quoted fields with commas, and trailing commas.
    #[test]
    fn byte_order_marks_quotes_and_trailing_commas() {
        let table = parse_mach_csv("\u{feff}0.01,0.949\n0.02,1.05\n", None).unwrap();
        assert_eq!(table.xs(), [0.01, 0.02]);
        let text = "\"Mach\",\"CN (0,4)\",\"CD\",\n0.1,6.1,0.5,\n0.2,6.2,0.6,\n";
        let table = parse_mach_csv(text, Some("cd")).unwrap();
        assert_eq!(table.ys(), [0.5, 0.6]);
        assert_eq!(split_fields("a,\"b,\"\"c\"\"\",d,,"), ["a", "b,\"c\"", "d"]);
    }
}
