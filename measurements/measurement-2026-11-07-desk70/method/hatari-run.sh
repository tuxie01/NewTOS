#!/bin/bash
# hatari-run.sh <image> <out-dir> <action-file> [extra hatari args...]
# DESK70: pinned EmuTOS on the pinned Hatari, profile "disk" machine (Falcon030, VGA, floppies off,
# IDE master = per-run APFS clone of <image>), NO GEMDOS drive and NO --auto: EmuTOS boots from its own IDE C:
# with C:\EMUDESK.INF, real time (--fast-forward off), a real-time command fifo and an AVI of every frame.
# tools/hatari-oracle.sh cannot be used: it requires --auto and a dumper, and both replace the desktop path measured here.
# Same pin: ROM sha checked, --configfile tools/oracle/hatari.cfg, empty HOME, SDL dummy.
# Actions, one per line:  sleep <s> | send <hatari command> | shot <name> | key <ST scancode dec> | alt <scan> <n> (n Alt+arrow presses)
set -u
IMG=$1; OUT=$2; ACT=$3; shift 3
ROOT=/Volumes/Daten/Development/PiRusTOS
. $ROOT/tools/oracle/oracle.env
[ "$(shasum -a 256 "$ORACLE_ROM" | cut -d' ' -f1)" = "$ORACLE_ROM_SHA256" ] || { echo ROM mismatch; exit 1; }
[ ! -e "$OUT" ] || { echo "out exists: $OUT"; exit 1; }
mkdir -p "$OUT/home" "$OUT/shots"; OUT=$(cd "$OUT" && pwd)
cp -c "$IMG" "$OUT/disk.img"
echo "before $(shasum -a 256 "$OUT/disk.img" | cut -d' ' -f1)" > "$OUT/disk.sha256"
FIFO=$OUT/fifo
HOME="$OUT/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" --configfile "$ORACLE_HATARI_CFG" \
  --machine falcon --monitor vga --drive-a off --drive-b off --tos "$ORACLE_ROM" \
  --sound off --fast-forward off --conout 2 --ide-master "$OUT/disk.img" --ide-swap 0=off \
  --screenshot-dir "$OUT/shots" --screenshot-format png --cmd-fifo "$FIFO" "$@" > "$OUT/hatari.log" 2>&1 &
PID=$!
T0=$(python3 -I -c 'import time;print(time.time())')
now() { python3 -I -c "import time;print('%.2f'%(time.time()-$T0))"; }
for _ in $(seq 1 50); do [ -p "$FIFO" ] && break; sleep 0.1; done
exec 3<>"$FIFO"
send() { echo "$(now) send $*" >> "$OUT/actions.log"; echo "$*" >&3; }
while read -r op a b; do
  case "$op" in
    ''|'#'*) ;;
    sleep) sleep "$a" ;;
    send) send "$a $b" ;;
    shot) sleep 0.05; send "hatari-shortcut screenshot"; sleep 0.25
          f=$(ls -t "$OUT/shots" | head -1); [ -n "$f" ] && mv "$OUT/shots/$f" "$OUT/shots-$a.png"; echo "$(now) shot $a <- $f" >> "$OUT/actions.log" ;;
    key) send "hatari-event keypress $a"; sleep 0.4 ;;
    alt) send "hatari-event keydown 56"; sleep 0.2; i=0; while [ $i -lt "$b" ]; do send "hatari-event keypress $a"; sleep 0.12; i=$((i+1)); done; send "hatari-event keyup 56"; sleep 0.2 ;;
    dclick) send "hatari-event doubleclick"; sleep 0.1 ;;
    *) echo "bad action $op"; ;;
  esac
done < "$ACT"
kill -9 $PID 2>/dev/null; wait $PID 2>/dev/null
echo "after  $(shasum -a 256 "$OUT/disk.img" | cut -d' ' -f1)" >> "$OUT/disk.sha256"
rm -rf "$OUT/home" "$FIFO"; ls "$OUT"
