#!/usr/bin/env bash
# adversarial pipe harness (own cases). Guard only reads JSON.
G=${G:-/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-aeebbf8f8af122b9d/tools/hooks/pin-guard.sh}
HERE=$(cd "$(dirname "$0")" && pwd)
B=$HERE/fx
rm -rf $B; mkdir -p $B; B=$(cd -P $B && pwd -P)
unset GIT_DIR GIT_WORK_TREE
gq() { git -c user.name=t -c user.email=t@t.invalid -c init.defaultBranch=master -c core.hooksPath=/dev/null "$@" >/dev/null 2>&1; }
P=$B/proj; E=$P/references/emutos; O=$P/references/emutos-oracle/1a7484ab; WT=$B/wt; S=$B/scratch
mkdir -p $P $E/bios $P/references/freemint $S
gq init -q $P; printf 'references/\n' > $P/.gitignore; echo x > $P/a; gq -C $P add -A; gq -C $P commit -q -m i
gq init -q $E; echo r > $E/README; echo c > $E/bios/k.c; gq -C $E add -A; gq -C $E commit -q -m p
mkdir -p $P/references/emutos-oracle; gq clone -q $E $O
gq -C $P worktree add -q -b wt $WT; mkdir -p $WT/references; ln -s $E $WT/references/emutos; ln -s $P/references/emutos-oracle $WT/references/emutos-oracle
gq clone -q $E $S/emutos
ln -s $E $S/x; ln -s $E/bios $S/xb; ln -s $P/references $S/refs; ln -s ../proj/references/emutos $S/rel
fail=0; n=0
# t want proj cwd cmd
t() { n=$((n+1)); jq -nc --arg c "$4" --arg d "$3" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$2 $G >/dev/null 2>&1; rc=$?
  if [ $rc = $1 ]; then echo "ok   rc=$rc: $4"; else echo "MISS rc=$rc want=$1 [cwd=${3#$B}]: $4"; fail=$((fail+1)); fi; }
w() { n=$((n+1)); jq -nc --arg t "$2" --arg p "$3" --arg d "$P" '{tool_name:$t,cwd:$d,tool_input:{file_path:$p,notebook_path:$p}}' | CLAUDE_PROJECT_DIR=$P $G >/dev/null 2>&1; rc=$?
  if [ $rc = $1 ]; then echo "ok   rc=$rc: $2 $3"; else echo "MISS rc=$rc want=$1: $2 $3"; fail=$((fail+1)); fi; }
b() { t 2 $P $P "$1"; }     # expect block, cwd project
a() { t 0 $P $P "$1"; }     # expect allow
. $HERE/${CASES:-cases.sh}
echo "TOTAL $n MISS $fail"
