#!/usr/bin/env python3
"""wire.py <tree> -- A115, SCRATCH ONLY: insert the three scheduler selftests (selftest_sleepwake, selftest_timer_restart,
selftest_sleepwake_switch -- none is wired into a boot path) after `tos_api::aes::init::init();` in <tree>/kernel/src/lib.rs
(never the shared tree). Pattern of measurements/measurement-2026-10-21-gh1-reg/method/selftest-wire.py."""
import sys
p = sys.argv[1] + "/kernel/src/lib.rs"; s = open(p).read()
anchor = "    tos_api::aes::init::init();\n"
assert s.count(anchor) == 1
calls = [("sleepwake", "selftest_sleepwake::run()"), ("timer-restart", "selftest_timer_restart::run()"),
         ("sleepwake-switch", "selftest_sleepwake_switch::run()")]
blk = "".join('    kprintln!("[ST] >> %s");\n    %s;\n    kprintln!("[ST] << %s");\n' % (n, c, n) for n, c in calls)
open(p, "w").write(s.replace(anchor, anchor + "    // SCRATCH SELFTEST WIRING (A115)\n" + blk))
