#!/bin/bash
# compare-testpart.sh <before.img> <after.img> -- the image comparison of TASK/TOOL07 item 4,
# on the TEST PARTITION only (dd'd out of the card / the oracle's disk.img by the caller).
#   1. raw: sha256 of both; equal -> "testpart: identical".
#   2. otherwise: the differing 512-byte sectors, and the same comparison with the FAT
#      time stamps normalised (normalise-fat-times.py: create/access/write time and date of
#      every root directory entry zeroed) -- "identical after normalising time stamps" or not.
#   3. fsck_msdos -n on both partition files.
set -u
B=$1; A=$2; HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
sb=$(shasum -a 256 "$B" | cut -d' ' -f1); sa=$(shasum -a 256 "$A" | cut -d' ' -f1)
echo "testpart before $sb"
echo "testpart after  $sa"
rc=0
if [ "$sb" = "$sa" ]; then
  echo "testpart: identical"
else
  echo "testpart: DIFFERENT; differing sectors (partition-relative):"
  cmp -l "$B" "$A" | awk '{ s = int(($1 - 1) / 512); if (s != last) { print "  sector " s; last = s } }' | head -50
  python3 -I "$HERE/normalise-fat-times.py" "$B" "$A" || rc=1
fi
for f in "$B" "$A"; do
  echo "fsck_msdos -n $(basename "$f"):"
  fsck_msdos -n "$f" 2>&1 | sed 's/^/  /'; echo "  exit ${PIPESTATUS[0]}"
done
exit $rc
