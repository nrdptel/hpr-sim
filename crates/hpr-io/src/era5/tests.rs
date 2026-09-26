use std::f64::consts::{FRAC_PI_2, PI};

use hpr_atmos::wind::Wind;
use serde_json::Value;

use super::*;
use crate::netcdf::Variable;

/// The extracts `validation/oracles/netcdf/era5.py` cut from RocketPy's ERA5 files.
fn extract(id: &str) -> NetCdf {
    let bytes: &[u8] = match id {
        "bella-lui" => include_bytes!("../../../../validation/fixtures/weather/era5/bella-lui.nc"),
        "ndrt-2020" => include_bytes!("../../../../validation/fixtures/weather/era5/ndrt-2020.nc"),
        "ndrt-2020-cds" => {
            include_bytes!("../../../../validation/fixtures/weather/era5/ndrt-2020-cds.nc")
        }
        other => panic!("no extract {other}"),
    };
    NetCdf::parse(bytes).unwrap()
}

/// RocketPy's reading of the full files at each case's times.
fn rocketpy() -> Value {
    serde_json::from_str(include_str!(
        "../../../../validation/fixtures/weather/era5-rocketpy.json"
    ))
    .unwrap()
}

fn case<'a>(fixture: &'a Value, id: &str) -> &'a Value {
    fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
}

fn request(case: &Value, unix_s: f64) -> Era5Request {
    Era5Request {
        latitude_deg: case["latitude_deg"].as_f64().unwrap(),
        longitude_deg: case["longitude_deg"].as_f64().unwrap(),
        time: UtcTime::from_unix_seconds(unix_s).unwrap(),
    }
}

/// RocketPy's levels at one reading, as (pressure, geometric height, T, u, v, its Earth radius).
fn levels(reading: &Value) -> Vec<[f64; 6]> {
    let radius = reading["rocketpy"]["earth_radius_m"].as_f64().unwrap();
    reading["rocketpy"]["levels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let f = |k: &str| l[k].as_f64().unwrap();
            [
                f("pressure_pa"),
                f("height_m"),
                f("temperature_k"),
                f("wind_east_m_s"),
                f("wind_north_m_s"),
                radius,
            ]
        })
        .collect()
}

fn close(a: f64, b: f64, relative: f64, what: &str) {
    assert!(
        (a - b).abs() <= relative * b.abs().max(1.0),
        "{what}: {a} against {b}"
    );
}

/// WMO-No. 8 (2023), Vol. I, eqs. 12.15 and 12.16, written out here rather than taken from
/// `hpr_atmos`: the normal gravity `γ_s(φ)` and radius `R(φ)` at latitude `φ`.
fn wmo_gravity_and_radius(latitude_rad: f64) -> (f64, f64) {
    let s2 = latitude_rad.sin().powi(2);
    let gamma_s = 9.780_325 * (1.0 + 0.001_931_85 * s2) / (1.0 - 0.006_694_35 * s2).sqrt();
    (gamma_s, 6_378_137.0 / (1.006_803 - 0.006_706 * s2))
}

/// Geometric height of WMO geopotential height `z` at latitude `φ`: eq. 12.15,
/// `Z = (γ_s/γ₄₅) R h/(R + h)`, solved for `h`.
fn wmo_geometric(z: f64, latitude_rad: f64) -> f64 {
    let (gamma_s, radius) = wmo_gravity_and_radius(latitude_rad);
    let reduced = z * ERA5_GRAVITY_M_S2 / gamma_s;
    radius * reduced / (radius - reduced)
}

fn latitude_rad(case: &Value) -> f64 {
    case["latitude_deg"].as_f64().unwrap().to_radians()
}

#[test]
fn on_the_hour_it_reads_the_levels_rocketpy_reads() {
    // RocketPy's values are the same arithmetic on the same stored numbers, so the two agree to
    // rounding; 1e-12 leaves room for the order of the operations.
    let fixture = rocketpy();
    let mut readings = 0;
    let mut levels_checked = 0;
    for id in ["bella-lui", "ndrt-2020", "ndrt-2020-cds"] {
        let case = case(&fixture, id);
        let file = extract(id);
        for reading in case["readings"].as_array().unwrap() {
            readings += 1;
            let unix_s = reading["unix_s"].as_f64().unwrap();
            let profile = Era5Profile::read(&file, request(case, unix_s)).unwrap();
            assert_eq!(profile.times.len(), 1, "{id}: {unix_s} is on the hour");
            let expected = levels(reading);
            assert_eq!(profile.levels.len(), expected.len(), "{id}");
            // RocketPy sorts by height, hpr by pressure; both lowest first.
            for (ours, [p, h, t, u, v, radius]) in profile.levels.iter().zip(expected) {
                let what = format!("{id} at {unix_s} s, {p} Pa");
                assert_eq!(ours.pressure_pa, p, "{what}");
                close(ours.temperature_k, t, 1e-12, &what);
                close(ours.wind_east_m_s, u, 1e-12, &what);
                close(ours.wind_north_m_s, v, 1e-12, &what);
                // RocketPy's h = R Z/(R − Z), inverted for its Z.
                let z = radius * h / (radius + h);
                close(ours.geopotential_height_m, z, 1e-12, &what);
                // hpr's geometric height is WMO's from that Z.
                close(
                    ours.height_msl_m,
                    wmo_geometric(z, latitude_rad(case)),
                    1e-12,
                    &what,
                );
                levels_checked += 1;
            }
        }
    }
    assert_eq!(readings, 5);
    assert_eq!(levels_checked, 3 * 14 + 14 + 37);
}

#[test]
fn the_two_height_readings_differ_as_the_guide_says() {
    // Near the ground the two readings differ by g₀/γ_s(φ) − 1 of the height; the guide and the
    // module documentation quote it, and the gap at each file's top level used in the guide.
    let fixture = rocketpy();
    for (id, ratio, top_gap_m) in [
        ("bella-lui", -1.58e-4, "-0.69"),
        ("ndrt-2020", 3.43e-4, "1.45"),
    ] {
        let case = case(&fixture, id);
        let (gamma_s, _) = wmo_gravity_and_radius(latitude_rad(case));
        let exact = ERA5_GRAVITY_M_S2 / gamma_s - 1.0;
        assert_eq!(format!("{exact:.2e}"), format!("{ratio:.2e}"), "{id}");
        let reading = &case["readings"][0];
        let unix_s = reading["unix_s"].as_f64().unwrap();
        let profile = Era5Profile::read(&extract(id), request(case, unix_s)).unwrap();
        let (ours, theirs) = (profile.levels.last().unwrap(), levels(reading)[13]);
        assert_eq!(
            format!("{:.2}", ours.height_msl_m - theirs[1]),
            top_gap_m,
            "{id}"
        );
        // At the lowest level the gap is that fraction of the height, to the radius term.
        let (low, low_theirs) = (&profile.levels[0], levels(reading)[0]);
        let gap = (low.height_msl_m - low_theirs[1]) / low_theirs[1];
        assert!(
            (gap - exact).abs() < 1e-6 * exact.abs().max(1e-4) + 1e-7,
            "{id}: {gap}"
        );
    }
}

#[test]
fn each_height_reading_errs_as_the_module_documentation_says() {
    // A model ground at true height `h_s` with geopotential `g₀ h_s`, and gravity above it falling
    // off as WMO's eq. 12.15 takes it: a level at true height `h` has
    // `Z = h_s + (γ_s/g₀) R (h/(R + h) − h_s/(R + h_s))`. hpr reads `Z` by WMO, ECMWF by
    // `R Z/(R − Z)`; the module documentation and the guide quote the errors and the crossing.
    let errors = |latitude_deg: f64, ground_m: f64, above_m: f64| {
        let latitude = latitude_deg.to_radians();
        let (gamma_s, radius) = wmo_gravity_and_radius(latitude);
        let wmo = |h: f64| gamma_s / ERA5_GRAVITY_M_S2 * radius * h / (radius + h);
        let h = ground_m + above_m;
        let z = ground_m + wmo(h) - wmo(ground_m);
        let hpr = wmo_geometric(z, latitude) - h;
        let ecmwf = radius * z / (radius - z) - h;
        (hpr, ecmwf)
    };
    let two = |x: f64| format!("{x:.2}");
    // A model ground at 407 m at 47.2° N: hpr's error is a constant few centimetres.
    for above_m in [0.0, 3000.0] {
        assert_eq!(two(errors(47.2, 407.0, above_m).0), "-0.04");
    }
    assert_eq!(two(errors(47.2, 407.0, 0.0).1), "0.03");
    assert_eq!(two(errors(47.2, 407.0, 3000.0).1), "0.50");
    // A pad 1400 m up at 33° N: hpr's is constant, ECMWF's grows with the height above the ground.
    for above_m in [0.0, 3000.0] {
        assert_eq!(two(errors(33.0, 1400.0, above_m).0), "1.88");
    }
    assert_eq!(two(errors(33.0, 1400.0, 0.0).1), "0.31");
    assert_eq!(two(errors(33.0, 1400.0, 3000.0).1), "-3.05");
    // ECMWF's is the smaller up to about 1.95 km above that ground, hpr's beyond.
    let (mut low, mut high) = (0.0, 3000.0);
    for _ in 0..60 {
        let mid = 0.5 * (low + high);
        let (hpr, ecmwf) = errors(33.0, 1400.0, mid);
        if ecmwf.abs() < hpr.abs() {
            low = mid;
        } else {
            high = mid;
        }
    }
    assert_eq!(format!("{:.2}", low / 1000.0), "1.95");
}

#[test]
fn between_hours_it_weights_the_two_hours_in_time() {
    let fixture = rocketpy();
    let case = case(&fixture, "bella-lui");
    let file = extract("bella-lui");
    let at = |hour: f64| {
        let unix = UtcTime::from_civil(2020, 2, 22, 0, 0, 0.0)
            .unwrap()
            .unix_seconds()
            + hour * 3600.0;
        Era5Profile::read(&file, request(case, unix)).unwrap()
    };
    let reading = |hour: i64| {
        let readings = case["readings"].as_array().unwrap();
        levels(
            readings
                .iter()
                .find(|r| r["time"][3].as_i64() == Some(hour))
                .unwrap(),
        )
    };
    let (r12, r13, r18) = (reading(12), reading(13), reading(18));
    // 12:30 is halfway between the file's 12 h and 13 h; 14:00 a fifth of the way to 18 h.
    for (hour, a, b, weight) in [(12.5, &r12, &r13, 0.5), (14.0, &r13, &r18, 0.2)] {
        let profile = at(hour);
        assert_eq!(profile.times.len(), 2);
        assert_eq!(profile.times[1].1, weight);
        close(
            profile.times[0].1 + profile.times[1].1,
            1.0,
            1e-15,
            "weights",
        );
        for (k, ours) in profile.levels.iter().enumerate() {
            let what = format!("{hour} h, {} Pa", ours.pressure_pa);
            for (value, column) in [
                (ours.temperature_k, 2),
                (ours.wind_east_m_s, 3),
                (ours.wind_north_m_s, 4),
            ] {
                let expected = (1.0 - weight) * a[k][column] + weight * b[k][column];
                close(value, expected, 1e-12, &what);
            }
            // Geopotential too, from RocketPy's heights: Z = R h/(R + h).
            let z = |r: &[f64; 6]| r[5] * r[1] / (r[5] + r[1]);
            let expected = (1.0 - weight) * z(&a[k]) + weight * z(&b[k]);
            close(ours.geopotential_height_m, expected, 1e-12, &what);
        }
    }
}

#[test]
fn the_current_data_store_file_converted_as_the_guide_says_reads_like_the_older_file() {
    // The same ERA5 analysis, downloaded in 2021 as packed shorts and in 2024 as netCDF-4 floats
    // (then converted). Both are quantized: the older to half its packing step (0.31 m² s⁻² in
    // z, 0.19 mK in t, 0.16 and 0.14 mm/s in u and v), the newer by the GRIB packing it was made
    // from, whose step the file does not keep. The bounds are a sanity check on the conversion,
    // not a precision: a flipped axis, a wrong level or unit would miss them by kelvins and
    // metres. The largest gaps, which the guide quotes, are pinned as it rounds them.
    let fixture = rocketpy();
    let case = case(&fixture, "ndrt-2020");
    let unix_s = case["readings"][0]["unix_s"].as_f64().unwrap();
    let old_file = extract("ndrt-2020");
    let old = Era5Profile::read(&old_file, request(case, unix_s)).unwrap();
    let new = Era5Profile::read(&extract("ndrt-2020-cds"), request(case, unix_s)).unwrap();
    assert_eq!(new.levels.len(), 37);
    let mut largest = [0.0_f64; 4];
    for ours in &old.levels {
        let theirs = new
            .levels
            .iter()
            .find(|l| l.pressure_pa == ours.pressure_pa)
            .unwrap();
        let what = format!("{} Pa", ours.pressure_pa);
        let dz = (ours.geopotential_height_m - theirs.geopotential_height_m) * ERA5_GRAVITY_M_S2;
        assert!(dz.abs() < 1.0, "{what}: z {dz}");
        let dt = ours.temperature_k - theirs.temperature_k;
        assert!(dt.abs() < 1e-3, "{what}: t {dt}");
        let du = ours.wind_east_m_s - theirs.wind_east_m_s;
        assert!(du.abs() < 1e-3, "{what}: u {du}");
        let dv = ours.wind_north_m_s - theirs.wind_north_m_s;
        assert!(dv.abs() < 1e-3, "{what}: v {dv}");
        for (slot, gap) in largest.iter_mut().zip([dz, dt, du, dv]) {
            *slot = slot.max(gap.abs());
        }
    }
    let [dz, dt, du, dv] = largest;
    assert_eq!(format!("{dz:.2}"), "0.23", "z, m² s⁻²");
    assert_eq!(format!("{:.2}", dt * 1e3), "0.35", "t, mK");
    assert_eq!(format!("{:.2}", du.max(dv) * 1e3), "0.11", "wind, mm/s");
}

#[test]
fn the_sounding_passes_through_every_level() {
    let fixture = rocketpy();
    let case = case(&fixture, "bella-lui");
    let unix_s = case["readings"][0]["unix_s"].as_f64().unwrap();
    let profile = Era5Profile::read(&extract("bella-lui"), request(case, unix_s)).unwrap();
    assert!(profile.unread.is_empty());
    let sounding = profile.sounding(WindInterpolation::Components).unwrap();
    for level in &profile.levels {
        let sample = sounding.sample(level.height_msl_m).unwrap();
        let what = format!("{} Pa", level.pressure_pa);
        close(sample.air.temperature_k, level.temperature_k, 1e-12, &what);
        close(sample.air.pressure_pa, level.pressure_pa, 1e-12, &what);
        let wind = sounding.wind().unwrap().wind(level.height_msl_m).unwrap();
        close(wind.velocity_enu_m_s.x, level.wind_east_m_s, 1e-12, &what);
        close(wind.velocity_enu_m_s.y, level.wind_north_m_s, 1e-12, &what);
    }
}

#[test]
fn a_site_on_a_grid_point_takes_that_point_alone() {
    let file = extract("bella-lui");
    let time = UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0).unwrap();
    let on = Era5Profile::read(
        &file,
        Era5Request {
            latitude_deg: 47.25,
            longitude_deg: 9.0,
            time,
        },
    )
    .unwrap();
    let t = file.variable("t").unwrap();
    let lats = axis(file.variable("latitude").unwrap()).unwrap();
    let lons = axis(file.variable("longitude").unwrap()).unwrap();
    let i = lats.iter().position(|&x| x == 47.25).unwrap() as u64;
    let j = lons.iter().position(|&x| x == 9.0).unwrap() as u64;
    let levels = axis(file.variable("level").unwrap()).unwrap();
    for level in &on.levels {
        let k = levels
            .iter()
            .position(|&p| p * 100.0 == level.pressure_pa)
            .unwrap() as u64;
        let stored = t.unpacked(&[3, k, i, j]).unwrap().unwrap();
        assert_eq!(level.temperature_k, stored);
    }
}

#[test]
fn a_longitude_a_turn_away_reads_the_same() {
    let fixture = rocketpy();
    for id in ["bella-lui", "ndrt-2020"] {
        let case = case(&fixture, id);
        let file = extract(id);
        let unix_s = case["readings"][0]["unix_s"].as_f64().unwrap();
        let base = request(case, unix_s);
        let turned = Era5Request {
            longitude_deg: if base.longitude_deg < 0.0 {
                base.longitude_deg + 360.0
            } else {
                base.longitude_deg - 360.0
            },
            ..base
        };
        let a = Era5Profile::read(&file, base).unwrap();
        let b = Era5Profile::read(&file, turned).unwrap();
        // The same four points and weights, up to the rounding of `y − y₁` a turn away.
        for (a, b) in a.levels.iter().zip(&b.levels) {
            close(a.temperature_k, b.temperature_k, 1e-13, id);
            close(a.wind_east_m_s, b.wind_east_m_s, 1e-13, id);
            close(a.wind_north_m_s, b.wind_north_m_s, 1e-13, id);
            close(a.height_msl_m, b.height_msl_m, 1e-13, id);
        }
    }
}

#[test]
fn a_site_or_time_outside_the_file_is_refused() {
    let file = extract("bella-lui");
    let time = UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0).unwrap();
    let site = Era5Request {
        latitude_deg: 47.213476,
        longitude_deg: 9.003336,
        time,
    };
    let late = Era5Request {
        time: UtcTime::from_civil(2020, 2, 22, 18, 0, 1.0).unwrap(),
        ..site
    };
    assert!(matches!(
        Era5Profile::read(&file, late),
        Err(Era5Error::OutsideTimes { .. })
    ));
    let north = Era5Request {
        latitude_deg: 48.0,
        ..site
    };
    assert!(matches!(
        Era5Profile::read(&file, north),
        Err(Era5Error::OutsideGrid {
            axis: "latitude",
            ..
        })
    ));
    let east = Era5Request {
        longitude_deg: 20.0,
        ..site
    };
    assert!(matches!(
        Era5Profile::read(&file, east),
        Err(Era5Error::OutsideGrid {
            axis: "longitude",
            ..
        })
    ));
    let bad = Era5Request {
        latitude_deg: f64::NAN,
        ..site
    };
    assert!(matches!(
        Era5Profile::read(&file, bad),
        Err(Era5Error::Domain {
            what: "the latitude (deg)",
            ..
        })
    ));
}

#[test]
fn a_file_without_the_variables_or_units_it_needs_is_refused() {
    let time = UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0).unwrap();
    let site = Era5Request {
        latitude_deg: 47.213476,
        longitude_deg: 9.003336,
        time,
    };
    let mut no_z = extract("bella-lui");
    no_z.variables.retain(|v| v.name != "z");
    assert_eq!(
        Era5Profile::read(&no_z, site).unwrap_err(),
        Era5Error::MissingVariable { name: "z".into() }
    );

    let mut celsius = extract("bella-lui");
    for variable in &mut celsius.variables {
        if variable.name == "t" {
            for attribute in &mut variable.attributes {
                if attribute.name == "units" {
                    attribute.values = crate::netcdf::Values::Char(b"degC".to_vec());
                }
            }
        }
    }
    assert!(matches!(
        Era5Profile::read(&celsius, site),
        Err(Era5Error::Units { ref variable, ref units, .. }) if variable == "t" && units == "degC"
    ));

    let mut missing = extract("bella-lui");
    for variable in &mut missing.variables {
        if variable.name == "u"
            && let crate::netcdf::Values::Short(values) = &mut variable.values
        {
            values.fill(-32767);
        }
    }
    assert!(matches!(
        Era5Profile::read(&missing, site),
        Err(Era5Error::MissingValue { ref variable, .. }) if variable == "u"
    ));
}

#[test]
fn coordinates_must_lie_along_their_own_dimension_and_be_present() {
    let time = UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0).unwrap();
    let site = Era5Request {
        latitude_deg: 47.213476,
        longitude_deg: 9.003336,
        time,
    };
    let edit = |change: &dyn Fn(&mut Variable)| {
        let mut file = extract("bella-lui");
        for variable in &mut file.variables {
            if variable.name == "latitude" {
                change(variable);
            }
        }
        Era5Profile::read(&file, site)
    };
    assert!(matches!(
        edit(&|v| v.dimensions = vec!["longitude".into()]),
        Err(Era5Error::Dimensions { ref variable, .. }) if variable == "latitude"
    ));
    assert!(matches!(
        edit(&|v| if let crate::netcdf::Values::Float(values) = &mut v.values {
            values[1] = f32::NAN;
        }),
        Err(Era5Error::MissingCoordinate { ref axis, index: 1 }) if axis == "latitude"
    ));
    assert!(matches!(
        edit(&|v| v.values = crate::netcdf::Values::Float(vec![])),
        Err(Era5Error::EmptyAxis { ref axis }) if axis == "latitude"
    ));
}

#[test]
fn a_time_serializes_as_its_seconds_and_refuses_what_is_not_finite() {
    let time = UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0).unwrap();
    let text = serde_json::to_string(&time).unwrap();
    assert_eq!(text, "1582376400.0");
    assert_eq!(serde_json::from_str::<UtcTime>(&text).unwrap(), time);
    assert!(UtcTime::try_from(f64::NAN).is_err());
}

#[test]
fn time_units_read_as_cf_writes_them() {
    let hours = time_units("hours since 1900-01-01 00:00:00.0", Some("gregorian")).unwrap();
    assert_eq!(hours, (3600.0, -2_208_988_800.0));
    let seconds = time_units("seconds since 1970-01-01", Some("proleptic_gregorian")).unwrap();
    assert_eq!(seconds, (1.0, 0.0));
    let days = time_units("days since 2000-01-01T12:00:00Z", None).unwrap();
    assert_eq!(days, (86_400.0, 946_728_000.0));
    let minutes = time_units("minutes since 2020-02-22 13:30 UTC", None).unwrap();
    assert_eq!(minutes, (60.0, 1_582_378_200.0));
    for (units, calendar) in [
        ("hours since 1500-01-01", None),
        ("hours since 2000-01-01", Some("noleap")),
        ("furlongs since 2000-01-01", None),
        ("hours after 2000-01-01", None),
        ("hours since 2000-01-01 00:00 +05:00", None),
        ("hours since 2000-13-01", None),
        ("hours since yesterday", None),
    ] {
        assert!(
            matches!(time_units(units, calendar), Err(Era5Error::Time { .. })),
            "{units}"
        );
    }
    // The proleptic calendar reads a date before 1582 as Gregorian.
    assert!(time_units("days since 1500-01-01", Some("proleptic_gregorian")).is_ok());
}

#[test]
fn civil_dates_count_seconds_as_posix_does() {
    let at = |y, mo, d, h, mi, s| {
        UtcTime::from_civil(y, mo, d, h, mi, s)
            .unwrap()
            .unix_seconds()
    };
    assert_eq!(at(1970, 1, 1, 0, 0, 0.0), 0.0);
    assert_eq!(at(2020, 2, 22, 13, 0, 0.0), 1_582_376_400.0);
    assert_eq!(at(2000, 2, 29, 23, 59, 59.5), 951_868_799.5);
    assert_eq!(at(1900, 1, 1, 0, 0, 0.0), -2_208_988_800.0);
    for (y, mo, d, h, mi, s) in [
        (2001, 2, 29, 0, 0, 0.0),
        (1900, 2, 29, 0, 0, 0.0),
        (2020, 13, 1, 0, 0, 0.0),
        (2020, 4, 31, 0, 0, 0.0),
        (2020, 1, 1, 24, 0, 0.0),
        (2020, 1, 1, 0, 60, 0.0),
        (2020, 1, 1, 0, 0, 60.0),
    ] {
        assert!(
            UtcTime::from_civil(y, mo, d, h, mi, s).is_err(),
            "{y}-{mo}-{d}"
        );
    }
    assert!(UtcTime::from_unix_seconds(f64::INFINITY).is_err());
}

#[test]
fn directions_are_where_the_wind_blows_from() {
    assert_eq!(direction_from_rad(0.0, -1.0), 0.0);
    // Positive zero, so it never prints as "-0".
    assert!(direction_from_rad(0.0, -1.0).is_sign_positive());
    close(
        direction_from_rad(-1.0, 0.0),
        FRAC_PI_2,
        1e-15,
        "from the east",
    );
    close(direction_from_rad(0.0, 1.0), PI, 1e-15, "from the south");
    close(
        direction_from_rad(1.0, 0.0),
        3.0 * FRAC_PI_2,
        1e-15,
        "from the west",
    );
    assert_eq!(direction_from_rad(0.0, 0.0), 0.0);
    assert_eq!(direction_from_rad(-0.0, -0.0), 0.0);
}

/// The weather records move only when their scripts run (Loft lesson L76), and the scripts they
/// name are the committed ones. `.gitattributes` keeps the scripts LF on every platform.
#[test]
fn the_weather_records_are_their_scripts_output() {
    use sha2::Digest as _;
    let hex = |bytes: &[u8]| -> String {
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let netcdf: Value = serde_json::from_str(include_str!(
        "../../../../validation/fixtures/weather/netcdf-reads.json"
    ))
    .unwrap();
    for (record, script) in [
        (
            &netcdf,
            include_bytes!("../../../../validation/oracles/netcdf/write_cases.py").as_slice(),
        ),
        (
            &rocketpy(),
            include_bytes!("../../../../validation/oracles/netcdf/era5.py").as_slice(),
        ),
    ] {
        assert_eq!(
            record["inputs_sha256"]["script"],
            hex(script).as_str(),
            "{} changed since its record was written: rerun it",
            record["generator"]
        );
    }
}
