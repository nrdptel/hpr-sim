use std::io::Cursor;

use proptest::prelude::*;
use tiff::encoder::{TiffEncoder, colortype};

use super::*;

/// The keys of a geographic WGS 84 file in degrees, pixel is area: (key, value).
const WGS84: [(u16, u16); 3] = [(1024, 2), (1025, 1), (2048, 4326)];

/// How a synthetic file is georeferenced.
enum Georef {
    /// `ModelPixelScaleTag` and `ModelTiepointTag`.
    Tiepoint {
        scale: [f64; 3],
        tie: Vec<f64>,
    },
    /// `ModelTransformationTag`.
    Matrix([f64; 16]),
    None,
}

fn tie(lon: f64, lat: f64, dlon: f64, dlat: f64) -> Georef {
    Georef::Tiepoint {
        scale: [dlon, dlat, 0.0],
        tie: vec![0.0, 0.0, 0.0, lon, lat, 0.0],
    }
}

/// A GeoKey directory: version 1, revision 1.1, then SHORT keys stored in the directory.
fn directory(keys: &[(u16, u16)]) -> Vec<u16> {
    let mut dir = vec![1, 1, 1, u16::try_from(keys.len()).unwrap()];
    for &(key, value) in keys {
        dir.extend([key, 0, 1, value]);
    }
    dir
}

/// A one-band int32 GeoTIFF in strips of `rows_per_strip`.
fn geotiff_i32(
    width: u32,
    height: u32,
    rows_per_strip: u32,
    dir: &[u16],
    georef: &Georef,
    nodata: Option<&str>,
    data: &[i32],
) -> Vec<u8> {
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let mut image = tiff.new_image::<colortype::GrayI32>(width, height).unwrap();
    image.rows_per_strip(rows_per_strip).unwrap();
    let encoder = image.encoder();
    if !dir.is_empty() {
        encoder.write_tag(Tag::GeoKeyDirectoryTag, dir).unwrap();
    }
    match georef {
        Georef::Tiepoint { scale, tie } => {
            encoder
                .write_tag(Tag::ModelPixelScaleTag, &scale[..])
                .unwrap();
            encoder
                .write_tag(Tag::ModelTiepointTag, tie.as_slice())
                .unwrap();
        }
        Georef::Matrix(m) => {
            encoder
                .write_tag(Tag::ModelTransformationTag, &m[..])
                .unwrap();
        }
        Georef::None => {}
    }
    if let Some(text) = nodata {
        encoder
            .write_tag(Tag::Unknown(GDAL_NODATA_TAG), text)
            .unwrap();
    }
    image.write_data(data).unwrap();
    buf.into_inner()
}

/// A 4 by 3 raster, value `10·row + col`, its corner at 10° E, 20° N, pixels 0.5° by 0.25°.
fn small(dir: &[u16], georef: &Georef, nodata: Option<&str>) -> Vec<u8> {
    let data: Vec<i32> = (0..3)
        .flat_map(|r| (0..4).map(move |c| 10 * r + c))
        .collect();
    geotiff_i32(4, 3, 2, dir, georef, nodata, &data)
}

fn small_wgs84() -> Vec<u8> {
    small(&directory(&WGS84), &tie(10.0, 20.0, 0.5, 0.25), None)
}

#[test]
fn a_pixel_holds_the_points_of_its_area() {
    let bytes = small_wgs84();
    let raster = ElevationRaster::parse(&bytes).unwrap();
    let info = raster.info();
    assert_eq!(
        (
            info.corner_longitude_deg,
            info.corner_latitude_deg,
            info.pixel_longitude_deg,
            info.pixel_latitude_deg
        ),
        (10.0, 20.0, 0.5, -0.25)
    );
    assert_eq!(
        info.bounds(),
        Bounds {
            south_deg: 19.25,
            north_deg: 20.0,
            west_deg: 10.0,
            east_deg: 12.0
        }
    );
    // Pixel (row 1, column 2) spans 10.0..10.5 + 2·0.5 in longitude, 19.75..19.5 in latitude.
    assert_eq!(raster.value_at(19.6, 11.2).unwrap(), Some(12.0));
    // A west and a north edge belong to the pixel (the floor of an exact quotient).
    assert_eq!(raster.value_at(19.75, 11.0).unwrap(), Some(12.0));
    assert_eq!(raster.value_at(20.0, 10.0).unwrap(), Some(0.0));
    // The east and south edges of the raster are off it.
    assert!(matches!(
        raster.value_at(19.5, 12.0),
        Err(GeoTiffError::Outside { .. })
    ));
    assert!(matches!(
        raster.value_at(19.25, 11.0),
        Err(GeoTiffError::Outside { .. })
    ));
    assert_eq!(raster.pixel(Pixel { row: 2, col: 3 }).unwrap(), Some(23.0));
    assert_eq!(
        raster.pixel(Pixel { row: 3, col: 0 }),
        Err(GeoTiffError::NoSuchPixel {
            pixel: Pixel { row: 3, col: 0 },
            width: 4,
            height: 3
        })
    );
    assert_eq!(
        raster.values().unwrap(),
        [
            0.0, 1.0, 2.0, 3.0, 10.0, 11.0, 12.0, 13.0, 20.0, 21.0, 22.0, 23.0
        ]
    );
}

#[test]
fn an_outside_point_names_the_raster_s_edges() {
    let bytes = small_wgs84();
    let raster = ElevationRaster::parse(&bytes).unwrap();
    let e = raster.value_at(30.0, 11.0).unwrap_err();
    assert_eq!(
        e,
        GeoTiffError::Outside {
            latitude_deg: 30.0,
            longitude_deg: 11.0,
            bounds: Bounds {
                south_deg: 19.25,
                north_deg: 20.0,
                west_deg: 10.0,
                east_deg: 12.0
            }
        }
    );
    assert!(
        e.to_string().contains("latitudes 19.250000° to 20.000000°"),
        "{e}"
    );
}

#[test]
fn a_latitude_or_longitude_that_is_not_a_place_is_refused() {
    let bytes = small_wgs84();
    let raster = ElevationRaster::parse(&bytes).unwrap();
    for (lat, lon) in [(90.5, 11.0), (f64::NAN, 11.0), (19.6, f64::INFINITY)] {
        assert!(
            matches!(
                raster.height_at(lat, lon),
                Err(GeoTiffError::Location { .. })
            ),
            "({lat}, {lon})"
        );
    }
}

#[test]
fn pixel_is_point_moves_the_corner_back_half_a_pixel() {
    let keys = [(1024, 2), (1025, 2), (2048, 4326)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    let raster = ElevationRaster::parse(&bytes).unwrap();
    let info = raster.info();
    assert_eq!(info.raster_type, RasterType::PixelIsPoint);
    assert_eq!(
        (info.corner_longitude_deg, info.corner_latitude_deg),
        (9.75, 20.125)
    );
    // The tiepoint is pixel (0, 0)'s centre, so a point a little west and north of it is in it.
    assert_eq!(raster.value_at(20.1, 9.8).unwrap(), Some(0.0));
    assert_eq!(raster.value_at(19.9, 10.3).unwrap(), Some(1.0));
    assert_eq!(raster.value_at(19.8, 10.3).unwrap(), Some(11.0));
}

#[test]
fn a_tiepoint_off_the_first_pixel_is_carried_to_the_corner() {
    // Raster (2, 1) at 11° E, 19.75° N: the corner is 2 pixels west and 1 north of it.
    let georef = Georef::Tiepoint {
        scale: [0.5, 0.25, 0.0],
        tie: vec![2.0, 1.0, 0.0, 11.0, 19.75, 0.0],
    };
    let bytes = small(&directory(&WGS84), &georef, None);
    let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
    assert_eq!(
        (info.corner_longitude_deg, info.corner_latitude_deg),
        (10.0, 20.0)
    );
}

#[test]
fn a_transformation_matrix_without_rotation_reads_as_the_tiepoint_does() {
    let m = [
        0.5, 0.0, 0.0, 10.0, 0.0, -0.25, 0.0, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let bytes = small(&directory(&WGS84), &Georef::Matrix(m), None);
    let matrix = ElevationRaster::parse(&bytes).unwrap();
    let tiepoint = small_wgs84();
    let tiepoint = ElevationRaster::parse(&tiepoint).unwrap();
    assert_eq!(matrix.info(), tiepoint.info());
    assert_eq!(matrix.value_at(19.6, 11.2).unwrap(), Some(12.0));
}

#[test]
fn a_rotated_raster_is_refused() {
    let m = [
        0.5, 0.01, 0.0, 10.0, 0.0, -0.25, 0.0, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let bytes = small(&directory(&WGS84), &Georef::Matrix(m), None);
    let e = ElevationRaster::parse(&bytes).unwrap_err();
    assert!(
        matches!(e, GeoTiffError::Unsupported { what, ref value, .. }
            if what.starts_with("a rotated") && value == "0.01, 0"),
        "{e}"
    );
}

#[test]
fn ground_control_points_are_refused() {
    let georef = Georef::Tiepoint {
        scale: [0.5, 0.25, 0.0],
        tie: vec![
            0.0, 0.0, 0.0, 10.0, 20.0, 0.0, 3.0, 2.0, 0.0, 11.5, 19.5, 0.0,
        ],
    };
    let bytes = small(&directory(&WGS84), &georef, None);
    let e = ElevationRaster::parse(&bytes).unwrap_err();
    assert!(
        matches!(e, GeoTiffError::Unsupported { what, ref value, .. }
            if what.starts_with("ModelTiepointTag") && value == "12"),
        "{e}"
    );
}

#[test]
fn missing_georeferencing_is_named() {
    let no_keys = small(&[], &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&no_keys),
        Err(GeoTiffError::Missing { what }) if what.starts_with("GeoKeyDirectoryTag")
    ));
    let no_scale = small(&directory(&WGS84), &Georef::None, None);
    assert!(matches!(
        ElevationRaster::parse(&no_scale),
        Err(GeoTiffError::Missing { what }) if what.starts_with("ModelPixelScaleTag")
    ));
    let no_crs = small(&directory(&[(1025, 1)]), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&no_crs),
        Err(GeoTiffError::Missing { what }) if what.starts_with("GTModelTypeGeoKey")
    ));
}

#[test]
fn a_projected_file_is_refused_naming_its_crs() {
    let e = ElevationRaster::parse(include_bytes!(
        "../../tests/fixtures/geotiff/utm13n-refused.tif"
    ))
    .unwrap_err();
    assert!(
        matches!(e, GeoTiffError::Unsupported { what, ref value, .. }
            if what == "a projected CRS, EPSG" && value == "32613"),
        "{e}"
    );
    assert!(e.to_string().contains("gdalwarp -t_srs EPSG:4326"), "{e}");
}

#[test]
fn a_geodetic_crs_without_a_model_type_is_read() {
    let bytes = small(
        &directory(&[(2048, 4269)]),
        &tie(10.0, 20.0, 0.5, 0.25),
        None,
    );
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!(raster.info().geographic_crs_epsg, 4269);
}

#[test]
fn angles_other_than_degrees_are_refused() {
    let keys = [(1024, 2), (2048, 4326), (2054, 9101)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what: "GeogAngularUnitsGeoKey", ref value, .. })
            if value == "9101"
    ));
}

#[test]
fn a_malformed_key_directory_is_refused() {
    let mut dir = directory(&WGS84);
    dir[0] = 2;
    let bytes = small(&dir, &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "GeoKeyDirectoryTag", ref reason })
            if reason.starts_with("version 2")
    ));
    // A model type stored in the doubles' tag instead of the directory.
    let mut dir = directory(&WGS84);
    dir[5] = 34736;
    let bytes = small(&dir, &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "GeoKeyDirectoryTag", ref reason })
            if reason.starts_with("key 1024 is a SHORT")
    ));
    // Three keys promised, two given.
    let mut dir = directory(&WGS84);
    dir.truncate(12);
    let bytes = small(&dir, &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "GeoKeyDirectoryTag", ref reason })
            if reason.starts_with("3 keys need 16 values")
    ));
    // A key twice.
    let bytes = small(
        &directory(&[(1024, 2), (2048, 4326), (2048, 4269)]),
        &tie(10.0, 20.0, 0.5, 0.25),
        None,
    );
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "GeoKeyDirectoryTag", ref reason })
            if reason == "key 2048 appears twice"
    ));
}

#[test]
fn a_zero_pixel_size_is_refused() {
    let bytes = small(&directory(&WGS84), &tie(10.0, 20.0, 0.0, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed {
            what: "the georeferencing",
            ..
        })
    ));
}

/// Extra keys, then the vertical CRS, unit and whether it is stated that they read as.
type Case = (&'static [(u16, u16)], Option<u16>, VerticalUnit, bool);

#[test]
fn vertical_units_convert_to_metres() {
    let cases: [Case; 5] = [
        (&[], None, VerticalUnit::Metre, false),
        (&[(4099, 9001)], None, VerticalUnit::Metre, true),
        (&[(4099, 9002)], None, VerticalUnit::Foot, true),
        (
            &[(4096, 6360)],
            Some(6360),
            VerticalUnit::UsSurveyFoot,
            true,
        ),
        (&[(4096, 5703)], Some(5703), VerticalUnit::Metre, true),
    ];
    for (extra, crs, unit, stated) in cases {
        let mut keys = WGS84.to_vec();
        keys.extend_from_slice(extra);
        let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
        let raster = ElevationRaster::parse(&bytes).unwrap();
        let info = raster.info();
        assert_eq!(
            (
                info.vertical_crs_epsg,
                info.vertical_unit,
                info.vertical_unit_stated
            ),
            (crs, unit, stated),
            "{extra:?}"
        );
        assert_eq!(
            raster.height_at(19.6, 11.2).unwrap(),
            Some(12.0 * unit.metres())
        );
    }
    assert_eq!(VerticalUnit::UsSurveyFoot.metres(), 1200.0 / 3937.0);
}

#[test]
fn an_unknown_vertical_crs_without_a_unit_is_refused() {
    let mut keys = WGS84.to_vec();
    keys.push((4096, 5705));
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, ref value, .. })
            if what.starts_with("a vertical CRS") && value == "5705"
    ));
    let mut keys = WGS84.to_vec();
    keys.push((4099, 9030));
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what: "VerticalUnitsGeoKey", ref value, .. })
            if value == "9030"
    ));
}

#[test]
fn nodata_reads_as_no_height() {
    let bytes = small(
        &directory(&WGS84),
        &tie(10.0, 20.0, 0.5, 0.25),
        Some(" 12 "),
    );
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!(raster.info().nodata, Some(12.0));
    assert_eq!(raster.value_at(19.6, 11.2).unwrap(), None);
    assert_eq!(raster.value_at(19.6, 11.7).unwrap(), Some(13.0));
    assert!(raster.values().unwrap()[6].is_nan());
    // A nodata an int32 cannot hold matches nothing.
    let bytes = small(
        &directory(&WGS84),
        &tie(10.0, 20.0, 0.5, 0.25),
        Some("12.5"),
    );
    assert_eq!(ElevationRaster::parse(&bytes).unwrap().info().nodata, None);
    let bytes = small(
        &directory(&WGS84),
        &tie(10.0, 20.0, 0.5, 0.25),
        Some("twelve"),
    );
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed {
            what: "GDAL_NODATA",
            ..
        })
    ));
}

#[test]
fn nodata_is_rounded_to_the_sample_type() {
    // GDAL's usual float32 nodata, written as a double: the float32 lowest.
    let lowest = SampleType::F32.round_nodata(-3.402_823_466_385_288_6e38);
    assert_eq!(lowest, Some(f64::from(f32::MIN)));
    assert_eq!(SampleType::F32.round_nodata(0.1), Some(f64::from(0.1_f32)));
    assert!(SampleType::F32.round_nodata(f64::NAN).unwrap().is_nan());
    assert_eq!(SampleType::F64.round_nodata(0.1), Some(0.1));
    assert_eq!(SampleType::I16.round_nodata(-32768.0), Some(-32768.0));
    assert_eq!(SampleType::I16.round_nodata(-32769.0), None);
    assert_eq!(SampleType::U8.round_nodata(-1.0), None);
    assert_eq!(SampleType::U16.round_nodata(65535.0), Some(65535.0));
    assert_eq!(SampleType::U32.round_nodata(0.5), None);
}

#[test]
fn a_float_raster_reads_nan_as_no_height() {
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let mut image = tiff.new_image::<colortype::Gray32Float>(2, 1).unwrap();
    let dir = directory(&WGS84);
    let encoder = image.encoder();
    encoder
        .write_tag(Tag::GeoKeyDirectoryTag, &dir[..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelPixelScaleTag, &[0.5, 0.25, 0.0][..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelTiepointTag, &[0.0, 0.0, 0.0, 10.0, 20.0, 0.0][..])
        .unwrap();
    image.write_data(&[f32::NAN, 1234.5]).unwrap();
    let bytes = buf.into_inner();
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!(raster.info().sample, SampleType::F32);
    assert_eq!(raster.value_at(19.9, 10.2).unwrap(), None);
    assert_eq!(raster.value_at(19.9, 10.7).unwrap(), Some(1234.5));
}

#[test]
fn a_multi_band_file_is_refused() {
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let mut image = tiff.new_image::<colortype::RGB8>(1, 1).unwrap();
    let dir = directory(&WGS84);
    image
        .encoder()
        .write_tag(Tag::GeoKeyDirectoryTag, &dir[..])
        .unwrap();
    image.write_data(&[1, 2, 3]).unwrap();
    let bytes = buf.into_inner();
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { ref value, .. }) if value == "3"
    ));
}

#[test]
fn not_a_tiff_is_refused() {
    assert!(matches!(
        ElevationRaster::parse(b"GRIB\0\0\0\x02"),
        Err(GeoTiffError::Tiff(_))
    ));
    assert!(matches!(
        ElevationRaster::parse(&[]),
        Err(GeoTiffError::Tiff(_))
    ));
}

#[test]
fn a_file_on_0_to_360_reads_a_longitude_on_minus_180_to_180() {
    let bytes = include_bytes!("../../tests/fixtures/geotiff/usgs-i32-lzw-tiles-lon360.tif");
    let raster = ElevationRaster::parse(bytes).unwrap();
    assert!(raster.info().corner_longitude_deg > 180.0);
    let east = raster.value_at(32.9903, 253.025).unwrap();
    assert!(east.is_some());
    assert_eq!(raster.value_at(32.9903, -106.975).unwrap(), east);
}

#[test]
fn heights_in_us_survey_feet_read_in_metres() {
    let bytes = include_bytes!("../../tests/fixtures/geotiff/usgs-u16-packbits-ftus-nodata.tif");
    let raster = ElevationRaster::parse(bytes).unwrap();
    let info = raster.info();
    assert_eq!(
        (
            info.vertical_crs_epsg,
            info.vertical_unit,
            info.vertical_unit_stated
        ),
        (Some(6360), VerticalUnit::UsSurveyFoot, true)
    );
    assert_eq!(info.sample, SampleType::U16);
    assert_eq!(info.nodata, Some(0.0));
    // rasterio reads 4607 ftUS at Spaceport America's runway; the float32 file there reads
    // 1404.090 m, which 4607 ftUS rounds (to the nearest foot, 0.3 m).
    assert_eq!(raster.value_at(32.9903, -106.975).unwrap(), Some(4607.0));
    let metres = raster.height_at(32.9903, -106.975).unwrap().unwrap();
    assert_eq!(metres, 4607.0 * (1200.0 / 3937.0));
    assert!((metres - 1_404.090_332_031_25).abs() < 0.16, "{metres}");
}

proptest! {
    /// On any strip layout, a point reads the pixel `pixel_of` names, alone or with others, and
    /// `values` lays the pixels out row by row.
    #[test]
    fn a_point_reads_the_pixel_it_is_placed_in(
        width in 1_u32..40,
        height in 1_u32..40,
        rows_per_strip in 1_u32..45,
        fractions in proptest::collection::vec((0.0_f64..1.0, 0.0_f64..1.0), 1..30),
    ) {
        let data: Vec<i32> = (0..width * height).map(|i| i32::try_from(i).unwrap()).collect();
        let bytes = geotiff_i32(
            width, height, rows_per_strip, &directory(&WGS84),
            &tie(-107.0, 33.0, 1.0 / 3600.0, 1.0 / 3600.0), None, &data,
        );
        let raster = ElevationRaster::parse(&bytes).unwrap();
        let Bounds { south_deg: south, north_deg: north, west_deg: west, east_deg: east } = raster.info().bounds();
        let points: Vec<(f64, f64)> = fractions
            .iter()
            .map(|&(a, b)| (south + a * (north - south), west + b * (east - west)))
            .collect();
        let together = raster.values_at(&points);
        for (&(lat, lon), read) in points.iter().zip(together) {
            let Pixel { row, col } = raster.info().pixel_of(lat, lon).unwrap();
            let expected = Some(f64::from(row * width + col));
            prop_assert_eq!(raster.value_at(lat, lon).unwrap(), expected);
            prop_assert_eq!(read.unwrap(), expected);
        }
        let values = raster.values().unwrap();
        prop_assert!(values.iter().enumerate().all(|(i, &v)| v == i as f64));
    }
}

#[test]
fn a_codec_left_out_is_refused_by_name() {
    let bytes = include_bytes!("../../tests/fixtures/geotiff/zstd-refused.tif");
    let e = ElevationRaster::parse(bytes).unwrap_err();
    assert!(
        matches!(&e, GeoTiffError::Unsupported { what: "compression", value, .. }
            if value == "50000 (zstd)"),
        "{e}"
    );
}

/// A little-endian classic TIFF, written by hand: one tiled image of `bits`-bit samples of
/// `format` (1 unsigned, 2 signed, 3 float), uncompressed, each tile's bytes as given, WGS 84
/// keys, its corner at 107° W, 33° N with one-arc-second pixels.
fn tiled(
    width: u32,
    height: u32,
    tile: (u32, u32),
    bits: u16,
    format: u16,
    tiles: &[Vec<u8>],
) -> Vec<u8> {
    const SHORT: u16 = 3;
    const LONG: u16 = 4;
    const DOUBLE: u16 = 12;
    let second = 1.0 / 3600.0;
    let doubles = |v: &[f64]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let shorts = |v: &[u16]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let dir = directory(&WGS84);
    // (tag, type, count, bytes): the bytes inline when 4 or fewer, else at an offset.
    let mut entries: Vec<(u16, u16, u32, Vec<u8>)> = vec![
        (256, LONG, 1, width.to_le_bytes().to_vec()),
        (257, LONG, 1, height.to_le_bytes().to_vec()),
        (258, SHORT, 1, shorts(&[bits])),
        (259, SHORT, 1, shorts(&[1])),
        (262, SHORT, 1, shorts(&[1])),
        (277, SHORT, 1, shorts(&[1])),
        (322, LONG, 1, tile.0.to_le_bytes().to_vec()),
        (323, LONG, 1, tile.1.to_le_bytes().to_vec()),
        (324, LONG, u32::try_from(tiles.len()).unwrap(), Vec::new()),
        (325, LONG, u32::try_from(tiles.len()).unwrap(), Vec::new()),
        (339, SHORT, 1, shorts(&[format])),
        (33550, DOUBLE, 3, doubles(&[second, second, 0.0])),
        (
            33922,
            DOUBLE,
            6,
            doubles(&[0.0, 0.0, 0.0, -107.0, 33.0, 0.0]),
        ),
        (
            34735,
            SHORT,
            u32::try_from(dir.len()).unwrap(),
            shorts(&dir),
        ),
    ];
    let ifd_len = 2 + 12 * entries.len() + 4;
    let mut data_at = 8 + ifd_len;
    let mut tile_offsets = Vec::new();
    for t in tiles {
        tile_offsets.push(u32::try_from(data_at).unwrap());
        data_at += t.len();
    }
    let longs = |v: &[u32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let counts: Vec<u32> = tiles
        .iter()
        .map(|t| u32::try_from(t.len()).unwrap())
        .collect();
    entries[8].3 = longs(&tile_offsets);
    entries[9].3 = longs(&counts);
    let mut out = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
    let mut ifd = Vec::new();
    let mut extra: Vec<u8> = Vec::new();
    ifd.extend(u16::try_from(entries.len()).unwrap().to_le_bytes());
    for (tag, kind, count, bytes) in &entries {
        ifd.extend(tag.to_le_bytes());
        ifd.extend(kind.to_le_bytes());
        ifd.extend(count.to_le_bytes());
        if bytes.len() <= 4 {
            let mut inline = bytes.clone();
            inline.resize(4, 0);
            ifd.extend(inline);
        } else {
            let at = data_at + extra.len();
            ifd.extend(u32::try_from(at).unwrap().to_le_bytes());
            extra.extend(bytes);
        }
    }
    ifd.extend(0_u32.to_le_bytes());
    out.extend(ifd);
    for t in tiles {
        out.extend(t);
    }
    out.extend(extra);
    out
}

/// An int32 raster, value `row·width + col`, in uncompressed tiles of `tile`.
fn tiled_i32(width: u32, height: u32, tile: (u32, u32)) -> Vec<u8> {
    let (tw, th) = tile;
    let mut tiles = Vec::new();
    for tr in 0..height.div_ceil(th) {
        for tc in 0..width.div_ceil(tw) {
            let mut bytes = Vec::new();
            for r in 0..th {
                for c in 0..tw {
                    let (row, col) = (tr * th + r, tc * tw + c);
                    let v = if row < height && col < width {
                        i32::try_from(row * width + col).unwrap()
                    } else {
                        -1
                    };
                    bytes.extend(v.to_le_bytes());
                }
            }
            tiles.push(bytes);
        }
    }
    tiled(width, height, tile, 32, 2, &tiles)
}

#[test]
fn a_hand_written_tiled_file_reads() {
    let bytes = tiled_i32(40, 20, (32, 16));
    let raster = ElevationRaster::parse(&bytes).unwrap();
    let values = raster.values().unwrap();
    assert!(values.iter().enumerate().all(|(i, &v)| v == i as f64));
    // Row 17, column 35: in the last tile, past both paddings' starts.
    let lat = 33.0 - 17.5 / 3600.0;
    let lon = -107.0 + 35.5 / 3600.0;
    assert_eq!(raster.value_at(lat, lon).unwrap(), Some(17.0 * 40.0 + 35.0));
}

#[test]
fn a_tile_too_large_to_decode_is_refused() {
    // A 1 by 1 float64 raster in one tile 2^32 − 1 pixels wide: 32 GiB once padded.
    let bytes = tiled(1, 1, (u32::MAX, 1), 64, 3, &[vec![0; 8]]);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::TooLarge { size, limit: MAX_CHUNK_BYTES, .. }) if size == u64::from(u32::MAX) * 8
    ));
    // Tiles whose size in bytes passes a u64: 2^31 by 2^30 by 8 is 2^64, and the largest.
    for tile in [(1 << 31, 1 << 30), (u32::MAX, u32::MAX)] {
        let bytes = tiled(1, 1, tile, 64, 3, &[vec![0; 8]]);
        assert!(
            matches!(
                ElevationRaster::parse(&bytes),
                Err(GeoTiffError::TooLarge {
                    size: u64::MAX,
                    limit: MAX_CHUNK_BYTES,
                    ..
                })
            ),
            "{tile:?}"
        );
    }
}

#[test]
fn a_large_raster_whose_tiles_do_not_decode_allocates_nothing_large() {
    // It claims 16,384 by 16,384 float32 pixels (2^28, 2 GiB as f64) in 256-pixel tiles, and
    // holds one byte of each.
    let tiles = vec![vec![0_u8]; 64 * 64];
    let bytes = tiled(16_384, 16_384, (256, 256), 32, 3, &tiles);
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert!(matches!(raster.values(), Err(GeoTiffError::Tiff(_))));
    let one = raster.values_at(&[(32.99, -106.99), (32.99, -106.99)]);
    assert!(
        one.iter().all(|r| matches!(r, Err(GeoTiffError::Tiff(_)))),
        "{one:?}"
    );
}

#[test]
fn a_negative_y_scale_is_refused() {
    let bytes = small(&directory(&WGS84), &tie(10.0, 20.0, 0.5, -0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a negative ModelPixelScaleTag")
    ));
}

#[test]
fn a_pixel_scale_beside_a_matrix_is_refused() {
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let mut image = tiff.new_image::<colortype::GrayI32>(1, 1).unwrap();
    let dir = directory(&WGS84);
    let encoder = image.encoder();
    encoder
        .write_tag(Tag::GeoKeyDirectoryTag, &dir[..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelPixelScaleTag, &[0.5, 0.25, 0.0][..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelTiepointTag, &[0.0, 0.0, 0.0, 10.0, 20.0, 0.0][..])
        .unwrap();
    let m = [
        0.5, 0.0, 0.0, 50.0, 0.0, -0.25, 0.0, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    encoder
        .write_tag(Tag::ModelTransformationTag, &m[..])
        .unwrap();
    image.write_data(&[1]).unwrap();
    let bytes = buf.into_inner();
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "the georeferencing", ref reason })
            if reason.contains("together")
    ));
}

#[test]
fn a_datum_far_from_wgs84_is_refused() {
    // NAD27 (4267), Tokyo (4301), JGD2000 (4612), a user-defined CRS, and none named.
    for keys in [
        vec![(1024, 2), (2048, 4267)],
        vec![(1024, 2), (2048, 4301)],
        vec![(1024, 2), (2048, 4612)],
        vec![(1024, 2), (2048, 32767)],
        vec![(1024, 2)],
    ] {
        let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
        let e = ElevationRaster::parse(&bytes).unwrap_err();
        assert!(
            matches!(
                &e,
                GeoTiffError::Unsupported {
                    what: "a geographic CRS, EPSG",
                    ..
                }
            ),
            "{keys:?}: {e}"
        );
    }
    let paris = [(1024, 2), (2048, 4326), (2051, 8903)];
    let bytes = small(&directory(&paris), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what: "a prime meridian, EPSG", ref value, .. }) if value == "8903"
    ));
}

#[test]
fn an_internal_mask_is_refused() {
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let dir = directory(&WGS84);
    let mut image = tiff.new_image::<colortype::GrayI32>(2, 1).unwrap();
    let encoder = image.encoder();
    encoder
        .write_tag(Tag::GeoKeyDirectoryTag, &dir[..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelPixelScaleTag, &[0.5, 0.25, 0.0][..])
        .unwrap();
    encoder
        .write_tag(Tag::ModelTiepointTag, &[0.0, 0.0, 0.0, 10.0, 20.0, 0.0][..])
        .unwrap();
    image.write_data(&[1, 2]).unwrap();
    let mut mask = tiff.new_image::<colortype::Gray8>(2, 1).unwrap();
    mask.encoder()
        .write_tag(Tag::NewSubfileType, 4_u32)
        .unwrap();
    mask.write_data(&[255, 0]).unwrap();
    let bytes = buf.into_inner();
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, ref value, .. })
            if what.starts_with("an internal nodata mask") && value == "4"
    ));
}

/// A WGS 84 file with a vertical CRS (NAVD88, 5703) and a pixel scale whose `S_z` and tiepoint
/// heights are given.
fn with_z(sz: f64, z0: f64, z: f64, metadata: Option<&str>) -> Vec<u8> {
    let keys = [(1024, 2), (1025, 1), (2048, 4326), (4096, 5703)];
    with_z_keys(&directory(&keys), sz, z0, z, metadata)
}

/// [`with_z`] with its own GeoKey directory.
fn with_z_keys(dir: &[u16], sz: f64, z0: f64, z: f64, metadata: Option<&str>) -> Vec<u8> {
    let georef = Georef::Tiepoint {
        scale: [0.5, 0.25, sz],
        tie: vec![0.0, 0.0, z0, 10.0, 20.0, z],
    };
    let data: Vec<i32> = (0..3)
        .flat_map(|r| (0..4).map(move |c| 10 * r + c))
        .collect();
    let mut buf = Cursor::new(Vec::new());
    let mut tiff = TiffEncoder::new(&mut buf).unwrap();
    let mut image = tiff.new_image::<colortype::GrayI32>(4, 3).unwrap();
    let encoder = image.encoder();
    encoder.write_tag(Tag::GeoKeyDirectoryTag, dir).unwrap();
    if let Georef::Tiepoint { scale, tie } = &georef {
        encoder
            .write_tag(Tag::ModelPixelScaleTag, &scale[..])
            .unwrap();
        encoder
            .write_tag(Tag::ModelTiepointTag, tie.as_slice())
            .unwrap();
    }
    if let Some(text) = metadata {
        encoder
            .write_tag(Tag::Unknown(GDAL_METADATA_TAG), text)
            .unwrap();
    }
    image.write_data(&data).unwrap();
    buf.into_inner()
}

#[test]
fn a_pixel_scale_and_offset_apply_as_gdal_applies_them() {
    // S_z 0.1, z₀ 0, Z₀ 1000: v·0.1 + 1000. Pixel (1, 2) holds 12.
    let bytes = with_z(0.1, 0.0, 1000.0, None);
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!((raster.info().scale, raster.info().offset), (0.1, 1000.0));
    assert_eq!(raster.value_at(19.6, 11.2).unwrap(), Some(12.0));
    assert_eq!(
        raster.height_at(19.6, 11.2).unwrap(),
        Some(12.0 * 0.1 + 1000.0)
    );
    // z₀ shifts the offset: Z₀ − z₀·S_z.
    let bytes = with_z(2.0, 5.0, 100.0, None);
    let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
    assert_eq!((info.scale, info.offset), (2.0, 90.0));
    // All three zero: no scale.
    let bytes = with_z(0.0, 0.0, 0.0, None);
    let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
    assert_eq!((info.scale, info.offset), (1.0, 0.0));
    // Z₀ alone gives GDAL a scale of 0, which is refused.
    let bytes = with_z(0.0, 0.0, 7.0, None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed {
            what: "the pixel scale and offset",
            ..
        })
    ));
    // Without a vertical CRS, S_z is not a scale.
    let georef = Georef::Tiepoint {
        scale: [0.5, 0.25, 0.1],
        tie: vec![0.0, 0.0, 0.0, 10.0, 20.0, 1000.0],
    };
    let bytes = small(&directory(&WGS84), &georef, None);
    let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
    assert_eq!((info.scale, info.offset), (1.0, 0.0));
}

#[test]
fn gdal_metadata_gives_a_scale_and_offset() {
    let xml = |scale: &str| {
        format!(
            "<GDALMetadata>\n  <Item name=\"OFFSET\" sample=\"0\" role=\"offset\">1395</Item>\n  \
             <Item name=\"SCALE\" sample=\"0\" role=\"scale\">{scale}</Item>\n  \
             <Item name=\"SCALE\" sample=\"1\" role=\"scale\">9</Item>\n</GDALMetadata>\n"
        )
    };
    let bytes = with_z(0.0, 0.0, 0.0, Some(&xml("0.25")));
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!((raster.info().scale, raster.info().offset), (0.25, 1395.0));
    assert_eq!(
        raster.height_at(19.6, 11.2).unwrap(),
        Some(12.0 * 0.25 + 1395.0)
    );
    // The same pair from the tags and the metadata is one pair.
    let bytes = with_z(0.25, 0.0, 1395.0, Some(&xml("0.25")));
    assert_eq!(ElevationRaster::parse(&bytes).unwrap().info().scale, 0.25);
    // Two different pairs are refused.
    let bytes = with_z(0.5, 0.0, 1395.0, Some(&xml("0.25")));
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a pixel scale and offset given twice")
    ));
    let bytes = with_z(0.0, 0.0, 0.0, Some(&xml("a quarter")));
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed {
            what: "GDAL_METADATA",
            ..
        })
    ));
    let bytes = with_z(0.0, 0.0, 0.0, Some("<GDALMetadata><Item"));
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed {
            what: "GDAL_METADATA",
            ..
        })
    ));
}

#[test]
fn heights_in_the_tags_apply_only_where_gdal_reads_a_vertical_crs() {
    let base = [(1024, 2), (1025, 1), (2048, 4326)];
    let keys = |more: &[(u16, u16)]| [&base[..], more].concat();
    // A GeoTIFF 1.0 directory: GDAL drops the vertical CRS and the heights with it.
    let mut dir = directory(&keys(&[(4096, 5703)]));
    dir[2] = 0;
    let info = ElevationRaster::parse(&with_z_keys(&dir, 0.1, 0.0, 1000.0, None))
        .unwrap()
        .info()
        .clone();
    assert_eq!((info.scale, info.offset), (1.0, 0.0));
    assert_eq!(info.vertical_crs_epsg, Some(5703));
    // Vertical keys GDAL may or may not resolve to a vertical CRS: a unit alone, a datum alone,
    // a user-defined CRS with a unit, a vertical CRS this reader doesn't know with a unit.
    for more in [
        vec![(4099, 9001)],
        vec![(4098, 5103)],
        vec![(4096, 32767), (4099, 9001)],
        // A known vertical CRS beside a datum key (6030 beside WGS 84, which GDAL turns into
        // WGS 84 3D, is refused before: `vertical_keys_gdal_drops_are_refused`).
        vec![(4096, 5703), (4098, 5103)],
    ] {
        let bytes = with_z_keys(&directory(&keys(&more)), 0.1, 0.0, 1000.0, None);
        assert!(
            matches!(
                ElevationRaster::parse(&bytes),
                Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("heights in ModelPixelScaleTag")
            ),
            "{more:?}"
        );
        // With no heights in the tags, or GDAL's own S_z 1 and Z₀ 0, the same keys read.
        for z in [(0.0, 0.0), (1.0, 0.0)] {
            let bytes = with_z_keys(&directory(&keys(&more)), z.0, 0.0, z.1, None);
            let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
            assert_eq!((info.scale, info.offset), (1.0, 0.0), "{more:?}");
        }
    }
    // With no model type but a unit key, GDAL keeps the vertical CRS, but this reader doesn't
    // claim to know it applies the heights.
    let keys = [(1025, 1), (2048, 4326), (4096, 3855), (4099, 9001)];
    let bytes = with_z_keys(&directory(&keys), 0.1, 0.0, 1000.0, None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("heights in ModelPixelScaleTag")
    ));
    // GDAL's own S_z 1 and Z₀ 0 beside keys GDAL may resolve away, with another scale in
    // GDAL_METADATA: GDAL reads S_z where it keeps the vertical CRS, the metadata where not.
    let meta =
        "<GDALMetadata><Item name=\"S\" sample=\"0\" role=\"scale\">0.25</Item></GDALMetadata>";
    let keys = [
        (1024, 2),
        (1025, 1),
        (2048, 4326),
        (4096, 8228),
        (4098, 5103),
    ];
    let bytes = with_z_keys(&directory(&keys), 1.0, 0.0, 0.0, Some(meta));
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a pixel scale and offset given twice")
    ));
}

#[test]
fn vertical_keys_gdal_drops_are_refused() {
    // Measured through rasterio: GDAL reports no vertical unit for any of these (or, for a
    // private value with no model type, reads it by rules of its own), where this reader would
    // read one.
    for (keys, why) in [
        (
            vec![(1024, 2), (2048, 4326), (4096, 40_000), (4099, 9002)],
            "private value 40000",
        ),
        (
            vec![(1024, 2), (2048, 4326), (4096, 6360), (4098, 40_000)],
            "private value 40000",
        ),
        (
            vec![
                (1024, 2),
                (2048, 4326),
                (4096, 32767),
                (4098, 40_000),
                (4099, 9002),
            ],
            "private value 40000",
        ),
        (
            vec![(2048, 4326), (4096, 6360)],
            "no model type and no unit key",
        ),
        (
            vec![(2048, 4326), (4098, 5103)],
            "no model type and no unit key",
        ),
        (vec![(1024, 2), (2048, 4979), (4096, 6360)], "WGS 84 3D"),
        (
            vec![(1024, 2), (2048, 4326), (4098, 6030), (4099, 9002)],
            "6030",
        ),
        (
            vec![(1024, 2), (2048, 4326), (4096, 6360), (4098, 6030)],
            "6030",
        ),
    ] {
        let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
        assert!(
            matches!(
                ElevationRaster::parse(&bytes),
                Err(GeoTiffError::Unsupported { what: "vertical keys GDAL drops or reads by rules of its own:", ref value, .. })
                    if value.contains(why)
            ),
            "{keys:?}"
        );
    }
    // WGS 84 3D with no vertical key reads, as does datum 6030 beside NAD83, or beside WGS 84 with
    // no model type (GDAL then builds a local CRS and reads the unit key: feet, measured).
    for keys in [
        vec![(1024, 2), (2048, 4979)],
        vec![(1024, 2), (2048, 4269), (4098, 6030), (4099, 9002)],
        vec![(2048, 4326), (4098, 6030), (4099, 9002)],
    ] {
        let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
        let info = ElevationRaster::parse(&bytes)
            .unwrap_or_else(|e| panic!("{keys:?}: {e}"))
            .info()
            .clone();
        assert_eq!(
            info.vertical_unit,
            if keys.len() > 2 {
                VerticalUnit::Foot
            } else {
                VerticalUnit::Metre
            },
            "{keys:?}"
        );
    }
    // WGS 84 3D as GDAL writes it, as a vertical CRS: refused, saying what it is.
    let keys = [(1024, 2), (2048, 4326), (4096, 4979)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { ref value, hint, .. }) if value == "4979" && hint.contains("WGS 84 3D")
    ));
}

#[test]
fn a_user_defined_vertical_crs_is_no_epsg_code() {
    let keys = [(1024, 2), (2048, 4326), (4096, 32767), (4099, 9002)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    let info = ElevationRaster::parse(&bytes).unwrap().info().clone();
    assert_eq!(info.vertical_crs_epsg, None);
    assert_eq!(
        (info.vertical_unit, info.vertical_unit_stated),
        (VerticalUnit::Foot, true)
    );
    // Without a unit, its unit is unknown.
    let keys = [(1024, 2), (2048, 4326), (4096, 32767)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a user-defined vertical CRS")
    ));
}

#[test]
fn gdal_metadata_gives_a_unit_and_is_matched_as_gdal_matches_it() {
    let item = |attributes: &str, text: &str| {
        format!("<GDALMetadata><Item {attributes}>{text}</Item></GDALMetadata>")
    };
    let wgs84 = directory(&[(1024, 2), (1025, 1), (2048, 4326)]);
    let parse = |dir: &[u16], xml: &str| {
        ElevationRaster::parse(&with_z_keys(dir, 0.0, 0.0, 0.0, Some(xml)))
            .map(|r| r.info().clone())
    };
    let unit = item(r#"name="UNITTYPE" sample="0" role="unittype""#, "ft");
    let info = parse(&wgs84, &unit).unwrap();
    assert_eq!(
        (info.vertical_unit, info.vertical_unit_stated),
        (VerticalUnit::Foot, true)
    );
    let info = parse(
        &wgs84,
        &item(r#"name="U" sample="0" role="unittype""#, "US survey foot"),
    )
    .unwrap();
    assert_eq!(info.vertical_unit, VerticalUnit::UsSurveyFoot);
    // Feet in the metadata, metres by NAVD88's code: refused.
    let navd88 = directory(&[(1024, 2), (1025, 1), (2048, 4326), (4096, 5703)]);
    assert!(matches!(
        parse(&navd88, &unit),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a vertical unit given twice")
    ));
    assert_eq!(
        parse(
            &navd88,
            &item(r#"name="U" sample="0" role="unittype""#, "metre")
        )
        .unwrap()
        .vertical_unit,
        VerticalUnit::Metre
    );
    assert!(matches!(
        parse(
            &wgs84,
            &item(r#"name="U" sample="0" role="unittype""#, "furlong")
        ),
        Err(GeoTiffError::Unsupported {
            what: "a GDAL_METADATA unit type",
            ..
        })
    ));
    // Read as GDAL 3.12.2 reads them (each measured through rasterio): any domain but
    // IMAGE_STRUCTURE, the element names in either case.
    let scale = |attributes: &str| item(attributes, "0.5");
    for read in [
        scale(r#"name="S" sample="0" role="scale" domain="x""#),
        scale(r#"name="S" sample="0" role="scale" domain="""#),
        r#"<gdalmetadata><item name="S" sample="0" role="scale">0.5</item></gdalmetadata>"#
            .to_string(),
    ] {
        assert_eq!(parse(&wgs84, &read).unwrap().scale, 0.5, "{read}");
    }
    // Skipped, as GDAL skips them: no name, no sample, another band, another root.
    for skipped in [
        scale(r#"sample="0" role="scale""#),
        scale(r#"name="S" role="scale""#),
        scale(r#"name="S" sample="1" role="scale""#),
        r#"<Other><Item name="S" sample="0" role="scale">0.5</Item></Other>"#.to_string(),
    ] {
        assert_eq!(parse(&wgs84, &skipped).unwrap().scale, 1.0, "{skipped}");
    }
    // An empty item is skipped, so it doesn't undo the unit before it.
    let two = "<GDALMetadata><Item name=\"U\" sample=\"0\" role=\"unittype\">ft</Item>\
               <Item name=\"U\" sample=\"0\" role=\"unittype\"></Item></GDALMetadata>";
    assert_eq!(
        parse(&wgs84, two).unwrap().vertical_unit,
        VerticalUnit::Foot
    );
    // Refused: forms GDAL doesn't write, where its quirks (attribute names in any case, C's
    // `atoi` for the sample, text only as an item's one child, prefixed names compared whole,
    // IMAGE_STRUCTURE's own keys) could read another scale than hpr would.
    let caps = "a namespace or an attribute name in capitals";
    let sample = "sample is not a plain number";
    let text = "value is not plain text";
    for (refused, why) in [
        (scale(r#"NAME="S" SAMPLE="0" ROLE="scale""#), caps),
        (scale(r#"name="S" sample="0" Role="scale""#), caps),
        (
            r#"<GDALMetadata xmlns:x="u"><Item name="S" sample="0" x:role="foo" role="scale">0.5</Item></GDALMetadata>"#
                .to_string(),
            caps,
        ),
        (
            r#"<GDALMetadata xmlns:x="u"><x:Item name="S" sample="0" role="scale">0.5</x:Item></GDALMetadata>"#
                .to_string(),
            caps,
        ),
        (scale(r#"name="S" sample=" 0" role="scale""#), sample),
        (scale(r#"name="S" sample="0.0" role="scale""#), sample),
        (scale(r#"name="S" sample="x" role="scale""#), sample),
        (scale(r#"name="S" sample="" role="scale""#), sample),
        (scale(r#"name="S" sample="-1" role="scale""#), sample),
        (scale(r#"name="S" sample="4294967296" role="scale""#), sample),
        (
            scale(r#"name="S" sample="0" role="scale" domain="image_structure""#),
            "IMAGE_STRUCTURE",
        ),
        (item(r#"name="S" sample="0" role="scale""#, "0.5<!--c-->"), text),
        (item(r#"name="S" sample="0" role="scale""#, "0.5<b/>"), text),
        (item(r#"name="S" sample="0" role="scale""#, "0.<![CDATA[5]]>"), text),
        (item(r#"name="S" sample="0" role="scale""#, "<![CDATA[0.5]]>"), text),
        (item(r#"name="U" sample="0" role="unittype""#, "f<![CDATA[t]]>"), text),
        (
            r#"<x:GDALMetadata xmlns:x="u"><Item name="S" sample="0" role="scale">0.5</Item></x:GDALMetadata>"#
                .to_string(),
            "root in an XML namespace",
        ),
    ] {
        assert!(
            matches!(
                parse(&wgs84, &refused),
                Err(GeoTiffError::Unsupported { what: "GDAL_METADATA", ref value, .. })
                    if value.contains(why)
            ),
            "{refused}"
        );
    }
    // Unicode spaces are not the blanks GDAL's number parsing skips: they are not numbers, nor
    // units.
    assert!(matches!(
        parse(
            &wgs84,
            &item(r#"name="S" sample="0" role="scale""#, "&#160;0.5")
        ),
        Err(GeoTiffError::Malformed {
            what: "GDAL_METADATA",
            ref reason,
        }) if reason.contains("is not a number")
    ));
    assert!(matches!(
        parse(
            &wgs84,
            &item(r#"name="U" sample="0" role="unittype""#, "&#160;ft")
        ),
        Err(GeoTiffError::Unsupported {
            what: "a GDAL_METADATA unit type",
            ref value,
            ..
        }) if value.contains(r"\u{a0}")
    ));
    // A blank written as a character reference, which GDAL reads as 0 or a blank unit, is
    // refused; after a value, it would otherwise leave that value standing.
    for blank in ["&#32;", "  &#32;  ", "\n&#10;\n", "\t&#13;"] {
        for (role, first) in [("scale", "0.5"), ("offset", "10"), ("unittype", "ft")] {
            let xml = format!(
                "<GDALMetadata><Item name=\"A\" sample=\"0\" role=\"{role}\">{first}</Item>\
                 <Item name=\"A\" sample=\"0\" role=\"{role}\">{blank}</Item></GDALMetadata>"
            );
            assert!(
                matches!(
                    parse(&wgs84, &xml),
                    Err(GeoTiffError::Unsupported { what: "GDAL_METADATA", ref value, .. })
                        if value.contains("blank character reference")
                ),
                "{xml}"
            );
        }
    }
    // Blanks typed as they are, around a number or alone, are skipped, as GDAL skips them.
    assert_eq!(
        parse(
            &wgs84,
            &item(r#"name="S" sample="0" role="scale""#, " 0.5\n")
        )
        .unwrap()
        .scale,
        0.5
    );
}

#[test]
fn a_unit_key_against_its_vertical_crs_is_refused() {
    // NAVD88 in metres (5703) with US survey feet (9003): GDAL takes metres, a writer may mean
    // feet.
    let keys = [(1024, 2), (2048, 4326), (4096, 5703), (4099, 9003)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a vertical unit given twice")
    ));
    // A vertical CRS this reader doesn't know (5705, CGVD2013's 6647), whose unit GDAL takes
    // from EPSG's registry whatever the key says.
    for code in [5705, 6647] {
        let keys = [(1024, 2), (2048, 4326), (4096, code), (4099, 9002)];
        let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
        assert!(
            matches!(
                ElevationRaster::parse(&bytes),
                Err(GeoTiffError::Unsupported { what, .. }) if what.starts_with("a vertical CRS this reader doesn't know")
            ),
            "{code}"
        );
    }
    // Agreeing, it reads.
    let keys = [(1024, 2), (2048, 4326), (4096, 5703), (4099, 9001)];
    let bytes = small(&directory(&keys), &tie(10.0, 20.0, 0.5, 0.25), None);
    assert!(ElevationRaster::parse(&bytes).is_ok());
}

#[test]
fn a_later_image_the_decoder_cannot_read_is_not_a_mask() {
    // A second directory holding only ImageWidth: not an image, so not a mask GDAL would apply.
    let mut bytes = small_wgs84();
    let first = usize::try_from(u32::from_le_bytes(bytes[4..8].try_into().unwrap())).unwrap();
    let entries = usize::from(u16::from_le_bytes(
        bytes[first..first + 2].try_into().unwrap(),
    ));
    let next = first + 2 + 12 * entries;
    if bytes.len() % 2 == 1 {
        bytes.push(0);
    }
    let second = u32::try_from(bytes.len()).unwrap();
    bytes[next..next + 4].copy_from_slice(&second.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(256_u16.to_le_bytes());
    bytes.extend(4_u16.to_le_bytes());
    bytes.extend(1_u32.to_le_bytes());
    bytes.extend(2_u32.to_le_bytes());
    bytes.extend(0_u32.to_le_bytes());
    let raster = ElevationRaster::parse(&bytes).unwrap();
    assert_eq!(raster.value_at(19.6, 11.2).unwrap(), Some(12.0));
}

#[test]
fn deeply_nested_gdal_metadata_is_refused_before_it_is_parsed() {
    let xml = format!("{}{}", "<a>".repeat(10_000), "</a>".repeat(10_000));
    let bytes = with_z(0.0, 0.0, 0.0, Some(&xml));
    assert!(matches!(
        ElevationRaster::parse(&bytes),
        Err(GeoTiffError::Malformed { what: "GDAL_METADATA", ref reason }) if reason.starts_with("nested 10000 deep")
    ));
}

#[test]
fn the_debug_form_does_not_print_the_file() {
    let bytes = small_wgs84();
    let raster = ElevationRaster::parse(&bytes).unwrap();
    let shown = format!("{raster:?}");
    assert!(
        shown.contains(&format!("bytes: {}", bytes.len())),
        "{shown}"
    );
}

proptest! {
    /// On any tile layout, square or not, a point reads the pixel `pixel_of` names, and `values`
    /// lays the pixels out row by row.
    #[test]
    fn a_point_reads_its_pixel_on_any_tiles(
        width in 1_u32..70,
        height in 1_u32..70,
        tw in prop::sample::select(vec![16_u32, 32, 48]),
        th in prop::sample::select(vec![16_u32, 32, 48]),
        fractions in proptest::collection::vec((0.0_f64..1.0, 0.0_f64..1.0), 1..20),
    ) {
        let bytes = tiled_i32(width, height, (tw, th));
        let raster = ElevationRaster::parse(&bytes).unwrap();
        let b = raster.info().bounds();
        let points: Vec<(f64, f64)> = fractions
            .iter()
            .map(|&(a, c)| (b.south_deg + a * (b.north_deg - b.south_deg), b.west_deg + c * (b.east_deg - b.west_deg)))
            .collect();
        for (&(lat, lon), read) in points.iter().zip(raster.values_at(&points)) {
            let Pixel { row, col } = raster.info().pixel_of(lat, lon).unwrap();
            let expected = Some(f64::from(row * width + col));
            prop_assert_eq!(raster.value_at(lat, lon).unwrap(), expected);
            prop_assert_eq!(read.unwrap(), expected);
        }
        prop_assert!(raster.values().unwrap().iter().enumerate().all(|(i, &v)| v == i as f64));
    }

    /// A fixture with bytes changed reads or is refused, and never panics.
    #[test]
    fn a_damaged_file_never_panics(
        which in 0_usize..3,
        changes in proptest::collection::vec((any::<usize>(), any::<u8>()), 1..8),
    ) {
        let files: [&[u8]; 3] = [
            include_bytes!("../../tests/fixtures/geotiff/usgs-f32-lzw-fp-tiles.tif"),
            include_bytes!("../../tests/fixtures/geotiff/usgs-i16-deflate-strips-be.tif"),
            include_bytes!("../../tests/fixtures/geotiff/usgs-u16-packbits-ftus-nodata.tif"),
        ];
        let mut bytes = files[which].to_vec();
        for (at, byte) in changes {
            let n = bytes.len();
            bytes[at % n] = byte;
        }
        if let Ok(raster) = ElevationRaster::parse(&bytes) {
            let _ = raster.value_at(32.99, -106.97);
            let _ = raster.values_at(&[(32.99, -106.97), (32.985, -106.98)]);
        }
    }
}
