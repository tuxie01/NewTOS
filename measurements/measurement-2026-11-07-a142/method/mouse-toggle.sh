#!/bin/sh
# TASK/A142: the injector script of runs/toggle: a check box and a radio button clicked, so that they are drawn through objc_change.
python3 -I "$(dirname "$0")/gen-mouse.py" "$1" corner move:470,8 wait:40 click:470,91 at:800 click:340,157 at:1000 move:340,157 at:1100 click:538,404 at:1500 move:82,404 wait:10 move:82,8 wait:40 click:100,27 at:2100 move:82,8 wait:40 click:100,219 at:2800 click:300,289 at:3100
