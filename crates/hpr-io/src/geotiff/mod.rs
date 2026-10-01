//! A launch site's height from a user's elevation file: a GeoTIFF digital elevation model (DEM)
//! on a geographic (latitude and longitude) grid.
//!
//! A GeoTIFF is a TIFF image whose pixels are heights and whose tags say where on Earth they lie.
//! The caller reads the whole file into memory and passes its bytes; [`ElevationRaster::parse`]
//! reads the tags, and [`ElevationRaster::height_at`] finds the pixel a latitude and longitude
//! fall in and decodes only the tile or strip that holds it, so a lookup adds about one tile's
//! memory to the file's. The TIFF itself (codecs, predictors, tiles, strips, byte order, BigTIFF)
//! is decoded by image-rs's `tiff` crate ([docs.rs][tiff]); this module reads the geographic tags
//! as the OGC GeoTIFF Standard 1.1 (OGC 19-008r4, 2019) defines them, and where GDAL reads a file
//! differently from the standard it either follows GDAL or refuses the file, so a height read
//! here is the one GDAL's readers give for the same point.
//!
//! **Where a pixel lies.** The file ties raster coordinates to longitude and latitude with a
//! tiepoint `(I, J) ↦ (X, Y)` and a pixel size `(S_x, S_y)` (`ModelTiepointTag`,
//! `ModelPixelScaleTag`; §7.3), or with an affine matrix (`ModelTransformationTag`; its terms
//! `a, b, d` and `e, f, h` give `X = a·I + b·J + d`, `Y = e·I + f·J + h`). A positive `S_y` means
//! latitude falls as rows go down (Requirement 10.4). As GDAL does, both become the longitude and
//! latitude of the outer corner of pixel `(0, 0)` and a signed pixel size:
//!
//! `λ₀ = X − I·S_x`, `φ₀ = Y − J·(−S_y)`, `Δλ = S_x`, `Δφ = −S_y`, or from the matrix
//! `λ₀ = d`, `Δλ = a`, `φ₀ = h`, `Δφ = f`.
//!
//! The raster type (`GTRasterTypeGeoKey`, §7.2.1) says what a raster coordinate names. In the
//! usual *pixel is area*, `(0, 0)` is the outer corner of the first pixel; in *pixel is point* it
//! is that pixel's centre, so the corner is half a pixel back: `λ₀ −= Δλ/2`, `φ₀ −= Δφ/2`. A point
//! `(φ, λ)` then lies in column `⌊(λ − λ₀)/Δλ⌋` and row `⌊(φ − φ₀)/Δφ⌋`: the pixel whose area
//! holds it, with no interpolation between pixels. On a 1-arc-second grid (about 31 m by 26 m at
//! 33° N) that is the height of the ground within about 20 m of the site. A point within about
//! 10⁻¹³ of a pixel's width of an edge can fall on either side, here and in GDAL, by rounding.
//!
//! Refused where the standard and GDAL disagree, or where GDAL needs more than this reads: a
//! negative `S_y` (GDAL reads it as north-up, the standard as south-up), both a pixel scale and a
//! matrix (the standard forbids it; GDAL takes the scale), a rotated matrix, several tiepoints
//! (ground control points) and an internal nodata mask.
//!
//! **What is read.** One band of unsigned or signed 8-, 16- or 32-bit integers, or 32- or 64-bit
//! floats; uncompressed, LZW, Deflate or PackBits, with or without the horizontal or
//! floating-point predictor; tiles or strips; little- or big-endian; classic TIFF or BigTIFF.
//! The first image in the file is the one read (later ones are a cloud-optimized GeoTIFF's
//! overviews). The CRS must be geographic (`GTModelTypeGeoKey` 2) in degrees from Greenwich, and
//! one of [`NEAR_WGS84`]: datums within a few metres of WGS 84, where a point's WGS 84 latitude
//! and longitude read the right pixel to within a few metres (more near the rupture of a large
//! earthquake since the datum was fixed; the list gives examples). Its EPSG code is reported. Any
//! other, and a projected file (UTM, say), is refused with [`GeoTiffError::Unsupported`] naming
//! its code; `gdalwarp -t_srs EPSG:4326 in.tif out.tif` turns it into one this reads.
//!
//! **Heights.** A pixel's raw value `v` becomes a height in metres as
//! `(v·scale + offset)·unit`. The scale and offset are GDAL's: for a GeoTIFF 1.1 file naming a
//! vertical CRS this module knows, `S_z` from `ModelPixelScaleTag` and `Z₀ − z₀·S_z` from the
//! tiepoint's heights; otherwise the `scale` and `offset` items of GDAL's `GDAL_METADATA` tag;
//! otherwise 1 and 0. Heights in those tags with no vertical key are ignored, as GDAL ignores
//! them; with other vertical keys, whether GDAL applies them turns on how it resolves the keys,
//! and the file is refused. In a GeoTIFF 1.0 directory GDAL drops the vertical CRS, and with it
//! those heights; this module does the same, and reports the CRS's code and unit. The unit is the
//! vertical unit the file states
//! (`VerticalUnitsGeoKey`: metres, international feet or US survey feet, refused if it disagrees
//! with a vertical CRS this module knows, whose unit GDAL takes), the unit of such a CRS, or the `unittype` item of `GDAL_METADATA` (refused if it disagrees with
//! the keys); a file that states none is read as metres, an assumption GDAL doesn't make (it
//! reports no unit), flagged by [`RasterInfo::vertical_unit_stated`]. A file in feet that states
//! no unit reads 3.28 times too high. The vertical datum (`VerticalGeoKey`, NAVD88 or EGM2008,
//! say) is reported, not applied. A value equal to the file's nodata value (GDAL's `GDAL_NODATA`
//! tag) or a NaN reads as no height; a nodata value the sample type can't hold exactly, such as
//! 12.5 on integers, matches nothing.
//!
//! **Bounds on a hostile file.** A tile or strip larger than [`MAX_CHUNK_BYTES`] decoded is
//! refused at [`ElevationRaster::parse`], and [`ElevationRaster::values`] grows the raster
//! fallibly, a row of tiles at a time as they decode. `GDAL_METADATA` nested more than 16 deep
//! is refused before it is parsed. The `tiff` crate prints one
//! debug line to standard error when a tag's value passes its 1 MiB limit (its own `dbg!`).
//!
//! **Guide:** [A launch site's elevation][guide] walks through an example and says how the reader
//! is checked: against GDAL's reading, through rasterio, of seven files and a whole USGS tile.
//! The choices are in [ADR-128, a site's height from a user's GeoTIFF][adr].
//!
//! ```
//! use hpr_io::geotiff::ElevationRaster;
//!
//! // A real program reads its file: `let bytes = std::fs::read(path)?;`.
//! let bytes = include_bytes!("../../tests/fixtures/geotiff/usgs-f32-lzw-fp-tiles.tif");
//! let raster = ElevationRaster::parse(bytes)?;
//! // Spaceport America's runway: 1,400.691 m in the USGS's terrain model.
//! let height_m = raster.height_at(32.99, -106.97)?;
//! assert_eq!(height_m.map(|h| (h * 1000.0).round() / 1000.0), Some(1400.691));
//! # Ok::<(), hpr_io::geotiff::GeoTiffError>(())
//! ```
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/elevation.html#from-an-elevation-file-of-your-own
//! [adr]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-128-m53c2-a-sites-height-from-a-users-geotiff-held-to-rasterios-reading-2026-09-30
//! [tiff]: https://docs.rs/tiff/0.11.3/tiff/

use std::io::Cursor;

use serde::{Deserialize, Serialize};
use tiff::decoder::{ChunkType, Decoder, DecodingResult};
use tiff::tags::{CompressionMethod, PhotometricInterpretation, SampleFormat, Tag};

/// `GTModelTypeGeoKey` (OGC 19-008r4 §7.2.2): 1 projected, 2 geographic, 3 geocentric.
const MODEL_TYPE_KEY: u16 = 1024;
/// `GTRasterTypeGeoKey` (§7.2.1): 1 pixel is area, 2 pixel is point.
const RASTER_TYPE_KEY: u16 = 1025;
/// `GeodeticCRSGeoKey` (§7.4.3), `GeographicTypeGeoKey` in GeoTIFF 1.0.
const GEODETIC_CRS_KEY: u16 = 2048;
/// `PrimeMeridianGeoKey` (§7.5.3).
const PRIME_MERIDIAN_KEY: u16 = 2051;
/// `GeogAngularUnitsGeoKey` (§7.5.1).
const ANGULAR_UNITS_KEY: u16 = 2054;
/// `ProjectedCRSGeoKey` (§7.4.2).
const PROJECTED_CRS_KEY: u16 = 3072;
/// `VerticalGeoKey` (§7.4.4), `VerticalCSTypeGeoKey` in GeoTIFF 1.0.
const VERTICAL_CRS_KEY: u16 = 4096;
const VERTICAL_DATUM_KEY: u16 = 4098;
/// `VerticalUnitsGeoKey` (§7.5.1).
const VERTICAL_UNITS_KEY: u16 = 4099;
/// GDAL's `GDAL_METADATA` TIFF tag: an XML list of items, a band's scale and offset among them.
const GDAL_METADATA_TAG: u16 = 42112;
/// GDAL's `GDAL_NODATA` TIFF tag: the nodata value as ASCII text.
const GDAL_NODATA_TAG: u16 = 42113;
/// EPSG's Greenwich prime meridian.
const GREENWICH: u16 = 8901;
/// The most images the reader walks looking for an internal mask; a cloud-optimized GeoTIFF has
/// one per overview level, a handful.
const MAX_IMAGES_WALKED: usize = 64;

/// The most pixels [`ElevationRaster::values`] decodes into one vector: 2²⁸, 2 GiB of `f64`.
pub const MAX_VALUES_PIXELS: u64 = 1 << 28;

/// The largest tile or strip, decoded, a file may have: 256 MiB, the `tiff` crate's own limit
/// on a decoded chunk, which its padding of a floating-point tile would otherwise bypass.
pub const MAX_CHUNK_BYTES: u64 = 256 << 20;

/// The geographic CRSs read, by EPSG code: datums whose latitude and longitude lie within a few
/// metres of WGS 84's, so a point given in WGS 84 reads the right pixel, or its neighbour on a
/// grid finer than a few metres. They part by plate motion since each was fixed, and by
/// earthquakes: near the rupture of a large one since a datum was fixed, such as Chile's in 2010
/// for SIRGAS 2000 or Wenchuan's in 2008 for CGCS2000, the ground moved several metres. JGD2000
/// is left out: Japan's 2011 earthquake moved its north-east by more than 5 m, and JGD2011
/// replaced it.
pub const NEAR_WGS84: [(u16, &str); 14] = [
    (4326, "WGS 84"),
    (4979, "WGS 84, 3D"),
    (4269, "NAD83"),
    (4152, "NAD83(HARN)"),
    (4759, "NAD83(NSRS2007)"),
    (6318, "NAD83(2011)"),
    (4617, "NAD83(CSRS)"),
    (4258, "ETRS89"),
    (4283, "GDA94"),
    (7844, "GDA2020"),
    (4167, "NZGD2000"),
    (6668, "JGD2011"),
    (4674, "SIRGAS 2000"),
    (4490, "CGCS2000"),
];

/// Why a GeoTIFF elevation file could not be read, or a height not taken from it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum GeoTiffError {
    /// The TIFF decoder refused the file or one of its tiles.
    #[error("not a readable TIFF: {0}")]
    Tiff(String),
    /// A tag or key the reader needs is absent.
    #[error("the file has no {what}")]
    Missing {
        /// The tag or key.
        what: &'static str,
    },
    /// A tag or key holds a value the standard does not allow.
    #[error("{what} is malformed: {reason}")]
    Malformed {
        /// The tag or key.
        what: &'static str,
        /// What is wrong with it.
        reason: String,
    },
    /// The file is valid but uses something this reader does not read.
    #[error("{what} {value} is not read{hint}")]
    Unsupported {
        /// The feature.
        what: &'static str,
        /// Its value in the file.
        value: String,
        /// What to do instead, with a leading separator, or empty.
        hint: &'static str,
    },
    /// A latitude or longitude that is not a place.
    #[error("latitude {latitude_deg}° and longitude {longitude_deg}° are not a place")]
    Location {
        /// Latitude asked for, degrees.
        latitude_deg: f64,
        /// Longitude asked for, degrees.
        longitude_deg: f64,
    },
    /// The point is not over the raster.
    #[error(
        "latitude {latitude_deg}° and longitude {longitude_deg}° are outside the raster, which spans latitudes {:.6}° to {:.6}° and longitudes {:.6}° to {:.6}°",
        bounds.south_deg, bounds.north_deg, bounds.west_deg, bounds.east_deg
    )]
    Outside {
        /// Latitude asked for, degrees.
        latitude_deg: f64,
        /// Longitude asked for, degrees.
        longitude_deg: f64,
        /// The raster's edges.
        bounds: Bounds,
    },
    /// A pixel past the raster's last row or column.
    #[error("pixel (row {}, column {}) is outside a raster of {width} by {height}", pixel.row, pixel.col)]
    NoSuchPixel {
        /// The pixel asked for.
        pixel: Pixel,
        /// The raster's columns.
        width: u32,
        /// Its rows.
        height: u32,
    },
    /// The raster or one of its tiles is too large to decode.
    #[error("{what} of {size} is more than the {limit} decoded at once")]
    TooLarge {
        /// What is too large, with its unit.
        what: &'static str,
        /// Its size.
        size: u64,
        /// The limit.
        limit: u64,
    },
}

impl From<tiff::TiffError> for GeoTiffError {
    fn from(e: tiff::TiffError) -> Self {
        GeoTiffError::Tiff(e.to_string())
    }
}

/// A pixel's place in the raster, counted from 0 at the first row and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pixel {
    /// Row, from the first (northernmost when north is up).
    pub row: u32,
    /// Column, from the first (westernmost when east is right).
    pub col: u32,
}

/// A raster's edges, degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    /// Southern edge.
    pub south_deg: f64,
    /// Northern edge.
    pub north_deg: f64,
    /// Western edge.
    pub west_deg: f64,
    /// Eastern edge.
    pub east_deg: f64,
}

/// What a raster coordinate names (`GTRasterTypeGeoKey`, OGC 19-008r4 §7.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RasterType {
    /// Raster coordinate `(0, 0)` is the outer corner of the first pixel (code 1, the default).
    PixelIsArea,
    /// Raster coordinate `(0, 0)` is the centre of the first pixel (code 2).
    PixelIsPoint,
}

/// The unit a pixel's value is in, converted to metres by [`VerticalUnit::metres`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VerticalUnit {
    /// EPSG 9001.
    Metre,
    /// The international foot, 0.3048 m exactly (EPSG 9002).
    Foot,
    /// The US survey foot, 1200/3937 m (EPSG 9003).
    UsSurveyFoot,
}

impl VerticalUnit {
    /// The unit's length in metres.
    #[must_use]
    pub fn metres(self) -> f64 {
        match self {
            VerticalUnit::Metre => 1.0,
            VerticalUnit::Foot => 0.3048,
            VerticalUnit::UsSurveyFoot => 1200.0 / 3937.0,
        }
    }

    fn from_epsg(code: u16) -> Option<Self> {
        match code {
            9001 => Some(VerticalUnit::Metre),
            9002 => Some(VerticalUnit::Foot),
            9003 => Some(VerticalUnit::UsSurveyFoot),
            _ => None,
        }
    }
}

/// The vertical CRSs whose unit this reader knows when a file names one without stating its
/// unit, from EPSG's registry: (code, unit). All are gravity-related heights, positive up.
const VERTICAL_CRS_UNITS: [(u16, VerticalUnit); 8] = [
    (3855, VerticalUnit::Metre),        // EGM2008 height
    (5701, VerticalUnit::Metre),        // ODN height
    (5703, VerticalUnit::Metre),        // NAVD88 height
    (5714, VerticalUnit::Metre),        // MSL height
    (5773, VerticalUnit::Metre),        // EGM96 height
    (5798, VerticalUnit::Metre),        // EGM84 height
    (6360, VerticalUnit::UsSurveyFoot), // NAVD88 height (ftUS)
    (8228, VerticalUnit::Foot),         // NAVD88 height (ft)
];

/// The sample type of the raster's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SampleType {
    /// Unsigned 8-bit integers.
    U8,
    /// Signed 8-bit integers.
    I8,
    /// Unsigned 16-bit integers.
    U16,
    /// Signed 16-bit integers.
    I16,
    /// Unsigned 32-bit integers.
    U32,
    /// Signed 32-bit integers.
    I32,
    /// IEEE 754 single precision.
    F32,
    /// IEEE 754 double precision.
    F64,
}

impl SampleType {
    /// Bytes per sample.
    fn bytes(self) -> u64 {
        match self {
            SampleType::U8 | SampleType::I8 => 1,
            SampleType::U16 | SampleType::I16 => 2,
            SampleType::U32 | SampleType::I32 | SampleType::F32 => 4,
            SampleType::F64 => 8,
        }
    }

    /// The nodata value as a sample of this type can hold it, or `None` if none can.
    fn round_nodata(self, nodata: f64) -> Option<f64> {
        let integral = |lo: f64, hi: f64| {
            (nodata.fract() == 0.0 && (lo..=hi).contains(&nodata)).then_some(nodata)
        };
        match self {
            SampleType::U8 => integral(0.0, f64::from(u8::MAX)),
            SampleType::I8 => integral(f64::from(i8::MIN), f64::from(i8::MAX)),
            SampleType::U16 => integral(0.0, f64::from(u16::MAX)),
            SampleType::I16 => integral(f64::from(i16::MIN), f64::from(i16::MAX)),
            SampleType::U32 => integral(0.0, f64::from(u32::MAX)),
            SampleType::I32 => integral(f64::from(i32::MIN), f64::from(i32::MAX)),
            // Rounded to nearest, as GDAL's GeoTIFF reader rounds it; a NaN stays NaN and is
            // matched by `is_nan`.
            #[expect(clippy::cast_possible_truncation, reason = "f32 nodata is f32-rounded")]
            SampleType::F32 => Some(f64::from(nodata as f32)),
            SampleType::F64 => Some(nodata),
        }
    }
}

/// What an elevation file says about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RasterInfo {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Longitude of the outer corner of pixel `(0, 0)` (its west edge when `Δλ > 0`), degrees.
    pub corner_longitude_deg: f64,
    /// Latitude of the outer corner of pixel `(0, 0)` (its north edge when `Δφ < 0`), degrees.
    pub corner_latitude_deg: f64,
    /// `Δλ`: longitude change from one column to the next, degrees.
    pub pixel_longitude_deg: f64,
    /// `Δφ`: latitude change from one row to the next, degrees; negative when north is up.
    pub pixel_latitude_deg: f64,
    /// What the file's tiepoint names; already applied to the corner above.
    pub raster_type: RasterType,
    /// The geographic CRS's EPSG code, one of [`NEAR_WGS84`].
    pub geographic_crs_epsg: u16,
    /// The vertical CRS's EPSG code, if the file names one (a user-defined or private code,
    /// 32767 and above, is not one).
    pub vertical_crs_epsg: Option<u16>,
    /// The unit pixel values are in, after the scale and offset.
    pub vertical_unit: VerticalUnit,
    /// Whether the file states that unit (by `VerticalUnitsGeoKey`, a vertical CRS this reader
    /// knows, or a unit type in GDAL's `GDAL_METADATA`); `false` means metres were assumed.
    pub vertical_unit_stated: bool,
    /// The pixels' scale: a raw value `v` stands for `v·scale + offset` in the vertical unit.
    pub scale: f64,
    /// The pixels' offset, in the vertical unit.
    pub offset: f64,
    /// The pixels' sample type.
    pub sample: SampleType,
    /// The nodata value, rounded to the sample type, if the file has one a sample can hold.
    pub nodata: Option<f64>,
}

impl RasterInfo {
    /// The pixel holding a point, by the module's rule, or `None` if the point is off the
    /// raster. The longitude is tried as given and then 360° to either side, so a file on 0° to
    /// 360° reads a longitude given on −180° to 180°.
    #[must_use]
    pub fn pixel_of(&self, latitude_deg: f64, longitude_deg: f64) -> Option<Pixel> {
        let row = Self::index(
            latitude_deg,
            self.corner_latitude_deg,
            self.pixel_latitude_deg,
            self.height,
        )?;
        [longitude_deg, longitude_deg + 360.0, longitude_deg - 360.0]
            .into_iter()
            .find_map(|lon| {
                Self::index(
                    lon,
                    self.corner_longitude_deg,
                    self.pixel_longitude_deg,
                    self.width,
                )
            })
            .map(|col| Pixel { row, col })
    }

    fn index(at: f64, corner: f64, step: f64, count: u32) -> Option<u32> {
        let i = ((at - corner) / step).floor();
        // `i` is an integer-valued f64 below `count` (at most u32::MAX), so the cast is exact.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked to lie in 0..count"
        )]
        (i >= 0.0 && i < f64::from(count)).then_some(i as u32)
    }

    /// The raster's edges.
    #[must_use]
    pub fn bounds(&self) -> Bounds {
        let lat_far = self.corner_latitude_deg + f64::from(self.height) * self.pixel_latitude_deg;
        let lon_far = self.corner_longitude_deg + f64::from(self.width) * self.pixel_longitude_deg;
        Bounds {
            south_deg: self.corner_latitude_deg.min(lat_far),
            north_deg: self.corner_latitude_deg.max(lat_far),
            west_deg: self.corner_longitude_deg.min(lon_far),
            east_deg: self.corner_longitude_deg.max(lon_far),
        }
    }

    /// A raw value as a height in metres: `(v·scale + offset)·unit`.
    #[must_use]
    pub fn metres(&self, value: f64) -> f64 {
        (value * self.scale + self.offset) * self.vertical_unit.metres()
    }
}

/// A GeoTIFF elevation file, its tags read and its pixels left packed in the borrowed bytes.
#[derive(Clone)]
pub struct ElevationRaster<'a> {
    bytes: &'a [u8],
    info: RasterInfo,
    chunk_type: ChunkType,
    chunk_width: u32,
    chunk_height: u32,
}

impl std::fmt::Debug for ElevationRaster<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElevationRaster")
            .field("bytes", &self.bytes.len())
            .field("info", &self.info)
            .field("chunk_type", &self.chunk_type)
            .field("chunk_width", &self.chunk_width)
            .field("chunk_height", &self.chunk_height)
            .finish()
    }
}

impl<'a> ElevationRaster<'a> {
    /// Reads a GeoTIFF's tags: its size, sample type, layout, georeferencing, scale, units and
    /// nodata value.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError`] if the TIFF is unreadable, its georeferencing is missing, malformed or
    /// one the module docs list as refused, its CRS is not one of [`NEAR_WGS84`] in degrees, its
    /// pixels are not one band of a [`SampleType`] in a codec it reads, or a tile is larger than
    /// [`MAX_CHUNK_BYTES`].
    pub fn parse(bytes: &'a [u8]) -> Result<Self, GeoTiffError> {
        let mut decoder = Decoder::new(Cursor::new(bytes))?;
        let (width, height) = decoder.dimensions()?;
        if width == 0 || height == 0 {
            return Err(GeoTiffError::Malformed {
                what: "the image size",
                reason: format!("{width} by {height} pixels"),
            });
        }
        let samples: u16 = decoder
            .find_tag_unsigned(Tag::SamplesPerPixel)?
            .unwrap_or(1);
        if samples != 1 {
            return Err(GeoTiffError::Unsupported {
                what: "a pixel of samples numbering",
                value: samples.to_string(),
                hint: "; an elevation file has one band",
            });
        }
        let photometric: Option<u16> = decoder.find_tag_unsigned(Tag::PhotometricInterpretation)?;
        if photometric == Some(PhotometricInterpretation::WhiteIsZero.to_u16()) {
            return Err(GeoTiffError::Unsupported {
                what: "photometric interpretation",
                value: "WhiteIsZero".into(),
                hint: "; its values would read inverted",
            });
        }
        compression(&mut decoder)?;
        let sample = sample_type(&mut decoder)?;
        let keys = GeoKeys::read(&mut decoder)?;
        let raster_type = match keys.get(RASTER_TYPE_KEY)? {
            None | Some(1) => RasterType::PixelIsArea,
            Some(2) => RasterType::PixelIsPoint,
            Some(other) => {
                return Err(GeoTiffError::Unsupported {
                    what: "GTRasterTypeGeoKey",
                    value: other.to_string(),
                    hint: "",
                });
            }
        };
        let geographic_crs_epsg = geographic_crs(&keys)?;
        let (vertical_crs_epsg, vertical_unit, vertical_unit_stated) = vertical(&keys)?;
        let georef = transform(&mut decoder)?;
        let [mut lon0, lon_step, mut lat0, lat_step] = georef.corner;
        if raster_type == RasterType::PixelIsPoint {
            lon0 -= lon_step * 0.5;
            lat0 -= lat_step * 0.5;
        }
        let heights = if keys.minor == 1
            && vertical_crs_epsg.is_some_and(|c| VERTICAL_CRS_UNITS.iter().any(|(v, _)| *v == c))
            && !keys.has(VERTICAL_DATUM_KEY)
            && geographic_crs_epsg != 4979
        {
            ZTerms::Applied
        } else if keys.minor == 0
            || [VERTICAL_CRS_KEY, VERTICAL_DATUM_KEY, VERTICAL_UNITS_KEY]
                .iter()
                .all(|&k| !keys.has(k))
        {
            ZTerms::Ignored
        } else {
            ZTerms::Unknown
        };
        let (scale, offset, unit) = scale_offset(&mut decoder, &georef, heights)?;
        let (vertical_unit, vertical_unit_stated) = match unit {
            Some(unit) if vertical_unit_stated && unit != vertical_unit => {
                return Err(GeoTiffError::Unsupported {
                    what: "a vertical unit given twice, differently:",
                    value: format!("{vertical_unit:?} by the GeoKeys, {unit:?} by GDAL_METADATA"),
                    hint: "",
                });
            }
            Some(unit) => (unit, true),
            None => (vertical_unit, vertical_unit_stated),
        };
        let nodata = nodata(&mut decoder)?.and_then(|v| sample.round_nodata(v));
        let chunk_type = decoder.get_chunk_type();
        // The decoder has validated its tile or strip attributes; this cannot fail after `new`.
        let (chunk_width, chunk_height) = decoder.chunk_dimensions();
        if chunk_width == 0 || chunk_height == 0 {
            return Err(GeoTiffError::Malformed {
                what: "the tile or strip size",
                reason: format!("{chunk_width} by {chunk_height} pixels"),
            });
        }
        // A strip's rows past the image's are not decoded; a tile's padding is.
        let rows = match chunk_type {
            ChunkType::Strip => chunk_height.min(height),
            ChunkType::Tile => chunk_height,
        };
        // In u128: a tile's width and length are each up to 2³² − 1, so their product times
        // 8 bytes can pass a u64.
        let chunk_bytes = u128::from(chunk_width) * u128::from(rows) * u128::from(sample.bytes());
        if chunk_bytes > u128::from(MAX_CHUNK_BYTES) {
            return Err(GeoTiffError::TooLarge {
                what: "a tile or strip, in bytes,",
                size: u64::try_from(chunk_bytes).unwrap_or(u64::MAX),
                limit: MAX_CHUNK_BYTES,
            });
        }
        // Every chunk index is below the chunk count, so `locate`'s u32 sums cannot overflow.
        let chunks = u64::from(height.div_ceil(chunk_height))
            * match chunk_type {
                ChunkType::Strip => 1,
                ChunkType::Tile => u64::from(width.div_ceil(chunk_width)),
            };
        if chunks > u64::from(u32::MAX) {
            return Err(GeoTiffError::Malformed {
                what: "the tile layout",
                reason: format!("{chunks} tiles"),
            });
        }
        no_mask(&mut decoder)?;
        Ok(ElevationRaster {
            bytes,
            info: RasterInfo {
                width,
                height,
                corner_longitude_deg: lon0,
                corner_latitude_deg: lat0,
                pixel_longitude_deg: lon_step,
                pixel_latitude_deg: lat_step,
                raster_type,
                geographic_crs_epsg,
                vertical_crs_epsg,
                vertical_unit,
                vertical_unit_stated,
                scale,
                offset,
                sample,
                nodata,
            },
            chunk_type,
            chunk_width,
            chunk_height,
        })
    }

    /// What the file says about itself.
    #[must_use]
    pub fn info(&self) -> &RasterInfo {
        &self.info
    }

    /// The ground's height at a site, metres above the file's vertical datum: the value of the
    /// pixel holding the point ([`RasterInfo::pixel_of`]) through [`RasterInfo::metres`]. `None`
    /// where that pixel is nodata.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::Location`] for a latitude outside ±90° or a longitude that is not finite,
    /// [`GeoTiffError::Outside`] off the raster, and [`GeoTiffError::Tiff`] if the pixel's tile
    /// will not decode.
    pub fn height_at(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Result<Option<f64>, GeoTiffError> {
        let value = self.value_at(latitude_deg, longitude_deg)?;
        Ok(value.map(|v| self.info.metres(v)))
    }

    /// The raw value of the pixel holding a point, before the scale, offset and unit; `None`
    /// where it is nodata. This is the value GDAL reads at the point.
    ///
    /// # Errors
    ///
    /// As [`ElevationRaster::height_at`].
    pub fn value_at(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Result<Option<f64>, GeoTiffError> {
        let pixel = self.place(latitude_deg, longitude_deg)?;
        self.pixel(pixel)
    }

    /// The raw value of a pixel, decoding only its tile or strip; `None` where it is nodata.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::NoSuchPixel`] for a pixel off the raster, [`GeoTiffError::Tiff`] if its
    /// tile will not decode.
    pub fn pixel(&self, pixel: Pixel) -> Result<Option<f64>, GeoTiffError> {
        if pixel.row >= self.info.height || pixel.col >= self.info.width {
            return Err(GeoTiffError::NoSuchPixel {
                pixel,
                width: self.info.width,
                height: self.info.height,
            });
        }
        let (index, at) = self.locate(pixel);
        let mut decoder = Decoder::new(Cursor::new(self.bytes))?;
        let chunk = decoder.read_chunk(index)?;
        let value = chunk_sample(&chunk, index, at)?;
        Ok(self.unless_nodata(value))
    }

    /// The raw values at several points, decoding each tile or strip they fall in once: what
    /// [`ElevationRaster::value_at`] gives for each, in order, its error included. A tile that
    /// will not decode fails only the points in it.
    #[must_use]
    pub fn values_at(&self, points: &[(f64, f64)]) -> Vec<Result<Option<f64>, GeoTiffError>> {
        let mut out: Vec<Result<Option<f64>, GeoTiffError>> = Vec::with_capacity(points.len());
        let mut wanted: Vec<(u32, usize, usize)> = Vec::new();
        for (i, &(latitude_deg, longitude_deg)) in points.iter().enumerate() {
            match self.place(latitude_deg, longitude_deg) {
                Ok(pixel) => {
                    let (index, at) = self.locate(pixel);
                    wanted.push((index, at, i));
                    out.push(Ok(None));
                }
                Err(e) => out.push(Err(e)),
            }
        }
        wanted.sort_unstable();
        let mut decoder = Decoder::new(Cursor::new(self.bytes)).map_err(GeoTiffError::from);
        let mut decoded: Option<(u32, Result<DecodingResult, GeoTiffError>)> = None;
        for (index, at, i) in wanted {
            if decoded.as_ref().is_none_or(|(d, _)| *d != index) {
                let chunk = match &mut decoder {
                    Ok(decoder) => decoder.read_chunk(index).map_err(GeoTiffError::from),
                    Err(e) => Err(e.clone()),
                };
                decoded = Some((index, chunk));
            }
            if let (Some((_, chunk)), Some(slot)) = (&decoded, out.get_mut(i)) {
                *slot = match chunk {
                    Ok(chunk) => chunk_sample(chunk, index, at).map(|v| self.unless_nodata(v)),
                    Err(e) => Err(e.clone()),
                };
            }
        }
        out
    }

    /// The pixel holding a point, or why there is none.
    fn place(&self, latitude_deg: f64, longitude_deg: f64) -> Result<Pixel, GeoTiffError> {
        if !(latitude_deg.abs() <= 90.0 && longitude_deg.is_finite()) {
            return Err(GeoTiffError::Location {
                latitude_deg,
                longitude_deg,
            });
        }
        self.info
            .pixel_of(latitude_deg, longitude_deg)
            .ok_or_else(|| GeoTiffError::Outside {
                latitude_deg,
                longitude_deg,
                bounds: self.info.bounds(),
            })
    }

    /// The tile or strip holding a pixel, which must be on the raster, and the pixel's place in
    /// it once decoded. A chunk decodes to its data's size, without the padding of the last
    /// tiles, so its row length is the data's width. The index is below the chunk count, which
    /// `parse` holds within a u32.
    fn locate(&self, pixel: Pixel) -> (u32, usize) {
        let Pixel { row, col } = pixel;
        let (index, chunk_row, chunk_col, data_width) = match self.chunk_type {
            ChunkType::Strip => (
                row / self.chunk_height,
                row % self.chunk_height,
                col,
                self.info.width,
            ),
            ChunkType::Tile => {
                let across = self.info.width.div_ceil(self.chunk_width);
                let col0 = (col / self.chunk_width) * self.chunk_width;
                (
                    (row / self.chunk_height) * across + col / self.chunk_width,
                    row % self.chunk_height,
                    col - col0,
                    self.chunk_width.min(self.info.width - col0),
                )
            }
        };
        let at = u64::from(chunk_row) * u64::from(data_width) + u64::from(chunk_col);
        // Under one chunk's size, which `parse` holds under MAX_CHUNK_BYTES; a value that did
        // not fit a usize would read as a short chunk.
        (index, usize::try_from(at).unwrap_or(usize::MAX))
    }

    /// Every pixel's raw value, row by row from the first; NaN where it is nodata. Each tile or
    /// strip is decoded once.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::TooLarge`] past [`MAX_VALUES_PIXELS`] or when the memory can't be had,
    /// [`GeoTiffError::Tiff`] if a tile will not decode.
    pub fn values(&self) -> Result<Vec<f64>, GeoTiffError> {
        let (width, height) = (self.info.width, self.info.height);
        let pixels = u64::from(width) * u64::from(height);
        let too_large = || GeoTiffError::TooLarge {
            what: "a raster, in pixels,",
            size: pixels,
            limit: MAX_VALUES_PIXELS,
        };
        if pixels > MAX_VALUES_PIXELS {
            return Err(too_large());
        }
        let n = usize::try_from(pixels).map_err(|_| too_large())?;
        let mut decoder = Decoder::new(Cursor::new(self.bytes))?;
        let mut out = Vec::new();
        let (across, down) = match self.chunk_type {
            ChunkType::Strip => (1, height.div_ceil(self.chunk_height)),
            ChunkType::Tile => (
                width.div_ceil(self.chunk_width),
                height.div_ceil(self.chunk_height),
            ),
        };
        let chunk_width = match self.chunk_type {
            ChunkType::Strip => width,
            ChunkType::Tile => self.chunk_width,
        };
        for chunk_row in 0..down {
            for chunk_col in 0..across {
                let index = chunk_row * across + chunk_col;
                let chunk = decoder.read_chunk(index)?;
                let col0 = chunk_col * chunk_width;
                let row0 = chunk_row * self.chunk_height;
                let data_width = chunk_width.min(width - col0);
                let data_height = self.chunk_height.min(height - row0);
                if chunk_col == 0 {
                    // The raster grows as rows of tiles decode, doubling up to its size, so a
                    // file whose tiles fail early allocates little. A tall tile grows it by its
                    // whole row; `MAX_VALUES_PIXELS` bounds that.
                    let rows = u64::from(row0 + data_height) * u64::from(width);
                    let rows = usize::try_from(rows).map_err(|_| too_large())?;
                    if rows > out.capacity() {
                        let target = rows.max(out.capacity().saturating_mul(2)).min(n);
                        out.try_reserve_exact(target - out.len())
                            .map_err(|_| too_large())?;
                    }
                    out.resize(rows, f64::NAN);
                }
                for r in 0..data_height {
                    for c in 0..data_width {
                        let at = u64::from(r) * u64::from(data_width) + u64::from(c);
                        let at = usize::try_from(at).unwrap_or(usize::MAX);
                        let value = chunk_sample(&chunk, index, at)?;
                        let to = u64::from(row0 + r) * u64::from(width) + u64::from(col0 + c);
                        // Within the rows grown above.
                        if let Some(slot) = usize::try_from(to).ok().and_then(|to| out.get_mut(to))
                        {
                            *slot = self.unless_nodata(value).unwrap_or(f64::NAN);
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn unless_nodata(&self, value: f64) -> Option<f64> {
        let nodata = self.info.nodata.is_some_and(|n| value == n);
        (!(nodata || value.is_nan())).then_some(value)
    }
}

/// Sample `at` of decoded chunk `index`, or why the chunk is short.
fn chunk_sample(chunk: &DecodingResult, index: u32, at: usize) -> Result<f64, GeoTiffError> {
    sample_at(chunk, at).ok_or_else(|| GeoTiffError::Malformed {
        what: "a tile or strip",
        reason: format!("chunk {index} holds no pixel {at}"),
    })
}

/// Sample `at` of a decoded chunk, as `f64` (exact for every type read).
fn sample_at(chunk: &DecodingResult, at: usize) -> Option<f64> {
    match chunk {
        DecodingResult::U8(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::I8(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::U16(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::I16(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::U32(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::I32(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::F32(v) => v.get(at).map(|&x| f64::from(x)),
        DecodingResult::F64(v) => v.get(at).copied(),
        // `sample_type` refuses these before any chunk is decoded.
        DecodingResult::U64(_) | DecodingResult::I64(_) | DecodingResult::F16(_) => None,
    }
}

/// Refuses a codec this build leaves out, naming it, before any pixel is read.
fn compression<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<(), GeoTiffError> {
    let code: u16 = decoder
        .find_tag_unsigned(Tag::Compression)?
        .unwrap_or(CompressionMethod::None.to_u16());
    let name = match CompressionMethod::from_u16(code) {
        Some(
            CompressionMethod::None
            | CompressionMethod::LZW
            | CompressionMethod::Deflate
            | CompressionMethod::OldDeflate
            | CompressionMethod::PackBits,
        ) => return Ok(()),
        Some(CompressionMethod::ZSTD) => "zstd",
        Some(CompressionMethod::WebP) => "WebP",
        Some(CompressionMethod::JPEG | CompressionMethod::ModernJPEG) => "JPEG",
        Some(CompressionMethod::Fax3 | CompressionMethod::Fax4 | CompressionMethod::Huffman) => {
            "fax"
        }
        _ => "an unlisted codec",
    };
    Err(GeoTiffError::Unsupported {
        what: "compression",
        value: format!("{code} ({name})"),
        hint: "; LZW, Deflate and PackBits are read: `gdal_translate -co COMPRESS=DEFLATE in.tif out.tif` rewrites it",
    })
}

/// Refuses a file holding an internal mask (an image with bit 4 of `NewSubfileType`), which
/// GDAL reads as nodata and this reader would not. The decoder is left on a later image.
fn no_mask<R: std::io::Read + std::io::Seek>(decoder: &mut Decoder<R>) -> Result<(), GeoTiffError> {
    for _ in 0..MAX_IMAGES_WALKED {
        if !decoder.more_images() {
            return Ok(());
        }
        // A later image the decoder can't read is no mask GDAL would apply either: the walk
        // stops there, the first image being the one read.
        if decoder.next_image().is_err() {
            return Ok(());
        }
        let Ok(kind) = decoder.find_tag_unsigned::<u32>(Tag::NewSubfileType) else {
            return Ok(());
        };
        let kind = kind.unwrap_or(0);
        if kind & 4 != 0 {
            return Err(GeoTiffError::Unsupported {
                what: "an internal nodata mask, NewSubfileType",
                value: kind.to_string(),
                hint: "; `gdalwarp -dstnodata <value> in.tif out.tif` turns it into a nodata value",
            });
        }
    }
    Ok(())
}

fn sample_type<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<SampleType, GeoTiffError> {
    let bits: Vec<u16> = decoder.get_tag_u16_vec(Tag::BitsPerSample)?;
    let format: u16 = decoder
        .find_tag_unsigned_vec::<u16>(Tag::SampleFormat)?
        .and_then(|v| v.first().copied())
        .unwrap_or(SampleFormat::Uint.to_u16());
    let bits = bits.first().copied().unwrap_or(1);
    let uint = SampleFormat::Uint.to_u16();
    let int = SampleFormat::Int.to_u16();
    let float = SampleFormat::IEEEFP.to_u16();
    Ok(match (format, bits) {
        (f, 8) if f == uint => SampleType::U8,
        (f, 8) if f == int => SampleType::I8,
        (f, 16) if f == uint => SampleType::U16,
        (f, 16) if f == int => SampleType::I16,
        (f, 32) if f == uint => SampleType::U32,
        (f, 32) if f == int => SampleType::I32,
        (f, 32) if f == float => SampleType::F32,
        (f, 64) if f == float => SampleType::F64,
        (f, b) => {
            return Err(GeoTiffError::Unsupported {
                what: "a sample of format and bits",
                value: format!("{f}, {b}"),
                hint: "",
            });
        }
    })
}

/// The GeoKey directory's SHORT keys (OGC 19-008r4 §7.1.2): those stored in the directory itself.
struct GeoKeys {
    /// The directory's minor revision: 0 for GeoTIFF 1.0, 1 for 1.1.
    minor: u16,
    /// (key, location, value): location 0 means `value` is the key's value.
    entries: Vec<(u16, u16, u16)>,
}

impl GeoKeys {
    fn read<R: std::io::Read + std::io::Seek>(
        decoder: &mut Decoder<R>,
    ) -> Result<Self, GeoTiffError> {
        let Some(dir) = decoder.find_tag_unsigned_vec::<u16>(Tag::GeoKeyDirectoryTag)? else {
            return Err(GeoTiffError::Missing {
                what: "GeoKeyDirectoryTag (34735), so it is not a GeoTIFF",
            });
        };
        let malformed = |reason: String| GeoTiffError::Malformed {
            what: "GeoKeyDirectoryTag",
            reason,
        };
        let [version, revision, minor, count, ..] = dir[..] else {
            return Err(malformed(format!(
                "{} values, under the header's 4",
                dir.len()
            )));
        };
        if version != 1 || revision != 1 {
            return Err(malformed(format!(
                "version {version}, revision {revision}; the standard's are 1 and 1"
            )));
        }
        let needed = 4 + 4 * usize::from(count);
        if dir.len() < needed {
            return Err(malformed(format!(
                "{count} keys need {needed} values, it has {}",
                dir.len()
            )));
        }
        let entries = dir[4..needed]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|e| (e[0], e[1], e[3]))
            .collect();
        Ok(GeoKeys { minor, entries })
    }

    /// Whether the directory holds `key` at all.
    fn has(&self, key: u16) -> bool {
        self.entries.iter().any(|e| e.0 == key)
    }

    /// A SHORT key's value, `None` if absent. A key repeated is refused.
    fn get(&self, key: u16) -> Result<Option<u16>, GeoTiffError> {
        let mut found = self.entries.iter().filter(|e| e.0 == key);
        let Some(&(_, location, value)) = found.next() else {
            return Ok(None);
        };
        if found.next().is_some() {
            return Err(GeoTiffError::Malformed {
                what: "GeoKeyDirectoryTag",
                reason: format!("key {key} appears twice"),
            });
        }
        if location != 0 {
            return Err(GeoTiffError::Malformed {
                what: "GeoKeyDirectoryTag",
                reason: format!("key {key} is a SHORT but is stored in tag {location}"),
            });
        }
        Ok(Some(value))
    }
}

fn geographic_crs(keys: &GeoKeys) -> Result<u16, GeoTiffError> {
    let projected = keys.get(PROJECTED_CRS_KEY)?;
    let geodetic = keys.get(GEODETIC_CRS_KEY)?;
    match keys.get(MODEL_TYPE_KEY)? {
        Some(2) => {}
        // GeoTIFF 1.0 files from some writers name a geodetic CRS without a model type.
        None if projected.is_none() && geodetic.is_some() => {}
        None if projected.is_none() => {
            return Err(GeoTiffError::Missing {
                what: "GTModelTypeGeoKey (1024) or a geodetic CRS (2048)",
            });
        }
        Some(1) | None => {
            return Err(GeoTiffError::Unsupported {
                what: "a projected CRS, EPSG",
                value: projected.map_or_else(|| "unnamed".into(), |c| c.to_string()),
                hint: "; reproject it to latitude and longitude, as with `gdalwarp -t_srs EPSG:4326 in.tif out.tif`",
            });
        }
        Some(other) => {
            return Err(GeoTiffError::Unsupported {
                what: "GTModelTypeGeoKey",
                value: other.to_string(),
                hint: "; only a geographic CRS (2) is read",
            });
        }
    }
    let Some(code) = geodetic.filter(|c| NEAR_WGS84.iter().any(|(near, _)| near == c)) else {
        return Err(GeoTiffError::Unsupported {
            what: "a geographic CRS, EPSG",
            value: geodetic.map_or_else(|| "unnamed".into(), |c| c.to_string()),
            hint: "; only datums within a few metres of WGS 84 are read (`NEAR_WGS84`): `gdalwarp -t_srs EPSG:4326 in.tif out.tif` converts it",
        });
    };
    match keys.get(ANGULAR_UNITS_KEY)? {
        // 9102 is EPSG's degree, 9122 its degree as a CRS's unit.
        None | Some(9102 | 9122) => {}
        Some(other) => {
            return Err(GeoTiffError::Unsupported {
                what: "GeogAngularUnitsGeoKey",
                value: other.to_string(),
                hint: "; only degrees are read",
            });
        }
    }
    match keys.get(PRIME_MERIDIAN_KEY)? {
        None => {}
        Some(GREENWICH) => {}
        Some(other) => {
            return Err(GeoTiffError::Unsupported {
                what: "a prime meridian, EPSG",
                value: other.to_string(),
                hint: "; only Greenwich (8901) is read",
            });
        }
    }
    Ok(code)
}

fn vertical(keys: &GeoKeys) -> Result<(Option<u16>, VerticalUnit, bool), GeoTiffError> {
    // 0 is "undefined", 32767 "user-defined" and those above private: none is an EPSG code.
    let crs = keys
        .get(VERTICAL_CRS_KEY)?
        .filter(|c| (1..32767).contains(c));
    let user_defined = keys.get(VERTICAL_CRS_KEY)?.is_some_and(|c| c >= 32767);
    let stated = match keys.get(VERTICAL_UNITS_KEY)? {
        Some(code) => Some(
            VerticalUnit::from_epsg(code).ok_or(GeoTiffError::Unsupported {
                what: "VerticalUnitsGeoKey",
                value: code.to_string(),
                hint: "; metres (9001), feet (9002) and US survey feet (9003) are read",
            })?,
        ),
        None => None,
    };
    let from_crs = crs.and_then(|c| {
        VERTICAL_CRS_UNITS
            .iter()
            .find(|(code, _)| *code == c)
            .map(|&(_, unit)| unit)
    });
    match (stated, from_crs, crs) {
        // GDAL takes a known CRS's unit and ignores the key; the file's writer may have meant
        // the key. Neither reading is safe.
        (Some(key), Some(of_crs), Some(code)) if key != of_crs => Err(GeoTiffError::Unsupported {
            what: "a vertical unit given twice, differently:",
            value: format!(
                "{key:?} by VerticalUnitsGeoKey, {of_crs:?} by vertical CRS EPSG:{code}"
            ),
            hint: "",
        }),
        (Some(unit), _, _) | (None, Some(unit), _) => Ok((crs, unit, true)),
        (None, None, None) if user_defined => Err(GeoTiffError::Unsupported {
            what: "a user-defined vertical CRS without VerticalUnitsGeoKey",
            value: "VerticalGeoKey 32767 or above".to_string(),
            hint: "; its unit is unknown here",
        }),
        (None, None, None) => Ok((None, VerticalUnit::Metre, false)),
        // A vertical CRS this reader does not know, with no unit: its unit could be feet.
        (None, None, Some(code)) => Err(GeoTiffError::Unsupported {
            what: "a vertical CRS without VerticalUnitsGeoKey, EPSG",
            value: code.to_string(),
            hint: "; its unit is unknown here",
        }),
    }
}

/// The georeferencing: `corner` is `[λ₀, Δλ, φ₀, Δφ]` for pixel is area, and `z` the pixel
/// scale's `S_z` and the tiepoint's `z₀` and `Z₀`, when the file has them.
struct Georef {
    corner: [f64; 4],
    z: Option<[f64; 3]>,
}

/// The georeferencing from the tiepoint and pixel scale or the transformation matrix (the module
/// docs).
fn transform<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<Georef, GeoTiffError> {
    let scale = decoder.find_tag(Tag::ModelPixelScaleTag)?;
    let tiepoint = decoder.find_tag(Tag::ModelTiepointTag)?;
    let matrix = decoder.find_tag(Tag::ModelTransformationTag)?;
    let doubles = |what: &'static str, v: tiff::decoder::ifd::Value| {
        v.into_f64_vec().map_err(|e| GeoTiffError::Malformed {
            what,
            reason: e.to_string(),
        })
    };
    let out = match (scale, tiepoint, matrix) {
        (Some(_), _, Some(_)) => {
            return Err(GeoTiffError::Malformed {
                what: "the georeferencing",
                reason: "ModelPixelScaleTag and ModelTransformationTag together, which the standard forbids (§7.1.1)".into(),
            });
        }
        (Some(scale), Some(tiepoint), None) => {
            let scale = doubles("ModelPixelScaleTag", scale)?;
            let tie = doubles("ModelTiepointTag", tiepoint)?;
            let [sx, sy, ref rest @ ..] = scale[..] else {
                return Err(GeoTiffError::Malformed {
                    what: "ModelPixelScaleTag",
                    reason: format!("{} values, not 3", scale.len()),
                });
            };
            let [i, j, z0, x, y, z] = tie[..] else {
                return Err(GeoTiffError::Unsupported {
                    what: "ModelTiepointTag with values numbering",
                    value: tie.len().to_string(),
                    hint: "; one tiepoint (6 values) with a pixel scale is read, not ground control points",
                });
            };
            if sy < 0.0 {
                return Err(GeoTiffError::Unsupported {
                    what: "a negative ModelPixelScaleTag Y,",
                    value: sy.to_string(),
                    hint: "; the standard reads it south-up and GDAL north-up",
                });
            }
            // GDAL's order of operations, so a corner reads as GDAL reads it.
            let dlat = -sy;
            Georef {
                corner: [x - i * sx, sx, y - j * dlat, dlat],
                z: rest.first().map(|&sz| [sz, z0, z]),
            }
        }
        (None, _, Some(matrix)) => {
            let m = doubles("ModelTransformationTag", matrix)?;
            // The standard's letters: X = a·I + b·J + d, Y = e·I + f·J + h.
            let [a, b, _c, d, e, f, _g, h, ..] = m[..] else {
                return Err(GeoTiffError::Malformed {
                    what: "ModelTransformationTag",
                    reason: format!("{} values, not 16", m.len()),
                });
            };
            if m.len() != 16 {
                return Err(GeoTiffError::Malformed {
                    what: "ModelTransformationTag",
                    reason: format!("{} values, not 16", m.len()),
                });
            }
            if b != 0.0 || e != 0.0 {
                return Err(GeoTiffError::Unsupported {
                    what: "a rotated or sheared raster, with terms b and e",
                    value: format!("{b}, {e}"),
                    hint: "; `gdalwarp` turns it north-up",
                });
            }
            Georef {
                corner: [d, a, h, f],
                z: None,
            }
        }
        (None, _, None) => {
            return Err(GeoTiffError::Missing {
                what: "ModelPixelScaleTag or ModelTransformationTag",
            });
        }
        (Some(_), None, None) => {
            return Err(GeoTiffError::Missing {
                what: "ModelTiepointTag",
            });
        }
    };
    let c = out.corner;
    if !(c.iter().all(|v| v.is_finite()) && c[1] != 0.0 && c[3] != 0.0) {
        return Err(GeoTiffError::Malformed {
            what: "the georeferencing",
            reason: format!(
                "corner ({}, {}) and pixel size ({}, {})",
                c[0], c[2], c[1], c[3]
            ),
        });
    }
    Ok(out)
}

/// Whether GDAL takes a scale and offset from `S_z` and the tiepoint's heights. It does for one
/// band when its CRS is vertical, which depends on the directory's revision and on how GDAL and
/// PROJ resolve the vertical keys (GDAL 3.12.2's `gt_wkt_srs.cpp` drops the vertical CRS for a
/// private key value, for `VerticalDatumGeoKey` 6030 beside WGS 84, and beside WGS 84 3D).
/// Certain: applied for a GeoTIFF 1.1 directory naming a vertical CRS this reader knows, with no
/// datum key, beside any geographic CRS but WGS 84 3D; ignored for a 1.0 directory (GDAL drops
/// its vertical CRS, rasterio 1.5.2 shows) and for one with no vertical key. Anything between is
/// refused, unless the tags hold GDAL's own `S_z` 1 and offset 0, which read the same either way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ZTerms {
    Applied,
    Ignored,
    Unknown,
}

/// The pixels' scale and offset as GDAL sets them (the module docs), and `GDAL_METADATA`'s unit
/// if it gives one: from the pixel scale's `S_z` and the tiepoint's heights where [`ZTerms`]
/// applies them, otherwise from `GDAL_METADATA`.
fn scale_offset<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
    georef: &Georef,
    heights: ZTerms,
) -> Result<(f64, f64, Option<VerticalUnit>), GeoTiffError> {
    let from_tags = match georef.z {
        // GDAL's rule: one band, a vertical CRS, and any of S_z, z₀, Z₀ non-zero.
        Some([sz, z0, z]) if sz != 0.0 || z0 != 0.0 || z != 0.0 => match heights {
            ZTerms::Applied => Some((sz, z - z0 * sz)),
            ZTerms::Ignored => None,
            ZTerms::Unknown if (sz, z - z0 * sz) == (1.0, 0.0) => None,
            ZTerms::Unknown => {
                return Err(GeoTiffError::Unsupported {
                    what: "heights in ModelPixelScaleTag or ModelTiepointTag,",
                    value: format!("S_z {sz}, z₀ {z0}, Z₀ {z}"),
                    hint: ", with vertical keys GDAL may or may not read as a vertical CRS",
                });
            }
        },
        _ => None,
    };
    let metadata = gdal_metadata(decoder)?;
    let from_metadata = (metadata.scale.is_some() || metadata.offset.is_some()).then(|| {
        (
            metadata.scale.unwrap_or(1.0),
            metadata.offset.unwrap_or(0.0),
        )
    });
    let (scale, offset) = match (from_tags, from_metadata) {
        (Some(tags), Some(meta)) if tags != meta => {
            return Err(GeoTiffError::Unsupported {
                what: "a pixel scale and offset given twice, differently:",
                value: format!("{tags:?} by the tags, {meta:?} by GDAL_METADATA"),
                hint: "",
            });
        }
        (Some(pair), _) | (None, Some(pair)) => pair,
        (None, None) => (1.0, 0.0),
    };
    if !(scale.is_finite() && scale != 0.0 && offset.is_finite()) {
        return Err(GeoTiffError::Malformed {
            what: "the pixel scale and offset",
            reason: format!("scale {scale}, offset {offset}"),
        });
    }
    Ok((scale, offset, metadata.unit))
}

/// What GDAL's `GDAL_METADATA` tag says of band 1.
#[derive(Default)]
struct Metadata {
    scale: Option<f64>,
    offset: Option<f64>,
    unit: Option<VerticalUnit>,
}

/// The deepest nesting `GDAL_METADATA` may have: GDAL writes a root and its items, two levels.
const MAX_METADATA_DEPTH: usize = 16;

/// A unit as GDAL's `unittype` item names it; `None` for an empty name.
fn unit_type(name: &str) -> Result<Option<VerticalUnit>, GeoTiffError> {
    let lower = name.trim().to_ascii_lowercase();
    Ok(Some(match lower.as_str() {
        "" => return Ok(None),
        "m" | "metre" | "meter" | "metres" | "meters" => VerticalUnit::Metre,
        "ft" | "foot" | "feet" | "international foot" => VerticalUnit::Foot,
        "us survey foot" | "us survey feet" | "ftus" | "us-ft" => VerticalUnit::UsSurveyFoot,
        _ => {
            return Err(GeoTiffError::Unsupported {
                what: "a GDAL_METADATA unit type",
                value: format!("{name:?}"),
                hint: "; metres, feet and US survey feet are read",
            });
        }
    }))
}

/// The first band's `scale`, `offset` and `unittype` items in GDAL's `GDAL_METADATA` XML.
fn gdal_metadata<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<Metadata, GeoTiffError> {
    let Some(value) = decoder.find_tag(Tag::Unknown(GDAL_METADATA_TAG))? else {
        return Ok(Metadata::default());
    };
    let malformed = |reason: String| GeoTiffError::Malformed {
        what: "GDAL_METADATA",
        reason,
    };
    let text = value.into_string().map_err(|e| malformed(e.to_string()))?;
    let text = text.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    // roxmltree recurses on nesting; the tag is the file's, so its depth is bounded first.
    let depth = crate::ork::document::deepest_nesting(text);
    if depth > MAX_METADATA_DEPTH {
        return Err(malformed(format!(
            "nested {depth} deep, past {MAX_METADATA_DEPTH}"
        )));
    }
    let document = roxmltree::Document::parse(text).map_err(|e| malformed(e.to_string()))?;
    let mut metadata = Metadata::default();
    let root = document.root_element();
    let named = |n: &roxmltree::Node, name: &str| {
        n.is_element() && n.tag_name().name().eq_ignore_ascii_case(name)
    };
    if !named(&root, "GDALMetadata") {
        return Ok(metadata);
    }
    // As GDAL 3.12.2 matches them (`gtiffdataset_read.cpp`): an `Item`, either case, with a name,
    // a sample that C's `atoi` reads as 0, any domain but IMAGE_STRUCTURE, and some text.
    for item in root.children().filter(|n| {
        named(n, "Item")
            && n.attribute("name").is_some()
            && !n
                .attribute("domain")
                .is_some_and(|d| d.eq_ignore_ascii_case("IMAGE_STRUCTURE"))
            && n.attribute("sample").is_some_and(|s| atoi(s) == 0)
    }) {
        let text = item.text().unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        let slot = match item.attribute("role") {
            Some(role) if role.eq_ignore_ascii_case("scale") => &mut metadata.scale,
            Some(role) if role.eq_ignore_ascii_case("offset") => &mut metadata.offset,
            Some(role) if role.eq_ignore_ascii_case("unittype") => {
                metadata.unit = unit_type(text)?;
                continue;
            }
            _ => continue,
        };
        *slot = Some(
            text.parse::<f64>()
                .map_err(|_| malformed(format!("{text:?} is not a number")))?,
        );
    }
    Ok(metadata)
}

/// C's `atoi`: leading blanks, a sign, then digits as far as they go; 0 if there are none.
fn atoi(text: &str) -> i64 {
    let text = text.trim_start();
    let (negative, digits) = match text.as_bytes().first() {
        Some(b'-') => (true, &text[1..]),
        Some(b'+') => (false, &text[1..]),
        _ => (false, text),
    };
    let value = digits
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0_i64, |v, d| {
            v.saturating_mul(10).saturating_add(i64::from(d - b'0'))
        });
    if negative { -value } else { value }
}

fn nodata<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<Option<f64>, GeoTiffError> {
    let Some(value) = decoder.find_tag(Tag::Unknown(GDAL_NODATA_TAG))? else {
        return Ok(None);
    };
    let text = value.into_string().map_err(|e| GeoTiffError::Malformed {
        what: "GDAL_NODATA",
        reason: e.to_string(),
    })?;
    let text = text.trim_matches(|c: char| c == '\0' || c.is_whitespace());
    text.parse::<f64>()
        .map(Some)
        .map_err(|_| GeoTiffError::Malformed {
            what: "GDAL_NODATA",
            reason: format!("{text:?} is not a number"),
        })
}

#[cfg(test)]
mod tests;
