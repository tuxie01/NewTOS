#!/usr/bin/env python3
"""apply-injector.py <tree> -- scratch only, never the shared tree.

QEMU raspi4b has no keyboard. This puts a key injector into the kernel's
blocking console read (kernel/src/host_env.rs read_key_loop, the loop
Cconin/Cnecin wait in; the older injectors sat only in pump_input, which does
not run there -- memory note injector-does-not-run-during-cconin). A key is
fed only while a program waits in that loop, after 3 s (600 ticks) of
waiting: make + break through crate::ikbd::feed, the path a USB key takes.
Keys: 'u' (menu: case u2), 'd' (drive D:), then spaces for every prompt.
Each feed is logged as "[wfp] key ..." on the serial line.
"""
import sys, pathlib
t = pathlib.Path(sys.argv[1])
he = t / "kernel/src/host_env.rs"
s = he.read_text()
old = """    let word = loop {
        // Drain USB-HID interrupt transfers (keyboard) → ikbd::feed →"""
assert s.count(old) == 1, "read_key_loop anchor"
s = s.replace(old, """    wfp_inject::enter(); // WFP-TEMP
    let word = loop {
        wfp_inject::step(); // WFP-TEMP
        // Drain USB-HID interrupt transfers (keyboard) → ikbd::feed →""")
s += '''
// WFP-TEMP BEGIN -- scripted keys in the blocking console read (scratch only)
pub mod wfp_inject {
    use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    static SINCE: AtomicU32 = AtomicU32::new(0);
    const KEYS: &[u8] = &[
        0x16, 0x20, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39,
        0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39, 0x39,
    ];
    pub fn enter() {
        SINCE.store(tos_api::bios::tick::hz_200(), Ordering::Relaxed);
    }
    pub fn step() {
        if tos_api::bdos::process::RUN.load(Ordering::Acquire).is_null() {
            return;
        }
        let i = NEXT.load(Ordering::Relaxed);
        if i >= KEYS.len() {
            return;
        }
        let now = tos_api::bios::tick::hz_200();
        if now.wrapping_sub(SINCE.load(Ordering::Relaxed)) < 600 {
            return;
        }
        crate::kprintln!("\\r\\n[wfp] key {} scancode {:#04x} at hz_200 {}", i, KEYS[i], now);
        crate::ikbd::feed(KEYS[i]);
        crate::ikbd::feed(KEYS[i] | 0x80);
        NEXT.store(i + 1, Ordering::Relaxed);
        SINCE.store(now, Ordering::Relaxed);
    }
}
// WFP-TEMP END
'''
he.write_text(s)
print("instrumented", he)
