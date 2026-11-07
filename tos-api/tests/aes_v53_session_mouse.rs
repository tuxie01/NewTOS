//! V53 — the AES side of `v_clswk` / `v_opnwk` (`TASK/V53.md`, AES counter-check).
//!
//! In EmuTOS the AES hears the mouse only through the VDI vectors it installs
//! in `gsx_init` right after its `v_opnwk`: `gsx_setmb(far_bcha, far_mcha, …)`
//! (`references/emutos/aes/gemgsxif.c:271`, `vex_butv`/`vex_motv`) and
//! `vex_wheelv(aes_wheel, …)` (`:288-292`). The session end's `gsx_wsclose`
//! (`:191-194`, `geminit.c:749`) is a `v_clswk`, whose `vdimouse_exit` puts
//! `just_rts` into all of them and switches the IKBD mouse off
//! (`vdi/vdi_mouse.c:622-645`): from then until the next session's
//! `gsx_init` no packet, no extra button and no wheel detent reaches the AES.
//!
//! NewTOS' AES is called directly by the VDI mouse feed
//! (`tos_api::aes::user_but` / `user_mot`, `tos-api/src/vdi/mouse.rs`
//! `feed_relative`), so the same must hold for those direct calls: after
//! `gsx_wsclose` they stop, and `gsx_init` brings them — and the AES's wheel
//! vector — back. The AES's own view is what `evnt_multi` reports
//! (`ev_rets`: the AES `button`, `aes/gemevlib.c:42-58`).

#[path = "aes_reference/harness.rs"]
mod harness;

use harness::{context, host_only, Pb, AES_LOCK};
use tos_api::bdos::process::Abi;
use tos_api::bios::vectors::mousexvec;
use tos_api::host::Host;

const OP_EVNT_MULTI: i16 = 25;
const MU_TIMER: i16 = 0x0020;

fn backend() -> &'static dyn tos_api::vdi::backend::RasterBackend {
    Host::raster(host_only()).expect("the harness host has a raster backend")
}

/// `evnt_multi(MU_TIMER, 0 ms)` → `(x, y, button)` as the AES reports them
/// (`int_out[1..3]`).
fn aes_mouse(pb: &mut Pb) -> (i16, i16, i16) {
    pb.poison();
    pb.intin.fill(0);
    pb.intin[0] = MU_TIMER;
    let mut msg = [0i16; 8];
    pb.addrin[0] = msg.as_mut_ptr() as u64;
    pb.call(OP_EVNT_MULTI, 16, 7, 1, 0);
    (pb.intout[1], pb.intout[2], pb.intout[3])
}

/// Put the pointer at `(x, y)`, button up.
fn park(x: i16, y: i16) {
    tos_api::vdi::feed_relative(-4000, -4000, 0, backend());
    tos_api::vdi::feed_relative(x, y, 0, backend());
}

/// `vex_wheelv(new)` on the AES's physical workstation → the previous vector
/// (LP64 pointer slot 1, CONTRL words 11..14).
fn vex_wheelv(new: u64) -> u64 {
    vex(134, new)
}

fn vex(op: i16, new: u64) -> u64 {
    let mut contrl = [0i16; 16];
    let mut intin = [0i16; 16];
    let mut ptsin = [0i16; 16];
    let mut intout = [0i16; 16];
    let mut ptsout = [0i16; 16];
    contrl[0] = op;
    contrl[6] = tos_api::aes::gsxif::gl_handle();
    let block: [u64; 5] = [
        contrl.as_mut_ptr() as u64,
        intin.as_mut_ptr() as u64,
        ptsin.as_mut_ptr() as u64,
        intout.as_mut_ptr() as u64,
        ptsout.as_mut_ptr() as u64,
    ];
    // SAFETY: the arrays outlive the call; slots 0/1 lie inside CONTRL[7..15].
    unsafe {
        (contrl[7..].as_mut_ptr() as *mut u64).write_unaligned(new);
        tos_api::vdi::dispatch(block.as_ptr() as u64, Abi::Lp64);
        (contrl[11..].as_ptr() as *const u64).read_unaligned()
    }
}

#[test]
fn gsx_wsclose_stops_the_aes_mouse_and_gsx_init_brings_it_back() {
    let _guard = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    let mut pb = Pb::new();

    // A running session: packets reach the AES.
    park(200, 200);
    assert_eq!(aes_mouse(&mut pb), (200, 200, 0), "session running, pointer parked");
    tos_api::vdi::feed_relative(0, 0, 0x01, backend());
    assert_eq!(aes_mouse(&mut pb).2, 1, "a press reaches the AES (user_but)");
    tos_api::vdi::feed_relative(0, 0, 0, backend());
    assert_eq!(aes_mouse(&mut pb).2, 0, "and the release");
    let aes_wheel = vex_wheelv(0);
    vex_wheelv(aes_wheel);

    // The session end: v_clswk on gl_handle.
    tos_api::aes::gsxif::gsx_wsclose();
    tos_api::vdi::feed_relative(10, 10, 0x01, backend());
    assert_eq!(
        aes_mouse(&mut pb),
        (200, 200, 0),
        "after gsx_wsclose no packet reaches the VDI cursor or the AES (vdimouse_exit, Initmous(0))"
    );
    mousexvec(0x37); // button 3: MOUSE_BT follows, the AES does not hear it
    assert_eq!(aes_mouse(&mut pb).2, 0, "the extra button does not reach the AES (user_but = just_rts)");
    mousexvec(0xB7);
    let closed_wheel = vex_wheelv(0);
    vex_wheelv(closed_wheel);
    assert_ne!(closed_wheel, aes_wheel, "vdimouse_exit takes the AES's wheel vector out (user_wheel = just_rts)");
    let closed_but = vex(125, 0);
    vex(125, closed_but);
    assert_eq!(closed_wheel, closed_but, "the wheel vector is just_rts, as user_but");

    // The next session: gsx_init's v_opnwk + the AES's vectors.
    tos_api::aes::gsxif::gsx_init();
    assert_eq!(aes_mouse(&mut pb), (319, 239, 0), "v_opnwk parks the pointer at the centre");
    tos_api::vdi::feed_relative(5, 5, 0, backend());
    assert_eq!(aes_mouse(&mut pb), (324, 244, 0), "packets move the pointer again");
    tos_api::vdi::feed_relative(0, 0, 0x01, backend());
    assert_eq!(aes_mouse(&mut pb).2, 1, "a press reaches the AES again");
    tos_api::vdi::feed_relative(0, 0, 0, backend());
    assert_eq!(aes_mouse(&mut pb).2, 0, "and the release");
    let reopened_wheel = vex_wheelv(0);
    vex_wheelv(reopened_wheel);
    assert_eq!(reopened_wheel, aes_wheel, "gsx_init installs the AES's wheel vector again (gemgsxif.c:288-292)");
}
