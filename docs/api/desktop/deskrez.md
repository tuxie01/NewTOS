# `deskrez.c` — desktop resolution-change dialog

> Phase-00 extraction, Track A. Source: EmuTOS @ `1a7484ab`,
> `desk/deskrez.c` (334 lines). Per
> [`docs/api/desktop/_overview.md`](_overview.md): listed as "out of
> scope (but interacts with `app_start`'s recompute of `g_wicon`/`g_icw`
> on rez change)" — this entry now covers it directly, since it's part
> of this task's assignment.

## Naming note: `chkall` does not exist

The task brief asks for `change_resolution` and `chkall` to be
extracted from `deskrez.c`. `chkall` does not exist anywhere in
`references/emutos/` (verified by grep). The only public function in
this file is `change_resolution()`; the closest thing to a "check"
function is `rez_changeable()`, which lives in **`bios/screen.c:681-699`**
(BIOS layer, not `desk/`) — called once at startup from
`deskmain.c:396` (`detect_features()`, per `deskmain.md`'s own citation)
to cache whether the resolution-change menu item (`RESITEM`) should even
be enabled. `rez_changeable()` itself is out of scope for a `desk/`
extraction task (it belongs to a BIOS-layer spec, `docs/api/bios/`, if
not already covered there — not verified in this pass) and is only
noted here as a dependency.

---

## EmuTOS source

**Primary location:** `desk/deskrez.c:316-333` — `change_resolution
(newres, newmode)`

**Installed handler / vector:** `(none)` — called once, from
`deskmain.c`'s `do_optnmenu()` `RESITEM` case (per `deskmain.md`:
"`RESITEM` → `change_resolution`; on success (`rebld==1`), issues
`shel_write(SHW_RESCHNG,...)` … and sets `done=TRUE` — a resolution
change ends the desktop process").

**Helpers called (this file, `static`):**
- `deskrez.c:92-99` `change_st_rez(newres)` — ST-Shifter 2-resolution
  toggle (always compiled; the fallback path when neither TT-Shifter nor
  Videl hardware is present)
- `deskrez.c:107-142` `change_tt_rez(newres)` (`CONF_WITH_TT_SHIFTER`) —
  5-button dialog (`ADTTREZ`) mapping to `ST_LOW`/`ST_MEDIUM`/`ST_HIGH`/
  `TT_LOW`/`TT_MEDIUM`
- `deskrez.c:152-231` `change_falcon_rez(newres, newmode)`
  (`CONF_WITH_VIDEL`) — the Falcon/Videl resolution picker (detailed
  [below](#change_falcon_rez--the-videl-path))
- `deskrez.c:256-302` `change_amiga_rez(newres, newmode)`
  (`MACHINE_AMIGA`) — Amiga-specific resolution picker, **not
  applicable to NewTOS** (a different, non-Atari-hardware EmuTOS port —
  see [Configuration notes](#configuration--version-notes))

**Helpers called (other files, cited — not re-derived here):**
- `deskinf.c` `inf_show`/`inf_gindex`/`inf_what` — dialog primitives (see
  [`deskinf.md`](deskinf.md))
- `deskfun.c:75` `fun_alert()` (used by `change_st_rez` only)
- XBIOS wrappers (`xbiosbind.h`): `Getrez`, `VsetMode(-1)` (query
  current mode), `VgetMonitor`, `Srealloc(-1L)` (query available
  memory), `VgetSize(mode)` (query memory a mode would need)
- `has.h`: `has_videl`, `has_tt_shifter` — hardware-presence flags
  (cross-check `docs/api/bios/` or `docs/hardware/` if a Track B spec
  covers BCM2711 video presence-detection analogues — not verified in
  this pass, and moot for NewTOS: see
  [Configuration notes](#configuration--version-notes))

**Headers / type definitions used:**
- `deskrez.h:13` — `int change_resolution(WORD *newres, WORD *newmode);`
- `biosdefs.h`/`biosext.h` — `ST_LOW`/`ST_MEDIUM`/`ST_HIGH`/`TT_LOW`/
  `TT_MEDIUM`, `VIDEL_*` bit constants (`VIDEL_80COL`/`VIDEL_1BPP..
  VIDEL_TRUECOLOR`/`VIDEL_VERTICAL`/`VIDEL_COMPAT`/`VIDEL_OVERSCAN`/
  `VIDEL_PAL`/`VIDEL_VGA`/`VIDEL_BPPMASK`), `FALCON_REZ`=3,
  `MON_VGA`

**System variables / globals read:** none directly (all state comes
from XBIOS query calls each time `change_resolution` is invoked — no
cached `G.*` field for "current resolution" in this file).

**System variables / globals written:** none — `change_resolution`
returns the new resolution/mode via output parameters; the actual mode
switch happens **after** the desktop process exits (per `deskmain.md`'s
citation: `shel_write(SHW_RESCHNG,...)` then `done=TRUE` — the mode
change itself is not this function's job at all, only the **user's
choice** of new mode is).

**Build configuration affecting this function:**
- `CONF_WITH_TT_SHIFTER` — gates the TT resolution dialog entirely
  (`change_tt_rez`, the `ttrez_from_button[]` table); default **on** in
  the general/Atari build (`include/config.h:1086`, forced off on non-TT
  hardware profiles per the various `#if`-guarded `0` defines throughout
  `config.h`'s per-machine sections — this is a hardware-capability
  gate, not a `TARGET_192`-style space-saving gate)
- `CONF_WITH_VIDEL` — gates the Falcon resolution dialog
  (`change_falcon_rez`, `falconmode_from_button[]`); default **on**
  (`include/config.h:1093`)
- `CONF_WITH_VDI_16BIT` — within `change_falcon_rez`, hides/disables
  "Truecolor" (16-bit) mode buttons if the VDI wasn't built with 16-bit
  support (not traced further — VDI-layer gate, `docs/api/vdi/` if
  covered there)
- `MACHINE_AMIGA` — compile-time machine selector, **not** a
  `CONF_WITH_*` flag but a whole different EmuTOS *port* (Amiga
  hardware); mutually exclusive with the Atari-hardware paths in this
  file. **Not applicable to NewTOS** in any form.

---

## Source extraction summary

`change_resolution()` is a thin **dispatch-by-hardware-capability**
function: it tries, in order, the Amiga path (compile-time only), the
Falcon/Videl path (`has_videl`), the TT-Shifter path
(`has_tt_shifter`), and finally falls back to the plain ST 2-resolution
toggle. Exactly one of these branches ever executes per build/hardware
combination — there is no runtime fallthrough between, say, Videl and
TT-Shifter (a real Falcon never has a TT-Shifter, and vice versa; the
`#if`/`has_*` guards reflect genuine hardware mutual exclusivity, not an
arbitrary priority order). Each branch shows a resolution-picker UI
appropriate to that hardware's actual capabilities (a plain confirm
alert for the 2-mode ST case; a 5-button dialog for the TT's 5 modes; a
large button-grid dialog, with several rows hidden/shown depending on
monitor type and VDI build options, for the Falcon's much larger mode
space) and returns whether the user actually chose a **different**
resolution than the current one, plus what that resolution/mode value
is. It performs **no actual mode switch itself** — the caller
(`deskmain.c`'s `do_optnmenu`) is responsible for applying the change,
and does so only *after* ending the desktop process (a resolution
change is destructive enough — different screen dimensions, possibly
different colour depth — that EmuDesk restarts fresh at the new
resolution rather than trying to live-rebuild everything in place).

---

## `change_st_rez` — the ST fallback path

`deskrez.c:92-99`. The simplest branch, always compiled (no `#if`
guard) since it's the universal fallback when neither TT-Shifter nor
Videl hardware is detected. No dialog at all — a plain confirm alert
(`STRESOL`, "Do you want to change resolution?"). If confirmed, toggles
between the ST's two resolutions via `Getrez() ? 0 : 1` (`Getrez()`
returns the *current* ST resolution as a small integer; this ternary
just flips between resolution `0` and `1` — i.e. **this path assumes
exactly two ST resolutions exist** and does not handle ST-high (mono)
at all, matching the `deskmain.md`/`docs/api/desktop.md` note that
`RESITEM` is only enabled when `rez_changeable()` is true, which is
false on ST-high hardware in the first place — see `bios/screen.c`, out
of scope here).

## `change_tt_rez` — the TT-Shifter path (`CONF_WITH_TT_SHIFTER`)

`deskrez.c:107-142`. Maps 5 dialog buttons (`ADTTREZ` resource,
`TTREZSTL` base object) to 5 resolutions via a fixed lookup table
`ttrez_from_button[]` = `{ST_LOW, ST_MEDIUM, ST_HIGH, TT_LOW,
TT_MEDIUM}` (`:48-49`). Pre-selects whichever button matches
`Getrez()`'s current value (linear search, `:114-116`). If the user
picks the **same** button again, `change_tt_rez` returns `0` (no
change) rather than `1` — i.e. re-confirming the current resolution is
treated identically to Cancel (`:137-138`).

## `change_falcon_rez` — the Videl path (`CONF_WITH_VIDEL`)

`deskrez.c:152-231`. Considerably more involved than the ST/TT paths,
because Videl supports many more mode combinations:

1. Reads the current mode (`VsetMode(-1)`) and decomposes it into a
   "base" mode (resolution/colour-depth bits) plus "options"
   (overscan/PAL/VGA bits) — **the vertical/interlace bit's meaning is
   inverted in RGB (non-VGA) mode** relative to VGA
   (`:162-163`, `if (!(oldoptions&VIDEL_VGA)) oldbase ^=
   VIDEL_VERTICAL;` — a hardware quirk the code compensates for
   explicitly, both when reading the current mode and, symmetrically,
   when writing the chosen one back at `:219-220`).
2. Looks up which of the `falconmode_from_button[]` table's 21 entries
   matches the current base mode (linear search); if none match
   (shouldn't normally happen), `selected` ends up `==
   NUM_FALCON_BUTTONS` (out of range — no button pre-selected, not
   treated as an error).
3. **Monitor-type-dependent dialog adjustment** (`:176-179`): on a
   non-VGA monitor, the resolution *names* shown are swapped to their
   RGB-monitor equivalents (`STREZ1`+i replacing the default VGA-centric
   labels) — the underlying button→mode table doesn't change, only the
   **displayed text**, since the same base-mode bits mean a different
   physical resolution depending on monitor type (VGA vs. RGB
   monitor cabling on real Falcon hardware).
4. **Truecolor / VGA-monitor mode hiding** (`:184-200`): if
   `!CONF_WITH_VDI_16BIT`, the "Truecolor" header text and every
   Truecolor-capable button are hidden entirely (`HIDETREE` flag) —
   there is no point offering a mode the VDI can't actually render into.
   Independently, even *with* 16-bit VDI support, Truecolor modes that
   require 80-column + VGA cabling are hidden if the monitor is VGA
   (some Truecolor timings are RGB-monitor-only on real Falcon hardware).
5. On OK, reconstructs the full mode word (`falconmode_from_button[i] |
   oldoptions`, re-inverting the vertical bit for RGB mode symmetrically
   with step 1), and **validates available memory** before returning
   success: `if (Srealloc(-1L) < VgetSize(mode))` → `malloc_fail_alert()`
   and returns `0` (cancel) rather than committing to a mode the system
   doesn't have enough ST-RAM to support (`:222-225`). This is the
   **only** hardware-capability check performed after the user's choice
   — none of the ST/TT paths have an equivalent memory check, presumably
   because their mode set never varies in required frame-buffer size
   the way Falcon truecolor/high-resolution modes do.

---

## Edge cases discovered in source

- **Re-selecting the current resolution is treated as "no change"**, not
  as "confirm the same resolution" — in both `change_tt_rez` (`:137-138`)
  and `change_falcon_rez` (`:215-216`), if the newly-selected button
  index equals the pre-selected (current) one, the function returns `0`
  exactly as if the user had cancelled. `change_st_rez` doesn't have
  this check (it always toggles to *the other* mode, so "the same mode"
  isn't a reachable outcome via that path's confirm-alert UI alone).
- **The Videl vertical-bit inversion for RGB monitors** (`:162-163`,
  `:219-220`) is applied **symmetrically** on read and write — a subtle
  hardware compensation that must be reproduced identically (invert on
  read, invert again on write) or the resulting mode will be visually
  correct but semantically wrong (or vice versa) for RGB-monitor Falcons
  specifically.
- **Falcon memory-availability check happens only after the user
  commits**, not while browsing the button list — a user can select and
  see a Truecolor high-resolution mode highlighted, click OK, and *then*
  get `malloc_fail_alert()` and have the whole operation cancelled with
  no resolution change — there's no proactive graying-out of
  memory-infeasible buttons.
- **`MACHINE_AMIGA` is a mutually-exclusive alternate universe within
  this same file** — `change_resolution`'s very first line is `#ifdef
  MACHINE_AMIGA return change_amiga_rez(...); #endif`, meaning on an
  Amiga build, **none** of the ST/TT/Falcon code paths are even
  reachable, regardless of `CONF_WITH_TT_SHIFTER`/`CONF_WITH_VIDEL`
  settings (which would presumably be `0` on that port anyway, but the
  `#ifdef` short-circuit makes this unconditional either way).

---

## Error conditions

| Condition | Source location | Result |
|---|---|---|
| User declines the ST resolution-change confirm alert | `deskrez.c:94-95` | `change_st_rez` returns `0` |
| User picks the same TT/Falcon resolution as current | `deskrez.c:137-138`,`:215-216` | returns `0` (treated as cancel) |
| No button matched the current mode (`selected` out of range) | `deskrez.c:118`(TT)/`:168`(Falcon) | no button pre-selected in the dialog; not otherwise an error |
| `Srealloc(-1L) < VgetSize(mode)` (insufficient memory for chosen Falcon mode) | `deskrez.c:222-225` | `malloc_fail_alert()`, returns `0` (cancel), no mode change |
| User cancels any dialog (`inf_what(...) == 0`) | `deskrez.c:130-131`(TT),`:208-209`(Falcon) | returns `0` |

---

## Verification criteria

- **No-op re-selection**: choosing the currently-active resolution in
  any dialog must return `0` (the caller must not end the desktop
  process nor issue `shel_write(SHW_RESCHNG,...)`).
- **Falcon memory guard**: selecting a mode whose `VgetSize()` exceeds
  currently-available ST-RAM must show `malloc_fail_alert()` and return
  `0`, leaving the desktop process running (per `deskmain.md`'s
  `RESITEM` handling: `done=TRUE` only "on success (`rebld==1`)").
- **RGB vertical-bit round-trip**: on a non-VGA (RGB) monitor,
  selecting the same base mode that's already active must reconstruct
  the *original* mode word bit-for-bit after the double inversion
  (read-invert, write-invert-back) — this is really a test of
  `change_falcon_rez`'s internal consistency, not externally observable
  beyond "resolution doesn't silently change when re-selecting the
  current one."

---

## Dependencies

### Requires (must be initialized / functional)
- XBIOS: `Getrez`/`VsetMode`/`VgetMonitor`/`Srealloc`/`VgetSize` — see
  `docs/api/xbios/` (already spec'd, per `_overview.md`'s cross-reference
  convention — not re-verified present in this pass)
- `has_videl`/`has_tt_shifter` (`has.h`) — hardware-presence flags,
  themselves presumably derived from a BIOS-layer probe (`docs/api/bios/`,
  out of scope here)
- [`deskinf.md`](deskinf.md) — `inf_show`/`inf_gindex`/`inf_what` dialog
  primitives
- [`deskfun.md`](deskfun.md) — `fun_alert` (used by `change_st_rez` only)

### Used by
- `deskmain.c`'s `do_optnmenu()` `RESITEM` case (the sole caller) — see
  `deskmain.md`'s citation `deskrez.c:316`

## Related data structures

None specific to this file beyond the XBIOS mode-word bit layout, which
would belong to `docs/api/xbios/` (Videl mode word) if not already
covered there — not independently verified in this pass.

## Cross-references

Not cross-checked against Compendium/Profibuch (hardware-resolution
dialog UI, not a documented TOS API surface function itself — though the
underlying XBIOS calls it makes, `Getrez`/`VsetMode`/etc., are
documented TOS API and already spec'd separately).

## Configuration / version notes

**This entire file is not applicable to NewTOS in its current form.**
NewTOS targets a single AArch64/RPi4 framebuffer configuration (no ST
Shifter, no TT Shifter, no Falcon Videl, no Amiga chipset) — per
`CLAUDE.md`'s non-goals ("No hardware-level Atari emulation") and
`docs/MEMORY.md`'s ADRs on the display pipeline (not independently
re-checked here). If NewTOS ever supports **multiple** RPi4 video modes
(e.g. different HDMI resolutions/refresh rates via the VideoCore
firmware), the **observable contract** worth preserving from this file
is narrow:
- A "change resolution" desktop action ends the desktop process rather
  than live-rebuilding in place (matches `deskmain.md`'s `RESITEM`
  handling).
- Re-selecting the current mode is a no-op, not a forced restart.
- A mode requiring more framebuffer memory than available is rejected
  with a clear alert before any state changes.

Everything else here (the specific ST/TT/Falcon/Amiga hardware
dispatch, the button-to-mode tables, the RGB-vertical-bit inversion) is
Atari/Amiga-hardware-specific and **must not** be ported — this is a
Track A extraction for historical/behavioural completeness per the task
brief, not a NewTOS implementation guide. Flag for the lead: this file
is the strongest candidate in the whole `desk/` extraction for "read,
understood, explicitly not reused."

---

## NewTOS implementation notes

(Empty during Phase 00. Given the note above, this section may remain
permanently empty/`N/A` unless NewTOS adopts multi-resolution support.)
