//! Tests of writing a `.ork`: each read back as the design it was written from.

use super::super::{design, read};
use super::*;

/// Reads `bytes` as a `.ork` and returns its design.
fn design_of(bytes: &[u8]) -> Design {
    let file = read(bytes).expect("a readable .ork").value;
    design(&file).value
}

/// A document with nothing in it but a rocket's name is written as schema 1.10, in a zip with the
/// design as `rocket.ork`, and reads back as the same design.
#[test]
fn a_bare_rocket_round_trips() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Sounder</name></rocket>
</openrocket>"#;
    let original = design_of(xml);
    let written = write(&original, &[]).expect("written").value;
    let file = read(&written).expect("readable").value;
    assert_eq!(file.container, super::super::Container::Zip);
    assert_eq!(file.design_entry.as_deref(), Some(DESIGN_ENTRY));
    assert_eq!(file.document.version, SCHEMA);
    assert_eq!(design(&file).value, original);
}
