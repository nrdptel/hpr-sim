#!/usr/bin/env bash
# Wait, in the foreground, for a pull request's CI checks to finish, then print the final table
# once. Watching with `gh pr checks --watch` directly reprints the whole table every interval,
# and right after a push, before the new commit's checks have registered, it returns at once or
# reports the previous commit's.
#
# Usage:   scripts/ci-wait.sh <pr-number>
# Exit:    0  every check passed or was skipped
#          1  a check failed or was cancelled
#          2  no checks registered for the PR's head within two minutes (dispatch CI, then rerun)
#          3  still running after CI_WAIT_MAX seconds (default 1080); run it again to keep waiting
# Timing:  CI takes about 6 to 7 minutes. The default bound stays under a 20-minute call timeout.

set -uo pipefail
pr="${1:?usage: scripts/ci-wait.sh <pr-number>}"
max="${CI_WAIT_MAX:-1080}"
case "$max" in ''|*[!0-9]*) max=1080 ;; esac
start=$(date +%s)

# When the PR's branch is the one checked out here, its checks count only once GitHub shows the
# commit just pushed as the PR's head.
want=""
branch=$(git branch --show-current 2>/dev/null || true)
pr_branch=$(gh pr view "$pr" --json headRefName -q .headRefName 2>/dev/null || true)
[ -n "$branch" ] && [ "$branch" = "$pr_branch" ] && want=$(git rev-parse HEAD 2>/dev/null || true)

registered=0
for _ in $(seq 1 24); do
  head=$(gh pr view "$pr" --json headRefOid -q .headRefOid 2>/dev/null || true)
  n=$(gh pr checks "$pr" --json name --jq 'length' 2>/dev/null || echo 0)
  if { [ -z "$want" ] || [ "$head" = "$want" ]; } && [ "${n:-0}" -gt 0 ] 2>/dev/null; then
    registered=1
    break
  fi
  sleep 5
done
if [ "$registered" -eq 0 ]; then
  echo "no checks registered for PR $pr's head${want:+ ($want)} after 2 minutes"
  exit 2
fi

while :; do
  pending=$(gh pr checks "$pr" --json bucket --jq '[.[] | select(.bucket == "pending")] | length' 2>/dev/null || echo 1)
  [ "${pending:-1}" -eq 0 ] 2>/dev/null && break
  if [ $(( $(date +%s) - start )) -ge "$max" ]; then
    gh pr checks "$pr"
    echo "CI STILL RUNNING on PR $pr after ${max}s: $pending check(s) pending; run this again to keep waiting"
    exit 3
  fi
  sleep 30
done

gh pr checks "$pr"
bad=$(gh pr checks "$pr" --json bucket --jq '[.[] | select(.bucket == "fail" or .bucket == "cancel")] | length' 2>/dev/null || echo 1)
if [ "${bad:-1}" -eq 0 ] 2>/dev/null; then
  echo "CI PASSED on PR $pr"
  exit 0
fi
echo "CI NOT GREEN on PR $pr: $bad check(s) failed or were cancelled"
exit 1
