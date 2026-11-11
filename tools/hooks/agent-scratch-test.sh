#!/usr/bin/env bash
# Pipe-tests for tools/hooks/agent-scratch.sh (TOOL46). Run from the repo root
# (or a worktree root). Prints one line per case; exits 1 if any line is FAIL.
#
# Everything happens in a fixture repo (git init) plus a worktree of it, made
# with mktemp under $AS_TEST_TMP and removed at exit; CLAUDE_PROJECT_DIR names
# the fixture, so the real target-agents/ is never touched.
H=$PWD/tools/hooks/agent-scratch.sh
fail=0
base=${AS_TEST_TMP:-}
if [ -z "$base" ] || [ ! -d "$base" ]; then
    echo "FAIL: no temp base (set AS_TEST_TMP to an existing dir on the data volume)"
    exit 1
fi
tmp=$(mktemp -d "${base%/}/as-test.XXXXXX") || { echo "FAIL: mktemp"; exit 1; }
trap 'rm -rf "$tmp"' EXIT
tmp=$(cd "$tmp" && pwd -P)
# $tmp may lie inside this repo (target/): keep git from finding it above $tmp.
export GIT_CEILING_DIRECTORIES=$tmp
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR NEWTOS_AGENT_SCRATCH_WARN_KB
F=$tmp/repo
O=$tmp/outside
mkdir -p "$F" "$O"
git init -q "$F"
git -C "$F" -c user.name=t -c user.email=t@t -c core.hooksPath=/nonexistent commit -q --allow-empty -m init
git -C "$F" worktree add -q "$F/.claude/worktrees/agent-w1" -b wt1 2>/dev/null
W=$F/.claude/worktrees/agent-w1
R=$F/target-agents

ok() { echo "ok: $1"; }
no() { echo "FAIL: $1"; fail=1; }
check() { if eval "$1"; then ok "$2"; else no "$2"; fi; }   # check <test expr> <description>
# run <mode> <json> [project dir]: stdout in $out, stderr in $err, exit in $rc.
run() {
    out=$(printf '%s' "$2" | CLAUDE_PROJECT_DIR=${3-$F} "$H" "$1" 2>"$tmp/err"); rc=$?
    err=$(cat "$tmp/err")
}
# clean <project dir> <id>...: the command mode.
clean() {
    local p=$1; shift
    out=$(CLAUDE_PROJECT_DIR=$p "$H" clean "$@" 2>"$tmp/err"); rc=$?
    err=$(cat "$tmp/err")
}
start() { run subagent-start "{\"hook_event_name\":\"SubagentStart\",\"agent_id\":$1}" "${2-$F}"; }
stop() { run subagent-stop "{\"hook_event_name\":\"SubagentStop\",\"agent_id\":$1}" "${2-$F}"; }
session() { out=$(printf '{}' | CLAUDE_PROJECT_DIR=$F "$H" session-start 2>/dev/null); rc=$?; msg=$(printf '%s' "$out" | jq -r .systemMessage 2>/dev/null); }
age() { perl -e '$t = time - $ARGV[0] * 60; utime $t, $t, $ARGV[1] or die' "$1" "$2"; }   # age <minutes> <file>
fixture() {   # fresh target-agents with two workers and a file outside
    rm -rf "$R" "$O"; mkdir -p "$R/agent-a1/clone/sub" "$R/agent-a2" "$O"
    echo x > "$R/agent-a1/clone/sub/f"; echo y > "$R/agent-a2/f"; echo z > "$O/keep"; echo k > "$F/keep"
}
intact() {   # the other worker, the outside dir and the repo file survive
    [ -f "$R/agent-a2/f" ] && [ -f "$O/keep" ] && [ -f "$F/keep" ]
}

# ── subagent-start names the directory ────────────────────────────────
rm -rf "$R"
start '"a1b2"'
ctx=$(printf '%s' "$out" | jq -r '.hookSpecificOutput.additionalContext' 2>/dev/null)
check '[ "$rc" = 0 ] && printf "%s" "$ctx" | grep -qF "$R/agent-a1b2/"' "start: context names target-agents/agent-<id>/"
check '[ "$(printf "%s" "$out" | jq -r .hookSpecificOutput.hookEventName)" = SubagentStart ]' "start: hookEventName"
check '[ ! -e "$R/agent-a1b2" ]' "start: makes no directory"
start '"agent-a1b2"'
check 'printf "%s" "$out" | grep -qF "$R/agent-a1b2/" && ! printf "%s" "$out" | grep -qF "agent-agent-"' "start: agent- prefix of the id normalised"
start '"a1"' "$W"
check 'printf "%s" "$out" | grep -qF "$R/agent-a1/"' "start: project dir a worktree -> main repo target-agents"

# ── subagent-stop marks, never deletes ────────────────────────────────
fixture; stop '"a1"'
check '[ "$rc" = 0 ] && [ -f "$R/agent-a1/clone/sub/f" ] && intact' "stop: the worker's dir and contents stay"
check '[ -f "$R/agent-a1/.stopped" ] && grep -qE "^[0-9]{4}-[0-9]{2}-[0-9]{2}T" "$R/agent-a1/.stopped"' "stop: writes the stop time into .stopped"
check 'grep -q " stop$" "$R/agent-a1/.stop-log" && [ -z "$out" ]' "stop: appends to .stop-log, prints nothing"
check '[ ! -e "$R/agent-a2/.stopped" ]' "stop: no marker in another worker's dir"
age 600 "$R/agent-a1/.stopped"; stop '"agent-a1"'
check '[ -n "$(find "$R/agent-a1/.stopped" -mmin -5)" ]' "stop again (agent- prefix): marker overwritten, mtime fresh"
# Waiting on its own background work: SubagentStop may fire while a cargo test
# still runs in the clone. The stop leaves the clone alone.
fixture; mkdir -p "$R/agent-a1/clone/target/debug"; echo run > "$R/agent-a1/clone/target/debug/in-use"
stop '"a1"'; stop '"a1"'
check '[ -f "$R/agent-a1/clone/target/debug/in-use" ] && intact' "stop while waiting on background work: clone survives"
# Resume: stop -> start -> the same id works on.
start '"a1"'
check '[ -f "$R/agent-a1/clone/sub/f" ] && [ -f "$R/agent-a1/clone/target/debug/in-use" ]' "resume: stop -> start keeps dir and contents"
check '[ ! -e "$R/agent-a1/.stopped" ] && [ "$(grep -c "" "$R/agent-a1/.stop-log")" = 3 ]' "resume: start removes the marker, logs it"
stop '"a9"'
check '[ "$rc" = 0 ] && [ ! -e "$R/agent-a9" ] && [ -z "$out" ]' "stop: no dir for the id: no-op, makes none"
fixture; run subagent-stop '{"hook_event_name":"SubagentStop","agent_type":""}'
check '[ "$rc" = 0 ] && [ -d "$R/agent-a1" ] && intact && [ -z "$out" ]' "stop: no agent_id (internal agent) no-op"
fixture; ln -s "$O" "$R/agent-evil"; stop '"evil"'
check '[ "$rc" = 0 ] && [ ! -e "$O/.stopped" ] && [ -L "$R/agent-evil" ]' "stop: symlinked worker dir: no marker written through it"
fixture; ln -s "$O/keep" "$R/agent-a1/.stopped"; ln -s "$O/log" "$R/agent-a1/.stop-log"; stop '"a1"'
check '[ "$(cat "$O/keep")" = z ] && [ ! -e "$O/log" ] && [ -f "$R/agent-a1/.stopped" ] && [ ! -L "$R/agent-a1/.stopped" ]' "stop: planted marker symlinks not followed"

# ── clean removes only its own directory ──────────────────────────────
fixture; ln -s "$O" "$R/agent-a1/clone/link-out"
clean "$F" a1
check '[ "$rc" = 0 ] && [ ! -e "$R/agent-a1" ] && intact' "clean: removes agent-a1 only"
check '[ -d "$R" ]' "clean: target-agents itself stays"
check 'printf "%s" "$out" | grep -q "removed .*agent-a1 ([0-9]* KiB)"' "clean: reports path and size"
check '[ -f "$O/keep" ]' "clean: symlink inside the worker dir not followed"
fixture; clean "$F" agent-a1
check '[ ! -e "$R/agent-a1" ] && intact' "clean: agent- prefixed id"
fixture; clean "$W" a1
check '[ ! -e "$R/agent-a1" ] && intact && [ -d "$W" ]' "clean: from a worktree project dir"
fixture; mkdir "$R/agent-a3"; clean "$F" a1 a3
check '[ "$rc" = 0 ] && [ ! -e "$R/agent-a1" ] && [ ! -e "$R/agent-a3" ] && intact' "clean: several ids"
fixture; clean "$F" a9
check '[ "$rc" = 0 ] && [ -d "$R/agent-a1" ] && intact' "clean: unknown id is a no-op"

# ── refusals: nothing removed ─────────────────────────────────────────
for bad in '' '..' '../outside' 'a1/../a2' 'a 1' '.' '*' 'a1;rm -rf x' '-rf' 'agent-' '/' "$O"; do
    fixture; clean "$F" "$bad"
    check '[ "$rc" = 1 ] && [ -d "$R/agent-a1" ] && intact && [ -d "$O" ]' "clean refuses id '$bad'"
done
for bad in '""' '".."' '"../outside"' '"a 1"' 'null' '7' '"-rf"'; do
    fixture; stop "$bad"
    check '[ "$rc" = 0 ] && [ -d "$R/agent-a1" ] && intact' "stop hook, id $bad: exit 0, nothing removed"
done
fixture; ln -s "$O" "$R/agent-evil"; clean "$F" evil
check '[ "$rc" = 1 ] && [ -L "$R/agent-evil" ] && intact && [ -d "$O" ]' "refuse: worker dir is a symlink outside"
check 'printf "%s" "$out" | grep -q symlink' "refuse: symlink reported"
fixture; ln -s "$R/agent-a2" "$R/agent-alias"; clean "$F" alias
check '[ -L "$R/agent-alias" ] && intact' "refuse: worker dir is a symlink to another worker"
fixture; echo f > "$R/agent-file"; clean "$F" file
check '[ "$rc" = 1 ] && [ -f "$R/agent-file" ] && intact' "refuse: worker entry is a file"
fixture; mv "$R" "$tmp/real-ta"; ln -s "$tmp/real-ta" "$R"; clean "$F" a1
check '[ "$rc" = 1 ] && [ -d "$tmp/real-ta/agent-a1" ] && [ -f "$tmp/real-ta/agent-a2/f" ]' "refuse: target-agents is a symlink"
rm -f "$R"; rm -rf "$tmp/real-ta"
fixture; clean "" a1
check '[ "$rc" = 1 ] && [ -d "$R/agent-a1" ] && intact' "refuse: CLAUDE_PROJECT_DIR empty"
fixture; clean "$O" a1
check '[ "$rc" = 1 ] && [ -d "$R/agent-a1" ] && intact' "refuse: project dir not a git repo"
fixture; clean "$F"
check '[ "$rc" = 1 ] && [ -d "$R/agent-a1" ] && intact' "refuse: clean without an id"

# ── session-start: 72 h expiry ────────────────────────────────────────
fixture; mkdir -p "$R/agent-old" "$R/agent-young" "$R/agent-nomark"
echo t > "$R/agent-old/.stopped"; age 4330 "$R/agent-old/.stopped"
echo t > "$R/agent-young/.stopped"; age 4310 "$R/agent-young/.stopped"
age 9000 "$R/agent-nomark"; echo n > "$R/agent-nomark/f"; age 9000 "$R/agent-nomark/f"
ln -s "$O" "$R/agent-link"
session
check '[ "$rc" = 0 ] && [ ! -e "$R/agent-old" ]' "session: last stop 72 h 10 min ago: removed"
check '[ -f "$R/agent-young/.stopped" ]' "session: last stop 71 h 50 min ago: kept"
check '[ -f "$R/agent-nomark/f" ]' "session: no marker (old files): never deleted"
check '[ -L "$R/agent-link" ] && [ -f "$O/keep" ] && intact && [ -d "$R/agent-a1" ]' "session: symlink and the others kept"
check 'printf "%s" "$msg" | grep -q "older than 72 h" && printf "%s" "$msg" | grep -q "removed .*agent-old"' "session: reports the expired removal"
fixture; mkdir -p "$R/agent-old"; echo t > "$R/agent-old/.stopped"; age 4330 "$R/agent-old/.stopped"; start '"old"'; session
check '[ -d "$R/agent-old" ]' "session: a resumed worker (marker removed at start) is not expired"

# ── session-start warning ─────────────────────────────────────────────
fixture; dd if=/dev/zero of="$R/agent-a2/big" bs=1024 count=300 2>/dev/null
session
check '[ "$rc" = 0 ] && [ -z "$out" ]' "session: below the default 20 GiB, nothing expired: silent"
out=$(printf '{}' | NEWTOS_AGENT_SCRATCH_WARN_KB=100 CLAUDE_PROJECT_DIR=$F "$H" session-start 2>/dev/null); rc=$?
msg=$(printf '%s' "$out" | jq -r .systemMessage 2>/dev/null)
check '[ "$rc" = 0 ] && printf "%s" "$msg" | grep -q "WARNING .*target-agents holds"' "session: over the threshold: warning"
check 'printf "%s" "$msg" | sed -n 2p | grep -q "agent-a2$"' "session: largest subdirectory first"
check 'printf "%s" "$msg" | grep -q "Without a stop marker.*agent-a1"' "session: lists the dirs without a marker"
check '[ "$(printf "%s" "$out" | jq -r .hookSpecificOutput.additionalContext)" = "$msg" ]' "session: same text as context for Claude"
check '[ -d "$R/agent-a1" ] && intact' "session: the warning removes nothing"
rm -rf "$R"; session
check '[ "$rc" = 0 ] && [ -z "$out" ]' "session: no target-agents: silent"
exit $fail
