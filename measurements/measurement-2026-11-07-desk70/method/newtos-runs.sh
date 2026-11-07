#!/bin/bash
# newtos-runs.sh <scratch> <evidence dir> -- NewTOS runs of DESK70: scratch injector kernels (one per click target; build-inj.sh), one QEMU at a time, APFS clone per run
S=$1; E=$2; M=$(cd "$(dirname "$0")" && pwd); set -eu
for t in m0ntp m1ncp m2ncp; do "$M/build-inj.sh" "$S" "$M/n-$t.actions" "$t" >> "$S/kernels.txt"; done
shasum -a 256 "$S"/kern/kernel8-m0ncp.img "$S"/kern/kernel8-plain.img >> "$S/kernels.txt"
one() { t=$1; k=$2; s=$3; python3 "$M/qemu-run.py" "$S/kern/kernel8-$k.img" "$S/card.img" "$S/nr/$t" "$s"; python3 "$M/classify.py" dir "$S/nr/$t" > "$S/nr/$t/classify.txt"; }
mkdir -p "$S/nr"
one n-m0ncp-r1 m0ncp 30
one n-m0ncp-r2 m0ncp 30
one n-m0ntp m0ntp 34
one n-m1ncp m1ncp 40
one n-m2ncp m2ncp 95
