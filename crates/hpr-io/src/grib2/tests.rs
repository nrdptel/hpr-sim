//! GRIB2 messages built byte by byte from WMO-No. 306's section layouts, and read back.
//!
//! The recorded GFS and RAP cuts, checked against ecCodes, are in `hpr-net`'s `tests/nomads.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use super::*;

/// A section: its length, its number, then `body`.
fn section(number: u8, body: &[u8]) -> Vec<u8> {
    let mut s = u32::try_from(body.len() + 5)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    s.push(number);
    s.extend_from_slice(body);
    s
}

/// A sign-and-magnitude 32-bit integer.
fn sm32(v: i64) -> [u8; 4] {
    let raw = u32::try_from(v.unsigned_abs()).unwrap() | if v < 0 { 0x8000_0000 } else { 0 };
    raw.to_be_bytes()
}

/// A sign-and-magnitude 16-bit integer.
fn sm16(v: i16) -> [u8; 2] {
    (v.unsigned_abs() | if v < 0 { 0x8000 } else { 0 }).to_be_bytes()
}

/// Section 1: NCEP, 2026-09-30 00:00 UTC, start of forecast.
fn identification() -> Vec<u8> {
    let mut b = vec![0, 7, 0, 0, 2, 1, 1];
    b.extend_from_slice(&2026_u16.to_be_bytes());
    b.extend_from_slice(&[9, 30, 0, 0, 0, 0, 1]);
    section(1, &b)
}

/// Section 3, template 3.0: `ni` by `nj` from (`la1`, `lo1`) µdeg, 0.25° steps, sphere code 6.
fn latlon_grid(ni: u32, nj: u32, la1: i64, lo1: i64, scanning: u8, flags: u8) -> Vec<u8> {
    let mut b = vec![0];
    b.extend_from_slice(&(ni * nj).to_be_bytes());
    b.extend_from_slice(&[0, 0, 0, 0]);
    b.push(6); // shape of the Earth
    b.extend_from_slice(&[0; 15]); // radius and axes, unused for code 6
    b.extend_from_slice(&ni.to_be_bytes());
    b.extend_from_slice(&nj.to_be_bytes());
    b.extend_from_slice(&[0; 4]); // basic angle
    b.extend_from_slice(&[0xFF; 4]); // subdivisions
    b.extend_from_slice(&sm32(la1));
    b.extend_from_slice(&sm32(lo1));
    b.push(flags);
    b.extend_from_slice(&sm32(la1 + 250_000 * i64::from(nj - 1)));
    b.extend_from_slice(&sm32((lo1 + 250_000 * i64::from(ni - 1)) % 360_000_000));
    b.extend_from_slice(&250_000_u32.to_be_bytes());
    b.extend_from_slice(&250_000_u32.to_be_bytes());
    b.push(scanning);
    section(3, &b)
}

/// Section 3, template 3.30: RAP's 13 km grid's parameters (tangent at 25° N, `LoV` 265°).
fn lambert_grid(ni: u32, nj: u32, latin1: i64, latin2: i64) -> Vec<u8> {
    let mut b = vec![0];
    b.extend_from_slice(&(ni * nj).to_be_bytes());
    b.extend_from_slice(&[0, 0, 0, 30]);
    b.push(6);
    b.extend_from_slice(&[0; 15]);
    b.extend_from_slice(&ni.to_be_bytes());
    b.extend_from_slice(&nj.to_be_bytes());
    b.extend_from_slice(&sm32(32_854_458));
    b.extend_from_slice(&sm32(252_775_632));
    b.push(0x38); // increments given, winds along the grid
    b.extend_from_slice(&sm32(latin1)); // LaD
    b.extend_from_slice(&sm32(265_000_000));
    b.extend_from_slice(&13_545_000_u32.to_be_bytes());
    b.extend_from_slice(&13_545_000_u32.to_be_bytes());
    b.push(0); // north pole on the plane
    b.push(0x40);
    b.extend_from_slice(&sm32(latin1));
    b.extend_from_slice(&sm32(latin2));
    b.extend_from_slice(&[0; 8]);
    section(3, &b)
}

/// Section 4, template 4.0: parameter `category`/`number`, 18 h forecast, on `kind` at `value`.
fn product(category: u8, number: u8, kind: u8, value: u32) -> Vec<u8> {
    let mut b = vec![0, 0, 0, 0, category, number, 2, 0, 96, 0, 0, 0, 1];
    b.extend_from_slice(&sm32(18));
    b.extend_from_slice(&[kind, 0]);
    b.extend_from_slice(&value.to_be_bytes());
    b.extend_from_slice(&[255, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    section(4, &b)
}

/// Section 5, template 5.0.
fn packing(count: u32, reference: f32, e: i16, d: i16, bits: u8) -> Vec<u8> {
    let mut b = count.to_be_bytes().to_vec();
    b.extend_from_slice(&[0, 0]);
    b.extend_from_slice(&reference.to_bits().to_be_bytes());
    b.extend_from_slice(&sm16(e));
    b.extend_from_slice(&sm16(d));
    b.extend_from_slice(&[bits, 0]);
    section(5, &b)
}

/// Packs `xs` MSB first at `bits` each.
fn pack(xs: &[u32], bits: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc: u64 = 0;
    let mut have = 0;
    for &x in xs {
        acc = (acc << bits) | u64::from(x);
        have += bits;
        while have >= 8 {
            out.push(u8::try_from((acc >> (have - 8)) & 0xFF).unwrap());
            have -= 8;
        }
    }
    if have > 0 {
        out.push(u8::try_from((acc << (8 - have)) & 0xFF).unwrap());
    }
    out
}

/// A whole message from its sections 1 onward.
fn message(sections: &[Vec<u8>]) -> Vec<u8> {
    let body: Vec<u8> = sections.concat();
    let length = 16 + body.len() + 4;
    let mut m = b"GRIB".to_vec();
    m.extend_from_slice(&[0, 0, 0, 2]);
    m.extend_from_slice(&(length as u64).to_be_bytes());
    m.extend_from_slice(&body);
    m.extend_from_slice(b"7777");
    m
}

/// A 3 by 2 temperature field at 500 hPa on a latitude/longitude grid.
fn temperature(xs: &[u32], bits: u8) -> Vec<u8> {
    message(&[
        identification(),
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(6, 250.0, -2, 1, bits),
        section(6, &[255]),
        section(7, &pack(xs, bits.into())),
    ])
}

#[test]
fn simple_packing_unpacks_by_the_regulation() {
    // Y = (R + X · 2^E) / 10^D, with R = 250, E = −2, D = 1: (250 + X/4)/10.
    let xs = [0, 1, 2, 3, 1000, 4095];
    let bytes = temperature(&xs, 12);
    let fields = parse(&bytes).unwrap();
    assert_eq!(fields.len(), 1);
    let f = &fields[0];
    let expect: Vec<Option<f64>> = xs
        .iter()
        .map(|&x| Some((250.0 + f64::from(x) / 4.0) / 10.0))
        .collect();
    assert_eq!(f.values().unwrap(), expect);
    for (i, e) in expect.iter().enumerate() {
        assert_eq!(f.value(i as u64).unwrap(), *e);
    }
    assert_eq!(
        f.value(6),
        Err(Grib2Error::PointOutside {
            index: 6,
            points: 6
        })
    );
    assert_eq!(f.product.category, 0);
    assert_eq!(f.product.surface.kind, 100);
    assert_eq!(f.product.surface.value, Some(50_000.0));
    assert_eq!(f.product.second_surface.value, None);
    assert_eq!(f.product.forecast_time_s(), Some(18 * 3_600));
    assert_eq!(f.reference_time.year, 2026);
    assert_eq!(f.centre, 7);
}

#[test]
fn every_bit_width_to_32_unpacks() {
    for bits in 1..=32_u8 {
        let top = u32::try_from((1_u64 << bits) - 1).unwrap();
        let xs = [top, 0, top / 3, 1, top, top / 2];
        let bytes = temperature(&xs, bits);
        let f = &parse(&bytes).unwrap()[0];
        let Packing::Simple(p) = f.packing else {
            panic!("simple packing was written");
        };
        for (i, &x) in xs.iter().enumerate() {
            assert_eq!(f.packed(&p, i as u64), x, "{bits} bits, value {i}");
        }
    }
}

#[test]
fn a_zero_bit_field_is_its_reference_everywhere() {
    let bytes = temperature(&[], 0);
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.values().unwrap(), vec![Some(25.0); 6]);
}

#[test]
fn negative_scale_factors_are_sign_and_magnitude() {
    // D = −2 multiplies by 100; E = 3 multiplies X by 8. Two's complement would read 0x8002 as
    // −32766.
    let bytes = message(&[
        identification(),
        latlon_grid(1, 1, 0, 0, 0, 0),
        product(3, 0, 1, 0),
        packing(1, -1.5, 3, -2, 4),
        section(6, &[255]),
        section(7, &pack(&[5], 4)),
    ]);
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.value(0).unwrap(), Some((-1.5 + 5.0 * 8.0) * 100.0));
}

#[test]
fn a_bitmap_skips_the_points_it_marks_missing() {
    // Points 0, 2, 3 and 5 have values; 1 and 4 do not.
    let bytes = message(&[
        identification(),
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(4, 0.0, 0, 0, 8),
        section(6, &[0, 0b1011_0100]),
        section(7, &[10, 20, 30, 40]),
    ]);
    let f = &parse(&bytes).unwrap()[0];
    let expect = [Some(10.0), None, Some(20.0), Some(30.0), None, Some(40.0)];
    assert_eq!(f.values().unwrap(), expect);
    for (i, e) in expect.iter().enumerate() {
        assert_eq!(f.value(i as u64).unwrap(), *e);
    }
}

#[test]
fn a_bitmap_marking_another_count_is_refused() {
    let bytes = message(&[
        identification(),
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(5, 0.0, 0, 0, 8),
        section(6, &[0, 0b1011_0100]),
        section(7, &[10, 20, 30, 40, 50]),
    ]);
    assert!(
        matches!(parse(&bytes), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("marks 4"))
    );
}

#[test]
fn repeated_fields_in_one_message_reuse_the_grid_and_the_bitmap() {
    // Sections 4 to 7 twice after one grid; the second field's section 6 is 254, "the bitmap
    // before".
    let bytes = message(&[
        identification(),
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(4, 0.0, 0, 0, 8),
        section(6, &[0, 0b1011_0100]),
        section(7, &[10, 20, 30, 40]),
        product(0, 0, 100, 70_000),
        packing(4, 1.0, 0, 0, 8),
        section(6, &[254]),
        section(7, &[1, 2, 3, 4]),
    ]);
    let fields = parse(&bytes).unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[1].product.surface.value, Some(70_000.0));
    assert_eq!(
        fields[1].values().unwrap(),
        [Some(2.0), None, Some(3.0), Some(4.0), None, Some(5.0)]
    );
    assert_eq!(fields[0].message, 0);
    assert_eq!(fields[1].message, 0);
}

#[test]
fn messages_follow_one_another() {
    let mut bytes = temperature(&[0; 6], 8);
    bytes.extend(temperature(&[1; 6], 8));
    let fields = parse(&bytes).unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[1].message, 1);
    assert_eq!(fields[1].value(0).unwrap(), Some(25.025));
}

#[test]
fn latlon_points_follow_the_scanning_mode() {
    for (scanning, lat_step) in [(0x40, 0.25), (0x00, -0.25)] {
        let bytes = message(&[
            identification(),
            latlon_grid(3, 2, 32_750_000, 252_750_000, scanning, 0x30),
            product(0, 0, 100, 50_000),
            packing(6, 0.0, 0, 0, 0),
            section(6, &[255]),
            section(7, &[]),
        ]);
        let grid = parse(&bytes).unwrap()[0].grid;
        assert_eq!(grid.point_deg(0), Some((32.75, 252.75)));
        assert_eq!(grid.point_deg(2), Some((32.75, 253.25)));
        assert_eq!(grid.point_deg(4), Some((32.75 + lat_step, 253.0)));
        assert_eq!(grid.point_deg(6), None);
        // A place a third of the way along i and half of j, given west of Greenwich.
        let (i, j) = grid.index_at(32.75 + 0.5 * lat_step, 252.75 + 0.25 / 3.0 - 360.0);
        assert!(
            (i - 1.0 / 3.0).abs() < 1e-12 && (j - 0.5).abs() < 1e-12,
            "{i} {j}"
        );
        assert!(!grid.winds_grid_relative);
        assert_eq!(grid.earth_relative_wind(3.0, 4.0, 253.0), (3.0, 4.0));
    }
}

fn lambert(ni: u32, nj: u32) -> Grid {
    let bytes = message(&[
        identification(),
        lambert_grid(ni, nj, 25_000_000, 25_000_000),
        product(2, 2, 100, 50_000),
        packing(ni * nj, 0.0, 0, 0, 0),
        section(6, &[255]),
        section(7, &[]),
    ]);
    parse(&bytes).unwrap()[0].grid
}

#[test]
fn lambert_points_invert_to_their_indices() {
    let grid = lambert(4, 3);
    assert!(grid.winds_grid_relative);
    let (lat0, lon0) = grid.point_deg(0).unwrap();
    assert!((lat0 - 32.854_458).abs() < 1e-9 && (lon0 - 252.775_632).abs() < 1e-9);
    for index in 0..12 {
        let (lat, lon) = grid.point_deg(index).unwrap();
        let (i, j) = grid.index_at(lat, lon);
        assert!((i - (index % 4) as f64).abs() < 1e-9, "{index}: i {i}");
        assert!((j - (index / 4) as f64).abs() < 1e-9, "{index}: j {j}");
    }
}

#[test]
fn lambert_grid_steps_are_the_grid_length_at_the_tangent_latitude() {
    // Along a row near the tangent latitude the scale factor is 1, so neighbours are 13.545 km
    // apart on the sphere (haversine), to the square of the offset from 25°.
    let bytes = message(&[
        identification(),
        {
            let mut g = lambert_grid(2, 1, 25_000_000, 25_000_000);
            // First point at 25° N, 265° E, on the central meridian.
            g[38..42].copy_from_slice(&sm32(25_000_000));
            g[42..46].copy_from_slice(&sm32(265_000_000));
            g
        },
        product(2, 2, 100, 50_000),
        packing(2, 0.0, 0, 0, 0),
        section(6, &[255]),
        section(7, &[]),
    ]);
    let grid = parse(&bytes).unwrap()[0].grid;
    let (a, b) = (grid.point_deg(0).unwrap(), grid.point_deg(1).unwrap());
    let (p1, p2) = (a.0.to_radians(), b.0.to_radians());
    let dl = (b.1 - a.1).to_radians();
    let h = ((p2 - p1) / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    let d = 2.0 * 6_371_229.0 * h.sqrt().asin();
    assert!((d - 13_545.0).abs() < 0.01, "{d}");
}

#[test]
fn lambert_winds_turn_by_the_meridians_convergence() {
    let grid = lambert(4, 3);
    // West of the central meridian (265°), at 253° E: θ = sin 25° · (−12°).
    let theta = 25_f64.to_radians().sin() * (-12_f64).to_radians();
    assert!((grid.north_to_grid_y_rad(253.0) - theta).abs() < 1e-15);
    assert_eq!(grid.north_to_grid_y_rad(265.0), 0.0);
    // A wind along the grid's +y blows toward its bearing θ: east component sin θ.
    let (u, v) = grid.earth_relative_wind(0.0, 10.0, 253.0);
    assert!((u - 10.0 * theta.sin()).abs() < 1e-12 && (v - 10.0 * theta.cos()).abs() < 1e-12);
    // The turn keeps the speed.
    let (u, v) = grid.earth_relative_wind(3.0, -7.0, 253.0);
    assert!((u.hypot(v) - 3_f64.hypot(7.0)).abs() < 1e-12);
    // The grid's +x runs a little north of east there, as the grid points show: a step along i
    // gains latitude.
    let (lat0, _) = grid.point_deg(0).unwrap();
    let (lat1, _) = grid.point_deg(1).unwrap();
    assert!(lat1 > lat0);
}

#[test]
fn a_secant_lambert_cone_is_refused() {
    let bytes = message(&[
        identification(),
        lambert_grid(4, 3, 25_000_000, 50_000_000),
        product(2, 2, 100, 50_000),
        packing(12, 0.0, 0, 0, 0),
        section(6, &[255]),
        section(7, &[]),
    ]);
    assert!(
        matches!(parse(&bytes), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("tangent cone"))
    );
}

#[test]
fn other_templates_are_refused_by_name() {
    // PNG, template 5.41.
    let mut repr = packing(6, 0.0, 0, 0, 8);
    repr[10] = 41;
    let bytes = message(&[
        identification(),
        latlon_grid(3, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        repr,
        section(6, &[255]),
        section(7, &[0; 6]),
    ]);
    assert_eq!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            message: 0,
            what: "data representation template",
            value: 41
        })
    );
    // A Gaussian grid, template 3.40.
    let mut grid = latlon_grid(3, 2, 0, 0, 0x40, 0x30);
    grid[13] = 40;
    let bytes = message(&[identification(), grid]);
    assert!(matches!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            what: "grid definition template",
            value: 40,
            ..
        })
    ));
    // Scanning in −i.
    let bytes = message(&[identification(), latlon_grid(3, 2, 0, 0, 0xC0, 0x30)]);
    assert!(matches!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            what: "scanning mode",
            value: 0xC0,
            ..
        })
    ));
}

#[test]
fn broken_files_are_refused() {
    let good = temperature(&[1, 2, 3, 4, 5, 6], 8);
    // Not GRIB.
    assert!(matches!(
        parse(b"GRIX0000000000000000"),
        Err(Grib2Error::NotGrib { offset: 0, .. })
    ));
    // Junk after a message.
    let mut junk = good.clone();
    junk.extend_from_slice(b"\n");
    assert!(
        matches!(parse(&junk), Err(Grib2Error::NotGrib { offset, .. }) if offset == good.len())
    );
    // Edition 1.
    let mut ed1 = good.clone();
    ed1[7] = 1;
    assert!(matches!(
        parse(&ed1),
        Err(Grib2Error::Edition { edition: 1, .. })
    ));
    // Cut short.
    assert!(matches!(
        parse(&good[..good.len() - 1]),
        Err(Grib2Error::Truncated { .. })
    ));
    // A wrong end marker.
    let mut end = good.clone();
    let n = end.len();
    end[n - 1] = b'6';
    assert!(matches!(parse(&end), Err(Grib2Error::Malformed { .. })));
    // Packed values shorter than the count needs.
    let short = message(&[
        identification(),
        latlon_grid(3, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(6, 0.0, 0, 0, 8),
        section(6, &[255]),
        section(7, &[1, 2, 3, 4, 5]),
    ]);
    assert!(matches!(
        parse(&short),
        Err(Grib2Error::Truncated {
            what: "the packed values",
            needed: 6,
            available: 5,
            ..
        })
    ));
    // A section running past the message.
    let mut long = good.clone();
    long[16..20].copy_from_slice(&1000_u32.to_be_bytes());
    assert!(matches!(parse(&long), Err(Grib2Error::Truncated { .. })));
    // A field without its bitmap section.
    let bare = message(&[
        identification(),
        latlon_grid(3, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(6, 0.0, 0, 0, 8),
        section(7, &[1, 2, 3, 4, 5, 6]),
    ]);
    assert!(matches!(parse(&bare), Err(Grib2Error::Malformed { .. })));
    // Scale factors that overflow.
    let huge = message(&[
        identification(),
        latlon_grid(3, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(6, 1.0, 2000, 0, 8),
        section(6, &[255]),
        section(7, &[1, 2, 3, 4, 5, 6]),
    ]);
    assert!(
        matches!(parse(&huge), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("not finite"))
    );
}

#[test]
fn a_huge_grid_claimed_by_a_tiny_file_is_refused() {
    // 0 bits per value would let a 100-byte file claim any grid; MAX_POINTS bounds it.
    // 2²⁴ points along each of 2 rows: on the Earth, but twice MAX_POINTS.
    let row = 1 << 24;
    let bytes = message(&[
        identification(),
        latlon_grid(row, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(row * 2, 0.0, 0, 0, 0),
        section(6, &[255]),
        section(7, &[]),
    ]);
    assert!(matches!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            what: "grid of this many points",
            ..
        })
    ));
    // A grid whose size and point count disagree.
    let mut grid = latlon_grid(3, 2, 0, 0, 0x40, 0x30);
    grid[6..10].copy_from_slice(&7_u32.to_be_bytes());
    let bytes = message(&[identification(), grid]);
    assert!(
        matches!(parse(&bytes), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("declares 7"))
    );
}

/// [`two_messages`], then a field of second-order differences with missing values and a bitmap,
/// one of complex packing whose lists have no bits, and one of template 4.8: every section and
/// template the reader takes.
fn seed() -> Vec<u8> {
    let mut bytes = two_messages();
    let g = Groups {
        groups: &[(0, 3, 5)],
        x2: &[0, 0, 5, 7, 0],
    };
    let repr = complex(5, 1, &g, Some((2, 1, [100, 103], -3)));
    bytes.extend(complex_field(repr, section(6, &[0, 0b1101_1100])));
    let lists = Lists {
        width_bits: 0,
        width_reference: 2,
        length_bits: 0,
        length_reference: 2,
        ..LISTS
    };
    let g = Groups {
        groups: &[(1, 2, 2), (5, 2, 2), (9, 2, 2)],
        x2: &[0, 3, 1, 2, 0, 1],
    };
    bytes.extend(complex_field(
        complex_with(6, 0, &g, None, lists),
        section(6, &[255]),
    ));
    let mut sections = temperature_sections(packing(6, 0.0, 0, 0, 8), section(6, &[255]));
    let mut body = sections[1][5..].to_vec();
    body[3] = 8;
    body.extend_from_slice(&2026_u16.to_be_bytes());
    body.extend_from_slice(&[
        9, 30, 18, 0, 0, 1, 0, 0, 0, 0, 1, 2, 1, 0, 0, 0, 6, 1, 0, 0, 0, 0,
    ]);
    sections[1] = section(4, &body);
    let mut all = vec![identification()];
    all.extend(sections);
    bytes.extend(message(&all));
    bytes
}

/// A message with a bitmap, then one on a Lambert grid, back to back.
fn two_messages() -> Vec<u8> {
    let mut bytes = message(&[
        identification(),
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(4, 1.5, -1, 1, 7),
        section(6, &[0, 0b1011_0100]),
        section(7, &pack(&[1, 90, 3, 127], 7)),
    ]);
    bytes.extend(message(&[
        identification(),
        lambert_grid(2, 2, 25_000_000, 25_000_000),
        product(2, 2, 103, 10),
        packing(4, -3.0, 0, 2, 9),
        section(6, &[255]),
        section(7, &pack(&[0, 511, 256, 7], 9)),
    ]));
    bytes
}

proptest::proptest! {
    /// Any byte changed, and any cut, reads or is refused: it never panics, and a field that
    /// reads gives every value, each one finite, by `value` as by `values`.
    #[test]
    fn damaged_files_never_panic(
        edits in proptest::collection::vec((0_usize..700, 0_u8..=255), 0..6),
        cut in 0_usize..700,
    ) {
        let mut bytes = seed();
        for (at, byte) in edits {
            let n = bytes.len();
            bytes[at % n] = byte;
        }
        let n = bytes.len();
        bytes.truncate(n - cut % n);
        if let Ok(fields) = parse(&bytes) {
            for f in &fields {
                // A complex-packed field whose differences overflow is refused when read.
                let Ok(values) = f.values() else {
                    let every: Vec<u64> = (0..f.points()).collect();
                    proptest::prop_assert!(f.values_at(&every).is_err());
                    continue;
                };
                proptest::prop_assert_eq!(values.len() as u64, f.points());
                for (i, v) in values.iter().enumerate() {
                    proptest::prop_assert_eq!(f.value(i as u64).unwrap(), *v);
                    proptest::prop_assert!(v.is_none_or(f64::is_finite));
                }
                let _ = f.grid.point_deg(0);
                let _ = f.grid.index_at(33.0, 253.0);
            }
        }
    }
}

#[test]
fn a_wind_along_the_grids_x_axis_bears_east_by_theta() {
    // The grid's +x is east turned by θ: a 10 m/s wind along it has east component 10 cos θ and
    // north −10 sin θ (north of east when θ < 0, as the grid points' latitudes show).
    let grid = lambert(4, 3);
    let theta = grid.north_to_grid_y_rad(253.0);
    let (u, v) = grid.earth_relative_wind(10.0, 0.0, 253.0);
    assert!((u - 10.0 * theta.cos()).abs() < 1e-12, "{u}");
    assert!((v + 10.0 * theta.sin()).abs() < 1e-12, "{v}");
    assert!(v > 0.0);
}

/// A message whose sections 3 to 7 are `sections`.
fn with(sections: Vec<Vec<u8>>) -> Result<Vec<Field<'static>>, Grib2Error> {
    let mut all = vec![identification()];
    all.extend(sections);
    let bytes: &'static [u8] = Vec::leak(message(&all));
    parse(bytes)
}

fn temperature_sections(repr: Vec<u8>, bitmap: Vec<u8>) -> Vec<Vec<u8>> {
    vec![
        latlon_grid(3, 2, 32_750_000, 252_750_000, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        repr,
        bitmap,
        section(7, &[0; 24]),
    ]
}

/// The refusal a message gets, as `(what, value)` for [`Grib2Error::Unsupported`] or the reason
/// for [`Grib2Error::Malformed`].
fn refusal(sections: Vec<Vec<u8>>) -> String {
    match with(sections) {
        Err(Grib2Error::Unsupported { what, value, .. }) => format!("{what} {value}"),
        Err(Grib2Error::Malformed { reason, .. }) => reason,
        other => panic!("{other:?}"),
    }
}

#[test]
fn each_refusal_names_its_cause() {
    let plain = || packing(6, 0.0, 0, 0, 8);
    let none = || section(6, &[255]);
    // Section 6: a bitmap reused before any is given, and a predefined one.
    assert!(refusal(temperature_sections(plain(), section(6, &[254]))).contains("reuses a bitmap"));
    assert_eq!(
        refusal(temperature_sections(plain(), section(6, &[7]))),
        "bitmap indicator 7"
    );
    // Section 5: more than 32 bits, a NaN reference, integers of another type.
    assert_eq!(
        refusal(temperature_sections(packing(6, 0.0, 0, 0, 33), none())),
        "bits per value 33"
    );
    assert!(
        refusal(temperature_sections(packing(6, f32::NAN, 0, 0, 8), none())).contains("not finite")
    );
    let mut other_type = plain();
    other_type[20] = 2;
    assert_eq!(
        refusal(temperature_sections(other_type, none())),
        "type of original field values 2"
    );
    // A decimal scale that overflows the other way.
    assert!(
        refusal(temperature_sections(packing(6, 1.0, 0, -400, 8), none())).contains("not finite")
    );
    // Section 4: another product template.
    let mut sections = temperature_sections(plain(), none());
    sections[1][8] = 15;
    assert_eq!(refusal(sections), "product definition template 15");
    // Section 7 before section 4.
    let sections = vec![latlon_grid(3, 2, 0, 0, 0x40, 0x30), section(7, &[0; 6])];
    assert!(refusal(sections).contains("before sections"));
    // A basic angle of 1 with 0 subdivisions reads as millionths of a degree, as ecCodes reads it.
    let mut grid = latlon_grid(3, 2, 32_750_000, 0, 0x40, 0x30);
    grid[38..42].copy_from_slice(&1_u32.to_be_bytes());
    grid[42..46].copy_from_slice(&0_u32.to_be_bytes());
    let mut sections = temperature_sections(plain(), none());
    sections[0] = grid;
    let read = with(sections).unwrap()[0].grid;
    assert_eq!(read.point_deg(4), Some((33.0, 0.25)));
    // Rows running off the Earth: 2 rows from 90° N northward.
    assert!(refusal(vec![latlon_grid(3, 2, 90_000_000, 0, 0x40, 0x30)]).contains("off the Earth"));
    // Lambert: the south pole on the plane; an oblate Earth; LaD off the tangent latitude; a
    // radius not the Earth's.
    let mut south = lambert_grid(4, 3, 25_000_000, 25_000_000);
    south[63] = 0x80;
    assert_eq!(refusal(vec![south]), "Lambert projection centre flag 128");
    let mut oblate = lambert_grid(4, 3, 25_000_000, 25_000_000);
    oblate[14] = 5;
    assert_eq!(
        refusal(vec![oblate]),
        "Lambert grid on the figure of the Earth 5"
    );
    let mut lad = lambert_grid(4, 3, 25_000_000, 25_000_000);
    lad[47..51].copy_from_slice(&sm32(38_500_000));
    assert!(refusal(vec![lad]).contains("tangent cone"));
    let mut tiny = lambert_grid(4, 3, 25_000_000, 25_000_000);
    tiny[14] = 1;
    tiny[15] = 0;
    tiny[16..20].copy_from_slice(&0_u32.to_be_bytes());
    assert!(refusal(vec![tiny]).contains("radius is 0 m"));
}

#[test]
fn a_given_radius_is_read_with_its_scale() {
    // Code 1: 6,371,000 m written as 63,710 × 10² (scale factor −2).
    let mut g = lambert_grid(4, 3, 25_000_000, 25_000_000);
    g[14] = 1;
    g[15] = 0x82;
    g[16..20].copy_from_slice(&63_710_u32.to_be_bytes());
    let fields = with(vec![
        g,
        product(2, 2, 100, 50_000),
        packing(12, 0.0, 0, 0, 0),
        section(6, &[255]),
        section(7, &[]),
    ])
    .unwrap();
    assert_eq!(
        fields[0].grid.earth,
        Earth::Sphere {
            radius_m: 6_371_000.0
        }
    );
}

#[test]
#[expect(
    clippy::disallowed_types,
    reason = "a test timing the reader, not a result; the core itself reads no clock"
)]
fn reusing_one_large_bitmap_costs_no_more_per_field() {
    // 20,000 fields over one bitmap of about a million points, the last point's value read from
    // each: counting the bitmap per field and per value would read 5 GB, seconds even optimised;
    // the running counts make each a few dozen bytes.
    let (ni, nj) = (1_440_u32, 721_u32);
    let points = ni * nj;
    let mut bits = vec![0_u8; usize::try_from(points.div_ceil(8)).unwrap()];
    bits[0] = 0x80; // the first point and the last have values
    let last = usize::try_from(points - 1).unwrap();
    bits[last / 8] |= 0x80 >> (last % 8);
    let mut sections = vec![latlon_grid(ni, nj, -90_000_000, 0, 0x40, 0x30)];
    for k in 0..20_000_u32 {
        sections.push(product(0, 0, 100, k));
        sections.push(packing(2, 0.0, 0, 0, 8));
        let s6 = if k == 0 {
            [&[0][..], &bits].concat()
        } else {
            vec![254]
        };
        sections.push(section(6, &s6));
        sections.push(section(7, &[1, u8::try_from(k % 256).unwrap()]));
    }
    let start = std::time::Instant::now();
    let fields = with(sections).unwrap();
    assert_eq!(fields.len(), 20_000);
    for (k, f) in fields.iter().enumerate() {
        assert_eq!(
            f.value(u64::from(points) - 1).unwrap(),
            Some((k % 256) as f64)
        );
        assert_eq!(f.value(1).unwrap(), None);
    }
    // Measured in milliseconds; a loaded debug build has seconds to spare.
    assert!(start.elapsed().as_secs_f64() < 5.0, "{:?}", start.elapsed());
}

#[test]
fn running_counts_rank_every_point() {
    // A pseudo-random bitmap across several 64-byte blocks, against a plain count.
    let mut state = 0x2545_f491_u32;
    let bits: Vec<u8> = (0..300)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state & 0xFF) as u8
        })
        .collect();
    let bitmap = Bitmap::new(&bits);
    let mut plain = 0;
    for index in 0..(bits.len() as u64 * 8) {
        assert_eq!(bitmap.ones_before(index), plain, "bit {index}");
        let set = bits[(index / 8) as usize] & (0x80 >> (index % 8)) != 0;
        assert_eq!(bitmap.get(index), set);
        plain += u64::from(set);
    }
    assert_eq!(bitmap.ones_before(bits.len() as u64 * 8), plain);
}

/// Bits written first bit high, as GRIB2 packs them.
#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u32,
}

impl Bits {
    fn push(&mut self, value: u64, bits: u32) {
        for k in (0..bits).rev() {
            if self.used.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if value >> k & 1 == 1 {
                *self.bytes.last_mut().unwrap() |= 0x80 >> (self.used % 8);
            }
            self.used += 1;
        }
    }

    /// Pads to a whole byte, as each list in section 7 is.
    fn pad(&mut self) {
        self.used = self.used.div_ceil(8) * 8;
    }
}

/// Complex packing's groups: reference, width and length of each, then each value's `X2`.
struct Groups<'a> {
    groups: &'a [(u64, u32, u64)],
    x2: &'a [u64],
}

/// The lists' layout: bits per reference, width and length, the width and length references,
/// and the length increment.
#[derive(Clone, Copy)]
struct Lists {
    reference_bits: u32,
    width_bits: u32,
    width_reference: u8,
    length_bits: u32,
    length_reference: u64,
    increment: u64,
}

/// 8-bit references, 2-bit widths above 0, and lengths scaled from 1 by 1 in 2 bits.
const LISTS: Lists = Lists {
    reference_bits: 8,
    width_bits: 2,
    width_reference: 0,
    length_bits: 2,
    length_reference: 1,
    increment: 1,
};

/// Section 5, template 5.2 (`sd` `None`) or 5.3 (`sd` the order and bytes per descriptor),
/// with [`LISTS`]; and section 7 with its descriptors (`first`, `minimum`), lists and values.
fn complex(
    count: u32,
    missing: u8,
    g: &Groups<'_>,
    sd: Option<(u8, u8, [i64; 2], i64)>,
) -> (Vec<u8>, Vec<u8>) {
    complex_with(count, missing, g, sd, LISTS)
}

/// [`complex`] with other lists.
fn complex_with(
    count: u32,
    missing: u8,
    g: &Groups<'_>,
    sd: Option<(u8, u8, [i64; 2], i64)>,
    l: Lists,
) -> (Vec<u8>, Vec<u8>) {
    let n = g.groups.len();
    let mut b = count.to_be_bytes().to_vec();
    b.extend_from_slice(&(if sd.is_some() { 3_u16 } else { 2 }).to_be_bytes());
    b.extend_from_slice(&1.5_f32.to_bits().to_be_bytes());
    b.extend_from_slice(&sm16(-1));
    b.extend_from_slice(&sm16(1));
    // Reference bits, type, splitting, missing values.
    b.extend_from_slice(&[u8::try_from(l.reference_bits).unwrap(), 0, 1, missing]);
    b.extend_from_slice(&[0xFF; 8]); // missing value substitutes, unused
    b.extend_from_slice(&u32::try_from(n).unwrap().to_be_bytes());
    b.extend_from_slice(&[l.width_reference, u8::try_from(l.width_bits).unwrap()]);
    b.extend_from_slice(&u32::try_from(l.length_reference).unwrap().to_be_bytes());
    b.push(u8::try_from(l.increment).unwrap());
    b.extend_from_slice(&u32::try_from(g.groups[n - 1].2).unwrap().to_be_bytes());
    b.push(u8::try_from(l.length_bits).unwrap());
    let mut d = Bits::default();
    if let Some((order, octets, first, minimum)) = sd {
        b.extend_from_slice(&[order, octets]);
        let width = 8 * u32::from(octets);
        let signed = |v: i64| v.unsigned_abs() | if v < 0 { 1 << (width - 1) } else { 0 };
        for &f in first.iter().take(usize::from(order)) {
            d.push(signed(f), width);
        }
        d.push(signed(minimum), width);
    }
    for &(reference, _, _) in g.groups {
        d.push(reference, l.reference_bits);
    }
    d.pad();
    for &(_, width, _) in g.groups {
        d.push(
            u64::from(width - u32::from(l.width_reference)),
            l.width_bits,
        );
    }
    d.pad();
    for (k, &(_, _, length)) in g.groups.iter().enumerate() {
        // The last group's length is in section 5; its place in the list is unused.
        let scaled = if k + 1 == n {
            0
        } else {
            (length - l.length_reference) / l.increment
        };
        d.push(scaled, l.length_bits);
    }
    d.pad();
    let mut x2 = g.x2.iter();
    for &(_, width, length) in g.groups {
        for _ in 0..(if width > 0 { length } else { 0 }) {
            d.push(*x2.next().unwrap(), width);
        }
    }
    (section(5, &b), section(7, &d.bytes))
}

/// A 3 by 2 temperature field over complex packing, with `bitmap` as section 6.
fn complex_field(repr: (Vec<u8>, Vec<u8>), bitmap: Vec<u8>) -> Vec<u8> {
    message(&[
        identification(),
        latlon_grid(3, 2, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        repr.0,
        bitmap,
        repr.1,
    ])
}

/// `Y = (1.5 + X / 2) / 10`, the builder's scaling.
fn y(x: f64) -> f64 {
    (1.5 + x * 0.5) / 10.0
}

#[test]
fn complex_packing_adds_each_groups_reference() {
    // A constant group of 3 at 10; 2 values of 3 bits above 5; the last, 1 of 2 bits above 100.
    let g = Groups {
        groups: &[(10, 0, 3), (5, 3, 2), (100, 2, 1)],
        x2: &[0, 7, 3],
    };
    let bytes = complex_field(complex(6, 0, &g, None), section(6, &[255]));
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.packing.template(), 2);
    let expect: Vec<_> = [10.0, 10.0, 10.0, 5.0, 12.0, 103.0]
        .map(|x| Some(y(x)))
        .into();
    assert_eq!(f.values().unwrap(), expect);
    // Points read alone, and several out of order in one pass, give the same values.
    for (k, v) in expect.iter().enumerate() {
        assert_eq!(f.value(k as u64).unwrap(), *v);
    }
    assert_eq!(
        f.values_at(&[5, 0, 4, 4]).unwrap(),
        [expect[5], expect[0], expect[4], expect[4]]
    );
    assert!(matches!(
        f.values_at(&[1, 6]),
        Err(Grib2Error::PointOutside { index: 6, .. })
    ));
}

#[test]
fn complex_packing_marks_primary_and_secondary_missing_values() {
    // Reference all ones (primary) and all ones but the last bit (secondary) in groups of width 0;
    // X2 = 3 (primary) and 2 (secondary) in a group of width 2.
    let g = Groups {
        groups: &[(255, 0, 2), (254, 0, 1), (7, 2, 3)],
        x2: &[3, 2, 1],
    };
    let bytes = complex_field(complex(6, 2, &g, None), section(6, &[255]));
    let values = parse(&bytes).unwrap()[0].values().unwrap();
    assert_eq!(values, [None, None, None, None, None, Some(y(8.0))]);
    // With primary missing values only, the secondary codes are values.
    let bytes = complex_field(complex(6, 1, &g, None), section(6, &[255]));
    let values = parse(&bytes).unwrap()[0].values().unwrap();
    assert_eq!(
        values,
        [None, None, Some(y(254.0)), None, Some(y(9.0)), Some(y(8.0))]
    );
}

#[test]
fn first_order_differences_rebuild_the_values() {
    // h = 20, 22, 21, 25, 25, 30: differences 2, −1, 4, 0, 5; minimum −1; packed 3, 0, 5, 1, 6
    // after an unused first place.
    let g = Groups {
        groups: &[(0, 3, 6)],
        x2: &[0, 3, 0, 5, 1, 6],
    };
    let bytes = complex_field(
        complex(6, 0, &g, Some((1, 2, [20, 0], -1))),
        section(6, &[255]),
    );
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.packing.template(), 3);
    let expect: Vec<_> = [20.0, 22.0, 21.0, 25.0, 25.0, 30.0]
        .map(|x| Some(y(x)))
        .into();
    assert_eq!(f.values().unwrap(), expect);
}

#[test]
fn second_order_differences_skip_missing_values_and_the_bitmap() {
    // The bitmap leaves out point 2; the packed value in place 3 is missing (X2 = 7). The values
    // present, h = 100, 103, 108, 110, have second differences 2 and −3; minimum −3, packed 5
    // and 0 after two unused places.
    let g = Groups {
        groups: &[(0, 3, 5)],
        x2: &[0, 0, 5, 7, 0],
    };
    let bytes = complex_field(
        complex(5, 1, &g, Some((2, 1, [100, 103], -3))),
        section(6, &[0, 0b1101_1100]),
    );
    let f = &parse(&bytes).unwrap()[0];
    let expect = [
        Some(y(100.0)),
        Some(y(103.0)),
        None,
        Some(y(108.0)),
        None,
        Some(y(110.0)),
    ];
    assert_eq!(f.values().unwrap(), expect);
    assert_eq!(
        f.values_at(&[5, 3, 2]).unwrap(),
        [expect[5], expect[3], None]
    );
}

#[test]
fn broken_complex_packing_is_refused() {
    let bitmap = || section(6, &[255]);
    let one = |count, groups: &[(u64, u32, u64)], x2: &[u64]| {
        complex_field(complex(count, 0, &Groups { groups, x2 }, None), bitmap())
    };
    let refused = |bytes: Vec<u8>| match parse(&bytes) {
        Err(Grib2Error::Unsupported { what, value, .. }) => format!("{what} {value}"),
        Err(Grib2Error::Malformed { reason, .. }) => reason,
        Err(Grib2Error::Truncated { what, .. }) => what.to_owned(),
        other => panic!("{other:?}"),
    };
    // Groups holding 5 values for a count of 6.
    let bytes = one(6, &[(0, 1, 2), (0, 1, 3)], &[0; 5]);
    assert_eq!(
        refused(bytes),
        "the groups hold 5 values but section 5 packs 6"
    );
    // More groups than values.
    let mut repr = complex(
        6,
        0,
        &Groups {
            groups: &[(0, 0, 6)],
            x2: &[],
        },
        None,
    );
    repr.0[31..35].copy_from_slice(&7_u32.to_be_bytes());
    assert_eq!(
        refused(complex_field(repr, bitmap())),
        "7 groups for 6 packed values"
    );
    // A group of 33 bits: a width reference of 31 plus 2.
    let mut repr = complex(
        6,
        0,
        &Groups {
            groups: &[(0, 2, 6)],
            x2: &[0; 6],
        },
        None,
    );
    repr.0[35] = 31;
    assert_eq!(
        refused(complex_field(repr, bitmap())),
        "group width, bits 33"
    );
    // Packed values cut short.
    let mut repr = complex(
        6,
        0,
        &Groups {
            groups: &[(0, 3, 6)],
            x2: &[1; 6],
        },
        None,
    );
    repr.1.truncate(repr.1.len() - 1);
    let len = u32::try_from(repr.1.len()).unwrap();
    repr.1[..4].copy_from_slice(&len.to_be_bytes());
    assert_eq!(refused(complex_field(repr, bitmap())), "the packed values");
    // A third order, no descriptor bytes, 33-bit references, and missing values with 0-bit
    // references.
    let g = Groups {
        groups: &[(0, 0, 6)],
        x2: &[],
    };
    for (at, byte, cause) in [
        (47, 3, "order of spatial differencing 3"),
        (48, 0, "bytes per extra descriptor 0"),
        (19, 33, "bits per group reference 33"),
        (22, 3, "missing value management 3"),
    ] {
        let mut repr = complex(6, 0, &g, Some((1, 1, [0, 0], 0)));
        repr.0[at] = byte;
        assert_eq!(refused(complex_field(repr, bitmap())), cause);
    }
    // Second differences past 2^63 parse, but are refused when read.
    let bytes = complex_field(
        complex(
            6,
            0,
            &Groups {
                groups: &[(0, 0, 6)],
                x2: &[],
            },
            Some((2, 8, [0, 1 << 62], 0)),
        ),
        bitmap(),
    );
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.value(1).unwrap(), Some(y(2_f64.powi(62))));
    for read in [f.values().map(|_| ()), f.value(2).map(|_| ())] {
        assert!(
            matches!(read, Err(Grib2Error::Malformed { reason, .. }) if reason.contains("overflow"))
        );
    }
}

#[test]
fn template_4_8_reads_its_interval() {
    let mut sections = temperature_sections(packing(6, 0.0, 0, 0, 8), section(6, &[255]));
    // Template 4.8: 4.0's octets, then the interval's end (2026-09-30 18:07:09), one time range,
    // no missing values, and a minimum (3) over 6 units of 6 hours (11).
    let mut body = sections[1][5..].to_vec();
    body[3] = 8;
    body.extend_from_slice(&2026_u16.to_be_bytes());
    body.extend_from_slice(&[9, 30, 18, 7, 9, 1, 0, 0, 0, 0, 3, 2, 11]);
    body.extend_from_slice(&6_u32.to_be_bytes());
    body.extend_from_slice(&[1, 0, 0, 0, 0]);
    sections[1] = section(4, &body);
    let p = with(sections.clone()).unwrap()[0].product;
    assert_eq!(p.template, 8);
    let s = p.statistics.unwrap();
    assert_eq!((s.process, s.time_unit, s.length), (3, 11, 6));
    let e = s.end;
    assert_eq!(
        (e.year, e.month, e.day, e.hour, e.minute, e.second),
        (2026, 9, 30, 18, 7, 9)
    );
    // Two time ranges are refused.
    sections[1][41] = 2;
    assert_eq!(refusal(sections), "number of time ranges 2");
}

#[test]
fn group_lengths_scale_by_their_increment() {
    // Lengths 1 + 1 · 3 = 4, then the last's 2, from a length reference of 1 and increment 3.
    let g = Groups {
        groups: &[(10, 0, 4), (100, 2, 2)],
        x2: &[3, 1],
    };
    let lists = Lists {
        increment: 3,
        ..LISTS
    };
    let bytes = complex_field(complex_with(6, 0, &g, None, lists), section(6, &[255]));
    let expect: Vec<_> = [10.0, 10.0, 10.0, 10.0, 103.0, 101.0]
        .map(|x| Some(y(x)))
        .into();
    assert_eq!(parse(&bytes).unwrap()[0].values().unwrap(), expect);
}

#[test]
fn lists_of_no_bits_make_every_group_alike() {
    // No width or length list: every group is 2 wide and 2 long (the last's length given as 2).
    let g = Groups {
        groups: &[(1, 2, 2), (5, 2, 2), (9, 2, 2)],
        x2: &[0, 3, 1, 2, 0, 1],
    };
    let lists = Lists {
        width_bits: 0,
        width_reference: 2,
        length_bits: 0,
        length_reference: 2,
        ..LISTS
    };
    let bytes = complex_field(complex_with(6, 0, &g, None, lists), section(6, &[255]));
    let expect: Vec<_> = [1.0, 4.0, 6.0, 7.0, 9.0, 10.0].map(|x| Some(y(x))).into();
    assert_eq!(parse(&bytes).unwrap()[0].values().unwrap(), expect);
    // Groups that don't add up to the count are refused without reading a list.
    let bytes = complex_field(complex_with(5, 0, &g, None, lists), section(6, &[255]));
    assert!(parse(&bytes).is_err());
}

#[test]
fn missing_values_with_references_of_no_bits_mark_every_constant_group() {
    // With 0-bit references every reference is all ones: a group of width 0 is missing, and one
    // of width 2 has its values above 0, 3 marking a missing one.
    let g = Groups {
        groups: &[(0, 0, 2), (0, 2, 4)],
        x2: &[1, 3, 2, 0],
    };
    let lists = Lists {
        reference_bits: 0,
        ..LISTS
    };
    let bytes = complex_field(complex_with(6, 1, &g, None, lists), section(6, &[255]));
    assert_eq!(
        parse(&bytes).unwrap()[0].values().unwrap(),
        [None, None, Some(y(1.0)), None, Some(y(2.0)), Some(y(0.0))]
    );
}

#[test]
fn ambiguous_or_unread_complex_packing_is_refused_by_name() {
    let refused = |repr: (Vec<u8>, Vec<u8>)| match parse(&complex_field(repr, section(6, &[255]))) {
        Err(Grib2Error::Unsupported { what, value, .. }) => format!("{what} {value}"),
        Err(Grib2Error::Malformed { reason, .. }) => reason,
        other => panic!("{other:?}"),
    };
    let g = Groups {
        groups: &[(0, 1, 6)],
        x2: &[0; 6],
    };
    // Row-by-row group splitting.
    let mut repr = complex(6, 0, &g, None);
    repr.0[21] = 0;
    assert_eq!(refused(repr), "group splitting method 0");
    // Secondary missing values with 0-bit references.
    let lists = Lists {
        reference_bits: 0,
        ..LISTS
    };
    assert_eq!(
        refused(complex_with(6, 2, &g, None, lists)),
        "secondary missing values with 0-bit group references, management 2"
    );
    // No groups for 6 values.
    let mut repr = complex(6, 0, &g, None);
    repr.0[31..35].copy_from_slice(&0_u32.to_be_bytes());
    assert_eq!(refused(repr), "complex packing with no groups, values 6");
    // A first value with its top bit set: 200 in one byte.
    assert!(refused(complex(6, 0, &g, Some((1, 1, [200, 0], 0)))).contains("top bit"));
    // A first group far longer than the count ends the sum at once.
    let two = Groups {
        groups: &[(0, 1, 4), (0, 1, 2)],
        x2: &[0; 6],
    };
    let mut repr = complex(6, 0, &two, None);
    repr.0[37..41].copy_from_slice(&u32::MAX.to_be_bytes());
    repr.0[41] = 255;
    // 2^32 − 1 + 3 · 255, and the second group not added.
    assert_eq!(
        refused(repr),
        "the groups hold 4294968060 values but section 5 packs 6"
    );
    // No groups and no values (a bitmap marking no point) read as a field with no values.
    let lists = Lists {
        width_bits: 0,
        length_bits: 0,
        ..LISTS
    };
    let mut repr = complex_with(0, 0, &g, None, lists);
    repr.0[31..35].copy_from_slice(&0_u32.to_be_bytes());
    let bytes = complex_field(repr, section(6, &[0, 0]));
    assert_eq!(parse(&bytes).unwrap()[0].values().unwrap(), [None; 6]);
}

#[test]
fn the_damaged_files_start_from_every_template() {
    let bytes = seed();
    let fields = parse(&bytes).unwrap();
    let templates: Vec<(u16, u16)> = fields
        .iter()
        .map(|f| (f.product.template, f.packing.template()))
        .collect();
    for want in [(0, 0), (0, 2), (0, 3), (8, 0)] {
        assert!(templates.contains(&want), "{want:?} in {templates:?}");
    }
    for f in &fields {
        f.values().unwrap();
    }
}

/// The RAP fixture's first message (grid 200, 500 hPa temperature, 6 bits, no bitmap), and where
/// its sections 5 and 7 start.
fn rap_jpeg2000() -> (Vec<u8>, usize, usize) {
    let file = include_bytes!("../../tests/fixtures/rap-jpeg2000.grib2");
    let length = usize::try_from(u64::from_be_bytes(file[8..16].try_into().unwrap())).unwrap();
    let bytes = file[..length].to_vec();
    let (mut at, mut starts) = (16, [0; 8]);
    while at + 5 <= length && &bytes[at..at + 4] != b"7777" {
        let n = usize::try_from(be_u32(&bytes[at..at + 4])).unwrap();
        starts[usize::from(bytes[at + 4])] = at;
        at += n;
    }
    (bytes, starts[5], starts[7])
}

#[test]
fn jpeg2000_reads_its_template_and_refuses_what_it_cannot_decode() {
    let (good, s5, s7) = rap_jpeg2000();
    let fields = parse(&good).unwrap();
    let Packing::Jpeg2000(p) = fields[0].packing else {
        panic!("{:?}", fields[0].packing)
    };
    assert_eq!(
        (p.bits, p.binary_scale, p.decimal_scale, p.count),
        (6, -4, 0, 10_152)
    );
    assert_eq!(fields[0].packing.template(), 40);
    // More bits than the decoder's `f32` wavelet keeps exact.
    let mut bytes = good.clone();
    bytes[s5 + 19] = MAX_JPEG2000_BITS + 1;
    assert_eq!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            message: 0,
            what: "bits per value in JPEG 2000",
            value: 22
        })
    );
    // Lossy coding.
    let mut bytes = good.clone();
    bytes[s5 + 21] = 1;
    assert_eq!(
        parse(&bytes),
        Err(Grib2Error::Unsupported {
            message: 0,
            what: "type of JPEG 2000 compression",
            value: 1
        })
    );
    // Section 5's bits not the image's.
    let mut bytes = good.clone();
    bytes[s5 + 19] = 7;
    assert!(
        matches!(parse(&bytes), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("6 bits per sample but section 5 gives 7"))
    );
    // Not a codestream.
    let mut bytes = good.clone();
    bytes[s7 + 5] = 0;
    assert!(
        matches!(parse(&bytes), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("the JPEG 2000 codestream"))
    );
    // A codestream cut short, at every length: refused when parsed or when read, never decoded
    // (the decoder's lenient mode would fill a cut image with its DC offset).
    let n = usize::try_from(be_u32(&good[s7..s7 + 4])).unwrap();
    for cut in 5..n {
        let mut bytes = good.clone();
        bytes.drain(s7 + cut..s7 + n);
        let short = u32::try_from(cut).unwrap().to_be_bytes();
        bytes[s7..s7 + 4].copy_from_slice(&short);
        let total = u64::try_from(bytes.len()).unwrap().to_be_bytes();
        bytes[8..16].copy_from_slice(&total);
        match parse(&bytes) {
            Err(Grib2Error::Malformed { .. } | Grib2Error::Unsupported { .. }) => {}
            Ok(fields) => assert!(
                matches!(fields[0].values(), Err(Grib2Error::Malformed { reason, .. }) if reason.contains("JPEG 2000")),
                "a codestream cut to {cut} of {n} bytes decoded"
            ),
            Err(e) => panic!("{e}"),
        }
    }
}

/// The RAP fixture's first message with its codestream's main header edited at `at`.
fn with_header_byte(at: usize, value: u8) -> Vec<u8> {
    let (mut bytes, _, s7) = rap_jpeg2000();
    bytes[s7 + 5 + at] = value;
    bytes
}

#[test]
fn jpeg2000_refuses_every_other_coding_by_name() {
    let refused = |bytes: &[u8]| match parse(bytes) {
        Err(Grib2Error::Unsupported { what, value, .. }) => (what, value),
        other => panic!("{other:?}"),
    };
    // SIZ: Csiz at 40 and 41, Ssiz at 42, XRsiz and YRsiz at 43 and 44.
    assert_eq!(
        refused(&with_header_byte(41, 3)),
        ("JPEG 2000 components", 3)
    );
    assert_eq!(
        refused(&with_header_byte(42, 0x85)),
        ("JPEG 2000 signed samples", 0x85)
    );
    assert_eq!(
        refused(&with_header_byte(43, 2)),
        ("JPEG 2000 subsampling", 0x201)
    );
    // A side past the decoder's 60,000 (Xsiz, bytes 8 to 11: 60,001).
    let mut bytes = with_header_byte(9, 0);
    let (_, _, s7) = rap_jpeg2000();
    bytes[s7 + 5 + 10..s7 + 5 + 12].copy_from_slice(&[0xEA, 0x61]);
    assert_eq!(refused(&bytes), ("JPEG 2000 image side", 60_001));
    // An image offset (XOsiz, bytes 16 to 19).
    assert_eq!(
        refused(&with_header_byte(19, 1)),
        ("JPEG 2000 image or tile offset", 1)
    );
    // Tiles smaller than the image (XTsiz, bytes 24 to 27: 108 wide, tiles of 16).
    let mut bytes = with_header_byte(26, 0);
    let (_, _, s7) = rap_jpeg2000();
    bytes[s7 + 5 + 27] = 16;
    let (what, _) = refused(&bytes);
    assert_eq!(what, "JPEG 2000 tiles");
    // COD's transform (the 9/7) and QCD's quantization, where the fixture writes them.
    let (good, _, s7) = rap_jpeg2000();
    let codestream = &good[s7 + 5..];
    let find = |marker: [u8; 2]| codestream.windows(2).position(|w| w == marker).unwrap();
    let cod = find([0xFF, 0x52]);
    assert_eq!(
        refused(&with_header_byte(cod + 13, 0)),
        ("JPEG 2000 wavelet transform", 0)
    );
    assert_eq!(
        refused(&with_header_byte(cod + 4, 1)),
        ("JPEG 2000 precinct sizes", 1)
    );
    let qcd = find([0xFF, 0x5C]);
    assert_eq!(
        refused(&with_header_byte(qcd + 4, 0x42)),
        ("JPEG 2000 quantization", 2)
    );
    // A segment hidden inside a longer SIZ, COD or SOT (their lengths at 4 and 5, COD's and SOT's
    // two bytes after their markers).
    let malformed = |bytes: &[u8]| match parse(bytes) {
        Err(Grib2Error::Malformed { reason, .. }) => reason,
        other => panic!("{other:?}"),
    };
    assert!(malformed(&with_header_byte(5, 58)).contains("0xFF51 is 58 bytes long, not 41"));
    assert!(malformed(&with_header_byte(cod + 3, 20)).contains("0xFF52 is 20 bytes long, not 12"));
    let sot = find([0xFF, 0x90]);
    assert!(malformed(&with_header_byte(sot + 3, 11)).contains("0xFF90 is 11 bytes long, not 10"));
    // A marker the reader does not take (RGN in place of QCD).
    assert_eq!(
        refused(&with_header_byte(qcd + 1, 0x5E)),
        ("JPEG 2000 main-header marker", 0xFF5E)
    );
    // A tile-part's QCC, which the encoder writes for 15-bit fields, is held to the same rule.
    let file = include_bytes!("../../tests/fixtures/rap-jpeg2000.grib2");
    let first = usize::try_from(u64::from_be_bytes(file[8..16].try_into().unwrap())).unwrap();
    let second = &file[first..];
    let qcc = second
        .windows(4)
        .position(|w| w == [0xFF, 0x90, 0, 10])
        .unwrap()
        + 12;
    assert_eq!(&second[qcc..qcc + 2], &[0xFF, 0x5D]);
    let mut bytes = second.to_vec();
    bytes[qcc + 5] |= 0x02;
    assert_eq!(refused(&bytes), ("JPEG 2000 quantization", 2));
}

#[test]
fn jpeg2000_of_0_bits_is_its_reference_everywhere() {
    // The same message with 0 bits: no codestream is read, every value is `R / 10^D`.
    let (mut bytes, s5, _) = rap_jpeg2000();
    bytes[s5 + 19] = 0;
    let fields = parse(&bytes).unwrap();
    let Packing::Jpeg2000(p) = fields[0].packing else {
        panic!("{:?}", fields[0].packing)
    };
    let r = f64::from(p.reference);
    assert!(fields[0].values().unwrap().iter().all(|v| *v == Some(r)));
    assert_eq!(fields[0].value(10_151).unwrap(), Some(r));
}

proptest::proptest! {
    /// A damaged JPEG 2000 codestream is refused, when parsed or when read, and never panics.
    #[test]
    fn damaged_jpeg2000_never_panics(
        edits in proptest::collection::vec((0_usize..2_300, 0_u8..=255), 1..6),
    ) {
        let (mut bytes, _, s7) = rap_jpeg2000();
        let n = bytes.len();
        for (at, byte) in edits {
            // Within section 7's codestream, before the end marker `7777`.
            let data = s7 + 5;
            bytes[data + at % (n - 4 - data)] = byte;
        }
        if let Ok(fields) = parse(&bytes)
            && let Ok(values) = fields[0].values()
        {
            proptest::prop_assert_eq!(values.len() as u64, fields[0].points());
            proptest::prop_assert!(values.iter().all(|v| v.is_some_and(f64::is_finite)));
        }
    }
}
