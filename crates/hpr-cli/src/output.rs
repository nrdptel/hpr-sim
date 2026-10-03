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

/// `hpr motors search`: motor.fusionspace.co's motors that pass the filters, with their stock and
/// prices.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct MotorSearch {
    /// The credits, shown with every list: motor.fusionspace.co's, as its data licence (CC BY
    /// 4.0) asks, with its caution to check stock and price on the vendor's own page; then
    /// ThrustCurve.org's, whose figures the site repeats.
    pub attribution: Vec<String>,
    /// Where the list was read from.
    pub read_from: ReadFrom,
    /// When motor.fusionspace.co built the list, ISO 8601 UTC as the site writes it.
    pub generated_at: String,
    /// The motors that pass the filters, cheapest first: by one motor's price at the cheapest
    /// vendor with it in stock, then by maker and designation. Motors with no such price in U.S.
    /// dollars come last, by maker and designation.
    pub motors: Vec<FoundMotor>,
}

/// One motor that passed `hpr motors search`'s filters, as motor.fusionspace.co lists it. The
/// figures are ThrustCurve.org's published ones, which the site repeats.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct FoundMotor {
    /// The manufacturer, as the site names it, such as `Cesaroni Technology`.
    pub manufacturer: String,
    /// The designation, as ThrustCurve.org spells it, such as `3683L851-P`.
    pub designation: String,
    /// The designation without its propellant code, such as `L851`.
    pub common_name: Option<String>,
    /// The impulse class, such as `L`.
    pub impulse_class: String,
    /// `single_use`, `reload` or `hybrid`, when the site says.
    pub motor_type: Option<MotorKind>,
    /// Diameter, mm.
    pub diameter_mm: f64,
    /// Total impulse, N·s, as stated.
    pub total_impulse_ns: Option<f64>,
    /// Average thrust, N, as stated.
    pub average_thrust_n: Option<f64>,
    /// Burn time, s, as stated.
    pub burn_time_s: Option<f64>,
    /// The propellant's trade name, such as `White Lightning`.
    pub propellant: Option<String>,
    /// The delays it comes with, s, comma-separated, or `P` for plugged.
    pub delays: Option<String>,
    /// In stock at one vendor or more.
    pub in_stock: bool,
    /// How many vendors have it in stock.
    pub in_stock_vendor_count: u32,
    /// The vendor with it in stock at the lowest price of one motor; null when out of stock.
    pub cheapest_in_stock: Option<FoundOffer>,
}

/// A vendor's offer of a motor in stock, as motor.fusionspace.co lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct FoundOffer {
    /// The vendor's name.
    pub vendor: String,
    /// The product page, where stock and price are the vendor's to say.
    pub url: String,
    /// The price of one motor, in hundredths of the currency (cents), when the page shows one.
    pub unit_price_cents: Option<u64>,
    /// The price of the pack, in hundredths of the currency, when the page shows one.
    pub price_cents: Option<u64>,
    /// How many motors the pack holds.
    pub pack_size: u32,
    /// The currency, such as `USD`.
    pub currency: String,
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
    /// A rocket's JSON (`.json`): a `hpr_design::Rocket` alone.
    HprJson,
    /// A document of the hpr design format (`.hpr`).
    Hpr,
    /// A design in the hpr design format's zip container (`.hprz`).
    Hprz,
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

/// A flight's metrics, as the library's `hpr_sim::metrics::FlightSummary` gives them, each peak
/// marked if it came after apogee. Heights are the centre of gravity's above the launch site;
/// speeds are relative to the ground.
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
    /// Whether it came after apogee, in the fall. With no recovery device flown, as `hpr sim`
    /// flies today, a peak in the fall is not a prediction: the fall rests on small-angle
    /// aerodynamics far outside their range.
    pub after_apogee: bool,
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
    /// `.geojson`: the centre of mass's path on the Earth.
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

/// `hpr convert`: a motor file converted, which has `motors`, or a design, which has `rocket`.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Convert {
    /// Motors, written as a `.eng` or `.rse` file.
    Motors(ConvertMotors),
    /// A design, written as a `.ork`, `.hpr` or `.hprz` file.
    Design(ConvertDesign),
}

/// `hpr convert` of motors: the motors read, and the file they were written to.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConvertMotors {
    /// Where the motors came from: a motor file, or the bundled catalog.
    pub input: MotorSource,
    /// The file written.
    pub output: ConvertedFile,
    /// The motors written, by the names the file gives them, in its order.
    pub motors: Vec<String>,
    /// What the reader flagged in the input, then what the conversion dropped or wrote
    /// differently, such as `.rse` figures a `.eng` file has no place for.
    pub warnings: Vec<Warning>,
}

/// `hpr convert` of a design: the design read, and the file it was written to.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct ConvertDesign {
    /// The design file read.
    pub input: DesignFilePath,
    /// The design file written.
    pub output: DesignFilePath,
    /// The rocket's name, as the design writes it.
    pub rocket: String,
    /// How many motor configurations the design holds, flyable or not.
    pub configurations: usize,
    /// The version of the hpr design format a `.hpr` or `.hprz` input was written in, when it was
    /// older than the version written and was migrated to it; `null` otherwise.
    pub migrated_from: Option<String>,
    /// The files written into a `.hprz` beside the design, by name, in order; empty for any other
    /// output.
    pub attachments: Vec<String>,
    /// What the `.ork` reader flagged in the input, what the `.ork` writer flagged in the output,
    /// and each attachment of a `.hprz` input that the output has no place for.
    pub warnings: Vec<InputWarning>,
}

/// A design file `hpr convert` read or wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct DesignFilePath {
    /// The path as given.
    pub path: String,
    /// Its format: `ork`, `hpr` or `hprz`.
    pub format: DesignFormat,
}

/// A motor file `hpr convert` wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ConvertedFile {
    /// The path as given.
    pub path: String,
    /// Its format: `eng` or `rse`.
    pub format: FileFormat,
}

/// `hpr validate`: every validation case run, and the run held to the committed reports.
///
/// Printed whether or not the check passes; when it fails, the exit status is 1.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Validate {
    /// Whether the check passed: every scored metric within its tolerance, the run reproducing
    /// the committed report, and the committed reports holding to the accepted census. The
    /// same as `cargo xtask validate --check`.
    pub passed: bool,
    /// How each case came out, in the order the case lock runs them.
    pub cases: Vec<ValidateCase>,
    /// The whole run, counted.
    pub totals: ValidateTotals,
    /// Whether the run reproduces `validation/reports/latest.{md,json}` to the digits the
    /// platforms share.
    pub reproduced: bool,
    /// The committed reports against the accepted census; `null` when it couldn't be taken,
    /// which `problems` says why.
    pub census: Option<CensusHeld>,
    /// Why the check failed, one reason each; empty when it passed.
    pub problems: Vec<String>,
}

/// How one validation case came out.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ValidateCase {
    /// A known gap: hpr refuses the flight, so nothing is compared.
    Gap {
        /// The case's id.
        case: String,
        /// The metrics the case would compare.
        metrics: usize,
        /// Why hpr refuses it.
        refusal: String,
    },
    /// Predicted mode: every metric reported against a target, none gating.
    Predicted {
        /// The case's id.
        case: String,
        /// The metrics reported.
        metrics: usize,
        /// How many are within their target.
        within_target: usize,
        /// The metric furthest from its reference; `null` if none has a relative difference.
        largest: Option<LargestDifference>,
    },
    /// Every metric held to its tolerance.
    Scored {
        /// The case's id.
        case: String,
        /// The metrics compared.
        metrics: usize,
        /// The largest relative difference among the scored metrics, as a fraction (0.01 is 1%),
        /// without its sign.
        worst_scored: f64,
        /// The metrics the case declares not scored.
        not_scored: Vec<String>,
        /// How many metrics are outside their tolerance.
        failed: usize,
    },
}

/// A predicted case's largest difference.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct LargestDifference {
    /// The metric, such as `apogee_m`.
    pub metric: String,
    /// hpr's value against the reference's, as a signed fraction: 0.01 is 1% above it.
    pub relative: f64,
}

/// A validation run, counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ValidateTotals {
    /// The cases run.
    pub cases: usize,
    /// The metrics compared or reported.
    pub metrics: usize,
    /// The metrics their cases declare not scored.
    pub not_scored: usize,
    /// The metrics reported against a target (predicted mode).
    pub predicted: usize,
    /// How many of those are outside their target.
    pub outside_target: usize,
    /// The scored metrics outside their tolerance.
    pub failed: usize,
}

/// The committed reports against the accepted accuracy census.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct CensusHeld {
    /// The accepted census's rows.
    pub rows: usize,
    /// Each row that differs from it, in words.
    pub changes: Vec<String>,
    /// How many of those differ for the worse.
    pub worse: usize,
    /// Each file written from the census whose committed text isn't what it writes.
    pub stale: Vec<String>,
}

/// `hpr weather`: a site's air and wind, level by level, from one source.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Weather {
    /// The source, and the credit its terms ask for wherever the data is shown.
    pub source: WeatherSource,
    /// Where the source's answer was read from.
    pub read_from: ReadFrom,
    /// Where the profile is: Open-Meteo's grid point, the balloon's release, or the site the
    /// forecast or file was read at.
    pub position: WeatherPosition,
    /// The time the profile is for, UTC, `YYYY-MM-DDTHH:MM:SSZ`: the launch time asked for, the
    /// balloon's release, or the time a forecast run is for.
    pub time: String,
    /// The start of the forecast run, for GFS and RAP.
    pub run: Option<String>,
    /// The profile's levels, lowest first: the ground, then the levels above it; ERA5's levels
    /// are pressure levels only, from 1000 hPa up, some of them below the ground where it is high.
    pub levels: Vec<ProfileLevel>,
    /// The levels the source gave but the profile leaves out, and why.
    pub dropped: Vec<DroppedLevel>,
    /// The file the profile was written to, as given.
    pub profile: Option<String>,
}

/// A weather source and its credit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct WeatherSource {
    /// Which source.
    pub name: WeatherSourceName,
    /// The credit to show with its data.
    pub attribution: String,
}

/// The weather sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeatherSourceName {
    /// Open-Meteo's forecast or historical-forecast API.
    OpenMeteo,
    /// The University of Wyoming's radiosonde archive.
    Wyoming,
    /// NOAA's GFS forecast, from NOMADS.
    Gfs,
    /// NOAA's RAP forecast, from NOMADS.
    Rap,
    /// An ECMWF ERA5 pressure-level file.
    Era5,
}

/// Where a source's answer was read from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadFrom {
    /// A file given on the command line: `--from`, or an ERA5 file.
    File {
        /// The path as given.
        path: String,
    },
    /// Fetched now, and saved in the cache.
    Network {
        /// When, in seconds since the Unix epoch.
        fetched_at_unix_s: u64,
    },
    /// The cache, still fresh.
    Cache {
        /// When the copy was fetched, in seconds since the Unix epoch.
        fetched_at_unix_s: u64,
    },
    /// The cache, older than the source keeps a copy fresh: offline, or the fetch failed.
    StaleCache {
        /// When the copy was fetched, in seconds since the Unix epoch.
        fetched_at_unix_s: u64,
        /// Online, why the fetch failed or its answer was refused.
        reason: Option<String>,
    },
}

/// Where a weather profile is.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct WeatherPosition {
    /// Degrees north.
    pub latitude_deg: f64,
    /// Degrees east.
    pub longitude_deg: f64,
}

/// One level of a weather profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct ProfileLevel {
    /// Geometric height above mean sea level, m.
    pub height_msl_m: f64,
    /// Pressure, Pa.
    pub pressure_pa: Option<f64>,
    /// Temperature, K.
    pub temperature_k: f64,
    /// Relative humidity over liquid water, a fraction; absent where the source gives none
    /// (ERA5 as read, dry air).
    pub relative_humidity: Option<f64>,
    /// Wind speed, m/s.
    pub wind_speed_m_s: Option<f64>,
    /// The direction the wind blows from, degrees clockwise from true north.
    pub wind_from_deg: Option<f64>,
}

/// A level a weather source gave that the profile leaves out.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct DroppedLevel {
    /// Its pressure, Pa, for a forecast's pressure level.
    pub pressure_pa: Option<f64>,
    /// Its line in the answer, for a sounding's row (the header is line 1).
    pub line: Option<usize>,
    /// Why it was left out.
    pub reason: DropReason,
}

/// Why a weather level was left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    /// Its pressure is not below the ground's, or its height not above it: the model's values
    /// under high ground.
    BelowGround,
    /// A value is missing.
    NoData,
    /// A value is out of range.
    OutOfRange,
    /// A sounding's row with the same pressure as its neighbours, not the middle one.
    SamePressure,
    /// A sounding's row not above the last one kept.
    NotAbove,
    /// A sounding's row whose height doesn't fit the thickness its pressure and temperature give.
    Thickness,
    /// A reason this build of `hpr` doesn't name.
    Other,
}

impl DropReason {
    /// A few words for the text output.
    pub fn describe(self) -> &'static str {
        match self {
            Self::BelowGround => "below the ground",
            Self::NoData => "a value is missing",
            Self::OutOfRange => "a value is out of range",
            Self::SamePressure => "a repeat of its pressure",
            Self::NotAbove => "not above the row before",
            Self::Thickness => "its height doesn't fit the layer's thickness",
            Self::Other => "another reason",
        }
    }
}

/// `hpr analyze`: a flight log's readings, taken from the log alone, with no design file and no
/// simulation. Heights are metres above the logger's own zero, which a PerfectFlite takes on the
/// pad; times are seconds on the log's clock.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct Analyze {
    /// The log read.
    pub log: AnalyzedLog,
    /// What the file states about the flight: the logger's own figures, printed beside hpr's
    /// readings and never in place of them.
    pub stated: LoggerStated,
    /// How the readings were taken.
    pub method: AnalyzeMethod,
    /// Liftoff.
    pub liftoff: LogReading<LiftoffReading>,
    /// The highest point.
    pub apogee: LogReading<ApogeeReading>,
    /// The top vertical speed from liftoff to apogee.
    pub max_speed: LogReading<MaxSpeedReading>,
    /// The top acceleration.
    pub max_acceleration: LogReading<MaxAccelerationReading>,
    /// Landing, and the descent before it.
    pub landing: LogReading<LandingReading>,
}

/// The log `hpr analyze` read.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct AnalyzedLog {
    /// The file, as given.
    pub path: String,
    /// Its format.
    pub format: LogFormatName,
    /// The logger, as the file names it.
    pub logger: String,
    /// The logger's serial number, as the file states it.
    pub serial_number: Option<String>,
    /// The logger's firmware version, as the file states it.
    pub firmware: Option<String>,
    /// The flight's number in the logger's memory, as the file states it.
    pub flight_number: Option<u32>,
    /// How many samples the file holds.
    pub samples: usize,
    /// The first sample's time, s.
    pub first_time_s: f64,
    /// The last sample's time, s.
    pub last_time_s: f64,
    /// What the reader noticed and worked around, such as a sample count that differs from the
    /// one the file states.
    pub notes: Vec<String>,
}

/// A flight log's format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogFormatName {
    /// PerfectFlite's `.pf2`: the Pnut, the StratoLogger and the StratoLoggerCF.
    PerfectFlitePf2,
    /// A format this build of `hpr` doesn't name.
    Other,
}

/// What a flight log states about the flight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct LoggerStated {
    /// The apogee the logger computed, m above its own zero; `null` if the file states none, or
    /// states something that isn't a height, such as a PerfectFlite's `PWRLOSS`.
    pub apogee_m: Option<f64>,
    /// The launch site's elevation, m above mean sea level, as the logger states it.
    pub ground_elevation_msl_m: Option<f64>,
}

/// How the readings were taken. Every field but `altitude_resolution_m` is `null` for a log too
/// short to read, or one withheld whole as `bad_record`; all but it and `sample_interval_s` for
/// one withheld as `sampled_too_fast`; and `pad_altitude_m` for one withheld as `no_climb`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct AnalyzeMethod {
    /// The median interval between samples, s.
    pub sample_interval_s: Option<f64>,
    /// The running median's span, s: every height and time is read from the altitude after it.
    pub median_window_s: Option<f64>,
    /// How far below its true peak the running median can read a peak bent by gravity alone, m.
    pub peak_bound_m: Option<f64>,
    /// The altitude's resolution in the log's format, m: a PerfectFlite writes whole feet.
    pub altitude_resolution_m: f64,
    /// The pad: the median of the altitude before it first rises 1 m, m.
    pub pad_altitude_m: Option<f64>,
}

/// A reading, or why the log can't support it.
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LogReading<T> {
    /// The reading.
    Read(T),
    /// The log can't support the reading.
    Withheld(WithheldReading),
}

/// Why a reading was withheld.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct WithheldReading {
    /// The reason, as a code.
    pub reason: WithheldReason,
    /// The reason in words, with the log's own numbers.
    pub detail: String,
}

/// The reason a reading was withheld.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WithheldReason {
    /// The log has fewer than three samples.
    TooShort,
    /// The altitude never climbs 3 m above where the log starts.
    NoClimb,
    /// The pad, the median altitude before the first metre of rise, is more than 3 m from the
    /// logger's zero: the log didn't start on the pad.
    StartsOffThePad,
    /// The log ends before the rocket is seen to land.
    EndsBeforeLanding,
    /// The altitude reaches the ground sooner than a fall from rest at apogee in vacuum could.
    FasterThanFreeFall,
    /// The log has no speed column.
    NoSpeedColumn,
    /// The top speed is above 4,000 m/s.
    ImplausibleSpeed,
    /// The climb's speed swings negative by more than 20% of its top.
    NoisySpeed,
    /// The top speed falls on the liftoff sample itself.
    SpeedPeakAtLiftoff,
    /// The log has no accelerometer.
    NoAccelerometer,
    /// The reading needs another, which was withheld.
    Needs,
    /// The record breaks what every reader guarantees; only a record built by hand can.
    BadRecord,
    /// The samples come so often that the 0.3 s median would hold more than 1,000 either side.
    SampledTooFast,
    /// A reason this build of `hpr` doesn't name.
    Other,
}

/// Where a reading's value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReadingSource {
    /// The logger's barometric altitude, after the running median.
    Barometer,
    /// A speed column the logger computed from its own barometric altitude.
    LoggerSpeedFromBarometer,
    /// A source this build of `hpr` doesn't name.
    Other,
}

/// Liftoff.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct LiftoffReading {
    /// The last sample on the pad, s: the rocket had risen less than the altitude's resolution
    /// then, and rose past it within one sample interval after.
    pub time_s: f64,
    /// Where it came from.
    pub source: ReadingSource,
}

/// The highest point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct ApogeeReading {
    /// When, s.
    pub time_s: f64,
    /// When, s after liftoff; `null` if liftoff was withheld.
    pub time_after_liftoff_s: Option<f64>,
    /// The filtered altitude there, m.
    pub altitude_m: f64,
    /// Whether the log may have ended before the peak, so the altitude is a floor.
    pub is_floor: bool,
    /// The highest sample the log holds before the filter: above the apogee when the median set
    /// a pulse aside.
    pub highest_sample: HighestSample,
    /// Where it came from.
    pub source: ReadingSource,
}

/// The highest sample of the altitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct HighestSample {
    /// When, s.
    pub time_s: f64,
    /// The altitude, m.
    pub altitude_m: f64,
}

/// The top vertical speed from liftoff to apogee.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct MaxSpeedReading {
    /// The speed, m/s, up.
    pub speed_m_s: f64,
    /// When, s.
    pub time_s: f64,
    /// The filtered altitude then, m.
    pub altitude_m: f64,
    /// Where it came from.
    pub source: ReadingSource,
}

/// The top acceleration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct MaxAccelerationReading {
    /// The acceleration, m/s².
    pub acceleration_m_s2: f64,
    /// When, s.
    pub time_s: f64,
}

/// Landing, and the descent before it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
pub struct LandingReading {
    /// The first sample within 2 m of the pad that stays under 5 m for a second, s: before
    /// touchdown by the time the last 2 m took.
    pub time_s: f64,
    /// From liftoff to landing, s.
    pub flight_time_s: f64,
    /// From apogee to landing, s.
    pub descent_time_s: f64,
    /// The mean rate of descent from apogee to landing, m/s: the height lost over the time
    /// taken, drogue and main together.
    pub mean_descent_rate_m_s: f64,
    /// Where it came from.
    pub source: ReadingSource,
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
        ("motors-search.schema.json", schema::<MotorSearch>()),
        ("sim.schema.json", schema::<SimFlight>()),
        ("completions.schema.json", schema::<Completions>()),
        ("convert.schema.json", schema::<Convert>()),
        ("validate.schema.json", schema::<Validate>()),
        ("analyze.schema.json", schema::<Analyze>()),
        ("weather.schema.json", schema::<Weather>()),
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
