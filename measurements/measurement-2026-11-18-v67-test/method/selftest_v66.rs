//! TEMPORARY tester selftest for TASK/V66 (never committed to a branch).
//! Drives vq_scrninfo / vq_extnd / v_opnvwk through the real trap (svc #2) at whatever depth the
//! kernel booted with. The whole 273-word answer is printed as PROBE lines (diffed 16-plane vs main);
//! the structural statements S1-S7 of docs/api/vdi/eddi-vq_scrninfo-32.md are asserted from the
//! returned words, and the full 32-plane table is compared word by word with an independently
//! written expectation (the spec table of section 2).
use tos_api::vdi::backend::RasterBackend;

use crate::kprintln;
use crate::vdi_backend::VDI_SCREEN;
use crate::vdi_client::Mfdb;

struct V {
    contrl: [i16; 16],
    intin: [i16; 64],
    ptsin: [i16; 32],
    intout: [i16; 300],
    ptsout: [i16; 12],
    h: i16,
    fails: u32,
    checks: u32,
}

fn mf(addr: u64, w: i16, h: i16, wd: i16, stand: i16, np: i16) -> Mfdb {
    Mfdb { addr, w, h, wdwidth: wd, stand, nplanes: np, r1: 0, r2: 0, r3: 0 }
}

impl V {
    fn call(&mut self) {
        let block: [u64; 5] = [
            self.contrl.as_mut_ptr() as u64,
            self.intin.as_mut_ptr() as u64,
            self.ptsin.as_mut_ptr() as u64,
            self.intout.as_mut_ptr() as u64,
            self.ptsout.as_mut_ptr() as u64,
        ];
        let pb = block.as_ptr() as u64;
        // SAFETY: GEM trap, selector 115, parameter block in x1 (as vdi_client does).
        unsafe { core::arch::asm!("svc #2", inout("x0") 115u64 => _, in("x1") pb, options(nostack)) };
    }
    fn begin(&mut self, op: i16, np: i16, ni: i16) {
        self.contrl.fill(0);
        self.contrl[0] = op;
        self.contrl[1] = np;
        self.contrl[3] = ni;
        self.contrl[6] = self.h;
    }
    fn u64c(&mut self, idx: usize, val: u64) {
        let b = val.to_le_bytes();
        for i in 0..4 {
            self.contrl[idx + i] = i16::from_le_bytes([b[i * 2], b[i * 2 + 1]]);
        }
    }
    fn open(&mut self) {
        self.contrl.fill(0);
        self.contrl[0] = 100;
        self.contrl[3] = 11;
        self.intin[..11].copy_from_slice(&[1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 2]);
        self.call();
        self.h = self.contrl[6];
    }
    fn clear(&mut self) {
        self.begin(3, 0, 0);
        self.call();
    }
    fn set1(&mut self, op: i16, v: i16) {
        self.begin(op, 0, 1);
        self.intin[0] = v;
        self.call();
    }
    fn vs_color(&mut self, pen: i16, r: i16, g: i16, b: i16) {
        self.begin(14, 0, 4);
        self.intin[..4].copy_from_slice(&[pen, r, g, b]);
        self.call();
    }
    fn recfl(&mut self, x0: i16, y0: i16, x1: i16, y1: i16) {
        self.begin(114, 2, 0);
        self.ptsin[..4].copy_from_slice(&[x0, y0, x1, y1]);
        self.call();
    }
    fn gpx(&mut self, x: i16, y: i16) -> u32 {
        self.begin(105, 1, 0);
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        self.call();
        ((self.intout[0] as u16 as u32) << 16) | (self.intout[1] as u16 as u32)
    }
    fn vro(&mut self, s: &Mfdb, d: &Mfdb, sr: [i16; 4], dr: [i16; 4], op: i16) {
        self.begin(109, 4, 1);
        self.u64c(7, s as *const Mfdb as u64);
        self.u64c(11, d as *const Mfdb as u64);
        self.ptsin[..8].copy_from_slice(&[sr[0], sr[1], sr[2], sr[3], dr[0], dr[1], dr[2], dr[3]]);
        self.intin[0] = op;
        self.call();
    }
    fn chk(&mut self, name: &str, ok: bool, detail: u64) {
        self.checks += 1;
        if !ok {
            self.fails += 1;
        }
        kprintln!("[v66] CHK {} {} ({:#x})", name, if ok { "OK" } else { "FAIL" }, detail);
    }
    fn dump(&self, tag: &str, n: usize) {
        let mut i = 0;
        while i < n {
            let e = core::cmp::min(i + 16, n);
            kprintln!(
                "[v66] PROBE {} {:03}: {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x} {:04x}",
                tag, i,
                self.intout[i] as u16, self.intout[i + 1] as u16, self.intout[i + 2] as u16, self.intout[i + 3] as u16,
                self.intout[i + 4] as u16, self.intout[i + 5] as u16, self.intout[i + 6] as u16, self.intout[i + 7] as u16,
                self.intout[i + 8] as u16, self.intout[i + 9] as u16, self.intout[i + 10] as u16, self.intout[i + 11] as u16,
                self.intout[i + 12] as u16, self.intout[i + 13] as u16, self.intout[i + 14] as u16, self.intout[i + 15] as u16
            );
            let _ = e;
            i += 16;
        }
    }
    fn scrninfo(&mut self) {
        self.begin(102, 0, 1);
        self.contrl[5] = 1;
        self.intin[0] = 2;
        self.intout.fill(0x7E7E);
        self.ptsout.fill(0x6161);
        self.call();
    }
}

pub fn run() {
    let planes = VDI_SCREEN.planes();
    let wide = planes == 32;
    kprintln!("[v66] === V66 tester selftest: screen planes {} ===", planes);
    let mut v = V {
        contrl: [0; 16],
        intin: [0; 64],
        ptsin: [0; 32],
        intout: [0; 300],
        ptsout: [0; 12],
        h: 0,
        fails: 0,
        checks: 0,
    };
    v.open();
    if v.h == 0 {
        kprintln!("[v66] no workstation");
        return;
    }
    // work_out of v_opnvwk (45 words), kept for the consistency table
    let wo: [i16; 45] = core::array::from_fn(|i| v.intout[i]);
    v.dump("v_opnvwk work_out", 48);
    v.set1(32, 1);
    v.set1(23, 1);
    v.clear();

    // ---- vq_extnd mode 1 ----
    v.begin(102, 0, 1);
    v.intin[0] = 1;
    v.call();
    let ext1: [i16; 45] = core::array::from_fn(|i| v.intout[i]);
    v.dump("vq_extnd1", 48);
    v.begin(102, 0, 1);
    v.intin[0] = 0;
    v.call();
    let ext0: [i16; 45] = core::array::from_fn(|i| v.intout[i]);
    v.dump("vq_extnd0", 48);

    // ---- vq_scrninfo ----
    v.scrninfo();
    let t: [i16; 300] = v.intout;
    let pts_ok = v.ptsout.iter().all(|&x| x == 0x6161);
    let c4 = v.contrl[4];
    let c2 = v.contrl[2];
    kprintln!("[v66] PROBE scrninfo contrl[2]={} contrl[4]={} ptsout_untouched={} intout[273..]_untouched={}",
        c2, c4, v.ptsout.iter().all(|&x| x == 0x6161), t[273..].iter().all(|&x| x == 0x7E7E));
    v.dump("scrninfo", 272);
    kprintln!("[v66] PROBE scrninfo 272: {:04x}", t[272] as u16);

    // gate negatives: wrong INTIN[0] / wrong sub-op fall to vq_extnd
    v.begin(102, 0, 1);
    v.contrl[5] = 1;
    v.intin[0] = 1;
    v.call();
    kprintln!("[v66] PROBE subop1 intin0=1 contrl[4]={} intout[4]={}", v.contrl[4], v.intout[4]);

    // cookie
    let ck = tos_api::cookie::get(tos_api::cookie::EDDI);
    kprintln!("[v66] PROBE EdDI cookie {:?}", ck);

    // ---- structural statements, valid at both depths ----
    v.chk("S0 contrl[4]==272 contrl[2]==0", c4 == 272 && c2 == 0, c4 as u64);
    v.chk("S0 ptsout untouched, intout[273..] untouched", pts_ok && t[273..].iter().all(|&x| x == 0x7E7E), 0);
    v.chk("S0 intout[128..273] all zero", t[128..273].iter().all(|&x| x == 0), 0);
    v.chk("S1 T0=2 T1=2", t[0] == 2 && t[1] == 2, t[0] as u64);
    v.chk("S1 T2==planes", t[2] as i32 == planes as i32, t[2] as u64);
    v.chk("S1 T2==vq_extnd1 INTOUT4", t[2] == ext1[4], ext1[4] as u64);
    v.chk("S1 T2==work_out? (INQ_TAB[4] via ext1 only)", ext1[4] as i32 == planes as i32, 0);
    v.chk("S1 vq_extnd1 INTOUT5 (LUT)==0", ext1[5] == 0, ext1[5] as u64);
    v.chk("S1 work_out[13]==256 ncol and ext0[13]", wo[13] == 256 && ext0[13] == 256, wo[13] as u64);
    v.chk("S1 work_out[35]==1", wo[35] == 1, wo[35] as u64);
    v.chk("S1 work_out[39]==0", wo[39] == 0, wo[39] as u64);
    v.chk("S1 EdDI cookie present", ck == Some(1), 0);
    // S2
    let rng = [(8usize, 16usize, 32usize), (9, 32, 48), (10, 48, 64), (11, 64, 80), (12, 80, 96), (13, 96, 128)];
    let mut s2 = true;
    let mut seen = [0u8; 64];
    let mut s3dup = false;
    for &(ci, lo, hi) in &rng {
        let mut n = 0usize;
        let mut seen_neg = false;
        for k in lo..hi {
            if t[k] != -1 {
                if seen_neg {
                    s2 = false;
                }
                n += 1;
                let b = t[k];
                if (0..64).contains(&b) {
                    if seen[b as usize] != 0 {
                        s3dup = true;
                    }
                    seen[b as usize] = 1;
                } else {
                    s3dup = true;
                }
            } else {
                seen_neg = true;
            }
        }
        if n != t[ci] as usize {
            s2 = false;
        }
        kprintln!("[v66] PROBE S2 table@{} count {} word[{}]={}", lo, n, ci, t[ci]);
    }
    v.chk("S2 counts match, entries are leading", s2, 0);
    let pl = t[2] as usize;
    let part = (0..64).all(|b| (seen[b] != 0) == (b < pl));
    v.chk("S3 tables partition 0..planes, disjoint", part && !s3dup, 0);
    let cnt = ((t[3] as u16 as u32) << 16) | (t[4] as u16 as u32);
    let bits = (t[8] + t[9] + t[10]) as u32;
    v.chk("S5 colour count == 1<<(r+g+b)", cnt == 1u32 << bits, cnt as u64);
    kprintln!("[v66] PROBE S5 count {:#010x} bits {}", cnt, bits);
    let mut s4 = true;
    for (c, nb) in [(16usize, t[8]), (32, t[9]), (48, t[10])] {
        for k in 1..nb as usize {
            if t[c + k] != t[c] + k as i16 {
                s4 = false;
            }
        }
    }
    v.chk("S4 gun bits ascending by 1", s4, 0);

    if !wide {
        kprintln!("[v66] === done: checks {} fails {} (16 setting: probes + structural) ===", v.checks, v.fails);
        kprintln!("[v66] {}", if v.fails == 0 { "V66 SELFTEST PASS" } else { "V66 SELFTEST FAIL" });
        v.begin(101, 0, 0);
        v.call();
        return;
    }

    // ---- 32: word-by-word against the spec table (section 2), written independently ----
    let mut exp = [0i16; 273];
    let head: [i16; 16] = [2, 2, 32, 0x0100, 0, 0, 0, 0, 8, 8, 8, 0, 0, 8, 1, 0];
    exp[..16].copy_from_slice(&head);
    for i in 16..128 {
        exp[i] = -1;
    }
    for k in 0..8 {
        exp[16 + k] = 16 + k as i16;
        exp[32 + k] = 8 + k as i16;
        exp[48 + k] = k as i16;
        exp[96 + k] = 24 + k as i16;
    }
    let mut diff = 0u32;
    for i in 0..273 {
        if t[i] != exp[i] {
            diff += 1;
            kprintln!("[v66] DIFF word {} got {} want {}", i, t[i], exp[i]);
        }
    }
    v.chk("T32 273 words equal the spec table", diff == 0, diff as u64);
    // S6 / S7
    let rb = |k: usize| t[16 + k] as u32; // red bit numbers
    let gb = |k: usize| t[32 + k] as u32;
    let bb = |k: usize| t[48 + k] as u32;
    let build = |r: u32, g: u32, b: u32| -> u32 {
        let mut p = 0u32;
        for k in 0..t[8] as usize { p |= ((r >> k) & 1) << rb(k); }
        for k in 0..t[9] as usize { p |= ((g >> k) & 1) << gb(k); }
        for k in 0..t[10] as usize { p |= ((b >> k) & 1) << bb(k); }
        p
    };
    let mut mask = 0u32;
    for k in 0..8 { mask |= (1 << rb(k)) | (1 << gb(k)) | (1 << bb(k)); }
    v.chk("S6 mask == 0x00FFFFFF", mask == 0x00FF_FFFF, mask as u64);
    let mut trip: [(u32, u32, u32); 40] = [(0, 0, 0); 40];
    let mut n = 0;
    for tr in [(0, 0, 0), (255, 255, 255), (255, 0, 0), (0, 255, 0), (0, 0, 255), (0x12, 0x34, 0x56), (0xA5, 0x5A, 0xC3), (0xFE, 0xDC, 0xBA)] {
        trip[n] = tr;
        n += 1;
    }
    for k in 0..8 {
        trip[n] = (1 << k, 0, 0); n += 1;
        trip[n] = (0, 1 << k, 0); n += 1;
        trip[n] = (0, 0, 1 << k); n += 1;
    }
    let scr = mf(0, 0, 0, 0, 0, 0);
    let mut bad6 = 0u32;
    let mut bad6x = 0u32;
    let mut bad7 = 0u32;
    let mut bad7r = 0u32;
    for (i, &(r, g, b)) in trip[..n].iter().enumerate() {
        let p = build(r, g, b);
        // S6: pixel (with X set to 0x5A to prove the mask) -> screen -> v_get_pixel
        for (xv, bad) in [(0u32, &mut bad6), (0x5A00_0000u32, &mut bad6x)] {
            let word = [p | xv; 16];
            let src = mf(word.as_ptr() as u64, 1, 1, 1, 0, t[2]);
            v.vro(&src, &scr, [0, 0, 0, 0], [500 + i as i16, 300, 500 + i as i16, 300], 3);
            let g2 = v.gpx(500 + i as i16, 300);
            if g2 != p & mask { *bad += 1; kprintln!("[v66] S6 mismatch trip {} x {:#x} got {:#x} want {:#x}", i, xv, g2, p & mask); }
        }
        // S7: pen drawn with vs_color (DOOM rule), read back v_get_pixel, and screen->memory
        let c = |q: u32| ((q * 1000 + 127) / 255) as i16;
        v.vs_color(9, c(r), c(g), c(b));
        v.set1(25, 9);
        v.recfl(500 + i as i16, 320, 500 + i as i16, 320);
        let g3 = v.gpx(500 + i as i16, 320);
        if g3 != p & mask { bad7 += 1; kprintln!("[v66] S7 pen mismatch trip {} got {:#x} want {:#x}", i, g3, p & mask); }
        let mut outw = [0xEEEE_EEEEu32; 16];
        let dst = mf(outw.as_mut_ptr() as u64, 1, 1, 1, 0, t[2]);
        v.vro(&scr, &dst, [500 + i as i16, 320, 500 + i as i16, 320], [0, 0, 0, 0], 3);
        if outw[0] & mask != p & mask { bad7r += 1; }
    }
    kprintln!("[v66] PROBE S6/S7 triples {}", n);
    v.chk("S6 table-built pixel -> vro_cpyfm -> v_get_pixel (X=0)", bad6 == 0, bad6 as u64);
    v.chk("S6 same with X byte 0x5A set (masked on read)", bad6x == 0, bad6x as u64);
    v.chk("S7 pen via vs_color/vr_recfl -> v_get_pixel", bad7 == 0, bad7 as u64);
    v.chk("S7 pen -> vro_cpyfm screen->mem (masked)", bad7r == 0, bad7r as u64);
    v.set1(25, 1);
    v.clear();
    v.begin(101, 0, 0);
    v.call();
    kprintln!("[v66] === done: checks {} fails {} ===", v.checks, v.fails);
    kprintln!("[v66] {}", if v.fails == 0 { "V66 SELFTEST PASS" } else { "V66 SELFTEST FAIL" });
}
