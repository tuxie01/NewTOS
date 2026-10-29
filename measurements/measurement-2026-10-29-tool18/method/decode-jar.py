#!/usr/bin/env python3
"""decode-jar.py <BIOS.BIN> <table.tsv> -- one line per record of a jar table
(TASK/TOOL18; docs/TOS-REFERENCE-DUMPER.md section 6, "The cookie jar";
tosdump/core/tosdump.h, "A JAR CASE"): the case id, the cookie, its value or
"absent"; for jar:* the count and the tags in order."""
import struct, sys

def tag(v):
    b = struct.pack('>I', v)
    return b.decode('ascii') if all(0x21 <= c <= 0x7E for c in b) else '0x%08X' % v

data = open(sys.argv[1], 'rb').read()
lines = open(sys.argv[2], 'rb').read().decode('ascii').split('\n')
magic, ver, area, rsize, count = struct.unpack('>4sHHHH', data[:12])
assert magic == b'TDMP' and rsize == 320, (magic, rsize)
print('format %d, area %d, %d records, build %s, case CRC %08X' % (
    ver, area, count, data[12:16].decode('ascii', 'replace'),
    struct.unpack('>I', data[16:20])[0]))
for i in range(count):
    r = data[32 + i * 320: 32 + (i + 1) * 320]
    cid, op = struct.unpack('>HH', r[:4])
    arg0 = struct.unpack('>I', r[4:8])[0]
    ret, = struct.unpack('>I', r[36:40])
    hi, = struct.unpack('>I', r[296:300])
    trap, = struct.unpack('>H', r[300:302])
    v = (hi << 32) | ret
    src = lines[cid - 1].split('\t')[0] if cid <= len(lines) else '?'
    if trap != 256:
        print('%4d  %-10s trap %d fn 0x%02X ret 0x%016X' % (cid, src, trap, op, v))
        continue
    if op == 1:
        if hi:
            print('%4d  jar:*      no jar' % cid)
        else:
            n = min(ret, 64)
            tags = [tag(struct.unpack('>I', r[40 + 4 * k:44 + 4 * k])[0]) for k in range(n)]
            print('%4d  jar:*      %d entries: %s' % (cid, ret, ' '.join(tags)))
    else:
        print('%4d  jar:%-6s %s' % (cid, tag(arg0), 'absent' if hi else '0x%08X' % ret))
