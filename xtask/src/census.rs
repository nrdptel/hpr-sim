//! `cargo xtask census [--check | --accept --reason <why>]`: the accuracy census and its gate.
//!
//! The census (`hpr_validate::census`) counts the numbers the committed reports hold hpr to. The
//! census accepted last is committed as `validation/reports/census.json`, with its page
//! `census.md`, the table in `README.md` and `docs/accuracy.md`, and the two badges in
//! `docs/images/` (`census-badge.svg` and `real-flights-badge.svg`), all written from it.
//!
//! - No flag prints how the committed reports differ from the accepted census.
//! - `--check` fails on any such difference, or on an output that isn't what the accepted census
//!   writes. `cargo xtask validate --check` runs it whether or not the report reproduces, so CI
//!   holds every row to the census on each platform (ADR-084).
//! - `--accept --reason <why>` takes the census of the committed reports and writes it and its
//!   outputs. The reason is required whenever a row changed, and is kept in the census, so a
//!   regression is accepted in writing, in the diff that brings it, or not at all.

use std::path::Path;

use hpr_validate::census::{self, Accepted, Census};
use hpr_validate::committed::{
    census_changes, census_outputs, change_lines, check_census, read_accepted, take_census,
};

pub const USAGE: &str = "  census [--check | --accept --reason <why>]
                           The accuracy census of the committed reports, held to the one accepted
                           in validation/reports/census.json. No flag prints the differences;
                           --check fails on any; --accept writes the census, its page, the tables
                           in README.md and docs/accuracy.md and the badges, and needs --reason when
                           a row changed.";

/// Runs the command.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut check_only = false;
    let mut accept = false;
    let mut reason = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--accept" => accept = true,
            "--reason" => {
                reason = Some(
                    args.next()
                        .ok_or_else(|| format!("`--reason` needs its text\n\n{USAGE}"))?
                        .clone(),
                );
            }
            other => return Err(format!("unknown argument `{other}`\n\n{USAGE}")),
        }
    }
    if check_only && accept {
        return Err(format!(
            "`--check` and `--accept` exclude each other\n\n{USAGE}"
        ));
    }
    if reason.is_some() && !accept {
        return Err(format!("`--reason` goes with `--accept`\n\n{USAGE}"));
    }
    let root = crate::designs::root()?;
    if check_only {
        return check(&root);
    }
    if accept {
        return accept_census(&root, reason.as_deref().unwrap_or(""));
    }
    let changes = changes(&root)?;
    print_changes(&changes);
    Ok(())
}

/// Fails unless the committed reports hold to the accepted census and every output is what it
/// writes (`hpr_validate::committed::check_census`).
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let census = check_census(root).map_err(|error| error.to_string())?;
    for line in census.lines() {
        println!("{line}");
    }
    census.problem().map_or(Ok(()), Err)
}

/// The differences between the committed reports and the accepted census.
pub(crate) fn changes(root: &Path) -> Result<Vec<census::Change>, String> {
    census_changes(root).map_err(|error| error.to_string())
}

fn print_changes(changes: &[census::Change]) {
    for line in change_lines(changes) {
        println!("{line}");
    }
}

fn accept_census(root: &Path, reason: &str) -> Result<(), String> {
    let now = take(root)?;
    let previous = read_accepted(root).map_err(|error| error.to_string())?;
    let accepted = acceptance(previous, now, reason)?;
    for (path, text) in outputs(root, &accepted)? {
        let full = root.join(&path);
        std::fs::write(&full, text)
            .map_err(|error| format!("writing {}: {error}", full.display()))?;
        println!("census: wrote {}", path.display());
    }
    for change in &accepted.changes {
        println!("census: accepted: {change}");
    }
    Ok(())
}

/// The census to write: `now` accepted for `reason` when it differs from `previous`, or
/// `previous` kept as it was when nothing differs, so movement below the slack doesn't churn the
/// file.
///
/// A changed census needs a reason. An unchanged one keeps its own, and a reason given for it is
/// refused rather than dropped without a word.
fn acceptance(previous: Option<Accepted>, now: Census, reason: &str) -> Result<Accepted, String> {
    let changes = previous
        .as_ref()
        .map(|previous| census::compare(&previous.census, &now));
    match (previous, changes) {
        (Some(previous), Some(changes)) if changes.is_empty() => {
            if reason.trim().is_empty() {
                Ok(previous.refreshed())
            } else {
                Err(
                    "nothing differs from the accepted census, so there is nothing to give a \
                     reason for; run `cargo xtask census --accept` alone to rewrite its outputs"
                        .to_owned(),
                )
            }
        }
        (previous, _) if reason.trim().is_empty() => Err(format!(
            "{}: say why with `--reason <why>`, in words a reviewer can check",
            if previous.is_some() {
                "rows differ from the accepted census"
            } else {
                "there is no accepted census yet"
            }
        )),
        (previous, _) => Ok(Accepted::new(
            now,
            previous.as_ref().map(|previous| &previous.census),
            reason,
        )),
    }
}

/// Every file the accepted census writes, with its text.
fn outputs(root: &Path, accepted: &Accepted) -> Result<Vec<(std::path::PathBuf, String)>, String> {
    census_outputs(root, accepted).map_err(|error| error.to_string())
}

/// The census of the committed reports.
fn take(root: &Path) -> Result<Census, String> {
    take_census(root).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use hpr_validate::committed::{CENSUS_JSON, stale};

    use super::*;

    #[test]
    fn the_committed_reports_hold_to_the_accepted_census() {
        let root = crate::designs::root().unwrap();
        check(&root).unwrap();
    }

    #[test]
    fn an_output_edited_by_hand_is_stale() {
        let root = crate::designs::root().unwrap();
        let accepted = read_accepted(&root).unwrap().unwrap();
        let files = outputs(&root, &accepted).unwrap();
        let committed = |path: &Path| std::fs::read_to_string(root.join(path));
        assert!(stale(files.clone(), committed).is_empty());
        // A summary edited in census.json, or a number in a table, is stale: both are written
        // from the rows.
        for (target, from, to) in [
            (CENSUS_JSON, "\"rows\": 30", "\"rows\": 31"),
            ("README.md", "6.04%", "4.04%"),
        ] {
            let edited = |path: &Path| {
                committed(path).map(|text| {
                    if path == Path::new(target) {
                        assert!(text.contains(from), "{target} has no `{from}`");
                        text.replacen(from, to, 1)
                    } else {
                        text
                    }
                })
            };
            assert_eq!(
                stale(files.clone(), edited),
                vec![format!("{target} is not what the accepted census writes")]
            );
        }
    }

    #[test]
    fn a_change_is_accepted_only_with_a_reason() {
        let root = crate::designs::root().unwrap();
        let accepted = read_accepted(&root).unwrap().unwrap();
        let now = take(&root).unwrap();
        // Nothing moved: the census is kept, and a reason for nothing is refused.
        let kept = acceptance(Some(accepted.clone()), now.clone(), "").unwrap();
        assert_eq!(kept, accepted.refreshed());
        assert!(
            acceptance(Some(accepted.clone()), now.clone(), "why")
                .unwrap_err()
                .contains("nothing differs")
        );
        // A row moved: accepted only with a reason, which is kept with the changes.
        let mut moved = now.clone();
        let row = &mut moved.rows[0];
        let was = row.difference;
        row.difference += 1.0 + 2.0 * row.slack;
        // Its percentage follows it: the reference stays where it was.
        row.percent = row.percent.map(|percent| percent * row.difference / was);
        assert!(
            acceptance(Some(accepted.clone()), moved.clone(), " ")
                .unwrap_err()
                .contains("--reason")
        );
        let written = acceptance(Some(accepted), moved.clone(), "a test").unwrap();
        assert_eq!(written.reason, "a test");
        assert_eq!(written.changes.len(), 1, "{:?}", written.changes);
        assert_eq!(written.census, moved);
        // The first census needs one too.
        assert!(
            acceptance(None, now.clone(), "")
                .unwrap_err()
                .contains("no accepted census")
        );
        assert!(acceptance(None, now, "first").is_ok());
    }

    #[test]
    fn flags_are_refused_in_the_wrong_company() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        assert!(run(&args(&["--check", "--accept"])).is_err());
        assert!(run(&args(&["--reason", "why"])).is_err());
        assert!(run(&args(&["--accept", "--reason"])).is_err());
        assert!(run(&args(&["--bogus"])).is_err());
    }
}
