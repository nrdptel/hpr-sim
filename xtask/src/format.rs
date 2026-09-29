//! `cargo xtask format`: writes the hpr design format's JSON Schema (M3.3a) from its types, and
//! the TypeScript and Python types generated from the schema (M3.3c).

use std::path::Path;

/// The command's line in `cargo xtask help`.
pub const USAGE: &str = "  \
format [--check|--typecheck]
                           Write the hpr design format's JSON Schema to
                           schema/format/hpr-design-<version>.schema.json from its types, and
                           the TypeScript and Python types generated from it (M3.3c) to
                           schema/format/typescript/ and schema/format/python/.
                           --check fails if any is stale. --typecheck checks every public
                           design's document against the types with tsc and mypy, fetched
                           by npx and uvx at pinned versions.";

/// Runs the command.
pub fn run(args: &[String]) -> Result<(), String> {
    let check_only = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        [flag] if flag == "--typecheck" => {
            return crate::format_types::typecheck(&crate::designs::root()?);
        }
        _ => return Err(format!("unknown arguments {args:?}\n\n{USAGE}")),
    };
    let root = crate::designs::root()?;
    let mut stale = Vec::new();
    for (path, text) in outputs()? {
        let full = root.join(&path);
        if check_only {
            if std::fs::read_to_string(&full).ok().as_deref() != Some(text.as_str()) {
                stale.push(path);
            }
            continue;
        }
        if let Some(dir) = full.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&full, text).map_err(|e| format!("{}: {e}", full.display()))?;
        println!("format: wrote {path}");
    }
    if !check_only {
        return Ok(());
    }
    if stale.is_empty() {
        println!("format: the schema and the generated types are current");
        Ok(())
    } else {
        Err(format!(
            "stale: {}; run `cargo xtask format` and commit the result",
            stale.join(", ")
        ))
    }
}

/// Every file the command writes, from the root, with its text: the schema, then the types
/// generated from it.
pub fn outputs() -> Result<Vec<(String, String)>, String> {
    let schema = hpr_format::schema();
    Ok(vec![
        (schema_path(), hpr_format::schema_json()),
        (
            crate::format_types::TYPESCRIPT_PATH.to_owned(),
            crate::format_types::typescript(&schema)?,
        ),
        (
            crate::format_types::PYTHON_PATH.to_owned(),
            crate::format_types::python(&schema)?,
        ),
    ])
}

/// The schema's path from the root.
fn schema_path() -> String {
    Path::new("schema/format")
        .join(format!("hpr-design-{}.schema.json", hpr_format::VERSION))
        .to_string_lossy()
        .replace('\\', "/")
}
