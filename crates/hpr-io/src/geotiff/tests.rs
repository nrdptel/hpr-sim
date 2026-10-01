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
    assert_eq!(info.bounds_deg(), (19.25, 20.0, 10.0, 12.0));
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
    assert_eq!(raster.pixel(2, 3).unwrap(), Some(23.0));
    assert_eq!(
        raster.pixel(3, 0),
        Err(GeoTiffError::NoSuchPixel {
            row: 3,
            col: 0,
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
            south_deg: 19.25,
            north_deg: 20.0,
            west_deg: 10.0,
            east_deg: 12.0
        }
    );
    assert!(e.to_string().contains("latitudes 19.25° to 20°"), "{e}");
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
    assert_eq!(raster.info().geographic_crs_epsg, Some(4269));
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
    let cases: [Case; 6] = [
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
        // The stated unit wins over the CRS's.
        (
            &[(4096, 5703), (4099, 9003)],
            Some(5703),
            VerticalUnit::UsSurveyFoot,
            true,
        ),
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
        let (south, north, west, east) = raster.info().bounds_deg();
        let points: Vec<(f64, f64)> = fractions
            .iter()
            .map(|&(a, b)| (south + a * (north - south), west + b * (east - west)))
            .collect();
        let together = raster.values_at(&points).unwrap();
        for (&(lat, lon), read) in points.iter().zip(together) {
            let (col, row) = raster.info().pixel_of(lat, lon).unwrap();
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
    // The tags read; the pixels do not.
    let raster = ElevationRaster::parse(bytes).unwrap();
    let e = raster.value_at(19.9, 10.2).unwrap_err();
    assert!(
        matches!(&e, GeoTiffError::Tiff(text) if text.to_lowercase().contains("zstd")),
        "{e}"
    );
}
