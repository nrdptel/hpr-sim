//! Motor stock and prices from [motor.fusionspace.co](https://motor.fusionspace.co)'s public API.
//!
//! The motor finder lists every AeroTech, Cesaroni and Loki motor that a dozen U.S. vendors carry,
//! with each vendor's stock and price. Its [API][api] is a handful of static JSON files, rebuilt
//! about hourly, with no key, no rate limit and no query parameters: a client fetches a whole file
//! and filters it itself. The files are [`Endpoint::Meta`] (when the data was built, and counts),
//! [`Endpoint::Motors`] (every motor some vendor lists, D class and up), [`Endpoint::InStock`]
//! (the same, only those in stock at one vendor or more), [`Endpoint::Vendors`], and one motor's
//! page ([`Endpoint::motor`]). Prices are whole cents; a listing's unit price is its sticker
//! price over its pack size. The motor data itself (classes, impulses, delays) comes from
//! ThrustCurve.org, and designations are spelled as ThrustCurve spells them.
//!
//! [`parse_meta`], [`parse_motors`], [`parse_in_stock`], [`parse_vendors`] and [`parse_motor`]
//! read an answer, refusing one of another schema version, one whose count disagrees with its
//! list, and a motor or listing that breaks the API's own rules (a pack of no motors, a unit price
//! over the sticker price, a cheapest offer on a motor out of stock). The `fetch_` functions ask a
//! [`Client`] for the file, so the answer comes from the cache when it can, and offline from the
//! cache only; an answer that doesn't parse is never cached. Stock moves by the hour, so a copy
//! stays fresh for [`TTL_S`], an hour. Show [`ATTRIBUTION`] (it is on every [`Fetched`]) wherever
//! a price or stock is shown: the API's terms ask for credit to motor.fusionspace.co, and say to
//! check stock and price on the vendor's own page before relying on them.
//!
//! **How far to trust it:** a value is the answer's, unchanged (`tests/motor_finder.rs` reads
//! every field of every recorded answer back). Whether a vendor really has a motor, at that price,
//! is the vendor's to say: the finder reads their public listings, up to an hour old.
//!
//! ```
//! use hpr_net::motor_finder;
//!
//! // The in-stock list as recorded on 2026-10-01 at 07:07 UTC.
//! let body = include_bytes!("../tests/fixtures/replay/motor-finder-in-stock.json");
//! let list = motor_finder::parse_in_stock(body)?;
//! let l_class: Vec<_> = list.motors.iter().filter(|m| m.impulse_class == "L").collect();
//! assert_eq!((list.motors.len(), l_class.len()), (282, 20));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [api]: https://github.com/nrdptel/Hobby-Rocket-Motor-Finder/blob/main/docs/api.md

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{Client, Fetched, NetError, Source, Transport};

/// The API's base URL, version 1.
pub const BASE_URL: &str = "https://motor.fusionspace.co/api/v1";

/// The schema version this module reads. A breaking change ships under a new path (`/api/v2/`),
/// so an answer of another version is refused.
pub const SCHEMA_VERSION: u32 = 1;

/// How long a cached answer counts as fresh, s: an hour, as often as the site rebuilds the files.
pub const TTL_S: u64 = 3_600;

/// The credit the API's terms ask for, with their caution about stock and price.
pub const ATTRIBUTION: &str = "Motor stock and prices from motor.fusionspace.co, aggregated from \
                               public vendor listings and ThrustCurve.org; provided as is, with \
                               no warranty: check stock and price on the vendor's own page \
                               before relying on them";

/// The manufacturers the API lists: each one's name as the API writes it, and the slug its
/// motors' pages sit under.
pub const MANUFACTURERS: [(&str, &str); 3] = [
    ("AeroTech", "aerotech"),
    ("Cesaroni Technology", "cesaroni"),
    ("Loki Research", "loki"),
];

/// One of the API's files.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    /// `meta.json`: when the data was built, its counts and the endpoint index.
    Meta,
    /// `motors.json`: every motor a vendor lists.
    Motors,
    /// `in-stock.json`: the motors in stock at one vendor or more.
    InStock,
    /// `vendors.json`: the vendors read.
    Vendors,
    /// One motor's page. Build it with [`Endpoint::motor`].
    Motor {
        /// The manufacturer's slug, one of [`MANUFACTURERS`].
        manufacturer_slug: &'static str,
        /// The designation, as ThrustCurve spells it (`H128W`, `F27R/L`, `3683L851-P`).
        designation: String,
    },
}

impl Endpoint {
    /// One motor's page, by its manufacturer (the API's name or slug, any case) and designation.
    ///
    /// # Errors
    /// [`MotorFinderError::Request`] for a manufacturer not in [`MANUFACTURERS`], or a designation
    /// that is empty, all dots, or holds a character other than an ASCII letter, a digit, `-`,
    /// `_`, `.` or `/` (the characters ThrustCurve's designations use).
    pub fn motor(manufacturer: &str, designation: &str) -> Result<Self, MotorFinderError> {
        let manufacturer_slug = manufacturer_slug(manufacturer)?;
        let allowed = |c: char| c.is_ascii_alphanumeric() || "-_./".contains(c);
        if designation.is_empty()
            || designation.chars().all(|c| c == '.')
            || !designation.chars().all(allowed)
        {
            return Err(MotorFinderError::Request {
                what: "designation",
                value: designation.to_owned(),
            });
        }
        Ok(Self::Motor {
            manufacturer_slug,
            designation: designation.to_owned(),
        })
    }

    /// The file's URL under [`BASE_URL`]. A `/` in a motor's designation is written `~`, as the
    /// API names the file.
    #[must_use]
    pub fn url(&self) -> String {
        match self {
            Self::Meta => format!("{BASE_URL}/meta.json"),
            Self::Motors => format!("{BASE_URL}/motors.json"),
            Self::InStock => format!("{BASE_URL}/in-stock.json"),
            Self::Vendors => format!("{BASE_URL}/vendors.json"),
            Self::Motor {
                manufacturer_slug,
                designation,
            } => format!(
                "{BASE_URL}/motors/{manufacturer_slug}/{}.json",
                designation.replace('/', "~")
            ),
        }
    }

    /// The [`Source`] a [`Client`] caches every file under: the motor finder, its
    /// [`ATTRIBUTION`], and [`TTL_S`].
    #[must_use]
    pub fn source() -> Source {
        Source {
            name: "motor.fusionspace.co".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s: TTL_S,
        }
    }
}

/// `meta.json`: when the data was built, and how much there is.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    /// The schema version, [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// When the files were built, ISO 8601 UTC (`2026-10-01T07:07:29+00:00`).
    pub generated_at: String,
    /// How many motors, motors in stock and vendors the files hold.
    pub counts: Counts,
    /// The manufacturers' names, as motors write them.
    pub manufacturers: Vec<String>,
    /// The endpoint index: each file's name and its path on the site.
    pub endpoints: BTreeMap<String, String>,
    /// Where the API is documented.
    pub docs: Option<String>,
    /// The terms, in the API's words.
    pub license: Option<String>,
    /// Notes on how the API is served.
    pub notes: Option<String>,
}

/// [`Meta`]'s counts.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// Motors in `motors.json`.
    pub motors: u32,
    /// Motors in `in-stock.json`.
    pub in_stock: u32,
    /// Vendors in `vendors.json`.
    pub vendors: u32,
}

/// `motors.json` or `in-stock.json`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorList {
    /// The schema version, [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// When the file was built, ISO 8601 UTC.
    pub generated_at: String,
    /// How many motors it holds.
    pub count: u32,
    /// The motors.
    pub motors: Vec<Motor>,
}

/// One motor's page, `motors/{manufacturer}/{designation}.json`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorPage {
    /// The schema version, [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// When the file was built, ISO 8601 UTC.
    pub generated_at: String,
    /// The motor.
    pub motor: Motor,
}

/// A motor, with every vendor's listing of it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Motor {
    /// The finder's own id for the motor, stable across builds; not ThrustCurve's.
    pub id: u64,
    /// The path of the motor's own page on the site.
    pub path: String,
    /// The manufacturer's name, one of [`MANUFACTURERS`] today.
    pub manufacturer: String,
    /// The designation, as ThrustCurve spells it (`H128W`, `3683L851-P`).
    pub designation: String,
    /// The designation without its propellant code (`H128`).
    pub common_name: Option<String>,
    /// The impulse class, one letter (`D` to `O` today).
    pub impulse_class: String,
    /// The motor's diameter, mm.
    pub diameter_mm: f64,
    /// Total impulse, N·s.
    pub total_impulse_ns: Option<f64>,
    /// Average thrust, N.
    pub avg_thrust_n: Option<f64>,
    /// Burn time, s.
    pub burn_time_s: Option<f64>,
    /// The propellant's trade name (`White Lightning`).
    pub propellant: Option<String>,
    /// Whether the propellant throws sparks (a metal additive).
    pub sparky: bool,
    /// A reload, a single-use motor or a hybrid.
    pub motor_type: Option<MotorType>,
    /// The reload hardware it fits (`RMS-29/180`); none for a single-use motor.
    pub case_info: Option<String>,
    /// Whether it ships as hazardous material.
    pub hazmat: Hazmat,
    /// The delays it comes with, s, comma-separated, or `P` for plugged.
    pub delays: Option<String>,
    /// Whether the delay can be shortened by the flyer.
    pub delay_adjustable: bool,
    /// Out of production: old stock only.
    pub discontinued: bool,
    /// In stock at one vendor or more.
    pub in_stock: bool,
    /// How many vendors list it, in stock or not.
    pub vendor_count: u32,
    /// How many vendors have it in stock.
    pub in_stock_vendor_count: u32,
    /// How many listings it has: a vendor may list several (delays, packs).
    pub listing_count: u32,
    /// The in-stock listing with the lowest unit price; `None` exactly when out of stock.
    pub cheapest_in_stock: Option<Offer>,
    /// Every listing.
    pub listings: Vec<Listing>,
}

/// How a motor is built.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotorType {
    /// Reloadable: a propellant kit for reusable hardware.
    #[serde(rename = "reload")]
    Reload,
    /// Single-use.
    #[serde(rename = "SU")]
    SingleUse,
    /// Hybrid.
    #[serde(rename = "hybrid")]
    Hybrid,
}

/// Whether a motor ships as hazardous material.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hazmat {
    /// Always: over 62.5 g of propellant, H class and up.
    #[serde(rename = "required")]
    Required,
    /// It depends on the vendor: F and G motors near the limit.
    #[serde(rename = "varies")]
    Varies,
    /// Never: 62.5 g of propellant or less, A to E.
    #[serde(rename = "none")]
    NotRequired,
}

/// The cheapest in-stock offer of a motor.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    /// The sticker price, cents.
    pub price_cents: u64,
    /// The price of one motor, cents: the sticker price over the pack size.
    pub unit_price_cents: u64,
    /// The currency (`USD`).
    pub currency: String,
    /// The vendor's name.
    pub vendor: String,
    /// The vendor's slug, as [`Vendor::slug`].
    pub vendor_slug: String,
    /// The product page.
    pub url: String,
    /// How many motors the pack holds.
    pub pack_size: u32,
}

/// One vendor's listing of a motor.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Listing {
    /// The vendor's name.
    pub vendor: String,
    /// The vendor's slug, as [`Vendor::slug`].
    pub vendor_slug: String,
    /// The product page.
    pub url: String,
    /// In stock, out of stock, special order or unknown.
    pub status: ListingStatus,
    /// The sticker price, cents.
    pub price_cents: Option<u64>,
    /// The price of one motor, cents: the sticker price over the pack size.
    pub unit_price_cents: Option<u64>,
    /// The currency (`USD`).
    pub currency: String,
    /// How many motors the pack holds.
    pub pack_size: u32,
    /// Units on hand, when the vendor shows them.
    pub stock_count: Option<u64>,
    /// The wait on a back order (`16–20 weeks`).
    pub lead_time: Option<String>,
    /// When the finder last read the listing, ISO 8601 UTC.
    pub last_seen: String,
}

/// A listing's stock.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    /// In stock, whether or not a count is shown.
    InStock,
    /// Out of stock.
    OutOfStock,
    /// Made or ordered for the buyer, with a lead time.
    SpecialOrder,
    /// The vendor's page doesn't say.
    Unknown,
}

/// `vendors.json`.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorList {
    /// The schema version, [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// When the file was built, ISO 8601 UTC.
    pub generated_at: String,
    /// How many vendors it holds.
    pub count: u32,
    /// The vendors.
    pub vendors: Vec<Vendor>,
}

/// A vendor the finder reads.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vendor {
    /// Its slug, as listings name it (`csrocketry`).
    pub slug: String,
    /// Its name (`Chris' Rocket Supplies`).
    pub name: String,
    /// How many motors it lists.
    pub motor_count: u32,
    /// How many of them it has in stock.
    pub in_stock_count: u32,
}

/// Reads `meta.json`.
///
/// # Errors
/// [`MotorFinderError::Json`] when the body is not JSON of this shape; [`MotorFinderError::Schema`]
/// for another schema version; [`MotorFinderError::Field`] for a `generated_at` that isn't an
/// ISO 8601 UTC time.
pub fn parse_meta(body: &[u8]) -> Result<Meta, MotorFinderError> {
    let meta: Meta = json(body)?;
    header(meta.schema_version, &meta.generated_at)?;
    Ok(meta)
}

/// Reads `motors.json`.
///
/// # Errors
/// As [`parse_meta`]; [`MotorFinderError::Count`] when `count` disagrees with the list; and
/// [`MotorFinderError::Field`] for a motor or listing that breaks the API's rules: an impulse
/// class that isn't one capital letter, a diameter, impulse, thrust or burn time below zero (or a
/// diameter of zero), a `listing_count` other than its listings', a cheapest offer on a motor out
/// of stock or none on one in stock, a pack of no motors, or a unit price over its sticker price.
pub fn parse_motors(body: &[u8]) -> Result<MotorList, MotorFinderError> {
    let list: MotorList = json(body)?;
    header(list.schema_version, &list.generated_at)?;
    let found = list.motors.len();
    if usize::try_from(list.count).ok() != Some(found) {
        return Err(MotorFinderError::Count {
            expected: list.count,
            found,
        });
    }
    for (i, motor) in list.motors.iter().enumerate() {
        check_motor(motor, &format!("motors[{i}]"))?;
    }
    Ok(list)
}

/// Reads `in-stock.json`: as [`parse_motors`], and every motor must be in stock.
///
/// # Errors
/// As [`parse_motors`], and [`MotorFinderError::Field`] naming a motor out of stock.
pub fn parse_in_stock(body: &[u8]) -> Result<MotorList, MotorFinderError> {
    let list = parse_motors(body)?;
    if let Some(i) = list.motors.iter().position(|m| !m.in_stock) {
        return Err(field(format!("motors[{i}].in_stock"), "false"));
    }
    Ok(list)
}

/// Reads `vendors.json`.
///
/// # Errors
/// As [`parse_meta`], and [`MotorFinderError::Count`] when `count` disagrees with the list.
pub fn parse_vendors(body: &[u8]) -> Result<VendorList, MotorFinderError> {
    let list: VendorList = json(body)?;
    header(list.schema_version, &list.generated_at)?;
    let found = list.vendors.len();
    if usize::try_from(list.count).ok() != Some(found) {
        return Err(MotorFinderError::Count {
            expected: list.count,
            found,
        });
    }
    Ok(list)
}

/// Reads one motor's page.
///
/// # Errors
/// As [`parse_meta`], and [`MotorFinderError::Field`] for a motor that breaks the rules
/// [`parse_motors`] lists.
pub fn parse_motor(body: &[u8]) -> Result<MotorPage, MotorFinderError> {
    let page: MotorPage = json(body)?;
    header(page.schema_version, &page.generated_at)?;
    check_motor(&page.motor, "motor")?;
    Ok(page)
}

/// Fetches `meta.json` through `client`.
///
/// The answer comes from the client's cache while fresh (see [`Endpoint::source`]); offline,
/// from the cache only. The [`Fetched`] says which, carries [`ATTRIBUTION`] and holds the body.
/// Only an answer that [`parse_meta`] reads is cached ([`Client::fetch_checked`]): one that
/// doesn't never takes a good copy's place, and online a stale good copy is returned instead,
/// with the reason. The other `fetch_` functions work the same way.
///
/// # Errors
/// [`MotorFinderError::Net`] when the fetch fails, or with [`NetError::Refused`] naming what the
/// parser refused when the only answer there is doesn't parse.
pub fn fetch_meta<T: Transport>(
    client: &Client<T>,
    now_s: u64,
) -> Result<(Meta, Fetched), MotorFinderError> {
    fetch(client, &Endpoint::Meta, now_s, parse_meta)
}

/// Fetches `motors.json` through `client`, as [`fetch_meta`] does.
///
/// # Errors
/// As [`fetch_meta`].
pub fn fetch_motors<T: Transport>(
    client: &Client<T>,
    now_s: u64,
) -> Result<(MotorList, Fetched), MotorFinderError> {
    fetch(client, &Endpoint::Motors, now_s, parse_motors)
}

/// Fetches `in-stock.json` through `client`, as [`fetch_meta`] does.
///
/// # Errors
/// As [`fetch_meta`].
pub fn fetch_in_stock<T: Transport>(
    client: &Client<T>,
    now_s: u64,
) -> Result<(MotorList, Fetched), MotorFinderError> {
    fetch(client, &Endpoint::InStock, now_s, parse_in_stock)
}

/// Fetches `vendors.json` through `client`, as [`fetch_meta`] does.
///
/// # Errors
/// As [`fetch_meta`].
pub fn fetch_vendors<T: Transport>(
    client: &Client<T>,
    now_s: u64,
) -> Result<(VendorList, Fetched), MotorFinderError> {
    fetch(client, &Endpoint::Vendors, now_s, parse_vendors)
}

/// Fetches one motor's page through `client`, as [`fetch_meta`] does. A page for another motor
/// than the one asked for is refused, and never cached.
///
/// # Errors
/// [`MotorFinderError::Request`] for a bad manufacturer or designation ([`Endpoint::motor`]); as
/// [`fetch_meta`] otherwise. An unknown motor is the site's "not found" page, which `Http`
/// reports as [`NetError::Transport`] naming status 404.
pub fn fetch_motor<T: Transport>(
    client: &Client<T>,
    manufacturer: &str,
    designation: &str,
    now_s: u64,
) -> Result<(Motor, Fetched), MotorFinderError> {
    let endpoint = Endpoint::motor(manufacturer, designation)?;
    let asked_slug = manufacturer_slug(manufacturer)?;
    let read = |body: &[u8]| {
        let page = parse_motor(body)?;
        let slug = MANUFACTURERS
            .iter()
            .find(|(name, _)| *name == page.motor.manufacturer)
            .map(|&(_, slug)| slug);
        if slug != Some(asked_slug) || page.motor.designation != designation {
            return Err(MotorFinderError::OtherMotor {
                asked: format!("{asked_slug}/{designation}"),
                found: format!("{}/{}", page.motor.manufacturer, page.motor.designation),
            });
        }
        Ok(page.motor)
    };
    fetch(client, &endpoint, now_s, read)
}

/// The slug of a manufacturer in [`MANUFACTURERS`], by its name or slug, any case.
fn manufacturer_slug(manufacturer: &str) -> Result<&'static str, MotorFinderError> {
    MANUFACTURERS
        .iter()
        .find(|(name, slug)| {
            manufacturer.eq_ignore_ascii_case(name) || manufacturer.eq_ignore_ascii_case(slug)
        })
        .map(|&(_, slug)| slug)
        .ok_or_else(|| MotorFinderError::Request {
            what: "manufacturer",
            value: manufacturer.to_owned(),
        })
}

/// Fetches `endpoint` through `client`, caching only an answer `read` reads.
fn fetch<T: Transport, R>(
    client: &Client<T>,
    endpoint: &Endpoint,
    now_s: u64,
    read: impl Fn(&[u8]) -> Result<R, MotorFinderError>,
) -> Result<(R, Fetched), MotorFinderError> {
    let check = |body: &[u8]| read(body).map(drop).map_err(|e| e.to_string());
    let fetched = client.fetch_checked(&Endpoint::source(), &endpoint.url(), now_s, check)?;
    let value = read(&fetched.body)?;
    Ok((value, fetched))
}

/// Seconds since the Unix epoch of an ISO 8601 UTC time as the API writes it,
/// `YYYY-MM-DDTHH:MM:SS` followed by `Z` or `+00:00`, with optional fractional seconds dropped;
/// `None` for anything else.
#[must_use]
pub fn unix_s(timestamp: &str) -> Option<i64> {
    let rest = timestamp
        .strip_suffix('Z')
        .or_else(|| timestamp.strip_suffix("+00:00"))?;
    let (date, time) = rest.split_once('T')?;
    let time = time.split_once('.').map_or(time, |(whole, fraction)| {
        if !fraction.is_empty() && fraction.bytes().all(|b| b.is_ascii_digit()) {
            whole
        } else {
            ""
        }
    });
    let number = |text: &str, digits: usize| -> Option<i64> {
        (text.len() == digits && text.bytes().all(|b| b.is_ascii_digit()))
            .then(|| text.parse().ok())
            .flatten()
    };
    let mut parts = date.split('-');
    let (year, month, day) = (
        number(parts.next()?, 4)?,
        number(parts.next()?, 2)?,
        number(parts.next()?, 2)?,
    );
    let mut parts = time.split(':');
    let (hour, minute, second) = (
        number(parts.next()?, 2)?,
        number(parts.next()?, 2)?,
        number(parts.next()?, 2)?,
    );
    if parts.next().is_some() || date.split('-').count() != 3 {
        return None;
    }
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let start = crate::civil::unix_day_start(year, month, day)?;
    Some(start + hour * 3_600 + minute * 60 + second)
}

/// Deserializes `body`.
fn json<'a, D: Deserialize<'a>>(body: &'a [u8]) -> Result<D, MotorFinderError> {
    serde_json::from_slice(body).map_err(|e| MotorFinderError::Json(e.to_string()))
}

/// Checks the schema version and the build time every file carries.
fn header(schema_version: u32, generated_at: &str) -> Result<(), MotorFinderError> {
    if schema_version != SCHEMA_VERSION {
        return Err(MotorFinderError::Schema {
            found: schema_version,
        });
    }
    if unix_s(generated_at).is_none() {
        return Err(field("generated_at".to_owned(), generated_at));
    }
    Ok(())
}

/// A [`MotorFinderError::Field`].
fn field(field: String, value: impl ToString) -> MotorFinderError {
    MotorFinderError::Field {
        field,
        value: value.to_string(),
    }
}

/// Checks a motor against the API's rules; `at` names it in an error (`motors[12]`).
fn check_motor(motor: &Motor, at: &str) -> Result<(), MotorFinderError> {
    let class = motor.impulse_class.as_bytes();
    if !(class.len() == 1 && class[0].is_ascii_uppercase()) {
        return Err(field(format!("{at}.impulse_class"), &motor.impulse_class));
    }
    if motor.diameter_mm <= 0.0 {
        return Err(field(format!("{at}.diameter_mm"), motor.diameter_mm));
    }
    let figures = [
        ("total_impulse_ns", motor.total_impulse_ns),
        ("avg_thrust_n", motor.avg_thrust_n),
        ("burn_time_s", motor.burn_time_s),
    ];
    for (name, value) in figures {
        if let Some(value) = value.filter(|v| *v < 0.0) {
            return Err(field(format!("{at}.{name}"), value));
        }
    }
    if usize::try_from(motor.listing_count).ok() != Some(motor.listings.len()) {
        let value = format!(
            "{} for {} listings",
            motor.listing_count,
            motor.listings.len()
        );
        return Err(field(format!("{at}.listing_count"), value));
    }
    if motor.cheapest_in_stock.is_some() != motor.in_stock {
        let value = format!(
            "in_stock {} beside {:?}",
            motor.in_stock, motor.cheapest_in_stock
        );
        return Err(field(format!("{at}.cheapest_in_stock"), value));
    }
    if let Some(offer) = &motor.cheapest_in_stock {
        let at = format!("{at}.cheapest_in_stock");
        check_price(
            &at,
            Some(offer.price_cents),
            Some(offer.unit_price_cents),
            offer.pack_size,
        )?;
    }
    for (j, listing) in motor.listings.iter().enumerate() {
        let at = format!("{at}.listings[{j}]");
        check_price(
            &at,
            listing.price_cents,
            listing.unit_price_cents,
            listing.pack_size,
        )?;
    }
    Ok(())
}

/// Checks a price: a pack of one motor or more, and a unit price no more than the sticker price.
fn check_price(
    at: &str,
    price_cents: Option<u64>,
    unit_price_cents: Option<u64>,
    pack_size: u32,
) -> Result<(), MotorFinderError> {
    if pack_size == 0 {
        return Err(field(format!("{at}.pack_size"), pack_size));
    }
    if let (Some(price), Some(unit)) = (price_cents, unit_price_cents)
        && unit > price
    {
        let value = format!("{unit} over a price of {price}");
        return Err(field(format!("{at}.unit_price_cents"), value));
    }
    Ok(())
}

/// Why a motor finder request or answer was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MotorFinderError {
    /// A request field is not one the API serves.
    #[error("the motor finder request's {what} is not one it serves: {value:?}")]
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
    #[error("the motor finder answer is not of the expected shape: {0}")]
    Json(String),
    /// The answer is of another schema version than [`SCHEMA_VERSION`].
    #[error("the motor finder answer is schema version {found}, not {SCHEMA_VERSION}")]
    Schema {
        /// The version found.
        found: u32,
    },
    /// A list's `count` disagrees with its length.
    #[error("the motor finder answer counts {expected} but lists {found}")]
    Count {
        /// The `count` stated.
        expected: u32,
        /// The entries listed.
        found: usize,
    },
    /// A field breaks the API's rules.
    #[error("the motor finder answer's {field} is not usable: {value}")]
    Field {
        /// Where it is (`motors[12].listings[3].pack_size`).
        field: String,
        /// Its value.
        value: String,
    },
    /// A motor's page is another motor's.
    #[error("asked the motor finder for {asked}, but its page is {found}'s")]
    OtherMotor {
        /// The manufacturer's slug and designation asked for.
        asked: String,
        /// The manufacturer and designation the page holds.
        found: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_follow_the_apis_names() {
        assert_eq!(
            Endpoint::InStock.url(),
            "https://motor.fusionspace.co/api/v1/in-stock.json"
        );
        let f27 = Endpoint::motor("AeroTech", "F27R/L").unwrap();
        assert_eq!(
            f27.url(),
            "https://motor.fusionspace.co/api/v1/motors/aerotech/F27R~L.json"
        );
        let cti = Endpoint::motor("CESARONI", "3683L851-P").unwrap();
        assert!(cti.url().ends_with("/motors/cesaroni/3683L851-P.json"));
        let loki = Endpoint::motor("Loki Research", "D2.3T").unwrap();
        assert!(loki.url().ends_with("/motors/loki/D2.3T.json"));
    }

    #[test]
    fn requests_outside_the_apis_files_are_refused() {
        let refusal = |manufacturer: &str, designation: &str| match Endpoint::motor(
            manufacturer,
            designation,
        ) {
            Err(MotorFinderError::Request { what, value }) => (what, value),
            other => panic!("{manufacturer}/{designation}: {other:?}"),
        };
        assert_eq!(refusal("Estes", "C6"), ("manufacturer", "Estes".to_owned()));
        for designation in [
            "", ".", "..", "H128W?x", "H128W#x", "a b", "H128W%2F", "~", "é",
        ] {
            let expected = ("designation", designation.to_owned());
            assert_eq!(refusal("aerotech", designation), expected);
        }
    }

    #[test]
    fn times_read_as_the_api_writes_them() {
        assert_eq!(unix_s("2026-10-01T07:07:29+00:00"), Some(1_790_838_449));
        assert_eq!(unix_s("2026-10-01T07:07:29Z"), Some(1_790_838_449));
        assert_eq!(unix_s("2026-10-01T07:07:29.123Z"), Some(1_790_838_449));
        assert_eq!(unix_s("1970-01-01T00:00:00Z"), Some(0));
        for bad in [
            "2026-10-01T07:07:29",
            "2026-10-01T07:07:29+01:00",
            "2026-10-01 07:07:29Z",
            "2026-13-01T07:07:29Z",
            "2026-02-30T07:07:29Z",
            "2026-10-01T24:00:00Z",
            "2026-10-01T07:60:00Z",
            "2026-10-01T07:07:60Z",
            "2026-10-01T07:07:29.Z",
            "2026-10-01T07:07:29.1aZ",
            "2026-10-01T07:07Z",
            "2026-10-01T07:07:29:00Z",
            "26-10-01T07:07:29Z",
            "2026-10-1T07:07:29Z",
            "+026-10-01T07:07:29Z",
            "",
        ] {
            assert_eq!(unix_s(bad), None, "{bad}");
        }
    }
}
