#!/usr/bin/env bash
#
# scratch-guard.sh -- PreToolUse hook (.claude/settings.json) that keeps
# scratch files out of the shared /tmp (CLAUDE.md "Scratch"; user decision
# 2026-10-30 after two agents wrote there because CLAUDE_JOB_DIR was empty).
#
# Reads the hook JSON on stdin. Blocks with exit 2 (message on stderr, shown
# to the agent) when
#   Bash:        the command uses $CLAUDE_JOB_DIR / ${CLAUDE_JOB_DIR...} while
#                that variable is empty, or names a path under /tmp or
#                /private/tmp outside the session scratch root
#                /private/tmp/claude-<uid>/ (also reachable as /tmp/claude-<uid>/);
#   Write/Edit:  file_path lies under /tmp or /private/tmp outside that root.
#   Bash, TOOL46: a `git clone` or a cargo target directory
#                (CARGO_TARGET_DIR=..., --target-dir ...) in a command segment
#                that names a path under the session scratch root: clones and
#                cargo targets fill the system disk; they go under
#                <repo>/target-agents/agent-<id>/ (named to the worker by
#                tools/hooks/agent-scratch.sh at SubagentStart) or the
#                worktree's own target/.
# Everything else passes (exit 0); target-agents/ lies on the data volume and
# is never matched. The Bash check is a text match: a path assembled at run
# time is not seen.
#
# TOOL46 check, known false positives (blocked though nothing is cloned): the
# words `git clone` as text -- in an echo, a `git commit -m "..."` message, a
# comment -- in the same segment (between ; & | or newlines) as a path under
# the scratch root. Known gaps (not blocked): a relative clone target after
# a cd (`cd <scratch> && git clone <url> c`, the clone's segment names no
# scratch path), `git worktree add <scratch>/...`, a target dir set through a
# variable (`CARGO_TARGET_DIR=$X`, `$TMPDIR/...`), cargo's `build.target-dir`
# in a config file.

set -u

input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
uid=$(id -u)
allowed_re="^(/private)?/tmp/claude-${uid}(/|$)"

block() {
    echo "scratch-guard: $1" >&2
    echo "scratch: use the session scratchpad given in your brief; CLAUDE_JOB_DIR is empty" >&2
    exit 2
}

# Every /tmp or /private/tmp path token in $1 that is not under the scratch root.
foreign_tmp_paths() {
    printf '%s\n' "$1" \
        | grep -oE '(^|[^A-Za-z0-9_.~/-])(/private)?/tmp(/[^[:space:]"'"'"'`;|&)<>]*)?' \
        | sed -E 's#^[^/]##' \
        | grep -vE "$allowed_re" || true
}

# TOOL46: prints what it found when a command segment (split at ; & | and
# newlines) runs `git clone` or sets a cargo target dir and names a path under
# the scratch root; prints nothing otherwise.
bulky_in_scratch() {
    printf '%s\n' "$1" | tr ';&|' '\n\n\n' | while IFS= read -r seg; do
        under=$(printf '%s\n' "$seg" \
            | grep -oE '(^|[^A-Za-z0-9_.~/-])(/private)?/tmp(/[^[:space:]"'"'"'`;|&)<>]*)?' \
            | sed -E 's#^[^/]##' | grep -E "$allowed_re" | head -1)
        [ -n "$under" ] || continue
        if printf '%s' "$seg" | grep -qE "$clone_re"; then
            echo "git clone into $under"
            return
        fi
        if printf '%s' "$seg" | grep -qE "$target_re"; then
            echo "cargo target dir $under"
            return
        fi
    done
}
# `git [global options] clone`; a global option may take one argument (-C dir).
clone_re='(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+([[:space:]]+[^-[:space:]][^[:space:]]*)?)*[[:space:]]+clone([[:space:]]|$)'
target_re="(CARGO_TARGET_DIR=|--target-dir([[:space:]]+|=))[\"']?(/private)?/tmp/claude-${uid}(/|[[:space:]\"']|$)"

case "$tool" in
    Bash)
        cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')
        if [ -z "${CLAUDE_JOB_DIR:-}" ] && printf '%s' "$cmd" | grep -qE '\$\{?CLAUDE_JOB_DIR'; then
            block "the command uses \$CLAUDE_JOB_DIR, which is empty here (it would expand to /tmp)"
        fi
        bad=$(foreign_tmp_paths "$cmd")
        if [ -n "$bad" ]; then
            block "path(s) in the shared /tmp: $(printf '%s' "$bad" | tr '\n' ' ')"
        fi
        big=$(bulky_in_scratch "$cmd")
        if [ -n "$big" ]; then
            echo "scratch-guard: $big: the session scratch root lies on the system disk" >&2
            echo "scratch: clones and cargo target dirs go under <repo>/target-agents/agent-<id>/ (TOOL46) or the worktree's target/" >&2
            exit 2
        fi
        ;;
    Write|Edit)
        path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty')
        if printf '%s' "$path" | grep -qE '^(/private)?/tmp(/|$)' \
            && ! printf '%s' "$path" | grep -qE "$allowed_re"; then
            block "file_path in the shared /tmp: $path"
        fi
        ;;
esac
exit 0
