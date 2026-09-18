# Lessons from Loft

Loft (`nrdptel/fusionspace-loft`, MIT, Neer's own) was the TypeScript predecessor. This file records
what it modelled, where it was wrong or weak, the format quirks it found, the tests worth porting,
and the process mistakes to avoid. It covers Loft at `64f51ef1b3`; evidence paths are relative to
`refs/fusionspace-loft`. Private-corpus facts appear only as counts and error statistics.

**Rules.** Each `L` row names its milestones (the first owns the tests) and the tests to write.
Each named milestone's `ROADMAP.md` entry lists the id; a checked-off milestone's tests must exist
as live tests (`xtask/src/docs.rs` checks both). Renames that keep the assertion are fine (note them
in `STATUS.md`); moving a lesson later or dropping one needs an ADR. `P` rows name a process guard.
Loft's claims are leads: every test re-derives its numbers from a primary source or an oracle.

## What Loft was

- **Dynamics and environment:** 3-DOF point mass, thrust along the velocity, RK4 with no error
  control (0.01 s up, 0.2 s down), a 1200 s cap, a 4-layer atmosphere, constant gravity.
- **Aero:** Barrowman CNα/CP (used only for the printed margin) and a drag buildup after the
  OpenRocket technical documentation, with an invented wave-drag curve. Claimed envelope M ≤ 0.8.
- **Motors, mass, recovery, Monte Carlo:** 108 `.eng` curves with impulse-fraction mass, pitch
  inertia only, CdA that opens instantly, staging planned before flight, and one seeded stream.
- **Validation:** stored results in design files (35 files, 97 runs, median apogee disagreement 2.9%
  over a mixed population: neither Loft's error nor a reference), a RocketPy guard fed Loft's drag.
- **Worth keeping:** withholding a number with a reason (as `Option` and validity flags, not zeros).

## Clean-room hazards inside Loft: never port these; re-derive from format docs or jar runs

- **From OpenRocket's GPL Java, by Loft's own comments:** CG and override precedence
  (`lib/sim/mass.ts:250-350`), parachute Cd 0.8 (`lib/sim/recovery-defaults.ts:38-60`), the ring
  auto bore (`lib/ork/adapt.ts:1350-1393`), the 24.12 ground-hit frame (`lib/ork/adapt.ts:84-108`).
- **Blanket rule:** any Loft text that names a `.java` file, quotes Java or OpenRocket's message
  strings, or says it was read from OpenRocket's source is off limits. Known places: Loft's
  `ROADMAP.md` (1355, 2080-2096, 2258-2266, 2486-2496, 2605-2640, 3545-3560),
  `BACKLOG.md:1428,2841`, `COMPETITION.md` rows 35, 36, 41, 47, `lib/ork/adapt.test.ts:968-980`,
  `lib/corpus/sweep.test.ts:160-162`, the RockSim shape codes (`lib/rkt/adapt.ts:68-106`), the
  methods page's auto-radius text.
- **Example files:** OpenRocket's example `.ork` files are GPL data: inputs only, never committed.

## Numbers not to reuse as references

- **`fixtures/rocketpy-cross-check.json`:** records RocketPy 1.12.1 but no install pin, date or
  inputs hash; fed Loft's own Cd, mass and inertia, under ISA.
- **Demo "stored" figures, demo `.rkt` results, the OpenRocket example summary:** author estimates
  (one set inconsistent) or unasserted, with no Loft commit or date.
- **Stored results as found** (L87): 8 of 79 OpenRocket runs outdated and 7 not simulated; two
  RockSim test files that don't match their geometry; a `.CDX1` storing apogees under 4 m.
- **Certified impulses in `db.test.ts`:** re-read them from the ThrustCurve snapshot.

## Physics and numerics

| id | lesson | Loft evidence | milestone | test to write |
|---|---|---|---|---|
| L1 | Constant gravity 9.80665 on a flat Earth; RocketPy at latitude 0 used 9.78033, so Loft read low against it | lib/units.ts:52; scripts/rocketpy/fly.py:30 | M1.1 | `hpr_core::gravity::tests::somigliana_matches_published_values` |
| L2 | Geometric altitude treated as geopotential: 11 km gave 216.65 K and 22,632 Pa, where USSA76 gives 216.774 K and 22,699.96 Pa (the table truncates to 2.2699E+2 mb, so compare within one count) | lib/sim/atmosphere.ts:37,94 | M1.2 | `hpr_atmos::ussa76::tests::geometric_11_km_matches_the_1976_tables` |
| L3 | Only 4 layers, and the 32 km lapse runs on forever: 335 K at 70 km against 219.6 K | lib/sim/atmosphere.ts:37-42 | M1.2 | `hpr_atmos::ussa76::tests::fifty_km_is_270_65_k_and_79_779_pa` |
| L4 | Sutherland constants aren't USSA76's (sea-level viscosity +1.3%) | lib/sim/atmosphere.ts:31-34 | M1.2 | `hpr_atmos::ussa76::tests::sea_level_viscosity_is_1_7894e_5` |
| L5 | "Today's conditions" keep the standard lapse from the field up; dry air; no sounding temperatures | lib/sim/atmosphere.ts:147-158 | M1.2 | `hpr_atmos::profile::tests::sounding_temperature_overrides_standard_lapse` |
| L6 | Design-file runs use one wind vector; forecast profiles step at the lowest level instead of blending from the surface; no gusts | lib/sim/simulate.ts:588-596; lib/weather.ts:242-261 | M1.2 | `hpr_atmos::wind::tests::layered_wind_interpolates_speed_and_heading` |
| L7 | Fin CNα has no compressibility factor, so CNα and CP never change with Mach | lib/sim/aero.ts:176 | M1.8 | `hpr_aero::fins::tests::fin_cna_compressibility_reduces_to_barrowman_at_m0` |
| L8 | Fin CNα grows linearly with fin count; no correction for 5 to 8 fins | lib/sim/aero.ts:177 | M1.5a | `hpr_aero::fins::tests::six_fin_cna_applies_fin_count_factor` |
| L9 | The conical-transition CP formula is used for every transition shape | lib/sim/aero.ts:80-86 | M1.5a | `hpr_aero::body::tests::ogive_transition_cp_uses_volume_form` |
| L10 | Elliptical-fin CNα comes from an equal-area trapezoid with the wrong mid-chord sweep | lib/sim/aero.ts:165-175 | M1.5a | `hpr_aero::fins::tests::elliptical_fin_cna_uses_zero_midchord_sweep` |
| L11 | Fin sets are merged into one for drag, so the result depends on set order (a fix was reverted 3 times) | lib/sim/aero.ts:467-495 | M1.5b | `hpr_aero::drag::tests::drag_invariant_to_fin_set_order` |
| L12 | Uncited constants: body form factor 1+60/f³+0.0025f (1.95 at fineness 4, where Niskanen's 1+1/(2f) gives 1.13), friction compressibility 0.144/0.65, roughness values 60 µm and 2 µm that aren't in Niskanen Table 3.2 | lib/sim/aero.ts:320,600,621 | M1.5b | `hpr_aero::drag::tests::form_factor_and_roughness_match_cited_values` |
| L13 | No power-on base drag relief; Loft's comment says Niskanen has none, but its §3.4.5 subtracts the thrusting motors' area | lib/sim/aero.ts:646-656 | M1.5b | `hpr_aero::drag::tests::power_on_base_drag_subtracts_thrusting_motor_area` |
| L14 | Lug drag uncited (C 0.5 on a solid disc plus a flat 0.01); rail buttons drag as lugs | lib/sim/aero.ts:723-731,789 | M1.5b | `hpr_aero::drag::tests::launch_lug_drag_matches_cited_hollow_tube_formula` |
| L15 | Bare diameter steps get no drag; shoulder drag jumps from 0.8ΔA to 0 as transition length reaches 0 | app/docs/limitations/page.tsx:472-526 | M1.5b | `hpr_aero::drag::tests::shoulder_drag_continuous_as_transition_length_tends_to_zero` |
| L16 | A silent Cd cap of 10 hides malformed geometry | lib/sim/aero.ts:768,781 | M1.5b | `hpr_aero::drag::tests::malformed_geometry_is_an_error_not_a_clamped_cd` |
| L17 | Fin leading-edge drag frozen at its M1 value; nose and shoulder pressure drag have no Mach term | lib/sim/aero.ts:408-419,690-693 | M1.8 | `hpr_aero::drag::tests::leading_edge_and_cone_pressure_drag_have_supersonic_branches` |
| L18 | Wave drag is an invented curve (M_crit 0.8, peak M1.15, 0.12 floor) never measured against RASAero | lib/sim/aero.ts:783-809 | M1.8 | `hpr_aero::drag::tests::supersonic_cd_within_tolerance_of_rasaero_tables` |
| L19 | Tube-fin CP about 0.9 cal forward of OpenRocket's; ring tails skipped | app/docs/limitations/page.tsx:651-667 | M2.2 | `hpr_validate::openrocket::tests::tube_fin_cp_within_0_25_cal_of_oracle` |
| L20 | 3-DOF: no angle of attack, body lift, damping or roll, and no weathercocking (the boost drifts downwind) | lib/sim/simulate.ts:6-12,660-667 | M1.6b | `hpr_sim::tests::stable_rocket_weathercocks_into_crosswind` |
| L21 | RK4 with no error control (steps capped only for canopy stiffness) and no apogee convergence test | lib/sim/simulate.ts:436-468 | M1.6a | `hpr_sim::integrator::tests::apogee_converges_under_tolerance_halving` |
| L22 | Events not root-found: apogee quantised to the step, altitude deploys overshoot by v·dt, landing taken below ground (truth: the L23 vacuum case) | lib/sim/simulate.ts:874-958 | M1.6a | `hpr_sim::events::tests::apogee_landing_and_altitude_deploy_located_within_1e_6_s` |
| L23 | Discontinuities fall inside steps; the vacuum closed form was only checked to ±2% | lib/sim/simulate.test.ts:79-104 | M1.6a | `hpr_sim::integrator::tests::constant_thrust_vacuum_matches_closed_form` |
| L24 | `simulate()` mutates its input recovery devices, so repeated runs (and a convergence test) were contaminated | lib/sim/simulate.ts:902-905 | M1.6b | `hpr_sim::tests::repeated_runs_are_bit_identical` |
| L25 | Any stop before the time cap was labelled "step budget", including a rocket that never lifted off | lib/sim/simulate.ts:772,982 | M1.6b | `hpr_sim::tests::termination_reason_distinguishes_no_liftoff_time_cap_and_step_limit` |
| L26 | Rail has no friction or button geometry; "last button clears" was blamed for a gap the oracle couldn't show | lib/sim/simulate.ts:692; scripts/rocketpy/fly.py:48-91 | M1.6b | `hpr_sim::rail::tests::rail_exit_is_when_the_last_button_leaves` |
| L27 | Instant inflation, no opening load, an unsourced 0.5·A_ref body term, and no drogue-release option | lib/sim/recovery.ts:41; lib/sim/simulate.ts:680 | M1.7a | `hpr_sim::recovery::tests::inflation_time_limits_peak_opening_load` |
| L28 | Stiff canopy drag under explicit RK4 forced a 2e-4 s step floor; the 1200 s cap left high descents unlanded | lib/sim/simulate.ts:436,452-468 | M1.7a | `hpr_sim::recovery::tests::oversized_canopy_and_ten_km_descent_land_without_step_collapse` |
| L29 | Parachute Cd 0.8 came from OpenRocket source; Knacke Table 5-1 gives 0.75 to 0.80 on nominal area for flat circular canopies (RocketPy's 1.4 is hemispherical) | lib/sim/recovery-defaults.ts:38-60 | M1.7a | `hpr_sim::recovery::tests::default_canopy_cd_carries_its_citation` |
| L30 | Staging fixed before flight: apogee or altitude separation fell back to burnout, and boosters were never flown | lib/sim/setup.ts:288-300; lib/sim/simulate.ts:1039-1063 | M1.9 | `hpr_sim::staging::tests::apogee_separation_fires_in_flight_and_booster_flies_to_landing` |
| L31 | Clusters are on-axis only (no motor-out moment); mixed clusters were sent to the oracle as N copies of motor 0 | lib/sim/setup.ts:358; lib/validation/rocketpy-spec.ts:198-230 | M1.9 | `hpr_sim::staging::tests::cluster_motor_out_produces_pitch_moment` |
| L32 | Flutter uses 1.337·(λ+1)/2, but NACA TN 4197 eq. 18 gives (39.3/p₀)·(λ+1)/2 with p₀ 14.696 psi = 2.674·(λ+1)/2, so Loft's flutter speed is √2 too high (the unsafe side); 7 shear moduli unsourced | lib/sim/flutter.ts:11,287 | M1.10 | `hpr_sim::flutter::tests::flutter_denominator_matches_tn_4197_eq_18`, `hpr_sim::flutter::tests::scaling_laws_in_thickness_shear_modulus_and_pressure` |
| L33 | Static margin blew up as CNα went to 0 and was published as ±12 to 15 cal | COMPETITION.md:129 | M1.10 | `hpr_sim::metrics::tests::static_margin_undefined_when_cn_alpha_near_zero` |
| L34 | Opening shock inflated max acceleration, and a finite difference missed thrust-spike peaks | lib/sim/simulate.test.ts:232-288 | M1.10 | `hpr_sim::metrics::tests::peak_acceleration_is_analytic_and_excludes_opening_shock` |
| L35 | Zeros stood in for "never happened"; apogee datum and ground-hit frame went unstated | lib/sim/withheld.ts:23-139; BACKLOG.md:1010-1016 | M1.10 | `hpr_sim::metrics::tests::unlanded_flight_has_no_ground_hit_speed_and_outputs_name_datum` |

## Motors, mass and design checks

| id | lesson | Loft evidence | milestone | test to write |
|---|---|---|---|---|
| L36 | `.eng` reads only the first header; a second block's samples are appended to the first curve | lib/motors/eng.ts:69-83 | M1.3 | `hpr_motor::eng::tests::multiple_blocks_parse_separately` |
| L37 | Delays split on "-" only: `P` and comma lists are lost, and 100 and 1000 (presumably plugged markers; the RASP spec defines only P and 0) become 100 s and 1000 s | lib/motors/eng.ts:127-132 | M1.3 | `hpr_motor::eng::tests::delay_lists_and_plugged_markers_parse` |
| L38 | Class letter off by one at band tops (2.5 N·s gives B); no 1/8A | lib/motors/eng.ts:53-59 | M1.3 | `hpr_motor::tests::impulse_class_upper_bound_inclusive` |
| L39 | Time samples not checked for order; burn time is the last sample, while ThrustCurve's glossary uses NFPA 1125 (5% of peak) | lib/motors/eng.ts:88-149 | M1.3 | `hpr_motor::eng::tests::rejects_non_monotonic_time`, `hpr_motor::tests::burn_time_uses_the_nfpa_1125_definition` |
| L40 | Motor CG fixed at the casing midpoint with zero own inertia; the impulse-fraction model is uncited | lib/sim/setup.ts:246; lib/motors/eng.ts:175-179 | M1.3 | `hpr_motor::tests::cg_and_inertia_move_from_loaded_to_burnout` |
| L41 | Bundled curve licences: 45 PD, 38 none, 22 unknown, 3 "free" (which can include GPL) | lib/motors/curves/provenance.json | M1.3 | `hpr_motor::catalog::tests::bundled_curves_have_permissive_license` |
| L42 | Impulse checks were loose (-8% to +8%); a mis-sourced curve flew about 26% high until caught | lib/motors/db.test.ts:35-59,351-367 | M1.3 | `hpr_motor::catalog::tests::every_bundled_curve_impulse_within_1pct_of_thrustcurve` |
| L43 | ThrustCurve metadata has to override the `.eng` header envelope (one header said 75 mm for a 54 mm motor) | lib/motors/db.ts:48-136 | M1.3 | `hpr_motor::catalog::tests::metadata_overrides_header_envelope` |
| L44 | Inertia: pitch only, rod formula without the radial term, fins ignore span, rings and masses get 0 | lib/sim/mass.ts:38-172 | M1.4a | `hpr_design::mass::tests::thin_tube_inertia_includes_radial_term` |
| L45 | Hollow transition CG uses the solid centroid; freeform fin CG hard-coded at 0.42 c_r | lib/sim/mass.ts:107,172 | M1.4a | `hpr_design::mass::tests::hollow_transition_and_freeform_fin_cg_are_exact_centroids` |
| L46 | Fin tabs never read (100 to 120 g lost on 2 designs); rail buttons came in at 0 kg | BACKLOG.md:5027; lib/ork/adapt.ts:852-869 | M1.4a | `hpr_design::mass::tests::fin_tab_and_rail_button_mass_counted` |
| L47 | Reference diameter is the widest component, which can be internal; OpenRocket also offers nose or custom | app/docs/methods/page.tsx:296-315 | M1.4b | `hpr_design::tests::reference_diameter_ignores_internal_components` |
| L48 | Tangent ogive only (no secant), a silent power-series default, Haack names swapped in a comment | lib/sim/shapes.ts:31-48 | M1.4a | `hpr_design::shapes::tests::secant_ogive_and_haack_parameters_change_profile` |
| L49 | Transitions use a "clipped nose" profile with a kink, never checked against `.ork` meaning | lib/sim/shapes.ts:61-91 | M1.4a, M3.1 | `hpr_design::shapes::tests::ogive_transition_hits_both_radii_and_is_monotone` |
| L50 | A motor wider than its mount gave +69% apogee (the flattering side); fins could sit off the airframe | ROADMAP.md:8413; COMPETITION.md:131 | M1.4b | `hpr_design::checks::tests::motor_wider_than_mount_is_rejected`, `hpr_design::checks::tests::fin_root_must_touch_body` |
| L51 | CG override on a part with a shoulder unsettled (up to 133 mm); Loft's precedence came from OpenRocket source | BACKLOG.md:1587; lib/sim/mass.ts:250-283 | M2.2 | `hpr_validate::openrocket::tests::override_precedence_matches_oracle` |
| L52 | Monte Carlo scaled thrust but not propellant mass, silently changing specific impulse | lib/sim/montecarlo.ts:277 | M6.1 | `hpr_analysis::montecarlo::tests::impulse_dispersion_preserves_specific_impulse` |
| L53 | One random stream: adding a draw reshuffles every later sample, so parallel runs aren't reproducible | lib/sim/montecarlo.ts:262-273 | M6.1 | `hpr_analysis::montecarlo::tests::sample_k_independent_of_n_and_thread_count` |
| L54 | Failed or impossible samples dropped silently; probabilities computed over survivors | lib/sim/montecarlo.ts:221-233,312-327 | M6.1 | `hpr_analysis::montecarlo::tests::failed_samples_are_counted_and_reported` |
| L55 | Wind and rail bearings uniformly random, discarding the forecast heading; burn time, delay and CG never dispersed | lib/sim/montecarlo.ts:270-296 | M6.1 | `hpr_analysis::montecarlo::tests::wind_heading_dispersed_about_nominal` |

## File formats

| id | lesson | Loft evidence | milestone | test to write |
|---|---|---|---|---|
| L56 | Containers sniffed by magic bytes (zip, gzip, raw XML); malformed input must error, never crash | lib/ork/zip.ts:85-99; lib/ork/import.test.ts:14-55 | M3.1 | `hpr_io::ork::tests::malformed_inputs_error_not_panic` |
| L57 | Embedded `thrustcurves/*.rse` entries in `.ork` archives were discarded | BACKLOG.md:5018 | M3.1 | `hpr_io::ork::tests::embedded_rse_curves_are_read` |
| L58 | `auto 0.025` kept the number but lost the auto flag, so saving made automatic dimensions hand-typed | BACKLOG.md:17; lib/ork/xml.ts:136-146 | M3.1 | `hpr_io::ork::tests::auto_flag_kept_with_cached_value` |
| L59 | Auto radii don't resolve across a stage boundary (OpenRocket's own files show they should) | lib/ork/adapt.ts:1232; BACKLOG.md:26 | M3.1 | `hpr_io::ork::tests::auto_fore_radius_resolves_across_stage_boundary` |
| L60 | Auto resolution depended on sibling order, and a bulkhead inside a coupler stayed NaN | lib/ork/adapt.ts:1398-1432 | M3.1 | `hpr_io::ork::tests::auto_resolution_is_order_independent_and_finite` |
| L61 | A ring with an auto bore weighed 0 g (4 rings on one design); a stated wall was dropped when the outer radius was auto | lib/ork/adapt.ts:1188-1197,1350-1393 | M3.1 | `hpr_io::ork::tests::auto_ring_bore_and_stated_wall_match_oracle` |
| L62 | Legacy and modern tags coexist (`position`/`axialoffset`, `fincount`/`instancecount`...); a stated 0 read as missing | lib/ork/adapt.ts:258-273 | M3.1 | `hpr_io::ork::tests::legacy_tags_equal_modern_and_zero_is_stated` |
| L63 | `overridesubcomponentscg` and `overridecd` never read, so a part set to Cd 0 was still charged drag | BACKLOG.md:1482,5023 | M3.1 | `hpr_io::ork::tests::cd_and_cg_subcomponent_overrides_are_independent` |
| L64 | Wind direction read from `launchroddirection`, and dropped from stored conditions | lib/ork/adapt.ts:1070; lib/sim/run.ts:66-80 | M3.1 | `hpr_io::ork::tests::wind_direction_is_not_rod_direction` |
| L65 | Configurations split across rocket and mounts, some undeclared; a motor on a dangling mount id fired from stage 0 | lib/ork/adapt.ts:440-490; BACKLOG.md:4127 | M3.1 | `hpr_io::ork::tests::per_config_overrides_and_dangling_mount_warn` |
| L66 | Pods, parallel stages and booster sets dropped; export then lost the "reduced" flag | lib/ork/adapt.ts:1468-1489; BACKLOG.md:4751 | M3.1 | `hpr_io::ork::tests::pods_kept_in_extensions_or_flagged_reduced` |
| L67 | Export wrote plugged delays as 0 s (42 motors), mount-level ignition only, no conditions, and computed masses as overrides | lib/ork/export.ts:218-241,279-285; BACKLOG.md:3008 | M3.2 | `hpr_io::ork::export::tests::round_trip_keeps_delays_ignition_conditions_and_override_flags` |
| L68 | Freeform fins exported as an equal-area trapezoid (42% too big); cluster scale and rotation lost; 6-decimal rounding | lib/ork/export.ts:91-94,372-386; BACKLOG.md:1765 | M3.2 | `hpr_io::ork::export::tests::fin_points_clusters_and_floats_round_trip_exactly` |
| L69 | RockSim `LocationMode` 1 (absolute) and 2 (from the parent's rear) were misread | lib/rkt/adapt.ts:155-176 | M3.4 | `hpr_io::rkt::tests::location_modes_0_1_2` |
| L70 | RockSim mixes units (mm, g, radians, °C, a barometer inferred as mmHg); `Stage3Parts` is the top; negative delay means plugged; `UseKnownCG` marks cached values | lib/rkt/adapt.ts:11-15,195-290,609-662 | M3.4 | `hpr_io::rkt::tests::units_stages_delays_and_cached_cg` |
| L71 | RockSim stores no canopy Cd or deploy event; deploying at apogee flew one design about 545% high | lib/rkt/adapt.ts:512-541 | M3.4 | `hpr_io::rkt::tests::recovery_deploys_at_ejection_charge` |
| L72 | RASAero `<Location>` never read, so parts were stacked end to end (+11.7% and +14.3% length) | BACKLOG.md:278,488 | M3.5 | `hpr_io::cdx1::tests::parts_placed_at_absolute_location` |
| L73 | RASAero fin `<Location>` datum unsettled (6 of 8 equal the chord); needs a primary source | lib/rasaero/adapt.ts:105-106; BACKLOG.md:407 | M3.5 | `hpr_io::cdx1::tests::fin_location_datum` |
| L74 | RASAero boattail described twice, LV-Haack given the Von Kármán parameter, protuberance drag dropped, mass per simulation | lib/rasaero/adapt.ts:68-72,154-188,581-618 | M3.5 | `hpr_io::cdx1::tests::boattail_once_lv_haack_and_launch_weight_per_simulation` |

## Validation

| id | lesson | Loft evidence | milestone | test to write |
|---|---|---|---|---|
| L75 | The RocketPy check wasn't like-for-like: ISA against stored conditions, unstated latitude and gravity, Loft's own drag and mass fed to the oracle | scripts/rocketpy/fly.py:30-46; lib/validation/rocketpy-spec.ts:127-259 | M2.1b, M2.1b2 | `hpr_validate::rocketpy::tests::oracle_inputs_come_from_the_case_file_not_hpr_outputs` |
| L76 | "If the drift guard fails, regenerate the reference", and the reference moved with Loft's drag | scripts/rocketpy/README.md:23-29 | M2.1a | `hpr_validate::tests::references_unchanged_when_hpr_drag_is_perturbed` |
| L77 | Hand-written "stored" results in demo designs, one set internally inconsistent | app/docs/validation/page.tsx:152-170 | M2.1a | `hpr_validate::tests::every_reference_value_has_provenance` |
| L78 | Suites skipped themselves without fixtures and reported green; a filter ignored 2 of 5 tool families | MAINTAINING.md:45-55; BACKLOG.md:1075-1082 | M2.1a | `hpr_validate::tests::fewer_cases_run_than_the_lock_expects_fails` |
| L79 | Only 2 of 12 metrics had per-case gates; a +204% deployment velocity passed as "ungated" | BACKLOG.md:1787-1791 | M2.1a | `hpr_validate::tests::every_census_metric_has_a_per_case_tolerance` |
| L80 | Same word, different quantity: optimum delay, deployment and ground-hit velocity differ by tool and OpenRocket version (Loft took the 24.12 boundary from source; the older side needs a second pinned jar) | lib/validation/compare.ts:105-124; lib/ork/adapt.ts:1009-1054 | M2.2 | `hpr_validate::tests::stored_metric_definitions_are_per_tool_and_version` |
| L81 | Metrics for events that never happened were scored as 0 | lib/validation/compare.ts:47-61 | M2.2 | `hpr_validate::tests::metric_for_missing_event_is_withheld_not_scored` |
| L82 | References 60% apart were excused as "no single target", and known issues excused the two largest misses | lib/corpus/sweep.test.ts:86-110; BACKLOG.md:1792-1799 | M2.2 | `hpr_validate::tests::excused_cases_stay_in_the_census_statistics_against_both_references` |
| L83 | No real-flight validation ever happened | app/docs/validation/page.tsx:624-636 | M2.3 | `hpr_validate::tests::real_flight_cases_report_apogee_and_trace_rms` |
| L84 | Hand-written counts for unnamed populations (27 `.ork` vs 35 files; 8 of 79 vs 11 of 91 runs); one disagreement counted 15 times | lib/validation/stored-status.ts:15-17; ROADMAP.md:2389-2391 | M2.4 | `hpr_validate::census::tests::counts_are_generated_and_each_case_counts_once` |
| L85 | The known-issue "now passes" nudge used half the tolerance, so it was blind in the 6 to 12% band | lib/corpus/sweep.test.ts:2898-2913 | M2.4 | `hpr_validate::census::tests::known_gap_that_starts_passing_fails_the_gate` |
| L86 | A headline accuracy figure was published without its oracle kind, population or Mach regime | COMPETITION.md:74,132 | M2.4 | `hpr_validate::census::tests::headline_names_oracle_kind_population_and_regime` |
| L87 | Stored results were scored as references regardless of status (labelled but still counted): outdated and not-simulated runs, files that don't match their geometry, impossible apogees | app/docs/validation/page.tsx:119-126; lib/corpus/sweep.test.ts:96-104; BACKLOG.md:1083-1085 | M2.2 | `hpr_validate::openrocket::tests::stale_and_implausible_stored_results_are_excluded_from_gates_and_census` |
| L88 | The only independent check held drag equal; Loft's own aero was never gated against an oracle | lib/validation/rocketpy-spec.ts:127-137 | M2.4 | `hpr_validate::census::tests::predicted_mode_regressions_fail_the_gate` |

## Tests worth porting (closed forms; Loft's tolerances were loose)

| id | lesson | Loft evidence | milestone | test to write |
|---|---|---|---|---|
| L89 | Barrowman hand values: cone CNα 2 and CP 2L/3; conical transition from radius 20 to 40 mm over 0.1 m (reference radius 40 mm) gives CNα 1.5 and CP 0.05556 m from its fore end; elliptical fin CP 0.28779 c_r from the root leading edge | lib/sim/aero.test.ts:69-263 | M1.5a | `hpr_aero::tests::barrowman_hand_values` |
| L90 | Drag invariants: skin friction follows Niskanen eq. 3.81 piecewise (it is discontinuous at R_crit) and stays finite to M5; split fin sets drag like one set; base drag continuous at M1 | lib/sim/aero.test.ts:297-744 | M1.5b | `hpr_aero::drag::tests::skin_friction_follows_eq_3_81_and_drag_invariants_hold` |
| L91 | Nose volumes: cone πR²L/3; tangent ogive R 0.04 m, L 0.25 m gives 6.7509e-4 m³; Haack πR²L(1/2 + 3C/16) | lib/sim/shapes.test.ts:1-58 | M1.4a | `hpr_design::shapes::tests::nose_volumes_match_closed_forms` |
| L92 | Terminal descent: 1.1 kg, Cd 0.8, 1 m canopy, ρ 1.225, g 9.80665, canopy drag only, gives 5.294 m/s (Loft allowed ±30%) | lib/sim/simulate.test.ts:166-201 | M1.7a | `hpr_sim::recovery::tests::descent_rate_equals_terminal_velocity` |
| L93 | Staging: sustainer lights at booster burnout plus delay, mass steps at separation, an unreachable trigger never lights | lib/sim/staging.test.ts:267-311,687-800 | M1.9 | `hpr_sim::staging::tests::serial_plan_timing_and_mass_step` |
| L94 | Optimum delay is the same whether the flown delay is early or late | lib/sim/flight.test.ts:1044-1063 | M1.10 | `hpr_sim::metrics::tests::optimum_delay_independent_of_flown_delay` |
| L95 | Degenerate designs (zero radius, NaN tokens, zero fins, negative mass) must error or stay finite | lib/sim/robustness.test.ts:41-62 | M4.1 | `hpr::tests::degenerate_designs_error_or_stay_finite` |
| L96 | Monte Carlo: same seed gives identical samples; zero dispersion gives sd 0 and the nominal flight | lib/sim/montecarlo.test.ts:37-310 | M6.1 | `hpr_analysis::montecarlo::tests::zero_dispersion_reproduces_nominal_flight` |
| L97 | Ballast trim: CP 0.80 m, CG 0.55 m, 2 kg, d 0.1 m, nose 0.15 m gives 0.285714 kg for 3.0 cal; a sized canopy lands within 3% of target | lib/sim/trim.test.ts:14-72; lib/sim/recovery.test.ts:55-80 | M8.1 | `hpr_design::assist::tests::ballast_and_canopy_sizing_round_trip` |

## Process mistakes

| id | mistake | Loft evidence | guard in hpr-sim |
|---|---|---|---|
| P1 | A two-track rule put UI ahead of physics; still 3-DOF after 22 runs | ROADMAP.md:10-16; MAINTAINING.md:477-483 | CLAUDE.md hard rule 1; UI is Phase 7, after validation |
| P2 | 6-DOF was queued, deferred twice, and its slot id reused | ROADMAP.md:2452-2457,8012-8040 | ROADMAP.md rule: milestones are never removed, renumbered or moved later without an ADR |
| P3 | The defect ledger became the work queue: 18 merged commits added no capability | MAINTAINING.md:493-515; BACKLOG.md:3-6 | CLAUDE.md: defects outside the milestone become GitHub issues; a wrong number in merged physics preempts it |
| P4 | Open-ended audit sweeps each run produced defects faster than they were fixed | BACKLOG.md:400-403,894,1008 | Reviews are scoped to the diff (CLAUDE.md "Review before merge") |
| P5 | Docs grew without bound: an 8,761-line roadmap, one milestone about 1,700 lines | ROADMAP.md:2593-4302 | `xtask` test `docs_stay_within_budget` (STATUS 150 lines, research files 200) |
| P6 | The status line named the wrong current milestone for six milestones | ROADMAP.md:28-47 | `xtask` test `status_names_the_first_open_milestone` |
| P7 | Clean-room slips: an agent fetched OpenRocket `.java` files, and Loft code cites them | BACKLOG.md:35-41; lib/sim/mass.ts:254 | Hard rule 3; `guard-bash.py` blocks fetching OpenRocket source; the hazard list above |
| P8 | An oracle and an LGPL wheel shipped in the browser bundle while the notices said otherwise | OWNER-NOTES.md:752-770 | ADR-001 licence policy and `cargo deny`; oracles live only in `validation/oracles` and `refs/` (ADR-002) |
| P9 | Reading the tail of test output missed three red tests | AGENTS.md:40-43 | CLAUDE.md: read the summary lines, not the tail |
| P10 | README and changelog claimed importers that didn't exist | BACKLOG.md:1888-1895 | M4.2 generates the README's command and format table from the registered commands |
| P11 | A fix reached one surface of seven; tests re-implemented the conversion they tested | HANDOFF.md:70-97; MAINTAINING.md:808-810 | code-reviewer flags tests that re-implement what they test; L35 puts withheld values in output types |
| P12 | Throughput pressure (an increment every 15 to 25 minutes, ritual tracker rows) made tiny slices | MAINTAINING.md:598-700 | CLAUDE.md "How work ships": the unit is a milestone or increment with its own *done when*; no quotas |
| P13 | Unreproduced subagent claims were recorded as facts; several "Sev-1" findings were later refuted | BACKLOG.md:87-92,1008-1009 | CLAUDE.md: a subagent finding is a claim until reproduced in the session |
| P14 | Questions parked in an overwritten handoff vanished within a day | HANDOFF.md:3 | STATUS.md "Needs Neer" and "Decided without Neer" persist |
| P15 | Agents built interactions Neer rejected (drag-to-shape, a wall of parameters) that surfaced only on the live site | OWNER-NOTES.md:312-320,441-452 | M9.0 is an ADR plus spike and reads that note first: select-and-edit, component tree with property dialogs, popovers, phone portrait |
| P16 | Attribution footers leaked into PR bodies; pushes to main deployed untested | MAINTAINING.md:736,955; AGENTS.md:35-37 | `guard-bash.py` blocks traces, pushes to main and merges without green CI |

## Competition notes (from Loft's `COMPETITION.md`; re-check the sources before relying on them)

- **Edge:** Loft read the file you have and set several answers side by side (M4.2 `hpr compare`);
  OpenRocket already reads `.ork`, `.rkt` and `.CDX1`, so the edge is a lossless round trip (L58+).
- **Accuracy bars (UNVERIFIED):** RocketPy's README cites 2 flights at +0.45% and -0.75% apogee
  (J. Aerosp. Eng., doi 10.1061/(ASCE)AS.1943-5525.0001331); Loft's RASAero II figure has no source.
- **Gaps in every tool:** none flags out-of-envelope numbers (M1.8, M1.10); RocketPy's
  ensemble-weather Monte Carlo is the bar for M6.1; none runs on mobile (M9.4).
