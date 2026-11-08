#!/usr/bin/env bash
#
# run-wvprobe.sh -- tosdump/probes/wvprobe.c (TASK/V56): the vex_* default
# vectors and the save / restore of the AES's wheel vector, and on NewTOS the
# desktop's wheel after it.
#
#   tosdump/probes/run-wvprobe.sh emutos <out-dir>
#   tosdump/probes/run-wvprobe.sh kernel <tree> <kernel-out>
#   tosdump/probes/run-wvprobe.sh newtos <out-dir> <kernel8.img> probe|control
#
#   emutos   the pinned oracle, profile falcontc, with tools/hatari-oracle.sh's
#            command line (as run-wkprobe.sh): --auto C:\WVPROBE.PRG, the run
#            ends at the LOG's "END". The LOG lines E1-E3, R1, R2 are the ones
#            to compare; "A " lines carry raw addresses.
#   kernel   copies <tree> (a `git archive` tree, references/ linked) into
#            scratch, arms it with `tools/inject-arm.py <copy>
#            desktop=wvprobe-mouse.txt`, builds `cargo xtask build --board
#            rpi4 --release` and copies kernel8.img to <kernel-out>.
#   newtos   tools/mkcard.sh: C:\LONGNM01.TXT .. LONGNM40.TXT, C:\WVPROBE.NAP
#            and a NEWDESK.INF that opens a window on C:\ (the V34 desktop
#            scene's INF); `probe` adds "#Z 01 C:\WVPROBE.NAP@", `control`
#            does not. QEMU raspi4b on an APFS clone of the card, -append
#            serial=on. With `probe` the #Z autorun runs WVPROBE first and the
#            desktop comes up when it ends; the armed injector starts at that
#            "desktop up" (the first one) and feeds wvprobe-mouse.txt: the
#            pointer into the window's file list, three wheel-down detents,
#            three wheel-up detents. QMP screendumps from wvprobe-shots.txt.
#            The window must scroll down a row per detent and back: shots
#            00 (before) != 01 (after down), 02 (after up) == 00. With the
#            V56 defect the restored AES wheel vector is taken for "no
#            handler" and 00 == 01 == 02.
#
# <out-dir> (must not exist) receives run.txt, the side's log (hatari.log, or
# BOOT-SERIAL.txt + card.img.manifest + shots/), WVPROBE.LOG (emutos, newtos
# probe) and, for newtos, verdict.txt (the shot byte compares).
# Scratch: $WVPROBE_SCRATCH (required; never /tmp itself). Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos <out-dir> | kernel <tree> <kernel-out> | newtos <out-dir> <kernel8.img> probe|control" >&2
    exit 2
}
[ $# -ge 2 ] || usage
SIDE=$1
case $SIDE in
    emutos) [ $# -eq 2 ] || usage; OUT=$2 ;;
    kernel) [ $# -eq 3 ] || usage; TREE=$2; KOUT=$3; [ -d "$TREE/kernel" ] || { echo "not a tree: $TREE" >&2; exit 1; } ;;
    newtos) [ $# -eq 4 ] || usage; OUT=$2; KERNEL=$3; MODE=$4
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; }
            case $MODE in probe|control) ;; *) usage ;; esac ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
MOUSE=$HERE/wvprobe-mouse.txt
SHOTS=$HERE/wvprobe-shots.txt
BASE=${WVPROBE_SCRATCH:?set WVPROBE_SCRATCH (scratch, never /tmp)}
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/wvprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$TREE" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "desktop=$MOUSE" | tail -1
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (tree $TREE, armed desktop=$(sha "$MOUSE"))"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

# ---- build both halves into scratch ------------------------------------------
. "$REPO/tools/oracle/oracle.env"
PRG=$SCR/WVPROBE.PRG
NAP=$SCR/WVPROBE.NAP
(cd "$TD" && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/wvprobe.c abi/m68k_trap.c -o "$PRG")
SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
[ -f "$LIBC/libc.a" ] || { echo "sdk/libc/libc.a missing (sdk/libc/build-libc.sh)" >&2; exit 1; }
CLANG=${CLANG:-clang}
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/wvprobe.c" -o "$SCR/wvprobe.o"
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$SCR/newtos_svc.o"
"$CLANG" "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$SCR/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$SCR/crt0.o" "$SCR/wvprobe.o" "$SCR/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
[ "$("$BIN/llvm-nm" -u "$NAP" | grep -c . || true)" -eq 0 ] || { echo "WVPROBE.NAP has undefined symbols" >&2; exit 1; }
{
    echo "wvprobe run, side $SIDE${MODE:+ $MODE} ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- tosdump/probes | grep -c . || true) dirty paths in tosdump/probes"
    echo "source    wvprobe.c $(sha "$HERE/wvprobe.c") wvprobe-mouse.txt $(sha "$MOUSE") wvprobe-shots.txt $(sha "$SHOTS")"
    echo "probes    WVPROBE.PRG $(sha "$PRG") WVPROBE.NAP $(sha "$NAP")"
    echo "clang     $($CLANG --version | head -1)"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/WVPROBE.PRG"
    # tools/hatari-oracle.sh's falcontc INF, byte for byte (as run-wkprobe.sh)
    printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
        > "$SCR/hd/EMUDESK.INF"
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), cfg $(sha "$ORACLE_HATARI_CFG"), profile falcontc" >> "$OUT/run.txt"
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" --machine falcon --monitor vga --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\WVPROBE.PRG' \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    state=timeout
    for _ in $(seq 1 240); do
        if [ -s "$SCR/hd/WVPROBE.LOG" ] && grep -q '^END' "$SCR/hd/WVPROBE.LOG"; then
            sleep 1; state=done; break
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.5
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state" >> "$OUT/run.txt"
    [ -s "$SCR/hd/WVPROBE.LOG" ] || { echo "no WVPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    cp "$SCR/hd/WVPROBE.LOG" "$OUT/"
    cat "$OUT/WVPROBE.LOG"
    exit 0
fi

# ---- NewTOS: the card, the boot, the shots ----------------------------------
echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
mkdir -p "$SCR/files"
for i in $(seq -w 1 40); do printf 'V56 desktop file %s\r\n' "$i" > "$SCR/files/LONGNM$i.TXT"; done
# measurement-2026-11-07-v34-test/method/desk-NEWDESK.INF: drive C: icon,
# trash, one window on C:\*.* at the right
printf '#R 02\r\n#E 1B E0 00 00 60\r\n#M 00 00 FF FF C Hard disk@ @\r\n#T 04 00 FF FF   Trash@ @\r\n#W 00 00 2A 06 24 12 00 C:\\*.*@\r\n' > "$SCR/newdesk.inf"
[ "$MODE" = probe ] && printf '#Z 01 C:\\WVPROBE.NAP@\r\n' >> "$SCR/newdesk.inf"
cp "$SCR/newdesk.inf" "$OUT/NEWDESK.INF"
PAIRS=(WVPROBE.NAP="$NAP")
for f in "$SCR"/files/*; do PAIRS+=("$(basename "$f")=$f"); done
tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" "${PAIRS[@]}" > "$SCR/mkcard.stdout" 2>&1 || {
    cat "$SCR/mkcard.stdout" >&2; exit 1; }
cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\WVPROBE.NAP, 40 files, mode $MODE)" >> "$OUT/run.txt"
cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
mkdir -p "$OUT/shots"
cp "$SHOTS" "$OUT/shots.txt"
: > "$OUT/BOOT-SERIAL.txt"
# The QMP socket is relative to $SCR: a Unix socket path must stay under 104
# bytes on macOS, and the session scratchpad's path alone is longer.
cd "$SCR"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
    -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
    -qmp unix:qmp.sock,server=on,wait=off &
pid=$!
echo "qemu-pid  $pid" >> "$OUT/run.txt"
for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done
python3 -I "$HERE/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" \
    "[inj] desktop armed at hz_200" || echo "qmpshots  failed" >> "$OUT/run.txt"
sleep 1
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
cd "$REPO"
echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
grep -a -c -i -E "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exc-panic /' >> "$OUT/run.txt" || true
if [ "$MODE" = probe ]; then
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" "::/WVPROBE.LOG" "$OUT/WVPROBE.LOG" ||
        echo "no WVPROBE.LOG on C:" >> "$OUT/run.txt"
fi
# The verdict: byte compares of the three shots (QEMU's PNG of equal pixels is
# equal bytes; look at the images for the why).
s() { ls "$OUT"/shots/"$1"-*.png 2>/dev/null | head -1; }
{
    a=$(s 00); b=$(s 01); c=$(s 02)
    if [ -z "$a" ] || [ -z "$b" ] || [ -z "$c" ]; then
        echo "verdict   MISSING SHOTS"
    else
        d1=same; cmp -s "$a" "$b" || d1=differ
        d2=same; cmp -s "$a" "$c" || d2=differ
        echo "00 vs 01  $d1 (wheel down: the window must scroll -> differ)"
        echo "00 vs 02  $d2 (wheel back up: the window must return -> same)"
        if [ "$d1" = differ ] && [ "$d2" = same ]; then echo "verdict   WHEEL SCROLLS"; else echo "verdict   WHEEL DEAD OR UNEXPECTED"; fi
    fi
} > "$OUT/verdict.txt"
cat "$OUT/run.txt" "$OUT/verdict.txt"
[ -f "$OUT/WVPROBE.LOG" ] && cat "$OUT/WVPROBE.LOG"
exit 0
