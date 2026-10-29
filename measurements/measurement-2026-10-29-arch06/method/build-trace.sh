#!/usr/bin/env bash
# build-trace.sh <scratch> [script-name] -- the trace kernel of this run: a scratch copy of the working tree (mktree.sh), the IKBD
# injector armed on TeraDesk (tools/inject-arm.py, up:DESKTOP=<script-name>.script, default vid), built release rpi4 with
# --features svc-seq-trace,bios-arg-trace (C:\SVCSEQ.LOG + one serial line per BIOS/XBIOS call). -> <scratch>/bin/kernel8-trace[-<script-name>].img
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; WS="$(cd "$HERE/../../.." && pwd)"; S="$1"; SC="${2:-vid}"; SUF=""; [ "$SC" = vid ] || SUF="-$SC"; T="$S/trees/trace$SUF"
export SOURCE_DATE_EPOCH=1792368000
bash "$HERE/mktree.sh" "$T" >/dev/null
python3 "$WS/tools/inject-arm.py" "$T" --tag inj up:DESKTOP="$HERE/$SC.script" | tail -1
(cd "$T" && cargo xtask build --board rpi4 --release --features svc-seq-trace,bios-arg-trace) > "$S/logs/build-trace$SUF.log" 2>&1 \
  || { tail -30 "$S/logs/build-trace$SUF.log"; exit 1; }
cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$S/bin/kernel8-trace$SUF.img"; shasum -a 256 "$S/bin/kernel8-trace$SUF.img"
