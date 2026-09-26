//! Flying what an OpenRocket `.ork` file says about staging.
//!
//! `hpr_io` reads a `.ork` file's one powered separation as an [`hpr_io::ork::Staging`], naming the
//! motor it is timed from by its mount, since `hpr_io` does not depend on `hpr_sim`.
//! [`separation`] turns it into the flight's [`hpr_sim::Separation`], naming that motor by its index
//! among the assembled motors. The flight also needs a recovery device on each of the two bodies
//! the separation makes (the decision record on staging, [ADR-074][adr-074]); the example
//! `ork_two_stage` in this crate shows one way. The decision record for reading a `.ork` file's
//! staging is [ADR-076][adr-076].
//!
//! [adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
//! [adr-076]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-076-a-ork-files-ignitions-and-one-powered-separation-flown-against-openrocket-2026-09-25

use hpr_design::Assembly;
use hpr_io::ork::{Staging, StagingTrigger};
use hpr_sim::{Separation, SimError, Trigger};

/// The flight's separation for `staging`, with the motors of `assembly`, the configuration the
/// `.ork` file's staging belongs to, assembled.
///
/// # Errors
///
/// [`SimError::Domain`] if the mount a separation is timed from places no lit motor in `assembly`
/// (it is not that configuration's, or every tube of it is set to fail), or if `staging` holds a trigger this function does not know.
pub fn separation(staging: &Staging, assembly: &Assembly) -> Result<Separation, SimError> {
    let trigger = match &staging.trigger {
        StagingTrigger::Time { time_s } => Trigger::Time { time_s: *time_s },
        StagingTrigger::Burnout { mount, delay_s } => Trigger::Burnout {
            // A cluster's tubes light together, so its first lit tube's burnout is the mount's, as
            // `Assembly::ignition_times_s` takes it.
            motor: assembly
                .motors
                .iter()
                .position(|motor| motor.mounted.mount == *mount && !motor.fails)
                .ok_or(SimError::Domain {
                    what: "count of lit motors in the mount a separation is timed from",
                    value: 0.0,
                })?,
            delay_s: *delay_s,
        },
        _ => {
            return Err(SimError::Domain {
                what: "kind of `.ork` separation trigger (one this version does not know)",
                value: f64::NAN,
            });
        }
    };
    Ok(Separation::new(trigger, staging.after_stage))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two stages, an Estes F15 from the bundled catalog in each: configuration `burn` drops the
    /// booster at its burnout and lights the sustainer there; `time` drops it 4 s after launch
    /// and lights the sustainer at 6 s.
    const ORK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Two-stage</name>
    <motorconfiguration configid="burn"/><motorconfiguration configid="time"/>
    <subcomponents>
      <stage><name>Sustainer</name><id>upper</id><subcomponents>
        <nosecone><name>Nose</name><id>nose</id><length>0.15</length><thickness>0.002</thickness>
          <shape>ogive</shape><aftradius>0.0165</aftradius></nosecone>
        <bodytube><name>Sustainer</name><id>sustainer</id><length>0.4</length>
          <thickness>0.001</thickness><radius>0.0165</radius>
          <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
            <overhang>0.0</overhang>
            <motor configid="burn"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>6.0</delay></motor>
            <motor configid="time"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>6.0</delay></motor>
            <ignitionconfiguration configid="time"><ignitionevent>launch</ignitionevent>
              <ignitiondelay>6.0</ignitiondelay></ignitionconfiguration>
          </motormount></bodytube></subcomponents></stage>
      <stage><name>Booster</name><id>lower</id>
        <separationevent>burnout</separationevent><separationdelay>0.0</separationdelay>
        <separationconfiguration configid="time"><separationevent>launch</separationevent>
          <separationdelay>4.0</separationdelay></separationconfiguration>
        <subcomponents>
        <bodytube><name>Booster</name><id>booster</id><length>0.3</length>
          <thickness>0.001</thickness><radius>0.0165</radius>
          <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
            <overhang>0.0</overhang>
            <motor configid="burn"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>0.0</delay></motor>
            <motor configid="time"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>0.0</delay></motor>
          </motormount></bodytube></subcomponents></stage>
    </subcomponents></rocket>
</openrocket>"#;

    /// The configuration's staging and its assembly.
    fn read(id: &str) -> (Staging, Assembly) {
        let file = hpr_io::ork::read(ORK.as_bytes()).expect("a readable design");
        let design = hpr_io::ork::design(&file.value).value;
        let configuration = design
            .motors
            .configurations
            .iter()
            .find(|configuration| configuration.id == id)
            .expect("the configuration");
        let staging = configuration.staging.clone().expect("a separation");
        (staging, design.rocket.assemble(id).expect("flown"))
    }

    #[test]
    fn a_burnout_separation_names_the_booster_motor_by_its_index() {
        let (staging, assembly) = read("burn");
        let booster = assembly
            .motors
            .iter()
            .position(|motor| motor.mount == "booster")
            .expect("the booster's motor");
        assert_eq!(
            separation(&staging, &assembly).expect("maps"),
            Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: 0.0
                },
                0
            )
        );

        // A tube set to fail is passed over for the first that lights, as the design's own
        // ignition times take it.
        let mut failing = assembly.clone();
        let mut dud = failing.motors[booster].clone();
        dud.fails = true;
        failing.motors.insert(booster, dud);
        assert_eq!(
            separation(&staging, &failing).expect("maps").trigger,
            Trigger::Burnout {
                motor: booster + 1,
                delay_s: 0.0
            }
        );

        // A mount whose every tube fails, or another configuration's assembly with no motor in
        // that mount, is refused.
        let mut dead = assembly.clone();
        dead.motors[booster].fails = true;
        assert!(matches!(
            separation(&staging, &dead),
            Err(SimError::Domain {
                what: "count of lit motors in the mount a separation is timed from",
                ..
            })
        ));
        let mut other = assembly;
        other.motors.retain(|motor| motor.mount != "booster");
        let error = separation(&staging, &other).expect_err("no booster motor");
        assert!(
            matches!(
                error,
                SimError::Domain {
                    what: "count of lit motors in the mount a separation is timed from",
                    ..
                }
            ),
            "{error}"
        );
    }

    #[test]
    fn a_separation_at_a_time_stays_a_time() {
        let (staging, assembly) = read("time");
        assert_eq!(
            separation(&staging, &assembly).expect("maps"),
            Separation::new(Trigger::Time { time_s: 4.0 }, 0)
        );
    }
}
