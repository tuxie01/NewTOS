#!/bin/bash
# usage: warnings.sh <scratch-dir> <repo>
# scratch/copy = `git archive HEAD` tree (+ references symlink); per state the 8 changed files are
# overlaid (base = HEAD version, change = working-tree version). Fresh target dir per build.
S=$1; R=$2; C=$S/copy; FILES="tools/oracle/oracle.env tools/deps/gen-deps.py tools/sd-manifest.sh tools/xtask/src/main.rs tools/xtask/src/gen_deps_tests.rs tools/deps/README.md docs/sdk/06-image-and-testing.md TASK/TOOL39.md"
unset CARGO_TARGET_DIR
rm -rf $S/w; mkdir -p $S/w
for state in base change; do
  for f in $FILES; do
    if [ $state = base ]; then (cd $R; git show HEAD:$f > $C/$f 2>/dev/null) || rm -f $C/$f; else cp $R/$f $C/$f; fi
  done
  for b in rpi4 qemu-virt; do
    rm -rf $C/target
    (cd $C; cargo xtask build --board $b) > $S/w/$state-$b-build.txt 2>&1; echo "$state $b build rc=$?" >> $S/w/rc.txt
    (cd $C; cargo clippy -p kernel --no-default-features --features board-$b) > $S/w/$state-$b-clippy.txt 2>&1; echo "$state $b clippy rc=$?" >> $S/w/rc.txt
  done
  rm -rf $C/target
  (cd $C; cargo clippy -p xtask --tests) > $S/w/$state-xtask-clippy.txt 2>&1; echo "$state xtask clippy rc=$?" >> $S/w/rc.txt
done
for f in $S/w/*.txt; do echo "$(basename $f): $(grep -E 'generated [0-9]+ warning' $f | sed 's/^warning: //; s/ (run .*//' | tr '\n' ';')"; done > $S/w/summary.txt
echo done >> $S/w/rc.txt
