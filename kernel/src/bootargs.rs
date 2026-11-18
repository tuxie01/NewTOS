//! Boot-argument tokens (SER01).
//!
//! The boot firmware of a Raspberry Pi reads `cmdline.txt` from the boot
//! partition and hands its text to the kernel; NewTOS fetches it with the
//! VideoCore mailbox tag "Get command line"
//! (`drivers::bcm2711::mailbox::with_command_line`, source:
//! `references/circle/lib/koptions.cpp:52-61`). On QEMU `raspi4b` the same
//! tag answers the `-append` string.
//!
//! The text is a list of options separated by blanks
//! (`references/circle/doc/cmdline.txt:3-4`: *"Multiple options must be placed
//! on a single line delimited by a space character"*). The parser here splits
//! on any ASCII whitespace rather than on the space alone, so a `cmdline.txt`
//! saved with a trailing CR/LF or with tabs still yields clean tokens, and it
//! stops at a NUL. A token matches only as a whole: `serial=on` is found in
//! `quiet serial=on`, but not in `serial=onx` or `xserial=on`.
//!
//! Pure byte logic with no kernel dependencies, so the host test suite
//! (`tools/xtask`, which includes this file by path) can run its tests.

/// The token that switches the serial (UART0) output on, `board-rpi4`.
pub const SERIAL_ON: &[u8] = b"serial=on";

/// The token that switches memory protection off (ARCH03 3b) — FreeMiNT's
/// `MEM_PROT=NO` (`references/freemint/sys/bootmenu.c:207-219`). Without it
/// every process has its own rights (`PROT_P/G/S/PR/I`).
pub const MEMPROT_OFF: &[u8] = b"memprot=off";

/// The token that makes the parent-to-child rule strict (ARCH03 E5,
/// FreeMiNT `MPF_STRICT`, `references/freemint/sys/k_exec.c:799-806`): a
/// child no longer keeps access to its parent's private memory.
pub const MEMPROT_STRICT: &[u8] = b"memprot=strict";

/// The token that switches the memory-protection verification mode on
/// (MEMPROT-FAST): after every table sync the incrementally maintained
/// tables are compared with a complete recomputation, differences go to the
/// serial line. Combines with `memprot=strict` (a second token); without
/// effect under `memprot=off`. Slow — a debugging and test aid.
pub const MEMPROT_VERIFY: &[u8] = b"memprot=verify";

/// The token that switches per-process ASIDs on (ARCH03 stage 4,
/// `docs/analysis/arch03-implementation-plan.md` Stufe 4, E12): every root
/// is installed with an ASID of its own and an address-space switch drops no
/// TLB entries. Off without it (the TLBI-on-every-switch path).
pub const ASID_ON: &[u8] = b"asid=on";

/// The rollover test hook of stage 4: ASIDs on, but only four numbers per
/// generation (three usable), so that ASID generations roll over after
/// every few process starts — the path that otherwise needs 65 535 of them.
pub const ASID_TINY: &[u8] = b"asid=tiny";

/// The fallback switch of the SD driver (FS08): `emmc=compat` keeps the
/// microSD bus at 1 bit and moves every sector with its own single-block
/// command (`CMD17`/`CMD24`) — the driver as it was before FS08. Without it
/// the driver switches the card to the 4-bit bus (`ACMD6`) when its SCR
/// allows, and moves multi-sector requests with one `CMD18`/`CMD25`.
pub const EMMC_COMPAT: &[u8] = b"emmc=compat";

/// The fallback switch of the ARM clock (FS08 part 3): `cpuclk=firmware`
/// leaves the ARM core at the rate the boot firmware started it with. Without
/// it the kernel asks the firmware for the ARM clock's maximum rate at boot
/// and sets it (mailbox "Get max clock rate" / "Set clock rate").
pub const CPUCLK_FIRMWARE: &[u8] = b"cpuclk=firmware";

/// The fallback switch of the input pump (INPUT01): `input=aes` drains the
/// keyboard/mouse (USB-HID) **only** where it was drained before INPUT01 —
/// in the AES wait loops, the blocking console reads and `Bconstat` on an
/// empty queue (`Host::pump_input`). Without it the 200 Hz timer interrupt
/// additionally drains it whenever it interrupts a program at EL0 and moves
/// the pointer, so the pointer also moves while a program computes or
/// between its calls; buttons, wheel and keys keep their old moment
/// (`kernel::input_gate`).
pub const INPUT_AES: &[u8] = b"input=aes";

/// The size of the block read cache in KiB (FS09): `bcache=<KiB>`, `0` = off.
/// FreeMiNT's `CACHE=` / `FS_CACHE_SIZE=` (`references/freemint/sys/
/// cnf_mint.c:97,128`), but a boot argument: the cache's memory is carved off
/// the top of DRAM before the memory pools exist and before the card is read
/// (`docs/analysis/fs09-block-cache-2026-10-07.md` §2.4).
pub const BCACHE: &[u8] = b"bcache=";

/// The share of the block cache one linear read may fill, in percent (FS09):
/// `bcachepct=<0..100>` — FreeMiNT's `PERCENTAGE=` / `FS_CACHE_PERCENTAGE=`
/// (`cnf_mint.c:109,129`).
pub const BCACHE_PCT: &[u8] = b"bcachepct=";

/// The per-process limit of allocated memory blocks (MEM05):
/// `mdlimit=<n>`, `0` = no per-process limit (only the global budget of the
/// MD supply applies). Default
/// [`tos_api::bdos::mem::owner_limit::DEFAULT_OWNER_MD_LIMIT`] (8192, user
/// decision E3/E4, `docs/analysis/mem05-design-2026-10-13.md` §3.3). A boot
/// argument like [`BCACHE`], not an `Ssystem` call: no new run-time contract
/// surface, and a program cannot lift its own limit.
pub const MDLIMIT: &[u8] = b"mdlimit=";

/// What the boot arguments say about one numeric setting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    /// No token with this prefix: the built-in default applies.
    Default,
    /// The first such token, a decimal number within the allowed maximum.
    Given(u32),
    /// The first such token is not a decimal number, or above the maximum:
    /// the default applies, and the boot line says so.
    Invalid,
}

/// Look up the numeric setting `prefix<n>` (e.g. `bcache=8192`) in `args`.
///
/// Tokens are split as for [`has_token`]; the **first** token that starts with
/// `prefix` decides. `n` is ASCII decimal digits only (no sign, no unit, no
/// blanks), at most `max`.
pub fn numeric_setting(args: &[u8], prefix: &[u8], max: u32) -> Setting {
    if prefix.is_empty() {
        return Setting::Default;
    }
    let end = args.iter().position(|&b| b == 0).unwrap_or(args.len());
    let Some(tok) = args[..end]
        .split(|b| b.is_ascii_whitespace())
        .find(|t| t.starts_with(prefix))
    else {
        return Setting::Default;
    };
    let digits = &tok[prefix.len()..];
    if digits.is_empty() || digits.len() > 10 || !digits.iter().all(u8::is_ascii_digit) {
        return Setting::Invalid;
    }
    let v = digits.iter().fold(0u64, |a, &d| a * 10 + u64::from(d - b'0'));
    if v > u64::from(max) {
        Setting::Invalid
    } else {
        Setting::Given(v as u32)
    }
}

/// `true` if `args` contains `token` as a whole whitespace-delimited token.
///
/// `args` ends at its first NUL byte, if any. An empty `token` never
/// matches.
pub fn has_token(args: &[u8], token: &[u8]) -> bool {
    if token.is_empty() {
        return false;
    }
    let end = args.iter().position(|&b| b == 0).unwrap_or(args.len());
    args[..end]
        .split(|b| b.is_ascii_whitespace())
        .any(|t| t == token)
}

/// The VDI device depth (TASK/V62, ADR-038 point 2): `vdiplanes=<16|32>`,
/// the plane count `v_planes` the VDI and AES report (`vq_extnd`
/// `INQ_TAB[4]`, `ap_planes`). Absent = [`VDI_PLANES_DEFAULT`]; every other
/// value is [`Setting::Invalid`] and keeps the default, with a boot line
/// ([`vdi_planes`]). A boot argument like [`MDLIMIT`], not a call: a
/// program cannot change the depth, and there is no run-time switch.
/// Specification: `docs/api/vdi/workstation/100-v_opnvwk.md`, section
/// "32-plane setting and the boot argument `vdiplanes`".
pub const VDIPLANES: &[u8] = b"vdiplanes=";

/// The default VDI depth: 16 planes, the Falcon truecolor workstation of
/// ADR-024.
pub const VDI_PLANES_DEFAULT: u16 = 16;

/// The second VDI depth: 32 planes over the 32-bpp store (ADR-038 point 1).
pub const VDI_PLANES_WIDE: u16 = 32;

/// What the boot arguments say about the VDI depth ([`VDIPLANES`]).
///
/// [`numeric_setting`] with the prefix `vdiplanes=`; of the numbers it
/// accepts only 16 and 32 are depths ([`VDI_PLANES_DEFAULT`],
/// [`VDI_PLANES_WIDE`]), so every other number is [`Setting::Invalid`] as
/// well (ADR-038 point 2: "any other value counts as `Setting::Invalid`").
/// `Given(n)` therefore always holds 16 or 32.
pub fn vdi_planes(args: &[u8]) -> Setting {
    match numeric_setting(args, VDIPLANES, u32::from(VDI_PLANES_WIDE)) {
        Setting::Given(n) if n == u32::from(VDI_PLANES_DEFAULT) || n == u32::from(VDI_PLANES_WIDE) => {
            Setting::Given(n)
        }
        Setting::Given(_) => Setting::Invalid,
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memprot_tokens_are_whole_tokens() {
        assert!(has_token(b"serial=on memprot=off", MEMPROT_OFF));
        assert!(!has_token(b"serial=on memprot=offx", MEMPROT_OFF));
        assert!(has_token(b"memprot=strict\n", MEMPROT_STRICT));
        assert!(!has_token(b"memprot=off", MEMPROT_STRICT));
        assert!(has_token(b"serial=on memprot=strict memprot=verify", MEMPROT_VERIFY));
        assert!(!has_token(b"memprot=verifyx", MEMPROT_VERIFY));
    }

    #[test]
    fn asid_tokens_are_whole_tokens() {
        assert!(has_token(b"serial=on asid=on memprot=off", ASID_ON));
        assert!(!has_token(b"serial=on asid=only", ASID_ON));
        assert!(!has_token(b"asid=tiny", ASID_ON));
        assert!(has_token(b"asid=tiny\r\n", ASID_TINY));
        assert!(!has_token(b"asid=on", ASID_TINY));
    }

    #[test]
    fn input_aes_is_a_whole_token() {
        assert!(has_token(b"serial=on input=aes", INPUT_AES));
        assert!(has_token(b"input=aes\r\n", INPUT_AES));
        assert!(!has_token(b"input=aesx", INPUT_AES));
        assert!(!has_token(b"input=AES", INPUT_AES));
        assert!(!has_token(b"xinput=aes", INPUT_AES));
        assert!(!has_token(b"serial=on memprot=on", INPUT_AES));
    }

    #[test]
    fn emmc_compat_is_a_whole_token() {
        assert!(has_token(b"serial=on emmc=compat memprot=on", EMMC_COMPAT));
        assert!(has_token(b"emmc=compat\r\n", EMMC_COMPAT));
        assert!(!has_token(b"emmc=compatx", EMMC_COMPAT));
        assert!(!has_token(b"emmc=COMPAT", EMMC_COMPAT));
        assert!(!has_token(b"emmc=", EMMC_COMPAT));
        assert!(!has_token(b"serial=on memprot=on", EMMC_COMPAT));
    }

    #[test]
    fn cpuclk_firmware_is_a_whole_token() {
        assert!(has_token(b"serial=on cpuclk=firmware memprot=on", CPUCLK_FIRMWARE));
        assert!(has_token(b"cpuclk=firmware\r\n", CPUCLK_FIRMWARE));
        assert!(!has_token(b"cpuclk=firmwarex", CPUCLK_FIRMWARE));
        assert!(!has_token(b"cpuclk=FIRMWARE", CPUCLK_FIRMWARE));
        assert!(!has_token(b"cpuclk=", CPUCLK_FIRMWARE));
        assert!(!has_token(b"cpuclk=max", CPUCLK_FIRMWARE));
        assert!(!has_token(b"serial=on emmc=compat", CPUCLK_FIRMWARE));
    }

    #[test]
    fn numeric_settings_parse_strictly() {
        let max = 262_144;
        assert_eq!(numeric_setting(b"serial=on memprot=on", BCACHE, max), Setting::Default);
        assert_eq!(numeric_setting(b"serial=on bcache=8192", BCACHE, max), Setting::Given(8192));
        assert_eq!(numeric_setting(b"bcache=0\r\n", BCACHE, max), Setting::Given(0));
        assert_eq!(numeric_setting(b"bcache=262144", BCACHE, max), Setting::Given(262_144));
        assert_eq!(numeric_setting(b"bcache=262145", BCACHE, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"bcache=99999999999", BCACHE, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"bcache=", BCACHE, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"bcache=8M", BCACHE, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"bcache=-1", BCACHE, max), Setting::Invalid);
        // The first token decides; another prefix is not this one.
        assert_eq!(numeric_setting(b"bcache=16 bcache=32", BCACHE, max), Setting::Given(16));
        assert_eq!(numeric_setting(b"bcachepct=50", BCACHE, max), Setting::Default);
        assert_eq!(numeric_setting(b"bcachepct=50", BCACHE_PCT, 100), Setting::Given(50));
        assert_eq!(numeric_setting(b"bcachepct=101", BCACHE_PCT, 100), Setting::Invalid);
        assert_eq!(numeric_setting(b"xbcache=5", BCACHE, max), Setting::Default);
        assert_eq!(numeric_setting(b"quiet\0bcache=5", BCACHE, max), Setting::Default);
        assert_eq!(numeric_setting(b"bcache=5", b"", max), Setting::Default);
    }

    #[test]
    fn mdlimit_parses_like_the_other_numeric_settings() {
        let max = u32::MAX;
        assert_eq!(numeric_setting(b"serial=on memprot=on", MDLIMIT, max), Setting::Default);
        assert_eq!(numeric_setting(b"serial=on mdlimit=64", MDLIMIT, max), Setting::Given(64));
        assert_eq!(numeric_setting(b"mdlimit=0\r\n", MDLIMIT, max), Setting::Given(0));
        assert_eq!(numeric_setting(b"mdlimit=4294967295", MDLIMIT, max), Setting::Given(u32::MAX));
        assert_eq!(numeric_setting(b"mdlimit=4294967296", MDLIMIT, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"mdlimit=99999999999", MDLIMIT, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"mdlimit=", MDLIMIT, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"mdlimit=8k", MDLIMIT, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"mdlimit=-1", MDLIMIT, max), Setting::Invalid);
        assert_eq!(numeric_setting(b"xmdlimit=5", MDLIMIT, max), Setting::Default);
    }

    #[test]
    fn finds_the_token_alone_and_among_others() {
        assert!(has_token(b"serial=on", SERIAL_ON));
        assert!(has_token(b"quiet serial=on", SERIAL_ON));
        assert!(has_token(b"serial=on quiet", SERIAL_ON));
        assert!(has_token(b"a  serial=on  b", SERIAL_ON));
    }

    #[test]
    fn accepts_tabs_and_line_ends_as_separators() {
        assert!(has_token(b"quiet\tserial=on\r\n", SERIAL_ON));
        assert!(has_token(b"serial=on\n", SERIAL_ON));
    }

    #[test]
    fn rejects_partial_and_different_tokens() {
        assert!(!has_token(b"serial=onx", SERIAL_ON));
        assert!(!has_token(b"xserial=on", SERIAL_ON));
        assert!(!has_token(b"serial=off", SERIAL_ON));
        assert!(!has_token(b"serial", SERIAL_ON));
        assert!(!has_token(b"SERIAL=ON", SERIAL_ON));
        assert!(!has_token(b"serial = on", SERIAL_ON));
    }

    #[test]
    fn empty_input_and_empty_token_never_match() {
        assert!(!has_token(b"", SERIAL_ON));
        assert!(!has_token(b"   ", SERIAL_ON));
        assert!(!has_token(b"serial=on", b""));
    }

    #[test]
    fn stops_at_the_first_nul() {
        assert!(has_token(b"serial=on\0garbage", SERIAL_ON));
        assert!(!has_token(b"quiet\0serial=on", SERIAL_ON));
    }

    /// The result table of `docs/api/vdi/workstation/100-v_opnvwk.md`
    /// ("32-plane setting", §1): absent = `Default`, 16 and 32 are
    /// `Given`, everything else is `Invalid` (TASK/V62, ADR-038 point 2).
    #[test]
    fn vdiplanes_accepts_16_and_32_only() {
        assert_eq!(vdi_planes(b"serial=on memprot=on"), Setting::Default);
        assert_eq!(vdi_planes(b""), Setting::Default);
        assert_eq!(vdi_planes(b"vdiplanes=16"), Setting::Given(16));
        assert_eq!(vdi_planes(b"serial=on vdiplanes=32\r\n"), Setting::Given(32));
        // Not a number.
        assert_eq!(vdi_planes(b"vdiplanes="), Setting::Invalid);
        assert_eq!(vdi_planes(b"vdiplanes=abc"), Setting::Invalid);
        assert_eq!(vdi_planes(b"vdiplanes=32x"), Setting::Invalid);
        assert_eq!(vdi_planes(b"vdiplanes=-16"), Setting::Invalid);
        // A number, but not a depth: below and above 32 alike.
        for bad in [&b"vdiplanes=0"[..], b"vdiplanes=1", b"vdiplanes=4", b"vdiplanes=8", b"vdiplanes=15",
            b"vdiplanes=24", b"vdiplanes=31", b"vdiplanes=33", b"vdiplanes=64", b"vdiplanes=99999999999"]
        {
            assert_eq!(vdi_planes(bad), Setting::Invalid, "{}", core::str::from_utf8(bad).unwrap());
        }
        // Leading zeros are digits (`numeric_setting`): 032 is 32.
        assert_eq!(vdi_planes(b"vdiplanes=032"), Setting::Given(32));
        // The first token decides; another prefix is not this one.
        assert_eq!(vdi_planes(b"vdiplanes=32 vdiplanes=16"), Setting::Given(32));
        assert_eq!(vdi_planes(b"vdiplanes=8 vdiplanes=32"), Setting::Invalid);
        assert_eq!(vdi_planes(b"xvdiplanes=32"), Setting::Default);
        assert_eq!(vdi_planes(b"quiet\0vdiplanes=32"), Setting::Default);
    }
}
