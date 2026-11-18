//! V66 — the EdDI `vq_scrninfo` answer at both VDI depths: the 32-plane table
//! of `docs/api/vdi/eddi-vq_scrninfo-32.md` §2 word for word, and the
//! structural self-test of ADR-038 point 13 (i) as the spec's statements
//! S1-S7 (§6), each at 16 and at 32 planes.
//!
//! - §2: the 273 words at 32 planes through the dispatcher, on the virtual and
//!   the physical handle; output counts, nothing past `INTOUT[272]`, PTSOUT
//!   untouched. The expected table is written from the spec, not from the code.
//! - §5: at 16 the dispatched answer is the 16-plane table; its word-for-word
//!   check against the Enhancer dump is `vdi_eddi_scrninfo.rs`, unchanged.
//! - §6 S1 shape, S2 counts, S3 partition, S4 ascending gun bits, S5 colour
//!   count, S6 the round trip (a pixel built from the tables, written through a
//!   memory form of the setting's depth with `vro_cpyfm`, read with
//!   `v_get_pixel`, equal after masking to the R, G, B table bits), S7 the pen
//!   drawn and copied back into a memory form (optional in the ticket).
//! - The plane counts without a table (`scrninfo_table_for` gives none); the
//!   dispatcher's fall-through for them is in `vdi_v62_planes_setting.rs`.
//!
//! The backend is the kernel's `VdiScreen` over a host store, shared with the
//! V63-V65 tests (`vdi_planes32_common`).
//!
//! Ticket: `TASK/V66.md`.

#![cfg(all(target_arch = "aarch64", any(target_os = "macos", target_os = "linux")))]

mod vdi_planes32_common;

use tos_api::bdos::process::Abi;
use tos_api::vdi::eddi::{scrninfo_table, scrninfo_table_for, SCRNINFO_WORDS};
use vdi_planes32_common::*;

const OP_VQ_EXTND: i16 = 102;
const OP_VRO_CPYFM: i16 = 109;
const S_ONLY: i16 = 3;

/// `vq_scrninfo` at 32 planes as the spec's §2 lists it, written from the
/// table there: format 2, software CLUT, 32 planes, 2^24 colours high word
/// first, `[5..7] = 0`, 8/8/8 gun bits, no alpha or genlock, 8 unused bits,
/// bit order 1; red 16..23, green 8..15, blue 0..7, unused 24..31; `-1` in
/// the rest of 16..127; 0 in 128..272.
fn spec_table_32() -> [i16; SCRNINFO_WORDS] {
    let mut w = [0i16; SCRNINFO_WORDS];
    w[..16].copy_from_slice(&[2, 2, 32, 0x0100, 0, 0, 0, 0, 8, 8, 8, 0, 0, 8, 1, 0]);
    for v in &mut w[16..128] {
        *v = -1;
    }
    for k in 0..8usize {
        w[16 + k] = 16 + k as i16; // red
        w[32 + k] = 8 + k as i16; // green
        w[48 + k] = k as i16; // blue
        w[96 + k] = 24 + k as i16; // unused (the X byte)
    }
    w
}

/// The six bit-number tables of the answer (`n_vq_scrninfo.c`): first index,
/// length, index of the count word. Red, green, blue, alpha, genlock, unused.
const TABLES: [(usize, usize, usize); 6] = [(16, 16, 8), (32, 16, 9), (48, 16, 10), (64, 16, 11), (80, 16, 12), (96, 32, 13)];

/// One VDI call with the outputs poisoned; returns `d0`.
fn call(pb: &mut Pb, opcode: i16, sub: i16, handle: i16, intin: &[i16]) -> i16 {
    pb.call(opcode, sub, handle, intin, &[])
}

/// The physical workstation's handle (opened at the current setting).
fn phys_handle() -> i16 {
    let mut pb = Pb::new();
    let mut work_in = [1i16; 11];
    work_in[10] = 2;
    pb.call(1, 0, 0, &work_in, &[]);
    pb.contrl[6]
}

/// `vq_scrninfo` (`CONTRL[5] == 1`, `INTIN[0] == 2`) on `handle`: the 273
/// words, after checking the call's frame (spec §2, unchanged from 16).
fn vq_scrninfo(handle: i16) -> [i16; SCRNINFO_WORDS] {
    let mut pb = Pb::new();
    let ret = call(&mut pb, OP_VQ_EXTND, 1, handle, &[2]);
    assert_eq!(ret, 0, "d0");
    assert_eq!(pb.contrl[4], 272, "CONTRL[4]");
    assert_eq!(pb.contrl[2], 0, "CONTRL[2]");
    assert!(pb.intout[SCRNINFO_WORDS..].iter().all(|&w| w == POISON), "INTOUT past 272 written");
    assert!(pb.ptsout.iter().all(|&w| w == POISON), "PTSOUT written");
    let mut t = [0i16; SCRNINFO_WORDS];
    t.copy_from_slice(&pb.intout[..SCRNINFO_WORDS]);
    t
}

/// The numbered entries of table `i` of [`TABLES`].
fn numbers(t: &[i16; SCRNINFO_WORDS], i: usize) -> Vec<i16> {
    let (at, len, _) = TABLES[i];
    t[at..at + len].iter().copied().filter(|&n| n != -1).collect()
}

/// `M`: the OR of `1 << n` over the R, G, B table numbers (spec S6).
fn colour_mask(t: &[i16; SCRNINFO_WORDS]) -> u32 {
    (0..3).flat_map(|c| numbers(t, c)).fold(0u32, |m, n| m | (1u32 << n))
}

/// The pixel a program builds from the tables (spec S6): bit `k` of the gun
/// value `q[c]` goes to pixel bit `T[16 + 16 * c + k]`.
fn build_pixel(t: &[i16; SCRNINFO_WORDS], q: [u32; 3]) -> u32 {
    let mut p = 0u32;
    for (c, &qc) in q.iter().enumerate() {
        for k in 0..t[8 + c] as usize {
            p |= ((qc >> k) & 1) << t[16 + 16 * c + k];
        }
    }
    p
}

/// The gun triples of S6/S7: black, white, each single-gun maximum, one triple
/// per table bit, and one mixed value; gun values have `T[8 + c]` bits.
fn triples(t: &[i16; SCRNINFO_WORDS]) -> Vec<[u32; 3]> {
    let max = |c: usize| (1u32 << t[8 + c]) - 1;
    let mut v = vec![[0, 0, 0], [max(0), max(1), max(2)], [max(0), 0, 0], [0, max(1), 0], [0, 0, max(2)]];
    for c in 0..3 {
        for k in 0..t[8 + c] as u32 {
            let mut q = [0u32; 3];
            q[c] = 1 << k;
            v.push(q);
        }
    }
    v.push([0x12 & max(0), 0x34 & max(1), 0x56 & max(2)]);
    v
}

/// An LP64 MFDB (`mt_gem.h:2757-2768`): `fd_addr` 8 bytes, then the WORDs.
#[repr(C)]
#[derive(Clone, Copy)]
struct Mfdb {
    addr: u64,
    w: i16,
    h: i16,
    wdwidth: i16,
    stand: i16,
    nplanes: i16,
    r1: i16,
    r2: i16,
    r3: i16,
}

impl Mfdb {
    fn screen() -> Mfdb {
        Mfdb { addr: 0, w: 0, h: 0, wdwidth: 0, stand: 0, nplanes: 0, r1: 0, r2: 0, r3: 0 }
    }
}

/// A one-row device memory form of the setting's depth (`fd_stand = 0`,
/// `fd_nplanes = T[2]`, `fd_wdwidth * 16` pixels a row; ADR-038 point 5 at 32,
/// ADR-024 point 4 at 16), over a `u32` buffer with a canary tail.
struct Form {
    buf: Vec<u32>,
    w: i16,
    planes: i16,
}

const TAIL: usize = 32;
const CANARY: u32 = 0xC0DE_CAFE;

impl Form {
    fn new(w: i16, planes: i16) -> Form {
        let px = (w as usize).div_ceil(16) * 16;
        let words = px * planes as usize / 32;
        Form { buf: vec![0; words + TAIL], w, planes }
    }
    fn body_words(&self) -> usize {
        self.buf.len() - TAIL
    }
    fn set(&mut self, x: usize, v: u32) {
        if self.planes == 32 {
            self.buf[x] = v;
        } else {
            // SAFETY: `x` lies inside the row of 16-bit pixels the buffer holds.
            unsafe { (self.buf.as_mut_ptr() as *mut u16).add(x).write(v as u16) }
        }
    }
    fn get(&self, x: usize) -> u32 {
        if self.planes == 32 {
            self.buf[x]
        } else {
            // SAFETY: as in `set`.
            u32::from(unsafe { (self.buf.as_ptr() as *const u16).add(x).read() })
        }
    }
    fn mfdb(&mut self) -> Mfdb {
        let wdwidth = (self.w + 15) / 16;
        Mfdb { addr: self.buf.as_mut_ptr() as u64, w: self.w, h: 1, wdwidth, stand: 0, nplanes: self.planes, r1: 0, r2: 0, r3: 0 }
    }
    fn tail_intact(&self) -> bool {
        self.buf[self.body_words()..].iter().all(|&v| v == CANARY)
    }
    fn arm_tail(&mut self) {
        let n = self.body_words();
        self.buf[n..].fill(CANARY);
    }
}

impl Ws {
    /// `vro_cpyfm` with the two MFDB pointers at `CONTRL[7..10]` and
    /// `CONTRL[11..14]` (LP64, `N_PTRINTS = 4`, `gem_vdiP.h:177,204`).
    fn vro_cpyfm(&mut self, op: i16, s: [i16; 4], d: [i16; 4], src: &Mfdb, dst: &mut Mfdb) {
        let pb = &mut self.pb;
        let pts = [s[0], s[1], s[2], s[3], d[0], d[1], d[2], d[3]];
        pb.contrl.fill(0);
        pb.intout.fill(POISON);
        pb.ptsout.fill(POISON);
        pb.intin[0] = op;
        pb.ptsin[..8].copy_from_slice(&pts);
        pb.contrl[0] = OP_VRO_CPYFM;
        pb.contrl[1] = 4;
        pb.contrl[3] = 1;
        pb.contrl[6] = self.h;
        let (s, d) = (src as *const Mfdb as u64, dst as *mut Mfdb as u64);
        // SAFETY: `contrl` has 16 WORDs; slots 7..10 and 11..14 hold the two
        // pointers, written unaligned as gemlib packs them.
        unsafe {
            (pb.contrl[7..].as_mut_ptr() as *mut u64).write_unaligned(s);
            (pb.contrl[11..].as_mut_ptr() as *mut u64).write_unaligned(d);
        }
        // SAFETY: `block` holds five live pointers to arrays owned by `pb`; the
        // MFDBs and their bitmaps outlive the call.
        unsafe { tos_api::vdi::dispatch(pb.block.as_ptr() as u64, Abi::Lp64) };
    }
}

// ── §2 / §5: the answer at each setting ──────────────────────────────────

/// Spec §2: the 273 words at 32 planes, word for word, on the virtual and the
/// physical handle (E6 as at 16), with the documented frame.
#[test]
fn vq_scrninfo_at_32_planes_answers_the_spec_table_word_for_word() {
    let _g = setting(32);
    let ws = Ws::open();
    let want = spec_table_32();
    for handle in [ws.h, phys_handle()] {
        let t = vq_scrninfo(handle);
        for i in 0..SCRNINFO_WORDS {
            assert_eq!(t[i], want[i], "handle {handle}: work_out[{i}]");
        }
    }
    // The library's own table is the one dispatched.
    assert_eq!(scrninfo_table_for(32), Some(want));
    // Spec §2 tally: 4 tables of 8 numbered entries, 80 `-1` in 16..127.
    assert_eq!(want[16..128].iter().filter(|&&n| n != -1).count(), 32);
    assert_eq!(want[16..128].iter().filter(|&&n| n == -1).count(), 80);
}

/// Spec §5: at 16 the dispatched answer is the 16-plane table (whose word-for-
/// word check against the Enhancer is `vdi_eddi_scrninfo.rs`), and the table
/// for 16 is the default one.
#[test]
fn at_16_planes_the_dispatched_answer_is_the_16_plane_table() {
    let _g = setting(16);
    let ws = Ws::open();
    let t16 = scrninfo_table();
    for handle in [ws.h, phys_handle()] {
        assert_eq!(vq_scrninfo(handle), t16, "handle {handle}");
    }
    assert_eq!(scrninfo_table_for(16), Some(t16));
    assert_eq!(t16[2], 16);
    assert_eq!(t16[14], 0x02, "Falcon bit order, E3");
}

/// Spec §5 [I]: only 16 and 32 have a table; any other count gets none (the
/// dispatcher then answers `vq_extnd`, `vdi_v62_planes_setting.rs`).
#[test]
fn other_plane_counts_have_no_table() {
    for planes in [0u16, 1, 2, 4, 8, 15, 17, 24, 31, 33, 64] {
        assert_eq!(scrninfo_table_for(planes), None, "{planes} planes");
    }
}

// ── §6: the structural self-test, at 16 and at 32 ────────────────────────

/// S1, shape: `T[0] = 2`, `T[1] = 2`, `T[2]` = the setting, and `T[2]` equals
/// `vq_extnd(1)` `INTOUT[4]` of the same setting.
#[test]
fn s1_shape_and_the_plane_count_of_vq_extnd() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let ws = Ws::open();
        let t = vq_scrninfo(ws.h);
        assert_eq!((t[0], t[1], t[2]), (2, 2, planes as i16), "{planes}: T[0..3]");
        let mut pb = Pb::new();
        assert_eq!(call(&mut pb, OP_VQ_EXTND, 0, ws.h, &[1]), 1, "{planes}: vq_extnd(1) d0");
        assert_eq!(pb.intout[4], t[2], "{planes}: INQ_TAB[4] = T[2]");
    }
}

/// S2, counts match the tables: the numbered entries of each table are its
/// first `T[8..=13]` entries, a `-1` is never followed by a number.
#[test]
fn s2_the_counts_match_the_tables() {
    for (planes, counts) in [(16u16, [5i16, 5, 5, 0, 1, 0]), (32, [8, 8, 8, 0, 0, 8])] {
        let _g = setting(planes);
        let ws = Ws::open();
        let t = vq_scrninfo(ws.h);
        for (i, &(at, len, count_at)) in TABLES.iter().enumerate() {
            let n = t[count_at] as usize;
            assert_eq!(t[count_at], counts[i], "{planes}: T[{count_at}]");
            assert!(t[at..at + n].iter().all(|&b| b != -1), "{planes}: table {at}: a gap in the first {n}");
            assert!(t[at + n..at + len].iter().all(|&b| b == -1), "{planes}: table {at}: a number after the first {n}");
        }
    }
}

/// S3, the tables partition the pixel: the six sets are pairwise disjoint and
/// their union is exactly `{0 .. T[2]-1}`.
#[test]
fn s3_the_bit_tables_partition_the_pixel() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let ws = Ws::open();
        let t = vq_scrninfo(ws.h);
        let mut seen = vec![0u32; t[2] as usize];
        let mut total = 0usize;
        for i in 0..TABLES.len() {
            for n in numbers(&t, i) {
                assert!((0..t[2]).contains(&n), "{planes}: bit {n} outside the pixel");
                seen[n as usize] += 1;
                total += 1;
            }
        }
        assert!(seen.iter().all(|&s| s == 1), "{planes}: every bit exactly once: {seen:?}");
        assert_eq!(total, t[2] as usize, "{planes}: sum of the counts = T[2]");
        let count_sum: i16 = TABLES.iter().map(|&(_, _, c)| t[c]).sum();
        assert_eq!(count_sum, t[2], "{planes}: T[8] + .. + T[13] = T[2]");
    }
}

/// S4, gun bits ascending: within R, G, B entry `k` is the gun's least
/// significant bit plus `k`.
#[test]
fn s4_gun_bits_ascend_from_the_lsb() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let ws = Ws::open();
        let t = vq_scrninfo(ws.h);
        for c in 0..3 {
            let g = numbers(&t, c);
            for (k, &n) in g.iter().enumerate() {
                assert_eq!(n, g[0] + k as i16, "{planes}: gun {c} entry {k}");
            }
        }
    }
}

/// S5, colour count: the LONG `T[3]:T[4]` is `1 << (T[8] + T[9] + T[10])`.
#[test]
fn s5_the_colour_count_matches_the_gun_bits() {
    for (planes, colours) in [(16u16, 0x0000_8000u32), (32, 0x0100_0000)] {
        let _g = setting(planes);
        let ws = Ws::open();
        let t = vq_scrninfo(ws.h);
        let long = (u32::from(t[3] as u16) << 16) | u32::from(t[4] as u16);
        assert_eq!(long, 1u32 << (t[8] + t[9] + t[10]), "{planes}: [3]:[4]");
        assert_eq!(long, colours, "{planes}: [3]:[4]");
    }
}

/// S6, the round trip: a pixel built from the tables, written (X = 0) into a
/// memory form of depth `T[2]`, copied to the screen with `vro_cpyfm` S_ONLY
/// and read with `v_get_pixel`, comes back as `p & M`.
#[test]
fn s6_a_pixel_built_from_the_tables_round_trips() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let t = vq_scrninfo(ws.h);
        let m = colour_mask(&t);
        assert_eq!(m, if planes == 32 { 0x00FF_FFFF } else { 0xFFDF }, "{planes}: M");
        let qs = triples(&t);
        let n = qs.len() as i16;
        let mut form = Form::new(n, t[2]);
        let pixels: Vec<u32> = qs.iter().map(|&q| build_pixel(&t, q)).collect();
        for (x, &p) in pixels.iter().enumerate() {
            form.set(x, p);
        }
        form.arm_tail();
        let (x0, y0) = (17i16, 41i16);
        ws.vro_cpyfm(S_ONLY, [0, 0, n - 1, 0], [x0, y0, x0 + n - 1, y0], &form.mfdb(), &mut Mfdb::screen());
        for (i, (&q, &p)) in qs.iter().zip(&pixels).enumerate() {
            let (hi, lo) = ws.v_get_pixel(x0 + i as i16, y0);
            let got = (u32::from(hi as u16) << 16) | u32::from(lo);
            assert_eq!(got, p & m, "{planes}: triple {q:?}, pixel {p:#010x}");
        }
        assert!(form.tail_intact(), "{planes}: form tail");
    }
}

/// S7, the other direction and the pen: a pen set as DOOM sets it
/// (`c = (q * 1000 + 127) / 255` at 32; the 5-bit counterpart
/// `(q * 1000 + 16) / 31`, EmuTOS `fivebits2vdi`, at 16), drawn with
/// `vr_recfl` and copied into a memory form, gives `w & M == p` for the
/// table-built `p`.
#[test]
fn s7_a_drawn_pen_copies_back_as_the_table_built_pixel() {
    const PEN: i16 = 20;
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let mut ws = Ws::open();
        ws.v_clrwk();
        let t = vq_scrninfo(ws.h);
        let m = colour_mask(&t);
        let qs = triples(&t);
        let n = qs.len() as i16;
        let (x0, y0) = (5i16, 77i16);
        ws.solid_fill(PEN);
        for (i, q) in qs.iter().enumerate() {
            let rgb: Vec<i16> = q.iter().map(|&qc| if planes == 32 { a8(qc) } else { a5(qc) }).collect();
            ws.call(14, 0, &[PEN, rgb[0], rgb[1], rgb[2]], &[]); // vs_color
            let x = x0 + i as i16;
            ws.vr_recfl(x, y0, x, y0);
        }
        let mut form = Form::new(n, t[2]);
        form.arm_tail();
        ws.vro_cpyfm(S_ONLY, [x0, y0, x0 + n - 1, y0], [0, 0, n - 1, 0], &Mfdb::screen(), &mut form.mfdb());
        for (i, &q) in qs.iter().enumerate() {
            let p = build_pixel(&t, q);
            assert_eq!(form.get(i) & m, p, "{planes}: pen for {q:?}");
        }
        assert!(form.tail_intact(), "{planes}: form tail");
    }
}
