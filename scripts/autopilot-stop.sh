#!/usr/bin/env bash
# Stop an autopilot run, however it was started.
#
#   scripts/autopilot-stop.sh         # graceful: the current cycle finishes its milestone first
#   scripts/autopilot-stop.sh --now   # immediate: the cycle is cut off; committed and pushed work stays
#
# --now sends the loop SIGTERM. Its trap stops the cycle's session and every process group the
# cycle used (builds, test runs), exactly as Ctrl+C does in the window it runs in.
set -uo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
STATE=.autopilot

pid=$(tr -dc '0-9' 2>/dev/null < "$STATE/pid")
if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null \
  || ! ps -o command= -p "$pid" 2>/dev/null | grep -q 'autopilot\.sh'; then
  echo "No autopilot run is going."
  exit 0
fi

case "${1:-}" in
  --now)
    kill -TERM "$pid"
    for _ in $(seq 1 30); do kill -0 "$pid" 2>/dev/null || break; sleep 1; done
    if kill -0 "$pid" 2>/dev/null; then
      echo "✗ The autopilot (pid $pid) is still running 30 s after SIGTERM. Try: kill -KILL $pid" >&2
      exit 1
    fi
    echo "✓ Autopilot stopped now. Work already committed and pushed is kept; a branch may be left mid-milestone."
    ;;
  '')
    touch "$STATE/STOP"
    echo "✓ STOP requested: the run stops once the current cycle ends (it may take a while to finish its milestone)."
    echo "  To stop at once instead: scripts/autopilot-stop.sh --now"
    ;;
  *) echo "usage: $0 [--now]" >&2; exit 64 ;;
esac
