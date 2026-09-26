#!/usr/bin/env bash
# SessionStart hook (startup, resume, clear, compact). Whatever this prints to stdout is added to
# Claude's context. It re-anchors every session, and every session after compaction, on the
# current state. Keep the output short.
set -u
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$root" 2>/dev/null || exit 0

now=$(date +%s)
echo "## hpr-sim session context (injected by .claude/hooks/session-context.sh)"
echo "- Local time: $(date '+%Y-%m-%d %H:%M %Z')"

# Only an autopilot cycle (scripts/autopilot.sh sets HPR_AUTOPILOT=1) is bound by the deadline.
# An interactive session is Neer's: it hears whether a run is going, and to keep out of its way.
run_pid=$(tr -dc '0-9' 2>/dev/null < .autopilot/pid)
if [ -n "$run_pid" ] && kill -0 "$run_pid" 2>/dev/null \
  && ps -o command= -p "$run_pid" 2>/dev/null | grep -q 'autopilot\.sh'; then :; else run_pid=""; fi
if [ "${HPR_AUTOPILOT:-}" != "1" ]; then
  if [ -n "$run_pid" ]; then
    deadline=$(tr -dc '0-9' 2>/dev/null < .autopilot/deadline)
    left=$(( (${deadline:-$now} - now) / 60 ))
    echo "- Interactive session. An autopilot run is going in the background (pid $run_pid, $(( left / 60 ))h $(( left % 60 ))m left). This session is not part of it: don't edit files, switch branches or commit in this checkout while it runs. To check on it or stop it, use the autopilot skill."
  else
    echo "- Interactive session; no autopilot run is going. The autopilot skill starts one (\"start autopilot for 8 hours\")."
  fi
elif [ -f .autopilot/deadline ]; then
  deadline=$(tr -dc '0-9' < .autopilot/deadline)
  if [ -n "$deadline" ]; then
    left=$(( (deadline - now) / 60 ))
    if [ "$left" -le 0 ]; then
      echo "- AUTOPILOT DEADLINE HAS PASSED. Commit work in progress to a branch, push it, open a draft PR, update docs/STATUS.md, then stop."
    elif [ "$left" -le 45 ]; then
      echo "- Autopilot: ${left} min left. Don't start new work; wrap up as CLAUDE.md describes."
    else
      echo "- Autopilot: $(( left / 60 ))h $(( left % 60 ))m left in this run (deadline in .autopilot/deadline)."
    fi
  fi
fi

if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  branch=$(git branch --show-current 2>/dev/null)
  dirty=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
  echo "- Git: branch '${branch:-detached}', ${dirty} changed path(s). Last commit: $(git log -1 --format='%h %s' 2>/dev/null)"
  open_prs=$(gh pr list --state open --json number,title,headRefName --jq '.[] | "#\(.number) \(.headRefName): \(.title)"' 2>/dev/null | head -5)
  if [ -n "$open_prs" ]; then
    echo "- Open PRs (finish or close these before starting new work):"
    printf '%s\n' "$open_prs" | sed 's/^/  - /'
  fi
fi

echo "- Rules: CLAUDE.md is binding. Never ask Neer questions. No AI traces. Clean room (no GPL source). Never commit refs/ or loft-fixtures content."
echo
if [ -f docs/STATUS.md ]; then
  echo "### docs/STATUS.md (first 60 lines)"
  head -n 60 docs/STATUS.md
fi
exit 0
