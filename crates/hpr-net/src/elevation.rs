//! A launch site's ground elevation from Open-Meteo's elevation API.
//!
//! [Open-Meteo's elevation API](https://open-meteo.com/en/docs/elevation-api)
//! (`api.open-meteo.com/v1/elevation`) answers the ground's height at up to [`MAX_PLACES`] places
//! in one request, as `{"elevation":[1400.0]}`, one number per place in the order asked. Its data is
//! the Copernicus DEM GLO-90 (2021 release), a digital elevation model with a 3 arc-second grid,
//! about 90 m. Its heights are orthometric, above the EGM2008 geoid (mean sea level), not
//! above the WGS 84 ellipsoid: a site's ellipsoidal height is `h = H + N`, with `N` the geoid
//! undulation there, which hpr has no model for (`docs/physics/geodesy.md`).
//!
//! An [`ElevationRequest`] names the places; [`parse`] reads the answer, refusing one with the
//! wrong number of heights, a height that is not a number, or a height outside
//! [`HEIGHT_RANGE_M`]. [`fetch`] asks a [`Client`] for the URL, so the answer comes from the cache
//! when it can, and offline from the cache only; an answer that doesn't parse is never cached.
//! The ground doesn't move, so a copy stays fresh for [`TTL_S`], a year. Show [`ATTRIBUTION`] (it
//! is on every [`Fetched`]) wherever the height is shown.
//!
//! **How far to trust it:** a height is the answer's number, unchanged (the tests). Open-Meteo
//! answers in whole metres: every height in 100 random places asked on 2026-10-01 was whole. How
//! close that is to the ground under a launch rail depends on the DEM, whose cells are about 90 m
//! across; nothing here measures that. The [guide page][guide] says more.
//!
//! ```
//! use hpr_net::elevation;
//!
//! // An answer recorded for Spaceport America's launch area, 32.99° N, 106.97° W.
//! let body = include_bytes!("../tests/fixtures/replay/open-meteo-elevation.json");
//! assert_eq!(elevation::parse(body, 1)?, [1400.0]);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
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

/// The heights [`parse`] accepts, m above mean sea level. The lowest land is the Dead Sea's shore,
/// about −430 m, and the highest Everest's summit, 8,849 m; the margins leave room for the DEM's
/// own errors. A height outside them is a broken answer, such as a 16-bit no-data value (−32,768).
pub const HEIGHT_RANGE_M: std::ops::RangeInclusive<f64> = -1_000.0..=9_000.0;

/// The credit Open-Meteo asks for: itself, and the Copernicus programme whose DEM it serves, in
/// the DEM licence's words.
pub const ATTRIBUTION: &str = "Elevation data by Open-Meteo.com, from the Copernicus DEM GLO-90: \
                               © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 \
                               provided under COPERNICUS by the European Union and ESA; all rights \
                               reserved";

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
    /// comma-separated in the places' order.
    ///
    /// # Errors
    /// [`ElevationError::Request`] when there are no places or more than [`MAX_PLACES`], a place
    /// is outside the range its doc gives, or the endpoint is empty or holds a `?` or `#`.
    pub fn url(&self) -> Result<String, ElevationError> {
        let refuse = |what: &'static str, value: String| ElevationError::Request { what, value };
        if !(1..=MAX_PLACES).contains(&self.places.len()) {
            return Err(refuse("number of places", self.places.len().to_string()));
        }
        for place in &self.places {
            if !(-90.0..=90.0).contains(&place.latitude_deg) {
                return Err(refuse("latitude (deg)", place.latitude_deg.to_string()));
            }
            if !(-180.0..=180.0).contains(&place.longitude_deg) {
                return Err(refuse("longitude (deg)", place.longitude_deg.to_string()));
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
            let values: Vec<String> = self.places.iter().map(|p| value(p).to_string()).collect();
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

/// Reads an answer of the elevation API: the ground's height at each of `places` places, m above
/// mean sea level (the EGM2008 geoid), in the order asked.
///
/// # Errors
/// [`ElevationError::Json`] when the body is not JSON; [`ElevationError::Server`] for Open-Meteo's
/// error answer; [`ElevationError::Missing`] when `elevation` is not an array of numbers;
/// [`ElevationError::Count`] when it holds another number of heights than `places`;
/// [`ElevationError::OutOfRange`] for a height outside [`HEIGHT_RANGE_M`].
pub fn parse(body: &[u8], places: usize) -> Result<Vec<f64>, ElevationError> {
    let json: Value =
        serde_json::from_slice(body).map_err(|e| ElevationError::Json(e.to_string()))?;
    if json.get("error").and_then(Value::as_bool) == Some(true) {
        let reason = json
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or("none given");
        return Err(ElevationError::Server {
            reason: reason.to_owned(),
        });
    }
    let missing = |field: String| ElevationError::Missing { field };
    let heights = json
        .get("elevation")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("elevation".to_owned()))?;
    if heights.len() != places {
        return Err(ElevationError::Count {
            expected: places,
            found: heights.len(),
        });
    }
    heights
        .iter()
        .enumerate()
        .map(|(index, height)| {
            let height_m = height
                .as_f64()
                .ok_or_else(|| missing(format!("elevation[{index}]")))?;
            if HEIGHT_RANGE_M.contains(&height_m) {
                Ok(height_m)
            } else {
                Err(ElevationError::OutOfRange { index, height_m })
            }
        })
        .collect()
}

/// Fetches `request` through `client` and reads each place's height, m above mean sea level.
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
) -> Result<(Vec<f64>, Fetched), ElevationError> {
    let url = request.url()?;
    let places = request.places.len();
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
        /// Its value.
        value: String,
    },
    /// The fetch failed.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The body is not JSON.
    #[error("the Open-Meteo elevation answer is not JSON: {0}")]
    Json(String),
    /// The body is Open-Meteo's error answer. Open-Meteo sends it with HTTP status 400, which
    /// `Http` reports as [`NetError::Transport`] without the body; [`fetch`] reports any answer
    /// that doesn't parse as [`NetError::Refused`], with this error's message.
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
    #[error("the Open-Meteo elevation answer holds {found} heights for {expected} places")]
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

    fn request(places: &[(f64, f64)]) -> ElevationRequest {
        ElevationRequest::new(
            places
                .iter()
                .map(|&(lat, lon)| Place::new(lat, lon))
                .collect(),
        )
    }

    /// The field a request refusal names.
    fn refused(request: &ElevationRequest) -> (&'static str, String) {
        match request.url() {
            Err(ElevationError::Request { what, value }) => (what, value),
            other => panic!("not a request refusal: {other:?}"),
        }
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
            ("latitude (deg)", "90.5".to_owned())
        );
        assert_eq!(
            refused(&request(&[(f64::NAN, 0.0)])),
            ("latitude (deg)", "NaN".to_owned())
        );
        assert_eq!(
            refused(&request(&[(-90.0, -180.5)])),
            ("longitude (deg)", "-180.5".to_owned())
        );
        assert_eq!(
            refused(&request(&[(0.0, f64::NAN)])),
            ("longitude (deg)", "NaN".to_owned())
        );
        assert!(request(&[(-90.0, 180.0), (90.0, -180.0)]).url().is_ok());
        for endpoint in ["", "https://x.test/v1/elevation?a=1", "https://x.test/#e"] {
            let mut r = request(&[(0.0, 0.0)]);
            r.endpoint = Some(endpoint.to_owned());
            assert_eq!(refused(&r), ("endpoint", endpoint.to_owned()));
        }
    }

    #[test]
    fn the_source_carries_the_copernicus_credit_and_a_year() {
        let source = request(&[(0.0, 0.0)]).source();
        assert_eq!(source.ttl_s, 31_536_000);
        assert!(
            source
                .attribution
                .starts_with("Elevation data by Open-Meteo.com")
        );
        assert!(source.attribution.contains("Copernicus DEM GLO-90"));
        assert!(source.attribution.ends_with("all rights reserved"));
        assert!(!source.attribution.contains("  "), "{}", source.attribution);
    }

    #[test]
    fn parse_reads_every_height_in_order() {
        assert_eq!(
            parse(br#"{"elevation":[1400.0, -427.0, 0.0]}"#, 3).unwrap(),
            [1400.0, -427.0, 0.0]
        );
        // Another field beside `elevation` is ignored, as Open-Meteo may add some.
        assert_eq!(
            parse(br#"{"elevation":[12.5],"generationtime_ms":0.1}"#, 1).unwrap(),
            [12.5]
        );
        let edges = parse(br#"{"elevation":[-1000, 9000]}"#, 2).unwrap();
        assert_eq!(edges, [-1_000.0, 9_000.0]);
    }

    #[test]
    fn parse_refuses_a_broken_answer() {
        let error = |body: &[u8], places| parse(body, places).unwrap_err();
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
            ElevationError::Server { reason } => assert_eq!(reason, "none given"),
            other => panic!("{other:?}"),
        }
        for (body, field) in [
            (&br#"{}"#[..], "elevation"),
            (br#"{"elevation":1400.0}"#, "elevation"),
            (br#"{"elevation":[1400.0, null]}"#, "elevation[1]"),
            (br#"{"elevation":["1400"]}"#, "elevation[0]"),
        ] {
            let places = if field == "elevation[1]" { 2 } else { 1 };
            match error(body, places) {
                ElevationError::Missing { field: f } => assert_eq!(f, field),
                other => panic!("{other:?}"),
            }
        }
        assert!(matches!(
            error(br#"{"elevation":[1.0, 2.0]}"#, 3),
            ElevationError::Count {
                expected: 3,
                found: 2
            }
        ));
        assert!(matches!(
            error(br#"{"elevation":[]}"#, 1),
            ElevationError::Count {
                expected: 1,
                found: 0
            }
        ));
        for (body, index, height) in [
            (&br#"{"elevation":[0.0, -32768]}"#[..], 1, -32_768.0),
            (br#"{"elevation":[-1000.5]}"#, 0, -1_000.5),
            (br#"{"elevation":[9000.5]}"#, 0, 9_000.5),
        ] {
            match error(body, if index == 1 { 2 } else { 1 }) {
                ElevationError::OutOfRange { index: i, height_m } => {
                    assert_eq!((i, height_m), (index, height));
                }
                other => panic!("{other:?}"),
            }
        }
    }
}
