#!/usr/bin/env bash
# The A126 runs, as run (2026-10-30), from the repository root. SCR is the
# session scratchpad (never /tmp itself, never the repository). Every run
# directory below was copied into runs/ afterwards.
set -euo pipefail
SCR=${1:?usage: akprobe-runs.sh <scratch>}
export AKPROBE_SCRATCH=$SCR/tmp KSPROBE_SCRATCH=$SCR/tmp

# 1 the oracle, both profiles
tosdump/probes/run-akprobe.sh emutos tt       "$SCR/runs/emutos-tt"
tosdump/probes/run-akprobe.sh emutos falcontc "$SCR/runs/emutos-falcontc"

# 2 NewTOS after: the A126 tree, injector armed on AKPROBE, input trace
tosdump/probes/run-akprobe.sh kernel . "$SCR/kernel8-a126-bat.img" bios-arg-trace
tosdump/probes/run-akprobe.sh newtos "$SCR/runs/newtos-raspi4b" "$SCR/kernel8-a126-bat.img"

# 3 NewTOS before: the same tree with the four AES files of HEAD (0a07db38),
#   i.e. without the A126 change; armed and built as run-akprobe.sh kernel does
T=$SCR/pretree
mkdir -p "$T"
(git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
ln -s "$PWD/references" "$T/references"
for f in tos-api/src/aes/event.rs tos-api/src/aes/graf.rs tos-api/src/aes/gsxif.rs tos-api/src/aes/menu.rs; do
    git show "HEAD:$f" > "$T/$f"
done
python3 -I tools/inject-arm.py "$T" "launch:AKPROBE=$PWD/tosdump/probes/akprobe-keys.txt"
(cd "$T" && cargo xtask build --board rpi4 --release --features bios-arg-trace)
cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$SCR/kernel8-pre-bat.img"
rm -rf "$T"
tosdump/probes/run-akprobe.sh newtos "$SCR/runs/newtos-raspi4b-before" "$SCR/kernel8-pre-bat.img"

# 4 no new divergence in the Kbshift probe (TASK/BIOS10): the A126 tree
tosdump/probes/run-ksprobe.sh kernel . "$SCR/kernel8-ks-a126.img"
tosdump/probes/run-ksprobe.sh newtos "$SCR/runs/ksprobe-newtos-a126" "$SCR/kernel8-ks-a126.img"
M=measurements/measurement-2026-10-29-bios10/runs
diff <(grep -v '^I00' $M/ksprobe-NEWTOS-after/KSPROBE.LOG) <(grep -v '^I00' "$SCR/runs/ksprobe-newtos-a126/KSPROBE.LOG")

# 5 the oracle canary (docs/analysis/ORACLE.md section 6)
sh measurements/measurement-2026-10-28-oracle-canary/method/canary.sh "$SCR/canary-$(date +%Y%m%d-%H%M%S)"

# compare: every line but the side-specific I-lines
for s in emutos-falcontc newtos-raspi4b newtos-raspi4b-before; do
    echo "== emutos-tt vs $s"
    diff <(grep -v '^I' "$SCR/runs/emutos-tt/AKPROBE.LOG") <(grep -v '^I' "$SCR/runs/$s/AKPROBE.LOG") || true
done
