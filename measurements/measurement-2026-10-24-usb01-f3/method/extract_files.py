#!/usr/bin/env python3
"""extract_files.py SERIAL_LOG OUTDIR TAG -- cut the files USBSNAP printed between "=====USBSNAP FILE BEGIN <path>=====" and
"=====USBSNAP FILE END=====" (measurement build usbsnap-virt.c, qemu-virt has no drive) out of a serial log into
OUTDIR/TAG-<name>; then check every file: CRLF line ends only, ASCII only, <= 78 columns, the section headers
"=== 0. FILE ===" .. "=== 6. REPORTS ===" and "=== END ===" in order. Exit 1 on a failed check."""
import os, re, sys
log = open(sys.argv[1], 'rb').read()
out, tag = sys.argv[2], sys.argv[3]
os.makedirs(out, exist_ok=True)
bad = 0
for m in re.finditer(rb"=====USBSNAP FILE BEGIN (.*?)=====\r?\n(.*?)=====USBSNAP FILE END=====", log, re.S):
    name = m.group(1).decode().split("\\")[-1]
    body = m.group(2)
    p = os.path.join(out, f"{tag}-{name}")
    open(p, 'wb').write(body)
    lines = body.split(b"\r\n")
    probs = []
    if lines[-1] != b"": probs.append("does not end with CRLF")
    if any(b"\n" in l or b"\r" in l for l in lines): probs.append("bare CR or LF")
    if any(c > 0x7e or (c < 0x20) for l in lines for c in l): probs.append("non-ASCII / control byte")
    w = max(len(l) for l in lines)
    if w > 78: probs.append(f"line of {w} columns")
    heads = [l.decode() for l in lines if l.startswith(b"=== ")]
    want = ["=== 0. FILE ===", "=== 1. IDENTITY ===", "=== 2. POSITION ===", "=== 3. DRIVER DECISION ===",
            "=== 4. RAW DESCRIPTORS (hex, offset: 16 bytes) ===", "=== 5. DECODED DESCRIPTORS ===", "=== 6. REPORTS ===", "=== END ==="]
    if heads != want: probs.append(f"sections {heads}")
    print(f"{tag}-{name}: {len(body)} bytes, {len(lines)-1} lines, widest {w}: {'OK' if not probs else 'FAIL ' + '; '.join(probs)}")
    bad += bool(probs)
sys.exit(1 if bad else 0)
