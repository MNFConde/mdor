#!/usr/bin/env python3
"""Crush PreToolUse bash gate.

Classifies a bash command into:
  - allow    : safe read-only (in/out of repo) or safe in-repo write -> {"decision":"allow"}
  - deny     : irreversible/destructive -> exit 2 (blocked)
  - confirm  : everything else -> exit 0, no JSON (normal permission prompt)

Uses bashlex (AST) so compound commands (&& ; |), write-flags (-o/--output/
-w/--pretty), and path escapes (../) are handled precisely, not by regex.
This directly addresses the two known holes in a regex-based hook:
  1) 'echo hi && rm -rf /' slips past a prefix-match on 'echo'.
  2) 'git show --output=.git/config HEAD' writes via a flag (GitPwned).
"""

import os
import sys
import json

import bashlex

# ---------------------------------------------------------------------------
# Project boundary. Everything resolves against CRUSH_PROJECT_DIR.
# ---------------------------------------------------------------------------
def project_dir():
    return os.path.abspath(os.environ.get("CRUSH_PROJECT_DIR", "")) or os.getcwd()


PROJECT = project_dir()


def inside_repo(path):
    """True if the (possibly relative) path resolves inside the repo tree."""
    expanded = os.path.expanduser(path)
    abs_path = os.path.abspath(os.path.join(PROJECT, expanded))
    root = os.path.normcase(os.path.abspath(PROJECT))
    target = os.path.normcase(abs_path)
    return target == root or target.startswith(root + os.sep)


# ---------------------------------------------------------------------------
# Command classification tables.
# ---------------------------------------------------------------------------
READONLY = {
    "ls", "cat", "grep", "rg", "find", "head", "tail", "wc", "pwd",
    "echo", "printf", "which", "file", "stat", "sort", "uniq", "comm",
    "diff", "md5sum", "sha1sum", "sha256sum", "sha512sum", "date", "env",
    "du", "nl", "less", "more", "tree", "ls-files", "rev-parse",
}

DESTRUCTIVE = {
    "sudo", "rm", "mkfs", "shutdown", "reboot", "dd", "halt",
    "poweroff", "init", "parted", "fdisk", "wipefs",
}

# Git subcommands and their classification.
GIT_READONLY = {
    "status", "log", "diff", "show", "branch", "--version", "remote",
    "ls-files", "config", "rev-parse", "blame", "shortlog", "tag",
    "help", "describe",
}
GIT_SAFE_WRITE = {"add", "commit", "checkout", "switch", "restore", "mv", "rm"}
GIT_DESTRUCTIVE = {
    "reset", "clean", "rebase", "revert", "cherry-pick", "push", "pull",
    "fetch", "gc", "prune", "filter-branch", "reflog",
}
# Subcommands that can mutate state/config but also have pure-read forms.
# The per-subcommand tokens tell us which invocations are writes; any other
# invocation of these subcommands is a read and stays allow.
GIT_ACTION = {
    "branch": ("-d", "-D", "-m", "-M", "-c", "--delete", "--move", "--create"),
    "remote": ("add", "set-url", "remove", "rename", "set-head", "set-branches"),
    "tag": ("-d", "--delete", "-a", "-s", "-m", "-f", "-u", "--annotate"),
    "config": ("--global", "--system", "--local", "--file", "--add",
               "--replace-all", "--unset", "--unset-all", "--rename-section",
               "--remove-section"),
}

WRITE_FLAGS = {
    "--output", "-o", "--pretty", "--format", "-c", "--config",
    "-w", "--write", "--in-place", "--force", "-f", "--hard", "-h",
}
WRITE_FLAG_PREFIXES = ("--output=", "-o=", "--pretty=", "--format=", "--config=")


# ---------------------------------------------------------------------------
# Helpers.
# ---------------------------------------------------------------------------
def word_list(node):
    return [c.word for c in node.parts if getattr(c, "kind", "") == "word"]


def has_redirection(node):
    return any(getattr(c, "kind", "") == "redirect" for c in node.parts)


def find_mutates(node):
    """True if a 'find' command performs a write/delete action (-delete,
    -exec/-execdir/-ok). find is otherwise read-only, so these must not
    auto-allow (they bypass the rm -rf gate)."""
    words = word_list(node)
    if not words or words[0] != "find":
        return False
    for w in words[1:]:
        if w in ("-delete", "-exec", "-execdir", "-ok", "-okdir") or \
           w.startswith("-delete"):
            return True
    return False


def has_write_flag(args):
    for w in args:
        flag = w.split("=", 1)[0]
        if flag in WRITE_FLAGS or w in WRITE_FLAGS:
            return True
        if any(w.startswith(p) for p in WRITE_FLAG_PREFIXES):
            return True
    return False


def path_tokens(node):
    """Words that look like path targets (contain separator/.. or exist)."""
    words = word_list(node)
    out = []
    for w in words[1:]:
        if w in WRITE_FLAGS or w.startswith("-"):
            continue
        if os.sep in w or "/" in w or "\\" in w or ".." in w or w == ".":
            out.append(w)
        elif os.path.exists(os.path.join(PROJECT, w)):
            out.append(w)
    return out


def path_escapes(node):
    """True if any path token escapes the repo (../ or absolute outside)."""
    for c in node.parts:
        if getattr(c, "kind", "") != "word":
            continue
        w = c.word
        if w in WRITE_FLAGS or w.startswith("-"):
            continue
        if w in (".", ".."):
            if not inside_repo(w):
                return True
            continue
        if "\\" in w:
            # skip Windows-style absolute paths like C:\foo
            w = w.replace("\\", "/")
        if os.path.isabs(w) or "/" in w or ".." in w:
            if not inside_repo(w):
                return True
    return False


def reject_pipe(node):
    """True if a pipeline has a dangerous sink (| sh/bash/python etc)."""
    parts = node.parts
    for i, p in enumerate(parts):
        if getattr(p, "kind", "") == "command" and i + 1 < len(parts):
            nxt_kind = getattr(parts[i + 1], "kind", "")
            if nxt_kind == "pipe":
                # swallow the pipe node; the following command is the sink
                sink = word_list(parts[i + 2]) if i + 2 < len(parts) else []
                if sink:
                    cmd = sink[0]
                    if cmd in ("bash", "sh", "zsh", "python", "python3", "perl", "php", "ruby"):
                        return True
    return False


# ---------------------------------------------------------------------------
# Classification of one simple command node.
# ---------------------------------------------------------------------------
def classify(sub_cmd):
    words = word_list(sub_cmd)
    if not words:
        return "confirm"
    bin_ = words[0]
    args = words[1:]

    # --- Destructive / irreversible: hard deny -----------------------------
    if bin_ == "sudo":
        return "deny"
    if bin_ in DESTRUCTIVE:
        # rm is always user-confirmed (project decision): never auto-allow,
        # so no flag variant (-rf / --force / -fx ...) can slip through as
        # allow, and bare 'rm foo.txt' also gets a confirm.
        if bin_ == "rm":
            return "confirm"
        return "deny"
    # Prefix-based destructive: mkfs.ext4, apt-get, etc.
    for d in DESTRUCTIVE:
        if bin_.startswith(d):
            return "deny"
    if bin_ in ("curl", "wget"):
        if any("|" in a for a in args):
            return "deny"  # pipe to shell
        return "confirm"

    # --- git ---------------------------------------------------------------
    if bin_ == "git":
        sub = args[0] if args else ""
        if sub in GIT_DESTRUCTIVE:
            return "deny"
        # git rm / git restore / git reset always mutate working tree/index.
        if sub in ("rm", "restore", "reset"):
            return "confirm"
        # git config writes when given a key+value (2+ positionals) or a
        # write flag; a bare key/--list is a read.
        if sub == "config":
            if any(a in ("--add", "--replace-all", "--unset", "--unset-all",
                         "--remove-section", "--rename-section") for a in args):
                return "confirm"
            positionals = [a for a in args[1:] if not a.startswith("-")]
            if len(positionals) >= 2:
                return "confirm"
            return "allow"
        # Subcommands with both read and write forms: confirm only when a
        # write action-token is present; pure reads (git branch, git remote -v,
        # git tag) stay allow.
        if sub in GIT_ACTION:
            toks = GIT_ACTION[sub]
            if any(a in toks or any(a == t or a.startswith(t + "=") for t in toks)
                   for a in args):
                return "confirm"
            return "allow"
        if sub in GIT_READONLY:
            if has_write_flag(args):
                return "confirm"  # git show --output=... -> not safe
            return "allow"
        if sub in GIT_SAFE_WRITE:
            if path_escapes(sub_cmd):
                return "confirm"
            return "allow"
        return "confirm"

    # --- package managers (write): confirm, build/test: allow --------------
    if bin_ == "npm":
        sub = args[0] if args else ""
        if sub in ("install", "i", "ci", "add", "uninstall", "remove", "update", "publish"):
            return "confirm"
        return "allow"
    if bin_ in ("pnpm", "yarn", "bun"):
        sub = args[0] if args else ""
        if sub in ("install", "add", "remove", "upgrade", "update", "publish"):
            return "confirm"
        return "allow"
    if bin_ in ("pip", "pip3"):
        return "confirm"

    # --- language toolchains ------------------------------------------------
    if bin_ == "go":
        sub = args[0] if args else ""
        if sub in ("build", "test", "vet", "fmt", "mod", "run"):
            if path_escapes(sub_cmd):
                return "confirm"
            return "allow"
        return "confirm"
    if bin_ == "cargo":
        sub = args[0] if args else ""
        if sub in ("build", "test", "fmt", "check", "clippy"):
            if path_escapes(sub_cmd):
                return "confirm"
            return "allow"
        return "confirm"
    if bin_ in ("gofmt", "black", "ruff", "dprint"):
        if path_escapes(sub_cmd):
            return "confirm"
        return "allow"
    if bin_ in ("make", "just"):
        return "allow"
    if bin_ == "pytest":
        if path_escapes(sub_cmd):
            return "confirm"
        return "allow"
    if bin_ in ("touch", "mkdir"):
        if path_escapes(sub_cmd):
            return "confirm"
        return "allow"
    if bin_ == "npx":
        return "confirm"

    # --- Read-only commands --------------------------------------------------
    if bin_ in READONLY:
        # A read-only command writing via flag or redirection is NOT safe.
        if has_write_flag(args) or has_redirection(sub_cmd):
            return "confirm"
        # find with -delete/-exec mutates files and bypasses the rm gate.
        if bin_ == "find" and find_mutates(sub_cmd):
            return "confirm"
        # Reading a file outside the repo is harmless (no write side effect),
        # so we do NOT apply path_escapes here. The only risk is a write
        # (redirection/write-flag), already checked above.
        return "allow"

    # --- Default: confirm ------------------------------------------------------
    return "confirm"


# ---------------------------------------------------------------------------
# Top-level decision.
# ---------------------------------------------------------------------------
def gather_commands(ast):
    """Flatten AST into a list of simple command nodes (incl. pipeline sides)."""
    out = []
    for top in ast:
        kind = getattr(top, "kind", "")
        if kind == "command":
            out.append(top)
        elif kind in ("list", "pipeline", "andor", "compound"):
            for c in top.parts:
                ck = getattr(c, "kind", "")
                if ck == "command":
                    out.append(c)
                elif ck in ("list", "pipeline", "andor", "compound"):
                    out.extend(gather_commands([c]))
    return out


def decide(command):
    """Returns dict with 'decision' ('allow'|'deny'|'') and optional 'reason'."""
    try:
        ast = bashlex.parser.parse(command)
    except Exception as e:
        return {"decision": "", "reason": f"unparseable: {e}"}

    # Pipeline to a shell sink always hard-denies.
    for top in ast:
        if getattr(top, "kind", "") in ("pipeline",):
            if reject_pipe(top):
                return {"decision": "deny", "reason": "pipe to shell blocked"}

    cmds = gather_commands(ast)
    if not cmds:
        return {"decision": ""}

    verdicts = [classify(c) for c in cmds]
    if any(v == "deny" for v in verdicts):
        return {"decision": "deny", "reason": "destructive command blocked"}
    if verdicts and all(v == "allow" for v in verdicts):
        return {"decision": "allow"}
    return {"decision": ""}


# ---------------------------------------------------------------------------
# Entry point.
# ---------------------------------------------------------------------------
def main():
    cmd = os.environ.get("CRUSH_TOOL_INPUT_COMMAND", "")
    if not cmd:
        try:
            data = json.load(sys.stdin)
            cmd = data.get("tool_input", {}).get("command", "")
        except Exception:
            pass

    result = decide(cmd)
    decision = result.get("decision", "")

    if decision == "allow":
        print(json.dumps({"decision": "allow"}))
        sys.exit(0)
    elif decision == "deny":
        sys.stderr.write(result.get("reason", "blocked") + "\n")
        sys.exit(2)
    else:
        sys.exit(0)


if __name__ == "__main__":
    main()
