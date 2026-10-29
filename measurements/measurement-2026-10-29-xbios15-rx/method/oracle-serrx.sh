#!/bin/sh
# oracle-serrx.sh <tt|falcontc> <SERRX.PRG> <out-dir> [<bytes>] -- XBIOS15: the serrx probe
# under the pinned EmuTOS on Hatari with bytes on the serial line.
# Machine, ROM, config, hermetic HOME, GEMDOS drive C: and --auto start are exactly
# tools/hatari-oracle.sh's (profiles tt and falcontc, incl. the falcontc EMUDESK.INF);
# changed after measurements/measurement-2026-10-29-xbios01/method/oracle-flush.sh:
#   --fast-forward off (real-time VBLs, the probe's Vsync wait lasts seconds),
#   the serial input: a FIFO <out>/serin, held open by a writer started before Hatari,
#     tt:       --rs232-in <fifo>   (the ST MFP USART = maptable[0], Bconmap device 6,
#                                    EmuTOS' mapped device on the TT, bios/serport.c:966)
#     falcontc: --scc-b-in <fifo>   (SCC channel B = maptable[1], Bconmap device 7,
#                                    EmuTOS' mapped device on the Falcon, :966,975-977)
#   plus the matching output option (--rs232-out / --scc-b-out <out>/serout.bin): without
#   it Hatari opens its default output /dev/modem, fails and disables RS-232 altogether
#   ("RS232 input or output file open failed. RS232 support disabled.", first attempt).
#   When <out>/hd/SERRX.RDY exists (the probe's ready marker, written before its
#   Vsync wait), <bytes> (printf format, default 'aZ1'; 'none' = inject nothing)
#   are written into the FIFO.
#   ORACLE_SERRX_TRACE=1 adds --trace bios,xbios --trace-file <out>/trace.txt (the
#   oracle-side input check: the BIOS/XBIOS calls and arguments as EmuTOS received them).  The run ends when SERRX.LOG holds "Z99 end".
set -eu
PROFILE=$1; PRG=$2; OUT=$3; BYTES=${4:-aZ1}
HERE=$(cd "$(dirname "$0")" && pwd); REPO=$(cd "$HERE/../../.." && pwd)
. "$REPO/tools/oracle/oracle.env"
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo "ROM mismatch" >&2; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists: $OUT" >&2; exit 1; }
case $PROFILE in
    tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga"; SER=--rs232-in; SEROUT=--rs232-out ;;
    falcontc) MACHINE="--machine falcon --monitor vga"; SER=--scc-b-in; SEROUT=--scc-b-out ;;
    *) echo "profile: tt or falcontc" >&2; exit 2 ;;
esac
mkdir -p "$OUT/hd" "$OUT/home"; OUT=$(cd "$OUT" && pwd)
cp "$PRG" "$OUT/hd/SERRX.PRG"
if [ "$PROFILE" = falcontc ]; then
    printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
        > "$OUT/hd/EMUDESK.INF"
fi
TRACE=""
[ "${ORACLE_SERRX_TRACE:-0}" = 1 ] && TRACE="--trace bios,xbios --trace-file $OUT/trace.txt"
mkfifo "$OUT/serin"
sleep 900 > "$OUT/serin" &
HOLD=$!
echo "serial input: $SER $OUT/serin (fifo); bytes: $BYTES" > "$OUT/inject.txt"
# shellcheck disable=SC2086
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
    --sound off --fast-forward off --conout 2 --run-vbls 60000 \
    $TRACE $SER "$OUT/serin" $SEROUT "$OUT/serout.bin" \
    -d "$OUT/hd" --gemdos-drive C --auto 'C:\SERRX.PRG' \
    > "$OUT/hatari.log" 2>&1 &
PID=$!; i=0; sent=0
while kill -0 "$PID" 2>/dev/null; do
    if [ $sent = 0 ] && [ -e "$OUT/hd/SERRX.RDY" ]; then
        sleep 0.3
        if [ "$BYTES" != none ]; then
            # shellcheck disable=SC2059
            ( printf "$BYTES" > "$OUT/serin" ) &
            W=$!; sleep 1; kill -0 $W 2>/dev/null && { echo "writer still blocked" >> "$OUT/inject.txt"; kill -9 $W; }
        fi
        sent=1; echo "bytes written at poll $i ($(date +%H:%M:%S))" >> "$OUT/inject.txt"
    fi
    if [ -s "$OUT/hd/SERRX.LOG" ] && grep -q '^Z99 end' "$OUT/hd/SERRX.LOG"; then
        sleep 1; break
    fi
    i=$((i + 1)); [ "$i" -lt 900 ] || { echo "timeout at poll $i" >> "$OUT/inject.txt"; break; }
    sleep 0.1
done
kill -9 "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true
kill -9 "$HOLD" 2>/dev/null || true; wait "$HOLD" 2>/dev/null || true
rm -rf "$OUT/home"; rm -f "$OUT/serin"
[ -s "$OUT/hd/SERRX.LOG" ] || { echo "no SERRX.LOG -- see $OUT/hatari.log" >&2; exit 1; }
cp "$OUT/hd/SERRX.LOG" "$OUT/SERRX.LOG"
cat "$OUT/SERRX.LOG"
