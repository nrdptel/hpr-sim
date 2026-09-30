//! `cargo xtask grib2-values`: every value `hpr_io::grib2` decodes from a GRIB2 file, for
//! `validation/oracles/grib2/whole_file.py` to compare with ecCodes'.

use std::io::Write;

pub const USAGE: &str = "  \
grib2-values FILE        Write every field's values as hpr_io::grib2 decodes them, for
                           validation/oracles/grib2/whole_file.py: per field, its point
                           count (u64) then each value (f64, NaN where there is none),
                           little-endian, on stdout.";

pub fn run(args: &[String]) -> Result<(), String> {
    let [path] = args else {
        return Err(format!("grib2-values takes one file\n\n{USAGE}"));
    };
    let bytes = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    let fields = hpr_io::grib2::parse(&bytes).map_err(|e| format!("{path}: {e}"))?;
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    let write = |out: &mut std::io::BufWriter<_>, b: &[u8]| {
        out.write_all(b).map_err(|e| format!("stdout: {e}"))
    };
    for field in &fields {
        let values = field
            .values()
            .map_err(|e| format!("{path}, message {}: {e}", field.message))?;
        write(&mut out, &(values.len() as u64).to_le_bytes())?;
        let mut buf = Vec::with_capacity(values.len() * 8);
        for v in values {
            buf.extend_from_slice(&v.unwrap_or(f64::NAN).to_le_bytes());
        }
        write(&mut out, &buf)?;
    }
    out.flush().map_err(|e| format!("stdout: {e}"))
}
