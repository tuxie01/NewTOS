#!/bin/zsh
# usage: refuse.sh  (cwd = clone) ; each case resets to base
BASE=$(cat ../head1)
T=$'\t'
AP='# approved: owner 2026-11-11 test'
reset() { git reset -q --hard $BASE; git clean -fdq -e target; }
snap() { echo "$(git rev-parse HEAD) $(git status --porcelain | md5) $(md5 < tools/xtask/ticket-baseline.tsv) staged=$(git diff --cached --name-only | wc -l | tr -d ' ')"; }
runcase() { # name, setup-fn
  reset; eval "$2"; local b=$(snap); local out; out=$(cargo xtask migrate --batch ${3:-2} 2>&1 | grep -v "Finished\|Running"); local rc=$?
  local a=$(snap); [ "$b" = "$a" ] && s=UNCHANGED || s="CHANGED($b -> $a)"
  echo "### $1 :: $s"; echo "$out" | head -${4:-4} | cut -c1-260
}
mk() { # file contents(lines) ; commit
  printf '%s\n' "$@" > /dev/null
}
commit_batch() { git add TASK/migration/batch-$1.tsv && git commit -q -m "docs(task): batch $1 test" >/dev/null 2>&1; }
wb() { n=$1; shift; mkdir -p TASK/migration; printf '%s\n' "$@" > TASK/migration/batch-$n.tsv; }
runcase "no approval line" 'wb 2 "# comment" "A119${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "malformed approval" 'wb 2 "# approved: owner 11-11-2026" "A119${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "approval without owner" 'wb 2 "# approved: lead 2026-11-11" "A119${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "batch not committed (untracked)" 'wb 2 "$AP" "A119${T}OPEN${T}Priority: P3${T}ev"'
runcase "batch committed, then modified" 'wb 2 "$AP" "A119${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2; echo "A120${T}OPEN${T}Priority: P3${T}ev" >> TASK/migration/batch-2.tsv'
runcase "batch staged only" 'wb 2 "$AP" "A119${T}OPEN${T}Priority: P3${T}ev"; git add TASK/migration/batch-2.tsv'
runcase "id not in baseline (TOOL54)" 'wb 2 "$AP" "TOOL54${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "id not in baseline (A119 after batch1 -> use batch 1 state: A140 not legacy?)" 'wb 2 "$AP" "NOSUCH${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "DONE without receipt file" 'wb 2 "$AP" "A119${T}DONE${T}Priority: P3 | Commit: 033da16f | Receipt: A119-STATUS-DONE.MD${T}A119-STATUS-DONE.MD names 033da16f"; commit_batch 2'
runcase "DONE evidence lacks commit" 'wb 2 "$AP" "A130${T}DONE${T}Priority: P2 | Commit: d9e44143 | Receipt: A130-STATUS-DONE.MD${T}receipt A130-STATUS-DONE.MD only"; commit_batch 2'
runcase "DONE evidence lacks receipt name" 'wb 2 "$AP" "A130${T}DONE${T}Priority: P2 | Commit: d9e44143 | Receipt: A130-STATUS-DONE.MD${T}fix d9e44143 only"; commit_batch 2'
runcase "DONE without Receipt field" 'wb 2 "$AP" "A130${T}DONE${T}Priority: P2 | Commit: d9e44143${T}A130-STATUS-DONE.MD d9e44143"; commit_batch 2'
runcase "PARKED without Reopen" 'wb 2 "$AP" "A120${T}PARKED${T}Priority: P3 | Reason: later${T}ev"; commit_batch 2'
runcase "PARKED without Reason" 'wb 2 "$AP" "A120${T}PARKED${T}Priority: P3 | Reopen: x${T}ev"; commit_batch 2'
runcase "OBSOLETE without Reason" 'wb 2 "$AP" "A120${T}OBSOLETE${T}Priority: P3 | Target: A27${T}ev"; commit_batch 2'
runcase "OBSOLETE with Target to no ticket" 'wb 2 "$AP" "A120${T}OBSOLETE${T}Priority: P3 | Reason: dup | Target: ZZZ99${T}ev"; commit_batch 2'
runcase "ACCEPTED without Ref" 'wb 2 "$AP" "A120${T}ACCEPTED${T}Priority: P3${T}ev"; commit_batch 2'
runcase "IMPLEMENTED without Missing" 'wb 2 "$AP" "A120${T}IMPLEMENTED${T}Priority: P3 | Commit: 033da16f${T}ev"; commit_batch 2'
runcase "missing Priority" 'wb 2 "$AP" "A120${T}OPEN${T}Reason: x${T}ev"; commit_batch 2'
runcase "Commit: self" 'wb 2 "$AP" "A120${T}IMPLEMENTED${T}Priority: P3 | Commit: self | Missing: x${T}ev"; commit_batch 2'
runcase "Commit not on history" 'wb 2 "$AP" "A120${T}IMPLEMENTED${T}Priority: P3 | Commit: deadbeef1 | Missing: x${T}ev"; commit_batch 2'
runcase "id twice" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev" "A120${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2'
runcase "one good + one bad row (atomic)" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev" "A121${T}PARKED${T}Priority: P3${T}ev"; commit_batch 2'
runcase "closed.txt committed" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev"; echo "owner 2026-11-11 end" > TASK/migration/closed.txt; git add TASK/migration; git commit -q -m "docs(task): close" >/dev/null 2>&1'
runcase "closed.txt uncommitted" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2; echo "owner end" > TASK/migration/closed.txt'
runcase "other file staged" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2; echo x >> docs/STATUS.md; git add docs/STATUS.md'
runcase "written file dirty (A120.md modified)" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2; echo x >> TASK/A120.md'
runcase "baseline dirty" 'wb 2 "$AP" "A120${T}OPEN${T}Priority: P3${T}ev"; commit_batch 2; echo "# x" >> tools/xtask/ticket-baseline.tsv'
runcase "no batch file at all (batch 9)" ':' 9
runcase "bad --batch arg" ':' x
reset
