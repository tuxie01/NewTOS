#!/bin/sh
# oracle-runs.sh <scratch> <runs-dir> -- TOOL18: tosdump/cases/jar-r1.tsv on the
# pinned oracle, profiles tt, falcontc and disk, twice each (tools/hatari-oracle.sh),
# with the DUMPBIOS.PRG of `make -C tosdump BUILD=<scratch>/dumpers/<id> dumpbios-m68k`
# (<id> = the first 4 hex digits of HEAD). Profile disk boots a fresh clone of
# <scratch>/test.img = tools/mktestdisk.sh with no files (the table touches no disk;
# the profile is measured because its machine is its own Hatari setup). BIN and LOG of
# each run into <runs-dir>/oracle-<profile>-<n>/ with decoded.txt (method/decode-jar.py),
# then cmp of the two runs per profile and tosdiff --selfcheck of run 1, and the
# profiles against each other.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
T=tosdump/cases/jar-r1.tsv
ID=$(git rev-parse HEAD | cut -c1-4)
mkdir -p "$S/dumpers/$ID" "$R"
make -s -C tosdump BUILD="$S/dumpers/$ID" dumpbios-m68k > "$S/make-$ID.txt" 2>&1 || { cat "$S/make-$ID.txt"; exit 1; }
make -s -C tosdump/host build/tosdiff > "$S/make-tosdiff.txt" 2>&1 || { cat "$S/make-tosdiff.txt"; exit 1; }
echo "DUMPBIOS.PRG $(shasum -a 256 "$S/dumpers/$ID/DUMPBIOS.PRG" | cut -d' ' -f1)"
[ -f "$S/test.img" ] || tools/mktestdisk.sh "$S/test.img" > "$S/mktestdisk.stdout" 2>&1
cp "$S/test.img.manifest" "$R/test.img.manifest"
echo "test.img $(shasum -a 256 "$S/test.img" | cut -d' ' -f1)"
for P in tt falcontc disk; do
    for N in 1 2; do
        HATARI_ORACLE_DISK="$S/test.img" tools/hatari-oracle.sh "$P" \
            "$S/dumpers/$ID/DUMPBIOS.PRG" "$T" "$S/emu-$P-$N" > "$S/emu-$P-$N.stdout" 2>&1
        echo "oracle-$P-$N: hatari-oracle.sh exit $?"
        mkdir -p "$R/oracle-$P-$N"
        cp "$S/emu-$P-$N/BIOS.BIN" "$S/emu-$P-$N/BIOS.LOG" "$R/oracle-$P-$N/"
        [ -f "$S/emu-$P-$N/disk.sha256" ] && cp "$S/emu-$P-$N/disk.sha256" "$R/oracle-$P-$N/"
        rm -f "$S/emu-$P-$N/disk.img"
        python3 -I "$HERE/decode-jar.py" "$R/oracle-$P-$N/BIOS.BIN" "$T" > "$R/oracle-$P-$N/decoded.txt"
    done
    cmp "$R/oracle-$P-1/BIOS.BIN" "$R/oracle-$P-2/BIOS.BIN" && echo "$P: BIN run 1 = run 2"
    cmp "$R/oracle-$P-1/BIOS.LOG" "$R/oracle-$P-2/BIOS.LOG" && echo "$P: LOG run 1 = run 2"
    tosdump/host/build/tosdiff --selfcheck "$T" "$R/oracle-$P-1/BIOS.BIN" > "$R/selfcheck-$P.txt" 2>&1
    echo "selfcheck-$P: exit $?"
done
tosdump/host/build/tosdiff "$T" "$R/oracle-tt-1/BIOS.BIN" "$R/oracle-falcontc-1/BIOS.BIN" \
    > "$R/tosdiff-tt-vs-falcontc.txt" 2>&1
echo "tt vs falcontc: exit $?"
tosdump/host/build/tosdiff "$T" "$R/oracle-falcontc-1/BIOS.BIN" "$R/oracle-disk-1/BIOS.BIN" \
    > "$R/tosdiff-falcontc-vs-disk.txt" 2>&1
echo "falcontc vs disk: exit $?"
