#!/bin/sh
# TOOL04: does the first-launch wait follow the file (path) or its content?
# $1, $2 = two freshly linked, never launched test binaries; $3 = scratch dir.
# A: copy without xattrs (cp -X) to scratch, launch the copy, then the original.
# B: copy with xattrs (cp) to scratch, launch the copy, then the original.
M=$(dirname "$0"); A=$1; B=$2; S=$3
mkdir -p "$S/copy"
cp -X "$A" "$S/copy/A"; cp "$B" "$S/copy/B"
echo "# A xattrs (copy, original):"; xattr "$S/copy/A"; xattr "$A"
echo "# B xattrs (copy, original):"; xattr "$S/copy/B"; xattr "$B"
python3 -I "$M/t.py" A-copy-noxattr-first "$S/copy/A" --list
python3 -I "$M/t.py" A-original-after   "$A" --list
python3 -I "$M/t.py" B-copy-xattr-first  "$S/copy/B" --list
python3 -I "$M/t.py" B-original-after   "$B" --list
python3 -I "$M/t.py" B-copy-second      "$S/copy/B" --list
