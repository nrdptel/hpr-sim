#!/usr/bin/env python3
"""PreToolUse guard for Bash calls. It enforces the parts of CLAUDE.md that must never slip:

* no pushes to main/master and no force pushes (work ships through PRs, and CI must run first)
* no AI-attribution traces in commit messages or PR bodies (Neer's zero-trace rule)
* no changes to the git identity other than Neer's approved one
* no force-adding the private reference library (refs/, corpus/), and no committing any file whose
  bytes match a file in the private loft-fixtures corpus
* no `gh pr merge` unless every CI check on the PR has passed, and never with --admin
* no fetching OpenRocket's GPL source code (clean room). Its release assets (the jar, the thesis and
  technical-documentation PDFs) and other repos such as openrocket-database stay allowed

Exit code 2 blocks the call, and stderr goes back to Claude as the reason. Any internal error
exits 0 so a bug in this guard never wedges a run; the settings.json deny rules are
the other layer (they still apply in bypassPermissions mode).
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shlex
import subprocess
import sys

APPROVED_NAME = "Neer Patel"
APPROVED_EMAIL = "135655563+nrdptel@users.noreply.github.com"
PROTECTED_BRANCHES = {"main", "master"}
SEPARATORS = {"&&", "||", ";", "|", "&", "\n", "(", ")"}

TRACE_PATTERNS = [
    r"co-authored-by",
    r"generated (with|by) \[?(claude|an? ai|ai)",
    r"written (with|by) \[?(claude|an? ai|ai)",
    r"\U0001F916",  # robot emoji
    r"claude\.ai/code",
    r"claude\.com/claude-code",
    r"claude-session",
    r"anthropic",
    r"noreply@anthropic",
    r"\bclaude\b",
]


# OpenRocket's source repository, in the forms a fetch names it. Release downloads
# (github.com/openrocket/openrocket/releases/...) don't match, and neither do sibling repos such as
# openrocket-database, because the repo name must end at a slash, `.git` or the end of the token.
GPL_SOURCE = re.compile(
    r"raw\.githubusercontent\.com/openrocket/openrocket/"
    r"|github\.com/openrocket/openrocket/(?:blob|tree|raw|archive|commits?|zipball|tarball)/"
    r"|github\.com[:/]openrocket/openrocket(?:\.git)?/?$"
    r"|repos/openrocket/openrocket/(?:contents|zipball|tarball|git|commits|readme)"
    r"|^openrocket/openrocket(?:\.git)?$",
    re.IGNORECASE,
)
FETCHERS = {"curl", "wget", "git", "gh", "svn", "http", "https", "xh", "aria2c"}


def block(reason: str) -> None:
    sys.stderr.write(f"Blocked by .claude/hooks/guard-bash.py: {reason}\n")
    sys.exit(2)


def has_trace(text: str) -> str | None:
    # CLAUDE.md and the .claude/ directory are legitimate file names in this repo.
    scrubbed = re.sub(r"CLAUDE\.md|\.claude/?[\w./-]*", "", text, flags=re.IGNORECASE)
    for pat in TRACE_PATTERNS:
        if re.search(pat, scrubbed, flags=re.IGNORECASE):
            return pat
    return None


def split_commands(command: str) -> list[list[str]]:
    lexer = shlex.shlex(command, posix=True, punctuation_chars=";&|()\n")
    lexer.whitespace = " \t\r"
    lexer.whitespace_split = True
    lexer.commenters = ""
    segments: list[list[str]] = [[]]
    for tok in lexer:
        if tok in SEPARATORS or set(tok) <= set("&|;()\n"):
            segments.append([])
        else:
            segments[-1].append(tok)
    return [s for s in segments if s]


def git_subcommand(tokens: list[str]) -> tuple[str | None, list[str]]:
    """For a `git [-C dir] [-c k=v] <sub> args...` segment, return (sub, args)."""
    # Allow env-var prefixes such as `GIT_TRACE=1 git push`.
    i = 0
    while i < len(tokens) and re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", tokens[i]):
        i += 1
    if i >= len(tokens) or os.path.basename(tokens[i]) != "git":
        return None, []
    i += 1
    while i < len(tokens):
        t = tokens[i]
        if t in ("-C", "-c", "--git-dir", "--work-tree", "--namespace"):
            i += 2
            continue
        if t.startswith("-"):
            i += 1
            continue
        return t, tokens[i + 1 :]
    return None, []


def current_branch(cwd: str) -> str:
    try:
        out = subprocess.run(
            ["git", "branch", "--show-current"], cwd=cwd, capture_output=True, text=True, timeout=5
        )
        return out.stdout.strip()
    except Exception:
        return ""


def check_push(args: list[str], cwd: str, branch: str) -> None:
    for a in args:
        if a in ("--all", "--mirror", "--tags", "--follow-tags"):
            block(f"`git push {a}` is not allowed. Push one feature branch at a time.")
        if a in ("-f", "--force", "--force-with-lease", "--force-if-includes", "--delete", "-d") or a.startswith("--force"):
            block(f"`git push {a}` is not allowed. Never force-push or delete remote refs; open a new branch/PR instead.")
    positionals = [a for a in args if not a.startswith("-")]
    refspecs = positionals[1:] if positionals else []
    for spec in refspecs:
        if spec.startswith("+"):
            block("Force refspecs (+ref) are not allowed.")
        dest = spec.split(":", 1)[1] if ":" in spec else spec
        dest = dest.removeprefix("refs/heads/")
        if dest in PROTECTED_BRANCHES:
            block(f"Pushing to '{dest}' is not allowed. Push a feature branch and merge through a PR with green CI (CLAUDE.md, 'How work ships').")
        if dest == "HEAD" and branch in PROTECTED_BRANCHES:
            block("You're on main; `git push ... HEAD` would push to main. Create a feature branch.")
    if not refspecs and branch in PROTECTED_BRANCHES:
        block("You're on main; a bare `git push` would push to main. Create a feature branch and open a PR.")


def message_from_args(args: list[str], cwd: str, flags_inline: tuple[str, ...], flags_file: tuple[str, ...]) -> str:
    parts: list[str] = []
    for i, a in enumerate(args):
        for f in flags_inline:
            if a == f and i + 1 < len(args):
                parts.append(args[i + 1])
            elif a.startswith(f + "="):
                parts.append(a.split("=", 1)[1])
            elif len(f) == 2 and a.startswith(f) and len(a) > 2 and not a.startswith("--"):
                parts.append(a[2:])
            elif len(f) == 2 and re.fullmatch(r"-[a-zA-Z]*" + re.escape(f[1]), a) and i + 1 < len(args):
                parts.append(args[i + 1])  # combined short flags such as -am "msg"
        for f in flags_file:
            path = None
            if a == f and i + 1 < len(args):
                path = args[i + 1]
            elif a.startswith(f + "="):
                path = a.split("=", 1)[1]
            if path and path != "-":
                full = path if os.path.isabs(path) else os.path.join(cwd, path)
                try:
                    with open(full, encoding="utf-8", errors="replace") as fh:
                        parts.append(fh.read())
                except OSError:
                    pass
    return "\n".join(parts)


def private_fixture_hashes(cwd: str) -> set[str]:
    """sha256 of every file in the private corpus (refs/loft-fixtures), if it is present."""
    root = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR") or cwd, "refs", "loft-fixtures")
    hashes: set[str] = set()
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d != ".git"]
        for name in filenames:
            try:
                with open(os.path.join(dirpath, name), "rb") as fh:
                    data = fh.read()
            except OSError:
                continue
            if len(data) >= 64:  # ignore tiny files (licenses, empty stubs) that could collide innocently
                hashes.add(hashlib.sha256(data).hexdigest())
    return hashes


def staged_private_files(cwd: str, include_tracked_changes: bool) -> list[str]:
    private = private_fixture_hashes(cwd)
    if not private:
        return []
    try:
        names = subprocess.run(
            ["git", "diff", "--cached", "--name-only", "-z"], cwd=cwd, capture_output=True, timeout=10
        ).stdout.decode("utf-8", "replace").split("\0")
        if include_tracked_changes:
            names += subprocess.run(
                ["git", "diff", "--name-only", "-z"], cwd=cwd, capture_output=True, timeout=10
            ).stdout.decode("utf-8", "replace").split("\0")
    except Exception:
        return []
    hits = []
    for name in {n for n in names if n}:
        try:
            blob = subprocess.run(["git", "show", f":{name}"], cwd=cwd, capture_output=True, timeout=10).stdout
            with open(os.path.join(cwd, name), "rb") as fh:
                work = fh.read()
        except Exception:
            blob, work = b"", b""
        if hashlib.sha256(blob).hexdigest() in private or hashlib.sha256(work).hexdigest() in private:
            hits.append(name)
    return hits


def check_pr_merge(args: list[str], cwd: str) -> None:
    if any(a == "--admin" or a.startswith("--admin=") for a in args):
        block("`gh pr merge --admin` is not allowed; merge only after CI is green.")
    target = [a for a in args if not a.startswith("-")][:1]
    try:
        out = subprocess.run(
            ["gh", "pr", "checks", *target, "--json", "name,bucket"],
            cwd=cwd, capture_output=True, text=True, timeout=25,
        )
    except Exception:
        return  # can't verify (gh missing or offline); don't wedge the run
    if "no checks reported" in (out.stderr or "").lower():
        block("This PR has no CI checks reported yet. Wait for CI (`gh pr checks --watch`) before merging.")
    try:
        checks = json.loads(out.stdout) if out.stdout.strip() else None
    except ValueError:
        checks = None
    if checks is None:
        return  # an older gh without --json, or another failure: can't verify, don't wedge the run
    if not checks:
        block("This PR has no CI checks reported yet. Wait for CI (`gh pr checks --watch`) before merging.")
    bad = [c.get("name", "?") for c in checks if c.get("bucket") not in ("pass", "skipping")]
    if bad:
        block("CI isn't green yet: " + ", ".join(bad[:6]) + ". Wait with `gh pr checks --watch` and fix failures before merging.")


def check_segment(tokens: list[str], cwd: str, raw: str, state: dict) -> None:
    if tokens and os.path.basename(tokens[0]) in FETCHERS and any(GPL_SOURCE.search(t) for t in tokens[1:]):
        block("That fetches OpenRocket's GPL source code, which this clean-room project never reads (CLAUDE.md hard rule 3). "
              "Use its published docs, or run the pinned jar as an oracle.")
    for t in tokens:
        m = re.match(r"^(?:user\.(?:name|email)|GIT_(?:AUTHOR|COMMITTER)_(?:NAME|EMAIL))=(.*)$", t)
        if m and m.group(1) not in (APPROVED_NAME, APPROVED_EMAIL):
            block(f"Overriding the git identity is not allowed; it must stay '{APPROVED_NAME} <{APPROVED_EMAIL}>'.")
    sub, args = git_subcommand(tokens)
    if sub in ("switch", "checkout"):
        # Track branch changes earlier in the same command line (e.g. `git switch main && git push`).
        positional = [a for a in args if not a.startswith("-")]
        if "-c" in args or "-b" in args or "-C" in args or "-B" in args or "--create" in args:
            state["branch"] = positional[0] if positional else state["branch"]
        elif positional and positional[0] not in (".", "--"):
            state["branch"] = positional[0]
    elif sub == "push":
        check_push(args, cwd, state["branch"])
    elif sub == "commit":
        msg = message_from_args(args, cwd, ("-m", "--message"), ("-F", "--file"))
        reads_stdin = any(a in ("-F-", "--file=-") for a in args) or any(
            a in ("-F", "--file") and i + 1 < len(args) and args[i + 1] == "-" for i, a in enumerate(args)
        )
        if reads_stdin:
            msg += "\n" + raw  # the message arrives through a heredoc or pipe, so check the whole command
        pat = has_trace(msg)
        if pat:
            block(f"Commit message contains an AI-attribution trace (matched /{pat}/). Rewrite it without any reference to the tool that wrote it.")
        if any(a == "--author" or a.startswith("--author=") for a in args):
            block("Don't override the commit author; the repo identity is set by scripts/preflight.sh.")
        include_tracked = any(a in ("-a", "--all") or re.fullmatch(r"-[a-zA-Z]*a[a-zA-Z]*", a) for a in args)
        hits = staged_private_files(cwd, include_tracked)
        if hits:
            block("These files are byte-identical to files in the private loft-fixtures corpus and must never be committed: "
                  + ", ".join(hits[:5]) + ". Unstage them (git restore --staged) and use them only from refs/.")
    elif sub == "config":
        joined = " ".join(args)
        if "--global" in args or "--system" in args:
            block("Global/system git config changes are not allowed.")
        if re.search(r"\buser\.(name|email)\b", joined):
            values = [a for a in args if not a.startswith("-") and not re.match(r"user\.(name|email)$", a)]
            if values and not all(v in (APPROVED_NAME, APPROVED_EMAIL) for v in values):
                block(f"The git identity must stay '{APPROVED_NAME} <{APPROVED_EMAIL}>'.")
    elif sub == "add":
        if any(a in ("-f", "--force") for a in args) and re.search(r"(^|\s|/)(refs|corpus)(/|\s|$)", " ".join(args)):
            block("refs/ and corpus/ hold private or third-party data and must never be committed.")

    if tokens and os.path.basename(tokens[0]) == "gh" and len(tokens) >= 3 and tokens[1] == "pr" and tokens[2] == "merge":
        check_pr_merge(tokens[3:], cwd)
    if tokens and os.path.basename(tokens[0]) == "gh" and len(tokens) >= 3 and tokens[1] in ("pr", "issue") and tokens[2] in ("create", "edit", "comment"):
        body = message_from_args(tokens[3:], cwd, ("-b", "--body", "-t", "--title"), ("-F", "--body-file"))
        pat = has_trace(body)
        if pat:
            block(f"PR/issue text contains an AI-attribution trace (matched /{pat}/). Remove it; after posting, read the body back with `gh pr view`.")


def main() -> None:
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)
    if data.get("tool_name") != "Bash":
        sys.exit(0)
    command = (data.get("tool_input") or {}).get("command") or ""
    cwd = data.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd()
    try:
        segments = split_commands(command)
    except ValueError:
        # Unbalanced quotes etc. Fall back to a coarse check on the raw text.
        if re.search(r"git\s+push\b.*(--force|\s-f\b|\bmain\b|\bmaster\b)", command):
            block("This push looks like a force push or a push to main, and the command could not be parsed safely. Rewrite it plainly.")
        sys.exit(0)
    state = {"branch": current_branch(cwd)}
    for seg in segments:
        check_segment(seg, cwd, command, state)
    sys.exit(0)


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        sys.exit(0)
