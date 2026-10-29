#!/usr/bin/env python3
"""find-idt.py <ram.bin> <QED.NAP> <sym-offset-hex>

Reads QED's `_idt` (src/global.c, `static long _idt`) out of a physical RAM
dump of QEMU raspi4b taken with `pmemsave 0 <size>` while QED runs.

NewTOS maps the TPA VA = PA (tos-api/src/bdos/process/arena.rs, module doc),
and the loader places link VA v of the PIE at B + v, B = TPA base
(tos-api/src/bdos/process/loader.rs, populate_and_map). So:
  1. take the first PT_LOAD segment with PF_X of QED.NAP and search its first
     256 file bytes (code: R_AARCH64_RELATIVE never patches .text) in the dump;
  2. every hit at PA h gives a candidate B = h - p_vaddr; keep a candidate
     only if the BASEPAGE at B says p_lowtpa == B (first field, a 4-byte
     Long, tos-types/src/basepage.rs; written by the loader's
     populate_basepage) -- a copy of the file in a sector cache does not;
  3. read the 8-byte little-endian long at B + <sym-offset>.
Prints every hit and the verdict; exit 0 iff exactly one candidate passed.
"""
import os
import struct
import sys


def main():
    ram_path, elf_path, sym_hex = sys.argv[1], sys.argv[2], sys.argv[3]
    sym = int(sym_hex, 16)
    elf = open(elf_path, "rb").read()
    assert elf[:4] == b"\x7fELF" and elf[4] == 2 and elf[5] == 1, "ELF64 LE expected"
    e_phoff, = struct.unpack_from("<Q", elf, 0x20)
    e_phentsize, e_phnum = struct.unpack_from("<HH", elf, 0x36)
    text = None
    for i in range(e_phnum):
        p_type, p_flags, p_offset, p_vaddr, _pa, p_filesz, _ms, _al = struct.unpack_from(
            "<IIQQQQQQ", elf, e_phoff + i * e_phentsize)
        if p_type == 1 and p_flags & 1 and p_filesz >= 256:
            text = (p_offset, p_vaddr)
            break
    assert text, "no executable PT_LOAD"
    off, vaddr = text
    needle = elf[off:off + 256]
    print(f"elf {os.path.basename(elf_path)}: exec PT_LOAD offset 0x{off:x} vaddr 0x{vaddr:x}; _idt link VA 0x{sym:x}")
    ram = open(ram_path, "rb").read()
    print(f"ram dump {os.path.basename(ram_path)}: {len(ram)} bytes")
    good = []
    pos = ram.find(needle)
    while pos >= 0:
        base = pos - vaddr
        lowtpa = struct.unpack_from("<I", ram, base)[0] if 0 <= base <= len(ram) - 4 else None
        ok = lowtpa == (base & 0xFFFFFFFF)
        print(f"  hit PA 0x{pos:x} -> B 0x{base:x}; BASEPAGE p_lowtpa "
              f"{'n/a' if lowtpa is None else hex(lowtpa)} -> {'TPA' if ok else 'not a TPA'}")
        if ok:
            good.append(base)
        pos = ram.find(needle, pos + 1)
    if len(good) != 1:
        print(f"verdict: {len(good)} TPA candidates -- no value read")
        return 1
    base = good[0]
    idt, = struct.unpack_from("<Q", ram, base + sym)
    print(f"QED _idt @ PA 0x{base + sym:x} = 0x{idt:016x}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
