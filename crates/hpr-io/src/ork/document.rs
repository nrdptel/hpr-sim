//! The design document: the XML tree inside a `.ork`, kept whole.
//!
//! OpenRocket publishes no schema for `.ork` (there is no XSD), and every version has added tags.
//! So the document is read into a plain tree of elements and text, with nothing thrown away and
//! nothing interpreted: later milestones walk it into [`hpr_design`] types, and whatever they do
//! not understand is still here to be written back out. Reading and writing are exact inverses —
//! [`Document::to_xml`] followed by [`Document::parse`] returns the same document.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::OrkError;
use super::warning::{Imported, Warning, WarningKind};

/// The `.ork` schema versions this reader knows, as `1.0` up to and including `1.MAX_KNOWN_MINOR`.
///
/// 1.9 is OpenRocket 23.09 and 1.10 is 24.12; 1.11 is documented for 26.xx and adds embedded
/// `.rse` thrust curves, CSV lookup tables, a gravity model and `preview.png`
/// (`docs/VALIDATION.md`). A newer file is read anyway, with a warning.
pub const MAX_KNOWN_MINOR: u32 = 11;

/// How deeply elements may nest before the document is refused.
///
/// An ordinary `.ork` design nests 11 deep, and the deepest of the 76 in the reference corpus —
/// OpenRocket's own parallel-booster example — reaches 17, so 64 leaves room to spare. The limit
/// is here because both the XML parser underneath and this module's own reader descend the tree:
/// a file written to nest far enough exhausts the stack, which is a crash where [Loft lesson L56:
/// malformed input must give an error rather than crash][lessons] asks for an error. On a debug
/// test build with a 2 MiB stack, `roxmltree` read 120 levels and died on 130, so the depth is
/// counted before the text is handed to it.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
pub const MAX_DEPTH: usize = 64;

/// A `.ork` schema version, as written in the root element's `version` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// The major version. Every `.ork` ever written has 1 here.
    pub major: u32,
    /// The minor version: 0 to 11 so far.
    pub minor: u32,
}

impl SchemaVersion {
    /// Reads `major.minor`, the only form the attribute takes.
    pub fn parse(text: &str) -> Result<Self, OrkError> {
        let version = || OrkError::Version {
            text: text.to_owned(),
        };
        let (major, minor) = text.split_once('.').ok_or_else(version)?;
        let number = |field: &str| {
            if field.is_empty() || !field.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(version());
            }
            field.parse::<u32>().map_err(|_| version())
        };
        Ok(Self {
            major: number(major)?,
            minor: number(minor)?,
        })
    }

    /// Whether this version is one the format documentation describes.
    pub fn is_known(self) -> bool {
        self.major == 1 && self.minor <= MAX_KNOWN_MINOR
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// A design document: its schema version, the program that wrote it, and the whole XML tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    /// The root element's `version` attribute.
    pub version: SchemaVersion,
    /// The root element's `creator` attribute, for example `OpenRocket 24.12`. Absent in files
    /// some other program wrote.
    pub creator: Option<String>,
    /// The `<openrocket>` element, with every attribute and child it was written with.
    pub root: Element,
}

/// An XML element: its name, its attributes in the order they were written, and its children.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Element {
    /// The element's name, such as `nosecone`.
    pub name: String,
    /// The attributes, in document order, as name and value.
    pub attributes: Vec<(String, String)>,
    /// The children, in document order.
    pub children: Vec<Node>,
}

/// A child of an [`Element`]: another element, or text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Node {
    /// A child element.
    Element(Element),
    /// Text. Character and entity references are already resolved.
    Text {
        /// The text as it stands.
        text: String,
    },
}

impl Element {
    /// The value of the attribute called `name`, if it has one.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The child elements called `name`, in document order.
    pub fn children_named<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a Element> {
        let name = name.to_owned();
        self.elements().filter(move |child| child.name == name)
    }

    /// The first child element called `name`.
    pub fn child<'a>(&'a self, name: &str) -> Option<&'a Element> {
        self.elements().find(|child| child.name == name)
    }

    /// Every child element, in document order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|child| match child {
            Node::Element(element) => Some(element),
            Node::Text { .. } => None,
        })
    }

    /// The element's text: its text children joined, or the empty string when it has none.
    ///
    /// `.ork` values sit in leaf elements (`<length>0.61</length>`), so this is how a value is
    /// read. It is returned as written; the caller trims and parses it.
    pub fn text(&self) -> String {
        let mut text = String::new();
        for child in &self.children {
            if let Node::Text { text: part } = child {
                text.push_str(part);
            }
        }
        text
    }

    fn has_element_child(&self) -> bool {
        self.elements().next().is_some()
    }

    fn has_text_child(&self) -> bool {
        self.children
            .iter()
            .any(|child| matches!(child, Node::Text { .. }))
    }
}

impl Document {
    /// Reads a design document from XML text.
    ///
    /// Fails when the text is not well-formed XML, when its root is not `<openrocket>`, or when
    /// the root carries no readable `version`. Everything else is a warning: a schema version
    /// newer than [`MAX_KNOWN_MINOR`], a missing `creator`, or a comment (which is dropped).
    ///
    /// Blank text between child elements — the indentation OpenRocket writes — is not kept, so
    /// that writing the document back out is free to lay it out again. Text an element holds
    /// beside child elements *is* kept: OpenRocket writes a simulation `<warning>` that way.
    pub fn parse(text: &str) -> Result<Imported<Self>, OrkError> {
        let deepest = deepest_nesting(text);
        if deepest > MAX_DEPTH {
            return Err(OrkError::TooDeep {
                limit: MAX_DEPTH,
                depth: deepest,
            });
        }
        let parsed = roxmltree::Document::parse(text).map_err(|error| OrkError::Xml {
            reason: error.to_string(),
        })?;
        let root = parsed.root_element();
        if root.tag_name().name() != "openrocket" {
            return Err(OrkError::NotOpenRocket {
                root: root.tag_name().name().to_owned(),
            });
        }
        let version = SchemaVersion::parse(root.attribute("version").unwrap_or_default())?;
        let creator = root.attribute("creator").map(str::to_owned);

        let mut warnings = Vec::new();
        if !version.is_known() {
            warnings.push(Warning::new(
                "openrocket",
                WarningKind::Unusual,
                if version.major == 1 {
                    format!(
                        "schema version {version} is newer than 1.{MAX_KNOWN_MINOR}, the newest \
                         this reader knows; it was read as far as it is understood"
                    )
                } else {
                    format!(
                        "schema version {version} is not one this reader knows (1.0 to \
                         1.{MAX_KNOWN_MINOR}); it was read as far as it is understood"
                    )
                },
            ));
        }
        if creator.is_none() {
            warnings.push(Warning::new(
                "openrocket",
                WarningKind::Unusual,
                "the root element has no `creator` attribute, so the program that wrote this \
                 design is unknown",
            ));
        }
        if root.tag_name().namespace().is_some()
            || parsed
                .descendants()
                .any(|node| node.namespaces().next().is_some())
        {
            warnings.push(Warning::new(
                "openrocket",
                WarningKind::Dropped,
                "this document declares XML namespaces; prefixes and declarations are dropped, \
                 and two attributes that differ only by prefix become one. No `.ork` OpenRocket \
                 writes uses them",
            ));
        }
        let root = read_element(root, "", &mut warnings);
        Ok(Imported {
            value: Self {
                version,
                creator,
                root,
            },
            warnings,
        })
    }

    /// Writes the document back out as XML.
    ///
    /// The result is written in one fixed style rather than byte-identical to the file it was
    /// read from: two-space indentation, attributes in the order they were read, and `<tag/>` for
    /// an empty element. Reading it again gives an equal [`Document`], which is what makes the
    /// tree lossless.
    ///
    /// [`version`](Self::version) and [`creator`](Self::creator) are written onto the root, so
    /// changing either changes the file. Everything else comes from [`root`](Self::root).
    ///
    /// This is defined for a document that came from [`parse`](Self::parse). `Element`'s fields
    /// are public, and a tree built by hand can hold a name that is not an XML name, or nest
    /// deeper than [`MAX_DEPTH`] — writing either gives XML that will not read back.
    pub fn to_xml(&self) -> String {
        let mut root = self.root.clone();
        set_attribute(&mut root, "version", Some(self.version.to_string()));
        set_attribute(&mut root, "creator", self.creator.clone());
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        write_element(&root, 0, &mut out);
        out.push('\n');
        out
    }
}

/// Reads one element and its children, recording `at` as the path for any warning.
fn read_element(
    node: roxmltree::Node<'_, '_>,
    parent: &str,
    warnings: &mut Vec<Warning>,
) -> Element {
    let name = node.tag_name().name().to_owned();
    let at = if parent.is_empty() {
        name.clone()
    } else {
        format!("{parent}/{name}")
    };
    let attributes = node
        .attributes()
        .map(|attribute| (attribute.name().to_owned(), attribute.value().to_owned()))
        .collect();

    let keeps_blank_text = !node.children().any(|child| child.is_element());
    let mut children = Vec::new();
    for child in node.children() {
        if child.is_element() {
            children.push(Node::Element(read_element(child, &at, warnings)));
        } else if child.is_text() {
            let text = child.text().unwrap_or_default();
            // A comment, a processing instruction or a CDATA section splits a run of text in two.
            // Joining them here is what keeps writing and reading again exact: the writer has
            // nowhere to put the split, since it drops the comment that made it.
            if let Some(Node::Text { text: last }) = children.last_mut()
                && (keeps_blank_text || !text.trim().is_empty())
            {
                last.push_str(text);
                continue;
            }
            if keeps_blank_text {
                children.push(Node::Text {
                    text: text.to_owned(),
                });
            } else if !text.trim().is_empty() {
                // An element with both child elements and text of its own. OpenRocket writes one:
                // a simulation's `<warning>` prints its own message after its fields. Both are
                // kept, and the blank text that only lays the file out is not.
                children.push(Node::Text {
                    text: text.to_owned(),
                });
            }
        } else if child.is_comment() {
            warnings.push(Warning::new(
                at.clone(),
                WarningKind::Dropped,
                "an XML comment was dropped; `.ork` carries no meaning in comments",
            ));
        } else if child.is_pi() {
            warnings.push(Warning::new(
                at.clone(),
                WarningKind::Dropped,
                "an XML processing instruction was dropped",
            ));
        }
    }
    Element {
        name,
        attributes,
        children,
    }
}

/// Counts how deeply elements nest in `text`, without building a tree.
///
/// This is a scan, not a parser: it walks the text, skips comments, CDATA sections and processing
/// instructions, tracks quotes so that a `>` inside an attribute value does not end a tag, and
/// counts start tags against end tags. It can only ever return more nesting than a parser will
/// find — XML forbids a raw `<` inside an attribute value, so every element start it sees is one —
/// which is what makes it safe to use as a guard.
pub(super) fn deepest_nesting(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut depth = 0usize;
    let mut deepest = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        let rest = &bytes[index..];
        if let Some(skip) = skipped(rest, b"<!--", b"-->")
            .or_else(|| skipped(rest, b"<![CDATA[", b"]]>"))
            .or_else(|| skipped(rest, b"<?", b"?>"))
        {
            index += skip;
            continue;
        }
        let closing = rest.starts_with(b"</");
        // `<!DOCTYPE …>` and the like: not an element, so it changes no depth.
        let declaration = rest.starts_with(b"<!");

        let mut cursor = index + 1;
        let mut quote: Option<u8> = None;
        let mut last = 0u8;
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            match quote {
                Some(open) if byte == open => quote = None,
                Some(_) => {}
                None if byte == b'"' || byte == b'\'' => quote = Some(byte),
                None if byte == b'>' => break,
                None => {}
            }
            if !byte.is_ascii_whitespace() {
                last = byte;
            }
            cursor += 1;
        }
        if !declaration {
            if closing {
                depth = depth.saturating_sub(1);
            } else {
                depth += 1;
                deepest = deepest.max(depth);
                if last == b'/' {
                    depth -= 1;
                }
            }
        }
        index = cursor + 1;
    }
    deepest
}

/// When `rest` starts with `opening`, the length up to and including the next `closing`, or to the
/// end of the text when there is none.
fn skipped(rest: &[u8], opening: &[u8], closing: &[u8]) -> Option<usize> {
    if !rest.starts_with(opening) {
        return None;
    }
    let from = opening.len();
    let found = rest[from..]
        .windows(closing.len())
        .position(|window| window == closing)
        .map(|at| from + at + closing.len());
    Some(found.unwrap_or(rest.len()))
}

/// Sets, replaces or removes one attribute of an element, keeping the order of the rest.
fn set_attribute(element: &mut Element, name: &str, value: Option<String>) {
    let at = element.attributes.iter().position(|(key, _)| key == name);
    match (at, value) {
        (Some(at), Some(value)) => element.attributes[at].1 = value,
        (Some(at), None) => {
            element.attributes.remove(at);
        }
        (None, Some(value)) => element.attributes.push((name.to_owned(), value)),
        (None, None) => {}
    }
}

/// Writes an element, indented by `depth`, laying out child elements one per line.
fn write_element(element: &Element, depth: usize, out: &mut String) {
    let mixed = element.has_element_child() && element.has_text_child();
    if mixed {
        // Adding whitespace around text would change the text when the document is read again,
        // so an element that mixes the two is written with no layout at all.
        write_compact(element, out);
        return;
    }
    for _ in 0..depth {
        out.push_str("  ");
    }
    write_open_tag(element, out);
    if element.children.is_empty() {
        out.truncate(out.len() - 1);
        out.push_str("/>");
        return;
    }
    if element.has_element_child() {
        for child in &element.children {
            if let Node::Element(child) = child {
                out.push('\n');
                write_element(child, depth + 1, out);
            }
        }
        out.push('\n');
        for _ in 0..depth {
            out.push_str("  ");
        }
    } else {
        out.push_str(&escape_text(&element.text()));
    }
    out.push_str("</");
    out.push_str(&element.name);
    out.push('>');
}

/// Writes an element with no added whitespace, used for mixed content.
fn write_compact(element: &Element, out: &mut String) {
    write_open_tag(element, out);
    if element.children.is_empty() {
        out.truncate(out.len() - 1);
        out.push_str("/>");
        return;
    }
    for child in &element.children {
        match child {
            Node::Element(child) => write_compact(child, out),
            Node::Text { text } => out.push_str(&escape_text(text)),
        }
    }
    out.push_str("</");
    out.push_str(&element.name);
    out.push('>');
}

fn write_open_tag(element: &Element, out: &mut String) {
    out.push('<');
    out.push_str(&element.name);
    for (name, value) in &element.attributes {
        out.push(' ');
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_attribute(value));
        out.push('"');
    }
    out.push('>');
}

/// Escapes text. A carriage return becomes a character reference: XML parsers turn a literal one
/// into a line feed, which would change the text when the document is read again.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            other => out.push(other),
        }
    }
    out
}

/// Escapes an attribute value. Every whitespace character but the space becomes a character
/// reference, because a parser turns literal ones into spaces.
fn escape_attribute(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            other => out.push(other),
        }
    }
    out
}
