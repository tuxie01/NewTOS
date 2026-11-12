# XBIOS DMA sound group — 0x80…0x8D

**Date:** 2026-09-29, re-validated and translated 2026-11-12 · **Primary source:**
`references/emutos` @ `1a7484aba1bfba23e2c0899e243b6d8177fe41df` (pin) ·
**Secondary source (for what EmuTOS programs but never executes):**
`references/freemint` @ `f906416a9387ed68876c738cfc5c30037b294fc0` ·
**Target profile:** `make 512`, Falcon030/TOS4 · **Occasion:** `TASK/SND01.md`,
step 1 (source analysis, no code).

Form template: `docs/api/xbios/_psg-group.md` (group spec) and
`docs/api/xbios/2c-bconmap.md` (full contract). Citation rule as there: no C
body copied, constants/register tables as data, every statement with
`file:line`.

> This file closes the sub-function level that
> `docs/api/xbios/_full-function-inventory.md` (header and §2.9) left open for
> the range **0x80–0x8D**. The DSP group 0x60–0x7F stays open.

> **Re-validation 2026-11-12 (researcher).** Every EmuTOS and FreeMiNT cite was
> re-read against the pinned trees; the BCM2711 facts in §9.1 were re-read
> against `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf` (text
> extracted from the PDF streams in this session). Corrections to the 2026-09-29
> text, each of them marked "(corrected 2026-11-12)" where it stands:
>
> 1. **§4.7 — wrong claim.** The text said the *odd* in-between Falcon prescale
>    values are "invalid for CODEC". The source says the opposite: **6, 8, 10 and
>    12–15 are invalid; 5, 7, 9, 11 are valid** (`references/freemint/sys/mint/falcon.h:309-324`).
>    Also `PRE160/320/640` are the **values** 3/2/1 of `mode_control` bits 0-1, not
>    "bits 3/2/1".
> 2. **§1.4, §4.4, §9.1 — NewTOS state outdated.** `Mfpint` (0x0D) and `Xbtimer`
>    (0x1F) are wired since TASK/BIOS15 (`tos-api/src/xbios/mod.rs:140@MFPINT`):
>    they record the handler, warn once per process (ADR-031) and install
>    nothing. `Jdisint`/`Jenabint` (0x1A/0x1B) are still unwired. The old text said
>    none of the four was wired.
> 3. **Stale line cites** (content unchanged, line moved): `bios/machine.c`
>    `_SND` comment is `:709-716` and the `_SND` block `:709-719` (was `:707-716`,
>    `:707-719`); `dmasound_init()` call is `bios/bios.c:442-445` (was `:440-444`);
>    `docs/api/_emutos-config.md` rows are `:565`, `:605` (was `:564`, `:604`);
>    "Native vs m68k struct variants" is `docs/COMPATIBILITY.md:1960` (was
>    `:623-637`); `references/README.md` lists Compendium/Profibuch at `:29-30`
>    (was `:24-25`) and the tree now also holds `xgdos`, `tos4x`, `magicmac`,
>    `fvdi`, `usb`; NewTOS code cites re-anchored (`tos-api/src/xbios/mod.rs`,
>    `system.rs`, `trap.rs`, `cookie.rs` moved).
> 4. **Additions from source** (not corrections): `Gpio` mode 2 writes `data`
>    **unmasked** (§3/0x8A); `Buffptr` reads three counter bytes non-atomically
>    (§3/0x8D, inference); FreeMiNT documents that `Sndstatus(1)` also disables
>    interrupts and buffer operation, EmuTOS' register list does not (§3/0x8C,
>    source conflict); the SDK's `falcon.h` overlay confirms the GSXB extension
>    signatures and the `SND_EXT` gate (§10.5, §6); new §9.1 "End-of-buffer
>    notification without MFP".
> 5. **Wording:** "measured frequencies of the ST path" (§4.7) is "listed" — the
>    FreeMiNT driver contains the numbers, it does not measure them.

---

## 0. The result up front

1. **EmuTOS implements all 14 functions 0x80–0x8D completely**
   (`bios/dmasound.c:215-803`) — this is not a stub group.
2. **EmuTOS never plays a buffer itself.** The only EmuTOS-internal callers of
   the group are `falcon_dmasound_init()` (`bios/dmasound.c:182-198`, calls
   `devconnect`/`setsndmode`/`soundcmd`) and the XBIOS vector table
   (`bios/xbios.c:1500-1513`) with its debug wrappers (`bios/xbios.c:1171-1240`).
   `setbuffer`, `buffoper`, `buffptr`, `setinterrupt`, `settracks`,
   `setmontracks`, `locksnd`, `unlocksnd`, `dsptristate`, `gpio`, `sndstatus`
   have **no** EmuTOS-internal caller (tree sweep over all `.c`/`.h`/`.S`,
   re-run 2026-11-12, see §10.1). So the **playback path itself is not
   observable in EmuTOS** — EmuTOS writes registers it never uses. For the buffer
   semantics (§4) FreeMiNT is therefore the load-bearing source, as protocol
   Track A point 2b permits.
3. In the `make 512` reference build **`CONF_WITH_DMASOUND = 1` and
   `CONF_WITH_XBIOS_SOUND = 1`** (`docs/api/_emutos-config.md:565,605`). Both are
   run-time gated; without the chip everything is off (§1).
4. **NewTOS already behaves conformantly today** — every call 0x80…0x8D returns
   its function number, exactly as EmuTOS on a machine without a DMA sound chip
   (§1.4). Building sound means leaving that conformant answer on purpose.
5. The expensive part is **not** the API but the **end-of-buffer interrupt**: it
   runs through MFP Timer A, and NewTOS has no MFP (§9.1).
6. **Hardware side, new in this re-validation (§9.1.4, source: datasheet):** the
   PWM FIFO word is *one duty-cycle value, not one PCM sample*. A Falcon buffer
   can therefore never be handed to a DMA engine as it stands; a CPU conversion
   pass into a staging buffer is unavoidable on the jack path, with or without
   DMA.

---

## 1. Gating — what is built and what lives at run time

### 1.1 Compile time

| Switch | `make 512` | Effect |
|---|:--:|---|
| `CONF_WITH_DMASOUND` | **1** | `bios/dmasound.c` is compiled at all (`bios/dmasound.c:23`, `bios/dmasound.h:17`); `LAST_ENTRY = 0x8d`, table of 142 slots (`bios/xbios.c:1273-1283`) |
| `CONF_WITH_XBIOS_SOUND` | **1** | selects the availability rule (§1.3); requires `CONF_WITH_DMASOUND`, else `#error` (`include/config.h:2180-2183`). Its own comment: "provides (some of) the Falcon XBIOS sound functions when running on STe- or TT-compatible hardware" (`include/config.h:1514-1519`) |

Values from the generated table `docs/api/_emutos-config.md:565,605` (audit
table) and `:181,221` (column `TARGET_512` of the divergence table, header
`:162-165`), not re-derived from `config.h` (protocol step 3.7). Defaults are
`include/config.h:1129-1134` and `:1514-1522`; the non-Atari targets switch both
off (`include/config.h:274-279`, `515-520`, `755-760`).

**Standing warning:** `1` in the table means "compiled in for all machines", not
"active in the Falcon profile". That is the whole point here.

### 1.2 Run-time detection (`bios/dmasound.c:128-150`)

`detect_dmasound()` (called from `machine_detect()`, `bios/machine.c:588-590`)
sets three global flags (`bios/dmasound.c:113-115`, exported through
`include/has.h:124-130`):

| Flag | How determined |
|---|---|
| `has_dmasound` | bus-error probe `check_read_byte()` on the control register `$FFFF8901` (`bios/dmasound.c:131`) |
| `has_falcon_dmasound` | only if `has_dmasound`: probe on `gpx_data_port` `$FFFF8943` (`bios/dmasound.c:139`) |
| `has_microwire` | `has_dmasound && !has_falcon_dmasound` — the Falcon has no Microwire and that is *not* detectable by bus error (`bios/dmasound.c:146-149`) |

Side effect of detection: if `has_dmasound`, `control = 0` is written — "Stop all
sound" (`bios/dmasound.c:141-142`).

### 1.3 `SOUND_IS_AVAILABLE` — the one macro that gates everything

`bios/dmasound.c:122-126`:

* with `CONF_WITH_XBIOS_SOUND` (our profile): `SOUND_IS_AVAILABLE = has_dmasound`
* without: `SOUND_IS_AVAILABLE = has_falcon_dmasound`

**Each** of the 14 functions begins with `if (!SOUND_IS_AVAILABLE) return
<function number>;` (`bios/dmasound.c:217, 232, 344, 371, 405, 426, 443, 461,
501, 523, 547, 723, 753, 782`; the `return` is on the following line). That is
the same signal as `xbios_unimpl` — the positive function number, **not**
`EINVFN` (`docs/api/xbios/_dispatcher.md §3`, D-008).

The three machine classes that follow:

| Machine | `has_dmasound` | `has_falcon_dmasound` | Behaviour of the group |
|---|:--:|:--:|---|
| Falcon | 1 | 1 | full function set |
| STe / TT | 1 | 0 | ST subset; Falcon-only functions → `EBADRQ` (-5), see §3 |
| no chip (**RPi4**) | 0 | 0 | all 14 return their function number |

### 1.4 What NewTOS does today — and why that is already conformant

`tos-api/src/xbios/mod.rs:157@other`: every id without an arm — so also
0x80…0x8D — falls to `other => other as i64`, i.e. returns the function number.
That is **bit-identical** with EmuTOS on a machine without a DMA sound chip
(§1.3, last row). NewTOS has no conformance bug here, only an honest "not
present" answer.

Two consequences to record in the ticket:

* The justification "no PSG" in `tos-api/src/xbios/system.rs:108@giaccess`
  (PSG group 0x1C/0x1D/0x1E/0x20, plausible-zero values) stays **correct** —
  the PSG is a different chip from DMA sound. Once DMA sound exists it is
  **incomplete**: `docs/api/xbios/_psg-group.md` (end, "A future NewTOS audio
  extension (`Dosound` stays a no-op) can expose PCM playback … at a reserved
  XBIOS slot", `:109-110`) is superseded by this spec: the slot is not to be
  reserved, it is taken by 0x80–0x8D. Sharpening the wording ("no PSG, but DMA
  sound present") belongs in the code — **that is the Developer's job, not this
  spec's.**
* `docs/api/xbios/_dispatcher.md §4` fixes `LAST_ENTRY = 0x57` for NewTOS
  (TT-shifter profile, 88 slots). With DMA sound the table grows to `0x8D` / 142
  slots — which also changes what `Xbios(N)` returns for **0x58…0x7F** (DSP
  range). In EmuTOS it stays the function number (`xbios_unimpl` slot instead of a
  range check — same answer, `docs/api/xbios/_dispatcher.md §3`), so it is
  indistinguishable from outside. **No contract break, but a deliberate profile
  decision** that the dispatcher spec §4 must be brought up to date with (see
  §10.4).

---

## 2. Opcode table and ABI

Table: `bios/xbios.c:1500-1513`. Prototypes: `bios/dmasound.h:26-39`. The m68k
stack shape is independently confirmed by FreeMiNT's trap bindings
`references/freemint/sys/mint/falcon.h:356-386` — EmuTOS itself defines **no**
binding macros for this group (`include/xbiosbind.h` knows only `Dosound`,
line 50).

| ID | TOS name | EmuTOS function | Signature (`bios/dmasound.h`) | Bytes on the stack | FreeMiNT binding |
|---:|---|---|---|:--:|---|
| 0x80 | `Locksnd` | `locksnd` | `LONG (void)` | 2 | `sys/mint/falcon.h:356` |
| 0x81 | `Unlocksnd` | `unlocksnd` | `LONG (void)` | 2 | `sys/mint/falcon.h:358` |
| 0x82 | `Soundcmd` | `soundcmd` | `LONG (WORD mode, WORD data)` | 6 | `sys/mint/falcon.h:362` |
| 0x83 | `Setbuffer` | `setbuffer` | `LONG (UWORD mode, ULONG start, ULONG end)` | 12 | `sys/mint/falcon.h:360` |
| 0x84 | `Setmode` | `setsndmode` | `LONG (UWORD mode)` | 4 | `sys/mint/falcon.h:364` |
| 0x85 | `Settracks` | `settracks` | `LONG (UWORD play, UWORD rec)` | 6 | `sys/mint/falcon.h:366` |
| 0x86 | `Setmontracks` | `setmontracks` | `LONG (UWORD track)` | 4 | `sys/mint/falcon.h:368` |
| 0x87 | `Setinterrupt` | `setinterrupt` | `LONG (UWORD mode, WORD cause)` | 6 | `sys/mint/falcon.h:370` |
| 0x88 | `Buffoper` | `buffoper` | `LONG (WORD mode)` | 4 | `sys/mint/falcon.h:373` |
| 0x89 | `Dsptristate` | `dsptristate` | `LONG (WORD dspxmit, WORD dsprec)` | 6 | `sys/mint/falcon.h:375` |
| 0x8A | `Gpio` | `gpio` | `LONG (UWORD mode, UWORD data)` | 6 | `sys/mint/falcon.h:378` |
| 0x8B | `Devconnect` | `devconnect` | `LONG (WORD src, WORD dst, WORD clk, WORD prescale, WORD protocol)` | 12 | `sys/mint/falcon.h:380` |
| 0x8C | `Sndstatus` | `sndstatus` | `LONG (WORD reset)` | 4 | `sys/mint/falcon.h:383` |
| 0x8D | `Buffptr` | `buffptr` | `LONG (LONG ptr)` | 6 | `sys/mint/falcon.h:385` |

**Correction to the ticket (made in the ticket, 2026-11-12):** the ticket named
`Buffoper` as 0x89. The right value is **0x88** (`bios/xbios.c:1508`); 0x89 is
`Dsptristate` (`:1509`). The ticket also forgot `Setmode` 0x84 — EmuTOS calls the
C function `setsndmode`, the API name is `Setmode` (`bios/xbios.c:1193` debug text
and `references/freemint/sys/mint/falcon.h:364`).

**AArch64 mapping** (NewTOS side): id in `x8`, arguments from `x0`, result in
`x0` (`tos-api/src/xbios/mod.rs:1-8@XBIOS`). Word arguments as usual through
`arg_i16`/`arg_u16` (`tos-api/src/trap.rs:92@arg_i16`). The SDK's own macros
already pass the `Setbuffer` addresses and the `Buffptr` pointer as `long`, i.e.
full 64-bit registers (`sdk/libc/overlay/include/mint/falcon.h:424@Setbuffer`,
`:450@Buffptr`; `sdk/libc/mintlib/include/mint/osbind.h:352@trap_14_wwll` `trap_14_wwll`:
`(short)` region, `(long)` beg, `(long)` end). For the width decision see §9.2.

---

## 3. The functions one by one

All line numbers in `references/emutos/bios/dmasound.c` unless noted. Error codes:
`EBADRQ = -5` (`include/gemerror.h:24`), `SNDNOTLOCK = -128`, `SNDLOCKED = -129`
(`bios/dmasound.h:19-20`).

### 0x80 `Locksnd()` / 0x81 `Unlocksnd()` — `:215-240`

* `Locksnd`: already locked → `SNDLOCKED` (-129); else lock and return **`1`**
  (`:220-224`). Not `0`.
* `Unlocksnd`: not locked → `SNDNOTLOCK` (-128); else unlock and `0`
  (`:235-239`).
* State: `static int sound_locked` (`:111`), zeroed in `dmasound_init()` (`:202`).
* **The lock is purely advisory.** `sound_locked` is read by **none** of the
  other twelve functions (only occurrences: `:111, 202, 220, 223, 236, 238`). A
  program that skips `Locksnd` can play unhindered. That is EmuTOS behaviour and
  thus the target for us.

### 0x82 `Soundcmd(mode, data)` — `:342-364`, helpers `:246-303` / `:308-337`

`data == -1` is the inquiry throughout; any other value sets and returns the new
value. **`Soundcmd` never returns an error code** — unknown `mode` values fall to
`return 0` (`:302`, `:336`).

`mode == 6` (`SETPRESCALE`) is handled **before** the Falcon/STe branch and
applies to both (`:347-356`): writes `data` (0…3) into `mode_control` bits 0-1,
returns `mode_control & 3`. Values outside 0…3 (so also `-1`) set nothing and
return the current value.

Falcon branch (`sndcmd_falcon`, `:246-303`):

| mode | Name (FreeMiNT `sys/mint/falcon.h:231-241`) | Register | Position of the value |
|---:|---|---|---|
| 0 | `LTATTEN` | `channel_attenuation` `$FFFF893A` | API bits 4-7 ↔ register bits 8-11 (`:256-260`) |
| 1 | `RTATTEN` | same | API bits 4-7 ↔ register bits 4-7 (`:263-269`) |
| 2 | `LTGAIN` | `channel_amplification` `$FFFF8939` | API bits 4-7 ↔ register bits 4-7 (`:272-278`) |
| 3 | `RTGAIN` | same | API bits 4-7 ↔ register bits 0-3 (`:281-287`) |
| 4 | `ADDERIN` | `codec_16bit_source` `$FFFF8937` | 2 bits (`:288-293`) |
| 5 | `ADCINPUT` | `codec_adc_source` `$FFFF8938` | 2 bits (`:294-299`) |

The unit "1.5 dB per step, form `0x00V0`" is **not** in EmuTOS but in FreeMiNT
(`sys/mint/falcon.h:235`) — secondary source, marked as such.

STe branch (`sndcmd_ste`, `:308-337`): only `mode` 0/1. The value is interpreted
**differently from the Falcon**: `atten = (data & 0xff) / 6`, capped at 40, from
that an LMC1992 fader command over Microwire (`:317-322`); the return is
`atten * 6`.

> **Edge (source: calculation over `:317-323`):** on the STe branch the value does
> **not round-trip** — `Soundcmd(LTATTEN, 0x0010)` returns `12`, not `0x10`. The
> current value also comes from two `static UWORD` (`:310`), not from the
> hardware (Microwire is write-only).

### 0x83 `Setbuffer(mode, start, end)` — `:369-396`

See §4. Checks: `mode > 1` → `EBADRQ`; `mode == 1` (record) without Falcon →
`EBADRQ` (`:374-375`). On the Falcon, bit 7 of the control register is set
(record) or cleared (playback) before writing (`:377-383`). Then start and end
address are written as three bytes each (`:386-393`) — **24 bit**, the upper 8
bits of a `ULONG` fall away silently. Always returns `0`.

### 0x84 `Setmode(mode)` — `:401-419`

`mode > 2` → `EBADRQ`. Else `mode_control &= 0x3f`, then `mode == 2` → bit 7
(mono), `mode == 1` → bit 6 (16 bit) (`:411-416`). Returns `0`. Values:
`STEREO8 = 0`, `STEREO16 = 1`, `MONO8 = 2` (FreeMiNT `sys/mint/falcon.h:251-253`).

> **Edge:** the check does **not** query `has_falcon_dmasound`. `Setmode(STEREO16)`
> therefore returns `0` (success) on STe/TT too, although the hardware cannot do
> 16 bit (`:405-418`).

### 0x85 `Settracks(play, rec)` / 0x86 `Setmontracks(track)` — `:424-452`

Both: without Falcon → `EBADRQ`; value > 3 → `EBADRQ`. `Settracks` writes `play`
into `track_control` bits 0-1 and `rec` into `record_tracks` `$FFFF8936`
(`:432-433`); `Setmontracks` writes `track` into `track_control` bits 4-5
(`:449`). Returns `0`.

### 0x87 `Setinterrupt(mode, cause)` — `:457-494`

* `mode > 1` → `EBADRQ` (`:464-465`).
* **Without Falcon** (STe/TT): no register access at all. `cause == 1` → `0`,
  anything else → `EBADRQ`, with the reason in the source: "On STE and TT,
  interrupts are always enabled" (`:467-475`).
* **With Falcon:** `cause &= 3`; `mode == 0` writes `cause` into bits 2-3 of the
  interrupt register `$FFFF8900`, `mode == 1` into bits 0-1 (`:477-491`). Returns
  `0`.

Meaning of the values — EmuTOS names them only in the function heading
("Timer-A or MFP-i7", `:455`); the full assignment is in FreeMiNT
(`sys/mint/falcon.h:259-268`): `SI_TIMERA = 0`, `SI_MFPI7 = 1`; `SI_NONE = 0`,
`SI_PLAY = 1`, `SI_RECORD = 2`, `SI_BOTH = 3`. The FreeMiNT driver confirms the
mapping *by use*: it writes `FalInt = 0x4` (i.e. `cause = 1` in bits 2-3) and at
the same time hooks Timer A (`sys/xdd/audio/falcon.c:82,187`). So
`mode 0 → bits 2-3 → Timer A` is backed by two sources.

### 0x88 `Buffoper(mode)` — `:499-516`

* `mode < 0` (**any** negative value, not just -1) is the inquiry: returns
  `control & 3`, on the Falcon additionally the record bits as
  `((control >> 2) & 0xc)` (`:504-511`).
* Else: `control = ((mode & 0xc) << 2) | (mode & 3)` (`:514`). API bits 0-1 →
  control bits 0-1, API bits 2-3 → control bits 4-5. Returns `0`.

Meaning (FreeMiNT `sys/mint/falcon.h:272-275`): `SB_PLA_ENA = 1`,
`SB_PLA_RPT = 2`, `SB_REC_ENA = 4`, `SB_REC_RPT = 8`.

> **Two edges, both read from `:504-514`:**
> 1. The **write path is not** `has_falcon_dmasound`-dependent, the **read path
>    is**. On STe/TT `Buffoper(SB_REC_ENA)` sets control bits 4-5 that the
>    inquiry then does not report.
> 2. The write writes the **whole byte** — bit 7 (the record/playback register
>    bank switch, §4.6) is reset to 0 (playback) on **every** `Buffoper` set call.
>
> **Not established by any pinned source:** whether the hardware clears control
> bit 0 itself when a single-shot frame ends. EmuTOS has no reader of the bit;
> the FreeMiNT driver never reads `control` (it keeps its own `status` counter,
> `sys/xdd/audio/falcon.c:131-140`). A program that polls `Buffoper(-1)` until the
> bit drops depends on it — see §9.1.2 S3 (measure on the oracle).

### 0x89 `Dsptristate(dspxmit, dsprec)` — `:521-540`

Without Falcon → `EBADRQ`. Else set/clear bit 7 in `crossbar_src` and
`crossbar_dest` respectively (`:529-537`); returns `0`. "0 = tristate, 1 =
enable" (FreeMiNT `sys/mint/falcon.h:277`).

### 0x8A `Gpio(mode, data)` — `:545-568`

Without Falcon → `EBADRQ`. `mode 0` = set direction (`data & 7` into
`gpx_data_direction` `$FFFF8941`, `:556`), `mode 1` = read pins (`gpx_data_port &
7` **as return value**, `:559`), `mode 2` = write pins (`gpx_data_port = data`,
**unmasked**, `:561` — corrected/added 2026-11-12); anything else → `EBADRQ`
(`:553-565`). Three bits, `GPIO_SET/READ/WRITE = 0/1/2` (FreeMiNT
`sys/mint/falcon.h:281-283`).

### 0x8B `Devconnect(src, dst, clk, prescale, protocol)` — `:721-730`

Falcon: `devconnect_falcon` (`:590-686`), else `devconnect_ste` (`:691-719`).

`src` (FreeMiNT `sys/mint/falcon.h:287-290`): 0 `DMAPLAY`, 1 `DSPXMIT`, 2
`EXTINP`, 3 `ADC`. `dst` is a **bit mask** (`:294-297`): 1 `DMAREC`, 2 `DSPRECV`, 4
`EXTOUT`, 8 `DAC`. `clk` (`:301-303`): 0 = internal 25.175 MHz, 1 = external, 2 =
internal 32 MHz.

Falcon branch: writes the source bits into `crossbar_src` (`:597-635`), then for
every set destination bit the matching field of `crossbar_dest` (`:640-675`),
finally `prescale` into `freq_ext` (external clock) or `freq_int` (internal
clock) (`:680-683`). Return: **`0` for an invalid `src`** (`:633`, deliberately as
TOS 4.04), else **`1`** (`:685`).

> EmuTOS documents three defects of the TOS 4.04 original at `:573-589` (hardly any
> error checking, "ill-defined" return value, garbage in `crossbar_dest` with an
> external clock) and deviates deliberately: only `src` is checked, return `1`
> instead of garbage. The author also notes: "I do not know if `crossbar_dest` is
> set correctly, having no way of testing this" (`:586-588`). **This is untested
> EmuTOS code** — a warning for us, not evidence.

STe branch (`:691-719`): accepts **only** `src == 0 && dst == 8 && clk == 0`, else
`EBADRQ` (`:696-697`). `prescale == 0` = "STE compatible mode", returns `0`
immediately (`:702-703`); otherwise `prescale` is mapped to the four STe rates
(§4.7). Returns `0` — **not `1`** as the Falcon branch.

### 0x8C `Sndstatus(reset)` — `:751-767`

* Without Falcon: **always `0`**, `reset` has no effect (`:756-757`).
* With Falcon and `reset != 0`: `channel_attenuation = 0`, then set bit 3 in
  `codec_16bit_source` `$FFFF8937` and clear it again (`:759-764`). Which registers
  this toggle zeroes is listed by the source comment `:735-750`: `$FF8920`,
  `$FF8921`, `$FF8930-33`, `$FF8934-39`, `$FF893A-3B`; `$FF893C-3D` is set to
  `$003F`. **`$FF8900`/`$FF8901` (interrupt and control) are not in that list.**
* Return always `(codec_status >> 4) & 0x3f` (`:766`).

> **Source conflict (added 2026-11-12):** FreeMiNT documents the reset effects as
> "DSP tristated, gain=atten=0, matrix reset, ADDERIN=0, Mode=STEREO8,
> Play=Record=Monitor tracks=0, **interrupts disabled, buffer operation
> disabled**" (`references/freemint/sys/mint/falcon.h:328-333`). EmuTOS' list does
> not name the interrupt/control registers. Whether the TOS 4 original (what
> FreeMiNT describes) or EmuTOS is right about `$FF8900/01` cannot be decided from
> the sources; EmuTOS is primary, but this is a measurement item (oracle, falcontc
> profile): `Buffoper(SB_PLA_ENA)`, `Sndstatus(1)`, `Buffoper(-1)`.

Field layout of the return — from FreeMiNT (`sys/mint/falcon.h:338-344`): bits 0-1
are an error code (`SS_OK 0`, `SS_CTRL 1`, `SS_SYNC 2`, `SS_SCLK 3`), bit 4
`SS_RTCLIP`, bit 5 `SS_LTCLIP`. Both together are consistent: **conclusion
(inference, not cited)** — right after a reset the register value `$003F` named in
the comment gives exactly `3` = `SS_SCLK` ("serial clock out of valid range"),
which fits a just-zeroed clock matrix. A program should therefore call
`Devconnect` after `Sndstatus(1)` and then check again.

### 0x8D `Buffptr(ptr)` — `:772-803`

`ptr` points to a 16-byte structure `SndBufPtr` — four `LONG`: `play`, `record`,
`res1`, `res2` (`:774-779`; identical in FreeMiNT `sys/mint/falcon.h:348-353`,
there with `char *` for the first two).

Flow: on the Falcon first set control bit 7, read the three counter bytes →
`record`, clear bit 7 again (`:787-795`); then read the counter bytes → `play`
(`:797-800`). Returns `0`.

> **Edges:** (a) **no null-pointer check** — `ptr == 0` writes to address 0
> (`:785`). (b) `res1`/`res2` are **not** written; the caller sees its own old
> contents there. (c) On STe/TT `record` is **not written at all** (`:787` is
> Falcon-gated), so also old contents. (d) The function leaves control bit 7
> **cleared**, even if the caller had set it before (§4.6). (e) **Inference
> (added 2026-11-12):** the three counter bytes are read one after the other with
> no latch in the code (`:792-794`, `:797-799`); while the counter advances, the
> assembled value can be torn at a byte carry. Whether the hardware latches is not
> in any pinned source.

---

## 4. The buffer mechanism — what a driver must serve

> **Source position, honestly:** EmuTOS *programs* the registers but never *uses*
> them (§0.2). The meaning of "end address", "loop" and "end signal" therefore
> cannot be proved from EmuTOS alone. What follows has two sources: EmuTOS for the
> register image and the API effect, FreeMiNT's Falcon audio driver
> (`references/freemint/sys/xdd/audio/falcon.c`,
> `references/freemint/sys/xdd/audio/dmasnd.h`) for the actual playback sequence.
> Every line below says which one.

### 4.1 The register image (EmuTOS `bios/dmasound.c:25-72`)

Base `0xFFFF8900` (`:72`). From the structure offsets (`:28-69`):

| Address | Field | Role |
|---|---|---|
| `$FFFF8900` | `interrupt` | interrupt enable (Falcon only), §3/0x87 |
| `$FFFF8901` | `control` | bit 0/1 playback on/loop, bit 4/5 record on/loop, **bit 7 register bank** |
| `$FFFF8903/05/07` | `frame_start_{high,mid,low}` | start address, 24 bit |
| `$FFFF8909/0B/0D` | `frame_counter_{high,mid,low}` | running pointer, 24 bit, **read-only in practice** |
| `$FFFF890F/11/13` | `frame_end_{high,mid,low}` | end address, 24 bit |
| `$FFFF8920` | `track_control` | tracks (Falcon) |
| `$FFFF8921` | `mode_control` | bit 7 mono, bit 6 16 bit, bits 0-1 STe prescaler |
| `$FFFF8922/24` | `microwire_data`/`_mask` | LMC1992 (STe/TT only) |
| `$FFFF8930/32` | `crossbar_src`/`_dest` | connection matrix (Falcon) |
| `$FFFF8934/35` | `freq_ext`/`freq_int` | prescalers (Falcon) |
| `$FFFF8936…3D` | codec registers | see §3 |
| `$FFFF8941/43` | `gpx_data_direction`/`_port` | DSP connector |

The addresses `$FF8903/05/07`, `$FF8909/0B/0D`, `$FF890F/11/13` and `$FF8920` are
independently confirmed by FreeMiNT (`sys/xdd/audio/dmasnd.h:13-30`).

**For NewTOS this register image is internals, not contract** — see §8.

### 4.2 Handing over a buffer

1. `Setbuffer(SR_PLAY=0, start, end)` writes start and end into the playback
   register bank (EmuTOS `:386-393`).
2. `Buffoper(SB_PLA_ENA)` starts (EmuTOS `:514`; FreeMiNT writes `DMActl =
   DMA_PLAY = 0x01` at the same point, `sys/xdd/audio/dmasnd.h:10`, `falcon.c:181`).
3. The chip reads from `start` upwards and increments `frame_counter`; when it
   reaches `end`, the frame is done.

**The end address is exclusive** — proved from FreeMiNT, not from EmuTOS: the
driver fills `dmab[i].used` bytes from `dmab[i].buf` and sets
`DMASetEnd(dmab[i].buf + dmab[i].used)` (`sys/xdd/audio/falcon.c:179-180,
184-185`; macro `sys/xdd/audio/dmasnd.h:39-44`). End = first address **behind**
the data; the frame length is `end - start`.

**Alignment:** the same driver makes sure the length is **even** and if need be
doubles the last sample — comment in the source: "DMA wants even number of
samples" (`sys/xdd/audio/falcon.c:168-175`). That is a hardware requirement on
the frame length, not an API check: **neither EmuTOS nor any XBIOS function
checks it.**

**Next-frame semantics (inference, added 2026-11-12):** the FreeMiNT driver writes
the start/end registers of buffer *i* **while the other buffer is still playing**
(`falcon.c:182-188`, the `else` branch taken when `status != 0`). Writing the
register bank while a frame plays therefore prepares the *next* frame; the hardware
latches start/end at the frame boundary. Neither source states this in words; it
follows from the driver's structure.

### 4.3 End and loop

* `SB_PLA_ENA` alone: one frame, then the chip stops. The FreeMiNT driver uses
  exactly that for the single buffer (`falcon.c:176-181`) — **but note** that the
  stop itself is not read anywhere (§3/0x88, "Not established").
* `SB_PLA_ENA | SB_PLA_RPT` (= 3, `DMA_PLAY|DMA_LOOP`): "Play repeat (continuous
  loop mode)" — the name in FreeMiNT (`sys/mint/falcon.h:273`, constant
  `sys/xdd/audio/dmasnd.h:10-11`, used `falcon.c:186`). **Inference (not cited):**
  that start and end are reloaded from the frame registers at the frame boundary
  follows from the double-buffer scheme below, but is in neither source in words.
* Stop: `Buffoper(0)` (`DMA_OFF`, `falcon.c:112,134`).

The double-buffer scheme of the FreeMiNT driver is the blueprint for gapless
playback (`falcon.c:121-146` together with `:176-188`): buffer A runs in a loop; in
the end interrupt the driver switches to buffer B by re-setting the `Setbuffer`
registers and re-writing `control`; if no more data is queued it switches to
`DMA_OFF`.

### 4.4 The interrupt contract

**This is the core and the most expensive spot.** Two layers:

1. **Chip side:** `Setinterrupt(mode, cause)` enables in the sound chip *that* a
   signal is generated at frame end — Timer A (`mode 0`) or MFP-i7 (`mode 1`)
   (EmuTOS `:477-491`).
2. **Vector side:** the **program itself** installs the handler, through the MFP:
   `Xbtimer(0, 8, 1, handler)` plus `Jenabint(13)`
   (`references/freemint/sys/xdd/audio/falcon.c:81-83`). **EmuTOS installs not a
   single sound interrupt handler** — `bios/dmasound.c` contains no handler
   function, and `setinterrupt` touches no vector.

The number is EmuTOS-proved: `MFP_TIMERA = 13`, and EmuTOS even defines the
synonym **`MFP_DMASOUND`** (`include/biosdefs.h:192-193`). `Xbtimer` → `setup_timer`
(writes `tacr = 0`, `tadr = data`, `tacr = control` for timer 0, `bios/mfp.c:133-141`)
+ `mfpint` → vector at `(0x40 + 13) * 4` = `0x134` (`bios/mfp.c:114-120,
163-169`). MFP-i7 is interrupt 7, vector `(0x40 + 7) * 4` = `0x11C` (same
formula).

What `control = 8` means to the MFP (Timer A event-count mode) is MFP hardware
semantics: **not stated in any pinned source** (EmuTOS only writes the value). It
is the FreeMiNT driver's choice that makes the chip's frame-end pulse count as one
event; flagged as inference.

**Consequence for NewTOS:** the program-visible sequence (`Jdisint(13)`,
`Xbtimer(0,8,1,h)`, `Jenabint(13)`) exists only partially (§1.4 correction 2) and
nothing calls `h`. See §9.1.

### 4.5 Polling as a replacement

`Buffptr` delivers the running counter (§3/0x8D). A program can use it to see
without an interrupt how far playback is. The FreeMiNT driver does not do that (it
is interrupt-driven), but the function exists for exactly this. For NewTOS this is
the **realistic first playback path** (§9.1, option D).

### 4.6 The bit-7 state machine (easy to overlook)

Control bit 7 switches between the record and the playback register bank. Who sets
and clears it:

| Call | Effect on bit 7 | Line |
|---|---|---|
| `Setbuffer(0, …)` | clears | `:382` |
| `Setbuffer(1, …)` | **sets — and leaves it set** | `:380` |
| `Buffptr(…)` | sets, reads, **clears** | `:789, 794` |
| `Buffoper(mode ≥ 0)` | clears (whole-byte write) | `:514` |
| `Buffoper(mode < 0)` | unchanged (read only) | `:507` |

After `Setbuffer(SR_RECORD, …)` the record bank stays selected until one of the
other three operations switches back. That is **observable state across call
boundaries** — a NewTOS backend must reproduce it or justify the deviation.

### 4.7 Sample rates

**STe-compatible path** (`Devconnect(…, prescale)` with `clk = 0` and
`prescale != 0`, resp. `Soundcmd(SETPRESCALE, n)`), EmuTOS `devconnect_ste:700-716`
with source comments:

| `mode_control` bits 0-1 | EmuTOS comment | `prescale` values mapped there |
|:--:|---|---|
| 3 | "50 kHz" (`:705`) | 1 |
| 2 | "25 kHz" (`:709`) | 2, 3, 4 |
| 1 | "12 kHz" (`:712`) | 5, 6, 7, 8 |
| 0 | "leave it at 6 kHz" (`:714`) | all others |

`prescale == 0` means "STE compatible mode" and returns **before** any change
(`:702-703`). The divider names `PRE160/320/640` are the **values 3/2/1** of
`mode_control` bits 0-1 (corrected 2026-11-12: the old text wrote "= bits 3/2/1"),
`PREMUTE` = 0 ("was /1280, now is invalid, mutes") — in FreeMiNT
(`sys/mint/falcon.h:243-247`); the rates of the ST path **listed** (not measured) by
the FreeMiNT driver — 50066, 25033, 12517, 6285 Hz — also there
(`sys/xdd/audio/falcon.c:209-250`). Note the EmuTOS source comment at `:197`
("TT-compatible prescale to /160 = 50MHz") means kHz; it is a comment error.

**Falcon path** (internal 25.175 MHz clock): EmuTOS writes `prescale` **raw** to
`freq_int` (`:683`) and says nothing about the resulting rate. FreeMiNT names the
table (`sys/mint/falcon.h:309-324`):

| `prescale` | Hz | | `prescale` | Hz |
|:--:|---:|---|:--:|---:|
| 1 | 49170 | | 7 | 12292 |
| 2 | 32780 | | 9 | 9834 |
| 3 | 24585 | | 11 | 8195 |
| 4 | 19668 | | 5 | 16390 |

FreeMiNT itself notes that the official documentation names wrong Hz numbers for
many values (`sys/mint/falcon.h:307`). **Corrected 2026-11-12:** the values that
are "invalid for CODEC" are **6 (14049 Hz), 8 (10927), 10 (8940) and 12–15** — the
*even* in-between values and 12–15 (comments `sys/mint/falcon.h:315-324`); the
odd values 5, 7, 9, 11 in the table above are valid. The old text said the odd
values were invalid, which is backwards. **Inference (not cited, but recomputed):**
the table is exactly `25 175 000 / (256 × (prescale + 1))` — for 1, 2, 3, 4 that
gives 49170, 32780, 24585, 19668; the invalid ones give 14049, 10927, 8940
likewise.

### 4.8 Sample format

From FreeMiNT (`sys/mint/falcon.h:213-222`), described **nowhere** in EmuTOS: all
values are **signed**; 8-bit stereo is `L R L R …` (1 byte each), 16-bit stereo is
`LL RR LL RR …` (2 bytes each), with several tracks interleaved `L0 R0 L1 R1 …`.
The byte order of the 16-bit values is that of the m68k side (big-endian) — not
stated there, but follows from the target machine. **Added 2026-11-12:** the SDK's
`falcon.h` overlay says "big-endian" explicitly for the extended modes
(`sdk/libc/overlay/include/mint/falcon.h:380@MODE_MONO16`: "16 bit mono,
big-endian"), which supports the same reading for the base modes. **For NewTOS an
open contract question, see §9.3.**

---

## 5. State after system start

`dmasound_init()` (`bios/dmasound.c:200-208`, called from `bios/bios.c:442-445`)
does three things: `sound_locked = 0`, `lmc1992_init()` (only with Microwire, i.e.
STe/TT: volume 0 dB, fader 0 dB, treble/bass 0 dB, "Mix GI sound chip output" —
`:167-180`) and on the Falcon `falcon_dmasound_init()` (`:182-198`):

| Call | Meaning according to the source comment `:184-189` / `:192-197` |
|---|---|
| `devconnect(0, 8, 0, 0, 1)` | DMA playback → DAC (headphone/speaker), internal 25.175 MHz clock, TT/STe compatibility mode, "disable handshaking" (`protocol = 1`) |
| `setsndmode(0)` | 8-bit stereo |
| `soundcmd(2, 0x0080)` / `(3, 0x0080)` | left/right gain = 8 |
| `soundcmd(4, 0x0003)` | ADDERIN = ADC + matrix |
| `soundcmd(5, 0x0003)` | ADC source left+right = PSG |
| `soundcmd(6, 0x0003)` | TT-compatible prescaler /160 (≙ 50 kHz) |

This is the state a program finds on an EmuTOS Falcon **before its first own
call** — so part of what interrogating programs see. For NewTOS exactly this
initial state should be reproduced, so that `Soundcmd(…, -1)` inquiries answer
plausibly.

---

## 6. The `_SND` cookie — the discovery side of the contract

`setvalue_snd()` (`bios/machine.c:409-447`), hooked into `fill_cookie_jar()` at
`bios/machine.c:709-719`, tag `COOKIE_SND` = `0x5F534E44` ("_SND",
`include/cookie.h:30`).

| Bit | Name (`include/cookie.h:81-86`) | When set |
|---:|---|---|
| 0x01 | `SND_PSG` | `CONF_WITH_YM2149` (`machine.c:413-415`) |
| 0x02 | `SND_8BIT` | `HAS_DMASOUND` (`:417-420`) |
| 0x04 | `SND_16BIT` | `HAS_FALCON_DMASOUND` (`:422-425`) |
| 0x10 | `SND_MATRIX` | same, same line |
| 0x08 | `SND_DSP` | `HAS_DSP` (`:427-430`) |
| 0x20 | `SND_XBIOS` ("XBIOS / XGSB") | **never** — defined but set nowhere in the tree (tree sweep, re-run 2026-11-12: only occurrence `include/cookie.h:86`) |

In addition: if DIP switch 8 is on (bit 7 of `cookie_swi` off), `SND_8BIT` is
cleared again — "just like TOS3/TOS4 do" (`bios/machine.c:432-444`; only with
`has_dip_switches`).

> **Comment against code (classic case):** the comment block at
> `bios/machine.c:709-716` names a **different** bit assignment (0x04 = "DMA Record
> (w/XBIOS)", 0x08 = "16-bit CODEC", 0x10 = "DSP") than the constants the code
> uses one line later (`include/cookie.h:81-86`). The code wins. Numerically it
> does not show, because on the Falcon all five bits are set anyway (0x1F) and on
> the STe only 0x03 — but whoever copies the assignment from the comment builds the
> cookie wrong for in-between cases. **Entry for `docs/api/_discrepancies.md` (not
> yet allocated), see §10.**

**Measured values of the oracle** (`docs/COMPATIBILITY.md:1636`, row 49): `_SND` =
`0x00000003` on the TT profile, `0x0000001F` on the falcontc profile; NewTOS has
the cookie **absent** (ADR-012 amendment 2026-10-28, `docs/MEMORY.md`).

**What consumers do with the bits** (re-read 2026-11-12, FreeMiNT audio drivers —
they show how the ecosystem reads the cookie):

* `0x02` → STe DMA driver and LMC mixer: `references/freemint/sys/xdd/audio/dma.c:41-47`,
  `lmc.c:44-50`; both then write the STe register image directly
  (`dmasnd.h`) — a program that reads bit 1 as "I may poke `$FF8900`" fails on
  NewTOS.
* any of `0x04|0x08|0x10` → Falcon driver and Falcon mixer:
  `sys/xdd/audio/falcon.c:47-53`, `:318-324`.
* `0x08` → DSP driver: `sys/xdd/dsp56k/dsp56k.c:625-635`.
* `0x01` → PSG driver: `sys/xdd/audio/psg.c:66-73`.
* `0x20` is `SND_EXT` in the SDK header: "Extended XBIOS routines (Milan, GSXB)",
  gating `NSoundcmd`, `NSetinterrupt` (handler argument), the extended `Sndstatus`
  commands and the extended `Setmode` modes
  (`sdk/libc/overlay/include/mint/falcon.h:179@SND_EXT`, `:217`, `:320`, `:378`).

**Why that matters for SND01:** the FreeMiNT Falcon audio driver refuses service
when `_SND` is missing or none of the bits `SND_16BIT|SND_DSP|SND_MATRIX` is set
(`sys/xdd/audio/falcon.c:47-53`). NewTOS sets **no** `_SND` today —
`tos-api/src/cookie.rs:223@seed_defaults` deliberately leaves out
`_VDO`/`_SND`/`_FDC`/`_FPU`/`_CPU` (ADR-012 amendment 2026-10-28). Without `_SND`, a
ported sound program will not call the XBIOS functions at all. **The cookie is part
of the work, not a side issue.** What value and under which condition: owner
decision, draft in `TASK/SND01.md` § "Spec review", "Draft for the owner".

---

## 7. Error codes of the group

| Value | Name | Origin | When |
|---:|---|---|---|
| function number (0x80…0x8D) | — | `bios/dmasound.c:218` etc. | no sound chip present |
| `-5` | `EBADRQ` | `include/gemerror.h:24` | invalid mode/range; Falcon-only function on STe/TT |
| `-128` | `SNDNOTLOCK` | `bios/dmasound.h:19` | `Unlocksnd` without prior `Locksnd` |
| `-129` | `SNDLOCKED` | `bios/dmasound.h:20` | `Locksnd` with an existing lock |
| `0` | E_OK | everywhere | success |
| `1` | — | `:224` resp. `:685` | `Locksnd` succeeded; `Devconnect` (Falcon) succeeded |

> **Difference between the sources:** FreeMiNT lists the same two sound errors as
> `ESNDLOCKED = -129` and **`ESNDNOTLOCK = -130`** (`sys/mint/errno.h:561-562`), not
> -128. EmuTOS is primary, so **-128** holds. **Inference on the cause:** -128 is
> already taken by `EINTR` in FreeMiNT's errno table (`sys/mint/errno.h:558`), which
> would explain the shift; that is not proved. Candidate for the deviation log
> (§10).

---

## 8. Contract or internals — the line per `docs/COMPATIBILITY.md`

Per `docs/COMPATIBILITY.md:35-47` (contract surface) and `:49-64` (free
implementation), rule of thumb `:63-64`: "if a program can read it, time it, or
depend on it through a documented interface, it is contract."

**Contract — must match:**

* Trap 14, function ids 0x80…0x8D and their argument count/order (§2).
* All return values including the special cases: `Locksnd → 1`, `Devconnect → 1`
  (Falcon) resp. `0` (STe branch and invalid `src`), `EBADRQ`,
  `SNDLOCKED`/`SNDNOTLOCK`, and the function-number-as-"not present".
* The inquiry forms: `Soundcmd(mode, -1)`, `Buffoper(mode < 0)`, `Gpio(1, …)`,
  `Sndstatus(0)`.
* `SndBufPtr` (4 × LONG, `play` first) and the fact that `res1`/`res2` stay
  untouched (§3/0x8D) — in the pointer width of the caller's ABI
  (`docs/COMPATIBILITY.md:1960`, §9.2).
* The meaning of start/end (end exclusive), the loop bits, and that a frame end
  can reach a program (§9.1).
* The `_SND` cookie (§6) — `docs/COMPATIBILITY.md:46` names the cookie jar
  explicitly as contract surface.
* The state after start (§5), as far as inquiries expose it.

**Internals — NewTOS may differ:**

* The whole register image `$FFFF8900…$FFFF8943` (§4.1). That address does not
  exist on the Pi; a program that writes there cannot work on NewTOS anyway — the
  same deliberate "ABI silence" as for the PSG group
  (`docs/api/xbios/_psg-group.md`, edge 2).
* The connection matrix as a *mechanism* (crossbar bits). The *effect* — "DMA
  playback ends at the DAC" — is contract, the bit encoding is not.
* The 24-bit address limit of the frame registers (§4.2) — a quirk of Falcon DMA,
  not an API promise. **But:** the question it leads to, what NewTOS does with a
  pointer above 16 MB, is contract-relevant (§9.2).
* Whether PWM, HDMI or something else sounds inside — a device parameter.

---

## 9. Gaps — what NewTOS honestly cannot deliver

Each gap here is to be carried as a documented "not present", not as silent
success (CLAUDE.md rule 2, `TASK/SND01.md` "Abnahme").

### 9.1 End-of-buffer notification without MFP

EmuTOS/Falcon signal the end of a frame through the sound chip's pulse into MFP
Timer A (or MFP i7), selected with `Setinterrupt` (0x87); the program catches it
with a handler it installed through `Xbtimer`/`Jenabint` (§4.4). NewTOS has no MFP.
This section lays out which observable semantics a replacement must keep and which
mechanisms can carry it. Hardware facts in §9.1.4 are from the BCM2711 datasheet
(Track B) and say so; **where a fact is not in a pinned source, the text says so
instead of filling the gap.**

#### 9.1.1 NewTOS state today (read 2026-11-12)

* `Mfpint` (0x0D) and `Xbtimer` (0x1F) are wired: they record the handler with
  `note_running(Hook::Vector(0x40 + num), …)`, warn once per process (ADR-031),
  call nothing, and answer the oracle's `d0` (`tos-api/src/xbios/mod.rs:140@MFPINT`,
  `tos-api/src/xbios/mfp.rs:69@mfpint`, `:79@xbtimer`). `Jdisint`/`Jenabint`
  (0x1A/0x1B) are not wired and answer their function number
  (`tos-api/src/xbios/mod.rs:157@other`). ADR-031's table lists all four as "not
  served, warning" (`docs/MEMORY.md`, ADR-031 table row "trap and CPU vectors via
  `Setexc` (< 0x100), `Mfpint`, `Xbtimer`, `Jdisint`, `Jenabint`").
* An EL0 upcall mechanism exists (ADR-029): `post_upcall(slot, desc)` stores a
  handler VA plus up to four arguments in the target process's slot
  (`tos-api/src/bdos/process.rs:1213@post_upcall`, `:1233@take_pending_upcall`);
  the SVC handler delivers it **at that process's next syscall return to EL0**
  (`kernel/src/arch/aarch64/exceptions.rs:791@take_pending_upcall`,
  `deliver_upcall` at `:1809@deliver_upcall`). The handler is an ordinary C-callable
  function that returns through a trampoline and `Psigreturn`
  (`docs/analysis/coop-kernel/10-upcall-signal-design.md` §1, §7; `kbdvec` is the
  existing consumer, `kernel/src/arch/aarch64/exceptions.rs:829@deliver_pending_kbdvec`).
* **Delivery from an interrupt is not built.** The lower-EL IRQ vector now reaches
  `rust_irq_handler` (`kernel/src/arch/aarch64/exception_vectors.s:115@__vec_irq`;
  the design note of 2026-07-24 §7.3 still says it halts), but
  `rust_irq_handler` handles only the PL011 and the 200 Hz tick and never looks at
  a pending upcall (`kernel/src/drivers/gic.rs:203@rust_irq_handler`). So a posted
  upcall waits for the program's next trap. A program that computes without
  trapping is not interrupted. Whether the AES wait loops deliver pending upcalls
  while a program is parked there is **not established here** (needs a read of
  `docs/analysis/coop-kernel/20-sleepwake-substrate-design.md`).
* A TOS handler is an m68k interrupt routine (clears its in-service bit, ends in
  `rte`). It cannot be called on AArch64. The existing precedent for a program hook
  is a C-callable function (ADR-031 "served": `kbdvec`). Which calling convention a
  `Setinterrupt` handler has on NewTOS is an owner decision (Q-N2 in the ticket).

#### 9.1.2 What a replacement must keep — observable semantics

Each item says where it comes from and whether it is established.

| # | Semantic | Source | Status |
|---|---|---|---|
| S1 | `Setinterrupt(mode, cause)` return values: `mode > 1` → `EBADRQ`; Falcon branch returns `0` for any `cause`; STe/TT branch returns `0` only for `cause == 1`, else `EBADRQ` | `bios/dmasound.c:464-491` | read |
| S2 | While enabled (`cause` bit 0 = play), the program's handler runs once at the end of each play frame, asynchronously to the main program | EmuTOS: only the comment "Timer-A or MFP-i7" (`:455`); FreeMiNT driver relies on one interrupt per frame (`falcon.c:121-146`, `:187`) | read (driver use) — "once per frame **pass** in loop mode" is **inference** |
| S3 | `Buffoper(-1)` bit 0: whether it drops when a single-shot frame ends | no pinned source (§3/0x88) | **not established** → measure |
| S4 | `Buffptr` `play`: lies in `[start, end]`, advances while playing, stops at `end`/loops back in loop mode | EmuTOS reads the counter only (`:797-800`); semantics from hardware | **not established** → measure (granularity, off-by-one, what it reads after the frame ended) |
| S5 | `Setbuffer` during playback prepares the next frame (gapless double buffering) | FreeMiNT driver structure (`falcon.c:182-188`) | **inference** |
| S6 | Handler vector selection: `mode 0` = Timer A = MFP interrupt 13 (vector `0x134`), `mode 1` = MFP i7 (vector `0x11C`) | `bios/dmasound.c:477-491`, `include/biosdefs.h:192-193`, `bios/mfp.c:114-120` | read |
| S7 | Time budget: a handler has up to one frame time to queue the next frame (loop mode replays the old frame otherwise) | inference from S5 | **inference** |

S3, S4 and part of S2 are what an oracle measurement settles: the oracle's falcontc
profile runs the real EmuTOS Falcon code in Hatari (`docs/COMPATIBILITY.md:1636`).
The existing tosdump round only measures the inquiries (`tosdump/cases/bios-r1.tsv:230-234@Soundcmd`;
`Buffoper(-1)`, `Sndstatus(0)`, `Soundcmd(m,-1)`); playback-state cases are marked
"later" in `tosdump/cases/COVERAGE-BIOS.md:208-220@Locksnd`. Proposed cases are in the
ticket's "Spec review".

#### 9.1.3 The options

Detection of "frame end" (A–C) and delivery to the program (U1, U2) are separate
choices.

**Detection**

* **A — DMA completion interrupt.** The play frame is a chain of DMA control
  blocks; the last block of each user frame has `TI.INTEN`. The channel's
  completion interrupt (`CS.INT`, datasheet Table 38 / 40, §9.1.4) reaches the GIC
  as INTID `112 + n` (§9.1.4); `gic.enable_spi` is the existing hook
  (`kernel/src/drivers/gic.rs:95@enable_spi`) and `rust_irq_handler` needs a branch
  for the channel INTID. *Keeps:* S1, S6 trivially (software); S2 within IRQ
  latency; S4 from `SOURCE_AD`/`TXFR_LEN` ("updated by the DMA engine as the
  transfer progresses", datasheet Table 41/48) mapped back from the staging buffer
  to the user buffer (§9.1.4, point 1); S5 by queueing the next frame's block chain
  behind the running one. *Does not keep, or needs a decision:* `CS.INT` is
  sticky and level ("must be manually cleared down, even if the next CB has
  `INTEN = 0`", Table 38), so two frame ends between two ISR runs merge into one
  notification — a counter from the progress registers is needed if every end must
  be seen. DMA completion is *not* audible end: the PWM FIFO still holds up to its
  depth in samples (§9.1.4, point 3).
* **B — DMA ring, frame ends found by polling the existing 200 Hz tick.** Same
  transport as A (cyclic block chain over a staging ring, refilled by the CPU), but
  no new interrupt source: the tick (`kernel/src/drivers/timer.rs:154@handle_irq`)
  converts the next PCM into the ring, reads the progress register and detects
  frame ends. *Keeps:* S1, S4, S5, S6; S2 with up to one tick (5 ms) of lateness.
  *Cost:* no GIC wiring, no ISR for a new INTID. *Risk:* the ring must hold more
  than the longest time the tick can be late; a ring of N frames of audio gives
  `N / rate` seconds of slack (arithmetic, not measured).
* **C — PWM FIFO fed by CPU from a timer interrupt, no DMA.** The PWM block has no
  interrupt of its own (datasheet ch. 8, Table 102 — §9.1.4), so a *foreign* timer
  must come back at least every FIFO-drain time (§9.1.4, point 3) and write words to
  `FIF1` until `FULL1`. Frame ends and the pointer are pure software bookkeeping
  (there is no FIFO fill-level register, only `FULL1`/`EMPT1`, datasheet Table 154).
  *Keeps:* S1, S5, S6 (software); S2 within timer latency; S4 only to FIFO
  granularity (inference). *Needs:* a timer source other than the 200 Hz tick
  (generic timer reprogrammed, or the BCM system timer channels INTID 96–99; whether
  the firmware uses some of the four channels is **not in any pinned source**,
  `docs/hardware/pwm-audio.md` §7.2/§10.7).
* **D — no notification; polling only.** `Setinterrupt` records the request and
  warns (ADR-031 "never silent"), or returns the STe-branch answer (S1: `cause ==
  1` → `0`, else `EBADRQ`), with the handler never called; programs use
  `Buffoper(-1)` / `Buffptr`. This is today's ADR-031 line for `Xbtimer`. *Keeps:*
  S1, S3–S5. *Loses:* S2 entirely — a program that needs the callback for gapless
  double buffering stalls or repeats. EmuTOS has a precedent for "interrupts are
  always enabled, nothing to set" only on the STe branch (`:467-475`); there the
  hardware still raises them.

**Delivery of a detected frame end to the program**

* **U1 — at the next syscall return** (what `post_upcall` does today,
  `tos-api/src/bdos/process.rs:1213@post_upcall`). No new kernel work. Latency is
  bounded only for a program that traps often. Double-buffer players usually do
  (they write to the device or poll), but that is a property of the program, not
  the contract.
* **U2 — at the return from the IRQ that detected it.** Needs the delivery check
  in the IRQ-return path that design §7.3 names as open
  (`docs/analysis/coop-kernel/10-upcall-signal-design.md` §7.3). Gives
  Falcon-like asynchrony. Open design points are reentrancy (the handler may call
  `Setbuffer`; the sound IRQ must not re-enter), the kernel operations suspended
  under the program (the exclusion list in `kernel/src/vbl.rs` module docs), and
  the FP/SIMD context.

**Observable-semantics matrix** (K = kept as read from source, k = kept by
inference, – = lost, ? = needs measurement)

| | S1 | S2 | S3 | S4 | S5 | S6 |
|---|:--:|:--:|:--:|:--:|:--:|:--:|
| A + U2 | K | k (IRQ latency) | ? | k (mapped) | k | K |
| A + U1 | K | k (until next trap) | ? | k | k | K |
| B + U1 | K | k (≤ 5 ms + next trap) | ? | k | k | K |
| C + U1/U2 | K | k | ? | k (FIFO granularity) | k | K |
| D | K | – | ? | k | k | K |

S3 is `?` in every row because it is a property of hardware nobody has measured in
our sources; the NewTOS implementation of `Buffoper` decides it, and the decision
has to match the oracle.

#### 9.1.4 Datasheet facts the options stand on (Track B)

Source: `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`, "BCM2711
ARM Peripherals", text re-extracted 2026-11-12. Edition-first citing as in
`docs/hardware/pwm-audio.md` §1 (chapter/table numbers, no page numbers).
Nothing here is from memory; each line names the table.

1. **A PWM FIFO word is one duty-cycle value, not a PCM sample.** `PWM_DATi`: "In
   PWM mode, data is sent by pulse width modulation: the value of this register
   defines the number of pulses which are sent within the period defined by
   `PWM_RNGi`" (BCM2711 Rel. 4 §8.6, Table 157); `FIF1` words are "used as data to be
   sent" for channels with `USEFi` (Table 158). The word range is therefore
   `0…RNG`, 32 bits per channel per sample, whatever the Falcon format. Falcon
   8-bit stereo is 2 bytes per frame; the FIFO needs 8 bytes per frame. A DMA source
   buffer must be a staging area of 32-bit words produced from the PCM by the CPU
   (scale to `RNG`, bias signed to unsigned, widen, interleave `L R`). **This
   corrects the mapping table in `docs/hardware/pwm-audio.md` §9**, which maps the
   `Setbuffer` start/end directly onto `SOURCE_AD`/`TXFR_LEN`. Circle's PWM sound
   confirms the staging design: its chunks are `u32` words and the chunk size is
   asserted against the Lite length limit (`references/circle/lib/sound/dmasoundbuffers.cpp:92-96`;
   tertiary, GPLv3 — read, not copied).
2. **The PWM block has no interrupt**: Chapter 8 (Tables 152–158) names none, and the
   VC interrupt list (Table 102, §6.2.4) has no PWM entry. DMA channel 0–6 are VC
   IRQ 16–22, "DMA 7 & 8" 23, "DMA 9 & 10" 24, "DMA 11"–"DMA 14" 25–28, "DMA 15"
   31 (Table 102). GIC INTID = 96 + VC IRQ (`docs/hardware/pwm-audio.md` §7.1,
   datasheet §6.3 "SPI IDs 96-159"; the kernel's PL011 INTID 153 = 96 + 57
   matches, `kernel/src/drivers/gic.rs` board table). So a full channel *n* in 0…6
   has INTID `112 + n`; Lite channels 7/8 share INTID 119 and 9/10 share 120.
3. **FIFO depth and slack.** The FIFO "can store up to sixty-four 32-bit words"
   (Rel. 4 §8.1) — stated under the serialiser bullet; that the PWM mode has the
   same depth is **inference** (as in `pwm-audio.md` §3.5). At 2 words per stereo
   frame that is 32 frames: `32 / rate` seconds between completion and silence
   (651 µs at 49 170 Hz, 5.09 ms at 6 285 Hz — arithmetic). When the FIFO runs empty
   the channel stops, `GAPO` is set (Table 154), and with `RPTL` forbidden for a
   shared FIFO (Table 158 text) the output drops to the idle level `SBIT`.
4. **DMA mechanics used by A and B.** Control block = 8 words, 256-bit aligned
   (Table 34); `NEXTCONBK = 0` stops the channel and clears `ACTIVE` (§4.2.1.1 text);
   `NEXTCONBK` may be rewritten only while the DMA is paused (Table 45 text) — so
   queueing a next frame is done by chaining blocks *ahead*, not by patching a
   running one. `TI.INTEN` (bit 0), `TI.PERMAP` (20:16), `TI.DEST_DREQ` (bit 6),
   `TI.SRC_INC` (bit 8) (Table 40). `CS.INT` bit 2 W1C (Table 38). `SOURCE_AD`
   "Updated by the DMA engine as the transfer progresses" (Table 41).
   `INT_STATUS` (Table 61) is read-only per the datasheet.
5. **Length limits.** On channels 0–6 `TXFR_LEN` is `XLENGTH` 16 bits plus `YLENGTH`
   in bits 29:16 that "in normal linear mode … becomes the top bits of the XLENGTH"
   (Table 43): 30 bits. On the **Lite channels 7–10 `TXFR_LEN` is 16 bits, "Length
   of transfer, in bytes. Updated by the DMA engine as the transfer progresses"**
   (Table 48): at most 65 535 bytes per control block. (`pwm-audio.md` §7.3 calls
   Lite "enough for an audio stream" without this limit; it fits, but one control
   block covers at most 16 383 words.)
6. **Pacing.** DREQ 1 = DSI0 / PWM1, selected by `PACTL_CS` bit 23
   `DMA_CNTRL_MUX_2` ("0 selects DSI0, 1 selects PWM1", reset `0x1`); DREQ 5 =
   PWM0 (§4.2.1.3). `TI.PERMAP = 0` is "continuous un-paced" (Table 40). The
   thresholds `DMAC.DREQ`/`PANIC` (default 7) say "threshold level … going active"
   without saying whether that is fill level or free space (Table 155; open item
   `pwm-audio.md` §10.3).
7. **Addresses and caches.** "The DMA controller must be set-up to use the Legacy
   Master addresses of the peripherals" (§4.1), so the `FIF1` target is
   `0x7E20_C818`; RAM is addressed through the legacy window
   `0xC000_0000…0xFFFF_FFFF` ("1G uncached range"), whose SDRAM page is `PAGE`
   (channels 0–6) / `PAGELITE` (7–10) of the `ENABLE` register, reset 0 (Table 62).
   **Not in any pinned source:** whether the A72's data caches need cleaning for a
   buffer the CPU wrote and the DMA reads (no ARM ARM in `references/`;
   `pwm-audio.md` §10.9). The kernel already does explicit `dc cvac` /
   `dc ivac` for the mailbox buffers (`kernel/src/drivers/bcm2711/mailbox.rs:55@clean_dcache`);
   that is the in-tree precedent, not evidence about the DMA's coherency.
8. **PWM clock.** The datasheet names `clk_pwm` ("nominally 100MHz, but can be
   varied by the clock manager", §8.1, §8.5) and documents no clock-manager register
   for it (full-text search for `PWMCTL`/`PWMDIV`/`CM_PWM` finds none;
   `pwm-audio.md` §10.1). Both A–C need it; this gap does not discriminate between
   them.

#### 9.1.5 Where this leaves Setinterrupt / Buffoper / Buffptr

* `Setinterrupt` can be built under every option; what differs is whether it does
  anything (S2). Under D it is an honest "recorded, never called" with the ADR-031
  warning.
* `Buffoper` and `Buffptr` are served by software state in every option; S3/S4 need
  the oracle measurement before the Developer fixes the rules.
* Option A with U2 comes closest to the Falcon; option B with U1 is the cheapest
  complete DMA design. The comparison for the jack (work, CPU cost, testability,
  sources) is in `TASK/SND01.md` § "Spec review"; the owner chooses.

### 9.2 Pointer width for `Setbuffer`/`Buffptr` — a real ABI decision

EmuTOS takes two `ULONG` and cuts them to 24 bits (`:386-393`). NewTOS runs LP64
for programs (the ILP32 app-ABI of ADR-020 is superseded; the loader refuses 32-bit
ELF, ADR-020 status line in `docs/MEMORY.md`). **Added 2026-11-12:** the SDK macros
the ported programs compile against already pass the `Setbuffer` addresses and the
`Buffptr` pointer as `long` (64-bit registers, §2), and the SDK's `SndBufPtr` is
`{ char *play; char *record; long reserve1; long reserve2; }`
(`sdk/libc/overlay/include/mint/falcon.h:409@SndBufPtr`), i.e. **32 bytes** under
LP64 against EmuTOS' 16. `docs/COMPATIBILITY.md:1960` ("Native vs m68k struct
variants") says a struct's pointer width follows the caller's ABI, so the native
variant is the consistent reading: the kernel writes `play` and `record` as 64-bit
values into the 32-byte structure and leaves `reserve1`/`reserve2` untouched.
**This is a reading of existing project rules, not a source fact — owner decision
(Q-N3).** The 24-bit truncation is an EmuTOS internal (§8) and is **not** to be
reproduced; what NewTOS does with an address that does not point into readable
memory of the caller is a separate contract point (EmuTOS does nothing, `:785`).

### 9.3 Sample format and byte order

§4.8: signed and interleaved is proved; the **byte order of the 16-bit values** is
not proved by EmuTOS. On m68k it is big-endian, and the SDK header's "big-endian"
on the extended modes (§4.8) points the same way; a native AArch64 LP64 producer
writes little-endian. **Open contract question (Q-N4)** — a wrong guess sounds like
noise, not like an error, hence the explicit mark. Note for the options: the PWM
path converts every sample on the CPU anyway (§9.1.4, point 1), so reading either
byte order costs one byte swap per sample, not an extra pass.

### 9.4 Recording

No input on the Pi: `Setbuffer(SR_RECORD, …)`, the record bits in `Buffoper`,
`SndBufPtr.record`, `Settracks(·, rec)` and `Soundcmd(ADCINPUT)` have nothing to
serve. EmuTOS' own answer for "no Falcon" is `EBADRQ` (`:374-375`, `:429-430`) —
the honest, precedent-backed choice.

### 9.5 DSP connection

`Dsptristate` (0x89) and `Gpio` (0x8A) configure the DSP connector. Without a DSP
there is nothing to switch; EmuTOS' non-Falcon answer is `EBADRQ` (`:526-527`,
`:550-551`). Accordingly `SND_DSP` (0x08) in the `_SND` cookie must **not** be set —
otherwise programs look for a DSP.

### 9.6 Rates

The Falcon rate table (§4.7) is tied to a 25.175 MHz clock. Which rates a PWM or HDMI
output on the Pi hits is **Track B** and expressly not in this spec
(`TASK/SND01.md` step 2, `docs/hardware/pwm-audio.md` §5.4-§5.5). A backend must
either hit exactly or resample; which of the two the hardware research decides.

### 9.7 `Locksnd` is advisory

§3/0x80: EmuTOS enforces nothing. A NewTOS that *enforces* the lock would be
"better" and still a deviation — programs that start playing without `Locksnd` work
on real TOS. **Do not enforce without noting it as a deliberate deviation.**

---

## 10. Cross references, corrections, deviation candidates

### 10.1 Method of the "no caller" statement

Sweep over the whole EmuTOS tree (`.c`/`.h`/`.S`, without `doc/` and object files)
for all 14 function names (word match), re-run 2026-11-12: hits only in
`bios/dmasound.c` (definition + `falcon_dmasound_init`), `bios/dmasound.h`,
`bios/xbios.c:1171-1240` (debug wrappers) and `bios/xbios.c:1500-1513` (table).

### 10.2 Correction to the ticket

`TASK/SND01.md:25` — `Buffoper` is 0x88, not 0x89 (§2). Already marked in the
ticket's own text (since 2026-09-29).

### 10.3 Sharpening of `_psg-group.md`

The closing sentence there ("a future NewTOS audio extension … at a reserved XBIOS
slot", `docs/api/xbios/_psg-group.md:109-110`) is superseded: playback has its
planned place at 0x80–0x8D. The PSG statement itself ("no YM2149") stays correct.

### 10.4 Conflict with the dispatcher profile

`docs/api/xbios/_dispatcher.md §4` fixes NewTOS at `LAST_ENTRY = 0x57`; the Falcon
030/TOS4 target profile and this group require `0x8D`. From outside the difference
is not observable (§1.4), but the spec should follow once the group is built.
**Not changed here** — another spec entry, belongs to a decision of its own.

### 10.5 FreeMiNT's GSXB is not usable evidence for semantics

`references/freemint/sys/gsxb.c` has **empty bodies** for 0x82…0x8D (`:263-334`) and
in `sys_gsxb_unlocksnd` an assignment instead of a comparison (`if (lock =
ENOLOCK)`, `:252`). Usable from it are only two structure statements: GSXB extends
`Soundcmd` by a third `long` argument (`:263`) and `Setinterrupt` by a handler
pointer (`:294`), and there is a `Waveplay` at 0xA5 (`:336`). **Unusable as a
semantics source.** **Added 2026-11-12:** the SDK's `falcon.h` overlay independently
shows the same signatures — `NSoundcmd(mode, data, data2)` and
`NSetinterrupt(src_inter, cause, inth_addr)` on 0x82 and 0x87 with a `long` third
argument (`sdk/libc/overlay/include/mint/falcon.h:422@NSoundcmd`,
`:435@NSetinterrupt`), documented as "only available with `SND_EXT`" (the `_SND`
bit 0x20). A NewTOS that does not set `SND_EXT` is not asked for them.

### 10.6 Deviation candidates for `docs/api/_discrepancies.md`

Deliberately **not** entered here (number allocation belongs to the lead;
`_discrepancies.md` is shared):

1. `_SND` comment against `_SND` constants in EmuTOS (§6).
2. `SNDNOTLOCK` -128 (EmuTOS) against `ESNDNOTLOCK` -130 (FreeMiNT) (§7).
3. `Buffoper`: write path not Falcon-gated, read path is (§3/0x88).
4. `Setmode(STEREO16)` reports success on STe/TT for an impossible mode (§3/0x84).
5. `Soundcmd(LTATTEN)` does not round-trip on the STe branch (§3/0x82).
6. `Devconnect` returns `1` in the Falcon branch, `0` in the STe branch — same
   success, two values (§3/0x8B).
7. (added 2026-11-12) `Sndstatus(1)`: FreeMiNT says interrupts and buffer operation
   are disabled by a reset, EmuTOS' register list does not include
   `$FF8900/01` (§3/0x8C).
8. (added 2026-11-12) EmuTOS comment "/160 = 50MHz" for a rate in kHz
   (`bios/dmasound.c:197`) (§4.7).

### 10.7 Compendium / Profibuch

Neither is in `references/` (`references/README.md:29-30` lists them as to be
acquired separately; present are `emutos`, `freemint`, `xgdos`, `tos4x`, `magicmac`,
`fvdi`, `datasheets`, `sd-spec`, `circle`, `usb`). The cross-check foreseen in
protocol step 3.8 is therefore **deferred**, as for `_psg-group.md`. `tos4x` is a
read-only comparison where a ticket names it (ADR-034); `TASK/SND01.md` does not,
so it was not consulted. Every "meaning" statement that does not come from EmuTOS
comes from FreeMiNT in this spec and is cited there individually.

---

## 11. NewTOS implementation notes

(Empty per protocol — filled at the implementation step. The open decisions are
collected in §9; they are decisions, not implementation notes.)
