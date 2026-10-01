//! A launch site's height from a user's elevation file: a GeoTIFF digital elevation model (DEM)
//! on a geographic (latitude and longitude) grid.
//!
//! A GeoTIFF is a TIFF image whose pixels are heights and whose tags say where on Earth they lie.
//! [`ElevationRaster::parse`] reads the tags; [`ElevationRaster::height_at`] finds the pixel a
//! latitude and longitude fall in and decodes only the tile or strip that holds it, so a lookup in
//! a large file costs one tile's memory. The TIFF itself (codecs, predictors, tiles, strips, byte
//! order, BigTIFF) is decoded by image-rs's [`tiff`] crate; this module reads the geographic tags
//! as the OGC GeoTIFF Standard 1.1 (OGC 19-008r4, 2019) defines them, and places points in pixels
//! the way GDAL does, so a height read here is the value GDAL's readers return for the same point.
//!
//! **Where a pixel lies.** The file ties raster coordinates to longitude and latitude with a
//! tiepoint `(I, J) ↦ (X, Y)` and a pixel size `(S_x, S_y)` (`ModelTiepointTag`,
//! `ModelPixelScaleTag`; §7.3), or with an affine matrix (`ModelTransformationTag`). A positive
//! `S_y` means latitude falls as rows go down (Requirement 10.4). As GDAL does, both become the
//! longitude and latitude of the outer corner of pixel `(0, 0)` and a signed pixel size:
//!
//! `λ₀ = X − I·S_x`, `φ₀ = Y + J·S_y`, `Δλ = S_x`, `Δφ = −S_y`.
//!
//! The raster type (`GTRasterTypeGeoKey`, §7.2.1) says what a raster coordinate names. In the
//! usual *pixel is area*, `(0, 0)` is the outer corner of the first pixel; in *pixel is point* it
//! is that pixel's centre, so the corner is half a pixel back: `λ₀ −= Δλ/2`, `φ₀ −= Δφ/2`. A point
//! `(φ, λ)` then lies in column `⌊(λ − λ₀)/Δλ⌋` and row `⌊(φ − φ₀)/Δφ⌋`: the pixel whose area
//! holds it, with no interpolation between pixels. On a 1-arc-second grid (about 31 m by 26 m at
//! 33° N) that is the height of the ground within about 20 m of the site.
//!
//! **What is read.** One band of unsigned or signed 8-, 16- or 32-bit integers, or 32- or 64-bit
//! floats; uncompressed, LZW, Deflate or PackBits, with or without the horizontal or
//! floating-point predictor; tiles or strips; little- or big-endian; classic TIFF or BigTIFF.
//! The first image in the file is the one read (later ones are a cloud-optimized GeoTIFF's
//! overviews and masks). The CRS must be geographic (`GTModelTypeGeoKey` 2) in degrees; its EPSG
//! code is reported and not otherwise used: a point is read in the file's own datum, and the
//! common ones (WGS 84, NAD83, ETRS89) lie within about 2 m of each other, under a 10 m pixel.
//! A projected file (UTM, say) is refused with [`GeoTiffError::Unsupported`] naming its code;
//! `gdalwarp -t_srs EPSG:4326 in.tif out.tif` turns it into one this reads.
//!
//! **Heights.** A pixel's value is converted to metres by the vertical unit the file states
//! (`VerticalUnitsGeoKey`: metres, international feet or US survey feet), or by the unit of a
//! vertical CRS this module knows; a file that states neither is read as metres, as GDAL reads
//! it. The vertical datum (`VerticalGeoKey`, NAVD88 or EGM2008, say) is reported, not applied.
//! The pixel-value scale `S_z` in `ModelPixelScaleTag` is ignored, as GDAL ignores it. A value
//! equal to the file's nodata value (GDAL's `GDAL_NODATA` tag, rounded to the sample type) or a
//! NaN reads as no height.
//!
//! **Guide:** [A launch site's elevation][guide] walks through an example and says how the reader
//! is checked: against GDAL's reading, through rasterio, of five files and a whole USGS tile
//! (ADR-128).
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/elevation.html#from-an-elevation-file-of-your-own

use std::io::Cursor;

use serde::{Deserialize, Serialize};
use tiff::decoder::{ChunkType, Decoder, DecodingResult};
use tiff::tags::{PhotometricInterpretation, SampleFormat, Tag};

/// `GTModelTypeGeoKey` (OGC 19-008r4 §7.2.2): 1 projected, 2 geographic, 3 geocentric.
const MODEL_TYPE_KEY: u16 = 1024;
/// `GTRasterTypeGeoKey` (§7.2.1): 1 pixel is area, 2 pixel is point.
const RASTER_TYPE_KEY: u16 = 1025;
/// `GeodeticCRSGeoKey` (§7.4.3), `GeographicTypeGeoKey` in GeoTIFF 1.0.
const GEODETIC_CRS_KEY: u16 = 2048;
/// `GeogAngularUnitsGeoKey` (§7.5.1).
const ANGULAR_UNITS_KEY: u16 = 2054;
/// `ProjectedCRSGeoKey` (§7.4.2).
const PROJECTED_CRS_KEY: u16 = 3072;
/// `VerticalGeoKey` (§7.4.4), `VerticalCSTypeGeoKey` in GeoTIFF 1.0.
const VERTICAL_CRS_KEY: u16 = 4096;
/// `VerticalUnitsGeoKey` (§7.5.1).
const VERTICAL_UNITS_KEY: u16 = 4099;
/// The `GDAL_NODATA` TIFF tag: the nodata value as ASCII text.
const GDAL_NODATA_TAG: u16 = 42113;
/// EPSG's user-defined code, which names no CRS.
const USER_DEFINED: u16 = 32767;

/// The most pixels [`ElevationRaster::values`] decodes into one vector: 2²⁸, 2 GiB of `f64`.
pub const MAX_VALUES_PIXELS: u64 = 1 << 28;

/// Why a GeoTIFF elevation file could not be read, or a height not taken from it.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
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
        "latitude {latitude_deg}° and longitude {longitude_deg}° are outside the raster, which spans latitudes {south_deg}° to {north_deg}° and longitudes {west_deg}° to {east_deg}°"
    )]
    Outside {
        /// Latitude asked for, degrees.
        latitude_deg: f64,
        /// Longitude asked for, degrees.
        longitude_deg: f64,
        /// The raster's southern edge, degrees.
        south_deg: f64,
        /// Its northern edge, degrees.
        north_deg: f64,
        /// Its western edge, degrees.
        west_deg: f64,
        /// Its eastern edge, degrees.
        east_deg: f64,
    },
    /// A pixel past the raster's last row or column.
    #[error("pixel (row {row}, column {col}) is outside a raster of {width} by {height}")]
    NoSuchPixel {
        /// Row asked for.
        row: u32,
        /// Column asked for.
        col: u32,
        /// The raster's columns.
        width: u32,
        /// Its rows.
        height: u32,
    },
    /// The raster is too large to decode into one vector ([`MAX_VALUES_PIXELS`]).
    #[error("{pixels} pixels is more than the {limit} decoded at once")]
    TooLarge {
        /// Width times height.
        pixels: u64,
        /// [`MAX_VALUES_PIXELS`].
        limit: u64,
    },
}

impl From<tiff::TiffError> for GeoTiffError {
    fn from(e: tiff::TiffError) -> Self {
        GeoTiffError::Tiff(e.to_string())
    }
}

/// What a raster coordinate names (`GTRasterTypeGeoKey`, OGC 19-008r4 §7.2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RasterType {
    /// Raster coordinate `(0, 0)` is the outer corner of the first pixel (code 1, the default).
    PixelIsArea,
    /// Raster coordinate `(0, 0)` is the centre of the first pixel (code 2).
    PixelIsPoint,
}

/// The unit a pixel's value is in, converted to metres by [`VerticalUnit::metres`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
/// unit, from EPSG's registry: (code, unit).
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
            // Rounded to nearest, as GDAL casts it; a NaN stays NaN and is matched by `is_nan`.
            #[expect(clippy::cast_possible_truncation, reason = "f32 nodata is f32-rounded")]
            SampleType::F32 => Some(f64::from(nodata as f32)),
            SampleType::F64 => Some(nodata),
        }
    }
}

/// What an elevation file says about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// The geographic CRS's EPSG code, or `None` for a user-defined one.
    pub geographic_crs_epsg: Option<u16>,
    /// The vertical CRS's EPSG code, if the file names one.
    pub vertical_crs_epsg: Option<u16>,
    /// The unit pixel values are in.
    pub vertical_unit: VerticalUnit,
    /// Whether the file states that unit (by `VerticalUnitsGeoKey` or a vertical CRS this reader
    /// knows); `false` means metres were assumed.
    pub vertical_unit_stated: bool,
    /// The pixels' sample type.
    pub sample: SampleType,
    /// The nodata value, rounded to the sample type, if the file has one a sample can hold.
    pub nodata: Option<f64>,
}

impl RasterInfo {
    /// The column and row of the pixel holding a point, by GDAL's rule (the module docs), or
    /// `None` if the point is off the raster. The longitude is tried as given and then 360° to
    /// either side, so a file on 0° to 360° reads a longitude given on −180° to 180°.
    #[must_use]
    pub fn pixel_of(&self, latitude_deg: f64, longitude_deg: f64) -> Option<(u32, u32)> {
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
            .map(|col| (col, row))
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

    /// The raster's edges, degrees: (south, north, west, east).
    #[must_use]
    pub fn bounds_deg(&self) -> (f64, f64, f64, f64) {
        let lat_far = self.corner_latitude_deg + f64::from(self.height) * self.pixel_latitude_deg;
        let lon_far = self.corner_longitude_deg + f64::from(self.width) * self.pixel_longitude_deg;
        (
            self.corner_latitude_deg.min(lat_far),
            self.corner_latitude_deg.max(lat_far),
            self.corner_longitude_deg.min(lon_far),
            self.corner_longitude_deg.max(lon_far),
        )
    }
}

/// A GeoTIFF elevation file, its tags read and its pixels left packed in the borrowed bytes.
#[derive(Debug, Clone)]
pub struct ElevationRaster<'a> {
    bytes: &'a [u8],
    info: RasterInfo,
    chunk_type: ChunkType,
    chunk_width: u32,
    chunk_height: u32,
}

impl<'a> ElevationRaster<'a> {
    /// Reads a GeoTIFF's tags: its size, sample type, layout, georeferencing and nodata value.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError`] if the TIFF is unreadable, its georeferencing is missing, malformed or
    /// rotated, its CRS is not geographic in degrees, or its pixels are not one band of a sample
    /// type [`SampleType`] lists.
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
        let [mut lon0, lon_step, mut lat0, lat_step] = transform(&mut decoder)?;
        if raster_type == RasterType::PixelIsPoint {
            lon0 -= lon_step * 0.5;
            lat0 -= lat_step * 0.5;
        }
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
        // Every chunk index is below the chunk count, so `locate`'s u32 sums cannot overflow.
        let chunks = match chunk_type {
            ChunkType::Strip => u64::from(height.div_ceil(chunk_height)),
            ChunkType::Tile => {
                u64::from(width.div_ceil(chunk_width)) * u64::from(height.div_ceil(chunk_height))
            }
        };
        if chunks > u64::from(u32::MAX) {
            return Err(GeoTiffError::Malformed {
                what: "the tile layout",
                reason: format!("{chunks} tiles"),
            });
        }
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

    /// The ground's height at a site, metres in the file's vertical datum: the value of the pixel
    /// holding the point ([`RasterInfo::pixel_of`]) times the vertical unit. `None` where that
    /// pixel is nodata.
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
        Ok(value.map(|v| v * self.info.vertical_unit.metres()))
    }

    /// The raw value of the pixel holding a point, in the file's vertical unit; `None` where it
    /// is nodata. This is the value GDAL reads at the point.
    ///
    /// # Errors
    ///
    /// As [`ElevationRaster::height_at`].
    pub fn value_at(
        &self,
        latitude_deg: f64,
        longitude_deg: f64,
    ) -> Result<Option<f64>, GeoTiffError> {
        let (col, row) = self.place(latitude_deg, longitude_deg)?;
        self.pixel(row, col)
    }

    /// The raw value of pixel `(row, col)`, decoding only its tile or strip; `None` where it is
    /// nodata.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::NoSuchPixel`] for a pixel off the raster, [`GeoTiffError::Tiff`] if its
    /// tile will not decode.
    pub fn pixel(&self, row: u32, col: u32) -> Result<Option<f64>, GeoTiffError> {
        if row >= self.info.height || col >= self.info.width {
            return Err(GeoTiffError::NoSuchPixel {
                row,
                col,
                width: self.info.width,
                height: self.info.height,
            });
        }
        let (index, at) = self.locate(row, col);
        let mut decoder = Decoder::new(Cursor::new(self.bytes))?;
        let chunk = decoder.read_chunk(index)?;
        let value = chunk_sample(&chunk, index, at)?;
        Ok(self.unless_nodata(value))
    }

    /// The raw values at several points, decoding each tile or strip they fall in once: what
    /// [`ElevationRaster::value_at`] gives for each, in order.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::Tiff`] if a tile will not decode; a point's own error (off the raster,
    /// not a place) is its entry's.
    pub fn values_at(
        &self,
        points: &[(f64, f64)],
    ) -> Result<Vec<Result<Option<f64>, GeoTiffError>>, GeoTiffError> {
        let mut out: Vec<Result<Option<f64>, GeoTiffError>> = Vec::with_capacity(points.len());
        let mut wanted: Vec<(u32, usize, usize)> = Vec::new();
        for (i, &(latitude_deg, longitude_deg)) in points.iter().enumerate() {
            match self.place(latitude_deg, longitude_deg) {
                Ok((col, row)) => {
                    let (index, at) = self.locate(row, col);
                    wanted.push((index, at, i));
                    out.push(Ok(None));
                }
                Err(e) => out.push(Err(e)),
            }
        }
        wanted.sort_unstable();
        let mut decoder = Decoder::new(Cursor::new(self.bytes))?;
        let mut decoded: Option<(u32, DecodingResult)> = None;
        for (index, at, i) in wanted {
            if decoded.as_ref().is_none_or(|(d, _)| *d != index) {
                decoded = Some((index, decoder.read_chunk(index)?));
            }
            if let (Some((_, chunk)), Some(slot)) = (&decoded, out.get_mut(i)) {
                *slot = Ok(self.unless_nodata(chunk_sample(chunk, index, at)?));
            }
        }
        Ok(out)
    }

    /// The pixel holding a point, or why there is none.
    fn place(&self, latitude_deg: f64, longitude_deg: f64) -> Result<(u32, u32), GeoTiffError> {
        if !(latitude_deg.abs() <= 90.0 && longitude_deg.is_finite()) {
            return Err(GeoTiffError::Location {
                latitude_deg,
                longitude_deg,
            });
        }
        self.info
            .pixel_of(latitude_deg, longitude_deg)
            .ok_or_else(|| {
                let (south_deg, north_deg, west_deg, east_deg) = self.info.bounds_deg();
                GeoTiffError::Outside {
                    latitude_deg,
                    longitude_deg,
                    south_deg,
                    north_deg,
                    west_deg,
                    east_deg,
                }
            })
    }

    /// The tile or strip holding pixel `(row, col)`, which must be on the raster, and the
    /// pixel's place in it once decoded. A chunk decodes to its data's size, without the
    /// padding of the last tiles, so its row length is the data's width. The index is below the
    /// chunk count, which `parse` holds within a u32.
    fn locate(&self, row: u32, col: u32) -> (u32, usize) {
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
        // Under one chunk's size, which the decoder's 256 MiB limit holds within a usize; a
        // value that did not fit reads as a short chunk.
        (index, usize::try_from(at).unwrap_or(usize::MAX))
    }

    /// Every pixel's raw value, row by row from the first; NaN where it is nodata. Each tile or
    /// strip is decoded once.
    ///
    /// # Errors
    ///
    /// [`GeoTiffError::TooLarge`] past [`MAX_VALUES_PIXELS`], [`GeoTiffError::Tiff`] if a tile
    /// will not decode.
    pub fn values(&self) -> Result<Vec<f64>, GeoTiffError> {
        let (width, height) = (self.info.width, self.info.height);
        let pixels = u64::from(width) * u64::from(height);
        if pixels > MAX_VALUES_PIXELS {
            return Err(GeoTiffError::TooLarge {
                pixels,
                limit: MAX_VALUES_PIXELS,
            });
        }
        // At most 2^28 by the check above, so it fits a usize on every target.
        let mut out = vec![f64::NAN; usize::try_from(pixels).unwrap_or(usize::MAX)];
        let mut decoder = Decoder::new(Cursor::new(self.bytes))?;
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
                for r in 0..data_height {
                    for c in 0..data_width {
                        let at = u64::from(r) * u64::from(data_width) + u64::from(c);
                        let at = usize::try_from(at).unwrap_or(usize::MAX);
                        let value = chunk_sample(&chunk, index, at)?;
                        let to = u64::from(row0 + r) * u64::from(width) + u64::from(col0 + c);
                        // Within `pixels`, which fits a usize (above).
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
        let [version, revision, _minor, count, ..] = dir[..] else {
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
        Ok(GeoKeys { entries })
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

fn geographic_crs(keys: &GeoKeys) -> Result<Option<u16>, GeoTiffError> {
    let projected = keys.get(PROJECTED_CRS_KEY)?;
    match keys.get(MODEL_TYPE_KEY)? {
        Some(2) => {}
        // GeoTIFF 1.0 files from some writers name a geodetic CRS without a model type.
        None if projected.is_none() && keys.get(GEODETIC_CRS_KEY)?.is_some() => {}
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
    Ok(keys.get(GEODETIC_CRS_KEY)?.filter(|&c| c != USER_DEFINED))
}

fn vertical(keys: &GeoKeys) -> Result<(Option<u16>, VerticalUnit, bool), GeoTiffError> {
    let crs = keys.get(VERTICAL_CRS_KEY)?.filter(|&c| c != USER_DEFINED);
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
        (Some(unit), _, _) => Ok((crs, unit, true)),
        (None, Some(unit), _) => Ok((crs, unit, true)),
        (None, None, None) => Ok((crs, VerticalUnit::Metre, false)),
        // A vertical CRS this reader does not know, with no unit: its unit could be feet.
        (None, None, Some(code)) => Err(GeoTiffError::Unsupported {
            what: "a vertical CRS without VerticalUnitsGeoKey, EPSG",
            value: code.to_string(),
            hint: "; its unit is unknown here",
        }),
    }
}

/// `[λ₀, Δλ, φ₀, Δφ]` for pixel-is-area, from the tiepoint and pixel scale or the
/// transformation matrix (the module docs).
fn transform<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<[f64; 4], GeoTiffError> {
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
        (Some(scale), Some(tiepoint), None) => {
            let scale = doubles("ModelPixelScaleTag", scale)?;
            let tie = doubles("ModelTiepointTag", tiepoint)?;
            let [sx, sy, ..] = scale[..] else {
                return Err(GeoTiffError::Malformed {
                    what: "ModelPixelScaleTag",
                    reason: format!("{} values, not 3", scale.len()),
                });
            };
            let [i, j, _k, x, y, _z] = tie[..] else {
                return Err(GeoTiffError::Unsupported {
                    what: "ModelTiepointTag with values numbering",
                    value: tie.len().to_string(),
                    hint: "; one tiepoint (6 values) with a pixel scale is read, not ground control points",
                });
            };
            // GDAL's order of operations, so a corner reads as GDAL reads it.
            let dlat = -sy;
            [x - i * sx, sx, y - j * dlat, dlat]
        }
        (_, _, Some(matrix)) => {
            let m = doubles("ModelTransformationTag", matrix)?;
            let [a, b, _, c, d, e, _, f, ..] = m[..] else {
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
            if b != 0.0 || d != 0.0 {
                return Err(GeoTiffError::Unsupported {
                    what: "a rotated or sheared raster, with terms",
                    value: format!("{b}, {d}"),
                    hint: "; `gdalwarp` turns it north-up",
                });
            }
            [c, a, f, e]
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
    if !(out.iter().all(|v| v.is_finite()) && out[1] != 0.0 && out[3] != 0.0) {
        return Err(GeoTiffError::Malformed {
            what: "the georeferencing",
            reason: format!(
                "corner ({}, {}) and pixel size ({}, {})",
                out[0], out[2], out[1], out[3]
            ),
        });
    }
    Ok(out)
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
