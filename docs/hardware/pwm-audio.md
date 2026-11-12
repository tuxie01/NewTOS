# BCM2711 PWM — analog audio at the headphone jack

> **Track B hardware note** per `docs/SOURCE-ANALYSIS-PROTOCOL.md`,
> written **before** the driver (CLAUDE.md rule 6). Occasion:
> `TASK/SND01.md`, **step 2**. Counterpart on the API side:
> `docs/api/xbios/_dmasound-group.md` (step 1) — its **buffer contract**
> (§4 there) is what this hardware has to serve.
>
> **Subject:** the BCM2711 PWM block as the audio output at the 3.5 mm
> jack of the Raspberry Pi 4 B. **Not** subject: HDMI audio (§11.1) and
> the PCM/I2S block as an output of its own (§7.4).
>
> **Anyone who writes a refill loop or an interrupt handler reads §7 and
> §8 first** — they say that the PWM block has **no interrupt**, and what
> that costs.
>
> **§10 is the list of what is NOT backed.** It is part of the note, not
> an appendix. Whoever needs a number that is listed there measures it or
> procures a source — does not guess it.
>
> **Real-Pi measurements (2026-11-12).** Facts marked **MEASURED** come
> from the owner's run of the SND03 probe on a real Pi 4:
> `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT` (values
> transcribed from three screen photos), and, for QEMU, from
> `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`. A measurement
> is an observation on one board, not a datasheet statement; where the
> text interprets it, the interpretation is labelled. **Still open:** the
> hearing test (the tones were played, nobody listened; §10.5).

---

## 0. The result up front

1. **The jack is reachable.** The path is fully described in the pinned
   BCM2711 datasheet: PWM1 on GPIO 40/41 in ALT0, both channels through
   the **common 64-word FIFO**, one sample per `RNG` clock of `clk_pwm`
   (§3, §4, §6).
2. **The PWM block has no interrupt.** Neither chapter 8 nor the
   datasheet's interrupt table knows a PWM source (§7.1). The FIFO can
   therefore **not** be refilled "from an interrupt"; exactly two ways
   remain: **refill from a foreign timer** (§7.2) or **DMA** (§7.3).
3. **There is no DMA driver in this tree.** The DMA path is fully
   documented in the datasheet and fits the Falcon buffer contract
   strikingly well (§9), but it is **additional work**: a channel driver
   with control blocks, bus pointers and GIC hookup.
4. **The load is real on the timer path.** At 49 kHz stereo the FIFO
   drains in **651 µs**; that is at least **1537 visits per second**
   against today's 200 Hz system tick, and about **98 000 peripheral
   writes per second** in continuous operation (§8). The DMA path costs
   **one** interrupt per Falcon frame.
5. **All Falcon and STe rates are hittable**, with at most **0.032 %**
   deviation (§5.5) — but the *amplitude resolution* depends on the PWM
   clock: at 50 MHz it is only **10 bit** at 49 kHz, at 100 MHz 11 bit
   (§5.4). 16-bit Falcon material loses bits there. **MEASURED (real
   Pi):** the clock that actually runs after a 100 MHz request is
   107 143 019 Hz, which gives 11.09 bit at 49 170 Hz (§5.5, last table).
6. **QEMU does not help here.** The installed QEMU has **no** model of
   the BCM PWM block (§10.6). **MEASURED (QEMU):** an access to PWM1
   aborts. The jack path can be tested on real hardware only.
7. **MEASURED (real Pi):** `clk_pwm` is **off** at boot; the firmware
   accepts a mailbox request "on, 100 MHz" but the clock then runs at
   107.143 MHz, so rates are computed from the **measured** clock, never
   from the requested one (§5.2, §5.6). The FIFO is 64 words deep in PWM
   mode (§3.5). GPIO 40/41 are already in ALT0 and `PACTL_CS` bit 23
   already selects PWM1 for DREQ 1 when the kernel starts (§6.2, §7.3).
   On the real Pi the DMA is **not cache-coherent** with the CPU in
   either direction (§7.3).

---

## 1. Sources

| # | Source | Used for |
|---|--------|----------|
| 1 | **BCM2711 ARM Peripherals, Release 4 (2022-01-18)** — `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`, sha256 `d6e16ea0…dc49dcdc78` | **primary**: PWM registers and bits (ch. 8), GPIO alternate functions (ch. 5), interrupt sources (ch. 6), DMA (ch. 4), system timer (ch. 10), address translation (§1.2.4) |
| 2 | Firmware device tree `bcm2711-rpi-4-b.dtb` of the boot card — the local backup sd-card-backup-20260618/boot-files/ (gitignored, no longer in the tree), 56 407 bytes, sha256 `75761b73c284e26623e4d1624bff13e67bce2ae620880efd81d6571a3739fcfb`, 2026-04-17 | **board level**: which PWM block on which pins, oscillator frequency, requested clock rate, DMA channel mask, GIC numbers. Ranks below the datasheet for everything the datasheet covers (`references/README.md`, section "Raspberry Pi firmware device tree") |
| 3 | `references/circle` @ `6177984e` — `references/circle/lib/sound/pwmsoundbasedevice.cpp`, `references/circle/lib/gpioclock.cpp`, `references/circle/include/circle/bcm2835.h`, `references/circle/include/circle/gpioclock.h` | **tertiary cross-reference**, never authority: a running RPi4 PWM sound driver. Used here only to cross-check our own derivations and to **make one gap visible** (§5.2) |
| 4 | This tree, read: `kernel/src/drivers/timer.rs`, `kernel/src/drivers/gic.rs`, `kernel/src/drivers/bcm2711/` | what interrupt and timer infrastructure **already exists** (§7.2) |
| 5 | `docs/api/xbios/_dmasound-group.md` | the buffer contract this hardware has to serve (§9) |
| 6 | **Real-Pi and QEMU measurements, 2026-11-12** — `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT` (real Pi 4, owner's run), `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT` (QEMU raspi4b, developer's run), probe `kernel/src/snd_probe.rs`, ticket `TASK/SND03.md` | **measured facts**, marked **MEASURED** below |

**Citation rule (lesson from REF02).** The edition stands **before** the
section number: "BCM2711 Rel. 4 §8.6, Table 153", not "§8.6". The
BCM2711 is a living document; table numbers have shifted between
editions. Page numbers are **not** cited.

**What was missing when this note was written:** the ARM ARM (`references/README.md`
checklist entry arm-arm = TODO). It is in `references/arm-arm/` now (2026-11-12, DDI0487M);
this note has not been re-read against it. This note therefore contains **no** ARM architecture statement beyond
what `kernel/src/drivers/timer.rs` has already established. Likewise the
**BCM2835** datasheet is not present here; where BCM2835 lore is in
circulation (FIFO depth, PWM clock register), it is **not** cited but
kept as a gap.

---

## 2. Which block, which addresses

Two PWM instances, two output channels each:

> "The BCM2711 device has two instances of this block, named PWM0 and
> PWM1 (each with two output channels)." — BCM2711 Rel. 4 §8.2

| Block | Bus address (legacy) | ARM address | Source |
|---|---|---|---|
| PWM0 | `0x7E20_C000` | `0xFE20_C000` | Rel. 4 §8.6 ("The PWM0 register base address is 0x7e20c000"); translation §1.2.4 |
| **PWM1** ← ours | `0x7E20_C800` | **`0xFE20_C800`** | ditto ("… and the PWM1 register base address is 0x7e20c800") |

Translation rule verbatim, BCM2711 Rel. 4 §1.2.4: "a peripheral
described in this document as being at legacy address `0x7Enn_nnnn` … is
… visible to the ARM at `0x0_FEnn_nnnn` if Low Peripheral mode is
enabled." This is the same base our other drivers already use
(`kernel/src/drivers/bcm2711/mod.rs` (module head)).

Further addresses this path touches (all via §1.2.4):

| Block | Bus | ARM | Source |
|---|---|---|---|
| GPIO | `0x7E20_0000` | `0xFE20_0000` | Rel. 4 §5.2 |
| Clock manager (CPRMAN) | `0x7E10_1000` | `0xFE10_1000` | Rel. 4 §5.4.2 ("The General Purpose clocks register base address is 0x7e101000") |
| DMA (channels 0–14) | `0x7E00_7000` | `0xFE00_7000` | Rel. 4 §4.2 |
| System timer | `0x7E00_3000` | `0xFE00_3000` | Rel. 4 §10.1 |
| `PACTL_CS` | `0x7E20_4E00` | `0xFE20_4E00` | Rel. 4 §4.2.1.3 and §6.2.4 |

**Peripheral access and barriers.** BCM2711 Rel. 4 §1.3: "place a memory
write barrier before the first write to a peripheral" and "a memory read
barrier after the last read of a peripheral"; expressly **not** after
each single access, but at the entry and exit points of the peripheral
code. For a refill loop over 64 FIFO words this means: **one** barrier
before, **one** after — not 64. The same section expressly requires that
interrupt routines are bracketed in the same way.

---

## 3. The register set

BCM2711 Rel. 4 §8.6, Table 152 (offsets relative to the block base):

| Offset | Name | Role |
|---|---|---|
| `0x00` | `CTL` | mode, channel enable, FIFO use, FIFO clear |
| `0x04` | `STA` | FIFO status, error and gap flags, channel state |
| `0x08` | `DMAC` | DMA enable and the two thresholds (DREQ, PANIC) |
| `0x10` | `RNG1` | range (= period length in `clk_pwm` clocks) channel 1 |
| `0x14` | `DAT1` | data word channel 1 when `USEF1 = 0` |
| `0x18` | `FIF1` | **FIFO input, common to both channels**, write-only |
| `0x20` | `RNG2` | range channel 2 |
| `0x24` | `DAT2` | data word channel 2 when `USEF2 = 0` |

There is **no** interrupt enable register and **no** interrupt status
bits. That is not an omission in this table — see §7.1.

### 3.1 `CTL` — BCM2711 Rel. 4 §8.6, Table 153

| Bit | Name | Meaning (datasheet) | Reset |
|---:|---|---|:--:|
| 15 | `MSEN2` | 0 = PWM algorithm, 1 = M/S transmission | 0 |
| 13 | `USEF2` | 0 = `DAT2` is sent, 1 = FIFO | 0 |
| 12 | `POLA2` | 1 = output inverted | 0 |
| 11 | `SBIT2` | output level **when not transmitting** | 0 |
| 10 | `RPTL2` | 0 = transmission stops when the FIFO is empty; 1 = last word is repeated | 0 |
| 9 | `MODE2` | 0 = PWM mode, 1 = serialiser | 0 |
| 8 | `PWEN2` | enable channel 2 | 0 |
| 7 | `MSEN1` | like `MSEN2`, channel 1 | 0 |
| 6 | `CLRF` | **one-shot**: writing 1 clears the FIFO; always reads 0 (`W1SC`) | 0 |
| 5 | `USEF1` | like `USEF2`, channel 1 | 0 |
| 4 | `POLA1` | like `POLA2`, channel 1 | 0 |
| 3 | `SBIT1` | like `SBIT2`, channel 1 | 0 |
| 2 | `RPTL1` | like `RPTL2`, channel 1 | 0 |
| 1 | `MODE1` | like `MODE2`, channel 1 | 0 |
| 0 | `PWEN1` | enable channel 1 | 0 |

From the description text of Table 153, because it cannot be seen from
the bit table:

* "All registers and FIFOs are writeable without setting this bit
  [`PWENi`]" — the FIFO may be pre-filled **before** the channel is
  enabled.
* In PWM mode (`MODEi = 0`), `MSENi` decides whether the even
  distribution algorithm (§8.3) or a simple mark/space output is used.
  Datasheet on `MSEN = 1`: "This mode may be preferred if high frequency
  modulation is not required or has negative effects."
* "The channel sends its output continuously as long as the data
  register is used (`USEFi = 0`), or the FIFO is used and it is not
  empty." (§8.4) — **this is the underrun rule**: empty FIFO = end of
  transmission.

### 3.2 `STA` — BCM2711 Rel. 4 §8.6, Table 154

| Bit | Name | Type | Meaning |
|---:|---|:--:|---|
| 10 | `STA2` | RO | 1 = channel 2 is transmitting |
| 9 | `STA1` | RO | 1 = channel 1 is transmitting |
| 8 | `BERR` | W1C | bus error on register write via APB |
| 5 | `GAPO2` | W1C | **gap** between two words on channel 2 |
| 4 | `GAPO1` | W1C | **gap** between two words on channel 1 |
| 3 | `RERR1` | W1C | read on empty FIFO |
| 2 | `WERR1` | W1C | write on full FIFO |
| 1 | `EMPT1` | RO | FIFO empty (reset: **1**) |
| 0 | `FULL1` | RO | FIFO full |

**There are only `FULL1` and `EMPT1` — no fill level indicator and no
half-full threshold.** Software can therefore not read the fill level;
it can only distinguish "full" and "empty" and has to count along.

An underrun reports itself through `GAPOi` (datasheet: "there has been a
gap between transmission of two consecutive data from FIFO. This may
happen when the FIFO becomes empty after the state machine has sent a
word and is waiting for the next word. If control bit `RPTLi` is set to
high this event will not occur.") and through `RERR1`.

`BERR` is the hint that writing **to the same register group** in too
quick succession overruns the synchroniser: "This may happen if the bus
tries to write successively to same set of registers faster than the
synchroniser block can cope with." The datasheet gives **no** minimum
wait time — see §10.2.

**MEASURED (real Pi,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`, H2):**
register values as found at probe time: `CTL 00000000`, `STA 00000002`
(= `EMPT1`), `DMAC 00000707`, `RNG1 00000020`, `RNG2 00000020` — equal
to the reset values of Tables 153–156. After `CLRF` `STA` is `00000002`,
at `FULL1` `00000001`, after one more write `00000005` (`WERR1` = 1 plus
`FULL1`; bit decode per the table above). With the channels enabled and
the FIFO running, `STA` read `00000601` (bits 10, 9 and 0 =
`STA2`, `STA1`, `FULL1`; no `GAPOi`, `RERR1`, `WERR1`, `BERR`). The
as-found timing run (clock off, §5.2) ended with `STA 00000101`
(`BERR` and `FULL1`); the cause of that `BERR` is not established.

### 3.3 `DMAC` — BCM2711 Rel. 4 §8.6, Table 155

| Bit | Name | Meaning | Reset |
|---:|---|---|:--:|
| 31 | `ENAB` | start DMA | 0 |
| 15:8 | `PANIC` | threshold for the PANIC signal | `0x07` |
| 7:0 | `DREQ` | threshold for the DREQ signal | `0x07` |

The datasheet says only "threshold level for … going active" — **not**
whether the threshold means the fill level or the free space. See §10.3.
**MEASURED (real Pi, H2):** `DMAC` reads `00000707` as found, i.e. the
reset value 7/7 is what the firmware leaves.

### 3.4 `RNG`, `DAT`, `FIF1` — Tables 156/157/158

* `RNG1`/`RNG2`, 32 bit, **reset `0x0000_0020`** (= 32): "In PWM mode,
  evenly distributed pulses are sent within a period of length defined by
  this register." In serialiser mode, `RNG` is sent bit-exact: "If the
  value in `PWM_RNGi` is less than 32, only the first `PWM_RNGi` bits are
  sent resulting in a truncation. If it is larger than 32, excess zero
  bits are padded at the end of data."
* `DAT1`/`DAT2`, 32 bit, reset 0: valid only for `USEFi = 0`. "In PWM
  mode, data is sent by pulse width modulation: the value of this
  register defines the number of pulses which are sent within the period
  defined by `PWM_RNGi`."
* `FIF1`, **write-only** (`WO`): "This register is the FIFO input for
  **both** the channels."

### 3.5 The FIFO — depth, sharing, rules

**Depth: 64 words of 32 bit each**, BCM2711 Rel. 4 §8.1, bullet point:
"Serialise mode configured to read data from a FIFO storage block, which
can store up to sixty-four 32-bit words."

> **Honest about the source situation:** this sentence stands under the
> *serialiser* bullet. That the same FIFO is used in PWM mode with
> `USEFi = 1` is stated in the `CTL` description ("data stored in either
> `PWM_DATi` or FIFO is transmitted by pulse width modulation") and in the
> `FIF1` description; that it has the same **depth** there was
> **inference, not quotation**. A driver should therefore not rely on the
> number 64 but check `FULL1` — that is the only indication valid at run
> time anyway.
>
> **MEASURED (real Pi,
> `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`, H2):** with
> the channels disabled, **64 words** were accepted until `FULL1` was set
> (`STA` after `CLRF` `00000002`, at `FULL1` `00000001`, after one more
> write `00000005` = `WERR1` 1). The probe's summary line reads "H2 FIFO
> depth 64 words". The depth is thus measured for PWM mode on this board;
> the run counted with the channels disabled — the depth with the
> channels draining was not counted separately. The advice to check
> `FULL1` stands. A write to a full FIFO sets `WERR1`.

The sharing rules for stereo operation are given completely in the
`FIF1` description (Rel. 4 §8.6, before Table 158):

1. **Interleaving is built in.** "if the word series A B C D E F G H
   I .. is written to the FIFO and both channels are active and
   configured to use FIFO, then channel 1 will transmit words A C E G I
   .. and channel 2 will transmit words B D F H ..". An `L R L R …`
   stream thus fits into the FIFO **unchanged**.
2. **Both channels must have the same `RNG`.** "If the channel range
   (period) value of one channel is different to the other, this will
   cause the channel with the smaller range value to wait between words,
   hence resulting in gaps between words. To avoid that, each channel
   sharing the FIFO should be configured to use the same range value."
3. **`RPTLi` must be 0 when shared.** "the `RPTLi` bits are not
   meaningful when the FIFO is shared between channels as there is no
   defined channel to own the last data in the FIFO. Therefore sharing
   channels must have their `RPTLi` bit set to zero."
4. **After a reconfiguration: clear the FIFO.** "If the set of channels
   sharing the FIFO has been modified after a configuration change, the
   FIFO should be cleared before writing new data." — `CTL.CLRF`.

**Consequence from (3) + `SBITi` + `STAi`, labelled as inference:** in
stereo operation there is no "hold the last value" protection. If the
FIFO runs empty, transmission stops (§8.4), `STAi` goes to 0, and the
output takes the `SBITi` level — preset 0, i.e. the lower stop. For a
unipolar PWM converter whose rest position is the *middle*, that is a DC
step, hence an **audible click**. An underrun here is therefore not "a
bit of noise" but a click. The individual parts are backed (`§8.4`,
Table 153 `SBIT`, Table 154 `STA`); the audibility statement is the
inference.

> **Cross-reference with contradiction.** `references/circle`
> (`references/circle/lib/sound/pwmsoundbasedevice.cpp:165-166`) sets `RPTL1|RPTL2` when
> stopping, expressly "to avoid clicks", and clears them again on
> starting — exactly what rule (3) excludes for shared FIFOs. **The
> datasheet is primary.** If we want to adopt this trick, it is to be
> recorded as a deliberate deviation and measured on real hardware, not
> taken over from circle.

---

## 4. Operating modes — which one for audio?

BCM2711 Rel. 4 §8.4 knows three:

| Mode | `MODEi` | `MSENi` | What is sent |
|---|:--:|:--:|---|
| PWM, even distribution | 0 | 0 | `N` pulses evenly spread over `RNG` clocks (algorithm §8.3) |
| PWM, mark/space | 0 | 1 | one contiguous pulse of length `N` within `RNG` clocks (Fig. 15) |
| Serialiser | 1 | — | the word is output bit by bit, MSB first, over `RNG` clocks |

For audio the **PWM mode** is meant: one sample per period. Whether
`MSEN = 0` or `1` sounds better depends on the low-pass of the board; the
datasheet says only the sentence quoted above and **nothing** about the
board (§10.5). The serialiser mode is not meant for audio — it outputs
bits, not amplitudes.

**MEASURED (real Pi, tones T1–T4):** both `MSEN = 0` (T1–T3) and
`MSEN = 1` (T4, mark/space) were run on PWM1 without any FIFO error
flag; which one sounds better is **not** measured (hearing test open,
§10.5).

---

## 5. The clock

### 5.1 What the datasheet says

Two sentences, no more:

* §8.1: "Both modes clocked by **`clk_pwm` which is nominally 100MHz**,
  but can be varied by the clock manager."
* §8.5: "PWM clock source and frequency is controlled in **CPRMAN**."

### 5.2 What the datasheet does **not** say — the central gap

Of the clock manager (base `0x7E10_1000`, 8 KiB) the datasheet documents
**only the three general-purpose clocks**: §5.4.2, Table 98 lists
`CM_GP0CTL`/`DIV` (`0x70`/`0x74`), `CM_GP1CTL`/`DIV` (`0x78`/`0x7c`),
`CM_GP2CTL`/`DIV` (`0x80`/`0x84`) — and nothing else. **A register
`CM_PWMCTL`/`CM_PWMDIV` does not occur anywhere in the whole document**
(sweep over the complete text extract: the only hit for "0x7e1010" is the
base address sentence in §5.4.2).

What the datasheet **does** deliver is the **bit layout** that holds for
the GP clocks — and that is the part a driver really needs:

`CM_GPnCTL`, Rel. 4 §5.4.2, Table 99:

| Bits | Name | Meaning |
|---|---|---|
| 31:24 | `PASSWD` | password `0x5A`, write-only |
| 10:9 | `MASH` | 0 = integer, 1…3 = MASH stages |
| 8 | `FLIP` | invert output (test/debug only) |
| 7 | `BUSY` | RO: clock generator is running — **source and divider must not be changed while set** |
| 5 | `KILL` | stop generator hard (test/debug only) |
| 4 | `ENAB` | request/switch off generator, glitch-free |
| 3:0 | `SRC` | 0 = GND, 1 = oscillator, 4 = PLLA per, 5 = PLLC per, 6 = PLLD per, 7 = HDMI aux, 8–15 = GND |

`CM_GPnDIV`, Table 100: `PASSWD` 31:24, **`DIVI` 23:12** (integer
part), **`DIVF` 11:0** (fraction).

Frequency formula, Rel. 4 §5.4, Table 96:

| `MASH` | min `DIVI` | average output frequency |
|:--:|:--:|---|
| 0 | 1 | `source / DIVI` |
| 1 | 2 | `source / (DIVI + DIVF/1024)` |
| 2 | 3 | `source / (DIVI + DIVF/1024)` |
| 3 | 5 | `source / (DIVI + DIVF/1024)` |

> **Contradiction in the document, noted and not resolved:** Table 96
> divides `DIVF` by **1024**, while Table 100 lists `DIVF` as **12 bit**
> (0…4095). The document's own worked example Table 97 supports 1024
> (divider 35.480 → `DIVI` 35, `DIVF` 492; 492/1024 = 0.4805).
> **For us the dispute is avoidable:** with `MASH = 0` there is no
> `DIVF`. Whoever wants to use MASH has to clarify the question first
> (§10.4).

§5.4 also warns: with MASH "the user must ensure that the module is
not exposed to frequencies higher than 25MHz" — that is a statement
about the *GPIO* clocks; whether it holds for `clk_pwm` is stated
nowhere (§10.4).

**Where the register offset for the PWM clock would have to come from.**
Three possibilities, with their price in certainty:

| Way | What it delivers | Cost in certainty |
|---|---|---|
| **A — do not touch** | We do not program `clk_pwm` at all and **measure** instead what is running (§5.6). | Safest: no undocumented address. Price: we take the frequency we find and choose `RNG` accordingly. |
| **B — mailbox** | The firmware property interface has `SET_CLOCK_RATE` = `0x0003_8002` (`references/circle/include/circle/bcmpropertytags.h:50`; our driver already knows `GET_CLOCK_RATE` `0x0003_0002`, `kernel/src/drivers/bcm2711/mailbox.rs:339@PROPTAG_GET_CLOCK_RATE`). | **Was open**: circle's identifier list (`:182-187`) lists `EMMC`, `UART`, `ARM`, `CORE`, `EMMC2`, `PIXEL_BVB` — **no PWM identifier**. Whether the firmware knows one could not be established from our pinned sources. **Now MEASURED, see below.** |
| **C — CPRMAN directly** | circle computes the register offset from the clock number: `nCtlReg = ARM_CM_BASE + Clock*8` (`references/circle/lib/gpioclock.cpp:66`) with `GPIOClockPWM = 20` (`references/circle/include/circle/gpioclock.h:37`), i.e. `0x7E1010A0`/`0x7E1010A4`. | **This address is in none of our pinned documents.** It comes from a tertiary cross-reference. Adopting it would bend rule 6 ("No invented hardware") at a point where way A exists. |

**Recommendation of this note: way A**, as a fallback B if someone
procures a firmware source. C only with an express decision and recorded
as such.

**MEASURED (real Pi, H1,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`) — what way A
and way B yielded:**

* **`clk_pwm` is off when the kernel starts.** Firmware view as found
  (mailbox, clock id `0x0a` = PWM): state 0 (off), rate 0, measured
  65 917 968 Hz, max 500 000 000 Hz. PWM1 timing as found, `RNG` 10 000:
  `STA 00000101`, **no FIFO word consumed in 200 ms** — `clk_pwm` is not
  running.
* **Way B works on this firmware.** The probe asked the firmware: Set
  clock state "on" → reply 1; Set clock rate 100 000 000 → reply
  100 000 000. Afterwards: state 1, rate 100 000 000, **measured
  107 143 064 Hz**, max 500 000 000. The firmware thus answers to clock
  id `0x0a`; the id itself is not sourced in our pinned documents (it is
  the value the probe uses) — what is measured is the answer.
* **The requested rate is not the effective rate.** FIFO timing after
  the request, `RNG` 10 000: `STA 00000601`; 10 716 words consumed in
  27 004 279 ticks (`CNTFRQ` 54 000 000), 10 720 written ⇒
  **`clk_pwm` = 107 143 019 Hz** (10 714 periods/s per channel). The
  firmware's own measurement (107 143 064 Hz) and the FIFO timing agree
  to 0.00004 %. The firmware accepts 100 MHz, but the clock runs at
  107.143 MHz: **software must use the measured rate, not the requested
  one.**
* **Interpretation, unsourced (not a fact until sourced):**
  107 142 857 Hz = 750 MHz / 7, i.e. an integer divider on a 750 MHz
  source. No pinned document names the source or the divider.
* **The clock-manager register stays unsourced.** The probe read, read
  only, the words at circle's PWM offset `0xa0`/`0xa4` (the unsourced
  offset of way C): `00000200 00000000` as found, `00000096 00007800`
  after the request. These raw values are recorded but **not
  interpreted** here; they do not source the register.

### 5.3 What the device tree says

From `bcm2711-rpi-4-b.dtb` (decompiled by ourselves, sha256 as in §1):

```
clocks: cprman@7e101000 { compatible = "brcm,bcm2711-cprman";
                          reg = <0x7e101000 0x2000>; }            /* l. 161-167 */

clk_osc: clk-osc { compatible = "fixed-clock";
                   clock-frequency = <0x337f980>; }               /* 54 000 000 Hz, l. 2019-2024 */

pwm:  pwm@7e20c000 { compatible = "brcm,bcm2835-pwm";
                     reg = <0x7e20c000 0x28>;
                     clocks = <cprman 30>;
                     assigned-clock-rates = <0x2faf080>;          /* 50 000 000 Hz */
                     status = "disabled"; }                       /* l. 1377-1386 */

pwm1: pwm@7e20c800 { … same picture …
                     pinctrl-0 = <&pwm1_0_gpio40 &pwm1_1_gpio41>;
                     status = "disabled"; }                       /* l. 1725-1736 */
```

Three statements from it, cleanly separated:

1. **The oscillator runs at 54 MHz.** That is a board statement of the
   Raspberry Pi firmware; for `SRC = 1` the datasheet gives only the
   name "oscillator", no frequency.
2. **Both PWM blocks are switched off from the factory** (`status =
   "disabled"`). We would have to switch them on ourselves — and nobody
   else uses them as long as we do not boot Linux.
3. **`assigned-clock-rates = 50 MHz` is a wish to Linux's clock
   framework, not a state statement.** It means: *if* the
   `brcm,bcm2711-cprman` driver loads, it programs 50 MHz. Our kernel is
   not Linux. **What `clk_pwm` actually does at the jump into
   `kernel8.img` the device tree does not say** — the datasheet says
   "nominally 100MHz", circle programs 125 MHz for RPi4
   (`references/circle/lib/sound/pwmsoundbasedevice.cpp:40`). Three numbers, three scopes
   of validity. **Hence way A from §5.2: measure.**
   **MEASURED (real Pi, §5.2):** at the jump into `kernel8.img` the clock
   is **off** (state 0); after the mailbox request 100 MHz it runs at
   107.143 MHz. None of the three numbers (50, 100, 125 MHz) is the
   effective one.

### 5.4 The derivation: clock, range, sample rate, resolution

From §8.3/§8.4 and the `RNG` description (§8.6, Table 156) it follows
directly — **this is arithmetic, not quotation**:

* A PWM period lasts `RNG` clocks of `clk_pwm`.
* **One** value is output per period (from `DATi` or from the FIFO).

So:

```
f_sample = f_pwm / RNG                     (one sample per period)
RNG      = round(f_pwm / f_sample)         (the inverse, integer)
Levels   = RNG + 1   (values 0 … RNG)   →   resolution ≈ log2(RNG) bit
```

Sample rate and amplitude resolution thus hang on **the same knob**:
whoever wants a high rate gets few bits, and vice versa. Only a higher
`f_pwm` improves both.

> **Cross-checked, not copied:** circle computes
> `m_nRange = (CLOCK_RATE + nSampleRate/2) / nSampleRate`
> (`references/circle/lib/sound/pwmsoundbasedevice.cpp:109`) — the same formula, rounded.
> It also sets `RNG1 = RNG2 = m_nRange` and
> `CTL = PWEN1|USEF1|PWEN2|USEF2 [|MSEN1|MSEN2] |CLRF`
> (`:217-223`), i.e. exactly the configuration §3.5 prescribes. The
> agreement is a confirmation of the derivation, **not** its source.
>
> **MEASURED (real Pi, H1):** the formula was exercised with the probe's
> tone rate: `RNG 2233` gave 47 981 Hz at the measured clock (tones
> T1–T4, §3.2 values: no gap, no error), and the FIFO-timing run
> (`RNG 10 000`, 10 714 periods/s per channel) matches
> `f_pwm / RNG` with `f_pwm` = 107 143 019 Hz by construction (that run
> is what determined `f_pwm`). The formula is therefore consistent with
> what the hardware consumes; `f_pwm` in it must be the **measured**
> clock (§5.2).

**Sign.** The PWM value is an **unsigned** pulse count `0 … RNG`; Falcon
samples are **signed** (`docs/api/xbios/_dmasound-group.md` §4.8, from
FreeMiNT). The conversion is thus `pwm = ((sample + 2^(n-1)) * RNG) >> n`
for `n`-bit material — **arithmetic, no source**, and the rounding
strategy (truncation, noise shaping) is a design decision, not a
hardware requirement.

### 5.5 Which Falcon rates are hittable

`RNG = round(f_pwm / f_sample)`, target rates from
`docs/api/xbios/_dmasound-group.md` §4.7 (Falcon table from FreeMiNT,
STe rates measured there). "Bit" = `log2(RNG+1)`, the achievable
amplitude resolution.

**f_pwm = 50 MHz** (the value the device tree provides for Linux):

| Target rate | `RNG` | Actual rate | Error | Bit |
|---:|---:|---:|---:|---:|
| 49 170 (Falcon `prescale` 1) | 1 017 | 49 164.21 | −118 ppm | 9.99 |
| 32 780 (2) | 1 525 | 32 786.89 | +210 ppm | 10.58 |
| 24 585 (3) | 2 034 | 24 582.10 | −118 ppm | 10.99 |
| 19 668 (4) | 2 542 | 19 669.55 | +79 ppm | 11.31 |
| 16 390 (5) | 3 051 | 16 388.07 | −118 ppm | 11.58 |
| 12 292 (7) | 4 068 | 12 291.05 | −77 ppm | 11.99 |
| 9 834 (9) | 5 084 | 9 834.78 | +79 ppm | 12.31 |
| 8 195 (11) | 6 101 | 8 195.38 | +46 ppm | 12.58 |
| 50 066 (STe /160) | 999 | 50 050.05 | **−319 ppm** | 9.97 |
| 25 033 (STe /320) | 1 997 | 25 037.56 | +182 ppm | 10.96 |
| 12 517 (STe /640) | 3 995 | 12 515.64 | −108 ppm | 11.96 |
| 6 285 (STe 6 kHz) | 7 955 | 6 285.36 | +57 ppm | 12.96 |

**f_pwm = 100 MHz** (the datasheet's "nominal" value; **not** what runs
after a 100 MHz request, see the measured table below):

| Target rate | `RNG` | Actual rate | Error | Bit |
|---:|---:|---:|---:|---:|
| 49 170 | 2 034 | 49 164.21 | −118 ppm | 10.99 |
| 32 780 | 3 051 | 32 776.14 | −118 ppm | 11.58 |
| 24 585 | 4 068 | 24 582.10 | −118 ppm | 11.99 |
| 19 668 | 5 084 | 19 669.55 | +79 ppm | 12.31 |
| 16 390 | 6 101 | 16 390.76 | +46 ppm | 12.58 |
| 12 292 | 8 135 | 12 292.56 | +46 ppm | 12.99 |
| 9 834 | 10 169 | 9 833.81 | −19 ppm | 13.31 |
| 8 195 | 12 203 | 8 194.71 | −36 ppm | 13.58 |
| 50 066 | 1 997 | 50 075.11 | +182 ppm | 10.96 |
| 25 033 | 3 995 | 25 031.29 | −68 ppm | 11.96 |
| 12 517 | 7 989 | 12 517.21 | +17 ppm | 12.96 |
| 6 285 | 15 911 | 6 284.96 | −6 ppm | 13.96 |

**f_pwm = 125 MHz** (circle's choice for RPi 4 — for comparison only,
no recommendation): the same rates, errors ≤ 118 ppm, resolution
11.3…14.3 bit, largest `RNG` = 19 889 at 6 285 Hz.

**f_pwm = 107 143 019 Hz** (the clock **MEASURED** on the real Pi after
the request "on, 100 MHz", §5.2; **the rates below are arithmetic** from
that measured clock, the clock itself is the measurement):

| Target rate | `RNG` | Actual rate | Error | Bit |
|---:|---:|---:|---:|---:|
| 49 170 (Falcon `prescale` 1) | 2 179 | 49 170.73 | +15 ppm | 11.09 |
| 32 780 (2) | 3 269 | 32 775.47 | −138 ppm | 11.68 |
| 24 585 (3) | 4 358 | 24 585.36 | +15 ppm | 12.09 |
| 19 668 (4) | 5 448 | 19 666.49 | −77 ppm | 12.41 |
| 16 390 (5) | 6 537 | 16 390.24 | +15 ppm | 12.67 |
| 12 292 (7) | 8 716 | 12 292.68 | +56 ppm | 13.09 |
| 9 834 (9) | 10 895 | 9 834.15 | +15 ppm | 13.41 |
| 8 195 (11) | 13 074 | 8 195.12 | +15 ppm | 13.67 |
| 50 066 (STe /160) | 2 140 | 50 066.83 | +17 ppm | 11.06 |
| 25 033 (STe /320) | 4 280 | 25 033.42 | +17 ppm | 12.06 |
| 12 517 (STe /640) | 8 560 | 12 516.71 | −23 ppm | 13.06 |
| 6 285 (STe 6 kHz) | 17 047 | 6 285.15 | +24 ppm | 14.06 |

Largest error in this table: **138 ppm** (32 780 Hz). The four STe
rates are hit within 23 ppm. **Owner decision Q-N6**
(`TASK/SND01.md`, "Decisions", 2026-11-12, after H1): **no
resampling**; every rate is set directly through the PWM range register,
computed from the measured clock, never from the requested one. The STe 6 kHz rate is 6 285 Hz as listed in
`docs/api/xbios/_dmasound-group.md` §4.7 (the first version of the SND01
entry wrote 6258 Hz; corrected there).

**Result.** *Every* rate the DMA sound spec knows is hittable; the
largest error over all clocks above is **319 ppm ≈ 0.032 %**, i.e. a
good half cent of pitch — inaudible, but **not exact**. A program that
counts samples against `_hz_200` drifts by that amount. Answering that
is up to the API side, not the hardware.

**Where it hurts is the resolution, not the rate.** At 50 MHz and 49 kHz
**10 bit** remain. Falcon `STEREO16` delivers 16; six bits are lost.
`STEREO8`/`MONO8`, in contrast, fit everywhere. For the acceptance of
SND01 ("a program audibly plays a buffer") that is amply enough; for a
claim "16-bit playback" it is not. The honest way is to document the
effective resolution. At the measured 107.143 MHz the figure is 11.09 bit
at 49 170 Hz — better than 50 MHz, still far from 16.

### 5.6 How to measure `clk_pwm` instead of assuming it

Observation, no intervention (way A from §5.2), with what the kernel
already has:

1. Set `RNG1 = RNG2 = R`, both channels to FIFO (`USEF`), `CLRF`, enable
   the channels.
2. Write exactly `N` words into `FIF1` and read the free-running
   counter — `CNTPCT_EL0` with `CNTFRQ_EL0` as base, both already read by
   `kernel/src/drivers/timer.rs` (`cntfrq()`, module head) —, then wait
   until `EMPT1` is set **and** `STA1`/`STA2` are 0, and read again.
3. `f_pwm ≈ (N/2) · R / Δt` per channel (each channel gets half of the
   words, §3.5 rule 1).

This is a pure read operation against documented status bits and
delivers the only number that really holds: the one present on *this*
board. **Must run on real hardware** — QEMU does not know the block
(§10.6).

**MEASURED (real Pi, H1, `measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):**
this method was carried out: `RNG` 10 000, 10 716 words consumed in
27 004 279 ticks at `CNTFRQ` 54 000 000 with 10 720 words written ⇒
107 143 019 Hz, agreeing with the firmware's own clock measurement
(107 143 064 Hz) to 0.00004 %. With the clock off the same method
consumes **no** word (200 ms), which is how "clock not running" is
detected.

---

## 6. The GPIO side

### 6.1 Which pins, from the datasheet

BCM2711 Rel. 4 §5.3, table of alternate functions (column header
`GPIO | Pull | ALT0 | ALT1 | ALT2 | ALT3 | ALT4 | ALT5`):

| GPIO | Pull | **ALT0** | ALT1 | ALT2 | ALT3 | ALT4 | ALT5 |
|---|---|---|---|---|---|---|---|
| 40 | Low | **`PWM1_0`** | `SD4` | — | `SD1_DAT4` | `SPI0_MISO` | `TXD1` |
| 41 | Low | **`PWM1_1`** | `SD5` | ⟨reserved⟩ | `SD1_DAT5` | `SPI0_MOSI` | `RXD1` |

The same assignment once more in the short overview §8.5 ("GPIOs are
assigned to PWM channels as below"): `GPIO40 → PWM1_0`,
`GPIO41 → PWM1_1`. With that, **ALT0 on 40/41 is backed by the
datasheet** — that was the task.

For completeness the other PWM pins from §8.5: `GPIO12 → PWM0_0`,
`GPIO13 → PWM0_1` (ALT0), `GPIO18 → PWM0_0`, `GPIO19 → PWM0_1` (ALT5),
`GPIO45 → PWM0_1` (ALT0). On the Pi 4 B, 12/13/18/19 are on the 40-pin
header, not at the jack.

### 6.2 How to switch

GPIO base `0x7E20_0000` (Rel. 4 §5.2, Table 63); `GPFSEL4` is at offset
**`0x10`** and covers GPIO 40–49. Rel. 4 §5.2, Table 68: **`FSEL40` =
bits 2:0**, **`FSEL41` = bits 5:3**.

Encoding, Rel. 4 §5.2, Table 64 (written out with `FSEL9` as the
example, valid for all):

| Value | Meaning |
|:--:|---|
| `000` | input |
| `001` | output |
| **`100`** | **alternate function 0** |
| `101` | ALT1 |
| `110` | ALT2 |
| `111` | ALT3 |
| `011` | ALT4 |
| `010` | ALT5 |

So: `GPFSEL4[2:0] = 0b100` and `GPFSEL4[5:3] = 0b100`, other bits
unchanged (read–modify–write). Reset is `000` — "All pins reset to
normal GPIO input operation" (description text of Table 64).

**MEASURED (real Pi, tones page,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):** `GPFSEL4`
reads `00000064` **before** the probe touches it, still `00000064`
after the probe's own ALT0 write, and `00000064` after the tones:
**GPIO 40/41 are already in ALT0 (= PWM1) when the kernel starts** — set
by the firmware. Decoded with the table above (decode, not separately
measured): `0x64` = `FSEL40` `100`, `FSEL41` `100`, and `FSEL42` (bits
8:6) `001` = output, i.e. the firmware also leaves GPIO 42 as an output.
A driver still sets ALT0 itself (read–modify–write); it should not rely
on the firmware's state.

On the pull: the device tree demands `bias-disable` for both pins
(§6.3); the pull registers `GPIO_PUP_PDN_CNTRL_REG*` are in Rel. 4 §5.2,
Table 63 from offset `0xe4`. An output needs no pull; whether the preset
"Low" of the table in §5.3 means anything different is not pursued
further.

### 6.3 What the device tree says — and a correction

The `pwm1` node points to two pin groups (l. 1734,
`pinctrl-0 = <0x28 0x29>`); resolved, these are the nodes

```
pwm1_0_gpio40 { pin-pwm { pins = "gpio40"; function = "alt0"; bias-disable; } }   /* phandle 0x28, l. 659-667 */
pwm1_1_gpio41 { pin-pwm { pins = "gpio41"; function = "alt0"; bias-disable; } }   /* phandle 0x29, l. 689-697 */
```

That agrees with the datasheet.

> **Correction to the ticket.** `TASK/SND01.md:56-62` reads the numbers
> `<0x28 0x29>` from `pwm1/pinctrl-0` as pin numbers 40/41 and merges
> them with the node `audio_pins` (`brcm,pins = <0x28 0x29>;
> brcm,function = <0x04>`). Both are right in substance, but the chain of
> proof is not:
> * `pinctrl-0` holds **phandles**, not pin numbers. That they happen to
>   read `0x28`/`0x29` here as well is a coincidence of the compiler.
> * `audio_pins` (phandle `0x32`, l. 1193-1198) hangs **not** on the PWM
>   node but on the **`vchiq`** node (`mailbox@7e00b840`,
>   `compatible = "brcm,bcm2711-vchiq"`, `pinctrl-0 = <0x32>`,
>   l. 1877-1884). That is the path of the **VideoCore firmware**
>   (`snd_bcm2835`), which serves the jack in Linux operation — the same
>   pins, another instance. The kernel command line of our card
>   correspondingly contains `snd_bcm2835.enable_headphones=0` (l. 71).
>
> **The ticket's conclusion remains valid** — PWM1, GPIO 40/41, ALT0 —,
> and it now stands on the datasheet (§6.1) instead of on a mix-up. By
> the way: the identifier `brcm,function = <0x04>` = ALT0 fits the `FSEL`
> encoding `100` = 4 from §6.2.
> **Practically significant for us:** the firmware uses the same pins. We
> take them over as soon as we switch them. As long as we do not trigger
> a firmware audio service, there is no second candidate.

The device tree also names the two lines of the board itself:
`gpio-line-names` of the GPIO node (l. 187) carries at index 40/41
`"PWM0_MISO"` / `"PWM1_MOSI"` — the two functions these pins are
intended for on this board.

> **What this does NOT back:** that GPIO 40/41 physically go through a
> low-pass to tip and ring of the 3.5 mm socket. That is a statement
> about the circuit board; it is in neither the datasheet (which does not
> know the board) nor the device tree (which assigns functions, not
> nets). See §10.5.

### 6.4 Nothing else to enable

The board's firmware GPIO expander lists
`"BT_ON", "WL_ON", "PWR_LED_OFF", "GLOBAL_RESET", "VDD_SD_IO_SEL",
"CAM_GPIO", "SD_PWR_ON", "SD_OC_N"` (l. 1853) — **no audio or mute
line**. According to the device tree there is therefore nothing to
release besides the two pins and the PWM block.

---

## 7. Feeding the FIFO

### 7.1 The PWM block has no interrupt — evidence

This is the most important single statement of this note, hence the
evidence twice and by absence soundly argued:

1. **There is none in the block itself.** Chapter 8 has been read
   completely (§8.1 to §8.6, Tables 152–158). The register set has eight
   registers (§3); `CTL` contains no enable bit for any interrupt, `STA`
   calls its bits "flag"/"status" and is exclusively `RO`/`W1C`. An
   interrupt line is not mentioned anywhere in the whole chapter.
2. **It is not in the chip's interrupt table.** BCM2711 Rel. 4 §6.2.4,
   Table 102 lists all 64 VC peripheral interrupts (0 Timer 0 … 63
   ETH_PCIe secure). **`PWM` does not occur in it.** Neither does it in
   the breakdown of the OR-ed sources (Figure 6: `AUX_IRQ` bits and
   `PACTL_CS` bits 0–20 — SPI, I2C, UART; no PWM) nor in Table 101
   (ARMC) or Table 103 (ETH_PCIe).

**Consequence:** the question "can the FIFO be fed from an interrupt,
and which one is it?" has the answer **"not from a PWM interrupt — there
is none"**. §7.2 and §7.3 remain.

To place the GIC numbers needed further below (Rel. 4 §6.3, Figure 7):
`VC peripheral IRQs → SPI IDs 96-159`, i.e. **INTID = 96 + VC IRQ**. Our
kernel calculates in absolute INTIDs
(`kernel/src/drivers/gic.rs:260-263@PL011_INTID`: PL011 = 153 = 96 + 57,
and Table 102 lists VC IRQ 57 as "OR of all PL011 UART" — the
calculation holds).

### 7.2 Way 1 — refill from a foreign timer

**What exists:**

* **ARM generic timer, EL1 physical**, INTID **30**, today fixed at
  **200 Hz** as the system tick (`kernel/src/drivers/timer.rs`, module
  head and `TICK_HZ`). One compare register, one interrupt.
* **GIC-400**, with `enable_spi` and `enable_ppi`
  (`kernel/src/drivers/gic.rs:95@enable_spi`,
  `kernel/src/drivers/gic.rs:143@enable_ppi`) — the distribution is
  there.
* **Not present:** a second timer channel, a driver for the BCM system
  timer, and any DMA.

**What the datasheet offers in additional timers** — BCM2711 Rel. 4
§10.1/§10.2: the system timer at `0x7E00_3000` has "four 32-bit timer
channels and a single 64-bit free running counter"; `CLO`/`CHI` at
`0x04`/`0x08`, the compare registers `C0`…`C3` at `0x0c`…`0x18`, the
match flags `M0`…`M3` in `CS` (`W1C`). The four matches are VC IRQ 0…3
(Table 102), i.e. **INTID 96…99**.

The **count frequency is not named by the datasheet** ("The free running
counter is driven by the timer clock"). The device tree names it:
`timer@7e003000 { clock-frequency = <0xf4240>; }` = **1 000 000 Hz**
(l. 144-151); the same node carries
`interrupts = <0 64 4> <0 65 4> <0 66 4> <0 67 4>`, which with
`INTID = 32 + n` gives exactly 96…99 and thus confirms Table 102.
1 MHz and four independent comparators suffice easily for audio refill.

**MEASURED (real Pi, H4,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):** the system
timer runs at **1 MHz** — `CLO` advanced 100 003 in 100 002 µs (measured
against `CNTPCT`), confirming the device tree value. **The firmware uses
channels 0 and 2:** at kernel entry (`CNTPCT` 2 657 319) and at the
probe (`CNTPCT` 878 798 579) `C0` and `C2` hold values near `CLO` (at
entry `CLO 00742806`, `C0 00742f13`, `C2 00745963`); within 100 002 µs
`C0` went `016bd583` → `016d5c23` and `C2` `016bbc9a` → `016d47cf` — the
firmware **re-arms** `C0` and `C2` all the time; **`C1` and `C3` stay
at 0**, `SYST CS` 00000000. (The same probe in QEMU raspi4b: `C0`–`C3`
all 0, `CLO` advances 1 per µs — `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`.)
So `C1`/`C3` (INTID 97/99) are free on this board as found; which
channel the firmware uses and why is an observation, not a sourced
fact (§10.7).

> **Known trap, kept as a warning:** it is widespread lore that the
> VideoCore firmware uses two of the four system timer channels. **In no
> pinned source is this stated** — neither in the datasheet nor in the
> device tree. **MEASURED (real Pi, H4, above):** `C0` and `C2` are
> re-armed by the firmware, `C1` and `C3` are 0 — consistent with the
> lore. Whoever goes this way still checks at run time whether a channel
> already carries a compare register other than 0, instead of believing
> it (§10.7).

**The flow** would be: timer fires → read `STA` → as long as `FULL1` is
not set, write a word to `FIF1` → check and clear
`GAPO1|GAPO2|RERR1|WERR1` → next deadline. Bracketing with exactly
**one** barrier before and after the block per §1.3.

**Why that is expensive is in §8.**

### 7.3 Way 2 — DMA (documented, but we have no driver)

The datasheet describes the path completely.

**The DREQ.** BCM2711 Rel. 4 §4.2.1.3, assignment table:
**DREQ 1 = "DSI0 / PWM1"**, **DREQ 5 = "PWM0"**. The switching of DREQ 1
is in the same section: `PACTL_CS` (`0x7E20_4E00`) **bit 23
`DMA_CNTRL_MUX_2`** — "0 selects DSI0, 1 selects PWM1", **reset `0x1`**.
For PWM1 the preset is thus already the right one; the section also
admonishes "Other bits in this register should be treated as
reserved, and only written back with the previously read value."

**MEASURED (real Pi, H4,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):**
`PACTL_CS` reads `08800000` at kernel entry: **bit 23 is set (DREQ 1 =
PWM1), by the firmware**, as the reset value says. (`0x08800000` also
has bit 27 set; the datasheet text read for this note does not say what
bit 27 is, so it is not interpreted. Per the datasheet's own advice a
driver reads and writes back the other bits unchanged.)

> **Inconsistency in the document, named:** §8.5 words the same numbers
> as "PWM0 DMA is mapped to **DMA channel** 5. PWM1 DMA is mapped to
> **DMA channel** 1 (muxed with DSI0)." These are the **DREQ** numbers
> from §4.2.1.3, not channel numbers: §4.1 says expressly "The
> allocation of peripherals to DMA channels is programmable", and the
> selection is done through `TI.PERMAP` (§4.2, Table 40, bits 20:16:
> "Indicates the peripheral number (1-31) whose ready signal shall be
> used to control the rate of the transfers"). **Inference, not
> quotation:** §8.5 means DREQ; any channel can listen to PWM1 with
> `PERMAP = 1`.

**The channel.** §4.2: channel *n* is at `0x7E00_7000 + n·0x100`
(channel 15 separate at `0x7EE0_5000` and "exclusively used by the
VPU"). Register offsets within a channel (§4.2.1.2): `CS` `0x00`,
`CONBLK_AD` `0x04`, `TI` `0x08`, `SOURCE_AD` `0x0c`, `DEST_AD` `0x10`,
`TXFR_LEN` `0x14`, `STRIDE` `0x18`, `NEXTCONBK` `0x1c`, `DEBUG` `0x20`.
Global: `INT_STATUS` `0xfe0`, `ENABLE` `0xff0` (all engines enabled from
reset, `EN0`…`EN14` reset `0x1`; `PAGE`/`PAGELITE` reset 0).

**MEASURED (real Pi, H4):** `ENABLE` = `00007fff` (`EN` `7fff`, `PAGE` 0,
`PAGELITE` 0), `INT_STATUS` 0 — as the reset values say. `CS` per
channel as found: ch 0, 2, 4, 5, 7, 8, 9, 10 = `00000008`; ch 1, 3 =
`20000008`; ch 6 = `20000009`; ch 11 = `20000309`; ch 12, 13 =
`00000308`; ch 14 = `20000308`; `CONBLK_AD` 0 on all; `DEBUG` `040180n0`
for ch 0–6, `100187n0`…`10018a00` for ch 7–10, `1b000400`…`1e000400` for
ch 11–14. The probe's "free channel" test (`CS = CONBLK_AD = 0`, no
`DEBUG` error) finds **no** channel, because bit 3 is set on every `CS`
(mask `0000`); **the probe's free-channel mask is not usable as it
stands**, and these raw `CS` values are not interpreted here. (QEMU: all
15 channels `CS = CONBLK_AD = 0`, `ENABLE` `0000ffff`.)

**The control block.** §4.2.1.1: eight words, 256-bit aligned; word 0
`TI`, 1 `SOURCE_AD`, 2 `DEST_AD`, 3 `TXFR_LEN`, 4 `STRIDE`, 5
`NEXTCONBK`, 6–7 reserved = 0. Start: CB address into `CONBLK_AD`, then
`CS.ACTIVE`. At the end the channel loads `NEXTCONBK` and continues —
"In this way a linked list of Control Blocks can be constructed in order
to execute a sequence of DMA operations without software intervention"
(§4.1). With `NEXTCONBK = 0` it stops and clears `ACTIVE`.

**The bits this case needs** (§4.2, Table 40 `TI`): `DEST_DREQ` (bit 6)
— "The DREQ selected by `PERMAP` will gate the destination writes";
`SRC_INC` (bit 8); `PERMAP` (20:16) = 1 for PWM1; `INTEN` (bit 0) —
"Generate an interrupt when the transfer described by the current
Control Block completes". `DEST_INC` stays 0 because the destination is
the fixed `FIF1`.

**The interrupt.** `CS` bit 2 `INT` (`W1C`, §4.2 Table 38): "This is
set when the transfer for the CB ends and `INTEN` is set to 1. Once set
it must be manually cleared down, even if the next CB has `INTEN = 0`."
Channel *n* is VC IRQ 16 + *n* (Table 102: 16 "DMA 0" … 22 "DMA 6",
23 "DMA 7 & 8", 24 "DMA 9 & 10", 31 "DMA 15"), hence **INTID 112 + n**
for *n* = 0…6. The device tree confirms it independently: the DMA node
carries `interrupts = <0 80 4> …` with `interrupt-names = "dma0" …
"dma10"` (l. 1516-1524) — `32 + 80 = 112`, and `dma7`/`dma8` share
`<0 87 4>` (= 119 = 96+23), `dma9`/`dma10` share `<0 88 4>`
(= 120 = 96+24), exactly as Table 102.

**MEASURED (real Pi and QEMU, H5):** for channel 5 the interrupt line
reaches the GIC distributor: `GICD_ISPENDR` bit of **INTID 117**
(= 96 + 16 + 5) reads **1** with the SPI disabled and **1** with it
enabled, and **0** after `CS.INT` was cleared — identical on the real Pi
(`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`, H5, both
runs) and in QEMU raspi4b 11.1.2
(`measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`, RESULT 1; IRQs
masked, no interrupt taken; delivery through `GICC_IAR` to the CPU was
**not** exercised). This confirms the INTID 112 + *n* derivation for
*n* = 5.

**Which channels are free.** The device tree reserves:
`brcm,dma-channel-mask = <0x7f5>` (l. 1522) — a bit mask, hence
**channels 0, 2, 4, 5, 6, 7, 8, 9, 10 available; 1 and 3 not**.
Channels 7–10 are "DMA Lite" (§4.1: "Four of these are DMA Lite channels
(with reduced performance and features)"), 11–14 are DMA4 with a larger
address space. For an audio stream Lite is amply enough; whoever wants to
be sure takes one of the full channels 2, 4, 5 or 6. **MEASURED (real
Pi, H3):** the probe's memory-to-memory transfer ran on **channel 5**
and completed without error (below), so at least channel 5 is usable
from the kernel as found.

**The addresses the DMA sees.** §4.1: "the DMA controller must be
set-up to use the **Legacy Master addresses** of the peripherals" — the
destination is therefore `0x7E20_C818` (PWM1 `FIF1`), **not**
`0xFE20_C818`. And §1.2.4: "Software accessing RAM using the DMA engines
must use legacy addresses (between `0xC000_0000` and `0xFFFF_FFFF`).
This accesses a 1GB window within the full 16GB SDRAM address space."
With `ENABLE.PAGE` = 0 (reset) that is **the first GiB of main memory**.
A sound buffer must therefore either lie there, move the window, or use
a DMA4 channel. **That is a constraint for the memory manager**, not for
the sound driver — it belongs in the decision where `Setbuffer` buffers
come from.

**MEASURED (real Pi, H3,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`) — DMA
memory to memory, channel 5, 4096 bytes, `PERMAP` 0.** Bus addresses
`SRC c0200000`, `DST c01ff000`, `CB c01fe000` (bus = `0xC000_0000 | PA`,
§1.2.4 above). `CS` after the end: `1000000e` (`END` 1, `INT` 1,
`ERROR` 0), `INT_STATUS 00000020`, `TXFR_LEN` 0.

* **Run 1 (no cache maintenance):** `END` after 125 µs. The CPU reads
  `DST` as: 0 words new `SRC`, 0 old `SRC`, 1024 old `DST` (the CPU sees
  its stale cached `DST`). After `dc ivac` on `DST` (= what the DMA
  actually wrote): 624 words new `SRC`, 400 old `SRC`, 0 old `DST` — the
  DMA read `SRC` partly from RAM that the CPU's writes had not reached
  yet.
* **Run 2 (`dc cvac` on `SRC`; `dc civac` + `ivac` on `DST`):** `END`
  after 132 µs, `CS 1000000e`, `DST` 1024 of 1024 words = `SRC`.
* **Result:** on the real Pi the CPU caches are **not coherent with the
  DMA in either direction**: **clean before the DMA reads, invalidate
  before the CPU reads what the DMA wrote.** With both, the copy is
  exact. A 4096-byte copy takes 125–132 µs (the QEMU model: 109 µs and
  48 µs in two runs, **no cache model**, so QEMU gives 1024/1024 even
  without maintenance — H3's coherence question can be answered only on
  the Pi).
* After the run in QEMU, `SOURCE_AD`/`DEST_AD` = start + `0x1000` and
  `TXFR_LEN` 0 (the registers advance as the datasheet says;
  `measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`).

**What we lack:** `kernel/src/drivers/bcm2711/` contains `emmc`,
`framebuffer`, `mailbox`, `pl011` — **no DMA**. A DMA channel driver
(control-block management, cache maintenance for the CBs and the
buffer, bus/ARM address translation, GIC entry, error handling via
`CS.ERROR`/`DEBUG`) is **additional work** and not part of this note.
It would, however, be useful for more than audio. (The H3/H5 probe code
in `kernel/src/snd_probe.rs` is a debug-feature probe, not a driver.)

### 7.4 For completeness: PCM/I2S **does** have an interrupt

BCM2711 Rel. 4 chapter 7 describes the PCM/I2S block, and this one, in
contrast to PWM, has a full interrupt hookup: §7.9, Table 142 lists
`INTEN_A` (`0x18`) and `INTSTC_A` (`0x1c`); Table 149 `INTEN_A` bit 0
`TXW` — "Setting this bit enables interrupts from PCM block when TX FIFO
level is **less than the specified TXTHR level**" —, Table 150
`INTSTC_A` with `TXW`/`RXR`/`TXERR`/`RXERR` as `W1C`. The source is VC
IRQ **55** "PCM/I2S" (Table 102), hence INTID **151**.

Exactly what one would have wished for PWM. **Of no use to us here,
though:** PCM sits on GPIO 18–21 (§5.3: `PCM_CLK`, `PCM_FS`, `PCM_DIN`,
`PCM_DOUT` as ALT0) and presupposes an external converter. The Pi 4 B
has none on board, and PWM hangs at the jack, not PCM. The remark stands
here so that the question "is there any FIFO interrupt on this chip at
all?" is answered — yes, but at the wrong block.

---

## 8. What this costs the kernel

Everything that follows is **arithmetic** from §3.5 (FIFO 64 words,
shared) and §5.4 (one word per channel per period). No number of it
stands like this in the datasheet. (The 64-word depth is MEASURED, §3.5;
the arithmetic is unchanged.)

**Drain time of the full FIFO in stereo operation.** 64 words, two words
per stereo frame → **32 frames**. Hence
`t_empty = 32 / f_sample`.

| Rate | `t_empty` | Minimum visits/s (full→empty) | with safety factor 2 |
|---:|---:|---:|---:|
| 50 066 (STe /160) | 639 µs | 1 565 | 3 130 |
| **49 170 (Falcon 1)** | **651 µs** | **1 537** | **3 073** |
| 32 780 | 976 µs | 1 024 | 2 049 |
| 25 033 | 1.28 ms | 782 | 1 565 |
| 24 585 | 1.30 ms | 768 | 1 537 |
| 19 668 | 1.63 ms | 615 | 1 229 |
| 16 390 | 1.95 ms | 512 | 1 024 |
| 12 517 | 2.56 ms | 391 | 782 |
| 12 292 | 2.60 ms | 384 | 768 |
| 9 834 | 3.25 ms | 307 | 615 |
| 8 195 | 3.90 ms | 256 | 512 |
| 6 285 (STe 6 kHz) | 5.09 ms | 196 | 393 |

**What follows from it:**

* **Today's 200 Hz system tick is not enough.** Its period is 5 ms; the
  FIFO is empty after 0.65 ms at 49 kHz. Only the lowest STe rate
  (6 285 Hz, 5.09 ms) is within reach at all — and without any reserve.
  **Refilling in the 200 Hz tick is not a compromise but a permanent
  click.**
* **The timer path costs about 1 500–3 100 interrupts per second at
  49 kHz** — **8 to 16 times** today's tick, on a cooperative
  single-core kernel.
* **The write effort is independent of the visit rate:** per frame two
  words must go to `FIF1`, i.e. at 49 170 Hz **98 340 peripheral writes
  per second**, plus at least one `STA` read per visit. What a
  peripheral access costs on this board is stated by none of our sources
  (§10.2) — the number is thus a count, not a time.
* **The DMA path costs exactly one interrupt per control block.** With
  one CB per Falcon frame that is **one interrupt per frame** — with a
  32 KiB buffer and 49 kHz stereo/8 bit about **3 per second**. The
  refilling itself is done by the hardware; the CPU does not touch
  `FIF1` at all.

That is the actual finding of this section: **between the two ways lie
about three orders of magnitude of interrupt load.**

---

## 9. How this maps onto the spec's buffer contract

> **Everything in this section is mapping, hence derivation** — from
> `docs/api/xbios/_dmasound-group.md` §4 on one side and from §3/§7.3 of
> this note on the other. It is **not** a hardware statement and **not**
> a decision; the lead takes that.

| Spec contract (§4 there) | PWM + timer (§7.2) | PWM + DMA (§7.3) |
|---|---|---|
| Start address, **exclusive** end address | software pointer; the hardware knows no buffer | `SOURCE_AD` + `TXFR_LEN` in the CB; length = `end − start` |
| **Even** frame length | optional | `TXFR_LEN` in bytes, alignment freely selectable |
| `SB_PLA_ENA` (one frame, then stop) | software state | `NEXTCONBK = 0` → "The DMA will stop (and clear the `ACTIVE` bit) … when `NEXTCONBK` is set to `0x0000_0000`" (§4.2.1.1) |
| `SB_PLA_RPT` (endless loop) | software state | CB points to itself |
| End-of-frame **interrupt** | **none from the PWM** (§7.1); only "timer notices it is done" | `TI.INTEN` → `CS.INT` → INTID 112+*n*. **A real end-of-frame interrupt.** |
| `Buffptr` — running counter | software must count itself | `SOURCE_AD` is readable and kept up to date: "Source address for the DMA operation. **Updated by the DMA engine as the transfer progresses**" (§4.2, Table 41) |
| 24-bit address limit of the Falcon | not applicable (internals) | replaced by the 1 GiB window (§7.3) — a different limit, but a real one |

**The point that stands out here:** the DMA path delivers the two things
the spec marked as expensive or as a gap
(`_dmasound-group.md` §9.1 "The end-of-buffer interrupt — the biggest
gap", and §4.5 "Polling as a substitute"), **directly from documented
hardware**: an end-of-frame interrupt and an accompanying source address
counter. The timer path delivers neither without additional bookkeeping.
That is an argument that belongs on the table before the order decision
— the spec had initially noted `Buffptr` polling for NewTOS as the
"realistic first playback path"; this note shows that the DMA path is
technically the one closer to the contract, even if it is more work.

---

## 10. What is NOT backed

This list is the part of the note that distinguishes it from a pretty
note. Each line: what is missing, how bad, what one would do. Where an
item has since been measured (real Pi, 2026-11-12), the line says so and
keeps what is still open.

### 10.1 The register address of the PWM clock
`CM_PWMCTL`/`CM_PWMDIV` are **in no pinned source**. Of the clock
manager the datasheet documents only `GP0`…`GP2` (§5.2 here, Rel. 4
§5.4.2 there). **Effect:** we cannot program `clk_pwm` source-backed.
**What to do:** §5.2, way A — measure the clock found (§5.6) and choose
`RNG` accordingly. Only if that is not enough, way B or C, and then
recorded as a deliberate deviation.

**Update — MEASURED (real Pi, H1,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):** way B was
exercised: the firmware answers to clock id `0x0a`; `clk_pwm` is **off**
at boot; "on" + 100 MHz makes it run, at **107 143 019 Hz** (FIFO
timing) / 107 143 064 Hz (firmware measurement), i.e. **not** at the
requested rate (§5.2). So the *need* for the register is reduced: a
driver can start the clock through the mailbox and must then use the
measured rate. **Still open:** the clock-manager register itself
remains unsourced (the raw words at circle's `0xa0`/`0xa4` were read,
not interpreted); the interpretation "750 MHz / 7" is unsourced;
whether the mailbox clock-id `0x0a` and the effective 107.143 MHz are
stable across firmware versions and boards is not established (one
board, one firmware).

### 10.2 Time cost of a peripheral access
Neither the datasheet nor the device tree says how long a 32-bit write
to `0xFE20_C818` takes. The numbers in §8 are therefore **counts**, not
percentages of CPU load. **What to do:** measure on real hardware (N
writes against `CNTPCT_EL0`) as soon as the first driver exists. Until
then no statement "costs x % of the CPU". (**Still open** — the SND03
probe did not measure it; the only timing measured is the DMA copy in
§7.3.)

### 10.3 Direction of the `DMAC` thresholds
Rel. 4 §8.6, Table 155 says only "threshold level for DREQ/PANIC signal
going active" — not whether the threshold means the **fill level** or
the **free space**. **Effect:** only for the DMA path, and only for fine
tuning; the preset 7/7 is valid and is used unchanged by circle
(`references/circle/lib/sound/pwmsoundbasedevice.cpp:161-163`). **What to do:** start with the preset
and touch it only if `GAPO` occurs. (**Still open.** MEASURED: the
firmware leaves `DMAC` at the preset `00000707`, §3.3; no DMA-to-PWM
transfer was run by the probe, so no `GAPO` observation exists.)

### 10.4 `DIVF` denominator and the 25 MHz limit
Table 96 says `DIVF/1024`, Table 100 makes `DIVF` 12 bit wide (§5.2).
Likewise it is unclear whether the MASH warning "not exposed to
frequencies higher than 25MHz" holds for `clk_pwm` or only for the GPIO
clocks. **Effect:** none as long as `MASH = 0` is used — and because of
10.1 we do not program the clock for the time being anyway. (**Still
open**, unchanged by the measurement.)

### 10.5 The board between GPIO 40/41 and the socket
**Not backed:** that the two pins go through a low-pass to the 3.5 mm
socket; which cutoff frequency this filter has; whether the socket uses
the same wires additionally for composite video and what that means for
the level. The datasheet describes the chip, not the board; the device
tree describes functions, not nets. Backed is only that the firmware
switches exactly these two pins to ALT0 for its audio output (§6.3) and
that the board calls them `PWM0_MISO`/`PWM1_MOSI` (§6.3). **Effect:** the
choice between `MSEN = 0` and `MSEN = 1` (§4) and the question how much
the PWM carrier frequency shows through cannot be predicted. **What to
do:** procure the schematic datasheet of the Pi 4 B (Raspberry Pi
publishes a reduced schematic) and pin it in `references/README.md` —
or listen to both modes on real hardware.

**The hearing test is OPEN.** On the real Pi (2026-11-12) the probe
played T1 (both channels 440 Hz, 2000 ms), T2 (left only, GPIO 40,
440 Hz, 1500 ms), T3 (right only, GPIO 41, 880 Hz, 1500 ms) and T4 (both
440 Hz, 2000 ms, mark/space), all at `RNG 2233` = 47 981 Hz; each ended
"played, `STA 00000601` (`GAPO1` 0 `GAPO2` 0 `RERR1` 0)" — so PWM1 ran
through all four tones without a FIFO gap or error flag. **Nobody
listened:** the owner had no device with a 3.5 mm jack at hand. Owner:
"Ja halte den Hörtest offen" (yes, keep the hearing test open); the
probe kernel stays on the card as `C:\kerns03.img`
(`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`). That
audible output reaches the jack, which channel is left/right, and the
`MSEN` question are **not** established.

### 10.6 QEMU does not model the PWM block
Observation, cleanly named: `strings` over the installed
`qemu-system-aarch64` (**QEMU 11.0.2**, `/opt/homebrew/bin`) shows the
device names `bcm2835-dma`, `bcm2835-cprman`, `bcm2835-sys-timer`,
`bcm2835-i2s` and about forty more — **but no `bcm2835-pwm`**; the only
PWM models in the binary image are `aspeed.pwm`, `npcm7xx-pwm` and
`exynos4210.pwm`. **What it is:** a finding about device model names in
the binary image, **not** a source reading. **What it means:** the jack
path cannot be tested in QEMU `raspi4b`.

**MEASURED (QEMU raspi4b 11.1.2,
`measurements/measurement-2026-11-12-snd03/RUNINFO.TXT`, RESULT 1):**
how an access behaves there is now established — an access to PWM1
(`0xFE20C800`) ends in a **synchronous external abort** (`ESR=0x0000000096000010`,
`FAR=0xffff0000fe20c800`, `measurements/measurement-2026-11-12-snd03/serial/qemu-pwm-abort.log`); it
is neither RAZ/WI nor a silent hit into the void. An access to
`PACTL_CS` (`0xFE204E00`) **stalls the boot**: the boot stops at the
probe's first read of it with no further output (`measurements/measurement-2026-11-12-snd03/serial/qemu-pactl-stop.log`;
no exception report, because the exception vectors are not installed
yet at that point — **inference**). The probe therefore has a
`sndprobe=qemu` switch that leaves both out. (Note: the run used QEMU
11.1.2; the string finding above was made on 11.0.2.)
**What to do:** build the driver so that on a machine without PWM it
reports "not present" cleanly instead of hanging — the abort means the
absence cannot be detected by reading PWM1 itself; a platform check is
needed. The QEMU DMA model, in contrast, runs control blocks (§7.3) and
a DMA driver can have QEMU tests for control-block execution,
completion flags, progress registers and the interrupt line at the GIC.
The PWM side has to be tested on real hardware.

### 10.7 Allocation of the system timer channels
Whether the VideoCore firmware uses channels of the system timer is
stated in **no** pinned source (§7.2). **What to do:** check at run time, do not
believe it. **MEASURED (real Pi, H4, §7.2):** the firmware re-arms `C0`
and `C2` continuously; `C1` and `C3` stay 0; the timer runs at 1 MHz.
**Still open:** *why* and whether that holds on other firmware versions;
the run-time check stays advisable.

### 10.8 FIFO depth in PWM mode
The number 64 stands under the serialiser bullet (§3.5). That it is the
same in PWM mode was inference. **Effect:** low, because `FULL1` holds at
run time. **What to do:** tie the refill loop to `FULL1`, not to a
constant 64 — and note the measured depth on the first hardware run.
**Now MEASURED (real Pi, H2,
`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`):** **64
words** until `FULL1` (channels disabled), then `WERR1` on the next
write — see §3.5. The inference is confirmed for this board; tying the
loop to `FULL1` stays the rule.

### 10.9 The ARM ARM
It was not present when this note was written (§1); it is in `references/arm-arm/` now and the
DMA driver (SND05) cites it for cache maintenance. This note makes **no**
architecture statement about barrier semantics, cache maintenance or
memory attributes. The DMA path (§7.3) needs both; whoever builds it
needs the ARM ARM or has to rely on what the framebuffer/EMMC driver
has already established. **MEASURED (real Pi, H3, §7.3):** the
*empirical* rule — clean (`dc cvac`) the source before the DMA reads,
invalidate (`dc ivac`, or `civac` after the transfer's destination was
cleaned) the destination before the CPU reads what the DMA wrote —
makes a 4096-byte copy exact on this board, and without it the copy is
visibly wrong in both directions. This is a measured behaviour of the
combination board + kernel memory attributes as mapped, **not** an ARM
architecture statement; the barrier semantics and the memory-attribute
rules remain unsourced until the ARM ARM is procured. (**Still open.**)

---

## 11. Observations outside this note

1. **HDMI audio.** The BCM2711 ARM Peripherals **have no HDMI
   chapter** — the table of contents lists 13 chapters (Introduction,
   Auxiliaries, BSC, DMA, GPIO, Interrupts, PCM/I2S, PWM, SPI, System
   Timer, UART, Timer (ARM side), ARM Mailboxes), none of them HDMI. The
   register ranges that `TASK/SND01.md:70-82` quotes from the device tree
   are **named there, but documented nowhere with us**. Furthermore the
   DMA channel of the HDMI node is called `dma-names = "audio-rx"` —
   **rx**, not tx; and the card carries `snd_bcm2835.enable_hdmi` in its
   switches (l. 2383), i.e. the firmware path. The ticket's assessment
   that HDMI audio is "an ordinary driver against documented registers"
   is **not** backed with our pinned sources. Ticket of its own.
2. **A DMA driver is missing from the tree as a whole**, not just for
   audio — `kernel/src/drivers/bcm2711/` has emmc, framebuffer, mailbox,
   pl011. Whoever builds it for SND01 builds it for more.
3. **`references/README.md`, checklist:** the ARM ARM stands there as
   TODO. For the DMA path it is needed.
