#!/usr/bin/env python3
"""check_files.py FILE... -- the §4.4 format checks of extract_files.py for files copied off a card."""
import sys
bad = 0
want = ["=== 0. FILE ===", "=== 1. IDENTITY ===", "=== 2. POSITION ===", "=== 3. DRIVER DECISION ===",
        "=== 4. RAW DESCRIPTORS (hex, offset: 16 bytes) ===", "=== 5. DECODED DESCRIPTORS ===", "=== 6. REPORTS ===", "=== END ==="]
for p in sys.argv[1:]:
    body = open(p, "rb").read(); lines = body.split(b"\r\n"); probs = []
    if lines[-1] != b"": probs.append("does not end with CRLF")
    if any(b"\n" in l or b"\r" in l for l in lines): probs.append("bare CR or LF")
    if any(c > 0x7e or c < 0x20 for l in lines for c in l): probs.append("non-ASCII / control byte")
    w = max(len(l) for l in lines)
    if w > 78: probs.append(f"line of {w} columns")
    if [l.decode() for l in lines if l.startswith(b"=== ")] != want: probs.append("section headers")
    print(f"{p.split('/')[-1]}: {len(body)} bytes, {len(lines)-1} lines, widest {w}: {'OK' if not probs else 'FAIL ' + '; '.join(probs)}")
    bad += bool(probs)
sys.exit(1 if bad else 0)
