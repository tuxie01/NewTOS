# What a program needs to read the visible screen (PITEST03 spec gate)

Date 2026-11-12. Researcher, ticket PITEST03. Scope: the values a probe that saves
the framebuffer reads, where each comes from on NewTOS, and its mapping on `raspi4b`
and `qemu-virt`. Read in this run: the five named spec entries, `docs/api/gemdos/154-ssystem.md`,
`docs/api/_line-a.md`, `docs/api/vdi/eddi-vq_scrninfo.md`, and the NewTOS code cited below.
EmuTOS is the reference of the XBIOS entries (see them); the Falcon calls named here are
*not* implemented on NewTOS, so EmuTOS cites are by way of the spec entries.

## Verdict: GAPS (one value has no program-readable source: bytes per line)

## Value table

| Value the probe reads | Source a program can use | Cited spec entry | NewTOS mapping |
|---|---|---|---|
| Base address | `Physbase` (XBIOS 2); `Logbase` (3) is the draw page | `docs/api/xbios/02-physbase.md` (Pi binding) | EL0 gets the user address of the screen's own physical address (VA = PA, uncached); `Host::screen().phys_base`, `ScreenInfo`. `0` before the framebuffer is up. |
| Width, height (px) | `Ssystem(S_NEWTOS_SCRWIDTH 2013 / SCRHEIGHT 2014)`, or VDI `v_opnwk`/`v_opnvwk` `work_out[0]+1`, `work_out[1]+1` | `docs/api/gemdos/154-ssystem.md:357-358`; `docs/api/vdi/workstation/01-v_opnwk.md` | `Ssystem` reads `Host::screen().width/height` (`tos-api/src/gemdos/ssystem.rs:186@S_NEWTOS_SCRWIDTH`); the VDI sets `dev[0] = xres - 1` from the backend width (`tos-api/src/vdi/workstation.rs:224@dev`). Both come from the same `active_fb()` (`kernel/src/vdi_backend.rs:1038@dims`). Value is config.txt-driven; default and `qemu-virt` 1920x1080 (`kernel/src/drivers/mod.rs:31@BOOT_WIDTH`); observed on QEMU `raspi4b` 640x480 (`02-physbase.md`, "Geometry"). The probe must read it, not assume it. |
| Bits per pixel of the screen memory | `Ssystem(S_NEWTOS_SCRDEPTH 2015)` = 32 | `154-ssystem.md:359` ("the framebuffer, not the VDI workstation's colour model") | `bpp: 32` constant (`kernel/src/host_env.rs:301@bpp`). |
| Bytes per line (pitch) | **none** | - | **GAP.** `Host::screen().stride_bytes` exists (`tos-api/src/host.rs:576@stride_bytes`) but no trap, `Ssystem` key, cookie or VDI value exposes it (`154-ssystem.md` lists only 2013-2015 for the display; `eddi.rs` reports `[5]` line width = 0, `tos-api/src/vdi/eddi.rs:120@w`; no Line-A, ADR-030, `docs/api/_line-a.md:82-90`). |
| Pixel layout / byte order | spec text only | `02-physbase.md` ("Format"); `kernel/src/hal/framebuffer.rs:36@rgb` | XRGB8888, one 32-bit little-endian word per pixel `0xXX_RR_GG_BB`, memory bytes `[B, G, R, X]`. The X byte is **not a colour**: the VDI sets it to `0xFF` when the overlay bit is set (same module doc); a PNG writer must ignore it. Same on both boards: `qemu-virt` ramfb fourcc XRGB8888 (`kernel/src/drivers/qemu_virt/ramfb.rs:189@cfg`); `raspi4b` depth 32, pixel order BGR, alpha ignored (`kernel/src/drivers/bcm2711/framebuffer.rs:253@PIXEL_ORDER_BGR`). |
| Palette for indexed modes | not applicable | `docs/api/vdi/eddi-vq_scrninfo.md` (3.5); `docs/MEMORY.md` ADR-024 | There is no indexed screen: the memory is direct colour. The VDI palette (256 pens) is converted at draw time, so the framebuffer holds final colours. A saved image needs no palette. |
| Screen pages | none | `02-physbase.md`, `05-setscreen.md` ("Screen pages available") | `raspi4b` 2 pages (virtual height doubled, `WANTED_PAGES`), `qemu-virt` 1. `Physbase` is the page being scanned out; the saved image is that page. `Ssystem` does not give the page count. |

## Calls that do not answer (what a program that tries them sees)

- `Getrez` (4) returns the constant `TT_MEDIUM` = 4 (`tos-api/src/xbios/screen.rs:164@getrez`);
  it carries no geometry or depth (`docs/api/xbios/04-getrez.md`, "NewTOS behavior").
  Cited, but under review (TASK/XBIOS23); the probe must not use it.
- `Vsetmode`/`VsetScreen`/`VgetMonitor` (0x58, 0x5C, 0x59) and `VgetSize` (0x5B): all above
  the last implemented id, so they return the function number (`tos-api/src/xbios/mod.rs:157@other`;
  `docs/api/xbios/5b-vgetsize.md`, `59-vgetmonitor.md`: DIFF to the oracle, owner options open).
  Even if implemented, a Falcon mode word has no 1920-wide entry
  (`5b-vgetsize.md`, edge case 1) and `VgetSize` option B would answer the 32-bpp byte count.
- `vq_extnd` / `v_opnwk` `work_out` colour fields describe the **VDI device** (16 planes,
  RGB565-like word, ADR-024), not the memory behind `Physbase`: do not derive the memory
  layout from them. `vq_scrninfo` (EdDI) reports format 2 (packed), 16 planes and `[5]`, `[6/7]` = 0 (`tos-api/src/vdi/eddi.rs:120@w`).
- Line-A: not provided (ADR-030, `docs/api/_line-a.md:82`).

## What the probe can do today

Read base (`Physbase`), width and height (`Ssystem` 2013/2014 or `work_out`), and the depth
(`Ssystem` 2015 = 32). The pitch it cannot read. Observable facts that bound it:
`qemu-virt` pitch = `WIDTH * 4` (`kernel/src/drivers/qemu_virt/ramfb.rs:193@cfg`);
`raspi4b` pitch is whatever the firmware reports (`kernel/src/drivers/bcm2711/framebuffer.rs:596@stride_px`;
observed 2560 B at 640 wide, `02-physbase.md`), so `width * 4` there is **observed, not
guaranteed** (`tos-api/src/host.rs:574@stride_bytes` states the pitch may exceed it). Until a pitch source exists a
probe that assumes `width * 4` is correct only where that holds; it must say so in its
RUNINFO. Options for the owner (no decision here): (a) a new `Ssystem` key
`S_NEWTOS_SCRPITCH` (NewTOS extension, FreeMiNT-style, needs an ADR/ticket and a
`154-ssystem.md` row); (b) the probe checks `width * 4 * height` against what
`Physbase` spans on both boards and stops otherwise.

## Corrections made

- `docs/api/xbios/02-physbase.md`: the line "The 640x480 geometry is hard-coded today"
  ("1080p later") contradicted the code (`BOOT_WIDTH` 1920, config.txt-driven geometry, `Ssystem`
  2013/2014). Replaced by a pointer to this table.

## Open points

- STOP: bytes per line has no program-readable source (the one GAP; needs an owner decision, options above).
- Inferred: that the `raspi4b` pitch equals `width * 4` for the modes in use rests on one
  observation (640x480, 2560 B), not on a rule; the firmware pitch tag decides.
- Inferred: that the QMP screendump equals the saved image after dropping the X byte (a
  measurement for the tester, not a reading).
- Out of scope: `Getrez` (TASK/XBIOS23) and `VgetSize`/`VgetMonitor` mapping choices are owner questions already open; `eddi-vq_scrninfo.md` is still German (translate when touched).
