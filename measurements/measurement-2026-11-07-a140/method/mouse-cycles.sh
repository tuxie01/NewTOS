#!/usr/bin/env bash
# The acceptance script (TASK/A140 "mehrfach"): open from the Desk menu, pick TEST.HYP, follow a link so that the
# remembered node is not 0, close; reopen three more times (the window comes back from last_path/last_node); then, with
# the window open, a further accessory click (file selector; HypView loads into the same window, va_start_newwin 0),
# close, reopen; close with Ctrl+Q (AES-side path RemoveItems, no WM_CLOSED), reopen. $1 = output file.
HERE=$(cd "$(dirname "$0")" && pwd)
D="move:30,10 wait:100 move:30,59 wait:60 click:30,59"
python3 -I "$HERE/gen-mouse.py" "$1" corner at:600 $D \
    at:1300 click:260,222 at:1500 click:220,400 \
    at:2300 click:60,110 \
    at:2800 click:10,29 at:3300 $D \
    at:4100 click:10,29 at:4600 $D \
    at:5400 click:10,29 at:5900 $D \
    at:6700 $D \
    at:7500 click:260,222 at:7700 click:220,400 \
    at:8500 click:10,29 at:9000 $D \
    at:9800 keys:1d,10,90,9d \
    at:10300 $D
