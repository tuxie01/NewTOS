# `Rsconf` (XBIOS 0x0F)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/xbios.c:376` (`rsconf` wrapper) and the default
> handler `bios/serport.c:217` (`rsconf_mfp`, installed as `rsconf1`).

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

**Installed handler / vector:** `rsconfptr` (`bios/serport.c:74`), default
`rsconf1 = rsconf_mfp` (`bios/serport.c:217-249`, set `:1018`); `Bconmap`
can retarget it to a SCC/TT-MFP handler.

**Helpers called:**
- `bios/serport.c:217` — `rsconf_mfp()` — the MFP USART implementation.

**Headers / type definitions used:**
- `bios/serport.h:23-38` — the 16 baud constants (`B19200`..`B50`).
- `bios/serport.h:46-52` — flow-control constants (`FLOW_CTRL_NONE`..`_BOTH`).
- `bios/serport.h:57-65` — `EXT_IOREC` (`.baudrate` read by the query form).

**System variables / globals read/written:**
- `rs232iorecptr->baudrate` (read on the query path), the MFP UART registers.

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
- **Native (AArch64) ABI:** ID `0x0F` in `x8`; args in `x0..x5`; the ULONG
  result **zero-extended** into `x0` (packed register bytes; treat as
  unsigned 32-bit).

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
| arg1     | `ctrl`  | `WORD` | 2    | `bios/serport.c:...` | flow mode 0..3; `-1` keep |
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
- **Encoding (MFP handler):** `(old_ucr << 24) | (old_rsr << 16) |
  (old_tsr << 8)` — **`scr` byte not included** (`bios/serport.c:229`). On the
  **`baud == -2` query** it returns the current baud code
  (`rs232iorecptr->baudrate`) instead (`bios/serport.c:222-223`). The SCC
  handler packs a different subset.
- **Source location:** `bios/serport.c:222-223,229`.

### Side effects on global state / hardware

- Programs the MFP USART registers for the parameters that are `>= 0`; updates
  `rs232iorecptr->baudrate`/`flowctrl` shadows.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

Out-of-range values are passed to hardware without validation; a `-1`-only
call cannot fail.

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

- `CONF_WITH_MFP` handler by default; `CONF_WITH_SCC`/`CONF_WITH_TT_MFP` swap
  a different handler with a different return packing.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: NewTOS implements `Rsconf` against the PL011 UART — the baud index
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
| `speed` (baud idx 0..15) | MFP timer-D divisor | `IBRD`/`FBRD` (DS §11.5 p.150) | **not wired** (dispatcher gap R1); designed to program IBRD/FBRD |
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

- **R1 (GAP):** `Rsconf (0x0F)` is **not in the XBIOS dispatcher**
  (`tos-api/src/xbios/mod.rs:33-55`) → returns `EINVFN`, so *no* baud change,
  query, or flow config works today. The "PHASE-07" note above describes the
  intended behaviour, not the wired one.
- **R2 (GAP):** even when wired, the `ucr` framing bits are planned to be
  **ignored on input** rather than translated to `LCRH` — silently does nothing
  for word-length/parity/stop changes.
- **R5 (NOTE):** HARD/BOTH flow (`ctrl` 2/3) cannot be honoured until GPIO16/17
  are muxed to CTS0/RTS0 (ALT3); today only NONE/SOFT are physically possible.
