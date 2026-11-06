#!/bin/bash
# armed kernels, shprobe/slprobe (gui-runs), desktop+slprobe repeat+geometries+boot console (rest-runs)
cd /Volumes/Daten/Development/PiRusTOS
M=measurements/measurement-2026-11-06-v33-test
sh $M/method/armed-kernels.sh
sh $M/method/gui-runs.sh
bash $M/method/rest-runs.sh
echo ALLFINISHED
