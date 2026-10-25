#!/bin/sh
# hatari-newfolder.sh <hd-dir> <shots-dir> <tag> -- pinned EmuTOS (etos512us.img) on Hatari Falcon VGA truecolor:
# Alt+C opens C:, Ctrl+N opens "New folder", then type A, B, Left, Left, Right and screenshot after each step.
S=$(dirname "$0"); HD=$1; SHOTS=$2; TAG=$3
H=/Volumes/Daten/hatari-2.6.1_macos/Hatari.app/Contents/MacOS/Hatari
TOS=/Volumes/Daten/Development/PiRusTOS/references/emutos/etos512us.img
FIFO=$S/fifo-$TAG; rm -f "$FIFO"; mkdir -p "$SHOTS"
send() { ( echo "$1" > "$FIFO" ) & w=$!; sleep 1.0; kill $w 2>/dev/null; }
key() { send "hatari-event keypress $1"; sleep 0.8; }
SDL_VIDEODRIVER=dummy "$H" --machine falcon --tos "$TOS" --monitor vga --sound off --fast-forward on \
  --conout 2 -d "$HD" --gemdos-drive C --screenshot-dir "$SHOTS" --screenshot-format png \
  --cmd-fifo "$FIFO" > "$S/hatari-$TAG.log" 2>&1 &
PID=$!
sleep 8
send "hatari-event keydown 56"; sleep 0.3; send "hatari-event keypress 46"; sleep 0.3; send "hatari-event keyup 56"; sleep 3
send "hatari-event keydown 29"; sleep 0.3; send "hatari-event keypress 49"; sleep 0.3; send "hatari-event keyup 29"; sleep 3
send "hatari-shortcut screenshot"; sleep 1.5     # 1: dialog up, empty field, caret at col 0
key 30; send "hatari-shortcut screenshot"; sleep 1.5   # 2: "A"
key 48; send "hatari-shortcut screenshot"; sleep 1.5   # 3: "AB"
key 75; send "hatari-shortcut screenshot"; sleep 1.5   # 4: Left
key 75; send "hatari-shortcut screenshot"; sleep 1.5   # 5: Left
key 77; send "hatari-shortcut screenshot"; sleep 1.5   # 6: Right
key 14; send "hatari-shortcut screenshot"; sleep 1.5   # 7: Backspace
kill $PID 2>/dev/null; wait $PID 2>/dev/null
ls "$SHOTS"
