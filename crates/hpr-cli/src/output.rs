//! The types `--json` prints: one per command's output, and [`ErrorDocument`] for a failure.
//!
//! They belong to this crate, not the libraries', so that the published schemas change only when
//! the command's output does. Each derives [`JsonSchema`]; [`schemas`] generates the documents
//! `cargo xtask cli` writes to `schema/cli/`, and the tests check every output against them.
//!
//! Units are SI and named in the field, except where a motor's catalog states millimetres or
//! grams, as its field names say.

use schemars::JsonSchema;
use serde::Serialize;

/// `hpr motors list`: the bundled catalog's motors, as ThrustCurve.org states them.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct MotorList {
    /// Where the catalog comes from.
    pub catalog: CatalogInfo,
    /// The motors that pass the filters, in catalog order (by impulse class).
    pub motors: Vec<ListedMotor>,
}

/// Where the bundled catalog comes from.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct CatalogInfo {
    /// The source, such as `ThrustCurve.org API v1`.
    pub source: String,
    /// The date the catalog was captured, `YYYY-MM-DD`.
    pub captured: String,
    /// Which motors were taken, and why.
    pub selection: String,
}

/// One motor in the catalog, with ThrustCurve.org's stated figures.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ListedMotor {
    /// The manufacturer's full designation, such as `1266J760-19A`.
    pub designation: String,
    /// The common name, such as `J760`.
    pub common_name: String,
    /// The manufacturer.
    pub manufacturer: String,
    /// The manufacturer's abbreviation, such as `CTI`.
    pub manufacturer_abbrev: String,
    /// The impulse class, such as `J`.
    pub impulse_class: String,
    /// `single_use` or `reload`.
    pub motor_type: MotorKind,
    /// Casing diameter, mm.
    pub diameter_mm: f64,
    /// Casing length, mm.
    pub length_mm: f64,
    /// Total impulse, N·s, as stated.
    pub total_impulse_ns: f64,
    /// Average thrust, N, as stated.
    pub average_thrust_n: f64,
    /// Burn time, s, as stated.
    pub burn_time_s: f64,
    /// The delays as the catalog writes them, such as `6,8,10` or `P` (plugged).
    pub delays: Option<String>,
}

/// Whether a motor is used once or reloaded into a reusable case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MotorKind {
    /// A single-use motor.
    SingleUse,
    /// A reload for a reusable case.
    Reload,
    /// A hybrid: listed, but hpr models solid motors only.
    Hybrid,
}

/// `hpr motors show`: each motor's figures, worked out from its thrust curve.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct MotorShow {
    /// The motors found: every catalog match for a name, or every motor in a file.
    pub motors: Vec<MotorFigures>,
    /// What the reader accepted with a caveat, in file order.
    pub warnings: Vec<Warning>,
}

/// One motor's figures, worked out by hpr from its thrust curve.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct MotorFigures {
    /// The motor's name: the catalog's designation, or the file's.
    pub name: String,
    /// The manufacturer, as the catalog or the file writes it.
    pub manufacturer: String,
    /// Where the motor and its curve come from.
    pub source: MotorSource,
    /// Casing diameter, m.
    pub diameter_m: f64,
    /// Casing length, m.
    pub length_m: f64,
    /// Propellant mass, kg.
    pub propellant_mass_kg: f64,
    /// Loaded motor mass, kg.
    pub loaded_mass_kg: f64,
    /// Total impulse, N·s: the thrust curve's area, joining its points with straight lines.
    pub total_impulse_ns: f64,
    /// The impulse class the total impulse falls in, such as `J`.
    pub impulse_class: String,
    /// Average thrust, N: total impulse over the burn time.
    pub average_thrust_n: f64,
    /// Peak thrust, N: the curve's largest point.
    pub peak_thrust_n: f64,
    /// Burn time, s, by NFPA 1125: from when thrust first reaches 5% of its peak to when it last
    /// falls to it.
    pub burn_time_s: f64,
    /// When the NFPA 1125 burn starts, s.
    pub burn_start_s: f64,
    /// When the NFPA 1125 burn ends, s.
    pub burn_end_s: f64,
    /// The curve's last time, s.
    pub curve_end_s: f64,
    /// The ejection delays offered.
    pub delays: Vec<Delay>,
    /// ThrustCurve.org's stated figures, for a catalog motor.
    pub stated: Option<StatedFigures>,
}

/// Where a motor comes from.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MotorSource {
    /// The bundled catalog, with a public-domain curve from ThrustCurve.org.
    Catalog {
        /// The curve file's ThrustCurve.org page.
        curve_url: String,
        /// The curve's format: `eng` or `rse`.
        format: FileFormat,
    },
    /// A motor file named on the command line.
    File {
        /// The path as given.
        path: String,
        /// The file's format: `eng` or `rse`.
        format: FileFormat,
    },
}

/// A motor file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FileFormat {
    /// RASP `.eng`.
    Eng,
    /// RockSim `.rse`.
    Rse,
}

/// One ejection delay setting.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Delay {
    /// The ejection charge fires this many seconds after burnout.
    Seconds(f64),
    /// No ejection charge: the forward closure is plugged.
    Plugged,
    /// A `0`: the RASP format means a charge at burnout, but most files mean plugged.
    ZeroOrPlugged,
}

/// ThrustCurve.org's stated figures for a catalog motor, to set beside hpr's.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct StatedFigures {
    /// The impulse class.
    pub impulse_class: String,
    /// Total impulse, N·s.
    pub total_impulse_ns: f64,
    /// Average thrust, N.
    pub average_thrust_n: f64,
    /// Peak thrust, N, where stated.
    pub max_thrust_n: Option<f64>,
    /// Burn time, s.
    pub burn_time_s: f64,
}

/// Something a reader accepted with a caveat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Warning {
    /// The motor it is about, when it is about one.
    pub motor: Option<String>,
    /// The 1-based line in the file, when it is about one.
    pub line: Option<usize>,
    /// How serious it is.
    pub kind: WarningKind,
    /// What was found, and how it was read.
    pub message: String,
}

/// How serious a [`Warning`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WarningKind {
    /// A whole motor had an error and was left out.
    Skipped,
    /// A value was dropped or ignored.
    Dropped,
    /// Something unusual was read as it stands.
    Unusual,
}

/// `hpr completions`: a shell completion script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Completions {
    /// The shell, such as `bash`.
    pub shell: String,
    /// The script.
    pub script: String,
}

/// What `--json` prints when a command fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ErrorDocument {
    /// The failure.
    pub error: ErrorBody,
}

/// A failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ErrorBody {
    /// What kind of failure it is; the exit status says the same.
    pub kind: ErrorKind,
    /// What went wrong, for a person.
    pub message: String,
    /// The command, when the command line named one.
    pub command: Option<String>,
    /// For `not_available`, the milestone that brings the command, such as `M4.2b`.
    pub milestone: Option<String>,
}

/// What kind of failure an [`ErrorDocument`] reports, with its exit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Exit 1: an input was missing, unreadable or refused.
    Input,
    /// Exit 2: the command line was wrong.
    Usage,
    /// Exit 3: the command's milestone hasn't come yet.
    NotAvailable,
}

impl ErrorDocument {
    /// A failure of `kind`.
    pub fn new(
        kind: ErrorKind,
        message: impl Into<String>,
        command: Option<&str>,
        milestone: Option<&str>,
    ) -> Self {
        Self {
            error: ErrorBody {
                kind,
                message: message.into(),
                command: command.map(str::to_owned),
                milestone: milestone.map(str::to_owned),
            },
        }
    }
}

/// The JSON Schema of each output, by the file name it is published under.
pub fn schemas() -> Vec<(&'static str, String)> {
    let mut schemas = vec![
        ("motors-list.schema.json", schema::<MotorList>()),
        ("motors-show.schema.json", schema::<MotorShow>()),
        ("completions.schema.json", schema::<Completions>()),
        ("error.schema.json", schema::<ErrorDocument>()),
    ];
    schemas.sort_by_key(|(name, _)| *name);
    schemas
}

/// One type's schema, pretty-printed with a trailing newline.
#[expect(
    clippy::expect_used,
    reason = "a `Schema` is a JSON value with string keys, which always serializes"
)]
fn schema<T: JsonSchema>() -> String {
    let schema = schemars::schema_for!(T);
    let text = serde_json::to_string_pretty(&schema).expect("a JSON value serializes");
    format!("{text}\n")
}
