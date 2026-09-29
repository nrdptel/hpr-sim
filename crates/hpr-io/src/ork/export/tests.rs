//! Tests of writing a `.ork`: each read back as the design it was written from.
//!
//! Every design here is invented. Two of the tests pin Loft lessons, the mistakes an earlier
//! project's `.ork` export made (`docs/research/loft-lessons.md`): L67 and L68.

use std::io::{Cursor, Read as _};

use hpr_design::fins::FinPlanform;
use hpr_design::tree::Part;
use hpr_motor::Delay;

use super::super::document::Element;
use super::super::motors::{Ignition, IgnitionEvent};
use super::super::{Container, design, read};
use super::motors::tests::{assert_same, component};
use super::*;

/// Reads `bytes` as a `.ork` and returns its design.
fn design_of(bytes: &[u8]) -> Design {
    // A design written by hand here names its parts; the ids become UUIDs, as OpenRocket's are.
    let xml = std::str::from_utf8(bytes).map(super::rocket::tests::uuids);
    let bytes = xml.as_ref().map_or(bytes, |xml| xml.as_bytes());
    let file = read(bytes).expect("a readable .ork").value;
    design(&file).value
}

/// Every entry of the zip archive `bytes`, in the archive's order, as its name and contents.
fn entries(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("a zip archive");
    (0..archive.len())
        .map(|index| {
            let mut entry = archive.by_index(index).expect("an entry");
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).expect("the entry's bytes");
            (entry.name().to_owned(), contents)
        })
        .collect()
}

/// `design` written as a `.ork` file and read back: the design read, the text of the file's
/// `rocket.ork` exactly as written, and the export's warnings.
fn written_and_read(design: &Design) -> (Design, String, Vec<Warning>) {
    let written = write(design, &[]).expect("written");
    let entries = entries(&written.value);
    assert_eq!(entries.len(), 1, "only the design was written");
    assert_eq!(entries[0].0, DESIGN_ENTRY);
    let text = String::from_utf8(entries[0].1.clone()).expect("UTF-8");
    (design_of(&written.value), text, written.warnings)
}

/// The UUID a hand-written design's part named `name` is given ([`design_of`]).
fn uuid_of(name: &str) -> String {
    let tagged = super::rocket::tests::uuids(&format!("<id>{name}</id>"));
    tagged["<id>".len()..tagged.len() - "</id>".len()].to_owned()
}

/// The component a hand-written design names `name`, in `design`.
fn part<'a>(design: &'a Design, name: &str) -> &'a hpr_design::tree::Component {
    component(design, &uuid_of(name))
}

/// The element anywhere under `root` whose `<id>` is the UUID a hand-written design's part named
/// `id` is given.
fn by_id<'a>(root: &'a Element, id: &str) -> &'a Element {
    let id = &uuid_of(id);
    fn find<'a>(element: &'a Element, id: &str) -> Option<&'a Element> {
        if element
            .child("id")
            .is_some_and(|own| own.text().trim() == id)
        {
            return Some(element);
        }
        element.elements().find_map(|child| find(child, id))
    }
    find(root, id).expect("an element with that id")
}

/// The written document `text`, parsed back into elements.
fn root_of(text: &str) -> Element {
    read(text.as_bytes())
        .expect("the written text reads")
        .value
        .document
        .root
}

/// The text of `element`'s child `name`, if it has one.
fn text_of(element: &Element, name: &str) -> Option<String> {
    element.child(name).map(Element::text)
}

/// The child of `element` called `name` whose `configid` is `id`.
fn for_configuration<'a>(element: &'a Element, name: &str, id: &str) -> &'a Element {
    element
        .children_named(name)
        .find(|child| child.attribute("configid") == Some(id))
        .expect("a child for that configuration")
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
    assert_eq!(file.container, Container::Zip);
    assert_eq!(file.design_entry.as_deref(), Some(DESIGN_ENTRY));
    assert_eq!(file.document.version, SCHEMA);
    assert_eq!(design(&file).value, original);
    // The entry is stamped with zip's zero date, 1980-01-01, never the clock's: the same design
    // is the same bytes whenever it is written.
    let mut archive = zip::ZipArchive::new(Cursor::new(&written)).expect("a zip archive");
    let stamp = archive
        .by_index(0)
        .expect("an entry")
        .last_modified()
        .expect("a date");
    assert_eq!(stamp, zip::DateTime::DEFAULT);
}

/// An Estes F15 for configuration `config`, with `delay` as its `<delay>`.
fn f15(config: &str, delay: &str) -> String {
    format!(
        "<motor configid=\"{config}\"><type>single</type><manufacturer>Estes</manufacturer>\
         <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>\
         <delay>{delay}</delay></motor>"
    )
}

/// The invented two-stage design of Loft lesson L67's test: three configurations; a sustainer
/// whose mount lights at its own default in one configuration and differently in the other two
/// (at the booster's burnout and half a second after it, and at its ejection charge and 1.25 s
/// after it); a booster whose motor is plugged in two configurations and has a 3 s delay in the
/// third; a mass override that covers the parts inside, a centre-of-gravity override that does
/// not, and parts with no override at all; and a stored simulation with every launch condition.
fn staged_design() -> String {
    let upper = format!(
        "<ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>\
         <overhang>0.005</overhang>{}\
         <ignitionconfiguration configid=\"one\"><ignitionevent>burnout</ignitionevent>\
         <ignitiondelay>0.5</ignitiondelay></ignitionconfiguration>{}{}\
         <ignitionconfiguration configid=\"three\"><ignitionevent>ejectioncharge</ignitionevent>\
         <ignitiondelay>1.25</ignitiondelay></ignitionconfiguration>",
        f15("one", "6"),
        f15("two", "4"),
        f15("three", "8"),
    );
    let lower = format!(
        "<ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>\
         <overhang>0.01</overhang>{}{}{}",
        f15("one", "none"),
        f15("two", "3"),
        f15("three", "none"),
    );
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Invented two-stager</name><referencetype>maximum</referencetype>
    <motorconfiguration configid="one" default="true"><name>Staged</name></motorconfiguration>
    <motorconfiguration configid="two"/>
    <motorconfiguration configid="three"/>
    <subcomponents>
      <stage><name>Sustainer</name><id>upper</id><subcomponents>
        <nosecone><name>Nose</name><id>nose</id>
          <overridecg>0.0875</overridecg><overridesubcomponentscg>false</overridesubcomponentscg>
          <material type="bulk" density="1050.0">Invented plastic</material>
          <length>0.14</length><thickness>0.002</thickness><shape>ogive</shape>
          <shapeparameter>1.0</shapeparameter><aftradius>0.0195</aftradius></nosecone>
        <bodytube><name>Upper body</name><id>upper-body</id>
          <overridemass>0.2375</overridemass>
          <overridesubcomponentsmass>true</overridesubcomponentsmass>
          <material type="bulk" density="680.0">Invented card</material>
          <length>0.45</length><thickness>0.0008</thickness><radius>0.0195</radius>
          <subcomponents>
            <innertube><name>Upper mount</name><id>upper-mount</id>
              <axialoffset method="bottom">0.0</axialoffset>
              <material type="bulk" density="680.0">Invented card</material>
              <length>0.15</length><outerradius>0.0152</outerradius>
              <thickness>0.0005</thickness>
              <motormount>{upper}</motormount></innertube>
          </subcomponents></bodytube>
      </subcomponents></stage>
      <stage><name>Booster</name><id>lower</id>
        <separationevent>burnout</separationevent><separationdelay>0.0</separationdelay>
        <subcomponents>
          <bodytube><name>Booster body</name><id>lower-body</id>
            <material type="bulk" density="680.0">Invented card</material>
            <length>0.3</length><thickness>0.0008</thickness><radius>0.0195</radius>
            <motormount>{lower}</motormount></bodytube>
      </subcomponents></stage>
    </subcomponents></rocket>
  <simulations>
    <simulation status="uptodate"><name>Breezy field</name>
      <simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>
      <conditions>
        <configid>one</configid><launchrodlength>1.83</launchrodlength>
        <launchintowind>false</launchintowind><launchrodangle>4.5</launchrodangle>
        <launchroddirection>200.0</launchroddirection><windaverage>3.6</windaverage>
        <windturbulence>0.12</windturbulence><winddirection>0.35</winddirection>
        <wind model="average"><speed>3.6</speed><direction>0.35</direction>
          <standarddeviation>0.432</standarddeviation></wind>
        <windmodeltype>Average</windmodeltype>
        <launchaltitude>1401.5</launchaltitude><launchlatitude>40.5</launchlatitude>
        <launchlongitude>-104.25</launchlongitude><geodeticmethod>spherical</geodeticmethod>
        <atmosphere model="extendedisa"><basetemperature>295.4</basetemperature>
          <basepressure>86000.0</basepressure></atmosphere>
        <timestep>0.01</timestep><maxtime>600.0</maxtime>
      </conditions>
      <flightdata maxaltitude="412.5" maxvelocity="98.25" timetoapogee="9.75"/>
    </simulation>
  </simulations>
</openrocket>"#
    )
}

/// Loft lesson L67. Loft's `.ork` export lost four things a design says and made up a fifth: it
/// wrote a plugged motor's delay as 0 s, which OpenRocket reads as an ejection charge at burnout
/// (42 motors in its corpus); it wrote when a motor lights only at the mount, so every
/// configuration lit the same way; it wrote no launch conditions for a stored simulation; and it
/// wrote the masses it had worked out as overrides, so a part nobody had overridden came back
/// overridden.
///
/// Here a two-stage design with three configurations is written with `write` and read back. The
/// whole design comes back equal; the plugged booster reads back plugged, not as a 0 s delay; the
/// sustainer lights three different ways in three configurations; the simulation's conditions
/// come back one by one; and the only overrides in the written file are the two the design had,
/// each with its own flag as given.
#[test]
fn round_trip_keeps_delays_ignition_conditions_and_override_flags() {
    let original = design_of(staged_design().as_bytes());
    let ids: Vec<&str> = original
        .motors
        .configurations
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(ids, ["one", "two", "three"], "the design is as meant");

    let (back, text, warnings) = written_and_read(&original);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_same(&original, &back);
    assert_eq!(back, original);

    // The motor in `mount` in configuration `id`, in `design`.
    let motor = |design: &Design, id: &str, mount: &str| {
        design
            .motors
            .configurations
            .iter()
            .find(|c| c.id == id)
            .and_then(|c| c.motors.iter().find(|m| m.mount == uuid_of(mount)))
            .cloned()
            .expect("the motor")
    };

    // Plugged is plugged, and a delay is its seconds, in the design read back.
    for (id, delay) in [
        ("one", Delay::Plugged),
        ("two", Delay::Seconds(3.0)),
        ("three", Delay::Plugged),
    ] {
        assert_eq!(
            motor(&original, id, "lower-body").delay,
            Some(delay),
            "{id}"
        );
        assert_eq!(motor(&back, id, "lower-body").delay, Some(delay), "{id}");
    }
    // And in the file: `none`, never a 0 s delay.
    let root = root_of(&text);
    let lower = by_id(&root, "lower-body")
        .child("motormount")
        .expect("the booster's mount");
    for (id, delay) in [("one", "none"), ("two", "3"), ("three", "none")] {
        let written = for_configuration(lower, "motor", id);
        assert_eq!(text_of(written, "delay").as_deref(), Some(delay), "{id}");
    }
    assert!(!text.contains("<delay>0</delay>"), "{text}");
    assert!(!text.contains("<delay>0.0</delay>"), "{text}");

    // Each configuration lights the sustainer its own way.
    for (id, event, delay_s) in [
        ("one", IgnitionEvent::Burnout, 0.5),
        ("two", IgnitionEvent::Automatic, 0.0),
        ("three", IgnitionEvent::EjectionCharge, 1.25),
    ] {
        let ignition = Ignition { event, delay_s };
        assert_eq!(
            motor(&original, id, "upper-mount").ignition,
            ignition,
            "{id}"
        );
        assert_eq!(motor(&back, id, "upper-mount").ignition, ignition, "{id}");
    }
    // In the file, each configuration states its own; the mount's own is OpenRocket's default,
    // since its motors differ.
    let upper = by_id(&root, "upper-mount")
        .child("motormount")
        .expect("the sustainer's mount");
    assert_eq!(
        text_of(upper, "ignitionevent").as_deref(),
        Some("automatic")
    );
    assert_eq!(text_of(upper, "ignitiondelay").as_deref(), Some("0"));
    for (id, event, delay) in [
        ("one", "burnout", "0.5"),
        ("two", "automatic", "0"),
        ("three", "ejectioncharge", "1.25"),
    ] {
        let written = for_configuration(upper, "ignitionconfiguration", id);
        assert_eq!(text_of(written, "ignitionevent").as_deref(), Some(event));
        assert_eq!(text_of(written, "ignitiondelay").as_deref(), Some(delay));
    }

    // The launch conditions, one by one, to the bit.
    let conditions = |design: &Design| {
        design.simulations[0]
            .conditions
            .clone()
            .expect("the conditions")
    };
    let (was, is) = (conditions(&original), conditions(&back));
    let bits = |value: Option<f64>| value.expect("a stated condition").to_bits();
    for (name, was, is) in [
        ("rod length", was.rod_length_m, is.rod_length_m),
        ("rod angle", was.rod_angle_rad, is.rod_angle_rad),
        ("rod direction", was.rod_direction_rad, is.rod_direction_rad),
        ("wind speed", was.wind_speed_m_s, is.wind_speed_m_s),
        ("turbulence", was.wind_turbulence, is.wind_turbulence),
        ("wind direction", was.wind_from_rad, is.wind_from_rad),
        ("altitude", was.launch_altitude_m, is.launch_altitude_m),
        ("latitude", was.latitude_deg, is.latitude_deg),
        ("longitude", was.longitude_deg, is.longitude_deg),
        ("time step", was.time_step_s, is.time_step_s),
        ("maximum time", was.max_time_s, is.max_time_s),
    ] {
        assert_eq!(bits(was), bits(is), "{name}");
    }
    assert_eq!(was.rod_angle_rad, Some(4.5_f64.to_radians()));
    assert_eq!(was.configuration.as_deref(), Some("one"));
    assert_eq!(is.configuration, was.configuration);
    assert_eq!(is.into_wind, Some(false));
    assert_eq!(is.wind_model, was.wind_model);
    assert_eq!(is.wind_levels, was.wind_levels);
    assert_eq!(is.wind_levels_above, was.wind_levels_above);
    assert_eq!(is.geodetic_method.as_deref(), Some("spherical"));
    assert!(was.atmosphere.is_some());
    assert_eq!(is.atmosphere, was.atmosphere);
    for written in [
        "<configid>one</configid>",
        "<launchrodangle>4.5</launchrodangle>",
        "<launchroddirection>200</launchroddirection>",
        "<windaverage>3.6</windaverage>",
        "<standarddeviation>0.432</standarddeviation>",
        "<launchlongitude>-104.25</launchlongitude>",
        "<timestep>0.01</timestep>",
    ] {
        assert!(text.contains(written), "{written} in {text}");
    }

    // The overrides are the design's two, each with its flag as given; nothing is overridden
    // that was not.
    let nose = by_id(&root, "nose");
    assert_eq!(text_of(nose, "overridecg").as_deref(), Some("0.0875"));
    assert_eq!(
        text_of(nose, "overridesubcomponentscg").as_deref(),
        Some("false")
    );
    assert_eq!(text_of(nose, "overridemass"), None);
    assert_eq!(text_of(nose, "overridesubcomponentsmass"), None);
    let body = by_id(&root, "upper-body");
    assert_eq!(text_of(body, "overridemass").as_deref(), Some("0.2375"));
    assert_eq!(
        text_of(body, "overridesubcomponentsmass").as_deref(),
        Some("true")
    );
    assert_eq!(text_of(body, "overridecg"), None);
    assert_eq!(text_of(body, "overridesubcomponentscg"), None);
    for id in ["upper", "lower", "upper-mount", "lower-body"] {
        let element = by_id(&root, id);
        let overrides: Vec<&str> = element
            .elements()
            .map(|child| child.name.as_str())
            .filter(|name| name.starts_with("override"))
            .collect();
        assert!(overrides.is_empty(), "{id} has {overrides:?}");
    }
    assert_eq!(text.matches("<overridemass>").count(), 1, "{text}");
    assert_eq!(text.matches("<overridecg>").count(), 1, "{text}");
    assert!(!part(&back, "nose").overrides_include_children);
    assert!(part(&back, "upper-body").overrides_include_children);
    assert!(part(&back, "lower-body").overrides.is_empty());
}

/// A freeform fin's outline, `[x, h]` from the root leading edge in metres: numbers whose
/// shortest decimals run to 16 and 17 significant digits, such as `0.1 + 0.2` and a third.
fn awkward_outline() -> Vec<[f64; 2]> {
    vec![
        [0.0, 0.0],
        [0.0254 * 7.0 / 16.0, (0.1 + 0.2) / 3.0],
        [0.1 + 0.2 / 3.0, 0.123_456_789_012_345_68],
        [0.187_654_321_098_765_43, 0.098_765_432_109_876_54],
        [0.2 + 0.1 * 0.7, 0.0],
    ]
}

/// Loft lesson L68. Loft's `.ork` export wrote a freeform fin set as the trapezoid of the same
/// area, which OpenRocket then flew with fins 42% too big; it dropped a cluster's scale and
/// rotation, so the motor tubes moved; and it rounded every number to six decimals.
///
/// Here a freeform fin set whose points have 16- and 17-digit coordinates, a four-tube cluster
/// spread by 1.15 and turned 22.5°, and lengths and walls in fractions of an inch are written and
/// read back. The design comes back equal; the fin's points and the cluster's places come back to
/// the bit; the cluster is written with its own scale and rotation, and the tube's roll angle,
/// which turns the pattern too, under `radialdirection`, the name OpenRocket 24.12 reads on an
/// inner tube (it ignores `angleoffset` there, as the small design's test says); and every number
/// in the written file parses to exactly the design's.
#[test]
fn fin_points_clusters_and_floats_round_trip_exactly() {
    let outline = awkward_outline();
    let points: String = outline
        .iter()
        .map(|[x, h]| format!("<point x=\"{x}\" y=\"{h}\"/>"))
        .collect();
    let body_length_m: f64 = 0.3048 * 2.0 + 0.1 * 3.0;
    let body_radius_m: f64 = 0.0254 * 3.0 / 2.0;
    let fin_thickness_m: f64 = 0.0254 / 16.0;
    let mount_length_m: f64 = 0.0254 * 9.5;
    let mount_wall_m: f64 = 0.0254 / 64.0;
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Invented awkward numbers</name><referencetype>maximum</referencetype>
    <subcomponents><stage><name>Sustainer</name><id>stage</id><subcomponents>
      <nosecone><name>Nose</name><id>nose</id>
        <material type="bulk" density="1050.0">Invented plastic</material>
        <length>0.3</length><thickness>0.002</thickness><shape>conical</shape>
        <aftradius>{body_radius_m}</aftradius></nosecone>
      <bodytube><name>Body</name><id>body</id>
        <material type="bulk" density="680.0">Invented card</material>
        <length>{body_length_m}</length><thickness>0.0015</thickness>
        <radius>{body_radius_m}</radius>
        <subcomponents>
          <freeformfinset><name>Fins</name><id>fins</id>
            <instancecount>4</instancecount><axialoffset method="bottom">0.0</axialoffset>
            <material type="bulk" density="1200.0">Invented plywood</material>
            <thickness>{fin_thickness_m}</thickness><crosssection>square</crosssection>
            <finpoints>{points}</finpoints></freeformfinset>
          <innertube><name>Cluster</name><id>cluster</id>
            <axialoffset method="bottom">0.0</axialoffset>
            <material type="bulk" density="680.0">Invented card</material>
            <length>{mount_length_m}</length><outerradius>0.0127</outerradius>
            <thickness>{mount_wall_m}</thickness><radialposition>0.0</radialposition>
            <angleoffset>10.0</angleoffset>
            <clusterconfiguration>4-ring</clusterconfiguration>
            <clusterscale>1.15</clusterscale><clusterrotation>22.5</clusterrotation></innertube>
        </subcomponents></bodytube>
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#
    );
    let original = design_of(xml.as_bytes());

    // The fin's points and the cluster's places, as a design holds them.
    let outline_of = |design: &Design| match &part(design, "fins").part {
        Part::FinSet(fins) => match &fins.planform {
            FinPlanform::Freeform { points_m } => points_m.clone(),
            other => panic!("a freeform fin, not {other:?}"),
        },
        other => panic!("a fin set, not {other:?}"),
    };
    let places_of = |design: &Design| match &part(design, "cluster").part {
        Part::InnerTube(tube) => tube.cluster_m.clone(),
        other => panic!("an inner tube, not {other:?}"),
    };
    let bits = |points: &[[f64; 2]]| -> Vec<[u64; 2]> {
        points
            .iter()
            .map(|[x, y]| [x.to_bits(), y.to_bits()])
            .collect()
    };
    // The reader took the outline exactly as it was written, and made a cluster of four.
    assert_eq!(bits(&outline_of(&original)), bits(&outline));
    assert_eq!(places_of(&original).len(), 4);

    let (back, text, warnings) = written_and_read(&original);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_same(&original, &back);
    assert_eq!(back, original);
    assert_eq!(bits(&outline_of(&back)), bits(&outline));
    assert_eq!(bits(&places_of(&back)), bits(&places_of(&original)));

    // The outline is written as points, not as some trapezoid, and each coordinate parses to the
    // same bits. None of them survives rounding to six decimals, so none was rounded.
    let root = root_of(&text);
    let fins = by_id(&root, "fins");
    assert_eq!(fins.name, "freeformfinset");
    for name in ["rootchord", "tipchord", "sweeplength", "height"] {
        assert_eq!(text_of(fins, name), None, "{name}");
    }
    let written: Vec<[f64; 2]> = fins
        .child("finpoints")
        .expect("the outline")
        .children_named("point")
        .map(|point| {
            let number = |name: &str| -> f64 {
                point
                    .attribute(name)
                    .expect("a coordinate")
                    .parse()
                    .expect("a number")
            };
            [number("x"), number("y")]
        })
        .collect();
    assert_eq!(bits(&written), bits(&outline));
    let six = |value: f64| format!("{value:.6}").parse::<f64>().expect("a number");
    for [x, h] in &outline[1..outline.len() - 1] {
        assert_ne!(six(*x).to_bits(), x.to_bits(), "{x} is awkward");
        assert_ne!(six(*h).to_bits(), h.to_bits(), "{h} is awkward");
    }

    // The cluster keeps its pattern, and its scale and rotation are written, not 1 and 0.
    let cluster = by_id(&root, "cluster");
    assert_eq!(
        text_of(cluster, "clusterconfiguration").as_deref(),
        Some("4-ring")
    );
    assert_eq!(text_of(cluster, "clusterscale").as_deref(), Some("1.15"));
    assert_eq!(text_of(cluster, "clusterrotation").as_deref(), Some("22.5"));
    assert_eq!(text_of(cluster, "radialdirection").as_deref(), Some("10"));
    assert_eq!(text_of(cluster, "angleoffset"), None);

    // The lengths and walls, each needing more than six decimals, parse to the same bits.
    let body = by_id(&root, "body");
    for (element, name, value) in [
        (body, "length", body_length_m),
        (body, "radius", body_radius_m),
        (fins, "thickness", fin_thickness_m),
        (cluster, "length", mount_length_m),
        (cluster, "thickness", mount_wall_m),
    ] {
        let written: f64 = text_of(element, name)
            .expect("the tag")
            .parse()
            .expect("a number");
        assert_eq!(written.to_bits(), value.to_bits(), "{name}");
    }
    assert_ne!(six(mount_wall_m).to_bits(), mount_wall_m.to_bits());
    assert_ne!(six(fin_thickness_m).to_bits(), fin_thickness_m.to_bits());
}

/// A small one-stage design with nothing in it hpr keeps unread: a nose cone, and a body tube
/// holding a parachute, a fin set and an inner tube that mounts a C6 in the design's one
/// configuration.
const SMALL: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Invented small rocket</name><referencetype>maximum</referencetype>
    <motorconfiguration configid="00000000-0000-4000-8000-0000000000c1" default="true"/>
    <subcomponents><stage><name>Sustainer</name><id>00000000-0000-4000-8000-000000000001</id><subcomponents>
      <nosecone><name>Nose cone</name><id>00000000-0000-4000-8000-000000000002</id><finish>normal</finish>
        <material type="bulk" density="1050.0">Invented plastic</material>
        <length>0.09</length><thickness>0.002</thickness><shape>ogive</shape>
        <shapeparameter>1.0</shapeparameter><aftradius>0.0124</aftradius>
        <isflipped>false</isflipped></nosecone>
      <bodytube><name>Body tube</name><id>00000000-0000-4000-8000-000000000003</id><finish>smooth</finish>
        <material type="bulk" density="680.0">Invented card</material>
        <length>0.28</length><thickness>0.0008</thickness><radius>0.0124</radius>
        <subcomponents>
          <parachute><name>Parachute</name><id>00000000-0000-4000-8000-000000000004</id>
            <axialoffset method="top">0.03</axialoffset>
            <packedlength>0.04</packedlength><packedradius>0.009</packedradius>
            <radialposition>0.0</radialposition><cd>auto</cd>
            <material type="surface" density="0.05">Invented nylon</material>
            <deployevent>ejection</deployevent><deployaltitude>200.0</deployaltitude>
            <deploydelay>0.0</deploydelay><diameter>0.3</diameter><linecount>6</linecount>
            <linelength>0.25</linelength>
            <linematerial type="line" density="0.002">Invented cord</linematerial></parachute>
          <trapezoidfinset><name>Fins</name><id>00000000-0000-4000-8000-000000000005</id><instancecount>3</instancecount>
            <axialoffset method="bottom">0.0</axialoffset><finish>normal</finish>
            <material type="bulk" density="680.0">Invented card</material>
            <thickness>0.002</thickness><crosssection>rounded</crosssection><cant>0.0</cant>
            <rootchord>0.05</rootchord><tipchord>0.025</tipchord>
            <sweeplength>0.03</sweeplength><height>0.035</height></trapezoidfinset>
          <innertube><name>Motor mount</name><id>00000000-0000-4000-8000-000000000006</id>
            <axialoffset method="bottom">0.004</axialoffset>
            <material type="bulk" density="680.0">Invented card</material>
            <length>0.07</length><outerradius>0.0095</outerradius><thickness>0.0005</thickness>
            <radialposition>0.0</radialposition>
            <motormount><ignitionevent>automatic</ignitionevent>
              <ignitiondelay>0.0</ignitiondelay><overhang>0.004</overhang>
              <motor configid="00000000-0000-4000-8000-0000000000c1"><type>single</type><manufacturer>Estes</manufacturer>
                <designation>C6</designation><diameter>0.018</diameter><length>0.07</length>
                <delay>5.0</delay></motor>
              <ignitionconfiguration configid="00000000-0000-4000-8000-0000000000c1"><ignitionevent>automatic</ignitionevent>
                <ignitiondelay>0.0</ignitiondelay></ignitionconfiguration></motormount>
          </innertube>
        </subcomponents></bodytube>
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#;

/// The written document for the small design, in full. Each element is one the readers ask for,
/// under the name and in the form OpenRocket 24.12 writes it, derived by hand from the readers'
/// tags: a stage, its parts in the order it holds them, a material with its kind of density, a
/// position with the end it is measured from, the motor in its configuration followed by that
/// configuration's ignition, and the parachute's drag and deployment. A number is its shortest
/// decimal (`5`, not `5.0`), which OpenRocket reads as the same number.
///
/// A part's roll angle is under the name OpenRocket 24.12 reads for its kind: `angleoffset` on a
/// fin set, and `radialdirection` on a parachute and an inner tube. Loaded as an external oracle,
/// OpenRocket 24.12 warns "Unknown parameter type 'angleoffset' for Parachute, ignoring" (and the
/// same for an inner tube, coupler, engine block, streamer, shock cord and mass component) and
/// leaves the angle at zero; it reads `radialdirection` on each of those. The ids are UUIDs
/// because OpenRocket refuses a file whose ids are not.
const SMALL_WRITTEN: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="hpr-sim VERSION">
  <rocket>
    <name>Invented small rocket</name>
    <referencetype>maximum</referencetype>
    <motorconfiguration configid="00000000-0000-4000-8000-0000000000c1" default="true"/>
    <subcomponents>
      <stage>
        <name>Sustainer</name>
        <id>00000000-0000-4000-8000-000000000001</id>
        <subcomponents>
          <nosecone>
            <name>Nose cone</name>
            <id>00000000-0000-4000-8000-000000000002</id>
            <length>0.09</length>
            <aftradius>0.0124</aftradius>
            <thickness>0.002</thickness>
            <shape>ogive</shape>
            <shapeparameter>1</shapeparameter>
            <material type="bulk" density="1050">Invented plastic</material>
            <isflipped>false</isflipped>
            <finish>normal</finish>
          </nosecone>
          <bodytube>
            <name>Body tube</name>
            <id>00000000-0000-4000-8000-000000000003</id>
            <length>0.28</length>
            <radius>0.0124</radius>
            <thickness>0.0008</thickness>
            <material type="bulk" density="680">Invented card</material>
            <finish>smooth</finish>
            <subcomponents>
              <parachute>
                <name>Parachute</name>
                <id>00000000-0000-4000-8000-000000000004</id>
                <axialoffset method="top">0.03</axialoffset>
                <diameter>0.3</diameter>
                <material type="surface" density="0.05">Invented nylon</material>
                <linecount>6</linecount>
                <linelength>0.25</linelength>
                <linematerial type="line" density="0.002">Invented cord</linematerial>
                <packedlength>0.04</packedlength>
                <packedradius>0.009</packedradius>
                <radialposition>0</radialposition>
                <radialdirection>0</radialdirection>
                <cd>auto</cd>
                <deployevent>ejection</deployevent>
                <deployaltitude>200</deployaltitude>
                <deploydelay>0</deploydelay>
              </parachute>
              <trapezoidfinset>
                <name>Fins</name>
                <id>00000000-0000-4000-8000-000000000005</id>
                <axialoffset method="bottom">0</axialoffset>
                <instancecount>3</instancecount>
                <radiusoffset>0</radiusoffset>
                <angleoffset>0</angleoffset>
                <material type="bulk" density="680">Invented card</material>
                <thickness>0.002</thickness>
                <crosssection>rounded</crosssection>
                <cant>0</cant>
                <rootchord>0.05</rootchord>
                <tipchord>0.025</tipchord>
                <sweeplength>0.03</sweeplength>
                <height>0.035</height>
                <finish>normal</finish>
              </trapezoidfinset>
              <innertube>
                <name>Motor mount</name>
                <id>00000000-0000-4000-8000-000000000006</id>
                <axialoffset method="bottom">0.004</axialoffset>
                <length>0.07</length>
                <outerradius>0.0095</outerradius>
                <thickness>0.0005</thickness>
                <radialposition>0</radialposition>
                <radialdirection>0</radialdirection>
                <material type="bulk" density="680">Invented card</material>
                <motormount>
                  <ignitionevent>automatic</ignitionevent>
                  <ignitiondelay>0</ignitiondelay>
                  <overhang>0.004</overhang>
                  <motor configid="00000000-0000-4000-8000-0000000000c1">
                    <type>single</type>
                    <manufacturer>Estes</manufacturer>
                    <designation>C6</designation>
                    <diameter>0.018</diameter>
                    <length>0.07</length>
                    <delay>5</delay>
                  </motor>
                  <ignitionconfiguration configid="00000000-0000-4000-8000-0000000000c1">
                    <ignitionevent>automatic</ignitionevent>
                    <ignitiondelay>0</ignitiondelay>
                  </ignitionconfiguration>
                </motormount>
              </innertube>
            </subcomponents>
          </bodytube>
        </subcomponents>
      </stage>
    </subcomponents>
  </rocket>
</openrocket>
"#;

/// The small design is written as exactly the document above, and reads back as itself.
#[test]
fn a_small_design_is_written_as_expected() {
    let original = design_of(SMALL.as_bytes());
    assert!(
        original.extensions.x_openrocket.tags.is_empty()
            && original.extensions.x_openrocket.attributes.is_empty()
            && original.extensions.x_openrocket.sections.is_empty(),
        "the design keeps nothing unread: {:?}",
        original.extensions
    );
    let written = document(&original);
    assert!(written.warnings.is_empty(), "{:?}", written.warnings);
    let text = written.value.to_xml();
    let expected = SMALL_WRITTEN.replace("VERSION", env!("CARGO_PKG_VERSION"));
    assert_eq!(text, expected);
    assert_eq!(design_of(text.as_bytes()), original);
}

/// `write` puts the design first, as `rocket.ork`, and then each attachment in the order given,
/// byte for byte; an attachment called `rocket.ork` would be a second design, so it is left out
/// with a warning. Reading the file back gives the same attachments and the same design.
#[test]
fn attachments_follow_the_design_and_one_named_like_it_is_skipped() {
    let original = design_of(SMALL.as_bytes());
    let attachment = |name: &str, bytes: &[u8]| Attachment {
        name: name.to_owned(),
        bytes: bytes.to_vec(),
    };
    let curve = attachment(
        "thrustcurves/Invented_G42.rse",
        b"<engine-database>an invented curve</engine-database>",
    );
    let image = attachment("preview.png", b"\x89PNG\r\n\x1a\n\x00\xffinvented pixels");
    let impostor = attachment(DESIGN_ENTRY, b"<openrocket version=\"1.10\"/>");
    let empty = attachment("decals/empty.png", b"");
    let written = write(
        &original,
        &[curve.clone(), impostor, image.clone(), empty.clone()],
    )
    .expect("written");

    assert_eq!(written.warnings.len(), 1, "{:?}", written.warnings);
    let warning = &written.warnings[0];
    assert_eq!(warning.at, DESIGN_ENTRY);
    assert_eq!(warning.kind, super::super::WarningKind::Skipped);
    assert!(warning.message.contains("rocket.ork"), "{warning:?}");

    let entries = entries(&written.value);
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [DESIGN_ENTRY, &curve.name, &image.name, &empty.name],
        "the design first, the impostor left out"
    );
    assert_eq!(
        entries[0].1,
        document(&original).value.to_xml().into_bytes(),
        "the design entry is the design's document"
    );

    let file = read(&written.value).expect("readable").value;
    assert_eq!(file.design_entry.as_deref(), Some(DESIGN_ENTRY));
    assert_eq!(file.attachments, [curve, image, empty]);
    assert_eq!(design(&file).value, original);
}

/// OpenRocket 24.12 will not open a file holding an id that is not a UUID, so the export leaves
/// such an id out and OpenRocket gives the part one of its own. A part the file gave no id reads
/// with an id the reader invents, and reading the export invents the same one again, so that one
/// goes unremarked; a part the file named some other way reads back under an invented id instead,
/// and the export warns of it.
#[test]
fn ids_openrocket_would_refuse_are_left_out() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Probe</name><referencetype>maximum</referencetype>
    <subcomponents><stage><name>Sustainer</name><id>sustainer</id><subcomponents>
      <nosecone><name>Nose</name>
        <material type="bulk" density="1000.0">Invented plastic</material>
        <length>0.2</length><thickness>0.002</thickness><shape>conical</shape>
        <aftradius>0.02</aftradius></nosecone>
      <bodytube><name>Tube</name><id>0f0e0d0c-0b0a-4900-8800-070605040302</id>
        <material type="bulk" density="700.0">Invented paper</material>
        <length>0.5</length><thickness>0.001</thickness><radius>0.02</radius></bodytube>
    </subcomponents></stage></subcomponents></rocket>
</openrocket>"#;
    let file = read(xml).expect("readable").value;
    let original = design(&file).value;
    let written = document(&original);
    let text = written.value.to_xml();
    assert!(!text.contains("<id>sustainer</id>"), "{text}");
    // One warning, for the stage's id; none for the nose cone's, which the reader invented.
    assert_eq!(written.warnings.len(), 1, "{:?}", written.warnings);
    let warning = &written.warnings[0];
    assert_eq!(warning.at, "openrocket/rocket/stage[0]");
    assert_eq!(warning.kind, super::super::WarningKind::Dropped);
    assert!(
        warning.message.contains("the id `sustainer` is not a UUID"),
        "{}",
        warning.message
    );
    assert_eq!(text.matches("<id>").count(), 1, "{text}");
    let back = design(&read(text.as_bytes()).expect("readable").value).value;
    let (was, is) = (&original.rocket.stages[0], &back.rocket.stages[0]);
    assert_eq!(was.id, "sustainer");
    assert_ne!(is.id, was.id);
    let ids = |stage: &hpr_design::tree::Stage| {
        stage
            .components
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(is), ids(was));
    assert_eq!(ids(was)[1], "0f0e0d0c-0b0a-4900-8800-070605040302");
}
