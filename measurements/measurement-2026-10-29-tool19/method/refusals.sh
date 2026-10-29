#!/bin/sh
# refusals.sh <scratch> <run-dir> -- TASK/TOOL19: the profile values (D:\DISKUNIT.TXT,
# D:\PROGEXT.TXT) refused when missing or not the profile's, and recorded when they are.
#
#   A  tools/hatari-oracle.sh, before Hatari starts: a u:0 table in profile tt, an x:
#      table (method/x-plain.tsv) in profile falcontc -- no row in tosdump/profiles.tsv
#   B  cargo xtask tosdump, before anything is built: the u:0 table and the x: table
#      without --disk
#   C  tosdiff --check-profile and the comparison, on copies of the two pinned oracle
#      dumps that leave a value to the profile, their LOGs edited: missing, no line,
#      another value, the other machine's value
#   D  one run each side of bios-r2-phys.tsv (u:0) with the profile: oracle profile disk
#      (profile.txt, the LOG check) and cargo xtask tosdump --disk (tosdump-run.txt,
#      read-back from the card, the LOG check, tosdiff against the pinned reference)
#
# Every refusal's exit status and message go to <run-dir>/results/<name>.txt.
set -u
S=$1; R=$2
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S" "$R/results" "$R/runs"
ID=$(git rev-parse HEAD | cut -c1-4)
REF=tosdump/reference/EMUTOS-1a7484ab
PHYS=tosdump/cases/bios-r2-phys.tsv
FS22=tosdump/cases/gemdos-mc-fs22.tsv

rec() {   # rec <name> <cmd...>: run, keep exit status and output
	n=$1; shift
	"$@" > "$R/results/$n.txt" 2>&1
	echo "exit $?" >> "$R/results/$n.txt"
	echo "$n: $(tail -n 1 "$R/results/$n.txt")"
}

make -C tosdump BUILD="$S/dumpers/$ID" BUILD_ID="$ID" dumpbios-m68k > "$R/dumper-build.stdout" 2>&1
make -C tosdump/host build/tosdiff > "$R/tosdiff-build.stdout" 2>&1
TOSDIFF=tosdump/host/build/tosdiff
PRG=$S/dumpers/$ID/DUMPBIOS.PRG

# A
rec A1-oracle-tt-u0 tools/hatari-oracle.sh tt "$PRG" "$PHYS" "$S/A1"
rec A2-oracle-falcontc-x tools/hatari-oracle.sh falcontc "$PRG" "$HERE/x-plain.tsv" "$S/A2"
[ -e "$S/A1" ] || [ -e "$S/A2" ] && echo "A: an out-dir was created" >> "$R/results/A-outdirs.txt"

# B
rec B1-xtask-u0-nodisk cargo xtask tosdump bios "$PHYS" --out "$S/B1" --scratch "$S/xtask"
rec B2-xtask-x-nodisk cargo xtask tosdump bios "$FS22" --out "$S/B2" --scratch "$S/xtask"

# C
C=$S/C
pc() {   # pc <dir> <ref-stem>
	mkdir -p "$C/$1"; cp "$2.BIN" "$C/$1/BIOS.BIN"; cp "$2.LOG" "$C/$1/BIOS.LOG"
}
pc ok "$REF-BIOS-r2-phys-disk"
rec C0-u0-pinned "$TOSDIFF" --check-profile "$PHYS" "$C/ok/BIOS.BIN"
pc nolog "$REF-BIOS-r2-phys-disk"; rm "$C/nolog/BIOS.LOG"
rec C1-u0-log-missing "$TOSDIFF" --check-profile "$PHYS" "$C/nolog/BIOS.BIN"
pc noline "$REF-BIOS-r2-phys-disk"; sed '/^unit token/d' "$C/noline/BIOS.LOG" > "$C/l" && mv "$C/l" "$C/noline/BIOS.LOG"
rec C2-u0-no-unit-line "$TOSDIFF" --check-profile "$PHYS" "$C/noline/BIOS.BIN"
pc wrong "$REF-BIOS-r2-phys-disk"; sed 's/physical unit 18/physical unit 26/' "$C/wrong/BIOS.LOG" > "$C/l" && mv "$C/l" "$C/wrong/BIOS.LOG"
rec C3-u0-wrong-unit "$TOSDIFF" --check-profile "$PHYS" "$C/wrong/BIOS.BIN"
rec C4-compare-wrong-unit "$TOSDIFF" "$PHYS" "$C/ok/BIOS.BIN" "$C/wrong/BIOS.BIN"
rec C5-compare-log-missing "$TOSDIFF" "$PHYS" "$C/nolog/BIOS.BIN" "$C/ok/BIOS.BIN"
pc xok "$REF-BIOS-mc-fs22-disk"
rec C6-x-pinned "$TOSDIFF" --check-profile "$FS22" "$C/xok/BIOS.BIN"
pc xwrong "$REF-BIOS-mc-fs22-disk"; sed 's/x:<path>\.PRG/x:<path>.NAP/' "$C/xwrong/BIOS.LOG" > "$C/l" && mv "$C/l" "$C/xwrong/BIOS.LOG"
rec C7-x-wrong-ext "$TOSDIFF" --check-profile "$FS22" "$C/xwrong/BIOS.BIN"
pc xnoline "$REF-BIOS-mc-fs22-disk"; sed '/^program extension/d' "$C/xnoline/BIOS.LOG" > "$C/l" && mv "$C/l" "$C/xnoline/BIOS.LOG"
rec C8-x-no-ext-line "$TOSDIFF" --check-profile "$FS22" "$C/xnoline/BIOS.BIN"
rec C9-selfcheck-wrong-ext "$TOSDIFF" --selfcheck "$FS22" "$C/xwrong/BIOS.BIN"

# D
printf '#R 02\r\n#Z 01 D:\\DUMPBIOS.NAP@\r\n' > "$S/newdesk-d.inf"
tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$S/newdesk-d.inf" > "$R/mktestdisk.stdout" 2>&1
cp "$S/test.img.manifest" "$R/test.img.manifest"
shasum -a 256 "$PRG" "$S/test.img" "$PHYS" "$FS22" "$HERE/x-plain.tsv" tosdump/profiles.tsv > "$R/inputs.sha256"
O=$S/D1
HATARI_ORACLE_DISK="$S/test.img" rec D1-oracle-disk-u0 tools/hatari-oracle.sh disk "$PRG" "$PHYS" "$O"
mkdir -p "$R/runs/oracle-phys"
for f in BIOS.BIN BIOS.LOG profile.txt disk.sha256 hatari.log; do
	[ -f "$O/$f" ] && cp "$O/$f" "$R/runs/oracle-phys/$f"
done
rm -f "$O/disk.img"
rec D1b-oracle-vs-pinned-payload bash -c "cmp <(head -c 12 '$O/BIOS.BIN') <(head -c 12 '$REF-BIOS-r2-phys-disk.BIN') && \
	cmp <(tail -c +17 '$O/BIOS.BIN') <(tail -c +17 '$REF-BIOS-r2-phys-disk.BIN') && echo 'BIN equal to the pinned reference but the build id (bytes 12-15)'; \
	diff '$O/BIOS.LOG' '$REF-BIOS-r2-phys-disk.LOG'"
rec D2-xtask-disk-u0 cargo xtask tosdump bios "$PHYS" --disk \
	--reference "$REF-BIOS-r2-phys-disk.BIN" --out "$R/runs/newtos-phys" --scratch "$S/xtask"
