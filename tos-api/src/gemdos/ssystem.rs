//! `Ssystem` — GEMDOS 0x154 (FreeMiNT extension).
//!
//! Spec: [`docs/api/gemdos/154-ssystem.md`]. Of FreeMiNT's modes only the
//! cookie subfunctions (`S_GETCOOKIE`/`S_SETCOOKIE`/`S_DELCOOKIE`) are
//! implemented; every other FreeMiNT mode returns `EINVFN` (NewTOS has no
//! `ENOSYS`), so the surface stays honest. Cookie storage: [`crate::cookie`].
//!
//! On top of those sit NewTOS's **own** subfunctions from 2000 upwards
//! ([`S_NEWTOS_CPUCLOCK`], [`S_NEWTOS_CPUTEMP`]) — machine facts an EL0
//! program cannot otherwise reach, because the hardware that knows them (the
//! BCM2711 mailbox) is kernel-only — [`S_NEWTOS_KBDVEC`] (2002), the
//! kernel entry of the per-process default `kbdvec` stub (KEYS02,
//! `docs/api/xbios/22-kbdvbase.md` B3/D6), and the system-information modes
//! 2003–2020 ([`S_NEWTOS_CPUMEAS`] … [`S_NEWTOS_KVERSION`], SYSINFO01, and
//! [`S_NEWTOS_INPUTMODE`], KEYOWNER): the effective settings the kernel runs
//! with — and [`S_NEWTOS_RESET`] (2021), the kernel entry of the OSHEADER
//! reset stub (RST01, `docs/api/bios/_reset-warm-cold.md`) — and
//! [`S_NEWTOS_USBSNAP`] (2022), the USB snapshot (USB01 step 2,
//! `docs/api/usb/usb-snapshot.md`). 2000 is free: FreeMiNT's own header
//! reserves 0–29, 100, `0x54f8`, 900–999 and 1000–**1999**
//! (`references/freemint/sys/mint/ssystem.h`, the two `reserved` comments),
//! and the highest number its dispatcher answers is the build-gated test mode
//! 3000 (`sys/ssystem.c:631`). The gap 2000…2999 belongs to nobody.

use tos_types::error::{EINVAL, EINVFN, ENSMEM, ERROR, E_OK};
use tos_types::Long;

use crate::cookie;
use crate::host;
use crate::trap::TrapArgs;

/// `Ssystem`'s GEMDOS function id. It is above `MAX_FN`, so the GEMDOS
/// dispatcher special-cases it before the static table lookup.
pub const SSYSTEM: usize = 0x154;

// Cookie subfunction modes (FreeMiNT `sys/mint/ssystem.h`).
const S_GETCOOKIE: u16 = 8;
const S_SETCOOKIE: u16 = 9;
const S_DELCOOKIE: u16 = 26;

/// NewTOS-own subfunction: the CPU core clock in **Hz**.
pub const S_NEWTOS_CPUCLOCK: u16 = 2000;
/// NewTOS-own subfunction: the CPU/SoC temperature in **thousandths of a
/// degree Celsius**.
pub const S_NEWTOS_CPUTEMP: u16 = 2001;
/// NewTOS-own subfunction: run the default `kbdvec` (EmuTOS `kbd_int`) for
/// one IKBD key byte — what the `kbdvec_default` stub in every process's page
/// 0 calls (KEYS02). Defined next to the stub in [`crate::xbios::kbdvecs`].
pub const S_NEWTOS_KBDVEC: u16 = crate::xbios::kbdvecs::S_NEWTOS_KBDVEC;
/// NewTOS-own subfunction: restart the machine — what the reset stub behind
/// OSHEADER `reseth`/`os_res1` calls (TASK/RST01). Defined next to the stub
/// in [`crate::bios::reset`].
pub const S_NEWTOS_RESET: u16 = crate::bios::reset::S_NEWTOS_RESET;

// SYSINFO01: the effective system settings a system-information tool shows
// (`docs/api/gemdos/154-ssystem.md` §"System-information modes (2003–2020)").
// Scalars 2003–2018 and 2020 share the 2000/2001 interface ([`newtos_scalar`]);
// 2019 is a string ([`s_newtos_kversion`]).

/// The ARM clock the firmware measured, Hz.
pub const S_NEWTOS_CPUMEAS: u16 = 2003;
/// The ARM clock policy in effect: 0 firmware, 1 fixed at max, 2 max refused.
pub const S_NEWTOS_CPUPOLICY: u16 = 2004;
/// Memory protection in effect: 0 off, 1 on, 2 strict.
pub const S_NEWTOS_MEMPROT: u16 = 2005;
/// Address-space IDs in effect: 0 off, 1 on, 2 test (`asid=tiny`).
pub const S_NEWTOS_ASID: u16 = 2006;
/// SD transfer mode in effect: 0 fast, 1 compatibility (`emmc=compat`).
pub const S_NEWTOS_SDMODE: u16 = 2007;
/// SD data-bus width negotiated with the card, bits.
pub const S_NEWTOS_SDBUS: u16 = 2008;
/// Block read cache size, KiB (0 = off).
pub const S_NEWTOS_BCACHE: u16 = 2009;
/// Block cache share of one linear read, percent.
pub const S_NEWTOS_BCACHEPCT: u16 = 2010;
/// ST-RAM pool size, KiB.
pub const S_NEWTOS_STRAM: u16 = 2011;
/// TT-RAM pool size, KiB (0 = none).
pub const S_NEWTOS_TTRAM: u16 = 2012;
/// Framebuffer width, pixels.
pub const S_NEWTOS_SCRWIDTH: u16 = 2013;
/// Framebuffer height, pixels.
pub const S_NEWTOS_SCRHEIGHT: u16 = 2014;
/// Framebuffer depth, bits per pixel.
pub const S_NEWTOS_SCRDEPTH: u16 = 2015;
/// VideoCore (GPU) memory, KiB.
pub const S_NEWTOS_GPUMEM: u16 = 2016;
/// Source of the active keyboard layout: 0 US, 1 German, 2 file, 3 changed.
pub const S_NEWTOS_KEYBOARD: u16 = 2017;
/// Serial console output: 0 off, 1 on.
pub const S_NEWTOS_SERIAL: u16 = 2018;
/// Kernel version + build stamp, a string into the caller's buffer.
pub const S_NEWTOS_KVERSION: u16 = 2019;
/// Where the keyboard/mouse pump runs: 0 also from the timer interrupt (the
/// pointer moves while a program is busy), 1 `input=aes` (KEYOWNER).
pub const S_NEWTOS_INPUTMODE: u16 = 2020;
/// The USB snapshot: device list, descriptors, report ring, input mute
/// (`docs/api/usb/usb-snapshot.md`).
pub const S_NEWTOS_USBSNAP: u16 = 2022;

/// `Ssystem(WORD mode, LONG arg1, LONG arg2)`: `mode` in `x0`, `arg1` in
/// `x1`, `arg2` in `x2`.
pub fn ssystem(args: &TrapArgs) -> Long {
    // `mode == -1` is the "does this call exist" feature probe. FreeMiNT's
    // `sys_s_system` pre-sets `r = E_OK` (`sys/ssystem.c:66`) and `case -1:`
    // only TRACEs + `break`s (`sys/ssystem.c:72-76`), so it falls through to
    // `return r;` with `r` still 0 — `E_OK` unconditionally, for exactly
    // `-1` (not any negative mode). Without this, a probe hits the wildcard
    // arm, sees `EINVFN`, and wrongly concludes `Ssystem` is absent (D-072).
    if args.arg_i16(0) == -1 {
        return E_OK;
    }
    match args.arg_u16(0) {
        S_GETCOOKIE => s_getcookie(args),
        S_SETCOOKIE => s_setcookie(args),
        S_DELCOOKIE => s_delcookie(args),
        S_NEWTOS_CPUCLOCK => {
            newtos_scalar(args, || host::host()?.cpu_clock_hz().map(|hz| hz as Long))
        }
        S_NEWTOS_CPUTEMP => {
            newtos_scalar(args, || host::host()?.cpu_temperature_mdeg())
        }
        S_NEWTOS_KBDVEC => s_newtos_kbdvec(args),
        S_NEWTOS_RESET => crate::bios::reset::s_newtos_reset(args),
        S_NEWTOS_KVERSION => s_newtos_kversion(args),
        S_NEWTOS_USBSNAP => s_newtos_usbsnap(args),
        mode @ (S_NEWTOS_CPUMEAS..=S_NEWTOS_SERIAL | S_NEWTOS_INPUTMODE) => {
            newtos_scalar(args, || sysinfo_value(mode))
        }
        _ => EINVFN,
    }
}

/// The value of one scalar system-information mode (2003–2018, 2020), or `None`
/// when this machine does not know it (`docs/api/gemdos/154-ssystem.md`
/// §"System-information modes"). Every answer is the state the kernel runs
/// with — read from the module that holds it, or from the host for facts that
/// live in the kernel's drivers and boot decisions.
fn sysinfo_value(mode: u16) -> Option<Long> {
    use crate::bdos::mem::init as pools;
    use crate::bdos::process::{memprot, mmu};
    let kib = |bytes: u64| (bytes / 1024) as Long;
    match mode {
        S_NEWTOS_CPUMEAS => host::host()?.cpu_clock_measured_hz().map(|hz| hz as Long),
        S_NEWTOS_CPUPOLICY => host::host()?.cpu_clock_policy().map(|p| p as Long),
        S_NEWTOS_MEMPROT => Some(match (memprot::enabled(), memprot::strict()) {
            (false, _) => 0,
            (true, false) => 1,
            (true, true) => 2,
        }),
        S_NEWTOS_ASID => Some(if !mmu::asids_enabled() {
            0
        } else if mmu::asid_stats().3 == mmu::ASID_TEST_LIMIT {
            2
        } else {
            1
        }),
        S_NEWTOS_SDMODE => host::host()?.sd_card_mode().map(|m| m.compat as Long),
        S_NEWTOS_SDBUS => host::host()?.sd_card_mode().map(|m| m.bus_width as Long),
        S_NEWTOS_BCACHE => Some(kib(crate::bios::bcache::arena_bytes() as u64)),
        S_NEWTOS_BCACHEPCT => crate::bios::bcache::stats().map(|s| s.pct as Long),
        S_NEWTOS_STRAM | S_NEWTOS_TTRAM => {
            if !pools::is_initialised() {
                return None;
            }
            let (start, end) =
                if mode == S_NEWTOS_STRAM { pools::stram_range() } else { pools::ttram_range() };
            Some(kib((end as u32).wrapping_sub(start as u32) as u64))
        }
        S_NEWTOS_SCRWIDTH => host::host()?.screen().map(|s| s.width as Long),
        S_NEWTOS_SCRHEIGHT => host::host()?.screen().map(|s| s.height as Long),
        S_NEWTOS_SCRDEPTH => host::host()?.screen().map(|s| s.bpp as Long),
        S_NEWTOS_GPUMEM => host::host()?.gpu_memory_bytes().map(kib),
        S_NEWTOS_KEYBOARD => host::host()?.keyboard_layout().map(|k| k as Long),
        S_NEWTOS_SERIAL => host::host()?.serial_console().map(Long::from),
        S_NEWTOS_INPUTMODE => host::host()?.input_mode().map(|m| m as Long),
        _ => None,
    }
}

/// `S_NEWTOS_KVERSION` (2019): `arg1` = the caller's buffer, `arg2` = its
/// size in bytes (signed). FreeMiNT's `S_KNAME` rule (`sys/ssystem.c:572-576`
/// → `strncpy_f`, `sys/libkern/strncpy_f.c:47-56`): at most `len - 1`
/// characters and always a NUL; `len <= 0` writes nothing and is `E_OK`. A
/// null buffer with `len > 0` is `EINVAL` (FreeMiNT would write to address 0).
/// The string is the host's OS version and the build stamp, e.g.
/// `"NewTOS 0.6.1 2026-10-07 (c50a9c9)"`; `ERROR` without one.
fn s_newtos_kversion(args: &TrapArgs) -> Long {
    let buf = args.arg_u64(1);
    let len = args.arg_i32(2);
    let Some(name) = host::host().and_then(|h| h.os_version()) else {
        return ERROR;
    };
    if len <= 0 {
        return E_OK;
    }
    if buf == 0 {
        return EINVAL;
    }
    let mut text = [0u8; 96];
    let mut n = 0;
    for part in [name.as_bytes(), b" ", crate::build::BUILD_STAMP.as_bytes()] {
        let take = part.len().min(text.len() - 1 - n);
        text[n..n + take].copy_from_slice(&part[..take]);
        n += take;
    }
    let n = n.min(len as usize - 1);
    // The NUL right after the copied characters (the cut may fall inside
    // the text).
    text[n] = 0;
    if args.space.copy_to(buf, &text[..n + 1]).is_err() {
        return crate::uaccess::FAULTED;
    }
    E_OK
}

/// `S_NEWTOS_USBSNAP` (2022): `arg1` = the caller's buffer, `arg2` = its
/// size in bytes (signed). The buffer starts with a 32-byte request header;
/// the answer is written over it (`docs/api/usb/usb-snapshot.md` §2.1–§2.3).
///
/// Checks in the order of §2.3, the first that applies wins:
/// 1. `buf == 0` or `len < 32` → `EINVAL`, nothing written;
/// 2. magic not `"USBS"` → `EINVAL`, nothing written;
/// 3. `version != 1`, 4. `op` not 1–4 → `EINVAL` with a header that says
///    `version = 1`, `used_len = 32` (a later program learns it is talking
///    to an older kernel, the rule of [`newtos_scalar`]);
/// 5. no USB controller → `ERROR`, nothing written;
/// 6. index out of range, 7. a MUTE argument refused → `EINVAL`, header;
/// 8. the answer does not fit → `E_OK`, header only, `TRUNCATED`;
/// 9. otherwise `E_OK`, header and body.
///
/// 5–9 are the host's ([`host::Host::usb_snapshot`]); the body arrives in
/// chunks and goes straight into the caller's buffer behind the header,
/// which is written last. A bad pointer ends the calling program
/// ([`crate::uaccess::FAULTED`]), as for `S_GETCOOKIE` and 2019.
fn s_newtos_usbsnap(args: &TrapArgs) -> Long {
    const HDR: usize = 32;
    const VERSION: u16 = 1;
    const FLAG_TRUNCATED: u16 = 1 << 0;
    const FLAG_MORE: u16 = 1 << 1;

    let buf = args.arg_u64(1);
    let len = args.arg_i32(2);
    if buf == 0 || len < HDR as i32 {
        return EINVAL;
    }
    let mut req = [0u8; HDR];
    if args.space.copy_from(&mut req, buf).is_err() {
        return crate::uaccess::FAULTED;
    }
    if &req[0..4] != b"USBS" {
        return EINVAL;
    }
    let le16 = |o: usize| u16::from_le_bytes([req[o], req[o + 1]]);
    let le32 = |o: usize| u32::from_le_bytes([req[o], req[o + 1], req[o + 2], req[o + 3]]);
    let (version, op, index, arg) = (le16(4), le16(8), le32(12), le32(16));
    let now = crate::bios::tick::hz_200();

    // The answer header: the request echoed, with what the kernel used.
    let header = |flags: u16, total: u32, used: u32| {
        let mut h = [0u8; HDR];
        h[0..4].copy_from_slice(b"USBS");
        h[4..6].copy_from_slice(&VERSION.to_le_bytes());
        h[6..8].copy_from_slice(&(HDR as u16).to_le_bytes());
        h[8..10].copy_from_slice(&op.to_le_bytes());
        h[10..12].copy_from_slice(&flags.to_le_bytes());
        h[12..16].copy_from_slice(&index.to_le_bytes());
        h[16..20].copy_from_slice(&arg.to_le_bytes());
        h[20..24].copy_from_slice(&total.to_le_bytes());
        h[24..28].copy_from_slice(&used.to_le_bytes());
        h[28..32].copy_from_slice(&now.to_le_bytes());
        h
    };
    let put_header = |h: [u8; HDR]| -> Result<(), ()> {
        args.space.copy_to(buf, &h).map_err(|_| ())
    };
    let refuse = |rc: Long| -> Long {
        match put_header(header(0, HDR as u32, HDR as u32)) {
            Ok(()) => rc,
            Err(()) => crate::uaccess::FAULTED,
        }
    };

    if version != VERSION || !(1..=4).contains(&op) {
        return refuse(EINVAL);
    }
    let Some(h) = host::host() else {
        return ERROR;
    };
    let room = len as u32 - HDR as u32;
    let request = host::UsbSnapRequest { op, index, arg, now_hz200: now, room };
    let mut off: u32 = 0;
    let mut faulted = false;
    let mut overrun = false;
    let answer = h.usb_snapshot(&request, &mut |chunk: &[u8]| {
        // A host never writes past the caller's buffer, whatever it says.
        if chunk.len() as u64 > u64::from(room - off) {
            overrun = true;
            return false;
        }
        if args.space.copy_to(buf + HDR as u64 + u64::from(off), chunk).is_err() {
            faulted = true;
            return false;
        }
        off += chunk.len() as u32;
        true
    });
    if faulted {
        return crate::uaccess::FAULTED;
    }
    if overrun {
        return ERROR;
    }
    match answer {
        host::UsbSnapAnswer::NoController => ERROR,
        host::UsbSnapAnswer::Invalid => refuse(EINVAL),
        host::UsbSnapAnswer::Faulted => crate::uaccess::FAULTED,
        host::UsbSnapAnswer::Done { total, written, truncated, more } => {
            if written != off {
                return ERROR;
            }
            let flags = if truncated { FLAG_TRUNCATED } else { 0 } | if more { FLAG_MORE } else { 0 };
            let total_len = (HDR as u32).saturating_add(total);
            match put_header(header(flags, total_len, HDR as u32 + written)) {
                Ok(()) => E_OK,
                Err(()) => crate::uaccess::FAULTED,
            }
        }
    }
}

/// `S_GETCOOKIE` (8). `arg1` = tag, `arg2` = pointer to a LONG (or 0).
/// `tag == 0` → jar **capacity** (total slot count, not the occupied
/// count); small `tag` (high word 0) → the 1-based Nth cookie's tag
/// (enumeration); otherwise → the tag's value. With `arg2 == 0` the result
/// is returned in `x0`; otherwise it is written to `*arg2` and `E_OK` is
/// returned. Not-found differs by branch: a missing tag → `ERROR`
/// (FreeMiNT `EERROR`, `sys/cookie.c:508-509`), an out-of-range slot index
/// → `EINVAL` (`sys/cookie.c:458-462`).
fn s_getcookie(args: &TrapArgs) -> Long {
    let tag = args.arg_i32(1) as u32;
    let ret = args.arg_u64(2);

    // Each branch carries the error code to use if it yields `None`, so the
    // slot-index-overflow (`EINVAL`) and tag-miss (`ERROR`) cases stay
    // distinct (D-073). The `tag == 0` branch always returns `Some`.
    let (value, not_found) = if tag == 0 {
        (Some(cookie::capacity()), ERROR)
    } else if tag & 0xFFFF_0000 == 0 {
        (cookie::slot_tag(tag), EINVAL)
    } else {
        (cookie::get(tag), ERROR)
    };

    match value {
        Some(v) if ret != 0 => {
            // `arg2` is written in the caller's space (ARCH03 3a,
            // `crate::uaccess`): a program's pointer with its rights — a bad
            // one ends the call and the program, not the kernel — an
            // in-kernel caller's as the kernel address it is.
            if args.space.put_u32(ret, v).is_err() {
                return crate::uaccess::FAULTED;
            }
            E_OK
        }
        Some(v) => v as Long,
        None => not_found,
    }
}

/// `S_SETCOOKIE` (9). `arg1` = tag, `arg2` = value. Add or update. An
/// invalid tag (any zero byte) → `EINVAL` (`sys/cookie.c:538-541`); a full
/// jar → `ENSMEM` (FreeMiNT `ENOMEM`, `sys/cookie.c:574-575`).
fn s_setcookie(args: &TrapArgs) -> Long {
    let tag = args.arg_i32(1) as u32;
    let value = args.arg_i32(2) as u32;
    if !cookie::valid_tag(tag) {
        return EINVAL;
    }
    if cookie::set(tag, value) {
        E_OK
    } else {
        ENSMEM // jar full
    }
}

/// `S_DELCOOKIE` (26). `arg1` = tag. Remove it. A tag that is not present →
/// `EINVAL` (`sys/cookie.c:606-607`) — deliberately distinct from
/// `S_GETCOOKIE`'s `ERROR` for a missing tag.
fn s_delcookie(args: &TrapArgs) -> Long {
    let tag = args.arg_i32(1) as u32;
    if cookie::del(tag) {
        E_OK
    } else {
        EINVAL
    }
}

/// The shared body of the NewTOS-own scalar subfunctions (2000 upwards).
///
/// Argument convention — FreeMiNT's own, where `arg1` is the caller's buffer
/// and `arg2` its length (`S_GETBOOTLOG` 29, `S_KNAME` 900,
/// `sys/mint/ssystem.h`), narrowed to a single LONG:
///
/// * `arg1` — a user pointer to a LONG that receives the value, or **0** to
///   have the value returned in the call's own LONG. The pointer form is the
///   unambiguous one: it carries all 32 bits, while a returned LONG cannot
///   tell a value of `0xFFFF_FFFF` from `ERROR`. (For a Pi 4 neither reading
///   can reach that far — its ARM clock tops out at 2.0 GHz and its die
///   temperature at some tens of degrees — but the contract, not the current
///   board, decides which form a caller should use.)
/// * `arg2` — reserved for a future selector (a second clock, a second
///   sensor); it must be **0** today. Anything else is `EINVAL` rather than
///   silently ignored, so a program written against a later, wider version
///   of this call is told it is talking to an older kernel instead of
///   receiving the wrong quantity.
///
/// `read` answers `None` when this board cannot measure the value — then the
/// call is `ERROR` and nothing is written through `arg1`. That is the whole
/// point of routing through [`crate::host::Host`]: a board with no sensor
/// says so.
fn newtos_scalar(args: &TrapArgs, read: impl FnOnce() -> Option<Long>) -> Long {
    if args.arg_u64(2) != 0 {
        return EINVAL;
    }
    let Some(value) = read() else {
        return ERROR;
    };
    let out = args.arg_u64(1);
    if out == 0 {
        return value;
    }
    // `arg1` is written in the caller's space — the same rule as
    // `s_getcookie`'s `arg2`.
    if args.space.put_u32(out, value as u32).is_err() {
        return crate::uaccess::FAULTED;
    }
    E_OK
}

/// `S_NEWTOS_KBDVEC` (2002): `arg1` = the IKBD key byte (bit 7 = break, as
/// `kbdvec` receives it in `x0`), `arg2` reserved, must be 0. Runs the default
/// `kbdvec` — EmuTOS `_kbdvec` → `kbd_int(scancode)` (`bios/aciavecs.S:
/// 372-376`, `bios/ikbd.c:725-910`) — through the host's key decoder:
/// modifier state, the key word into the console queue, the key repeat.
///
/// * `E_OK` — the byte was decoded;
/// * `EINVAL` — `arg1` is not a byte (`> 0xFF`) or `arg2 != 0`; nothing ran;
/// * `ERROR` — this host has no keyboard decoder (a hostless build).
///
/// Any process may call it; the effect is exactly that of calling the default
/// `kbdvec` it can already read from its own table, as on TOS.
fn s_newtos_kbdvec(args: &TrapArgs) -> Long {
    let byte = args.arg_u64(1);
    if byte > 0xFF || args.arg_u64(2) != 0 {
        return EINVAL;
    }
    match host::host() {
        Some(h) if h.kbd_int(byte as u8) => E_OK,
        _ => ERROR,
    }
}
