//! TASK/FS11 — `Fread` / `Fwrite` on character-device handles through the one
//! per-BIOS-device dispatch point.
//!
//! Spec: `docs/api/gemdos/file/3f-fread.md` and `40-fwrite.md`, sections
//! "Character-device path" and the tables "NewTOS host tests" (R1..R32,
//! RT1..RT3, W1..W28, WT1..WT3; WT4 is the in-crate unit test
//! `gemdos::console::tests::wt4_*`, the host cannot fault a user pointer).
//! The device table and the absent-device profile: `3f-fread.md` "NewTOS
//! implementation notes"; the BIOS/GEMDOS/XBIOS notes in
//! `docs/api/bios/{01,02,03,08}-*.md`, `docs/api/gemdos/console/*.md`,
//! `docs/api/xbios/0c-midiws.md`.
//!
//! EmuTOS `osif` (`bdos/bdosmain.c:604-640`): `Fread` on a device = HUGE guard,
//! `conin` for one byte, else `cgets`; `Fwrite` = `tabout` on CON, `Bconout`
//! (short count on 0) elsewhere.
//!
//! Basepages are host buffers; on the host a PD's user address is its kernel
//! address.

mod fatvol;

use std::collections::VecDeque;
use std::sync::Mutex;

use fatvol::*;
use tos_api::bdos::process;
use tos_api::bios::chardev::{install_device, CharDevice};
use tos_api::trap::{TrapArgs, TrapClass};
use tos_types::BasePage;

const FDUP: u16 = 0x45;
const FFORCE: u16 = 0x46;
const FSEEK: u16 = 0x42;
const CAUXIN: u16 = 0x03;
const CAUXOUT: u16 = 0x04;
const CPRNOUT: u16 = 0x05;
const CPRNOS: u16 = 0x11;
const CAUXIS: u16 = 0x12;
const CAUXOS: u16 = 0x13;
const CCONOUT: u16 = 0x02;
const CCONWS: u16 = 0x09;
const CRAWCIN: u16 = 0x07;
const CCONIS: u16 = 0x0B;

const BCONSTAT: u16 = 0x01;
const BCONIN: u16 = 0x02;
const BCONOUT: u16 = 0x03;
const BCOSTAT: u16 = 0x08;
const MIDIWS: u16 = 0x0C;

const EIHNDL: i32 = -37;

const CR: u8 = 0x0D;
const LF: u8 = 0x0A;
const BS: u8 = 0x08;
const DEL: u8 = 0x7F;
const TAB: u8 = 0x09;
const ESC: u8 = 0x1B;
const CTRL_C: u8 = 0x03;
const CTRL_S: u8 = 0x13;
const CTRL_Q: u8 = 0x11;
const CTRL_U: u8 = 0x15;
const CTRL_X: u8 = 0x18;

/// EmuTOS `default_handle` (`bdos/console.c:60-68`).
const DEFAULT: [i8; 6] = [-1, -1, -2, -3, -1, -1];

// ── process ───────────────────────────────────────────────────────────────

/// A process basepage in host memory with the default standard handles, made
/// the running process while it lives (`run`).
struct Pd(*mut BasePage);
impl Pd {
    fn new() -> Pd {
        let bp = Box::into_raw(Box::new([0u8; 256])) as *mut BasePage;
        // SAFETY: fresh box.
        unsafe { (*bp).p_uft = DEFAULT };
        process::set_current(bp);
        Pd(bp)
    }
    fn uft(&self) -> [i8; 6] {
        // SAFETY: the box is alive.
        unsafe { (*self.0).p_uft }
    }
}
impl Drop for Pd {
    fn drop(&mut self) {
        if process::current() == self.0 {
            process::set_current(core::ptr::null_mut());
        }
        // SAFETY: nothing names the box any more.
        drop(unsafe { Box::from_raw(self.0 as *mut [u8; 256]) });
    }
}

/// The per-test setup: the file-system lock, the host with an empty console,
/// a running process with the default standard handles, and no stale
/// terminate request. The BDOS console state (TASK/CON01) is reset through
/// the interface: the typeahead drained (`Crawcin` while `Cconis`), the CON
/// column put to 0 by a CR.
fn setup() -> (std::sync::MutexGuard<'static, ()>, &'static DiskHost, Pd) {
    let (g, host) = fs();
    host.reset_console();
    let _ = process::take_pending_terminate();
    let pd = Pd::new();
    while gemdos0(CCONIS) != 0 {
        let _ = gemdos0(CRAWCIN);
    }
    let _ = call(CCONOUT, [CR as u64, 0, 0, 0, 0, 0, 0, 0]);
    host.reset_console();
    (g, host, pd)
}

// ── calls ─────────────────────────────────────────────────────────────────

/// `Fread(h, len, buf)` with the handle WORD sign-extended and the LONG `len`
/// as given (its low 32 bits are the LONG).
fn fread_raw(h: i32, len: i64, buf: *mut u8) -> i32 {
    call(FREAD, [h as i64 as u64, len as u64, buf as u64, 0, 0, 0, 0, 0])
}

fn fread(h: i32, buf: &mut [u8]) -> i32 {
    fread_raw(h, buf.len() as i64, buf.as_mut_ptr())
}

fn fwrite_raw(h: i32, len: i64, buf: *const u8) -> i32 {
    call(FWRITE, [h as i64 as u64, len as u64, buf as u64, 0, 0, 0, 0, 0])
}

fn fwrite(h: i32, data: &[u8]) -> i32 {
    fwrite_raw(h, data.len() as i64, data.as_ptr())
}

fn fclose(h: i32) -> i32 {
    gemdos1(FCLOSE, h as u16 as u64)
}

fn fdup(h: i32) -> i32 {
    gemdos1(FDUP, h as u64)
}

fn fforce(std: i32, h: i32) -> i32 {
    gemdos2(FFORCE, std as u16 as u64, h as u16 as u64)
}

fn gemdos0(func: u16) -> i32 {
    call(func, [0; 8])
}

fn bios(func: u16, a0: u64, a1: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, a1, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Bios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::bios::dispatch(&args)
}

fn xbios(func: u16, a0: u64, a1: u64) -> i64 {
    let args = TrapArgs {
        regs: [a0, a1, 0, 0, 0, 0, 0, 0],
        func: func as u64,
        svc_imm: TrapClass::Xbios as u16,
        space: tos_api::uaccess::Space::Kernel,
    };
    tos_api::xbios::dispatch(&args)
}

fn fcreate(name: &str) -> i32 {
    let p = cpath(name);
    let h = gemdos2(FCREATE, p.as_ptr() as u64, 0);
    assert!(h >= 6, "Fcreate {name} -> {h}");
    h
}

fn fopen(name: &str) -> i32 {
    let p = cpath(name);
    let h = gemdos2(FOPEN, p.as_ptr() as u64, 0);
    assert!(h >= 6, "Fopen {name} -> {h}");
    h
}

// ── a replaceable device entry (RT*, WT*, W19-W21) ───────────────────────

/// A test character device: `Bconin` pops its input queue (0 when empty, never
/// blocks), `Bconout` captures and accepts up to `accept` bytes (then 0),
/// `Bconstat` = -1 while input is queued, `Bcostat` = -1 (so a status read of
/// this entry is distinguishable from the absent entry's 0).
struct TestDev {
    input: Mutex<VecDeque<u8>>,
    out: Mutex<Vec<u8>>,
    accept: Mutex<usize>,
    instat: Mutex<u32>,
    outstat: Mutex<u32>,
}

impl TestDev {
    const fn new() -> TestDev {
        TestDev {
            input: Mutex::new(VecDeque::new()),
            out: Mutex::new(Vec::new()),
            accept: Mutex::new(usize::MAX),
            instat: Mutex::new(0),
            outstat: Mutex::new(0),
        }
    }
    fn reset(&self, input: &[u8], accept: usize) {
        *self.input.lock().unwrap() = input.iter().copied().collect();
        self.out.lock().unwrap().clear();
        *self.accept.lock().unwrap() = accept;
        *self.instat.lock().unwrap() = 0;
        *self.outstat.lock().unwrap() = 0;
    }
    fn feed(&self, input: &[u8]) {
        self.input.lock().unwrap().extend(input.iter().copied());
    }
    fn out(&self) -> Vec<u8> {
        core::mem::take(&mut *self.out.lock().unwrap())
    }
    fn untouched(&self) -> bool {
        self.out.lock().unwrap().is_empty()
            && *self.instat.lock().unwrap() == 0
            && *self.outstat.lock().unwrap() == 0
    }
}

impl CharDevice for TestDev {
    fn bconstat(&self) -> i32 {
        *self.instat.lock().unwrap() += 1;
        if self.input.lock().unwrap().is_empty() { 0 } else { -1 }
    }
    fn bconin(&self) -> i32 {
        self.input.lock().unwrap().pop_front().map_or(0, |b| b as i32)
    }
    fn bconout(&self, byte: u8) -> i32 {
        let mut a = self.accept.lock().unwrap();
        if *a == 0 {
            return 0;
        }
        *a -= 1;
        self.out.lock().unwrap().push(byte);
        1
    }
    fn bcostat(&self) -> i32 {
        *self.outstat.lock().unwrap() += 1;
        -1
    }
}

static DEV_A: TestDev = TestDev::new();
static DEV_A_REF: &dyn CharDevice = &DEV_A;
static DEV_B: TestDev = TestDev::new();
static DEV_B_REF: &dyn CharDevice = &DEV_B;

/// Install `entry` as BIOS device `dev` for the life of the guard; the entry
/// that was there comes back on drop.
struct Installed {
    dev: u16,
    prev: &'static &'static dyn CharDevice,
}
impl Installed {
    fn new(dev: u16, entry: &'static &'static dyn CharDevice) -> Installed {
        let prev = install_device(dev, entry).expect("device number in range");
        Installed { dev, prev }
    }
}
impl Drop for Installed {
    fn drop(&mut self) {
        install_device(self.dev, self.prev);
    }
}

// ═══ Fread, CON ══════════════════════════════════════════════════════════

/// R1, R2: one byte = `conin`: blocking key, echoed, stored; CR stored raw.
#[test]
fn r01_r02_one_byte_is_conin() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 1];
    host.type_keys(b"A");
    assert_eq!(fread(-1, &mut b), 1, "R1");
    assert_eq!(b[0], b'A');
    assert_eq!(host.take_output(), b"A", "R1 echo");

    host.type_keys(&[CR]);
    assert_eq!(fread(-1, &mut b), 1, "R2");
    assert_eq!(b[0], CR, "R2 CR stored");
    assert_eq!(host.take_output(), [CR], "R2 echo CR only");
}

/// R1 detail: the stored byte is the low byte of the `Bconin` LONG — a
/// scancode-only key (arrow) is stored as 0x00 and echoed raw
/// (`conin` = `conout(h, (unsigned char)getch(h))`, `bdos/console.c:384-390`).
#[test]
fn r01_scancode_only_key_is_stored_as_its_low_byte() {
    let (_g, host, _p) = setup();
    let mut b = [0xFFu8; 1];
    host.type_word(0x4800); // up arrow: scancode 0x48, ASCII 0
    assert_eq!(fread(-1, &mut b), 1);
    assert_eq!(b[0], 0);
}

/// R3, R4, R5: `len >= 2` is a line read; the terminator is echoed as CR and
/// not stored.
#[test]
fn r03_r04_r05_line_read_ends_at_cr_or_lf() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 80];
    host.type_keys(&[b'h', b'i', CR]);
    assert_eq!(fread(-1, &mut b), 2, "R3");
    assert_eq!(&b[..3], &[b'h', b'i', 0], "R3 no CR/LF/NUL stored");
    assert_eq!(host.take_output(), b"hi\r", "R3 echo");

    let mut b = [0u8; 80];
    host.type_keys(&[b'h', b'i', LF]);
    assert_eq!(fread(-1, &mut b), 2, "R4");
    assert_eq!(&b[..2], b"hi");
    assert_eq!(host.take_output(), b"hi\r", "R4 echo");

    host.type_keys(&[CR]);
    assert_eq!(fread(-1, &mut b), 0, "R5 empty line");
    assert_eq!(host.take_output(), b"\r", "R5 echo");
}

/// R6: the line read ends at `len`; the next key stays queued — the echo's
/// `conbrk` moved it into the BDOS typeahead (TASK/CON01), so the next read
/// returns it.
#[test]
fn r06_line_read_ends_at_len() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 3];
    host.type_keys(b"abcd");
    assert_eq!(fread(-1, &mut b), 3, "R6");
    assert_eq!(&b, b"abc");
    assert!(!host.take_output().contains(&CR), "R6 no CR echoed");
    let mut b = [0u8; 1];
    assert_eq!(fread(-1, &mut b), 1, "R6 'd' still queued");
    assert_eq!(b[0], b'd');
}

/// R7, R8: BS / DEL erase the last character; on an empty buffer nothing.
#[test]
fn r07_r08_backspace_and_delete() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 80];
    host.type_keys(&[b'a', b'b', BS, b'c', CR]);
    assert_eq!(fread(-1, &mut b), 2, "R7");
    assert_eq!(&b[..2], b"ac");
    let out = host.take_output();
    assert!(out.windows(3).any(|w| w == [BS, b' ', BS]), "R7 erase echo: {out:02x?}");

    let mut b = [0u8; 80];
    host.type_keys(&[b'a', DEL, DEL, b'b', CR]);
    assert_eq!(fread(-1, &mut b), 1, "R8");
    assert_eq!(b[0], b'b');
}

/// R9, R10: Ctrl-X erases the line (`BS SP BS` per column), Ctrl-U echoes
/// `#` and a newline to the start column (`cgets`, TASK/CON01).
#[test]
fn r09_r10_ctrl_x_and_ctrl_u_discard_the_line() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 80];
    host.type_keys(&[b'a', b'b', CTRL_X, b'c', CR]);
    assert_eq!(fread(-1, &mut b), 1, "R9");
    assert_eq!(b[0], b'c');
    assert_eq!(host.take_output(), [b'a', b'b', BS, b' ', BS, BS, b' ', BS, b'c', CR], "R9 echo");

    let mut b = [0u8; 80];
    host.type_keys(&[b'a', b'b', CTRL_U, b'c', CR]);
    assert_eq!(fread(-1, &mut b), 1, "R10");
    assert_eq!(b[0], b'c');
    // The start column is 0 here (the CR echo of R9).
    assert_eq!(host.take_output(), b"ab#\r\nc\r", "R10 echo");
}

/// R11, R12: TAB and ESC are stored raw; the echo is `cookdout`: TAB
/// expanded to column 8, ESC as `^[` (TASK/CON01).
#[test]
fn r11_r12_tab_and_esc_are_stored_raw_and_echoed_cooked() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 80];
    host.type_keys(&[b'a', TAB, b'b', CR]);
    assert_eq!(fread(-1, &mut b), 3, "R11");
    assert_eq!(&b[..3], &[b'a', TAB, b'b']);
    assert_eq!(host.take_output(), b"a       b\r", "R11 echo: 7 spaces to column 8");

    let mut b = [0u8; 80];
    host.type_keys(&[ESC, CR]);
    assert_eq!(fread(-1, &mut b), 1, "R12");
    assert_eq!(b[0], ESC);
    assert_eq!(host.take_output(), b"^[\r", "R12 echo");
}

/// R13, R14, R15: `len == 0`, HUGE (high word set, incl. negative) and a
/// negative low word return 0 without touching the console.
#[test]
fn r13_r14_r15_zero_huge_and_negative_low_word() {
    let (_g, host, _p) = setup();
    host.type_keys(b"x\r");
    let mut b = [0u8; 4];
    for len in [0i64, 0x10000, -1, 0x7FFF_FFFF, 0x8000, 0xFFFF] {
        assert_eq!(fread_raw(-1, len, b.as_mut_ptr()), 0, "Fread(-1, {len:#x})");
    }
    assert_eq!(b, [0; 4], "nothing stored");
    assert_eq!(host.pending_keys(), vec![b'x' as u16, CR as u16], "nothing consumed");
    assert!(host.take_output().is_empty(), "nothing echoed");
}

/// R16: the console standard handles 0, 1, 4, 5 read the keyboard.
#[test]
fn r16_console_standard_handles() {
    let (_g, host, _p) = setup();
    for h in [0, 1, 4, 5] {
        let mut b = [0u8; 80];
        host.type_keys(&[b'x', CR]);
        assert_eq!(fread(h, &mut b), 1, "R16 Fread({h})");
        assert_eq!(b[0], b'x');
    }
}

/// R27: Ctrl-C in a line read terminates the process (exit -32).
#[test]
fn r27_ctrl_c_in_a_line_read_terminates() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 80];
    host.type_keys(&[CTRL_C]);
    let _ = fread(-1, &mut b);
    assert_eq!(process::take_pending_terminate(), Some(-32), "R27");
}

/// `conin` (len 1) stores Ctrl-C as data — no terminate (spec item 3).
#[test]
fn r27b_ctrl_c_in_a_one_byte_read_is_data() {
    let (_g, host, _p) = setup();
    let mut b = [0u8; 1];
    host.type_keys(&[CTRL_C]);
    assert_eq!(fread(-1, &mut b), 1);
    assert_eq!(b[0], CTRL_C);
    assert_eq!(process::take_pending_terminate(), None);
}

/// R28: keys stay queued between calls.
#[test]
fn r28_typeahead_survives_between_calls() {
    let (_g, host, _p) = setup();
    host.type_keys(&[b'a', b'b', CR]);
    let mut b = [0u8; 1];
    assert_eq!(fread(-1, &mut b), 1, "R28 first");
    assert_eq!(b[0], b'a');
    let mut b = [0u8; 80];
    assert_eq!(fread(-1, &mut b), 1, "R28 second");
    assert_eq!(b[0], b'b');
}

// ═══ Fread, AUX (no cable) / PRN (absent) ═══════════════════════════════

/// R17, R18, R29: AUX has no input ("no cable", item 1a) — every `Bconin` is
/// 0 without blocking, so a
/// line read fills the buffer with NULs; nothing reaches the console.
#[test]
fn r17_r18_r29_absent_aux() {
    let (_g, host, _p) = setup();
    host.type_keys(b"k"); // must not be read
    for h in [-2, 2] {
        let mut b = [0xAAu8; 80];
        assert_eq!(fread(h, &mut b), 80, "R17 Fread({h}, 80)");
        assert_eq!(b, [0; 80]);
        let mut b = [0xAAu8; 2];
        assert_eq!(fread(h, &mut b), 2, "R29 Fread({h}, 2)");
        assert_eq!(b, [0; 2]);
    }
    let mut b = [0xAAu8; 1];
    assert_eq!(fread(-2, &mut b), 1, "R18");
    assert_eq!(b[0], 0);
    assert_eq!(host.pending_keys(), vec![b'k' as u16], "CON input untouched");
    assert!(host.take_output().is_empty(), "nothing echoed to CON");
}

/// R19, R20: PRN is absent: `Fread(-3|3, 1)` = one NUL; `Fread(-3, 4)` = four
/// NULs (inferred by tracing `cgets` over a `Bconin` = 0 device,
/// `console.c:548,577-579`).
#[test]
fn r19_r20_absent_prn() {
    let (_g, host, _p) = setup();
    for h in [-3, 3] {
        let mut b = [0xAAu8; 1];
        assert_eq!(fread(h, &mut b), 1, "R19 Fread({h}, 1)");
        assert_eq!(b[0], 0);
    }
    let mut b = [0xAAu8; 4];
    assert_eq!(fread(-3, &mut b), 4, "R20 (inferred)");
    assert_eq!(b, [0; 4]);
    assert!(host.take_output().is_empty());
}

/// R30: the largest line read on an absent device returns at once.
#[test]
fn r30_largest_line_read_on_absent_devices() {
    let (_g, _host, _p) = setup();
    for h in [-2, -3] {
        let mut b = vec![0xAAu8; 0x7FFF];
        assert_eq!(fread(h, &mut b), 0x7FFF, "R30 Fread({h}, 0x7FFF)");
        assert!(b.iter().all(|&c| c == 0));
    }
}

/// R31, R32: the guards apply on every device; the device is not touched.
#[test]
fn r31_r32_guards_on_absent_devices() {
    let (_g, _host, _p) = setup();
    DEV_A.reset(b"zz", usize::MAX);
    let _aux = Installed::new(1, &DEV_A_REF);
    let mut b = [0xAAu8; 4];
    assert_eq!(fread_raw(-3, 0x8000, b.as_mut_ptr()), 0, "R31 PRN 0x8000");
    assert_eq!(fread_raw(-2, 0x10000, b.as_mut_ptr()), 0, "R31 AUX 0x10000");
    assert_eq!(fread_raw(-2, 0, b.as_mut_ptr()), 0, "R31 AUX 0");
    assert_eq!(fread_raw(-3, -1, b.as_mut_ptr()), 0, "R32 PRN -1");
    assert_eq!(fread_raw(-2, -1, b.as_mut_ptr()), 0, "R32 AUX -1");
    assert_eq!(b, [0xAA; 4]);
    assert_eq!(DEV_A.input.lock().unwrap().len(), 2, "AUX input not consumed");
    assert!(DEV_A.untouched());
}

// ═══ Fread, handle resolution ════════════════════════════════════════════

/// R21, R22: `<= -4` and a free sft slot are `EIHNDL`.
#[test]
fn r21_r22_invalid_handles() {
    let (_g, _host, _p) = setup();
    let mut b = [0u8; 1];
    assert_eq!(fread(-4, &mut b), EIHNDL, "R21 -4");
    assert_eq!(fread(-32768, &mut b), EIHNDL, "R21 -32768");
    assert_eq!(fread(6 + 15, &mut b), EIHNDL, "R22 free slot");
}

/// R23: an `Fdup` of the console handle reads the console; closed → `EIHNDL`.
#[test]
fn r23_fdup_of_console() {
    let (_g, host, _p) = setup();
    let d = fdup(0);
    assert!(d >= 6, "Fdup(0) -> {d}");
    host.type_keys(b"q");
    let mut b = [0u8; 1];
    assert_eq!(fread(d, &mut b), 1, "R23");
    assert_eq!(b[0], b'q');
    assert_eq!(fclose(d), E_OK);
    assert_eq!(fread(d, &mut b), EIHNDL, "R23 after Fclose");
}

/// R24: `Fforce(0, -2)` makes stdin AUX (no cable: no input).
#[test]
fn r24_fforce_stdin_to_aux() {
    let (_g, host, p) = setup();
    assert_eq!(fforce(0, -2), E_OK);
    host.type_keys(b"k");
    let mut b = [0xAAu8; 80];
    assert_eq!(fread(0, &mut b), 80, "R24");
    assert_eq!(b, [0; 80]);
    assert_eq!(host.pending_keys(), vec![b'k' as u16]);
    assert_eq!(fclose(0), E_OK);
    assert_eq!(p.uft(), DEFAULT);
}

/// R25, R26: stdin forced to a file reads the file without echo; `Fclose(0)`
/// reverts it to the console.
#[test]
fn r25_r26_fforce_stdin_to_a_file_and_back() {
    let (_g, host, p) = setup();
    let w = fcreate("C:\\FS11R25.TXT");
    assert_eq!(fwrite(w, b"AB"), 2);
    assert_eq!(fclose(w), E_OK);
    let fh = fopen("C:\\FS11R25.TXT");
    assert_eq!(fforce(0, fh), E_OK);
    let mut b = [0u8; 10];
    assert_eq!(fread(0, &mut b), 2, "R25");
    assert_eq!(&b[..2], b"AB");
    assert!(host.take_output().is_empty(), "R25 no echo");
    let _ = fclose(0);
    assert_eq!(p.uft(), DEFAULT);
    host.type_keys(b"z");
    let mut b = [0u8; 1];
    assert_eq!(fread(0, &mut b), 1, "R26");
    assert_eq!(b[0], b'z');
    let _ = fclose(fh);
}

// ═══ Fread, single dispatch point ════════════════════════════════════════

/// RT1, RT2: a device installed in the AUX slot is what `Fread(-2|2)`,
/// `Cauxin`, `Cauxis`, `Bconin(1)` and `Bconstat(1)` reach; CON stays
/// untouched.
#[test]
fn rt1_rt2_aux_entry_is_the_one_dispatch_point() {
    let (_g, host, _p) = setup();
    DEV_A.reset(&[b'o', b'k', CR], usize::MAX);
    let _aux = Installed::new(1, &DEV_A_REF);
    let mut b = [0u8; 80];
    assert_eq!(fread(-2, &mut b), 2, "RT1 Fread(-2)");
    assert_eq!(&b[..2], b"ok");
    assert_eq!(DEV_A.out(), b"ok\r", "RT1 echo on AUX");

    DEV_A.feed(&[b'o', b'k', CR]);
    let mut b = [0u8; 80];
    assert_eq!(fread(2, &mut b), 2, "RT1 Fread(2)");
    assert_eq!(&b[..2], b"ok");
    assert_eq!(DEV_A.out(), b"ok\r");

    DEV_A.feed(b"z");
    let mut b = [0u8; 1];
    assert_eq!(fread(-2, &mut b), 1, "RT2");
    assert_eq!(b[0], b'z');
    assert_eq!(DEV_A.out(), b"z", "RT2 echo");

    DEV_A.feed(b"c");
    assert_eq!(gemdos0(CAUXIS), -1, "Cauxis reads the entry's status");
    assert_eq!(gemdos0(CAUXIN), b'c' as i32, "Cauxin");
    assert_eq!(gemdos0(CAUXIS), 0);
    DEV_A.feed(b"d");
    assert_eq!(bios(BCONSTAT, 1, 0), -1, "Bconstat(1)");
    assert_eq!(bios(BCONIN, 1, 0), b'd' as i64, "Bconin(1)");
    assert!(host.take_output().is_empty(), "CON untouched");
}

/// RT3: the PRN entry (index 0) is reached by `Fread(-3|3)` and `Bconin(0)`,
/// never the AUX entry.
#[test]
fn rt3_prn_entry_is_index_0() {
    let (_g, _host, _p) = setup();
    DEV_A.reset(b"", usize::MAX);
    DEV_B.reset(b"pq", usize::MAX);
    let _aux = Installed::new(1, &DEV_A_REF);
    let _prn = Installed::new(0, &DEV_B_REF);
    let mut b = [0u8; 1];
    assert_eq!(fread(-3, &mut b), 1);
    assert_eq!(b[0], b'p', "RT3 Fread(-3)");
    assert_eq!(fread(3, &mut b), 1);
    assert_eq!(b[0], b'q', "RT3 Fread(3)");
    DEV_B.feed(b"r");
    assert_eq!(bios(BCONIN, 0, 0), b'r' as i64, "RT3 Bconin(0)");
    assert_eq!(DEV_B.out(), b"pq", "echo on PRN");
    assert!(DEV_A.untouched(), "AUX entry not hit");
}

// ═══ Fwrite, CON ═════════════════════════════════════════════════════════

/// W1, W2: CON and the console standard handles write raw to the console.
#[test]
fn w01_w02_console_writes() {
    let (_g, host, _p) = setup();
    assert_eq!(fwrite(-1, b"hello"), 5, "W1");
    assert_eq!(host.take_output(), b"hello");
    for h in [1, 0, 4, 5] {
        assert_eq!(fwrite(h, b"abc"), 3, "W2 Fwrite({h})");
        assert_eq!(host.take_output(), b"abc");
    }
}

/// W3-W8: `tabout` — TAB to the next multiple of 8 of the tracked column,
/// CR resets it, no CR inserted for LF (TASK/CON01).
#[test]
fn w03_to_w08_tabs_follow_the_column() {
    let (_g, host, _p) = setup();
    let sp = |n: usize| vec![b' '; n];
    assert_eq!(fwrite(-1, b"\tX"), 2, "W3");
    assert_eq!(host.take_output(), [sp(8), b"X".to_vec()].concat());
    assert_eq!(fwrite(-1, b"\r"), 1);
    assert_eq!(fwrite(-1, b"ab\tX"), 4, "W4");
    assert_eq!(host.take_output(), [b"\rab".to_vec(), sp(6), b"X".to_vec()].concat());
    assert_eq!(fwrite(-1, b"\r12345"), 6);
    assert_eq!(fwrite(-1, b"\tX"), 2, "W5");
    assert_eq!(&host.take_output()[6..], [sp(3), b"X".to_vec()].concat());
    assert_eq!(fwrite(-1, b"\r1234567"), 8);
    let _ = host.take_output();
    assert_eq!(fwrite(-1, b"\t"), 1, "W6 (column 8 → at least one space)");
    assert_eq!(host.take_output(), sp(1));
    assert_eq!(fwrite(-1, b"\t"), 1, "W6 at column 8");
    assert_eq!(host.take_output(), sp(8));
    assert_eq!(fwrite(-1, b"a\nb"), 3, "W7");
    assert_eq!(host.take_output(), [0x61, 0x0A, 0x62]);
    assert_eq!(fwrite(-1, b"a\rb"), 3, "W8");
    assert_eq!(fwrite(-1, b"\t"), 1, "W8 tab");
    assert_eq!(host.take_output(), [b"a\rb".to_vec(), sp(7)].concat());
}

/// W9, W10, W11: `count <= 0` returns `count` itself; NULs are written.
#[test]
fn w09_w10_w11_counts_and_nuls() {
    let (_g, host, _p) = setup();
    let b = [0x41u8; 8];
    assert_eq!(fwrite_raw(-1, 0, b.as_ptr()), 0, "W9");
    assert_eq!(fwrite_raw(-1, -5, b.as_ptr()), -5, "W10");
    assert!(host.take_output().is_empty());
    assert_eq!(fwrite(-1, b"a\0b"), 3, "W11");
    assert_eq!(host.take_output(), [0x61, 0x00, 0x62]);
}

/// W12: a pending key is moved by the write's `conbrk` from the key ring into
/// the BDOS typeahead and returned by the next read (TASK/CON01).
#[test]
fn w12_pending_key_goes_to_the_typeahead() {
    let (_g, host, _p) = setup();
    host.type_keys(b"k");
    assert_eq!(fwrite(-1, b"x"), 1, "W12");
    assert!(host.pending_keys().is_empty(), "taken from the key ring");
    let mut b = [0u8; 1];
    assert_eq!(fread(-1, &mut b), 1);
    assert_eq!(b[0], b'k');
}

/// W13: a Ctrl-C at the front of the queue is consumed, terminate is
/// requested, nothing is written.
#[test]
fn w13_ctrl_c_terminates_before_the_first_byte() {
    let (_g, host, _p) = setup();
    host.type_keys(&[CTRL_C]);
    let _ = fwrite(-1, b"abc");
    assert_eq!(process::take_pending_terminate(), Some(-32), "W13");
    assert!(host.take_output().is_empty(), "W13 nothing written");
    assert!(host.pending_keys().is_empty(), "W13 key consumed");
}

/// W14: Ctrl-S pauses the write until Ctrl-Q; both are consumed, neither
/// stored (TASK/CON01; the pause itself is timed in
/// `con01_console_typeahead.rs`).
#[test]
fn w14_ctrl_s_pauses_until_ctrl_q() {
    let (_g, host, _p) = setup();
    host.type_keys(&[CTRL_S, CTRL_Q]);
    assert_eq!(fwrite(-1, b"abc"), 3, "W14");
    assert_eq!(host.take_output(), b"abc");
    assert!(host.pending_keys().is_empty());
    assert_eq!(gemdos0(CCONIS), 0, "nothing stored in the typeahead");
}

// ═══ Fwrite, AUX / PRN ═══════════════════════════════════════════════════

/// W15-W17: AUX (1) is "port present, no cable" (`3f-fread.md` item 1a,
/// option A): every `Bconout(1)` accepts and discards, so `Fwrite(-2|2, n)`
/// returns `n`; no tab expansion and no break check on non-CON (the queued
/// Ctrl-C stays). W18: PRN stays absent — the first `Bconout` returns 0, the
/// call returns the short count 0.
#[test]
fn w15_to_w18_aux_no_cable_and_absent_prn() {
    let (_g, host, _p) = setup();
    host.type_keys(&[CTRL_C]);
    for h in [-2, 2] {
        assert_eq!(fwrite(h, b"ATZ\r"), 4, "W15 Fwrite({h})");
    }
    assert_eq!(fwrite(-2, b"\tX"), 2, "W16");
    assert_eq!(fwrite(-2, b"ab"), 2, "W17");
    for h in [-3, 3] {
        assert_eq!(fwrite(h, b"abc"), 0, "W18 Fwrite({h})");
    }
    assert!(host.take_output().is_empty(), "nothing on CON");
    assert_eq!(host.pending_keys(), vec![CTRL_C as u16], "W17 Ctrl-C not consumed");
    assert_eq!(process::take_pending_terminate(), None, "W17 no terminate");
}

/// W19, W20, W21: short counts when the device refuses a byte.
#[test]
fn w19_w20_w21_short_counts() {
    let (_g, _host, _p) = setup();
    DEV_B.reset(b"", 2);
    let _prn = Installed::new(0, &DEV_B_REF);
    assert_eq!(fwrite(-3, b"abcde"), 2, "W19");
    assert_eq!(DEV_B.out(), b"ab");
    DEV_B.reset(b"", 0);
    assert_eq!(fwrite(-3, b"abcde"), 0, "W20");
    assert!(DEV_B.out().is_empty());

    DEV_A.reset(b"", 1);
    let _aux = Installed::new(1, &DEV_A_REF);
    assert_eq!(fwrite(-2, b"xyz"), 1, "W21");
    assert_eq!(DEV_A.out(), b"x");
}

/// W22: `count <= 0` returns `count` on every device.
#[test]
fn w22_counts_on_absent_devices() {
    let (_g, _host, _p) = setup();
    let b = [0u8; 4];
    assert_eq!(fwrite_raw(-2, 0, b.as_ptr()), 0, "W22 AUX 0");
    assert_eq!(fwrite_raw(-3, -1, b.as_ptr()), -1, "W22 PRN -1");
}

/// W23, W24: invalid handles.
#[test]
fn w23_w24_invalid_handles() {
    let (_g, _host, _p) = setup();
    assert_eq!(fwrite(-4, b"x"), EIHNDL, "W23 -4");
    assert_eq!(fwrite(-32768, b"x"), EIHNDL, "W23 -32768");
    assert_eq!(fwrite(6 + 15, b"x"), EIHNDL, "W24 free slot");
}

/// W25: `Fdup(2)` is AUX (no-cable entry accepts: 2); closed → `EIHNDL`.
#[test]
fn w25_fdup_of_aux() {
    let (_g, host, _p) = setup();
    let d = fdup(2);
    assert!(d >= 6, "Fdup(2) -> {d}");
    assert_eq!(fwrite(d, b"ok"), 2, "W25");
    assert_eq!(fclose(d), E_OK);
    assert_eq!(fwrite(d, b"ok"), EIHNDL, "W25 after Fclose");
    assert!(host.take_output().is_empty(), "nothing on CON");
}

/// W26: `Fforce(1, -3)` sends stdout to the (absent) printer.
#[test]
fn w26_fforce_stdout_to_prn() {
    let (_g, host, p) = setup();
    assert_eq!(fforce(1, -3), E_OK);
    assert_eq!(fwrite(1, b"ok"), 0, "W26");
    assert!(host.take_output().is_empty());
    assert_eq!(fclose(1), E_OK);
    assert_eq!(p.uft(), DEFAULT);
}

/// W27: stdout forced to a file writes the file; `Fclose(1)` reverts to CON.
#[test]
fn w27_fforce_stdout_to_a_file_and_back() {
    let (_g, host, p) = setup();
    let fh = fcreate("C:\\FS11W27.TXT");
    assert_eq!(fforce(1, fh), E_OK);
    assert_eq!(fwrite(1, b"ok"), 2, "W27 into the file");
    assert!(host.take_output().is_empty());
    let _ = fclose(1);
    assert_eq!(p.uft(), DEFAULT);
    assert_eq!(fwrite(1, b"x"), 1, "W27 CON again");
    assert_eq!(host.take_output(), b"x");
    let _ = fclose(fh);
    let r = fopen("C:\\FS11W27.TXT");
    let mut b = [0u8; 8];
    assert_eq!(fread(r, &mut b), 2);
    assert_eq!(&b[..2], b"ok");
    assert_eq!(fclose(r), E_OK);
}

/// W28 (regression guard, FS11 acceptance finding 1): a default process with
/// **no** `Fforce` writes to handle 2 (stderr = AUX). Every write returns its
/// full count — a 0 made mintlib's write loop retry forever (the pre-
/// LIBC-MAIN stderr hang). Nothing reaches CON.
#[test]
fn w28_fwrite_2_without_fforce_returns_count() {
    let (_g, host, p) = setup();
    assert_eq!(p.uft(), DEFAULT, "no Fforce");
    assert_eq!(fwrite(2, b"hello"), 5, "W28 5");
    assert_eq!(fwrite(2, b"x"), 1, "W28 1");
    let buf = vec![0x5Au8; 0x1000];
    assert_eq!(fwrite(2, &buf), 0x1000, "W28 0x1000");
    assert!(host.take_output().is_empty(), "nothing on CON");
}

// ═══ Fwrite, single dispatch point ═══════════════════════════════════════

/// WT1, WT2: the AUX entry is reached by `Fwrite(-2|2)`, an `Fdup(2)` handle,
/// `Cauxout`, `Bconout(1)`; `Bcostat(1)` / `Cauxos` read its status. Bytes go
/// out raw (no tab expansion).
#[test]
fn wt1_wt2_aux_entry_is_the_one_dispatch_point() {
    let (_g, host, _p) = setup();
    DEV_A.reset(b"", usize::MAX);
    let _aux = Installed::new(1, &DEV_A_REF);
    assert_eq!(fwrite(-2, b"ATZ\r"), 4, "WT1 Fwrite(-2)");
    assert_eq!(DEV_A.out(), [0x41, 0x54, 0x5A, 0x0D]);
    assert_eq!(fwrite(2, b"AT"), 2, "WT1 Fwrite(2)");
    assert_eq!(DEV_A.out(), b"AT");
    let d = fdup(2);
    assert_eq!(fwrite(d, b"dd"), 2, "WT1 Fdup(2) handle");
    assert_eq!(fclose(d), E_OK);
    assert_eq!(DEV_A.out(), b"dd");
    assert_eq!(call(CAUXOUT, [b'x' as u64, 0, 0, 0, 0, 0, 0, 0]), 1, "WT1 Cauxout");
    assert_eq!(bios(BCONOUT, 1, b'y' as u64), 1, "WT1 Bconout(1)");
    assert_eq!(DEV_A.out(), b"xy");
    assert_eq!(bios(BCOSTAT, 1, 0), -1, "WT1 Bcostat(1)");
    assert_eq!(gemdos0(CAUXOS), -1, "WT1 Cauxos");

    assert_eq!(fwrite(-2, b"\tX"), 2, "WT2");
    assert_eq!(DEV_A.out(), [0x09, 0x58]);
    assert!(host.take_output().is_empty(), "CON untouched");
}

/// WT3: the PRN entry (index 0) is reached by `Fwrite(-3|3)`, `Cprnout`,
/// `Bconout(0)`, and `Cprnos` reads its status; never the AUX entry.
#[test]
fn wt3_prn_entry_is_index_0() {
    let (_g, host, _p) = setup();
    DEV_A.reset(b"", usize::MAX);
    DEV_B.reset(b"", usize::MAX);
    let _aux = Installed::new(1, &DEV_A_REF);
    let _prn = Installed::new(0, &DEV_B_REF);
    assert_eq!(fwrite(-3, b"abc"), 3, "WT3 Fwrite(-3)");
    assert_eq!(fwrite(3, b"de"), 2, "WT3 Fwrite(3)");
    assert_eq!(call(CPRNOUT, [b'f' as u64, 0, 0, 0, 0, 0, 0, 0]), 1, "WT3 Cprnout");
    assert_eq!(bios(BCONOUT, 0, b'g' as u64), 1, "WT3 Bconout(0)");
    assert_eq!(DEV_B.out(), b"abcdefg");
    assert_eq!(gemdos0(CPRNOS), -1, "WT3 Cprnos reads entry 0");
    assert!(DEV_A.untouched(), "AUX entry not hit");
    assert!(host.take_output().is_empty());
}

// ═══ BIOS / GEMDOS / XBIOS on the absent profile ═════════════════════════

/// The device profile per BIOS device (`docs/api/bios/{01,02,03,08}`): PRN
/// is absent and answers 0 everywhere; AUX is "port present, no cable"
/// (`Bconout` 1, `Bcostat` -1, `Bconstat`/`Bconin` 0 without blocking), and
/// handle 6 is the one-entry `Bconmap` table's AUX (BIOS08,
/// `tests/bios08_bconmap_handles.rs`); MIDI and IKBD accept and discard
/// output and are ready (BIOS04; `Bconin(3)` waits for the `Iorec(2)` ring,
/// `tests/bios04_midi_ikbd.rs`); CON is the console; raw-CON writes; handles
/// past the `Bconmap` table (7 and up) and negative handles are 0.
#[test]
fn bios_absent_profile() {
    let (_g, host, _p) = setup();
    for dev in [0u64, 7, 8, 0xFFFF] {
        assert_eq!(bios(BCONSTAT, dev, 0), 0, "Bconstat({dev})");
        assert_eq!(bios(BCONIN, dev, 0), 0, "Bconin({dev})");
        assert_eq!(bios(BCONOUT, dev, b'x' as u64), 0, "Bconout({dev})");
        assert_eq!(bios(BCOSTAT, dev, 0), 0, "Bcostat({dev})");
    }
    for dev in [1u64, 6] {
        assert_eq!(bios(BCONSTAT, dev, 0), 0, "Bconstat({dev}) AUX no cable");
        assert_eq!(bios(BCONIN, dev, 0), 0, "Bconin({dev}) AUX non-blocking");
        assert_eq!(bios(BCONOUT, dev, b'x' as u64), 1, "Bconout({dev}) AUX accepts");
        assert_eq!(bios(BCOSTAT, dev, 0), -1, "Bcostat({dev}) AUX ready");
    }
    assert_eq!(bios(BCONSTAT, 3, 0), 0, "Bconstat(3) MIDI, empty ring");
    assert_eq!(bios(BCONOUT, 3, b'x' as u64), 1, "Bconout(3) MIDI accepts");
    assert_eq!(bios(BCOSTAT, 3, 0), -1, "Bcostat(3) IKBD slot ready");
    assert_eq!(bios(BCONSTAT, 4, 0), 0, "Bconstat(4) char_dummy");
    assert_eq!(bios(BCONIN, 4, 0), 0, "Bconin(4) char_dummy, no wait");
    assert_eq!(bios(BCONOUT, 4, b'x' as u64), 1, "Bconout(4) IKBD accepts");
    assert_eq!(bios(BCOSTAT, 4, 0), -1, "Bcostat(4) MIDI slot ready");
    assert!(host.take_output().is_empty(), "absent/AUX/MIDI/IKBD output discarded");
    assert_eq!(bios(BCOSTAT, 2, 0), -1, "Bcostat(2)");
    assert_eq!(bios(BCOSTAT, 5, 0), 0, "Bcostat(5) char_dummy");
    assert_eq!(bios(BCONOUT, 2, b'c' as u64), 1);
    assert_eq!(bios(BCONOUT, 5, b'r' as u64), 1);
    // Raw CON is the screen's `ascii_out`, not the console device: it does
    // not pass the VT52 console output (`docs/api/bios/_vt52.md` §9, CON04
    // decision D5).
    assert_eq!(host.take_output(), b"c");
    assert_eq!(bios(BCONSTAT, 2, 0), 0);
    host.type_keys(b"m");
    assert_eq!(bios(BCONSTAT, 2, 0), -1, "Bconstat(2)");
    assert_eq!(bios(BCONIN, 2, 0), b'm' as i64, "Bconin(2)");
    assert_eq!(bios(BCONSTAT, 5, 0), 0, "Bconstat(5)");
}

/// The GEMDOS AUX/PRN calls (`docs/api/gemdos/console/{03,04,05,11,12,13}`):
/// AUX is the no-cable entry (`Cauxout` 1, `Cauxos` -1, `Cauxin`/`Cauxis` 0),
/// PRN is absent (all 0).
#[test]
fn gemdos_aux_prn_absent_profile() {
    let (_g, host, _p) = setup();
    assert_eq!(gemdos0(CAUXIN), 0, "Cauxin");
    assert_eq!(call(CAUXOUT, [b'x' as u64, 0, 0, 0, 0, 0, 0, 0]), 1, "Cauxout");
    assert_eq!(call(CPRNOUT, [b'x' as u64, 0, 0, 0, 0, 0, 0, 0]), 0, "Cprnout");
    assert_eq!(gemdos0(CPRNOS), 0, "Cprnos");
    assert_eq!(gemdos0(CAUXIS), 0, "Cauxis");
    assert_eq!(gemdos0(CAUXOS), -1, "Cauxos");
    assert!(host.take_output().is_empty());
}

/// `Cconout` / `Cconws` reach the console through the CON entry; a device in
/// the CON slot sees them (`docs/api/gemdos/console/{02,09}`).
#[test]
fn cconout_cconws_go_through_the_con_entry() {
    let (_g, host, _p) = setup();
    assert_eq!(call(CCONOUT, [b'a' as u64, 0, 0, 0, 0, 0, 0, 0]), 1);
    let s = cpath("bc");
    assert_eq!(call(CCONWS, [s.as_ptr() as u64, 0, 0, 0, 0, 0, 0, 0]), 2);
    assert_eq!(host.take_output(), b"abc");
    DEV_A.reset(b"", usize::MAX);
    {
        let _con = Installed::new(2, &DEV_A_REF);
        assert_eq!(call(CCONOUT, [b'z' as u64, 0, 0, 0, 0, 0, 0, 0]), 1);
        assert_eq!(fwrite(-1, b"yy"), 2);
    }
    assert_eq!(DEV_A.out(), b"zyy");
    assert!(host.take_output().is_empty());
}

/// The absent entry, to make the other slot of the swap distinguishable
/// (since BIOS04 both default entries are ready, -1).
static ABSENT_REF: &dyn CharDevice = &tos_api::bios::chardev::Absent;

/// The `Bcostat` IKBD/MIDI slot swap (`bios/chardev.c:38-42`): `Bcostat(3)`
/// reads the IKBD entry (4), `Bcostat(4)` the MIDI entry (3).
#[test]
fn bcostat_keeps_the_ikbd_midi_swap() {
    let (_g, _host, _p) = setup();
    DEV_A.reset(b"", usize::MAX);
    {
        let _ikbd = Installed::new(4, &DEV_A_REF);
        let _midi = Installed::new(3, &ABSENT_REF);
        assert_eq!(bios(BCOSTAT, 3, 0), -1, "Bcostat(3) -> IKBD entry");
        assert_eq!(bios(BCOSTAT, 4, 0), 0, "Bcostat(4) -> MIDI entry");
    }
    {
        let _midi = Installed::new(3, &DEV_A_REF);
        let _ikbd = Installed::new(4, &ABSENT_REF);
        assert_eq!(bios(BCOSTAT, 4, 0), -1, "Bcostat(4) -> MIDI entry");
        assert_eq!(bios(BCOSTAT, 3, 0), 0, "Bcostat(3) -> IKBD entry");
        // Bconout is not swapped.
        assert_eq!(bios(BCONOUT, 3, b'm' as u64), 1, "Bconout(3) -> MIDI entry");
    }
    assert_eq!(DEV_A.out(), b"m");
}

/// `Midiws(cnt, ptr)` sends `cnt + 1` bytes (unsigned) from `ptr` through the
/// MIDI entry (`docs/api/xbios/0c-midiws.md`, FS11 note).
#[test]
fn midiws_sends_cnt_plus_one_bytes_through_the_midi_entry() {
    let (_g, host, _p) = setup();
    let data = b"abcdef";
    assert_eq!(xbios(MIDIWS, 2, data.as_ptr() as u64), 0, "MIDI port: void");
    DEV_A.reset(b"", usize::MAX);
    {
        let _midi = Installed::new(3, &DEV_A_REF);
        assert_eq!(xbios(MIDIWS, 2, data.as_ptr() as u64), 0);
        assert_eq!(DEV_A.out(), b"abc", "cnt 2 -> 3 bytes");
        assert_eq!(xbios(MIDIWS, 0, data.as_ptr() as u64), 0);
        assert_eq!(DEV_A.out(), b"a", "cnt 0 -> 1 byte");
        let big = vec![0x5Au8; 0x10000];
        assert_eq!(xbios(MIDIWS, 0xFFFF, big.as_ptr() as u64), 0);
        assert_eq!(DEV_A.out().len(), 0x10000, "cnt 0xFFFF -> 65536 bytes");
    }
    assert!(host.take_output().is_empty());
}

/// `Fseek` on device handles still answers 0 (FS10, unchanged by the device
/// table).
#[test]
fn fseek_on_devices_unchanged() {
    let (_g, _host, _p) = setup();
    for h in [-1i64, -2, -3, 0, 1, 2, 3] {
        assert_eq!(call(FSEEK, [0, h as u64, 1, 0, 0, 0, 0, 0]), 0, "Fseek(0, {h}, 1)");
    }
}
