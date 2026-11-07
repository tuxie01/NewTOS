#!/bin/bash
# V53: a program switch through v_clswk/v_opnwk. #Z autoruns WKPROBE.NAP (which closes and reopens the AES's
# physical workstation itself, wkprobe.c W1/W2/C1/Z/W9), the desktop starts when it ends, and the V34 desktop
# injector scene (menu, dialog, rubberband, window/slider/sizer/icon drags) runs on it. The kernel carries both
# arms: launch:WKPROBE=tosdump/probes/wkprobe-mouse.txt and desktop=<V34 desk-mouse.txt>. 640x480, base vs fix.
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}
REPO=$PWD
V=measurements/measurement-2026-11-07-v34-test/method
export DESK_SCRATCH=$S/dk S
# WKPROBE.NAP: run-wkprobe.sh's NewTOS link line
N=$S/sw; mkdir -p $N
SDK=$REPO/sdk; LIBC=$SDK/libc; MINT=$LIBC/mintlib/include; TD=$REPO/tosdump; HERE=$TD/probes
RES="$(clang -print-resource-dir)/include"; SYSROOT="$(rustc --print sysroot)"
BIN="$SYSROOT/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin"
CF=(-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc -D__NEWTOS__)
clang "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -I"$MINT" -isystem "$RES" -c "$HERE/wkprobe.c" -o "$N/wkprobe.o"
clang "${CF[@]}" -std=c99 -Wall -Wextra -Werror -I"$TD/core" -isystem "$RES" -c "$TD/abi/newtos_svc.c" -o "$N/newtos_svc.o"
clang "${CF[@]}" -I"$SDK/include" -I"$MINT" -isystem "$RES" -c "$LIBC/crt0.S" -o "$N/crt0.o"
mkdir -p $N/files
"$BIN/rust-lld" -flavor gnu -T "$SDK/newtos.ld" -pie --no-dynamic-linker -z text -z max-page-size=0x1000 \
    --build-id=none "$N/crt0.o" "$N/wkprobe.o" "$N/newtos_svc.o" "$LIBC/libc.a" -o "$N/files/WKPROBE.NAP"
echo "WKPROBE.NAP $(shasum -a 256 $N/files/WKPROBE.NAP | cut -d' ' -f1)"
for i in $(seq -w 1 40); do printf 'V53 desktop file %s\r\n' "$i" > "$N/files/LONGNM$i.TXT"; done
{ cat $V/desk-NEWDESK.INF; printf '#Z 01 C:\\WKPROBE.NAP@\r\n'; } > $N/NEWDESK.INF
for T in base fix; do
  [ -f $S/k-sw-$T.img ] && continue
  W=$(mktemp -d "$S/swk.XXXXXX"); (cd "$S/t/$T" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$W"; ln -s "$REPO/references" "$W/references"
  python3 -I tools/inject-arm.py "$W" "launch:WKPROBE=$HERE/wkprobe-mouse.txt" "desktop=$V/desk-mouse.txt" | tail -1
  (cd "$W" && cargo xtask build --board rpi4 --release) > "$W.build.log" 2>&1 || { tail -30 "$W.build.log" >&2; exit 1; }
  cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" $S/k-sw-$T.img; rm -rf "$W"
  echo "kernel $T $(shasum -a 256 $S/k-sw-$T.img | cut -d' ' -f1)"
done
[ -n "${KERNELS_ONLY:-}" ] && exit 0
mkdir -p $S/runs/gui
for T in base fix; do DESK_FILES=$N/files $V/desk-inj.sh newtos $S/runs/gui/switch480-$T $S/k-sw-$T.img $N/NEWDESK.INF $V/desk-shots.txt | tail -3; done
echo SWITCHFINISHED
