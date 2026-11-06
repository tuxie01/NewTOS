//! BCM2711 EMMC2 (Arasan/iproc SDHCI) microSD driver.
//!
//! Reference: [`docs/hardware/emmc-sd.md`] — the on-board microSD card
//! slot (EMMC2 @ `0xFE34_0000`), the SD Host Controller + SD Physical
//! Layer specs. PIO data transfers (no DMA), polled (no IRQ). First test
//! vehicle is QEMU `raspi4b`'s `emmc` device.
//!
//! **Transfer modes (FS08).** By default a request of more than one sector
//! goes to the card as **one** `CMD18`/`CMD25` with Block Count and Auto
//! CMD12 (SD-HC 3.00 §3.7.2.1, §1.11), and after identification the bus is
//! switched to **4 bit** (`ACMD6` + Host Control 1 `Data Transfer Width`,
//! SD-HC 3.00 §3.4) when the card's SCR says it can. The boot argument
//! `emmc=compat` ([`crate::bootargs::EMMC_COMPAT`], applied through
//! [`set_compat`] before [`init`]) keeps the pre-FS08 behaviour: 1-bit bus,
//! one `CMD17`/`CMD24` per sector. The pure planning arithmetic lives in
//! [`super::emmc_xfer`] so the host suite can test it.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

#[cfg(feature = "sd-diag")]
use super::emmc_diag;
use super::emmc_xfer::{self as xfer, Transfer, TM_DAT_CARD2HOST, TM_MULTI};
use crate::hal::block::{BlockDevice, BlockError, SECTOR_SIZE};

/// SD-host controller candidate bases (low-peripheral mode). On real
/// RPi4 the microSD slot is **EMMC2** (`0xFE34_0000`); QEMU `raspi4b`
/// wires its emulated card to the legacy **EMMC** SDHCI (`0xFE30_0000`).
/// Both are standard SDHCI, so one driver serves both — `init()` probes
/// EMMC2 first (real HW), then EMMC (QEMU).
const EMMC2_BASE: usize = crate::drivers::mmio_va(0xFE34_0000);
const EMMC_BASE: usize = crate::drivers::mmio_va(0xFE30_0000);

/// The controller base the driver is currently bound to.
static BASE_ADDR: AtomicUsize = AtomicUsize::new(EMMC2_BASE);

fn base() -> usize {
    BASE_ADDR.load(Ordering::Relaxed)
}

// Register offsets (datasheet §5 grouping; see the Track-B reference).
const BLKSIZECNT: usize = 0x04;
const ARG1: usize = 0x08;
const CMDTM: usize = 0x0C;
const RESP0: usize = 0x10;
const RESP1: usize = 0x14;
const RESP2: usize = 0x18;
const RESP3: usize = 0x1C;
const DATA: usize = 0x20;
const STATUS: usize = 0x24;
const CONTROL0: usize = 0x28;
const CONTROL1: usize = 0x2C;
const INTERRUPT: usize = 0x30;
const IRPT_MASK: usize = 0x34;
const IRPT_EN: usize = 0x38;
/// `CONTROL2`: low half = **Auto CMD Error Status** (SD-HC 3.00 §2.2.23,
/// offset 03Ch), high half = Host Control 2 (03Eh) — `docs/hardware/emmc-sd.md`
/// §1.
const CONTROL2: usize = 0x3C;
const CAPABILITIES: usize = 0x40;
const SLOTISR_VER: usize = 0xFC;

// STATUS (present state) bits.
const SR_CMD_INHIBIT: u32 = 1 << 0;
const SR_DAT_INHIBIT: u32 = 1 << 1;

// CONTROL1 bits.
const C1_CLK_INTLEN: u32 = 1 << 0;
const C1_CLK_STABLE: u32 = 1 << 1;
const C1_CLK_EN: u32 = 1 << 2;
const C1_TOUNIT_MAX: u32 = 0xE << 16;
const C1_SRST_HC: u32 = 1 << 24;
/// `Software Reset For CMD Line` — resets the controller's command circuit.
/// SD Host Controller Simplified Spec 3.00 §2.2.15; same bit in the BCM
/// Arasan block (`docs/hardware/emmc-sd.md` §1 "CONTROL1", bit 25) and in
/// the pinned RPi4 reference (`references/circle/addon/SDCard/emmc.cpp:523`).
const C1_SRST_CMD: u32 = 1 << 25;
/// `Software Reset For DAT Line` — resets the controller's data circuit.
/// Same sources as [`C1_SRST_CMD`] (bit 26, `emmc.cpp:524`).
const C1_SRST_DATA: u32 = 1 << 26;

// INTERRUPT bits.
const INT_CMD_DONE: u32 = 1 << 0;
const INT_DATA_DONE: u32 = 1 << 1;
const INT_WRITE_RDY: u32 = 1 << 4;
const INT_READ_RDY: u32 = 1 << 5;
/// `Card Interrupt` (Normal Interrupt Status bit 8, SD-HC 3.00 §2.2.17).
/// Its status enable in `IRPT_MASK` is switched off while the bus width
/// changes — SD-HC 3.00 §3.4 steps (1) and (8).
const INT_CARD: u32 = 1 << 8;
const INT_ERROR_MASK: u32 = 0xFFFF_0000;

// INTERRUPT error bits (upper half), SD-HC 3.00 §2.2.18 "Error Interrupt
// Status". Bit positions cross-checked against the pinned RPi4 reference
// (`references/circle/addon/SDCard/emmc.cpp:153-165`) and against the set
// the FS04 audit asks the driver to report instead of silently clearing
// (`docs/api/hw/emmc-audit.md` §5 M2: 16 CTO, 17 CCRC, 20 DTO, 21 DCRC,
// 22 DEND).
const INT_ERR_CTO: u32 = 1 << 16; // Command Timeout Error
const INT_ERR_CCRC: u32 = 1 << 17; // Command CRC Error
const INT_ERR_CEND: u32 = 1 << 18; // Command End Bit Error
const INT_ERR_CBAD: u32 = 1 << 19; // Command Index Error
const INT_ERR_DTO: u32 = 1 << 20; // Data Timeout Error
const INT_ERR_DCRC: u32 = 1 << 21; // Data CRC Error
const INT_ERR_DEND: u32 = 1 << 22; // Data End Bit Error
/// `Auto CMD Error` — Error Interrupt Status bit 8 (SD-HC 3.00 §2.2.18),
/// bit 24 of the BCM's 32-bit `INTERRUPT` word: the Auto CMD12 that ends a
/// multiple-block transfer failed, details in [`CONTROL2`] (§2.2.23).
const INT_ERR_ACMD: u32 = 1 << 24;

// Auto CMD Error Status (SD-HC 3.00 §2.2.23, Table 2-30).
/// Bit 0 `Auto CMD12 Not Executed`: the controller could not issue the
/// Auto CMD12 at all; bits 4:1 are then meaningless.
const ACMD_NOT_EXECUTED: u32 = 1 << 0;

// ── R1 card status (SD Phys 9.10 Table 4-42, `docs/hardware/emmc-sd.md`
// §5) ────────────────────────────────────────────────────────────────
//
// An R1 response carries the card's own verdict on the command. The
// controller's `INTERRUPT` register says nothing about it: a command the
// card *refuses* still completes on the bus without a single error bit
// (audit finding 3 drove exactly that — a write past the end of the card
// returned `Ok(())` with `INTERRUPT = 0`).
//
// Listed below are the entries whose *Type* column in Table 4-42 contains
// `E` (error bit); status entries (`CURRENT_STATE`, `READY_FOR_DATA`,
// `APP_CMD`, `CARD_IS_LOCKED`, `CARD_ECC_DISABLED`, `ERASE_RESET`,
// `FX_EVENT`) are deliberately **not** here — they are not failures.
//
// Each comment below says in our own words what the bit means *for this
// driver*, not what the specification says about it; the long form, with
// the per-command cross-reference and the error-code mapping, is in
// `docs/hardware/emmc-sd.md` §5. All fifteen share one baseline:
// `check_r1()` tests them as a single mask, so any of them turns the call
// into `Err(BlockError::Card)` → `EREADF` / `EWRITF`; the per-bit counter
// in `ErrorStats` is diagnostic only.

/// Bit 31 — the block address we sent lies outside what the card accepts.
/// One of the two bits a `CMD17`/`CMD24` response realistically carries,
/// and the reason `check_r1()` exists at all: before it, a write past the
/// end of the card returned `Ok(())`. SD Phys 9.10 §4.10.1.
const R1_OUT_OF_RANGE: u32 = 1 << 31;
/// Bit 30 — the card rejected our address against its block length. The
/// other bit Table 4-43 lists for `CMD17`/`CMD24`; `card_address()` cannot
/// produce a misaligned argument, so this fires only if the card disagrees
/// with our own arithmetic.
const R1_ADDRESS_ERROR: u32 = 1 << 30;
/// Bit 29 — the card rejected the transfer length. We always request
/// exactly `SECTOR_SIZE` and pin an SDSC card to 512 bytes during init, so
/// nothing in our own request should reach this.
const R1_BLOCK_LEN_ERROR: u32 = 1 << 29;
/// Bit 28 — the card's erase sequence went wrong. We issue no erase
/// command, so this cannot arise from our own traffic; it stays in the
/// mask so a card that sets it anyway is still treated as a failure.
const R1_ERASE_SEQ_ERROR: u32 = 1 << 28;
/// Bit 27 — the card rejected an erase block range. Unreachable for the
/// same reason as `R1_ERASE_SEQ_ERROR`: we never erase.
const R1_ERASE_PARAM: u32 = 1 << 27;
/// Bit 26 — a write hit protected storage. Reachable through `CMD24`, our
/// only write command; like EmuTOS' own SD driver we fold it into the
/// generic `EWRITF` instead of a write-protect-specific code.
const R1_WP_VIOLATION: u32 = 1 << 26;
/// Bit 24 — the card's lock/unlock feature refused the request. We never
/// issue that command, so this is unreachable in practice.
const R1_LOCK_UNLOCK_FAILED: u32 = 1 << 24;
/// Bit 23 — the *card* found the command it received corrupted. Distinct
/// from `INT_ERR_CCRC`, which is the *controller's* verdict on the same
/// kind of failure and is handled earlier, in `wait_interrupt()`.
const R1_COM_CRC_ERROR: u32 = 1 << 23;
/// Bit 22 — the card refuses the command in the state it is currently in.
/// Reachable on either of our two data commands; no special handling.
const R1_ILLEGAL_COMMAND: u32 = 1 << 22;
/// Bit 21 — the card's internal ECC could not repair the data it read.
/// Reachable on `CMD17`; we neither retry nor report it separately.
const R1_CARD_ECC_FAILED: u32 = 1 << 21;
/// Bit 20 — the card's own controller failed internally. Generic and
/// reachable on either command, with no special handling.
const R1_CC_ERROR: u32 = 1 << 20;
/// Bit 19 — the card's catch-all rejection, given when it has no finer
/// reason to report. Beyond its own counter it is indistinguishable here
/// from any other bit in this list.
const R1_ERROR: u32 = 1 << 19;
/// Bit 16 — an illegal attempt to change a read-only CSD field or undo a
/// permanent write-protect bit. We issue none of the programming commands
/// this covers, so it is unreachable via `CMD17`/`CMD24`.
const R1_CSD_OVERWRITE: u32 = 1 << 16;
/// Bit 15 — an erase left write-protected blocks untouched. Unreachable
/// like the other two erase-family bits: we never erase.
const R1_WP_ERASE_SKIP: u32 = 1 << 15;
/// Bit 3 — the card's authentication sequence went wrong. We run no
/// authentication, so this is unreachable.
const R1_AKE_SEQ_ERROR: u32 = 1 << 3;

/// Every Type-`E` bit of Table 4-42 — a non-zero AND means the card
/// refused or failed the command.
const R1_ERROR_MASK: u32 = R1_OUT_OF_RANGE
    | R1_ADDRESS_ERROR
    | R1_BLOCK_LEN_ERROR
    | R1_ERASE_SEQ_ERROR
    | R1_ERASE_PARAM
    | R1_WP_VIOLATION
    | R1_LOCK_UNLOCK_FAILED
    | R1_COM_CRC_ERROR
    | R1_ILLEGAL_COMMAND
    | R1_CARD_ECC_FAILED
    | R1_CC_ERROR
    | R1_ERROR
    | R1_CSD_OVERWRITE
    | R1_WP_ERASE_SKIP
    | R1_AKE_SEQ_ERROR;

/// The R1 error bits the driver counts by name, in [`R1_BIT_NAMES`] order.
const R1_BIT_MASKS: [u32; 15] = [
    R1_OUT_OF_RANGE,
    R1_ADDRESS_ERROR,
    R1_BLOCK_LEN_ERROR,
    R1_ERASE_SEQ_ERROR,
    R1_ERASE_PARAM,
    R1_WP_VIOLATION,
    R1_LOCK_UNLOCK_FAILED,
    R1_COM_CRC_ERROR,
    R1_ILLEGAL_COMMAND,
    R1_CARD_ECC_FAILED,
    R1_CC_ERROR,
    R1_ERROR,
    R1_CSD_OVERWRITE,
    R1_WP_ERASE_SKIP,
    R1_AKE_SEQ_ERROR,
];

/// Names of the counted R1 error bits, in [`R1_BIT_MASKS`] order.
pub const R1_BIT_NAMES: [&str; 15] = [
    "OUT_OF_RANGE",
    "ADDRESS_ERROR",
    "BLOCK_LEN_ERROR",
    "ERASE_SEQ_ERROR",
    "ERASE_PARAM",
    "WP_VIOLATION",
    "LOCK_UNLOCK_FAILED",
    "COM_CRC_ERROR",
    "ILLEGAL_COMMAND",
    "CARD_ECC_FAILED",
    "CC_ERROR",
    "ERROR",
    "CSD_OVERWRITE",
    "WP_ERASE_SKIP",
    "AKE_SEQ_ERROR",
];

/// The error bits that belong to the **command** line — SD-HC §3.10.1 asks
/// for `Software Reset For CMD Line` when one of these is set.
const INT_ERR_CMD_MASK: u32 = INT_ERR_CTO | INT_ERR_CCRC | INT_ERR_CEND | INT_ERR_CBAD;
/// The error bits that belong to the **data** line — SD-HC §3.10.1 asks for
/// `Software Reset For DAT Line` when one of these is set.
const INT_ERR_DAT_MASK: u32 = INT_ERR_DTO | INT_ERR_DCRC | INT_ERR_DEND;

// CMDTM command-word flags (upper half).
const RSP_NONE: u32 = 0;
const RSP_136: u32 = 1 << 16;
const RSP_48: u32 = 2 << 16;
const RSP_48BUSY: u32 = 3 << 16;
const CMD_CRCCHK: u32 = 1 << 19;
const CMD_IXCHK: u32 = 1 << 20;
const CMD_ISDATA: u32 = 1 << 21;
/// `CMDTM` Command Type = **abort** (bits 23:22 = 11b). An abort command is
/// the one command the host may issue while Command Inhibit (DAT) is still
/// set — the reference driver skips its DAT-line wait for exactly this
/// command type (`references/circle/addon/SDCard/emmc.cpp:133` and
/// `:988-991`), which is what makes CMD12 usable as the abort of a stuck
/// data transaction (SD-HC 3.00 §3.8).
const CMD_TYPE_ABORT: u32 = 3 << 22;
const fn cmd(index: u32) -> u32 {
    index << 24
}
// Transfer-mode flags (lower half): `emmc_xfer::TM_*`, SD-HC 3.00 §2.2.5.

// SD commands as full CMDTM words.
const GO_IDLE: u32 = cmd(0) | RSP_NONE;
const SEND_IF_COND: u32 = cmd(8) | RSP_48 | CMD_CRCCHK | CMD_IXCHK;
const ALL_SEND_CID: u32 = cmd(2) | RSP_136 | CMD_CRCCHK;
const SEND_REL_ADDR: u32 = cmd(3) | RSP_48 | CMD_CRCCHK | CMD_IXCHK;
const SEND_CSD: u32 = cmd(9) | RSP_136 | CMD_CRCCHK;
const SELECT_CARD: u32 = cmd(7) | RSP_48BUSY | CMD_CRCCHK | CMD_IXCHK;
const SET_BLOCKLEN: u32 = cmd(16) | RSP_48 | CMD_CRCCHK | CMD_IXCHK;
const READ_SINGLE: u32 =
    cmd(17) | RSP_48 | CMD_CRCCHK | CMD_IXCHK | CMD_ISDATA | TM_DAT_CARD2HOST;
const WRITE_SINGLE: u32 = cmd(24) | RSP_48 | CMD_CRCCHK | CMD_IXCHK | CMD_ISDATA;
/// `CMD18 READ_MULTIPLE_BLOCK` (SD Phys 9.10 §4.3.3, command table §4.7.4:
/// adtc, R1) with Block Count + Auto CMD12 (SD-HC 3.00 §2.2.5, §1.11).
const READ_MULTIPLE: u32 =
    cmd(18) | RSP_48 | CMD_CRCCHK | CMD_IXCHK | CMD_ISDATA | TM_DAT_CARD2HOST | TM_MULTI;
/// `CMD25 WRITE_MULTIPLE_BLOCK` (SD Phys 9.10 §4.3.4, §4.7.4: adtc, R1)
/// with Block Count + Auto CMD12.
const WRITE_MULTIPLE: u32 = cmd(25) | RSP_48 | CMD_CRCCHK | CMD_IXCHK | CMD_ISDATA | TM_MULTI;
/// `ACMD6 SET_BUS_WIDTH` (SD Phys 9.10 §4.7.4 application command table:
/// ac, R1).
const SET_BUS_WIDTH: u32 = cmd(6) | RSP_48 | CMD_CRCCHK | CMD_IXCHK;
/// `ACMD51 SEND_SCR` (SD Phys 9.10 §4.7.4: adtc, R1) — one 8-byte block
/// card to host, the SCR (§5.6).
const SEND_SCR: u32 = cmd(51) | RSP_48 | CMD_CRCCHK | CMD_IXCHK | CMD_ISDATA | TM_DAT_CARD2HOST;
const APP_CMD: u32 = cmd(55) | RSP_48 | CMD_CRCCHK | CMD_IXCHK;
const SD_SEND_OP_COND: u32 = cmd(41) | RSP_48; // ACMD41, R3 (no CRC/index check)
/// CMD12 `STOP_TRANSMISSION`, issued as the **abort command** of SD-HC §3.8.
/// R1b with CRC check, no index check — the flag set the pinned reference
/// uses for this command (`emmc.cpp:284`, `SD_CMD_INDEX(12) | SD_RESP_R1b |
/// SD_CMD_TYPE_ABORT`, with `SD_RESP_R1b = RSPNS_TYPE_48B | CRCCHK_EN`
/// at `:193`).
const STOP_TRANSMISSION: u32 = cmd(12) | RSP_48BUSY | CMD_CRCCHK | CMD_TYPE_ABORT;

/// Busy-wait iteration budget for the **initialisation-phase** register
/// polls only: the controller reset, the clock-stable wait and the ACMD41
/// power-up loop. Those are audit findings 9 and 11 and belong to their own
/// tickets; the per-sector wait phases no longer use an iteration count at
/// all — they use [`Budget`], a real duration.
const SPIN_LIMIT: u32 = 1_000_000;

// ─────────────────────────────────────────────────────────────────────
// Wait budgets — a timeout is a duration, not an iteration count
// ─────────────────────────────────────────────────────────────────────
//
// FS04 audit finding 1 (`docs/api/hw/emmc-audit.md` §3.1, §4): the five
// waits of a sector operation used to share one iteration counter. On one
// and the same Raspberry Pi 4, inside a single measurement run, an
// iteration cost 104 ns in one block and 201 ns in another (the CPU clock
// moved) — so the "timeout" was 104–201 ms depending on how fast the
// machine happened to be running, while the specification allows the card
// 250 ms of programming time per write and 500 ms for the last busy.
// Twelve of 23 sector writes ran into that limit and were silently
// retried (audit §3.2).
//
// Every limit below is therefore a duration measured on the ARM generic
// timer, and each wait phase gets the limit the specification gives *that*
// phase. The SD specifications are not in `references/` (audit finding
// S1), so each quote is cited through the audit, which carries it verbatim.

/// Longest busy a write may legally take, in milliseconds.
///
/// SD Physical Layer Simplified Spec 6.00 §4.6.2.2: *"In case of High
/// Capacity SD Memory Card, maximum length of busy is defined as **250 ms**
/// for all write operation"*, and *"a) The last busy in any write operation
/// up to **500 ms** including single and multiple block write"*; its
/// Application Note: *"It is **strongly recommended for hosts to implement
/// more than 500 ms timeout value** even if the card indicates the 250 ms
/// maximum busy length"* (quoted in `docs/api/hw/emmc-audit.md` §3.1).
///
/// 1000 ms is that "more than 500 ms", with a factor-2 margin over the
/// longest busy the specification permits. It is also shorter than the
/// controller's own data timeout (`DATA_TOUNIT` = 0xE, measured at 2.58 s
/// in QEMU, audit §5 M2), so a card that exceeds its own specification is
/// caught here and recovered from rather than stalling the machine.
const WRITE_BUSY_MS: u32 = 1_000;

/// Longest read the host must tolerate, in milliseconds.
///
/// SD Phys 6.00 §4.6.2.1: *"The host should use **100 ms timeout
/// (minimum)** for single and multiple read operations"* (audit §3.1). That
/// is a *minimum*, and the audit counts it against the old budget that it
/// "sits exactly on the lower bound" (§3.1) — so this takes twice it, the
/// same factor-2 margin as [`WRITE_BUSY_MS`]. Measured read waits on the Pi
/// were 0.5–1.1 ms (audit §3.3), three orders of magnitude below.
const READ_MS: u32 = 200;

/// Backstop for the phases the SD Physical spec puts no *card*-side limit
/// on: the two Command Inhibit waits, Command Complete and Buffer Write
/// Ready. The card's response time is bounded by N_CR ≤ 64 clock cycles and
/// the controller enforces it in hardware (`Command Timeout Error`, SD-HC
/// §2.2.18), so a software limit here only bounds a *wedged controller*. It
/// is set to [`WRITE_BUSY_MS`], the longest card-side wait the specification
/// allows anywhere, so that no single wait can stall the system longer than
/// that.
const CTRL_MS: u32 = WRITE_BUSY_MS;

/// Budget for the recovery steps themselves (software reset, abort
/// command). Neither has a specified duration — this only keeps error
/// recovery from becoming the next hang; the pinned reference waits 1 s for
/// the same reset bits (`emmc.cpp:945`, `:960`). When it runs out, the
/// controller is reported as SD-HC §3.10.1 "Non-recoverable Error".
const RECOVERY_MS: u32 = 100;

/// Read `CNTPCT_EL0` — the free-running physical counter of the ARM generic
/// timer, the same clock source `drivers::timer` uses for the 200 Hz tick.
#[inline]
fn cntpct() -> u64 {
    let v: u64;
    // SAFETY: read-only system-register read, permitted at EL1.
    unsafe {
        core::arch::asm!("mrs {0}, cntpct_el0", out(reg) v,
                         options(nomem, nostack, preserves_flags));
    }
    v
}

/// Read `CNTFRQ_EL0` — the counter frequency in Hz, as the machine reports
/// it (0 when it reports none; see [`Budget::Spins`]).
#[inline]
fn cntfrq() -> u64 {
    let v: u64;
    // SAFETY: read-only system-register read, permitted at EL1.
    unsafe {
        core::arch::asm!("mrs {0}, cntfrq_el0", out(reg) v,
                         options(nomem, nostack, preserves_flags));
    }
    v
}

/// Counter ticks for `ms` milliseconds at `frq` Hz — never zero, so a wait
/// always gets at least one tick of budget.
const fn ticks_for_ms(frq: u64, ms: u32) -> u64 {
    let t = frq.saturating_mul(ms as u64) / 1000;
    if t == 0 { 1 } else { t }
}

/// Has a wait that started at counter value `start` reached `limit` ticks?
///
/// Only the *difference* of the two counter readings is used, so the test
/// survives the counter wrapping: `now.wrapping_sub(start)` is the true
/// elapsed count as long as the wait is shorter than a full counter period.
/// If the implemented counter is narrower than the 64-bit register and rolls
/// over to zero mid-wait, the difference jumps to a huge value and the wait
/// ends early — a timeout that error recovery handles, not a hang, and at
/// the counter frequencies these boards report (54 MHz on the Pi 4) that is
/// decades away.
const fn reached(start: u64, now: u64, limit: u64) -> bool {
    now.wrapping_sub(start) >= limit
}

/// The wait budget of one polling phase.
enum Budget {
    /// Normal case: the counter value the wait started at, and the number of
    /// counter ticks it may take.
    Ticks { start: u64, limit: u64 },
    /// `CNTFRQ_EL0` reads 0 — the machine reports no counter frequency (the
    /// case `drivers::timer::init` refuses to start the system tick in), so
    /// no duration can be formed here either. Falls back to a bounded
    /// iteration count purely so a wedged controller cannot hang the machine
    /// for ever. This is the old, CPU-clock-dependent behaviour and is
    /// named as such: on a machine without a counter, the limit is not a
    /// time.
    Spins(u32),
}

impl Budget {
    /// Open a budget of `ms` milliseconds for the current wait.
    fn new(ms: u32) -> Self {
        let frq = cntfrq();
        if frq == 0 {
            return Budget::Spins(SPIN_LIMIT);
        }
        Budget::Ticks { start: cntpct(), limit: ticks_for_ms(frq, ms) }
    }

    /// Has the budget run out? Call it *after* testing the register, so a
    /// condition that is already satisfied never reports a timeout.
    fn expired(&mut self) -> bool {
        match self {
            Budget::Ticks { start, limit } => reached(*start, cntpct(), *limit),
            Budget::Spins(left) => {
                if *left == 0 {
                    true
                } else {
                    *left -= 1;
                    false
                }
            }
        }
    }
}

/// The wait phases of one sector operation. Each carries the timeout the
/// specification gives *that* phase — they are not the same number — and
/// which line a failure in it belongs to.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum Phase {
    /// Command Inhibit (CMD) must be 0 before a command may be written
    /// (SD-HC §3.7.1.1 (1)).
    #[default]
    CmdInhibit,
    /// Command Inhibit (DAT) must be 0 before a command that uses the DAT
    /// line (SD-HC §3.7.1.1 (4)). What is actually being waited out here is
    /// the **previous** write's programming busy (audit §2.4).
    DatInhibit,
    /// Command Complete (SD-HC §3.7.1.2 (1)).
    CmdDone,
    /// Buffer Read Ready — the card's data has arrived in the FIFO.
    ReadRdy,
    /// Buffer Write Ready — the FIFO has room for the block. Inside a
    /// `CMD25` this also waits out the card's busy **between** blocks.
    WriteRdy,
    /// Transfer Complete of a read.
    ReadDone,
    /// Transfer Complete of a write: *"this bit is set … when the last data
    /// is written to the SD card as specified by data length **and the busy
    /// signal released**"* (SD-HC §2.2.17 bit 01, write case) — i.e. the end
    /// of the card's programming time.
    WriteDone,
}

impl Phase {
    /// Number of phases — the width of the per-phase counters.
    pub const COUNT: usize = 7;

    /// Index into the per-phase counters / [`PHASE_NAMES`].
    pub(super) const fn index(self) -> usize {
        self as usize
    }

    /// The timeout this phase is allowed, in milliseconds.
    const fn limit_ms(self) -> u32 {
        match self {
            // Card busy: the write-busy limit of SD Phys §4.6.2.2. Buffer
            // Write Ready belongs here since FS08: inside a multiple-block
            // write it is also the busy at the block gap (§4.6.2.2, 250 ms;
            // `docs/hardware/emmc-sd.md` §7.1 row 4).
            Phase::DatInhibit | Phase::WriteRdy | Phase::WriteDone => WRITE_BUSY_MS,
            // Card read latency: the 100 ms minimum of SD Phys §4.6.2.1.
            Phase::ReadRdy | Phase::ReadDone => READ_MS,
            // Controller handshakes: the wedged-controller backstop.
            Phase::CmdInhibit | Phase::CmdDone => CTRL_MS,
        }
    }

    /// Does a failure in this phase leave a transaction on the DAT line?
    /// Those are aborted with CMD12 + both software resets (SD-HC §3.8); a
    /// bare command failure only resets the line that failed (§3.10.1).
    const fn on_dat_line(self) -> bool {
        matches!(
            self,
            Phase::DatInhibit
                | Phase::ReadRdy
                | Phase::WriteRdy
                | Phase::ReadDone
                | Phase::WriteDone
        )
    }

    /// The software resets SD-HC §3.10.1 / §3.8 ask for, given this phase
    /// and the error bits that were set (0 for a plain timeout).
    const fn resets(self, status: u32) -> u32 {
        // §3.10.1: reset the line the reported error belongs to.
        let mut bits = 0;
        if status & INT_ERR_CMD_MASK != 0 {
            bits |= C1_SRST_CMD;
        }
        if status & INT_ERR_DAT_MASK != 0 {
            bits |= C1_SRST_DATA;
        }
        if self.on_dat_line() {
            // §3.8: an aborted data transaction takes both resets.
            bits |= C1_SRST_CMD | C1_SRST_DATA;
        } else {
            // A command phase that failed without any error bit (a plain
            // timeout) still leaves the command circuit mid-command.
            bits |= C1_SRST_CMD;
        }
        bits
    }
}

/// Phase names for the diagnostics, in [`Phase`] order.
pub const PHASE_NAMES: [&str; Phase::COUNT] =
    ["cmd_inhibit", "dat_inhibit", "cmd_done", "read_rdy", "write_rdy", "read_done", "write_done"];

/// The error bits the driver counts by name, in [`ERROR_BIT_NAMES`] order.
const ERROR_BIT_MASKS: [u32; 8] = [
    INT_ERR_CTO,
    INT_ERR_CCRC,
    INT_ERR_CEND,
    INT_ERR_CBAD,
    INT_ERR_DTO,
    INT_ERR_DCRC,
    INT_ERR_DEND,
    INT_ERR_ACMD,
];

/// Names of the counted error bits, in [`ERROR_BIT_MASKS`] order.
pub const ERROR_BIT_NAMES: [&str; 8] =
    ["CTO", "CCRC", "CEND", "CBAD", "DTO", "DCRC", "DEND", "ACMD"];

// ── Error bookkeeping (audit finding 12: report the bits, do not just
// clear them) ────────────────────────────────────────────────────────
static ERR_LAST_STATUS: AtomicU32 = AtomicU32::new(0);
static ERR_LAST_PHASE: AtomicU32 = AtomicU32::new(0);
static ERR_BIT_COUNT: [AtomicU32; ERROR_BIT_MASKS.len()] =
    [const { AtomicU32::new(0) }; ERROR_BIT_MASKS.len()];
static ERR_OTHER: AtomicU32 = AtomicU32::new(0);
static ERR_IO_TOTAL: AtomicU32 = AtomicU32::new(0);
static ERR_TIMEOUT_TOTAL: AtomicU32 = AtomicU32::new(0);
static ERR_TIMEOUT_PHASE: [AtomicU32; Phase::COUNT] =
    [const { AtomicU32::new(0) }; Phase::COUNT];
static ERR_RECOVERIES: AtomicU32 = AtomicU32::new(0);
static ERR_NONRECOVERABLE: AtomicU32 = AtomicU32::new(0);
static ERR_R1_TOTAL: AtomicU32 = AtomicU32::new(0);
static ERR_R1_LAST: AtomicU32 = AtomicU32::new(0);
static ERR_R1_BIT_COUNT: [AtomicU32; R1_BIT_MASKS.len()] =
    [const { AtomicU32::new(0) }; R1_BIT_MASKS.len()];
/// Auto CMD Error Status ([`CONTROL2`] low half, SD-HC 3.00 §2.2.23) as it
/// was at the most recent `Auto CMD Error`.
static ERR_LAST_ACMD: AtomicU32 = AtomicU32::new(0);

/// `emmc=compat` (FS08): 1-bit bus, one `CMD17`/`CMD24` per sector — the
/// pre-FS08 driver. Set by [`set_compat`] from the boot arguments before
/// [`init`]; default off.
static COMPAT: AtomicBool = AtomicBool::new(false);
/// Did [`init`] switch the card and the host to the 4-bit bus?
static BUS_4BIT: AtomicBool = AtomicBool::new(false);
/// The SCR's `SD_BUS_WIDTHS` field as read at init (0 = not read).
static SCR_WIDTHS: AtomicU32 = AtomicU32::new(0);
/// A `CMD18` just read the card's last block: the spurious `OUT_OF_RANGE`
/// SD Phys 9.10 §4.3.3 tells the host to ignore may still arrive in the
/// response of the next memory command (SD-HC 3.00 §3.7.1.2 Note 3).
static OOR_PENDING: AtomicBool = AtomicBool::new(false);

/// Select the transfer mode for the coming [`init`]: `true` = the
/// `emmc=compat` fallback (1-bit bus, single-block commands only).
/// Has an effect only before the card is initialised.
pub fn set_compat(compat: bool) {
    COMPAT.store(compat, Ordering::Relaxed);
}

/// Is the driver in the `emmc=compat` fallback mode?
fn compat() -> bool {
    COMPAT.load(Ordering::Relaxed)
}

/// The transfer setup the initialised card actually runs with —
/// `(compat, bus width in bits)` — or `None` while no card is up
/// (`Ssystem(S_NEWTOS_SDMODE / S_NEWTOS_SDBUS)`, SYSINFO01). The width is 4
/// only once [`init`] switched both the card (`ACMD6`) and the host to the
/// 4-bit bus; fast mode with a card that offers no 4-bit bus stays at 1.
pub fn transfer_mode() -> Option<(bool, u8)> {
    if !SDCARD.ready.load(Ordering::Acquire) {
        return None;
    }
    Some((compat(), if BUS_4BIT.load(Ordering::Relaxed) { 4 } else { 1 }))
}

/// Print failures on the console? Off while [`init`] probes the controller
/// bases — a slot without a card answers every response command with a
/// command timeout, and that is the probe doing its job.
static REPORT_ERRORS: AtomicBool = AtomicBool::new(false);

/// What the driver has seen go wrong — the counters audit finding 12 asks
/// for, so a caller can tell a timeout from a CRC error from a controller
/// data timeout instead of getting a bare `BlockError::Io`.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct ErrorStats {
    /// `INTERRUPT` as it was read at the most recent failed wait (0 when
    /// the failure was a plain timeout with no error bit set).
    pub last_status: u32,
    /// The phase that failed most recently.
    pub last_phase: Phase,
    /// How often each named error bit was seen, in [`ERROR_BIT_NAMES`] order.
    pub bits: [u32; ERROR_BIT_MASKS.len()],
    /// Errors with none of the named bits set (current-limit, ADMA,
    /// tuning — SD-HC §2.2.18 bits 23, 25, 26).
    pub other: u32,
    /// Waits that ended on an error bit (`BlockError::Io`).
    pub io_errors: u32,
    /// Waits that ran out of time (`BlockError::Timeout`).
    pub timeouts: u32,
    /// Timeouts per phase, in [`PHASE_NAMES`] order.
    pub timeouts_by_phase: [u32; Phase::COUNT],
    /// Recovery sequences run (SD-HC §3.10.1 / §3.8).
    pub recoveries: u32,
    /// Recoveries after which Command Inhibit was still set — SD-HC
    /// §3.10.1's "Non-recoverable Error".
    pub nonrecoverable: u32,
    /// Commands the **card** rejected: R1 responses carrying at least one
    /// Type-`E` bit of SD Phys Table 4-42 (audit finding 3).
    pub card_errors: u32,
    /// The card status word of the most recent rejected command.
    pub last_card_status: u32,
    /// How often each named R1 error bit was seen, in [`R1_BIT_NAMES`] order.
    pub card_bits: [u32; R1_BIT_MASKS.len()],
}

/// A snapshot of the driver's error bookkeeping. Cheap: plain relaxed loads.
pub fn error_stats() -> ErrorStats {
    let mut s = ErrorStats {
        last_status: ERR_LAST_STATUS.load(Ordering::Relaxed),
        last_phase: PHASES[(ERR_LAST_PHASE.load(Ordering::Relaxed) as usize) % Phase::COUNT],
        other: ERR_OTHER.load(Ordering::Relaxed),
        io_errors: ERR_IO_TOTAL.load(Ordering::Relaxed),
        timeouts: ERR_TIMEOUT_TOTAL.load(Ordering::Relaxed),
        recoveries: ERR_RECOVERIES.load(Ordering::Relaxed),
        nonrecoverable: ERR_NONRECOVERABLE.load(Ordering::Relaxed),
        card_errors: ERR_R1_TOTAL.load(Ordering::Relaxed),
        last_card_status: ERR_R1_LAST.load(Ordering::Relaxed),
        ..ErrorStats::default()
    };
    for (i, c) in ERR_BIT_COUNT.iter().enumerate() {
        s.bits[i] = c.load(Ordering::Relaxed);
    }
    for (i, c) in ERR_R1_BIT_COUNT.iter().enumerate() {
        s.card_bits[i] = c.load(Ordering::Relaxed);
    }
    for (i, c) in ERR_TIMEOUT_PHASE.iter().enumerate() {
        s.timeouts_by_phase[i] = c.load(Ordering::Relaxed);
    }
    s
}

/// [`Phase`] by index — the inverse of [`Phase::index`].
const PHASES: [Phase; Phase::COUNT] = [
    Phase::CmdInhibit,
    Phase::DatInhibit,
    Phase::CmdDone,
    Phase::ReadRdy,
    Phase::WriteRdy,
    Phase::ReadDone,
    Phase::WriteDone,
];

#[cfg(not(feature = "sd-fault"))]
#[inline(always)]
fn rd(off: usize) -> u32 {
    // SAFETY: MMIO read at a valid SD-host register address.
    unsafe { core::ptr::read_volatile((base() + off) as *const u32) }
}
/// With `sd-fault`: what the driver reads of `STATUS` carries the injected
/// stuck line ([`emmc_fault::Mode::Wedge`]).
#[cfg(feature = "sd-fault")]
#[inline(always)]
fn rd(off: usize) -> u32 {
    // SAFETY: MMIO read at a valid SD-host register address.
    let v = unsafe { core::ptr::read_volatile((base() + off) as *const u32) };
    if off == STATUS {
        v | super::emmc_fault::forced_status(SR_DAT_INHIBIT, cntpct())
    } else {
        v
    }
}
#[inline(always)]
fn wr(off: usize, val: u32) {
    // SAFETY: MMIO write at a valid SD-host register address.
    unsafe { core::ptr::write_volatile((base() + off) as *mut u32, val) }
}

/// Extract CSD field `[hi:lo]` from the 4-word response. SDHCI stores a
/// 136-bit response right-shifted by 8 bits (the CRC byte is dropped),
/// so CSD bit `n` is at response bit `n-8`.
fn csd_bits(r: &[u32; 4], hi: u32, lo: u32) -> u32 {
    let mut v = 0u32;
    for b in lo..=hi {
        let rb = b - 8;
        v |= ((r[(rb / 32) as usize] >> (rb % 32)) & 1) << (b - lo);
    }
    v
}

/// Compute the card's 512-byte block count from its CSD (v1 SDSC or
/// v2 SDHC/SDXC).
fn parse_capacity(r: &[u32; 4]) -> u64 {
    if csd_bits(r, 127, 126) == 1 {
        // v2: capacity = (C_SIZE + 1) × 512 KiB.
        (csd_bits(r, 69, 48) as u64 + 1) * 1024
    } else {
        // v1: (C_SIZE+1) × 2^(C_SIZE_MULT+2) blocks × 2^READ_BL_LEN bytes.
        let read_bl_len = csd_bits(r, 83, 80);
        let c_size = csd_bits(r, 73, 62) as u64;
        let c_size_mult = csd_bits(r, 49, 47);
        let blocknr = (c_size + 1) * (1u64 << (c_size_mult + 2));
        blocknr * (1u64 << read_bl_len) / SECTOR_SIZE as u64
    }
}

/// The microSD device on EMMC2.
pub struct Emmc {
    rca: AtomicU32,
    sdhc: AtomicBool,
    ready: AtomicBool,
    blocks: AtomicU64,
}

/// Board-selected block device instance.
pub static SDCARD: Emmc = Emmc {
    rca: AtomicU32::new(0),
    sdhc: AtomicBool::new(false),
    ready: AtomicBool::new(false),
    blocks: AtomicU64::new(0),
};

/// Record a failed wait and say so on the console — audit finding 12: the
/// bits used to be cleared before anyone could see which error it was, which
/// is why the twelve failed writes of the Pi measurement could not be told
/// apart from a CRC error or a controller data timeout.
///
/// The counters always run; the console line waits until [`REPORT_ERRORS`]
/// is set, because during the base probe a slot **without** a card answers
/// every response command with `CTO` — that is the probe working, not a
/// fault. Whatever the identification hit is printed once by
/// [`report_controller`] instead.
fn note_failure(phase: Phase, status: u32, timed_out: bool) {
    ERR_LAST_STATUS.store(status, Ordering::Relaxed);
    ERR_LAST_PHASE.store(phase.index() as u32, Ordering::Relaxed);
    let report = REPORT_ERRORS.load(Ordering::Relaxed);
    #[cfg(feature = "sd-diag")]
    if report {
        use tos_api::bios::sd_diag::FailKind;
        let kind = if timed_out { FailKind::Timeout } else { FailKind::Io };
        emmc_diag::failure(diag_now_us(), phase, kind, status);
    }
    if timed_out {
        ERR_TIMEOUT_TOTAL.fetch_add(1, Ordering::Relaxed);
        ERR_TIMEOUT_PHASE[phase.index()].fetch_add(1, Ordering::Relaxed);
        if report {
            crate::kprintln!(
                "[emmc] {} timed out after {} ms (INTERRUPT=0x{:08x}, STATUS=0x{:08x})",
                PHASE_NAMES[phase.index()],
                phase.limit_ms(),
                status,
                rd(STATUS),
            );
        }
        return;
    }

    ERR_IO_TOTAL.fetch_add(1, Ordering::Relaxed);
    // Auto CMD Error: its detail register is valid only while the error is
    // set (SD-HC 3.00 §2.2.23), so it is saved here, before any reset.
    let acmd = if status & INT_ERR_ACMD != 0 { rd(CONTROL2) & 0xFFFF } else { 0 };
    if status & INT_ERR_ACMD != 0 {
        ERR_LAST_ACMD.store(acmd, Ordering::Relaxed);
    }
    let mut named = 0u32;
    for (i, mask) in ERROR_BIT_MASKS.iter().enumerate() {
        if status & mask != 0 {
            ERR_BIT_COUNT[i].fetch_add(1, Ordering::Relaxed);
            named |= mask;
        }
    }
    if status & INT_ERROR_MASK & !named != 0 {
        ERR_OTHER.fetch_add(1, Ordering::Relaxed);
    }
    if !report {
        return;
    }
    crate::kprint!("[emmc] {} error: INTERRUPT=0x{:08x}", PHASE_NAMES[phase.index()], status);
    for (i, mask) in ERROR_BIT_MASKS.iter().enumerate() {
        if status & mask != 0 {
            crate::kprint!(" {}", ERROR_BIT_NAMES[i]);
        }
    }
    if status & INT_ERR_ACMD != 0 {
        crate::kprint!(" (Auto CMD Error Status=0x{:04x})", acmd);
    }
    crate::kprintln!("");
}

/// Evaluate an **R1 card status** word and report what the card said —
/// audit finding 3 (`docs/api/hw/emmc-audit.md` §4), the bit table is
/// `docs/hardware/emmc-sd.md` §5 (SD Phys 9.10 Table 4-42).
///
/// `cmd_index` is the SD command number, only used for the console line.
/// Returns `Err(BlockError::Card)` when any Type-`E` bit is set: the card
/// has refused or failed the command, whatever the controller's
/// `INTERRUPT` register says.
///
/// `mask` is the set of bits that count as a refusal — normally
/// [`R1_ERROR_MASK`]; without `OUT_OF_RANGE` where SD Phys 9.10 §4.3.3 says
/// to ignore it ([`xfer::r1_mask`]).
fn check_r1(cmd_index: u32, status: u32, mask: u32) -> Result<(), BlockError> {
    if status & mask == 0 {
        return Ok(());
    }
    #[cfg(feature = "sd-diag")]
    if REPORT_ERRORS.load(Ordering::Relaxed) {
        // The refusal is not a wait: the log names the command's data phase.
        let phase = if matches!(cmd_index, 24 | 25) { Phase::WriteRdy } else { Phase::ReadRdy };
        emmc_diag::failure(diag_now_us(), phase, tos_api::bios::sd_diag::FailKind::Card, status);
    }
    ERR_R1_TOTAL.fetch_add(1, Ordering::Relaxed);
    ERR_R1_LAST.store(status, Ordering::Relaxed);
    for (i, bit) in R1_BIT_MASKS.iter().enumerate() {
        if status & mask & bit != 0 {
            ERR_R1_BIT_COUNT[i].fetch_add(1, Ordering::Relaxed);
        }
    }
    if REPORT_ERRORS.load(Ordering::Relaxed) {
        crate::kprint!("[emmc] CMD{} rejected by card: status=0x{:08x}", cmd_index, status);
        for (i, bit) in R1_BIT_MASKS.iter().enumerate() {
            if status & mask & bit != 0 {
                crate::kprint!(" {}", R1_BIT_NAMES[i]);
            }
        }
        // CURRENT_STATE (bits 12:9) says which state the card was in when
        // it received the command — Table 4-42, entry 12:9.
        crate::kprintln!(" (state {})", (status >> 9) & 0xF);
    }
    Err(BlockError::Card)
}

impl Emmc {
    /// Poll until `(rd(off) & mask) == 0` within `phase`'s time budget.
    /// On expiry the failure is recorded and the controller is recovered
    /// (SD-HC §3.10.1) before `false` is returned.
    fn wait_clear(off: usize, mask: u32, phase: Phase) -> bool {
        let mut budget = Budget::new(phase.limit_ms());
        #[cfg(feature = "sd-diag")]
        let t0 = cntpct();
        loop {
            if rd(off) & mask == 0 {
                #[cfg(feature = "sd-diag")]
                emmc_diag::wait_done(phase, cntpct().wrapping_sub(t0));
                return true;
            }
            if budget.expired() {
                #[cfg(feature = "sd-diag")]
                emmc_diag::wait_done(phase, cntpct().wrapping_sub(t0));
                note_failure(phase, rd(INTERRUPT), true);
                Self::recover(phase, 0);
                return false;
            }
            core::hint::spin_loop();
        }
    }

    /// Wait for `flag` in INTERRUPT (clearing it), or an error / timeout.
    ///
    /// Both failure exits go through [`Emmc::recover`] — SD-HC 3.00 §3.10.1
    /// (Error Interrupt Recovery) and §3.8 (Abort Transaction). Before that
    /// the timeout exit left the controller mid-transfer with the DAT line
    /// still its own, so the *next* command paid the remainder of the same
    /// busy (audit finding 2, §3.2).
    fn wait_interrupt(flag: u32, phase: Phase) -> Result<(), BlockError> {
        let mut budget = Budget::new(phase.limit_ms());
        #[cfg(feature = "sd-diag")]
        let t0 = cntpct();
        loop {
            let i = rd(INTERRUPT);
            if i & INT_ERROR_MASK != 0 {
                wr(INTERRUPT, i); // clear (W1C) — the value is kept below
                #[cfg(feature = "sd-diag")]
                emmc_diag::wait_done(phase, cntpct().wrapping_sub(t0));
                note_failure(phase, i, false);
                Self::recover(phase, i);
                return Err(BlockError::Io);
            }
            if i & flag != 0 {
                wr(INTERRUPT, flag); // clear the bit we consumed
                #[cfg(feature = "sd-diag")]
                emmc_diag::wait_done(phase, cntpct().wrapping_sub(t0));
                return Ok(());
            }
            if budget.expired() {
                #[cfg(feature = "sd-diag")]
                emmc_diag::wait_done(phase, cntpct().wrapping_sub(t0));
                note_failure(phase, i, true);
                Self::recover(phase, 0);
                return Err(BlockError::Timeout);
            }
            core::hint::spin_loop();
        }
    }

    /// Put the controller straight after a failed wait — SD Host Controller
    /// Simplified Spec 3.00 §3.10.1 (Error Interrupt Recovery) and §3.8
    /// (Abort Transaction), as required by audit finding 2:
    ///
    /// 1. the error status has been read and cleared by the failing wait
    ///    (and kept, see [`note_failure`]); it is passed in as `status`,
    /// 2. a transaction still on the DAT line is aborted with CMD12,
    /// 3. the line(s) that failed are software-reset,
    /// 4. Command Inhibit must be clear again afterwards — if it is not,
    ///    the controller is blocked and that is reported as §3.10.1's
    ///    "Non-recoverable Error".
    fn recover(phase: Phase, status: u32) {
        Self::recover_inner(phase, status, phase.on_dat_line())
    }

    /// [`Emmc::recover`] with the abort command suppressed — for the case
    /// where the **card** rejected the command (its R1 carried an error
    /// bit) rather than the transfer breaking. The card never left transfer
    /// state, so there is no data transaction on its side to abort; all
    /// that is stuck is the controller's data state machine, and
    /// `Software Reset For DAT Line` is defined to clear exactly that:
    /// SD-HC 3.00 §2.2.15 lists "Buffer Data Port", "Buffer Read Enable",
    /// "Buffer Write Enable", "Command Inhibit (DAT)" and "DAT Line Active"
    /// among the bits it clears (`docs/hardware/emmc-sd.md` §1 CONTROL1).
    fn recover_no_abort(phase: Phase) {
        Self::recover_inner(phase, 0, false)
    }

    fn recover_inner(phase: Phase, status: u32, abort: bool) {
        ERR_RECOVERIES.fetch_add(1, Ordering::Relaxed);
        // §3.10.1 (4): save and clear the interrupt status. Saved already —
        // `note_failure` has it (finding 12) — so the W1C here loses
        // nothing and leaves no stale bit for the next command to trip on.
        wr(INTERRUPT, rd(INTERRUPT));
        if abort {
            // SD-HC 3.00 §3.10.2 (Auto CMD12 Error Recovery): when the Auto
            // CMD12 that ends a multiple-block transfer *was* issued and
            // failed (`Auto CMD12 Not Executed` = 0), the CMD line is reset
            // first, steps (6)-(7), and only then is CMD12 issued again,
            // step (8); the DAT line reset of steps (11)-(12) follows below.
            // No command without data is ever in flight during a transfer
            // here, so the CMD_wo_DAT branches of that figure do not arise.
            if status & INT_ERR_ACMD != 0
                && ERR_LAST_ACMD.load(Ordering::Relaxed) & ACMD_NOT_EXECUTED == 0
            {
                Self::soft_reset(C1_SRST_CMD);
            }
            Self::abort_transaction();
        }
        Self::soft_reset(phase.resets(status));

        let mut budget = Budget::new(RECOVERY_MS);
        loop {
            let s = rd(STATUS);
            if s & (SR_CMD_INHIBIT | SR_DAT_INHIBIT) == 0 {
                return;
            }
            if budget.expired() {
                #[cfg(feature = "sd-diag")]
                emmc_diag::failure(diag_now_us(), phase, tos_api::bios::sd_diag::FailKind::Nonrecoverable, s);
                ERR_NONRECOVERABLE.fetch_add(1, Ordering::Relaxed);
                crate::kprintln!(
                    "[emmc] recovery incomplete, STATUS=0x{:08x} — non-recoverable (SD-HC 3.00 §3.10.1)",
                    s,
                );
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// Write the given `Software Reset For …` bits and wait for the
    /// controller to clear them again (SD-HC §2.2.15). A reset that does not
    /// complete is not reported here — the Command Inhibit check in
    /// [`Emmc::recover`] is what decides recoverable vs. not.
    ///
    /// **One bit per write.** SD-HC §3.10.1 lists the two resets as two
    /// steps ("Software Reset For CMD Line", then "Software Reset For DAT
    /// Line"), and so does the abort sequence of §3.8; issuing them in one
    /// write is an optimisation the specification never asks for. It is
    /// also one a host controller may not implement: QEMU's SDHCI model
    /// decodes the reset byte with an **exact-match** `switch`
    /// (`hw/sd/sdhci.c:1150-1170`, v11.0.2 — `case SDHC_RESET_CMD:` /
    /// `case SDHC_RESET_DATA:`), so a write with both bits set matches no
    /// case and resets *nothing*, silently: the bits live in the upper byte
    /// of `CLKCON`, which the model does not store, so they read back as 0
    /// and the wait below reports a completed reset that never happened.
    /// Driven: with both bits in one write the controller stayed at
    /// `STATUS=0x01ff0506` (Command Inhibit (DAT) + DAT Line Active) after
    /// every recovery and no further sector access succeeded.
    fn soft_reset(bits: u32) {
        for one in [C1_SRST_CMD, C1_SRST_DATA] {
            if bits & one == 0 {
                continue;
            }
            wr(CONTROL1, (rd(CONTROL1) & !(C1_SRST_CMD | C1_SRST_DATA)) | one);
            let mut budget = Budget::new(RECOVERY_MS);
            while rd(CONTROL1) & one != 0 {
                if budget.expired() {
                    break;
                }
                core::hint::spin_loop();
            }
        }
    }

    /// Issue CMD12 as the abort command of SD-HC §3.8.
    ///
    /// Deliberately not routed through [`Emmc::command`]: that would open a
    /// second full wait budget and re-enter recovery on failure. The outcome
    /// is not checked — the software resets in [`Emmc::recover`] put the
    /// controller straight either way; the abort is what tells the *card* to
    /// leave the data transfer.
    fn abort_transaction() {
        // An abort command may be issued while Command Inhibit (DAT) is
        // still set, but not while the CMD line itself is busy.
        let mut budget = Budget::new(RECOVERY_MS);
        while rd(STATUS) & SR_CMD_INHIBIT != 0 {
            if budget.expired() {
                return; // CMD line blocked — the resets are all that is left
            }
            core::hint::spin_loop();
        }
        wr(INTERRUPT, rd(INTERRUPT));
        wr(ARG1, 0);
        wr(CMDTM, STOP_TRANSMISSION);
        let mut budget = Budget::new(RECOVERY_MS);
        loop {
            let i = rd(INTERRUPT);
            if i & (INT_CMD_DONE | INT_ERROR_MASK) != 0 {
                wr(INTERRUPT, i);
                return;
            }
            if budget.expired() {
                return;
            }
            core::hint::spin_loop();
        }
    }

    /// Issue a command and return `RESP0`.
    fn command(&self, cmdtm: u32, arg: u32) -> Result<u32, BlockError> {
        if !Self::wait_clear(STATUS, SR_CMD_INHIBIT, Phase::CmdInhibit) {
            return Err(BlockError::Timeout);
        }
        wr(INTERRUPT, rd(INTERRUPT)); // clear any leftover status
        wr(ARG1, arg);
        wr(CMDTM, cmdtm);
        Self::wait_interrupt(INT_CMD_DONE, Phase::CmdDone)?;
        Ok(rd(RESP0))
    }

    /// An application command (`CMD55` then the ACMD).
    fn app_command(&self, cmdtm: u32, arg: u32) -> Result<u32, BlockError> {
        let rca = self.rca.load(Ordering::Relaxed) << 16;
        self.command(APP_CMD, rca)?;
        self.command(cmdtm, arg)
    }

    /// Reset the controller and bring the clock up to `target_hz`.
    fn reset_and_clock(target_hz: u32) -> Result<(), BlockError> {
        wr(CONTROL0, 0);
        wr(CONTROL1, C1_SRST_HC);
        for _ in 0..SPIN_LIMIT {
            if rd(CONTROL1) & C1_SRST_HC == 0 {
                break;
            }
            core::hint::spin_loop();
        }
        // Power the SD bus: CONTROL0 byte 1 = Power Control —
        // SD_BUS_POWER (bit 8) + voltage select 3.3 V (0b111 at bits 11:9).
        wr(CONTROL0, 0x0F00);
        Self::set_clock(target_hz)?;
        wr(IRPT_EN, 0); // poll, don't signal
        wr(IRPT_MASK, 0xFFFF_FFFF); // but record all status bits
        Ok(())
    }

    /// Program the SD clock divider for ~`target_hz`.
    fn set_clock(target_hz: u32) -> Result<(), BlockError> {
        // Base clock from CAPABILITIES[15:8] (MHz); fall back to 100 MHz.
        let base_mhz = (rd(CAPABILITIES) >> 8) & 0xFF;
        let base_hz = if base_mhz == 0 { 100_000_000 } else { base_mhz * 1_000_000 };
        // SDCLK = base / (2 * div); pick the smallest div that fits.
        let mut div = 1u32;
        while base_hz / (2 * div) > target_hz && div < 0x3FF {
            div += 1;
        }
        let freq8 = (div & 0xFF) << 8;
        let freq_ms2 = ((div >> 8) & 0x3) << 6;
        wr(CONTROL1, C1_CLK_INTLEN | freq8 | freq_ms2 | C1_TOUNIT_MAX);
        for _ in 0..SPIN_LIMIT {
            if rd(CONTROL1) & C1_CLK_STABLE != 0 {
                break;
            }
            core::hint::spin_loop();
        }
        wr(CONTROL1, rd(CONTROL1) | C1_CLK_EN);
        Ok(())
    }

    /// Full card init: reset, clock, then the SD command sequence.
    fn init_card(&self) -> Result<(), BlockError> {
        Self::reset_and_clock(400_000)?; // identification speed

        self.command(GO_IDLE, 0)?;

        // CMD8: voltage check. Success ⇒ SD v2 (high-capacity capable).
        let v2 = self.command(SEND_IF_COND, 0x1AA).is_ok();

        // ACMD41: wait for power-up; HCS set when v2.
        let hcs = if v2 { 0x4000_0000 } else { 0 };
        let mut ocr = 0;
        for _ in 0..SPIN_LIMIT {
            ocr = self.app_command(SD_SEND_OP_COND, hcs | 0x00FF_8000)?;
            if ocr & 0x8000_0000 != 0 {
                break; // power-up complete
            }
            core::hint::spin_loop();
        }
        if ocr & 0x8000_0000 == 0 {
            return Err(BlockError::Timeout);
        }
        self.sdhc.store(ocr & 0x4000_0000 != 0, Ordering::Relaxed);

        self.command(ALL_SEND_CID, 0)?;
        let rca = self.command(SEND_REL_ADDR, 0)? >> 16;
        self.rca.store(rca, Ordering::Relaxed);

        // CMD9 (SEND_CSD) is valid in stand-by state — read it before
        // selecting the card to learn the capacity.
        self.command(SEND_CSD, rca << 16)?;
        let csd = [rd(RESP0), rd(RESP1), rd(RESP2), rd(RESP3)];
        self.blocks.store(parse_capacity(&csd), Ordering::Relaxed);

        self.command(SELECT_CARD, rca << 16)?;

        if !self.sdhc.load(Ordering::Relaxed) {
            self.command(SET_BLOCKLEN, SECTOR_SIZE as u32)?;
        }

        // Switch to default speed (~25 MHz) for transfers.
        Self::set_clock(25_000_000)?;

        // FS08: the 4-bit bus, unless `emmc=compat` asks for the old 1-bit
        // driver. A failure here is not a failed init — the card is in
        // transfer state on the 1-bit bus, which is what every SD card
        // starts with ("The extended DAT lines (DAT1-DAT3) are input on
        // power up. They start to operate as DAT lines after SET_BUS_WIDTH
        // command", SD Phys 9.10 §3.7.1, Table 3-1 note 2) — so it is
        // recorded and reported, and the driver carries on 1-bit.
        BUS_4BIT.store(false, Ordering::Relaxed);
        let wide = if compat() { WideBus::Compat } else { self.enable_wide_bus() };
        WIDE_BUS.store(wide as u32, Ordering::Relaxed);
        self.ready.store(true, Ordering::Release);
        Ok(())
    }

    /// Read the card's SCR with `ACMD51 SEND_SCR` — one 8-byte block, card
    /// to host (SD Phys 9.10 §5.6, §4.7.4), by the PIO read sequence of
    /// SD-HC 3.00 §3.7.2.1 with a Block Size of 8.
    fn read_scr(&self) -> Result<[u8; xfer::SCR_BYTES], BlockError> {
        if !Self::wait_clear(STATUS, SR_DAT_INHIBIT, Phase::DatInhibit) {
            return Err(BlockError::Timeout);
        }
        wr(BLKSIZECNT, xfer::blksizecnt(1, xfer::SCR_BYTES));
        let status = self.app_command(SEND_SCR, 0)?;
        if let Err(e) = check_r1(51, status, R1_ERROR_MASK) {
            Self::recover_no_abort(Phase::ReadRdy);
            return Err(e);
        }
        Self::wait_interrupt(INT_READ_RDY, Phase::ReadRdy)?;
        let mut scr = [0u8; xfer::SCR_BYTES];
        // Byte order of the 32-bit data port: little endian (SD-HC 3.00
        // §1.7 (2)), so the first byte on the bus lands in bits 7:0.
        for word in scr.chunks_exact_mut(4) {
            word.copy_from_slice(&rd(DATA).to_le_bytes());
        }
        Self::wait_interrupt(INT_DATA_DONE, Phase::ReadDone)?;
        Ok(scr)
    }

    /// Switch card and host to the 4-bit bus if the SCR allows it — the
    /// "Changing Bus Width" sequence of SD-HC 3.00 §3.4 for an SD memory
    /// card (steps 3 and 7 are SDIO-only):
    ///
    /// 1. `Card Interrupt Status Enable` off ([`INT_CARD`] in `IRPT_MASK`),
    /// 4. `ACMD6 SET_BUS_WIDTH` with `'10'` (SD Phys 9.10 §4.7.4),
    /// 5. Host Control 1 `Data Transfer Width` = 1 (SD-HC 3.00 §2.2.10),
    /// 8. `Card Interrupt Status Enable` back on.
    ///
    /// The card must be in transfer state (after `CMD7`) and not locked
    /// (§3.4: "if the card is locked, bus width cannot be changed") — a
    /// locked or unwilling card answers `ACMD6` with an R1 error, and the
    /// host bit is then left alone.
    fn enable_wide_bus(&self) -> WideBus {
        let scr = match self.read_scr() {
            Ok(scr) => scr,
            Err(_) => return WideBus::ScrFailed,
        };
        SCR_WIDTHS.store(xfer::scr_bus_widths(&scr) as u32, Ordering::Relaxed);
        if !xfer::scr_supports_4bit(&scr) {
            return WideBus::CardOneBit;
        }
        let mask = rd(IRPT_MASK);
        wr(IRPT_MASK, mask & !INT_CARD); // (1)
        let card = self
            .app_command(SET_BUS_WIDTH, xfer::ACMD6_ARG_4BIT) // (4)
            .and_then(|r1| check_r1(6, r1, R1_ERROR_MASK));
        let result = if card.is_ok() {
            wr(CONTROL0, rd(CONTROL0) | xfer::HC1_DATA_WIDTH_4BIT); // (5)
            BUS_4BIT.store(true, Ordering::Relaxed);
            WideBus::FourBit
        } else {
            WideBus::Acmd6Failed
        };
        wr(INTERRUPT, INT_CARD); // drop a card-interrupt glitch of the switch
        wr(IRPT_MASK, mask); // (8)
        result
    }

    /// The `ARG1` value that addresses block `lba` on this card.
    ///
    /// SDHC/SDXC take a **block number**, SDSC a **byte address**
    /// (`docs/hardware/emmc-sd.md` §3, `CCS`) — and both are a single
    /// 32-bit command argument. An `lba` whose address does not fit is
    /// refused here instead of being truncated: `(lba as u32) * 512`
    /// wrapped silently in a release build, which would have put the
    /// transfer on a **different, valid** sector of the card. Found while
    /// driving the finding-3 probe, which is the first caller that ever
    /// reached the arithmetic with a large `lba`.
    fn card_address(&self, lba: u64) -> Result<u32, BlockError> {
        if self.sdhc.load(Ordering::Relaxed) {
            u32::try_from(lba).map_err(|_| BlockError::OutOfRange)
        } else {
            u32::try_from(lba)
                .ok()
                .and_then(|b| b.checked_mul(SECTOR_SIZE as u32))
                .ok_or(BlockError::OutOfRange)
        }
    }

    /// The R1 mask for the next data command of `blocks` sectors at `lba`.
    ///
    /// Normally the full Type-`E` set. Right after a `CMD18` that read the
    /// card's last block, the spurious `OUT_OF_RANGE` of SD Phys 9.10
    /// §4.3.3 may arrive "in the response of the next memory command"
    /// (SD-HC 3.00 §3.7.1.2 Note 3) and is ignored there — once, and only
    /// if this command itself stays inside the card, so a request that
    /// really is out of range is still refused.
    fn next_r1_mask(&self, lba: u64, blocks: usize) -> u32 {
        let pending = OOR_PENDING.swap(false, Ordering::Relaxed);
        let capacity = self.blocks.load(Ordering::Relaxed);
        let inside = lba.checked_add(blocks as u64).is_some_and(|end| end <= capacity);
        xfer::r1_mask(R1_ERROR_MASK, R1_OUT_OF_RANGE, pending && inside)
    }

    /// Read `buf.len() / 512` consecutive blocks at `lba` with **one**
    /// command (PIO): `CMD17` for one block, `CMD18` + Block Count + Auto
    /// CMD12 for more — SD-HC 3.00 §3.7.2.1 (Not using DMA), steps (1)-(8)
    /// and (14)-(20). `buf` holds 1..=[`xfer::MAX_BLOCKS_PER_CMD`] sectors.
    fn read_run(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if !self.ready.load(Ordering::Acquire) {
            return Err(BlockError::NotReady);
        }
        let blocks = buf.len() / SECTOR_SIZE;
        let multi = xfer::transfer_for(blocks, compat()) == Transfer::Multiple;
        if !Self::wait_clear(STATUS, SR_DAT_INHIBIT, Phase::DatInhibit) {
            return Err(BlockError::Timeout);
        }
        let addr = self.card_address(lba)?;
        // (1)+(2) Block Size and Block Count.
        wr(BLKSIZECNT, xfer::blksizecnt(blocks, SECTOR_SIZE));
        let mask = self.next_r1_mask(lba, blocks);
        // The card answers CMD17/CMD18 with R1 **before** the data phase,
        // so a refusal (address past the end of the card, internal error)
        // is visible here — and only here (finding 3). A refused command
        // has no data phase: returning without aborting would leave the
        // next wait to run its full budget for data that never comes.
        let (cmdtm, index) = if multi { (READ_MULTIPLE, 18) } else { (READ_SINGLE, 17) };
        let status = self.command(cmdtm, addr)?;
        if let Err(e) = check_r1(index, status, mask) {
            Self::recover_no_abort(Phase::ReadRdy);
            return Err(e);
        }
        // (14)-(17) Buffer Read Ready, then one block from the data port —
        // per block. Little endian (SD-HC 3.00 §1.7 (2)).
        for block in buf.chunks_exact_mut(SECTOR_SIZE) {
            Self::wait_interrupt(INT_READ_RDY, Phase::ReadRdy)?;
            for word in block.chunks_exact_mut(4) {
                word.copy_from_slice(&rd(DATA).to_le_bytes());
            }
        }
        // (19)-(20) Transfer Complete — for CMD18 it also covers the Auto
        // CMD12 (SD-HC 3.00 §3.7.1.2 Note 2).
        Self::wait_interrupt(INT_DATA_DONE, Phase::ReadDone)?;
        if multi {
            // The Auto CMD12 response is in REP[127:96] (SD-HC 3.00 §2.2.7,
            // Table 2-12). A transfer that read the card's last block may
            // carry a spurious OUT_OF_RANGE, here or in the next command
            // (§3.7.1.2 Note 3, SD Phys 9.10 §4.3.3).
            let last = xfer::ends_on_last_block(lba, blocks, self.blocks.load(Ordering::Relaxed));
            check_r1(12, rd(RESP3), xfer::r1_mask(R1_ERROR_MASK, R1_OUT_OF_RANGE, last))?;
            if last {
                OOR_PENDING.store(true, Ordering::Relaxed);
            }
        }
        Ok(())
    }

    /// Write `buf.len() / 512` consecutive blocks at `lba` with **one**
    /// command (PIO): `CMD24` for one block, `CMD25` + Block Count + Auto
    /// CMD12 for more — SD-HC 3.00 §3.7.2.1, steps (1)-(13) and (18)-(20).
    fn write_run(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError> {
        if !self.ready.load(Ordering::Acquire) {
            return Err(BlockError::NotReady);
        }
        let blocks = buf.len() / SECTOR_SIZE;
        let multi = xfer::transfer_for(blocks, compat()) == Transfer::Multiple;
        if !Self::wait_clear(STATUS, SR_DAT_INHIBIT, Phase::DatInhibit) {
            return Err(BlockError::Timeout);
        }
        let addr = self.card_address(lba)?;
        // Debug feature `sd-fault` (TASK/HW08): the fault for this command.
        #[cfg(feature = "sd-fault")]
        let fault = super::emmc_fault::take_write(lba, blocks);
        #[cfg(feature = "sd-fault")]
        let addr = if fault == super::emmc_fault::Mode::OutOfRange {
            self.card_address(self.blocks.load(Ordering::Relaxed) + 0x1000)?
        } else {
            addr
        };
        wr(BLKSIZECNT, xfer::blksizecnt(blocks, SECTOR_SIZE));
        let mask = self.next_r1_mask(lba, blocks);
        // Same as the read side: CMD24/CMD25's R1 arrives before the data
        // phase. This is the check whose absence let a write past the end
        // of the card report success (audit finding 3, driven).
        let (cmdtm, index) = if multi { (WRITE_MULTIPLE, 25) } else { (WRITE_SINGLE, 24) };
        #[cfg(feature = "sd-fault")]
        if fault == super::emmc_fault::Mode::R1 {
            // No command: the R1 path with a status of a card in transfer
            // state (`CURRENT_STATE` = 4, SD Phys 9.10 Table 4-42).
            let e = check_r1(index, R1_WP_VIOLATION | (4 << 9), mask);
            Self::recover_no_abort(Phase::WriteRdy);
            return e;
        }
        let status = self.command(cmdtm, addr)?;
        if let Err(e) = check_r1(index, status, mask) {
            Self::recover_no_abort(Phase::WriteRdy);
            return Err(e);
        }
        #[cfg(feature = "sd-fault")]
        match fault {
            super::emmc_fault::Mode::Dcrc => {
                note_failure(Phase::WriteRdy, INT_ERR_DCRC, false);
                Self::recover(Phase::WriteRdy, INT_ERR_DCRC);
                return Err(BlockError::Io);
            }
            // A flag that never comes: the real wait and its timeout path.
            super::emmc_fault::Mode::TimeoutRdy => {
                Self::wait_interrupt(0, Phase::WriteRdy)?;
            }
            _ => {}
        }
        // (10)-(13) Buffer Write Ready, then one block into the data port —
        // per block. Between blocks of a CMD25 this wait also covers the
        // card's busy at the block gap (Phase::WriteRdy's limit).
        for block in buf.chunks_exact(SECTOR_SIZE) {
            Self::wait_interrupt(INT_WRITE_RDY, Phase::WriteRdy)?;
            for word in block.chunks_exact(4) {
                wr(DATA, u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
            }
        }
        #[cfg(feature = "sd-fault")]
        if fault == super::emmc_fault::Mode::Busy {
            let ms = super::emmc_fault::busy_ms();
            if ms >= Phase::WriteDone.limit_ms() {
                super::emmc_fault::busy_until(cntpct() + ticks_for_ms(cntfrq(), ms));
                Self::wait_interrupt(0, Phase::WriteDone)?;
            }
            let mut held = Budget::new(ms);
            while !held.expired() {
                core::hint::spin_loop();
            }
        }
        // (19)-(20) Transfer Complete: the last block is programmed and the
        // busy released (SD-HC 3.00 §2.2.17 bit 1) — for CMD25 after the
        // Auto CMD12 (§3.7.1.2 Note 2).
        Self::wait_interrupt(INT_DATA_DONE, Phase::WriteDone)?;
        if multi {
            // The card's verdict on the whole write, reported with the Auto
            // CMD12 response (SD-HC 3.00 §2.2.7 Table 2-12).
            check_r1(12, rd(RESP3), R1_ERROR_MASK)?;
        }
        Ok(())
    }
}

/// Outcome of the 4-bit switch at init, for the boot report.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
enum WideBus {
    /// Not attempted: `emmc=compat`.
    Compat = 0,
    /// Card and host are on the 4-bit bus.
    FourBit = 1,
    /// The SCR does not list the 4-bit bus.
    CardOneBit = 2,
    /// `ACMD51 SEND_SCR` failed — stays 1-bit.
    ScrFailed = 3,
    /// `ACMD6` failed or was refused — stays 1-bit.
    Acmd6Failed = 4,
}

/// [`WideBus`] of the last init, as `u32`.
static WIDE_BUS: AtomicU32 = AtomicU32::new(WideBus::Compat as u32);

impl BlockDevice for Emmc {
    fn block_count(&self) -> u64 {
        self.blocks.load(Ordering::Relaxed)
    }

    /// The whole request goes to the card in as few commands as the Block
    /// Count register allows (FS08): chunks of up to 65535 sectors, each
    /// one `CMD18` (or `CMD17` for a single sector). `emmc=compat`: one
    /// `CMD17` per sector.
    fn read_blocks(&self, lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
        if !buf.len().is_multiple_of(SECTOR_SIZE) {
            return Err(BlockError::Unsupported);
        }
        let compat = compat();
        let mut at = lba;
        let mut rest = buf;
        while !rest.is_empty() {
            let n = xfer::next_chunk(rest.len() / SECTOR_SIZE, compat);
            let (run, tail) = rest.split_at_mut(n * SECTOR_SIZE);
            #[cfg(not(feature = "sd-diag"))]
            self.read_run(at, run)?;
            #[cfg(feature = "sd-diag")]
            {
                emmc_diag::op_begin(false, at, n);
                let t0 = cntpct();
                let r = self.read_run(at, run);
                emmc_diag::op_end(false, emmc_diag::ticks_to_us(cntpct().wrapping_sub(t0), cntfrq()), n, r.is_ok());
                r?;
            }
            at += n as u64;
            rest = tail;
        }
        Ok(())
    }

    /// Like [`Emmc::read_blocks`], with `CMD25`/`CMD24`.
    fn write_blocks(&self, lba: u64, buf: &[u8]) -> Result<(), BlockError> {
        if !buf.len().is_multiple_of(SECTOR_SIZE) {
            return Err(BlockError::Unsupported);
        }
        let compat = compat();
        let mut at = lba;
        let mut rest = buf;
        while !rest.is_empty() {
            let n = xfer::next_chunk(rest.len() / SECTOR_SIZE, compat);
            let (run, tail) = rest.split_at(n * SECTOR_SIZE);
            #[cfg(not(feature = "sd-diag"))]
            self.write_run(at, run)?;
            #[cfg(feature = "sd-diag")]
            {
                emmc_diag::op_begin(true, at, n);
                let t0 = cntpct();
                let r = self.write_run(at, run);
                emmc_diag::op_end(true, emmc_diag::ticks_to_us(cntpct().wrapping_sub(t0), cntfrq()), n, r.is_ok());
                r?;
            }
            at += n as u64;
            rest = tail;
        }
        Ok(())
    }
}

/// Initialise the microSD card. Probes EMMC2 (real RPi4 SD slot) then
/// EMMC (QEMU raspi4b), using whichever has a responding card. Returns
/// `false` if neither has a card (the caller then has no block device —
/// never a stubbed success).
///
/// # Safety
/// Boot CPU, single-threaded; the VideoCore firmware has finished using
/// the card.
pub unsafe fn init() -> bool {
    // Idempotent: once a card is up, re-initialising would needlessly
    // re-run the SD power-up sequence. Callers may invoke this from both
    // boot bring-up and the self-test.
    if SDCARD.ready.load(Ordering::Acquire) {
        return true;
    }
    for &candidate in &[EMMC2_BASE, EMMC_BASE] {
        BASE_ADDR.store(candidate, Ordering::Relaxed);
        // Skip a base with no SDHCI controller behind it.
        let ver = rd(SLOTISR_VER);
        if ver == 0 || ver == 0xFFFF_FFFF {
            continue;
        }
        if SDCARD.init_card().is_ok() {
            crate::kprintln!("[emmc] SD card on controller @ 0x{:08x}", candidate);
            report_controller();
            #[cfg(feature = "sd-diag")]
            {
                let s = error_stats();
                DIAG_AT_INIT.0.store(s.io_errors, Ordering::Relaxed);
                DIAG_AT_INIT.1.store(s.timeouts, Ordering::Relaxed);
            }
            // The probe is over: from here a controller error is a real one
            // and gets a console line of its own.
            REPORT_ERRORS.store(true, Ordering::Relaxed);
            return true;
        }
    }
    false
}

/// `(io_errors, timeouts)` when the base probe of the boot ended (debug
/// feature `sd-diag`).
#[cfg(feature = "sd-diag")]
static DIAG_AT_INIT: (AtomicU32, AtomicU32) = (AtomicU32::new(0), AtomicU32::new(0));

/// Microseconds since the counter started (debug feature `sd-diag`).
#[cfg(feature = "sd-diag")]
fn diag_now_us() -> u64 {
    emmc_diag::ticks_to_us(cntpct(), cntfrq())
}

/// Run `f` on the driver half of the SD report (debug feature `sd-diag`,
/// TASK/HW08, `tos_api::bios::sd_diag`).
#[cfg(feature = "sd-diag")]
pub fn with_diag_report<R>(f: impl FnOnce(&tos_api::bios::sd_diag::DriverReport<'_>) -> R) -> R {
    use tos_api::bios::sd_diag::{DriverReport, Failure, LOG_LEN};
    let s = error_stats();
    let mut wait = [0u64; Phase::COUNT];
    emmc_diag::wait_max(&mut wait);
    for w in wait.iter_mut() {
        *w = emmc_diag::ticks_to_us(*w, cntfrq());
    }
    let mut log = [Failure::default(); LOG_LEN];
    let (first, n) = emmc_diag::log(&mut log);
    let [read, write] = emmc_diag::ops();
    f(&DriverReport {
        mode: transfer_mode(),
        io_errors: s.io_errors,
        timeouts: s.timeouts,
        recoveries: s.recoveries,
        nonrecoverable: s.nonrecoverable,
        card_errors: s.card_errors,
        at_init: (DIAG_AT_INIT.0.load(Ordering::Relaxed), DIAG_AT_INIT.1.load(Ordering::Relaxed)),
        last_phase: s.last_phase.index() as u8,
        last_status: s.last_status,
        last_card_status: s.last_card_status,
        phase_names: &PHASE_NAMES,
        bit_names: &ERROR_BIT_NAMES,
        bits: &s.bits,
        r1_names: &R1_BIT_NAMES,
        r1_bits: &s.card_bits,
        timeouts_by_phase: &s.timeouts_by_phase,
        wait_max_us: &wait,
        write,
        read,
        first,
        log: &log[..n],
    })
}

/// One-shot boot diagnostic: the values that decide whether the bus is
/// clocked the way the specification requires — measurement **M1** of
/// `docs/api/hw/emmc-audit.md` §5, for the controller base actually in use.
///
/// **Reported, not acted on.** Whether the guessed 100 MHz fallback base
/// clock is right (audit finding 9) and whether the divider is the 8-bit or
/// the 10-bit kind (finding 10) are their own tickets; this print is what
/// settles them on real hardware, where QEMU cannot (QEMU wires its card to
/// the legacy EMMC, so its `CAPABILITIES` are not the Pi's EMMC2 ones).
fn report_controller() {
    let ver = rd(SLOTISR_VER);
    let caps = rd(CAPABILITIES);
    let c0 = rd(CONTROL0);
    let c1 = rd(CONTROL1);
    // SLOTISR_VER[23:16] = Specification Version Number (SD-HC 3.00
    // §2.2.28): 0x00 = 1.00, 0x01 = 2.00, 0x02 = 3.00. Below 0x02 the
    // clock divider is the 8-bit kind and only powers of two are legal
    // (§2.2.14) — which our divider arithmetic does not know (finding 10).
    let spec_ver = (ver >> 16) & 0xFF;
    // CAPABILITIES[15:8] = Base Clock Frequency For SD Clock, in MHz
    // (SD-HC §2.2.26). 0 means the host system has to tell the driver by
    // another method (§3.2.1) — the mailbox rates below are that method.
    let base_mhz = (caps >> 8) & 0xFF;
    // CONTROL1[15:8] (low) + [7:6] (high) = the 10-bit SDCLK divider;
    // SDCLK = base / (2 × div), a divider of 0 meaning "base clock".
    let div = ((c1 >> 8) & 0xFF) | (((c1 >> 6) & 0x3) << 8);
    let base_hz = if base_mhz == 0 { 100_000_000 } else { base_mhz * 1_000_000 };
    let sdclk = if div == 0 { base_hz } else { base_hz / (2 * div) };
    crate::kprintln!(
        "[emmc] M1 SLOTISR_VER=0x{:08x} (SDHCI spec 0x{:02x}) CAPABILITIES=0x{:08x} (base {} MHz{})",
        ver,
        spec_ver,
        caps,
        base_mhz,
        if base_mhz == 0 { ", guessed 100" } else { "" },
    );
    crate::kprintln!(
        "[emmc] M1 CONTROL0=0x{:08x} CONTROL1=0x{:08x} div={} -> SDCLK ~{} Hz",
        c0,
        c1,
        div,
        sdclk,
    );
    // Mailbox `get clock rate` (tag 0x00030002) for both EMMC clock ids —
    // `docs/hardware/emmc-sd.md` §2 ("clock id 1 = EMMC") and
    // `references/circle/include/circle/bcmpropertytags.h:182,186`
    // (`CLOCK_ID_EMMC = 1`, `CLOCK_ID_EMMC2 = 12`). 0 = the firmware has no
    // rate for that clock.
    let rate = |id| super::mailbox::get_clock_rate(id).unwrap_or(0);
    crate::kprintln!(
        "[emmc] M1 mailbox clock rate: EMMC(1)={} Hz EMMC2(12)={} Hz",
        rate(super::mailbox::CLOCK_ID_EMMC),
        rate(super::mailbox::CLOCK_ID_EMMC2),
    );

    // FS08: which transfer path this boot uses, and why.
    let wide = WIDE_BUS.load(Ordering::Relaxed);
    crate::kprintln!(
        "[emmc] FS08 mode: {} — bus {}-bit ({}, SCR SD_BUS_WIDTHS=0x{:x}), CONTROL0[1] Data Transfer Width={}",
        if compat() { "emmc=compat (CMD17/CMD24 per sector)" } else { "multi-block (CMD18/CMD25 + Auto CMD12)" },
        if BUS_4BIT.load(Ordering::Relaxed) { 4 } else { 1 },
        match wide {
            0 => "not attempted, emmc=compat",
            1 => "ACMD6 ok",
            2 => "card lists no 4-bit",
            3 => "ACMD51 SEND_SCR failed",
            _ => "ACMD6 failed",
        },
        SCR_WIDTHS.load(Ordering::Relaxed),
        (c0 >> 1) & 1,
    );

    // Whatever the identification itself ran into, once (the per-failure
    // console lines are off until here — see `note_failure`). On a machine
    // where the first probed base has no card this is where its expected
    // command timeouts show up, instead of scattered through the boot log.
    let s = error_stats();
    crate::kprint!(
        "[emmc] M1 during identification: {} error(s), {} timeout(s), {} recovery(ies)",
        s.io_errors,
        s.timeouts,
        s.recoveries,
    );
    for (i, n) in s.bits.iter().enumerate() {
        if *n != 0 {
            crate::kprint!(" {}={}", ERROR_BIT_NAMES[i], n);
        }
    }
    crate::kprintln!("");
}

#[cfg(test)]
mod tests {
    //! Pure timeout arithmetic. **These do not run in the workspace test
    //! command**: the `kernel` crate is pinned to
    //! `aarch64-unknown-none-softfloat` via `forced-target` and is not
    //! host-linkable (`tools/xtask/src/main.rs:573-582`), so
    //! `cargo run -p xtask -- test` cannot execute them. They are kept
    //! because they are the whole of the arithmetic that decides a timeout,
    //! and they were driven on the host as a verbatim copy while the change
    //! was written.
    use super::*;

    #[test]
    fn a_millisecond_is_the_counter_frequency_over_a_thousand() {
        // The two frequencies these boards report: 54 MHz (Pi 4, measured
        // in `measurements/measurement-2026-09-25-fs04`) and 62.5 MHz (QEMU virt).
        assert_eq!(ticks_for_ms(54_000_000, 1), 54_000);
        assert_eq!(ticks_for_ms(54_000_000, 1_000), 54_000_000);
        assert_eq!(ticks_for_ms(62_500_000, 100), 6_250_000);
        // …and the budgets the driver actually opens, in ticks:
        assert_eq!(ticks_for_ms(54_000_000, WRITE_BUSY_MS), 54_000_000);
        assert_eq!(ticks_for_ms(54_000_000, READ_MS), 10_800_000);
    }

    #[test]
    fn a_budget_is_never_zero_ticks() {
        // A counter slow enough that a millisecond is less than one tick
        // still gets a tick, so a wait polls at least once more.
        assert_eq!(ticks_for_ms(100, 1), 1);
        assert_eq!(ticks_for_ms(1, 1), 1);
    }

    #[test]
    fn the_budget_does_not_overflow_at_any_plausible_frequency() {
        // 1 s at 4 GHz still fits; nothing here can wrap into a short wait.
        assert_eq!(ticks_for_ms(4_000_000_000, 1_000), 4_000_000_000);
    }

    #[test]
    fn expiry_is_the_elapsed_difference_not_the_absolute_value() {
        assert!(!reached(1_000, 1_999, 1_000));
        assert!(reached(1_000, 2_000, 1_000));
        assert!(reached(1_000, 5_000, 1_000));
    }

    #[test]
    fn expiry_survives_the_counter_wrapping() {
        // A wait that starts 500 ticks below the wrap and runs 400 ticks
        // past it has taken 900 ticks, not a negative or gigantic number.
        let start = u64::MAX - 499; // 500 ticks before the counter wraps
        let now = 400u64; // 500 + 400 = 900 ticks later
        assert!(!reached(start, now, 1_000));
        assert!(reached(start, now, 900));
        assert!(reached(start, now, 100));
    }

    #[test]
    fn each_phase_gets_the_limit_its_own_spec_clause_gives_it() {
        // SD Phys 6.00 §4.6.2.2 (write busy) vs. §4.6.2.1 (read) — the two
        // are not the same number, which is the point of finding 1.
        assert_eq!(Phase::WriteDone.limit_ms(), WRITE_BUSY_MS);
        assert_eq!(Phase::DatInhibit.limit_ms(), WRITE_BUSY_MS);
        assert_eq!(Phase::ReadRdy.limit_ms(), READ_MS);
        assert_eq!(Phase::ReadDone.limit_ms(), READ_MS);
        assert_eq!(Phase::CmdDone.limit_ms(), CTRL_MS);
        assert_eq!(Phase::CmdInhibit.limit_ms(), CTRL_MS);
        assert_eq!(Phase::WriteRdy.limit_ms(), WRITE_BUSY_MS);
        // The write budget must exceed the 500 ms the specification allows
        // the card for the last busy of a write.
        assert!(WRITE_BUSY_MS > 500);
        // …and the read budget must be at least the 100 ms minimum.
        assert!(READ_MS >= 100);
    }

    #[test]
    fn the_phase_table_and_the_names_line_up() {
        for (i, p) in PHASES.iter().enumerate() {
            assert_eq!(p.index(), i);
        }
        assert_eq!(PHASES.len(), Phase::COUNT);
        assert_eq!(PHASE_NAMES.len(), Phase::COUNT);
        assert_eq!(ERROR_BIT_NAMES.len(), ERROR_BIT_MASKS.len());
    }

    #[test]
    fn a_data_phase_takes_both_resets_a_command_phase_only_the_cmd_line() {
        // SD-HC §3.8: an aborted data transaction takes both software
        // resets; §3.10.1: a command-line error only the CMD line.
        assert_eq!(Phase::WriteDone.resets(0), C1_SRST_CMD | C1_SRST_DATA);
        assert_eq!(Phase::DatInhibit.resets(0), C1_SRST_CMD | C1_SRST_DATA);
        assert_eq!(Phase::CmdDone.resets(INT_ERR_CTO), C1_SRST_CMD);
        assert_eq!(Phase::CmdInhibit.resets(0), C1_SRST_CMD);
        // …unless a DAT-line error bit says otherwise.
        assert_eq!(Phase::CmdDone.resets(INT_ERR_DCRC), C1_SRST_CMD | C1_SRST_DATA);
    }

    #[test]
    fn the_error_masks_are_the_bits_the_audit_names() {
        // `docs/api/hw/emmc-audit.md` §5 M2: 16 CTO, 17 CCRC, 20 DTO,
        // 21 DCRC, 22 DEND.
        assert_eq!(INT_ERR_CTO, 1 << 16);
        assert_eq!(INT_ERR_CCRC, 1 << 17);
        assert_eq!(INT_ERR_DTO, 1 << 20);
        assert_eq!(INT_ERR_DCRC, 1 << 21);
        assert_eq!(INT_ERR_DEND, 1 << 22);
        assert_eq!(INT_ERR_CMD_MASK & INT_ERR_DAT_MASK, 0);
        assert_eq!((INT_ERR_CMD_MASK | INT_ERR_DAT_MASK) & !INT_ERROR_MASK, 0);
    }

    #[test]
    fn the_spin_fallback_is_bounded() {
        let mut b = Budget::Spins(3);
        assert!(!b.expired());
        assert!(!b.expired());
        assert!(!b.expired());
        assert!(b.expired());
        assert!(b.expired());
    }
}
