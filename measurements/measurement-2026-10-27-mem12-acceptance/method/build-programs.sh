#!/usr/bin/env bash
# build-programs.sh -- MEM12 3b: rebuild every SDK program in one go after the osbind.h change (Super/Supexec as
# traps again). Order of docs/sdk/06-image-and-testing.md "Build order", plus HypView, DOOM, APITEST and the three
# NewTOS dumpers. One log per step in <run>/logs/build-*.log; the first failing step stops the script.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"; R="$HERE/.."; WS="$(cd "$R/../.." && pwd)"; L="$R/logs"
cd "$WS"
step() { local n="$1"; shift; "$@" > "$L/build-$n.log" 2>&1; local rc=$?; echo "$n rc=$rc"; [ $rc -eq 0 ] || exit 1; }
step libc        sdk/libc/build-libc.sh
step argvtest    sdk/libc/build-libc.sh sdk/examples/argvtest.c sdk/libc/build/argvtest.NAP
step clocktest   sdk/libc/build-libc.sh sdk/libc/tests/clocktest.c
step setjmptst   sdk/libc/build-libc.sh sdk/examples/setjmptst.c sdk/libc/build/setjmptst.NCP
step mxprobe     sdk/libc/build-libc.sh sdk/libc/tests/mxprobe.c sdk/libc/build/MXPROBE.NCP
step libm        sdk/libm/build-libm.sh
step ldg-demo    sdk/ldg/build-ldg-demo.sh
step zview       sdk/zview/build-zview.sh
step coremark    sdk/apps/build-coremark.sh
step dumpvdi     make -C tosdump -B dumpvdi-newtos
step vdidraw     tosdump/build-vdidraw-newtos.sh
step dumpbios    make -C tosdump -B dumpbios-newtos
step qed         sdk/apps/build-qed.sh
step teradesk    sdk/apps/build-teradesk.sh
step present     sdk/apps/present/build-present.sh
step hypview     sdk/apps/build-hypview.sh
step doom        sdk/apps/build-doom.sh
step apitest     sdk/apps/apitest/build-apitest.sh
echo ALL-OK
