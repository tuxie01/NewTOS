#!/usr/bin/env bash
#
# run-mnprobe.sh -- the menu-selection latency probe tosdump/probes/mnprobe.c
# (TASK/A127 D1) on one side, with the mouse input of
# tosdump/probes/mnprobe-mouse.txt. Copied from run-akprobe.sh; no Makefile
# target (the probe is built here, with the Makefile's m68k probe command line
# and build-sbprobe-newtos.sh's link path).
#
#   tosdump/probes/run-mnprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-mnprobe.sh kernel <commit> <kernel-out> [dbg]
#   tosdump/probes/run-mnprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle, started as run-akprobe.sh starts it (ROM,
#            tools/oracle/hatari.cfg, hermetic HOME, GEMDOS drive C: with
#            MNPROBE.PRG and MNPROBE.INJ = mnprobe-mouse.txt, --auto
#            C:\MNPROBE.PRG; falcontc's EMUDESK.INF), --fast-forward off. No
#            --cmd-fifo: Hatari 2.6.1 has no left-button or motion event; the
#            probe feeds the packets to Kbdvbase()->mousevec itself (mnprobe.c).
#   kernel   `git archive <commit>` into scratch (references/ linked), armed
#            by `tools/inject-arm.py <copy> launch:MNPROBE=mnprobe-mouse.txt`,
#            with `dbg`: plus measurements/measurement-2026-11-04-a127-d1-test/
#            method/dbg-patch.py (prints "[dbg] track end hz_200 <n>"), built
#            `cargo xtask build --board rpi4 --release`; kernel8.img is copied
#            to <kernel-out>.
#   newtos   tools/mkcard.sh (C:\MNPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\MNPROBE.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun
#            starts the probe, the armed kernel feeds the mouse.
#
# MNPROBE_E1=<hex byte> (optional) puts "#E <hex> E0 00 00 60" into the INF on
# both sides (EmuTOS default desk_inf_data1 is "#E 1A E0 00 00 60",
# desk/deskapp.c:173; the low 3 bits are the double-click index,
# aes/geminit.c:420-425): on the oracle an EMUDESK.INF "#R 02" + that line for
# tt (falcontc keeps its own INF, whose #E is 1A), on NewTOS the line goes into
# C:\NEWDESK.INF before #Z. Unset: tt uses Hatari's virtual EMUDESK.INF
# (2.6.1: "#E 1A E1 FF 00 00"), NewTOS "#R 02" + "#Z" only.
#
# MNPROBE_MFIRST=1 puts an empty MNPROBE.MFS next to the probe on both sides
# (the probe's segment order M, D2, M, D2, D1 instead of D2, M, D2, M, D1).
#
# Oracle cross-check through the IKBD path: MNPROBE_INJ=<file> replaces
# C:\MNPROBE.INJ (a comment-only file = the probe feeds nothing) and
# MNPROBE_FIFO=<file> gives "<pause ms> <hatari-event args>" lines that are
# sent as "hatari-event <args>" through --cmd-fifo, the first 0.5 s after
# C:\MNPROBE.RDY appears (keydown/keyup only: EmuTOS's Alt-arrow / Alt-Insert
# mouse emulation, bios/ikbd.c:285-389). The run ends 5 s after the last one.
#
# <out-dir> (must not exist) receives MNPROBE.LOG, run.txt and the side's log
# (hatari.log, or BOOT-SERIAL.txt + card.img.manifest).
# Scratch: $MNPROBE_SCRATCH (required; never /tmp itself). Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos tt|falcontc <out-dir> | kernel <commit> <kernel-out> [dbg] | newtos <out-dir> <kernel8.img>" >&2
    exit 2
}
[ $# -ge 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) [ $# -eq 3 ] || usage; PROFILE=$2; OUT=$3
            case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    kernel) [ $# -le 4 ] || usage; COMMIT=$2; KOUT=$3; DBG=${4:-} ;;
    newtos) [ $# -eq 3 ] || usage; OUT=$2; KERNEL=$3
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
MOUSE=$HERE/mnprobe-mouse.txt
BASE=${MNPROBE_SCRATCH:?set MNPROBE_SCRATCH (scratch, never /tmp)}
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/mnprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$REPO" && git archive "$COMMIT") | tar -x -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "launch:MNPROBE=$MOUSE" | tail -1
    if [ "$DBG" = dbg ]; then
        python3 -I "$REPO/measurements/measurement-2026-11-04-a127-d1-test/method/dbg-patch.py" "$T"
    fi
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (commit $(cd "$REPO" && git rev-parse "$COMMIT"), armed launch:MNPROBE, ${DBG:-no dbg})"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

# ---- build both halves into scratch ------------------------------------------
. "$REPO/tools/oracle/oracle.env"
PRG=$SCR/MNPROBE.PRG
NAP=$SCR/MNPROBE.NAP
(cd "$TD" && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/mnprobe.c abi/m68k_trap.c -o "$PRG")
SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
[ -f "$LIBC/libc.a" ] || { echo "sdk/libc/libc.a missing (sdk/libc/build-libc.sh)" >&2; exit 1; }
CLANG=${CLANG:-clang}
RES="$($CLANG -print-resource-dir)/include"
SYSROOT="$(rustc --print sysroot)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/mnprobe.c" -o "$SCR/mnprobe.o"
"$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$SCR/newtos_svc.o"
"$CLANG" "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$SCR/crt0.o"
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$SCR/crt0.o" "$SCR/mnprobe.o" "$SCR/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
[ "$("$BIN/llvm-nm" -u "$NAP" | grep -c . || true)" -eq 0 ] || { echo "MNPROBE.NAP has undefined symbols" >&2; exit 1; }
{
    echo "mnprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- tosdump/probes | grep -c . || true) dirty paths in tosdump/probes"
    echo "source    mnprobe.c $(sha "$HERE/mnprobe.c") mnprobe-mouse.txt $(sha "$MOUSE")"
    echo "probes    MNPROBE.PRG $(sha "$PRG") MNPROBE.NAP $(sha "$NAP")"
    echo "clang     $($CLANG --version | head -1)"
    echo "inf-E1    ${MNPROBE_E1:-unset}"
    echo "m-first   ${MNPROBE_MFIRST:-no}"
    [ -z "${MNPROBE_INJ:-}" ] || echo "inj       $MNPROBE_INJ $(sha "$MNPROBE_INJ")"
    [ -z "${MNPROBE_FIFO:-}" ] || echo "fifo      $MNPROBE_FIFO $(sha "$MNPROBE_FIFO")"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    case $PROFILE in
        tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
        falcontc) MACHINE="--machine falcon --monitor vga" ;;
    esac
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/MNPROBE.PRG"
    cp "${MNPROBE_INJ:-$MOUSE}" "$SCR/hd/MNPROBE.INJ"
    [ -z "${MNPROBE_MFIRST:-}" ] || : > "$SCR/hd/MNPROBE.MFS"
    FIFO=()
    [ -z "${MNPROBE_FIFO:-}" ] || FIFO=(--cmd-fifo "$SCR/cmd")
    if [ "$PROFILE" = tt ] && [ -n "${MNPROBE_E1:-}" ]; then
        printf '#R 02\r\n#E %s E0 00 00 60\r\n' "$MNPROBE_E1" > "$SCR/hd/EMUDESK.INF"
    fi
    if [ "$PROFILE" = falcontc ]; then
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward off --conout 2 --run-vbls 60000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\MNPROBE.PRG' ${FIFO[@]+"${FIFO[@]}"} \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    state=timeout
    sent=
    for _ in $(seq 1 1500); do
        if [ -s "$SCR/hd/MNPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/MNPROBE.LOG"; then
            sleep 1; state=done; break
        fi
        if [ -n "${MNPROBE_FIFO:-}" ] && [ -z "$sent" ] && [ -e "$SCR/hd/MNPROBE.RDY" ]; then
            sleep 0.5
            : > "$OUT/fifo.txt"
            sed 's/[[:space:]]*#.*//' "$MNPROBE_FIFO" | grep . | while read -r ms ev; do
                sleep "$(LC_ALL=C awk -v d="$ms" 'BEGIN { printf "%.3f", d / 1000 }')"
                echo "hatari-event $ev" > "$SCR/cmd"
                echo "$ms hatari-event $ev" >> "$OUT/fifo.txt"
            done
            sent=1
            sleep 5; state=fifo-done; break
        fi
        kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
        sleep 0.1
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state" >> "$OUT/run.txt"
    [ -s "$SCR/hd/MNPROBE.LOG" ] || { echo "no MNPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/MNPROBE.LOG" > "$OUT/MNPROBE.LOG"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    if [ -n "${MNPROBE_E1:-}" ]; then
        printf '#R 02\r\n#E %s E0 00 00 60\r\n#Z 01 C:\\MNPROBE.NAP@\r\n' "$MNPROBE_E1" > "$SCR/newdesk.inf"
    else
        printf '#R 02\r\n#Z 01 C:\\MNPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    fi
    MFS=()
    if [ -n "${MNPROBE_MFIRST:-}" ]; then : > "$SCR/MNPROBE.MFS"; MFS=(MNPROBE.MFS="$SCR/MNPROBE.MFS"); fi
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" MNPROBE.NAP="$NAP" ${MFS[@]+"${MFS[@]}"} > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\MNPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 120); do
        sleep 2
        if grep -aq "MNPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/MNPROBE.LOG "$SCR/MNPROBE.LOG" \
        || { echo "no MNPROBE.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/MNPROBE.LOG" > "$OUT/MNPROBE.LOG"
fi
cat "$OUT/MNPROBE.LOG"
