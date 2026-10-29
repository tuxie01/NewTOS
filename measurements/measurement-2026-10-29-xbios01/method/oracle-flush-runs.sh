#!/bin/sh
# oracle-flush-runs.sh <scratch> <runs-dir> -- xbios-flush-r1.tsv on the pinned oracle,
# profiles tt and falcontc, twice each (method/oracle-flush.sh, keys via Hatari's
# --cmd-fifo), DUMPBIOS.PRG of `make -C tosdump dumpbios-m68k`; BIN, LOG and the
# key-send note into <runs-dir>/oracle-flush-<profile>-<n>/, cmp of the two runs,
# tosdiff --selfcheck of run 1, tt vs falcontc.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
T=tosdump/cases/xbios-flush-r1.tsv
for P in tt falcontc; do
    for N in 1 2; do
        "$HERE/oracle-flush.sh" "$P" tosdump/build/DUMPBIOS.PRG "$T" "$S/emu-flush-$P-$N" \
            > "$S/emu-flush-$P-$N.stdout" 2>&1
        echo "oracle-flush-$P-$N: oracle-flush.sh exit $?"
        mkdir -p "$R/oracle-flush-$P-$N"
        cp "$S/emu-flush-$P-$N/BIOS.BIN" "$S/emu-flush-$P-$N/BIOS.LOG" \
           "$S/emu-flush-$P-$N/keys-sent.txt" "$R/oracle-flush-$P-$N/"
    done
    cmp "$R/oracle-flush-$P-1/BIOS.BIN" "$R/oracle-flush-$P-2/BIOS.BIN" && echo "$P: BIN run 1 = run 2"
    cmp "$R/oracle-flush-$P-1/BIOS.LOG" "$R/oracle-flush-$P-2/BIOS.LOG" && echo "$P: LOG run 1 = run 2"
    tosdump/host/build/tosdiff --selfcheck "$T" "$R/oracle-flush-$P-1/BIOS.BIN" \
        > "$R/selfcheck-flush-$P.txt" 2>&1
    echo "selfcheck-flush-$P: exit $?"
done
tosdump/host/build/tosdiff "$T" "$R/oracle-flush-tt-1/BIOS.BIN" "$R/oracle-flush-falcontc-1/BIOS.BIN" \
    > "$R/tosdiff-flush-tt-vs-falcontc.txt" 2>&1
echo "flush tt vs falcontc: exit $?"
