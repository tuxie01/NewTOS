# `Getrez` (XBIOS 0x04)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1076` (`getrez`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the WORD result, the resolution-code values a
> program branches on. Reference: the Shifter/Videl register reads and
> `sshiftmod`.

## EmuTOS source

**Primary location:** `bios/screen.c:1076` — `WORD getrez(void)`, slot 4 of
`xbios_vecs[]` (`bios/xbios.c:1290`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/screen.c` — `atari_getrez()` — reads the resolution from the Videl
  mode (`vsetmode(-1)` mapped to a ST-compatible code), the TT shifter
  register (`0xFFFF8262`, masked), or the ST shifter register (`0xFFFF8260`,
  masked), depending on the compiled video.

**Headers / type definitions used:**
- `bios/screen.h` — resolution-code constants (`ST_LOW`=0, `ST_MEDIUM`=1,
  `ST_HIGH`=2, `FALCON_REZ`=3, `TT_MEDIUM`=4, `TT_HIGH`=6, `TT_LOW`=7).
- `include/tosvars.h` — `sshiftmod`.

**System variables / globals read:**
- Shifter/Videl mode register (Atari video), or
- `sshiftmod` (`0x44C`) — returned when there is no Atari video.

**System variables / globals written:** none.

**Build configuration affecting this function:**
- `CONF_WITH_ATARI_VIDEO`, `CONF_WITH_VIDEL`, `CONF_WITH_TT_SHIFTER` — which
  register is read. Without Atari video, returns `sshiftmod`.

---

## Source extraction summary

`Getrez` returns the current screen resolution as a small code. On Atari
video `atari_getrez` reads the active Shifter (ST/TT) or Videl mode and maps
it to the classic 0/1/2 (ST) plus the TT/Falcon extensions. Without Atari
video hardware it returns the shadow variable `sshiftmod`, which
`Setscreen`/`atari_setrez` keep in sync.

The codes are the contract programs branch on to pick a drawing strategy
(mono vs 4-plane vs 16-colour). NewTOS reports the code corresponding to the
framebuffer mode it presents.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x04
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0004
  ```
  No parameters (`Getrez()` = `xbios_w_v(4)`, `include/xbiosbind.h:23`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x04` in `x8`; no argument registers; the
  WORD result **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** WORD
- **Encoding:** resolution code:

  | Code | Name | Geometry (Atari) |
  |-----:|------|------------------|
  | 0 | `ST_LOW` | 320×200, 16 colours |
  | 1 | `ST_MEDIUM` | 640×200, 4 colours |
  | 2 | `ST_HIGH` | 640×400, mono |
  | 3 | `FALCON_REZ` | Videl mode (Falcon) |
  | 4 | `TT_MEDIUM` | 640×480, 16 colours |
  | 6 | `TT_HIGH` | 1280×960, mono |
  | 7 | `TT_LOW` | 320×480, 256 colours |

  (Code 5 is unused.)
- **Source location:** `bios/screen.c:1082` (`sshiftmod` fallback) or the
  `atari_getrez` register read.

### Side effects on global state / hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Getrez` cannot fail.

---

## Verification criteria

- **Nominal (mono):** in a mono framebuffer mode, `Getrez()` returns `2`
  (`ST_HIGH`).
- **Consistency with `sshiftmod`:** on the no-Atari-video path, `Getrez()`
  equals the current `sshiftmod`.
- (EmuTOS's no-Atari-video round trip, "after `Setscreen(-1, -1, r)` `Getrez()`
  returns `r`", does not apply to NewTOS: its `Setscreen` ignores `rez`
  (`05-setscreen.md`); removed 2026-11-18, XBIOS23.)

---

## Edge cases discovered in source

1. **`sshiftmod` fallback.** With no Atari video, `Getrez` returns the
   `sshiftmod` shadow (`bios/screen.c:1082`), which `atari_setrez` writes on
   every `Setscreen` rez change — so `Getrez` and `Setscreen` stay coherent
   without real hardware.
2. **Videl maps to ST-compatible codes — never to `FALCON_REZ`.**
   (Corrected 2026-11-12, XBIOS23 spec step: earlier text said "or reports
   `FALCON_REZ`"; the source never returns 3.) On Videl `atari_getrez` takes the
   current mode word `vsetmode(-1)` (the stored `current_video_mode`, not a
   register read-back, `bios/videl.c:619-627`) and returns
   (`bios/screen.c:948-987`):

   | Current Videl mode word | `Getrez()` | Source |
   |-------------------------|-----------:|--------|
   | `VIDEL_COMPAT` set, 1bpp (ST high) | 2 | `bios/screen.c:956-959` |
   | `VIDEL_COMPAT` set, 2bpp (ST medium) | 1 | `:961-962` |
   | `VIDEL_COMPAT` set, 4bpp (ST low) | 0 | `:964-965` |
   | `VIDEL_COMPAT` set, any other bpp field | 0, with `KINFO("Problem - unsupported video mode")` | `:967-969` |
   | `VIDEL_COMPAT` clear — **every** Falcon mode (1/2/4/8bpp, **truecolor**) | **2** | `:971-972` |

   `FALCON_REZ` (3) exists only as the value `vsetmode` stores in `sshiftmod` for a
   non-ST-compatible mode (`bios/videl.c:648-656`), so on Videl `Getrez()` and
   `sshiftmod` differ for every Falcon mode (2 vs 3). The oracle's falcontc boot
   mode is `0x1C` (VGA, 80 columns, truecolor; case 211 = `0x1C`), non-compat, hence
   case 87 = 2 (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).
   Other hardware: TT shifter `TT_SHIFTER & 0x07` (`:976-979`), ST shifter
   `ST_SHIFTER & 0x03` (`:981-984`), no Atari video: `sshiftmod` (`:1076-1085`).
   What EmuTOS itself concludes from `Getrez`: the AES opens its workstation with
   `FALCON_REZ + 2` on Videl and `Getrez() + 2` elsewhere (`aes/gemgsxif.c:159-165,180`),
   i.e. it does **not** consult `Getrez` on a Videl machine; the desktop takes the Videl
   mode word and falls back to `0xFF00 | Getrez()` without Videl (`desk/deskapp.c:1085-1096`);
   `v_opnwk` switches mode only for `work_in[0]-2` equal to an ST/TT code
   (`vdi/vdi_control.c:506-513`); `shifter_rez_changeable` is the other internal consumer
   (`bios/screen.c:112`).

---

## Dependencies

### Requires (must be initialized / functional)

- `sshiftmod` initialised by the display driver / `Setscreen`.

### Used by

- Every graphics program that adapts to resolution; VDI open; AES.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — `sshiftmod` and the resolution codes.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents 0/1/2 for
  ST; TT/Falcon codes added later.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- ST returns 0/1/2; TT adds 4/6/7; Falcon adds 3 (`FALCON_REZ`). The value
  set depends on `CONF_WITH_VIDEL`/`CONF_WITH_TT_SHIFTER`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

### NewTOS behavior (settled — ADR-017 Revision 3, owner decision 2026-11-12)

`getrez()` returns **`ST_HIGH` (2)** — `tos-api/src/xbios/screen.rs:180@getrez`.
NewTOS's screen belongs to the **non-ST-compatible truecolor Videl mode class**:
`VsetMode(-1)` reports the fixed word `0x001C` (VGA, 80 columns, truecolor,
`VIDEL_COMPAT` clear; `58-vsetmode.md`), and EmuTOS's `atari_getrez` answers 2 for
every such word (edge case 2 above). A build-time assertion in `screen.rs` ties the
two: the constant `ST_HIGH` is valid only while the mode word has `VIDEL_COMPAT`
clear. The value equals the oracle's on `falcontc` (bios-r1 case 87); the `tt`
reference (4) is not NewTOS's profile.

As on a Falcon, the code is an **ST-compatibility hint, not a description of the
screen**: depth and geometry come from VDI `DEV_TAB` / `vq_extnd` and `Physbase`,
the Falcon mode from `VsetMode(-1)`. A program written for ST codes reads
"640×400 monochrome"; this is the same answer EmuTOS gives a Falcon in truecolor.
The native geometry is **config.txt-driven and auto-detected at boot** (ADR-017)
and has no classic `vmode_table[]` entry (`bios/screen.c:724-735`). There is **no
in-system switch**: `Getrez` only reports, it never changes, and `Setscreen`'s
`rez` does not move it (`05-setscreen.md`).

Revision 2 (2026-07-16) had fixed the value at `TT_MEDIUM` (4) for a 16-colour
framebuffer; ADR-024 (truecolor VDI) removed that ground.

Host tests: `tos-api/tests/xbios23_getrez.rs` (value, no-framebuffer case,
agreement with `VsetMode(-1)` by EmuTOS's mapping, `Setscreen` rez ignored).

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (Shifter/Videl mode register, or the
> `sshiftmod` shadow at `0x44C`) at the live NewTOS framebuffer mode. The
> **contract is unchanged** — a WORD resolution code, no error path.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

> **Resolution policy (ADR-017, `docs/MEMORY.md`):** NewTOS renders
> **natively at the config.txt-driven HDMI geometry** (auto-detected at boot;
> `drivers::BOOT_WIDTH/BOOT_HEIGHT` = 1920×1080 is the fallback / QEMU-ramfb
> default) on a truecolor VDI (ADR-024). There is **no in-system resolution
> switch** — the firmware `config.txt` owns the mode. Revision 3 (2026-11-12)
> settles the `Getrez` value below.

- **Settled: `Getrez` returns `ST_HIGH` (`2`)** — EmuTOS's answer for the mode
  class the Pi screen belongs to: `atari_getrez()` (`bios/screen.c:948-987`)
  returns `rez=2` for *any* non-ST-compatible Videl mode regardless of its
  real geometry/depth, and NewTOS's `VsetMode(-1)` word `0x001C` is one
  (`58-vsetmode.md`). `vsetmode()` (`bios/videl.c:619-659`) sets the shadow
  `sshiftmod=FALCON_REZ` (`:648-656`) for the same case; NewTOS has no
  `sshiftmod` variable. Falcon software queries `VsetMode(-1)` for the mode;
  NewTOS software reads VDI `DEV_TAB` / `Physbase` for depth and geometry.
- **`Setscreen`'s `rez` is correspondingly accepted-but-ignored** — see
  `docs/api/xbios/05-setscreen.md` (Pi binding).
- **A NewTOS-native true-geometry query** (cookie-jar entry or new trap
  returning live width/height/depth) remains a possible future refinement,
  mirroring the Falcon `Getrez`(hint)/`VsetMode`(truth) split; not needed
  today since VDI `DEV_TAB` / `Physbase` already carry the live geometry.

---

## Coherence with `VsetMode(-1)` (added 2026-11-18, TASK/XBIOS22 spec step 2)

Owner decisions 2026-11-12 (`TASK/XBIOS23.md`: `Getrez()` = 2; `TASK/XBIOS22.md`: `VsetMode(-1)` answers a fixed
word) must agree. The word `0x1C` (`58-vsetmode.md`) has `VIDEL_COMPAT` clear, so by
`bios/screen.c:956-972` `Getrez` is 2 and by `bios/videl.c:648-656` `sshiftmod` would be 3: the same pair the
oracle's falcontc profile measures (cases 87, 211). The "round-trip with `Setscreen`" criterion was removed from
the verification criteria above (XBIOS23): the `rez` of `Setscreen` is ignored (`05-setscreen.md`).

## At the 32-plane setting (ADR-038 point 9, V68)

Spec gate of TASK/V68 (researcher, 2026-11-18). **Answer:** `Getrez()` is 2 (`ST_HIGH`) at `vdiplanes=16` and at
`vdiplanes=32`.

- **Contract (ADR-038 point 9):** unchanged. Reason, from the ADR: the answer depends on the mode word having
  `VIDEL_COMPAT` clear (`tos-api/src/xbios/screen.rs:180@getrez`; ADR-017 Revision 3), and `VsetMode(-1)` stays
  `0x001C` in both settings (`58-vsetmode.md`), so EmuTOS's own mapping gives 2 for it
  (`references/emutos/bios/screen.c:948-987` **[C]**). As before the code is an ST-compatibility hint; depth is read
  from `vq_extnd` / `vq_scrninfo` (32 at the 32-plane setting).
- **Read from the code [C]:** `getrez` returns the constant `ST_HIGH`, no argument, no state
  (`tos-api/src/xbios/screen.rs:180@getrez`), with the build-time check on `FALCON_MODE_WORD` (`tos-api/src/xbios/screen.rs:53@FALCON_MODE_WORD`). **No
  dependency on the VDI depth setting today.**

## Open points

- Inferred: the coherence statement rests on reading `screen.c`/`videl.c`; `0x1C` with `Getrez` 2 is measured
  together only on the oracle (cases 87 and 211), not on NewTOS.
- Inferred: independence of the setting is read from the code, not yet shown by a run at `vdiplanes=32`.
