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
# Everything else passes (exit 0). The Bash check is a text match: a path
# assembled at run time is not seen.

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
