# `v_opnwk` — VDI opcode 1 (open physical workstation)

> Phase 00b spec (full-contract template). Source: EmuTOS @ `1a7484ab` —
> `vdi/vdi_control.c:496` (`vdi_v_opnwk`). Shares `init_wk` and the
> `work_in`/`work_out` layout with
> [`100-v_opnvwk`](100-v_opnvwk.md). Calling convention:
> [`../_dispatcher.md`](../_dispatcher.md).
>
> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> **Contract:** opcode 1, `work_in[0] = rez+2` semantics, the returned
> handle `VDI_PHYS_HANDLE(1)`, and the same `DEV_TAB`/`SIZ_TAB` output as
> `v_opnvwk`. **Reference:** the `Setscreen` resolution switch, `phys_work`
> global, `init_colors`/`timer_init`/`vdimouse_init`/`esc_init` bring-up
> sequence — NewTOS reimplements these against the framebuffer.

Opens the one **physical** workstation — the screen device itself. GEM
calls this once at startup (via the AES); applications then open
**virtual** workstations on top with `v_opnvwk`. It optionally switches
screen resolution, (re)loads the device capability tables, initialises
the palette/mouse/timer/escape subsystems, and initialises the physical
workstation's attribute area.

## EmuTOS source

**Primary location:** `vdi/vdi_control.c:496` — `vdi_v_opnwk(Vwk *vwk)`
(reached via `jmptb1[0]`, `vdi_main.c:33`); called with `vwk == NULL`
(the dispatcher recovers `vwk = CUR_WORK` afterward, `vdi_main.c:168-171`).

**Helpers called:**
- `Getrez()`/`Setscreen()` (XBIOS) — read/change resolution (`:513-514`).
- `vdi/vdi_control.c:193` — `update_rez_dependent()` — patch
  `DEV_TAB[0,1,3,4,13,35,39]` and `INQ_TAB[4,5]` from the live geometry.
- `init_colors()` (`vdi_col.c`) — initialise the palette / `MAP_COL[]`.
- `text_init()` — fill the `SIZ_TAB` font-metric entries.
- `vdi/vdi_control.c:269` — `init_wk()` — attribute defaults + copy
  `DEV_TAB`→`INTOUT`, `SIZ_TAB`→`PTSOUT`.
- `timer_init()`, `vdimouse_init()` — timer + mouse subsystems.
- `esc_init(vwk)` (`vdi_esc.c:356` → `escfn2`) — enter graphics mode.

**Headers / type definitions used:**
- `vdi/vdi_control.c:27` — `VDI_PHYS_HANDLE = FIRST_VDI_HANDLE = 1`.
- resolution constants `ST_LOW/ST_MEDIUM/ST_HIGH`, `TT_LOW/TT_MEDIUM`
  (`CONF_WITH_TT_SHIFTER`), `FALCON_REZ` (`CONF_WITH_VIDEL`).

**System variables / globals read:** `INTIN[0]` (rez+2), `Getrez()`,
`V_REZ_HZ`/`V_REZ_VT`/`v_planes`; `INTOUT[45]` **as input** on the Videl
path (`:521`). **Written:** `SIZ_TAB`/`DEV_TAB`/`INQ_TAB` (reloaded from
ROM then patched), `vwk_ptr[VDI_PHYS_HANDLE]=&phys_work`, `CONTRL[6]`,
`vwk_ptr[2..]=NULL`, `line_cw=-1`, `CUR_WORK`.

**Build configuration affecting this function:**
- `CONF_WITH_TT_SHIFTER` — allows `newrez` = `TT_LOW`/`TT_MEDIUM`
  (`:509-511`).
- `CONF_WITH_VIDEL` — allows `newrez == FALCON_REZ`; the new Videl mode is
  passed in **`INTOUT[45]`** (sic — an *out* array used as input, an Atari
  TOS 4 quirk) (`:517-525`).
- `CONF_WITH_VDI_16BIT`, `HAVE_BEZIER` — as for `v_opnvwk`.

---

## Source extraction summary

`v_opnwk` computes `newrez = INTIN[0] - 2` (the classic Atari
"resolution + 2" convention). If `newrez` is a recognised ST (or TT/Videl)
mode **and** differs from the current `Getrez()`, it calls `Setscreen` to
switch modes. It then reloads the 12-word `SIZ_TAB` and the 45-word
`DEV_TAB`/`INQ_TAB` from their ROM templates and calls
`update_rez_dependent` to patch the geometry/colour entries from the live
`V_REZ_HZ`/`V_REZ_VT`/`v_planes`. It installs `&phys_work` as
`vwk_ptr[VDI_PHYS_HANDLE]`, returns the fixed handle `VDI_PHYS_HANDLE(1)`
in `CONTRL[6]`, clears all virtual handle slots, invalidates the cached
line width, and initialises the palette (`init_colors`), text metrics
(`text_init`), and the physical workstation's attributes (`init_wk` — the
same routine `v_opnvwk` uses). Finally it starts the timer and mouse
subsystems and calls `esc_init` to enter graphics mode, then makes the
physical workstation current for Line-A.

Unlike `v_opnvwk`, `v_opnwk` does **not** allocate a `Vwk` — the physical
workstation uses the static `phys_work` global.

---

## Trap interface

- **Opcode:** `CONTRL[0] = 1`
- **Handle:** `CONTRL[6]` → `VDI_PHYS_HANDLE` (1) on output (input ignored).
- **Input counts:** `# INTIN = 11` (`work_in`, same layout as `v_opnvwk`);
  `# PTSIN = 0`.
- **Output counts:** `# INTOUT = 45` (`DEV_TAB`), `# PTSOUT = 6`
  (`SIZ_TAB`), pre-loaded by the dispatcher.
- **Native (AArch64) ABI:** GEM trap `svc #2`, selector 115 in `x0`,
  `VDIPB*` in `x1`.

## Inputs — `work_in[11]`

Identical to [`v_opnvwk`](100-v_opnvwk.md) **except `work_in[0]`**:

| Idx | `work_in[0]` | Meaning |
|-----|--------------|---------|
| 0 | resolution + 2 | `newrez = work_in[0] - 2`; if it names a valid ST/TT/Videl mode and differs from the current one, `Setscreen` switches to it. Otherwise no mode change. |
| 1–10 | line/marker/font/fill/coord | exactly as `v_opnvwk` (`init_wk`). |

On the Videl path only, `INTOUT[45]` carries the new Videl mode on input
(`:519-521`).

## Outputs

- `CONTRL[6] = VDI_PHYS_HANDLE (1)`.
- `INTOUT[0..44]` = `DEV_TAB`, `PTSOUT[0..11]` = `SIZ_TAB` — the same
  patched tables `v_opnvwk` returns (see
  [`100-v_opnvwk`](100-v_opnvwk.md) for the full byte-index tables).

### Side effects
- Reloads `DEV_TAB`/`INQ_TAB`/`SIZ_TAB` and patches geometry/colour;
  may switch screen resolution via `Setscreen`; initialises palette,
  timer, mouse, and enters graphics mode; sets `CUR_WORK = &phys_work`.

---

## Error conditions

None — VDI has no error return. `v_opnwk` always succeeds (there is no
allocation and no free-handle scan; the physical handle is fixed).

---

## Verification criteria

- **Handle:** `v_opnwk` returns `CONTRL[6] = 1` (`VDI_PHYS_HANDLE`).
- **Tables:** `INTOUT`/`PTSOUT` match the `v_opnvwk` `DEV_TAB`/`SIZ_TAB`
  for the live geometry — e.g. `INTOUT[0] = width-1`, `INTOUT[44] = 2`.
- **Idempotent geometry:** with `work_in[0]` naming the current mode
  (`newrez == Getrez()`), no `Setscreen` occurs and the tables reflect
  the running framebuffer.

---

## Edge cases discovered in source

1. **`INTOUT[45]` is read as an input** on the Videl (`CONF_WITH_VIDEL`)
   path — an *output* array slot repurposed to pass the new Videl mode,
   matching Atari TOS 4 (`vdi_control.c:519-521`). Irrelevant to a
   non-Videl target but a genuine ABI quirk.
2. **The physical workstation uses the static `phys_work`**, not a heap
   allocation — so `v_opnwk` cannot fail for lack of memory or handles.
3. **Resolution only changes for a recognised, different mode** — an
   out-of-range `work_in[0]` leaves the current mode untouched (no error).

---

## Dependencies

### Requires
- XBIOS `Getrez`/`Setscreen`; the palette (`init_colors`), font metrics
  (`text_init`), timer/mouse subsystems, and the escape console
  (`esc_init`).

### Used by
- The AES at startup (once), before any `v_opnvwk`.

---

## Related data structures

- [`docs/api/structures/vwk.md`](../../structures/vwk.md) — `phys_work` layout.
- [`_state.md`](../_state.md) — `DEV_TAB`/`INQ_TAB`/`SIZ_TAB` ownership,
  the physical-workstation handle.

---

## Cross-references

### Atari Compendium / Profibuch
- Not present in `references/` (no PDFs). Cross-check of `work_in[0]`
  (rez+2) and the Videl `INTOUT[45]` quirk is **deferred**.

---

## Configuration / version notes

- `CONF_WITH_TT_SHIFTER` / `CONF_WITH_VIDEL` add TT/Falcon resolutions —
  irrelevant to the RPi4 framebuffer target.

---

## NewTOS implementation notes (Phase 12)

- NewTOS has a framebuffer with a fixed (board-negotiated) geometry; the
  `work_in[0]` resolution change is honoured only insofar as the board
  supports it. Bring-up uses one fixed mode, so `newrez` is effectively
  ignored and `DEV_TAB[0,1]` reflect the live framebuffer size.
- **Who opens it, and when (K1, 2026-10-07).** As in EmuTOS, the AES opens
  the physical workstation at the start of every AES session — `gsx_init` →
  `gsx_wsopen` (`aes/gemgsxif.c:169-186,267-270`), called from
  `run_accs_and_desktop` (`aes/geminit.c:675`) — and keeps the handle in
  `gl_handle`, which `graf_handle` returns (`aes/gemsuper.c:287-293`). The
  session end closes it with `v_clswk` (`gsx_wsclose`, `geminit.c:749`),
  which frees every virtual workstation and keeps handle 1
  (`vdi_control.c:573-590`). NewTOS: `tos_api::aes::gsxif::gsx_init` /
  `gsx_wsclose`, called from the kernel's AES session loop
  (`kernel/src/lib.rs`). *Correction:* this note used to say the physical
  workstation is "opened once at graphics init and stays open for the system
  lifetime" — nothing ever called `v_opnwk`, so handle 1 was closed and
  every call on the `graf_handle` handle was a VDI no-op (zView M4 K1,
  `measurements/measurement-2026-10-06-zview-m4/checks/k1-evidence.txt`).
- `v_opnwk`'s `vwk_ptr[2..] = NULL` (`vdi_control.c:544-545`) is **not**
  reproduced by `vwk::open_physical` (open point, not part of K1). In the
  AES's own sequence it only shows when a virtual workstation is still open
  at the first session start (an AUTO program that never called
  `v_clsvwk`); a later session opens after `v_clswk` closed them all. A
  program calling `v_opnwk` itself while virtual workstations are open can
  tell too.
- The `init_colors`/`timer_init`/`vdimouse_init`/`esc_init` bring-up
  sequence is **reference**; NewTOS runs the equivalent framebuffer /
  input / console init, not the EmuTOS routines.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the `Setscreen` resolution switch,
> `phys_work` Line-A globals) at the BCM2711 VideoCore framebuffer. The
> **contract is unchanged** — opcode 1, `work_in[0] = rez+2`, the fixed
> handle `1`, the same `DEV_TAB`/`SIZ_TAB` as `v_opnvwk`. Peripheral +
> driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

> **ADR-017 cross-reference (2026-07-09, `docs/MEMORY.md`).** See
> [`100-v_opnvwk`'s Pi/BCM2711 binding note](100-v_opnvwk.md#pi--bcm2711-binding).
> The VDI table code reads the live framebuffer geometry, so no VDI change
> is needed when the driver's target geometry changes.
>
> **Correction (A86, 2026-09-21):** this blockquote and the bullet below it
> previously quoted "640×480×16" `DEV_TAB` values as "today's staged bring-up
> geometry". Both the colour figure and the resolution figure were stale —
> see the corrected values and citations below.

- **Same resolved tables as `v_opnvwk`.** `v_opnwk` runs the same `init_wk`
  (`tos-api/src/vdi/workstation.rs:141-245`), so `DEV_TAB`/`SIZ_TAB` carry
  the same values pinned in
  [`100-v_opnvwk`](100-v_opnvwk.md#pi--bcm2711-binding): `[13]=256`
  (colours, `NUM_COLORS`, `vdi_backend.rs:33`), `[35]=1` (colour-capable),
  `[39]=0` (`PALETTE_PRECISION`, truecolor — `get_palette()`'s "no discrete
  palette" case, `workstation.rs:30,202`), type `[44]=2`; `[0]`/`[1]` are
  `width()-1`/`height()-1` of the live (negotiated, currently 1920×1080)
  framebuffer, not a fixed `639`/`479`.
- **Physical handle = 1.** `open_physical()` returns handle
  `PHYS_HANDLE = 1` (`tos-api/src/vdi/vwk.rs:23`,
  `workstation.rs:177-183`); the physical workstation uses a static slot,
  cannot fail, and stays open for the system lifetime.
- **`work_in[0]` (rez+2) is effectively ignored — no mode switch.** The
  framebuffer geometry is negotiated once at boot (currently 1920×1080,
  `kernel/src/drivers/mod.rs:31-33`), not switched per `v_opnwk` call; there
  is no `Setscreen` resolution change wired to a real mode switch (XBIOS
  `Setscreen` only stores a rez shadow — see
  `docs/api/xbios/05-setscreen.md#pi--bcm2711-binding`). **Correction (A86,
  2026-09-21):** previously said "a single fixed 640×480 mode" — stale, see
  the corrected geometry above.
  `DEV_TAB[0,1]` always reflect the live framebuffer size regardless of
  `work_in[0]`, matching this spec's "idempotent geometry" note.
- **Bring-up sequence is native.** `init_colors`/`timer_init`/
  `vdimouse_init`/`esc_init` are reference; NewTOS initialises the
  framebuffer palette (`vdi_backend::PALETTE`), the mouse sprite
  (`tos-api/src/vdi/mouse.rs`), and the frame clock instead.
- **1080p later.** As in `v_opnvwk`, the geometry flows from the
  `RasterBackend`; a negotiated larger mode needs no VDI table change.
