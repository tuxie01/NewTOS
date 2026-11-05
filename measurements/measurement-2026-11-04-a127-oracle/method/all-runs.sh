#!/usr/bin/env bash
# all-runs.sh <scratch> -- the final series of the A127 oracle measurement, as run, from the
# repository root (probe source frozen: run.txt "source" line). Kernels: see newtos-runs.sh.
set -uo pipefail
S=$1
M=measurements/measurement-2026-11-04-a127-oracle/method
export MNPROBE_SCRATCH=$S
o() { # name profile E1 [fifo]
    if [ -n "${4:-}" ]; then
        MNPROBE_E1=$3 MNPROBE_INJ=$PWD/$M/inj-empty.txt MNPROBE_FIFO=$PWD/$M/fifo-cross.txt \
            tosdump/probes/run-mnprobe.sh emutos "$2" "$S/runs/$1" > "$S/last-$1.out" 2>&1
    else
        MNPROBE_E1=$3 tosdump/probes/run-mnprobe.sh emutos "$2" "$S/runs/$1" > "$S/last-$1.out" 2>&1
    fi
    echo "$1 rc=$? $(grep -h '^H00' "$S/runs/$1/MNPROBE.LOG") $(grep -c '^R' "$S/runs/$1/MNPROBE.LOG") R $(grep -h '^hatari' "$S/runs/$1/run.txt")"
}
n() { # name kernel E1
    MNPROBE_E1=$3 tosdump/probes/run-mnprobe.sh newtos "$S/runs/$1" "$S/$2" > "$S/last-$1.out" 2>&1
    echo "$1 rc=$? $(grep -h '^H00' "$S/runs/$1/MNPROBE.LOG") $(grep -c '^R' "$S/runs/$1/MNPROBE.LOG") R $(grep -h '^qemu' "$S/runs/$1/run.txt")"
}
o emutos-tt-e1a-1 tt 1A
o emutos-tt-e1a-2 tt 1A
o emutos-tt-e1b-1 tt 1B
o emutos-tt-e1b-2 tt 1B
o emutos-tt-virtinf-1 tt ""
o emutos-falcontc-1 falcontc ""
o emutos-tt-fifo-e1a-1 tt 1A fifo
o emutos-tt-fifo-e1b-1 tt 1B fifo
n newtos-head-e1a-1 kernel-head.img 1A
n newtos-base-e1a-1 kernel-base.img 1A
n newtos-head-e1b-1 kernel-head.img 1B
n newtos-base-e1b-1 kernel-base.img 1B
n newtos-head-noe-1 kernel-head.img ""
n newtos-base-noe-1 kernel-base.img ""
n newtos-head-e1a-2 kernel-head.img 1A
n newtos-base-e1a-2 kernel-base.img 1A
n newtos-head-dbg-e1a kernel-head-dbg.img 1A
n newtos-base-dbg-e1a kernel-base-dbg.img 1A
echo ALL-DONE
