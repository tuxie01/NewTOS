#!/usr/bin/env bash
S=/Volumes/Daten/Development/PiRusTOS/target/agents/tool46-retest
FX=$S/fx; X=$S/tgt/debug/xtask; OUT=$S/outside
cd $FX
ok(){ echo "ok $1"; }; bad(){ echo "FAIL $1"; }
has(){ [ -e $FX/target-agents/agent-$1/clone/f ]; }
mkdirs(){ rm -rf $FX/target-agents; for n in "$@"; do mkdir -p $FX/target-agents/agent-$n/clone; echo x > $FX/target-agents/agent-$n/clone/f; done; }
tk(){ printf '# %s — t\n\nStatus: %s\n\nBody.\n' "$1" "$2"; }
git reset -q --hard $(cat $S/base); git clean -fdq TASK; rm -f code*.txt
MAIN=$(git symbolic-ref --short HEAD)
git branch -D feat >/dev/null 2>&1
tk L1 'OPEN (2026-11-10) | Priority: P1' > TASK/L1.md; git add TASK/L1.md; git commit -qm "docs(task): open L1 (L1)"
git switch -q -c feat
tk L1 'IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester' > TASK/L1.md
printf '# L1 — r\n\nRole: developer\nAgent: agent-dev-l\n\n## Done\n' > TASK/L1-STATUS-UNFINISHED.MD
echo c1 > code1.txt; git add -A; git commit -qm "feat(l): part one (L1)"
echo c2 > code2.txt; git add -A; git commit -qm "feat(l): part two (L1)"
git switch -q $MAIN
printf 'feat(l): the landed change (L1)\n\nBody.\n' > $S/msg.txt
echo "== refusals"
mkdirs dev-l tester-l other
echo y > y.txt; git add y.txt; H0=$(git rev-parse HEAD)
$X land feat -F $S/msg.txt >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && [ $(git rev-parse HEAD) = $H0 ] && has dev-l && ok "staged changes refused rc=$rc" || bad "staged"
git reset -q; rm y.txt
$X land nosuch -F $S/msg.txt >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-l && ok "unknown branch refused rc=$rc" || bad unknown
git switch -q feat; $X land feat -F $S/msg.txt >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-l && ok "checked-out branch refused rc=$rc" || bad checkedout; git switch -q $MAIN
$X land feat >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-l && ok "no -F refused rc=$rc" || bad nof
$X land feat -F $S/nomsg >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-l && ok "missing msg file refused rc=$rc" || bad nomsg
echo "== hook rejection leaves squash staged, deletes nothing"
printf '#!/bin/sh\necho REJECTED-BY-HOOK >&2\nexit 1\n' > .git/hooks/pre-commit; chmod +x .git/hooks/pre-commit
$X land feat -F $S/msg.txt > $S/land-rej.log 2>&1; rc=$?; tail -3 $S/land-rej.log
[ $rc != 0 ] && [ $(git rev-parse HEAD) = $H0 ] && has dev-l && has tester-l && has other && ok "rejected: rc=$rc HEAD unchanged, dirs kept" || bad rej
echo "staged: $(git diff --cached --name-only | tr '\n' ' ')"
git reset -q --hard $H0; git clean -fdq code1.txt code2.txt TASK 2>/dev/null
echo "== success, hook runs"
printf '#!/bin/sh\necho RAN > %s/hook-ran\nexit 0\n' $S > .git/hooks/pre-commit; rm -f $S/hook-ran
mkdirs dev-l tester-l other; mkdir -p target-agents/agent-dev-l/sub; ln -s $OUT target-agents/agent-dev-l/sub/lnk; echo p > $OUT/precious
$X land feat -F $S/msg.txt | tee $S/land-ok.log
[ -f $S/hook-ran ] && ok "pre-commit hook ran" || bad "hook did not run"
[ "$(git log -n1 --format=%P | wc -w | tr -d ' ')" = 1 ] && ok "one parent (squash)" || bad parents
git log -n1 --format=%s
[ -f code1.txt ] && [ -f code2.txt ] && ok "content landed" || bad content
! has dev-l && has other && ok "dev-l gone, other stays" || bad cleanset
has tester-l && ok "tester-l not recorded, stays" || echo "(tester-l gone)"
[ -f $OUT/precious ] && ok "outside intact" || bad outside
rm .git/hooks/pre-commit
