#!/usr/bin/env python3
"""PreToolUse guard for Bash (and WebFetch) calls. It enforces the parts of CLAUDE.md that must never slip:

* no pushes to main/master and no force pushes (work ships through PRs, and CI must run first)
* no AI-attribution traces in commit messages or PR bodies (Neer's zero-trace rule)
* no changes to the git identity other than Neer's approved one
* no force-adding the private reference library (refs/, corpus/), and no committing any file whose
  bytes match a file in the private loft-fixtures corpus
* no `gh pr merge` unless every CI check on the PR has passed, and never with --admin
* no fetching OpenRocket's GPL source code (clean room), from Bash or WebFetch. Release assets
  downloaded by URL (the jar, the thesis and technical-documentation PDFs), its issues and other
  repos such as openrocket-database stay allowed; `gh release download` is blocked because it can
  fetch the source archive

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


# OpenRocket's source repository, in the forms a fetch names it: files, clones, archives, diffs and
# the contents API. Release downloads (github.com/openrocket/openrocket/releases/...), issues and
# sibling repos such as openrocket-database don't match: the repo name must end at a slash, `.git`
# or the end of the token.
GPL_SOURCE = re.compile(
    r"raw\.githubusercontent\.com/openrocket/openrocket/"
    r"|codeload\.github\.com/openrocket/openrocket/"
    r"|patch-diff\.githubusercontent\.com/raw/openrocket/openrocket/"
    r"|github\.com/openrocket/openrocket/(?:blob|tree|raw|archive|commits?|zipball|tarball|compare)/"
    r"|github\.com/openrocket/openrocket/pull/\d+(?:/files|/commits|\.diff|\.patch)"
    r"|github\.com[:/]openrocket/openrocket(?:\.git)?/?$"
    r"|repos/openrocket/openrocket/(?:contents|zipball|tarball|git|commits|readme|compare|pulls/\d+/files)"
    r"|^(?:--repo=)?openrocket/openrocket(?:\.git)?$",
    re.IGNORECASE,
)
# Commands that download whatever URL they are given.
FETCHERS = {"curl", "wget", "svn", "http", "https", "xh", "aria2c"}
# git and gh subcommands that fetch a repository's content; everything else (commit messages, PR
# bodies, `git grep`, reading upstream issues) may mention the repo freely.
GIT_FETCHES = {"clone", "fetch", "pull", "submodule", "remote", "archive", "ls-remote"}
GH_FETCHES = {("repo", "clone"), ("pr", "diff"), ("pr", "checkout"), ("search", "code"), ("release", "download"), ("browse", None)}
# Wrappers that run the command after them.
WRAPPERS = {"env", "command", "nice", "nohup", "time", "exec", "timeout"}
WRAPPER_OPTIONS_WITH_VALUE = {"-s", "--signal", "-k", "--kill-after", "-n", "--adjustment", "-u", "--unset", "-C", "--chdir"}


def strip_wrappers(tokens: list[str]) -> list[str]:
    """Drops `VAR=value`, `env`, `timeout 60` and similar prefixes in front of the real command."""
    i = 0
    while i < len(tokens):
        t = tokens[i]
        base = os.path.basename(t)
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", t):
            i += 1
        elif base in WRAPPERS:
            i += 1
            while i < len(tokens) and tokens[i].startswith("-"):
                # Options such as `timeout -s KILL`, `nice -n 10` and `env -u HOME` take a value.
                i += 2 if tokens[i] in WRAPPER_OPTIONS_WITH_VALUE else 1
            if base == "timeout" and i < len(tokens):
                i += 1  # the duration
        else:
            break
    return tokens[i:]


def fetches_gpl_source(tokens: list[str]) -> bool:
    tokens = strip_wrappers(tokens)
    if not tokens:
        return False
    base = os.path.basename(tokens[0]).lower()
    if base == "git":
        sub, args = git_subcommand(tokens)
        candidates = args if sub in GIT_FETCHES else []
    elif base == "gh":
        words = [a for a in tokens[1:] if not a.startswith("-")]
        pair = (words[0] if words else None, words[1] if len(words) > 1 else None)
        fetches = pair[0] == "api" or pair in GH_FETCHES or (pair[0], None) in GH_FETCHES
        candidates = tokens[1:] if fetches else []
    elif base in FETCHERS:
        candidates = tokens[1:]
    else:
        candidates = []
    return any(GPL_SOURCE.search(t) for t in candidates)


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


# Screening of text that becomes public: PR and issue bodies, comments, reviews, API writes, and
# commit messages (public once pushed). CLAUDE.md rule 4 allows counts, error statistics and
# anonymised case ids from the private corpus. It does not allow a private design's name, its
# sizes or masses, or a ratio or percentage that back-computes one. A hook can't tell those numbers
# from allowed statistics. So it stops the post in two cases, until the command says the text was
# re-read (a trailing `# private-data-checked` comment):
#   - the text names a design in the private corpus;
#   - it talks about a private design and carries measured values.
# Why: the first revision of issue #186 quoted one private design's sizes, and GitHub keeps every
# revision of an issue or PR body until the owner deletes it by hand.
PRIVATE_DIRS = ("loft-fixtures", "debrief-fixtures")
GENERIC_NAMES = {"rocket", "untitled", "my rocket", "new rocket", "sustainer", "booster", "rocket design"}
PRIVATE_CONTEXT = re.compile(
    r"(?i)\bprivate\s+(?:design|flight|file|corpus|case|fixture|log|rocket|data|example)s?\b"
    r"|loft-fixtures|debrief-fixtures|\b[A-Z]\d{2}/\d+\b"
)
MEASURED = re.compile(
    r"(?<![\w.])[-−+]?\d+(?:[.,]\d+)?\s?(?:mm|cm|m|km|in|ft|g|kg|lb|oz|N|Ns|s|%|calibres?|calibers?|times)(?![A-Za-z])"
)
PRIVATE_ACK = re.compile(r"#\s*private-data-checked\b")


def private_names(cwd: str) -> set[str]:
    """The design names in the private corpus: each .ork file's rocket name, and long file stems.

    A name that already appears in the repository's tracked files is public and left out. The
    corpus holds copies of OpenRocket's bundled examples, whose names the public fixtures and
    reports quote; checking the past 112 PR bodies turned up only such names.
    """
    import gzip
    import zipfile

    names: set[str] = set()
    base = os.path.join(os.environ.get("CLAUDE_PROJECT_DIR") or cwd, "refs")
    for sub in PRIVATE_DIRS:
        for dirpath, dirnames, filenames in os.walk(os.path.join(base, sub)):
            dirnames[:] = [d for d in dirnames if d != ".git"]
            for fname in filenames:
                stem, ext = os.path.splitext(fname)
                if len(stem) >= 12 and not stem.startswith("."):
                    names.add(stem)
                if ext.lower() != ".ork":
                    continue
                path = os.path.join(dirpath, fname)
                data = b""
                try:
                    with zipfile.ZipFile(path) as z:
                        inner = [n for n in z.namelist() if n.endswith((".ork", ".xml"))]
                        data = z.read(inner[0]) if inner else b""
                except Exception:
                    try:
                        with gzip.open(path) as fh:
                            data = fh.read()
                    except Exception:
                        try:
                            with open(path, "rb") as fh:
                                data = fh.read()
                        except OSError:
                            data = b""
                m = re.search(r"<rocket>\s*<name>([^<]{1,120})</name>", data.decode("utf-8", "replace"))
                if m:
                    names.add(m.group(1).strip())
    names = {n for n in names if len(n) >= 5 and n.lower() not in GENERIC_NAMES}
    if names:
        root = os.environ.get("CLAUDE_PROJECT_DIR") or cwd
        pattern_args: list[str] = []
        for n in sorted(names):
            pattern_args += ["-e", n]
        try:
            found = subprocess.run(
                ["git", "grep", "-h", "-o", "-i", "-F", "-I", *pattern_args],
                cwd=root, capture_output=True, text=True, timeout=20,
            ).stdout
            public = {line.strip().lower() for line in found.splitlines()}
            names = {n for n in names if n.lower() not in public}
        except Exception:
            pass
    return names


def screen_private(text: str, raw: str, cwd: str, what: str) -> None:
    if not text.strip() or PRIVATE_ACK.search(raw):
        return
    lowered = text.lower()
    named = [n for n in private_names(cwd)
             if n.lower() in lowered and re.search(r"(?<!\w)" + re.escape(n) + r"(?!\w)", text, re.IGNORECASE)]
    if named:
        block(f"{what} names {len(named)} design(s) from the private corpus (for example '{named[0]}'). "
              "CLAUDE.md rule 4: refer to a private design only by its anonymised case id. Rewrite it; if the name is "
              "really a public one, re-run the same command with a trailing `# private-data-checked` comment.")
    if PRIVATE_CONTEXT.search(text):
        values = [m.group(0).strip() for m in MEASURED.finditer(text)][:4]
        if values:
            block(f"{what} talks about a private design and carries measured values ({', '.join(values)}). "
                  "CLAUDE.md rule 4 allows counts, error statistics and anonymised case ids, never a private design's "
                  "sizes, masses or positions, nor a ratio or percentage that back-computes one. Re-read the text for "
                  "exactly that and fix it, then re-run the same command with a trailing `# private-data-checked` "
                  "comment. Posted text can't be taken back: GitHub keeps every revision of a body.")


def files_named(tokens: list[str], cwd: str) -> str:
    """Contents of files a command reads its payload from: `@path` values and `--input path`."""
    parts: list[str] = []
    for i, t in enumerate(tokens):
        path = None
        if "=@" in t:
            path = t.split("=@", 1)[1]
        elif t.startswith("@"):
            path = t[1:]
        elif t == "--input" and i + 1 < len(tokens):
            path = tokens[i + 1]
        if path and path != "-":
            full = path if os.path.isabs(path) else os.path.join(cwd, path)
            try:
                with open(full, encoding="utf-8", errors="replace") as fh:
                    parts.append(fh.read())
            except OSError:
                pass
    return "\n".join(parts)


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
        block("This PR has no CI checks reported yet. Wait for CI (`scripts/ci-wait.sh <pr>`) before merging.")
    try:
        checks = json.loads(out.stdout) if out.stdout.strip() else None
    except ValueError:
        checks = None
    if checks is None:
        return  # an older gh without --json, or another failure: can't verify, don't wedge the run
    if not checks:
        block("This PR has no CI checks reported yet. Wait for CI (`scripts/ci-wait.sh <pr>`) before merging.")
    bad = [c.get("name", "?") for c in checks if c.get("bucket") not in ("pass", "skipping")]
    if bad:
        block("CI isn't green yet: " + ", ".join(bad[:6]) + ". Wait with `scripts/ci-wait.sh <pr>` and fix failures before merging.")


def check_segment(tokens: list[str], cwd: str, raw: str, state: dict) -> None:
    if fetches_gpl_source(tokens):
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
        screen_private(msg, raw, cwd, "The commit message (public once pushed)")
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
    is_gh = bool(tokens) and os.path.basename(tokens[0]) == "gh"
    if is_gh and len(tokens) >= 3 and tokens[1] in ("pr", "issue") and tokens[2] in ("create", "edit", "comment", "review"):
        body = message_from_args(tokens[3:], cwd, ("-b", "--body", "-t", "--title"), ("-F", "--body-file"))
        pat = has_trace(body)
        if pat:
            block(f"PR/issue text contains an AI-attribution trace (matched /{pat}/). Remove it; after posting, read the body back with `gh pr view`.")
        # A body written by a heredoc in this same command isn't on disk yet, so screen the command too.
        text = body + ("\n" + raw if "<<" in raw or not body.strip() else "")
        screen_private(text, raw, cwd, "This PR/issue text")
    if is_gh and len(tokens) >= 2 and tokens[1] == "api":
        method = next((tokens[i + 1] for i, t in enumerate(tokens[:-1]) if t in ("-X", "--method")), "")
        writes = method.upper() in ("POST", "PATCH", "PUT") or any(
            t in ("-f", "-F", "--field", "--raw-field", "--input") for t in tokens)
        if "graphql" in tokens[2:3]:
            writes = "mutation" in raw
        if writes and method.upper() != "GET":
            screen_private(raw + "\n" + files_named(tokens, cwd), raw, cwd, "This GitHub API write")


def main() -> None:
    try:
        data = json.load(sys.stdin)
    except Exception:
        sys.exit(0)
    if data.get("tool_name") == "WebFetch":
        url = (data.get("tool_input") or {}).get("url") or ""
        if GPL_SOURCE.search(url):
            block("That URL is OpenRocket's GPL source code, which this clean-room project never reads (CLAUDE.md hard rule 3). "
                  "Use its published docs (openrocket.readthedocs.io, the wiki, the technical documentation PDF) instead.")
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
