#!/usr/bin/env bash
# What a cold build of the workspace costs in memory, wall time and disk. The numbers in
# docs/perf.md's "Build memory during an unattended run" come from this script.
#
# Each repetition builds into an empty target directory, so nothing is reused, and samples the
# resident memory of the build's whole process group once a second. The memory figure sums
# processes, so it double-counts pages they share: compare runs with each other rather than
# treating one as exact. The directory is removed afterwards, and the real target/ is untouched.
#
# Usage:
#   scripts/build-memory.sh                                          # one repetition, defaults
#   scripts/build-memory.sh 3                                        # three repetitions
#   CARGO_BUILD_JOBS=6 scripts/build-memory.sh 3                     # a job cap
#   CARGO_PROFILE_DEV_DEBUG=line-tables-only scripts/build-memory.sh 3
#   HPR_MEASURE_RUN=1 scripts/build-memory.sh 3                    # build and run the tests
#
# Run it on an otherwise quiet machine: anything else building at the same time lands in the
# same measurement.

set -uo pipefail
set -m   # each build becomes its own process-group leader, so the sampler can find its children

REPS="${1:-1}"
case "$REPS" in ''|*[!0-9]*) echo "usage: $0 [repetitions]" >&2; exit 64 ;; esac

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

# Compilation only by default. HPR_MEASURE_RUN=1 also runs the tests, which is a different
# question: test execution is bounded by RUST_TEST_THREADS, not by CARGO_BUILD_JOBS.
BUILD_CMD=(cargo test --workspace --all-features --no-run)
[ "${HPR_MEASURE_RUN:-0}" = "1" ] && BUILD_CMD=(cargo test --workspace --all-features)
SAMPLE_SECONDS=1

# Resident memory of every process in group $1, in KB. `ps -eo pgid=,rss=` is used rather than
# `ps -g` because the meaning of -g differs between systems.
group_rss_kb() { ps -eo pgid=,rss= 2>/dev/null | awk -v g="$1" '$1 == g { s += $2 } END { print s + 0 }'; }

printf 'command: %s\n' "${BUILD_CMD[*]}"
printf 'jobs: %s   test threads: %s   dev debug: %s   repetitions: %s\n\n' \
  "${CARGO_BUILD_JOBS:-one per core}" "${RUST_TEST_THREADS:-one per core}" \
  "${CARGO_PROFILE_DEV_DEBUG:-2 (default)}" "$REPS"

total_peak=0; total_wall=0; rc_all=0
for rep in $(seq 1 "$REPS"); do
  tdir="$(mktemp -d "${TMPDIR:-/tmp}/hpr-build-memory.XXXXXX")" || exit 1
  peak_kb=0
  start=$(date +%s)

  CARGO_TARGET_DIR="$tdir" "${BUILD_CMD[@]}" > "$tdir.log" 2>&1 &
  pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    rss=$(group_rss_kb "$pid")
    [ "${rss:-0}" -gt "$peak_kb" ] && peak_kb="$rss"
    sleep "$SAMPLE_SECONDS"
  done
  wait "$pid" 2>/dev/null; rc=$?
  [ "$rc" -ne 0 ] && { rc_all=$rc; echo "build failed (rc=$rc); last lines:"; tail -n 15 "$tdir.log"; }

  wall=$(( $(date +%s) - start ))
  written_mb=$(du -sm "$tdir" 2>/dev/null | awk '{print $1}')
  printf 'run %s: peak %s GB   wall %ss   written %s MB\n' \
    "$rep" "$(awk -v k="$peak_kb" 'BEGIN { printf "%.2f", k / 1048576 }')" "$wall" "${written_mb:-?}"

  total_peak=$(( total_peak + peak_kb )); total_wall=$(( total_wall + wall ))
  rm -rf "$tdir" "$tdir.log"
done

[ "$REPS" -gt 1 ] && printf '\nmean of %s: peak %s GB   wall %s s\n' "$REPS" \
  "$(awk -v k="$total_peak" -v n="$REPS" 'BEGIN { printf "%.2f", k / n / 1048576 }')" \
  "$(awk -v w="$total_wall" -v n="$REPS" 'BEGIN { printf "%.1f", w / n }')"
exit "$rc_all"
