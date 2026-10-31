#!/usr/bin/env bash
#
# run-dcprobe.sh -- the Dcreate cost probe tosdump/probes/dcprobe.c (TASK/FS26)
# on one side, C: = a card-sized FAT16 partition.
#
#   tosdump/probes/run-dcprobe.sh emutos <out-dir>
#   tosdump/probes/run-dcprobe.sh newtos auto|gem <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle: tools/hatari-oracle.sh profile disk (Falcon030,
#            the card image below cloned as IDE master C:, the probe on the
#            GEMDOS drive D:, --auto D:\DCPROBE.PRG). The probe counts EmuTOS'
#            sector accesses through hdv_rw itself.
#   newtos   QEMU raspi4b with `-append serial=on` on an APFS clone of the card.
#            auto: C:\AUTO\DCPROBE.NCP, run before the AES (NEWDESK.INF
#            "#R 02" only); gem: C:\DCPROBE.NAP, started by the desktop's #Z
#            autorun. For the sector accesses build the kernel with
#            `cargo xtask build --board rpi4 --release --features fs-io-trace`;
#            its [fsio] lines are in <out-dir>/BOOT-SERIAL.txt.
#
# The card (tools/mkcard.sh, MKCARD_SIZE_MIB=512, as the real card's 500-MB
# FAT16 partitions; the same command on both sides, the probe file and
# NEWDESK.INF aside): C:\ROOTF01.BIN .. ROOTF24.BIN (1 KiB each, so the root
# directory holds as many entries as the boot partition's firmware files) and
# C:\BALLAST.BIN (16 MiB, so the first free cluster lies behind 16 MiB of used
# ones and the FAT scan for it crosses several FAT sectors). The files are
# generated here, byte-reproducible (fixed patterns, MKCARD_STAMP).
# DCPROBE_CARD_MIB overrides the card size.
#
# <out-dir> (must not exist) receives DCPROBE.LOG, run.txt (what ran, sha256 of
# probe, card, kernel) and the side's log (hatari.log + disk.sha256, or
# BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $DCPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() { echo "usage: $0 emutos <out-dir> | newtos auto|gem <out-dir> <kernel8.img>" >&2; exit 2; }
[ $# -ge 2 ] || usage
SIDE=$1
case $SIDE in
    emutos) [ $# -eq 2 ] || usage; OUT=$2; MODE=gem ;;
    newtos) [ $# -eq 4 ] || usage; MODE=$2; OUT=$3; KERNEL=$4
            case $MODE in auto|gem) ;; *) usage ;; esac
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
BASE=${DCPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set DCPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/dcprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT

mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

make -s -C tosdump dcprobe-m68k dcprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/DCPROBE.PRG
NAP=$TD/build/DCPROBE.NAP

# The card's files (header).
mkdir -p "$SCR/files"
python3 - "$SCR/files" <<'EOF'
import sys
d = sys.argv[1]
for i in range(1, 25):
    open(f"{d}/ROOTF{i:02d}.BIN", "wb").write(bytes((j * 7 + i) & 255 for j in range(1024)))
blk = bytes(range(256)) * 4096
with open(f"{d}/BALLAST.BIN", "wb") as f:
    for _ in range(16):
        f.write(blk)
EOF
PAIRS=()
for i in $(seq -w 1 24); do PAIRS+=("ROOTF$i.BIN=$SCR/files/ROOTF$i.BIN"); done
PAIRS+=("BALLAST.BIN=$SCR/files/BALLAST.BIN")
if [ "$MODE" = auto ]; then
    printf '#R 02\r\n' > "$SCR/newdesk.inf"
    PAIRS+=("AUTO/DCPROBE.NCP=$NAP")
else
    printf '#R 02\r\n#Z 01 C:\\DCPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    PAIRS+=("DCPROBE.NAP=$NAP")
fi
MKCARD_SIZE_MIB=${DCPROBE_CARD_MIB:-512} tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "${PAIRS[@]}" \
    > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
{
    echo "dcprobe run, side $SIDE ($MODE) ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    DCPROBE.PRG $(sha "$PRG") DCPROBE.NAP $(sha "$NAP")"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, MKCARD_SIZE_MIB=${DCPROBE_CARD_MIB:-512}, mode $MODE)"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    HATARI_ORACLE_DISK="$SCR/card.img" tools/hatari-oracle.sh disk "$PRG" - "$SCR/oracle" > "$SCR/oracle.stdout" 2>&1 || {
        cat "$SCR/oracle.stdout" >&2; exit 1; }
    tr -d '\r' < "$SCR/oracle/DCPROBE.LOG" > "$OUT/DCPROBE.LOG"
    cp "$SCR/oracle/hatari.log" "$SCR/oracle/disk.sha256" "$OUT/"
    echo "oracle    tools/hatari-oracle.sh disk (pinned ROM, tools/oracle/hatari.cfg)" >> "$OUT/run.txt"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if [ "$MODE" = auto ] && grep -q "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        if [ "$MODE" = gem ] && grep -q "DCPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then
            state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 2
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state; stopped" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    # The probe writes to its current directory: C:\ for the NAP; for the NCP
    # whatever directory the AUTO runner leaves it in (C:\ or C:\AUTO\).
    for src in ::/DCPROBE.LOG ::/AUTO/DCPROBE.LOG; do
        MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" "$src" "$SCR/DCPROBE.LOG" 2>/dev/null &&
            { echo "log       C:${src#::}" | tr / '\\' >> "$OUT/run.txt"; break; }
    done
    [ -s "$SCR/DCPROBE.LOG" ] || { echo "no DCPROBE.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/DCPROBE.LOG" > "$OUT/DCPROBE.LOG"
    # The partition must still be clean after the probe removed its folders.
    plen=$(od -An -t u4 -j 458 -N 4 "$SCR/boot.img" | tr -d ' ')
    dd if="$SCR/boot.img" of="$SCR/part.img" bs=512 skip="$cstart" count="$plen" 2>/dev/null
    if fsck_msdos -n "$SCR/part.img" > "$OUT/fsck-after.txt" 2>&1; then
        echo "fsck      C: after the run: fsck_msdos -n exit 0" >> "$OUT/run.txt"
    else
        echo "fsck      C: after the run: fsck_msdos -n exit $? (see fsck-after.txt)" >> "$OUT/run.txt"
    fi
fi
cat "$OUT/DCPROBE.LOG"
