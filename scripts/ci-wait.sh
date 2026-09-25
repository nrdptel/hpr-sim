#!/usr/bin/env bash
# Wait, in the foreground, for a pull request's CI checks to finish, then print the final table
# once. Watching with `gh pr checks --watch` directly reprints the whole table every interval,
# and right after a push, before the new commit's checks have registered, it returns at once.
#
# Usage:   scripts/ci-wait.sh <pr-number>
# Exit:    0 when every check passed or was skipped, 1 when any failed or was cancelled,
#          2 when no checks registered within two minutes (dispatch CI on the branch, then rerun).
# Timing:  CI takes about 6 to 7 minutes, so give the call a timeout of 15 minutes or more.

set -uo pipefail
pr="${1:?usage: scripts/ci-wait.sh <pr-number>}"

registered=0
for _ in $(seq 1 24); do
  n=$(gh pr checks "$pr" --json name --jq 'length' 2>/dev/null || echo 0)
  if [ "${n:-0}" -gt 0 ] 2>/dev/null; then registered=1; break; fi
  sleep 5
done
if [ "$registered" -eq 0 ]; then
  echo "no checks registered on PR $pr after 2 minutes"
  exit 2
fi

gh pr checks "$pr" --watch --interval 30 >/dev/null 2>&1
gh pr checks "$pr"
bad=$(gh pr checks "$pr" --json bucket --jq '[.[] | select(.bucket == "fail" or .bucket == "cancel" or .bucket == "pending")] | length' 2>/dev/null || echo 1)
if [ "${bad:-1}" -eq 0 ] 2>/dev/null; then
  echo "CI PASSED on PR $pr"
  exit 0
fi
echo "CI NOT GREEN on PR $pr: $bad check(s) failed, cancelled or still pending"
exit 1
