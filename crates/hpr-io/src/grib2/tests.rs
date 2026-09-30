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
    b.extend_from_slice(&sm32(lo1 + 250_000 * i64::from(ni - 1)));
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
    assert_eq!(f.values(), expect);
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
        for (i, &x) in xs.iter().enumerate() {
            assert_eq!(f.packed(i as u64), x, "{bits} bits, value {i}");
        }
    }
}

#[test]
fn a_zero_bit_field_is_its_reference_everywhere() {
    let bytes = temperature(&[], 0);
    let f = &parse(&bytes).unwrap()[0];
    assert_eq!(f.values(), vec![Some(25.0); 6]);
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
    assert_eq!(f.values(), expect);
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
        fields[1].values(),
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
    // Complex packing with spatial differencing, template 5.3.
    let mut repr = packing(6, 0.0, 0, 0, 8);
    repr[10] = 3;
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
            value: 3
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
    let side = 1 << 13;
    let bytes = message(&[
        identification(),
        latlon_grid(side, side, 0, 0, 0x40, 0x30),
        product(0, 0, 100, 50_000),
        packing(side * side, 0.0, 0, 0, 0),
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

/// A message with a bitmap, then one on a Lambert grid, back to back: every section kind the
/// reader takes.
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
        edits in proptest::collection::vec((0_usize..400, 0_u8..=255), 0..6),
        cut in 0_usize..400,
    ) {
        let mut bytes = two_messages();
        for (at, byte) in edits {
            let n = bytes.len();
            bytes[at % n] = byte;
        }
        let n = bytes.len();
        bytes.truncate(n - cut % n);
        if let Ok(fields) = parse(&bytes) {
            for f in &fields {
                let values = f.values();
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
