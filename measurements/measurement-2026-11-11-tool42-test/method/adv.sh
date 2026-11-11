#!/usr/bin/env bash
# adv.sh <guard> : adversarial cases; format: want|cmd
G=$1; M=/Volumes/Daten/Development/PiRusTOS
run(){ want=$1; cmd=$2; dir=${3:-$M}
  jq -nc --arg c "$cmd" --arg d "$dir" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | CLAUDE_PROJECT_DIR=$M "$G" >/dev/null 2>&1; rc=$?
  [ $rc = $want ] && r=ok || r=MISMATCH; echo "$r got=$rc want=$want :: $cmd"; }
while IFS='|' read -r want cmd; do [ -z "$want" ] && continue; run "$want" "$(printf '%b' "$cmd")"; done <<'CASES'
2|git  commit-tree 4b825dc -m x
2|"git" commit-tree 4b825dc
2|'git' commit-tree 4b825dc
2|git --no-pager commit-tree 4b825dc
2|git -c x=y update-ref refs/heads/main abc
2|git -c user.name=x commit-tree abc
2|/usr/bin/git commit-tree abc
2|env git commit-tree abc
2|env FOO=1 git commit-tree abc
2|command git commit-tree abc
2|sudo git commit-tree abc
2|echo hi; git commit-tree abc
2|true && git commit-tree abc
2|false || git commit-tree abc
2|echo a\ngit commit-tree abc
2|echo a | git commit-tree abc
2|(git commit-tree abc)
2|( cd /var && git commit-tree abc )
2|{ git commit-tree abc; }
2|x=$(git commit-tree abc)
2|echo `git commit-tree abc`
2|git update-ref HEAD abc
2|git update-ref main abc
2|git update-ref -d refs/heads/x
2|git update-ref --stdin < f
2|git update-ref -m msg refs/heads/main abc
2|git update-ref --no-deref HEAD abc
2|git hash-object -w -t commit f
2|git hash-object -t commit -w f
2|git hash-object -w --type=commit f
2|git hash-object -wt commit f
2|git mktree < f
2|git fast-import < f
2|git replace a b
2|git symbolic-ref HEAD refs/heads/x
2|git filter-branch --all
2|git filter-repo --force
2|git-filter-repo --force
2|git -C /Volumes/Daten/Development/PiRusTOS commit-tree abc
2|git --git-dir=/Volumes/Daten/Development/PiRusTOS/.git commit-tree abc
2|GIT_DIR=/Volumes/Daten/Development/PiRusTOS/.git git commit-tree abc
2|export GIT_DIR=/Volumes/Daten/Development/PiRusTOS/.git; git commit-tree abc
2|cd /Volumes/Daten/Development/PiRusTOS && git commit-tree abc
2|cd /Volumes/Daten/Development/PiRusTOS; git commit-tree abc
2|git commit-tree $(git write-tree) -m x
2|git commit-tree abc <<'EOF2'\nmsg\nEOF2
2|git\tcommit-tree abc
2|git -C . commit-tree abc
2|git -C "$CLAUDE_PROJECT_DIR" commit-tree abc
2|git -C $HOME/nonexist commit-tree abc
2|git commit-tree\\\n abc
2|xargs git commit-tree
2|nohup git commit-tree abc
2|time git commit-tree abc
2|git commit-tr\\ee abc
2|git "commit-tree" abc
2|git commit-"tree" abc
2|g=git; $g commit-tree abc
2|git update-ref  "refs/heads/main" abc
2|git update-ref refs/remotes/origin/main abc
0|git update-ref refs/tags/x abc
0|git update-ref -d refs/tags/x
0|git hash-object -w file
0|git hash-object -w --stdin
0|git hash-object file
0|git update-index --cacheinfo 100644,abc,f
0|git branch -D x
0|git commit -m "mention commit-tree and update-ref refs/heads/main"
0|git commit -F - <<'EOF2'\nUse git commit-tree and git update-ref refs/heads/main here\nEOF2
0|git commit -m "$(cat <<'EOF2'\nfoo git commit-tree x\nEOF2\n)"
0|echo "git commit-tree abc"
0|echo git commit-tree abc
0|grep -r commit-tree docs
0|# git commit-tree abc
0|git log --grep commit-tree
0|git status
0|git rev-parse HEAD
0|git write-tree
0|git symbolic-ref HEAD
0|git symbolic-ref --short HEAD
0|git replace -l
0|git show-ref
0|git for-each-ref refs/heads
0|git notes add -m x
0|git tag x
0|git stash list
0|cat commit-tree.txt
0|git cherry-pick abc
0|git rebase main
0|git branch -f x abc
0|cargo xtask test
CASES
