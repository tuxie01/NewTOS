#!/usr/bin/env python3
"""einvfn-patch.py <scratch-tree> -- run `einvfn` only: the scratch kernel
answers Ssystem(2019) with EINVFN, as a NewTOS kernel before SYSINFO01 (or any
kernel without the mode) would, so the About box's failure path is visible.
Applied to the scratch copy after build-kernel.sh's first build; the shared
tree is never touched."""
import pathlib, sys
p = pathlib.Path(sys.argv[1]) / "tos-api/src/gemdos/ssystem.rs"
s = p.read_text()
old = "        S_NEWTOS_KVERSION => s_newtos_kversion(args),\n"
assert s.count(old) == 1, "anchor"
p.write_text(s.replace(old, "        S_NEWTOS_KVERSION => { let _ = s_newtos_kversion; EINVFN } // REL07-EINVFN-TEMP\n"))
