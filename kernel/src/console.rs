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
//! burst beyond that drops keys as on EmuTOS. Who may fill it is a per-board
//! decision that mirrors EmuTOS' `CONF_SERIAL_CONSOLE` switch — see
//! [`serial_rx_byte`]: on real Pi-4 only the IKBD decoder ([`push_rx`], fed
//! from the USB-HID keyboard); on qemu-virt additionally the PL011 RX IRQ
//! handler ([`pl011_rx_drain`], serial console — a plain ASCII byte with
//! scancode 0).
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
/// Called from the GIC IRQ handler when the PL011 INTID is signalled.
/// Where the received bytes go is the board's console profile —
/// [`serial_rx_byte`].
pub fn pl011_rx_drain() {
    while let Some(b) = CONSOLE.read_byte() {
        serial_rx_byte(b);
    }
    CONSOLE.clear_rx_interrupt();
}

/// Route one byte received on the serial line — the board's EmuTOS console
/// profile decides where it goes.
///
/// qemu-virt has no other console input device, so it is the
/// `CONF_SERIAL_CONSOLE = 1` profile: *"use exclusively the serial port
/// input for console input"* (`references/emutos/include/config.h:1422-1429`).
/// There EmuTOS' RS-232 receive interrupt calls `push_ascii_ikbdiorec`, i.e.
/// the byte enters the **IKBD** queue (`bios/serport.c:352-357`), which is
/// exactly what happens here: it becomes a key word with scancode 0.
#[cfg(feature = "board-qemu-virt")]
fn serial_rx_byte(b: u8) {
    #[cfg(feature = "initinfo-diag")]
    crate::initinfo::diag::note_push(crate::initinfo::diag::PUSH_SRC_SERIAL, b as u16);
    iorec::push_key(u32::from(b));
}

/// Real Pi-4 has its own screen and its own keyboard (USB-HID), so it is
/// EmuTOS' default `CONF_SERIAL_CONSOLE = 0` profile
/// (`references/emutos/include/config.h:1427-1429`) — the one every machine
/// with a console of its own is built with. There the RS-232 receiver pushes
/// into `iorec1`, the **RS-232 device** (BIOS handle 1,
/// `bios/serport.c:331-343` `push_serial_iorec`), and *never* into the IKBD
/// queue; `push_ascii_ikbdiorec` is not even compiled (`bios/ikbd.c:175,231`).
/// NewTOS' device profile declares handle 1 (AUX) a port with no cable —
/// status 0, input 0 (`tos_api::bios::chardev::AuxNoCable`) — so the byte has
/// no destination and is dropped here.
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
