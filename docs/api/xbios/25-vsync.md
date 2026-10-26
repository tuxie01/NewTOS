# `Vsync` (XBIOS 0x25)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1205` (`vsync`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the "block until the next frame boundary" behaviour,
> `void` return. Reference: `frclock`, the SR manipulation, `stop_until_interrupt`.

## EmuTOS source

**Primary location:** `bios/screen.c:1205` — `void vsync(void)`, slot 0x25
of `xbios_vecs[]` (`bios/xbios.c:1340`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `stop_until_interrupt()` — optional low-power wait between polls
  (`bios/screen.c:1219-1221`).

**Headers / type definitions used:**
- `include/tosvars.h` — `frclock`.

**System variables / globals read:**
- `frclock` (`0x466`) — the free-running VBL frame counter
  (`bios/screen.c:1217-1218`).

**System variables / globals written:**
- temporarily lowers the SR to `0x2300` to allow the VBL interrupt, then
  restores it (`bios/screen.c:1209,1226`); no persistent global written.

**Build configuration affecting this function:**
- `CONF_WITH_ATARI_VIDEO` — the SR/VBL mechanics; a build without Atari video
  still needs a frame tick to increment `frclock`.

---

## Source extraction summary

`Vsync` blocks until the next vertical-blank interrupt. It snapshots the
current `frclock` value, temporarily sets the SR to `0x2300` so VBL
interrupts can fire (in case the caller had interrupts masked), and then
spins — optionally `stop_until_interrupt()`-ing — until `frclock` changes.
The VBL handler increments `frclock` once per frame, so the loop exits after
exactly one frame boundary. It then restores the saved SR and returns
nothing.

`Vsync` is the standard way to page-flip tear-free (draw off-screen, `Vsync`,
`Setscreen` the new physical base) and to pace animation to the display
refresh.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x25
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0025
  ```
  No parameters (`Vsync()` = `xbios_v_v(37)`, `include/xbiosbind.h:55`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x25` in `x8`; no argument registers;
  **`void`** — no return value.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Source location:** `bios/screen.c:1205` (declared `void`).

### Side effects on global state

- None persistent (SR restored on exit).

### Side effects on hardware

- Enables VBL interrupts for the duration of the wait; blocks the caller for
  up to one frame.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Vsync` cannot fail. It can, however, block indefinitely if the VBL never
fires (see edge cases).

---

## Verification criteria

- **Blocks one frame:** `Vsync()` returns after `frclock` has advanced by at
  least one (not before).
- **Paced loop:** a loop of N `Vsync()` calls takes ≈ N frame periods
  (≈ N/refresh seconds).
- **Enables VBL even if masked:** calling `Vsync()` with interrupts masked at
  entry still returns (it lowers the SR to allow the VBL).

---

## Edge cases discovered in source

1. **Waits on `frclock`, not a fixed delay.** It compares against the entry
   value and returns on the first change (`bios/screen.c:1217-1218`) — one
   frame, regardless of how far into the current frame the call was made.
2. **Temporarily unmasks interrupts.** SR → `0x2300` (`:1209`) so the VBL can
   fire even if the caller masked it; the original SR is restored on exit
   (`:1226`). A caller relying on interrupts staying masked across `Vsync`
   would be surprised.
3. **Hangs if no VBL source.** With no frame interrupt incrementing `frclock`,
   the loop never exits. NewTOS must drive `frclock` from a real frame tick
   (VideoCore vsync / a timer) so `Vsync` terminates.

---

## Dependencies

### Requires (must be initialized / functional)

- A VBL/frame interrupt incrementing `frclock`.

### Used by

- Page flipping, animation pacing, `Setpalette` timing, demos, games.

---

## Related data structures

- `docs/api/xbios/_state.md §8` — `frclock` and the VBL state.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents "wait for the
  next vertical blank".)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None functional; the frame source differs by machine (Atari VBL vs NewTOS
  framebuffer vsync).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Vsync` blocks on the VideoCore vsync / a frame counter equivalent
to `frclock`; must terminate on real hardware where a caller might have
masked interrupts.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`frclock` incremented by the Atari
> VBL interrupt, the SR-lowering) at a NewTOS frame counter derived from
> the ARM generic timer. The **contract is unchanged** — block until the
> next frame boundary, `void`, always terminates. Peripheral: see
> `docs/HARDWARE-PROFILE.md` §2/§3.5.

- **Frame source = a synthesized 60 Hz from the ARM generic timer, not a
  real VideoCore vblank.** `vsync` snapshots `Host::frame_clock()` and
  spins until it advances (`tos-api/src/xbios/system.rs:32-40`);
  `frame_clock()` is computed as `CNTPCT_EL0 * 60 / CNTFRQ_EL0`
  (`kernel/src/host_env.rs:191-206`) — i.e. a free-running ~60 Hz frame
  number off the generic timer, the NewTOS analogue of `frclock`. It is
  **not** tied to the display's actual scan-out, so `Vsync` paces at ~60 Hz
  regardless of the HDMI refresh.
- **It does block, and it paces at ~60 Hz — measured, 2026-09-05.** An EL0
  program (`sdk/examples/scrflip.c`) called `Vsync` 600 times between two
  serial markers on QEMU `raspi4b`; the markers were 9.9 s apart, i.e.
  **60.6 Hz ± 1.5 %** (the sampling resolution of the measurement). So
  `Vsync` is a real wait, not a no-op, and a `Vsync`-paced loop runs at the
  frame rate a program expects.
- **Always terminates.** Because the counter is driven by the always-
  running generic timer (not a maskable VBL IRQ), the spin cannot hang even
  if the caller had interrupts masked — satisfying the contract's "enables
  VBL even if masked / must terminate" criterion. No SR manipulation is
  needed on AArch64.
- **A real VideoCore vblank exists and is not used yet.** The firmware
  property interface has a **wait-for-vsync** tag, `0x0004_800E`, with an
  empty value buffer (`docs/hardware/mailbox-framebuffer.md` §3b, from
  `references/circle/include/circle/bcmpropertytags.h:67` +
  `references/circle/lib/bcmframebuffer.cpp:405-410`). Circle waits on it immediately before
  a set-virtual-offset page flip. Binding NewTOS' `Vsync` to it would make
  `Setscreen` page flips exactly tear-free instead of approximately so;
  deliberately **not** done in the page-flip work item, because it turns a
  free-running timer read into a blocking mailbox round trip in the trap
  path and wants its own measurement.
- **Not tear-free against real scan-out (Step-2 candidate).** Since the
  frame clock is a timer approximation, `Vsync` does not guarantee
  alignment to the VideoCore vertical blank, so page-flip / palette-swap
  timing is only *approximately* tear-free. A true vblank source (a
  VideoCore vsync mailbox/IRQ or the HVS) would be needed for exact
  frame-boundary sync — deferred, and noted here as a fidelity gap, not a
  correctness bug (the call still returns after ≈ one 60 Hz period).
- **Datasheet.** The generic-timer counter (`CNTPCT_EL0`/`CNTFRQ_EL0`) is
  ARM-architectural (ARM ARM D17.6). A real display-vblank signal would
  come from the BCM2711 HVS/PixelValve (datasheet §6), which NewTOS does
  not yet consume.
- **1080p later.** The 60 Hz approximation is geometry-independent; a real
  vblank binding would follow the negotiated HDMI mode's refresh.
