#!/bin/bash
# V56 test: base = git archive ad201a57, fix = git archive 559614ed (both without measurements/; fix plus the V34 test
# run's method/ for gui.sh), references/ symlinked (replacing the archive's directory). The host test file of the fix is
# copied into the base tree for the red run. $S/pr = scratch "probe repo": the fix commit's tosdump/ (committed vdprobe.c,
# not the main tree's V40 version) plus symlinks sdk, tools, references to the main checkout (libc.a, oracle ROM, mkcard).
S=${S:?}; cd /Volumes/Daten/Development/PiRusTOS
mkdir -p $S/t/base $S/t/fix
git archive ad201a574ddfb90aadb808da7fb67b085ddd5030 | tar -x -C $S/t/base --exclude measurements
git archive 559614ed | tar -x -C $S/t/fix --exclude measurements
git archive 559614ed measurements/measurement-2026-11-07-v34-test/method | tar -x -C $S/t/fix
for t in base fix; do rm -rf $S/t/$t/references; ln -s $PWD/references $S/t/$t/references; done
cp $S/t/fix/tos-api/tests/aes_v56_vector_tokens.rs $S/t/base/tos-api/tests/
mkdir -p $S/pr; cp -R $S/t/fix/tosdump $S/pr/tosdump; for d in sdk tools references; do ln -sfn $PWD/$d $S/pr/$d; done
