#!/usr/bin/env bash
# manifest-vs-image.sh <image> <manifest> <scratch-dir> -- independent check of a manifest against its image with
# mtools (partition at LBA 63, as diskutil lays out the one-partition MBR): image sha256, and every `file` line's
# sha256 against the file read back out of the FAT; then the file list and the FAT dates of the root entries.
set -euo pipefail
IMG="${1:?}"; MAN="${2:?}"; X="${3:?}"; OFF=$((63*512))
echo "image sha256 now:      $(shasum -a 256 "$IMG" | awk '{print $1}')"
echo "manifest image_sha256: $(awk '/^image_sha256/{print $2}' "$MAN")"
rm -rf "$X"; mkdir -p "$X"; mcopy -s -n -i "$IMG@@$OFF" ::/ "$X/" 2>/dev/null
grep '^file ' "$MAN" | while read -r _ sha size tos; do
  rel=$(printf '%s' "${tos#C:\\}" | tr '\\' '/'); got=$(shasum -a 256 "$X/$rel" | awk '{print $1}')
  [ "$got" = "$sha" ] && r=match || r=MISMATCH; echo "$r $tos ($size bytes)"; done
echo "files and folders in the image (mtools; .Spotlight-V100 left out):"
(cd "$X" && find . -path ./.Spotlight-V100 -prune -o -print | LC_ALL=C sort | sed 's/^/  /')
echo "FAT dates of the root entries (mdir):"; mdir -i "$IMG@@$OFF" ::/ | grep -E '^(NEWDESK|ARGVTEST|SUB|TMP|AUTO) ' | sed 's/^/  /'
