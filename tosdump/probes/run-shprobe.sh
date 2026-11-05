#!/usr/bin/env bash
#
# run-shprobe.sh -- the window frame / drop shadow probe tosdump/probes/shprobe.c
# (TASK/A131, GitHub #7), and the desktop's own drive windows, on one side.
# Derived from run-slprobe.sh (TASK/A130): same oracle invocation, same shot
# mechanics, plus profile falcontc and the "desk" runs without a probe.
#
#   tosdump/probes/run-shprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-shprobe.sh kernel <commit> <kernel-out>
#   tosdump/probes/run-shprobe.sh newtos <out-dir> <kernel8.img>
#   tosdump/probes/run-shprobe.sh emutos-desk tt|falcontc <out-dir>
#   tosdump/probes/run-shprobe.sh newtos-desk <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle (ROM checked against tools/oracle/oracle.env,
#            tools/oracle/hatari.cfg, hermetic HOME, --fast-forward off), profile
#              tt        --machine tt --tos-res ttmed --monitor vga
#              falcontc  --machine falcon --monitor vga, EMUDESK.INF video word
#                        0x001C (16-bit truecolour 640x480, as tools/hatari-oracle.sh)
#            GEMDOS drive C: with SHPROBE.PRG, EMUDESK.INF "#R 02" + "#E 1B E0 00
#            <vh> <vl>", --auto C:\SHPROBE.PRG; --statusbar off --crop on. The probe
#            writes the address of its shot counter to C:\SHPROBE.ADR; a Hatari
#            tracking breakpoint on it (--cmd-fifo) takes each screenshot at its
#            exact emulated tick; shots renamed in schedule order (shprobe-shots.txt).
#   kernel   `git archive <commit>` into scratch (references/ linked), armed by
#            `tools/inject-arm.py <copy> launch:SHPROBE=shprobe-clock.txt` (one
#            motionless mouse packet: the clock only), `cargo xtask build --board
#            rpi4 --release`; kernel8.img copied to <kernel-out>.
#   newtos   tools/mkcard.sh (C:\SHPROBE.NAP, C:\NEWDESK.INF "#R 02", "#E 1B E0 00
#            00 60", "#Z 01 C:\SHPROBE.NAP@"), QEMU raspi4b on an APFS clone,
#            -append serial=on, QMP screendumps at the schedule's ticks after the
#            injector's "launch:SHPROBE armed" line (shprobe-qmpshots.py).
#   emutos-desk / newtos-desk
#            no probe: the INF additionally carries two #W lines (two drive
#            windows of C:\, reopened by the desktop at start, EmuTOS
#            desk/deskapp.c:889-913 + deskmain.c cnx_get); screenshots after the
#            desktop settled: EmuTOS at 25 s and 30 s wall time (hatari-debug
#            screenshot through the cmd fifo), NewTOS 20 s and 25 s after
#            "[desk] desktop up" on serial. NewTOS boots a plain release kernel.
#
# <out-dir> (must not exist) receives SHPROBE.LOG (probe runs), run.txt,
# shots/ and the side's log. Scratch: $SHPROBE_SCRATCH (required; never /tmp).
set -euo pipefail

usage() {
    echo "usage: $0 emutos tt|falcontc <out> | kernel <commit> <kout> | newtos <out> <kernel8.img>" >&2
    echo "       $0 emutos-desk tt|falcontc <out> | newtos-desk <out> <kernel8.img>" >&2
    exit 2
}
[ $# -ge 2 ] || usage
SIDE=$1
case $SIDE in
    emutos|emutos-desk) [ $# -eq 3 ] || usage; PROFILE=$2; OUT=$3
        case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    kernel) [ $# -eq 3 ] || usage; COMMIT=$2; KOUT=$3 ;;
    newtos|newtos-desk) [ $# -eq 3 ] || usage; OUT=$2; KERNEL=$3
        [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
SHOTS=$HERE/shprobe-shots.txt
CLOCK=$HERE/shprobe-clock.txt
BASE=${SHPROBE_SCRATCH:?set SHPROBE_SCRATCH (scratch, never /tmp)}
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/shprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$REPO" && git archive "$COMMIT") | tar -x -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "launch:SHPROBE=$CLOCK" | tail -1
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (commit $(cd "$REPO" && git rev-parse "$COMMIT"), armed launch:SHPROBE)"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT/shots"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"
. "$REPO/tools/oracle/oracle.env"

# two drive windows of C:\ (EmuTOS #W: hsl vsl x y w h in characters, 8x16)
WINLINES='#W 00 00 02 02 26 0E 00 C:\*.*@\r\n#W 00 00 2A 06 24 12 00 C:\*.*@\r\n'

case $SIDE in emutos|newtos)
    PRG=$SCR/SHPROBE.PRG
    NAP=$SCR/SHPROBE.NAP
    (cd "$TD" && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/shprobe.c abi/m68k_trap.c -o "$PRG")
    SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include
    [ -f "$LIBC/libc.a" ] || { echo "sdk/libc/libc.a missing (sdk/libc/build-libc.sh)" >&2; exit 1; }
    CLANG=${CLANG:-clang}
    RES="$($CLANG -print-resource-dir)/include"
    SYSROOT="$(rustc --print sysroot)"
    BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
    CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
    "$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/shprobe.c" -o "$SCR/shprobe.o"
    "$CLANG" "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$SCR/newtos_svc.o"
    "$CLANG" "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$SCR/crt0.o"
    "$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
        --build-id=none "$SCR/crt0.o" "$SCR/shprobe.o" "$SCR/newtos_svc.o" "$LIBC/libc.a" -o "$NAP"
    [ "$("$BIN/llvm-nm" -u "$NAP" | grep -c . || true)" -eq 0 ] || { echo "SHPROBE.NAP has undefined symbols" >&2; exit 1; }
    ;;
esac
{
    echo "shprobe run, side $SIDE ${PROFILE:-} ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain | grep -c . || true) dirty paths in the tree"
    echo "source    shprobe.c $(sha "$HERE/shprobe.c") shprobe-shots.txt $(sha "$SHOTS") shprobe-clock.txt $(sha "$CLOCK")"
    [ -f "${PRG:-/nonexistent}" ] && echo "probes    SHPROBE.PRG $(sha "$PRG") SHPROBE.NAP $(sha "$NAP")"
    echo "clang     $(${CLANG:-clang} --version | head -1)"
} > "$OUT/run.txt"

case $SIDE in emutos|emutos-desk)
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    mkdir -p "$SCR/hd" "$SCR/home"
    if [ "$PROFILE" = tt ]; then
        MACH=(--machine tt --tos-res ttmed --monitor vga); INF_E='#E 1B E0 00 00 60'
    else
        MACH=(--machine falcon --monitor vga); INF_E='#E 1B E0 00 1C 60'
    fi
    if [ "$SIDE" = emutos ]; then
        cp "$PRG" "$SCR/hd/SHPROBE.PRG"
        printf '#R 02\r\n%s\r\n' "$INF_E" > "$SCR/hd/EMUDESK.INF"
        AUTO=(--auto 'C:\SHPROBE.PRG')
    else
        # some content for the drive windows: two folders and two files
        mkdir -p "$SCR/hd/FOLDER1" "$SCR/hd/FOLDER2"
        printf 'a131\r\n' > "$SCR/hd/FILE1.TXT"; printf 'a131\r\n' > "$SCR/hd/FILE2.TXT"
        printf "#R 02\r\n%s\r\n$WINLINES" "$INF_E" > "$SCR/hd/EMUDESK.INF"
        AUTO=()
    fi
    cp "$SCR/hd/EMUDESK.INF" "$OUT/EMUDESK.INF"
    printf 'screenshot %s/shot.png\n' "$SCR" > "$SCR/shot.ini"
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE (${MACH[*]})" >> "$OUT/run.txt"
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" "${MACH[@]}" --tos "$ORACLE_ROM" \
        --sound off --fast-forward off --conout 2 --run-vbls 200000 --statusbar off --drive-led off --crop on \
        -d "$SCR/hd" --gemdos-drive C ${AUTO[@]+"${AUTO[@]}"} --cmd-fifo "$SCR/cmd" \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    names=()
    while read -r tk name; do names+=("$name"); done < <(sed 's/#.*//' "$SHOTS" | grep .)
    state=timeout; n=0
    if [ "$SIDE" = emutos ]; then
        armed=
        for _ in $(seq 1 30000); do
            if [ -z "$armed" ] && [ -s "$SCR/hd/SHPROBE.ADR" ]; then
                addr=$(tr -d '\r\n ' < "$SCR/hd/SHPROBE.ADR")
                echo "hatari-debug b (\$$addr).l ! (\$$addr).l :trace :file $SCR/shot.ini" > "$SCR/cmd"
                echo "breakpoint (\$$addr).l ! (\$$addr).l :trace :file shot.ini" >> "$OUT/run.txt"
                sleep 1
                : > "$SCR/hd/SHPROBE.GO"
                armed=1
            fi
            if [ -e "$SCR/shot.png" ] && [ "$(grep -c "Screen dump to '$SCR/shot.png' succeeded" "$OUT/hatari.log" || true)" -gt "$n" ]; then
                cp "$SCR/shot.png" "$OUT/shots/$(printf '%02d' "$n")-${names[$n]:-extra}.png"; rm -f "$SCR/shot.png"
                n=$((n + 1))
            fi
            if [ -s "$SCR/hd/SHPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/SHPROBE.LOG"; then
                sleep 1; state=done; break
            fi
            kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
            sleep 0.02
        done
    else
        for k in 25 30; do
            while [ "$SECONDS" -lt "$k" ]; do sleep 0.2; done
            echo "hatari-debug screenshot $SCR/desk-$k.png" > "$SCR/cmd"
            for _ in $(seq 1 100); do [ -s "$SCR/desk-$k.png" ] && break; sleep 0.1; done
            sleep 0.5
            cp "$SCR/desk-$k.png" "$OUT/shots/$(printf '%02d' "$n")-desk-${k}s.png" && n=$((n + 1))
        done
        state=done
    fi
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "hatari    $state, $n shots" >> "$OUT/run.txt"
    if [ "$SIDE" = emutos ]; then
        [ -s "$SCR/hd/SHPROBE.LOG" ] || { echo "no SHPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
        tr -d '\r' < "$SCR/hd/SHPROBE.LOG" > "$OUT/SHPROBE.LOG"
    fi
    ;;
*)
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    INF_E='#E 1B E0 00 00 60'
    if [ "$SIDE" = newtos ]; then
        printf '#R 02\r\n%s\r\n#Z 01 C:\\SHPROBE.NAP@\r\n' "$INF_E" > "$SCR/newdesk.inf"
        EXTRA=(SHPROBE.NAP="$NAP")
    else
        printf "#R 02\r\n%s\r\n$WINLINES" "$INF_E" > "$SCR/newdesk.inf"
        EXTRA=()
    fi
    cp "$SCR/newdesk.inf" "$OUT/NEWDESK.INF"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" ${EXTRA[@]+"${EXTRA[@]}"} > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    cd "$SCR"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
        -qmp unix:qmp.sock,server=on,wait=off &
    pid=$!
    if [ "$SIDE" = newtos ]; then
        python3 -I "$HERE/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$SHOTS" "$OUT/shots" \
            "launch:SHPROBE armed at hz_200" &
    else
        printf '2000 desk-a\n3000 desk-b\n' > "$SCR/desk-shots.txt"
        python3 -I "$HERE/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$SCR/desk-shots.txt" "$OUT/shots" \
            "[desk] desktop up" &
    fi
    spid=$!
    cd "$REPO"
    state=timeout
    for _ in $(seq 1 200); do
        sleep 2
        if [ "$SIDE" = newtos ] && grep -aq "SHPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        if [ "$SIDE" = newtos-desk ] && ! kill -0 "$spid" 2>/dev/null; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
    sleep 2
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state, $(ls "$OUT/shots" | grep -c png || true) shots" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    if [ "$SIDE" = newtos ]; then
        cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
        MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/SHPROBE.LOG "$SCR/SHPROBE.LOG" \
            || { echo "no SHPROBE.LOG on C:" >&2; exit 1; }
        tr -d '\r' < "$SCR/SHPROBE.LOG" > "$OUT/SHPROBE.LOG"
    fi
    ;;
esac
cat "$OUT/run.txt"
[ -f "$OUT/SHPROBE.LOG" ] && grep -v '^[R]' "$OUT/SHPROBE.LOG" | head -80 || true
