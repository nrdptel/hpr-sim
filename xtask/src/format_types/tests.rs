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
    let mut schema = hpr_format::schema();
    schema["$defs"]["Version"]["format"] = serde_json::json!("date");
    assert!(typescript(&schema).unwrap_err().contains("format `date`"));
}

/// A command running `program`, after checking it runs at all, with what to install if not.
fn interpreter(candidates: &[&str], needs: &str) -> Command {
    for program in candidates {
        let works = Command::new(program)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());
        if works {
            return Command::new(program);
        }
    }
    panic!(
        "none of {candidates:?} runs: the generated types' readers (M3.3c) are tested under \
         {needs}, which must be on the path"
    );
}

/// What the TypeScript and the Python example print for the documents `names` in `dir`, one
/// line a document each.
fn read_all(dir: &Path, names: &[String]) -> (Vec<String>, Vec<String>) {
    let root = root();
    let (mut ts, mut py) = (Vec::new(), Vec::new());
    // A few hundred names a run keeps a command line well inside Windows' limit.
    for chunk in names.chunks(400) {
        let ts_out = interpreter(&["node"], "Node.js 22.18 or later")
            .arg(root.join("schema/format/typescript/read-design.ts"))
            .args(chunk)
            .current_dir(dir)
            .output()
            .unwrap();
        let py_out = interpreter(&["python3", "python"], "Python 3.11 or later")
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
/// parent, and the value swapped for others of each JSON type and for edge values. The root's
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
/// says, no more and no less.
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
    assert!(
        names.len() > 2000 && refused > 1000 && refused < names.len() - 200,
        "{} mutations, {refused} invalid: too few of either kind to test anything",
        names.len()
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

/// What each reader makes of documents the schema can't speak for: a byte-order mark, another
/// version or format, not JSON, a repeated key, `NaN`, and `2.0` for a whole number.
#[test]
fn both_readers_refuse_what_hpr_refuses_before_the_schema() {
    let documents = public_documents(&root()).unwrap();
    let good = &documents[0].1;
    let dir = tempfile::tempdir().unwrap();
    let mut cases: Vec<(&str, String)> = vec![
        ("bom.hpr", format!("\u{feff}{good}")),
        (
            "old.hpr",
            good.replacen("\"version\": \"0.2\"", "\"version\": \"0.1\"", 1),
        ),
        (
            "other.hpr",
            good.replacen("\"hpr-design\"", "\"hpr-other\"", 1),
        ),
        ("text.hpr", "not json".to_owned()),
        (
            "repeated.hpr",
            good.replacen(
                "\"rocket\": {",
                "\"rocket\": {\"name\": \"a\", \"name\": \"b\",",
                1,
            ),
        ),
        (
            "nan.hpr",
            good.replacen("\"length_m\": ", "\"length_m\": NaN, \"x\": ", 1),
        ),
    ];
    // A fin count, a whole number, written `2.0`.
    let mut whole: Value = serde_json::from_str(good).unwrap();
    let mut paths = Vec::new();
    places(&whole, &mut Vec::new(), &mut paths);
    let count = paths
        .iter()
        .find(|p| matches!(p.last(), Some(Step::Key(k)) if k == "count"))
        .unwrap();
    *at(&mut whole, count) = serde_json::json!(2.0);
    cases.push(("float.hpr", whole.to_string()));
    assert!(cases[6].1.contains("\"count\":2.0"));
    let names: Vec<String> = cases.iter().map(|(n, _)| (*n).to_owned()).collect();
    for (name, text) in &cases {
        assert!(text != good, "{name} changed nothing");
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    let (ts, py) = read_all(dir.path(), &names);
    let expected = |lines: &[String], i: usize, start: &str| {
        assert!(lines[i].starts_with(start), "{}: {}", names[i], lines[i]);
    };
    for lines in [&ts, &py] {
        expected(lines, 0, "read bom.hpr");
        expected(
            lines,
            1,
            "refused old.hpr: written in version \"0.1\"; these types read 0.2",
        );
        expected(lines, 2, "refused other.hpr: not an hpr design");
        expected(lines, 3, "refused text.hpr: not JSON");
        expected(lines, 5, "refused nan.hpr: not JSON");
    }
    // `JSON.parse` keeps a repeated key's last value, as the TypeScript reader's doc says; hpr
    // and the Python reader refuse it.
    expected(&ts, 4, "read repeated.hpr");
    expected(
        &py,
        4,
        "refused repeated.hpr: not JSON: the key \"name\" appears twice",
    );
    let hpr = hpr_format::read_json(&cases[4].1);
    assert!(hpr.is_err(), "hpr reads a repeated key");
    // `JSON.parse` reads `2.0` as `2`; hpr and the Python reader refuse it.
    expected(&ts, 6, "read float.hpr");
    expected(&py, 6, "refused float.hpr: $.rocket");
    assert!(
        hpr_format::read_json(&cases[6].1).is_err(),
        "hpr reads 2.0 as a count"
    );
}
