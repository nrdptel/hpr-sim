//! GRIB edition 2, the World Meteorological Organization's binary format for gridded weather:
//! the fields NOAA's NOMADS grib filter cuts from GFS and RAP.
//!
//! A GRIB2 file is a run of messages. Each holds one or more fields: a grid (section 3), what the
//! values are and at which level and time (section 4), how they are packed (section 5), which grid
//! points have a value (section 6, the bitmap) and the packed values (section 7). [`parse`] reads
//! the headers of every field and keeps the packed values borrowed, so reading a file costs memory
//! in proportion to the number of fields, not their grids; [`Field::value`] unpacks one grid
//! point, [`Field::values`] all of them. The layout is the WMO's *Manual on Codes*, WMO-No. 306,
//! Volume I.2, FM 92 GRIB edition 2, and its code and flag tables.
//!
//! What is read, which covers what the grib filter serves from GFS and RAP:
//!
//! | section | templates read |
//! |---|---|
//! | 3, grid | 3.0 latitude/longitude; 3.30 Lambert conformal, on a sphere, tangent cone, north pole on the plane |
//! | 4, product | 4.0, a field at a level at one time |
//! | 5, packing | 5.0, simple packing |
//! | 6, bitmap | none, one given, or the one before it in the message |
//!
//! Anything else is refused with [`Grib2Error::Unsupported`], naming the template, never read
//! wrongly. Complex packing (5.2, 5.3) and JPEG 2000 (5.40), which NCEP's whole files use, are for
//! [M5.2d][roadmap].
//!
//! **Values.** Simple packing stores each value as an integer `X` of a fixed number of bits, with
//! a reference value `R` (a 32-bit float), a binary scale factor `E` and a decimal scale factor
//! `D` shared by the field (WMO-No. 306, Regulation 92.9.4):
//!
//! `Y = (R + X · 2^E) / 10^D`
//!
//! evaluated in `f64` with exact powers of two and ten. A field of 0 bits is `R / 10^D` at every
//! point with a value. Signed integers in GRIB2 are sign and magnitude (the first bit is the sign),
//! not two's complement (Regulation 92.1.5).
//!
//! **Grids.** [`Grid::point_deg`] gives a grid point's latitude and longitude, and
//! [`Grid::index_at`] the (fractional) grid indices of a place. On a Lambert conformal grid both
//! use the spherical Lambert conformal conic projection of Snyder, *Map Projections: A Working
//! Manual*, USGS Professional Paper 1395 (1987): eqs. 14-1, 14-2, 14-4, 15-1 and 15-2, and for
//! the inverse 14-9 to 14-11 and 15-5, with a tangent cone's `n = sin φ₁` (the one-parallel case
//! of eq. 15-3). Winds on such a grid may be given along the
//! grid's axes rather than east and north (flag table 3.3, bit 5, [`Grid::winds_grid_relative`]);
//! [`Grid::earth_relative_wind`] turns them by the angle between the grid's `y` axis and true
//! north, `θ = n (λ − λ₀)`.
//!
//! **Checked against:** ecCodes 2.49.0, run as an outside decoder, on recorded GFS and RAP cuts
//! (`crates/hpr-net/tests/nomads.rs`): every value, and every grid point's latitude and longitude.
//!
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md

use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// The most grid points a field may have: 2²⁴, about 16.8 million. The largest common grids are
/// well inside it (GFS at 0.25°, about 1.04 million; ECMWF at 0.1°, about 6.5 million). A field of
/// 0 bits has no packed data to bound its grid, so this bounds what [`Field::values`] allocates:
/// 16 bytes a point, up to 256 MiB for one field, whatever the file's size. [`Field::value`] reads
/// one point and allocates nothing.
pub const MAX_POINTS: u64 = 1 << 24;

/// Why a GRIB2 file was refused.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Grib2Error {
    /// The bytes at a message's start are not `GRIB`.
    #[error("not a GRIB message at byte {offset}: it begins with {head}")]
    NotGrib {
        /// Where the message was expected.
        offset: usize,
        /// Its first bytes, printed as hex.
        head: String,
    },
    /// A GRIB message of another edition (edition 1 is the older format).
    #[error("the message at byte {offset} is GRIB edition {edition}; only edition 2 is read")]
    Edition {
        /// The message's first byte.
        offset: usize,
        /// Its edition.
        edition: u8,
    },
    /// A message or a section runs past the end of the bytes it has.
    #[error("message {message} ends early: {what} needs {needed} bytes, {available} remain")]
    Truncated {
        /// The message's index, from 0.
        message: usize,
        /// What was being read.
        what: &'static str,
        /// Bytes needed.
        needed: u64,
        /// Bytes there are.
        available: u64,
    },
    /// A message breaks the format.
    #[error("message {message} is malformed: {reason}")]
    Malformed {
        /// The message's index, from 0.
        message: usize,
        /// What is wrong.
        reason: String,
    },
    /// A template or an option the reader does not handle.
    #[error("message {message}: {what} {value} is not read")]
    Unsupported {
        /// The message's index, from 0.
        message: usize,
        /// What it is, such as `"data representation template"`.
        what: &'static str,
        /// Its code.
        value: u64,
    },
    /// A grid point asked for is outside the grid.
    #[error("grid point {index} is outside a grid of {points}")]
    PointOutside {
        /// The index asked for.
        index: u64,
        /// The grid's points.
        points: u64,
    },
}

/// When a field's data starts: section 1's reference time, in UTC as written.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceTime {
    /// Code table 1.2: 0 analysis, 1 start of forecast, 2 verifying time, 3 observation time.
    pub significance: u8,
    /// Year.
    pub year: u16,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day, 1 to 31.
    pub day: u8,
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute.
    pub minute: u8,
    /// Second.
    pub second: u8,
}

/// The figure of the Earth a grid is defined on (code table 3.2).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Earth {
    /// A sphere of this radius, m: codes 0 (6,367,470 m), 1 (given), 6 (6,371,229 m) and
    /// 8 (6,371,200 m).
    #[non_exhaustive]
    Sphere {
        /// Radius, m.
        radius_m: f64,
    },
    /// An oblate spheroid or another figure, by its code; a Lambert grid on it is refused.
    #[non_exhaustive]
    Other {
        /// The code in table 3.2.
        code: u8,
    },
}

/// How a grid's points map to the Earth.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Projection {
    /// Template 3.0: points evenly spaced in latitude and longitude.
    #[non_exhaustive]
    LatLon {
        /// The first point's latitude, degrees north.
        first_lat_deg: f64,
        /// The first point's longitude, degrees east, in `[0, 360)` as GRIB writes it.
        first_lon_deg: f64,
        /// The spacing in longitude between neighbouring points along `i`, degrees.
        di_deg: f64,
        /// The spacing in latitude between neighbouring points along `j`, degrees.
        dj_deg: f64,
    },
    /// Template 3.30: a Lambert conformal conic projection, on a tangent cone.
    #[non_exhaustive]
    LambertConformal {
        /// The first point's latitude, degrees north.
        first_lat_deg: f64,
        /// The first point's longitude, degrees east.
        first_lon_deg: f64,
        /// The latitude where the cone touches the sphere (`Latin1 = Latin2 = LaD`), degrees.
        tangent_lat_deg: f64,
        /// The meridian parallel to the grid's `y` axis (`LoV`), degrees east.
        orientation_lon_deg: f64,
        /// The grid spacing along `x` at the tangent latitude, m.
        dx_m: f64,
        /// The grid spacing along `y` at the tangent latitude, m.
        dy_m: f64,
        /// The sphere's radius, m.
        radius_m: f64,
    },
}

/// A field's grid: its size, its projection and how its points are ordered.
///
/// Points are numbered `i + ni · j`, `i` along a row (east on a latitude/longitude grid, `+x` on a
/// Lambert grid) and `j` from row to row. Rows run south to north when
/// [`Grid::south_to_north`], else north to south; the reader refuses other scanning modes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    /// Points along a row.
    pub ni: u32,
    /// Rows.
    pub nj: u32,
    /// The figure of the Earth.
    pub earth: Earth,
    /// Whether rows run south to north (`+j`, scanning mode bit 2); otherwise north to south.
    pub south_to_north: bool,
    /// Whether vector components (winds) are along the grid's `x` and `y` axes rather than east
    /// and north (flag table 3.3, bit 5).
    pub winds_grid_relative: bool,
    /// The projection.
    pub projection: Projection,
}

/// A surface a field is on (code table 4.5), such as an isobaric level.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    /// Its type: 1 the ground, 100 an isobaric level (value in Pa), 103 a height above the ground
    /// (value in m), 255 none.
    pub kind: u8,
    /// Its value, `scaled value / 10^scale factor`, or `None` when missing.
    pub value: Option<f64>,
}

/// What a field holds, and at which level and time: product definition template 4.0.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Product {
    /// Parameter category (code table 4.1), such as 0 temperature or 2 momentum.
    pub category: u8,
    /// Parameter number within the category (code table 4.2).
    pub number: u8,
    /// Type of generating process (code table 4.3): 0 analysis, 2 forecast, and so on.
    pub process: u8,
    /// Unit of the forecast time (code table 4.4): 0 minute, 1 hour, 2 day, 10 3 h, 11 6 h,
    /// 12 12 h, 13 second.
    pub time_unit: u8,
    /// Forecast time in that unit, after the reference time.
    pub forecast_time: i64,
    /// The first fixed surface.
    pub surface: Surface,
    /// The second fixed surface (kind 255 when there is none).
    pub second_surface: Surface,
}

impl Product {
    /// The forecast time in seconds, or `None` for a unit other than those listed on
    /// [`Product::time_unit`].
    #[must_use]
    pub fn forecast_time_s(&self) -> Option<i64> {
        let unit_s = match self.time_unit {
            0 => 60,
            1 => 3_600,
            2 => 86_400,
            10 => 3 * 3_600,
            11 => 6 * 3_600,
            12 => 12 * 3_600,
            13 => 1,
            _ => return None,
        };
        self.forecast_time.checked_mul(unit_s)
    }
}

/// Simple packing's parameters: data representation template 5.0.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SimplePacking {
    /// The reference value `R`, as written (a 32-bit float).
    pub reference: f32,
    /// The binary scale factor `E`.
    pub binary_scale: i16,
    /// The decimal scale factor `D`.
    pub decimal_scale: i16,
    /// Bits per packed value, 0 to 32.
    pub bits: u8,
    /// How many values are packed: the grid points the bitmap marks, or all of them.
    pub count: u32,
}

impl SimplePacking {
    /// `Y = (R + X · 2^E) / 10^D` for a packed integer `X`.
    #[must_use]
    pub fn unpack(&self, x: u32) -> f64 {
        let scaled =
            f64::from(self.reference) + f64::from(x) * 2_f64.powi(self.binary_scale.into());
        // Powers of ten to 10^22 are exact in f64, so dividing (or multiplying for D < 0) is one
        // rounding, whichever sign D has.
        let d = i32::from(self.decimal_scale);
        if d >= 0 {
            scaled / 10_f64.powi(d)
        } else {
            scaled * 10_f64.powi(-d)
        }
    }
}

/// One field of a GRIB2 file: its headers, with its bitmap and packed values borrowed from the
/// file's bytes.
///
/// It holds no serde derive, since it borrows; its parts ([`Grid`], [`Product`] and the others)
/// do.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct Field<'a> {
    /// The message it is in, from 0.
    pub message: usize,
    /// The discipline (code table 0.0): 0 meteorological products.
    pub discipline: u8,
    /// The originating centre (common code table C-11): 7 is NCEP.
    pub centre: u16,
    /// The reference time.
    pub reference_time: ReferenceTime,
    /// The grid.
    pub grid: Grid,
    /// What the field is.
    pub product: Product,
    /// How it is packed.
    pub packing: SimplePacking,
    /// One bit per grid point, first point in the first byte's high bit: 1 when the point has a
    /// value. `None` when every point has one.
    bitmap: Option<Bitmap<'a>>,
    /// The packed values, `packing.bits` each, from the first byte's high bit.
    data: &'a [u8],
}

impl Field<'_> {
    /// The grid's points, `ni · nj`.
    #[must_use]
    pub fn points(&self) -> u64 {
        u64::from(self.grid.ni) * u64::from(self.grid.nj)
    }

    /// The value at grid point `index` (numbered as on [`Grid`]), or `None` where the bitmap marks
    /// the point as having none.
    ///
    /// # Errors
    /// [`Grib2Error::PointOutside`] when `index` is not below [`Field::points`].
    pub fn value(&self, index: u64) -> Result<Option<f64>, Grib2Error> {
        let points = self.points();
        if index >= points {
            return Err(Grib2Error::PointOutside { index, points });
        }
        let packed = match &self.bitmap {
            Some(bitmap) => {
                if !bitmap.get(index) {
                    return Ok(None);
                }
                bitmap.ones_before(index)
            }
            None => index,
        };
        Ok(Some(self.packing.unpack(self.packed(packed))))
    }

    /// Every grid point's value, in grid order; `None` where the bitmap marks none. It allocates 16
    /// bytes a point, up to 256 MiB at [`MAX_POINTS`], even for a field of 0 bits in a tiny file;
    /// [`Field::value`] reads one point without allocating.
    #[must_use]
    pub fn values(&self) -> Vec<Option<f64>> {
        let points = self.points();
        // `parse` bounds `points` by `MAX_POINTS`, so this fits a `usize` on every target.
        let mut out = Vec::with_capacity(usize::try_from(points).unwrap_or(0));
        let mut packed = 0;
        for index in 0..points {
            let present = self.bitmap.as_ref().is_none_or(|bitmap| bitmap.get(index));
            out.push(present.then(|| {
                let x = self.packed(packed);
                packed += 1;
                self.packing.unpack(x)
            }));
        }
        out
    }

    /// The `k`-th packed integer. `parse` checked that the data holds `count · bits` bits, and
    /// callers pass `k < count`.
    fn packed(&self, k: u64) -> u32 {
        let bits = u64::from(self.packing.bits);
        if bits == 0 {
            return 0;
        }
        let start = k * bits;
        let mut acc: u64 = 0;
        let first_byte = start / 8;
        let last_byte = (start + bits - 1) / 8;
        for byte in first_byte..=last_byte {
            let b = usize::try_from(byte)
                .ok()
                .and_then(|i| self.data.get(i))
                .copied()
                .unwrap_or(0);
            acc = (acc << 8) | u64::from(b);
        }
        let used = (last_byte - first_byte + 1) * 8;
        let shift = used - (start % 8) - bits;
        let mask = (1_u64 << bits) - 1;
        // `bits` is at most 32, so the masked value fits.
        u32::try_from((acc >> shift) & mask).unwrap_or(u32::MAX)
    }
}

/// A bitmap, one bit per grid point from the first byte's high bit, with its running count of set
/// bits every 64 bytes: a point's place among the packed values then costs at most 64 bytes of
/// counting, however large the grid, and fields that reuse the bitmap share the counts.
#[derive(Debug, Clone, PartialEq)]
struct Bitmap<'a> {
    bits: &'a [u8],
    /// `ones[k]`: the set bits in `bits[..64 k]`, one entry per 64 bytes and one more, so an
    /// eighth of the bitmap's size.
    ones: Arc<[u64]>,
}

impl<'a> Bitmap<'a> {
    fn new(bits: &'a [u8]) -> Self {
        let ones = std::iter::once(0)
            .chain(bits.chunks(64).scan(0, |total, chunk| {
                *total += count_ones(chunk);
                Some(*total)
            }))
            .collect();
        Self { bits, ones }
    }

    /// Whether bit `index` is set; `parse` checked the bitmap covers the grid.
    fn get(&self, index: u64) -> bool {
        let byte = usize::try_from(index / 8)
            .ok()
            .and_then(|i| self.bits.get(i))
            .copied()
            .unwrap_or(0);
        byte & (0x80 >> (index % 8)) != 0
    }

    /// The set bits before bit `index`.
    fn ones_before(&self, index: u64) -> u64 {
        let byte = usize::try_from(index / 8)
            .unwrap_or(usize::MAX)
            .min(self.bits.len());
        let block = byte / 64;
        let mut n =
            self.ones.get(block).copied().unwrap_or(0) + count_ones(&self.bits[block * 64..byte]);
        let rest = index % 8;
        if rest > 0 {
            let partial = self.bits.get(byte).copied().unwrap_or(0) & !(0xFF_u8 >> rest);
            n += u64::from(partial.count_ones());
        }
        n
    }
}

fn count_ones(bytes: &[u8]) -> u64 {
    bytes.iter().map(|b| u64::from(b.count_ones())).sum()
}

impl Grid {
    /// The points, `ni · nj`.
    #[must_use]
    pub fn points(&self) -> u64 {
        u64::from(self.ni) * u64::from(self.nj)
    }

    /// Grid point `index`'s latitude (degrees north) and longitude (degrees east, in `[0, 360)`),
    /// or `None` outside the grid.
    #[must_use]
    pub fn point_deg(&self, index: u64) -> Option<(f64, f64)> {
        if index >= self.points() {
            return None;
        }
        let i = (index % u64::from(self.ni)) as f64;
        let j = (index / u64::from(self.ni)) as f64;
        let j_sign = if self.south_to_north { 1.0 } else { -1.0 };
        match self.projection {
            Projection::LatLon {
                first_lat_deg,
                first_lon_deg,
                di_deg,
                dj_deg,
            } => Some((
                first_lat_deg + j_sign * j * dj_deg,
                (first_lon_deg + i * di_deg).rem_euclid(360.0),
            )),
            Projection::LambertConformal {
                first_lat_deg,
                first_lon_deg,
                dx_m,
                dy_m,
                ..
            } => {
                let lambert = Lambert::new(&self.projection)?;
                let (x0, y0) = lambert.forward(first_lat_deg, first_lon_deg);
                let (lat, lon) = lambert.inverse(x0 + i * dx_m, y0 + j_sign * j * dy_m);
                Some((lat, lon.rem_euclid(360.0)))
            }
        }
    }

    /// The fractional grid indices `(i, j)` of a place, such that grid point `(⌊i⌋, ⌊j⌋)` and its
    /// neighbours at `+1` surround it; the place may be outside the grid.
    #[must_use]
    pub fn index_at(&self, latitude_deg: f64, longitude_deg: f64) -> (f64, f64) {
        let j_sign = if self.south_to_north { 1.0 } else { -1.0 };
        match self.projection {
            Projection::LatLon {
                first_lat_deg,
                first_lon_deg,
                di_deg,
                dj_deg,
            } => (
                (longitude_deg - first_lon_deg).rem_euclid(360.0) / di_deg,
                j_sign * (latitude_deg - first_lat_deg) / dj_deg,
            ),
            Projection::LambertConformal {
                first_lat_deg,
                first_lon_deg,
                dx_m,
                dy_m,
                ..
            } => match Lambert::new(&self.projection) {
                Some(lambert) => {
                    let (x0, y0) = lambert.forward(first_lat_deg, first_lon_deg);
                    let (x, y) = lambert.forward(latitude_deg, longitude_deg);
                    ((x - x0) / dx_m, j_sign * (y - y0) / dy_m)
                }
                None => (f64::NAN, f64::NAN),
            },
        }
    }

    /// The angle from true north to the grid's `+y` axis at a place, clockwise positive, rad:
    /// `θ = n (λ − λ₀)` on a Lambert grid (Snyder eq. 14-4), 0 on a latitude/longitude grid,
    /// whose axes point east and north. West of the central meridian `θ < 0`: the meridians lean
    /// toward the cone's apex, so north is turned toward `+x` and the grid's `+y` west of north.
    #[must_use]
    pub fn north_to_grid_y_rad(&self, longitude_deg: f64) -> f64 {
        match Lambert::new(&self.projection) {
            Some(lambert) => lambert.theta(longitude_deg),
            None => 0.0,
        }
    }

    /// A wind's east and north components, m/s, from its components as the field gives them:
    /// unchanged unless [`Grid::winds_grid_relative`], else turned from the grid's axes.
    ///
    /// On a Lambert grid true north is `(−sin θ, cos θ)` in grid axes and east `(cos θ, sin θ)`,
    /// with `θ = n (λ − λ₀)`, so `u_east = u cos θ + v sin θ` and `v_north = −u sin θ + v cos θ`.
    #[must_use]
    pub fn earth_relative_wind(&self, u_m_s: f64, v_m_s: f64, longitude_deg: f64) -> (f64, f64) {
        if !self.winds_grid_relative {
            return (u_m_s, v_m_s);
        }
        let theta = self.north_to_grid_y_rad(longitude_deg);
        let (sin, cos) = theta.sin_cos();
        (u_m_s * cos + v_m_s * sin, -u_m_s * sin + v_m_s * cos)
    }
}

/// The spherical Lambert conformal conic projection on a tangent cone (Snyder 1987, ch. 15).
struct Lambert {
    /// Cone constant `n = sin φ₁` (eq. 15-3 with one standard parallel).
    n: f64,
    /// `R F`, with `F = cos φ₁ tanⁿ(π/4 + φ₁/2) / n` (eq. 15-2).
    rf: f64,
    /// The central meridian `λ₀`, rad.
    lon0_rad: f64,
}

impl Lambert {
    fn new(projection: &Projection) -> Option<Self> {
        let Projection::LambertConformal {
            tangent_lat_deg,
            orientation_lon_deg,
            radius_m,
            ..
        } = *projection
        else {
            return None;
        };
        let phi1 = tangent_lat_deg.to_radians();
        let n = phi1.sin();
        let f = phi1.cos() * quarter_tan(phi1).powf(n) / n;
        Some(Self {
            n,
            rf: radius_m * f,
            lon0_rad: orientation_lon_deg.to_radians(),
        })
    }

    /// `θ = n (λ − λ₀)`, with `λ − λ₀` folded into `[−π, π)` (eq. 14-4).
    fn theta(&self, longitude_deg: f64) -> f64 {
        let d = (longitude_deg.to_radians() - self.lon0_rad + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        self.n * d
    }

    /// `x = ρ sin θ`, `y = −ρ cos θ`, `ρ = R F / tanⁿ(π/4 + φ/2)` (eqs. 14-1, 14-2 and 15-1, with
    /// the origin at the cone's apex, `ρ₀ = 0`; only differences are used).
    fn forward(&self, latitude_deg: f64, longitude_deg: f64) -> (f64, f64) {
        let rho = self.rf / quarter_tan(latitude_deg.to_radians()).powf(self.n);
        let theta = self.theta(longitude_deg);
        (rho * theta.sin(), -rho * theta.cos())
    }

    /// The inverse, `ρ = √(x² + y²)`, `θ = atan2(x, −y)`, `φ = 2 atan((R F/ρ)^(1/n)) − π/2`,
    /// `λ = θ/n + λ₀` (eqs. 14-10, 14-11, 15-5 and 14-9, for `n > 0` and `ρ₀ = 0`), in degrees.
    fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let rho = x.hypot(y);
        let theta = x.atan2(-y);
        let phi = 2.0 * (self.rf / rho).powf(1.0 / self.n).atan() - std::f64::consts::FRAC_PI_2;
        (
            phi.to_degrees(),
            (theta / self.n + self.lon0_rad).to_degrees(),
        )
    }
}

/// `tan(π/4 + φ/2)`.
fn quarter_tan(phi: f64) -> f64 {
    (std::f64::consts::FRAC_PI_4 + phi / 2.0).tan()
}

/// Reads every field of a GRIB2 file.
///
/// The messages must follow one another with nothing between or after them, as the grib filter
/// and NCEP write them.
///
/// # Errors
/// [`Grib2Error::NotGrib`] and [`Grib2Error::Edition`] when a message is not GRIB2,
/// [`Grib2Error::Truncated`] and [`Grib2Error::Malformed`] when one breaks the format, and
/// [`Grib2Error::Unsupported`] for a template or an option outside the module's table.
pub fn parse(bytes: &[u8]) -> Result<Vec<Field<'_>>, Grib2Error> {
    let mut fields = Vec::new();
    let mut offset = 0;
    let mut message = 0;
    while offset < bytes.len() {
        let rest = &bytes[offset..];
        if rest.len() < 16 || &rest[..4] != b"GRIB" {
            return Err(Grib2Error::NotGrib {
                offset,
                head: rest.iter().take(8).map(|b| format!("{b:02x}")).collect(),
            });
        }
        if rest[7] != 2 {
            return Err(Grib2Error::Edition {
                offset,
                edition: rest[7],
            });
        }
        let length = u64::from_be_bytes([
            rest[8], rest[9], rest[10], rest[11], rest[12], rest[13], rest[14], rest[15],
        ]);
        let available = rest.len() as u64;
        if length > available {
            return Err(Grib2Error::Truncated {
                message,
                what: "the message",
                needed: length,
                available,
            });
        }
        // `length ≤ rest.len()`, so it fits a `usize`.
        let length = usize::try_from(length).unwrap_or(usize::MAX);
        if length < 20 || &rest[length - 4..length] != b"7777" {
            return Err(malformed(message, "it does not end with 7777"));
        }
        read_message(&rest[..length], message, rest[6], &mut fields)?;
        offset += length;
        message += 1;
    }
    Ok(fields)
}

fn malformed(message: usize, reason: impl Into<String>) -> Grib2Error {
    Grib2Error::Malformed {
        message,
        reason: reason.into(),
    }
}

/// Reads one message's sections, appending a [`Field`] at each section 7.
fn read_message<'a>(
    bytes: &'a [u8],
    message: usize,
    discipline: u8,
    fields: &mut Vec<Field<'a>>,
) -> Result<(), Grib2Error> {
    let end = bytes.len() - 4;
    let mut offset = 16;
    let mut identification: Option<(u16, ReferenceTime)> = None;
    let mut grid: Option<Grid> = None;
    let mut product: Option<Product> = None;
    let mut packing: Option<SimplePacking> = None;
    // `Some(None)`: section 6 said there is no bitmap.
    let mut bitmap: Option<Option<Bitmap<'a>>> = None;
    let mut last_bitmap: Option<Bitmap<'a>> = None;
    let before = fields.len();
    while offset < end {
        if end - offset < 5 {
            return Err(truncated(message, "a section's header", 5, end - offset));
        }
        let length = be_u32(&bytes[offset..offset + 4]) as usize;
        let number = bytes[offset + 4];
        if length < 5 {
            return Err(malformed(
                message,
                format!("section {number} is {length} bytes long"),
            ));
        }
        if length > end - offset {
            return Err(truncated(message, "a section", length, end - offset));
        }
        let s = &bytes[offset..offset + length];
        match number {
            1 => identification = Some(read_identification(s, message)?),
            2 => {}
            3 => grid = Some(read_grid(s, message)?),
            4 => product = Some(read_product(s, message)?),
            5 => packing = Some(read_packing(s, message)?),
            6 => {
                let indicator = at(s, 5, message)?;
                bitmap = Some(match indicator {
                    0 => {
                        last_bitmap = Some(Bitmap::new(&s[6..]));
                        last_bitmap.clone()
                    }
                    254 => Some(last_bitmap.clone().ok_or_else(|| {
                        malformed(message, "section 6 reuses a bitmap none gave")
                    })?),
                    255 => None,
                    other => {
                        return Err(Grib2Error::Unsupported {
                            message,
                            what: "bitmap indicator",
                            value: other.into(),
                        });
                    }
                });
            }
            7 => {
                let (
                    Some((centre, reference_time)),
                    Some(grid),
                    Some(product),
                    Some(packing),
                    Some(bitmap),
                ) = (
                    identification,
                    grid,
                    product.take(),
                    packing.take(),
                    bitmap.take(),
                )
                else {
                    return Err(malformed(
                        message,
                        "section 7 comes before sections 1 and 3 to 6 are all given",
                    ));
                };
                let field = Field {
                    message,
                    discipline,
                    centre,
                    reference_time,
                    grid,
                    product,
                    packing,
                    bitmap,
                    data: &s[5..],
                };
                check_field(&field)?;
                fields.push(field);
            }
            other => {
                return Err(malformed(message, format!("it holds a section {other}")));
            }
        }
        offset += length;
    }
    if product.is_some() || packing.is_some() || bitmap.is_some() {
        return Err(malformed(message, "it ends before a section 7"));
    }
    if fields.len() == before {
        return Err(malformed(message, "it holds no field"));
    }
    Ok(())
}

/// Checks a field's bitmap and packed values are as long as its grid and packing say.
fn check_field(field: &Field<'_>) -> Result<(), Grib2Error> {
    let message = field.message;
    let points = field.points();
    let count = u64::from(field.packing.count);
    match &field.bitmap {
        Some(bitmap) => {
            let have = bitmap.bits.len() as u64 * 8;
            if have < points {
                return Err(truncated(
                    message,
                    "the bitmap",
                    points.div_ceil(8),
                    bitmap.bits.len(),
                ));
            }
            let marked = bitmap.ones_before(points);
            if marked != count {
                return Err(malformed(
                    message,
                    format!("the bitmap marks {marked} points but section 5 packs {count} values"),
                ));
            }
        }
        None if count != points => {
            return Err(malformed(
                message,
                format!("a grid of {points} points without a bitmap packs {count} values"),
            ));
        }
        None => {}
    }
    // `Y` rises with `X` (`2^E > 0`), so the ends bound every value.
    let packing = &field.packing;
    let largest = u32::try_from((1_u64 << packing.bits) - 1).unwrap_or(u32::MAX);
    if !(packing.unpack(0).is_finite() && packing.unpack(largest).is_finite()) {
        return Err(malformed(
            message,
            format!(
                "the scale factors (binary {}, decimal {}) make values that are not finite",
                packing.binary_scale, packing.decimal_scale
            ),
        ));
    }
    let bits = count * u64::from(field.packing.bits);
    let have = field.data.len() as u64 * 8;
    if have < bits {
        return Err(truncated(
            message,
            "the packed values",
            bits.div_ceil(8),
            field.data.len(),
        ));
    }
    Ok(())
}

fn truncated(
    message: usize,
    what: &'static str,
    needed: impl TryInto<u64>,
    available: impl TryInto<u64>,
) -> Grib2Error {
    Grib2Error::Truncated {
        message,
        what,
        needed: needed.try_into().unwrap_or(u64::MAX),
        available: available.try_into().unwrap_or(u64::MAX),
    }
}

/// Byte `i` of a section, or [`Grib2Error::Truncated`].
fn at(s: &[u8], i: usize, message: usize) -> Result<u8, Grib2Error> {
    s.get(i)
        .copied()
        .ok_or_else(|| truncated(message, "a section", i + 1, s.len()))
}

/// A section at least `len` bytes long, or [`Grib2Error::Truncated`].
fn need<'a>(
    s: &'a [u8],
    len: usize,
    what: &'static str,
    message: usize,
) -> Result<&'a [u8], Grib2Error> {
    if s.len() < len {
        return Err(truncated(message, what, len, s.len()));
    }
    Ok(s)
}

fn be_u16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}

fn be_u32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// A latitude, degrees, if it is on the Earth.
fn on_earth(latitude_deg: f64, message: usize) -> Result<f64, Grib2Error> {
    if latitude_deg.abs() <= 90.0 {
        Ok(latitude_deg)
    } else {
        Err(malformed(message, format!("a latitude of {latitude_deg}°")))
    }
}

/// A sign-and-magnitude 32-bit integer (WMO-No. 306, Regulation 92.1.5).
fn be_i32(b: &[u8]) -> i64 {
    let raw = be_u32(b);
    let magnitude = i64::from(raw & 0x7FFF_FFFF);
    if raw & 0x8000_0000 != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// A sign-and-magnitude 16-bit integer.
fn be_i16(b: &[u8]) -> i16 {
    let raw = be_u16(b);
    // The magnitude is at most 0x7FFF, so it fits.
    let magnitude = i16::try_from(raw & 0x7FFF).unwrap_or(i16::MAX);
    if raw & 0x8000 != 0 {
        -magnitude
    } else {
        magnitude
    }
}

/// A sign-and-magnitude 8-bit integer.
fn be_i8(b: u8) -> i32 {
    let magnitude = i32::from(b & 0x7F);
    if b & 0x80 != 0 { -magnitude } else { magnitude }
}

/// Section 1: the originating centre and the reference time.
fn read_identification(s: &[u8], message: usize) -> Result<(u16, ReferenceTime), Grib2Error> {
    let s = need(s, 19, "section 1", message)?;
    Ok((
        be_u16(&s[5..7]),
        ReferenceTime {
            significance: s[11],
            year: be_u16(&s[12..14]),
            month: s[14],
            day: s[15],
            hour: s[16],
            minute: s[17],
            second: s[18],
        },
    ))
}

/// Code table 3.2's figure of the Earth, from the template's first 16 bytes after its number.
fn read_earth(t: &[u8]) -> Earth {
    match t[0] {
        0 => Earth::Sphere {
            radius_m: 6_367_470.0,
        },
        1 => Earth::Sphere {
            radius_m: scaled(be_u32(&t[2..6]), be_i8(t[1])),
        },
        6 => Earth::Sphere {
            radius_m: 6_371_229.0,
        },
        8 => Earth::Sphere {
            radius_m: 6_371_200.0,
        },
        code => Earth::Other { code },
    }
}

/// Section 3: the grid.
fn read_grid(s: &[u8], message: usize) -> Result<Grid, Grib2Error> {
    let s = need(s, 14, "section 3", message)?;
    let unsupported = |what, value: u64| Grib2Error::Unsupported {
        message,
        what,
        value,
    };
    if s[5] != 0 {
        return Err(unsupported("source of grid definition", s[5].into()));
    }
    if s[10] != 0 {
        return Err(unsupported("list of points per row, bytes", s[10].into()));
    }
    let declared = u64::from(be_u32(&s[6..10]));
    let template = be_u16(&s[12..14]);
    let (ni, nj, flags, scanning, projection, earth) = match template {
        0 => {
            let s = need(s, 72, "grid template 3.0", message)?;
            let earth = read_earth(&s[14..30]);
            let basic = be_u32(&s[38..42]);
            let subdivisions = be_u32(&s[42..46]);
            // Code: 0 or all ones means units of 10⁻⁶ degree.
            // Template 3.0's note 1: angles are in units of `basic / subdivisions` degrees, a basic
            // angle of 0 or missing meaning 1 and missing subdivisions meaning 10⁶.
            let basic = if basic == 0 || basic == u32::MAX {
                1
            } else {
                basic
            };
            let subdivisions = if subdivisions == u32::MAX {
                1_000_000
            } else {
                subdivisions
            };
            if subdivisions == 0 {
                return Err(malformed(message, "the grid's angle subdivisions are 0"));
            }
            let degrees = |units: f64| units * f64::from(basic) / f64::from(subdivisions);
            let di = be_u32(&s[63..67]);
            let dj = be_u32(&s[67..71]);
            if di == u32::MAX || dj == u32::MAX || di == 0 || dj == 0 {
                return Err(malformed(message, "the grid's increments are missing or 0"));
            }
            let nj = be_u32(&s[34..38]);
            let first_lat_deg = degrees(be_i32(&s[46..50]) as f64);
            let (di_deg, dj_deg) = (degrees(f64::from(di)), degrees(f64::from(dj)));
            // The rows run from the first latitude by `dj`, north or south; all of them must be on
            // the Earth, and a step can't pass a whole turn.
            let span_deg = f64::from(nj.saturating_sub(1)) * dj_deg;
            let last_lat_deg = if s[71] & 0x40 != 0 {
                first_lat_deg + span_deg
            } else {
                first_lat_deg - span_deg
            };
            let on_earth = |lat: f64| lat.abs() <= 90.0 + 1e-6;
            if !on_earth(first_lat_deg) || !on_earth(last_lat_deg) || di_deg > 360.0 {
                return Err(malformed(
                    message,
                    format!(
                        "the grid's rows from {first_lat_deg}° to {last_lat_deg}° or its \
                         {di_deg}° steps are off the Earth"
                    ),
                ));
            }
            let projection = Projection::LatLon {
                first_lat_deg,
                first_lon_deg: degrees(be_i32(&s[50..54]) as f64).rem_euclid(360.0),
                di_deg,
                dj_deg,
            };
            (
                be_u32(&s[30..34]),
                be_u32(&s[34..38]),
                s[54],
                s[71],
                projection,
                earth,
            )
        }
        30 => {
            let s = need(s, 81, "grid template 3.30", message)?;
            let earth = read_earth(&s[14..30]);
            let Earth::Sphere { radius_m } = earth else {
                return Err(unsupported(
                    "Lambert grid on the figure of the Earth",
                    s[14].into(),
                ));
            };
            // A sphere given by its radius (code 1) must be about the Earth's size.
            if !(6.0e6..=7.0e6).contains(&radius_m) {
                return Err(malformed(
                    message,
                    format!("the Earth's radius is {radius_m} m"),
                ));
            }
            let centre = s[63];
            if centre & 0xC0 != 0 {
                return Err(unsupported("Lambert projection centre flag", centre.into()));
            }
            let micro = |b: &[u8]| be_i32(b) as f64 / 1e6;
            let lad = micro(&s[47..51]);
            let latin1 = micro(&s[65..69]);
            let latin2 = micro(&s[69..73]);
            // A secant cone, or grid lengths given away from the tangent latitude, would need the
            // scale factor there; no NCEP grid in use has either, so they are refused, not guessed.
            if latin1 != latin2 || lad != latin1 || !(latin1 > 0.0 && latin1 < 90.0) {
                return Err(malformed(
                    message,
                    format!(
                        "the Lambert grid's Latin1 {latin1}°, Latin2 {latin2}° and LaD {lad}° are \
                         not one latitude between 0° and 90°: only a northern tangent cone is read"
                    ),
                ));
            }
            let dx = be_u32(&s[55..59]);
            let dy = be_u32(&s[59..63]);
            if dx == 0 || dy == 0 || dx == u32::MAX || dy == u32::MAX {
                return Err(malformed(message, "the grid's lengths are missing or 0"));
            }
            let projection = Projection::LambertConformal {
                first_lat_deg: on_earth(micro(&s[38..42]), message)?,
                first_lon_deg: micro(&s[42..46]).rem_euclid(360.0),
                tangent_lat_deg: latin1,
                orientation_lon_deg: micro(&s[51..55]).rem_euclid(360.0),
                dx_m: f64::from(dx) * 1e-3,
                dy_m: f64::from(dy) * 1e-3,
                radius_m,
            };
            (
                be_u32(&s[30..34]),
                be_u32(&s[34..38]),
                s[46],
                s[64],
                projection,
                earth,
            )
        }
        other => return Err(unsupported("grid definition template", other.into())),
    };
    // Scanning mode (flag table 3.4): only bit 2 (+j, 0x40) may be set: +i along rows, rows
    // consecutive, not boustrophedon, no offsets.
    if scanning & !0x40 != 0 {
        return Err(unsupported("scanning mode", scanning.into()));
    }
    let points = u64::from(ni) * u64::from(nj);
    if points == 0 || points != declared {
        return Err(malformed(
            message,
            format!("the grid is {ni} by {nj} but declares {declared} points"),
        ));
    }
    if points > MAX_POINTS {
        return Err(unsupported("grid of this many points", points));
    }
    Ok(Grid {
        ni,
        nj,
        earth,
        south_to_north: scanning & 0x40 != 0,
        winds_grid_relative: flags & 0x08 != 0,
        projection,
    })
}

/// A fixed surface from its type, scale factor and scaled value.
/// `value / 10^scale`, with one rounding: powers of ten to 10^22 are exact.
fn scaled(value: u32, scale: i32) -> f64 {
    if scale >= 0 {
        f64::from(value) / 10_f64.powi(scale)
    } else {
        f64::from(value) * 10_f64.powi(-scale)
    }
}

fn read_surface(s: &[u8]) -> Surface {
    let kind = s[0];
    let scale = s[1];
    let raw = be_u32(&s[2..6]);
    let value = (scale != 0xFF && raw != u32::MAX).then(|| scaled(raw, be_i8(scale)));
    Surface { kind, value }
}

/// Section 4: product definition template 4.0.
fn read_product(s: &[u8], message: usize) -> Result<Product, Grib2Error> {
    let s = need(s, 9, "section 4", message)?;
    let template = be_u16(&s[7..9]);
    if template != 0 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "product definition template",
            value: template.into(),
        });
    }
    let s = need(s, 34, "product template 4.0", message)?;
    Ok(Product {
        category: s[9],
        number: s[10],
        process: s[11],
        time_unit: s[17],
        forecast_time: be_i32(&s[18..22]),
        surface: read_surface(&s[22..28]),
        second_surface: read_surface(&s[28..34]),
    })
}

/// Section 5: data representation template 5.0.
fn read_packing(s: &[u8], message: usize) -> Result<SimplePacking, Grib2Error> {
    let s = need(s, 11, "section 5", message)?;
    let template = be_u16(&s[9..11]);
    if template != 0 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "data representation template",
            value: template.into(),
        });
    }
    let s = need(s, 21, "data template 5.0", message)?;
    let bits = s[19];
    if bits > 32 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "bits per value",
            value: bits.into(),
        });
    }
    // Table 5.1: 0 floating point, 1 integer. Either unpacks the same way.
    if s[20] > 1 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "type of original field values",
            value: s[20].into(),
        });
    }
    let reference = f32::from_bits(be_u32(&s[11..15]));
    if !reference.is_finite() {
        return Err(malformed(message, "the reference value is not finite"));
    }
    Ok(SimplePacking {
        reference,
        binary_scale: be_i16(&s[15..17]),
        decimal_scale: be_i16(&s[17..19]),
        bits,
        count: be_u32(&s[5..9]),
    })
}
