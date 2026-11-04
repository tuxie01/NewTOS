#!/usr/bin/env bash
# run-a133.sh <scratch> -- A133 evidence. Every QEMU/Hatari/compare step calls the
# A129 scripts (measurements/measurement-2026-11-04-a129-measure/method/) or the
# A132 crop/residual scripts, unchanged; this directory adds only the TEDINFO
# probe (tedprobe.c + build-tedprobe.sh) and an exact pixel diff (pixdiff.py).
#   0. kernels: A133 = cargo xtask build --board rpi4 --release in the working
#      tree; base = the same build of `git archive HEAD` (the tree without the
#      A133 change) in <scratch>/base-src (its own target/).
#   1. file box: A129 FSPROBE on the A133 kernel vs the oracle (A129 cmp.py at
#      the A129 origins, A132 residual.py, A129 dump.py, A132 crop.py).
#   2. TEDINFO dialog: tedprobe on the A133 kernel, on the base kernel and on
#      the oracle (falcontc); tree at (16,48) on every screen; cmp.py over the
#      root rect +/-4.
#   3. regression: desktop boot (newdesk.inf below, no autorun), base vs A133,
#      screendumps at 30 s and 45 s, pixdiff.py.
set -euo pipefail
SCR=$1; KHEAD=$2; KBASE=$3
HERE=$(cd "$(dirname "$0")" && pwd); RUN=$(cd "$HERE/.." && pwd); ROOT=$(cd "$RUN/../.." && pwd)
A129=$ROOT/measurements/measurement-2026-11-04-a129-measure/method
A132=$ROOT/measurements/measurement-2026-11-04-a132/method
PY=$ROOT/.venv/bin/python3
mkdir -p "$SCR"/{fs,ted,boot,hat,base-src} "$RUN/analysis" "$RUN/hatari"

# 0. kernels: TESTER VARIANT -- prebuilt fresh release builds (head-src = git archive fd6c7e3f, base-src = git archive 3b1fd066)
cp "$KHEAD" "$SCR/kernel8-a133.img"; cp "$KBASE" "$SCR/kernel8-base.img"

# 1. file box (A129 probe)
"$A129/build-fsprobe.sh" "$SCR/fs/FSPROBE.NAP"
"$A129/mkcard-call.sh" "$SCR/fs"
cp "$SCR/fs/card.img.manifest" "$RUN/card-fsprobe.img.manifest"
"$A129/run.sh" "$SCR/kernel8-a133.img" "$SCR/fs/card.img" a133-fs "$RUN" "$SCR" 1920x1080 55
"$A129/build-fsprobe-m68k.sh" "$SCR/FSPROBE.PRG"
"$A129/run-hatari-fsel.sh" "$SCR/FSPROBE.PRG" "$RUN/hatari/fsel" "$SCR/hat" "$SCR/fs/list" 20
N=$RUN/screendumps/a133-fs-55.png; H=$RUN/hatari/fsel/shots/grab0001.png
{ echo "A133 vs EmuTOS, FILEAREA 181x199 at the A129 origins (NewTOS 823,476 / Hatari 183,176):"
  "$PY" -I "$A129/cmp.py" "$N" 823 476 "$H" 183 176 181 199
  echo; "$PY" -I "$A132/residual.py" "$N" 823 476 "$H" 183 176 181 199
} > "$RUN/analysis/fsel-compare.txt"
{ echo "NewTOS FTITLE (A133):"; "$PY" -I "$A129/dump.py" "$N" 823 476 1003 497
  echo; echo "EmuTOS FTITLE (Hatari falcontc):"; "$PY" -I "$A129/dump.py" "$H" 183 176 363 197
} > "$RUN/analysis/fsel-ftitle-dumps.txt"
"$PY" -I "$A132/crop.py" "$N" 823 476 "$H" 183 176 181 22 8 "$RUN/analysis/fsel-ftitle-newtos-top-hatari-bottom-x8.png"
"$PY" -I "$A132/crop.py" "$N" 823 476 "$H" 183 176 181 199 3 "$RUN/analysis/fsel-filearea-newtos-top-hatari-bottom-x3.png"

# 2. TEDINFO dialog (this run's probe; on the card under the A129 name FSPROBE.NAP)
"$HERE/build-tedprobe.sh" "$SCR/ted/FSPROBE.NAP" "$SCR/TEDPROBE.PRG"
"$A129/mkcard-call.sh" "$SCR/ted"
cp "$SCR/ted/card.img.manifest" "$RUN/card-tedprobe.img.manifest"
"$A129/run.sh" "$SCR/kernel8-a133.img" "$SCR/ted/card.img" a133-ted "$RUN" "$SCR" 1920x1080 55
"$A129/run.sh" "$SCR/kernel8-base.img" "$SCR/ted/card.img" base-ted "$RUN" "$SCR" 1920x1080 55
"$A129/run-hatari-fsel.sh" "$SCR/TEDPROBE.PRG" "$RUN/hatari/ted" "$SCR/hat" "$SCR/ted/list" 20
H=$RUN/hatari/ted/shots/grab0001.png
for k in a133 base; do
  N=$RUN/screendumps/$k-ted-55.png
  { echo "$k vs EmuTOS, tedprobe ROOT rect +/-4 (448x184 at 12,44 on both screens):"
    "$PY" -I "$A129/cmp.py" "$N" 12 44 "$H" 12 44 448 184
  } > "$RUN/analysis/ted-compare-$k.txt"
  "$PY" -I "$A132/crop.py" "$N" 12 44 "$H" 12 44 448 184 2 "$RUN/analysis/ted-$k-top-hatari-bottom-x2.png"
done
{ echo "NewTOS (A133) tedprobe:"; "$PY" -I "$A129/dump.py" "$RUN/screendumps/a133-ted-55.png" 12 44 459 227
  echo; echo "EmuTOS tedprobe:"; "$PY" -I "$A129/dump.py" "$H" 12 44 459 227
} > "$RUN/analysis/ted-dumps.txt"

# 3. regression: desktop boot, base vs A133
printf '#R 02\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#W 00 00 02 06 26 0C 00 C:\\*.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' > "$SCR/boot/newdesk.inf"
echo x > "$SCR/boot/readme.txt"
"$ROOT/tools/mkcard.sh" "$SCR/boot/card.img" "$SCR/boot/newdesk.inf" README.TXT="$SCR/boot/readme.txt"
cp "$SCR/boot/card.img.manifest" "$RUN/card-boot.img.manifest"
"$A129/run.sh" "$SCR/kernel8-base.img" "$SCR/boot/card.img" base-boot "$RUN" "$SCR" 1920x1080 30 45
"$A129/run.sh" "$SCR/kernel8-a133.img" "$SCR/boot/card.img" a133-boot "$RUN" "$SCR" 1920x1080 30 45
{ for s in 30 45; do echo "desktop boot at $s s, base vs A133:"
    "$PY" -I "$HERE/pixdiff.py" "$RUN/screendumps/base-boot-$s.png" "$RUN/screendumps/a133-boot-$s.png"; done
  echo "tedprobe, base vs A133:"
  "$PY" -I "$HERE/pixdiff.py" "$RUN/screendumps/base-ted-55.png" "$RUN/screendumps/a133-ted-55.png"
} > "$RUN/analysis/regression-diff.txt"

shasum -a 256 "$SCR/kernel8-a133.img" "$SCR/kernel8-base.img" "$SCR/fs/FSPROBE.NAP" "$SCR/FSPROBE.PRG" \
  "$SCR/ted/FSPROBE.NAP" "$SCR/TEDPROBE.PRG" "$SCR/fs/card.img" "$SCR/ted/card.img" "$SCR/boot/card.img" \
  "$ROOT/tos-api/src/aes/object.rs" > "$RUN/serial/sha256-a133rerun.txt"
(cd "$ROOT" && git rev-parse HEAD) >> "$RUN/serial/sha256-a133rerun.txt"
