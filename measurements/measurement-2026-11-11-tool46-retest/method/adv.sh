#!/usr/bin/env bash
# adversarial deletion safety, fixtures only
S=/Volumes/Daten/Development/PiRusTOS/target/agents/tool46-retest
FX=$S/fx; OUT=$S/outside; SCR=$FX/tools/hooks/agent-scratch.sh
export CLAUDE_PROJECT_DIR=$FX
X=$S/tgt/debug/xtask
pass=0; fail=0
snap(){ ( cd $S && find fx/target-agents outside -print0 2>/dev/null | sort -z | xargs -0 -n1 sh -c 'if [ -L "$0" ]; then echo "L $0 -> $(readlink "$0")"; elif [ -f "$0" ]; then echo "F $0 $(shasum "$0"|cut -c1-12)"; else echo "D $0"; fi' ); }
mk(){ # fresh fixture tree
  rm -rf $FX/target-agents $OUT; mkdir -p $FX/target-agents/agent-real/sub $FX/target-agents/agent-other $OUT/dir
  echo 1 > $FX/target-agents/agent-real/f; echo 2 > $FX/target-agents/agent-other/f
  echo p > $OUT/precious; echo q > $OUT/dir/precious2
  ln -s $OUT $FX/target-agents/agent-real/sub/lnk
  echo sib > $FX/target-agents/sibfile
}
check(){ # name want(NOCHANGE|DIFF) snapbefore
  after=$(snap)
  if [ "$2" = NOCHANGE ]; then [ "$3" = "$after" ] && r=ok || r=FAIL; else [ "$3" != "$after" ] && r=ok || r=FAIL; fi
  [ $r = ok ] && pass=$((pass+1)) || { fail=$((fail+1)); diff <(echo "$3") <(echo "$after"); }
  echo "$r $1"
}
outside_ok(){ [ -f $OUT/precious ] && [ -f $OUT/dir/precious2 ] && r=ok || r=FAIL; [ $r = ok ] && pass=$((pass+1)) || fail=$((fail+1)); echo "$r outside intact: $1"; }
bad_ids=('..' 'a/b' '' $'real\nzz' '-rf' '../x' '.' '*' '--' 'real/../other' ' ' '/')
echo "== A: clean (script)"
for id in "${bad_ids[@]}"; do mk; b=$(snap); bash $SCR clean "$id" >/dev/null 2>&1; check "clean [$(printf %q "$id")]" NOCHANGE "$b"; done
echo "== B: scratch-clean <id> (xtask)"
for id in "${bad_ids[@]}"; do mk; b=$(snap); $X scratch-clean "$id" >/dev/null 2>&1; check "scratch-clean [$(printf %q "$id")]" NOCHANGE "$b"; done
echo "== C: positive: real removed, others survive, outside survives"
mk; b=$(snap); $X scratch-clean agent-real >/dev/null 2>&1; check "scratch-clean agent-real removes" DIFF "$b"
[ ! -e $FX/target-agents/agent-real ] && [ -f $FX/target-agents/agent-other/f ] && [ -f $FX/target-agents/sibfile ] && echo "ok only real gone" || { echo "FAIL only real"; fail=$((fail+1)); }
outside_ok "inner symlink"
echo "== D: symlinked target-agents"
mk; mv $FX/target-agents $S/real-ta; ln -s $S/real-ta $FX/target-agents; b=$( cd $S; find real-ta -print|sort ); 
bash $SCR clean real >/dev/null 2>&1; $X scratch-clean real >/dev/null 2>&1
a=$( cd $S; find real-ta -print|sort ); [ "$a" = "$b" ] && echo "ok symlinked target-agents refused (clean+xtask)" || { echo "FAIL symlinked root"; fail=$((fail+1)); }
rm $FX/target-agents; mv $S/real-ta $FX/target-agents
echo "== E: agent dir is a symlink"
mk; rm -rf $FX/target-agents/agent-real; mkdir -p $S/victim; echo v > $S/victim/f; ln -s $S/victim $FX/target-agents/agent-real
bash $SCR clean real >/dev/null 2>&1; $X scratch-clean real >/dev/null 2>&1
[ -f $S/victim/f ] && [ -L $FX/target-agents/agent-real ] && echo "ok agent dir symlink untouched" || { echo "FAIL agent dir symlink"; fail=$((fail+1)); }
echo "== F: ticket receipts with hostile Agent lines (xtask --ticket)"
for id in '..' '../x' 'a/b' '-rf' '/'; do
  mk; printf '# Z9 — t\n\nStatus: OPEN (2026-11-10) | Priority: P1\n\nBody.\n' > $FX/TASK/Z9.md
  printf '# Z9 — r\n\nRole: developer\nAgent: %s\n\n## Done\n' "$id" > $FX/TASK/Z9-STATUS-UNFINISHED.MD
  b=$(snap); $X scratch-clean --ticket Z9 >/dev/null 2>&1; check "--ticket Z9 Agent: $id" NOCHANGE "$b"
done
rm -f $FX/TASK/Z9*
