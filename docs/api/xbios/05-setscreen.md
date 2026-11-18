# `Setscreen` (XBIOS 0x05)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1102` (`setscreen`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the negative-argument "ignore" convention, the WORD
> return (`-1` on error, previous Videl mode, else 0), the rez-change side
> effects a program can observe (screen cleared, cursor homed, VT52 reset).
> Reference: `vblsem`, `atari_setrez`, `Srealloc`, the Videl internals.

## EmuTOS source

**Primary location:** `bios/screen.c:1102` —
`WORD setscreen(UBYTE *logLoc, const UBYTE *physLoc, WORD rez, WORD videlmode)`,
slot 5 of `xbios_vecs[]` (`bios/xbios.c:1291`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/screen.c` — `setphys(physLoc)` — program the hardware physical base
  (`:1111`).
- `bios/videl.c` — `vfixmode()`, `vgetsize()`, `vsetmode()` — Videl mode
  fixup/size/set (`:1133,1135,1143`, `CONF_WITH_VIDEL`).
- `bios/biosmem.c` — `Srealloc()` — reallocate screen RAM for a new Videl
  size (`:1135`).
- `bios/screen.c` — `atari_setrez(rez, videlmode)` — apply the resolution
  (updates `sshiftmod`) (`:1154`).
- `bios/lineainit.c` — `linea_init()` — re-init Line-A vars (`:1160`).
- `bios/vt52.c` — `vt52_init()` — reset the VT52 emulator (`:1161`).
- `bios/screen.c:1205` — `vsync()` — wait for end-of-frame before switching
  (`:1149`).

**Headers / type definitions used:**
- `bios/screen.h` — `MIN_REZ`, `MAX_REZ`, `FALCON_REZ` (3), resolution codes.
- `include/tosvars.h` — `v_bas_ad`, `vblsem`.

**System variables / globals read:**
- `rez_was_hacked` (Line-A hack guard, `:1115`), `has_videl` (`:1130`).

**System variables / globals written:**
- `v_bas_ad` (`0x44E`, `:1107` and Videl realloc `:1139`).
- `vblsem` (`0x452`, cleared `:1158`, restored `:1163`).
- `sshiftmod` (`0x44C`, via `atari_setrez`).
- the physical base register (via `setphys`).

**Build configuration affecting this function:**
- `CONF_WITH_VIDEL` — enables the `rez == FALCON_REZ` reallocation/mode path.
- `CONF_WITH_ATARI_VIDEO` — `atari_setrez` / `vsync` mechanics.

---

## Source extraction summary

`Setscreen` reconfigures the display: it optionally moves the logical base
(`logLoc`), the physical base (`physLoc`), and the resolution (`rez`). A
**negative** value for any of the three means "leave that unchanged": the code
tests each as a signed LONG/WORD and only applies it when positive
(`logLoc > 0 → v_bas_ad = logLoc`, `:1106-1107`; `physLoc > 0 →
setphys(physLoc)`, `:1110-1111`; `rez == -1 → return 0` without a rez change,
`:1115-1116`).

The resolution is validated against `[MIN_REZ, MAX_REZ]`; an out-of-range
`rez` returns `-1` (`:1120-1121`). On a Videl machine, `rez == FALCON_REZ`
(3) enters a special path: if `videlmode != -1` it fixes the mode with
`vfixmode`; if *both* `logLoc` and `physLoc` are NULL it reallocates screen
RAM (`Srealloc(vgetsize(videlmode))`), returning `-1` on failure and setting
both bases to the new buffer (`:1130-1141`). It saves the previous Videl mode
(`oldmode = vsetmode(-1)`).

When the resolution actually changes it: `vsync()`s to a frame boundary,
clears `vblsem` to suspend the VBL, calls `atari_setrez` (which also updates
`sshiftmod`), re-inits Line-A (`linea_init`) and the VT52 (`vt52_init`) —
this is what clears the screen and homes the cursor — then restores
`vblsem`. The WORD return is `oldmode` for Falcon resolutions, else `0`.

Note the whole function is documented as `void` in Atari docs but EmuTOS
returns a WORD for TOS-4 compatibility.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x05
- **Stack frame on entry (m68k):** `Setscreen(logLoc, physLoc, rez,
  videlmode)` = `xbios_v_llww(5, …)` (`include/xbiosbind.h:24`):
  ```
  +10: videlmode (word)
   +8: rez       (word)
   +4: physLoc   (LONG, pointer)
   +2: logLoc    (LONG, pointer)
    0: function_id (word) = 0x0005
  ```
- **Total parameter bytes:** 12 (L + L + W + W).
- **Native (AArch64) ABI:** ID `0x05` in `x8`; args in `x0..x3`; the WORD
  result **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name        | Type / size |
  |------------|-------------|-------------|-------------|
  | +2         | `x0`        | `logLoc`    | `UBYTE *` (ptr) |
  | +4         | `x1`        | `physLoc`   | `const UBYTE *` (ptr) |
  | +8         | `x2`        | `rez`       | `WORD` (16) |
  | +10        | `x3`        | `videlmode` | `WORD` (16) |

---

## Inputs

| Position | Name        | Type    | Size | Source location       | Notes |
|----------|-------------|---------|------|-----------------------|-------|
| arg0     | `logLoc`    | `UBYTE*`| 4    | `bios/screen.c:1106`  | new logical base; ≤ 0 ⇒ unchanged |
| arg1     | `physLoc`   | `UBYTE*`| 4    | `bios/screen.c:1110`  | new physical base; ≤ 0 ⇒ unchanged |
| arg2     | `rez`       | `WORD`  | 2    | `bios/screen.c:1115`  | resolution code; `-1` ⇒ unchanged; out of `[MIN_REZ,MAX_REZ]` ⇒ error |
| arg3     | `videlmode` | `WORD`  | 2    | `bios/screen.c:1132`  | Videl mode word (used only when `rez == FALCON_REZ`); `-1` ⇒ keep |

> "≤ 0 ⇒ unchanged" for the pointers: the source tests `if ((LONG)logLoc > 0)`
> / `if ((LONG)physLoc > 0)`, so `NULL` (0) and any negative value skip the
> base update. (Documentation says "negative"; the code treats `0` the same
> as negative for the base pointers.)

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** WORD
- **Encoding:**
  - `-1` if `rez` is invalid, or `Srealloc()` failed (`:1121,1137`).
  - the **previous Videl mode** for Falcon resolutions (`oldmode =
    vsetmode(-1)`, `:1143`).
  - `0` otherwise (including `rez == -1` no-op, `:1116`).
- **Source location:** `bios/screen.c:1116,1121,1137,1143,1165`.

### Side effects on global state

- `v_bas_ad = logLoc` when `logLoc > 0` (`:1107`); both bases on a Videl
  realloc (`:1139-1140`).
- `sshiftmod` updated by `atari_setrez` on a rez change.
- `vblsem` cleared then restored around the rez change (`:1158,1163`).

### Side effects on hardware

- `setphys` programs the physical video-base register.
- On a rez change: screen cleared + cursor homed + VT52 reset (via
  `linea_init`/`vt52_init`), one `vsync` wait.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `rez < MIN_REZ` or `rez > MAX_REZ` (and `rez != -1`) | `bios/screen.c:1120-1121` | `-1` |
| Videl `Srealloc()` failed (both bases NULL, `rez == FALCON_REZ`) | `bios/screen.c:1135-1137` | `-1` |

`rez == -1` is **not** an error — it returns `0` and changes only the bases.

---

## Verification criteria

- **Move logical base only:** `Setscreen(buf, -1, -1, -1)` sets
  `Logbase() == buf`, leaves `Physbase()` and `Getrez()` unchanged, returns
  `0`.
- **Invalid rez:** `Setscreen(-1, -1, 99, -1)` returns `-1` and changes
  nothing.
- **Rez change clears screen:** `Setscreen(-1, -1, 2, -1)` sets `Getrez()`
  to `2`, clears the screen, homes the VT52 cursor, returns `0`.
- **No-op rez:** `Setscreen(-1, -1, -1, -1)` returns `0` and leaves the
  display untouched.
- **Falcon return:** on a Videl machine, `Setscreen(0, 0, 3, mode)` returns
  the previous Videl mode (or `-1` on realloc failure).

---

## Edge cases discovered in source

1. **Base pointers use `> 0`, not `>= 0`.** `NULL` (0) skips the base update
   just like a negative value (`:1106,1110`); only a strictly positive
   pointer moves the base.
2. **`rez == -1` short-circuits before validation.** It returns `0`
   immediately (`:1115-1116`), so you can move the bases without touching the
   resolution and without risking the `-1` error.
3. **Videl NULL/NULL triggers screen-RAM reallocation.** `rez ==
   FALCON_REZ` with both bases NULL calls `Srealloc(vgetsize(videlmode))`
   and points both bases at the fresh buffer; a failed alloc returns `-1`
   (`:1134-1141`).
4. **The WORD return is an EmuTOS/TOS-4 extension.** Atari docs call
   `Setscreen` `void`; EmuTOS returns `-1`/oldmode/0. NewTOS follows EmuTOS.
5. **Line-A "hacked" resolutions bypass the change.** If `rez_was_hacked`,
   the function returns `0` early without touching rez (`:1115`).

---

## Dependencies

### Requires (must be initialized / functional)

- The display driver (`v_bas_ad`, `setphys`, `atari_setrez`), the VBL
  (`vblsem`, `vsync`), the VT52/Line-A init.
- (Videl only) `Srealloc`/`vgetsize`/`vsetmode`.

### Used by

- Double-buffering, mode switches, VDI `v_opnwk`, demos.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — `v_bas_ad`, `sshiftmod`, `vblsem`.
- `docs/api/xbios/04-getrez.md` — the resolution codes.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `Setscreen`
  as `void` with the negative-ignore convention; the WORD return is EmuTOS/
  TOS-4.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The `FALCON_REZ` realloc path is `CONF_WITH_VIDEL`-only; on the NewTOS
  TT-shifter profile only ST/TT rez codes apply.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Setscreen` maps to framebuffer retarget (log base) + swap-chain
present (phys base) + mode reconfigure (rez) via the VideoCore mailbox; the
screen-clear/cursor-home/VT52-reset side effects on a rez change must be
reproduced. The WORD return follows EmuTOS (`-1`/0).

### NewTOS behavior (settled — ADR-017 rev 2, 2026-07-16)

Implemented in `tos-api/src/xbios/screen.rs`:

- **`rez` is accepted-but-ignored — for *any* value.** There is **no
  in-system resolution switch** (ADR-017 rev 2: the firmware `config.txt`
  owns the HDMI mode; a framebuffer-only in-menu switch would merely be
  VideoCore-scaled, not native). So `setscreen()` does **not** keep a `REZ`
  shadow and does **not** reject an out-of-range `rez`: every `rez`
  (including `-1`, `2`, `4`, or a nonsense code) is silently accepted and the
  call returns `0`. This mirrors EmuTOS's own precedent that an
  in-range-but-hardware-unsupported `rez` is a **silent no-op, not an error**
  (`atari_setrez`/`vsetmode`, `bios/screen.c:990-1013`,
  `bios/videl.c:619-659,976-993`) — here extended to *all* codes because
  *no* code is supported for switching. `Getrez` continues to report the
  fixed `ST_HIGH` (2) regardless (ADR-017 Revision 3, see `04-getrez.md`).
- **Base handling is unchanged and orthogonal.** `logLoc`/`physLoc` are each
  stored into their shadow only when `> 0` (`screen.c:1106,1110`); `Physbase`/
  `Logbase` read those shadows. This is a separate concern from `rez` and is
  out of scope of the ADR-017 rev 2 change.
- **Return value.** Always `0` — the `-1` invalid-rez path and the Falcon
  `oldmode` path do not apply (no in-system switch, no Videl).

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`setphys` video-base register,
> `atari_setrez`, the Videl realloc path) at the BCM2711 VideoCore
> framebuffer. The **contract is unchanged** — the negative-argument
> "ignore" convention, the `-1`-on-invalid-rez WORD return, `rez == -1` =
> no-op. Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

> **Resolution policy (ADR-017 rev 2, 2026-07-16, `docs/MEMORY.md`):**
> NewTOS renders **natively at the config.txt-driven HDMI geometry**
> (auto-detected at boot; 1920×1080 fallback / QEMU-ramfb default) at 16 VDI
> colours. There is **no in-system resolution switch** — the firmware
> `config.txt` owns the mode. This rev 2 **settles** the previously-open
> `Setscreen`-`rez` question: `rez` is accepted-but-ignored (see below).

- **Base re-pointing is unaffected by this policy** (separate concern), and
  since 2026-09-05 it is **real** — see "Page flipping" below.
- **Settled: `rez` is accepted-but-ignored for *any* value; the call always
  returns `0`.** There is **no** `Setscreen`-driven mode reconfigure — the
  1920×1080/1280×800/… choice is `config.txt`-driven and applied at boot,
  not via this XBIOS call. `setscreen()` therefore keeps **no `REZ`
  shadow**, performs **no `[MIN_REZ,MAX_REZ]` validation**, and **never
  returns `-1` for any rez**. This extends EmuTOS's own precedent that an
  in-range-but-hardware-unsupported `rez` is a **silent no-op, not an
  error** (`atari_setrez`/`vsetmode`, `bios/screen.c:990-1013`,
  `bios/videl.c:619-659,976-993`) to *every* code, because *no* rez is a
  switchable target on NewTOS:
  ```
  any rez (incl. -1, 2, 4, out-of-range) → accept, no change, return 0.
  ```
  (Contrast EmuTOS, which returns `-1` for `rez` outside `[MIN_REZ,MAX_REZ]`;
  NewTOS deliberately drops that rejection since the whole rez axis is inert.)
- **Falcon/Videl `videlmode` argument:** unused by NewTOS (no Falcon
  hardware; no in-system switch to overload it for).
- **`deskrez` (desktop resolution dialog) is out of scope** under rev 2 (the
  in-menu runtime switch is dropped; resolution is `config.txt`-driven) —
  see ADR-017 rev 2 in `docs/MEMORY.md`.

### Page flipping — how `logLoc` / `physLoc` are honoured (2026-09-05)

**Both bases move for real.** They are not shadows:

- `logLoc > 0` → the display's *draw* pointer (EmuTOS `v_bas_ad`). The VDI
  raster backend, the VT52 console and the desktop all draw through it, so
  after the call they draw into the named screen.
  (`Host::set_screen_logical` → `vdi_backend::set_draw_base` →
  `Framebuffer::set_draw_base`.)
- `physLoc > 0` → the display engine's scan-out base (EmuTOS `setphys`). On
  the BCM2711 that is the mailbox **set-virtual-offset** tag `0x0004_8009`
  selecting one of the stacked screens
  (`docs/hardware/mailbox-framebuffer.md` §3b).
  (`Host::set_screen_physical` → `vdi_backend::set_scanout_base` →
  `Framebuffer::set_scanout_base`.)
- `≤ 0` (including `0`) → that base is left alone, exactly as
  `if ((LONG)logLoc > 0)` / `if ((LONG)physLoc > 0)` (`screen.c:1106,1110`).
- `rez == -1`, or any other `rez` → no resolution change, return `0` (see the
  rev-2 policy above).
- **Return value: always `0`.** (The `-1`-on-invalid-rez branch is dropped by
  the rev-2 policy; there is no other error branch in EmuTOS for the bases.)

**Where a base may point.** The VideoCore allocates the screen memory and
only it can be scanned out, so a base must be the start of a whole screen
page inside that memory. A base outside it — or not on a page boundary — is
**not applied**: the display has no way to represent it, the same way a plain
ST's video-base register cannot represent the low byte of an address and
silently drops it (`atari_setphys`, `screen.c:939-947`). Nothing is echoed
back: the following `Physbase`/`Logbase` reports what the hardware really
does, because (like EmuTOS' `physbase()`, `screen.c:1030`) it reads the
hardware rather than a shadow. A program that checks is therefore told the
truth. **NewTOS cannot make an arbitrary caller-allocated buffer the
physical screen** — that is the one substantive divergence from the Atari,
and it is a property of the display hardware, not a shortcut.

**Address translation.** An EL0 caller passes the window VA it got from
`Physbase`/`Logbase` (ADR-023), not a physical address; `setscreen` resolves
it through the calling process's own block table
(`bdos::process::usermem::resolve_kva`). An EL1 caller passes a kernel
high-half VA (or a physical address, which is read as the same DRAM).

**Screen pages available.** `board-rpi4`: **2** — the VideoCore framebuffer
is allocated with double virtual height at boot
(`kernel/src/drivers/bcm2711/framebuffer.rs`, `WANTED_PAGES`), with a
single-page fallback if the firmware refuses. `board-qemu-virt` (ramfb): **1**
— the pixel store is a fixed reservation off the top of DRAM, so a second
page would shrink the ST/TT-RAM pools; `Setscreen` there accepts only the
page already in use and refuses any move.

**Proof.** `sdk/examples/scrflip.c` (`SCRFLIP.NCP`): finds the second page
through this contract (it walks offsets and watches `Logbase()`), paints it
while invisible, flips it in, and restores both bases. Verified on QEMU
`raspi4b` 2026-09-05 by a QMP screendump pair: console text before, three
full-screen colour bands after, with nothing copied.
