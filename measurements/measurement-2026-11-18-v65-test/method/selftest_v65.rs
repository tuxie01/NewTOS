//! TEMPORARY tester selftest for TASK/V65 (never committed to a branch).
//! Drives the VDI through the real trap (svc #2) at whatever depth the kernel booted with.
//! At 32 planes it asserts the V65 contract; at 16 planes it only prints PROBE lines (the same
//! probes run on main, and the two logs are diffed).
use tos_api::vdi::backend::RasterBackend;

use crate::kprintln;
use crate::vdi_backend::VDI_SCREEN;
use crate::vdi_client::Mfdb;

struct V {
    contrl: [i16; 16],
    intin: [i16; 64],
    ptsin: [i16; 32],
    intout: [i16; 45],
    ptsout: [i16; 12],
    h: i16,
    fails: u32,
    checks: u32,
    wide: bool,
}

fn mf(addr: u64, w: i16, h: i16, wd: i16, stand: i16, np: i16) -> Mfdb {
    Mfdb { addr, w, h, wdwidth: wd, stand, nplanes: np, r1: 0, r2: 0, r3: 0 }
}

fn opf(op: u8, s: u32, d: u32) -> u32 {
    match op {
        0 => 0,
        1 => s & d,
        2 => s & !d,
        3 => s,
        4 => !s & d,
        5 => d,
        6 => s ^ d,
        7 => s | d,
        8 => !(s | d),
        9 => !(s ^ d),
        10 => !d,
        11 => s | !d,
        12 => !s,
        13 => !s | d,
        14 => !(s & d),
        _ => !0,
    }
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
    fn gp(&mut self, x: i16, y: i16) -> (i16, i16) {
        self.begin(105, 1, 0);
        self.ptsin[0] = x;
        self.ptsin[1] = y;
        self.intout[0] = 0x5151;
        self.intout[1] = 0x5151;
        self.call();
        (self.intout[0], self.intout[1])
    }
    fn gpx(&mut self, x: i16, y: i16) -> u32 {
        let (a, b) = self.gp(x, y);
        ((a as u16 as u32) << 16) | (b as u16 as u32)
    }
    fn vro(&mut self, s: &Mfdb, d: &Mfdb, sr: [i16; 4], dr: [i16; 4], op: i16) {
        self.begin(109, 4, 1);
        self.u64c(7, s as *const Mfdb as u64);
        self.u64c(11, d as *const Mfdb as u64);
        self.ptsin[..8].copy_from_slice(&[sr[0], sr[1], sr[2], sr[3], dr[0], dr[1], dr[2], dr[3]]);
        self.intin[0] = op;
        self.call();
    }
    fn vrt(&mut self, s: &Mfdb, d: &Mfdb, sr: [i16; 4], dr: [i16; 4], mode: i16, fg: i16, bg: i16) {
        self.begin(121, 4, 3);
        self.u64c(7, s as *const Mfdb as u64);
        self.u64c(11, d as *const Mfdb as u64);
        self.ptsin[..8].copy_from_slice(&[sr[0], sr[1], sr[2], sr[3], dr[0], dr[1], dr[2], dr[3]]);
        self.intin[..3].copy_from_slice(&[mode, fg, bg]);
        self.call();
    }
    fn trn(&mut self, s: &Mfdb, d: &Mfdb) {
        self.begin(110, 0, 0);
        self.u64c(7, s as *const Mfdb as u64);
        self.u64c(11, d as *const Mfdb as u64);
        self.call();
    }
    fn chk(&mut self, name: &str, ok: bool, detail: u64) {
        self.checks += 1;
        if !ok {
            self.fails += 1;
        }
        kprintln!("[v65] CHK {} {} ({:#x})", name, if ok { "OK" } else { "FAIL" }, detail);
    }
    /// 32: assert; 16: print the probe only.
    fn eq32(&mut self, name: &str, got: u32, want: u32) {
        if self.wide {
            self.chk(name, got == want, got as u64);
        } else {
            kprintln!("[v65] PROBE {} {:#010x}", name, got);
        }
    }
    fn screen_fill_form(&mut self, buf: &mut [u32], w: usize, stride: usize, h: usize, f: &dyn Fn(usize, usize) -> u32) {
        for y in 0..h {
            for x in 0..w {
                buf[y * stride + x] = f(x, y);
            }
        }
    }
}

fn pat(x: usize, y: usize) -> u32 {
    0x5A00_0000 | (((x * 11 + 1) as u32) << 16) | (((y * 19 + 3) as u32) << 8) | (((x * 7 + y * 5) & 0xFF) as u32)
}

pub fn run() {
    let planes = VDI_SCREEN.planes();
    let wide = planes == 32;
    kprintln!("[v65] === V65 tester selftest: screen planes {} ===", planes);
    let mut v = V {
        contrl: [0; 16],
        intin: [0; 64],
        ptsin: [0; 32],
        intout: [0; 45],
        ptsout: [0; 12],
        h: 0,
        fails: 0,
        checks: 0,
        wide,
    };
    v.open();
    if v.h == 0 {
        kprintln!("[v65] no workstation");
        return;
    }
    v.set1(32, 1); // replace mode
    v.set1(23, 1); // solid fill
    v.clear();

    // ---- A: v_get_pixel ----
    let g = v.gpx(5, 5);
    v.eq32("A cleared(5,5)", g, 0x00FF_FFFF);
    if !wide {
        kprintln!("[v65] PROBE A cleared words {:#x}", g);
    }
    for (pen, want) in [(0i16, 0x00FF_FFFFu32), (1, 0), (2, 0x00FF_0000), (3, 0x0000_FF00), (4, 0x0000_00FF)] {
        v.set1(25, pen);
        v.recfl(300, 300, 309, 309);
        let g = v.gpx(305, 305);
        v.eq32(["A pen0", "A pen1", "A pen2", "A pen3", "A pen4"][pen as usize], g, want);
        let (a, b) = v.gp(305, 305);
        kprintln!("[v65] PROBE A pen {} intout {:#06x} {:#06x}", pen, a as u16, b as u16);
    }
    v.vs_color(5, 17, 0, 0);
    v.vs_color(6, 25, 0, 0);
    for (pen, want) in [(5i16, 0x0004_0000u32), (6, 0x0006_0000)] {
        v.set1(25, pen);
        v.recfl(300, 300, 309, 309);
        let g = v.gpx(305, 305);
        v.eq32(if pen == 5 { "A testpen17" } else { "A testpen25" }, g, want);
        let (a, b) = v.gp(305, 305);
        kprintln!("[v65] PROBE A testpen {} intout {:#06x} {:#06x}", pen, a as u16, b as u16);
    }
    let off = v.gp(-5, -5);
    kprintln!("[v65] PROBE A offscreen {:#06x} {:#06x}", off.0 as u16, off.1 as u16);
    // restore pens 5, 6 to defaults is not needed (boot goes on, fresh handle)

    if wide {
        run_wide(&mut v);
    } else {
        run_narrow(&mut v);
    }
    // ---- vr_trnfm: both settings (16 setting: 16-plane identity, probe) ----
    run_trn(&mut v);
    v.set1(25, 1);
    v.clear();
    v.begin(101, 0, 0);
    v.call();
    kprintln!("[v65] === done: checks {} fails {} ===", v.checks, v.fails);
    if wide {
        kprintln!("[v65] {}", if v.fails == 0 { "V65 SELFTEST PASS" } else { "V65 SELFTEST FAIL" });
    } else {
        kprintln!("[v65] 16-plane probes done");
    }
}

fn run_wide(v: &mut V) {
    const W: usize = 20;
    const H: usize = 12;
    const STR: usize = 32; // wdwidth 2
    let mut m1 = [0xEEEE_EEEEu32; STR * H];
    v.screen_fill_form(&mut m1, W, STR, H, &pat);
    let f1 = mf(m1.as_ptr() as u64, W as i16, H as i16, 2, 0, 32);
    let scr = mf(0, 0, 0, 0, 0, 0);
    // B1 memory -> screen replace
    v.clear();
    v.vro(&f1, &scr, [0, 0, 19, 11], [200, 200, 219, 211], 3);
    let mut bad = 0;
    for y in 0..H {
        for x in 0..W {
            if v.gpx(200 + x as i16, 200 + y as i16) != pat(x, y) & 0x00FF_FFFF {
                bad += 1;
            }
        }
    }
    v.chk("B1 mem->screen 240 px", bad == 0, bad);
    // B2 screen -> memory
    let mut m2 = [0xEEEE_EEEEu32; STR * H];
    let f2 = mf(m2.as_mut_ptr() as u64, W as i16, H as i16, 2, 0, 32);
    v.vro(&scr, &f2, [200, 200, 219, 211], [0, 0, 19, 11], 3);
    let mut bad = 0;
    let mut padbad = 0;
    for y in 0..H {
        for x in 0..STR {
            let g = m2[y * STR + x];
            if x < W {
                if g & 0x00FF_FFFF != pat(x, y) & 0x00FF_FFFF {
                    bad += 1;
                }
            } else if g != 0xEEEE_EEEE {
                padbad += 1;
            }
        }
    }
    kprintln!("[v65] PROBE B2 first word {:#010x} (X byte not asserted)", m2[0]);
    v.chk("B2 screen->mem 240 px (masked)", bad == 0, bad);
    v.chk("B2 padding untouched", padbad == 0, padbad);
    // B3 mem->mem exact incl X
    let mut m3 = [0xEEEE_EEEEu32; STR * H];
    let f3 = mf(m3.as_mut_ptr() as u64, W as i16, H as i16, 2, 0, 32);
    v.vro(&f1, &f3, [0, 0, 19, 11], [0, 0, 19, 11], 3);
    let mut bad = 0;
    for y in 0..H {
        for x in 0..W {
            if m3[y * STR + x] != pat(x, y) {
                bad += 1;
            }
        }
    }
    v.chk("B3 mem->mem exact 32 bit", bad == 0, bad);
    // B4 screen -> screen
    v.vro(&scr, &scr, [200, 200, 219, 211], [300, 200, 319, 211], 3);
    let mut bad = 0;
    for y in 0..H {
        for x in 0..W {
            if v.gpx(300 + x as i16, 200 + y as i16) != pat(x, y) & 0x00FF_FFFF {
                bad += 1;
            }
        }
    }
    v.chk("B4 screen->screen 240 px", bad == 0, bad);
    // B4b overlapping screen->screen (shift right by 5, both directions)
    v.vro(&scr, &scr, [200, 200, 219, 211], [205, 203, 224, 214], 3);
    let mut bad = 0;
    for y in 0..H {
        for x in 0..W {
            if v.gpx(205 + x as i16, 203 + y as i16) != pat(x, y) & 0x00FF_FFFF {
                bad += 1;
            }
        }
    }
    v.chk("B4b overlapping screen->screen", bad == 0, bad);
    // B5 logic ops, 4 pixels
    let sp = [0x00F0_F0F0u32, 0x0012_CCAA, 0x00FF_FFFF, 0x0000_0000];
    let dp = [0x00CC_CCCCu32, 0x00A5_A5A5, 0x0012_3456, 0x00FE_DCBA];
    let mut sm = [0u32; 32];
    let mut dm = [0u32; 32];
    for i in 0..4 {
        sm[i] = sp[i];
        dm[i] = dp[i];
    }
    let fs = mf(sm.as_ptr() as u64, 4, 1, 1, 0, 32);
    let fd = mf(dm.as_ptr() as u64, 4, 1, 1, 0, 32);
    let mut bad_ms = 0;
    let mut bad_ss = 0;
    let mut bad_sm = 0;
    for op in 0u8..16 {
        // mem -> screen
        v.vro(&fd, &scr, [0, 0, 3, 0], [400, 200, 403, 200], 3);
        v.vro(&fs, &scr, [0, 0, 3, 0], [400, 200, 403, 200], op as i16);
        for i in 0..4 {
            if v.gpx(400 + i as i16, 200) != opf(op, sp[i], dp[i]) & 0x00FF_FFFF {
                bad_ms += 1;
            }
        }
        // screen -> screen: S at (420..), D at (400..)
        v.vro(&fs, &scr, [0, 0, 3, 0], [420, 200, 423, 200], 3);
        v.vro(&fd, &scr, [0, 0, 3, 0], [400, 202, 403, 202], 3);
        v.vro(&scr, &scr, [420, 200, 423, 200], [400, 202, 403, 202], op as i16);
        for i in 0..4 {
            if v.gpx(400 + i as i16, 202) != opf(op, sp[i], dp[i]) & 0x00FF_FFFF {
                bad_ss += 1;
            }
        }
        // screen -> mem: dest buffer holds D, source screen S at 420..
        let mut dm2 = dm;
        let fd2 = mf(dm2.as_mut_ptr() as u64, 4, 1, 1, 0, 32);
        v.vro(&scr, &fd2, [420, 200, 423, 200], [0, 0, 3, 0], op as i16);
        for i in 0..4 {
            if dm2[i] & 0x00FF_FFFF != opf(op, sp[i], dp[i]) & 0x00FF_FFFF {
                bad_sm += 1;
            }
        }
    }
    v.chk("B5 16 ops mem->screen", bad_ms == 0, bad_ms);
    v.chk("B5 16 ops screen->screen", bad_ss == 0, bad_ss);
    v.chk("B5 16 ops screen->mem", bad_sm == 0, bad_sm);
    // B5b mem -> mem all 16 ops exact (X included)
    let mut bad = 0;
    for op in 0u8..16 {
        let mut dm3 = [0u32; 32];
        for i in 0..4 {
            dm3[i] = dp[i] | 0x7E00_0000;
        }
        let fd3 = mf(dm3.as_mut_ptr() as u64, 4, 1, 1, 0, 32);
        v.vro(&fs, &fd3, [0, 0, 3, 0], [0, 0, 3, 0], op as i16);
        for i in 0..4 {
            if dm3[i] != opf(op, sp[i], dp[i] | 0x7E00_0000) {
                bad += 1;
            }
        }
    }
    v.chk("B5b 16 ops mem->mem exact", bad == 0, bad);
    // B6 refused plane counts at the 32 setting: silent no-op
    for np in [16i16, 9, 24, 31, 33] {
        let mut src = [0xABCDu16; 128];
        let mut dst16 = [0x1357u16; 128];
        let fsrc = mf(src.as_mut_ptr() as u64, 16, 4, 1, 0, np);
        let fdst = mf(dst16.as_mut_ptr() as u64, 16, 4, 1, 0, np);
        // screen pre-filled red
        v.set1(25, 2);
        v.recfl(500, 300, 515, 303);
        let before = v.gpx(505, 301);
        v.vro(&fsrc, &scr, [0, 0, 15, 3], [500, 300, 515, 303], 3);
        let after = v.gpx(505, 301);
        v.chk(match np { 16 => "B6 np16 mem->screen refused", 9 => "B6 np9 mem->screen refused", 24 => "B6 np24 mem->screen refused", 31 => "B6 np31 mem->screen refused", _ => "B6 np33 mem->screen refused" }, before == after && after == 0x00FF_0000, after as u64);
        v.vro(&scr, &fdst, [500, 300, 515, 303], [0, 0, 15, 3], 3);
        let unchanged = dst16.iter().all(|&w| w == 0x1357);
        v.chk(match np { 16 => "B6 np16 screen->mem refused", 9 => "B6 np9 screen->mem refused", 24 => "B6 np24 screen->mem refused", 31 => "B6 np31 screen->mem refused", _ => "B6 np33 screen->mem refused" }, unchanged, 0);
        let mut dst2 = [0x2468u16; 128];
        let fdst2 = mf(dst2.as_mut_ptr() as u64, 16, 4, 1, 0, np);
        v.vro(&fsrc, &fdst2, [0, 0, 15, 3], [0, 0, 15, 3], 3);
        let unchanged = dst2.iter().all(|&w| w == 0x2468);
        v.chk(match np { 16 => "B6 np16 mem->mem refused", 9 => "B6 np9 mem->mem refused", 24 => "B6 np24 mem->mem refused", 31 => "B6 np31 mem->mem refused", _ => "B6 np33 mem->mem refused" }, unchanged, 0);
        // vrt_cpyfm into a refused form
        let mono = [0xF0F0u16; 4];
        let fm = mf(mono.as_ptr() as u64, 16, 4, 1, 0, 1);
        let mut dst3 = [0x3579u16; 128];
        let fdst3 = mf(dst3.as_mut_ptr() as u64, 16, 4, 1, 0, np);
        v.vrt(&fm, &fdst3, [0, 0, 15, 3], [0, 0, 15, 3], 1, 2, 4);
        let unchanged = dst3.iter().all(|&w| w == 0x3579);
        v.chk(match np { 16 => "B6 np16 vrt mem refused", 9 => "B6 np9 vrt mem refused", 24 => "B6 np24 vrt mem refused", 31 => "B6 np31 vrt mem refused", _ => "B6 np33 vrt mem refused" }, unchanged, 0);
    }
    // C: vrt_cpyfm
    let mono = [0xF0F0u16, 0xF0F0, 0xF0F0, 0xF0F0];
    let fm = mf(mono.as_ptr() as u64, 16, 4, 1, 0, 1);
    for mode in 1i16..=4 {
        let mut dd = [0x7E12_3456u32; 64];
        let fdd = mf(dd.as_mut_ptr() as u64, 16, 4, 1, 0, 32);
        v.vrt(&fm, &fdd, [0, 0, 15, 3], [0, 0, 15, 3], mode, 2, 4);
        let d0 = 0x7E12_3456u32;
        let (w1, w0): (u32, u32) = match mode {
            1 => (0x00FF_0000, 0x0000_00FF),
            2 => (0x00FF_0000, d0),
            3 => (!d0, d0),
            _ => (d0, 0x0000_00FF),
        };
        // bit set at x=0..3, clear at x=4..7 (0xF0F0 high nibble set) ; x=8..11 set again
        let ok = dd[0] == w1 && dd[4] == w0 && dd[8] == w1 && dd[12] == w0 && dd[16 + 1] == w1 && dd[16 + 5] == w0;
        kprintln!("[v65] PROBE C vrt mem mode {} px0 {:#010x} px4 {:#010x}", mode, dd[0], dd[4]);
        v.chk(match mode { 1 => "C vrt mem mode1 replace", 2 => "C vrt mem mode2 trans", 3 => "C vrt mem mode3 xor", _ => "C vrt mem mode4 erase" }, ok, dd[0] as u64);
        // screen destination
        let dm1 = [0x0012_3456u32; 16];
        let fdm = mf(dm1.as_ptr() as u64, 16, 1, 1, 0, 32);
        v.vro(&fdm, &scr, [0, 0, 15, 0], [100, 100, 115, 100], 3);
        v.vrt(&fm, &scr, [0, 0, 15, 0], [100, 100, 115, 100], mode, 2, 4);
        let d = 0x0012_3456u32;
        let (s1, s0): (u32, u32) = match mode {
            1 => (0x00FF_0000, 0x0000_00FF),
            2 => (0x00FF_0000, d),
            3 => (!d & 0x00FF_FFFF, d),
            _ => (d, 0x0000_00FF),
        };
        let ok = v.gpx(100, 100) == s1 && v.gpx(104, 100) == s0;
        v.chk(match mode { 1 => "C vrt screen mode1", 2 => "C vrt screen mode2", 3 => "C vrt screen mode3", _ => "C vrt screen mode4" }, ok, 0);
    }
    let mut dd = [0x7E12_3456u32; 64];
    let fdd = mf(dd.as_mut_ptr() as u64, 16, 4, 1, 0, 32);
    v.vrt(&fm, &fdd, [0, 0, 15, 3], [0, 0, 15, 3], 5, 2, 4);
    v.vrt(&fm, &fdd, [0, 0, 15, 3], [0, 0, 15, 3], 0, 2, 4);
    v.chk("C vrt mode 0 and 5 no-op", dd.iter().all(|&w| w == 0x7E12_3456), 0);
    // second vrt with a non-pen-0..: test pens 17/25 pixel in memory
    let mut dd = [0u32; 64];
    let fdd = mf(dd.as_mut_ptr() as u64, 16, 4, 1, 0, 32);
    v.vrt(&fm, &fdd, [0, 0, 15, 3], [0, 0, 15, 3], 1, 5, 6);
    v.chk("C vrt pen5 (17,0,0) fg and pen6 (25,0,0) bg 32-bit", dd[0] == 0x0004_0000 && dd[4] == 0x0006_0000, dd[0] as u64);
}

fn run_narrow(v: &mut V) {
    // 16-plane setting: the 16-plane contract, printed for the diff against main.
    let scr = mf(0, 0, 0, 0, 0, 0);
    let mut m = [0xEEEEu16; 16 * 4 * 2];
    for i in 0..(16 * 4) {
        m[i] = (i as u16).wrapping_mul(0x0777) ^ 0xA5A5;
    }
    let f = mf(m.as_ptr() as u64, 16, 4, 1, 0, 16);
    v.clear();
    v.vro(&f, &scr, [0, 0, 15, 3], [200, 200, 215, 203], 3);
    let mut sum = 0u32;
    for y in 0..4 {
        for x in 0..16 {
            let g = v.gp(200 + x, 200 + y);
            sum = sum.wrapping_mul(31).wrapping_add((g.1 as u16 as u32) ^ ((g.0 as u16 as u32) << 16));
        }
    }
    kprintln!("[v65] PROBE N mem16->screen readback hash {:#010x}", sum);
    let mut d = [0x1111u16; 16 * 4];
    let fd = mf(d.as_mut_ptr() as u64, 16, 4, 1, 0, 16);
    v.vro(&scr, &fd, [200, 200, 215, 203], [0, 0, 15, 3], 3);
    let mut h = 0u32;
    for w in d {
        h = h.wrapping_mul(31).wrapping_add(w as u32);
    }
    kprintln!("[v65] PROBE N screen->mem16 hash {:#010x}", h);
    // ops
    for op in [1i16, 6, 7, 12, 10] {
        v.vro(&f, &scr, [0, 0, 15, 3], [300, 200, 315, 203], 3);
        v.vro(&f, &scr, [0, 0, 15, 3], [300, 200, 315, 203], op);
        let g = v.gp(303, 201);
        kprintln!("[v65] PROBE N op {} px {:#06x} {:#06x}", op, g.0 as u16, g.1 as u16);
    }
    // vrt into a 16-plane mem form
    let mono = [0xF0F0u16; 4];
    let fm = mf(mono.as_ptr() as u64, 16, 4, 1, 0, 1);
    for mode in 1i16..=4 {
        let mut dd = [0x1234u16; 64];
        let fdd = mf(dd.as_mut_ptr() as u64, 16, 4, 1, 0, 16);
        v.vrt(&fm, &fdd, [0, 0, 15, 3], [0, 0, 15, 3], mode, 2, 4);
        kprintln!("[v65] PROBE N vrt mem16 mode {} {:#06x} {:#06x}", mode, dd[0], dd[4]);
    }
    // a 32-plane form at the 16 setting: refused
    let mut m32 = [0x5555_5555u32; 64];
    let f32m = mf(m32.as_mut_ptr() as u64, 16, 4, 1, 0, 32);
    v.vro(&scr, &f32m, [200, 200, 215, 203], [0, 0, 15, 3], 3);
    kprintln!("[v65] PROBE N 32-plane form at 16 setting untouched: {}", m32.iter().all(|&w| w == 0x5555_5555));
}

fn run_trn(v: &mut V) {
    let wide = v.wide;
    let np: i16 = if wide { 32 } else { 16 };
    // device form 32 x 4, wdwidth 2 -> planesize 8 words
    let (w, h, wd) = (32usize, 4usize, 2usize);
    let n = w * h; // 128 pixels
    let mut seed = 0x1234_5678u32;
    let mut devpx = [0u32; 128];
    for p in devpx.iter_mut() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        *p = if wide { seed } else { seed & 0xFFFF };
    }
    let mut std_w = [0xEEEEu16; 128 * 2];
    let mut back = [0xEEEEu32; 128];
    let mut dev16 = [0u16; 128];
    if wide {
        back = devpx;
    } else {
        for i in 0..128 {
            dev16[i] = devpx[i] as u16;
        }
    }
    let src_dev32 = devpx;
    let fs = mf(if wide { src_dev32.as_ptr() as u64 } else { dev16.as_ptr() as u64 }, w as i16, h as i16, wd as i16, 0, np);
    let fstd = mf(std_w.as_mut_ptr() as u64, w as i16, h as i16, wd as i16, 1, np);
    v.trn(&fs, &fstd); // device -> standard
    let planesize = h * wd;
    if wide {
        // check plane/bit layout: plane p holds bit 31-p; pixel k = bit 0x8000>>(k&15) of word k/16
        let mut bad = 0;
        for k in 0..n {
            for p in 0..32usize {
                let word = std_w[p * planesize + k / 16];
                let bit = (word >> (15 - (k & 15))) & 1;
                let want = (src_dev32[k] >> (31 - p)) & 1;
                if bit as u32 != want {
                    bad += 1;
                }
            }
        }
        v.chk("D vr_trnfm device->standard layout (4096 bits)", bad == 0, bad);
    } else {
        let mut h2 = 0u32;
        for w2 in std_w.iter().take(planesize * 16) {
            h2 = h2.wrapping_mul(31).wrapping_add(*w2 as u32);
        }
        kprintln!("[v65] PROBE T std16 hash {:#010x}", h2);
    }
    // back: standard -> device; the standard form's stand=1 as source
    let mut out32 = [0xEEEE_EEEEu32; 128];
    let mut out16 = [0xEEEEu16; 128];
    let fstd_src = mf(std_w.as_ptr() as u64, w as i16, h as i16, wd as i16, 1, np);
    let fdev = mf(if wide { out32.as_mut_ptr() as u64 } else { out16.as_mut_ptr() as u64 }, w as i16, h as i16, wd as i16, 0, np);
    v.trn(&fstd_src, &fdev);
    if wide {
        v.chk("D vr_trnfm round trip identity (bit for bit, X incl.)", out32 == src_dev32, out32[0] as u64);
        // in place: standard -> device in the same buffer
        let mut inplace = [0xEEEEu16; 256];
        inplace[..256].copy_from_slice(&std_w);
        let fi_src = mf(inplace.as_ptr() as u64, w as i16, h as i16, wd as i16, 1, np);
        let fi_dst = mf(inplace.as_ptr() as u64, w as i16, h as i16, wd as i16, 0, np);
        v.trn(&fi_src, &fi_dst);
        let mut ok = true;
        for k in 0..n {
            let lo = inplace[2 * k] as u32;
            let hi = inplace[2 * k + 1] as u32;
            let g = lo | (hi << 16);
            if g != src_dev32[k] {
                ok = false;
            }
        }
        v.chk("D vr_trnfm in place standard->device identity", ok, 0);
        // standard-first direction: random standard -> device -> standard
        let mut stdr = [0u16; 256];
        for x in stdr.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *x = (seed >> 8) as u16;
        }
        let mut devo = [0xEEEE_EEEEu32; 128];
        let mut stdo = [0xEEEEu16; 256];
        let a = mf(stdr.as_ptr() as u64, w as i16, h as i16, wd as i16, 1, 32);
        let b = mf(devo.as_mut_ptr() as u64, w as i16, h as i16, wd as i16, 0, 32);
        v.trn(&a, &b);
        let b2 = mf(devo.as_ptr() as u64, w as i16, h as i16, wd as i16, 0, 32);
        let c = mf(stdo.as_mut_ptr() as u64, w as i16, h as i16, wd as i16, 1, 32);
        v.trn(&b2, &c);
        v.chk("D vr_trnfm standard->device->standard identity", stdo == stdr, stdo[0] as u64);
        // refused counts: 16 and 24 : silent no-op, destination and its fd_stand untouched
        for np2 in [16i16, 24, 9] {
            let srcw = [0x1111u16; 256];
            let mut dstw = [0x2222u16; 256];
            let a = mf(srcw.as_ptr() as u64, w as i16, h as i16, wd as i16, 0, np2);
            let b = mf(dstw.as_mut_ptr() as u64, w as i16, h as i16, wd as i16, 7, np2);
            v.trn(&a, &b);
            let ok = dstw.iter().all(|&x| x == 0x2222) && unsafe { core::ptr::read_volatile(&b.stand) } == 7;
            v.chk(match np2 { 16 => "D vr_trnfm np16 refused", 24 => "D vr_trnfm np24 refused", _ => "D vr_trnfm np9 refused" }, ok, 0);
        }
    } else {
        let mut h2 = 0u32;
        for x in out16 {
            h2 = h2.wrapping_mul(31).wrapping_add(x as u32);
        }
        let same = (0..128).all(|i| out16[i] == dev16[i]);
        kprintln!("[v65] PROBE T round trip16 identity {} hash {:#010x}", same, h2);
        let _ = back;
    }
}
