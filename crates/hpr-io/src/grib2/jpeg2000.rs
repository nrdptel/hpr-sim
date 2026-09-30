//! JPEG 2000 packing: data representation template 5.40 (WMO-No. 306, Volume I.2, FM 92 GRIB
//! edition 2, template 5.40 and code table 5.40).
//!
//! The field's integers `X` are a greyscale image, one sample per packed value, coded as a JPEG
//! 2000 codestream (ISO/IEC 15444-1) in section 7; each unpacks as simple packing's do,
//! `Y = (R + X · 2^E) / 10^D`. The codestream is decoded by `hayro-jpeg2000` (MIT or Apache-2.0),
//! in strict mode, so a damaged or cut-short codestream is refused rather than filled in.
//!
//! The decoder runs the reversible 5/3 wavelet (ISO/IEC 15444-1, Annex F) in `f32`, whose whole
//! numbers are exact only below `2^24`. The inverse transform adds two neighbouring high-pass
//! coefficients before each floor, and for `B`-bit samples those sums reach nearly `16 · 2^(B−1)`
//! (the cascaded 5/3 analysis filters bound a coefficient by about `8.2 · 2^(B−1)`), so every step
//! stays exact only for `B ≤ 21`: fields of more bits are refused by name. Probes at 24 bits came
//! back off by one in 121 and 136 of 10,152 values, and one of six at 23 bits in one value, whole
//! and in range, which no later check could catch.
//!
//! Only the codestream NCEP and ecCodes write is read, checked from its main header before any
//! decoding (ISO/IEC 15444-1, Annex A): one unsigned component, no subsampling or offsets, one
//! tile, the reversible 5/3 transform without quantization, default precincts, and no marker that
//! overrides these per component or per tile. Anything else is refused by name, so a hostile
//! header cannot make the decoder size its buffers past the grid. Lossy coding (code table 5.40's
//! 1) is refused too.

use hayro_jpeg2000::{DecodeSettings, DecoderContext, Image};
use serde::{Deserialize, Serialize};

use super::{Grib2Error, be_i16, be_u32, malformed, need, unpack};

/// The most bits per value read: the 5/3 wavelet's sums in `hayro-jpeg2000`'s `f32` stay below
/// `2^24`, and so exact, for samples of at most 21 bits (see the module's documentation).
pub const MAX_BITS: u8 = 21;

/// `hayro-jpeg2000` refuses an image wider or taller than this. NCEP codes a field with a bitmap
/// as one row of its packed values, so such a field of more values is refused by name.
pub const MAX_SIDE: u32 = 60_000;

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
    /// `R / 10^D` at every point with a value, by the regulation (not checked against ecCodes,
    /// which gave `R` for a field of 0 bits with `D = 2`).
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
        // Refuse what the lenient mode fills in: a cut-short codestream decodes there to the DC
        // offset at every point, whole numbers in range.
        strict: true,
        ..DecodeSettings::default()
    }
}

/// Reads the codestream's main header and checks it holds one sample per packed value, at the
/// bits section 5 gives, coded as NCEP and ecCodes code it: `parse` calls it, so a field whose
/// image cannot hold its values, or that the decoder could size past the grid, is refused before
/// any is read.
pub(super) fn check(p: &Jpeg2000Packing, data: &[u8], message: usize) -> Result<(), Grib2Error> {
    if p.bits == 0 {
        return Ok(());
    }
    let header = Header::read(data, message)?;
    let samples = u64::from(header.width) * u64::from(header.height);
    if samples != u64::from(p.count) {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image has {samples} samples ({} by {}) but section 5 packs {} values",
                header.width, header.height, p.count
            ),
        ));
    }
    if header.bits != p.bits {
        return Err(malformed(
            message,
            format!(
                "the JPEG 2000 image has {} bits per sample but section 5 gives {}",
                header.bits, p.bits
            ),
        ));
    }
    Ok(())
}

/// What a coding marker sets.
enum Coding {
    /// COD, the default coding style.
    Style,
    /// QCD, the default quantization.
    Quantization,
    /// COC, QCC or COM.
    Other,
}

/// What `check` needs from a codestream's main header (ISO/IEC 15444-1, Annex A).
struct Header {
    width: u32,
    height: u32,
    bits: u8,
}

impl Header {
    /// Walks the main header and the tile-part headers, refusing by name anything but the one
    /// coding NCEP and ecCodes use.
    fn read(data: &[u8], message: usize) -> Result<Self, Grib2Error> {
        let bad = |reason: &str| malformed(message, format!("the JPEG 2000 codestream: {reason}"));
        let unsupported = |what, value: u64| Grib2Error::Unsupported {
            message,
            what,
            value,
        };
        let u16_at = |at: usize| -> Result<u16, Grib2Error> {
            data.get(at..at.saturating_add(2))
                .map(|b| u16::from_be_bytes([b[0], b[1]]))
                .ok_or_else(|| bad("its header is cut short"))
        };
        let u32_at = |at: usize| -> Result<u32, Grib2Error> {
            data.get(at..at.saturating_add(4))
                .map(be_u32)
                .ok_or_else(|| bad("its header is cut short"))
        };
        let byte = |at: usize| -> Result<u8, Grib2Error> {
            data.get(at)
                .copied()
                .ok_or_else(|| bad("its header is cut short"))
        };
        if u16_at(0)? != 0xFF4F || u16_at(2)? != 0xFF51 {
            return Err(bad("it does not start with SOC and SIZ"));
        }
        // SIZ, from byte 2: Lsiz, Rsiz, Xsiz, Ysiz, XOsiz, YOsiz, XTsiz, YTsiz, XTOsiz, YTOsiz,
        // Csiz, then Ssiz, XRsiz, YRsiz per component.
        let (x, y) = (u32_at(8)?, u32_at(12)?);
        if x > MAX_SIDE || y > MAX_SIDE {
            return Err(unsupported("JPEG 2000 image side", x.max(y).into()));
        }
        let (x0, y0) = (u32_at(16)?, u32_at(20)?);
        let (xt, yt) = (u32_at(24)?, u32_at(28)?);
        let (xt0, yt0) = (u32_at(32)?, u32_at(36)?);
        let components = u16_at(40)?;
        if components != 1 {
            return Err(unsupported("JPEG 2000 components", components.into()));
        }
        let ssiz = byte(42)?;
        if ssiz & 0x80 != 0 {
            return Err(unsupported("JPEG 2000 signed samples", ssiz.into()));
        }
        let (xr, yr) = (byte(43)?, byte(44)?);
        if (xr, yr) != (1, 1) {
            return Err(unsupported(
                "JPEG 2000 subsampling",
                u64::from(xr) << 8 | u64::from(yr),
            ));
        }
        if x0 != 0 || y0 != 0 || xt0 != 0 || yt0 != 0 {
            return Err(unsupported(
                "JPEG 2000 image or tile offset",
                u64::from(x0.max(y0).max(xt0).max(yt0)),
            ));
        }
        if xt < x || yt < y {
            return Err(unsupported(
                "JPEG 2000 tiles",
                u64::from(x.div_ceil(xt.max(1))) * u64::from(y.div_ceil(yt.max(1))),
            ));
        }
        // The first marker after SIZ: SIZ's marker at byte 2, then its length.
        // SIZ, COD, COC and SOT are read field by field by the decoder, not skipped by their
        // length, so a longer length would hide a segment from this walk that the decoder reads.
        let fixed = |at: usize, length: u16| -> Result<(), Grib2Error> {
            let marker = u16_at(at)?;
            let have = u16_at(at.saturating_add(2))?;
            if have == length {
                Ok(())
            } else {
                Err(malformed(
                    message,
                    format!(
                        "the JPEG 2000 codestream: marker {marker:#06X} is {have} bytes long, not {length}"
                    ),
                ))
            }
        };
        // The markers that set the coding, in the main header or a tile-part's, held to the one
        // coding read: `None` for any other marker.
        let coding = |marker: u16, at: usize| -> Result<Option<Coding>, Grib2Error> {
            // COD's and COC's style byte: bit 0 set when precinct sizes are given.
            let precincts = |scod: u8| {
                if scod & 1 == 0 {
                    Ok(())
                } else {
                    Err(unsupported("JPEG 2000 precinct sizes", scod.into()))
                }
            };
            let transform = |t: u8| {
                if t == 1 {
                    Ok(())
                } else {
                    Err(unsupported("JPEG 2000 wavelet transform", t.into()))
                }
            };
            // Sqcd's and Sqcc's low five bits: the quantization style, 0 for none.
            let quantization = |sq: u8| {
                if sq & 0x1F == 0 {
                    Ok(())
                } else {
                    Err(unsupported("JPEG 2000 quantization", (sq & 0x1F).into()))
                }
            };
            Ok(Some(match marker {
                // COD: Scod, SGcod (progression, layers, colour transform), SPcod (levels,
                // code-block width and height, style, transform).
                0xFF52 => {
                    fixed(at, 12)?;
                    precincts(byte(at + 4)?)?;
                    transform(byte(at + 13)?)?;
                    Coding::Style
                }
                // COC: Ccoc (one byte, with fewer than 257 components), Scoc, SPcoc.
                0xFF53 => {
                    fixed(at, 9)?;
                    precincts(byte(at + 5)?)?;
                    transform(byte(at + 10)?)?;
                    Coding::Other
                }
                // QCD: Sqcd.
                0xFF5C => {
                    quantization(byte(at + 4)?)?;
                    Coding::Quantization
                }
                // QCC: Cqcc (one byte), Sqcc.
                0xFF5D => {
                    quantization(byte(at + 5)?)?;
                    Coding::Other
                }
                // COM.
                0xFF64 => Coding::Other,
                _ => return Ok(None),
            }))
        };
        // The first marker after SIZ: SIZ's marker at byte 2, then its length, which for one
        // component is 41.
        fixed(2, 41)?;
        let mut at = 4 + usize::from(u16_at(4)?);
        let (mut cod, mut qcd) = (false, false);
        loop {
            let marker = u16_at(at)?;
            // SOT: the first tile-part starts.
            if marker == 0xFF90 {
                break;
            }
            let length = usize::from(u16_at(at.saturating_add(2))?);
            if length < 2 {
                return Err(bad("a marker segment is shorter than its length field"));
            }
            match coding(marker, at)? {
                Some(Coding::Style) => cod = true,
                Some(Coding::Quantization) => qcd = true,
                Some(Coding::Other) => {}
                // TLM and PLM carry no coding.
                None if matches!(marker, 0xFF55 | 0xFF57) => {}
                None => return Err(unsupported("JPEG 2000 main-header marker", marker.into())),
            }
            at = at.saturating_add(2 + length);
        }
        if !(cod && qcd) {
            return Err(bad("its main header has no COD or no QCD"));
        }
        // Each tile-part: SOT (Lsot, Isot, Psot, TPsot, TNsot), then markers up to SOD.
        while u16_at(at)? == 0xFF90 {
            fixed(at, 10)?;
            let psot = usize::try_from(u32_at(at.saturating_add(6))?).unwrap_or(usize::MAX);
            let mut marker_at = at.saturating_add(2 + usize::from(u16_at(at.saturating_add(2))?));
            loop {
                let marker = u16_at(marker_at)?;
                if marker == 0xFF93 {
                    break;
                }
                // PLT carries no coding.
                if coding(marker, marker_at)?.is_none() && marker != 0xFF58 {
                    return Err(unsupported("JPEG 2000 tile-part marker", marker.into()));
                }
                let length = usize::from(u16_at(marker_at.saturating_add(2))?);
                if length < 2 {
                    return Err(bad("a marker segment is shorter than its length field"));
                }
                marker_at = marker_at.saturating_add(2 + length);
            }
            // Psot 0: the tile-part runs to the end of the codestream.
            if psot == 0 {
                break;
            }
            if psot < 14 {
                return Err(bad("a tile-part is shorter than its header"));
            }
            at = at.saturating_add(psot);
            if at.saturating_add(2) > data.len() || u16_at(at)? == 0xFFD9 {
                break;
            }
        }
        let bits = (ssiz & 0x7F) + 1;
        Ok(Self {
            width: x,
            height: y,
            bits,
        })
    }
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
    // `read` caps `bits`; a caller who edited the public packing past it is refused here.
    if p.bits > MAX_BITS {
        return Err(Grib2Error::Unsupported {
            message,
            what: "bits per value in JPEG 2000",
            value: p.bits.into(),
        });
    }
    check(p, data, message)?;
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
