//! Board-selected console alias and the console key queue.
//!
//! `CONSOLE` resolves to the PL011 driver instance for the active board so
//! callers can `use kernel::console::CONSOLE;` without per-board `cfg`s.
//!
//! The key queue is the **IKBD IOREC ring** — EmuTOS' `ikbdiorec`
//! (`bios/ikbd.c:157-173`), the queue `bconstat2`/`bconin2` read — and lives
//! in the IOREC page `Iorec(1)` returns (`tos_api::xbios::iorec`, XBIOS01
//! E1): one ring for every consumer, a program can flush it by writing
//! `head = tail`. It holds 32-bit entries `shifty << 24 | scancode << 16 |
//! ascii`, the shift state taken at push time (E2); 63 of them, a typing
//! burst beyond that drops keys as on EmuTOS. Only the IKBD decoder fills it
//! ([`push_rx`], fed from the USB-HID keyboard) — on both boards, as EmuTOS'
//! default `CONF_SERIAL_CONSOLE = 0` profile of the pinned build.
//!
//! A byte received on the serial line never enters the key queue; where it
//! goes is decided per board in [`serial_rx_byte`]: on qemu-virt into the
//! input ring of `Iorec(0)` (the RS-232 device, EmuTOS `push_serial_iorec`),
//! read by the AUX driver [`AuxSerial`] (`Bconstat(1)`/`Bconin(1)`); on real
//! Pi-4 it is dropped (UART0 is the debug console there).
//!
//! The ring is program data: `tos_api::xbios::iorec` checks it before every
//! use and repairs what a program broke; [`report_iorec_repairs`] prints the
//! warning for each repair, once per process.

use core::sync::atomic::{AtomicBool, Ordering};

use tos_api::xbios::iorec;

use crate::hal::uart::Uart;

#[cfg(feature = "board-rpi4")]
use crate::drivers::bcm2711::pl011::CONSOLE as UART0;

#[cfg(feature = "board-qemu-virt")]
use crate::drivers::qemu_virt::pl011::CONSOLE as UART0;

// ---------------------------------------------------------------------------
// Serial output switch (SER01).
// ---------------------------------------------------------------------------

/// Whether UART0 transmits by default, before the boot arguments are read.
///
/// `board-rpi4`: **off**. Every kernel message and every byte a program
/// writes to the console used to go out synchronously; with the TX FIFO full
/// `write_byte` busy-waits (~87 us per byte at 115200 baud) and the whole
/// system stands still — for a cable that is normally not attached. The user
/// switches it on without a rebuild by putting `serial=on` into `cmdline.txt`
/// (user decision 2026-10-02, `TASK/SER01.md`); see [`configure_serial_tx`].
#[cfg(feature = "board-rpi4")]
const SERIAL_TX_DEFAULT: bool = false;

/// `board-qemu-virt`: **on**, unconditionally. The board exists only as a
/// QEMU machine, its serial log is the measurement channel of every
/// qemu-virt run, and there is no real hardware whose speed the UART could
/// cost; a switch would only add a way to lose the log.
#[cfg(feature = "board-qemu-virt")]
const SERIAL_TX_DEFAULT: bool = true;

static SERIAL_TX: AtomicBool = AtomicBool::new(SERIAL_TX_DEFAULT);

/// `true` while console bytes are actually sent to UART0.
#[inline]
pub fn serial_tx_enabled() -> bool {
    SERIAL_TX.load(Ordering::Relaxed)
}

/// Switch UART0 transmission on for good — the panic handler and the
/// fatal-exception path call this before they print, so their report always
/// leaves the machine whatever the boot arguments said. They never return,
/// so "temporarily" and "for good" are the same thing here.
pub fn force_serial_tx_on() {
    SERIAL_TX.store(true, Ordering::Relaxed);
}

/// Decide the serial switch from the boot arguments. `board-rpi4`: asks the
/// firmware for its command line (the text of `cmdline.txt`, or QEMU's
/// `-append`) through the mailbox tag "Get command line"
/// (`drivers::bcm2711::mailbox::with_command_line`) and switches TX on if it
/// contains the whole token `serial=on` ([`crate::bootargs::SERIAL_ON`]).
/// Without that token — or without any answer — the board default (off)
/// stays.
///
/// Must run right after `CONSOLE.init()` and before the first `kprint!`, so
/// nothing is sent before the decision. `entry_el` is the exception level
/// the boot stub was entered at (`1`..`3`): on this board the stub no longer
/// writes its pre-MMU heartbeat `RB<EL>` straight to the data register
/// (that byte would bypass the switch), so it is replayed here, through the
/// console, when output is on. Returns `true` if output was switched on.
#[cfg(feature = "board-rpi4")]
pub fn configure_serial_tx(entry_el: u64) -> bool {
    let on = crate::drivers::bcm2711::mailbox::with_command_line(|args| {
        crate::bootargs::has_token(args, crate::bootargs::SERIAL_ON)
    })
    .unwrap_or(false);
    if on {
        SERIAL_TX.store(true, Ordering::Relaxed);
        let el = b'0' + (entry_el & 3) as u8;
        for b in [b'R', b'B', el, b'\r', b'\n'] {
            CONSOLE.write_byte(b);
        }
    }
    on
}

/// The kernel console: UART0 of the active board behind the serial output
/// switch. Every transmit path goes through here — `kprint!`/`kprintln!`
/// (`hal::uart::UartWriter`), program console output (`host_env`), the panic
/// handler and the self-tests — so one check covers them all. Receive,
/// interrupt and init calls pass straight through: the switch is TX-only.
pub struct SerialConsole;

/// Board-selected console instance.
pub static CONSOLE: SerialConsole = SerialConsole;

impl Uart for SerialConsole {
    fn init(&self) {
        UART0.init();
    }

    #[inline]
    fn write_byte(&self, byte: u8) {
        if serial_tx_enabled() {
            UART0.write_byte(byte);
        }
    }

    fn read_byte(&self) -> Option<u8> {
        UART0.read_byte()
    }

    fn enable_rx_interrupt(&self) {
        UART0.enable_rx_interrupt();
    }

    fn clear_rx_interrupt(&self) {
        UART0.clear_rx_interrupt();
    }
}

/// Drain the PL011 RX FIFO and acknowledge the RX-side interrupt sources.
/// Called from the GIC IRQ handler when the PL011 INTID is signalled — the
/// receive interrupt EmuTOS handles in `mfp_rs232_rx_interrupt_handler`
/// (`bios/serport.c:350-359`) and, on the Falcon, `scc_rx_interrupt_handler`
/// (`:647-683`). Where the received bytes go is the board's decision —
/// [`serial_rx_byte`].
pub fn pl011_rx_drain() {
    while let Some(b) = CONSOLE.read_byte() {
        serial_rx_byte(b);
    }
    CONSOLE.clear_rx_interrupt();
}

/// Route one byte received on the serial line (qemu-virt): into the input
/// ring of device 0, the record `Iorec(0)` returns — EmuTOS'
/// `push_serial_iorec` (`bios/serport.c:331-343`), called by the MFP receive
/// interrupt in the default `CONF_SERIAL_CONSOLE = 0` profile
/// (`:350-359`; the pinned build has it 0,
/// `references/emutos/include/config.h:1422-1429`), and the push of the
/// Falcon's `scc_rx_interrupt_handler` (`:670-678`). A full ring drops the
/// byte. The byte never enters the key queue: console input on this board is
/// the USB-HID keyboard (`qemu-xhci` + `usb-kbd`), as on real Pi-4. AUX reads
/// the ring through [`AuxSerial`] (XBIOS15; oracle table
/// `docs/audits/2026-10-29-xbios15-serial-rx.md`, SERRX-001).
///
/// Called from the IRQ handler: the repair warning of a broken record (if
/// any) is printed after the next trap ([`check_iorec_after_trap`]).
#[cfg(feature = "board-qemu-virt")]
fn serial_rx_byte(b: u8) {
    iorec::push_serial(b);
}

/// Real Pi-4 has its own screen and its own keyboard (USB-HID), so it is
/// EmuTOS' default `CONF_SERIAL_CONSOLE = 0` profile
/// (`references/emutos/include/config.h:1427-1429`) — the one every machine
/// with a console of its own is built with. There the RS-232 receiver pushes
/// into `iorec1`, the **RS-232 device** (BIOS handle 1,
/// `bios/serport.c:331-343` `push_serial_iorec`), and *never* into the IKBD
/// queue; `push_ascii_ikbdiorec` is not even compiled (`bios/ikbd.c:175,231`).
/// NewTOS' device profile declares handle 1 (AUX) a port with no cable on
/// this board — status 0, input 0 (`tos_api::bios::chardev::AuxNoCable`) —
/// so the byte has no destination and is dropped here.
///
/// This is not cosmetic. UART0 is the debug console; on this board CON is the
/// USB-HID keyboard alone (STATUS 2026-07-03 K1, `c6e4bfb`), and that commit
/// left the IRQ path feeding CON only because the PL011 RX interrupt was
/// believed to be undeliverable here (`docs/MEMORY.md` QUIRK-002). The
/// 2026-09-04 boot-screen measurement disproves that: with no serial adapter
/// attached, GPIO 15 (RXD0) is an input with **no pull**
/// (`bcm2711::pl011::configure_uart0_pins`) and clocked in one spurious
/// `0x00` character — the same effect `docs/hardware/ikbd-uart.md:104` records
/// for a floating RXD2 — which arrived in this queue as a key word `0x0000`
/// before the boot screen was even drawn, and was eaten by the first
/// `Bconstat(2)`/`Bconin(2)` of the boot.
#[cfg(not(feature = "board-qemu-virt"))]
fn serial_rx_byte(_b: u8) {
    #[cfg(feature = "initinfo-diag")]
    crate::initinfo::diag::note_serial_drop(_b);
}

// ---------------------------------------------------------------------------
// AUX on qemu-virt (XBIOS15).
// ---------------------------------------------------------------------------

/// AUX (BIOS device 1) on qemu-virt: the serial port whose receive interrupt
/// fills device 0's input ring ([`serial_rx_byte`]). Input as EmuTOS'
/// `bconstat1`/`bconin1` over `iorec1` (`bios/serport.c:255-263`):
/// `Bconstat(1)` is `-1` while the ring holds a byte (`bconstat_iorec`,
/// `:191-199`), `Bconin(1)` waits for one and returns it zero-extended
/// (`bconin_iorec` + `get_iorecbuf`, `:171-210`); on the trap path the wait
/// gives way to a `kbdvec` hook ([`aux_read_blocking`], XBIOS18). Output is unchanged from
/// the "port present, no cable" entry it replaces
/// ([`tos_api::bios::chardev::AuxNoCable`]): `Bconout(1)` accepts and
/// discards (`1`), `Bcostat(1)` is ready (`-1`). Installed at boot by
/// [`install_aux`]; real Pi-4 keeps the no-cable entry.
#[cfg(feature = "board-qemu-virt")]
pub struct AuxSerial;

#[cfg(feature = "board-qemu-virt")]
impl tos_api::bios::chardev::CharDevice for AuxSerial {
    fn bconstat(&self) -> tos_types::Long {
        let r = if iorec::serial_pending() { -1 } else { 0 };
        report_iorec_repairs();
        r
    }

    fn bconin(&self) -> tos_types::Long {
        // Never gives way: `aux_read_blocking(false)` only returns with a byte.
        let b = aux_read_blocking(false).unwrap_or(0);
        report_iorec_repairs();
        tos_types::Long::from(b)
    }

    fn bconin_restartable(&self) -> Option<tos_types::Long> {
        let b = aux_read_blocking(true);
        report_iorec_repairs();
        b.map(tos_types::Long::from)
    }

    fn bconout(&self, byte: u8) -> tos_types::Long {
        tos_api::bios::chardev::AuxNoCable.bconout(byte)
    }

    fn bcostat(&self) -> tos_types::Long {
        tos_api::bios::chardev::AuxNoCable.bcostat()
    }
}

/// `bconin_iorec`'s wait (`bios/serport.c:202-210`): the oldest byte of
/// device 0's input ring, waiting until one arrives. Runs inside the
/// synchronous SVC handler with IRQs masked, so it unmasks them while it
/// waits — the receive interrupt is the producer — and `wfi`s between
/// checks (the next PL011 or timer interrupt wakes it). The keyboard and
/// mouse keep being pumped meanwhile, as EmuTOS' interrupts keep filling the
/// key queue while `bconin1` spins.
///
/// With `give_way` (the trap entry, [`AuxSerial`]'s `bconin_restartable`),
/// returns `None` as soon as the running process has bytes queued for its
/// `kbdvec` hook (XBIOS18; KEYS02, `docs/api/xbios/22-kbdvbase.md` C3 "O1"):
/// EmuTOS' IKBD interrupt jumps through `kbdvec` while `bconin1` spins
/// (`bios/aciavecs.S:407-414`), NewTOS delivers the bytes on the way back to
/// EL0 — the caller requests a restart, the hook runs, and the `svc` waits
/// again. Nothing is consumed before the byte arrives, so the restart has no
/// side effect. The same rule as the console wait
/// (`crate::host_env`'s `read_key_loop`).
#[cfg(feature = "board-qemu-virt")]
fn aux_read_blocking(give_way: bool) -> Option<u8> {
    let daif: u64;
    // SAFETY: reading DAIF and clearing the IRQ mask are permitted at EL1;
    // DAIF is restored before returning.
    unsafe {
        core::arch::asm!("mrs {0}, daif", out(reg) daif, options(nomem, nostack, preserves_flags));
        core::arch::asm!("msr daifclr, #2", options(nomem, nostack, preserves_flags));
    }
    let b = loop {
        if let Some(b) = iorec::pop_serial() {
            break Some(b);
        }
        if let Some(h) = tos_api::host::host() {
            h.pump_input();
        }
        if iorec::serial_pending() {
            continue;
        }
        if give_way && tos_api::xbios::kbdvecs::pending_for_running() {
            break None;
        }
        // SAFETY: wait for the next interrupt (PL011 receive or timer tick).
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)) };
    };
    // SAFETY: restore the caller's DAIF.
    unsafe { core::arch::asm!("msr daif, {0}", in(reg) daif, options(nomem, nostack, preserves_flags)) };
    b
}

#[cfg(feature = "board-qemu-virt")]
static AUX_SERIAL: &dyn tos_api::bios::chardev::CharDevice = &AuxSerial;

/// Put the qemu-virt AUX driver ([`AuxSerial`]) into BIOS device slot 1 —
/// the driver hook of the device table
/// (`tos_api::bios::chardev::install_device`). Called once at boot, before
/// the PL011 receive interrupt is enabled.
#[cfg(feature = "board-qemu-virt")]
pub fn install_aux() {
    let prev = tos_api::bios::chardev::install_device(tos_api::bios::chardev::DEV_AUX, &AUX_SERIAL);
    debug_assert!(prev.is_some(), "DEV_AUX is a device table slot");
}

/// Push a key **entry** into the IKBD ring from the IKBD decoder —
/// `shifty << 24 | scancode << 16 | ascii` (ascii 0 for keys with no
/// character, e.g. the arrow / function keys; `shifty` the shift state when
/// the key went down, EmuTOS `kb_last.k.shifty`, `bios/ikbd.c:898-909`). A
/// full ring drops it (`push_ikbdiorec`, `bios/ikbd.c:167-170`).
pub fn push_rx(entry: u32) {
    #[cfg(feature = "initinfo-diag")]
    crate::initinfo::diag::note_push(crate::initinfo::diag::PUSH_SRC_IKBD, iorec::word_from_entry(entry));
    iorec::push_key(entry);
    report_iorec_repairs();
}

/// Pop one **ASCII** byte from the key ring for the console byte-stream
/// readers (`Cconrs` line input, `Fread` from CON, VDI `v_locator`) —
/// scancode-only keys (arrows etc., ascii 0) are skipped, exactly as when they
/// were never queued. The scancode-returning char reads
/// (`Bconin(2)`/`Cconin`/`Cnecin`/`Crawcin`) instead pop the whole entry via
/// [`rx_pop_entry`].
pub fn rx_pop() -> Option<u8> {
    let r = loop {
        let Some(e) = iorec::pop_key() else { break None };
        let ascii = (e & 0xff) as u8;
        if ascii != 0 {
            break Some(ascii);
        }
    };
    report_iorec_repairs();
    r
}

/// Pop one whole key word (`scancode << 8 | ascii`) for the AES event
/// manager, so it can deliver scancodes (arrow / function keys) to apps.
pub fn rx_pop_key() -> Option<u16> {
    rx_pop_entry().map(iorec::word_from_entry)
}

/// Pop one whole ring entry (`shifty << 24 | scancode << 16 | ascii`) for
/// `Bconin(2)` and the GEMDOS single-key reads.
pub fn rx_pop_entry() -> Option<u32> {
    let r = iorec::pop_key();
    report_iorec_repairs();
    r
}

/// Non-destructively peek the oldest queued key word (`scancode << 8 |
/// ascii`) — the front element a subsequent [`rx_pop_key`] would return —
/// without consuming it. Backs `Host::console_peek_key`.
pub fn rx_peek_key() -> Option<u16> {
    let r = iorec::peek_key().map(iorec::word_from_entry);
    report_iorec_repairs();
    r
}

/// `true` if the key ring holds at least one key (`bconstat2`).
pub fn rx_has_input() -> bool {
    let r = iorec::key_pending();
    report_iorec_repairs();
    r
}

/// The key ring's `(head, tail)` byte indices.
///
/// Diagnostic accessor for the `initinfo-diag` build: the boot-screen wait
/// loop's "a key is waiting" test is `head != tail`, so the two raw indices
/// say both *whether* something was queued and *where* the ring stands.
#[cfg(feature = "initinfo-diag")]
pub fn rx_indices() -> (usize, usize) {
    let (head, tail) = iorec::ikbd_indices();
    (usize::from(head), usize::from(tail))
}

/// After a trap (XBIOS01 E4, "at the next trap"): check every IOREC record —
/// so a replaced buffer is noticed even when no key moves — and warn about
/// what was repaired.
pub fn check_iorec_after_trap() {
    iorec::check_records();
    report_iorec_repairs();
}

/// Print one warning per IOREC repair not yet reported for the running
/// process (`tos_api::xbios::iorec::take_warnings`): a program's change to
/// the records is never undone silently (`docs/api/xbios/0e-iorec.md` E4).
pub fn report_iorec_repairs() {
    use tos_api::bdos::process::{current, slot_for};
    let slot = slot_for(current());
    let bits = iorec::take_warnings(slot);
    if bits == 0 {
        return;
    }
    for (i, name) in iorec::RING_NAMES.iter().enumerate() {
        if bits & (iorec::FAULT_BUF << i) != 0 {
            crate::kprintln!(
                "[iorec] warning: {} buffer or size replaced by the program (slot {:?}); \
                 NewTOS keeps its own buffer, restored and emptied (docs/api/xbios/0e-iorec.md E4)",
                name,
                slot
            );
        }
        if bits & (iorec::FAULT_INDEX << i) != 0 {
            crate::kprintln!(
                "[iorec] warning: {} head/tail outside the ring (slot {:?}); ring emptied \
                 (docs/api/xbios/0e-iorec.md E1)",
                name,
                slot
            );
        }
    }
}

/// ADR-031 (`docs/MEMORY.md`): the hook checks after a trap — the `Kbdvbase`
/// slots the input since the last trap would have called, the page-0 cells of
/// a process in super state (`tos_api::bdos::process::hooks::after_trap`) —
/// then one serial line per OS hook a program installed that NewTOS does not
/// call (once per hook and process), or that its end left installed. `tos-api`
/// queues them; the log is the kernel's. Called by
/// `arch::aarch64::exceptions::rust_sync_handler` after every exception, with
/// `trap_or_el0` = it was a trap or a fault of an EL0 program; nothing runs
/// after the kernel's own recoverable aborts (`uaccess` fix-ups, MMIO probes),
/// which arrive in the middle of a trap.
pub fn report_hook_warnings(trap_or_el0: bool) {
    use tos_api::bdos::process::hooks;
    if !trap_or_el0 {
        return;
    }
    hooks::after_trap();
    let mut printed = false;
    while let Some(w) = hooks::pop() {
        crate::kprintln!("{}", w);
        printed = true;
    }
    // Only a full queue drops a warning, and a full queue was just printed.
    if printed {
        let lost = hooks::dropped();
        if lost != 0 {
            crate::kprintln!("[hook] warning: {} further hook warning(s) lost (queue full)", lost);
        }
    }
}

/// ADR-031 / TASK/BIOS16: the hook check before a trap is served — a vector
/// cell `$008`–`$407` a process in super state wrote straight into its page 0
/// since its last call becomes the shared cell first, so the call
/// (`Setexc(n, -1)`, a `Pexec`, its `Pterm`) sees the written value as on
/// EmuTOS (`tos_api::bdos::process::hooks::before_trap`). Called by
/// `arch::aarch64::exceptions::rust_sync_handler` before every exception, with
/// `trap_or_el0` as for [`report_hook_warnings`]; the warnings it queues are
/// printed by that after the call.
pub fn check_hooks_before_trap(trap_or_el0: bool) {
    if trap_or_el0 {
        tos_api::bdos::process::hooks::before_trap();
    }
}
