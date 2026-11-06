//! V33 (TASK/V33.md, finding VDI-TEXT-010 of
//! `docs/audits/2026-11-06-v18-t2-vdi-text.md`): `vst_load_fonts` /
//! `vst_unload_fonts` and the font chain, against the pinned oracle.
//!
//! Source of every expected value: the probe cases T13A/T13B and TX* of
//! `tosdump/probes/vdprobe.c` on EmuTOS 1a7484ab, Hatari Falcon030 truecolor,
//! run `measurements/measurement-2026-11-06-v33/RUNINFO.TXT` (`probe/e1`):
//! the numbers from `VDPROBE.LOG`, the pixels as one FNV-1a over each grab
//! rectangle's screen words (`method/gen-grabs.py`). A failing grab is looked
//! at in the probe's maps (`vdprobe-compare.py --maps`).
//!
//! The chain is what a GDOS builds: [`Fh`] is the caller's `Fonthead` in the
//! native layout (decision D3), passed in `CONTRL[12..15]` (decision D1). Not
//! on the oracle because undefined there: the NULL chain (decision D2), checked
//! here against its own rule.
//!
//! The kernel sits above this harness's [`MemBackend`](crate::harness), the
//! same as for `cases_v30`.

use crate::harness::{reset_vdi, screen_word, workstation, Pb, ABI, VDI_LOCK};

const F_HORZ: u16 = 2;
const F_STD: u16 = 4;
const F_MONO: u16 = 8;

/// EmuTOS `struct font_head` (`include/fonthdr.h:30-60`), LP64 native.
#[repr(C)]
#[derive(Clone, Copy)]
struct Fh {
    font_id: i16,
    point: i16,
    name: [u8; 32],
    first_ade: u16,
    last_ade: u16,
    top: u16,
    ascent: u16,
    half: u16,
    descent: u16,
    bottom: u16,
    max_char_width: u16,
    max_cell_width: u16,
    left_offset: u16,
    right_offset: u16,
    thicken: u16,
    ul_size: u16,
    lighten: u16,
    skew: u16,
    flags: u16,
    hor_table: *const u8,
    off_table: *const u16,
    dat_table: *const u8,
    form_width: u16,
    form_height: u16,
    next_font: *mut Fh,
    reserved: u16,
}

/// The tables outlive the test (a chain may be read until the next open).
fn leak<T: Clone>(v: Vec<T>) -> &'static mut [T] {
    Vec::leak(v)
}

/// The probe's `tx_pattern`.
fn pattern(fw: usize, fh: usize) -> &'static mut [u8] {
    let mut d = vec![0u8; fw * fh];
    for r in 0..fh {
        for c in 0..fw {
            d[r * fw + c] = (((c * 29 + r * 53) as u8) ^ 0xA5) | 0x81;
        }
    }
    leak(d)
}

fn off_linear(cell: u16, n: usize) -> &'static mut [u16] {
    leak((0..=n as u16).map(|c| c * cell).collect())
}

/// The probe's `tx_head`.
#[allow(clippy::too_many_arguments)]
fn head(
    id: i16,
    point: i16,
    name: &str,
    top: u16,
    bottom: u16,
    cell: u16,
    first: u16,
    last: u16,
    flags: u16,
    off: *const u16,
    dat: *const u8,
    fw: u16,
    fh: u16,
) -> Fh {
    let mut n = [0u8; 32];
    n[..name.len()].copy_from_slice(name.as_bytes());
    Fh {
        font_id: id,
        point,
        name: n,
        first_ade: first,
        last_ade: last,
        top,
        ascent: top,
        half: top * 2 / 3,
        descent: if bottom != 0 { bottom } else { 1 },
        bottom,
        max_char_width: cell,
        max_cell_width: cell,
        left_offset: 2,
        right_offset: 3,
        thicken: 1,
        ul_size: 1,
        lighten: 0x5555,
        skew: 0x5555,
        flags,
        hor_table: core::ptr::null(),
        off_table: off,
        dat_table: dat,
        form_width: fw,
        form_height: fh,
        next_font: core::ptr::null_mut(),
        reserved: 0,
    }
}

/// The probe's `t13_build`: one font, id 7, F_STDFORM | F_MONOSPACE.
fn t13_font() -> &'static mut Fh {
    let off = off_linear(8, 64);
    let dat = pattern(64, 8);
    let mut f = head(7, 12, "AUDIT FONT SEVEN", 6, 1, 8, 32, 95, F_STD | F_MONO, off.as_ptr(), dat.as_ptr(), 64, 8);
    f.ascent = 6;
    f.half = 4;
    f.left_offset = 1;
    f.right_offset = 1;
    Box::leak(Box::new(f))
}

/// The probe's `tx_prop`: char c is 3 + (c % 6) wide, 12 rows, top 8, bottom 3.
fn prop_font(flags: u16) -> &'static mut Fh {
    let mut off = vec![0u16; 65];
    let mut x = 0u16;
    for (c, o) in off.iter_mut().enumerate().take(64) {
        *o = x;
        x += 3 + (c as u16 % 6);
    }
    off[64] = x;
    let fw = (x.div_ceil(16)) * 2;
    let dat = pattern(fw as usize, 12);
    let hor = leak((0..128u32).map(|c| (c % 4) as u8).collect());
    let mut f = head(20, 12, "AUDIT PROPORTIONAL", 8, 3, 8, 32, 95, flags, leak(off).as_ptr(), dat.as_ptr(), fw, 12);
    f.hor_table = hor.as_ptr();
    Box::leak(Box::new(f))
}

/// A parameter block with a 16-word `CONTRL` (the harness's has 12).
struct T {
    contrl: [i16; 16],
    intin: [i16; 300],
    ptsin: [i16; 300],
    intout: [i16; 128],
    ptsout: [i16; 128],
    handle: i16,
}

impl T {
    /// The probe's `tbegin`: `reset` plus `treset`, the chain from a failed test gone.
    fn begin() -> Box<T> {
        let wk = workstation();
        let mut pb = Pb::new();
        reset_vdi(&mut pb, wk);
        let mut t = Box::new(T {
            contrl: [0; 16],
            intin: [0; 300],
            ptsin: [0; 300],
            intout: [0; 128],
            ptsout: [0; 128],
            handle: wk.handle,
        });
        t.vdi(120, 0, 0, 1);
        t.treset();
        t
    }

    fn call(&mut self) {
        let block: [u64; 5] = [
            self.contrl.as_mut_ptr() as u64,
            self.intin.as_mut_ptr() as u64,
            self.ptsin.as_mut_ptr() as u64,
            self.intout.as_mut_ptr() as u64,
            self.ptsout.as_mut_ptr() as u64,
        ];
        // SAFETY: five live arrays, larger than anything these opcodes touch.
        unsafe { tos_api::vdi::dispatch(block.as_ptr() as u64, ABI) };
    }

    fn vdi(&mut self, op: i16, sub: i16, npts: i16, nint: i16) {
        self.contrl = [0; 16];
        self.contrl[0] = op;
        self.contrl[1] = npts;
        self.contrl[3] = nint;
        self.contrl[5] = sub;
        self.contrl[6] = self.handle;
        self.call();
    }

    fn i1(&mut self, op: i16, a: i16) {
        self.intin[0] = a;
        self.vdi(op, 0, 0, 1);
    }

    fn i2(&mut self, op: i16, a: i16, b: i16) {
        self.intin[0] = a;
        self.intin[1] = b;
        self.vdi(op, 0, 0, 2);
    }

    fn p1(&mut self, op: i16, x: i16, y: i16) {
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        self.vdi(op, 0, 1, 0);
    }

    fn treset(&mut self) {
        self.i1(21, 1);
        self.p1(12, 0, workstation().siz_tab[1]);
        self.i1(13, 0);
        self.i1(22, 1);
        self.i1(106, 0);
        self.i2(39, 0, 0);
    }

    fn gtext(&mut self, x: i16, y: i16, s: &str) {
        for (i, b) in s.bytes().enumerate() {
            self.intin[i] = b as i16;
        }
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        self.vdi(8, 0, 1, s.len() as i16);
    }

    fn justified(&mut self, x: i16, y: i16, len: i16, iw: i16, ic: i16, s: &str) {
        self.intin[0] = iw;
        self.intin[1] = ic;
        for (i, b) in s.bytes().enumerate() {
            self.intin[2 + i] = b as i16;
        }
        self.ptsin[..4].copy_from_slice(&[x, y, len, 0]);
        self.vdi(11, 10, 2, s.len() as i16 + 2);
    }

    /// `vst_load_fonts` the way a GDOS calls it (decision D1): `INTOUT[0]`.
    fn load(&mut self, chain: *const Fh) -> i16 {
        self.contrl = [0; 16];
        self.contrl[0] = 119;
        self.contrl[3] = 1;
        self.contrl[6] = self.handle;
        self.contrl[11] = 2048;
        // SAFETY: `contrl` has 16 words; the two pointers are written unaligned.
        unsafe {
            self.contrl.as_mut_ptr().add(7).cast::<u64>().write_unaligned(0x1234_5678); // scratch: not read
            self.contrl.as_mut_ptr().add(12).cast::<u64>().write_unaligned(chain as u64);
        }
        self.intout[0] = 0x7777;
        self.call();
        assert_eq!((self.contrl[2], self.contrl[4]), (0, 1), "vst_load_fonts counts");
        self.intout[0]
    }

    fn unload(&mut self) {
        self.vdi(120, 0, 0, 1);
        assert_eq!((self.contrl[2], self.contrl[4]), (0, 0), "vst_unload_fonts counts");
    }

    /// The probe's `tx_end`.
    fn end(&mut self) {
        self.i1(21, 1);
        self.vdi(120, 0, 0, 1);
        self.treset();
    }

    /// `vqt_name`: the id and the name up to its NUL.
    fn name(&mut self, n: i16) -> (i16, String) {
        self.i1(130, n);
        assert_eq!(self.contrl[4], 33, "vqt_name CONTRL[4]");
        let s = self.intout[1..33].iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();
        (self.intout[0], s)
    }

    /// `vst_font`: `INTOUT[0]`, with its counts.
    fn font(&mut self, id: i16) -> i16 {
        self.i1(21, id);
        assert_eq!((self.contrl[2], self.contrl[4]), (0, 1), "vst_font counts");
        self.intout[0]
    }

    /// `vqt_attributes`: `INTOUT[0]` and `PTSOUT[0..4]`.
    fn attr(&mut self) -> (i16, [i16; 4]) {
        self.vdi(38, 0, 0, 0);
        (self.intout[0], [self.ptsout[0], self.ptsout[1], self.ptsout[2], self.ptsout[3]])
    }

    fn height(&mut self, h: i16) -> [i16; 4] {
        self.p1(12, 0, h);
        [self.ptsout[0], self.ptsout[1], self.ptsout[2], self.ptsout[3]]
    }

    /// `vst_point`: `INTOUT[0]`, `PTSOUT[0..4]`.
    fn point(&mut self, p: i16) -> (i16, [i16; 4]) {
        self.i1(107, p);
        (self.intout[0], [self.ptsout[0], self.ptsout[1], self.ptsout[2], self.ptsout[3]])
    }

    fn width(&mut self, c: i16) -> (i16, i16, i16, i16) {
        self.ptsout[0] = 0x7777;
        self.i1(117, c);
        (self.intout[0], self.ptsout[0], self.ptsout[2], self.ptsout[4])
    }

    /// `vqt_extent` of `s`: `PTSOUT[0..8]`.
    fn extent(&mut self, s: &str) -> [i16; 8] {
        for (i, b) in s.bytes().enumerate() {
            self.intin[i] = b as i16;
        }
        self.vdi(116, 0, 0, s.len() as i16);
        let mut o = [0; 8];
        o.copy_from_slice(&self.ptsout[..8]);
        o
    }

    /// The probe's `tx_prop_begin`.
    fn prop_begin() -> Box<T> {
        let mut t = T::begin();
        assert_eq!(t.load(prop_font(F_STD)), 1);
        t.i1(21, 20);
        t.p1(12, 0, 8);
        t
    }
}

/// `(name, (x, y, w, h), FNV-1a of the words, non-white words)` of `method/gen-grabs.py`.
type GrabHash = (&'static str, (i16, i16, i16, i16), u64, u32);

#[rustfmt::skip]
const GRABS: &[GrabHash] = &[
    ("T13A", (0, 30, 120, 50), 0x4e719d68f0b15a1d, 105),
    ("T13B", (0, 30, 120, 50), 0x547e7dbd1030d30d, 144),
    ("TXB", (0, 60, 140, 70), 0x83d64c400bb0e9d5, 2048),
    ("TXC", (0, 60, 120, 60), 0xac24f12bdf867b61, 1152),
    ("TXD", (0, 80, 120, 40), 0xd1d85af74adf4b6f, 256),
    ("TXG", (0, 80, 120, 40), 0xab0d943c9107443d, 105),
    ("TXI", (0, 80, 200, 40), 0x643b850710bf69bd, 804),
    ("TXIb", (0, 80, 200, 40), 0x201d73c263d370d5, 948),
    ("TXIs", (0, 80, 200, 40), 0xf81abf6e5851845d, 864),
    ("TXIo", (0, 80, 200, 40), 0x94edcb6c27d75bb9, 1274),
    ("TXIu", (0, 80, 200, 40), 0xa7f7d288ff6e0d85, 805),
    ("TXR1", (60, 60, 80, 160), 0x751de3811990105d, 804),
    ("TXR2", (120, 70, 200, 50), 0x8760bff6d9eaf89d, 804),
    ("TXR3", (60, 40, 80, 160), 0x62df09e8db0bc1dd, 804),
    ("TXS1", (0, 70, 300, 50), 0x4ffe1f1df1eade55, 3216),
    ("TXS2", (0, 70, 300, 50), 0xaf13336fde42f71b, 1800),
    ("TXS3", (0, 70, 300, 50), 0x0f36c1eb1e56e335, 294),
    ("TXJ", (0, 80, 260, 40), 0x60c2ab7c99bd322d, 804),
    ("TXO", (0, 80, 120, 40), 0xd41e62398f18cc1d, 216),
    ("TXM", (0, 40, 200, 80), 0x8d766944c4b57665, 1312),
    ("TXC2", (0, 40, 300, 100), 0xb2cfa5fc696fad83, 2172),
    ("TXH2", (0, 80, 240, 40), 0x115f53d1bc98cf5d, 804),
    ("TXK", (0, 50, 120, 70), 0xc64b7d23425f2265, 2960),
    ("TXK2", (0, 200, 200, 130), 0x4604ef47f09c1005, 11840),
    ("TXL", (0, 80, 120, 60), 0x8c1fbf5fcc45d76d, 1666),
    ("TXN", (0, 60, 120, 60), 0x478efe7aa2f3259d, 2048),
    ("TXQ", (0, 80, 200, 40), 0x6ac7324deb475fbf, 1664),
    ("TXQ2", (0, 80, 200, 40), 0x06460504c4347adf, 433),
    ("TXQ3", (0, 80, 200, 40), 0xdc773ac263a0b1dd, 508),
];

/// The grab `name` must be the oracle's, word for word.
fn compare(name: &str) {
    let &(_, (x, y, w, h), want, nonwhite) = GRABS.iter().find(|g| g.0 == name).expect("grab in the table");
    let mut v: u64 = 0xcbf2_9ce4_8422_2325;
    let mut n = 0;
    for yy in y..y + h {
        for xx in x..x + w {
            let word = screen_word(xx, yy);
            for b in [(word >> 8) as u8, word as u8] {
                v = (v ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
            if word != 0xFFFF {
                n += 1;
            }
        }
    }
    assert_eq!((v, n), (want, nonwhite), "{name}: the grab differs from the oracle (hash, non-white words)");
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    VDI_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// D3: the layout the kernel reads is the native one.
#[test]
fn v33_fonthead_layout_is_the_native_one() {
    use core::mem::{offset_of, size_of};
    assert_eq!(offset_of!(Fh, flags), 66);
    assert_eq!(offset_of!(Fh, hor_table), 72);
    assert_eq!(offset_of!(Fh, off_table), 80);
    assert_eq!(offset_of!(Fh, dat_table), 88);
    assert_eq!(offset_of!(Fh, form_width), 96);
    assert_eq!(offset_of!(Fh, form_height), 98);
    assert_eq!(offset_of!(Fh, next_font), 104);
    assert_eq!(offset_of!(Fh, reserved), 112);
    assert_eq!(size_of::<Fh>(), 120);
}

/// T13A/T13B: load, name, `vq_extnd` unchanged, select, metrics, draw, second load
/// 0, unload.
#[test]
fn v33_probe_t13a_load_select_draw_unload() {
    let _g = lock();
    let mut t = T::begin();
    let f = t13_font();
    assert_eq!(t.load(f), 1);
    assert_eq!(t.name(1), (1, "6x6 system font".into()));
    assert_eq!(t.name(2), (7, "AUDIT FONT SEVEN".into()));
    t.intin[0] = 0;
    t.vdi(102, 0, 0, 1);
    assert_eq!(t.intout[10], 1, "vq_extnd(0): DEV_TAB[10] stays (R10)");
    assert_eq!(t.font(7), 7);
    assert_eq!(t.attr(), (7, [5, 4, 5, 6]));
    t.vdi(131, 0, 0, 0);
    assert_eq!((t.intout[0], t.intout[1]), (32, 95));
    assert_eq!(t.ptsout[..10], [5, 1, 0, 1, 0, 3, 0, 4, 0, 4]);
    t.gtext(20, 60, "AB@_");
    compare("T13A");
    assert_eq!(t.load(f), 0, "one chance per workstation (R1)");
    t.unload();
    assert_eq!(t.font(7), 1);
    assert_eq!(t.name(2), (1, "6x6 system font".into()));
    t.i1(21, 1);
    t.treset();

    let mut t = T::begin();
    t.gtext(20, 60, "AB@_");
    compare("T13B");
}

/// TXA: ids 7,7,8,7 are 3 faces, named in chain order; 7,7 is one.
#[test]
fn v33_probe_txa_faces_of_a_chain() {
    let _g = lock();
    let mut t = T::begin();
    let chain: &'static mut [Fh] = leak(vec![*t13_font(); 4]);
    let base = chain.as_mut_ptr();
    for (i, f) in chain.iter_mut().enumerate().take(3) {
        // SAFETY: in the leaked slice.
        f.next_font = unsafe { base.add(i + 1) };
    }
    chain[2].font_id = 8;
    chain[2].name[..16].copy_from_slice(b"AUDIT FONT EIGHT");
    assert_eq!(t.load(base), 3);
    let want = [
        (0, 1, "6x6 system font"),
        (1, 1, "6x6 system font"),
        (2, 7, "AUDIT FONT SEVEN"),
        (3, 8, "AUDIT FONT EIGHT"),
        (4, 7, "AUDIT FONT SEVEN"),
        (5, 1, "6x6 system font"),
        (6, 1, "6x6 system font"),
    ];
    for (n, id, name) in want {
        assert_eq!(t.name(n), (id, name.into()), "vqt_name({n})");
    }
    assert_eq!(t.font(8), 8);
    assert_eq!(t.attr(), (8, [5, 4, 5, 6]));
    assert_eq!(t.font(7), 7);
    assert_eq!(t.attr(), (7, [5, 4, 5, 6]));
    t.end();

    let mut t = T::begin();
    chain[1].next_font = core::ptr::null_mut();
    assert_eq!(t.load(base), 1, "TXA2");
    assert_eq!(t.name(2).0, 7);
    assert_eq!(t.name(3).0, 1);
    t.end();
}

/// TXB: one face in two sizes: the nearest size, the DDA, the double, the re-drive.
#[test]
fn v33_probe_txb_sizes_of_a_face() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(8, 64);
    let f1 = Box::leak(Box::new(head(9, 16, "AUDIT SIZES", 13, 2, 8, 32, 95, F_STD | F_MONO, off.as_ptr(), pattern(64, 16).as_ptr(), 64, 16)));
    let f0 = Box::leak(Box::new(head(9, 8, "AUDIT SIZES", 6, 1, 8, 32, 95, F_STD | F_MONO, off.as_ptr(), pattern(64, 8).as_ptr(), 64, 8)));
    f0.next_font = f1;
    assert_eq!(t.load(f0), 1);
    assert_eq!(t.font(9), 9);
    assert_eq!(t.attr(), (9, [5, 4, 5, 6]));
    assert_eq!(t.height(6), [8, 6, 8, 8]);
    assert_eq!(t.height(13), [8, 13, 8, 16]);
    assert_eq!(t.height(10), [13, 10, 13, 13]);
    assert_eq!(t.height(26), [16, 27, 16, 32]);
    assert_eq!(t.height(3), [4, 3, 4, 5]);
    assert_eq!(t.height(0), [1, 1, 1, 3]);
    assert_eq!(t.point(8), (8, [8, 6, 8, 8]));
    assert_eq!(t.point(16), (16, [8, 13, 8, 16]));
    assert_eq!(t.point(12), (8, [8, 6, 8, 8]));
    assert_eq!(t.point(32), (32, [16, 27, 16, 32]));
    assert_eq!(t.point(20), (16, [8, 13, 8, 16]));
    assert_eq!(t.point(34), (32, [16, 27, 16, 32]));
    assert_eq!(t.point(4), (8, [8, 6, 8, 8]));
    assert_eq!(t.point(32), (32, [16, 27, 16, 32]));
    assert_eq!(t.font(9), 9);
    assert_eq!(t.attr(), (9, [16, 27, 16, 32]), "re-drive by point");
    t.gtext(20, 100, "AB@_");
    compare("TXB");
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [14, 27, 16, 32]), "system face, pts_mode");
    t.end();
}

/// TXC: a loaded font with the id of a built-in: no new face; `vst_font(1)` stays on
/// the built-in, the size walk finds it.
#[test]
fn v33_probe_txc_loaded_font_with_id_1() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(12, 64);
    let f = Box::leak(Box::new(head(1, 14, "AUDIT ONE", 20, 3, 12, 32, 95, F_STD | F_MONO, off.as_ptr(), pattern(96, 24).as_ptr(), 96, 24)));
    assert_eq!(t.load(f), 1);
    assert_eq!(t.name(1).1, "6x6 system font");
    assert_eq!(t.name(2).1, "6x6 system font");
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [5, 4, 6, 6]));
    assert_eq!(t.height(20), [12, 20, 12, 24]);
    assert_eq!(t.attr(), (1, [12, 20, 12, 24]));
    t.gtext(20, 100, "AB@_");
    compare("TXC");
    assert_eq!(t.height(8), [9, 8, 11, 10]);
    assert_eq!(t.attr(), (1, [9, 8, 11, 10]));
    assert_eq!(t.point(14), (14, [12, 20, 12, 24]));
    assert_eq!(t.attr(), (1, [12, 20, 12, 24]));
    t.end();
    assert_eq!(t.attr(), (1, [5, 4, 6, 6]));
}

/// TXD: no `F_STDFORM`, odd `form_width`: `(65 * 8) / 2` words swapped in the caller's
/// table, the flag set in its header, the glyphs read in the swapped order.
#[test]
fn v33_probe_txd_no_stdform_swaps_in_the_callers_memory() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(8, 64);
    // One byte behind the 520 of the table, untouched: the odd width's last byte is
    // the first half of a word the count leaves out only when the product is odd.
    let mut v = pattern(65, 8).to_vec();
    v.push(181);
    let d = leak(v);
    assert_eq!((d[0], d[1], d[2], d[3], d[518], d[519], d[520]), (165, 185, 159, 243, 179, 151, 181), "the probe's bytes");
    let f = Box::leak(Box::new(head(12, 12, "AUDIT SWAPPED", 6, 1, 8, 32, 95, F_MONO, off.as_ptr(), d.as_ptr(), 65, 8)));
    assert_eq!(t.load(f), 1);
    assert_eq!((d[0], d[1], d[2], d[3], d[518], d[519], d[520]), (185, 165, 243, 159, 151, 179, 181));
    assert_eq!(f.flags, 12);
    assert_eq!(t.font(12), 12);
    t.p1(12, 0, 6);
    t.gtext(20, 100, "AB@_");
    compare("TXD");
    t.end();
}

/// TXE: two headers without the flag share one table: the second swaps it back.
#[test]
fn v33_probe_txe_shared_table_swaps_back() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(8, 64);
    let d = pattern(64, 8);
    let f1 = Box::leak(Box::new(head(12, 14, "AUDIT SHARED", 6, 1, 8, 32, 95, F_MONO, off.as_ptr(), d.as_ptr(), 64, 8)));
    let f0 = Box::leak(Box::new(head(12, 12, "AUDIT SHARED", 6, 1, 8, 32, 95, F_MONO, off.as_ptr(), d.as_ptr(), 64, 8)));
    f0.next_font = f1;
    assert_eq!(t.load(f0), 1);
    assert_eq!((d[0], d[1], f0.flags, f1.flags), (165, 185, 12, 12));
    t.end();
}

/// TXF1-5: the number of words the swap takes: a 16-bit product.
#[test]
fn v33_probe_txf_swap_count_is_a_16_bit_product() {
    let _g = lock();
    for (fw, fh, swapped, last) in [
        (255u16, 128u16, 16320i64, 32638i64),
        (256, 128, 16384, 32766),
        (400, 100, 20000, 39998),
        (512, 128, 0, -1),
        (700, 100, 2232, 4462),
    ] {
        let mut t = T::begin();
        let byte = |i: usize| (i.wrapping_mul(7).wrapping_add(i >> 8).wrapping_add(1)) as u8;
        let d = leak((0..70100usize).map(byte).collect::<Vec<u8>>());
        let off = off_linear(8, 63);
        let f = Box::leak(Box::new(head(11, 12, "AUDIT SWAP", 6, 1, 8, 32, 95, F_MONO, off.as_ptr(), d.as_ptr(), fw, fh)));
        assert_eq!(t.load(f), 1);
        let (mut cnt, mut l) = (0i64, -1i64);
        for i in (0..70099usize).step_by(2) {
            if d[i] == byte(i + 1) && d[i + 1] == byte(i) {
                cnt += 1;
                l = i as i64;
            }
        }
        assert_eq!((cnt, l), (swapped, last), "fw {fw} fh {fh}");
        assert_eq!(f.flags, 12);
        t.end();
    }
}

/// TXG: unload keeps `cur_font` and frees nothing; a second load works; unloading
/// without a chain is harmless.
#[test]
fn v33_probe_txg_unload_keeps_cur_font() {
    let _g = lock();
    let mut t = T::begin();
    let f = t13_font();
    assert_eq!(t.load(f), 1);
    assert_eq!(t.font(7), 7);
    t.unload();
    assert_eq!(t.attr(), (7, [5, 4, 5, 6]));
    t.gtext(20, 100, "AB@_");
    compare("TXG");
    assert_eq!(t.name(2).0, 1);
    assert_eq!(t.load(f), 1);
    assert_eq!(t.name(2).0, 7);
    t.unload();
    t.unload();
    assert_eq!(t.load(f), 1);
    t.end();
}

/// TXH: the guard per workstation, the ring global (R13).
#[test]
fn v33_probe_txh_two_workstations() {
    let _g = lock();
    let mut t = T::begin();
    let f = t13_font();
    let a = t.handle;
    assert_eq!(t.load(f), 1);
    assert_eq!(t.name(2).0, 7);
    t.intin[..10].fill(1);
    t.intin[10] = 2;
    t.vdi(100, 0, 0, 11);
    let b = t.contrl[6];
    assert_eq!(t.name(2).0, 1, "an open empties the ring");
    assert_eq!(t.load(f), 0, "and leaves the guard");
    assert_eq!(t.name(2).0, 1);
    t.handle = b;
    assert_eq!(t.load(f), 1, "B has its own guard");
    assert_eq!(t.name(2).0, 7);
    t.handle = a;
    assert_eq!(t.name(2).0, 7, "the ring is global");
    t.handle = b;
    t.unload();
    t.handle = a;
    assert_eq!(t.name(2).0, 1, "and B's unload empties it for A");
    assert_eq!(t.load(f), 0);
    t.unload();
    assert_eq!(t.load(f), 1);
    assert_eq!(t.name(2).0, 7);
    t.handle = b;
    t.vdi(101, 0, 0, 0);
    t.handle = a;
    assert_eq!(t.name(2).0, 7, "v_clsvwk leaves the ring");
    t.end();
}

/// TXI: a proportional font: widths from its table, `vqt_width`'s -1, the draw.
#[test]
fn v33_probe_txi_proportional() {
    let _g = lock();
    let mut t = T::prop_begin();
    assert_eq!(t.attr(), (20, [8, 8, 8, 12]));
    t.vdi(131, 0, 0, 0);
    assert_eq!((t.intout[0], t.intout[1]), (32, 95));
    assert_eq!(t.ptsout[..10], [8, 3, 0, 3, 0, 5, 0, 8, 0, 8]);
    assert_eq!(t.width(65), (65, 6, 0, 0));
    assert_eq!(t.width(32), (32, 3, 0, 0));
    assert_eq!(t.width(95), (95, 6, 0, 0));
    assert_eq!(t.width(31).0, -1);
    assert_eq!(t.ptsout[0], 0x7777, "out of range writes no width");
    assert_eq!(t.width(96).0, -1);
    assert_eq!(t.width(-1).0, -1);
    assert_eq!(t.extent("ABC @_"), [0, 0, 35, 0, 35, 12, 0, 12]);
    t.gtext(20, 100, "ABC @_ HELLO");
    compare("TXI");
    t.end();
}

/// A proportional font with one effect word: `vqt_extent` and the draw.
fn prop_effect(style: i16, extent: [i16; 8], grab: &str) {
    let mut t = T::prop_begin();
    t.i1(106, style);
    if extent != [0; 8] {
        assert_eq!(t.extent("ABC @_"), extent, "{grab}");
    }
    t.gtext(20, 100, "ABC @_ HELLO");
    compare(grab);
    t.end();
}

#[test]
fn v33_probe_txib_proportional_bold() {
    let _g = lock();
    prop_effect(1, [0, 0, 41, 0, 41, 12, 0, 12], "TXIb");
}

#[test]
fn v33_probe_txio_proportional_outline() {
    let _g = lock();
    prop_effect(16, [0, 0, 47, 0, 47, 14, 0, 14], "TXIo");
}

#[test]
fn v33_probe_txiu_proportional_underline() {
    let _g = lock();
    prop_effect(8, [0; 8], "TXIu");
}

/// TXIs: the extent only: the cell's background in the replace mode is the open
/// skew finding of the built-in fonts too (probe TXQ); TXQ3 draws in transparent mode.
#[test]
fn v33_probe_txq3_proportional_skew_transparent() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.i1(106, 4);
    assert_eq!(t.extent("ABC @_"), [0, 0, 40, 0, 40, 12, 0, 12]);
    t.i1(32, 2);
    t.gtext(20, 100, "ABC @_ HELLO");
    t.i1(32, 1);
    compare("TXQ3");
    t.end();
}

/// The built-in control for the widened skew cell: unchanged.
#[test]
fn v33_probe_txq2_system_skew_transparent() {
    let _g = lock();
    let mut t = T::begin();
    t.p1(12, 0, 13);
    t.i1(106, 4);
    t.i1(32, 2);
    t.gtext(20, 100, "ABC @_ HELLO");
    t.i1(32, 1);
    compare("TXQ2");
    t.treset();
}

#[test]
fn v33_probe_txr_proportional_rotated() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.i1(13, 900);
    assert_eq!(t.extent("ABC @_"), [12, 0, 12, 35, 0, 35, 0, 0]);
    t.gtext(100, 200, "ABC @_ HELLO");
    compare("TXR1");
    t.end();
    let mut t = T::prop_begin();
    t.i1(13, 1800);
    t.gtext(300, 100, "ABC @_ HELLO");
    compare("TXR2");
    t.end();
    let mut t = T::prop_begin();
    t.i1(13, 2700);
    t.gtext(100, 60, "ABC @_ HELLO");
    compare("TXR3");
    t.end();
}

#[test]
fn v33_probe_txs_proportional_scaled() {
    let _g = lock();
    let mut t = T::prop_begin();
    assert_eq!(t.height(16), [16, 17, 16, 24]);
    assert_eq!(t.extent("ABC @_"), [0, 0, 70, 0, 70, 24, 0, 24]);
    assert_eq!(t.width(65), (65, 12, 0, 0));
    t.gtext(20, 100, "ABC @_ HELLO");
    compare("TXS1");
    t.end();
    let mut t = T::prop_begin();
    assert_eq!(t.height(12), [12, 12, 12, 17]);
    assert_eq!(t.extent("ABC @_"), [0, 0, 52, 0, 52, 17, 0, 17]);
    assert_eq!(t.width(65), (65, 9, 0, 0));
    t.gtext(20, 100, "ABC @_ HELLO");
    compare("TXS2");
    t.end();
    let mut t = T::prop_begin();
    assert_eq!(t.height(5), [5, 5, 5, 8]);
    assert_eq!(t.extent("ABC @_"), [0, 0, 22, 0, 22, 8, 0, 8]);
    t.gtext(20, 100, "ABC @_ HELLO");
    compare("TXS3");
    t.end();
}

#[test]
fn v33_probe_txj_proportional_justified() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.justified(20, 100, 200, 1, 1, "ABC @_ HELLO");
    compare("TXJ");
    t.end();
}

/// TXO: codes below `first_ade`, above `last_ade`, negative: `'?'`.
#[test]
fn v33_probe_txo_out_of_range_codes() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.intin[..4].copy_from_slice(&[10, 200, 65, -5]);
    t.ptsin[0] = 20;
    t.ptsin[1] = 100;
    t.vdi(8, 0, 1, 4);
    compare("TXO");
    t.end();
}

#[test]
fn v33_probe_txm_writing_modes() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.i1(32, 2);
    t.gtext(20, 60, "ABC @_ HELLO");
    t.i1(32, 3);
    t.gtext(20, 80, "ABC @_ HELLO");
    t.i1(32, 4);
    t.gtext(20, 100, "ABC @_ HELLO");
    t.i1(32, 1);
    compare("TXM");
    t.end();
}

#[test]
fn v33_probe_txc2_alignment_and_clip() {
    let _g = lock();
    let mut t = T::prop_begin();
    t.i2(39, 2, 1);
    t.gtext(200, 60, "ABC @_ HELLO");
    t.i2(39, 1, 4);
    t.gtext(200, 90, "ABC @_ HELLO");
    t.i2(39, 0, 0);
    t.intin[0] = 1;
    t.ptsin[..4].copy_from_slice(&[40, 100, 90, 125]);
    t.vdi(129, 0, 2, 1);
    t.gtext(20, 120, "ABC @_ HELLO");
    t.intin[0] = 0;
    t.ptsin[..4].copy_from_slice(&[0, 0, 0, 0]);
    t.vdi(129, 0, 2, 1);
    compare("TXC2");
    t.end();
}

/// TXH2: `F_HORZ_OFF`: the pen takes `hor_table[k]`, `vqt_width` `[k*2]`, `[k*2+1]`.
#[test]
fn v33_probe_txh2_horizontal_offsets() {
    let _g = lock();
    let mut t = T::begin();
    assert_eq!(t.load(prop_font(F_STD | F_HORZ)), 1);
    t.i1(21, 20);
    t.p1(12, 0, 8);
    assert_eq!(t.width(65), (65, 6, 2, 3));
    assert_eq!(t.width(66), (66, 7, 0, 1));
    assert_eq!(t.width(95), (95, 6, 2, 3));
    assert_eq!(t.extent("ABC @_"), [0, 0, 35, 0, 35, 12, 0, 12]);
    t.gtext(20, 100, "ABC @_ HELLO");
    compare("TXH2");
    t.end();
}

/// TXK: a cell of 37 x 40, unaligned in its table, and doubled: past what a built-in
/// font ever needs.
#[test]
fn v33_probe_txk_big_cell() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(37, 64);
    let f = Box::leak(Box::new(head(13, 30, "AUDIT BIG", 30, 9, 37, 32, 95, F_STD | F_MONO, off.as_ptr(), pattern(296, 40).as_ptr(), 296, 40)));
    assert_eq!(t.load(f), 1);
    t.i1(21, 13);
    assert_eq!(t.height(30), [37, 30, 37, 40]);
    t.gtext(20, 100, "AB");
    compare("TXK");
    assert_eq!(t.height(60), [74, 61, 74, 80]);
    t.gtext(20, 300, "AB");
    compare("TXK2");
    t.end();
}

/// TXL: monospace 8x16 with all 256 codes: EmuTOS' direct blit, NewTOS' general path.
#[test]
fn v33_probe_txl_direct_blit_shape() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(8, 256);
    let f = Box::leak(Box::new(head(14, 10, "AUDIT DIRECT", 13, 2, 8, 0, 255, F_STD | F_MONO, off.as_ptr(), pattern(256, 16).as_ptr(), 256, 16)));
    assert_eq!(t.load(f), 1);
    t.i1(21, 14);
    assert_eq!(t.height(13), [8, 13, 8, 16]);
    t.gtext(16, 100, "AB@_ abc");
    t.i1(32, 2);
    t.gtext(16, 130, "AB@_ abc");
    t.i1(32, 1);
    compare("TXL");
    t.end();
}

/// TXN: the re-drive on the built-in faces, no chain: `vst_font` keeps a scaled size
/// and `pts_mode`.
#[test]
fn v33_probe_txn_redrive_on_system_faces() {
    let _g = lock();
    let mut t = T::begin();
    assert_eq!(t.point(20), (20, [14, 27, 16, 32]));
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [14, 27, 16, 32]));
    assert_eq!(t.font(99), 1);
    assert_eq!(t.attr(), (1, [14, 27, 16, 32]));
    t.gtext(20, 100, "AB@_");
    compare("TXN");
    assert_eq!(t.height(20), [11, 20, 12, 24]);
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [11, 20, 12, 24]));
    assert_eq!(t.height(3), [4, 3, 4, 5]);
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [4, 3, 4, 5]));
    assert_eq!(t.height(11), [13, 11, 15, 14]);
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [13, 11, 15, 14]));
    assert_eq!(t.point(9), (9, [7, 6, 8, 8]));
    assert_eq!(t.font(1), 1);
    assert_eq!(t.attr(), (1, [7, 6, 8, 8]));
    t.treset();
}

/// TXP: signed `char`; a name of 32 characters ends at the field.
#[test]
fn v33_probe_txp_name_bytes() {
    let _g = lock();
    let mut t = T::begin();
    let f = t13_font();
    f.name = [b'N'; 32];
    f.name[1] = 0xE4;
    assert_eq!(t.load(f), 1);
    t.intout[..40].fill(0x7777);
    t.i1(130, 2);
    assert_eq!(t.contrl[4], 33);
    assert_eq!(t.intout[0], 7);
    assert_eq!((t.intout[1], t.intout[2], t.intout[3], t.intout[32]), (78, -28, 78, 78));
    assert_eq!(t.intout[33], 0);
    assert_eq!(t.intout[34], 0x7777, "nothing behind the NUL");
    t.end();
}

/// D2: a NULL chain answers 0 and changes nothing (EmuTOS dereferences it): no chain
/// joins the ring, the one chance stays open, the current font stays.
#[test]
fn v33_d2_null_chain_answers_zero_and_changes_nothing() {
    let _g = lock();
    let mut t = T::begin();
    let before = t.attr();
    assert_eq!(t.load(core::ptr::null()), 0);
    assert_eq!(t.attr(), before);
    assert_eq!(t.name(2).0, 1);
    let f = t13_font();
    assert_eq!(t.load(f), 1, "the one chance is still there");
    assert_eq!(t.load(core::ptr::null()), 0, "guard first");
    assert_eq!(t.name(2).0, 7, "a NULL does not drop a loaded chain");
    t.end();
}

/// A current font on a face gone from the ring: EmuTOS reads low memory, NewTOS goes
/// on from `fon6x6` (`docs/COMPATIBILITY.md` "GDOS font chain").
#[test]
fn v33_face_gone_from_the_ring_falls_back_to_the_system_face() {
    let _g = lock();
    let mut t = T::begin();
    assert_eq!(t.load(t13_font()), 1);
    assert_eq!(t.font(7), 7);
    t.unload();
    assert_eq!(t.height(4), [5, 4, 6, 6], "6x6 at its own size");
    assert_eq!(t.attr().0, 1);
    t.end();
}

/// A font's metrics are the program's: a zero `top`, huge values and a glyph past the
/// working store must not take the kernel down (the dev profile checks overflow).
#[test]
fn v33_absurd_metrics_do_not_panic() {
    let _g = lock();
    let mut t = T::begin();
    let off = off_linear(600, 64);
    let f = Box::leak(Box::new(head(15, 30000, "AUDIT ABSURD", 0, 0, 600, 32, 95, F_STD | F_MONO, off.as_ptr(), pattern(4800, 600).as_ptr(), 4800, 600)));
    f.half = 60000;
    f.ascent = 60000;
    f.descent = 60000;
    f.thicken = 65000;
    f.left_offset = 40000;
    f.right_offset = 40000;
    f.ul_size = 3;
    assert_eq!(t.load(f), 1);
    t.i1(21, 15);
    for style in [0i16, 1, 4, 8, 16, 31] {
        t.i1(106, style);
        for v in 0..6 {
            t.i2(39, 2, v);
            for chup in [0i16, 900, 1800, 2700] {
                t.i1(13, chup);
                t.gtext(100, 100, "AB");
                t.justified(100, 100, 300, 1, 1, "A B");
                t.extent("AB");
                t.vdi(38, 0, 0, 0);
            }
        }
        t.height(0);
        t.height(i16::MAX);
        t.point(i16::MAX);
    }
    t.end();
}
