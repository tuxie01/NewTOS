# `Rsconf` (XBIOS 0x0F)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/xbios.c:376` (`rsconf` wrapper) and the handlers
> `bios/serport.c:217` (`rsconf_mfp`: `rsconf1` / `rsconfTT`),
> `bios/serport.c:775` (`rsconf_scc`: `rsconfA` / `rsconfB`) and
> `bios/serport.c:945` (`rsconf_dummy`). Re-validated 2026-10-30 (XBIOS06):
> ABI sign-extension, per-machine handler selection, the Iorec(0) writes.
> Decision analysis: `docs/analysis/xbios06-rsconf-profile-2026-10-30.md`.

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `-1` "don't change" per parameter, the
> **`speed == -2` query** convention, the baud table, the flow-control
> constants, the packed old-register return. Reference: the MFP 68901
> register mechanics.

## EmuTOS source

**Primary location:** `bios/xbios.c:376` —
`static ULONG rsconf(WORD baud, WORD ctrl, WORD ucr, WORD rsr, WORD tsr, WORD
scr)`, which forwards to `(*rsconfptr)(…)`. Slot 0x0F of `xbios_vecs[]`
(`bios/xbios.c:1305`).

**Installed handler / vector:** `rsconfptr` (`bios/serport.c:74`). Which
handler it points at depends on the machine (`init_bconmap`,
`bios/serport.c:951-1004`, applies when `BCONMAP_AVAILABLE`,
`bios/serport.h:18`):

| Machine | `mapped_device` | Entry the vector is copied from | `Rsconf` handler | Port |
|---|---|---|---|---|
| plain ST/STE (`maptabsize` 1) | 6 (`:966`) | `maptable[0]` = `maptable_mfp` (`:98`, `:972`) | `rsconf1` -> `rsconf_mfp(MFP_BASE,&iorec1,..)` (`:386-389`) | MFP |
| TT | 6 | `maptable[0]` (`:972`, `:990-994`) | `rsconf1` (MFP); TT-MFP = device 8 (`rsconfTT`, `:488-490`) | MFP |
| Falcon, MSTe, ARAnyM | **7** (`:966`) | `maptable[1]` = `maptable_port_b` (`:975-979`, `:997-1003`) | `rsconfB` -> `rsconf_scc(&scc->portB,&iorecB,..)` (`:872-876`) | **SCC B** |

(`rsconfptr = maptabptr->Rsconf` at `:1002`; before `init_bconmap` runs,
`init_serport` sets `rsconfptr = rsconf1`, `:1019`.) A `Bconmap` switch
re-copies the vector (`:1114-1121`). `rsconf_dummy` (`:945-948`) returns 0 and
backs the unused `maptable` slots (`:956-960`).

**Helpers called:**
- `bios/serport.c:217` — `rsconf_mfp()` — the MFP USART implementation
  (ST MFP and TT second MFP).
- `bios/serport.c:775` — `rsconf_scc()` — the SCC implementation (Falcon
  port B, Falcon/TT port A).
- `bios/serport.c:945` — `rsconf_dummy()` — returns 0.

**Headers / type definitions used:**
- `bios/serport.h:23-38` — the 16 baud constants (`B19200`..`B50`).
- `bios/serport.h:46-52` — flow-control constants (`FLOW_CTRL_NONE`..`_BOTH`).
- `bios/serport.h:57-65` — `EXT_IOREC` (`.baudrate` read by the query form).

**System variables / globals read/written:**
- `iorec->baudrate` (read on the query path; the record is the handler's own
  `iorec1`/`iorecB`/`iorecA`/`iorecTT`, which is what `Iorec(0)` returns while
  that port is mapped, `bios/xbios.c:339-340`).
- Writes to that record (the **EXT_IOREC** after the two `IOREC`s,
  `bios/serport.h:57-65`): MFP handler: `baudrate` (`:232`, only when
  `0 <= baud <= 15`) and `flowctrl` (`:238`, only when `0 <= ctrl <= 3`);
  SCC handler additionally `ucr` (`:811`), `datamask` (`:830`), `wr5`
  (`:832`, `:857`) (see "Side effects"). The MFP handler does **not** write
  `iorec->ucr`.
- The MFP UART registers (MFP handler) / SCC WR3,4,5,12,13 (SCC handler).

**Build configuration affecting this function:**
- `CONF_WITH_MFP` / `CONF_WITH_SCC` / `CONF_WITH_TT_MFP` — which handler is
  installed and what registers it touches. On the NewTOS PL011 profile a
  substitute handler maps these onto the UART.

---

## Source extraction summary

`Rsconf` configures the RS-232 port. The XBIOS wrapper just calls the active
handler `*rsconfptr`; the default is the MFP handler. Each parameter that is
`-1` leaves its register untouched (`ucr`/`rsr`/`tsr`/`scr` are each applied
only when `>= 0`).

`baud` selects the rate from a 16-entry index (0 = 19200 … 15 = 50). The
special value **`baud == -2`** is a **query**: the handler returns the current
baud code (`rs232iorecptr->baudrate`) without changing anything. `ctrl`
selects the flow-control mode (0 none / 1 XON-XOFF / 2 RTS-CTS / 3 both) when
`>= 0`.

The `ULONG` return packs the **old** MFP register state as
`(old_ucr << 24) | (old_rsr << 16) | (old_tsr << 8)` — note the low byte
(`scr`) is **not** included by the MFP handler. (The SCC handler packs a
different subset.) So callers that only care about the previous baud read the
low 16 bits after a `baud`-only call, but the canonical query is `baud == -2`.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x0F
- **Stack frame on entry (m68k):** `Rsconf(speed, ctrl, ucr, rsr, tsr, scr)` =
  `xbios_v_wwwwww(15, …)` (`include/xbiosbind.h:33`):
  ```
  +12: scr   (word)
  +10: tsr   (word)
   +8: rsr   (word)
   +6: ucr   (word)
   +4: ctrl  (word)
   +2: speed (word)
    0: function_id (word) = 0x000F
  ```
- **Total parameter bytes:** 14 (W × 6).
- **Native (AArch64) ABI:** ID `0x0F` in `x8`; args in `x0..x5`. **Contract:**
  the `ULONG` result is a 32-bit `d0` on the m68k and arrives in `x0` as a
  **sign-extended `i64`**, like every LONG trap result
  (`tos-api/src/trap.rs:6@TrapArgs`: "return value in `x0` (sign-extended
  `i64`)"; `docs/COMPATIBILITY.md` "LONG results in `x0`: sign-extended";
  dumper rule `docs/TOS-REFERENCE-DUMPER.md` section 3 `ret_hi`). Because the
  packed return puts the old UCR in bits 31..24, **every UCR with bit 7 set
  (the power-up value is `0x88`) makes the result negative**:
  `0x88xxxxxx` -> `0xFFFFFFFF_88xxxxxx`. The `-2` query returns a small
  non-negative baud code (`0x00000000_0000000n`), unaffected.
  - **Oracle (measured):** `Rsconf(-1 x6)` = `0xFFFFFFFF_88018100` (TT, MFP),
    `0xFFFFFFFF_88000000` (Falcon, SCC B) (`tosdump/cases/bios-r1.tsv:105@Rsconf`,
    `tosdump/cases/xbios-chardev-r1.tsv:31@Rsconf`; probe S02 in
    `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/XCPROBE.LOG`
    and `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-falcontc/XCPROBE.LOG`).
  - **NewTOS today (deviates from the contract, pending XBIOS06):** the result
    is **zero-extended**: `tos-api/src/xbios/system.rs:286@old`
    (`old as i64`, `old: u32`) gives `0x00000000_88010100`
    (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b/XCPROBE.LOG`,
    S02). The code comments at `tos-api/src/xbios/system.rs:253@rsconf` and
    `:285` say "zero-extended" and describe today's code, not the contract;
    they change with the developer fix.
  - **Correction history:** the text before XBIOS07 said "zero-extended" as
    the contract; that was wrong (XBIOS07). XBIOS06 (2026-10-30) removes the
    last ambiguity: contract = sign-extended, implementation = zero-extended
    until fixed.
  - **Which register bytes** the oracle returns is a per-machine device
    parameter (next section), separate from the extension rule.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `speed` | `WORD` (16) — baud index; `-1` keep, `-2` query |
  | +4         | `x1`        | `ctrl`  | `WORD` (16) — flow mode; `-1` keep |
  | +6         | `x2`        | `ucr`   | `WORD` (16) — MFP UCR; `-1` keep |
  | +8         | `x3`        | `rsr`   | `WORD` (16) — MFP RSR; `-1` keep |
  | +10        | `x4`        | `tsr`   | `WORD` (16) — MFP TSR; `-1` keep |
  | +12        | `x5`        | `scr`   | `WORD` (16) — MFP SCR; `-1` keep |

---

## Inputs

| Position | Name    | Type   | Size | Source location      | Notes |
|----------|---------|--------|------|----------------------|-------|
| arg0     | `speed` | `WORD` | 2    | `bios/serport.c:222` | baud index 0..15; `-1` keep; **`-2` query** |
| arg1     | `ctrl`  | `WORD` | 2    | `bios/serport.c:237-238` (MFP), `:792-793` (SCC) | flow mode 0..3; `-1` keep |
| arg2..5  | `ucr`/`rsr`/`tsr`/`scr` | `WORD` | 2 | `bios/serport.c:239-246` | MFP 68901 regs; each applied only if `>= 0` |

### Baud index (`bios/serport.h:23-38`)

| Idx | Baud | Idx | Baud | Idx | Baud | Idx | Baud |
|----:|-----:|----:|-----:|----:|-----:|----:|-----:|
| 0 | 19200 | 4 | 2400 | 8 | 600 | 12 | 134 |
| 1 | 9600  | 5 | 2000 | 9 | 300 | 13 | 110 |
| 2 | 4800  | 6 | 1800 | 10 | 200 | 14 | 75 |
| 3 | 3600  | 7 | 1200 | 11 | 150 | 15 | 50 |

### Flow-control mode (`bios/serport.h:46-52`)

| Value | Constant | Meaning |
|------:|----------|---------|
| 0 | `FLOW_CTRL_NONE` | none |
| 1 | `FLOW_CTRL_SOFT` | XON/XOFF |
| 2 | `FLOW_CTRL_HARD` | RTS/CTS |
| 3 | `FLOW_CTRL_BOTH` | XON/XOFF + RTS/CTS |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** ULONG
- **Encoding (MFP handler: ST MFP, TT second MFP):** `(old_ucr << 24) |
  (old_rsr << 16) | (old_tsr << 8)` — the three bytes are the **live MFP
  hardware registers** read before any write; **`scr` byte not included**
  (`bios/serport.c:229`). Measured on TT (power-up after `init_serport`'s
  `Rsconf(B9600,0,0x88,1,1,0)`, `:1066`): UCR `0x88`, RSR `0x01`, TSR `0x81`
  -> `0x88018100` (`tosdump/cases/xbios-chardev-r1.tsv:31@Rsconf`, probe S02).
  *Inferred, not in EmuTOS source:* TSR bit 7 is the MFP 68901's "buffer
  empty" flag, set by the chip, not by the `tsr=1` write; EmuTOS itself reads
  it as "transmitter empty" in `bconout1`/`bconoutTT` (`:314`, `:433`) and,
  only in `RS232_DEBUG_PRINT` builds, in `bcostat1` (`:277`). In the normal
  build (`RS232_DEBUG_PRINT` 0, `include/config.h:1963-1968`) `bcostat1`
  answers from the output ring (`:279-282`), not from TSR.
- **Encoding (SCC handler: Falcon port B, device 7; Falcon/TT port A):**
  `(iorec->ucr << 24)` plus `0x0800` if the shadow `wr5` bit 4 (break) is set
  (`bios/serport.c:788-790`); RSR and SCR bytes are **0**, the only TSR bit is
  bit 3 (comment cites the "TT030 TOS Release notes"; code is the authority).
  The UCR byte is the *remembered* last-written value (`iorec->ucr`,
  power-up `0x88`, `bios/serport.c:89`), not hardware. Measured on Falcon:
  `0x88000000`. TSR bit 7 is never set.
- **`rsconf_dummy`:** `0` (`bios/serport.c:947`).
- **`baud == -2` query** (all handlers, tested first): returns
  `iorec->baudrate` (`bios/serport.c:222-223`, `:779-780`) — a baud code, not
  packed.
- **Extension:** 32-bit `ULONG`; on AArch64 sign-extended into `x0`
  (see "Native ABI" above).
- **Source location:** `bios/serport.c:222-223,229` (MFP),
  `:779-790` (SCC), `:947` (dummy).

### Side effects on global state / hardware

- Programs the MFP USART registers for the parameters that are `>= 0`
  (SCC: WR12/13 for the baud, WR3/4/5 for `ucr`, WR5 break for `tsr`).
- **Writes the record `Iorec(0)` returns** (observable by a program that
  reads it): MFP handler `baudrate` (`:231-232`, `0 <= baud <= 15`) and
  `flowctrl` (`:237-238`, `0 <= ctrl <= 3`); SCC handler `flowctrl`
  (`:792-793`), `baudrate` (`:800`), and for `ucr >= 0` `ucr` (`:811`),
  `datamask` (`:830`), `wr5` (`:832`). Field offsets: see
  `docs/api/structures/iorec.md` (`extiorec`: `baudrate` m68k 28 / LP64 48,
  `flowctrl` 29 / 49, `ucr` 30 / 50, `datamask` 31 / 51).
- **NewTOS today:** the shadows `RS_*` are private statics
  (`tos-api/src/xbios/system.rs:235@RS_BAUD`..`:239@RS_TSR`); the record at
  `Iorec(0)` is **not** updated (it keeps the power-up line
  `tos-api/src/xbios/iorec.rs:154@AUX_LINE`) — a gap, XBIOS06 sub-point
  2026-10-29, probe design in the analysis.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`baud` outside 0..15 and `ctrl` outside 0..3 are skipped
(`bios/serport.c:231,237`); `ucr`/`rsr`/`tsr`/`scr` `>= 0` are written as is
(truncated to the register's byte). A `-1`-only call cannot fail.

---

## Verification criteria

- **Set baud:** `Rsconf(1, -1, -1, -1, -1, -1)` sets 9600 baud and leaves
  flow/registers unchanged; the return packs the previous UCR/RSR/TSR.
- **Query baud:** `Rsconf(-2, -1, -1, -1, -1, -1)` returns the current baud
  index and changes nothing.
- **Keep-all:** `Rsconf(-1, -1, -1, -1, -1, -1)` changes nothing.
- **Return excludes `scr`:** the low byte of the MFP return is 0 (the `tsr`
  is in bits 8-15, not `scr` in bits 0-7).

---

## Edge cases discovered in source

1. **`speed == -2` is the query.** It returns the current baud code without
   changing state (`bios/serport.c:222-223`) — distinct from `-1` ("keep",
   which returns the packed old registers). The old spec said `baud == -1` is
   "query" — corrected (D-041).
2. **The MFP return omits `scr`.** Only UCR/RSR/TSR are packed
   (`(ucr<<24)|(rsr<<16)|(tsr<<8)`, `:229`); the old spec claimed
   `… | scr` in the low byte. The SCC handler packs yet another layout
   (D-041).
3. **Per-register `>= 0` gate.** Each of `ucr`/`rsr`/`tsr`/`scr` is applied
   only if non-negative (`:239-246`), so you can set just the baud without
   disturbing the framing registers.

---

4. **The `Rsconf` answer depends on the port `Bconmap` maps to device 1.**
   Same call, same ROM: TT `0x88018100` (MFP), Falcon `0x88000000` (SCC B)
   (`bios/serport.c:966,972,975-979`). The bytes are device parameters, not
   an EmuTOS inconsistency (XBIOS06).
5. **The MFP return carries hardware state, the SCC return remembered state.**
   MFP: live `ucr`/`rsr`/`tsr` registers (`:229`); SCC: `iorec->ucr` shadow,
   RSR/SCR 0, TSR only bit 3 (`:788-790`).
6. **`ctrl` is validated against 0..3 and `baud` against 0..15** in both
   handlers (`:231,237`; `:792,797`); out-of-range values are skipped, not
   passed to hardware. (The earlier sentence "Out-of-range values are passed
   to hardware without validation" applies to `ucr`/`rsr`/`tsr`/`scr` only.)

## Dependencies

### Requires (must be initialized / functional)

- `rsconfptr` installed (default `rsconf1`); the serial hardware.

### Used by

- Terminal programs, modem dialers, MIDI-over-serial adaptors, `Bconmap`
  reconfig.

---

## Related data structures

- `docs/api/xbios/_state.md §3` — `rsconfptr`, `EXT_IOREC`.
- `docs/api/xbios/2c-bconmap.md` — retargets `rsconfptr`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the baud
  table + flow modes; the `-2` query and the `scr`-less return are EmuTOS
  specifics.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `rsconf_mfp` needs `CONF_WITH_MFP_RS232 || CONF_WITH_TT_MFP`
  (`bios/serport.c:213`); `rsconf_scc` needs `CONF_WITH_SCC`; `rsconf1` returns
  0 without `CONF_WITH_MFP_RS232` (`:391`). `BCONMAP_AVAILABLE` =
  `CONF_WITH_SCC || CONF_WITH_TT_MFP` (`bios/serport.h:18`). The reference
  build (make 512, `docs/api/_emutos-config.md`) compiles all three; the
  handler chosen at run time is by machine (table under "Installed handler").

---

## NewTOS implementation notes

**XBIOS06 status (2026-10-30), facts only:** NewTOS keeps private shadows
(`tos-api/src/xbios/system.rs:235@RS_BAUD`, `:237@RS_UCR`, `:238@RS_RSR`,
`:239@RS_TSR`), does not touch the PL011 in `rsconf`
(`tos-api/src/xbios/system.rs:254@rsconf`), does not write the `Iorec(0)`
record, returns zero-extended, and starts with the register image
`0x88010100` (TSR `0x01`). The register-byte profile (TT-like `0x81`,
Falcon-like `0x00`, or own) is an open owner decision; see
`docs/analysis/xbios06-rsconf-profile-2026-10-30.md`.

PHASE-07 (historic intent, not a measured statement about today's code): NewTOS implements `Rsconf` against the PL011 UART — the baud index
maps to IBRD/FBRD divisors; flow bits map to CR.RTSEn/CTSEn. The MFP register
arguments are ignored on input, and the ULONG return synthesises plausible
UCR/RSR/TSR bytes; the `-2` query returns the current baud index.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the **MFP 68901 USART** to the
> **BCM2711 PL011 UART0**. The contract above (trap ID/ABI, the `-1` keep /
> `-2` query convention, the 16-entry baud table, the flow-control constants,
> the packed `(ucr<<24)|(rsr<<16)|(tsr<<8)` return) is **unchanged**; only the
> silicon it programs changes. See `docs/HARDWARE-PROFILE.md` §2, §3.2.

**Peripheral / driver.** AUX / serial = **PL011 UART0** (`kernel/src/drivers/bcm2711/pl011.rs`;
`virt:` `kernel/src/drivers/qemu_virt/pl011.rs`) via the `hal/uart.rs::Uart` trait. This UART is a
**debug channel only** (`kprintln!` + bring-up shell); the TOS console (CON,
handle 2) does **not** use it (`HARDWARE-PROFILE.md` §3.2). Datasheet: BCM2711
ARM Peripherals **§11 "UART"** (the PL011), p.144; **§11.5 "Register View"**,
p.146. UART0 physical base `0x7E20_1000` (mapped `0xFE20_1000`,
`kernel/src/drivers/bcm2711/pl011.rs:29`).

### Concrete Pi values (as configured today)

| Item | Value | Source |
|------|-------|--------|
| UART reference clock `FUARTCLK` | **48 MHz** (firmware `init_uart_clock=48000000`) | `kernel/src/drivers/bcm2711/pl011.rs:14-18` |
| Default baud | **115200** | `kernel/src/drivers/bcm2711/pl011.rs:14`, `hal/uart.rs:11` |
| `IBRD` (0x24) | **26** = `trunc(48e6 / (16·115200))` | `kernel/src/drivers/bcm2711/pl011.rs:136`; DS §11.5 IBRD p.150 |
| `FBRD` (0x28) | **3** = `round((48e6/(16·115200) − 26)·64)` | `kernel/src/drivers/bcm2711/pl011.rs:137`; DS §11.5 FBRD p.150 |
| Line setting | **8N1** (LCRH `WLEN_8` + `FEN`; no `PEN`, no `STP2`) | `kernel/src/drivers/bcm2711/pl011.rs:140`; DS §11.5 LCRH p.150-151 |
| FIFO | 32×8 TX / 32×12 RX, `FEN=1` | DS §11.1 p.144; `kernel/src/drivers/bcm2711/pl011.rs:56,140` |
| Flow control | **none** — `CR = UARTEN\|TXE\|RXE`; `CTSEN`/`RTSEN` **not set** | `kernel/src/drivers/bcm2711/pl011.rs:143`; DS §11.5 CR p.151-152 |
| `virt:` baud | same 115200, but `FUARTCLK=24 MHz` → IBRD=13, FBRD=1 | `kernel/src/drivers/qemu_virt/pl011.rs:11-12,71-72` |

**Baud formula (datasheet §11.5, p.150):**
`BAUDDIV = FUARTCLK / (16 · baud)`; the 6-bit `FBRD` is
`round(frac(BAUDDIV)·64)`. So each Atari baud index (0..15) that NewTOS chooses
to support maps to an `(IBRD, FBRD)` pair for `FUARTCLK = 48 MHz`, e.g.:

| Idx | Baud | IBRD | FBRD | Idx | Baud | IBRD | FBRD |
|----:|-----:|-----:|-----:|----:|-----:|-----:|-----:|
| 0 | 19200 | 156 | 16 | 7 | 1200 | 2500 | 0 |
| 1 |  9600 | 312 | 32 | 9 |  300 | 10000 | 0 |
| — | 115200*| 26 |  3 | 15 |   50 | 60000 | 0 |

*115200 is the PL011 default; it has **no Atari baud index** (the MFP table tops
out at 19200 = idx 0). A NewTOS `Rsconf` must therefore decide the index→baud
policy — see decision D-SER-1.

### Atari MFP → PL011 mapping (what maps, what is synthesized/ignored)

| Rsconf parameter | Atari MFP mechanism | PL011 equivalent | NewTOS today |
|------------------|---------------------|------------------|--------------|
| `speed` (baud idx 0..15) | MFP timer-D divisor | `IBRD`/`FBRD` (DS §11.5 p.150) | wired ([XBIOS07], see R1 below); whether it programs IBRD/FBRD is not verified here |
| `speed == -2` (query) | read `rs232iorec.baudrate` | return the driver's stored baud index | **synthesized** — PL011 has no "baud index" register; the code must keep a shadow (no HW read-back) |
| `ctrl` (flow 0..3) | XON/XOFF sw + MFP GPIP RTS/CTS | sw XON/XOFF in the driver; `CR.RTSEN`/`CR.CTSEN` bits 14/15 (DS §11.5 p.151-152) for RTS/CTS | RTS/CTS pins (GPIO16/17 ALT3, DS §11.3 p.145) are **not muxed** today (only TX/RX GPIO14/15 ALT0, `kernel/src/drivers/bcm2711/pl011.rs:98-103`) → HARD flow cannot actually assert; **map only NONE/SOFT truthfully** until pins are wired |
| `ucr` (USART control: word len, parity, stop, clock ÷16) | MFP UCR | `LCRH.WLEN`/`PEN`/`EPS`/`STP2`/`FEN` (DS §11.5 p.150-151) | **ignored on input**; the framing bits *could* map to LCRH but the MFP UCR bit layout ≠ LCRH, so a translation table is needed — not done |
| `rsr` (receiver status) | MFP RSR | no single PL011 equivalent (status is in `FR`/`RSRECR`) | **ignored on input; synthesized in the return** |
| `tsr` (transmitter status) | MFP TSR | no single PL011 equivalent (`FR.TXFE`/`FR.BUSY`) | **ignored on input; synthesized in the return** |
| `scr` (sync char) | MFP SCR | none (PL011 is async-only) | **ignored** (and, per contract, excluded from the return) |
| ULONG return `(ucr<<24)\|(rsr<<16)\|(tsr<<8)` | old MFP register bytes | no register model to read back | **fully synthesized** — the driver must fabricate plausible old-UCR/RSR/TSR bytes |

**Bottom line.** The PL011 has **no register-for-register MFP model**: `baud`
maps cleanly to `IBRD/FBRD`; the *framing* fields (`ucr` word-length/parity/
stop) *could* map to `LCRH` with a translation table but currently do **not**;
`rsr`/`tsr`/`scr` and the packed status **return** have no HW source and are
**synthesized or ignored**. The MFP GPIP/interrupt semantics of the status
registers do not exist on the PL011.

### Current-code gaps → Step-2

- **R1 (superseded, [XBIOS07]):** earlier text said `Rsconf` was not
  dispatched and returned `EINVFN`. It is wired
  (`tos-api/src/xbios/mod.rs:65-138@dispatch`, `system::rsconf`): baud
  set/query (`-2`), `-1` keep, out-of-range handling and UCR/RSR set/readback
  agree with the oracle (audit probe S01-S18); only the TSR byte (`0x81` TT
  vs `0x01` NewTOS shadow, `tos-api/src/xbios/system.rs:239@RS_TSR`) and the
  upper 32 bits differ (XBIOS-CHARDEV-006, **pending XBIOS06**). The
  "synthesizes plausible bytes" statements in the table above describe the
  intent; the measured result is as stated here.
- **R2 (GAP):** even when wired, the `ucr` framing bits are planned to be
  **ignored on input** rather than translated to `LCRH` — silently does nothing
  for word-length/parity/stop changes.
- **R5 (NOTE):** HARD/BOTH flow (`ctrl` 2/3) cannot be honoured until GPIO16/17
  are muxed to CTS0/RTS0 (ALT3); today only NONE/SOFT are physically possible.
