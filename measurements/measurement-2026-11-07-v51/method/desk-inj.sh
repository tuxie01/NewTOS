#!/usr/bin/env bash
# V51 (copy of the V50 run's helper, REPO = the V51 worktree): the desktop with the scripted IKBD injector. Copy of
# measurements/measurement-2026-11-07-v34-test/method/desk-inj.sh with REPO = the V51 worktree.
#   desk-inj.sh kernel <commit> <mouse.txt> <kernel-out>
#   desk-inj.sh newtos <out-dir> <kernel8.img> <NEWDESK.INF> <shots.txt>
# kernel: git archive of <commit> into scratch, armed with tools/inject-arm.py desktop=<mouse.txt>, release rpi4.
# newtos: tools/mkcard.sh card (INF + DESK_FILES), QEMU raspi4b (serial=on) on an APFS clone, QMP screendumps
#         from shots.txt (<ticks> <name>, ticks after the serial line "[inj] desktop armed").
# env DESK_SCRATCH (scratch, required), DESK_FILES (dir of files to put on C:, optional), QEMU_EXTRA
set -euo pipefail
REPO=/Volumes/Daten/Development/PiRusTOS/.claude/worktrees/agent-a76fdd16f4fbe1958
cd "$REPO"
BASE=${DESK_SCRATCH:?}
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/desk.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }
case $1 in
kernel)
    COMMIT=$2; MOUSE=$3; KOUT=$4
    T=$SCR/tree; mkdir -p "$T"
    git archive "$COMMIT" | tar -x -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I tools/inject-arm.py "$T" "desktop=$MOUSE" | tail -1
    (cd "$T" && cargo xtask build --board rpi4 --release) > "$SCR/build.log" 2>&1 || { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (commit $(git rev-parse "$COMMIT"), armed desktop, mouse $(sha "$MOUSE"))"
    ;;
newtos)
    OUT=$2; KERNEL=$3; INF=$4; SHOTS=$5
    [ ! -e "$OUT" ] || { echo "out exists" >&2; exit 1; }
    mkdir -p "$OUT/shots"; OUT=$(cd "$OUT" && pwd)
    PAIRS=()
    if [ -n "${DESK_FILES:-}" ]; then for f in "$DESK_FILES"/*; do PAIRS+=("$(basename "$f")=$f"); done; fi
    cp "$INF" "$OUT/NEWDESK.INF"; cp "$SHOTS" "$OUT/shots.txt"
    tools/mkcard.sh "$SCR/card.img" "$INF" ${PAIRS[@]+"${PAIRS[@]}"} > "$SCR/mkcard.stdout" 2>&1 || { cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    { echo "desk-inj run newtos ($(date -u +%Y-%m-%dT%H:%M:%SZ))"; echo "kernel $(sha "$KERNEL")"; echo "card $(sha "$SCR/card.img")"; echo "shots $(sha "$SHOTS")"; echo "qemu-extra [${QEMU_EXTRA:-}]"; } > "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    cd "$SCR"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        ${QEMU_EXTRA:-} -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw \
        -qmp unix:qmp.sock,server=on,wait=off &
    pid=$!
    for _ in $(seq 1 100); do [ -S qmp.sock ] && break; sleep 0.1; done; sleep 2
    python3 -I "$REPO/tosdump/probes/shprobe-qmpshots.py" qmp.sock "$OUT/BOOT-SERIAL.txt" "$OUT/shots.txt" "$OUT/shots" "[inj] desktop armed at hz_200" &
    spid=$!
    wait "$spid" 2>/dev/null || echo "qmpshots failed" >> "$OUT/run.txt"
    sleep 1
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "shots $(ls "$OUT/shots" | grep -c png)" >> "$OUT/run.txt"
    grep -a -c -i -E "exception|panic" "$OUT/BOOT-SERIAL.txt" | sed 's/^/exception-panic-lines /' >> "$OUT/run.txt" || true
    cat "$OUT/run.txt"
    ;;
esac
