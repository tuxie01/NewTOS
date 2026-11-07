//! The NEON screen-to-screen copy kernel of the kernel's VDI screen backend
//! against its scalar references, bit for bit (TASK/V51, ADR-036 point 4).
//!
//! The kernel lives in `kernel/src/raster_copy.rs` on the frame of
//! `kernel/src/raster_kernel.rs`; as in `raster_kernel_fill.rs`, this test
//! compiles both files into itself, so on an AArch64 host — the Mac this
//! project builds on — the very instructions the Pi runs execute here.
//!
//! Two layers, each against the scalar path it replaces, on two copies of the
//! same randomised store, comparing the **whole** store (a pixel written
//! outside the box, left out or combined from the wrong source fails):
//!
//! - `copy_area` against `copy_area_reference` (per pixel in the walk's
//!   order): all 16 ops, both column and both row directions, widths 1..72
//!   plus 127..129 and 255..257, heights, start columns 0..17, strides.
//! - `copy_rect` against `copy_rect_reference` (the backend's per-pixel loop
//!   before V51, its directions from the start points and its skip of
//!   off-screen sources included), behind the backend's own clipping
//!   (`ClipRect::clamp_to`): destination shifted against the source by
//!   -17..+17 pixels and -3..+3 rows — same row and same column included —,
//!   all 16 ops, widths and heights, strides, sources and destinations partly
//!   off the screen, clip rectangles cut on every side.
//!
//! Every store sits between two pages the test made inaccessible (`mmap` +
//! `mprotect`), once with its first pixel right after the lower one and once
//! with the last pixel of its last screen row right before the upper one, so a
//! kernel that reads or writes one pixel beyond the screen rectangle at either
//! end crashes the test instead of passing it.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[path = "../../kernel/src/raster_copy.rs"]
mod raster_copy;

use core::ffi::c_void;

use raster_copy::{copy_area, copy_area_reference, copy_rect, copy_rect_reference, raster_op};
use raster_kernel::{Area, Surface};
use tos_api::vdi::backend::ClipRect;

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

/// Read-write pixels between two inaccessible pages.
struct Guard {
    map: *mut u8,
    room: usize,
    page: usize,
}
impl Guard {
    fn new(room_bytes: usize) -> Guard {
        // SAFETY: plain libc calls; the mapping is private to this test.
        unsafe {
            let page = sysconf(SC_PAGESIZE) as usize;
            let room = room_bytes.div_ceil(page) * page;
            let p = mmap(core::ptr::null_mut(), room + 2 * page, PROT_RW, MAP_PRIVATE_ANON, -1, 0);
            assert!(p as isize != -1, "mmap failed");
            let p = p.cast::<u8>();
            assert_eq!(mprotect(p.cast(), page, PROT_NONE), 0, "mprotect failed");
            assert_eq!(mprotect(p.add(page + room).cast(), page, PROT_NONE), 0, "mprotect failed");
            Guard { map: p, room, page }
        }
    }
    /// `len` pixels right after the lower guard page (`tail` false) or right
    /// before the upper one (`tail` true), filled from `init`.
    fn place(&mut self, init: &[u32], tail: bool) -> &mut [u32] {
        let bytes = init.len() * 4;
        assert!(bytes <= self.room);
        let off = if tail { self.room - bytes } else { 0 };
        // SAFETY: `off .. off + bytes` lies in the read-write part of the
        // mapping, which lives as long as `self`; the borrow of `self` keeps
        // it from being handed out twice; `off` is a multiple of 4.
        let s = unsafe { core::slice::from_raw_parts_mut(self.map.add(self.page + off).cast::<u32>(), init.len()) };
        s.copy_from_slice(init);
        s
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        // SAFETY: the mapping made in `new`.
        unsafe { munmap(self.map.cast(), self.room + 2 * self.page) };
    }
}

/// Kernel (in the guard, at both ends) and reference (on a plain copy) on one
/// random store of `len` pixels at `stride`; panics with the first
/// differing pixel.
fn compare(
    rng: &mut Rng,
    guard: &mut Guard,
    len: usize,
    stride: usize,
    what: &dyn Fn() -> String,
    kernel: &mut dyn FnMut(Surface),
    reference: &mut dyn FnMut(Surface),
) {
    let init: Vec<u32> = (0..len).map(|_| rng.u32()).collect();
    let mut want = init.clone();
    reference(Surface { base: want.as_mut_ptr(), stride_px: stride });
    for tail in [false, true] {
        let got = guard.place(&init, tail);
        kernel(Surface { base: got.as_mut_ptr(), stride_px: stride });
        if *got != *want {
            let i = got.iter().zip(&want).position(|(a, b)| a != b).unwrap();
            panic!(
                "kernel != reference: {} (store at the {} guard): first difference at store index {i} \
                 (row {}, col {}): kernel {:08x}, reference {:08x}, before {:08x}",
                what(),
                if tail { "upper" } else { "lower" },
                i / stride,
                i % stride,
                got[i],
                want[i],
                init[i]
            );
        }
    }
}

/// The pixels of a `scr_w` x `scr_h` screen at `stride`: the last row ends
/// at its last visible pixel, so the upper guard page follows it.
fn screen_len(scr_w: usize, scr_h: usize, stride: usize) -> usize {
    (scr_h - 1) * stride + scr_w
}

/// One `copy_area` case: the `w` x `h` source at `(sx, sy)`, the destination
/// at `(x, y)`, on a screen just big enough for both.
#[allow(clippy::too_many_arguments)]
fn area_case(
    rng: &mut Rng,
    guard: &mut Guard,
    sx: usize,
    sy: usize,
    area: Area,
    op: u8,
    backward: bool,
    bottom_up: bool,
    pad: usize,
) {
    let scr_w = (sx + area.w).max(area.x + area.w);
    let scr_h = (sy + area.h).max(area.y + area.h);
    let stride = scr_w + pad;
    let what = || format!("op {op} src ({sx},{sy}) area {area:?} backward {backward} bottom_up {bottom_up} stride {stride}");
    compare(
        rng,
        guard,
        screen_len(scr_w, scr_h, stride),
        stride,
        &what,
        // SAFETY: both rectangles lie on the screen; the cases pick the
        // directions of the per-pixel loop (`copy_area` contract).
        &mut |s| unsafe { copy_area(s, sx, sy, area, op, backward, bottom_up) },
        &mut |s| unsafe { copy_area_reference(s, sx, sy, area, op, backward, bottom_up) },
    );
}

fn widths() -> Vec<usize> {
    let mut w: Vec<usize> = (1..=72).collect();
    w.extend([127, 128, 129, 255, 256, 257]);
    w
}

#[test]
fn copy_area_matches_the_reference_for_every_op_width_and_start_column() {
    let mut rng = Rng(0x5635_3100_0000_0001);
    let mut guard = Guard::new(1 << 16);
    for op in 0..16u8 {
        for &w in &widths() {
            for x in [0usize, 1, 3, 5, 8, 13, 16, 17] {
                // Disjoint rows (destination below, then above the source),
                // both column directions; the directions do not matter there.
                let area = Area { x, y: 2, w, h: 2 };
                area_case(&mut rng, &mut guard, (x * 7 + w) % 11, 0, area, op, x & 1 == 1, false, x % 3);
                let area = Area { x, y: 0, w, h: 1 };
                area_case(&mut rng, &mut guard, (x * 5 + w) % 13, 1, area, op, x & 2 == 2, false, 0);
            }
        }
    }
}

#[test]
fn copy_area_matches_the_reference_at_every_height_and_stride() {
    let mut rng = Rng(0x5635_3100_0000_0002);
    let mut guard = Guard::new(1 << 20);
    for op in [0u8, 3, 6, 7, 10, 12, 15] {
        for h in 1..=12 {
            for (x, w) in [(0usize, 1usize), (3, 5), (7, 8), (1, 16), (9, 17), (0, 33), (5, 64), (13, 65)] {
                for pad in [0usize, 1, 3, 1920 - (x + w)] {
                    for bottom_up in [false, true] {
                        // The destination `h` rows below the source: no
                        // shared row, either row order gives the same pixels.
                        let area = Area { x, y: h, w, h };
                        area_case(&mut rng, &mut guard, (x + 3) % 7, 0, area, op, false, bottom_up, pad);
                        area_case(&mut rng, &mut guard, (x + 3) % 7, 0, area, op, true, bottom_up, pad);
                    }
                }
            }
        }
    }
}

/// The backend's destination box: `copy_rect`'s clipping of the `w` x `h`
/// rectangle at `(dx, dy)` to `clip` and the screen (`ClipRect::clamp_to`).
fn backend_box(w: i32, h: i32, dx: i32, dy: i32, clip: ClipRect, scr_w: i32, scr_h: i32) -> Option<(i32, i32, i32, i32)> {
    let (dx, dy) = (dx as i16, dy as i16);
    let dbox = ClipRect {
        xmn: dx.max(clip.xmn),
        ymn: dy.max(clip.ymn),
        xmx: ((dx as i32 + w - 1) as i16).min(clip.xmx),
        ymx: ((dy as i32 + h - 1) as i16).min(clip.ymx),
    };
    dbox.clamp_to(scr_w as u16, scr_h as u16)
}

/// One `copy_rect` case through the backend's clipping; op 5 and an empty
/// box return before the copy, as in the backend.
#[allow(clippy::too_many_arguments)]
fn rect_case(
    rng: &mut Rng,
    guard: &mut Guard,
    scr_w: i32,
    scr_h: i32,
    pad: usize,
    src: (i32, i32, i32, i32),
    dx: i32,
    dy: i32,
    clip: ClipRect,
    op: u8,
) -> bool {
    if op & 0x0f == 5 {
        return false;
    }
    let (sx, sy, w, h) = src;
    let Some(bx) = backend_box(w, h, dx, dy, clip, scr_w, scr_h) else { return false };
    let stride = scr_w as usize + pad;
    let what = || {
        format!(
            "op {op} screen {scr_w}x{scr_h} stride {stride} src ({sx},{sy}) {w}x{h} -> ({dx},{dy}) clip {clip:?} box {bx:?}"
        )
    };
    compare(
        rng,
        guard,
        screen_len(scr_w as usize, scr_h as usize, stride),
        stride,
        &what,
        // SAFETY: `clamp_to` put the box on the screen; the store holds it.
        &mut |s| unsafe { copy_rect(s, scr_w, scr_h, sx, sy, dx, dy, bx, op) },
        &mut |s| unsafe { copy_rect_reference(s, scr_w, scr_h, sx, sy, dx, dy, bx, op) },
    );
    true
}

#[test]
fn copy_rect_matches_the_backend_loop_at_every_overlap() {
    let mut rng = Rng(0x5635_3100_0000_0003);
    let mut guard = Guard::new(1 << 18);
    let mut ws: Vec<i32> = (1..=20).collect();
    ws.extend([31, 32, 33, 47, 63, 64, 65, 66, 127, 128, 129, 255, 256, 257]);
    let mut n = 0usize;
    for dyo in -3..=3i32 {
        for dxo in -17..=17i32 {
            for (i, &w) in ws.iter().enumerate() {
                let h = [1, 2, 3, 5, 8][i % 5];
                // A roomy screen (the whole move on it) and a tight one (the
                // source fills it, the shifted copy runs off its edges).
                let (sx, sy) = (18, 4);
                let roomy = (w + 36, h + 8);
                for op in 0..16u8 {
                    let full = ClipRect::full(roomy.0 as u16, roomy.1 as u16);
                    n += rect_case(&mut rng, &mut guard, roomy.0, roomy.1, (i % 3) * 2, (sx, sy, w, h), sx + dxo, sy + dyo, full, op) as usize;
                    let full = ClipRect::full(w as u16, h as u16);
                    n += rect_case(&mut rng, &mut guard, w, h, i % 2, (0, 0, w, h), dxo, dyo, full, op) as usize;
                }
            }
        }
    }
    assert!(n > 150_000, "cases run: {n}");
}

#[test]
fn copy_rect_matches_the_backend_loop_for_every_op_on_overlap() {
    let mut rng = Rng(0x5635_3100_0000_0004);
    let mut guard = Guard::new(1 << 18);
    for op in 0..16u8 {
        for dyo in [-3i32, -1, 0, 1, 3] {
            for dxo in [-17i32, -16, -9, -8, -5, -1, 0, 1, 2, 4, 7, 15, 16, 17] {
                for w in [1i32, 2, 3, 4, 7, 9, 15, 16, 17, 31, 40, 64, 65, 130] {
                    let (sx, sy, h) = (18, 4, 1 + (w % 4));
                    let (scr_w, scr_h) = (w + 36, h + 8);
                    let full = ClipRect::full(scr_w as u16, scr_h as u16);
                    rect_case(&mut rng, &mut guard, scr_w, scr_h, 1, (sx, sy, w, h), sx + dxo, sy + dyo, full, op);
                }
            }
        }
    }
}

#[test]
fn copy_rect_matches_the_backend_loop_off_the_screen_and_clipped() {
    let mut rng = Rng(0x5635_3100_0000_0005);
    let mut guard = Guard::new(1 << 18);
    let (scr_w, scr_h) = (61, 23);
    let mut n = 0usize;
    for op in [3u8, 6, 10, 12, 0, 15, 7, 1] {
        for (sx, sy) in [(-20i32, -5i32), (-3, 0), (0, -2), (10, 4), (50, 20), (58, 21), (-40, 30), (70, 2)] {
            for (dx, dy) in [(-7i32, -1i32), (0, 0), (5, 3), (40, 18), (55, 20), (-30, 10), (12, -9), (63, 5)] {
                for (w, h) in [(1i32, 1i32), (9, 4), (17, 7), (33, 2), (64, 24), (130, 30), (0, 5), (5, -1)] {
                    for clip in [
                        ClipRect::full(scr_w as u16, scr_h as u16),
                        ClipRect { xmn: 3, ymn: 2, xmx: 40, ymx: 15 },
                        ClipRect { xmn: -10, ymn: -10, xmx: 200, ymx: 200 },
                        ClipRect { xmn: 20, ymn: 20, xmx: 19, ymx: 30 },
                    ] {
                        n += rect_case(&mut rng, &mut guard, scr_w, scr_h, 3, (sx, sy, w, h), dx, dy, clip, op) as usize;
                    }
                }
            }
        }
    }
    assert!(n > 1000, "cases run: {n}");
}

#[test]
fn copy_rect_at_screen_size_with_a_wide_stride() {
    // 1920 x 4 rows of a 1920 x 1080 screen with a stride wider than the
    // screen, a whole-row scroll up and down by one row and a pan by 16/8/3.
    let mut rng = Rng(0x5635_3100_0000_0006);
    let mut guard = Guard::new(1 << 16);
    let (scr_w, scr_h) = (1920, 6);
    let full = ClipRect::full(scr_w as u16, scr_h as u16);
    for op in [3u8, 6] {
        for (dx, dy) in [(0, -1), (0, 1), (-16, 0), (16, 0), (-8, 0), (8, 0), (-3, 0), (3, 0), (5, -2)] {
            for pad in [0usize, 64] {
                assert!(rect_case(&mut rng, &mut guard, scr_w, scr_h, pad, (0, 1, scr_w, 4), dx, 1 + dy, full, op));
            }
        }
    }
}

#[test]
fn copy_area_op_5_and_an_empty_area_write_nothing() {
    let mut rng = Rng(0x5635_3100_0000_0007);
    let init: Vec<u32> = (0..64 * 8).map(|_| rng.u32()).collect();
    for (op, area) in [
        (5u8, Area { x: 3, y: 1, w: 20, h: 4 }),
        (0x15, Area { x: 3, y: 1, w: 20, h: 4 }),
        (3, Area { x: 3, y: 1, w: 0, h: 4 }),
        (6, Area { x: 3, y: 1, w: 9, h: 0 }),
    ] {
        let mut got = init.clone();
        // SAFETY: the rectangles lie in the 64x8 store (or are empty).
        unsafe { copy_area(Surface { base: got.as_mut_ptr(), stride_px: 64 }, 30, 2, area, op, false, false) };
        assert_eq!(got, init, "op {op} {area:?}");
    }
}

#[test]
fn copy_area_takes_the_low_nibble_of_the_op() {
    let mut rng = Rng(0x5635_3100_0000_0008);
    let mut guard = Guard::new(1 << 14);
    for op in 0..16u8 {
        area_case(&mut rng, &mut guard, 1, 0, Area { x: 3, y: 1, w: 37, h: 3 }, op | 0x10, false, false, 2);
        area_case(&mut rng, &mut guard, 1, 0, Area { x: 3, y: 1, w: 37, h: 3 }, op | 0xF0, true, true, 2);
    }
    assert_eq!(raster_op(0x16, 0x1234_5678, 0x0F0F_0F0F), raster_op(6, 0x1234_5678, 0x0F0F_0F0F));
}

#[test]
fn copy_area_xor_twice_restores_the_store() {
    let mut rng = Rng(0x5635_3100_0000_0009);
    let init: Vec<u32> = (0..100 * 9).map(|_| rng.u32()).collect();
    let mut got = init.clone();
    let s = Surface { base: got.as_mut_ptr(), stride_px: 100 };
    let area = Area { x: 7, y: 4, w: 81, h: 4 };
    // SAFETY: source rows 0..4 and destination rows 4..8 of the 100x9 store.
    unsafe {
        copy_area(s, 11, 0, area, 6, false, false);
        copy_area(s, 11, 0, area, 6, true, true);
    }
    assert_eq!(got, init);
}

/// Each kernel saves before its first V register use what it restores at its
/// end: every V/Q register the file's assembler names is one of `v0..v13`,
/// and each kernel's save list (`stp` to `{save}`) names exactly the
/// registers of its restore list (`ldp` from `{save}`): the copy kernel
/// v0..v3, the op kernel v0..v13.
#[test]
fn every_v_register_used_is_saved_and_restored() {
    let src = include_str!("../../kernel/src/raster_copy.rs");
    let code: Vec<&str> = src.lines().map(str::trim).filter(|l| !l.starts_with("//")).collect();
    let mut all: Vec<u32> = code.iter().filter(|l| l.contains('"')).flat_map(|l| regs_in(l)).collect();
    all.sort();
    all.dedup();
    assert!(all.iter().all(|&r| r <= 13), "V registers named: {all:?}");
    let saves: Vec<Vec<u32>> = blocks(&code, &["\"stp q", "\"str q"]).iter().map(|b| flat(b)).collect();
    let loads: Vec<Vec<u32>> = blocks(&code, &["\"ldp q", "\"ldr q"]).iter().map(|b| flat(b)).collect();
    assert_eq!(saves.len(), 2, "two kernels");
    assert_eq!(saves, loads, "save list == restore list per kernel");
    let sorted = |v: &Vec<u32>| {
        let mut v = v.clone();
        v.sort();
        v
    };
    // `copy_kernel!` is defined before `rop_kernel!`.
    assert_eq!(sorted(&saves[0]), (0..=3).collect::<Vec<u32>>());
    assert_eq!(sorted(&saves[1]), (0..=13).collect::<Vec<u32>>());
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
