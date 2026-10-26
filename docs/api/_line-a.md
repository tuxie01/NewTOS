# Line-A Overview

> Phase 00 deliverable D10. Captures *what Line-A is*, *which calls
> exist*, and *which calls AES/VDI actually need*. Implementation is
> deferred to PHASE-08+ (display) and PHASE-12 (AES/VDI). Source:
> EmuTOS @ `1a7484ab` — `bios/lineavars.S`, `bios/lineainit.c`,
> `bios/linea.S`.

## What Line-A is

On the Atari ST, the m68k opcode `0xA000..0xAFFF` is illegal and
traps to vector 0x28 ("Line-1010 emulator"). TOS installs a handler
that reads the low 4 bits of the opcode and dispatches to one of
16 graphics primitives — a private side channel into the VDI's
fastest pixel/glyph routines, used by AES and the Desktop.

```
 op=0xA000   →   init / get LINEA vars pointer
 op=0xA001   →   put_pixel
 op=0xA002   →   get_pixel
 op=0xA003   →   line
 op=0xA004   →   horizontal line
 op=0xA005   →   filled rectangle
 op=0xA006   →   filled polygon
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

The handler reads register parameters (D0..D7 / A0..A2) and the
shared "LINEA variables" block (`v_planes`, `v_lin_wr`, `v_cel_wr`,
mouse pointer, fonts list, …). Every Line-A call returns through
the trap, restoring registers per m68k convention.

## Why it exists / why it matters

1. **Speed.** Line-A primitives are hand-written m68k assembly that
   bypasses the VDI parameter-block setup, the device-driver vector
   indirection, and the trap-1 round-trip. On a 7 MHz 68000 this
   matters: the AES desktop calls put_pixel hundreds of times per
   redraw.
2. **AES/Desktop dependency.** The desktop window manager, alert
   boxes, and menu rendering call Line-A directly (mostly mouse
   show/hide and text-blit) for performance.
3. **Third-party programs.** Many ST games and tools call Line-A
   instead of VDI. They will not run without it.

## Source layout in EmuTOS

| File                       | Purpose                                  |
|----------------------------|------------------------------------------|
| `bios/lineavars.S`         | LINEA variables block (16-bit fields)    |
| `bios/lineainit.c`         | `linea_init()` populates pointers/fonts  |
| `bios/linea.S`             | The 0xA000..0xA00F dispatch entry        |
| `vdi/vdi_blit.S`           | `bit_blt`
, shared with Line-A           |
| `vdi/vdi_main.c`           | `init_wk()` — LINEA fields seeded here   |
| `bios/bios.c:430`          | `linea_init()` called when LINEA enabled |

## NewTOS implementation strategy

NewTOS is AArch64; opcode 0xA000..0xAFFF cannot trap on ARM.
Line-A is therefore re-exposed as a **C-callable function group**
behind macros, so source-level ports of TOS programs link against
the NewTOS Line-A library and hit the same primitives.

- m68k user-emulator (PHASE-10) intercepts `0xA000..0xAFFF` opcodes
  in software, marshals registers into the C ABI, calls the same
  primitives. Binary compat preserved.
- Native (Rust/C) ports use a header `tos-linea.h` that maps
  `linea_put_pixel(...)` → `pub extern "C" fn linea_put_pixel(...)`.

## Required for AES/VDI (Phase 12)

The AES/Desktop calls only a small subset:

| Op       | Routine             | Used for                         |
|----------|---------------------|----------------------------------|
| 0xA000   | init / vars block    | initial setup, font loading      |
| 0xA009   | show_mouse           | every mouse-position update      |
| 0xA00A   | hide_mouse           | before any redraw under cursor   |
| 0xA007   | bit_blit             | window content drawing           |
| 0xA008   | text_blit            | text rendering everywhere        |

The remaining 11 ops are exercised by VDI, which AES calls into via
the standard VDI parameter block. They must exist for VDI to work,
but AES does not depend on them as Line-A.

## Required for VDI (Phase 12)

VDI internally calls into shared C primitives that Line-A also
exposes (`bit_blt`, `arb_corner`, `seedfill`, `cur_ms_stat`).
Implementing VDI implies implementing all Line-A primitives — the
two groups share ~90% of their code. The Line-A trap dispatch is a
thin shim around the same C functions.

## Required for Phase 09 source ports

The first source-port target (a TOS-native CLI utility, e.g.
`zoo` or `gzip`) does **not** use Line-A — it is a purely
character-mode program against GEMDOS. Line-A is **not on the
critical path** until Phase 12.

## Open questions / pending ADRs

- **ADR-Line-A-01:** C-ABI shape for the function-group export.
  Decision: pass an explicit `LINEA_VARS *` pointer rather than
  relying on a global. Rationale: SMP-friendly, easier to mock for
  tests.
- **ADR-Line-A-02:** Mouse cursor state ownership. Either Line-A
  owns it (`v_mwk` etc.) or the framebuffer compositor does. Defer
  to PHASE-08.

## Cross-references

- `docs/api/_dependency-graph.md` — Line-A nodes appear as
  inbound edges from AES.
- `docs/api/structures/` — pending: `LINEA_VARS` struct extraction
  (Phase 12 deliverable).
- `docs/STATUS.md` — Phase 12 entry.
