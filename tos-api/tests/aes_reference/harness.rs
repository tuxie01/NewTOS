//! Harness for the AES reference-conformance tests.
//!
//! These tests replay measured TOS behaviour against `tos_api::aes`. The
//! reference is a dump taken by `tosdump/` on the oracle — **EmuTOS 1a7484ab
//! on Hatari's Atari TT** (`docs/analysis/ORACLE.md`; the same tables on a
//! real TT under EmuTOS 1.4 are `EMUTOS14-*`), produced by `DUMPAES`: round 1,
//! `tosdump/reference/EMUTOS-1a7484ab-AES.BIN` from `tosdump/cases/aes-r1.tsv` (95
//! cases, CRC `33C13C8F`), and round 2, the window manager,
//! `tosdump/reference/EMUTOS-1a7484ab-AES-r2wind.BIN` from
//! `tosdump/cases/aes-r2-wind.tsv` (78 cases, CRC `AFB11C9F`) — and
//! `docs/api/aes/` was extracted from that same EmuTOS source, so a difference
//! here normally has exactly one reading: NewTOS is wrong. The three exceptions
//! (EmuTOS *version* deltas, [`DELTAS`]; the fields that are a property of the
//! machine rather than of the AES, [`GLOBAL_EXCLUDED`]; and the deviations a
//! human decided and registered in `docs/COMPATIBILITY.md`, [`SANCTIONED`])
//! are named there with a reason and a `file:line`; nothing is skipped
//! silently and no case is `#[ignore]`d.
//!
//! **What these tests cover.** The record holds the six AES parameter-block
//! arrays as they stood *after* the call — `control`, `global`, `int_in`,
//! `int_out`, `addr_in`, `addr_out` — so they exercise the dispatcher's
//! marshalling (`min(IN_LEN, I_SIZE)` words in, exactly `OUT_LEN` words back
//! out, `aes/gemsuper.c:428-448`), every opcode's computed outputs, and the
//! rule that a manager may not write the caller's input arrays. They say
//! nothing about pixels: round 1 is pointer-free and draws nothing.
//!
//! **The trap's `d0` is out of reach.** The record carries it (offset 106);
//! both machines wrote 0. `tos_api::aes::dispatch` returns
//! `Option<sched::Switch>`, not the WORD the trap stub puts in `d0`, so the
//! field cannot be compared from here — the same limitation the VDI harness
//! has, and for the same reason. `tosdump/host/build/tosdiff` compares it.
//!
//! **The reset, and why the order does not matter.** Which reset runs before
//! a case is the case *table's* to declare, on its `!reset` directive line,
//! and the runner issues it before every case out of `ct->reset`
//! (`tosdump/core/cases.h`, `tosdump/core/runner_aes.c:150-154`). Round 1
//! declares none — the AES has no documented default state to restore
//! (`tosdump/core/runner_aes.h`) — and was therefore built out of opcodes
//! that do not mutate state another case reads (`tosdump/cases/aes-r1.tsv`
//! header). Round 2 declares `wind_new`, the AES's own reset
//! (`aes/gemwmlib.c:1875-1893`), which is what makes handle allocation and
//! window geometry reproducible from case to case. [`Reset`] runs it, and
//! [`Table`] is what carries the declaration from the generated module to
//! here, so the replay cannot measure something the dump did not.
//!
//! A declared reset is issued **after** the case as well as before it. The
//! dumper needs no such thing — the next case's leading reset sweeps whatever
//! the previous one left, and it runs one last one after the last case
//! (`runner_aes.c:218-227`) — but here round 1 and round 2 share one test
//! binary in an arbitrary order, and round 1 has cases that read exactly what
//! round 2 leaves behind: `wind_get(1, WF_WXYWH)` on "a window that was never
//! created", `wind_get(0, WF_TOP)`, `wind_find`. So the trailing reset is what
//! keeps cargo's ordering irrelevant — measured, not assumed: with it removed,
//! `cargo test -- -Zunstable-options --shuffle --test-threads=1` turns round 1's
//! case 163 (`wind_get(0, WF_TOP)`) red, reading the handle 2 a round-2 case
//! left open. It runs even when the case is about to fail, or one red case
//! would turn every round-1 case that follows it red too.
//!
//! Cases run one at a time whatever the order: the AES window table, the
//! process descriptors and the installed host are process-wide statics; hence
//! [`AES_LOCK`].
//!
//! **Why the screen is 640×480 at four planes.** The reference was measured
//! in TT-medium. Several outputs are a function of the screen — `wind_get`'s
//! desktop rectangles, `appl_init`'s `ap_planes` — so replaying at the
//! machine's geometry is what turns them into a real comparison instead of an
//! excused difference. On the Pi 4 the same code answers 1920×1080 at 16
//! planes (Falcon truecolor), and that is correct there: the values are the
//! workstation's, and this harness supplies a workstation shaped like the TT's.

#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

/// The geometry the reference dump was measured at: the TT in TT-medium,
/// 640×480 at four planes / sixteen pens.
pub const SCREEN_W: u16 = 640;
pub const SCREEN_H: u16 = 480;
pub const SCREEN_PLANES: u16 = 4;
pub const SCREEN_COLORS: u16 = 16;

// ── the memory raster backend ────────────────────────────────────────────

/// A plain linear pixel buffer at the reference geometry. The kernel's
/// rasterizer is pinned to AArch64 and reaches the board framebuffer through
/// the HAL, so it cannot run in a host test; nothing in round 1 draws, and
/// this exists only so that the AES sees a screen of the right size and depth.
pub struct MemBackend {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl MemBackend {
    fn new() -> Self {
        MemBackend {
            px: Mutex::new(vec![0; SCREEN_W as usize * SCREEN_H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new([(0, 0, 0); 256]),
        }
    }

    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < SCREEN_W as i32 && (y as i32) < SCREEN_H as i32
    }

    fn idx(x: i16, y: i16) -> usize {
        y as usize * SCREEN_W as usize + x as usize
    }

    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(SCREEN_W, SCREEN_H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return;
            }
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }
}

impl RasterBackend for MemBackend {
    fn width(&self) -> u16 {
        SCREEN_W
    }
    fn height(&self) -> u16 {
        SCREEN_H
    }
    fn num_colors(&self) -> u16 {
        SCREEN_COLORS
    }
    fn planes(&self) -> u16 {
        SCREEN_PLANES
    }

    fn set_color(&self, index: u16, r: u16, g: u16, b: u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize] = (r, g, b);
        }
    }
    fn query_color(&self, index: u16) -> (u16, u16, u16) {
        if (index as usize) < 256 {
            self.req.lock().unwrap()[index as usize]
        } else {
            (0, 0, 0)
        }
    }
    fn set_active_palette(&self, palette: &[u16; 256]) {
        *self.palette.lock().unwrap() = *palette;
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        let (r, g, b) = ((argb >> 16) & 0xFF, (argb >> 8) & 0xFF, argb & 0xFF);
        (((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)) as u16
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        let (r5, g6, b5) = ((px >> 11) & 0x1F, (px >> 5) & 0x3F, px & 0x1F);
        let r = ((r5 << 3) | (r5 >> 2)) as u32;
        let g = ((g6 << 2) | (g6 >> 4)) as u32;
        let b = ((b5 << 3) | (b5 >> 2)) as u32;
        (r << 16) | (g << 8) | b
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        if !Self::inside(x, y) {
            return 0;
        }
        let v = self.px.lock().unwrap()[Self::idx(x, y)];
        let want = self.device_to_rgb565(v);
        let pal = *self.palette.lock().unwrap();
        pal.iter().position(|&p| p == want).unwrap_or(0) as Pel
    }

    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, pel: Pel, clip: ClipRect) {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        for yy in y..y.saturating_add(h) {
            for xx in x..x.saturating_add(w) {
                self.put(xx, yy, v, clip);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn abline(
        &self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        pel: Pel,
        _mode: WriteMode,
        clip: ClipRect,
        mask: u16,
        _last: bool,
    ) -> u16 {
        let v = self.rgb565_to_device(self.pel_to_device16(pel));
        let (mut x, mut y) = (x0 as i32, y0 as i32);
        let (dx, dy) = ((x1 as i32 - x).abs(), -(y1 as i32 - y).abs());
        let (sx, sy) = (if x < x1 as i32 { 1 } else { -1 }, if y < y1 as i32 { 1 } else { -1 });
        let mut err = dx + dy;
        let mut m = mask;
        loop {
            if m & 0x8000 != 0 {
                self.put(x as i16, y as i16, v, clip);
            }
            m = m.rotate_left(1);
            if x == x1 as i32 && y == y1 as i32 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        m
    }

    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, fill: PatternFill, clip: ClipRect) {
        let fg = self.rgb565_to_device(self.pel_to_device16(fill.fg));
        let bg = self.rgb565_to_device(self.pel_to_device16(fill.bg));
        for yy in y..y.saturating_add(h) {
            let row = fill.pattern[(yy.rem_euclid(16)) as usize % fill.pattern.len()];
            for xx in x..x.saturating_add(w) {
                let bit = row & (0x8000u16 >> (xx.rem_euclid(16) as u16));
                self.put(xx, yy, if bit != 0 { fg } else { bg }, clip);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        glyph: &[u8],
        cell_w: i16,
        cell_h: i16,
        fg: Pel,
        _mode: WriteMode,
        clip: ClipRect,
    ) {
        let v = self.rgb565_to_device(self.pel_to_device16(fg));
        for row in 0..cell_h {
            let byte = glyph.get(row as usize).copied().unwrap_or(0);
            for col in 0..cell_w.min(8) {
                if byte & (0x80 >> col) != 0 {
                    self.put(x + col, y + row, v, clip);
                }
            }
        }
    }

    fn blit_mono(&self, dst: Rect, src: MonoSrc, fg: Pel, bg: Pel, _m: WriteMode, clip: ClipRect) {
        let f = self.rgb565_to_device(self.pel_to_device16(fg));
        let b = self.rgb565_to_device(self.pel_to_device16(bg));
        for row in 0..dst.h {
            for col in 0..dst.w {
                let sx = src.x + col;
                let sy = src.y + row;
                let wi = (sy as usize) * src.stride_words as usize + (sx as usize) / 16;
                let w = src.bits.word(wi).unwrap_or(0);
                let bit = w & (0x8000u16 >> (sx.rem_euclid(16)) as u16);
                self.put(dst.x + col, dst.y + row, if bit != 0 { f } else { b }, clip);
            }
        }
    }

    fn copy_rect(&self, src: Rect, dx: i16, dy: i16, _op: u8, clip: ClipRect) {
        let mut tmp = Vec::with_capacity((src.w as usize) * (src.h as usize));
        {
            let px = self.px.lock().unwrap();
            for row in 0..src.h {
                for col in 0..src.w {
                    let (sx, sy) = (src.x + col, src.y + row);
                    tmp.push(if Self::inside(sx, sy) { px[Self::idx(sx, sy)] } else { 0 });
                }
            }
        }
        for row in 0..src.h {
            for col in 0..src.w {
                let v = tmp[(row as usize) * (src.w as usize) + col as usize];
                self.put(dx + col, dy + row, v, clip);
            }
        }
    }

    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)]
        } else {
            0
        }
    }
    fn put_device_pixel(&self, x: i16, y: i16, argb: u32) {
        if Self::inside(x, y) {
            self.px.lock().unwrap()[Self::idx(x, y)] = argb;
        }
    }
    fn flush_rect(&self, _x0: i16, _y0: i16, _x1: i16, _y1: i16) {}
}

// ── the host ─────────────────────────────────────────────────────────────

/// The host the AES runs against. `pump_input` is what EmuTOS's `dsptch()`
/// fall-through reaches (`aes/gemsuper.c:452-464`); there is no input script
/// here, so it only advances the clock. The clock also advances on every
/// read, which is what lets the two `evnt_timer` cases terminate: a wait of
/// one millisecond is over after the harness has looked at the clock twice.
pub struct TestHost {
    raster: MemBackend,
    clock: AtomicU64,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
    fn monotonic_millis(&self) -> u64 {
        self.clock.fetch_add(1, Ordering::Relaxed)
    }
    fn pump_input(&self) {
        self.clock.fetch_add(1, Ordering::Relaxed);
    }
}

// SAFETY: every field is behind a Mutex or an atomic.
unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

/// The AES keeps process-global state (window table, AES process descriptors,
/// the built-in DESKTOP tree). Cargo runs tests on parallel threads, so every
/// case takes this lock.
pub static AES_LOCK: Mutex<()> = Mutex::new(());
static HOST_CELL: OnceLock<&'static TestHost> = OnceLock::new();
static HOST_REF: OnceLock<&'static dyn Host> = OnceLock::new();
static CTX: OnceLock<Context> = OnceLock::new();

/// The ABI the parameter block is laid at. The measured NewTOS run was an
/// LP64 program (`tosdump/build-dumpaes-newtos.sh` compiles with
/// `-target aarch64-unknown-none-elf`), and LP64 is NewTOS's native
/// application ABI, so the replay uses it too. It is the one place the two
/// machines genuinely cannot agree byte for byte — see [`ap_private`] in
/// [`cmp_global`].
pub const ABI: Abi = Abi::Lp64;

// ── the AES parameter block ──────────────────────────────────────────────

/// `control[5]` — in the AES **all five** slots are inputs: the dispatcher
/// copies `control[]` onto a local buffer and never writes any of it back
/// (`aes/gemsuper.c:435`).
pub const CONTRL_N: usize = 5;
/// `global[15]` — EmuTOS `struct aesglobal`, 30 bytes (`aes/gemrslib.h:15-27`,
/// `desk/gembind.h:27` `G_SIZE 15`). There is no index 15.
pub const GLOBAL_N: usize = 15;
/// `I_SIZE` (`include/aesdefs.h`), the dispatcher's input clamp.
pub const INTIN_N: usize = 16;
/// `O_SIZE`, the largest `OUT_LEN` any opcode produces (`evnt_multi`'s seven).
pub const INTOUT_N: usize = 7;
/// `AI_SIZE` / the single `addr_out` slot.
pub const ADDRIN_N: usize = 3;
pub const ADDROUT_N: usize = 1;

/// The 0xDEAD poison (`docs/TOS-REFERENCE-DUMPER.md` section 4). Everything
/// the call is not supposed to write stays at this value, which is what makes
/// "wrote a slot it must not touch" visible at all.
pub const POISON: i16 = 0xDEADu16 as i16;
/// The pointer-array poison. `tosdump/core/runner_aes.c` builds it as a
/// *value* repeated to the target's pointer width, so the low 32 bits — the
/// only part the record carries — read `0xDEADDEAD` on the 32-bit m68k and on
/// 64-bit AArch64 alike.
pub const POISON_PTR: u64 = 0xDEAD_DEAD_DEAD_DEAD;

// ── the AES pointer buffers (the `addr_in` column) ───────────────────────

/// Buffers a case table may name by number, `tosdump/core/tosdump.h`
/// (`TD_AES_BUF_N` / `TD_AES_BUF_BYTES` / `TD_AES_BUF_TEXT_LEN`).
///
/// A dozen AES opcodes do all of their work behind a pointer, and a case table
/// cannot state an address: it is one file carried to two machines. So the
/// table names a **buffer by number** and each runner supplies its own
/// address. The dumper's are file-scope statics laid fresh before every call
/// it issues (`tosdump/core/runner_aes.c`, `lay_buffers`); these are the
/// harness's counterpart, and they have to be the same size and the same bytes
/// or the case would measure the buffer instead of the function.
pub const BUF_N: usize = 4;
pub const BUF_BYTES: usize = 256;
/// Printable characters before the NUL. Eight, because that is the AES' own
/// name length (`aes/struct.h`, `AP_NAMELEN`), which `fpdnm`'s `strncmp`
/// compares (`aes/gempd.c:48`).
pub const BUF_TEXT_LEN: usize = 8;

/// What the **record** carries for an `addr_in` slot a case pointed at a
/// buffer: `TD_AES_ADDR_TOKEN(n)` (`tosdump/core/tosdump.h`).
///
/// A buffer's address is different on a TT and on a Pi by construction, so
/// recording it would fabricate a difference in every case that uses one —
/// the trap the poison pattern exists to avoid. A slot that still holds what
/// was planted therefore records this token, which says what the *table* said;
/// a slot the call modified records the modified value, which is exactly the
/// difference it is.
pub const ADDR_TOKEN_BASE: u32 = 0xB0FF_0000;

/// One buffer, aligned like the dumper's (a union with the machine's widest
/// scalar) so a call that reads WORDs or LONGs out of one is aligned on both.
#[repr(C, align(8))]
pub struct Buf(pub [u8; BUF_BYTES]);

/// Lay one buffer exactly as `td_aes_buf_fill` does
/// (`tosdump/core/record.c`): every byte `'A' + (index * 7 + i) % 26`, with a
/// NUL at [`BUF_TEXT_LEN`] and the same printable pattern behind it as a
/// canary.
///
/// **A string and not the raster's word pattern**, and that is not a taste:
/// half of the calls this unlocks read a C string through the pointer
/// (`appl_find`'s `AP_PNAME`, `rsrc_load`'s filename, `menu_text`,
/// `shel_find`, the scrap manager's `strcpy`) and neither AES bounds-checks
/// one, so a pseudo-random fill would send `strlen` off the end.
///
/// The seven-letter stride makes the four start with four visibly different
/// strings — `ABCDEFGH`, `HIJKLMNO`, `OPQRSTUV`, `VWXYZABC` — none of which is
/// the name of any AES process, so `appl_find` against one answers -1 on both
/// machines rather than a machine-dependent `ap_id`.
fn buf_fill(buf: &mut Buf, index: usize) {
    for (i, b) in buf.0.iter_mut().enumerate() {
        let n = index as u32 * 7 + i as u32;
        *b = b'A' + (n % 26) as u8;
    }
    buf.0[BUF_TEXT_LEN] = 0;
}

/// The four buffers, leaked once and shared by every parameter block —
/// statics, like the dumper's, and for a reason beyond symmetry: an AES call
/// may **keep** the pointer (`wind_set(WF_NAME)` stores it in the window table
/// and a later `wind_get(WF_NAME)` reads it back), so a buffer owned by one
/// [`Pb`] would dangle the moment that `Pb` was dropped.
///
/// Handing out `&'static mut` is sound only because [`AES_LOCK`] serialises
/// the cases — the same discipline [`global_array`] runs under.
fn buffers() -> &'static mut [Buf; BUF_N] {
    static B: OnceLock<usize> = OnceLock::new();
    let p = *B.get_or_init(|| {
        let b: Box<[Buf; BUF_N]> =
            Box::new([Buf([0; BUF_BYTES]), Buf([0; BUF_BYTES]), Buf([0; BUF_BYTES]), Buf([0; BUF_BYTES])]);
        Box::leak(b) as *mut _ as usize
    });
    // SAFETY: leaked above, so it lives for the whole program; access is
    // serialised by `AES_LOCK`.
    unsafe { &mut *(p as *mut [Buf; BUF_N]) }
}

/// The bytes of one buffer as they stand — what a call reads through a planted
/// pointer. The caller must hold [`AES_LOCK`]; a copy, so the snapshot cannot
/// alias the live array.
pub fn buffer_snapshot(n: usize) -> [u8; BUF_BYTES] {
    buffers()[n].0
}

/// Where one plant puts a buffer's address (`tosdump/core/cases.h`,
/// `TD_AES_PLANT_*`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    /// `aN=B` — `addr_in[N]`, the address array proper. Only reachable when
    /// the case declares `AIN_LEN > N`: the dispatcher copies
    /// `min(AIN_LEN, AI_SIZE)` slots onto its local buffer
    /// (`aes/gemsuper.c:439`).
    AddrIn,
    /// `iN=B` — a pointer packed into the `int_in` WORDs at word N, at the
    /// **target's own pointer width**, which is the classic GEM binding
    /// (gemlib `a_wind_set_str.c:25`) and how EmuTOS' `wm_set` reads
    /// `WF_NAME`: `*(char **)pinwds` with `pinwds = &int_in[2]`
    /// (`aes/gemwmlib.c:1661`). Prelude-only — `int_in` is recorded, and a
    /// packed pointer occupies two WORDs on the m68k and four here.
    IntIn,
}

/// One entry of a case table's `addr_in` column, as `mkrusttests` emits it.
pub struct Plant {
    pub kind: Kind,
    /// `addr_in` slot index, or `int_in` word index.
    pub slot: usize,
    /// Which of the [`BUF_N`] buffers, `0..BUF_N`.
    pub buffer: usize,
}

/// The WORDs a planted `int_in` pointer occupies at this harness's [`ABI`]
/// (LP64: eight bytes / two bytes per WORD). `TD_AES_PLANT_PTR_WORDS`.
const PLANT_PTR_WORDS: usize = 4;

pub struct Pb {
    pub contrl: Box<[i16; CONTRL_N]>,
    pub intin: Box<[i16; INTIN_N]>,
    pub intout: Box<[i16; INTOUT_N]>,
    pub addrin: Box<[u64; ADDRIN_N]>,
    pub addrout: Box<[u64; ADDROUT_N]>,
    /// The application's live `AESGLOBAL` — **not** owned by the block. It is
    /// the process-wide array [`global_array`] leaked once, exactly as
    /// `tosdump/core/gem.c` keeps one file-scope `aes_global[15]` for the
    /// whole run: `appl_init` fills it and every case afterwards is measured
    /// against what it left there.
    global: &'static mut [i16; GLOBAL_N],
    block: Box<[u64; 6]>,
    /// Which buffer, if any, each `addr_in` slot was pointed at for the call
    /// that just ran, and what was planted there — so the comparison can tell
    /// "the slot still holds the buffer" from "the call wrote something else
    /// into it" (`tosdump/core/runner_aes.c`, `run_planted_buf` /
    /// `run_planted_ptr`). `None` is "not planted", which is every slot of
    /// every seven-column table.
    planted: [Option<(usize, u64)>; ADDRIN_N],
}

impl Pb {
    pub fn new() -> Pb {
        let mut pb = Pb {
            contrl: Box::new([0; CONTRL_N]),
            intin: Box::new([0; INTIN_N]),
            intout: Box::new([0; INTOUT_N]),
            addrin: Box::new([0; ADDRIN_N]),
            addrout: Box::new([0; ADDROUT_N]),
            global: global_array(),
            block: Box::new([0; 6]),
            planted: [None; ADDRIN_N],
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.global.as_ptr() as u64;
        pb.block[2] = pb.intin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.addrin.as_ptr() as u64;
        pb.block[5] = pb.addrout.as_ptr() as u64;
        pb
    }

    /// The live `AESGLOBAL` as it stands.
    pub fn global(&self) -> &[i16; GLOBAL_N] {
        self.global
    }

    /// Poison everything **except** `global`, which is live AES state and
    /// belongs to the application rather than to the case
    /// (`tosdump/core/runner_aes.c` step 2), and lay the four pointer buffers
    /// over again.
    ///
    /// The buffers are laid fresh before **every** call the harness issues,
    /// not once per case — exactly where `runner_aes.c` calls `lay_buffers`,
    /// which is next to every one of its three poison sites. A buffer the
    /// previous call wrote into would carry that call's result into the next
    /// one, and the whole point of the poison rule is that nothing does. The
    /// plant record is cleared with it, as `plant_buffers` clears it.
    pub fn poison(&mut self) {
        self.contrl.fill(POISON);
        self.intin.fill(POISON);
        self.intout.fill(POISON);
        self.addrin.fill(POISON_PTR);
        self.addrout.fill(POISON_PTR);
        for (i, b) in buffers().iter_mut().enumerate() {
            buf_fill(b, i);
        }
        self.planted = [None; ADDRIN_N];
    }

    /// Put the buffers a case table line named where the line said
    /// (`tosdump/core/runner_aes.c`, `plant_buffers`). Issued after
    /// [`Pb::poison`] and after the line's `int_in` values, so what a slot
    /// holds going in is either a buffer this line asked for or the poison.
    pub fn plant(&mut self, plants: &[Plant]) {
        for p in plants {
            assert!(p.buffer < BUF_N, "plant names buffer {} of {BUF_N}", p.buffer);
            let addr = buffers()[p.buffer].0.as_ptr() as u64;
            match p.kind {
                Kind::AddrIn => {
                    assert!(p.slot < ADDRIN_N, "plant names addr_in[{}]", p.slot);
                    self.addrin[p.slot] = addr;
                    self.planted[p.slot] = Some((p.buffer, addr));
                }
                Kind::IntIn => {
                    // A target-width pointer written over the WORDs, as the
                    // dumper's `memcpy(&int_in[w], &p, sizeof p)` does — and
                    // not a cast, because `&int_in[w]` is only WORD aligned.
                    assert!(
                        p.slot + PLANT_PTR_WORDS <= INTIN_N,
                        "a packed pointer at int_in[{}] runs past I_SIZE",
                        p.slot
                    );
                    let bytes = addr.to_ne_bytes();
                    // SAFETY: `intin` is `[i16; INTIN_N]`; the assert above
                    // keeps the eight bytes inside it.
                    unsafe {
                        let dst = (self.intin.as_mut_ptr() as *mut u8).add(p.slot * 2);
                        dst.copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
                    }
                }
            }
        }
    }

    /// What `tosdump/core/runner_aes.c` step 6 records for one `addr_in` slot:
    /// the buffer **token** while the slot still holds what was planted, the
    /// low 32 bits of whatever is there otherwise.
    pub fn recorded_addrin(&self, i: usize) -> u32 {
        match self.planted[i] {
            Some((buf, ptr)) if self.addrin[i] == ptr => ADDR_TOKEN_BASE + buf as u32,
            _ => self.addrin[i] as u32,
        }
    }

    /// Lay `control[0..4]` and issue the call — `tosdump/core/runner_aes.c`'s
    /// `aes_call`, which the measured call and every prelude both go through
    /// so the two cannot lay the block differently.
    pub fn call(&mut self, opcode: i16, in_len: i16, out_len: i16, ain_len: i16, aout_len: i16) {
        self.contrl[0] = opcode;
        self.contrl[1] = in_len;
        self.contrl[2] = out_len;
        self.contrl[3] = ain_len;
        self.contrl[4] = aout_len;
        // SAFETY: `block` holds six live pointers to arrays owned by `self`,
        // each at least as long as the AES parameter block requires.
        unsafe { tos_api::aes::dispatch(self.block.as_ptr() as u64, ABI) };
    }
}

// ── the application context ──────────────────────────────────────────────

/// What `tosdump/core/gem.c`'s `td_gem_open` established before the first
/// case ran: `appl_init`, `graf_handle`, and `v_opnvwk` on the handle
/// `graf_handle` returned. The AES cases need none of the VDI workstation,
/// but the reference was measured with it open and the context has to be the
/// same for the comparison to mean anything.
#[derive(Copy, Clone)]
pub struct Context {
    pub ap_id: i16,
    pub phys_handle: i16,
    pub handle: i16,
}

/// `work_in` for `v_opnvwk`: every attribute index 1, raster coordinates —
/// the classic GEM application open, stated once in `tosdump/core/gem.c` and
/// repeated here call for call.
fn open_workstation(phys: i16) -> i16 {
    let contrl = Box::new([0i16; 12]);
    let intin = Box::new([0i16; 128]);
    let ptsin = Box::new([0i16; 256]);
    let intout = Box::new([0i16; 128]);
    let ptsout = Box::new([0i16; 256]);
    let mut block = [0u64; 5];
    block[0] = contrl.as_ptr() as u64;
    block[1] = intin.as_ptr() as u64;
    block[2] = ptsin.as_ptr() as u64;
    block[3] = intout.as_ptr() as u64;
    block[4] = ptsout.as_ptr() as u64;

    // SAFETY: the five arrays outlive the call and are large enough for
    // `v_opnvwk`'s `work_out` (45 words) and `SIZ_TAB` (12 words).
    unsafe {
        let c = contrl.as_ptr() as *mut i16;
        let i = intin.as_ptr() as *mut i16;
        for k in 0..10 {
            i.add(k).write(1);
        }
        i.add(10).write(2); // 2 = raster coordinates
        for k in 0..12 {
            c.add(k).write(0);
        }
        c.write(100); // v_opnvwk
        c.add(3).write(11); // 11 intin words
        c.add(6).write(phys);
        let _flip_y = tos_api::vdi::dispatch(block.as_ptr() as u64, ABI);
        c.add(6).read()
    }
}

/// The one application context of this test binary, opened once. `global` is
/// leaked deliberately: the AES filled it at `appl_init` and every case is
/// measured against it afterwards, exactly as the dumper's file-scope
/// `aes_global[15]` lives for the whole run.
pub fn context() -> &'static Context {
    CTX.get_or_init(|| {
        let host = host_only();
        install_screen_buffer(&host.raster);

        // The AES session start the dumper ran under: `gsx_init`
        // (`geminit.c:675`) opens the physical workstation whose handle
        // `graf_handle` reports, before any application exists.
        tos_api::aes::gsxif::gsx_init();

        let mut pb = Pb::new();
        pb.poison();
        pb.call(10, 0, 1, 0, 0); // appl_init
        let ap_id = pb.intout[0];
        assert!(ap_id >= 0, "appl_init failed ({ap_id})");

        pb.poison();
        pb.call(77, 0, 5, 0, 0); // graf_handle
        let phys_handle = pb.intout[0];
        assert!(phys_handle > 0, "graf_handle returned no handle ({phys_handle})");

        let handle = open_workstation(phys_handle);
        assert!(handle > 0, "v_opnvwk returned no handle ({handle})");

        Context { ap_id, phys_handle, handle }
    })
}

/// The host alone, installed once — no application opened, **no screen
/// buffer**. [`context`] builds on it; a binary that wants the AES exactly as
/// it stands before `gsx_malloc` (the buffer-less `wind_get(WF_SCREEN)`
/// fallback, `tests/aes_window_screen_buffer_absent.rs`) uses it directly.
pub fn host_only() -> &'static TestHost {
    HOST_CELL.get_or_init(|| {
        let host: &'static TestHost = Box::leak(Box::new(TestHost {
            raster: MemBackend::new(),
            clock: AtomicU64::new(0),
        }));
        let href: &'static &'static dyn Host =
            Box::leak(Box::new(*HOST_REF.get_or_init(|| host as &'static dyn Host)));
        tos_api::host::install_host(href);
        host
    })
}

/// The AES menu/alert screen buffer this binary runs with — `gsx_malloc`'s
/// block (`aes/gemgsxif.c:108-114`), which on the target comes from the BDOS
/// pool when the AES goes graphic and which a host process cannot take (no
/// pool, and a host `Vec` does not lie under 2 GiB). Installed as the fixture
/// `tos_api::aes::gsxif::install_screen_buffer` provides, sized as
/// `gsx_mcalc` sizes it for this backend (one 640×480 screen in device
/// pixels, 614 400 bytes), and reported to an EL1 caller of
/// `wind_get(WF_SCREEN)` at [`SCREEN_BUFFER_PA`]. Leaked deliberately: the
/// AES keeps the address for the rest of the run, as the TT kept
/// `0x01000218` for the whole dump.
fn install_screen_buffer(backend: &MemBackend) {
    let len = tos_api::aes::gsxif::gsx_mcalc(backend);
    let buf: &'static mut [u8] = Box::leak(vec![0u8; len].into_boxed_slice());
    // SAFETY: the leaked slice lives for the rest of the process and holds
    // exactly `gsx_mcalc` bytes.
    unsafe { tos_api::aes::gsxif::install_screen_buffer(buf.as_mut_ptr(), SCREEN_BUFFER_PA, len) };
}

/// The "physical address" the fixture reports for the screen buffer — a
/// page-aligned TT-RAM-looking value under 2 GiB, the kind the pool hands
/// out on the target. Machine-dependent on every machine (the TT recorded
/// `0x01000218`, `measurements/measurement-2026-09-12-aes-r4/`), so no reference dump
/// decides it; `tests/aes_window_screen_buffer.rs` checks it by formula.
pub const SCREEN_BUFFER_PA: u64 = 0x0100_0000;

/// The live `AESGLOBAL`, allocated once and shared by every parameter block.
/// Zero-initialised, which is what the dumper's `.bss` array was. Handing out
/// several `&'static mut` to it is sound only because [`AES_LOCK`] serialises
/// the cases and no two `Pb`s are alive across the lock — the same discipline
/// the AES itself runs under.
fn global_array() -> &'static mut [i16; GLOBAL_N] {
    static G: OnceLock<usize> = OnceLock::new();
    let p = *G.get_or_init(|| Box::leak(Box::new([0i16; GLOBAL_N])) as *mut _ as usize);
    // SAFETY: the array was leaked above, so it lives for the whole program;
    // access is serialised by `AES_LOCK`.
    unsafe { &mut *(p as *mut [i16; GLOBAL_N]) }
}

// ── the reference record ─────────────────────────────────────────────────

pub const HDR_SIZE: usize = 32;
pub const REC_SIZE: usize = 120;
const OFF_CASEID: usize = 0;
const OFF_OPCODE: usize = 2;
const OFF_CONTRL: usize = 4;
const OFF_GLOBAL: usize = 14;
const OFF_INTIN: usize = 44;
const OFF_INTOUT: usize = 76;
const OFF_ADDRIN: usize = 90;
const OFF_ADDROUT: usize = 102;
const OFF_RET: usize = 106;

fn be16(b: &[u8], off: usize) -> i16 {
    (((b[off] as u16) << 8) | b[off + 1] as u16) as i16
}

fn be32(b: &[u8], off: usize) -> u32 {
    ((b[off] as u32) << 24) | ((b[off + 1] as u32) << 16) | ((b[off + 2] as u32) << 8)
        | b[off + 3] as u32
}

// ── the case table ───────────────────────────────────────────────────────

/// The state reset a case table declares on its `!reset` directive line.
///
/// Which one runs is the **table's** to say and not the program's: the dumper
/// is double-clicked on the TT with no arguments and always reads `AES.TSV`,
/// so the file is the only thing that can tell two rounds apart, and the
/// file's CRC is what the dump header carries (`tosdump/core/cases.h`). The
/// sequence itself lives in one place so both machines run the identical one
/// (`tosdump/core/runner_aes.c`, `td_reset_aes`).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Reset {
    /// `!reset none`, or no directive at all. Round 1's table is frozen and
    /// its dumps are taken; any other default would silently change what
    /// re-running it measures.
    None,
    /// `!reset wind_new`. The AES's own reset: `wm_new` closes and deletes
    /// every window with a handle ≥ 1 and drains the `wind_update` mouse
    /// control and screen sync locks (`aes/gemwmlib.c:1875-1893`). EmuTOS
    /// issues it itself after every graphics-mode program the shell runs
    /// (`aes/gemshlib.c:676-677`).
    WindNew,
}

impl Reset {
    /// Issue it exactly as `td_reset_aes` does: poisoned like any other call
    /// the runner issues, and laid as gemlib lays it — `AES_PARAMS(109,0,0,0,0)`
    /// (`a_wind_new.c:25`). `OUT_LEN` 0 is not cosmetic: `xif` copies
    /// `OUT_LEN` words back out of an **uninitialised** stack buffer
    /// (`aes/gemsuper.c:430-432,444-445`), so a reset that declared output
    /// words would write stack garbage over the caller's array before the
    /// case it precedes even starts.
    fn run(self, pb: &mut Pb) {
        match self {
            Reset::None => {}
            Reset::WindNew => {
                pb.poison();
                pb.call(109, 0, 0, 0, 0);
            }
        }
    }
}

// ── the 3-D look variables: state no declared reset sweeps ───────────────

/// `objc_sysvar`'s `which` selectors (`aes/obdefs.h`), and EmuTOS' `LWHITE`.
const LK3DIND: i16 = 1;
const LK3DACT: i16 = 2;
const INDBUTCOL: i16 = 3;
const ACTBUTCOL: i16 = 4;
const BACKGRCOL: i16 = 5;
const LWHITE: i16 = 8;

/// The button colour `init_3d` picks for **this harness's** workstation.
///
/// EmuTOS chooses by the palette it has: `actbutcol = indbutcol = backgrcol =`
/// `WHITE` when `gl_ws.ws_ncolors <= LWHITE`, `LWHITE` otherwise
/// (`aes/gemoblib.c`, `init_3d`). This harness opens a TT-shaped workstation
/// with [`SCREEN_COLORS`] pens, so the branch is decided here rather than
/// written down as a number that happens to be right today.
const BUTCOL_DEFAULT: i16 = if SCREEN_COLORS as i16 <= LWHITE { 0 } else { LWHITE };

/// Put the seven 3-D look variables back where `init_3d` left them.
///
/// **Why this exists at all, and why it is not a [`Reset`].** A reset is the
/// *table's* to declare and the dumper runs the identical one
/// (`tosdump/core/runner_aes.c`, `td_reset_aes`). This is not that: it is a
/// harness-only device of exactly the same nature as the *trailing* reset
/// (module header) — the dumper needs neither, because it runs a table **once,
/// in order, on a freshly started AES**, while cargo runs one `#[test]` per
/// case in an arbitrary order inside one process.
///
/// `tosdump/cases/aes-m06-sysvar.tsv` says so itself: "the 3D variables are
/// file-local statics set once by `init_3d` … NOTHING resets them inside a
/// run: `wm_new` sweeps windows and locks, not these. So section A — which
/// measures the DEFAULTS — must come before any set". Section B's setters are
/// cases too, and with them free to run first, section A measured whatever
/// they left: without this sweep,
/// `cargo test -- -Zunstable-options --shuffle` failed on one to three of the
/// nineteen m06 cases on **every** one of eight consecutive runs — case 90
/// reading `int_out[1..2] = 1, 0` (what case 107 set) instead of the recorded
/// defaults `0, 1`.
///
/// **What it restores to is the dumper's own starting point.** Every read-back
/// case in that table carries its own prelude, so with the variables at the
/// `init_3d` defaults before each case, each case sees exactly the state the
/// dumper's run had reached at that line — including case 116, whose point is
/// that the refused `BACKGRCOL 32767` changed nothing.
///
/// **It is done through the AES' own set arm**, not by reaching into
/// `tos_api::aes::object`'s statics: `ob_sysvar(mode=1, …)` is the only writer
/// EmuTOS has (`aes/gemoblib.c`) and the only one NewTOS has, so the sweep
/// cannot restore something the API could not.
///
/// **The price, stated.** A case table can no longer measure a 3-D look value
/// that *persists from one case to the next*; it has to establish what it
/// measures in its own preludes, as m06 does. Nothing else is given up — the
/// cases still run in parallel, serialised by [`AES_LOCK`] as before.
fn sweep_3d_look(pb: &mut Pb) {
    // `in1`/`in2` per selector; -1 is ob_sysvar's "leave this one alone", so
    // the three colour selectors state their one value and nothing else.
    const RESTORE: [[i16; 4]; 5] = [
        [1, LK3DIND, 0, 1],                  // indtxtmove 0, indcolchange 1
        [1, LK3DACT, 1, 0],                  // acttxtmove 1, actcolchange 0
        [1, INDBUTCOL, BUTCOL_DEFAULT, -1],
        [1, ACTBUTCOL, BUTCOL_DEFAULT, -1],
        [1, BACKGRCOL, BUTCOL_DEFAULT, -1],
    ];
    for v in RESTORE {
        pb.poison();
        pb.intin[..v.len()].copy_from_slice(&v);
        // OUT_LEN 1: a set writes RET_CODE and nothing else, and a larger
        // OUT_LEN would copy back words the handler never wrote.
        pb.call(48, 4, 1, 0, 0);
        assert_eq!(
            pb.intout[0], 1,
            "the 3-D look sweep was refused for which {} -- objc_sysvar's set arm \
             must accept the init_3d defaults on this workstation",
            v[1]
        );
    }
}

// ── the application's message queue: state no declared reset sweeps ──────

/// `evnt_multi`'s event bits (`aes/gemlib.h`).
const MU_MESAG: i16 = 0x10;
const MU_TIMER: i16 = 0x20;

/// How many quick checks [`sweep_message_queue`] issues before it calls the
/// queue stuck. The AES pipe holds `QUEUE_SIZE` = 128 bytes
/// (`aes/struct.h`), at most eight standard 16-byte messages; anything that
/// still answers `MU_MESAG` after many times that is not a queue being
/// drained.
const DRAIN_LIMIT: usize = 64;

/// Empty the application's message queue, through the AES' own dequeue.
///
/// **Why this exists at all, and why it is not a [`Reset`].** Same nature as
/// [`sweep_3d_look`] and the trailing reset: a harness-only device the dumper
/// does not need, because it runs a table once, in order, on a freshly
/// started AES, while cargo runs one `#[test]` per case in an arbitrary order
/// inside one process. The table's declared reset does not cover it either:
/// `wm_new` closes and deletes windows and clears the `wind_update` locks
/// (`aes/gemwmlib.c:1874-1892`) and leaves the queue alone, so whatever a
/// case's window traffic posted to the application — the `WM_REDRAW` that
/// opening a window sends its owner (`w_redraw`'s `ap_sendmsg`,
/// `aes/gemwmlib.c:887-904`) — is still there for the next case to read.
///
/// Measured, not assumed: with `tosdump/cases/aes-m07-parameter.tsv` in the
/// binary and no such sweep, `cases_m07_parameter` next to `cases_m05_addrin`
/// alone turned case 69 (`MU_MESAG|MU_TIMER`, quick-check timer) red, or case
/// 70 if it ran first, answering `int_out[0] = 0x30` (resp. `0x32`) where the
/// TT answered `0x20` (`0x22`): the message was the `WM_REDRAW` for window 1's
/// work area that an m05 case's `wind_open` prelude had posted. Round 2
/// leaves the same message behind from every case that opens a window; it
/// only did not show because libtest's default order runs `cases_m07_*`
/// before `cases_r2_*`: with `--shuffle`, m07 next to `cases_r2_wind` was red
/// on five of five runs, cases 69 and 70 both.
///
/// **It is done through `evnt_multi`'s quick checks**, not by reaching into
/// `tos_api::aes`' process descriptors: `MU_MESAG|MU_TIMER` with a zero
/// `MT_LOCOUNT/MT_HICOUNT` sets `MU_TIMER` before the wait block, so the call
/// always returns, and the message quick check right after it dequeues one
/// message — `ev_mesag` — whenever `p_qindex > 0` (`aes/gemevlib.c:214-226`).
/// One message per call; the sweep ends on the first call that answers
/// `MU_TIMER` alone. `MME_PBUFF` is buffer 0, so the 16 bytes land in a
/// harness buffer that the next [`Pb::poison`] lays over again.
///
/// **The price, stated.** A case table can no longer measure a message that
/// *a previous case* posted; whatever a case reads from the queue it has to
/// post in its own preludes. And every case now starts and ends with at least
/// one quick-check `evnt_multi`, which sets `AP_MESAG` on the application's
/// descriptor (`gemsuper.c:147-148`) — a flag no table measures, and one the
/// first `evnt_multi(MU_MESAG)` of any table sets anyway. The cases still run
/// in parallel, serialised by [`AES_LOCK`] as before.
fn sweep_message_queue(pb: &mut Pb) {
    const PLANT: [Plant; 1] = [Plant { kind: Kind::AddrIn, slot: 0, buffer: 0 }];
    for _ in 0..DRAIN_LIMIT {
        pb.poison();
        // MU_FLAGS, then fifteen zeros: no clicks, two empty MOBLKs and
        // MT_LOCOUNT/MT_HICOUNT = 0 -- the quick-check timer
        // (`gemevlib.c:216-220`). IN_LEN 16, because a word outside it is
        // not the zero written here.
        pb.intin[..16].fill(0);
        pb.intin[0] = MU_MESAG | MU_TIMER;
        pb.plant(&PLANT);
        pb.call(25, 16, 1, 1, 0);
        let what = pb.intout[0];
        assert_eq!(
            what & MU_TIMER,
            MU_TIMER,
            "the message-queue sweep's quick check did not report MU_TIMER (int_out[0] = \
             0x{what:04X}) -- a zero tmcount must fire before the wait block"
        );
        if what & MU_MESAG == 0 {
            return;
        }
    }
    panic!(
        "the message-queue sweep still read a message after {DRAIN_LIMIT} quick checks -- \
         the queue holds at most eight"
    );
}

/// One case table and the dump taken from it — what a generated case module
/// declares once and every one of its cases is checked against.
///
/// `name` is load-bearing, not decoration: a case id is the **physical line
/// number** of the table it came from, so two tables reuse ids —
/// `tosdump/cases/aes-r1.tsv` and `tosdump/cases/aes-r2-wind.tsv` both have a
/// case 228, and they are different calls. Anything keyed on a case id
/// ([`DELTAS`]) is therefore keyed on the table as well.
pub struct Table {
    /// The case table's path from the repository root.
    pub name: &'static str,
    /// The dump produced from it, embedded by the generated module.
    pub dump: &'static [u8],
    /// The reset its `!reset` directive line declares.
    pub reset: Reset,
}

// ── the case ─────────────────────────────────────────────────────────────

/// A call issued before the measured one: it establishes the state the case
/// measures and is not recorded. Poisoned and laid exactly like a measured
/// call (`tosdump/core/runner_aes.c` step 1).
pub struct Call {
    pub opcode: i16,
    pub in_len: i16,
    pub out_len: i16,
    pub ain_len: i16,
    pub aout_len: i16,
    pub intin: &'static [i16],
    /// The table's `addr_in` column ([`Plant`]), empty for a seven-column
    /// table. A prelude is the only line that may carry a [`Kind::IntIn`]
    /// plant, because it is not recorded.
    pub plant: &'static [Plant],
}

/// One case, as `tosdump/host/mkrusttests` emits it. The four counts are
/// `control[1..4]` **as the table declares them**, not derived from `intin`:
/// several cases exist precisely to measure a count that disagrees with the
/// values listed (`aes/gemsuper.c:428-448`).
pub struct Case {
    pub prelude: &'static [Call],
    pub id: u16,
    pub rec: usize,
    pub opcode: i16,
    pub in_len: i16,
    pub out_len: i16,
    pub ain_len: i16,
    pub aout_len: i16,
    pub intin: &'static [i16],
    /// The table's `addr_in` column ([`Plant`]), empty for a seven-column
    /// table. [`Kind::AddrIn`] only: a packed `int_in` pointer occupies two
    /// WORDs on the m68k and four here, and `int_in` is recorded, so the case
    /// loader refuses one on a measured line (`tosdump/core/cases.c`,
    /// `check_plants`).
    pub plant: &'static [Plant],
    pub comment: &'static str,
}

struct Diff {
    field: &'static str,
    index: usize,
    got: i64,
    want: i64,
    hex_width: usize,
    /// True when `want` is not the reference's number but a substitute — the
    /// message has to say so, or it claims the dump decided something it
    /// cannot.
    substitute: bool,
}

fn push_diff(out: &mut Vec<Diff>, field: &'static str, index: usize, got: i64, want: i64, w: usize) {
    if got != want {
        out.push(Diff { field, index, got, want, hex_width: w, substitute: false });
    }
}

fn push_subst(out: &mut Vec<Diff>, field: &'static str, index: usize, got: i64, want: i64, w: usize) {
    if got != want {
        out.push(Diff { field, index, got, want, hex_width: w, substitute: true });
    }
}

/// Everything one comparison needs that is not in the record: which table and
/// case it belongs to (a [`Delta`] is keyed on both, because a case id is only
/// a line number and two tables reuse them), and which [`DELTAS`] entries the
/// comparison actually consulted.
struct Oracle {
    table: &'static str,
    case: u16,
    /// One flag per [`DELTAS`] entry, set when [`Oracle::want`] used it. An
    /// entry naming a field or an index no comparison visits would otherwise
    /// be a silent skip; [`Oracle::finish`] refuses that.
    used: [bool; DELTAS.len()],
    /// The same, for [`SANCTIONED`].
    used_sanctioned: [bool; SANCTIONED.len()],
}

impl Oracle {
    fn new(table: &'static str, case: u16) -> Oracle {
        Oracle {
            table,
            case,
            used: [false; DELTAS.len()],
            used_sanctioned: [false; SANCTIONED.len()],
        }
    }

    /// What one recorded word is compared against: normally the record's own
    /// word, and where a [`Delta`] covers the field the value the pinned
    /// EmuTOS source mandates instead.
    ///
    /// This is a **substitution**, not a filter over differences that already
    /// exist. An expectation applied only after the recorded value had been
    /// contradicted would let NewTOS answer exactly what the older EmuTOS
    /// answered and still pass — which is the one regression a [`Delta`]
    /// exists to catch.
    fn want(&mut self, field: &'static str, index: usize, rec: &[u8], off: usize) -> (i64, bool) {
        let recorded = be16(rec, off + index * 2) as i64;
        for (k, d) in DELTAS.iter().enumerate() {
            if d.table != self.table || d.case != self.case || d.field != field || d.index != index
            {
                continue;
            }
            // The entry states what the older EmuTOS wrote. If the dump no
            // longer says that, the delta describes a measurement that no
            // longer exists and must be re-stated rather than silently
            // overriding a record it does not match.
            assert_eq!(
                recorded, d.recorded,
                "{} case {}: the DELTAS entry for {}[{}] claims the reference recorded \
                 {}, but the dump holds {} -- re-state or remove the entry",
                self.table, self.case, field, index, d.recorded, recorded
            );
            self.used[k] = true;
            return (d.expected.value(), true);
        }
        for (k, s) in SANCTIONED.iter().enumerate() {
            if s.table != self.table || s.case != self.case || s.field != field || s.index != index
            {
                continue;
            }
            // As above: the entry states what the TT wrote. A dump that no
            // longer says that is a different measurement, and the deviation
            // has to be re-stated against it rather than override it blind.
            assert_eq!(
                recorded, s.recorded,
                "{} case {}: the SANCTIONED entry for {}[{}] claims the reference recorded \
                 {}, but the dump holds {} -- re-state or remove the entry",
                self.table, self.case, field, index, s.recorded, recorded
            );
            // A sanctioned deviation that has quietly become a match is no
            // longer a deviation: the register in docs/COMPATIBILITY.md would
            // be describing something that is not happening.
            assert_ne!(
                s.answer, s.recorded,
                "{} case {}: the SANCTIONED entry for {}[{}] answers exactly what the \
                 reference recorded -- that is not a deviation, delete the entry",
                self.table, self.case, field, index
            );
            self.used_sanctioned[k] = true;
            return (s.answer, true);
        }
        (recorded, false)
    }

    /// Every [`Delta`] and every [`Sanctioned`] entry that names this table and
    /// case must have been used.
    fn finish(&self) {
        for (k, d) in DELTAS.iter().enumerate() {
            if d.table == self.table && d.case == self.case && !self.used[k] {
                panic!(
                    "{} case {}: the DELTAS entry for {}[{}] was never consulted -- it \
                     names a field or an index this comparison does not visit, so it \
                     is an exception that excepts nothing",
                    self.table, self.case, d.field, d.index
                );
            }
        }
        for (k, s) in SANCTIONED.iter().enumerate() {
            if s.table == self.table && s.case == self.case && !self.used_sanctioned[k] {
                panic!(
                    "{} case {}: the SANCTIONED entry for {}[{}] was never consulted -- it \
                     names a field or an index this comparison does not visit, so it \
                     is an exception that excepts nothing",
                    self.table, self.case, s.field, s.index
                );
            }
        }
    }
}

fn cmp_words(
    out: &mut Vec<Diff>,
    or: &mut Oracle,
    field: &'static str,
    got: &[i16],
    rec: &[u8],
    off: usize,
) {
    for (i, &g) in got.iter().enumerate() {
        let (want, subst) = or.want(field, i, rec, off);
        if subst {
            push_subst(out, field, i, g as i64, want, 4);
        } else {
            push_diff(out, field, i, g as i64, want, 4);
        }
    }
}

// ── the `global` array: what is compared, and what is not ────────────────

/// `global` word indices that are **not** compared, each because the value is
/// a property of the machine the dump was taken on and not of the AES. Every
/// other word — and the `ap_private` LONG, see [`cmp_global`] — is compared.
///
/// * **`global[2]` — `ap_id` — is compared, since 2026-09-20 (A56).** It
///   used to be excluded here with the argument that the number was "a
///   property of the process model": EmuTOS reports 0 (`pglobal->ap_id =
///   rlr->p_pid`, `aes/gemsuper.c:94`; the whole application chain runs on
///   `D.g_int[0]`, `aes/geminit.c:850-877`) and NewTOS reported the GEMDOS
///   slot + 1. That argument licensed a divergence instead of naming it:
///   the value **is** contract — `docs/api/aes/application.md` § "AES
///   process identity and pid", (b) — and NewTOS now reports 0 for every
///   member of the chain, the kernel-context desktop this harness runs as
///   included. The record's 0 is therefore compared as it stands, and the
///   `ap_id == appl_init ret` property [`cmp_global`] also checks is kept
///   (both are `rlr->p_pid`, `gemaplib.c:54-63`).
/// * **`global[11]`, `global[12]` — `ap_3resv`.** EmuTOS:
///   `pglobal->ap_3resv = (LONG)&D` (`aes/gemsuper.c:98`), the address of the
///   AES's own global data area (`aes/gemrslib.h:25`). An address: 0x0000BE04
///   on the TT, the AES process-descriptor table's address on NewTOS
///   (`tos-api/src/aes/appl.rs`, `pd::table_addr()`). That it is an address
///   and not a value is directly measurable in the reference set — between
///   `NEWTOS-AES.BIN` and `NEWTOS-AES-r2.BIN`, two runs of the *same* binary
///   against the *same* table, `global[11]` alone moved from 0x80B0 to
///   0x80D8.
///
/// `ap_3resv` is not left unchecked. [`cmp_global`] compares the
/// machine-independent part instead of the number: it must be non-zero
/// exactly when the reference's is, i.e. `appl_init` must have put *an*
/// address there.
///
/// **These two are the only ones, and the other `global` words the two
/// reference runs disagree about are compared as they stand.** `tosdiff`
/// reports six differing words in all 78 round-2 records
/// (`tosdump/reference/*-AES-r2wind.LOG`): `ap_id` (2), the `ap_private` LONG
/// (3–4), `ap_planes` (10) and the `ap_3resv` LONG (11–12). One of the four
/// fields is excluded above; `ap_id` is compared and passes since A56 (see
/// above); the remaining two are compared, and pass, for reasons that are
/// worth stating because they are conditions on the harness rather than
/// luck:
///
/// * **`ap_planes`** — TT 4, Pi 16. It is the workstation's plane count, and
///   this harness supplies a workstation shaped like the TT's (see the module
///   header). A harness that opened a 16-plane screen would have to exclude
///   it; this one measures it.
/// * **`ap_private`** — TT 4419 (`0x1143`), Pi 4475 (`0x117B`). Both are
///   `rs_trees[DESKTOP][ROOT].ob_spec` (`aes/gemsuper.c:95-96`), i.e. `0x1100`
///   with the `.INF` `#Q` background byte ORed into the low byte
///   (`aes/geminit.c:635-641,719-728`). That low byte is **data, not code**:
///   EmuTOS ships `#Q 41 40 43 40 43 40` (`desk/deskapp.c:175`) and the Pi's
///   card carried `7B` there. The host replay reads no `NEWDESK.INF` at all,
///   so `bgfound` is FALSE, `set_aes_background` never runs and the
///   compiled-in `DESKTOP_ROOT_OBSPEC` stands — which is the same `0x1143` the
///   TT's shipped file produced. So the value **is** comparable here, and it
///   is compared. The chain from `#Q` line to `ap_private` is pinned
///   separately by `tos-api/src/aes/init.rs`'s
///   `ap_private_is_the_desktop_root_obspec_with_the_inf_background_byte`.
pub const GLOBAL_EXCLUDED: [usize; 2] = [11, 12];

/// Compare the caller's `AESGLOBAL` against the record.
///
/// Fifteen WORDs, but not fifteen independent values: `ap_private`
/// (`gemrslib.h:19`) is a `LONG`, and a `LONG` occupies its two WORD slots in
/// the machine's byte order. The m68k reference holds it high-word-first at
/// `global[3..4]`; a little-endian AArch64 caller holds the same *value*
/// low-word-first. Comparing those two slots word-for-word would therefore
/// report a difference for a value that is identical. So the WORD fields are
/// compared as WORDs and `ap_private` is compared as the LONG it is — the
/// reference's big-endian 32 bits against **four** bytes at byte 6 here.
/// Four on every ABI: `AESGLOBAL` keeps the m68k field widths, because it is
/// addressed by byte offset and by WORD index at once (decision of
/// 2026-09-13, `tos-api/src/aes/appl.rs` module ABI note), and NewTOS's
/// gemlib overlay narrows `_AESappglobal` (`gem.h:30`) to match.
///
/// The words that carry no `appl_init` output — `ap_ptree` (5–6),
/// `ap_rscmem` (7–8), `ap_rsclen` (9), `ap_4resv` (13–14) — **are** compared,
/// word for word and independently of `ap_private`: EmuTOS's `APPL_INIT` arm
/// does not name them (`gemsuper.c:92-98`), both arrays start zeroed, so a
/// difference there means somebody wrote where EmuTOS does not.
fn cmp_global(out: &mut Vec<Diff>, got: &[i16; GLOBAL_N], rec: &[u8], ap_id: i16) {
    // `ap_id` is compared against the record in the loop below (A56); the
    // "agrees with appl_init's return" property is kept as well — both are
    // `rlr->p_pid` (`gemaplib.c:54-63`, `gemsuper.c:94`). The excluded
    // field is checked by the property that does not depend on the machine
    // (see [`GLOBAL_EXCLUDED`]).
    push_subst(out, "ap_id == appl_init ret", 0, got[2] as i64, ap_id as i64, 4);
    let ref_3resv = ((be16(rec, OFF_GLOBAL + 11 * 2) as u16 as u32) << 16)
        | be16(rec, OFF_GLOBAL + 12 * 2) as u16 as u32;
    let my_3resv = ((got[11] as u16 as u32) as u64) | ((got[12] as u16 as u32) as u64) << 16;
    push_subst(
        out,
        "ap_3resv is an address",
        0,
        i64::from(my_3resv != 0),
        i64::from(ref_3resv != 0),
        4,
    );

    // ap_private as a value: the reference's big-endian m68k LONG at words
    // 3–4 against the four-byte LONG at byte 6.
    let want = ((be16(rec, OFF_GLOBAL + 3 * 2) as u16 as u32) << 16)
        | be16(rec, OFF_GLOBAL + 4 * 2) as u16 as u32;
    // SAFETY: `got` is 15 WORDs = 30 bytes; byte 6 + 4 = 10 is inside it. The
    // slot is only 2-byte aligned, hence the unaligned read.
    let mine = unsafe { (got.as_ptr() as *const u8).add(6).cast::<u32>().read_unaligned() };
    push_diff(out, "ap_private", 0, mine as i64, want as i64, 8);

    for (i, &g) in got.iter().enumerate() {
        if GLOBAL_EXCLUDED.contains(&i) || i == 3 || i == 4 {
            continue; // 3–4 are the ap_private LONG, compared above
        }
        push_diff(out, "global", i, g as i64, be16(rec, OFF_GLOBAL + i * 2) as i64, 4);
    }
}

// ── where the recorded reference is not the oracle ───────────────────────

/// What a [`Delta`] says the pinned EmuTOS source mandates in place of the
/// recorded value.
///
/// Until 2026-09-20 (A56) there was a second variant, `ApId` — "this
/// application's own id, whatever `appl_init` answered on this machine" —
/// used for `WF_OWNER`'s owner pid, on the argument that the number was a
/// property of the process model. It is not: `w_owner->p_pid` is 0 for every
/// application window (`aes/gemwmlib.c:1580`; `w_owner = rlr` at
/// `wind_create`, `:1377`, and the whole chain is pid 0), and NewTOS now
/// reports exactly that — so the entries pin the literal, as every other
/// delta does (`docs/api/aes/application.md` § "AES process identity and
/// pid", contract (b)).
#[derive(Copy, Clone, Debug)]
pub enum Expected {
    /// A literal, when the pinned source computes one.
    Value(i64),
}

impl Expected {
    fn value(self) -> i64 {
        match self {
            Expected::Value(v) => v,
        }
    }
}

/// One field of one case where **the build the dump was taken from is older
/// than the EmuTOS source this project implements against** (`references/emutos`,
/// pinned at `1a7484ab`, 2026-04-14) -- as EmuTOS 1.4 was for the `EMUTOS14-*`
/// dumps. The field is still
/// compared; only the value it is compared *against* changes, and the entry
/// has to say exactly why. Nothing is skipped:
///
/// * the recorded value is asserted, so an entry cannot describe a
///   measurement the dump does not hold ([`Oracle::want`]);
/// * the expectation is *substituted*, not applied as an afterthought, so
///   NewTOS producing the old EmuTOS's answer fails ([`Oracle::want`]);
/// * an entry the comparison never reaches is refused ([`Oracle::finish`]).
pub struct Delta {
    /// The case table this entry belongs to, by the name its [`Table`]
    /// carries. A case id alone is ambiguous: it is a physical line number,
    /// and round 1 and round 2 both have a case 228.
    pub table: &'static str,
    /// Case id = the physical line number in that table.
    pub case: u16,
    pub field: &'static str,
    pub index: usize,
    /// What the machine the dump was taken on actually wrote.
    pub recorded: i64,
    /// What the pinned EmuTOS source mandates, i.e. what NewTOS must produce.
    pub expected: Expected,
    pub why: &'static str,
}

/// Empty since the reference is the pinned source itself (2026-10-26).
///
/// The list had one subject, `WF_OWNER` (20): round 1 case 167 and round 2
/// cases 244/247. EmuTOS 1.4 had no `WF_OWNER` arm, answered `RET_CODE = 0`
/// and left `xif`'s uninitialised `int_out` behind it; the source this project
/// implements against gained the arm in `acb7f3b4` ("Add partial implementation
/// of WF_OWNER to wind_get", 2025-08-23, `aes/gemwmlib.c:1578-1589`). Since
/// the dumps are measured on the oracle built from that source
/// (`tosdump/reference/EMUTOS-1a7484ab-*`, `docs/analysis/ORACLE.md`) they
/// record exactly what the entries used to substitute -- 167: `1`; 244:
/// `1, 0, 0, -1, -1`; 247: `1, 0, 1, 1, 1` -- so the cases are compared
/// against the dump as they stand.
///
/// A new entry belongs here only if the oracle's pin moves behind the source
/// again; the mechanism ([`Oracle::want`], [`Oracle::finish`]) stays for that.
pub const DELTAS: &[Delta] = &[];

/// One field of one case where **NewTOS deliberately answers something else
/// than EmuTOS**, because a human decided so and wrote it down in
/// `docs/COMPATIBILITY.md`'s "Sanctioned deviations, one by one".
///
/// This is *not* a second [`DELTAS`]. A [`Delta`] says "the dump is older than
/// the source; NewTOS must produce what the source mandates" — the reference is
/// still the arbiter, only a newer edition of it. An entry **here** says the
/// opposite: the pinned source and NewTOS disagree, knowingly, and the register
/// in `docs/COMPATIBILITY.md` carries the reason, the date and who decided.
/// The standing rule remains *match EmuTOS*; a case landing here is a gap that
/// is answered honestly rather than papered over, not a licence.
///
/// It is held to the same three rules as [`DELTAS`], so it cannot become a
/// quiet way to make red go green:
///
/// * the recorded value is asserted, so an entry cannot describe a measurement
///   the dump does not hold ([`Oracle::want`]);
/// * the answer is *substituted*, so NewTOS drifting to a third value — the
///   EmuTOS one included — fails ([`Oracle::want`]);
/// * an entry the comparison never reaches is refused ([`Oracle::finish`]).
///
/// The failure message marks a substituted word as `expected`, never as
/// `reference`, so no report can claim the dump decided it ([`Diff`]).
pub struct Sanctioned {
    /// The case table this entry belongs to, by the name its [`Table`] carries.
    pub table: &'static str,
    /// Case id = the physical line number in that table.
    pub case: u16,
    pub field: &'static str,
    pub index: usize,
    /// What the machine the dump was taken on actually wrote.
    pub recorded: i64,
    /// What NewTOS answers instead, per the `docs/COMPATIBILITY.md` entry.
    pub answer: i64,
    pub why: &'static str,
}

/// The whole list — **empty**. Its one entry, round-1 case 166
/// (`wind_get(0, WF_SCREEN)`, `OUT_LEN 1`), left with A57: the AES has the
/// persistent menu/alert screen buffer `gsx_malloc` takes
/// (`tos_api::aes::gsxif`; on this host, the fixture [`install_screen_buffer`]
/// in [`context`]), so `wm_get`'s field 17 answers TRUE out of `gsx_mret`
/// again (`aes/gemwmlib.c:1563-1565`) and the case compares directly against
/// the dump. The A54 register entry in `docs/COMPATIBILITY.md` narrowed to the
/// buffer-less fallback, which no case of any table exercises.
///
/// The type and the three rules stay, so the next honest gap has a named
/// place to go and cannot become a quiet way to make red go green.
pub const SANCTIONED: &[Sanctioned] = &[];

/// Replay one case of `table` and compare every array against the reference.
pub fn check(table: &Table, case: &Case) {
    let _guard = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let ctx = context();
    let mut pb = Pb::new();

    // 0a. the state no declared reset sweeps, put back where `init_3d` left
    //     it. Not the dumper's doing — see `sweep_3d_look` for why cargo's
    //     arbitrary case order needs it and what it costs.
    sweep_3d_look(&mut pb);
    sweep_message_queue(&mut pb);

    // 0b. the reset the table declares (`tosdump/core/runner_aes.c:150-154`),
    //    ahead of the preludes, so a prelude establishes state on top of a
    //    known one rather than on top of whatever ran before it.
    table.reset.run(&mut pb);

    // 1. the case's preludes, in order, exactly as `runner_aes.c` issues
    //    them: each poisoned and laid like a measured call, none recorded.
    for p in case.prelude {
        pb.poison();
        pb.intin[..p.intin.len()].copy_from_slice(p.intin);
        pb.plant(p.plant);
        pb.call(p.opcode, p.in_len, p.out_len, p.ain_len, p.aout_len);
    }

    let rec = &table.dump[HDR_SIZE + case.rec * REC_SIZE..][..REC_SIZE];
    assert_eq!(
        be16(rec, OFF_CASEID) as u16,
        case.id,
        "record {} is case {}, expected case {} -- {} and generated tests disagree",
        case.rec,
        be16(rec, OFF_CASEID),
        case.id,
        table.name
    );
    assert_eq!(
        be16(rec, OFF_OPCODE),
        case.opcode,
        "{} case {}: record opcode {} vs case opcode {}",
        table.name,
        case.id,
        be16(rec, OFF_OPCODE),
        case.opcode
    );

    // 2. poison; 3. lay the case's int_in values over it; 3a. the buffers its
    //    addr_in column names; 4. the call.
    pb.poison();
    pb.intin[..case.intin.len()].copy_from_slice(case.intin);
    for p in case.plant {
        assert_eq!(
            p.kind,
            Kind::AddrIn,
            "{} case {}: a measured case may not pack a pointer into int_in -- it is \
             recorded, and the two targets disagree about how many WORDs it takes",
            table.name,
            case.id
        );
    }
    pb.plant(case.plant);
    pb.call(case.opcode, case.in_len, case.out_len, case.ain_len, case.aout_len);

    let mut or = Oracle::new(table.name, case.id);
    let mut diffs = Vec::new();
    cmp_words(&mut diffs, &mut or, "contrl", &pb.contrl[..], rec, OFF_CONTRL);
    cmp_global(&mut diffs, pb.global(), rec, ctx.ap_id);
    cmp_words(&mut diffs, &mut or, "int_in", &pb.intin[..], rec, OFF_INTIN);
    cmp_words(&mut diffs, &mut or, "int_out", &pb.intout[..], rec, OFF_INTOUT);
    // The record carries the low 32 bits of each address slot, because the
    // format's slots are 32 bit while the block's are the target's pointer
    // width (`tosdump/core/runner_aes.c`) — except a slot a case pointed at
    // one of the four buffers, which records the buffer TOKEN instead
    // ([`Pb::recorded_addrin`], [`ADDR_TOKEN_BASE`]).
    for i in 0..ADDRIN_N {
        let got = pb.recorded_addrin(i) as i64;
        push_diff(&mut diffs, "addr_in", i, got, be32(rec, OFF_ADDRIN + i * 4) as i64, 8);
    }
    for (i, &a) in pb.addrout.iter().enumerate() {
        push_diff(&mut diffs, "addr_out", i, a as u32 as i64, be32(rec, OFF_ADDROUT + i * 4) as i64, 8);
    }

    // 5. the declared reset again, and the 3-D look sweep with it, BEFORE
    //    anything below can panic: a red case that left a window open — or a
    //    3-D variable set — would turn every case cargo happens to run after
    //    it red as well, and the first failure is the only one worth reading.
    table.reset.run(&mut pb);
    sweep_3d_look(&mut pb);
    sweep_message_queue(&mut pb);
    or.finish();

    if !diffs.is_empty() {
        let mut msg = format!(
            "{} case {} (opcode {}) -- {}\n  {} field(s) differ from the EmuTOS \
             1.4 reference:\n",
            table.name,
            case.id,
            case.opcode,
            case.comment,
            diffs.len()
        );
        for d in DELTAS.iter().filter(|d| d.table == table.name && d.case == case.id) {
            msg += &format!(
                "  note: `{}[{}]` is compared against what the pinned EmuTOS source \
                 mandates, not against the dump (which holds {}):\n        {}\n",
                d.field, d.index, d.recorded, d.why
            );
        }
        for s in SANCTIONED.iter().filter(|s| s.table == table.name && s.case == case.id) {
            msg += &format!(
                "  note: `{}[{}]` is compared against a SANCTIONED deviation, not against \
                 the dump (which holds {}):\n        {}\n",
                s.field, s.index, s.recorded, s.why
            );
        }
        for d in diffs.iter().take(24) {
            let source = if d.substitute { "expected" } else { "reference" };
            msg += &format!(
                "    {}[{}]  NewTOS = 0x{:0w$X} ({})   {} = 0x{:0w$X} ({})\n",
                d.field,
                d.index,
                d.got & ((1i64 << (d.hex_width * 4)) - 1),
                d.got,
                source,
                d.want & ((1i64 << (d.hex_width * 4)) - 1),
                d.want,
                w = d.hex_width
            );
        }
        if diffs.len() > 24 {
            msg += &format!("    ... and {} more\n", diffs.len() - 24);
        }
        // The trap's `d0` is recorded at OFF_RET and compared by
        // `tosdump/host/build/tosdiff`; `tos_api::aes::dispatch` does not
        // return it, so it is out of reach here.
        let _ = OFF_RET;
        panic!("{}", msg);
    }
}
