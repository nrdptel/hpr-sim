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

/// The one component of a one-component document, the way a component reader will meet it.
fn component(xml: &str) -> Element {
    let document = Document::parse(&format!(
        r#"<openrocket version="1.10" creator="x">{xml}</openrocket>"#
    ))
    .expect("read the design");
    assert!(document.warnings.is_empty(), "{:?}", document.warnings);
    document
        .value
        .root
        .child("bodytube")
        .expect("a body tube")
        .clone()
}

/// Where a warning from one of these tests says it happened.
const AT: &str = "openrocket/rocket/bodytube";

/// Loft lesson L58: `auto 0.025` kept the number but lost the flag, so saving the design turned an
/// automatic dimension into a hand-typed one. Both halves are kept, and a bare `auto` is a
/// dimension with no cached number rather than a parse failure.
#[test]
fn auto_flag_kept_with_cached_value() {
    let tube = component(concat!(
        "<bodytube>",
        "<outerradius>auto 0.0125</outerradius>",
        "<innerradius>auto</innerradius>",
        "<thickness>0.0016</thickness>",
        "<length>  0.61  </length>",
        "<aftradius>auto  2.5e-2</aftradius>",
        "<foreradius>sometimes</foreradius>",
        "</bodytube>",
    ));
    let mut warnings = Vec::new();
    let mut read = Values::new(&tube, AT, &mut warnings);

    let outer = read.dimension(&["outerradius"]).expect("an outer radius");
    assert_eq!(
        outer,
        Dimension::Automatic {
            cached: Some(0.0125)
        }
    );
    assert!(outer.is_automatic());
    assert_eq!(outer.value(), Some(0.0125));

    // A bare `auto` is automatic with nothing cached — not a missing tag, and not an error.
    let inner = read.dimension(&["innerradius"]).expect("an inner radius");
    assert_eq!(inner, Dimension::Automatic { cached: None });
    assert_eq!(inner.value(), None);

    // A stated dimension is not automatic, and whitespace around either form is nothing.
    assert_eq!(
        read.dimension(&["thickness"]),
        Some(Dimension::Stated { value: 0.0016 })
    );
    assert_eq!(read.number(&["length"]), Some(0.61));
    assert_eq!(
        read.dimension(&["aftradius"]),
        Some(Dimension::Automatic {
            cached: Some(0.025)
        })
    );

    // A tag that is neither is dropped with a word about it, not silently taken as zero.
    assert_eq!(read.dimension(&["foreradius"]), None);
    assert_eq!(read.dimension(&["nosuchtag"]), None);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].kind, WarningKind::Dropped);
    assert_eq!(warnings[0].at, AT);
    assert!(
        warnings[0].message.contains("sometimes"),
        "{:?}",
        warnings[0]
    );
}

/// Loft lesson L62: OpenRocket renamed several tags and writes both names, and Loft read a stated
/// `0` as a missing value.
///
/// **Observed** by `cargo xtask ork`: OpenRocket writes both names of a pair the reader takes
/// either of on 751 elements of the reference corpus — 642 `position`/`axialoffset` and 109
/// `fincount`/`instancecount` — and on every one of them the two agree on the text *and* on the
/// `type`/`method` attribute that says what the number is measured from. So either name may be
/// read; this takes the newer.
#[test]
fn legacy_tags_equal_modern_and_zero_is_stated() {
    let tube = component(concat!(
        "<bodytube>",
        // Both names, agreeing, as OpenRocket writes them.
        r#"<position type="bottom">0.0</position><axialoffset method="bottom">0.0</axialoffset>"#,
        "<fincount>3</fincount><instancecount>3</instancecount>",
        // The legacy name alone, as an older file has it.
        "<radialdirection>60.0</radialdirection>",
        "<overridecd>0.0</overridecd>",
        "</bodytube>",
    ));
    let mut warnings = Vec::new();
    let mut read = Values::new(&tube, AT, &mut warnings);

    // A stated zero is a value. Reading it as missing is what charged a zero-drag part full drag.
    assert_eq!(read.number(&AXIAL_OFFSET), Some(0.0));
    assert_eq!(read.number(&["overridecd"]), Some(0.0));
    assert_ne!(read.number(&["overridecd"]), None);

    // The newer name wins, and carries the newer attribute; the older one says the same thing.
    let offset = read.element(&AXIAL_OFFSET).expect("an axial offset");
    assert_eq!(offset.name, "axialoffset");
    assert_eq!(offset.attribute("method"), Some("bottom"));
    assert_eq!(
        tube.child("position").expect("a position").text().trim(),
        offset.text().trim()
    );

    assert_eq!(read.count(&INSTANCE_COUNT), Some(3));
    // The older name alone is read as itself.
    let older = component("<bodytube><fincount>4</fincount></bodytube>");
    let mut older_warnings = Vec::new();
    assert_eq!(
        Values::new(&older, AT, &mut older_warnings).count(&INSTANCE_COUNT),
        Some(4)
    );
    assert!(older_warnings.is_empty(), "{older_warnings:?}");
    assert!(warnings.is_empty(), "{warnings:?}");

    // Two names that disagree is not something OpenRocket writes: the newer wins, and says so.
    let odd =
        component("<bodytube><axialoffset>0.1</axialoffset><position>0.2</position></bodytube>");
    let mut odd_warnings = Vec::new();
    let mut read = Values::new(&odd, AT, &mut odd_warnings);
    assert_eq!(read.number(&AXIAL_OFFSET), Some(0.1));
    assert_eq!(odd_warnings.len(), 1, "{odd_warnings:?}");
    assert!(
        odd_warnings[0].message.contains("two names"),
        "{:?}",
        odd_warnings[0]
    );

    // The same number measured from two different places is two different places, and agreeing on
    // the text hides it. On all 642 corpus elements that carry both, OpenRocket agrees on both.
    let framed = component(concat!(
        r#"<bodytube><axialoffset method="top">0.1</axialoffset>"#,
        r#"<position type="bottom">0.1</position></bodytube>"#,
    ));
    let mut frame_warnings = Vec::new();
    let mut read = Values::new(&framed, AT, &mut frame_warnings);
    assert_eq!(read.number(&AXIAL_OFFSET), Some(0.1));
    assert_eq!(frame_warnings.len(), 1, "{frame_warnings:?}");
    assert!(
        frame_warnings[0].message.contains("measure it from"),
        "{:?}",
        frame_warnings[0]
    );
}

/// `auto` is a word. `automatic` is an ignition event, not a dimension, and nothing glued to the
/// four letters is one either — every `auto`-prefixed text in the reference corpus is `auto`,
/// `auto <number>` or `automatic`.
#[test]
fn a_dimension_is_auto_a_number_or_nothing() {
    assert_eq!(
        Dimension::parse("auto"),
        Some(Dimension::Automatic { cached: None })
    );
    assert_eq!(
        Dimension::parse("auto 0.025"),
        Some(Dimension::Automatic {
            cached: Some(0.025)
        })
    );
    for word in [
        "automatic",
        "auto-1",
        "auto0.5",
        "burnout",
        "",
        "auto x",
        "inf",
        "NaN",
    ] {
        assert_eq!(Dimension::parse(word), None, "{word}");
    }
    assert_eq!(
        Dimension::parse("0.025"),
        Some(Dimension::Stated { value: 0.025 })
    );
}

/// A count is a whole number of things, and anything else is dropped with a word about it rather
/// than rounded, truncated or wrapped.
#[test]
fn a_count_that_is_not_a_whole_number_of_things_is_dropped() {
    for (text, expected) in [
        ("3", Some(3)),
        ("0", Some(0)),
        ("4294967295", Some(u32::MAX)),
    ] {
        let element = component(&format!("<bodytube><fincount>{text}</fincount></bodytube>"));
        let mut warnings = Vec::new();
        assert_eq!(
            Values::new(&element, AT, &mut warnings).count(&INSTANCE_COUNT),
            expected,
            "{text}"
        );
        assert!(warnings.is_empty(), "{text}: {warnings:?}");
    }
    for text in ["-1", "2.5", "4294967296", "1e300", "three"] {
        let element = component(&format!("<bodytube><fincount>{text}</fincount></bodytube>"));
        let mut warnings = Vec::new();
        assert_eq!(
            Values::new(&element, AT, &mut warnings).count(&INSTANCE_COUNT),
            None,
            "{text}"
        );
        assert_eq!(warnings.len(), 1, "{text}: {warnings:?}");
        assert_eq!(warnings[0].kind, WarningKind::Dropped, "{text}");
    }
}

/// A flag is `true` or `false`; anything else is dropped rather than read as false.
#[test]
fn a_flag_that_is_neither_true_nor_false_is_dropped() {
    let element = component(
        "<bodytube><overridesubcomponentsmass>yes</overridesubcomponentsmass></bodytube>",
    );
    let mut warnings = Vec::new();
    let overrides = Values::new(&element, AT, &mut warnings).overrides();
    assert_eq!(overrides.subcomponents_mass, None);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(warnings[0].kind, WarningKind::Dropped);
}

/// Loft lesson L63: Loft read neither `overridecd` nor the subcomponent flags, so a part set to a
/// drag coefficient of zero was still charged drag. Each value and each flag stands on its own.
#[test]
fn cd_and_cg_subcomponent_overrides_are_independent() {
    let tube = component(concat!(
        "<bodytube>",
        "<overridemass>0.25</overridemass>",
        "<overridecg>0.0</overridecg>",
        "<overridecd>0.0</overridecd>",
        "<overridesubcomponentsmass>true</overridesubcomponentsmass>",
        "<overridesubcomponentscg>false</overridesubcomponentscg>",
        "</bodytube>",
    ));
    let mut warnings = Vec::new();
    let mut read = Values::new(&tube, AT, &mut warnings);
    let overrides = read.overrides();

    assert_eq!(overrides.mass_kg, Some(0.25));
    // Zero is an override to zero, not an absent one.
    assert_eq!(overrides.cg_m, Some(0.0));
    assert_eq!(overrides.cd, Some(0.0));
    // One flag set does not set the others, and an absent flag is absent, not false.
    assert_eq!(overrides.subcomponents_mass, Some(true));
    assert_eq!(overrides.subcomponents_cg, Some(false));
    assert_eq!(overrides.subcomponents_cd, None);
    assert!(warnings.is_empty(), "{warnings:?}");

    // A component with nothing to say overrides nothing.
    let plain = component("<bodytube><length>0.61</length></bodytube>");
    let mut plain_warnings = Vec::new();
    let mut read = Values::new(&plain, AT, &mut plain_warnings);
    assert_eq!(read.overrides(), Overrides::default());
    assert!(plain_warnings.is_empty(), "{plain_warnings:?}");

    // Before schema 1.9 the three flags were one. 20 elements of the reference corpus carry it,
    // and none of them carries a per-quantity flag, so it is read as setting all three — out loud.
    let old = component(concat!(
        "<bodytube><overridemass>0.25</overridemass>",
        "<overridesubcomponents>true</overridesubcomponents></bodytube>",
    ));
    let mut old_warnings = Vec::new();
    let mut read = Values::new(&old, AT, &mut old_warnings);
    let overrides = read.overrides();
    assert_eq!(overrides.subcomponents_mass, Some(true));
    assert_eq!(overrides.subcomponents_cg, Some(true));
    assert_eq!(overrides.subcomponents_cd, Some(true));
    assert_eq!(old_warnings.len(), 1, "{old_warnings:?}");
    assert_eq!(old_warnings[0].kind, WarningKind::Unusual);
    assert!(
        old_warnings[0]
            .message
            .contains("the mass, the centre of gravity, the drag flags"),
        "{:?}",
        old_warnings[0]
    );

    // A per-quantity flag beside it wins, and the warning says only what the old flag did set.
    let mixed = component(concat!(
        "<bodytube><overridesubcomponents>true</overridesubcomponents>",
        "<overridesubcomponentscd>false</overridesubcomponentscd></bodytube>",
    ));
    let mut mixed_warnings = Vec::new();
    let overrides = Values::new(&mixed, AT, &mut mixed_warnings).overrides();
    assert_eq!(overrides.subcomponents_mass, Some(true));
    assert_eq!(overrides.subcomponents_cg, Some(true));
    assert_eq!(overrides.subcomponents_cd, Some(false));
    assert_eq!(mixed_warnings.len(), 1, "{mixed_warnings:?}");
    assert!(
        mixed_warnings[0]
            .message
            .contains("the mass, the centre of gravity flags"),
        "{:?}",
        mixed_warnings[0]
    );
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

/// A two-stage design whose second stage opens with a transition of automatic fore radius.
const ACROSS_A_STAGE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket>
    <name>Two stage</name>
    <referencetype>maximum</referencetype>
    <subcomponents>
      <stage>
        <name>Sustainer</name>
        <subcomponents>
          <nosecone>
            <name>Nose</name>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.3</length><thickness>0.002</thickness>
            <shape>ogive</shape><shapeparameter>1.0</shapeparameter>
            <aftradius>auto</aftradius>
          </nosecone>
          <bodytube>
            <name>Upper tube</name>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.6</length><thickness>0.002</thickness>
            <radius>0.05</radius>
          </bodytube>
        </subcomponents>
      </stage>
      <stage>
        <name>Booster</name>
        <subcomponents>
          <transition>
            <name>Shoulder up</name>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.1</length><thickness>0.002</thickness>
            <shape>conical</shape>
            <foreradius>auto</foreradius>
            <aftradius>0.08</aftradius>
          </transition>
          <bodytube>
            <name>Booster tube</name>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.5</length><thickness>0.002</thickness>
            <radius>auto</radius>
          </bodytube>
        </subcomponents>
      </stage>
    </subcomponents>
  </rocket>
</openrocket>
"#;

/// Reads a design document's spine, failing the test on any warning.
fn spine(xml: &str) -> hpr_design::tree::Rocket {
    let read = read(xml.as_bytes()).expect("a readable design");
    assert!(read.warnings.is_empty(), "container: {:?}", read.warnings);
    let spine = component::rocket(&read.value.document);
    assert!(spine.warnings.is_empty(), "spine: {:?}", spine.warnings);
    spine.value
}

/// [Loft lesson L59][lessons]: Loft resolved an automatic radius among a stage's own components
/// only, so the first component of a booster — which takes its radius from the stage ahead of it —
/// came out as whatever it had cached, or as zero. OpenRocket's own two-stage designs rely on the
/// resolution crossing that boundary.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
#[test]
fn auto_fore_radius_resolves_across_stage_boundary() {
    let rocket = spine(ACROSS_A_STAGE);
    assert_eq!(rocket.stages.len(), 2);
    let transition = &rocket.stages[1].components[0];
    assert_eq!(
        transition.auto,
        vec![hpr_design::tree::AutoDimension::ForeRadius]
    );

    let layout = rocket.layout().expect("a design that lays out");
    let (_, placed) = layout.find(&transition.id).expect("the transition");
    let hpr_design::tree::Part::Transition(resolved) = &placed.part else {
        panic!("a transition");
    };
    // The tube at the end of the stage ahead, not the 0 the file leaves cached.
    assert!(
        (resolved.fore_radius_m - 0.05).abs() < 1e-12,
        "{resolved:?}"
    );
    // And the tube behind it takes the transition's aft radius, forward to aft.
    let (_, tube) = layout
        .find(&rocket.stages[1].components[1].id)
        .expect("the booster tube");
    let hpr_design::tree::Part::BodyTube(tube) = &tube.part else {
        panic!("a body tube");
    };
    assert!((tube.outer_radius_m - 0.08).abs() < 1e-12, "{tube:?}");
}

/// OpenRocket's ogive parameter is `κ = ρ_tangent/ρ` (Niskanen, appendix A, equation A.3) and
/// `hpr-design` states the same shape as `ρ/ρ_tangent`, so reading one as the other would turn
/// every secant ogive into a bulged one. `κ = 0` is an infinite radius, which is a cone.
#[test]
fn ogive_parameter_is_read_as_its_reciprocal() {
    for (kappa, expected) in [("1.0", 1.0), ("0.5", 2.0), ("0.8", 1.25)] {
        let xml = ACROSS_A_STAGE.replace(
            "<shapeparameter>1.0</shapeparameter>",
            &format!("<shapeparameter>{kappa}</shapeparameter>"),
        );
        let rocket = spine(&xml);
        let hpr_design::tree::Part::NoseCone(nose) = &rocket.stages[0].components[0].part else {
            panic!("a nose cone");
        };
        assert_eq!(
            nose.shape,
            hpr_design::shapes::NoseShape::Ogive {
                radius_ratio: expected
            },
            "κ = {kappa}"
        );
    }
    let xml = ACROSS_A_STAGE.replace(
        "<shapeparameter>1.0</shapeparameter>",
        "<shapeparameter>0</shapeparameter>",
    );
    let hpr_design::tree::Part::NoseCone(nose) = &spine(&xml).stages[0].components[0].part else {
        panic!("a nose cone");
    };
    assert_eq!(nose.shape, hpr_design::shapes::NoseShape::Conical {});
}

/// `<thickness>filled</thickness>` is a solid part, and a wall as thick as the part is the same
/// thing. Reading `filled` as a number would leave the part weightless.
#[test]
fn a_filled_part_is_solid() {
    let xml = ACROSS_A_STAGE.replace(
        "<length>0.3</length><thickness>0.002</thickness>",
        "<length>0.3</length><thickness>filled</thickness>",
    );
    let hpr_design::tree::Part::NoseCone(nose) = &spine(&xml).stages[0].components[0].part else {
        panic!("a nose cone");
    };
    assert_eq!(nose.wall, hpr_design::solids::Wall::Filled {});
}

/// A part no milestone has reached yet is counted and named, not silently dropped. A pod set
/// carries a spine of its own, which is M3.1c's work.
#[test]
fn parts_no_milestone_reads_yet_are_reported_not_dropped() {
    let xml = ACROSS_A_STAGE.replace(
        "</bodytube>",
        "<subcomponents><podset><name>Pods</name></podset></subcomponents></bodytube>",
    );
    let read = read(xml.as_bytes()).expect("a readable design");
    let spine = component::rocket(&read.value.document);
    let warnings: Vec<&str> = spine.warnings.iter().map(|w| w.message.as_str()).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("2 `podset`"), "{warnings:?}");
    assert_eq!(spine.count(WarningKind::Skipped), 1);
}

/// A wall of no thickness, and no wall tag at all, are the same thing: a solid part. Read as a
/// wall of zero, the part is weightless and `hpr-design` refuses the whole design — so the rule a
/// shoulder gets applies to a body component too, and either way the reader says so.
#[test]
fn a_wall_of_no_thickness_is_solid_and_says_so() {
    for (from, to) in [
        (
            "<thickness>0.002</thickness>\n            <shape>ogive",
            "<thickness>0</thickness>\n            <shape>ogive",
        ),
        (
            "<thickness>0.002</thickness>\n            <shape>ogive",
            "<shape>ogive",
        ),
    ] {
        let xml = ACROSS_A_STAGE.replacen(from, to, 1);
        let read = read(xml.as_bytes()).expect("a readable design");
        let spine = component::rocket(&read.value.document);
        let hpr_design::tree::Part::NoseCone(nose) = &spine.value.stages[0].components[0].part
        else {
            panic!("a nose cone");
        };
        assert_eq!(nose.wall, hpr_design::solids::Wall::Filled {});
        assert_eq!(spine.count(WarningKind::Unusual), 1, "{:?}", spine.warnings);
        assert!(spine.value.layout().is_ok());
    }
}

/// A single-stage design carrying one of most of the parts that hang off a spine: a motor tube, a
/// ring whose bore and outer radius are both automatic, a coupler with a bulkhead nested inside
/// it, a canted fin set with a tab, and a parachute packed to fill the bore.
const WITH_PARTS_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket>
    <name>Parts</name>
    <referencetype>maximum</referencetype>
    <subcomponents>
      <stage>
        <name>Sustainer</name>
        <id>stage</id>
        <subcomponents>
          <nosecone>
            <name>Nose</name><id>nose</id>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.3</length><thickness>0.002</thickness>
            <shape>ogive</shape><shapeparameter>1.0</shapeparameter>
            <aftradius>0.05</aftradius>
            <finish>smooth</finish>
          </nosecone>
          <bodytube>
            <name>Tube</name><id>tube</id>
            <material type="bulk" density="680.0">Cardboard</material>
            <length>0.6</length><thickness>0.002</thickness>
            <radius>0.05</radius>
            <finish>normal</finish>
            <subcomponents>
{parts}
            </subcomponents>
          </bodytube>
        </subcomponents>
      </stage>
    </subcomponents>
  </rocket>
</openrocket>
"#;

/// The parts inside the body tube of [`WITH_PARTS_TEMPLATE`], each one complete on its own.
const PARTS: [&str; 5] = [
    r#"              <innertube>
                <name>Mount</name><id>mount</id>
                <material type="bulk" density="680.0">Cardboard</material>
                <axialoffset method="bottom">0.0</axialoffset>
                <length>0.2</length><outerradius>0.0095</outerradius><thickness>0.0005</thickness>
                <radialposition>0.0</radialposition><radialdirection>0.0</radialdirection>
              </innertube>"#,
    r#"              <centeringring>
                <name>Ring</name><id>ring</id>
                <material type="bulk" density="680.0">Plywood</material>
                <axialoffset method="bottom">0.0</axialoffset>
                <length>0.005</length>
                <outerradius>auto</outerradius><innerradius>auto 0.0095</innerradius>
              </centeringring>"#,
    r#"              <tubecoupler>
                <name>Coupler</name><id>coupler</id>
                <material type="bulk" density="680.0">Cardboard</material>
                <axialoffset method="top">0.05</axialoffset>
                <length>0.1</length><outerradius>auto</outerradius><thickness>0.0015</thickness>
                <subcomponents>
                  <bulkhead>
                    <name>Bulkhead</name><id>bulkhead</id>
                    <material type="bulk" density="680.0">Plywood</material>
                    <axialoffset method="top">0.0</axialoffset>
                    <length>0.003</length><outerradius>auto</outerradius>
                  </bulkhead>
                </subcomponents>
              </tubecoupler>"#,
    r#"              <trapezoidfinset>
                <name>Fins</name><id>fins</id>
                <material type="bulk" density="680.0">Plywood</material>
                <axialoffset method="bottom">0.0</axialoffset>
                <instancecount>3</instancecount>
                <rootchord>0.12</rootchord><tipchord>0.06</tipchord>
                <height>0.07</height><sweeplength>0.04</sweeplength>
                <thickness>0.003</thickness><crosssection>rounded</crosssection>
                <cant>1.0</cant>
                <angleoffset method="relative">180.0</angleoffset>
                <radiusoffset method="surface">0.0</radiusoffset>
                <finish>polished</finish>
                <tabheight>0.01</tabheight><tablength>0.03</tablength>
                <tabposition relativeto="center">0.0</tabposition>
              </trapezoidfinset>"#,
    r#"              <parachute>
                <name>Chute</name><id>chute</id>
                <axialoffset method="top">0.1</axialoffset>
                <packedlength>0.08</packedlength><packedradius>auto</packedradius>
                <radialposition>0.0</radialposition><radialdirection>0.0</radialdirection>
                <material type="surface" density="0.06">Ripstop nylon</material>
                <diameter>0.6</diameter>
                <linecount>6</linecount><linelength>0.7</linelength>
                <linematerial type="line" density="0.0016">Paracord</linematerial>
              </parachute>"#,
];

/// The design with its parts written in the order `order` gives.
fn with_parts(order: [usize; 5]) -> String {
    let parts: Vec<&str> = order.into_iter().map(|k| PARTS[k]).collect();
    WITH_PARTS_TEMPLATE.replace("{parts}", &parts.join("\n"))
}

/// The radius `.ork` tag `dimension` resolved to on the component `id`, after the layout.
fn resolved(layout: &hpr_design::tree::Layout, id: &str) -> hpr_design::tree::Part {
    let (_, placed) = layout
        .find(id)
        .unwrap_or_else(|| panic!("no component `{id}`"));
    placed.part.clone()
}

/// [Loft lesson L60][lessons]: Loft resolved automatic dimensions as it walked the tree, so a ring
/// whose bore comes from the motor tube beside it got the right answer only when the tube happened
/// to be written first, and a bulkhead inside a coupler — two levels of automatic radius — stayed
/// `NaN`. Nothing here is resolved while walking: the reader marks the dimension and
/// `Rocket::layout` resolves every one of them afterwards, in a pass that cannot see file order.
///
/// So the same design written with its parts in the opposite order must lay out to the same
/// numbers, and every one of them must be a real number.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
#[test]
fn auto_resolution_is_order_independent_and_finite() {
    let forwards = spine(&with_parts([0, 1, 2, 3, 4]))
        .layout()
        .expect("a design that lays out");

    // The bore of the tube is 0.05 - 0.002; the coupler fills it, and the bulkhead fills the
    // coupler. Two levels of automatic radius, which is the half of L60 that stayed NaN.
    let hpr_design::tree::Part::InnerTube(coupler) = resolved(&forwards, "coupler") else {
        panic!("a coupler");
    };
    assert!(
        (coupler.outer_radius_m - 0.048).abs() < 1e-12,
        "{coupler:?}"
    );
    let hpr_design::tree::Part::CenteringRing(bulkhead) = resolved(&forwards, "bulkhead") else {
        panic!("a bulkhead");
    };
    assert!(
        (bulkhead.outer_radius_m - (0.048 - 0.0015)).abs() < 1e-12,
        "{bulkhead:?}"
    );

    for placed in &forwards.components {
        for value in [
            placed.part.fore_radius_m(),
            placed.part.aft_radius_m(),
            placed.part.outer_radius_about_axis_m(),
            placed.part.inner_radius_m(),
        ]
        .into_iter()
        .flatten()
        {
            assert!(value.is_finite(), "{}: {value}", placed.id);
        }
        assert!(placed.own.mass_kg.is_finite(), "{}", placed.id);
    }

    // The same design with the tube's parts written in the opposite order. None of them is placed
    // `after` a sibling, so file order is the only thing that changed.
    let backwards = spine(&with_parts([4, 3, 2, 1, 0]))
        .layout()
        .expect("a design that lays out");

    assert_eq!(backwards.components.len(), forwards.components.len());
    for id in ["mount", "ring", "coupler", "bulkhead", "fins", "chute"] {
        assert_eq!(resolved(&backwards, id), resolved(&forwards, id), "{id}");
    }
    assert!(
        (backwards.structure.mass_kg - forwards.structure.mass_kg).abs() < 1e-15,
        "{} vs {}",
        backwards.structure.mass_kg,
        forwards.structure.mass_kg
    );
}

/// [Loft lesson L61][lessons], both halves. A centering ring whose bore is automatic weighed 0 g
/// in Loft, because the bore came out equal to the ring; and a stated wall was thrown away
/// whenever the radius it sat in was automatic, because the wall was judged against a radius that
/// was not known yet.
///
/// The oracle for the first half is in the file: `auto 0.0095` is the bore OpenRocket itself last
/// worked out, so the resolution can be held to it. Over the reference corpus the same check runs
/// on every automatic dimension that caches a number — `cargo xtask ork` reports how many agree.
///
/// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
#[test]
fn auto_ring_bore_and_stated_wall_match_oracle() {
    let rocket = spine(&with_parts([0, 1, 2, 3, 4]));
    let layout = rocket.layout().expect("a design that lays out");

    // The bore OpenRocket cached, worked out again from the motor tube beside the ring.
    let hpr_design::tree::Part::CenteringRing(ring) = resolved(&layout, "ring") else {
        panic!("a ring");
    };
    assert!((ring.inner_radius_m - 0.0095).abs() < 1e-12, "{ring:?}");
    assert!((ring.outer_radius_m - 0.048).abs() < 1e-12, "{ring:?}");
    // And so the ring weighs what a ring weighs, rather than nothing.
    let (_, placed) = layout.find("ring").expect("the ring");
    let annulus = std::f64::consts::PI * (0.048 * 0.048 - 0.0095 * 0.0095) * 0.005;
    assert!(
        (placed.own.mass_kg - 680.0 * annulus).abs() < 1e-12,
        "{} kg",
        placed.own.mass_kg
    );

    // The coupler's wall is stated and its outer radius is automatic. The wall survives.
    let hpr_design::tree::Part::InnerTube(coupler) = resolved(&layout, "coupler") else {
        panic!("a coupler");
    };
    assert!((coupler.thickness_m - 0.0015).abs() < 1e-12, "{coupler:?}");
}

/// A `.ork` writes angles in degrees and `hpr-design` holds them in radians, and nothing in the
/// file says which it is: read as radians, 180 is more than twenty-eight turns, not half of one.
#[test]
fn angles_are_degrees_not_radians() {
    let hpr_design::tree::Part::FinSet(fins) = fins(&with_parts([0, 1, 2, 3, 4])) else {
        panic!("a fin set");
    };
    assert!(
        (fins.base_angle_rad - std::f64::consts::PI).abs() < 1e-12,
        "{fins:?}"
    );
    assert!(
        (fins.cant_rad - 1.0_f64.to_radians()).abs() < 1e-12,
        "{fins:?}"
    );
    assert_eq!(fins.count, 3);
    assert_eq!(fins.cross_section, hpr_design::FinCrossSection::Rounded);
    // The tab is measured from the fin's centre in the file and from its leading edge here.
    assert_eq!(
        fins.tab,
        Some(hpr_design::FinTab {
            height_m: 0.01,
            length_m: 0.03,
            offset_m: 0.5 * (0.12 - 0.03),
        })
    );
}

/// A fin tab's place along the root is written from the fin's front, its centre or its end, under
/// either of two vocabularies, and `hpr-design` states one thing: the distance from the root
/// leading edge. A non-zero centre offset is what the corpus actually writes (eight fin sets), and
/// `end` is written nowhere at all, so both are only held up here.
#[test]
fn a_fin_tab_is_placed_from_the_root_leading_edge_whichever_way_the_file_measures_it() {
    let root = 0.12;
    let tab = 0.03;
    for (relative_to, offset, expected) in [
        ("front", 0.0, 0.0),
        ("front", 0.006, 0.006),
        ("top", 0.006, 0.006),
        ("center", 0.0, 0.5 * (root - tab)),
        ("center", -0.0022, 0.5 * (root - tab) - 0.0022),
        ("middle", -0.0022, 0.5 * (root - tab) - 0.0022),
        ("end", 0.0, root - tab),
        ("bottom", -0.004, root - tab - 0.004),
    ] {
        let xml = with_parts([0, 1, 2, 3, 4]).replace(
            "<tabposition relativeto=\"center\">0.0</tabposition>",
            &format!("<tabposition relativeto=\"{relative_to}\">{offset}</tabposition>"),
        );
        let hpr_design::tree::Part::FinSet(fins) = fins(&xml) else {
            panic!("a fin set");
        };
        let placed = fins.tab.expect("a tab").offset_m;
        assert!(
            (placed - expected).abs() < 1e-12,
            "`{relative_to}` at {offset}: {placed} m, wanted {expected} m"
        );
    }
}

/// The five words OpenRocket writes for a surface finish, as roughness heights. The numbers are
/// sourced on `hpr_io::ork::attached::finish` and on the guide's `.ork` page.
#[test]
fn a_surface_finish_is_a_roughness_height() {
    let rocket = spine(&with_parts([0, 1, 2, 3, 4]));
    let tube = &rocket.stages[0].components[1];
    assert_eq!(
        rocket.stages[0].components[0].finish,
        Some(hpr_design::Finish::Custom { roughness_m: 20e-6 })
    );
    assert_eq!(
        tube.finish,
        Some(hpr_design::Finish::Custom { roughness_m: 60e-6 })
    );
    let fins = tube
        .children
        .iter()
        .find(|child| child.id == "fins")
        .expect("the fins");
    assert_eq!(
        fins.finish,
        Some(hpr_design::Finish::Custom { roughness_m: 2e-6 })
    );

    // A word with no sourced roughness takes hpr's default, and says so.
    let xml =
        with_parts([0, 1, 2, 3, 4]).replace("<finish>normal</finish>", "<finish>anodised</finish>");
    let read = read(xml.as_bytes()).expect("a readable design");
    let imported = component::rocket(&read.value.document);
    assert_eq!(imported.value.stages[0].components[1].finish, None);
    assert_eq!(imported.count(WarningKind::Unusual), 1);
}

/// A part this reader cannot give an honest shape to is left out with its reason, rather than
/// guessed at or silently dropped. Each of these is a real shape in the reference corpus.
#[test]
fn a_part_that_cannot_be_read_honestly_is_left_out_with_its_reason() {
    let cases: [(&str, &str, &str); 3] = [
        // Fins on a nose cone: hpr attaches an external part to a body tube and nothing else.
        (
            "<finish>smooth</finish>",
            "<finish>smooth</finish><subcomponents><trapezoidfinset><name>Winglets</name>\
             <axialoffset method=\"bottom\">0.0</axialoffset><instancecount>3</instancecount>\
             <rootchord>0.05</rootchord><tipchord>0.02</tipchord><height>0.03</height>\
             <sweeplength>0.01</sweeplength><thickness>0.002</thickness>\
             <material type=\"bulk\" density=\"680.0\">Plywood</material></trapezoidfinset>\
             </subcomponents>",
            "hpr attaches one only to a body tube",
        ),
        // A freeform outline that leaves the body at its trailing edge.
        (
            "<trapezoidfinset>",
            "<freeformfinset><name>Winglet</name>\
             <axialoffset method=\"bottom\">0.0</axialoffset><instancecount>3</instancecount>\
             <thickness>0.002</thickness>\
             <material type=\"bulk\" density=\"680.0\">Plywood</material>\
             <finpoints><point x=\"0.0\" y=\"0.0\"/><point x=\"0.05\" y=\"0.04\"/>\
             <point x=\"0.1\" y=\"0.002\"/></finpoints></freeformfinset>\
             <trapezoidfinset>",
            "does not run from the root leading edge",
        ),
        // A tube fin set sized from the body, which hpr has no rule for yet.
        (
            "<trapezoidfinset>",
            "<tubefinset><name>Tubes</name><axialoffset method=\"bottom\">0.0</axialoffset>\
             <instancecount>6</instancecount><length>0.1</length><radius>auto</radius>\
             <thickness>0.001</thickness>\
             <material type=\"bulk\" density=\"680.0\">Cardboard</material></tubefinset>\
             <trapezoidfinset>",
            "hpr does not resolve that yet",
        ),
    ];
    for (from, to, says) in cases {
        let xml = with_parts([0, 1, 2, 3, 4]).replacen(from, to, 1);
        let read = read(xml.as_bytes()).expect("a readable design");
        let imported = component::rocket(&read.value.document);
        let said: Vec<&str> = imported
            .warnings
            .iter()
            .map(|warning| warning.message.as_str())
            .collect();
        assert!(
            said.iter().any(|message| message.contains(says)),
            "expected `{says}`, got {said:?}"
        );
        assert_eq!(imported.count(WarningKind::Skipped), 1, "{said:?}");
        // The rest of the design still opens, and still lays out.
        imported.value.layout().expect("a design that lays out");
    }
}

/// The fin set of a design read from `xml`.
fn fins(xml: &str) -> hpr_design::tree::Part {
    spine(xml).stages[0].components[1]
        .children
        .iter()
        .find(|child| child.id == "fins")
        .expect("the fins")
        .part
        .clone()
}

/// Three ways a reader can lose a number without saying so, all found reviewing the spine
/// ([#130][i130], [#131][i131] and [#132][i132]) and all fixed by the milestone that attaches
/// parts, because that is when each of them starts to change a mass.
///
/// [i130]: https://github.com/nrdptel/hpr-sim/issues/130
/// [i131]: https://github.com/nrdptel/hpr-sim/issues/131
/// [i132]: https://github.com/nrdptel/hpr-sim/issues/132
#[test]
fn a_number_is_never_lost_in_silence() {
    // A shoulder's stated wall survives an automatic shoulder radius: the wall must not be judged
    // against a radius that has not resolved yet.
    let xml = with_parts([0, 1, 2, 3, 4]).replace(
        "<aftradius>0.05</aftradius>",
        "<aftradius>0.05</aftradius><aftshoulderlength>0.06</aftshoulderlength>\
         <aftshoulderradius>auto</aftshoulderradius>\
         <aftshoulderthickness>0.002</aftshoulderthickness>",
    );
    let rocket = spine(&xml);
    let hpr_design::tree::Part::NoseCone(nose) = &rocket.stages[0].components[0].part else {
        panic!("a nose cone");
    };
    let shoulder = nose.shoulder.as_ref().expect("a shoulder");
    assert!((shoulder.thickness_m - 0.002).abs() < 1e-12, "{shoulder:?}");
    // And the layout gives it the tube's bore, wall and all.
    let layout = rocket.layout().expect("a design that lays out");
    let hpr_design::tree::Part::NoseCone(nose) = resolved(&layout, "nose") else {
        panic!("a nose cone");
    };
    let shoulder = nose.shoulder.expect("a shoulder");
    assert!(
        (shoulder.outer_radius_m - 0.048).abs() < 1e-12,
        "{shoulder:?}"
    );
    assert!((shoulder.thickness_m - 0.002).abs() < 1e-12, "{shoulder:?}");

    // A radius the file simply does not give is read as zero, and says so rather than laying out
    // as a part with no width.
    let xml = with_parts([0, 1, 2, 3, 4]).replace("<aftradius>0.05</aftradius>", "");
    let opened = read(xml.as_bytes()).expect("a readable design");
    let imported = component::rocket(&opened.value.document);
    assert!(
        imported
            .warnings
            .iter()
            .any(|warning| warning.message == "no `aftradius`, so it was read as zero"),
        "{:?}",
        imported.warnings
    );

    // An id the file repeats, or one that collides with a name invented for a part that has none,
    // is made unique with a warning rather than left to make the whole design unopenable.
    let xml = with_parts([0, 1, 2, 3, 4]).replace("<id>ring</id>", "<id>mount</id>");
    let opened = read(xml.as_bytes()).expect("a readable design");
    let imported = component::rocket(&opened.value.document);
    assert!(
        imported
            .warnings
            .iter()
            .any(|warning| warning.message.contains("this one was called `mount-2`")),
        "{:?}",
        imported.warnings
    );
    imported
        .value
        .layout()
        .expect("a design that still lays out");

    // A warning names which part it is about, not just the tag.
    let xml =
        with_parts([0, 1, 2, 3, 4]).replace("<finish>normal</finish>", "<finish>gilt</finish>");
    let opened = read(xml.as_bytes()).expect("a readable design");
    let imported = component::rocket(&opened.value.document);
    assert_eq!(
        imported.warnings[0].at,
        "openrocket/rocket/stage[0]/bodytube[1]"
    );
}

/// Where a part sits along its parent is five words, and each one has to put it somewhere
/// different. Nothing in the corpus exercises `after` at all, and the stations below are worked
/// out by hand from the tube's own geometry rather than from the reader.
///
/// The tube runs from station 0.3 (behind a 0.3 m nose) to 0.9, and the ring is 0.005 m long.
#[test]
fn every_axial_offset_word_puts_a_part_somewhere_different() {
    let cases: [(&str, f64, f64); 6] = [
        // word, offset in the file, the station its forward end should land at
        ("top", 0.0, 0.3),
        ("top", 0.1, 0.4),
        ("middle", 0.0, 0.3 + 0.5 * (0.6 - 0.005)),
        ("bottom", 0.0, 0.9 - 0.005),
        ("bottom", -0.02, 0.9 - 0.005 - 0.02),
        ("absolute", 0.42, 0.42),
    ];
    for (word, offset, station) in cases {
        let xml = with_parts([1, 0, 2, 3, 4]).replace(
            "<axialoffset method=\"bottom\">0.0</axialoffset>\n                <length>0.005</length>",
            &format!("<axialoffset method=\"{word}\">{offset}</axialoffset><length>0.005</length>"),
        );
        let layout = spine(&xml).layout().expect("a design that lays out");
        let (_, placed) = layout.find("ring").expect("the ring");
        assert!(
            (placed.fore_station_m - station).abs() < 1e-12,
            "`{word}` at {offset}: {} m, wanted {station} m",
            placed.fore_station_m
        );
    }

    // `after` puts a part behind the sibling written before it, and no file in the reference
    // library uses it, so this is the only thing holding that arm up.
    let xml = with_parts([1, 0, 2, 3, 4]).replace(
        "<axialoffset method=\"bottom\">0.0</axialoffset>\n                <length>0.2</length>",
        "<axialoffset method=\"after\">0.01</axialoffset><length>0.2</length>",
    );
    let layout = spine(&xml).layout().expect("a design that lays out");
    let (_, ring) = layout.find("ring").expect("the ring");
    let (_, mount) = layout.find("mount").expect("the mount");
    // The ring is written first and sits at the tube's aft end; the mount follows it by 0.01 m.
    assert!(
        (ring.fore_station_m - (0.9 - 0.005)).abs() < 1e-12,
        "{ring:?}"
    );
    assert!(
        (mount.fore_station_m - (ring.fore_station_m + 0.005 + 0.01)).abs() < 1e-12,
        "{mount:?}"
    );
}

/// What OpenRocket 24.12 did with each of the oracle's designs:
/// `validation/fixtures/ork/openrocket-automatic-radius.json`, written by
/// `validation/oracles/openrocket/automatic_radius.py` (ADR-054).
fn automatic_radius_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../validation/fixtures/ork/openrocket-automatic-radius.json"
    ))
    .expect("the fixture is JSON")
}

/// A body component's radii, forward to aft, in the fixture's order: a nose cone's base, a tube's
/// outer radius, a transition's forward then aft radius.
fn body_radii(part: &hpr_design::tree::Part) -> Vec<f64> {
    use hpr_design::tree::Part;
    match part {
        Part::NoseCone(p) => vec![p.base_radius_m],
        Part::BodyTube(p) => vec![p.outer_radius_m],
        Part::Transition(p) => vec![p.fore_radius_m, p.aft_radius_m],
        _ => Vec::new(),
    }
}

/// An automatic radius with no fixed radius anywhere along its chain takes OpenRocket's default
/// radius: on every design the oracle ran, every radius hpr resolves is the one OpenRocket 24.12
/// resolved, bit for bit, except where OpenRocket answers −1 m — a nose cone's base or a
/// transition's forward radius looking at an automatic tube, which no geometry can take. Those
/// take the default too, and there are exactly three of them, so a change either way shows.
///
/// The fixture's cached numbers (`auto 0.04`) are ignored by OpenRocket, and so by hpr: the
/// number is an answer OpenRocket last wrote, never an input.
#[test]
fn a_radius_with_nothing_to_take_is_openrockets_default() {
    let fixture = automatic_radius_fixture();
    assert_eq!(fixture["openrocket"], "24.12");
    let mut compared = 0usize;
    let mut departures = 0usize;
    let mut defaulted = 0usize;
    for case in fixture["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let text = case["document"].as_str().expect("a document");
        let read = read(text.as_bytes()).expect("a readable design");
        let spine = component::rocket(&read.value.document);
        defaulted += spine
            .warnings
            .iter()
            .filter(|w| w.message.contains("OpenRocket's default radius"))
            .count();
        let layout = spine.value.layout().expect("a design that lays out");
        let ours: Vec<f64> = layout.body().flat_map(|c| body_radii(&c.part)).collect();
        let theirs: Vec<f64> = case["resolved"]
            .as_array()
            .expect("resolved radii")
            .iter()
            .flat_map(|c| {
                ["base", "outer", "fore", "aft"]
                    .iter()
                    .filter_map(|key| c[key].as_f64())
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(ours.len(), theirs.len(), "{name}");
        for (ours, theirs) in ours.iter().zip(&theirs) {
            compared += 1;
            if *theirs < 0.0 {
                departures += 1;
                assert_eq!(*ours, OPENROCKET_DEFAULT_RADIUS_M, "{name}");
            } else {
                assert_eq!(ours, theirs, "{name}");
            }
        }
    }
    assert_eq!((compared, departures), (17, 3));
    // Every radius but the control's two, and the two the chain case states outright.
    assert_eq!(defaulted, 13);

    // The two reference-library designs the question came from, when the oracle had them: the
    // database's parachute catalogue opens with four tubes at the default, and Loft's quirks
    // fixture does not open in OpenRocket at all.
    for run in fixture["library"].as_array().expect("library runs") {
        let file = run["file"].as_str().expect("a file");
        if file.ends_with("parachutes.ork") {
            let radii: Vec<f64> = run["resolved"]
                .as_array()
                .expect("resolved radii")
                .iter()
                .map(|c| c["outer"].as_f64().expect("an outer radius"))
                .collect();
            assert_eq!(radii, [OPENROCKET_DEFAULT_RADIUS_M; 4], "{file}");
        } else {
            assert_eq!(run["opens"], false, "{file}");
        }
    }
}

/// Where the neighbour rule does reach a fixed radius, nothing is defaulted and nothing warns: the
/// default is for a chain with nothing on it, not a fallback for an awkward one.
#[test]
fn a_chain_that_reaches_a_fixed_radius_takes_no_default() {
    let fixture = automatic_radius_fixture();
    let control = fixture["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["name"] == "a nose cone before a fixed tube")
        .expect("the control case");
    let rocket = spine(control["document"].as_str().expect("a document"));
    assert!(rocket.unresolvable_body_radii().is_empty());
    let layout = rocket.layout().expect("a design that lays out");
    let radii: Vec<f64> = layout.body().flat_map(|c| body_radii(&c.part)).collect();
    assert_eq!(radii, [0.03, 0.03]);
}

/// Debrief's demonstration file carries a rocket's name and a stored simulation, and nothing to
/// build. That is a document holding no design, said as such — not a design that fails to lay out.
/// A rocket holding only a part no milestone reads yet does hold something, so it says the other.
#[test]
fn a_rocket_with_nothing_in_it_holds_no_design() {
    let xml = r#"<openrocket version="1.10" creator="synthesized">
        <rocket><name>Demonstrator</name><comment>results only</comment></rocket>
        <simulations><simulation status="uptodate"><name>Demo</name>
          <flightdata maxaltitude="1599.72"/></simulation></simulations></openrocket>"#;
    let file = read(xml.as_bytes()).expect("a readable document");
    let spine = component::rocket(&file.value.document);
    assert!(spine.value.stages.is_empty());
    let messages: Vec<&str> = spine.warnings.iter().map(|w| w.message.as_str()).collect();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert!(messages[0].contains("holds no design"), "{messages:?}");
    assert_eq!(spine.count(WarningKind::Unusual), 1);
    assert!(spine.value.layout().is_err());

    let xml = r#"<openrocket version="1.10" creator="synthesized">
        <rocket><name>Pods only</name><subcomponents><podset><name>P</name></podset>
        </subcomponents></rocket></openrocket>"#;
    let file = read(xml.as_bytes()).expect("a readable document");
    let spine = component::rocket(&file.value.document);
    assert!(
        spine
            .warnings
            .iter()
            .all(|w| !w.message.contains("holds no design")),
        "{:?}",
        spine.warnings
    );
}
