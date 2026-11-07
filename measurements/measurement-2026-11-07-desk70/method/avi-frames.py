#!/usr/bin/env python3
"""avi-frames.py <file.avi> [--dump <dir> frame ...] -- walk the frames of a Hatari PNG AVI (Hatari was SIGKILLed: no index,
so the movi chunks are read sequentially; Hatari writes no odd-size padding). Every video chunk is followed by one audio
chunk holding the samples of that frame's emulated time; their sum (stereo 16 bit, rate from the audio strh/strf) is the
EMULATED clock, independent of how fast the host encoded the frames.
Without --dump: print the header and one line per run of identical frames (the top 480 lines; Hatari's status bar is
outside the comparison), with start and length in emulated seconds. With --dump: write those frames as PNG (640x480 crop)."""
import sys, io, struct, hashlib, os
from PIL import Image
path = sys.argv[1]; dump = None; want = set()
if len(sys.argv) > 2 and sys.argv[2] == '--dump':
    dump = sys.argv[3]; want = {int(x) for x in sys.argv[4:]}; os.makedirs(dump, exist_ok=True)
data = open(path, 'rb').read()
assert data[:4] == b'RIFF'
i = data.find(b'auds'); j = data.find(b'strf', i)
wfmt, chans, srate, brate, align, bits = struct.unpack('<HHIIHH', data[j+8:j+24])
m = data.find(b'movi'); p = m + 4; n = 0; runs = []; prev = None; start = 0; end = len(data); tsamples = 0; tstart = 0; sz = None
times = []
if not dump:
    print(f"audio clock: {srate} Hz, {chans} ch, {bits} bit, {brate} bytes/s; file {len(data)} bytes")
while p + 8 <= end:
    tag = data[p:p+4]; size = struct.unpack('<I', data[p+4:p+8])[0]
    if tag in (b'LIST', b'RIFF'):
        p += 12; continue
    if tag == b'idx1' or tag == b'indx' or p + 8 + size > end:
        break
    if tag == b'00dc':
        chunk = data[p+8:p+8+size]
        if dump:
            if n in want:
                Image.open(io.BytesIO(chunk)).convert('RGB').crop((0,0,640,480)).save(f"{dump}/frame-{n:06d}.png")
        else:
            img = Image.open(io.BytesIO(chunk)).convert('RGB'); w, h = img.size
            h2 = hashlib.sha1(img.crop((0, 0, w, min(h, 480))).tobytes()).hexdigest()[:12]
            if h2 != prev:
                if prev is not None: runs.append((start, n-1, prev, sz, tstart, tsamples))
                prev, start, sz, tstart = h2, n, (w, h), tsamples
        n += 1
    elif tag == b'01wb':
        tsamples += size
    p += 8 + size
if not dump:
    runs.append((start, n-1, prev, sz, tstart, tsamples))
    print(f"frames {n}, emulated {tsamples/brate:.3f} s, runs of identical frames {len(runs)}")
    print(f"{'first':>7}..{'last':>7} {'frames':>6} {'t_start_s':>10} {'len_s':>9}  sha1(top 480 lines)  size")
    for a, b, h, s, t0, t1 in runs:
        print(f"{a:7d}..{b:7d} {b-a+1:6d} {t0/brate:10.3f} {(t1-t0)/brate:9.3f}  {h} {s[0]}x{s[1]}")
