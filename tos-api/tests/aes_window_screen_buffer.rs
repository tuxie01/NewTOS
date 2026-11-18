//! `wind_get(WF_SCREEN)` hands out the AES menu/alert screen buffer — address
//! and length, as two native LONGs over `int_out[1..4]`, TRUE.
//!
//! EmuTOS answers field 17 out of `gsx_mret`: `gsx_mret((LONG *)poutwds,
//! (LONG *)(poutwds + 2))` with `poutwds = &int_out[1]`
//! (`references/emutos/aes/gemwmlib.c:1563-1565`, `aes/gemsuper.c:343`;
//! `gsx_mret` `aes/gemgsxif.c:125-129`, pinned tree
//! `1a7484aba1bfba23e2c0899e243b6d8177fe41df`) — `*(LONG *)&int_out[1] =
//! gl_tmp.fd_addr; *(LONG *)&int_out[3] = gl_mlen;` — and returns TRUE
//! (`:1595-1598`), on any handle (`:1517-1520`). The buffer is taken **once**
//! at AES start by `gsx_malloc` (`:108-114`, from `sh_tographic`,
//! `aes/gemshlib.c:236`); the 1.4 ROM on the reference TT reported address
//! `0x01000218`, length `40960` (`measurements/measurement-2026-09-12-aes-r4/`, cases 331,
//! 332 and 439 of `tosdump/cases/aes-r4-subfunctions.tsv`, CRC `227422B2`).
//!
//! NewTOS has the buffer since A57 (`tos_api::aes::gsxif`, design
//! `docs/analysis/a57-aes-screen-buffer-design-2026-09-20.md`); this binary
//! runs it through the harness fixture (`aes_reference/harness.rs`,
//! `install_screen_buffer`) because a host process has no BDOS pool. A57
//! **turned this file round**: until then it pinned the A54 answer, FALSE with
//! nothing written, the honest reply while there was no buffer.
//!
//! # What the reference decides, and what the formula decides
//!
//! The r4 table itself declares `WF_SCREEN` a **device parameter** —
//! "RECORDED, not compared away" (`aes-r4-subfunctions.tsv`, "WHAT IS DEVICE
//! DEPENDENT"): the address is wherever that machine's pool put the block and
//! the length is `gsx_mcalc` of that machine's screen. So, as the VDI suite
//! does for its plane-dependent words (`vdi_reference/harness.rs`,
//! `MACHINE_EXCLUDED` + `substitute`), the two LONGs are checked here **by
//! EmuTOS' rule evaluated on this workstation** rather than against the TT's
//! numbers:
//!
//! * `int_out[0]` — the sub-function question, device-free: **1**, as the TT
//!   recorded for 331/332/439 (and for round-1 case 166, which the reference
//!   harness now compares directly);
//! * `int_out[1..2]` — the buffer's address: the fixture's
//!   [`SCREEN_BUFFER_PA`] (an EL1 caller gets the physical/pool address,
//!   ADR-021; the TT's `0x01000218` is that machine's), **the same on every
//!   call**;
//! * `int_out[3..4]` — the length: NewTOS' `gsx_mcalc`, one screen in device
//!   pixels, `bb_save_words(W, H) * 4` = 614 400 at 640×480 (the TT's 40 960
//!   is `MENU_BUFFER_SIZE * 8 * 16 * 4 / 8` at four planes — the named
//!   deviation in `docs/COMPATIBILITY.md`, A57).
//!
//! Tickets: `TASK/A57.md` (closes the A54 remainder), `TASK/A54.md`.

#![allow(dead_code)]

#[path = "aes_reference/harness.rs"]
mod harness;

use harness::{context, Pb, AES_LOCK, POISON, SCREEN_BUFFER_PA, SCREEN_H, SCREEN_W};

const OP_WIND_CREATE: i16 = 100;
const OP_WIND_OPEN: i16 = 101;
const OP_WIND_GET: i16 = 104;
const OP_WIND_NEW: i16 = 109;

const WF_SCREEN: i16 = 17;
const WF_TOP: i16 = 10;
const WF_DCOLOR: i16 = 19;

const KIND: i16 = 0x0FEF;

fn wind_new(pb: &mut Pb) {
    pb.poison();
    pb.call(OP_WIND_NEW, 0, 0, 0, 0);
}

/// One `wind_get`, laid exactly as the r4 table lays cases 331/332/439:
/// `IN_LEN 2`, `OUT_LEN 5`, everything poisoned first.
fn wind_get5(pb: &mut Pb, handle: i16, field: i16) {
    pb.poison();
    pb.intin[0] = handle;
    pb.intin[1] = field;
    pb.call(OP_WIND_GET, 2, 5, 0, 0);
}

/// The LONG EmuTOS stored at `&int_out[word]` (`*(LONG *)poutwds`), read the
/// way it was written: one native 32-bit load at the WORD-aligned address —
/// what TeraDesk's `union { long size; struct { _WORD high, low; } words; }`
/// (`sdk/apps/teradesk/resource.c:233-241`) and cflib's
/// `*((long *)&aes_intout[3])` (`sdk/cflib/cflib-21/purec.c:169-171`) do.
fn long_at(pb: &Pb, word: usize) -> u32 {
    let bytes = [
        pb.intout[word].to_ne_bytes(),
        pb.intout[word + 1].to_ne_bytes(),
    ];
    u32::from_ne_bytes([bytes[0][0], bytes[0][1], bytes[1][0], bytes[1][1]])
}

/// `gsx_mcalc` at NewTOS' size for this harness's 640×480 workstation (its
/// installed raster backend, after [`context`]).
fn expected_len() -> u32 {
    let raster = tos_api::host::host().and_then(|h| h.raster()).expect("context() installs the harness backend");
    (tos_api::vdi::bb_save_words(raster, SCREEN_W as i16, SCREEN_H as i16) * 4) as u32
}

/// The three handles the measurement asks WF_SCREEN with: the desktop window
/// (case 331), an invalid one (case 332) and a real open window (case 439).
/// All three answer TRUE with the same address and length, because the reply
/// is about the buffer, not the handle (`gemwmlib.c:1517-1520`).
#[test]
fn wf_screen_reports_the_one_buffer_on_every_handle() {
    let _g = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    let mut pb = Pb::new();
    wind_new(&mut pb);

    // Case 331: the desktop handle.
    wind_get5(&mut pb, 0, WF_SCREEN);
    assert_eq!(pb.intout[0], 1, "wind_get(0, WF_SCREEN) must answer TRUE: gsx_mret reports the buffer");
    let addr = long_at(&pb, 1);
    let len = long_at(&pb, 3);
    assert_ne!(addr, 0, "the buffer address must not be 0");
    assert_eq!(
        addr, SCREEN_BUFFER_PA as u32,
        "an EL1 caller gets the block's physical (pool) address (ADR-021)"
    );
    assert_eq!(
        len,
        expected_len(),
        "the length is gsx_mcalc: one 640x480 screen in device pixels, bb_save_words(W,H)*4"
    );
    assert_eq!(len, 614_400, "614 400 bytes at 640x480 (the design's number)");

    // Case 332: WF_SCREEN needs no valid handle, so an invalid one must not
    // change the answer — the same buffer, the same address.
    wind_get5(&mut pb, -1, WF_SCREEN);
    assert_eq!(pb.intout[0], 1, "wind_get(-1, WF_SCREEN) must answer TRUE too");
    assert_eq!(long_at(&pb, 1), addr, "the address is the same on every call (one buffer, gl_tmp)");
    assert_eq!(long_at(&pb, 3), len, "the length is the same on every call (gl_mlen)");

    // Case 439: a window that really exists and is open.
    pb.poison();
    pb.intin[0] = KIND;
    pb.intin[1] = 10;
    pb.intin[2] = 20;
    pb.intin[3] = 300;
    pb.intin[4] = 200;
    pb.call(OP_WIND_CREATE, 5, 1, 0, 0);
    let h = pb.intout[0];
    assert!(h >= 1, "wind_create returned {h}");
    pb.poison();
    pb.intin[0] = h;
    pb.intin[1] = 30;
    pb.intin[2] = 40;
    pb.intin[3] = 320;
    pb.intin[4] = 240;
    pb.call(OP_WIND_OPEN, 5, 1, 0, 0);

    wind_get5(&mut pb, h, WF_SCREEN);
    assert_eq!(pb.intout[0], 1, "wind_get({h}, WF_SCREEN) on an open window must answer TRUE as well");
    assert_eq!(long_at(&pb, 1), addr, "same address on the open window's handle");
    assert_eq!(long_at(&pb, 3), len, "same length on the open window's handle");

    wind_new(&mut pb);
}

/// The two LONGs land where EmuTOS puts them and nowhere else: `int_out[1..2]`
/// and `int_out[3..4]` are written, `int_out[5]` past `OUT_LEN` is not
/// (`gemsuper.c:444-445`), and each LONG is one native 32-bit store at its
/// WORD-aligned slot — on AArch64 the low half first.
#[test]
fn wf_screen_writes_two_native_longs_over_int_out_1_to_4() {
    let _g = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    let mut pb = Pb::new();
    wind_new(&mut pb);

    wind_get5(&mut pb, 0, WF_SCREEN);
    assert_eq!(pb.intout[0], 1, "TRUE");
    let len = expected_len();
    assert_eq!(
        pb.intout[3] as u16,
        (len & 0xFFFF) as u16,
        "int_out[3] holds bits 0-15 of the length (little-endian native store)"
    );
    assert_eq!(
        pb.intout[4] as u16,
        (len >> 16) as u16,
        "int_out[4] holds bits 16-31 of the length"
    );
    assert_eq!(
        pb.intout[1] as u16,
        (SCREEN_BUFFER_PA & 0xFFFF) as u16,
        "int_out[1] holds bits 0-15 of the address"
    );
    assert_eq!(
        pb.intout[2] as u16,
        (SCREEN_BUFFER_PA >> 16) as u16,
        "int_out[2] holds bits 16-31 of the address"
    );
    assert_eq!(
        pb.intout[5], POISON,
        "the dispatcher copies back exactly OUT_LEN = 5 words (gemsuper.c:444-445)"
    );

    wind_new(&mut pb);
}

/// `WF_TOP` and `WF_DCOLOR` are the other two fields `wm_get` answers without
/// a valid handle (`gemwmlib.c:1517-1520`); both still answer TRUE on the
/// invalid handle case 333 uses — the trio is intact.
#[test]
fn the_other_two_handle_free_fields_still_answer() {
    let _g = AES_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    context();
    let mut pb = Pb::new();
    wind_new(&mut pb);

    wind_get5(&mut pb, -1, WF_TOP);
    assert_eq!(pb.intout[0], 1, "wind_get(-1, WF_TOP) still needs no handle");

    pb.poison();
    pb.intin[0] = -1;
    pb.intin[1] = WF_DCOLOR;
    pb.intin[2] = 0; // gadget
    pb.call(OP_WIND_GET, 3, 5, 0, 0);
    assert_eq!(pb.intout[0], 1, "wind_get(-1, WF_DCOLOR) still needs no handle");

    wind_new(&mut pb);
}
