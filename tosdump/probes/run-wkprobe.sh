#!/usr/bin/env bash
#
# run-wkprobe.sh -- the workstation / mouse-pointer probe tosdump/probes/wkprobe.c
# (TASK/V18 part 3) on one side. No disk. Built here, like run-mnprobe.sh (the
# Makefile's m68k probe command line, build-vdprobe-newtos.sh's link path).
#
#   tosdump/probes/run-wkprobe.sh emutos falcontc|tt <out-dir>
#   tosdump/probes/run-wkprobe.sh kernel <tree> <kernel-out>
#   tosdump/probes/run-wkprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle with tools/hatari-oracle.sh's command line
#            (ROM and sha256 check, tools/oracle/hatari.cfg, hermetic HOME,
#            SDL dummy video, falcontc's EMUDESK.INF, --fast-forward on,
#            --auto C:\WKPROBE.PRG); hatari-oracle.sh itself knows only its
#            listed probes. The probe feeds its mouse packets itself.
#   kernel   copies <tree> (a fresh `git archive` tree, references/ linked)
#            into scratch, arms it with `tools/inject-arm.py <copy>
#            launch:WKPROBE=wkprobe-mouse.txt`, builds `cargo xtask build
#            --board rpi4 --release` and copies kernel8.img to <kernel-out>.
#   newtos   tools/mkcard.sh (C:\WKPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\WKPROBE.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun starts
#            the probe. With an armed kernel the injector feeds the packets;
#            with a plain kernel every wait times out (30 s each).
#
# <out-dir> (must not exist) receives WKPROBE.LOG, WKPROBE.BIN, run.txt and
# the side's log (hatari.log, or BOOT-SERIAL.txt + card.img.manifest).
# Compare with tosdump/probes/wkprobe-compare.py <emutos-out> <newtos-out>.
# Scratch: $WKPROBE_SCRATCH (required; never /tmp itself). Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos falcontc|tt <out-dir> | kernel <tree> <kernel-out> | newtos <out-dir> <kernel8.img>" >&2
    exit 2
}
[ $# -eq 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) PROFILE=$2; OUT=$3; case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    kernel) TREE=$2; KOUT=$3; [ -d "$TREE/kernel" ] || { echo "not a tree: $TREE" >&2; exit 1; } ;;
    newtos) OUT=$2; KERNEL=$3; [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
MOUSE=$HERE/wkprobe-mouse.txt
BASE=${WKPROBE_SCRATCH:?set WKPROBE_SCRATCH (scratch, never /tmp)}
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/wkprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$TREE" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "launch:WKPROBE=$MOUSE" | tail -1
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (tree $TREE, armed launch:WKPROBE=$(sha "$MOUSE"))"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

# ---- build both halves into scratch ------------------------------------------
. "$REPO/tools/oracle/oracle.env"
PRG=$SCR/WKPROBE.PRG
NAP=$SCR/WKPROBE.NAP
(cd "$TD" && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/wkprobe.c abi/m68k_trap.c -o "$PRG")
SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
[ -f "$LIBC/libc.a" ] || { echo "sdk/libc/libc.a missing (sdk/libc/build-libc.sh)" >&2; exit 1; }
CLANG=${CLANG:-clang}
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/wkprobe.c" -o "$SCR/wkprobe.o"
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$SCR/newtos_svc.o"
"$CLANG" "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$SCR/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$SCR/crt0.o" "$SCR/wkprobe.o" "$SCR/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
[ "$("$BIN/llvm-nm" -u "$NAP" | grep -c . || true)" -eq 0 ] || { echo "WKPROBE.NAP has undefined symbols" >&2; exit 1; }
{
    echo "wkprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- tosdump/probes | grep -c . || true) dirty paths in tosdump/probes"
    echo "source    wkprobe.c $(sha "$HERE/wkprobe.c") wkprobe-mouse.txt $(sha "$MOUSE")"
    echo "probes    WKPROBE.PRG $(sha "$PRG") WKPROBE.NAP $(sha "$NAP")"
    echo "clang     $($CLANG --version | head -1)"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    case $PROFILE in
        tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
        falcontc) MACHINE="--machine falcon --monitor vga" ;;
    esac
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/WKPROBE.PRG"
    if [ "$PROFILE" = falcontc ]; then
        # tools/hatari-oracle.sh's falcontc INF, byte for byte
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), cfg $(sha "$ORACLE_HATARI_CFG"), profile $PROFILE" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\WKPROBE.PRG' \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    # Done at "END"; the last case W9 may hang EmuTOS (wkprobe.c k_unpaired):
    # 20 s after its "W9 before" line the run ends as "w9-no-end".
    state=timeout
    w9=0
    for _ in $(seq 1 600); do
        if [ -s "$SCR/hd/WKPROBE.LOG" ] && grep -q '^END' "$SCR/hd/WKPROBE.LOG"; then
            sleep 1; state=done; break
        fi
        if [ -s "$SCR/hd/WKPROBE.LOG" ] && grep -q 'W9 before' "$SCR/hd/WKPROBE.LOG"; then
            w9=$((w9 + 1)); [ "$w9" -lt 40 ] || { state=w9-no-end; break; }
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.5
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state" >> "$OUT/run.txt"
    [ -s "$SCR/hd/WKPROBE.LOG" ] || { echo "no WKPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    cp "$SCR/hd/WKPROBE.LOG" "$SCR/hd/WKPROBE.BIN" "$OUT/"
    # a screenshot-free record of where the run stopped
    tail -n 3 "$OUT/WKPROBE.LOG" > "$OUT/last-lines.txt"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\WKPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" WKPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\WKPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    echo "qemu-pid  $pid" >> "$OUT/run.txt"
    state=timeout
    for _ in $(seq 1 240); do
        sleep 2
        if grep -q "WKPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    for f in WKPROBE.LOG WKPROBE.BIN; do
        MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" "::/$f" "$OUT/$f" \
            || { echo "no $f on C:" >&2; exit 1; }
    done
fi
cat "$OUT/WKPROBE.LOG"
