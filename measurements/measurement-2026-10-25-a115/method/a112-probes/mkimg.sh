#!/bin/bash
# usage: mkimg.sh <img> files...
set -e
IMG=$1; shift
dd if=/dev/zero of=$IMG bs=1m count=64 status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" 2>/dev/null | head -1 | awk '{print $1}')
diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" EVPRB 100% >/dev/null
V=$(diskutil info "${DEV}s1" | sed -n 's/^ *Mount Point: *//p')
cp "$@" "$V/"
printf '#Z 01 C:\\MAIN.NAP@\r\n' > "$V/NEWDESK.INF"
rm -rf "$V/.Spotlight-V100" "$V/.fseventsd" "$V/.Trashes" 2>/dev/null || true
dot_clean -m "$V"; ls "$V"; sync
diskutil unmount "${DEV}s1" >/dev/null; hdiutil detach $DEV >/dev/null 2>&1
