//! INPUT01 — when the 200 Hz timer interrupt may run the input pump.
//!
//! # The defect
//!
//! The keyboard and mouse reach the system only when somebody drains the USB
//! host controller (`drivers::usb::poll` → `ikbd::feed` → console ring, VDI
//! cursor, AES button/motion handlers). Before INPUT01 that happened only
//! inside the synchronous trap handler — the AES wait loops, the blocking
//! console reads and `Bconstat` on an empty queue (`Host::pump_input`) — so a
//! program that computed, loaded codecs or read a file without calling the
//! AES froze the pointer (`TASK/INPUT01.md`; measured in
//! `measurements/measurement-2026-10-08-input01/`).
//!
//! EmuTOS moves the pointer from interrupts: the IKBD ACIA interrupt runs
//! `mouse_int` (`references/emutos/bios/aciavecs.S:249` `_int_acia` →
//! `ikbdsys` → `mousevec`), which updates the position and calls the AES's
//! `user_but`/`user_mot` vectors, and the cursor is redrawn in the VBL
//! (`vdi/vdi_mouse.c:602` `vblqueue[0] = vb_draw`, `:668-686`); the GEMDOS
//! trap does not change the interrupt mask (`bdos/rwa.S:158-204` `_enter`).
//!
//! # What NewTOS does — and where it stops
//!
//! The behaviour, not the hardware (altitude rule): the timer interrupt
//! drains the USB controller like `Host::pump_input` does — one report per
//! pass — but **only** when every condition of [`irq_pump_allowed`] holds,
//! and it decodes only pointer motion (next section):
//!
//! * **The interrupt was taken from EL0.** All kernel work on this single
//!   CPU runs either inside the synchronous exception handler, where IRQs are
//!   masked (that masking *is* the BDOS/AES/VDI mutual exclusion — see
//!   `tos_api::bios::tick::publish`), or as EL1 code with IRQs open (the
//!   boot path and the shell loop, the pump windows of `pump_input` and the
//!   blocking reads; until DESK43 U10 also the built-in desktop). An IRQ from
//!   EL0 therefore interrupts no kernel code: no VDI operation is drawing, no
//!   AES call is changing window or menu state, no `usb::poll` or USB
//!   transfer is in flight, no lock is held. The kernel state it sees is the
//!   state at the *entry* of any trap the same program could make next — the
//!   state in which `pump_input` already runs today. An IRQ from EL1 never
//!   pumps: the interrupted kernel code may be in the middle of an in-kernel
//!   `objc_draw` (the shell's `sh_draw`, a kernel-built alert).
//! * **A process is running.** The EL0 code belongs to a slot (always true
//!   at EL0; checked rather than assumed, so a pump never runs without a
//!   well-defined running process for the AES motion handler to act for).
//! * **No upcall is in flight** for that process (`upcall_depth == 0`). An
//!   EL0 upcall — a `kbdvec` hook, a `G_USERDEF` callout in the middle of an
//!   AES draw, a wheel vector, a signal-style handler — runs while a kernel
//!   operation is suspended underneath it; pumping then could re-enter the
//!   AES half-way through a call.
//! * **The AES fiber is idle.** Belt and braces for the same case: a
//!   `G_USERDEF` stall keeps the one AES fiber suspended mid-draw
//!   (`tos_api::aes::thread`).
//!
//! # Only the pointer moves from the interrupt
//!
//! The interrupt pump drains **one** USB report (as one trap-side pump does)
//! into a capture buffer ([`HeldBytes`]) instead of straight into the
//! decoder, and then looks at it ([`pure_motion`]):
//!
//! * nothing but relative-mouse packets that keep the current button state —
//!   pointer motion — is decoded at once: the cursor moves (its position at
//!   once, its image in the next frame, `crate::vbl`, TASK/V55), the AES sees
//!   the motion (`user_mot`, EmuTOS `mchange`);
//! * anything else — a button change, a wheel detent, a key make or break —
//!   stays **held**, and the interrupt drains nothing more (the following
//!   reports wait in the USB controller, as before INPUT01) until the next
//!   trap-side pump decodes the held report in place of a USB drain.
//!
//! So clicks, the double-click window, the wheel and every key byte reach
//! the AES, the console ring and a `kbdvec` hook at exactly the moment they
//! did before INPUT01 — inside a pump of the trap path, in order, none lost
//! and none decoded twice — and the key repeat runs only there, as before.
//! What INPUT01 changes is where the pointer moves, not when input events are
//! seen. (EmuTOS delivers button changes from the interrupt as well; doing
//! that here would change what a program that is busy when the user clicks
//! gets to see — a separate decision, not taken here.)
//!
//! What is deliberately **not** done: IRQs stay masked during kernel work,
//! and no pump runs at "safe points" inside long traps (`Pexec`, the SD
//! read loop, memory-protection resyncs). Such a point is inside a BDOS call
//! — the running process may be half-created (`Pexec`) and the AES may be
//! the caller (file selector, `rsrc_load`, `shel_write`) — and the AES motion
//! handler runs inline here (NewTOS has no fork queue). A pending timer
//! interrupt is taken on the very first instruction after the trap returns
//! to EL0, so the pump still runs between any two traps: the pointer stands
//! still for at most the longest single trap (measured, see the run).
//!
//! The decision logic and the one-at-a-time guard are pure so the host test
//! suite (`tools/xtask`, which includes this file by path) can run them.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// Where the input pump runs (boot argument `input=aes`,
/// `crate::bootargs::INPUT_AES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// Before INPUT01: only inside the trap handler (`Host::pump_input`, the
    /// blocking console reads).
    Aes = 0,
    /// INPUT01 default: additionally from the 200 Hz timer interrupt when it
    /// interrupted a program at EL0 ([`irq_pump_allowed`]).
    Irq = 1,
}

static MODE: AtomicU8 = AtomicU8::new(InputMode::Irq as u8);

/// Select the pump mode. Boot CPU, from the boot arguments, before the first
/// program runs.
pub fn set_mode(mode: InputMode) {
    MODE.store(mode as u8, Ordering::Relaxed);
}

/// The selected pump mode.
pub fn mode() -> InputMode {
    if MODE.load(Ordering::Relaxed) == InputMode::Aes as u8 {
        InputMode::Aes
    } else {
        InputMode::Irq
    }
}

/// What the timer interrupt knows about the context it interrupted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interrupted {
    /// `SPSR_EL1.M[3:0] == 0` — the interrupt was taken from EL0.
    pub from_el0: bool,
    /// The running process's slot, if any.
    pub process: bool,
    /// The running process's upcall nesting depth (0 = no upcall in flight).
    pub upcall_depth: u8,
    /// The AES fiber is suspended inside a `G_USERDEF` callout.
    pub aes_suspended: bool,
}

/// `true` when the timer interrupt may run the input pump — see the module
/// documentation for why each condition is needed.
pub fn irq_pump_allowed(mode: InputMode, ctx: Interrupted) -> bool {
    mode == InputMode::Irq
        && ctx.from_el0
        && ctx.process
        && ctx.upcall_depth == 0
        && !ctx.aes_suspended
}

/// `SPSR_EL1.M[3:0]` of an exception frame says EL0t (AArch64, EL0).
pub fn spsr_is_el0(spsr: u64) -> bool {
    spsr & 0xF == 0
}

/// Capacity of the held-report buffer. One USB boot report becomes at most
/// 20 IKBD bytes for a keyboard (4 modifier breaks and 4 makes, 6 key breaks
/// and 6 makes) and 3 + 2 × 128 for a mouse (packet + wheel detents, each a
/// make/break pair, `drivers/usb/hid.rs`); one capture holds one report.
pub const HELD_CAP: usize = 512;

/// The IKBD bytes of the report the interrupt pump drained but did not
/// decode, in arrival order. Touched only inside the pump (one at a time).
pub struct HeldBytes {
    buf: [u8; HELD_CAP],
    len: usize,
}

impl HeldBytes {
    /// An empty buffer.
    pub const fn new() -> Self {
        Self { buf: [0; HELD_CAP], len: 0 }
    }

    /// Append `byte`; `false` (byte not stored) when the buffer is full.
    pub fn push(&mut self, byte: u8) -> bool {
        if self.len >= HELD_CAP {
            return false;
        }
        self.buf[self.len] = byte;
        self.len += 1;
        true
    }

    /// The held bytes, oldest first.
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Nothing held.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Move the held bytes into `out` (oldest first) and empty the buffer;
    /// returns how many. `out` must hold [`HELD_CAP`] bytes.
    pub fn take_into(&mut self, out: &mut [u8; HELD_CAP]) -> usize {
        let n = self.len;
        out[..n].copy_from_slice(&self.buf[..n]);
        self.len = 0;
        n
    }
}

impl Default for HeldBytes {
    fn default() -> Self {
        Self::new()
    }
}

/// `true` when `bytes` consists only of complete IKBD relative-mouse packets
/// (`0xF8..=0xFB`, `dx`, `dy` — `docs/api/vdi/input/_mouse-packet.md`) whose
/// buttons equal `vdi_buttons`, the decoder's current button state (VDI
/// convention: bit 0 = left, bit 1 = right; the packet header carries them
/// swapped, bit 0 = right, bit 1 = left). Such a report only moves the
/// pointer. Anything else — another header, a key byte, a button change, a
/// truncated packet — is not pure motion. An empty report is (nothing to do).
pub fn pure_motion(bytes: &[u8], vdi_buttons: u8) -> bool {
    let mut i = 0;
    while i < bytes.len() {
        let h = bytes[i];
        if !(0xF8..=0xFB).contains(&h) || i + 3 > bytes.len() {
            return false;
        }
        let wire = h & 0x03;
        let vdi = ((wire >> 1) & 0x01) | ((wire & 0x01) << 1);
        if vdi != vdi_buttons & 0x03 {
            return false;
        }
        i += 3;
    }
    true
}

/// The one-at-a-time guard around the input pump (`usb::poll` + key
/// repeat): the USB driver keeps per-device ring producers and the decoder
/// keeps packet-assembly state, neither of which may be entered twice.
///
/// By construction the pump never overlaps itself — the interrupt pumps only
/// from EL0, every other pump runs in kernel code at EL1 — so the guard is
/// never found taken in a correct kernel. It exists so that a future change
/// that breaks that construction (an IRQ path that pumps at EL1, a pump that
/// opens IRQs) skips a pump instead of corrupting the controller state.
pub struct PumpLock {
    busy: AtomicBool,
}

impl PumpLock {
    /// A free guard.
    pub const fn new() -> Self {
        Self { busy: AtomicBool::new(false) }
    }

    /// Take the guard, or `None` if a pump is already running.
    pub fn try_enter(&self) -> Option<PumpGuard<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| PumpGuard { lock: self })
    }

    /// `true` while a pump holds the guard.
    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }
}

impl Default for PumpLock {
    fn default() -> Self {
        Self::new()
    }
}

/// Held for the duration of one pump; releases the guard when dropped.
pub struct PumpGuard<'a> {
    lock: &'a PumpLock,
}

impl Drop for PumpGuard<'_> {
    fn drop(&mut self) {
        self.lock.busy.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUIET_EL0: Interrupted = Interrupted {
        from_el0: true,
        process: true,
        upcall_depth: 0,
        aes_suspended: false,
    };

    #[test]
    fn irq_mode_pumps_only_from_a_quiet_el0_program() {
        assert!(irq_pump_allowed(InputMode::Irq, QUIET_EL0));
        // Interrupted kernel code (desktop, boot path, a pump window): never.
        assert!(!irq_pump_allowed(InputMode::Irq, Interrupted { from_el0: false, ..QUIET_EL0 }));
        // No process: nobody to route kbdvec bytes to.
        assert!(!irq_pump_allowed(InputMode::Irq, Interrupted { process: false, ..QUIET_EL0 }));
        // An upcall is running on top of a suspended kernel operation.
        assert!(!irq_pump_allowed(InputMode::Irq, Interrupted { upcall_depth: 1, ..QUIET_EL0 }));
        assert!(!irq_pump_allowed(InputMode::Irq, Interrupted { upcall_depth: 8, ..QUIET_EL0 }));
        // The AES fiber is suspended in a G_USERDEF callout.
        assert!(!irq_pump_allowed(InputMode::Irq, Interrupted { aes_suspended: true, ..QUIET_EL0 }));
    }

    #[test]
    fn aes_mode_never_pumps_from_the_interrupt() {
        assert!(!irq_pump_allowed(InputMode::Aes, QUIET_EL0));
        for from_el0 in [false, true] {
            for process in [false, true] {
                for upcall_depth in [0u8, 1] {
                    for aes_suspended in [false, true] {
                        let c = Interrupted { from_el0, process, upcall_depth, aes_suspended };
                        assert!(!irq_pump_allowed(InputMode::Aes, c));
                    }
                }
            }
        }
    }

    #[test]
    fn exactly_one_combination_pumps() {
        let mut n = 0;
        for from_el0 in [false, true] {
            for process in [false, true] {
                for upcall_depth in [0u8, 1, 2] {
                    for aes_suspended in [false, true] {
                        let c = Interrupted { from_el0, process, upcall_depth, aes_suspended };
                        if irq_pump_allowed(InputMode::Irq, c) {
                            n += 1;
                            assert_eq!(c, QUIET_EL0);
                        }
                    }
                }
            }
        }
        assert_eq!(n, 1);
    }

    #[test]
    fn spsr_el0_is_mode_field_zero() {
        assert!(spsr_is_el0(0x0000_0000));
        // DAIF/NZCV bits do not matter, only M[3:0].
        assert!(spsr_is_el0(0x6000_03C0));
        assert!(!spsr_is_el0(0x0000_0004)); // EL1t
        assert!(!spsr_is_el0(0x0000_0005)); // EL1h
        assert!(!spsr_is_el0(0x6000_0004));
    }

    #[test]
    fn mode_round_trips() {
        set_mode(InputMode::Aes);
        assert_eq!(mode(), InputMode::Aes);
        set_mode(InputMode::Irq);
        assert_eq!(mode(), InputMode::Irq);
    }

    #[test]
    fn motion_with_unchanged_buttons_is_pure() {
        assert!(pure_motion(&[], 0));
        assert!(pure_motion(&[0xF8, 2, 0], 0));
        assert!(pure_motion(&[0xF8, 2, 0, 0xF8, 0xFE, 3], 0));
        // Left held (VDI bit 0) is wire bit 1 = header 0xFA: a drag step.
        assert!(pure_motion(&[0xFA, 5, 5], 0x01));
        // Right held (VDI bit 1) is wire bit 0 = header 0xF9.
        assert!(pure_motion(&[0xF9, 0, 1], 0x02));
        assert!(pure_motion(&[0xFB, 1, 1], 0x03));
        // Only the two packet buttons count (extended buttons live in bits 2-4).
        assert!(pure_motion(&[0xF8, 1, 1], 0x1C));
    }

    #[test]
    fn button_changes_keys_and_odd_bytes_are_held() {
        // Press: the header's buttons differ from the decoder's state.
        assert!(!pure_motion(&[0xFA, 0, 0], 0));
        // Release.
        assert!(!pure_motion(&[0xF8, 0, 0], 0x01));
        // Left/right swapped is a change, not the same state.
        assert!(!pure_motion(&[0xF9, 0, 0], 0x01));
        // Motion then a press in the same report.
        assert!(!pure_motion(&[0xF8, 1, 1, 0xFA, 0, 0], 0));
        // A wheel detent (scancode make/break after the packet).
        assert!(!pure_motion(&[0xF8, 0, 0, 0x59, 0xD9], 0));
        // Key make / break.
        assert!(!pure_motion(&[0x1E], 0));
        assert!(!pure_motion(&[0x9E], 0));
        // Truncated packet.
        assert!(!pure_motion(&[0xF8, 1], 0));
        assert!(!pure_motion(&[0xF8], 0));
        // Other IKBD packets (absolute mouse, joystick, clock).
        assert!(!pure_motion(&[0xF7, 0, 0, 0, 0, 0], 0));
        assert!(!pure_motion(&[0xFF, 0], 0));
        assert!(!pure_motion(&[0xFC, 0, 0, 0, 0, 0, 0], 0));
    }

    #[test]
    fn held_bytes_keep_order_and_refuse_overflow() {
        let mut h = HeldBytes::new();
        assert!(h.is_empty());
        for b in [0xFAu8, 0, 0, 0x1E, 0x9E] {
            assert!(h.push(b));
        }
        assert_eq!(h.as_slice(), &[0xFA, 0, 0, 0x1E, 0x9E]);
        let mut out = [0u8; HELD_CAP];
        assert_eq!(h.take_into(&mut out), 5);
        assert_eq!(&out[..5], &[0xFA, 0, 0, 0x1E, 0x9E]);
        assert!(h.is_empty());
        // Fill to the brim: the next byte is refused (the caller then decodes
        // instead of dropping it), nothing already held is lost.
        for i in 0..HELD_CAP {
            assert!(h.push(i as u8));
        }
        assert!(!h.push(0xEE));
        assert_eq!(h.as_slice().len(), HELD_CAP);
        assert_eq!(h.as_slice()[HELD_CAP - 1], (HELD_CAP - 1) as u8);
    }

    #[test]
    fn biggest_reports_fit() {
        // A wheel mouse report with the largest detent count the boot report
        // can carry (i8 -128 → 128 detents, each a make/break pair).
        assert!(3 + 2 * 128 <= HELD_CAP);
        // A keyboard report: 4+4 modifier makes/breaks, 6+6 key makes/breaks.
        assert!(20 <= HELD_CAP);
    }

    #[test]
    fn pump_lock_is_exclusive_and_released_on_drop() {
        let l = PumpLock::new();
        assert!(!l.is_busy());
        {
            let g = l.try_enter();
            assert!(g.is_some());
            assert!(l.is_busy());
            // A second pump while the first one runs (the re-entrancy the
            // guard exists for) is refused, not queued.
            assert!(l.try_enter().is_none());
            assert!(l.try_enter().is_none());
        }
        assert!(!l.is_busy());
        // Free again: the next pump gets it.
        let g2 = l.try_enter();
        assert!(g2.is_some());
        drop(g2);
        assert!(!l.is_busy());
    }

    #[test]
    fn pump_lock_nested_attempt_does_not_release_the_outer_guard() {
        let l = PumpLock::new();
        let outer = l.try_enter().expect("free");
        // The refused inner attempt produces no guard, so nothing is dropped
        // that could clear the outer pump's flag.
        let inner = l.try_enter();
        assert!(inner.is_none());
        drop(inner);
        assert!(l.is_busy());
        drop(outer);
        assert!(!l.is_busy());
    }
}
