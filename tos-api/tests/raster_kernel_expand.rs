//! The NEON 1-bit-to-32-bit expansion kernel of the kernel's VDI screen
//! backend against its scalar references, bit for bit (TASK/V50, ADR-036
//! point 4).
//!
//! The kernel lives in `kernel/src/raster_expand.rs` on the frame of
//! `kernel/src/raster_kernel.rs`; as in `raster_kernel_fill.rs`, this test
//! compiles both files into itself, so on an AArch64 host — the Mac this
//! project builds on — the very instructions the Pi runs execute here.
//!
//! Three layers, each against the scalar path it replaces, on two copies of
//! the same randomised store, comparing the **whole** store (a pixel written
//! outside the box or left out fails as much as a wrong colour):
//!
//! - `expand` against `expand_reference`: both source units (glyph bytes,
//!   mono words), all four write modes, start bits 0..40, widths 1..72 plus
//!   127..129 and 255..257, heights, source pitches (minimal, wider, `0`),
//!   odd source addresses.
//! - `glyph` against `glyph_reference` (the backend's box loop before V50):
//!   cell widths 1..24 and 31..33, 40, 64, 65, 72 (6, 8 and the wider cells of
//!   text effects), cell heights, cells partly off the store, clip boxes cut
//!   on every side.
//! - `mono` against `mono_reference` (the backend's `blit_mono` loop before
//!   V50, including `MD_ERASE`'s fg/bg swap): source columns 0..33, widths,
//!   source rows, word pitches (minimal, wider, narrower than the row, `0`),
//!   odd raster addresses, clipping.
//!
//! Every source ends at a page the test made inaccessible (`mmap` +
//! `mprotect`), so a kernel that reads one byte past the last unit the box
//! needs crashes the test instead of passing it. The cases the adapters
//! refuse (short glyph, short raster, negative source origin) must return
//! `false` and leave the store alone.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[path = "../../kernel/src/raster_expand.rs"]
mod raster_expand;

use core::ffi::c_void;

use raster_expand::{
    expand, expand_reference, glyph, glyph_reference, mono, mono_colors, mono_reference, Bits, Expand, Unit,
};
use raster_kernel::{Area, Surface};
use tos_api::vdi::backend::{MonoBits, MonoSrc, WriteMode};

const MODES: [WriteMode; 4] =
    [WriteMode::Replace, WriteMode::Transparent, WriteMode::Xor, WriteMode::RevTransparent];

/// xorshift64*, deterministic.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn u32(&mut self) -> u32 {
        (self.next() >> 32) as u32
    }
    fn u8(&mut self) -> u8 {
        (self.next() >> 56) as u8
    }
    /// Source bytes: random, or sparse (mostly clear, the glyph-cell case
    /// the skip of a no-op word serves), or dense (mostly set).
    fn source_byte(&mut self, kind: u64) -> u8 {
        match kind % 3 {
            0 => self.u8(),
            1 => self.u8() & self.u8() & self.u8(),
            _ => self.u8() | self.u8() | self.u8(),
        }
    }
}

extern "C" {
    fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn sysconf(name: i32) -> i64;
}
const PROT_NONE: i32 = 0;
const PROT_RW: i32 = 1 | 2;
#[cfg(target_os = "macos")]
const MAP_PRIVATE_ANON: i32 = 0x0002 | 0x1000;
#[cfg(target_os = "linux")]
const MAP_PRIVATE_ANON: i32 = 0x02 | 0x20;
#[cfg(target_os = "macos")]
const SC_PAGESIZE: i32 = 29;
#[cfg(target_os = "linux")]
const SC_PAGESIZE: i32 = 30;

/// Readable bytes followed by an inaccessible page: [`Guard::tail`] hands
/// out the last `len` readable bytes, so the byte after them faults.
struct Guard {
    base: *mut u8,
    room: usize,
    page: usize,
}
impl Guard {
    fn new(room: usize) -> Guard {
        // SAFETY: plain libc calls; the mapping is private to this test.
        unsafe {
            let page = sysconf(SC_PAGESIZE) as usize;
            let room = room.div_ceil(page) * page;
            let p = mmap(core::ptr::null_mut(), room + page, PROT_RW, MAP_PRIVATE_ANON, -1, 0);
            assert!(p as isize != -1, "mmap failed");
            assert_eq!(mprotect(p.cast::<u8>().add(room).cast(), page, PROT_NONE), 0, "mprotect failed");
            Guard { base: p.cast(), room, page }
        }
    }
    /// The last `len` readable bytes, filled with `fill`.
    fn tail(&mut self, len: usize, mut fill: impl FnMut() -> u8) -> &mut [u8] {
        assert!(len <= self.room);
        // SAFETY: `room - len .. room` lies in the readable part of the
        // mapping, which lives as long as `self`; the borrow of `self` keeps
        // it from being handed out twice.
        let s = unsafe { core::slice::from_raw_parts_mut(self.base.add(self.room - len), len) };
        s.iter_mut().for_each(|b| *b = fill());
        s
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: the mapping made in `new`.
        unsafe { munmap(self.base.cast(), self.room + self.page) };
    }
}

/// Kernel and reference on two copies of one random `stride` x `rows` store;
/// panics with the first differing pixel.
fn compare(
    rng: &mut Rng,
    stride: usize,
    rows: usize,
    what: &dyn Fn() -> String,
    kernel: &mut dyn FnMut(Surface),
    reference: &mut dyn FnMut(Surface),
) {
    let init: Vec<u32> = (0..stride * rows + 7).map(|_| rng.u32()).collect();
    let mut got = init.clone();
    let mut want = init;
    kernel(Surface { base: got.as_mut_ptr(), stride_px: stride });
    reference(Surface { base: want.as_mut_ptr(), stride_px: stride });
    if got != want {
        let i = got.iter().zip(&want).position(|(a, b)| a != b).unwrap();
        panic!(
            "kernel != reference: {}: first difference at store index {i} (row {}, col {}): kernel {:08x}, \
             reference {:08x}",
            what(),
            i / stride,
            i % stride,
            got[i],
            want[i]
        );
    }
}

/// Bytes of one source row that the bits `bit .. bit + w` need.
fn row_bytes(unit: Unit, bit: usize, w: usize) -> usize {
    match unit {
        Unit::Byte => (bit + w - 1) / 8 + 1,
        Unit::Word => ((bit + w - 1) / 16 + 1) * 2,
    }
}

/// One `expand` case: `h` source rows `sstride` bytes apart ending at the
/// guard page (`slack` readable bytes after the last needed one), the area
/// at `(x, y)` of a store `x + w + 3` pixels wide.
#[allow(clippy::too_many_arguments)]
fn expand_case(
    rng: &mut Rng,
    guard: &mut Guard,
    unit: Unit,
    mode: WriteMode,
    bit: usize,
    w: usize,
    h: usize,
    sstride: usize,
    slack: usize,
) {
    let need = (h - 1) * sstride + row_bytes(unit, bit, w);
    let kind = rng.next();
    let src = guard.tail(need + slack, || rng.source_byte(kind));
    let base = src.as_ptr();
    let ex = Expand { mode, fg: rng.u32(), bg: rng.u32() };
    let (x, y) = ((bit * 7 + w) % 19, 1);
    let area = Area { x, y, w, h };
    let bits = Bits { base, stride_bytes: sstride, bit, unit };
    let what = || format!("{unit:?} {mode:?} bit {bit} w {w} h {h} sstride {sstride} slack {slack} area {area:?}");
    compare(
        rng,
        x + w + 3,
        y + h + 1,
        &what,
        // SAFETY: the area lies in the store, the source rows in the guard's
        // readable bytes, not in the store.
        &mut |s| unsafe { expand(s, area, bits, &ex) },
        &mut |s| unsafe { expand_reference(s, area, bits, &ex) },
    );
}

#[test]
fn expand_matches_the_reference_at_every_start_bit_and_width() {
    let mut rng = Rng(0x5635_3000_0000_0001);
    let mut guard = Guard::new(1 << 16);
    let mut widths: Vec<usize> = (1..=72).collect();
    widths.extend([127, 128, 129, 255, 256, 257]);
    for unit in [Unit::Byte, Unit::Word] {
        for mode in MODES {
            for bit in 0..=40 {
                for &w in &widths {
                    let min = row_bytes(unit, bit, w);
                    // Minimal pitch, one row; a wider pitch, two rows; the
                    // odd addresses of `slack` 1 for the words.
                    expand_case(&mut rng, &mut guard, unit, mode, bit, w, 1, min, 0);
                    expand_case(&mut rng, &mut guard, unit, mode, bit, w, 2, min + 2, 1);
                }
            }
        }
    }
}

#[test]
fn expand_matches_the_reference_at_every_height_and_pitch() {
    let mut rng = Rng(0x5635_3000_0000_0002);
    let mut guard = Guard::new(1 << 18);
    for unit in [Unit::Byte, Unit::Word] {
        for mode in MODES {
            for h in 1..=20 {
                for (bit, w) in [(0usize, 1usize), (3, 5), (7, 8), (15, 16), (9, 17), (0, 33), (5, 64), (13, 65)] {
                    let min = row_bytes(unit, bit, w);
                    for sstride in [min, min + 1, min + 2, min + 6, 0] {
                        for slack in [0, 1] {
                            expand_case(&mut rng, &mut guard, unit, mode, bit, w, h, sstride, slack);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn expand_of_an_empty_area_writes_nothing() {
    let mut rng = Rng(0x5635_3000_0000_0003);
    let init: Vec<u32> = (0..64 * 8).map(|_| rng.u32()).collect();
    let src = [0xFFu8; 64];
    for unit in [Unit::Byte, Unit::Word] {
        for mode in MODES {
            for area in [Area { x: 3, y: 1, w: 0, h: 4 }, Area { x: 3, y: 1, w: 9, h: 0 }] {
                let mut got = init.clone();
                let ex = Expand { mode, fg: 0x1234_5678, bg: 0x9ABC_DEF0 };
                let bits = Bits { base: src.as_ptr(), stride_bytes: 4, bit: 3, unit };
                // SAFETY: an empty area touches nothing.
                unsafe { expand(Surface { base: got.as_mut_ptr(), stride_px: 64 }, area, bits, &ex) };
                assert_eq!(got, init, "{unit:?} {mode:?} {area:?}");
            }
        }
    }
}

#[test]
fn expand_xor_twice_restores_the_store() {
    let mut rng = Rng(0x5635_3000_0000_0004);
    let init: Vec<u32> = (0..100 * 9).map(|_| rng.u32()).collect();
    let src: Vec<u8> = (0..6 * 16).map(|_| rng.u8()).collect();
    for unit in [Unit::Byte, Unit::Word] {
        let mut got = init.clone();
        let s = Surface { base: got.as_mut_ptr(), stride_px: 100 };
        let area = Area { x: 7, y: 2, w: 81, h: 6 };
        let bits = Bits { base: src.as_ptr(), stride_bytes: 16, bit: 5, unit };
        let ex = Expand { mode: WriteMode::Xor, fg: 0, bg: 0 };
        // SAFETY: the area lies in the 100x9 store, 6 rows of 16 bytes hold
        // bits 5..86.
        unsafe {
            expand(s, area, bits, &ex);
            expand(s, area, bits, &ex);
        }
        assert_eq!(got, init, "{unit:?}");
    }
}

/// The backend's clipped box: the `w` x `h` rectangle at `(x, y)` cut to the
/// clip rectangle `(cx0, cy0, cx1, cy1)` and the `sw` x `sh` store
/// (`ClipRect::clamp_to`).
#[allow(clippy::too_many_arguments)]
fn clip_box(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    clip: (i32, i32, i32, i32),
    sw: i32,
    sh: i32,
) -> Option<(i32, i32, i32, i32)> {
    let x0 = x.max(clip.0).max(0);
    let y0 = y.max(clip.1).max(0);
    let x1 = (x + w - 1).min(clip.2).min(sw - 1);
    let y1 = (y + h - 1).min(clip.3).min(sh - 1);
    (x0 <= x1 && y0 <= y1).then_some((x0, y0, x1, y1))
}

/// Clip rectangles around a `w` x `h` cell at `(x, y)`: none, and cuts of
/// the left, right, top and bottom edges and all four.
fn clips(x: i32, y: i32, w: i32, h: i32) -> Vec<(i32, i32, i32, i32)> {
    let mut v = vec![(-1000, -1000, 1000, 1000)];
    for k in [1, 3, 7, 8, 9, 15, 17] {
        if k < w {
            v.push((x + k, -1000, 1000, 1000));
            v.push((-1000, -1000, x + w - 1 - k, 1000));
        }
    }
    for k in [1, 5] {
        if k < h {
            v.push((-1000, y + k, 1000, 1000));
            v.push((-1000, -1000, 1000, y + h - 1 - k));
            if 2 * k < w {
                v.push((x + k, y + k, x + w - 1 - k, y + h - 1 - k));
            }
        }
    }
    v
}

#[test]
fn glyph_matches_the_backend_loop() {
    let mut rng = Rng(0x5635_3000_0000_0005);
    let mut guard = Guard::new(1 << 16);
    let mut cells: Vec<usize> = (1..=24).collect();
    cells.extend([31, 32, 33, 40, 64, 65, 72]);
    let (sw, sh) = (110i32, 30i32);
    for &cw in &cells {
        for ch in [1usize, 6, 8, 16, 20] {
            let bpr = cw.div_ceil(8);
            for (x, y) in [(0i32, 0i32), (3, 2), (8, 1), (13, 5), (-5, 0), (-9, -3), (sw - cw as i32 + 4, 4)] {
                for clip in clips(x, y, cw as i32, ch as i32) {
                    let Some(bx) = clip_box(x, y, cw as i32, ch as i32, clip, sw, sh) else { continue };
                    for mode in MODES {
                        let kind = rng.next();
                        let g: &[u8] = guard.tail(ch * bpr, || rng.source_byte(kind));
                        let (fg, bg) = (rng.u32(), rng.u32());
                        let what = || format!("glyph {cw}x{ch} at ({x}, {y}) box {bx:?} {mode:?}");
                        compare(
                            &mut rng,
                            sw as usize + 2,
                            sh as usize,
                            &what,
                            // SAFETY: the box lies in the store (`clip_box`).
                            &mut |s| assert!(unsafe { glyph(s, x, y, g, cw, bx, mode, fg, bg) }, "{}", what()),
                            &mut |s| unsafe { glyph_reference(s, x, y, g, cw, bx, mode, fg, bg) },
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn glyph_refuses_a_short_glyph_and_a_box_outside_the_cell() {
    let mut rng = Rng(0x5635_3000_0000_0006);
    let init: Vec<u32> = (0..40 * 20).map(|_| rng.u32()).collect();
    let g = [0xA5u8; 2 * 16];
    for mode in MODES {
        for (cw, len, bx) in [
            (16usize, 31usize, (2i32, 1i32, 17i32, 16i32)), // one byte short
            (8, 15, (2, 1, 9, 16)),                         // one row short
            (8, 16, (1, 1, 9, 16)),                         // box left of the cell
            (8, 16, (2, 1, 10, 16)),                        // box right of the cell
            (8, 16, (2, 0, 9, 16)),                         // box above the cell
        ] {
            let mut got = init.clone();
            // SAFETY: the boxes lie in the 40x20 store.
            let took = unsafe {
                glyph(Surface { base: got.as_mut_ptr(), stride_px: 40 }, 2, 1, &g[..len], cw, bx, mode, 1, 2)
            };
            assert!(!took, "{mode:?} cell {cw} len {len} box {bx:?}");
            assert_eq!(got, init, "{mode:?} cell {cw} len {len} box {bx:?}");
        }
    }
}

/// One `mono` case: a raster of `rows` rows of `stride_w` words ending at the
/// guard page (`slack` bytes after it end readable), its sub-rectangle at
/// `(sx, sy)` blitted to `(dx, dy)` with size `w` x `h`, clipped by `clip`.
#[allow(clippy::too_many_arguments)]
fn mono_case(
    rng: &mut Rng,
    guard: &mut Guard,
    mode: WriteMode,
    (sx, sy): (i16, i16),
    (dx, dy, w, h): (i32, i32, i32, i32),
    stride_w: usize,
    clip: (i32, i32, i32, i32),
    slack: usize,
) {
    let (sw, sh) = (dx + w + 4, dy + h + 2);
    let Some(bx) = clip_box(dx, dy, w, h, clip, sw, sh) else { return };
    // The words the box reads end at word `last`; the raster holds exactly
    // those (and `slack` bytes more readable after them).
    let sx0 = sx as usize + (bx.0 - dx) as usize;
    let sy0 = sy as usize + (bx.1 - dy) as usize;
    let last = (sy0 + (bx.3 - bx.1) as usize) * stride_w + (sx0 + (bx.2 - bx.0) as usize) / 16;
    let kind = rng.next();
    let raster = guard.tail(2 * (last + 1) + slack, || rng.source_byte(kind));
    // SAFETY: `last + 1` words at the raster's start, any alignment.
    let bits = unsafe { MonoBits::from_raw(raster.as_ptr(), last + 1) };
    let src = MonoSrc { bits, stride_words: stride_w as u16, x: sx, y: sy };
    let (fg, bg) = (rng.u32(), rng.u32());
    let what = || {
        format!("mono {mode:?} src ({sx}, {sy}) stride {stride_w} dst ({dx}, {dy}) {w}x{h} box {bx:?} slack {slack}")
    };
    compare(
        rng,
        sw as usize,
        sh as usize,
        &what,
        // SAFETY: the box lies in the store (`clip_box`).
        &mut |s| assert!(unsafe { mono(s, dx, dy, &src, bx, mode, fg, bg) }, "{}", what()),
        &mut |s| unsafe { mono_reference(s, dx, dy, &src, bx, mode, fg, bg) },
    );
}

#[test]
fn mono_matches_the_backend_loop_at_every_source_column_and_width() {
    let mut rng = Rng(0x5635_3000_0000_0007);
    let mut guard = Guard::new(1 << 16);
    let mut widths: Vec<i32> = (1..=72).collect();
    widths.extend([127, 128, 129, 255, 256, 257]);
    let open = (-1000, -1000, 5000, 5000);
    for mode in MODES {
        for sx in 0..=33i16 {
            for &w in &widths {
                let min = (sx as usize + w as usize).div_ceil(16);
                mono_case(&mut rng, &mut guard, mode, (sx, 0), (5, 1, w, 2), min, open, 0);
                mono_case(&mut rng, &mut guard, mode, (sx, 1), ((sx as i32 * 3) % 17, 2, w, 3), min + 1, open, 1);
            }
        }
    }
}

#[test]
fn mono_matches_the_backend_loop_at_every_pitch_row_and_clip() {
    let mut rng = Rng(0x5635_3000_0000_0008);
    let mut guard = Guard::new(1 << 18);
    for mode in MODES {
        for (sx, w) in [(0i16, 1i32), (3, 5), (15, 16), (9, 17), (0, 32), (7, 64), (13, 65), (1, 100)] {
            let min = (sx as usize + w as usize).div_ceil(16);
            for h in [1i32, 2, 7, 16] {
                for sy in [0i16, 1, 5] {
                    // Minimal, wider, narrower than a row (rows overlap, as
                    // the scalar loop's word arithmetic has it) and zero.
                    for stride_w in [min, min + 1, min + 3, min.saturating_sub(1), 0] {
                        for (dx, dy) in [(0i32, 0i32), (6, 3), (-4, -2)] {
                            for clip in clips(dx, dy, w, h) {
                                for slack in [0, 1] {
                                    mono_case(&mut rng, &mut guard, mode, (sx, sy), (dx, dy, w, h), stride_w, clip, slack);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn mono_refuses_a_short_raster_and_a_negative_origin() {
    let mut rng = Rng(0x5635_3000_0000_0009);
    let init: Vec<u32> = (0..60 * 20).map(|_| rng.u32()).collect();
    let raster: Vec<u8> = (0..2 * 3 * 8).map(|_| rng.u8()).collect();
    for mode in MODES {
        // 3 words per row, box 40x8 at source (3, 0): the last word read is
        // 7 * 3 + 42 / 16 = 23, so 23 words are one short.
        for (words, sx, sy) in [(23usize, 3i16, 0i16), (24, -1, 0), (24, 3, -1)] {
            let mut got = init.clone();
            // SAFETY: `words` <= 24 words of `raster`.
            let bits = unsafe { MonoBits::from_raw(raster.as_ptr(), words) };
            let src = MonoSrc { bits, stride_words: 3, x: sx, y: sy };
            let bx = (2, 1, 41, 8);
            // SAFETY: the box lies in the 60x20 store.
            let took = unsafe { mono(Surface { base: got.as_mut_ptr(), stride_px: 60 }, 2, 1, &src, bx, mode, 1, 2) };
            assert!(!took, "{mode:?} words {words} src ({sx}, {sy})");
            assert_eq!(got, init, "{mode:?} words {words} src ({sx}, {sy})");
        }
    }
}

#[test]
fn md_erase_paints_the_background_colour_on_clear_bits() {
    // `vrt_cpyfm` MD_ERASE: the 0-bits take INTIN[2] (the background), the
    // 1-bits stay (`RasterBackend::blit_mono` docs).
    assert_eq!(mono_colors(WriteMode::RevTransparent, 1, 2), (2, 1));
    for mode in [WriteMode::Replace, WriteMode::Transparent, WriteMode::Xor] {
        assert_eq!(mono_colors(mode, 1, 2), (1, 2));
    }
    let raster = [0x00u8, 0xF0]; // host-order word 0xF000: pixels 0..3 set
    // SAFETY: one word.
    let bits = unsafe { MonoBits::from_raw(raster.as_ptr(), 1) };
    let src = MonoSrc { bits, stride_words: 1, x: 0, y: 0 };
    let mut store = [7u32; 16];
    // SAFETY: a 16x1 box in a 16x1 store.
    let took = unsafe {
        mono(Surface { base: store.as_mut_ptr(), stride_px: 16 }, 0, 0, &src, (0, 0, 15, 0), WriteMode::RevTransparent, 0xF, 0xB)
    };
    assert!(took);
    assert_eq!(store, [7, 7, 7, 7, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB, 0xB]);
}

/// ADR-036 point 3, checked on the source: every V register the kernel
/// file's assembler names is one of `v0..v14`, and each kernel's save list
/// (`stp`/`str` to `{save}`) names exactly the registers of its restore list
/// (`ldp`/`ldr` from `{save}`): `Replace` v0..v10, the read-modify-write
/// kernel v0..v8 and v10..v14.
#[test]
fn every_v_register_used_is_saved_and_restored() {
    let src = include_str!("../../kernel/src/raster_expand.rs");
    let code: Vec<&str> = src.lines().map(str::trim).filter(|l| !l.starts_with("//")).collect();
    let mut all: Vec<u32> = code.iter().filter(|l| l.contains('"')).flat_map(|l| regs_in(l)).collect();
    all.sort();
    all.dedup();
    assert!(all.iter().all(|&r| r <= 14), "V registers named: {all:?}");
    let saves: Vec<Vec<u32>> = blocks(&code, &["\"stp q", "\"str q"]).iter().map(|b| flat(b)).collect();
    let loads: Vec<Vec<u32>> = blocks(&code, &["\"ldp q", "\"ldr q"]).iter().map(|b| flat(b)).collect();
    assert_eq!(saves.len(), 2, "two kernels");
    assert_eq!(saves, loads, "save list == restore list per kernel");
    let sorted = |v: &Vec<u32>| {
        let mut v = v.clone();
        v.sort();
        v
    };
    // `replace_kernel!` is defined before `rmw_kernel!`.
    assert_eq!(sorted(&saves[0]), (0..=10).collect::<Vec<u32>>());
    assert_eq!(sorted(&saves[1]), (0..=8).chain(10..=14).collect::<Vec<u32>>());
}

/// V/Q register numbers named in one line.
fn regs_in(l: &str) -> Vec<u32> {
    let b = l.as_bytes();
    let mut out = Vec::new();
    for i in 0..b.len() {
        let prev_ok = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if prev_ok && (b[i] == b'v' || b[i] == b'q') && i + 1 < b.len() && b[i + 1].is_ascii_digit() {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let next_ok = j == b.len() || matches!(b[j], b'.' | b',' | b' ' | b'"' | b'}');
            if next_ok {
                out.push(l[i + 1..j].parse().unwrap());
            }
        }
    }
    out
}

/// Runs of consecutive lines starting with one of `starts`.
fn blocks<'a>(code: &[&'a str], starts: &[&str]) -> Vec<Vec<&'a str>> {
    let mut out: Vec<Vec<&str>> = Vec::new();
    let mut open = false;
    for l in code {
        if starts.iter().any(|s| l.starts_with(s)) {
            if !open {
                out.push(Vec::new());
                open = true;
            }
            out.last_mut().unwrap().push(l);
        } else {
            open = false;
        }
    }
    out
}

/// The register list before the address operand of each line.
fn flat(lines: &[&str]) -> Vec<u32> {
    lines.iter().flat_map(|l| regs_in(l.split('[').next().unwrap())).collect()
}
