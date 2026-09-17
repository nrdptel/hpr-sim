//! Whole-flight tests: the analytic cases of M1.6 and Loft lessons L20, L24 and L25.

use std::f64::consts::PI;

use hpr_aero::{AeroError, Flow};
use hpr_atmos::ConstantWind;
use hpr_core::{DQuat, DVec3};

use crate::environment::Environment;
use crate::error::SimError;
use crate::events::{Direction, find_root};
use crate::flight::{EventKind, FlightSettings, Simulation, Termination, UserEvent};
use crate::rail::Rail;
use crate::recorder::{Channel, Recorder};
use crate::state::State;
use crate::testing::{
    UniformAir, analytic_environment, constant_drag, design, site, windy_environment,
};

const G: f64 = 9.806_65;

/// Valetudo (K400C), whose motor burns out at 3.26 s: the analytic cases fly it from t = 10 s.
fn valetudo(environment: Environment, settings: FlightSettings) -> Simulation {
    Simulation::new(
        &design("rocketpy-valetudo"),
        "example",
        environment,
        Rail::vertical(3.0),
        settings,
    )
    .unwrap()
}

fn capped(max_time_s: f64) -> FlightSettings {
    FlightSettings {
        max_time_s,
        ..FlightSettings::default()
    }
}

/// Column `name` of every row.
fn column(recorder: &Recorder, name: &str) -> Vec<f64> {
    let index = recorder
        .columns()
        .iter()
        .position(|c| c == name)
        .unwrap_or_else(|| panic!("no column {name}"));
    recorder.rows().iter().map(|row| row[index]).collect()
}

#[test]
fn vacuum_ballistic_flight_follows_the_parabola() {
    // In a vacuum under uniform gravity the centre of mass follows a parabola whatever the body
    // does, and the angular momentum about it is constant. The rocket starts tumbling and spinning,
    // with the nose tip (the state's reference point) well away from the centre of mass.
    let t0 = 10.0;
    let sim = valetudo(
        analytic_environment(UniformAir::vacuum(), G),
        FlightSettings::default(),
    );
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 500.0),
        velocity_enu_m_s: DVec3::new(30.0, -20.0, 80.0),
        attitude: DQuat::from_rotation_z(0.3)
            * DQuat::from_rotation_y(0.4)
            * DQuat::from_rotation_x(0.2),
        body_rate_rad_s: DVec3::new(0.4, -0.3, 3.0),
    };
    let mass = sim.assembly().mass_properties(t0);
    let q = state.unit_attitude();
    let cg0 = state.point_enu_m(mass.cg_m);
    let v_cg0 = state.velocity_enu_m_s + q.mul_vec3(state.body_rate_rad_s.cross(mass.cg_m));
    let h0 = q.mul_vec3(mass.inertia_kg_m2 * state.body_rate_rad_s);
    let parabola = |t: f64| {
        let tau = t - t0;
        cg0 + v_cg0 * tau - DVec3::Z * (0.5 * G * tau * tau)
    };

    let mut recorder = Recorder::new(
        vec![
            Channel::Time,
            Channel::CgPosition,
            Channel::Attitude,
            Channel::BodyRates,
        ],
        Some(0.5),
    )
    .unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::GroundHit);

    let mut worst_position: f64 = 0.0;
    let mut worst_momentum: f64 = 0.0;
    for row in recorder.rows() {
        let cg = DVec3::new(row[1], row[2], row[3]);
        worst_position = worst_position.max((cg - parabola(row[0])).length());
        let q = DQuat::from_xyzw(row[5], row[6], row[7], row[4]);
        let rates = DVec3::new(row[8], row[9], row[10]);
        let h = q.mul_vec3(mass.inertia_kg_m2 * rates);
        worst_momentum = worst_momentum.max((h - h0).length() / h0.length());
    }

    // Apogee and ground contact on the parabola, from the ellipsoidal height of the closed form.
    let frame = sim.environment().earth.frame();
    let ground_m = frame.origin().height_m;
    let height = |t: f64| frame.geodetic_from_enu(parabola(t)).unwrap().height_m - ground_m;
    let rate = |t: f64| (height(t + 1e-4) - height(t - 1e-4)) / 2e-4;
    let apogee_guess = t0 + v_cg0.z / G;
    let apogee_s = find_root(
        rate,
        apogee_guess - 1.0,
        apogee_guess + 1.0,
        rate(apogee_guess - 1.0),
        rate(apogee_guess + 1.0),
        1e-12,
    )
    .unwrap();
    let ground_s = find_root(
        height,
        apogee_s,
        apogee_s + 60.0,
        height(apogee_s),
        height(apogee_s + 60.0),
        1e-12,
    )
    .unwrap();
    let apogee = result.event(EventKind::Apogee).unwrap().sample;
    let ground = result.event(EventKind::GroundHit).unwrap().sample;
    // Measured at the default tolerances: 1.7e-6 m over 25 s and 700 m, momentum 7.8e-7 after
    // 60 rad of spin, apogee −4.7e-7 s and ground contact 1.1e-8 s from the closed form.
    assert!(worst_position < 5e-6, "{worst_position}");
    assert!(worst_momentum < 2.5e-6, "{worst_momentum}");
    assert!((apogee.time_s - apogee_s).abs() < 1e-6);
    assert!((ground.time_s - ground_s).abs() < 1e-6);
    assert!((ground.cg_enu_m - parabola(ground_s)).length() < 5e-6);
}

#[test]
fn nose_down_fall_reaches_the_closed_form_terminal_velocity() {
    // A body falling nose down through uniform air with a constant drag coefficient:
    // `v = v_t tanh(g t / v_t)`, `v_t = √(2 m g / (ρ A C_D))`.
    let t0 = 10.0;
    let cd = 0.5;
    let air = UniformAir::sea_level();
    let sim = valetudo(analytic_environment(air, G), capped(t0 + 150.0))
        .with_drag_table(constant_drag(cd));
    let mass = sim.assembly().mass_properties(t0).mass_kg;
    let area = sim.aero().reference_area_m2();
    let terminal = (2.0 * mass * G / (air.0.density_kg_m3 * area * cd)).sqrt();
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 40_000.0),
        velocity_enu_m_s: DVec3::ZERO,
        attitude: DQuat::from_rotation_x(PI),
        body_rate_rad_s: DVec3::ZERO,
    };
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::Velocity], Some(5.0)).unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::TimeCap);
    let times = column(&recorder, "time_s");
    let fall = column(&recorder, "velocity_up_m_s");
    let mut worst: f64 = 0.0;
    for (t, v) in times.iter().zip(&fall) {
        let expected = -terminal * (G * (t - t0) / terminal).tanh();
        worst = worst.max((v - expected).abs() / terminal);
    }
    let last = fall[fall.len() - 1];
    // Measured: 7.6e-9 of v_t over the first minute; after 150 s the speed is v_t to 1e-5.
    assert!(worst < 2.5e-8, "{worst}");
    assert!(
        (-last / terminal - 1.0).abs() < 2e-5,
        "{last} vs {terminal}"
    );
}

#[test]
fn torque_free_precession_matches_eulers_solution() {
    // An axisymmetric body with no torque: the spin `ω_z` is constant and the transverse rate
    // turns in body axes at `Ω = (I_a − I_t) ω_z / I_t`; the angular momentum is fixed in the
    // launch frame.
    let t0 = 10.0;
    let sim = valetudo(
        analytic_environment(UniformAir::vacuum(), 0.0),
        capped(t0 + 3.0),
    );
    let mass = sim.assembly().mass_properties(t0);
    let inertia = mass.inertia_kg_m2;
    let (i_t, i_a) = (inertia.x_axis.x, inertia.z_axis.z);
    assert!((inertia.y_axis.y - i_t).abs() <= 1e-12 * i_t);
    assert!(inertia.x_axis.y.abs() + inertia.x_axis.z.abs() + inertia.y_axis.z.abs() <= 1e-15);
    let omega0 = DVec3::new(0.3, 0.1, 25.0);
    let spin_rate = (i_a - i_t) / i_t * omega0.z;
    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, 10.0),
        attitude: DQuat::IDENTITY,
        body_rate_rad_s: omega0,
    };
    let mut recorder = Recorder::new(
        vec![Channel::Time, Channel::Attitude, Channel::BodyRates],
        Some(0.01),
    )
    .unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    assert_eq!(result.termination, Termination::TimeCap);
    let transverse = omega0.x.hypot(omega0.y);
    let phase0 = omega0.y.atan2(omega0.x);
    let h0 = inertia * omega0;
    let (mut worst_rate, mut worst_momentum): (f64, f64) = (0.0, 0.0);
    for row in recorder.rows() {
        let tau = row[0] - t0;
        let expected = DVec3::new(
            transverse * (spin_rate * tau + phase0).cos(),
            transverse * (spin_rate * tau + phase0).sin(),
            omega0.z,
        );
        let rates = DVec3::new(row[5], row[6], row[7]);
        worst_rate = worst_rate.max((rates - expected).length());
        let q = DQuat::from_xyzw(row[2], row[3], row[4], row[1]);
        let h = q.mul_vec3(inertia * rates);
        worst_momentum = worst_momentum.max((h - h0).length() / h0.length());
    }
    // Measured at the default tolerances: 4.7e-7 rad/s (2e-8 of the spin) and a momentum drift
    // of 1.5e-6 after 75 rad of spin, about the tolerance per radian.
    assert!(worst_rate < 1.5e-6, "{worst_rate}");
    assert!(worst_momentum < 5e-6, "{worst_momentum}");
}

#[test]
fn pitch_oscillation_matches_linear_theory() {
    // At constant speed, with no drag and no gravity, a small angle of attack oscillates. With
    // `ℓᵢ` each component's lever arm aft of the centre of mass and `q̄A` the dynamic pressure on
    // the reference area, `Z = q̄A Σ C_Nαᵢ`, `K₁ = q̄A Σ C_Nαᵢ ℓᵢ` and `K₂ = q̄A Σ C_Nαᵢ ℓᵢ²`.
    // The angle of attack `α` and pitch rate `θ̇` obey
    //   α̇ = −Z/(mV) α + (1 − K₁/(mV²)) θ̇,   I θ̈ = −K₁ α − K₂/V θ̇
    // (the normal force turning the path, the restoring moment, and the damping from the airspeed
    // the rotation adds at each component), whose eigenvalues give the damped frequency and decay.
    let t0 = 10.0;
    let speed = 100.0;
    let air = UniformAir::sea_level();
    let sim = valetudo(analytic_environment(air, 0.0), capped(t0 + 10.0))
        .with_drag_table(constant_drag(0.0))
        .with_event(UserEvent {
            name: "pitch rate crosses zero".to_owned(),
            direction: Direction::Either,
            function: Box::new(|sample| sample.state.body_rate_rad_s.y),
        });
    let mass = sim.assembly().mass_properties(t0);
    let (m, x_cg, inertia) = (mass.mass_kg, -mass.cg_m.z, mass.inertia_kg_m2.y_axis.y);
    let aero = sim.aero();
    let mach = speed / air.0.speed_of_sound_m_s;
    let q_area = 0.5 * air.0.density_kg_m3 * speed * speed * aero.reference_area_m2();
    let (mut z, mut k1, mut k2) = (0.0, 0.0, 0.0);
    for index in 0..aero.component_count() {
        let slope = aero
            .component_normal_force(index, &Flow::new(mach, 1e-7, 0.0))
            .unwrap()
            .slope_per_rad;
        let lever = aero.component_station_m(index).unwrap() - x_cg;
        z += q_area * slope;
        k1 += q_area * slope * lever;
        k2 += q_area * slope * lever * lever;
    }
    let (a11, a12) = (-z / (m * speed), 1.0 - k1 / (m * speed * speed));
    let (a21, a22) = (-k1 / inertia, -k2 / (inertia * speed));
    let trace = a11 + a22;
    let determinant = a11 * a22 - a12 * a21;
    let damped = (determinant - 0.25 * trace * trace).sqrt();
    let period = 2.0 * PI / damped;

    let state = State {
        position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
        velocity_enu_m_s: DVec3::new(0.0, 0.0, speed),
        attitude: DQuat::from_rotation_y(0.005),
        body_rate_rad_s: DVec3::ZERO,
    };
    let mut recorder = Recorder::new(vec![Channel::Time, Channel::BodyRates], Some(0.001)).unwrap();
    let result = sim.run_free(t0, state, &mut recorder).unwrap();
    let crossings: Vec<f64> = result
        .events
        .iter()
        .filter(|event| event.kind == EventKind::User(0))
        .map(|event| event.sample.time_s)
        .collect();
    assert!(crossings.len() >= 6, "{crossings:?}");
    let measured =
        2.0 * (crossings[crossings.len() - 1] - crossings[0]) / (crossings.len() - 1) as f64;

    // Decay: the largest |θ̇| in each half period falls by exp(trace/2 · period/2) per half.
    let times = column(&recorder, "time_s");
    let rates = column(&recorder, "body_rate_y_rad_s");
    let peaks: Vec<f64> = crossings
        .windows(2)
        .map(|w| {
            times
                .iter()
                .zip(&rates)
                .filter(|(t, _)| **t > w[0] && **t < w[1])
                .fold(0.0_f64, |peak, (_, r)| peak.max(r.abs()))
        })
        .collect();
    let decay = (peaks[peaks.len() - 1] / peaks[0]).ln() / (peaks.len() - 1) as f64;
    let expected_decay = 0.25 * trace * period;
    // Measured: the period 1.44965 s against 1.44954 s (8e-5), the decay 0.2% from the theory's.
    assert!((measured / period - 1.0).abs() < 3e-4);
    assert!((decay / expected_decay - 1.0).abs() < 0.01);
}

#[test]
fn stable_rocket_weathercocks_into_crosswind() {
    // Loft lesson L20: a 3-DOF boost drifted downwind. A stable 6-DOF rocket turns into the wind
    // off the rail and flies upwind. Wind 5 m/s from the west (toward +x, east).
    let fly = |environment| {
        valetudo(environment, FlightSettings::default())
            .run(&mut ())
            .unwrap()
    };
    let calm = fly(Environment::standard(site()).unwrap());
    let windy = fly(windy_environment(ConstantWind::new(5.0, 1.5 * PI).unwrap()));
    let at = |result: &crate::flight::FlightResult, kind| result.event(kind).unwrap().sample;
    let calm_apogee = at(&calm, EventKind::Apogee);
    let windy_apogee = at(&windy, EventKind::Apogee);
    let burnout = at(&windy, EventKind::Burnout);
    let axis_east = burnout.state.unit_attitude().mul_vec3(DVec3::Z).x;
    // Measured: calm −1.0 m (Earth rotation), windy −96 m, axis 0.12 west of vertical at burnout.
    assert!(calm_apogee.cg_enu_m.x.abs() < 5.0);
    assert!(axis_east < -0.05, "{axis_east}");
    assert!(windy_apogee.cg_enu_m.x - calm_apogee.cg_enu_m.x < -50.0);
}

#[test]
fn repeated_runs_are_bit_identical() {
    // Loft lesson L24: `simulate()` mutated its inputs, so repeated runs differed. The same
    // simulation run twice, and a second one built the same way, give the same bits.
    let build = || {
        valetudo(
            windy_environment(ConstantWind::new(4.0, 1.0).unwrap()),
            capped(40.0),
        )
    };
    let record = |sim: &Simulation| {
        let mut recorder = Recorder::new(Channel::ALL.to_vec(), None).unwrap();
        let result = sim.run(&mut recorder).unwrap();
        let bits: Vec<Vec<u64>> = recorder
            .rows()
            .iter()
            .map(|row| row.iter().map(|v| v.to_bits()).collect())
            .collect();
        (bits, result)
    };
    let sim = build();
    let first = record(&sim);
    let second = record(&sim);
    let third = record(&build());
    assert!(first.0.len() > 100);
    assert_eq!(first.0, second.0);
    assert_eq!(first.0, third.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.1, third.1);
}

#[test]
fn termination_reason_distinguishes_no_liftoff_time_cap_and_step_limit() {
    // Loft lesson L25: every stop before the time cap was called "step budget", even a rocket that
    // never lifted off.
    let rocket = design("rocketpy-valetudo");
    let fly = |rail: Rail, settings: FlightSettings| {
        Simulation::new(
            &rocket,
            "example",
            Environment::standard(site()).unwrap(),
            rail,
            settings,
        )
        .unwrap()
        .run(&mut ())
        .unwrap()
    };
    let rail = Rail::vertical(3.0);
    // Friction needs a normal force: on a rail 30° from vertical, the weight's share across the rail.
    let stuck = Rail {
        elevation_rad: PI / 3.0,
        friction_coefficient: 20.0,
        ..rail
    };
    let normal = fly(rail, FlightSettings::default());
    assert_eq!(normal.termination, Termination::GroundHit);
    let no_liftoff = fly(stuck, FlightSettings::default());
    assert_eq!(no_liftoff.termination, Termination::NoLiftoff);
    assert!(no_liftoff.event(EventKind::Liftoff).is_none());
    assert_eq!(no_liftoff.final_sample.state.velocity_enu_m_s, DVec3::ZERO);
    let capped_flight = fly(rail, capped(5.0));
    assert_eq!(capped_flight.termination, Termination::TimeCap);
    assert_eq!(capped_flight.final_sample.time_s, 5.0);
    let limited = fly(
        rail,
        FlightSettings {
            step_limit: 40,
            ..FlightSettings::default()
        },
    );
    assert_eq!(limited.termination, Termination::StepLimit);
    assert!(limited.final_sample.time_s < normal.final_sample.time_s);
}

#[test]
fn flight_events_come_in_order_and_the_recorder_keeps_its_interval() {
    let sim = valetudo(
        Environment::standard(site()).unwrap(),
        FlightSettings::default(),
    );
    let mut recorder =
        Recorder::new(vec![Channel::Time, Channel::HeightAboveGround], Some(0.25)).unwrap();
    let result = sim.run(&mut recorder).unwrap();
    let kinds: Vec<EventKind> = result.events.iter().map(|event| event.kind).collect();
    assert_eq!(
        kinds,
        [
            EventKind::Liftoff,
            EventKind::RailExit,
            EventKind::Burnout,
            EventKind::Apogee,
            EventKind::GroundHit
        ]
    );
    let times = column(&recorder, "time_s");
    assert!(times.windows(2).all(|w| w[1] >= w[0]));
    let event_times: Vec<f64> = result.events.iter().map(|e| e.sample.time_s).collect();
    for t in &times {
        let on_grid = ((t / 0.25).round() * 0.25 - t).abs() < 1e-12;
        assert!(on_grid || event_times.contains(t), "{t}");
    }
    let apogee = result.event(EventKind::Apogee).unwrap().sample;
    let heights = column(&recorder, "height_above_ground_m");
    assert!(
        heights
            .iter()
            .all(|h| *h <= apogee.height_above_ground_m + 1e-9)
    );
    assert!(apogee.vertical_speed_m_s.abs() < 1e-6);
    let ground = result.event(EventKind::GroundHit).unwrap().sample;
    assert!(ground.height_above_ground_m.abs() < 1e-6);
}

#[test]
fn a_supersonic_flight_is_refused_until_m1_8() {
    // The subsonic aerodynamics refuse Mach 1 (ADR-008); a flight that gets there stops with that
    // error rather than flying on with wrong forces.
    let sim = Simulation::new(
        &design("synthetic-54mm-three-fin"),
        "i175",
        Environment::standard(site()).unwrap(),
        Rail::vertical(2.0),
        FlightSettings::default(),
    )
    .unwrap();
    let error = sim.run(&mut ()).unwrap_err();
    assert!(
        matches!(error, SimError::Aero(AeroError::Mach { mach }) if mach >= 1.0),
        "{error:?}"
    );
}
