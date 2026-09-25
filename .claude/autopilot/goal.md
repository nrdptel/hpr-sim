Autopilot cycle for hpr-sim: ship the current milestone. First run `git fetch origin && git switch main && git pull --ff-only`, unless an unfinished branch from an earlier cycle needs finishing: then finish that branch, or close its PR, before anything else. Read CLAUDE.md (binding), then docs/STATUS.md and docs/ROADMAP.md, and whichever of docs/VISION.md, docs/ARCHITECTURE.md and docs/VALIDATION.md the milestone needs. Use subagents to keep this context lean. Use dynamic workflows (multi-agent orchestration, medium size) when work really fans out: validation sweeps over many cases, multi-angle reviews, porting several formats or models in parallel.

This goal is MET only when all of the following are shown in this conversation:
(1) Every "done when" bullet of the milestone that STATUS.md named as current when this session started is demonstrated by real command output. If the milestone is too big for one session, split it in ROADMAP.md into increments with their own "done when" bullets; then shipping the first increment satisfies this item.
(2) The local gate from CLAUDE.md passes (fmt, clippy -D warnings, tests, docs, plus the wasm/validate/deny steps once they exist).
(3) The physics-reviewer and/or code-reviewer subagent has reviewed the diff, and its blocking findings are fixed.
(4) The work is merged into main through a squash-merged PR whose checks are green on macOS, Windows and Linux (show `gh pr checks` and the merge).
(5) On main, ROADMAP.md has the milestone (or increment) checked off, and STATUS.md names the next current milestone with a short handoff.

The goal is also MET, as a clean stop, when either:
(a) fewer than 45 minutes remain before the Unix time in .autopilot/deadline (compare with `date +%s`), the work in progress is committed and pushed to a branch with a draft PR, and STATUS.md (in that PR) says exactly where to resume; or
(b) the milestone is truly blocked on Neer, the blocker is recorded under "Needs Neer" in STATUS.md, the milestone is marked [blocked] in ROADMAP.md, and those doc changes are merged through a PR. If more than 45 minutes remain, continue with the next unblocked milestone before stopping, unless every remaining milestone is blocked.

The goal is IMPOSSIBLE only if the repository or toolchain is broken in a way that needs a human (for example, git or gh auth failure). Record that in STATUS.md if you can.

This session is headless: ending your turn ends the session and kills whatever is still running in the background (a gate, a CI watch, a reviewer), and a scheduled wakeup never fires. So never end a turn to wait. Wait in the foreground instead: run the gate with `scripts/gate.sh`, watch CI with `gh pr checks <n> --watch --interval 30 >/dev/null 2>&1; gh pr checks <n>` (Bash timeout 600000), and launch reviewers together in one message with `run_in_background: false`. Don't poll `ListAgents`, `gh pr checks` or output files, and don't wait with `sleep`, `ScheduleWakeup` or `Monitor`: each poll re-reads the whole context.

Never ask questions: decide, record the decision, and continue. Never weaken tests, tolerances, references or "done when" criteria to satisfy this goal. Never leave AI-attribution text anywhere. Keep going until the goal is met; don't stop to summarize.
