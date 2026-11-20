#!/bin/sh
# control-head-dumper.sh <wt> <scratch>: the control for oracle-regression.sh.
# DUMPBIOS.PRG built from HEAD 13b18e50f (git archive of tosdump/ and
# tools/oracle/oracle.env, build id 13b1) runs bios-r1.tsv on the oracle (tt);
# its records are compared with this tree's dumper's run and with the frozen
# reference, record by record: which records differ, and in which bytes.
set -u
WT=$1; S=$2
RUN=$(cd "$(dirname "$0")/.." && pwd)/runs
mkdir -p "$S/headsrc"
(cd "$WT" && git archive 13b18e50f tosdump tools/oracle/oracle.env) | tar -x -C "$S/headsrc"
make -C "$S/headsrc/tosdump" dumpbios-m68k BUILD_ID=13b1 > /dev/null 2>&1
"$WT/tools/hatari-oracle.sh" tt "$S/headsrc/tosdump/build/DUMPBIOS.PRG" "$WT/tosdump/cases/bios-r1.tsv" "$S/control-head-bios-r1-tt" > "$S/control.out" 2>&1
python3 -I - "$S/control-head-bios-r1-tt/BIOS.BIN" "$S/regress-bios-r1-tt/BIOS.BIN" "$WT/tosdump/reference/EMUTOS-1a7484ab-BIOS.BIN" "$WT/tosdump/cases/bios-r1.tsv" > "$RUN/control-head-dumper.txt" <<'PY'
import struct, sys
h, t, r = (open(p, 'rb').read()[32:] for p in sys.argv[1:4])
lines = open(sys.argv[4], encoding='latin-1').read().split('\n')
def diff(x, y):
    out = []
    for i in range(len(x) // 320):
        a, b = x[i*320:(i+1)*320], y[i*320:(i+1)*320]
        if a != b:
            cid = struct.unpack('>H', a[:2])[0]
            out.append('record %d case %d bytes %s  %s' % (i, cid, [k for k in range(320) if a[k] != b[k]], lines[cid-1].split('\t')[4]))
    return out or ['none']
print('HEAD dumper vs this dumper (bios-r1, tt, same day):'); print('\n'.join(diff(h, t)))
print('HEAD dumper vs frozen reference EMUTOS-1a7484ab-BIOS:'); print('\n'.join(diff(h, r)))
print('this dumper vs frozen reference EMUTOS-1a7484ab-BIOS:'); print('\n'.join(diff(t, r)))
PY
