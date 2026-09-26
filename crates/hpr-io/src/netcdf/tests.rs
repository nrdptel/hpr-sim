use super::*;
use proptest::prelude::*;
use serde_json::Value;

/// The files `validation/oracles/netcdf/write_cases.py` wrote with the Unidata netCDF library.
macro_rules! fixture {
    ($name:literal) => {
        (
            $name,
            include_bytes!(concat!(
                "../../../../validation/fixtures/weather/netcdf/",
                $name
            ))
            .as_slice(),
        )
    };
}

const FILES: &[(&str, &[u8])] = &[
    fixture!("types-classic.nc"),
    fixture!("types-offset64.nc"),
    fixture!("records-classic.nc"),
    fixture!("records-offset64.nc"),
    fixture!("one-record-short-classic.nc"),
    fixture!("one-record-short-offset64.nc"),
    fixture!("one-record-byte-classic.nc"),
    fixture!("one-record-byte-offset64.nc"),
    fixture!("no-records-classic.nc"),
    fixture!("no-records-offset64.nc"),
    fixture!("packing-classic.nc"),
    fixture!("packing-offset64.nc"),
];

/// Where hpr reads a value as missing and the Unidata library's Python interface does not, or
/// the reverse: (file case, variable, index, hpr reads it as missing). The netCDF Users Guide's
/// attribute conventions (`_FillValue`, `valid_range`) say that with no valid bounds the fill
/// bounds the valid range on its own side, and that a byte with no explicit fill has every value
/// valid; netCDF4-python 1.7.4 masks only values equal to a fill, the byte default included.
const DEPARTURES: &[(&str, &str, usize, bool)] = &[
    // -32768 lies below the fill -32767, given or the short's default.
    ("types", "h", 0, true),
    ("types", "i", 0, true),
    // 3.4e38 lies above the float's default fill, 9.97e36.
    ("types", "f", 2, true),
    ("packing", "era5_like", 0, true),
    ("packing", "default_fill", 0, true),
    // -128 and -127 lie below a byte's explicit fill, -1.
    ("packing", "byte_fill", 0, true),
    ("packing", "byte_fill", 1, true),
    // 100 lies above the positive fill 99.
    ("packing", "positive_fill", 3, true),
    // 1e37 lies above the double's default fill, 9.969e36.
    ("packing", "double_default", 2, true),
    // -127 is the byte's default fill, which the Guide leaves valid.
    ("packing", "byte_no_fill", 1, false),
];

fn reads() -> Value {
    serde_json::from_str(include_str!(
        "../../../../validation/fixtures/weather/netcdf-reads.json"
    ))
    .unwrap()
}

fn type_name(kind: Type) -> &'static str {
    match kind {
        Type::Byte => "byte",
        Type::Char => "char",
        Type::Short => "short",
        Type::Int => "int",
        Type::Float => "float",
        Type::Double => "double",
    }
}

fn numbers(values: &Values) -> Vec<f64> {
    (0..values.len()).map(|i| values.get(i).unwrap()).collect()
}

fn json_numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn check_attributes(context: &str, ours: &[Attribute], theirs: &Value) {
    let theirs = theirs.as_object().unwrap();
    assert_eq!(ours.len(), theirs.len(), "{context}: attribute count");
    for attribute in ours {
        let expected = &theirs[&attribute.name];
        let what = format!("{context}: attribute `{}`", attribute.name);
        assert_eq!(
            type_name(attribute.values.kind()),
            expected["type"],
            "{what}"
        );
        if attribute.values.kind() == Type::Char {
            assert_eq!(attribute.values.text().unwrap(), expected["text"], "{what}");
        } else {
            assert_eq!(
                numbers(&attribute.values),
                json_numbers(&expected["values"]),
                "{what}"
            );
        }
    }
}

#[test]
fn every_file_reads_as_the_unidata_library_reads_it() {
    let reads = reads();
    let files = reads["files"].as_array().unwrap();
    assert_eq!(files.len(), FILES.len());
    let mut departures_seen = 0;
    for (name, bytes) in FILES {
        let expected = files.iter().find(|f| f["file"] == *name).unwrap();
        let file = NetCdf::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let format = match file.format {
            Format::Classic => "NETCDF3_CLASSIC",
            Format::Offset64 => "NETCDF3_64BIT_OFFSET",
        };
        assert_eq!(format, expected["format"], "{name}");

        let dimensions = expected["dimensions"].as_array().unwrap();
        assert_eq!(file.dimensions.len(), dimensions.len(), "{name}");
        for (ours, theirs) in file.dimensions.iter().zip(dimensions) {
            assert_eq!(ours.name, theirs["name"], "{name}");
            assert_eq!(ours.length, theirs["length"].as_u64().unwrap(), "{name}");
            assert_eq!(ours.is_record, theirs["is_record"], "{name}");
        }
        check_attributes(name, &file.attributes, &expected["attributes"]);

        let variables = expected["variables"].as_array().unwrap();
        assert_eq!(file.variables.len(), variables.len(), "{name}");
        for (ours, theirs) in file.variables.iter().zip(variables) {
            let what = format!("{name}: `{}`", ours.name);
            assert_eq!(ours.name, theirs["name"], "{what}");
            assert_eq!(type_name(ours.values.kind()), theirs["type"], "{what}");
            let dims: Vec<&str> = theirs["dimensions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d.as_str().unwrap())
                .collect();
            assert_eq!(ours.dimensions, dims, "{what}");
            let shape: Vec<u64> = theirs["shape"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap())
                .collect();
            assert_eq!(ours.shape, shape, "{what}");
            check_attributes(&what, &ours.attributes, &theirs["attributes"]);

            let raw = json_numbers(&theirs["raw"]);
            match &ours.values {
                Values::Char(bytes) => {
                    let bytes: Vec<f64> = bytes.iter().map(|&b| f64::from(b)).collect();
                    assert_eq!(bytes, raw, "{what}");
                    continue;
                }
                values => assert_eq!(numbers(values), raw, "{what}"),
            }
            // Raw values compare bit for bit, so -0.0 and the subnormals are checked too.
            if let Values::Double(values) = &ours.values {
                for (a, b) in values.iter().zip(&raw) {
                    assert_eq!(a.to_bits(), b.to_bits(), "{what}");
                }
            }

            let packing = ours.packing().unwrap();
            let case = name.rsplit_once('-').unwrap().0;
            for (index, library) in theirs["unpacked"].as_array().unwrap().iter().enumerate() {
                let ours_value = packing.unpack(raw[index]);
                let departure = DEPARTURES
                    .iter()
                    .find(|d| d.0 == case && d.1 == ours.name && d.2 == index);
                if let Some(&(_, _, _, hpr_missing)) = departure {
                    departures_seen += 1;
                    assert_eq!(ours_value.is_none(), hpr_missing, "{what}[{index}]");
                    assert_eq!(library.is_null(), !hpr_missing, "{what}[{index}]");
                    continue;
                }
                match (ours_value, library.as_f64()) {
                    (None, None) => {}
                    (Some(a), Some(b)) => {
                        // The library unpacks in the type of the scale and offset: single
                        // precision for `float_packed`, whose attributes are floats, and double
                        // (so exactly as hpr does) everywhere else.
                        let single = ours.name == "float_packed";
                        let tolerance = if single { 1e-7 * b.abs() } else { 0.0 };
                        assert!(
                            (a - b).abs() <= tolerance,
                            "{what}[{index}]: {a} against {b}"
                        );
                        assert_eq!(
                            a,
                            raw[index] * packing.scale_factor + packing.add_offset,
                            "{what}[{index}]"
                        );
                    }
                    (a, b) => panic!("{what}[{index}]: hpr reads {a:?}, the library {b:?}"),
                }
            }
        }
    }
    // Each departure, in each of the two formats.
    assert_eq!(departures_seen, 2 * DEPARTURES.len());
}

#[test]
fn a_lone_record_variable_of_shorts_is_read_unpadded() {
    // Three shorts are six bytes a record; padded they would be eight, and every record after the
    // first would be read two bytes late.
    let (_, bytes) = FILES[4];
    let file = NetCdf::parse(bytes).unwrap();
    let h = file.variable("h").unwrap();
    assert_eq!(h.shape, [5, 3]);
    assert_eq!(h.unpacked(&[4, 1]).unwrap(), Some(-40.0));
    assert_eq!(h.unpacked(&[4, 2]).unwrap(), Some(4.0));
}

#[test]
fn lookups_find_by_name_and_index_in_row_major_order() {
    let (_, bytes) = FILES[0];
    let file = NetCdf::parse(bytes).unwrap();
    assert_eq!(file.dimension("strlen").unwrap().length, 5);
    assert!(file.dimension("nothing").is_none());
    assert_eq!(
        file.attribute("title").unwrap().values.text(),
        Some("every type")
    );
    let h = file.variable("h").unwrap();
    assert_eq!(h.offset(&[1, 2]), Some(5));
    assert_eq!(h.offset(&[2, 0]), None);
    assert_eq!(h.offset(&[1]), None);
    assert!(matches!(
        h.unpacked(&[0, 3]),
        Err(NetCdfError::Malformed { .. })
    ));
    let c = file.variable("c").unwrap();
    assert_eq!(c.values.get(0), None);
    assert!(matches!(
        c.packing(),
        Err(NetCdfError::Unsupported { ref reason, .. }) if reason.contains("character")
    ));
    assert_eq!(
        file.variable("s").unwrap().unpacked(&[]).unwrap(),
        Some(std::f64::consts::PI)
    );
}

#[test]
fn other_formats_are_refused_with_the_way_to_convert_them() {
    let hdf5 = NetCdf::parse(b"\x89HDF\r\n\x1a\n rest").unwrap_err();
    assert_eq!(hdf5, NetCdfError::NetCdf4);
    assert!(hdf5.to_string().contains("NETCDF3_64BIT"));
    let cdf5 = NetCdf::parse(b"CDF\x05\0\0\0\0").unwrap_err();
    assert_eq!(cdf5, NetCdfError::Cdf5);
    assert!(cdf5.to_string().contains("NETCDF3_64BIT"));
    assert_eq!(
        NetCdf::parse(b"GRIB").unwrap_err(),
        NetCdfError::NotNetCdf {
            head: "47524942".into()
        }
    );
}

#[test]
fn a_streaming_record_count_is_refused() {
    let (_, bytes) = FILES[2];
    let mut bytes = bytes.to_vec();
    bytes[4..8].copy_from_slice(&[0xFF; 4]);
    assert_eq!(NetCdf::parse(&bytes).unwrap_err(), NetCdfError::Streaming);
}

#[test]
fn every_truncation_is_an_error_unless_only_padding_is_lost() {
    for (name, bytes) in FILES {
        let whole = NetCdf::parse(bytes).unwrap();
        let mut readable = 0;
        for len in 0..bytes.len() {
            match NetCdf::parse(&bytes[..len]) {
                Ok(file) => {
                    // Only the last record's trailing padding can go without losing a value.
                    assert_eq!(file, whole, "{name} cut to {len}");
                    assert!(bytes.len() - len < 4, "{name} cut to {len}");
                    readable += 1;
                }
                Err(NetCdfError::Truncated { .. } | NetCdfError::NotNetCdf { .. }) => {}
                Err(error) => panic!("{name} cut to {len}: {error}"),
            }
        }
        assert!(readable < 4, "{name}");
    }
}

fn variable(kind: Values, attributes: Vec<(&str, Values)>) -> Variable {
    Variable {
        name: "v".into(),
        dimensions: vec!["n".into()],
        shape: vec![kind.len() as u64],
        attributes: attributes
            .into_iter()
            .map(|(name, values)| Attribute {
                name: name.into(),
                values,
            })
            .collect(),
        values: kind,
    }
}

#[test]
fn conventions_it_does_not_apply_are_refused() {
    let unsigned = variable(
        Values::Byte(vec![1]),
        vec![("_Unsigned", Values::Char(b"true".to_vec()))],
    );
    assert!(matches!(
        unsupported_reason(&unsigned),
        Some(r) if r.contains("_Unsigned")
    ));
    let both = variable(
        Values::Short(vec![1]),
        vec![
            ("valid_range", Values::Short(vec![0, 2])),
            ("valid_min", Values::Short(vec![0])),
        ],
    );
    assert!(matches!(unsupported_reason(&both), Some(r) if r.contains("valid_range")));
    let three = variable(
        Values::Short(vec![1]),
        vec![("valid_range", Values::Short(vec![0, 1, 2]))],
    );
    assert!(matches!(unsupported_reason(&three), Some(r) if r.contains("two numbers")));
    let two_scales = variable(
        Values::Short(vec![1]),
        vec![("scale_factor", Values::Double(vec![1.0, 2.0]))],
    );
    assert!(matches!(unsupported_reason(&two_scales), Some(r) if r.contains("scale_factor")));
    let text_fill = variable(
        Values::Short(vec![1]),
        vec![("_FillValue", Values::Char(b"x".to_vec()))],
    );
    assert!(matches!(unsupported_reason(&text_fill), Some(r) if r.contains("text")));
}

fn unsupported_reason(variable: &Variable) -> Option<String> {
    match variable.packing() {
        Err(NetCdfError::Unsupported { reason, .. }) => Some(reason),
        _ => None,
    }
}

#[test]
fn a_float_fill_bounds_the_range_two_units_in_the_last_place_away() {
    // FILL_FLOAT, `\x7C \xF0 \x00 \x00` in the specification.
    let fill = f32::from_bits(0x7CF0_0000);
    let below = f32::from_bits(fill.to_bits() - 1);
    let two_below = f32::from_bits(fill.to_bits() - 2);
    let three_below = f32::from_bits(fill.to_bits() - 3);
    let v = variable(
        Values::Float(vec![three_below, two_below, below, fill]),
        vec![],
    );
    let packing = v.packing().unwrap();
    let read: Vec<Option<f64>> = numbers(&v.values)
        .into_iter()
        .map(|x| packing.unpack(x))
        .collect();
    assert_eq!(
        read,
        [
            Some(f64::from(three_below)),
            Some(f64::from(two_below)),
            None,
            None
        ]
    );
}

/// A classic file built by hand: `dims` as (name, length; 0 for the record dimension), then
/// variables as (name, dimension ids, type tag, begin), no attributes, and `data` after the
/// header.
fn forged(
    numrecs: u32,
    dims: &[(&str, u32)],
    vars: &[(&str, &[u32], u32, u32)],
    data: &[u8],
) -> Vec<u8> {
    fn name(out: &mut Vec<u8>, name: &str) {
        out.extend((name.len() as u32).to_be_bytes());
        out.extend(name.as_bytes());
        out.resize(out.len().div_ceil(4) * 4, 0);
    }
    let mut out = b"CDF\x01".to_vec();
    out.extend(numrecs.to_be_bytes());
    out.extend(0x0Au32.to_be_bytes());
    out.extend((dims.len() as u32).to_be_bytes());
    for (n, length) in dims {
        name(&mut out, n);
        out.extend(length.to_be_bytes());
    }
    out.extend([0; 8]); // no global attributes
    if vars.is_empty() {
        out.extend([0; 8]);
    } else {
        out.extend(0x0Bu32.to_be_bytes());
        out.extend((vars.len() as u32).to_be_bytes());
    }
    for (n, ids, kind, begin) in vars {
        name(&mut out, n);
        out.extend((ids.len() as u32).to_be_bytes());
        for id in *ids {
            out.extend(id.to_be_bytes());
        }
        out.extend([0; 8]); // no attributes
        out.extend(kind.to_be_bytes());
        out.extend(4u32.to_be_bytes()); // vsize, which the reader ignores
        out.extend(begin.to_be_bytes());
    }
    out.extend(data);
    out
}

fn malformed_reason(bytes: &[u8]) -> String {
    match NetCdf::parse(bytes) {
        Err(NetCdfError::Malformed { reason }) => reason,
        other => panic!("expected a malformed header, got {other:?}"),
    }
}

#[test]
fn a_hand_built_file_reads() {
    // One int variable of two values at byte 80, just past the 80-byte header.
    let bytes = forged(
        0,
        &[("x", 2)],
        &[("v", &[0], 4, 80)],
        &[0, 0, 0, 7, 255, 255, 255, 254],
    );
    let file = NetCdf::parse(&bytes).unwrap();
    assert_eq!(file.variable("v").unwrap().values, Values::Int(vec![7, -2]));
}

#[test]
fn each_break_of_the_grammar_is_refused_for_its_reason() {
    let data = [0u8; 16];
    let cases: Vec<(Vec<u8>, &str)> = vec![
        (
            forged(1, &[("a", 0), ("b", 0)], &[], &data),
            "more than one record dimension",
        ),
        (
            forged(1, &[("x", 2), ("t", 0)], &[("v", &[0, 1], 4, 80)], &data),
            "record dimension other than first",
        ),
        (
            forged(0, &[("x", 2)], &[("v", &[0], 9, 72)], &data),
            "unknown type tag 9",
        ),
        (
            forged(0, &[("x", 2)], &[("v", &[3], 4, 72)], &data),
            "names dimension 3",
        ),
        (
            forged(0, &[("x", 2), ("x", 3)], &[], &data),
            "two dimensions are called `x`",
        ),
        (
            forged(
                0,
                &[("x", 1)],
                &[("v", &[0], 4, 72), ("v", &[0], 4, 76)],
                &data,
            ),
            "two variables are called `v`",
        ),
    ];
    for (bytes, expected) in cases {
        let reason = malformed_reason(&bytes);
        assert!(reason.contains(expected), "{reason} (expected {expected})");
    }

    // The dimension list's tag replaced by the variable list's.
    let mut bytes = forged(0, &[("x", 2)], &[], &data);
    bytes[8..12].copy_from_slice(&0x0Bu32.to_be_bytes());
    assert!(malformed_reason(&bytes).contains("dimension list has tag 0xb"));
    // A dimension length with the sign bit set.
    let mut bytes = forged(0, &[("x", 2)], &[], &data);
    bytes[24..28].copy_from_slice(&0x8000_0000u32.to_be_bytes());
    assert!(malformed_reason(&bytes).contains("a dimension length is negative"));
    // A name that is not UTF-8.
    let mut bytes = forged(0, &[("x", 2)], &[], &data);
    bytes[20] = 0xFF;
    assert!(malformed_reason(&bytes).contains("not UTF-8"));
}

#[test]
fn variables_that_share_their_bytes_are_refused() {
    // Twenty variables of 200 bytes each, all at the same offset: 4000 bytes claimed from a file
    // of about a thousand.
    let names: Vec<String> = (0..20).map(|i| format!("v{i}")).collect();
    let vars: Vec<(&str, &[u32], u32, u32)> = names
        .iter()
        .map(|n| (n.as_str(), [0u32].as_slice(), 1, 600))
        .collect();
    let bytes = forged(0, &[("x", 200)], &vars, &[0; 200]);
    assert!(bytes.len() < 2000);
    assert!(malformed_reason(&bytes).contains("more than the file's"));
}

#[test]
fn a_fill_at_the_largest_finite_value_leaves_the_rest_valid() {
    for fill in [f32::MAX, f32::MIN] {
        let v = variable(
            Values::Float(vec![1.0, fill]),
            vec![("_FillValue", Values::Float(vec![fill]))],
        );
        let packing = v.packing().unwrap();
        assert_eq!(packing.unpack(1.0), Some(1.0), "fill {fill}");
        assert_eq!(packing.unpack(f64::from(fill)), None, "fill {fill}");
    }
    for fill in [f64::MAX, f64::MIN] {
        let v = variable(
            Values::Double(vec![1.0, fill]),
            vec![("_FillValue", Values::Double(vec![fill]))],
        );
        let packing = v.packing().unwrap();
        assert_eq!(packing.unpack(1.0), Some(1.0), "fill {fill}");
        assert_eq!(packing.unpack(fill), None, "fill {fill}");
    }
}

proptest! {
    /// Whatever follows a netCDF signature, reading returns rather than panics.
    #[test]
    fn arbitrary_bytes_never_panic(
        version in prop_oneof![Just(1u8), Just(2u8)],
        body in proptest::collection::vec(any::<u8>(), 0..512),
    ) {
        let mut bytes = b"CDF".to_vec();
        bytes.push(version);
        bytes.extend(body);
        let _ = NetCdf::parse(&bytes);
    }

    /// A fixture with any one byte changed reads or is refused, never panics.
    #[test]
    fn a_changed_byte_never_panics(file in 0..FILES.len(), at in any::<usize>(), to in any::<u8>()) {
        let mut bytes = FILES[file].1.to_vec();
        let at = at % bytes.len();
        bytes[at] = to;
        let _ = NetCdf::parse(&bytes);
    }
}
