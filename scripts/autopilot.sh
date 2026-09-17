#!/usr/bin/env bash
# hpr-sim autopilot: runs Claude Code unattended for a fixed number of hours.
#
# Each cycle is a fresh headless session (`claude -p "/goal ..."`) that ships one roadmap milestone
# and hands off through docs/STATUS.md. Fresh sessions keep each context clean; the repo is the
# memory.
#
# The loop is built to survive:
#   - usage limits: the session waits them out in place (CLAUDE_CODE_RETRY_WATCHDOG), and the
#     loop also polls if a cycle exits on one
#   - API or network outages: long in-session retries, then capped backoff between cycles
#   - hung sessions: a stall watchdog that understands retry waits
# It keeps the Mac awake with caffeinate. Only sign-in/account problems, a permission-mode
# fallback, the deadline, or a STOP file end it early.
#
# Usage:
#   scripts/autopilot.sh            # 48-hour run (reuses an unexpired deadline if one exists)
#   scripts/autopilot.sh 24         # 24-hour run
#   scripts/autopilot.sh 48 --fresh # ignore any existing deadline and start a new 48-hour window
#
# Stop:     touch .autopilot/STOP   (graceful: the current cycle finishes first)
#           Ctrl+C                  (immediate: the current cycle is cut off; its commits are kept)
# Watch:    scripts/autopilot-status.sh   |   tail -f .autopilot/runs.log
#
# Environment overrides: HPR_MODEL (default claude-opus-5), HPR_EFFORT (default xhigh),
#   HPR_CYCLE_MAX_HOURS (default 10), HPR_STALL_MINUTES (default 120), HPR_LIMIT_POLL_MINUTES (default 10)

set -uo pipefail

HOURS="48"
FRESH=0
for arg in "$@"; do
  case "$arg" in
    --fresh) FRESH=1 ;;
    ''|*[!0-9]*) echo "usage: $0 [hours] [--fresh]" >&2; exit 64 ;;
    *) HOURS="$arg" ;;
  esac
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

STATE="$ROOT/.autopilot"
LOGS="$STATE/logs"
RUNLOG="$STATE/runs.log"
mkdir -p "$LOGS"

MODEL="${HPR_MODEL:-claude-opus-5}"
EFFORT="${HPR_EFFORT:-xhigh}"
CYCLE_MAX=$(( ${HPR_CYCLE_MAX_HOURS:-10} * 3600 ))
STALL_MAX=$(( ${HPR_STALL_MINUTES:-120} * 60 ))
LIMIT_POLL=$(( ${HPR_LIMIT_POLL_MINUTES:-10} * 60 ))
WRAPUP=2700   # don't start a cycle with less than 45 minutes left (matches CLAUDE.md)
GOAL_FILE="$ROOT/.claude/autopilot/goal.md"
SETTINGS_FILE="$ROOT/.claude/autopilot/settings.json"

log() { printf '%s  %s\n' "$(date '+%Y-%m-%d %H:%M:%S')" "$*" | tee -a "$RUNLOG"; }
notify() {
  if command -v osascript >/dev/null 2>&1; then
    osascript -e "display notification \"${1//\"/\\\"}\" with title \"hpr-sim autopilot\"" >/dev/null 2>&1 || true
  fi
}
filesize() { wc -c < "$1" 2>/dev/null | tr -d ' ' || echo 0; }
fmt_time() { date -r "$1" 2>/dev/null || date -d "@$1"; }

# Sleep in 30 s slices so STOP and the deadline are still honored. $1 = seconds.
nap() {
  local waited=0
  while [ "$waited" -lt "$1" ] && [ ! -f "$STATE/STOP" ] && [ "$(date +%s)" -lt "$deadline" ]; do
    sleep 30; waited=$(( waited + 30 ))
  done
}

for f in "$GOAL_FILE" "$SETTINGS_FILE"; do
  [ -f "$f" ] || { echo "missing $f" >&2; exit 1; }
done
command -v claude >/dev/null 2>&1 || { echo "claude CLI not found on PATH" >&2; exit 1; }
command -v python3 >/dev/null 2>&1 || { echo "python3 not found (needed for log parsing and the hooks)" >&2; exit 1; }
if [ -z "$(git config user.email 2>/dev/null)" ]; then
  echo "git identity not set; run scripts/preflight.sh first" >&2; exit 1
fi

# ---- deadline -------------------------------------------------------------------------------
now=$(date +%s)
deadline=""
if [ "$FRESH" -eq 0 ] && [ -f "$STATE/deadline" ]; then
  existing=$(tr -dc '0-9' < "$STATE/deadline")
  if [ -n "$existing" ] && [ "$existing" -gt "$now" ]; then
    deadline="$existing"
    log "Resuming the existing run window. Deadline: $(fmt_time "$deadline")"
  fi
fi
if [ -z "$deadline" ]; then
  deadline=$(( now + HOURS * 3600 ))
  echo "$deadline" > "$STATE/deadline"
  log "New ${HOURS}h run window. Deadline: $(fmt_time "$deadline")"
fi
rm -f "$STATE/STOP"

# ---- keep the Mac awake -------------------------------------------------------------------------
if command -v caffeinate >/dev/null 2>&1; then
  caffeinate -ims -w $$ &
  log "caffeinate is holding off idle and system sleep (system-sleep prevention needs AC power)."
fi

child=""
on_signal() {
  log "Interrupted: stopping now. Work is saved in git; rerun scripts/autopilot.sh to resume the same window."
  if [ -n "$child" ]; then kill "$child" 2>/dev/null; sleep 5; kill -9 "$child" 2>/dev/null; fi
  exit 130
}
trap on_signal INT TERM HUP
trap 'if [ -n "$child" ]; then kill "$child" 2>/dev/null; fi' EXIT

GOAL_TEXT="$(cat "$GOAL_FILE")"
SETTINGS_JSON="$(cat "$SETTINGS_FILE")"

# Parse a cycle's stream-json log. Prints one line:
#   <kind>|<is_error>|<turns>|<permission_mode>|<snippet>
# kind is: ok, limit, auth, error, or none (no result event).
analyze() {
  python3 - "$1" "$2" <<'PY' 2>/dev/null || echo "none|?|?|?|(could not parse log)"
import json, re, sys
out, err = sys.argv[1], sys.argv[2]
res, mode = None, "?"
with open(out, encoding="utf-8", errors="replace") as fh:
    for line in fh:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            ev = json.loads(line)
        except Exception:
            continue
        if ev.get("type") == "system" and ev.get("subtype") == "init":
            mode = ev.get("permissionMode") or ev.get("permission_mode") or mode
        elif ev.get("type") == "result":
            res = ev
try:
    err_text = open(err, encoding="utf-8", errors="replace").read()[-4000:]
except OSError:
    err_text = ""
text = ""
if res:
    parts = [res.get("result"), res.get("error"), res.get("errors"), res.get("subtype")]
    text = " ".join(p if isinstance(p, str) else json.dumps(p) for p in parts if p)
blob = (text + "\n" + err_text).lower()
if re.search(r"authentication_failed|not logged in|invalid api key|oauth_org_not_allowed|account_on_hold|billing_error|please run /login", blob):
    kind = "auth"
elif re.search(r"hit your (session|weekly|opus|sonnet) limit|usage limit", blob):
    kind = "limit"
elif res is None:
    kind = "none"
elif res.get("is_error"):
    kind = "error"
else:
    kind = "ok"
snippet = (text or err_text.strip()[-160:] or "(no output)").replace("\n", " ").replace("|", "/")[:160]
print(f"{kind}|{res.get('is_error') if res else '?'}|{res.get('num_turns') if res else '?'}|{mode}|{snippet}")
PY
}

# Seconds of silence to tolerate right now: longer while the session is waiting out an API retry.
stall_allowance() {
  python3 - "$1" "$STALL_MAX" <<'PY' 2>/dev/null || echo "$STALL_MAX"
import json, sys
path, base = sys.argv[1], int(sys.argv[2])
try:
    with open(path, "rb") as fh:
        fh.seek(0, 2)
        size = fh.tell()
        fh.seek(max(0, size - 65536))
        lines = [l for l in fh.read().decode("utf-8", "replace").splitlines() if l.strip().startswith("{")]
    ev = json.loads(lines[-1]) if lines else {}
except Exception:
    ev = {}
if ev.get("type") == "system" and ev.get("subtype") == "api_retry":
    print(base + int((ev.get("retry_delay_ms") or 0) / 1000) + 1800)
else:
    print(base)
PY
}

cycle=0
fails=0
quick=0
if [ -f "$STATE/cycle" ]; then cycle=$(tr -dc '0-9' < "$STATE/cycle"); cycle=${cycle:-0}; fi

while :; do
  now=$(date +%s)
  if [ "$now" -ge "$deadline" ]; then log "Deadline reached. Autopilot finished."; notify "Run finished (deadline reached)."; break; fi
  if [ $(( deadline - now )) -lt "$WRAPUP" ]; then log "Less than 45 min left; not starting another cycle. Autopilot finished."; notify "Run finished."; break; fi
  if [ -f "$STATE/STOP" ]; then log "STOP file found. Autopilot stopped."; notify "Autopilot stopped."; break; fi

  cycle=$(( cycle + 1 ))
  echo "$cycle" > "$STATE/cycle"
  stamp=$(date +%Y%m%d-%H%M%S)
  out="$LOGS/cycle-$(printf '%03d' "$cycle")-$stamp.jsonl"
  err="${out%.jsonl}.err"
  left_min=$(( (deadline - now) / 60 ))
  log "Cycle $cycle starting (${left_min} min left in the window). Log: ${out#"$ROOT"/}"

  start=$(date +%s)
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
  CLAUDE_CODE_RETRY_WATCHDOG=1 \
    claude -p "/goal $GOAL_TEXT" \
      --model "$MODEL" \
      --effort "$EFFORT" \
      --permission-mode auto \
      --settings "$SETTINGS_JSON" \
      --output-format stream-json --verbose \
      --name "hpr-autopilot-$cycle" \
      > "$out" 2> "$err" < /dev/null &
  child=$!

  # Watchdog: cycle wall-clock cap, stall detection, hard stop 30 min after the deadline.
  last_size=0; last_change=$start; killed=""
  while kill -0 "$child" 2>/dev/null; do
    sleep 30
    t=$(date +%s)
    size=$(filesize "$out")
    if [ "$size" != "$last_size" ]; then last_size=$size; last_change=$t; fi
    if [ $(( t - start )) -ge "$CYCLE_MAX" ]; then killed="cycle exceeded ${HPR_CYCLE_MAX_HOURS:-10}h"; fi
    if [ $(( t - last_change )) -ge "$STALL_MAX" ]; then
      allow=$(stall_allowance "$out")
      if [ $(( t - last_change )) -ge "$allow" ]; then killed="no output for $(( (t - last_change) / 60 )) min"; fi
    fi
    if [ "$t" -ge $(( deadline + 1800 )) ]; then killed="30 min past the deadline"; fi
    if [ -n "$killed" ]; then
      log "Watchdog: stopping cycle $cycle ($killed)."
      kill "$child" 2>/dev/null; sleep 20; kill -9 "$child" 2>/dev/null
      break
    fi
  done
  wait "$child" 2>/dev/null; rc=$?
  child=""
  dur=$(( $(date +%s) - start ))
  IFS='|' read -r kind is_error turns pmode snippet <<< "$(analyze "$out" "$err")"
  log "Cycle $cycle ended: rc=$rc, ${dur}s, turns=$turns, mode=$pmode, result=$kind (error=$is_error). $snippet"

  if [ "$pmode" != "?" ] && [ "$pmode" != "auto" ]; then
    log "The session ran in '$pmode' mode instead of auto (auto mode unavailable?). Stopping; headless runs can't edit files without it."
    notify "Stopped: auto mode wasn't available."
    break
  fi
  if [ "$kind" = "auth" ]; then
    log "Sign-in or account problem. Stopping; run 'claude' once to sign in, then restart the autopilot."
    notify "Stopped: sign-in or account problem."
    break
  fi
  if [ "$kind" = "limit" ]; then
    reset_hint=$(grep -Eoi "resets [^\"|]{1,40}" "$err" 2>/dev/null | head -1)
    [ -z "$reset_hint" ] && reset_hint=$(tail -n 5 "$out" 2>/dev/null | grep -Eoi "resets [^\"|]{1,40}" | head -1)
    log "Usage limit (${reset_hint:-no reset time found}). Checking again every $(( LIMIT_POLL / 60 )) min."
    notify "Usage limit reached; waiting. ${reset_hint}"
    nap "$LIMIT_POLL"
    fails=0; quick=0
    continue
  fi

  if [ -n "$killed" ] || [ "$rc" -ne 0 ] || [ "$kind" != "ok" ]; then
    fails=$(( fails + 1 ))
    # Keep trying until the deadline, with backoff capped at 30 min. Repeated failures only notify.
    exp=$(( fails - 1 )); [ "$exp" -gt 5 ] && exp=5
    backoff=$(( 60 * (2 ** exp) )); [ "$backoff" -gt 1800 ] && backoff=1800
    log "Failure $fails in a row; retrying in $(( backoff / 60 )) min. Latest logs: ${out#"$ROOT"/}"
    [ "$fails" -eq 3 ] && notify "Three failed cycles in a row; still retrying."
    nap "$backoff"
    continue
  fi
  fails=0

  if [ "$dur" -lt 300 ]; then
    quick=$(( quick + 1 ))
    if [ "$quick" -ge 6 ]; then
      log "Six cycles in a row finished in under 5 minutes; the roadmap is probably done or fully blocked. Stopping. Check docs/STATUS.md."
      notify "Stopped: nothing left to do, or everything is blocked."
      break
    elif [ "$quick" -ge 3 ]; then
      log "Several very short cycles in a row; pausing 30 min before trying again."
      nap 1800
      continue
    fi
  else
    quick=0
  fi
  sleep 20
done

log "Autopilot exited after $cycle cycle(s)."
