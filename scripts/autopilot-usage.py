#!/usr/bin/env python3
"""Tokens and list-price cost of every Claude Code session run in this repository.

Usage:
  scripts/autopilot-usage.py              # the report
  scripts/autopilot-usage.py --days 7     # only the last 7 days
  scripts/autopilot-usage.py --summary    # one line (scripts/autopilot-status.sh prints it)
  scripts/autopilot-usage.py --ingest     # update the ledger quietly (the autopilot does this)
  scripts/autopilot-usage.py --json       # the ledger's session records

Where the numbers come from. Claude Code keeps a transcript of every session, and of every
subagent it starts, under ~/.claude/projects/<this repository's path>/, with each API call's token
counts. It deletes them after `cleanupPeriodDays` (30 by default), and the autopilot prunes its own
cycle logs, so each run folds what it finds into .autopilot/usage-ledger.json and the history
outlives both. The ledger stays on this machine: .autopilot/ is gitignored.

What the cost means. Tokens are priced at the API's list prices (PRICES below). A Max plan bills a
flat monthly fee instead, but its limits are drawn down by the same tokens, so the list price is a
fair measure of how much of the plan the work used. Sessions whose calls went to another API
(message ids that aren't the first-party `msg_` kind) are counted too, and reported apart, because
that service bills them. Checked against Claude Code's own figure for autopilot cycle 78: $20.03
here against $20.18 there. The small gap is the goal checker's calls, which no transcript records.
"""

from __future__ import annotations

import argparse
import datetime as dt
import glob
import gzip
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LEDGER = os.path.join(ROOT, ".autopilot", "usage-ledger.json")
CYCLE_LOGS = os.path.join(ROOT, ".autopilot", "logs")
TRANSCRIPTS = os.path.join(os.path.expanduser("~"), ".claude", "projects")

# USD per million tokens: input, output, cache read, 5-minute cache write, 1-hour cache write.
# From https://platform.claude.com/docs/en/about-claude/pricing, read 2026-09-25. A model missing
# here is counted in tokens and reported as unpriced, never guessed.
PRICES = {
    "claude-fable-5-1": (10, 50, 0.25, 12.5, 20),
    "claude-opus-5-5": (4, 20, 0.20, 5, 8),
    "claude-opus-5": (5, 25, 0.50, 6.25, 10),
    "claude-sonnet-5": (2, 10, 0.20, 2.5, 4),
    "claude-haiku-4-5-20251001": (1, 5, 0.10, 1.25, 2),
}
FIELDS = ("input", "output", "cache_read", "cache_write_5m", "cache_write_1h")


def project_dirs() -> list[str]:
    # Claude Code names a project's folder after its path with every other character made a dash.
    # The prefix match also takes in worktrees made under the repository.
    encoded = re.sub(r"[^A-Za-z0-9]", "-", ROOT)
    return sorted(d for d in glob.glob(os.path.join(TRANSCRIPTS, encoded + "*")) if os.path.isdir(d))


def when(stamp: str | None) -> dt.datetime | None:
    if not stamp:
        return None
    try:
        return dt.datetime.fromisoformat(stamp.replace("Z", "+00:00")).astimezone()
    except ValueError:
        return None


def load_ledger() -> dict:
    try:
        with open(LEDGER, encoding="utf-8") as fh:
            ledger = json.load(fh)
    except (OSError, ValueError):
        ledger = {}
    ledger.setdefault("sessions", {})
    ledger.setdefault("cycles", {})
    ledger.setdefault("cycle_logs_read", [])
    return ledger


def save_ledger(ledger: dict) -> None:
    os.makedirs(os.path.dirname(LEDGER), exist_ok=True)
    tmp = LEDGER + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(ledger, fh, indent=1, sort_keys=True)
    os.replace(tmp, LEDGER)


def map_cycles(ledger: dict) -> None:
    """Record which session each autopilot cycle ran, from the start of its log."""
    seen = set(ledger["cycle_logs_read"])
    for path in sorted(glob.glob(os.path.join(CYCLE_LOGS, "cycle-*.jsonl*"))):
        name = os.path.basename(path).replace(".gz", "")
        match = re.match(r"cycle-(\d+)-", name)
        if not match or name in seen:
            continue
        opener = gzip.open if path.endswith(".gz") else open
        session = None
        try:
            with opener(path, "rt", encoding="utf-8", errors="replace") as fh:
                for i, line in enumerate(fh):
                    if i > 200:
                        break
                    try:
                        session = json.loads(line).get("session_id")
                    except ValueError:
                        continue
                    if session:
                        break
        except OSError:
            continue
        if session:
            ledger["cycles"][session] = int(match.group(1))
        # A log still being written may not name its session yet; look again next time.
        if session or not path.endswith(".jsonl"):
            ledger["cycle_logs_read"].append(name)


def read_calls(path: str, calls: dict, meta: dict | None) -> None:
    """Fold one transcript's API calls into `calls`, keyed so a call split over lines counts once."""
    try:
        fh = open(path, encoding="utf-8", errors="replace")
    except OSError:
        return
    with fh:
        for line in fh:
            if '"assistant"' not in line:
                continue
            try:
                ev = json.loads(line)
            except ValueError:
                continue
            if ev.get("type") != "assistant":
                continue
            msg = ev.get("message") or {}
            model = msg.get("model")
            if not model or model == "<synthetic>":
                continue
            usage = msg.get("usage") or {}
            key = (msg.get("id"), ev.get("requestId"), path if msg.get("id") is None else "")
            calls[key] = (model, usage, msg.get("id") or "")
            if meta is not None:
                stamp = when(ev.get("timestamp"))
                if stamp:
                    meta["start"] = min(meta.get("start") or stamp, stamp)
                    meta["end"] = max(meta.get("end") or stamp, stamp)
                context = sum(usage.get(k) or 0 for k in
                              ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"))
                meta["peak"] = max(meta.get("peak", 0), context)


def summarize(session: str, files: list[str]) -> dict | None:
    calls: dict = {}
    meta: dict = {}
    read_calls(files[0], calls, meta)
    for sub in files[1:]:
        read_calls(sub, calls, None)
    if not calls:
        return None
    models: dict = defaultdict(lambda: dict.fromkeys(FIELDS, 0))
    other_api = False
    for model, usage, mid in calls.values():
        tally = models[model]
        split = usage.get("cache_creation") or {}
        written = usage.get("cache_creation_input_tokens") or 0
        one_hour = split.get("ephemeral_1h_input_tokens") or 0
        tally["input"] += usage.get("input_tokens") or 0
        tally["output"] += usage.get("output_tokens") or 0
        tally["cache_read"] += usage.get("cache_read_input_tokens") or 0
        tally["cache_write_1h"] += one_hour
        tally["cache_write_5m"] += max(0, written - one_hour)
        if mid and not mid.startswith("msg_"):
            other_api = True
    cost, unpriced = 0.0, []
    for model, tally in models.items():
        price = PRICES.get(model)
        if price is None:
            unpriced.append(model)
            tally["cost_usd"] = None
            continue
        tally["cost_usd"] = round(sum(tally[f] * p for f, p in zip(FIELDS, price)) / 1e6, 4)
        cost += tally["cost_usd"]
    return {
        "session": session,
        "start": meta["start"].isoformat() if meta.get("start") else None,
        "end": meta["end"].isoformat() if meta.get("end") else None,
        "peak_context": meta.get("peak", 0),
        "models": dict(models),
        "cost_usd": round(cost, 4),
        "unpriced_models": unpriced,
        "route": "other-api" if other_api else "subscription",
        "calls": len(calls),
    }


def ingest(ledger: dict) -> None:
    map_cycles(ledger)
    for folder in project_dirs():
        for main in glob.glob(os.path.join(folder, "*.jsonl")):
            session = os.path.basename(main)[: -len(".jsonl")]
            files = [main] + sorted(glob.glob(os.path.join(folder, session, "**", "*.jsonl"), recursive=True))
            stamp = [[os.path.getsize(f), int(os.path.getmtime(f))] for f in files if os.path.exists(f)]
            old = ledger["sessions"].get(session)
            if old and old.get("fingerprint") == stamp:
                continue
            record = summarize(session, files)
            if record is None:
                continue
            record["fingerprint"] = stamp
            ledger["sessions"][session] = record
    # Label every record by what ran it, including records whose transcripts are gone.
    for session, record in ledger["sessions"].items():
        cycle = ledger["cycles"].get(session)
        record["cycle"] = cycle
        record["kind"] = "autopilot" if cycle is not None else "interactive"


def merged_prs() -> list[dt.datetime] | None:
    try:
        out = subprocess.run(
            ["gh", "pr", "list", "--state", "merged", "--limit", "1000", "--json", "mergedAt"],
            cwd=ROOT, capture_output=True, text=True, timeout=60,
        )
        if out.returncode != 0:
            return None
        return [t for t in (when(p.get("mergedAt")) for p in json.loads(out.stdout)) if t]
    except (OSError, ValueError, subprocess.SubprocessError):
        return None


def money(x: float) -> str:
    return f"${x:,.2f}" if x < 100 else f"${x:,.0f}"


def tokens(n: float) -> str:
    for unit, size in (("B", 1e9), ("M", 1e6), ("k", 1e3)):
        if n >= size:
            return f"{n / size:.1f}{unit}"
    return str(int(n))


def report(ledger: dict, days: int | None, cycles_shown: int, use_gh: bool) -> None:
    records = [r for r in ledger["sessions"].values() if r.get("start")]
    if days:
        cutoff = dt.datetime.now().astimezone() - dt.timedelta(days=days)
        records = [r for r in records if when(r["end"]) >= cutoff]
    if not records:
        print("No sessions recorded yet.")
        return
    records.sort(key=lambda r: r["start"])
    first, last = when(records[0]["start"]), when(records[-1]["end"])
    prs = merged_prs() if use_gh else None
    if prs is not None:
        prs = [p for p in prs if first <= p <= last + dt.timedelta(minutes=5)]

    total = Counter()
    by_model: Counter = Counter()
    for r in records:
        total[r["kind"]] += r["cost_usd"]
        total[r["kind"] + "_n"] += 1
        for model, t in r["models"].items():
            for f in FIELDS:
                total[f] += t[f]
            by_model[model] += t["cost_usd"] or 0
    cost = total["autopilot"] + total["interactive"]
    print(f"hpr-sim: Claude Code usage {first:%Y-%m-%d} to {last:%Y-%m-%d}, at API list prices "
          "(a Max plan bills a flat fee; its limits count the same tokens)")
    print()
    print(f"  Total        {money(cost)} over {len(records)} sessions: "
          f"autopilot {money(total['autopilot'])} ({int(total['autopilot_n'])} cycles), "
          f"interactive {money(total['interactive'])} ({int(total['interactive_n'])} sessions)")
    print(f"  Tokens       cache reads {tokens(total['cache_read'])}, cache writes "
          f"{tokens(total['cache_write_5m'] + total['cache_write_1h'])}, uncached input "
          f"{tokens(total['input'])}, output {tokens(total['output'])}")
    if prs is not None:
        per = money(cost / len(prs)) if prs else "n/a"
        print(f"  PRs merged   {len(prs)}, so {per} per merged PR")
    other = [r for r in records if r["route"] == "other-api"]
    if other:
        print(f"  Other API    {len(other)} session(s), {money(sum(r['cost_usd'] for r in other))} of the "
              "total: billed by that service, not the plan")
    unpriced = sorted({m for r in records for m in r.get("unpriced_models", [])})
    if unpriced:
        print(f"  Unpriced     {', '.join(unpriced)}: tokens counted, cost left out (add to PRICES)")

    print("\n  By day        autopilot  interactive      total   output" + ("   PRs" if prs is not None else ""))
    days_seen: dict = defaultdict(Counter)
    for r in records:
        day = when(r["start"]).date()
        days_seen[day][r["kind"]] += r["cost_usd"]
        days_seen[day]["output"] += sum(t["output"] for t in r["models"].values())
    pr_days = Counter(p.date() for p in prs) if prs is not None else Counter()
    for day in sorted(days_seen):
        d = days_seen[day]
        line = (f"  {day:%a %m-%d}  {money(d['autopilot']):>10} {money(d['interactive']):>12} "
                f"{money(d['autopilot'] + d['interactive']):>10} {tokens(d['output']):>8}")
        if prs is not None:
            line += f" {pr_days.get(day, 0):>5}"
        print(line)

    print("\n  By model")
    for model, c in by_model.most_common():
        print(f"    {model:28s} {money(c):>10}  {100 * c / cost if cost else 0:5.1f}%")

    cycles = [r for r in records if r["kind"] == "autopilot"][-cycles_shown:]
    if cycles:
        print(f"\n  Last {len(cycles)} autopilot cycles")
        print("    cycle  started       min      cost   reads  output   peak ctx" + ("  PRs" if prs is not None else ""))
        for r in cycles:
            s, e = when(r["start"]), when(r["end"])
            reads = sum(t["cache_read"] for t in r["models"].values())
            out = sum(t["output"] for t in r["models"].values())
            flag = " *" if r["route"] == "other-api" else ""
            line = (f"    {r['cycle']:>5}  {s:%m-%d %H:%M}  {(e - s).total_seconds() / 60:5.0f} "
                    f"{money(r['cost_usd']):>9} {tokens(reads):>7} {tokens(out):>7} {tokens(r['peak_context']):>10}")
            if prs is not None:
                line += f"  {sum(1 for p in prs if s <= p <= e + dt.timedelta(minutes=5)):>3}"
            print(line + flag)
        if any(r["route"] == "other-api" for r in cycles):
            print("    * ran on another API")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--days", type=int, help="only sessions that ended in the last N days")
    ap.add_argument("--cycles", type=int, default=10, help="autopilot cycles to list (default 10)")
    ap.add_argument("--no-gh", action="store_true", help="skip the merged-PR counts (no network)")
    ap.add_argument("--ingest", action="store_true", help="update the ledger and print nothing")
    ap.add_argument("--summary", action="store_true", help="print one line")
    ap.add_argument("--json", action="store_true", help="print the ledger's session records")
    args = ap.parse_args()

    ledger = load_ledger()
    ingest(ledger)
    save_ledger(ledger)
    if args.ingest:
        return 0
    if args.json:
        json.dump(sorted(ledger["sessions"].values(), key=lambda r: r.get("start") or ""), sys.stdout, indent=1)
        print()
        return 0
    if args.summary:
        records = list(ledger["sessions"].values())
        auto = sum(r["cost_usd"] for r in records if r["kind"] == "autopilot")
        inter = sum(r["cost_usd"] for r in records if r["kind"] == "interactive")
        print(f"{money(auto + inter)} at list prices so far: autopilot {money(auto)} over "
              f"{sum(1 for r in records if r['kind'] == 'autopilot')} cycles, interactive {money(inter)} "
              "(scripts/autopilot-usage.py for the breakdown)")
        return 0
    report(ledger, args.days, args.cycles, not args.no_gh)
    return 0


if __name__ == "__main__":
    sys.exit(main())
