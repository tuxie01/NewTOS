#!/usr/bin/env python3
"""qmpshot.py <host:port> <out.png> -- one QMP screendump (QMP on TCP), written as PNG."""
import json, socket, struct, sys, zlib, os, tempfile

def qmp(sock_path, ppm):
    host, port = sock_path.rsplit(":", 1)
    s = socket.create_connection((host, int(port)))
    f = s.makefile("rwb")
    f.readline()
    for cmd in ({"execute": "qmp_capabilities"},
                {"execute": "screendump", "arguments": {"filename": ppm}}):
        f.write((json.dumps(cmd) + "\n").encode()); f.flush()
        while True:
            r = json.loads(f.readline())
            if "return" in r or "error" in r:
                if "error" in r:
                    raise SystemExit(r)
                break
    s.close()

def ppm_to_png(ppm, png):
    d = open(ppm, "rb").read()
    parts = d.split(maxsplit=4)
    assert parts[0] == b"P6"
    w, h, mx = int(parts[1]), int(parts[2]), int(parts[3])
    px = parts[4]
    raw = b"".join(b"\0" + px[y * w * 3:(y + 1) * w * 3] for y in range(h))
    def chunk(t, b):
        return struct.pack(">I", len(b)) + t + b + struct.pack(">I", zlib.crc32(t + b) & 0xffffffff)
    open(png, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                          + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))

sock, out = sys.argv[1], sys.argv[2]
fd, ppm = tempfile.mkstemp(suffix=".ppm", dir=os.path.dirname(os.path.abspath(out)))
os.close(fd)
qmp(sock, ppm)
ppm_to_png(ppm, out)
os.unlink(ppm)
