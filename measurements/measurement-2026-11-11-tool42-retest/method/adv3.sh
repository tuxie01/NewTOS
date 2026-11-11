#!/usr/bin/env bash
# adv3.sh <guard> <scratch>: project-dir spelling variants, cwd field absent / odd, hook PWD fallback.
G=$1; S=$2; M=/Volumes/Daten/Development/PiRusTOS; W=$M/.claude/worktrees/agent-a15921e5b605761cb
# t <want> <project dir> <hook pwd> <json cwd or NONE> <cmd>
t(){ want=$1; pd=$2; hp=$3; cw=$4; cmd=$5
  if [ "$cw" = NONE ]; then j=$(jq -nc --arg c "$cmd" '{tool_name:"Bash",tool_input:{command:$c}}'); else j=$(jq -nc --arg c "$cmd" --arg d "$cw" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}'); fi
  (cd "$hp" && printf '%s' "$j" | CLAUDE_PROJECT_DIR=$pd "$G" >/dev/null 2>&1); rc=$?
  [ $rc = $want ] && r=ok || r=MISMATCH; echo "$r got=$rc want=$want proj=$pd hookpwd=${hp#$S/} cwd=${cw#$S/} :: $cmd"; }
C=$S/clone2
for pd in "$M" "$M/" "/$M" "$S/lnk" "$S/lnk/" "$W" "$S/lnkwt" "$M/tools/.."; do
  t 2 "$pd" "$M" NONE 'git commit-tree abc'
  t 2 "$pd" "$M" "$M" 'git commit-tree abc'
  t 2 "$pd" "$C" "$M" 'git commit-tree abc'
  t 2 "$pd" "$C" "$W" 'git commit-tree abc'
  t 2 "$pd" "$C" "$S/lnk" 'git update-ref refs/heads/main abc'
  t 2 "$pd" "$C" "$C" "git -C $pd commit-tree abc"
  t 2 "$pd" "$C" "$C" 'git -C $CLAUDE_PROJECT_DIR commit-tree abc'
  t 0 "$pd" "$M" "$C" 'git commit-tree abc'
  t 0 "$pd" "$C" "$C" 'git commit-tree abc'
  t 0 "$pd" "$C" NONE 'git commit-tree abc'
  t 0 "$pd" "$M" "$C/" 'git commit-tree abc'
  t 0 "$pd" "$M" "$C//" 'git commit-tree abc'
done
# cwd absent: falls back to hook pwd
t 2 "$M" "$M" NONE 'git update-ref HEAD abc'
t 0 "$M" "$C" NONE 'git update-ref refs/heads/x abc'
# cwd nonexistent / empty / relative
t 2 "$M" "$M" "/nonexistent/dir" 'git commit-tree abc'
t 2 "$M" "$M" "" 'git commit-tree abc'
t 2 "$M" "$M" "relative/dir" 'git commit-tree abc'
t 2 "$M" "$C" "$M/nonexistent" 'git commit-tree abc'
# project dir unknown / nonexistent / empty
t 2 "/nonexistent" "$M" "$M" 'git commit-tree abc'
t 2 "" "$M" "$M" 'git commit-tree abc'
t 2 "" "$M" "$C" 'git commit-tree abc'
