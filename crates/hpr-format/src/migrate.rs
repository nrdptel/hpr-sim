//! Migrations: a document of an older version rewritten into the current version's shape.
//!
//! Each step takes a document of one version to the next, working on its JSON before any type
//! reads it, since the older version's types are gone. A document is taken through every step from
//! its own version to [`VERSION`], then read as a document of the current version,
//! so everything the current reader checks, it still checks ([ADR-112][adr-112]).
//!
//! | from | to | what changes |
//! |---|---|---|
//! | 0.1 | 0.2 | `attachments`, the source file's other files, is renamed `source_files`, leaving "attachment" to mean a file carried beside the design in a [`.hprz`](crate::container); a `.ork` source's `airframe_not_as_written`, which 0.1 didn't record, is worked out from the configurations, or set to [`UNKNOWN`] when they don't show it |
//!
//! [adr-112]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-112-m33b-the-hprz-container-and-migrations-2026-09-29

use serde_json::{Map, Value};

use crate::{VERSION, Version};

/// The oldest version this reader migrates from.
pub const OLDEST: Version = Version { major: 0, minor: 1 };

/// One step: a document of `from` rewritten as a document of `to`.
struct Step {
    from: Version,
    to: Version,
    /// Rewrites the document's top-level object, or says why the document isn't one of `from`.
    apply: fn(&mut Map<String, Value>) -> Result<(), String>,
}

/// Every step, oldest first; each `to` is the next `from`, and the last `to` is [`VERSION`].
const STEPS: &[Step] = &[Step {
    from: Version { major: 0, minor: 1 },
    to: Version { major: 0, minor: 2 },
    apply: source_files,
}];

/// Whether this reader migrates a document of `version` to the current one.
pub fn migrates_from(version: Version) -> bool {
    STEPS.iter().any(|step| step.from == version)
}

/// Takes `document`, of `version`, through every step to [`VERSION`], setting its `version` at
/// each.
///
/// Returns why not when the document doesn't hold what its version's step needs; a version no step
/// starts from is left as it is, for the caller to refuse.
pub(crate) fn migrate(document: &mut Map<String, Value>, version: Version) -> Result<(), String> {
    let mut at = version;
    for step in STEPS.iter().skip_while(|step| step.from != version) {
        debug_assert_eq!(step.from, at, "the steps follow each other");
        (step.apply)(document).map_err(|why| format!("as a {} document, {why}", step.from))?;
        document.insert("version".to_owned(), Value::String(step.to.to_string()));
        at = step.to;
    }
    debug_assert!(
        at == version || at == VERSION,
        "the last step ends at VERSION"
    );
    Ok(())
}

/// 0.1 to 0.2: `attachments` becomes `source_files`, and a `.ork` source's
/// `airframe_not_as_written` is worked out from the configurations.
///
/// 0.1 kept why a `.ork`'s airframe was not read exactly as written only in a configuration left
/// out for it. The `.ork` reader asks that after a configuration's motors and their ignition and
/// before its stages' separation, and a rocket whose airframe it can't read flies no
/// configuration ([`hpr_io::ork::NotFlown`]). So a configuration left out for the airframe gives
/// the reason; one that flies, or is left out for its separation, shows the airframe was read as
/// written; and when every configuration was left out for an earlier reason, or there is none, it
/// can't be known, and the reason says so ([`UNKNOWN`]), so that nothing flies the rocket with a
/// motor of another's choosing on a guess.
fn source_files(document: &mut Map<String, Value>) -> Result<(), String> {
    if document.contains_key("source_files") {
        return Err("it has a \"source_files\", which 0.1 doesn't define".to_owned());
    }
    let files = document
        .remove("attachments")
        .ok_or_else(|| "it has no \"attachments\"".to_owned())?;
    document.insert("source_files".to_owned(), files);
    let configurations = document
        .get("motors")
        .and_then(|motors| motors.get("configurations"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let reason = configurations
        .iter()
        .find(|configuration| why(configuration) == Some(AIRFRAME))
        .and_then(|configuration| configuration.get("left_out")?.get("message")?.as_str())
        .map(|message| message.strip_prefix(PREFIX).unwrap_or(message).to_owned());
    let as_written = configurations
        .iter()
        .any(|configuration| matches!(why(configuration), None | Some(SEPARATION)));
    let source = document
        .get_mut("provenance")
        .and_then(|provenance| provenance.get_mut("source"))
        .and_then(Value::as_object_mut);
    if let Some(source) = source {
        if source.contains_key("airframe_not_as_written") {
            return Err(
                "its source has an \"airframe_not_as_written\", which 0.1 doesn't define"
                    .to_owned(),
            );
        }
        if source.get("format").and_then(Value::as_str) == Some("ork") {
            let reason = match (reason, as_written) {
                (Some(reason), _) => Some(reason),
                (None, true) => None,
                (None, false) => Some(UNKNOWN.to_owned()),
            };
            if let Some(reason) = reason {
                source.insert("airframe_not_as_written".to_owned(), Value::String(reason));
            }
        }
    }
    Ok(())
}

/// Why a 0.1 configuration was left out, as 0.1 writes it; `None` if it flies, and `""` if its
/// reason isn't a string, which the reader refuses once migrated.
fn why(configuration: &Value) -> Option<&str> {
    configuration
        .get("left_out")
        .filter(|left_out| !left_out.is_null())
        .map(|left_out| left_out.get("why").and_then(Value::as_str).unwrap_or(""))
}

/// How 0.1 writes [`NotFlown::AirframeNotAsWritten`](hpr_io::ork::NotFlown::AirframeNotAsWritten).
const AIRFRAME: &str = "airframe_not_as_written";

/// How 0.1 writes [`NotFlown::SeparationNotFlown`](hpr_io::ork::NotFlown::SeparationNotFlown),
/// the one reason the `.ork` reader asks after the airframe.
const SEPARATION: &str = "separation_not_flown";

/// What the `.ork` reader puts before the reason in such a configuration's message.
const PREFIX: &str = "the airframe was not read exactly as written: ";

/// The reason a 0.1 document migrates with when none of its configurations shows whether its
/// `.ork` airframe was read exactly as written.
pub const UNKNOWN: &str = "unknown: the document is from version 0.1 of the format, which \
                           didn't record it, and none of its motor configurations shows it; \
                           convert the .ork again to find out";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_run_from_the_oldest_to_the_current_version() {
        assert_eq!(STEPS[0].from, OLDEST);
        for pair in STEPS.windows(2) {
            assert_eq!(pair[0].to, pair[1].from);
        }
        assert_eq!(STEPS.last().map(|step| step.to), Some(VERSION));
        assert!(migrates_from(OLDEST));
        assert!(!migrates_from(VERSION));
    }
}
