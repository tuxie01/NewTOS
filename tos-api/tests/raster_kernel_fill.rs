//! The NEON pattern-fill kernel of the kernel's VDI screen backend against
//! its scalar reference, bit for bit (TASK/V49, ADR-036 point 4).
//!
//! The kernel lives in `kernel/src/raster_kernel.rs`; the kernel crate cannot
//! be linked for the host, so this test compiles that one file into itself.
//! It has no dependency on the kernel crate (only `tos_api`'s `WriteMode`),
//! and its assembler is plain AArch64 Advanced SIMD, so on an AArch64 host —
//! the Mac this project builds on — the very instructions the Pi runs execute
//! here. On any other host the file does not compile, which is the point:
//! these tests mean nothing off AArch64.
//!
//! The reference, `fill_pattern_reference`, is the backend's per-pixel path
//! before V49 (`fill_pattern` → `combine` → `put_pixel` / `xor_pixel`) on a
//! plain store. Every case runs both on two copies of the same randomised
//! store and compares the **whole** store, so a pixel the kernel writes
//! outside its area, or one it leaves out, fails the case as much as a wrong
//! colour does.
//!
//! Sweep: all four write modes; patterns of length 1..32 (solid, empty,
//! single bits, alternating, random); start columns 0..33 (every phase of the
//! 16-pixel pattern and of the 4-pixel vector, twice); widths 1..72 plus
//! 127..129 and 255..257 (every tail length, one-block and many-block rows);
//! heights 1..20 at every pattern-row phase; strides equal to the row end, a
//! few pixels more, and the 1920-pixel screen.

#![cfg(target_arch = "aarch64")]

#[path = "../../kernel/src/raster_kernel.rs"]
mod raster_kernel;

use raster_kernel::{fill_pattern, fill_pattern_reference, fill_solid, Area, Fill, Surface};
use tos_api::vdi::backend::WriteMode;

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
    fn u16(&mut self) -> u16 {
        (self.next() >> 48) as u16
    }
}

/// The pattern set of the sweep.
fn patterns(rng: &mut Rng) -> Vec<Vec<u16>> {
    let mut p = vec![
        vec![0xFFFF],
        vec![0x0000],
        vec![0x8000],
        vec![0x0001],
        vec![0xAAAA, 0x5555],
        vec![0x8001, 0x7FFE, 0xFFFF, 0x0000],
    ];
    for len in [1usize, 2, 8, 16, 32] {
        p.push((0..len).map(|_| rng.u16()).collect());
    }
    p
}

/// One case: `area` of a store `stride` pixels wide with one spare row and
/// a spare tail, both filled from `rng`; kernel and reference on two copies.
fn case(rng: &mut Rng, mode: WriteMode, pattern: &[u16], area: Area, stride: usize) {
    assert!(area.x + area.w <= stride);
    let len = (area.y + area.h + 1) * stride + 7;
    let init: Vec<u32> = (0..len).map(|_| rng.u32()).collect();
    let (fg, bg) = (rng.u32(), rng.u32());
    let fill = Fill { pattern, mode, fg, bg };
    let mut got = init.clone();
    let mut want = init;
    // SAFETY: the area lies inside both buffers (assert above, `len`).
    unsafe {
        fill_pattern(Surface { base: got.as_mut_ptr(), stride_px: stride }, area, &fill);
        fill_pattern_reference(Surface { base: want.as_mut_ptr(), stride_px: stride }, area, &fill);
    }
    if got != want {
        let i = got.iter().zip(&want).position(|(a, b)| a != b).unwrap();
        panic!(
            "kernel != reference: mode {mode:?} pattern {pattern:04x?} area {area:?} stride {stride} \
             fg {fg:08x} bg {bg:08x}: first difference at store index {i} (row {}, col {}): \
             kernel {:08x}, reference {:08x}",
            i / stride,
            i % stride,
            got[i],
            want[i]
        );
    }
}

#[test]
fn every_phase_and_width_matches_the_reference() {
    let mut rng = Rng(0x5649_3439_0000_0001);
    let pats = patterns(&mut rng);
    let mut widths: Vec<usize> = (1..=72).collect();
    widths.extend([127, 128, 129, 255, 256, 257]);
    for mode in MODES {
        for pat in &pats {
            for x in 0..=33 {
                for &w in &widths {
                    for y in [0usize, 5] {
                        case(&mut rng, mode, pat, Area { x, y, w, h: 3 }, x + w + 3);
                    }
                }
            }
        }
    }
}

#[test]
fn every_height_row_phase_and_stride_matches_the_reference() {
    let mut rng = Rng(0x5649_3439_0000_0002);
    let pats = patterns(&mut rng);
    for mode in MODES {
        for pat in &pats {
            for h in 1..=20 {
                for y in 0..=17 {
                    for (x, w) in [(0usize, 1usize), (3, 5), (15, 16), (1, 17), (0, 33), (7, 64), (9, 65)] {
                        for stride in [x + w, x + w + 1] {
                            case(&mut rng, mode, pat, Area { x, y, w, h }, stride);
                        }
                    }
                }
                // The 1920-pixel screen pitch, two row phases.
                for y in [0usize, 13] {
                    for (x, w) in [(0usize, 1usize), (15, 16), (9, 65)] {
                        case(&mut rng, mode, pat, Area { x, y, w, h }, 1920);
                    }
                }
            }
        }
    }
}

#[test]
fn a_full_screen_row_set_matches_the_reference() {
    // The geometry Fbench's "Fill full window" draws on the Pi's 1920x1080
    // screen, at an odd start column, every mode.
    let mut rng = Rng(0x5649_3439_0000_0003);
    let pats = patterns(&mut rng);
    for mode in MODES {
        for pat in [&pats[0], &pats[4], &pats[10]] {
            case(&mut rng, mode, pat, Area { x: 35, y: 41, w: 850, h: 40 }, 1920);
        }
    }
}

#[test]
fn an_empty_area_or_pattern_writes_nothing() {
    let mut rng = Rng(0x5649_3439_0000_0004);
    let init: Vec<u32> = (0..64 * 8).map(|_| rng.u32()).collect();
    for mode in MODES {
        for (area, pat) in [
            (Area { x: 3, y: 1, w: 0, h: 4 }, &[0xFFFFu16][..]),
            (Area { x: 3, y: 1, w: 9, h: 0 }, &[0xFFFF][..]),
            (Area { x: 3, y: 1, w: 9, h: 4 }, &[][..]),
        ] {
            let mut got = init.clone();
            let fill = Fill { pattern: pat, mode, fg: 0x1234_5678, bg: 0x9ABC_DEF0 };
            // SAFETY: the areas lie inside the 64x8 store.
            unsafe { fill_pattern(Surface { base: got.as_mut_ptr(), stride_px: 64 }, area, &fill) };
            assert_eq!(got, init, "mode {mode:?} area {area:?} pattern {pat:?}");
        }
    }
}

#[test]
fn the_solid_fill_writes_its_colour_on_the_area_only() {
    let mut rng = Rng(0x5649_3439_0000_0005);
    for (x, w) in [(0usize, 1usize), (1, 3), (5, 16), (2, 31), (13, 70), (0, 257)] {
        for h in [1usize, 2, 7] {
            let stride = x + w + 2;
            let init: Vec<u32> = (0..(h + 3) * stride).map(|_| rng.u32()).collect();
            let color = rng.u32();
            let mut got = init.clone();
            let area = Area { x, y: 1, w, h };
            // SAFETY: the area lies inside the store.
            unsafe { fill_solid(Surface { base: got.as_mut_ptr(), stride_px: stride }, area, color) };
            for (i, (&g, &o)) in got.iter().zip(&init).enumerate() {
                let (r, c) = (i / stride, i % stride);
                let inside = (1..1 + h).contains(&r) && (x..x + w).contains(&c);
                assert_eq!(g, if inside { color } else { o }, "area {area:?} stride {stride} index {i}");
            }
        }
    }
}

#[test]
fn xor_twice_restores_the_store() {
    // `WM_XOR` is self-inverse (`VdiScreen::xor_pixel` docs); the kernel too.
    let mut rng = Rng(0x5649_3439_0000_0006);
    let init: Vec<u32> = (0..100 * 9).map(|_| rng.u32()).collect();
    let pat: Vec<u16> = (0..16).map(|_| rng.u16()).collect();
    let fill = Fill { pattern: &pat, mode: WriteMode::Xor, fg: 0, bg: 0 };
    let mut got = init.clone();
    let s = Surface { base: got.as_mut_ptr(), stride_px: 100 };
    let area = Area { x: 7, y: 2, w: 81, h: 6 };
    // SAFETY: the area lies inside the 100x9 store.
    unsafe {
        fill_pattern(s, area, &fill);
        fill_pattern(s, area, &fill);
    }
    assert_eq!(got, init);
}

/// ADR-036 point 3, checked on the source: every V register the kernel file's
/// assembler names is one of `v0..v14`, and each kernel's save list (`stp`/
/// `str` to `{save}`) names exactly the registers of its restore list
/// (`ldp`/`ldr` from `{save}`), covering the registers that kernel uses.
#[test]
fn every_v_register_used_is_saved_and_restored() {
    let src = include_str!("../../kernel/src/raster_kernel.rs");
    // The assembler lines: string literals inside the file's code, not its
    // comments.
    let code: Vec<&str> = src
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .collect();
    let regs_in = |l: &str| -> Vec<u32> {
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
    };
    let mut all: Vec<u32> = code.iter().filter(|l| l.contains('"')).flat_map(|l| regs_in(l)).collect();
    all.sort();
    all.dedup();
    assert!(all.iter().all(|&r| r <= 14), "V registers named: {all:?}");
    // The two kernels' save/restore blocks, in file order: Replace (inline),
    // then the read-modify-write macro.
    let saves: Vec<Vec<u32>> = blocks(&code, &["\"stp q", "\"str q"]).iter().map(|b| flat(b, regs_in)).collect();
    let loads: Vec<Vec<u32>> = blocks(&code, &["\"ldp q", "\"ldr q"]).iter().map(|b| flat(b, regs_in)).collect();
    assert_eq!(saves.len(), 2, "two kernels");
    assert_eq!(saves, loads, "save list == restore list per kernel");
    let replace: Vec<u32> = (0..=10).collect();
    let rmw: Vec<u32> = (0..=8).chain(10..=14).collect();
    let mut s0 = saves[0].clone();
    s0.sort();
    let mut s1 = saves[1].clone();
    s1.sort();
    // The rmw macro is defined before `fill_pattern`, so it comes first.
    assert_eq!((s1, s0), (replace, rmw));
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

fn flat(lines: &[&str], regs_in: impl Fn(&str) -> Vec<u32>) -> Vec<u32> {
    // Only the register list before the address operand.
    lines.iter().flat_map(|l| regs_in(l.split('[').next().unwrap())).collect()
}

/// `fill_span` (a line's horizontal run, TASK/V58) is the one-row fill with
/// the dash word as its pattern, started at its own phase: against the
/// reference on the area of the same row whose start column has that phase,
/// every mode, phase, length (every tail, several blocks) and word.
#[test]
fn a_span_is_the_one_row_fill_at_its_phase() {
    let mut rng = Rng(0x5635_3800_0000_0007);
    let mut lens: Vec<usize> = (0..=72).collect();
    lens.extend([127, 128, 129, 255, 256, 257]);
    let mut words = vec![0xFFFFu16, 0x0000, 0x8000, 0x0001, 0xAAAA, 0xF391];
    words.extend((0..4).map(|_| rng.u16()));
    for mode in MODES {
        for &word in &words {
            for phase in 0u32..16 {
                for &n in &lens {
                    let x = 16 + phase as usize;
                    let stride = x + n + 5;
                    let init: Vec<u32> = (0..2 * stride).map(|_| rng.u32()).collect();
                    let (fg, bg) = (rng.u32(), rng.u32());
                    let mut got = init.clone();
                    let mut want = init.clone();
                    let pat = [word];
                    let fill = Fill { pattern: &pat, mode, fg, bg };
                    // SAFETY: row 1, columns `x .. x + n`, lies inside both
                    // stores of two rows of `x + n + 5` pixels.
                    unsafe {
                        raster_kernel::fill_span(got.as_mut_ptr().add(stride + x), n, phase, word, mode, fg, bg);
                        fill_pattern_reference(
                            Surface { base: want.as_mut_ptr(), stride_px: stride },
                            Area { x, y: 1, w: n, h: 1 },
                            &fill,
                        );
                    }
                    assert!(got == want, "mode {mode:?} word {word:04x} phase {phase} n {n}");
                }
            }
        }
    }
}
