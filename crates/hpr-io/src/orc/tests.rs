use super::*;

/// A catalogue file holding `materials` and `parts` (their XML).
fn file(materials: &str, parts: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<OpenRocketComponent><Version>0.1</Version>\
         <Materials>{materials}</Materials><Components>{parts}</Components></OpenRocketComponent>"
    )
}

/// A bulk material `name` of `density` in `unit`.
fn bulk(name: &str, density: &str, unit: &str) -> String {
    format!(
        "<Material UnitsOfMeasure=\"{unit}\"><Name>{name}</Name><Density>{density}</Density>\
         <Type>BULK</Type></Material>"
    )
}

/// A body tube numbered `number` in `material`, with `extra` fields after its three dimensions.
fn body_tube(number: &str, material: &str, extra: &str) -> String {
    format!(
        "<BodyTube><Manufacturer>Acme</Manufacturer><PartNumber>{number}</PartNumber>\
         <Description>A tube</Description><Material Type=\"BULK\">{material}</Material>\
         <InsideDiameter Unit=\"in\">1.0</InsideDiameter>\
         <OutsideDiameter Unit=\"in\">1.1</OutsideDiameter><Length Unit=\"in\">10</Length>{extra}\
         </BodyTube>"
    )
}

fn read_ok(text: &str) -> Read {
    match read(text, "test.orc") {
        Ok(read) => read,
        Err(error) => panic!("the test file should read: {error}"),
    }
}

#[test]
fn every_bundled_file_reads_with_only_the_known_warnings() {
    let mut parts = 0;
    let mut warnings = Vec::new();
    for (name, text) in BUNDLED_FILES {
        let read = read(text, name).expect("every bundled file reads");
        // Every part element the file holds is read: none left out.
        let document = roxmltree::Document::parse(text).expect("bundled files are XML");
        let elements = document
            .descendants()
            .find(|node| node.has_tag_name("Components"))
            .map_or(0, |list| {
                list.children().filter(roxmltree::Node::is_element).count()
            });
        assert_eq!(read.catalog.parts.len(), elements, "{name}");
        assert!(read.catalog.parts.iter().all(|part| part.file == *name));
        parts += elements;
        warnings.extend(read.warnings.into_iter().map(|warning| (*name, warning)));
    }
    assert_eq!(parts, 3449);
    assert_eq!(bundled().parts.len(), 3449);
    // The warnings are the database's own slips, each counted: 37 nose cones state an inside
    // diameter (no nose cone field), three parts state their description twice, and three name a
    // material their file doesn't define.
    let count = |needle: &str| {
        warnings
            .iter()
            .filter(|(_, warning)| warning.message.contains(needle))
            .count()
    };
    assert_eq!(
        count("<InsideDiameter> is not a field of a <NoseCone> OpenRocket reads"),
        37
    );
    assert_eq!(count("<Description> is stated 2 times"), 3);
    assert_eq!(
        count("is not defined in this file, so it has no density"),
        3
    );
    assert_eq!(warnings.len(), 37 + 3 + 3, "{warnings:#?}");
    let undefined: Vec<_> = bundled()
        .parts
        .iter()
        .flat_map(|part| kind_materials(&part.kind))
        .filter(|material| material.density.is_none())
        .map(|material| material.name.as_str())
        .collect();
    assert_eq!(
        undefined,
        [
            "Carpet Thread",
            "Carpet Thread",
            "Balsa, bulk, Estes typical"
        ]
    );
}

#[test]
fn a_part_reads_in_si() {
    let text = file(
        &bulk("Kraft", "0.8", "g/cm3"),
        &body_tube("BT-1", "Kraft", "<Mass Unit=\"oz\">2</Mass>"),
    );
    let read = read_ok(&text);
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    assert_eq!(read.version.as_deref(), Some("0.1"));
    assert_eq!(
        read.materials,
        [CatalogMaterial {
            name: "Kraft".to_owned(),
            kind: MaterialKind::Bulk,
            density: 800.0,
        }]
    );
    let part = &read.catalog.parts[0];
    assert_eq!(part.file, "test.orc");
    assert_eq!(part.manufacturer, "Acme");
    assert_eq!(part.part_number, "BT-1");
    assert_eq!(part.description, "A tube");
    assert_eq!(part.mass_kg, Some(2.0 * 0.028_349_523_125));
    let PartKind::BodyTube(tube) = &part.kind else {
        panic!("a body tube: {:?}", part.kind)
    };
    assert_eq!(tube.inner_diameter_m, 0.0254);
    assert_eq!(tube.outer_diameter_m, 1.1 * 0.0254);
    assert_eq!(tube.length_m, 10.0 * 0.0254);
    assert_eq!(tube.thickness_m(), (1.1 * 0.0254 - 0.0254) / 2.0);
    assert_eq!(
        tube.material.material(),
        Some(hpr_design::Material::bulk("Kraft", 800.0))
    );
}

#[test]
#[allow(
    clippy::excessive_precision,
    reason = "the exact values are written to every digit Python's rational arithmetic printed"
)]
fn units_are_their_exact_definitions() {
    // NIST Handbook 44 (2024), Appendix C: the inch, foot and avoirdupois pound and ounce.
    for (unit, factor) in [
        ("m", 1.0),
        ("cm", 0.01),
        ("mm", 0.001),
        ("in", 0.0254),
        ("ft", 0.3048),
    ] {
        assert_eq!(length_factor(unit), Some(factor), "{unit}");
    }
    for (unit, factor) in [
        ("kg", 1.0),
        ("g", 0.001),
        ("lb", 0.453_592_37),
        ("oz", 0.028_349_523_125),
    ] {
        assert_eq!(mass_factor(unit), Some(factor), "{unit}");
    }
    let close = |value: Option<f64>, expected: f64| {
        let value = value.expect("a unit the format has");
        assert!(
            ((value - expected) / expected).abs() < 1e-15,
            "{value} against {expected}"
        );
    };
    // Each derived density factor against its exact value, worked out in rational arithmetic
    // from the definitions.
    close(
        density_factor(MaterialKind::Bulk, "lb/ft3"),
        16.018_463_373_960_138,
    );
    close(
        density_factor(MaterialKind::Surface, "oz/in2"),
        43.941_848_727_447_457,
    );
    close(
        density_factor(MaterialKind::Surface, "oz/ft2"),
        0.305_151_727_273_940_63,
    );
    close(
        density_factor(MaterialKind::Surface, "lb/ft2"),
        4.882_427_636_383_050_1,
    );
    close(
        density_factor(MaterialKind::Line, "oz/ft"),
        0.093_010_246_473_097_108,
    );
    close(density_factor(MaterialKind::Bulk, "g/cm3"), 1000.0);
    close(density_factor(MaterialKind::Bulk, "kg/dm3"), 1000.0);
    close(density_factor(MaterialKind::Surface, "g/cm2"), 10.0);
    close(density_factor(MaterialKind::Surface, "g/m2"), 0.001);
    close(density_factor(MaterialKind::Line, "g/m"), 0.001);
    close(density_factor(MaterialKind::Line, "g/cm"), 0.1);
    // Units the format doesn't have, or a unit of another kind, are not read.
    assert_eq!(length_factor("in/64"), None);
    assert_eq!(length_factor("furlong"), None);
    assert_eq!(mass_factor("slug"), None);
    assert_eq!(density_factor(MaterialKind::Line, "oz/in"), None);
    assert_eq!(density_factor(MaterialKind::Bulk, "kg/m2"), None);
    assert_eq!(density_factor(MaterialKind::Surface, "kg/m"), None);
}

#[test]
fn values_with_no_unit_are_si() {
    let text = file(
        "<Material><Name>Plain</Name><Density>700</Density><Type>BULK</Type></Material>",
        "<BodyTube><Manufacturer>Acme</Manufacturer><PartNumber>1</PartNumber>\
         <Material Type=\"BULK\">Plain</Material><InsideDiameter>0.02</InsideDiameter>\
         <OutsideDiameter> 0.021 </OutsideDiameter><Length>0.5</Length></BodyTube>",
    );
    let read = read_ok(&text);
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    let part = &read.catalog.parts[0];
    assert_eq!(part.description, "");
    let PartKind::BodyTube(tube) = &part.kind else {
        panic!("a body tube")
    };
    assert_eq!(
        (tube.inner_diameter_m, tube.outer_diameter_m, tube.length_m),
        (0.02, 0.021, 0.5)
    );
    assert_eq!(tube.material.density, Some(700.0));
}

#[test]
fn every_kind_of_part_reads() {
    let materials = [
        bulk("Plastic", "1000", "kg/m3"),
        "<Material UnitsOfMeasure=\"g/m2\"><Name>Nylon</Name><Density>60</Density>\
         <Type>SURFACE</Type></Material>"
            .to_owned(),
        "<Material UnitsOfMeasure=\"kg/m\"><Name>Cord</Name><Density>0.002</Density>\
         <Type>LINE</Type></Material>"
            .to_owned(),
    ]
    .concat();
    let common = |tag: &str, number: &str, material: &str| {
        format!(
            "<{tag}><Manufacturer>Acme</Manufacturer><PartNumber>{number}</PartNumber>{material}"
        )
    };
    let plastic = "<Material Type=\"BULK\">Plastic</Material>";
    let nylon = "<Material Type=\"SURFACE\">Nylon</Material>";
    let mut parts = String::new();
    for tag in ["TubeCoupler", "EngineBlock", "CenteringRing", "LaunchLug"] {
        parts += &common(tag, tag, plastic);
        parts += "<InsideDiameter>0.01</InsideDiameter><OutsideDiameter>0.02</OutsideDiameter>\
                  <Length>0.03</Length>";
        parts += &format!("</{tag}>");
    }
    parts += &common("BulkHead", "BH", plastic);
    parts += "<OutsideDiameter>0.02</OutsideDiameter><Length>0.005</Length><Filled>1</Filled>\
              </BulkHead>";
    parts += &common("NoseCone", "NC", plastic);
    parts += "<Shape>HAACK</Shape><OutsideDiameter>0.04</OutsideDiameter>\
              <ShoulderDiameter>0.038</ShoulderDiameter><ShoulderLength>0.03</ShoulderLength>\
              <Length>0.15</Length><Thickness Unit=\"mm\">2</Thickness></NoseCone>";
    parts += &common("Transition", "TR", plastic);
    parts += "<Shape>OGIVE</Shape><ForeOutsideDiameter>0.03</ForeOutsideDiameter>\
              <ForeShoulderDiameter>0.028</ForeShoulderDiameter>\
              <ForeShoulderLength>0.02</ForeShoulderLength>\
              <AftOutsideDiameter>0.04</AftOutsideDiameter>\
              <AftShoulderDiameter>0.038</AftShoulderDiameter>\
              <AftShoulderLength>0</AftShoulderLength><Length>0.05</Length>\
              <Filled>false</Filled><Thickness>0.001</Thickness><Mass Unit=\"g\">12</Mass>\
              </Transition>";
    parts += &common("Parachute", "PC", nylon);
    parts += "<Diameter Unit=\"in\">18</Diameter><Sides>6</Sides><LineCount>6</LineCount>\
              <LineLength Unit=\"in\">18</LineLength><LineMaterial Type=\"LINE\">Cord</LineMaterial>\
              </Parachute>";
    parts += &common("Streamer", "ST", nylon);
    parts += "<Width Unit=\"in\">2</Width><Length Unit=\"ft\">3</Length>\
              <Thickness Unit=\"in\">0.001</Thickness></Streamer>";
    let read = read_ok(&file(&materials, &parts));
    assert!(read.warnings.is_empty(), "{:?}", read.warnings);
    let kinds: Vec<_> = read.catalog.parts.iter().map(|part| &part.kind).collect();
    let tube = Tube {
        inner_diameter_m: 0.01,
        outer_diameter_m: 0.02,
        length_m: 0.03,
        material: MaterialRef {
            name: "Plastic".to_owned(),
            kind: MaterialKind::Bulk,
            density: Some(1000.0),
        },
    };
    assert_eq!(kinds[0], &PartKind::TubeCoupler(tube.clone()));
    assert_eq!(kinds[1], &PartKind::EngineBlock(tube.clone()));
    assert_eq!(kinds[2], &PartKind::CenteringRing(tube.clone()));
    assert_eq!(kinds[3], &PartKind::LaunchLug(tube.clone()));
    assert_eq!(
        kinds[4],
        &PartKind::Bulkhead(Bulkhead {
            outer_diameter_m: 0.02,
            length_m: 0.005,
            filled: Some(true),
            material: tube.material.clone(),
        })
    );
    assert_eq!(
        kinds[5],
        &PartKind::NoseCone(NoseCone {
            shape: Shape::Haack,
            length_m: 0.15,
            outer_diameter_m: 0.04,
            shoulder_diameter_m: 0.038,
            shoulder_length_m: 0.03,
            filled: None,
            thickness_m: Some(0.002),
            material: tube.material.clone(),
        })
    );
    assert_eq!(
        kinds[6],
        &PartKind::Transition(Transition {
            shape: Shape::Ogive,
            length_m: 0.05,
            fore_outer_diameter_m: 0.03,
            fore_shoulder_diameter_m: 0.028,
            fore_shoulder_length_m: 0.02,
            aft_outer_diameter_m: 0.04,
            aft_shoulder_diameter_m: 0.038,
            aft_shoulder_length_m: 0.0,
            filled: Some(false),
            thickness_m: Some(0.001),
            material: tube.material.clone(),
        })
    );
    assert_eq!(read.catalog.parts[6].mass_kg, Some(0.012));
    // The canopy's `g/m2` is read as written: 60 g/m² is 0.06 kg/m².
    assert_eq!(
        kinds[7],
        &PartKind::Parachute(Parachute {
            diameter_m: 18.0 * 0.0254,
            sides: 6,
            line_count: 6,
            line_length_m: 18.0 * 0.0254,
            material: MaterialRef {
                name: "Nylon".to_owned(),
                kind: MaterialKind::Surface,
                density: Some(60.0 * 0.001),
            },
            line_material: Some(MaterialRef {
                name: "Cord".to_owned(),
                kind: MaterialKind::Line,
                density: Some(0.002),
            }),
        })
    );
    assert_eq!(
        kinds[8],
        &PartKind::Streamer(Streamer {
            length_m: 3.0 * 0.3048,
            width_m: 2.0 * 0.0254,
            thickness_m: 0.001 * 0.0254,
            material: MaterialRef {
                name: "Nylon".to_owned(),
                kind: MaterialKind::Surface,
                density: Some(60.0 * 0.001),
            },
        })
    );
    let ids: Vec<_> = read
        .catalog
        .parts
        .iter()
        .map(|part| part.part_number.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "TubeCoupler",
            "EngineBlock",
            "CenteringRing",
            "LaunchLug",
            "BH",
            "NC",
            "TR",
            "PC",
            "ST"
        ]
    );
}

#[test]
fn text_that_is_not_a_catalogue_is_refused() {
    assert!(matches!(
        read("not xml", "x.orc"),
        Err(OrcError::Xml { .. })
    ));
    assert_eq!(
        read("<openrocket version=\"1.9\"/>", "x.orc"),
        Err(OrcError::NotACatalog {
            root: "openrocket".to_owned()
        })
    );
    // A long root name is quoted shortened.
    let long = "a".repeat(500);
    let Err(OrcError::NotACatalog { root }) = read(&format!("<{long}/>"), "x.orc") else {
        panic!("refused")
    };
    assert_eq!(root.chars().count(), QUOTE_LIMIT + 1);
}

/// The one warning reading `parts` gives, with no part read.
fn left_out(parts: &str) -> String {
    let read = read_ok(&file(&bulk("Kraft", "800", "kg/m3"), parts));
    assert!(read.catalog.parts.is_empty(), "{:?}", read.catalog.parts);
    assert_eq!(read.warnings.len(), 1, "{:?}", read.warnings);
    read.warnings[0].message.clone()
}

#[test]
fn a_part_that_cant_be_read_is_left_out_with_why() {
    let message =
        left_out(&body_tube("1", "Kraft", "").replace("Unit=\"in\">10", "Unit=\"furlong\">10"));
    assert_eq!(
        message,
        "<Length>'s unit `furlong` is not one the format has; the part was left out"
    );
    let message = left_out(&body_tube("1", "Kraft", "").replace(">10<", ">ten<"));
    assert_eq!(
        message,
        "<Length> is `ten`, not a number; the part was left out"
    );
    let message = left_out(&body_tube("1", "Kraft", "").replace(">10<", ">NaN<"));
    assert_eq!(
        message,
        "<Length> is `NaN`, not a number; the part was left out"
    );
    let message =
        left_out(&body_tube("1", "Kraft", "").replace("<Length Unit=\"in\">10</Length>", ""));
    assert_eq!(message, "it has no <Length>; the part was left out");
    let message =
        left_out(&body_tube("1", "Kraft", "").replace("<Manufacturer>Acme</Manufacturer>", ""));
    assert_eq!(message, "it names no <Manufacturer>; the part was left out");
    let message = left_out(&body_tube("1", "Kraft", "").replace("<PartNumber>1</PartNumber>", ""));
    assert_eq!(message, "it has no <PartNumber>; the part was left out");
    let message =
        left_out(&body_tube("1", "Kraft", "").replace("Type=\"BULK\"", "Type=\"SURFACE\""));
    assert_eq!(
        message,
        "<Material> names a material of the wrong kind for the part; the part was left out"
    );
    let message = left_out(&body_tube("1", "Kraft", "").replace("Type=\"BULK\"", "Type=\"bulk\""));
    assert_eq!(
        message,
        "<Material>'s Type is not BULK, SURFACE or LINE; the part was left out"
    );
    let message = left_out(
        "<NoseCone><Manufacturer>Acme</Manufacturer><PartNumber>2</PartNumber>\
         <Material Type=\"BULK\">Kraft</Material><Shape>BULLET</Shape>\
         <OutsideDiameter>0.04</OutsideDiameter><ShoulderDiameter>0</ShoulderDiameter>\
         <ShoulderLength>0</ShoulderLength><Length>0.1</Length></NoseCone>",
    );
    assert_eq!(
        message,
        "<Shape> is `BULLET`, not one of CONICAL, OGIVE, ELLIPSOID, PARABOLIC, HAACK or POWER; \
         the part was left out"
    );
    let message = left_out(
        "<BulkHead><Manufacturer>Acme</Manufacturer><PartNumber>3</PartNumber>\
         <Material Type=\"BULK\">Kraft</Material><OutsideDiameter>0.04</OutsideDiameter>\
         <Length>0.01</Length><Filled>yes</Filled></BulkHead>",
    );
    assert_eq!(
        message,
        "<Filled> is `yes`, not true or false; the part was left out"
    );
    let message = left_out(
        "<RailButton><Manufacturer>Acme</Manufacturer><PartNumber>4</PartNumber></RailButton>",
    );
    assert_eq!(
        message,
        "<RailButton> is not a kind of part this reads; it was left out"
    );
}

#[test]
fn odd_fields_are_reported_and_the_part_kept() {
    let text = file(
        &bulk("Kraft", "800", "kg/m3"),
        &body_tube(
            "1",
            "Kraft",
            "<Colour>red</Colour><Description>Second</Description>",
        ),
    );
    let read = read_ok(&text);
    assert_eq!(read.catalog.parts.len(), 1);
    assert_eq!(read.catalog.parts[0].description, "Second");
    let messages: Vec<_> = read.warnings.iter().map(|w| w.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "<Description> is stated 2 times; the last was read, as OpenRocket reads it",
            "<Colour> is not a field of a <BodyTube> OpenRocket reads; it was ignored",
        ]
    );
    assert!(read.warnings.iter().all(|w| w.at == "Acme 1"));
}

#[test]
fn a_material_not_defined_reads_with_no_density() {
    let read = read_ok(&file("", &body_tube("1", "Missing", "")));
    let PartKind::BodyTube(tube) = &read.catalog.parts[0].kind else {
        panic!("a body tube")
    };
    assert_eq!(tube.material.density, None);
    assert_eq!(tube.material.material(), None);
    assert_eq!(
        read.warnings[0].message,
        "its material `Missing` is not defined in this file, so it has no density"
    );
}

#[test]
fn materials_that_cant_be_read_are_left_out() {
    let materials = [
        bulk("Odd", "800", "kg/m2"),
        "<Material><Name>NoKind</Name><Density>800</Density></Material>".to_owned(),
        "<Material><Name>NoDensity</Name><Type>BULK</Type></Material>".to_owned(),
        "<Stuff/>".to_owned(),
    ]
    .concat();
    let read = read_ok(&file(&materials, ""));
    assert!(read.materials.is_empty());
    let messages: Vec<_> = read.warnings.iter().map(|w| w.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "`kg/m2` is not a unit of bulk density the format has; it was left out",
            "its <Type> is not BULK, SURFACE or LINE; it was left out",
            "its <Density> is missing or not a number; it was left out",
            "<Stuff> is not a material; it was left out",
        ]
    );
}

#[test]
fn parts_are_found_by_maker_and_number() {
    let catalog = bundled();
    let found = catalog.find("estes", " BT-20, 30316 ");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].manufacturer, "Estes");
    assert_eq!(found[0].part_number, "BT-20, 30316");
    assert!(matches!(found[0].kind, PartKind::BodyTube(_)));
    // The number is matched whole and exactly.
    assert!(catalog.find("Estes", "BT-20").is_empty());
    assert!(catalog.find("Estes", "bt-20, 30316").is_empty());
    assert!(catalog.find("Nobody", "BT-20, 30316").is_empty());
    // A search finds it by a piece of its number, with every part whose number or description
    // (\"fits BT-20, ...\") holds the piece.
    let found = catalog.search(Some("ESTES"), "bt-20,");
    assert_eq!(found[0].part_number, "BT-20, 30316");
    assert!(found.iter().all(|part| part.manufacturer == "Estes"));
    let all = catalog.search(None, "BT-20");
    assert!(all.len() > 5 && all.iter().any(|part| part.manufacturer != "Estes"));
    assert!(all.iter().all(|part| {
        part.part_number.to_lowercase().contains("bt-20")
            || part.description.to_lowercase().contains("bt-20")
    }));
    // A number that names two different parts gives both, in catalogue order.
    let pair = catalog.find("SEMROC", "BC-926");
    assert_eq!(pair.len(), 2);
    assert_ne!(pair[0], pair[1]);
    let makers = catalog.manufacturers();
    assert_eq!(makers.len(), 16, "{makers:?}");
    assert_eq!(makers[0], "BalsaMachining");
}
