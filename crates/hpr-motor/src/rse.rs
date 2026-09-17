//! RockSim `.rse` motor files: reading and writing (`docs/format/rse.md`).
//!
//! A file is an XML engine database. Each `<engine>` carries its summary as attributes (diameter
//! and length in mm, masses in grams) and its samples as `<eng-data t f m cg/>` points, where `m`
//! is the propellant left in grams and `cg` the motor's centre of gravity in mm.
//!
//! The published guide is thin and disagrees with every real file on names, so the reader follows
//! the observed structure and also accepts the guide's (`<point>`, `type`, `initMass`,
//! `propMass`). Like [`crate::eng`], the file model keeps the file's units and points exactly, the
//! reader reports what it accepted as [`ParseWarning`]s, and the writer refuses what the reader
//! would read back differently.

use std::fmt::Write as _;

use roxmltree::{Document, Node, ParsingOptions};
use serde::{Deserialize, Serialize};

use crate::curve::ThrustCurve;
use crate::delay::DelayList;
use crate::error::MotorError;
use crate::text::{ParseWarning, Parsed, WarningKind, check_writable, finite};

const FORMAT: &str = ".rse";

/// Attributes that only control how RockSim draws graphs [P p.2]; read and dropped silently.
const RENDERING: [&str; 12] = [
    "tDiv", "tStep", "tFix", "FDiv", "FStep", "FFix", "mDiv", "mStep", "mFix", "cgDiv", "cgStep",
    "cgFix",
];

/// A parsed `.rse` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RseFile {
    /// The engines, in file order.
    pub engines: Vec<RseEngine>,
}

/// One engine in a `.rse` file. Optional attributes the file doesn't give are `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RseEngine {
    /// `mfg`: the manufacturer, verbatim (real files have leading spaces and commas).
    pub manufacturer: String,
    /// `code`: the engine's designation, the lookup key.
    pub code: String,
    /// `Type`: `single-use`, `reloadable`, `hybrid` or other text.
    pub motor_type: Option<String>,
    /// `dia`: diameter, mm.
    pub diameter_mm: f64,
    /// `len`: length, mm.
    pub length_mm: f64,
    /// `initWt`: loaded motor mass, g.
    pub initial_mass_g: f64,
    /// `propWt`: propellant mass, g.
    pub propellant_mass_g: f64,
    /// `delays`: the delay string as written, such as `6,10,14`; see [`RseEngine::delays`].
    pub delays: Option<String>,
    /// `auto-calc-mass`: RockSim computes the mass curve itself.
    pub auto_calc_mass: Option<bool>,
    /// `auto-calc-cg`: RockSim holds the CG at the engine's centre.
    pub auto_calc_cg: Option<bool>,
    /// `avgThrust`: average thrust, N.
    pub average_thrust_n: Option<f64>,
    /// `peakThrust`: peak thrust, N.
    pub peak_thrust_n: Option<f64>,
    /// `throatDia`: nozzle throat diameter, mm (zero in every observed file).
    pub throat_diameter_mm: Option<f64>,
    /// `exitDia`: nozzle exit diameter, mm (zero in every observed file).
    pub exit_diameter_mm: Option<f64>,
    /// `Itot`: total impulse, N·s.
    pub total_impulse_ns: Option<f64>,
    /// `burn-time`: burn time, s.
    pub burn_time_s: Option<f64>,
    /// `massFrac`: propellant mass fraction, %.
    pub mass_fraction_pct: Option<f64>,
    /// `Isp`: specific impulse, s.
    pub isp_s: Option<f64>,
    /// `<comments>` text, verbatim.
    pub comments: Option<String>,
    /// The samples, as listed.
    pub points: Vec<RsePoint>,
}

/// One `<eng-data>` sample.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RsePoint {
    /// `t`: time since ignition, s.
    pub time_s: f64,
    /// `f`: thrust, N.
    pub thrust_n: f64,
    /// `m`: propellant mass left, g (observed; the guide says only "mass").
    pub mass_g: Option<f64>,
    /// `cg`: motor centre of gravity, mm, from an end the guide doesn't name.
    pub cg_mm: Option<f64>,
}

impl RseEngine {
    /// The delay settings read from [`RseEngine::delays`]; empty when the file gives none.
    pub fn delays(&self) -> DelayList {
        DelayList::parse(self.delays.as_deref().unwrap_or(""))
    }

    /// The engine's thrust curve.
    ///
    /// # Errors
    ///
    /// As [`ThrustCurve::new`]: negative thrust, decreasing time, or no thrust.
    pub fn thrust_curve(&self) -> Result<ThrustCurve, MotorError> {
        let (times, thrusts) = self.points.iter().map(|p| (p.time_s, p.thrust_n)).unzip();
        ThrustCurve::new(times, thrusts)
    }
}

/// Reads a `.rse` file.
///
/// An engine with an error is skipped, with the error as a warning, when other engines in the file
/// read. An `<engine>` nested inside another engine is not read.
///
/// # Errors
///
/// [`MotorError::Syntax`], with the line number, for XML that isn't well formed (or has a DTD),
/// elements nested more than [`MAX_ELEMENT_DEPTH`] deep, or no `<engine>` elements; and, when no engine reads, the first engine's error: a missing required
/// attribute (`code`, `dia`, `len`, `initWt`, `propWt`), an unreadable or non-finite number, a
/// non-positive diameter or length, a negative mass, fewer than two points, a point without `t`
/// or `f`, or negative or decreasing time.
pub fn parse(text: &str) -> Result<Parsed<RseFile>, MotorError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    check_nesting(text)?;
    let options = ParsingOptions {
        allow_dtd: false,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(text, options).map_err(|error| {
        syntax(
            error.pos().row as usize,
            format!("not well-formed XML: {error}"),
        )
    })?;
    let lines = Lines::new(text);
    let mut warnings = Vec::new();
    let mut engines = Vec::new();
    let mut errors = Vec::new();
    let is_engine = |node: &Node<'_, '_>| node.has_tag_name("engine");
    for node in doc.descendants().filter(is_engine) {
        if node
            .ancestors()
            .skip(1)
            .any(|ancestor| is_engine(&ancestor))
        {
            warnings.push(ParseWarning::new(
                lines.of(node),
                WarningKind::Dropped,
                "an <engine> inside another engine is ignored",
            ));
            continue;
        }
        let mut engine_warnings = Vec::new();
        match engine(&lines, node, &mut engine_warnings) {
            Ok(engine) => {
                engines.push(engine);
                warnings.append(&mut engine_warnings);
            }
            Err(error) => errors.push(error),
        }
    }
    if engines.is_empty() {
        return Err(errors
            .into_iter()
            .next()
            .unwrap_or_else(|| syntax(1, "no <engine> elements".into())));
    }
    for error in errors {
        let line = match &error {
            MotorError::Syntax { line, .. } => *line,
            _ => 0,
        };
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Skipped,
            format!("engine skipped: {error}"),
        ));
    }
    warnings.sort_by_key(|warning| warning.line);
    Ok(Parsed {
        value: RseFile { engines },
        warnings,
    })
}

/// The byte offset where each line starts, to turn node positions into line numbers without
/// rescanning the text for every node.
struct Lines {
    starts: Vec<usize>,
}

impl Lines {
    fn new(text: &str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { starts }
    }

    /// The 1-based line a node starts on.
    fn of(&self, node: Node<'_, '_>) -> usize {
        self.starts
            .partition_point(|&start| start <= node.range().start)
    }
}

/// Writes a `.rse` file in RockSim's layout and attribute order.
///
/// # Errors
///
/// - [`MotorError::Inconsistent`] for no engines, an engine with fewer than two points, points of
///   which only some carry `m` or `cg`, or text holding a character XML 1.0 can't carry.
/// - [`MotorError::Domain`] for the values the reader rejects: non-finite numbers, a
///   non-positive diameter or length, negative masses or times, and decreasing times.
pub fn write(file: &RseFile) -> Result<String, MotorError> {
    if file.engines.is_empty() {
        return Err(MotorError::Inconsistent(
            "a .rse file needs an engine".into(),
        ));
    }
    let mut out = String::from("<engine-database>\n  <engine-list>\n");
    for engine in &file.engines {
        write_engine(&mut out, engine)?;
    }
    out.push_str("  </engine-list>\n</engine-database>\n");
    Ok(out)
}

fn write_engine(out: &mut String, engine: &RseEngine) -> Result<(), MotorError> {
    check_dimensions(engine.diameter_mm, engine.length_mm)?;
    check_writable(engine.initial_mass_g, "initial mass (g)", true)?;
    check_writable(engine.propellant_mass_g, "propellant mass (g)", true)?;
    if engine.points.len() < 2 {
        return Err(MotorError::Inconsistent(format!(
            "the .rse engine {:?} needs at least two points",
            engine.code
        )));
    }
    let mut attributes: Vec<(&str, String)> = vec![
        ("mfg", xml_text(&engine.manufacturer, true)?),
        ("code", xml_text(&engine.code, true)?),
    ];
    let mut text = |name, value: &Option<String>| -> Result<(), MotorError> {
        if let Some(value) = value {
            attributes.push((name, xml_text(value, true)?));
        }
        Ok(())
    };
    text("Type", &engine.motor_type)?;
    attributes.push(("dia", engine.diameter_mm.to_string()));
    attributes.push(("len", engine.length_mm.to_string()));
    attributes.push(("initWt", engine.initial_mass_g.to_string()));
    attributes.push(("propWt", engine.propellant_mass_g.to_string()));
    if let Some(delays) = &engine.delays {
        attributes.push(("delays", xml_text(delays, true)?));
    }
    for (name, flag) in [
        ("auto-calc-mass", engine.auto_calc_mass),
        ("auto-calc-cg", engine.auto_calc_cg),
    ] {
        if let Some(flag) = flag {
            attributes.push((name, if flag { "1" } else { "0" }.to_owned()));
        }
    }
    for (name, value) in [
        ("avgThrust", engine.average_thrust_n),
        ("peakThrust", engine.peak_thrust_n),
        ("throatDia", engine.throat_diameter_mm),
        ("exitDia", engine.exit_diameter_mm),
        ("Itot", engine.total_impulse_ns),
        ("burn-time", engine.burn_time_s),
        ("massFrac", engine.mass_fraction_pct),
        ("Isp", engine.isp_s),
    ] {
        if let Some(value) = value {
            check_writable(value, "optional .rse attribute", false)?;
            attributes.push((name, value.to_string()));
        }
    }
    out.push_str("    <engine");
    for (name, value) in &attributes {
        let _ = write!(out, " {name}=\"{value}\"");
    }
    out.push_str(">\n");
    if let Some(comments) = &engine.comments {
        let _ = writeln!(
            out,
            "      <comments>{}</comments>",
            xml_text(comments, false)?
        );
    }
    out.push_str("      <data>\n");
    let with_mass = engine.points.iter().filter(|p| p.mass_g.is_some()).count();
    let with_cg = engine.points.iter().filter(|p| p.cg_mm.is_some()).count();
    for (count, what) in [(with_mass, "m"), (with_cg, "cg")] {
        if count != 0 && count != engine.points.len() {
            return Err(MotorError::Inconsistent(format!(
                "the .rse engine {:?} gives `{what}` on some points but not all",
                engine.code
            )));
        }
    }
    let mut previous = 0.0;
    for point in &engine.points {
        check_writable(point.time_s, "time (s)", true)?;
        check_writable(point.thrust_n, "thrust (N)", false)?;
        if point.time_s < previous {
            return Err(MotorError::Domain {
                what: "time (s), which decreases",
                value: point.time_s,
            });
        }
        previous = point.time_s;
        let _ = write!(
            out,
            "        <eng-data t=\"{}\" f=\"{}\"",
            point.time_s, point.thrust_n
        );
        if let Some(mass) = point.mass_g {
            check_writable(mass, "point mass (g)", false)?;
            let _ = write!(out, " m=\"{mass}\"");
        }
        if let Some(cg) = point.cg_mm {
            check_writable(cg, "point CG (mm)", false)?;
            let _ = write!(out, " cg=\"{cg}\"");
        }
        out.push_str("/>\n");
    }
    out.push_str("      </data>\n    </engine>\n");
    Ok(())
}

/// Escapes text for an attribute value (`attribute`) or element content. Characters XML 1.0
/// normalizes on reading are written as references: tab, LF and CR in attributes [X §3.3.3], and
/// CR in content [X §2.11].
fn xml_text(value: &str, attribute: bool) -> Result<String, MotorError> {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' if attribute => escaped.push_str("&quot;"),
            '\t' if attribute => escaped.push_str("&#9;"),
            '\n' if attribute => escaped.push_str("&#10;"),
            '\r' => escaped.push_str("&#13;"),
            '\t' | '\n' => escaped.push(c),
            // XML 1.0 §2.2: no other C0 controls, surrogates (impossible in a str) or U+FFFE/FFFF.
            '\u{0}'..='\u{1f}' | '\u{fffe}' | '\u{ffff}' => {
                return Err(MotorError::Inconsistent(format!(
                    "{value:?} holds a character XML 1.0 can't carry"
                )));
            }
            _ => escaped.push(c),
        }
    }
    Ok(escaped)
}

fn engine(
    lines: &Lines,
    node: Node<'_, '_>,
    warnings: &mut Vec<ParseWarning>,
) -> Result<RseEngine, MotorError> {
    let line = lines.of(node);
    for attribute in node.attributes() {
        let name = attribute.name();
        if !KNOWN.iter().any(|known| known.eq_ignore_ascii_case(name))
            && !RENDERING
                .iter()
                .any(|known| known.eq_ignore_ascii_case(name))
        {
            warnings.push(ParseWarning::new(
                line,
                WarningKind::Dropped,
                format!("unknown <engine> attribute `{name}` ignored"),
            ));
        }
    }
    let code = attr(node, &["code"]).ok_or_else(|| missing(line, "code"))?;
    let manufacturer = match attr(node, &["mfg"]) {
        Some(mfg) => mfg.to_owned(),
        None => {
            warnings.push(ParseWarning::new(
                line,
                WarningKind::Unusual,
                format!("engine {code:?} has no `mfg`; read as empty"),
            ));
            String::new()
        }
    };
    let required = |names: &[&'static str]| -> Result<f64, MotorError> {
        let text = attr(node, names).ok_or_else(|| missing(line, names[0]))?;
        number(text, names[0], line)
    };
    let optional = |name: &'static str| -> Result<Option<f64>, MotorError> {
        attr(node, &[name])
            .map(|text| number(text, name, line))
            .transpose()
    };
    let diameter_mm = required(&["dia"])?;
    let length_mm = required(&["len"])?;
    check_dimensions(diameter_mm, length_mm).map_err(|e| syntax(line, e.to_string()))?;
    let initial_mass_g = required(&["initWt", "initMass"])?;
    let propellant_mass_g = required(&["propWt", "propMass"])?;
    for (value, what) in [(initial_mass_g, "initWt"), (propellant_mass_g, "propWt")] {
        if value < 0.0 {
            return Err(syntax(line, format!("negative `{what}` {value}")));
        }
    }
    if propellant_mass_g >= initial_mass_g && initial_mass_g > 0.0 {
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Unusual,
            format!(
                "engine {code:?}: propellant mass {propellant_mass_g} g is not below the loaded \
                 mass {initial_mass_g} g"
            ),
        ));
    }
    let mut flag = |name: &'static str| match attr(node, &[name]) {
        None => None,
        Some("1") => Some(true),
        Some("0") => Some(false),
        Some(other) => {
            warnings.push(ParseWarning::new(
                line,
                WarningKind::Dropped,
                format!("`{name}` is {other:?}, not 0 or 1; ignored"),
            ));
            None
        }
    };
    let auto_calc_mass = flag("auto-calc-mass");
    let auto_calc_cg = flag("auto-calc-cg");

    let mut comments = None;
    let mut data = None;
    for child in node.children().filter(Node::is_element) {
        let name = child.tag_name().name();
        let duplicate = match name {
            "comments" => comments
                .replace(
                    // The text only: XML comments and processing instructions inside are not part
                    // of it, and nested markup contributes its text.
                    child
                        .descendants()
                        .filter(Node::is_text)
                        .filter_map(|text| text.text())
                        .collect::<String>(),
                )
                .is_some(),
            "data" => data.replace(child).is_some(),
            other => {
                warnings.push(ParseWarning::new(
                    lines.of(child),
                    WarningKind::Dropped,
                    format!("unknown element <{other}> in an engine ignored"),
                ));
                false
            }
        };
        if duplicate {
            warnings.push(ParseWarning::new(
                lines.of(child),
                WarningKind::Dropped,
                format!("engine {code:?} has more than one <{name}>; the last is read"),
            ));
        }
    }
    let data = data.ok_or_else(|| syntax(line, format!("engine {code:?} has no <data>")))?;
    let points = points(lines, data, code, warnings)?;

    let engine = RseEngine {
        manufacturer,
        code: code.to_owned(),
        motor_type: attr(node, &["Type"]).map(str::to_owned),
        diameter_mm,
        length_mm,
        initial_mass_g,
        propellant_mass_g,
        delays: attr(node, &["delays"]).map(str::to_owned),
        auto_calc_mass,
        auto_calc_cg,
        average_thrust_n: optional("avgThrust")?,
        peak_thrust_n: optional("peakThrust")?,
        throat_diameter_mm: optional("throatDia")?,
        exit_diameter_mm: optional("exitDia")?,
        total_impulse_ns: optional("Itot")?,
        burn_time_s: optional("burn-time")?,
        mass_fraction_pct: optional("massFrac")?,
        isp_s: optional("Isp")?,
        comments,
        points,
    };
    summary_warnings(&engine, line, warnings);
    Ok(engine)
}

/// Every `<engine>` attribute the reader reads (with the guide's aliases).
const KNOWN: [&str; 20] = [
    "mfg",
    "code",
    "Type",
    "dia",
    "len",
    "initWt",
    "initMass",
    "propWt",
    "propMass",
    "delays",
    "auto-calc-mass",
    "auto-calc-cg",
    "avgThrust",
    "peakThrust",
    "throatDia",
    "exitDia",
    "Itot",
    "burn-time",
    "massFrac",
    "Isp",
];

fn points(
    lines: &Lines,
    data: Node<'_, '_>,
    code: &str,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Vec<RsePoint>, MotorError> {
    let mut points: Vec<RsePoint> = Vec::new();
    for node in data.children().filter(Node::is_element) {
        let line = lines.of(node);
        if !(node.has_tag_name("eng-data") || node.has_tag_name("point")) {
            warnings.push(ParseWarning::new(
                line,
                WarningKind::Dropped,
                format!(
                    "unknown element <{}> in <data> ignored",
                    node.tag_name().name()
                ),
            ));
            continue;
        }
        let get = |name: &'static str| -> Result<Option<f64>, MotorError> {
            attr(node, &[name])
                .map(|text| number(text, name, line))
                .transpose()
        };
        let time_s = get("t")?.ok_or_else(|| missing(line, "t"))?;
        let thrust_n = get("f")?.ok_or_else(|| missing(line, "f"))?;
        if time_s < 0.0 {
            return Err(syntax(line, format!("negative time {time_s}")));
        }
        if let Some(previous) = points.last()
            && time_s < previous.time_s
        {
            return Err(syntax(
                line,
                format!(
                    "time {time_s} s is before the previous point's {} s",
                    previous.time_s
                ),
            ));
        }
        points.push(RsePoint {
            time_s,
            thrust_n,
            mass_g: get("m")?,
            cg_mm: get("cg")?,
        });
    }
    let line = lines.of(data);
    if points.len() < 2 {
        return Err(syntax(
            line,
            format!("engine {code:?} needs at least two points"),
        ));
    }
    let total = points.len();
    let partial = |count: usize| count != 0 && count != total;
    if partial(points.iter().filter(|p| p.mass_g.is_some()).count()) {
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Dropped,
            format!("engine {code:?} gives `m` on only some points; all dropped"),
        ));
        points.iter_mut().for_each(|p| p.mass_g = None);
    }
    if partial(points.iter().filter(|p| p.cg_mm.is_some()).count()) {
        warnings.push(ParseWarning::new(
            line,
            WarningKind::Dropped,
            format!("engine {code:?} gives `cg` on only some points; all dropped"),
        ));
        points.iter_mut().for_each(|p| p.cg_mm = None);
    }
    Ok(points)
}

/// Warnings about a curve's end, its delays, a hybrid motor, and summary attributes that disagree
/// with the curve.
fn summary_warnings(engine: &RseEngine, line: usize, warnings: &mut Vec<ParseWarning>) {
    let mut warn = |kind: WarningKind, message: String| {
        warnings.push(ParseWarning::new(
            line,
            kind,
            format!("engine {:?}: {message}", engine.code),
        ));
    };
    for warning in engine.delays().warnings {
        warn(warning.kind, warning.message);
    }
    if engine
        .motor_type
        .as_deref()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("hybrid"))
    {
        warn(
            WarningKind::Unusual,
            "a hybrid motor: hpr models solid motors only, and a `SolidMotor` built from this \
             curve would be wrong"
                .into(),
        );
    }
    if let Some(last) = engine.points.last()
        && last.thrust_n != 0.0
    {
        warn(
            WarningKind::Unusual,
            format!("the curve ends at {} N, not zero", last.thrust_n),
        );
    }
    if engine.points.iter().any(|p| p.thrust_n < 0.0) {
        warn(WarningKind::Unusual, "the curve has negative thrust".into());
        return;
    }
    let Ok(curve) = engine.thrust_curve() else {
        return;
    };
    for (given, computed, name) in [
        (engine.total_impulse_ns, curve.total_impulse_ns(), "Itot"),
        (engine.peak_thrust_n, curve.peak_thrust_n(), "peakThrust"),
    ] {
        if let Some(given) = given
            && (given - computed).abs() > 0.01 * computed.abs()
        {
            warn(
                WarningKind::Unusual,
                format!("`{name}` {given} differs from the curve's {computed} by more than 1%"),
            );
        }
    }
}

/// An attribute by any of `names`, matched exactly first and then ignoring ASCII case.
fn attr<'a>(node: Node<'a, '_>, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| node.attribute(*name))
        .or_else(|| {
            node.attributes()
                .find(|a| names.iter().any(|name| a.name().eq_ignore_ascii_case(name)))
                .map(|a| a.value())
        })
}

fn check_dimensions(diameter_mm: f64, length_mm: f64) -> Result<(), MotorError> {
    for (value, what) in [(diameter_mm, "diameter (mm)"), (length_mm, "length (mm)")] {
        if !(value.is_finite() && value > 0.0) {
            return Err(MotorError::Domain { what, value });
        }
    }
    Ok(())
}

fn number(text: &str, what: &'static str, line: usize) -> Result<f64, MotorError> {
    finite(text, what).map_err(|_| syntax(line, format!("can't read `{what}` from {text:?}")))
}

fn missing(line: usize, what: &str) -> MotorError {
    syntax(line, format!("missing the `{what}` attribute"))
}

fn syntax(line: usize, message: String) -> MotorError {
    MotorError::Syntax {
        format: FORMAT,
        line,
        message,
    }
}

/// The deepest element nesting [`parse`] accepts. Real files nest five deep (`<engine-database>`,
/// `<engine-list>`, `<engine>`, `<data>`, `<eng-data>`).
///
/// The XML parser descends into nested elements recursively, and a stack overflow aborts the
/// process rather than panicking, so a hostile or corrupt file must be refused before parsing.
/// Measured on macOS aarch64: a debug build uses about 15 KB of stack per level (48 levels fit in
/// 1 MB, the wasm32 default, and 64 don't), a release build under 1 KB.
pub const MAX_ELEMENT_DEPTH: usize = 32;

/// Refuses elements nested more than [`MAX_ELEMENT_DEPTH`] deep, scanning the text without
/// recursion.
///
/// The count is exact for well-formed XML: comments, CDATA sections, processing instructions and
/// declarations are skipped, a `>` inside a quoted attribute doesn't end a tag, and `<a/>` doesn't
/// nest. On malformed text it may count too many levels but never too few before the point where
/// the parser stops: a `<` ends a tag even inside quotes (it can't appear there in XML).
fn check_nesting(text: &str) -> Result<(), MotorError> {
    let bytes = text.as_bytes();
    let find = |from: usize, needle: &[u8]| {
        bytes[from..]
            .windows(needle.len())
            .position(|window| window == needle)
            .map_or(bytes.len(), |at| from + at + needle.len())
    };
    let mut depth = 0_usize;
    let mut i = 0;
    while let Some(offset) = bytes[i..].iter().position(|&b| b == b'<') {
        let start = i + offset;
        let rest = &bytes[start..];
        i = if rest.starts_with(b"<!--") {
            find(start + 4, b"-->")
        } else if rest.starts_with(b"<![CDATA[") {
            find(start + 9, b"]]>")
        } else if rest.starts_with(b"<?") {
            find(start + 2, b"?>")
        } else if rest.starts_with(b"<!") || rest.starts_with(b"</") {
            if rest[1] == b'/' {
                depth = depth.saturating_sub(1);
            }
            find(start + 2, b">")
        } else {
            let mut quote = None;
            let mut end = bytes.len();
            let mut self_closing = false;
            for (j, &b) in bytes.iter().enumerate().skip(start + 1) {
                match (quote, b) {
                    (_, b'<') => {
                        end = j;
                        break;
                    }
                    (None, b'"' | b'\'') => quote = Some(b),
                    (Some(open), _) if b == open => quote = None,
                    (None, b'>') => {
                        self_closing = bytes[j - 1] == b'/';
                        end = j + 1;
                        break;
                    }
                    _ => {}
                }
            }
            if !self_closing {
                depth += 1;
                if depth > MAX_ELEMENT_DEPTH {
                    let line = bytes[..start].iter().filter(|&&b| b == b'\n').count() + 1;
                    return Err(syntax(
                        line,
                        format!("elements nested more than {MAX_ELEMENT_DEPTH} deep"),
                    ));
                }
            }
            end
        };
        if i >= bytes.len() {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::delay::Delay;

    const SAMPLE: &str = r#"<engine-database>
  <engine-list>
    <engine  mfg=" Aerotech" code="H128W" Type="reloadable" dia="29." len="194." initWt="196.6"
      propWt="90.6" delays="6,10,14" auto-calc-mass="1" auto-calc-cg="1" avgThrust="128."
      peakThrust="150." throatDia="0." exitDia="0." Itot="10.2" burn-time="0.8" massFrac="46.1"
      Isp="200." tDiv="10" tStep="-1." tFix="1" FDiv="10" FStep="-1." FFix="1" mDiv="10"
      mStep="-1." mFix="1" cgDiv="10" cgStep="-1." cgFix="1">
      <comments>Line one &amp; two
<![CDATA[raw & data]]></comments>
      <data>
        <eng-data  t="0." f="0." m="90.6" cg="97."/>
        <eng-data  t="0.02" f="150." m="89.1" cg="97."/>
        <eng-data  t="0.1" f="100." m="80.2" cg="97."/>
        <eng-data  t="0.1" f="50." m="80.2" cg="97."/>
        <eng-data  t="0.2" f="0." m="0." cg="97."/>
      </data>
    </engine>
  </engine-list>
</engine-database>"#;

    #[test]
    fn deep_nesting_is_refused_before_the_xml_parser_recurses() {
        // Unchecked, 100,000 levels overflow any test thread's stack inside the XML parser, in
        // debug and release builds alike, and abort the test process; the scan refuses them
        // without recursing.
        let deep = |n: usize| format!("{}{}", "<a>".repeat(n), "</a>".repeat(n));
        let result = parse(&deep(100_000));
        assert!(
            matches!(&result, Err(MotorError::Syntax { line: 1, message, .. })
                if message.contains("nested")),
            "{result:?}"
        );

        // The sample nests four deep; wrapping it to exactly the limit still reads, and one more
        // level is refused on the line where it opens.
        let body = SAMPLE.split_once('\n').unwrap().1;
        let inner = body.strip_suffix("</engine-database>").unwrap();
        let wrapped = |extra: usize| {
            format!(
                "<engine-database>{}\n{inner}{}</engine-database>",
                "<!-- <x> --><x a='>'><y/>".repeat(extra),
                "</x>".repeat(extra)
            )
        };
        let at_limit = wrapped(MAX_ELEMENT_DEPTH - 4);
        assert_eq!(parse(&at_limit).unwrap().value.engines.len(), 1);
        let comments_line = at_limit
            .lines()
            .position(|l| l.contains("<comments>"))
            .unwrap()
            + 1;
        assert!(matches!(
            parse(&wrapped(MAX_ELEMENT_DEPTH - 3)),
            Err(MotorError::Syntax { line, .. }) if line == comments_line
        ));
        // A CDATA section, comment or processing instruction full of tags doesn't count.
        let hidden = format!(
            "<?xml version='1.0'?><!-- {0} --><engine-database><![CDATA[{0}]]><?pi {0}?>\n{body}",
            "<a>".repeat(100)
        );
        assert_eq!(parse(&hidden).unwrap().value.engines.len(), 1);
    }

    #[test]
    fn reads_the_observed_layout() {
        let parsed = parse(SAMPLE).unwrap();
        let engine = &parsed.value.engines[0];
        assert_eq!(engine.manufacturer, " Aerotech");
        assert_eq!(engine.code, "H128W");
        assert_eq!(engine.motor_type.as_deref(), Some("reloadable"));
        assert_eq!(engine.diameter_mm, 29.0);
        assert_eq!(engine.initial_mass_g, 196.6);
        assert_eq!(engine.auto_calc_mass, Some(true));
        assert_eq!(engine.exit_diameter_mm, Some(0.0));
        assert_eq!(
            engine.comments.as_deref(),
            Some("Line one & two\nraw & data")
        );
        assert_eq!(engine.points.len(), 5);
        assert_eq!(engine.points[1].mass_g, Some(89.1));
        assert_eq!(engine.points[4].cg_mm, Some(97.0));
        assert_eq!(
            engine.delays().delays,
            [
                Delay::Seconds(6.0),
                Delay::Seconds(10.0),
                Delay::Seconds(14.0)
            ]
        );
        let curve = engine.thrust_curve().unwrap();
        assert_eq!(curve.times_s()[3], 0.1);
        // Only the impulse disagrees with the curve (10.2 against 12.5 N·s).
        assert_eq!(parsed.warnings.len(), 1, "{:?}", parsed.warnings);
        assert_eq!(parsed.warnings[0].line, 3);
    }

    #[test]
    fn reads_the_guides_names_and_nested_engines() {
        let text = r#"<engine code="A1" mfg="X" type="single-use" dia="18" len="70" initMass="16"
            propMass="3"><data><point t="0.1" f="2"/><point t="0.5" f="0"/></data></engine>"#;
        let parsed = parse(text).unwrap();
        let engine = &parsed.value.engines[0];
        assert_eq!(engine.motor_type.as_deref(), Some("single-use"));
        assert_eq!(engine.initial_mass_g, 16.0);
        assert_eq!(engine.propellant_mass_g, 3.0);
        assert_eq!(engine.points[0].mass_g, None);
        assert!(parsed.warnings.is_empty(), "{:?}", parsed.warnings);
    }

    #[test]
    fn rejects_malformed_files() {
        let engine = |attrs: &str, points: &str| {
            format!(
                r#"<engine-database><engine-list><engine {attrs}><data>{points}</data></engine></engine-list></engine-database>"#
            )
        };
        let good_attrs = r#"code="A" mfg="X" dia="18" len="70" initWt="16" propWt="3""#;
        let good_points =
            r#"<eng-data t="0" f="0"/><eng-data t="0.1" f="2"/><eng-data t="0.5" f="0"/>"#;
        assert!(parse(&engine(good_attrs, good_points)).is_ok());
        let bad = [
            "".to_owned(),
            "<engine-database/>".to_owned(),
            "<engine code=\"A\"".to_owned(),
            "<!DOCTYPE x [<!ENTITY e \"boom\">]><engine-database/>".to_owned(),
            engine(
                r#"mfg="X" dia="18" len="70" initWt="16" propWt="3""#,
                good_points,
            ),
            engine(
                r#"code="A" mfg="X" len="70" initWt="16" propWt="3""#,
                good_points,
            ),
            engine(
                r#"code="A" mfg="X" dia="0" len="70" initWt="16" propWt="3""#,
                good_points,
            ),
            engine(
                r#"code="A" mfg="X" dia="18" len="70" initWt="16" propWt="-3""#,
                good_points,
            ),
            engine(
                r#"code="A" mfg="X" dia="NaN" len="70" initWt="16" propWt="3""#,
                good_points,
            ),
            engine(good_attrs, r#"<eng-data t="0" f="1"/>"#),
            engine(good_attrs, r#"<eng-data t="0" f="1"/><eng-data t="0.1"/>"#),
            engine(
                good_attrs,
                r#"<eng-data t="0.2" f="1"/><eng-data t="0.1" f="0"/>"#,
            ),
            engine(
                good_attrs,
                r#"<eng-data t="-0.1" f="1"/><eng-data t="0.1" f="0"/>"#,
            ),
            engine(
                good_attrs,
                r#"<eng-data t="0" f="inf"/><eng-data t="0.1" f="0"/>"#,
            ),
        ];
        for text in &bad {
            assert!(parse(text).is_err(), "{text}");
        }
    }

    /// Every f64 in a file, as bits, `None` as a marker: `==` alone treats `-0.0` as `0.0`.
    pub(crate) fn bits(file: &RseFile) -> Vec<Option<u64>> {
        file.engines
            .iter()
            .flat_map(|e| {
                [
                    Some(e.diameter_mm),
                    Some(e.length_mm),
                    Some(e.initial_mass_g),
                    Some(e.propellant_mass_g),
                    e.average_thrust_n,
                    e.peak_thrust_n,
                    e.throat_diameter_mm,
                    e.exit_diameter_mm,
                    e.total_impulse_ns,
                    e.burn_time_s,
                    e.mass_fraction_pct,
                    e.isp_s,
                ]
                .into_iter()
                .chain(
                    e.points
                        .iter()
                        .flat_map(|p| [Some(p.time_s), Some(p.thrust_n), p.mass_g, p.cg_mm]),
                )
                .map(|value| value.map(f64::to_bits))
                .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn comments_keep_only_their_text() {
        let text = r#"<engine code="A" mfg="X" dia="18" len="70" initWt="16" propWt="3">
<comments>keep<!-- editor note -->this<?pi x?><b>bold</b><b>a<i>b</i>c</b></comments>
<comments>second</comments>
<data><eng-data t="0" f="0"/><eng-data t="0.2" f="2"/><eng-data t="0.3" f="0"/></data>
<data><eng-data t="0" f="0"/><eng-data t="0.2" f="3"/><eng-data t="0.3" f="0"/></data>
<comments><engine code="B" mfg="X" dia="18" len="70" initWt="16" propWt="3"><data>
<eng-data t="0" f="0"/><eng-data t="0.2" f="2"/></data></engine></comments>
</engine>"#;
        let parsed = parse(text).unwrap();
        assert_eq!(
            parsed.value.engines.len(),
            1,
            "the nested engine is not read"
        );
        let engine = &parsed.value.engines[0];
        // The last <comments> wins; it holds only the nested engine, whose text is one line break.
        assert_eq!(engine.comments.as_deref(), Some("\n"));
        assert_eq!(engine.points[1].thrust_n, 3.0);
        let dropped: Vec<usize> = parsed
            .warnings
            .iter()
            .filter(|w| w.kind == WarningKind::Dropped)
            .map(|w| w.line)
            .collect();
        assert_eq!(dropped, [3, 5, 6, 6], "{:?}", parsed.warnings);
        let single = r#"<engine code="A" mfg="X" dia="18" len="70" initWt="16" propWt="3">
<comments>keep<!-- editor note -->this<?pi x?><b>bold</b><b>a<i>b</i>c</b></comments>
<data><eng-data t="0" f="0"/><eng-data t="0.2" f="2"/><eng-data t="0.3" f="0"/></data></engine>"#;
        let engine = &parse(single).unwrap().value.engines[0];
        assert_eq!(engine.comments.as_deref(), Some("keepthisboldabc"));
    }

    #[test]
    fn large_files_read_in_linear_time() {
        // 200 engines of 500 points each (about 10 MB of XML): line numbers come from a table, so
        // this takes milliseconds rather than rescanning the text for every point.
        let points: String = (0..500)
            .map(|i| {
                format!(
                    "<eng-data t=\"{}\" f=\"{}\" m=\"1\" cg=\"35\"/>\n",
                    f64::from(i) * 0.01,
                    10.0
                )
            })
            .collect();
        let engine = format!(
            "<engine code=\"A\" mfg=\"X\" dia=\"18\" len=\"70\" initWt=\"16\" propWt=\"3\">\n<data>\n{points}</data></engine>\n"
        );
        let text = format!(
            "<engine-database><engine-list>\n{}</engine-list></engine-database>",
            engine.repeat(200)
        );
        let parsed = parse(&text).unwrap();
        assert_eq!(parsed.value.engines.len(), 200);
        let last = text.lines().count();
        assert!(parsed.warnings.iter().all(|w| w.line <= last));
    }

    #[test]
    fn a_broken_engine_is_skipped_with_a_warning() {
        let text = r#"<engine-database><engine-list>
<engine code="A" mfg="X" dia="18" len="70" initWt="16" propWt="3"><data>
  <eng-data t="0" f="0"/><eng-data t="0.2" f="2"/><eng-data t="0.1" f="0"/></data></engine>
<engine code="B" mfg="X" dia="18" len="70" initWt="16" propWt="3"><data>
  <eng-data t="0" f="0"/><eng-data t="0.2" f="2"/><eng-data t="0.3" f="0"/></data></engine>
</engine-list></engine-database>"#;
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.value.engines.len(), 1);
        assert_eq!(parsed.value.engines[0].code, "B");
        assert_eq!(parsed.warnings.len(), 1);
        assert_eq!(parsed.warnings[0].line, 3);
        assert_eq!(parsed.warnings[0].kind, WarningKind::Skipped);
    }

    #[test]
    fn partial_mass_columns_are_dropped_with_a_warning() {
        let text = r#"<engine code="A" mfg="X" dia="18" len="70" initWt="16" propWt="3"><data>
            <eng-data t="0" f="0" m="3"/><eng-data t="0.1" f="2"/><eng-data t="0.5" f="0" m="0"/>
            </data></engine>"#;
        let parsed = parse(text).unwrap();
        assert!(
            parsed.value.engines[0]
                .points
                .iter()
                .all(|p| p.mass_g.is_none())
        );
        assert_eq!(parsed.warnings.len(), 1);
    }

    #[test]
    fn a_hybrid_is_read_with_a_warning() {
        let hybrid = |kind: &str| {
            format!(
                r#"<engine code="H" mfg="X" Type="{kind}" dia="38" len="300" initWt="900" propWt="200"><data>
                <eng-data t="0" f="0"/><eng-data t="0.5" f="200"/><eng-data t="1" f="0"/>
                </data></engine>"#
            )
        };
        let parsed = parse(&hybrid(" Hybrid")).unwrap();
        assert_eq!(parsed.value.engines.len(), 1);
        assert!(matches!(
            parsed.warnings.as_slice(),
            [w] if w.kind == WarningKind::Unusual && w.message.contains("hybrid")
        ));
        assert!(parse(&hybrid("reloadable")).unwrap().warnings.is_empty());
    }

    #[test]
    fn writes_what_it_reads() {
        let file = parse(SAMPLE).unwrap().value;
        let text = write(&file).unwrap();
        assert_eq!(parse(&text).unwrap().value, file);
        assert_eq!(write(&parse(&text).unwrap().value).unwrap(), text);
    }

    #[test]
    fn writer_refuses_what_it_cannot_represent() {
        let mut engine = parse(SAMPLE).unwrap().value.engines[0].clone();
        engine.points[1].mass_g = None;
        assert!(
            write(&RseFile {
                engines: vec![engine.clone()]
            })
            .is_err()
        );
        engine.points.iter_mut().for_each(|p| p.mass_g = None);
        assert!(
            write(&RseFile {
                engines: vec![engine.clone()]
            })
            .is_ok()
        );
        engine.code = "bell\u{7}".into();
        assert!(
            write(&RseFile {
                engines: vec![engine.clone()]
            })
            .is_err()
        );
        engine.code = "A".into();
        engine.points.truncate(1);
        assert!(
            write(&RseFile {
                engines: vec![engine]
            })
            .is_err()
        );
        assert!(write(&RseFile { engines: vec![] }).is_err());
    }

    fn text(max: usize) -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop_oneof![
                Just('&'),
                Just('<'),
                Just('>'),
                Just('"'),
                Just('\''),
                Just('\t'),
                Just('\n'),
                Just('\r'),
                Just(' '),
                Just('é'),
                Just(']'),
                proptest::char::range('a', 'z'),
            ],
            0..max,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    fn engines() -> impl Strategy<Value = RseEngine> {
        let number = || prop_oneof![Just(-0.0), -1e6..1e6f64];
        let option = move || prop::option::of(number());
        (
            (
                text(12),
                text(12),
                prop::option::of(text(8)),
                prop::option::of(text(8)),
            ),
            (1e-3..1e4f64, 1e-3..1e5f64, 0.0..1e5f64, 0.0..1e5f64),
            (
                prop::option::of(any::<bool>()),
                prop::option::of(any::<bool>()),
            ),
            (
                option(),
                option(),
                option(),
                option(),
                option(),
                option(),
                option(),
                option(),
            ),
            prop::option::of(text(40)),
            (any::<bool>(), any::<bool>()),
            prop::collection::vec((0.0..10.0f64, 0.0..1e5f64, number(), number()), 2..20),
        )
            .prop_map(
                |(strings, envelope, flags, summary, comments, columns, mut samples)| {
                    samples.sort_by(|a, b| a.0.total_cmp(&b.0));
                    RseEngine {
                        manufacturer: strings.0,
                        code: strings.1,
                        motor_type: strings.2,
                        delays: strings.3,
                        diameter_mm: envelope.0,
                        length_mm: envelope.1,
                        initial_mass_g: envelope.2,
                        propellant_mass_g: envelope.3,
                        auto_calc_mass: flags.0,
                        auto_calc_cg: flags.1,
                        average_thrust_n: summary.0,
                        peak_thrust_n: summary.1,
                        throat_diameter_mm: summary.2,
                        exit_diameter_mm: summary.3,
                        total_impulse_ns: summary.4,
                        burn_time_s: summary.5,
                        mass_fraction_pct: summary.6,
                        isp_s: summary.7,
                        comments,
                        points: samples
                            .into_iter()
                            .map(|(t, f, m, cg)| RsePoint {
                                time_s: t,
                                thrust_n: f,
                                mass_g: columns.0.then_some(m),
                                cg_mm: columns.1.then_some(cg),
                            })
                            .collect(),
                    }
                },
            )
    }

    proptest! {
        #[test]
        fn write_parse_round_trips_bit_for_bit(engines in prop::collection::vec(engines(), 1..4)) {
            let file = RseFile { engines };
            let written = write(&file).unwrap();
            let back = parse(&written).unwrap().value;
            prop_assert_eq!(&back, &file);
            prop_assert_eq!(bits(&back), bits(&file));
        }
    }
}
