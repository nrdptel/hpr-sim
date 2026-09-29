//! The simulations OpenRocket last ran, with their conditions and results, written as
//! [`super::super::simulations`] reads them.
//!
//! Each tag goes where OpenRocket 24.12 writes it, in its order, with the number the design holds.
//! Two things are written in more than one place, as OpenRocket writes them:
//!
//! - **The average wind.** A schema-1.10 file gives it twice: in the older tags `<windaverage>`
//!   and `<winddirection>`, and in a `<wind model="average">` block beside the multilevel one.
//!   The reader takes the older tags first. Both are written, with the same numbers, whenever the
//!   design came from a file that names its wind model (`<windmodeltype>`) or has a multilevel
//!   wind: those are the files that write the blocks. A file without them, written by an older
//!   OpenRocket, gets the older tags alone, as it had. The block's `<standarddeviation>`, which hpr
//!   does not read, is kept with the design and put back inside it.
//! - **The rod's tilt and direction** are held in radians and written in degrees, as OpenRocket
//!   writes them ([ADR-057][adr-057]). The degrees written are a number that reads back as exactly
//!   the same radians: the shortest such number, so 90° is written `90`.
//!
//! A `<datapoint>` row is written as its numbers, comma-separated, each exactly; a value
//! OpenRocket did not compute is written `NaN`, as OpenRocket writes it.
//!
//! [adr-057]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-057-a-ork-designs-stored-simulations-read-back-as-written-with-their-units-measured-2026-09-21

use super::super::Design;
use super::super::document::Element;
use super::super::simulations::{
    Atmosphere, LaunchConditions, StoredBranch, StoredEvent, StoredResults, StoredSimulation,
    WindLevel,
};
use super::super::warning::{Warning, WarningKind};
use super::xml::{self, Build as _};

/// The path of the `<simulations>` element, as warnings and kept items give it.
const SIMULATIONS: &str = "openrocket/simulations";

/// How many neighbouring numbers either side of the first guess are tried when looking for the
/// degrees that read back as an angle's radians. Converting there and back is off by a few units
/// in the last place at most, so eight is ample.
const NEIGHBOURS: usize = 8;

/// The `<simulations>` element, or `None` when the design has none and keeps nothing that was
/// inside the file's `<simulations>`.
///
/// A file can have a `<simulations>` with no `<simulation>` in it but with something else hpr
/// keeps; the element is written then too, so that what was kept has somewhere to go back.
pub(super) fn simulations(design: &Design, warnings: &mut Vec<Warning>) -> Option<Element> {
    let kept = &design.extensions.x_openrocket;
    let keeps_something_inside = kept
        .sections
        .iter()
        .chain(&kept.tags)
        .map(|kept| kept.at.as_str())
        .chain(kept.attributes.iter().map(|kept| kept.at.as_str()))
        .any(|at| {
            at.strip_prefix(SIMULATIONS)
                .is_some_and(|rest| rest.starts_with(['/', '[']))
        });
    if design.simulations.is_empty() && !keeps_something_inside {
        return None;
    }
    let mut element = xml::element("simulations");
    for (index, stored) in design.simulations.iter().enumerate() {
        let at = format!("{SIMULATIONS}/simulation[{index}]");
        element.push(simulation(stored, &at, warnings));
    }
    Some(element)
}

/// One `<simulation>`: its status, name, simulator, calculator, conditions and results.
fn simulation(stored: &StoredSimulation, at: &str, warnings: &mut Vec<Warning>) -> Element {
    let mut element = xml::element("simulation");
    if let Some(status) = &stored.status {
        element.with_attribute("status", status.clone());
    }
    // The reader takes a missing name as an empty one, so an empty one is written either way.
    element.leaf("name", stored.name.clone());
    if let Some(simulator) = &stored.simulator {
        element.leaf("simulator", simulator.clone());
    }
    if let Some(calculator) = &stored.calculator {
        element.leaf("calculator", calculator.clone());
    }
    if let Some(conditions) = &stored.conditions {
        element.push(launch_conditions(
            conditions,
            &format!("{at}/conditions"),
            warnings,
        ));
    }
    if let Some(results) = &stored.results {
        element.push(flight_data(results, &format!("{at}/flightdata"), warnings));
    }
    element
}

/// `<conditions>`: every launch condition the design holds, in the order OpenRocket 24.12 writes
/// them.
fn launch_conditions(
    conditions: &LaunchConditions,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> Element {
    let mut element = xml::element("conditions");
    if let Some(configuration) = &conditions.configuration {
        element.leaf("configid", configuration.clone());
    }
    put(
        &mut element,
        &[("launchrodlength", conditions.rod_length_m)],
        at,
        warnings,
    );
    if let Some(into_wind) = conditions.into_wind {
        element.flag("launchintowind", into_wind);
    }
    for (name, radians) in [
        ("launchrodangle", conditions.rod_angle_rad),
        ("launchroddirection", conditions.rod_direction_rad),
    ] {
        let Some(radians) = radians else {
            continue;
        };
        match degrees(radians) {
            Some(degrees) => put(&mut element, &[(name, Some(degrees))], at, warnings),
            None => warnings.push(Warning::new(
                at,
                WarningKind::Dropped,
                format!(
                    "`{name}` is {radians} rad, which no number of degrees reads back as \
                     exactly; it was left out"
                ),
            )),
        }
    }
    put(
        &mut element,
        &[
            ("windaverage", conditions.wind_speed_m_s),
            ("windturbulence", conditions.wind_turbulence),
            ("winddirection", conditions.wind_from_rad),
        ],
        at,
        warnings,
    );
    let multilevel = !conditions.wind_levels.is_empty() || conditions.wind_levels_above.is_some();
    if conditions.wind_model.is_some() || multilevel {
        let mut average = xml::element("wind");
        average.with_attribute("model", "average");
        put(
            &mut average,
            &[
                ("speed", conditions.wind_speed_m_s),
                ("direction", conditions.wind_from_rad),
            ],
            at,
            warnings,
        );
        element.push(average);
    }
    if multilevel {
        let mut wind = xml::element("wind");
        wind.with_attribute("model", "multilevel");
        if let Some(above) = &conditions.wind_levels_above {
            wind.with_attribute("altituderef", above.clone());
        }
        for level in &conditions.wind_levels {
            wind.push(wind_level(level, at, warnings));
        }
        element.push(wind);
    }
    if let Some(model) = &conditions.wind_model {
        element.leaf("windmodeltype", model.clone());
    }
    put(
        &mut element,
        &[
            ("launchaltitude", conditions.launch_altitude_m),
            ("launchlatitude", conditions.latitude_deg),
            ("launchlongitude", conditions.longitude_deg),
        ],
        at,
        warnings,
    );
    if let Some(method) = &conditions.geodetic_method {
        element.leaf("geodeticmethod", method.clone());
    }
    if let Some(atmosphere) = &conditions.atmosphere {
        let mut written = xml::element("atmosphere");
        match atmosphere {
            Atmosphere::Isa => {
                written.with_attribute("model", "isa");
            }
            Atmosphere::Extended {
                temperature_k,
                pressure_pa,
            } => {
                written.with_attribute("model", "extendedisa");
                put(
                    &mut written,
                    &[
                        ("basetemperature", *temperature_k),
                        ("basepressure", *pressure_pa),
                    ],
                    at,
                    warnings,
                );
            }
            // An empty name is an atmosphere that had no `model`; it is written without one.
            Atmosphere::Other { name } => {
                if !name.is_empty() {
                    written.with_attribute("model", name.clone());
                }
            }
        }
        element.push(written);
    }
    put(
        &mut element,
        &[
            ("timestep", conditions.time_step_s),
            ("maxtime", conditions.max_time_s),
        ],
        at,
        warnings,
    );
    element
}

/// Adds `<name>value</name>` to `element` for each of `numbers` that is set and finite, in turn.
fn put(
    element: &mut Element,
    numbers: &[(&str, Option<f64>)],
    at: &str,
    warnings: &mut Vec<Warning>,
) {
    for &(name, value) in numbers {
        if let Some(value) = finite(value, at, name, warnings) {
            element.number(name, value);
        }
    }
}

/// One `<windlevel>` of a multilevel wind, its numbers as attributes.
fn wind_level(level: &WindLevel, at: &str, warnings: &mut Vec<Warning>) -> Element {
    let mut element = xml::element("windlevel");
    for (name, value) in [
        ("altitude", level.altitude_m),
        ("speed", level.speed_m_s),
        ("direction", level.from_rad),
        ("standarddeviation", level.standard_deviation_m_s),
    ] {
        if let Some(value) = finite(value, at, name, warnings) {
            element.with_attribute(name, xml::number(value));
        }
    }
    element
}

/// `<flightdata>`: the summary as attributes, then the warnings, then a `<databranch>` per stage.
fn flight_data(results: &StoredResults, at: &str, warnings: &mut Vec<Warning>) -> Element {
    let mut element = xml::element("flightdata");
    for (name, value) in [
        ("maxaltitude", results.max_altitude_m),
        ("maxvelocity", results.max_speed_m_s),
        ("maxacceleration", results.max_acceleration_m_s2),
        ("maxmach", results.max_mach),
        ("timetoapogee", results.time_to_apogee_s),
        ("flighttime", results.flight_time_s),
        ("groundhitvelocity", results.ground_hit_speed_m_s),
        ("launchrodvelocity", results.rod_exit_speed_m_s),
        ("deploymentvelocity", results.deployment_speed_m_s),
        ("optimumdelay", results.optimum_delay_s),
    ] {
        if let Some(value) = finite(value, at, name, warnings) {
            element.with_attribute(name, xml::number(value));
        }
    }
    // A warning OpenRocket 24.12 writes as tags inside it reads as empty text; the tags are kept
    // with the design and put back inside it.
    for warning in &results.warnings {
        element.leaf("warning", warning.clone());
    }
    for (index, branch) in results.branches.iter().enumerate() {
        element.push(data_branch(
            branch,
            &format!("{at}/databranch[{index}]"),
            warnings,
        ));
    }
    element
}

/// One `<databranch>`: its name and columns, its events, then its rows.
fn data_branch(branch: &StoredBranch, at: &str, warnings: &mut Vec<Warning>) -> Element {
    let mut element = xml::element("databranch");
    element.with_attribute("name", branch.name.clone());
    // No columns is no `types`: an empty one would read back as one column with no name.
    if !branch.types.is_empty() {
        element.with_attribute("types", branch.types.join(","));
    }
    for event in &branch.events {
        if let Some(written) = flight_event(event, at, warnings) {
            element.push(written);
        }
    }
    let mut unfit = 0usize;
    for row in &branch.rows {
        // A row the reader would leave out is left out here, rather than written for OpenRocket
        // to trip on.
        if row.is_empty() || row.len() != branch.types.len() {
            unfit += 1;
            continue;
        }
        let text: Vec<String> = row
            .iter()
            .map(|value| match value {
                Some(value) if value.is_finite() => xml::number(*value),
                _ => "NaN".to_owned(),
            })
            .collect();
        element.leaf("datapoint", text.join(","));
    }
    if unfit > 0 {
        warnings.push(Warning::new(
            at,
            WarningKind::Dropped,
            format!(
                "{unfit} row(s) do not have one value per column, {} of them; they were left out",
                branch.types.len()
            ),
        ));
    }
    element
}

/// One `<event>`, or `None` with a warning when its time is not a finite number.
fn flight_event(event: &StoredEvent, at: &str, warnings: &mut Vec<Warning>) -> Option<Element> {
    let time_s = finite(Some(event.time_s), at, "an event's `time`", warnings)?;
    let mut element = xml::element("event");
    element
        .with_attribute("time", xml::number(time_s))
        .with_attribute("type", event.kind.clone());
    if let Some(source) = &event.source {
        element.with_attribute("source", source.clone());
    }
    Some(element)
}

/// `value` when it is a finite number, which is all a `.ork` holds; otherwise `None`, with a
/// warning when there was a value to leave out.
fn finite(value: Option<f64>, at: &str, what: &str, warnings: &mut Vec<Warning>) -> Option<f64> {
    let value = value?;
    if value.is_finite() {
        return Some(value);
    }
    warnings.push(Warning::new(
        at,
        WarningKind::Dropped,
        format!("{what} is {value}, which a `.ork` cannot hold; it was left out"),
    ));
    None
}

/// The degrees whose conversion to radians, as the reader makes it (`f64::to_radians`), gives
/// exactly `radians`: the shortest to write of those near `radians` in degrees, or `None` when
/// there is none.
///
/// Converting to degrees and back can be a unit in the last place off, so the neighbours of the
/// first guess are tried too.
fn degrees(radians: f64) -> Option<f64> {
    let guess = radians.to_degrees();
    if !guess.is_finite() {
        return None;
    }
    let mut candidates = vec![guess];
    let (mut below, mut above) = (guess, guess);
    for _ in 0..NEIGHBOURS {
        below = below.next_down();
        above = above.next_up();
        candidates.extend([below, above]);
    }
    candidates
        .into_iter()
        .filter(|degrees| degrees.to_radians().to_bits() == radians.to_bits())
        // The first of the shortest, which is the one nearest the guess.
        .min_by_key(|degrees| xml::number(*degrees).len())
}

#[cfg(test)]
mod tests {
    use super::super::super::{Design, design, read};
    use super::super::document;
    use super::*;

    /// The design a `.ork` document reads as.
    fn design_of(xml: &str) -> Design {
        let file = read(xml.as_bytes()).expect("a readable .ork").value;
        design(&file).value
    }

    /// Writes `design` and reads it back, returning the design read and the text written.
    fn round_trip(design: &Design) -> (Design, String) {
        let written = document(design);
        let text = written.value.to_xml();
        (design_of(&text), text)
    }

    /// A document holding `simulations` beside a bare rocket.
    fn with_simulations(simulations: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12"><rocket><name>R</name></rocket>
<simulations>{simulations}</simulations></openrocket>"#
        )
    }

    /// Every condition the reader takes, each with an invented value, reads back the same, and
    /// the angles are written in the degrees they were read from.
    #[test]
    fn every_condition_round_trips() {
        let original = design_of(&with_simulations(
            r#"<simulation status="uptodate"><name>Full</name>
  <simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>
  <conditions>
    <configid>cfg-1</configid><launchrodlength>1.5</launchrodlength>
    <launchintowind>false</launchintowind><launchrodangle>3.5</launchrodangle>
    <launchroddirection>275.25</launchroddirection><windaverage>4.25</windaverage>
    <windturbulence>0.15</windturbulence><winddirection>2.1</winddirection>
    <wind model="average"><speed>4.25</speed><direction>2.1</direction>
      <standarddeviation>0.6375</standarddeviation></wind>
    <wind model="multilevel" altituderef="agl">
      <windlevel altitude="0" speed="3.1" direction="1.2" standarddeviation="0.4"/>
      <windlevel altitude="250.5" speed="6.7" direction="1.35" standarddeviation="0.9"/>
      <windlevel altitude="900" speed="11.2" direction="1.6" standarddeviation="1.3"/>
    </wind>
    <windmodeltype>Average</windmodeltype><launchaltitude>612.4</launchaltitude>
    <launchlatitude>-33.7</launchlatitude><launchlongitude>151.05</launchlongitude>
    <geodeticmethod>wgs84</geodeticmethod>
    <atmosphere model="extendedisa"><basetemperature>301.3</basetemperature>
      <basepressure>99870</basepressure></atmosphere>
    <timestep>0.02</timestep><maxtime>900</maxtime>
  </conditions></simulation>"#,
        ));
        let conditions = original.simulations[0]
            .conditions
            .as_ref()
            .expect("conditions");
        assert_eq!(conditions.wind_levels.len(), 3);
        assert!(conditions.max_time_s.is_some() && conditions.atmosphere.is_some());
        let (back, text) = round_trip(&original);
        assert_eq!(back.simulations, original.simulations);
        assert!(!back.simulations[0].parser_warnings);
        for written in [
            "<launchrodangle>3.5</launchrodangle>",
            "<launchroddirection>275.25</launchroddirection>",
            r#"<wind model="average">"#,
            r#"<windlevel altitude="250.5" speed="6.7" direction="1.35" standarddeviation="0.9"/>"#,
        ] {
            assert!(text.contains(written), "{written} in {text}");
        }
    }

    /// A file from an older OpenRocket, with the wind only in the older tags and an atmosphere
    /// of its own, is written the way it came: no wind blocks, and the atmosphere with no model.
    #[test]
    fn an_older_files_conditions_stay_in_the_older_tags() {
        for atmosphere in ["<atmosphere/>", r#"<atmosphere model="isa"/>"#] {
            let original = design_of(&with_simulations(&format!(
                r"<simulation><name>Old</name><conditions>
  <launchrodlength>1</launchrodlength><launchrodangle>0</launchrodangle>
  <launchroddirection>0</launchroddirection><windaverage>2</windaverage>
  <windturbulence>0.1</windturbulence>{atmosphere}<timestep>0.01</timestep>
  </conditions></simulation>"
            )));
            let (back, text) = round_trip(&original);
            assert_eq!(back.simulations, original.simulations);
            assert!(!text.contains("<wind "), "{text}");
            assert!(!text.contains("<windmodeltype>"), "{text}");
        }
    }

    /// Flight data with its summary, a warning, and two branches with events and rows, one with a
    /// value OpenRocket did not compute, reads back the same; the uncomputed value is `NaN`.
    #[test]
    fn flight_data_round_trips() {
        let original = design_of(&with_simulations(
            r#"<simulation status="uptodate"><name>Flown</name>
  <flightdata maxaltitude="812.5" maxvelocity="160.25" maxacceleration="210.75" maxmach="0.48"
    timetoapogee="12.5" flighttime="75.25" groundhitvelocity="5.5" launchrodvelocity="18.2"
    deploymentvelocity="12.4" optimumdelay="9.75">
    <warning>Invented warning text</warning>
    <databranch name="Sustainer" types="Time,Altitude,Mach number">
      <event time="0" type="launch" source="part-1"/>
      <event time="12.5" type="apogee"/>
      <datapoint>0,0,NaN</datapoint>
      <datapoint>0.01,0.000125,1.5e-5</datapoint>
      <datapoint>12.5,812.5,0.0125</datapoint>
    </databranch>
    <databranch name="Booster" types="Time,Altitude">
      <event time="3.25" type="stageseparation" source="stage-2"/>
      <datapoint>3.25,120.5</datapoint>
    </databranch>
  </flightdata></simulation>"#,
        ));
        let results = original.simulations[0].results.as_ref().expect("results");
        assert_eq!(results.branches.len(), 2);
        assert_eq!(results.branches[0].rows[0][2], None);
        assert_eq!(results.warnings, ["Invented warning text"]);
        let (back, text) = round_trip(&original);
        assert_eq!(back.simulations, original.simulations);
        assert!(text.contains("<datapoint>0,0,NaN</datapoint>"), "{text}");
        assert!(
            text.contains("<datapoint>0.01,0.000125,0.000015</datapoint>"),
            "{text}"
        );
    }

    /// A simulation with no flight data, one with nothing but a name, and a structured warning
    /// whose parts are kept rather than read all read back the same.
    #[test]
    fn sparse_simulations_round_trip() {
        let original = design_of(&with_simulations(
            r#"<simulation status="notsimulated"><name>Never flown</name>
  <simulator>RK4Simulator</simulator><conditions><configid>c</configid></conditions>
</simulation>
<simulation><name/></simulation>
<simulation status="outdated"><name>Warned</name><flightdata maxaltitude="10">
  <warning type="Invented"><id>w-1</id><description>Text</description></warning>
</flightdata></simulation>"#,
        ));
        assert_eq!(original.simulations.len(), 3);
        assert!(original.simulations[0].results.is_none());
        let (back, _) = round_trip(&original);
        assert_eq!(back.simulations, original.simulations);
    }

    /// A design with no simulations writes no `<simulations>`.
    #[test]
    fn no_simulations_writes_none() {
        let original = design_of(
            r#"<openrocket version="1.10" creator="OpenRocket 24.12"><rocket><name>R</name></rocket></openrocket>"#,
        );
        let mut warnings = Vec::new();
        assert!(simulations(&original, &mut warnings).is_none());
        assert!(warnings.is_empty());
    }

    /// A number the reader would drop is left out, with a warning, rather than written as text
    /// OpenRocket cannot read.
    #[test]
    fn a_number_a_ork_cannot_hold_is_left_out() {
        let mut original = design_of(&with_simulations(
            r#"<simulation><name>S</name><conditions><timestep>0.01</timestep></conditions>
  <flightdata maxaltitude="10"><databranch name="B" types="Time">
    <event time="1" type="apogee"/><datapoint>1</datapoint></databranch></flightdata>
</simulation>"#,
        ));
        let simulation = &mut original.simulations[0];
        simulation
            .conditions
            .as_mut()
            .expect("conditions")
            .time_step_s = Some(f64::NAN);
        let results = simulation.results.as_mut().expect("results");
        results.max_altitude_m = Some(f64::INFINITY);
        results.branches[0].events[0].time_s = f64::NAN;
        results.branches[0].rows[0][0] = Some(f64::INFINITY);
        results.branches[0].rows.push(vec![Some(1.0), Some(2.0)]);
        let mut warnings = Vec::new();
        let written = simulations(&original, &mut warnings).expect("written");
        assert_eq!(warnings.len(), 4, "{warnings:?}");
        let text = document(&original).value.to_xml();
        assert!(!text.contains("inf"), "{text}");
        assert!(text.contains("<datapoint>NaN</datapoint>"), "{text}");
        assert!(!text.contains("<timestep>"), "{text}");
        assert_eq!(written.children_named("simulation").count(), 1);
    }

    /// Every angle in degrees reads back through radians as the degrees written, or as degrees
    /// that read as the same radians; round numbers are written round.
    #[test]
    fn degrees_invert_the_readers_conversion_exactly() {
        assert_eq!(degrees(90_f64.to_radians()), Some(90.0));
        assert_eq!(degrees(0.0), Some(0.0));
        assert_eq!(
            degrees((-0.0_f64).to_radians()).map(f64::to_bits),
            Some((-0.0_f64).to_bits())
        );
        // An invented spread of angles, stepped by an irrational fraction of a degree.
        let mut value = -720.0_f64;
        while value < 720.0 {
            for degrees_in in [value, value / 7.0, value * 1e-9, value * 1e6] {
                let radians = degrees_in.to_radians();
                let written = degrees(radians).expect("a preimage");
                assert_eq!(
                    written.to_radians().to_bits(),
                    radians.to_bits(),
                    "{degrees_in}"
                );
                assert!(
                    xml::number(written).len() <= xml::number(degrees_in).len(),
                    "{degrees_in} wrote {written}"
                );
            }
            value += 0.618_033_988_749_894_9;
        }
        assert_eq!(degrees(f64::MAX), None);
    }
}
