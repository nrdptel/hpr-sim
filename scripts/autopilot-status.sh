#!/usr/bin/env bash
# One-screen summary of an autopilot run. Safe to run at any time; it changes nothing.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1
STATE=.autopilot

if [ -f "$STATE/deadline" ]; then
  d=$(tr -dc '0-9' < "$STATE/deadline"); now=$(date +%s)
  left=$(( (d - now) / 60 ))
  when=$(date -r "$d" 2>/dev/null || date -d "@$d")
  if [ "$left" -gt 0 ]; then echo "Window: $(( left / 60 ))h $(( left % 60 ))m left (ends $when)"; else echo "Window ended $when"; fi
fi
# The run writes its own pid file (scripts/autopilot.sh); a stale one names a dead process.
pid=$(tr -dc '0-9' 2>/dev/null < "$STATE/pid")
if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null \
  && ps -o command= -p "$pid" 2>/dev/null | grep -q 'autopilot\.sh'; then
  echo "Autopilot: RUNNING (pid $pid)"
  running=1
else
  echo "Autopilot: not running"
  running=0
fi
[ -f "$STATE/STOP" ] && echo "STOP requested: the autopilot stops after the current cycle"
latest=$(ls -t "$STATE"/logs/*.jsonl 2>/dev/null | head -1)
if [ "$running" -eq 1 ]; then
  cyc=$(grep 'Cycle [0-9]* starting' "$STATE/runs.log" 2>/dev/null | tail -1)
  if [ -n "$cyc" ]; then
    started=$(date -j -f '%Y-%m-%d %H:%M:%S' "${cyc:0:19}" +%s 2>/dev/null || date -d "${cyc:0:19}" +%s 2>/dev/null)
    n=$(printf '%s' "$cyc" | sed -E 's/.*Cycle ([0-9]+) starting.*/\1/')
    [ -n "$started" ] && echo "Current cycle: $n, running for $(( ($(date +%s) - started) / 60 )) min"
  fi
  # The transcript grows with every event, retries included, so its age is the run's pulse. The
  # loop's own stall watchdog steps in at HPR_STALL_MINUTES (default 120).
  if [ -n "$latest" ]; then
    echo "Last activity: $(( ($(date +%s) - $(stat -f %m "$latest" 2>/dev/null || stat -c %Y "$latest")) / 60 )) min ago"
  fi
  echo "Checkout is on branch: $(git branch --show-current 2>/dev/null || echo '?')"
fi
echo
echo "== Recent autopilot events"
tail -n 12 "$STATE/runs.log" 2>/dev/null || echo "(no runs yet)"
echo
if [ -n "$latest" ]; then
  echo "== Latest thing Claude said (${latest#"$ROOT"/})"
  python3 - "$latest" <<'PY' 2>/dev/null
import json, sys
last = ""
actions = []
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    try:
        ev = json.loads(line)
    except Exception:
        continue
    if ev.get("type") == "assistant":
        for block in (ev.get("message") or {}).get("content") or []:
            if block.get("type") == "text" and block.get("text", "").strip():
                last = block["text"].strip()
            elif block.get("type") == "tool_use":
                inp = block.get("input") or {}
                what = (inp.get("description") or inp.get("file_path") or inp.get("command")
                        or inp.get("prompt") or "")
                actions.append(f"{block.get('name', '?')}: {' '.join(str(what).split())[:100]}")
print(last[-800:] if last else "(nothing yet)")
if actions:
    print("\n== Its last actions (oldest first)")
    for a in actions[-6:]:
        print("  " + a)
PY
  echo
fi
echo "== Status (docs/STATUS.md, 'Now' section)"
sed -n '/^## Now/,/^## Handoff/p' docs/STATUS.md 2>/dev/null | sed '$d'
echo "== Recent commits on main"
git log origin/main -8 --format='%h %ad %s' --date=format:'%m-%d %H:%M' 2>/dev/null || git log -8 --oneline
echo
echo "== Open PRs"
gh pr list --state open 2>/dev/null || echo "(gh unavailable)"
echo
echo "== Usage (at API list prices)"
python3 scripts/autopilot-usage.py --summary 2>/dev/null || echo "(scripts/autopilot-usage.py failed)"
echo
echo "== Needs Neer"
sed -n '/^## Needs Neer/,/^## /p' docs/STATUS.md 2>/dev/null | sed '1d;$d'
