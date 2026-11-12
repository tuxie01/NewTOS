#!/usr/bin/env bash
# Pipe-tests for tools/hooks/role-guard.sh. Run from the repo root (or a
# worktree root). Builds a throw-away repo with a branch worktree, an audit
# worktree, a detached worktree and a scratch clone under $ROLE_GUARD_TMP
# (default: target-agents/role-guard-test.$$ of the current repo), runs the
# guard against it and removes it. Prints one line per case; exits 1 if any
# line is FAIL. The guard only reads its JSON input: no checked file is made.
G=$PWD/tools/hooks/role-guard.sh
T=${ROLE_GUARD_TMP:-$PWD/target-agents/role-guard-test.$$}
fail=0

rm -rf "$T"; mkdir -p "$T" || exit 1
M=$T/main
git init -q -b main "$M" || exit 1
mkdir -p "$M/docs/archive" "$M/docs/api" "$M/TASK" "$M/kernel/src" "$M/measurements" "$M/tosdump/cases"
: > "$M/docs/STATUS.md"; : > "$M/kernel/src/lib.rs"
git -C "$M" add -A >/dev/null
git -C "$M" -c user.name=t -c user.email=t@t commit -q -m init
mkdir -p "$M/target-agents/agent-dev" "$M/target-agents/agent-aud" "$M/target-agents/agent-tst"
git -C "$M" worktree add -q "$M/target-agents/agent-dev/wt" -b xbios99
git -C "$M" worktree add -q "$M/target-agents/agent-aud/wt" -b audit-xbios
git -C "$M" worktree add -q --detach "$M/target-agents/agent-tst/wt"
git clone -q "$M" "$T/clone"
DEV=$M/target-agents/agent-dev/wt
AUD=$M/target-agents/agent-aud/wt
DET=$M/target-agents/agent-tst/wt

# c <role> <tool> <path> <want exit> <description>; role "-" = no agent_type.
c() {
    if [ "$1" = - ]; then
        j=$(jq -nc --arg t "$2" --arg p "$3" '{tool_name:$t,tool_input:{file_path:$p}}')
    else
        j=$(jq -nc --arg a "$1" --arg t "$2" --arg p "$3" '{agent_type:$a,tool_name:$t,tool_input:{file_path:$p}}')
    fi
    printf '%s' "$j" | CLAUDE_PROJECT_DIR=$M "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$4" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $4): $1: $5"
}

# ── lead ───────────────────────────────────────────────────────────────
c lead Write "$M/TASK/XBIOS99.md" 0 "a ticket"
c lead Edit "$M/docs/STATUS.md" 0 "docs/STATUS.md"
c lead Write "$M/docs/archive/status-log.md" 0 "the archive"
c lead Edit "$M/docs/MEMORY.md" 0 "docs/MEMORY.md"
c lead Edit "$M/CLAUDE.md" 0 "CLAUDE.md"
c lead Edit "$M/kernel/src/lib.rs" 2 "product code"
c lead Write "$M/docs/api/x.md" 2 "a spec file"
c lead Write "$M/TASK/XBIOS99-STATUS-DONE.MD" 2 "a DONE receipt"
c lead Write "$M/TASK/XBIOS99-STATUS-DONE-2.MD" 2 "a numbered DONE receipt"
c lead Write "$M/TASK/XBIOS99-STATUS-UNFINISHED.MD" 0 "an UNFINISHED receipt"
c lead Edit "$DEV/kernel/src/lib.rs" 2 "a developer's worktree"
c lead Write "$M/.claude/agents/lead.md" 2 "its own brief"
c - Edit "$M/kernel/src/lib.rs" 2 "no agent_type counts as lead"
c - Write "$M/TASK/A1.md" 0 "no agent_type, a ticket"
c lead Write "$M/target-agents/lead-x/msg.txt" 0 "scratch under target-agents/"

# ── developer ──────────────────────────────────────────────────────────
c developer Edit "$DEV/kernel/src/lib.rs" 0 "product code on its branch"
c developer Write "$DEV/TASK/XBIOS99-STATUS-UNFINISHED.MD" 0 "its receipt"
c developer Edit "$DEV/TASK/XBIOS99.md" 0 "line 3 of its ticket"
c developer Edit "$DEV/docs/STATUS.md" 2 "docs/STATUS.md"
c developer Write "$DEV/docs/archive/x.md" 2 "the archive"
c developer Write "$DEV/TASK/XBIOS99-STATUS-DONE.MD" 2 "a DONE receipt"
c developer Edit "$M/kernel/src/lib.rs" 2 "the main tree"
c developer Edit "$DET/kernel/src/lib.rs" 2 "a detached tree"
c developer Write "$DEV/target/x.txt" 0 "its target/"
c developer Write "$M/target-agents/agent-dev/notes.txt" 0 "its scratch directory"

# ── researcher ─────────────────────────────────────────────────────────
c researcher Write "$DEV/docs/api/x.md" 0 "a spec on a branch"
c researcher Edit "$DEV/kernel/src/lib.rs" 2 "product code"
c researcher Edit "$DEV/docs/STATUS.md" 2 "docs/STATUS.md"
c researcher Write "$M/docs/api/x.md" 2 "the main tree"

# ── tester ─────────────────────────────────────────────────────────────
c tester Write "$M/measurements/measurement-2026-11-12-xbios99/RUNINFO.TXT" 0 "its run directory"
c tester Write "$M/measurements/README.md" 2 "measurements/ outside a run directory"
c tester Write "$M/kernel/src/selftest_x.rs" 2 "product code in the main tree"
c tester Write "$DET/kernel/src/selftest_x.rs" 0 "a temporary selftest in its detached tree"
c tester Write "$DEV/kernel/src/selftest_x.rs" 2 "the developer's worktree"
c tester Write "$M/TASK/XBIOS99-STATUS-DONE.MD" 2 "a DONE receipt by hand"

# ── auditor ────────────────────────────────────────────────────────────
c auditor Write "$AUD/docs/audits/2026-11-12-xbios.md" 0 "its report"
c auditor Write "$AUD/tosdump/cases/xbios-r9.tsv" 0 "a case table"
c auditor Write "$AUD/tosdump/reference/EMUTOS-1a7484ab-X.BIN" 0 "a reference record"
c auditor Write "$AUD/measurements/measurement-2026-11-12-xbios-audit/RUNINFO.TXT" 0 "its run folder"
c auditor Write "$AUD/kernel/src/lib.rs" 2 "product code on its branch"
c auditor Write "$DET/kernel/src/probe.rs" 0 "a probe in the frozen tree"
c auditor Write "$DEV/docs/audits/x.md" 2 "another worker's branch"
c auditor Write "$M/docs/audits/x.md" 2 "the main tree"

# ── everyone ───────────────────────────────────────────────────────────
c developer Write "$T/clone/kernel/src/lib.rs" 0 "a scratch clone (another common dir)"
c developer Write "$T/outside.txt" 0 "inside the test dir: the outer repo decides"
c Explore Write "$M/TASK/A1.md" 2 "an unknown role"
c developer Read "$M/kernel/src/lib.rs" 0 "other tool: not checked"
c developer Write "$DEV/task/xbios99-status-done.md" 2 "case folded path: a DONE receipt in lower case"
r=$(jq -nc --arg p "$M/TASK/A1.md" '{agent_type:"lead",tool_name:"Write",tool_input:{file_path:$p}}' | CLAUDE_PROJECT_DIR=/nonexistent "$G" >/dev/null 2>&1; echo $?)
if [ "$r" = 2 ]; then echo "ok exit 2 (want 2): main repo not found blocks"; else echo "FAIL exit $r (want 2): main repo not found blocks"; fail=1; fi

git -C "$M" worktree prune >/dev/null 2>&1
rm -rf "$T"
exit $fail
