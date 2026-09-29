//! Converting motor files between RASP `.eng` and RockSim `.rse`.
//!
//! Both formats give a motor's name, maker, casing, masses, delays and thrust curve; the rules
//! for each are in `docs/format/eng.md` and `docs/format/rse.md`. What differs is how they say
//! it, and what `.rse` adds:
//!
//! | | `.eng` | `.rse` |
//! |---|---|---|
//! | masses | kg | g |
//! | delays | `6-10-14`, `P` for plugged | `6,10,14`, `1000` for plugged |
//! | the curve's `(0, 0)` origin | implicit, "should not be specified" | written out, in every observed file |
//! | comments | `;` lines before the header | one `<comments>` text |
//! | type, stated figures, `m` and `cg` per point | none | `Type`, `Itot`, `avgThrust`, `peakThrust`, `burn-time`, `massFrac`, `Isp`, … |
//!
//! [`eng_to_rse`] fills what `.eng` lacks the way the observed `.rse` files do (the rse page's
//! writer policy): a `(0, 0)` origin when the curve starts after ignition; `Itot` by the
//! trapezoid rule; `peakThrust`; `burn-time`, the last time; `avgThrust = Itot / burn-time`;
//! `m`, the propellant left, `m(t) = m₀ (1 − I(t) / Itot)` with `m₀` the propellant mass and `I`
//! the trapezoidal impulse; `cg`, half the length; both auto-calc flags `1`; `massFrac =
//! 100 m₀ / initWt`; `Isp = Itot / (m₀ g₀)`; and `Type="unspecified"`, which RockSim's guide
//! requires and the files use when they don't say.
//!
//! [`rse_to_eng`] drops what `.eng` can't hold, each named in a [`ConvertWarning`]. hpr uses
//! none of it for a solid motor: it works the mass and centre of gravity out from the curve and
//! the masses ([`SolidMotor::from_envelope`](crate::SolidMotor::from_envelope)). It refuses a
//! hybrid, whose `Type` a `.eng` file couldn't keep, and an engine without delays it can read,
//! since a `.eng` header must give them. A `.eng` header is seven fields split on spaces, and
//! OpenRocket refuses more, so a name or a maker of several words is written with `_` between
//! them (`Estes_Industries,_Inc.`).
//!
//! **Round trips.** Masses move between kg and g by moving the decimal point in their shortest
//! digits, not by multiplying, so a mass of at most 15 significant digits, in the normal range
//! of a double, comes back bit for bit. One of 16 or 17 digits may not (2 of the 29 bundled
//! `.eng` files write one, such as `0.0036000000000000003`), and a warning says so.
//!
//! The thrust curve hpr flies comes back bit for bit both ways, and so do the diameter, the
//! length, and the masses above. The rest may come back written differently:
//!
//! - delays spelled with commas, spaces, `p` or `1000` come back in the other spelling, and read
//!   as the same delays ([`DelayList`]);
//! - a name or maker of several words comes back with `_` between them, said in a warning;
//! - a `(0, 0)` origin is written in `.rse` and left out of `.eng`, so a curve that gives it the
//!   other way comes back the usual way;
//! - comments lose blank lines and the spaces ending a line, said in a warning; comments after a
//!   `.eng` file's last motor are dropped, said in a warning;
//! - the `.rse` figures `.eng` can't hold are dropped, said in a warning, and filled again by the
//!   rules above.
//!
//! After one conversion, a file converts to the other format and back to the same bytes, but for
//! a `.eng` motor whose delays name none (`-`): its `.rse` file leaves them out, and a `.eng`
//! header must give them, so converting it back needs them given again.

use hpr_core::gravity::STANDARD_GRAVITY_MPS2;
use serde::{Deserialize, Serialize};

use crate::delay::DelayList;
use crate::eng::{EngEntry, EngFile};
use crate::error::MotorError;
use crate::rse::{RseEngine, RseFile, RsePoint};
use crate::text::WarningKind;

/// The `Type` [`eng_to_rse`] writes: a `.eng` file doesn't say.
pub const UNSPECIFIED_TYPE: &str = "unspecified";

/// The delay a `.rse` file writes for a plugged motor, where `.eng` writes `P`.
pub const RSE_PLUGGED: &str = "1000";

/// A converted file and what the conversion said.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Converted<T> {
    /// The file, in the other format.
    pub value: T,
    /// What was dropped or changed on the way, in motor order.
    pub warnings: Vec<ConvertWarning>,
}

/// Something the other format couldn't hold as it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvertWarning {
    /// The motor it is about: its name or code.
    pub motor: String,
    /// [`WarningKind::Dropped`] for a value the other format has no place for;
    /// [`WarningKind::Unusual`] for one written differently, such as a name with spaces.
    pub kind: WarningKind,
    /// What happened, for a person.
    pub message: String,
}

impl ConvertWarning {
    fn new(motor: &str, kind: WarningKind, message: impl Into<String>) -> Self {
        Self {
            motor: motor.to_owned(),
            kind,
            message: message.into(),
        }
    }
}

/// A `.eng` file as a `.rse` file, filled the way RockSim's files are (see the module's docs).
///
/// # Errors
///
/// What [`EngEntry::thrust_curve`] refuses: a curve with negative thrust, decreasing time or
/// no impulse, whose `Itot` and `m` can't be worked out. [`MotorError::Domain`] for a mass that
/// has no value in grams, or one that makes a filled figure infinite, such as a propellant mass
/// so small that the specific impulse overflows.
pub fn eng_to_rse(file: &EngFile) -> Result<Converted<RseFile>, MotorError> {
    let mut warnings = Vec::new();
    let engines = file
        .entries
        .iter()
        .map(|entry| rse_engine(entry, &mut warnings))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(last) = file.entries.last()
        && !file.trailing_comments.is_empty()
    {
        warnings.push(ConvertWarning::new(
            &last.name,
            WarningKind::Dropped,
            format!(
                "{} comment line(s) after the last motor: a .rse file has no place for them",
                file.trailing_comments.len()
            ),
        ));
    }
    Ok(Converted {
        value: RseFile { engines },
        warnings,
    })
}

fn rse_engine(
    entry: &EngEntry,
    warnings: &mut Vec<ConvertWarning>,
) -> Result<RseEngine, MotorError> {
    let curve = entry.thrust_curve()?;
    let total_impulse_ns = curve.total_impulse_ns();
    let burn_time_s = curve.end_time_s();
    let initial_mass_g = scaled(&entry.name, entry.total_mass_kg, Unit::Kg, warnings)?;
    let propellant_mass_g = scaled(&entry.name, entry.propellant_mass_kg, Unit::Kg, warnings)?;
    let cg_mm = entry.length_mm / 2.0;
    let mut points = entry.points.clone();
    // The origin `ThrustCurve::new` adds, written out.
    if points.first().is_some_and(|&(time_s, _)| time_s > 0.0) {
        points.insert(0, (0.0, 0.0));
    }
    // The impulse summed as `ThrustCurve::new` sums it, over the same points, so the last `m`
    // is exactly zero.
    let mut impulse_ns = 0.0;
    let mut previous: Option<(f64, f64)> = None;
    let points = points
        .into_iter()
        .map(|(time_s, thrust_n)| {
            if let Some((t0, f0)) = previous {
                impulse_ns += 0.5 * (f0 + thrust_n) * (time_s - t0);
            }
            previous = Some((time_s, thrust_n));
            RsePoint {
                time_s,
                thrust_n,
                mass_g: Some(propellant_mass_g * (1.0 - impulse_ns / total_impulse_ns)),
                cg_mm: Some(cg_mm),
            }
        })
        .collect();
    let positive = |value: f64| (value > 0.0).then_some(value);
    let delays = if DelayList::parse(&entry.delays).delays.is_empty() {
        warnings.push(ConvertWarning::new(
            &entry.name,
            WarningKind::Dropped,
            format!(
                "dropped the delays {:?}, which name no delay; a .rse file may leave them out",
                entry.delays
            ),
        ));
        None
    } else {
        Some(delays_to_rse(&entry.delays))
    };
    let engine = RseEngine {
        manufacturer: entry.manufacturer.clone(),
        code: entry.name.clone(),
        motor_type: Some(UNSPECIFIED_TYPE.to_owned()),
        diameter_mm: entry.diameter_mm,
        length_mm: entry.length_mm,
        initial_mass_g,
        propellant_mass_g,
        delays,
        auto_calc_mass: Some(true),
        auto_calc_cg: Some(true),
        // `ThrustCurve::new` refuses a curve with no impulse or no burn time.
        average_thrust_n: Some(total_impulse_ns / burn_time_s),
        peak_thrust_n: Some(curve.peak_thrust_n()),
        throat_diameter_mm: None,
        exit_diameter_mm: None,
        total_impulse_ns: Some(total_impulse_ns),
        burn_time_s: Some(burn_time_s),
        // A motor file may give no propellant or no mass; then there is no fraction and no
        // specific impulse to state.
        mass_fraction_pct: positive(initial_mass_g)
            .map(|initial| 100.0 * propellant_mass_g / initial),
        isp_s: positive(entry.propellant_mass_kg)
            .map(|propellant| total_impulse_ns / (propellant * STANDARD_GRAVITY_MPS2)),
        comments: (!entry.comments.is_empty()).then(|| entry.comments.join("\n")),
        points,
    };
    // A mass near a double's limits can make a figure infinite, which no file can hold.
    for (what, value) in [
        ("average thrust (N)", engine.average_thrust_n),
        ("mass fraction (%)", engine.mass_fraction_pct),
        ("specific impulse (s)", engine.isp_s),
    ] {
        if let Some(value) = value
            && !value.is_finite()
        {
            return Err(MotorError::Domain { what, value });
        }
    }
    Ok(engine)
}

/// A `.rse` file as a `.eng` file, with what `.eng` can't hold named in the warnings.
///
/// # Errors
///
/// [`MotorError::Inconsistent`] for an engine without delays it can read (none, or none
/// [`DelayList::parse`](crate::DelayList::parse) finds), which a `.eng` header must give, or a
/// hybrid, which a `.eng` file couldn't say it is; [`MotorError::Domain`] for a mass that has no
/// value in kg (see the module's docs).
pub fn rse_to_eng(file: &RseFile) -> Result<Converted<EngFile>, MotorError> {
    let mut warnings = Vec::new();
    let entries = file
        .engines
        .iter()
        .map(|engine| eng_entry(engine, &mut warnings))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Converted {
        value: EngFile {
            entries,
            trailing_comments: Vec::new(),
        },
        warnings,
    })
}

fn eng_entry(
    engine: &RseEngine,
    warnings: &mut Vec<ConvertWarning>,
) -> Result<EngEntry, MotorError> {
    let code = &engine.code;
    if engine
        .motor_type
        .as_deref()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("hybrid"))
    {
        return Err(MotorError::Inconsistent(format!(
            "{code} is a hybrid, which a .eng file can't say; hpr models solid motors only"
        )));
    }
    let delays = engine
        .delays
        .as_deref()
        .filter(|delays| !DelayList::parse(delays).delays.is_empty())
        .ok_or_else(|| {
            MotorError::Inconsistent(format!(
                "the .rse engine {code:?} gives no delays, and a .eng header must"
            ))
        })?;
    // A `.eng` header is seven fields split on spaces. hpr reads the maker as the rest of the
    // line, but OpenRocket refuses a header of more than seven fields, so each is one word.
    let name = code.split_whitespace().collect::<Vec<_>>().join("_");
    if name != *code {
        warnings.push(ConvertWarning::new(
            code,
            WarningKind::Unusual,
            format!("a .eng name is one word, so {code:?} is written {name:?}"),
        ));
    }
    let manufacturer = engine
        .manufacturer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_");
    if manufacturer != engine.manufacturer {
        warnings.push(ConvertWarning::new(
            code,
            WarningKind::Unusual,
            format!(
                "a .eng maker is one word, so {:?} is written {manufacturer:?}",
                engine.manufacturer
            ),
        ));
    }
    let comments = match &engine.comments {
        Some(text) => {
            let lines: Vec<String> = text
                .lines()
                .map(str::trim_end)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
            if lines.join("\n") != *text {
                warnings.push(ConvertWarning::new(
                    code,
                    WarningKind::Dropped,
                    "dropped the comments' blank lines and the spaces that end their lines: a \
                     .eng comment is one line of text",
                ));
            }
            lines
        }
        None => Vec::new(),
    };
    let mut points: Vec<(f64, f64)> = engine
        .points
        .iter()
        .map(|point| (point.time_s, point.thrust_n))
        .collect();
    // `.eng` leaves the origin implicit; `ThrustCurve::new` puts it back exactly when the next
    // point is after ignition. A `-0` stays written, as it wouldn't come back.
    if let [(t0, f0), (t1, _), ..] = points[..]
        && t0.to_bits() == 0
        && f0.to_bits() == 0
        && t1 > 0.0
    {
        points.remove(0);
    }
    let dropped = dropped(engine);
    if !dropped.is_empty() {
        warnings.push(ConvertWarning::new(
            code,
            WarningKind::Dropped,
            format!(
                "dropped {}: a .eng file has no place for them. hpr doesn't use them for a solid \
                 motor: it works the mass and centre of gravity out from the curve and the masses",
                dropped.join(", ")
            ),
        ));
    }
    Ok(EngEntry {
        comments,
        name,
        diameter_mm: engine.diameter_mm,
        length_mm: engine.length_mm,
        delays: delays_to_eng(delays),
        propellant_mass_kg: scaled(code, engine.propellant_mass_g, Unit::G, warnings)?,
        total_mass_kg: scaled(code, engine.initial_mass_g, Unit::G, warnings)?,
        manufacturer,
        points,
    })
}

/// The `.rse` values of `engine` a `.eng` file has no place for, by their attribute names.
fn dropped(engine: &RseEngine) -> Vec<&'static str> {
    let flags = [
        ("Type", engine.motor_type.is_some()),
        ("auto-calc-mass", engine.auto_calc_mass.is_some()),
        ("auto-calc-cg", engine.auto_calc_cg.is_some()),
        ("avgThrust", engine.average_thrust_n.is_some()),
        ("peakThrust", engine.peak_thrust_n.is_some()),
        ("throatDia", engine.throat_diameter_mm.is_some()),
        ("exitDia", engine.exit_diameter_mm.is_some()),
        ("Itot", engine.total_impulse_ns.is_some()),
        ("burn-time", engine.burn_time_s.is_some()),
        ("massFrac", engine.mass_fraction_pct.is_some()),
        ("Isp", engine.isp_s.is_some()),
        (
            "m",
            engine.points.iter().any(|point| point.mass_g.is_some()),
        ),
        (
            "cg",
            engine.points.iter().any(|point| point.cg_mm.is_some()),
        ),
    ];
    flags
        .into_iter()
        .filter_map(|(name, present)| present.then_some(name))
        .collect()
}

/// `.eng` delays as `.rse` writes them: commas, and `1000` for plugged.
fn delays_to_rse(delays: &str) -> String {
    delays
        .split(['-', ','])
        .filter(|piece| !piece.is_empty())
        .map(|piece| {
            if piece.eq_ignore_ascii_case("P") {
                RSE_PLUGGED
            } else {
                piece
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// `.rse` delays as `.eng` writes them: dashes, and `P` for plugged.
fn delays_to_eng(delays: &str) -> String {
    delays
        .split([',', '-'])
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(|piece| if piece == RSE_PLUGGED { "P" } else { piece })
        .collect::<Vec<_>>()
        .join("-")
}

/// The unit a mass is given in.
#[derive(Clone, Copy)]
enum Unit {
    Kg,
    G,
}

/// A mass in the other format's unit, with a warning if it wouldn't come back bit for bit: a
/// value of 16 or 17 digits, such as a file's `7.5120000000000005` kg, can fall between the
/// doubles the other unit reaches.
///
/// # Errors
///
/// [`MotorError::Domain`] for a mass whose value in the other unit is infinite, or zero when the
/// mass isn't: a double's limits, which no motor comes near.
fn scaled(
    motor: &str,
    mass: f64,
    unit: Unit,
    warnings: &mut Vec<ConvertWarning>,
) -> Result<f64, MotorError> {
    let (there, back, from, to): (f64, fn(f64) -> f64, _, _) = match unit {
        Unit::Kg => (kg_to_g(mass), g_to_kg, "kg", "g"),
        Unit::G => (g_to_kg(mass), kg_to_g, "g", "kg"),
    };
    if !there.is_finite() || (there == 0.0 && mass != 0.0) {
        return Err(MotorError::Domain {
            what: match unit {
                Unit::Kg => "mass (kg), which has no value in g",
                Unit::G => "mass (g), which has no value in kg",
            },
            value: mass,
        });
    }
    let returned = back(there);
    if returned.to_bits() != mass.to_bits() {
        warnings.push(ConvertWarning::new(
            motor,
            WarningKind::Unusual,
            format!(
                "{mass} {from} has more digits than a value in {to} keeps: it is written \
                 {there} {to}, which reads back as {returned} {from}"
            ),
        ));
    }
    Ok(there)
}

/// Kilograms to grams, by moving the decimal point in the mass's shortest digits rather than
/// multiplying: `0.0041` kg is `4.1` g, where `0.0041 × 1000` is `4.1000000000000005`.
pub fn kg_to_g(kg: f64) -> f64 {
    shift(kg, 3)
}

/// Grams to kilograms, by moving the decimal point as [`kg_to_g`] does.
pub fn g_to_kg(g: f64) -> f64 {
    shift(g, -3)
}

/// `value × 10^places`, taken by moving the decimal point in `value`'s shortest digits (Rust's
/// `{:e}`) and reading the result.
///
/// Multiplying by 1000 would round in binary: `0.0041 × 1000` is `4.1000000000000005`, as it is
/// for about a quarter of the masses with four decimals. Moving the point gives the double
/// nearest `4.1`, which a file writes as `4.1`, and moving it back gives `0.0041` again whenever
/// the digits number at most 15 and both values are normal doubles, since two decimals of up to
/// 15 significant digits never share a double.
fn shift(value: f64, places: i32) -> f64 {
    let text = format!("{value:e}");
    text.split_once('e')
        .and_then(|(digits, exponent)| {
            let exponent: i32 = exponent.parse().ok()?;
            format!("{digits}e{}", exponent + places).parse().ok()
        })
        // `{:e}` of a finite f64 always reads back; a non-finite one, which no writer accepts,
        // keeps the product so that the writer refuses it by its value.
        .unwrap_or(value * 10f64.powi(places))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::{DelayList, eng, rse};

    #[test]
    fn a_decimal_shift_reads_as_the_file_writes() {
        assert_eq!(0.0041_f64 * 1000.0, 4.100_000_000_000_000_5);
        assert_eq!(kg_to_g(0.0041), 4.1);
        assert_eq!(g_to_kg(4.1), 0.0041);
        assert_eq!(kg_to_g(0.0773), 77.3);
        assert_eq!(g_to_kg(77.3), 0.0773);
        assert_eq!(kg_to_g(0.0).to_bits(), 0);
        assert_eq!(kg_to_g(1.5e-5), 0.015);
        assert_eq!(g_to_kg(182.5), 0.1825);
        assert!(kg_to_g(f64::NAN).is_nan());
        assert_eq!(kg_to_g(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn delays_trade_dashes_and_p_for_commas_and_1000() {
        assert_eq!(delays_to_rse("4-5-7-8-9"), "4,5,7,8,9");
        assert_eq!(delays_to_rse("P"), "1000");
        assert_eq!(delays_to_rse("6-10-P"), "6,10,1000");
        assert_eq!(delays_to_rse("5,8,11"), "5,8,11");
        assert_eq!(delays_to_eng("10,14,1000"), "10-14-P");
        assert_eq!(delays_to_eng("15, 12,10"), "15-12-10");
        assert_eq!(delays_to_eng("0"), "0");
        for eng in ["4-5-7-8-9", "P", "6-10-P", "0", "5,8,11", "1000", "100"] {
            let there_and_back = delays_to_eng(&delays_to_rse(eng));
            assert_eq!(
                DelayList::parse(&there_and_back),
                DelayList::parse(eng),
                "{eng}"
            );
        }
    }

    const ENG: &str = "; a comment\n; another\nH54 38 152 6-10-P 0.0773 0.172 Test_Maker\n   0.05 80.25\n   0.4 60\n   1.2 0\n;\n";

    #[test]
    fn eng_to_rse_fills_what_rocksim_files_give() {
        let eng = eng::parse(ENG).unwrap().value;
        let rse = eng_to_rse(&eng).unwrap();
        assert!(rse.warnings.is_empty(), "{:?}", rse.warnings);
        let [engine] = &rse.value.engines[..] else {
            panic!("one engine");
        };
        assert_eq!(engine.code, "H54");
        assert_eq!(engine.manufacturer, "Test_Maker");
        assert_eq!(engine.motor_type.as_deref(), Some("unspecified"));
        assert_eq!(engine.initial_mass_g, 172.0);
        assert_eq!(engine.propellant_mass_g, 77.3);
        assert_eq!(engine.delays.as_deref(), Some("6,10,1000"));
        assert_eq!(engine.comments.as_deref(), Some(" a comment\n another"));
        let times: Vec<f64> = engine.points.iter().map(|p| p.time_s).collect();
        assert_eq!(times, [0.0, 0.05, 0.4, 1.2]);
        // By hand: 0.5·80.25·0.05 + 0.5·(80.25 + 60)·0.35 + 0.5·60·0.8.
        let itot = 2.006_25 + 24.543_75 + 24.0;
        assert!((engine.total_impulse_ns.unwrap() - itot).abs() < 1e-12);
        assert_eq!(engine.peak_thrust_n, Some(80.25));
        assert_eq!(engine.burn_time_s, Some(1.2));
        assert!((engine.average_thrust_n.unwrap() - itot / 1.2).abs() < 1e-12);
        let masses: Vec<f64> = engine.points.iter().map(|p| p.mass_g.unwrap()).collect();
        assert_eq!(masses[0], 77.3);
        assert!((masses[1] - 77.3 * (1.0 - 2.006_25 / itot)).abs() < 1e-12);
        assert_eq!(masses[3], 0.0);
        assert!(engine.points.iter().all(|p| p.cg_mm == Some(76.0)));
        assert!((engine.mass_fraction_pct.unwrap() - 100.0 * 77.3 / 172.0).abs() < 1e-12);
        assert!((engine.isp_s.unwrap() - itot / (0.0773 * 9.806_65)).abs() < 1e-9);
        // What it writes reads back as it was written, and without the reader's complaints.
        let text = rse::write(&rse.value).unwrap();
        let read = rse::parse(&text).unwrap();
        assert!(read.warnings.is_empty(), "{:?}", read.warnings);
        assert_eq!(read.value, rse.value);
    }

    #[test]
    fn a_round_trip_through_rse_gives_the_eng_back() {
        let eng = eng::parse(ENG).unwrap().value;
        let rse = eng_to_rse(&eng).unwrap().value;
        let back = rse_to_eng(&rse).unwrap();
        assert_eq!(back.value, eng);
        assert_eq!(back.warnings.len(), 1);
        assert_eq!(back.warnings[0].kind, WarningKind::Dropped);
        assert!(
            back.warnings[0].message.starts_with(
                "dropped Type, auto-calc-mass, auto-calc-cg, avgThrust, peakThrust, Itot, burn-time, \
                 massFrac, Isp, m, cg:"
            ),
            "{}",
            back.warnings[0].message
        );
    }

    const RSE: &str = r#"<engine-database>
  <engine-list>
    <engine mfg=" Some  Maker" code="Micro Maxx II" Type="reloadable" dia="38." len="191." initWt="330." propWt="182.5" delays="2,4,6" auto-calc-mass="1" auto-calc-cg="1" avgThrust="148.737" peakThrust="205.821" throatDia="0." exitDia="0." Itot="318." burn-time="2.14" massFrac="55.3" Isp="177.68">
      <comments>
      Line one
      Line two   </comments>
      <data>
        <eng-data t="0." f="0." m="182.5" cg="95.5"/>
        <eng-data t="0.1" f="205.821" m="170." cg="95.5"/>
        <eng-data t="2.14" f="0." m="0." cg="95.5"/>
      </data>
    </engine>
  </engine-list>
</engine-database>
"#;

    #[test]
    fn rse_to_eng_names_what_it_drops_and_changes() {
        let rse = rse::parse(RSE).unwrap().value;
        let eng = rse_to_eng(&rse).unwrap();
        let [entry] = &eng.value.entries[..] else {
            panic!("one entry");
        };
        assert_eq!(entry.name, "Micro_Maxx_II");
        assert_eq!(entry.manufacturer, "Some_Maker");
        assert_eq!(entry.delays, "2-4-6");
        assert_eq!(entry.total_mass_kg, 0.33);
        assert_eq!(entry.propellant_mass_kg, 0.1825);
        assert_eq!(entry.comments, ["      Line one", "      Line two"]);
        assert_eq!(entry.points, [(0.1, 205.821), (2.14, 0.0)]);
        let kinds: Vec<(WarningKind, &str)> = eng
            .warnings
            .iter()
            .map(|w| (w.kind, &w.message[..12]))
            .collect();
        assert_eq!(
            kinds,
            [
                (WarningKind::Unusual, "a .eng name "),
                (WarningKind::Unusual, "a .eng maker"),
                (WarningKind::Dropped, "dropped the "),
                (WarningKind::Dropped, "dropped Type"),
            ]
        );
        assert!(eng.warnings[3].message.starts_with(
            "dropped Type, auto-calc-mass, auto-calc-cg, avgThrust, peakThrust, throatDia, exitDia, Itot, \
             burn-time, massFrac, Isp, m, cg:"
        ));
        // The curve hpr flies is the same.
        assert_eq!(
            entry.thrust_curve().unwrap(),
            rse.engines[0].thrust_curve().unwrap()
        );
        // And written, it reads back unchanged.
        let text = eng::write(&eng.value).unwrap();
        assert_eq!(eng::parse(&text).unwrap().value, eng.value);
    }

    #[test]
    fn a_round_trip_through_eng_keeps_what_both_formats_carry() {
        let rse = rse::parse(RSE).unwrap().value;
        let there = rse_to_eng(&rse).unwrap().value;
        let back = eng_to_rse(&there).unwrap().value;
        let (a, b) = (&rse.engines[0], &back.engines[0]);
        assert_eq!(
            (
                a.diameter_mm,
                a.length_mm,
                a.initial_mass_g,
                a.propellant_mass_g
            ),
            (
                b.diameter_mm,
                b.length_mm,
                b.initial_mass_g,
                b.propellant_mass_g
            )
        );
        assert_eq!(a.delays, b.delays);
        let curve = |e: &RseEngine| -> Vec<(f64, f64)> {
            e.points.iter().map(|p| (p.time_s, p.thrust_n)).collect()
        };
        assert_eq!(curve(a), curve(b));
        // Once converted, a file comes back bit for bit.
        assert_eq!(rse_to_eng(&back).unwrap().value, there);
    }

    #[test]
    fn a_hybrid_or_an_engine_without_delays_is_refused() {
        let mut rse = rse::parse(RSE).unwrap().value;
        rse.engines[0].motor_type = Some("Hybrid".to_owned());
        let error = rse_to_eng(&rse).unwrap_err().to_string();
        assert!(error.contains("is a hybrid"), "{error}");
        rse.engines[0].motor_type = None;
        rse.engines[0].delays = None;
        let error = rse_to_eng(&rse).unwrap_err().to_string();
        assert!(error.contains("gives no delays"), "{error}");
    }

    #[test]
    fn a_written_origin_and_trailing_comments_are_said() {
        let text = "X1 29 100 P 0.01 0.05 M\n 0 0\n 0.5 10\n 1 0\n; after\n";
        let eng = eng::parse(text).unwrap().value;
        assert_eq!(eng.trailing_comments, [" after"]);
        let rse = eng_to_rse(&eng).unwrap();
        assert_eq!(rse.value.engines[0].points.len(), 3, "no second origin");
        assert_eq!(rse.warnings.len(), 1);
        assert!(
            rse.warnings[0]
                .message
                .starts_with("1 comment line(s) after")
        );
        // The origin comes back implicit: the same curve, one point fewer.
        let back = rse_to_eng(&rse.value).unwrap().value;
        assert_eq!(back.entries[0].points, [(0.5, 10.0), (1.0, 0.0)]);
        assert_eq!(
            back.entries[0].thrust_curve().unwrap(),
            eng.entries[0].thrust_curve().unwrap()
        );
        // A first point at ignition with thrust, or a `-0` origin, is kept.
        let mut rse = rse.value;
        rse.engines[0].points[0].thrust_n = -0.0;
        let back = rse_to_eng(&rse).unwrap().value;
        assert_eq!(back.entries[0].points.len(), 3);
    }

    /// Every bundled curve converts, writes, reads back as written, and comes back from the other
    /// format: a `.eng` file bit for bit, a `.rse` file in what both formats carry.
    #[test]
    fn every_bundled_curve_round_trips() {
        let (mut engs, mut rses) = (0, 0);
        let mut respelled = Vec::new();
        let mut said = Vec::new();
        let mut makers = Vec::new();
        let mut moved = Vec::new();
        for (name, text) in crate::bundled::CURVE_FILES {
            if name.ends_with(".eng") {
                engs += 1;
                let eng = eng::parse(text).unwrap().value;
                let rse = eng_to_rse(&eng).unwrap();
                said.extend(
                    rse.warnings
                        .iter()
                        .map(|w| format!("{name}: {}", w.message)),
                );
                // The figures filled agree with the curve; the reader may still flag the
                // delays, as it does the source's.
                let written = rse::parse(&rse::write(&rse.value).unwrap()).unwrap();
                for warning in &written.warnings {
                    assert!(warning.message.contains("delay"), "{name}: {warning:?}");
                }
                assert_eq!(written.value, rse.value, "{name}");
                let mut back = rse_to_eng(&written.value).unwrap().value;
                for (back, eng) in back.entries.iter_mut().zip(&eng.entries) {
                    // The two files whose masses move, each said in a warning pinned below;
                    // every other mass must come back bit for bit.
                    if rse
                        .warnings
                        .iter()
                        .any(|w| w.message.contains("more digits"))
                    {
                        moved.push(*name);
                        back.propellant_mass_kg = eng.propellant_mass_kg;
                        back.total_mass_kg = eng.total_mass_kg;
                    }
                    if back.delays != eng.delays {
                        respelled.push(format!("{name}: {} as {}", eng.delays, back.delays));
                        assert_eq!(back.delays(), eng.delays(), "{name}");
                        back.delays.clone_from(&eng.delays);
                    }
                }
                assert_eq!(back, eng, "{name}");
            } else {
                rses += 1;
                let rse = rse::parse(text).unwrap().value;
                let eng = rse_to_eng(&rse).unwrap();
                // Read back, it may be flagged as its source was, such as for a maker of two
                // words, but it reads as written.
                let written = eng::parse(&eng::write(&eng.value).unwrap()).unwrap();
                assert_eq!(written.value, eng.value, "{name}");
                let back = eng_to_rse(&written.value).unwrap().value;
                for (a, b) in rse.engines.iter().zip(&back.engines) {
                    let mut b = b.clone();
                    if b.manufacturer != a.manufacturer {
                        makers.push(format!("{name}: {} as {}", a.manufacturer, b.manufacturer));
                        b.manufacturer.clone_from(&a.manufacturer);
                    }
                    let carried = |e: &RseEngine| {
                        let points: Vec<(u64, u64)> = e
                            .points
                            .iter()
                            .map(|p| (p.time_s.to_bits(), p.thrust_n.to_bits()))
                            .collect();
                        (
                            e.code.clone(),
                            e.manufacturer.clone(),
                            [
                                e.diameter_mm,
                                e.length_mm,
                                e.initial_mass_g,
                                e.propellant_mass_g,
                            ]
                            .map(f64::to_bits),
                            e.delays.clone(),
                            points,
                        )
                    };
                    assert_eq!(carried(a), carried(&b), "{name}");
                }
                assert_eq!(rse_to_eng(&back).unwrap().value, written.value, "{name}");
            }
        }
        assert_eq!((engs, rses), (29, 3));
        // `.rse` writes a plugged motor `1000`, which comes back `P`, however `.eng` spelled it.
        assert_eq!(
            respelled,
            [
                "curves/5f4294d20002e90000000875.eng: p as P",
                "curves/5f4294d20002e90000000876.eng: 1000 as P"
            ]
        );
        // A maker of several words, which a .eng header joins with `_`.
        assert_eq!(
            makers,
            [
                "curves/5f923edb1bca5800041716ab.rse: Estes Industries, Inc. as Estes_Industries,_Inc."
            ]
        );
        assert_eq!(
            moved,
            [
                "curves/5f4294d20002e90000000884.eng",
                "curves/5f4294d20002e9000000088e.eng"
            ]
        );
        // The two masses of 17 digits, each said, and the only values that move.
        assert_eq!(
            said,
            [
                "curves/5f4294d20002e90000000884.eng: 7.5120000000000005 kg has more digits than a \
                 value in g keeps: it is written 7512.000000000001 g, which reads back as \
                 7.512000000000001 kg",
                "curves/5f4294d20002e9000000088e.eng: 0.0036000000000000003 kg has more digits \
                 than a value in g keeps: it is written 3.6 g, which reads back as 0.0036 kg"
            ]
        );
    }

    /// Delays that name none are as good as none: `.rse` leaves them out, and `.eng` refuses them.
    #[test]
    fn delays_that_name_none_are_left_out_or_refused() {
        let mut eng = eng::parse(ENG).unwrap().value;
        eng.entries[0].delays = "-".to_owned();
        let rse = eng_to_rse(&eng).unwrap();
        assert_eq!(rse.value.engines[0].delays, None);
        assert!(
            rse.warnings[0]
                .message
                .starts_with("dropped the delays \"-\"")
        );
        let mut rse = rse::parse(RSE).unwrap().value;
        for delays in ["", " ", "abc"] {
            rse.engines[0].delays = Some(delays.to_owned());
            let error = rse_to_eng(&rse).unwrap_err().to_string();
            assert!(error.contains("gives no delays"), "{delays:?}: {error}");
        }
    }

    /// A mass at a double's limits has no value in the other unit, and a figure that would be
    /// infinite is refused rather than written.
    #[test]
    fn a_mass_at_a_doubles_limits_is_refused() {
        let mut rse = rse::parse(RSE).unwrap().value;
        rse.engines[0].propellant_mass_g = 5e-324;
        let error = rse_to_eng(&rse).unwrap_err();
        assert!(
            matches!(
                error,
                MotorError::Domain {
                    what: "mass (g), which has no value in kg",
                    ..
                }
            ),
            "{error:?}"
        );
        let mut eng = eng::parse(ENG).unwrap().value;
        eng.entries[0].total_mass_kg = 1e306;
        let error = eng_to_rse(&eng).unwrap_err();
        assert!(
            matches!(
                error,
                MotorError::Domain {
                    what: "mass (kg), which has no value in g",
                    ..
                }
            ),
            "{error:?}"
        );
        let mut eng = eng::parse(ENG).unwrap().value;
        eng.entries[0].propellant_mass_kg = 1e-310;
        let error = eng_to_rse(&eng).unwrap_err();
        assert!(
            matches!(
                error,
                MotorError::Domain {
                    what: "specific impulse (s)",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    /// A mass with at most 15 significant digits, as a file writes it.
    fn file_mass() -> impl Strategy<Value = f64> {
        (1u64..1_000_000_000_000_000, -12i32..4)
            .prop_map(|(digits, exponent)| format!("{digits}e{exponent}").parse().unwrap())
    }

    proptest! {
        #[test]
        fn a_mass_with_15_digits_survives_kg_to_g_and_back(mass in file_mass()) {
            prop_assert_eq!(g_to_kg(kg_to_g(mass)).to_bits(), mass.to_bits());
            prop_assert_eq!(kg_to_g(g_to_kg(mass)).to_bits(), mass.to_bits());
        }
    }
}
