#!/bin/zsh
# TOOL04 profile run: is the debug info of a packed test binary usable?
# Usage: debuginfo-check.sh <test binary in target/debug/deps>
set -u
b=$1; cd ${b:h}; b=${b:t}
echo "== binary and .dSYM"; ls -ld $b $b.dSYM
echo "== UUIDs (binary and dSYM must match)"; dwarfdump --uuid $b $b.dSYM
echo "== debug map (N_OSO) of the binary: own .o vs rlib members, present on disk?"
dsymutil -s $b | grep N_OSO | sed -E "s/.*'(.*)'.*/\1/" | python3 -I -c '
import os,sys,re,collections
c=collections.Counter()
for l in sys.stdin:
    l=l.strip(); p=re.sub(r"\(.*\)$","",l)
    c[("rlib member" if "(" in l else "own .o", "present" if os.path.exists(p) else "deleted")]+=1
for k,v in sorted(c.items()): print(v,*k)'
echo "== lldb: symbol file loaded, source lines in the test file and in tos-api"
lldb --batch -o "target modules list -S $b" \
  -o "breakpoint set -f aes_window_rectlist_capacity.rs -l 347" \
  -o "breakpoint set -f window.rs -l 3360" $b 2>&1 | grep -vE '^\(lldb\)|^Current|^$'
