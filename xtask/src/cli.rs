//! `cargo xtask cli`: writes what is generated from the `hpr` command's own code, so the README and
//! the guide can't claim a command, a format or an output the tool doesn't have (Loft lesson P10):
//!
//! - `schema/cli/*.schema.json`: the JSON Schema of each `--json` output (`hpr_cli::schemas`);
//! - the command table in `README.md` and `docs/cli.md`, from the registered commands
//!   (`hpr_cli::registry::command_table`);
//! - each example in `docs/cli.md`: the command run in-process, with what it printed. An argument
//!   with a `/` in it is a file of the repository, given from its root, as a reader who runs the
//!   example from a copy of it types it.
//!
//! `--check` (and the test below, which the gate runs) fails when a committed copy differs.

use std::path::{Path, PathBuf};

use hpr_cli::registry::{Links, command_table};

/// The usage line in `cargo xtask help`.
pub const USAGE: &str = "  \
cli [--check]            Write the `hpr` command's generated outputs: its JSON Schemas in
                           schema/cli/, the command tables in README.md and docs/cli.md, and
                           docs/cli.md's example outputs. --check fails if one is stale.";

/// Where the schemas go.
const SCHEMAS: &str = "schema/cli";
/// The guide's page.
const PAGE: &str = "docs/cli.md";
/// The start of a command table.
const TABLE: &str = "<!-- cli: commands, written by `cargo xtask cli` from the registered commands; do not edit -->";
/// The start of an example, before the command line in backticks. After them, `exits N` names
/// the exit status the example is meant to show; without it, the example must succeed.
const EXAMPLE: &str = "<!-- cli: example `";
/// What every generated block's markers start with.
const MARKER: &str = "<!-- cli:";
/// The end of a generated block.
const END: &str = "<!-- cli: end -->";

/// Runs the command.
pub fn run(args: &[String]) -> Result<(), String> {
    let check_only = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("unknown arguments {args:?}\n\n{USAGE}")),
    };
    let root = crate::designs::root()?;
    if check_only {
        return check(&root);
    }
    for (path, text) in outputs(&root)? {
        let path = root.join(path);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    for extra in extra_schemas(&root)? {
        let path = root.join(extra);
        std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    println!("cli: wrote the schemas, the command tables and the examples");
    Ok(())
}

/// Fails unless every output is committed as `cargo xtask cli` writes it.
pub(crate) fn check(root: &Path) -> Result<(), String> {
    let mut stale = Vec::new();
    for (path, text) in outputs(root)? {
        if std::fs::read_to_string(root.join(&path)).ok().as_deref() != Some(&text) {
            stale.push(path.display().to_string());
        }
    }
    for extra in extra_schemas(root)? {
        stale.push(format!("{} (no output type makes it)", extra.display()));
    }
    if stale.is_empty() {
        println!("cli: the schemas, command tables and examples are current");
        Ok(())
    } else {
        Err(format!(
            "stale: {}; run `cargo xtask cli` and commit the result",
            stale.join(", ")
        ))
    }
}

/// Every generated file, by its path from the root, with the text it should hold.
fn outputs(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut outputs: Vec<(PathBuf, String)> = hpr_cli::schemas()
        .into_iter()
        .map(|(name, text)| (Path::new(SCHEMAS).join(name), text))
        .collect();
    let read =
        |path: &str| std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"));
    let readme = read("README.md")?;
    outputs.push((
        PathBuf::from("README.md"),
        splice_table(&readme, &command_table(Links::Readme), "README.md")?,
    ));
    let page = splice_table(&read(PAGE)?, &command_table(Links::Guide), PAGE)?;
    outputs.push((PathBuf::from(PAGE), examples(&page, root)?));
    Ok(outputs)
}

/// Schema files (`*.schema.json`) no output type makes any more. Other files there are left be.
fn extra_schemas(root: &Path) -> Result<Vec<PathBuf>, String> {
    let dir = root.join(SCHEMAS);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let names: Vec<&str> = hpr_cli::schemas().iter().map(|(name, _)| *name).collect();
    let mut extra = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".schema.json") && !names.contains(&name.as_str()) {
            extra.push(Path::new(SCHEMAS).join(name));
        }
    }
    extra.sort();
    Ok(extra)
}

/// `text` with its one command table replaced by `table`.
fn splice_table(text: &str, table: &str, file: &str) -> Result<String, String> {
    let start = text
        .find(TABLE)
        .ok_or_else(|| format!("{file} has no command table (`{TABLE}`)"))?;
    if text[start + TABLE.len()..].contains(TABLE) {
        return Err(format!("{file} has two command tables"));
    }
    let body = start + TABLE.len();
    let end = block_end(text, body, file, "the command table")?;
    Ok(format!("{}\n\n{table}\n{}", &text[..body], &text[end..]))
}

/// Where the block whose body starts at `body` ends: its `END`, with no other marker before it,
/// so that a lost `END` can't make a block swallow the page up to the next block's.
fn block_end(text: &str, body: usize, file: &str, what: &str) -> Result<usize, String> {
    let end = text[body..]
        .find(END)
        .map(|at| body + at)
        .ok_or_else(|| format!("{file}: {what} has no `{END}`"))?;
    if text[body..end].contains(MARKER) {
        return Err(format!(
            "{file}: {what} holds another `{MARKER}` marker before its `{END}`: an end is missing"
        ));
    }
    Ok(end)
}

/// `page` with each example's output replaced by what the command prints now; the repository's
/// root is at `root`.
fn examples(page: &str, root: &Path) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = page;
    while let Some(start) = rest.find(EXAMPLE) {
        let line_end = rest[start..]
            .find('\n')
            .map(|at| start + at)
            .ok_or_else(|| format!("{PAGE}: an example's opening line doesn't end"))?;
        let opening = &rest[start..line_end];
        let (command, after) = opening[EXAMPLE.len()..]
            .split_once('`')
            .filter(|(command, _)| command.starts_with("hpr "))
            .ok_or_else(|| format!("{PAGE}: `{opening}` names no `hpr` command"))?;
        // Split on spaces, run in-process: quotes would be passed on as part of an argument, and a
        // backslash is a path only on Windows.
        if command.contains(['"', '\'', '\\']) {
            return Err(format!(
                "{PAGE}: the example `{command}` has a quote or a backslash, which examples can't \
                 take"
            ));
        }
        // A path is a file of the repository, from its root: the xtask runs from the root, or
        // from `xtask/` under the tests, so it is given to the command whole.
        for path in command.split_whitespace().filter(|arg| arg.contains('/')) {
            if !tracked(root, path) {
                return Err(format!(
                    "{PAGE}: the example `{command}` names `{path}`, which isn't a file of the \
                     repository given from its root (an argument with a `/` is a path)"
                ));
            }
        }
        let expected = match after.split_once("exits ") {
            Some((_, code)) => code
                .split(|c: char| !c.is_ascii_digit())
                .next()
                .and_then(|code| code.parse::<u8>().ok())
                .ok_or_else(|| format!("{PAGE}: `{opening}` says `exits` without a status"))?,
            None => 0,
        };
        let end = block_end(rest, line_end, PAGE, &format!("the example `{command}`"))?;
        let (block, code) = example(command, root);
        // The command was given the root's whole path; the page must not show it.
        if block.contains(&*root.to_string_lossy()) {
            return Err(format!(
                "{PAGE}: the example `{command}` prints the repository's own path, which differs \
                 from machine to machine"
            ));
        }
        if code != expected {
            return Err(format!(
                "{PAGE}: the example `{command}` exits {code}, not {expected}: fix the command, or \
                 say `exits {code}` after it if that is what it is meant to show"
            ));
        }
        result.push_str(&rest[..=line_end]);
        result.push('\n');
        result.push_str(&block);
        result.push('\n');
        rest = &rest[end..];
    }
    result.push_str(rest);
    Ok(result)
}

/// Whether `path`, given from the repository's root, is a file git tracks there: never a
/// gitignored one, such as the private designs under `refs/`, and nothing outside the root.
fn tracked(root: &Path, path: &str) -> bool {
    let relative = Path::new(path)
        .components()
        .all(|part| matches!(part, std::path::Component::Normal(_)));
    relative
        && root.join(path).is_file()
        && std::process::Command::new("git")
            .args(["ls-files", "--error-unmatch", "--", path])
            .current_dir(root)
            .output()
            .is_ok_and(|output| output.status.success())
}

/// A fenced block with the command line and what it printed, and its exit status if not 0; and
/// that status. The command's paths are read from `root`.
fn example(command: &str, root: &Path) -> (String, u8) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let args = command.split_whitespace().map(|arg| {
        if arg.contains('/') {
            root.join(arg).into_os_string()
        } else {
            arg.into()
        }
    });
    let exit = hpr_cli::run(args, &mut out, &mut err);
    let mut block = format!("```text\n$ {command}\n");
    block.push_str(&String::from_utf8_lossy(&out));
    block.push_str(&String::from_utf8_lossy(&err));
    if exit.code() != 0 {
        block.push_str(&format!("$ echo $?\n{}\n", exit.code()));
    }
    block.push_str("```\n");
    (block, exit.code())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cli_outputs_are_current() {
        let root = crate::designs::root().unwrap();
        check(&root).unwrap();
    }

    #[test]
    fn a_table_is_replaced_whole_and_only_once() {
        let text = format!("before\n{TABLE}\nold\n{END}\nafter\n");
        assert_eq!(
            splice_table(&text, "new\n", "f").unwrap(),
            format!("before\n{TABLE}\n\nnew\n\n{END}\nafter\n")
        );
        assert!(
            splice_table("none", "new\n", "f")
                .unwrap_err()
                .contains("no command table")
        );
        let twice = format!("{text}{text}");
        assert!(
            splice_table(&twice, "new\n", "f")
                .unwrap_err()
                .contains("two")
        );
        let open = format!("{TABLE}\nold");
        assert!(
            splice_table(&open, "new\n", "f")
                .unwrap_err()
                .contains("no `")
        );
    }

    #[test]
    fn a_lost_end_is_refused_not_swallowed() {
        let text = format!("{TABLE}\nold\n\n{EXAMPLE}hpr mc x`, exits 3 -->\nold\n{END}\n");
        assert!(
            splice_table(&text, "new\n", "f")
                .unwrap_err()
                .contains("an end is missing")
        );
        let page =
            format!("{EXAMPLE}hpr mc x`, exits 3 -->\n\n{EXAMPLE}hpr mc`, exits 3 -->\n{END}\n");
        assert!(
            examples(&page, &crate::designs::root().unwrap())
                .unwrap_err()
                .contains("an end is missing")
        );
    }

    #[test]
    fn an_example_exits_as_its_marker_says() {
        let unsaid = format!("{EXAMPLE}hpr mc x` -->\n{END}\n");
        assert!(
            examples(&unsaid, &crate::designs::root().unwrap())
                .unwrap_err()
                .contains("exits 3, not 0")
        );
        let wrong = format!("{EXAMPLE}hpr motors list`, exits 3 -->\n{END}\n");
        assert!(
            examples(&wrong, &crate::designs::root().unwrap())
                .unwrap_err()
                .contains("exits 0, not 3")
        );
        let root = crate::designs::root().unwrap();
        for (command, why) in [
            ("hpr motors show \"J 760\"", "a quote or a backslash"),
            ("hpr motors show a\\b.eng", "a quote or a backslash"),
            (
                "hpr motors show a/b.eng",
                "`a/b.eng`, which isn't a file of the repository",
            ),
            ("hpr motors show /etc/hosts", "`/etc/hosts`, which isn't"),
            (
                "hpr motors show xtask/../README.md",
                "which isn't a file of the repository",
            ),
            ("hpr motors show xtask/", "`xtask/`, which isn't"),
            (
                "hpr motors show C:/Windows/win.ini",
                "`C:/Windows/win.ini`, which isn't",
            ),
            (
                "hpr motors show target/nothing.eng",
                "which isn't a file of the repository",
            ),
        ] {
            let page = format!("{EXAMPLE}{command}` -->\n{END}\n");
            assert!(
                examples(&page, &root).unwrap_err().contains(why),
                "{command}"
            );
        }
        // A file of the repository is read from the root wherever the xtask runs, and the page
        // shows the path as given; an output that shows the root's own path is refused.
        let command = "hpr sim validation/fixtures/ork/pod-flights/pods-none.ork --motor H54";
        let page = format!("{EXAMPLE}{command}` -->\n{END}\n");
        let written = examples(&page, &root).unwrap();
        assert!(written.contains(&format!("$ {command}\n")), "{written}");
        assert!(written.contains("168H54-10A"), "{written}");
        let command =
            "hpr motors show crates/hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000735.eng";
        let page = format!("{EXAMPLE}{command}` -->\n{END}\n");
        assert!(
            examples(&page, &root)
                .unwrap_err()
                .contains("prints the repository's own path")
        );
    }

    #[test]
    fn an_example_is_run_and_its_output_replaced() {
        let page = format!("a\n{EXAMPLE}hpr mc x`, exits 3 -->\nstale\n{END}\nb\n");
        let written = examples(&page, &crate::designs::root().unwrap()).unwrap();
        assert!(written.starts_with(&format!(
            "a\n{EXAMPLE}hpr mc x`, exits 3 -->\n\n```text\n$ hpr mc x\n"
        )));
        assert!(written.contains("error: hpr mc is not available yet"));
        assert!(written.ends_with("$ echo $?\n3\n```\n\n<!-- cli: end -->\nb\n"));
        assert!(!written.contains("stale"));
        let bad = format!("{EXAMPLE}ls` -->\n{END}\n");
        assert!(
            examples(&bad, &crate::designs::root().unwrap())
                .unwrap_err()
                .contains("names no `hpr` command")
        );
    }
}
