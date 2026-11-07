#!/bin/sh
# V53: the injector-armed probe kernels (tools/inject-arm.py launch:WKPROBE=tosdump/probes/wkprobe-mouse.txt)
# of the base and the fix tree, built by tosdump/probes/run-wkprobe.sh kernel.
S=${S:?}
cd /Volumes/Daten/Development/PiRusTOS
export WKPROBE_SCRATCH=$S/wk
for T in base fix; do tosdump/probes/run-wkprobe.sh kernel $S/t/$T $S/kinj-$T.img; done
