#!/usr/bin/env bash
S=/Volumes/Daten/Development/PiRusTOS/target/agents/tool46-retest
FX=$S/fx; X=$S/tgt/debug/xtask; OUT=$S/outside
cd $FX
ok(){ echo "ok $1"; }; bad(){ echo "FAIL $1"; }
has(){ [ -e $FX/target-agents/agent-$1/clone/f ]; }
mkdirs(){ rm -rf $FX/target-agents; for n in "$@"; do mkdir -p $FX/target-agents/agent-$n/clone; echo x > $FX/target-agents/agent-$n/clone/f; done; }
tk(){ printf '# %s — t\n\nStatus: %s\n\nBody.\n' "$1" "$2"; }
git reset -q; git clean -fdq TASK 2>/dev/null
tk Z1 'OPEN (2026-11-10) | Priority: P1' > TASK/Z1.md; git add TASK/Z1.md; git commit -qm "docs(task): open Z1 (Z1)"
tk Z1 'IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: self | Missing: tester' > TASK/Z1.md
printf '# Z1 — receipt\n\nRole: developer\nAgent: agent-dev-a\n\n## Done\n\nWork.\n' > TASK/Z1-STATUS-UNFINISHED.MD
printf '# Z1 — receipt\n\nRole: developer\nAgent: dev-c\n\n## Why\n' > TASK/Z1-STATUS-BLOCKED.MD
git add TASK; git commit -qm "feat(z): the change (Z1)"; H=$(git rev-parse --short=9 HEAD)
echo "== accept refusals delete nothing"
mkdirs dev-a dev-c tester-b other
before=$(find target-agents | sort | wc -l)
$X accept Z1 --commit deadbeef0 --result PASS --agent tester-b >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-a && has dev-c && ok "bad commit refused rc=$rc, nothing deleted" || bad "bad commit"
$X accept Z1 --commit $H --result PASS --agent agent-dev-a >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-a && has dev-c && ok "developer-as-acceptor refused rc=$rc, nothing deleted" || bad "dev acceptor"
$X accept Z1 --commit $H --result MAYBE --agent tester-b >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-a && ok "bad result refused rc=$rc, nothing deleted" || bad "bad result"
$X accept Z2 --commit $H --result PASS --agent tester-b >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-a && ok "unknown ticket refused rc=$rc" || bad "unknown ticket"
echo x > staged.txt; git add staged.txt
$X accept Z1 --commit $H --result PASS --agent tester-b >/dev/null 2>&1; rc=$?; [ $rc != 0 ] && has dev-a && ok "staged changes refused rc=$rc, nothing deleted" || bad "staged"
git reset -q; rm staged.txt
# hook rejection
mkdir -p .git/hooks; printf '#!/bin/sh\necho REJECTED-BY-HOOK >&2\nexit 1\n' > .git/hooks/pre-commit; chmod +x .git/hooks/pre-commit
$X accept Z1 --commit $H --result PASS --note n --agent tester-b > $S/acc-hookrej.log 2>&1; rc=$?
[ $rc != 0 ] && has dev-a && has dev-c && has tester-b && ok "hook rejection: rc=$rc, nothing deleted" || bad "hook rej"
git diff --cached --name-only | tr '\n' ' '; echo "(staged after rejection)"
rm .git/hooks/pre-commit; git reset -q; git checkout -q -- TASK/Z1.md 2>/dev/null; git clean -fdq TASK/Z1-STATUS-DONE.MD 2>/dev/null
git status --short | head -3
echo "== accept PASS"
mkdir -p $OUT; echo p > $OUT/precious
mkdirs dev-a tester-b other z2dev; mkdir -p target-agents/agent-dev-c; rm -rf target-agents/agent-dev-c; ln -s $OUT target-agents/agent-dev-c
$X accept Z1 --commit $H --result PASS --note "fixture" --agent tester-b | tee $S/acc-pass.log
! has dev-a && has tester-b && has other && has z2dev && ok "dev-a gone; acceptor, other, unrelated survive" || bad "accept clean set"
[ -f $OUT/precious ] && [ -L target-agents/agent-dev-c ] && ok "symlinked recorded agent dir refused, outside intact" || bad "symlink"
git log -n1 --format=%s; git status --short | head -3
