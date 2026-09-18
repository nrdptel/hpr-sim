//! The documentation site (M0.4a; ADR-016 in `docs/DECISIONS.md`). mdBook builds it from `docs/`,
//! and the checks here hold every page to what a reader needs.
//!
//! `docs/SUMMARY.md` lists the pages. Each page has one source, which GitHub and the site both
//! render, so the checks hold it to both:
//!
//! - **Links.** A relative link reaches another page of the site, or a file under `docs/` that
//!   mdBook copies, and its `#fragment` a heading there. The rest of the repository is linked by
//!   its `https://github.com/nrdptel/hpr-sim/blob/main/...` URL, which is checked against the
//!   working tree, fragment included, so a PR that moves or renames a file fails until its links
//!   follow. Other web links are counted, not fetched: a PR's checks don't depend on the network.
//! - **Bare labels.** An internal label (`L12`, a Loft lesson; `ADR-008`; a milestone id such as
//!   `M1.5b`) means nothing to a reader on its own, so outside a link's text it fails (CLAUDE.md,
//!   "Easy to read"). Fenced code blocks are exempt: they quote files and output verbatim.
//! - **Equations** are written in Unicode, which reads the same on GitHub, on the site and in
//!   rustdoc. `$$` display math and ```` ```math ```` blocks render on GitHub only, so they fail.
//! - **Coverage.** Every page under `docs/physics/` and `docs/format/` is in the summary.
//!
//! `cargo test` runs these on the real pages. `cargo xtask site` runs them, builds the site into
//! `target/site` with mdBook, and checks the built HTML as well: every relative `href` must reach
//! a file and an `id` in the output, which catches what mdBook itself rewrites.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use pulldown_cmark::{BrokenLink, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};

pub const USAGE: &str =
    "  site [--no-build]        Check the documentation site's pages, build the site with
                           mdBook 0.5 into target/site and check the built HTML.
                           --no-build checks the pages only.";

/// The site's source directory, relative to the workspace root, as `book.toml` sets it.
const SOURCE: &str = "docs";
/// The site's list of pages, relative to [`SOURCE`].
const SUMMARY: &str = "SUMMARY.md";
/// Where `book.toml` has mdBook write the site, relative to the workspace root.
const OUTPUT: &str = "target/site";
/// The repository on GitHub. A page links anything that isn't a page of the site this way.
const GITHUB: &str = "https://github.com/nrdptel/hpr-sim/";
/// Directories under [`SOURCE`] whose every page must be in the summary.
const COVERED: [&str; 2] = ["physics", "format"];
/// What `mdbook --version` prints for the release series `book.toml` is written for.
const MDBOOK_SERIES: &str = "mdbook v0.5.";
/// How to install the mdBook release CI uses.
const MDBOOK_INSTALL: &str = "install mdBook 0.5.4: `cargo install mdbook --version 0.5.4 --locked` or `brew install mdbook`";

pub fn run(args: &[String]) -> Result<(), String> {
    let build = match args {
        [] => true,
        [flag] if flag == "--no-build" => false,
        _ => return Err(format!("unexpected arguments {args:?}\n\n{USAGE}")),
    };
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
    let (files, problems) = check_html(&output)?;
    if !problems.is_empty() {
        return Err(failure("the built site", &problems));
    }
    println!("built site: {files} HTML files in {OUTPUT}, every relative link resolves");
    Ok(())
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
    let output = Command::new("mdbook")
        .arg("build")
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
                pages.insert(name.clone(), read_page(&text));
            }
            Err(err) => report.problems.push(format!(
                "{SOURCE}/{name}: listed in {SOURCE}/{SUMMARY}, but can't be read: {err}"
            )),
        }
    }

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
/// `others` caches the other Markdown files a GitHub URL points into.
fn check_link(
    root: &Path,
    pages: &BTreeMap<String, Page>,
    others: &mut BTreeMap<String, Page>,
    from: &str,
    dest: &str,
) -> Result<Reach, String> {
    if dest.is_empty() {
        return Err("empty destination".to_owned());
    }
    if let Some(rest) = dest.strip_prefix(GITHUB) {
        // Issues, pull requests and the like are on the web; files on `main` are checked here.
        let Some(target) = rest
            .strip_prefix("blob/main/")
            .or_else(|| rest.strip_prefix("tree/main/"))
        else {
            return Ok(Reach::Web);
        };
        let (path, fragment) = split_fragment(target);
        let path = path.trim_end_matches('/');
        if path.is_empty() || join("", path).as_deref() != Some(path) || !root.join(path).exists() {
            return Err(format!("no `{path}` in the repository"));
        }
        if let Some(fragment) = fragment
            && path.ends_with(".md")
        {
            let anchors = match path
                .strip_prefix(SOURCE)
                .and_then(|name| name.strip_prefix('/'))
                .and_then(|name| pages.get(name))
            {
                Some(page) => &page.anchors,
                None => {
                    if !others.contains_key(path) {
                        let text = fs::read_to_string(root.join(path))
                            .map_err(|err| format!("can't read `{path}`: {err}"))?;
                        others.insert(path.to_owned(), read_page(&text));
                    }
                    &others[path].anchors
                }
            };
            return has_anchor(anchors, fragment, path).map(|()| Reach::Repository);
        }
        return Ok(Reach::Repository);
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
    let (path, fragment) = split_fragment(dest);
    let name = if path.is_empty() {
        from.to_owned()
    } else {
        join(parent(from), path).ok_or_else(|| {
            format!(
                "leaves `{SOURCE}/`, which the site can't follow: link it by its GitHub URL, \
                 {GITHUB}blob/main/..."
            )
        })?
    };
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

/// Whether `anchors` (of the file `what`) has `fragment`, which may be percent-encoded.
fn has_anchor(anchors: &BTreeSet<String>, fragment: &str, what: &str) -> Result<(), String> {
    if anchors.contains(fragment) || anchors.contains(&percent_decode(fragment)) {
        Ok(())
    } else {
        Err(format!(
            "`{what}` has no heading with the anchor `#{fragment}`"
        ))
    }
}

/// A link to another site: it names a scheme, or is protocol-relative.
fn is_web(dest: &str) -> bool {
    dest.contains("://") || dest.starts_with("//") || dest.starts_with("mailto:")
}

/// Splits `path#fragment`; an empty fragment is none.
fn split_fragment(dest: &str) -> (&str, Option<&str>) {
    match dest.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment).filter(|f| !f.is_empty())),
        None => (dest, None),
    }
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
    /// The anchors of its headings, as GitHub derives them.
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

fn read_page(text: &str) -> Page {
    let line = |offset: usize| {
        1 + text.as_bytes()[..offset.min(text.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
    };
    let mut page = Page::default();
    let mut undefined = Vec::new();
    let mut headings = Vec::new();
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
        let mut in_link = 0usize;
        let mut in_code_block = false;
        let mut heading: Option<String> = None;
        // Consecutive text outside links is scanned as one, however the parser splits it.
        let mut prose = String::new();
        let mut prose_at = 0;
        for (event, range) in parser.into_offset_iter() {
            let is_prose = matches!(event, Event::Text(_)) && in_link == 0 && !in_code_block;
            if !is_prose && !prose.is_empty() {
                scan(&prose, line(prose_at), &mut page.problems);
                prose.clear();
            }
            match event {
                Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                    page.links.push((line(range.start), dest_url.into_string()));
                    in_link += 1;
                }
                Event::End(TagEnd::Link | TagEnd::Image) => in_link = in_link.saturating_sub(1),
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

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for heading in headings {
        let base = slug(&heading);
        let count = seen.entry(base.clone()).or_insert(0);
        page.anchors.insert(if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        });
        *count += 1;
    }
    page
}

/// Scans prose (text outside links and code) for bare labels and for math only GitHub renders.
fn scan(text: &str, line: usize, problems: &mut Vec<(usize, String)>) {
    scan_labels(text, line, problems);
    if text.contains("$$") || text.contains("$\\") {
        problems.push((line, MATH.to_owned()));
    }
}

fn scan_labels(text: &str, line: usize, problems: &mut Vec<(usize, String)>) {
    for label in labels(text) {
        problems.push((
            line,
            format!(
                "bare label `{label}`: make it a link, with a few words saying what it is \
                 (ADR-016). A motor designation is written in full, such as `L1150R`"
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
    } else {
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
    };
    (!bytes.get(end).is_some_and(u8::is_ascii_alphanumeric)).then_some(end)
}

/// The anchor GitHub gives a heading with this text: lower case; letters, digits, `-` and `_`
/// kept; each space a `-`; everything else dropped. A repeated heading's anchor is numbered
/// (`-1`, `-2`) by [`read_page`]. mdBook derives the same ids for this site's headings, which the
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

/// Checks the built site in `dir`: every relative `href` in its HTML reaches a file, and its
/// `#fragment` an `id` in that file. Returns the number of HTML files and the problems.
fn check_html(dir: &Path) -> Result<(usize, Vec<String>), String> {
    let mut names = Vec::new();
    html_files(dir, "", &mut names)?;
    if names.is_empty() {
        return Err(format!("no HTML files in {}", dir.display()));
    }
    let mut texts = BTreeMap::new();
    let mut ids: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in &names {
        let text = fs::read_to_string(dir.join(name))
            .map_err(|err| format!("could not read {}: {err}", dir.join(name).display()))?;
        ids.insert(
            name.clone(),
            attributes(&text, "id")
                .map(|id| percent_decode(&unescape(id)))
                .collect(),
        );
        texts.insert(name.clone(), text);
    }
    let mut problems = Vec::new();
    for (name, text) in &texts {
        for raw in attributes(text, "href") {
            let href = unescape(raw);
            if is_web(&href) || href.starts_with("javascript:") || href.starts_with("data:") {
                continue;
            }
            let (path, fragment) = split_fragment(&href);
            let target = if path.is_empty() {
                name.clone()
            } else {
                let Some(target) = join(parent(name), &percent_decode(path)) else {
                    problems.push(format!("{name}: `href=\"{href}\"` climbs out of the site"));
                    continue;
                };
                if target.is_empty() || target.ends_with('/') || dir.join(&target).is_dir() {
                    format!("{}/index.html", target.trim_end_matches('/'))
                        .trim_start_matches('/')
                        .to_owned()
                } else {
                    target
                }
            };
            let Some(target_ids) = ids.get(&target) else {
                if !dir.join(&target).is_file() {
                    problems.push(format!(
                        "{name}: `href=\"{href}\"`: no `{target}` in the site"
                    ));
                }
                continue;
            };
            if let Some(fragment) = fragment
                && !target_ids.contains(&percent_decode(fragment))
            {
                problems.push(format!(
                    "{name}: `href=\"{href}\"`: `{target}` has no `id=\"{fragment}\"`"
                ));
            }
        }
    }
    Ok((names.len(), problems))
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
                    "# Gravity\n\n## Formulas\n\nBack to [the start](../start-here.md#start-here), \
                     [here](#formulas), [NGA] as a citation, and [the crate](https://github.com/nrdptel/hpr-sim/tree/main/crates/).\n",
                ),
                ("docs/DECISIONS.md", "# Decisions\n\n## ADR-003: Frames (2026-09-17)\n"),
                ("docs/research/loft-lessons.md", "# Lessons\n"),
                ("crates/README.md", "crates\n"),
            ])
            .path(),
        )
        .unwrap();
        assert_eq!(report.problems, Vec::<String>::new());
        // The summary's two links count too.
        assert_eq!((report.pages, report.links, report.web), (2, 9, 1));
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
                    [j](physics)\n";
        assert_eq!(
            problems(&[
                ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
                ("docs/start-here.md", page),
                ("docs/physics/gravity.md", "# Gravity\n"),
                ("docs/notes.md", "# Notes\n"),
                ("docs/DECISIONS.md", "# Decisions\n\n## ADR-001: One\n"),
                ("ROADMAP.md", "# Roadmap\n"),
            ]),
            [
                "docs/start-here.md:10: `[undefined]` is used as a link, and no `[undefined]: ...` \
                 defines it",
                "docs/start-here.md:3: link `missing.md`: no page `docs/missing.md`",
                "docs/start-here.md:4: link `physics/gravity.md#nowhere`: \
                 `docs/physics/gravity.md` has no heading with the anchor `#nowhere`",
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
                 `docs/DECISIONS.md` has no heading with the anchor `#adr-999`",
                "docs/start-here.md:11: link `#nowhere`: `docs/start-here.md` has no heading \
                 with the anchor `#nowhere`",
                "docs/start-here.md:12: link `physics`: `docs/physics` is a directory, which has \
                 no page on the site",
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
                    [d]: https://example.com\n";
        let found = problems(&[
            ("docs/SUMMARY.md", "[Start here](start-here.md)\n"),
            ("docs/start-here.md", page),
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
            labels("L1150R M1297 Mach 1.8 ADR- XL10 L10x M1 M1.x ADR-7b L 1 M.1 ML1"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn math_that_only_github_renders_fails() {
        let page = "# Start here\n\n$$x = 1$$\n\nInline $\\alpha$ too.\n\n```math\nx\n```\n\n\
                    Unicode `γ = GM/r²` and a price of $5 are fine.\n";
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
                "docs/start-here.md:7"
            ]
        );
    }

    #[test]
    fn every_model_and_format_page_is_in_the_summary() {
        assert_eq!(
            problems(&[
                ("docs/SUMMARY.md", SUMMARY_TWO_PAGES),
                ("docs/start-here.md", "# Start here\n"),
                ("docs/physics/gravity.md", "# Gravity\n"),
                ("docs/physics/wind.md", "# Wind\n"),
                ("docs/format/eng.md", "# eng\n"),
            ]),
            [
                "docs/physics/wind.md: not in docs/SUMMARY.md, so the site doesn't show it",
                "docs/format/eng.md: not in docs/SUMMARY.md, so the site doesn't show it",
            ]
        );
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
    fn broken_links_in_the_built_site_are_reported() {
        let dir = workspace(&[
            (
                "index.html",
                "<a href=\"physics/aero.html#lift\">ok</a> <a href=\"physics/aero.html#drag\">x</a>\n\
                 <a href=\"physics/wind.html\">x</a> <a href=\"../outside.html\">x</a>\n\
                 <a href=\"https://example.com/nowhere.html\">web</a> <a href=\"#top\">ok</a>\n\
                 <a href=\"physics/\">ok</a> <a href=\"physics/aero.html#%C2%B5-term\">ok</a>\n\
                 <link rel=\"stylesheet\" href=\"css/site.css\"><h1 id=\"top\">t</h1>",
            ),
            (
                "physics/aero.html",
                "<h2 id=\"lift\">Lift</h2><h2 id=\"µ-term\">µ</h2><a href=\"../index.html#top\">ok</a>",
            ),
            ("physics/index.html", "<p>index</p>"),
            ("css/site.css", "body {}"),
        ]);
        let (files, problems) = check_html(dir.path()).unwrap();
        assert_eq!(files, 3);
        assert_eq!(
            problems,
            [
                "index.html: `href=\"physics/aero.html#drag\"`: `physics/aero.html` has no \
                 `id=\"drag\"`",
                "index.html: `href=\"physics/wind.html\"`: no `physics/wind.html` in the site",
                "index.html: `href=\"../outside.html\"` climbs out of the site",
            ]
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
