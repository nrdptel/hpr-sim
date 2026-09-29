//! `cargo xtask format`: writes the hpr design format's JSON Schema (M3.3a) from its types.

use std::path::Path;

/// The command's line in `cargo xtask help`.
pub const USAGE: &str = "  \
format [--check]         Write the hpr design format's JSON Schema to
                           schema/format/hpr-design-<version>.schema.json from its types.
                           --check fails if it is stale.";

/// Runs the command.
pub fn run(args: &[String]) -> Result<(), String> {
    let check_only = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("unknown arguments {args:?}\n\n{USAGE}")),
    };
    let root = crate::designs::root()?;
    let path = root.join(schema_path());
    let text = hpr_format::schema_json();
    if check_only {
        return if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
            println!("format: the schema is current");
            Ok(())
        } else {
            Err(format!(
                "stale: {}; run `cargo xtask format` and commit the result",
                schema_path()
            ))
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("format: wrote {}", schema_path());
    Ok(())
}

/// The schema's path from the root.
fn schema_path() -> String {
    Path::new("schema/format")
        .join(format!("hpr-design-{}.schema.json", hpr_format::VERSION))
        .to_string_lossy()
        .replace('\\', "/")
}
