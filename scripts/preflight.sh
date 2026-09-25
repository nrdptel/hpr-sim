#!/usr/bin/env bash
# One-time setup before the first autopilot run. Run it from the project folder on your Mac:
#
#   bash scripts/preflight.sh          # asks before installing or creating anything
#   bash scripts/preflight.sh --yes    # accepts every default
#
# It checks the toolchain, sets this repo's git identity, makes the first commit, creates the
# public GitHub repo nrdptel/hpr-sim, and clones the two reference repos into the gitignored refs/.
# It is safe to run again; every step skips itself when it's already done.

set -uo pipefail
YES=0; [ "${1:-}" = "--yes" ] && YES=1
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

REPO="nrdptel/hpr-sim"
GIT_NAME="Neer Patel"
GIT_EMAIL="135655563+nrdptel@users.noreply.github.com"
MIN_CLAUDE="2.1.269"

ok()   { printf '  \033[32m✓\033[0m %s\n' "$*"; }
warn() { printf '  \033[33m!\033[0m %s\n' "$*"; WARNINGS=$((WARNINGS+1)); }
bad()  { printf '  \033[31m✗\033[0m %s\n' "$*"; PROBLEMS=$((PROBLEMS+1)); }
ask()  {
  [ "$YES" -eq 1 ] && return 0
  local a
  read -r -p "  → $1 [Y/n] " a || return 1   # no input available (EOF): treat as "no"
  [ -z "$a" ] || [ "$a" = "y" ] || [ "$a" = "Y" ]
}
WARNINGS=0; PROBLEMS=0
have() { command -v "$1" >/dev/null 2>&1; }
vergte() { python3 -c 'import sys; v=lambda s: tuple(int(x) for x in s.split(".")); sys.exit(0 if v(sys.argv[1]) >= v(sys.argv[2]) else 1)' "$1" "$2" 2>/dev/null; }

echo "== Tools"
if [ "$(uname -s)" != "Darwin" ]; then warn "Not macOS. The scripts should still work, but caffeinate and notifications won't."; fi

if have brew; then ok "Homebrew"; else warn "Homebrew not found (https://brew.sh). Some installs below need it."; fi

if have claude; then
  cv=$(claude --version 2>/dev/null | grep -Eo '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
  if [ -n "$cv" ] && vergte "$cv" "$MIN_CLAUDE"; then ok "Claude Code $cv"
  else
    warn "Claude Code ${cv:-unknown} is older than $MIN_CLAUDE, the version this kit was written against."
    if ask "Run 'claude update' now?"; then claude update || bad "claude update failed"; fi
  fi
else
  bad "Claude Code CLI not found. Install it: https://code.claude.com/docs/en/setup"
fi

have git && ok "git $(git --version | awk '{print $3}')" || bad "git not found (xcode-select --install)"

if have python3 && python3 -c 'import sys; sys.exit(0 if sys.version_info >= (3, 9) else 1)'; then
  ok "python3 $(python3 -c 'import platform; print(platform.python_version())') (used by the hooks)"
else
  bad "python3 ≥ 3.9 is required by .claude/hooks/guard-bash.py (xcode-select --install, or brew install python)"
fi

for tool in gh jq uv mdbook; do
  if have "$tool"; then ok "$tool"
  elif have brew && ask "Install $tool with Homebrew?"; then brew install "$tool" && ok "$tool installed" || bad "brew install $tool failed"
  else warn "$tool not found (brew install $tool)"; fi
done

if have cargo && have rustup; then ok "Rust $(rustc --version | awk '{print $2}') via rustup"
elif ask "Install Rust with rustup (the official installer from https://rustup.rs)?"; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile default \
    && . "$HOME/.cargo/env" && ok "Rust installed" || bad "rustup install failed"
else bad "Rust not installed"; fi
if have rustup; then rustup target add wasm32-unknown-unknown >/dev/null 2>&1 && ok "wasm32 target"; fi

java_ok=0
if [ -x /usr/libexec/java_home ]; then
  /usr/libexec/java_home -v 17+ >/dev/null 2>&1 && java_ok=1   # avoids macOS's "install Java" pop-up
elif have java && java -version 2>&1 | grep -Eq 'version "(1[7-9]|[2-9][0-9])'; then
  java_ok=1
fi
if [ "$java_ok" -eq 1 ]; then ok "Java 17+ (for the OpenRocket oracle)"
else warn "Java 17+ not found. Claude Code can install it later (brew install openjdk@21); only the OpenRocket oracle needs it."; fi

echo
echo "== GitHub"
if have gh; then
  if gh auth status >/dev/null 2>&1; then
    ok "gh is signed in as $(gh api user --jq .login 2>/dev/null)"
    scopes=$(gh auth status 2>&1 | grep -i 'token scopes' || true)
    if ! printf '%s' "$scopes" | grep -q "workflow"; then
      warn "The gh token lacks the 'workflow' scope, which pushing CI files needs."
      if ask "Run 'gh auth refresh -s workflow' now (opens a browser)?"; then gh auth refresh -h github.com -s workflow || bad "scope refresh failed"; fi
    else ok "gh token has the workflow scope"; fi
    gh auth setup-git >/dev/null 2>&1 && ok "git uses gh credentials for github.com"
  else
    bad "gh isn't signed in. Run: gh auth login -s workflow   then rerun this script."
  fi
fi

echo
echo "== Claude Code project config (setup/ → .claude/ and .cargo/)"
# The kit ships these inert under setup/ so you can read them first. This step puts them in place.
if [ -d setup/dot-claude ]; then
  echo "  setup/ holds the project settings, hooks, review agents and autopilot goal for .claude/,"
  echo "  plus .cargo/config.toml. Read them first if you like (setup/README.md explains each file)."
  if ask "Install them into .claude/ and .cargo/ now (this overwrites existing files with the same names)?"; then
    mkdir -p .claude .cargo \
      && cp -R setup/dot-claude/. .claude/ \
      && cp setup/dot-cargo/config.toml .cargo/config.toml \
      && chmod +x .claude/hooks/* \
      && rm -rf setup \
      && ok "installed .claude/ and .cargo/config.toml (setup/ removed)" \
      || bad "couldn't install the project config from setup/"
  else
    bad "the project config in setup/ isn't installed; the autopilot needs .claude/"
  fi
elif [ -f .claude/settings.json ] && [ -f .claude/autopilot/goal.md ]; then
  ok ".claude/ project config is in place"
else
  bad ".claude/ project config is missing (expected .claude/settings.json and .claude/autopilot/goal.md)"
fi

echo
echo "== Repository"
if [ ! -d .git ]; then
  if ask "Initialize a git repo here (branch main)?"; then git init -b main >/dev/null && ok "git repo created"; fi
else ok "git repo exists"; fi
if [ -d .git ]; then
  git config user.name "$GIT_NAME" && git config user.email "$GIT_EMAIL" && ok "repo identity: $GIT_NAME <$GIT_EMAIL>"
  chmod +x scripts/*.sh .claude/hooks/* 2>/dev/null && ok "scripts and hooks are executable"
  if [ -z "$(git log --oneline -1 2>/dev/null)" ]; then
    if ask "Make the first commit (project brief, roadmap, automation)?"; then
      git add -A && git commit -q -m "Add project brief, architecture, roadmap and automation" && ok "first commit made"
    fi
  else ok "history exists ($(git rev-list --count HEAD) commits)"; fi

  if have gh && gh auth status >/dev/null 2>&1; then
    if gh repo view "$REPO" >/dev/null 2>&1; then
      ok "GitHub repo $REPO exists"
      git remote get-url origin >/dev/null 2>&1 || git remote add origin "https://github.com/$REPO.git"
    elif ask "Create the PUBLIC GitHub repo $REPO and push?"; then
      gh repo create "$REPO" --public --source . --remote origin \
        --description "Open-source 6-DOF flight simulator for hobby and high-power rockets, written in Rust (pre-alpha)" \
        --push && ok "created and pushed https://github.com/$REPO"
    fi
    if git remote get-url origin >/dev/null 2>&1; then
      git fetch -q origin 2>/dev/null
      if ! git rev-parse -q --verify origin/main >/dev/null; then git push -q -u origin main 2>/dev/null; git fetch -q origin 2>/dev/null; fi
      if [ "$(git rev-parse main 2>/dev/null)" = "$(git rev-parse origin/main 2>/dev/null)" ]; then ok "main matches origin/main"
      else warn "local main and origin/main differ; reconcile them (git pull / git push) before starting"; fi
      gh repo edit "$REPO" --delete-branch-on-merge --enable-squash-merge >/dev/null 2>&1 && ok "repo merge settings: squash, delete branch on merge"
    fi
  fi
fi

echo
echo "== Reference repos (gitignored refs/)"
mkdir -p refs
for r in fusionspace-loft loft-fixtures; do
  if [ -d "refs/$r/.git" ]; then ok "refs/$r present"
  elif have gh && gh repo clone "nrdptel/$r" "refs/$r" >/dev/null 2>&1; then ok "cloned nrdptel/$r into refs/$r"
  else warn "couldn't clone nrdptel/$r (Claude Code will skip what it can't reach)"; fi
done
if [ -d .git ]; then
  if git check-ignore -q refs/; then ok "refs/ is ignored by git"; else bad "refs/ is NOT gitignored; fix .gitignore before running"; fi
fi

echo
echo "== Claude Code workspace trust"
trusted=$(python3 - "$ROOT" <<'PY' 2>/dev/null
import json, os, sys
p = os.path.expanduser("~/.claude.json")
try:
    data = json.load(open(p))
except Exception:
    print("unknown"); sys.exit()
proj = (data.get("projects") or {}).get(sys.argv[1]) or {}
print("yes" if proj.get("hasTrustDialogAccepted") else "no")
PY
)
case "$trusted" in
  yes) ok "this folder is trusted; the project's allow rules apply in headless runs" ;;
  *)   warn "This folder isn't marked trusted yet. Run 'claude' here once, accept the trust prompt, then type /exit. Without that, headless runs ignore the project's allow rules." ;;
esac

if have claude && [ "$trusted" = "yes" ] && ask "Run a tiny headless test (one short Opus 5.5 reply) to confirm sign-in, the model and the permission mode?"; then
  smoke=$(claude -p "Reply with exactly the word OK and nothing else." --model "${HPR_MODEL:-claude-opus-5-5}" --permission-mode "${HPR_PERMISSION_MODE:-bypassPermissions}" \
            --output-format stream-json --verbose --max-turns 1 < /dev/null 2>&1 | python3 -c '
import json, sys
mode, result, err = "?", "", False
for line in sys.stdin:
    try:
        ev = json.loads(line)
    except Exception:
        continue
    if ev.get("type") == "system" and ev.get("subtype") == "init":
        mode = ev.get("permissionMode") or ev.get("permission_mode") or mode
        model = ev.get("model")
    if ev.get("type") == "result":
        result, err = str(ev.get("result", ""))[:120], bool(ev.get("is_error"))
print(f"{mode}|{err}|{result}")')
  IFS='|' read -r smode serr sresult <<< "$smoke"
  if [ "$serr" = "False" ] && [ "$smode" = "${HPR_PERMISSION_MODE:-bypassPermissions}" ]; then ok "headless test passed (mode: $smode, reply: $sresult)"
  else bad "headless test failed (mode: ${smode:-?}, reply: ${sresult:-none}). Fix sign-in, model access or the permission mode before starting."; fi
fi

case "$ROOT" in
  "$HOME/Documents/"*|"$HOME/Desktop/"*)
    if [ -d "$HOME/Library/Mobile Documents/com~apple~CloudDocs/Documents" ] || [ -d "$HOME/Library/Mobile Documents/com~apple~CloudDocs/Desktop" ]; then
      warn "This project is under ~/Documents or ~/Desktop and iCloud 'Desktop & Documents' looks enabled. Build output, refs/ and .git would sync to iCloud; turn that off or move the project (see docs/AUTOPILOT.md)."
    fi ;;
esac

echo
echo "== Power"
if have pmset; then
  if pmset -g batt 2>/dev/null | grep -q "AC Power"; then ok "on AC power"; else warn "Not on AC power. Plug in; caffeinate can only prevent system sleep on AC."; fi
  warn "Keep the lid open (or use clamshell mode with an external display). A closed lid on a MacBook sleeps regardless of caffeinate."
fi

echo
if [ "$PROBLEMS" -gt 0 ]; then
  echo "Preflight found $PROBLEMS problem(s) and $WARNINGS warning(s). Fix the ✗ items, then run this again."
  exit 1
fi
echo "Preflight passed with $WARNINGS warning(s). Start the run with:"
echo "    scripts/autopilot.sh 48"
