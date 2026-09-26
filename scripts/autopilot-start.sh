#!/usr/bin/env bash
# Start an autopilot run detached from the terminal, so it outlives the window or the Claude Code
# session that started it. This is what the `autopilot` skill runs when Neer asks a session to
# "start autopilot for 8 hours"; it works the same typed straight into Terminal.
#
# Usage:
#   scripts/autopilot-start.sh 8h            # a new 8-hour window (also 12, 90m, 2h30m)
#   scripts/autopilot-start.sh --resume      # carry on with the unexpired window in .autopilot/deadline
#   scripts/autopilot-start.sh 8h --force    # start even though the working tree has changes
#
# It refuses to start a second run, or to start on a working tree with uncommitted changes (the
# first cycle switches branches in this checkout, and a commit could sweep those changes in).
# Environment overrides (HPR_EFFORT, HPR_COMPACT_WINDOW, ...) pass through to scripts/autopilot.sh.
#
# Check:  scripts/autopilot-status.sh      Stop:  scripts/autopilot-stop.sh [--now]
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
STATE=.autopilot
mkdir -p "$STATE"

DURATION=""
RESUME=0
FORCE=0
for arg in "$@"; do
  case "$arg" in
    --resume) RESUME=1 ;;
    --force) FORCE=1 ;;
    *) DURATION="$arg" ;;
  esac
done

# Turn the duration into minutes here too, to refuse a window too short to do anything: a cycle
# is never started with less than 45 minutes left.
minutes=""
if [ -n "$DURATION" ]; then
  if [[ "$DURATION" =~ ^[0-9]+$ ]]; then
    minutes=$(( 10#$DURATION * 60 ))
  elif [[ "$DURATION" =~ ^(([0-9]+)h)?(([0-9]+)m)?$ ]]; then
    minutes=$(( 10#${BASH_REMATCH[2]:-0} * 60 + 10#${BASH_REMATCH[4]:-0} ))
  else
    echo "✗ Can't read the duration '$DURATION'. Use hours (8), or 8h, 90m, 2h30m." >&2; exit 64
  fi
  if [ "$minutes" -lt 60 ]; then
    echo "✗ $DURATION is too short: no cycle starts with under 45 minutes left. Use at least 1h." >&2
    exit 64
  fi
elif [ "$RESUME" -eq 0 ]; then
  echo "✗ Say how long to run, e.g. scripts/autopilot-start.sh 8h (or --resume)." >&2; exit 64
fi
if [ "$RESUME" -eq 1 ] && [ -n "$DURATION" ]; then
  echo "✗ Give a duration for a new window, or --resume for the existing one, not both." >&2; exit 64
fi

# ---- refuse what would break the run -------------------------------------------------------
pid=$(tr -dc '0-9' 2>/dev/null < "$STATE/pid")
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null \
  && ps -o command= -p "$pid" 2>/dev/null | grep -q 'autopilot\.sh'; then
  echo "✗ An autopilot run is already going (pid $pid). See scripts/autopilot-status.sh;" >&2
  echo "  stop it with scripts/autopilot-stop.sh first if you want a new window." >&2
  exit 1
fi
if [ "$RESUME" -eq 1 ]; then
  d=$(tr -dc '0-9' 2>/dev/null < "$STATE/deadline")
  if [ -z "$d" ] || [ "$d" -le "$(date +%s)" ]; then
    echo "✗ No unexpired window to resume. Start a new one with a duration, e.g. 8h." >&2; exit 1
  fi
fi
dirty=$(git status --porcelain 2>/dev/null)
if [ -n "$dirty" ] && [ "$FORCE" -eq 0 ]; then
  echo "✗ The working tree has uncommitted changes; the run would switch branches over them:" >&2
  printf '%s\n' "$dirty" | head -10 | sed 's/^/    /' >&2
  echo "  Commit or stash them, or pass --force to start anyway." >&2
  exit 1
fi

# ---- warnings: worth knowing, not worth refusing -------------------------------------------
branch=$(git branch --show-current 2>/dev/null)
[ "$branch" != "main" ] && echo "! On branch '${branch:-detached}', not main. The first cycle will finish or leave it as goal.md says."
if command -v pmset >/dev/null 2>&1 && ! pmset -g batt 2>/dev/null | grep -q "AC Power"; then
  echo "! On battery. caffeinate cannot hold off system sleep without AC power: plug the Mac in."
fi
free=$(memory_pressure 2>/dev/null | awk -F': *' '/free percentage/ { gsub(/%/, "", $2); print $2 }')
if [ -n "$free" ] && [ "$free" -lt 40 ]; then
  echo "! Only ${free}% of memory is free. Quit the browser and other large apps before a long window (docs/AUTOPILOT.md, Memory)."
fi

# ---- launch ----------------------------------------------------------------------------------
# A session started from inside Claude Code inherits that session's variables (CLAUDECODE, the
# session id, its messaging socket, the launcher's routing, the settings' ANTHROPIC_MODEL). Each
# cycle must start as clean as one typed into Terminal, so drop them; nothing in the shell profile
# sets any of these. CLAUDE_NATIVE_BIN is the one Claude variable the run reads, so it stays.
for v in $(compgen -e); do
  case "$v" in
    CLAUDE_NATIVE_BIN) ;;
    CLAUDECODE|CLAUDE_*|ANTHROPIC_*) unset "$v" ;;
  esac
done

args=()
if [ "$RESUME" -eq 0 ]; then args=("$DURATION" --fresh); fi

[ -f "$STATE/console.log" ] && mv -f "$STATE/console.log" "$STATE/console.prev.log"
# perl forks, and the child becomes a new session leader (setsid) before running the loop: no
# controlling terminal, so closing the window or the Claude Code session sends it no hangup, and
# its process groups are its own. macOS has no setsid command; perl ships with it.
perl -MPOSIX -e 'my $p = fork() // die "fork: $!"; exit 0 if $p; POSIX::setsid() or die "setsid: $!"; exec @ARGV or die "exec: $!"' \
  bash "$ROOT/scripts/autopilot.sh" ${args[@]+"${args[@]}"} > "$STATE/console.log" 2>&1 < /dev/null

# Confirm it came up: the loop writes its pid file within a second, then checks its binary and
# logs its window. Wait for the first cycle line or an exit, up to 30 s.
started=""
for _ in $(seq 1 30); do
  sleep 1
  pid=$(tr -dc '0-9' 2>/dev/null < "$STATE/pid")
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    started="$pid"
    grep -q "Cycle [0-9]* starting" "$STATE/console.log" 2>/dev/null && break
  elif [ -n "$started" ] || grep -q . "$STATE/console.log" 2>/dev/null; then
    break
  fi
done

pid=$(tr -dc '0-9' 2>/dev/null < "$STATE/pid")
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
  echo "✓ Autopilot running (pid $pid), detached: closing this window or session does not stop it."
  sed -n '/run window/,$p' "$STATE/console.log" | head -6 | sed 's/^/  /'
  echo "  Check: scripts/autopilot-status.sh   Stop: scripts/autopilot-stop.sh [--now]"
else
  echo "✗ The autopilot exited during startup. Its output (.autopilot/console.log):" >&2
  tail -20 "$STATE/console.log" 2>/dev/null | sed 's/^/  /' >&2
  exit 1
fi
