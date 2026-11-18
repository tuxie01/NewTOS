//! V62 — the VDI depth as a boot-time value: what `v_opnwk`, `v_opnvwk`,
//! `vq_extnd` and `appl_init` answer at the default 16 planes and at the
//! 32-plane setting (`vdiplanes=32`).
//!
//! Spec: `docs/api/vdi/workstation/100-v_opnvwk.md`, section "32-plane
//! setting and the boot argument `vdiplanes`" §3 (the value table) and §4
//! (the EdDI gate), `docs/api/vdi/workstation/102-vq_extnd.md` and
//! `docs/api/aes/application.md` (`ap_planes`); contract ADR-038 point 3.
//! EmuTOS has no path above 16 planes (`vdi/vdi_defs.h:22`); the 16-plane
//! values are EmuTOS's (`vdi/vdi_control.c:206-213`, `aes/gemsuper.c:97`).
//!
//! The kernel's backend answers `planes()` from the boot argument
//! (`kernel/src/vdi_backend.rs`, `set_planes`); here a metadata backend
//! answers whatever the test stores, so one binary checks both settings.
//! The boot-argument parsing itself is tested with `kernel/src/bootargs.rs`
//! (`vdiplanes_accepts_16_and_32_only`, run by the xtask host suite).
//!
//! Ticket: `TASK/V62.md`.

#![allow(clippy::too_many_arguments, unused_variables)]

use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, OnceLock};

use tos_api::bdos::process::Abi;
use tos_api::host::Host;
use tos_api::vdi::backend::{ClipRect, MonoSrc, PatternFill, Pel, RasterBackend, Rect, WriteMode};

// ── a metadata backend with a settable plane count ───────────────────────

struct Profile {
    planes: AtomicU16,
}

impl RasterBackend for Profile {
    fn width(&self) -> u16 {
        640
    }
    fn height(&self) -> u16 {
        480
    }
    fn planes(&self) -> u16 {
        self.planes.load(Ordering::Relaxed)
    }
    /// The 256 pens of both settings (ADR-024 point 2, ADR-038 point 3).
    fn num_colors(&self) -> u16 {
        256
    }
    fn set_color(&self, i: u16, r: u16, g: u16, b: u16) {}
    fn query_color(&self, i: u16) -> (u16, u16, u16) {
        (0, 0, 0)
    }
    fn set_active_palette(&self, p: &[u16; 256]) {}
    fn pel_to_device16(&self, p: Pel) -> u16 {
        0
    }
    fn device_to_rgb565(&self, p: u32) -> u16 {
        0
    }
    fn rgb565_to_device(&self, p: u16) -> u32 {
        0
    }
    fn read_pixel(&self, x: i16, y: i16) -> Pel {
        0
    }
    fn fill_rect(&self, x: i16, y: i16, w: i16, h: i16, p: Pel, c: ClipRect) {}
    fn abline(
        &self,
        x: i16,
        y: i16,
        xx: i16,
        yy: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
        mask: u16,
        last: bool,
    ) -> u16 {
        mask
    }
    fn fill_pattern(&self, x: i16, y: i16, w: i16, h: i16, p: PatternFill, c: ClipRect) {}
    fn draw_glyph_sized(
        &self,
        x: i16,
        y: i16,
        g: &[u8],
        w: i16,
        h: i16,
        p: Pel,
        m: WriteMode,
        c: ClipRect,
    ) {
    }
    fn blit_mono(&self, d: Rect, s: MonoSrc, f: Pel, b: Pel, m: WriteMode, c: ClipRect) {}
    fn copy_rect(&self, s: Rect, x: i16, y: i16, op: u8, c: ClipRect) {}
    fn get_device_pixel(&self, x: i16, y: i16) -> u32 {
        0
    }
    fn put_device_pixel(&self, x: i16, y: i16, p: u32) {}
    fn flush_rect(&self, x: i16, y: i16, xx: i16, yy: i16) {}
}

struct TestHost {
    raster: Profile,
}

impl Host for TestHost {
    fn console_write_byte(&self, byte: u8) {}
    fn raster(&self) -> Option<&dyn RasterBackend> {
        Some(&self.raster)
    }
}

static HOST: TestHost = TestHost { raster: Profile { planes: AtomicU16::new(16) } };
static HOST_REF: &dyn Host = &HOST;

/// The installed host, the VDI handle table and the AES state are
/// process-wide.
static SERIAL: Mutex<()> = Mutex::new(());
static INSTALL: OnceLock<()> = OnceLock::new();

/// Take the lock and set the depth the boot argument would have chosen.
fn setting(planes: u16) -> std::sync::MutexGuard<'static, ()> {
    let g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    INSTALL.get_or_init(|| tos_api::host::install_host(&HOST_REF));
    HOST.raster.planes.store(planes, Ordering::Relaxed);
    g
}

// ── VDI parameter block ──────────────────────────────────────────────────

const POISON: i16 = 0x5a3cu16 as i16;

struct Vpb {
    contrl: Box<[i16; 16]>,
    intin: Box<[i16; 32]>,
    ptsin: Box<[i16; 32]>,
    intout: Box<[i16; 512]>,
    ptsout: Box<[i16; 64]>,
    block: Box<[u64; 5]>,
}

impl Vpb {
    fn new() -> Vpb {
        let mut pb = Vpb {
            contrl: Box::new([0; 16]),
            intin: Box::new([0; 32]),
            ptsin: Box::new([0; 32]),
            intout: Box::new([0; 512]),
            ptsout: Box::new([0; 64]),
            block: Box::new([0; 5]),
        };
        pb.block[0] = pb.contrl.as_ptr() as u64;
        pb.block[1] = pb.intin.as_ptr() as u64;
        pb.block[2] = pb.ptsin.as_ptr() as u64;
        pb.block[3] = pb.intout.as_ptr() as u64;
        pb.block[4] = pb.ptsout.as_ptr() as u64;
        pb
    }

    /// Issue `opcode` with the outputs poisoned; returns the trap's `d0`.
    fn op(&mut self, opcode: i16, sub: i16, n_pts: i16, n_int: i16, handle: i16) -> i16 {
        self.contrl.fill(0);
        self.intout.fill(POISON);
        self.ptsout.fill(POISON);
        self.contrl[0] = opcode;
        self.contrl[1] = n_pts;
        self.contrl[3] = n_int;
        self.contrl[5] = sub;
        self.contrl[6] = handle;
        // SAFETY: `block` holds five live pointers to arrays owned by `self`,
        // each larger than any opcode issued here writes (`vq_scrninfo`'s
        // 273 words included).
        unsafe { tos_api::vdi::dispatch(self.block.as_ptr() as u64, Abi::Lp64) }
    }

    /// `v_opnwk` (1) or `v_opnvwk` (100) with every attribute index 1 and
    /// raster coordinates; returns `CONTRL[6]`, `DEV_TAB` is in `intout`.
    fn open(&mut self, opcode: i16, handle: i16) -> i16 {
        for i in 0..10 {
            self.intin[i] = 1;
        }
        self.intin[10] = 2;
        self.op(opcode, 0, 0, 11, handle);
        self.contrl[6]
    }

    /// `vq_extnd` (102) mode `mode` with sub-opcode `sub`.
    fn vq_extnd(&mut self, sub: i16, mode: i16, handle: i16) -> i16 {
        self.intin[0] = mode;
        self.op(102, sub, 0, 1, handle)
    }
}

/// The physical workstation and one virtual workstation on it, opened at the
/// current setting. Returns (physical handle, `DEV_TAB` of `v_opnwk`, virtual
/// handle, `DEV_TAB` of `v_opnvwk`).
fn open_pair() -> (i16, [i16; 45], i16, [i16; 45]) {
    let mut pb = Vpb::new();
    let phys = pb.open(1, 0);
    let mut dev_phys = [0i16; 45];
    dev_phys.copy_from_slice(&pb.intout[..45]);
    let virt = pb.open(100, phys);
    assert!(virt > 0, "v_opnvwk handle {virt}");
    let mut dev_virt = [0i16; 45];
    dev_virt.copy_from_slice(&pb.intout[..45]);
    (phys, dev_phys, virt, dev_virt)
}

fn close(virt: i16) {
    Vpb::new().op(101, 0, 0, 0, virt); // v_clsvwk
}

// ── the value table, §3 ──────────────────────────────────────────────────

/// `DEV_TAB[13]`, `[35]`, `[39]` of `v_opnwk`, `v_opnvwk` and `vq_extnd`
/// mode 0 at both settings: 256, 1, 0 (EmuTOS's formulas,
/// `vdi_control.c:206-207`, `bios/screen.c:838-846`; ADR-038 point 3).
#[test]
fn dev_tab_colour_words_at_16_and_at_32_planes() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let (phys, dev_phys, virt, dev_virt) = open_pair();
        let mut pb = Vpb::new();
        assert_eq!(pb.vq_extnd(0, 0, virt), 1, "{planes}: vq_extnd(0) d0");
        let dev_q = &pb.intout[..45];
        for (name, dev) in [("v_opnwk", &dev_phys[..]), ("v_opnvwk", &dev_virt[..]), ("vq_extnd(0)", dev_q)] {
            assert_eq!(dev[13], 256, "{planes} planes, {name}: DEV_TAB[13] simultaneous colours");
            assert_eq!(dev[35], 1, "{planes} planes, {name}: DEV_TAB[35] colour capable");
            assert_eq!(dev[39], 0, "{planes} planes, {name}: DEV_TAB[39] palette");
        }
        // The setting changes no other DEV_TAB word.
        assert_eq!(dev_virt[..], dev_q[..], "{planes}: vq_extnd(0) = v_opnvwk's DEV_TAB");
        close(virt);
    }
}

/// `vq_extnd` mode 1: `INQ_TAB[4]` = the setting's depth, `INQ_TAB[5]` = 0
/// at both (EmuTOS `vdi_control.c:210-213` at 16; ADR-038 point 3 [D] at
/// 32), and no other `INQ_TAB` word depends on the depth.
#[test]
fn inq_tab_planes_and_lookup_table_at_16_and_at_32_planes() {
    let mut inq_at = Vec::new();
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let (_, _, virt, _) = open_pair();
        let mut pb = Vpb::new();
        assert_eq!(pb.vq_extnd(0, 1, virt), 1, "{planes}: vq_extnd(1) d0");
        assert_eq!(pb.intout[4], planes as i16, "{planes}: INQ_TAB[4] = v_planes");
        assert_eq!(pb.intout[5], 0, "{planes}: INQ_TAB[5], no video lookup table");
        assert_eq!((pb.contrl[2], pb.contrl[4]), (6, 45), "{planes}: CONTRL[2]/[4]");
        assert!(pb.intout[45..].iter().all(|&w| w == POISON), "{planes}: written past INTOUT[44]");
        let mut inq = [0i16; 45];
        inq.copy_from_slice(&pb.intout[..45]);
        inq_at.push(inq);
        close(virt);
    }
    let (mut a, mut b) = (inq_at[0], inq_at[1]);
    a[4] = 0;
    b[4] = 0;
    assert_eq!(a, b, "INQ_TAB differs between 16 and 32 planes in a word other than [4]");
}

/// A plane count that is neither 16 nor 32 — only a test double has one —
/// keeps EmuTOS's rule for `INQ_TAB[5]` (`vdi_control.c:211-213`): a lookup
/// table is present.
#[test]
fn inq_tab_lookup_table_keeps_the_emutos_rule_for_other_depths() {
    for planes in [1u16, 4, 8, 24] {
        let _g = setting(planes);
        let (_, _, virt, _) = open_pair();
        let mut pb = Vpb::new();
        pb.vq_extnd(0, 1, virt);
        assert_eq!(pb.intout[4], planes as i16, "{planes}: INQ_TAB[4]");
        assert_eq!(pb.intout[5], 1, "{planes}: INQ_TAB[5]");
        close(virt);
    }
}

// ── the EdDI gate, §4 ────────────────────────────────────────────────────

/// Between V62 and V66 the EdDI gate is not widened (TASK/V62.md, spec gate
/// point 1): at 32 planes `vq_scrninfo` (`CONTRL[5] == 1`, `INTIN[0] == 2`)
/// is answered as plain `vq_extnd` mode 2, i.e. `INQ_TAB` with 45 words; at
/// 16 the 273-word table stays.
#[test]
fn vq_scrninfo_at_32_planes_falls_through_to_vq_extnd() {
    {
        let _g = setting(32);
        let (_, _, virt, _) = open_pair();
        let mut eddi = Vpb::new();
        assert_eq!(eddi.vq_extnd(1, 2, virt), 1, "d0");
        let mut plain = Vpb::new();
        plain.vq_extnd(0, 1, virt);
        assert_eq!(eddi.contrl[4], 45, "CONTRL[4]: the INQ_TAB answer, not 272");
        assert_eq!(eddi.intout[..], plain.intout[..], "INTOUT = vq_extnd mode 1");
        assert_eq!(eddi.ptsout[..], plain.ptsout[..], "PTSOUT = vq_extnd mode 1");
        assert_eq!(eddi.intout[4], 32);
        close(virt);
    }
    {
        let _g = setting(16);
        let (_, _, virt, _) = open_pair();
        let mut eddi = Vpb::new();
        eddi.vq_extnd(1, 2, virt);
        assert_eq!(eddi.contrl[4], 272, "16 planes: the vq_scrninfo answer");
        assert_eq!(eddi.intout[2], 16, "16 planes: vq_scrninfo [2] bits per pixel");
        close(virt);
    }
}

// ── AES: ap_planes ───────────────────────────────────────────────────────

/// `appl_init` through the LP64 dispatcher; returns the caller's 16
/// `AESGLOBAL` words.
fn appl_init() -> [i16; 16] {
    let mut control = [10i16, 0, 1, 0, 0];
    let mut global = [POISON; 16];
    let mut intin = [0i16; 16];
    let mut intout = [POISON; 8];
    let mut addrin = [0u64; 2];
    let mut addrout = [0u64; 2];
    let raw = [
        control.as_mut_ptr() as u64,
        global.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        addrin.as_mut_ptr() as u64,
        addrout.as_mut_ptr() as u64,
    ];
    // SAFETY: a well-formed LP64 parameter block, alive for the call.
    unsafe { tos_api::aes::dispatch(raw.as_ptr() as u64, Abi::Lp64) };
    assert!(intout[0] >= 0, "appl_init failed");
    global
}

/// `ap_planes` (`global[10]`, byte 20) = `planes()`: 16 at the default, 32
/// at the 32-plane setting (EmuTOS `gemsuper.c:97` via `gemgraf.c:395` and
/// `gemgsxif.c:495-501`; ADR-038 points 3 and 10).
#[test]
fn ap_planes_follows_the_setting() {
    for planes in [16u16, 32] {
        let _g = setting(planes);
        let global = appl_init();
        assert_eq!(global[10], planes as i16, "ap_planes at {planes} planes");
    }
}
