//! Motor records and thrust curves from [ThrustCurve.org](https://www.thrustcurve.org)'s API, and
//! the join that gives a motor in stock its curve.
//!
//! ThrustCurve.org aims to hold every certified hobby motor's published figures and the simulator
//! files (thrust curves) people have contributed for them. Its [API][api] answers two questions: a
//! *search* ([`Search`], read by [`parse_search`]), which lists motor records, each with
//! ThrustCurve's own id, its `motorId`; and a *download* ([`Download`], read by
//! [`parse_download`]), which gives a motor's data files, each a RASP (`.eng`) or RockSim (`.rse`)
//! file that [`DataFile::read`] hands to `hpr_motor`'s readers. The `fetch_` functions ask a
//! [`Client`] for the answer, so it comes from the cache when it can, and offline from the cache
//! only; an answer that doesn't parse is never cached. Motor records and curves change seldom, so
//! a copy stays fresh for [`TTL_S`], a day. Show [`ATTRIBUTION`] (it is on every [`Fetched`])
//! wherever the data is shown.
//!
//! [`join`] maps the motor finder's motors ([`crate::motor_finder`]) to ThrustCurve records. The
//! finder carries no ThrustCurve id, but spells each designation as ThrustCurve does, so a motor
//! maps when exactly one record has its manufacturer's full name and its designation, character
//! for character. Anything else is a [`Miss`], with its reason, and [`Join::report`] lists them.
//! [`fetch_finder_records`] fetches the records the join needs: one search per maker the finder
//! reads.
//!
//! **How far to trust it:** a value is the answer's, unchanged (`tests/thrustcurve.rs` reads every
//! field of every recorded answer back). A curve is the file a contributor uploaded, as
//! ThrustCurve serves it, read by the same readers as a file on disk. The join is by name only: it
//! doesn't compare impulse or diameter, so a finder motor whose designation ThrustCurve spells
//! differently is a miss, not a wrong match. On the recorded answers of 2026-10-01, 282 of the
//! finder's 282 motors in stock mapped, each to one record.
//!
//! ```
//! use hpr_net::thrustcurve;
//!
//! // AeroTech's J450DM curve as ThrustCurve.org served it on 2026-10-01.
//! let body = include_bytes!("../tests/fixtures/replay/thrustcurve-download-J450DM.json");
//! let answer = thrustcurve::parse_download(body)?;
//! let curve = answer.results[0].read()?.thrust_curve()?;
//! // The file's 36 points, after the curve's start at zero thrust.
//! assert_eq!((curve.times_s().len(), curve.end_time_s()), (37, 2.311));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [api]: https://www.thrustcurve.org/info/api.html

use std::collections::BTreeMap;
use std::fmt::Write as _;

use base64::Engine as _;
use hpr_motor::text::Parsed;
use hpr_motor::{MotorError, ThrustCurve, eng, rse};
use serde::{Deserialize, Serialize};

use crate::motor_finder::{self, MANUFACTURERS};
use crate::{Client, Fetched, NetError, Source, Transport};

/// The API's base URL, version 1 of its JSON endpoints.
pub const BASE_URL: &str = "https://www.thrustcurve.org/api/v1";

/// How long a cached answer counts as fresh, s: a day. Motor records and curves change seldom.
pub const TTL_S: u64 = 86_400;

/// The most records a search asks for. ThrustCurve's whole database held 1,156 motors on
/// 2026-10-01; a search that matches more than it returns is refused by
/// [`fetch_finder_records`].
pub const MAX_RESULTS: u32 = 5_000;

/// The credit for ThrustCurve.org's data.
pub const ATTRIBUTION: &str = "Motor data and thrust curves courtesy of ThrustCurve.org, \
                               https://www.thrustcurve.org/";

/// A motor search. Every field set narrows it; the API joins them with "and". Start from
/// [`Search::manufacturer`] or `Search::default()` and set the fields wanted: more may be added.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Search {
    /// The manufacturer's name or abbreviation (`AeroTech`, `Cesaroni Technology`, `Loki`).
    pub manufacturer: Option<String>,
    /// The manufacturer's designation (`J450DM`, `F27R/L`).
    pub designation: Option<String>,
    /// The common name (`J450`).
    pub common_name: Option<String>,
    /// The impulse class letter (`L`).
    pub impulse_class: Option<String>,
    /// The most records to return; the API's own default when `None`.
    pub max_results: Option<u32>,
}

impl Search {
    /// Every motor of one manufacturer, up to [`MAX_RESULTS`].
    #[must_use]
    pub fn manufacturer(name: &str) -> Self {
        Self {
            manufacturer: Some(name.to_owned()),
            max_results: Some(MAX_RESULTS),
            ..Self::default()
        }
    }

    /// The search's URL under [`BASE_URL`]: its fields as query parameters, in the order of the
    /// struct, each value percent-encoded; no query at all when no field is set.
    #[must_use]
    pub fn url(&self) -> String {
        let max_results = self.max_results.map(|n| n.to_string());
        let fields = [
            ("manufacturer", self.manufacturer.as_deref()),
            ("designation", self.designation.as_deref()),
            ("commonName", self.common_name.as_deref()),
            ("impulseClass", self.impulse_class.as_deref()),
            ("maxResults", max_results.as_deref()),
        ];
        let query: Vec<String> = fields
            .iter()
            .filter_map(|(name, value)| Some(format!("{name}={}", encode(value.as_ref()?))))
            .collect();
        if query.is_empty() {
            return format!("{BASE_URL}/search.json");
        }
        format!("{BASE_URL}/search.json?{}", query.join("&"))
    }
}

/// A data file's format, as the API names it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Format {
    /// RASP, a `.eng` file.
    #[serde(rename = "RASP")]
    Rasp,
    /// RockSim, a `.rse` file.
    #[serde(rename = "RockSim")]
    RockSim,
}

impl Format {
    /// The API's word for it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rasp => "RASP",
            Self::RockSim => "RockSim",
        }
    }
}

/// A request for one motor's data files, in one format. Build it with [`Download::new`], which
/// checks the motor id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Download {
    motor_id: String,
    format: Format,
}

impl Download {
    /// A motor's data files in `format`, by its ThrustCurve id, in either case: it is kept in
    /// lower case, as the API writes ids, so one motor has one URL and its answer matches.
    ///
    /// # Errors
    /// [`ThrustCurveError::Request`] for an id that isn't 24 hexadecimal digits, the form the
    /// API's ids take.
    pub fn new(motor_id: &str, format: Format) -> Result<Self, ThrustCurveError> {
        if !is_motor_id(motor_id) {
            return Err(ThrustCurveError::Request {
                what: "motor id",
                value: motor_id.to_owned(),
            });
        }
        Ok(Self {
            motor_id: motor_id.to_ascii_lowercase(),
            format,
        })
    }

    /// The motor's ThrustCurve id, in lower case.
    #[must_use]
    pub fn motor_id(&self) -> &str {
        &self.motor_id
    }

    /// The format asked for.
    #[must_use]
    pub fn format(&self) -> Format {
        self.format
    }

    /// The request's URL under [`BASE_URL`], asking for the files themselves (`data=file`).
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "{BASE_URL}/download.json?motorId={}&format={}&data=file",
            self.motor_id,
            self.format.as_str()
        )
    }
}

/// The [`Source`] a [`Client`] caches every answer under: ThrustCurve.org, its [`ATTRIBUTION`],
/// and [`TTL_S`].
#[must_use]
pub fn source() -> Source {
    Source {
        name: "ThrustCurve.org".to_owned(),
        attribution: ATTRIBUTION.to_owned(),
        ttl_s: TTL_S,
    }
}

/// A search's answer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchAnswer {
    /// Each criterion asked, with how many motors it matches alone.
    pub criteria: Vec<Criterion>,
    /// How many motors match every criterion: more than [`SearchAnswer::results`] holds when the
    /// search's `maxResults` cut it short.
    pub matches: u32,
    /// The motors returned.
    pub results: Vec<MotorRecord>,
    /// The page of the site's own search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

impl SearchAnswer {
    /// Whether it holds every motor that matched.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        usize::try_from(self.matches).ok() == Some(self.results.len())
    }
}

/// One search criterion, as the answer echoes it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    /// Its name (`manufacturer`).
    pub name: String,
    /// Its value, as text.
    pub value: String,
    /// How many motors it matches alone.
    pub matches: u32,
}

/// One motor's record. The API returns only the fields that have values, so all but the id,
/// manufacturer and designation are optional. Units are the API's: newtons, newton-seconds,
/// seconds, millimetres and grams.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotorRecord {
    /// ThrustCurve's id, 24 hexadecimal digits: what a [`Download`] names.
    pub motor_id: String,
    /// The manufacturer's full name (`Cesaroni Technology`).
    pub manufacturer: String,
    /// Its short name (`Cesaroni`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_abbrev: Option<String>,
    /// The manufacturer's designation (`3683L851-P`).
    pub designation: String,
    /// The common name (`L851`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub common_name: Option<String>,
    /// The impulse class letter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impulse_class: Option<String>,
    /// The certifying body's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_org: Option<String>,
    /// Diameter, mm.
    #[serde(rename = "diameter", skip_serializing_if = "Option::is_none")]
    pub diameter_mm: Option<f64>,
    /// Length, mm.
    #[serde(rename = "length", skip_serializing_if = "Option::is_none")]
    pub length_mm: Option<f64>,
    /// `SU` (single use), `reload` or `hybrid`, as the API writes it.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub motor_type: Option<String>,
    /// Average thrust, N.
    #[serde(rename = "avgThrustN", skip_serializing_if = "Option::is_none")]
    pub avg_thrust_n: Option<f64>,
    /// Peak thrust, N.
    #[serde(rename = "maxThrustN", skip_serializing_if = "Option::is_none")]
    pub max_thrust_n: Option<f64>,
    /// Total impulse, N·s.
    #[serde(rename = "totImpulseNs", skip_serializing_if = "Option::is_none")]
    pub total_impulse_ns: Option<f64>,
    /// Burn time, s.
    #[serde(rename = "burnTimeS", skip_serializing_if = "Option::is_none")]
    pub burn_time_s: Option<f64>,
    /// How many data files (curves) it has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_files: Option<u32>,
    /// The certification or manufacturer's page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_url: Option<String>,
    /// Loaded weight, g.
    #[serde(rename = "totalWeightG", skip_serializing_if = "Option::is_none")]
    pub total_weight_g: Option<f64>,
    /// Propellant weight, g.
    #[serde(rename = "propWeightG", skip_serializing_if = "Option::is_none")]
    pub propellant_weight_g: Option<f64>,
    /// The delays offered, as the API writes them (`6,10,14`, `P` for a plugged motor).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delays: Option<String>,
    /// Whether the delay is cut to length by the flier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_adjustable: Option<bool>,
    /// The reload's case (`RMS-54/852`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_info: Option<String>,
    /// The propellant's name (`Dark Matter`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prop_info: Option<String>,
    /// Whether the exhaust throws sparks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparky: Option<bool>,
    /// When the record last changed, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_on: Option<String>,
    /// `regular`, `occasional` or `OOP` (out of production), as the API writes it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<String>,
    /// The motor's page on the site.
    #[serde(rename = "source_url", skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

/// A download's answer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadAnswer {
    /// The data files: none for a motor with none in the format asked, or an unknown id.
    pub results: Vec<DataFile>,
}

/// One data file of a motor.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFile {
    /// The motor's ThrustCurve id.
    pub motor_id: String,
    /// The file's own id.
    pub simfile_id: String,
    /// Its format.
    pub format: Format,
    /// Who measured it, as the API writes it: `cert` (a certification test), `mfr` (the
    /// manufacturer) or `user`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Its licence, as the API writes it: `PD` (public domain), `free` or `other`; `None` when the
    /// contributor named none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// The file, base64-encoded; [`DataFile::text`] decodes it.
    pub data: String,
    /// The file's page on the site, as a path (`/simfiles/{id}/`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info_url: Option<String>,
    /// Where the site serves the file itself, as a path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_url: Option<String>,
    /// The file's page, as a full URL.
    #[serde(rename = "source_url", skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
}

/// A data file read by `hpr_motor`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Curve {
    /// A RASP file, read by [`hpr_motor::eng::parse`].
    Rasp(Parsed<eng::EngFile>),
    /// A RockSim file, read by [`hpr_motor::rse::parse`].
    RockSim(Parsed<rse::RseFile>),
}

impl Curve {
    /// The first motor's thrust curve: a RASP file's first entry, a RockSim file's first engine.
    ///
    /// # Errors
    /// [`MotorError`] when the curve breaks `hpr_motor`'s rules, or the file holds no motor.
    pub fn thrust_curve(&self) -> Result<ThrustCurve, MotorError> {
        let none = || MotorError::Inconsistent("the file holds no motor".to_owned());
        match self {
            Self::Rasp(parsed) => parsed
                .value
                .entries
                .first()
                .ok_or_else(none)?
                .thrust_curve(),
            Self::RockSim(parsed) => parsed
                .value
                .engines
                .first()
                .ok_or_else(none)?
                .thrust_curve(),
        }
    }
}

impl DataFile {
    /// The file's text: its data decoded from base64, as UTF-8.
    ///
    /// # Errors
    /// [`ThrustCurveError::Field`] when the file isn't UTF-8, or (only in a file not read by
    /// [`parse_download`], which refuses that) its data isn't base64. One file that isn't UTF-8
    /// doesn't refuse the answer: the motor's other files stay readable.
    pub fn text(&self) -> Result<String, ThrustCurveError> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.data)
            .map_err(|e| field(format!("{}.data", self.simfile_id), e))?;
        String::from_utf8(bytes)
            .map_err(|e| field(format!("{}.data", self.simfile_id), e.utf8_error()))
    }

    /// The file read by `hpr_motor`'s reader for its format.
    ///
    /// # Errors
    /// As [`DataFile::text`], and [`ThrustCurveError::Motor`] when the reader refuses the file.
    pub fn read(&self) -> Result<Curve, ThrustCurveError> {
        let text = self.text()?;
        Ok(match self.format {
            Format::Rasp => Curve::Rasp(eng::parse(&text)?),
            Format::RockSim => Curve::RockSim(rse::parse(&text)?),
        })
    }
}

/// Reads a search's answer.
///
/// # Errors
/// [`ThrustCurveError::Json`] when the body is not JSON of this shape; [`ThrustCurveError::Api`]
/// when the answer or one of its criteria carries the API's `error`; [`ThrustCurveError::Count`]
/// when it returns more motors than it says match; [`ThrustCurveError::Field`] for a motor id
/// that isn't 24 hexadecimal digits, or a diameter, length, thrust, impulse, burn time or weight
/// below zero.
pub fn parse_search(body: &[u8]) -> Result<SearchAnswer, ThrustCurveError> {
    #[derive(Deserialize)]
    struct Errors {
        error: Option<String>,
        #[serde(default)]
        criteria: Vec<CriterionError>,
    }
    #[derive(Deserialize)]
    struct CriterionError {
        name: String,
        error: Option<String>,
    }
    let errors: Errors = json(body)?;
    if let Some(error) = errors.error {
        return Err(ThrustCurveError::Api(error));
    }
    if let Some(c) = errors.criteria.into_iter().find(|c| c.error.is_some()) {
        let error = c.error.unwrap_or_default();
        return Err(ThrustCurveError::Api(format!("{}: {error}", c.name)));
    }
    let answer: SearchAnswer = json(body)?;
    let found = answer.results.len();
    if usize::try_from(answer.matches).map_or(true, |m| found > m) {
        return Err(ThrustCurveError::Count {
            matches: answer.matches,
            found,
        });
    }
    for (i, motor) in answer.results.iter().enumerate() {
        check_record(motor, &format!("results[{i}]"))?;
    }
    Ok(answer)
}

/// Reads a download's answer.
///
/// # Errors
/// [`ThrustCurveError::Json`] when the body is not JSON of this shape; [`ThrustCurveError::Api`]
/// when it carries the API's `error`; [`ThrustCurveError::Field`] for a motor id that isn't 24
/// hexadecimal digits, or a file's data that isn't base64. A file that decodes but isn't UTF-8
/// text is kept: [`DataFile::text`] reports it, for that file alone.
pub fn parse_download(body: &[u8]) -> Result<DownloadAnswer, ThrustCurveError> {
    #[derive(Deserialize)]
    struct Errors {
        error: Option<String>,
    }
    if let Some(error) = json::<Errors>(body)?.error {
        return Err(ThrustCurveError::Api(error));
    }
    let answer: DownloadAnswer = json(body)?;
    for (i, file) in answer.results.iter().enumerate() {
        if !is_motor_id(&file.motor_id) {
            return Err(field(format!("results[{i}].motorId"), &file.motor_id));
        }
        base64::engine::general_purpose::STANDARD
            .decode(&file.data)
            .map_err(|e| field(format!("results[{i}].data"), e))?;
    }
    Ok(answer)
}

/// Runs a search through `client`.
///
/// The answer comes from the client's cache while fresh (see [`source`]); offline, from the cache
/// only. The [`Fetched`] says which, carries [`ATTRIBUTION`] and holds the body. Only an answer
/// that [`parse_search`] reads is cached ([`Client::fetch_checked`]): one that doesn't never takes
/// a good copy's place, and online a stale good copy is returned instead, with the reason.
///
/// # Errors
/// [`ThrustCurveError::Net`] when the fetch fails, or with [`NetError::Refused`] naming what the
/// parser refused when the only answer there is doesn't parse.
pub fn fetch_search<T: Transport>(
    client: &Client<T>,
    search: &Search,
    now_s: u64,
) -> Result<(SearchAnswer, Fetched), ThrustCurveError> {
    fetch(client, &search.url(), now_s, parse_search)
}

/// Fetches a motor's data files through `client`, as [`fetch_search`] does. An answer holding
/// another motor's file, or a file in another format, is refused and never cached.
///
/// # Errors
/// As [`fetch_search`]; with [`NetError::Refused`] naming [`ThrustCurveError::OtherMotor`] for a
/// file of another motor or format than asked.
pub fn fetch_download<T: Transport>(
    client: &Client<T>,
    download: &Download,
    now_s: u64,
) -> Result<(DownloadAnswer, Fetched), ThrustCurveError> {
    let read = |body: &[u8]| {
        let answer = parse_download(body)?;
        let other = answer.results.iter().find(|f| {
            !f.motor_id.eq_ignore_ascii_case(&download.motor_id) || f.format != download.format
        });
        if let Some(file) = other {
            return Err(ThrustCurveError::OtherMotor {
                asked: format!("{} ({})", download.motor_id, download.format.as_str()),
                found: format!("{} ({})", file.motor_id, file.format.as_str()),
            });
        }
        Ok(answer)
    };
    fetch(client, &download.url(), now_s, read)
}

/// Fetches every record of the makers the motor finder reads ([`MANUFACTURERS`]), one complete
/// search each, as [`fetch_search`] does: the records [`join`] needs. A search that matches more
/// motors than it returns, or holds another maker's record, is refused and never cached, so a
/// join never runs on part of a maker's records.
///
/// # Errors
/// As [`fetch_search`]; with [`NetError::Refused`] naming [`ThrustCurveError::Incomplete`] for a
/// search cut short, or [`ThrustCurveError::OtherManufacturer`] for another maker's record.
pub fn fetch_finder_records<T: Transport>(
    client: &Client<T>,
    now_s: u64,
) -> Result<(Vec<MotorRecord>, Vec<Fetched>), ThrustCurveError> {
    let mut records = Vec::new();
    let mut fetched = Vec::new();
    for &(name, _) in MANUFACTURERS {
        let read = |body: &[u8]| {
            let answer = parse_search(body)?;
            if !answer.is_complete() {
                return Err(ThrustCurveError::Incomplete {
                    matches: answer.matches,
                    found: answer.results.len(),
                });
            }
            if let Some(other) = answer.results.iter().find(|r| r.manufacturer != name) {
                return Err(ThrustCurveError::OtherManufacturer {
                    asked: name.to_owned(),
                    found: other.manufacturer.clone(),
                });
            }
            Ok(answer)
        };
        let (answer, f) = fetch(client, &Search::manufacturer(name).url(), now_s, read)?;
        records.extend(answer.results);
        fetched.push(f);
    }
    Ok((records, fetched))
}

/// The motor finder's motors matched to ThrustCurve records, with the misses.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Join {
    /// The motors that mapped, in the finder's order.
    pub mapped: Vec<Mapped>,
    /// The motors that didn't, in the finder's order.
    pub misses: Vec<Miss>,
}

/// A finder motor and the one ThrustCurve record with its name.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mapped {
    /// The manufacturer's full name.
    pub manufacturer: String,
    /// The designation.
    pub designation: String,
    /// Where the motor is in the list given to [`join`].
    pub finder_index: usize,
    /// The record: its `motor_id` is what a [`Download`] names.
    pub record: MotorRecord,
}

/// A finder motor that didn't map, and why.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Miss {
    /// The manufacturer's full name.
    pub manufacturer: String,
    /// The designation.
    pub designation: String,
    /// Why it didn't map.
    pub reason: MissReason,
}

/// Why a finder motor didn't map.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissReason {
    /// No record has its manufacturer and designation.
    NoRecord,
    /// Several do; their ids.
    SeveralRecords(Vec<String>),
}

impl Join {
    /// How many motors mapped, and how many were joined.
    #[must_use]
    pub fn coverage(&self) -> (usize, usize) {
        (self.mapped.len(), self.mapped.len() + self.misses.len())
    }

    /// The join as Markdown: counts by manufacturer, among them the mapped motors whose record
    /// lists no data file (`dataFiles` 0, or left out, as the API leaves out a field with no
    /// value), and every miss with its reason.
    #[must_use]
    pub fn report(&self) -> String {
        let mut by_maker: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
        for m in &self.mapped {
            let row = by_maker.entry(&m.manufacturer).or_default();
            row.0 += 1;
            row.1 += 1;
            if m.record.data_files.unwrap_or(0) == 0 {
                row.2 += 1;
            }
        }
        for m in &self.misses {
            by_maker.entry(&m.manufacturer).or_default().1 += 1;
        }
        let (mapped, total) = self.coverage();
        let mut out = String::new();
        // Writing to a String can't fail.
        let _ = writeln!(
            out,
            "{mapped} of {total} motors mapped to one record each.\n"
        );
        let _ = writeln!(
            out,
            "| Manufacturer | Motors | Mapped | Mapped, no data file listed | Missed |"
        );
        let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        for (maker, (mapped, total, no_file)) in &by_maker {
            let missed = total - mapped;
            let _ = writeln!(
                out,
                "| {maker} | {total} | {mapped} | {no_file} | {missed} |"
            );
        }
        let _ = writeln!(out);
        if self.misses.is_empty() {
            let _ = writeln!(out, "No misses.");
        } else {
            let _ = writeln!(out, "| Manufacturer | Designation | Why |");
            let _ = writeln!(out, "|---|---|---|");
            for m in &self.misses {
                let why = match &m.reason {
                    MissReason::NoRecord => "no record of that name".to_owned(),
                    MissReason::SeveralRecords(ids) => {
                        format!("{} records of that name: {}", ids.len(), ids.join(", "))
                    }
                };
                let _ = writeln!(out, "| {} | `{}` | {why} |", m.manufacturer, m.designation);
            }
        }
        out
    }
}

/// Maps each finder motor to the one ThrustCurve record whose manufacturer (its full name) and
/// designation equal the motor's, character for character. A motor with no such record, or
/// several, is a [`Miss`]. A record given twice (the same `motor_id`, as when two searches
/// overlap) counts once: the first copy is kept.
#[must_use]
pub fn join(motors: &[motor_finder::Motor], records: &[MotorRecord]) -> Join {
    let mut by_name: BTreeMap<(&str, &str), Vec<&MotorRecord>> = BTreeMap::new();
    for r in records {
        let same_name = by_name
            .entry((&r.manufacturer, &r.designation))
            .or_default();
        if same_name
            .iter()
            .all(|kept| !kept.motor_id.eq_ignore_ascii_case(&r.motor_id))
        {
            same_name.push(r);
        }
    }
    let mut join = Join {
        mapped: Vec::new(),
        misses: Vec::new(),
    };
    for (finder_index, m) in motors.iter().enumerate() {
        let found = by_name
            .get(&(m.manufacturer.as_str(), m.designation.as_str()))
            .map_or(&[][..], Vec::as_slice);
        let miss = |reason| Miss {
            manufacturer: m.manufacturer.clone(),
            designation: m.designation.clone(),
            reason,
        };
        match found {
            [record] => join.mapped.push(Mapped {
                manufacturer: m.manufacturer.clone(),
                designation: m.designation.clone(),
                finder_index,
                record: (*record).clone(),
            }),
            [] => join.misses.push(miss(MissReason::NoRecord)),
            several => join.misses.push(miss(MissReason::SeveralRecords(
                several.iter().map(|r| r.motor_id.clone()).collect(),
            ))),
        }
    }
    join
}

/// Fetches `url` through `client`, caching only an answer `read` reads.
fn fetch<T: Transport, R>(
    client: &Client<T>,
    url: &str,
    now_s: u64,
    read: impl Fn(&[u8]) -> Result<R, ThrustCurveError>,
) -> Result<(R, Fetched), ThrustCurveError> {
    let check = |body: &[u8]| read(body).map(drop).map_err(|e| e.to_string());
    let fetched = client.fetch_checked(&source(), url, now_s, check)?;
    let value = read(&fetched.body)?;
    Ok((value, fetched))
}

/// Whether `id` has the form of a ThrustCurve id: 24 hexadecimal digits.
fn is_motor_id(id: &str) -> bool {
    id.len() == 24 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Percent-encodes a query value: every byte but an ASCII letter, digit, `-`, `.`, `_` or `~`
/// (RFC 3986's unreserved characters) is written `%XX`.
fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Checks a record: a motor id of the API's form, and no figure below zero.
fn check_record(motor: &MotorRecord, at: &str) -> Result<(), ThrustCurveError> {
    if !is_motor_id(&motor.motor_id) {
        return Err(field(format!("{at}.motorId"), &motor.motor_id));
    }
    let figures = [
        ("diameter", motor.diameter_mm),
        ("length", motor.length_mm),
        ("avgThrustN", motor.avg_thrust_n),
        ("maxThrustN", motor.max_thrust_n),
        ("totImpulseNs", motor.total_impulse_ns),
        ("burnTimeS", motor.burn_time_s),
        ("totalWeightG", motor.total_weight_g),
        ("propWeightG", motor.propellant_weight_g),
    ];
    for (name, value) in figures {
        if let Some(value) = value.filter(|v| *v < 0.0) {
            return Err(field(format!("{at}.{name}"), value));
        }
    }
    Ok(())
}

/// Deserializes `body`.
fn json<'a, D: Deserialize<'a>>(body: &'a [u8]) -> Result<D, ThrustCurveError> {
    serde_json::from_slice(body).map_err(|e| ThrustCurveError::Json(e.to_string()))
}

/// A [`ThrustCurveError::Field`].
fn field(field: String, value: impl ToString) -> ThrustCurveError {
    ThrustCurveError::Field {
        field,
        value: value.to_string(),
    }
}

/// Why a ThrustCurve request or answer was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ThrustCurveError {
    /// A request field is not one the API serves.
    #[error("the ThrustCurve request's {what} is not one it serves: {value:?}")]
    Request {
        /// The field.
        what: &'static str,
        /// Its value.
        value: String,
    },
    /// The fetch failed.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The body is not JSON of the expected shape: a field missing, or of the wrong type.
    #[error("the ThrustCurve answer is not of the expected shape: {0}")]
    Json(String),
    /// The answer carries the API's `error`, or one of its criteria does.
    #[error("ThrustCurve refused the request: {0}")]
    Api(String),
    /// A search returns more motors than it says match.
    #[error("the ThrustCurve search says {matches} motors match but returns {found}")]
    Count {
        /// The `matches` stated.
        matches: u32,
        /// The motors returned.
        found: usize,
    },
    /// A search returns fewer motors than match: its `maxResults` cut it short.
    #[error("the ThrustCurve search matches {matches} motors but returns only {found}")]
    Incomplete {
        /// The `matches` stated.
        matches: u32,
        /// The motors returned.
        found: usize,
    },
    /// A field breaks the API's rules.
    #[error("the ThrustCurve answer's {field} is not usable: {value}")]
    Field {
        /// Where it is (`results[12].motorId`).
        field: String,
        /// Its value.
        value: String,
    },
    /// A maker's search holds another maker's record.
    #[error("asked ThrustCurve for {asked}'s motors, but the answer holds one of {found}'s")]
    OtherManufacturer {
        /// The maker asked for.
        asked: String,
        /// The maker found.
        found: String,
    },
    /// A download holds a file of another motor or format than asked.
    #[error("asked ThrustCurve for {asked}'s files, but the answer holds {found}'s")]
    OtherMotor {
        /// The motor id and format asked for.
        asked: String,
        /// The motor id and format found.
        found: String,
    },
    /// `hpr_motor`'s reader refused a data file.
    #[error(transparent)]
    Motor(#[from] MotorError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_follow_the_apis_names() {
        assert_eq!(
            Search::manufacturer("Cesaroni Technology").url(),
            "https://www.thrustcurve.org/api/v1/search.json?manufacturer=Cesaroni%20Technology&maxResults=5000"
        );
        let search = Search {
            designation: Some("F27R/L".to_owned()),
            impulse_class: Some("F".to_owned()),
            ..Search::default()
        };
        assert_eq!(
            search.url(),
            "https://www.thrustcurve.org/api/v1/search.json?designation=F27R%2FL&impulseClass=F"
        );
        assert_eq!(
            Search::default().url(),
            "https://www.thrustcurve.org/api/v1/search.json"
        );
        let download = Download::new("5f4294d2000231000000044f", Format::Rasp).unwrap();
        assert_eq!(
            download.url(),
            "https://www.thrustcurve.org/api/v1/download.json?motorId=5f4294d2000231000000044f&format=RASP&data=file"
        );
    }

    #[test]
    fn a_motor_id_must_be_24_hex_digits() {
        for bad in [
            "",
            "5f4294d2000231000000044",
            "5f4294d2000231000000044f0",
            "5f4294d2000231000000044g",
            "5f4294d200023100000004&f",
        ] {
            let err = Download::new(bad, Format::Rasp).unwrap_err();
            assert!(
                matches!(&err, ThrustCurveError::Request { what: "motor id", value } if value == bad),
                "{bad}: {err}"
            );
        }
        let upper = Download::new("5F4294D2000231000000044F", Format::RockSim).unwrap();
        assert_eq!(upper.motor_id(), "5f4294d2000231000000044f");
        assert!(upper.url().contains("motorId=5f4294d2000231000000044f&"));
    }

    #[test]
    fn query_values_are_percent_encoded() {
        assert_eq!(encode("AZaz09-._~"), "AZaz09-._~");
        assert_eq!(encode("a b/c&d=e?é"), "a%20b%2Fc%26d%3De%3F%C3%A9");
    }
}
