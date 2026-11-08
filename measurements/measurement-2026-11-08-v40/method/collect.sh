#!/bin/bash
# V40: copy the evidence from the scratchpad S into this run directory (kernels, cards and probe BINs of NewTOS
# stay out: their sha256 are recorded; the oracle's probe BIN is kept gzipped, the host test's hashes come from it).
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
R=measurements/measurement-2026-11-08-v40
for d in e1 n-base n-fix1 n-fix n-fix2; do
  mkdir -p $R/probe/$d
  for f in run.txt VDPROBE.LOG hatari.log BOOT-SERIAL.txt card.img.manifest; do [ -f $S/$d/$f ] && cp $S/$d/$f $R/probe/$d/; done
  (cd $S/$d && shasum -a 256 VDPROBE.BIN VDPROBE.LOG) > $R/probe/$d/sha256.txt
done
gzip -9 -c $S/e1/VDPROBE.BIN > $R/probe/e1/VDPROBE.BIN.gz
cp $S/compare-*.txt $R/probe/
for t in $S/runs/tab-*; do
  n=$(basename $t); mkdir -p $R/runs/$n
  for f in $t/card.img.manifest $t/tosdiff-*.txt $t/tosdump-run.txt $t/VDI.BIN $t/VDI.LOG $t/AES.BIN $t/AES.LOG; do [ -f $f ] && cp $f $R/runs/$n/; done
  (cd $t && shasum -a 256 *.BIN *.LOG 2>/dev/null) > $R/runs/$n/sha256.txt || true
done
if [ -d $S/runs/gui ]; then
  for g in $S/runs/gui/*; do
    n=$(basename $g); mkdir -p $R/runs/gui/$n
    for f in run.txt card.img.manifest NEWDESK.INF shots.txt; do [ -f $g/$f ] && cp $g/$f $R/runs/gui/$n/; done
    [ -d $g/shots ] && (cd $g/shots && shasum -a 256 * ) > $R/runs/gui/$n/shots.sha256
    /usr/bin/grep -ciE "exception|panic" $g/BOOT-SERIAL.txt > $R/runs/gui/$n/exceptions.count 2>/dev/null || true
  done
fi
mkdir -p $R/builds/base $R/builds/fix $R/builds/fix1 $R/builds/fix2
for L in base fix fix1 fix2; do cp $S/builds/$L/warnings.txt $R/builds/$L/; done
for L in base fix2; do /usr/bin/grep -E 'generated [0-9]+ warning' $S/builds/$L/clippy-tos-api-tests.log > $R/builds/$L/clippy-tos-api-tests.txt; done
cp $S/gui-compare.txt $R/runs/gui/compare.txt
: # test logs: copied by hand, see RUNINFO TESTS
cp $S/red-noaes-window.txt $R/hosttest/red-noaes-window.txt
/usr/bin/grep -E '^test |test result|differs|Running' $S/red-base-full.txt > $R/hosttest/red-base.txt
/usr/bin/grep -E '^test |test result|differs|Running|panicked' $S/red-noaes.txt > $R/hosttest/red-noaes.txt
echo collected
