#!/bin/bash
# V56: desktop with the IKBD injector (640x480 and 1920x1080) and boot screen + AUTO console (both sizes), base vs
# fix, one QEMU at a time. The V34 test run's desk-inj.sh / boot-console.sh / inputs unchanged; the armed desktop
# kernels are built here from the scratch trees WITHOUT their target/ (V35's desk-kernel.sh copies it).
set -euo pipefail
cd /Volumes/Daten/Development/PiRusTOS
S=${S:?}; export S
V=measurements/measurement-2026-11-07-v34-test/method
export DESK_SCRATCH=$S/dk DESK_FILES=$S/files
for T in base fix; do
  [ -f $S/k/k-desk-$T.img ] && continue
  W=$(mktemp -d "$S/dkk.XXXXXX"); (cd "$S/t/$T" && tar --exclude ./target --exclude ./references -cf - .) | tar -xf - -C "$W"; ln -s "$PWD/references" "$W/references"
  python3 -I tools/inject-arm.py "$W" "desktop=$V/desk-mouse.txt" | tail -1
  (cd "$W" && cargo xtask build --board rpi4 --release) > "$W.build.log" 2>&1 || { tail -30 "$W.build.log" >&2; exit 1; }
  cp "$W/target/aarch64-unknown-none-softfloat/release/kernel8.img" $S/k/k-desk-$T.img; rm -rf "$W" "$W.build.log"
  echo "desk kernel $T $(shasum -a 256 $S/k/k-desk-$T.img | cut -d' ' -f1)"
done
[ -n "${KERNELS_ONLY:-}" ] && exit 0
mkdir -p $S/files; for i in $(seq -w 1 40); do printf 'V53 desktop file %s\r\n' "$i" > "$S/files/LONGNM$i.TXT"; done
mkdir -p $S/runs/gui
for g in "480:" "1080:-global bcm2835-fb.xres=1920 -global bcm2835-fb.yres=1080"; do
  n=${g%%:*}; x=${g#*:}
  for T in base fix; do QEMU_EXTRA="$x" $V/desk-inj.sh newtos $S/runs/gui/desk$n-$T $S/k/k-desk-$T.img $V/desk-NEWDESK.INF $V/desk-shots.txt | tail -3; done
  for T in base fix; do bash $V/boot-console.sh $S/runs/gui/bc$n-$T $S/k/$T-release.img "$x" | tail -3; done
done
echo GUIFINISHED
