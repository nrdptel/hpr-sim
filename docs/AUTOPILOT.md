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
5. Memory: quit your browser and shut down any virtual machine before a long window. On a 16 GB
   Mac what you leave open is the difference between a run that finishes and one that stalls; see
   [Memory](#memory).

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

The run shares the Mac's memory with everything else that is open. On a 16 GB machine that margin
is thin, and when it runs out macOS suspends applications and shows "Your system has run out of
application memory". That dialog waits for a click, so a run left overnight sits paused until
someone clicks it — which is what happened on 2026-09-19, about 14 hours into a window. The steps
below were added afterwards and **have not yet been through a full unattended window**. They
reduce what the run holds; they do not guarantee a 16 GB machine will not run out.

**Before you start, close things.** This is the larger half. On 2026-09-19 a web browser, the
Claude desktop app and a virtual machine held about 7 GB between them. Quit what you are not
using and shut down any VM before leaving a long window unattended. At startup the run logs the
largest processes holding 0.5 GB or more, up to six of them, so the baseline is visible before
you walk away.

**The build is the smaller half.** Building and running the whole test suite peaks at 2.58 GB —
[Performance](perf.md) has the measurements and `scripts/build-memory.sh` repeats them. The run
exports `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6` for its cycles, which takes about a third
off that peak for a few seconds per run. `HPR_CARGO_JOBS=10 scripts/autopilot.sh 48` gives the
cores back. Both are set in the script rather than in the repository's cargo configuration so
they do not follow the project into CI, where the hosted runners have fewer cores than this Mac
and a fixed six would start more jobs than there are cores to run them.

Those two together account for roughly 9.5 GB of the 16. The rest is the session process itself,
which nothing has measured yet — the per-cycle line below is there to find out.

**Leftovers are cleaned up.** Each cycle runs in its own process group, and the group is killed
when the cycle ends, after a normal finish as well as after a watchdog kill. Without that, a build
or a CI watch the session left running would survive into every cycle that follows and go on
holding memory. A descendant that deliberately detaches itself into a new group escapes this, so
it reduces leftovers rather than eliminating them.

**Reading it afterwards.** Every cycle appends a memory line to `.autopilot/runs.log`, like this
made-up example:

```
Cycle 12 memory (sampled every 5 s): peak group RSS 3.41 GB, least 12% spare, worst pressure
level 2, most swap 1830 MB.
```

- **Peak group RSS** is the resident set size — the memory actually held — summed over the
  session and everything it started. Summing double-counts pages those processes share, so read
  it as a trend from cycle to cycle rather than an exact total.
- **Least spare** is the lowest percentage of memory macOS reported as available. This is the
  figure the system itself acts on; counting free pages instead would read near-empty on a
  perfectly healthy Mac.
- **Worst pressure level** is 1 normal, 2 warning, 4 critical. Anything above 1 means macOS was
  already under strain during that cycle.
- **Most swap** is how much swap was in use at the worst sample.

Everything is sampled every 5 seconds, so a shorter spike can pass between two samples: read them
as a floor, not a maximum. If pressure reaches warning, or spare memory drops to 15% or less, the
run says so once in `runs.log` and keeps going.

**Transcripts are pruned.** At the start of each cycle the newest 20 stream-json transcripts in
`.autopilot/logs/` are left readable, older ones are gzipped, and gzipped ones beyond the newest
60 are **deleted**. The environment variable `HPR_KEEP_LOGS` sets the 20; the 60 is three times
it. A 48-hour run can exceed 60 cycles, so copy anything you want to keep out of
`.autopilot/logs/` before starting one.

## Where the project lives

If iCloud Drive's "Desktop & Documents Folders" is on, everything in `~/Documents` syncs to iCloud.
That includes `target/` (gigabytes of build output), `refs/` (the private fixtures) and `.git`.
Git repositories in iCloud can get corrupted, and optimized storage can evict files mid-build.
Either turn that option off, or move the project, for example to `~/dev/hpr-sim`, before
starting.
