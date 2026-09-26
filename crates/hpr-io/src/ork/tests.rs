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

/// A wall of no thickness is a surface with no wall: the nose keeps its shape and weighs nothing.
/// A nose that writes no thickness at all has a 2 mm wall. Both are OpenRocket 24.12's readings,
/// measured on its probe designs (`validation/fixtures/ork/openrocket-conventions.json`, ADR-061),
/// so neither is an assumption to warn of.
#[test]
fn a_wall_of_no_thickness_weighs_nothing_and_an_unwritten_one_is_two_millimetres() {
    for (to, wall) in [
        (
            "<thickness>0</thickness>\n            <shape>ogive",
            hpr_design::solids::Wall::Shell { thickness_m: 0.0 },
        ),
        (
            "<shape>ogive",
            hpr_design::solids::Wall::Shell { thickness_m: 0.002 },
        ),
    ] {
        let from = "<thickness>0.002</thickness>\n            <shape>ogive";
        let xml = ACROSS_A_STAGE.replacen(from, to, 1);
        let read = read(xml.as_bytes()).expect("a readable design");
        let spine = component::rocket(&read.value.document);
        let hpr_design::tree::Part::NoseCone(nose) = &spine.value.stages[0].components[0].part
        else {
            panic!("a nose cone");
        };
        assert_eq!(nose.wall, wall);
        assert_eq!(spine.count(WarningKind::Unusual), 0, "{:?}", spine.warnings);
        let layout = spine.value.layout().expect("a layout");
        let nose_mass = layout.body().next().expect("the nose").own.mass_kg;
        assert_eq!(
            nose_mass == 0.0,
            wall == hpr_design::solids::Wall::Shell { thickness_m: 0.0 }
        );
    }
}

/// A negative wall, on a nose or on its shoulder, is no wall, and the reader says so for each; a
/// nose that writes no thickness and is no wider than OpenRocket's 2 mm default wall is solid.
#[test]
fn a_negative_wall_is_no_wall_and_a_narrow_default_one_is_solid() {
    let from = "<thickness>0.002</thickness>\n            <shape>ogive";
    let to = "<thickness>-0.001</thickness><aftshoulderlength>0.05</aftshoulderlength>\
              <aftshoulderradius>0.01</aftshoulderradius>\
              <aftshoulderthickness>-0.001</aftshoulderthickness>\n            <shape>ogive";
    let xml = ACROSS_A_STAGE.replacen(from, to, 1);
    let first = read(xml.as_bytes()).expect("a readable design");
    let spine = component::rocket(&first.value.document);
    let hpr_design::tree::Part::NoseCone(nose) = &spine.value.stages[0].components[0].part else {
        panic!("a nose cone");
    };
    assert_eq!(
        nose.wall,
        hpr_design::solids::Wall::Shell { thickness_m: 0.0 }
    );
    let shoulder = nose.shoulder.expect("a shoulder");
    assert_eq!((shoulder.thickness_m, shoulder.capped), (0.0, false));
    assert_eq!(spine.count(WarningKind::Dropped), 2, "{:?}", spine.warnings);

    let narrow = ACROSS_A_STAGE.replacen(from, "<shape>ogive", 1).replacen(
        "<aftradius>auto</aftradius>\n          </nosecone>",
        "<aftradius>0.0015</aftradius>\n          </nosecone>",
        1,
    );
    assert_ne!(narrow, ACROSS_A_STAGE.replacen(from, "<shape>ogive", 1));
    let second = read(narrow.as_bytes()).expect("a readable design");
    let spine = component::rocket(&second.value.document);
    let hpr_design::tree::Part::NoseCone(nose) = &spine.value.stages[0].components[0].part else {
        panic!("a nose cone");
    };
    assert_eq!(nose.wall, hpr_design::solids::Wall::Filled {});
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

/// A part inside an inner tube set off the body's axis is read from the body's axis, where
/// OpenRocket places it from the tube's (#181); the reader says so, and says nothing for a tube on
/// the axis.
#[test]
fn a_part_inside_an_off_axis_tube_is_said_out_loud() {
    let block = "<subcomponents><engineblock><name>Block</name><id>block</id>\
        <axialoffset method=\"top\">0.0</axialoffset><length>0.01</length>\
        <outerradius>auto</outerradius><thickness>0.002</thickness></engineblock>\
        </subcomponents></innertube>";
    let warned = |radial: &str| {
        let xml = with_parts([0, 1, 2, 3, 4])
            .replacen("</innertube>", block, 1)
            .replacen(
                "<radialposition>0.0</radialposition>",
                &format!("<radialposition>{radial}</radialposition>"),
                1,
            );
        let read = read(xml.as_bytes()).expect("a readable design");
        let imported = component::rocket(&read.value.document);
        imported
            .warnings
            .iter()
            .filter(|w| w.message.contains("#181"))
            .map(|w| w.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(warned("0.0"), []);
    assert_eq!(warned("0.004"), [WarningKind::Unusual]);
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

/// A packed part whose file writes no packed size is packed as OpenRocket 24.12 packs it, 25 mm
/// long and 12.5 mm in radius, each on its own, and nothing is said: it is the file's meaning, not
/// a guess ([ADR-063][adr-063]; the probes are `hpr_validate::openrocket`'s).
///
/// [adr-063]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-063-packed-parts-read-and-weighed-as-openrocket-packs-them-2026-09-21
#[test]
fn an_unwritten_packed_size_is_openrockets() {
    let whole = with_parts([0, 1, 2, 3, 4]);
    let written = "<packedlength>0.08</packedlength><packedradius>auto</packedradius>";
    assert_eq!(whole.matches(written).count(), 1);
    for (size, length_m, radius_m) in [
        ("", 0.025, 0.0125),
        ("<packedlength>0.08</packedlength>", 0.08, 0.0125),
        ("<packedradius>0.02</packedradius>", 0.025, 0.02),
    ] {
        let xml = whole.replacen(written, size, 1);
        let read = read(xml.as_bytes()).expect("a readable design");
        let imported = component::rocket(&read.value.document);
        assert!(
            imported
                .warnings
                .iter()
                .all(|warning| !warning.message.contains("packed")),
            "{size}: {:?}",
            imported.warnings
        );
        let layout = imported.value.layout().expect("a design that lays out");
        let hpr_design::tree::Part::Parachute(chute) = resolved(&layout, "chute") else {
            panic!("the parachute")
        };
        assert_eq!(
            (chute.packing.length_m, chute.packing.radius_m),
            (length_m, radius_m),
            "{size}"
        );
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
/// radius. On every design the oracle ran, every radius hpr resolves is the one OpenRocket 24.12
/// settles on, bit for bit, except where OpenRocket answers −1 m: a nose cone's base or a
/// transition's end looking at another automatic radius, which no shape can take. Those take the
/// default too. The counts are pinned, so a change either way shows.
///
/// The cached numbers (`auto 0.04`) are ignored by OpenRocket, and so by hpr: the number is an
/// answer OpenRocket last wrote, never an input. And OpenRocket's first reading of one shape — a
/// tube beside an automatic tube that holds a part of automatic radius — is its default, which it
/// corrects to the neighbour's radius as soon as it works the design out again; hpr is held to the
/// settled answer, and the count of first readings that differ is pinned here too.
#[test]
fn a_radius_with_nothing_to_take_is_openrockets_default() {
    let fixture = automatic_radius_fixture();
    assert_eq!(fixture["openrocket"], "24.12");
    let radii = |components: &serde_json::Value| -> Vec<f64> {
        components
            .as_array()
            .expect("radii")
            .iter()
            .flat_map(|c| {
                ["base", "outer", "fore", "aft"]
                    .iter()
                    .filter_map(|key| c[key].as_f64())
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let mut compared = 0usize;
    let mut departures = 0usize;
    let mut defaulted = 0usize;
    let mut unsettled = 0usize;
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
        let theirs = radii(&case["resolved"]);
        if radii(&case["opened"]) != theirs {
            unsettled += 1;
        }
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
    assert_eq!((compared, departures, defaulted, unsettled), (39, 6, 17, 1));

    // The files the oracle ran from the reference library and the jar must all be there, or this
    // half of the test checks nothing: Loft's quirks fixture, which OpenRocket will not open; the
    // database's parachute catalogue, four tubes at the default; and the jar's 17 examples, one of
    // which OpenRocket first reads with a tube at its default and then settles. `cargo xtask ork`
    // holds hpr's radii for each of them to OpenRocket's.
    let library = fixture["library"].as_array().expect("library runs");
    let run = |suffix: &str| {
        library
            .iter()
            .find(|run| run["file"].as_str().is_some_and(|f| f.ends_with(suffix)))
            .unwrap_or_else(|| panic!("the oracle ran {suffix}"))
    };
    assert_eq!(run("demo-quirks.ork")["opens"], false);
    assert_eq!(
        radii(&run("parachutes.ork")["resolved"]),
        [OPENROCKET_DEFAULT_RADIUS_M; 4]
    );
    let examples: Vec<_> = library
        .iter()
        .filter(|run| {
            run["file"]
                .as_str()
                .is_some_and(|f| f.contains("!/datafiles/examples/"))
        })
        .collect();
    assert_eq!(examples.len(), 17);
    assert!(examples.iter().all(|run| run["opens"] == true));
    let settled_later = library
        .iter()
        .filter(|run| run["opens"] == true && radii(&run["opened"]) != radii(&run["resolved"]))
        .count();
    assert_eq!(settled_later, 1);
}

/// A body tube given the default radius has its wall judged against it, by the rule a stated
/// radius gets. `filled`, whether the file cached a radius or not, is solid to 25 mm — not
/// weightless for want of a radius, and not a wall as thick as a cached number OpenRocket ignores —
/// and a wall thicker than 25 mm is solid rather than a tube `hpr-design` must refuse. The warnings
/// that said the radius was unknown are withdrawn. A thin wall stays the wall it was.
#[test]
fn a_tube_given_the_default_has_its_wall_judged_against_it() {
    let tube = |thickness: &str, radius: &str| {
        format!(
            r#"<openrocket version="1.10" creator="test"><rocket><name>R</name><subcomponents>
            <stage><name>S</name><subcomponents><bodytube><name>T</name>
            <material type="bulk" density="680.0">Cardboard</material><length>0.3</length>
            <thickness>{thickness}</thickness><radius>{radius}</radius></bodytube>
            </subcomponents></stage></subcomponents></rocket></openrocket>"#
        )
    };
    let solid_kg = 680.0 * std::f64::consts::PI * 0.025 * 0.025 * 0.3;
    for (thickness, radius, wall_m) in [
        ("filled", "auto", 0.025),
        ("filled", "auto 0.01", 0.025),
        ("0.03", "auto", 0.025),
        ("0.002", "auto", 0.002),
    ] {
        let xml = tube(thickness, radius);
        let file = read(xml.as_bytes()).expect("a readable design");
        let spine = component::rocket(&file.value.document);
        let messages: Vec<&str> = spine.warnings.iter().map(|w| w.message.as_str()).collect();
        assert_eq!(messages.len(), 1, "{thickness}, {radius}: {messages:?}");
        // The warning names the tag the file wrote, not hpr's field.
        assert!(messages[0].contains("its `radius`;"), "{messages:?}");
        let hpr_design::tree::Part::BodyTube(t) = &spine.value.stages[0].components[0].part else {
            panic!("a body tube");
        };
        assert_eq!((t.outer_radius_m, t.thickness_m), (0.025, wall_m));
        let layout = spine.value.layout().expect("a design that lays out");
        let mass_kg = layout.structure.mass_kg;
        if wall_m == 0.025 {
            assert!(
                (mass_kg - solid_kg).abs() <= 1e-12 * solid_kg,
                "{thickness}, {radius}: {mass_kg} kg, solid is {solid_kg} kg"
            );
        } else {
            assert!(mass_kg < solid_kg / 5.0, "{mass_kg} kg");
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

/// A one-stage design with a nose cone and a 33 mm body tube that is a motor mount; `mount` is
/// the `<motormount>`'s contents, `rocket` any extra children of `<rocket>` (its configurations),
/// and `inside` any extra parts inside the body tube.
fn motor_design(rocket: &str, mount: &str, inside: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.11" creator="OpenRocket 24.12">
  <rocket><name>Sounder</name>{rocket}
    <subcomponents><stage><name>Sustainer</name><id>sustainer</id><subcomponents>
      <nosecone><name>Nose</name><id>nose</id>
        <material type="bulk" density="1000.0">Plastic</material>
        <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
        <aftradius>0.0165</aftradius></nosecone>
      <bodytube><name>Body</name><id>body</id>
        <material type="bulk" density="680.0">Cardboard</material>
        <length>0.6</length><thickness>0.001</thickness><radius>0.0165</radius>
        <motormount>{mount}</motormount>
        <subcomponents>{inside}</subcomponents></bodytube>
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#
    )
}

/// A synthetic motor no catalog lists: 100 N for a second, ramped up over 0.05 s and down over
/// 0.1 s, so 102.5 N·s from 60 g of propellant (an exhaust velocity of 1,708 m/s).
const SYNTHETIC_RSE: &str = r#"<engine-database>
  <engine-list>
    <engine mfg="Nobody" code="G100T" Type="single-use" dia="29." len="124." initWt="150."
      propWt="60." delays="6" Itot="102.5" burn-time="1.1">
      <data>
        <eng-data t="0." f="0." m="60."/>
        <eng-data t="0.05" f="100." m="58."/>
        <eng-data t="1." f="100." m="3."/>
        <eng-data t="1.1" f="0." m="0."/>
      </data>
    </engine>
  </engine-list>
</engine-database>"#;

/// Loft lesson L57: Loft's zip reader kept the design and dropped the `thrustcurves/*.rse`
/// entries, so a design was refused a flight for want of a curve the file was carrying. Here the
/// entry the `<digest>` names is the motor's curve, and it is used ahead of the catalog: an Estes
/// F15, which the catalog holds at 49.61 N·s, flies the 102.5 N·s curve its file embeds.
#[test]
fn embedded_rse_curves_are_read() {
    let xml = motor_design(
        r#"<motorconfiguration configid="c1" default="true"><name>G100T</name></motorconfiguration>
           <motorconfiguration configid="c2"><name>F15</name></motorconfiguration>"#,
        r"<ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
          <overhang>0.01</overhang>
          <motor configid='c1'><type>single</type><manufacturer>Nobody</manufacturer>
            <digest>d1935f00</digest><designation>G100T</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>
          <motor configid='c2'><type>single</type><manufacturer>Estes</manufacturer>
            <digest>f15f15f15</digest><designation>F15</designation>
            <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "",
    );
    let f15 = SYNTHETIC_RSE
        .replace(r#"mfg="Nobody" code="G100T""#, r#"mfg="Estes" code="F15""#)
        .replace(r#"len="124.""#, r#"len="114.""#);
    let archive = zip_of(&[
        ("rocket.ork", xml.as_bytes()),
        ("thrustcurves/d1935f00.rse", SYNTHETIC_RSE.as_bytes()),
        ("thrustcurves/f15f15f15.rse", f15.as_bytes()),
    ]);
    let read = read(&archive).expect("a readable archive");
    let design = design(&read.value);
    assert!(design.warnings.is_empty(), "{:?}", design.warnings);
    let configuration = &design.value.motors.configurations[0];
    let motor = &configuration.motors[0];
    let Curve::Embedded {
        entry,
        motor: solid,
    } = &motor.curve
    else {
        panic!("the embedded curve: {:?}", motor.curve);
    };
    assert_eq!(entry, "thrustcurves/d1935f00.rse");
    let impulse_ns = solid.curve().total_impulse_ns();
    assert!((impulse_ns - 102.5).abs() <= 1e-9 * 102.5, "{impulse_ns}");
    assert_eq!(motor.delay, Some(hpr_motor::Delay::Seconds(6.0)));
    assert_eq!(configuration.left_out, None);

    // The catalog has an F15 too, and the file's own curve wins.
    let f15 = &design.value.motors.configurations[1].motors[0];
    let Curve::Embedded { motor: solid, .. } = &f15.curve else {
        panic!("the F15's embedded curve: {:?}", f15.curve);
    };
    let impulse_ns = solid.curve().total_impulse_ns();
    assert!((impulse_ns - 102.5).abs() <= 1e-9 * 102.5, "{impulse_ns}");

    // It flies: the nozzle sits 10 mm aft of the 0.75 m airframe's tail.
    let assembly = design
        .value
        .rocket
        .assemble("c1")
        .expect("a flyable configuration");
    assert_eq!(assembly.motors.len(), 1);
    assert!((assembly.motors[0].nozzle_m.z + 0.76).abs() < 1e-12);

    // The same document without the curves names what is missing, and flies no G100T. The F15
    // falls back to the catalog's curve.
    let bare = read_design(xml.as_bytes());
    assert!(matches!(
        bare.motors.configurations[1].motors[0].curve,
        Curve::Catalog { .. }
    ));
    let configuration = &bare.motors.configurations[0];
    let Curve::Unresolved { why, reason } = &configuration.motors[0].curve else {
        panic!("no curve without the entry");
    };
    assert_eq!(*why, NoCurve::NotFound);
    assert!(reason.contains("no embedded curve"), "{reason}");
    let left_out = configuration.left_out.as_ref().expect("left out");
    assert_eq!(left_out.why, NotFlown::NoCurve);
    let flown: Vec<&str> = bare
        .rocket
        .configurations
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(flown, ["c2"]);
}

/// The synthetic G100T as a motor database would hand it in: built from the same samples and
/// envelope as the embedded `.rse`.
fn synthetic_motor() -> hpr_motor::SolidMotor {
    let thrust =
        hpr_motor::ThrustCurve::new(vec![0.0, 0.05, 1.0, 1.1], vec![0.0, 100.0, 100.0, 0.0])
            .expect("a valid curve");
    hpr_motor::SolidMotor::from_envelope(thrust, 0.029, 0.124, 0.06, 0.15).expect("a valid motor")
}

/// The case of the synthetic G100T.
const G100T_CASE: CaseSize = CaseSize {
    diameter_m: 0.029,
    length_m: 0.124,
};

/// M2.2c2: a curve supplied for a motor's digest flies it when the archive embeds none, an
/// embedded curve still comes first, a supplied curve comes ahead of the bundled catalog and is
/// never matched by name, and a motor the supplied curves lack says so in its reason.
#[test]
fn supplied_curves_are_used_by_digest_only() {
    let xml = motor_design(
        r#"<motorconfiguration configid="c1" default="true"><name>G100T</name></motorconfiguration>
           <motorconfiguration configid="c2"><name>Other</name></motorconfiguration>
           <motorconfiguration configid="c3"><name>No digest</name></motorconfiguration>
           <motorconfiguration configid="c4"><name>F15</name></motorconfiguration>
           <motorconfiguration configid="c5"><name>Hybrid</name></motorconfiguration>"#,
        r"<ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
          <overhang>0.01</overhang>
          <motor configid='c1'><type>single</type><manufacturer>Nobody</manufacturer>
            <digest>d1935f00</digest><designation>G100T</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>
          <motor configid='c2'><type>single</type><manufacturer>Nobody</manufacturer>
            <digest>0ther</digest><designation>G100T</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>
          <motor configid='c3'><type>single</type><manufacturer>Nobody</manufacturer>
            <designation>G100T</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>
          <motor configid='c4'><type>single</type><manufacturer>Estes</manufacturer>
            <digest>f15f15f15</digest><designation>F15</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>4.0</delay></motor>
          <motor configid='c5'><type>hybrid</type><manufacturer>Nobody</manufacturer>
            <digest>d1935f00</digest><designation>G100T</designation>
            <diameter>0.029</diameter><length>0.124</length><delay>6.0</delay></motor>",
        "",
    );
    let mut supplied = SuppliedCurves::new("a test database");
    assert!(supplied.is_empty());
    for digest in ["d1935f00", "f15f15f15"] {
        let replaced = supplied
            .insert(digest, G100T_CASE, synthetic_motor())
            .expect("a valid case");
        assert!(replaced.is_none());
    }
    assert_eq!((supplied.len(), supplied.source()), (2, "a test database"));
    assert!(supplied.get("d1935f00").is_some() && supplied.get("0ther").is_none());

    let plain = read(xml.as_bytes()).expect("a readable document");
    let design = design_with(&plain.value, &supplied);
    assert!(design.warnings.is_empty(), "{:?}", design.warnings);
    let [c1, c2, c3, c4, c5] = design.value.motors.configurations.as_slice() else {
        panic!("five configurations");
    };
    let Curve::Supplied {
        digest,
        from,
        motor,
    } = &c1.motors[0].curve
    else {
        panic!("the supplied curve: {:?}", c1.motors[0].curve);
    };
    assert_eq!(
        (digest.as_str(), from.as_str()),
        ("d1935f00", "a test database")
    );
    let impulse_ns = motor.curve().total_impulse_ns();
    assert!((impulse_ns - 102.5).abs() <= 1e-9 * 102.5, "{impulse_ns}");
    assert_eq!(c1.left_out, None);
    assert!(design.value.rocket.assemble("c1").is_ok());

    // Same maker and designation, another digest: not supplied, and the reason says where hpr
    // looked. With no digest at all, it says there was nothing to look up.
    for (configuration, says) in [
        (c2, "a test database has no curve for its digest"),
        (c3, "it records no digest to look up in the supplied curves"),
    ] {
        let Curve::Unresolved { why, reason } = &configuration.motors[0].curve else {
            panic!("no curve for {}", configuration.id);
        };
        assert_eq!(*why, NoCurve::NotFound);
        assert!(reason.contains(says), "{reason}");
        assert!(reason.contains("bundled catalog"), "{reason}");
        assert_eq!(
            configuration.left_out.as_ref().map(|out| out.why),
            Some(NotFlown::NoCurve)
        );
    }

    // The bundled catalog has an Estes F15, and the curve supplied for its digest comes first.
    assert!(matches!(c4.motors[0].curve, Curve::Supplied { .. }));
    assert!(matches!(
        super::design(&plain.value).value.motors.configurations[3].motors[0].curve,
        Curve::Catalog { .. }
    ));

    // A motor the design calls a hybrid is refused, whatever is supplied for its digest.
    assert!(matches!(
        c5.motors[0].curve,
        Curve::Unresolved {
            why: NoCurve::Hybrid,
            ..
        }
    ));

    // The archive's own curve still comes first, even with a curve supplied for its digest.
    let archive = zip_of(&[
        ("rocket.ork", xml.as_bytes()),
        ("thrustcurves/d1935f00.rse", SYNTHETIC_RSE.as_bytes()),
    ]);
    let zipped = read(&archive).expect("a readable archive");
    let design = design_with(&zipped.value, &supplied);
    assert!(matches!(
        design.value.motors.configurations[0].motors[0].curve,
        Curve::Embedded { .. }
    ));

    // An embedded curve that does not read is passed over, with a warning, for the supplied one.
    let broken = zip_of(&[
        ("rocket.ork", xml.as_bytes()),
        ("thrustcurves/d1935f00.rse", b"<engine-database>".as_slice()),
    ]);
    let broken = read(&broken).expect("a readable archive");
    let design = design_with(&broken.value, &supplied);
    assert!(matches!(
        design.value.motors.configurations[0].motors[0].curve,
        Curve::Supplied { .. }
    ));
    assert!(
        design.warnings.iter().any(|w| w
            .message
            .contains("the embedded curve for G100T was not used")),
        "{:?}",
        design.warnings
    );

    // A supplied case that differs from the design's by more than a millimetre is warned about.
    let mut apart = SuppliedCurves::new("a test database");
    let wide = CaseSize {
        diameter_m: 0.038,
        ..G100T_CASE
    };
    apart
        .insert("d1935f00", wide, synthetic_motor())
        .expect("a valid case");
    let design = design_with(&plain.value, &apart);
    assert!(
        design
            .warnings
            .iter()
            .any(|w| w.kind == WarningKind::Unusual && w.message.contains("in a test database")),
        "{:?}",
        design.warnings
    );

    // A case that is not a size is refused, and nothing is supplied.
    let mut none = SuppliedCurves::default();
    for case in [
        CaseSize {
            diameter_m: f64::NAN,
            ..G100T_CASE
        },
        CaseSize {
            length_m: 0.0,
            ..G100T_CASE
        },
    ] {
        assert!(none.insert("d1935f00", case, synthetic_motor()).is_err());
    }
    assert!(none.is_empty());
    assert_eq!(none.source(), "the supplied curves");
}

/// Reads a design from raw XML, expecting it to read.
fn read_design(xml: &[u8]) -> Design {
    design(&read(xml).expect("a readable design").value).value
}

/// Loft lesson L65: a configuration is split between `<rocket>`, which declares it, and each
/// mount, which holds its motor and may change when that motor ignites; some mounts name a
/// configuration `<rocket>` never declares; and Loft fired a motor whose mount it could not find
/// from stage 0. Here each configuration takes its own ignition, an undeclared one is read with a
/// warning, and a motor in a pod — a mount hpr does not read — keeps its configuration out of the
/// rocket rather than flying from anywhere else.
#[test]
fn per_config_overrides_and_dangling_mount_warn() {
    let f15 = |config: &str| {
        format!(
            "<motor configid='{config}'><type>single</type><manufacturer>Estes</manufacturer>\
             <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>\
             <delay>none</delay></motor>"
        )
    };
    let mount = format!(
        "<ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>\
         <overhang>0.0</overhang>{}{}{}\
         <ignitionconfiguration configid='a'><ignitionevent>launch</ignitionevent>\
         </ignitionconfiguration>\
         <ignitionconfiguration configid='b'><ignitiondelay>1.5</ignitiondelay>\
         </ignitionconfiguration>",
        f15("a"),
        f15("b"),
        f15("d")
    );
    let pod = format!(
        "<podset><name>Pods</name><id>pods</id><instancecount>2</instancecount>\
         <subcomponents><bodytube><name>Pod</name><id>pod</id>\
         <material type='bulk' density='680.0'>Cardboard</material><length>0.2</length>\
         <thickness>0.001</thickness><radius>0.015</radius>\
         <motormount><overhang>0.0</overhang>{}</motormount></bodytube></subcomponents></podset>",
        f15("c")
    );
    let xml = motor_design(
        r#"<motorconfiguration configid="a" default="true"/>
           <motorconfiguration configid="b"/>
           <motorconfiguration configid="c"/>"#,
        &mount,
        &pod,
    );
    let read = read(xml.as_bytes()).expect("a readable design");
    let design = design(&read.value);
    let motors = &design.value.motors;
    let ids: Vec<&str> = motors
        .configurations
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(ids, ["a", "b", "c", "d"]);
    let by_id = |id: &str| {
        motors
            .configurations
            .iter()
            .find(|c| c.id == id)
            .expect("the configuration")
    };

    // `a` takes its override's event and, having no delay of its own, the mount's.
    let a = &by_id("a").motors[0];
    assert_eq!(a.ignition.event, IgnitionEvent::Launch);
    assert_eq!(a.ignition.delay_s, 0.0);
    assert_eq!(a.delay, Some(hpr_motor::Delay::Plugged));
    assert!(matches!(a.curve, Curve::Catalog { .. }));
    // `b` keeps the mount's event and takes its own delay, so it lights 1.5 s after launch.
    let b = by_id("b");
    assert_eq!(
        b.motors[0].ignition,
        Ignition {
            event: IgnitionEvent::Automatic,
            delay_s: 1.5
        }
    );
    assert_eq!(
        b.left_out.as_ref().map(|l| l.why),
        Some(NotFlown::IgnitesInFlight)
    );
    // `c`'s only motor is in the pod: read nowhere, flown from nowhere.
    let c = by_id("c");
    assert!(c.motors.is_empty());
    assert_eq!(c.unread.len(), 1);
    assert!(c.unread[0].reason.contains("pod set"), "{:?}", c.unread);
    assert_eq!(
        c.left_out.as_ref().map(|l| l.why),
        Some(NotFlown::UnreadMotor)
    );
    // `d` is named only by the mount: read, flown, and said out loud.
    let d = by_id("d");
    assert!(!d.declared);
    assert!(
        design.warnings.iter().any(|w| w
            .message
            .contains("configuration `d`, which the rocket does not declare")),
        "{:?}",
        design.warnings
    );

    // The pod is part of the airframe hpr has not read, so nothing flies on this rocket, and `c`
    // is never flown from the body's mount or any other.
    assert_eq!(
        by_id("a").left_out.as_ref().map(|l| l.why),
        Some(NotFlown::AirframeNotAsWritten)
    );
    assert!(design.value.rocket.configurations.is_empty());
    assert!(design.value.rocket.assemble("c").is_err());

    // Without the pod, `a` and `d` fly from the body tube, and `c` holds no motor.
    let bare = read_design(
        motor_design(
            r#"<motorconfiguration configid="a" default="true"/>
               <motorconfiguration configid="b"/>
               <motorconfiguration configid="c"/>"#,
            &mount,
            "",
        )
        .as_bytes(),
    );
    let flown: Vec<&str> = bare
        .rocket
        .configurations
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(flown, ["a", "d"]);
    for id in flown {
        let assembly = bare.rocket.assemble(id).expect("assembles");
        assert_eq!(assembly.motors[0].mount, "body");
        assert_eq!(assembly.motors[0].stage, 0);
    }
    let c = bare
        .motors
        .configurations
        .iter()
        .find(|c| c.id == "c")
        .expect("c");
    assert_eq!(c.left_out.as_ref().map(|l| l.why), Some(NotFlown::NoMotor));
}

/// Which configurations a design flies. hpr lights every motor at launch and flies the airframe as
/// one body, so a configuration is flown only when that is what the file says. In a two-stage
/// design none is (`Staged`), and each is left out with the first reason that applies: a
/// sustainer's `automatic` motor lit in flight, a cluster, a switched-off stage, a hybrid, a missing
/// case size. The same booster flown alone, from its inner tube, is. A motor with a blank
/// `configid`, and a second motor for one configuration in one mount, are warned about and left out
/// without costing any other configuration its flight; an Estes `B4` does not take the catalog's
/// Quest `B4`; and a `0` delay is a charge at burnout.
#[test]
fn a_configuration_flies_only_as_written() {
    let motor = |config: &str, maker: &str, name: &str, delay: &str| {
        format!(
            "<motor configid='{config}'><type>single</type><manufacturer>{maker}</manufacturer>\
             <designation>{name}</designation><diameter>0.029</diameter><length>0.114</length>\
             <delay>{delay}</delay></motor>"
        )
    };
    let f15 = |config: &str| motor(config, "Estes", "F15", "4.0");
    let booster_mount = format!(
        "<overhang>0.0</overhang>{}{}{}{}{}{}",
        f15("boost"),
        f15("two"),
        f15("clu"),
        f15("off"),
        motor("b4", "Estes", "B4", "0.0"),
        "<motor configid='hyb'><type>hybrid</type><manufacturer>Estes</manufacturer>\
         <designation>F15</designation><diameter>0.029</diameter><length>0.114</length></motor>\
         <motor configid='nosize'><type>single</type><manufacturer>Estes</manufacturer>\
         <designation>F15</designation><delay>4.0</delay></motor>"
    );
    let tube = |name: &str, mount: &str, inside: &str| {
        format!(
            "<bodytube><name>{name}</name><id>{name}</id>\
             <material type='bulk' density='680.0'>Cardboard</material>\
             <length>0.4</length><thickness>0.001</thickness><radius>0.02</radius>{mount}\
             <subcomponents>{inside}</subcomponents></bodytube>"
        )
    };
    let inner = |id: &str, cluster: &str, mount: &str| {
        format!(
            "<innertube><name>{id}</name><id>{id}</id>\
             <material type='bulk' density='680.0'>Cardboard</material>\
             <axialoffset method='bottom'>0.0</axialoffset><length>0.2</length>\
             <outerradius>0.0152</outerradius><thickness>0.0005</thickness>{cluster}\
             <motormount>{mount}</motormount></innertube>"
        )
    };
    let sustainer_mount = format!(
        "<motormount><overhang>0.0</overhang>{}</motormount>",
        f15("two")
    );
    let cluster = inner(
        "cluster-mount",
        "<clusterconfiguration>3-ring</clusterconfiguration>",
        &format!(
            "<ignitionevent>launch</ignitionevent><overhang>0.0</overhang>{}",
            f15("clu")
        ),
    );
    let sustainer = tube("sustainer", &sustainer_mount, &cluster);
    let plain_sustainer = tube("sustainer", &sustainer_mount, "");
    let booster = tube("booster", "", &inner("booster-mount", "", &booster_mount));
    // The same mount with a second motor for `dup` and a motor in no configuration.
    let messy_mount = format!("{booster_mount}{}{}{}", f15("dup"), f15("dup"), f15(""));
    let messy_booster = tube("booster", "", &inner("booster-mount", "", &messy_mount));
    let nose = r#"<nosecone><name>Nose</name><id>nose</id>
          <material type="bulk" density="1000.0">Plastic</material>
          <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
          <aftradius>0.02</aftradius></nosecone>"#;
    let document = |stages: &str| {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Two-stage</name>
    <motorconfiguration configid="boost" default="true"/>
    <motorconfiguration configid="two"/>
    <motorconfiguration configid="clu"/>
    <motorconfiguration configid="off"><stage active="false"/></motorconfiguration>
    <motorconfiguration configid="b4"/>
    <motorconfiguration configid="hyb"/>
    <motorconfiguration configid="nosize"/>
    <motorconfiguration configid="dup"/>
    <subcomponents>{stages}</subcomponents></rocket>
</openrocket>"#
        )
    };
    let two_stage = document(&format!(
        "<stage><name>Sustainer</name><id>upper</id><subcomponents>{nose}{sustainer}\
         </subcomponents></stage>\
         <stage><name>Booster</name><id>lower</id><subcomponents>{booster}</subcomponents></stage>"
    ));
    let plain_two_stage = document(&format!(
        "<stage><name>Sustainer</name><id>upper</id><subcomponents>{nose}{plain_sustainer}\
         </subcomponents></stage>\
         <stage><name>Booster</name><id>lower</id><subcomponents>{booster}</subcomponents></stage>"
    ));
    let messy_one_stage = document(&format!(
        "<stage><name>Booster</name><id>lower</id><subcomponents>{nose}{messy_booster}\
         </subcomponents></stage>"
    ));
    let one_stage = document(&format!(
        "<stage><name>Booster</name><id>lower</id><subcomponents>{nose}{booster}\
         </subcomponents></stage>"
    ));

    let read_one = |xml: &str| {
        let file = read(xml.as_bytes()).expect("a readable design");
        design(&file.value)
    };
    let why = |design: &Imported<Design>, id: &str| {
        design
            .value
            .motors
            .configurations
            .iter()
            .find(|c| c.id == id)
            .expect("the configuration")
            .left_out
            .as_ref()
            .map(|l| l.why)
    };
    let flown = |design: &Imported<Design>| -> Vec<String> {
        design
            .value
            .rocket
            .configurations
            .iter()
            .map(|c| c.id.clone())
            .collect()
    };

    // The cluster tube is read with its three tubes (M1.9b), so the airframe is the design's, and
    // it is the second stage that keeps `boost` out.
    let two = read_one(&two_stage);
    assert_eq!(why(&two, "boost"), Some(NotFlown::Staged));
    assert_eq!(why(&two, "two"), Some(NotFlown::IgnitesInFlight));
    assert_eq!(why(&two, "clu"), Some(NotFlown::Cluster));
    assert_eq!(why(&two, "off"), Some(NotFlown::InactiveStage));
    assert_eq!(why(&two, "hyb"), Some(NotFlown::NoCurve));
    assert_eq!(why(&two, "nosize"), Some(NotFlown::NoSize));
    assert!(flown(&two).is_empty());
    let configurations = &two.value.motors.configurations;
    let off = configurations.iter().find(|c| c.id == "off").expect("off");
    assert_eq!(off.inactive_stages, [None]);
    // So it is without the cluster.
    assert_eq!(
        why(&read_one(&plain_two_stage), "boost"),
        Some(NotFlown::Staged)
    );
    // A mount with a second motor for one configuration, and a motor in none, says both; the
    // second motor keeps `dup` out, and the mount's warnings keep every configuration on it out.
    let messy = read_one(&messy_one_stage);
    for said in [
        "with no `configid`",
        "a second motor for configuration `dup`",
    ] {
        assert!(
            messy.warnings.iter().any(|w| w.message.contains(said)),
            "{said}: {:?}",
            messy.warnings
        );
    }
    assert_eq!(why(&messy, "dup"), Some(NotFlown::UnreadMotor));
    assert_eq!(why(&messy, "boost"), Some(NotFlown::AirframeNotAsWritten));
    assert!(
        !messy
            .value
            .motors
            .configurations
            .iter()
            .any(|c| c.id.is_empty())
    );
    let b4 = &configurations
        .iter()
        .find(|c| c.id == "b4")
        .expect("b4")
        .motors[0];
    assert!(
        matches!(
            b4.curve,
            Curve::Unresolved {
                why: NoCurve::NotFound,
                ..
            }
        ),
        "an Estes B4 is not the catalog's Quest B4: {:?}",
        b4.curve
    );
    assert_eq!(b4.delay, Some(hpr_motor::Delay::Seconds(0.0)));

    // The booster alone flies, from its inner tube, and so do the configurations whose only
    // trouble was the sustainer.
    let one = read_one(&one_stage);
    assert_eq!(flown(&one), ["boost", "two", "clu"]);
    let assembly = one.value.rocket.assemble("boost").expect("assembles");
    assert_eq!(assembly.motors.len(), 1);
    assert_eq!(assembly.motors[0].mount, "booster-mount");
    assert_eq!(assembly.motors[0].stage, 0);
}

/// A configuration whose airframe was read only in part is not flown: here a pod set with no
/// motor in it, which hpr does not read yet, would otherwise fly the rocket without the pod.
#[test]
fn a_configuration_on_an_incomplete_airframe_is_not_flown() {
    let xml = motor_design(
        r#"<motorconfiguration configid="a" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='a'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "<podset><name>Pods</name><id>pods</id><instancecount>2</instancecount></podset>",
    );
    let design = read_design(xml.as_bytes());
    let a = &design.motors.configurations[0];
    assert!(matches!(
        a.motors.first().map(|m| &m.curve),
        Some(Curve::Catalog { .. })
    ));
    assert_eq!(
        a.left_out.as_ref().map(|l| l.why),
        Some(NotFlown::AirframeNotAsWritten)
    );
    assert!(design.rocket.configurations.is_empty());
}

/// Nor is one whose airframe rests on an assumption: a nose shape hpr does not know is read as a
/// cone, with a warning. A shoulder of no wall is no assumption: OpenRocket gives it no mass, and
/// so does hpr (ADR-061), so that design flies, as does one with a shoulder wall stated.
#[test]
fn a_configuration_on_an_assumed_airframe_is_not_flown() {
    let with_shoulder = |thickness: &str| {
        motor_design(
            r#"<motorconfiguration configid="a" default="true"/>"#,
            "<overhang>0.0</overhang><motor configid='a'><type>single</type>\
             <manufacturer>Estes</manufacturer><designation>F15</designation>\
             <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
            "",
        )
        .replace(
            "<aftradius>0.0165</aftradius></nosecone>",
            &format!(
                "<aftradius>0.0165</aftradius><aftshoulderradius>0.0155</aftshoulderradius>\
                 <aftshoulderlength>0.03</aftshoulderlength>\
                 <aftshoulderthickness>{thickness}</aftshoulderthickness></nosecone>"
            ),
        )
    };
    let assumed = read_design(
        with_shoulder("0.001")
            .replacen("<shape>ogive</shape>", "<shape>fancy</shape>", 1)
            .as_bytes(),
    );
    assert_eq!(
        assumed.motors.configurations[0]
            .left_out
            .as_ref()
            .map(|l| l.why),
        Some(NotFlown::AirframeNotAsWritten)
    );
    for thickness in ["0.001", "0.0"] {
        let stated = read_design(with_shoulder(thickness).as_bytes());
        let flown: Vec<&str> = stated
            .rocket
            .configurations
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(flown, ["a"], "a shoulder wall of {thickness} m");
    }
}

/// Every event word OpenRocket 24.12 writes, as its committed probe measured them
/// (`validation/fixtures/ork/openrocket-events.json`), reads as the event it names and is written
/// back the same: none falls through to `Other`.
#[test]
fn every_event_word_openrocket_writes_is_read() {
    let text = include_str!("../../../../validation/fixtures/ork/openrocket-events.json");
    let fixture: serde_json::Value = serde_json::from_str(text).expect("JSON");
    let words = |event: &str| -> Vec<String> {
        fixture["words"][event]
            .as_object()
            .expect("an event")
            .values()
            .map(|entry| entry["word"].as_str().expect("a word").to_owned())
            .collect()
    };
    let ignition = words("ignition");
    assert_eq!(ignition.len(), 5);
    for word in &ignition {
        let event = IgnitionEvent::parse(word);
        assert!(!matches!(event, IgnitionEvent::Other(_)), "{word}");
        assert_eq!(event.as_str(), word);
    }
    let deployment = words("deployment");
    assert_eq!(deployment.len(), 6);
    for word in &deployment {
        let event = DeployEvent::parse(word);
        assert!(!matches!(event, DeployEvent::Other(_)), "{word}");
        assert_eq!(event.as_str(), word);
    }
    let separation = words("separation");
    assert_eq!(separation.len(), 9);
    for word in &separation {
        let event = SeparationEvent::parse(word);
        assert!(!matches!(event, SeparationEvent::Other(_)), "{word}");
        assert_eq!(event.as_str(), word);
    }
    // These guard the fixture the docs quote, not the reader: OpenRocket reports an automatic
    // parachute's drag coefficient as 0.8; a deploy height is above the ground (on a pad 1,000 m
    // up, set to 30 m, it opens within a metre of 30 m above the ground); and set above apogee, the
    // parachute never opens on a flight that reaches the ground.
    assert_eq!(
        fixture["drag"]["parachute_automatic_cd"].as_f64(),
        Some(0.8)
    );
    let low = &fixture["deploy_height"][0];
    let above_ground = low["at_deployment_above_ground_m"]
        .as_f64()
        .expect("deployed");
    assert!((above_ground - low["deploy_altitude_m"].as_f64().expect("set")).abs() < 1.0);
    let high = &fixture["deploy_height"][1];
    assert!(high["deploy_altitude_m"].as_f64() > high["apogee_above_ground_m"].as_f64());
    assert_eq!(high["reached_the_ground"].as_bool(), Some(true));
    assert_eq!(high["deployed"].as_bool(), Some(false));
}

/// A recovery setting hpr cannot read is not flown, so it does not keep a configuration out of the
/// air; the same trouble in the parachute's shape does, because that is the airframe.
#[test]
fn a_recovery_warning_does_not_ground_a_configuration_but_a_shape_warning_does() {
    let with_chute = |chute: &str| {
        motor_design(
            r#"<motorconfiguration configid="a" default="true"/>"#,
            "<overhang>0.0</overhang><motor configid='a'><type>single</type>\
             <manufacturer>Estes</manufacturer><designation>F15</designation>\
             <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
            &format!(
                "<parachute><name>Main</name><id>main</id>\
                 <axialoffset method='top'>0.0</axialoffset><packedlength>0.05</packedlength>\
                 <packedradius>0.01</packedradius><cd>auto</cd>\
                 <material type='surface' density='0.067'>Ripstop nylon</material>{chute}\
                 <linecount>6</linecount><linelength>0.5</linelength>\
                 <linematerial type='line' density='0.0018'>Elastic cord</linematerial>\
                 </parachute>"
            ),
        )
    };
    let flown = |design: &Design| design.rocket.configurations.len();
    let late = read_design(
        with_chute(
            "<deployevent>Apogee</deployevent><deploydelay>soon</deploydelay>\
             <diameter>0.6</diameter>",
        )
        .as_bytes(),
    );
    assert_eq!(flown(&late), 1);
    assert_eq!(
        late.recovery.devices[0].deployment.event,
        Some(DeployEvent::Other("Apogee".to_owned()))
    );
    let bad_shape = read_design(
        with_chute("<deployevent>apogee</deployevent><diameter>wide</diameter>").as_bytes(),
    );
    assert_eq!(flown(&bad_shape), 0);
}

/// A parachute's own deployment, a configuration that changes one of its three settings, the drag
/// coefficient stated or left to OpenRocket, a stage's separation per configuration, and a
/// parachute inside a pod, which is kept apart as not read.
#[test]
fn recovery_settings_are_read_per_configuration() {
    let chute = |id: &str, cd: &str, extra: &str| {
        format!(
            "<parachute><name>{id}</name><id>{id}</id>\
             <axialoffset method='top'>0.0</axialoffset><packedlength>0.05</packedlength>\
             <packedradius>0.01</packedradius><cd>{cd}</cd>\
             <material type='surface' density='0.067'>Ripstop nylon</material>\
             <deployevent>ejection</deployevent><deployaltitude>200.0</deployaltitude>\
             <deploydelay>0.0</deploydelay>{extra}<diameter>0.6</diameter><linecount>6</linecount>\
             <linelength>0.5</linelength>\
             <linematerial type='line' density='0.0018'>Elastic cord</linematerial></parachute>"
        )
    };
    let main = chute(
        "main",
        "auto",
        "<deploymentconfiguration configid='a'><deployevent>altitude</deployevent>\
         <deployaltitude>150.0</deployaltitude></deploymentconfiguration>",
    );
    let pod = format!(
        "<podset><name>Pods</name><id>pods</id><instancecount>2</instancecount><subcomponents>\
         <bodytube><name>Pod</name><id>pod</id>\
         <material type='bulk' density='680.0'>Cardboard</material><length>0.2</length>\
         <thickness>0.001</thickness><radius>0.015</radius><subcomponents>{}</subcomponents>\
         </bodytube></subcomponents></podset>",
        chute("pod-chute", "1.5", "")
    );
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Two-stage</name><motorconfiguration configid="a" default="true"/>
    <subcomponents>
      <stage><name>Sustainer</name><id>upper</id><subcomponents>
        <nosecone><name>Nose</name><id>nose</id>
          <material type="bulk" density="1000.0">Plastic</material>
          <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
          <aftradius>0.02</aftradius></nosecone>
        <bodytube><name>Body</name><id>body</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.4</length><thickness>0.001</thickness><radius>0.02</radius>
          <subcomponents>{main}{pod}</subcomponents></bodytube>
      </subcomponents></stage>
      <stage><name>Booster</name><id>lower</id>
        <separationevent>ejection</separationevent><separationaltitude>200.0</separationaltitude>
        <separationdelay>0.0</separationdelay>
        <separationconfiguration configid="a"><separationevent>burnout</separationevent>
          <separationdelay>0.5</separationdelay></separationconfiguration>
        <subcomponents>
        <bodytube><name>Booster</name><id>booster</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.3</length><thickness>0.001</thickness><radius>0.02</radius>
          <subcomponents>{drogue}</subcomponents></bodytube>
      </subcomponents></stage>
    </subcomponents></rocket>
</openrocket>"#,
        drogue = chute("drogue", "0.61", "")
    );
    let design = read_design(xml.as_bytes());
    let recovery = &design.recovery;
    let ids: Vec<&str> = recovery.devices.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["main", "drogue"]);

    let main = &recovery.devices[0];
    assert_eq!((main.kind, main.stage), (DeviceKind::Parachute, 0));
    assert_eq!(main.cd, Some(Dimension::Automatic { cached: None }));
    assert_eq!(main.deployment.event, Some(DeployEvent::Ejection));
    // Configuration `a` changes the event and the height, and keeps the device's own delay.
    let in_a = main.deployment_in("a");
    assert_eq!(in_a.event, Some(DeployEvent::Altitude));
    assert_eq!(in_a.altitude_m, Some(150.0));
    assert_eq!(in_a.delay_s, Some(0.0));
    assert_eq!(main.deployment_in("b"), &main.deployment);

    let drogue = &recovery.devices[1];
    assert_eq!(drogue.stage, 1);
    assert_eq!(drogue.cd, Some(Dimension::Stated { value: 0.61 }));

    // Only the booster states a separation; `a` makes it burnout plus half a second.
    assert_eq!(recovery.separations.len(), 1);
    let booster = &recovery.separations[0];
    assert_eq!((booster.id.as_str(), booster.stage), ("lower", 1));
    assert_eq!(booster.separation.event, Some(SeparationEvent::Ejection));
    let in_a = booster.separation_in("a");
    assert_eq!(in_a.event, Some(SeparationEvent::Burnout));
    assert_eq!((in_a.altitude_m, in_a.delay_s), (Some(200.0), Some(0.5)));

    assert_eq!(recovery.unread.len(), 1);
    assert_eq!(recovery.unread[0].inside, "podset");
    assert!(recovery.unread_separations.is_empty());
}

/// Loft lesson L64: Loft read the wind's direction from `launchroddirection`, and dropped it from
/// the stored conditions. The wind's direction is `winddirection`, in radians, the bearing it blows
/// from; the rod's is `launchroddirection`, in degrees, the bearing it tilts toward. Every
/// condition OpenRocket 24.12 wrote in its committed probe
/// (`validation/fixtures/ork/openrocket-conditions.json`) reads as the value it held.
#[test]
fn wind_direction_is_not_rod_direction() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../validation/fixtures/ork/openrocket-conditions.json"
    ))
    .expect("the fixture is JSON");
    let written = &fixture["conditions"]["written"];
    let loaded = &fixture["conditions"]["loaded"];
    let text = |tag: &str| written[tag].as_str().expect("written").to_owned();
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12"><rocket><name>R</name></rocket>
  <simulations><simulation status="uptodate"><name>Probe</name><conditions>
    <launchrodlength>{}</launchrodlength><launchrodangle>{}</launchrodangle>
    <launchroddirection>{}</launchroddirection><launchintowind>{}</launchintowind>
    <windaverage>{}</windaverage><windturbulence>{}</windturbulence>
    <winddirection>{}</winddirection><launchaltitude>{}</launchaltitude>
    <launchlatitude>{}</launchlatitude><launchlongitude>{}</launchlongitude>
    <atmosphere model="{}"><basetemperature>{}</basetemperature>
      <basepressure>{}</basepressure></atmosphere>
  </conditions></simulation></simulations>
</openrocket>"#,
        text("launchrodlength"),
        text("launchrodangle"),
        text("launchroddirection"),
        text("launchintowind"),
        text("windaverage"),
        text("windturbulence"),
        text("winddirection"),
        text("launchaltitude"),
        text("launchlatitude"),
        text("launchlongitude"),
        written["atmosphere"]["model"].as_str().expect("a model"),
        written["atmosphere"]["basetemperature"]
            .as_str()
            .expect("K"),
        written["atmosphere"]["basepressure"].as_str().expect("Pa"),
    );
    let design = read_design(xml.as_bytes());
    let conditions = design.simulations[0]
        .conditions
        .as_ref()
        .expect("conditions");
    let held = |key: &str| loaded[key].as_f64().expect("a number");
    let close = |ours: Option<f64>, key: &str| {
        let theirs = held(key);
        let ours = ours.expect("read");
        assert!(
            (ours - theirs).abs() <= 1e-12 * theirs.abs().max(1.0),
            "{key}: {ours} against {theirs}"
        );
    };
    // The file wrote 45 for the rod and 0.5 for the wind: degrees for one, radians for the other.
    assert_eq!(text("launchroddirection"), "45.0");
    assert_eq!(text("winddirection"), "0.5");
    close(conditions.rod_direction_rad, "launchroddirection");
    close(conditions.wind_from_rad, "winddirection");
    assert_ne!(conditions.wind_from_rad, conditions.rod_direction_rad);
    close(conditions.rod_angle_rad, "launchrodangle");
    close(conditions.rod_length_m, "launchrodlength");
    close(conditions.wind_speed_m_s, "windaverage");
    close(conditions.wind_turbulence, "windturbulence");
    close(conditions.launch_altitude_m, "launchaltitude");
    close(conditions.latitude_deg, "launchlatitude");
    close(conditions.longitude_deg, "launchlongitude");
    assert_eq!(conditions.into_wind, Some(false));
    let Some(Atmosphere::Extended {
        temperature_k,
        pressure_pa,
    }) = conditions.atmosphere
    else {
        panic!("an extended atmosphere: {:?}", conditions.atmosphere);
    };
    close(temperature_k, "launch_temperature");
    close(pressure_pa, "launch_pressure");

    // These guard the fixture the docs quote, not the reader. The rod's direction is a compass
    // bearing: toward 0 the rocket lands north, toward 90 east, while the example's own wind stays
    // where it was. The wind's direction is where it blows from: from the east, a rocket off a
    // vertical rod drifts west. With `launchintowind`, the rod is written at the wind's bearing,
    // in degrees.
    let direction = &fixture["direction"];
    assert!(direction[0]["landed_north_m"].as_f64() > Some(10.0));
    assert!(direction[1]["landed_east_m"].as_f64() > Some(10.0));
    assert_eq!(
        direction[0]["wind_direction_rad"],
        direction[1]["wind_direction_rad"]
    );
    let drift = &fixture["drift"];
    assert_eq!(
        drift[0]["wind_from_rad"].as_f64(),
        Some(std::f64::consts::FRAC_PI_2)
    );
    assert!(drift[0]["landed_east_m"].as_f64() < Some(-10.0));
    assert!(drift[1]["landed_north_m"].as_f64() < Some(-10.0));
    let into_wind = &fixture["into_wind"];
    let rod_deg: f64 = into_wind["written"]["launchroddirection"]
        .as_str()
        .expect("written")
        .parse()
        .expect("a number");
    assert!((rod_deg.to_radians() - 0.5).abs() < 1e-12);
}

/// A design's stored results read back: the summary, each stage's time series by column, `NaN`
/// kept where OpenRocket computed nothing, and the events; a row with the wrong number of values
/// is left out with a warning.
#[test]
fn stored_results_are_read_back() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12"><rocket><name>R</name></rocket>
  <simulations><simulation status="uptodate"><name>Simulation 1</name>
    <simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>
    <flightdata maxaltitude="50.59" maxvelocity="29.249" maxacceleration="143.649" maxmach="0.086"
        timetoapogee="3.481" flighttime="15.888" groundhitvelocity="4.681"
        launchrodvelocity="15.365" deploymentvelocity="2.646" optimumdelay="2.751">
      <databranch name="Sustainer" types="Time,Altitude,Stability margin calibers">
        <event time="0" type="launch" source="rocket"/>
        <event time="3.481" type="apogee" source="rocket"/>
        <datapoint>0,0,NaN</datapoint>
        <datapoint>1.5,30.25,1.8</datapoint>
        <datapoint>3.481,50.59</datapoint>
      </databranch>
    </flightdata>
  </simulation></simulations>
</openrocket>"#;
    let read = read(xml.as_bytes()).expect("a readable document");
    let design = design(&read.value);
    let simulation = &design.value.simulations[0];
    assert_eq!(simulation.name, "Simulation 1");
    assert_eq!(simulation.status.as_deref(), Some("uptodate"));
    let results = simulation.results.as_ref().expect("results");
    assert_eq!(results.max_altitude_m, Some(50.59));
    assert_eq!(results.optimum_delay_s, Some(2.751));
    let branch = &results.branches[0];
    assert_eq!(branch.name, "Sustainer");
    assert_eq!(
        branch.column("Altitude"),
        Some(vec![Some(0.0), Some(30.25)])
    );
    let margin = branch
        .column("Stability margin calibers")
        .expect("a column");
    assert_eq!(margin, [None, Some(1.8)]);
    let kinds: Vec<&str> = branch.events.iter().map(|e| e.kind.as_str()).collect();
    assert_eq!(kinds, ["launch", "apogee"]);
    assert!(
        design
            .warnings
            .iter()
            .any(|w| w.message.contains("1 `datapoint` row(s) are not 3 numbers")),
        "{:?}",
        design.warnings
    );

    // A design with stored results survives JSON and back unchanged, `NaN`s and all.
    let json = serde_json::to_string(&design.value).expect("JSON");
    let back: Design = serde_json::from_str(&json).expect("read back");
    assert_eq!(back, design.value);
}

/// Stored simulations are read from a document that holds no design at all, the average `<wind>`
/// stands in where the legacy tags are missing, a multilevel wind keeps its levels, and an event
/// with no type is left out with a warning.
#[test]
fn stored_conditions_read_without_a_design() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <simulations><simulation status="loaded"><name>Only a result</name><conditions>
    <launchroddirection>90.0</launchroddirection>
    <wind model="average"><speed>4.0</speed><direction>1.0</direction></wind>
    <wind model="multilevel" altituderef="agl">
      <windlevel altitude="0.0" speed="2.0" direction="1.57" standarddeviation="0.2"/>
      <windlevel altitude="100.0" speed="3.0" direction="1.6" standarddeviation="0.3"/>
    </wind>
    <windmodeltype>Average</windmodeltype>
  </conditions>
  <flightdata maxaltitude="10.0"><warning>Recovery device deployment at high speed</warning>
    <databranch name="Sustainer" types="Time"><event time="1.0"/><datapoint>0</datapoint>
    </databranch></flightdata>
  </simulation></simulations>
</openrocket>"#;
    let read = read(xml.as_bytes()).expect("a readable document");
    let design = design(&read.value);
    let simulation = &design.value.simulations[0];
    let conditions = simulation.conditions.as_ref().expect("conditions");
    assert_eq!(conditions.wind_speed_m_s, Some(4.0));
    assert_eq!(conditions.wind_from_rad, Some(1.0));
    assert_eq!(conditions.wind_model.as_deref(), Some("Average"));
    assert_eq!(conditions.wind_levels.len(), 2);
    assert_eq!(conditions.wind_levels[1].altitude_m, Some(100.0));
    assert_eq!(conditions.wind_levels_above.as_deref(), Some("agl"));
    let results = simulation.results.as_ref().expect("results");
    assert_eq!(
        results.warnings,
        ["Recovery device deployment at high speed"]
    );
    assert!(results.branches[0].events.is_empty());
    assert!(
        design
            .warnings
            .iter()
            .any(|w| w.message.contains("no time or no type"))
    );
}

/// A stored result can be eligible as OpenRocket data even when hpr cannot reproduce the design's
/// selected configuration. The two screens must not collapse into one reason.
#[test]
fn stored_reference_and_reproduction_screens_are_separate() {
    let stored_results = r#"<simulations>
      <simulation status="uptodate"><name>Stored</name><simulator>RK4Simulator</simulator>
        <calculator>BarrowmanCalculator</calculator><conditions><configid>c1</configid></conditions>
        <flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/>
      </simulation>
    </simulations>"#;
    let stored_without_configuration = r#"<simulations>
      <simulation status="uptodate"><name>Stored</name><simulator>RK4Simulator</simulator>
        <calculator>BarrowmanCalculator</calculator>
        <flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/>
      </simulation>
    </simulations>"#;
    let xml = motor_design(
        r#"<motorconfiguration configid="c1" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='c1'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "",
    )
    .replace("</openrocket>", &format!("{stored_results}</openrocket>"));
    let design = read_design(xml.as_bytes());
    let simulation = &design.simulations[0];
    assert_eq!(simulation.reference_exclusion(), None);
    assert_eq!(design.reproduction_exclusion(simulation), None);
    assert_eq!(design.reference_exclusion(simulation), None);

    // The same stored result remains a valid reference when the configuration is absent from hpr's
    // motor list; only the reproduction screen changes.
    let missing = motor_design(
        r#"<motorconfiguration configid="c1" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='other'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "",
    )
    .replace("</openrocket>", &format!("{stored_results}</openrocket>"));
    let missing = read_design(missing.as_bytes());
    let simulation = &missing.simulations[0];
    assert_eq!(simulation.reference_exclusion(), None);
    assert_eq!(
        missing.reproduction_exclusion(simulation),
        Some(StoredReferenceExclusion::UnflyableConfiguration)
    );
    assert_eq!(
        missing.reference_exclusion(simulation),
        Some(StoredReferenceExclusion::UnflyableConfiguration)
    );

    // A missing configuration has its own stable reason when the design is otherwise complete.
    let missing_configuration = motor_design(
        r#"<motorconfiguration configid="c1" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='c1'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "",
    )
    .replace(
        "</openrocket>",
        &format!("{stored_without_configuration}</openrocket>"),
    );
    let missing_configuration = read_design(missing_configuration.as_bytes());
    let simulation = &missing_configuration.simulations[0];
    assert_eq!(simulation.reference_exclusion(), None);
    assert_eq!(
        missing_configuration.reproduction_exclusion(simulation),
        Some(StoredReferenceExclusion::MissingConfiguration)
    );

    // A reduced airframe takes precedence for reproduction even when the conditions name no
    // configuration; it is a design limitation, not a stored-data failure.
    let reduced = motor_design(
        r#"<motorconfiguration configid="c1" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='c1'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "<podset><name>Pod</name><id>pod</id><instancecount>2</instancecount></podset>",
    )
    .replace("</openrocket>", &format!("{stored_results}</openrocket>"));
    let reduced = read_design(reduced.as_bytes());
    let simulation = &reduced.simulations[0];
    assert_eq!(simulation.reference_exclusion(), None);
    assert_eq!(
        reduced.reproduction_exclusion(simulation),
        Some(StoredReferenceExclusion::ReducedDesign)
    );
}

/// Loft lesson L66: Loft dropped pods, parallel stages and booster sets, and its export then lost
/// the note that the rocket was reduced. Here both are kept whole in `extensions.x-openrocket`, at
/// paths that lead back to them, and the design says it is reduced.
#[test]
fn pods_kept_in_extensions_or_flagged_reduced() {
    let xml = motor_design(
        r#"<motorconfiguration configid="a" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='a'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "<podset><name>Pods</name><id>pods</id><instancecount>2</instancecount>\
         <radiusoffset method='surface'>0.0</radiusoffset>\
         <angleoffset method='relative'>90.0</angleoffset><subcomponents>\
         <bodytube><name>Pod</name><id>pod</id><length>0.2</length><radius>0.01</radius>\
         </bodytube></subcomponents></podset>\
         <parallelstage><name>Boosters</name><id>boosters</id><instancecount>2</instancecount>\
         <separationevent>burnout</separationevent><subcomponents>\
         <nosecone><name>Booster nose</name><id>booster-nose</id></nosecone>\
         </subcomponents></parallelstage>",
    );
    let file = read(xml.as_bytes()).expect("a readable design").value;
    let design = design(&file).value;
    assert!(design.is_reduced());
    let parts = &design.extensions.x_openrocket.parts;
    let at: Vec<&str> = parts.iter().map(|kept| kept.at.as_str()).collect();
    assert_eq!(
        at,
        [
            "openrocket/rocket/stage[0]/bodytube[1]/podset[0]",
            "openrocket/rocket/stage[0]/bodytube[1]/parallelstage[1]",
        ]
    );
    for kept in parts {
        assert_eq!(element_at(&file.document, &kept.at), Some(&kept.element));
    }
    // Neither is in the rocket, and no configuration flies on a rocket read only in part.
    assert!(motors::stage_of(&design.rocket, "pod").is_none());
    assert!(design.rocket.configurations.is_empty());

    // A design with nothing left out is not reduced, and keeps no parts.
    let whole = read_design(
        motor_design(
            r#"<motorconfiguration configid="a"/>"#,
            "<overhang>0.0</overhang>",
            "",
        )
        .as_bytes(),
    );
    assert!(!whole.is_reduced());
    assert!(whole.extensions.x_openrocket.parts.is_empty());
}

/// A document with content hpr does not model — a section it does not read, a component tag it
/// has never seen, a simulation's extension — keeps each in `extensions.x-openrocket`, and the
/// extension survives being written out and read back: every kept element comes back the same, at
/// a path that leads to it in the original document.
#[test]
fn unknown_content_round_trips_through_x_openrocket() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>R</name><subcomponents>
    <stage><name>Sustainer</name><id>s</id><subcomponents>
      <nosecone><name>Nose</name><id>nose</id>
        <material type="bulk" density="1000.0" group="Plastics">Plastic</material>
        <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
        <aftradius>0.02</aftradius><appearance><paint red="51" green="51" blue="51"/></appearance>
        <glowsinthedark>true</glowsinthedark></nosecone>
      <fancything kind="new"><name>Something new</name><size unit="m">0.1</size></fancything>
    </subcomponents></stage></subcomponents></rocket>
  <simulations><simulation status="uptodate"><name>Simulation 1</name>
    <conditions><launchrodlength>1.0</launchrodlength><randomseed>42</randomseed>
      <wind model="average"><speed>2.0</speed><gusts>3.0</gusts></wind></conditions>
    <extension extensionid="com.example.Wind"><config key="gust">3.0</config></extension>
  </simulation></simulations>
  <photostudio><roll>0.5</roll><sky>Mountains</sky></photostudio>
  <docprefs><docmaterials><material>BULK|Custom|680.0|1.2E9|Custom</material></docmaterials>
  </docprefs>
</openrocket>"#;
    let file = read(xml.as_bytes()).expect("a readable document").value;
    let design = design(&file).value;
    let kept = &design.extensions.x_openrocket;
    let at = |list: &[Kept]| list.iter().map(|k| k.at.clone()).collect::<Vec<_>>();
    assert_eq!(
        at(&kept.parts),
        ["openrocket/rocket/stage[0]/fancything[1]"]
    );
    assert_eq!(
        at(&kept.sections),
        [
            "openrocket/simulations/simulation[0]/extension[2]",
            "openrocket/photostudio[2]",
            "openrocket/docprefs[3]",
        ]
    );
    // The tags no reader asked for, in parts it did read: the nose cone's colour and a tag hpr has
    // never seen, and the stored conditions' random seed.
    assert_eq!(
        at(&kept.tags),
        [
            "openrocket/rocket/stage[0]/nosecone[0]/@appearance[7]",
            "openrocket/rocket/stage[0]/nosecone[0]/@glowsinthedark[8]",
            "openrocket/simulations/simulation[0]/conditions[1]/@randomseed[1]",
            "openrocket/simulations/simulation[0]/conditions[1]/@wind[2]/@gusts[1]",
        ]
    );
    // And an attribute no reader asked for, on a tag one did: the material's group.
    let attributes: Vec<(&str, &str, &str)> = kept
        .attributes
        .iter()
        .map(|a| (a.at.as_str(), a.name.as_str(), a.value.as_str()))
        .collect();
    assert_eq!(
        attributes,
        [(
            "openrocket/rocket/stage[0]/nosecone[0]/@material[2]",
            "group",
            "Plastics"
        )]
    );

    // Written out as JSON under its namespace and read back, the extension is unchanged...
    let json = serde_json::to_string(&design.extensions).expect("JSON");
    assert!(json.starts_with(r#"{"x-openrocket":"#), "{json}");
    let back: Extensions = serde_json::from_str(&json).expect("read back");
    assert_eq!(back, design.extensions);
    // ...and every element in it is the one at its path in the document it came from.
    let every = &back.x_openrocket;
    for kept in every.parts.iter().chain(&every.sections).chain(&every.tags) {
        assert_eq!(
            element_at(&file.document, &kept.at),
            Some(&kept.element),
            "{}",
            kept.at
        );
    }
    for attribute in &every.attributes {
        let on = element_at(&file.document, &attribute.at).expect("the element it was on");
        assert_eq!(
            on.attribute(&attribute.name),
            Some(attribute.value.as_str())
        );
    }
}

/// A path that does not lead to an element gives `None`, never a panic, and an extension written
/// before a namespace had anything in it still reads.
#[test]
fn a_bad_path_leads_nowhere() {
    let xml = motor_design(
        r#"<motorconfiguration configid="a"/>"#,
        "<overhang>0.0</overhang>",
        "",
    );
    let file = read(xml.as_bytes()).expect("a readable design").value;
    for at in [
        "",
        "/",
        "rocket",
        "openrocket/rocket/stage[9]",
        "openrocket/rocket/stage[0]/bodytube[0]",
        "openrocket/rocket/stage[-1]",
        "openrocket/rocket/stage[0][0]",
        "openrocket/rocket/stage[99999999999999999999999]",
        "openrocket/rocket/stage[0]/@name[0]/@x[0]",
        "openrocket/photostudio[0]/extra[0]",
        "openrocket/simulations/simulation[0]/conditions[0]/@x[0]/y[0]",
    ] {
        assert_eq!(element_at(&file.document, at), None, "{at}");
    }
    let empty: Extensions = serde_json::from_str(r#"{"x-openrocket":{}}"#).expect("defaults");
    assert_eq!(empty, Extensions::default());
}

/// A number rounded to nine significant figures, so a snapshot does not change with the last
/// digits a platform's arithmetic leaves.
fn rounded(value: f64) -> serde_json::Value {
    if !value.is_finite() || value == 0.0 {
        return serde_json::json!(value);
    }
    let text = format!("{value:.8e}");
    serde_json::json!(text.parse::<f64>().unwrap_or(value))
}

/// What `design` reads a file as, in the numbers and words a reader would check against the file:
/// the spine and its mass, the motors and which configurations fly, the recovery settings, the
/// stored simulations, what is kept in `x-openrocket`, and every warning.
fn snapshot_of(design: &Imported<Design>) -> serde_json::Value {
    use serde_json::json;
    let value = &design.value;
    let stages: Vec<_> = value
        .rocket
        .stages
        .iter()
        .map(|stage| {
            json!({
                "id": stage.id,
                "components": stage.components.iter().map(|component| json!({
                    "id": component.id,
                    "kind": component.part.kind_name(),
                    "parts": component.children.len(),
                    "motor_mount": component.motor_mount.is_some(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    let layout = match value.rocket.layout() {
        Ok(layout) => json!({
            "mass_kg": rounded(layout.structure.mass_kg),
            "cg_z_m": rounded(layout.structure.cg_m.z),
        }),
        Err(error) => json!({ "error": error.to_string() }),
    };
    let configurations: Vec<_> = value
        .motors
        .configurations
        .iter()
        .map(|configuration| {
            json!({
                "id": configuration.id,
                "name": configuration.name,
                "default": configuration.default,
                "left_out": configuration.left_out.as_ref().map(|out| json!(out.why)),
                "motors": configuration.motors.iter().map(|motor| json!({
                    "motor": format!("{} {}", motor.manufacturer, motor.designation),
                    "mount": motor.mount,
                    "curve": match &motor.curve {
                        Curve::Embedded { .. } => json!("embedded"),
                        Curve::Supplied { .. } => json!("supplied"),
                        Curve::Catalog { .. } => json!("catalog"),
                        Curve::Unresolved { why, .. } => json!({ "none": why }),
                    },
                    "delay": json!(motor.delay),
                    "ignition": format!("{} + {} s", motor.ignition.event.as_str(), motor.ignition.delay_s),
                })).collect::<Vec<_>>(),
                "unread": configuration.unread.len(),
            })
        })
        .collect();
    let devices: Vec<_> = value
        .recovery
        .devices
        .iter()
        .map(|device| {
            json!({
                "id": device.id,
                "kind": json!(device.kind),
                "cd": json!(device.cd),
                "deploy": device.deployment.event.as_ref().map(DeployEvent::as_str),
                "altitude_m": device.deployment.altitude_m,
                "delay_s": device.deployment.delay_s,
                "configurations": device.configurations.len(),
            })
        })
        .collect();
    let separations: Vec<_> = value
        .recovery
        .separations
        .iter()
        .map(|stage| {
            json!({
                "stage": stage.id,
                "event": stage.separation.event.as_ref().map(SeparationEvent::as_str),
                "configurations": stage.configurations.len(),
            })
        })
        .collect();
    let simulations: Vec<_> = value
        .simulations
        .iter()
        .map(|simulation| {
            let results = simulation.results.as_ref();
            json!({
                "name": simulation.name,
                "status": simulation.status,
                "configuration": simulation.conditions.as_ref().and_then(|c| c.configuration.clone()),
                "max_altitude_m": results.and_then(|r| r.max_altitude_m),
                "branches": results.map_or(0, |r| r.branches.len()),
                "rows": results.map_or(0, |r| r.branches.iter().map(|b| b.rows.len()).sum::<usize>()),
            })
        })
        .collect();
    let kept = &value.extensions.x_openrocket;
    json!({
        "rocket": value.rocket.name,
        "stages": stages,
        "layout": layout,
        "configurations": configurations,
        "flown": value.rocket.configurations.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
        "recovery": { "devices": devices, "separations": separations },
        "simulations": simulations,
        "reduced": value.is_reduced(),
        "kept": {
            "parts": kept.parts.iter().map(|k| k.at.clone()).collect::<Vec<_>>(),
            "sections": kept.sections.len(),
            "tags": kept.tags.len(),
            "attributes": kept.attributes.len(),
        },
        "warnings": design.warnings.iter().map(|w| json!({
            "at": w.at,
            "kind": json!(w.kind),
            "says": w.message,
        })).collect::<Vec<_>>(),
    })
}

/// Loft's seven demonstration designs, committed under `validation/fixtures/ork/loft-demo/` from
/// `nrdptel/fusionspace-loft` (MIT, Neer's own), read whole and held to a snapshot each: a change
/// in what hpr reads from a real `.ork` shows up here as a diff to review. The private reference
/// library is never snapshotted; `cargo xtask ork` reports it as counts.
#[test]
fn loft_demo_designs_read_as_snapshotted() {
    let designs: [(&str, &[u8]); 7] = [
        (
            "demo_boattail",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-boattail.ork"),
        ),
        (
            "demo_dual_deploy",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-dual-deploy.ork"),
        ),
        (
            "demo_multi_config",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-multi-config.ork"),
        ),
        (
            "demo_payload_separation",
            include_bytes!(
                "../../../../validation/fixtures/ork/loft-demo/demo-payload-separation.ork"
            ),
        ),
        (
            "demo_quirks",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-quirks.ork"),
        ),
        (
            "demo_single_deploy",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-single-deploy.ork"),
        ),
        (
            "demo_stable",
            include_bytes!("../../../../validation/fixtures/ork/loft-demo/demo-stable.ork"),
        ),
    ];
    for (name, bytes) in designs {
        let file = read(bytes).expect("a readable design");
        let design = design(&file.value);
        insta::assert_json_snapshot!(name, snapshot_of(&design));
    }
    // None of Loft's demonstration motors is in the bundled catalog, so a synthetic design stands
    // for one that flies: an Estes F15 from the catalog, in a body tube.
    let flies = motor_design(
        r#"<motorconfiguration configid="a" default="true"/>"#,
        "<overhang>0.0</overhang><motor configid='a'><type>single</type>\
         <manufacturer>Estes</manufacturer><designation>F15</designation>\
         <diameter>0.029</diameter><length>0.114</length><delay>4.0</delay></motor>",
        "",
    );
    let file = read(flies.as_bytes()).expect("a readable design");
    insta::assert_json_snapshot!("synthetic_f15", snapshot_of(&design(&file.value)));
}
