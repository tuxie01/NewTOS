//! The mouse pointer's cell in the kernel's VDI screen backend against the
//! per-pixel path it replaces, bit for bit (TASK/V59, ADR-036 point 4).
//!
//! The pieces live in `kernel/src/raster_copy.rs` (`copy_rows`, the NEON
//! row copy between the screen and a buffer, with `read_rect` /
//! `write_rect`) and `kernel/src/raster_expand.rs` (`sprite`: the form as two
//! `Transparent` expansions of the V50 kernel); as in the other
//! `raster_kernel_*.rs` tests this file compiles them into itself, so on an
//! AArch64 host the very instructions the Pi runs execute here.
//!
//! Against the scalar references, which are the backend's per-pixel
//! definitions before V59:
//!
//! - `read_rect` against `read_rect_reference` (`get_device_pixel` per pixel,
//!   `0` off the screen) and `write_rect` against `write_rect_reference`
//!   (`put_device_pixel` per pixel, dropped off the screen), for the 16x16
//!   cell at every position from 17 pixels off the left/top edge to past the
//!   right/bottom edge, and for other rectangle sizes 1..20; strides
//!   minimal, +1, +3 and 1920-wide rows.
//! - `sprite` behind the backend's clipping (`ClipRect::clamp_to` of the
//!   cell) against `sprite_reference` (per cell pixel the old 1x1 fill:
//!   `fg` on a data bit, else `bg` on a mask bit), over the eight preset
//!   forms, full/empty/random forms, random colours, every position.
//! - the round trip of the pointer (save, paint, restore) gives the store
//!   back unchanged at every position.
//!
//! Every store sits between two inaccessible pages (`mmap` + `mprotect`),
//! once right after the lower one and once with its last screen pixel right
//! before the upper one; the buffer of `read_rect` ends right before a guard
//! page too. A kernel that reads or writes one pixel beyond the screen
//! rectangle or the buffer crashes the test. The whole store is compared.
//!
//! The V-register save/restore check of `copy_rows` is the source check in
//! `raster_kernel_copy.rs` (three kernels in that file); `sprite` adds no
//! assembler of its own.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_copy.rs"]
mod raster_copy;
#[allow(dead_code)]
#[path = "../../kernel/src/raster_expand.rs"]
mod raster_expand;

use core::ffi::c_void;

use raster_copy::{read_rect, read_rect_reference, write_rect, write_rect_reference};
use raster_expand::{sprite, sprite_reference};
use raster_kernel::Surface;
use tos_api::vdi::backend::ClipRect;
use tos_api::vdi::mouse::PRESET_FORMS;

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
    fn u16(&mut self) -> u16 {
        (self.next() >> 48) as u16
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

/// The pixels of a `scr_w` x `scr_h` screen at `stride`: the last row ends
/// at its last visible pixel, so the upper guard page follows it.
fn screen_len(scr_w: usize, scr_h: usize, stride: usize) -> usize {
    (scr_h - 1) * stride + scr_w
}

/// A screen geometry of the cases: width, height, stride.
#[derive(Clone, Copy, Debug)]
struct Geo {
    w: i32,
    h: i32,
    stride: usize,
}

fn geos() -> [Geo; 5] {
    [
        Geo { w: 40, h: 30, stride: 40 },
        Geo { w: 37, h: 23, stride: 38 },
        Geo { w: 33, h: 19, stride: 36 },
        Geo { w: 16, h: 16, stride: 16 },
        Geo { w: 1920, h: 20, stride: 1920 },
    ]
}

/// Cell positions along one axis of a `n`-pixel screen: from 17 off the
/// low edge to 1 past the high one, every one near the edges, a few inside.
fn positions(n: i32) -> Vec<i32> {
    let mut v: Vec<i32> = (-17..=3).collect();
    v.extend([n / 2 - 1, n / 2]);
    v.extend(n - 18..=n + 1);
    v.sort();
    v.dedup();
    v
}

/// The backend's box of a sprite: the cell at `(x, y)` clamped to the screen
/// (`VdiScreen::draw_sprite`).
fn sprite_box(x: i32, y: i32, g: Geo) -> Option<(i32, i32, i32, i32)> {
    let cell = ClipRect {
        xmn: x as i16,
        ymn: y as i16,
        xmx: (x + 15).min(i16::MAX as i32) as i16,
        ymx: (y + 15).min(i16::MAX as i32) as i16,
    };
    cell.clamp_to(g.w as u16, g.h as u16)
}

/// One `read_rect` case: the `w` x `h` rectangle at `(x, y)`; store at both
/// guard ends, the buffer ending right before a guard page and filled with
/// noise first (so the zero fill off the screen is checked).
#[allow(clippy::too_many_arguments)]
fn read_case(rng: &mut Rng, sg: &mut Guard, bg: &mut Guard, g: Geo, x: i32, y: i32, w: usize, h: usize) {
    let init: Vec<u32> = (0..screen_len(g.w as usize, g.h as usize, g.stride)).map(|_| rng.u32()).collect();
    let noise: Vec<u32> = (0..w * h).map(|_| rng.u32()).collect();
    let mut want_store = init.clone();
    let mut want = noise.clone();
    // SAFETY: the reference reads only on-screen pixels of the store.
    unsafe {
        read_rect_reference(
            Surface { base: want_store.as_mut_ptr(), stride_px: g.stride },
            g.w,
            g.h,
            x,
            y,
            w,
            h,
            &mut want,
        )
    };
    for tail in [false, true] {
        let store = sg.place(&init, tail);
        let out = bg.place(&noise, true);
        // SAFETY: the store holds the screen; `out` is a separate mapping.
        unsafe { read_rect(Surface { base: store.as_mut_ptr(), stride_px: g.stride }, g.w, g.h, x, y, w, h, out) };
        assert_eq!(*out, *want, "read_rect {g:?} at ({x},{y}) {w}x{h} tail {tail}");
        assert_eq!(*store, *want_store, "read_rect changed the store: {g:?} at ({x},{y}) {w}x{h}");
    }
}

/// One `write_rect` case, the source ending right before a guard page.
#[allow(clippy::too_many_arguments)]
fn write_case(rng: &mut Rng, sg: &mut Guard, bg: &mut Guard, g: Geo, x: i32, y: i32, w: usize, h: usize) {
    let init: Vec<u32> = (0..screen_len(g.w as usize, g.h as usize, g.stride)).map(|_| rng.u32()).collect();
    let pix: Vec<u32> = (0..w * h).map(|_| rng.u32()).collect();
    let mut want = init.clone();
    // SAFETY: the reference writes only on-screen pixels of the store.
    unsafe {
        write_rect_reference(Surface { base: want.as_mut_ptr(), stride_px: g.stride }, g.w, g.h, x, y, w, h, &pix)
    };
    for tail in [false, true] {
        let store = sg.place(&init, tail);
        let src = bg.place(&pix, true);
        // SAFETY: the store holds the screen; `src` is a separate mapping.
        unsafe { write_rect(Surface { base: store.as_mut_ptr(), stride_px: g.stride }, g.w, g.h, x, y, w, h, src) };
        if *store != *want {
            let i = store.iter().zip(&want).position(|(a, b)| a != b).unwrap();
            panic!(
                "write_rect {g:?} at ({x},{y}) {w}x{h} tail {tail}: first difference at row {}, col {}: {:08x} != {:08x}",
                i / g.stride,
                i % g.stride,
                store[i],
                want[i]
            );
        }
    }
}

#[test]
fn read_rect_matches_get_device_pixel_for_the_cell_at_every_edge_position() {
    let mut rng = Rng(0x5635_3900_0000_0001);
    let mut sg = Guard::new(1 << 18);
    let mut bg = Guard::new(1 << 16);
    let mut n = 0usize;
    for g in geos() {
        for y in positions(g.h) {
            for x in positions(g.w) {
                read_case(&mut rng, &mut sg, &mut bg, g, x, y, 16, 16);
                n += 1;
            }
        }
    }
    assert!(n > 2000, "cases run: {n}");
}

#[test]
fn write_rect_matches_put_device_pixel_for_the_cell_at_every_edge_position() {
    let mut rng = Rng(0x5635_3900_0000_0002);
    let mut sg = Guard::new(1 << 18);
    let mut bg = Guard::new(1 << 16);
    let mut n = 0usize;
    for g in geos() {
        for y in positions(g.h) {
            for x in positions(g.w) {
                write_case(&mut rng, &mut sg, &mut bg, g, x, y, 16, 16);
                n += 1;
            }
        }
    }
    assert!(n > 2000, "cases run: {n}");
}

#[test]
fn read_and_write_rect_match_for_other_sizes() {
    // Every width 1..20 through the 16/8/4/2/1 blocks of `copy_rows`,
    // heights 1..5, rectangles on, across and off every edge; empty ones.
    let mut rng = Rng(0x5635_3900_0000_0003);
    let mut sg = Guard::new(1 << 18);
    let mut bg = Guard::new(1 << 16);
    let g = Geo { w: 29, h: 11, stride: 31 };
    for w in 0..=20usize {
        for h in 0..=5usize {
            for (x, y) in [(0, 0), (3, 2), (-5, 1), (20, 9), (25, -3), (-21, 4), (29, 0), (0, 11), (-1, -1), (9, 6)] {
                read_case(&mut rng, &mut sg, &mut bg, g, x, y, w, h);
                write_case(&mut rng, &mut sg, &mut bg, g, x, y, w, h);
            }
        }
    }
    // Rows of a 1920-pixel screen: 64 and 1920 pixels wide.
    let g = Geo { w: 1920, h: 4, stride: 1920 };
    for (x, w) in [(0, 64usize), (1856, 64), (1900, 64), (0, 1920), (-7, 1920)] {
        read_case(&mut rng, &mut sg, &mut bg, g, x, 1, w, 3);
        write_case(&mut rng, &mut sg, &mut bg, g, x, 1, w, 3);
    }
}

/// The forms of the sprite cases: the eight presets, empty, full, the
/// presets' data alone, random.
fn forms(rng: &mut Rng) -> Vec<([u16; 16], [u16; 16])> {
    let mut v: Vec<([u16; 16], [u16; 16])> = PRESET_FORMS.iter().map(|f| (f.2, f.3)).collect();
    v.push(([0; 16], [0; 16]));
    v.push(([0xFFFF; 16], [0xFFFF; 16]));
    v.push(([0xFFFF; 16], [0; 16]));
    v.push(([0; 16], PRESET_FORMS[2].3));
    for _ in 0..4 {
        v.push((core::array::from_fn(|_| rng.u16()), core::array::from_fn(|_| rng.u16())));
    }
    v
}

#[test]
fn sprite_matches_the_per_pixel_paint_at_every_position_form_and_colour() {
    let mut rng = Rng(0x5635_3900_0000_0004);
    let mut sg = Guard::new(1 << 18);
    let forms = forms(&mut rng);
    let mut n = 0usize;
    for g in geos() {
        for y in positions(g.h) {
            for x in positions(g.w) {
                for (i, (mask, data)) in forms.iter().enumerate() {
                    // Skip most of the inner positions of the wide screen
                    // for the random forms (the edges are what matters).
                    if g.w == 1920 && i > 8 && (3..1900).contains(&x) {
                        continue;
                    }
                    let (fg, bgc) = if i % 3 == 0 { (0x0000_0000, 0x00FF_FFFF) } else { (rng.u32(), rng.u32()) };
                    let init: Vec<u32> =
                        (0..screen_len(g.w as usize, g.h as usize, g.stride)).map(|_| rng.u32()).collect();
                    let mut want = init.clone();
                    // SAFETY: the reference writes only on-screen pixels.
                    unsafe {
                        sprite_reference(
                            Surface { base: want.as_mut_ptr(), stride_px: g.stride },
                            g.w,
                            g.h,
                            x,
                            y,
                            mask,
                            data,
                            fg,
                            bgc,
                        )
                    };
                    for tail in [false, true] {
                        let store = sg.place(&init, tail);
                        if let Some(bx) = sprite_box(x, y, g) {
                            // SAFETY: the clamped box lies on the screen and
                            // inside the cell.
                            unsafe {
                                sprite(Surface { base: store.as_mut_ptr(), stride_px: g.stride }, x, y, mask, data, bx, fg, bgc)
                            };
                        }
                        if *store != *want {
                            let k = store.iter().zip(&want).position(|(a, b)| a != b).unwrap();
                            panic!(
                                "sprite form {i} {g:?} at ({x},{y}) tail {tail}: first difference at row {}, col {}: \
                                 {:08x} != {:08x}",
                                k / g.stride,
                                k % g.stride,
                                store[k],
                                want[k]
                            );
                        }
                    }
                    n += 1;
                }
            }
        }
    }
    assert!(n > 20_000, "cases run: {n}");
}

#[test]
fn save_paint_restore_gives_the_store_back_at_every_position() {
    let mut rng = Rng(0x5635_3900_0000_0005);
    let mut sg = Guard::new(1 << 18);
    for g in geos() {
        for y in positions(g.h) {
            for x in positions(g.w) {
                let init: Vec<u32> = (0..screen_len(g.w as usize, g.h as usize, g.stride)).map(|_| rng.u32()).collect();
                let store = sg.place(&init, (x + y) & 1 == 0);
                let s = Surface { base: store.as_mut_ptr(), stride_px: g.stride };
                let mut cell = [0u32; 256];
                let (mask, data) = (PRESET_FORMS[0].2, PRESET_FORMS[0].3);
                // SAFETY: as in the cases above.
                unsafe {
                    read_rect(s, g.w, g.h, x, y, 16, 16, &mut cell);
                    if let Some(bx) = sprite_box(x, y, g) {
                        sprite(s, x, y, &mask, &data, bx, 1, 2);
                    }
                    write_rect(s, g.w, g.h, x, y, 16, 16, &cell);
                }
                assert_eq!(*store, *init, "round trip {g:?} at ({x},{y})");
            }
        }
    }
}

#[test]
#[should_panic(expected = "sprite box outside its cell")]
fn sprite_refuses_a_box_outside_its_cell() {
    let mut store = vec![0u32; 40 * 30];
    let s = Surface { base: store.as_mut_ptr(), stride_px: 40 };
    // SAFETY: the box lies in the 40x30 store; the call panics before
    // drawing.
    unsafe { sprite(s, 5, 5, &[0xFFFF; 16], &[0; 16], (5, 5, 21, 20), 1, 2) };
}
