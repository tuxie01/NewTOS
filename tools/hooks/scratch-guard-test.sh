#!/usr/bin/env bash
# Pipe-tests for tools/hooks/scratch-guard.sh (TOOL46). Run from the repo root
# (or a worktree root). Prints one line per case; exits 1 if any line is FAIL.
# The guard only reads its JSON input: no file is made or touched.
G=$PWD/tools/hooks/scratch-guard.sh
uid=$(id -u)
S=/private/tmp/claude-$uid/-proj/sess/scratchpad   # under the scratch root
TA=$PWD/target-agents/agent-a0123456789abcdef
fail=0

# b <command> <want exit> <description>: a Bash call, CLAUDE_JOB_DIR set.
b() {
    jq -nc --arg c "$1" '{tool_name:"Bash",tool_input:{command:$c}}' | CLAUDE_JOB_DIR=/x "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$2" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $2): $3"
}
# w <tool> <file_path> <want exit> <description>
w() {
    jq -nc --arg t "$1" --arg p "$2" '{tool_name:$t,tool_input:{file_path:$p}}' | "$G" >/dev/null 2>&1
    rc=$?
    if [ "$rc" = "$3" ]; then r=ok; else r=FAIL; fail=1; fi
    echo "$r exit $rc (want $3): $4"
}

# ── the cases before TOOL46, unchanged ────────────────────────────────
b 'ls /tmp/foo' 2 "shared /tmp path"
b 'cp x /private/tmp/foo/bar' 2 "shared /private/tmp path"
b 'ls /tmp' 2 "bare /tmp"
b "ls $S/x" 0 "session scratch root (/private/tmp)"
b "ls /tmp/claude-$uid/x" 0 "session scratch root (/tmp)"
b "cp a /tmp/claude-$uid" 0 "the scratch root itself"
b 'ls ./tmp/x' 0 "relative ./tmp"
b 'ls target/tmp/x' 0 "tmp below another dir"
b 'echo hi' 0 "no path"
r=$(jq -nc '{tool_name:"Bash",tool_input:{command:"mkdir -p $CLAUDE_JOB_DIR/tmp"}}' | env -u CLAUDE_JOB_DIR "$G" >/dev/null 2>&1; echo $?)
if [ "$r" = 2 ]; then echo "ok exit 2 (want 2): \$CLAUDE_JOB_DIR while empty"; else echo "FAIL exit $r (want 2): \$CLAUDE_JOB_DIR while empty"; fail=1; fi
b 'mkdir -p ${CLAUDE_JOB_DIR}/x' 0 "\${CLAUDE_JOB_DIR} while set"
w Write /tmp/x.txt 2 "Write into the shared /tmp"
w Edit /private/tmp/y/z.rs 2 "Edit in the shared /private/tmp"
w Write "$S/notes.txt" 0 "Write into the scratch root"
w Write "$PWD/docs/x.md" 0 "Write into the repo"
w Read /tmp/x.txt 0 "other tool: not checked"

# ── TOOL46: target-agents allowed, bulk not in the scratch root ───────
b "git clone -q --shared $PWD $TA/clone" 0 "clone into target-agents"
b "mkdir -p $TA && git clone -q $PWD $TA/c && cd $TA/c && CARGO_TARGET_DIR=$TA/t cargo build" 0 "clone + CARGO_TARGET_DIR in target-agents"
b "cargo build --target-dir $TA/t" 0 "--target-dir in target-agents"
b "cargo build --target-dir=$TA/t" 0 "--target-dir= in target-agents"
b "rm -rf $TA" 0 "remove in target-agents"
b "git clone -q $PWD $S/clone" 2 "clone into the scratch root"
b "git clone -q $PWD /tmp/claude-$uid/x/clone" 2 "clone into the scratch root (/tmp form)"
b "git -C $PWD clone -q . $S/c" 2 "git -C dir clone into the scratch root"
b "git -c core.x=1 clone $PWD \"$S/c\"" 2 "git -c clone, quoted dest"
b "cd $S && git clone -q $PWD c" 0 "clone after cd into the scratch root (relative dest, not seen: text match)"
b "cd $S; git clone -q $PWD $S/c2" 2 "clone after ; into the scratch root"
b "CARGO_TARGET_DIR=$S/t cargo build" 2 "CARGO_TARGET_DIR in the scratch root"
b "export CARGO_TARGET_DIR=\"$S/t\"" 2 "export CARGO_TARGET_DIR (quoted) in the scratch root"
b "cargo test --target-dir $S/t" 2 "--target-dir in the scratch root"
b "cargo test --target-dir=/tmp/claude-$uid/t" 2 "--target-dir= in the scratch root (/tmp form)"
b "git clone -q $PWD $TA/c && ls $S" 0 "clone into target-agents, scratch named in another segment"
b "cp out.log $S/out.log && git log -n1" 0 "small scratch file, no clone"
b "echo 'git clone' > $S/note.txt" 0 "word clone in a quoted string (no git clone command)"
b "cargo build && ls $S" 0 "cargo build without a target dir"
b "git clone -q $PWD /tmp/other" 2 "clone into the shared /tmp (old rule)"
# Documented in the guard's comment block: pinned so a change is noticed.
b "echo git clone > $S/note.txt" 2 "known false positive: 'git clone' as echo text with a scratch path"
b "git commit -q -m 'git clone docs' && ls $S" 0 "commit message text, scratch path in another segment: passes"
b "git worktree add $S/wt" 0 "known gap: git worktree add into the scratch root"
b "CARGO_TARGET_DIR=\$X cargo build" 0 "known gap: target dir through a variable"
# TOOL50: paths are matched case folded (/TMP is /tmp on APFS).
b 'ls /TMP/foo' 2 "TOOL50: shared /tmp in upper case"
b 'cp x /Private/Tmp/foo' 2 "TOOL50: shared /private/tmp in mixed case"
b "ls /TMP/claude-$uid/x" 0 "TOOL50: the scratch root in upper case"
b "git clone -q $PWD /Private/TMP/claude-$uid/x/clone" 2 "TOOL50: clone into the scratch root, mixed case"
b "CARGO_TARGET_DIR=/TMP/claude-$uid/t cargo build" 2 "TOOL50: CARGO_TARGET_DIR in the scratch root, upper case"
w Write /TMP/x.txt 2 "TOOL50: Write into the shared /tmp, upper case"
w Write "/TMP/claude-$uid/notes.txt" 0 "TOOL50: Write into the scratch root, upper case"
exit $fail
