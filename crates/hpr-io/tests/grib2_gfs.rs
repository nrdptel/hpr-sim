//! Whole messages of a whole GFS file, decoded by `hpr_io::grib2` and checked against ecCodes.
//!
//! `tests/fixtures/gfs-messages.grib2` is eight messages cut unchanged from GFS's 0.25° file of
//! the 00 UTC run of 2026-09-30 at hour 18 (`gfs.t00z.pgrb2.0p25.f018`, 743 messages, 550 MB):
//! the smallest of each kind the file holds (complex packing with spatial differencing and 1, 2
//! or 3 bytes per descriptor, a bitmap, missing values, template 4.8, and its one simple-packed
//! field). `gfs-messages-eccodes.json` is ecCodes 2.49.0's reading of them, written by
//! `validation/oracles/grib2/whole_file.py cut`: each message's identity, its missing points, two
//! correctly rounded sums over every value, and every 997th value. The same script's `compare`
//! checks every value of the whole file, which is not committed (ADR-123).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use hpr_io::grib2;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// ecCodes computes `(R + X · 2^E) · 10^−D` with `10^−D` rounded, so its values and ours may
/// differ by a rounding or two: 4.4e-16 relative at most over the whole file's 746,770,303 values.
const ONE_ROUNDING: f64 = 4.5e-16;

#[derive(Deserialize)]
struct Reading {
    sha256: String,
    stride: usize,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    #[serde(rename = "message_in_file")]
    message_in_file: usize,
    kind: String,
    discipline: u8,
    parameter_category: u8,
    parameter_number: u8,
    product_definition_template_number: u16,
    data_representation_template_number: u16,
    type_of_first_fixed_surface: u8,
    forecast_time: i64,
    #[serde(rename = "Ni")]
    ni: u32,
    #[serde(rename = "Nj")]
    nj: u32,
    points: u64,
    missing: u64,
    sum: f64,
    #[serde(rename = "index_weighted_sum")]
    index_weighted_sum: f64,
    samples: Vec<Option<f64>>,
}

fn read() -> (&'static [u8], Reading) {
    let bytes = include_bytes!("fixtures/gfs-messages.grib2");
    let json = include_str!("fixtures/gfs-messages-eccodes.json");
    (bytes, serde_json::from_str(json).unwrap())
}

/// A sum with Neumaier's compensation, and the sum of the terms' sizes, which bounds how far a
/// rounding in each term can move it.
fn sum(terms: impl Iterator<Item = f64>) -> (f64, f64) {
    let (mut s, mut c, mut size) = (0.0_f64, 0.0_f64, 0.0_f64);
    for t in terms {
        let n = s + t;
        c += if s.abs() >= t.abs() {
            (s - n) + t
        } else {
            (t - n) + s
        };
        s = n;
        size += t.abs();
    }
    (s + c, size)
}

fn close(ours: f64, theirs: f64, bound: f64) -> bool {
    (ours - theirs).abs() <= bound
}

#[test]
fn whole_gfs_messages_decode_to_eccodes_values() {
    let (bytes, reading) = read();
    assert_eq!(
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        reading.sha256,
        "the fixture is the one ecCodes read"
    );
    let fields = grib2::parse(bytes).unwrap();
    assert_eq!(fields.len(), reading.messages.len());
    for (field, m) in fields.iter().zip(&reading.messages) {
        let at = format!("message {} ({})", m.message_in_file, m.kind);
        let p = field.product;
        assert_eq!(
            (
                field.discipline,
                p.category,
                p.number,
                p.template,
                field.packing.template(),
                p.surface.kind,
                p.forecast_time,
                field.grid.ni,
                field.grid.nj,
            ),
            (
                m.discipline,
                m.parameter_category,
                m.parameter_number,
                m.product_definition_template_number,
                m.data_representation_template_number,
                m.type_of_first_fixed_surface,
                m.forecast_time,
                m.ni,
                m.nj,
            ),
            "{at}"
        );
        assert_eq!(p.statistics.is_some(), p.template == 8, "{at}");
        let values = field.values().unwrap();
        assert_eq!(values.len() as u64, m.points, "{at}");
        let missing = values.iter().filter(|v| v.is_none()).count() as u64;
        assert_eq!(missing, m.missing, "{at}");
        // Every value moves each sum by at most a rounding of its term, and the compensated sum
        // adds two more; a value off by one packing step would move it far more.
        let present = || {
            values
                .iter()
                .enumerate()
                .filter_map(|(k, v)| Some((k, (*v)?)))
        };
        let (s, size) = sum(present().map(|(_, v)| v));
        assert!(
            close(s, m.sum, 3.0 * ONE_ROUNDING * size + f64::MIN_POSITIVE),
            "{at}: sum {s}, ecCodes {}",
            m.sum
        );
        let (s, size) = sum(present().map(|(k, v)| v * (k + 1) as f64));
        assert!(
            close(
                s,
                m.index_weighted_sum,
                3.0 * ONE_ROUNDING * size + f64::MIN_POSITIVE
            ),
            "{at}: weighted sum {s}, ecCodes {}",
            m.index_weighted_sum
        );
        for (n, want) in m.samples.iter().enumerate() {
            let got = values[n * reading.stride];
            match (got, want) {
                (None, None) => {}
                (Some(g), Some(w)) => assert!(
                    close(g, *w, ONE_ROUNDING * w.abs()),
                    "{at}, point {}: {g}, ecCodes {w}",
                    n * reading.stride
                ),
                _ => panic!(
                    "{at}, point {}: {got:?}, ecCodes {want:?}",
                    n * reading.stride
                ),
            }
        }
    }
}

#[test]
fn points_read_alone_or_together_match_the_whole_field() {
    let (bytes, reading) = read();
    let fields = grib2::parse(bytes).unwrap();
    // The fields with points without a value (a bitmap, or missing values), and the first.
    for (n, (field, m)) in fields.iter().zip(&reading.messages).enumerate() {
        if n > 0 && m.missing == 0 {
            continue;
        }
        let all = field.values().unwrap();
        let mut indices: Vec<u64> = (0..field.points()).step_by(reading.stride * 31).collect();
        indices.reverse();
        indices.push(field.points() - 1);
        let together = field.values_at(&indices).unwrap();
        for (&k, v) in indices.iter().zip(&together) {
            assert_eq!(*v, all[usize::try_from(k).unwrap()], "point {k}");
        }
        for &k in indices.iter().take(4) {
            assert_eq!(field.value(k).unwrap(), all[usize::try_from(k).unwrap()]);
        }
    }
}
