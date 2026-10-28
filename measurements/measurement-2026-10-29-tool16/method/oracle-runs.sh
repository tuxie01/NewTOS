#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> -- TOOL16: tosdump/cases/xbios-ptr-r1.tsv on the
# pinned oracle, profiles tt and falcontc, twice each (tools/hatari-oracle.sh), with the
# DUMPBIOS.PRG of `make -C tosdump dumpbios-m68k`; BIN and LOG of each run into
# <runs-dir>/oracle-<profile>-<n>/, then cmp of the two runs per profile and
# tosdiff --selfcheck of run 1.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
for P in tt falcontc; do
    for N in 1 2; do
        tools/hatari-oracle.sh "$P" tosdump/build/DUMPBIOS.PRG tosdump/cases/xbios-ptr-r1.tsv \
            "$S/emu-$P-$N" > "$S/emu-$P-$N.stdout" 2>&1
        echo "oracle-$P-$N: hatari-oracle.sh exit $?"
        mkdir -p "$R/oracle-$P-$N"
        cp "$S/emu-$P-$N/BIOS.BIN" "$S/emu-$P-$N/BIOS.LOG" "$R/oracle-$P-$N/"
    done
    cmp "$R/oracle-$P-1/BIOS.BIN" "$R/oracle-$P-2/BIOS.BIN" && echo "$P: BIN run 1 = run 2"
    cmp "$R/oracle-$P-1/BIOS.LOG" "$R/oracle-$P-2/BIOS.LOG" && echo "$P: LOG run 1 = run 2"
    tosdump/host/build/tosdiff --selfcheck tosdump/cases/xbios-ptr-r1.tsv \
        "$R/oracle-$P-1/BIOS.BIN" > "$R/selfcheck-$P.txt" 2>&1
    echo "selfcheck-$P: exit $?"
done
tosdump/host/build/tosdiff tosdump/cases/xbios-ptr-r1.tsv "$R/oracle-tt-1/BIOS.BIN" \
    "$R/oracle-falcontc-1/BIOS.BIN" > "$R/tosdiff-tt-vs-falcontc.txt" 2>&1
echo "tt vs falcontc: exit $?"
