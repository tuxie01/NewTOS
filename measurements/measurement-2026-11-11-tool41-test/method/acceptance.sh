#!/bin/bash
# Acceptance cases of TOOL41, switch ON, in a throwaway clone ($1); $2 = scratch out file.
cd "$1" || exit 2
OUT=$2; export LC_ALL=C
git config newtos.ticketChecks true
git checkout -q -B acc
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
D=2026-11-11
run() { # name expect cmd...
  local name=$1 exp=$2; shift 2
  local t0=$(python3 -c 'import time;print(time.time())')
  "$@" >$OUT 2>&1; local rc=$?
  local t1=$(python3 -c 'import time;print(time.time())')
  printf 'CASE %s: expect %s -> exit=%s time=%.2fs\n' "$name" "$exp" "$rc" "$(echo "$t1 - $t0" | bc)"
  sed 's/^/    | /' $OUT | head -${LINES_MAX:-10}
}
tk() { printf '# %s — t\n\nStatus: %s\n\nBody.\n' "$1" "$2" > TASK/$1.md; }
x() { cargo -q xtask "$@"; }
mk() { git add -A; git commit -q -m "$1"; }
tk ZZ1 "OPEN ($D) | Priority: P2"
tk ZZ2 "OPEN ($D) | Priority: P2"
run setup-open-tickets PASS bash -c 'git add -A && git commit -q -m "docs(task): open ZZ1 ZZ2 (ZZ1)"'
echo a > code1.txt; git add -A
run fix-names-OPEN REJECT git commit -q -m "fix(x): thing (ZZ1)"
tk ZZ1 "IMPLEMENTED ($D) | Priority: P2 | Commit: self | Missing: tester acceptance"
printf '# ZZ1 — receipt\n\nRole: developer\nAgent: dev-a\n\n## Done\n\nw\n' > TASK/ZZ1-STATUS-UNFINISHED.MD
git add -A
run fix-names-IMPLEMENTED PASS git commit -q -m "fix(x): thing (ZZ1)"
SELF=$(git rev-parse HEAD)
echo b >> code1.txt; git add -A
run multi-id-one-OPEN REJECT git commit -q -m "fix(x): more (ZZ1, ZZ2)"
tk ZZ2 "IMPLEMENTED ($D) | Priority: P2 | Commit: self | Missing: tester acceptance"
printf '# ZZ2 — receipt\n\nRole: developer\nAgent: dev-b\n\n## Done\n\nw\n' > TASK/ZZ2-STATUS-UNFINISHED.MD
git add -A
run multi-id-both-IMPLEMENTED PASS git commit -q -m "fix(x): more (ZZ1, ZZ2)"
SELF2=$(git rev-parse HEAD)
printf '# ZZ1 — done\n\nRole: tester\nAgent: t1\n\nCommit: %s\nResult: PASS\n' "${SELF:0:9}" > TASK/ZZ1-STATUS-DONE.MD; git add -A
run done-receipt-line3-not-DONE REJECT git commit -q -m "docs(task): receipt (ZZ1)"
git reset -q; rm TASK/ZZ1-STATUS-DONE.MD
tk ZZ1 "DONE ($D) | Priority: P2 | Commit: ${SELF:0:9} | Receipt: ZZ1-STATUS-DONE.MD"; git add -A
run line3-DONE-no-receipt REJECT git commit -q -m "docs(task): done (ZZ1)"
git reset -q; tk ZZ1 "IMPLEMENTED ($D) | Priority: P2 | Commit: self | Missing: tester acceptance"
tk ZZ3 "IMPLEMENTED ($D) | Priority: P2 | Commit: self"; git add -A
run IMPLEMENTED-no-Missing REJECT git commit -q -m "docs(task): zz3 (ZZ3)"
git reset -q; rm TASK/ZZ3.md
tk ZZ3 "PARKED ($D) | Priority: P2 | Reason: nothing"; git add -A
run PARKED-no-Reopen REJECT git commit -q -m "docs(task): zz3 (ZZ3)"
git reset -q; rm TASK/ZZ3.md
echo "status before accept:"; git status --short
run accept-self REFUSE x accept ZZ1 --commit "$SELF" --result PASS --agent dev-a
echo c > stage.txt; git add stage.txt
run accept-staged-dirty REFUSE x accept ZZ1 --commit "$SELF" --result PASS --agent tester-1
git reset -q; rm stage.txt
run set-status-PARKED-no-reopen REFUSE x set-status ZZ2 PARKED --reason why
run set-status-DONE REFUSE x set-status ZZ2 DONE
run set-status-IMPLEMENTED REFUSE x set-status ZZ2 IMPLEMENTED
echo "status after refusals:"; git status --short
before=$(git rev-parse HEAD)
run accept-PASS-other OK-one-commit x accept ZZ1 --commit "$SELF" --result PASS --agent tester-1 --note "acc test"
echo "commits added: $(git rev-list --count $before..HEAD)"; git show --stat --format='%s%n%b' HEAD | head -12
echo "tree status lines: $(git status --short | wc -l)"
sed -n 3p TASK/ZZ1.md; cat TASK/ZZ1-STATUS-DONE.MD
before=$(git rev-parse HEAD); L3=$(sed -n 3p TASK/ZZ2.md)
run accept-FAIL OK x accept ZZ2 --commit "$SELF2" --result FAIL --agent tester-1 --note "broken"
echo "commits added: $(git rev-list --count $before..HEAD)"; git show --stat --format='%s' HEAD | head
[ "$(sed -n 3p TASK/ZZ2.md)" = "$L3" ] && echo "line3 unchanged: yes" || echo "line3 unchanged: NO"
cat TASK/ZZ2-STATUS-UNFINISHED.MD
echo "tree status lines: $(git status --short | wc -l)"
tk ZZ4 "OPEN ($D) | Priority: P3"; mk "docs(task): zz4 (ZZ4)"
run set-status-PARKED-ok OK x set-status ZZ4 PARKED --reason "waits" --reopen "dep lands"
sed -n 3p TASK/ZZ4.md; git add -A; run commit-parked OK git commit -q -m "docs(task): park ZZ4 (ZZ4)"
run set-status-OPEN-ok OK x set-status ZZ4 OPEN --reason "dep landed"
sed -n 3p TASK/ZZ4.md; tail -6 TASK/ZZ4.md
run set-status-OPEN-from-OPEN REFUSE x set-status ZZ4 OPEN --reason again
git add -A; git commit -q -m "docs(task): reopen ZZ4 (ZZ4)"
echo "--- Commit:self resolution via report"; x report 2>&1 | grep -E "ZZ" | head
echo "=== blocked-by-DONE report"
tk ZZ5 "PARKED ($D) | Priority: P3 | Reason: blocked by ZZ1 | Reopen: ZZ1 lands"; git add -A
run blocked-by-DONE-commit PASS-with-report git commit -q -m "docs(task): zz5 (ZZ5)"
x status-check 2>&1 | grep -E "blocked|ZZ5" | head -3
echo "=== rename of ticket file, Commit: self still resolves"
git mv TASK/ZZ2.md TASK/ZZ8.md; git mv TASK/ZZ2-STATUS-UNFINISHED.MD TASK/ZZ8-STATUS-UNFINISHED.MD
sed -i '' 's/ZZ2/ZZ8/g' TASK/ZZ8.md TASK/ZZ8-STATUS-UNFINISHED.MD
git add -A; run rename-commit PASS git commit -q -m "docs(task): rename ZZ2 to ZZ8 (ZZ8)"
x report 2>&1 | grep -E "ZZ8"; echo "expected self = $SELF2"
