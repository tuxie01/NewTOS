# DEVIATIONS — known differences to EmuTOS, looked up by trigger

A debugging aid (user 2026-11-07): when a program misbehaves, look here first for a known place where NewTOS
does something other than EmuTOS. Every row says **when** the difference shows (the trigger), **what** a
program sees on each side, and **where** it is decided or tracked. The reasoning lives there, not here — a
justification copied into two places drifts apart.

Rules:

- A row is added when a difference is found and **not** fixed in the same work unit: sanctioned
  (`docs/COMPATIBILITY.md`, an ADR), parked, or an open ticket. A fix removes its row in the same commit.
- Measured and read-only findings are marked: *measured* (oracle, probe or Pi run) or *read* (source only,
  a hypothesis).
- The status column links the ticket or the decision; no row without one.
- This list is not complete yet. `docs/COMPATIBILITY.md` ("Sanctioned deviations, one by one") holds more
  sanctioned differences than are indexed here; add them when one is touched.

## VDI

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Call with `CONTRL[1]` smaller than the points the handler reads, after a handler that writes its `PTSIN` (`vs_clip`, `vr_recfl`, `v_bar`, `v_rbox`, arcs, `polygon`, arrows, underline) | handler reads the **previous call's input** values from the local copy | handler reads what the **previous handler wrote** into the copy | measured, probe TPH (3 vs 9) | [V52](../TASK/V52.md) parked (P4, until the m68k thunk) |
| Call with negative `CONTRL[1]` | copy capped at 1024 pairs | copy loop runs 65536−\|n\| pairs past the buffer, over text scratch and contour-fill queue | read | sanctioned, `docs/COMPATIBILITY.md` (V23, V35) |
| Anything relying on the `PTSIN` copy sharing memory with text scratch or the contour-fill queue | separate buffers | one union (`include/vdiext.h:48-56`) | read | sanctioned, `docs/COMPATIBILITY.md` (V35) |
| AES drawing seen through the VDI (a hooked trap #2, the `PTSIN` copy) | AES draws straight through the raster backend, never through the VDI trap | AES draws through `gsx2` = the trap | read | side finding [V35](../TASK/V35.md), no ticket |
| `v_hardcopy` (escape 17) / `Scrdmp` | sets `dumpflg` to -1, prints nothing; `dump_vec` never called | VBL calls `dump_vec` when `dumpflg == 0` | measured (T18A) | ADR-031; [V34](../TASK/V34.md) |
| Program hooks a trap vector (`Setexc(34, …)` etc.) | vector stored, never called | called on every trap | measured ([PORT12](../TASK/PORT12.md) stage 1) | ADR-031 |
| `vst_effects` light (`F_LIGHT`) in truecolor | glyph cells lightened | glyph cells not lightened | measured | [V39](../TASK/V39.md) open |
| Skewed text in replace mode; `bold+skew+outline` | glyph damaged | correct | measured | [V41](../TASK/V41.md) open, cause known |
| More than 4096 characters in one call, unscaled 8x16 font | pen wraps | width check overflows `WORD`, direct blit runs on | measured | [V42](../TASK/V42.md) open (P4) |
| `vst_height` with clipping, also without effect | was: wholly clipped glyphs moved the DDA | skips `scale` for them | measured | [V45](../TASK/V45.md): ZG012 equal since [V31](../TASK/V31.md), rest of ZG013 is skew ([V41](../TASK/V41.md)); likely done, to be confirmed |
| `v_pmarker` after `vs_clip(0)` | markers clipped, workstation `clip` stays 0 | sets `vwk->clip = 1` and leaves it on | read | [V46](../TASK/V46.md) open |
| `vex_butv`/`vex_motv`/`vex_timv` during an AES session, before a `v_opnwk` | returns the default `just_rts`: the AES is called directly, not through the vectors | returns the AES's `far_bcha`/`far_mcha`/tick handler (`gsx_setmb`, `gsx_tick`); a program's own `v_opnwk` then cuts the AES off | measured (wkprobe W1, C1 "vector before" lines) | side finding [V53](../TASK/V53.md), no ticket |
| Mouse moved during one long system call (e.g. loading a program file), no AES/VDI pump inside it | pointer stays where it was until the call returns | pointer keeps moving (VBL draws during the call) | read | since INPUT01, `kernel/src/input_gate.rs` (IRQ pump and frame tick only over EL0); side finding of [V55](../TASK/V55.md), no ticket |
| `graf_mouse(USER_DEF)` with MFORM colours other than 0/1 or a hotspot outside 0..15 | mask pen 0, data pen 1, hotspot clamped | colours from `mf_bg`/`mf_fg`, hotspot `& 0x0f` | read | [A147](../TASK/A147.md) open, measure first |
| `v_opnwk` while the physical workstation is open | returns, virtual workstations stay open | hangs | measured (wkprobe W9) | sanctioned, `docs/COMPATIBILITY.md` (`v_opnwk` while the physical workstation is open, [SPEC04](../TASK/SPEC04.md)); the virtual slots EmuTOS clears on a returning `v_opnwk` are not covered, see [SPEC04](../TASK/SPEC04.md) (VDI-WK-006) |
| First `v_opnwk` of a session after an AUTO program left a virtual workstation open (no `v_clsvwk`) | the workstation stays open (`open_physical` resets only the physical slot) | closed (`vwk_ptr[2..] = NULL`, `vdi_control.c:545-546`) | read (PLAUSIBLE); measuring case designed | [V57](../TASK/V57.md) parked ([SPEC04](../TASK/SPEC04.md)) (VDI-WK-006), case not yet run |
| `v_opnwk` with `work_in[0]` = mode + 2 | no mode switch, `work_out` = running geometry | `Setscreen`/`VsetMode` to that mode | measured (`vdi-v18-t3.tsv` 13) | sanctioned, ADR-017 Rev. 2 and `docs/COMPATIBILITY.md` (`v_opnwk` with a video-mode request, [SPEC04](../TASK/SPEC04.md)) |
| `v_contourfill` whose seed queue never runs empty (smallest: 5x3 clip, dots at (3,1) and (1,2), seed (0,2), fill up to the boundary) | fill ends after 16 spans per screen pixel; what is drawn stays | loops for ever with IRQs masked, the machine stops | measured (oracle: EmuTOS does not return in 30000 VBL; NewTOS with the brake returns, QEMU ~3 s at 640x480) | sanctioned, `docs/COMPATIBILITY.md` ([V38](../TASK/V38.md)) |
| A program writes the screen memory `Physbase`/`Logbase` returns directly (no VDI), or sizes a buffer from `vq_extnd` `intout[4]` and `VgetSize`, **at the default 16-plane setting** | the VDI reports 16 planes (`kernel/src/vdi_backend.rs:44@PLANES`) while the memory is a linear 32-bpp XRGB8888 store (`kernel/src/drivers/bcm2711/framebuffer.rs:34@DEPTH_BPP`, `sdk/examples/physbase.c:19-25@XRGB8888`); `VgetSize` counts 32-bpp bytes. With `vdiplanes=32` the VDI reports 32 planes and the word is the store word (section "VDI at 32 planes" below) | Falcon truecolor profile: the screen memory is 16-bit words, `VgetSize` = `width * 2 * height` (`references/emutos/bios/videl.c:756`) | read (source only; no run compares the memory layout; `VgetSize` is measured, row below) | [V61](../TASK/V61.md) (owner decision 2026-11-18, option C), ADR-038: the 32-plane setting landed with [V62](../TASK/V62.md)-[V67](../TASK/V67.md) and removes the mismatch there; at 16 planes the row stays (ADR-024 point 3 calls the store internal) |

## VDI at 32 planes (`vdiplanes=32`)

Boot argument `vdiplanes=32` (ADR-038 point 2); at the default 16 planes none of these rows applies. EmuTOS has no
path above 16 planes (`references/emutos/vdi/vdi_defs.h:22`), so the EmuTOS column names what a program written for
the 16-plane or Falcon-truecolor machine expects, and the contract is ADR-038 (`docs/COMPATIBILITY.md`, "The VDI
device depth: 16 planes by default, 32 planes by a boot setting").

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| A program reads the depth from the Falcon mode word `VsetMode(-1)` (bits 2-0), from `Getrez` or from `VgetSize` | `0x001C` = 16-bit truecolor, `Getrez` 2, `VgetSize` the 32-bpp screen: the same as at 16 planes, beside `vq_extnd` `INTOUT[4]` = 32 and `vq_scrninfo` `[2]` = 32 | Falcon truecolor: word, planes and pixel size agree at 16 | read (the three calls do not read the depth, `docs/api/xbios/58-vsetmode.md`, `5b-vgetsize.md`, `04-getrez.md`; host test `tos-api/tests/xbios_v68_planes_setting.rs`, TASK/V68.md) | sanctioned, ADR-038 point 9 (owner choice O4), `docs/COMPATIBILITY.md`; [V68](../TASK/V68.md) |
| `vro_cpyfm`/`vrt_cpyfm` with a memory MFDB of 16 planes (any count 9..31), `vr_trnfm` alike | refused: silent no-op, `INTOUT`, `PTSOUT` and the destination untouched | 16-plane forms accepted (`references/emutos/vdi/vdi_raster.c:962-966`) | read (host tests `tos-api/tests/vdi_v65_memory_forms.rs`; 9..31 inferred, `docs/api/vdi/raster/_raster.md`) | sanctioned, ADR-038 point 5 (owner choice O5); a hardwired-16 program runs at the default or gets a patch |
| `PRESENT.NAP` (on the standard image) shows slides with `vdiplanes=32` | the slide show runs: font capture and frame blit use 32-plane memory forms (`sdk/apps/present/present.c:643,1148@fd_nplanes`); the frame is composed in 16-bit form and expanded at the blit, so the colours keep 16-bit quality (5 bits per gun, the same store pixels as at 16 planes), not 24-bit; any plane count but 16 or 32 gives an alert and a clean exit | n/a (no 32-plane setting; at 16 planes the same picture) | read (source; no run at 32 planes yet) | [V69](../TASK/V69.md) (owner 2026-11-19, option P2 of `docs/analysis/v69-programs-at-32-planes-2026-11-18.md` section 2.2; 24-bit slides, option P3, not chosen); [V65](../TASK/V65.md) note |
| TeraDesk's *Options > Video options* dialog (`DESKTOP.NAP`, on the standard image) | *Colours* shows 64K: the patch clamps the dialog's plane count to 16 (`sdk/overlays/teradesk/video.c:483@xd_nplanes`), so the dialog describes the Falcon mode word `0x001C` (16-bit truecolor, first row of this section), not the 32-plane VDI; the rest of the dialog as at 16 planes | n/a (no 32-plane setting; at 16 planes the dialog shows 64K) | measured (`measurements/measurement-2026-11-19-v69b-test2/RUNINFO.TXT`, results 3 and 4: at `vdiplanes=32` the patched dialog shows "Colours: 64K" and every frame equals the 16-plane run at 5 bits per gun; unpatched, "4194304K" overruns the Colours field); source and patch: `docs/analysis/v69-programs-at-32-planes-2026-11-18.md` section 2.1 | [V69](../TASK/V69.md) (owner 2026-11-19, option T1); showing the real mode: [XBIOS24](../TASK/XBIOS24.md) (parked) |
| HighWire (vendored upstream `sdk/apps/highwire/`, sources only) | not built: no build recipe; by the source its 32-bit raster functions write 8-byte `ULONG` pixels under LP64 (`raster_32`, `gscale_32`, `dither_32` and the `_32r`, `_32z` families, `sdk/apps/highwire/src/raster.c:1392-1515`; `ULONG` is `unsigned long`, `sdk/apps/highwire/src/hw-types.h:34`) | n/a (no NewTOS binary to compare) | read (source only, `docs/analysis/v69-programs-at-32-planes-2026-11-18.md` section 2.3; no binary, `measurements/measurement-2026-11-18-v66-test/RUNINFO.TXT`) | struck from [V69](../TASK/V69.md) (owner 2026-11-19); the `ULONG` note goes into a port ticket if one is filed |
| A program reads `v_get_pixel` as `INTOUT[1]` only, the 16-bit word | `INTOUT[1]` is the low word `0xGGBB`; `INTOUT[0]` carries `0x00RR` | `INTOUT[0] = 0`, `INTOUT[1]` the 16-bit word (`references/emutos/vdi/vdi_fill.c:1104-1110`) | read (host tests V65) | sanctioned, ADR-038 point 7 (owner choice O3) |

## XBIOS, keyboard, system variables

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Alt+Help | ordinary key, `dumpflg` unchanged | `dumpflg++` | read | [KEYS05](../TASK/KEYS05.md) (P4) |
| A child process reads `dumpflg` (`$4EE`) set by its parent | child starts with -1: the cell is per process (page 0) | one global cell | read | [KEYS05](../TASK/KEYS05.md) |
| A program polls `_frclock` (`$466`) or `_vbclock` (`$462`) without `Vsync`, or reads them after another process ran | written into the running process's own page 0 by the 200 Hz tick, by `Vsync` before it returns and at every switch-in: a frame boundary shows up to 5 ms late; another process's copy does not move while it is away | written by the VBL at the frame boundary; one global cell | read (host tests `tos-api/tests/bios20_sysvars.rs`) | sanctioned, owner 2026-11-20 (P1 with P3), [BIOS20](../TASK/BIOS20.md); `docs/COMPATIBILITY.md` ("System variables") |
| A program compares `_frclock` with `_vbclock` | equal while `_vblsem` stayed above 0: both start at 0 at boot | `_frclock` ahead by the VBLs of the boot and of each `Setscreen` mode change (`_vblsem` 0 there) | read (`docs/api/bios/_sysvars-counters-contract.md` section 2) | sanctioned, owner 2026-11-20 (C1, Q1 (c)), [BIOS20](../TASK/BIOS20.md) |
| A program counts frames to measure time (`_frclock`, `_vbclock`, `Vsync`) | 60 Hz on every board: the frame clock `Vsync` waits on | the VBL rate of the video mode: oracle 60.0 Hz on tt, about 50.5 Hz on falcontc (200 Hz `_hz_200` ticks per frame: 3.33 and 3.96) | measured (probe C09b, `measurements/measurement-2026-11-20-bios20-test/RUNINFO.TXT`) | sanctioned, owner 2026-11-20 (option C1), [BIOS20](../TASK/BIOS20.md) |
| A program writes `_vblsem` (`$452`) = 0 | `_vbclock` stops and the mouse pointer is not redrawn while it is at or below 0, for this process only | the whole VBL body stops: cursor blink, palette and screen-base load, `vb_draw`, the `vblqueue` | read | sanctioned, ADR-031 amendment 2026-11-20 item 4, [BIOS20](../TASK/BIOS20.md) |
| A program enters a routine into a `_vblqueue` slot (`$4CE`-`$4EA`), or reads slot 0 | never called, one warning per process; slot 0 reads 0 | called once per VBL; slot 0 holds `vb_draw` after the VDI is up (`vdi/vdi_mouse.c:602,631`) | read | sanctioned, ADR-031 amendment 2026-11-20 item 3, [BIOS20](../TASK/BIOS20.md) |
| A program dereferences `_v_bas_ad` (`$44E`) before it called `Logbase` or `Physbase` | bus error, the program ends: the cell holds the screen address, the screen is not mapped yet | the screen | read | sanctioned, owner 2026-11-20 (Q4 (a); mapping at the first access named as a later step), [BIOS20](../TASK/BIOS20.md) |
| A program writes `_v_bas_ad` to move VT52/VDI drawing | only its own page-0 copy changes; drawing stays where `Setscreen` put it | VT52 and the VDI draw at the new address | read (`docs/api/bios/_sysvars-counters-contract.md` section 8) | sanctioned, `docs/api/_super-state.md` S10 (the cell is output only), [BIOS20](../TASK/BIOS20.md) |
| A program reads `_themd` (`$48E`) | `{0, membot, memtop - membot, 0}`, the extent at load | read by a running program (field classes): tt the same classes as NewTOS (link 0, start `membot`, length `memtop - membot`, owner 0); falcontc the length differs from `memtop - membot`, the other three as tt | measured (probe C12, `measurements/measurement-2026-11-20-bios20-test/RUNINFO.TXT`) | sanctioned, owner 2026-11-20 (Q5 (b)), [BIOS20](../TASK/BIOS20.md); the falcontc length is measured, not explained |
| A program reads the sector buffer `_dskbufp` (`$4C6`) points at | its own 3072-byte buffer, zero at load; no NewTOS code reads or writes it | the BIOS disk code's buffer: the last boot or BPB sector it read | read | sanctioned, owner 2026-11-20 (Q6 (b)), [BIOS20](../TASK/BIOS20.md) |
| A program calls `_hdv_boot` (`$47A`) or `_hdv_init` (`$46A`) through the cell | `hdv_boot` returns 4, `hdv_init` returns at once | `blkdev_hdv_boot` loads and runs the boot sector (2, 3, 4 or no return); `flop_hdv_init` re-detects the floppies | read | sanctioned, ADR-031 amendment 2026-11-20 item 1 (Q8), [BIOS20](../TASK/BIOS20.md) |
| A program reads `_pun_ptr` (`$516`) | 0: no AHDI table | pointer to EmuTOS's `PUN_INFO` (`'AHDI'`, version `0x300`) | read | [BIOS18](../TASK/BIOS18.md) (Q7 of [BIOS20](../TASK/BIOS20.md)) |
| `VsetMode(mode)` with `mode != -1`, `Vfixmode(mode)` | the function number (`0x58`, `0x5F`); no mode switch, `VsetMode(-1)` stays `0x001C` | Falcon: the word fixed up for the monitor, the Videl programmed, the old word returned | measured (`bios-r1` 215, `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`) | sanctioned, ADR-017 and `docs/COMPATIBILITY.md` ("XBIOS Falcon video", [XBIOS22](../TASK/XBIOS22.md)) |
| `VgetSize(mode)` | bytes of the current 32-bpp screen for every `mode` (`0x5B` before a framebuffer is up) | Falcon: bytes of the named Falcon mode (`VgetSize(0x1C)` 614400) | measured (`bios-r1` 213, same run) | sanctioned, `docs/COMPATIBILITY.md` ("XBIOS Falcon video", [XBIOS22](../TASK/XBIOS22.md)) |
| A program that reads `_VDO` = `0x00030000` (Falcon) and then accesses the Videl registers (`$FFFF82xx`) directly | bus error, the program ends (no Videl) | Falcon: the registers | read | ADR-012 amendment 2026-11-18 ([XBIOS22](../TASK/XBIOS22.md)); no in-repo program does it |
| `Settime` with impossible packed fields (month 0 or above 12, day 0 or above the month's days, hour above 23, minute above 59, second 60 or 62), read back with `Tgetdate`/`Tgettime` | month and day clamped, hour/minute/second carried into the date: a valid value (set 2001-02-03 25:00:00 reads 2001-02-04 01:00:00; set 2001-02-30 reads 2001-02-28; set `0x00000000` reads 1980-01-01 00:00:00) | the raw fields are kept and counted on by the tick (2001-02-03 25:00:00 reads back unchanged; 2001-02-30 unchanged; one tick later 23:59:62 reads 23:60:00; `0x00000000` reads 1980-00-00) | measured (probe T10-T28, `measurements/measurement-2026-10-28-xbios-system-audit/RUNINFO.TXT`; code `tos-api/src/gemdos/time.rs:210-221@xbios_settime`) | sanctioned, `docs/COMPATIBILITY.md` ("`Settime` with impossible fields", [XBIOS08](../TASK/XBIOS08.md), owner 2026-11-19, option B) |

## XBIOS DMA sound (0x80-0x8D)

EmuTOS column: the Falcon branch (`has_falcon_dmasound`, oracle profile `falcontc`), the machine NewTOS's `_SND`
value `0x14` announces. Decisions and reasons: `docs/COMPATIBILITY.md` ("XBIOS DMA sound 0x80-0x8D"), spec
`docs/api/xbios/_dmasound-group.md` §9 and §11.

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Any call 0x80-0x8D, or `Getcookie(_SND)`, on a machine without a sound backend (QEMU `raspi4b`, `aarch64-virt`, a Pi whose firmware reports no PWM clock) | the function number; `_SND` absent | oracle (tt, falcontc): the chip's answers, `_SND` `0x03` / `0x1F`; EmuTOS without a chip: the function number | measured (`bios-r1` 230-235, `measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`) | sanctioned, ADR-012 amendment 2026-11-12 ([SND05](../TASK/SND05.md)) |
| Program waits for the frame-end interrupt (`Setinterrupt` with a cause, handler through `Xbtimer`/`Mfpint`/`Jenabint`) | request stored, `0`; the handler is never called; one warning per process | handler runs once per frame pass (falcontc: at the start of each pass) | read (EmuTOS side measured, spec §9.1.6) | sanctioned, ADR-031, SND01 Q-N2 (option D) |
| Recording: `Setbuffer(1, …)`, `Buffoper` with bit 2 or 3 | `EBADRQ`; `SndBufPtr.record` reads `0` | `0`, the record bank is used | read | sanctioned, spec §9.4 |
| `Dsptristate`, `Gpio` | `EBADRQ` | `0` / the port bits | read | sanctioned, spec §9.5 |
| `Buffptr(ptr)` | `play`, `record` as 64-bit values at offsets 0 and 8 of a 32-byte `SndBufPtr`; full 64-bit addresses | two 4-byte `LONG`s in 16 bytes, 24-bit frame counter | read | sanctioned, spec §9.2 (Q-N3) |
| `Soundcmd(9, …)` | NewTOS byte-order switch for 16-bit samples: `0x05` default, `0x09` little-endian | `0`, nothing set | read | sanctioned extension, spec §9.3.1 (Q-N4a-c) |
| `Settracks(play > 0, …)`, `Setmontracks(n)`; a buffer with 2-4 interleaved tracks | stored and accepted as on the Falcon, not applied: the buffer is read as one `L R` track, so the interleaved tracks play one after the other, sample pair by sample pair | the frame holds `play + 1` tracks; the monitor track goes to the DAC | read | [SND05](../TASK/SND05.md), spec §11 |
| `Soundcmd(0..3, …)` (attenuation, gain) | stored and read back, not applied to the output | applied by the CODEC | read | [SND05](../TASK/SND05.md), spec §11 |
| `Devconnect` routing (`src`, `dst`, `protocol`) | the matrix is not modelled: DMA playback always reaches the jack; return values as on the Falcon | crossbar routes the sources | read | [SND05](../TASK/SND05.md), spec §11 |
| `Devconnect(…, clk = 2, …)` (32 MHz), `clk = 1` (external), prescale 6, 8, 10, 12-15 (invalid for the CODEC) | `clk` 2: the prescale counts on the 25.175 MHz formula; `clk` 1: the prescale is stored, the rate does not change; the invalid prescales play at `25 175 000 / (256 (p + 1))` Hz | the CODEC's clock and divider | read | owner decision 2026-11-17, spec §9.6 |
| `Setmode` while a frame plays | the new sample mode applies from the next pass | not in any pinned source | read | owner decision 2026-11-17, spec §11 |
| `Setbuffer` frame outside GEMDOS memory | plays as silence at its length; pointer and end bit as for a real frame | DMA reads whatever lies there | read | [SND05](../TASK/SND05.md), spec §11 |
| `Buffptr` after a single-shot frame or a stop (tt profile) | falcontc model: `end`, resp. frozen at the stop | tt: `start` | read (oracle measured, spec §9.1.6) | [SND05](../TASK/SND05.md): NewTOS claims the Falcon |

## GEMDOS, file system, SD card

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Second `Fopen` on a file that is open and growing | length 0, `Fread` 0; `Fsfirst` after closing the second handle 0 | shares the `DFD`: current length | measured | [FS33](../TASK/FS33.md) open |
| `Fclose` whose write-back fails | handle freed | handle stays allocated | measured (QEMU injection) | [HW08](../TASK/HW08.md) point (a), user decision open; named in `docs/COMPATIBILITY.md` |
| Card rejects a data sector | -10 reported, but clusters stay allocated and the file has 0 bytes; `Dcreate` keeps its entry | cross-linked clusters (worse) | measured (QEMU injection) | [FS34](../TASK/FS34.md) open |
| Card write time-out | -2 (`EDRVNR`), the call is not restarted, no card re-initialisation; no write-back retry limit | `E_CHNG`, card re-initialised, call restarted | measured / read | sanctioned, `docs/COMPATIBILITY.md` (HW08) |
| Process deletes a file it still has open (`Fdelete`, `Fcreate` over it, `Ddelete`) | handle stays valid but no longer reaches the card | keeps writing to clusters the FAT lists as free | read | sanctioned, `docs/COMPATIBILITY.md` (PROC04 point 5) |
| `Fcreate` with a name longer than 8.3 | refused | truncated | read ([PORT12](../TASK/PORT12.md) stage 1) | no ticket |

## AES

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| `_appl_yield` (trap selector 201), e.g. a busy loop waiting for an accessory | returns at once, no dispatch | calls `dsptch()` | read | [A143](../TASK/A143.md) open |
| Program mixes `graf_mouse(M_OFF/M_ON)` with `v_hide_c`/`v_show_c`, unbalanced | every `graf_mouse` goes to the VDI counter | AES counter `gl_moff`, VDI called only on 0↔1 | read | [A145](../TASK/A145.md) open |
