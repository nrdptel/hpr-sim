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
# Environment overrides: HPR_MODEL (default claude-opus-5-5), HPR_EFFORT (default high; the reviewer
#   subagents set their own in .claude/agents/),
#   HPR_COMPACT_WINDOW (default 400000; the context size at which a session compacts, see below),
#   HPR_CLAUDE_BIN (the Claude Code binary to run; see below),
#   HPR_PERMISSION_MODE (default bypassPermissions; set it to auto for the classifier-checked mode),
#   HPR_CYCLE_MAX_HOURS (default 10), HPR_STALL_MINUTES (default 120), HPR_LIMIT_POLL_MINUTES (default 10),
#   HPR_KEEP_LOGS (default 20 cycle transcripts kept uncompressed; archives beyond 3x that are
#     deleted), HPR_CARGO_JOBS (default 6; caps both cargo jobs and test threads during the run)

set -uo pipefail
# Job control, so each cycle's `claude` is forked as its own process-group leader. The whole group
# is killed when the cycle ends (see reap), which is the only way to be sure cargo, rustc, test
# binaries and anything the session left running in the background go with it: a process group id
# is inherited by every descendant and survives reparenting to launchd.
set -m

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

MODEL="${HPR_MODEL:-claude-opus-5-5}"
# high, not xhigh: on Opus 5.5, Anthropic measured xhigh at about 1.4 points more than high on
# SWE-bench Pro for 2.5 times the cost, and advises keeping xhigh for work where a gain has been
# measured. The adversarial reviewers (physics, code, validation) stay at xhigh in their own
# definitions, so the checking is done at the higher level. docs/AUTOPILOT.md has the sources.
EFFORT="${HPR_EFFORT:-high}"
PERM_MODE="${HPR_PERMISSION_MODE:-bypassPermissions}"
CYCLE_MAX=$(( ${HPR_CYCLE_MAX_HOURS:-10} * 3600 ))
STALL_MAX=$(( ${HPR_STALL_MINUTES:-120} * 60 ))
LIMIT_POLL=$(( ${HPR_LIMIT_POLL_MINUTES:-10} * 60 ))
KEEP_LOGS="${HPR_KEEP_LOGS:-20}"   # cycle transcripts to keep uncompressed; older ones are gzipped
# Validate before the value ever reaches $(( )): 0 would gzip every transcript and then delete
# every archive, a non-numeric value would abort pruning under `set -u`, and arithmetic evaluation
# of an unchecked string is a way to run a command.
case "$KEEP_LOGS" in ''|*[!0-9]*) KEEP_LOGS=20 ;; esac
[ "$KEEP_LOGS" -lt 1 ] && KEEP_LOGS=1

# Cap the parallelism of the run's builds and tests. Measured with scripts/build-memory.sh over
# three cold `cargo test --workspace --all-features --no-run` builds on this 10-core, 16 GB
# machine: one job per core peaked at 2.55 GB in 11.7 s, six jobs at 1.62 GB in 13.7 s. That buys
# about 0.9 GB of headroom for two seconds a build, which is worth it when the rest of the machine
# is busy. RUST_TEST_THREADS is capped with it because CARGO_BUILD_JOBS does not reach test
# execution: each test binary otherwise runs one thread per core. Both are set here rather than in
# .cargo/config.toml on purpose, so they do not follow the project into CI, where the hosted
# runners have fewer cores and a fixed six would start more jobs than there are cores.
export CARGO_BUILD_JOBS="${HPR_CARGO_JOBS:-6}"
export RUST_TEST_THREADS="${HPR_CARGO_JOBS:-6}"
# Treat the context window as 400k tokens for compaction, rather than the model's 1M. Every API
# call re-reads the whole context, so its size, not the work, set most of a cycle's cost: over
# cycles 1 to 77, cache reads were 70% of token spend, and long cycles averaged about 480k tokens
# of context per call. Compaction triggers at the window less a 33k buffer, so about 367k here
# against about 967k by default (both read from `/context`). Replaying the logged cycles' own
# per-call context sizes at 367k cuts the main session's context re-reads by 49%. Claude Code reads
# CLAUDE_CODE_AUTO_COMPACT_WINDOW (100000 to 1000000 tokens; the autoCompactWindow setting does the
# same). docs/AUTOPILOT.md has the measurement.
COMPACT_WINDOW="${HPR_COMPACT_WINDOW:-400000}"
case "$COMPACT_WINDOW" in ''|*[!0-9]*) COMPACT_WINDOW=400000 ;; esac
[ "$COMPACT_WINDOW" -lt 100000 ] && COMPACT_WINDOW=100000
[ "$COMPACT_WINDOW" -gt 1000000 ] && COMPACT_WINDOW=1000000
export CLAUDE_CODE_AUTO_COMPACT_WINDOW="$COMPACT_WINDOW"
# A Bash call with no timeout of its own gets 120 s and is then moved to the background, where it
# dies with a session that ends its turn. The gate takes about 4 minutes warm and more cold, so
# give foreground commands 20 minutes by default and allow up to 40.
export BASH_DEFAULT_TIMEOUT_MS=1200000
export BASH_MAX_TIMEOUT_MS=2400000
SCRIPT_PGID=$(ps -o pgid= -p $$ 2>/dev/null | tr -d ' ')
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
fmt_gb() { awk -v kb="${1:-0}" 'BEGIN { printf "%.2f", kb / 1048576 }'; }

# Spare memory as the percentage macOS itself reports, or -1 if it cannot be read. Free pages are
# not usable here: they exclude inactive, purgeable and compressed memory, so on a healthy 16 GB
# Mac they read near-empty (1.2 GB free while the system reported 75% spare), which would make
# every cycle look like an emergency.
free_pct_now() {
  memory_pressure 2>/dev/null \
    | awk -F': *' '/free percentage/ { gsub(/%/, "", $2); printf "%d", $2; seen = 1 }
                   END { if (!seen) printf "-1" }'
}
# 1 normal, 2 warning, 4 critical: the level macOS acts on when it suspends applications.
pressure_level_now() { sysctl -n kern.memorystatus_vm_pressure_level 2>/dev/null || echo 0; }
swap_mb_now() {
  # vm.swapusage prints "total = 2048.00M  used = 818.88M  free = 1229.12M"; $6 is the used figure.
  # The suffix is read rather than assumed, so a machine that reports G or K is not off by 1024.
  sysctl -n vm.swapusage 2>/dev/null | awk '{
    v = $6; u = substr(v, length(v)); gsub(/[A-Za-z]/, "", v)
    if (u == "G") v *= 1024; else if (u == "K") v /= 1024
    printf "%d", v }' 2>/dev/null || echo 0
}

# Everything below is descendant-scoped rather than group-scoped, because a cycle's real work is
# not in the cycle's process group. Each command the session runs is given a process group of its
# own, so `cargo`, `rustc` and the rest sit outside it: measured on this machine, a cycle that ran
# the whole gate reported 0.66 GB for its group while its builds, which peak far higher, were
# invisible to it. What the commands do stay is descendants of the session, so the process tree
# finds them while the session lives, and the groups recorded from it reach them after it dies.
#
# bash 3.2 (what macOS ships) has no mapfile and no associative arrays; none are used here.

# Resident KB of $1 and every descendant.
descendant_rss_kb() {
  ps -eo pid=,ppid=,rss= 2>/dev/null | awk -v root="${1:-0}" '
    { ppid[$1] = $2; rss[$1] = $3; pid[n++] = $1 }
    END { desc[root] = 1; ch = 1
          while (ch) { ch = 0
            for (i = 0; i < n; i++) { p = pid[i]; if (!desc[p] && desc[ppid[p]]) { desc[p] = 1; ch = 1 } } }
          s = 0; for (i = 0; i < n; i++) { p = pid[i]; if (desc[p]) s += rss[p] }
          print s + 0 }'
}
# Distinct process groups among $1's descendants, leaving out group $2 (this script's own).
descendant_pgids() {
  ps -eo pid=,ppid=,pgid= 2>/dev/null | awk -v root="${1:-0}" -v skip="${2:-0}" '
    { ppid[$1] = $2; pgid[$1] = $3; pid[n++] = $1 }
    END { desc[root] = 1; ch = 1
          while (ch) { ch = 0
            for (i = 0; i < n; i++) { p = pid[i]; if (!desc[p] && desc[ppid[p]]) { desc[p] = 1; ch = 1 } } }
          for (i = 0; i < n; i++) { p = pid[i]; if (desc[p] && p != root && pgid[p] != skip) g[pgid[p]] = 1 }
          for (k in g) print k }'
}
group_count() { ps -eo pgid= 2>/dev/null | awk -v g="${1:-}" '$1 == g { n++ } END { print n + 0 }'; }
group_rss_kb() { ps -eo pgid=,rss= 2>/dev/null | awk -v g="${1:-}" '$1 == g { s += $2 } END { print s + 0 }'; }
group_survivors() {
  ps -eo pgid=,rss= 2>/dev/null | awk -v g="${1:-}" '$1 == g { n++; s += $2 }
    END { if (n) printf "%d process%s holding %.2f GB", n, (n == 1 ? "" : "es"), s / 1048576 }'
}
# "3-05:12" / "05:12" / "12" -> seconds. macOS ps accepts etimes but prints nothing, so etime it is.
etime_seconds() {
  awk -v t="${1:-0}" 'BEGIN { d = 0; if (index(t, "-")) { split(t, a, "-"); d = a[1]; t = a[2] }
    m = split(t, b, ":")
    if (m == 3) s = b[1] * 3600 + b[2] * 60 + b[3]; else if (m == 2) s = b[1] * 60 + b[2]; else s = b[1]
    print d * 86400 + s }'
}

# Stop a finished or wedged cycle. $1 is the session's pid, $2 the process group captured when it
# was forked, $3 how many seconds to wait before escalating to KILL.
#
# Killing the group is what catches the cycle's descendants — cargo, rustc, test binaries — since a
# process group id is inherited and survives reparenting to launchd. It is not a guarantee: a
# descendant that calls setsid leaves the group and would have to be found another way. Scoping by
# group is still exact, so nothing outside the cycle is signalled; a sweep by process name would
# kill unrelated builds on the same machine.
#
# With no group (job control not in effect, so the cycle shares this script's own group) only the
# session's own pid is signalled, because killing the shared group would kill the run itself.
reap() {
  local pid="$1" pgid="${2:-}" grace="${3:-5}" waited=0 target left
  [ -n "$pid" ] || return 0
  if [ -n "$pgid" ]; then target="-$pgid"; else target="$pid"; fi
  kill -TERM -- "$target" 2>/dev/null
  while [ "$waited" -lt "$grace" ] && kill -0 -- "$target" 2>/dev/null; do
    sleep 1; waited=$(( waited + 1 ))
  done
  if [ -n "$pgid" ]; then
    left=$(group_survivors "$pgid")
    [ -n "$left" ] && log "Cycle left work behind: $left. Stopping it."
  fi
  kill -KILL -- "$target" 2>/dev/null

  # Then the groups the cycle's own commands ran in, recorded while it was alive. A process group
  # id survives reparenting, so this reaches a build the session abandoned; the process tree no
  # longer would, because an orphan's parent is launchd by now. A group whose processes are older
  # than the cycle is left alone: over a long run a group id can be reused by something unrelated.
  local g oldest e secs age
  age=$(( $(date +%s) - ${cycle_start:-0} ))
  for g in ${cycle_groups:-}; do
    [ -z "$g" ] && continue
    [ "$g" = "$pgid" ] && continue
    [ "$(group_count "$g")" -eq 0 ] && continue
    oldest=0
    for e in $(ps -eo pgid=,etime= 2>/dev/null | awk -v g="$g" '$1 == g { print $2 }'); do
      secs=$(etime_seconds "$e"); [ "${secs:-0}" -gt "$oldest" ] && oldest="$secs"
    done
    if [ "$oldest" -gt "$(( age + 30 ))" ]; then
      log "Leaving process group $g alone: ${oldest}s old, older than the ${age}s cycle, so it is not ours."
      continue
    fi
    log "Cycle left a command running in group $g ($(group_survivors "$g")). Stopping it."
    kill -TERM -- "-$g" 2>/dev/null
    sleep 1
    kill -KILL -- "-$g" 2>/dev/null
  done
  return 0
}

# Sample the cycle's memory into peak_rss_kb / min_free_pct / worst_pressure / peak_swap_mb, and
# record the process groups its commands are running in. Called every few seconds rather than once
# a tick, because a link step's peak can last only seconds. These are still samples, so read them
# as a floor rather than a true maximum. The RSS figure sums the session and every descendant,
# which double-counts pages they share; read it as a trend between cycles. The rest are
# system-wide and include everything else running on the machine.
sample_memory() {
  local rss pct swap level g
  rss=$(descendant_rss_kb "$child")
  # Remember each group the cycle's commands run in, so they can be reaped once it ends.
  for g in $(descendant_pgids "$child" "$SCRIPT_PGID"); do
    case " ${cycle_groups:-} " in *" $g "*) ;; *) cycle_groups="${cycle_groups:-} $g" ;; esac
  done
  [ "${rss:-0}" -gt "$peak_rss_kb" ] && peak_rss_kb="$rss"
  pct=$(free_pct_now)
  [ "${pct:--1}" -ge 0 ] && [ "$pct" -lt "$min_free_pct" ] && min_free_pct="$pct"
  level=$(pressure_level_now)
  [ "${level:-0}" -gt "$worst_pressure" ] && worst_pressure="$level"
  swap=$(swap_mb_now)
  [ "${swap:-0}" -gt "$peak_swap_mb" ] && peak_swap_mb="$swap"
  if [ "$mem_warned" -eq 0 ] && { [ "${level:-0}" -ge 2 ] || { [ "${pct:--1}" -ge 0 ] && [ "$pct" -le 15 ]; }; }; then
    mem_warned=1
    log "Memory is tight: ${pct}% spare, pressure level ${level}, ${swap} MB of swap in use. The cycle keeps going; its memory line has the peak."
  fi
  return 0
}

# Keep the newest KEEP_LOGS transcripts readable, gzip the rest, and delete archives beyond three
# times that many. A long window writes hundreds of MB of stream-json, and free disk is what the
# system swaps into. KEEP_LOGS is validated above, so neither offset can collapse to +1.
prune_logs() {
  local f
  ls -t "$LOGS"/cycle-*.jsonl 2>/dev/null | tail -n +$(( KEEP_LOGS + 1 )) | while read -r f; do
    gzip -f "$f" 2>/dev/null || true
  done
  ls -t "$LOGS"/cycle-*.jsonl.gz 2>/dev/null | tail -n +$(( KEEP_LOGS * 3 + 1 )) | while read -r f; do
    rm -f "$f"
  done
  return 0
}

# What else is holding memory before the run starts. Advisory only: this never touches a process.
memory_baseline() {
  local total_gb hogs
  total_gb=$(awk -v b="$(sysctl -n hw.memsize 2>/dev/null || echo 0)" 'BEGIN { printf "%.0f", b / 1073741824 }')
  log "Memory at start: ${total_gb} GB installed, $(free_pct_now)% spare, $(swap_mb_now) MB swap in use."
  # Summed per application, largest first. awk walks its keys in hash order, so the sort is what
  # makes these the six biggest rather than six arbitrary ones.
  hogs=$(ps -Ao rss=,comm= 2>/dev/null | awk '
      { rss = $1; $1 = ""; name = $0
        sub(/^ +/, "", name); sub(/\.app\/Contents\/.*$/, "", name); sub(/.*\//, "", name)
        total[name] += rss }
      END { for (k in total) if (total[k] > 524288) printf "%d\t%s\n", total[k], k }' \
    | sort -rn | head -6 | awk -F'\t' '{ printf "%s %.1f GB; ", $2, $1 / 1048576 }')
  if [ -n "$hogs" ]; then
    log "Largest holding 0.5 GB or more (up to six): ${hogs%; }"
    log "Browsers and virtual machines left running take memory the build needs; closing them before an unattended window leaves more room."
  fi
  return 0
}

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
# The binary each cycle runs. On this Mac `claude` is a launcher that, once a Max usage limit is
# recorded, sends every session to a different API until the weekly reset. For the unattended run
# that route cached poorly, ran 3 to 6 times slower per call and failed outright twice (cycles 65 to
# 75, docs/AUTOPILOT.md), and it puts the private fixtures in front of another service. So the run
# calls the real binary the launcher wraps, when it exists, and waits out a limit itself (the loop
# below). The launcher's own subscription path clears the same four variables before it runs that
# binary, so an API key or base URL left in the environment cannot reroute a cycle either.
# HPR_CLAUDE_BIN names another binary and skips all of this.
CLAUDE_BIN="${HPR_CLAUDE_BIN:-}"
if [ -z "$CLAUDE_BIN" ]; then
  native="${CLAUDE_NATIVE_BIN:-$HOME/.local/bin/claude-native}"
  if [ -x "$native" ]; then
    CLAUDE_BIN="$native"
    unset ANTHROPIC_API_KEY ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL CLAUDE_SMART_API_MODE
  else
    CLAUDE_BIN="claude"
  fi
fi
command -v "$CLAUDE_BIN" >/dev/null 2>&1 || { echo "Claude Code binary not found: $CLAUDE_BIN" >&2; exit 1; }
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
memory_baseline
log "Running $CLAUDE_BIN ($("$CLAUDE_BIN" --version 2>/dev/null | head -1))."

child=""
child_pgid=""
cycle_groups=""
cycle_start=0
NOT_SAMPLED=101   # a free-percentage sentinel: no real sample can exceed 100
on_signal() {
  log "Interrupted: stopping now. Work is saved in git; rerun scripts/autopilot.sh to resume the same window."
  reap "$child" "$child_pgid" 5
  # Clear it so the EXIT trap does not signal the same group again: by then the pid has been
  # released and could in principle belong to something else.
  child=""; child_pgid=""
  exit 130
}
trap on_signal INT TERM HUP
trap 'reap "$child" "$child_pgid" 2' EXIT

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

# One line of what a cycle spent, from its stream-json log, so a change to the run's settings can be
# judged by numbers rather than impressions. The cost is at API list prices, as the log reports it:
# a Max plan does not bill it, but its limits are drawn down by the same tokens. Cache reads are the
# context each call re-read; peak context is the main session's largest single request.
usage_line() {
  python3 - "$1" <<'PY' 2>/dev/null || echo "usage not readable"
import json, sys
res, peak, compacts, seen, main = None, 0, 0, set(), {"in": 0, "cr": 0, "cw": 0}
with open(sys.argv[1], encoding="utf-8", errors="replace") as fh:
    for line in fh:
        if not line.startswith("{"):
            continue
        try:
            ev = json.loads(line)
        except Exception:
            continue
        t = ev.get("type")
        if t == "result":
            res = ev
        elif t == "system" and ev.get("subtype") == "compact_boundary":
            compacts += 1
            peak = max(peak, (ev.get("compact_metadata") or {}).get("pre_tokens") or 0)
        elif t == "assistant" and ev.get("parent_tool_use_id") is None:
            msg = ev.get("message") or {}
            mid = msg.get("id")
            if mid is not None:
                if mid in seen:
                    continue
                seen.add(mid)
            u = msg.get("usage") or {}
            ins = [u.get(k) or 0 for k in
                   ("input_tokens", "cache_read_input_tokens", "cache_creation_input_tokens")]
            for key, v in zip(("in", "cr", "cw"), ins):
                main[key] += v
            peak = max(peak, sum(ins))
tail = f"peak context {peak/1e3:.0f}k; {compacts} compaction(s)"
if res is None:
    # Cut off by the watchdog, a cap or a crash: only the main session's own calls can be counted.
    print(f"no result event; main session only: cache reads {main['cr']/1e6:.1f}M, "
          f"cache writes {main['cw']/1e6:.2f}M, uncached input {main['in']/1e6:.2f}M; {tail}")
    sys.exit(0)
tot = {"in": 0, "cr": 0, "cw": 0, "out": 0}
for m in (res.get("modelUsage") or {}).values():
    tot["in"] += m.get("inputTokens") or 0
    tot["cr"] += m.get("cacheReadInputTokens") or 0
    tot["cw"] += m.get("cacheCreationInputTokens") or 0
    tot["out"] += m.get("outputTokens") or 0
cost = res.get("total_cost_usd")
cost = f"${cost:.2f}" if isinstance(cost, (int, float)) else "$?"
sub = (res.get("subagent_stats") or {}).get("spawned", "?")
print(f"{cost} at list prices; tokens: cache reads {tot['cr']/1e6:.1f}M, "
      f"cache writes {tot['cw']/1e6:.2f}M, uncached input {tot['in']/1e6:.2f}M, output {tot['out']/1e6:.2f}M; "
      f"{tail}; {sub} subagent(s)")
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

  prune_logs
  cycle=$(( cycle + 1 ))
  echo "$cycle" > "$STATE/cycle"
  stamp=$(date +%Y%m%d-%H%M%S)
  out="$LOGS/cycle-$(printf '%03d' "$cycle")-$stamp.jsonl"
  err="${out%.jsonl}.err"
  left_min=$(( (deadline - now) / 60 ))
  log "Cycle $cycle starting (${left_min} min left in the window; $MODEL, effort $EFFORT, compact window $COMPACT_WINDOW). Log: ${out#"$ROOT"/}"

  start=$(date +%s)
  CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 \
  CLAUDE_CODE_RETRY_WATCHDOG=1 \
    "$CLAUDE_BIN" -p "/goal $GOAL_TEXT" \
      --model "$MODEL" \
      --effort "$EFFORT" \
      --permission-mode "$PERM_MODE" \
      --settings "$SETTINGS_JSON" \
      --output-format stream-json --verbose \
      --name "hpr-autopilot-$cycle" \
      > "$out" 2> "$err" < /dev/null &
  child=$!
  # The group the cycle was forked into. Captured now, while the process is alive, because after
  # `wait` the pid is gone and it could no longer be looked up. It is only used if job control
  # really gave the cycle a group of its own: otherwise it would be this script's own group, and
  # killing that would kill the run.
  child_pgid=$(ps -o pgid= -p "$child" 2>/dev/null | tr -d ' ')
  [ -n "$child_pgid" ] && [ "$child_pgid" != "$SCRIPT_PGID" ] || child_pgid=""

  # Watchdog: cycle wall-clock cap, stall detection, hard stop 30 min after the deadline.
  last_size=0; last_change=$start; killed=""
  peak_rss_kb=0; min_free_pct=$NOT_SAMPLED; worst_pressure=0; peak_swap_mb=-1; mem_warned=0
  cycle_groups=""; cycle_start=$start
  while kill -0 "$child" 2>/dev/null; do
    # Sample every 5 s across the 30 s tick. A link step's peak lasts seconds, so one sample a
    # tick would usually walk straight past the thing these numbers exist to catch.
    for _ in 1 2 3 4 5 6; do
      sample_memory
      sleep 5
      kill -0 "$child" 2>/dev/null || break
    done
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
      reap "$child" "$child_pgid" 20
      break
    fi
  done
  wait "$child" 2>/dev/null; rc=$?
  # Reap even after a clean exit. CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 abandons the session's
  # background tasks rather than draining them, so a build or a CI watch can outlive the session
  # and go on holding memory through every cycle that follows. The grace is generous because
  # anything still running here is mid-work: a push or a build killed outright can leave a lock
  # file behind that the next cycle then trips over.
  reap "$child" "$child_pgid" 20
  child=""; child_pgid=""
  dur=$(( $(date +%s) - start ))
  IFS='|' read -r kind is_error turns pmode snippet <<< "$(analyze "$out" "$err")"
  log "Cycle $cycle ended: rc=$rc, ${dur}s, turns=$turns, mode=$pmode, result=$kind (error=$is_error). $snippet"
  free_note="${min_free_pct}% spare"; [ "$min_free_pct" -eq "$NOT_SAMPLED" ] && free_note="spare not sampled"
  rss_note="$(fmt_gb "$peak_rss_kb") GB"; [ "$peak_rss_kb" -eq 0 ] && rss_note="not sampled"
  swap_note="${peak_swap_mb} MB"; [ "$peak_swap_mb" -lt 0 ] && swap_note="not sampled"
  log "Cycle $cycle memory (sampled every 5 s): peak RSS of the session and its commands ${rss_note}, least ${free_note}, worst pressure level ${worst_pressure}, most swap ${swap_note}."
  log "Cycle $cycle usage: $(usage_line "$out")"
  # Fold the cycle into the usage ledger now: Claude Code deletes transcripts after 30 days, and
  # prune_logs gzips and later deletes this cycle's log.
  python3 "$ROOT/scripts/autopilot-usage.py" --ingest >/dev/null 2>&1 || log "Usage ledger not updated (scripts/autopilot-usage.py failed)."

  if [ "$pmode" != "?" ] && [ "$pmode" != "$PERM_MODE" ]; then
    log "The session ran in '$pmode' mode instead of $PERM_MODE (mode unavailable or disabled by policy?). Stopping; headless runs can't edit files without it."
    notify "Stopped: $PERM_MODE mode wasn't available."
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
