# `Physbase` (XBIOS 0x02)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1042` (`physbase`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the full-64-bit pointer result, the
> physical-vs-logical distinction a program can observe. Reference: the
> Shifter video-base hardware registers and the platform fallback chain.

## EmuTOS source

**Primary location:** `bios/screen.c:1042` — `const UBYTE *physbase(void)`,
slot 2 of `xbios_vecs[]` (`bios/xbios.c:1288`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/screen.c` — `atari_physbase()` — reconstructs the 24-bit base from
  the Shifter registers `0xFFFF8201`/`0xFFFF8203` (+ `0xFFFF820D` on
  STe/TT/Videl for byte-granular base).
- `bios/screen.c:1071` — `logbase()` — fallback when there is no Atari video.

**Headers / type definitions used:**
- `include/tosvars.h` — `v_bas_ad` (only via the `logbase` fallback).

**System variables / globals read:**
- Shifter video-base MMIO registers (Atari video only).
- `v_bas_ad` (`0x44E`) — only on the `#else` fallback path.

**System variables / globals written:** none.

**Build configuration affecting this function:**
- `CONF_WITH_ATARI_VIDEO` — with it, reads the Shifter registers; without
  it, returns `logbase()`.
- `MACHINE_AMIGA` / `MACHINE_LISA` — platform-specific `*_physbase()`.

---

## Source extraction summary

`Physbase` returns the address the video hardware is *currently displaying*
— the physical screen base. On Atari video, `atari_physbase` reads the
Shifter's high/mid (and, on STe and later, low) video-base registers and
assembles the pointer. When no Atari video is compiled in, EmuTOS falls back
to `logbase()`, so `Physbase() == Logbase()` and there is no hardware page
flip.

This differs from `Logbase` (0x03), which returns where *future drawing*
lands (`v_bas_ad`). After `Setscreen(log, phys, …)` the two may differ until
the display catches up, which is how double-buffered programs flip pages.

The value is read straight back from hardware (or `v_bas_ad`); there is no
validation and the call cannot fail.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x02
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0002
  ```
  No parameters (`Physbase()` = `xbios_l_v(2)`, `include/xbiosbind.h:21`).
- **Total parameter bytes:** 2 (function word only).
- **Native (AArch64) ABI:** ID `0x02` in `x8`; no argument registers; result
  is a **pointer** occupying the full 64-bit `x0` (not sign-extended from
  32 bits).

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

| Position | Name | Type | Size | Source location | Notes |
|----------|------|------|------|-----------------|-------|
| —        | —    | —    | —    | —               | —     |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** pointer (`UBYTE *`)
- **Encoding:** the physical screen base address. On a 24-bit ST this is a
  ≤ 0x00FFFFFF value; on NewTOS it is a full 64-bit framebuffer pointer.
- **Source location:** `bios/screen.c:1049` (`atari_physbase`) or `:1052`
  (`logbase` fallback).

### Side effects on global state

None.

### Side effects on hardware

None (a pure register read on Atari video).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Physbase` has no error path; it always returns a pointer.

---

## Verification criteria

- **Nominal:** on the NewTOS profile (no Atari video), `Physbase()` returns
  the same pointer as `Logbase()` until real page flipping exists.
- **Page-flip observability:** after `Setscreen(log, phys, -1)` with distinct
  `log != phys`, `Physbase()` returns `phys` and `Logbase()` returns `log`.
- **Alignment preserved:** `Physbase()` returns exactly the pointer last set
  by `Setscreen` (NewTOS must not silently re-align it away from the caller's
  value).

---

## Edge cases discovered in source

1. **Fallback to `logbase()` with no Atari video.** `bios/screen.c:1052` —
   `Physbase() == Logbase()`, so there is no hardware-level double buffer;
   NewTOS implements real flipping via the framebuffer swap chain.
2. **STe adds a low base byte.** On STe/TT/Videl `atari_physbase` folds in
   `0xFFFF820D`, giving byte-granular base; plain ST is 256-byte aligned.
   NewTOS preserves whatever value `Setscreen` recorded regardless of Atari
   alignment rules.

---

## Dependencies

### Requires (must be initialized / functional)

- The display driver / framebuffer (`v_bas_ad` set by boot / `Setscreen`).

### Used by

- Double-buffering, screen savers, hardware-cursor emulation, screenshot
  tools.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — display state, `v_bas_ad`.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  `Physbase` as the displayed base, distinct from `Logbase`.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `CONF_WITH_ATARI_VIDEO` off → fallback to `logbase()`. Base granularity
  differs ST (256-byte) vs STe/TT/Videl (byte).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Physbase` = the currently-displayed framebuffer in the VideoCore
swap chain; after `Setscreen(log, phys, …)` the next mailbox-driven flip
aligns the two on the next vblank.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (Shifter video-base registers
> `0xFFFF8201`/`0xFFFF8203`/`0xFFFF820D`) at the BCM2711 VideoCore
> framebuffer. The **contract is unchanged** — a full-64-bit pointer, no
> error path, `Physbase() == Logbase()` on the no-page-flip profile.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2 (Screen /
> framebuffer).

- **What the base *is*.** The single VideoCore-allocated framebuffer, one
  buffer for both physical and logical (no hardware page flip). The mailbox
  reply's bus address is masked to an ARM physical address and offset into
  the kernel high half (`kernel/src/drivers/bcm2711/framebuffer.rs`), and
  that region is what `screen()`/`Physbase` describe
  (`kernel/src/vdi_backend.rs`, `kernel/src/host_env.rs`).

### Address space and cacheability (NewTOS, 2026-09-05)

- **The returned pointer is in the caller's address space** (ADR-023). An
  **EL1** caller — the desktop, the VDI, a boot self-test — gets the kernel
  high-half VA, which is what it can reach. An **EL0** caller gets the
  framebuffer published into the user address space (`usermem::expose_attr`)
  and the **user address** back; the kernel VA is EL1-only and a program that
  followed it would take a permission fault, so handing it out satisfied the
  letter of "a pointer to the screen" and not its substance. Since ARCH03
  stage 1 (2026-10-05) the user address is the screen memory's own physical
  address (VA = PA, the same in every process; QEMU `raspi4b`
  `0x3C10_0000`, `virt` `0x5081_7000`, the real Pi 4 `0x3DBF_5000` —
  `docs/analysis/arch03-stage0b-spike.md` §6), mapped with 4-KiB pages
  because it is not 2-MiB aligned. Until then it was a per-process window VA
  (measured `0x00400000`), and before ADR-023 `0xFFFF0000…`.
- **The mapping is `PROT_G`** (FreeMiNT "global — any access OK",
  `references/freemint/sys/mint/mem.h:130-134`), not `PROT_PR`: the point of
  `Physbase` is that the program *writes* pixels.
- **The mapping is Normal Non-Cacheable** (MAIR AttrIdx 2). The pixels are
  read out by the VideoCore display engine, a DMA master that scans DRAM
  from outside the ARM caches (`docs/hardware/mailbox-framebuffer.md` §6).
  The kernel's own alias is Write-Back and the VDI cleans each drawn
  rectangle to the point of coherency itself; an EL0 program has no such
  hook and the TOS API has no cache-maintenance call, so the pages it is
  handed are the ones where a plain store lands in DRAM unaided. While such
  a mapping is live the VDI backend cleans **and invalidates** its
  rectangles instead of merely cleaning them, so the two aliases with
  mismatched cacheability stay coherent (`kernel/src/vdi_backend.rs`,
  `tos-api/src/bdos/process/usermem.rs`). "Live" means: a process that was
  handed the pointer is still running; the first such holder triggers a
  clean-and-invalidate of the whole span.
- **The published span** is `stride_bytes × height` — the whole pixel store,
  1.17 MiB at 640×480 and 7.91 MiB at 1920×1080 — entered once into the
  shared user page tables; it no longer costs a program any `Malloc`
  space.
- **Format.** Linear XRGB8888 (`0xFF_RR_GG_BB`), *not* Atari bitplanes.
  Shifter-format direct-screen writers are a project non-goal (CLAUDE.md
  "Non-goals") and belong to the later m68k emulator; a program written for
  NewTOS gets the linear buffer.
- **Proof.** `sdk/examples/physbase.c` (`PHYSBASE.NCP`) calls `Physbase`,
  stores three colour bands through the returned pointer with no cache
  maintenance of any kind, and blocks; the QMP screendump shows the bands.
- **How it is read (rev 2026-09-05).** `physbase()` returns
  `Host::screen().phys_base` — the address the **display engine is really
  scanning out**, read back from the display rather than from a shadow, the
  way EmuTOS' `physbase()` reconstructs the address from the video-base
  registers (`bios/screen.c:1030`, `atari_physbase()`). Before the
  framebuffer is up, `screen()` returns `None` and `Physbase` yields `0`.
- **Geometry:** config.txt-driven, 32 bpp XRGB8888. On `board-rpi4` the
  screen memory holds **two** whole screens (double virtual height, see
  `docs/hardware/mailbox-framebuffer.md` §3b); on `board-qemu-virt` (ramfb)
  one. Observed on QEMU `raspi4b`: 640 × 480, pitch 2560 B/line, buffer
  2400 KiB = 2 × 1 228 800 B.
- **Setscreen interaction — real since 2026-09-05.** `Setscreen(-1, phys,
  -1, -1)` moves the VideoCore scan-out (mailbox set-virtual-offset tag
  `0x0004_8009`) and `Physbase` then reports the new page; a base the
  display cannot show is not applied and `Physbase` keeps reporting where
  the display really is. See `docs/api/xbios/05-setscreen.md`
  §"Page flipping".
- **What is published to EL0.** `Physbase`/`Logbase` map the display's
  **whole** screen memory (all pages) into the calling process's window as
  one `PROT_G`, Normal-Non-Cacheable block, and return the address of the
  physical / logical page inside it. Publishing the pages as one block is
  what lets a program compute the hidden page as `Logbase() + page_size` and
  hand it back to `Setscreen`.
- **Datasheet.** Mailbox property transport: BCM2711 peripherals datasheet
  §1.3 (`references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`);
  the framebuffer allocate/get-pitch tags are firmware-property tags, not
  peripheral registers.
- **Geometry other than 640x480.** The geometry is config.txt-driven, not hard-coded
  (`kernel/src/drivers/mod.rs:31@BOOT_WIDTH`, default 1920x1080); a program reads it
  through `Ssystem` 2013/2014/2015 or VDI `work_out`, see
  `docs/analysis/screen-readout-spec-2026-11-12.md` (pitch has no program-readable source).
