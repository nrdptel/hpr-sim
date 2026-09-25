#!/usr/bin/env bash
# The local gate from CLAUDE.md, with terse output: one line per step, and for a failing step only
# the lines that say why. Every step runs even after a failure, so one run reports everything.
# Full logs stay in target/gate/<step>.log for when the summary is not enough.
#
# Usage:
#   scripts/gate.sh                 # every step, in CLAUDE.md's order
#   scripts/gate.sh clippy test     # just these steps
#   GATE_TAIL=80 scripts/gate.sh    # show more of each failure (default 40 lines)
#
# Steps: fmt clippy test doc wasm validate deny site examples
# `validate` runs `cargo xtask validate --check`, which is what CI runs: every case, compared with
# the committed report. Regenerating the report is a separate, deliberate step
# (`cargo xtask validate`, debug build).
#
# Run it in the foreground. A background gate dies with a headless session that ends its turn.

set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

ALL="fmt clippy test doc wasm validate deny site examples"
STEPS="${*:-$ALL}"
TAIL="${GATE_TAIL:-40}"
LOGDIR="target/gate"
mkdir -p "$LOGDIR"

cmd_for() {
  case "$1" in
    fmt) echo "cargo fmt --all --check" ;;
    clippy) echo "cargo clippy --workspace --all-targets --all-features -- -D warnings" ;;
    # cargo test rather than nextest: nextest skips doctests, and CI runs cargo test.
    test) echo "cargo test --workspace --all-features" ;;
    doc) echo "env RUSTDOCFLAGS=-D\\ warnings cargo doc --workspace --no-deps --all-features" ;;
    wasm) echo "cargo xtask wasm-check" ;;
    validate) echo "cargo xtask validate --check" ;;
    deny) echo "cargo deny check" ;;
    site) echo "cargo xtask site" ;;
    examples) echo "cargo xtask examples --check" ;;
    *) return 1 ;;
  esac
}

# The lines that explain a failure, in log order, then the log's last lines. A panic and a compiler
# error keep a few lines after them, where the assertion's left and right values and the source
# excerpt are.
why() {
  local log="$1" hits
  hits=$( {
    grep -nE '^test .* FAILED|^\s+FAIL \[|test result: FAILED|^Diff in |mismatch|not reproduced|does not reproduce' "$log"
    grep -nE -A5 '^(error|warning)(\[|:)|panicked at' "$log"
  } | grep -v '^--$' | sort -n -u | head -n "$TAIL")
  if [ -n "$hits" ]; then
    printf '%s\n' "$hits" | sed 's/^/    /'
    echo "    (first $TAIL matching lines of $log; last lines follow)"
  fi
  tail -n 12 "$log" | sed 's/^/    | /'
}

failed=""
start_all=$(date +%s)
for step in $STEPS; do
  cmd=$(cmd_for "$step") || { echo "unknown step: $step (known: $ALL)" >&2; exit 64; }
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
