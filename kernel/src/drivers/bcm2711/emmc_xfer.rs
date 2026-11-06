//! Pure transfer planning for the EMMC2 SD driver (FS08).
//!
//! Everything here is byte/bit arithmetic with no register access and no
//! kernel dependency, so the host test suite (`tools/xtask`, which includes
//! this file by path) can run its tests; `emmc.rs` does the I/O.
//!
//! Sources (Track B, `docs/hardware/emmc-sd.md` "Sources", pinned in
//! `references/README.md`):
//! * **SD-HC 3.00** = SD Host Controller Simplified Specification Ver. 3.00
//!   (`references/sd-spec/archive/…Ver3.00.pdf`) — the edition matching the
//!   controller's `SLOTISR_VER` (see `docs/hardware/emmc-sd.md` §8).
//! * **SD Phys 9.10** = SD Physical Layer Simplified Specification Ver. 9.10.

// ── Transfer Mode register (SD-HC 3.00 §2.2.5, offset 00Ch) ─────────────
//
// The BCM groups it as the low half of the 32-bit `CMDTM` word, with the
// Command register (00Eh) in the high half (`docs/hardware/emmc-sd.md` §1).

/// Bit 1 `Block Count Enable` — the Block Count register is used
/// (SD-HC 3.00 §2.2.5, Table 2-7; Table 2-8 "Multiple Transfer").
pub const TM_BLKCNT_EN: u32 = 1 << 1;
/// Bits 3:2 `Auto CMD Enable` = `01b` **Auto CMD12 Enable**: the controller
/// issues CMD12 itself when the last block has been transferred
/// (SD-HC 3.00 §2.2.5, Table 2-7; §1.11 "Auto CMD12").
pub const TM_AUTO_CMD12: u32 = 1 << 2;
/// Bit 4 `Data Transfer Direction Select`, 1 = read (card to host)
/// (SD-HC 3.00 §2.2.5).
pub const TM_DAT_CARD2HOST: u32 = 1 << 4;
/// Bit 5 `Multi / Single Block Select`, 1 = multiple block
/// (SD-HC 3.00 §2.2.5).
pub const TM_MULTI_BLOCK: u32 = 1 << 5;

/// The Transfer Mode bits of a multiple-block command: multi-block, block
/// count enabled (Table 2-8: "1 / 1 / Not Zero → Multiple Transfer") and
/// Auto CMD12 — §1.11: *"The Host Driver should set Auto CMD12 Enable in the
/// Transfer Mode register when issuing a multiple block transfer command."*
pub const TM_MULTI: u32 = TM_MULTI_BLOCK | TM_BLKCNT_EN | TM_AUTO_CMD12;

/// Largest block count one command can carry: the Block Count register is
/// 16 bits wide, `FFFFh` = 65535 blocks (SD-HC 3.00 §2.2.3, Table 2-5).
/// `0000h` is "Stop Count" — no blocks — so a count is always 1..=this.
pub const MAX_BLOCKS_PER_CMD: usize = 0xFFFF;

/// How one request of `blocks` sectors is put on the bus.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Transfer {
    /// `CMD17` / `CMD24` — one block per command.
    Single,
    /// `CMD18` / `CMD25` with Block Count + Auto CMD12.
    Multiple,
}

/// The command kind for a chunk of `blocks` sectors (`blocks` >= 1).
///
/// One block always goes as `CMD17`/`CMD24`; so does everything in the
/// `emmc=compat` mode, which is the pre-FS08 driver (one single-block
/// command per sector).
pub const fn transfer_for(blocks: usize, compat: bool) -> Transfer {
    if compat || blocks <= 1 {
        Transfer::Single
    } else {
        Transfer::Multiple
    }
}

/// Size of the next command's chunk when `remaining` sectors are still to
/// go: all of them, capped at [`MAX_BLOCKS_PER_CMD`]. In compat mode every
/// chunk is a single sector.
pub const fn next_chunk(remaining: usize, compat: bool) -> usize {
    if compat && remaining > 0 {
        1
    } else if remaining > MAX_BLOCKS_PER_CMD {
        MAX_BLOCKS_PER_CMD
    } else {
        remaining
    }
}

/// The `BLKSIZECNT` word for `blocks` blocks of `block_size` bytes:
/// `[31:16]` Block Count (SD-HC 3.00 §2.2.3), `[11:0]` Transfer Block Size
/// (§2.2.2). `blocks` must be 1..=[`MAX_BLOCKS_PER_CMD`].
pub const fn blksizecnt(blocks: usize, block_size: usize) -> u32 {
    ((blocks as u32) << 16) | (block_size as u32 & 0xFFF)
}

/// Does a read of `blocks` sectors at `lba` end on the card's **last**
/// block (`capacity` blocks in all)?
///
/// SD Phys 9.10 §4.3.3: *"When the last block of user area is read using
/// CMD18, the host should ignore OUT_OF_RANGE error that may occur even the
/// sequence is correct."* SD-HC 3.00 §3.7.1.2 Note 3 says where it shows
/// up: *"in the response of Auto CMD12 or in the response of the next
/// memory command."*
pub const fn ends_on_last_block(lba: u64, blocks: usize, capacity: u64) -> bool {
    capacity != 0 && lba + blocks as u64 == capacity
}

/// The R1 error mask to apply to a response, with `out_of_range` removed
/// when the spurious `OUT_OF_RANGE` of [`ends_on_last_block`] may be in it.
pub const fn r1_mask(full: u32, out_of_range: u32, ignore_out_of_range: bool) -> u32 {
    if ignore_out_of_range {
        full & !out_of_range
    } else {
        full
    }
}

// ── Card state, R1 `CURRENT_STATE` (SD Phys 9.10 Table 4-42, bits 12:9) ───

/// `CURRENT_STATE` of an R1 word. Status type `S`, clear condition `A`: it
/// says what the card is doing now.
pub const fn r1_state(r1: u32) -> u32 {
    (r1 >> 9) & 0xF
}

/// `tran`, the state every data command needs (Table 4-35).
pub const CARD_TRAN: u32 = 4;
/// `data`: the card is sending a block.
pub const CARD_DATA: u32 = 5;
/// `rcv`: the card is waiting for a block.
pub const CARD_RCV: u32 = 6;
/// `prg`: the card is programming; only `CMD0`, `CMD7`, `CMD13` and `CMD15`
/// are legal (Table 4-35).
pub const CARD_PRG: u32 = 7;

/// May the host send `CMD12` to a card in `state`? SD Phys 9.10 Table 4-35,
/// row `CMD12`: only out of `data` and `rcv`. In `tran` (below SDUC) and `prg`
/// it is illegal: the card does not answer and sets `ILLEGAL_COMMAND`
/// (§4.6.1). The driver stops a transfer only when one is really running.
pub const fn transfer_running(state: u32) -> bool {
    state == CARD_DATA || state == CARD_RCV
}

/// What the recovery does to the card, by the state `CMD13` reported (`None`:
/// it did not answer).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct CardRecovery {
    /// Stop the running transfer with `CMD12`.
    pub stop: bool,
    /// Ask the card again before the next data command: it may be programming,
    /// and a data command in `prg` gets no answer.
    pub wait_ready: bool,
}

/// The plan for a card in `state`. An unknown state sends nothing blind; the
/// next access asks again.
pub const fn card_recovery(state: Option<u32>) -> CardRecovery {
    match state {
        Some(s) if transfer_running(s) => CardRecovery { stop: true, wait_ready: true },
        Some(CARD_TRAN) => CardRecovery { stop: false, wait_ready: false },
        _ => CardRecovery { stop: false, wait_ready: true },
    }
}

/// The R1 bits that refuse a data command, out of the full Type-`E` set.
///
/// `ILLEGAL_COMMAND` is not one of them: its clear condition is `B`, "Always
/// related to the previous command" (SD Phys 9.10 Table 4-42), and a command
/// that is itself illegal is not answered at all (§4.6.1). So when a data
/// command's R1 carries the bit, the bit is about the command **before** it,
/// and this command **was accepted**: the card has moved on to `rcv` or
/// `data`, and the data phase must run. Measured on the Pi: `CMD24` answered
/// `0x00400900`, taken for a refusal, the next command unanswered because the
/// card was still in `rcv` (`measurements/measurement-2026-11-06-hw08-pi/RUNINFO.TXT`).
pub const fn data_command_mask(mask: u32, illegal_command: u32) -> u32 {
    mask & !illegal_command
}

// ── SCR register (SD Phys 9.10 §5.6) ─────────────────────────────────────

/// Size of the SCR as `ACMD51` sends it: 64 bits (SD Phys 9.10 §5.6).
pub const SCR_BYTES: usize = 8;

/// `SD_BUS_WIDTHS` bit 2 = "4 bit (DAT0-3)" (SD Phys 9.10 §5.6, Table 5-21).
pub const SCR_BUS_WIDTH_4: u8 = 1 << 2;

/// The `SD_BUS_WIDTHS` field (SCR `[51:48]`) from the SCR as received.
///
/// The SCR is "wide width data", which *"is shifted from the MSB bit"*
/// (SD Phys 9.10 §3.6.1, Data Packet Format): the first byte on the bus —
/// the first byte in `scr` — holds SCR `[63:56]`, the second `[55:48]`.
/// `[51:48]` is therefore the low nibble of byte 1.
pub const fn scr_bus_widths(scr: &[u8; SCR_BYTES]) -> u8 {
    scr[1] & 0x0F
}

/// Does the card's SCR say it supports the 4-bit bus?
pub const fn scr_supports_4bit(scr: &[u8; SCR_BYTES]) -> bool {
    scr_bus_widths(scr) & SCR_BUS_WIDTH_4 != 0
}

/// `ACMD6 SET_BUS_WIDTH` argument for the 4-bit bus: bits `[1:0]` = `'10'`
/// (SD Phys 9.10 §4.7.4, application command table: *"'00'=1bit or '10'=4
/// bits bus"*).
pub const ACMD6_ARG_4BIT: u32 = 0b10;

/// Host Control 1 bit 1 `Data Transfer Width`, 1 = 4-bit mode
/// (SD-HC 3.00 §2.2.10; byte 0 of the BCM's `CONTROL0` word).
pub const HC1_DATA_WIDTH_4BIT: u32 = 1 << 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_block_is_always_a_single_block_command() {
        assert_eq!(transfer_for(1, false), Transfer::Single);
        assert_eq!(transfer_for(1, true), Transfer::Single);
        assert_eq!(transfer_for(2, false), Transfer::Multiple);
        assert_eq!(transfer_for(128, false), Transfer::Multiple);
        assert_eq!(transfer_for(MAX_BLOCKS_PER_CMD, false), Transfer::Multiple);
    }

    #[test]
    fn compat_mode_is_single_block_only() {
        assert_eq!(transfer_for(128, true), Transfer::Single);
        assert_eq!(next_chunk(128, true), 1);
        assert_eq!(next_chunk(1, true), 1);
        assert_eq!(next_chunk(0, true), 0);
    }

    #[test]
    fn requests_are_split_at_the_block_count_register_width() {
        assert_eq!(next_chunk(1, false), 1);
        assert_eq!(next_chunk(128, false), 128);
        assert_eq!(next_chunk(0xFFFF, false), 0xFFFF);
        assert_eq!(next_chunk(0x10000, false), 0xFFFF);
        assert_eq!(next_chunk(200_000, false), 0xFFFF);
        // Walking a big request: every chunk is 1..=65535 and they add up.
        let mut left = 200_000usize;
        let mut chunks = 0;
        while left > 0 {
            let c = next_chunk(left, false);
            assert!((1..=MAX_BLOCKS_PER_CMD).contains(&c));
            left -= c;
            chunks += 1;
        }
        assert_eq!(chunks, 4); // 3 × 65535 + 3395
    }

    #[test]
    fn blksizecnt_packs_count_and_size() {
        assert_eq!(blksizecnt(1, 512), 0x0001_0200);
        assert_eq!(blksizecnt(128, 512), 0x0080_0200);
        assert_eq!(blksizecnt(0xFFFF, 512), 0xFFFF_0200);
        assert_eq!(blksizecnt(1, SCR_BYTES), 0x0001_0008);
    }

    #[test]
    fn multi_block_transfer_mode_bits() {
        // SD-HC 3.00 Table 2-7: bit 1 BCE, bits 3:2 = 01b ACMD12, bit 4 read,
        // bit 5 multi.
        assert_eq!(TM_MULTI, 0x26);
        assert_eq!(TM_MULTI | TM_DAT_CARD2HOST, 0x36);
        // Auto CMD Enable is the two-bit field 3:2 holding 01b.
        assert_eq!((TM_AUTO_CMD12 >> 2) & 0b11, 0b01);
    }

    #[test]
    fn out_of_range_is_ignored_only_at_the_end_of_the_card() {
        assert!(ends_on_last_block(1000 - 16, 16, 1000));
        assert!(!ends_on_last_block(1000 - 17, 16, 1000));
        assert!(!ends_on_last_block(0, 16, 0)); // capacity unknown
        let full = 0xFDF9_8008u32;
        let oor = 1u32 << 31;
        assert_eq!(r1_mask(full, oor, false), full);
        assert_eq!(r1_mask(full, oor, true), full & !oor);
        assert_eq!(r1_mask(full, oor, true) & oor, 0);
    }

    #[test]
    fn cmd12_only_out_of_data_and_rcv() {
        // SD Phys 9.10 Table 4-35, row CMD12.
        for state in 0..=15 {
            assert_eq!(transfer_running(state), state == 5 || state == 6, "state {state}");
        }
    }

    #[test]
    fn the_state_is_bits_12_to_9() {
        // The two words of the Pi run: tran, ready for data, with and without
        // ILLEGAL_COMMAND.
        assert_eq!(r1_state(0x0040_0900), CARD_TRAN);
        assert_eq!(r1_state(0x0000_0900), CARD_TRAN);
        assert_eq!(r1_state(0x0000_0E00), CARD_PRG);
        assert_eq!(r1_state(0x0000_0C00), CARD_RCV);
        assert_eq!(r1_state(0x0000_0A00), CARD_DATA);
    }

    #[test]
    fn the_recovery_stops_only_a_running_transfer_and_waits_unless_tran() {
        let plan = |stop, wait_ready| CardRecovery { stop, wait_ready };
        assert_eq!(card_recovery(Some(CARD_RCV)), plan(true, true));
        assert_eq!(card_recovery(Some(CARD_DATA)), plan(true, true));
        assert_eq!(card_recovery(Some(CARD_TRAN)), plan(false, false));
        assert_eq!(card_recovery(Some(CARD_PRG)), plan(false, true));
        assert_eq!(card_recovery(None), plan(false, true));
    }

    #[test]
    fn illegal_command_does_not_refuse_a_data_command() {
        let full = 0xFDF9_8008u32;
        let illegal = 1u32 << 22;
        let mask = data_command_mask(full, illegal);
        // The Pi's word.
        assert_eq!(0x0040_0900 & mask, 0);
        // A real refusal still is one, also with the stale bit beside it.
        assert_ne!((1u32 << 26) & mask, 0);
        assert_ne!((1u32 << 31 | illegal) & mask, 0);
    }

    #[test]
    fn scr_bus_widths_is_the_low_nibble_of_the_second_byte() {
        // A typical SDHC SCR: SCR_STRUCTURE 0, SD_SPEC 2, SD_BUS_WIDTHS 0101b.
        let scr = [0x02, 0x35, 0x80, 0x00, 0, 0, 0, 0];
        assert_eq!(scr_bus_widths(&scr), 0x5);
        assert!(scr_supports_4bit(&scr));
        // 1-bit only (bit 0 alone): not 4-bit capable.
        let one = [0x02, 0x31, 0, 0, 0, 0, 0, 0];
        assert!(!scr_supports_4bit(&one));
        // The high nibble of byte 1 (DATA_STAT_AFTER_ERASE, SD_SECURITY)
        // must not leak into the field.
        let hi = [0x00, 0xF1, 0, 0, 0, 0, 0, 0];
        assert_eq!(scr_bus_widths(&hi), 0x1);
    }

    #[test]
    fn acmd6_and_host_width_constants() {
        assert_eq!(ACMD6_ARG_4BIT, 2);
        assert_eq!(HC1_DATA_WIDTH_4BIT, 0x2);
    }
}
