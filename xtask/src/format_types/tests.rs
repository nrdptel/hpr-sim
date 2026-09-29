//! The generated types are current, and their readers read every public document and agree with
//! the schema on mutations of one. The readers run under Node.js and Python, which must be on the
//! path: Node.js 22.18 or later, Python 3.11 or later.

use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    crate::designs::root().unwrap()
}

#[test]
fn the_committed_types_are_the_generated_ones() {
    let root = root();
    for (path, text) in crate::format::outputs().unwrap() {
        let committed = std::fs::read_to_string(root.join(&path)).unwrap_or_default();
        assert!(
            committed == text,
            "{path} is stale: run `cargo xtask format`"
        );
    }
}

/// The generator refuses a schema with a keyword it doesn't check, rather than letting the
/// readers pass what the keyword constrains.
#[test]
fn a_keyword_the_readers_do_not_check_is_refused() {
    let mut schema = hpr_format::schema();
    schema["$defs"]["Version"]["maxLength"] = serde_json::json!(8);
    for generated in [typescript(&schema), python(&schema)] {
        let error = generated.unwrap_err();
        assert!(error.contains("`maxLength`"), "{error}");
    }
    // Each change to the committed schema, and what the refusal names.
    let refused: [(&str, Value, &str); 9] = [
        (
            "/$defs/Version/format",
            serde_json::json!("date"),
            "format `date`",
        ),
        (
            "/$defs/Motors/properties/configurations/items",
            serde_json::json!(true),
            "a schema that is `true`",
        ),
        (
            "/$defs/Motors/properties/configurations/items/maxItems",
            serde_json::json!(3),
            "`maxItems` beside `$ref`",
        ),
        (
            "/$defs/Format/type",
            serde_json::json!("string"),
            "`type` beside `oneOf`",
        ),
        (
            "/$defs/Version/pattern",
            serde_json::json!("^\\d+$"),
            "reads differently",
        ),
        (
            "/$defs/Version/pattern",
            serde_json::json!("^a.b$"),
            "reads differently",
        ),
        (
            "/$defs/Version/pattern",
            serde_json::json!("^a$|^b$"),
            "reads differently",
        ),
        (
            "/$defs/Version/pattern",
            serde_json::json!("^a\\$"),
            "reads differently",
        ),
        (
            "/$defs/ValueError",
            serde_json::json!({"type": "string"}),
            "ValueError",
        ),
    ];
    for (pointer, value, says) in refused {
        let mut schema = hpr_format::schema();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        schema.pointer_mut(parent).unwrap()[key] = value;
        let error = python(&schema).unwrap_err();
        assert!(error.contains(says), "{pointer}: {error}");
    }
    // A definition named as the TypeScript reader's own code.
    let mut schema = hpr_format::schema();
    schema["$defs"]["SchemaNode"] = serde_json::json!({"type": "string"});
    assert!(typescript(&schema).unwrap_err().contains("SchemaNode"));
}

/// Links to Rust items become the code they show; web and reference links, and text that only
/// looks like a link, stay as written.
#[test]
fn rust_links_become_code() {
    let cases = [
        ("see [`Self::pods`].", "see `Self::pods`."),
        ("see [`pods`](crate::PodSet::pods) here", "see `pods` here"),
        (
            "[`web`](https://example.com)",
            "[`web`](https://example.com)",
        ),
        ("[`adr`][adr-074]", "[`adr`][adr-074]"),
        ("a [`]` b", "a [`]` b"),
        (
            "see [`x`](crate::Y and more text",
            "see [`x`](crate::Y and more text",
        ),
        ("[`a` and `b`]", "[`a` and `b`]"),
    ];
    for (text, plain) in cases {
        assert_eq!(without_rust_links(text), plain, "{text}");
    }
}

/// A command running the first of `candidates` that is new enough, as `probe` tells by its exit
/// status, or a failure saying what to install.
fn interpreter(candidates: &[&str], probe: &[&str], needs: &str) -> Command {
    for program in candidates {
        let works = Command::new(program)
            .args(probe)
            .output()
            .is_ok_and(|o| o.status.success());
        if works {
            return Command::new(program);
        }
    }
    panic!(
        "none of {candidates:?} runs, or is new enough: the generated types' readers (M3.3c) \
         are tested under {needs}, which must be on the path"
    );
}

/// Succeeds under a Node.js that runs TypeScript as it is: 22.18 or later, 23.6 or later.
const NODE_PROBE: &[&str] = &[
    "-e",
    "const [a, b] = process.versions.node.split('.').map(Number); \
     process.exit(a > 23 || (a === 23 && b >= 6) || (a === 22 && b >= 18) ? 0 : 1)",
];

/// Succeeds under Python 3.11 or later, which has `typing.NotRequired`.
const PYTHON_PROBE: &[&str] = &["-c", "import sys; sys.exit(sys.version_info < (3, 11))"];

/// What the TypeScript and the Python example print for the documents `names` in `dir`, one
/// line a document each.
fn read_all(dir: &Path, names: &[String]) -> (Vec<String>, Vec<String>) {
    let root = root();
    let (mut ts, mut py) = (Vec::new(), Vec::new());
    // A few hundred names a run keeps a command line well inside Windows' limit.
    for chunk in names.chunks(400) {
        let ts_out = interpreter(&["node"], NODE_PROBE, "Node.js 22.18 or later")
            .arg(root.join("schema/format/typescript/read-design.ts"))
            .args(chunk)
            .current_dir(dir)
            .output()
            .unwrap();
        let py_out = interpreter(&["python3", "python"], PYTHON_PROBE, "Python 3.11 or later")
            .arg(root.join("schema/format/python/read_design.py"))
            .args(chunk)
            .current_dir(dir)
            .output()
            .unwrap();
        for (out, lines) in [(ts_out, &mut ts), (py_out, &mut py)] {
            let text = String::from_utf8_lossy(&out.stdout);
            let got: Vec<String> = text.lines().map(str::to_owned).collect();
            assert_eq!(
                got.len(),
                chunk.len(),
                "a reader didn't print a line a document:\n{text}\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
            lines.extend(got);
        }
    }
    (ts, py)
}

/// How many parts `components` hold, counting the parts inside parts.
fn parts(components: &Value) -> usize {
    components
        .as_array()
        .into_iter()
        .flatten()
        .map(|c| 1 + c.get("children").map_or(0, parts))
        .sum()
}

#[test]
fn both_readers_read_every_public_document() {
    let documents = public_documents(&root()).unwrap();
    assert_eq!(
        documents.len(),
        18,
        "17 public designs and the migrated 0.1 document"
    );
    let dir = tempfile::tempdir().unwrap();
    let mut names = Vec::new();
    let mut expected = Vec::new();
    for (stem, text) in &documents {
        let name = format!("{stem}.hpr");
        std::fs::write(dir.path().join(&name), text).unwrap();
        let value: Value = serde_json::from_str(text).unwrap();
        let stages = value["rocket"]["stages"].as_array().unwrap();
        let count: usize = stages.iter().map(|s| parts(&s["components"])).sum();
        expected.push(format!(
            "read {name}: stages {}, parts {count}, motor configurations {}",
            stages.len(),
            value["motors"]["configurations"].as_array().unwrap().len()
        ));
        names.push(name);
    }
    let (ts, py) = read_all(dir.path(), &names);
    assert_eq!(ts, expected, "the TypeScript reader");
    assert_eq!(py, expected, "the Python reader");
    // Each language's JSON writer, given what its reader read, writes a document hpr reads as
    // the same design, though it may spell numbers differently (`1850` for `1850.0`).
    let rewrite = |mut program: Command, script: &str| {
        let out = program
            .args([
                if script.contains("require") {
                    "-e"
                } else {
                    "-c"
                },
                script,
            ])
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    rewrite(
        interpreter(&["node"], NODE_PROBE, "Node.js 22.18 or later"),
        "const fs = require('node:fs'); \
         for (const f of fs.readdirSync('.').filter((f) => f.endsWith('.hpr'))) \
         fs.writeFileSync(f + '.js.json', \
         JSON.stringify(JSON.parse(fs.readFileSync(f, 'utf8')), null, 2));",
    );
    rewrite(
        interpreter(&["python3", "python"], PYTHON_PROBE, "Python 3.11 or later"),
        "import json, pathlib\n\
         for f in pathlib.Path('.').glob('*.hpr'):\n    \
         pathlib.Path(f'{f}.py.json').write_text(\
         json.dumps(json.loads(f.read_text('utf-8')), indent=2), 'utf-8')",
    );
    let mut respelt = 0;
    for name in &names {
        let text = std::fs::read_to_string(dir.path().join(name)).unwrap();
        let original = hpr_format::read_json(&text).unwrap().value;
        for writer in ["js", "py"] {
            let path = dir.path().join(format!("{name}.{writer}.json"));
            let written = std::fs::read_to_string(path).unwrap();
            let read = hpr_format::read_json(&written).unwrap().value;
            assert!(
                read == original,
                "{name} written by {writer} reads as another design"
            );
            respelt += usize::from(written != text);
        }
    }
    assert!(
        respelt > 0,
        "no writer spelt a number differently: the page's example is wrong"
    );
    // The line the format's page shows, for the design it converts.
    let shown =
        std::fs::read_to_string(root().join("schema/format/read-design.output.txt")).unwrap();
    assert!(
        expected.contains(&shown.trim_end().to_owned()),
        "schema/format/read-design.output.txt is not what the readers print: {expected:#?}"
    );
}

/// Every value in `value`, by its path of keys and indices.
fn places(value: &Value, path: &mut Vec<Step>, out: &mut Vec<Vec<Step>>) {
    out.push(path.clone());
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                path.push(Step::Key(key.clone()));
                places(child, path, out);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (i, child) in items.iter().enumerate() {
                path.push(Step::Index(i));
                places(child, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}

/// One step into a JSON value.
#[derive(Clone, Debug)]
enum Step {
    Key(String),
    Index(usize),
}

fn at<'a>(value: &'a mut Value, path: &[Step]) -> &'a mut Value {
    path.iter().fold(value, |v, step| match step {
        Step::Key(k) => &mut v[k.as_str()],
        Step::Index(i) => &mut v[*i],
    })
}

/// Mutations of `document`: at every value, a key added to an object, a key taken out of its
/// parent, the value swapped for others of each JSON type and for edge values, a string given a
/// final newline, and an array a copy of its last item or without its first. The root's
/// `format` and `version`, which the readers check before the schema, are left alone.
fn mutations(document: &Value) -> Vec<Value> {
    let mut paths = Vec::new();
    places(document, &mut Vec::new(), &mut paths);
    let mut out = Vec::new();
    for path in paths {
        if matches!(path.first(), Some(Step::Key(k)) if k == "format" || k == "version") {
            continue;
        }
        let original = {
            let mut copy = document.clone();
            at(&mut copy, &path).clone()
        };
        let mut replacements = vec![
            serde_json::json!(null),
            serde_json::json!(true),
            serde_json::json!(-1),
            serde_json::json!(0.5),
            serde_json::json!("not-a-tag"),
            serde_json::json!([]),
            serde_json::json!({}),
        ];
        match &original {
            Value::Object(map) => {
                let mut more = map.clone();
                more.insert("unknown_key".to_owned(), serde_json::json!(0));
                replacements.push(Value::Object(more));
            }
            // A final newline, which Python's `$` would let through a pattern.
            Value::String(text) => replacements.push(Value::String(format!("{text}\n"))),
            Value::Array(items) if !items.is_empty() => {
                let mut more = items.clone();
                more.push(items[items.len() - 1].clone());
                replacements.push(Value::Array(more));
                replacements.push(Value::Array(items[1..].to_vec()));
            }
            _ => {}
        }
        for replacement in replacements {
            if replacement == original {
                continue;
            }
            let mut mutated = document.clone();
            *at(&mut mutated, &path) = replacement;
            out.push(mutated);
        }
        if let Some((Step::Key(key), parent)) = path.split_last() {
            let mut mutated = document.clone();
            if let Value::Object(map) = at(&mut mutated, parent) {
                map.remove(key);
            }
            out.push(mutated);
        }
    }
    out
}

/// On every mutation of two public designs, each reader takes the document exactly when the
/// committed schema does, as the `jsonschema` crate reads it: the readers check what the schema
/// says, no more and no less. (The crate leaves `format` unchecked, as JSON Schema allows; the
/// readers' limits for `uint32` and `uint` are tested on their own, against hpr.)
#[test]
fn both_readers_agree_with_the_schema_on_mutations() {
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("schema/format/hpr-design-0.2.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let documents = public_documents(&root()).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (mut names, mut valid) = (Vec::new(), Vec::new());
    for stem in ["demo-dual-deploy", "pods-motors-2"] {
        let text = &documents.iter().find(|(s, _)| s == stem).unwrap().1;
        let document: Value = serde_json::from_str(text).unwrap();
        for mutated in mutations(&document) {
            let name = format!("m{}.hpr", names.len());
            std::fs::write(dir.path().join(&name), mutated.to_string()).unwrap();
            valid.push(validator.is_valid(&mutated));
            names.push(name);
        }
    }
    let (ts, py) = read_all(dir.path(), &names);
    let refused = valid.iter().filter(|v| !**v).count();
    // The format's page, ADR-113 and the roadmap quote these counts.
    assert_eq!(
        (names.len(), refused),
        (4892, 4114),
        "mutations, and those the schema refuses: update the docs that quote them"
    );
    for (reader, lines) in [("TypeScript", &ts), ("Python", &py)] {
        let disagree: Vec<String> = names
            .iter()
            .zip(&valid)
            .zip(lines)
            .filter(|((_, valid), line)| **valid != line.starts_with("read "))
            .map(|((name, valid), line)| format!("{name} (schema: valid {valid}): {line}"))
            .collect();
        assert!(
            disagree.is_empty(),
            "the {reader} reader disagrees with the schema on {} of {} mutations, first:\n{}",
            disagree.len(),
            names.len(),
            disagree
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// `good` with the first value under the key `key` set to `value`.
fn with(good: &str, key: &str, value: Value) -> String {
    let mut document: Value = serde_json::from_str(good).unwrap();
    let mut paths = Vec::new();
    places(&document, &mut Vec::new(), &mut paths);
    let path = paths
        .iter()
        .find(|p| matches!(p.last(), Some(Step::Key(k)) if k == key))
        .unwrap_or_else(|| panic!("no {key}"));
    *at(&mut document, path) = value;
    document.to_string()
}

/// `good` with a key `x` at its root holding arrays nested so the document is `levels` deep.
fn nested(good: &str, levels: usize) -> String {
    let inner = format!("{}{}", "[".repeat(levels - 1), "]".repeat(levels - 1));
    good.replacen('{', &format!("{{\"x\": {inner},"), 1)
}

/// What each reader makes of documents at the edges: JSON's corners, which hpr refuses before
/// the schema is asked (numbers too large, lone surrogates, nesting, repeated keys, `NaN`, `2.0`
/// for a whole number), another version or format, the limits of `uint32` and `uint`, and the
/// message for a misspelt tag. Each case is held to hpr's own reader where the page says they
/// agree.
#[test]
fn both_readers_at_the_edges() {
    let documents = public_documents(&root()).unwrap();
    let good = &documents[0].1;
    let most_u32 = serde_json::json!(4_294_967_295_u64);
    let past_u32 = serde_json::json!(4_294_967_296_u64);
    let safe = serde_json::json!(9_007_199_254_740_991_u64);
    let unsafe_ = serde_json::json!(9_007_199_254_740_992_u64);
    // (file, text, what the TypeScript reader prints, what the Python one does, whether hpr
    // refuses it; `None` where hpr's verdict turns on its checks beyond the schema).
    let cases: Vec<(&str, String, &str, &str, Option<bool>)> = vec![
        (
            "bom.hpr",
            format!("\u{feff}{good}"),
            "read",
            "read",
            Some(false),
        ),
        (
            "old.hpr",
            good.replacen("\"version\": \"0.2\"", "\"version\": \"0.1\"", 1),
            "refused old.hpr: written in version 0.1; these types read 0.2 only",
            "refused old.hpr: written in version 0.1; these types read 0.2 only",
            None,
        ),
        (
            "new.hpr",
            good.replacen("\"version\": \"0.2\"", "\"version\": \"0.10\"", 1),
            "refused new.hpr: written in version 0.10, newer than these types",
            "refused new.hpr: written in version 0.10, newer than these types",
            Some(true),
        ),
        (
            "other.hpr",
            good.replacen("\"hpr-design\"", "\"hpr-other\"", 1),
            "refused other.hpr: not an hpr design",
            "refused other.hpr: not an hpr design",
            Some(true),
        ),
        (
            "text.hpr",
            "not json".to_owned(),
            "refused text.hpr: not JSON",
            "refused text.hpr: not JSON",
            Some(true),
        ),
        (
            "nan.hpr",
            good.replacen("\"length_m\": ", "\"length_m\": NaN, \"x\": ", 1),
            "refused nan.hpr: not JSON",
            "refused nan.hpr: not JSON",
            Some(true),
        ),
        (
            "huge.hpr",
            good.replacen("\"length_m\": ", "\"length_m\": 1e400, \"x\": ", 1),
            "refused huge.hpr: not JSON: a number too large for a 64-bit float",
            "refused huge.hpr: not JSON: a number too large for a 64-bit float",
            Some(true),
        ),
        (
            "huge-integer.hpr",
            good.replacen(
                "\"length_m\": ",
                &format!("\"length_m\": 1{}, \"x\": ", "0".repeat(400)),
                1,
            ),
            "refused huge-integer.hpr: not JSON: a number too large for a 64-bit float",
            "refused huge-integer.hpr: not JSON: a number too large for a 64-bit float",
            Some(true),
        ),
        (
            "surrogate.hpr",
            good.replacen(
                "\"format\": \"hpr-design\",",
                "\"format\": \"hpr-design\", \"x\": \"\\ud800\",",
                1,
            ),
            "refused surrogate.hpr: not JSON: a lone UTF-16 surrogate",
            "refused surrogate.hpr: not JSON: a lone UTF-16 surrogate",
            Some(true),
        ),
        // 127 levels is the most hpr reads: the reader gets to the schema, which refuses `x`.
        (
            "deep-127.hpr",
            nested(good, 127),
            "refused deep-127.hpr: $: has the unknown key \"x\"",
            "refused deep-127.hpr: $: has the unknown key \"x\"",
            Some(true),
        ),
        (
            "deep-128.hpr",
            nested(good, 128),
            "refused deep-128.hpr: not JSON: nested more than 127 levels deep",
            "refused deep-128.hpr: not JSON: nested more than 127 levels deep",
            Some(true),
        ),
        (
            "deep-5000.hpr",
            nested(good, 5000),
            "refused deep-5000.hpr: not JSON",
            "refused deep-5000.hpr: not JSON",
            Some(true),
        ),
        (
            "repeated.hpr",
            good.replacen(
                "\"rocket\": {",
                "\"rocket\": {\"name\": \"a\", \"name\": \"b\",",
                1,
            ),
            // `JSON.parse` keeps a repeated key's last value, as the TypeScript reader says.
            "read",
            "refused repeated.hpr: not JSON: the key \"name\" appears twice",
            Some(true),
        ),
        (
            "float.hpr",
            with(good, "count", serde_json::json!(2.0)),
            // `JSON.parse` reads `2.0` as `2`.
            "read",
            "refused float.hpr: $.rocket",
            Some(true),
        ),
        (
            "u32-most.hpr",
            with(good, "count", most_u32),
            "read",
            "read",
            None,
        ),
        // The format's page shows this message.
        (
            "missing.hpr",
            good.replacen("\"length_m\"", "\"lenght_m\"", 1),
            "refused missing.hpr: $.rocket.stages[0].components[0].part.nose_cone: has no \"length_m\", \
             which it needs",
            "refused missing.hpr: $.rocket.stages[0].components[0].part.nose_cone: has no \"length_m\", \
             which it needs",
            Some(true),
        ),
        (
            "exponent.hpr",
            with(good, "count", serde_json::json!(123_456_789)).replacen("123456789", "3e0", 1),
            // `JSON.parse` reads `3e0` as `3`.
            "read",
            "refused exponent.hpr: $.rocket",
            Some(true),
        ),
        (
            "u32-past.hpr",
            with(good, "count", past_u32),
            "refused u32-past.hpr: $.rocket",
            "refused u32-past.hpr: $.rocket",
            Some(true),
        ),
        (
            "uint-safe.hpr",
            with(good, "stage", safe),
            "read",
            "read",
            Some(false),
        ),
        (
            "uint-unsafe.hpr",
            with(good, "stage", unsafe_),
            // JavaScript can't hold 2^53 apart from 2^53 + 1.
            "refused uint-unsafe.hpr: $.",
            "read",
            Some(false),
        ),
        (
            "tag.hpr",
            good.replacen("\"from\": \"top\"", "\"from\": \"topp\"", 1),
            "refused tag.hpr: $.rocket.stages[0].components[1].children[0].position.from: is \"topp\", \
             not \"top\", \"middle\", \"bottom\" or \"after\"",
            "refused tag.hpr: $.rocket.stages[0].components[1].children[0].position.from: is \"topp\", \
             not \"top\", \"middle\", \"bottom\" or \"after\"",
            None,
        ),
    ];
    let names: Vec<String> = cases.iter().map(|c| c.0.to_owned()).collect();
    let dir = tempfile::tempdir().unwrap();
    for (name, text, ..) in &cases {
        assert!(text != good, "{name} changed nothing");
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let (ts, py) = read_all(dir.path(), &names);
    for (i, (name, text, ts_start, py_start, hpr_refuses)) in cases.iter().enumerate() {
        let expected = |start: &str| {
            if start == "read" {
                format!("read {name}")
            } else {
                start.to_owned()
            }
        };
        let (ts_start, py_start) = (expected(ts_start), expected(py_start));
        assert!(
            ts[i].starts_with(&ts_start),
            "TypeScript, {name}: {}",
            ts[i]
        );
        assert!(py[i].starts_with(&py_start), "Python, {name}: {}", py[i]);
        if let Some(refuses) = hpr_refuses {
            let hpr = hpr_format::read_json(text);
            assert_eq!(hpr.is_err(), *refuses, "hpr, {name}: {:?}", hpr.err());
        }
    }
    // hpr's reader stops at the same depth, for the same reason.
    let case = |name: &str| &cases.iter().find(|c| c.0 == name).unwrap().1;
    let deep = hpr_format::read_json(case("deep-128.hpr"))
        .unwrap_err()
        .to_string();
    assert!(deep.contains("recursion limit"), "{deep}");
    let shallow = hpr_format::read_json(case("deep-127.hpr"))
        .unwrap_err()
        .to_string();
    assert!(!shallow.contains("recursion limit"), "{shallow}");
}
