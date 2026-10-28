#!/bin/sh
# TOOL04: are first launches serialised by the system? Launch N fresh binaries
# (arguments) at the same time with --list; each line = that launch's wall time.
M=$(dirname "$0")
t0=$(python3 -I -c 'import time; print(time.time())')
for b in "$@"; do python3 -I "$M/t.py" "$(basename "$b")" "$b" --list & done
wait
python3 -I -c "import time; print('total\t%.3f' % (time.time() - $t0))"
