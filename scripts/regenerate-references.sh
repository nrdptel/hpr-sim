#!/usr/bin/env bash
# Regenerates the references `cargo xtask validate` compares against, from the oracles, then the
# validation report, and leaves the result in the working tree as a diff for a person to review.
# It never commits (M2.1c1, ADR-022). The `Regenerate references` workflow runs it when someone
# triggers it by hand; it runs locally too, on macOS or Linux.
#
# Needs the oracle environment: `cargo xtask refs fetch python rocketpy` (uv installs RocketPy and
# its dependencies, pinned by `validation/oracles/uv.lock`, into the gitignored refs/venv).
#
# The chain, in order, because each step reads what the one before it wrote:
#
# 1. rocket_mass.py: RocketPy's example rockets, their masses, inertias and motors, as RocketPy
#    computes them (validation/fixtures/design/rocketpy-rocket-mass.json).
# 2. `cargo xtask designs`: hpr's design files for those rockets (validation/designs/).
# 3. recovery.py: RocketPy's descents under each rocket's parachutes
#    (validation/fixtures/recovery/rocketpy-descent.json).
# 4. flight.py: RocketPy's whole flights, pad to landing, on the declared drag and, with
#    `--own-drag`, on each example's own (validation/fixtures/flight/; the second reads the drag
#    curves from refs/rocketpy).
# 5. `cargo xtask validate`: the report, validation/reports/latest.{md,json}, rewritten only when
#    the run does not reproduce the committed one.
#
# A generator writes to a temporary file first, so one that fails leaves the committed fixture as
# it was. The script stops at the first generator that fails. A report whose metrics fall outside
# tolerance is still written, so the diff shows what moved, and the script then exits non-zero.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

python=refs/venv/bin/python
if [[ ! -x $python ]]; then
    echo "error: $python is missing; run \`cargo xtask refs fetch python rocketpy\` first" >&2
    exit 1
fi

# `cargo xtask` without its alias, so the build is --locked, as in CI.
xtask() {
    cargo run --locked --quiet --package xtask -- "$@"
}

# Runs a generator from the repository root and replaces its fixture only if it succeeded. The
# temporary file sits beside the fixture, so the move is atomic, and is removed if the script stops.
tmp=
trap 'rm -f -- "$tmp"' EXIT
generate() {
    local script=$1 fixture=$2
    shift 2
    echo "regenerate: $script $* > $fixture"
    tmp="$fixture.new"
    "$python" "$script" "$@" > "$tmp"
    mv "$tmp" "$fixture"
    tmp=
}

generate validation/oracles/rocketpy/rocket_mass.py validation/fixtures/design/rocketpy-rocket-mass.json
xtask designs
generate validation/oracles/rocketpy/recovery.py validation/fixtures/recovery/rocketpy-descent.json
generate validation/oracles/rocketpy/flight.py validation/fixtures/flight/rocketpy-whole-flight.json
generate validation/oracles/rocketpy/flight.py validation/fixtures/flight/rocketpy-whole-flight-own-drag.json \
    --own-drag

# How far each regenerated fixture moved from the committed one, number by number. Another machine's
# floating point moves RocketPy's last digits, which changes a fixture's hash and so the report: on
# GitHub's macOS runner the descents moved by at most 3.6e-11 relative, and the whole flights' 742
# moved values by at most 2.9e-6 on this measure (a landing height of 2e-8 m moving by 3e-9 m). A
# real change moves numbers by far more.
for fixture in \
    validation/fixtures/design/rocketpy-rocket-mass.json \
    validation/fixtures/recovery/rocketpy-descent.json \
    validation/fixtures/flight/rocketpy-whole-flight.json \
    validation/fixtures/flight/rocketpy-whole-flight-own-drag.json; do
    # A fixture not committed yet has nothing to compare with; `|| true` keeps pipefail from
    # stopping the script there.
    { git show "HEAD:$fixture" 2>/dev/null || true; } | "$python" -c '
import json, sys

def walk(old, new, path, moved):
    if isinstance(old, dict) and isinstance(new, dict) and old.keys() == new.keys():
        for key in old:
            walk(old[key], new[key], f"{path}.{key}", moved)
    elif isinstance(old, list) and isinstance(new, list) and len(old) == len(new):
        for index, (a, b) in enumerate(zip(old, new)):
            walk(a, b, f"{path}[{index}]", moved)
    elif isinstance(old, (int, float)) and isinstance(new, (int, float)) \
            and not isinstance(old, bool) and not isinstance(new, bool):
        if old != new:
            # Relative to the larger, and numbers under 1e-3, such as a height at landing that
            # the event solver leaves at 2e-8 m, as if they were 1e-3.
            moved.append((abs(old - new) / max(abs(old), abs(new), 1e-3), path))
    elif old != new:
        moved.append((float("inf"), path))

try:
    old = json.load(sys.stdin)
except ValueError:
    sys.exit(0)  # not committed yet: there is nothing to compare with
with open(sys.argv[1]) as file:
    new = json.load(file)
moved = []
walk(old, new, "$", moved)
if not moved:
    print(f"regenerate: {sys.argv[1]}: unchanged")
else:
    worst, where = max(moved)
    print(f"regenerate: {sys.argv[1]}: {len(moved)} value(s) moved, the largest by {worst:.1e} "
          f"relative, at {where}")
' "$fixture"
done

# The committed report was written on macOS, and another platform rounds a whole flight's last
# digits differently, so the report is rewritten only when this run does not reproduce it
# (`cargo xtask validate --check`, to the digits the platforms share). A fixture that moved at all
# changes its hash, which the report records, so the report is then rewritten.
status=0
if xtask validate --check; then
    echo "regenerate: the committed report reproduces; leaving it as it is"
else
    xtask validate || status=$?
fi

# Only validation/ is written above, so only it is summarised: other local edits are not ours.
echo
if [[ -z $(git status --porcelain -- validation) ]]; then
    echo "regenerate: nothing changed; the committed references reproduce"
else
    echo "regenerate: these files changed; review the diff before committing any of it:"
    git status --short -- validation
    git diff --stat -- validation
fi
exit "$status"
