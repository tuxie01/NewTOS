#!/usr/bin/env bash
# The issue's sequence as an IKBD script: open HypView from the Desk menu, pick TEST.HYP in the file selector, OK,
# close the window with its closer, open HypView from the Desk menu again. $1 = output file.
HERE=$(cd "$(dirname "$0")" && pwd)
python3 -I "$HERE/gen-mouse.py" "$1" corner at:600 move:30,300 move:30,10 wait:100 move:30,59 wait:60 click:30,59 \
    at:1300 click:260,222 at:1500 click:220,400 \
    at:2300 click:10,29 \
    at:2800 move:30,10 wait:100 move:30,59 wait:60 click:30,59
