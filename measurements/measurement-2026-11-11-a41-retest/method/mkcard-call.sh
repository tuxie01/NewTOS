#!/usr/bin/env bash
# The one tools/mkcard.sh call of this run (CLAUDE.md placement rules). Run from the repo root.
# The image is the QEMU mirror of what the retest puts on the owner's card (BOOT = C:): the release
# kernel as C:\kernel8.img, TeraDesk with its three resources incl. the colour icons, QED.
# NEWDESK.INF here autoruns TeraDesk (#Z) for the QEMU smoke run only; the owner's card keeps its own.
set -euo pipefail
R=measurements/measurement-2026-11-11-a41-retest
IMG="${A41_IMG:?set A41_IMG to the image path; its folder must not lie on /Volumes/Daten, where diskutil refuses the mount point}"
MKCARD_LABEL=NTA41 tools/mkcard.sh "$IMG" "$R/method/newdesk-teradesk.inf" \
  kernel8.img="$R/kernels/kern1111.img" \
  Apps/TERADESK/DESKTOP.NAP=sdk/apps/teradesk/build/dist/DESKTOP.NAP \
  Apps/TERADESK/desktop.rsc=sdk/apps/teradesk/build/dist/desktop.rsc \
  Apps/TERADESK/icons.rsc=sdk/apps/teradesk/build/dist/icons.rsc \
  Apps/TERADESK/cicons.rsc=sdk/apps/teradesk/build/dist/cicons.rsc \
  Apps/TERADESK/TERADESK.INF="$R/method/teradesk.inf" \
  Apps/QED/QED.NAP=sdk/apps/qed/build/QED.NAP \
  Apps/QED/qed.rsc=sdk/apps/qed/build/qed.rsc
cp "$IMG.manifest" "$R/a41-retest.img.manifest"
