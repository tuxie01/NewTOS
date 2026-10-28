#!/bin/sh
# mktestdisk-repro.sh <empty scratch dir> -- is tools/mktestdisk.sh byte-reproducible,
# and is its FAT16 clean? Builds the empty image three times 7 s apart (different
# seconds of the host clock), an image with two files (one in a folder, one with
# its own stamp) twice 13 s apart, prints every sha256 and runs fsck_msdos -n on
# the dd'd-out partition (LBA 63, 130977 sectors) of each kind. Repository root.
set -eu
S=$1
[ ! -e "$S" ] || { echo "scratch dir exists: $S" >&2; exit 2; }
mkdir -p "$S"
printf 'tool06 test file\r\n' > "$S/f.txt"
for n in 1 2 3; do tools/mktestdisk.sh "$S/empty-$n.img" > /dev/null; date -u +"empty-$n built %H:%M:%S"; sleep 7; done
for n in 1 2; do
    tools/mktestdisk.sh "$S/files-$n.img" A/B.TXT="$S/f.txt" X.TXT="$S/f.txt@202001010000" > /dev/null
    date -u +"files-$n built %H:%M:%S"; sleep 13
done
(cd "$S" && shasum -a 256 empty-*.img files-*.img)
for k in empty-1 files-1; do
    dd if="$S/$k.img" of="$S/$k.part" bs=512 skip=63 count=130977 status=none
    echo "fsck_msdos -n $k (partition):"; rc=0
    fsck_msdos -n "$S/$k.part" > "$S/fsck.txt" 2>&1 || rc=$?
    sed 's#.*/##' "$S/fsck.txt"; echo "fsck_msdos exit $rc"
done
MTOOLSRC=/dev/null mdir -/ -i "$S/files-1.img@@32256" ::
MTOOLSRC=/dev/null minfo -i "$S/files-1.img@@32256" :: | sed -n '/^bootsector/,$p'
echo "MBR partition table (offset 446, 66 bytes):"; xxd -s 446 -l 66 "$S/empty-1.img"
mformat --version | head -1
rm -f "$S"/*.img "$S"/*.part "$S"/*.manifest "$S/fsck.txt"
