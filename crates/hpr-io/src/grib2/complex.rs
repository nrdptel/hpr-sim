//! Complex packing, with and without spatial differencing: data representation templates 5.2
//! and 5.3, and the data they pack (templates 7.2 and 7.3).
//!
//! The values are split into groups. Each group has a reference `X1`, a width `W` in bits and a
//! length `L`; its values are `X1 + X2` with `X2` an integer of `W` bits (none when `W = 0`, so the
//! group is `X1` throughout). Section 7 holds, in order: for 5.3, the extra descriptors (the first
//! one or two values and the overall minimum of the differences, each a fixed number of bytes,
//! [`SpatialDifferencing::octets`], the minimum signed); every group's reference, then every
//! group's width above the reference width, then every group's length as a scaled number `l`,
//! each list padded to a whole byte; then the groups' values, one after another. A group's
//! length is `L = L_ref + l · increment`, except the last's, which is given as it is (WMO-No. 306, Volume I.2, templates 5.2, 5.3, 7.2, 7.3 and Regulation 92.9.4).
//!
//! **Missing values** (code table 5.5): with primary missing values, a group of width `W > 0`
//! marks a missing value by `X2 = 2^W − 1`, and a group of width 0 is missing throughout when its
//! reference is all ones; with secondary ones too, `2^W − 2` (or a reference of all ones but the
//! last bit) marks the secondary kind. Both read as no value.
//!
//! **Spatial differencing** (5.3) packs differences of the values, taken over the values that are
//! not missing, in grid order. With `h` the reconstructed integers, `z` the packed ones and `m`
//! the overall minimum:
//!
//! - first order: `h₁` is given, and `hₙ = zₙ + m + hₙ₋₁`;
//! - second order: `h₁` and `h₂` are given, and `hₙ = zₙ + m + 2 hₙ₋₁ − hₙ₋₂`.
//!
//! The packed integers in the first one or two places are not used. Each `h` then unpacks as simple
//! packing does, `Y = (R + h · 2^E) / 10^D`.

use serde::{Deserialize, Serialize};

use super::{Grib2Error, be_i16, be_u32, malformed, need, truncated};

/// Complex packing's parameters: data representation templates 5.2 and 5.3.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComplexPacking {
    /// The reference value `R`, as written (a 32-bit float).
    pub reference: f32,
    /// The binary scale factor `E`.
    pub binary_scale: i16,
    /// The decimal scale factor `D`.
    pub decimal_scale: i16,
    /// How many values are packed: the grid points the bitmap marks, or all of them.
    pub count: u32,
    /// Bits per group reference.
    pub group_reference_bits: u8,
    /// How missing values are marked (code table 5.5): 0 not at all, 1 primary, 2 primary and
    /// secondary.
    pub missing_values: u8,
    /// The number of groups.
    pub groups: u32,
    /// The reference for group widths, bits.
    pub width_reference: u8,
    /// Bits per group width.
    pub width_bits: u8,
    /// The reference for group lengths.
    pub length_reference: u32,
    /// The increment for group lengths.
    pub length_increment: u8,
    /// The last group's length, as it is.
    pub last_length: u32,
    /// Bits per scaled group length.
    pub length_bits: u8,
    /// Template 5.3's spatial differencing; `None` for 5.2.
    pub spatial_differencing: Option<SpatialDifferencing>,
}

/// Template 5.3's spatial differencing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialDifferencing {
    /// The order, 1 or 2 (code table 5.6).
    pub order: u8,
    /// Bytes per extra descriptor in section 7, 1 to 8.
    pub octets: u8,
}

impl ComplexPacking {
    /// `Y = (R + h · 2^E) / 10^D` for a reconstructed integer `h`.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "a valid field's integers are far below 2^53; a hostile one's only lose digits"
    )]
    pub fn unpack(&self, h: i64) -> f64 {
        super::unpack(
            self.reference,
            self.binary_scale,
            self.decimal_scale,
            h as f64,
        )
    }
}

/// Section 5, template 5.2 or 5.3, from the section's bytes.
pub(super) fn read(s: &[u8], template: u16, message: usize) -> Result<ComplexPacking, Grib2Error> {
    let s = need(s, 47, "data template 5.2", message)?;
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
    // Code table 5.4: 1 is general group splitting. With 0, row by row, the group lengths
    // have no meaning (note 1 to template 5.2).
    if s[21] != 1 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "group splitting method",
            value: s[21].into(),
        });
    }
    let missing_values = s[22];
    if missing_values > 2 {
        return Err(Grib2Error::Unsupported {
            message,
            what: "missing value management",
            value: missing_values.into(),
        });
    }
    let spatial_differencing = if template == 3 {
        let s = need(s, 49, "data template 5.3", message)?;
        let (order, octets) = (s[47], s[48]);
        if !(1..=2).contains(&order) {
            return Err(Grib2Error::Unsupported {
                message,
                what: "order of spatial differencing",
                value: order.into(),
            });
        }
        if !(1..=8).contains(&octets) {
            return Err(Grib2Error::Unsupported {
                message,
                what: "bytes per extra descriptor",
                value: octets.into(),
            });
        }
        Some(SpatialDifferencing { order, octets })
    } else {
        None
    };
    let packing = ComplexPacking {
        reference,
        binary_scale: be_i16(&s[15..17]),
        decimal_scale: be_i16(&s[17..19]),
        count: be_u32(&s[5..9]),
        group_reference_bits: s[19],
        missing_values,
        groups: be_u32(&s[31..35]),
        width_reference: s[35],
        width_bits: s[36],
        length_reference: be_u32(&s[37..41]),
        length_increment: s[41],
        last_length: be_u32(&s[42..46]),
        length_bits: s[46],
        spatial_differencing,
    };
    for (what, bits) in [
        ("bits per group reference", packing.group_reference_bits),
        ("bits per group width", packing.width_bits),
        ("bits per group length", packing.length_bits),
    ] {
        if bits > 32 {
            return Err(Grib2Error::Unsupported {
                message,
                what,
                value: bits.into(),
            });
        }
    }
    if missing_values == 2 && packing.group_reference_bits == 0 {
        // A 0-bit reference is all ones, so every group of width 0 is missing (ecCodes writes a
        // field with no values that way); all ones less one, the secondary code, has no bits.
        return Err(Grib2Error::Unsupported {
            message,
            what: "secondary missing values with 0-bit group references, management",
            value: missing_values.into(),
        });
    }
    Ok(packing)
}

/// Where each part of section 7 starts, in bits from the start of its data, found by
/// [`layout`], with the packing it was checked against: the field's public copy may be changed,
/// this one not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Layout {
    packing: ComplexPacking,
    /// The first one or two values, and the overall minimum of the differences.
    first: [i64; 2],
    minimum: i64,
    references: u64,
    widths: u64,
    lengths: u64,
    values: u64,
}

/// Checks section 7 holds what the packing says, reading each group's width and length once,
/// and gives where its parts start.
///
/// The groups' lengths must add up to the packed count and their values fit the data; a group
/// wider than 32 bits is refused. The work is bounded by the file's size: the lists of widths and
/// lengths hold a group each, and when both are 0 bits wide every group is alike and is counted
/// at once.
pub(super) fn layout(
    p: &ComplexPacking,
    data: &[u8],
    message: usize,
) -> Result<Layout, Grib2Error> {
    let have = data.len() as u64 * 8;
    let groups = u64::from(p.groups);
    let count = u64::from(p.count);
    if groups == 0 && count > 0 {
        // ecCodes reads this as the reference everywhere (its issue ECC-2095); the regulation
        // doesn't say.
        return Err(Grib2Error::Unsupported {
            message,
            what: "complex packing with no groups, values",
            value: count,
        });
    }
    if groups > count {
        return Err(malformed(
            message,
            format!("{groups} groups for {count} packed values"),
        ));
    }
    let (mut first, mut minimum, mut at) = ([0; 2], 0, 0_u64);
    if let Some(d) = p.spatial_differencing {
        let octets = usize::from(d.octets);
        let len = octets * (usize::from(d.order) + 1);
        if data.len() < len {
            return Err(truncated(message, "the extra descriptors", len, data.len()));
        }
        let field = |k: usize| &data[k * octets..(k + 1) * octets];
        for (k, slot) in first.iter_mut().take(usize::from(d.order)).enumerate() {
            // The first values are scaled values less the reference, so not negative. ecCodes
            // reads them unsigned and NCEP's g2clib as sign and magnitude, so a top bit set
            // would be read two ways.
            if field(k)[0] & 0x80 != 0 {
                return Err(malformed(
                    message,
                    "a first value has its top bit set, which decoders read two ways",
                ));
            }
            *slot = sign_magnitude(field(k));
        }
        minimum = sign_magnitude(field(usize::from(d.order)));
        at = len as u64 * 8;
    }
    let list = |at: u64, bits: u8| at + (groups * u64::from(bits)).div_ceil(8) * 8;
    let references = at;
    let widths = list(references, p.group_reference_bits);
    let lengths = list(widths, p.width_bits);
    let values = list(lengths, p.length_bits);
    if values > have {
        return Err(truncated(
            message,
            "the group descriptors",
            values.div_ceil(8),
            data.len(),
        ));
    }
    let wide = |width: u64| Grib2Error::Unsupported {
        message,
        what: "group width, bits",
        value: width,
    };
    let (mut total, mut bits) = (0_u64, 0_u64);
    if p.width_bits == 0 && p.length_bits == 0 {
        // Every group but the last is `length_reference` long, and all `width_reference` wide.
        let width = u64::from(p.width_reference);
        if width > 32 {
            return Err(wide(width));
        }
        // Below 2^64 (`groups`, `length_reference` and `last_length` are 32 bits); `bits` only
        // counts when the lengths add up. No groups hold no values (the count is then 0 too).
        total = groups.checked_sub(1).map_or(0, |g| {
            g * u64::from(p.length_reference) + u64::from(p.last_length)
        });
        bits = if total <= count { total * width } else { 0 };
    } else {
        for k in 0..groups {
            let (width, length) = group(p, data, widths, lengths, k);
            if width > 32 {
                return Err(wide(width));
            }
            total += length;
            // A group is at most 2^40 long and 32 bits wide, and `total ≤ count ≤ 2^32` before
            // it, so neither sum can overflow.
            bits += length * width;
            if total > count {
                break;
            }
        }
    }
    if total != count {
        return Err(malformed(
            message,
            format!("the groups hold {total} values but section 5 packs {count}"),
        ));
    }
    if values + bits > have {
        return Err(truncated(
            message,
            "the packed values",
            (values + bits).div_ceil(8),
            data.len(),
        ));
    }
    Ok(Layout {
        packing: *p,
        first,
        minimum,
        references,
        widths,
        lengths,
        values,
    })
}

/// Group `k`'s width and length.
fn group(p: &ComplexPacking, data: &[u8], widths: u64, lengths: u64, k: u64) -> (u64, u64) {
    let wb = u64::from(p.width_bits);
    let width = u64::from(p.width_reference) + bits(data, widths + k * wb, p.width_bits);
    let length = if k + 1 == u64::from(p.groups) {
        u64::from(p.last_length)
    } else {
        let lb = u64::from(p.length_bits);
        u64::from(p.length_reference)
            + bits(data, lengths + k * lb, p.length_bits) * u64::from(p.length_increment)
    };
    (width, length)
}

/// `n ≤ 32` bits from bit `start`, first bit high; bits past the data read as 0 (`layout` checked
/// the data holds every bit read).
pub(super) fn bits(data: &[u8], start: u64, n: u8) -> u64 {
    if n == 0 {
        return 0;
    }
    let byte = usize::try_from(start / 8).unwrap_or(usize::MAX);
    let mut word = [0_u8; 8];
    let rest = data.get(byte..).unwrap_or(&[]);
    let take = rest.len().min(8);
    word[..take].copy_from_slice(&rest[..take]);
    // `start % 8 ≤ 7` and `n ≤ 32`, so the bits wanted are inside the 64 read.
    (u64::from_be_bytes(word) << (start % 8)) >> (64 - u32::from(n))
}

/// A sign-and-magnitude integer of 1 to 8 bytes.
fn sign_magnitude(b: &[u8]) -> i64 {
    let mut raw: u64 = 0;
    for &byte in b {
        raw = (raw << 8) | u64::from(byte);
    }
    let top = 8 * b.len() as u32 - 1;
    // Below 2^63 once the sign bit is cleared.
    let magnitude = i64::try_from(raw & ((1_u64 << top) - 1)).unwrap_or(i64::MAX);
    if raw >> top & 1 == 1 {
        -magnitude
    } else {
        magnitude
    }
}

/// Decodes the packed values in order, up to (not including) position `end`, calling `sink`
/// with each position and its value, or `None` for a missing value.
///
/// # Errors
/// [`Grib2Error::Malformed`] when reconstructing the differences overflows 64 bits, which only a
/// broken file does.
pub(super) fn decode(
    layout: &Layout,
    data: &[u8],
    end: u64,
    message: usize,
    mut sink: impl FnMut(u64, Option<f64>),
) -> Result<(), Grib2Error> {
    let p = &layout.packing;
    let order = p.spatial_differencing.map_or(0, |d| d.order);
    let rb = p.group_reference_bits;
    let all_ones = |bits: u64| (1_u64 << bits) - 1;
    let overflow = || malformed(message, "the spatial differences overflow 64 bits");
    let (mut position, mut at, mut present) = (0_u64, layout.values, 0_u64);
    let (mut previous, mut before) = (0_i64, 0_i64);
    for k in 0..u64::from(p.groups) {
        if position >= end {
            break;
        }
        let (width, length) = group(p, data, layout.widths, layout.lengths, k);
        let reference = bits(data, layout.references + k * u64::from(rb), rb);
        for _ in 0..length {
            if position >= end {
                return Ok(());
            }
            // `layout` refused a group wider than 32 bits.
            let w = u8::try_from(width).unwrap_or(32);
            let (x, missing) = if width == 0 {
                let r = u64::from(rb);
                (
                    reference,
                    (p.missing_values >= 1 && reference == all_ones(r))
                        || (p.missing_values == 2 && reference == all_ones(r) - 1),
                )
            } else {
                let x2 = bits(data, at, w);
                at += width;
                (
                    reference + x2,
                    (p.missing_values >= 1 && x2 == all_ones(width))
                        || (p.missing_values == 2 && x2 == all_ones(width) - 1),
                )
            };
            if missing {
                sink(position, None);
                position += 1;
                continue;
            }
            // `reference + x2 < 2^33`.
            let z = i64::try_from(x).unwrap_or(i64::MAX);
            let h = match (order, present) {
                (0, _) => z,
                (_, 0) => layout.first[0],
                (2, 1) => layout.first[1],
                (1, _) => z
                    .checked_add(layout.minimum)
                    .and_then(|v| v.checked_add(previous))
                    .ok_or_else(overflow)?,
                _ => z
                    .checked_add(layout.minimum)
                    .and_then(|v| v.checked_add(previous.checked_mul(2)?))
                    .and_then(|v| v.checked_sub(before))
                    .ok_or_else(overflow)?,
            };
            before = previous;
            previous = h;
            present += 1;
            sink(position, Some(p.unpack(h)));
            position += 1;
        }
    }
    Ok(())
}
