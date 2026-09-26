---
name: autopilot
description: Start, check on, or stop an hpr-sim autopilot run from an interactive Claude Code session. Use when Neer asks to start or run the autopilot for some amount of time ("start autopilot for 8 hours", "run overnight until 7am"), asks how the run is going, what it is working on, or its status, or asks to stop it.
argument-hint: "[start <duration> | status | stop [--now]]"
---

# Autopilot controller

Neer is in this session. This session **controls** the run; it is not part of it. The run itself
is `scripts/autopilot.sh`, a loop of fresh headless sessions, one milestone each, described in
`docs/AUTOPILOT.md`. It runs detached, so it keeps going if this session or its terminal window
closes, and any later session can check on it or stop it.

While a run is going, **don't edit files, switch branches, commit, fetch or build in this
checkout**: the run's cycles are switching branches and committing here. Reading files and
running the status script are fine.

## Start

1. Work out the duration from what Neer said. The script takes `8`, `8h`, `90m` or `2h30m`. For
   "until 7am", compute the minutes from `date` and round down to whole minutes. Nothing under
   1h: no cycle starts with under 45 minutes left.
2. Run it, in the foreground:

   ```bash
   scripts/autopilot-start.sh <duration>
   ```

   - `--resume` continues an unexpired window instead (only when Neer asks to carry on the
     previous one).
   - It refuses if a run is already going, or if the working tree has uncommitted changes. Show
     Neer the refusal and the fix it names. Pass `--force` only if Neer says so. Never stash or
     discard his changes yourself.
   - `!` lines are warnings (battery, memory, branch). Pass them on in one line each.
3. Reply in two or three lines: that it is running, when the window ends, and that he can close
   this session or ask here for status any time. Don't start any other work.

## Status

Run `scripts/autopilot-status.sh` (it changes nothing) and answer in about six short lines. Lead
with the answer, then the detail:

- Running or not, and the time left in the window.
- The current cycle, how long it has run, and what it is doing now: its milestone (the STATUS
  "Now" section and the checkout's branch) and its latest message and actions, in plain words.
- What has shipped during this window: the `Recent commits on main` whose times fall inside it.
- Anything under **Needs Neer**.
- Anything wrong. Look for these:
  - **Last activity** over 30 minutes ago. The cycle may be waiting out a usage limit (look for
    `limit` in `tail -20 .autopilot/runs.log`), or stalled. The loop's own watchdog kills a
    cycle after 120 minutes without activity, so say which it looks like, and don't act.
  - A cycle that ended with `rc` not 0, or `result=` other than `ok`, in the recent events.
  - `Autopilot: not running` while the window still has time. Show the end of
    `.autopilot/console.log` and `.autopilot/runs.log`, and say why it stopped if they show it.
    Offer `scripts/autopilot-start.sh --resume`.

If Neer asks for more, `tail -40 .autopilot/runs.log` and the latest transcript under
`.autopilot/logs/` hold the detail; read their tails with `tail`, never the whole file.

## Stop

- `scripts/autopilot-stop.sh` stops gracefully: the current cycle finishes its milestone first,
  which can take an hour or more.
- `scripts/autopilot-stop.sh --now` stops at once: the cycle is cut off, and committed and pushed
  work is kept. Its branch may be left mid-milestone. The next run's first cycle finishes it.

Use `--now` only when Neer says now, immediately, or similar. Then confirm with the status
script.
