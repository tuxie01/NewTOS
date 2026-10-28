#!/bin/bash
# oracle-probe.sh <scratch> -- TOOL08 item 3: how the oracle shows Rwabs' sixth argument.
# Runs bios-r2.tsv once on the oracle's disk profile through oracle-traced.sh (a copy of
# tools/hatari-oracle.sh plus Hatari's --trace bios,xbios and a debugger breakpoint on the
# pinned ROM's _lrwabs), with the test image that tool08-runs.sh made (<scratch>/run/test.img)
# and an m68k DUMPBIOS.PRG built from this tree (make -C tosdump ... dumpbios-m68k).
# Writes runs/oracle-r2/: BIOS.BIN/.LOG, disk.sha256, hatari.log, trace.txt, the debugger
# files, rwabs-in-hatari-trace.txt (Hatari's own Rwabs lines), oracle-rwabs-check.txt, and
# bin-vs-reference.txt (records of this BIN against tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN).
set -u
S=$1
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; REPO="$(cd "$HERE/../../.." && pwd)"
R="$HERE/../runs"; mkdir -p "$R"; R="$(cd "$R" && pwd)"; O="$R/oracle-r2"
cd "$REPO"
BID="$(git rev-parse HEAD | cut -c1-4)"
make -C tosdump BUILD="$S/m68k" BUILD_ID="$BID" dumpbios-m68k > "$R/oracle-dumper-build.stdout" 2>&1 || exit 1
rm -rf "$S/oracle-r2"
HATARI_ORACLE_DISK="$S/run/test.img" "$HERE/oracle-traced.sh" disk "$S/m68k/DUMPBIOS.PRG" \
    tosdump/cases/bios-r2.tsv "$S/oracle-r2" > "$S/oracle-r2.stdout" 2>&1
rm -rf "$O"; mkdir -p "$O"
cp "$S/oracle-r2.stdout" "$O/oracle-traced.stdout"
cp "$S/oracle-r2/BIOS.BIN" "$S/oracle-r2/BIOS.LOG" "$S/oracle-r2/disk.sha256" "$S/oracle-r2/hatari.log" \
   "$S/oracle-r2/trace.txt" "$O/"
cp -R "$S/oracle-r2/hd-dbg" "$O/hd-dbg"
shasum -a 256 "$S/m68k/DUMPBIOS.PRG" "$S/run/test.img" tosdump/cases/bios-r2.tsv \
    tools/oracle/etos512de-1a7484ab.img > "$O/inputs.sha256"
/usr/bin/grep -n "Rwabs" "$O/trace.txt" > "$O/rwabs-in-hatari-trace.txt"
python3 -I "$HERE/oracle-rwabs-check.py" "$REPO" "$O/hatari.log" tosdump/cases/bios-r2.tsv \
    > "$O/oracle-rwabs-check.txt"; echo "exit $?" >> "$O/oracle-rwabs-check.txt"
{ cmp <(tail -c +33 "$O/BIOS.BIN") <(tail -c +33 tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN) \
    && echo "records (bytes 33..) equal to tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN"
  cmp -l <(head -c 32 "$O/BIOS.BIN") <(head -c 32 tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN) \
    | sed 's/^/header byte (1-based, octal this ref): /'; } > "$O/bin-vs-reference.txt" 2>&1
rm -rf "$S/oracle-r2"
tail -n 2 "$O/oracle-rwabs-check.txt"; cat "$O/bin-vs-reference.txt"
