//! Consistency checks on the planning docs, run by `cargo test` (M0.3). They guard against process
//! mistakes Loft made (`docs/research/loft-lessons.md`, P5 and P6):
//!
//! - docs stay within their size budgets: `STATUS.md` 150 lines, each `docs/research/` file 200;
//! - `STATUS.md` names the first open milestone of `ROADMAP.md` as the current one;
//! - the Loft lessons doc and the roadmap agree, as below.
//!
//! Every lesson row in the lessons doc names the roadmap milestone that fixes it and the test that
//! will prove hpr-sim doesn't repeat it:
//!
//! ```text
//! | L7 | lesson | Loft evidence | M3.1 | `hpr_io::ork::tests::auto_radius_crosses_stages` |
//! ```
//!
//! The checks:
//!
//! - lesson ids are unique, and each row has the five columns;
//! - each milestone exists in `ROADMAP.md`, and its entry lists the lesson id, so the work is
//!   visible where the milestone is picked up;
//! - each test is a `crate::path::test_name` path into a workspace crate;
//! - once a milestone is checked off, each test it owes exists in that crate's source.
//!
//! Process rows (`| P3 | mistake | evidence | guard |`) must name their guard.

use std::fs;
use std::path::{Path, PathBuf};

/// The lessons doc, relative to the workspace root.
const LESSONS: &str = "docs/research/loft-lessons.md";

/// Line budgets: `STATUS.md` (CLAUDE.md, "Keep docs short"), and each research note.
const STATUS_MAX_LINES: usize = 150;
const RESEARCH_MAX_LINES: usize = 200;

#[derive(Debug)]
struct Lesson {
    id: String,
    milestones: Vec<String>,
    tests: Vec<String>,
}

#[derive(Debug)]
struct Milestone {
    id: String,
    done: bool,
    /// Marked `[blocked]` on its checkbox line (the roadmap's rule for blocked milestones).
    blocked: bool,
    /// The entry's text, from its checkbox line up to the next entry or heading.
    text: String,
}

/// Parses the `- [ ] **M1.2 Title.**` entries of the roadmap, including indented increments.
fn milestones(roadmap: &str) -> Vec<Milestone> {
    let mut out: Vec<Milestone> = Vec::new();
    let mut open = false;
    for line in roadmap.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("- [") {
            let done = rest.starts_with("x]");
            let blocked = rest.contains("[blocked]");
            let id = rest.split_once("**M").map(|(_, tail)| {
                let len = tail
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.'))
                    .unwrap_or(tail.len());
                format!("M{}", tail[..len].trim_end_matches('.'))
            });
            open = id.is_some();
            if let Some(id) = id {
                out.push(Milestone {
                    id,
                    done,
                    blocked,
                    text: String::new(),
                });
            }
        } else if line.starts_with('#') {
            open = false;
        }
        if open && let Some(last) = out.last_mut() {
            last.text.push_str(line);
            last.text.push('\n');
        }
    }
    out
}

/// The cells of a markdown table row, trimmed, without the outer pipes.
fn cells(line: &str) -> Vec<&str> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
}

/// Whether `line` is a table row whose first cell is `<prefix><digits>`.
fn row_id(line: &str, prefix: char) -> Option<&str> {
    let first = *cells(line).first()?;
    let digits = first.strip_prefix(prefix)?;
    (line.trim_start().starts_with('|')
        && !digits.is_empty()
        && digits.bytes().all(|b| b.is_ascii_digit()))
    .then_some(first)
}

/// The backticked spans of a cell.
fn code_spans(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Whether `id` occurs in `text` as a whole token (`L1` does not match `L12`).
fn has_token(text: &str, id: &str) -> bool {
    text.match_indices(id).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + id.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

/// Checks the lessons doc against the roadmap. `test_exists(crate_dir, fn_name)` answers whether
/// the named test function exists in `crates/<crate_dir>`. Returns one message per problem.
fn check(
    doc: &str,
    roadmap: &str,
    crate_exists: &dyn Fn(&str) -> bool,
    test_exists: &dyn Fn(&str, &str) -> bool,
) -> Vec<String> {
    let roadmap = milestones(roadmap);
    let mut problems = Vec::new();
    let mut lessons: Vec<Lesson> = Vec::new();
    let mut processes = 0;

    for line in doc.lines() {
        if let Some(id) = row_id(line, 'L') {
            let row = cells(line);
            if row.len() != 5 {
                problems.push(format!("{id}: expected 5 cells, found {}", row.len()));
                continue;
            }
            if lessons.iter().any(|lesson| lesson.id == id) {
                problems.push(format!("{id}: duplicate lesson id"));
            }
            let milestones: Vec<String> = row[3]
                .split(',')
                .map(|m| m.trim().to_owned())
                .filter(|m| !m.is_empty())
                .collect();
            let tests = code_spans(row[4]);
            if row[1].is_empty() || row[2].is_empty() {
                problems.push(format!(
                    "{id}: the lesson and its Loft evidence must be filled in"
                ));
            }
            if milestones.is_empty() {
                problems.push(format!("{id}: names no milestone"));
            }
            if tests.is_empty() {
                problems.push(format!("{id}: names no test"));
            }
            lessons.push(Lesson {
                id: id.to_owned(),
                milestones,
                tests,
            });
        } else if let Some(id) = row_id(line, 'P') {
            processes += 1;
            let row = cells(line);
            if row.len() != 4 || row.iter().any(|cell| cell.is_empty()) {
                problems.push(format!(
                    "{id}: expected 4 filled cells (id, mistake, evidence, guard)"
                ));
            }
        }
    }
    if lessons.is_empty() || processes == 0 {
        problems.push("the doc has no lesson rows or no process rows".to_owned());
    }

    for lesson in &lessons {
        let id = &lesson.id;
        for test in &lesson.tests {
            let segments: Vec<&str> = test.split("::").collect();
            let snake = |s: &str| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            };
            if segments.len() < 2 || !segments.iter().all(|s| snake(s)) {
                problems.push(format!(
                    "{id}: `{test}` is not a snake_case `crate::path::test` path"
                ));
                continue;
            }
            let crate_dir = segments[0].replace('_', "-");
            if !crate_exists(&crate_dir) {
                problems.push(format!(
                    "{id}: `{test}` names no workspace crate `{crate_dir}`"
                ));
            }
        }
        for milestone_id in &lesson.milestones {
            let Some(milestone) = roadmap.iter().find(|m| &m.id == milestone_id) else {
                problems.push(format!(
                    "{id}: milestone {milestone_id} is not in ROADMAP.md"
                ));
                continue;
            };
            if !has_token(&milestone.text, id) {
                problems.push(format!(
                    "{id}: the {milestone_id} entry in ROADMAP.md doesn't list it"
                ));
            }
        }
        // The tests are owed by the first milestone named; once it ships, they must exist.
        let owner = lesson
            .milestones
            .first()
            .and_then(|first| roadmap.iter().find(|m| &m.id == first));
        if let Some(owner) = owner.filter(|m| m.done) {
            for test in &lesson.tests {
                let segments: Vec<&str> = test.split("::").collect();
                if let (Some(krate), Some(name)) = (segments.first(), segments.last())
                    && segments.len() >= 2
                    && !test_exists(&krate.replace('_', "-"), name)
                {
                    problems.push(format!(
                        "{id}: {} is checked off, but `{test}` doesn't exist",
                        owner.id
                    ));
                }
            }
        }
    }
    problems
}

/// Checks that `STATUS.md`'s `**Current milestone:** M1.2 ...` line names the first milestone that
/// is neither checked off nor blocked. When that milestone is split, its first open increment
/// (`M1.2a`, listed right after it) is accepted too.
fn current_milestone_problem(status: &str, roadmap: &str) -> Option<String> {
    let Some(named) = status.lines().find_map(|line| {
        let rest = line.split_once("**Current milestone:**")?.1.trim_start();
        rest.split_whitespace().next()
    }) else {
        return Some("STATUS.md has no `**Current milestone:**` line".to_owned());
    };
    let roadmap = milestones(roadmap);
    let mut open = roadmap.iter().filter(|m| !m.done && !m.blocked);
    let Some(first) = open.next() else {
        return Some("ROADMAP.md has no open milestone".to_owned());
    };
    let increment = open
        .next()
        .filter(|next| next.id.starts_with(&first.id) && next.id.len() > first.id.len());
    let accepted = named == first.id || increment.is_some_and(|inc| named == inc.id);
    (!accepted).then(|| {
        format!(
            "STATUS.md names {named} as current, but the first open milestone in ROADMAP.md is {}",
            first.id
        )
    })
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    const ROADMAP: &str = "\
## Phase 0

- [x] **M0.3 Lessons.** Done.
- [ ] **M1.5 Aerodynamics I (subsonic).**
  - Loft lessons: L1, L2.

  *Done when:* things.

- [x] **M1.10 Outputs.** Loft lessons: L3.

## Phase 1
Stray text mentioning L4.
";

    fn run(doc: &str) -> Vec<String> {
        let crates = ["hpr-aero", "hpr-sim"];
        check(
            doc,
            ROADMAP,
            &|krate| crates.contains(&krate),
            &|krate, name| krate == "hpr-sim" && name == "optimum_delay_is_tested",
        )
    }

    const PROCESS: &str = "| P1 | a mistake | loft/MAINTAINING.md | CLAUDE.md hard rule 2 |\n";

    #[test]
    fn roadmap_entries_are_parsed_with_their_text() {
        let parsed = milestones(ROADMAP);
        let ids: Vec<&str> = parsed.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["M0.3", "M1.5", "M1.10"]);
        assert!(parsed[0].done && !parsed[1].done && parsed[2].done);
        assert!(has_token(&parsed[1].text, "L2"));
        assert!(
            !has_token(&parsed[2].text, "L4"),
            "a heading ends the entry"
        );
    }

    #[test]
    fn whole_tokens_only() {
        assert!(has_token("lessons: L1, L12.", "L1"));
        assert!(!has_token("lessons: L12, L13.", "L1"));
        assert!(!has_token("see XL1", "L1"));
    }

    #[test]
    fn a_consistent_doc_passes() {
        let doc = format!(
            "| id | lesson | evidence | milestone | test |\n|---|---|---|---|---|\n\
             | L1 | drag | loft/lib/sim/aero.ts | M1.5 | `hpr_aero::drag::tests::base_drag` |\n\
             | L3 | delay | loft/lib/sim/simulate.ts | M1.10 | `hpr_sim::tests::optimum_delay_is_tested` |\n\
             {PROCESS}"
        );
        assert_eq!(run(&doc), Vec::<String>::new());
    }

    #[test]
    fn broken_rows_are_reported() {
        let doc = format!(
            "| L1 | drag | loft/x | M1.5 | `hpr_aero::a` |\n\
             | L1 | again | loft/x | M1.5 | `hpr_aero::b` |\n\
             | L2 | no test | loft/x | M1.5 | none yet |\n\
             | L5 | bad path | loft/x | M2.9 | `hpr_nope::Tests::x` |\n\
             | L6 | too few | M1.5 |\n\
             | L3 | missing | loft/x | M1.10 | `hpr_sim::tests::not_written` |\n\
             | L7 | unlisted | loft/x | M1.10, M1.5 | `hpr_aero::c` |\n\
             {PROCESS}| P2 | no guard | loft/x |  |\n"
        );
        let problems = run(&doc);
        let expected = [
            "L1: duplicate lesson id",
            "L2: names no test",
            "L6: expected 5 cells, found 3",
            "P2: expected 4 filled cells (id, mistake, evidence, guard)",
            "L5: `hpr_nope::Tests::x` is not a snake_case `crate::path::test` path",
            "L5: milestone M2.9 is not in ROADMAP.md",
            "L7: the M1.10 entry in ROADMAP.md doesn't list it",
            "L7: the M1.5 entry in ROADMAP.md doesn't list it",
        ];
        for message in expected {
            assert!(
                problems.iter().any(|p| p == message),
                "missing {message:?} in {problems:#?}"
            );
        }
        assert!(
            problems.iter().any(|p| p
                == "L3: M1.10 is checked off, but `hpr_sim::tests::not_written` doesn't exist"),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_doc_without_rows_fails() {
        assert_eq!(
            run("# nothing here\n"),
            ["the doc has no lesson rows or no process rows"]
        );
    }

    #[test]
    fn the_current_milestone_is_the_first_open_one() {
        let roadmap = "\
- [x] **M0.3 Lessons.**
- [ ] [blocked] **M1.1 Core.** See STATUS.
- [ ] **M1.2 Atmosphere.**
  - [x] **M1.2a Tables.**
  - [ ] **M1.2b Wind.**
- [ ] **M1.3 Motors.**
";
        let status = |id: &str| format!("## Now\n\n- **Current milestone:** {id} Something\n");
        assert_eq!(current_milestone_problem(&status("M1.2"), roadmap), None);
        assert_eq!(current_milestone_problem(&status("M1.2b"), roadmap), None);
        assert_eq!(
            current_milestone_problem(&status("M1.3"), roadmap).as_deref(),
            Some(
                "STATUS.md names M1.3 as current, but the first open milestone in ROADMAP.md is M1.2"
            )
        );
        assert!(current_milestone_problem("no line", roadmap).is_some());
        assert!(current_milestone_problem(&status("M1.1"), "- [x] **M1.1 A.**\n").is_some());
    }

    #[test]
    fn status_names_the_first_open_milestone() {
        let root = root();
        let status = fs::read_to_string(root.join("docs/STATUS.md")).unwrap();
        let roadmap = fs::read_to_string(root.join("docs/ROADMAP.md")).unwrap();
        let problem = current_milestone_problem(&status, &roadmap);
        assert!(problem.is_none(), "{}", problem.unwrap_or_default());
    }

    #[test]
    fn docs_stay_within_budget() {
        let root = root();
        let mut docs = vec![(root.join("docs/STATUS.md"), STATUS_MAX_LINES)];
        for entry in fs::read_dir(root.join("docs/research")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|ext| ext == "md") {
                docs.push((path, RESEARCH_MAX_LINES));
            }
        }
        let over: Vec<String> = docs
            .iter()
            .filter_map(|(path, max)| {
                let lines = fs::read_to_string(path).unwrap().lines().count();
                (lines > *max).then(|| format!("{}: {lines} lines (budget {max})", path.display()))
            })
            .collect();
        assert!(over.is_empty(), "split or trim:\n{}", over.join("\n"));
    }

    #[test]
    fn the_loft_lessons_map_to_the_roadmap() {
        let root = root();
        let doc = fs::read_to_string(root.join(LESSONS)).unwrap();
        let roadmap = fs::read_to_string(root.join("docs/ROADMAP.md")).unwrap();
        let crates = root.join("crates");
        let test_exists = |krate: &str, name: &str| {
            let mut files = Vec::new();
            rust_files(&crates.join(krate), &mut files);
            let needle = format!("fn {name}(");
            files
                .iter()
                .any(|file| fs::read_to_string(file).is_ok_and(|src| src.contains(&needle)))
        };
        let problems = check(
            &doc,
            &roadmap,
            &|krate| crates.join(krate).join("Cargo.toml").is_file(),
            &test_exists,
        );
        assert!(problems.is_empty(), "{LESSONS}:\n{}", problems.join("\n"));
    }
}
