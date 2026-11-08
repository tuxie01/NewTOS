//! V56 — the `vex_*` default vectors are three distinct addresses, and an
//! application that saves the AES's wheel vector and later puts it back gives
//! the wheel back to the AES (`TASK/V56.md`).
//!
//! EmuTOS' slots hold real routines: `just_rts`, `mov_cur`
//! (`vdi/vdi_mouse.c:585-589`) and, after `gsx_init`, the AES's `aes_wheel`
//! (`aes/gemgsxif.c:288-292`). NewTOS stores address tokens for them
//! (`tos-api/src/vdi/mouse.rs`, `token_va`), and `vex_wheelv` tells the AES's
//! token apart from `just_rts` to know whether the restored vector is the AES
//! (`vex_wheelv`'s classification). Until V56 the tokens were three empty
//! functions, which the release kernel folded into one address: the restore
//! was taken for "no handler" and the desktop's wheel stayed dead.
//!
//! The fold is an optimiser effect, so these tests show the defect only in an
//! optimised build: `cargo test -p tos-api --release --test
//! aes_v56_vector_tokens` is red on the pre-V56 code; the default (debug)
//! build of `cargo xtask test` passes either way and pins the contract.

#[path = "aes_reference/harness.rs"]
mod harness;

use harness::{context, Pb, AES_LOCK};
use tos_api::bdos::process::Abi;
use tos_api::bios::vectors::mousexvec;

const VEX_BUTV: i16 = 125;
const VEX_CURV: i16 = 127;
const VEX_WHEELV: i16 = 134;

// AES opcodes.
const WIND_CREATE: i16 = 100;
const WIND_OPEN: i16 = 101;
const WIND_CLOSE: i16 = 102;
const WIND_DELETE: i16 = 103;

// Message words (`include/aesdefs.h:233,248-249`).
const WM_ARROWED: i16 = 24;
const WA_DNLINE: i16 = 3;

/// The window the wheel scrolls, away from the menu bar.
const WX: i16 = 100;
const WY: i16 = 120;
const WW: i16 = 240;
const WH: i16 = 160;

/// An application's own routine, as far as the VDI can tell: an address that
/// is none of the tokens. Never called (no live process issues the trap here,
/// so `vex_wheelv` stores it without a kind to call).
const OWN: u64 = 0x0000_0000_0004_2000;

/// `vex_*(new)` on the AES's physical workstation → the previous vector
/// (LP64 pointer slot 1, CONTRL words 11..14).
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

/// The default a program sees: swap in `OWN`, take the old one, put it back.
fn default_of(op: i16) -> u64 {
    let old = vex(op, OWN);
    assert_eq!(vex(op, old), OWN, "opcode {op}: the restore hands back the own routine");
    old
}

/// A fresh AES session, as the kernel's session loop starts one:
/// `gsx_wsclose` + `gsx_init` (its `v_opnwk` resets the vectors,
/// `vdimouse_init`, and the AES claims the wheel, `install_aes_wheel`).
fn new_session() {
    tos_api::aes::gsxif::gsx_wsclose();
    tos_api::aes::gsxif::gsx_init();
}

/// Put the pointer at `(x, y)` through the real input path, so the AES's
/// `xrat`/`yrat` follow it (as `tests/aes_wheel_change.rs`).
fn move_pointer_to(x: i16, y: i16) {
    let backend = tos_api::host::host().unwrap().raster().unwrap();
    let (cx, cy) = tos_api::vdi::mouse_position();
    let (dx, dy) = (x - cx, y - cy);
    if dx == 0 && dy == 0 {
        tos_api::vdi::feed_relative(1, 1, 0, backend);
        tos_api::vdi::feed_relative(-1, -1, 0, backend);
        return;
    }
    tos_api::vdi::feed_relative(dx, dy, 0, backend);
    assert_eq!(tos_api::vdi::mouse_position(), (x, y), "pointer parked at {x},{y}");
}

fn open_window(pb: &mut Pb) -> i16 {
    pb.poison();
    pb.intin[0] = 0;
    pb.intin[1] = WX;
    pb.intin[2] = WY;
    pb.intin[3] = WW;
    pb.intin[4] = WH;
    pb.call(WIND_CREATE, 5, 1, 0, 0);
    let handle = pb.intout[0];
    assert!(handle > 0, "wind_create returned no handle ({handle})");
    pb.poison();
    pb.intin[0] = handle;
    pb.intin[1] = WX;
    pb.intin[2] = WY;
    pb.intin[3] = WW;
    pb.intin[4] = WH;
    pb.call(WIND_OPEN, 5, 1, 0, 0);
    handle
}

fn close_window(pb: &mut Pb, handle: i16) {
    pb.poison();
    pb.intin[0] = handle;
    pb.call(WIND_CLOSE, 1, 1, 0, 0);
    pb.poison();
    pb.intin[0] = handle;
    pb.call(WIND_DELETE, 1, 1, 0, 0);
}

fn pending() -> Option<[i16; 8]> {
    tos_api::aes::window::test_pending_msg(0)
}

fn clear() {
    tos_api::aes::window::test_clear_msg(0);
}

#[test]
fn just_rts_mov_cur_and_the_aes_wheel_token_are_three_addresses() {
    let _guard = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    new_session();

    let but = default_of(VEX_BUTV); // `user_but = just_rts` (`vdi_mouse.c:585`)
    let cur = default_of(VEX_CURV); // `user_cur = mov_cur` (`:587`)
    let wheel = default_of(VEX_WHEELV); // the AES's `aes_wheel` (`gemgsxif.c:291`)
    assert_ne!(wheel, but, "the AES's wheel vector is not just_rts");
    assert_ne!(cur, but, "mov_cur is not just_rts");
    assert_ne!(wheel, cur, "the AES's wheel vector is not mov_cur");

    // `vdimouse_exit` (`vdi_mouse.c:624-628`): every slot `just_rts`, `user_cur`
    // too — the one place two slots hold the same address on purpose.
    tos_api::aes::gsxif::gsx_wsclose();
    let closed_but = default_of(VEX_BUTV);
    let closed_cur = default_of(VEX_CURV);
    let closed_wheel = default_of(VEX_WHEELV);
    assert_eq!(closed_but, but, "just_rts stays one address");
    assert_eq!(closed_cur, but, "v_clswk puts just_rts into user_cur");
    assert_eq!(closed_wheel, but, "v_clswk puts just_rts into user_wheel");
    tos_api::aes::gsxif::gsx_init();
}

#[test]
fn restoring_the_saved_aes_wheel_vector_gives_the_wheel_back_to_the_aes() {
    let _guard = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    new_session();
    let mut pb = Pb::new();
    let win = open_window(&mut pb);
    move_pointer_to(WX + WW / 2, WY + WH / 2);
    clear();

    // The AES owns the wheel: a detent over the window scrolls it.
    mousexvec(0x5A);
    let msg = pending().expect("the session's wheel reaches the AES");
    assert_eq!((msg[0], msg[3], msg[4]), (WM_ARROWED, win, WA_DNLINE));
    clear();

    // A program takes the wheel (`vex_wheelv(own, &old)`) ...
    let old = vex(VEX_WHEELV, OWN);
    mousexvec(0x5A);
    assert!(pending().is_none(), "while the program's vector is in, the AES hears no detent");

    // ... and gives it back (`vex_wheelv(old, &dummy)`): the AES's again.
    assert_eq!(vex(VEX_WHEELV, old), OWN);
    mousexvec(0x5A);
    let msg = pending().expect("the restored AES wheel vector must reach the AES again");
    assert_eq!((msg[0], msg[3], msg[4]), (WM_ARROWED, win, WA_DNLINE));
    clear();

    close_window(&mut pb, win);
}
