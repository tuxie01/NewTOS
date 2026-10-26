#!/bin/sh
# compare-newtos-v3.sh <runs-root> -- per new dump: validate; payload (bytes after the 32-byte header)
# vs every v2 NEWTOS-* file of the same table (sha256, equal?, count of differing bytes via cmp -l);
# tosdiff against the EmuTOS counterpart.
REPO=/Volumes/Daten/Development/PiRusTOS; RUNS=$1
TD=$REPO/tosdump/host/build/tosdiff; REF=$REPO/tosdump/reference
pl() { tail -c +33 "$1"; }
while read -r SUF AREA TABLE OLDS EMU; do
  case $SUF in ''|\#*) continue;; esac
  N=$RUNS/$SUF/$AREA.BIN; C=$REPO/tosdump/cases/$TABLE
  echo "######## $SUF  (table $TABLE)"
  echo "-- header new: $(xxd -l 32 -p $N | tr -d '\n')"
  $TD --validate $N $C 2>&1
  for o in $(echo $OLDS | tr "," " "); do
    O=$REF/NEWTOS-$o.BIN
    echo "-- payload vs NEWTOS-$o (v2): old sha $(pl $O | shasum -a 256 | cut -c1-16) new sha $(pl $N | shasum -a 256 | cut -c1-16)"
    pl $O > $RUNS/.o; pl $N > $RUNS/.n
    if cmp -s $RUNS/.o $RUNS/.n; then echo "   IDENTICAL"; else echo "   CHANGED, differing bytes: $(cmp -l $RUNS/.o $RUNS/.n | wc -l | tr -d ' ') of $(wc -c < $RUNS/.n | tr -d ' ')"; fi
    echo "   LOG diff:"; diff $REF/NEWTOS-$o.LOG $RUNS/$SUF/$AREA.LOG | sed 's/^/     /'
  done
  rm -f $RUNS/.o $RUNS/.n
  echo "-- tosdiff vs EMUTOS-1a7484ab-$EMU"
  $TD $C $REF/EMUTOS-1a7484ab-$EMU.BIN $N > $RUNS/$SUF/tosdiff-vs-emutos.txt 2>&1; echo "exit $?"
  head -3 $RUNS/$SUF/tosdiff-vs-emutos.txt; tail -8 $RUNS/$SUF/tosdiff-vs-emutos.txt
done <<'T'
VDI-all VDI vdi-all.tsv VDI-all,VDI-all-0906 VDI-all
VDI-r2 VDI vdi-r2.tsv VDI-r2 VDI-r2
AES AES aes-r1.tsv AES,AES-r2 AES
AES-r2wind AES aes-r2-wind.tsv AES-r2wind AES-r2wind
RASTER RASTER vdi-raster.tsv RASTER,RASTER-fixed RASTER
T
