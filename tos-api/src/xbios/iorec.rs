//! XBIOS `Iorec` (0x0E) and the **IOREC page**: the three records `Iorec`
//! returns and their rings — the IKBD ring being the live console key queue
//! every keyboard consumer reads (`Bconstat(2)`/`Bconin(2)`, the GEMDOS
//! console, the AES, `Fread` on `CON:`).
//!
//! Spec: `docs/api/xbios/0e-iorec.md` (contract F1-F4, user decisions E1-E6
//! of 2026-10-28), layouts `docs/api/structures/iorec.md`. EmuTOS:
//! `bios/xbios.c:336-348` (`iorec`), `bios/ikbd.c:157-173`
//! (`push_ikbdiorec`), `:236-283` (`bconstat2`/`bconin2`), the records
//! `bios/aciavecs.S:93-114,233-239` and `bios/serport.c:84-89`.
//!
//! # Layout (F1)
//!
//! A NewTOS program reads the records through the SDK's `_IOREC`
//! (`sdk/libc/overlay/include/mint/ostruct.h:576-583@_IOREC`), so the kernel
//! lays them out as LP64 C does: `ibuf` an 8-byte EL0 pointer, the five
//! `short`s behind it, 24 bytes; device 0 two of them plus five bytes, 56.
//! All of it lives in one page mapped EL0 read-write in every process at
//! [`IOREC_PAGE_VA`] (E1); the kernel reaches it through its own alias
//! ([`mmu::iorec_page_kva`]), never through `ibuf`.
//!
//! # Ring (F3)
//!
//! EmuTOS' ring, byte for byte: `head`/`tail` are byte indices, empty iff
//! equal; the producer advances `tail` first and drops the entry when the ring
//! is full, the consumer advances `head` first and then reads. The IKBD steps
//! by 4 — an entry is the 32-bit `shifty << 24 | scancode << 16 | ascii`, the
//! shift state taken when the key was pushed (E2, `bios/ikbd.c:427-435,900`);
//! 256 bytes hold 63 keys. MIDI and RS-232 step by 1. No kernel producer
//! fills the MIDI ring: NewTOS has no MIDI driver behind it (E5); the BIOS
//! MIDI entry reads it ([`midi_pending`], [`pop_midi`], `Bconstat(3)`/
//! `Bconin(3)`, BIOS04), so a byte a program puts there is what `Bconin(3)`
//! returns, as on EmuTOS. Device 0's input ring is filled on both boards
//! by the serial receive interrupt ([`push_serial`], EmuTOS
//! `push_serial_iorec` / the SCC receive handler; XBIOS15 qemu-virt, BIOS09
//! rpi4) and read by the kernel's AUX driver ([`pop_serial`],
//! [`serial_pending`]). Its output ring stays empty everywhere: AUX output
//! is discarded at once.
//!
//! # The page is program data (E1, E4)
//!
//! Any program may write any byte of the page at any time, as on TOS. The
//! kernel therefore checks a record before **every** use: `ibuf`/`ibufsiz`
//! must be its own (E4 (a): a replaced buffer is not adopted — the record gets
//! the kernel buffer back, emptied), `ibufhd`/`ibuftl` must be inside the ring
//! and on an entry boundary (otherwise the ring is emptied, `head = tail =
//! 0`). It never reads or writes outside its own buffers. Each repair is
//! remembered ([`take_warnings`]) so the kernel warns once per process —
//! never silently. `ibuflow`/`ibufhi` are plain data nobody reads (EmuTOS does
//! not either, spec "Ringsemantik").
//!
//! # Delivery (E3 (a))
//!
//! Keys reach the ring only when the kernel pumps its input: at a trap that
//! reads or polls the console, in the AES idle loop, never from the 200 Hz
//! timer (`docs/api/xbios/0e-iorec.md` F3). A program that polls `ibuftl`
//! and never traps sees no key — documented, decision E3 (a).
//!
//! # Concurrency
//!
//! Every operation runs with IRQs masked: the PL011 receive
//! interrupt produces into device 0's input ring
//! (`kernel::console::serial_rx_byte`) while a consumer may run with IRQs
//! open (the blocking reads unmask them while they wait), as EmuTOS'
//! `bconin2` and `get_iorecbuf` mask around their reads
//! (`bios/ikbd.c:268-276`, `bios/serport.c:171-189`). The index cells are
//! accessed as 16-bit atomics (acquire/release), the slots with plain stores
//! ordered by them.

use core::sync::atomic::{AtomicBool, AtomicU16, AtomicU8, Ordering};

use crate::bdos::process::mmu::{self, IOREC_PAGE_VA};
use crate::bdos::process::MAX_PROCESSES;
use crate::trap::TrapArgs;

// ---------------------------------------------------------------------------
// LP64 layout (F1; checked against the SDK header by
// `sdk/libc/tests/iorec-layout.c`).
// ---------------------------------------------------------------------------

/// `sizeof(_IOREC)` under LP64.
pub const IOREC_SIZE: usize = 24;
/// `_IOREC.ibuf` (`char *`, 8 bytes).
pub const OFF_IBUF: usize = 0;
/// `_IOREC.ibufsiz` (`short`).
pub const OFF_IBUFSIZ: usize = 8;
/// `_IOREC.ibufhd` (`volatile short`, the consumer's byte index).
pub const OFF_IBUFHD: usize = 10;
/// `_IOREC.ibuftl` (`volatile short`, the producer's byte index).
pub const OFF_IBUFTL: usize = 12;
/// `_IOREC.ibuflow` (`short`).
pub const OFF_IBUFLOW: usize = 14;
/// `_IOREC.ibufhi` (`short`).
pub const OFF_IBUFHI: usize = 16;

/// `sizeof` of device 0's record (EmuTOS `EXT_IOREC`, `bios/serport.h:57-65`)
/// under LP64.
pub const EXT_IOREC_SIZE: usize = 56;
/// `EXT_IOREC.in` (an `_IOREC`).
pub const OFF_IN: usize = 0;
/// `EXT_IOREC.out` (an `_IOREC`).
pub const OFF_OUT: usize = 24;
/// `EXT_IOREC.baudrate` (`UBYTE`).
pub const OFF_BAUDRATE: usize = 48;
/// `EXT_IOREC.flowctrl` (`UBYTE`).
pub const OFF_FLOWCTRL: usize = 49;
/// `EXT_IOREC.ucr` (`UBYTE`).
pub const OFF_UCR: usize = 50;
/// `EXT_IOREC.datamask` (`UBYTE`).
pub const OFF_DATAMASK: usize = 51;
/// `EXT_IOREC.wr5` (`UBYTE`, the SCC shadow register — reference, F4).
pub const OFF_WR5: usize = 52;

const _: () = assert!(OFF_IBUFHI + 2 <= IOREC_SIZE && IOREC_SIZE.is_multiple_of(8));
const _: () = assert!(OFF_OUT == OFF_IN + IOREC_SIZE && OFF_BAUDRATE == OFF_OUT + IOREC_SIZE);
const _: () = assert!(OFF_WR5 < EXT_IOREC_SIZE && EXT_IOREC_SIZE.is_multiple_of(8));

// ---------------------------------------------------------------------------
// Page layout (reference, F4: the addresses are never contract).
// ---------------------------------------------------------------------------

/// Device 0 (RS-232) record, an `EXT_IOREC`.
pub(super) const AUX_RECORD: usize = 0x000;
/// Device 1 (IKBD) record.
const IKBD_RECORD: usize = 0x040;
/// Device 2 (MIDI) record.
const MIDI_RECORD: usize = 0x060;
/// The `Bconmap` table — `BCONMAP`, its one `MAPTAB` and the call targets of
/// the entry's function slots (XBIOS05, [`super::bconmap`]).
pub(super) const BCONMAP_AREA: usize = 0x080;
/// The IKBD ring (`ikbd_bufsize` 256, `bios/aciavecs.S:97`).
const IKBD_BUF: usize = 0x100;
/// The MIDI ring (`midi_bufsize` 128, `bios/aciavecs.S:98`).
const MIDI_BUF: usize = 0x200;
/// Device 0's input ring (`RS232_BUFSIZE` 256, `bios/serport.c:84-89`).
const AUX_IN_BUF: usize = 0x300;
/// Device 0's output ring.
const AUX_OUT_BUF: usize = 0x400;

const _: () = assert!(AUX_RECORD + EXT_IOREC_SIZE <= IKBD_RECORD);
const _: () = assert!(IKBD_RECORD + IOREC_SIZE <= MIDI_RECORD);
const _: () = assert!(MIDI_RECORD + IOREC_SIZE <= BCONMAP_AREA);
const _: () = assert!(BCONMAP_AREA + super::bconmap::AREA_SIZE <= IKBD_BUF);
const _: () = assert!(IKBD_BUF + 256 <= MIDI_BUF && MIDI_BUF + 128 <= AUX_IN_BUF);
const _: () = assert!(AUX_IN_BUF + 256 <= AUX_OUT_BUF && AUX_OUT_BUF + 256 <= 4096);

/// EL0 address of the IKBD record — `Iorec(1)`, and what the `kbdvec` hook
/// receives in `x1` (EmuTOS `a0 = &ikbdiorec`, `bios/aciavecs.S:412`; E6).
pub const IKBD_IOREC_VA: u64 = IOREC_PAGE_VA + IKBD_RECORD as u64;

/// Device 0's line settings after `init_serport` (`bios/serport.c:86-89`):
/// `DEFAULT_BAUDRATE` = `B9600` = 1 (`include/config.h:1527-1528`),
/// `FLOW_CTRL_NONE`, `ucr` 0x88, `datamask` 0xff, `wr5` 0xea.
const AUX_LINE: [u8; 5] = [1, 0, 0x88, 0xFF, 0xEA];

/// One ring: its record, the kernel's buffer, the geometry.
#[derive(Clone, Copy)]
struct RingDesc {
    /// Offset of the `_IOREC` in the page.
    rec: usize,
    /// Offset of the kernel's buffer in the page.
    buf: usize,
    /// `ibufsiz`.
    size: u16,
    /// Bytes per entry: 4 for the IKBD (`bios/ikbd.c:163,269`), 1 otherwise
    /// (`bios/midi.c:48`, `bios/serport.c:150`).
    step: u16,
    /// `ibuflow` / `ibufhi`: a quarter and three quarters of the size
    /// (`bios/aciavecs.S:235-239`, `bios/serport.c:87-88`).
    low: u16,
    high: u16,
}

const IKBD: RingDesc = RingDesc { rec: IKBD_RECORD, buf: IKBD_BUF, size: 256, step: 4, low: 64, high: 192 };
const MIDI: RingDesc = RingDesc { rec: MIDI_RECORD, buf: MIDI_BUF, size: 128, step: 1, low: 32, high: 96 };
const AUX_IN: RingDesc =
    RingDesc { rec: AUX_RECORD + OFF_IN, buf: AUX_IN_BUF, size: 256, step: 1, low: 64, high: 192 };
const AUX_OUT: RingDesc =
    RingDesc { rec: AUX_RECORD + OFF_OUT, buf: AUX_OUT_BUF, size: 256, step: 1, low: 64, high: 192 };

/// Every ring, in the order of the fault bits ([`take_warnings`]).
const RINGS: [RingDesc; 4] = [IKBD, MIDI, AUX_IN, AUX_OUT];
/// Index of the IKBD ring in [`RINGS`].
const IKBD_IDX: usize = 0;
/// Index of the MIDI ring in [`RINGS`].
const MIDI_IDX: usize = 1;
/// Index of device 0's input ring in [`RINGS`].
const AUX_IN_IDX: usize = 2;

const _: () = assert!(RINGS[IKBD_IDX].rec == IKBD_RECORD && RINGS[AUX_IN_IDX].rec == AUX_RECORD + OFF_IN);
const _: () = assert!(RINGS[MIDI_IDX].rec == MIDI_RECORD);

// ---------------------------------------------------------------------------
// Repairs and their warnings.
// ---------------------------------------------------------------------------

/// Fault bit: ring `i`'s `ibuf`/`ibufsiz` was not the kernel's; the record
/// got the kernel buffer back, emptied (E4 (a)). Shifted left by `i`.
pub const FAULT_BUF: u8 = 0x01;
/// Fault bit: ring `i`'s `ibufhd`/`ibuftl` was outside the ring or off an
/// entry boundary; the ring was emptied (E1). Shifted left by `i + 4`.
pub const FAULT_INDEX: u8 = 0x10;

/// Names of the rings for a warning, in [`RINGS`] order.
pub const RING_NAMES: [&str; 4] = ["Iorec(1) IKBD", "Iorec(2) MIDI", "Iorec(0) RS-232 in", "Iorec(0) RS-232 out"];

/// Repairs made since the last [`take_warnings`].
static PENDING: AtomicU8 = AtomicU8::new(0);
/// Per process slot (and one for "no process"): the repairs already warned
/// about during this tenancy.
static WARNED: [AtomicU8; MAX_PROCESSES + 1] = [const { AtomicU8::new(0) }; MAX_PROCESSES + 1];

/// Whether the page holds its records ([`init_page`] ran).
static READY: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// The page.
// ---------------------------------------------------------------------------

/// A view of an IOREC page (the real one or, in unit tests, a local copy).
#[derive(Clone, Copy)]
struct Page {
    base: *mut u8,
    /// The EL0 address the page is mapped at (what `ibuf` holds).
    va: u64,
}

impl Page {
    fn idx(&self, off: usize) -> &AtomicU16 {
        debug_assert!(off.is_multiple_of(2) && off + 2 <= 4096);
        // SAFETY: `base` is a live, 4-KiB-aligned page; `off` is one of the
        // record offsets above (even, inside the page). The cell is shared
        // with EL0, which may write it at any time — an atomic view is the
        // honest type for that.
        unsafe { AtomicU16::from_ptr(self.base.add(off) as *mut u16) }
    }

    fn read_u64(&self, off: usize) -> u64 {
        // SAFETY: as in `idx`; `off` is 8-aligned (the `ibuf` fields).
        unsafe { (self.base.add(off) as *const u64).read_volatile() }
    }

    fn write_u64(&self, off: usize, v: u64) {
        // SAFETY: as in `read_u64`.
        unsafe { (self.base.add(off) as *mut u64).write_volatile(v) }
    }

    fn write_u8(&self, off: usize, v: u8) {
        // SAFETY: `off` is inside the page.
        unsafe { self.base.add(off).write_volatile(v) }
    }

    fn read_u8(&self, off: usize) -> u8 {
        // SAFETY: `off` is inside the page.
        unsafe { self.base.add(off).read_volatile() }
    }

    fn slot_u8(&self, d: &RingDesc, at: u16) -> *mut u8 {
        debug_assert!(d.step == 1 && at < d.size);
        // SAFETY: `at` is a validated index of `d`'s byte ring (below `size`),
        // so the slot lies inside the kernel's buffer.
        unsafe { self.base.add(d.buf + at as usize) }
    }

    fn slot_u32(&self, d: &RingDesc, at: u16) -> *mut u32 {
        debug_assert!(at < d.size && at.is_multiple_of(4));
        // SAFETY: `at` is a validated index of `d`'s ring (below `size`, a
        // multiple of the 4-byte step), so the slot lies inside the kernel's
        // buffer and is 4-aligned.
        unsafe { self.base.add(d.buf + at as usize) as *mut u32 }
    }

    /// Write every record's initial values (E5, the oracle's I01-I03):
    /// kernel buffer, size, empty, the water marks, device 0's line bytes;
    /// and the `Bconmap` table (XBIOS05).
    fn init(&self) {
        for d in &RINGS {
            self.reset(d);
            self.idx(d.rec + OFF_IBUFLOW).store(d.low, Ordering::Relaxed);
            self.idx(d.rec + OFF_IBUFHI).store(d.high, Ordering::Relaxed);
        }
        for (i, b) in AUX_LINE.iter().enumerate() {
            self.write_u8(AUX_RECORD + OFF_BAUDRATE + i, *b);
        }
        self.bconmap_area().init();
    }

    /// The `Bconmap` table's area of this page.
    fn bconmap_area(&self) -> super::bconmap::Area {
        // SAFETY: `BCONMAP_AREA + AREA_SIZE` lies inside the page (asserted
        // above).
        let base = unsafe { self.base.add(BCONMAP_AREA) };
        super::bconmap::Area { base, va: self.va + BCONMAP_AREA as u64 }
    }

    /// Give ring `d` the kernel buffer and size back, empty.
    fn reset(&self, d: &RingDesc) {
        self.write_u64(d.rec + OFF_IBUF, self.va + d.buf as u64);
        self.idx(d.rec + OFF_IBUFSIZ).store(d.size, Ordering::Relaxed);
        self.idx(d.rec + OFF_IBUFHD).store(0, Ordering::Release);
        self.idx(d.rec + OFF_IBUFTL).store(0, Ordering::Release);
    }

    /// Check ring `i` before use; repair it if a program broke it. Returns
    /// the valid `(head, tail)` and the fault bits of the repair (0 = none).
    fn check(&self, i: usize) -> (u16, u16, u8) {
        let d = &RINGS[i];
        let buf = self.read_u64(d.rec + OFF_IBUF);
        let size = self.idx(d.rec + OFF_IBUFSIZ).load(Ordering::Relaxed);
        if buf != self.va + d.buf as u64 || size != d.size {
            self.reset(d);
            return (0, 0, FAULT_BUF << i);
        }
        let head = self.idx(d.rec + OFF_IBUFHD).load(Ordering::Acquire);
        let tail = self.idx(d.rec + OFF_IBUFTL).load(Ordering::Acquire);
        let ok = |x: u16| x < d.size && x.is_multiple_of(d.step);
        if !ok(head) || !ok(tail) {
            self.idx(d.rec + OFF_IBUFHD).store(0, Ordering::Release);
            self.idx(d.rec + OFF_IBUFTL).store(0, Ordering::Release);
            return (0, 0, FAULT_INDEX << i);
        }
        (head, tail, 0)
    }

    /// `push_ikbdiorec` (`bios/ikbd.c:157-173`): `false` when the ring is
    /// full and the entry is dropped.
    fn push(&self, entry: u32) -> (bool, u8) {
        let (head, tail, fault) = self.check(IKBD_IDX);
        let d = &IKBD;
        let mut next = tail + d.step;
        if next >= d.size {
            next = 0;
        }
        if next == head {
            return (false, fault); // full
        }
        // SAFETY: `next` is a valid slot index (see `slot_u32`).
        unsafe { self.slot_u32(d, next).write_volatile(entry) };
        self.idx(d.rec + OFF_IBUFTL).store(next, Ordering::Release);
        (true, fault)
    }

    /// `push_serial_iorec` (`bios/serport.c:331-343`) and the receive half of
    /// `scc_rx_interrupt_handler` (`:670-678`) into byte ring `i`: the byte
    /// masked with device 0's `datamask` (the SCC path, `:671`; 0xff unless a
    /// program wrote the record), `tail` advanced first (`incr_tail`,
    /// `:146-154`), dropped when the ring is full. `false` when dropped.
    fn push_byte(&self, i: usize, b: u8) -> (bool, u8) {
        let (head, tail, fault) = self.check(i);
        let d = &RINGS[i];
        let mut next = tail + d.step;
        if next >= d.size {
            next = 0;
        }
        if next == head {
            return (false, fault); // full
        }
        let data = b & self.read_u8(AUX_RECORD + OFF_DATAMASK);
        // SAFETY: `next` is a valid slot index (see `slot_u8`).
        unsafe { self.slot_u8(d, next).write_volatile(data) };
        self.idx(d.rec + OFF_IBUFTL).store(next, Ordering::Release);
        (true, fault)
    }

    /// `get_iorecbuf` (`bios/serport.c:171-189`) on byte ring `i`: `head`
    /// advanced first, then the byte; `None` when the ring is empty (EmuTOS'
    /// callers test `bconstat_iorec` first, `:203-209`).
    fn pop_byte(&self, i: usize) -> (Option<u8>, u8) {
        let (head, tail, fault) = self.check(i);
        if head == tail {
            return (None, fault);
        }
        let d = &RINGS[i];
        let mut next = head + d.step;
        if next >= d.size {
            next = 0;
        }
        // SAFETY: `next` is a valid slot index (see `slot_u8`).
        let v = unsafe { self.slot_u8(d, next).read_volatile() };
        self.idx(d.rec + OFF_IBUFHD).store(next, Ordering::Release);
        (Some(v), fault)
    }

    /// `bconin2`'s dequeue (`bios/ikbd.c:269-273`); `consume = false` peeks.
    fn pop(&self, consume: bool) -> (Option<u32>, u8) {
        let (head, tail, fault) = self.check(IKBD_IDX);
        if head == tail {
            return (None, fault);
        }
        let d = &IKBD;
        let mut next = head + d.step;
        if next >= d.size {
            next = 0;
        }
        // SAFETY: `next` is a valid slot index (see `slot_u32`).
        let v = unsafe { self.slot_u32(d, next).read_volatile() };
        if consume {
            self.idx(d.rec + OFF_IBUFHD).store(next, Ordering::Release);
        }
        (Some(v), fault)
    }
}

/// The real page, its records written on first use.
fn page() -> Page {
    let p = Page { base: mmu::iorec_page_kva(), va: IOREC_PAGE_VA };
    if !READY.load(Ordering::Acquire) {
        p.init();
        READY.store(true, Ordering::Release);
    }
    p
}

/// Run `f` on the real page with IRQs masked (see "Concurrency"), and note
/// its repairs for [`take_warnings`].
fn with_page<R>(f: impl FnOnce(&Page) -> (R, u8)) -> R {
    let saved = crate::uaccess::irq_mask();
    let (r, fault) = f(&page());
    crate::uaccess::irq_restore(saved);
    if fault != 0 {
        PENDING.fetch_or(fault, Ordering::AcqRel);
    }
    r
}

/// Write the records if they are not there yet.
fn ensure_ready() {
    with_page(|_| ((), 0));
}

/// Run `f` on the `Bconmap` table of the real page (written first if it is
/// not there yet), IRQs masked like every page access ([`super::bconmap`]).
pub(super) fn with_bconmap_area<R>(f: impl FnOnce(&super::bconmap::Area) -> R) -> R {
    with_page(|p| (f(&p.bconmap_area()), 0))
}

// ---------------------------------------------------------------------------
// The key entry.
// ---------------------------------------------------------------------------

/// The IKBD ring entry (E2, `bios/ikbd.c:427-435`) for a key word
/// `scancode << 8 | ascii` and the shift state at push time.
pub const fn entry_from_word(word: u16, shifty: u8) -> u32 {
    (shifty as u32) << 24 | ((word >> 8) as u32) << 16 | (word & 0xFF) as u32
}

/// The key word `scancode << 8 | ascii` of a ring entry — the form the AES
/// event manager and the VDI string input deliver.
pub const fn word_from_entry(entry: u32) -> u16 {
    (((entry >> 16) & 0xFF) as u16) << 8 | (entry & 0xFF) as u16
}

// ---------------------------------------------------------------------------
// The IKBD queue.
// ---------------------------------------------------------------------------

/// Queue a key entry (`push_ikbdiorec`). `false` when the ring is full and
/// the key is dropped, as on EmuTOS.
pub fn push_key(entry: u32) -> bool {
    with_page(|p| p.push(entry))
}

/// Dequeue the oldest key entry (`bconin2`'s read), `None` when empty.
pub fn pop_key() -> Option<u32> {
    with_page(|p| p.pop(true))
}

/// The oldest key entry without dequeuing it.
pub fn peek_key() -> Option<u32> {
    with_page(|p| p.pop(false))
}

/// `bconstat2` (`bios/ikbd.c:241-246`): a key is waiting.
pub fn key_pending() -> bool {
    with_page(|p| {
        let (head, tail, fault) = p.check(IKBD_IDX);
        (head != tail, fault)
    })
}

/// The IKBD ring's `(head, tail)` after the check — diagnostics only.
pub fn ikbd_indices() -> (u16, u16) {
    with_page(|p| {
        let (head, tail, fault) = p.check(IKBD_IDX);
        ((head, tail), fault)
    })
}

// ---------------------------------------------------------------------------
// The MIDI input ring (device 2, read by BIOS device 3).
// ---------------------------------------------------------------------------

/// Dequeue the oldest byte of the MIDI ring — `bconin3`'s read
/// (`bios/midi.c:46-57`: `head` advanced first, then the byte), `None` when
/// it is empty. No producer in the kernel fills this ring (E5); a byte in it
/// was put there by a program, through the record `Iorec(2)` returns (E1).
pub fn pop_midi() -> Option<u8> {
    with_page(|p| p.pop_byte(MIDI_IDX))
}

/// `bconstat3` (`bios/midi.c:22-33`): a byte waits in the MIDI ring.
pub fn midi_pending() -> bool {
    with_page(|p| {
        let (head, tail, fault) = p.check(MIDI_IDX);
        (head != tail, fault)
    })
}

// ---------------------------------------------------------------------------
// Device 0's input ring (RS-232, AUX).
// ---------------------------------------------------------------------------

/// Queue one byte received on the serial line into device 0's input ring —
/// EmuTOS `push_serial_iorec` (`bios/serport.c:331-343`, ST/TT MFP) and the
/// push in `scc_rx_interrupt_handler` (`:647-683`, Falcon SCC B): the ring
/// `Iorec(0)` returns, the one `bconstat1`/`bconin1` read. `false` when the
/// ring is full and the byte is dropped, as on EmuTOS. Called from the
/// receive interrupt on both boards (XBIOS15, BIOS09).
pub fn push_serial(b: u8) -> bool {
    with_page(|p| p.push_byte(AUX_IN_IDX, b))
}

/// Dequeue the oldest byte of device 0's input ring (`get_iorecbuf`,
/// `bios/serport.c:171-189`), `None` when it is empty.
pub fn pop_serial() -> Option<u8> {
    with_page(|p| p.pop_byte(AUX_IN_IDX))
}

/// `bconstat_iorec` (`bios/serport.c:191-199`): a byte waits in device 0's
/// input ring.
pub fn serial_pending() -> bool {
    with_page(|p| {
        let (head, tail, fault) = p.check(AUX_IN_IDX);
        (head != tail, fault)
    })
}

/// Check every record (E4: "at the next trap"): called by the kernel after
/// each trap, so a replaced buffer is noticed even when no key moves.
pub fn check_records() {
    with_page(|p| {
        let mut fault = 0;
        for i in 0..RINGS.len() {
            fault |= p.check(i).2;
        }
        ((), fault)
    })
}

/// The repairs not yet warned about for process `slot` (`None` = no process
/// runs), as `FAULT_*` bits; marks them warned. The kernel prints one line
/// per bit — a repair is never silent, and each kind is reported once per
/// process (E4). Bit `i` ([`FAULT_BUF`]) / `i + 4` ([`FAULT_INDEX`]) names
/// ring [`RING_NAMES`]`[i]`.
pub fn take_warnings(slot: Option<usize>) -> u8 {
    let pending = PENDING.swap(0, Ordering::AcqRel);
    if pending == 0 {
        return 0;
    }
    let cell = &WARNED[slot.filter(|&s| s < MAX_PROCESSES).unwrap_or(MAX_PROCESSES)];
    let old = cell.fetch_or(pending, Ordering::AcqRel);
    pending & !old
}

/// Process exit (`free_slot`): the next tenant of `slot` gets its own
/// warnings.
pub fn release_slot(slot: usize) {
    if let Some(w) = WARNED.get(slot) {
        w.store(0, Ordering::Release);
    }
}

// ---------------------------------------------------------------------------
// XBIOS 0x0E.
// ---------------------------------------------------------------------------

/// The EL0 address of device `devno`'s record, `None` for any other device.
/// The selector is a signed `WORD` (`bios/xbios.c:336-348`): `3`, `-1`,
/// `0x7FFF`, `-32768` are no device (oracle I04-I07).
pub fn record_va(devno: i16) -> Option<u64> {
    let off = match devno {
        0 => AUX_RECORD,
        1 => IKBD_RECORD,
        2 => MIDI_RECORD,
        _ => return None,
    };
    Some(IOREC_PAGE_VA + off as u64)
}

/// `Iorec(devno)` (XBIOS 0x0E): a pointer to the device's record, the same
/// one on every call, or `-1` (`bios/xbios.c:336-348`). The records are
/// written before the pointer is handed out.
pub fn iorec(args: &TrapArgs) -> i64 {
    match record_va(args.arg_i16(0)) {
        Some(va) => {
            ensure_ready();
            va as i64
        }
        None => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(4096))]
    struct TestPage([u8; 4096]);

    const VA: u64 = 0x7D000;

    fn fresh() -> (Box<TestPage>, Page) {
        let mut b = Box::new(TestPage([0xA5; 4096]));
        let p = Page { base: b.0.as_mut_ptr(), va: VA };
        p.init();
        (b, p)
    }

    fn rd16(b: &TestPage, off: usize) -> u16 {
        u16::from_ne_bytes([b.0[off], b.0[off + 1]])
    }
    fn wr16(b: &mut TestPage, off: usize, v: u16) {
        b.0[off..off + 2].copy_from_slice(&v.to_ne_bytes());
    }
    fn rd64(b: &TestPage, off: usize) -> u64 {
        u64::from_ne_bytes(b.0[off..off + 8].try_into().unwrap())
    }

    #[test]
    fn records_start_as_the_oracle_shows_them() {
        let (b, _) = fresh();
        // IKBD: 256, empty, 64/192 (I02b).
        assert_eq!(rd64(&b, IKBD_RECORD + OFF_IBUF), VA + IKBD_BUF as u64);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFSIZ), 256);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFHD), 0);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFTL), 0);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFLOW), 64);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFHI), 192);
        // MIDI: 128, 32/96 (I03b).
        assert_eq!(rd16(&b, MIDI_RECORD + OFF_IBUFSIZ), 128);
        assert_eq!(rd16(&b, MIDI_RECORD + OFF_IBUFLOW), 32);
        assert_eq!(rd16(&b, MIDI_RECORD + OFF_IBUFHI), 96);
        // Device 0: in and out 256, 64/192, baud 1, flow 0, ucr 0x88,
        // datamask 0xff (I01b/I01c), wr5 0xea (bios/serport.c:89).
        for r in [OFF_IN, OFF_OUT] {
            assert_eq!(rd16(&b, AUX_RECORD + r + OFF_IBUFSIZ), 256);
            assert_eq!(rd16(&b, AUX_RECORD + r + OFF_IBUFLOW), 64);
            assert_eq!(rd16(&b, AUX_RECORD + r + OFF_IBUFHI), 192);
            assert_eq!(rd16(&b, AUX_RECORD + r + OFF_IBUFHD), rd16(&b, AUX_RECORD + r + OFF_IBUFTL));
        }
        assert_ne!(rd64(&b, AUX_RECORD + OFF_IN), rd64(&b, AUX_RECORD + OFF_OUT));
        assert_eq!(b.0[AUX_RECORD + OFF_BAUDRATE..AUX_RECORD + OFF_WR5 + 1], [1, 0, 0x88, 0xFF, 0xEA]);
    }

    #[test]
    fn keys_come_out_in_order_with_their_push_time_shift_state() {
        let (_b, p) = fresh();
        // Shift+b pushed with shifty 0x02; Shift released afterwards does not
        // change the entry (BIOS11, bios/ikbd.c:900).
        assert!(p.push(entry_from_word(0x3042, 0x02)).0);
        assert!(p.push(entry_from_word(0x1E61, 0x00)).0);
        assert_eq!(p.pop(false).0, Some(0x0230_0042), "peek");
        assert_eq!(p.pop(true).0, Some(0x0230_0042));
        assert_eq!(p.pop(true).0, Some(0x001E_0061));
        assert_eq!(p.pop(true).0, None);
    }

    #[test]
    fn the_ring_steps_by_four_and_holds_63_keys() {
        let (b, p) = fresh();
        for k in 0..63u32 {
            assert!(p.push(k).0, "key {k}");
        }
        assert!(!p.push(99).0, "the 64th is dropped (bios/ikbd.c:167-170)");
        // The producer advanced tail first: the first entry is at byte 4.
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFTL), 252);
        for k in 0..63u32 {
            assert_eq!(p.pop(true).0, Some(k));
        }
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFHD), 252);
        // Wrap: the next entry goes to byte 0.
        assert!(p.push(7).0);
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFTL), 0);
        assert_eq!(p.pop(true).0, Some(7));
    }

    #[test]
    fn a_program_flush_head_eq_tail_empties_the_queue() {
        let (mut b, p) = fresh();
        for k in 1..=3u32 {
            p.push(k);
        }
        let tail = rd16(&b, IKBD_RECORD + OFF_IBUFTL);
        wr16(&mut b, IKBD_RECORD + OFF_IBUFHD, tail); // io->ibufhd = io->ibuftl
        let (h, t, fault) = p.check(IKBD_IDX);
        assert_eq!((h == t, fault), (true, 0), "a valid flush is no fault");
        assert_eq!(p.pop(true).0, None);
    }

    #[test]
    fn bad_indices_empty_the_ring_and_are_reported() {
        for bad in [256u16, 2, 0x8000, 0xFFFF] {
            let (mut b, p) = fresh();
            p.push(1);
            wr16(&mut b, IKBD_RECORD + OFF_IBUFTL, bad);
            assert_eq!(p.pop(true), (None, FAULT_INDEX), "tail {bad:#x}");
            assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFHD), 0);
            assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFTL), 0);
        }
        // MIDI steps by 1: an odd index is fine there, past the size is not.
        let (mut b, p) = fresh();
        wr16(&mut b, MIDI_RECORD + OFF_IBUFHD, 5);
        wr16(&mut b, MIDI_RECORD + OFF_IBUFTL, 5);
        assert_eq!(p.check(1).2, 0);
        wr16(&mut b, MIDI_RECORD + OFF_IBUFTL, 128);
        assert_eq!(p.check(1).2, FAULT_INDEX << 1);
    }

    #[test]
    fn a_replaced_buffer_is_not_adopted() {
        let (mut b, p) = fresh();
        p.push(1);
        // A terminal program installs its own buffer (iorec.md, "Install
        // custom buffer"): E4 (a) gives the record the kernel buffer back.
        b.0[IKBD_RECORD..IKBD_RECORD + 8].copy_from_slice(&0x1234_5678u64.to_ne_bytes());
        assert_eq!(p.pop(true), (None, FAULT_BUF));
        assert_eq!(rd64(&b, IKBD_RECORD + OFF_IBUF), VA + IKBD_BUF as u64);
        wr16(&mut b, IKBD_RECORD + OFF_IBUFSIZ, 1024);
        assert_eq!(p.push(5), (true, FAULT_BUF));
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFSIZ), 256);
        assert_eq!(p.pop(true).0, Some(5), "the ring works on after the repair");
        // Device 0's out ring: bit 3.
        wr16(&mut b, AUX_RECORD + OFF_OUT + OFF_IBUFSIZ, 4096);
        assert_eq!(p.check(3).2, FAULT_BUF << 3);
    }

    #[test]
    fn serial_bytes_fill_device_0_in_not_the_key_ring() {
        // XBIOS15, oracle SERRX-001: 'a' 'Z' '1' land in device 0's input
        // ring as hd 0 tl 3 with the bytes at 1..=3; the IKBD ring stays empty.
        let (b, p) = fresh();
        for c in *b"aZ1" {
            assert_eq!(p.push_byte(AUX_IN_IDX, c), (true, 0));
        }
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFHD), 0);
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFTL), 3);
        assert_eq!(b.0[AUX_IN_BUF + 1..AUX_IN_BUF + 4], *b"aZ1");
        assert_eq!(rd16(&b, IKBD_RECORD + OFF_IBUFHD), rd16(&b, IKBD_RECORD + OFF_IBUFTL));
        assert_eq!(rd16(&b, AUX_RECORD + OFF_OUT + OFF_IBUFTL), 0, "the out ring is untouched");
        // get_iorecbuf: head first, then the byte (oracle C01: hd 1 tl 3).
        assert_eq!(p.pop_byte(AUX_IN_IDX), (Some(b'a'), 0));
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFHD), 1);
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(b'Z'));
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(b'1'));
        assert_eq!(p.pop_byte(AUX_IN_IDX), (None, 0));
    }

    #[test]
    fn the_serial_ring_holds_255_bytes_and_wraps() {
        let (b, p) = fresh();
        for k in 0..255u32 {
            assert!(p.push_byte(AUX_IN_IDX, k as u8).0, "byte {k}");
        }
        assert!(!p.push_byte(AUX_IN_IDX, 0xEE).0, "the 256th is dropped (bios/serport.c:337-338)");
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFTL), 255);
        for k in 0..255u32 {
            assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(k as u8));
        }
        assert!(p.push_byte(AUX_IN_IDX, 0x42).0);
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFTL), 0, "wrap to byte 0");
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(0x42));
    }

    #[test]
    fn the_serial_byte_is_masked_with_datamask() {
        // scc_rx_interrupt_handler: `port->data & extiorec->datamask`
        // (bios/serport.c:671); 0xff after init (bios/serport.c:89).
        let (mut b, p) = fresh();
        assert!(p.push_byte(AUX_IN_IDX, 0xE1).0);
        b.0[AUX_RECORD + OFF_DATAMASK] = 0x7F;
        assert!(p.push_byte(AUX_IN_IDX, 0xE1).0);
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(0xE1));
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(0x61));
    }

    #[test]
    fn a_broken_serial_ring_is_repaired_before_the_push() {
        let (mut b, p) = fresh();
        wr16(&mut b, AUX_RECORD + OFF_IN + OFF_IBUFTL, 256);
        assert_eq!(p.push_byte(AUX_IN_IDX, b'x'), (true, FAULT_INDEX << AUX_IN_IDX));
        assert_eq!(rd16(&b, AUX_RECORD + OFF_IN + OFF_IBUFTL), 1);
        assert_eq!(p.pop_byte(AUX_IN_IDX).0, Some(b'x'));
    }

    #[test]
    fn entry_and_word_convert_both_ways() {
        assert_eq!(entry_from_word(0x3B00, 0), 0x003B_0000); // F1
        assert_eq!(entry_from_word(0x2E03, 0x04), 0x042E_0003); // Ctrl+c
        assert_eq!(word_from_entry(0x0230_0042), 0x3042);
    }

    #[test]
    fn iorec_selects_by_signed_word() {
        assert_eq!(record_va(0), Some(IOREC_PAGE_VA));
        assert_eq!(record_va(1), Some(IKBD_IOREC_VA));
        assert_eq!(record_va(2), Some(IOREC_PAGE_VA + MIDI_RECORD as u64));
        for d in [3i16, -1, 0x7FFF, -32768] {
            assert_eq!(record_va(d), None, "devno {d}");
        }
    }

    #[test]
    fn warnings_are_given_once_per_process() {
        // Global state: only this test touches PENDING/WARNED in this module.
        PENDING.store(FAULT_BUF, Ordering::Release);
        assert_eq!(take_warnings(Some(3)), FAULT_BUF);
        PENDING.store(FAULT_BUF | FAULT_INDEX, Ordering::Release);
        assert_eq!(take_warnings(Some(3)), FAULT_INDEX, "the buffer repair was warned already");
        PENDING.store(FAULT_BUF, Ordering::Release);
        assert_eq!(take_warnings(Some(4)), FAULT_BUF, "another process gets its own");
        release_slot(3);
        PENDING.store(FAULT_BUF, Ordering::Release);
        assert_eq!(take_warnings(Some(3)), FAULT_BUF, "the next tenant of slot 3 too");
        assert_eq!(take_warnings(Some(3)), 0, "nothing pending");
    }
}
