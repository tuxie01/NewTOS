#!/usr/bin/env bash
# runs.sh <scratch> -- every run of measurement-2026-10-31-fs26 (TASK/FS26), in
# order, one QEMU/Hatari at a time; results into ../runs/.
#   1. kernels: rpi4 release with and without --features fs-io-trace (this tree)
#   2. oracle, twice: tosdump/probes/run-dcprobe.sh emutos
#   3. NewTOS layer (a) AUTO .NCP and (b) GEM .NAP, feature on (counts + us),
#      once each: run-dcprobe.sh newtos auto|gem
#   4. the same with the feature off (_hz_200 timing only), three times each
#   5. layer (c) the desktop: method/run-desktop.sh (scratch tree, injector,
#      feature on) + method/timeline.py
# Scratch: <scratch>, the session scratchpad (never the shared temp dir); the
# caller deletes it.
set -euo pipefail
SCR=$1
HERE=$(cd "$(dirname "$0")" && pwd)
RUN=$(cd "$HERE/.." && pwd)/runs
REPO=$(cd "$HERE/../../.." && pwd)
mkdir -p "$RUN" "$SCR/k" "$SCR/tmp"
cd "$REPO"
K=target/aarch64-unknown-none-softfloat/release/kernel8.img
cargo xtask build --board rpi4 --release --features fs-io-trace > "$SCR/k/build-on.log" 2>&1
cp "$K" "$SCR/k/kernel-on.img"
cargo xtask build --board rpi4 --release > "$SCR/k/build-off.log" 2>&1
cp "$K" "$SCR/k/kernel-off.img"
{ grep -E "generated [0-9]+ warnings" "$SCR/k/build-on.log" | sed 's/^/on : /'
  grep -E "generated [0-9]+ warnings" "$SCR/k/build-off.log" | sed 's/^/off: /'
  (cd "$SCR/k" && shasum -a 256 kernel-on.img kernel-off.img); } > "$RUN/kernels.txt"
export DCPROBE_SCRATCH=$SCR/tmp
for i in 1 2; do tosdump/probes/run-dcprobe.sh emutos "$RUN/emutos-$i" > /dev/null; done
tosdump/probes/run-dcprobe.sh newtos auto "$RUN/newtos-auto-on" "$SCR/k/kernel-on.img" > /dev/null
tosdump/probes/run-dcprobe.sh newtos gem "$RUN/newtos-gem-on" "$SCR/k/kernel-on.img" > /dev/null
for i in 1 2 3; do
    tosdump/probes/run-dcprobe.sh newtos auto "$RUN/newtos-auto-off-$i" "$SCR/k/kernel-off.img" > /dev/null
    tosdump/probes/run-dcprobe.sh newtos gem "$RUN/newtos-gem-off-$i" "$SCR/k/kernel-off.img" > /dev/null
done
rm -rf "$SCR/tree"; mkdir -p "$SCR/tree"
git ls-files -co --exclude-standard -z | tr '\0' '\n' | grep -v '^measurements/' | tr '\n' '\0' |
    rsync -a --from0 --files-from=- ./ "$SCR/tree/"
"$HERE/run-desktop.sh" "$SCR" "$RUN/desktop-on" > /dev/null
python3 "$HERE/timeline.py" "$RUN/desktop-on/BOOT-SERIAL.txt" > "$RUN/desktop-on/timeline.txt"
echo "runs.sh done"
