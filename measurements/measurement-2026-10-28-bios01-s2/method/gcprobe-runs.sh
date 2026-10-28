#!/bin/bash
# gcprobe-runs.sh <scratch> <runs-dir> <kernel8.img> <label> [emutos] --
# gcprobe.c on both sides with the SAME test image as C: (tools/mktestdisk.sh,
# autorun INF "#R 02 / #Z 01 D:\GCPROBE.NAP@", inert on EmuTOS) and the probe
# on D:. <scratch> holds GCPROBE.PRG / GCPROBE.NAP (gcprobe-build.sh).
#   EmuTOS (with "emutos"): the pinned ROM + Hatari + tools/oracle/hatari.cfg as
#     tools/hatari-oracle.sh profile disk runs them (Falcon030, VGA, both floppy
#     drives off), C: = --ide-master <APFS clone of test.img>, D: = Hatari GEMDOS
#     drive with the probe, --auto D:\GCPROBE.PRG; the method of
#     measurements/measurement-2026-10-28-bios-storage-audit/method/run-emutos.sh.
#   NewTOS: tools/mkcard.sh with MKCARD_TESTDISK (C: = the test image byte for
#     byte, D: = program partition with GCPROBE.NAP), QEMU raspi4b booting an
#     APFS clone, #Z autorun; GCPROBE.LOG is copied from D: after "desktop up".
# Kept: <runs-dir>/gc-<side>-<label>/ GCPROBE.LOG, serial / hatari.log,
# testpart.sha256; the test partitions (LBA 63, 130977) go to
# <scratch>/end/gc-<side>-<label>.img for gcprobe-compare.sh.
set -u
S=$1; R=$2; K=$3; L=$4; E=${5:-}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
cd "$REPO"
mkdir -p "$S/end"
printf '#R 02\r\n#Z 01 D:\\GCPROBE.NAP@\r\n' > "$S/newdesk-gc.inf"
[ -f "$S/test.img" ] || tools/mktestdisk.sh "$S/test.img" NEWDESK.INF="$S/newdesk-gc.inf" > "$S/mktestdisk.stdout"
cp "$S/test.img.manifest" "$R/gc-test.img.manifest"
dd if="$S/test.img" of="$S/end/gc-original.img" bs=512 skip=63 count=130977 2>/dev/null

if [ "$E" = emutos ]; then
    . tools/oracle/oracle.env
    [ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
    O=$R/gc-emutos-$L; mkdir -p "$O" "$S/emu-$L/hd" "$S/emu-$L/home"
    cp "$S/GCPROBE.PRG" "$S/emu-$L/hd/GCPROBE.PRG"
    IMG=$S/emu-$L/disk.img; rm -f "$IMG"; cp -c "$S/test.img" "$IMG"
    HOME="$S/emu-$L/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" --machine falcon --monitor vga --tos "$ORACLE_ROM" \
        --drive-a off --drive-b off \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        --ide-master "$IMG" -d "$S/emu-$L/hd" --gemdos-drive D --auto 'D:\GCPROBE.PRG' \
        > "$O/hatari.log" 2>&1 &
    PID=$!
    i=0
    while kill -0 "$PID" 2>/dev/null; do
        [ -s "$S/emu-$L/hd/GCPROBE.LOG" ] && { sleep 2; break; }
        i=$((i + 1)); [ "$i" -lt 240 ] || break
        sleep 0.5
    done
    kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
    cp "$S/emu-$L/hd/GCPROBE.LOG" "$O/GCPROBE.LOG" 2>/dev/null || echo "no GCPROBE.LOG" > "$O/GCPROBE.LOG"
    dd if="$IMG" of="$S/end/gc-emutos-$L.img" bs=512 skip=63 count=130977 2>/dev/null
    shasum -a 256 "$S/end/gc-emutos-$L.img" | cut -d' ' -f1 > "$O/testpart.sha256"
    rm -rf "$S/emu-$L"
else
    O=$R/gc-newtos-$L; mkdir -p "$O"
    rm -f "$S/card-$L.img"
    MKCARD_TESTDISK="$S/test.img" tools/mkcard.sh "$S/card-$L.img" "$S/newdesk-gc.inf" \
        GCPROBE.NAP="$S/GCPROBE.NAP" > "$O/mkcard.stdout" 2>&1
    cp "$S/card-$L.img.manifest" "$O/card.img.manifest"
    shasum -a 256 "$K" > "$O/kernel8.sha256"
    IMG=$S/card-$L-boot.img; rm -f "$IMG"; cp -c "$S/card-$L.img" "$IMG"
    : > "$O/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$O/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$K" -drive file="$IMG",if=sd,format=raw &
    pid=$!
    for i in $(seq 1 120); do
        sleep 2
        /usr/bin/grep -q "GCPROBE.NAP.*exited\|exited with rc" "$O/BOOT-SERIAL.txt" \
            && /usr/bin/grep -q "desktop up" "$O/BOOT-SERIAL.txt" && break
        kill -0 $pid 2>/dev/null || break
    done
    sleep 3; kill $pid 2>/dev/null; sleep 1; kill -9 $pid 2>/dev/null; wait $pid 2>/dev/null
    export MTOOLS_SKIP_CHECK=1
    mcopy -o -i "$IMG@@$((131072 * 512))" ::/GCPROBE.LOG "$O/GCPROBE.LOG" || echo "no GCPROBE.LOG" > "$O/GCPROBE.LOG"
    dd if="$IMG" of="$S/end/gc-newtos-$L.img" bs=512 skip=63 count=130977 2>/dev/null
    shasum -a 256 "$S/end/gc-newtos-$L.img" | cut -d' ' -f1 > "$O/testpart.sha256"
    rm -f "$IMG" "$S/card-$L.img" "$S/card-$L.img.manifest"
fi
cat "$O/GCPROBE.LOG"
