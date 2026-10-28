"""normalise-fat-times.py BEFORE AFTER -- compare two FAT16 partition files with the time
stamps of every root directory entry zeroed (bytes 13..19 create tenth/time/date and access
date, 22..25 write time/date). Prints the result; exit 0 when equal after normalising.
Root directory only: the test partition of TOOL07 holds NEWDESK.INF in its root and nothing
else; a difference anywhere else stays a difference."""
import struct
import sys


def normalise(path):
    data = bytearray(open(path, 'rb').read())
    bps, = struct.unpack_from('<H', data, 11)
    res, = struct.unpack_from('<H', data, 14)
    nfats = data[16]
    nroot, = struct.unpack_from('<H', data, 17)
    spf, = struct.unpack_from('<H', data, 22)
    start = (res + nfats * spf) * bps
    for off in range(start, start + nroot * 32, 32):
        if data[off] == 0:
            break
        for i in list(range(13, 20)) + list(range(22, 26)):
            data[off + i] = 0
    return bytes(data)


b, a = normalise(sys.argv[1]), normalise(sys.argv[2])
if b == a:
    print("testpart: identical after normalising root directory time stamps")
    sys.exit(0)
diff = sorted({i // 512 for i in range(len(b)) if b[i] != a[i]})
print("testpart: still different after normalising, sectors:", diff[:50])
sys.exit(1)
