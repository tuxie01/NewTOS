# decode-case.py <BIOS.BIN> <case id>... -- print trap, function and the 32-bit
# return value of BIOS-area records (record format 3, docs/TOS-REFERENCE-DUMPER.md
# section 3: 32-byte header, 320-byte records, case id @0, ret @36, trap @300;
# m68k dump = big endian). Run with python3 -I.
import struct, sys
b = open(sys.argv[1], 'rb').read()
want = set(int(x) for x in sys.argv[2:])
for off in range(32, len(b), 320):
    r = b[off:off + 320]
    cid, op = struct.unpack('>HH', r[0:4])
    if cid in want:
        ret, = struct.unpack('>I', r[36:40])
        trap, = struct.unpack('>H', r[300:302])
        print(f"case {cid} trap {trap} function {op} ret 0x{ret:08X}")
