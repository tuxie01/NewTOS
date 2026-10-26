#!/usr/bin/env bash
# all.sh -- A118 rework: the whole run in order (one QEMU at a time; scratch $S = scratchpad/a118/fix):
#   lint-both.sh (builds dev+release both boards, clippy, host test in both trees) -> xtest-both.sh (cargo xtask test, correct links)
#   -> mkcards.sh, api/api-build.sh + api-card.sh, a118/run-a118.sh card -> build-all.sh, a118/run-a118.sh build, a117/tdq.sh build
#   -> qemu-all.sh (GH1 reg nd/td/qed on, std, A117, TeraDesk quit, a118 probe, APITEST on/off/strict).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; . "$HERE/reg/env.sh"; L="$S/logs"; mkdir -p "$L"
bash "$HERE/lint-both.sh" > "$L/lint-both.out" 2>&1
bash "$HERE/xtest-both.sh" > "$L/xtest-both.out" 2>&1
bash "$HERE/mkcards.sh" > "$L/mkcards.out" 2>&1
grep -E "^[0-9a-f]{64}" "$L/mkcards.out" > "$HERE/../checks/cards-and-programs.sha256"
bash "$HERE/api/api-build.sh" "$S/api" > "$L/api-build.out" 2>&1
bash "$HERE/api/api-card.sh" "$S/api/api.img" api "$S/api/APITEST.NCP" "$S/api/PTERMCH.NCP" "$S/api/DTACHILD.NCP" > "$L/api-card.out" 2>&1
bash "$HERE/a118/run-a118.sh" card >> "$HERE/../checks/cards-and-programs.sha256" 2>/dev/null
bash "$HERE/build-all.sh" > "$L/build-all.out" 2>&1
bash "$HERE/a118/run-a118.sh" build > "$L/build-a118.out" 2>&1
bash "$HERE/a117/tdq.sh" build > "$L/build-tdq.out" 2>&1
bash "$HERE/qemu-all.sh" > "$L/qemu-all.out" 2>&1
echo ALL DONE
