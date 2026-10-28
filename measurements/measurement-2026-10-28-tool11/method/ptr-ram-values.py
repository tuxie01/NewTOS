#!/usr/bin/env python3
"""ptr-ram-values.py -- TOOL11: every PTR_RAM case of a call table, with the
64-bit return (ret_hi:ret) of each dump given and the new rule's verdict
(non-null, >= 0x400, < 0x80000000, even; tosdump/core/record.c,
td_ptr_ram_violation).  Read only.

usage: ptr-ram-values.py <table.tsv> <label>=<dump.BIN>...

Record layout (tosdump/core/tosdump.h): 32-byte header, then 320-byte
records; case id u16 at 0, ret u32 at 36, ret_hi u32 at 296, big-endian.
"""
import struct
import sys


def verdict(v):
    if v == 0:
        return "is null"
    if v >= 0x80000000:
        return "not < 0x80000000"
    if v < 0x400:
        return "below 0x400"
    if v & 1:
        return "is odd"
    return "ok"


def ptr_cases(tsv):
    out = {}
    with open(tsv, encoding="latin-1") as f:
        for line_no, line in enumerate(f, 1):
            if line.startswith("#") or not line.strip():
                continue
            cols = line.rstrip("\n").split("\t")
            if len(cols) >= 3 and cols[2] == "PTR_RAM":
                out[line_no] = cols[4] if len(cols) > 4 else ""
    return out


def rets(path):
    data = open(path, "rb").read()
    out = {}
    for off in range(32, len(data) - 319, 320):
        rec = data[off:off + 320]
        cid = struct.unpack(">H", rec[0:2])[0]
        lo = struct.unpack(">I", rec[36:40])[0]
        hi = struct.unpack(">I", rec[296:300])[0]
        out[cid] = (hi << 32) | lo
    return out


def main():
    tsv = sys.argv[1]
    dumps = [a.split("=", 1) for a in sys.argv[2:]]
    cases = ptr_cases(tsv)
    vals = [(label, rets(path)) for label, path in dumps]
    for cid, comment in cases.items():
        print(f"case {cid}  {comment}")
        for label, r in vals:
            if cid not in r:
                print(f"    {label:10s} (no record)")
                continue
            v = r[cid]
            print(f"    {label:10s} 0x{v:016X}  {verdict(v)}")


if __name__ == "__main__":
    main()
