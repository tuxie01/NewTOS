#!/usr/bin/env bash
#
# run-akprobe.sh -- the AES keyboard-state probe tosdump/probes/akprobe.c
# (TASK/A126) on one side, with the key strokes and the mouse button of
# tosdump/probes/akprobe-keys.txt injected. Copied from run-ksprobe.sh; the one
# difference is the mouse packet, which Hatari gets as rightdown / rightup.
# No disk: the probe touches no drive but its own.
#
#   tosdump/probes/run-akprobe.sh emutos tt|falcontc <out-dir>
#   tosdump/probes/run-akprobe.sh kernel <tree> <kernel-out> [<features>]
#   tosdump/probes/run-akprobe.sh newtos <out-dir> <kernel8.img>
#
#   emutos   the pinned oracle, started as tools/hatari-oracle.sh starts
#            profile tt / falcontc (ROM, tools/oracle/hatari.cfg, hermetic
#            HOME, GEMDOS drive C:, --auto C:\AKPROBE.PRG; falcontc's
#            EMUDESK.INF), with --fast-forward off (real-time VBLs: the key
#            pauses are wall-clock) and the input through --cmd-fifo
#            ("hatari-event keydown|keyup <scancode>", the packets 0xF9 0 0 /
#            0xF8 0 0 as "hatari-event rightdown|rightup"), the first one 0.3 s
#            after C:\AKPROBE.RDY appears.
#   kernel   the NewTOS kernel the newtos side needs: a copy of <tree> (its
#            tracked and untracked-not-ignored files, references/ linked) in
#            scratch, armed by `tools/inject-arm.py <copy>
#            launch:AKPROBE=tosdump/probes/akprobe-keys.txt` (the IKBD bytes
#            go to the decoder, kernel/src/ikbd.rs feed, at their tick after
#            the launch), built `cargo xtask build --board rpi4 --release
#            [--features <features>]` (bios-arg-trace for the input check);
#            kernel8.img is copied to <kernel-out>. <tree> itself is not
#            touched.
#   newtos   tools/mkcard.sh (C:\AKPROBE.NAP, C:\NEWDESK.INF = "#R 02",
#            "#Z 01 C:\AKPROBE.NAP@"), QEMU raspi4b booting an APFS clone of
#            the card with `-append serial=on`; the desktop's #Z autorun
#            starts the probe. Without an armed kernel no input comes: S01
#            says "timeout", E01 and K01 return MU_TIMER.
#
# <out-dir> (must not exist) receives AKPROBE.LOG, run.txt (what ran, sha256
# of the probe, card, kernel) and the side's log (hatari.log + inject.txt, or
# BOOT-SERIAL.txt + card.img.manifest).
#
# Scratch: $AKPROBE_SCRATCH, else $CLAUDE_JOB_DIR/tmp; never /tmp itself.
# Deleted at the end.
set -euo pipefail

usage() {
    echo "usage: $0 emutos tt|falcontc <out-dir> | kernel <tree> <kernel-out> [<features>] | newtos <out-dir> <kernel8.img>" >&2
    exit 2
}
[ $# -ge 3 ] || usage
SIDE=$1
case $SIDE in
    emutos) [ $# -eq 3 ] || usage; PROFILE=$2; OUT=$3
            case $PROFILE in tt|falcontc) ;; *) usage ;; esac ;;
    kernel) [ $# -le 4 ] || usage; TREE=$(cd "$2" && pwd); KOUT=$3; FEATURES=${4:-} ;;
    newtos) [ $# -eq 3 ] || usage; OUT=$2; KERNEL=$3
            [ -f "$KERNEL" ] || { echo "kernel not found: $KERNEL" >&2; exit 1; } ;;
    *) usage ;;
esac

HERE=$(cd "$(dirname "$0")" && pwd)
TD=$(cd "$HERE/.." && pwd)
REPO=$(cd "$TD/.." && pwd)
KEYS=$HERE/akprobe-keys.txt
BASE=${AKPROBE_SCRATCH:-${CLAUDE_JOB_DIR:+$CLAUDE_JOB_DIR/tmp}}
[ -n "$BASE" ] || { echo "set AKPROBE_SCRATCH or CLAUDE_JOB_DIR (scratch, never /tmp)" >&2; exit 2; }
case $(cd "$BASE" 2>/dev/null && pwd -P || echo "$BASE") in
    /tmp|/private/tmp) echo "scratch must not be /tmp itself" >&2; exit 2 ;;
esac
mkdir -p "$BASE"
SCR=$(mktemp -d "$BASE/akprobe.XXXXXX")
trap 'rm -rf "$SCR"' EXIT
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

if [ "$SIDE" = kernel ]; then
    T=$SCR/tree
    mkdir -p "$T"
    (cd "$TREE" && git ls-files -z -co --exclude-standard | tar --null -T - -cf -) | tar -x -C "$T"
    ln -s "$REPO/references" "$T/references"
    python3 -I "$REPO/tools/inject-arm.py" "$T" "launch:AKPROBE=$KEYS" | tail -1
    feat=(); [ -z "$FEATURES" ] || feat=(--features "$FEATURES")
    (cd "$T" && cargo xtask build --board rpi4 --release ${feat[@]+"${feat[@]}"}) > "$SCR/build.log" 2>&1 ||
        { tail -30 "$SCR/build.log" >&2; exit 1; }
    cp "$T/target/aarch64-unknown-none-softfloat/release/kernel8.img" "$KOUT"
    echo "kernel $(sha "$KOUT") $KOUT (tree $TREE, armed launch:AKPROBE, features '${FEATURES}')"
    exit 0
fi

[ ! -e "$OUT" ] || { echo "out-dir exists: $OUT" >&2; exit 1; }
mkdir -p "$OUT"
OUT=$(cd "$OUT" && pwd)
cd "$REPO"

make -s -C tosdump akprobe-m68k akprobe-newtos > "$SCR/make.txt" 2>&1 || { cat "$SCR/make.txt" >&2; exit 1; }
PRG=$TD/build/AKPROBE.PRG
NAP=$TD/build/AKPROBE.NAP
{
    echo "akprobe run, side $SIDE ($(date -u +%Y-%m-%dT%H:%M:%SZ))"
    echo "tree      $(git rev-parse HEAD) $(git status --porcelain -- kernel tos-api tos-types tosdump/probes | grep -c . || true) dirty paths in kernel tos-api tos-types tosdump/probes"
    echo "probes    AKPROBE.PRG $(sha "$PRG") AKPROBE.NAP $(sha "$NAP")"
    echo "keys      $(sha "$KEYS") tosdump/probes/akprobe-keys.txt"
} > "$OUT/run.txt"

if [ "$SIDE" = emutos ]; then
    . "$REPO/tools/oracle/oracle.env"
    [ "$(sha "$ORACLE_ROM")" = "$ORACLE_ROM_SHA256" ] || { echo "pinned ROM mismatch" >&2; exit 1; }
    case $PROFILE in
        tt)       MACHINE="--machine tt --tos-res ttmed --monitor vga" ;;
        falcontc) MACHINE="--machine falcon --monitor vga" ;;
    esac
    mkdir -p "$SCR/hd" "$SCR/home"
    cp "$PRG" "$SCR/hd/AKPROBE.PRG"
    if [ "$PROFILE" = falcontc ]; then
        printf '#R 02\r\n#E 1A E0 00 1C 60\r\n#Q 41 40 43 40 43 40\r\n#W 00 00 02 06 26 0C 00 @\r\n#W 00 00 02 08 26 0C 00 @\r\n#W 00 00 02 0A 26 0C 00 @\r\n#W 00 00 02 0D 26 0C 00 @\r\n#M 00 00 00 FF C HARD DISK C@ @\r\n#M 01 00 00 FF D HARD DISK D@ @\r\n#F FF 07 @ *.*@\r\n#T 00 03 03 FF   TRASH@ @\r\n' \
            > "$SCR/hd/EMUDESK.INF"
    fi
    echo "oracle    $ORACLE_HATARI, ROM $ORACLE_ROM ($ORACLE_ROM_SHA256), profile $PROFILE" >> "$OUT/run.txt"
    # shellcheck disable=SC2086
    HOME="$SCR/home" SDL_VIDEODRIVER=dummy "$ORACLE_HATARI" \
        --configfile "$ORACLE_HATARI_CFG" $MACHINE --tos "$ORACLE_ROM" \
        --sound off --fast-forward off --conout 2 --run-vbls 60000 \
        -d "$SCR/hd" --gemdos-drive C --auto 'C:\AKPROBE.PRG' --cmd-fifo "$SCR/cmd" \
        > "$OUT/hatari.log" 2>&1 &
    pid=$!
    i=0; state=wait
    : > "$OUT/inject.txt"
    while kill -0 "$pid" 2>/dev/null; do
        if [ $state = wait ] && [ -e "$SCR/hd/AKPROBE.RDY" ]; then
            sleep 0.3
            last=
            # shellcheck disable=SC2034
            grep -v '^#' "$KEYS" | grep . | while read -r t b rest; do
                [ -z "$last" ] || sleep "$(LC_ALL=C awk -v d=$((t - last)) 'BEGIN { printf "%.3f", d / 200 }')"
                last=$t
                v=$((b))
                case "$v $rest" in
                    "249 0x00 0x00") ev="rightdown" ;;
                    "248 0x00 0x00") ev="rightup" ;;
                    *) [ -z "$rest" ] || { echo "$t $b $rest: no Hatari event for this packet" >> "$OUT/inject.txt"; continue; }
                       if [ $v -ge 128 ]; then ev="keyup $((v - 128))"; else ev="keydown $v"; fi ;;
                esac
                echo "hatari-event $ev" > "$SCR/cmd"
                echo "$t $b${rest:+ $rest} hatari-event $ev ($(date +%H:%M:%S))" >> "$OUT/inject.txt"
            done
            state=sent
        fi
        if [ -s "$SCR/hd/AKPROBE.LOG" ] && grep -q '^Z99 end' "$SCR/hd/AKPROBE.LOG"; then
            sleep 1; break
        fi
        i=$((i + 1)); [ "$i" -lt 900 ] || { echo "timeout at poll $i" >> "$OUT/inject.txt"; break; }
        sleep 0.1
    done
    kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    [ -s "$SCR/hd/AKPROBE.LOG" ] || { echo "no AKPROBE.LOG -- see $OUT/hatari.log" >&2; exit 1; }
    tr -d '\r' < "$SCR/hd/AKPROBE.LOG" > "$OUT/AKPROBE.LOG"
else
    echo "kernel    $(sha "$KERNEL") $KERNEL" >> "$OUT/run.txt"
    printf '#R 02\r\n#Z 01 C:\\AKPROBE.NAP@\r\n' > "$SCR/newdesk.inf"
    tools/mkcard.sh "$SCR/card.img" "$SCR/newdesk.inf" AKPROBE.NAP="$NAP" > "$SCR/mkcard.stdout" 2>&1 || {
        cat "$SCR/mkcard.stdout" >&2; exit 1; }
    cp "$SCR/card.img.manifest" "$OUT/card.img.manifest"
    echo "card      $(sha "$SCR/card.img") (tools/mkcard.sh, C:\\AKPROBE.NAP, #Z autorun)" >> "$OUT/run.txt"
    cp -c "$SCR/card.img" "$SCR/boot.img" 2>/dev/null || cp "$SCR/card.img" "$SCR/boot.img"
    : > "$OUT/BOOT-SERIAL.txt"
    qemu-system-aarch64 -M raspi4b -no-reboot -display none -serial file:"$OUT/BOOT-SERIAL.txt" \
        -append serial=on -kernel "$KERNEL" -drive file="$SCR/boot.img",if=sd,format=raw &
    pid=$!
    state=timeout
    for _ in $(seq 1 150); do
        sleep 2
        if grep -aq "AKPROBE.NAP' exited with rc" "$OUT/BOOT-SERIAL.txt" &&
           grep -aq "desktop up" "$OUT/BOOT-SERIAL.txt"; then state=done; break; fi
        kill -0 "$pid" 2>/dev/null || { state=qemu-exited; break; }
    done
    sleep 3
    kill "$pid" 2>/dev/null || true; sleep 1; kill -9 "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true
    echo "qemu      $state" >> "$OUT/run.txt"
    echo "card      after $(sha "$SCR/boot.img") (the booted clone)" >> "$OUT/run.txt"
    # C: = MBR entry 1; its first sector is the LONG at byte 454.
    cstart=$(od -An -t u4 -j 454 -N 4 "$SCR/boot.img" | tr -d ' ')
    MTOOLS_SKIP_CHECK=1 mcopy -o -i "$SCR/boot.img@@$((cstart * 512))" ::/AKPROBE.LOG "$SCR/AKPROBE.LOG" \
        || { echo "no AKPROBE.LOG on C:" >&2; exit 1; }
    tr -d '\r' < "$SCR/AKPROBE.LOG" > "$OUT/AKPROBE.LOG"
fi
cat "$OUT/AKPROBE.LOG"
