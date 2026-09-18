//! The documentation site (M0.4a; ADR-016 in `docs/DECISIONS.md`). mdBook builds it from `docs/`,
//! and the checks here hold every page to what a reader needs.
//!
//! `docs/SUMMARY.md` lists the pages. Each page has one source, which GitHub and the site both
//! render, so the checks hold it to both:
//!
//! - **Links.** A relative link reaches another page of the site, or a file under `docs/` that
//!   mdBook copies, and its `#fragment` a heading there, or an `<a id="...">` such as a table
//!   row's. The rest of the repository is linked by
//!   its `https://github.com/nrdptel/hpr-sim/blob/main/...` URL, which is checked against the
//!   working tree, fragment included, so a PR that moves or renames a file fails until its links
//!   follow. Other web links are counted, not fetched: a PR's checks don't depend on the network.
//! - **Bare labels.** An internal label (`L12`, a Loft lesson; `ADR-008`; a milestone id such as
//!   `M1.5b`) means nothing to a reader on its own, so outside a link's text it fails (ADR-016):
//!   in prose, tables, headings, inline code, raw HTML and image alt text. Fenced code blocks are
//!   exempt: they quote files and output verbatim. A link inside a heading fails
//!   too, since mdBook makes every heading a link.
//! - **Equations** are written in Unicode, which reads the same on GitHub, on the site and in
//!   rustdoc. `$x$`, `$$` and ```` ```math ```` render as math on GitHub only, so they fail.
//! - **Coverage.** Every page under `docs/physics/` and `docs/format/` is in the summary.
//! - **In short** (M0.4b). Every model page, one per file under `docs/physics/`, opens with a
//!   `## In short` section right under its title: a bulleted list of four items, each opening with
//!   its label in bold, saying what it models, its sources, how well it is validated and what it
//!   leaves out ([`IN_SHORT_ITEMS`]). A reader who stops there knows how far to trust the page.
//!   Each number it quotes must be in the rest of the page, or in a file its item links to.
//! - **Accuracy** (`docs/accuracy.md`) gives every validation result. Each number it quotes must
//!   appear, written the same way, in a file that the same paragraph, list item or table row links
//!   to (a model page less its *In short*, the report, a case file; not a page of the site's top
//!   level); its results tables must match the committed report cell by cell, and hold every
//!   result; and it links every model page. So a number that moves at its source fails here
//!   until the page follows (ADR-017).
//! - **Decisions and the roadmap** (`docs/decisions-and-roadmap.md`) links every decision record
//!   in `docs/DECISIONS.md` and every phase of `docs/ROADMAP.md`, so a new one can't be missed.
//!   Its table of milestones has a row for each milestone and increment of the roadmap, anchored
//!   by its id (`M1.10` as `#m1-10`) and with the roadmap's status, so pages can link a label to
//!   a line of plain words, and a milestone checked off in the roadmap fails here until its row
//!   follows (M0.4e).
//! - **Quotes** (M0.4c). A fenced code block right under a `<!-- quote: <path> -->` comment must
//!   be that file of the repository, line for line. So the code and output a page shows, such as
//!   *Getting started*'s example and what it prints, can't drift from the files CI compiles and
//!   runs. mdBook's `{{#include}}` would render on the site only, not on GitHub.
//!
//! `cargo test` runs these on the real pages. `cargo xtask site` runs them, builds the site into
//! `target/site` with mdBook, and checks the built HTML as well: every relative `href` and `src`
//! must reach a file, and every fragment an `id`, in the output, which catches what mdBook itself
//! rewrites.
//!
//! **The API reference** (M0.4d; ADR-019). `cargo xtask site` also builds the rustdoc of every
//! library in the workspace into `target/site/api`, so the site serves it beside the guide, and
//! the two must link each other: a page of the site links each crate's front page, and each
//! crate's documentation links the guide by its published address, [`SITE_URL`], which the check
//! reads as a page of the built site. Its pages are held to the guide's rule on **bare labels**
//! (M0.4e): a label in the text a reader sees fails, and one in a link's text, in code, or on
//! rustdoc's source pages, which quote the code as written, does not. A doc comment links a label
//! to its row in *Decisions and the roadmap* by the site's address, and a decision record by its
//! GitHub URL, which is checked against the working tree as the guide's are. CI deploys
//! `target/site` to GitHub Pages from `main`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pulldown_cmark::{
    BrokenLink, CodeBlockKind, Event, HeadingLevel, LinkType, Options, Parser, Tag, TagEnd,
};

use crate::workspace::{Package, Workspace};

pub const USAGE: &str = "  site [--no-build] [--locked]
                           Check the documentation site's pages, build the site with
                           mdBook 0.5 into target/site and the workspace's rustdoc into
                           target/site/api, and check the built HTML. --no-build checks the
                           pages only; --locked is passed on to `cargo doc`.";

/// The site's source directory, relative to the workspace root, as `book.toml` sets it.
const SOURCE: &str = "docs";
/// The site's list of pages, relative to [`SOURCE`].
const SUMMARY: &str = "SUMMARY.md";
/// Where `book.toml` has mdBook write the site, relative to the workspace root.
const OUTPUT: &str = "target/site";
/// The repository on GitHub. A page links anything that isn't a page of the site this way.
const GITHUB: &str = "https://github.com/nrdptel/hpr-sim/";
/// How a GitHub URL of this repository names a file or folder on `main`.
const ON_MAIN: [&str; 3] = ["blob/main/", "tree/main/", "raw/main/"];
/// Directories under [`SOURCE`] whose every page must be in the summary.
const COVERED: [&str; 2] = ["physics", "format"];
/// The directory under [`SOURCE`] that holds the model pages, one per model. Each opens with
/// *In short*.
const MODELS: &str = "physics";
/// The heading of the section a model page opens with.
const IN_SHORT: &str = "In short";
/// The bold labels that open the items of a model page's *In short* list, in this order: what it
/// models, its source, how well it is validated, and what it leaves out (M0.4 in
/// `docs/ROADMAP.md`).
const IN_SHORT_ITEMS: [&str; 4] = [
    "What it models:",
    "Sources:",
    "How well it is validated:",
    "What it leaves out:",
];
/// The page that gives every validation result, relative to [`SOURCE`]. Every number it quotes is
/// traced to a file it links ([`untraced_numbers`]).
const ACCURACY: &str = "accuracy.md";
/// The committed validation report, relative to the workspace root. [`ACCURACY`] names every case.
const REPORT: &str = "validation/reports/latest.md";
/// The page that indexes the decisions and the roadmap, relative to [`SOURCE`].
const RECORDS: &str = "decisions-and-roadmap.md";
/// The files [`RECORDS`] indexes, relative to the workspace root, and the prefix of the level-2
/// headings in each that it must link: every decision, and every phase of the roadmap.
/// The roadmap, relative to the workspace root, whose milestones [`RECORDS`] has a row for each.
const ROADMAP: &str = "docs/ROADMAP.md";
const INDEXED: [(&str, &str); 2] = [("docs/DECISIONS.md", "ADR-"), (ROADMAP, "Phase ")];
/// Where GitHub Pages serves the site: the project page of `nrdptel/hpr-sim`. The crates'
/// documentation links the guide by this address, and the README's first lines link it.
const SITE_URL: &str = "https://nrdptel.github.io/hpr-sim/";
/// The path of [`SITE_URL`] on its host, which `book.toml` gives mdBook as `site-url`. A
/// root-absolute link on the built site, such as those on mdBook's 404 page, starts with it.
const SITE_PATH: &str = "/hpr-sim/";
/// Where the site holds the workspace's rustdoc, relative to [`OUTPUT`]: the API reference.
const API: &str = "api";
/// Cargo's target directory for building the API reference, relative to the workspace root. It
/// is the site's own, so clearing its documentation touches no other build.
const RUSTDOC_TARGET: &str = "target/site-rustdoc";
/// What `mdbook --version` prints for the release series `book.toml` is written for.
const MDBOOK_SERIES: &str = "mdbook v0.5.";
/// How to install the mdBook release CI uses.
const MDBOOK_INSTALL: &str = "install mdBook 0.5.4: `cargo install mdbook --version 0.5.4 --locked` or `brew install mdbook`";

pub fn run(args: &[String]) -> Result<(), String> {
    let (build, locked) = parse_args(args)?;
    let root = crate::designs::root()?;
    let report = check_sources(&root)?;
    if !report.problems.is_empty() {
        return Err(failure("the site's pages", &report.problems));
    }
    println!(
        "site pages: {} pages, {} links ({} to the web, not fetched), no problems",
        report.pages, report.links, report.web
    );
    if !build {
        return Ok(());
    }
    mdbook_build(&root)?;
    let output = root.join(OUTPUT);
    let workspace = crate::workspace::load(&root)?;
    let crates = rustdoc_build(&root, &workspace, &output.join(API), locked)?;
    let built = check_html(&root, &output, &crates)?;
    if !built.problems.is_empty() {
        return Err(failure("the built site", &built.problems));
    }
    println!(
        "built site: {} HTML files in {OUTPUT}, every relative link resolves",
        built.pages
    );
    println!(
        "API reference: {} crates in {OUTPUT}/{API} ({} HTML files), every link resolves, each \
         crate is linked from the site and links the guide, and no page has a bare label",
        crates.len(),
        built.api
    );
    Ok(())
}

/// Reads `site`'s arguments: whether to build the site, and whether `cargo doc` runs `--locked`.
fn parse_args(args: &[String]) -> Result<(bool, bool), String> {
    let mut build = true;
    let mut locked = false;
    for arg in args {
        match arg.as_str() {
            "--no-build" => build = false,
            "--locked" => locked = true,
            _ => return Err(format!("unexpected argument `{arg}`\n\n{USAGE}")),
        }
    }
    Ok((build, locked))
}

fn failure(what: &str, problems: &[String]) -> String {
    format!(
        "{} problem(s) in {what}:\n  {}",
        problems.len(),
        problems.join("\n  ")
    )
}

/// Builds the site with mdBook into [`OUTPUT`], from scratch so no page of an earlier build
/// survives to hide a broken link. A warning fails the build like an error does.
fn mdbook_build(root: &Path) -> Result<(), String> {
    let version = Command::new("mdbook")
        .arg("--version")
        .output()
        .map_err(|err| format!("could not run mdbook ({err}); {MDBOOK_INSTALL}"))?;
    let version = String::from_utf8_lossy(&version.stdout).trim().to_owned();
    if !version.starts_with(MDBOOK_SERIES) {
        return Err(format!(
            "book.toml is written for mdBook 0.5, and `mdbook --version` says `{version}`; \
             {MDBOOK_INSTALL}"
        ));
    }
    let output_dir = root.join(OUTPUT);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)
            .map_err(|err| format!("could not clear {OUTPUT}: {err}"))?;
    }
    // mdBook's log level comes from the environment; a quieter one would hide the warnings.
    let output = Command::new("mdbook")
        .args(["build", "--dest-dir", OUTPUT])
        .env_remove("MDBOOK_LOG")
        .env_remove("RUST_LOG")
        .current_dir(root)
        .output()
        .map_err(|err| format!("could not run `mdbook build`: {err}"))?;
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let complaints: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("WARN") || line.contains("ERROR"))
        .collect();
    if !output.status.success() || !complaints.is_empty() {
        return Err(format!("`mdbook build` failed or warned:\n{log}"));
    }
    Ok(())
}

/// Builds the rustdoc of every library in the workspace and copies it to `dest`, under the site,
/// so the site serves the API reference beside the guide. Returns the libraries' crate names,
/// which name rustdoc's directories (`hpr_core`), sorted.
///
/// - **Its own target directory,** [`RUSTDOC_TARGET`], which starts without documentation each
///   time: rustdoc merges its search index and list of crates with whatever an earlier run left,
///   so a crate since renamed, or `xtask`, would be published too. Only the documentation is
///   cleared, so the dependencies stay built and a rebuild takes seconds.
/// - **One crate at a time, each after the crates it depends on.** Rustdoc links another crate's
///   item (`[hpr_design::Layout]`) only if that crate's pages are already there, and otherwise
///   leaves the link as text, without a warning; with `--no-deps`, cargo documents the crates in
///   no set order.
/// - A warning fails, as in CI's `doc` job.
fn rustdoc_build(
    root: &Path,
    workspace: &Workspace,
    dest: &Path,
    locked: bool,
) -> Result<Vec<String>, String> {
    let target = root.join(RUSTDOC_TARGET);
    for doc in doc_dirs(&target) {
        fs::remove_dir_all(&doc)
            .map_err(|err| format!("could not clear {}: {err}", doc.display()))?;
    }
    for package in doc_order(&workspace.packages)? {
        let mut command = Command::new(crate::workspace::cargo());
        command.args(["doc", "--no-deps", "--all-features", "--lib", "--package"]);
        command.arg(&package.name).arg("--target-dir").arg(&target);
        if locked {
            command.arg("--locked");
        }
        let output = command
            .env("RUSTDOCFLAGS", "-D warnings")
            .current_dir(root)
            .output()
            .map_err(|err| format!("could not run `cargo doc`: {err}"))?;
        if !output.status.success() {
            return Err(format!(
                "`cargo doc --package {}` failed ({}):\n{}",
                package.name,
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    // `target/doc`, or `target/<triple>/doc` when a cargo config sets `build.target`.
    let doc = match doc_dirs(&target).as_slice() {
        [doc] => doc.clone(),
        found => {
            return Err(format!(
                "expected rustdoc's output in one directory under {}, found {found:?}",
                target.display()
            ));
        }
    };
    if dest.exists() {
        return Err(format!(
            "{} already exists; the API reference is copied into a fresh site",
            dest.display()
        ));
    }
    copy_dir(&doc, dest)?;
    // Rustdoc writes no page for the directory itself, and its help and settings pages link one.
    fs::write(dest.join("index.html"), API_INDEX)
        .map_err(|err| format!("could not write {}/index.html: {err}", dest.display()))?;
    let mut crates: Vec<String> = workspace
        .packages
        .iter()
        .filter_map(|package| package.lib.clone())
        .collect();
    crates.sort();
    Ok(crates)
}

/// The front page of [`API`]: it sends a reader to the guide's page that lists the crates.
const API_INDEX: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>hpr-sim: the API reference</title>
<meta http-equiv="refresh" content="0; url=../api.html">
</head>
<body>
<p>The API reference starts at <a href="../api.html">its page in the guide</a>, which lists every
crate.</p>
</body>
</html>
"#;

/// The directories under `target` that hold rustdoc's output: `doc`, and `<triple>/doc` for a
/// build for a named target.
fn doc_dirs(target: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(target)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .flat_map(|path| {
            let own = path.file_name().is_some_and(|name| name == "doc");
            if own {
                vec![path]
            } else {
                vec![path.join("doc")]
            }
        })
        .filter(|path| path.is_dir())
        .collect();
    found.sort();
    found
}

/// The workspace's packages that have a library, each after the others it depends on, and
/// otherwise by name, so rustdoc can link from each to what it uses.
fn doc_order(packages: &[Package]) -> Result<Vec<&Package>, String> {
    let mut left: Vec<&Package> = packages.iter().filter(|p| p.lib.is_some()).collect();
    left.sort_by(|a, b| a.name.cmp(&b.name));
    let mut order: Vec<&Package> = Vec::new();
    while !left.is_empty() {
        let waits = |package: &Package| {
            package
                .dependencies
                .iter()
                .any(|dependency| left.iter().any(|other| &other.name == dependency))
        };
        let Some(at) = left.iter().position(|package| !waits(package)) else {
            let names: Vec<&str> = left.iter().map(|package| package.name.as_str()).collect();
            return Err(format!(
                "the libraries {names:?} depend on each other in a cycle"
            ));
        };
        order.push(left.remove(at));
    }
    Ok(order)
}

/// Copies the directory `from` to `to`, recursively, leaving out the `.lock` file cargo keeps in
/// `target/doc`.
fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|err| format!("could not create {}: {err}", to.display()))?;
    let entries =
        fs::read_dir(from).map_err(|err| format!("could not read {}: {err}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("could not read {}: {err}", from.display()))?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else if entry.file_name() != ".lock" {
            fs::copy(&path, &dest)
                .map_err(|err| format!("could not copy {}: {err}", path.display()))?;
        }
    }
    Ok(())
}

/// What checking the pages found.
#[derive(Debug, Default)]
struct Report {
    /// Pages in the summary.
    pages: usize,
    /// Links and images on them.
    links: usize,
    /// Of those, links to the web outside this repository, which are not fetched.
    web: usize,
    /// One line each, `docs/<page>:<line>: <what is wrong>`.
    problems: Vec<String>,
}

/// Checks the pages `docs/SUMMARY.md` lists, and the summary itself. Fails only when the summary
/// can't be read; everything else is a problem in the report.
fn check_sources(root: &Path) -> Result<Report, String> {
    let source = root.join(SOURCE);
    let summary = fs::read_to_string(source.join(SUMMARY))
        .map_err(|err| format!("could not read {SOURCE}/{SUMMARY}: {err}"))?;
    let mut report = Report::default();

    let mut names = vec![SUMMARY.to_owned()];
    for (line, dest) in read_page(&summary).links {
        match summary_entry(&dest) {
            Some(name) if names.contains(&name) => report.problems.push(format!(
                "{SOURCE}/{SUMMARY}:{line}: `{name}` is listed twice"
            )),
            Some(name) => names.push(name),
            None => report.problems.push(format!(
                "{SOURCE}/{SUMMARY}:{line}: `{dest}` is not a page: the summary lists `.md` files \
                 under `{SOURCE}/` by relative path, without a `#fragment`"
            )),
        }
    }
    report.pages = names.len() - 1;
    // The pages that hold the site's own rules can't leave the summary, and with it the checks.
    for required in [ACCURACY, RECORDS] {
        if source.join(required).is_file() && !names.iter().any(|name| name == required) {
            report.problems.push(format!(
                "{SOURCE}/{required}: not in {SOURCE}/{SUMMARY}, so the site doesn't show it"
            ));
        }
    }
    // The pages at the top of the site, which guide a reader and are not sources of numbers.
    let guides: BTreeSet<String> = names
        .iter()
        .filter(|name| parent(name).is_empty() && *name != SUMMARY)
        .cloned()
        .collect();
    for dir in COVERED {
        for file in markdown_files(&source.join(dir)) {
            let name = format!("{dir}/{file}");
            if !names.contains(&name) {
                report.problems.push(format!(
                    "{SOURCE}/{name}: not in {SOURCE}/{SUMMARY}, so the site doesn't show it"
                ));
            }
        }
    }

    let mut pages = BTreeMap::new();
    for name in &names {
        match fs::read_to_string(source.join(name)) {
            Ok(text) => {
                let mut page = read_page(&text);
                page.problems
                    .extend(page_rules(root, name, &text, &page.links, &guides));
                page.problems.sort_by_key(|(line, _)| *line);
                pages.insert(name.clone(), page);
            }
            Err(err) => report.problems.push(format!(
                "{SOURCE}/{name}: listed in {SOURCE}/{SUMMARY}, but can't be read: {err}"
            )),
        }
    }

    // The anchors of the other Markdown files that GitHub URLs point into, read once each.
    let mut others = BTreeMap::new();
    for (name, page) in &pages {
        for (line, problem) in &page.problems {
            report
                .problems
                .push(format!("{SOURCE}/{name}:{line}: {problem}"));
        }
        for (line, dest) in &page.links {
            report.links += 1;
            match check_link(root, &pages, &mut others, name, dest) {
                Ok(Reach::Web) => report.web += 1,
                Ok(Reach::Repository) => {}
                Err(why) => report
                    .problems
                    .push(format!("{SOURCE}/{name}:{line}: link `{dest}`: {why}")),
            }
        }
    }
    Ok(report)
}

/// The page a summary entry names, relative to [`SOURCE`], if it names one.
fn summary_entry(dest: &str) -> Option<String> {
    if is_web(dest) || dest.starts_with('/') || dest.contains('#') || !dest.ends_with(".md") {
        return None;
    }
    join("", dest)
}

/// The `.md` files directly in `dir`, sorted; none if it doesn't exist.
fn markdown_files(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();
    names
}

/// Where a link that passed its check goes.
#[derive(Debug, PartialEq)]
enum Reach {
    /// A page or file of this repository, checked against the working tree.
    Repository,
    /// The web outside this repository, not fetched.
    Web,
}

/// Checks one link on page `from` (relative to [`SOURCE`]). `pages` are the site's pages;
/// `others` caches the anchors of the other Markdown files a GitHub URL points into.
fn check_link(
    root: &Path,
    pages: &BTreeMap<String, Page>,
    others: &mut BTreeMap<String, BTreeSet<String>>,
    from: &str,
    dest: &str,
) -> Result<Reach, String> {
    if dest.is_empty() {
        return Err("empty destination".to_owned());
    }
    if let Some(rest) = dest.strip_prefix(GITHUB) {
        return on_main(root, Some(pages), others, rest).map(|checked| {
            if checked {
                Reach::Repository
            } else {
                Reach::Web
            }
        });
    }
    if is_web(dest) {
        return Ok(Reach::Web);
    }
    if dest.starts_with('/') {
        return Err(
            "an absolute path means different things on GitHub and on the site; use a relative \
             link"
                .to_owned(),
        );
    }
    let (path, fragment) = split_target(dest);
    let name = if path.is_empty() {
        from.to_owned()
    } else {
        join(parent(from), &path).ok_or_else(|| {
            format!(
                "leaves `{SOURCE}/`, which the site can't follow: link it by its GitHub URL, \
                 {GITHUB}blob/main/..."
            )
        })?
    };
    if name == SUMMARY {
        return Err(format!(
            "`{SOURCE}/{SUMMARY}` is the site's table of contents, which mdBook doesn't write as a \
             page"
        ));
    }
    // The API reference is rustdoc's output, which `cargo xtask site` builds into the site under
    // `api/`; the check of the built site follows these links (`check_html`).
    if name.starts_with(&format!("{API}/")) && name.ends_with(".html") {
        return Ok(Reach::Repository);
    }
    if let Some(page) = pages.get(&name) {
        return match fragment {
            Some(fragment) => has_anchor(&page.anchors, fragment, &format!("{SOURCE}/{name}"))
                .map(|()| Reach::Repository),
            None => Ok(Reach::Repository),
        };
    }
    let file = root.join(SOURCE).join(&name);
    if name.ends_with(".md") {
        return Err(if file.is_file() {
            format!(
                "`{SOURCE}/{name}` is not a page of the site ({SUMMARY} doesn't list it): link it \
                 by its GitHub URL, {GITHUB}blob/main/{SOURCE}/{name}"
            )
        } else {
            format!("no page `{SOURCE}/{name}`")
        });
    }
    if file.is_dir() {
        return Err(format!(
            "`{SOURCE}/{name}` is a directory, which has no page on the site"
        ));
    }
    if !file.is_file() {
        return Err(format!("no file `{SOURCE}/{name}`"));
    }
    if fragment.is_some() {
        return Err("only a page has anchors".to_owned());
    }
    Ok(Reach::Repository)
}

/// Checks a link into this repository, `rest` being what follows [`GITHUB`]. A file on `main` must
/// be in the working tree, and a fragment in a Markdown file must name one of its anchors; the
/// result is `true` once checked. Issues, pull requests and the like are on the web: `false`.
/// `pages`, the site's pages if read, and `others` hold the anchors of the files already read.
fn on_main(
    root: &Path,
    pages: Option<&BTreeMap<String, Page>>,
    others: &mut BTreeMap<String, BTreeSet<String>>,
    rest: &str,
) -> Result<bool, String> {
    let Some(target) = ON_MAIN.iter().find_map(|prefix| rest.strip_prefix(prefix)) else {
        return Ok(false);
    };
    let (path, fragment) = split_target(target);
    let path = path.trim_end_matches('/');
    if path.is_empty() || join("", path).as_deref() != Some(path) || !root.join(path).exists() {
        return Err(format!("no `{path}` in the repository"));
    }
    if let Some(fragment) = fragment
        && path.ends_with(".md")
    {
        let page = path
            .strip_prefix(SOURCE)
            .and_then(|name| name.strip_prefix('/'))
            .and_then(|name| pages?.get(name));
        let anchors = match page {
            Some(page) => &page.anchors,
            None => {
                if !others.contains_key(path) {
                    let text = fs::read_to_string(root.join(path))
                        .map_err(|err| format!("can't read `{path}`: {err}"))?;
                    others.insert(path.to_owned(), anchors_of(&text));
                }
                &others[path]
            }
        };
        has_anchor(anchors, fragment, path)?;
    }
    Ok(true)
}

/// Whether `anchors` (of the file `what`) has `fragment`, which may be percent-encoded.
fn has_anchor(anchors: &BTreeSet<String>, fragment: &str, what: &str) -> Result<(), String> {
    if anchors.contains(fragment) || anchors.contains(&percent_decode(fragment)) {
        Ok(())
    } else {
        Err(format!(
            "`{what}` has no heading or `<a id>` with the anchor `#{fragment}`"
        ))
    }
}

/// A link to another site: it names a scheme, or is protocol-relative.
fn is_web(dest: &str) -> bool {
    dest.contains("://") || dest.starts_with("//") || dest.starts_with("mailto:")
}

/// Splits `path?query#fragment` into the path, percent-decoded, and the fragment. The query
/// (`?plain=1`, `?raw=true`) is dropped, and an empty fragment is none.
fn split_target(dest: &str) -> (String, Option<&str>) {
    let (path, fragment) = match dest.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment).filter(|f| !f.is_empty())),
        None => (dest, None),
    };
    let path = path.split_once('?').map_or(path, |(path, _)| path);
    (percent_decode(path), fragment)
}

/// The directory of a `/`-separated relative path: `physics` for `physics/aero.md`.
fn parent(name: &str) -> &str {
    name.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// Resolves the relative path `rel` against the directory `dir`, both `/`-separated, without
/// touching the filesystem. `None` if it climbs out of the root they are relative to.
fn join(dir: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    let mut joined = parts.join("/");
    if rel.ends_with('/') && !joined.is_empty() {
        joined.push('/');
    }
    Some(joined)
}

/// What the checks need from one Markdown file.
#[derive(Debug, Default)]
struct Page {
    /// The anchors of its headings, as GitHub derives them, and the `id`s of its `<a id="...">`
    /// elements, which label a table's rows.
    anchors: BTreeSet<String>,
    /// Its links and images, as (line, destination).
    links: Vec<(usize, String)>,
    /// What is wrong on the page itself, as (line, message): bare labels, math only GitHub
    /// renders, and references to link definitions that don't exist.
    problems: Vec<(usize, String)>,
}

/// The Markdown extensions that both GitHub and mdBook render.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
}

const MATH: &str = "GitHub renders this math and the site doesn't: write the equation in Unicode, \
                    in a ```text block (ADR-016)";

/// Reads a page: its anchors, its links and images, and what is wrong on it by itself.
fn read_page(text: &str) -> Page {
    let line = line_index(text);
    let mut page = Page::default();
    let mut undefined = Vec::new();
    let mut headings = Vec::new();
    let mut ids = Vec::new();
    {
        // `[text][ref]` and `[ref][]` are meant as links. `[ref]` alone may be a citation key such
        // as `[NGA]`, which GitHub and the site both show as text.
        let callback = |link: BrokenLink<'_>| {
            if matches!(
                link.link_type,
                LinkType::Reference
                    | LinkType::ReferenceUnknown
                    | LinkType::Collapsed
                    | LinkType::CollapsedUnknown
            ) {
                undefined.push((link.span.start, link.reference.to_string()));
            }
            None
        };
        let parser = Parser::new_with_broken_link_callback(text, options(), Some(callback));
        // A label belongs in a link's text. An image's alt text is read like prose.
        let mut in_link = 0usize;
        let mut in_code_block = false;
        let mut heading: Option<String> = None;
        // Consecutive text outside links is scanned as one, however the parser splits it.
        let mut prose = String::new();
        let mut prose_at = 0;
        for (event, range) in parser.into_offset_iter() {
            let is_prose = matches!(event, Event::Text(_)) && in_link == 0 && !in_code_block;
            // GitHub's other inline math: a dollar sign right before a code span, `` $`x^2`$ ``.
            let dollar_code = matches!(event, Event::Code(_)) && prose.ends_with('$');
            if !is_prose && !prose.is_empty() {
                scan(&prose, line(prose_at), &mut page.problems);
                prose.clear();
            }
            if dollar_code {
                page.problems.push((line(range.start), MATH.to_owned()));
            }
            if let Event::Html(html) | Event::InlineHtml(html) = &event {
                ids.extend(anchor_ids(html));
            }
            match event {
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    ..
                }) => {
                    if heading.is_some() {
                        page.problems
                            .push((line(range.start), HEADING_LINK.to_owned()));
                    }
                    // An email autolink's destination is the bare address.
                    let dest = if matches!(link_type, LinkType::Email) {
                        format!("mailto:{dest_url}")
                    } else {
                        dest_url.into_string()
                    };
                    page.links.push((line(range.start), dest));
                    in_link += 1;
                }
                Event::End(TagEnd::Link) => in_link = in_link.saturating_sub(1),
                Event::Start(Tag::Image { dest_url, .. }) => {
                    page.links.push((line(range.start), dest_url.into_string()));
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    if let CodeBlockKind::Fenced(info) = kind
                        && info.split_whitespace().next() == Some("math")
                    {
                        page.problems.push((line(range.start), MATH.to_owned()));
                    }
                }
                Event::End(TagEnd::CodeBlock) => in_code_block = false,
                Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
                Event::End(TagEnd::Heading(_)) => headings.extend(heading.take()),
                Event::Text(text) => {
                    if let Some(heading) = heading.as_mut() {
                        heading.push_str(&text);
                    }
                    if is_prose {
                        if prose.is_empty() {
                            prose_at = range.start;
                        }
                        prose.push_str(&text);
                    }
                }
                Event::Code(code) => {
                    if let Some(heading) = heading.as_mut() {
                        heading.push_str(&code);
                    }
                    if in_link == 0 {
                        scan_labels(&code, line(range.start), &mut page.problems);
                    }
                }
                // Raw HTML shows its text too, and a label there is as bare as anywhere.
                Event::Html(html) | Event::InlineHtml(html) if in_link == 0 => {
                    scan_labels(&visible_text(&html), line(range.start), &mut page.problems);
                }
                _ => {}
            }
        }
        if !prose.is_empty() {
            scan(&prose, line(prose_at), &mut page.problems);
        }
    }
    for (at, reference) in undefined {
        page.problems.push((
            line(at),
            format!("`[{reference}]` is used as a link, and no `[{reference}]: ...` defines it"),
        ));
    }
    page.problems.sort_by_key(|(line, _)| *line);
    page.anchors = anchors(headings);
    page.anchors.extend(ids);
    page
}

const HEADING_LINK: &str = "a link inside a heading: mdBook already makes each heading a link, and \
                            a link inside a link is invalid HTML; move it to the text below";

/// The rules only some pages follow, as (line, message): a model page opens with *In short*;
/// [`ACCURACY`] traces its numbers, names every case of the report and links every model page;
/// [`RECORDS`] links every decision and every phase of the roadmap. `links` are the page's, and
/// `guides` the pages of the site at the top of [`SOURCE`], which are not sources of numbers.
fn page_rules(
    root: &Path,
    name: &str,
    text: &str,
    links: &[(usize, String)],
    guides: &BTreeSet<String>,
) -> Vec<(usize, String)> {
    let mut problems = quotes(root, text);
    if parent(name) == MODELS {
        problems.extend(in_short(text));
        // Each number *In short* quotes is in the rest of the page, or in a file its item links.
        if let Some(span) = in_short_span(text) {
            let rest = format!("{}{}", &text[..span.start], &text[span.end..]);
            problems.extend(untraced_numbers(
                root,
                name,
                text,
                Some(span),
                guides,
                &quoted(&rest),
                "the rest of this page, or in a file this item links to",
            ));
        }
    }
    let linked: BTreeSet<(String, Option<String>)> = links
        .iter()
        .filter_map(|(_, dest)| repository_file(name, dest))
        .collect();
    if name == ACCURACY {
        problems.extend(untraced_numbers(
            root,
            name,
            text,
            None,
            guides,
            &[],
            "a file that this paragraph, list item or table row links to",
        ));
        match fs::read_to_string(root.join(REPORT)) {
            Ok(report) => problems.extend(results_tables(text, &report)),
            Err(err) => problems.push((1, format!("can't read {REPORT}: {err}"))),
        }
        for file in markdown_files(&root.join(SOURCE).join(MODELS)) {
            let model = format!("{SOURCE}/{MODELS}/{file}");
            if !linked.iter().any(|(path, _)| *path == model) {
                problems.push((
                    1,
                    format!("no link to the model page `{model}`, whose checks this page gives"),
                ));
            }
        }
    }
    if name == RECORDS {
        let mut roadmap = None;
        for (file, prefix) in INDEXED {
            let indexed = match fs::read_to_string(root.join(file)) {
                Ok(indexed) => indexed,
                Err(err) => {
                    problems.push((1, format!("can't read {file}: {err}")));
                    continue;
                }
            };
            for heading in headings(&indexed, Some(HeadingLevel::H2)) {
                let anchor = slug(&heading);
                if heading.starts_with(prefix)
                    && !linked.contains(&(file.to_owned(), Some(anchor.clone())))
                {
                    problems.push((
                        1,
                        format!(
                            "no link to `{heading}` in {file}: link it as \
                             {GITHUB}blob/main/{file}#{anchor}"
                        ),
                    ));
                }
            }
            if file == ROADMAP {
                roadmap = Some(indexed);
            }
        }
        // A roadmap that can't be read is reported above.
        if let Some(roadmap) = roadmap {
            problems.extend(milestone_rows(text, &roadmap));
        }
    }
    problems
}

/// The milestones and increments of the roadmap, in order, as (id, status): `done` once checked
/// off, `blocked` when marked so, and `not yet done` otherwise. Each is a task-list item whose
/// bold text opens with its id: `- [x] **M1.1 Core math.**`, `- [ ] [blocked] **M0.4d Publish.**`.
fn roadmap_milestones(roadmap: &str) -> Vec<(String, &'static str)> {
    roadmap
        .lines()
        .filter_map(|line| {
            let (mark, rest) = line.trim_start().strip_prefix("- [")?.split_once("] ")?;
            let (blocked, rest) = match rest.strip_prefix("[blocked] ") {
                Some(rest) => (true, rest),
                None => (false, rest),
            };
            let id = rest.strip_prefix("**")?.split_whitespace().next()?;
            let status = match (mark, blocked) {
                ("x" | "X", _) => "done",
                (" ", true) => "blocked",
                (" ", false) => "not yet done",
                _ => return None,
            };
            is_milestone_id(id).then(|| (id.to_owned(), status))
        })
        .collect()
}

/// The roadmap's task-list items that look like a milestone, their bold text opening with `M` and
/// a digit, but that [`roadmap_milestones`] can't read: an id it doesn't recognise (`**M1.11**
/// Foo`, `**M1.11: Foo**`) or a mark other than `x`, `X` or a space. Each would need no row, so
/// each is a problem.
fn unreadable_milestones(roadmap: &str) -> Vec<String> {
    let read: BTreeSet<String> = roadmap_milestones(roadmap)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    roadmap
        .lines()
        .filter_map(|line| {
            let (_, rest) = line.trim_start().strip_prefix("- [")?.split_once("] ")?;
            let rest = rest.strip_prefix("[blocked] ").unwrap_or(rest);
            let bold = rest.strip_prefix("**M")?;
            if !bold.starts_with(|c: char| c.is_ascii_digit()) {
                return None;
            }
            let id = rest.strip_prefix("**")?.split_whitespace().next()?;
            (!read.contains(id)).then(|| line.trim().to_owned())
        })
        .collect()
}

/// Whether `id` is a milestone's: `M`, a number, a dot, a number, and an increment's letter and
/// number if any (`M1.10`, `M2.1b2`).
fn is_milestone_id(id: &str) -> bool {
    let Some((topic, rest)) = id.strip_prefix('M').and_then(|rest| rest.split_once('.')) else {
        return false;
    };
    let number = |text: &str| !text.is_empty() && text.chars().all(|c| c.is_ascii_digit());
    // After the place, an increment is a letter and perhaps a number.
    let increment = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    let after_letter = increment.strip_prefix(|c: char| c.is_ascii_lowercase());
    number(topic)
        && increment.len() < rest.len()
        && match after_letter {
            None => increment.is_empty(),
            Some(after) => after.is_empty() || number(after),
        }
}

/// The anchor of a milestone's row in [`RECORDS`]: its id in lower case, the dot a hyphen.
fn milestone_anchor(id: &str) -> String {
    id.to_lowercase().replace('.', "-")
}

/// Holds [`RECORDS`]'s table of milestones to the roadmap, as (line, message). A row opens with
/// `| <a id="<anchor>"></a>[<id>]...` and ends with the status; rows whose label is not a
/// milestone id, such as the Loft lessons', are someone else's.
fn milestone_rows(text: &str, roadmap: &str) -> Vec<(usize, String)> {
    let expected = roadmap_milestones(roadmap);
    let mut problems: Vec<(usize, String)> = unreadable_milestones(roadmap)
        .into_iter()
        .map(|line| {
            (
                1,
                format!(
                    "{ROADMAP} has `{line}`, which reads like a milestone and isn't one: write it \
                     `- [ ] **M<topic>.<place> Title.**`, with `x` in the box once it is done"
                ),
            )
        })
        .collect();
    let mut rows = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        let Some(row) = line.strip_prefix("| <a id=\"") else {
            continue;
        };
        let Some((anchor, rest)) = row.split_once('"') else {
            continue;
        };
        let Some(id) = rest
            .split_once('[')
            .and_then(|(_, label)| label.split_once(']'))
            .map(|(id, _)| id)
            .filter(|id| is_milestone_id(id))
        else {
            continue;
        };
        let at = index + 1;
        let status = line.trim_end().trim_end_matches('|').rsplit('|').next();
        let status = status.map_or("", str::trim);
        match expected.iter().find(|(milestone, _)| milestone == id) {
            None => problems.push((at, format!("`{id}` is not a milestone of {ROADMAP}"))),
            Some((_, want)) if *want != status => problems.push((
                at,
                format!("`{id}` is {want} in {ROADMAP}, and its row says `{status}`"),
            )),
            Some(_) => {}
        }
        let want = milestone_anchor(id);
        if anchor != want {
            problems.push((
                at,
                format!("`{id}`'s row is anchored `#{anchor}`: anchor it `#{want}`"),
            ));
        }
        if !rows.insert(id.to_owned()) {
            problems.push((at, format!("`{id}` has two rows")));
        }
    }
    for (id, status) in &expected {
        if !rows.contains(id) {
            problems.push((
                1,
                format!(
                    "`{id}`, {status} in {ROADMAP}, has no row in the table of milestones: add \
                     `| <a id=\"{}\"></a>[{id}][phase-N] | what it covers | {status} |`",
                    milestone_anchor(id)
                ),
            ));
        }
    }
    problems
}

/// What opens the comment that marks a quote: `<!-- quote: <path> -->`.
const QUOTE: &str = "quote:";

/// Each fenced code block marked as a quote, by a `<!-- quote: <path> -->` comment on its own
/// right above it, must be that file of the repository, `path` relative to the workspace root, line
/// for line. A comment that mentions `quote:` and isn't such a marker fails too, so a mistyped
/// marker can't leave its block unchecked. Problems as (line of the marker, message).
fn quotes(root: &Path, text: &str) -> Vec<(usize, String)> {
    let line = line_index(text);
    let mut problems = Vec::new();
    let unused = |at: usize, path: &str| {
        (
            at,
            format!(
                "`<!-- {QUOTE} {path} -->` marks the block right after it as a quote of that \
                 file, and no fenced code block follows it"
            ),
        )
    };
    let malformed = |at: usize| {
        (
            at,
            format!(
                "this looks like a quote marker, and isn't one: a marker is a comment \
                 `<!-- {QUOTE} <path> -->` on its own, right above a fenced code block"
            ),
        )
    };
    // The quote a marker is waiting to see, then the one being read, as (line, path, text).
    let mut marked: Option<(usize, String)> = None;
    let mut quoting: Option<(usize, String, String)> = None;
    // An HTML block's text, which the parser hands over a line at a time, and its first line.
    let mut html: Option<(usize, String)> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::HtmlBlock) => html = Some((line(range.start), String::new())),
            Event::Html(part) if html.is_some() => {
                if let Some((_, block)) = html.as_mut() {
                    block.push_str(&part);
                }
            }
            Event::End(TagEnd::HtmlBlock) => {
                // Any HTML between a marker and its block parts them.
                if let Some((at, path)) = marked.take() {
                    problems.push(unused(at, &path));
                }
                if let Some((at, block)) = html.take() {
                    match quote_marker(&block) {
                        Some(path) => marked = Some((at, path)),
                        None if mentions_quote(&block) => problems.push(malformed(at)),
                        None => {}
                    }
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_))) if marked.is_some() => {
                quoting = marked.take().map(|(at, path)| (at, path, String::new()));
            }
            Event::Text(code) if quoting.is_some() => {
                if let Some((_, _, quoted)) = quoting.as_mut() {
                    quoted.push_str(&code);
                }
            }
            Event::End(TagEnd::CodeBlock) if quoting.is_some() => {
                if let Some((at, path, quoted)) = quoting.take()
                    && let Err(why) = quote_matches(root, &path, &quoted)
                {
                    problems.push((at, why));
                }
            }
            other => {
                if let Some((at, path)) = marked.take() {
                    problems.push(unused(at, &path));
                }
                // A marker inside a paragraph is inline HTML, and marks nothing.
                if let Event::InlineHtml(inline) | Event::Html(inline) = other
                    && mentions_quote(&inline)
                {
                    problems.push(malformed(line(range.start)));
                }
            }
        }
    }
    if let Some((at, path)) = marked {
        problems.push(unused(at, &path));
    }
    problems
}

/// The path an HTML block names, if the block is only a quote marker, `<!-- quote: <path> -->`,
/// on one line or several.
fn quote_marker(block: &str) -> Option<String> {
    let path = block
        .trim()
        .strip_prefix("<!--")?
        .strip_suffix("-->")?
        .trim()
        .strip_prefix(QUOTE)?
        .trim();
    Some(path.to_owned())
}

/// Whether HTML holds a comment that mentions `quote:`, in any case.
fn mentions_quote(html: &str) -> bool {
    let html = html.to_ascii_lowercase();
    html.contains("<!--") && html.contains(QUOTE)
}

/// Whether `quoted` is the file `path` of the repository, line for line (a `\r\n` reads as `\n`).
fn quote_matches(root: &Path, path: &str, quoted: &str) -> Result<(), String> {
    let Some(file) = join("", path).filter(|file| !file.is_empty() && !path.starts_with('/'))
    else {
        return Err(format!(
            "`{path}` is not a file of the repository: a quote names one by its path from the \
             workspace root"
        ));
    };
    let text = fs::read_to_string(root.join(&file))
        .map_err(|err| format!("the quote of `{file}` can't be checked: {err}"))?;
    let (text, quoted) = (text.replace("\r\n", "\n"), quoted.replace("\r\n", "\n"));
    if text == quoted {
        return Ok(());
    }
    let (ours, theirs): (Vec<&str>, Vec<&str>) = (quoted.lines().collect(), text.lines().collect());
    let Some(n) = (0..ours.len().max(theirs.len())).find(|&n| ours.get(n) != theirs.get(n)) else {
        return Err(format!(
            "the quote of `{file}` differs from it only in its final newline; a fenced code block \
             ends with one, so the file must too"
        ));
    };
    let shown = |line: Option<&&str>| line.map_or("nothing".to_owned(), |line| format!("`{line}`"));
    Err(format!(
        "the quote of `{file}` differs from it at its line {}: the page has {}, the file {}; copy \
         the file into the block again",
        n + 1,
        shown(ours.get(n)),
        shown(theirs.get(n)),
    ))
}

/// The file of the repository a link on page `from` reaches, relative to the workspace root, and
/// its fragment: a page or file under [`SOURCE`] by a relative link, or anything by its GitHub
/// URL. `None` for the web and for links that reach nothing.
fn repository_file(from: &str, dest: &str) -> Option<(String, Option<String>)> {
    let (path, fragment) = if let Some(rest) = dest.strip_prefix(GITHUB) {
        let target = ON_MAIN
            .iter()
            .find_map(|prefix| rest.strip_prefix(prefix))?;
        let (path, fragment) = split_target(target);
        (join("", &path)?, fragment)
    } else if is_web(dest) || dest.starts_with('/') {
        return None;
    } else {
        let (path, fragment) = split_target(dest);
        let name = if path.is_empty() {
            from.to_owned()
        } else {
            join(parent(from), &path)?
        };
        (format!("{SOURCE}/{name}"), fragment)
    };
    Some((path, fragment.map(percent_decode)))
}

/// A cell of a Markdown table: its text, and the code spans in it.
#[derive(Debug, Default)]
struct Cell {
    text: String,
    code: Vec<String>,
}

/// A Markdown table: its header's cells, and each row's line and cells.
#[derive(Debug, Default)]
struct Table {
    header: Vec<Cell>,
    rows: Vec<(usize, Vec<Cell>)>,
}

/// The tables of a Markdown file, in order.
fn tables(text: &str) -> Vec<Table> {
    let line = line_index(text);
    let mut found: Vec<Table> = Vec::new();
    // The cells of the header or row being read, and where it starts.
    let mut row: Option<(usize, Vec<Cell>)> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Table(_)) => found.push(Table::default()),
            Event::Start(Tag::TableHead | Tag::TableRow) => row = Some((range.start, Vec::new())),
            Event::End(TagEnd::TableHead) => {
                if let (Some(table), Some((_, cells))) = (found.last_mut(), row.take()) {
                    table.header = cells;
                }
            }
            Event::End(TagEnd::TableRow) => {
                if let (Some(table), Some((at, cells))) = (found.last_mut(), row.take()) {
                    table.rows.push((line(at), cells));
                }
            }
            Event::Start(Tag::TableCell) => {
                if let Some((_, cells)) = row.as_mut() {
                    cells.push(Cell::default());
                }
            }
            Event::Text(text) => {
                if let Some(cell) = row.as_mut().and_then(|(_, cells)| cells.last_mut()) {
                    cell.text.push_str(&text);
                }
            }
            Event::Code(code) => {
                if let Some(cell) = row.as_mut().and_then(|(_, cells)| cells.last_mut()) {
                    cell.text.push_str(&code);
                    cell.code.push(code.into_string());
                }
            }
            _ => {}
        }
    }
    found
}

/// The results of the committed report, in order: each row's case, metric and difference, as
/// the report writes them.
fn report_results(report: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    for table in tables(report) {
        let column = |name: &str| {
            table
                .header
                .iter()
                .position(|cell| cell.text.trim() == name)
        };
        let (Some(case), Some(metric), Some(difference)) =
            (column("case"), column("metric"), column("difference"))
        else {
            continue;
        };
        for (_, row) in &table.rows {
            if let (Some(c), Some(m), Some(d)) =
                (row.get(case), row.get(metric), row.get(difference))
            {
                results.push((
                    c.text.trim().to_owned(),
                    m.text.trim().to_owned(),
                    d.text.trim().to_owned(),
                ));
            }
        }
    }
    results
}

/// Checks the results tables on [`ACCURACY`] against the committed `report`, as (line, message).
/// A results table's header opens with `case`, and its other columns each name a metric in code,
/// such as `` `drift_m` ``; each row names its case in code in its first cell. Every such cell
/// must be the report's difference for its case and metric, as the report writes it (a minus may
/// be `−`), and every result of the report must be in one: the page gives every result, and each
/// exactly.
fn results_tables(page: &str, report: &str) -> Vec<(usize, String)> {
    let results = report_results(report);
    let mut shown = BTreeSet::new();
    let mut problems = Vec::new();
    if results.is_empty() {
        problems.push((
            1,
            format!(
                "{REPORT} has no table of results, with `case`, `metric` and `difference` columns"
            ),
        ));
    }
    let mut keys = BTreeSet::new();
    for (case, metric, _) in &results {
        if !keys.insert((case, metric)) {
            problems.push((
                1,
                format!(
                    "{REPORT} gives `{metric}` of `{case}` twice, so the page can't give it once"
                ),
            ));
        }
    }
    for table in tables(page) {
        if table
            .header
            .first()
            .is_none_or(|cell| cell.text.trim() != "case")
        {
            continue;
        }
        let metrics: Vec<Option<&String>> = table
            .header
            .iter()
            .skip(1)
            .map(|cell| cell.code.first())
            .collect();
        for (line, row) in &table.rows {
            let Some(case) = row.first().and_then(|cell| cell.code.first()) else {
                problems.push((
                    *line,
                    "a row of a results table names no case in code, such as `descent-valetudo`"
                        .to_owned(),
                ));
                continue;
            };
            for (cell, metric) in row.iter().skip(1).zip(&metrics) {
                let written = cell.text.trim();
                // An empty cell gives no result; the report's must then be in another row.
                let Some(metric) = metric.filter(|_| !written.is_empty()) else {
                    continue;
                };
                match results.iter().find(|(c, m, _)| c == case && m == metric) {
                    Some((c, m, difference)) if normalized(difference) == normalized(written) => {
                        shown.insert((c.as_str(), m.as_str()));
                    }
                    Some((_, _, difference)) => problems.push((
                        *line,
                        format!(
                            "`{metric}` of `{case}` is {difference} in {REPORT}, and {written} here"
                        ),
                    )),
                    None => {
                        problems.push((*line, format!("{REPORT} has no `{metric}` for `{case}`")))
                    }
                }
            }
        }
    }
    for (case, metric, difference) in &results {
        if !shown.contains(&(case.as_str(), metric.as_str())) {
            problems.push((
                1,
                format!(
                    "`{metric}` of `{case}` ({difference} in {REPORT}) is in no results table: \
                     the page gives every validation result"
                ),
            ));
        }
    }
    problems
}

/// The texts of a Markdown file's headings, in order: all of them, or those of one level.
fn headings(text: &str, level: Option<HeadingLevel>) -> Vec<String> {
    let mut found = Vec::new();
    let mut heading: Option<String> = None;
    for event in Parser::new_ext(text, options()) {
        match event {
            Event::Start(Tag::Heading { level: this, .. }) => {
                heading = level.is_none_or(|level| level == this).then(String::new);
            }
            Event::End(TagEnd::Heading(_)) => found.extend(heading.take()),
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = heading.as_mut() {
                    heading.push_str(&text);
                }
            }
            _ => {}
        }
    }
    found
}

/// The line of each byte offset of `text`, counted from 1.
fn line_index(text: &str) -> impl Fn(usize) -> usize {
    let newlines: Vec<usize> = text.match_indices('\n').map(|(at, _)| at).collect();
    move |offset| 1 + newlines.partition_point(|&at| at < offset)
}

/// The numbers on page `name` (its `text`, or the bytes `only` of it) that can't be traced, as
/// (line, message). Every number a paragraph, list item or table row quotes, outside code, must
/// appear among the
/// `known` numbers or in a file of the repository that the same paragraph, item or row links
/// to: a model page (less its *In short*, which is what is being traced), the report, a case
/// file; but not a page among the `guides`, which would let a page vouch for itself. A number
/// matches one written the same way there ([`Quoted::matches`]). That is how a reader checks it
/// in one click, and how a number that moves at its source fails here until the page follows
/// (ADR-017). A heading can't link a source, so it quotes no numbers. `missing` says where a
/// number should have been.
fn untraced_numbers(
    root: &Path,
    name: &str,
    text: &str,
    only: Option<std::ops::Range<usize>>,
    guides: &BTreeSet<String>,
    known: &[Quoted],
    missing: &str,
) -> Vec<(usize, String)> {
    let line = line_index(text);
    /// A paragraph, list item or table row: where it starts, what it quotes and what it links.
    struct Block {
        at: usize,
        text: String,
        links: Vec<String>,
    }
    let mut problems = Vec::new();
    // Each source's numbers, read once; `None` if it can't be read.
    let mut files: BTreeMap<String, Option<Vec<Quoted>>> = BTreeMap::new();
    let mut blocks: Vec<Block> = Vec::new();
    let mut in_heading = false;
    // A heading's text, and where it starts.
    let mut heading = String::new();
    let mut heading_at = 0;
    let mut in_code_block = false;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Paragraph | Tag::Item | Tag::TableHead | Tag::TableRow) => {
                blocks.push(Block {
                    at: range.start,
                    text: String::new(),
                    links: Vec::new(),
                });
            }
            Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::TableHead | TagEnd::TableRow) => {
                let Some(block) = blocks.pop() else { continue };
                if only.as_ref().is_some_and(|only| !only.contains(&block.at)) {
                    continue;
                }
                let paths: Vec<String> = block
                    .links
                    .iter()
                    .filter_map(|dest| repository_file(name, dest))
                    .map(|(path, _)| path)
                    .filter(|path| {
                        !guides
                            .iter()
                            .any(|guide| *path == format!("{SOURCE}/{guide}"))
                    })
                    .collect();
                for path in &paths {
                    files.entry(path.clone()).or_insert_with(|| {
                        let text = fs::read_to_string(root.join(path)).ok()?;
                        let model = parent(path) == format!("{SOURCE}/{MODELS}");
                        Some(match in_short_span(&text).filter(|_| model) {
                            Some(span) => {
                                quoted(&format!("{}{}", &text[..span.start], &text[span.end..]))
                            }
                            None => quoted(&text),
                        })
                    });
                }
                let sources: Vec<&[Quoted]> = std::iter::once(known)
                    .chain(paths.iter().filter_map(|path| files.get(path)?.as_deref()))
                    .collect();
                for number in quoted(&block.text).into_iter().filter(Quoted::is_checked) {
                    if !sources
                        .iter()
                        .any(|source| source.iter().any(|there| number.matches(there)))
                    {
                        problems.push((
                            line(block.at),
                            format!(
                                "`{number}` is not in {missing}: link where the number comes \
                                 from (a model page, the report, a case file), and quote it as \
                                 it is there"
                            ),
                        ));
                    }
                }
            }
            Event::Start(Tag::Heading { .. }) => {
                in_heading = true;
                heading.clear();
                heading_at = range.start;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let inside = only.as_ref().is_none_or(|only| only.contains(&heading_at));
                for number in quoted(&heading).into_iter().filter(Quoted::is_checked) {
                    if inside {
                        problems.push((
                            line(heading_at),
                            format!(
                                "a heading quotes `{number}`, and can't link where it comes \
                                 from: move the number to the text below"
                            ),
                        ));
                    }
                }
            }
            Event::Text(words) if in_heading => heading.push_str(&words),
            Event::Start(Tag::CodeBlock(_)) => in_code_block = true,
            Event::End(TagEnd::CodeBlock) => in_code_block = false,
            Event::Start(Tag::Link { dest_url, .. }) => {
                if let Some(block) = blocks.last_mut() {
                    block.links.push(dest_url.into_string());
                    block.text.push(' ');
                }
            }
            Event::Text(words) if !in_heading && !in_code_block => {
                if let Some(block) = blocks.last_mut() {
                    block.text.push_str(&words);
                }
            }
            // Nothing else joins the words on either side into one number: code, breaks, the
            // edges of links, images and table cells, raw HTML such as `<br>`, footnote marks.
            Event::End(TagEnd::Link)
            | Event::Start(Tag::TableCell)
            | Event::End(TagEnd::TableCell)
            | Event::Start(Tag::Image { .. })
            | Event::End(TagEnd::Image)
            | Event::Code(_)
            | Event::SoftBreak
            | Event::HardBreak
            | Event::Html(_)
            | Event::InlineHtml(_)
            | Event::FootnoteReference(_) => {
                if let Some(block) = blocks.last_mut() {
                    block.text.push(' ');
                }
            }
            _ => {}
        }
    }
    problems
}

/// A number as a page or a source writes it.
#[derive(Debug, Clone, PartialEq)]
struct Quoted {
    /// Its digits, with any decimal point and exponent: `2.865`, `1e-6`, `10⁻¹²`, `1.2×10⁻⁶`.
    /// Thousands separators are dropped, and a minus is always `-`.
    digits: String,
    /// The `+` or `-` written just before it, if any.
    sign: Option<char>,
    /// Whether a `%` follows it.
    percent: bool,
}

impl Quoted {
    /// Whether a reader would check it: it has a decimal point, an exponent, a percent sign or
    /// at least two digits, so "6-DOF", "Level 2" and "3 fins" are words, and "3%", "0.28",
    /// "1e-6" and "1,708" are numbers.
    fn is_checked(&self) -> bool {
        self.percent || self.digits.len() >= 2 || self.digits.contains(['.', 'e', 'E', '×'])
    }

    /// Whether `there`, in a source, is this number: the same digits, as a whole number, with the
    /// same sign and percent sign when this one writes them. So `2.8` is not `2.865`, `30%` is
    /// not "30 metrics", and `−2.865%` is not `+2.865%`.
    fn matches(&self, there: &Self) -> bool {
        self.digits == there.digits
            && self.sign.is_none_or(|sign| there.sign == Some(sign))
            && (!self.percent || there.percent)
    }
}

impl std::fmt::Display for Quoted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = self.sign.map(String::from).unwrap_or_default();
        let percent = if self.percent { "%" } else { "" };
        write!(f, "{sign}{}{percent}", self.digits)
    }
}

/// What glued to the end of a whole number makes it part of a word, as in `3D`, `1st` or
/// `32bit`, rather than a number with its unit.
const WORD_SUFFIXES: [&str; 7] = ["D", "DOF", "st", "nd", "rd", "th", "bit"];

/// The superscripts that write a power of ten, `10⁻¹²`.
const SUPERSCRIPTS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹⁻⁺";

/// Every number in `text`, as [`Quoted`]. Digits glued to a word before them are part of it
/// (`M1.8`, `C_D0`, `v1.13.0`), and so is a whole number glued to a [`WORD_SUFFIXES`] after it
/// (`3D`, `1st`); one glued to its unit (`12.5m`, `3048m`) is a number. A `#` makes an issue's
/// number (`#39`), which is not quoted.
fn quoted(text: &str) -> Vec<Quoted> {
    let chars: Vec<char> = normalized(text).chars().collect();
    let digit = |at: usize| chars.get(at).is_some_and(char::is_ascii_digit);
    let superscript = |at: usize| chars.get(at).is_some_and(|c| SUPERSCRIPTS.contains(*c));
    // The end of the digits and dotted digits from `at`: `1.13.0` is one number.
    let digits_end = |mut end: usize| {
        while digit(end) || (chars.get(end) == Some(&'.') && digit(end + 1)) {
            end += 1;
        }
        end
    };
    let mut found = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        if !digit(at) {
            at += 1;
            continue;
        }
        let before = at.checked_sub(1).map(|before| chars[before]);
        if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '#') {
            at = digits_end(at);
            continue;
        }
        let mut end = digits_end(at);
        let mut scaled = chars[at..end].contains(&'.');
        // An exponent: `e-6`, `E9`, a power of ten in superscripts, or `×10⁻⁶`.
        if matches!(chars.get(end), Some('e' | 'E')) {
            let from = if matches!(chars.get(end + 1), Some('-' | '+')) {
                end + 2
            } else {
                end + 1
            };
            if digit(from) {
                end = digits_end(from);
                scaled = true;
            }
        }
        if chars.get(end) == Some(&'×') && chars.get(end + 1) == Some(&'1') && digit(end + 2) {
            let from = digits_end(end + 1);
            if superscript(from) {
                end = from;
            }
        }
        while superscript(end) {
            end += 1;
            scaled = true;
        }
        // A whole number glued to a word suffix is part of a word (`3D`, `1st`); glued to a unit
        // (`3048m`, `20kg`) it is a number.
        let suffix: String = chars[end..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect();
        if !scaled && WORD_SUFFIXES.contains(&suffix.as_str()) {
            at = end;
            continue;
        }
        // A `-` is a sign only where it doesn't join two words, as in `2026-09-17` or `WGS-84`.
        let joins = at >= 2 && chars[at - 2].is_alphanumeric();
        found.push(Quoted {
            digits: chars[at..end].iter().collect(),
            sign: before.filter(|c| matches!(c, '+' | '-') && !joins),
            percent: chars.get(end) == Some(&'%'),
        });
        at = end;
    }
    found
}

/// `text` with numbers written one way: without the commas that separate thousands (`1,708`
/// reads as `1708`, while `0.5,12.3` keeps its comma), with `−` as `-`, and with no spaces around
/// a `×`.
fn normalized(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let digit = |at: usize| chars.get(at).is_some_and(char::is_ascii_digit);
    let mut out = String::with_capacity(text.len());
    for (at, &c) in chars.iter().enumerate() {
        let separator = c == ','
            && at > 0
            && digit(at - 1)
            && (1..=3).all(|ahead| digit(at + ahead))
            && !digit(at + 4);
        let around_times =
            c == ' ' && (chars.get(at + 1) == Some(&'×') || at > 0 && chars[at - 1] == '×');
        if separator || around_times {
            continue;
        }
        out.push(if c == '−' { '-' } else { c });
    }
    out
}

/// Where a page's *In short* section is, from its heading to the next heading of level 1 or 2,
/// in bytes; `None` if it has none.
fn in_short_span(text: &str) -> Option<std::ops::Range<usize>> {
    let mut start = None;
    let mut heading: Option<(usize, String)> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1 | HeadingLevel::H2,
                ..
            }) => {
                if let Some(start) = start {
                    return Some(start..range.start);
                }
                heading = Some((range.start, String::new()));
            }
            Event::Text(words) => {
                if let Some((_, heading)) = heading.as_mut() {
                    heading.push_str(&words);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((at, heading)) = heading.take()
                    && heading.trim() == IN_SHORT
                {
                    start = Some(at);
                }
            }
            _ => {}
        }
    }
    start.map(|start| start..text.len())
}

/// How a model page opens, for the messages of [`in_short`].
const IN_SHORT_FORM: &str = "a model page opens with *In short*: right under its `# title`, a \
                             `## In short` section holding only a bulleted list of four items, \
                             which open with `**What it models:**`, `**Sources:**`, `**How well it \
                             is validated:**` and `**What it leaves out:**`, in that order, each \
                             followed by its answer";

/// A Markdown event and the bytes of the page it comes from.
type Spanned<'a> = (Event<'a>, std::ops::Range<usize>);

/// What is wrong with the *In short* a model page opens with, if anything, as (line, message):
/// the first thing that departs from [`IN_SHORT_FORM`].
fn in_short(text: &str) -> Option<(usize, String)> {
    let line = line_index(text);
    let fail = |offset: usize, what: &str| Some((line(offset), format!("{what}: {IN_SHORT_FORM}")));
    let events: Vec<Spanned<'_>> = Parser::new_ext(text, options())
        .into_offset_iter()
        .collect();
    let offset = |at: usize| events.get(at).map_or(text.len(), |(_, range)| range.start);

    // The title.
    if !matches!(
        events.first(),
        Some((
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }),
            _
        ))
    ) {
        return fail(offset(0), "the page doesn't open with its title, `# ...`");
    }
    let Some(title_end) = events
        .iter()
        .position(|(event, _)| matches!(event, Event::End(TagEnd::Heading(_))))
    else {
        return fail(text.len(), "the title never ends");
    };

    // `## In short`, right under it.
    let mut at = title_end + 1;
    if !matches!(
        events.get(at),
        Some((
            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                ..
            }),
            _
        ))
    ) {
        return fail(offset(at), "the title is not followed by `## In short`");
    }
    let heading_at = offset(at);
    let mut heading = String::new();
    at += 1;
    while let Some((event, _)) = events.get(at) {
        at += 1;
        match event {
            Event::End(TagEnd::Heading(_)) => break,
            Event::Text(text) | Event::Code(text) => heading.push_str(text),
            _ => {}
        }
    }
    if heading.trim() != IN_SHORT {
        return fail(
            heading_at,
            &format!(
                "the first section is `## {}`, not `## {IN_SHORT}`",
                heading.trim()
            ),
        );
    }

    // Its bulleted list, split into items.
    if !matches!(events.get(at), Some((Event::Start(Tag::List(None)), _))) {
        return fail(
            offset(at),
            "`## In short` doesn't open with a bulleted list",
        );
    }
    let list_at = offset(at);
    at += 1;
    let mut items: Vec<(usize, &[Spanned<'_>])> = Vec::new();
    let mut depth = 0usize;
    let mut item_from = at;
    let list_end = loop {
        let Some((event, _)) = events.get(at) else {
            return fail(text.len(), "the list never ends");
        };
        match event {
            Event::Start(_) => {
                if depth == 0 {
                    item_from = at;
                }
                depth += 1;
            }
            Event::End(_) if depth == 0 => break at,
            Event::End(_) => {
                depth -= 1;
                if depth == 0 {
                    items.push((offset(item_from), &events[item_from + 1..at]));
                }
            }
            _ => {}
        }
        at += 1;
    };
    // The section holds nothing else: the next thing is a heading, or the end of the page.
    match events.get(list_end + 1) {
        None
        | Some((
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1 | HeadingLevel::H2,
                ..
            }),
            _,
        )) => {}
        Some((_, range)) => {
            return fail(
                range.start,
                "`## In short` holds more than its list; move the rest under a heading of its own",
            );
        }
    }

    // Each item: a bold label, then its answer.
    let mut labels = Vec::new();
    for (item_at, item) in &items {
        let body = match item.first() {
            Some((Event::Start(Tag::Paragraph), _)) => &item[1..],
            _ => item,
        };
        let mut label = None;
        if let Some((Event::Start(Tag::Strong), _)) = body.first()
            && let Some(close) = body
                .iter()
                .position(|(event, _)| matches!(event, Event::End(TagEnd::Strong)))
        {
            let text: String = body[1..close]
                .iter()
                .filter_map(|(event, _)| match event {
                    Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
                    // A label wrapped over two lines reads as one.
                    Event::SoftBreak | Event::HardBreak => Some(" "),
                    _ => None,
                })
                .collect();
            let answered = body[close + 1..].iter().any(|(event, _)| match event {
                Event::Text(text) | Event::Code(text) => text.chars().any(char::is_alphanumeric),
                _ => false,
            });
            if !answered {
                return fail(*item_at, &format!("`**{text}**` has no answer after it"));
            }
            label = Some(text);
        }
        let Some(label) = label else {
            return fail(*item_at, "an item doesn't open with its label in bold");
        };
        labels.push(label);
    }
    if labels != IN_SHORT_ITEMS {
        return fail(
            list_at,
            &format!(
                "its items open with {}",
                labels
                    .iter()
                    .map(|label| format!("`**{label}**`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    None
}

/// The anchors GitHub gives the headings of a Markdown file, which is read for nothing else, and
/// the `id`s of its `<a id="...">` elements.
fn anchors_of(text: &str) -> BTreeSet<String> {
    let mut found = anchors(headings(text, None));
    // Only raw HTML names an anchor: not code, which may show a tag as an example.
    for event in Parser::new_ext(text, options()) {
        if let Event::Html(html) | Event::InlineHtml(html) = event {
            found.extend(anchor_ids(&html));
        }
    }
    found
}

/// The anchors of headings with these texts, in order: each one's [`slug`], a repeat numbered
/// `-1`, `-2` and so on, as GitHub numbers them.
fn anchors(headings: Vec<String>) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = BTreeSet::new();
    for heading in headings {
        let base = slug(&heading);
        let count = seen.entry(base.clone()).or_insert(0);
        out.insert(if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        });
        *count += 1;
    }
    out
}

/// The text a reader sees in raw HTML: without its tags and comments.
fn visible_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.split_once("-->").map_or("", |(_, after)| after);
        } else if let Some(tag) = rest.strip_prefix('<') {
            rest = tag.split_once('>').map_or("", |(_, after)| after);
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    out
}

/// The elements of a built page whose text isn't prose to scan for bare labels: a link's text
/// already leads to what the label means, code and preformatted blocks quote code as written, and
/// no reader sees a script or a style.
const NOT_PROSE: [&str; 5] = ["a", "code", "pre", "script", "style"];

/// The prose a reader sees on a built page: its [`visible_text`], without the text of its
/// comments and of its [`NOT_PROSE`] elements. An element ends at the first closing tag of its
/// name, as none of them nests in itself.
fn prose_text(html: &str) -> String {
    let mut kept = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        kept.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(comment) = rest.strip_prefix("<!--") {
            rest = comment.split_once("-->").map_or("", |(_, after)| after);
        } else if let Some(name) = NOT_PROSE.into_iter().find(|name| opens(rest, name)) {
            let close = format!("</{name}>");
            rest = rest.split_once(&close).map_or("", |(_, after)| after);
            // What was on either side of the element stays two words.
            kept.push(' ');
        } else {
            // Any other tag goes through, for `visible_text` to drop, and leaves a space: rustdoc
            // closes a heading, a summary or a table cell right before the next text, which is a
            // word of its own.
            let end = rest.find('>').map_or(rest.len(), |end| end + 1);
            kept.push_str(&rest[..end]);
            kept.push(' ');
            rest = &rest[end..];
        }
    }
    kept.push_str(rest);
    visible_text(&kept)
}

/// Whether `html` starts with a start tag of the element `name`: `<a>` or `<a href="...">` for
/// `a`, but not `<abbr>`.
fn opens(html: &str, name: &str) -> bool {
    html.strip_prefix('<')
        .and_then(|tag| tag.strip_prefix(name))
        .is_some_and(|after| after.starts_with(['>', ' ', '\t', '\n', '\r']))
}

/// The `id` of each `<a id="...">` in raw HTML: an anchor for a place that isn't a heading, such
/// as a row of a table. mdBook keeps the `id`; GitHub prefixes it with `user-content-` and still
/// scrolls to `#id`.
fn anchor_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    // A tag inside a comment is not in the page.
    let mut uncommented = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((before, after)) = rest.split_once("<!--") {
        uncommented.push_str(before);
        rest = after.split_once("-->").map_or("", |(_, after)| after);
    }
    uncommented.push_str(rest);
    let mut rest = uncommented.as_str();
    while let Some(start) = rest.find("<a ") {
        let tag = &rest[start + 3..];
        let tag = tag.split_once('>').map_or(tag, |(inside, _)| inside);
        if let Some(id) = tag
            .split_whitespace()
            .find_map(|attribute| attribute.strip_prefix("id=\""))
            .and_then(|value| value.split_once('"'))
            .map(|(id, _)| id)
            .filter(|id| !id.is_empty())
        {
            ids.push(id.to_owned());
        }
        rest = &rest[start + 3 + tag.len()..];
    }
    ids
}

/// Scans prose (text outside links and code) for bare labels and for math only GitHub renders.
fn scan(text: &str, line: usize, problems: &mut Vec<(usize, String)>) {
    scan_labels(text, line, problems);
    if text.contains("$$") || has_inline_math(text) {
        problems.push((line, MATH.to_owned()));
    }
}

/// Whether `text` holds GitHub's inline math, `$x$`: a `$` before a non-space, closed by a later
/// `$` after a non-space and not before a digit. Two prices, "$5 and $10", are not math.
fn has_inline_math(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    chars.iter().enumerate().any(|(open, &c)| {
        c == '$'
            && chars
                .get(open + 1)
                .is_some_and(|next| !next.is_whitespace() && *next != '$')
            && (open + 2..chars.len()).any(|close| {
                chars[close] == '$'
                    && !chars[close - 1].is_whitespace()
                    && !chars.get(close + 1).is_some_and(char::is_ascii_digit)
            })
    })
}

fn scan_labels(text: &str, line: usize, problems: &mut Vec<(usize, String)>) {
    for label in labels(text) {
        problems.push((
            line,
            format!(
                "bare label `{label}`: make it a link, with a few words saying what it is \
                 (ADR-016). A motor designation is written in full, such as `L1150R`, and a \
                 certification level in words, such as \"Level 2\""
            ),
        ));
    }
}

/// The internal labels in `text`, each a whole word: a Loft lesson (`L12`), a decision
/// (`ADR-008`), or a milestone (`M1.5b`, `M2.1b1`).
fn labels(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        // Labels start with an ASCII letter, so `at` is on a char boundary whenever one matches.
        let starts_word = at == 0 || !bytes[at - 1].is_ascii_alphanumeric();
        if starts_word
            && matches!(bytes[at], b'L' | b'A' | b'M')
            && let Some(len) = label_at(&bytes[at..])
        {
            found.push(&text[at..at + len]);
            at += len;
        } else {
            at += 1;
        }
    }
    found
}

/// The length of the label at the start of `bytes`, if one is there and ends at a word boundary.
fn label_at(bytes: &[u8]) -> Option<usize> {
    let digits = |from: usize| {
        let count = bytes.get(from..).map_or(0, |rest| {
            rest.iter().take_while(|b| b.is_ascii_digit()).count()
        });
        (count > 0).then_some(from + count)
    };
    let end = if bytes.starts_with(b"ADR-") {
        digits(4)?
    } else if bytes.first() == Some(&b'L') {
        digits(1)?
    } else if bytes.first() == Some(&b'M') {
        // A milestone: M, digits, a dot, digits, then an optional letter and digits.
        let major = digits(1)?;
        if bytes.get(major) != Some(&b'.') {
            return None;
        }
        let mut end = digits(major + 1)?;
        if bytes.get(end).is_some_and(u8::is_ascii_lowercase) {
            end = digits(end + 1).unwrap_or(end + 1);
        }
        end
    } else {
        return None;
    };
    (!bytes.get(end).is_some_and(u8::is_ascii_alphanumeric)).then_some(end)
}

/// The anchor GitHub gives a heading with this text: lower case; letters, digits, `-` and `_`
/// kept; each space a `-`; everything else dropped. A repeated heading's anchor is numbered
/// (`-1`, `-2`) by [`anchors`]. mdBook derives the same ids for this site's headings, which the
/// check of the built pages confirms.
fn slug(heading: &str) -> String {
    heading
        .trim()
        .chars()
        .flat_map(char::to_lowercase)
        .filter_map(|c| match c {
            ' ' => Some('-'),
            '-' | '_' => Some(c),
            _ if c.is_alphanumeric() => Some(c),
            _ => None,
        })
        .collect()
}

/// What checking the built site found.
#[derive(Debug, Default)]
struct Built {
    /// The site's own HTML files.
    pages: usize,
    /// The HTML files of the API reference under [`API`].
    api: usize,
    /// One line each, `<file>: <what is wrong>`.
    problems: Vec<String>,
}

/// mdBook's page for an address the site doesn't have. It links its own heading, which its
/// `<base href>` sends to the front page instead, as on every mdBook site; nothing else it links
/// is exempt.
const MDBOOK_404: &str = "404.html";

/// Checks the built site in `dir`: every `href` and `src` that stays on the site reaches a file,
/// and its `#fragment` an `id` in that file.
///
/// - A root-absolute link, and a link by the site's address, [`SITE_URL`], are read under the
///   path Pages serves the site at, [`SITE_PATH`]; a `<base href>` moves where relative links and
///   fragments start, as in a browser.
/// - The API reference under [`API`] is held to the same, with three exceptions for what rustdoc
///   writes and a browser forgives ([`rustdoc_may_miss`]). Each library of `crates` (rustdoc's
///   names, `hpr_core`) has a front page there, which a page of the site links and which links
///   the guide by [`SITE_URL`]. No page leaves a link to another crate as text, which rustdoc
///   does without a warning when that crate's pages aren't built yet. And no page but a source
///   page (under `api/src/`) has a bare label in its [`prose_text`], as [`scan_labels`] finds
///   them on the guide's pages. Its links into this repository on GitHub must reach a file of
///   `root`, the workspace, and a fragment an anchor there, as the guide's must ([`on_main`]).
fn check_html(root: &Path, dir: &Path, crates: &[String]) -> Result<Built, String> {
    let mut names = Vec::new();
    html_files(dir, "", &mut names)?;
    let api_dir = format!("{API}/");
    let (api, site): (Vec<String>, Vec<String>) = names
        .into_iter()
        .partition(|name| name.starts_with(&api_dir));
    if site.is_empty() {
        return Err(format!("no HTML files in {}", dir.display()));
    }
    let mut built = Built {
        pages: site.len(),
        api: api.len(),
        problems: Vec::new(),
    };
    let mut ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    // The anchors of the repository's Markdown files that the API reference links on GitHub.
    let mut repository = BTreeMap::new();
    // The files the site's own pages link, so a crate's front page can't go unlinked.
    let mut linked = BTreeSet::new();
    for name in site.iter().chain(&api) {
        let in_api = name.starts_with(&api_dir);
        let text = read_html(dir, name)?;
        let base = match base_of(&text) {
            Ok(base) => base,
            Err(why) => {
                built.problems.push(format!("{name}: {why}"));
                continue;
            }
        };
        // Rustdoc's source pages quote the code, doc comments included, as written.
        if in_api && !name.starts_with(&format!("{API}/src/")) {
            for path in unplaced_links(&text, crates) {
                built.problems.push(format!(
                    "{name}: rustdoc left the link to `{path}` unresolved, because that crate's \
                     pages weren't built before this one's"
                ));
            }
            let mut bare = Vec::new();
            scan_labels(&prose_text(&text), 0, &mut bare);
            built.problems.extend(
                bare.into_iter()
                    .map(|(_, why)| format!("{name}: a doc comment has a {why}")),
            );
        }
        let links = attributes(&text, "href")
            .map(|raw| ("href", raw))
            .chain(attributes(&text, "src").map(|raw| ("src", raw)));
        for (attribute, raw) in links {
            let value = unescape(raw);
            if in_api && rustdoc_may_miss(attribute, &value) {
                continue;
            }
            let link = format!("`{attribute}=\"{value}\"`");
            // The guide's links into the repository are checked on its pages; the API
            // reference's are checked here, against the same working tree.
            if in_api && let Some(rest) = value.strip_prefix(GITHUB) {
                if let Err(why) = on_main(root, None, &mut repository, rest) {
                    built.problems.push(format!("{name}: {link}: {why}"));
                }
                continue;
            }
            let (target, fragment) = match html_target(dir, name, base.as_deref(), &value) {
                Ok(Some(found)) => found,
                Ok(None) => continue,
                Err(why) => {
                    built.problems.push(format!("{name}: {link} {why}"));
                    continue;
                }
            };
            if !in_api {
                linked.insert(target.clone());
            }
            if !dir.join(&target).is_file() {
                built
                    .problems
                    .push(format!("{name}: {link}: no `{target}` in the site"));
                continue;
            }
            let Some(fragment) = fragment.filter(|_| target.ends_with(".html")) else {
                continue;
            };
            // Rustdoc's source pages take a range of lines, `#12-40`, which their script reads.
            if target.starts_with(&format!("{API}/src/")) {
                continue;
            }
            let own_heading = || {
                attributes(&text, "id")
                    .any(|id| percent_decode(&unescape(id)) == percent_decode(&fragment))
            };
            if name == MDBOOK_404 && value.starts_with('#') && own_heading() {
                continue;
            }
            if !ids.contains_key(&target) {
                let found = attributes(&read_html(dir, &target)?, "id")
                    .map(|id| percent_decode(&unescape(id)))
                    .collect();
                ids.insert(target.clone(), found);
            }
            if !ids[&target].contains(&percent_decode(&fragment)) {
                built.problems.push(format!(
                    "{name}: {link}: `{target}` has no `id=\"{fragment}\"`"
                ));
            }
        }
    }
    for krate in crates {
        let front = format!("{API}/{krate}/index.html");
        if !dir.join(&front).is_file() {
            built.problems.push(format!(
                "{front}: missing, so the crate `{krate}` has no API reference"
            ));
            continue;
        }
        if !linked.contains(&front) {
            built.problems.push(format!(
                "{front}: no page of the site links it, so a reader of the guide can't find it"
            ));
        }
        let text = read_html(dir, &front)?;
        if !attributes(&text, "href").any(|href| by_address(&unescape(href)).is_some()) {
            built.problems.push(format!(
                "{front}: doesn't link the guide: the crate's documentation (`//!` in its \
                 `lib.rs`) must link a page of {SITE_URL}"
            ));
        }
    }
    Ok(built)
}

/// Whether a link on a page of the API reference is one that rustdoc writes without its target,
/// and a reader never misses:
///
/// - the implementors of a trait, or the implementations of a type alias, in other crates, which
///   its pages load from `trait.impl/` and `type.impl/` when there are any;
/// - a link in documentation that rustdoc copies from a dependency, such as glam's for
///   `hpr_core::DVec3`, which it passes on as written (`crate::DAffine2`) when it can't place it.
///   A path into one of the workspace's crates is reported by [`unplaced_links`] instead, and a
///   link in hpr's own documentation that doesn't resolve at all fails `cargo doc`.
fn rustdoc_may_miss(attribute: &str, value: &str) -> bool {
    let optional_script = attribute == "src"
        && (value.contains("trait.impl/") || value.contains("type.impl/"))
        && value.ends_with(".js");
    let copied_path = is_path(value) && value.contains("::");
    optional_script || copied_path
}

/// Whether `text` is a name or a path of names (`hpr_design::Layout`).
fn is_path(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
}

/// Whether the path `text` starts with one of the workspace's `crates`.
fn is_ours(text: &str, crates: &[String]) -> bool {
    text.split("::")
        .next()
        .is_some_and(|first| crates.iter().any(|krate| krate == first))
}

/// The paths of the links to items that rustdoc couldn't place on a page, which it does without a
/// warning when the item's crate has no pages yet. It leaves them as text, `[<code>path</code>]`
/// or `[hpr_design::Layout]`, or as a link to the path itself, `href="hpr_design::Layout"`. Other
/// code in brackets (`[self.xy()]`) is not a link.
fn unplaced_links<'a>(html: &'a str, crates: &[String]) -> Vec<&'a str> {
    let in_code = html.match_indices("[<code>").filter_map(|(at, open)| {
        let rest = &html[at + open.len()..];
        Some(&rest[..rest.find("</code>]")?]).filter(|path| is_path(path))
    });
    let in_text = html.match_indices('[').filter_map(|(at, _)| {
        let rest = &html[at + 1..];
        Some(&rest[..rest.find(']')?]).filter(|path| is_path(path) && is_ours(path, crates))
    });
    let as_href = attributes(html, "href")
        .filter(|href| is_path(href) && href.contains("::") && is_ours(href, crates));
    in_code.chain(in_text).chain(as_href).collect()
}

/// Reads the built file `name` (relative to `dir`).
fn read_html(dir: &Path, name: &str) -> Result<String, String> {
    fs::read_to_string(dir.join(name))
        .map_err(|err| format!("could not read {}: {err}", dir.join(name).display()))
}

/// What follows the site's root in `value`, a link by the site's address, [`SITE_URL`], with or
/// without its last `/`; `None` for any other link.
fn by_address(value: &str) -> Option<&str> {
    under(value, SITE_URL)
}

/// What follows `root` in `value`: the rest of a link under it, such as `physics/aero.html`, or
/// what follows the root itself without its last `/` (nothing, `?...` or `#...`).
fn under<'a>(value: &'a str, root: &str) -> Option<&'a str> {
    if let Some(rest) = value.strip_prefix(root) {
        return Some(rest);
    }
    let rest = value.strip_prefix(root.trim_end_matches('/'))?;
    (rest.is_empty() || rest.starts_with(['?', '#'])).then_some(rest)
}

/// The path, relative to the site's root, of a page's `<base href>`, if it has one. mdBook's 404
/// page has one, [`SITE_PATH`], so that it works at any address.
fn base_of(html: &str) -> Result<Option<String>, String> {
    let Some(at) = html.find("<base ") else {
        return Ok(None);
    };
    let tag = &html[at..at + html[at..].find('>').unwrap_or(html.len() - at)];
    let Some(value) = attributes(tag, "href").next() else {
        return Ok(None);
    };
    let value = unescape(value);
    let outside = || {
        format!("`<base href=\"{value}\">` isn't under `{SITE_PATH}`, where Pages serves the site")
    };
    // Unlike a link, a base must end in `/` to name the root: `/hpr-sim` is a file in `/`.
    let rest = value
        .strip_prefix(SITE_URL)
        .or_else(|| value.strip_prefix(SITE_PATH))
        .ok_or_else(outside)?;
    let (path, _) = split_target(rest);
    join("", &path).map(Some).ok_or_else(outside)
}

/// The file that `value`, an `href` or `src` on the built page `name`, reaches in the site
/// (relative to its root, `dir`), with its fragment; `None` if it leaves for the web, which isn't
/// fetched. `base` is the page's `<base href>`, relative to the site's root, if it has one.
fn html_target(
    dir: &Path,
    name: &str,
    base: Option<&str>,
    value: &str,
) -> Result<Option<(String, Option<String>)>, String> {
    let (start, rest) = if let Some(rest) = by_address(value) {
        ("", rest)
    } else if is_web(value) || value.starts_with("javascript:") || value.starts_with("data:") {
        return Ok(None);
    } else if value.starts_with('/') {
        let rest = under(value, SITE_PATH)
            .ok_or_else(|| format!("isn't under `{SITE_PATH}`, where Pages serves the site"))?;
        ("", rest)
    } else {
        let (path, fragment) = split_target(value);
        // A fragment alone names an `id` on the page, or, under a `<base href>`, on the base.
        if path.is_empty() {
            let page = base.map_or_else(|| name.to_owned(), |base| page_of(dir, base));
            return Ok(Some((page, fragment.map(str::to_owned))));
        }
        (base.map_or_else(|| parent(name), parent), value)
    };
    let (path, fragment) = split_target(rest);
    let target = join(start, &path).ok_or_else(|| "climbs out of the site".to_owned())?;
    Ok(Some((page_of(dir, &target), fragment.map(str::to_owned))))
}

/// The file a path of the site (relative to its root, `dir`) serves: itself, or the `index.html`
/// of a directory.
fn page_of(dir: &Path, path: &str) -> String {
    if path.is_empty() || path.ends_with('/') || dir.join(path).is_dir() {
        format!("{}/index.html", path.trim_end_matches('/'))
            .trim_start_matches('/')
            .to_owned()
    } else {
        path.to_owned()
    }
}

/// The `.html` files under `dir`, as `/`-separated paths relative to it, appended to `out`.
fn html_files(dir: &Path, prefix: &str, out: &mut Vec<String>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|err| format!("could not read {}: {err}", dir.display()))?;
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let Ok(file_name) = entry.file_name().into_string() else {
            continue;
        };
        let name = if prefix.is_empty() {
            file_name
        } else {
            format!("{prefix}/{file_name}")
        };
        let path = entry.path();
        if path.is_dir() {
            html_files(&path, &name, out)?;
        } else if name.ends_with(".html") {
            out.push(name);
        }
    }
    Ok(())
}

/// The values of the double-quoted attribute `name` in `html`, wherever it follows whitespace.
fn attributes<'a>(html: &'a str, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
    let needle = format!("{name}=\"");
    let starts: Vec<usize> = html
        .match_indices(&needle)
        .filter(|(at, _)| {
            html.as_bytes()
                .get(at.wrapping_sub(1))
                .is_some_and(u8::is_ascii_whitespace)
        })
        .map(|(at, _)| at + needle.len())
        .collect();
    starts
        .into_iter()
        .filter_map(move |start| html[start..].split_once('"').map(|(value, _)| value))
}

/// Undoes the HTML escapes mdBook writes in attribute values.
fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Decodes `%XX` escapes; anything that doesn't decode to UTF-8 is left as it was.
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match (bytes[at], hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A workspace in a temporary directory holding `files`, as (path, contents).
    fn workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        dir
    }

    const SUMMARY_TWO_PAGES: &str =
        "# Summary\n\n[Start here](start-here.md)\n\n- [Gravity](physics/gravity.md)\n";

    fn problems(files: &[(&str, &str)]) -> Vec<String> {
        let dir = workspace(files);
        check_sources(dir.path()).unwrap().problems
    }

    #[test]
    fn the_site_passes() {
        let report = check_sources(&crate::designs::root().unwrap()).unwrap();
        assert!(
            report.problems.is_empty(),
            "{}",
            failure("the site's pages", &report.problems)
        );
        assert!(report.pages >= 19, "{} pages", report.pages);
    }

    #[test]
    fn a_consistent_site_passes() {
        let report = check_sources(
            workspace(&[
                ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
                (
                    "docs/start-here.md",
                    "# Start here\n\nSee [gravity](physics/gravity.md#formulas), \
                     [the decision][adr-003] and [Loft lesson L1][lessons], and [the web](https://example.com).\n\n\
                     ```text\nL1150 M1.8 ADR-001 in a code block are quoted verbatim\n```\n\n\
                     [adr-003]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-003-frames-2026-09-17\n\
                     [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md\n",
                ),
                (
                    "docs/physics/gravity.md",
                    &format!("{IN_SHORT_OK}\nBack to [the start](../start-here.md#start-here), \
                     [here](#formulas), [NGA] as a citation, and [the crate](https://github.com/nrdptel/hpr-sim/tree/main/crates/).\n\n\
                     ![A plot of gravity](plot%20one.png?raw=true), <neer@example.com>, \
                     [the source](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md?plain=1#adr-003-frames-2026-09-17) \
                     and [raw](https://github.com/nrdptel/hpr-sim/raw/main/crates/README.md).\n"),
                ),
                ("docs/physics/plot one.png", "png"),
                ("docs/DECISIONS.md", "# Decisions\n\n## ADR-003: Frames (2026-09-17)\n"),
                ("docs/research/loft-lessons.md", "# Lessons\n"),
                ("crates/README.md", "crates\n"),
            ])
            .path(),
        )
        .unwrap();
        assert_eq!(report.problems, Vec::<String>::new());
        // The summary's two links count too; the email address is on the web.
        assert_eq!((report.pages, report.links, report.web), (2, 13, 2));
    }

    #[test]
    fn broken_links_are_reported_exactly() {
        let page = "# Start here\n\n\
                    [a](missing.md)\n\
                    [b](physics/gravity.md#nowhere)\n\
                    [c](../ROADMAP.md)\n\
                    [d](notes.md)\n\
                    [e](/physics/gravity.md)\n\
                    [f](https://github.com/nrdptel/hpr-sim/blob/main/crates/nope.rs)\n\
                    [g](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-999)\n\
                    [h][undefined]\n\
                    [i](#nowhere)\n\
                    [j](physics)\n\
                    [k](SUMMARY.md)\n\n\
                    ## Drag ([a link](physics/gravity.md))\n";
        assert_eq!(
            problems(&[
                ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
                ("docs/start-here.md", page),
                ("docs/physics/gravity.md", IN_SHORT_OK),
                ("docs/notes.md", "# Notes\n"),
                ("docs/DECISIONS.md", "# Decisions\n\n## ADR-001: One\n"),
                ("ROADMAP.md", "# Roadmap\n"),
            ]),
            [
                // What is wrong on the page itself comes first, then its links, each by line.
                "docs/start-here.md:10: `[undefined]` is used as a link, and no `[undefined]: ...` \
                 defines it",
                "docs/start-here.md:15: a link inside a heading: mdBook already makes each \
                 heading a link, and a link inside a link is invalid HTML; move it to the text \
                 below",
                "docs/start-here.md:3: link `missing.md`: no page `docs/missing.md`",
                "docs/start-here.md:4: link `physics/gravity.md#nowhere`: \
                 `docs/physics/gravity.md` has no heading or `<a id>` with the anchor `#nowhere`",
                "docs/start-here.md:5: link `../ROADMAP.md`: leaves `docs/`, which the site can't \
                 follow: link it by its GitHub URL, https://github.com/nrdptel/hpr-sim/blob/main/...",
                "docs/start-here.md:6: link `notes.md`: `docs/notes.md` is not a page of the site \
                 (SUMMARY.md doesn't list it): link it by its GitHub URL, \
                 https://github.com/nrdptel/hpr-sim/blob/main/docs/notes.md",
                "docs/start-here.md:7: link `/physics/gravity.md`: an absolute path means \
                 different things on GitHub and on the site; use a relative link",
                "docs/start-here.md:8: link \
                 `https://github.com/nrdptel/hpr-sim/blob/main/crates/nope.rs`: no \
                 `crates/nope.rs` in the repository",
                "docs/start-here.md:9: link \
                 `https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-999`: \
                 `docs/DECISIONS.md` has no heading or `<a id>` with the anchor `#adr-999`",
                "docs/start-here.md:11: link `#nowhere`: `docs/start-here.md` has no heading or \
                 `<a id>` with the anchor `#nowhere`",
                "docs/start-here.md:12: link `physics`: `docs/physics` is a directory, which has \
                 no page on the site",
                "docs/start-here.md:13: link `SUMMARY.md`: `docs/SUMMARY.md` is the site's table \
                 of contents, which mdBook doesn't write as a page",
            ]
        );
    }

    #[test]
    fn bare_labels_fail_and_linked_ones_pass() {
        let page = "# Start here\n\n\
                    Loft lesson L12 is bare.\n\
                    So is ADR-008.\n\
                    And `M1.5b` in code.\n\
                    | table | M2.1b1 |\n|---|---|\n\n\
                    ## Heading for M1.8\n\n\
                    [Loft lesson L12](https://example.com), [ADR-008][d] and [M1.5b](https://example.com) \
                    are links.\n\n\
                    ```text\nL12 in a fenced block is quoted verbatim\n```\n\n\
                    L1150R and M1297 are motors, Mach 1.8 a speed, XL12 and L12x and M2 and ADR- nothing.\n\n\
                    ![A diagram for ADR-008](x.png)\n\n\
                    <details>\nThe L12 fix <!-- and L13, which no reader sees -->\n</details>\n\n\
                    [d]: https://example.com\n";
        let found = problems(&[
            ("docs/SUMMARY.md", "[Start here](start-here.md)\n"),
            ("docs/start-here.md", page),
            ("docs/x.png", "png"),
        ]);
        let labels: Vec<(&str, &str)> = found
            .iter()
            .map(|problem| {
                let (at, rest) = problem.split_once(": bare label `").unwrap();
                (at, rest.split_once('`').unwrap().0)
            })
            .collect();
        assert_eq!(
            labels,
            [
                ("docs/start-here.md:3", "L12"),
                ("docs/start-here.md:4", "ADR-008"),
                ("docs/start-here.md:5", "M1.5b"),
                ("docs/start-here.md:6", "M2.1b1"),
                ("docs/start-here.md:9", "M1.8"),
                ("docs/start-here.md:19", "ADR-008"),
                ("docs/start-here.md:22", "L12"),
            ]
        );
    }

    #[test]
    fn labels_are_whole_words() {
        assert_eq!(
            labels("L10, ADR-008 and M1.5b's; L41–L43; M0.4a-e; M2.1b1. (L7)"),
            [
                "L10", "ADR-008", "M1.5b", "L41", "L43", "M0.4a", "M2.1b1", "L7"
            ]
        );
        assert_eq!(
            labels(
                "L1150R M1297 Mach 1.8 ADR- XL10 L10x M1 M1.x ADR-7b L 1 M.1 ML1 eq. A1.3 A2.1b"
            ),
            Vec::<&str>::new()
        );
        // A certification level looks like a lesson, so pages write "Level 2" (ADR-016).
        assert_eq!(labels("an L2 certified flyer"), ["L2"]);
    }

    #[test]
    fn math_that_only_github_renders_fails() {
        let page = "# Start here\n\n$$x = 1$$\n\nInline $\\alpha$ too.\n\n```math\nx\n```\n\n\
                    Unicode `γ = GM/r²`, a price of $5, and $5 and $10 and $5-$10 are fine.\n\n\
                    But $F = ma$ is math,\n\nand so is $`x^2`$ and $\\{x\\}$.\n";
        let found = problems(&[
            ("docs/SUMMARY.md", "[Start here](start-here.md)\n"),
            ("docs/start-here.md", page),
        ]);
        let lines: Vec<&str> = found
            .iter()
            .map(|problem| problem.split_once(": GitHub renders").unwrap().0)
            .collect();
        assert_eq!(
            lines,
            [
                "docs/start-here.md:3",
                "docs/start-here.md:5",
                "docs/start-here.md:7",
                "docs/start-here.md:13",
                "docs/start-here.md:15",
                "docs/start-here.md:15",
            ]
        );
    }

    #[test]
    fn every_model_and_format_page_is_in_the_summary() {
        assert_eq!(
            problems(&[
                ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
                ("docs/start-here.md", "# Start here\n"),
                ("docs/physics/gravity.md", IN_SHORT_OK),
                ("docs/physics/wind.md", "# Wind\n"),
                ("docs/format/eng.md", "# eng\n"),
            ]),
            [
                "docs/physics/wind.md: not in docs/SUMMARY.md, so the site doesn't show it",
                "docs/format/eng.md: not in docs/SUMMARY.md, so the site doesn't show it",
            ]
        );
    }

    /// A model page's opening, as the check wants it.
    const IN_SHORT_OK: &str = "# Gravity\n\n## In short\n\n\
                               - **What it models:** the pull of the Earth.\n\
                               - **Sources:** the NGA's standard.\n\
                               - **How well it is validated:** against its printed values.\n\
                               - **What it leaves out:** the Moon.\n\n\
                               ## Formulas\n";

    /// The line and first words of each problem on a site whose one model page is `gravity`.
    fn in_short_problems(gravity: &str) -> Vec<String> {
        problems(&[
            ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
            ("docs/start-here.md", "# Start here\n"),
            ("docs/physics/gravity.md", gravity),
        ])
        .iter()
        .map(|problem| {
            problem
                .split(": a model page opens")
                .next()
                .unwrap()
                .to_owned()
        })
        .collect()
    }

    #[test]
    fn a_model_page_without_in_short_fails() {
        assert_eq!(in_short_problems(IN_SHORT_OK), Vec::<String>::new());
        // A loose list, links and code in the answers, a label wrapped over two lines, and nothing
        // after the list are fine too.
        assert_eq!(
            in_short_problems(
                "# Gravity\n\n## In short\n\n- **What it models:** `γ(φ, h)`.\n\n\
                 - **Sources:** [NGA](https://example.com).\n\n\
                 - **How well it is\n  validated:** [well](#in-short).\n\n\
                 - **What it leaves out:** tides,\n  - and the Moon.\n"
            ),
            Vec::<String>::new()
        );
        for (page, problem) in [
            (
                "# Gravity\n\nCode: `hpr_core::gravity`.\n\n## Formulas\n",
                "docs/physics/gravity.md:3: the title is not followed by `## In short`",
            ),
            (
                "Gravity, with no title.\n",
                "docs/physics/gravity.md:1: the page doesn't open with its title, `# ...`",
            ),
            (
                "# Gravity\n\n## Formulas\n\n## In short\n",
                "docs/physics/gravity.md:3: the first section is `## Formulas`, not `## In short`",
            ),
            (
                "# Gravity\n\n### In short\n",
                "docs/physics/gravity.md:3: the title is not followed by `## In short`",
            ),
            (
                "# Gravity\n\n## In short\n\nIt pulls.\n",
                "docs/physics/gravity.md:5: `## In short` doesn't open with a bulleted list",
            ),
            (
                "# Gravity\n\n## In short\n\n1. **What it models:** the pull.\n",
                "docs/physics/gravity.md:5: `## In short` doesn't open with a bulleted list",
            ),
            (
                &IN_SHORT_OK.replace("## Formulas", "More words."),
                "docs/physics/gravity.md:10: `## In short` holds more than its list; move the \
                 rest under a heading of its own",
            ),
            // A lower heading doesn't end the section.
            (
                &IN_SHORT_OK.replace("## Formulas", "### Also\n\nMore words."),
                "docs/physics/gravity.md:10: `## In short` holds more than its list; move the \
                 rest under a heading of its own",
            ),
            (
                &IN_SHORT_OK.replace("the Moon.", ""),
                "docs/physics/gravity.md:8: `**What it leaves out:**` has no answer after it",
            ),
            (
                &IN_SHORT_OK.replace("the Moon.", "."),
                "docs/physics/gravity.md:8: `**What it leaves out:**` has no answer after it",
            ),
            (
                &IN_SHORT_OK.replace("**Sources:**", "Sources:"),
                "docs/physics/gravity.md:6: an item doesn't open with its label in bold",
            ),
            (
                &IN_SHORT_OK.replace("**Sources:**", "**Sources**:"),
                "docs/physics/gravity.md:5: its items open with `**What it models:**`, \
                 `**Sources**`, `**How well it is validated:**`, `**What it leaves out:**`",
            ),
            (
                &IN_SHORT_OK.replace("- **What it leaves out:** the Moon.\n", ""),
                "docs/physics/gravity.md:5: its items open with `**What it models:**`, \
                 `**Sources:**`, `**How well it is validated:**`",
            ),
        ] {
            assert_eq!(in_short_problems(page), [problem], "{page}");
        }
        // Only model pages open with it: not *Start here*, and not a file format's page.
        assert_eq!(
            problems(&[
                (
                    "docs/SUMMARY.md",
                    "[Start here](start-here.md)\n- [eng](format/eng.md)\n"
                ),
                ("docs/start-here.md", "# Start here\n\nWords.\n"),
                ("docs/format/eng.md", "# eng\n\nWords.\n"),
            ]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn in_short_quotes_only_numbers_its_page_or_links_give() {
        let page = |validated: &str| {
            IN_SHORT_OK
                .replace("against its printed values.", validated)
                .replace(
                    "## Formulas\n",
                    "## Formulas\n\nTable 3.6, to 1e-6.\n\n[wind]: wind.md\n",
                )
        };
        let found = |validated: &str| {
            problems(&[
                (
                    "docs/SUMMARY.md",
                    "[Start here](start-here.md)\n- [Gravity](physics/gravity.md)\n\
                     - [Wind](physics/wind.md)\n",
                ),
                ("docs/start-here.md", "# Start here\n"),
                ("docs/physics/gravity.md", &page(validated)),
                (
                    "docs/physics/wind.md",
                    &IN_SHORT_OK.replace("## Formulas\n", "## Tests\n\nWithin 0.28%.\n"),
                ),
            ])
        };
        // From the page's own body, or from a page the item links, by reference or inline.
        assert_eq!(found("Table 3.6, to 1e-6."), Vec::<String>::new());
        assert_eq!(
            found("drift within 0.28% ([Wind][wind])."),
            Vec::<String>::new()
        );
        assert_eq!(
            found("to 1e-7, and within 0.29% ([Wind](wind.md))."),
            [
                "docs/physics/gravity.md:7: `1e-7` is not in the rest of this page, or in a file \
                 this item links to: link where the number comes from (a model page, the report, \
                 a case file), and quote it as it is there",
                "docs/physics/gravity.md:7: `0.29%` is not in the rest of this page, or in a file \
                 this item links to: link where the number comes from (a model page, the report, \
                 a case file), and quote it as it is there",
            ]
        );
        // A linked model page's own *In short* is not a source: that would trace a summary to a
        // summary.
        assert_eq!(
            problems(&[
                (
                    "docs/SUMMARY.md",
                    "[Start here](start-here.md)\n- [Gravity](physics/gravity.md)\n\
                     - [Wind](physics/wind.md)\n",
                ),
                ("docs/start-here.md", "# Start here\n"),
                (
                    "docs/physics/gravity.md",
                    &IN_SHORT_OK.replace("printed values.", "values, 0.28% ([Wind](wind.md))."),
                ),
                (
                    "docs/physics/wind.md",
                    &IN_SHORT_OK.replace("printed values.", "values, 0.28% (in short only)."),
                ),
            ])
            .iter()
            .map(|problem| problem.split(" is not in").next().unwrap())
            .collect::<Vec<_>>(),
            [
                "docs/physics/gravity.md:7: `0.28%`",
                "docs/physics/wind.md:7: `0.28%`"
            ]
        );
    }

    #[test]
    fn numbers_are_read_as_a_reader_would_check_them() {
        let read = |text: &str| -> Vec<String> {
            quoted(text)
                .iter()
                .filter(|number| number.is_checked())
                .map(ToString::to_string)
                .collect()
        };
        assert_eq!(
            read(
                "Within 3% on 30 numbers; +2.865% and −10% to +19%; 1e-6, 2e−16, 1E-9 and 1,708 \
                 motors; RocketPy 1.13.0 and 2026-09-17."
            ),
            [
                "3%", "30", "+2.865%", "-10%", "+19%", "1e-6", "2e-16", "1E-9", "1708", "1.13.0",
                "2026", "09", "17"
            ]
        );
        // A decimal or exponent glued to its unit is a number; powers of ten in superscripts are
        // one number with their base.
        assert_eq!(
            read("12.5m, 3048m, 20kg, 1.2 × 10⁻⁶, 10⁻¹² and 2²⁰ samples"),
            ["12.5", "3048", "20", "1.2×10⁻⁶", "10⁻¹²", "2²⁰"]
        );
        // A hyphen that joins two words is no minus sign.
        assert_eq!(read("10-20%, WGS-84 and x −5%"), ["10", "20%", "84", "-5%"]);
        assert_eq!(
            read("6-DOF, Level 2, 3 fins, M1.8, C_D0, v0.5.4, L1150R, 3D, 21st, x2.5, #39 and 4."),
            Vec::<String>::new()
        );
        // Commas separate thousands only before exactly three digits.
        assert_eq!(read("0.5,12.3"), ["0.5", "12.3"]);
        let one = |text: &str| quoted(text).remove(0);
        let found =
            |page: &str, source: &str| quoted(source).iter().any(|there| one(page).matches(there));
        assert!(found("+2.865%", "at +2.865% against"));
        assert!(found("2.865%", "at +2.865% against"));
        assert!(found("2e−16", "to 2e-16"));
        assert!(!found("−2.865%", "at +2.865% against"));
        assert!(!found("30%", "5 cases, 30 metrics"));
        assert!(!found("2.865", "2.865e-3"));
        assert!(!found("2.8", "2.865"));
        assert!(!found("30", "M30 and 130"));
    }

    /// A site with an Accuracy page, one model page and a report of the cases `alpha` and `beta`.
    fn accuracy_problems(accuracy: &str, extra: &[(&str, &str)]) -> Vec<String> {
        let gravity = format!("{IN_SHORT_OK}\nIt is 9.780 m/s² at the equator, to 1e-6.\n");
        let mut files = vec![
            (
                "docs/SUMMARY.md",
                "[Start here](start-here.md)\n[Accuracy](accuracy.md)\n\
                 - [Gravity](physics/gravity.md)\n",
            ),
            ("docs/start-here.md", "# Start here\n\nIt is 12.5% off.\n"),
            ("docs/accuracy.md", accuracy),
            ("docs/physics/gravity.md", gravity.as_str()),
            (
                "validation/reports/latest.md",
                "# Report\n\n5 cases.\n\n| case | metric | hpr | difference |\n|---|---|---|---|\n\
                 | alpha | drift_m | 1.0 | +2.865% |\n| alpha | time_s | 2.0 | -0.019% |\n\
                 | beta | drift_m | 3.0 | +0.075% |\n",
            ),
        ];
        files.extend_from_slice(extra);
        problems(&files)
    }

    const REPORT_LINK: &str =
        "https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md";

    /// The link to [`accuracy_problems`]'s one model page that *Accuracy* must have.
    const GRAVITY: &str = "[Gravity](physics/gravity.md).\n\n";

    /// A results table giving all of [`accuracy_problems`]'s report, as it should.
    const RESULTS: &str = "| case | `drift_m` | `time_s` |\n|---|---|---|\n\
                           | [`alpha`][report] | +2.865% | −0.019% |\n\
                           | [`beta`][report] | +0.075% | |\n\n\
                           [report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md\n";

    #[test]
    fn the_accuracy_page_traces_every_number_it_quotes() {
        let page = format!(
            "# Accuracy\n\n\
             The descent agrees within +2.865% ([the report]({REPORT_LINK})).\n\n\
             Gravity matches to 1e-6 ([Gravity](physics/gravity.md)).\n\n\
             Gravity matches to 1e-7 ([Gravity](physics/gravity.md)).\n\n\
             | model | how well |\n|---|---|\n\
             | [Gravity](physics/gravity.md) | 9.780 and 9.781 |\n\n\
             - An item quoting 12.5% with no link.\n\
             - Level 2, 6-DOF, 3 fins and `0.123` in code are not quoted numbers.\n\
             - Nor is a block of code in an item:\n\n  ```text\n  apogee 1234.5 m\n  ```\n\n\
             A link to this page is no source: 9.99% ([below](#details)), nor is a guide page: \
             12.5% ([Start here](start-here.md)).\n\n\
             Numbers in link text count: [within 9.98%](physics/gravity.md).\n\n\
             Nothing glues numbers: 9.780<br>1e-6 and 9[.](physics/gravity.md)780 \
             ([Gravity](physics/gravity.md)).\n\n\
             {RESULTS}\n\
             ## Details\n\nNo numbers here.\n\n## Within 88.8% everywhere\n"
        );
        let found: Vec<String> = accuracy_problems(&page, &[])
            .iter()
            .map(|problem| problem.split(" is not in").next().unwrap().to_owned())
            .collect();
        assert_eq!(
            found,
            [
                "docs/accuracy.md:7: `1e-7`",
                "docs/accuracy.md:11: `9.781`",
                "docs/accuracy.md:13: `12.5%`",
                "docs/accuracy.md:21: `9.99%`",
                "docs/accuracy.md:21: `12.5%`",
                "docs/accuracy.md:23: `9.98%`",
                // `9[.](...)780` is `9` and `780`, not `9.780`.
                "docs/accuracy.md:25: `780`",
                "docs/accuracy.md:38: a heading quotes `88.8%`, and can't link where it comes \
                 from: move the number to the text below",
            ]
        );
    }

    #[test]
    fn the_accuracy_page_gives_every_result_of_the_report_exactly() {
        assert_eq!(
            accuracy_problems(&format!("# Accuracy\n\n{GRAVITY}{RESULTS}"), &[]),
            Vec::<String>::new()
        );
        for (table, problem) in [
            (
                RESULTS.replace("| −0.019% |", "| +0.019% |"),
                "docs/accuracy.md:7: `time_s` of `alpha` is -0.019% in \
                 validation/reports/latest.md, and +0.019% here",
            ),
            (
                RESULTS.replace("| [`beta`][report] | +0.075% | |\n", ""),
                "docs/accuracy.md:1: `drift_m` of `beta` (+0.075% in validation/reports/latest.md) \
                 is in no results table: the page gives every validation result",
            ),
            (
                RESULTS.replace("+0.075% | |", "+0.075% | +1.000% |"),
                "docs/accuracy.md:8: validation/reports/latest.md has no `time_s` for `beta`",
            ),
            (
                RESULTS.replace("| [`beta`][report] |", "| [beta][report] |"),
                "docs/accuracy.md:8: a row of a results table names no case in code, such as \
                 `descent-valetudo`",
            ),
        ] {
            let found = accuracy_problems(&format!("# Accuracy\n\n{GRAVITY}{table}"), &[]);
            assert!(found.iter().any(|found| found == problem), "{found:#?}");
        }
        // A report that gives a result twice, or none at all, can't be checked against.
        let twice = "# Report\n\n| case | metric | difference |\n|---|---|---|\n\
                     | alpha | drift_m | +2.865% |\n\n| case | metric | difference |\n|---|---|---|\n\
                     | alpha | drift_m | +9.999% |\n";
        let none =
            "# Report\n\n| Case | Metric | Diff |\n|---|---|---|\n| alpha | drift_m | +2.865% |\n";
        for (report, problem) in [
            (
                twice,
                "docs/accuracy.md:1: validation/reports/latest.md gives `drift_m` of `alpha` twice, \
                 so the page can't give it once",
            ),
            (
                none,
                "docs/accuracy.md:1: validation/reports/latest.md has no table of results, with \
                 `case`, `metric` and `difference` columns",
            ),
        ] {
            let found = accuracy_problems(
                &format!("# Accuracy\n\n{GRAVITY}"),
                &[("validation/reports/latest.md", report)],
            );
            assert!(found.iter().any(|found| found == problem), "{found:#?}");
        }
    }

    #[test]
    fn the_accuracy_page_links_every_model_page() {
        let wind = IN_SHORT_OK.replace("# Gravity", "# Wind");
        assert_eq!(
            accuracy_problems(
                &format!("# Accuracy\n\n[Gravity](physics/gravity.md).\n\n{RESULTS}"),
                &[("docs/physics/wind.md", &wind)]
            ),
            [
                "docs/physics/wind.md: not in docs/SUMMARY.md, so the site doesn't show it",
                "docs/accuracy.md:1: no link to the model page `docs/physics/wind.md`, whose \
                 checks this page gives",
            ]
        );
    }

    #[test]
    fn the_pages_that_hold_the_rules_stay_in_the_summary() {
        assert_eq!(
            problems(&[
                ("docs/SUMMARY.md", "[Start here](start-here.md)\n"),
                ("docs/start-here.md", "# Start here\n"),
                (
                    "docs/accuracy.md",
                    "# Accuracy\n\nWithin 99.9% with no link.\n"
                ),
                ("docs/decisions-and-roadmap.md", "# Records\n"),
            ]),
            [
                "docs/accuracy.md: not in docs/SUMMARY.md, so the site doesn't show it",
                "docs/decisions-and-roadmap.md: not in docs/SUMMARY.md, so the site doesn't show it",
            ]
        );
    }

    #[test]
    fn the_records_page_links_every_decision_and_phase() {
        let blob = "https://github.com/nrdptel/hpr-sim/blob/main/docs";
        let page = format!(
            "# Decisions and the roadmap\n\n\
             [ADR-000: kickoff]({blob}/DECISIONS.md#adr-000-kickoff-2026-09-16) and \
             [Phase 0]({blob}/ROADMAP.md#phase-0-foundations).\n"
        );
        assert_eq!(
            problems(&[
                (
                    "docs/SUMMARY.md",
                    "[Decisions and the roadmap](decisions-and-roadmap.md)\n"
                ),
                ("docs/decisions-and-roadmap.md", &page),
                (
                    "docs/DECISIONS.md",
                    "# Decisions\n\n## ADR-000: Kickoff (2026-09-16)\n\n### ADR-000a: a part\n\n\
                     ## ADR-001: Licence\n\n## Index\n",
                ),
                (
                    "docs/ROADMAP.md",
                    "# Roadmap\n\n## Phase 0: Foundations\n\n## Phase 1: Physics\n\n## Notes\n",
                ),
            ]),
            [
                "docs/decisions-and-roadmap.md:1: no link to `ADR-001: Licence` in \
                 docs/DECISIONS.md: link it as \
                 https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-001-licence",
                "docs/decisions-and-roadmap.md:1: no link to `Phase 1: Physics` in \
                 docs/ROADMAP.md: link it as \
                 https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md#phase-1-physics",
            ]
        );
    }

    #[test]
    fn the_records_page_has_a_row_for_every_milestone_with_its_status() {
        let roadmap = "# Roadmap\n\n## Phase 0: Foundations\n\n\
                       - [x] **M0.1 Workspace.**\n\
                       - [ ] **M0.4 A site.**\n  - [x] **M0.4a Checks.**\n\
                       \x20 - [ ] [blocked] **M0.4d Publish.**\n\
                       - [ ] **M1.10 Outputs.**\n- [ ] **Not a milestone.**\n";
        assert_eq!(
            roadmap_milestones(roadmap),
            [
                ("M0.1".to_owned(), "done"),
                ("M0.4".to_owned(), "not yet done"),
                ("M0.4a".to_owned(), "done"),
                ("M0.4d".to_owned(), "blocked"),
                ("M1.10".to_owned(), "not yet done"),
            ]
        );
        let page = "| milestone | what | status |\n|---|---|---|\n\
                    | <a id=\"m0-1\"></a>[M0.1][phase-0] | the workspace | done |\n\
                    | <a id=\"m0-4\"></a>[M0.4][phase-0] | a site | done |\n\
                    | <a id=\"m04a\"></a>[M0.4a][phase-0] | checks | done |\n\
                    | <a id=\"m0-4d\"></a>[M0.4d][phase-0] | publish | blocked |\n\
                    | <a id=\"m0-4d\"></a>[M0.4d][phase-0] | publish | blocked |\n\
                    | <a id=\"m9-9\"></a>[M9.9][phase-9] | nothing | not yet done |\n\
                    | <a id=\"l15\"></a>[L15][lessons] | a lesson | [M1.5b](#m1-5b) |\n";
        assert_eq!(
            milestone_rows(page, roadmap),
            [
                (
                    4,
                    "`M0.4` is not yet done in docs/ROADMAP.md, and its row says `done`".to_owned()
                ),
                (
                    5,
                    "`M0.4a`'s row is anchored `#m04a`: anchor it `#m0-4a`".to_owned()
                ),
                (7, "`M0.4d` has two rows".to_owned()),
                (8, "`M9.9` is not a milestone of docs/ROADMAP.md".to_owned()),
                (
                    1,
                    "`M1.10`, not yet done in docs/ROADMAP.md, has no row in the table of \
                     milestones: add `| <a id=\"m1-10\"></a>[M1.10][phase-N] | what it covers | \
                     not yet done |`"
                        .to_owned()
                ),
            ]
        );
        let odd = "- [X] **M0.1 Done.**\n- [ ] **M1.11** Foo\n- [ ] **M1.12: Bar.**\n\
                   - [?] **M1.13 Baz.**\n- [ ] **Mars.**\n";
        assert_eq!(roadmap_milestones(odd), [("M0.1".to_owned(), "done")]);
        assert_eq!(
            unreadable_milestones(odd),
            [
                "- [ ] **M1.11** Foo",
                "- [ ] **M1.12: Bar.**",
                "- [?] **M1.13 Baz.**"
            ]
        );
        assert_eq!(milestone_rows("", odd).len(), 4);
        for id in ["M1.1", "M1.10", "M2.1b2", "M0.4e"] {
            assert!(is_milestone_id(id), "{id}");
        }
        for id in ["M1", "M.1", "Mx.1", "M1.", "M1.1bb", "M1.1b2c", "L15"] {
            assert!(!is_milestone_id(id), "{id}");
        }
    }

    #[test]
    fn the_summary_lists_pages_once_by_relative_path() {
        assert_eq!(
            problems(&[
                (
                    "docs/SUMMARY.md",
                    "[A](start-here.md)\n[B](start-here.md)\n[C](https://example.com)\n[D](gone.md)\n"
                ),
                ("docs/start-here.md", "# Start here\n"),
            ])
            .iter()
            .map(|problem| problem.split(": ").take(2).collect::<Vec<_>>().join(": "))
            .collect::<Vec<_>>(),
            [
                "docs/SUMMARY.md:2: `start-here.md` is listed twice",
                "docs/SUMMARY.md:3: `https://example.com` is not a page",
                "docs/gone.md: listed in docs/SUMMARY.md, but can't be read",
                "docs/SUMMARY.md:4: link `gone.md`",
            ]
        );
    }

    #[test]
    fn a_quote_is_its_file_line_for_line() {
        let dir = workspace(&[
            ("crates/a/examples/fly.rs", "fn main() {\n    fly();\n}\n"),
            ("crates/a/examples/crlf.rs", "fn main() {}\r\n"),
        ]);
        let check = |page: &str| -> Vec<String> {
            quotes(dir.path(), page)
                .into_iter()
                .map(|(line, why)| format!("{line}: {why}"))
                .collect()
        };
        let quote = |path: &str, block: &str| {
            format!("# Page\n\nThe example:\n\n<!-- quote: {path} -->\n```rust\n{block}```\n")
        };
        // The file as it is, whatever its line endings; an unmarked block isn't a quote.
        assert!(
            check(&quote(
                "crates/a/examples/fly.rs",
                "fn main() {\n    fly();\n}\n"
            ))
            .is_empty()
        );
        assert!(check(&quote("crates/a/examples/crlf.rs", "fn main() {}\n")).is_empty());
        assert!(check("# Page\n\n```rust\nanything\n```\n").is_empty());

        assert_eq!(
            check(&quote(
                "crates/a/examples/fly.rs",
                "fn main() {\n    walk();\n}\n"
            )),
            [
                "5: the quote of `crates/a/examples/fly.rs` differs from it at its line 2: the page \
              has `    walk();`, the file `    fly();`; copy the file into the block again"
            ]
        );
        assert_eq!(
            check(&quote("crates/a/examples/fly.rs", "fn main() {\n")),
            [
                "5: the quote of `crates/a/examples/fly.rs` differs from it at its line 2: the page \
              has nothing, the file `    fly();`; copy the file into the block again"
            ]
        );
        let missing = check(&quote("crates/a/examples/walk.rs", "fn main() {}\n"));
        assert!(
            missing[0].starts_with("5: the quote of `crates/a/examples/walk.rs` can't be checked"),
            "{missing:?}"
        );
        for path in ["../outside.rs", "/etc/hosts", ""] {
            let outside = check(&quote(path, "x\n"));
            assert!(
                outside[0].contains("is not a file of the repository"),
                "{path}: {outside:?}"
            );
        }
    }

    #[test]
    fn a_quote_marker_needs_a_fenced_block_right_after_it() {
        let dir = workspace(&[("fly.rs", "x\n")]);
        let lines = |page: &str| -> Vec<usize> {
            quotes(dir.path(), page)
                .into_iter()
                .inspect(|(_, why)| {
                    assert!(why.contains("no fenced code block follows it"), "{why}")
                })
                .map(|(line, _)| line)
                .collect()
        };
        // Prose in between, an indented block, another marker, or the end of the page.
        assert_eq!(
            lines("<!-- quote: fly.rs -->\n\nSome prose.\n\n```\nx\n```\n"),
            [1]
        );
        assert_eq!(lines("<!-- quote: fly.rs -->\n\n    x\n"), [1]);
        assert_eq!(
            lines("<!-- quote: fly.rs -->\n<!-- quote: fly.rs -->\n```\nx\n```\n"),
            [1]
        );
        assert_eq!(lines("Text.\n\n<!-- quote: fly.rs -->\n"), [3]);
        // Other comments are not markers, and a blank line before the block is fine.
        assert!(lines("<!-- a note -->\n```\ny\n```\n").is_empty());
        assert!(lines("<!-- quote: fly.rs -->\n\n```text\nx\n```\n").is_empty());
    }

    #[test]
    fn a_mistyped_quote_marker_fails_rather_than_checking_nothing() {
        let dir = workspace(&[("fly.rs", "x\n")]);
        let found = |page: &str| -> Vec<(usize, bool)> {
            quotes(dir.path(), page)
                .into_iter()
                .map(|(line, why)| (line, why.contains("looks like a quote marker")))
                .collect()
        };
        // A marker may span lines.
        assert!(found("<!--\nquote: fly.rs\n-->\n```\nx\n```\n").is_empty());
        // Text after it, a capital letter, or a marker inside a paragraph are mistakes.
        assert_eq!(
            found("<!-- quote: fly.rs --> note\n```\nx\n```\n"),
            [(1, true)]
        );
        assert_eq!(found("<!-- Quote: fly.rs -->\n```\nx\n```\n"), [(1, true)]);
        assert_eq!(
            found("Some text <!-- quote: fly.rs --> here.\n\n```\nx\n```\n"),
            [(1, true)]
        );
        // Other HTML between a marker and its block parts them.
        assert_eq!(
            found("<!-- quote: fly.rs -->\n<!-- a note -->\n```\nx\n```\n"),
            [(1, false)]
        );
        assert_eq!(
            found("<!-- quote: fly.rs -->\n<div>\n\n```\nx\n```\n"),
            [(1, false)]
        );
    }

    #[test]
    fn a_page_that_misquotes_a_file_fails_the_site() {
        let found = problems(&[
            (
                "docs/SUMMARY.md",
                "# Summary\n\n[Start here](start-here.md)\n",
            ),
            (
                "docs/start-here.md",
                "# Start here\n\n<!-- quote: out.txt -->\n```text\nold\n```\n",
            ),
            ("out.txt", "new\n"),
        ]);
        assert_eq!(
            found,
            [
                "docs/start-here.md:3: the quote of `out.txt` differs from it at its line 1: the page \
              has `old`, the file `new`; copy the file into the block again"
            ]
        );
    }

    #[test]
    fn slugs_match_githubs() {
        for (heading, anchor) in [
            (
                "The 1976 standard, −5 km to 86 km",
                "the-1976-standard-5-km-to-86-km",
            ),
            (
                "Geodetic to ECEF ([NGA] eqs. 4-14, 4-15)",
                "geodetic-to-ecef-nga-eqs-4-14-4-15",
            ),
            (
                "ADR-008: Body CP, fins and interference (2026-09-17)",
                "adr-008-body-cp-fins-and-interference-2026-09-17",
            ),
            ("snake_case and µ", "snake_case-and-µ"),
        ] {
            assert_eq!(slug(heading), anchor);
        }
        let page = read_page("# Tests\n\n## Tests\n\n## Tests\n");
        assert_eq!(
            page.anchors.into_iter().collect::<Vec<_>>(),
            ["tests", "tests-1", "tests-2"]
        );
    }

    #[test]
    fn an_a_id_is_an_anchor_and_other_ids_are_not() {
        let page = read_page(
            "# Records\n\n| id | what |\n|---|---|\n| <a id=\"m1-9\"></a>M | staging |\n\
             | <a  href=\"x.md\" id=\"l15\">L</a> | a lesson |\n\n<span id=\"not-a\"></span>\n\
             <a id=\"\"></a> <a name=\"old\"></a> <abbr id=\"abbr\">x</abbr>\n",
        );
        assert_eq!(
            page.anchors.into_iter().collect::<Vec<_>>(),
            ["l15", "m1-9", "records"]
        );
        assert_eq!(anchor_ids("<a id=\"a\"><a id=\"b\">"), ["a", "b"]);
        // A tag shown in code, or inside a comment, names no anchor.
        let text = "# T\n\n`<a id=\"code\">`\n\n```html\n<a id=\"block\"></a>\n```\n\n\
                    <!-- <a id=\"comment\"></a> -->\n\n<a id=\"real\"></a>\n";
        assert_eq!(
            anchors_of(text).into_iter().collect::<Vec<_>>(),
            ["real", "t"]
        );
        assert_eq!(
            read_page(text).anchors.into_iter().collect::<Vec<_>>(),
            ["real", "t"]
        );
    }

    #[test]
    fn broken_links_in_the_built_site_are_reported() {
        let dir = workspace(&[
            (
                "index.html",
                "<a href=\"physics/aero.html#lift\">ok</a> <a href=\"physics/aero.html#drag\">x</a>\n\
                 <a href=\"physics/wind.html\">x</a> <a href=\"../outside.html\">x</a>\n\
                 <a href=\"https://example.com/nowhere.html\">web</a> <a href=\"#top\">ok</a>\n\
                 <a href=\"physics/\">ok</a> <a href=\"physics/aero.html#%C2%B5-term\">ok</a>\n\
                 <link rel=\"stylesheet\" href=\"css/site.css\"><h1 id=\"top\">t</h1>\n\
                 <script src=\"css/site.css\"></script><img src=\"missing.png\">",
            ),
            (
                "physics/aero.html",
                "<h2 id=\"lift\">Lift</h2><h2 id=\"µ-term\">µ</h2><a href=\"../index.html#top\">ok</a>",
            ),
            ("physics/index.html", "<p>index</p>"),
            ("css/site.css", "body {}"),
        ]);
        let built = check_html(dir.path(), dir.path(), &[]).unwrap();
        assert_eq!((built.pages, built.api), (3, 0));
        assert_eq!(
            built.problems,
            [
                "index.html: `href=\"physics/aero.html#drag\"`: `physics/aero.html` has no \
                 `id=\"drag\"`",
                "index.html: `href=\"physics/wind.html\"`: no `physics/wind.html` in the site",
                "index.html: `href=\"../outside.html\"` climbs out of the site",
                "index.html: `src=\"missing.png\"`: no `missing.png` in the site",
            ]
        );
    }

    #[test]
    fn root_absolute_links_and_a_base_are_read_under_the_site_path() {
        let dir = workspace(&[
            // As mdBook writes its 404 page when `book.toml` sets `site-url`.
            (
                "404.html",
                "<base href=\"/hpr-sim/\"><link href=\"css/site.css\">\n\
                 <h1 id=\"not-found\"><a href=\"#not-found\">Not found</a></h1> <a href=\"#gone\">x</a>\n\
                 <a href=\"/hpr-sim/\">ok</a> <a href=\"/hpr-sim\">ok</a> <a href=\"/hpr-sim#top\">ok</a>\n\
                 <a href=\"/hpr-sim?q=1\">ok</a> <a href=\"/hpr-sim/physics/aero.html#lift\">ok</a>\n\
                 <a href=\"/elsewhere/\">x</a> <a href=\"/hpr-simulator/\">x</a>\n\
                 <a href=\"https://nrdptel.github.io/hpr-sim/physics/aero.html#drag\">x</a>\n\
                 <a href=\"https://nrdptel.github.io/hpr-sim\">ok</a>",
            ),
            // Under a base, a fragment alone names an `id` on the base's page, not this one.
            (
                "physics/aero.html",
                "<base href=\"/hpr-sim/physics/\"><a href=\"../index.html\">ok</a>\n\
                 <h2 id=\"lift\">Lift</h2><a href=\"aero.html#lift\">ok</a> <a href=\"#lift\">x</a>",
            ),
            ("physics/climbs.html", "<base href=\"/hpr-sim/../\">"),
            ("physics/moved.html", "<base href=\"/other/\">"),
            ("physics/slashless.html", "<base href=\"/hpr-sim\">"),
            // A base without an `href` moves nothing; the next tag's `href` is not the base.
            (
                "physics/tagged.html",
                "<base target=\"_blank\"><a href=\"/hpr-sim/physics/aero.html\">ok</a>\n\
                 <a href=\"aero.html\">ok</a>",
            ),
            // A base is an `href` too, so it must reach a page.
            ("physics/index.html", "<p>physics</p>"),
            ("index.html", "<h1 id=\"top\">home</h1>"),
            ("css/site.css", "body {}"),
        ]);
        let built = check_html(dir.path(), dir.path(), &[]).unwrap();
        assert_eq!(
            built.problems,
            [
                "404.html: `href=\"#gone\"`: `index.html` has no `id=\"gone\"`",
                "404.html: `href=\"/elsewhere/\"` isn't under `/hpr-sim/`, where Pages serves the \
                 site",
                "404.html: `href=\"/hpr-simulator/\"` isn't under `/hpr-sim/`, where Pages serves \
                 the site",
                "404.html: `href=\"https://nrdptel.github.io/hpr-sim/physics/aero.html#drag\"`: \
                 `physics/aero.html` has no `id=\"drag\"`",
                "physics/aero.html: `href=\"#lift\"`: `physics/index.html` has no `id=\"lift\"`",
                "physics/climbs.html: `<base href=\"/hpr-sim/../\">` isn't under `/hpr-sim/`, \
                 where Pages serves the site",
                "physics/moved.html: `<base href=\"/other/\">` isn't under `/hpr-sim/`, where Pages \
                 serves the site",
                "physics/slashless.html: `<base href=\"/hpr-sim\">` isn't under `/hpr-sim/`, \
                 where Pages serves the site",
            ]
        );
    }

    #[test]
    fn links_resolve_as_a_browser_reads_them_on_the_site() {
        let dir = workspace(&[("physics/index.html", ""), ("index.html", "")]);
        let target = |name: &str, base: Option<&str>, value: &str| {
            html_target(dir.path(), name, base, value)
        };
        let found = |target: &str, fragment: Option<&str>| {
            Ok(Some((target.to_owned(), fragment.map(str::to_owned))))
        };
        let aero = "physics/aero.html";
        assert_eq!(
            target(aero, None, "wind.html#gust"),
            found("physics/wind.html", Some("gust"))
        );
        assert_eq!(target(aero, None, "#lift"), found(aero, Some("lift")));
        assert_eq!(
            target(aero, Some(""), "#lift"),
            found("index.html", Some("lift"))
        );
        assert_eq!(
            target(aero, Some(""), "wind.html"),
            found("wind.html", None)
        );
        // A directory, with or without its `/`.
        assert_eq!(
            target("a.html", None, "physics"),
            found("physics/index.html", None)
        );
        assert_eq!(
            target("a.html", None, "physics/"),
            found("physics/index.html", None)
        );
        assert_eq!(target("a.html", None, SITE_URL), found("index.html", None));
        assert_eq!(
            target(
                "a.html",
                None,
                &format!("{SITE_URL}physics/aero.html?x=1#drag")
            ),
            found(aero, Some("drag"))
        );
        assert_eq!(
            target("a.html", None, "physics/%C2%B5.html"),
            found("physics/µ.html", None)
        );
        assert_eq!(
            target("a.html", None, &format!("{SITE_URL}../x.html")),
            Err("climbs out of the site".to_owned())
        );
        assert_eq!(target("a.html", None, "https://example.com/"), Ok(None));
        assert_eq!(target("a.html", None, "mailto:neer@example.com"), Ok(None));
    }

    /// A built site with the API reference of the crates `a` and `b` under `api/`, where `b`'s
    /// front page says `b_front`, and whose guide page says `guide`.
    fn api_problems(guide: &str, b_front: &str) -> Vec<String> {
        let dir = workspace(&[
            ("index.html", "<h1 id=\"start\">Start</h1>"),
            ("api.html", guide),
            (
                "api/a/index.html",
                "<a href=\"https://nrdptel.github.io/hpr-sim/#start\">guide</a>\n\
                 <link href=\"../static.files/x.css\"> <a href=\"https://docs.rs/\">web</a>\n\
                 <a href=\"struct.A.html#method.fly\">ok</a> <a href=\"../src/a/lib.rs.html#10-20\">ok</a>\n\
                 <script src=\"../trait.impl/a/trait.T.js\" async></script>\n\
                 <a href=\"crate::DAffine2\">copied from a dependency's documentation</a>",
            ),
            (
                "api/a/struct.A.html",
                "<h4 id=\"method.fly\">fly</h4>\n\
                 <a href=\"https://nrdptel.github.io/hpr-sim/api.html\">ok</a>",
            ),
            ("api/src/a/lib.rs.html", "<a href=\"#10\" id=\"10\">10</a>"),
            ("api/static.files/x.css", "body {}"),
            ("api/b/index.html", b_front),
        ]);
        let built = check_html(dir.path(), dir.path(), &["a".to_owned(), "b".to_owned()]).unwrap();
        assert_eq!((built.pages, built.api), (2, 4));
        built.problems
    }

    #[test]
    fn the_api_reference_and_the_guide_link_each_other() {
        let guide = "<a href=\"api/a/index.html\">a</a> <a href=\"api/b/\">b</a>";
        let b_front = "<a href=\"https://nrdptel.github.io/hpr-sim/api.html\">guide</a>";
        assert_eq!(api_problems(guide, b_front), Vec::<String>::new());
        // `b` doesn't link the guide, and the guide doesn't link `b`.
        assert_eq!(
            api_problems(
                "<a href=\"api/a/index.html\">a</a> <a href=\"api/b/struct.B.html\">b</a>",
                "<a href=\"https://nrdptel.github.io/hpr-sim-old/\">elsewhere</a>"
            ),
            [
                "api.html: `href=\"api/b/struct.B.html\"`: no `api/b/struct.B.html` in the site",
                "api/b/index.html: no page of the site links it, so a reader of the guide can't \
                 find it",
                "api/b/index.html: doesn't link the guide: the crate's documentation (`//!` in its \
                 `lib.rs`) must link a page of https://nrdptel.github.io/hpr-sim/",
            ]
        );
        // A link from the API reference to a page the guide doesn't have.
        assert_eq!(
            api_problems(
                guide,
                "<a href=\"https://nrdptel.github.io/hpr-sim/physics/gone.html\">x</a>"
            ),
            [
                "api/b/index.html: `href=\"https://nrdptel.github.io/hpr-sim/physics/gone.html\"`: \
                 no `physics/gone.html` in the site"
            ]
        );
    }

    #[test]
    fn the_api_reference_is_checked_like_the_guide() {
        let guide = "<a href=\"api/a/index.html\">a</a> <a href=\"api/b/\">b</a>";
        let front = |rest: &str| format!("<a href=\"{SITE_URL}api.html\">guide</a>\n{rest}");
        assert_eq!(
            api_problems(
                guide,
                &front(
                    "<a href=\"struct.Gone.html\">x</a> <a href=\"../a/struct.A.html#method.run\">x</a>\n\
                     <script src=\"../static.files/gone.js\"></script>"
                )
            ),
            [
                "api/b/index.html: `href=\"struct.Gone.html\"`: no `api/b/struct.Gone.html` in the \
                 site",
                "api/b/index.html: `href=\"../a/struct.A.html#method.run\"`: `api/a/struct.A.html` \
                 has no `id=\"method.run\"`",
                "api/b/index.html: `src=\"../static.files/gone.js\"`: no `api/static.files/gone.js` \
                 in the site",
            ]
        );
        // A link to another crate's item that rustdoc could only leave as text; a list of code
        // in brackets is not one.
        assert_eq!(
            api_problems(
                guide,
                &front(
                    "<p>From a [<code>a::Layout</code>], [a::Thing], [<code>x</code>, <code>y</code>], \
                     [<code>self.xy()</code>] and [glam::Vec3]</p>\n\
                     <a href=\"a::Model\">x</a> <a href=\"crate::DAffine2\">copied</a>"
                )
            ),
            [
                "api/b/index.html: rustdoc left the link to `a::Layout` unresolved, because that \
                 crate's pages weren't built before this one's",
                "api/b/index.html: rustdoc left the link to `a::Thing` unresolved, because that \
                 crate's pages weren't built before this one's",
                "api/b/index.html: rustdoc left the link to `a::Model` unresolved, because that \
                 crate's pages weren't built before this one's",
            ]
        );
    }

    /// The problems of a built site with the API reference of one crate, `b`, whose page for an
    /// item says `item` and whose source page says `source`.
    fn api_label_problems(item: &str, source: &str) -> Vec<String> {
        let front = format!("<a href=\"{SITE_URL}\">guide</a>");
        let dir = workspace(&[
            ("index.html", "<a href=\"api/b/index.html\">b</a>"),
            ("api/b/index.html", &front),
            ("api/b/struct.B.html", item),
            ("api/src/b/lib.rs.html", source),
            // A file its GitHub links may reach, in the workspace the check reads.
            ("docs/research/loft-lessons.md", "# Lessons\n"),
        ]);
        let built = check_html(dir.path(), dir.path(), &["b".to_owned()]).unwrap();
        assert_eq!((built.pages, built.api), (1, 3));
        built.problems
    }

    #[test]
    fn a_bare_label_in_the_api_reference_fails() {
        // Seen as the guide's pages are: in prose, headings and tables, and in any element but a
        // link, code, a script or a style, whatever comes before it.
        let item = "<h1>Struct <span>B</span></h1>\n\
                    <p>Staging comes with M1.9 (ADR-014).</p><!-- <code> -->\n\
                    <h2 id=\"x\"><a class=\"doc-anchor\" href=\"#x\">§</a>Loft lesson L25</h2>\n\
                    <table><tr><td><abbr>M2.1b1</abbr></td></tr></table>\n\
                    <pre><code>L77</code></pre><em>M1.8</em>";
        let found = api_label_problems(item, "");
        let labels: Vec<(&str, &str)> = found
            .iter()
            .map(|problem| {
                let (at, rest) = problem
                    .split_once(": a doc comment has a bare label `")
                    .unwrap();
                (at, rest.split_once('`').unwrap().0)
            })
            .collect();
        let at = "api/b/struct.B.html";
        assert_eq!(
            labels,
            [
                (at, "M1.9"),
                (at, "ADR-014"),
                (at, "L25"),
                (at, "M2.1b1"),
                (at, "M1.8")
            ]
        );
        assert_eq!(
            found[0],
            "api/b/struct.B.html: a doc comment has a bare label `M1.9`: make it a link, with a \
             few words saying what it is (ADR-016). A motor designation is written in full, such \
             as `L1150R`, and a certification level in words, such as \"Level 2\""
        );
    }

    #[test]
    fn a_linked_or_quoted_label_in_the_api_reference_passes() {
        let item = "<p>A value needs a source \
                    (<a href=\"https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md\">\
                    Loft lesson L77</a>), and staging comes with <a href=\"index.html\">\
                    <code>M1.9</code></a>.</p>\n\
                    <pre class=\"rust\"><code>// ADR-009, <span>M1.8</span></code></pre> \
                    <code>M1.5b</code>\n\
                    <script>let m = \"M1.9\";</script><style>/* L6 */</style>\n\
                    <!-- L25, which no reader sees --> L1150R and M1297 are motors, Mach 1.8 a speed";
        assert_eq!(api_label_problems(item, ""), Vec::<String>::new());
    }

    #[test]
    fn the_api_references_source_pages_quote_labels_as_written() {
        let source =
            "<p>/// Loft lesson L77: Loft shipped hand-written results (ADR-015, M2.1a).</p>";
        assert_eq!(api_label_problems("", source), Vec::<String>::new());
        // The same text on an item's page fails.
        assert_eq!(api_label_problems(source, "").len(), 3);
    }

    #[test]
    fn prose_is_what_a_reader_sees_outside_links_and_code() {
        assert_eq!(
            prose_text(
                "<p>A <a href=\"x\">L1</a> b <abbr>c</abbr> <code>L2</code>d<pre>L3</pre>\
                 <!-- <a> --> e <aside>f</aside> <script>L4</script><style>L5</style>g</p> h"
            ),
            " A   b  c   d  e  f    g  h"
        );
        // Rustdoc closes one element right before the next text; the two stay apart, so a doc
        // comment that opens with a label is seen.
        let glued = "<span>Expand description</span></summary><div class=\"docblock\"><p>M1.9 adds\
                     </p><table><tr><td>x</td><td>L25</td></tr></table>";
        let mut bare = Vec::new();
        scan_labels(&prose_text(glued), 0, &mut bare);
        assert_eq!(bare.len(), 2, "{bare:?}");
        // An element that never closes hides the rest of the page, as an unclosed tag does.
        assert_eq!(prose_text("a <code>b c"), "a  ");
        assert!(opens("<a>", "a") && opens("<a\nhref=\"x\">", "a"));
        assert!(!opens("<abbr>", "a") && !opens("</a>", "a") && !opens("<code/>", "code"));
    }

    #[test]
    fn the_api_references_links_into_the_repository_are_checked() {
        let front = format!(
            "<a href=\"https://nrdptel.github.io/hpr-sim/start-here.html\">guide</a>\
             <a href=\"{GITHUB}blob/main/docs/DECISIONS.md#adr-001-licence\">ok</a>\
             <a href=\"{GITHUB}blob/main/docs/DECISIONS.md#adr-999\">x</a>\
             <a href=\"{GITHUB}blob/main/crates/nope.rs\">x</a>\
             <a href=\"{GITHUB}issues/27\">web</a>"
        );
        let dir = workspace(&[
            ("start-here.html", "<a href=\"api/a/index.html\">api</a>"),
            ("api/a/index.html", &front),
            ("docs/DECISIONS.md", "# Decisions\n\n## ADR-001: Licence\n"),
        ]);
        let built = check_html(dir.path(), dir.path(), &["a".to_owned()]).unwrap();
        assert_eq!(
            built.problems,
            [
                format!(
                    "api/a/index.html: `href=\"{GITHUB}blob/main/docs/DECISIONS.md#adr-999\"`: \
                     `docs/DECISIONS.md` has no heading or `<a id>` with the anchor `#adr-999`"
                ),
                format!(
                    "api/a/index.html: `href=\"{GITHUB}blob/main/crates/nope.rs\"`: no \
                     `crates/nope.rs` in the repository"
                ),
            ]
        );
    }

    #[test]
    fn a_crate_without_an_api_reference_fails() {
        let dir = workspace(&[("index.html", "<p>home</p>")]);
        let built = check_html(dir.path(), dir.path(), &["hpr_core".to_owned()]).unwrap();
        assert_eq!(
            built.problems,
            ["api/hpr_core/index.html: missing, so the crate `hpr_core` has no API reference"]
        );
    }

    fn package(name: &str, lib: bool, dependencies: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            wasm: false,
            lib: lib.then(|| name.replace('-', "_")),
            dependencies: dependencies.iter().map(|&d| d.to_owned()).collect(),
            examples: Vec::new(),
        }
    }

    #[test]
    fn crates_are_documented_after_the_crates_they_use() {
        let packages = [
            package("sim", true, &["core", "aero", "serde"]),
            package("cli", false, &["sim"]),
            package("aero", true, &["core"]),
            package("core", true, &[]),
            package("atmos", true, &["core"]),
        ];
        let order: Vec<&str> = doc_order(&packages)
            .unwrap()
            .iter()
            .map(|package| package.name.as_str())
            .collect();
        assert_eq!(order, ["core", "aero", "atmos", "sim"]);
        let cycle = [package("a", true, &["b"]), package("b", true, &["a"])];
        assert_eq!(
            doc_order(&cycle).unwrap_err(),
            "the libraries [\"a\", \"b\"] depend on each other in a cycle"
        );
    }

    #[test]
    fn rustdocs_output_is_found_for_any_target() {
        let host = workspace(&[("doc/a/index.html", ""), ("debug/x", "")]);
        assert_eq!(doc_dirs(host.path()), [host.path().join("doc")]);
        let named = workspace(&[("x86_64-unknown-linux-gnu/doc/a/index.html", "")]);
        assert_eq!(
            doc_dirs(named.path()),
            [named.path().join("x86_64-unknown-linux-gnu").join("doc")]
        );
        assert!(doc_dirs(&host.path().join("missing")).is_empty());
    }

    #[test]
    fn copying_the_reference_leaves_out_cargos_lock() {
        let from = workspace(&[
            ("doc/.lock", ""),
            ("doc/a/index.html", "a"),
            ("doc/a/b/c.js", "c"),
        ]);
        let to = tempfile::tempdir().unwrap();
        let dest = to.path().join("api");
        copy_dir(&from.path().join("doc"), &dest).unwrap();
        assert_eq!(fs::read_to_string(dest.join("a/index.html")).unwrap(), "a");
        assert_eq!(fs::read_to_string(dest.join("a/b/c.js")).unwrap(), "c");
        assert!(!dest.join(".lock").exists());
    }

    #[test]
    fn the_arguments_are_read_in_any_order() {
        let args =
            |list: &[&str]| parse_args(&list.iter().map(|&arg| arg.to_owned()).collect::<Vec<_>>());
        assert_eq!(args(&[]), Ok((true, false)));
        assert_eq!(args(&["--locked"]), Ok((true, true)));
        assert_eq!(args(&["--locked", "--no-build"]), Ok((false, true)));
        let err = args(&["--fast"]).unwrap_err();
        assert!(err.starts_with("unexpected argument `--fast`"), "{err}");
    }

    #[test]
    fn the_site_is_built_for_the_address_pages_serves_it_at() {
        assert!(SITE_URL.ends_with(SITE_PATH), "{SITE_URL} {SITE_PATH}");
        let root = crate::designs::root().unwrap();
        let book = fs::read_to_string(root.join("book.toml")).unwrap();
        let site_url = format!("site-url = \"{SITE_PATH}\"");
        assert!(
            book.lines().any(|line| line.trim() == site_url),
            "book.toml must set `{site_url}` under [output.html]"
        );
        // The README's first lines link the published site (M0.4d).
        let readme = fs::read_to_string(root.join("README.md")).unwrap();
        let top: String = readme.lines().take(8).collect::<Vec<_>>().join("\n");
        assert!(
            top.contains(SITE_URL),
            "README.md's first lines must link {SITE_URL}"
        );
    }

    #[test]
    fn paths_resolve_without_the_filesystem() {
        assert_eq!(
            join("physics", "../start-here.md").as_deref(),
            Some("start-here.md")
        );
        assert_eq!(
            join("physics", "./aero.md").as_deref(),
            Some("physics/aero.md")
        );
        assert_eq!(join("", "../ROADMAP.md"), None);
        assert_eq!(join("a", "b/").as_deref(), Some("a/b/"));
        assert_eq!(percent_decode("%C2%B5-term"), "µ-term");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(unescape("a&amp;b"), "a&b");
    }
}
