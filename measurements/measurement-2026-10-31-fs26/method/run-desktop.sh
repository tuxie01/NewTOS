#!/usr/bin/env bash
# run-desktop.sh <scratch> <out-dir> -- TASK/FS26 layer (c): the desktop's
# File > New folder end to end, twice, in QEMU raspi4b with the fs-io-trace
# kernel (kernel/src/dbg_fsio.rs) and the scripted IKBD injector.
#   <scratch>/tree  a copy of the working tree (git ls-files -co
#                   --exclude-standard); tools/inject-arm.py instruments it
#                   there (never the shared tree), the kernel is built there.
#   card            tools/mkcard.sh, MKCARD_SIZE_MIB=512, the C: files of
#                   tosdump/probes/run-dcprobe.sh (ROOTF01..24.BIN, BALLAST.BIN)
#                   and NEWDESK-W.INF (a window on C:\*.* at start).
# <out-dir> gets BOOT-SERIAL.txt, card.img.manifest, after.png, fsck-after.txt,
# run.txt. One QEMU, killed at the end.
set -euo pipefail
SCR=$1; OUT=$2
HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
T=$SCR/tree
[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

grep -q INJ-ARM-TEMP "$T/kernel/src/host_env.rs" ||
    python3 "$T/tools/inject-arm.py" "$T" desktop="$HERE/newfolder.script" > "$OUT/inject-arm.txt"
(cd "$T" && cargo xtask build --board rpi4 --release --features fs-io-trace) > "$SCR/build.txt" 2>&1 ||
    { tail -30 "$SCR/build.txt" >&2; exit 1; }
K=$T/target/aarch64-unknown-none-softfloat/release/kernel8.img
cp "$K" "$SCR/kernel-desk.img"

mkdir -p "$SCR/files"
python3 - "$SCR/files" <<'PY'
import sys
d = sys.argv[1]
for i in range(1, 25):
    open(f"{d}/ROOTF{i:02d}.BIN", "wb").write(bytes((j * 7 + i) & 255 for j in range(1024)))
blk = bytes(range(256)) * 4096
with open(f"{d}/BALLAST.BIN", "wb") as f:
    for _ in range(16):
        f.write(blk)
PY
PAIRS=()
for i in $(seq -w 1 24); do PAIRS+=("ROOTF$i.BIN=$SCR/files/ROOTF$i.BIN"); done
PAIRS+=("BALLAST.BIN=$SCR/files/BALLAST.BIN")
(cd "$REPO" && MKCARD_SIZE_MIB=512 tools/mkcard.sh "$SCR/card-w.img" "$HERE/NEWDESK-W.INF" "${PAIRS[@]}") > "$SCR/mkcard.txt" 2>&1 ||
    { cat "$SCR/mkcard.txt" >&2; exit 1; }
cp "$SCR/card-w.img.manifest" "$OUT/card.img.manifest"
cc -o "$SCR/qmpshot" "$REPO/tools/qmpshot.c"
cp -c "$SCR/card-w.img" "$SCR/boot-w.img"
{
    echo "desktop New folder run ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      scratch copy of $(cd "$REPO" && git rev-parse HEAD) + working-tree changes, injector armed on 'desktop' (newfolder.script)"
    echo "kernel    $(sha "$K") (rpi4 release, --features fs-io-trace, injector)"
    echo "card      $(sha "$SCR/card-w.img") (tools/mkcard.sh MKCARD_SIZE_MIB=512, NEWDESK-W.INF)"
} > "$OUT/run.txt"
cd "$SCR"
SOCK=q.sock; rm -f "$SOCK"
: > "$OUT/BOOT-SERIAL.txt"
qemu-system-aarch64 -M raspi4b -no-reboot -display none -append serial=on -kernel "$SCR/kernel-desk.img" \
    -drive file="$SCR/boot-w.img",if=sd,format=raw -serial file:"$OUT/BOOT-SERIAL.txt" \
    -qmp unix:"$SOCK",server=on,wait=off &
pid=$!
state=timeout
for _ in $(seq 1 120); do
    sleep 1
    if grep -q "feed 0x9c" "$OUT/BOOT-SERIAL.txt" && [ "$(grep -c 'feed 0x9c' "$OUT/BOOT-SERIAL.txt")" -ge 2 ]; then
        state=done; sleep 8; break
    fi
    kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
done
"$SCR/qmpshot" "$SOCK" "$SCR/after.ppm" > /dev/null 2>&1 && sips -s format png "$SCR/after.ppm" --out "$OUT/after.png" > /dev/null 2>&1 || true
kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
echo "qemu      $state; stopped" >> "$OUT/run.txt"
cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot-w.img" | tr -d ' ')
plen=$(od -An -t u4 -j 458 -N 4 "$SCR/boot-w.img" | tr -d ' ')
dd if="$SCR/boot-w.img" of="$SCR/part-w.img" bs=512 skip="$cstart" count="$plen" 2>/dev/null
if fsck_msdos -n "$SCR/part-w.img" > "$OUT/fsck-after.txt" 2>&1; then rc=0; else rc=$?; fi
echo "fsck      C: after the run: fsck_msdos -n exit $rc" >> "$OUT/run.txt"
MTOOLS_SKIP_CHECK=1 mdir -i "$SCR/boot-w.img@@$((cstart * 512))" ::/ > "$OUT/c-root-after.txt" 2>&1 || true
rm -f "$SCR/boot-w.img" "$SCR/part-w.img" "$SCR/after.ppm" "$SOCK"
cat "$OUT/run.txt"
