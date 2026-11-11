#!/usr/bin/env bash
#
# agent-scratch.sh -- worker scratch off the system disk (TOOL46): the Claude
# Code hooks of .claude/settings.json, and the one implementation of the
# deletion safety checks (`cargo xtask scratch-clean`, `accept` and `land`
# call the `clean` mode; tools/xtask/src/scratch.rs).
#
# The place: <main repo top>/target-agents/agent-<id>/, gitignored, on the
# data volume. <main repo top> is the parent of the git common dir of
# $CLAUDE_PROJECT_DIR, so a session or a command in a worktree uses the main
# repo's directory too. A leading `agent-` of an id is dropped, so `a1b2` and
# `agent-a1b2` name the same directory `agent-a1b2`.
#
# Hook input (https://code.claude.com/docs/en/hooks, read 2026-11-11):
# SubagentStart and SubagentStop both receive `agent_id` on stdin, the same id
# for the same subagent, also when a stopped subagent is resumed; SessionStart
# receives none. Modes:
#
#   subagent-start  (hook) names the worker's directory in its context
#                   (hookSpecificOutput.additionalContext; no environment
#                   variable carries the id) and removes the stop marker
#                   `.stopped` a previous run of the same id left.
#   subagent-stop   (hook) deletes nothing: when the directory exists, it
#                   writes the stop time into `.stopped` (overwritten on every
#                   stop; its mtime is the last stop). A worker resumed by
#                   SendMessage, or one SubagentStop fired for while it waits
#                   on its own background work, keeps its clone (owner
#                   2026-11-11). Start and stop also append a UTC line to
#                   `.stop-log`, so when SubagentStop fires is observable.
#   session-start   (hook) deletes the directories whose `.stopped` is older
#                   than 72 h (4320 min); a directory without a marker is never
#                   deleted, only listed. Then warns (systemMessage for the
#                   user, additionalContext for Claude) when target-agents/
#                   holds more than NEWTOS_AGENT_SCRATCH_WARN_KB KiB (default
#                   20971520 = 20 GiB), with the largest subdirectories.
#   clean <id>...   (command) deletes each agent-<id>/ now, marker or not.
#                   Exit 1 if one was refused, else 0.
#
# Every deletion goes through remove_agent_dir: it refuses an id that is empty
# or not [A-Za-z0-9][A-Za-z0-9_-]*, a symlink in place of target-agents/ or of
# the worker's directory, and a directory whose physical path is not
# <top>/target-agents/agent-<id>. A symlink inside the directory is removed as
# a link, never followed. The hook modes always exit 0 (exit 2 on
# SubagentStop would keep the subagent running); a refusal is reported in
# systemMessage and on stderr.

set -u

mode=${1:-}
[ $# -gt 0 ] && shift
expire_min=4320

case "$mode" in
    subagent-start) event=SubagentStart ;;
    subagent-stop) event=SubagentStop ;;
    session-start) event=SessionStart ;;
    clean) event= ;;
    *) echo "agent-scratch: unknown mode '$mode' (subagent-start|subagent-stop|session-start|clean <id>...)" >&2; exit 0 ;;
esac
input=
[ -n "$event" ] && input=$(cat)

say() {   # say <systemMessage> [additionalContext]: one JSON object on stdout
    if [ $# -ge 2 ]; then
        jq -nc --arg m "$1" --arg c "$2" --arg e "$event" \
            '{systemMessage:$m, hookSpecificOutput:{hookEventName:$e, additionalContext:$c}}'
    else
        jq -nc --arg m "$1" '{systemMessage:$m}'
    fi
}
# A hook mode gives up: message as systemMessage and on stderr, exit 0. The
# clean mode: stderr, exit 1.
give_up() {
    echo "agent-scratch: $1" >&2
    [ -n "$event" ] || exit 1
    say "agent-scratch: $1"
    exit 0
}

# top / root: the main repo top (physical) and its target-agents path.
proj=${CLAUDE_PROJECT_DIR:-}
[ -n "$proj" ] && [ -d "$proj" ] || give_up "CLAUDE_PROJECT_DIR is empty or not a directory; nothing done"
common=$(git -C "$proj" rev-parse --path-format=absolute --git-common-dir 2>/dev/null) \
    || give_up "$proj is not in a git repository; nothing done"
[ "$(basename "$common")" = ".git" ] || give_up "git common dir $common is not a .git directory; nothing done"
top=$(cd "$common/.." && pwd -P) || give_up "cannot resolve the repo top of $common"
root=$top/target-agents
[ -L "$root" ] && give_up "$root is a symlink; nothing done"

# norm_id <raw>: prints the id without a leading `agent-`; fails if invalid.
norm_id() {
    local id=${1#agent-}
    printf '%s' "$id" | grep -qE '^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$' || return 1
    printf '%s' "$id"
}

# agent_dir <id>: prints the physical path of an existing, safe agent-<id>/.
# Status 0 = safe dir, 3 = absent, 1 = refused (reason on stdout).
agent_dir() {
    local d=$root/agent-$1 phys
    if [ -L "$d" ]; then echo "$d is a symlink; not touched"; return 1; fi
    [ -e "$d" ] || return 3
    if [ ! -d "$d" ]; then echo "$d is not a directory; not touched"; return 1; fi
    phys=$(cd "$d" && pwd -P) || { echo "cannot resolve $d; not touched"; return 1; }
    if [ "$phys" != "$top/target-agents/agent-$1" ]; then
        echo "$d resolves to $phys, outside $root; not touched"
        return 1
    fi
    printf '%s' "$phys"
}

# remove_agent_dir <raw id>: the only deletion. Prints one line; status 0 =
# removed or absent, 1 = refused.
remove_agent_dir() {
    local id phys kb rc
    id=$(norm_id "$1") || { echo "agent id '$1' is not [A-Za-z0-9_-]; nothing removed"; return 1; }
    phys=$(agent_dir "$id"); rc=$?
    [ $rc = 3 ] && { echo "no $root/agent-$id; nothing to remove"; return 0; }
    [ $rc = 0 ] || { echo "$phys"; return 1; }
    kb=$(du -sk "$phys" 2>/dev/null | awk '{print $1}')
    rm -rf -- "$phys"
    if [ -e "$phys" ]; then echo "removing $phys (${kb:-?} KiB) failed in part"; return 1; fi
    echo "removed $phys (${kb:-?} KiB)"
}

if [ "$mode" = clean ]; then
    [ $# -gt 0 ] || give_up "clean needs at least one agent id"
    status=0
    for raw in "$@"; do
        line=$(remove_agent_dir "$raw") || status=1
        echo "agent-scratch: $line"
    done
    exit $status
fi

if [ "$mode" = session-start ]; then
    [ -d "$root" ] || exit 0
    expired= unmarked=
    for d in "$root"/agent-*; do
        [ -e "$d" ] || [ -L "$d" ] || continue
        name=${d##*/}
        if [ -L "$d" ] || [ ! -d "$d" ]; then
            unmarked="$unmarked $name"
            continue
        fi
        if [ -f "$d/.stopped" ] && [ ! -L "$d/.stopped" ]; then
            if [ -n "$(find "$d/.stopped" -maxdepth 0 -type f -mmin +$expire_min 2>/dev/null)" ]; then
                expired="$expired
  $(remove_agent_dir "$name")"
            fi
        else
            unmarked="$unmarked $name"
        fi
    done
    limit=${NEWTOS_AGENT_SCRATCH_WARN_KB:-20971520}
    case "$limit" in ''|*[!0-9]*) limit=20971520 ;; esac
    total=$(du -sk "$root" 2>/dev/null | awk '{print $1}')
    msg=
    if [ -n "$expired" ]; then
        msg="agent-scratch: last stop older than 72 h:$expired"
    fi
    if [ -n "$total" ] && [ "$total" -gt "$limit" ]; then
        largest=$(find "$root" -mindepth 1 -maxdepth 1 -exec du -sk {} + 2>/dev/null \
            | sort -rn | head -5 | awk '{printf "  %d MiB  %s\n", $1/1024, $2}')
        [ -n "$msg" ] && msg="$msg
"
        msg="${msg}agent-scratch: WARNING $root holds $((total / 1024)) MiB (limit $((limit / 1024)) MiB). Largest:
$largest
Without a stop marker (never deleted automatically):${unmarked:- none}
Remove a finished worker's directory after its evidence is recorded: cargo xtask scratch-clean <agent-id>."
    fi
    [ -n "$msg" ] && say "$msg" "$msg"
    exit 0
fi

raw=$(printf '%s' "$input" | jq -r '.agent_id // empty' 2>/dev/null)
if [ -z "${raw#agent-}" ]; then
    # SubagentStop also fires for Claude Code's internal agents; no id, no directory.
    [ "$mode" = subagent-stop ] && exit 0
    give_up "no agent_id in the hook input; no scratch directory named"
fi
id=$(norm_id "$raw") || give_up "agent_id '$raw' is not [A-Za-z0-9_-]; nothing done"
dir=$root/agent-$id
phys=$(agent_dir "$id"); rc=$?
[ $rc = 1 ] && give_up "$phys"
now=$(date -u +%Y-%m-%dT%H:%M:%SZ)

if [ "$mode" = subagent-start ]; then
    if [ $rc = 0 ]; then
        rm -f -- "$phys/.stopped"
        [ -L "$phys/.stop-log" ] || printf '%s start\n' "$now" >> "$phys/.stop-log"
    fi
    ctx="Scratch (TOOL46): put every test clone and every CARGO_TARGET_DIR of this run under $dir/ (mkdir -p it), never under /tmp or the session scratchpad (system disk). It stays when you stop or wait; it is removed when your ticket is accepted or landed, or 72 h after your last stop. Before you finish, record in the run's RUNINFO.TXT the clone path, its size (du -sh) and the sha256 of every gitignored output you cite."
    jq -nc --arg c "$ctx" '{hookSpecificOutput:{hookEventName:"SubagentStart", additionalContext:$c}}'
    exit 0
fi

# subagent-stop: mark, never delete.
[ $rc = 0 ] || exit 0
rm -f -- "$phys/.stopped"
printf '%s\n' "$now" > "$phys/.stopped"
[ -L "$phys/.stop-log" ] || printf '%s stop\n' "$now" >> "$phys/.stop-log"
exit 0
