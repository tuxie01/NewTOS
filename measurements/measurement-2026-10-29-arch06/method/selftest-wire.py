#!/usr/bin/env python3
"""selftest-wire.py <tree> -- SCRATCH ONLY: insert selftest_cookie::run() after `tos_api::aes::init::init();`
in <tree>/kernel/src/lib.rs (never the shared tree). Pattern of measurement-2026-10-21-gh1-reg/method/selftest-wire.py,
cookie test only."""
import sys
p = sys.argv[1] + "/kernel/src/lib.rs"; s = open(p).read()
anchor = "    tos_api::aes::init::init();\n"
assert s.count(anchor) == 1
blk = '    // SCRATCH SELFTEST WIRING (ARCH06)\n    kprintln!("[ST] >> cookie");\n    selftest_cookie::run();\n    kprintln!("[ST] << cookie");\n'
open(p, "w").write(s.replace(anchor, anchor + blk))
