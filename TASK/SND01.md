# SND01 — Sound: Falcon DMA sound as API, HDMI as output

Status: OPEN (2026-11-11) | Priority: P2

Migrated (owner batch 13, 2026-11-11), evidence: TASK/SND01.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id, owner 2026-11-11: stays OPEN (1.0 scope (owner 2026-10-23))

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P2** (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Treiber + API-Ebene — Anwenderwunsch 2026-09-28

Dependencies: none · Precondition for [SDL01](SDL01.md)

## Why

NewTOS has **no sound**. `Dosound`, `Giaccess`, `Offgibit`, `Ongibit` are
honest no-ops with the reason "no PSG" (`tos-api/src/xbios/system.rs:108-124@giaccess`).
So far that was no shortcoming, but it becomes one as soon as something beyond a
desktop is to be shown — and it is the precondition for an SDL backend.

The owner, 2026-09-28: this is "not such a big problem at all" (German: „gar nicht
sooo ein grosses Problem"), and **the sound should come out over HDMI; the jack is
fine too**.

## The two halves that are not connected

### A — the API side: which TOS sound do we replicate?

Our target profile is **EmuTOS Falcon030/TOS4**
([[emutos-target-profile-falcon-tos4]]). So the conforming choice is **not** PSG
emulation but the Falcon's **DMA sound**: `Devconnect` (0x8B), `Sndstatus` (0x8C),
**`Buffoper` (0x88 — the ticket said 0x89, wrong; 0x89 is `Dsptristate`)**,
`Setbuffer` (0x83), `Soundcmd` (0x82), `Buffptr` (0x8D), **`Setmode` (0x84 — at
first forgotten entirely in the ticket; EmuTOS calls the C function
`setsndmode`)**, `Dsptristate`/`Gpio` as far as sensible.

**To be clarified first, not assumed:** what EmuTOS implements of this group at
all and under which `CONF_WITH_*`, and which of it is on in the `make 512`
reference build (`docs/api/_emutos-config.md`). The PSG group stays a no-op; the
reasoning there then has to be made more precise ("no PSG, but DMA sound is
present").

### B — the hardware side: how does sound get out of a Pi 4?

**HDMI is the wish and the harder way.** There, sound does not go through a
simple peripheral register set but through the VideoCore; which route is open to
us (mailbox interface, the HDMI block directly, or not at all without firmware
cooperation) is **Track B research and still outstanding**. Primary sources per
`docs/SOURCE-ANALYSIS-PROTOCOL.md`: the BCM2711 datasheet, the Raspberry Pi
Foundation's firmware/mailbox documentation, the device tree of our boot card
(`bcm2711-rpi-4-b.dtb` — it already gave us the right answer once at REF02).
Linux' `vc4-hdmi` and circle are **cross-references, not authority**.

### Established from the firmware device tree (2026-09-28)

Read from `bcm2711-rpi-4-b.dtb` in the local card backup
directory sd-card-backup-20260618, subfolder boot-files (gitignored, not in the repo; read with
`dtc -I dtb -O dts`) — the same kind of source that gave the right answer at
REF02 where the datasheet was silent.

**Jack** (correction 2026-09-29, Track B): the original reading of the device
tree was wrong — `<0x28 0x29>` in `pwm1/pinctrl-0` are **phandles**, not pin
numbers, and `audio_pins` hangs on the **`vchiq`** node, i.e. on the VideoCore
firmware path, not on the PWM driver. **The conclusion stays correct** — PWM1 at
`0xFE20_C800`, GPIO 40/41 in ALT0 — but it now rests on the **datasheet** (rel. 4
§5.3, §8.5) instead of on the mix-up. Details in `docs/hardware/pwm-audio.md`.

**HDMI — and here the lead was wrong again, this time too optimistic:**

```
clock-names = "hdmi", "bvb", "audio", "cec";
dmas = <0x2e 0x41fa000a>;
dma-names = "audio-rx";
reg-names = "hdmi", "dvp", "phy", "rm", "packet", "metadata", "csc", "cec", "hd", "intr2";
```

The HDMI block has its **own audio clock and its own DMA channel**, and its
register ranges are named. So the sound is pushed in by DMA from the **ARM side**
— **not** through the firmware, as first suspected above. That is hereby
corrected.

**Retracted 2026-09-29 (Track B):** the **BCM2711 datasheet has no HDMI chapter
at all** (13 chapters, none of them HDMI). The statement "a normal driver against
documented registers" is **not established** with our pinned sources. What the
device tree shows (own audio clock, own DMA channel, named register ranges)
stays correct — it only says nothing about whether the registers are
**documented**. Also, the DMA name of the HDMI node is `audio-rx`, and the card
carries `snd_bcm2835.enable_hdmi` among its switches. **Own ticket, own
research.**

**The jack is still the first step:** PWM on GPIO 40/41. Explicitly wanted as a
fallback, and sensible as a first step — audible sound over the jack proves the
whole API side before the HDMI question is settled.

## Proposed order

1. **Clarify A** (source analysis): which XBIOS functions, which semantics, which
   buffer mechanism — spec before code, rule 5.
2. **Research B** (Track B): establish the HDMI route or establish that it is not
   viable. No code before the source stands — rule 6.
3. **Build the jack first**, because it proves the API side.
4. **HDMI afterwards**, with the output choice as a device parameter, not as a
   second driver.

## Acceptance

A program that plays a buffer through the XBIOS sound group is **audible** —
first on the jack, then over HDMI. No stub: as long as a function is not
implemented, it stays a documented "not present", not a silent success.

## Working rule

[Common rules](README.md) apply.

---

## Decision 2026-09-29 — deferred, with an improved starting point

The owner: *„gehen wir erstmal ohne Sound und gehen den Sound nach der Ofam an,
DOOM geht auch ohne Sound"* (English: let us go without sound for now and tackle
sound after the Ofam; DOOM works without sound too). The ticket stays **P3 after
the OFAM date**. [PORT06](PORT06.md) (Doom) therefore explicitly does **not**
depend on SND01.

**What carried the deferral, and which part of it turned out wrong.** The lead
named three obstacles as the reason. After the owner's objection — *„aber wir
haben doch Circle als Vorlage"* (English: but we do have circle as a template) —
two of them fell away:

| The lead's objection | State after reviewing `references/circle` @ `6177984e` |
|---|---|
| "There is no DMA driver" | True for **our** tree. But `kernel/src/drivers/gic.rs:95@enable_spi` has `enable_spi(intid)` — exactly what a DMA channel interrupt needs (circle: file dmachannel.cpp, line 387, in the circle tree's lib directory, not in references/; `ARM_IRQ_DMA0+channel`). DMA is in the **datasheet** ch. 4, interrupt sources in ch. 6. So primary source, circle only as a cross-check (protocol rank 6). **No obstacle, a work item.** |
| "The PWM clock registers are in no pinned source" | Stays correct (§7 of this file, route C). For **HDMI** it is moot: circle does not touch the clock on RPi4 at all — file hdmisoundbasedevice.cpp, line 234, in the circle tree's lib/sound directory, not in references/, `TODO? manage clocks for RASPPI >= 4`. |
| "QEMU does not model the PWM block" | **Unchanged, and the real price.** Every iteration is a hardware round. |

**The finding that changes the order.** A DMA driver is not sound work.
[HW05](HW05.md) — 89 KB/s write throughput — sits on the same driver; `emmc.rs`
polls today. Whoever builds DMA pays twice.

**Order after the OFAM** (does not replace the proposal above but puts one step
in front of it):

0. **DMA driver** against datasheet ch. 4 + ch. 6, `gic::enable_spi` as the
   existing hook-up, circle as a cross-check. Serves sound **and** HW05.
1. Jack (circle: `pwmsoundbasedevice.cpp`, 257 lines).
2. HDMI as [SND02](SND02.md) — **the bigger chunk**, 794 lines against 257, and
   HDMI0 only on RPi 4 (circle's own note, file hdmisoundbasedevice.h, line 41,
   in the circle tree's include/circle/sound directory, not in references/).

**License.** circle is GPLv3, this project GPLv2 like EmuTOS. Reading and
rebuilding is covered and provided for by the protocol as rank 6; verbatim
adoption would be a derived work. Register numbers remain facts.

## Idea 2026-10-23 — sound as a loadable driver (owner)

The owner proposes delivering sound as a **loadable driver**, as under MagiC. The
model is MacSound for MagiCMac: a driver that provides the Falcon XBIOS sound
functions and maps them onto the host's sound hardware. Programs there check via
cookies whether sound is present and do not need to know where it comes from. The
name "magxsound" was not unambiguous in the conversation; before the analysis,
clarify whether MacSound is meant or GSXB (Milan/Hades).

What speaks for it:

- Jack (SND01) and HDMI (SND02) would be two interchangeable drivers behind the
  same XBIOS interface.
- The kernel stays free of sound hardware code; without a driver it honestly
  reports "no sound" through the cookies.
- It is the first consumer for [MOD01](MOD01.md) (loadable modules after
  FreeMiNT). The same layer is needed later for network (NET02) and USB.

Open, to be clarified before any decision:

1. ~~Can an EL0 program take over the XBIOS trap with us?~~
   **Clarified 2026-10-23: no.** Programs run in EL0 (owner).
   `Setexc(46, …)` only writes into the vector shadow, which is not distributed
   for the CPU vectors `0..0x3F`
   (`tos-api/src/bios/vectors.rs:1-13@Setexc`, `tos-api/src/bios/vectors.rs:98-107@NSHARED`). A driver in the ZMAGXSND style
   from `C:\AUTO` would have no effect; on top of that come the missing MMIO
   access and no interrupts in EL0. So the sound driver runs in the kernel, as a
   module per MOD01 or built in with exchangeable outputs.
2. Which cookies and functions MacSound or GSXB create; establish from source or
   documentation first, do not assume.
3. What EmuTOS itself has of the sound group in the Falcon profile (half A
   above). If EmuTOS has nothing there, a loadable driver is exactly the pattern
   an EmuTOS Falcon user already knows.

### Finding 2026-10-23 — MagiC sources (th-otto/MagicMac, local `/Volumes/Daten/Development/MagicMac`, GPL-3)

The paths in this finding are relative to that local MagicMac checkout, outside
this repository.

- **MagiCMac itself has no sound.** The XBIOS table ends at function 95
  (file mac_bios.s of the MagicMac tree, lines 1032-1033, `DC.W 95`), the kernel
  creates `_SND = 0` ("no sound", mac_bios.s lines 2865-2868; likewise
  file macxbios.s in the magcmacx directory, lines 3179-3182). The Falcon functions 96-141 therefore
  only arrive with a driver loaded later, such as MacSound. Its source is **not**
  in the tree; how it hooks in and which cookies it sets thus remains open.
- **MagiC for the Atari** brings the Falcon sound and DSP functions 96-141 in the
  kernel (file magibios.s in the atari directory, lines 1627-1671, implementation file dsp.s in atari/modules,
  directly on the Falcon registers).
- **Bell and key click under AtariX/MagicMac X:** `Dosound`, bell and click go
  through a host call (`MacSysX_Dosound`) if the emulator offers it, otherwise
  through the old YM script (file hostbios.s in the magiclin directory, lines 2383-2415, 2553-2555).
- **License:** MagicMac is under GPL-3, NewTOS under GPL-2+. Reading as a
  comparison is unproblematic; taking code from it would make the result GPL-3.
  Clarify before any adoption.

### Finding 2026-10-23 — ZMAGXSND 0.96 (Didier Mequignon, 2002), source with the owner

Source: `~/Downloads/MAGXSND.097/` on the owner's machine (file ZMAGXSND.L in its SOURCES folder,
68k assembler, 5165 lines; ZMAGXSND.TXT in its DOC folder; the CPX source
MAGXSND.C in SOURCES/MAGXSND.CPX). Not in the repo. License per `README.TXT` only
"This program and sources are free" — no named license, so a template to read,
no code to adopt.

This is exactly the driver the owner meant ("magxsound"):

- **Scope:** the Falcon sound functions **XBIOS 128-141** (`Locksnd` through
  `Buffptr`; `Dsptristate` 137 not implemented, `Gpio` 138 only for clock
  selection). Correction to the finding above: 96-127 are the DSP functions, the
  sound is at 128-141. 8/16 bit mono/stereo, recording 16 bit stereo, tracks 1-4,
  level and gain per channel; GSXB/MilanBlaster extensions (free sample rate), no
  24/32 bit, no mixer, Motorola byte order only.
- **Hooking in:** AUTO program that hooks into trap #13 and trap #14 via `Setexc`
  with XBRA (ZMAGXSND.L lines 1817/1821 of the CR-converted version, labels
  `DET_BIOS`/`DET_XBIOS`).
- **Cookies:** `McSn` (pointer to `struct McSnCookie`: version, size,
  playback/record capability, DSP; comment at line 475ff.), `GSXB`, and `_SND`
  with bits 1, 2, 5 set (8-bit DMA, 16-bit DMA, extended); the PSG and DSP bits of
  an existing `_SND` are preserved (line 1075ff.).
- **Three exchangeable outputs in one driver:** MacOS sound under MagiCMac, YAMAHA
  or MV16/PSOUND on the Hades (timer A of MFP2), SDL under ARAnyM (no recording
  there). Exactly the pattern for jack/HDMI on the Pi.

**Consequence for NewTOS:** the outward interface = XBIOS 128-141 plus the
cookies `_SND`/`McSn`/`GSXB`, as ZMAGXSND sets them. The hook-in route via
`Setexc` on trap #14 from an AUTO program does not fit an EL0 program without
further ado (open question 1 above); a kernel module (MOD01) with output backends
remains the obvious choice.

---

## Spec review (researcher, 2026-11-12)

Doc-only. Occasion: owner 2026-11-12, "DMA sound is the most critical missing
piece". The argument "a DMA driver also fixes HW05" is void (HW05 OBSOLETE, owner
2026-11-12; multi-block SD writes already reach up to 9447 KB/s): a DMA driver now
serves sound only, and its cost is charged to sound alone. The ticket text above is
still German (translation of the whole ticket is a separate step; only this section
is English). Line 3 is untouched.

### 1. What was done and how

* `docs/api/xbios/_dmasound-group.md` re-validated against `references/emutos`
  (pin `1a7484ab`, read-only) and `references/freemint` (`f906416a`) per
  `docs/SOURCE-ANALYSIS-PROTOCOL.md` Track A, corrected, translated to English, and
  extended with §9.1 "End-of-buffer notification without MFP". Its header lists every
  correction.
* The BCM2711 facts were re-read against
  `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`. This host has no
  PDF tool; the text was extracted from the PDF's Flate streams with a throw-away
  script (scratchpad, not kept). The chapter-4 DMA registers (Tables 33-48, 61, 62),
  the DREQ table and `PACTL_CS` bit 23, the PWM chapter 8 (Tables 152-158) and the VC
  interrupt table 102 were read this session; cites in the spec name the table.
* QEMU observation (read-only, no guest run): `qemu-system-aarch64 --version` is now
  **11.1.2** (the Track B note recorded 11.0.2, by `strings`).
  `qemu-system-aarch64 -M raspi4b -S -display none -monitor stdio` with `info mtree`
  lists `bcm2835-dma` at `0xFE007000` and `bcm2835-dma-chan15` at `0xFEE05000` with
  priority 0 (the unimplemented-device stubs such as `bcm2835-i2s`, `-smi`, `-v3d`
  show priority -1000), `bcm2835-sys-timer` at `0xFE003000`, `bcm2835-cprman` at
  `0xFE101000`, and **no region at `0xFE20C000`/`0xFE20C800`** (PWM). So the earlier
  statement "QEMU does not model PWM" (`docs/hardware/pwm-audio.md` §10.6) is now an
  mtree observation for 11.1.2, not a `strings` finding. What a guest access to the
  unmapped PWM address does (RAZ/WI or abort) is still **not established**; that the
  priority-0 DMA region is a functional model rather than a stub is an inference from
  the priorities.

### 2. Corrections to the spec and to the record

1. **Spec §4.7 was wrong** (corrected): the "odd in-between prescale values are
   invalid for CODEC" claim is backwards. FreeMiNT marks **6, 8, 10, 12-15** invalid
   and 5, 7, 9, 11 valid (`references/freemint/sys/mint/falcon.h:309-324`).
   `PRE160/320/640` are values 3/2/1, not "bits 3/2/1".
2. **Spec §1.4/§4.4/§9.1 were outdated** (corrected): `Mfpint`/`Xbtimer` are wired
   since TASK/BIOS15 as warn-only recorders (`tos-api/src/xbios/mod.rs:140@MFPINT`),
   `Jdisint`/`Jenabint` are not. The lower-EL IRQ vector now reaches
   `rust_irq_handler` (`kernel/src/arch/aarch64/exception_vectors.s:115@__vec_irq`),
   but that handler never delivers a pending upcall
   (`kernel/src/drivers/gic.rs:203@rust_irq_handler`).
3. **`docs/hardware/pwm-audio.md` §9 mapping table is wrong as written** (not edited
   by this review; scope): it maps `Setbuffer` start/end straight onto `SOURCE_AD` /
   `TXFR_LEN`. A PWM FIFO word is one duty-cycle value in `0…RNG` (datasheet
   Table 157/158), so the DMA source must be a CPU-converted staging buffer of 32-bit
   words (4 bytes per sample per channel). `Buffptr` then needs the staging position
   mapped back to the user buffer. The same note's §7.3 omits that DMA **Lite**
   channels have a 16-bit `TXFR_LEN` (at most 65 535 bytes per control block,
   Table 48); full channels 0-6 have 30 bits (Table 43).
4. Stale line cites in the spec fixed (`docs/COMPATIBILITY.md:1960`,
   `references/README.md:29-30`, `docs/api/_emutos-config.md:565,605`,
   `bios/machine.c:709-719`, `bios/bios.c:442-445`).
5. Source conflict recorded, not resolved: `Sndstatus(1)` — FreeMiNT says interrupts
   and buffer operation are disabled by a reset (`references/freemint/sys/mint/falcon.h:328-333`), EmuTOS' register
   list does not name `$FF8900/01` (`bios/dmasound.c:735-750`). Needs the oracle.

All other entries of the 2026-09-29 spec were re-read and stand (function behaviour
`:215-803`, register image, gating, tree sweep for callers, `_SND` bit table and the
comment-against-code case).

### 3. The jack: "PWM FIFO fed by IRQ, no DMA" (P) against "DMA controller driver" (D)

Both paths need the same front half: GPIO 40/41 in ALT0, PWM `CTL`/`RNG`, a clock
(`clk_pwm` has **no** register in the pinned datasheet, `pwm-audio.md` §10.1), the
CPU conversion from Falcon PCM to PWM words, the XBIOS state machine with its
shadows (`Setbuffer`/`Buffoper`/`Buffptr`/`Soundcmd`/`Devconnect`/`Sndstatus`), the
cookie, and some delivery path for frame-end handlers. Only the transport differs.
Variants of D: **D-tick** (cyclic control-block ring over a staging ring, refilled
from the existing 200 Hz tick, frame ends found by polling) and **D-irq** (the same
ring, `TI.INTEN` per chunk, refill in the ISR) — spec §9.1.3 options B and A.

| | P: CPU feeds the FIFO from a timer IRQ | D: DMA channel feeds the FIFO |
|---|---|---|
| **New kernel work, beyond the common part** | a timer source other than the 200 Hz tick (reprogram the generic timer, or the BCM system timer, INTID 96-99; firmware use of its channels is not in any pinned source) with a branch in `rust_irq_handler`; ISR that converts and writes until `FULL1`; frame/loop/pointer bookkeeping in software | DMA channel driver: 32-byte-aligned control blocks, `ENABLE`/`CS.RESET`/`CONBLK_AD`/`ACTIVE`, `PERMAP = 1`, `DEST_DREQ`, `SRC_INC`, keep `PACTL_CS` bit 23, `DMAC.ENAB`; staging ring; bus-address conversion and the 1 GiB legacy window (datasheet §4.1/Table 62); cache maintenance; D-irq additionally a GIC entry (`gic.rs:95@enable_spi`) and ISR |
| **Interrupts per second at 49 170 Hz stereo** | at least 1 537, with margin about 3 073, and every one must be serviced within the FIFO drain time (arithmetic, `pwm-audio.md` §8) | D-tick: 0 new (the existing 200); D-irq: `98 340 words/s / chunk words`, e.g. 24/s for 4096-word chunks |
| **Peripheral accesses** | about 98 340 writes to `FIF1` per second plus at least one `STA` read per visit | none to the FIFO; one control-block start, optional progress reads |
| **CPU conversion** | in the ISR, 98 340 conversions per second | the same count, outside the ISR (refill), plus 8 bytes of staging written and read per stereo frame |
| **Slack before an underrun click** | the FIFO drain time: 32 stereo frames = 651 µs at 49 170 Hz (depth 64 is inferred for PWM mode, `pwm-audio.md` §3.5) | the ring: N frames of staging = `N / rate`, a design choice (2048 frames = 41.6 ms at 49 170 Hz, arithmetic) |
| **Exposure to IRQ-masked kernel time** | high. Every trap runs with IRQs masked (`kernel/src/vbl.rs` module docs) and `emmc.rs` polls; any such stretch over 0.65 ms is an audible gap. Not measured, a consequence of the two facts | low, up to the ring slack |
| **Frame-end / `Buffptr` fidelity** | software only; pointer to FIFO granularity (inference) | D-irq: block completion (`CS.INT`, sticky, merges two ends between ISR runs) and `SOURCE_AD` mapped back; D-tick: polled, up to 5 ms late |
| **Testability in QEMU `raspi4b` 11.1.2** | none for the output (no PWM region); the ISR logic is host-testable against a fake FIFO | PWM output: none. The DMA model is registered (priority 0): a memory-to-memory test with `PERMAP = 0` ("continuous un-paced", Table 40) might exercise control-block chaining and the completion interrupt; **not established** that the model supports it or routes its IRQ to the GIC |
| **Testability on host** | conversion, bookkeeping, state machine | conversion, ring and pointer arithmetic, control-block layout, state machine |
| **Testability on the Pi** | audible plus an objective check: `STA.GAPO`, `RERR1`, `WERR1` are readable and can be printed to serial; the FIFO depth can be counted by writing until `FULL1` with channels disabled ("all registers and FIFOs are writeable without setting `PWENi`", Table 153 text) | the same checks plus `CS.ERROR`/`DEBUG` |
| **Sources** | datasheet ch. 8 and 6 (read); **no reference implementation** (circle's PWM sound is DMA-only: `references/circle/lib/sound/dmasoundbuffers.cpp:92-96`, tertiary, GPLv3, read not copied) | datasheet ch. 4 (read) and ch. 8; circle `references/circle/lib/sound/pwmsoundbasedevice.cpp` (257 lines), `references/circle/lib/sound/dmasoundbuffers.cpp` (423), `references/circle/lib/dmachannel.cpp` (566) as cross-check; device tree: channel mask `0x7f5` (channels 0, 2, 4-10 usable), INTIDs per `pwm-audio.md` §7.3 |
| **Unknown to both** | `clk_pwm` register; cache coherency (no ARM ARM in `references/`) matters for D only | same |

**Recommendation for the jack: D, in two steps.** (1) D-tick, because it needs the
least new kernel plumbing of all variants: no new interrupt source and no GIC
change, where P needs a second timer source and an ISR at 8-16 times the tick rate
on a cooperative kernel whose traps mask IRQs; (2) D-irq, per-chunk `INTEN`, when
the frame-end handler delivery is built. P has no reference implementation, the
narrowest slack, and its only advantage (no DMA driver) is the one the owner has
just withdrawn as a shared benefit. Step 0 before either: a small hardware probe
that settles the open facts on the Pi (H1-H4 below), because three of them are not
in any pinned source.

The DMA driver is not free: cache maintenance and the legacy-address window are
open (no ARM ARM), and the first hardware round may stay silent. P would hit the
same clock gap.

### 4. Proposed measurements (new tickets, none created here)

Oracle (falcontc profile, tosdump, `docs/analysis/ORACLE.md`; today only the
inquiries are measured, `tosdump/cases/bios-r1.tsv:230-234@Soundcmd`):

* O1 `Locksnd`/`Unlocksnd` return pairs; boot state through the inquiry forms
  (`Soundcmd(m,-1)` for modes 0-5, `Buffoper(-1)`, `Setmode`, `Devconnect` return).
* O2 `Setbuffer` + `Buffoper(1)` + poll `Buffoper(-1)`: does bit 0 drop at the end
  of a single-shot frame (spec S3)? After how long?
* O3 `Buffptr` while playing, at the end and after it: range, granularity,
  `res1`/`res2`/`record` untouched (S4).
* O4 `Buffoper(1)`, `Sndstatus(1)`, `Buffoper(-1)` and the interrupt register's
  effect (the FreeMiNT/EmuTOS conflict).
* O5 `Setinterrupt(0,1)` with `Xbtimer(0,8,1,h)` + `Jenabint(13)` and an m68k handler
  stub that counts: once per frame, once per loop pass (S2)? Needs a handler stub in
  tosdump.
  The oracle is EmuTOS in Hatari; Hatari's DMA-sound model is the reference here, not
  real silicon.

Hardware (Track B, real Pi 4):

* H1 effective `clk_pwm`: time the FIFO drain at a known `RNG`.
* H2 FIFO depth in PWM mode: count writes to `FIF1` until `FULL1`, channels disabled.
* H3 cache behaviour of a CPU-written staging buffer read by a DMA channel, with and
  without `dc cvac` (mailbox precedent: `kernel/src/drivers/bcm2711/mailbox.rs:55@clean_dcache`).
* H4 system-timer compare registers at boot, to see whether the firmware uses channels.
* H5 QEMU: does the modelled DMA do memory-to-memory with `PERMAP = 0` and does its
  interrupt reach the GIC (decides how much of a DMA driver CI can test).

### 5. Draft for the owner — ADR-012 amendment for `_SND`

`docs/MEMORY.md` ADR-012 (Consequences, "drop `_VDO`/`_SND`/`_FDC`/`_FPU`") and its
amendment 2026-10-28 keep `_SND` absent; `docs/COMPATIBILITY.md:1636` row 49 records
`absent` against the oracle's `0x03` (TT) / `0x1F` (falcontc). This draft does **not**
touch `docs/MEMORY.md`; the owner decides.

What EmuTOS sets (`references/emutos/bios/machine.c:409-447`): `0x01` PSG if the
build has a YM2149; `0x02` if `has_dmasound`; `0x04|0x10` if `has_falcon_dmasound`;
`0x08` if a DSP; `0x20` never. What consumers conclude, as read in the ecosystem
(spec §6): `0x02` — "STe DMA register image exists", then they poke `$FFFF8900`
and Microwire (FreeMiNT `sys/xdd/audio/dma.c:41`, `lmc.c:44`); any of
`0x04|0x08|0x10` — "Falcon XBIOS sound", then `Soundcmd`/`Devconnect`/`Setbuffer`
(`falcon.c:47`); `0x08` — "DSP present" (`dsp56k.c:625`); `0x01` — "PSG present";
`0x20` — extended (GSXB/Milan) XBIOS calls such as `NSetinterrupt` with a handler
argument (`sdk/libc/overlay/include/mint/falcon.h:179@SND_EXT`).

> *Amendment (draft, date of the owner decision).* **`_SND` exists exactly when a
> sound backend has initialised** (the jack's PWM output, later HDMI); while none has,
> the cookie stays absent and XBIOS 0x80-0x8D answer their function number
> (`SOUND_IS_AVAILABLE` of `references/emutos/bios/dmasound.c:122-126`, spec §1.3).
> Value: **`SND_16BIT | SND_MATRIX` = `0x00000014`**, set by the sound driver through
> `Setcookie` after its probe, not in `seed_defaults`
> (`tos-api/src/cookie.rs:223@seed_defaults`). Bits not set and why: `SND_PSG`
> (0x01) — no YM2149, `Dosound`/`Giaccess` stay plausible-zero no-ops;
> `SND_8BIT` (0x02) — programs read it as "the STe register image at `$FFFF8900`
> exists", NewTOS has no such image and does not emulate one (ADR-012, spec §8), and
> they would fault; `SND_DSP` (0x08) — no DSP, 0x60-0x7F answer function numbers
> (spec §9.5); `SND_XBIOS`/`SND_EXT` (0x20) — the GSXB extensions (`NSoundcmd`,
> `NSetinterrupt`, free sample rates, 24/32-bit) are not implemented. A program that
> reads `0x14` concludes: Falcon CODEC and connection-matrix XBIOS sound is present
> (`Devconnect`, `Soundcmd`, `Setmode` incl. 16-bit stereo, `Setbuffer`, `Buffoper`,
> `Buffptr`), no PSG, no DSP, no STe register image. It may attempt recording (no bit
> says "no input"): `Setbuffer(SR_RECORD, …)` and the record bits answer `EBADRQ`
> (spec §9.4). `SND_16BIT` states API capability, not DAC resolution (the PWM jack
> resolves about 10-11 bit at 49 kHz, `pwm-audio.md` §5.4). The ADR-012 consequence
> "drop `_SND`" is superseded for a system with a sound backend. `docs/COMPATIBILITY.md`
> row 49 changes from `absent` to `0x14` with a sound backend, still ACCEPTED against
> the oracle (`0x03`/`0x1F`: no PSG, no 8-bit, no DSP).

Variant for the owner to choose against: **`0x16`** (add `SND_8BIT`), which a Falcon
also reports and which some programs use as their only "DMA sound present" test; it
makes STe-style programs poke registers that do not exist. The draft recommends
`0x14`. `SND_EXT` is not offered until GSXB is built.

### 6. Owner questions

* **Q-N1 Path.** DMA transport (D-tick first, D-irq second) as recommended in §3, or
  the FIFO-by-IRQ path P?
* **Q-N2 Frame-end handlers.** Serve `Setinterrupt` with a program handler (a
  C-callable function through `Xbtimer(0,…)`/`Mfpint(13|7,…)`, upcall delivery), or
  keep it "recorded, never called, warning" (spec option D)? If served: delivery at
  the next syscall return (U1, no new kernel work) or at the IRQ return (U2, needs
  the open §7.3 of `docs/analysis/coop-kernel/10-upcall-signal-design.md`)?
  Serving it amends ADR-031's table (row for `Mfpint`/`Xbtimer`: served for vectors
  13 and 7 as sound sources only).
* **Q-N3 Pointer width.** Native layout for `SndBufPtr` (32 bytes under LP64, as the
  SDK header defines it, `docs/COMPATIBILITY.md:1960`) and 64-bit `Setbuffer`
  addresses (as the SDK macros already pass them)? And the policy for a `Buffptr`
  pointer that is null or not writable (EmuTOS writes to it; the kernel cannot).
* **Q-N4 16-bit byte order.** Big-endian as on the Falcon (the SDK header says so for
  the extended modes), little-endian as the native program writes, or selectable by a
  NewTOS flag? A byte swap per sample is free on the PWM path.
* **Q-N5 `_SND`.** `0x14` (recommended), `0x16`, or other; created at backend init
  (draft in §5).
* **Q-N6 Rates.** Hit the Falcon prescale rates exactly or resample, and which
  prescales to accept (valid: 1-5, 7, 9, 11; spec §4.7)? Depends on H1.
* **Q-N7 Measurements.** Approve tickets for O1-O5 (oracle) and H1-H5 (hardware).
* **Q-N8 HDMI** (SND02) is unaffected by this review. One observation for it: the
  datasheet's DREQ table lists DREQ 10 = HDMI0 (§4.2.1.3), which matches the `0x0a`
  in the device tree's HDMI `dmas`; the HDMI registers themselves are still in no
  pinned source.

### 7. Not done / out of scope

* The German text of this ticket is not translated (separate step).
* `docs/hardware/pwm-audio.md` is not edited (the two corrections in §2 point 3 are
  recorded here and in spec §9.1.4); `docs/MEMORY.md` is not edited; no code; nothing
  committed or staged.
* Observed, not acted on: ADR-031's table row for `bell_hook`/`kcl_hook` says "no
  sound device yet" — it changes once a backend exists (bell and key click through
  the sound device).

## Cross-reference from the owner (2026-11-12): iosoft DMA article

The owner pointed to https://iosoft.blog/2020/05/25/raspberry-pi-dma-programming/ (Linux user space, `/dev/mem`;
read by the lead 2026-11-12). Not a source in the Track B hierarchy (`docs/SOURCE-ANALYSIS-PROTOCOL.md`, below rank
6): it gives hypotheses for the hardware probe (H1-H4), no facts for the code.

- PWM paced by DREQ, a looping control-block chain writing `PWM_FIF1` with `PWM_DMAC` enabled (threshold 1): the
  pattern of the recommended ring (step 1).
- Clock manager for the PWM: `CLK_PWM_CTL`/`CLK_PWM_DIV` with a password, stop (bit 5), wait for busy (bit 7), set
  the divisor, start with source 6 and enable (bit 4). Addresses and password value are not given there; on the Pi 4
  the article uses a 375 MHz base for source 6 (250 MHz on Pi 0-3). Changing the clock while the PWM runs can lock it
  up. This is exactly the gap of `docs/hardware/pwm-audio.md` §5.2 (no clock-manager register in the datasheet):
  H1 measures it, the article says what to expect.
- DMA channels on the Pi 4: the article recommends 2, 11-14 (others are used by the firmware); a fact for H4 to check
  against the firmware's channel mask, not to assume.
- Cache coherency: the article needs uncached VideoCore-mailbox memory because Linux caches user memory. Bare metal,
  NewTOS maps the staging ring and control blocks itself (non-cacheable attribute or explicit cache maintenance,
  ARM ARM); H3 decides which.

## Owner decision 2026-11-12: the jack first

Owner: "ok dann klinke als erster ausgang" (ok, then the jack as the first output). The PWM jack (GPIO 40/41,
`docs/hardware/pwm-audio.md`) is the first output backend; HDMI ([SND02](SND02.md)) follows behind the same XBIOS
interface and DMA ring. HDMI audio has no chapter in the BCM2711 datasheet, so SND02 needs an owner exception to
CLAUDE.md rule 6 (circle and Linux `vc4-hdmi` as the template, every register confirmed on the Pi); not decided yet.
Still open: Q-N1 to Q-N7 of the spec review above (path, `Setinterrupt`, sample byte order, rates, `_SND`, release
of the probes).

## Decisions 2026-11-12 (owner and lead)

Owner: "Ich würde den DMA weg gehen wollen, rest überlasse ich dir" (I want to take the DMA path, the rest I leave
to you). The lead decides the rest:

- **Q-N1 (owner):** DMA transport, D-tick first (control-block ring over a staging ring, refilled in the 200 Hz tick),
  D-irq second (per-chunk `INTEN`).
- **Q-N2 (lead):** `Setinterrupt` stays "recorded, never called, warning" (spec option D) for the first version, as
  `Mfpint`/`Xbtimer` today; serving program handlers is a later ticket with the ADR-031 amendment.
- **Q-N3 (lead):** native `SndBufPtr` layout and 64-bit `Setbuffer` addresses as the SDK header defines them
  (`docs/COMPATIBILITY.md:1960`); a null or unwritable `Buffptr` pointer ends the program through the existing
  crash path, like every other unwritable pointer (BIOS06, `tos-api/src/uaccess.rs`).
- **Q-N4 (owner, revised 2026-11-12):** big-endian 16-bit samples as on the Falcon by default (sample files and
  unchanged Atari programs work as they are), plus a NewTOS switch for native little-endian samples, so a ported
  program that computes or mixes samples in C writes them without swapping each one. Owner: "ja" to that proposal.
  The PWM FIFO takes duty-cycle words, not PCM, so every sample is converted anyway; either order costs the same.
  The form of the switch (a `Setmode` bit, a `Soundcmd` mode, or an existing convention such as GSXB) is settled in
  the spec before SND05 builds it: look for an existing convention first, invent bits only if there is none.
- **Q-N5 (owner, 2026-11-12, after H1):** `_SND` = `0x14` as drafted in §5 (owner: "ok machen wir so", ok, we do it
  that way). The H1 result does not change the draft. The ADR-012 amendment text of §5 goes into `docs/MEMORY.md` with
  [SND05](SND05.md), in the commit that creates the cookie.
- **Q-N6 (owner, 2026-11-12, after H1):** no resampling; every rate is set directly through the PWM range register,
  computed from the measured clock, never from the requested one. H1 on the real Pi: `clk_pwm` 107 143 019 Hz
  (`measurements/measurement-2026-11-12-snd03-pi/RUNINFO.TXT`). The rates are those the spec maps the prescale values
  to (EmuTOS maps Falcon prescales to the four STe rates, `docs/api/xbios/_dmasound-group.md` §4.7); integer RNG
  hits each within 0.003 % (6285 Hz: RNG 17047; 12517: 8560; 25033: 4280; 50066: 2140; lead's arithmetic; the
  rates as FreeMiNT lists them, spec §4.7; corrected 2026-11-12, the first version wrote 6258 Hz / RNG 17121 from
  memory, not from the spec).
  Same owner answer.
- **Q-N7 (lead):** approved: [SND03](SND03.md) (hardware probe H1-H5), [SND04](SND04.md) (oracle cases O1-O5),
  [SND05](SND05.md) (implementation: DMA ring, PWM jack backend, XBIOS 0x80-0x8D), after SND03 and SND04.
