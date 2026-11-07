#!/usr/bin/env python3
"""convert-inline-rsh.py <inline.rsh> -- TASK/A142, used once on sdk/cflib/overlay/inline.rsh: every `static _UBYTE inline_*[] = { bytes }` raster table
(ORCS writes them in m68k order) becomes a table of 16-bit words with the same numbers read big-endian, line for line, 16 bytes -> 8 words.
Nothing else in the file changes. ORCS itself is not in the build: inline.rsh is upstream's checked-in output, this overlay a copy of it."""
import re, sys
p = sys.argv[1]; s = open(p).read()
n = 0
def table(m):
    global n
    out = []
    for line in m.group(2).strip("\n").split("\n"):
        vals = [int(x, 16) for x in re.findall(r"0x[0-9A-Fa-f]{2}", line)]
        assert len(vals) % 2 == 0, line
        out.append(", ".join(f"0x{(a << 8) | b:04X}" for a, b in zip(vals[::2], vals[1::2])) + ("," if line.rstrip().endswith(",") else ""))
    n += 1
    return f"static unsigned short {m.group(1)}[] = {{\n" + "\n".join(out) + "};"
s = re.sub(r"static _UBYTE (inline_\w+)\[\] = \{\n(.*?)\};", table, s, flags=re.S)
assert n == 44, n
marker = "like the m68k-GNU case so it, too, skips portab.h. Arch-detection fix only.\n */\n"
assert s.count(marker) == 1
s = s.replace(marker, """like the m68k-GNU case so it, too, skips portab.h. Arch-detection fix only.
 *
 * NewTOS overlay (TASK/A142): the 44 image tables below are words, not bytes.
 * ORCS writes them as `_UBYTE` tables in m68k order, cast to short * further
 * down; VDI and AES read an MFDB or a BITBLK as native 16-bit words with bit 15
 * the leftmost pixel, so on little-endian AArch64 the two 8-pixel halves of
 * every word traded places (and a table could lie on an odd address). As words
 * the same numbers are right in either byte order. Values unchanged: the host
 * test sdk/cflib/tests/inline-rsh-host.sh compares them with upstream's bytes.
 */
""")
open(p, "w").write(s)
print("tables", n)
