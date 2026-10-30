#!/bin/bash
# run-before.sh -- vwprobe, sxprobe, hkprobe on NewTOS with the "before" kernel (HEAD f2652440,
# built in the scratch before-tree by run-all.sh step 1), run after run-all.sh, as run.
set -u
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/45e8e806-7ba7-4edc-ace3-d4dd625a7f8d/scratchpad/bios17
REPO=/Volumes/Daten/Development/PiRusTOS
R=$REPO/measurements/measurement-2026-10-30-bios17/runs
cd $REPO
VWPROBE_SCRATCH=$S/vw tosdump/probes/run-vwprobe.sh newtos $R/vwprobe-NEWTOS-before $S/kernel-before.img > $S/vw-before.out 2>&1; echo "vw before $?"
SXPROBE_SCRATCH=$S/sx tosdump/probes/run-sxprobe.sh newtos $R/sxprobe-NEWTOS-before $S/kernel-before.img > $S/sx-before.out 2>&1; echo "sx before $?"
HKPROBE_SCRATCH=$S/hk tosdump/probes/run-hkprobe.sh newtos $R/hkprobe-NEWTOS-before $S/kernel-before.img > $S/hk-before.out 2>&1; echo "hk before $?"
