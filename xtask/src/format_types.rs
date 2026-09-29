//! TypeScript and Python types for the hpr design format (M3.3c), generated from its JSON Schema.
//!
//! Each language gets one module: a type for every definition in the schema, and a reader,
//! `readDesign` or `read_design`, that parses a document's text and checks it against the schema
//! before handing it back typed. The reader checks against a copy of the schema embedded in the
//! module, stripped of its prose, so a document either of them takes is one the schema takes;
//! the tests hold them to that on every public design and on mutations of two. `cargo xtask
//! format` writes both modules and `--check` fails when either is stale (ADR-113).

use serde_json::{Map, Value};

/// Where the TypeScript module is written, from the repository's root.
pub const TYPESCRIPT_PATH: &str = "schema/format/typescript/hpr-design.ts";

/// Where the Python module is written, from the repository's root.
pub const PYTHON_PATH: &str = "schema/format/python/hpr_design.py";

/// The name the schema's root, a whole document, takes in both languages.
const ROOT: &str = "DesignFile";

/// The names the TypeScript module's own code takes, which no type may.
const TYPESCRIPT_TAKEN: [&str; 17] = [
    "FORMAT",
    "VERSION",
    "SCHEMA",
    "DesignFormatError",
    "readDesign",
    "versionMessage",
    "SchemaNode",
    "Problem",
    "MOST_LEVELS",
    "LONE_SURROGATE",
    "scan",
    "shown",
    "has",
    "isObject",
    "isType",
    "member",
    "union",
];

/// The names the Python module's own code and imports take, which no type may.
const PYTHON_TAKEN: [&str; 13] = [
    "FORMAT",
    "VERSION",
    "DesignFormatError",
    "read_design",
    "json",
    "math",
    "re",
    "Any",
    "Literal",
    "NotRequired",
    "TypedDict",
    "Union",
    "cast",
];

/// The TypeScript reader, which the generated module ends with.
const TYPESCRIPT_READER: &str = include_str!("format_types/reader.ts");

/// The Python reader, which the generated module ends with.
const PYTHON_READER: &str = include_str!("format_types/reader.py");

/// Every schema keyword the generator understands. A schema that uses another fails to generate,
/// so a new kind of constraint can't pass the readers unchecked.
const KEYWORDS: [&str; 20] = [
    "$defs",
    "$ref",
    "$schema",
    "additionalProperties",
    "anyOf",
    "const",
    "default",
    "description",
    "format",
    "items",
    "maxItems",
    "minItems",
    "minimum",
    "oneOf",
    "pattern",
    "prefixItems",
    "properties",
    "required",
    "title",
    "type",
];

/// The `format`s whose values the readers check, as serde does when it reads the number: an
/// unsigned integer of 32 bits, and one the size of a pointer, which on the 64-bit machines hpr
/// runs on is 64 bits. The rest (`double`) constrain nothing JSON can carry.
const CHECKED_FORMATS: [&str; 2] = ["uint32", "uint"];

/// The TypeScript module for `schema`.
///
/// # Errors
///
/// The schema uses something the generator doesn't understand.
pub fn typescript(schema: &Value) -> Result<String, String> {
    let (root, defs) = parts(schema)?;
    if let Some(name) = defs
        .keys()
        .find(|n| TYPESCRIPT_TAKEN.contains(&n.as_str()) || n.as_str() == "check")
    {
        return Err(format!(
            "the definition {name} takes a name the reader uses"
        ));
    }
    let mut out = String::new();
    out.push_str(&header("//"));
    out.push_str(&format!(
        "\n/** The format's name: every document says `\"format\": \"{}\"`. */\n\
         export const FORMAT = \"{}\";\n\n\
         /** The version these types describe; `readDesign` reads documents of this version only. */\n\
         export const VERSION = \"{}\";\n",
        hpr_format::FORMAT,
        hpr_format::FORMAT,
        hpr_format::VERSION
    ));
    for (name, node) in
        std::iter::once((ROOT, root)).chain(defs.iter().map(|(k, v)| (k.as_str(), v)))
    {
        out.push('\n');
        out.push_str(&ts_doc(node, 0));
        if is_plain_object(node) {
            out.push_str(&format!(
                "export interface {name} {}\n",
                ts_object(node, 0)?
            ));
        } else {
            let body = ts_type(node, 0)?;
            let gap = if body.starts_with('\n') { "" } else { " " };
            out.push_str(&format!("export type {name} ={gap}{body};\n"));
        }
    }
    out.push_str(&format!(
        "\n// The schema the reader checks a document against: the committed schema without its prose.\n\
         const SCHEMA: SchemaNode = {};\n",
        serde_json::to_string(&embedded(schema)).map_err(|e| e.to_string())?
    ));
    out.push('\n');
    out.push_str(TYPESCRIPT_READER);
    Ok(out)
}

/// The Python module for `schema`.
///
/// # Errors
///
/// The schema uses something the generator doesn't understand, or two of its inline objects
/// would take the same class name.
pub fn python(schema: &Value) -> Result<String, String> {
    let (root, defs) = parts(schema)?;
    let mut py = Python::default();
    for name in PYTHON_TAKEN {
        py.claim(name)?;
    }
    for (name, _) in defs {
        py.claim(name)?;
    }
    py.claim(ROOT)?;
    py.definition(ROOT, root)?;
    for (name, node) in defs {
        py.definition(name, node)?;
    }
    let mut out = String::new();
    out.push_str(&header("#"));
    out.push_str(&format!(
        "\n\"\"\"Types for the hpr design format {}, and a reader that checks a document against its\n\
         schema. Generated from the schema by `cargo xtask format`; don't edit by hand.\"\"\"\n\n\
         import json\n\
         import math\n\
         import re\n\
         from typing import Any, Literal, NotRequired, TypedDict, Union, cast\n\n\
         FORMAT = \"{}\"\n\
         \"\"\"The format's name: every document says `\"format\": \"{}\"`.\"\"\"\n\n\
         VERSION = \"{}\"\n\
         \"\"\"The version these types describe; `read_design` reads documents of this version only.\"\"\"\n",
        hpr_format::VERSION,
        hpr_format::FORMAT,
        hpr_format::FORMAT,
        hpr_format::VERSION
    ));
    for item in &py.items {
        out.push_str("\n\n");
        out.push_str(item);
    }
    out.push_str(&format!(
        "\n\n# The schema the reader checks a document against: the committed schema without its prose.\n\
         _SCHEMA: Any = json.loads(\n    r\"\"\"{}\"\"\"\n)\n",
        serde_json::to_string(&embedded(schema)).map_err(|e| e.to_string())?
    ));
    out.push_str("\n\n");
    out.push_str(PYTHON_READER);
    Ok(out)
}

/// The first lines of either module, as comments starting with `comment`.
fn header(comment: &str) -> String {
    format!(
        "{comment} The hpr design format {version}: types for a document, and a reader that checks one.\n\
         {comment} SPDX-License-Identifier: MIT OR Apache-2.0 (https://github.com/nrdptel/hpr-sim)\n\
         {comment} Generated from schema/format/hpr-design-{version}.schema.json by `cargo xtask format`.\n\
         {comment} Don't edit by hand: the next run overwrites it, and CI fails while it is stale.\n",
        version = hpr_format::VERSION
    )
}

/// The schema's root and its definitions, after checking every keyword in it is understood.
fn parts(schema: &Value) -> Result<(&Value, &Map<String, Value>), String> {
    check_keywords(schema, "#")?;
    let defs = schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or("the schema has no $defs")?;
    Ok((schema, defs))
}

/// Fails on what the readers or the types would get wrong: a keyword outside [`KEYWORDS`]; a
/// schema that is `true` or `false` anywhere but `additionalProperties`; a `$ref`, union or
/// `const` with a constraint beside it, which the types would drop; a `format` other than
/// `double` that the readers don't check; and a `pattern` whose meaning differs between
/// JavaScript and Python (a class such as `\d`, or an escaped final `$`).
fn check_keywords(node: &Value, path: &str) -> Result<(), String> {
    let Value::Object(map) = node else {
        return Err(format!("{path}: a schema that is `{node}`, not an object"));
    };
    let beside = |allowed: &[&str]| -> Result<(), String> {
        match map.keys().find(|k| {
            !["description", "default", "title", "$schema", "$defs"].contains(&k.as_str())
                && !allowed.contains(&k.as_str())
        }) {
            Some(other) => Err(format!("{path}: `{other}` beside `{}`", allowed[0])),
            None => Ok(()),
        }
    };
    if map.contains_key("$ref") {
        beside(&["$ref"])?;
    }
    if map.contains_key("oneOf") {
        beside(&["oneOf"])?;
    }
    if map.contains_key("anyOf") {
        beside(&["anyOf"])?;
    }
    if map.contains_key("const") {
        beside(&["const", "type"])?;
    }
    for (key, value) in map {
        if !KEYWORDS.contains(&key.as_str()) {
            return Err(format!("{path}: the generator doesn't understand `{key}`"));
        }
        match key.as_str() {
            "properties" | "$defs" => {
                let children = value
                    .as_object()
                    .ok_or(format!("{path}/{key}: not an object"))?;
                for (name, child) in children {
                    check_keywords(child, &format!("{path}/{key}/{name}"))?;
                }
            }
            "oneOf" | "anyOf" | "prefixItems" => {
                let children = value
                    .as_array()
                    .ok_or(format!("{path}/{key}: not an array"))?;
                for (i, child) in children.iter().enumerate() {
                    check_keywords(child, &format!("{path}/{key}/{i}"))?;
                }
            }
            "additionalProperties" if value.is_boolean() => {}
            "items" | "additionalProperties" => check_keywords(value, &format!("{path}/{key}"))?,
            "format" => {
                let format = value.as_str().unwrap_or_default();
                if format != "double" && !CHECKED_FORMATS.contains(&format) {
                    return Err(format!(
                        "{path}: the generator doesn't check format `{format}`"
                    ));
                }
            }
            "const" if !value.is_string() => {
                return Err(format!("{path}: a `const` that isn't a string"));
            }
            "pattern" => {
                let pattern = value.as_str().unwrap_or_default();
                let class = pattern
                    .as_bytes()
                    .windows(2)
                    .any(|w| w[0] == b'\\' && w[1].is_ascii_alphabetic());
                if class || pattern.ends_with("\\$") {
                    return Err(format!(
                        "{path}: the pattern `{pattern}` reads differently in JavaScript and Python"
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// `schema` as the readers embed it: its prose (`description`, `title`), its defaults and the
/// formats they don't check taken out, which leaves only what decides whether a document is valid.
fn embedded(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, value)| match key.as_str() {
                    "description" | "title" | "default" | "$schema" => false,
                    "format" => value.as_str().is_some_and(|f| CHECKED_FORMATS.contains(&f)),
                    _ => true,
                })
                .map(|(key, value)| {
                    // `properties` and `$defs` map names to schemas: a property named, say,
                    // `description` is kept.
                    let value = if key == "properties" || key == "$defs" {
                        Value::Object(
                            value
                                .as_object()
                                .into_iter()
                                .flatten()
                                .map(|(name, child)| (name.clone(), embedded(child)))
                                .collect(),
                        )
                    } else {
                        embedded(value)
                    };
                    (key.clone(), value)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(embedded).collect()),
        other => other.clone(),
    }
}

/// Whether `node` is an object with named properties and nothing else: a TypeScript interface, a
/// Python `TypedDict`.
fn is_plain_object(node: &Value) -> bool {
    node.get("type").and_then(Value::as_str) == Some("object") && node.get("properties").is_some()
}

/// The name a `$ref` points at, if `node` is one.
fn reference(node: &Value) -> Result<Option<&str>, String> {
    match node.get("$ref").and_then(Value::as_str) {
        Some(target) => target
            .strip_prefix("#/$defs/")
            .map(Some)
            .ok_or(format!("a $ref outside $defs: {target}")),
        None => Ok(None),
    }
}

/// `node`'s JSON types: the `type` keyword as a list, or none when it has no `type`.
fn types(node: &Value) -> Vec<&str> {
    match node.get("type") {
        Some(Value::String(t)) => vec![t.as_str()],
        Some(Value::Array(ts)) => ts.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    }
}

/// `node`'s description, and what an absent value means where it has a default, as prose.
fn prose(node: &Value) -> Option<String> {
    let description = node
        .get("description")
        .and_then(Value::as_str)
        .map(without_rust_links);
    let description = description.as_deref();
    let default = node.get("default").map(|d| format!("Absent means `{d}`."));
    match (description, default) {
        (Some(d), Some(a)) => Some(format!("{d}\n\n{a}")),
        (Some(d), None) => Some(d.to_owned()),
        (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

/// `text` with its links to Rust items, which mean nothing outside rustdoc, left as the code
/// they show: `` [`Self::pods`] `` and `` [`pods`](crate::PodSet::pods) `` become `` `pods` ``
/// and `` `Self::pods` ``. Links to web pages and reference links (`[ADR-074][adr-074]`) stay.
fn without_rust_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("[`") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find("`]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let code = &after[..=end];
        let tail = &after[end + 2..];
        if code[1..code.len() - 1].contains('`') {
            // Not one code span: leave the bracket.
            out.push('[');
            rest = after;
        } else if let Some(target) = tail.strip_prefix('(') {
            let close = target.find(')').unwrap_or(target.len());
            let url = &target[..close];
            if url.starts_with("http") || url.starts_with('#') {
                out.push_str(&rest[start..start + 1 + end + 2]);
                rest = tail;
            } else {
                out.push_str(code);
                rest = target.get(close + 1..).unwrap_or_default();
            }
        } else if tail.starts_with('[') {
            out.push_str(&rest[start..start + 1 + end + 2]);
            rest = tail;
        } else {
            out.push_str(code);
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// Whether `key` can stand bare as a property name in both languages.
fn is_identifier(key: &str) -> bool {
    let mut chars = key.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `text` as a JSON string literal, which is a string literal in TypeScript and in Python too.
fn quoted(text: &str) -> String {
    Value::String(text.to_owned()).to_string()
}

// --- TypeScript ---------------------------------------------------------------------------------

/// `node`'s prose as a doc comment at `indent` levels, or nothing.
fn ts_doc(node: &Value, indent: usize) -> String {
    let Some(text) = prose(node) else {
        return String::new();
    };
    let pad = "  ".repeat(indent);
    let text = text.replace("*/", "*\\/");
    if !text.contains('\n') && text.len() + pad.len() < 94 {
        return format!("{pad}/** {text} */\n");
    }
    let mut out = format!("{pad}/**\n");
    for line in text.lines() {
        if line.is_empty() {
            out.push_str(&format!("{pad} *\n"));
        } else {
            out.push_str(&format!("{pad} * {line}\n"));
        }
    }
    out.push_str(&format!("{pad} */\n"));
    out
}

/// The TypeScript type of `node`, written at `indent` levels.
fn ts_type(node: &Value, indent: usize) -> Result<String, String> {
    if let Some(name) = reference(node)? {
        return Ok(name.to_owned());
    }
    if let Some(value) = node.get("const") {
        return Ok(value.to_string());
    }
    if let Some(members) = node
        .get("oneOf")
        .or(node.get("anyOf"))
        .and_then(Value::as_array)
    {
        let types = members
            .iter()
            .map(|m| ts_type(m, indent + 1))
            .collect::<Result<Vec<_>, _>>()?;
        let one_line = types.join(" | ");
        if types.len() < 2 || (!one_line.contains('\n') && one_line.len() < 80) {
            return Ok(one_line);
        }
        // One member a line, each after its own doc comment.
        let pad = "  ".repeat(indent + 1);
        return Ok(members
            .iter()
            .zip(&types)
            .map(|(m, t)| format!("\n{}{pad}| {t}", ts_doc(m, indent + 1)))
            .collect());
    }
    let types = types(node);
    if types.is_empty() {
        return Ok("unknown".to_owned());
    }
    let members = types
        .iter()
        .map(|t| ts_single(node, t, indent))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ts_union(&members, indent))
}

/// `members` joined as a union: on one line when it is short, else one member a line.
fn ts_union(members: &[String], indent: usize) -> String {
    let one_line = members.join(" | ");
    if members.len() < 2 || (!one_line.contains('\n') && one_line.len() < 80) {
        return one_line;
    }
    let pad = "  ".repeat(indent + 1);
    members
        .iter()
        .map(|m| format!("\n{pad}| {m}"))
        .collect::<String>()
}

/// The TypeScript type of `node` as the one JSON type `kind`.
fn ts_single(node: &Value, kind: &str, indent: usize) -> Result<String, String> {
    Ok(match kind {
        "string" => "string".to_owned(),
        "number" | "integer" => "number".to_owned(),
        "boolean" => "boolean".to_owned(),
        "null" => "null".to_owned(),
        "array" => {
            if let Some(prefix) = node.get("prefixItems").and_then(Value::as_array) {
                let items = prefix
                    .iter()
                    .map(|p| ts_type(p, indent))
                    .collect::<Result<Vec<_>, _>>()?;
                format!("[{}]", items.join(", "))
            } else {
                let item = node.get("items").ok_or("an array without items")?;
                let item = ts_type(item, indent)?;
                if item.contains(' ') || item.contains('\n') {
                    format!("Array<{item}>")
                } else {
                    format!("{item}[]")
                }
            }
        }
        "object" => ts_object(node, indent)?,
        other => return Err(format!("unknown JSON type `{other}`")),
    })
}

/// An object type's body, `{ ... }`, written at `indent` levels.
fn ts_object(node: &Value, indent: usize) -> Result<String, String> {
    let properties = node.get("properties").and_then(Value::as_object);
    let extra = node.get("additionalProperties");
    let Some(properties) = properties else {
        return match extra {
            Some(Value::Bool(false)) => Ok("Record<string, never>".to_owned()),
            Some(schema @ Value::Object(_)) => {
                Ok(format!("{{ [key: string]: {} }}", ts_type(schema, indent)?))
            }
            _ => Ok("{ [key: string]: unknown }".to_owned()),
        };
    };
    if !matches!(extra, Some(Value::Bool(false))) {
        return Err("an object with properties that allows other keys".to_owned());
    }
    let required = required(node);
    let pad = "  ".repeat(indent + 1);
    let mut out = "{\n".to_owned();
    for (key, schema) in properties {
        out.push_str(&ts_doc(schema, indent + 1));
        let name = if is_identifier(key) {
            key.clone()
        } else {
            quoted(key)
        };
        let optional = if required.contains(&key.as_str()) {
            ""
        } else {
            "?"
        };
        out.push_str(&format!(
            "{pad}{name}{optional}: {};\n",
            ts_type(schema, indent + 1)?
        ));
    }
    out.push_str(&"  ".repeat(indent));
    out.push('}');
    Ok(out)
}

/// The names `node` requires.
fn required(node: &Value) -> Vec<&str> {
    node.get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

// --- Python -------------------------------------------------------------------------------------

/// The Python module under construction: its items in order, and the names they have taken.
#[derive(Debug, Default)]
struct Python {
    /// Each type alias or `TypedDict`, as text.
    items: Vec<String>,
    /// Every name taken, so an inline object can't take a definition's.
    names: std::collections::BTreeSet<String>,
}

impl Python {
    /// Takes `name`, failing if it is taken.
    fn claim(&mut self, name: &str) -> Result<(), String> {
        if self.names.insert(name.to_owned()) {
            Ok(())
        } else {
            Err(format!("two Python types would be named {name}"))
        }
    }

    /// Writes the definition `name`, and the inline objects it holds after it.
    fn definition(&mut self, name: &str, node: &Value) -> Result<(), String> {
        if is_plain_object(node) {
            self.typed_dict(name, node)
        } else {
            let slot = self.items.len();
            self.items.push(String::new());
            let annotation = self.annotation(node, name)?;
            self.items[slot] = format!("{name} = {}\n{}", py_wrapped(&annotation), py_doc(node, 0));
            Ok(())
        }
    }

    /// Writes the object `node` as the `TypedDict` `name`, and the inline objects it holds after
    /// it.
    fn typed_dict(&mut self, name: &str, node: &Value) -> Result<(), String> {
        let slot = self.items.len();
        self.items.push(String::new());
        let required = required(node);
        let properties = node
            .get("properties")
            .and_then(Value::as_object)
            .ok_or("a TypedDict without properties")?;
        let mut fields = Vec::new();
        for (key, schema) in properties {
            let annotation = self.annotation(schema, &format!("{name}{}", camel(key)))?;
            let annotation = if required.contains(&key.as_str()) {
                annotation
            } else {
                format!("NotRequired[{annotation}]")
            };
            fields.push((key, annotation, schema));
        }
        // A key that isn't a Python identifier (`from`, `x-openrocket`) needs the functional form.
        let functional = fields
            .iter()
            .any(|(key, _, _)| !is_identifier(key) || is_python_keyword(key));
        let mut text = String::new();
        if functional {
            if let Some(doc) = prose(node) {
                for line in doc.lines() {
                    text.push_str(&format!("# {line}\n").replace("# \n", "#\n"));
                }
            }
            text.push_str(&format!(
                "{name} = TypedDict(\n    {},\n    {{\n",
                quoted(name)
            ));
            for (key, annotation, schema) in &fields {
                if let Some(doc) = prose(schema) {
                    for line in doc.lines() {
                        text.push_str(&format!("        # {line}\n").replace("# \n", "#\n"));
                    }
                }
                text.push_str(&format!("        {}: {annotation},\n", quoted(key)));
            }
            text.push_str("    },\n)\n");
        } else {
            text.push_str(&format!("class {name}(TypedDict):\n"));
            text.push_str(&py_doc(node, 1));
            for (key, annotation, schema) in &fields {
                text.push_str(&format!("\n    {key}: {annotation}\n"));
                text.push_str(&py_doc(schema, 1));
            }
        }
        self.items[slot] = text;
        Ok(())
    }

    /// The annotation for `node`; an inline object in it becomes a `TypedDict` named after
    /// `context`, the place it sits.
    fn annotation(&mut self, node: &Value, context: &str) -> Result<String, String> {
        if let Some(name) = reference(node)? {
            return Ok(quoted(name));
        }
        if let Some(value) = node.get("const") {
            return Ok(format!("Literal[{value}]"));
        }
        if let Some(members) = node
            .get("oneOf")
            .or(node.get("anyOf"))
            .and_then(Value::as_array)
        {
            let members = members
                .iter()
                .enumerate()
                .map(|(i, m)| self.annotation(m, &format!("{context}{}", variant_name(m, i))))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(py_union(&members));
        }
        let types = types(node);
        if types.is_empty() {
            return Ok("Any".to_owned());
        }
        let members = types
            .iter()
            .map(|t| self.single(node, t, context))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(py_union(&members))
    }

    /// The annotation of `node` as the one JSON type `kind`.
    fn single(&mut self, node: &Value, kind: &str, context: &str) -> Result<String, String> {
        Ok(match kind {
            "string" => "str".to_owned(),
            "number" => "float".to_owned(),
            "integer" => "int".to_owned(),
            "boolean" => "bool".to_owned(),
            "null" => "None".to_owned(),
            "array" => {
                if let Some(prefix) = node.get("prefixItems").and_then(Value::as_array) {
                    let items = prefix
                        .iter()
                        .enumerate()
                        .map(|(i, p)| self.annotation(p, &format!("{context}{i}")))
                        .collect::<Result<Vec<_>, _>>()?;
                    format!("tuple[{}]", items.join(", "))
                } else {
                    let item = node.get("items").ok_or("an array without items")?;
                    format!(
                        "list[{}]",
                        self.annotation(item, &format!("{context}Item"))?
                    )
                }
            }
            "object" => {
                if node.get("properties").is_some() {
                    self.claim(context)?;
                    self.typed_dict(context, node)?;
                    quoted(context)
                } else {
                    match node.get("additionalProperties") {
                        Some(schema @ Value::Object(_)) => format!(
                            "dict[str, {}]",
                            self.annotation(schema, &format!("{context}Value"))?
                        ),
                        _ => "dict[str, Any]".to_owned(),
                    }
                }
            }
            other => return Err(format!("unknown JSON type `{other}`")),
        })
    }
}

/// The part of an inline union member's class name that tells it from its siblings: the value
/// of its string tag (`{"from": "top", ...}`), else the key of an externally tagged variant
/// (`{"nose_cone": ...}`), else its place.
fn variant_name(member: &Value, index: usize) -> String {
    let properties = member.get("properties").and_then(Value::as_object);
    if let Some(properties) = properties {
        if let Some(tag) = properties
            .values()
            .find_map(|p| p.get("const").and_then(Value::as_str))
        {
            return camel(tag);
        }
        if properties.len() == 1
            && let Some(key) = properties.keys().next()
        {
            return camel(key);
        }
    }
    format!("Variant{index}")
}

/// `members` as one annotation: a single member bare, string constants merged into one
/// `Literal`, anything else a `Union`.
fn py_union(members: &[String]) -> String {
    if members.len() == 1 {
        return members[0].clone();
    }
    let literals: Vec<&str> = members
        .iter()
        .filter_map(|m| m.strip_prefix("Literal[")?.strip_suffix(']'))
        .collect();
    if literals.len() == members.len() {
        return format!("Literal[{}]", literals.join(", "));
    }
    format!("Union[{}]", members.join(", "))
}

/// A top-level annotation, one member a line when it is long: `Union[\n    "A",\n    "B",\n]`.
fn py_wrapped(annotation: &str) -> String {
    let Some(open) = annotation.find('[') else {
        return annotation.to_owned();
    };
    if annotation.len() <= 88 || !annotation.ends_with(']') {
        return annotation.to_owned();
    }
    let inner = &annotation[open + 1..annotation.len() - 1];
    let mut members = Vec::new();
    let (mut depth, mut start, mut in_string) = (0_i32, 0, false);
    for (i, c) in inner.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '[' if !in_string => depth += 1,
            ']' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                members.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    members.push(inner[start..].trim());
    let lines: String = members.iter().map(|m| format!("    {m},\n")).collect();
    format!("{}\n{lines}]", &annotation[..=open])
}

/// `text` in CamelCase: `nose_cone` → `NoseCone`, `x-openrocket` → `XOpenrocket`.
fn camel(text: &str) -> String {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            chars
                .next()
                .map(|c| c.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Whether `word` is reserved in Python, so can't name a class field.
fn is_python_keyword(word: &str) -> bool {
    const KEYWORDS: [&str; 35] = [
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return",
        "try", "while", "with", "yield",
    ];
    KEYWORDS.contains(&word)
}

/// `node`'s prose as a docstring at `indent` levels, or nothing.
fn py_doc(node: &Value, indent: usize) -> String {
    let Some(text) = prose(node) else {
        return String::new();
    };
    let pad = "    ".repeat(indent);
    let text = text.replace('\\', "\\\\").replace("\"\"\"", "\\\"\\\"\\\"");
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        return format!("{pad}\"\"\"{first}\"\"\"\n");
    }
    let mut out = format!("{pad}\"\"\"{first}\n");
    for line in rest {
        if line.is_empty() {
            out.push('\n');
        } else {
            out.push_str(&format!("{pad}{line}\n"));
        }
    }
    out.push_str(&format!("{pad}\"\"\"\n"));
    out
}

// --- Public documents and the static check ------------------------------------------------------

/// The public `.ork` designs whose documents the readers and the type checkers read: Loft's seven
/// demonstrations and the OpenRocket probes committed under `validation/fixtures/ork/`.
const PUBLIC_DIRS: [&str; 3] = [
    "validation/fixtures/ork/loft-demo",
    "validation/fixtures/ork/pod-flights",
    "validation/fixtures/ork/rod-flights",
];

/// The committed 0.1 document (M3.3b), which the readers see migrated to this version.
const DOCUMENT_0_1: &str = "crates/hpr-format/fixtures/embedded-curve-0.1.hpr";

/// Every public design as a document of this version: its file's stem and the document's text,
/// as `hpr convert` writes it (`hpr_format::to_json`). The committed 0.1 document comes last,
/// migrated.
///
/// # Errors
///
/// A design doesn't read or a document doesn't write.
pub fn public_documents(root: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let mut documents = Vec::new();
    for dir in PUBLIC_DIRS {
        let mut paths: Vec<_> = std::fs::read_dir(root.join(dir))
            .map_err(|e| format!("{dir}: {e}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "ork"))
            .collect();
        paths.sort();
        for path in paths {
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let document = hpr_format::DesignFile::from_ork(&bytes)
                .map_err(|e| format!("{}: {e}", path.display()))?
                .value;
            let text = hpr_format::to_json(&document).map_err(|e| e.to_string())?;
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            documents.push((stem.into_owned(), text));
        }
    }
    let old = std::fs::read_to_string(root.join(DOCUMENT_0_1))
        .map_err(|e| format!("{DOCUMENT_0_1}: {e}"))?;
    let migrated = hpr_format::read_json(&old)
        .map_err(|e| e.to_string())?
        .value;
    documents.push((
        "embedded-curve-0.1-migrated".to_owned(),
        hpr_format::to_json(&migrated).map_err(|e| e.to_string())?,
    ));
    Ok(documents)
}

/// The TypeScript compiler the static check runs, from npm.
const TYPESCRIPT_PACKAGE: &str = "typescript@7.0.2";

/// The Python type checker the static check runs, from PyPI.
const MYPY_PACKAGE: &str = "mypy==2.3.1";

/// `cargo xtask format --typecheck`: checks every public document, written as a literal of type
/// `DesignFile`, with `tsc --strict` and `mypy --strict`, and that each of them refuses a
/// document with a misspelt tag. Needs `npx` and `uvx`, and the network the first time each
/// fetches its checker.
///
/// # Errors
///
/// A checker refuses a public document, takes the misspelt one, or can't run.
pub fn typecheck(root: &std::path::Path) -> Result<(), String> {
    let dir = root.join("target/format-types");
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["ts/good", "ts/bad", "py/good", "py/bad"] {
        std::fs::create_dir_all(dir.join(sub)).map_err(|e| format!("{sub}: {e}"))?;
    }
    let documents = public_documents(root)?;
    let (mut ts, mut py) = (Vec::new(), Vec::new());
    for (i, (_, text)) in documents.iter().enumerate() {
        let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        ts.push(format!("doc_{i}.ts"));
        write(
            &dir.join("ts/good").join(format!("doc_{i}.ts")),
            &ts_literal(&value),
        )?;
        py.push(format!("doc_{i}.py"));
        write(
            &dir.join("py/good").join(format!("doc_{i}.py")),
            &py_literal(&value),
        )?;
    }
    // The first document with a position's tag misspelt, which both checkers must refuse.
    let bad: Value = serde_json::from_str(&documents[0].1.replacen(
        "\"from\": \"top\"",
        "\"from\": \"topp\"",
        1,
    ))
    .map_err(|e| e.to_string())?;
    write(&dir.join("ts/bad/doc.ts"), &ts_literal(&bad))?;
    write(&dir.join("py/bad/doc.py"), &py_literal(&bad))?;
    for sub in ["ts/good", "ts/bad"] {
        std::fs::copy(
            root.join(TYPESCRIPT_PATH),
            dir.join(sub).join("hpr-design.ts"),
        )
        .map_err(|e| e.to_string())?;
    }
    for sub in ["py/good", "py/bad"] {
        std::fs::copy(root.join(PYTHON_PATH), dir.join(sub).join("hpr_design.py"))
            .map_err(|e| e.to_string())?;
    }
    // Each example runs on its module, so it is checked with it: the TypeScript one against a
    // declaration of the little of Node.js it uses, since the compiler has no Node.js types here.
    std::fs::copy(
        root.join("schema/format/typescript/read-design.ts"),
        dir.join("ts/good/read-design.ts"),
    )
    .map_err(|e| e.to_string())?;
    write(&dir.join("ts/good/node.d.ts"), NODE_DECLARATIONS)?;
    ts.push("read-design.ts".to_owned());
    ts.push("node.d.ts".to_owned());
    // The Python example runs on the module, so it is checked with it.
    std::fs::copy(
        root.join("schema/format/python/read_design.py"),
        dir.join("py/good/read_design.py"),
    )
    .map_err(|e| e.to_string())?;
    py.push("read_design.py".to_owned());

    let tsc = |sub: &str, files: &[String]| {
        let mut args: Vec<String> = [
            "--yes",
            "--package",
            TYPESCRIPT_PACKAGE,
            "tsc",
            "--strict",
            "--noEmit",
            "--target",
            "es2022",
            "--module",
            "nodenext",
            "--allowImportingTsExtensions",
            "hpr-design.ts",
        ]
        .map(str::to_owned)
        .to_vec();
        args.extend_from_slice(files);
        run(
            if cfg!(windows) { "npx.cmd" } else { "npx" },
            &args,
            &dir.join(sub),
        )
    };
    let mypy = |sub: &str, files: &[String]| {
        let mut args: Vec<String> = [
            "--quiet",
            "--from",
            MYPY_PACKAGE,
            "mypy",
            "--strict",
            "--no-incremental",
            "hpr_design.py",
        ]
        .map(str::to_owned)
        .to_vec();
        args.extend_from_slice(files);
        run("uvx", &args, &dir.join(sub))
    };
    let (ts_ok, ts_out) = tsc("ts/good", &ts)?;
    if !ts_ok {
        return Err(format!("tsc refused a public document:\n{ts_out}"));
    }
    let (py_ok, py_out) = mypy("py/good", &py)?;
    if !py_ok {
        return Err(format!("mypy refused a public document:\n{py_out}"));
    }
    let (ts_bad, _) = tsc("ts/bad", &["doc.ts".to_owned()])?;
    let (py_bad, _) = mypy("py/bad", &["doc.py".to_owned()])?;
    if ts_bad || py_bad {
        return Err(format!(
            "a checker took a misspelt tag: tsc {}, mypy {}",
            if ts_bad { "took it" } else { "refused it" },
            if py_bad { "took it" } else { "refused it" }
        ));
    }
    println!(
        "format: {TYPESCRIPT_PACKAGE} and {MYPY_PACKAGE} take {} public documents typed as \
         DesignFile, and refuse a misspelt tag",
        documents.len()
    );
    Ok(())
}

/// What the TypeScript example uses of Node.js, declared for the compiler.
const NODE_DECLARATIONS: &str = "\
declare module \"node:fs\" {
  export function readFileSync(path: string, encoding: \"utf8\"): string;
}
declare const process: { argv: string[]; exitCode: number | undefined };
";

/// Writes `text` to `path`.
fn write(path: &std::path::Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Runs `program` with `args` in `dir`: whether it succeeded, and what it printed.
fn run(program: &str, args: &[String], dir: &std::path::Path) -> Result<(bool, String), String> {
    let output = std::process::Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| format!("can't run {program}: {e}"))?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    Ok((output.status.success(), text))
}

/// A TypeScript module holding `document` as a constant of type `DesignFile`. JSON is a
/// TypeScript expression, and a literal's excess keys and wrong tags are errors, so the compiler
/// checks the whole document against the types.
fn ts_literal(document: &Value) -> String {
    format!(
        "import type {{ DesignFile }} from \"./hpr-design.ts\";\n\nexport const document: DesignFile = {document};\n"
    )
}

/// A Python module holding `document` as a variable of type `DesignFile`, which mypy checks
/// as it checks a `TypedDict` literal.
fn py_literal(document: &Value) -> String {
    format!(
        "from hpr_design import DesignFile\n\ndocument: DesignFile = {}\n",
        python_value(document)
    )
}

/// `value` as a Python expression.
fn python_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_owned(),
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        // JSON's numbers and strings are Python's too: serde_json writes no `\/` escape.
        Value::Number(_) | Value::String(_) => value.to_string(),
        Value::Array(items) => {
            let items: Vec<String> = items.iter().map(python_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(map) => {
            let entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", quoted(k), python_value(v)))
                .collect();
            format!("{{{}}}", entries.join(", "))
        }
    }
}

#[cfg(test)]
mod tests;
