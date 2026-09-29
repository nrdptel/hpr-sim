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
    /// `single_use`, `reload` or `hybrid`.
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

/// How serious a warning is.
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

/// `hpr sim`: what was flown, from where, and how the flight went.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SimFlight {
    /// The design and the configuration flown.
    pub design: SimDesign,
    /// The motors flown.
    pub motors: Vec<SimMotor>,
    /// The launch site, rail and wind.
    pub launch: Launch,
    /// The flight's metrics.
    pub summary: Summary,
    /// The flight's events, in order.
    pub events: Vec<SimEvent>,
    /// The recording files written, in the order given.
    pub exports: Vec<Export>,
    /// What the flight leaves out of the design, such as its parachutes, and what to make of the
    /// numbers it leaves out.
    pub notes: Vec<String>,
    /// What the design's or the motor file's reader accepted with a caveat, and what the design's
    /// checks found unusual but buildable.
    pub warnings: Vec<InputWarning>,
}

/// The design flown.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SimDesign {
    /// The file's name, without its folder.
    pub file: String,
    /// The file's format.
    pub format: DesignFormat,
    /// The rocket's name, as the file writes it.
    pub name: String,
    /// The id of the motor configuration flown.
    pub configuration: String,
    /// Its name, as the file writes it; often empty.
    pub configuration_name: String,
    /// Every motor configuration the file holds, the one flown among them, before `--motor`
    /// changed any.
    pub configurations: Vec<DesignConfiguration>,
}

/// A motor configuration of the design file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DesignConfiguration {
    /// Its id, which `--config` takes.
    pub id: String,
    /// Its name, as the file writes it; often empty.
    pub name: String,
    /// Whether `hpr sim` flies it as the file has it, without `--motor`.
    pub flies: bool,
}

/// A design file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DesignFormat {
    /// An OpenRocket `.ork` file.
    Ork,
    /// An hpr design file: a `hpr_design::Rocket` as JSON.
    HprJson,
}

/// One motor flown.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct SimMotor {
    /// Its designation, such as `168H54-10A`.
    pub designation: String,
    /// The id of the motor mount it is in, which `--mount` takes.
    pub mount: String,
    /// The mount's name, as the file writes it.
    pub mount_name: String,
    /// How many of it fly: one per tube of a cluster, and one per pod of a pod set.
    pub count: usize,
    /// How many of those never light: in a tube the design marks failed, or set never to light.
    pub unlit: usize,
    /// Where it comes from.
    pub source: SimMotorSource,
    /// When it lights, in words: `at launch`, or as the design says.
    pub ignition: String,
}

/// Where a flown motor comes from.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SimMotorSource {
    /// The design file's own configuration.
    Design,
    /// The bundled catalog, named with `--motor`.
    Catalog,
    /// A motor file named with `--motor`.
    File {
        /// The file's name, without its folder.
        file: String,
        /// The file's format: `eng` or `rse`.
        format: FileFormat,
    },
}

/// Where and how the rocket was launched.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Launch {
    /// The site's latitude, degrees north.
    pub latitude_deg: f64,
    /// The site's longitude, degrees east.
    pub longitude_deg: f64,
    /// The site's elevation above sea level, m.
    pub elevation_m: f64,
    /// The rail's length, m, from the rocket's aft end at the start to the rail's top.
    pub rail_length_m: f64,
    /// The rail's angle above the horizon, degrees: 90 is vertical.
    pub inclination_deg: f64,
    /// The direction the rail leans toward, clockwise from true north, degrees.
    pub heading_deg: f64,
    /// The wind's speed, m/s, the same at every height; 0 for calm air.
    pub wind_speed_m_s: f64,
    /// The direction the wind blows from, clockwise from true north, degrees.
    pub wind_from_deg: f64,
    /// The atmosphere: always `standard`, the 1976 US Standard Atmosphere.
    pub atmosphere: String,
}

/// A flight's metrics, as the library's `hpr_sim::metrics::FlightSummary` gives them. Heights are
/// the centre of gravity's above the launch site; speeds are relative to the ground.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Summary {
    /// Why the flight ended.
    pub termination: Termination,
    /// The centre of gravity's height above the launch site at launch, m: not zero, as the rocket
    /// stands on the rail.
    pub launch_height_m: Option<f64>,
    /// The speed as the rocket left the rail, m/s.
    pub rail_exit_speed_m_s: Option<Peak>,
    /// The highest point.
    pub apogee: Option<Apogee>,
    /// The top speed, m/s.
    pub max_speed_m_s: Option<Peak>,
    /// The top Mach number.
    pub max_mach: Option<Peak>,
    /// The top dynamic pressure, Pa.
    pub max_dynamic_pressure_pa: Option<Peak>,
    /// The top acceleration of the nose tip relative to the launch site's frame, from liftoff
    /// until a recovery device opens, m/s²: the motion's, not what an accelerometer reads.
    pub max_acceleration_m_s2: Option<Peak>,
    /// The top acceleration under the recovery devices, m/s²: the opening shock.
    pub max_descent_acceleration_m_s2: Option<Peak>,
    /// The least static stability margin, calibres, from the rail exit to apogee or the first
    /// deployment.
    pub min_static_margin_cal: Option<Peak>,
    /// The least flight margin, calibres, over the same span: the margin at the flight's Mach
    /// number, along the axis.
    pub min_flight_margin_cal: Option<Peak>,
    /// The stability as the rocket left the rail.
    pub rail_exit_stability: Option<Stability>,
    /// Where and how fast the rocket landed. With no recovery device flown, as `hpr sim` flies
    /// today, the fall from apogee rests on small-angle aerodynamics far outside their range: not
    /// a prediction (the notes say so).
    pub landing: Option<Landing>,
    /// Where each part that came apart from the rocket landed, if any did.
    pub body_landings: Vec<Landing>,
}

/// Why a flight ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Termination {
    /// The centre of mass reached the ground.
    GroundHit,
    /// Every motor burned out before the rocket lifted off.
    NoLiftoff,
    /// The rocket lifted off but stopped on the rail after every motor burned out.
    StalledOnRail,
    /// The time cap was reached.
    TimeCap,
    /// The integrator's step limit was reached.
    StepLimit,
    /// The stack separated, and each part flew on as its own descent.
    Separated,
    /// A way of ending this build of `hpr` doesn't name.
    Other,
}

/// A metric's extreme and when it came.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct Peak {
    /// The value, in the unit the field's name gives.
    pub value: f64,
    /// When, s after launch.
    pub time_s: f64,
    /// The height above the launch site then, m.
    pub height_above_ground_m: f64,
}

/// The highest point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct Apogee {
    /// When, s after launch.
    pub time_s: f64,
    /// The height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The height gained from where the centre of gravity stood at launch, m, as OpenRocket's
    /// altitude counts.
    pub gain_m: Option<f64>,
}

/// The rocket's stability at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct Stability {
    /// When, s after launch.
    pub time_s: f64,
    /// The height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The dynamic pressure, Pa.
    pub dynamic_pressure_pa: f64,
    /// The centre of gravity, m aft of the nose tip.
    pub cg_station_m: f64,
    /// The reference diameter the margins are counted in, m.
    pub reference_diameter_m: f64,
    /// The static margin: the air along the axis, at Mach 0.
    pub static_margin: Margin,
    /// The flight margin: the air along the axis, at the flight's Mach number. The angle of attack
    /// is left out.
    pub flight_margin: Margin,
}

/// A stability margin and what it rests on.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct Margin {
    /// The Mach number.
    pub mach: f64,
    /// The total angle of attack, rad.
    pub angle_of_attack_rad: f64,
    /// The normal-force slope `C_Nα` on the reference area, per radian.
    pub normal_force_slope_per_rad: f64,
    /// The sum of the parts' slope magnitudes, per radian: the scale the net slope is judged by.
    pub slope_magnitude_sum_per_rad: f64,
    /// The pitch-moment slope about the centre of mass, per radian; negative restores.
    pub pitch_moment_slope_per_rad: f64,
    /// The centre of pressure, m aft of the nose tip; `null` when the margin is.
    pub cp_station_m: Option<f64>,
    /// The margin, calibres: positive with the centre of pressure aft of the centre of mass;
    /// `null` when the net slope is too small for the quotient to mean anything.
    pub margin_cal: Option<f64>,
}

/// Where and how fast a body landed: where its centre of mass came down to the launch site's
/// height on the ellipsoid. There is no terrain.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct Landing {
    /// The body: `null` for the rocket, or the index of a part that came apart from it.
    pub body: Option<usize>,
    /// When, s after launch.
    pub time_s: f64,
    /// Latitude, degrees north.
    pub latitude_deg: f64,
    /// Longitude, degrees east.
    pub longitude_deg: f64,
    /// Distance east of the launch site, m.
    pub east_m: f64,
    /// Distance north of the launch site, m.
    pub north_m: f64,
    /// Horizontal distance from the launch site, m.
    pub distance_m: f64,
    /// The speed at the ground, m/s.
    pub ground_hit_speed_m_s: f64,
    /// The rate of descent at the ground, m/s.
    pub descent_rate_m_s: f64,
}

/// One event of a flight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct SimEvent {
    /// What happened.
    pub kind: EventKind,
    /// The device, part or motor it is about, by its index, for the kinds that have one.
    pub index: Option<usize>,
    /// When, s after launch.
    pub time_s: f64,
    /// The centre of gravity's height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The speed relative to the ground, m/s.
    pub speed_m_s: f64,
}

/// What happened at an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// The rocket started to move along the rail.
    Liftoff,
    /// The rocket left the rail.
    RailExit,
    /// Every motor lit, or due to light at a known time, has burned out.
    Burnout,
    /// The highest point.
    Apogee,
    /// The centre of mass came down to the launch site's height.
    GroundHit,
    /// A recovery device's charge fired.
    Trigger,
    /// A recovery device deployed.
    Deployment,
    /// A recovery device was released.
    Release,
    /// The stack came apart.
    Separation,
    /// A piece left the airframe at an ejection.
    Ejection,
    /// A part started to move along the airframe.
    Shift,
    /// A part left the airframe.
    MassRelease,
    /// A user event.
    User,
    /// A motor lit after launch.
    Ignition,
    /// An event this build of `hpr` doesn't name.
    Other,
}

/// A recording file written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct Export {
    /// The path as given.
    pub path: String,
    /// The file's format.
    pub format: ExportFormat,
    /// The rows recorded: one every `--interval` seconds and one at every event.
    pub rows: usize,
}

/// A recording file's format, from its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// `.csv`: a header of column names with their units, then one line per row.
    Csv,
    /// `.json`: `{"columns": [...], "rows": [[...], ...]}`.
    Json,
    /// `.parquet`: Apache Parquet.
    Parquet,
    /// `.geojson`: the centre of mass's path on the Earth, with the flight's landmarks.
    Geojson,
    /// `.kml`: the same path, for Google Earth.
    Kml,
}

/// Something a reader accepted with a caveat, or the design's checks found unusual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct InputWarning {
    /// Where: in a `.ork` file, a path of element names from the root or a zip entry's name; in a
    /// motor file, its name and line; `design checks` for a check's finding.
    pub at: String,
    /// How serious it is.
    pub kind: WarningKind,
    /// What was found, and how it was read.
    pub message: String,
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
    /// The command that failed; `null` for a usage error, where the command line may name none.
    pub command: Option<String>,
    /// For `not_available`, the milestone that brings the command, such as `M4.2b`.
    pub milestone: Option<String>,
}

/// What kind of failure an error document reports, with its exit status.
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
        ("sim.schema.json", schema::<SimFlight>()),
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
