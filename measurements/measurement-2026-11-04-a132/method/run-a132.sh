#!/usr/bin/env bash
# run-a132.sh <scratch> -- A132 evidence: the A129 file-box comparison
# (measurements/measurement-2026-11-04-a129-measure/method/) re-run against the
# working tree (HEAD + the A132 change). Every step calls an A129 script; only
# the kernel build, the paths and the crops are this run's.
#   1. kernel8.img: cargo xtask build --board rpi4 --release (working tree)
#   2. FSPROBE.NAP + card (A129 build-fsprobe.sh, mkcard-call.sh)
#   3. QEMU raspi4b 1920x1080, screendump at 55 s (A129 run.sh)
#   4. EmuTOS oracle: FSPROBE.PRG in Hatari falcontc (A129 build-fsprobe-m68k.sh,
#      run-hatari-fsel.sh)
#   5. compare FILEAREA 181x199 (A129 cmp.py, origins NewTOS 823,476 /
#      Hatari 183,176 as in A129), dump FTITLE (A129 dump.py), crops (crop.py)
set -euo pipefail
SCR=$1
HERE=$(cd "$(dirname "$0")" && pwd); RUN=$(cd "$HERE/.." && pwd); ROOT=$(cd "$RUN/../.." && pwd)
A129=$ROOT/measurements/measurement-2026-11-04-a129-measure/method
PY=$ROOT/.venv/bin/python3          # PIL for cmp.py / dump.py / crop.py
mkdir -p "$SCR/card" "$SCR/hat"

(cd "$ROOT" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1
cp "$ROOT/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$SCR/kernel8.img"

"$A129/build-fsprobe.sh" "$SCR/card/FSPROBE.NAP"
"$A129/mkcard-call.sh" "$SCR/card"
cp "$SCR/card/card.img.manifest" "$RUN/card.img.manifest"

"$A129/run.sh" "$SCR/kernel8.img" "$SCR/card/card.img" a132 "$RUN" "$SCR" 1920x1080 55

"$A129/build-fsprobe-m68k.sh" "$SCR/FSPROBE.PRG"
"$A129/run-hatari-fsel.sh" "$SCR/FSPROBE.PRG" "$RUN/hatari" "$SCR/hat" "$SCR/card/list" 20

N=$RUN/screendumps/a132-55.png; H=$RUN/hatari/shots/grab0001.png
{
  echo "A132 (HEAD + A132 change) vs EmuTOS, FILEAREA 181x199 anchored at FTITLE outline top-left (NewTOS 823,476 / Hatari 183,176):"
  "$PY" -I "$A129/cmp.py" "$N" 823 476 "$H" 183 176 181 199
} > "$RUN/analysis/compare.txt"
{
  echo "NewTOS FTITLE (A132):"; "$PY" -I "$A129/dump.py" "$N" 823 476 1003 497
  echo; echo "EmuTOS FTITLE (Hatari falcontc):"; "$PY" -I "$A129/dump.py" "$H" 183 176 363 197
} > "$RUN/analysis/ftitle-dumps.txt"
"$PY" -I "$HERE/crop.py" "$N" 823 476 "$H" 183 176 181 22 8 "$RUN/analysis/ftitle-newtos-top-hatari-bottom-x8.png"
"$PY" -I "$HERE/crop.py" "$N" 823 476 "$H" 183 176 181 199 3 "$RUN/analysis/filearea-newtos-top-hatari-bottom-x3.png"

shasum -a 256 "$SCR/kernel8.img" "$SCR/card/FSPROBE.NAP" "$SCR/FSPROBE.PRG" "$SCR/card/card.img" \
  "$ROOT/tos-api/src/aes/fsel.rs" "$N" "$H" > "$RUN/serial/sha256.txt"
