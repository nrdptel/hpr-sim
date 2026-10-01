//! A launch site's elevation (height above sea level) from Open-Meteo's elevation API.
//!
//! [Open-Meteo's elevation API](https://open-meteo.com/en/docs/elevation-api)
//! (`api.open-meteo.com/v1/elevation`) answers the height at up to [`MAX_PLACES`] places in one
//! request, as `{"elevation":[1400.0]}`, one number per place in the order asked. Its data is the
//! Copernicus DEM GLO-90 (2021 release), a digital elevation model on a grid 3 arc-seconds apart
//! in latitude, about 93 m; in longitude its spacing narrows from 93 m at the equator to 60 m at
//! 50°, then widens in steps (the [product handbook][handbook], issue 5.0, Table 3, p. 15). It is
//! a *surface* model: its heights include buildings and vegetation, so over a tree line or
//! buildings they sit above the bare ground. Its heights are above the EGM2008 geoid (mean sea
//! level), not the WGS 84 ellipsoid (§1.2.1, p. 13): a site's ellipsoidal height is `h = H + N`,
//! with `N` the geoid undulation there, which hpr has no model for ([geodesy notes][geodesy]).
//! The ocean has no tiles and reads 0 m.
//!
//! An [`ElevationRequest`] names the places; [`parse`] reads the answer, refusing one with the
//! wrong number of heights, a height that is not a number, or a height outside
//! [`HEIGHT_RANGE_M`]. [`fetch`] asks a [`Client`] for the URL, so the answer comes from the cache
//! when it can, and offline from the cache only; an answer that doesn't parse is never cached.
//! The cache key is the whole request: the same places, in the same order. The URL writes each
//! coordinate to 5 decimals (about 1 m), so a place given to 8 decimals or fewer and rebuilt from
//! radians finds its cached answer. The ground doesn't move, so a copy stays fresh for [`TTL_S`],
//! a year. Show [`ATTRIBUTION`] (it is on every [`Fetched`]) wherever the height is shown.
//!
//! **How far to trust it:** a height is the answer's number, unchanged (`tests/elevation.rs`).
//! The recorded heights are whole metres; Open-Meteo doesn't document its rounding. The handbook
//! states the DEM's absolute vertical accuracy as under 4 m (90% linear error), a global mean
//! outside Antarctica and Greenland (Table 1, p. 10); in 184 of the 16,363 geotiles there, each
//! about a degree across (1.1%; the table's 0.9% is of all tiles), it is over 10 m (Table 12,
//! p. 31). Nothing here measures it. The [guide page][guide]
//! says more.
//!
//! ```
//! use hpr_net::elevation::{self, Place};
//!
//! // An answer recorded for Spaceport America's launch area, 32.99° N, 106.97° W.
//! let body = include_bytes!("../tests/fixtures/replay/open-meteo-elevation.json");
//! let spaceport = Place::new(32.99, -106.97);
//! let heights = elevation::parse(body, &[spaceport])?;
//! assert_eq!((heights[0].place, heights[0].height_msl_m), (spaceport, 1400.0));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [handbook]: https://dataspace.copernicus.eu/sites/default/files/media/files/2024-06/geo1988-copernicusdem-spe-002_producthandbook_i5.0.pdf
//! [geodesy]: https://nrdptel.github.io/hpr-sim/physics/geodesy.html
//! [guide]: https://nrdptel.github.io/hpr-sim/elevation.html

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Client, Fetched, NetError, Source, Transport};

/// Open-Meteo's elevation endpoint.
pub const ENDPOINT: &str = "https://api.open-meteo.com/v1/elevation";

/// The most places Open-Meteo answers in one request.
pub const MAX_PLACES: usize = 100;

/// How long a cached answer counts as fresh, s: a year. The model behind it changes with a new
/// DEM release, years apart.
pub const TTL_S: u64 = 365 * 86_400;

/// The heights [`parse`] accepts, m above mean sea level. The lowest land, by the Dead Sea, lies a
/// little over 400 m below sea level, and the highest, Everest's summit, 8,849 m above it; the
/// margins leave room for the DEM's own errors. A height outside them is a broken answer, such as
/// a 16-bit no-data value.
pub const HEIGHT_RANGE_M: std::ops::RangeInclusive<f64> = -1_000.0..=9_000.0;

/// The decimals each coordinate is written with in the URL: 1e-5° is at most 1.1 m on the ground,
/// far inside the DEM's 90 m cells.
const URL_DECIMALS: u32 = 5;

/// The decimals a coordinate is first written to, exactly, before it is rounded to
/// [`URL_DECIMALS`]. A trip through radians moves a value by a few units in its last place (under
/// 1e-13°), so a value given to 8 decimals or fewer is written the same after one.
const FIRST_DECIMALS: u32 = 9;

/// The credit Open-Meteo's licence (CC BY 4.0) asks for: itself, and the Copernicus programme
/// whose DEM it serves, in the DEM licence's words.
pub const ATTRIBUTION: &str = "Elevation data by Open-Meteo.com (CC BY 4.0), from the Copernicus \
                               DEM GLO-90: © DLR e.V. 2010-2014 and © Airbus Defence and Space \
                               GmbH 2014-2018 provided under COPERNICUS by the European Union and \
                               ESA; all rights reserved";

/// A place on the ground.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// Latitude, degrees north, in `[-90, 90]`.
    pub latitude_deg: f64,
    /// Longitude, degrees east, in `[-180, 180]`.
    pub longitude_deg: f64,
}

impl Place {
    /// A place from its latitude and longitude, degrees.
    #[must_use]
    pub fn new(latitude_deg: f64, longitude_deg: f64) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
        }
    }
}

/// A place's height, as the elevation API answers it.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Elevation {
    /// The place, as asked for.
    pub place: Place,
    /// The DEM's height there, m above mean sea level (the EGM2008 geoid), not the ellipsoid.
    pub height_msl_m: f64,
}

/// What to ask Open-Meteo's elevation API for: one place or up to [`MAX_PLACES`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElevationRequest {
    /// The places, in the order their heights come back.
    pub places: Vec<Place>,
    /// Another server's endpoint in place of [`ENDPOINT`], for a self-hosted Open-Meteo; `None`
    /// for Open-Meteo's own. It must hold no `?` or `#`.
    pub endpoint: Option<String>,
}

impl ElevationRequest {
    /// A request to Open-Meteo's own server for `places`.
    #[must_use]
    pub fn new(places: Vec<Place>) -> Self {
        Self {
            places,
            endpoint: None,
        }
    }

    /// The URL that asks for every place's height: the latitudes, then the longitudes, each
    /// comma-separated in the places' order and written to 5 decimals, trailing zeros dropped.
    ///
    /// # Errors
    /// [`ElevationError::Request`] when there are no places or more than [`MAX_PLACES`], a place
    /// is outside the range its doc gives (the value names the place's index), or the endpoint is
    /// empty or holds a `?` or `#`.
    pub fn url(&self) -> Result<String, ElevationError> {
        let refuse = |what: &'static str, value: String| ElevationError::Request { what, value };
        if !(1..=MAX_PLACES).contains(&self.places.len()) {
            return Err(refuse("number of places", self.places.len().to_string()));
        }
        for (i, place) in self.places.iter().enumerate() {
            if !(-90.0..=90.0).contains(&place.latitude_deg) {
                let value = format!("{} (place {i})", place.latitude_deg);
                return Err(refuse("latitude (deg)", value));
            }
            if !(-180.0..=180.0).contains(&place.longitude_deg) {
                let value = format!("{} (place {i})", place.longitude_deg);
                return Err(refuse("longitude (deg)", value));
            }
        }
        let endpoint = match &self.endpoint {
            Some(endpoint) if endpoint.is_empty() || endpoint.contains(['?', '#']) => {
                return Err(refuse("endpoint", endpoint.clone()));
            }
            Some(endpoint) => endpoint.as_str(),
            None => ENDPOINT,
        };
        let join = |value: fn(&Place) -> f64| {
            let values: Vec<String> = self.places.iter().map(|p| coordinate(value(p))).collect();
            values.join(",")
        };
        Ok(format!(
            "{endpoint}?latitude={}&longitude={}",
            join(|p| p.latitude_deg),
            join(|p| p.longitude_deg),
        ))
    }

    /// The [`Source`] a [`Client`] caches this request's answer under: Open-Meteo's elevation,
    /// its [`ATTRIBUTION`], and [`TTL_S`].
    #[must_use]
    pub fn source(&self) -> Source {
        Source {
            name: "Open-Meteo elevation".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s: TTL_S,
        }
    }
}

/// A coordinate as the URL writes it: its [`FIRST_DECIMALS`]-decimal value rounded to
/// [`URL_DECIMALS`] decimals, halves away from zero, trailing zeros and a bare point dropped, and a
/// value that rounds to zero written as `0`. A value given to more than 9 decimals is so rounded
/// twice, by at most 1e-5° in all.
///
/// Rounding the binary value straight to 5 decimals would let a trip through radians flip a value
/// that sits on a half step (32.990415 is 32.990415000000006 after one): it is written to
/// [`FIRST_DECIMALS`] first, exactly, and that decimal is rounded. `deg` is finite and within
/// ±180 (checked by [`ElevationRequest::url`]).
fn coordinate(deg: f64) -> String {
    let fixed = format!("{:.*}", FIRST_DECIMALS as usize, deg.abs());
    // The digits of `|deg|` in units of 10^-FIRST_DECIMALS: under 2e11 for a checked coordinate,
    // and saturating rather than overflowing for any other.
    let units = fixed
        .bytes()
        .filter(u8::is_ascii_digit)
        .fold(0_u64, |n, b| {
            n.saturating_mul(10).saturating_add(u64::from(b - b'0'))
        });
    let step = 10_u64.pow(FIRST_DECIMALS - URL_DECIMALS);
    let kept = units.saturating_add(step / 2) / step;
    let one = 10_u64.pow(URL_DECIMALS);
    let sign = if kept != 0 && deg < 0.0 { "-" } else { "" };
    let (whole, fraction) = (kept / one, kept % one);
    if fraction == 0 {
        return format!("{sign}{whole}");
    }
    let digits = format!("{fraction:05}");
    format!("{sign}{whole}.{}", digits.trim_end_matches('0'))
}

/// Reads an answer of the elevation API for `places`: each place's height, m above mean sea level
/// (the EGM2008 geoid), in the order asked.
///
/// # Errors
/// [`ElevationError::Json`] when the body is not JSON; [`ElevationError::Server`] for Open-Meteo's
/// error answer; [`ElevationError::Missing`] when `elevation` is not an array of numbers;
/// [`ElevationError::Count`] when it holds another number of heights than places;
/// [`ElevationError::OutOfRange`] for a height outside [`HEIGHT_RANGE_M`].
pub fn parse(body: &[u8], places: &[Place]) -> Result<Vec<Elevation>, ElevationError> {
    let json: Value =
        serde_json::from_slice(body).map_err(|e| ElevationError::Json(e.to_string()))?;
    if json.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = json
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("no reason given");
        return Err(ElevationError::Server {
            reason: reason.to_owned(),
        });
    }
    let missing = |field: String| ElevationError::Missing { field };
    let heights = json
        .get("elevation")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("elevation".to_owned()))?;
    if heights.len() != places.len() {
        return Err(ElevationError::Count {
            expected: places.len(),
            found: heights.len(),
        });
    }
    heights
        .iter()
        .zip(places)
        .enumerate()
        .map(|(index, (height, &place))| {
            let height_msl_m = height
                .as_f64()
                .ok_or_else(|| missing(format!("elevation[{index}]")))?;
            if HEIGHT_RANGE_M.contains(&height_msl_m) {
                Ok(Elevation {
                    place,
                    height_msl_m,
                })
            } else {
                Err(ElevationError::OutOfRange {
                    index,
                    height_m: height_msl_m,
                })
            }
        })
        .collect()
}

/// Fetches `request` through `client` and reads each place's height above mean sea level.
///
/// The answer comes from the client's cache while fresh (see [`ElevationRequest::source`]);
/// offline, from the cache only. The [`Fetched`] says which, carries [`ATTRIBUTION`] and holds the
/// body. Only an answer that [`parse`] reads is cached ([`Client::fetch_checked`]): one that
/// doesn't never takes a good copy's place, and online a stale good copy is returned instead, with
/// the reason.
///
/// # Errors
/// [`ElevationError::Request`] for a bad request; [`ElevationError::Net`] when the fetch fails,
/// or with [`NetError::Refused`] naming what [`parse`] refused when the only answer there is
/// doesn't parse.
pub fn fetch<T: Transport>(
    client: &Client<T>,
    request: &ElevationRequest,
    now_s: u64,
) -> Result<(Vec<Elevation>, Fetched), ElevationError> {
    let url = request.url()?;
    let places = &request.places;
    let check = |body: &[u8]| parse(body, places).map(drop).map_err(|e| e.to_string());
    let fetched = client.fetch_checked(&request.source(), &url, now_s, check)?;
    let heights = parse(&fetched.body, places)?;
    Ok((heights, fetched))
}

/// Why an elevation request or answer was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ElevationError {
    /// A request field is outside its range.
    #[error("the elevation request's {what} is out of range: {value}")]
    Request {
        /// The field.
        what: &'static str,
        /// Its value; for a coordinate, followed by its place's index from 0, as `90.5 (place 1)`.
        value: String,
    },
    /// The fetch failed.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The body is not JSON.
    #[error("the Open-Meteo elevation answer is not JSON: {0}")]
    Json(String),
    /// The body is Open-Meteo's error answer. Open-Meteo sends it with HTTP status 400, which
    /// `Http` reports as [`NetError::Transport`] naming the status, without the body; this error
    /// comes only from a transport that hands the body on, or a self-hosted server that answers
    /// it with status 200.
    #[error("Open-Meteo refused the elevation request: {reason}")]
    Server {
        /// Its reason.
        reason: String,
    },
    /// A field is absent, or not of the expected type.
    #[error("the Open-Meteo elevation answer has no usable {field}")]
    Missing {
        /// The field.
        field: String,
    },
    /// The answer holds another number of heights than places asked for.
    #[error("the Open-Meteo elevation answer holds {found} heights, not the {expected} asked for")]
    Count {
        /// The places asked for.
        expected: usize,
        /// The heights in the answer.
        found: usize,
    },
    /// A height is outside [`HEIGHT_RANGE_M`].
    #[error("the Open-Meteo elevation answer's height {index} is out of range: {height_m} m")]
    OutOfRange {
        /// Its place's index in the request.
        index: usize,
        /// The height, m.
        height_m: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn places(places: &[(f64, f64)]) -> Vec<Place> {
        places
            .iter()
            .map(|&(lat, lon)| Place::new(lat, lon))
            .collect()
    }

    fn request(list: &[(f64, f64)]) -> ElevationRequest {
        ElevationRequest::new(places(list))
    }

    /// The field a request refusal names.
    fn refused(request: &ElevationRequest) -> (&'static str, String) {
        match request.url() {
            Err(ElevationError::Request { what, value }) => (what, value),
            other => panic!("not a request refusal: {other:?}"),
        }
    }

    /// Only the heights of an answer.
    fn heights(body: &[u8], list: &[(f64, f64)]) -> Vec<f64> {
        let read = parse(body, &places(list)).unwrap();
        assert_eq!(
            read.iter().map(|e| e.place).collect::<Vec<_>>(),
            places(list)
        );
        read.iter().map(|e| e.height_msl_m).collect()
    }

    #[test]
    fn the_url_lists_latitudes_then_longitudes_in_order() {
        let one = request(&[(32.99, -106.97)]);
        assert_eq!(
            one.url().unwrap(),
            "https://api.open-meteo.com/v1/elevation?latitude=32.99&longitude=-106.97"
        );
        let three = request(&[(32.99, -106.97), (31.5, 35.5), (0.0, -30.0)]);
        assert_eq!(
            three.url().unwrap(),
            "https://api.open-meteo.com/v1/elevation?latitude=32.99,31.5,0&longitude=-106.97,35.5,-30"
        );
        let mut own = one.clone();
        own.endpoint = Some("http://localhost:8080/v1/elevation".to_owned());
        assert_eq!(
            own.url().unwrap(),
            "http://localhost:8080/v1/elevation?latitude=32.99&longitude=-106.97"
        );
    }

    /// A place rebuilt from radians has other last digits (−106.91° comes back as
    /// −106.91000000000001); the URL, and so the cache key, is the same.
    #[test]
    fn the_url_survives_a_round_trip_through_radians() {
        let mut changed = 0;
        for hundredths in -18_000..=18_000 {
            let deg = f64::from(hundredths) / 100.0;
            let again = deg.to_radians().to_degrees();
            changed += usize::from(again.to_bits() != deg.to_bits());
            let lat = deg / 2.0;
            let back = (lat.to_radians().to_degrees(), again);
            assert_eq!(
                request(&[back]).url().unwrap(),
                request(&[(lat, deg)]).url().unwrap(),
                "{deg}"
            );
        }
        // The case is real: thousands of two-decimal longitudes change on the way.
        assert!(changed > 1_000, "{changed}");
        assert_eq!(coordinate(-106.910_000_000_000_01), "-106.91");
    }

    /// Values on a half step, given to 6 decimals (as a GPS gives them), and values given to 8,
    /// are written the same after a trip through radians. About 1 in 17 of the six-decimal values,
    /// all on a half step, would flip if rounded straight to 5 decimals (21,127 of 360,000 when
    /// measured), and 19 of the eight-decimal ones.
    #[test]
    fn half_steps_survive_a_round_trip_through_radians() {
        let mut changed = 0;
        let mut flips_if_rounded_straight = 0;
        for (units, scale) in (0..360_000)
            .map(|i: i64| (-179_999_995 + i * 1_000, 1e6))
            .chain((0..360_000).map(|i| (-17_999_999_999 + i * 99_999, 1e8)))
        {
            // Under 2^53, so exact as an `f64`.
            let deg = units as f64 / scale;
            let again = deg.to_radians().to_degrees();
            changed += usize::from(again.to_bits() != deg.to_bits());
            flips_if_rounded_straight += usize::from(format!("{deg:.5}") != format!("{again:.5}"));
            assert_eq!(coordinate(again), coordinate(deg), "{deg}");
        }
        assert!(changed > 10_000, "{changed}");
        assert!(
            flips_if_rounded_straight > 1_000,
            "{flips_if_rounded_straight}"
        );
    }

    #[test]
    fn coordinates_are_written_to_five_decimals() {
        assert_eq!(coordinate(0.0), "0");
        assert_eq!(coordinate(-0.0), "0");
        assert_eq!(coordinate(-0.000_001), "0");
        assert_eq!(coordinate(-0.000_01), "-0.00001");
        assert_eq!(coordinate(90.0), "90");
        assert_eq!(coordinate(-180.0), "-180");
        assert_eq!(coordinate(12.345_678), "12.34568");
        assert_eq!(coordinate(1.5), "1.5");
        // Halves round away from zero, carrying into the whole degrees.
        assert_eq!(coordinate(32.990_415), "32.99042");
        assert_eq!(coordinate(-106.969_225), "-106.96923");
        assert_eq!(coordinate(-0.000_005), "-0.00001");
        assert_eq!(coordinate(0.000_004_999), "0");
        assert_eq!(coordinate(179.999_995), "180");
        assert_eq!(coordinate(-179.999_996), "-180");
        assert_eq!(coordinate(89.999_996), "90");
    }

    #[test]
    fn a_request_out_of_range_names_its_field() {
        assert_eq!(refused(&request(&[])), ("number of places", "0".to_owned()));
        let full = vec![(0.0, 0.0); MAX_PLACES];
        assert!(request(&full).url().is_ok());
        let over = vec![(0.0, 0.0); MAX_PLACES + 1];
        assert_eq!(
            refused(&request(&over)),
            ("number of places", "101".to_owned())
        );
        assert_eq!(
            refused(&request(&[(0.0, 0.0), (90.5, 0.0)])),
            ("latitude (deg)", "90.5 (place 1)".to_owned())
        );
        assert_eq!(
            refused(&request(&[(f64::NAN, 0.0)])),
            ("latitude (deg)", "NaN (place 0)".to_owned())
        );
        assert_eq!(
            refused(&request(&[(-90.0, -180.5)])),
            ("longitude (deg)", "-180.5 (place 0)".to_owned())
        );
        assert_eq!(
            refused(&request(&[(0.0, f64::NAN)])),
            ("longitude (deg)", "NaN (place 0)".to_owned())
        );
        assert!(request(&[(-90.0, 180.0), (90.0, -180.0)]).url().is_ok());
        for endpoint in ["", "https://x.test/v1/elevation?a=1", "https://x.test/#e"] {
            let mut r = request(&[(0.0, 0.0)]);
            r.endpoint = Some(endpoint.to_owned());
            assert_eq!(refused(&r), ("endpoint", endpoint.to_owned()));
        }
    }

    #[test]
    fn the_source_carries_both_credits_and_a_year() {
        let source = request(&[(0.0, 0.0)]).source();
        assert_eq!(source.ttl_s, 31_536_000);
        assert!(
            source
                .attribution
                .starts_with("Elevation data by Open-Meteo.com (CC BY 4.0)")
        );
        assert!(source.attribution.contains("Copernicus DEM GLO-90"));
        assert!(source.attribution.ends_with("all rights reserved"));
        assert!(!source.attribution.contains("  "), "{}", source.attribution);
    }

    #[test]
    fn parse_reads_every_height_in_order() {
        let three = [(32.99, -106.97), (31.5, 35.5), (0.0, -30.0)];
        assert_eq!(
            heights(br#"{"elevation":[1400.0, -427.0, 0.0]}"#, &three),
            [1400.0, -427.0, 0.0]
        );
        // Another field beside `elevation` is ignored, as Open-Meteo may add some.
        assert_eq!(
            heights(
                br#"{"elevation":[12.5],"generationtime_ms":0.1}"#,
                &[(1.0, 2.0)]
            ),
            [12.5]
        );
        let edges = heights(br#"{"elevation":[-1000, 9000]}"#, &[(0.0, 0.0), (1.0, 1.0)]);
        assert_eq!(edges, [-1_000.0, 9_000.0]);
    }

    #[test]
    fn parse_refuses_a_broken_answer() {
        let at = |n: usize| vec![Place::new(0.0, 0.0); n];
        let error = |body: &[u8], n: usize| parse(body, &at(n)).unwrap_err();
        assert!(matches!(error(b"<html>", 1), ElevationError::Json(_)));
        let server =
            r#"{"error":true,"reason":"Latitude must be in range of -90 to 90°. Given: 95.0."}"#;
        match error(server.as_bytes(), 1) {
            ElevationError::Server { reason } => {
                assert_eq!(
                    reason,
                    "Latitude must be in range of -90 to 90°. Given: 95.0."
                );
            }
            other => panic!("{other:?}"),
        }
        match error(br#"{"error":true}"#, 1) {
            ElevationError::Server { reason } => assert_eq!(reason, "no reason given"),
            other => panic!("{other:?}"),
        }
        for (body, n, field) in [
            (&br#"{}"#[..], 1, "elevation"),
            (br#"{"elevation":1400.0}"#, 1, "elevation"),
            (br#"{"elevation":[1400.0, null]}"#, 2, "elevation[1]"),
            (br#"{"elevation":["1400"]}"#, 1, "elevation[0]"),
        ] {
            match error(body, n) {
                ElevationError::Missing { field: f } => assert_eq!(f, field),
                other => panic!("{other:?}"),
            }
        }
        let count = error(br#"{"elevation":[1.0, 2.0]}"#, 3);
        assert!(matches!(
            count,
            ElevationError::Count {
                expected: 3,
                found: 2
            }
        ));
        assert_eq!(
            count.to_string(),
            "the Open-Meteo elevation answer holds 2 heights, not the 3 asked for"
        );
        assert!(matches!(
            error(br#"{"elevation":[]}"#, 1),
            ElevationError::Count {
                expected: 1,
                found: 0
            }
        ));
        for (body, n, index, height) in [
            (&br#"{"elevation":[0.0, -32768]}"#[..], 2, 1, -32_768.0),
            (br#"{"elevation":[-32767]}"#, 1, 0, -32_767.0),
            (br#"{"elevation":[-1000.5]}"#, 1, 0, -1_000.5),
            (br#"{"elevation":[9000.5]}"#, 1, 0, 9_000.5),
        ] {
            match error(body, n) {
                ElevationError::OutOfRange { index: i, height_m } => {
                    assert_eq!((i, height_m), (index, height));
                }
                other => panic!("{other:?}"),
            }
        }
    }
}
