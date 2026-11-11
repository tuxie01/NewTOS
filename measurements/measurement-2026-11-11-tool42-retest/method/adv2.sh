#!/usr/bin/env bash
# adv2.sh <guard> <scratch> : target-repo-resolution adversarial cases.
# case line: want|cwd|HOME-override-or-"-"|command   ; placeholders:
# @M main repo, @W worktree (agent-a15921..), @W2 other worktree, @L symlink->main, @LW symlink->worktree,
# @C scratch clone (main checked out), @H fake HOME (repo->main, clone->@C)
G=$1; S=$2; M=/Volumes/Daten/Development/PiRusTOS
W=$M/.claude/worktrees/agent-a15921e5b605761cb; W2=$M/.claude/worktrees/agent-a823a57e109be35e1
sub(){ printf '%s' "$1" | sed -e "s#@W2#$W2#g" -e "s#@LW#$S/lnkwt#g" -e "s#@W#$W#g" -e "s#@M#$M#g" -e "s#@L#$S/lnk#g" -e "s#@C#$S/clone2#g" -e "s#@H#$S/fakehome#g"; }
bad=0
while IFS='|' read -r want cwd hm cmd; do
  [ -z "$want" ] && continue
  cmd=$(sub "$cmd"); cwd=$(sub "$cwd"); h=$HOME; [ "$hm" != "-" ] && h=$(sub "$hm")
  jq -nc --arg c "$cmd" --arg d "$cwd" '{tool_name:"Bash",cwd:$d,tool_input:{command:$c}}' | HOME=$h CLAUDE_PROJECT_DIR=$M "$G" >/dev/null 2>&1; rc=$?
  if [ $rc = $want ]; then r=ok; else r=MISMATCH; bad=$((bad+1)); fi
  echo "$r got=$rc want=$want cwd=${cwd#$M} home=$hm :: $cmd"
done <<'CASES'
2|@M|-|git -C $CLAUDE_PROJECT_DIR commit-tree abc
2|@M|-|git -C ${CLAUDE_PROJECT_DIR} commit-tree abc
2|@M|-|git -C "$CLAUDE_PROJECT_DIR" commit-tree abc
2|@M|-|git -C "$CLAUDE_PROJECT_DIR/" commit-tree abc
2|@M|-|git -C $CLAUDE_PROJECT_DIR// commit-tree abc
2|@M|-|git -C $CLAUDE_PROJECT_DIR/./ commit-tree abc
2|@M|-|git -C $CLAUDE_PROJECT_DIR/tools/.. commit-tree abc
2|@M|-|git -C $CLAUDE_PROJECT_DIR/tools/../ commit-tree abc
2|@M|-|git -C $CLAUDE_PROJECT_DIR/tools/../../PiRusTOS commit-tree abc
2|@M|-|git -C @M//tools/.. commit-tree abc
2|@M|-|git -C //@M commit-tree abc
2|@M|-|git -C @M/./tools commit-tree abc
2|@M|-|git -C @L commit-tree abc
2|@M|-|git -C @L/ commit-tree abc
2|@M|-|git -C @L/tools commit-tree abc
2|@M|-|cd @L && git commit-tree abc
2|@M|-|cd @L/tools; git commit-tree abc
2|@C|-|cd @L && git commit-tree abc
2|@C|-|git -C @LW commit-tree abc
2|@C|-|git -C @LW/tools update-ref refs/heads/main abc
2|@C|-|git -C @W commit-tree abc
2|@C|-|git -C @W2 commit-tree abc
2|@C|-|cd @W2 && git commit-tree abc
2|@C|-|cd / ; cd $CLAUDE_PROJECT_DIR && git commit-tree abc
2|@C|-|cd /; cd ${CLAUDE_PROJECT_DIR} && git commit-tree abc
2|@C|-|cd "$CLAUDE_PROJECT_DIR" && git commit-tree abc
2|@C|-|cd /var && cd @M && git commit-tree abc
2|@C|-|pushd @M >/dev/null; git commit-tree abc
2|@C|-|pushd $CLAUDE_PROJECT_DIR; git commit-tree abc
2|@C|-|pushd @L; git commit-tree abc
2|@M|-|pushd @C; popd; git commit-tree abc
2|@C|-|pushd @C; pushd @M; git commit-tree abc
2|@C|-|git --git-dir=$CLAUDE_PROJECT_DIR/.git commit-tree abc
2|@C|-|git --git-dir=${CLAUDE_PROJECT_DIR}/.git commit-tree abc
2|@C|-|git --git-dir "$CLAUDE_PROJECT_DIR/.git" commit-tree abc
2|@C|-|git --git-dir=@L/.git commit-tree abc
2|@C|-|git --git-dir=@M/.git/ commit-tree abc
2|@C|-|git --git-dir=@M/.git/. commit-tree abc
2|@C|-|git --git-dir=@M//.git commit-tree abc
2|@C|-|GIT_DIR=$CLAUDE_PROJECT_DIR/.git git commit-tree abc
2|@C|-|GIT_DIR=@L/.git git update-ref HEAD abc
2|@C|-|export GIT_DIR=$CLAUDE_PROJECT_DIR/.git; git commit-tree abc
2|@C|-|export GIT_DIR=@M/.git && git commit-tree abc
2|@C|-|GIT_DIR=@M/.git GIT_WORK_TREE=@C git commit-tree abc
2|@C|-|GIT_DIR=@M/.git GIT_WORK_TREE=@M git commit-tree abc
2|@C|-|git --git-dir=@M/.git --work-tree=@C commit-tree abc
2|@C|-|GIT_WORK_TREE=@M git commit-tree abc
2|@C|-|git --work-tree=@M commit-tree abc
2|@C|-|git -C @C --git-dir=@M/.git commit-tree abc
2|@C|-|git --git-dir=@M/.git -C @C commit-tree abc
2|@C|-|git -C @C -C @M commit-tree abc
2|@C|-|git -C @C -C @M/tools commit-tree abc
2|@C|-|git -C ~/x commit-tree abc
2|@C|@H|git -C ~/repo commit-tree abc
2|@C|@H|git -C $HOME/repo commit-tree abc
2|@C|@H|git -C ${HOME}/repo commit-tree abc
2|@C|@H|git -C "$HOME/repo" commit-tree abc
2|@C|@H|git -C ~/repo/ commit-tree abc
2|@C|@H|git -C ~/repo/tools/.. commit-tree abc
2|@C|@H|cd ~/repo && git commit-tree abc
2|@C|@H|cd $HOME/repo && git commit-tree abc
2|@C|@H|cd ~ && cd repo && git commit-tree abc
2|@C|@H|cd ~; git -C repo commit-tree abc
2|@C|@H|git --git-dir=~/repo/.git commit-tree abc
2|@C|@H|git --git-dir=$HOME/repo/.git commit-tree abc
2|@C|-|git -C ~/../../Volumes/Daten/Development/PiRusTOS commit-tree abc
2|@C|-|git -C $HOME/../../Volumes/Daten/Development/PiRusTOS commit-tree abc
2|@M|-|cd tools && git commit-tree abc
2|@M|-|cd tools; cd ..; git commit-tree abc
2|@M|-|cd .claude/worktrees/agent-a15921e5b605761cb && git commit-tree abc
2|@W|-|git commit-tree abc
2|@W2|-|git commit-tree abc
2|@LW|-|git commit-tree abc
2|@L|-|git commit-tree abc
2|@M/tools|-|git update-ref refs/heads/main abc
2|@M|-|cd @C; cd - ; git commit-tree abc
2|@C|-|cd @M ; ( cd @C ); git commit-tree abc
2|@C|-|cd @M ; x=$(cd @C); git commit-tree abc
2|@C|-|git -C $CLAUDE_PROJECT_DIRX commit-tree abc
2|@C|-|git -C $FOO commit-tree abc
2|@C|-|git -C @M/nonexistent commit-tree abc
2|@C|-|cd /nonexistent && git commit-tree abc
2|@C|-|git -C @C/../@M commit-tree abc
0|@C|-|git -C @C commit-tree abc
0|@C|-|git -C @C/ commit-tree abc
0|@C|-|git -C @C//. commit-tree abc
0|@C|@H|git -C ~/clone commit-tree abc
0|@C|@H|git -C $HOME/clone update-ref refs/heads/x abc
0|@C|-|cd @C && git commit-tree abc
0|@C|-|git commit-tree abc
0|@C|-|git --git-dir=@C/.git commit-tree abc
0|@C|-|GIT_DIR=@C/.git git commit-tree abc
0|@M|-|git -C @C commit-tree abc
0|@M|-|cd @C && git commit-tree abc
0|@M|-|cd @C; git commit-tree abc
0|@M|-|( cd @C && git commit-tree abc )
0|@M|-|pushd @C; git commit-tree abc
0|@M|-|pushd @C >/dev/null && git update-ref refs/heads/main abc
0|@M|-|cd @C; cd @M; popd; git commit-tree abc
0|@M|@H|cd ~/clone && git commit-tree abc
0|@M|-|git commit-tree abc -C @C
0|@M|-|git -C @C --work-tree=@M commit-tree abc
0|@M|-|git -C @M/ rev-parse HEAD
0|@M|-|git -C $CLAUDE_PROJECT_DIR status
0|@M|-|git -C @L log -1
0|@M|-|cd @L && git status
2|@M|-|"git" commit-tree abc
2|@M|-|'git' commit-tree abc
2|@M|-|"git" -C @M commit-tree abc
2|@M|-|"git"  update-ref refs/heads/x abc
2|@M|-|FOO=1 "git" commit-tree abc
2|@M|-|env "git" commit-tree abc
2|@M|-|"/usr/bin/git" commit-tree abc
2|@M|-|'/usr/bin/git' commit-tree abc
2|@M|-|git "commit-tree" abc
2|@M|-|git 'commit-tree' abc
2|@M|-|"git" "commit-tree" abc
2|@M|-|git -C "@M" "update-ref" refs/heads/main abc
2|@M|-|(  "git" commit-tree abc )
2|@M|-|echo x && "git" commit-tree abc
2|@M|-|git" "commit-tree abc
2|@M|-|git -C "@M"/tools commit-tree abc
0|@M|-|echo "git" commit-tree abc
0|@M|-|echo git commit-tree abc
0|@M|-|"git" status
2|@M|-|git merge --ff-only origin/main feature
0|@M|-|git merge origin/main
0|@M|-|git merge --ff-only origin/main
0|@M|-|git merge refs/remotes/origin/main
0|@M|-|git merge origin/main origin/dev
2|@M|-|git merge origin/main feature
2|@M|-|git merge feature origin/main
2|@M|-|git merge nosuchremote/x
2|@M|-|git merge origin/main nosuchremote/x
2|@M|-|git merge feature
2|@M|-|git merge --no-ff origin/main feature
2|@M|-|git merge --ff-only feature
2|@M|-|git merge -m msg feature
2|@M|-|git merge -m origin/main feature
0|@M|-|git merge -m msg origin/main
2|@M|-|git merge refs/heads/origin/main
2|@M|-|git merge HEAD~1
2|@M|-|git merge origin/main HEAD
2|@M|-|git merge FETCH_HEAD
2|@M|-|git merge origin
2|@M|-|git merge upstream/main
0|@M|-|git merge --squash feature
0|@M|-|git merge --abort
0|@C|-|git merge feature
0|@M|-|git -C @C merge feature
2|@C|-|git -C @M merge origin/main feature
2|@C|-|cd @M && git merge feature
0|@M|-|cd @C && git merge --ff-only x
2|@M|-|git merge --ff-only $CLAUDE_PROJECT_DIR/x
2|@W|-|git merge feature
0|@W|-|git merge --squash feature
2|@M|-|git -C @L merge feature
2|@M|-|git -C @C pull @M
2|@M|-|git pull .
0|@M|-|git pull origin main
CASES
echo "mismatches: $bad"
