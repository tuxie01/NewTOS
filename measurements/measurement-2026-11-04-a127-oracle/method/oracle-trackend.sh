#!/usr/bin/env bash
# oracle-trackend.sh <scratch> <out-dir> <E1> -- the end of menu tracking on the oracle (A127):
# the probe run of tosdump/probes/run-mnprobe.sh emutos tt (same Hatari line, MNPROBE_E1 INF),
# plus the Hatari debugger (--parse) with two tracing breakpoints whose :file prints _hz_200:
#   _mn_do       0x00e38f68  (aes/gemmnlib.c:249; menu tracking starts)
#   _ap_sendmsg  0x00e4003c  (called by ct_msgup right after mn_do returned, aes/gemctrl.c:331-366)
# addresses from references/emutos-oracle/1a7484ab/emutos.map; that build's emutos.img is the
# first 377156 bytes of the pinned ROM (same sha256). Breakpoints stop the emulation, not the
# emulated clock: _hz_200 is unaffected.
set -euo pipefail
S=$1; OUT=$2; E1=$3
REPO=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$REPO"; . tools/oracle/oracle.env
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || exit 1
[ ! -e "$OUT" ] || { echo "exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"; SCR=$(mktemp -d "$S/trackend.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
mkdir -p "$SCR/hd" "$SCR/home"
(cd tosdump && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/mnprobe.c abi/m68k_trap.c -o "$SCR/hd/MNPROBE.PRG")
cp tosdump/probes/mnprobe-mouse.txt "$SCR/hd/MNPROBE.INJ"
printf '#R 02\r\n#E %s E0 00 00 60\r\n' "$E1" > "$SCR/hd/EMUDESK.INF"
printf 'echo "BP mn_do"\nm $4ba-$4be\n' > "$SCR/in.ini"
printf 'echo "BP ap_sendmsg"\nm $4ba-$4be\n' > "$SCR/send.ini"
printf 'b pc = $e38f68 :trace :quiet :file %s\nb pc = $e4003c :trace :quiet :file %s\n' "$SCR/in.ini" "$SCR/send.ini" > "$SCR/bp.ini"
cp "$SCR/bp.ini" "$OUT/bp.ini"
{ echo "probe MNPROBE.PRG $(shasum -a 256 "$SCR/hd/MNPROBE.PRG" | cut -d' ' -f1) source $(shasum -a 256 tosdump/probes/mnprobe.c | cut -d' ' -f1)"
  echo "oracle $ORACLE_HATARI ROM $ORACLE_ROM_SHA256 profile tt E1 $E1"; } > "$OUT/run.txt"
HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" \
    --machine tt --tos-res ttmed --monitor vga --tos "$ORACLE_ROM" --sound off --fast-forward off \
    --conout 2 --run-vbls 60000 -d "$SCR/hd" --gemdos-drive C --auto 'C:\MNPROBE.PRG' \
    --parse "$SCR/bp.ini" > "$OUT/hatari.log" 2>&1 &
pid=$!; state=timeout
for _ in $(seq 1 3000); do
    if [ -s "$SCR/hd/MNPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/MNPROBE.LOG"; then sleep 1; state=done; break; fi
    kill -0 "$pid" 2>/dev/null || { state=exited; break; }
    sleep 0.1
done
kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "hatari $state" >> "$OUT/run.txt"
tr -d '\r' < "$SCR/hd/MNPROBE.LOG" > "$OUT/MNPROBE.LOG"
