# Checking a claim

Every number hpr-sim computes, and every number on this site, should lead back to three things: the
published source of the model behind it, a test that pins it, and, where one exists, a comparison
with another program or a real flight. This page shows how to follow that trail, with two worked
examples. It is for anyone who would rather check a claim than trust it. If a trail breaks, that is
a bug, and the last section says how to report it.

## The trail

1. **The page.** Each model has a page on this site. Its *In short* says what the model is, where
   it comes from, how well it is validated and what it leaves out. Below that, each section gives
   the equations in words and symbols, and cites its source by a short key in square brackets,
   such as **[NGA]**.
2. **The source.** Near the top of the page, under *Code and sources* (or *Sources*), each key's
   full reference is listed. Sources that can be downloaded are pinned in the
   [reference lock file][lock] by address and SHA-256 hash, a fingerprint of the file's exact bytes
   ([ADR-002][adr-002], the reference library decision), so everyone checks the same file.
   `cargo xtask refs fetch` downloads them into the repository's `refs/` folder, which is never
   committed, and `cargo xtask refs verify` checks their hashes again. A few sources could not be
   fetched, such as Dormand and Prince's 1980 paper, and the page says so: "cited, not fetched".
3. **The test.** The page's *Verification* or *Tests that pin this* section names the tests that
   hold the code to the source, and the [tolerance](glossary.md#tolerance) of each: how far a result
   may be from its reference and still pass. `cargo test` runs all of them, on every change, on
   macOS, Windows and Linux.
4. **The comparison.** Where a model is compared with another program or a high-precision
   calculation:
   - The reference values are a [fixture](glossary.md#reference-value-and-fixture), a file under
     [`validation/fixtures/`][fixtures].
   - A program under [`validation/oracles/`][oracles], an [oracle](glossary.md#oracle), writes
     each fixture, with a record of how.
   - The model page's verification section names the fixture and the test that reads it.
   - Some comparisons, today the descents under a parachute, are also run by the validation
     harness: the program behind `cargo xtask validate`, which flies them again and rewrites the
     committed [validation report][report], one row per number.
   - Each of those has a [case file](glossary.md#validation-case) under
     [`validation/cases/`][cases]. It says what is flown and how close the two must agree, and
     argues for that tolerance.
   - The rules behind this are in [ADR-015][adr-015], the validation harness decision.

The [Accuracy](accuracy.md) page gathers every result from steps 3 and 4 in one place.

## Example: gravity at the equator

**The claim.** hpr's gravity on the equator, on the [WGS 84](glossary.md#wgs-84) ellipsoid, is
9.7803253359 m/s². That is at height 0 above the ellipsoid, which is not quite sea level (see
[ellipsoidal height](glossary.md#ellipsoidal-height)).

1. **The page.** [Gravity and Earth rotation](physics/gravity.md) models
   [normal gravity](glossary.md#normal-gravity), the pull of an idealised Earth plus the effect of
   its spin. The table under [Defining parameters and
   derived constants](physics/gravity.md#defining-parameters-and-derived-constants) gives `γ_e`, the
   value at the equator, as 9.7803253359 m/s², and cites Table 3.6 of **[NGA]**.
2. **The source.** **[NGA]** is the US National Geospatial-Intelligence Agency's WGS 84 standard,
   NGA.STND.0036 (2014), a US government work. The lock file pins it as `wgs84-nga-stnd-0036`, and
   `cargo xtask refs fetch wgs84-nga-stnd-0036` downloads it. Table 3.6 prints the same value.
3. **The test.** [Tests that pin this](physics/gravity.md#tests-that-pin-this) names
   `gravity::tests::somigliana_matches_published_values` in
   [`crates/hpr-core/src/gravity.rs`][gravity-rs]. It checks the constant against Table 3.6 to its
   printed digits. It also checks gravity at 11 latitudes and heights against
   [values computed from the standard's formulas at 40 digits][gravity-fixture], by a separate
   script, [`normal_gravity.py`][gravity-oracle]. Run it with:

   ```text
   cargo test -p hpr-core somigliana_matches_published_values
   ```

4. **The comparison.** None is needed here: the standard itself is the reference. This is the
   second of the four kinds of evidence that [Accuracy](accuracy.md) describes: a model checked
   against its published source.

## Example: how far a rocket drifts under its parachute

**The claim.** For the NDRT 2020 rocket, one of RocketPy's
[example rockets](glossary.md#example-rockets), hpr's north [drift](glossary.md#drift) under the
parachute differs from RocketPy's by +2.865%, inside the 3% tolerance its case allows.

1. **The report.** The [validation report][report] has a row for the case
   `descent-ndrt-2020-nose-to-tail` and the metric `drift_north_m`: hpr −50.83 m, RocketPy
   −49.42 m, a difference of +2.865%, a tolerance of 3.000%, and the verdict *pass*.
2. **The case.** [`descent-ndrt-2020-nose-to-tail.toml`][ndrt-case] names the rocket's design and
   the reference file, and holds each [metric](glossary.md#metric) to 3%. Its opening comment
   argues why there is no absolute floor, a fixed allowance in metres that would pass any smaller
   difference: a floor big enough to excuse a drift component near zero would hide a real error
   in a large one.
3. **The reference.** The values come from [`rocketpy-descent.json`][descent-fixture], which the
   report names with its hash. The script [`recovery.py`][recovery-oracle] wrote it by flying
   RocketPy 1.13.0's own parachute phase; the lock file pins that RocketPy by commit. Its first
   lines say what RocketPy computes, and what the comparison overrides (both codes start from the
   same state, with the first parachute opening at once).
4. **The model.** [Recovery](physics/recovery.md) explains hpr's descent, and its section
   [Against RocketPy](physics/recovery.md#against-rocketpy) explains the differences that remain.
   The likely cause is [added mass](glossary.md#added-mass): RocketPy treats the air a canopy
   drags along as extra mass, 15.9 kg for NDRT's main against the rocket's 20.8 kg, and hpr has no
   such term. It slows the response to the opening, which most likely moves this small drift
   component by 2.86%. That fits the size of the gap, but no test has isolated it yet.
5. **The test.** `recovery::tests::descent_matches_rocketpy_examples` in
   [`crates/hpr-sim/src/recovery.rs`][recovery-rs] replays all five cases on every change, and
   `cargo xtask validate` writes them into the report.

What this shows, and what it doesn't: the two codes agree on the physics of a descent. It says
nothing about whether either matches a real parachute on a real day. Real flights are compared in
[M2.3](decisions-and-roadmap.md#m2-3), the real-flights milestone.

## Rules that keep the trail honest

- **A reference is never changed to make a comparison pass.** It moves only when its generator runs
  again, which is a deliberate step recorded in the history. This guards against
  [Loft lesson L76](decisions-and-roadmap.md#l76): Loft, the project before this one, said to regenerate a reference
  when its drift check failed, so the reference moved with Loft's own drag.
- **A tolerance lives in its case file, with its reason.** Loosening one to turn a failure into a
  pass is not allowed. A target that can't be met gets a
  [decision record](glossary.md#decision-record-adr) that shows the measurement, and the gap stays
  in the report.
- **The site checks its own numbers.** The site's build fails if any of these breaks:
  - Every number on [Accuracy](accuracy.md) outside code (anything with two or more digits, a
    decimal point, an exponent or a percent sign) must appear in a file that its paragraph, list
    item or table row links to, such as a model page outside its *In short*, the validation report
    or a case file. A guide page such as this one doesn't count, so no page can vouch for itself.
  - It must appear there written the same way: the same digits, and the same sign and percent sign
    where Accuracy writes them. So 2.8 doesn't match 2.865, and −2.865% doesn't match +2.865%.
  - Accuracy's table of the descent results must match the validation report cell by cell, and
    hold every result in it.
  - Every number in a model page's *In short* must appear in the rest of that page, or in a file
    its item links to.
- **What that check can't see.** When a number changes at its source, the build fails until the
  page that quotes it follows, unless the old number still appears somewhere else in that file.
  And the check can't tell whether a number is quoted in the right context, such as for the right
  rocket. The reviews check that, and the model pages say what each number means.

## When the trail breaks

If a number on this site or from the code has no source, no test, or a test that doesn't check what
the page says, please [open an issue][issues] with the page and the number. The pencil icon at the
top of each page opens its source on GitHub, where you can propose a fix.

[adr-002]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-002-the-reference-library-lock-file-fetch-verify-and-doctor-2026-09-17
[adr-015]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-015-the-validation-harness-cases-references-tolerances-and-reports-2026-09-17
[cases]: https://github.com/nrdptel/hpr-sim/tree/main/validation/cases
[descent-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/recovery/rocketpy-descent.json
[fixtures]: https://github.com/nrdptel/hpr-sim/tree/main/validation/fixtures
[gravity-fixture]: https://github.com/nrdptel/hpr-sim/blob/main/validation/fixtures/earth/wgs84-normal-gravity.json
[gravity-oracle]: https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/wgs84/normal_gravity.py
[gravity-rs]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-core/src/gravity.rs
[issues]: https://github.com/nrdptel/hpr-sim/issues
[lock]: https://github.com/nrdptel/hpr-sim/blob/main/validation/refs.lock.toml
[ndrt-case]: https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/descent-ndrt-2020-nose-to-tail.toml
[oracles]: https://github.com/nrdptel/hpr-sim/tree/main/validation/oracles
[recovery-oracle]: https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/rocketpy/recovery.py
[recovery-rs]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/src/recovery.rs
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
