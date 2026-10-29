#!/bin/sh
# oracle-serrx-runs.sh <SERRX.PRG> <scratch> -- the oracle runs of this measurement:
# tt and falcontc with 'aZ1' twice each, and once each with no byte injected
# (control), and once each with 'aZ1' and the Hatari BIOS/XBIOS trace (input check).
# Each run: method/oracle-serrx.sh, at most 100 s.
HERE=$(cd "$(dirname "$0")" && pwd)
PRG=$1; S=$2
run() { # <profile> <name> <bytes>
    export ORACLE_SERRX_TRACE
    "$HERE/oracle-serrx.sh" "$1" "$PRG" "$S/$2" "$3" > "$S/$2.out" 2>&1 &
    pid=$!; n=0
    while kill -0 $pid 2>/dev/null && [ $n -lt 100 ]; do sleep 1; n=$((n + 1)); done
    kill $pid 2>/dev/null
    echo "$2: done after ${n}s"
}
for spec in "tt emu-tt-1 aZ1" "falcontc emu-fc-1 aZ1" "tt emu-tt-2 aZ1" "falcontc emu-fc-2 aZ1" \
            "tt emu-tt-none none" "falcontc emu-fc-none none" \
            "tt emu-tt-trace aZ1 1" "falcontc emu-fc-trace aZ1 1"; do
    # shellcheck disable=SC2086
    set -- $spec
    [ -e "$S/$2/SERRX.LOG" ] && { echo "$2: exists, kept"; continue; }
    ORACLE_SERRX_TRACE=${4:-0} run "$1" "$2" "$3"
done
