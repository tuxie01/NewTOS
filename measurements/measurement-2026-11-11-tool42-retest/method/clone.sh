#!/usr/bin/env bash
# clone.sh <guard> <scratch>: TOOL40 setup chain in a scratch clone, project dir = real main repo
G=$1; S=$2; M=/Volumes/Daten/Development/PiRusTOS; C=$S/clone2; bad=0
for v in "write-tree" "commit-tree 4b825dc -m x" "mktree < f" "update-ref refs/heads/main abc" "update-ref HEAD abc" "hash-object -w -t commit f" "fast-import < f" "filter-branch --all" "symbolic-ref HEAD refs/heads/x" "replace a b" "merge --ff-only x" "merge feature" "pull $M" "config core.hooksPath tools/git-hooks"; do
 for pre in "@cwd" "git -C $C" "cd $C &&" "cd $C;" "pushd $C;" "git --git-dir=$C/.git" "GIT_DIR=$C/.git git"; do
  case "$pre" in @cwd) cmd="git $v"; cwd=$C;; "git -C"*|"git --git-dir"*|GIT_DIR*) cmd="${pre/#git /git }"; case "$pre" in git*) cmd="$pre $v";; *) cmd="$pre $v";; esac; cwd=$M;; *) cmd="$pre git $v"; cwd=$M;; esac
  jq -nc --arg c "$cmd" --arg d "$cwd" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$M "$G" >/dev/null 2>&1; rc=$?
  [ $rc = 0 ] && r=ok || { r=BLOCKED; bad=$((bad+1)); }; echo "$r rc=$rc cwd=${cwd#$M} :: ${cmd//$S\//}"
 done; done; echo "blocked: $bad"
