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
//! - **Two columns, no header**: Mach number and `C_D`, as in RocketPy's drag-curve files.
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
pub struct DragTable {
    /// `C_D0(M)` with no motor thrusting.
    pub power_off: Table1D,
    /// `C_D0(M)` while a motor thrusts; the power-off table applies when this is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub power_on: Option<Table1D>,
}

impl DragTable {
    /// A table with a power-off curve and an optional power-on curve.
    pub fn new(power_off: Table1D, power_on: Option<Table1D>) -> Self {
        Self {
            power_off,
            power_on,
        }
    }

    /// Reads a power-off curve and an optional power-on curve from two-column CSV text
    /// ([`parse_mach_csv`] with no column name).
    ///
    /// # Errors
    ///
    /// As [`parse_mach_csv`].
    pub fn from_csv(power_off: &str, power_on: Option<&str>) -> Result<Self, AeroError> {
        Ok(Self {
            power_off: parse_mach_csv(power_off, None)?,
            power_on: power_on
                .map(|text| parse_mach_csv(text, None))
                .transpose()?,
        })
    }

    /// `C_D0` at `mach`, from the power-on curve when `thrusting` and it exists.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] for a negative or non-finite Mach number, and table errors.
    pub fn lookup(&self, mach: f64, thrusting: bool) -> Result<Lookup, AeroError> {
        if !(mach.is_finite() && mach >= 0.0) {
            return Err(AeroError::Mach { mach });
        }
        let table = match (&self.power_on, thrusting) {
            (Some(on), true) => on,
            _ => &self.power_off,
        };
        Ok(table.lookup(mach)?)
    }
}

/// Reads a table of a value against Mach number from CSV text.
///
/// With `column` `None`, the text must have exactly two numeric columns (Mach, value) and may
/// start with one header row, which is ignored. With `column` `Some(name)`, the first non-blank
/// row must be a header; the Mach column is the first whose name contains `mach` and the value
/// column the one named `name` (both without regard to case or surrounding space). A column named
/// like `alpha` (the angle of attack) selects the rows where it is zero.
///
/// Blank lines are skipped, `\r\n` line ends are accepted, and numbers may have leading zeros
/// (`01.05`). The Mach numbers must strictly increase. The table interpolates linearly and holds
/// its end values outside its range.
///
/// # Errors
///
/// - [`AeroError::Csv`] naming the line for a row that doesn't parse, a missing column, or no
///   rows.
/// - [`AeroError::Table`] for Mach numbers that don't strictly increase, or too few rows.
pub fn parse_mach_csv(text: &str, column: Option<&str>) -> Result<Table1D, AeroError> {
    let mut lines = text
        .lines()
        .enumerate()
        .map(|(i, line)| (i + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .peekable();
    let csv = |line: usize, message: String| AeroError::Csv { line, message };
    let fields = |line: &str| -> Vec<String> {
        line.split(',')
            .map(|f| f.trim().trim_matches('"').trim().to_owned())
            .collect()
    };
    let numbers = |row: &[String]| -> Option<Vec<f64>> {
        row.iter().map(|f| f.parse::<f64>().ok()).collect()
    };

    let first = lines
        .peek()
        .map(|&(n, line)| (n, fields(line)))
        .ok_or_else(|| csv(0, "no rows".to_owned()))?;
    let header = numbers(&first.1).is_none();
    let (mach_col, value_col, alpha_col, expected) = match column {
        None => {
            if header {
                lines.next();
            }
            (0, 1, None, Some(2))
        }
        Some(name) => {
            if !header {
                return Err(csv(first.0, format!("expected a header naming `{name}`")));
            }
            lines.next();
            let names: Vec<String> = first.1.iter().map(|f| f.to_lowercase()).collect();
            let mach = names
                .iter()
                .position(|f| f.contains("mach"))
                .ok_or_else(|| csv(first.0, "no Mach column".to_owned()))?;
            let wanted = name.trim().to_lowercase();
            let value = names
                .iter()
                .position(|f| *f == wanted)
                .ok_or_else(|| csv(first.0, format!("no column named `{name}`")))?;
            let alpha = names.iter().position(|f| f.starts_with("alpha"));
            (mach, value, alpha, None)
        }
    };

    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for (n, line) in lines {
        let row = fields(line);
        let values =
            numbers(&row).ok_or_else(|| csv(n, format!("not a row of numbers: `{line}`")))?;
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
        xs.push(get(mach_col)?);
        ys.push(get(value_col)?);
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
        assert!(matches!(err("", None), AeroError::Csv { line: 0, .. }));
        assert!(matches!(
            err("0.1,0.5\n0.2,x\n", None),
            AeroError::Csv { line: 2, .. }
        ));
        assert!(matches!(
            err("0.1,0.5,0.4\n", None),
            AeroError::Csv { line: 1, .. }
        ));
        assert!(matches!(
            err("0.1,0.5\n", Some("cd")),
            AeroError::Csv { line: 1, .. }
        ));
        assert!(matches!(
            err("Mach,CD\n0.1,0.5\n", Some("CA")),
            AeroError::Csv { line: 1, .. }
        ));
        assert!(matches!(
            err("0.2,0.5\n0.1,0.5\n", None),
            AeroError::Table(_)
        ));
        assert!(matches!(err("0.2,0.5\n", None), AeroError::Table(_)));
        let table = DragTable::from_csv("0.1,0.5\n0.2,0.6\n", None).unwrap();
        assert!(matches!(
            table.lookup(-0.1, false),
            Err(AeroError::Mach { .. })
        ));
        assert!(matches!(
            table.lookup(f64::NAN, false),
            Err(AeroError::Mach { .. })
        ));
        // No power-on curve: the power-off one applies while thrusting.
        assert_eq!(table.lookup(0.1, true).unwrap().value, 0.5);
    }
}
