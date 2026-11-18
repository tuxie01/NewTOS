//! V11 / V12 — `v_clrwk` and the `OVERLAY_BIT`, read back through
//! `v_get_pixel` (VDI-OVL-001), pinned against the **Falcon truecolor**
//! EmuTOS reference measured 2026-09-21.
//!
//! **The EmuTOS contract (Falcon030 / TOS4 target profile, `TRUECOLOR_MODE`
//! = `v_planes > 8`, `vdi/vdi_defs.h:22`):**
//!
//! * `v_clrwk` clears the whole screen with `memset(v_bas_ad, 0xff, size)`
//!   under truecolor (`vdi/vdi_control.c:599-618`), i.e. every screen word
//!   becomes `0xFFFF`.
//! * `v_get_pixel` returns `INTOUT[0] = 0`, `INTOUT[1] = pixelread(x, y)`,
//!   and `pixelread` returns the screen word **unmasked** under truecolor
//!   (`vdi/vdi_fill.c:671-681`, `:1095-1115`).
//! * The pseudo-palette never sets bit 5: `set_color16` packs
//!   `(r << 11) | (g << 6) | b` from 5-bit guns (`vdi/vdi_col.c:601-614`),
//!   so pen 0 (white, 1000/1000/1000) is `0xFFDF`, and the cleared screen
//!   differs from a pen-0 fill by exactly `OVERLAY_BIT` (`0x0020`,
//!   `vdi/vdi_fill.c:26`).
//!
//! **Measured, not reasoned:** EmuTOS 1.4 (release ROM, and bit-identically
//! the pinned source build) on Hatari Falcon in `VIDEL_TRUECOLOR` at 640×480
//! answers `vdi-all.tsv` cases 54, 160 and 161 (`v_get_pixel` at (5,5),
//! (0,0), (639,479) after the reset's `v_clrwk`) with `INTOUT[1] = 0xFFFF`,
//! and cases 270/272 (a pixel drawn in pen 1) with `0x0000` —
//! `measurements/measurement-2026-09-21-vdi-qemu/emutos-falcon-tc-hatari/`, reference
//! copy `tosdump/reference/EMUTOS14-VDI-all-falcontc.BIN`. NewTOS on QEMU
//! `raspi4b` at the same geometry answers `0xFFDF` for the three cleared
//! pixels (`…/newtos-qemu-raspi4b/VDI.BIN`,
//! `DIFF-emutos-falcontc-rel-vs-newtos-qemu.txt`) — the only three cells of
//! 182 cases that differ from the Falcon truecolor reference.
//!
//! **Why this is its own test binary with its own backend.** The defect
//! lived in the kernel's device-pixel model (`kernel/src/vdi_backend.rs`),
//! which is not host-linkable: until 2026-09-21 the framebuffer word was
//! XRGB8888 with the X byte fixed at `0xFF`, `rgb565_to_device` expanded
//! three 5-bit guns and `device_to_rgb565` took the top 5 bits of each gun
//! back, so bit 5 of the 16-bit word had no place in the pixel and `0xFFFF`
//! and `0xFFDF` landed on the same `0xFFFFFFFF`. [`XrgbDevice`] below is
//! the kernel's model, transcribed expression for expression (line numbers
//! at each function), so the real `tos_api::vdi` dispatch path can be
//! driven against it on the host. `tos_api::host::install_host` accepts one
//! host per process, which is why this cannot share the `vdi_reference`
//! binary (whose double, `harness.rs` `MemBackend`, is a plainer
//! rasteriser over the same two conversions).
//!
//! **The fix (V12, 2026-09-21)** is the kernel's device-pixel model:
//! `OVERLAY_BIT` is kept in the X byte of the 32-bpp store, replicated
//! into all eight bits like the guns (`0xFF` set / `0x00` clear;
//! `kernel/src/vdi_backend.rs` `word_to_device`/`device_to_word`,
//! `kernel/src/hal/framebuffer.rs` module doc). **Keep the transcription
//! below in sync with those two functions** — the tests are only as
//! faithful as the copy.

#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

const W: u16 = 640;
const H: u16 = 480;

/// `OVERLAY_BIT`, `vdi/vdi_fill.c:26`.
const OVERLAY_BIT: u16 = 0x0020;
/// The cleared truecolor screen word: `memset(v_bas_ad, 0xff, …)`,
/// `vdi/vdi_control.c:610-618`.
const CLEARED_WORD: u16 = 0xFFFF;
/// Pen 0 (white) through `set_color16`: `(31 << 11) | (31 << 6) | 31`,
/// `vdi/vdi_col.c:601-614`.
const PEN0_WHITE_WORD: u16 = 0xFFDF;

// ── the kernel's device-pixel model, transcribed ─────────────────────────
//
// Keep in sync with `kernel/src/vdi_backend.rs` (2026-09-21 stand).

/// `kernel/src/hal/framebuffer.rs:31-35` — a plain colour, X = 0x00.
const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// `kernel/src/vdi_backend.rs:172-174`.
const fn expand5to8(c5: u16) -> u8 {
    ((c5 << 3) | (c5 >> 2)) as u8
}

/// `kernel/src/vdi_backend.rs:191` — the X byte, all eight bits.
const OVERLAY_X: u32 = 0xFF00_0000;

/// `kernel/src/vdi_backend.rs:199-206` (`word_to_device`), which is what
/// `VdiScreen::rgb565_to_device` calls (`:452-457`).
fn word_to_device(word: u16) -> u32 {
    let x = if word & OVERLAY_BIT != 0 { OVERLAY_X } else { 0 };
    x | rgb(expand5to8((word >> 11) & 0x1f), expand5to8((word >> 6) & 0x1f), expand5to8(word & 0x1f))
}

/// `kernel/src/vdi_backend.rs:212-218` (`device_to_word`), which is what
/// `VdiScreen::device_to_rgb565` calls (`:438-450`).
fn device_to_word(px: u32) -> u16 {
    let r5 = ((px >> 19) & 0x1f) as u16;
    let g5 = ((px >> 11) & 0x1f) as u16;
    let b5 = ((px >> 3) & 0x1f) as u16;
    let o = ((px >> 26) & OVERLAY_BIT as u32) as u16; // bit 31 → bit 5
    (r5 << 11) | (g5 << 6) | b5 | o
}

/// EmuTOS' `videl_palette1` (`vdi/vdi_col.c:130-136`) as VDI 0..=1000
/// triples — the device default the VDI seeds every workstation's
/// pseudo-palette from at open (`init_wk`, `vdi_control.c:354-363`;
/// NewTOS reads it back through `query_color`). The kernel backend seeds
/// the same sixteen (`kernel/src/vdi_backend.rs`, `ensure_palette_init`).
/// Pen 0 is white, pen 1 black.
#[rustfmt::skip]
const VIDEL_PALETTE1: [(u16, u16, u16); 16] = [
    (1000, 1000, 1000), (0, 0, 0),       (1000, 0, 0),    (0, 1000, 0),
    (0, 0, 1000),       (0, 1000, 1000), (1000, 1000, 0), (1000, 0, 1000),
    (733, 733, 733),    (533, 533, 533), (667, 0, 0),     (0, 667, 0),
    (0, 0, 667),        (0, 667, 667),   (667, 667, 0),   (667, 0, 667),
];

/// A 640×480 XRGB8888 framebuffer with the kernel's two 16↔32-bit
/// conversions. Everything else is the plainest possible rasteriser; the
/// tests only fill rectangles and read pixels back.
pub struct XrgbDevice {
    px: Mutex<Vec<u32>>,
    palette: Mutex<[u16; 256]>,
    req: Mutex<[(u16, u16, u16); 256]>,
}

impl XrgbDevice {
    fn new() -> Self {
        let mut req = [(0u16, 0u16, 0u16); 256];
        req[..16].copy_from_slice(&VIDEL_PALETTE1);
        XrgbDevice {
            px: Mutex::new(vec![0; W as usize * H as usize]),
            palette: Mutex::new([0; 256]),
            req: Mutex::new(req),
        }
    }
    fn inside(x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as i32) < W as i32 && (y as i32) < H as i32
    }
    fn idx(x: i16, y: i16) -> usize {
        y as usize * W as usize + x as usize
    }
    fn put(&self, x: i16, y: i16, v: u32, clip: ClipRect) {
        if !Self::inside(x, y) {
            return;
        }
        if let Some((x0, y0, x1, y1)) = clip.clamp_to(W, H) {
            let (xi, yi) = (x as i32, y as i32);
            if xi < x0 || xi > x1 || yi < y0 || yi > y1 {
                return;
            }
        }
        self.px.lock().unwrap()[Self::idx(x, y)] = v;
    }
    fn pel_device(&self, pel: Pel) -> u32 {
        self.rgb565_to_device(self.pel_to_device16(pel))
    }
}

impl RasterBackend for XrgbDevice {
    fn width(&self) -> u16 {
        W
    }
    fn height(&self) -> u16 {
        H
    }
    /// `kernel/src/vdi_backend.rs:33,40`: 256 pens over 16 planes.
    fn num_colors(&self) -> u16 {
        256
    }
    fn planes(&self) -> u16 {
        16
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
    fn set_active_palette(&self, palette: &[u32; 256]) {
        *self.palette.lock().unwrap() = palette.map(|p| p as u16);
    }
    fn pel_to_device16(&self, pel: Pel) -> u16 {
        self.palette.lock().unwrap()[pel as usize]
    }
    fn device_to_rgb565(&self, argb: u32) -> u16 {
        device_to_word(argb)
    }
    fn rgb565_to_device(&self, px: u16) -> u32 {
        word_to_device(px)
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
        let v = self.pel_device(pel);
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
        let v = self.pel_device(pel);
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
        let fg = self.pel_device(fill.fg);
        let bg = self.pel_device(fill.bg);
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
        let v = self.pel_device(fg);
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
        let f = self.pel_device(fg);
        let b = self.pel_device(bg);
        for row in 0..dst.h {
            for col in 0..dst.w {
                let (sx, sy) = (src.x + col, src.y + row);
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

// ── host + workstation ───────────────────────────────────────────────────

pub struct TestHost {
    raster: XrgbDevice,
}

impl Host for TestHost {
    fn console_write_byte(&self, _byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
}

// SAFETY: every field is behind a Mutex.
unsafe impl Sync for TestHost {}
unsafe impl Send for TestHost {}

static VDI_LOCK: Mutex<()> = Mutex::new(());
static HOST_CELL: OnceLock<&'static TestHost> = OnceLock::new();
static HOST_REF: OnceLock<&'static dyn Host> = OnceLock::new();
static HANDLE: OnceLock<i16> = OnceLock::new();

const ABI: Abi = Abi::Lp64;

struct Pb {
    contrl: Box<[i16; 12]>,
    intin: Box<[i16; 128]>,
    ptsin: Box<[i16; 256]>,
    intout: Box<[i16; 128]>,
    ptsout: Box<[i16; 256]>,
    block: Box<[u64; 5]>,
}

impl Pb {
    fn new() -> Pb {
        let mut pb = Pb {
            contrl: Box::new([0; 12]),
            intin: Box::new([0; 128]),
            ptsin: Box::new([0; 256]),
            intout: Box::new([0; 128]),
            ptsout: Box::new([0; 256]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    /// Issue one VDI call through the real dispatcher, exactly as the trap
    /// would: `contrl[0]` opcode, `[1]` # PTSIN pairs, `[3]` # INTIN,
    /// `[5]` sub-opcode, `[6]` handle.
    fn call(&mut self, opcode: i16, sub: i16, handle: i16, intin: &[i16], ptsin: &[i16]) -> i16 {
        self.contrl.fill(0);
        self.intout.fill(0x7EAD);
        self.ptsout.fill(0x7EAD);
        self.intin[..intin.len()].copy_from_slice(intin);
        self.ptsin[..ptsin.len()].copy_from_slice(ptsin);
        self.contrl[0] = opcode;
        self.contrl[1] = (ptsin.len() / 2) as i16;
        self.contrl[3] = intin.len() as i16;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        // SAFETY: `block` holds five live pointers to arrays owned by `self`.
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, ABI) }
    }
}

/// Open the physical workstation and one virtual workstation on it with
/// the dumper's `work_in` (all attributes 1, raster coordinates), once per
/// process. Returns the virtual handle the cases use.
fn workstation() -> i16 {
    *HANDLE.get_or_init(|| {
        let host: &'static TestHost =
            HOST_CELL.get_or_init(|| Box::leak(Box::new(TestHost { raster: XrgbDevice::new() })));
        let href: &'static &'static dyn Host =
            Box::leak(Box::new(*HOST_REF.get_or_init(|| host as &'static dyn Host)));
        tos_api::host::install_host(href);

        let mut pb = Pb::new();
        let mut work_in = [1i16; 11];
        work_in[10] = 2;
        pb.call(1, 0, 0, &work_in, &[]); // v_opnwk
        let phys = pb.contrl[6];
        pb.call(100, 0, phys, &work_in, &[]); // v_opnvwk
        pb.contrl[6]
    })
}

fn device() -> &'static XrgbDevice {
    &HOST_CELL.get().expect("workstation() not opened").raster
}

/// `v_get_pixel(x, y)` → `(INTOUT[0], INTOUT[1])`.
fn v_get_pixel(pb: &mut Pb, h: i16, x: i16, y: i16) -> (i16, u16) {
    pb.call(105, 0, h, &[], &[x, y]);
    (pb.intout[0], pb.intout[1] as u16)
}

// ── the tests ────────────────────────────────────────────────────────────

/// Control (green before and after the fix): a pixel *drawn* in pen 0 reads
/// back as the pseudo-palette's white `0xFFDF` (bit 5 clear), and that word
/// survives the kernel's 16→32→16 round trip. The 5-bit green quantisation
/// is EmuTOS-faithful (`set_color16`, `vq_color16`: `vdi_col.c:601-614,802-838`)
/// and must stay; a fix that makes a pen-0 pixel read `0xFFFF` (six green
/// bits) is the wrong fix.
#[test]
fn pen_0_fill_then_v_get_pixel_reports_the_pseudo_palette_white_0xffdf() {
    let _g = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = workstation();
    let dev = device();
    assert_eq!(
        dev.device_to_rgb565(dev.rgb565_to_device(PEN0_WHITE_WORD)),
        PEN0_WHITE_WORD,
        "pen-0 white 0xFFDF must survive the 16->32->16 round trip"
    );
    let mut pb = Pb::new();
    pb.call(3, 0, h, &[], &[]); // v_clrwk
    pb.call(23, 0, h, &[1], &[]); // vsf_interior solid
    pb.call(25, 0, h, &[0], &[]); // vsf_color pen 0
    pb.call(104, 0, h, &[0], &[]); // vsf_perimeter off
    pb.call(114, 0, h, &[], &[100, 100, 199, 199]); // vr_recfl
    let (i0, drawn) = v_get_pixel(&mut pb, h, 150, 150);
    assert_eq!(i0, 0, "INTOUT[0] is 0 under TRUECOLOR_MODE (vdi_fill.c:1105-1110)");
    assert_eq!(
        drawn, PEN0_WHITE_WORD,
        "a pen-0 pixel is set_color16's (31<<11)|(31<<6)|31 (vdi_col.c:601-614)"
    );
}

/// The sharpest form of the defect, at the backend seam: the kernel's two
/// conversions must round-trip the one word the VDI writes without going
/// through a pen — `v_clrwk`'s `0xFFFF` (red until V12).
#[test]
fn the_device_pixel_round_trips_the_cleared_word_0xffff() {
    let _g = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    workstation();
    let dev = device();
    assert_eq!(
        dev.device_to_rgb565(dev.rgb565_to_device(CLEARED_WORD)),
        CLEARED_WORD,
        "the cleared-screen word 0xFFFF (OVERLAY_BIT set, vdi_fill.c:26; \
         memset 0xff in v_clrwk, vdi_control.c:610-618) must survive the \
         16->32->16 round trip: the X byte of the XRGB8888 pixel carries bit 5"
    );
}

/// `vdi-all.tsv` cases 54, 160, 161 on the Falcon truecolor reference:
/// after the reset's `v_clrwk`, `v_get_pixel` answers `0, 0xFFFF` at (5,5),
/// (0,0) and (639,479). NewTOS answered `0, 0xFFDF` before V12 (QEMU
/// raspi4b, 640×480, 2026-09-21).
#[test]
fn v_clrwk_then_v_get_pixel_reports_the_falcon_word_0xffff() {
    let _g = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = workstation();
    let mut pb = Pb::new();
    pb.call(3, 0, h, &[], &[]); // v_clrwk

    for (case, x, y) in [(54, 5, 5), (160, 0, 0), (161, 639, 479)] {
        let (i0, i1) = v_get_pixel(&mut pb, h, x, y);
        assert_eq!(i0, 0, "case {case}: INTOUT[0] is 0 under TRUECOLOR_MODE (vdi_fill.c:1105-1110)");
        assert_eq!(
            i1, CLEARED_WORD,
            "case {case}: v_get_pixel({x},{y}) after v_clrwk -- EmuTOS Falcon truecolor \
             reference (EMUTOS14-VDI-all-falcontc.BIN) = 0x{CLEARED_WORD:04X}, \
             NewTOS = 0x{i1:04X}"
        );
    }
}

/// The two words a Falcon keeps apart: a cleared pixel and a pen-0 pixel on
/// the same screen must differ by `OVERLAY_BIT` and nothing else. (EmuTOS'
/// truecolor seed fill relies on exactly this — it masks the bit out of both
/// sides before comparing, `vdi_fill.c:702-709,726-733,755`.)
#[test]
fn a_cleared_pixel_and_a_pen_0_pixel_differ_by_overlay_bit_alone() {
    let _g = VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let h = workstation();
    let mut pb = Pb::new();
    pb.call(3, 0, h, &[], &[]); // v_clrwk
    pb.call(23, 0, h, &[1], &[]); // vsf_interior solid
    pb.call(25, 0, h, &[0], &[]); // vsf_color pen 0
    pb.call(104, 0, h, &[0], &[]); // vsf_perimeter off
    pb.call(114, 0, h, &[], &[100, 100, 199, 199]); // vr_recfl
    let (_, drawn) = v_get_pixel(&mut pb, h, 150, 150);
    let (_, cleared) = v_get_pixel(&mut pb, h, 5, 5);
    assert_eq!(
        cleared ^ drawn,
        OVERLAY_BIT,
        "cleared 0x{cleared:04X} and pen-0 0x{drawn:04X} must differ by OVERLAY_BIT (0x0020) alone"
    );
}
