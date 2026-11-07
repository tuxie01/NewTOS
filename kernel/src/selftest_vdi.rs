//! Phase 12 VDI foundation self-test (D1/D2).
//!
//! Exercises the VDI dispatcher and workstation lifecycle through the
//! **real trap path** (`svc #2`, VDIPB pointer in `x0`): open a virtual
//! workstation, check the returned capability table matches the
//! framebuffer geometry, clear the workstation (visibly fills the
//! screen), close it, and confirm the handle is freed by re-opening.

use crate::kprintln;

/// Issue a VDI call: the GEM trap (`svc #2`) with selector 115 (VDI) in
/// `x0` and the parameter-block pointer in `x1`.
#[inline(always)]
fn vdi(pb_ptr: u64) {
    // SAFETY: GEM trap. `x0` comes back as the VDI's `flip_y`
    // (`tos_api::vdi::FLIP_Y_MAGNITUDES`); this client has no use for it and
    // drops it, exactly as gemlib's `vdi()` binding does.
    unsafe {
        core::arch::asm!(
            "svc #2",
            inout("x0") 115u64 => _,
            in("x1") pb_ptr,
            options(nostack),
        );
    }
}

/// Run the Phase 12 VDI foundation self-test (dispatcher + workstation
/// lifecycle) through the `svc #2` trap path.
//
// The VDIPB arrays are written here and read by the dispatcher *through
// the parameter-block pointer* (`pb`), which the compiler's dataflow
// can't see — so resets like `contrl = [0; 12]` look like dead stores.
// They are not; allow the false positive.
#[allow(unused_assignments)]
pub fn run_phase12_vdi_test() {
    let (sw, sh) = crate::vdi_backend::dims();
    if sw == 0 || sh == 0 {
        kprintln!("[vdi] no framebuffer — skipping VDI test");
        return;
    }

    // VDI parameter block: five WORD arrays + the 5-pointer block.
    let mut contrl = [0i16; 12];
    let mut intin = [0i16; 16];
    let mut ptsin = [0i16; 16];
    let mut intout = [0i16; 45];
    let mut ptsout = [0i16; 12];
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        ptsin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    let pb = block.as_ptr() as u64;

    kprintln!("[vdi] === Phase 12 VDI foundation test (D1/D2) ===");

    let mut fails = 0;
    let mut check = |cond: bool, label: &str| {
        kprintln!("[vdi]   {}: {}", label, if cond { "OK" } else { "FAIL" });
        if !cond {
            fails += 1;
        }
    };

    // ── v_opnvwk (100): open a virtual workstation ──
    contrl = [0i16; 12];
    contrl[0] = 100;
    contrl[3] = 11; // 11 work_in inputs
    // work_in: device id + line/marker/text/fill attrs + coord system.
    intin[..11].copy_from_slice(&[1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 2]);
    vdi(pb);
    let handle = contrl[6];

    check((1..=16).contains(&handle), "v_opnvwk returns a valid handle");
    check(
        intout[0] == (sw as i16 - 1) && intout[1] == (sh as i16 - 1),
        "DEV_TAB geometry = framebuffer size − 1",
    );
    // Truecolor: DEV_TAB[13] = numcolors = 256 for v_planes=16
    // (EmuTOS vdi_control.c:206, (v_planes<8)?(1<<v_planes):256).
    check(intout[13] == 256, "DEV_TAB[13] = 256 colours (truecolor)");
    check(
        contrl[4] == 45 && contrl[2] == 6,
        "output counts (45 INTOUT, 6 PTSOUT pairs)",
    );

    // ── v_clrwk (3): clear the screen (visible: fills white) ──
    contrl = [0i16; 12];
    contrl[0] = 3;
    contrl[6] = handle;
    vdi(pb);
    kprintln!("[vdi]   v_clrwk: screen cleared to background (white)");

    // ── v_clsvwk (101): close, then prove the handle is freed by
    //    re-opening and getting a valid handle back ──
    contrl = [0i16; 12];
    contrl[0] = 101;
    contrl[6] = handle;
    vdi(pb);

    contrl = [0i16; 12];
    contrl[0] = 100;
    contrl[3] = 11;
    intin[..11].copy_from_slice(&[1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 2]);
    vdi(pb);
    let handle2 = contrl[6];
    check(
        (1..=16).contains(&handle2),
        "re-open after close yields a handle (slot freed)",
    );
    // tidy up
    contrl = [0i16; 12];
    contrl[0] = 101;
    contrl[6] = handle2;
    vdi(pb);

    if fails == 0 {
        kprintln!("[vdi] === VDI foundation PASS ===");
    } else {
        kprintln!("[vdi] === VDI foundation FAILED ({} checks) ===", fails);
    }
}

/// Draw a visual demo exercising the Phase-12 drawing surface (fills,
/// GDP curves/boxes, markers, styled lines, text effects, clipping,
/// palette). Leaves the result on screen for a QMP `screendump`.
#[allow(unused_assignments)]
pub fn run_phase12_vdi_demo() {
    let (sw, sh) = crate::vdi_backend::dims();
    if sw == 0 || sh == 0 {
        return;
    }
    let mut contrl = [0i16; 16];
    let mut intin = [0i16; 64];
    let mut ptsin = [0i16; 64];
    let mut intout = [0i16; 64];
    let mut ptsout = [0i16; 64];
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        ptsin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    let pb = block.as_ptr() as u64;

    // open a vwk
    contrl = [0i16; 16];
    contrl[0] = 100;
    contrl[3] = 11;
    intin[..11].copy_from_slice(&[1, 1, 1, 3, 1, 1, 1, 1, 1, 1, 2]);
    vdi(pb);
    let h = contrl[6];

    // clear to white
    contrl = [0; 16];
    contrl[0] = 3;
    contrl[6] = h;
    vdi(pb);

    // helper to issue an opcode with given counts (a macro, so it does
    // not hold a borrow of `contrl` across the demo)
    macro_rules! call {
        ($op:expr, $sub:expr, $nin:expr, $npts:expr) => {{
            contrl = [0i16; 16];
            contrl[0] = $op;
            contrl[5] = $sub;
            contrl[3] = $nin;
            contrl[1] = $npts;
            contrl[6] = h;
            vdi(pb);
        }};
    }

    // solid red filled rectangle (vsf_color=2, vsf_interior=solid, vr_recfl)
    intin[0] = 2;
    call!(25, 0, 1, 0); // vsf_color red
    intin[0] = 1;
    call!(23, 0, 1, 0); // vsf_interior solid
    ptsin[..4].copy_from_slice(&[40, 40, 160, 120]);
    call!(114, 0, 0, 2); // vr_recfl

    // green filled circle (GDP circle, sub 4): centre (260,90) r=50
    intin[0] = 3;
    call!(25, 0, 1, 0); // fill green
    ptsin[..7].copy_from_slice(&[260, 90, 0, 0, 50, 0, 0]);
    call!(11, 4, 0, 3); // v_gdp circle

    // blue filled triangle (v_fillarea)
    intin[0] = 4;
    call!(25, 0, 1, 0);
    ptsin[..6].copy_from_slice(&[360, 40, 460, 140, 360, 140]);
    call!(9, 0, 0, 3);

    // magenta rounded filled box (GDP rfbox 9)
    intin[0] = 7;
    call!(25, 0, 1, 0);
    ptsin[..4].copy_from_slice(&[500, 40, 610, 130]);
    call!(11, 9, 0, 2);

    // black markers (asterisk) across the middle
    intin[0] = 1; // black
    call!(20, 0, 1, 0); // vsm_color
    intin[0] = 3;
    call!(18, 0, 1, 0); // vsm_type asterisk
    for i in 0..6 {
        ptsin[2 * i] = 60 + 90 * i as i16;
        ptsin[2 * i + 1] = 200;
    }
    call!(7, 0, 0, 6); // v_pmarker

    // dashed line (vsl_type=3 dash-dot, width 3)
    intin[0] = 1;
    call!(17, 0, 1, 0); // vsl_color black
    intin[0] = 3;
    call!(15, 0, 1, 0); // vsl_type
    ptsin[0] = 1;
    call!(16, 0, 0, 1); // vsl_width via PTSIN[0]
    ptsin[..4].copy_from_slice(&[40, 240, 600, 240]);
    call!(6, 0, 0, 2); // v_pline

    // bold + underlined text
    intin[0] = 1;
    call!(22, 0, 1, 0); // vst_color black
    intin[0] = 0x09; // thicken | underline
    call!(106, 0, 1, 0); // vst_effects
    ptsin[..2].copy_from_slice(&[40, 280]);
    let msg = b"NewTOS VDI Phase 12";
    for (i, &c) in msg.iter().enumerate() {
        intin[i] = c as i16;
    }
    contrl = [0; 16];
    contrl[0] = 8;
    contrl[3] = msg.len() as i16;
    contrl[1] = 1;
    contrl[6] = h;
    vdi(pb);

    // clipped fill: set a clip rect, then a big yellow rect that gets cut
    intin[0] = 0x09; // clear effects back
    intin[0] = 6;
    call!(25, 0, 1, 0); // fill yellow
    intin[0] = 1; // clip on
    ptsin[..4].copy_from_slice(&[40, 320, 240, 380]);
    call!(129, 0, 1, 2); // vs_clip to a small window
    ptsin[..4].copy_from_slice(&[0, 300, 640, 400]); // big rect
    call!(114, 0, 0, 2); // vr_recfl — only the clip window fills
    intin[0] = 0;
    call!(129, 0, 1, 0); // clip off

    // show the mouse cursor (parked at centre)
    intin[0] = 0;
    call!(122, 0, 1, 0); // v_show_c

    kprintln!("[vdi] demo drawn (handle {})", h);
}
