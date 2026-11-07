#!/bin/sh
# V53: tosdiff base vs fix per table (tosdump/host/build/tosdiff <table> A.BIN B.BIN) plus a byte compare.
S=${S:?}
cd /Volumes/Daten/Development/PiRusTOS
TD=tosdump/host/build/tosdiff
for t in v18t3:vdi-v18-t3:VDI v18t2:vdi-v18-t2:VDI all:vdi-all:VDI r2:aes-r2-wind:AES r3:aes-r3-wind-state:AES r4:aes-r4-subfunctions:AES; do
  n=${t%%:*}; r=${t#*:}; tab=${r%%:*}; a=${r#*:}
  D=$(mktemp -d "$S/td.XXXXXX")
  cp $S/runs/tab-base-$n/$a.BIN $D/BASE.BIN; cp $S/runs/tab-base-$n/$a.LOG $D/BASE.LOG
  cp $S/runs/tab-fix-$n/$a.BIN $D/FIX.BIN; cp $S/runs/tab-fix-$n/$a.LOG $D/FIX.LOG
  (cd $D && $OLDPWD/$TD $OLDPWD/tosdump/cases/$tab.tsv BASE.BIN FIX.BIN) > $S/runs/tosdiff-base-vs-fix-$n.txt 2>&1
  echo "$n: $(/usr/bin/grep -E 'cases differ' $S/runs/tosdiff-base-vs-fix-$n.txt | head -1); bytes: $(cmp -l $D/BASE.BIN $D/FIX.BIN | awk '{print $1}' | tr '\n' ' ' | cut -c1-80)"
  rm -rf $D
done
