# DEVIATIONS — known differences to EmuTOS, looked up by trigger

A debugging aid (user 2026-11-07): when a program misbehaves, look here first for a known place where NewTOS
does something other than EmuTOS. Every row says **when** the difference shows (the trigger), **what** a
program sees on each side, and **where** it is decided or tracked. The reasoning lives there, not here — a
justification copied into two places drifts apart.

Rules:

- A row is added when a difference is found and **not** fixed in the same work unit: sanctioned
  (`docs/COMPATIBILITY.md`, an ADR), parked, or an open ticket. A fix removes its row in the same commit.
- Measured and read-only findings are marked: *gemessen* (oracle, probe or Pi run) or *gelesen* (source only,
  a hypothesis).
- The status column links the ticket or the decision; no row without one.
- This list is not complete yet. `docs/COMPATIBILITY.md` ("Sanctioned deviations, one by one") holds more
  sanctioned differences than are indexed here; add them when one is touched.

## VDI

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Call with `CONTRL[1]` smaller than the points the handler reads, after a handler that writes its `PTSIN` (`vs_clip`, `vr_recfl`, `v_bar`, `v_rbox`, arcs, `polygon`, arrows, underline) | handler reads the **previous call's input** values from the local copy | handler reads what the **previous handler wrote** into the copy | gemessen, probe TPH (3 vs 9) | [V52](../TASK/V52.md) geparkt (P4, until the m68k thunk) |
| Call with negative `CONTRL[1]` | copy capped at 1024 pairs | copy loop runs 65536−\|n\| pairs past the buffer, over text scratch and contour-fill queue | gelesen | sanctioned, `docs/COMPATIBILITY.md` (V23, V35) |
| Anything relying on the `PTSIN` copy sharing memory with text scratch or the contour-fill queue | separate buffers | one union (`include/vdiext.h:48-56`) | gelesen | sanctioned, `docs/COMPATIBILITY.md` (V35) |
| AES drawing seen through the VDI (a hooked trap #2, the `PTSIN` copy) | AES draws straight through the raster backend, never through the VDI trap | AES draws through `gsx2` = the trap | gelesen | side finding [V35](../TASK/V35.md), no ticket |
| `v_hardcopy` (escape 17) / `Scrdmp` | sets `dumpflg` to -1, prints nothing; `dump_vec` never called | VBL calls `dump_vec` when `dumpflg == 0` | gemessen (T18A) | ADR-031; [V34](../TASK/V34.md) |
| Program hooks a trap vector (`Setexc(34, …)` etc.) | vector stored, never called | called on every trap | gemessen ([PORT12](../TASK/PORT12.md) stage 1) | ADR-031 |
| `vst_effects` light (`F_LIGHT`) in truecolor | glyph cells lightened | glyph cells not lightened | gemessen | [V39](../TASK/V39.md) offen |
| Skewed text in replace mode; `bold+skew+outline` | glyph damaged | correct | gemessen | [V41](../TASK/V41.md) offen, cause known |
| More than 4096 characters in one call, unscaled 8x16 font | pen wraps | width check overflows `WORD`, direct blit runs on | gemessen | [V42](../TASK/V42.md) offen (P4) |
| `vst_height` with clipping, also without effect | was: wholly clipped glyphs moved the DDA | skips `scale` for them | gemessen | [V45](../TASK/V45.md): ZG012 equal since [V31](../TASK/V31.md), rest of ZG013 is skew ([V41](../TASK/V41.md)); likely done, to be confirmed |
| `v_pmarker` after `vs_clip(0)` | markers clipped, workstation `clip` stays 0 | sets `vwk->clip = 1` and leaves it on | gelesen | [V46](../TASK/V46.md) offen |
| `vex_butv`/`vex_motv`/`vex_timv` during an AES session, before a `v_opnwk` | returns the default `just_rts`: the AES is called directly, not through the vectors | returns the AES's `far_bcha`/`far_mcha`/tick handler (`gsx_setmb`, `gsx_tick`); a program's own `v_opnwk` then cuts the AES off | gemessen (wkprobe W1, C1 "vector before" lines) | side finding [V53](../TASK/V53.md), no ticket |
| `vsc_form` with the pointer visible (direct, not via `graf_mouse`) | new form/hotspot on screen at once | old image until the next redraw | gemessen (wkprobe F1, F2) | [V54](../TASK/V54.md) offen |
| Reading the screen right after a mouse packet (before the next frame) | pointer already at the new position | still at the old one, `vq_mouse` already new | gemessen (wkprobe M1, M2) | [V55](../TASK/V55.md), user decision open |
| `v_opnwk` while the physical workstation is open | returns, virtual workstations stay open | hangs | gemessen (wkprobe W9) | sanctioned, `docs/COMPATIBILITY.md` (`v_opnwk` while the physical workstation is open, [SPEC04](../TASK/SPEC04.md)); the virtual slots EmuTOS clears on a returning `v_opnwk` are not covered, see [SPEC04](../TASK/SPEC04.md) (VDI-WK-006) |
| First `v_opnwk` of a session after an AUTO program left a virtual workstation open (no `v_clsvwk`) | the workstation stays open (`open_physical` resets only the physical slot) | closed (`vwk_ptr[2..] = NULL`, `vdi_control.c:545-546`) | gelesen (PLAUSIBLE); measuring case designed | [V57](../TASK/V57.md) geparkt ([SPEC04](../TASK/SPEC04.md)) (VDI-WK-006), case not yet run |
| `v_opnwk` with `work_in[0]` = mode + 2 | no mode switch, `work_out` = running geometry | `Setscreen`/`VsetMode` to that mode | gemessen (`vdi-v18-t3.tsv` 13) | sanctioned, ADR-017 Rev. 2 and `docs/COMPATIBILITY.md` (`v_opnwk` with a video-mode request, [SPEC04](../TASK/SPEC04.md)) |
| `v_contourfill` whose seed queue never runs empty (smallest: 5x3 clip, dots at (3,1) and (1,2), seed (0,2), fill up to the boundary) | fill ends after 16 spans per screen pixel; what is drawn stays | loops for ever with IRQs masked, the machine stops | gemessen (oracle: EmuTOS does not return in 30000 VBL; NewTOS with the brake returns, QEMU ~3 s at 640x480) | sanctioned, `docs/COMPATIBILITY.md` ([V38](../TASK/V38.md)) |

## XBIOS, keyboard, system variables

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Alt+Help | ordinary key, `dumpflg` unchanged | `dumpflg++` | gelesen | [KEYS05](../TASK/KEYS05.md) (P4) |
| A child process reads `dumpflg` (`$4EE`) set by its parent | child starts with -1: the cell is per process (page 0) | one global cell | gelesen | [KEYS05](../TASK/KEYS05.md) |

## GEMDOS, file system, SD card

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| Second `Fopen` on a file that is open and growing | length 0, `Fread` 0; `Fsfirst` after closing the second handle 0 | shares the `DFD`: current length | gemessen | [FS33](../TASK/FS33.md) offen |
| `Fclose` whose write-back fails | handle freed | handle stays allocated | gemessen (QEMU injection) | [HW08](../TASK/HW08.md) point (a), user decision open; named in `docs/COMPATIBILITY.md` |
| Card rejects a data sector | -10 reported, but clusters stay allocated and the file has 0 bytes; `Dcreate` keeps its entry | cross-linked clusters (worse) | gemessen (QEMU injection) | [FS34](../TASK/FS34.md) offen |
| Card write time-out | -2 (`EDRVNR`), the call is not restarted, no card re-initialisation; no write-back retry limit | `E_CHNG`, card re-initialised, call restarted | gemessen / gelesen | sanctioned, `docs/COMPATIBILITY.md` (HW08) |
| Process deletes a file it still has open (`Fdelete`, `Fcreate` over it, `Ddelete`) | handle stays valid but no longer reaches the card | keeps writing to clusters the FAT lists as free | gelesen | sanctioned, `docs/COMPATIBILITY.md` (PROC04 point 5) |
| `Fcreate` with a name longer than 8.3 | refused | truncated | gelesen ([PORT12](../TASK/PORT12.md) stage 1) | no ticket |

## AES

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| `_appl_yield` (trap selector 201), e.g. a busy loop waiting for an accessory | returns at once, no dispatch | calls `dsptch()` | gelesen | [A143](../TASK/A143.md) offen |
| Program mixes `graf_mouse(M_OFF/M_ON)` with `v_hide_c`/`v_show_c`, unbalanced | every `graf_mouse` goes to the VDI counter | AES counter `gl_moff`, VDI called only on 0↔1 | gelesen | [A145](../TASK/A145.md) offen |
