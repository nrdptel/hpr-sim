# Running the autopilot

## One-time setup (about 10 minutes)

1. Open **Terminal** (not the VS Code chat panel) in this folder:
   `cd ~/Documents/local-projects/hpr-sim`
2. Run `claude update`, then `claude` once. Accept the "trust this folder" prompt, confirm
   `/status` shows Opus 5, then type `/exit`.
3. Run `bash scripts/preflight.sh`. It:
   - Moves the Claude Code project config from `setup/` into `.claude/` and `.cargo/` (read it
     first if you like; `setup/README.md` lists each file).
   - Checks and installs tools (Rust, gh, jq, uv).
   - Sets this repo's git identity.
   - Makes the first commit.
   - Creates the public repo `nrdptel/hpr-sim`.
   - Clones `fusionspace-loft` and `loft-fixtures` into the gitignored `refs/`.

   Fix any ✗ it reports, then run it again until it passes.
4. Power: plug the Mac in and keep the lid open. Optionally, turn off automatic updates and
   restarts for two days (System Settings → General → Software Update → Automatic updates).

## Start

```bash
scripts/autopilot.sh 48
```

Leave that Terminal window open; closing it stops the run. The script:

- Keeps the Mac awake with `caffeinate`.
- Starts a fresh `claude -p "/goal …"` session per milestone.
- Logs everything under `.autopilot/`.
- Waits out usage limits and restarts after crashes.
- Stops at the 48-hour mark.

If it stops for any reason, running it again resumes the same window. Add `--fresh` to start a
new one.

If you might close the window by accident, run it in the background instead:

```bash
nohup scripts/autopilot.sh 48 > .autopilot/console.log 2>&1 &
```

To stop a background run, `touch .autopilot/STOP` (graceful) or `pkill -f scripts/autopilot.sh`
(immediate).

## Check in (optional)

- `scripts/autopilot-status.sh` shows the time left, recent cycles, the last thing Claude said,
  the current milestone, open PRs and "Needs Neer".
- `tail -f .autopilot/runs.log` streams one line per event.
- The PRs on https://github.com/nrdptel/hpr-sim/pulls are the real record of the work.
- `docs/STATUS.md` → **Needs Neer** lists decisions only you can make. The run keeps going
  without them.

## Stop

- **Graceful** (the current milestone finishes first): `touch .autopilot/STOP`
- **Immediate:** Ctrl+C in the autopilot window. Work already committed and pushed is kept.

## Steer it without stopping

Don't edit files in this folder while the run is going; the agent's next commit could sweep your
edits in. Instead, edit on GitHub (the pencil icon, committing straight to `main`):

- Reorder or add milestones in `docs/ROADMAP.md`.
- Add a note in `docs/STATUS.md`.
- Change the rules in `CLAUDE.md`.

Each cycle pulls `main` before it starts.

## Plan B: one interactive session you can watch from your phone

Use this instead of the script if you'd rather watch it in the Claude app. It's less robust: one
long session with many context compactions, and it doesn't wait out weekly limits.

```bash
mkdir -p .autopilot && echo $(( $(date +%s) + 48*3600 )) > .autopilot/deadline
caffeinate -ims claude --model claude-opus-5 --effort xhigh --permission-mode bypassPermissions \
  --remote-control hpr-sim --settings "$(cat .claude/autopilot/settings.json)"
```

Then paste:

```text
/goal Work through docs/ROADMAP.md one milestone at a time, exactly as CLAUDE.md and .claude/autopilot/goal.md describe for a single milestone (read both first). After each milestone is merged, re-read docs/STATUS.md and start the next one. The goal is met only when fewer than 45 minutes remain before the time in .autopilot/deadline and the final handoff is pushed as described in goal.md, or when every remaining milestone is blocked on Neer. Never ask questions.
```

In `/config`, make sure **Continue automatically at usage limit** is on.

## Costs and limits

- Everything draws on your Max 20x plan's 5-hour and weekly allowances.
- Opus 5 at xhigh, with review subagents and occasional workflows, will probably hit the 5-hour
  limit several times in 48 hours. The script waits those out.
- A weekly limit pauses the run until it resets, which may be after the window ends.
- Check with `/usage` in any Claude Code session.
- To spend less, run with `HPR_EFFORT=high scripts/autopilot.sh 48`.

## Memory

The run shares the Mac's memory with everything else that is open. On a 16 GB machine the margin
is thin, and when it runs out macOS suspends applications and shows "Your system has run out of
application memory". That dialog waits for a click, so a run left overnight can sit paused for
hours. Two things fill the memory, and the build is the smaller one.

**What the build needs.** Measured over three cold `cargo test --workspace --all-features
--no-run` builds on a 10-core, 16 GB machine (full numbers in [Performance](perf.md)):

| cargo jobs | peak build memory | wall time |
|---|---|---|
| 10, one per core | 2.42 GB | 12.0 s |
| 6, what the run uses | 1.72 GB | 13.0 s |

So a full cold build peaks under 2.5 GB. The script exports `CARGO_BUILD_JOBS=6` for its cycles,
trading about a second per build for 0.7 GB of headroom — well under a percent of a multi-hour
cycle. Set `HPR_CARGO_JOBS` to take the cores back. This is not in `.cargo/config.toml` on purpose:
CI runners have fewer cores, where a fixed 6 would oversubscribe them.

**What everything else needs.** Usually the larger half. When the run ran out of memory on
2026-09-19, a browser, the desktop app and a virtual machine held about 7 GB between them. Quit
what you are not using and shut down any VM before leaving a long window unattended. The script
logs whatever is holding 0.5 GB or more when it starts, so the baseline is visible before you walk
away.

**Leftovers are cleaned up.** Each cycle runs in its own process group, and the group is killed
when the cycle ends — after a normal finish as well as after a watchdog kill. Without that, a build
or a CI watch the session left running would survive into every cycle that follows and keep its
memory.

**Reading it afterwards.** Every cycle appends a memory line to `.autopilot/runs.log`:

```
Cycle 12 memory: peak group RSS 3.41 GB, least free 512 MB, most swap 2048 MB.
```

Peak group RSS sums the whole cycle — the session and every build it started — so it double-counts
shared pages; read it as a trend from cycle to cycle rather than an exact figure. Least free and
most swap are system-wide and are the honest numbers. If swap passes 4 GB mid-cycle the script says
so once, and keeps going. Older transcripts are gzipped, keeping the newest 20 readable
(`HPR_KEEP_LOGS`).

## Where the project lives

If iCloud Drive's "Desktop & Documents Folders" is on, everything in `~/Documents` syncs to iCloud.
That includes `target/` (gigabytes of build output), `refs/` (the private fixtures) and `.git`.
Git repositories in iCloud can get corrupted, and optimized storage can evict files mid-build.
Either turn that option off, or move the project, for example to `~/dev/hpr-sim`, before
starting.
