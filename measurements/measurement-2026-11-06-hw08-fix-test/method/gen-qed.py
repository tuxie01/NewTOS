#!/usr/bin/env python3 -I
"""gen-qed.py <script-out> <shots-out> <lines> : IKBD script for QED.NAP (hw08 test): type <lines> lines of text, ^S, type the file name QT.TXT
into the file selector, Return. Ticks = 200 Hz after 'up:QED armed'. ST scancodes."""
import sys
out, shots_out, nl = sys.argv[1], sys.argv[2], int(sys.argv[3])
SC = {'a':0x1e,'b':0x30,'c':0x2e,'d':0x20,'e':0x12,'f':0x21,'g':0x22,'h':0x23,'i':0x17,'j':0x24,'k':0x25,'l':0x26,'m':0x32,'n':0x31,'o':0x18,'p':0x19,'q':0x10,'r':0x13,'s':0x1f,'t':0x14,'u':0x16,'v':0x2f,'w':0x11,'x':0x2d,'y':0x15,'z':0x2c,' ':0x39,'.':0x34,'\n':0x1c}
L = []; S = []; t = 400
for c in (0x1d, 0x31, 0xb1, 0x9d): L.append((t, c)); t += 4   # ^N new file
S.append((1400, 'new'))
t = 1600
def typ(ch):
    global t
    L.append((t, SC[ch])); L.append((t + 8, SC[ch] | 0x80)); t += 16
for i in range(nl):
    for ch in "abcdefghij klmnopqrst\n": typ(ch)
S.append((t + 300, "typed"))
t += 400
for c in (0x1d, 0x1f, 0x9f, 0x9d): L.append((t, c)); t += 4
S.append((t + 500, "fsel"))
t += 1000
for ch in "qt.txt": typ(ch)
S.append((t + 200, "name-typed"))
t += 400
L.append((t, 0x1c)); L.append((t + 4, 0x9c)); t += 8
S.append((t + 600, "after-save")); S.append((t + 2000, "end"))
open(out, "w").write("".join("%d 0x%02x\n" % (tt, c) for tt, c in L))
open(shots_out, "w").write("".join("%d %s\n" % (tt, n) for tt, n in sorted(S)))
