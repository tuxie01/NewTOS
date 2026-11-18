//! Phase 10 XBIOS screen-group self-test: exercises Physbase / Logbase /
//! Getrez / Setscreen / Setcolor over the real `svc #14` trap path.

use crate::kprintln;

/// Generic XBIOS call: `svc #14`, function id in `x8`, up to six args in
/// `x0..x5`, result in `x0`.
fn xbios6(func: u16, a: [u64; 6]) -> i64 {
    let r: i64;
    // SAFETY: XBIOS trap ABI; results come back in x0.
    unsafe {
        core::arch::asm!(
            "svc #14",
            in("x8") func as u64,
            inout("x0") a[0] => r,
            in("x1") a[1],
            in("x2") a[2],
            in("x3") a[3],
            in("x4") a[4],
            in("x5") a[5],
            options(nostack),
        );
    }
    r
}

/// Whole-sector big-endian word checksum (mirror of the on-device
/// `compute_cksum`, independent reimplementation for the assert).
fn cksum(buf: &[u8; 512]) -> u16 {
    let mut sum: u16 = 0;
    let mut i = 0;
    while i < 512 {
        sum = sum.wrapping_add(((buf[i] as u16) << 8) | (buf[i + 1] as u16));
        i += 2;
    }
    sum
}

/// XBIOS audit self-test (Task: XBIOS code-audit fixes). Drives the trap
/// path directly and asserts the *contract value*, not just that a call
/// returns. Does not require a framebuffer, so it runs early in boot.
pub fn run_audit() {
    kprintln!("[xbios-audit] === XBIOS trap-14 contract test ===");
    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[xbios-audit]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // (a) Dispatcher convention: unimplemented slots return the function
    //     number itself (never EINVFN -32), for both an in-range padded slot
    //     and an out-of-range id.
    let mfpint = xbios6(0x0D, [0, 0, 0, 0, 0, 0]); // Mfpint: unimplemented in-range
    check(mfpint == 0x0D, "unimpl in-range Xbios(0x0D) returns func# 13");
    let oor = xbios6(0x60, [0, 0, 0, 0, 0, 0]); // out of range (> LAST_ENTRY 0x57)
    check(oor == 0x60, "out-of-range Xbios(0x60) returns func# 96");

    // (b) Floppy/DMA group → EUNDEV (-15), the "device not present" contract
    //     (Floprd/Flopwr shown; the rest share the arm).
    let floprd = xbios6(0x08, [0, 0, 0, 1, 0, 0]);
    check(floprd == -15, "Floprd(0x08) -> EUNDEV (-15)");
    let flopwr = xbios6(0x09, [0, 0, 0, 1, 0, 0]);
    check(flopwr == -15, "Flopwr(0x09) -> EUNDEV (-15)");
    let dmard = xbios6(0x2A, [1, 1, 0, 0, 0, 0]);
    check(dmard == -15, "DMAread(0x2A) -> EUNDEV (-15)");

    // (c) Protobt (0x12): build a bootable + a non-bootable sector and verify
    //     the whole-sector checksum (0x1234 / 0x1235) and the copied BPB.
    let mut sec = [0u8; 512];
    let ptr = sec.as_mut_ptr() as u64;
    // disktype 1 = 720K DS, serialno 0x123456, execflag 1 (bootable).
    xbios6(0x12, [ptr, 0x0012_3456, 1, 1, 0, 0]);
    let sum_exec = cksum(&sec);
    check(sum_exec == 0x1234, "Protobt(exec) checksum == 0x1234");
    // BPB copied little-endian at 0x0B..; serial little-endian at 0x08..
    check(sec[0x08] == 0x56 && sec[0x09] == 0x34 && sec[0x0A] == 0x12, "Protobt serial LE 0x123456");
    check(sec[0x0B] == 0x00 && sec[0x0C] == 0x02, "Protobt BPB bps=512");
    check(sec[0x0D] == 0x02, "Protobt BPB spc=2");
    check(sec[0x13] == 0xD0 && sec[0x14] == 0x02, "Protobt BPB sec=720");
    check(sec[0x15] == 0xFD, "Protobt BPB media=0xFD");

    let mut sec2 = [0u8; 512];
    let ptr2 = sec2.as_mut_ptr() as u64;
    // Same geometry, execflag 0 (non-bootable) → checksum 0x1235.
    xbios6(0x12, [ptr2, 0x0012_3456, 1, 0, 0, 0]);
    let sum_nonexec = cksum(&sec2);
    check(sum_nonexec == 0x1235, "Protobt(non-exec) checksum == 0x1235");

    // (d) Wired contract functions return their contract value.
    // Kbrate (0x23): query returns default 0x0F02; set 20/3, get old, re-query.
    let kb_q = xbios6(0x23, [0xFFFF, 0xFFFF, 0, 0, 0, 0]); // Kbrate(-1,-1)
    check(kb_q == 0x0F02, "Kbrate(-1,-1) query = 0x0F02 (defaults 15/2)");
    let kb_old = xbios6(0x23, [20, 3, 0, 0, 0, 0]); // set both, get old
    check(kb_old == 0x0F02, "Kbrate(20,3) returns old 0x0F02");
    let kb_new = xbios6(0x23, [0xFFFF, 0xFFFF, 0, 0, 0, 0]);
    check(kb_new == 0x1403, "Kbrate re-query = 0x1403 (20<<8|3)");

    // Rsconf (0x0F) query: speed==-2 returns the current baud code (B9600=1).
    let rs_q = xbios6(0x0F, [0xFFFE, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]);
    check(rs_q == 1, "Rsconf(-2,...) query baud = 1 (B9600)");
    // Rsconf set-and-read-back the synthesised packed old registers: default
    // old = (UCR 0x88 << 24)|(RSR 1 << 16)|(TSR 1 << 8) = 0x88010100.
    let rs_old = xbios6(0x0F, [1, 0, 0x8A, 0xFFFF, 0xFFFF, 0xFFFF]); // set baud 1, ucr 0x8A
    check(rs_old == 0x8801_0100, "Rsconf packed old regs = 0x88010100");
    let rs_old2 = xbios6(0x0F, [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]);
    check(rs_old2 == 0x8A01_0100, "Rsconf UCR round-trip = 0x8A010100");

    // Giaccess (0x1C): no YM2149 → 0 (not the function number).
    let gia = xbios6(0x1C, [0, 14, 0, 0, 0, 0]);
    check(gia == 0, "Giaccess -> 0 (no PSG)");
    // Ikbdws (0x19): void no-op → 0 (not func#).
    let ikw = xbios6(0x19, [0, ptr, 0, 0, 0, 0]);
    check(ikw == 0, "Ikbdws -> 0 (void no-op)");

    // Setprt (0x21): pure-software config word — query default 0, set 0x000A,
    // get old, re-query round-trip (not the function number 33).
    let sp_def = xbios6(0x21, [0xFFFF, 0, 0, 0, 0, 0]); // Setprt(-1) query
    check(sp_def == 0, "Setprt(-1) query = 0 (default)");
    let sp_old = xbios6(0x21, [0x000A, 0, 0, 0, 0, 0]); // set, get old
    check(sp_old == 0, "Setprt(0x000A) returns old 0");
    let sp_new = xbios6(0x21, [0xFFFF, 0, 0, 0, 0, 0]);
    check(sp_new == 0x000A, "Setprt re-query = 0x000A (round-trip)");

    kprintln!("[xbios-audit] === 23 checks, {} FAIL ===", fails);
}

/// Run the XBIOS screen-group self-test (requires a framebuffer).
pub fn run() {
    if !crate::vdi_backend::available() {
        return;
    }
    let phys0 = physbase();
    let log0 = logbase();
    let rez = getrez();
    kprintln!(
        "[xbios] Physbase=0x{:x} Logbase=0x{:x} Getrez={} (expect base!=0, log==phys, rez=2 ST_HIGH)",
        phys0, log0, rez,
    );

    // Setscreen now moves the *real* bases, so the self-test exercises what
    // the display can actually do: how many whole screens its memory holds,
    // and whether the logical base really lands on the second one.
    let page = crate::vdi_backend::size_bytes() as u64;
    let pages = crate::vdi_backend::mem_bytes() as u64 / page.max(1);
    kprintln!(
        "[xbios] screen memory: {} page(s) of {} bytes (expect 2 on the VideoCore, 1 on ramfb)",
        pages, page,
    );

    if pages > 1 {
        // Draw into the hidden page: Logbase moves, Physbase does not.
        let hidden = log0 + page as i64;
        let r = setscreen(hidden, -1, -1);
        let log1 = logbase();
        let phys1 = physbase();
        kprintln!(
            "[xbios] Setscreen(log=0x{:x}) ret={} -> Logbase=0x{:x} (==log+page) Physbase=0x{:x} (unchanged 0x{:x})",
            hidden, r, log1, phys1, phys0,
        );
        // Show it, then put both back where the console expects them.
        setscreen(-1, hidden, -1);
        kprintln!(
            "[xbios] Setscreen(phys=0x{:x}) -> Physbase=0x{:x} (==phys+page)",
            hidden,
            physbase(),
        );
        setscreen(log0, phys0, -1);
    }

    // A base the display cannot show is refused, and the read-back keeps
    // telling the truth rather than echoing the request (EmuTOS `physbase()`
    // likewise reads the hardware, `bios/screen.c:1030`).
    let fake = 0x4200_0000u64;
    setscreen(fake as i64, fake as i64, -1);
    kprintln!(
        "[xbios] Setscreen(0x{:x}) off-screen -> Logbase=0x{:x} Physbase=0x{:x} (both unchanged)",
        fake,
        logbase(),
        physbase(),
    );

    // Setscreen rez is accepted-but-ignored (no in-system switch, ADR-017 rev 2).
    let bad = setscreen(-1, -1, 9);
    kprintln!("[xbios] Setscreen(rez=9) -> {} (expect 0, ignored)", bad);

    // Setcolor: set entry 3, then query it back.
    let old = setcolor(3, 0x0700);
    let got = setcolor(3, -1);
    kprintln!(
        "[xbios] Setcolor(3,0x700) old={} ; query=0x{:x} (expect 0x700)",
        old, got,
    );

    // Phase 11: Random / Vsync / Kbdvbase / Tickcal.
    let r1 = random();
    let r2 = random();
    kprintln!(
        "[xbios] Random -> 0x{:06x}, 0x{:06x} ({}; both 24-bit)",
        r1, r2,
        if r1 != r2 && r1 >> 24 == 0 && r2 >> 24 == 0 { "OK" } else { "FAIL" },
    );
    let kb = kbdvbase();
    kprintln!("[xbios] Kbdvbase -> 0x{:x} ({})", kb, if kb != 0 { "OK" } else { "FAIL" });
    let tc = tickcal();
    kprintln!("[xbios] Tickcal -> {} (expect 20)", tc);
    // Vsync should block ~1 frame each and return (not hang).
    vsync();
    vsync();
    kprintln!("[xbios] Vsync x2 returned (did not hang) OK");

    // Settime/Gettime clock round-trip (shares the GEMDOS wall clock). Set
    // 2021-06-15 14:30:20 as packed date<<16 | time, read it straight back.
    let date = (((2021 - 1980) << 9) | (6 << 5) | 15) as u32; // 0x52CF
    let time = ((14 << 11) | (30 << 5) | (20 / 2)) as u32; // 0x73CA
    let packed = (date << 16) | time;
    settime(packed as i64);
    let got = gettime() as u32;
    kprintln!(
        "[xbios] Settime(2021-06-15 14:30:20) -> Gettime=0x{:08x} (expect 0x{:08x}) {}",
        got, packed,
        if got == packed { "OK" } else { "FAIL" },
    );
}

/// XBIOS `Random` (0x11): no args.
fn random() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Random ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x11u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Settime` (0x16): x0=packed datetime (date<<16 | time). Void.
fn settime(packed: i64) {
    // SAFETY: svc #14, XBIOS Settime ABI.
    unsafe {
        core::arch::asm!("svc #14", in("x8") 0x16u64, in("x0") packed as u64, lateout("x0") _, options(nostack));
    }
}

/// XBIOS `Gettime` (0x17): no args. Returns the packed datetime.
fn gettime() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Gettime ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x17u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Kbdvbase` (0x22): no args.
fn kbdvbase() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Kbdvbase ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x22u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Vsync` (0x25): no args.
fn vsync() {
    // SAFETY: svc #14, XBIOS Vsync ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x25u64, lateout("x0") _, options(nostack)); }
}

/// BIOS `Tickcal` (0x06) via svc #13: no args.
fn tickcal() -> i64 {
    let r: i64;
    // SAFETY: svc #13, BIOS Tickcal ABI.
    unsafe { core::arch::asm!("svc #13", in("x8") 0x06u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Physbase` (0x02): no args.
fn physbase() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Physbase ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x02u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Logbase` (0x03): no args.
fn logbase() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Logbase ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x03u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Getrez` (0x04): no args.
fn getrez() -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Getrez ABI.
    unsafe { core::arch::asm!("svc #14", in("x8") 0x04u64, out("x0") r, options(nostack)); }
    r
}

/// XBIOS `Setscreen` (0x05): x0=logLoc, x1=physLoc, x2=rez, x3=videl.
fn setscreen(log: i64, phys: i64, rez: i16) -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Setscreen ABI.
    unsafe {
        core::arch::asm!("svc #14", in("x8") 0x05u64, inout("x0") log => r, in("x1") phys, in("x2") rez as u64, in("x3") 0u64, options(nostack));
    }
    r
}

/// XBIOS `Setcolor` (0x07): x0=index, x1=value (-1 to query). Old value.
fn setcolor(index: u16, value: i16) -> i64 {
    let r: i64;
    // SAFETY: svc #14, XBIOS Setcolor ABI.
    unsafe {
        core::arch::asm!("svc #14", in("x8") 0x07u64, inout("x0") index as u64 => r, in("x1") value as u64, options(nostack));
    }
    r
}
