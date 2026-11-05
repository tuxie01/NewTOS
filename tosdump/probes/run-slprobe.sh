#!/usr/bin/env bash
#
# run-slprobe.sh -- the window scroll-bar probe tosdump/probes/slprobe.c
# (TASK/A130, GitHub #2) on one side, with the mouse input
# tosdump/probes/slprobe-mouse.txt and the screenshot schedule
# tosdump/probes/slprobe-shots.txt. Copied from run-mgprobe.sh (names only).
# SLPROBE_MOUSE / SLPROBE_SHOTS override the two input files.
#
#   tosdump/probes/run-slprobe.sh emutos <out-dir>
#   tosdump/probes/run-slprobe.sh kernel <commit> <kernel-out>
#   tosdump/probes/run-slprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle, profile tt (--machine tt --tos-res ttmed
#            --monitor vga), ROM checked against tools/oracle/oracle.env,
#            tools/oracle/hatari.cfg, hermetic HOME, --fast-forward off, GEMDOS
#            drive C: with SLPROBE.PRG, SLPROBE.INJ (= slprobe-mouse.txt),
#            SLPROBE.SHT (= slprobe-shots.txt), EMUDESK.INF "#R 02" + "#E 1B E0
#            00 00 60", --auto C:\SLPROBE.PRG. Screen capture: --statusbar off
#            --crop on (capture options only). The probe writes the address of
#            its shot counter to C:\SLPROBE.ADR; this script sets a Hatari
#            tracking breakpoint on it through --cmd-fifo (hatari-debug b
#            (<addr>).l ! (<addr>).l :trace :file shot.ini, shot.ini =
#            "screenshot <scratch>/shot.png"), creates C:\SLPROBE.GO, and
#            renames every shot.png to <out>/shots/<NN>-<name>.png in schedule
#            order. So each oracle shot falls on its exact emulated tick.
#   kernel   `git archive <commit>` into scratch (references/ linked), armed
#            by `tools/inject-arm.py <copy> launch:SLPROBE=slprobe-mouse.txt`,
#            built `cargo xtask build --board rpi4 --release`; kernel8.img is
#            copied to <kernel-out>.
#   newtos   tools/mkcard.sh (C:\SLPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#E 1B E0 00 00 60", "#Z 01 C:\SLPROBE.NAP@"), QEMU raspi4b booting
#            an APFS clone of the card with `-append serial=on` and a QMP
#            socket; slprobe-qmpshots.py takes the screendumps at the schedule's
#            ticks after the injector's "armed" line.
#
# <out-dir> (must not exist) receives SLPROBE.LOG, run.txt, shots/ and the
# side's log (hatari.log, or BOOT-SERIAL.txt + card.img.manifest).
# Scratch: $SLPROBE_SCRATCH (required; never /tmp itself). Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos <out-dir> | kernel <commit> <kernel-out> | newtos <out-dir> <kernel8.img>" >&2
    exit 2
}
[ $# -ge 2 ] || usage
SIDE=$1
case $SIDE in
    emutos) [ $# -eq 2 ] || usage; OUT=$2 ;;
    kernel) [ $# -eq 3 ] || usage; COMMIT=$2; KOUT=$3 ;;
    newtos) [ $# -eq 3 ] || usage; OUT=$2; KERNEL=$3
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
MOUSE=${SLPROBE_MOUSE:-$HERE/slprobe-mouse.txt}
SHOTS=${SLPROBE_SHOTS:-$HERE/slprobe-shots.txt}
BASE=${SLPROBE_SCRATCH:?set SLPROBE_SCRATCH (scratch, never /tmp)}
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/slprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$REPO" && git archive "$COMMIT") | tar -x -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "launch:SLPROBE=$MOUSE" | tail -1
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (commit $(cd "$REPO" && git rev-parse "$COMMIT"), armed launch:SLPROBE)"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

# ---- build both halves into scratch ------------------------------------------
. "$REPO/tools/oracle/oracle.env"
PRG=$SCR/SLPROBE.PRG
NAP=$SCR/SLPROBE.NAP
(cd "$TD" && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/slprobe.c abi/m68k_trap.c -o "$PRG")
SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
[ -f "$LIBC/libc.a" ] || { echo "sdk/libc/libc.a missing (sdk/libc/build-libc.sh)" >&2; exit 1; }
CLANG=${CLANG:-clang}
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/slprobe.c" -o "$SCR/slprobe.o"
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$SCR/newtos_svc.o"
"$CLANG" "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$SCR/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$SCR/crt0.o" "$SCR/slprobe.o" "$SCR/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
[ "$("$BIN/llvm-nm" -u "$NAP" | grep -c . || true)" -eq 0 ] || { echo "SLPROBE.NAP has undefined symbols" >&2; exit 1; }
{
    echo "slprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain | grep -c . || true) dirty paths in the tree"
    echo "source    slprobe.c $(sha "$HERE/slprobe.c") slprobe-mouse.txt $(sha "$MOUSE") slprobe-shots.txt $(sha "$SHOTS")"
    echo "probes    SLPROBE.PRG $(sha "$PRG") SLPROBE.NAP $(sha "$NAP")"
    echo "clang     $($CLANG --version | head -1)"
} > "$OUT/run.txt"
INF_E='#E 1B E0 00 00 60'

if [ "$SIDE" = emutos ]; then
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/SLPROBE.PRG"
    cp "$MOUSE" "$SCR/hd/SLPROBE.INJ"
    cp "$SHOTS" "$SCR/hd/SLPROBE.SHT"
    printf '#R 02\r\n%s\r\n' "$INF_E" > "$SCR/hd/EMUDESK.INF"
    printf 'screenshot %s/shot.png\n' "$SCR" > "$SCR/shot.ini"
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile tt" >> "$OUT/run.txt"
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" --machine tt --tos-res ttmed --monitor vga --tos "$ORACLE_ROM" \
        --sound off --fast-forward off --conout 2 --run-vbls 200000 --statusbar off --drive-led off --crop on \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\SLPROBE.PRG' --cmd-fifo "$SCR/cmd" \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    names=()
    while read -r tk name; do names+=("$name"); done < <(sed 's/#.*//' "$SHOTS" | grep .)
    state=timeout; armed=; n=0
    for _ in $(seq 1 60000); do
        if [ -z "$armed" ] && [ -s "$SCR/hd/SLPROBE.ADR" ]; then
            addr=$(tr -d '\r\n ' < "$SCR/hd/SLPROBE.ADR")
            echo "hatari-debug b (\$$addr).l ! (\$$addr).l :trace :file $SCR/shot.ini" > "$SCR/cmd"
            echo "breakpoint (\$$addr).l ! (\$$addr).l :trace :file shot.ini" >> "$OUT/run.txt"
            sleep 1
            : > "$SCR/hd/SLPROBE.GO"
            armed=1
        fi
        if [ -e "$SCR/shot.png" ] && [ "$(grep -c "Screen dump to '$SCR/shot.png' succeeded" "$OUT/hatari.log" || true)" -gt "$n" ]; then
            cp "$SCR/shot.png" "$OUT/shots/$(printf '%02d' "$n")-${names[$n]:-extra}.png"; rm -f "$SCR/shot.png"
            n=$((n + 1))
        fi
        if [ -s "$SCR/hd/SLPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/SLPROBE.LOG"; then
            sleep 1; state=done; break
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.02
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state, $n shots of ${#names[@]}" >> "$OUT/run.txt"
    [ -s "$SCR/hd/SLPROBE.LOG" ] || { echo "no SLPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/SLPROBE.LOG" > "$OUT/SLPROBE.LOG"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n%s\r\n#Z 01 C:\\SLPROBE.NAP@\r\n' "$INF_E" > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" SLPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\SLPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    # the QMP socket path is relative to $SCR: a unix socket path must stay below 104 bytes
    cd "$SCR"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
        -qmp unix:qmp.sock,server=on,wait=off &
    pid=$!
    python3 -I "$HERE/slprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$SHOTS" "$OUT/shots" &
    spid=$!
    cd "$REPO"
    state=timeout
    for _ in $(seq 1 400); do
        sleep 2
        if grep -aq "SLPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
    sleep 2
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state, $(ls "$OUT/shots" | grep -c png || true) shots" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/SLPROBE.LOG "$SCR/SLPROBE.LOG" \
        || { echo "no SLPROBE.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/SLPROBE.LOG" > "$OUT/SLPROBE.LOG"
fi
cat "$OUT/SLPROBE.LOG" | grep -v '^[JK]' | head -80
