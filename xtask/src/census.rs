//! `cargo xtask census [--check | --accept --reason <why>]`: the accuracy census and its gate.
//!
//! The census (`hpr_validate::census`) counts every number the committed reports compare. The
//! census accepted last is committed as `validation/reports/census.json`, with its page
//! `census.md`, the table in `README.md` and `docs/accuracy.md`, and the badge
//! `docs/images/census-badge.svg`, all written from it.
//!
//! - No flag prints how the committed reports differ from the accepted census.
//! - `--check` fails on any such difference, or on an output that isn't what the accepted census
//!   writes. `cargo xtask validate --check` runs it after the report reproduces, so CI holds every
//!   row to the census on each platform (ADR-084).
//! - `--accept --reason <why>` takes the census of the committed reports and writes it and its
//!   outputs. The reason is required whenever a row changed, and is kept in the census, so a
//!   regression is accepted in writing, in the diff that brings it, or not at all.

use std::path::{Path, PathBuf};

use hpr_validate::Report;
use hpr_validate::census::{self, Accepted, Census, Change, Reports};
use hpr_validate::real_flight::RealFlightReport;
use serde_json::Value;

pub const USAGE: &str = "  census [--check | --accept --reason <why>]
                           The accuracy census of the committed reports, held to the one accepted
                           in validation/reports/census.json. No flag prints the differences;
                           --check fails on any; --accept writes the census, its page, the tables
                           in README.md and docs/accuracy.md and the badge, and needs --reason when
                           a row changed.";

/// The accepted census.
const CENSUS_JSON: &str = "validation/reports/census.json";
/// Its page.
const CENSUS_MD: &str = "validation/reports/census.md";
/// Its page on GitHub, which the tables link to: the README and the site read it at one address.
const CENSUS_URL: &str =
    "https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md";
/// The badge.
const BADGE: &str = "docs/images/census-badge.svg";
/// The files that show the census table, between [`BEGIN`] and [`END`].
const TABLES: [&str; 2] = ["README.md", "docs/accuracy.md"];
const BEGIN: &str = "<!-- census: written by `cargo xtask census --accept` from \
                     validation/reports/census.json; do not edit -->";
const END: &str = "<!-- census: end -->";

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
/// writes.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let changes = changes(root)?;
    let accepted = read_accepted(root)?.ok_or_else(|| {
        format!("{CENSUS_JSON} is missing; run `cargo xtask census --accept --reason <why>`")
    })?;
    let stale: Vec<String> = outputs(root, &accepted)?
        .into_iter()
        .filter_map(
            |(path, text)| match std::fs::read_to_string(root.join(&path)) {
                Ok(committed) if committed == text => None,
                Ok(_) => Some(format!(
                    "{} is not what the accepted census writes",
                    path.display()
                )),
                Err(error) => Some(format!("{}: {error}", path.display())),
            },
        )
        .collect();
    if changes.is_empty() && stale.is_empty() {
        println!(
            "census: the committed reports hold to the accepted census ({} rows)",
            accepted.census.rows.len()
        );
        return Ok(());
    }
    print_changes(&changes);
    for line in &stale {
        println!("census: {line}");
    }
    let worse = changes.iter().filter(|change| change.is_worse()).count();
    let mut problems = Vec::new();
    if !changes.is_empty() {
        problems.push(format!(
            "{} row(s) differ from the accepted census, {worse} of them for the worse: a \
             regression beyond its slack fails here. If the change is meant, accept it in writing: \
             `cargo xtask census --accept --reason <why>`, and commit {CENSUS_JSON}",
            changes.len()
        ));
    }
    if !stale.is_empty() {
        problems.push(format!(
            "{} output(s) are stale: run `cargo xtask census --accept`",
            stale.len()
        ));
    }
    Err(problems.join("; "))
}

/// The differences between the committed reports and the accepted census; every row is added
/// when none is accepted yet.
pub(crate) fn changes(root: &Path) -> Result<Vec<Change>, String> {
    let now = take(root)?;
    let accepted = read_accepted(root)?;
    let empty = Census {
        slack_share: census::SLACK_SHARE,
        references: now.references.clone(),
        rows: Vec::new(),
    };
    Ok(census::compare(
        accepted
            .as_ref()
            .map_or(&empty, |accepted| &accepted.census),
        &now,
    ))
}

fn print_changes(changes: &[Change]) {
    if changes.is_empty() {
        println!("census: no row differs from the accepted census");
        return;
    }
    for change in changes {
        println!("census: {}", change.describe());
    }
    println!(
        "census: {} row(s) differ from the accepted census, {} for the worse",
        changes.len(),
        changes.iter().filter(|change| change.is_worse()).count()
    );
}

fn accept_census(root: &Path, reason: &str) -> Result<(), String> {
    let now = take(root)?;
    let previous = read_accepted(root)?;
    let changed = previous
        .as_ref()
        .is_none_or(|previous| !census::compare(&previous.census, &now).is_empty());
    let reason = if changed {
        if reason.trim().is_empty() {
            return Err(
                "rows differ from the accepted census: say why with `--reason <why>`, in words \
                 a reviewer can check"
                    .to_owned(),
            );
        }
        reason.to_owned()
    } else {
        // Nothing moved: the outputs are rewritten, and the last reason stands.
        previous
            .as_ref()
            .map(|previous| previous.reason.clone())
            .unwrap_or_default()
    };
    let accepted = if changed {
        Accepted::new(
            now,
            previous.as_ref().map(|previous| &previous.census),
            &reason,
        )
    } else {
        // Keep the accepted rows as they were, so jitter below the slack doesn't churn the file.
        match previous {
            Some(previous) => previous,
            None => return Err("no census to keep".to_owned()),
        }
    };
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

/// Every file the accepted census writes, with its text.
fn outputs(root: &Path, accepted: &Accepted) -> Result<Vec<(PathBuf, String)>, String> {
    let json = serde_json::to_string_pretty(accepted)
        .map_err(|error| format!("serialising the census: {error}"))?;
    let mut files = vec![
        (PathBuf::from(CENSUS_JSON), format!("{json}\n")),
        (PathBuf::from(CENSUS_MD), accepted.to_markdown()),
        (PathBuf::from(BADGE), census::badge_svg(&accepted.summaries)),
    ];
    let block = format!(
        "{BEGIN}\n\n{}\n{END}",
        census::table(&accepted.summaries, Some(CENSUS_URL))
    );
    for path in TABLES {
        let text = std::fs::read_to_string(root.join(path))
            .map_err(|error| format!("reading {path}: {error}"))?;
        files.push((PathBuf::from(path), splice(&text, &block, path)?));
    }
    Ok(files)
}

/// `text` with what lies from [`BEGIN`] to [`END`] replaced by `block`.
fn splice(text: &str, block: &str, path: &str) -> Result<String, String> {
    let start = text
        .find(BEGIN)
        .ok_or_else(|| format!("{path} has no census block: add the line `{BEGIN}` and `{END}`"))?;
    let end = text[start..]
        .find(END)
        .map(|at| start + at + END.len())
        .ok_or_else(|| format!("{path}: the census block has no `{END}`"))?;
    if text[end..].contains(BEGIN) {
        return Err(format!("{path} has two census blocks"));
    }
    Ok(format!("{}{block}{}", &text[..start], &text[end..]))
}

fn read_json<T: serde::de::DeserializeOwned>(root: &Path, relative: &str) -> Result<T, String> {
    let path = root.join(relative);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("reading {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("reading {}: {error}", path.display()))
}

fn read_accepted(root: &Path) -> Result<Option<Accepted>, String> {
    if root.join(CENSUS_JSON).exists() {
        read_json(root, CENSUS_JSON).map(Some)
    } else {
        Ok(None)
    }
}

/// The census of the committed reports.
fn take(root: &Path) -> Result<Census, String> {
    let harness: Report = read_json(root, "validation/reports/latest.json")?;
    let real: RealFlightReport = read_json(root, "validation/reports/real-flights.json")?;
    let examples: Value = read_json(root, "validation/reports/openrocket-flights.json")?;
    let library: Value = read_json(root, "validation/reports/openrocket-library-flights.json")?;
    Census::take(Reports {
        harness: &harness,
        real_flights: &real,
        openrocket_examples: &examples,
        openrocket_library: &library,
    })
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_is_replaced_whole_and_only_once() {
        let text = format!("before\n{BEGIN}\nold\n{END}\nafter\n");
        let block = format!("{BEGIN}\nnew\n{END}");
        assert_eq!(
            splice(&text, &block, "f").unwrap(),
            format!("before\n{BEGIN}\nnew\n{END}\nafter\n")
        );
        assert!(
            splice("no block", &block, "f")
                .unwrap_err()
                .contains("no census block")
        );
        assert!(
            splice(&format!("{BEGIN}\nold"), &block, "f")
                .unwrap_err()
                .contains("no `")
        );
        let twice = format!("{text}{text}");
        assert!(splice(&twice, &block, "f").unwrap_err().contains("two"));
    }

    #[test]
    fn the_committed_reports_hold_to_the_accepted_census() {
        let root = crate::designs::root().unwrap();
        check(&root).unwrap();
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
