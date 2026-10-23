#!/usr/bin/env bash
# api-card.sh <out.img> api <files...> -- A113 APITEST card (copied from measurement-2026-10-17-a112-fix/method/api-card.sh, label A113API) (QEMU raspi4b only): 64 MiB, MBR + one FAT16 partition
# (type 0x06, as tools/make-sd-image.sh), volume label A113API (never NEWTOS: no clash with a real card's mount).
# Copied from measurements/measurement-2026-10-17-arch02/method/make-card.sh (kind fp removed).
#   kind api  <APITEST.NCP> <PTERMCH.NCP> <DTACHILD.NCP>
#                                                C:\APITEST.NCP + C:\PTERMCH.NCP + C:\DTACHILD.NCP, NEWDESK.INF = '#Z 01 C:\APITEST.NCP@'
#                                                (the EmuTOS #Z autorun line, as measurement-2026-10-17-mem02).
# Runs use per-run APFS clones (cp -c) of these.
set -euo pipefail
IMG="$1"; KIND="$2"; shift 2
rm -f "$IMG"; dd if=/dev/zero of="$IMG" bs=1m count=64 status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" | head -1 | awk '{print $1}')
trap 'hdiutil detach "$DEV" >/dev/null 2>&1 || true' EXIT
diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" A113API 100% >/dev/null
V="$(diskutil info "${DEV}s1" | sed -n 's/^ *Mount Point: *//p')"
[ -d "$V" ] && [ "$(basename "$V")" = A113API ] || { echo "error: unexpected mount point '$V'" >&2; exit 1; }
mdutil -i off "$V" >/dev/null 2>&1 || true
export COPYFILE_DISABLE=1
case "$KIND" in
  api) cp -X "$1" "$V/APITEST.NCP"; cp -X "$2" "$V/PTERMCH.NCP"; cp -X "$3" "$V/DTACHILD.NCP"
       printf '#Z 01 C:\\APITEST.NCP@\r\n' > "$V/NEWDESK.INF";;
  *) echo "kind? $KIND" >&2; exit 2;;
esac
find "$V" -name '._*' -delete 2>/dev/null || true
rm -rf "$V/.fseventsd" "$V/.Spotlight-V100" "$V/.Trashes" "$V/.TemporaryItems" 2>/dev/null || true
ls -la "$V" "$V/AUTO" 2>/dev/null | grep -v '^total' || true
sync; diskutil unmount "${DEV}s1" >/dev/null
echo "fsck: $(fsck_msdos -n "${DEV}s1" 2>&1 | tr '\n' ' ')"
