//! JPEG 2000 packing: data representation template 5.40 (WMO-No. 306, Volume I.2, FM 92 GRIB
//! edition 2, template 5.40 and code table 5.40).
//!
//! The field's integers `X` are a greyscale image, one sample per packed value, coded as a JPEG
//! 2000 codestream (ISO/IEC 15444-1) in section 7; each unpacks as simple packing's do,
//! `Y = (R + X · 2^E) / 10^D`. The codestream is decoded by `hayro-jpeg2000` (MIT or Apache-2.0),
//! which holds samples as `f32`: its reversible 5/3 wavelet takes exact floors, so every integer
//! below `2^24` comes back exact, and fields of more than 24 bits are refused by name. Lossy
//! coding (code table 5.40's 1) is refused too: its samples are rounded by the decoder's own
//! arithmetic, and none was checked against ecCodes.

use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};
use serde::{Deserialize, Serialize};

use super::{Grib2Error, be_i16, be_u32, malformed, need, unpack};

/// The most bits per value read: `hayro-jpeg2000`'s `f32` samples hold every integer below
/// `2^24` exactly.
pub const MAX_BITS: u8 = 24;

/// JPEG 2000 packing's parameters: data representation template 5.40.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Jpeg2000Packing {
    /// The reference value `R`, as written (a 32-bit float).
    pub reference: f32,
    /// The binary scale factor `E`.
    pub binary_scale: i16,
    /// The decimal scale factor `D`.
    pub decimal_scale: i16,
    /// The image's bits per sample, 0 to [`MAX_BITS`]. A field of 0 bits has no codestream: it is
    /// `R / 10^D` at every point with a value.
    pub bits: u8,
    /// How many values are packed: the grid points the bitmap marks, or all of them.
    pub count: u32,
}

impl Jpeg2000Packing {
    /// `Y = (R + X · 2^E) / 10^D` for a packed integer `X`.
    #[must_use]
    pub fn unpack(&self, x: u32) -> f64 {
        unpack(
            self.reference,
            self.binary_scale,
            self.decimal_scale,
            f64::from(x),
        )
    }
}

/// Section 5 in template 5.40: 23 bytes.
pub(super) fn read(s: &[u8], message: usize) -> Result<Jpeg2000Packing, Grib2Error> {
    let s = need(s, 23, "data template 5.40", message)?;
    let unsupported = |what, value: u8| Grib2Error::Unsupported {
        message,
        what,
        value: value.into(),
    };
    let bits = s[19];
    if bits > MAX_BITS {
        return Err(unsupported("bits per value in JPEG 2000", bits));
    }
    // Table 5.1: 0 floating point, 1 integer. Either unpacks the same way.
    if s[20] > 1 {
        return Err(unsupported("type of original field values", s[20]));
    }
    // Code table 5.40: 0 lossless, 1 lossy.
    if s[21] != 0 {
        return Err(unsupported("type of JPEG 2000 compression", s[21]));
    }
    let reference = f32::from_bits(be_u32(&s[11..15]));
    if !reference.is_finite() {
        return Err(malformed(message, "the reference value is not finite"));
    }
    Ok(Jpeg2000Packing {
        reference,
        binary_scale: be_i16(&s[15..17]),
        decimal_scale: be_i16(&s[17..19]),
        bits,
        count: be_u32(&s[5..9]),
    })
}

fn settings() -> DecodeSettings {
    DecodeSettings {
        // A GRIB2 codestream is raw (no JP2 boxes), so there is no palette to resolve.
        resolve_palette_indices: false,
        ..DecodeSettings::default()
    }
}

/// Reads the codestream's header and checks it holds one sample per packed value, at the bits
/// section 5 gives: `parse` calls it, so a field whose image cannot hold its values is refused
/// before any is read, and the decoder never sizes an image past the grid.
pub(super) fn check(p: &Jpeg2000Packing, data: &[u8], message: usize) -> Result<(), Grib2Error> {
    if p.bits == 0 {
        return Ok(());
    }
    let image = Image::new(data, &settings())
        .map_err(|e| malformed(message, format!("the JPEG 2000 codestream: {e}")))?;
    let samples = u64::from(image.width()) * u64::from(image.height());
    if samples != u64::from(p.count) {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image has {samples} samples ({} by {}) but section 5 packs {} values",
                image.width(),
                image.height(),
                p.count
            ),
        ));
    }
    if image.original_bit_depth() != p.bits {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image has {} bits per sample but section 5 gives {}",
                image.original_bit_depth(),
                p.bits
            ),
        ));
    }
    Ok(())
}

/// Every packed integer, in order. `check` has passed on the same bytes.
pub(super) fn decode(
    p: &Jpeg2000Packing,
    data: &[u8],
    message: usize,
) -> Result<Vec<u32>, Grib2Error> {
    // `parse` bounds `count` by the grid's points, which fit a `usize` on every target.
    let count = usize::try_from(p.count).unwrap_or(0);
    if p.bits == 0 {
        return Ok(vec![0; count]);
    }
    let codestream = |e| malformed(message, format!("the JPEG 2000 codestream: {e}"));
    let image = Image::new(data, &settings()).map_err(codestream)?;
    let mut context = DecoderContext::default();
    let decoded = image.decode(&mut context).map_err(codestream)?;
    let [component] = decoded.components() else {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image has {} components, not one",
                decoded.components().len()
            ),
        ));
    };
    let samples = component.samples();
    if samples.len() != count {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image decodes to {} samples but section 5 packs {count} values",
                samples.len()
            ),
        ));
    }
    let largest = (1_u32 << p.bits) - 1;
    samples
        .iter()
        .map(|&s| {
            whole(s, largest).ok_or_else(|| {
                malformed(
                    message,
                    format!("a JPEG 2000 sample, {s}, is not a whole number from 0 to {largest}"),
                )
            })
        })
        .collect()
}

/// A sample as an integer, when it is a whole number from 0 to `largest` (at most `2^24 − 1`):
/// a lossless codestream's samples all are, and anything else is a codestream this reader must
/// not guess at.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "`largest` is below 2^24, so exact in f32, and `s` is checked whole and in range first"
)]
fn whole(s: f32, largest: u32) -> Option<u32> {
    (s.fract() == 0.0 && (0.0..=largest as f32).contains(&s)).then_some(s as u32)
}
