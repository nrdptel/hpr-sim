//! Tests for the `.ork` container and document.

use std::io::Write as _;

use proptest::prelude::*;

use super::*;

/// A small but complete design document, as a `.ork` writes it.
const DESIGN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket>
    <name>Sounder</name>
    <subcomponents>
      <stage>
        <name>Sustainer</name>
        <subcomponents>
          <nosecone>
            <length>0.3</length>
            <thickness>auto 0.0025</thickness>
          </nosecone>
        </subcomponents>
      </stage>
    </subcomponents>
  </rocket>
</openrocket>
"#;

/// Packs `entries` into a zip archive the way OpenRocket does: deflated, in the order given.
fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in entries {
        writer.start_file(*name, options).expect("start an entry");
        writer.write_all(bytes).expect("write an entry");
    }
    writer.finish().expect("finish the archive").into_inner()
}

/// Gzips `bytes`, the container OpenRocket wrote before it moved to zip.
fn gzip_of(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).expect("gzip the design");
    encoder.finish().expect("finish the gzip stream")
}

#[test]
fn containers_are_told_apart_by_their_first_bytes() {
    assert_eq!(
        Container::sniff(&zip_of(&[("rocket.ork", DESIGN.as_bytes())])),
        Some(Container::Zip)
    );
    assert_eq!(
        Container::sniff(&gzip_of(DESIGN.as_bytes())),
        Some(Container::Gzip)
    );
    assert_eq!(Container::sniff(DESIGN.as_bytes()), Some(Container::Xml));
    // A byte-order mark and leading blank lines still leave XML.
    assert_eq!(
        Container::sniff(b"\xef\xbb\xbf\n  <openrocket/>"),
        Some(Container::Xml)
    );
    // An empty zip archive is still a zip archive.
    assert_eq!(
        Container::sniff(b"PK\x05\x06\0\0\0\0"),
        Some(Container::Zip)
    );
    assert_eq!(Container::sniff(b"not a rocket"), None);
    assert_eq!(Container::sniff(b""), None);
}

#[test]
fn the_same_design_reads_the_same_in_all_three_containers() {
    let zipped = read(&zip_of(&[("rocket.ork", DESIGN.as_bytes())])).expect("read the zip");
    let gzipped = read(&gzip_of(DESIGN.as_bytes())).expect("read the gzip");
    let raw = read(DESIGN.as_bytes()).expect("read the raw XML");

    assert_eq!(zipped.value.container, Container::Zip);
    assert_eq!(gzipped.value.container, Container::Gzip);
    assert_eq!(raw.value.container, Container::Xml);
    assert_eq!(zipped.value.document, gzipped.value.document);
    assert_eq!(zipped.value.document, raw.value.document);
    assert_eq!(zipped.value.design_entry.as_deref(), Some("rocket.ork"));
    assert_eq!(gzipped.value.design_entry, None);
    assert!(zipped.warnings.is_empty(), "{:?}", zipped.warnings);

    let document = &raw.value.document;
    assert_eq!(
        document.version,
        SchemaVersion {
            major: 1,
            minor: 10
        }
    );
    assert_eq!(document.creator.as_deref(), Some("OpenRocket 24.12"));
    let nose = document
        .root
        .child("rocket")
        .and_then(|rocket| rocket.child("subcomponents"))
        .and_then(|subcomponents| subcomponents.child("stage"))
        .and_then(|stage| stage.child("subcomponents"))
        .and_then(|subcomponents| subcomponents.child("nosecone"))
        .expect("the nose cone");
    assert_eq!(nose.child("length").expect("a length").text(), "0.3");
    // The `auto` prefix is still there to be read: keeping it is Loft lesson L58's whole point.
    assert_eq!(
        nose.child("thickness").expect("a thickness").text(),
        "auto 0.0025"
    );
}

#[test]
fn an_archives_other_entries_are_kept() {
    let curve = b"<engine-database>a thrust curve</engine-database>";
    let preview = b"\x89PNG\r\n\x1a\n and some pixels";
    let bytes = zip_of(&[
        ("rocket.ork", DESIGN.as_bytes()),
        ("thrustcurves/AeroTech_K550W.rse", curve),
        ("preview.png", preview),
    ]);

    let read = read(&bytes).expect("read the archive");
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    let names: Vec<&str> = read
        .value
        .attachments
        .iter()
        .map(|attachment| attachment.name.as_str())
        .collect();
    assert_eq!(names, ["thrustcurves/AeroTech_K550W.rse", "preview.png"]);
    // Byte for byte, so that reading the curve (Loft lesson L57) and writing the file back out
    // both have everything they need.
    assert_eq!(
        read.value
            .attachment("thrustcurves/AeroTech_K550W.rse")
            .expect("the curve")
            .bytes,
        curve
    );
    assert_eq!(
        read.value
            .attachment("preview.png")
            .expect("the preview")
            .bytes,
        preview
    );
}

#[test]
fn an_archive_without_rocket_ork_reads_the_first_candidate_and_says_so() {
    let bytes = zip_of(&[
        ("notes.txt", b"not a design"),
        ("Sounder.ork", DESIGN.as_bytes()),
        ("spare.xml", b"<openrocket version=\"1.10\"/>"),
    ]);
    let read = read(&bytes).expect("read the archive");
    assert_eq!(read.value.design_entry.as_deref(), Some("Sounder.ork"));
    assert_eq!(read.value.attachments.len(), 2);
    assert_eq!(read.count(WarningKind::Unusual), 1);
    assert_eq!(read.warnings[0].at, "Sounder.ork");
    assert!(
        read.warnings[0].message.contains("no `rocket.ork`"),
        "{:?}",
        read.warnings[0]
    );
}

#[test]
fn a_newer_schema_and_a_missing_creator_are_read_with_warnings() {
    let newer = DESIGN.replace(
        r#"version="1.10" creator="OpenRocket 24.12""#,
        r#"version="1.99""#,
    );
    let read = read(newer.as_bytes()).expect("read a newer schema");
    assert_eq!(
        read.value.document.version,
        SchemaVersion {
            major: 1,
            minor: 99
        }
    );
    assert!(!read.value.document.version.is_known());
    assert_eq!(read.value.document.creator, None);
    assert_eq!(read.count(WarningKind::Unusual), 2);
    assert!(
        read.warnings
            .iter()
            .all(|warning| warning.at == "openrocket")
    );
}

#[test]
fn schema_versions_are_read_as_major_and_minor() {
    assert_eq!(
        SchemaVersion::parse("1.4"),
        Ok(SchemaVersion { major: 1, minor: 4 })
    );
    assert!(SchemaVersion::parse("1.0").expect("1.0").is_known());
    assert!(
        SchemaVersion::parse(&format!("1.{MAX_KNOWN_MINOR}"))
            .expect("the newest known")
            .is_known()
    );
    assert!(!SchemaVersion::parse("2.0").expect("2.0").is_known());
    for bad in ["", "1", "1.", ".1", "1.x", "1.10.2", " 1.10", "-1.2"] {
        assert!(SchemaVersion::parse(bad).is_err(), "{bad} should not read");
    }
}

/// Loft lesson L56: containers are sniffed by magic bytes, and malformed input must error rather
/// than crash. Every input here is broken in a different way, and none of them may panic.
#[test]
fn malformed_inputs_error_not_panic() {
    let zipped = zip_of(&[("rocket.ork", DESIGN.as_bytes())]);
    let gzipped = gzip_of(DESIGN.as_bytes());
    let deep = {
        let mut xml = String::from(r#"<openrocket version="1.10">"#);
        for _ in 0..MAX_DEPTH + 1 {
            xml.push_str("<subcomponents>");
        }
        for _ in 0..MAX_DEPTH + 1 {
            xml.push_str("</subcomponents>");
        }
        xml.push_str("</openrocket>");
        xml
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("nothing at all", Vec::new()),
        ("a byte", vec![b'<']),
        ("plain text", b"this is not a rocket".to_vec()),
        ("a zip signature and nothing else", b"PK\x03\x04".to_vec()),
        ("a truncated zip", zipped[..zipped.len() / 2].to_vec()),
        (
            "a zip with its directory cut off",
            zipped[..zipped.len() - 8].to_vec(),
        ),
        ("a truncated gzip", gzipped[..gzipped.len() - 4].to_vec()),
        ("a gzip header over rubbish", {
            let mut bytes = vec![0x1f, 0x8b, 0x08, 0, 0, 0, 0, 0, 0, 0];
            bytes.extend_from_slice(b"not deflated");
            bytes
        }),
        (
            "a zip with no design in it",
            zip_of(&[("notes.txt", b"nothing here")]),
        ),
        ("an empty zip", zip_of(&[])),
        (
            "a design that is not UTF-8",
            zip_of(&[("rocket.ork", &[0xff, 0xfe, 0x00])]),
        ),
        (
            "XML that never closes",
            b"<openrocket version=\"1.10\">".to_vec(),
        ),
        ("XML rooted somewhere else", b"<RockSimDocument/>".to_vec()),
        (
            "a root with no version",
            b"<openrocket creator=\"someone\"/>".to_vec(),
        ),
        (
            "a version that is not a number",
            b"<openrocket version=\"ten\"/>".to_vec(),
        ),
        ("a design nested past the limit", deep.into_bytes()),
        ("a bomb of angle brackets", vec![b'<'; 64 * 1024]),
    ];

    for (what, bytes) in cases {
        let outcome = read(&bytes);
        assert!(
            outcome.is_err(),
            "{what} should be refused, but it read as {outcome:?}"
        );
        // The error prints, which is what a caller shows the user.
        assert!(!outcome.unwrap_err().to_string().is_empty(), "{what}");
    }

    // And every prefix of a real file is refused or read, never a crash.
    for cut in 0..zipped.len() {
        let _ = read(&zipped[..cut]);
    }
}

#[test]
fn nesting_is_counted_before_the_document_is_parsed() {
    let nested = |depth: usize, tag: &str| {
        let mut xml = String::from(r#"<openrocket version="1.10" creator="x">"#);
        for _ in 0..depth {
            xml.push_str(&format!("<{tag}>"));
        }
        for _ in 0..depth {
            xml.push_str(&format!("</{tag}>"));
        }
        xml.push_str("</openrocket>");
        xml
    };
    // The root counts as one level, so `MAX_DEPTH - 1` children of it are the deepest allowed.
    assert!(read(nested(MAX_DEPTH - 1, "s").as_bytes()).is_ok());
    assert_eq!(
        read(nested(MAX_DEPTH, "s").as_bytes()),
        Err(OrkError::TooDeep {
            limit: MAX_DEPTH,
            depth: MAX_DEPTH + 1
        })
    );

    // Comments, CDATA, processing instructions and quoted attributes hide tags from the count,
    // and a self-closing tag opens nothing. Hiding them is safe because a parser does not descend
    // into them either; the test below shows the scan never counts fewer levels than the tree has.
    let hidden = concat!(
        r#"<openrocket version="1.10" creator="x">"#,
        "<!-- <a><b><c> --><![CDATA[<d><e>]]><?pi <f><g>?>",
        r#"<rocket note="a > sign, and &lt;h&gt;"><tube/><tube /></rocket>"#,
        "</openrocket>",
    );
    let read = read(hidden.as_bytes()).expect("read the design");
    assert!(read.value.document.root.child("rocket").is_some());
    assert_eq!(read.count(WarningKind::Dropped), 2);
}

/// An element that holds text beside its child elements. OpenRocket writes one: a simulation's
/// `<warning>` prints its own message after its fields, in 48 elements of 19 files of the
/// reference corpus (`cargo xtask ork`). Both parts are kept, and neither is worth a warning.
#[test]
fn an_element_that_mixes_text_and_children_keeps_both() {
    let xml = concat!(
        r#"<openrocket version="1.10" creator="x"><rocket>"#,
        r#"<warning type="HighSpeedDeployment"><priority>NORMAL</priority>"#,
        "\n          Recovery device deployment at high speed (31.9 m/s)\n        ",
        r#"</warning></rocket></openrocket>"#,
    );
    let read = read(xml.as_bytes()).expect("read the design");
    let warning = read
        .value
        .document
        .root
        .child("rocket")
        .and_then(|rocket| rocket.child("warning"))
        .expect("a flight warning");
    assert_eq!(
        warning.text().trim(),
        "Recovery device deployment at high speed (31.9 m/s)"
    );
    assert_eq!(
        warning.child("priority").expect("a priority").text(),
        "NORMAL"
    );
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);

    // And it survives being written out: the text is not laid out again, which would change it.
    let once = read.value.document;
    let twice = Document::parse(&once.to_xml())
        .expect("read what was written")
        .value;
    assert_eq!(once, twice);
}

/// A comment, a processing instruction or a CDATA section splits a run of text in two, and the
/// writer has nowhere to put the split — it drops the comment that made it. So the reader joins
/// the pieces, and reading what was written gives the same document.
#[test]
fn text_split_by_a_comment_is_joined() {
    for xml in [
        r#"<openrocket version="1.10" creator="x"><a>foo<!--c-->bar</a></openrocket>"#,
        r#"<openrocket version="1.10" creator="x"><a>foo<?pi x?>bar</a></openrocket>"#,
        r#"<openrocket version="1.10" creator="x"><a>foo<![CDATA[bar]]></a></openrocket>"#,
    ] {
        let once = Document::parse(xml).expect("read the design").value;
        let a = once.root.child("a").expect("an a");
        assert_eq!(a.text(), "foobar", "{xml}");
        assert_eq!(a.children.len(), 1, "{xml}: {:?}", a.children);
        let twice = Document::parse(&once.to_xml())
            .expect("read what was written")
            .value;
        assert_eq!(once, twice, "{xml}");
    }
}

/// The two fields that are read off the root are written back onto it, so setting either changes
/// the file rather than being dropped without a word.
#[test]
fn the_version_and_creator_are_written_back_onto_the_root() {
    let mut document = Document::parse(DESIGN).expect("read the design").value;
    document.version = SchemaVersion {
        major: 1,
        minor: 11,
    };
    document.creator = None;
    let again = Document::parse(&document.to_xml())
        .expect("read what was written")
        .value;
    assert_eq!(
        again.version,
        SchemaVersion {
            major: 1,
            minor: 11
        }
    );
    assert_eq!(again.creator, None);
    assert_eq!(again.root.attribute("creator"), None);
    assert_eq!(again.root.attribute("version"), Some("1.11"));
}

/// XML namespaces are the one thing the tree does not keep, so it says so.
#[test]
fn a_namespaced_document_says_what_it_dropped() {
    let xml = concat!(
        r#"<or:openrocket xmlns:or="urn:x" version="1.10" creator="x">"#,
        r#"<or:rocket/></or:openrocket>"#,
    );
    let read = read(xml.as_bytes()).expect("read the design");
    assert!(read.value.document.root.child("rocket").is_some());
    assert_eq!(read.count(WarningKind::Dropped), 1);
    assert!(
        read.warnings[0].message.contains("namespaces"),
        "{:?}",
        read.warnings[0]
    );
}

/// A deflate stream can expand about a thousandfold, so an archive that unpacks to more than it
/// is allowed leaves the entry out rather than asking for the memory.
#[test]
fn an_archive_that_unpacks_too_far_is_refused_not_swallowed() {
    let big = vec![b'A'; 4 * 1024 * 1024];
    let bytes = zip_of(&[("rocket.ork", DESIGN.as_bytes()), ("pad.bin", &big)]);
    assert!(
        bytes.len() < 64 * 1024,
        "the bomb should be small on disk, not {} bytes",
        bytes.len()
    );

    // Room for the design and nothing else: the padding is left out, with a warning.
    let tight = container::unpack_within(&bytes, DESIGN.len() as u64).expect("read the design");
    assert_eq!(tight.value.design.len(), DESIGN.len());
    assert!(tight.value.attachments.is_empty());
    assert_eq!(tight.count(WarningKind::Skipped), 1);
    assert!(
        tight.warnings[0].message.contains("limit"),
        "{:?}",
        tight.warnings[0]
    );

    // No room even for the design: an error, not half a file.
    assert_eq!(container::unpack_within(&bytes, 8), Err(OrkError::NoDesign));

    // And the same for a gzip stream.
    assert_eq!(
        container::unpack_within(&gzip_of(&big), 1024),
        Err(OrkError::TooBig { limit: 1024 })
    );

    // The default budget reads it all.
    let whole = read(&bytes).expect("read the whole archive");
    assert_eq!(whole.value.attachments.len(), 1);
    assert!(whole.warnings.is_empty(), "{:?}", whole.warnings);
}

#[test]
fn a_comment_is_dropped_and_counted() {
    let xml = r#"<openrocket version="1.10" creator="x"><!-- a note --><rocket/></openrocket>"#;
    let read = read(xml.as_bytes()).expect("read the design");
    assert_eq!(read.count(WarningKind::Dropped), 1);
    assert_eq!(read.warnings[0].at, "openrocket");
}

#[test]
fn reading_writing_and_reading_again_gives_the_same_document() {
    let once = read(DESIGN.as_bytes())
        .expect("read the design")
        .value
        .document;
    let twice = Document::parse(&once.to_xml())
        .expect("read what was written")
        .value;
    assert_eq!(once, twice);
    assert_eq!(once.to_xml(), twice.to_xml());
}

#[test]
fn awkward_text_and_attributes_survive_being_written() {
    let xml = concat!(
        r#"<openrocket version="1.10" creator="a &quot;quoted&quot; name&#10;on two lines">"#,
        r#"<rocket><name>Ampersand &amp; &lt;angle&gt; and a return&#13;here</name>"#,
        r#"<comment>   leading and trailing   </comment>"#,
        r#"<empty/><blank></blank></rocket></openrocket>"#,
    );
    let once = Document::parse(xml).expect("read the design").value;
    let twice = Document::parse(&once.to_xml())
        .expect("read what was written")
        .value;
    assert_eq!(once, twice);

    let rocket = once.root.child("rocket").expect("a rocket");
    assert_eq!(
        rocket.child("name").expect("a name").text(),
        "Ampersand & <angle> and a return\rhere"
    );
    assert_eq!(
        rocket.child("comment").expect("a comment").text(),
        "   leading and trailing   "
    );
    assert_eq!(
        once.creator.as_deref(),
        Some("a \"quoted\" name\non two lines")
    );
    assert_eq!(rocket.child("empty").expect("an empty tag").children, []);
}

/// Text a `.ork` could hold: the characters that make writing awkward, and a few ordinary ones.
/// Everything XML 1.0 forbids outright (a NUL, say) is left out — such a document could not have
/// been read from a file in the first place.
fn any_text(length: std::ops::Range<usize>) -> impl Strategy<Value = String> {
    proptest::collection::vec(
        proptest::sample::select(vec![
            'a', 'Z', '7', ' ', '&', '<', '>', '"', '\'', '/', '\t', '\n', '\r', 'é', '—',
        ]),
        length,
    )
    .prop_map(|characters| characters.into_iter().collect())
}

/// Element names, attributes and text that a `.ork` could hold, kept shallow and few so that a
/// failing case is still readable.
fn any_element(depth: u32) -> impl Strategy<Value = Element> {
    let attributes = || {
        proptest::collection::vec(("[a-z]{1,4}", any_text(0..8)), 0..3).prop_map(
            |attributes: Vec<(String, String)>| {
                attributes
                    .into_iter()
                    .enumerate()
                    .map(|(index, (name, value))| (format!("{name}{index}"), value))
                    .collect::<Vec<_>>()
            },
        )
    };
    let leaf = ("[a-z][a-z0-9]{0,6}", attributes(), any_text(0..12)).prop_map(
        |(name, attributes, text)| Element {
            name,
            attributes,
            children: if text.is_empty() {
                Vec::new()
            } else {
                vec![Node::Text { text }]
            },
        },
    );
    leaf.prop_recursive(depth, 32, 3, move |inner| {
        (
            "[a-z][a-z0-9]{0,6}",
            attributes(),
            proptest::collection::vec(inner, 1..3),
            // Text beside child elements, which is what sends the writer down its compact path.
            proptest::option::of(any_text(0..6)),
        )
            .prop_map(|(name, attributes, children, text)| {
                let mut nodes: Vec<Node> = children.into_iter().map(Node::Element).collect();
                if let Some(text) = text {
                    nodes.insert(0, Node::Text { text });
                }
                Element {
                    name,
                    attributes,
                    children: nodes,
                }
            })
    })
}

proptest! {
    /// The guard is only worth anything if it never *under*counts: the scan runs before the text
    /// reaches a parser that would overflow the stack, so every level the parser will descend has
    /// to be a level the scan saw. The hazards are the four things a scan of text can misread —
    /// a comment, a CDATA section, a processing instruction, and a `>` inside a quoted attribute
    /// value — so the text here is built from them rather than from the writer's output, which
    /// escapes them all away.
    #[test]
    fn the_depth_scan_never_undercounts(
        openers in proptest::collection::vec(
            proptest::sample::select(vec![
                "<s>",
                "<s x=\">\">",
                "<s x='>'>",
                "<s x=\"a/\">",
                "<s x=\"--> ]]> ?>\">",
            ]),
            1..40,
        ),
        noise in proptest::collection::vec(
            proptest::sample::select(vec![
                "",
                "<!-- <a><b> -->",
                "<![CDATA[<c><d>]]>",
                "<?pi <e><f>?>",
                "<g/>",
                "<h />",
            ]),
            1..40,
        ),
    ) {
        let mut xml = String::from(r#"<openrocket version="1.10">"#);
        for (opener, noise) in openers.iter().zip(noise.iter().cycle()) {
            xml.push_str(noise);
            xml.push_str(opener);
        }
        for _ in &openers {
            xml.push_str("</s>");
        }
        xml.push_str("</openrocket>");

        let scanned = super::document::deepest_nesting(&xml);
        let parsed = roxmltree::Document::parse(&xml).expect("well-formed by construction");
        let deepest = parsed
            .descendants()
            .filter(|node| node.is_element())
            .map(|node| node.ancestors().filter(|a| a.is_element()).count())
            .max()
            .unwrap_or(0);
        prop_assert!(scanned >= deepest, "scan {scanned} < tree {deepest} in {xml}");
    }

    /// Whatever the tree, writing it and reading it back settles at once: the document written
    /// from a parsed document parses to that same document. That is what "lossless" means here.
    #[test]
    fn any_document_written_and_read_again_is_unchanged(root in any_element(4)) {
        // The root's attributes carry the version and the creator: the tree keeps everything the
        // file said, and `Document`'s two fields are that tree read.
        let mut attributes = vec![
            ("version".to_owned(), "1.10".to_owned()),
            ("creator".to_owned(), "proptest".to_owned()),
        ];
        attributes.extend(root.attributes);
        let document = Document {
            version: SchemaVersion { major: 1, minor: 10 },
            creator: Some("proptest".to_owned()),
            root: Element { name: "openrocket".to_owned(), attributes, children: root.children },
        };
        let once = Document::parse(&document.to_xml()).expect("read what was written").value;
        let twice = Document::parse(&once.to_xml()).expect("read it again").value;
        prop_assert_eq!(once, twice);
    }
}
