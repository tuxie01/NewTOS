//! The line path of the kernel's VDI screen backend against its per-pixel
//! path before TASK/V58, bit for bit (ADR-036 point 4).
//!
//! `kernel/src/raster_line.rs` (`abline`: the run form of the shared walk,
//! spans on kernel 1's NEON row path in `kernel/src/raster_kernel.rs`,
//! stride columns, index-stepped Bresenham) is compiled into this test, as
//! the other raster kernel tests do; on this AArch64 host its span kernel is
//! the very assembler the Pi runs. The reference, `abline_reference`, is the
//! backend's `abline` before V58: the shared walk
//! `tos_api::vdi::line::abline`, each pixel through `combine_index`, the
//! touched rectangle from `index % stride`, `index / stride`.
//!
//! Every case runs on one random store, the reference on a plain copy, the
//! kernel twice: once with the store's first pixel right after an
//! inaccessible page and once with its last pixel right before one (`mmap` +
//! `mprotect`), so a read or write one pixel outside the store crashes the
//! test. The **whole** store is compared, and the returned `LN_MASK` and the
//! touched rectangle (what the backend cleans).
//!
//! Sweep: every direction from a centre to endpoints `-70..=70` away (all
//! octants, lengths 0 to 70, horizontal and vertical included, both ways
//! round), all four write modes, dash masks solid, empty, single and mixed
//! bits, XOR with `last` true and false (the `LSTLIN` rule), clipping off
//! and on, plain and padded strides; spans and columns at every start
//! column 0..33 and length 0..70; clipping off with endpoints anywhere in
//! `WORD` range on small stores (run-on across rows, cuts at both ends of
//! the store, wrapped deltas) and random clip rectangles; full-screen lines
//! at 1920x1080; XOR twice.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

#[allow(dead_code)]
#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;
#[path = "../../kernel/src/raster_line.rs"]
mod raster_line;

use core::ffi::c_void;

use raster_line::{abline, abline_reference};
use tos_api::vdi::backend::{ClipRect, WriteMode};
use tos_api::vdi::line::LineTarget;

const MODES: [WriteMode; 4] = [WriteMode::Replace, WriteMode::Transparent, WriteMode::Xor, WriteMode::RevTransparent];

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
    fn i16_in(&mut self, lo: i32, hi: i32) -> i16 {
        (lo + (self.next() >> 33) as i32 % (hi - lo + 1)) as i16
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
            let room = room_bytes.max(1).div_ceil(page) * page;
            let p = mmap(core::ptr::null_mut(), room + 2 * page, PROT_RW, MAP_PRIVATE_ANON, -1, 0);
            assert!(p as isize != -1, "mmap failed");
            let p = p.cast::<u8>();
            assert_eq!(mprotect(p.cast(), page, PROT_NONE), 0, "mprotect failed");
            assert_eq!(mprotect(p.add(page + room).cast(), page, PROT_NONE), 0, "mprotect failed");
            Guard { map: p, room, page }
        }
    }
    /// `init.len()` pixels right after the lower guard page (`tail` false) or
    /// right before the upper one (`tail` true), filled from `init`.
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

/// One store `target` with its random contents and the guard it is drawn in.
struct Bench {
    target: LineTarget,
    init: Vec<u32>,
    guard: Guard,
    fg: u32,
    bg: u32,
}
impl Bench {
    fn new(rng: &mut Rng, target: LineTarget) -> Bench {
        let len = target.height as usize * target.stride as usize;
        let init = (0..len).map(|_| rng.u32()).collect();
        Bench { target, init, guard: Guard::new(len * 4), fg: rng.u32(), bg: rng.u32() }
    }

    /// One segment: the reference on a copy of the store, the kernel at both
    /// guards; the whole store, the mask and the touched rectangle compared.
    fn case(&mut self, seg: (i16, i16, i16, i16), mode: WriteMode, clip: ClipRect, mask: u16, last: bool) {
        let t = self.target;
        let mut want = self.init.clone();
        // SAFETY: `want` is the whole `height * stride` store.
        let (wm, wbox) = unsafe { abline_reference(want.as_mut_ptr(), &t, seg, mode, clip, mask, last, self.fg, self.bg) };
        for tail in [false, true] {
            let got = self.guard.place(&self.init, tail);
            // SAFETY: `got` is the whole `height * stride` store.
            let (gm, gbox) = unsafe { abline(got.as_mut_ptr(), &t, seg, mode, clip, mask, last, self.fg, self.bg) };
            let what = || {
                format!(
                    "{t:?} seg {seg:?} {mode:?} clip {clip:?} mask {mask:04x} last {last} \
                     (store at the {} guard)",
                    if tail { "upper" } else { "lower" }
                )
            };
            if *got != *want {
                let i = got.iter().zip(&want).position(|(a, b)| a != b).unwrap();
                let s = t.stride as usize;
                panic!(
                    "kernel != reference: {}: first difference at store index {i} (row {}, col {}): \
                     kernel {:08x}, reference {:08x}, before {:08x}",
                    what(),
                    i / s,
                    i % s,
                    got[i],
                    want[i],
                    self.init[i]
                );
            }
            assert_eq!(gm, wm, "mask after {}", what());
            assert_eq!(gbox, wbox, "touched rectangle of {}", what());
        }
    }
}

fn clip(xmn: i16, ymn: i16, xmx: i16, ymx: i16) -> ClipRect {
    ClipRect { xmn, ymn, xmx, ymx }
}

/// Every direction and length 0..=70 from the centre of a 146x146 screen —
/// all octants, horizontal and vertical included, both ways round — in all
/// four modes, solid, empty and dashed, XOR with `last` true and false,
/// clipping off and on (the clip cutting most directions), on a plain and
/// a padded stride.
#[test]
fn every_direction_length_mode_and_mask_matches_the_reference() {
    let mut rng = Rng(0x5635_3800_0000_0001);
    for stride in [146u32, 153] {
        let mut b = Bench::new(&mut rng, LineTarget { width: 146, height: 146, stride });
        for d in -70i16..=70 {
            for e in [-70i16, -64, -33, -17, -16, -15, -2, -1, 0, 1, 2, 15, 16, 17, 33, 64, 70] {
                for (dx, dy) in [(d, e), (e, d)] {
                    for seg in [(72, 72, 72 + dx, 72 + dy), (72 + dx, 72 + dy, 72, 72)] {
                        for mode in MODES {
                            for mask in [0xFFFF, 0x0000, 0xF391] {
                                for last in [true, false] {
                                    b.case(seg, mode, ClipRect::OFF, mask, last);
                                    b.case(seg, mode, clip(30, 41, 117, 100), mask, last);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Horizontal spans and vertical columns at every start column 0..=33 (every
/// phase of the dash word and of kernel 1's 4- and 16-pixel blocks, twice)
/// and every length 0..=70 (every tail of kernel 1, several blocks), both
/// ways round, all modes, six masks, `last` both, clipped and not.
#[test]
fn spans_and_columns_at_every_phase_and_length_match_the_reference() {
    let mut rng = Rng(0x5635_3800_0000_0002);
    let mut b = Bench::new(&mut rng, LineTarget { width: 120, height: 90, stride: 123 });
    for mode in MODES {
        for mask in [0xFFFF, 0x0000, 0x8000, 0x0001, 0xAAAA, 0xF391] {
            for x in 0i16..=33 {
                for n in 0i16..=70 {
                    for last in [true, false] {
                        b.case((x, 7, x + n, 7), mode, ClipRect::OFF, mask, last);
                        b.case((x + n, 8, x, 8), mode, ClipRect::OFF, mask, last);
                        b.case((x, 9, x + n, 9), mode, clip(3, 0, 60, 89), mask, last);
                        b.case((x, 10 + n / 2, x, 10), mode, ClipRect::OFF, mask, last);
                        b.case((x + 1, 5, x + 1, 5 + n), mode, clip(0, 12, 119, 50), mask, last);
                    }
                }
            }
        }
    }
}

/// Clipping off with endpoints anywhere in `WORD` range on small stores,
/// padded and not — run-on into the neighbouring rows, cuts at the first
/// and last store pixel (the guard pages sit right there), `WORD`-wrapped
/// deltas — and random clip rectangles.
#[test]
fn clipping_off_run_on_and_wild_endpoints_match_the_reference() {
    let mut rng = Rng(0x5635_3800_0000_0003);
    let targets = [
        LineTarget { width: 7, height: 5, stride: 7 },
        LineTarget { width: 7, height: 5, stride: 9 },
        LineTarget { width: 40, height: 30, stride: 48 },
        LineTarget { width: 1, height: 1, stride: 1 },
        LineTarget { width: 640, height: 4, stride: 648 },
    ];
    for t in targets {
        let mut b = Bench::new(&mut rng, t);
        for round in 0..3000usize {
            let r = [12, 100, 3000, 32767][round % 4];
            let seg = (rng.i16_in(-r, r), rng.i16_in(-r, r), rng.i16_in(-r, r), rng.i16_in(-r, r));
            let c = if round % 3 == 0 {
                let (a, bb, c2, d) = (rng.i16_in(-r, r), rng.i16_in(-r, r), rng.i16_in(-r, r), rng.i16_in(-r, r));
                clip(a.min(c2), bb.min(d), a.max(c2), bb.max(d))
            } else {
                ClipRect::OFF
            };
            let mask = (rng.next() >> 48) as u16;
            b.case(seg, MODES[(round / 4) % 4], c, mask, round % 5 != 0);
        }
        // Runs off each end of the store, every mode, both ways round.
        let (w, h) = (t.width as i16, t.height as i16);
        for mode in MODES {
            for seg in [
                (-20, 0, w + 20, 0),
                (-20, h - 1, w + 20, h - 1),
                (w - 3, h - 1, w + 40, h - 1),
                (0, -30, 0, h + 30),
                (w - 1, -30, w - 1, h + 30),
                (-50, -3, w + 50, h + 3),
                (w + 50, -3, -50, h + 3),
                (3, -40, 5, h + 40),
            ] {
                for last in [true, false] {
                    b.case(seg, mode, ClipRect::OFF, 0xF391, last);
                    b.case((seg.2, seg.3, seg.0, seg.1), mode, ClipRect::OFF, 0xFFFF, last);
                }
            }
        }
    }
}

/// The 1920x1080 screen, plain and padded: full-width and full-height
/// lines, the diagonals, a window frame's edges, the clipped variants.
#[test]
fn full_screen_lines_match_the_reference() {
    let mut rng = Rng(0x5635_3800_0000_0004);
    for stride in [1920u32, 1984] {
        let mut b = Bench::new(&mut rng, LineTarget { width: 1920, height: 1080, stride });
        for mode in MODES {
            for seg in [
                (0, 0, 1919, 0),
                (0, 1079, 1919, 1079),
                (1919, 500, 0, 500),
                (0, 0, 0, 1079),
                (1919, 1079, 1919, 0),
                (0, 0, 1919, 1079),
                (0, 1079, 1919, 0),
                (100, 19, 1819, 1060),
                (3, 7, 1003, 8),
                (5, 3, 6, 1003),
            ] {
                for last in [true, false] {
                    b.case(seg, mode, ClipRect::OFF, 0xFFFF, last);
                    b.case(seg, mode, clip(201, 40, 1700, 1000), 0xF0F0, last);
                }
            }
        }
    }
}

/// XOR twice puts the store back, segment by segment, `last` both ways.
#[test]
fn xor_twice_restores_the_store() {
    let mut rng = Rng(0x5635_3800_0000_0005);
    let t = LineTarget { width: 100, height: 80, stride: 104 };
    let init: Vec<u32> = (0..t.height as usize * t.stride as usize).map(|_| rng.u32()).collect();
    for seg in [(3, 4, 90, 4), (5, 2, 5, 70), (2, 3, 97, 60), (90, 1, 4, 77), (-20, 10, 130, 10), (50, 70, 52, -5)] {
        for last in [true, false] {
            let mut s = init.clone();
            for _ in 0..2 {
                // SAFETY: `s` is the whole store.
                unsafe { abline(s.as_mut_ptr(), &t, seg, WriteMode::Xor, ClipRect::OFF, 0xF391, last, 0, 0) };
            }
            assert!(s == init, "XOR twice {seg:?} last {last}");
        }
    }
}

/// Nothing to draw: a segment wholly outside the clip, a store of no
/// pixels — nothing written, the mask as the walk leaves it.
#[test]
fn empty_cases_match_the_reference() {
    let mut rng = Rng(0x5635_3800_0000_0006);
    let mut b = Bench::new(&mut rng, LineTarget { width: 30, height: 20, stride: 32 });
    for mode in MODES {
        b.case((0, 0, 20, 10), mode, clip(25, 15, 29, 19), 0x1234, true);
        b.case((-5, -5, -1, -9), mode, ClipRect::OFF, 0x1234, false);
        b.case((40, 30, 50, 30), mode, ClipRect::OFF, 0x1234, true);
    }
    for t in [LineTarget { width: 0, height: 0, stride: 0 }, LineTarget { width: 8, height: 0, stride: 8 }] {
        for seg in [(0, 0, 5, 0), (0, 0, 0, 5), (0, 0, 5, 3)] {
            let mut none: [u32; 0] = [];
            // SAFETY: a store of no pixels; nothing may be touched.
            let got = unsafe { abline(none.as_mut_ptr(), &t, seg, WriteMode::Replace, ClipRect::OFF, 0x1234, true, 1, 2) };
            // SAFETY: as above.
            let want =
                unsafe { abline_reference(none.as_mut_ptr(), &t, seg, WriteMode::Replace, ClipRect::OFF, 0x1234, true, 1, 2) };
            assert_eq!(got, want, "{t:?} {seg:?}");
        }
    }
}
