use proptest::prelude::*;

use super::*;

/// NCEI's coefficient file, committed unchanged.
const COF: &str = include_str!("../../data/wmm2025/WMM2025.COF");
/// NCEI's 100 high-precision test values (`WMM2025_TestValues.txt`, in `WMM2025COF.zip`).
const PRECISE: &str = include_str!("../../data/wmm2025/WMM2025_TestValues.txt");
/// The report's Table 6 (`WMM2025_TEST_VALUES.txt`): 12 values, with grid variation.
const TABLE_6: &str = include_str!("../../data/wmm2025/WMM2025_TEST_VALUES.txt");

fn rows(text: &str) -> Vec<Vec<f64>> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            line.split_whitespace()
                .map(|field| field.parse::<f64>().unwrap())
                .collect()
        })
        .collect()
}

fn at(year: f64, height_km: f64, lat_deg: f64, lon_deg: f64) -> MagneticField {
    let point = Geodetic::from_degrees(lat_deg, lon_deg, height_km * 1000.0).unwrap();
    WMM2025.field(point, year).unwrap()
}

/// Worst `|computed − printed|` over named columns, with the row it came from, less 64 units in
/// the last place of the row's total field `F`. Both sides are f64 sums of up to 90 terms as large
/// as the field, so their rounding scales with `F`, not with the component (a 30-digit evaluation
/// in review put this code's own error at up to 4.2e-10 nT, on an `X` of 255 nT); libm's `sin`
/// and `cos` also differ by a unit between platforms. The allowance is 7e-10 nT on a 50,000 nT
/// field, 700 times below the finest printed digit. A value that is not finite fails.
#[derive(Debug, Default)]
struct Worst {
    by_column: Vec<(&'static str, f64, f64)>,
    /// The current row's total field, nT.
    scale: f64,
}

impl Worst {
    fn check(&mut self, column: &'static str, computed: f64, printed: f64, row: f64) {
        assert!(
            computed.is_finite() && printed.is_finite(),
            "{column}, row {row}: {computed} against {printed}"
        );
        let error = ((computed - printed).abs() - 64.0 * f64::EPSILON * self.scale).max(0.0);
        match self
            .by_column
            .iter_mut()
            .find(|(name, _, _)| *name == column)
        {
            Some(entry) if error > entry.1 => *entry = (column, error, row),
            Some(_) => {}
            None => self.by_column.push((column, error, row)),
        }
    }

    fn get(&self, column: &str) -> f64 {
        self.by_column
            .iter()
            .find(|(name, _, _)| *name == column)
            .unwrap()
            .1
    }
}

#[test]
fn the_table_is_the_coefficient_file() {
    let mut lines = COF.lines();
    let header: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
    assert_eq!(header[0].parse::<f64>().unwrap(), WMM2025.epoch_year());
    assert_eq!(header[1], WMM2025.name());
    let mut count = 0;
    for line in lines {
        if line.starts_with("9999") {
            break;
        }
        let fields: Vec<f64> = line
            .split_whitespace()
            .map(|field| field.parse().unwrap())
            .collect();
        let (n, m) = (fields[0] as usize, fields[1] as usize);
        let row = coefficients::WMM2025[n * (n + 1) / 2 - 1 + m];
        assert_eq!(
            row.to_vec(),
            fields[2..].to_vec(),
            "coefficients of n = {n}, m = {m}"
        );
        count += 1;
    }
    assert_eq!(count, 90);
}

/// `Σₙ (a/r)ⁿ⁺² Σₘ (gₙᵐ cos mλ + hₙᵐ sin mλ) P̆ₙᵐ(sin φ′)`, the potential's sum divided by `r`, at
/// fixed `r` and `λ`: its derivative in `φ′` is `−X′` (report, eq. 10).
fn potential_sum(
    latitude_prime: f64,
    radius: f64,
    longitude: f64,
    g: &[[f64; DEGREE + 1]; DEGREE + 1],
    h: &[[f64; DEGREE + 1]; DEGREE + 1],
) -> f64 {
    let harmonics = Harmonics::new(latitude_prime, radius, longitude);
    (1..=DEGREE)
        .map(|n| {
            harmonics.radial[n]
                * (0..=n)
                    .map(|m| {
                        (g[n][m] * harmonics.cos_m[m] + h[n][m] * harmonics.sin_m[m])
                            * harmonics.p[n][m]
                    })
                    .sum::<f64>()
        })
        .sum()
}

/// `X′` as the potential's derivative in `φ′`, by a five-point difference with a step of 1e-4 rad:
/// independent of the code's Legendre derivative. Its own error is about 1e-7 nT.
fn x_prime_by_differences(
    latitude_prime: f64,
    radius: f64,
    longitude: f64,
    g: &[[f64; DEGREE + 1]; DEGREE + 1],
    h: &[[f64; DEGREE + 1]; DEGREE + 1],
) -> f64 {
    let step = 1e-4;
    let at = |k: f64| potential_sum(latitude_prime + k * step, radius, longitude, g, h);
    -(at(-2.0) - 8.0 * at(-1.0) + 8.0 * at(1.0) - at(2.0)) / (12.0 * step)
}

/// NCEI's 100 test values print the components and their rates to 1e-6 nT, and declination,
/// inclination and their rates to 0.01° and 1e-6° per year. Rows are counted from 0.
///
/// `Y`, `D`, `I` and the rates of `Y`, `Z`, `D` and `I` are reproduced to half the last printed
/// digit. `X` is not: the file's differs from ours at 97 of the 100 points, by up to 7.18e-4 nT
/// (row 35), at most 2.11e-8 of the total field; `H` and `F` follow it, and `Z` moves by up to
/// 2.2e-6 nT with it. The residue lies wholly in the geocentric `X′` (the latitude derivative):
///
/// - one residue `e` in `X′` moves `X` by `e cos(φ′ − φ)` and `Z` by `e sin(φ′ − φ)`; taking `e`
///   from `X` brings the file's `Z` to its printing;
/// - this code's `X′` is the potential's derivative, taken by differences, to 1e-6 nT at every
///   point, and the report's ten-decimal `X′` (Table 3b) to 5e-11 nT.
///
/// The rate of `X` differs too, by up to 9.5e-7 nT a year, a second, smaller residue whose cause is
/// not known; this code's `Ẋ′` is likewise the derivative of the rates' potential to 1e-6. The rates
/// of `H` and `F` follow `X` and its rate. The file is held to these measured residues, not to its
/// printing (ADR-125).
#[test]
fn ncei_high_precision_test_values() {
    let rows = rows(PRECISE);
    assert_eq!(rows.len(), 100);
    let mut worst = Worst::default();
    let (mut outside_printing, mut rate_outside_printing) = (0, 0);
    // The largest residue in X′, nT, and as a share of the total field.
    let (mut residue_max, mut share_max) = (0.0_f64, 0.0_f64);
    for (index, row) in rows.iter().enumerate() {
        let f = at(row[0], row[1], row[2], row[3]);
        let index = index as f64;
        worst.scale = row[10];
        worst.check("D", f.declination_rad.to_degrees(), row[4], index);
        worst.check("I", f.inclination_rad.to_degrees(), row[5], index);
        worst.check("H", f.horizontal_nt, row[6], index);
        worst.check("X", f.north_nt, row[7], index);
        worst.check("Y", f.east_nt, row[8], index);
        worst.check("Z", f.down_nt, row[9], index);
        worst.check("F", f.total_nt, row[10], index);
        worst.check(
            "dD",
            f.declination_rate_rad_per_year.to_degrees(),
            row[11],
            index,
        );
        worst.check(
            "dI",
            f.inclination_rate_rad_per_year.to_degrees(),
            row[12],
            index,
        );
        worst.check("dH", f.horizontal_rate_nt_per_year, row[13], index);
        worst.check("dX", f.north_rate_nt_per_year, row[14], index);
        worst.check("dY", f.east_rate_nt_per_year, row[15], index);
        worst.check("dZ", f.down_rate_nt_per_year, row[16], index);
        worst.check("dF", f.total_rate_nt_per_year, row[17], index);
        if (f.north_nt - row[7]).abs() > 5e-7 {
            outside_printing += 1;
        }
        if (f.north_rate_nt_per_year - row[14]).abs() > 5e-7 {
            rate_outside_printing += 1;
        }
        // The file's rates of H and F follow from its own X, Y, Z and their rates (eq. 20).
        let (x, y, z, h, total) = (row[7], row[8], row[9], row[6], row[10]);
        let (x_dot, y_dot, z_dot) = (row[14], row[15], row[16]);
        worst.check(
            "dH from the file",
            (x * x_dot + y * y_dot) / h,
            row[13],
            index,
        );
        worst.check(
            "dF from the file",
            (x * x_dot + y * y_dot + z * z_dot) / total,
            row[17],
            index,
        );

        // One residue `e` in X′ moves X by e cos(φ′ − φ) and Z by e sin(φ′ − φ).
        let point = Geodetic::from_degrees(row[2], row[3], row[1] * 1000.0).unwrap();
        let (latitude_prime, radius) = geocentric(point);
        let (sin_turn, cos_turn) = (latitude_prime - point.latitude_rad).sin_cos();
        let residue = (row[7] - f.north_nt) / cos_turn;
        residue_max = residue_max.max(residue.abs());
        share_max = share_max.max((residue / row[10]).abs());
        worst.check(
            "Z after X' residue",
            f.down_nt + residue * sin_turn,
            row[9],
            index,
        );

        // This code's X′ and Ẋ′ against the potential's derivative, taken by differences.
        let longitude = point.longitude_rad.rem_euclid(std::f64::consts::TAU);
        let (g, h, g_dot, h_dot) = WMM2025.coefficients_at(row[0]);
        let harmonics = Harmonics::new(latitude_prime, radius, longitude);
        worst.check(
            "X' by differences",
            harmonics.sum(&g, &h).north,
            x_prime_by_differences(latitude_prime, radius, longitude, &g, &h),
            index,
        );
        worst.check(
            "dX' by differences",
            harmonics.sum(&g_dot, &h_dot).north,
            x_prime_by_differences(latitude_prime, radius, longitude, &g_dot, &h_dot),
            index,
        );
    }
    println!(
        "{worst:?}, X outside its printing at {outside_printing} points, its rate at \
         {rate_outside_printing}, residue in X' {residue_max:e} nT, {share_max:e} of F"
    );
    assert_eq!((outside_printing, rate_outside_printing), (97, 24));
    // Products of values each rounded to 1e-6: within 1e-6 of the printed rates.
    for column in ["dH from the file", "dF from the file"] {
        assert!(worst.get(column) <= 1e-6, "{column}: {worst:?}");
    }
    for column in ["D", "I"] {
        assert!(worst.get(column) <= 0.005, "{column}: {worst:?}");
    }
    for column in ["Y", "dD", "dI", "dY", "dZ", "Z after X' residue"] {
        assert!(worst.get(column) <= 5e-7, "{column}: {worst:?}");
    }
    for column in ["X' by differences", "dX' by differences"] {
        assert!(worst.get(column) <= 1e-6, "{column}: {worst:?}");
    }
    // The measured residue, 7.18e-4 nT at row 35, and what it moves.
    assert!(residue_max <= 7.2e-4, "{residue_max}");
    assert!(share_max <= 2.11e-8, "{share_max}");
    for column in ["X", "H", "F"] {
        assert!(worst.get(column) <= 7.2e-4, "{column}: {worst:?}");
    }
    assert!(worst.get("Z") <= 2.2e-6, "Z: {worst:?}");
    for column in ["dX", "dH", "dF"] {
        assert!(worst.get(column) <= 1.5e-6, "{column}: {worst:?}");
    }
}

/// The report's Table 6: components to 0.1 nT, angles to 0.01°, rates to 0.1 nT and 0.01° a year,
/// and grid variation where the latitude is poleward of 55° (`NaN` in the table elsewhere).
#[test]
fn report_table_6_test_values() {
    let rows = rows(TABLE_6);
    assert_eq!(rows.len(), 12);
    let mut worst = Worst::default();
    for (index, row) in rows.iter().enumerate() {
        let f = at(row[0], row[1], row[2], row[3]);
        let index = index as f64;
        worst.scale = row[8];
        worst.check("X", f.north_nt, row[4], index);
        worst.check("Y", f.east_nt, row[5], index);
        worst.check("Z", f.down_nt, row[6], index);
        worst.check("H", f.horizontal_nt, row[7], index);
        worst.check("F", f.total_nt, row[8], index);
        worst.check("I", f.inclination_rad.to_degrees(), row[9], index);
        worst.check("D", f.declination_rad.to_degrees(), row[10], index);
        match (f.grid_variation_rad, row[11].is_nan()) {
            (Some(gv), false) => worst.check("GV", gv.to_degrees(), row[11], index),
            (None, true) => {}
            (gv, _) => panic!("row {index}: grid variation {gv:?} against {}", row[11]),
        }
        worst.check("Xdot", f.north_rate_nt_per_year, row[12], index);
        worst.check("Ydot", f.east_rate_nt_per_year, row[13], index);
        worst.check("Zdot", f.down_rate_nt_per_year, row[14], index);
        worst.check("Hdot", f.horizontal_rate_nt_per_year, row[15], index);
        worst.check("Fdot", f.total_rate_nt_per_year, row[16], index);
        worst.check(
            "Idot",
            f.inclination_rate_rad_per_year.to_degrees(),
            row[17],
            index,
        );
        worst.check(
            "Ddot",
            f.declination_rate_rad_per_year.to_degrees(),
            row[18],
            index,
        );
    }
    println!("{worst:?}");
    for column in [
        "X", "Y", "Z", "H", "F", "Xdot", "Ydot", "Zdot", "Hdot", "Fdot",
    ] {
        assert!(worst.get(column) <= 0.05, "{column}: {worst:?}");
    }
    for column in ["I", "D", "GV", "Idot", "Ddot"] {
        assert!(worst.get(column) <= 0.005, "{column}: {worst:?}");
    }
}

/// The report's Table 3b: the intermediate values for 2027.5, 100 km, 80° S, 240° E, printed to
/// ten decimals.
#[test]
#[expect(
    clippy::excessive_precision,
    reason = "the report's values, kept to the digits it prints"
)]
fn report_table_3b_intermediate_values() {
    let point = Geodetic::from_degrees(-80.0, 240.0, 100_000.0).unwrap();
    let (latitude_prime, radius) = geocentric(point);
    let close = |what: &str, computed: f64, printed: f64, bound: f64| {
        // Both sides are f64 computations, each a few units in the last place from exact: allow
        // eight of them on top of the printed rounding.
        let bound = bound + 8.0 * f64::EPSILON * printed.abs();
        let error = (computed - printed).abs();
        assert!(error <= bound, "{what}: {computed} against {printed}");
    };
    // Half the last printed digit, or two units in the last place where that is larger (r, about
    // 6.5e6 m, is held to 2.9e-9 m).
    close("phi'", latitude_prime, -1.395_128_958_9, 5e-11);
    close(
        "r",
        radius,
        6_457_402.348_447_370_5,
        2.0 * f64::EPSILON * radius,
    );

    let (g, h, _, _) = WMM2025.coefficients_at(2027.5);
    close("g(1,0)", g[1][0], -29_321.8, 5e-11);
    close("g(1,1)", g[1][1], -1_386.55, 5e-11);
    close("g(2,0)", g[2][0], -2_585.6, 5e-11);
    close("g(2,1)", g[2][1], 2_938.1, 5e-11);
    close("g(2,2)", g[2][2], 1_629.3, 5e-11);
    close("h(1,1)", h[1][1], 4_491.65, 5e-11);
    close("h(2,1)", h[2][1], -3_202.85, 5e-11);
    close("h(2,2)", h[2][2], -845.35, 5e-11);

    let harmonics = Harmonics::new(latitude_prime, radius, point.longitude_rad);
    let prime = harmonics.sum(&g, &h);
    close("X'", prime.north, 5_928.024_139_258_8, 5e-11);
    close("Y'", prime.east, 14_760.135_975_786_8, 5e-11);
    close("Z'", prime.down, -49_324.427_357_028_4, 5e-11);

    let f = WMM2025.field(point, 2027.5).unwrap();
    close("X", f.north_nt, 5_983.976_049_651_8, 5e-11);
    close("Y", f.east_nt, 14_760.135_975_786_8, 5e-11);
    close("Z", f.down_nt, -49_317.670_615_425_5, 5e-11);
    close("Xdot", f.north_rate_nt_per_year, 30.555_353_353_0, 5e-11);
    close("Ydot", f.east_rate_nt_per_year, -8.049_422_899_5, 5e-11);
    close("Zdot", f.down_rate_nt_per_year, 89.217_406_938_2, 5e-11);
    close("F", f.total_nt, 51_825.690_717_231_4, 5e-11);
    close("H", f.horizontal_nt, 15_927.007_986_013_0, 5e-11);
    close("D", f.declination_rad, 1.185_630_840_7, 5e-11);
    close("I", f.inclination_rad, -1.258_422_154_1, 5e-11);
    close("Fdot", f.total_rate_nt_per_year, -83.664_350_680_2, 5e-11);
    close(
        "Hdot",
        f.horizontal_rate_nt_per_year,
        4.020_336_160_1,
        5e-11,
    );
    close(
        "Ddot",
        f.declination_rate_rad_per_year,
        -0.001_967_791_0,
        5e-11,
    );
    close(
        "Idot",
        f.inclination_rate_rad_per_year,
        0.000_602_866_3,
        5e-11,
    );
}

/// The report's section 1.4: on 2025-01-01, 6,371,200 m from the Earth's centre over each pole,
/// with the pole given longitude 0°, the WMM's components are (1709.5, 417.8, 56517.5) nT in the
/// north and (14185.8, −8732.7, −51366.5) nT in the south, printed to 0.1 nT.
#[test]
fn report_pole_values() {
    let b = Ellipsoid::WGS84.semi_minor_axis_m();
    for (latitude, printed) in [
        (90.0, [1_709.5, 417.8, 56_517.5]),
        (-90.0, [14_185.8, -8_732.7, -51_366.5]),
    ] {
        let point = Geodetic::from_degrees(latitude, 0.0, REFERENCE_RADIUS_M - b).unwrap();
        let f = WMM2025.field(point, 2025.0).unwrap();
        for (computed, printed) in [f.north_nt, f.east_nt, f.down_nt].into_iter().zip(printed) {
            assert!(
                (computed - printed).abs() <= 0.05,
                "{latitude}°: {computed} against {printed}"
            );
        }
    }
}

/// North of 55° the grid variation is `D − λ`: at NCEI's 89° N, 121° W point, whose declination is
/// printed as −99.77°, it is 21.23°. (Table 6's northern points are all at λ = 0.)
#[test]
fn northern_grid_variation_subtracts_the_longitude() {
    let f = at(2025.0, 28.0, 89.0, -121.0);
    let gv = f.grid_variation_rad.unwrap().to_degrees();
    assert!((gv - 21.23).abs() <= 0.005, "{gv}");
}

/// Any finite longitude gives the field of that longitude reduced to a turn, and NaN anywhere
/// in the field puts it in the blackout zone.
#[test]
fn huge_longitudes_and_nan_fail_safe() {
    let reduced = 1e308_f64.rem_euclid(std::f64::consts::TAU);
    let huge = WMM2025
        .field(
            Geodetic::new(70_f64.to_radians(), 1e308, 0.0).unwrap(),
            2026.0,
        )
        .unwrap();
    let plain = WMM2025
        .field(
            Geodetic::new(70_f64.to_radians(), reduced, 0.0).unwrap(),
            2026.0,
        )
        .unwrap();
    assert_eq!(huge, plain);
    assert!(huge.declination_rad.is_finite());
    let gv = huge.grid_variation_rad.unwrap();
    assert!(gv > -std::f64::consts::PI && gv <= std::f64::consts::PI);
    for angle in [-1e18, 1e17, 1e300] {
        let wrapped = wrap_pi(angle);
        assert!(wrapped > -std::f64::consts::PI && wrapped <= std::f64::consts::PI);
    }
    let nan = MagneticField {
        horizontal_nt: f64::NAN,
        ..plain
    };
    assert_eq!(nan.compass_zone(), CompassZone::Blackout);
}

/// The model's surface minimum of `F` lies below the report's Table 1 floor of 23,000 nT: about
/// 21,900 nT over South America by 2030.
#[test]
fn the_surface_field_dips_below_table_1() {
    let f = at(2030.0, 0.0, -26.0, -61.0);
    assert!((21_900.0..22_000.0).contains(&f.total_nt), "{}", f.total_nt);
}

#[test]
fn enu_components_are_the_launch_frames() {
    let f = at(2026.0, 1.4, 32.99, -106.97);
    assert_eq!(f.enu_nt(), DVec3::new(f.east_nt, f.north_nt, -f.down_nt));
    assert!(f.enu_nt().z < 0.0, "the field points down in the north");
}

/// The field at a pole is the limit of the field approaching it along the same meridian.
#[test]
fn the_field_is_continuous_at_the_poles() {
    for (pole, near) in [(90.0, 90.0 - 1e-9), (-90.0, -90.0 + 1e-9)] {
        let at_pole = at(2027.0, 0.0, pole, 30.0);
        let beside = at(2027.0, 0.0, near, 30.0);
        for (a, b) in [
            (at_pole.north_nt, beside.north_nt),
            (at_pole.east_nt, beside.east_nt),
            (at_pole.down_nt, beside.down_nt),
            (at_pole.east_rate_nt_per_year, beside.east_rate_nt_per_year),
        ] {
            assert!((a - b).abs() < 1e-6, "{pole}: {a} against {b}");
        }
    }
}

/// The report's equation 6 prints `P₃,ₘ`; equation 5 normalizes them, and equation 16 gives their
/// derivative away from the poles.
#[test]
fn legendre_functions_match_the_reports_forms() {
    for latitude_deg in [-71.0_f64, -12.5, 0.0, 33.0, 64.0] {
        let phi = latitude_deg.to_radians();
        let (mu, s) = phi.sin_cos();
        let harmonics = Harmonics::new(phi, REFERENCE_RADIUS_M, 0.0);
        let unnormalized = [
            0.5 * mu * (5.0 * mu * mu - 3.0),
            -1.5 * s * (1.0 - 5.0 * mu * mu),
            15.0 * mu * (1.0 - mu * mu),
            15.0 * s * s * s,
        ];
        let norms = [
            1.0,
            (2.0_f64 / 12.0).sqrt(),
            (2.0_f64 / 120.0).sqrt(),
            (2.0_f64 / 720.0).sqrt(),
        ];
        for m in 0..4 {
            let expected = norms[m] * unnormalized[m];
            assert!(
                (harmonics.p[3][m] - expected).abs() < 1e-14,
                "P(3,{m}) at {latitude_deg}°"
            );
        }
        for n in 1..DEGREE {
            for m in 0..=n {
                let k = (((n + 1) * (n + 1) - m * m) as f64).sqrt();
                // P̆ₙ₊₁ᵐ in equation 16 is the Schmidt function of degree n + 1, whose norm
                // differs from degree n's; this takes it from the table rather than rescaling.
                let expected =
                    (n + 1) as f64 * phi.tan() * harmonics.p[n][m] - k / s * harmonics.p[n + 1][m];
                assert!(
                    (harmonics.dp[n][m] - expected).abs() < 1e-11,
                    "dP({n},{m}) at {latitude_deg}°: {} against {expected}",
                    harmonics.dp[n][m]
                );
            }
        }
    }
}

/// Every `dP̆ₙᵐ/dφ′` to degree 12 is the derivative of `P̆ₙᵐ(sin φ′)`, taken by a five-point
/// difference, and `m P̆ₙᵐ / cos φ′` is that quotient, away from the poles.
#[test]
fn legendre_derivatives_match_differences() {
    let step = 1e-4;
    for latitude_deg in [-89.5_f64, -60.0, -12.5, 0.0, 33.0, 71.0, 89.9] {
        let phi = latitude_deg.to_radians();
        let at = |k: f64| Harmonics::new(phi + k * step, REFERENCE_RADIUS_M, 0.0);
        let (m2, m1, p1, p2) = (at(-2.0), at(-1.0), at(1.0), at(2.0));
        let here = at(0.0);
        for n in 1..=DEGREE {
            for m in 0..=n {
                let by_differences =
                    (m2.p[n][m] - 8.0 * m1.p[n][m] + 8.0 * p1.p[n][m] - p2.p[n][m]) / (12.0 * step);
                assert!(
                    (here.dp[n][m] - by_differences).abs() < 1e-8,
                    "dP({n},{m}) at {latitude_deg}°: {} against {by_differences}",
                    here.dp[n][m]
                );
                let quotient = m as f64 * here.p[n][m] / phi.cos();
                assert!(
                    (here.p_over_cos[n][m] - quotient).abs() < 1e-9 * quotient.abs().max(1.0),
                    "mP/cos({n},{m}) at {latitude_deg}°"
                );
            }
        }
    }
}

#[test]
fn refuses_times_and_heights_outside_the_model() {
    let site = Geodetic::from_degrees(40.0, -105.0, 1_600.0).unwrap();
    for year in [2_024.999, 2_030.000_1, f64::NAN, f64::INFINITY] {
        let error = WMM2025.field(site, year).unwrap_err();
        assert!(
            matches!(
                error,
                CoreError::Domain {
                    what: "decimal year for the magnetic model",
                    ..
                }
            ),
            "{year}: {error}"
        );
    }
    for year in [2025.0, 2030.0] {
        assert!(WMM2025.field(site, year).is_ok());
    }
    for height in [-1_000.001, 850_000.001] {
        let point = Geodetic::from_degrees(40.0, -105.0, height).unwrap();
        let error = WMM2025.field(point, 2026.0).unwrap_err();
        assert!(
            matches!(
                error,
                CoreError::Domain { what: "ellipsoidal height for the magnetic model (m)", value }
                    if value == height
            ),
            "{height}: {error}"
        );
    }
    for height in [MIN_HEIGHT_M, MAX_HEIGHT_M] {
        let point = Geodetic::from_degrees(40.0, -105.0, height).unwrap();
        assert!(WMM2025.field(point, 2026.0).is_ok());
    }
    let unchecked = Geodetic {
        latitude_rad: 2.0,
        longitude_rad: 0.0,
        height_m: 0.0,
    };
    assert!(matches!(
        WMM2025.field(unchecked, 2026.0),
        Err(CoreError::Domain {
            what: "geodetic latitude (rad)",
            ..
        })
    ));
}

#[test]
fn compass_zones_follow_the_horizontal_intensity() {
    // From NCEI's test values: H = 1504.3 nT at 89° N, 121° W, and 2164.3 nT at 80° N, 96° W.
    assert_eq!(
        at(2025.0, 28.0, 89.0, -121.0).compass_zone(),
        CompassZone::Blackout
    );
    assert_eq!(
        at(2025.0, 48.0, 80.0, -96.0).compass_zone(),
        CompassZone::Caution
    );
    assert_eq!(
        at(2025.0, 0.0, 40.0, -105.0).compass_zone(),
        CompassZone::Reliable
    );
}

/// Equation 43 gives 0.29° where `H` is 41,875 nT, the strongest horizontal field at the surface
/// (report, section 3.4), and grows as `5417 / H` toward a magnetic pole.
#[test]
fn declination_uncertainty_follows_equation_43() {
    let with = |horizontal_nt: f64| MagneticField {
        horizontal_nt,
        ..at(2026.0, 0.0, 0.0, 0.0)
    };
    let strongest = with(41_875.0).declination_uncertainty_rad().to_degrees();
    assert!((strongest - 0.29).abs() < 0.005, "{strongest}");
    // Near a pole it behaves like 5417 nT / H (report, section 3.4, point 2).
    let weak = with(100.0).declination_uncertainty_rad().to_degrees();
    assert!((weak / (5_417.0 / 100.0) - 1.0).abs() < 1e-4, "{weak}");
}

#[test]
fn a_magnetic_bearing_turns_by_the_declination() {
    let f = at(2026.0, 1.6, 40.0, -105.0);
    assert!(f.declination_rad > 0.0);
    assert_eq!(f.true_from_magnetic_rad(0.0), f.declination_rad);
    assert_eq!(f.true_from_magnetic_rad(1.0), 1.0 + f.declination_rad);
    // A bearing past north wraps into [0, 2π).
    let tau = std::f64::consts::TAU;
    let wrapped = f.true_from_magnetic_rad(tau - 0.01);
    assert!(
        (wrapped - (f.declination_rad - 0.01)).abs() < 1e-15,
        "{wrapped}"
    );
}

#[test]
fn decimal_years_count_whole_days_from_january_first() {
    assert_eq!(decimal_year(2025, 1, 1).unwrap(), 2025.0);
    assert_eq!(decimal_year(2027, 7, 2).unwrap(), 2027.0 + 182.0 / 365.0);
    assert_eq!(decimal_year(2028, 12, 31).unwrap(), 2028.0 + 365.0 / 366.0);
    assert_eq!(decimal_year(2028, 2, 29).unwrap(), 2028.0 + 59.0 / 366.0);
    assert_eq!(decimal_year(2000, 3, 1).unwrap(), 2000.0 + 60.0 / 366.0);
    assert_eq!(decimal_year(2100, 3, 1).unwrap(), 2100.0 + 59.0 / 365.0);
    for (month, day, what, value) in [
        (0, 1, "month", 0.0),
        (13, 1, "month", 13.0),
        (2, 29, "day of the month", 29.0),
        (4, 31, "day of the month", 31.0),
        (1, 0, "day of the month", 0.0),
    ] {
        let error = decimal_year(2025, month, day).unwrap_err();
        assert!(
            matches!(error, CoreError::Domain { what: w, value: v } if w == what && v == value),
            "{month}-{day}: {error}"
        );
    }
}

#[test]
fn grid_variation_wraps_into_a_half_turn() {
    assert_eq!(wrap_pi(0.0), 0.0);
    assert_eq!(wrap_pi(std::f64::consts::PI), std::f64::consts::PI);
    assert_eq!(wrap_pi(-std::f64::consts::PI), std::f64::consts::PI);
    assert!((wrap_pi(308.78_f64.to_radians()).to_degrees() + 51.22).abs() < 1e-12);
}

proptest! {
    /// Anywhere at the surface the elements obey their definitions (report, equations 19 and
    /// 20), the angles stay in their ranges, and the intensity is the Earth's, 20 to 70 µT. (The
    /// report's Table 1 rounds the surface range to 23,000 to 67,000 nT; the WMM itself reaches
    /// about 21,900 nT over South America by 2030.)
    #[test]
    fn surface_elements_obey_their_definitions(
        lat in -90.0_f64..=90.0,
        lon in -180.0_f64..=180.0,
        year in 2025.0_f64..=2030.0,
    ) {
        let f = at(year, 0.0, lat, lon);
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b.abs().max(1.0);
        prop_assert!(close(f.horizontal_nt, f.north_nt.hypot(f.east_nt)));
        prop_assert!(close(f.total_nt, f.horizontal_nt.hypot(f.down_nt)));
        prop_assert!(close(f.declination_rad, f.east_nt.atan2(f.north_nt)));
        prop_assert!(close(f.inclination_rad, f.down_nt.atan2(f.horizontal_nt)));
        prop_assert!(f.inclination_rad.abs() <= std::f64::consts::FRAC_PI_2);
        prop_assert!(f.declination_rad.abs() <= std::f64::consts::PI);
        prop_assert!((20_000.0..=70_000.0).contains(&f.total_nt), "F = {}", f.total_nt);
        prop_assert_eq!(f.grid_variation_rad.is_some(), lat.abs() > 55.0);
    }
}
