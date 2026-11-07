#!/usr/bin/env bash
# tester drive: uses the developer's method/run.sh unchanged. env S = scratchpad.
S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad; R=/Volumes/Daten/Development/PiRusTOS; T=$R/measurements/measurement-2026-11-07-a140-test; M=$R/measurements/measurement-2026-11-07-a140/method
export A140_SCRATCH=$S/img A140_HYP=$R/Applications/cflib-git/bin/cflib.hyp
mkdir -p $A140_SCRATCH
run() { # name kernel which shots [append]
  A140_APPEND="${5:-serial=on}" "$M/run.sh" "$T/runs/$1" "$S/k/$2" "$S/$3/sdk/apps/hypview/build/HYP_VIEW.NAP" "$S/$3/sdk/apps/hypview/build/hyp_view.rsc" "$M/$4" > /dev/null 2>&1
  echo "$1: $(tail -2 $T/runs/$1/run.txt | tr '\n' ' ')"
}
for i in 1 2 3; do run base-issue-$i kernel-issue.img base shots-issue.txt; done
for i in 1 2 3; do run fix-issue-$i kernel-issue.img fix shots-issue.txt; done
run base-cycles kernel-cycles.img base shots-cycles.txt
run fix-cycles-1 kernel-cycles.img fix shots-cycles.txt
run fix-cycles-2 kernel-cycles.img fix shots-cycles.txt
run fix-cycles-strict kernel-cycles.img fix shots-cycles.txt "serial=on memprot=strict"
