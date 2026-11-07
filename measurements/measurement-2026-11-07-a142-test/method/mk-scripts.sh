#!/bin/sh
# A142-test: injector scripts (generated with the developer's gen-mouse.py, copied to this dir). usage: mk-scripts.sh <outdir>
O=$1; G="$(dirname "$0")/gen-mouse.py"
# issue: Optionen>Globales, Abbruch, Optionen>Schrift (alert), Return, Datei>Neuer Text, Datei>Info
python3 -I $G $O/issue.txt corner move:470,8 wait:40 click:470,91 at:800 click:538,404 at:1200 move:470,8 wait:40 click:470,75 at:1800 keys:1c,9c at:2200 move:82,8 wait:40 click:100,27 at:2800 move:82,8 wait:40 click:100,219 at:3500
# toggle: Globales, click one check box, hover, Abbruch ... then Textinfo, click a radio button
python3 -I $G $O/toggle.txt corner move:470,8 wait:40 click:470,91 at:800 click:340,157 at:1000 move:340,157 at:1100 click:538,404 at:1500 move:82,404 wait:10 move:82,8 wait:40 click:100,27 at:2100 move:82,8 wait:40 click:100,219 at:2800 click:300,289 at:3100
# alt: four alerts, Return after each
python3 -I $G $O/alt.txt corner at:600 keys:1c,9c at:1000 keys:1c,9c at:1400 keys:1c,9c at:1800 keys:1c,9c
python3 -I $G $O/hv.txt corner at:500 keys:1d,2a,4d,cd,aa,9d at:700 keys:1d,2a,4d,cd,aa,9d at:900 keys:1d,2a,4d,cd,aa,9d at:1100 keys:1d,2a,4d,cd,aa,9d at:1300 keys:1d,2a,4d,cd,aa,9d at:1500 keys:1d,2a,4d,cd,aa,9d at:1700 keys:1d,2a,4d,cd,aa,9d at:1900 keys:1d,2a,4d,cd,aa,9d at:2100 keys:1d,2a,4d,cd,aa,9d at:2300 keys:1d,2a,4d,cd,aa,9d at:2500 keys:1d,2a,4d,cd,aa,9d at:2700 keys:1d,2a,4d,cd,aa,9d at:2900 keys:1d,2a,4d,cd,aa,9d
