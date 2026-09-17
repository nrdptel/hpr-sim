//! Consistency checks on the planning docs, run by `cargo test` (M0.3). They guard against process
//! mistakes Loft made (`docs/research/loft-lessons.md`, P5 and P6):
//!
//! - docs stay within budget: `STATUS.md` 150 lines, each note under `docs/research/` 200 lines,
//!   `ROADMAP.md` 1,000 lines with at most 40 per milestone entry;
//! - `STATUS.md` names the first open milestone of `ROADMAP.md` as the current one;
//! - the Loft lessons doc and the roadmap agree, as below.
//!
//! Every lesson row in the lessons doc names the roadmap milestones that fix it (the first one owns
//! the tests) and the tests that will prove hpr-sim doesn't repeat it:
//!
//! ```text
//! | L7 | lesson | Loft evidence | M3.1 | `hpr_io::ork::tests::auto_radius_crosses_stages` |
//! ```
//!
//! The lesson checks:
//!
//! - every row of an `| id |` table parses, and ids run from `L1` and `P1` without gaps, so a row
//!   can't be hidden or dropped unnoticed;
//! - each milestone a lesson names exists, and its `- Loft lessons:` line lists the id; every id on
//!   such a line exists, and its row names that milestone;
//! - each test is a `crate::path::test_name` path into a workspace crate;
//! - once a lesson's first milestone is checked off, each of its tests is a live `#[test]` function
//!   (not ignored, not commented out) in that crate.
//!
//! Process rows (`| P3 | mistake | evidence | guard |`) must name their guard.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The lessons doc, relative to the workspace root.
const LESSONS: &str = "docs/research/loft-lessons.md";

/// Line budgets. STATUS's comes from CLAUDE.md ("Keep docs short"); the rest keep one topic per
/// file and a terse roadmap (Loft's grew to 8,761 lines, one milestone to about 1,700).
const STATUS_MAX_LINES: usize = 150;
const RESEARCH_MAX_LINES: usize = 200;
const ROADMAP_MAX_LINES: usize = 1000;
const MILESTONE_MAX_LINES: usize = 40;

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
    /// Marked `[blocked]` right after its checkbox.
    blocked: bool,
    /// Lines from the checkbox line up to the next entry or heading.
    lines: usize,
    /// The ids on the entry's `- Loft lessons:` line.
    lessons: Vec<String>,
}

/// `M1.2` or `M1.2a` at the start of `tail` (after the `M`), ending at a non-alphanumeric char.
fn milestone_number(tail: &str) -> Option<&str> {
    let bytes = tail.as_bytes();
    let digits = |from: usize| {
        let end = from
            + bytes[from..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
        (end > from).then_some(end)
    };
    let major = digits(0)?;
    if bytes.get(major) != Some(&b'.') {
        return None;
    }
    let mut end = digits(major + 1)?;
    if bytes.get(end).is_some_and(u8::is_ascii_lowercase) {
        end += 1;
    }
    (!bytes.get(end).is_some_and(u8::is_ascii_alphanumeric)).then(|| &tail[..end])
}

/// Parses a milestone checkbox line, `- [ ] **M1.2 Title.**` or `  - [x] [blocked] **M1.2a ...`,
/// into (id, done, blocked). Other lines, including other checkboxes and links, are not entries.
fn milestone_line(line: &str) -> Option<(String, bool, bool)> {
    let rest = line.trim_start().strip_prefix("- [")?;
    let done = match rest.chars().next()? {
        ' ' => false,
        'x' | 'X' => true,
        _ => return None,
    };
    let rest = rest.get(1..)?.strip_prefix("] ")?;
    let (blocked, rest) = match rest.strip_prefix("[blocked] ") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let number = milestone_number(rest.strip_prefix("**M")?)?;
    Some((format!("M{number}"), done, blocked))
}

/// The `L12` and `P3` tokens in `text`.
fn lesson_ids(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| numbered(word, 'L').is_some() || numbered(word, 'P').is_some())
        .map(str::to_owned)
}

/// Parses the milestone entries of the roadmap, including indented increments.
fn milestones(roadmap: &str) -> Vec<Milestone> {
    let mut out: Vec<Milestone> = Vec::new();
    let mut open = false;
    let mut in_lessons = false;
    for line in roadmap.lines() {
        if let Some((id, done, blocked)) = milestone_line(line) {
            out.push(Milestone {
                id,
                done,
                blocked,
                lines: 0,
                lessons: Vec::new(),
            });
            open = true;
            in_lessons = false;
        } else if line.starts_with('#') {
            open = false;
        }
        let Some(entry) = out.last_mut().filter(|_| open) else {
            continue;
        };
        entry.lines += 1;
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("- Loft lessons:") {
            in_lessons = true;
            entry.lessons.extend(lesson_ids(rest));
        } else if in_lessons
            && !trimmed.is_empty()
            && !trimmed.starts_with("- ")
            && !trimmed.starts_with('*')
        {
            entry.lessons.extend(lesson_ids(trimmed));
        } else {
            in_lessons = false;
        }
    }
    out
}

/// The cells of a markdown table row, trimmed, without the outer pipes.
fn cells(line: &str) -> Vec<&str> {
    let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
    inner.split('|').map(str::trim).collect()
}

/// The number of an id such as `L12` (for prefix `L`).
fn numbered(id: &str, prefix: char) -> Option<u32> {
    let digits = id.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The backticked spans of a cell.
fn code_spans(cell: &str) -> Vec<String> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Reports the first gap in ids that must run from 1.
fn gap(numbers: &BTreeSet<u32>, prefix: char, what: &str) -> Option<String> {
    let missing = (1..).find(|n| !numbers.contains(n))?;
    (numbers.last().is_some_and(|&last| missing < last)).then(|| {
        format!("{what} ids must run from {prefix}1 without gaps: {prefix}{missing} is missing")
    })
}

/// Whether `src` defines `name` as a live test: the `fn` line isn't commented out, and the
/// attributes right above it include `#[test]` and no `#[ignore]`.
fn defines_test(src: &str, name: &str) -> bool {
    let lines: Vec<&str> = src.lines().map(str::trim).collect();
    let needle = format!("fn {name}(");
    lines.iter().enumerate().any(|(at, line)| {
        if !line.starts_with(&needle) {
            return false;
        }
        let attributes: Vec<&&str> = lines[..at]
            .iter()
            .rev()
            .take_while(|l| l.starts_with("#["))
            .collect();
        attributes.iter().any(|a| **a == "#[test]")
            && !attributes.iter().any(|a| a.starts_with("#[ignore"))
    })
}

/// Checks the lessons doc against the roadmap. `test_exists(crate_dir, fn_name)` answers whether
/// the named live test exists in `crates/<crate_dir>`. Returns one message per problem.
fn check(
    doc: &str,
    roadmap: &str,
    crate_exists: &dyn Fn(&str) -> bool,
    test_exists: &dyn Fn(&str, &str) -> bool,
) -> Vec<String> {
    let roadmap = milestones(roadmap);
    let mut problems = Vec::new();
    let mut lessons: Vec<Lesson> = Vec::new();
    let mut lesson_numbers = BTreeSet::new();
    let mut process_numbers = BTreeSet::new();

    let mut in_table = false;
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            in_table = false;
            continue;
        }
        let row = cells(trimmed);
        if row.first() == Some(&"id") {
            in_table = true;
            continue;
        }
        let separator = row
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-' || c == ':'));
        if !in_table || separator {
            continue;
        }
        let id = row[0];
        if let Some(number) = numbered(id, 'L') {
            if !lesson_numbers.insert(number) {
                problems.push(format!("{id}: duplicate lesson id"));
            }
            if row.len() != 5 {
                problems.push(format!("{id}: expected 5 cells, found {}", row.len()));
                continue;
            }
            if row[1].is_empty() || row[2].is_empty() {
                problems.push(format!(
                    "{id}: the lesson and its Loft evidence must be filled in"
                ));
            }
            let milestones: Vec<String> = row[3]
                .split(',')
                .map(|m| m.trim().to_owned())
                .filter(|m| !m.is_empty())
                .collect();
            let tests = code_spans(row[4]);
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
        } else if let Some(number) = numbered(id, 'P') {
            process_numbers.insert(number);
            if row.len() != 4 || row.iter().any(|cell| cell.is_empty()) {
                problems.push(format!(
                    "{id}: expected 4 filled cells (id, mistake, evidence, guard)"
                ));
            }
        } else {
            problems.push(format!("table row `{id}` is not an L or P id"));
        }
    }
    if lesson_numbers.is_empty() || process_numbers.is_empty() {
        problems.push("the doc has no lesson rows or no process rows".to_owned());
    }
    problems.extend(gap(&lesson_numbers, 'L', "lesson"));
    problems.extend(gap(&process_numbers, 'P', "process"));

    let mut seen = BTreeSet::new();
    for milestone in &roadmap {
        if !seen.insert(milestone.id.as_str()) {
            problems.push(format!("ROADMAP.md has two entries for {}", milestone.id));
        }
    }

    for lesson in &lessons {
        let id = &lesson.id;
        for test in &lesson.tests {
            let segments: Vec<&str> = test.split("::").collect();
            let snake = |s: &&str| {
                !s.is_empty()
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            };
            if segments.len() < 2 || !segments.iter().all(snake) {
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
            if !milestone.lessons.contains(id) {
                problems.push(format!(
                    "{id}: the {milestone_id} entry in ROADMAP.md doesn't list it"
                ));
            }
        }
        // The first milestone named owns the tests; once it ships, they must exist.
        let owner = lesson
            .milestones
            .first()
            .and_then(|first| roadmap.iter().find(|m| &m.id == first));
        if let Some(owner) = owner.filter(|m| m.done) {
            for test in &lesson.tests {
                let segments: Vec<&str> = test.split("::").collect();
                if let [krate, .., name] = segments.as_slice()
                    && !test_exists(&krate.replace('_', "-"), name)
                {
                    problems.push(format!(
                        "{id}: {} is checked off, but `{test}` isn't a live test",
                        owner.id
                    ));
                }
            }
        }
    }

    for milestone in &roadmap {
        for id in &milestone.lessons {
            if numbered(id, 'P').is_some() {
                if !process_numbers.contains(&numbered(id, 'P').unwrap_or(0)) {
                    problems.push(format!(
                        "ROADMAP.md {} lists {id}, which isn't in the doc",
                        milestone.id
                    ));
                }
                continue;
            }
            match lessons.iter().find(|lesson| &lesson.id == id) {
                None => problems.push(format!(
                    "ROADMAP.md {} lists {id}, which isn't in the doc",
                    milestone.id
                )),
                Some(lesson) if !lesson.milestones.contains(&milestone.id) => {
                    problems.push(format!(
                        "ROADMAP.md {} lists {id}, but its row doesn't name {}",
                        milestone.id, milestone.id
                    ));
                }
                Some(_) => {}
            }
        }
    }
    problems
}

/// Checks that `STATUS.md`'s `- **Current milestone:** M1.2 ...` line names the first milestone
/// that is neither checked off nor blocked. When that milestone is split, its first open increment
/// (`M1.2a`, listed right after it) is accepted too.
fn current_milestone_problem(status: &str, roadmap: &str) -> Option<String> {
    let Some(named) = status.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("- **Current milestone:**")?;
        let word = rest.split_whitespace().next()?;
        Some(word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric()))
    }) else {
        return Some("STATUS.md has no `- **Current milestone:**` line".to_owned());
    };
    let roadmap = milestones(roadmap);
    let mut open = roadmap.iter().filter(|m| !m.done && !m.blocked);
    let Some(first) = open.next() else {
        return Some("ROADMAP.md has no open milestone".to_owned());
    };
    let increment = open.next().filter(|next| {
        next.id
            .strip_prefix(first.id.as_str())
            .is_some_and(|suffix| {
                suffix.len() == 1 && suffix.bytes().all(|b| b.is_ascii_lowercase())
            })
    });
    let accepted = named == first.id || increment.is_some_and(|inc| named == inc.id);
    (!accepted).then(|| {
        format!(
            "STATUS.md names {named} as current, but the first open milestone in ROADMAP.md is {}",
            first.id
        )
    })
}

/// Every file under `dir` with extension `ext`, recursively.
fn files_with_extension(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files_with_extension(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
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
  - [Barrowman 1967](https://example.org/thesis) is the source.
  - [ ] Deferred from **M1.4**: nothing.
  - Loft lessons: L1,
    L2, L4 (tests named in the doc).

  *Done when:* a typical L3 flight works.

- [X] **M1.10 Outputs.** An L1 flight.
  - Loft lessons: L3, L4.

## Phase 1

- Loft lessons: L5.
";

    fn run(doc: &str) -> Vec<String> {
        let crates = ["hpr-aero", "hpr-sim"];
        let mut problems = check(
            doc,
            ROADMAP,
            &|krate| crates.contains(&krate),
            &|krate, name| krate == "hpr-sim" && name == "optimum_delay_is_tested",
        );
        problems.sort();
        problems
    }

    const HEADER: &str = "| id | lesson | evidence | milestone | test |\n|---|---|---|---|---|\n";

    #[test]
    fn roadmap_entries_are_parsed_strictly() {
        let parsed = milestones(ROADMAP);
        let ids: Vec<&str> = parsed.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["M0.3", "M1.5", "M1.10"]);
        let done: Vec<bool> = parsed.iter().map(|m| m.done).collect();
        assert_eq!(done, [true, false, true]);
        assert_eq!(parsed[1].lessons, ["L1", "L2", "L4"]);
        assert_eq!(
            parsed[2].lessons,
            ["L3", "L4"],
            "the heading ends the entry"
        );
        assert_eq!(parsed[1].lines, 8);
        assert_eq!(
            milestone_line("- [ ] [blocked] **M1.3a Motors.**"),
            Some(("M1.3a".to_owned(), false, true))
        );
        for not_an_entry in [
            "- [ ] Deferred from **M1.4**: x",
            "- [link](https://example.org)",
            "- [ ] **M1.2ab Two letters.**",
            "- [ ] **M1 No minor.**",
            "- [ ]**M1.2 No space.**",
        ] {
            assert_eq!(milestone_line(not_an_entry), None, "{not_an_entry}");
        }
    }

    #[test]
    fn a_consistent_doc_passes() {
        let doc = format!(
            "{HEADER}\
             | L1 | drag | loft/aero.ts | M1.5 | `hpr_aero::drag::tests::base_drag` |\n\
             | L2 | lift | loft/aero.ts | M1.5 | `hpr_aero::tests::lift` |\n\
             | L3 | delay | loft/simulate.ts | M1.10 | `hpr_sim::tests::optimum_delay_is_tested` |\n\
             | L4 | owned by the open milestone | loft/x.ts | M1.5, M1.10 | `hpr_aero::tests::not_yet` |\n\
             \n| id | mistake | evidence | guard |\n|---|---|---|---|\n\
             | P1 | a mistake | loft/MAINTAINING.md | CLAUDE.md hard rule 2 |\n"
        );
        assert_eq!(run(&doc), Vec::<String>::new());
    }

    #[test]
    fn broken_rows_are_reported_exactly() {
        let doc = format!(
            "{HEADER}\
             | L1 | drag | loft/x | M1.5 | `hpr_aero::a` |\n\
             | L1 | again | loft/x | M1.5 | `hpr_aero::b` |\n\
             | L2 | no test | loft/x | M1.5 | none yet |\n\
             | L3 | missing | loft/x | M1.10 | `hpr_sim::tests::not_written` |\n\
             | L4 | wrong | loft/x | M2.9 | `hpr_nope::tests::x` |\n\
             | L6 | bad path | loft/x | M1.5 | `hpr_aero::Tests::x` |\n\
             | L7 | too few | M1.5 |\n\
             | **L8** | hidden | loft/x | M1.5 | `hpr_aero::c` |\n\
             | P1 | a mistake | loft/x | a guard |\n\
             | P3 | no guard | loft/x |  |\n"
        );
        let mut expected = vec![
            "L1: duplicate lesson id",
            "L2: names no test",
            "L3: M1.10 is checked off, but `hpr_sim::tests::not_written` isn't a live test",
            "L4: `hpr_nope::tests::x` names no workspace crate `hpr-nope`",
            "L4: milestone M2.9 is not in ROADMAP.md",
            "L6: `hpr_aero::Tests::x` is not a snake_case `crate::path::test` path",
            "L6: the M1.5 entry in ROADMAP.md doesn't list it",
            "L7: expected 5 cells, found 3",
            "P3: expected 4 filled cells (id, mistake, evidence, guard)",
            "ROADMAP.md M1.10 lists L4, but its row doesn't name M1.10",
            "ROADMAP.md M1.5 lists L4, but its row doesn't name M1.5",
            "lesson ids must run from L1 without gaps: L5 is missing",
            "process ids must run from P1 without gaps: P2 is missing",
            "table row `**L8**` is not an L or P id",
        ];
        expected.sort_unstable();
        assert_eq!(run(&doc), expected);
    }

    #[test]
    fn a_doc_without_rows_fails() {
        assert_eq!(
            run("# nothing here\n"),
            [
                "ROADMAP.md M1.10 lists L3, which isn't in the doc",
                "ROADMAP.md M1.10 lists L4, which isn't in the doc",
                "ROADMAP.md M1.5 lists L1, which isn't in the doc",
                "ROADMAP.md M1.5 lists L2, which isn't in the doc",
                "ROADMAP.md M1.5 lists L4, which isn't in the doc",
                "the doc has no lesson rows or no process rows",
            ]
        );
    }

    #[test]
    fn only_live_tests_count() {
        let src = "\
#[test]
fn live() {}

// #[test]
// fn commented() {}

#[test]
#[ignore = \"slow\"]
fn ignored() {}

fn helper() {}

    #[test]
    #[should_panic]
    fn indented_live() {}
";
        assert!(defines_test(src, "live"));
        assert!(defines_test(src, "indented_live"));
        for name in ["commented", "ignored", "helper", "missing"] {
            assert!(!defines_test(src, name), "{name}");
        }
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
        let status = |id: &str| {
            format!(
                "The `**Current milestone:**` line.\n\n- **Current milestone:** {id} Something\n"
            )
        };
        assert_eq!(current_milestone_problem(&status("M1.2"), roadmap), None);
        assert_eq!(current_milestone_problem(&status("M1.2,"), roadmap), None);
        assert_eq!(current_milestone_problem(&status("M1.2b"), roadmap), None);
        assert_eq!(
            current_milestone_problem(&status("M1.3"), roadmap).as_deref(),
            Some(
                "STATUS.md names M1.3 as current, but the first open milestone in ROADMAP.md is M1.2"
            )
        );
        let decimal = "- [ ] **M1.1 A.**\n- [ ] **M1.10 B.**\n";
        assert!(current_milestone_problem(&status("M1.10"), decimal).is_some());
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
        let line_count = |path: &Path| fs::read_to_string(path).unwrap().lines().count();
        let mut over = Vec::new();
        let mut budget = |path: &Path, lines: usize, max: usize| {
            if lines > max {
                over.push(format!("{}: {lines} lines (budget {max})", path.display()));
            }
        };
        let status = root.join("docs/STATUS.md");
        budget(&status, line_count(&status), STATUS_MAX_LINES);
        let roadmap_path = root.join("docs/ROADMAP.md");
        budget(&roadmap_path, line_count(&roadmap_path), ROADMAP_MAX_LINES);
        let roadmap = fs::read_to_string(&roadmap_path).unwrap();
        for milestone in milestones(&roadmap) {
            let entry = roadmap_path.join(&milestone.id);
            budget(&entry, milestone.lines, MILESTONE_MAX_LINES);
        }
        let mut notes = Vec::new();
        files_with_extension(&root.join("docs/research"), "md", &mut notes);
        assert!(!notes.is_empty());
        for note in notes {
            budget(&note, line_count(&note), RESEARCH_MAX_LINES);
        }
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
            files_with_extension(&crates.join(krate), "rs", &mut files);
            files
                .iter()
                .any(|file| fs::read_to_string(file).is_ok_and(|src| defines_test(&src, name)))
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
