#!/bin/bash
# $1 scratch dir (clone c1 on the branch; hooks cp'd from dc4c12093 = ced5e68f6 for "old"), $2 fakebin.
# Same tree, same staged content; old = dc4c12093 pre-commit and no commit-msg; new = the branch's hooks.
# Compares exit, stdout (timings stripped), stderr, and the cargo invocations (logged by a wrapper).
S=$1; export LC_ALL=C PATH=$2:$PATH; C=$S/c1
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
cd $C; git checkout -q -B off worktree-agent-a823a57e109be35e1
cp tools/git-hooks/pre-commit $S/pre-commit.new; cp tools/git-hooks/commit-msg $S/commit-msg.new
use_old() { cp $S/pre-commit.old tools/git-hooks/pre-commit; rm -f tools/git-hooks/commit-msg; }
use_new() { cp $S/pre-commit.new tools/git-hooks/pre-commit; cp $S/commit-msg.new tools/git-hooks/commit-msg; chmod +x tools/git-hooks/*; }
doit() { # label msg
  : > $S/clog; export CARGO_LOG=$S/clog
  git commit -q -m "$2" >$S/$1.out 2>$S/$1.err; echo $? > $S/$1.rc
  sed -E -i '' 's/[0-9]+\.[0-9]+s//g' $S/$1.out; cp $S/clog $S/$1.cargo
  [ "$(cat $S/$1.rc)" = 0 ] && git reset -q --soft HEAD~1
}
cmp_case() { # name msg   (index already staged; hook scripts are untracked-diff so keep them out of the commit)
  use_old; doit old "$2"; use_new; doit new "$2"
  if [ "$(cat $S/old.rc)" = "$(cat $S/new.rc)" ] && cmp -s $S/old.out $S/new.out && cmp -s $S/old.err $S/new.err && cmp -s $S/old.cargo $S/new.cargo; then
    echo "SAME $1: exit=$(cat $S/old.rc) stdout=$(wc -c <$S/old.out|tr -d ' ')B stderr=$(wc -c <$S/old.err|tr -d ' ')B cargo-calls=[$(tr '\n' ';' <$S/old.cargo)]"
  else echo "DIFF $1: old rc=$(cat $S/old.rc) new rc=$(cat $S/new.rc)"; diff $S/old.out $S/new.out | head -5; diff $S/old.err $S/new.err | head -5; diff $S/old.cargo $S/new.cargo | head; fi
}
stage() { eval "$1"; git add -- . ':!tools/git-hooks'; }
unstage() { git reset -q; eval "$1"; }
for sw in UNSET false no; do
  git config --unset newtos.ticketChecks 2>/dev/null; [ $sw = UNSET ] || git config newtos.ticketChecks $sw
  echo "== switch $sw"
  stage 'printf "\nx\n" >> TASK/A01.md'; cmp_case ticket-legacy-change "docs(task): touch (A01)"; unstage 'git checkout -q -- TASK/A01.md'
  stage 'echo "// x" >> tools/xtask/src/main.rs'; cmp_case unrelated-code "chore: x"; unstage 'git checkout -q -- tools/xtask/src/main.rs'
  stage 'echo x > zz_probe.txt'; cmp_case fix-names-OPEN "fix(x): y (A127)"; unstage 'rm zz_probe.txt'
  stage 'printf "# QQ1 — t\n\nStatus: IMPLEMENTED (2026-11-11) | Priority: P2\n" > TASK/QQ1.md'; cmp_case broken-new-ticket "docs(task): qq1 (QQ1)"; unstage 'rm TASK/QQ1.md'
  stage 'printf "\n" >> docs/MEMORY.md'; cmp_case memory-md "docs: memory"; unstage 'git checkout -q -- docs/MEMORY.md'
  stage 'printf "[x](nonexistent/path.md)\n" >> docs/STATUS.md'; cmp_case doc-check-failure "docs: broken cite"; unstage 'git checkout -q -- docs'
done
use_new; git config --unset newtos.ticketChecks 2>/dev/null; git status --short
