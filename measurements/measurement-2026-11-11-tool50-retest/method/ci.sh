#!/usr/bin/env bash
# Check 3: case-insensitive helper in no-verify-guard and scratch-guard. Pipe JSON only.
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-aeebbf8f8af122b9d
M=/Volumes/Daten/Development/PiRusTOS
SC=$M/target-agents/agent-a8cc259b29d497846
export GIT_OPTIONAL_LOCKS=0
n=0; bad=0
nv() { # want cwd cmd
  n=$((n+1))
  jq -nc --arg c "$3" --arg d "$2" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$W $W/tools/hooks/no-verify-guard.sh >/dev/null 2>&1; rc=$?
  if [ $rc = $1 ]; then echo "ok   nv rc=$rc: $3"; else echo "MISS nv rc=$rc want=$1 [cwd=$2]: $3"; bad=$((bad+1)); fi
}
sg() { # want cmd
  n=$((n+1))
  jq -nc --arg c "$2" '{tool_name:"Bash",tool_input:{command:$c}}' | $W/tools/hooks/scratch-guard.sh >/dev/null 2>&1; rc=$?
  if [ $rc = $1 ]; then echo "ok   sg rc=$rc: $2"; else echo "MISS sg rc=$rc want=$1: $2"; bad=$((bad+1)); fi
}
sgw() { # want tool path
  n=$((n+1))
  jq -nc --arg t "$2" --arg p "$3" '{tool_name:$t,tool_input:{file_path:$p}}' | $W/tools/hooks/scratch-guard.sh >/dev/null 2>&1; rc=$?
  if [ $rc = $1 ]; then echo "ok   sgw rc=$rc: $2 $3"; else echo "MISS sgw rc=$rc want=$1: $2 $3"; bad=$((bad+1)); fi
}
L=/volumes/daten/development/pirustos
U=/VOLUMES/DATEN/DEVELOPMENT/PIRUSTOS
T=/Volumes/Daten/Development/Pirustos
# main repo target, other letter cases: blocked
for r in $M $L $U $T; do
  nv 2 $M "git -C $r commit-tree HEAD^{tree} -m x"
  nv 2 $M "git -C $r update-ref refs/heads/main HEAD"
  nv 2 $M "git -C $r symbolic-ref HEAD refs/heads/x"
  nv 2 $M "git -C $r merge --squash foo"
  nv 2 $M "git -C $r commit --no-verify -m x"
  nv 2 $M "git -C $r hash-object -t commit x"
  nv 2 $M "git --git-dir=$r/.git commit-tree HEAD^{tree}"
  nv 2 $M "cd $r && git commit-tree HEAD^{tree}"
  nv 2 $r "git commit-tree HEAD^{tree}"
done
nv 2 $L "git update-ref refs/heads/main HEAD"
nv 2 $M "git -C /Volumes/Daten/Development/PiRusTOS/.claude/worktrees/../../.git commit-tree x"
# scratch clone (target-agents), other letter cases: allowed (plumbing in a scratch clone)
for r in $SC /VOLUMES/DATEN/DEVELOPMENT/PIRUSTOS/TARGET-AGENTS/AGENT-A8CC259B29D497846; do
  nv 0 $M "git -C $r/fx commit-tree HEAD^{tree} -m x"
  nv 0 $M "git -C $r/fx update-ref refs/heads/x HEAD"
done
# scratch-guard: case variants
sg 2 "echo hi > /TMP/x"
sg 2 "touch /Tmp/x"
sg 2 "touch /Private/Tmp/x"
sg 2 "touch /PRIVATE/TMP/x"
sg 2 "touch /private/TMP/x"
sg 2 "mkdir /tmp/x"
sg 2 "cp a /private/tmp/b"
sg 2 "touch /TMP/CLAUDE-501/../x"
sg 0 "touch /private/tmp/claude-501/x"
sg 0 "touch /TMP/claude-501/x"
sg 0 "touch /Private/Tmp/Claude-501/x"
sg 2 "touch /tmp/claude-502/x"
sg 2 "CARGO_TARGET_DIR=/private/tmp/claude-501/t cargo build"
sg 2 "CARGO_TARGET_DIR=/PRIVATE/TMP/CLAUDE-501/t cargo build"
sg 2 "git clone x /Private/Tmp/claude-501/c"
sg 2 "cargo build --target-dir /Private/Tmp/claude-501/t"
sgw 2 Write /TMP/x
sgw 2 Edit /Private/Tmp/x
sgw 2 Write /PRIVATE/TMP/x
sgw 0 Write /private/tmp/claude-501/x
sgw 0 Write /Private/Tmp/Claude-501/x
sgw 0 Write $SC/x
# ordinary commands, no new false positives
for c in "ls -la" "git status" "git log --oneline -5" "git diff --stat" "cargo xtask test" "cargo build --release" "grep -rn tmpfile src" "echo /Volumes/Daten/Tmp" "cat $M/CLAUDE.md" "ls $M/target-agents" "git commit -m fix" "git add -A" "git worktree list" "sed -n 1,5p docs/STATUS.md" "echo temporary" "ls /Users/x/Tmp" "git branch --list" "make -C $SC" "ls $M/references" "git -C $M log -1" "git -C $L log -1" "git -C $U status --short" "cd $M && git status" "git -C $T show HEAD" "git -C $M rev-parse HEAD"; do
  nv 0 $M "$c"; sg 0 "$c"
done
echo "TOTAL $n MISS $bad"
