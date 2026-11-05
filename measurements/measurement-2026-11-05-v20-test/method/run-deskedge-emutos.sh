#!/usr/bin/env bash
# V20 acceptance, EmuTOS oracle for the edge scenes: pinned EmuTOS (profile falcontc) in hermetic Hatari,
# desktop with EMUDESK.INF, DSFEED.PRG (tosdump/probes/dsfeed.c) feeds the mouse script and triggers the shots
# (Hatari tracking breakpoint on its counter, as run-slprobe.sh).
#   run-deskedge-emutos.sh <out-dir> <mouse.txt> <shots.txt> <INF>
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS
cd "$REPO"
OUT=$1; MOUSE=$2; SHOTS=$3; INF=$4
[ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
BASE=${DESK_SCRATCH:?}
mkdir -p "$BASE" "$OUT/shots"; OUT=$(cd "$OUT" && pwd)
SCR=$(mktemp -d "$BASE/dedge.XXXXXX"); trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
. tools/oracle/oracle.env
[ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
PRG=$SCR/DSFEED.PRG
(cd tosdump && "${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -Icore probes/dsfeed.c -o "$PRG")
mkdir -p "$SCR/hd" "$SCR/home"
cp "$PRG" "$SCR/hd/DSFEED.PRG"; cp "$MOUSE" "$SCR/hd/DSFEED.INJ"; cp "$SHOTS" "$SCR/hd/DSFEED.SHT"
# the Falcon profile's #E line (as run-shprobe.sh); the rest of the INF is the NewTOS INF
sed 's/^#E .*/#E 1B E0 00 1C 60\r/' "$INF" > "$SCR/hd/EMUDESK.INF"
cp "$SCR/hd/EMUDESK.INF" "$OUT/EMUDESK.INF"; cp "$SHOTS" "$OUT/shots.txt"; cp "$MOUSE" "$OUT/mouse.txt"
printf 'screenshot %s/shot.png\n' "$SCR" > "$SCR/shot.ini"
{ echo "deskedge emutos falcontc ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "DSFEED.PRG $(sha "$PRG")"; echo "mouse $(sha "$MOUSE") shots $(sha "$SHOTS") inf $(sha "$SCR/hd/EMUDESK.INF")"
  echo "oracle $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile falcontc"; } > "$OUT/run.txt"
names=()
while read -r tk name; do names+=("$name"); done < <(sed 's/#.*//' "$SHOTS" | grep .)
HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
    --configfile "$ORACLE_HATARI_CFG" --machine falcon --monitor vga --tos "$ORACLE_ROM" \
    --sound off --fast-forward off --conout 2 --run-vbls 200000 --statusbar off --drive-led off --crop on \
    -d "$SCR/hd" --gemdos-drive C --auto 'C:\DSFEED.PRG' --cmd-fifo "$SCR/cmd" \
    > "$OUT/hatari.log" 2>&1 &
pid=$!
armed=; n=0; state=timeout
for _ in $(seq 1 60000); do
    if [ -z "$armed" ] && [ -s "$SCR/hd/DSFEED.ADR" ]; then
        addr=$(tr -d '\r\n ' < "$SCR/hd/DSFEED.ADR")
        echo "hatari-debug b (\$$addr).l ! (\$$addr).l :trace :file $SCR/shot.ini" > "$SCR/cmd"
        echo "breakpoint (\$$addr).l ! (\$$addr).l :trace :file shot.ini" >> "$OUT/run.txt"
        sleep 1
        : > "$SCR/hd/DSFEED.GO"
        armed=1
    fi
    if [ -e "$SCR/shot.png" ] && [ "$(grep -c "Screen dump to '$SCR/shot.png' succeeded" "$OUT/hatari.log" || true)" -gt "$n" ]; then
        cp "$SCR/shot.png" "$OUT/shots/$(printf '%02d' "$n")-${names[$n]:-extra}.png"; rm -f "$SCR/shot.png"
        n=$((n + 1))
        [ "$n" -ge "${#names[@]}" ] && { sleep 1; state=done; break; }
    fi
    kill -0 "$pid" 2>/dev/null || { state=hatari-exited; break; }
    sleep 0.02
done
kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "hatari $state, $n shots of ${#names[@]}" >> "$OUT/run.txt"
cat "$OUT/run.txt"
