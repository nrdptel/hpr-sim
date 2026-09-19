//! Override tables: coefficients from another tool or a measurement in place of hpr's own.
//!
//! An override lets the flight engine fly with an oracle's aerodynamics, so that a comparison
//! isolates the dynamics, the environment and the motor from the aerodynamic prediction
//! (`docs/VALIDATION.md`, the same-drag mode of [M2.1][m2-1], the comparisons with RocketPy).
//!
//! - A [`DragTable`] gives the zero-lift drag coefficient `C_D0(M)` on the rocket's reference area;
//!   [`crate::AeroModel::drag`] applies the same angle-of-attack scaling to it as to the buildup.
//! - A [`NormalForceTable`] gives the normal force and its centre of pressure against Mach number
//!   and angle of attack, read from RASAero II's aerodynamic export
//!   ([`NormalForceTable::from_rasaero_csv`]); [`crate::AeroModel::normal_force`] returns it in
//!   place of the Barrowman sum. The export carries no damping, so a flight keeps hpr's own
//!   (`docs/physics/flight.md`).
//!
//! [m2-1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1
//!
//! [`parse_mach_csv`] reads CSV text for a drag table (no I/O: the caller supplies the text):
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

use std::f64::consts::{FRAC_PI_2, PI};

use hpr_core::interp::{Extrapolation, Interpolation, Lookup, Side, Table1D};
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
            // An identical repeated row is harmless (RocketPy's Cavour curve has several).
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

/// The inch in metres, exactly. RASAero II takes its dimensions in inches (RASAero II Users
/// Manual, 2019, p. 13), and its export gives the centre of pressure in them.
const INCH_M: f64 = 0.0254;

/// One angle of attack's column of a [`NormalForceTable`]: the normal force's slope and centre of
/// pressure against Mach number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct NormalForceColumn {
    /// The angle of attack, rad, in `[0, π/2)`.
    pub alpha_rad: f64,
    /// `C_N/α` against Mach number, per radian, on the table's reference area. In a column at
    /// `α = 0` it is the slope `∂C_N/∂α` there.
    pub slope_per_rad: Table1D,
    /// The centre of pressure against Mach number, m aft of the nose tip.
    pub cp_station_m: Table1D,
}

impl NormalForceColumn {
    /// The column at `alpha_rad`: `slope_per_rad` (`C_N/α`, per radian) and `cp_station_m` (m aft
    /// of the nose tip), each against Mach number. [`NormalForceTable::new`] checks the angle.
    pub fn new(alpha_rad: f64, slope_per_rad: Table1D, cp_station_m: Table1D) -> Self {
        Self {
            alpha_rad,
            slope_per_rad,
            cp_station_m,
        }
    }
}

/// The normal force and its centre of pressure against Mach number and angle of attack, from
/// another tool, in place of hpr's own ([`crate::AeroModel::with_normal_force_table`]).
///
/// A table is a set of columns, one per angle of attack, each holding `C_N/α` and the centre of
/// pressure against Mach number. A lookup at Mach `M` and angle `α`:
///
/// - reads each column at `M` (linear, holding the end values outside its Mach range, as the
///   column's own tables say), then interpolates linearly in `α` between the two columns around
///   it. Below the first column's angle it holds that column. So `C_N = (C_N/α)·α` gives the
///   columns' normal force back at their own angles, and between them a quadratic in `α`: a
///   potential-flow term linear in `α` plus a viscous cross-flow term in `α²`. RASAero II's
///   viscous part is `sin² α` from Mach 0.91 to 1.3 in the Calisto export, which `α²` matches
///   within 0.2% to 4°; faster, it grows more slowly, and the quadratic between the columns is an
///   assumption.
/// - Past the last column's angle `α_n`, with `s = sin α / sin α_n`, the force at `α_n` grows
///   as `s` at the last column's centre of pressure, as hpr's own fins follow `sin α` (the
///   decision record on flight, [ADR-011][adr-011]). When the table starts at 0°, the part of
///   that force beyond the 0° slope's linear share (`(C_N/α)(0) · α_n`), the rest `R`, grows
///   faster, as `s²`, the cross flow's form (Galejs; Niskanen 2009 eq. 3.26), which RASAero II's
///   viscous part takes from Jorgensen (RASAero II Users Manual, 2019, p. 55): the force is
///   `C_N(α_n) s + R (s² − s)`, with the extra term at the station where the rest acts in the
///   split (the one that gives the moment at `α_n` with the linear share at the 0° centre of
///   pressure). That is exactly the linear share growing as `s` and the rest as `s²`, each at its
///   own centre of pressure; the station is held within the rocket (or the table's own range of
///   centres of pressure, [`NormalForceTable::lookup`]), and the linear share within the force.
///   So the force never turns round and is zero tail first, the centre of pressure lies between
///   the last column's and that station while `s ≥ 1` (from `α_n` to `π − α_n`), and everything
///   is continuous at `α_n` and in the table's values. This part is an assumption, not the other
///   tool's result; the lookup reports it.
///
/// The table serializes as its columns and its [`TableReference`], and re-checks them when
/// read. The decisions are in the record on normal-force overrides, [ADR-032][adr-032].
///
/// [adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
/// [adr-032]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-032-normal-force-overrides-from-rasaero-ii-the-static-force-replaced-hprs-damping-kept-2026-09-19
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "NormalForceTableData", into = "NormalForceTableData")]
pub struct NormalForceTable {
    columns: Vec<NormalForceColumn>,
    reference: TableReference,
}

/// The area a [`NormalForceTable`]'s coefficients are on. [`crate::AeroModel::normal_force`]
/// rescales them to the rocket's reference area.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum TableReference {
    /// The rocket's own reference area.
    #[default]
    Rocket,
    /// The largest cross-section of the rocket's bodies, as RASAero II's (RASAero II Users
    /// Manual, 2019, p. 72).
    LargestBody,
    /// A circle of this diameter.
    Diameter {
        /// The diameter, m.
        diameter_m: f64,
    },
}

/// The serialized form of a [`NormalForceTable`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NormalForceTableData {
    columns: Vec<NormalForceColumn>,
    #[serde(default)]
    reference: TableReference,
}

impl TryFrom<NormalForceTableData> for NormalForceTable {
    type Error = AeroError;

    fn try_from(data: NormalForceTableData) -> Result<Self, AeroError> {
        NormalForceTable::new(data.columns)?.with_reference(data.reference)
    }
}

impl From<NormalForceTable> for NormalForceTableData {
    fn from(table: NormalForceTable) -> Self {
        NormalForceTableData {
            columns: table.columns,
            reference: table.reference,
        }
    }
}

/// A [`NormalForceTable`]'s normal force at one Mach number and angle of attack, on the table's
/// reference area.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct NormalForceLookup {
    /// The normal-force coefficient `C_N`.
    pub coefficient: f64,
    /// `C_N/α` per radian; at `α = 0`, the slope.
    pub slope_per_rad: f64,
    /// The centre of pressure, m aft of the nose tip.
    pub cp_station_m: f64,
    /// `Some` when the Mach number was outside a column's range and its end value was held.
    pub mach_extrapolated: Option<Side>,
    /// Whether the angle of attack was past the last column's, where the cross-flow continuation
    /// applies.
    pub beyond_alpha: bool,
}

impl NormalForceTable {
    /// A table from its columns, in increasing angle of attack, on the rocket's reference area.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for no columns, an angle outside `[0, π/2)`, or angles that don't
    /// strictly increase.
    pub fn new(columns: Vec<NormalForceColumn>) -> Result<Self, AeroError> {
        let domain = |value| AeroError::Domain {
            what: "normal-force table angle of attack",
            value,
        };
        if columns.is_empty() {
            return Err(AeroError::Domain {
                what: "normal-force table column count",
                value: 0.0,
            });
        }
        let mut previous: Option<f64> = None;
        for column in &columns {
            let alpha = column.alpha_rad;
            if !(0.0..FRAC_PI_2).contains(&alpha) || previous.is_some_and(|p| alpha <= p) {
                return Err(domain(alpha));
            }
            previous = Some(alpha);
        }
        Ok(Self {
            columns,
            reference: TableReference::Rocket,
        })
    }

    /// This table with its coefficients on `reference`. When it differs from the rocket's,
    /// [`crate::AeroModel::normal_force`] rescales by the ratio of the areas.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a diameter that isn't finite and positive.
    pub fn with_reference(mut self, reference: TableReference) -> Result<Self, AeroError> {
        if let TableReference::Diameter { diameter_m } = reference {
            crate::error::check_dimension(
                "normal-force table reference diameter",
                diameter_m,
                false,
            )?;
        }
        self.reference = reference;
        Ok(self)
    }

    /// This table with its coefficients on a circle of diameter `diameter_m`
    /// ([`NormalForceTable::with_reference`]).
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] unless `diameter_m` is finite and positive.
    pub fn with_reference_diameter_m(self, diameter_m: f64) -> Result<Self, AeroError> {
        self.with_reference(TableReference::Diameter { diameter_m })
    }

    /// The columns, in increasing angle of attack.
    pub fn columns(&self) -> &[NormalForceColumn] {
        &self.columns
    }

    /// The area the coefficients are on.
    pub fn reference(&self) -> TableReference {
        self.reference
    }

    /// The normal force at `mach` and angle of attack `alpha_rad`, on the table's reference area,
    /// as the type's documentation describes, with the growing share's centre of pressure past
    /// the last column held from the nose tip to twice the table's largest centre of pressure: a
    /// stand-in for the rocket, which the table doesn't know ([`NormalForceTable::lookup_within`];
    /// a flight passes the rocket's own length).
    ///
    /// # Errors
    ///
    /// As [`NormalForceTable::lookup_within`].
    pub fn lookup(&self, mach: f64, alpha_rad: f64) -> Result<NormalForceLookup, AeroError> {
        let largest = self
            .columns
            .iter()
            .flat_map(|c| c.cp_station_m.ys())
            .fold(0.0_f64, |largest, &cp| largest.max(cp));
        self.lookup_within(mach, alpha_rad, (0.0, 2.0 * largest))
    }

    /// As [`NormalForceTable::lookup`], with the growing share's centre of pressure past the last
    /// column held within `stations_m`, m aft of the nose tip: [`crate::AeroModel`] passes the
    /// rocket, from its nose tip to its aft end.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a negative or non-finite Mach number, an angle outside `[0, π]`,
    /// or stations that aren't finite and in order; and table errors from a column whose Mach
    /// range refuses to extrapolate.
    pub fn lookup_within(
        &self,
        mach: f64,
        alpha_rad: f64,
        stations_m: (f64, f64),
    ) -> Result<NormalForceLookup, AeroError> {
        crate::drag::check_mach_any(mach)?;
        let (fore, aft) = stations_m;
        if !(fore.is_finite() && aft.is_finite() && fore <= aft) {
            return Err(AeroError::Domain {
                what: "normal-force table's stations, fore end",
                value: fore,
            });
        }
        if !(0.0..=PI).contains(&alpha_rad) {
            return Err(AeroError::Domain {
                what: "angle of attack",
                value: alpha_rad,
            });
        }
        let mut mach_extrapolated = None;
        let mut read = |column: &NormalForceColumn| -> Result<(f64, f64), AeroError> {
            let slope = column.slope_per_rad.lookup(mach)?;
            let cp = column.cp_station_m.lookup(mach)?;
            mach_extrapolated = mach_extrapolated.or(slope.extrapolated).or(cp.extrapolated);
            Ok((slope.value, cp.value))
        };
        // The constructor refuses an empty table, so there is a last column.
        let last = self.columns.len() - 1;
        let alpha_n = self.columns[last].alpha_rad;
        let held = alpha_rad.min(alpha_n);
        // The first column at or above `held`: there is one, since `held` is at most `α_n`.
        let upper = self
            .columns
            .partition_point(|c| c.alpha_rad < held)
            .min(last);
        let above = &self.columns[upper];
        let (slope, cp) = if above.alpha_rad == held || upper == 0 {
            // At a column's own angle, or below the first: that column alone.
            read(above)?
        } else {
            let below = &self.columns[upper - 1];
            let t = (held - below.alpha_rad) / (above.alpha_rad - below.alpha_rad);
            let (s0, x0) = read(below)?;
            let (s1, x1) = read(above)?;
            (s0 + t * (s1 - s0), x0 + t * (x1 - x0))
        };
        let beyond_alpha = alpha_rad > alpha_n;
        let (coefficient, cp) = if !beyond_alpha {
            (slope * alpha_rad, cp)
        } else if alpha_n == 0.0 {
            // One column, at 0°: its slope, following the cross flow.
            (slope * alpha_rad.sin(), cp)
        } else {
            // The whole force at `α_n` grows as the cross flow, at its own centre of pressure;
            // the rest beyond the 0° slope's linear share grows faster, as its square, at the
            // station that keeps the moment of the split, held within `stations_m`. Its extra
            // term is zero at `α_n` whatever that station, so the force and its centre of
            // pressure are continuous there and in the table's values.
            let s1 = alpha_rad.sin() / alpha_n.sin();
            let force_n = slope * alpha_n;
            let rest = match self.columns.first() {
                Some(first) if first.alpha_rad == 0.0 && force_n > 0.0 => {
                    let (linear_slope, linear_cp) = read(first)?;
                    let linear = (linear_slope * alpha_n).clamp(0.0, force_n);
                    let rest = force_n - linear;
                    (rest > 0.0).then(|| {
                        let station = (force_n * cp - linear * linear_cp) / rest;
                        (rest, station.clamp(stations_m.0, stations_m.1))
                    })
                }
                _ => None,
            };
            match rest {
                Some((rest, station)) => {
                    let extra = rest * (s1 * s1 - s1);
                    let force = force_n * s1 + extra;
                    let moment = force_n * s1 * cp + extra * station;
                    (force, if force != 0.0 { moment / force } else { cp })
                }
                None => (force_n * s1, cp),
            }
        };
        Ok(NormalForceLookup {
            coefficient,
            slope_per_rad: if alpha_rad > 0.0 {
                coefficient / alpha_rad
            } else {
                slope
            },
            cp_station_m: cp,
            mach_extrapolated,
            beyond_alpha,
        })
    }

    /// Reads RASAero II's aerodynamic export (its Aero Plots screen, File, Export, To CSV File:
    /// RASAero II Users Manual, 2019, p. 76), one column per angle of attack in it.
    ///
    /// The first non-blank row is the header. The columns used are named `Mach`, `Alpha` (degrees),
    /// `CN`, `CN Potential` and `CP` (compared without case or surrounding space); the export's
    /// others, such as `CNalpha (0 to 4 deg) (per rad)`, a secant slope to 4°, are not read.
    ///
    /// - At an angle `α > 0`, the column's slope is `CN/α` and its centre of pressure is `CP`.
    /// - At `α = 0` the export's `CN` is zero, so the slope is its potential-flow normal force at
    ///   the smallest positive angle `α₁` and the same Mach number over that angle,
    ///   `CN Potential(α₁)/α₁`: the potential part is linear in `α`, and the viscous cross-flow
    ///   part, `sin² α` through Mach 1.3 in the Calisto export, has no slope at zero (faster, how
    ///   it starts from 0° isn't in the export, and leaving it out is an assumption). The centre of pressure is the export's at
    ///   `α = 0`.
    /// - `CP` is in inches (the manual, p. 13) from the nose tip ("distance measured from the
    ///   nose", p. 114), converted at 0.0254 m to the inch. hpr's stations are also aft of the
    ///   nose tip, so the design must start at the same nose tip as RASAero II's.
    /// - The coefficients are on RASAero II's reference area, the largest cross-section of the
    ///   body (p. 72): the table's reference is [`TableReference::LargestBody`], which
    ///   [`crate::AeroModel::normal_force`] rescales to the rocket's reference area.
    ///
    /// The five columns read must hold a number in every row; the others are not read. Within one
    /// angle of attack an identical repeated row is skipped, and otherwise the Mach numbers must strictly increase: a Mach number
    /// repeated with other values, or out of order, is refused with its line, not sorted. The
    /// angles need not all have the same Mach numbers (the export behind RocketPy's Calisto ends
    /// its 4° rows a row early).
    ///
    /// # Errors
    ///
    /// - [`AeroError::Csv`] naming the 1-based line for a missing header column, a field read that
    ///   isn't a number, a non-finite value, an angle outside `[0°, 90°)`, a Mach number that
    ///   doesn't increase within its angle, a row at `α = 0` whose Mach number has no row at `α₁`,
    ///   an angle with one Mach number (its row's line), or no rows at a positive angle (line 0).
    ///
    /// # Examples
    ///
    /// Two Mach numbers at 0° and 2°, with only the columns the reader uses (an export has more):
    ///
    /// ```
    /// use hpr_aero::NormalForceTable;
    ///
    /// let export = "Mach,Alpha,CN,CN Potential,CP\n\
    ///               0.3,0,0,0,40\n\
    ///               0.5,0,0,0,40\n\
    ///               0.3,2,0.35,0.35,40\n\
    ///               0.5,2,0.35,0.35,40\n";
    /// let table = NormalForceTable::from_rasaero_csv(export)?;
    /// // At 1°, halfway between the columns at 0° and 2°, which here agree.
    /// let at = table.lookup(0.4, 1_f64.to_radians())?;
    /// assert!((at.slope_per_rad - 0.35 / 2_f64.to_radians()).abs() < 1e-12);
    /// // 40 inches aft of the nose tip, in metres.
    /// assert!((at.cp_station_m - 1.016).abs() < 1e-12);
    /// # Ok::<(), hpr_aero::AeroError>(())
    /// ```
    pub fn from_rasaero_csv(text: &str) -> Result<Self, AeroError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = text
            .lines()
            .enumerate()
            .map(|(i, line)| (i + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty());
        let csv = |line: usize, message: String| AeroError::Csv { line, message };
        let (header_line, header) = lines.next().ok_or_else(|| csv(0, "no rows".to_owned()))?;
        let names: Vec<String> = split_fields(header)
            .iter()
            .map(|f| f.to_lowercase())
            .collect();
        let column = |name: &str| {
            names
                .iter()
                .position(|f| f == name)
                .ok_or_else(|| csv(header_line, format!("no column named `{name}`")))
        };
        let (mach_col, alpha_col, cn_col, potential_col, cp_col) = (
            column("mach")?,
            column("alpha")?,
            column("cn")?,
            column("cn potential")?,
            column("cp")?,
        );

        // The rows of each angle of attack (degrees, as written), in the order they come.
        let mut angles: Vec<(f64, Vec<RasaeroRow>)> = Vec::new();
        for (n, line) in lines {
            // Only the columns read must be numbers; the export's others may hold anything.
            let fields = split_fields(line);
            let get = |index: usize| {
                let field = fields
                    .get(index)
                    .ok_or_else(|| csv(n, format!("missing column {}", index + 1)))?;
                field.parse::<f64>().map_err(|_| {
                    csv(
                        n,
                        format!("column {} is not a number: `{field}`", index + 1),
                    )
                })
            };
            let alpha_deg = get(alpha_col)?;
            let row = RasaeroRow {
                line: n,
                mach: get(mach_col)?,
                cn: get(cn_col)?,
                potential: get(potential_col)?,
                cp_in: get(cp_col)?,
            };
            if ![alpha_deg, row.mach, row.cn, row.potential, row.cp_in]
                .iter()
                .all(|v| v.is_finite())
            {
                return Err(csv(n, format!("not finite: `{line}`")));
            }
            if !(0.0..90.0).contains(&alpha_deg) {
                return Err(csv(
                    n,
                    format!("angle of attack {alpha_deg}° is outside [0°, 90°)"),
                ));
            }
            let index = match angles.iter().position(|(a, _)| *a == alpha_deg) {
                Some(index) => index,
                None => {
                    angles.push((alpha_deg, Vec::new()));
                    angles.len() - 1
                }
            };
            let rows = &mut angles[index].1;
            if let Some(last) = rows.last() {
                // An identical repeated row is harmless, as in `parse_mach_csv`.
                if row.same_values(last) {
                    continue;
                }
                if row.mach <= last.mach {
                    return Err(csv(
                        n,
                        format!(
                            "Mach {} doesn't increase from line {}'s {} at {alpha_deg}°",
                            row.mach, last.line, last.mach
                        ),
                    ));
                }
            }
            rows.push(row);
        }
        angles.sort_by(|a, b| a.0.total_cmp(&b.0));
        let (first_positive_deg, first_positive) = angles
            .iter()
            .find(|(a, _)| *a > 0.0)
            .ok_or_else(|| csv(0, "no rows at a positive angle of attack".to_owned()))?;
        let first_positive_rad = first_positive_deg.to_radians();

        let table = |xs: Vec<f64>, ys: Vec<f64>| {
            Table1D::new(xs, ys, Interpolation::Linear, Extrapolation::Clamp)
        };
        let mut columns = Vec::with_capacity(angles.len());
        for (alpha_deg, rows) in &angles {
            let alpha_rad = alpha_deg.to_radians();
            let mut slopes = Vec::with_capacity(rows.len());
            for row in rows {
                slopes.push(if *alpha_deg > 0.0 {
                    row.cn / alpha_rad
                } else {
                    // Both sets of rows increase in Mach, so a binary search finds the match.
                    let at = first_positive.partition_point(|r| r.mach < row.mach);
                    match first_positive.get(at) {
                        Some(r) if r.mach == row.mach => r.potential / first_positive_rad,
                        _ => {
                            return Err(csv(
                                row.line,
                                format!(
                                    "no row at Mach {} and {first_positive_deg}° to give the \
                                     slope at 0°",
                                    row.mach
                                ),
                            ));
                        }
                    }
                });
            }
            if rows.len() < Table1D::MIN_KNOTS {
                let line = rows.first().map_or(0, |r| r.line);
                return Err(csv(
                    line,
                    format!("{alpha_deg}° has one Mach number; a column needs two"),
                ));
            }
            let machs: Vec<f64> = rows.iter().map(|r| r.mach).collect();
            let cps = rows.iter().map(|r| r.cp_in * INCH_M).collect();
            columns.push(NormalForceColumn::new(
                alpha_rad,
                table(machs.clone(), slopes)?,
                table(machs, cps)?,
            ));
        }
        Self::new(columns)?.with_reference(TableReference::LargestBody)
    }
}

/// One row of a RASAero II export, as [`NormalForceTable::from_rasaero_csv`] uses it.
struct RasaeroRow {
    line: usize,
    mach: f64,
    cn: f64,
    potential: f64,
    cp_in: f64,
}

impl RasaeroRow {
    fn same_values(&self, other: &RasaeroRow) -> bool {
        (self.mach, self.cn, self.potential, self.cp_in)
            == (other.mach, other.cn, other.potential, other.cp_in)
    }
}

#[cfg(test)]
mod tests {
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

    /// RASAero II's header, as in the export behind RocketPy's Calisto curve.
    const RASAERO_HEADER: &str = "Mach,Alpha,CD,CD Power-Off,CD Power-On,CA Power-Off,\
                                  CA Power-On,CL,CN,CN Potential,CN Viscous,\
                                  CNalpha (0 to 4 deg) (per rad),CP,CP (0 to 4 deg),\
                                  Reynolds Number";

    /// One export row: Mach, the angle in degrees, `CN Potential`, `CN Viscous` and `CP` in
    /// inches; `CN` is their sum, and the columns the reader doesn't use are filler.
    fn rasaero_row(mach: f64, alpha_deg: f64, potential: f64, viscous: f64, cp_in: f64) -> String {
        format!(
            "{mach},{alpha_deg},0.5,0.5,0.4,0.5,0.4,0,{},{potential},{viscous},9.9,{cp_in},55.5,1e6",
            potential + viscous
        )
    }

    /// A small export in RASAero II's layout, with invented numbers: all of 0°'s rows, then 2°'s,
    /// then 4°'s, whose last Mach number is missing as in the Calisto export. At Mach 0.5 the
    /// potential slope is 7 per radian with no viscous part; at Mach 1 and 1.5 a viscous part
    /// grows as `α²` and the CP moves forward with the angle.
    fn small_export() -> String {
        let r2 = 2.0_f64.to_radians();
        let r4 = 4.0_f64.to_radians();
        let mut rows = vec![RASAERO_HEADER.to_owned()];
        for (mach, cp_in) in [(0.5, 44.0), (1.0, 50.0), (1.5, 47.0)] {
            rows.push(rasaero_row(mach, 0.0, 0.0, 0.0, cp_in));
        }
        for (mach, slope, viscous, cp_in) in [
            (0.5, 7.0, 0.0, 44.0),
            (1.0, 10.0, 0.03, 49.0),
            (1.5, 9.0, 0.04, 46.0),
        ] {
            rows.push(rasaero_row(mach, 2.0, slope * r2, viscous, cp_in));
        }
        for (mach, slope, viscous, cp_in) in [(0.5, 7.0, 0.0, 44.0), (1.0, 10.0, 0.12, 48.0)] {
            rows.push(rasaero_row(mach, 4.0, slope * r4, viscous, cp_in));
        }
        rows.join("\r\n")
    }

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        assert!(
            (got - want).abs() <= rel * want.abs().max(1e-300),
            "{what}: {got} against {want}"
        );
    }

    #[test]
    fn reads_a_rasaero_export_by_angle_of_attack() {
        let table = NormalForceTable::from_rasaero_csv(&small_export()).unwrap();
        let (r2, r4) = (2.0_f64.to_radians(), 4.0_f64.to_radians());
        let alphas: Vec<f64> = table.columns().iter().map(|c| c.alpha_rad).collect();
        assert_eq!(alphas, [0.0, r2, r4]);
        assert_eq!(table.columns()[2].slope_per_rad.xs(), [0.5, 1.0]);
        assert_eq!(table.reference(), TableReference::LargestBody);
        // At 0°, the potential slope at 2°; at 2° and 4°, `CN/α` with the viscous part in it.
        close(
            table.columns()[0].slope_per_rad.ys()[1],
            10.0,
            1e-15,
            "0°, Mach 1",
        );
        close(
            table.columns()[1].slope_per_rad.ys()[1],
            10.0 + 0.03 / r2,
            1e-15,
            "2°, Mach 1",
        );
        // Inches from the nose tip, converted exactly.
        let inch = 0.0254;
        assert_eq!(table.columns()[0].cp_station_m.ys()[1], 50.0 * inch);

        let at = |mach, alpha: f64| table.lookup(mach, alpha).unwrap();
        // At a column's own angle and Mach number the export's `CN` and `CP` come back.
        let knot = at(1.0, r4);
        close(knot.coefficient, 10.0 * r4 + 0.12, 1e-15, "CN at 4°");
        assert_eq!(knot.cp_station_m, 48.0 * inch);
        assert_eq!((knot.mach_extrapolated, knot.beyond_alpha), (None, false));
        // Between the columns `C_N/α` and the CP are linear in the angle; with a viscous part in
        // `α²`, `C_N/α` is `10 + 0.03 α/α₂²`, which that reproduces between the columns.
        let s = |alpha: f64| 10.0 + 0.03 * alpha / (r2 * r2);
        let three = at(1.0, 3.0_f64.to_radians());
        close(
            three.slope_per_rad,
            s(3.0_f64.to_radians()),
            1e-14,
            "C_N/α at 3°",
        );
        close(three.cp_station_m, 48.5 * inch, 1e-14, "CP at 3°");
        let one = at(1.0, 1.0_f64.to_radians());
        close(
            one.slope_per_rad,
            s(1.0_f64.to_radians()),
            1e-14,
            "C_N/α at 1°",
        );
        close(one.cp_station_m, 49.5 * inch, 1e-14, "CP at 1°");
        // At zero the slope is the 0° column's.
        let zero = at(1.0, 0.0);
        assert_eq!((zero.coefficient, zero.slope_per_rad), (0.0, 10.0));
        // Between Mach numbers each column is linear in Mach.
        close(at(0.75, 0.0).slope_per_rad, 8.5, 1e-15, "Mach 0.75");

        // Past the last angle: the linear share, 10 per radian at 50 in, grows as sin α; the
        // rest of the 4° column's force (0.12) and moment grows as sin² α.
        let ten_rad = 10.0_f64.to_radians();
        let s1 = ten_rad.sin() / r4.sin();
        let (linear, rest) = (10.0 * r4, 0.12);
        let rest_moment = (linear + rest) * 48.0 * inch - linear * 50.0 * inch;
        let ten = at(1.0, ten_rad);
        let force = linear * s1 + rest * s1 * s1;
        close(ten.coefficient, force, 1e-14, "C_N at 10°");
        close(
            ten.cp_station_m,
            (linear * 50.0 * inch * s1 + rest_moment * s1 * s1) / force,
            1e-14,
            "CP at 10°",
        );
        assert!(ten.beyond_alpha);
        // The viscous share sits forward (36.4 in), so the CP moves forward with the angle.
        assert!(ten.cp_station_m < 48.0 * inch);
        // Continuous where the table ends.
        let just = at(1.0, r4 * (1.0 + 1e-9));
        close(just.coefficient, knot.coefficient, 1e-8, "C_N just past 4°");
        close(
            just.cp_station_m,
            knot.cp_station_m,
            1e-8,
            "CP just past 4°",
        );
        // Tail first there is none: `sin π` rounds to 1.2e-16, times forces of about 1.
        assert!(at(1.0, PI).coefficient.abs() < 1e-14);
        // Past the 4° column's last Mach number its end value holds, and the lookup says so.
        let fast = at(3.0, r4);
        close(fast.coefficient, 10.0 * r4 + 0.12, 1e-15, "Mach 3 at 4°");
        assert_eq!(fast.mach_extrapolated, Some(Side::Above));
        assert_eq!(at(3.0, r2).cp_station_m, 46.0 * inch);
        // Subsonic, where every column is the same, the continuation is the linear share alone.
        let sub = at(0.5, ten_rad);
        close(sub.coefficient, 7.0 * r4 * s1, 1e-14, "Mach 0.5 at 10°");
        close(sub.cp_station_m, 44.0 * inch, 1e-14, "Mach 0.5 CP at 10°");
    }

    #[test]
    fn rasaero_errors_name_the_line() {
        let line = |text: &str| match NormalForceTable::from_rasaero_csv(text) {
            Err(AeroError::Csv { line, .. }) => line,
            other => panic!("expected a CSV error, got {other:?}"),
        };
        let with = |rows: &[String]| format!("{RASAERO_HEADER}\n{}", rows.join("\n"));
        let row = |mach, alpha| rasaero_row(mach, alpha, 0.1 * alpha, 0.0, 60.0);
        assert_eq!(line(""), 0);
        assert_eq!(line("Mach,Alpha,CN,CP\n0.1,2,0.2,60\n"), 1);
        assert_eq!(line(&with(&[row(0.1, 0.0), row(0.2, 0.0)])), 0);
        // A 0° row needs the smallest positive angle's row at its Mach number.
        let missing = [row(0.1, 0.0), row(0.2, 0.0), row(0.1, 2.0), row(0.3, 2.0)];
        assert_eq!(line(&with(&missing)), 3);
        assert_eq!(line(&with(&[row(0.2, 2.0), row(0.1, 2.0)])), 3);
        assert_eq!(line(&with(&[row(0.1, 2.0), row(0.1, 90.0)])), 3);
        assert_eq!(line(&with(&[row(0.1, -2.0)])), 2);
        assert_eq!(line(&with(&[row(0.1, 2.0), "0.2,2,x".to_owned()])), 3);
        // A column the reader doesn't use may hold anything.
        let text_in_unused = row(0.2, 2.0).replacen(",9.9,", ",n/a,", 1);
        let table =
            NormalForceTable::from_rasaero_csv(&with(&[row(0.1, 2.0), text_in_unused])).unwrap();
        assert_eq!(table.columns()[0].slope_per_rad.xs(), [0.1, 0.2]);
        let nan = rasaero_row(0.2, 2.0, f64::NAN, 0.0, 60.0);
        assert_eq!(line(&with(&[row(0.1, 2.0), nan])), 3);
        // An identical repeated row is skipped; one Mach number is too few.
        let table = NormalForceTable::from_rasaero_csv(&with(&[
            row(0.1, 2.0),
            row(0.1, 2.0),
            row(0.2, 2.0),
        ]))
        .unwrap();
        assert_eq!(table.columns()[0].slope_per_rad.xs(), [0.1, 0.2]);
        assert_eq!(
            line(&with(&[row(0.1, 2.0), row(0.2, 2.0), row(0.1, 4.0)])),
            4
        );
    }

    #[test]
    fn normal_force_tables_check_their_columns_and_round_trip() {
        let flat = |value| {
            Table1D::new(
                vec![0.0, 2.0],
                vec![value, value],
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .unwrap()
        };
        let column = |alpha| NormalForceColumn::new(alpha, flat(6.0), flat(1.5));
        let domain = |result: Result<NormalForceTable, AeroError>| {
            assert!(
                matches!(result, Err(AeroError::Domain { .. })),
                "{result:?}"
            );
        };
        domain(NormalForceTable::new(Vec::new()));
        domain(NormalForceTable::new(vec![column(0.1), column(0.1)]));
        domain(NormalForceTable::new(vec![column(0.1), column(0.05)]));
        domain(NormalForceTable::new(vec![column(-0.1)]));
        domain(NormalForceTable::new(vec![column(FRAC_PI_2)]));
        domain(NormalForceTable::new(vec![column(f64::NAN)]));
        let table = NormalForceTable::new(vec![column(0.0), column(0.1)]).unwrap();
        domain(table.clone().with_reference_diameter_m(0.0));
        domain(table.clone().with_reference_diameter_m(f64::INFINITY));
        let table = table.with_reference_diameter_m(0.1).unwrap();
        assert_eq!(
            table.reference(),
            TableReference::Diameter { diameter_m: 0.1 }
        );
        let json = serde_json::to_string(&table).unwrap();
        assert_eq!(
            serde_json::from_str::<NormalForceTable>(&json).unwrap(),
            table
        );
        // Reading re-checks the columns.
        let reversed = json.replacen("\"alpha_rad\":0.1", "\"alpha_rad\":-0.1", 1);
        assert!(serde_json::from_str::<NormalForceTable>(&reversed).is_err());
        for (mach, alpha) in [(-0.1, 0.0), (f64::NAN, 0.0), (0.5, -0.01), (0.5, 3.2)] {
            assert!(matches!(
                table.lookup(mach, alpha),
                Err(AeroError::Domain { .. })
            ));
        }
        // A single column at 0° follows `sin α` at every angle.
        let single = NormalForceTable::new(vec![column(0.0)]).unwrap();
        let lookup = single.lookup(1.0, 0.3).unwrap();
        assert_eq!(lookup.coefficient, 6.0 * 0.3_f64.sin());
        assert!(lookup.beyond_alpha);
        // Mach 7 is fine for a table.
        assert_eq!(single.lookup(7.0, 0.0).unwrap().slope_per_rad, 6.0);
    }

    fn flat(value: f64) -> Table1D {
        Table1D::new(
            vec![0.0, 2.0],
            vec![value, value],
            Interpolation::Linear,
            Extrapolation::Clamp,
        )
        .unwrap()
    }

    /// A table shaped as RASAero II's export is (Calisto's: the potential part exactly linear in
    /// the angle, the viscous part exactly as `sin² α`), sampled at 0°, 2° and 4°: past 4° the
    /// viscous part continues exactly, and the potential part as `α₄ sin α / sin α₄`.
    #[test]
    fn a_rasaero_shaped_table_continues_its_crossflow_exactly() {
        let (a, x_a, b, x_b) = (9.0, 1.6, 4.0, 1.1);
        let (r2, r4) = (2.0_f64.to_radians(), 4.0_f64.to_radians());
        let force = |alpha: f64| a * alpha + b * alpha.sin().powi(2);
        let cp = |alpha: f64| (a * alpha * x_a + b * alpha.sin().powi(2) * x_b) / force(alpha);
        let column =
            |alpha: f64, slope, centre| NormalForceColumn::new(alpha, flat(slope), flat(centre));
        let table = NormalForceTable::new(vec![
            column(0.0, a, x_a),
            column(r2, force(r2) / r2, cp(r2)),
            column(r4, force(r4) / r4, cp(r4)),
        ])
        .unwrap();
        for alpha_deg in [10.0_f64, 30.0, 60.0, 90.0, 150.0] {
            let alpha = alpha_deg.to_radians();
            let s1 = alpha.sin() / r4.sin();
            let (linear, viscous) = (a * r4 * s1, b * alpha.sin().powi(2));
            let lookup = table.lookup(1.0, alpha).unwrap();
            close(lookup.coefficient, linear + viscous, 1e-12, "C_N");
            close(
                lookup.cp_station_m,
                (linear * x_a + viscous * x_b) / (linear + viscous),
                1e-12,
                "CP",
            );
        }
    }

    /// The case physics review found jumping in the first guarded split: as the 4° column's
    /// centre of pressure moves past the one that makes the rest's moment zero, near Mach 0.51,
    /// `C_N` at 30° fell from 8.59 to 5.50. Now it moves smoothly.
    #[test]
    fn the_continuation_does_not_jump_where_the_rest_s_moment_changes_sign() {
        let r4 = 4.0_f64.to_radians();
        let line = |a: f64, b: f64| {
            Table1D::new(
                vec![0.0, 1.0],
                vec![a, b],
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .unwrap()
        };
        let table = NormalForceTable::new(vec![
            NormalForceColumn::new(0.0, flat(10.0), flat(1.0)),
            NormalForceColumn::new(r4, flat(11.0), line(0.92, 0.90)),
        ])
        .unwrap();
        let alpha = 30.0_f64.to_radians();
        let at = |mach: f64| table.lookup_within(mach, alpha, (0.0, 1.3)).unwrap();
        let mut previous = at(0.4);
        for step in 1..=2000 {
            let next = at(0.4 + f64::from(step) * 1e-4);
            assert!(
                (next.coefficient - previous.coefficient).abs() < 1e-3,
                "{previous:?} {next:?}"
            );
            let moment = |l: &NormalForceLookup| l.coefficient * l.cp_station_m;
            assert!((moment(&next) - moment(&previous)).abs() < 1e-3);
            previous = next;
        }
        // Stations out of order, or not finite, are refused rather than panicking.
        for stations in [(1.3, 0.0), (f64::NAN, 1.3), (0.0, f64::INFINITY)] {
            assert!(matches!(
                table.lookup_within(0.5, alpha, stations),
                Err(AeroError::Domain { .. })
            ));
        }
    }

    proptest::proptest! {
        /// Past the last column the normal force never turns round, and while `s ≥ 1` its centre
        /// of pressure stays within the stations given, whatever the table: slopes that grow or
        /// fall with the angle, and centres of pressure that move either way.
        #[test]
        fn the_continuation_keeps_its_sign_and_centre(
            slope_0 in 0.5..20.0_f64,
            slope_n in 0.5..20.0_f64,
            cp_0 in 0.1..3.0_f64,
            cp_n in 0.1..3.0_f64,
            alpha_n_deg in 1.0..40.0_f64,
            beyond in 0.0..1.0_f64,
        ) {
            let alpha_n = alpha_n_deg.to_radians();
            let table = NormalForceTable::new(vec![
                NormalForceColumn::new(0.0, flat(slope_0), flat(cp_0)),
                NormalForceColumn::new(alpha_n, flat(slope_n), flat(cp_n)),
            ])
            .unwrap();
            let alpha = alpha_n + beyond * (PI - alpha_n);
            let lookup = table.lookup_within(1.0, alpha, (0.0, 3.0)).unwrap();
            // `sin π` rounds to 1.2e-16.
            proptest::prop_assert!(lookup.coefficient >= -1e-13, "{lookup:?}");
            if alpha.sin() >= alpha_n.sin() {
                proptest::prop_assert!(
                    (-1e-9..=3.0 + 1e-9).contains(&lookup.cp_station_m),
                    "{lookup:?}"
                );
            }
        }

        /// The continuation is continuous in the table's values: as a column's slope or centre
        /// of pressure moves with Mach number, through the points where the rest beyond the
        /// linear share, or its moment, changes sign, the force and its moment move smoothly.
        #[test]
        fn the_continuation_is_continuous_in_mach(
            slope_0 in 5.0..15.0_f64,
            cp_0 in 0.8..1.2_f64,
            slope_change in -0.2..0.2_f64,
            cp_change in -0.1..0.1_f64,
            cp_offset in -0.1..0.1_f64,
            alpha_deg in 5.0..170.0_f64,
        ) {
            let r4 = 4.0_f64.to_radians();
            let line = |a: f64, b: f64| {
                Table1D::new(vec![0.0, 1.0], vec![a, b], Interpolation::Linear, Extrapolation::Clamp)
                    .unwrap()
            };
            let table = NormalForceTable::new(vec![
                NormalForceColumn::new(0.0, flat(slope_0), flat(cp_0)),
                NormalForceColumn::new(
                    r4,
                    line(slope_0 - slope_change, slope_0 + slope_change),
                    line(cp_0 + cp_offset - cp_change, cp_0 + cp_offset + cp_change),
                ),
            ])
            .unwrap();
            let alpha = alpha_deg.to_radians();
            let at = |mach: f64| {
                let l = table.lookup_within(mach, alpha, (0.0, 1.5)).unwrap();
                (l.coefficient, l.coefficient * l.cp_station_m)
            };
            let mut previous = at(0.0);
            for step in 1..=1000 {
                let next = at(f64::from(step) * 1e-3);
                // The force is at most about 1/sin²(4°) ≈ 200 times a column's; a step of 1e-3
                // in Mach moves it by far less than 1% of that.
                proptest::prop_assert!((next.0 - previous.0).abs() < 0.05, "{previous:?} {next:?}");
                proptest::prop_assert!((next.1 - previous.1).abs() < 0.1, "{previous:?} {next:?}");
                previous = next;
            }
        }
    }
}
