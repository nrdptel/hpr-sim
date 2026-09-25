#!/usr/bin/env bash
# The local gate every change passes before it is pushed, with terse output: one line per step,
# and for a failing step only the lines that say why. Every step runs even after a failure, so one
# run reports everything. Full logs stay in target/gate/<step>.log for when the summary is not
# enough.
#
# Usage:
#   scripts/gate.sh                 # every step
#   scripts/gate.sh clippy test     # just these steps
#   GATE_TAIL=80 scripts/gate.sh    # show more of each failure (default 40 lines)
#
# Steps: fmt clippy test doc wasm validate deny site examples
# The commands are CI's own (.github/workflows/ci.yml), `--locked` included, so a Cargo.lock that
# needs updating fails here rather than on every CI job. `validate` is `--check`: every case,
# compared with the committed report. Regenerating the report is a separate, deliberate step
# (`cargo xtask validate`, debug build). CI's test job also sets RUSTFLAGS=-D warnings; clippy
# with -D warnings covers that here without rebuilding everything under a second set of flags.
#
# Run it in the foreground; it takes about 4 minutes on a warm build.

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

ALL="fmt clippy test doc wasm validate deny site examples"
STEPS="${*:-$ALL}"
TAIL="${GATE_TAIL:-40}"
LOGDIR="target/gate"
XTASK="cargo run --locked --quiet --package xtask --"

cmd_for() {
  case "$1" in
    fmt) echo "cargo fmt --all --check" ;;
    clippy) echo "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings" ;;
    # cargo test rather than nextest: nextest skips doctests, and CI runs cargo test.
    test) echo "cargo test --workspace --all-features --locked" ;;
    doc) echo "env RUSTDOCFLAGS=-D\\ warnings cargo doc --workspace --no-deps --all-features --locked" ;;
    wasm) echo "$XTASK wasm-check --locked" ;;
    validate) echo "$XTASK validate --check" ;;
    deny) echo "cargo deny check" ;;
    site) echo "$XTASK site --locked" ;;
    examples) echo "$XTASK examples --check --locked" ;;
    *) return 1 ;;
  esac
}

for step in $STEPS; do
  cmd_for "$step" >/dev/null || { echo "unknown step: $step (known: $ALL)" >&2; exit 64; }
done
mkdir -p "$LOGDIR"

# The lines that explain a failure, then the log's last lines. Failures come first, errors next and
# warnings last, so a wall of warnings cannot push a failing test out of the budget. A panic and a
# compiler message keep the lines after them, where an assertion's left and right values and the
# source excerpt are.
why() {
  local log="$1" hits
  hits=$( {
    { grep -nE '^test .* FAILED|^\s+FAIL \[|test result: FAILED|^Diff in |mismatch|not reproduced|does not reproduce' "$log"
      grep -nE -A5 'panicked at' "$log"; } | grep -v '^--$' | sort -n -u
    grep -nE -A5 '^error(\[|:)' "$log" | grep -v '^--$'
    grep -nE -A5 '^warning(\[|:)' "$log" | grep -v '^--$'
  } | awk '!seen[$0]++' | head -n "$TAIL")
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" | sed 's/^/    /'
    echo "    (up to $TAIL lines of $log: failures, then errors, then warnings; its last lines follow)"
  fi
  tail -n 12 "$log" | sed 's/^/    | /'
}

failed=""
start_all=$(date +%s)
for step in $STEPS; do
  cmd=$(cmd_for "$step")
  log="$LOGDIR/$step.log"
  t0=$(date +%s)
  eval "$cmd" > "$log" 2>&1
  rc=$?
  dt=$(( $(date +%s) - t0 ))
  if [ "$rc" -eq 0 ]; then
    extra=""
    [ "$step" = test ] && extra=" — $(grep -cE '^test result: ok' "$log") suites ok"
    echo "PASS $step (${dt}s)$extra"
  else
    echo "FAIL $step (${dt}s, exit $rc): $cmd"
    why "$log"
    failed="$failed $step"
  fi
done
total=$(( $(date +%s) - start_all ))
if [ -n "$failed" ]; then
  echo "GATE FAILED in ${total}s:$failed"
  exit 1
fi
echo "GATE PASSED in ${total}s ($STEPS)"
