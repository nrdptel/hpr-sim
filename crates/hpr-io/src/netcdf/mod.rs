//! netCDF classic files: the classic (`CDF\x01`) and 64-bit offset (`CDF\x02`) formats, read
//! from bytes.
//!
//! **Source.** Unidata, *NetCDF File Format Specifications*, "The Classic Format" and "The
//! 64-bit Offset Format" (netCDF-C documentation, captured 2026-09-26), whose grammar this reader
//! follows: a header of dimensions, global attributes and variables, then each non-record
//! variable's values contiguously at its `begin` offset, then the records, each holding one slab of
//! every record variable. Every number is big-endian; names and short values pad to four bytes.
//! The two formats differ only in the width of `begin`, 32 or 64 bits.
//!
//! **Packed data.** [`Variable::packing`] reads the attribute conventions of the netCDF Users
//! Guide and CF Conventions 1.11 §8.1 ("Packed Data") and §2.5.1 ("Missing data, valid and actual
//! range of data"): a stored value `p` means `p · scale_factor + add_offset`, and is missing if it
//! equals `_FillValue` or `missing_value` or lies outside `valid_min`, `valid_max` or
//! `valid_range`. With no valid bound given, the fill bounds the valid range on its own side, as
//! the Users Guide says: the valid range ends one step (integers) or two units in the last place
//! (floats) short of the fill, toward zero, so with a fill of −32767 a stored −32768 is missing
//! too. A byte variable with no `_FillValue` has no fill. netCDF4-python 1.7.4 masks only values equal to the
//! fill, and masks the byte default; the tests pin each value where the two differ.
//!
//! **Not read.** netCDF-4 files are HDF5 underneath (they begin `\x89HDF`) and the CDF-5 format
//! (`CDF\x05`) is not in the cited specification; both are refused with the conversion that makes
//! them readable ([`CONVERSION`]). A file whose record count was never written (`numrecs` "streaming") is
//! refused, as the specification leaves it unimplemented.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Header tag of a dimension list.
const NC_DIMENSION: u32 = 0x0A;
/// Header tag of a variable list.
const NC_VARIABLE: u32 = 0x0B;
/// Header tag of an attribute list.
const NC_ATTRIBUTE: u32 = 0x0C;
/// `numrecs` of a file whose record count was never written.
const STREAMING: u32 = 0xFFFF_FFFF;

/// How to rewrite a netCDF-4 or CDF-5 file as a 64-bit offset file, which this reader reads: with
/// Python's xarray, dropping any variable the classic formats cannot hold (64-bit integers other
/// than times, strings). ERA5 files from the Climate Data Store carry two, `number` and `expver`.
pub const CONVERSION: &str = "xarray.open_dataset(\"in.nc\").drop_vars([\"number\", \"expver\"], errors=\"ignore\").to_netcdf(\"out.nc\", format=\"NETCDF3_64BIT\")";

/// Why a netCDF file could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum NetCdfError {
    /// The bytes are an HDF5 file, as netCDF-4 files are.
    #[error(
        "this is a netCDF-4 (HDF5) file, which is not read; rewrite it as netCDF classic, for example with `{CONVERSION}`"
    )]
    NetCdf4,
    /// The bytes are the CDF-5 (64-bit data) format.
    #[error(
        "this is a CDF-5 (64-bit data) netCDF file, which is not read; rewrite it as netCDF classic, for example with `{CONVERSION}`"
    )]
    Cdf5,
    /// The bytes begin with neither a netCDF nor an HDF5 signature.
    #[error("not a netCDF file: it begins with {head}")]
    NotNetCdf {
        /// The first bytes, printed as hex.
        head: String,
    },
    /// The header or a variable's values run past the end of the bytes.
    #[error("the file ends early: {what} needs bytes {start}..{end} of {len}")]
    Truncated {
        /// What was being read.
        what: String,
        /// First byte needed.
        start: u64,
        /// One past the last byte needed.
        end: u64,
        /// The number of bytes there are.
        len: u64,
    },
    /// The header breaks the specification's grammar.
    #[error("malformed header: {reason}")]
    Malformed {
        /// What is wrong.
        reason: String,
    },
    /// The record count was never written.
    #[error(
        "the record count is \"streaming\" (never written), which the specification leaves unimplemented"
    )]
    Streaming,
    /// A variable's attributes use a convention this reader does not apply.
    #[error("variable `{variable}`: {reason}")]
    Unsupported {
        /// The variable.
        variable: String,
        /// What is not supported.
        reason: String,
    },
}

fn malformed(reason: impl Into<String>) -> NetCdfError {
    NetCdfError::Malformed {
        reason: reason.into(),
    }
}

/// Which of the two classic formats a file is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Format {
    /// `CDF\x01`: offsets are 32 bits.
    Classic,
    /// `CDF\x02`: offsets are 64 bits.
    Offset64,
}

/// A netCDF external type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Type {
    /// `NC_BYTE`: 8-bit signed integers.
    Byte,
    /// `NC_CHAR`: 8-bit characters.
    Char,
    /// `NC_SHORT`: 16-bit signed integers.
    Short,
    /// `NC_INT`: 32-bit signed integers.
    Int,
    /// `NC_FLOAT`: IEEE single precision.
    Float,
    /// `NC_DOUBLE`: IEEE double precision.
    Double,
}

impl Type {
    fn from_tag(tag: u32) -> Result<Self, NetCdfError> {
        Ok(match tag {
            1 => Type::Byte,
            2 => Type::Char,
            3 => Type::Short,
            4 => Type::Int,
            5 => Type::Float,
            6 => Type::Double,
            other => return Err(malformed(format!("unknown type tag {other}"))),
        })
    }

    /// Bytes per value.
    pub fn size(self) -> u64 {
        match self {
            Type::Byte | Type::Char => 1,
            Type::Short => 2,
            Type::Int | Type::Float => 4,
            Type::Double => 8,
        }
    }
}

/// A block of values of one type, in row-major order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Values {
    /// `NC_BYTE`, read as signed (the specification's default).
    Byte(Vec<i8>),
    /// `NC_CHAR`, as the bytes stored; the specification leaves their encoding open.
    Char(Vec<u8>),
    /// `NC_SHORT`.
    Short(Vec<i16>),
    /// `NC_INT`.
    Int(Vec<i32>),
    /// `NC_FLOAT`.
    Float(Vec<f32>),
    /// `NC_DOUBLE`.
    Double(Vec<f64>),
}

impl Values {
    fn decode(kind: Type, bytes: &[u8]) -> Values {
        match kind {
            Type::Byte => Values::Byte(bytes.iter().map(|&b| i8::from_be_bytes([b])).collect()),
            Type::Char => Values::Char(bytes.to_vec()),
            Type::Short => Values::Short(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&c| i16::from_be_bytes(c))
                    .collect(),
            ),
            Type::Int => Values::Int(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&c| i32::from_be_bytes(c))
                    .collect(),
            ),
            Type::Float => Values::Float(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&c| f32::from_be_bytes(c))
                    .collect(),
            ),
            Type::Double => Values::Double(
                bytes
                    .as_chunks::<8>()
                    .0
                    .iter()
                    .map(|&c| f64::from_be_bytes(c))
                    .collect(),
            ),
        }
    }

    /// The type of the values.
    pub fn kind(&self) -> Type {
        match self {
            Values::Byte(_) => Type::Byte,
            Values::Char(_) => Type::Char,
            Values::Short(_) => Type::Short,
            Values::Int(_) => Type::Int,
            Values::Float(_) => Type::Float,
            Values::Double(_) => Type::Double,
        }
    }

    /// The number of values.
    pub fn len(&self) -> usize {
        match self {
            Values::Byte(v) => v.len(),
            Values::Char(v) => v.len(),
            Values::Short(v) => v.len(),
            Values::Int(v) => v.len(),
            Values::Float(v) => v.len(),
            Values::Double(v) => v.len(),
        }
    }

    /// Whether there are no values.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Value `index` as a number, exactly (every type converts to `f64` without rounding); `None`
    /// past the end or for characters.
    pub fn get(&self, index: usize) -> Option<f64> {
        match self {
            Values::Byte(v) => v.get(index).map(|&x| f64::from(x)),
            Values::Char(_) => None,
            Values::Short(v) => v.get(index).map(|&x| f64::from(x)),
            Values::Int(v) => v.get(index).map(|&x| f64::from(x)),
            Values::Float(v) => v.get(index).map(|&x| f64::from(x)),
            Values::Double(v) => v.get(index).copied(),
        }
    }

    /// Characters as text: `None` unless they are UTF-8. Trailing NUL bytes, which some writers
    /// store after an attribute's text, are left off.
    pub fn text(&self) -> Option<&str> {
        match self {
            Values::Char(bytes) => {
                let end = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
                std::str::from_utf8(&bytes[..end]).ok()
            }
            _ => None,
        }
    }
}

/// A dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    /// Its name.
    pub name: String,
    /// Its length; for the record dimension, the number of records.
    pub length: u64,
    /// Whether it is the record (unlimited) dimension.
    pub is_record: bool,
}

/// A named attribute of the file or of a variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Its name.
    pub name: String,
    /// Its values.
    pub values: Values,
}

/// A variable, with all its values read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    /// Its name.
    pub name: String,
    /// Its dimensions' names, outermost first. Each is shared with every other variable along that
    /// dimension, so a header that names one dimension many times costs no copy per axis.
    pub dimensions: Vec<Arc<str>>,
    /// Its length along each dimension (the record count for the record dimension).
    pub shape: Vec<u64>,
    /// Its attributes.
    pub attributes: Vec<Attribute>,
    /// Its values, in row-major order (last dimension varying fastest).
    pub values: Values,
}

impl Variable {
    /// The attribute called `name`.
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }

    /// The row-major position of the value at `index` (one entry per dimension), or `None` if the
    /// index has the wrong rank or is out of range.
    pub fn offset(&self, index: &[u64]) -> Option<usize> {
        if index.len() != self.shape.len() {
            return None;
        }
        let mut offset: u64 = 0;
        for (&i, &n) in index.iter().zip(&self.shape) {
            if i >= n {
                return None;
            }
            offset = offset.checked_mul(n)?.checked_add(i)?;
        }
        usize::try_from(offset).ok()
    }

    /// How this variable's stored values map to physical ones, from its attributes (see the
    /// module documentation).
    ///
    /// # Errors
    ///
    /// [`NetCdfError::Unsupported`] for character data, an `_Unsigned` attribute (whose values
    /// this reader would read as signed), a packing or bound attribute that is not a single
    /// number (two for `valid_range`), or `valid_range` given with `valid_min` or `valid_max`.
    pub fn packing(&self) -> Result<Packing, NetCdfError> {
        let unsupported = |reason: String| NetCdfError::Unsupported {
            variable: self.name.clone(),
            reason,
        };
        let own = self.values.kind();
        if own == Type::Char {
            return Err(unsupported("character data has no numeric values".into()));
        }
        if self.attribute("_Unsigned").is_some() {
            return Err(unsupported("`_Unsigned` data is not read".into()));
        }
        let numbers = |name: &str| -> Result<Option<Vec<f64>>, NetCdfError> {
            match self.attribute(name) {
                None => Ok(None),
                Some(a) if a.values.kind() == Type::Char => {
                    Err(unsupported(format!("`{name}` is text")))
                }
                Some(a) => Ok(Some(
                    (0..a.values.len())
                        .filter_map(|i| a.values.get(i))
                        .collect(),
                )),
            }
        };
        let single = |name: &str| -> Result<Option<f64>, NetCdfError> {
            match numbers(name)? {
                None => Ok(None),
                Some(v) if v.len() == 1 => Ok(Some(v[0])),
                Some(_) => Err(unsupported(format!("`{name}` is not a single number"))),
            }
        };
        let scale_factor = single("scale_factor")?.unwrap_or(1.0);
        let add_offset = single("add_offset")?.unwrap_or(0.0);
        let explicit_fill = single("_FillValue")?;
        // A byte variable with no `_FillValue` has no fill: the default's use is "not
        // recommended" and every byte value is valid.
        let fill = explicit_fill.or(match own {
            Type::Byte | Type::Char => None,
            Type::Short => Some(-32767.0),
            Type::Int => Some(-2_147_483_647.0),
            // FILL_FLOAT and FILL_DOUBLE, both 9.969209968386869e36.
            Type::Float => Some(f64::from(f32::from_bits(0x7CF0_0000))),
            Type::Double => Some(f64::from_bits(0x479E_0000_0000_0000)),
        });
        let mut missing: Vec<f64> = fill.into_iter().collect();
        missing.extend(numbers("missing_value")?.unwrap_or_default());

        let mut valid_min = single("valid_min")?;
        let mut valid_max = single("valid_max")?;
        if let Some(range) = numbers("valid_range")? {
            if valid_min.is_some() || valid_max.is_some() {
                return Err(unsupported(
                    "`valid_range` is given with `valid_min` or `valid_max`".into(),
                ));
            }
            let [low, high] = range[..] else {
                return Err(unsupported("`valid_range` is not two numbers".into()));
            };
            valid_min = Some(low);
            valid_max = Some(high);
        }
        // With no valid bounds, the fill bounds the valid range on its own side, one unit away
        // for integers and two units in the last place for floating point.
        if valid_min.is_none()
            && valid_max.is_none()
            && let Some(fill) = fill.filter(|f| !f.is_nan())
            && (own != Type::Byte || explicit_fill.is_some())
        {
            // Stepped in the variable's own type, so a fill at the largest finite value still
            // has a finite neighbour.
            let toward_zero = |x: f64| match own {
                Type::Float => {
                    let x = x as f32;
                    let step = |y: f32| {
                        if fill > 0.0 {
                            y.next_down()
                        } else {
                            y.next_up()
                        }
                    };
                    f64::from(step(step(x)))
                }
                Type::Double => {
                    let step = |y: f64| {
                        if fill > 0.0 {
                            y.next_down()
                        } else {
                            y.next_up()
                        }
                    };
                    step(step(x))
                }
                _ => {
                    if fill > 0.0 {
                        x - 1.0
                    } else {
                        x + 1.0
                    }
                }
            };
            if fill > 0.0 {
                valid_max = Some(toward_zero(fill));
            } else {
                valid_min = Some(toward_zero(fill));
            }
        }
        Ok(Packing {
            scale_factor,
            add_offset,
            missing,
            valid_min,
            valid_max,
        })
    }

    /// The physical value at `index` (one entry per dimension): `Ok(None)` where it is missing.
    ///
    /// # Errors
    ///
    /// As [`Variable::packing`], and [`NetCdfError::Malformed`] if `index` is out of range.
    pub fn unpacked(&self, index: &[u64]) -> Result<Option<f64>, NetCdfError> {
        let packing = self.packing()?;
        let stored = self
            .offset(index)
            .and_then(|i| self.values.get(i))
            .ok_or_else(|| malformed(format!("index {index:?} is outside `{}`", self.name)))?;
        Ok(packing.unpack(stored))
    }
}

/// How a variable's stored values map to physical ones: [`Variable::packing`]. Every bound is in
/// the stored (packed) values' domain, as the netCDF Users Guide's attribute conventions ask.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Packing {
    /// `scale_factor`, 1 if absent.
    pub scale_factor: f64,
    /// `add_offset`, 0 if absent.
    pub add_offset: f64,
    /// Stored values that mean "missing": the fill value (`_FillValue`, or the type's default
    /// unless the type is byte) and every `missing_value`.
    pub missing: Vec<f64>,
    /// The least valid stored value: `valid_min`, the first of `valid_range`, or, with neither
    /// bound given, just above a fill that is not positive.
    pub valid_min: Option<f64>,
    /// The greatest valid stored value: `valid_max`, the second of `valid_range`, or, with
    /// neither bound given, just below a positive fill.
    pub valid_max: Option<f64>,
}

impl Packing {
    /// The physical value of stored value `stored`, `stored · scale_factor + add_offset`, or `None`
    /// if it is missing or outside the valid range.
    pub fn unpack(&self, stored: f64) -> Option<f64> {
        if stored.is_nan()
            || self.missing.contains(&stored)
            || self.valid_min.is_some_and(|low| stored < low)
            || self.valid_max.is_some_and(|high| stored > high)
        {
            return None;
        }
        Some(stored * self.scale_factor + self.add_offset)
    }
}

/// A netCDF classic or 64-bit offset file, read whole.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetCdf {
    /// Which of the two formats it is.
    pub format: Format,
    /// Its dimensions, in header order.
    pub dimensions: Vec<Dimension>,
    /// Its global attributes.
    pub attributes: Vec<Attribute>,
    /// Its variables, in header order.
    pub variables: Vec<Variable>,
}

impl NetCdf {
    /// Reads a file from its bytes.
    ///
    /// # Errors
    ///
    /// [`NetCdfError`]: a netCDF-4 or CDF-5 file, bytes that are not netCDF, a header that breaks
    /// the grammar, values past the end of the bytes, or a streaming record count.
    pub fn parse(bytes: &[u8]) -> Result<Self, NetCdfError> {
        let format = match bytes.get(..4) {
            Some(b"CDF\x01") => Format::Classic,
            Some(b"CDF\x02") => Format::Offset64,
            Some(b"CDF\x05") => return Err(NetCdfError::Cdf5),
            Some([0x89, b'H', b'D', b'F']) => return Err(NetCdfError::NetCdf4),
            _ => {
                let head = bytes.iter().take(8).map(|b| format!("{b:02x}")).collect();
                return Err(NetCdfError::NotNetCdf { head });
            }
        };
        let mut header = Reader { bytes, position: 4 };
        let numrecs = header.u32("the record count")?;
        if numrecs == STREAMING {
            return Err(NetCdfError::Streaming);
        }
        let numrecs = u64::from(numrecs);

        // Names seen so far go in sets, so a header of many names costs no quadratic time.
        let mut dimensions = Vec::new();
        let mut seen = BTreeSet::new();
        for _ in 0..header.list(NC_DIMENSION, "dimension")? {
            let name = header.name()?;
            if !seen.insert(name.clone()) {
                return Err(malformed(format!("two dimensions are called `{name}`")));
            }
            let length = header.count("a dimension length")?;
            let is_record = length == 0;
            if is_record && dimensions.iter().any(|d: &Dimension| d.is_record) {
                return Err(malformed("more than one record dimension"));
            }
            dimensions.push(Dimension {
                name,
                length: if is_record { numrecs } else { length },
                is_record,
            });
        }
        let attributes = header.attributes()?;

        let mut layouts = Vec::new();
        let mut seen = BTreeSet::new();
        for _ in 0..header.list(NC_VARIABLE, "variable")? {
            let name = header.name()?;
            if !seen.insert(name.clone()) {
                return Err(malformed(format!("two variables are called `{name}`")));
            }
            let rank = header.count("a variable's rank")?;
            let mut dims = Vec::new();
            for axis in 0..rank {
                let id = usize::try_from(header.count("a dimension id")?)
                    .map_err(|_| malformed("dimension id out of range"))?;
                let Some(dimension) = dimensions.get(id) else {
                    return Err(malformed(format!("variable `{name}` names dimension {id}")));
                };
                if dimension.is_record && axis != 0 {
                    return Err(malformed(format!(
                        "variable `{name}` has the record dimension other than first"
                    )));
                }
                dims.push(id);
            }
            let variable_attributes = header.attributes()?;
            let kind = Type::from_tag(header.u32("a variable's type")?)?;
            let _vsize = header.u32("a variable's size")?;
            let begin = match format {
                Format::Classic => u64::from(header.u32("a variable's offset")?),
                Format::Offset64 => header.u64("a variable's offset")?,
            };
            let is_record = dims.first().is_some_and(|&id| dimensions[id].is_record);
            // Values per record for a record variable, or in all for any other.
            let mut count: u64 = 1;
            for &id in dims.iter().skip(usize::from(is_record)) {
                count = count
                    .checked_mul(dimensions[id].length)
                    .ok_or_else(|| malformed(format!("variable `{name}` is too large")))?;
            }
            let slab = count
                .checked_mul(kind.size())
                .ok_or_else(|| malformed(format!("variable `{name}` is too large")))?;
            layouts.push(Layout {
                name,
                dims,
                attributes: variable_attributes,
                kind,
                begin,
                is_record,
                slab,
            });
        }

        // The record size (Note on vsize): each record variable's slab padded to four bytes,
        // except that a lone record variable is not padded (Note on padding; unpadded slabs of
        // the wider types are multiples of four already).
        let record_variables = layouts.iter().filter(|l| l.is_record).count();
        let mut record_size: u64 = 0;
        for layout in layouts.iter().filter(|l| l.is_record) {
            let padded = if record_variables == 1 {
                layout.slab
            } else {
                pad4(layout.slab)?
            };
            record_size = record_size
                .checked_add(padded)
                .ok_or_else(|| malformed("the record size is too large"))?;
        }

        // In a well-formed file no two variables share a byte, so together they hold no more than
        // the file does. A header whose variables overlap could otherwise make a small file decode
        // into far more memory than it takes.
        let mut claimed: u64 = 0;
        for layout in &layouts {
            let bytes_held = if layout.is_record {
                layout.slab.checked_mul(numrecs)
            } else {
                Some(layout.slab)
            };
            claimed = bytes_held
                .and_then(|b| claimed.checked_add(b))
                .ok_or_else(|| malformed("the variables' sizes overflow"))?;
        }
        if claimed > bytes.len() as u64 {
            return Err(malformed(format!(
                "the variables hold {claimed} bytes, more than the file's {}",
                bytes.len()
            )));
        }

        let names: Vec<Arc<str>> = dimensions
            .iter()
            .map(|d: &Dimension| Arc::from(d.name.as_str()))
            .collect();
        let mut variables = Vec::with_capacity(layouts.len());
        for layout in layouts {
            let values = if layout.is_record {
                let mut data = Vec::new();
                for record in 0..numrecs {
                    let start = record
                        .checked_mul(record_size)
                        .and_then(|offset| offset.checked_add(layout.begin))
                        .ok_or_else(|| malformed("a record offset is too large"))?;
                    data.extend_from_slice(slice(bytes, start, layout.slab, &layout.name)?);
                }
                Values::decode(layout.kind, &data)
            } else {
                Values::decode(
                    layout.kind,
                    slice(bytes, layout.begin, layout.slab, &layout.name)?,
                )
            };
            variables.push(Variable {
                name: layout.name,
                dimensions: layout
                    .dims
                    .iter()
                    .map(|&id| Arc::clone(&names[id]))
                    .collect(),
                shape: layout
                    .dims
                    .iter()
                    .map(|&id| dimensions[id].length)
                    .collect(),
                attributes: layout.attributes,
                values,
            });
        }

        Ok(NetCdf {
            format,
            dimensions,
            attributes,
            variables,
        })
    }

    /// The variable called `name`.
    pub fn variable(&self, name: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.name == name)
    }

    /// The dimension called `name`.
    pub fn dimension(&self, name: &str) -> Option<&Dimension> {
        self.dimensions.iter().find(|d| d.name == name)
    }

    /// The global attribute called `name`.
    pub fn attribute(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }
}

/// A variable's header entry, before its values are read.
struct Layout {
    name: String,
    dims: Vec<usize>,
    attributes: Vec<Attribute>,
    kind: Type,
    begin: u64,
    is_record: bool,
    /// Bytes of values per record (record variables) or in all (others), unpadded.
    slab: u64,
}

/// `n` rounded up to a multiple of four, or an error if that overflows (a header can make a
/// record variable's slab as large as `u64::MAX`).
fn pad4(n: u64) -> Result<u64, NetCdfError> {
    n.div_ceil(4)
        .checked_mul(4)
        .ok_or_else(|| malformed(format!("a size of {n} bytes is too large")))
}

/// `len` bytes of `bytes` from `start`, or [`NetCdfError::Truncated`].
fn slice<'a>(bytes: &'a [u8], start: u64, len: u64, what: &str) -> Result<&'a [u8], NetCdfError> {
    let total = bytes.len() as u64;
    let truncated = || NetCdfError::Truncated {
        what: what.to_owned(),
        start,
        end: start.saturating_add(len),
        len: total,
    };
    let end = start.checked_add(len).ok_or_else(truncated)?;
    if end > total {
        return Err(truncated());
    }
    let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
        return Err(truncated());
    };
    Ok(&bytes[start..end])
}

/// A cursor over the header.
struct Reader<'a> {
    bytes: &'a [u8],
    position: u64,
}

impl Reader<'_> {
    fn take(&mut self, len: u64, what: &str) -> Result<&[u8], NetCdfError> {
        let taken = slice(self.bytes, self.position, len, what)?;
        self.position += len;
        Ok(taken)
    }

    fn u32(&mut self, what: &str) -> Result<u32, NetCdfError> {
        let b = self.take(4, what)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self, what: &str) -> Result<u64, NetCdfError> {
        let b = self.take(8, what)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// A signed count read as `NON_NEG`: the specification's integers are signed, so a count
    /// with the top bit set is malformed.
    fn count(&mut self, what: &str) -> Result<u64, NetCdfError> {
        let n = self.u32(what)?;
        if n > i32::MAX as u32 {
            return Err(malformed(format!("{what} is negative")));
        }
        Ok(u64::from(n))
    }

    /// The length of a list tagged `tag`, or 0 if it is `ABSENT`.
    fn list(&mut self, tag: u32, what: &str) -> Result<u64, NetCdfError> {
        let found = self.u32(&format!("the {what} list's tag"))?;
        let n = self.count(&format!("the {what} count"))?;
        if found == 0 && n == 0 {
            return Ok(0);
        }
        if found != tag {
            return Err(malformed(format!(
                "the {what} list has tag {found:#x}, not {tag:#x}"
            )));
        }
        Ok(n)
    }

    fn name(&mut self) -> Result<String, NetCdfError> {
        let n = self.count("a name's length")?;
        let raw = self.take(pad4(n)?, "a name")?;
        let text = std::str::from_utf8(&raw[..raw.len().min(n as usize)])
            .map_err(|_| malformed("a name is not UTF-8"))?;
        Ok(text.to_owned())
    }

    fn attributes(&mut self) -> Result<Vec<Attribute>, NetCdfError> {
        let mut attributes = Vec::new();
        let mut seen = BTreeSet::new();
        for _ in 0..self.list(NC_ATTRIBUTE, "attribute")? {
            let name = self.name()?;
            if !seen.insert(name.clone()) {
                return Err(malformed(format!("two attributes are called `{name}`")));
            }
            let kind = Type::from_tag(self.u32("an attribute's type")?)?;
            let n = self.count("an attribute's length")?;
            let size = n
                .checked_mul(kind.size())
                .ok_or_else(|| malformed(format!("attribute `{name}` is too large")))?;
            let raw = self.take(pad4(size)?, &format!("attribute `{name}`"))?;
            let values = Values::decode(kind, &raw[..size as usize]);
            attributes.push(Attribute { name, values });
        }
        Ok(attributes)
    }
}

#[cfg(test)]
mod tests;
