#!/usr/bin/env python3
"""scan-tables.py <sdk-dir> -- TASK/A142 inventory: every C/header file under the SDK sources with a table of bytes (`char`/`BYTE`/`UBYTE`/`_UBYTE`/
`uint8`..) initialised with numbers, or with a word table of raster data; one line per file with counts, for reading by hand. Skips build output and
the -m68k copies."""
import pathlib, re, sys
root = pathlib.Path(sys.argv[1])
byte_t = re.compile(r"\b(?:unsigned\s+char|signed\s+char|char|BYTE|UBYTE|_UBYTE|uint8_t|uint8|u_char|UCHAR|uchar)\s+(\w+)\s*\[[^\]]*\]\s*=\s*\{\s*(?:0[xX][0-9a-fA-F]+|\d+)\s*,", re.S)
skip = ("/build/", "-m68k/", "/.git/", "/libc/mintlib", "/zlib/", "/libjpeg/", "/libpng/", "/giflib/", "/libexif/", "/libm/", "/compiler-rt/")
for p in sorted(root.rglob("*")):
    if p.suffix.lower() not in (".c", ".h", ".rsh", ".hrd", ".ic", ".i", ".inc", ".icn", ".img") or not p.is_file():
        continue
    if any(s in str(p) for s in skip):
        continue
    t = p.read_text(errors="replace")
    names = byte_t.findall(t)
    if names:
        print(f"{p.relative_to(root)}\t{len(names)}\t{' '.join(names[:6])}")
