# Line-A Overview

> Phase 00 deliverable D10. Captures *what Line-A is* and *which calls
> exist* on TOS. Source: EmuTOS @ `1a7484ab` — `bios/lineavars.S`,
> `bios/lineainit.c`, `bios/linea.S`.
>
> **NewTOS provides no Line-A** — ADR-030 (`docs/MEMORY.md`, Accepted
> 2026-10-26, user decision); see [NewTOS](#newtos-adr-030). The
> implementation strategy of the original D10 (a C-callable function group
> behind a header, a LINEA export) is dropped; the AES/desktop dependency it
> assumed does not exist at the pin (corrected 2026-10-26).

## What Line-A is

On the Atari ST, the m68k opcodes `0xA000..0xAFFF` are unimplemented
instructions that trap to vector `0x28` ("Line-1010 emulator"). EmuTOS
installs `int_linea` there (`references/emutos/bios/bios.c:176`, handler
`references/emutos/bios/linea.S:51`), which reads the low bits of the opcode
and dispatches to one of 16 graphics routines from a table
(`references/emutos/bios/linea.S:230-249`) — a side channel into the VDI's
pixel, blit and mouse routines, used by third-party programs.

```
 op=0xA000   →   init / get LINEA vars pointer
 op=0xA001   →   put_pixel
 op=0xA002   →   get_pixel
 op=0xA003   →   line
 op=0xA004   →   horizontal line
 op=0xA005   →   filled rectangle
 op=0xA006   →   filled polygon (one scan line per call)
 op=0xA007   →   bit-blit
 op=0xA008   →   text-blit (8-pixel monospaced glyph)
 op=0xA009   →   show mouse
 op=0xA00A   →   hide mouse
 op=0xA00B   →   transform mouse
 op=0xA00C   →   undraw sprite
 op=0xA00D   →   draw sprite
 op=0xA00E   →   copy raster (memory-to-screen, raster ops)
 op=0xA00F   →   seedfill / contourfill
```

The routines take register parameters (D0..D7 / A0..A2) and the shared
"LINEA variables" block (`v_planes`, `v_lin_wr`, `v_cel_wr`, mouse state,
fonts list, …). Op 0 returns the block's base in `a0`, the system font
headers in `a1` and the routine table in `a2`
(`references/emutos/bios/linea.S:156-163`). The block holds its fields at
fixed offsets around that base, e.g. `MOUSE_BT` at −596 and `V_REZ_HZ` at
−12, base at `line_a_vars` (`references/emutos/bios/lineavars.S:126`,
`:192`, `:266`).

## Why it exists / who uses it

1. **Speed.** The routines skip the VDI parameter-block setup and the
   trap-2 round trip; on a 7 MHz 68000 that matters.
2. **Third-party programs.** EmuTOS' own note: "Neochrome Master and old
   GFA programs use all of the lineA routines, and many (even new) freeware
   games still use bitblt" (`references/emutos/bios/linea.S:222-223`).

**Not** the EmuTOS AES or desktop. At the pin no file under `aes/` or
`desk/` issues a Line-A opcode; the AES hides and shows the mouse through
VDI opcodes (`gsx_moff`/`gsx_mon`, `references/emutos/aes/gemgsxif.c:440-455`).
The only reader of the variable block among them is EmuDESK, which includes
`lineavars.h` (`references/emutos/desk/desksupp.c:56`) and reads two fields:
`V_REZ_HZ` in `do_xyfix` (`references/emutos/desk/desksupp.c:267`) and
`MOUSE_BT` in `get_key` (`references/emutos/desk/desksupp.c:515-517`).
EmuCON reads `v_cel_mx`/`v_cel_my` through `$A000`
(`docs/analysis/line-a-readers-2026-10-26.md`, table 3). *(Corrected
2026-10-26: this section claimed the AES and desktop call show/hide mouse,
bit-blit and text-blit as Line-A.)*

## Source layout in EmuTOS

| File                       | Purpose                                  |
|----------------------------|------------------------------------------|
| `bios/lineavars.S`         | LINEA variables block (16-bit fields)    |
| `bios/lineainit.c`         | `linea_init()` populates pointers/fonts (`:50`) |
| `bios/linea.S`             | The 0xA000..0xA00F dispatch entry        |
| `vdi/vdi_blit.S`           | `bit_blt` (`:828`), shared with Line-A   |
| `vdi/vdi_control.c`        | `init_wk()` (`:269`), `update_rez_dependent()` (`:193`) — LINEA fields seeded here |
| `bios/bios.c:331`          | `linea_init()` called at boot            |

## NewTOS (ADR-030)

ADR-030 "No Line-A emulation" (`docs/MEMORY.md`, Accepted 2026-10-26):

- **No opcode trap.** AArch64 has no `$Axxx` trap, and NewTOS adds no
  substitute (no `svc` class, no undefined-instruction hook). `Setexc(10, …)`
  (vector `$28`) only stores into the software vector shadow, which nothing
  dispatches (`tos-api/src/bios/vectors.rs:98-109`). The m68k Line-A trap
  belongs to an m68k emulator, if one is ever built (FR-4).
- **No LINEA variable block** is exported to programs. The state the block
  would carry stays inside the VDI; programs reach it through the documented
  calls: `v_opnvwk`/`vq_extnd` `work_out` for the geometry (`DEV_TAB[0]` =
  `V_REZ_HZ − 1`, `references/emutos/vdi/vdi_control.c:203-204`), `vq_mouse`
  for buttons and position (`tos-api/src/vdi/mouse.rs:897-904`),
  `v_show_c`/`v_hide_c`, `vq_chcells`; the AES (`graf_mkstate`,
  `graf_handle`); the XBIOS.
- **No C function group, no `tos-linea.h`.** The SDK offers no Line-A
  bindings: mintlib's `linea.c` (`linea0()` … `lineaf()`) is excluded by
  ADR-030. A port that calls them does not link.
- **`__aline`, `__fonts`, `__funcs` stay null by design**
  (`sdk/libc/mintlib/mintlib/alglobal.c:7`, `:11`, `:16`). A reader of the
  named variables of `<mint/linea.h>` (`V_REZ_HZ`, `MOUSE_BT`, `V_CEL_MX`, …,
  negative offsets from `__aline`, `sdk/libc/mintlib/include/mint/linea.h:154-315`)
  addresses just below 2^64 and ends with an EL0 data abort
  (`kernel/src/arch/aarch64/exceptions.rs:303-309`). **Exception:** a reader of
  the `__LINEA` struct fields (`__aline->_VPLANES` and the MWC macros,
  `sdk/libc/mintlib/include/mint/linea.h:485`ff.) addresses null plus a small
  positive offset, i.e. page 0, which is mapped user read/write in every
  process (`tos-api/src/bdos/process/mmu.rs:643`); that access does not fault.
  No built program reads either form
  (`docs/analysis/line-a-readers-2026-10-26.md`).
- **The mouse cursor state is the VDI's** (`tos-api/src/vdi/mouse.rs:24-26`),
  reached only through VDI calls — ADR-030, Decision 3. This closes
  ADR-Line-A-02 below.
- **EmuDESK's two readers** are rebuilt in NEWDESK through VDI values
  ([DESK69](../../TASK/DESK69.md)).

The user-visible contract is in `docs/COMPATIBILITY.md` ("No Line-A").

## Closed questions

- **ADR-Line-A-01** (C ABI of the function-group export): moot — the export
  is dropped (ADR-030, Q2).
- **ADR-Line-A-02** (who owns the mouse cursor state): the VDI, behind its
  calls (ADR-030, Decision 3; `tos-api/src/vdi/mouse.rs`). ADR-011, named for
  it in `docs/archive/status-log-2026-10-26.md:1022`, does not mention the
  cursor.

## Cross-references

- ADR-030, ADR-011 (`docs/MEMORY.md`).
- `docs/analysis/line-a-readers-2026-10-26.md` — every Line-A reader in the
  ports, the NewTOS components and the reference.
- `docs/COMPATIBILITY.md` — "No Line-A".
