#!/usr/bin/env bash
# mkcard.sh <img> <NEWDESK.INF> [dest=host-file ...] -- fresh 64 MiB MBR + FAT16 QEMU card, label USB01T (private mount point,
# never a real card). Copied from measurements/measurement-2026-10-18-proc04-fdelete/method/mkcard.sh.
set -euo pipefail
IMG="${1:?}"; INF="${2:?}"; shift 2
rm -f "$IMG"; dd if=/dev/zero of="$IMG" bs=1m count=64 status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" | head -1 | awk '{print $1}')
MNT="$(dirname "$IMG")/mnt.$$"
cleanup() { diskutil unmount "$MNT" >/dev/null 2>&1 || true; hdiutil detach "$DEV" >/dev/null 2>&1 || true; rmdir "$MNT" 2>/dev/null || true; }
trap cleanup EXIT
diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" USB01T 100% >/dev/null
diskutil unmount "${DEV}s1" >/dev/null
mkdir -p "$MNT"; diskutil mount -mountPoint "$MNT" "${DEV}s1" >/dev/null
export COPYFILE_DISABLE=1; mdutil -i off "$MNT" >/dev/null 2>&1 || true
for d in ${DIRS-SUB TMP}; do mkdir -p "$MNT/$d"; done   # DIRS: empty folders to create (default SUB TMP)
for spec in "$@"; do d="${spec%%=*}"; s="${spec#*=}"; mkdir -p "$MNT/$(dirname "$d")"; cp -X "$s" "$MNT/$d"; touch -t 202610171200 "$MNT/$d"; done
cp -X "$INF" "$MNT/NEWDESK.INF"; touch -t 202610171200 "$MNT/NEWDESK.INF"
find "$MNT" -name '._*' -delete 2>/dev/null || true; dot_clean -m "$MNT" 2>/dev/null || true
rm -rf "$MNT/.fseventsd" "$MNT/.Spotlight-V100" "$MNT/.Trashes" 2>/dev/null || true
sync; (find "$MNT" 2>/dev/null | sed "s|$MNT||" | sort | head -40) || true
