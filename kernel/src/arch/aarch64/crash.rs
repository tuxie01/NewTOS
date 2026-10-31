//! Program crash report on the console + the "press any key" wait — the
//! output half of EmuTOS `dopanic` (`references/emutos/bios/kprint.c:302-596`),
//! ARCH03 stage 0c. Spec: `docs/api/bios/_exception-crash.md`.
//!
//! The text itself is rendered by [`tos_api::bios::crash::write_report`]
//! (host-tested); this module sends it to the console the way `kcprintf`
//! does — the screen (`bconout2`, here [`crate::fbcon`] via the host) and the
//! debug log (here the UART, subject to the `serial=on` switch on
//! `board-rpi4`) — and waits for a key. Terminating the program is the
//! caller's job ([`super::exceptions`], `Pterm(-1)` like EmuTOS
//! `kill_program`).

use core::fmt;

use tos_api::bios::crash::{CrashReport, ProgramInfo};

/// `fmt::Write` over the console host, adding a CR before every LF like
/// EmuTOS' `cprintf_outc` (`bios/kprint.c:53-60`). Without a host (no console
/// installed yet) it falls back to the UART alone.
struct ConsoleWriter;

impl fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match tos_api::host::host() {
            Some(h) => {
                for &b in s.as_bytes() {
                    if b == b'\n' {
                        h.console_write_byte(b'\r');
                    }
                    h.console_write_byte(b);
                }
            }
            None => {
                use crate::hal::uart::Uart as _;
                for &b in s.as_bytes() {
                    if b == b'\n' {
                        crate::console::CONSOLE.write_byte(b'\r');
                    }
                    crate::console::CONSOLE.write_byte(b);
                }
            }
        }
        Ok(())
    }
}

/// The running program's basepage fields for the report, read through the
/// kernel alias of its basepage (`RUN`). `None` when no program runs.
pub fn current_program() -> Option<ProgramInfo> {
    let bp = tos_api::bdos::process::current_kva();
    if bp.is_null() {
        return None;
    }
    // SAFETY: `current_kva()` is the kernel (high-half) alias of the running
    // process's basepage; the fields are plain `LONG`s. Volatile because the
    // program may have rewritten its basepage.
    unsafe {
        Some(ProgramInfo {
            basepage: core::ptr::addr_of!((*bp).p_lowtpa).read_volatile() as u32,
            tbase: core::ptr::addr_of!((*bp).p_tbase).read_volatile() as u32,
            tlen: core::ptr::addr_of!((*bp).p_tlen).read_volatile() as u32,
            dbase: core::ptr::addr_of!((*bp).p_dbase).read_volatile() as u32,
            bbase: core::ptr::addr_of!((*bp).p_bbase).read_volatile() as u32,
        })
    }
}

/// Print the report, then — like `dopanic` (`kprint.c:581-585`) — eat any
/// pending keys, show the prompt, and block until a key arrives.
///
/// `slot` is the dying program's process slot. A program with its own
/// `kbdvec` hook (KEYS02) has its keyboard bytes queued for an EL0 upcall
/// that can no longer run; EmuTOS would still call the hook from the IKBD
/// interrupt. Here those bytes are handed to the default decoder
/// ([`crate::ikbd::kbd_int`], what every hook chains to) so the prompt can be
/// answered.
///
/// With the debug feature `crash-nowait` (TASK/PITEST01) it returns after
/// eating the pending keys, without the prompt and without the wait; the
/// caller then terminates the program at once.
#[cfg_attr(feature = "crash-nowait", allow(unreachable_code))]
pub fn report_and_wait(report: &CrashReport<'_>, slot: usize) {
    use fmt::Write as _;

    crate::kprintln!(
        "[crash] slot {} {} esr=0x{:08x} pc=0x{:x} far={:?}",
        slot,
        report.kind.message().unwrap_or("exception"),
        report.esr as u32,
        report.pc,
        report.far,
    );
    let mut w = ConsoleWriter;
    let _ = tos_api::bios::crash::write_report(&mut w, report);

    let Some(h) = tos_api::host::host() else {
        // No console input at all: nothing could answer the prompt.
        return;
    };
    // `while (bconstat2()) bconin2();`
    drain_hook_bytes(slot);
    while h.console_try_read_key().is_some() {
        drain_hook_bytes(slot);
    }
    #[cfg(feature = "crash-nowait")]
    {
        crate::kprintln!("[crash] slot {} crash-nowait, terminating with Pterm(-1)", slot);
        return;
    }
    // `cprintf(_("\n*** Press any key to continue ***"));`
    let _ = w.write_str("\n");
    for &b in tos_api::i18n::sys_string(tos_api::i18n::Sys::PressAnyKey) {
        h.console_write_byte(b);
    }
    // `bconin2();`
    loop {
        drain_hook_bytes(slot);
        if h.console_try_read_key().is_some() {
            break;
        }
        // Same wait as the blocking console read (`host_env::read_key_loop`):
        // board-rpi4 polls the USB keyboard, so spin; elsewhere sleep until
        // the next interrupt (the PL011 RX IRQ, the 200 Hz tick).
        #[cfg(feature = "board-rpi4")]
        core::hint::spin_loop();
        #[cfg(not(feature = "board-rpi4"))]
        // SAFETY: `wfi` only waits; a pending (even masked) IRQ wakes it.
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
    // `cprintf("\n");`
    let _ = w.write_str("\n");
    crate::kprintln!("[crash] slot {} key pressed, terminating with Pterm(-1)", slot);
}

/// Decode the bytes queued for `slot`'s `kbdvec` hook with the default
/// handler (see [`report_and_wait`]).
fn drain_hook_bytes(slot: usize) {
    while let Some(b) = tos_api::xbios::kbdvecs::pop(slot) {
        crate::ikbd::kbd_int(b);
    }
}
