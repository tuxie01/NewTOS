# BCM2711 EMMC2 (microSD) — hardware reference

> Track-B reference per `docs/SOURCE-ANALYSIS-PROTOCOL.md`. Produced
> **before** the driver (Rule 6). Target: the **on-board microSD card
> slot** of the Raspberry Pi 4 — the card that holds the boot files —
> which is wired to the **EMMC2** controller. First test vehicle: QEMU
> `raspi4b` (emulates the SDHCI controller; debuggable).
>
> **Writing or reviewing a wait loop? Read §7 first** — every timing
> limit the card and the controller are allowed to take, with its section
> number. §8 maps the section numbers across specification editions;
> `references/README.md` pins the editions themselves.

## Which controller (important)

The RPi4 has **three** SD/SDIO controllers plus USB; we target exactly
one. Names matter here, because two of the three are routinely confused:

| Controller | Phys base | Device-tree node | State in the shipped DT | Drives |
|------------|-----------|------------------|-------------------------|--------|
| **EMMC2** (Arasan/iproc SDHCI) | **`0xFE34_0000`** | `mmc@7e340000`, `compatible = "brcm,bcm2711-emmc2"`, alias **`mmc0`** | `status = "okay"` | **the microSD card slot** (boot card) ← us |
| EMMC / "sdhci" (the BCM2835-style Arasan block) | `0xFE30_0000` | `mmcnr@7e300000`, `compatible = "brcm,bcm2835-sdhci"`, alias `mmc1` | `status = "okay"`, `non-removable` | the **on-board Wi-Fi** — the node carries the child `wifi@1`, `compatible = "brcm,bcm4329-fmac"`, on `sdio_pins` = GPIO 34–39, ALT3 |
| SDHOST (Broadcom's own controller) | `0xFE20_2000` | `mmc@7e202000`, `compatible = "brcm,bcm2835-sdhost"`, alias `sdhost` | `status = "disabled"` | nothing, in the default Pi 4 configuration |
| xHCI (USB) | — | — | — | external USB storage (later, loadable driver) |

**Correction (REF02, 2026-09-28).** Earlier editions of this note called
the block at `0xFE30_0000` "sdhost (legacy BCM2835 EMMC)". That is two
peripherals conflated: **SDHOST is a different controller at
`0xFE20_2000`**, and on the Pi 4 it is disabled. Broadcom's own
documentation keeps them apart — BCM2711 ARM Peripherals §6.2 Table 102
lists *IRQ 56 SDHOST* and *IRQ 62 "EMMC & EMMC2"* as separate sources —
and so does circle (`include/circle/bcm2835.h:256`
`ARM_SDHOST_BASE = ARM_IO_BASE + 0x202000` versus `:248`
`ARM_EMMC_BASE = ARM_IO_BASE + 0x300000`). The *substance* of the old
claim survives the correction: the controller at `0xFE30_0000` is indeed
the one the Wi-Fi hangs off — but it is the EMMC, not SDHOST, and the
claim now has a source.

Two device-tree nodes describe the same registers at `0x7e30_0000`:
`mmc@7e300000` (label `sdhci`, `status = "disabled"`) and
`mmcnr@7e300000` (label `mmcnr`, `status = "okay"`, `non-removable`,
child `wifi@1`). The active one is the second. Note what this evidence is
and is not: a device tree is the **firmware's configuration** of the
board, which overlays can change — it is not a statement about
unchangeable silicon wiring. For us that is enough, because it is the
configuration our Pi actually boots with, and because the driver probes
both bases anyway (§6).

**Where the evidence comes from, and what it is worth.** The BCM2711
datasheet documents **no** EMMC block at all: the only occurrence of the
name in the whole document is the interrupt table above (audit §0). The
base addresses themselves are therefore *not* in the datasheet; what the
datasheet does give is the mapping rule, §1.2.4: "a peripheral described
in this document as being at legacy address `0x7Enn_nnnn` … [is] visible
to the ARM at `0x0_FEnn_nnnn` if Low Peripheral mode is enabled". The
board-level assignment comes from the device tree the **Raspberry Pi
firmware itself ships** (`bcm2711-rpi-4-b.dtb`, our boot card's copy,
sha256 `75761b73…`, see `references/README.md`), which also states the
mapping for this bus explicitly: the `emmc2bus` node has
`ranges = <0x0 0x7e000000 0x0 0xfe000000 0x1800000>`. circle
(`include/circle/bcm2711.h:31`, `ARM_EMMC2_BASE = ARM_IO_BASE + 0x340000`)
agrees, as a tertiary cross-check.

The VideoCore firmware uses this card to load `start4.elf`/`kernel8.img`;
once our kernel runs the firmware is done, so we **re-initialise** the
controller and take it over (same physical card).

EMMC2 needs no GPIO setup: its device-tree node has no `pinctrl`
property at all, whereas the Wi-Fi controller at `0xFE30_0000` is
explicitly routed to `sdio_pins` (GPIO 34–39, ALT3). The card slot's
lines are dedicated.

## Sources

| # | Source | Used for |
|---|--------|----------|
| 1 | **SD Physical Layer Simplified Specification Ver. 9.10** (2023-12-01) | card protocol: init sequence, card status, **timing limits** (§7) |
| 2 | **SD Host Controller Simplified Specification Ver. 3.00** (2011-02-25) | controller register/bit semantics and sequences (EMMC2 = standard SDHCI, and reports HC version 3.00) |
| 3 | BCM2835 ARM Peripherals datasheet §5 "External Mass Media Controller" | the 32-bit register *grouping* the BCM puts over the SDHCI registers |
| 4 | BCM2711 ARM Peripherals datasheet §1.2.4 (legacy→ARM address rule), §6.2 Table 102 (IRQs) | the `0x7E…`→`0xFE…` mapping; the fact that EMMC has **no** chapter here |
| 5 | Raspberry Pi firmware device tree `bcm2711-rpi-4-b.dtb` | which controller drives which port, and on which pins |
| 6 | circle (`rsta2/circle` @ `6177984e`, `addon/SDCard`) | tertiary cross-reference: a verified-working RPi4 EMMC2 driver |

Sources 1 and 2 are **pinned by checksum** in `references/README.md`
("SD specifications"), together with the licence terms — the PDFs are
free to fetch but must not be committed or re-hosted, so the pin table is
the citation anchor. Quote them sparingly and always **by section
number, never by page**; §8 below maps the section numbers across
editions.

Exact register bits are validated empirically against QEMU `raspi4b`
during bring-up (QEMU implements the SDHCI standard).

---

## 1. Register map (32-bit accesses, offsets from `0xFE340000`)

The BCM groups the SDHCI registers into 32-bit words (datasheet §5).

| Off | Name | Contents |
|-----|------|----------|
| `0x00` | `ARG2` | argument 2 (ACMD23) |
| `0x04` | `BLKSIZECNT` | `[15:0]` block size, `[31:16]` block count |
| `0x08` | `ARG1` | command argument |
| `0x0C` | `CMDTM` | `[31:16]` command, `[15:0]` transfer mode |
| `0x10`..`0x1C` | `RESP0..3` | command response |
| `0x20` | `DATA` | buffer data port (PIO) |
| `0x24` | `STATUS` | present state (see below) |
| `0x28` | `CONTROL0` | host control 1 + power + block-gap |
| `0x2C` | `CONTROL1` | clock control + timeout + software reset |
| `0x30` | `INTERRUPT` | normal + error interrupt status |
| `0x34` | `IRPT_MASK` | interrupt status enable |
| `0x38` | `IRPT_EN` | interrupt signal enable |
| `0x3C` | `CONTROL2` | auto-CMD-err + host control 2 |
| `0x40` | `CAPABILITIES` | controller capabilities |
| `0xFC` | `SLOTISR_VER` | slot ISR + host-controller version |

### `STATUS` (present state) bits

```
0   CMD_INHIBIT     CMD line busy (cannot issue a command)
1   DAT_INHIBIT     DAT line busy
2   DAT_ACTIVE      data transfer active
8   WRITE_TRANSFER
9   READ_TRANSFER
10  BUFF_WRITE_EN   buffer ready for a PIO write
11  BUFF_READ_EN    buffer holds PIO read data
16  CARD_INSERTED
```

### `CONTROL1` (clock + reset) bits

```
0       CLK_INTLEN    internal clock enable
1       CLK_STABLE    internal clock stable (read)
2       CLK_EN        SD clock enable
6:7     CLK_FREQ_MS2  upper 2 bits of the 10-bit divider
8:15    CLK_FREQ8     lower 8 bits of the divider (SDCLK = base / (2*div))
16:19   DATA_TOUNIT   data timeout exponent
24      SRST_HC       reset the whole host controller
25      SRST_CMD      reset the command circuit
26      SRST_DATA     reset the data circuit
```

### `INTERRUPT` bits

```
0   CMD_DONE     command complete
1   DATA_DONE    transfer complete
4   WRITE_RDY    buffer write ready
5   READ_RDY     buffer read ready
15  ERR          an error bit (16..) is set
16+ CTO/CCRC/... command/data timeout & CRC errors
```

### `CMDTM` — command word (`[31:16]`)

```
29:24 CMD_INDEX      the SD command number
21    CMD_ISDATA     command involves a data transfer
17:16 CMD_RSPNS_TYPE 0=none, 1=136-bit, 2=48-bit, 3=48-bit-busy
19    CMD_CRCCHK_EN  check the response CRC
20    CMD_IXCHK_EN   check the response index
```

### `CMDTM` — transfer mode (`[15:0]`)

```
1   TM_BLKCNT_EN   block-count enable
3:2 TM_AUTO_CMD_EN 00 = none, 01 = Auto CMD12, 10 = Auto CMD23 (SD-HC 3.00 §2.2.5)
4   TM_DAT_DIR     1 = card → host (read)
5   TM_MULTI_BLOCK multi-block transfer
```

---

## 2. Controller init

1. **Reset:** set `CONTROL1.SRST_HC`; poll until it clears.
2. **Clock — identification (~400 kHz):** set `CONTROL1` =
   `CLK_INTLEN | (divider) | (DATA_TOUNIT=0xE)`, wait `CLK_STABLE`, then
   set `CLK_EN`. The base clock is the EMMC2 clock (≈ 100 MHz on RPi4 /
   reported via the mailbox `get clock rate` tag, clock id 1 = EMMC).
3. **Enable interrupt status** (`IRPT_MASK = 0xFFFFFFFF`) but **poll**
   `INTERRUPT` rather than taking IRQs in the bring-up driver.
4. **Wait out the card's preparation clocks before CMD0:** at least
   **74 SD clocks** with the CMD line high (SD Phys 9.10 §6.4.1.1 and
   §6.4.1.4 (5); see §7.4). At 400 kHz that is 185 µs. *The driver does
   not do this today* — audit finding 6, open.

After card identification, switch the clock to ~25 MHz (default speed).

> EMMC2 base clock: query the mailbox property `get clock rate`
> (tag `0x00030002`, clock id `1` = EMMC) — reuse the mailbox transport
> from `docs/hardware/mailbox-framebuffer.md`. QEMU returns a usable
> rate; on HW it is the real EMMC clock.

---

## 3. SD card init sequence (SD Phys. Layer spec)

Issue a command by: wait `!STATUS.CMD_INHIBIT`, write `ARG1`, write
`CMDTM`, wait `INTERRUPT.CMD_DONE` (check error bits), read `RESP0..3`.

| Cmd | Index | Arg | Resp | Purpose |
|-----|-------|-----|------|---------|
| GO_IDLE | CMD0 | 0 | none | reset card to idle |
| SEND_IF_COND | CMD8 | `0x1AA` | R7 | voltage check (echo `0xAA`); R7=`0x1AA` ⇒ SD v2 |
| APP_CMD | CMD55 | `RCA<<16` | R1 | prefix for an ACMD |
| SD_SEND_OP_COND | ACMD41 | `0x40FF_8000` | R3 | `HCS`(bit30) + **voltage window** (bits 23:15); loop until OCR bit31=ready; OCR bit30=`CCS` ⇒ SDHC. See the correction below |
| ALL_SEND_CID | CMD2 | 0 | R2 | card identification |
| SEND_REL_ADDR | CMD3 | 0 | R6 | get the card's `RCA` |
| SELECT_CARD | CMD7 | `RCA<<16` | R1b | put card in transfer state |
| SET_BLOCKLEN | CMD16 | `512` | R1 | block length (SDSC only; SDHC fixed at 512) |

`CCS` (Card Capacity Status) decides addressing: **SDHC/SDXC use block
addressing** (arg = block number), **SDSC uses byte addressing** (arg =
block × 512).

### The ACMD41 argument — correction (REF02, 2026-09-28)

Earlier editions of this note gave the ACMD41 argument as
`0x5100_0000`. That value has **no voltage window**, and a zero voltage
window does not mean "don't care" — it means a different command.
SD Phys 9.10 **§4.2.3.1**, the general rules for the argument:

1. voltage window field (bits 23:0) = 0 → this is the **"inquiry
   ACMD41"**; it *does not start initialization*, it only returns the
   OCR, and it explicitly **ignores** the rest of the argument
   (bits 31:24) — so the `HCS` bit in `0x5100_0000` would have had no
   effect either.
2. voltage window ≠ 0 on the first ACMD41 → this is the **"first
   ACMD41"** that starts initialization; now bits 31:24 are effective.
3. every following ACMD41 must repeat the *same* argument.

A host that loops on `0x5100_0000` waiting for the busy bit therefore
loops forever: the card never leaves the power-up procedure, because it
was never asked to start it. The documented value was wrong, not merely
incomplete; **the driver has always done it right**
(`kernel/src/drivers/bcm2711/emmc.rs`: `hcs | 0x00FF_8000`).

Argument layout, SD Phys 9.10 Figure 4-3 with the OCR field per Table 5-1
(§5.1):

| Argument bits | Field | Ours |
|---|---|---|
| 31 | reserved (0) | 0 |
| 30 | `HCS` — Host Capacity Support (1 = SDHC/SDXC supported) | 1 after a successful CMD8, else 0 |
| 29 | reserved for eSD Fast Boot | 0 |
| 28 | `XPC` — SDXC power control (1 = max performance) | 0 |
| 27:25 | reserved | 0 |
| 24 | `S18R` — request switch to 1.8 V signalling (UHS-I) | 0, we stay at 3.3 V |
| 23:15 | VDD voltage window, one bit per 100 mV step from 2.7–2.8 V (bit 15) to 3.5–3.6 V (bit 23) | **all nine set → `0x00FF_8000`** |
| 14:0 | reserved | 0 |

So `0x40FF_8000` for a v2 card (CMD8 answered) and `0x00FF_8000` for a
v1 card. The same OCR layout comes back in the R3 response, where
**bit 31** is the power-up/busy bit (0 while the card is still busy) and
**bit 30** is `CCS`.

---

## 4. Single-block read (CMD17)

1. `BLKSIZECNT = (1 << 16) | 512` (1 block, 512 bytes).
2. `ARG1 = block_addr` (block number for SDHC).
3. `CMDTM = (17 << 24) | RSPNS_48 | CRCCHK | IXCHK | ISDATA | TM_DAT_DIR`.
4. Wait `INTERRUPT.CMD_DONE`; check errors.
5. Wait `INTERRUPT.READ_RDY`; read 128 × 32-bit words from `DATA` into
   the buffer.
6. Wait `INTERRUPT.DATA_DONE`.

### 4a. Multi-block read/write (CMD18/CMD25) — FS08, 2026-10-07

SD-HC 3.00 §3.7.2.1 ("Not using DMA"), with Auto CMD12 (§1.11, §2.2.5):

1. `BLKSIZECNT = (n << 16) | 512` — Block Count is 16 bits, 1..=65535
   (§2.2.3, Table 2-5); a longer request is split into such chunks.
2. `ARG1` = first block address, as for CMD17.
3. `CMDTM = (18 << 24) | RSPNS_48 | CRCCHK | IXCHK | ISDATA | TM_DAT_DIR |
   TM_MULTI_BLOCK | TM_BLKCNT_EN | TM_AUTO_CMD12` (`0x…36` low half).
   CMD25: the same without `TM_DAT_DIR`.
4. Wait `CMD_DONE`; check the R1 in `RESP0` (refused ⇒ no data phase).
5. Per block: wait `READ_RDY` (`WRITE_RDY`), clear it, move 128 words.
6. Wait `DATA_DONE` — for a multiple-block command it also covers the Auto
   CMD12 (§3.7.1.2 Note 2).
7. The Auto CMD12's R1b is in `RESP3` = REP[127:96] (§2.2.7, Table 2-12);
   the driver checks it like any R1.

`OUT_OF_RANGE` at the end of the card: SD Phys 9.10 §4.3.3 "When the last
block of user area is read using CMD18, the host should ignore
OUT_OF_RANGE"; SD-HC 3.00 §3.7.1.2 Note 3 places it in the Auto CMD12
response or the next memory command's. The driver masks that bit in the
Auto CMD12 response of a CMD18 that ended on the last block, and once in
the next data command's R1 if that command stays inside the card.

Errors: `INTERRUPT` bit 24 is `Auto CMD Error` (Error Interrupt Status
bit 8, §2.2.18); details in `CONTROL2[15:0]` = Auto CMD Error Status
(§2.2.23; bit 0 `Auto CMD12 Not Executed`). Recovery §3.10.2: if the Auto
CMD12 was executed, CMD-line reset first, then CMD12, then DAT-line reset;
otherwise CMD12 and the resets as in §3.8. Data-phase errors without Auto
CMD error take the existing §3.8/§3.10.1 path.

Boot argument `emmc=compat` (`kernel/src/bootargs.rs`) keeps one
CMD17/CMD24 per sector and the 1-bit bus.

### 4b. 4-bit bus (ACMD51 + ACMD6) — FS08, 2026-10-07

After `CMD7` (transfer state) and the switch to ~25 MHz:

1. `ACMD51 SEND_SCR` (adtc, R1, SD Phys 9.10 §4.7.4): one 8-byte block,
   `BLKSIZECNT = (1 << 16) | 8`. The SCR is "wide width data … shifted
   from the MSB bit" (§3.6.1), the data port is little endian (SD-HC 3.00
   §1.7 (2)): byte 0 = SCR[63:56], byte 1 = SCR[55:48];
   `SD_BUS_WIDTHS` = SCR[51:48] = low nibble of byte 1, bit 2 = 4-bit
   (§5.6, Table 5-21).
2. SD-HC 3.00 §3.4, SD-memory-only path: (1) `IRPT_MASK` bit 8 (Card
   Interrupt Status Enable) off, (4) `ACMD6 SET_BUS_WIDTH` arg `0b10`
   (§4.7.4: "'00'=1bit or '10'=4 bits bus"), (5) Host Control 1 bit 1
   `Data Transfer Width` = 1 (§2.2.10; `CONTROL0` bit 1), (8) bit 8 back on.
3. Any failure (SCR read, ACMD6 R1) leaves card and host on the 1-bit bus
   (DAT1-3 "start to operate as DAT lines after SET_BUS_WIDTH", §3.7.1
   Table 3-1 note 2) and is shown in the boot line `[emmc] FS08 mode: …`.
   circle notes that SEND_SCR "may fail with a DATA_TIMEOUT on the
   Raspberry Pi 4" (`addon/SDCard/emmc.cpp:2367`, tertiary).

**How long may each of those waits take?** §7. Steps 5 and 6 are the ones
with a card-side limit (100 ms minimum for a read, 250 ms of programming
busy for a write, and the host is told to allow more than 500 ms); steps
1–4 have none that the Simplified Specification states (§7.5).

---

## 5. Card status (R1) — the response the host must check

Every command answered with an **R1** response carries a 32-bit *card
status* field in `RESP0`. Waiting for `CMD_DONE` and finding no
`INTERRUPT` error bit only says that the **controller** put the command on
the bus and got a well-formed answer back; whether the **card** accepted
the command is in that response and nowhere else:

> "The software is responsible for checking the status bits of the card's
> response in order to verify successful processing by the card."
> — BCM2835 ARM Peripherals §5 (External Mass Media Controller), p. 65

> §3.7.1.2, the Command Complete Sequence, step (7): "Check for errors in
> Response Data. If there is no error, go to step (8). If there is an
> error, go to step (9)." — and step (9) is: "Return Status of 'Response
> Contents Error'."
> — SD Host Controller Simplified Specification 3.00
> (checked against the PDF, REF02; identical wording and section number in
> Ver. 4.20. Earlier editions of this note ran the two steps together into
> one sentence.)

> "The response format R1 shall contain a 32-bit field named card status
> as a response to CMD13 with argument b[15]='0'. This field is intended
> to transmit the card's status information (which may be stored in a
> local status register) to the host. If not specified otherwise, the
> status entries are always related to the previous issued command."
> — SD Physical Layer Simplified Specification Ver. 9.10 §4.10.1
> (word-identical in Ver. 6.00. The earlier ellipsis in this note dropped
> "as a response to CMD13 with argument b[15]='0'". That clause does
> **not** narrow the card status to CMD13: it distinguishes CMD13's two
> answers from each other — "CMD13 R1 indicates either Card Status or
> Task Status by selecting b[15] of CMD13 argument" (§4.10.1, same
> paragraph). Which *other* commands answer with which status bits is
> Table 4-43's job, and CMD17 and CMD24 are both in it.)

### The error bits — SD Phys 9.10 Table 4-42 "Card Status"

Table 4-42's *Type* column marks each entry `E` (error bit), `S` (status
bit), `R` (set for the actual command response) and/or `X` (set during
command execution). **`E` is the discriminator**: the bits below are the
complete set of Type-`E` entries. Everything else in the word
(`CURRENT_STATE`, `READY_FOR_DATA`, `APP_CMD`, `CARD_IS_LOCKED`,
`CARD_ECC_DISABLED`, `ERASE_RESET`, `FX_EVENT`) is status, not failure,
and must **not** be treated as an error.

What every one of the fifteen bits below does to our driver, before any
per-bit detail, is the same: `check_r1()` (`emmc.rs:714`) treats the
whole Type-`E` set as one mask (`R1_ERROR_MASK`) — any bit set turns the
call into `Err(BlockError::Card)`, which the BIOS adapter
(`host_env.rs::block_error_code`) reports as `EREADF` on a read /
`EWRITF` on a write, the same way EmuTOS' own `sd.c` does (§5 "Mapping a
card error" above). Each bit is additionally counted on its own
(`ErrorStats::card_bits`), but that count is diagnostic only — it never
changes which error code comes back. `check_r1()` is called after
the data commands `CMD17`/`CMD18` (read) and `CMD24`/`CMD25` (write), on
the Auto CMD12 response of a multiple-block transfer, and since FS08 on
`ACMD51`/`ACMD6` at init; no other command's R1 is examined this way. The table states, in our own words, what each bit
means and — where our code treats it differently from that shared
baseline — what that difference is; it does not restate the
specification's own wording, only the section it comes from.

| Bit | Identifier | For our driver |
|-----|------------|-----------------|
| 31 | `OUT_OF_RANGE` | The block address we sent is outside what the card will accept. One of only two bits the per-command cross-reference actually lists for `CMD17`/`CMD24` (see "Per-command applicability" below) — the realistic case, and the one that motivated `check_r1()` existing at all: before it did, a write past the end of the card came back `Ok(())` because nothing looked at `RESP0` (`write_one`, `emmc.rs:1082-1084`). SD Phys 9.10 §4.10.1 / Table 4-42. |
| 30 | `ADDRESS_ERROR` | The other bit Table 4-43 lists for `CMD17`/`CMD24`: the address didn't match the card's block length. `card_address()` (`emmc.rs:1031`) cannot itself produce a misaligned argument — SDHC is a plain block number, SDSC is `block × 512` — so this bit would only fire if the card disagreed with our own arithmetic. |
| 29 | `BLOCK_LEN_ERROR` | Wrong or mismatched transfer length. We always ask for exactly `SECTOR_SIZE` (512) via `BLKSIZECNT`, and fix an SDSC card to 512 once during init with `SET_BLOCKLEN` (`emmc.rs:1012`) — nothing in our own request should be able to trigger this. |
| 28 | `ERASE_SEQ_ERROR` | Erase-sequence error. We issue no erase command at all (no `CMD38`); the situation this bit reports on cannot arise from our driver. It is still part of the same mask — a card that set it anyway on a `CMD17`/`CMD24` response would be handled exactly like every other bit here. |
| 27 | `ERASE_PARAM` | Invalid erase block-range selection — same as `ERASE_SEQ_ERROR`: unreachable, because we never send an erase command. |
| 26 | `WP_VIOLATION` | A write hit a protected block. Reachable through `CMD24`, our only write command. It gets no code of its own: like EmuTOS' own SD driver (§5 "Mapping a card error" above), we fold it into the generic write-fault code `EWRITF`, not a write-protect-specific one. |
| 24 | `LOCK_UNLOCK_FAILED` | Password/sequence error in the card lock feature. We never issue the lock/unlock command (`CMD42`) — unreachable in practice; same generic handling if the card set it anyway. |
| 23 | `COM_CRC_ERROR` | The card's own verdict that the command it just received failed CRC. Worth keeping apart from `INT_ERR_CCRC` (`emmc.rs:78`): that one is the **controller's** verdict on the same kind of failure, checked and recovered separately in `wait_interrupt()`/`note_failure()`, before `check_r1()` is ever reached. Both end up as the same TOS code, but they are two different mechanisms in the driver. |
| 22 | `ILLEGAL_COMMAND` | Clear condition B, "Always related to the previous command" (Table 4-42): a command that is itself illegal is not answered at all (§4.6.1). In the R1 of a data command it therefore says nothing about this command, which **was accepted**. The one bit a data command's mask leaves out (`emmc_xfer.rs`, `data_command_mask`, HW08; the Pi took it for a refusal and left the card in `rcv`). Still counted. |
| 21 | `CARD_ECC_FAILED` | The card's internal ECC could not correct the data it read. Reachable on `CMD17`; the driver does not distinguish it from any other card refusal — no retry, no separate code. |
| 20 | `CC_ERROR` | Internal card-controller error. Generic, reachable on either command, no special handling. |
| 19 | `ERROR` (general) | The card's catch-all for a failure it has no more specific code for. Gets exactly the same treatment as every other bit in this table — a card that only ever sets this one is, from our driver's point of view, indistinguishable from any other rejection beyond its own counter. |
| 16 | `CSD_OVERWRITE` | Illegal attempt to change a read-only CSD field or reverse a permanent write-protect bit. We issue none of the programming commands (e.g. `CMD27`) this reports on — unreachable via `CMD17`/`CMD24`. |
| 15 | `WP_ERASE_SKIP` | Erase partially skipped write-protected blocks. Same as the other two erase-family bits above: unreachable, we never erase. |
| 3 | `AKE_SEQ_ERROR` | Authentication-sequence error. We run no authentication command sequence — unreachable. |

Bits 18, 17, 7, 4, 2, 1, 0 are reserved. As one mask:

```
R1_ERROR_MASK = 0xFDF9_8008
```

**Per-command applicability** — SD Phys 9.10 Table 4-43 ("Card Status
Field/Command — Cross Reference") says per command which of those bits may
appear in its response; the rows for `CMD17` (READ_SINGLE_BLOCK) and
`CMD24` (WRITE_BLOCK) are among the widest in the table and both mark
`OUT_OF_RANGE` and `ADDRESS_ERROR`. Masking the whole Type-`E` set is
therefore never wider than what those two commands may answer with, and
the reserved bits are specified as zero ("Unused reserved bits shall be
set to 0", §4.10.1).

**Which response carries it for a data command:** `CMD17`/`CMD24` answer
R1 **before** the data phase, so a rejected transfer (an address past the
end of the card, a write-protected card) is visible in `RESP0` at a point
where the data phase has not started. The host must therefore check R1 and
**abort the transaction** (SD-HC 3.00 §3.8: CMD12 + both software resets)
instead of waiting out `Buffer Read/Write Ready`, which will never come.

**`CMD12` only for a running transfer (HW08).** SD Phys 9.10 Table 4-35, row
`CMD12`: legal out of `data` and `rcv`, illegal in `tran` (below SDUC) and `prg`,
and an illegal command goes unanswered and sets `ILLEGAL_COMMAND` for the next
R1 (§4.6.1). A failed wait does not tell which state the card is in: a write
whose data went out is `prg`, a `dat_inhibit` timeout has no transfer at all. So
the recovery asks `CMD13` first, legal in all of them and allowed while the DAT
lines are busy (SD-HC 3.00 §2.2.9, Implementation Note), and sends `CMD12` only
to `data` or `rcv`; SD-HC 4.20 §3.8.1 has the same `CMD13` after the stop. A
state other than `tran` makes the next data command ask again and wait for
`tran`, within `WRITE_BUSY_MS`: `CMD17`/`CMD24` to a card in `prg` go unanswered
too. A data command that got no response takes both resets: its data side is
armed, and §3.10.1's CMD line reset alone leaves Command Inhibit (DAT) set (on the
Pi, `STATUS` `0x1fff0106`). Measured in
`measurements/measurement-2026-11-06-hw08-pi/RUNINFO.TXT`, fixed in
`measurements/measurement-2026-11-06-hw08-fix/RUNINFO.TXT`.

The two recovery sequences are **not** the same, and it is worth keeping
them apart (both SD-HC 3.00):

* **§3.8.1 Asynchronous Abort**, step (2): "Set **both** Software Reset
  For DAT Line and Software Reset For CMD Line to 1 in the Software Reset
  register"; step (3) then polls until both read 0.
* **§3.10.1 Error Interrupt Recovery**, steps (2)–(7): each reset is
  **conditional and separate** — check Error Interrupt Status bits 3:0
  (CMD line errors) → CMD reset → poll it to 0; then check bits 6:4 (DAT
  line errors) → DAT reset → poll it to 0. Only afterwards save and clear
  the error status.

Neither sequence prescribes how many *register writes* the reset bits
take; §3.8.1 asks for both bits to be set, §3.10.1 arrives at them one at
a time. That is a separate question from whether a given controller
decodes a two-bit write — QEMU's SDHCI model does not (HW03, finding N1).

> **One exception — handled since FS08 (§4a).** SD-HC 3.00 §3.7.1.2,
> Note 3: "When the last block of un-protected area is read using memory
> multiple block read command (CMD18), OUT_OF_RANGE error may occur even
> if the sequence is correct. The Host Driver should ignore it. This
> error will appear in the response of Auto CMD12 or in the response of
> the next memory command." (Identical in Ver. 4.20.) We only issue
> `CMD17`/`CMD24` today, so the rule is not yet reachable — but a
> `check_r1()` that rejects `OUT_OF_RANGE` unconditionally will refuse a
> legal read the moment `CMD18` is added. Whoever adds multi-block owns
> this.

### Mapping a card error to a TOS error code

The block driver is the BIOS layer's disk driver, so it returns a TOS
error code, exactly as EmuTOS' own SD driver does (`bios/sd.c:187-190`):

```c
if (ret > 0L) {         /* error but not timeout */
    ret = (rw&RW_RW) ? EWRITF : EREADF;
    break;
}
```

| Driver outcome | TOS code | EmuTOS source |
|----------------|----------|---------------|
| card reported an R1 error bit | `EWRITF` (-10) on write, `EREADF` (-11) on read | `bios/sd.c:187-190` |
| controller reported an `INTERRUPT` error bit (CRC, end-bit, …) | `EWRITF` / `EREADF` | same path — EmuTOS makes no distinction between "card said no" and "transfer broke" |
| a wait ran out of time | `EDRVNR` (-2) | `bios/sd.c:195` (`RW_NOMEDIACH` case; the media-change branch does not apply to a non-removable card) |
| no card / device not initialised | `EDRVNR` (-2) | `bios/sd.c:172-173` (`sd_check` failure) |
| request the device cannot serve | `EUNDEV` (-15) | `bios/blkdev.c:440-441` |

Note that EmuTOS' SD driver does **not** translate `WP_VIOLATION` into
`EWRPRO`; a write-protect violation is an `EWRITF` like any other card
error. (Its *floppy* driver reports `EWRPRO` from the FDC status, which is
a different device.)

> **Source gap (audit finding S1) — closed, REF02, 2026-09-28.** Both SD
> specifications now live in `references/sd-spec/` and are pinned by
> edition, date and SHA-256 in `references/README.md`. The open question
> from HW03 — bit positions read from Ver. 9.10 against an audit that
> cites Ver. 6.00 — was settled by diffing the two texts: **Table 4-42 is
> unchanged in bit assignment and Type column**; 9.10 only extends the
> prose of `WP_VIOLATION` and `WP_ERASE_SKIP` by "write protected until
> power cycle card", adds a note to `CURRENT_STATE`, and adds rows for
> new commands to Table 4-43. §4.10.1 is word-identical. The table above
> quotes 9.10, which is this project's reference edition; every 6.00
> citation elsewhere in the tree remains correct.

---

## 6. NewTOS integration

- **`BlockDevice` HAL trait** (`kernel/src/hal/block.rs`), mirroring the
  `Framebuffer` trait: `block_count()`, `block_size()` (= 512),
  `read_blocks(lba, buf)`, `write_blocks(lba, buf)`.
- **Driver** `kernel/src/drivers/bcm2711/emmc.rs` (board-rpi4): the
  SDHCI controller + SD protocol above. The QEMU `raspi4b` `emmc`
  device is the dev vehicle — attach an SD image with `-drive
  file=sd.img,if=sd,format=raw`.
- **DMA / cache:** the bring-up path uses **PIO** (read words from the
  `DATA` register), so there is no DMA-coherency concern like the
  framebuffer. A later DMA path would need the same PoC clean/invalidate
  discipline.
- **BIOS layer (later):** `Rwabs`/`Mediach`/`Getbpb` on Trap 13 sit on
  top of `BlockDevice`; sector 0 read → FAT/MBR signature `0x55AA` at
  offset 510 is the bring-up exit check (EC1 reads via `Rwabs`).
- **QEMU vs HW base (verified).** QEMU `raspi4b` instantiates *two*
  `generic-sdhci` controllers (`0xFE300000` and `0xFE340000`) and wires
  its emulated SD card to the **legacy EMMC at `0xFE300000`** (that one
  is clocked at 200 MHz; the emmc2 clock reads 0 Hz). Real RPi4 has the
  microSD on **EMMC2 `0xFE340000`**. Both are standard SDHCI, so the
  driver `init()` **probes EMMC2 first then EMMC** and uses whichever has
  a responding card — works on QEMU and (expected) on HW with one binary.
- **Bus power matters:** after `SRST_HC`, set `CONTROL0` Power Control
  (`0x0F00` = SD bus power + 3.3 V) before issuing commands, or the card
  never responds (every response-command times out with `CTO`). The
  register is SD-HC 3.00 **§2.2.11** *Power Control Register* (offset
  `029h`): bit 0 `SD Bus Power`, bits 3:1 `SD Bus Voltage Select`
  (`111b` = 3.3 V typ.) — which, inside the BCM's 32-bit `CONTROL0` word
  at `028h`, are bit 8 and bits 11:9. Two rules of that section we do not
  follow yet: the voltage must be selected *before* the power bit ("Before
  setting this bit, the SD Host Driver shall set SD Bus Voltage Select"),
  and the Voltage Support bits in `CAPABILITIES` should be checked first.
- **Bring-up status:** init + single-block read verified on QEMU
  `raspi4b` (reads a known sector 0 back byte-exact, `0x55AA` signature).
  Real-HW verification pending: there, the card is on EMMC2 and is the
  live boot card — read-only sector probes are safe; the EMMC2 base
  clock is already running from the firmware boot.

---

## 7. Timing limits — how long the card is allowed to take

> **Why this section exists.** The FS04 audit found a driver that
> abandoned a write after 104–200 ms, while the card was legally still
> programming; the audit then found that **no document in this project
> named a single one of these limits** (audit finding 1, finding S1). A
> driver cannot obey a limit that is written nowhere. Every number below
> is therefore given with its section, so that the next person can check
> it instead of guessing it. Reference edition: **SD Physical Layer
> Simplified Specification Ver. 9.10**; §4.6.2 is materially identical to
> the Ver. 6.00 the audit cites (REF02 diffed them — see §8).

### 7.1 What the card may take — the four limits

| # | Situation | Limit | Section |
|---|-----------|-------|---------|
| 1 | **Programming (write busy) of a High Capacity card** — the DAT-line busy after a block write | **250 ms**, for *all* write operations | §4.6.2.2 |
| 2 | **The last busy of a write operation**, the busy from a CMD12 response, and the busy after the last data block when CMD23 stopped the transfer | **500 ms** — but this allowance is granted to **SDXC and SDUC** cards that cannot hold 250 ms, in four enumerated cases a)–d). For SDHC the limit stays 250 ms | §4.6.2.2 |
| 3 | **Read** on a High Capacity / Extended Capacity card | the host **should** use a timeout of **at least 100 ms** ("100 ms timeout (minimum) for single and multiple read operations"), *instead of* computing one from TAAC/NSAC — those are fixed values on such cards | §4.6.2.1 |
| 4 | **Busy at a block gap** inside a multiple-block write | 250 ms; 500 ms only for the two-block case that spans a physical block boundary (case d) | §4.6.2.2 |

Two more from the same section, for completeness:

* **Standard Capacity cards** are the case where the timeout *is*
  computed: read = the lower of `100 × (TAAC + NSAC)` and 100 ms;
  write = the lower of `100 ×` the typical program time (read access time
  × `R2W_FACTOR` from the CSD) and 250 ms (§4.6.2.1, §4.6.2.2).
* **Erase** has no fixed limit: estimate it as *number of write blocks ×
  250 ms*, or from the erase-timeout parameters in the SD Status if the
  card provides them (§4.6.2.3).

And the sentence that frames all of them, §4.6.2: "A card shall complete
the command within the time period defined as follows or give up and
return an error message. If the host does not get any response with the
given timeout it should assume that the card is not going to respond and
try to recover (e.g. reset the card, power cycle, reject, etc.)."

### 7.2 What the specification recommends the host do

§4.6.2.2, Application Notes — of its four notes, these two are aimed
exactly at the mistake FS04 made:

* "The host should use a **fixed timeout** for write operations rather
  than using a timeout calculated from the `R2W_FACTOR` parameter."
* "It is **strongly recommended for hosts to implement more than 500 ms
  timeout value** even if the card indicates the 250 ms maximum busy
  length."

The Host Controller specification says the same from its side, as an
Implementation Note to the Timeout Control Register (SD-HC 3.00
§2.2.15): "The Physical Layer Specification Version 3.0x defines that
SDXC card may indicate 500 ms busy. Then Host Driver may need to change
timeout value for SDXC. It is also possible to set more than 500 ms
timeout regardless of card capacities."

So: **a fixed wall-clock timeout of more than 500 ms for a write, at
least 100 ms for a read.** Note what "fixed" rules out — a loop counter.
A counter is a timeout that shrinks when the CPU speeds up; on one Pi 4,
inside one measurement run, the same counter was worth 104 ms in one
block and 200 ms in another (audit §3.1).

### 7.3 The controller's own data timeout

Independently of the software budget, the SDHCI controller times the DAT
line itself and reports `Data Timeout Error`. The limit is
`CONTROL1.DATA_TOUNIT` (`Data Timeout Counter Value`, SD-HC 3.00
§2.2.15, offset `02Eh` bits 3:0): the interval is **`TMCLK × 2^(13+n)`**
for value `n`, from `0000b` = `TMCLK × 2^13` to `1110b` = `TMCLK × 2^27`
(`1111b` is reserved). We set `0xE`, the maximum — with the 52 MHz TMCLK
measured in QEMU that is ≈ 2.58 s, i.e. deliberately *longer* than the
software budget, so the software limit is the one that fires and the
recovery path is ours.

### 7.4 Initialisation timing

| Situation | Limit | Section |
|---|---|---|
| Card must be ready to accept the first command | within **1 ms** from detecting VDD min; the card may use up to **74 clocks** to prepare | §6.4.1.1 |
| Host before the first command | supply power so VDD min is reached **within 250 ms**, then "start to supply **at least 74 SD clocks** to the SD card with keeping CMD line to high" (§6.4.1.1, text under Figure 6-4); "After 1 ms VDD stable time, host provides at least 74 clocks before issuing the first command" (§6.4.1.4 (5)) | §6.4.1.1, §6.4.1.4 |
| Power-down / power-cycle | VDD below 0.5 V for at least **1 ms** | §6.4.1.3, §6.4.1.5 |
| `ACMD41` initialisation | "Card initialization shall be completed within **1 second** from the first ACMD41. The host repeatedly issues ACMD41 for at least 1 second or until the busy bit are set to 1"; and: "The host shall set ACMD41 timeout **more than 1 second**", counted from the first ACMD41 that carries a voltage window | §4.2.3, §4.2.3.1 |

### 7.5 What the Simplified Specification does **not** contain

Honest gap, so nobody looks for it twice: the bus-level timing
parameters — `N_CR` (command-to-response), `N_AC`, `N_WR` and the rest of
the "Timing Values" table — are **not** in the Simplified edition.
§4.12 and all of its subsections are listed in the table of contents as
"(Removed in the Simplified Specification)". Any statement about the
number of clock cycles between a command and its response therefore has
**no citable source in our pinned documents**; what we can rely on is
that the controller polices that interval itself and reports
`Command Timeout Error` (SD-HC 3.00 §2.2.18). A software budget on the
command phase bounds a wedged *controller*, not a slow card, and should
be documented as such.

### 7.6 What the driver does with this today

Not part of the specification — recorded here so the numbers stay
traceable. `kernel/src/drivers/bcm2711/emmc.rs`:

| Constant | Value | Derived from |
|---|---|---|
| `WRITE_BUSY_MS` | 1000 ms | §4.6.2.2 Application Note ("more than 500 ms"), doubled for margin; still below the controller's own ≈ 2.58 s |
| `READ_MS` | 200 ms | §4.6.2.1 minimum of 100 ms, doubled for the same reason |
| `CTRL_MS` | = `WRITE_BUSY_MS` | the phases §7.5 gives no card-side limit for: the two Command Inhibit waits, Command Complete |
| Buffer Write Ready | `WRITE_BUSY_MS` | since FS08 it also waits out the busy at the block gap of a CMD25 (§7.1 row 4) |
| `DATA_TOUNIT` | `0xE` | SD-HC §2.2.15 maximum, see §7.3 |

Each is a **duration on the ARM generic timer**, not an iteration count —
that is the FS04 correction. `SPIN_LIMIT` still exists but only for the
initialisation-phase polls (controller reset, clock-stable, the ACMD41
loop); turning the ACMD41 loop into the "more than 1 second" of §4.2.3.1
is audit finding 11 and still open.

---

## 8. Citing the SD specifications — edition map

The pinned editions and their checksums are in `references/README.md`.
Two traps when citing them:

**1. Physical Layer 6.00 → 9.10: the sections we use did not move.**
Diffed text-for-text on 2026-09-28:

| Section | 6.00 vs 9.10 |
|---|---|
| §4.6.2, §4.6.2.1, §4.6.2.2, §4.6.2.3 (timeouts) | same numbers, same text, except that 9.10 adds "SDUC" alongside SDXC in three sentences |
| §4.10.1 (card status) | word-identical |
| Table 4-42 (card status bits) | identical bit assignment and Type column; 9.10 extends the prose of `WP_VIOLATION` / `WP_ERASE_SKIP` ("write protected until power cycle card") and adds a note to `CURRENT_STATE` |
| Table 4-43 (per-command cross reference) | 9.10 adds rows for CMD22, CMD39, ACMD53, ACMD54 |
| §4.2.3 / §4.2.3.1 (ACMD41), §6.4.1.x (power-up) | same numbers, same wording |

So every `SD Phys 6.00 §4.6.2.x` citation already in the tree — the
audit, the driver's doc comments — is **also a correct 9.10 citation**
and needs no rewriting. New work cites 9.10.

**2. Host Controller 3.00 → 4.20: the register sections all moved by
one.** This one *does* bite. 4.20 inserts a section in Chapter 2 and
renames the clock sequences:

| Register / sequence | HC 3.00 | HC 4.20 |
|---|---|---|
| Present State (`024h`) | §2.2.9 | §2.2.9 |
| Host Control 1 (`028h`) | §2.2.10 | §2.2.11 |
| Power Control (`029h`) | **§2.2.11** | §2.2.12 |
| Clock Control (`02Ch`) | §2.2.14 | §2.2.15 |
| Timeout Control (`02Eh`) | **§2.2.15** | §2.2.16 |
| Software Reset (`02Fh`) | **§2.2.16** | §2.2.17 |
| Normal Interrupt Status (`030h`) | §2.2.17 | §2.2.18 |
| Error Interrupt Status (`032h`) | §2.2.18 | §2.2.19 |
| Capabilities (`040h`) | §2.2.25 | §2.2.26 |
| SD clock supply | §3.2.1 "SD Clock Supply Sequence" | §3.2.1 is "Internal Clock Setup Sequence"; supply+stop merge into §3.2.2 |
| SD clock stop | §3.2.2 "SD Clock Stop Sequence" | §3.2.2 "SD Clock Supply and Stop Sequence" |
| Bus power, transaction, abort, error recovery | §3.3, §3.7.1.2, §3.8, §3.10.1 | unchanged numbers — but **§3.8.1 was rewritten**: 3.00 has "Asynchronous Abort" (CMD12, then both software resets, then poll them to 0), 4.20 has "Abort Command Sequence" (CMD12 → CMD13 → check R1 for `tran`, one retry, else "Abort Failed") |

**Always write the edition in front of the section** — `SD-HC 3.00
§2.2.16` — because `§2.2.16` alone means the Software Reset Register in
one edition and the Timeout Control Register in the other. There is no
Simplified edition 4.00 or 4.10; the series is 1.00, 2.00, 3.00, 4.20.
