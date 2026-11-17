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

## XBIOS, keyboard, system variables

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Alt+Help | ordinary key, `dumpflg` unchanged | `dumpflg++` | read | [KEYS05](../TASK/KEYS05.md) (P4) |
| A child process reads `dumpflg` (`$4EE`) set by its parent | child starts with -1: the cell is per process (page 0) | one global cell | read | [KEYS05](../TASK/KEYS05.md) |

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
