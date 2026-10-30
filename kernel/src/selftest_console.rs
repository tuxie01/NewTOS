//! GEMDOS console self-test — console-audit fixes F1 (Cconrs line
//! editor: Ctrl-U / Ctrl-R), F2 (Cauxin / Cauxout pass-through) and F3
//! (Cconout / Cconws output-side Ctrl-C break, the conbrk subset).
//!
//! * **F2** is driven through the real `svc #1` (GEMDOS) trap path:
//!   both route through the BIOS handle-1 pass-through. On rpi4 that is
//!   the AUX entry "port present, no cable"
//!   (`docs/api/gemdos/file/3f-fread.md` item 1a): `Cauxin` (0x03) returns
//!   `0` without blocking (no input). On qemu-virt it is the serial AUX
//!   driver over `Iorec(0)`'s input ring (XBIOS15): a byte queued there
//!   first is what `Cauxin` returns (it would wait on an empty ring, as
//!   EmuTOS' `bconin1`). `Cauxout` (0x04) accepts and discards the byte and
//!   returns `1` on both. See
//!   `docs/api/gemdos/console/{03-cauxin,04-cauxout}.md`.
//!
//! * **F1** exercises the factored [`tos_api::gemdos::console::edit_line`]
//!   directly with a scripted input + captured echo (a [`MockIo`]), so the
//!   line-editing contract is asserted without a live keyboard or user
//!   memory: Ctrl-U kills the line (`len = 0`), Ctrl-R redisplays it
//!   (`len` unchanged), alongside the existing CR / Backspace / Ctrl-X /
//!   Ctrl-C / max-length behaviour (no regress). See
//!   `docs/api/gemdos/console/0a-cconrs.md`.
//!
//! * **F3** injects keys into the console RX ring and calls `cconout`
//!   **directly** (not via `svc`, so the deferred-terminate atomic can be
//!   inspected here instead of being consumed by the SVC handler's Pterm
//!   redirect) — EmuTOS `conbrk` (TASK/CON01): a pending Ctrl-C must abort
//!   the write and set `take_pending_terminate() == Some(-32)` while
//!   consuming that key; a pending normal key moves from the ring into the
//!   BDOS typeahead (`Cconis` stays -1, the next `Cnecin` returns it); a
//!   Ctrl-S + Ctrl-Q pair pauses and resumes and is consumed. `cconws` runs
//!   the same `tabout`/`conbrk`; its user-pointer `ldtrb` faults at EL1 by
//!   design (see `selftest.rs`), so the full `cconws` round-trip is real-Pi4
//!   / EL0-Pexec. See `docs/api/gemdos/console/{02-cconout,09-cconws}.md`.
//!
//! Not wired into the normal boot; invoke [`run`] from `kernel_main` when
//! debugging (mirrors the other `selftest_*` modules).

use tos_api::bdos::process::take_pending_terminate;
use tos_api::gemdos::console::{cconout, cconws, edit_line, LineEdit, LineEditIo, Terminated};
use tos_api::TrapArgs;

use crate::{console, kprintln};

/// Issue a GEMDOS call: `svc #1`, function id in `x8`, one arg in `x0`.
fn gemdos(func: u16, a0: u64) -> i64 {
    let ret: i64;
    // SAFETY: GEMDOS trap ABI; the handler reads x0/x8 and writes x0.
    unsafe {
        core::arch::asm!(
            "svc #1",
            in("x8") func as u64,
            inout("x0") a0 => ret,
            options(nostack),
        );
    }
    ret
}

/// A one-argument [`TrapArgs`] for a **direct** (non-`svc`) call into a
/// GEMDOS leaf, so F3 can invoke `cconout`/`cconws` without the SVC
/// handler's post-dispatch terminate redirect.
fn args1(a0: u64) -> TrapArgs {
    TrapArgs {
        regs: [a0, 0, 0, 0, 0, 0, 0, 0],
        func: 0,
        svc_imm: 1,
        space: tos_api::uaccess::Space::Kernel,
    }
}

/// Drain any queued keys, the BDOS typeahead (`Crawcin` while `Cconis`) and
/// a stale pending-terminate so an F3 sub-test starts from a known-empty
/// state (and nothing leaks into the boot flow).
fn reset_input() {
    let _ = take_pending_terminate();
    while console::rx_pop_key().is_some() {}
    while gemdos(0x0B, 0) != 0 {
        let _ = gemdos(0x07, 0);
    }
}

// ── F1 line-editor mock backend ──────────────────────────────────────

const MOCK_BUF: usize = 64;
const MOCK_ECHO: usize = 256;

/// In-memory [`LineEditIo`]: replays `script` byte-by-byte, records echoes
/// and tracks their column (the `conout` rule), and stores the accumulated
/// line in `buf`. Reaching the end of the script yields CR (0x0D) so a
/// missing terminator cannot loop forever.
struct MockIo<'a> {
    script: &'a [u8],
    pos: usize,
    buf: [u8; MOCK_BUF],
    echo: [u8; MOCK_ECHO],
    echo_len: usize,
    col: i16,
}

impl<'a> MockIo<'a> {
    fn new(script: &'a [u8]) -> Self {
        Self {
            script,
            pos: 0,
            buf: [0; MOCK_BUF],
            echo: [0; MOCK_ECHO],
            echo_len: 0,
            col: 0,
        }
    }
}

impl LineEditIo for MockIo<'_> {
    fn read_byte(&mut self) -> u8 {
        let b = self.script.get(self.pos).copied().unwrap_or(0x0D);
        self.pos += 1;
        b
    }
    fn conout(&mut self, b: u8) -> Result<(), Terminated> {
        if self.echo_len < self.echo.len() {
            self.echo[self.echo_len] = b;
            self.echo_len += 1;
        }
        match b {
            0x20..=0xFF => self.col += 1,
            0x0D => self.col = 0,
            0x08 => self.col -= 1,
            _ => {}
        }
        Ok(())
    }
    fn column(&self) -> i16 {
        self.col
    }
    fn store_byte(&mut self, i: usize, b: u8) {
        if i < self.buf.len() {
            self.buf[i] = b;
        }
    }
    fn load_byte(&self, i: usize) -> u8 {
        self.buf.get(i).copied().unwrap_or(0)
    }
}

/// Run `edit_line` over `script` with the given `max`, returning the
/// resulting length (or `None` on Ctrl-C `Terminated`) and the `MockIo`
/// for buffer/echo inspection.
fn drive(script: &[u8], max: usize) -> (Option<usize>, MockIo<'_>) {
    let mut io = MockIo::new(script);
    let out = match edit_line(&mut io, max) {
        LineEdit::Done(n) => Some(n),
        LineEdit::Terminated => None,
        // The in-memory mock never faults.
        LineEdit::Faulted => None,
    };
    (out, io)
}

/// Run the console self-test suite.
pub fn run() {
    kprintln!("[console] === GEMDOS console test (F1 Cconrs editor / F2 Caux*) ===");
    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[console]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // ── F2: Cauxin / Cauxout pass through to the AUX entry ───────────
    // Cauxin waits for a byte on both boards (XBIOS15, BIOS09): one is
    // queued first.
    tos_api::xbios::iorec::push_serial(b'Q');
    check(gemdos(0x03, 0) == i64::from(b'Q'), "Cauxin() == 'Q' from Iorec(0) (XBIOS15/BIOS09, F2)");
    check(gemdos(0x04, b'Z' as u64) == 1, "Cauxout('Z') == 1 (AUX no cable, F2)");

    // ── F1: Ctrl-U kills the line → subsequent input starts at len 0 ──
    {
        let (len, io) = drive(b"foo\x15bar\r", 16);
        check(
            len == Some(3) && io.buf[..3] == *b"bar",
            "Cconrs Ctrl-U kills line (\"foo\"+^U+\"bar\" -> len 3 \"bar\", F1)",
        );
    }

    // ── F1: Ctrl-R redisplays, buffer unchanged ──────────────────────
    {
        let (len, io) = drive(b"foo\x12\r", 16);
        // The redisplay echoes `#`, CR LF (start column 0) then the kept
        // buffer "foo".
        let echo_ok = io.echo[..io.echo_len].windows(6).any(|w| w == b"#\r\nfoo");
        check(
            len == Some(3) && io.buf[..3] == *b"foo" && echo_ok,
            "Cconrs Ctrl-R redisplays, len kept 3 \"foo\" (F1)",
        );
    }

    // ── No regress: plain line, Backspace, Ctrl-X, max-length, Ctrl-C ─
    {
        let (len, io) = drive(b"AB\r", 10);
        check(len == Some(2) && io.buf[..2] == *b"AB", "Cconrs \"AB\"+CR -> len 2");
    }
    {
        let (len, io) = drive(b"A\x08B\r", 10);
        check(len == Some(1) && io.buf[0] == b'B', "Cconrs Backspace erases 1 (A BS B -> \"B\")");
    }
    {
        let (len, io) = drive(b"a\x18bc\r", 10);
        check(len == Some(2) && io.buf[..2] == *b"bc", "Cconrs Ctrl-X erases line (a ^X bc -> \"bc\")");
    }
    {
        // max = 2: three chars fill the buffer without a terminator.
        let (len, io) = drive(b"ABC", 2);
        check(len == Some(2) && io.buf[..2] == *b"AB", "Cconrs max-length stops at 2");
    }
    {
        let (len, _io) = drive(b"ab\x03", 10);
        check(len.is_none(), "Cconrs Ctrl-C terminates (no length written)");
    }

    // ── F3: Cconout/Cconws output-side Ctrl-C break (conbrk subset) ───
    // (a) Ctrl-C pending → cconout aborts, defers Pterm(-32), consumes it.
    reset_input();
    console::push_rx(0x03); // Ctrl-C (scancode 0, ascii 0x03)
    let _ = cconout(&args1(b'X' as u64));
    check(
        take_pending_terminate() == Some(-32) && console::rx_pop_key().is_none(),
        "cconout: pending Ctrl-C aborts + defers Pterm(-32), key consumed (F3)",
    );

    // (b) Normal key pending → moved into the BDOS typeahead, no terminate,
    //     byte written; the next Cnecin returns it.
    reset_input();
    console::push_rx(u32::from(b'A')); // normal key (ascii 0x41)
    let _ = cconout(&args1(b'Y' as u64));
    check(
        take_pending_terminate().is_none()
            && console::rx_pop_key().is_none()
            && gemdos(0x0B, 0) == -1
            && gemdos(0x08, 0) & 0xFF == b'A' as i64,
        "cconout: normal key -> BDOS typeahead, Cnecin returns it (F3, CON01)",
    );

    // (c) Ctrl-S then Ctrl-Q: pause and resume, both consumed, none stored.
    reset_input();
    console::push_rx(0x13); // Ctrl-S
    console::push_rx(0x11); // Ctrl-Q
    let _ = cconout(&args1(b'Z' as u64));
    check(
        take_pending_terminate().is_none() && console::rx_pop_key().is_none() && gemdos(0x0B, 0) == 0,
        "cconout: Ctrl-S/Ctrl-Q consumed, nothing stored (F3, CON01)",
    );

    // (d) Empty ring → normal write, no terminate (no regress).
    reset_input();
    let _ = cconout(&args1(b'.' as u64));
    check(
        take_pending_terminate().is_none() && console::rx_pop_key().is_none(),
        "cconout: empty ring -> normal write, no terminate (F3 no-regress)",
    );

    // (e) cconws NULL pointer → 0 (no regress; the user-pointer end-to-end
    //     path is real-Pi4 — EL1 ldtrb on a kernel string faults by design).
    check(cconws(&args1(0)) == 0, "cconws(NULL) == 0 (no regress)");

    // Ensure nothing leaks into the subsequent boot flow.
    reset_input();

    if fails == 0 {
        kprintln!("[console] === GEMDOS console test PASS ===");
    } else {
        kprintln!("[console] === GEMDOS console test FAILED ({}) ===", fails);
    }
}
