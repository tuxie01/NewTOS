#!/usr/bin/env bash
# Check 4: build-emutos.sh refusal, fixture copy only, shim git/make logging.
W=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-aeebbf8f8af122b9d
FX=/Volumes/Daten/Development/PiRusTOS/target-agents/agent-a8cc259b29d497846/bfx
rm -rf $FX; mkdir -p $FX; FX=$(cd -P $FX && pwd -P)
unset GIT_DIR GIT_WORK_TREE
gq() { git -c user.name=t -c user.email=t@t.invalid -c init.defaultBranch=master -c core.hooksPath=/dev/null "$@" >/dev/null 2>&1; }
P=$FX/proj
mkdir -p $P/tools/oracle $P/tools/hooks/lib $P/references/emutos $P/references/emutos-oracle/1a7484ab $P/references/freemint $FX/ta $FX/shim $FX/cross
gq init -q $P; printf 'references/\n' > $P/.gitignore; echo x > $P/a; gq -C $P add -A; gq -C $P commit -q -m i
cp $W/tools/oracle/build-emutos.sh $P/tools/oracle/; cp $W/tools/hooks/lib/paths.sh $P/tools/hooks/lib/
printf 'ORACLE_CROSS_MINT=%s/cross/m68k-\nORACLE_EMUTOS_COMMIT=none\nORACLE_ROM=x\nORACLE_ROM_SHA256=x\n' $FX > $P/tools/oracle/oracle.env
printf '#!/bin/sh\nexit 0\n' > $FX/cross/m68k-gcc; chmod +x $FX/cross/m68k-gcc
for c in git make; do
 if [ $c = git ]; then
  printf '#!/bin/sh\necho "git $*" >> %s/shim.log\ncase "$*" in *rev-parse*--git-common-dir*) exec /usr/bin/git "$@";; esac\nexit 99\n' $FX > $FX/shim/git
 else printf '#!/bin/sh\necho "make $*" >> %s/shim.log\nexit 99\n' $FX > $FX/shim/make; fi
 chmod +x $FX/shim/$c
done
# worktree-like: second repo dir whose references/* are symlinks into $P
WT=$FX/wt; gq -C $P worktree add -q -b wt $WT; mkdir -p $WT/tools/oracle $WT/tools/hooks/lib $WT/references; cp $P/tools/oracle/* $WT/tools/oracle/; cp $P/tools/hooks/lib/paths.sh $WT/tools/hooks/lib/
ln -s $P/references/emutos $WT/references/emutos; ln -s $P/references/emutos-oracle $WT/references/emutos-oracle
ln -s $P/references/emutos $FX/ta/lnk; ln -s ../proj/references $FX/ta/rel; ln -s $P/references $FX/ta/refs
ln -s $FX/ta/lnk $FX/ta/chain
n=0; bad=0
# r <script root> <want rc> <tree> [cwd]
r() {
  n=$((n+1)); : > $FX/shim.log
  out=$(cd "${4:-$FX}" && PATH="$FX/shim:$PATH" sh "$1/tools/oracle/build-emutos.sh" 1a7484ab 2026-04-14 "$3" 2>&1); rc=$?
  gl=$(grep -v 'rev-parse' $FX/shim.log | head -3 | tr '\n' '|')
  if [ $rc = $2 ]; then
    if [ $2 = 2 ] && [ -n "$gl" ]; then echo "MISS rc=2 but git/make ran ($gl): $3"; bad=$((bad+1)); else echo "ok   rc=$rc: $3  [root=${1#$FX}] ${gl:+log:$gl}"; fi
  else echo "MISS rc=$rc want=$2 [root=${1#$FX}] $3 :: $out"; bad=$((bad+1)); fi
}
for root in $P $WT; do
 r $root 2 $P/references/emutos
 r $root 2 $P/references/emutos-oracle/1a7484ab
 r $root 2 $P/references/emutos-oracle/new
 r $root 2 $P/references/freemint/x
 r $root 2 $P/references/newdir
 r $root 2 $P/references
 r $root 2 $P/REFERENCES/new
 r $root 2 $P/References/EmuTOS
 r $root 2 $P/references/EMUTOS-ORACLE/1A7484AB
 r $root 2 $FX/PROJ/references/x
 r $root 2 $FX/proj/references/../references/y
 r $root 2 $FX/proj/references/emutos/bios/../x
 r $root 2 $FX/ta/lnk
 r $root 2 $FX/ta/lnk/sub
 r $root 2 $FX/ta/chain/sub
 r $root 2 $FX/ta/rel/emutos
 r $root 2 $FX/ta/rel/new
 r $root 2 $FX/ta/refs/new
 r $root 2 $FX/TA/LNK/sub
 r $root 2 $FX/ta/lnk/../emutos-oracle/z
 r $root 2 references/emutos-oracle/1a7484ab $P
 r $root 2 references/emutos-oracle/1a7484ab $WT
 r $root 2 emutos $P/references
 r $root 2 ../references/q $P/tools
 r $root 2 ./emutos-oracle/x $WT/references
 r $root 2 emutos/sub $WT/references
 r $root 2 $WT/references/emutos
 r $root 2 $WT/references/emutos-oracle/x
 [ $root = $WT ] && r $root 2 $WT/references/anything
 # allowed
 r $root 99 $FX/ta/emutos-1a7484ab
 r $root 99 $FX/target-agents/agent-a1/emutos
 r $root 99 $FX/elsewhere/emutos
 r $root 99 $FX/ta/references-copy
 r $root 99 $FX/proj/references2/x
 r $root 99 $FX/proj/target-agents/agent-a1/emutos
 r $root 99 $FX/proj/refs/emutos
 r $root 99 tree-x $FX/ta
 r $root 99 ../x $FX/ta
done
echo "TOTAL $n MISS $bad"
