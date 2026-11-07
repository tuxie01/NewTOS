#!/bin/sh
# TASK/A142: the injector script of runs/before, runs/after, runs/probe-*: Optionen > Globales (check boxes), Abbruch, Optionen > Schrift (alert), Return, Datei > Neuer Text, Datei > Info (radio buttons).
python3 -I "$(dirname "$0")/gen-mouse.py" "$1" corner move:470,8 wait:40 click:470,91 at:800 click:538,404 at:1200 move:470,8 wait:40 click:470,75 at:1800 keys:1c,9c at:2200 move:82,8 wait:40 click:100,27 at:2800 move:82,8 wait:40 click:100,219 at:3500
