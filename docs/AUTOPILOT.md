# Running the autopilot

## One-time setup (about 10 minutes)

1. Open **Terminal** (not the VS Code chat panel) in this folder:
   `cd ~/Documents/local-projects/hpr-sim`
2. Run `claude update`, then `claude` once. Accept the "trust this folder" prompt, confirm
   `/status` shows Opus 5.5, then type `/exit`.
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
6. For the OpenRocket oracle (only needed from M2.2 on): `brew install openjdk@17`. OpenRocket
   24.12 refuses newer runtimes, and the formula is keg-only, so it does not become your default
   `java` and does not need to be.

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
CLAUDE_CODE_AUTO_COMPACT_WINDOW=400000 BASH_DEFAULT_TIMEOUT_MS=1200000 BASH_MAX_TIMEOUT_MS=2400000 \
  caffeinate -ims claude --model claude-opus-5-5 --effort high --permission-mode bypassPermissions \
  --remote-control hpr-sim --settings "$(cat .claude/autopilot/settings.json)"
```

Then paste:

```text
/goal Work through docs/ROADMAP.md one milestone at a time, exactly as CLAUDE.md and .claude/autopilot/goal.md describe for a single milestone (read both first). After each milestone is merged, re-read docs/STATUS.md and start the next one. The goal is met only when fewer than 45 minutes remain before the time in .autopilot/deadline and the final handoff is pushed as described in goal.md, or when every remaining milestone is blocked on Neer. Never ask questions.
```

In `/config`, make sure **Continue automatically at usage limit** is on.

## Costs and limits

- Everything draws on your Max 20x plan's 5-hour and weekly allowances. Check with `/usage` in any
  Claude Code session.
- When a cycle stops on a usage limit, the script checks again every 10 minutes until it resets.
  A weekly limit can outlast the window.
- Each cycle's start line in `.autopilot/runs.log` names its model, effort and compact window, and
  each cycle writes a `usage` line when it ends:
  - its cost at API list prices (a Max plan doesn't bill this, but the same tokens use up its
    limits);
  - its cache reads, cache writes and output tokens;
  - its largest single context, and how many times it compacted.

  Use these lines to judge any change to the settings below.

### Where the tokens went

This was measured over cycles 1 to 77 (2026-09-17 to 2026-09-25). Cycles 65 to 75 are left out
because their logs report tokens differently.

| Share of list-price cost | What it paid for |
| --- | --- |
| 70% | Cache reads: every API call re-reads the whole conversation so far |
| 16% | Output, thinking included |
| 13% | Cache writes |

Three things drove it:

- **Context size.** A session compacted only near its 1M-token window, so long cycles averaged
  about 480k tokens of context on every call. That context was roughly a third tool results, a
  third tool inputs (edits and patch scripts), and a third the model's own thinking, which stays in
  context.
- **Polling.** About 1,600 calls, roughly 8% of the spend, only checked on something: `gh pr checks`,
  `ListAgents`, output files. Each one re-read the whole context.
- **Waiting in the background.** About 20 of 70 cycles ended their turn to wait for CI or a
  background gate. In a headless session that ends the session and kills the work, so the next
  cycle started over.

### What the run does about it

These settings date from 2026-09-25.

| Setting | Default | Why |
| --- | --- | --- |
| `HPR_COMPACT_WINDOW` | `400000` | Treats the window as 400k tokens for compaction. Compaction starts 33k below the window, so at about 367k instead of about 967k (both read from `/context` on Opus 5.5). Replaying the logged cycles' context sizes at 367k cuts the main session's context re-reads by 49%, at the price of 54 compactions across those cycles instead of 4. `1000000` restores the old behavior. |
| `HPR_EFFORT` | `high` | On Opus 5.5, Anthropic measured `xhigh` at about 1.4 points above `high` on SWE-bench Pro, for 2.5 times the cost. Its advice is to keep `xhigh` for work where you've measured a gain ([effort levels](https://platform.claude.com/docs/en/build-with-claude/effort), [cost and intelligence](https://platform.claude.com/docs/en/about-claude/models/optimizing-for-cost-and-intelligence), [prompting Opus 5.5](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/prompting-claude-opus-5-5)). The physics, code and validation reviewers stay at `xhigh` in `.claude/agents/`, so the checking runs at the higher level. `HPR_EFFORT=xhigh` restores the old setting. |
| `HPR_MODEL` | `claude-opus-5-5` | Until 2026-09-25 the script defaulted to Opus 5. Its `--model` flag overrides `.claude/settings.json`, so cycles ran Opus 5 whatever that file said. Interactive sessions still take their model and `effortLevel` (`xhigh`) from that file. |
| `HPR_CLAUDE_BIN` | the binary `claude` wraps | On this Mac `claude` is a launcher that, after any Max usage limit, sends every session to a different API until the weekly reset. For the run, that route cached poorly, ran 3 to 6 times slower per call, failed outright twice (cycles 65 to 75), and let another service see the private fixtures. So the run calls the wrapped binary (`CLAUDE_NATIVE_BIN`, by default `~/.local/bin/claude-native`) and waits out a limit itself; interactive sessions keep the launcher. The run's first log line names the binary. |
| Waiting | in the foreground | The gate runs with `scripts/gate.sh`, about 4 minutes on this Mac with warm caches. CI is waited on with `scripts/ci-wait.sh`, and reviewers launch with `run_in_background: false`. A Bash call with no timeout of its own is moved to the background after 120 s, so the script raises the default to 20 minutes (`BASH_DEFAULT_TIMEOUT_MS`). No session ends mid-wait, and nothing polls. |

None of this has been through a full window yet, so trust the `usage` lines over this page.

If quality slips, raise `HPR_COMPACT_WINDOW` or `HPR_EFFORT` and say so in `STATUS.md`. Two signs
to watch for:

- more reviewer findings per milestone;
- a session that loses track of its work after a compaction.

## Memory

The run shares the Mac's memory with everything else that is open. On a 16 GB machine that margin
is thin, and when it runs out macOS suspends applications and shows "Your system has run out of
application memory". That dialog waits for a click, so a run left overnight sits paused until
someone clicks it — which is what happened on 2026-09-19, about 14 hours into a window. The steps
below were added afterwards and **have not yet been through a full unattended window**. They
reduce what the run holds; they do not guarantee a 16 GB machine will not run out.

**Before you start, close things.** This is the larger half. The dialog on 2026-09-19 listed
five applications holding 6.33 GB between them: 2.58 GB the Claude desktop app, 2.36 GB Safari,
1.18 GB Chrome, and a little over 0.2 GB for Terminal and Finder. Quit what you are not using
before leaving a long window unattended.

Note what that dialog does **not** show. It lists applications only, so `cargo`, `rustc`, the
session itself and any virtual machine were all absent from it — which is why the numbers in it
do not add up to a full machine. At startup the run logs the largest processes holding 0.5 GB or
more, up to six of them, counting everything rather than applications alone, so the baseline you
see there is the honest one.

**The build is the smaller half.** Building and running the whole test suite peaks at 2.58 GB —
[Performance](perf.md) has the measurements and `scripts/build-memory.sh` repeats them. The run
exports `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6` for its cycles, which takes about a third
off that peak for a few seconds per run. `HPR_CARGO_JOBS=10 scripts/autopilot.sh 48` gives the
cores back. Both are set in the script rather than in the repository's cargo configuration so
they do not follow the project into CI, where the hosted runners have fewer cores than this Mac
and a fixed six would start more jobs than there are cores to run them.

That is about 8.9 GB of the 16 accounted for. The rest is the kernel, anything the dialog left
out, and the session process itself, which nothing has measured yet — the per-cycle line below is
there to find out.

**Leftovers are cleaned up.** Every command a cycle runs — each `cargo`, each `rustc` — is given
a process group of its own, separate from the cycle's. So the run notes which groups the cycle's
commands are using as it samples, and when the cycle ends it stops the cycle's own group *and*
every group it noted, after a normal finish as well as after a watchdog kill. Without that, a
build or a CI watch the session left running would survive into every cycle that follows and go
on holding memory.

Noting the groups is what makes this work after the fact: a process group id survives its parent
dying, whereas walking the process tree would not, because an abandoned build is reparented the
moment the session exits. A group whose processes are older than the cycle is left alone and said
so in the log, since over a long run a group id can be reused by something unrelated.

**Reading it afterwards.** Every cycle appends a memory line to `.autopilot/runs.log`, like this
made-up example:

```
Cycle 12 memory (sampled every 5 s): peak RSS of the session and its commands 3.41 GB,
least 12% spare, worst pressure level 2, most swap 1830 MB.
```

- **Peak RSS of the session and its commands** is the resident set size — the memory actually
  held — summed over the session and every process descended from it, builds included. Summing
  double-counts pages those processes share, so read it as a trend from cycle to cycle rather
  than an exact total.
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
