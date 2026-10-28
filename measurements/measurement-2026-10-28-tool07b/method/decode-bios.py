"""decode-bios.py BIN [TSV] -- one line per record of a DUMPBIOS dump (format 3, 320-byte
records, docs/TOS-REFERENCE-DUMPER.md section 3): case id, trap:function, ret as the 64-bit
ret_hi:ret, the first 18 capture bytes, and the case comment from the table when given."""
import struct
import sys

data = open(sys.argv[1], 'rb').read()
comments = {}
if len(sys.argv) > 2:
    for n, line in enumerate(open(sys.argv[2], encoding='latin-1'), 1):
        cols = line.rstrip('\r\n').split('\t')
        if len(cols) >= 5 and not line.startswith(('#', '!', '+')):
            comments[n] = cols[4][:60]
size, count = struct.unpack_from('>HH', data, 8)
for i in range(count):
    r = data[32 + i * size: 32 + (i + 1) * size]
    cid, op = struct.unpack_from('>HH', r, 0)
    ret, = struct.unpack_from('>I', r, 36)
    hi, = struct.unpack_from('>I', r, 296)
    trap, = struct.unpack_from('>H', r, 300)
    v = (hi << 32) | ret
    if v >= 1 << 63:
        v -= 1 << 64
    cap = r[40:58].hex()
    print(f"{cid:3d} {trap:2d}:{op:<3d} ret={v:<12d} 0x{ret:08X} cap={cap} {comments.get(cid, '')}")
