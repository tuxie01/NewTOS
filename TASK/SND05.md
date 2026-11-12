# SND05 — Sound: DMA ring, PWM jack backend and XBIOS 0x80-0x8D

Status: OPEN (2026-11-12) | Priority: P2

Type: developer. From [SND01](SND01.md), decisions 2026-11-12. Starts after [SND03](SND03.md) (measured clock,
FIFO, cache, channels) and [SND04](SND04.md) (oracle semantics); Q-N5 (`_SND`) and Q-N6 (rates) are decided from
SND03 first.

## Facts

- Spec: `docs/api/xbios/_dmasound-group.md` (incl. §9.1 end-of-buffer without MFP); hardware
  `docs/hardware/pwm-audio.md` (its `Setbuffer` -> `SOURCE_AD` mapping is wrong: the DMA source is a CPU-converted
  staging ring of duty-cycle words; DMA-lite channels move at most 65535 bytes per control block).
- Decisions: D-tick transport first; `Setinterrupt` recorded, never called, warning; native `SndBufPtr` layout;
  big-endian 16-bit samples by default, a switch for native little-endian samples (form settled in the spec first,
  SND01 Q-N4).

## Work

1. DMA driver (BCM2711 datasheet chapter 4): channel from the SND03 H4 result, control-block ring, cache handling
   per H3.
2. PWM jack backend: clock per H1, GPIO 40/41 ALT0, conversion of 8/16-bit big-endian samples into duty-cycle words
   in the 200 Hz tick.
3. XBIOS 0x80-0x8D per spec; `_SND` cookie per the ADR-012 amendment (owner decision on the draft in SND01).
4. Host tests per function (documented return, documented error); oracle cases of SND04 equal or listed in a
   COMPATIBILITY entry.

## Acceptance

- A program plays a buffer audibly at the jack on the real Pi (owner); SND04 cases equal or accepted; `bios-r1`
  230-235 equal or accepted; `cargo xtask test` green; warnings delta 0; doc-check 0 failures.

## Spec step done (researcher, 2026-11-12)

The spec that must exist before the developer starts (CLAUDE.md rules 1, 5, 6) is written; nothing of it is code.

- Q-N4 form of the byte-order switch: `docs/api/xbios/_dmasound-group.md` §9.3.1, **a draft for owner approval**
  (Q-N4a form, Q-N4b lifetime, Q-N4c latching): approved by the owner 2026-11-12 with the whole spec step
  ("ich stimme allem so zu wie du vorgeschlagen hast", I agree to everything as you proposed); the Falcon prescale
  rates are served too (SND01 Q-N6 addendum).
- The wrong `Setbuffer` -> `SOURCE_AD` mapping is corrected in `docs/hardware/pwm-audio.md` §9 (the DMA reads a
  CPU-converted staging ring, never the program's buffer).
- Hardware implementation contract: `docs/hardware/pwm-audio.md` §12 (clock, rates, GPIO/`PACTL_CS`/PWM, channel and
  control blocks, cache, interrupt, ring size for the 200 Hz tick; each item labelled cited / measured / inferred /
  not measured). No run has exercised a PWM-paced DMA transfer, and the tone was never heard.

## ADR-012 amendment text (owner-approved 2026-11-12, to be applied in the commit creating the cookie)

Place: `docs/MEMORY.md`, ADR-012, directly after the paragraph "*Amendment 2026-10-28 (user decision on
TASK/ARCH05).* ..." and before "### Alternatives considered". The text is SND01 §5 "Draft for the owner" with the
draft marks removed and the date and decision filled in (owner: "ok machen wir so", SND01 "Decisions 2026-11-12",
Q-N5, after H1). Not applied here: `docs/MEMORY.md` is untouched.

> *Amendment 2026-11-12 (user decision on TASK/SND01, Q-N5; applied with TASK/SND05).* **`_SND` exists exactly when
> a sound backend has initialised** (the jack's PWM output, later HDMI); while none has, the cookie stays absent and
> XBIOS 0x80-0x8D answer their function number (`SOUND_IS_AVAILABLE` of `references/emutos/bios/dmasound.c:122-126`).
> Value: **`SND_16BIT | SND_MATRIX` = `0x00000014`**, set by the sound driver through `Setcookie` after its probe, not
> in `seed_defaults` (`tos-api/src/cookie.rs:223@seed_defaults`). EmuTOS builds the value in
> `references/emutos/bios/machine.c:409-447` (`0x01` PSG if the build has a YM2149, `0x02` with DMA sound, `0x04|0x10`
> with Falcon DMA sound, `0x08` with a DSP). Bits not set and why: `SND_PSG` (`0x01`) — no YM2149, `Dosound` and
> `Giaccess` stay plausible-zero no-ops; `SND_8BIT` (`0x02`) — programs read it as "the STe register image at
> `$FFFF8900` exists", NewTOS has none and does not emulate one (ADR-012), and such programs would fault;
> `SND_DSP` (`0x08`) — no DSP, XBIOS 0x60-0x7F answer their function numbers; `SND_EXT` (`0x20`, "Extended XBIOS
> routines (Milan, GSXB)", `sdk/libc/overlay/include/mint/falcon.h:179@SND_EXT`) — the GSXB extensions (`NSoundcmd`,
> `NSetinterrupt`, free sample rates, 24/32-bit formats) are not implemented. A program that reads `0x14` concludes:
> Falcon CODEC and connection-matrix XBIOS sound is present (`Devconnect`, `Soundcmd`, `Setmode` incl. 16-bit stereo,
> `Setbuffer`, `Buffoper`, `Buffptr`), no PSG, no DSP, no STe register image. It may attempt recording (no bit says
> "no input"): `Setbuffer(SR_RECORD, ...)` and the record bits answer `EBADRQ` (`docs/api/xbios/_dmasound-group.md`
> §9.4). `SND_16BIT` states API capability, not DAC resolution (the PWM jack resolves about 11 bit at 49 kHz,
> `docs/hardware/pwm-audio.md` §5.4, §5.5). The ADR-012 consequence "drop `_VDO`/`_SND`/`_FDC`/`_FPU`" and the
> amendment 2026-10-28 line "`_SND` ... stay absent" are superseded for a system with a sound backend; `_FDC`,
> `_VDO`, `_FPU`, `_CPU` stay as decided. `docs/COMPATIBILITY.md` row 49 (`_SND`) changes from `absent` to `0x14`
> with a sound backend, still ACCEPTED against the oracle (`0x03` / `0x1F`: no PSG, no 8-bit, no DSP); row 41 (the
> jar's tag list) gains `_SND`.

Notes for the commit that applies it (not part of the amendment text):

- The figure "about 11 bit" replaces SND01's "10-11 bit": at the measured `clk_pwm` of 107 143 019 Hz the resolution
  at 49 170 Hz is 11.09 bit (`docs/hardware/pwm-audio.md` §5.5, last table, arithmetic). The 10-11 bit was computed
  for 50-100 MHz.
- `docs/COMPATIBILITY.md:1636` (row 49) and the row-41 line (`:1630`, "5: `NTOS _MCH _IDT _AKP EdDI`" -> 6 with `_SND`)
  are lead-side doc edits; the selftest of the jar's contents (`kernel/src/selftest_cookie.rs`) must expect the sixth tag
  only when the sound backend initialised.
- If the owner approves Q-N4a (`Soundcmd(9, ...)`), add to the amendment: "The byte-order switch for 16-bit samples
  is not announced by the cookie (it would be `SND_EXT`); programs probe it by calling
  `Soundcmd(SET16BITFORMAT, SND_INQUIRE)`, which answers 0 on EmuTOS."
- ADR-031's row for `bell_hook`/`kcl_hook` ("no sound device yet", SND01 "Not done") changes with the same commit
  or a later one; not part of this amendment.

## Localization (lead's researcher, 2026-11-12)

Method: `cargo xtask deps <fn> --down` and `--up`, `cargo xtask deps --check-mapping` (run 2026-11-12: "1968 rows,
all resolve, every class is valid; every bios/, bdos/, vdi/, aes/ function has a row"). EmuTOS source:
`references/emutos/bios/dmasound.c`, vector table `references/emutos/bios/xbios.c:1500-1513`.

**Mapping rows: none missing.** All 22 functions of `bios/dmasound.c` have a row (`docs/analysis/deps/mapping.tsv:457-479`),
every one as `design` with the note "not implemented in NewTOS (XBIOS 0x80-0x8D fall to the dispatcher's default arm)".
That note becomes false with this ticket. Rows cannot name the new NewTOS symbols before they exist (a row naming a
missing symbol fails `--check-mapping`), so the **developer rewrites them in the implementing commit** (ratchet,
CLAUDE.md "Testing"):

| EmuTOS function (`bios/dmasound.c`) | `--down` set (inside the group) | New row |
|---|---|---|
| `locksnd` `:215`, `unlocksnd` `:230`, `setbuffer` `:369`, `setsndmode` `:401`, `settracks` `:424`, `setmontracks` `:441`, `setinterrupt` `:457`, `buffoper` `:499`, `buffptr` `:772`, `sndstatus` `:751` | none (leaf) | `mapped` to the function in the new `sound` module of `tos-api/src/xbios/` |
| `soundcmd` `:342` | `sndcmd_falcon` `:246`, `sndcmd_ste` `:308`, `write_microwire` `:152` | `soundcmd` `mapped`; `sndcmd_falcon` `mapped` (same module); `sndcmd_ste`, `write_microwire` stay `design` (no STe branch, no Microwire/LMC1992) with the reason text replaced |
| `devconnect` `:721` | `devconnect_falcon` `:590`, `devconnect_ste` `:691` | `devconnect` and `devconnect_falcon` `mapped`; `devconnect_ste` `design` (STe-only) |
| `dsptristate` `:521`, `gpio` `:545` | none | `mapped` (answers `EBADRQ`, spec §9.5) |
| `dmasound_init` `:200` | `falcon_dmasound_init` `:182`, `lmc1992_init` `:167`, `devconnect`, `setsndmode`, `soundcmd`, `write_microwire` | `design` with the new reason: the backend init (sound driver probe + `Setcookie`) replaces it; or `mapped` to the init function if it does the same register-image reset (the register image is the sound state of the new module) |
| `detect_dmasound` `:128` | 217 functions, all through `check_read_byte` -> `init_exc_vec` -> `dopanic` (bus-error probing of `$FF8900`); none useful | `design` ("no counterpart by design: no sound chip to probe by bus error; the backend's init decides availability" — hardware reason, no ADR) |

`--up`: the only callers are the vector table (`xbios_vecs[0x80..0x8d]`, `bios/xbios.c:1500-1513`, mapped to
the NewTOS XBIOS dispatcher) and `dmasound_init` <- `bios_init`/`biosmain`. `bios_init` has three rows (`mapping.tsv:295-297`,
one `unmapped` by design and two `mapped` halves); `biosmain` is `unmapped` by design (`mapping.tsv:298`). Those two are listed
by `deps --up` as unmapped in the touched layer and predate this ticket; the retest set is complete except for them
(`bios_init` and `biosmain`: "NewTOS boots through the kernel init", the reason is already in their rows).

**NewTOS places to change** (read 2026-11-12; every cite is current):

1. **XBIOS dispatch.** `tos-api/src/xbios/mod.rs:70@dispatch`: add arms 0x80-0x8D before the default arm
   (`:157@other`, today the function number); constants beside `tos-api/src/xbios/mod.rs:165@MFPINT`. New `sound` module
   beside `mfp.rs` in `tos-api/src/xbios/` (the sound state machine of spec §4 and §9.1: shadows for `Setbuffer`/`Buffoper`/
   `Soundcmd`/`Devconnect`, frame record, `Buffptr`). The `Setinterrupt` "recorded, never called, warning" pattern is
   `tos-api/src/xbios/mfp.rs:69@mfpint` (`note_running`, `Hook::Vector`, ADR-031). No range constant to raise in
   `tos-api/src/trap.rs` (it has no `LAST_ENTRY`); the spec/doc edit is `docs/api/xbios/_dispatcher.md` §4 (profile
   0x57 -> 0x8D, spec §1.4 and §10.4 of the dmasound group).
2. **Hardware access from tos-api.** `tos-api` has no hardware; it reaches the kernel through `tos_api::host::Host`
   (`tos-api/src/host.rs:16@Host`, e.g. `frame_clock` `:181@frame_clock`). The sound backend (queue a converted
   chunk, read the DMA position, start/stop) needs new methods on that trait or an equivalent registration; which
   one is a lead decision (not made here).
3. **Kernel drivers (new files).** `kernel/src/drivers/bcm2711/` has `emmc`, `framebuffer`, `mailbox`, `pl011`, `pm`,
   `clock_tags`, no DMA and no PWM. New: a DMA channel driver (control blocks, `CS`/`CONBLK_AD`, bus address,
   cache maintenance) and the PWM jack backend (`CTL`/`RNG`/`DMAC`, GPIO 40/41, conversion into the staging ring).
   Register values and sequence: `docs/hardware/pwm-audio.md` §12; the working probe code to copy facts (not code
   structure) from is `kernel/src/snd_probe.rs` (`dma_copy` `:556@dma_copy`, `pwm_off` `:786@pwm_off`, `pwm_start`
   `:881@pwm_start`), which stays a debug feature.
4. **Mailbox.** "Set clock state" has no wrapper (only `kernel/src/snd_probe.rs:195@TAG_SET_CLOCK_STATE`);
   `kernel/src/drivers/bcm2711/mailbox.rs:488@set_clock_rate` and `:468@get_clock_rate_measured` exist. The tag
   encoding is host-tested in `kernel/src/drivers/bcm2711/clock_tags.rs` (included by the xtask tests by path): add
   the state tag there with its test.
5. **Tick refill.** The 200 Hz tick is `kernel/src/drivers/timer.rs:154@handle_irq`, called from the tick branch of
   `kernel/src/drivers/gic.rs:203@rust_irq_handler`. **The refill must go in that branch (after the EOI, IRQs
   masked), not in `kernel/src/vbl.rs:97@irq_tick`:** `irq_tick` returns at once unless the tick interrupted a program
   at EL0 (`from_el0`) and `input_gate::frame_tick_allowed` agrees, so a tick that lands in EL1 or with an upcall
   in flight would skip the refill (read facts; whether such ticks are frequent is not measured). All user memory is
   one shared address space (VA = PA), so the tick can read the program's buffer in any process context
   (`docs/ARCHITECTURE.md:99-105`); page 0 is the only per-process mapping. D-irq (later): an INTID branch beside
   `BOARD_PL011_INTID` and `kernel/src/drivers/gic.rs:95@enable_spi`.
6. **User pointers.** `Buffptr(ptr)` writes `play`/`record` as 64-bit values into the program's 32-byte `SndBufPtr`
   (spec §9.2, Q-N3): `tos-api/src/uaccess.rs:198@put_user_u64`; a fault takes the existing crash path (Q-N3).
   `Setbuffer` addresses arrive as full 64-bit registers (spec §2) and are never dereferenced in the syscall, only by the
   converter.
7. **Cookie.** `tos-api/src/cookie.rs`: new tag constant beside `:27@MCH`; the sound driver calls `cookie::set` after
   its probe (`:104@set`); `seed_defaults` (`:223@seed_defaults`) stays as it is and its comment (`:218`) names
   `_SND` as set by the sound driver instead of "deliberately omitted". Apply the ADR-012 amendment above in this commit.
8. **Tests.** Host tests (tos-api, in-crate and `tos-api/tests/`): conversion (8/16-bit, both byte orders if Q-N4a is
   approved, mid-level and `RNG` clamp), ring and pointer arithmetic, frame records, the S1-S6 rules of spec §9.1.2 and
   the return value and the error value per function (CLAUDE.md "Testing"). QEMU: no PWM region (an access aborts),
   DMA memory-to-memory with `PERMAP = 0` works and raises INTID 117 (SND03 H5); the PWM-paced path is Pi-only. The
   cases to turn equal or accepted: `tosdump/cases/bios-r1.tsv:230-235@Soundcmd` (inquiries) and the SND04 probe
   `tosdump/probes/snprobe.c` (`measurements/measurement-2026-11-12-snd04/RUNINFO.TXT`).
9. **Docs edited by the commit** (doc-check clean before it): `docs/api/xbios/_dmasound-group.md` (§11 "NewTOS
   implementation notes" is still empty by protocol), `docs/api/xbios/_dispatcher.md` §4, `docs/COMPATIBILITY.md`
   (rows 41 and 49, `Soundcmd(9, ...)` as a NewTOS extension if approved, the `Locksnd` behaviour),
   `docs/DEVIATIONS.md` for each difference found and not fixed, `docs/STATUS.md`.

**Retest set (`--up`) for the touched layer:** every XBIOS call 0x80-0x8D through `xbiostrap`; the cookie jar
(`Getcookie`/`Setcookie`, `selftest_cookie`); the 200 Hz tick path (`rust_irq_handler`) and anything riding it
(frame tick, input pump, cursor blink) — the refill adds work in the tick.


## State when stopped (2026-11-12)

Developer (Opus, agent a9d05a81becfe1313), stopped by owner order before the work was finished. Work-in-progress
commit on branch `snd05`; the ticket stays OPEN (line 3 untouched), no receipt, nothing merged.

**Done (code, built, host-tested):**

- `tos-api/src/xbios/sound.rs` (new): XBIOS 0x80-0x8D as EmuTOS' Falcon branch once a backend is available, the
  function number before (`set_available`); spec §9.4 (record -> `EBADRQ` for `Setbuffer(1)` and the `Buffoper`
  record bits), §9.5 (`Dsptristate`/`Gpio` -> `EBADRQ`), §9.1 option D (`Setinterrupt` stores the bits, warns once
  per process as the MFP vector the frame end would use, `Hook::Vector(0x4D/0x4F)` with a new `Via::Setinterrupt`), §9.2 native `SndBufPtr` (play and
  record = 0 as u64), §9.3.1 `Soundcmd(9, ...)` (default 0x05, latched per frame at `Setbuffer`, reset at the
  setter's `Pterm`, hook in `pterm.rs`), boot state of spec §5. The converter: 2048-word ring = 1024 stereo slots,
  each slot carries a record (frame, address after the sample, start generation) so a state change rewrites the
  ring from 16 slots ahead of the DMA position; S3 (bit 0 drops when the engine reads the end slot), S4 falcontc
  (0 before the first start, `end` after a single-shot frame, frozen at a stop), S5 (Setbuffer during a loop pass
  takes effect at the next pass). RNG from the measured clock (STe rates for `freq_int` 0, else
  25 175 000 / (256 (p + 1))); checked against the 107 143 019 Hz table of `docs/hardware/pwm-audio.md` §5.5.
- `tos-api/src/xbios/sound/tests.rs` (new): 25 host tests, all pass (`cargo test -p tos-api --lib xbios::sound`).
- `tos-api/src/xbios/mod.rs` (dispatch arm), `cookie.rs` (`SND`, `SND_VALUE` = 0x14, seed comment),
  `bdos/process/usermem.rs` (`pool_kva`: non-faulting kernel alias for pool memory, used by the converter in the
  tick; a frame outside the BDOS pools plays as silence), `bdos/process/hooks.rs`, `bdos/process/pterm.rs`.
- Kernel: `kernel/src/drivers/bcm2711/dma.rs` (new, channel claim/start/`SOURCE_AD`, control block, bus address),
  `kernel/src/drivers/bcm2711/pwm_audio.rs` (new, backend: mailbox clock on + 100 MHz + measured rate, which is the
  QEMU detection (QEMU answers the measured tag with 0 bytes), GPIO 40/41 ALT0 RMW, `PACTL_CS` bit 23 RMW, PWM1
  off/CLRF/STA/DMAC 7/7, two cyclic CBs on channel 5 with PERMAP 1 | DEST_DREQ | SRC_INC | WAIT_RESP, ring clean
  after each write, `_SND` set via `cookie::set` after the probe; transport kept running once started),
  `clock_tags.rs` (+ "Set clock state" encode/decode with 2 tests), `mailbox.rs` (`set_clock_state`), `gic.rs`
  (refill in the tick branch after EOI, before `vbl::irq_tick`), `lib.rs` (init after the timer, not with
  `snd-probe`), `selftest_cookie.rs` (`_SND` = 0x14 iff backend up, else EERROR). New code sits where it shifts
  no anchored doc cite (`_SND` constants and `pub mod sound` at the file ends of `cookie.rs` / `xbios/mod.rs`).
- Builds: `cargo xtask build --board rpi4` and `--board qemu-virt` clean; warnings equal to the base `993e57b99`
  on both boards (tos-api 64, kernel 93; clippy tos-api 73, kernel 121): delta 0.
- QEMU raspi4b smoke boot (debug): boots to the desktop; log line `[sound] no PWM clock (firmware: set clock state
  Some(1), no measured rate) — no sound backend, _SND absent`. Log in the agent scratch dir only (no run dir).

**Not done:**

- Full `cargo xtask test` not run (only the sound test filter; the hooks change adds a NAMED entry, its tests not
  yet run).
- Docs of the commit: ADR-012 amendment in `docs/MEMORY.md`, `docs/COMPATIBILITY.md` rows 41/49 + the
  `Soundcmd(9)` extension + Buffptr layout / `Dsptristate`/`Gpio` deviations, `docs/DEVIATIONS.md` rows (tracks
  not used: always track 0; attenuation/gain stored, not applied; matrix not modelled; clocks other than 25.175 MHz
  treated as internal; CODEC-invalid prescales 6, 8, 10, 12-15 played at their formula rate), spec §11
  implementation notes, `docs/api/xbios/_dispatcher.md` §4 (0x8D), `docs/analysis/deps/mapping.tsv` rows for
  `bios/dmasound.c`. Anchors in docs citing `tos-api/src/cookie.rs` and `tos-api/src/xbios/mod.rs` may have
  shifted (doc-check not run).
- Native test program (440 Hz, then left/right), the developer run under `measurements/`, the snprobe run on
  NewTOS (in QEMU it can only show the function numbers again: no backend there).
- Ticket line 3 / receipt.

**Spec gaps found (decided in code, need lead/owner confirmation):**

1. §9.4 lists `Settracks(·, rec)` and `Soundcmd(ADCINPUT)` as "nothing to serve" without a concrete answer; the
   code keeps the Falcon behaviour (stored, no error) and answers `EBADRQ` only for `Setbuffer(1)` and the
   `Buffoper` record bits (the ADR-012 amendment text names exactly those two).
2. Falcon prescales "invalid for CODEC" (6, 8, 10, 12-15) and `clk` 1/2 have no NewTOS rule; code: formula rate,
   internal clock.
3. Mode (8/16 bit, mono) is latched per pass at pass start (only the byte order is specified, Q-N4c).
4. `Soundcmd(9, 0x08)` stores and returns 0x09 (signed bit always set; spec item 1 says "optionally or-ed").
5. `Sndstatus` returns 0 (no CODEC status source on the PWM path).
