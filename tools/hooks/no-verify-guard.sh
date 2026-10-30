#!/usr/bin/env bash
#
# no-verify-guard.sh -- PreToolUse hook (.claude/settings.json, Bash) that
# keeps agents from skipping the git pre-commit doc-check (TASK/TOOL29; user
# decision 2026-10-30: agents are blocked, the owner may still bypass by hand).
#
# Blocks with exit 2 (message on stderr, shown to the agent) a Bash command
# that runs git commit/push/merge/rebase/am with
#   --no-verify, or `git commit` with a short-flag cluster containing n
#   (`-n`, `-an`, ...: git commit's -n is --no-verify), or
#   an inline hook override `-c core.hooksPath=...`, or
#   `git config core.hooksPath` set to anything but the checked-in hook
#   directory, or unset.
# Text match on the command, like scratch-guard.sh: a command assembled at run
# time is not seen. Message texts are not parsed: pass a commit message with
# `git commit -F <file>` if it must contain these words.

set -u

input=$(cat)
tool=$(printf '%s' "$input" | jq -r '.tool_name // empty')
[ "$tool" = "Bash" ] || exit 0
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // empty')

block() {
    echo "no-verify-guard: $1" >&2
    echo "the pre-commit doc-check must run (TASK/TOOL29); fix the reported docs instead" >&2
    exit 2
}

printf '%s' "$cmd" | grep -qE '(^|[^A-Za-z0-9_-])git([[:space:]]|$)' || exit 0

if printf '%s' "$cmd" | grep -qE '(^|[[:space:]])--no-verify([[:space:]=]|$)'; then
    block "--no-verify is not allowed for agents"
fi
if printf '%s' "$cmd" | grep -qE -- '-c[[:space:]]*core\.hooks[Pp]ath='; then
    block "an inline core.hooksPath override is not allowed for agents"
fi
# `git config core.hooksPath`: agents may only read it or set it to the
# checked-in hook directory (user decision 2026-10-30). Any other value
# (empty, /dev/null, another dir) and --unset/--unset-all are blocked, in any
# config scope. Git config keys are case-insensitive.
repo=$(git -C "${CLAUDE_PROJECT_DIR:-.}" rev-parse --show-toplevel 2>/dev/null || echo "${CLAUDE_PROJECT_DIR:-.}")
allowed_rel="tools/git-hooks"
seg=$(printf '%s\n' "$cmd" | tr ';&|' '\n\n\n' | grep -iE '(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+config([[:space:]].*)?[[:space:]]core\.hookspath' || true)
if [ -n "$seg" ]; then
    while IFS= read -r line; do
        [ -n "$line" ] || continue
        if printf '%s' "$line" | grep -qE '(^|[[:space:]])--unset(-all)?([[:space:]]|$)'; then
            block "unsetting core.hooksPath is not allowed for agents"
        fi
        # A read (--get, or no value after the key) is allowed.
        printf '%s' "$line" | grep -qE '(^|[[:space:]])--get' && continue
        rest=$(printf '%s' "$line" | sed -nE 's/.*[[:space:]][Cc][Oo][Rr][Ee]\.[Hh][Oo][Oo][Kk][Ss][Pp][Aa][Tt][Hh]([[:space:]]+(.*))?$/\1/p')
        [ -n "$rest" ] || continue
        val=$(printf '%s' "$rest" | sed -E 's/^[[:space:]]+//; s/[[:space:]]+$//')
        val=${val#\"}; val=${val%\"}; val=${val#\'}; val=${val%\'}; val=${val%/}
        case "$val" in
            "$allowed_rel"|"./$allowed_rel"|"$repo/$allowed_rel") ;;
            *) block "core.hooksPath may only be set to $allowed_rel (got: '${val}')" ;;
        esac
    done <<EOF_SEG
$seg
EOF_SEG
fi

# `git commit ... -n` / `-an` / `-nm` ...: short-flag clusters after `git commit`,
# within the same simple command (tokens stop at ; & |).
if printf '%s' "$cmd" | grep -qE '(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+commit([[:space:]]+[^[:space:];&|]+)*[[:space:]]+-[A-Za-mo-z]*n[A-Za-z]*([[:space:];&|]|$)'; then
    block "git commit -n (= --no-verify) is not allowed for agents"
fi
exit 0
