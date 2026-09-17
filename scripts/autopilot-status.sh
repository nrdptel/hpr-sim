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
if pgrep -f "scripts/autopilot.sh" >/dev/null 2>&1; then echo "Autopilot: RUNNING"; else echo "Autopilot: not running"; fi
[ -f "$STATE/STOP" ] && echo "STOP requested: the autopilot stops after the current cycle"
echo
echo "== Recent autopilot events"
tail -n 12 "$STATE/runs.log" 2>/dev/null || echo "(no runs yet)"
echo
latest=$(ls -t "$STATE"/logs/*.jsonl 2>/dev/null | head -1)
if [ -n "$latest" ]; then
  echo "== Latest thing Claude said (${latest#"$ROOT"/})"
  python3 - "$latest" <<'PY' 2>/dev/null
import json, sys
last = ""
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    try:
        ev = json.loads(line)
    except Exception:
        continue
    if ev.get("type") == "assistant":
        for block in (ev.get("message") or {}).get("content") or []:
            if block.get("type") == "text" and block.get("text", "").strip():
                last = block["text"].strip()
print(last[-800:] if last else "(nothing yet)")
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
echo "== Needs Neer"
sed -n '/^## Needs Neer/,/^## /p' docs/STATUS.md 2>/dev/null | sed '1d;$d'
