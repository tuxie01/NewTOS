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
#   an inline hook override `-c core.hooksPath=...`.
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
# `git commit ... -n` / `-an` / `-nm` ...: short-flag clusters after `git commit`,
# within the same simple command (tokens stop at ; & |).
if printf '%s' "$cmd" | grep -qE '(^|[^A-Za-z0-9_-])git([[:space:]]+-[^[:space:]]+)*[[:space:]]+commit([[:space:]]+[^[:space:];&|]+)*[[:space:]]+-[A-Za-mo-z]*n[A-Za-z]*([[:space:];&|]|$)'; then
    block "git commit -n (= --no-verify) is not allowed for agents"
fi
exit 0
