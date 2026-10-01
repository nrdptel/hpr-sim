//! `hpr weather`: a launch site's air and wind, level by level, as a sounding profile, from
//! Open-Meteo, a University of Wyoming balloon sounding, NOAA's GFS or RAP, or an ERA5 file
//! (ADR-122).
//!
//! The online sources go through `hpr_net`'s cache in the platform's cache folder, so a second
//! run for the same launch reads the saved answer; `--offline` answers from the cache alone, and
//! `--from` reads an answer saved earlier (a GRIB2 file, a sounding's CSV, Open-Meteo's JSON) and
//! touches neither the network nor the cache. Each source's reader and its checks are the
//! library's: this module picks the source, prints the levels and writes the profile.
//!
//! The profile written is `hpr_atmos::SoundingProfile`'s JSON, which the library reads back with
//! the same checks.

use std::time::{SystemTime, UNIX_EPOCH};

use hpr::hpr_atmos::{AtmosError, SoundingProfile, WindInterpolation};
use hpr::hpr_io::era5::{Era5Error, Era5Profile, Era5Request, UtcTime};
use hpr::hpr_io::netcdf::NetCdf;
use hpr::hpr_net::nomads::{self, NomadsModel, NomadsProfile, NomadsRequest};
use hpr::hpr_net::open_meteo::{self, OpenMeteoApi, OpenMeteoProfile, OpenMeteoRequest};
use hpr::hpr_net::wyoming::{self, WyomingRequest, WyomingSounding, WyomingVersion};
use hpr::hpr_net::{Cache, Client, Fetched, Freshness, Http, Mode};

use crate::output::{
    DropReason, DroppedLevel, ProfileLevel, ReadFrom, Weather, WeatherPosition, WeatherSource,
    WeatherSourceName,
};
use crate::{Failure, Out};

/// The credit ERA5's licence (CC BY 4.0) asks for, as `docs/format/era5.md` gives it.
const ERA5_ATTRIBUTION: &str = "Contains modified Copernicus Climate Change Service information \
                                (ERA5, CC BY 4.0)";

/// `hpr weather`'s sources.
#[derive(Debug, clap::Subcommand)]
pub enum WeatherCommand {
    /// Open-Meteo's forecast at a site, or its archive of past forecasts
    OpenMeteo(OpenMeteoArgs),
    /// A University of Wyoming balloon sounding: a station's latest before the launch
    Wyoming(WyomingArgs),
    /// NOAA's GFS forecast (0.25°, the whole Earth) at a site, from NOMADS
    Gfs(NomadsArgs),
    /// NOAA's RAP forecast (13 km, the contiguous U.S. and nearby Canada and Mexico) at a site,
    /// from NOMADS
    Rap(NomadsArgs),
    /// An ERA5 pressure-level file (netCDF) read at a site and time
    Era5(Era5Args),
}

/// Where an online source's answer comes from, and where the profile goes.
#[derive(Debug, clap::Args)]
pub struct Fetching {
    /// Read the source's answer from this file, saved earlier, instead of fetching it
    #[arg(long, value_name = "FILE")]
    pub from: Option<String>,
    /// Answer from the cache only, never the network
    #[arg(long, conflicts_with = "from")]
    pub offline: bool,
    /// Write the profile to this file, as JSON
    #[arg(long, short, value_name = "FILE")]
    pub output: Option<String>,
}

/// `hpr weather open-meteo`'s arguments.
#[derive(Debug, clap::Args)]
pub struct OpenMeteoArgs {
    /// The site's latitude, degrees north (south is negative); not with --from, whose answer is
    /// for the place it was asked for
    #[arg(
        long,
        value_name = "DEG",
        allow_negative_numbers = true,
        required_unless_present = "from",
        conflicts_with = "from"
    )]
    pub latitude: Option<f64>,
    /// The site's longitude, degrees east (west is negative); not with --from
    #[arg(
        long,
        value_name = "DEG",
        allow_negative_numbers = true,
        required_unless_present = "from",
        conflicts_with = "from"
    )]
    pub longitude: Option<f64>,
    /// The launch time in UTC, such as 2025-06-21T15:30Z; it must be within the answer's hours
    #[arg(long, value_name = "TIME")]
    pub time: String,
    /// Ask the archive of past forecasts, for a launch already gone
    #[arg(long, conflicts_with = "from")]
    pub historical: bool,
    /// The weather model, by Open-Meteo's name, such as gfs_seamless; its best match by default
    #[arg(long, conflicts_with = "from")]
    pub model: Option<String>,
    /// The answer's source and the profile's file.
    #[command(flatten)]
    pub fetching: Fetching,
}

/// `hpr weather wyoming`'s arguments.
#[derive(Debug, clap::Args)]
pub struct WyomingArgs {
    /// The station's WMO number, such as 72364; not with --from, whose sounding names its own
    /// place and time
    #[arg(long, required_unless_present = "from", conflicts_with = "from")]
    pub station: Option<String>,
    /// The launch time in UTC; the sounding is the station's latest, at 00 or 12 UTC, before it
    #[arg(
        long,
        value_name = "TIME",
        required_unless_present = "from",
        conflicts_with = "from"
    )]
    pub time: Option<String>,
    /// Ask for the sounding decoded from BUFR, a row every second or two of the ascent, instead
    /// of the coded (FM 35) one
    #[arg(long, conflicts_with = "from")]
    pub bufr: bool,
    /// The answer's source and the profile's file.
    #[command(flatten)]
    pub fetching: Fetching,
}

/// `hpr weather gfs`'s and `hpr weather rap`'s arguments.
#[derive(Debug, clap::Args)]
pub struct NomadsArgs {
    /// The site's latitude, degrees north (south is negative)
    #[arg(long, value_name = "DEG", allow_negative_numbers = true)]
    pub latitude: f64,
    /// The site's longitude, degrees east (west is negative)
    #[arg(long, value_name = "DEG", allow_negative_numbers = true)]
    pub longitude: f64,
    /// The run, its start in UTC, such as 2026-09-30T00Z
    #[arg(
        long,
        value_name = "TIME",
        required_unless_present = "from",
        requires = "hour"
    )]
    pub cycle: Option<String>,
    /// The hours after the run's start that the forecast is for
    #[arg(long, required_unless_present = "from", requires = "cycle")]
    pub hour: Option<u32>,
    /// The answer's source and the profile's file.
    #[command(flatten)]
    pub fetching: Fetching,
}

/// `hpr weather era5`'s arguments.
#[derive(Debug, clap::Args)]
pub struct Era5Args {
    /// The ERA5 pressure-level file, netCDF classic
    pub file: String,
    /// The site's latitude, degrees north (south is negative)
    #[arg(long, value_name = "DEG", allow_negative_numbers = true)]
    pub latitude: f64,
    /// The site's longitude, degrees east (west is negative)
    #[arg(long, value_name = "DEG", allow_negative_numbers = true)]
    pub longitude: f64,
    /// The launch time in UTC, such as 2020-02-22T13:00Z
    #[arg(long, value_name = "TIME")]
    pub time: String,
    /// Write the profile to this file, as JSON
    #[arg(long, short, value_name = "FILE")]
    pub output: Option<String>,
}

/// A source read: the profile and what the output says about it.
struct Read {
    source: WeatherSource,
    read_from: ReadFrom,
    position: WeatherPosition,
    time_unix_s: i64,
    run_unix_s: Option<i64>,
    sounding: SoundingProfile,
    dropped: Vec<DroppedLevel>,
}

/// Runs `hpr weather`.
pub(crate) fn run(command: &WeatherCommand, to: &mut Out<'_>) -> Result<(), Failure> {
    let (read, output) = match command {
        WeatherCommand::OpenMeteo(args) => (open_meteo(args)?, &args.fetching.output),
        WeatherCommand::Wyoming(args) => (wyoming(args)?, &args.fetching.output),
        WeatherCommand::Gfs(args) => (
            nomads((NomadsModel::Gfs, "GFS", WeatherSourceName::Gfs), args)?,
            &args.fetching.output,
        ),
        WeatherCommand::Rap(args) => (
            nomads((NomadsModel::Rap, "RAP", WeatherSourceName::Rap), args)?,
            &args.fetching.output,
        ),
        WeatherCommand::Era5(args) => (era5(args)?, &args.output),
    };
    if let Some(path) = output {
        std::fs::write(path, profile_json(&read.sounding)?)
            .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
    }
    let document = document(read, output.clone());
    let lines = text_lines(&document);
    to.emit(&document, |out| {
        lines.iter().try_for_each(|line| writeln!(out, "{line}"))
    })
}

/// The profile's JSON, pretty-printed with a trailing newline.
fn profile_json(sounding: &SoundingProfile) -> Result<String, Failure> {
    let text = serde_json::to_string_pretty(sounding)
        .map_err(|error| Failure::Input(format!("the profile didn't serialize: {error}")))?;
    Ok(format!("{text}\n"))
}

pub(crate) fn read_file(path: &str) -> Result<Vec<u8>, Failure> {
    std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")))
}

/// A client over the network and the platform's cache, or the cache alone `offline`, for the
/// command named `command` (`hpr weather`).
pub(crate) fn client(offline: bool, command: &str) -> Result<Client<Http>, Failure> {
    let dir = Cache::platform_dir().ok_or_else(|| {
        Failure::Input(format!(
            "{command} keeps what it fetches in a cache folder, and this system names no home \
             folder to put it in: set HPR_CACHE_DIR to a folder"
        ))
    })?;
    let mode = if offline { Mode::Offline } else { Mode::Online };
    Ok(Client::new(Http::new(), Cache::new(dir), mode))
}

/// Seconds since the Unix epoch now; 0 if the clock is before it, which makes every cached copy
/// stale rather than fresh.
pub(crate) fn now_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

pub(crate) fn read_from(fetched: &Fetched) -> ReadFrom {
    match fetched.freshness {
        Freshness::Fetched => ReadFrom::Network {
            fetched_at_unix_s: fetched.fetched_at_s,
        },
        Freshness::Cached => ReadFrom::Cache {
            fetched_at_unix_s: fetched.fetched_at_s,
        },
        // `Stale`, and any state a later `hpr_net` adds: none of them is known to be fresh.
        _ => ReadFrom::StaleCache {
            fetched_at_unix_s: fetched.fetched_at_s,
            reason: fetched.stale_reason.clone(),
        },
    }
}

fn site(latitude: Option<f64>, longitude: Option<f64>) -> Result<(f64, f64), Failure> {
    // Clap requires both unless `--from` is given, and only a fetch asks.
    latitude.zip(longitude).ok_or_else(|| {
        Failure::Input("fetching needs the site's --latitude and --longitude".to_owned())
    })
}

fn sounding_failure(source: &str, error: &AtmosError) -> Failure {
    Failure::Input(format!("{source}'s levels don't make a profile: {error}"))
}

fn open_meteo(args: &OpenMeteoArgs) -> Result<Read, Failure> {
    let time = parse_utc(&args.time)?;
    let api = if args.historical {
        OpenMeteoApi::HistoricalForecast
    } else {
        OpenMeteoApi::Forecast
    };
    let refused = |error| Failure::Input(format!("Open-Meteo: {}", open_meteo_reason(error)));
    let (profile, read_from) = match &args.fetching.from {
        Some(path) => (
            OpenMeteoProfile::parse(&read_file(path)?, time)
                .map_err(|error| Failure::Input(format!("{path}: {}", open_meteo_reason(error))))?,
            ReadFrom::File { path: path.clone() },
        ),
        None => {
            let (latitude, longitude) = site(args.latitude, args.longitude)?;
            let mut request = OpenMeteoRequest::new(latitude, longitude, time, api);
            request.model.clone_from(&args.model);
            let (profile, fetched) = open_meteo::fetch(
                &client(args.fetching.offline, "hpr weather")?,
                &request,
                now_s(),
            )
            .map_err(refused)?;
            (profile, read_from(&fetched))
        }
    };
    let sounding = profile
        .sounding(WindInterpolation::SpeedDirection)
        .map_err(|error| sounding_failure("Open-Meteo", &error))?;
    let dropped = profile
        .dropped
        .iter()
        .map(|level| DroppedLevel {
            pressure_pa: Some(level.pressure_pa),
            line: None,
            reason: match level.reason {
                open_meteo::DropReason::BelowGround => DropReason::BelowGround,
                open_meteo::DropReason::NoData => DropReason::NoData,
                open_meteo::DropReason::Humidity => DropReason::OutOfRange,
                _ => DropReason::Other,
            },
        })
        .collect();
    Ok(Read {
        source: WeatherSource {
            name: WeatherSourceName::OpenMeteo,
            attribution: open_meteo::ATTRIBUTION.to_owned(),
        },
        read_from,
        position: WeatherPosition {
            latitude_deg: profile.latitude_deg,
            longitude_deg: profile.longitude_deg,
        },
        time_unix_s: profile.time_unix_s,
        run_unix_s: None,
        sounding,
        dropped,
    })
}

/// An Open-Meteo refusal, with a time outside the answer's hours written in UTC, as it was asked.
fn open_meteo_reason(error: open_meteo::OpenMeteoError) -> String {
    match error {
        open_meteo::OpenMeteoError::TimeOutside {
            time_unix_s,
            first_s,
            last_s,
            ..
        } => format!(
            "{} is outside the answer's hours, {} to {}",
            format_utc(time_unix_s),
            format_utc(first_s),
            format_utc(last_s)
        ),
        other => other.to_string(),
    }
}

fn wyoming(args: &WyomingArgs) -> Result<Read, Failure> {
    let (sounding, read_from) = match &args.fetching.from {
        Some(path) => (
            WyomingSounding::parse(&read_file(path)?)
                .map_err(|error| Failure::Input(format!("{path}: {error}")))?,
            ReadFrom::File { path: path.clone() },
        ),
        None => {
            // Clap requires both unless `--from` is given.
            let (Some(station), Some(time)) = (&args.station, &args.time) else {
                return Err(Failure::Input(
                    "fetching needs the --station and the launch --time".to_owned(),
                ));
            };
            let mut request = WyomingRequest::latest_before(station.clone(), parse_utc(time)?);
            if args.bufr {
                request.version = WyomingVersion::Bufr;
            }
            let (sounding, fetched) = wyoming::fetch(
                &client(args.fetching.offline, "hpr weather")?,
                &request,
                now_s(),
            )
            .map_err(|error| Failure::Input(format!("University of Wyoming: {error}")))?;
            (sounding, read_from(&fetched))
        }
    };
    let profile = sounding
        .sounding(WindInterpolation::SpeedDirection)
        .map_err(|error| sounding_failure("the sounding", &error))?;
    let dropped = sounding
        .dropped
        .iter()
        .map(|row| DroppedLevel {
            pressure_pa: None,
            line: Some(row.line),
            reason: match row.reason {
                wyoming::DropReason::NoData => DropReason::NoData,
                wyoming::DropReason::OutOfRange => DropReason::OutOfRange,
                wyoming::DropReason::SamePressure => DropReason::SamePressure,
                wyoming::DropReason::NotAbove => DropReason::NotAbove,
                wyoming::DropReason::Thickness => DropReason::Thickness,
                _ => DropReason::Other,
            },
        })
        .collect();
    Ok(Read {
        source: WeatherSource {
            name: WeatherSourceName::Wyoming,
            attribution: wyoming::ATTRIBUTION.to_owned(),
        },
        read_from,
        position: WeatherPosition {
            latitude_deg: sounding.latitude_deg,
            longitude_deg: sounding.longitude_deg,
        },
        time_unix_s: sounding.release_unix_s,
        run_unix_s: None,
        sounding: profile,
        dropped,
    })
}

fn nomads(
    (model, name, source): (NomadsModel, &str, WeatherSourceName),
    args: &NomadsArgs,
) -> Result<Read, Failure> {
    let run = match (&args.cycle, args.hour) {
        (Some(cycle), Some(hour)) => Some((parse_utc(cycle)?, hour)),
        _ => None,
    };
    let (profile, read_from) = match (&args.fetching.from, run) {
        (Some(path), _) => {
            let profile = NomadsProfile::parse(&read_file(path)?, args.latitude, args.longitude)
                .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
            // The checks `nomads::fetch` makes of a fetched cut.
            if !model.has_grid(&profile.grid) {
                return Err(Failure::Input(format!(
                    "{path}: its grid is not {name}'s; read it with the model it came from"
                )));
            }
            if let Some((cycle, hour)) = run {
                let valid = cycle + i64::from(hour) * 3_600;
                if profile.cycle_unix_s != cycle || profile.valid_unix_s != valid {
                    return Err(Failure::Input(format!(
                        "{path}: it is the run of {} for {}, not the run of {} for {}",
                        format_utc(profile.cycle_unix_s),
                        format_utc(profile.valid_unix_s),
                        format_utc(cycle),
                        format_utc(valid),
                    )));
                }
            }
            (profile, ReadFrom::File { path: path.clone() })
        }
        (None, Some((cycle, hour))) => {
            let request = NomadsRequest::new(args.latitude, args.longitude, model, cycle, hour);
            let (profile, fetched) = nomads::fetch(
                &client(args.fetching.offline, "hpr weather")?,
                &request,
                now_s(),
            )
            .map_err(|error| Failure::Input(format!("NOMADS: {error}")))?;
            (profile, read_from(&fetched))
        }
        (None, None) => {
            // Clap requires both unless `--from` is given.
            return Err(Failure::Input(
                "fetching needs the run's --cycle and the forecast --hour".to_owned(),
            ));
        }
    };
    let sounding = profile
        .sounding(WindInterpolation::SpeedDirection)
        .map_err(|error| sounding_failure(name, &error))?;
    let dropped = profile
        .dropped
        .iter()
        .map(|level| DroppedLevel {
            pressure_pa: Some(level.pressure_pa),
            line: None,
            reason: match level.reason {
                nomads::DropReason::BelowGround => DropReason::BelowGround,
                nomads::DropReason::NoData => DropReason::NoData,
                _ => DropReason::Other,
            },
        })
        .collect();
    Ok(Read {
        source: WeatherSource {
            name: source,
            attribution: nomads::ATTRIBUTION.to_owned(),
        },
        read_from,
        position: WeatherPosition {
            latitude_deg: profile.latitude_deg,
            longitude_deg: profile.longitude_deg,
        },
        time_unix_s: profile.valid_unix_s,
        run_unix_s: Some(profile.cycle_unix_s),
        sounding,
        dropped,
    })
}

fn era5(args: &Era5Args) -> Result<Read, Failure> {
    let path = &args.file;
    let time_unix_s = parse_utc(&args.time)?;
    let refused = |error: &dyn std::fmt::Display| Failure::Input(format!("{path}: {error}"));
    let file = NetCdf::parse(&read_file(path)?).map_err(|error| refused(&error))?;
    let request = Era5Request {
        latitude_deg: args.latitude,
        longitude_deg: args.longitude,
        // Whole seconds, exact in an f64 for any year `parse_utc` accepts.
        time: UtcTime::from_unix_seconds(time_unix_s as f64).map_err(|error| refused(&error))?,
    };
    let profile = Era5Profile::read(&file, request).map_err(|error| match error {
        // Written in UTC, as the time was asked; the file's times are whole seconds.
        Era5Error::OutsideTimes { first, last, .. } => Failure::Input(format!(
            "{path}: {} is outside the file's times, {} to {}",
            format_utc(time_unix_s),
            format_utc(first.floor() as i64),
            format_utc(last.floor() as i64)
        )),
        other => refused(&other),
    })?;
    let sounding = profile
        .sounding(WindInterpolation::SpeedDirection)
        .map_err(|error| sounding_failure("ERA5", &error))?;
    Ok(Read {
        source: WeatherSource {
            name: WeatherSourceName::Era5,
            attribution: ERA5_ATTRIBUTION.to_owned(),
        },
        read_from: ReadFrom::File { path: path.clone() },
        position: WeatherPosition {
            latitude_deg: args.latitude,
            longitude_deg: args.longitude,
        },
        time_unix_s,
        run_unix_s: None,
        sounding,
        dropped: Vec::new(),
    })
}

/// The output document.
fn document(read: Read, profile: Option<String>) -> Weather {
    let levels = read
        .sounding
        .levels()
        .iter()
        .map(|level| ProfileLevel {
            height_msl_m: level.height_msl_m,
            pressure_pa: level.pressure_pa,
            temperature_k: level.temperature_k,
            relative_humidity: level.relative_humidity,
            wind_speed_m_s: level.wind_speed_m_s,
            wind_from_deg: level.wind_direction_from_rad.map(f64::to_degrees),
        })
        .collect();
    Weather {
        source: read.source,
        read_from: read.read_from,
        position: read.position,
        time: format_utc(read.time_unix_s),
        run: read.run_unix_s.map(format_utc),
        levels,
        dropped: read.dropped,
        profile,
    }
}

/// The text output: where the profile is from, its levels, what was left out, and the file.
fn text_lines(document: &Weather) -> Vec<String> {
    let source = match document.source.name {
        WeatherSourceName::OpenMeteo => "Open-Meteo",
        WeatherSourceName::Wyoming => "University of Wyoming sounding",
        WeatherSourceName::Gfs => "GFS",
        WeatherSourceName::Rap => "RAP",
        WeatherSourceName::Era5 => "ERA5",
    };
    let run = document
        .run
        .as_ref()
        .map_or(String::new(), |run| format!(", run of {run}"));
    let position = &document.position;
    let mut lines = vec![
        format!(
            "{source} at {}, {}, for {}{run}",
            degrees(position.latitude_deg, ["N", "S"]),
            degrees(position.longitude_deg, ["E", "W"]),
            document.time
        ),
        read_from_line(&document.read_from),
        document.source.attribution.clone(),
        String::new(),
        format!(
            "{:>9}  {:>8}  {:>6}  {:>8}  {:>6}  {:>5}",
            "height", "pressure", "temp", "humidity", "wind", "from"
        ),
        format!(
            "{:>9}  {:>8}  {:>6}  {:>8}  {:>6}  {:>5}",
            "m MSL", "hPa", "°C", "%", "m/s", "°"
        ),
    ];
    let or_blank = |value: Option<f64>, scale: f64, decimals: usize| {
        value.map_or("-".to_owned(), |value| {
            format!("{:.decimals$}", value * scale)
        })
    };
    for level in &document.levels {
        lines.push(format!(
            "{:>9.1}  {:>8}  {:>6.1}  {:>8}  {:>6}  {:>5}",
            level.height_msl_m,
            or_blank(level.pressure_pa, 0.01, 1),
            level.temperature_k - 273.15,
            or_blank(level.relative_humidity, 100.0, 0),
            or_blank(level.wind_speed_m_s, 1.0, 1),
            or_blank(level.wind_from_deg, 1.0, 0),
        ));
    }
    if !document.dropped.is_empty() {
        lines.push(String::new());
        lines.push(format!("Left out: {}", count(document.dropped.len())));
        // By reason, in the order each first appears; a sounding can drop thousands of rows.
        let mut reasons: Vec<DropReason> = Vec::new();
        for level in &document.dropped {
            if !reasons.contains(&level.reason) {
                reasons.push(level.reason);
            }
        }
        for reason in reasons {
            let which: Vec<String> = document
                .dropped
                .iter()
                .filter(|level| level.reason == reason)
                .map(|level| match (level.pressure_pa, level.line) {
                    (Some(pressure), _) => format!("{:.0} hPa", pressure / 100.0),
                    (None, Some(line)) => format!("line {line}"),
                    (None, None) => "a level".to_owned(),
                })
                .collect();
            let shown = which.len().min(DROPPED_SHOWN);
            let more = match which.len() - shown {
                0 => String::new(),
                rest => format!(" and {rest} more"),
            };
            lines.push(format!(
                "  {}, {}: {}{more}",
                reason.describe(),
                count(which.len()),
                which[..shown].join(", ")
            ));
        }
    }
    if let Some(path) = &document.profile {
        lines.push(String::new());
        lines.push(format!("Profile written to {path}"));
    }
    lines
}

/// How many left-out levels of one reason the text names before counting the rest.
const DROPPED_SHOWN: usize = 6;

/// `1 level`, `2 levels`.
fn count(levels: usize) -> String {
    format!("{levels} level{}", if levels == 1 { "" } else { "s" })
}

/// An angle in degrees with its hemisphere, such as `106.9700° W`.
fn degrees(value: f64, [positive, negative]: [&str; 2]) -> String {
    let hemisphere = if value < 0.0 { negative } else { positive };
    format!("{:.4}° {hemisphere}", value.abs())
}

/// Reads a UTC time written `YYYY-MM-DDTHH[:MM[:SS]]Z`, as seconds since the Unix epoch.
fn parse_utc(text: &str) -> Result<i64, Failure> {
    let bad = || {
        Failure::Input(format!(
            "{text}: write a time in UTC as YYYY-MM-DDTHH:MMZ, such as 2025-06-21T15:30Z"
        ))
    };
    let (date, time) = text
        .strip_suffix(['Z', 'z'])
        .and_then(|rest| rest.split_once(['T', 't']))
        .ok_or_else(bad)?;
    let number = |part: &str, digits: usize| {
        (part.len() == digits && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse::<u32>().ok())
            .flatten()
            .ok_or_else(bad)
    };
    let date: Vec<&str> = date.split('-').collect();
    let [year, month, day] = date[..] else {
        return Err(bad());
    };
    let time: Vec<&str> = time.split(':').collect();
    let (hour, minute, second) = match time[..] {
        [hour] => (number(hour, 2)?, 0, 0),
        [hour, minute] => (number(hour, 2)?, number(minute, 2)?, 0),
        [hour, minute, second] => (number(hour, 2)?, number(minute, 2)?, number(second, 2)?),
        _ => return Err(bad()),
    };
    let year = i32::try_from(number(year, 4)?).map_err(|_| bad())?;
    let instant = UtcTime::from_civil(
        year,
        number(month, 2)?,
        number(day, 2)?,
        hour,
        minute,
        f64::from(second),
    )
    .map_err(|error| Failure::Input(format!("{text}: {error}")))?;
    // Whole seconds of years 0 to 9999: exact, and far inside an i64.
    Ok(instant.unix_seconds() as i64)
}

/// Where an answer was read from, as one line of text.
pub(crate) fn read_from_line(read_from: &ReadFrom) -> String {
    match read_from {
        // The name alone, as `hpr analyze` prints a log's; the JSON has the path.
        ReadFrom::File { path } => format!(
            "Read from {}",
            std::path::Path::new(path)
                .file_name()
                .map_or(path.clone(), |name| name.to_string_lossy().into_owned())
        ),
        ReadFrom::Network { .. } => "Fetched now".to_owned(),
        ReadFrom::Cache { fetched_at_unix_s } => format!(
            "From the cache, fetched {}",
            format_utc(i64::try_from(*fetched_at_unix_s).unwrap_or(i64::MAX))
        ),
        ReadFrom::StaleCache {
            fetched_at_unix_s,
            reason,
        } => format!(
            "From the cache, stale, fetched {}{}",
            format_utc(i64::try_from(*fetched_at_unix_s).unwrap_or(i64::MAX)),
            reason
                .as_ref()
                .map_or(String::new(), |reason| format!(": {reason}"))
        ),
    }
}

/// Writes seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`, with Hinnant's
/// `civil_from_days` (<https://howardhinnant.github.io/date_algorithms.html>, public domain).
pub(crate) fn format_utc(unix_s: i64) -> String {
    let (days, seconds) = (unix_s.div_euclid(86_400), unix_s.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_and_write_back() {
        for (text, unix_s) in [
            ("1970-01-01T00Z", 0),
            ("2025-06-21T15:30Z", 1_750_519_800),
            ("2026-09-30T00:00:00Z", 1_790_726_400),
            ("2000-02-29T23:59:59Z", 951_868_799),
        ] {
            assert_eq!(parse_utc(text).ok(), Some(unix_s), "{text}");
        }
        assert_eq!(format_utc(1_750_519_800), "2025-06-21T15:30:00Z");
        assert_eq!(format_utc(951_868_799), "2000-02-29T23:59:59Z");
        assert_eq!(format_utc(-1), "1969-12-31T23:59:59Z");
        // Every day of four centuries, at a second before midnight, round-trips.
        for day in -73_000_i64..73_000 {
            let unix_s = day * 86_400 + 86_399;
            assert_eq!(parse_utc(&format_utc(unix_s)).ok(), Some(unix_s));
        }
    }

    #[test]
    fn malformed_times_are_refused() {
        for text in [
            "2025-06-21 15:30Z",
            "2025-06-21T15:30",
            "2025-06-21T15:30+00:00",
            "2025-6-21T15:30Z",
            "2025-06-31T15:30Z",
            "2025-02-29T00Z",
            "2025-06-21T24:00Z",
            "2025-06-21T15:60Z",
            "2025-06-21T15:30:60Z",
            "2025-06-21T15:3Z",
            "2025-06-21T+5:30Z",
            "20250-06-21T15:30Z",
            "2025-06-21T15:30:00:00Z",
            "",
        ] {
            assert!(parse_utc(text).is_err(), "{text}");
        }
    }
}
