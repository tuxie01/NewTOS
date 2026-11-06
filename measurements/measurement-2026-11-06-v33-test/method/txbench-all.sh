#!/bin/bash
cd /Volumes/Daten/Development/PiRusTOS
M=measurements/measurement-2026-11-06-v33-test; S=/private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/7fba4f48-e8f1-4a5e-8c58-696d17828400/scratchpad/v33t; K=target/aarch64-unknown-none-softfloat/release/kernel8.img
for r in "head 4" "base 4" "head 5" "base 5" "head 6" "base 6"; do set -- $r; bash $M/method/txbench-run.sh $M/runs/txbench-$1-$2 $S/$1/$K 2>&1 | grep -E "^[A-E]_|qemu done|no LOG" | tr '\n' ' ' | sed "s/^/$1-$2: /"; echo; done
