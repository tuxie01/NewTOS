//! File-system I/O tracer (TASK/FS26) — where the time of a GEMDOS call goes,
//! down to the card.
//!
//! A debug instrument, compiled only with the kernel's cargo feature
//! `fs-io-trace` (the module is `#[cfg]`-gated in `lib.rs`, its hook sites in
//! the `svc` handler and in the rpi4 block device likewise), so a build
//! without the feature does not contain a byte of it.
//!
//! ## What it records
//!
//! For every `svc` the handler dispatches it takes the time and the I/O
//! counters before and after the call; for the outermost call (a nested `svc`
//! is counted into it) it prints, after the call completed:
//!
//! ```text
//! [fsio] 000042 U1 gemdos 0x39 t=8123456 dur=1234 ret=0 path="C:\FS26_1" | dev r=0/0/0 w=19/19/2345 | bc hit=40 miss=0 | bdos hit=51 miss=24 fat=1 data=23 | fatlk=3 | gap vdi=0/0 bios=2/10 aes=0/0 out=0/0 ovh=35
//! [fsio]   W lba=8259 n=1 us=123
//! ```
//!
//! | Field | Meaning |
//! |---|---|
//! | `000042` | running number of the printed line |
//! | `U1` / `K` | caller: `U<slot>` an EL0 process, `K` an `svc` from EL1 |
//! | `gemdos 0x39` / `aes 25` / `bios 0x04` | trap class and function (AES: `control[0]`) |
//! | `t=` | `CNTPCT_EL0` at call entry, in µs since the counter started |
//! | `dur=` | call entry to call end, µs |
//! | `how=` | only when the call did not complete: `yield` / `stall` (AES) |
//! | `path=` | the path argument of a path-taking GEMDOS call, read after the call (the call itself read it first) |
//! | `dev r=a/b/c` | device **read** requests / sectors / µs inside the call — the rpi4 `KernelBlock`, below the FS09 block cache: what reaches the SD driver |
//! | `dev w=a/b/c` | the same for writes |
//! | `bc hit= miss=` | FS09 block-cache sectors served from RAM / read from the device ([`tos_api::bios::bcache::stats`]) |
//! | `bdos hit= miss= fat= data=` | BDOS buffer lists (`bdos/fs/cache.rs`): hits, misses, misses per list |
//! | `fatlk=` | FAT entries looked up ([`tos_api::bdos::fs::fat::fat_lookups`]) |
//! | `gap …=n/us` | calls **not** printed since the previous line (VDI, BIOS/XBIOS without device I/O), count / µs inside them; `out=` device requests outside any call (program loading, which never reaches the dispatcher) |
//! | `ovh=` | µs the tracer spent printing the previous line(s): subtract it from a wall-clock span |
//!
//! One `[fsio]   R|W lba= n= us=` line follows per device request of the call
//! (at most [`MAX_REQ`]; `+N more` when there were more).
//!
//! Printed are every GEMDOS and AES call, and any other call that reached the
//! device. Everything else is folded into the next line's `gap`.
//!
//! ## Why the console
//!
//! Like `bios-arg-trace` (`dbg_biostrace`): the instrument must not write the
//! drive it measures. QEMU captures it with `-append serial=on -serial file:…`.
//! The printing happens after the call's end time was taken, so `dur` does not
//! contain it; `ovh` reports what it costs between calls.
//!
//! Single CPU, IRQ-off trap path (the device is reached only from there): no
//! locking.

use core::cell::UnsafeCell;
use core::fmt::Write;

use tos_api::trap::{TrapArgs, TrapClass};
use tos_api::uaccess::Space;

/// Device requests listed per printed call.
pub const MAX_REQ: usize = 96;

#[derive(Clone, Copy)]
struct Req {
    write: bool,
    ok: bool,
    lba: u64,
    n: u32,
    ticks: u64,
}

const NO_REQ: Req = Req { write: false, ok: true, lba: 0, n: 0, ticks: 0 };

/// Counter snapshot taken at call entry.
#[derive(Clone, Copy, Default)]
struct Snap {
    bc_hit: u64,
    bc_miss: u64,
    bdos_hit: u32,
    bdos_miss: u32,
    lm_fat: u32,
    lm_data: u32,
    fatlk: u32,
}

impl Snap {
    fn take() -> Snap {
        let (bc_hit, bc_miss) = tos_api::bios::bcache::stats()
            .map(|s| (s.hit_sectors, s.miss_sectors))
            .unwrap_or((0, 0));
        let (bdos_hit, bdos_miss) = tos_api::bdos::fs::cache::stats();
        let (lm_fat, lm_data) = tos_api::bdos::fs::cache::list_misses();
        Snap {
            bc_hit,
            bc_miss,
            bdos_hit,
            bdos_miss,
            lm_fat,
            lm_data,
            fatlk: tos_api::bdos::fs::fat::fat_lookups(),
        }
    }
}

/// Requests / sectors / ticks of one direction.
#[derive(Clone, Copy, Default)]
struct Dir {
    reqs: u32,
    sectors: u32,
    ticks: u64,
}

struct State {
    depth: u32,
    t0: u64,
    snap: Snap,
    rd: Dir,
    wr: Dir,
    reqs: [Req; MAX_REQ],
    nreq: usize,
    more: u32,
    /// Not printed since the last line: `[vdi, bios+xbios, aes, gemdos]`
    /// calls and their ticks.
    gap_n: [u32; 4],
    gap_t: [u64; 4],
    /// Device requests outside any call since the last line.
    out_n: u32,
    out_t: u64,
    /// Ticks spent printing since the last line.
    ovh: u64,
    seq: u32,
}

struct Cell(UnsafeCell<State>);
// SAFETY: touched only from the single-CPU, IRQ-off trap path.
unsafe impl Sync for Cell {}

static STATE: Cell = Cell(UnsafeCell::new(State {
    depth: 0,
    t0: 0,
    snap: Snap {
        bc_hit: 0,
        bc_miss: 0,
        bdos_hit: 0,
        bdos_miss: 0,
        lm_fat: 0,
        lm_data: 0,
        fatlk: 0,
    },
    rd: Dir { reqs: 0, sectors: 0, ticks: 0 },
    wr: Dir { reqs: 0, sectors: 0, ticks: 0 },
    reqs: [NO_REQ; MAX_REQ],
    nreq: 0,
    more: 0,
    gap_n: [0; 4],
    gap_t: [0; 4],
    out_n: 0,
    out_t: 0,
    ovh: 0,
    seq: 0,
}));

fn state() -> &'static mut State {
    // SAFETY: single trap path; no borrow outlives a hook.
    unsafe { &mut *STATE.0.get() }
}

/// `CNTPCT_EL0`.
#[inline(always)]
pub fn ticks() -> u64 {
    let c: u64;
    // SAFETY: read-only system-register read, permitted at EL1.
    unsafe {
        core::arch::asm!("mrs {}, cntpct_el0", out(reg) c, options(nomem, nostack, preserves_flags));
    }
    c
}

fn freq() -> u64 {
    let f: u64;
    // SAFETY: read-only system-register read, permitted at EL1.
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack, preserves_flags));
    }
    if f == 0 { 1 } else { f }
}

fn us(t: u64) -> u64 {
    ((t as u128) * 1_000_000 / freq() as u128) as u64
}

/// Called by the `svc` handler right before the dispatch.
pub fn begin(_class: TrapClass, _args: &TrapArgs) {
    let s = state();
    s.depth += 1;
    if s.depth > 1 {
        return;
    }
    s.snap = Snap::take();
    s.rd = Dir::default();
    s.wr = Dir::default();
    s.nreq = 0;
    s.more = 0;
    s.t0 = ticks();
}

/// Called by the rpi4 block device for every request it serves, after the
/// request: `t0` is [`ticks`] before it.
pub fn device(write: bool, lba: u64, n: u32, t0: u64, ok: bool) {
    let t = ticks().wrapping_sub(t0);
    let s = state();
    if s.depth == 0 {
        s.out_n += 1;
        s.out_t += t;
        return;
    }
    let d = if write { &mut s.wr } else { &mut s.rd };
    d.reqs += 1;
    d.sectors += n;
    d.ticks += t;
    if s.nreq < MAX_REQ {
        s.reqs[s.nreq] = Req { write, ok, lba, n, ticks: t };
        s.nreq += 1;
    } else {
        s.more += 1;
    }
}

const GEM_SEL_VDI: u64 = 115;
const GEM_SEL_AES: u64 = 200;

/// The AES opcode, `control[0]` of the caller's parameter block.
fn aes_op(args: &TrapArgs) -> Option<u16> {
    use tos_api::bdos::process::{current_abi, Abi};
    let block = args.regs[1];
    if block == 0 {
        return None;
    }
    let control = match current_abi() {
        Abi::Lp64 => args.space.get_u64(block),
        Abi::Ilp32 => args.space.get_u32(block).map(u64::from),
    }
    .ok()?;
    if control == 0 {
        return None;
    }
    args.space.get_u16(control).ok()
}

/// The argument index of the path of a path-taking GEMDOS call.
fn path_arg(op: u16) -> Option<usize> {
    match op {
        // Dcreate Ddelete Dsetpath Fcreate Fopen Fdelete Fattrib Fsfirst
        0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x41 | 0x43 | 0x4E => Some(0),
        // Frename(0, old, new): the old name
        0x56 => Some(1),
        _ => None,
    }
}

struct LineBuf {
    buf: [u8; 640],
    len: usize,
}

impl Write for LineBuf {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let room = self.buf.len() - self.len;
        let n = s.len().min(room);
        self.buf[self.len..self.len + n].copy_from_slice(&s.as_bytes()[..n]);
        self.len += n;
        if n < s.len() { Err(core::fmt::Error) } else { Ok(()) }
    }
}

fn emit(l: &LineBuf) {
    let mut len = l.len;
    let mut buf = l.buf;
    if len == buf.len() {
        buf[len - 1] = b'\n';
    } else if len == 0 || buf[len - 1] != b'\n' {
        buf[len] = b'\n';
        len += 1;
    }
    // Every byte comes from a `&str` or is replaced by `?` below.
    let text = core::str::from_utf8(&buf[..len]).unwrap_or("[fsio] (unprintable)\n");
    crate::kprint!("{}", text);
}

/// Called by the `svc` handler after the dispatch: `how` is `None` for a
/// completed call, `Some("yield")` / `Some("stall")` for an AES call that gave
/// way and will run again.
pub fn end(class: TrapClass, args: &TrapArgs, result: i64, how: Option<&str>) {
    let t1 = ticks();
    let s = state();
    if s.depth == 0 {
        return;
    }
    s.depth -= 1;
    if s.depth > 0 {
        return;
    }
    let dur = t1.wrapping_sub(s.t0);
    let (kind, op, gi) = match class {
        TrapClass::Gemdos => ("gemdos", args.func as u16, 3),
        TrapClass::Bios => ("bios", args.func as u16, 1),
        TrapClass::Xbios => ("xbios", args.func as u16, 1),
        TrapClass::Gem if args.regs[0] == GEM_SEL_AES => ("aes", aes_op(args).unwrap_or(0xFFFF), 2),
        TrapClass::Gem if args.regs[0] == GEM_SEL_VDI => ("vdi", 0, 0),
        _ => ("gem", args.regs[0] as u16, 0),
    };
    let io = s.nreq > 0 || s.more > 0;
    if !(io || gi >= 2) {
        s.gap_n[gi] += 1;
        s.gap_t[gi] += dur;
        return;
    }
    let p0 = ticks();
    let now = Snap::take();
    let a = s.snap;
    let mut l = LineBuf { buf: [0; 640], len: 0 };
    s.seq += 1;
    let _ = write!(l, "[fsio] {:06} ", s.seq);
    match args.space {
        Space::Kernel => {
            let _ = write!(l, "K  ");
        }
        Space::User => {
            let slot = tos_api::bdos::process::sched::running_slot();
            if slot >= tos_api::bdos::process::MAX_PROCESSES {
                let _ = write!(l, "U? ");
            } else {
                let _ = write!(l, "U{:<2}", slot);
            }
        }
    }
    if kind == "aes" {
        let _ = write!(l, " aes {}", op);
    } else {
        let _ = write!(l, " {} 0x{:02x}", kind, op);
    }
    let _ = write!(l, " t={} dur={}", us(s.t0), us(dur));
    if let Some(h) = how {
        let _ = write!(l, " how={}", h);
    }
    let _ = write!(l, " ret={}", result);
    if kind == "gemdos" {
        if let Some(i) = path_arg(op) {
            let mut p = [0u8; 64];
            if let Ok(n) = args.space.read_cstr(args.regs[i], &mut p) {
                for b in p[..n.min(64)].iter_mut() {
                    if !(0x20..0x7F).contains(b) || *b == b'"' {
                        *b = b'?';
                    }
                }
                let text = core::str::from_utf8(&p[..n.min(64)]).unwrap_or("?");
                let _ = write!(l, " path=\"{}\"", text);
            }
        }
    }
    let _ = write!(
        l,
        " | dev r={}/{}/{} w={}/{}/{}",
        s.rd.reqs,
        s.rd.sectors,
        us(s.rd.ticks),
        s.wr.reqs,
        s.wr.sectors,
        us(s.wr.ticks)
    );
    let _ = write!(
        l,
        " | bc hit={} miss={}",
        now.bc_hit.wrapping_sub(a.bc_hit),
        now.bc_miss.wrapping_sub(a.bc_miss)
    );
    let _ = write!(
        l,
        " | bdos hit={} miss={} fat={} data={}",
        now.bdos_hit.wrapping_sub(a.bdos_hit),
        now.bdos_miss.wrapping_sub(a.bdos_miss),
        now.lm_fat.wrapping_sub(a.lm_fat),
        now.lm_data.wrapping_sub(a.lm_data)
    );
    let _ = write!(l, " | fatlk={}", now.fatlk.wrapping_sub(a.fatlk));
    let _ = write!(
        l,
        " | gap vdi={}/{} bios={}/{} aes={}/{} gemdos={}/{} out={}/{} ovh={}",
        s.gap_n[0],
        us(s.gap_t[0]),
        s.gap_n[1],
        us(s.gap_t[1]),
        s.gap_n[2],
        us(s.gap_t[2]),
        s.gap_n[3],
        us(s.gap_t[3]),
        s.out_n,
        us(s.out_t),
        us(s.ovh)
    );
    emit(&l);
    for r in &s.reqs[..s.nreq] {
        let mut l = LineBuf { buf: [0; 640], len: 0 };
        let _ = write!(
            l,
            "[fsio]   {} lba={} n={} us={}{}",
            if r.write { 'W' } else { 'R' },
            r.lba,
            r.n,
            us(r.ticks),
            if r.ok { "" } else { " FAILED" }
        );
        emit(&l);
    }
    if s.more > 0 {
        let mut l = LineBuf { buf: [0; 640], len: 0 };
        let _ = write!(l, "[fsio]   +{} more", s.more);
        emit(&l);
    }
    s.gap_n = [0; 4];
    s.gap_t = [0; 4];
    s.out_n = 0;
    s.out_t = 0;
    s.ovh = ticks().wrapping_sub(p0);
}
