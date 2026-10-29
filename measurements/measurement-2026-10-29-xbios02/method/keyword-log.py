#!/usr/bin/env python3
"""keyword-log.py <scratch-tree> -- XBIOS02 item 3, scratch only: one serial line per
translated key, "[xb02] key_word 0x<scancode><ascii>", in kernel/src/ikbd.rs key_word
(the word every console reader and the AES get: scancode << 8 | the active layout's
ASCII). Marked XB02-SCRATCH; applied to an APFS clone of the tree after
tools/inject-arm.py, never to the shared tree."""
import pathlib, sys
p = pathlib.Path(sys.argv[1]) / "kernel/src/ikbd.rs"
s = p.read_text()
old = "    (code as u16) << 8 | ascii as u16\n}\n"
assert s.count(old) == 1, "anchor"
new = ("    let w = (code as u16) << 8 | ascii as u16; // XB02-SCRATCH\n"
       "    crate::kprintln!(\"[xb02] key_word {:#06x}\", w); // XB02-SCRATCH\n"
       "    w // XB02-SCRATCH\n}\n")
p.write_text(s.replace(old, new))
print(f"keyword-log.py: {p} instrumented")
