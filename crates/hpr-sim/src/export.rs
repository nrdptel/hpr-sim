//! Writing a flight out: a [`Recorder`]'s rows as CSV or JSON, and the centre of mass's path with
//! the landings as GeoJSON or KML for a map.
//!
//! Every function builds text and returns it; none writes a file (the core crates do no I/O).
//! Numbers are written in Rust's shortest round-trip form, so each parses back to the exact `f64`
//! that was recorded. A value that isn't finite is refused rather than written as text a reader
//! would misread (`NaN`, or JSON's `null`).
//!
//! The two map formats put heights on different datums, as their specifications require:
//!
//! - **GeoJSON** (RFC 7946, section 4): longitude, latitude and height in metres above the WGS 84
//!   ellipsoid.
//! - **KML** (OGC 07-147r2, `altitudeMode` `absolute`): longitude, latitude and height above mean
//!   sea level (KML's geoid is EGM96), which is the ellipsoidal height less the site's geoid
//!   undulation ([`Environment::geoid_undulation_m`]). The undulation is taken as constant over the
//!   flight; the geoid's slope, about 5 cm/km and up to some 30 cm/km in mountains, moves it by
//!   centimetres to decimetres over a rocket's few kilometres.
//!
//! Neither cuts a path that crosses the antimeridian (±180° longitude), which RFC 7946 section 3.1.9
//! asks for; a flight there draws a line the long way round the globe.

use hpr_core::DVec3;
use serde_json::{Value, json};

use crate::environment::Environment;
use crate::error::SimError;
use crate::metrics::{FlightSummary, Landing};
use crate::recorder::Recorder;

/// One point of the centre of mass's path, placed on the Earth.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackPoint {
    /// Time since launch, s.
    pub time_s: f64,
    /// Geodetic latitude, degrees north (WGS 84).
    pub latitude_deg: f64,
    /// Longitude, degrees east.
    pub longitude_deg: f64,
    /// Height above the WGS 84 ellipsoid, m.
    pub height_above_ellipsoid_m: f64,
    /// Height above mean sea level, m: the ellipsoidal height less the site's geoid undulation.
    pub height_above_sea_level_m: f64,
}

fn finite(what: &'static str, value: f64) -> Result<f64, SimError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(SimError::Domain { what, value })
    }
}

/// The recorded rows as CSV (RFC 4180): a header of the column names, which carry their units,
/// then one line per row, lines ending in CRLF.
///
/// # Errors
///
/// [`SimError::Domain`] for a value that isn't finite.
pub fn csv(recorder: &Recorder) -> Result<String, SimError> {
    let mut out = recorder.columns().join(",");
    out.push_str("\r\n");
    for row in recorder.rows() {
        let mut cells = Vec::with_capacity(row.len());
        for &value in row {
            cells.push(format!("{:?}", finite("recorded value", value)?));
        }
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    Ok(out)
}

/// The recorded rows as JSON: `{"columns": [names], "rows": [[values], ...]}`, each row in the
/// columns' order. A [`FlightSummary`] is serializable on its own (`serde_json::to_string`).
///
/// # Errors
///
/// [`SimError::Domain`] for a value that isn't finite.
pub fn json(recorder: &Recorder) -> Result<String, SimError> {
    for row in recorder.rows() {
        for &value in row {
            finite("recorded value", value)?;
        }
    }
    Ok(json!({"columns": recorder.columns(), "rows": recorder.rows()}).to_string())
}

/// The centre of mass's path from a recorder that kept [`crate::Channel::Time`] and
/// [`crate::Channel::CgPosition`], placed on `environment`'s ellipsoid.
///
/// # Errors
///
/// [`SimError::Unsupported`] if the recorder lacks either channel; [`SimError::Core`] if a
/// position has no geodetic coordinates.
pub fn track(recorder: &Recorder, environment: &Environment) -> Result<Vec<TrackPoint>, SimError> {
    let columns = recorder.columns();
    let index = |name: &str| columns.iter().position(|c| c == name);
    let (Some(t), Some(e), Some(n), Some(u)) = (
        index("time_s"),
        index("cg_east_m"),
        index("cg_north_m"),
        index("cg_up_m"),
    ) else {
        return Err(SimError::Unsupported {
            what: "a track from a recorder without the time and CG position channels",
        });
    };
    let frame = environment.earth.frame();
    recorder
        .rows()
        .iter()
        .map(|row| {
            let place = frame.geodetic_from_enu(DVec3::new(row[e], row[n], row[u]))?;
            Ok(TrackPoint {
                time_s: row[t],
                latitude_deg: place.latitude_rad.to_degrees(),
                longitude_deg: place.longitude_rad.to_degrees(),
                height_above_ellipsoid_m: place.height_m,
                height_above_sea_level_m: place.height_m - environment.geoid_undulation_m,
            })
        })
        .collect()
}

/// The flight's landing, then each separated body's, as `(label, landing)`.
fn landings(summary: &FlightSummary) -> Vec<(String, &Landing)> {
    let flight = summary.landing.iter().map(|l| ("landing".to_owned(), l));
    let bodies = summary.body_landings.iter().map(|l| {
        let label = match l.body {
            Some(body) => format!("body {body} landing"),
            None => "landing".to_owned(),
        };
        (label, l)
    });
    flight.chain(bodies).collect()
}

fn too_short(track: &[TrackPoint]) -> Result<(), SimError> {
    if track.len() < 2 {
        return Err(SimError::Unsupported {
            what: "a path of fewer than two points (a line needs two)",
        });
    }
    Ok(())
}

/// The path and the landings as a GeoJSON `FeatureCollection` (RFC 7946): a `LineString` of
/// `[longitude, latitude, height above the ellipsoid]` whose `properties.time_s` lists each
/// point's time, and a `Point` per landing of `summary` with its time, body, distance and descent
/// rate.
///
/// # Errors
///
/// [`SimError::Unsupported`] for a track of fewer than two points (a `LineString` needs two);
/// [`SimError::Domain`] for a coordinate that isn't finite.
pub fn geojson(track: &[TrackPoint], summary: &FlightSummary) -> Result<String, SimError> {
    too_short(track)?;
    let mut coordinates = Vec::with_capacity(track.len());
    let mut times = Vec::with_capacity(track.len());
    for p in track {
        coordinates.push(json!([
            finite("longitude", p.longitude_deg)?,
            finite("latitude", p.latitude_deg)?,
            finite("height", p.height_above_ellipsoid_m)?,
        ]));
        times.push(finite("time", p.time_s)?);
    }
    let mut features = vec![json!({
        "type": "Feature",
        "geometry": {"type": "LineString", "coordinates": coordinates},
        "properties": {"name": "flight path", "time_s": times},
    })];
    for (label, l) in landings(summary) {
        features.push(json!({
            "type": "Feature",
            "geometry": {"type": "Point", "coordinates": [
                finite("longitude", l.longitude_deg)?,
                finite("latitude", l.latitude_deg)?,
            ]},
            "properties": {
                "name": label,
                "body": l.body,
                "time_s": finite("time", l.time_s)?,
                "distance_m": finite("distance", l.distance_m)?,
                "descent_rate_m_s": finite("descent rate", l.descent_rate_m_s)?,
            },
        }));
    }
    Ok(json!({"type": "FeatureCollection", "features": Value::Array(features)}).to_string())
}

/// `text` with XML's five special characters escaped.
///
/// # Errors
///
/// [`SimError::Unsupported`] for a control character XML 1.0 forbids (all below U+0020 but tab,
/// line feed and carriage return; U+FFFE and U+FFFF).
fn escape(text: &str) -> Result<String, SimError> {
    let forbidden = |c: char| {
        (c < ' ' && !matches!(c, '\t' | '\n' | '\r')) || matches!(c, '\u{FFFE}' | '\u{FFFF}')
    };
    if text.chars().any(forbidden) {
        return Err(SimError::Unsupported {
            what: "a KML name with a control character XML 1.0 forbids",
        });
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    Ok(out)
}

/// The path and the landings as a KML 2.2 document (OGC 07-147r2) named `name`: a `LineString`
/// of `longitude,latitude,height above mean sea level` with `altitudeMode` `absolute`, and a
/// ground-clamped `Point` per landing of `summary`.
///
/// # Errors
///
/// [`SimError::Unsupported`] for a track of fewer than two points or a name with a control
/// character XML forbids; [`SimError::Domain`] for a coordinate that isn't finite.
pub fn kml(track: &[TrackPoint], summary: &FlightSummary, name: &str) -> Result<String, SimError> {
    too_short(track)?;
    let mut path = Vec::with_capacity(track.len());
    for p in track {
        path.push(format!(
            "{:?},{:?},{:?}",
            finite("longitude", p.longitude_deg)?,
            finite("latitude", p.latitude_deg)?,
            finite("height", p.height_above_sea_level_m)?,
        ));
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<kml xmlns=\"http://www.opengis.net/kml/2.2\">\n<Document>\n");
    out.push_str(&format!("<name>{}</name>\n", escape(name)?));
    out.push_str("<Placemark>\n<name>flight path</name>\n<LineString>\n");
    out.push_str("<altitudeMode>absolute</altitudeMode>\n<coordinates>\n");
    out.push_str(&path.join("\n"));
    out.push_str("\n</coordinates>\n</LineString>\n</Placemark>\n");
    for (label, l) in landings(summary) {
        out.push_str(&format!(
            "<Placemark>\n<name>{}</name>\n<description>t = {:?} s</description>\n<Point>\n\
             <coordinates>{:?},{:?}</coordinates>\n</Point>\n</Placemark>\n",
            escape(&label)?,
            finite("time", l.time_s)?,
            finite("longitude", l.longitude_deg)?,
            finite("latitude", l.latitude_deg)?,
        ));
    }
    out.push_str("</Document>\n</kml>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use hpr_atmos::ConstantWind;

    use super::*;
    use crate::flight::{FlightSettings, Simulation};
    use crate::metrics::FlightMetrics;
    use crate::rail::Rail;
    use crate::recorder::Channel;
    use crate::recovery::{Device, DeviceDrag, Trigger};
    use crate::testing::{design, windy_environment};

    /// Valetudo under a canopy from apogee, carried east by a 6 m/s west wind: its recording at
    /// 1 s, its path and its summary.
    fn flown() -> (Recorder, Vec<TrackPoint>, FlightSummary, Environment) {
        let wind = ConstantWind::new(6.0, 1.5 * std::f64::consts::PI).unwrap();
        let canopy = Device::new(
            "main",
            DeviceDrag::DragArea { cd_s_m2: 0.5 },
            Trigger::Apogee,
        );
        let sim = Simulation::new(
            &design("rocketpy-valetudo"),
            "example",
            windy_environment(wind).with_geoid_undulation_m(-25.0),
            Rail::vertical(3.0),
            FlightSettings {
                max_time_s: 600.0,
                ..FlightSettings::default()
            },
        )
        .unwrap()
        .with_recovery(vec![canopy])
        .unwrap();
        let recorder = Recorder::new(
            vec![Channel::Time, Channel::CgPosition, Channel::Mach],
            Some(1.0),
        )
        .unwrap();
        let mut watchers = (FlightMetrics::new(), recorder);
        let result = sim.run(&mut watchers).unwrap();
        let (metrics, recorder) = watchers;
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        let track = track(&recorder, sim.environment()).unwrap();
        (recorder, track, summary, sim.environment().clone())
    }

    #[test]
    fn csv_and_json_hold_every_recorded_value_exactly() {
        let (recorder, ..) = flown();
        assert!(recorder.rows().len() > 20);
        let text = csv(&recorder).unwrap();
        let mut lines = text.split("\r\n");
        let header: Vec<&str> = lines.next().unwrap().split(',').collect();
        assert_eq!(header, recorder.columns());
        let rows: Vec<Vec<f64>> = lines
            .filter(|l| !l.is_empty())
            .map(|l| l.split(',').map(|c| c.parse().unwrap()).collect())
            .collect();
        assert_eq!(rows, recorder.rows());
        assert!(text.ends_with("\r\n"));

        let value: Value = serde_json::from_str(&json(&recorder).unwrap()).unwrap();
        let columns: Vec<String> = serde_json::from_value(value["columns"].clone()).unwrap();
        let rows: Vec<Vec<f64>> = serde_json::from_value(value["rows"].clone()).unwrap();
        assert_eq!(columns, recorder.columns());
        assert_eq!(rows, recorder.rows());
    }

    #[test]
    fn a_value_that_is_not_finite_is_refused() {
        let point = TrackPoint {
            time_s: 0.0,
            latitude_deg: 32.0,
            longitude_deg: f64::NAN,
            height_above_ellipsoid_m: 0.0,
            height_above_sea_level_m: 0.0,
        };
        let (.., summary, _) = flown();
        let track = [
            point,
            TrackPoint {
                time_s: 1.0,
                ..point
            },
        ];
        for result in [geojson(&track, &summary), kml(&track, &summary, "x")] {
            assert!(matches!(
                result,
                Err(SimError::Domain { what: "longitude", value }) if value.is_nan()
            ));
        }
        for result in [
            geojson(&track[..1], &summary),
            kml(&track[..1], &summary, "x"),
        ] {
            assert!(matches!(
                result,
                Err(SimError::Unsupported { what }) if what.contains("fewer than two")
            ));
        }
        let fine = [
            TrackPoint {
                longitude_deg: -106.97,
                ..point
            },
            TrackPoint {
                time_s: 1.0,
                longitude_deg: -106.97,
                ..point
            },
        ];
        assert!(matches!(
            kml(&fine, &summary, "a\u{1}b"),
            Err(SimError::Unsupported { what }) if what.contains("control character")
        ));
        assert!(kml(&fine, &summary, "tab\tok").is_ok());
    }

    #[test]
    fn track_places_each_row_on_the_ellipsoid_and_the_geoid() {
        let (recorder, track, summary, environment) = flown();
        assert_eq!(track.len(), recorder.rows().len());
        let frame = environment.earth.frame();
        for (point, row) in track.iter().zip(recorder.rows()) {
            let place = frame
                .geodetic_from_enu(DVec3::new(row[1], row[2], row[3]))
                .unwrap();
            assert_eq!(point.time_s, row[0]);
            assert_eq!(point.latitude_deg, place.latitude_rad.to_degrees());
            assert_eq!(point.longitude_deg, place.longitude_rad.to_degrees());
            assert_eq!(point.height_above_ellipsoid_m, place.height_m);
            // h = H + N with N = −25 m: sea-level heights are 25 m above ellipsoidal ones.
            assert!((point.height_above_sea_level_m - place.height_m - 25.0).abs() < 1e-9);
        }
        // The wind carries it east of the site, and it lands where the summary says.
        let last = track.last().unwrap();
        let landing = summary.landing.unwrap();
        assert!(last.longitude_deg > track[0].longitude_deg);
        assert!(landing.east_m > 100.0);

        let without = Recorder::new(vec![Channel::Time], None).unwrap();
        assert!(matches!(
            super::track(&without, &environment),
            Err(SimError::Unsupported { .. })
        ));
    }

    /// The published GeoJSON `FeatureCollection` schema (geojson/schema, MIT, commit 268ba0a).
    fn geojson_validator() -> jsonschema::Validator {
        let schema: Value = serde_json::from_str(include_str!(
            "../tests/data/geojson-feature-collection.schema.json"
        ))
        .unwrap();
        jsonschema::draft7::new(&schema).unwrap()
    }

    #[test]
    fn geojson_passes_the_published_schema_in_longitude_latitude_order() {
        let (_, track, summary, _) = flown();
        let validator = geojson_validator();
        let value: Value = serde_json::from_str(&geojson(&track, &summary).unwrap()).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| e.to_string())
            .collect();
        assert!(errors.is_empty(), "{errors:?}");

        let features = value["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        let path = features[0]["geometry"]["coordinates"].as_array().unwrap();
        assert_eq!(path.len(), track.len());
        for (position, point) in path.iter().zip(&track) {
            let p: [f64; 3] = serde_json::from_value(position.clone()).unwrap();
            assert_eq!(
                p,
                [
                    point.longitude_deg,
                    point.latitude_deg,
                    point.height_above_ellipsoid_m
                ]
            );
        }
        let landing = summary.landing.unwrap();
        let spot: [f64; 2] =
            serde_json::from_value(features[1]["geometry"]["coordinates"].clone()).unwrap();
        assert_eq!(spot, [landing.longitude_deg, landing.latitude_deg]);

        // The validator does reject a broken document: a one-number position.
        let mut broken = value.clone();
        broken["features"][1]["geometry"]["coordinates"] = json!([landing.longitude_deg]);
        assert!(!validator.is_valid(&broken));
    }

    #[test]
    fn kml_parses_as_kml_2_2_with_every_position() {
        let (_, track, summary, _) = flown();
        let text = kml(&track, &summary, "Valetudo & <friends>").unwrap();
        let document = roxmltree::Document::parse(&text).unwrap();
        let root = document.root_element();
        assert_eq!(root.tag_name().name(), "kml");
        assert_eq!(
            root.tag_name().namespace(),
            Some("http://www.opengis.net/kml/2.2")
        );
        let named = |tag: &'static str| {
            let found: Vec<roxmltree::Node<'_, '_>> = document
                .descendants()
                .filter(|n| n.tag_name().name() == tag)
                .collect();
            found.into_iter()
        };
        let name = named("name").next().unwrap().text();
        assert_eq!(name, Some("Valetudo & <friends>"));
        assert_eq!(
            named("altitudeMode").next().unwrap().text(),
            Some("absolute")
        );
        let lists: Vec<&str> = named("coordinates").map(|n| n.text().unwrap()).collect();
        assert_eq!(lists.len(), 2);
        let path: Vec<Vec<f64>> = lists[0]
            .split_whitespace()
            .map(|t| t.split(',').map(|c| c.parse().unwrap()).collect())
            .collect();
        assert_eq!(path.len(), track.len());
        for (position, point) in path.iter().zip(&track) {
            assert_eq!(
                position,
                &[
                    point.longitude_deg,
                    point.latitude_deg,
                    point.height_above_sea_level_m
                ]
            );
        }
        let landing = summary.landing.unwrap();
        let spot: Vec<f64> = lists[1]
            .trim()
            .split(',')
            .map(|c| c.parse().unwrap())
            .collect();
        assert_eq!(spot, [landing.longitude_deg, landing.latitude_deg]);
    }
}
