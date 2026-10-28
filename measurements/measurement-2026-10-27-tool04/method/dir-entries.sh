#!/bin/sh
# TOOL04: first launch vs. the number of entries in the executable's directory.
# $1 = a freshly linked, never launched binary; $2 = an empty probe dir on the
# volume under test; $3 = number of empty files to create beside it.
# The binary is hard-linked in (same inode/content, new path), launched twice.
M=$(dirname "$0"); bin=$1; d=$2; n=$3
mkdir -p "$d"
python3 -I -c "
import os, sys
d, n = sys.argv[1], int(sys.argv[2])
for i in range(n):
    open(os.path.join(d, 'f%07d.o' % i), 'w').close()
" "$d" "$n"
ln "$bin" "$d/probe-bin"
printf 'entries=%s\t' "$(ls -f "$d" | wc -l | tr -d ' ')"
python3 -I "$M/t.py" first "$d/probe-bin" --list | tr '\n' '\t'
python3 -I "$M/t.py" second "$d/probe-bin" --list
