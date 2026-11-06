#!/bin/sh
# K19 (TASK/V18 part 2): the default font of a workstation on a screen below 400 lines.
# NOT a pinned oracle profile: the falcontc command line of tools/hatari-oracle.sh with the pinned ROM and
# tools/oracle/hatari.cfg, the INF mode word changed from 0x001C to 0x011C (VIDEL VERTFLAG: 640x240 truecolor
# on VGA), and VDPROBE.SEL "T19" (the probe runs only CAL + T19A). NewTOS: QEMU raspi4b with the
# bcm2835-fb device at 640x240 (-global bcm2835-fb.yres=240), the kernel adopting the reported geometry
# (kernel/src/drivers/bcm2711/framebuffer.rs query_physical_size).
#   k19-240.sh emutos <out-dir>      k19-240.sh newtos <out-dir> <kernel8.img>
set -eu
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
. tools/oracle/oracle.env
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v30t
SIDE=$1; OUT=$2
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
make -s -C tosdump vdprobe-m68k vdprobe-newtos > /dev/null 2>&1
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
SCR=$(mktemp -d "$S/k19.XXXXXX")
if [ "$SIDE" = emutos ]; then
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
    mkdir -p "$SCR/hd" "$SCR/home"
    cp tosdump/build/VDPROBE.PRG "$SCR/hd/"
    printf 'T19' > "$SCR/hd/VDPROBE.SEL"
    printf '#R 02\r\n#E 1A E0 01 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' > "$SCR/hd/EMUDESK.INF"
    cp "$SCR/hd/EMUDESK.INF" "$OUT/EMUDESK.INF"
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" \
        --machine falcon --monitor vga --tos "$ORACLE_ROM" \
        --sound off --fast-forward on --conout 2 --run-vbls 30000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\VDPROBE.PRG' > "$OUT/hatari.log" 2>&1 &
    PID=$!
    i=0
    while kill -0 "$PID" 2>/dev/null; do
        if [ -s "$SCR/hd/VDPROBE.LOG" ] && tail -n 1 "$SCR/hd/VDPROBE.LOG" | grep -q '^END'; then
            sleep 2; kill -9 "$PID" 2>/dev/null || true; break; fi
        i=$((i + 1)); [ "$i" -lt 600 ] || { kill -9 "$PID" 2>/dev/null || true; break; }
        sleep 0.5
    done
    wait "$PID" 2>/dev/null || true
    cp "$SCR/hd/VDPROBE.LOG" "$SCR/hd/VDPROBE.BIN" "$OUT/"
    echo "emutos ROM $(sha "$ORACLE_ROM") cfg $(sha "$ORACLE_HATARI_CFG") VDPROBE.PRG $(sha tosdump/build/VDPROBE.PRG)" > "$OUT/run.txt"
else
    KERNEL=$3
    printf '#R 02\r\n#Z 01 C:\\VDPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    printf 'T19' > "$SCR/VDPROBE.SEL"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" VDPROBE.NAP=tosdump/build/VDPROBE.NAP VDPROBE.SEL="$SCR/VDPROBE.SEL" > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    cp -c "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -global bcm2835-fb.yres=240 \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 120); do
        sleep 2
        if grep -q "VDPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/VDPROBE.LOG "$OUT/VDPROBE.LOG" || true
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/VDPROBE.BIN "$OUT/VDPROBE.BIN" || true
    echo "newtos kernel $(sha "$KERNEL") card $(sha "$SCR/card.img") after $(sha "$SCR/boot.img") qemu $state VDPROBE.NAP $(sha tosdump/build/VDPROBE.NAP)" > "$OUT/run.txt"
fi
rm -rf "$SCR"
cat "$OUT/VDPROBE.LOG" 2>/dev/null || echo "no LOG"
