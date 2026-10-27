#!/usr/bin/env bash
#
# mkcard.sh -- a fresh MBR + FAT16 QEMU card for one measurement run, with the
# same `<image>.manifest` as tools/make-sd-image.sh (TASK/TOOL02).
#
# Usage:
#   tools/mkcard.sh <image> <NEWDESK.INF> [dest=host-file[@YYYYMMDDhhmm] ...]
#
#     image          the image file to (re)create; `<image>.manifest` is
#                    written next to it
#     NEWDESK.INF    host file copied to C:\NEWDESK.INF (after the pairs, so it
#                    wins over a pair with the same destination)
#     dest=file      host file copied to C:\<dest>; dest is relative to C:\
#                    with `/` as separator, missing folders are created. An
#                    `@YYYYMMDDhhmm` suffix gives this file its own FAT time
#                    stamp instead of MKCARD_STAMP.
#
#   Environment (all optional; a run states the ones it sets in its call):
#     MKCARD_LABEL     FAT volume label, at most 11 characters (default NTCARD)
#     MKCARD_SIZE_MIB  card size in MiB (default 64)
#     MKCARD_DIRS      empty folders to create on C:\, space-separated
#                      (default: none)
#     MKCARD_STAMP     FAT time stamp of every copied file, `touch -t` form
#                      (default 202610171200, the stamp of the per-run copies)
#
# This replaces the per-run copies of measurements/*/method/mkcard.sh (same
# positional interface; their label, empty folders and stamp are the MKCARD_*
# variables now). What goes onto the card stays in the run's own call, which
# the run keeps in its method/ directory; the tool holds nothing run-specific.
#
# Every input is mandatory: a missing NEWDESK.INF or host file ends the script
# with a non-zero status before the image is touched. On any later failure the
# half-written image and its manifest are deleted, so a manifest always
# belongs to the image beside it.
#
# Manifest: format v1, written by tools/sd-manifest.sh. This tool reads no
# SD_* switches (those belong to tools/make-sd-image.sh), so its manifest says
# `switch n/a`; it has no optional parts, so `optional_missing none`.
#
# The image is attached with hdiutil as a user-owned raw device and the
# partition is mounted on a private mount point next to the image, never
# under /Volumes/<label>. QEMU images only -- never a real card.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
usage() {
  sed -n '/^# Usage:/,/^# This replaces/p' "${BASH_SOURCE[0]}" | sed '$d; s/^# \{0,1\}//' >&2
  exit 2
}
[ "$#" -ge 2 ] || usage
IMG="$1"; INF="$2"; shift 2
LABEL="${MKCARD_LABEL:-NTCARD}"
SIZE_MB="${MKCARD_SIZE_MIB:-64}"
DIRS="${MKCARD_DIRS:-}"
STAMP="${MKCARD_STAMP:-202610171200}"
STAMP_RE='[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]'

# Check every input before the image is touched; report all problems at once.
bad=0
[ -f "$INF" ] || { echo "mkcard: NEWDESK.INF source $INF not found" >&2; bad=$((bad + 1)); }
for spec in "$@"; do
  case "$spec" in
    *=*) ;;
    *) echo "mkcard: '$spec' is not dest=host-file" >&2; bad=$((bad + 1)); continue ;;
  esac
  d="${spec%%=*}"; s="${spec#*=}"
  case "$s" in *@$STAMP_RE) s="${s%@*}" ;; esac
  case "$d" in
    ''|/*|*..*) echo "mkcard: destination '$d' must be relative to C:\\ without '..'" >&2; bad=$((bad + 1)) ;;
  esac
  [ -f "$s" ] || { echo "mkcard: host file $s (for C:\\$d) not found" >&2; bad=$((bad + 1)); }
done
case "$SIZE_MB" in ''|*[!0-9]*) echo "mkcard: MKCARD_SIZE_MIB '$SIZE_MB' is not a number" >&2; bad=$((bad + 1)) ;; esac
if [ "$bad" -ne 0 ]; then
  echo "mkcard: $bad problem(s); no image written" >&2
  exit 1
fi

MANIFEST="${IMG}.manifest"
MANIFEST_FILES="${IMG}.manifest.part"
MNT="$(cd "$(dirname "$IMG")" && pwd)/mnt.$$"
DEV=
cleanup() {
  [ -d "$MNT" ] && { diskutil unmount "$MNT" >/dev/null 2>&1 || true; rmdir "$MNT" 2>/dev/null || true; }
  [ -z "$DEV" ] || hdiutil detach "$DEV" >/dev/null 2>&1 || true
  DEV=
}
trap 'rc=$?; cleanup; [ "$rc" -eq 0 ] || rm -f "$IMG" "$MANIFEST" "$MANIFEST_FILES"' EXIT

echo "mkcard: ${IMG} (${SIZE_MB} MiB, MBR + FAT16, label ${LABEL})"
rm -f "$IMG" "$MANIFEST" "$MANIFEST_FILES"
dd if=/dev/zero of="$IMG" bs=1m count="$SIZE_MB" status=none
DEV=$(hdiutil attach -imagekey diskimage-class=CRawDiskImage -nomount "$IMG" | head -1 | awk '{print $1}')
diskutil partitionDisk "$DEV" 1 MBR "MS-DOS FAT16" "$LABEL" 100% >/dev/null
diskutil unmount "${DEV}s1" >/dev/null
mkdir -p "$MNT"; diskutil mount -mountPoint "$MNT" "${DEV}s1" >/dev/null
export COPYFILE_DISABLE=1; mdutil -i off "$MNT" >/dev/null 2>&1 || true

for d in $DIRS; do mkdir -p "$MNT/$d"; echo "  + C:\\$(printf '%s' "$d" | tr '/' '\\')\\ (empty folder)"; done
for spec in "$@"; do
  d="${spec%%=*}"; s="${spec#*=}"; ts="$STAMP"
  case "$s" in *@$STAMP_RE) ts="${s##*@}"; s="${s%@*}" ;; esac
  mkdir -p "$MNT/$(dirname "$d")"; cp -X "$s" "$MNT/$d"; touch -t "$ts" "$MNT/$d"
  echo "  + C:\\$(printf '%s' "$d" | tr '/' '\\') <- $s"
done
cp -X "$INF" "$MNT/NEWDESK.INF"; touch -t "$STAMP" "$MNT/NEWDESK.INF"
echo "  + C:\\NEWDESK.INF <- $INF"

# Strip macOS metadata (AppleDouble files, volume directories) before hashing.
find "$MNT" -name '._*' -delete 2>/dev/null || true; dot_clean -m "$MNT" 2>/dev/null || true
rm -rf "$MNT/.fseventsd" "$MNT/.Spotlight-V100" "$MNT/.Trashes" "$MNT/.TemporaryItems" 2>/dev/null || true
sync

"$SCRIPT_DIR/sd-manifest.sh" files "$MNT" C "$IMG"
diskutil unmount "$MNT" >/dev/null
rmdir "$MNT"
hdiutil detach "$DEV" >/dev/null
DEV=

"$SCRIPT_DIR/sd-manifest.sh" write "$IMG" "$SIZE_MB" tools/mkcard.sh --switches n/a
echo "done: ${IMG}"
echo "manifest: ${MANIFEST} ($(sed -n '/^file /p' "$MANIFEST" | wc -l | tr -d ' ') files; copy it into the measurement run, RUNINFO.TXT cites it)"
