#!/usr/bin/env bash
# program mode (#Z autorun, HYP_VIEW.NAP as the shell child): pick TEST.HYP, OK, follow a link, click the closer. $1 = output file.
python3 -I /Volumes/Daten/Development/PiRusTOS/measurements/measurement-2026-11-07-a140/method/gen-mouse.py "$1" corner at:300 click:260,222 at:500 click:220,400 at:1000 click:60,110 at:1500 click:10,29
