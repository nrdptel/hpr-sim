"""How far RocketPy 1.13.0's calm-air drifts move when it frees the rocket where hpr does (M2.1d2,
issue #50). A measurement, not yet a fixture or a check.

RocketPy frees the rocket when its first (forward) rail button leaves the rail (`effective_1rl`,
`flight.py:1716-1730`); hpr keeps it guided until its last (aft) button leaves. Lengthening
RocketPy's rail by the distance between the two buttons makes its first button leave where hpr's
last one does, with everything else as `flight.py` flies the calm-air case. The script prints each
calm case's apogee and landing drift as committed and with the longer rail.

Since M2.1d3 `flight.fly` flies RocketPy's equations with the upstream corrections
(`corrections.py`, ADR-026), so this script's numbers are no longer the ones ADR-025 quotes, which
were measured on RocketPy as released. `wind_response.py` measures the release in every case, in
wind too, alongside the other differences between the codes.

Run from the repo root:

    refs/venv/bin/python validation/oracles/rocketpy/rail_release.py [case ...]

with no names for every calm-air case, or with `<base>-calm` names to pick some.
"""

import sys

import flight
import recovery


def main():
    keep = set(sys.argv[1:])
    document = recovery.mass_fixture()
    cases = flight.calm_air_cases(recovery.CASES + flight.WHOLE_FLIGHT_ONLY_CASES)
    cases = [case for case in cases if not keep or case["name"] in keep]
    if keep and len(cases) != len(keep):
        flight.fail(f"unknown case(s): {sorted(keep - {case['name'] for case in cases})}")
    for case in cases:
        base = case["base"]
        inputs = recovery.mass_case(document, base)
        inputs["name"] = base
        buttons = inputs["geometry"].get("rail_buttons") or flight.fail(f"{base} has no rail buttons")
        spacing = abs(buttons["upper_button_position"] - buttons["lower_button_position"])
        env = recovery.environment_of(case)
        for label, extra in (("as committed", 0.0), ("rail longer by the button spacing", spacing)):
            rocket, _ = flight.build_rocket(dict(inputs), False)
            rail = dict(flight.RAILS[base])
            rail["rail_length_m"] += extra
            metrics = flight.metrics_of(
                flight.fly(rocket, env, rail, flight.SOLVER), case, case["name"], flight.SOLVER
            )
            print(
                f"{case['name']}, {label} (+{extra:.3f} m): "
                f"apogee_drift_m {metrics['apogee_drift_m']:.1f}, "
                f"landing_drift_m {metrics['landing_drift_m']:.1f}, "
                f"apogee_agl_m {metrics['apogee_agl_m']:.1f}"
            )


if __name__ == "__main__":
    main()
